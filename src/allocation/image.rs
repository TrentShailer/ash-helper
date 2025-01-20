use ash::vk;

use crate::{try_name, VkError, VulkanContext};

use super::{memory::allocate_image_memory, AllocationError};

/// Allocate and bind memory to a new buffer.
pub unsafe fn allocate_image<Vk: VulkanContext>(
    vk: &Vk,
    image_create_info: &vk::ImageCreateInfo<'_>,
    memory_flags: vk::MemoryPropertyFlags,
    label: &str,
) -> Result<(vk::Image, vk::DeviceMemory, vk::MemoryRequirements), AllocationError> {
    let image = {
        let image = unsafe { vk.device().create_image(image_create_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateImage"))?;

        unsafe { try_name(vk, image, &format!("{label} Image")) };

        image
    };

    let (memory, requirements) = {
        let (memory, requirements) = unsafe { allocate_image_memory(vk, image, memory_flags) }?;

        unsafe { try_name(vk, memory, &format!("{label} Image Memory")) };

        (memory, requirements)
    };

    unsafe { vk.device().bind_image_memory(image, memory, 0) }
        .map_err(|e| VkError::new(e, "vkBindImageMemory"))?;

    Ok((image, memory, requirements))
}
