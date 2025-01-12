// slangc -g2 -emit-spirv-directly -target spirv -profile spirv_1_4  ./examples/shaders/maximum_reduction.slang -o ./examples/shaders/maximum_reduction.spv

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

[shader("compute")]
[numthreads(64, 1, 1)]
void main(
    uint3 global_id: SV_DispatchThreadID, // 0 - (WORKGROUP_SIZE * dispatch_x)
    uint3 group_id: SV_GroupID,           // 0 - dispatch_x
    uint3 thread_id: SV_GroupThreadID,    // 0 - WORKGROUP_SIZE
)
{
    let input_length = push_constants.input_length;
    let lane_count = WaveGetLaneCount();

    // The number of values that is processed by each thread.
    let thread_values = lane_count;

    // The starting index in the buffer for this global thread id.
    let index = global_id.x * thread_values;

    var thread_maximum = OUT_OF_BOUNDS;

    for (uint i = 0; i < lane_count; i++)
    {
        uint read_index = index + i;

        // Handle out of bounds using most negative value.
        let fetch_value = read_index < input_length ? buffers.read_buffer[read_index] : OUT_OF_BOUNDS;

        // Note that this \/ causes a stall on global memory load from /\ ~90% of samples are on this load.
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
