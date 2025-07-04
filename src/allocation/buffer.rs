use ash::vk;

use crate::{VK_GLOBAL_ALLOCATOR, VkError, VulkanContext, try_name};

use super::{AllocationError, memory::allocate_buffer_memory};

/// Allocate and bind memory to a new buffer.
pub unsafe fn allocate_buffer<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    create_info: &vk::BufferCreateInfo<'_>,
    memory_flags: vk::MemoryPropertyFlags,
    label: &str,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::MemoryRequirements), AllocationError> {
    let buffer = {
        let buffer = unsafe {
            vulkan
                .device()
                .create_buffer(create_info, VK_GLOBAL_ALLOCATOR.as_deref())
        }
        .map_err(|e| VkError::new(e, "vkCreateBuffer"))?;

        unsafe { try_name(vulkan, buffer, &format!("{label} Buffer")) };

        buffer
    };

    let (memory, requirements) = {
        let (memory, requirements) =
            unsafe { allocate_buffer_memory(vulkan, buffer, memory_flags) }?;

        unsafe { try_name(vulkan, memory, &format!("{label} Buffer Memory")) };

        (memory, requirements)
    };

    unsafe { vulkan.device().bind_buffer_memory(buffer, memory, 0) }
        .map_err(|e| VkError::new(e, "vkBindBufferMemory"))?;

    Ok((buffer, memory, requirements))
}
