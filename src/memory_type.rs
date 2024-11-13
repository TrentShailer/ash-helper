use ash::vk;

use crate::CoreVulkan;

/// Finds suitable memory type index for given requirements.
pub fn find_memorytype_index<Vk>(
    vk: &Vk,
    memory_requirements: vk::MemoryRequirements,
    memory_flags: vk::MemoryPropertyFlags,
) -> Option<u32>
where
    Vk: CoreVulkan,
{
    let memory_properties = unsafe {
        vk.vk_instance()
            .get_physical_device_memory_properties(vk.vk_physical_device())
    };

    memory_properties.memory_types[..memory_properties.memory_type_count as _]
        .iter()
        .enumerate()
        .find(|(index, memory_type)| {
            (1 << index) & memory_requirements.memory_type_bits != 0
                && memory_type.property_flags & memory_flags == memory_flags
        })
        .map(|(index, _memory_type)| index as _)
}
