// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  ./examples/shaders/maximum_reduction.slang -o ./examples/shaders/maximum_reduction.spv
// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  ./shaders/maximum_reduction.slang -o ./shaders/maximum_reduction.spv

struct BufferSet
{
    StructuredBuffer<int> read_buffer;
    RWStructuredBuffer<int> write_buffer;
}

struct PushConstants
{
    uint input_length;
}

[[vk::binding(0, 0)]]
ParameterBlock<BufferSet> buffers;

[vk::push_constant]
ConstantBuffer<PushConstants> push_constants;

[SpecializationConstant]
const uint SUBGROUP_SIZE;

static const int OUT_OF_BOUNDS = int.minValue;

// The amount each thread increments their read index by every iteration
uint iteration_stride()
{
    return WaveGetLaneCount();
}

// The starting index in the buffer for this thread
uint thread_start(uint global_index)
{
    let lane_count = WaveGetLaneCount();
    let values_per_warp = lane_count * lane_count;

    let global_warp_index = global_index / lane_count;
    let global_warp_start = global_warp_index * values_per_warp;

    return global_warp_start + WaveGetLaneIndex();
}

[shader("compute")]
[numthreads(128, 1, 1)]
void main(
    uint3 global_id: SV_DispatchThreadID,
    uint3 group_id: SV_GroupID,
    uint3 thread_id: SV_GroupThreadID, )
{
    let lane_count = WaveGetLaneCount();

    let input_length = push_constants.input_length;

    let stride = iteration_stride();
    let index = thread_start(global_id.x);

    var thread_maximum = OUT_OF_BOUNDS;

    for (uint i = 0; i < lane_count; i++)
    {
        uint read_index = index + i * stride;

        // Handle out of bounds using most negative value.
        let fetch_value = read_index < input_length ? buffers.read_buffer[read_index] : OUT_OF_BOUNDS;

        let maximum = WaveActiveMax(fetch_value);

        // Store the result if this maximum is for this lane.
        if (WaveGetLaneIndex() == i)
        {
            thread_maximum = maximum;
        }
    }

    let maximum = WaveActiveMax(thread_maximum);

    let wave_index = global_id.x / lane_count;
    if (WaveIsFirstLane())
    {
        buffers.write_buffer[wave_index] = maximum;
    }
}
