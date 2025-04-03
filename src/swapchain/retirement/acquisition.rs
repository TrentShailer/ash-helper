use core::slice;

use ash::vk;

use crate::{LabelledVkResult, VkError, VulkanContext, fences_are_signaled};

/// A swapchain image acquisition.
pub struct Acquisition {
    /// The swapchain that the image will be acquired from.
    pub swapchain: vk::SwapchainKHR,
    /// The fence tracking the acquisition.
    pub fence: vk::Fence,
    /// The image index that will be acquired.
    pub image_index: u32,
}

impl Acquisition {
    /// Is this image acquired.
    pub fn is_acquired<Vulkan: VulkanContext>(&self, vulkan: &Vulkan) -> LabelledVkResult<bool> {
        let is_acquired = unsafe { fences_are_signaled(vulkan, slice::from_ref(&self.fence)) }
            .map_err(|e| VkError::new(e, "vkWaitForFences"))?;

        Ok(is_acquired)
    }
}
