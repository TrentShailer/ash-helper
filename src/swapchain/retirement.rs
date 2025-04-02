use core::slice;

use ash::vk;
use tracing::error;

use crate::{
    LabelledVkResult, SurfaceContext, VkError, VulkanContext, fences_are_signaled, try_name,
};

use super::Swapchain;

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

/// Handles correctly destroying and freeing retired swapchains when their resources are no longer
/// in use.
pub struct SwapchainRetirement {
    /// The retired but non-destroyed swapchains.
    pub retired_swapchains: Vec<Swapchain>,

    /// Acquisitions to track for confirmation when that frame has been presented.
    pub tracked_acquisitions: Vec<Acquisition>,

    /// Fences that need to be freed once they have signalled.
    pub garbage_fences: Vec<vk::Fence>,

    /// The fences that are free to be used for tracking an acquisition.
    pub free_fences: Vec<vk::Fence>,

    /// The number of fences the swapchain retirement has created.
    pub fence_count: usize,
}

impl SwapchainRetirement {
    #[allow(clippy::new_without_default)]
    /// Create a new instance of Self.
    pub fn new() -> Self {
        Self {
            retired_swapchains: vec![],
            tracked_acquisitions: vec![],
            garbage_fences: vec![],
            free_fences: vec![],
            fence_count: 0,
        }
    }

    /// Returns if the retirement houses a given swapchain.
    pub fn houses_swapchain(&self, swapchain: vk::SwapchainKHR) -> bool {
        self.retired_swapchains
            .iter()
            .any(|housed_swapchain| housed_swapchain.swapchain == swapchain)
    }

    /// Recycle the garbage fences that are signalled.
    pub fn recycle_garbage<Vulkan: VulkanContext>(
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
    pub fn process_acquisitions<Vulkan>(&mut self, vulkan: &Vulkan) -> LabelledVkResult<()>
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
    pub fn destroy_completed_swapchains<Vulkan, Surface>(
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

    /// House a retired swapchain to be destroyed.
    pub fn house_swapchain(&mut self, swapchain: Swapchain) {
        self.retired_swapchains.push(swapchain);
    }

    /// Track a frame acquisition.
    pub fn track_acquisition(
        &mut self,
        swapchain: vk::SwapchainKHR,
        fence: vk::Fence,
        image_index: u32,
    ) {
        // If the image index is already tracked, then it should be replaced with this new
        // acquisition and the fence should be marked as garbage.
        // Else, a new acquisition should be tracked.
        match self
            .tracked_acquisitions
            .iter_mut()
            .find(|tracked_acquisition| tracked_acquisition.image_index == image_index)
        {
            Some(tracked_acquisition) => {
                self.garbage_fences.push(tracked_acquisition.fence);
                tracked_acquisition.swapchain = swapchain;
                tracked_acquisition.fence = fence;
            }
            None => {
                let acquisition = Acquisition {
                    swapchain,
                    fence,
                    image_index,
                };
                self.tracked_acquisitions.push(acquisition);
            }
        }
    }

    /// Get a free fence else create a new one to use with a `vkAcquireNextImageKHR` operation.
    /// The fence **MUST** be returned to the retirement via [`Self::track_acquisition`].
    pub fn get_fence<Vulkan: VulkanContext>(
        &mut self,
        vulkan: &Vulkan,
    ) -> LabelledVkResult<vk::Fence> {
        match self.free_fences.pop() {
            Some(fence) => Ok(fence),

            None => {
                let create_info = vk::FenceCreateInfo::default();

                let fence = unsafe { vulkan.device().create_fence(&create_info, None) }
                    .map_err(|e| VkError::new(e, "vkCreateFence"))?;

                unsafe {
                    try_name(
                        vulkan,
                        fence,
                        &format!("Swapchain Retirement Fence {}", self.fence_count),
                    )
                };

                self.fence_count += 1;

                Ok(fence)
            }
        }
    }

    /// Destroys the retirement, its resources, and all housed swapchains.
    pub fn destroy<Vulkan, Surface>(&mut self, vulkan: &Vulkan, surface: &Surface)
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
    {
        // Wait for device idle
        if let Err(e) = unsafe { vulkan.device().device_wait_idle() } {
            error!("Destroy SwapchainRetirement: Failed to wait for device idle: {e}");
            return;
        }

        // Destroy free fences
        {
            self.free_fences
                .iter()
                .for_each(|fence| unsafe { vulkan.device().destroy_fence(*fence, None) });
            self.free_fences.clear();
        }

        // Destroy garbage fences
        {
            self.garbage_fences
                .iter()
                .for_each(|fence| unsafe { vulkan.device().destroy_fence(*fence, None) });
            self.garbage_fences.clear();
        }

        // Destroy acquisition fences
        {
            self.tracked_acquisitions
                .iter()
                .for_each(|acquisition| unsafe {
                    vulkan.device().destroy_fence(acquisition.fence, None)
                });
            self.tracked_acquisitions.clear();
        }

        // destroy swapchains
        {
            self.retired_swapchains
                .iter()
                .for_each(|swapchain| unsafe { swapchain.destroy(vulkan, surface) });
            self.retired_swapchains.clear();
        }
    }
}
