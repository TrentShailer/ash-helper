use ash::{prelude::VkResult, vk};

use crate::VulkanContext;

/// Returns if all fences are signalled, does not wait.
pub unsafe fn fences_are_signaled<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    fences: &[vk::Fence],
) -> VkResult<bool> {
    let result = unsafe { vulkan.device().wait_for_fences(fences, true, 0) };

    let all_signaled = match result {
        Ok(_) => true,

        Err(error) => {
            if error == vk::Result::TIMEOUT {
                false
            } else {
                return Err(error);
            }
        }
    };

    Ok(all_signaled)
}
