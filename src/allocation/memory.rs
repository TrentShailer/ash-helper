use ash::vk;

use crate::{VK_GLOBAL_ALLOCATOR, VkError, VulkanContext};

use super::AllocationError;

/// Allocate memory for a buffer. Handles dedicated allocation.
pub unsafe fn allocate_buffer_memory<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    buffer: vk::Buffer,
    memory_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::DeviceMemory, vk::MemoryRequirements), AllocationError> {
    // Get the memory requirements
    let (memory_requirements, should_be_dedicated) = {
        let buffer_requirements = vk::BufferMemoryRequirementsInfo2::default().buffer(buffer);
        let mut dedicated_requirements = vk::MemoryDedicatedRequirements::default();
        let mut memory_requirements =
            vk::MemoryRequirements2::default().push_next(&mut dedicated_requirements);

        unsafe {
            vulkan
                .device()
                .get_buffer_memory_requirements2(&buffer_requirements, &mut memory_requirements)
        };

        let memory_requirements = memory_requirements.memory_requirements;
        let should_be_dedicated = dedicated_requirements.prefers_dedicated_allocation == vk::TRUE;

        (memory_requirements, should_be_dedicated)
    };

    // Find the memory index
    let memory_index = find_memorytype_index(vulkan, memory_requirements, memory_flags)
        .ok_or(AllocationError::NoSuitableMemoryType)?;

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

        unsafe {
            vulkan
                .device()
                .allocate_memory(&allocate_info, VK_GLOBAL_ALLOCATOR.as_deref())
        }
        .map_err(|e| VkError::new(e, "vkAllocateMemory"))?
    };

    Ok((memory, memory_requirements))
}

/// Allocate memory for an image. Handles dedicated allocation.
pub unsafe fn allocate_image_memory<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    image: vk::Image,
    memory_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::DeviceMemory, vk::MemoryRequirements), AllocationError> {
    // Get the memory requirements
    let (memory_requirements, should_be_dedicated) = {
        let image_requirements = vk::ImageMemoryRequirementsInfo2::default().image(image);
        let mut dedicated_requirements = vk::MemoryDedicatedRequirements::default();
        let mut memory_requirements =
            vk::MemoryRequirements2::default().push_next(&mut dedicated_requirements);

        unsafe {
            vulkan
                .device()
                .get_image_memory_requirements2(&image_requirements, &mut memory_requirements)
        };

        let memory_requirements = memory_requirements.memory_requirements;
        let should_be_dedicated = dedicated_requirements.prefers_dedicated_allocation == vk::TRUE;

        (memory_requirements, should_be_dedicated)
    };

    // Find the memory index
    let memory_index = find_memorytype_index(vulkan, memory_requirements, memory_flags)
        .ok_or(AllocationError::NoSuitableMemoryType)?;

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

        unsafe {
            vulkan
                .device()
                .allocate_memory(&allocate_info, VK_GLOBAL_ALLOCATOR.as_deref())
        }
        .map_err(|e| VkError::new(e, "vkAllocateMemory"))?
    };

    Ok((memory, memory_requirements))
}

/// Finds suitable memory type index for given requirements.
pub fn find_memorytype_index<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    memory_requirements: vk::MemoryRequirements,
    memory_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let memory_properties = unsafe {
        vulkan
            .instance()
            .get_physical_device_memory_properties(vulkan.physical_device())
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
