use ash::vk;
use thiserror::Error;

use crate::{find_memorytype_index, CoreVulkan};

/// Creates a basic buffer with backing memory, handles dedicated allocations when preferred or required.
pub unsafe fn create_buffer<Vk: CoreVulkan>(
    vk: &Vk,
    size: u64,
    usage: vk::BufferUsageFlags,
    memory_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::MemoryRequirements), Error> {
    let device = vk.vk_device();

    // Create buffer
    let buffer = {
        let create_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        device
            .create_buffer(&create_info, None)
            .map_err(|e| Error::VulkanCall(e, "vkCreateBuffer"))?
    };

    // Allocate Memory
    let (memory, memory_requirements) = {
        let memory_requirements_info = vk::BufferMemoryRequirementsInfo2::default().buffer(buffer);

        let mut dedicated_allocation_requirements = vk::MemoryDedicatedRequirements::default();

        let mut memory_requirements =
            vk::MemoryRequirements2::default().push_next(&mut dedicated_allocation_requirements);

        device.get_buffer_memory_requirements2(&memory_requirements_info, &mut memory_requirements);

        let memory_requirements = memory_requirements.memory_requirements;

        let memory_index = find_memorytype_index(vk, memory_requirements, memory_flags)
            .ok_or(Error::NoSuitableMemoryType)?;

        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_index);

        let should_be_dedicated =
            dedicated_allocation_requirements.prefers_dedicated_allocation == vk::TRUE;

        // Handle allocation or dedicated allocation
        let memory = if should_be_dedicated {
            let mut dedicated_allocation =
                vk::MemoryDedicatedAllocateInfo::default().buffer(buffer);
            let allocate_info = allocate_info.push_next(&mut dedicated_allocation);
            device.allocate_memory(&allocate_info, None)
        } else {
            device.allocate_memory(&allocate_info, None)
        }
        .map_err(|e| Error::VulkanCall(e, "vkAllocateMemory"))?;

        (memory, memory_requirements)
    };

    // bind buffer and memory
    device
        .bind_buffer_memory(buffer, memory, 0)
        .map_err(|e| Error::VulkanCall(e, "vkBindBufferMemory"))?;

    Ok((buffer, memory, memory_requirements))
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("{1} call failed:\n{0}")]
    VulkanCall(#[source] vk::Result, &'static str),

    #[error("No suitable memory type was available for the allocation.")]
    NoSuitableMemoryType,
}
