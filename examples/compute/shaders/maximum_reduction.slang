// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  ./examples/shaders/maximum_reduction.slang -o ./examples/shaders/maximum_reduction.spv
// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  ./shaders/maximum_reduction.slang -o ./shaders/maximum_reduction.spv

struct BufferSet
{
    StructuredBuffer<int> read_buffer;
    RWStructuredBuffer<int> write_buffer;
}
[[vk::binding(0, 0)]]
ParameterBlock<BufferSet> buffers;

struct PushConstants
{
    uint input_length;
}
[vk::push_constant]
ConstantBuffer<PushConstants> push_constants; // std140 alignment

static const int OUT_OF_BOUNDS = int.minValue;

[ForceInline]
uint get_thread_read_index(uint group_index, uint thread_index)
{
    let lane_count = WaveGetLaneCount();
    let lane_index = WaveGetLaneIndex();
    let warp_index = thread_index / lane_count;

    let warp_count = 128 / lane_count;

    let warp_stride = lane_count * lane_count;
    let group_stride = warp_count * warp_stride;

    let group_start = group_index * group_stride;
    let warp_start = group_start + warp_stride * warp_index;

    let thread_start = warp_start + lane_index;

    return thread_start;
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

    let stride = lane_count;
    let index = get_thread_read_index(group_id.x, thread_id.x);

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
