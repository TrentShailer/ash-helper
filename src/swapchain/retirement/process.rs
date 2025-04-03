use core::slice;

use crate::{LabelledVkResult, SurfaceContext, VkError, VulkanContext, fences_are_signaled};

use super::SwapchainRetirement;

impl SwapchainRetirement {
    /// Recycle the garbage fences that are signalled.
    pub(super) fn recycle_garbage<Vulkan: VulkanContext>(
        &mut self,
        vulkan: &Vulkan,
    ) -> LabelledVkResult<()> {
        // Find and remove the signaled fences from the garbage.
        let mut signaled_fences = {
            let mut signaled_fences = vec![];

            let mut end_index = self.garbage_fences.len();
            let mut index = 0;
            while index <= end_index {
                let is_signaled = unsafe {
                    fences_are_signaled(vulkan, slice::from_ref(&self.garbage_fences[index]))
                        .map_err(|e| VkError::new(e, "vkWaitForFences"))?
                };

                if is_signaled {
                    // The current index has been replaced with the last item, thus current index
                    // should not change.
                    let fence = self.garbage_fences.swap_remove(index);
                    signaled_fences.push(fence);

                    // An item has been removed from the vec, thus the end index should be decremented.
                    end_index -= 1;
                } else {
                    // Move to the next item
                    index += 1;
                }
            }

            signaled_fences
        };

        // Reset the signaled fences
        unsafe { vulkan.device().reset_fences(&signaled_fences) }
            .map_err(|e| VkError::new(e, "vkResetFences"))?;

        // Move the fences to free fences
        self.free_fences.append(&mut signaled_fences);

        Ok(())
    }

    /// Processes the tracked acquisitions to remove the completed acquisitions' image indicies from
    /// the retired swapchain's present history.
    pub(super) fn process_acquisitions<Vulkan>(&mut self, vulkan: &Vulkan) -> LabelledVkResult<()>
    where
        Vulkan: VulkanContext,
    {
        // Find and remove the completed acquisitions
        let completed_acquisitions = {
            let mut completed_acquisitions = vec![];

            let mut end_index = self.tracked_acquisitions.len();
            let mut index = 0;
            while index <= end_index {
                if self.tracked_acquisitions[index].is_acquired(vulkan)? {
                    // The current index has been replaced with the last item, thus current index
                    // should not change.
                    let acquisition = self.tracked_acquisitions.swap_remove(index);
                    completed_acquisitions.push(acquisition);

                    // An item has been removed from the vec, thus the end index should be decremented.
                    end_index -= 1;
                } else {
                    // Move to the next item
                    index += 1;
                }
            }

            completed_acquisitions
        };

        // Remove the completed acquisitions image index from each of the swapchains, where that swapchain is not associated with the acqusitions.
        for acquisition in &completed_acquisitions {
            self.retired_swapchains.iter_mut().for_each(|swapchain| {
                if swapchain.swapchain != acquisition.swapchain {
                    swapchain
                        .presented_images
                        .retain(|image| *image != acquisition.image_index)
                }
            });
        }

        // Recycle the fences
        {
            let mut fences: Vec<_> = completed_acquisitions
                .iter()
                .map(|acquisition| acquisition.fence)
                .collect();

            // Reset the fences
            unsafe { vulkan.device().reset_fences(&fences) }
                .map_err(|e| VkError::new(e, "vkResetFences"))?;

            // Recycle the fences
            self.free_fences.append(&mut fences);
        }

        Ok(())
    }

    /// Destroy the swapchains that have completed their work.
    pub(super) fn destroy_completed_swapchains<Vulkan, Surface>(
        &mut self,
        vulkan: &Vulkan,
        surface: &Surface,
    ) -> LabelledVkResult<()>
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
    {
        // Remove the swapchains that are safe to destory.
        let safe_swapchains = {
            let mut safe_swapchains = vec![];

            let mut end_index = self.retired_swapchains.len();
            let mut index = 0;
            while index <= end_index {
                if self.retired_swapchains[index].presented_images.is_empty() {
                    // The current index has been replaced with the last item, thus current index
                    // should not change.
                    let swapchain = self.retired_swapchains.swap_remove(index);
                    safe_swapchains.push(swapchain);

                    // An item has been removed from the vec, thus the end index should be decremented.
                    end_index -= 1;
                } else {
                    // Move to the next item
                    index += 1;
                }
            }

            safe_swapchains
        };

        // Destroy the swapchains
        safe_swapchains
            .into_iter()
            .for_each(|swapchain| unsafe { swapchain.destroy(vulkan, surface) });

        Ok(())
    }
}
