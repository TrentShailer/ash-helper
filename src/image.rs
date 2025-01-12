use ash::vk;
use thiserror::Error;

use crate::{allocate_image_memory, try_name, VkError, VulkanContext};

/// Allocate and bind memory to a new buffer.
pub unsafe fn allocate_image<Vk: VulkanContext>(
    vk: &Vk,
    image_create_info: &vk::ImageCreateInfo<'_>,
    memory_flags: vk::MemoryPropertyFlags,
    label: &str,
) -> Result<(vk::Image, vk::DeviceMemory, vk::MemoryRequirements), Error> {
    let image = {
        let image = unsafe { vk.device().create_image(image_create_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateImage"))?;

        unsafe { try_name(vk, image, &format!("{label} Image")) };

        image
    };

    let (memory, requirements) = {
        let (memory, requirements) = unsafe { allocate_image_memory(vk, image, memory_flags) }
            .map_err(|e| match e {
                crate::AllocateMemoryError::VkError(vk_error) => Error::VkError(vk_error),
                crate::AllocateMemoryError::NoSuitableMemoryType => Error::NoSuitableMemoryType,
            })?;

        unsafe { try_name(vk, memory, &format!("{label} Image Memory")) };
        (memory, requirements)
    };

    unsafe { vk.device().bind_image_memory(image, memory, 0) }
        .map_err(|e| VkError::new(e, "vkBindImageMemory"))?;

    Ok((image, memory, requirements))
}

/// Error variants for trying to allocate the image.
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
