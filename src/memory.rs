use ash::vk;
use thiserror::Error;

use crate::{VkError, VulkanContext};

/// Allocate memory for a buffer. Handles dedicated allocation.
pub unsafe fn allocate_buffer_memory<Vk: VulkanContext>(
    vk: &Vk,
    buffer: vk::Buffer,
    memory_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::DeviceMemory, vk::MemoryRequirements), Error> {
    // Get the memory requirements
    let (memory_requirements, should_be_dedicated) = {
        let buffer_requirements = vk::BufferMemoryRequirementsInfo2::default().buffer(buffer);
        let mut dedicated_requirements = vk::MemoryDedicatedRequirements::default();
        let mut memory_requirements =
            vk::MemoryRequirements2::default().push_next(&mut dedicated_requirements);

        unsafe {
            vk.device()
                .get_buffer_memory_requirements2(&buffer_requirements, &mut memory_requirements)
        };

        let memory_requirements = memory_requirements.memory_requirements;
        let should_be_dedicated = dedicated_requirements.prefers_dedicated_allocation == vk::TRUE;

        (memory_requirements, should_be_dedicated)
    };

    // Find the memory index
    let memory_index = find_memorytype_index(vk, memory_requirements, memory_flags)
        .ok_or(Error::NoSuitableMemoryType)?;

    // Allocate the memory
    let memory = {
        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_index);

        let mut dedicated_allocation = vk::MemoryDedicatedAllocateInfo::default().buffer(buffer);

        let allocate_info = if should_be_dedicated {
            allocate_info.push_next(&mut dedicated_allocation)
        } else {
            allocate_info
        };

        unsafe { vk.device().allocate_memory(&allocate_info, None) }
            .map_err(|e| VkError::new(e, "vkAllocateMemory"))?
    };

    Ok((memory, memory_requirements))
}

/// Allocate memory for an image. Handles dedicated allocation.
pub unsafe fn allocate_image_memory<Vk: VulkanContext>(
    vk: &Vk,
    image: vk::Image,
    memory_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::DeviceMemory, vk::MemoryRequirements), Error> {
    // Get the memory requirements
    let (memory_requirements, should_be_dedicated) = {
        let image_requirements = vk::ImageMemoryRequirementsInfo2::default().image(image);
        let mut dedicated_requirements = vk::MemoryDedicatedRequirements::default();
        let mut memory_requirements =
            vk::MemoryRequirements2::default().push_next(&mut dedicated_requirements);

        unsafe {
            vk.device()
                .get_image_memory_requirements2(&image_requirements, &mut memory_requirements)
        };

        let memory_requirements = memory_requirements.memory_requirements;
        let should_be_dedicated = dedicated_requirements.prefers_dedicated_allocation == vk::TRUE;

        (memory_requirements, should_be_dedicated)
    };

    // Find the memory index
    let memory_index = find_memorytype_index(vk, memory_requirements, memory_flags)
        .ok_or(Error::NoSuitableMemoryType)?;

    // Allocate the memory
    let memory = {
        let allocate_info = vk::MemoryAllocateInfo::default()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_index);

        let mut dedicated_allocation = vk::MemoryDedicatedAllocateInfo::default().image(image);

        let allocate_info = if should_be_dedicated {
            allocate_info.push_next(&mut dedicated_allocation)
        } else {
            allocate_info
        };

        unsafe { vk.device().allocate_memory(&allocate_info, None) }
            .map_err(|e| VkError::new(e, "vkAllocateMemory"))?
    };

    Ok((memory, memory_requirements))
}

/// Finds suitable memory type index for given requirements.
pub fn find_memorytype_index<Vk: VulkanContext>(
    vk: &Vk,
    memory_requirements: vk::MemoryRequirements,
    memory_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let memory_properties = unsafe {
        vk.instance()
            .get_physical_device_memory_properties(vk.physical_device())
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

/// Error variants for trying to allocate memory.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A Vulkan call failed.
    #[error(transparent)]
    VkError(#[from] VkError),

    /// No suitable memory type was available.
    #[error("No suitable memory type was available for the allocation.")]
    NoSuitableMemoryType,
}
