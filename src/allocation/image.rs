use ash::vk;

use crate::{VK_GLOBAL_ALLOCATOR, VkError, VulkanContext, try_name};

use super::{AllocationError, memory::allocate_image_memory};

/// Allocate and bind memory to a new buffer.
pub unsafe fn allocate_image<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    create_info: &vk::ImageCreateInfo<'_>,
    memory_flags: vk::MemoryPropertyFlags,
    label: &str,
) -> Result<(vk::Image, vk::DeviceMemory, vk::MemoryRequirements), AllocationError> {
    let image = {
        let image = unsafe {
            vulkan
                .device()
                .create_image(create_info, VK_GLOBAL_ALLOCATOR.as_deref())
        }
        .map_err(|e| VkError::new(e, "vkCreateImage"))?;

        unsafe { try_name(vulkan, image, &format!("{label} Image")) };

        image
    };

    let (memory, requirements) = {
        let (memory, requirements) = unsafe { allocate_image_memory(vulkan, image, memory_flags) }?;

        unsafe { try_name(vulkan, memory, &format!("{label} Image Memory")) };

        (memory, requirements)
    };

    unsafe { vulkan.device().bind_image_memory(image, memory, 0) }
        .map_err(|e| VkError::new(e, "vkBindImageMemory"))?;

    Ok((image, memory, requirements))
}
