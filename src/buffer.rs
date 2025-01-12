use ash::vk;
use thiserror::Error;

use crate::{allocate_buffer_memory, try_name, VkError, VulkanContext};

/// Allocate and bind memory to a new buffer.
pub unsafe fn allocate_buffer<Vk: VulkanContext>(
    vk: &Vk,
    buffer_create_info: &vk::BufferCreateInfo<'_>,
    memory_flags: vk::MemoryPropertyFlags,
    label: &str,
) -> Result<(vk::Buffer, vk::DeviceMemory, vk::MemoryRequirements), Error> {
    let buffer = {
        let buffer = unsafe { vk.device().create_buffer(buffer_create_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateBuffer"))?;

        unsafe { try_name(vk, buffer, &format!("{label} Buffer")) };

        buffer
    };

    let (memory, requirements) = {
        let (memory, requirements) = unsafe { allocate_buffer_memory(vk, buffer, memory_flags) }
            .map_err(|e| match e {
                crate::AllocateMemoryError::VkError(vk_error) => Error::VkError(vk_error),
                crate::AllocateMemoryError::NoSuitableMemoryType => Error::NoSuitableMemoryType,
            })?;

        unsafe { try_name(vk, memory, &format!("{label} Buffer Memory")) };
        (memory, requirements)
    };

    unsafe { vk.device().bind_buffer_memory(buffer, memory, 0) }
        .map_err(|e| VkError::new(e, "vkBindBufferMemory"))?;

    Ok((buffer, memory, requirements))
}

/// Error variants for trying to allocate buffer.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A Vulkan call failed.
    #[error(transparent)]
    VkError(#[from] VkError),

    /// No suitable memory type was available.
    #[error("No suitable memory type was available for the allocation.")]
    NoSuitableMemoryType,

    /// No Queue Family Index exists with the purpose.
    #[error("No queue family index exists with the purpose")]
    NoQueueFamilyIndex,
}
