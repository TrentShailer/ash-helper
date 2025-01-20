use ash::vk;

use crate::{try_name, VkError, VulkanContext};

use super::{memory::allocate_buffer_memory, AllocationError};

/// Allocate and bind memory to a new buffer.
pub unsafe fn allocate_buffer<Vk: VulkanContext>(
    vk: &Vk,
    buffer_create_info: &vk::BufferCreateInfo<'_>,
    memory_flags: vk::MemoryPropertyFlags,
    label: &str,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::MemoryRequirements), AllocationError> {
    let buffer = {
        let buffer = unsafe { vk.device().create_buffer(buffer_create_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateBuffer"))?;

        unsafe { try_name(vk, buffer, &format!("{label} Buffer")) };

        buffer
    };

    let (memory, requirements) = {
        let (memory, requirements) = unsafe { allocate_buffer_memory(vk, buffer, memory_flags) }?;

        unsafe { try_name(vk, memory, &format!("{label} Buffer Memory")) };

        (memory, requirements)
    };

    unsafe { vk.device().bind_buffer_memory(buffer, memory, 0) }
        .map_err(|e| VkError::new(e, "vkBindBufferMemory"))?;

    Ok((buffer, memory, requirements))
}
