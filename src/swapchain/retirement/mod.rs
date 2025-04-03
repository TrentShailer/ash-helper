use acquisition::Acquisition;
use ash::vk;
use tracing::error;

use crate::{LabelledVkResult, SurfaceContext, VkError, VulkanContext, try_name};

use super::Swapchain;

pub mod acquisition;
mod process;

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

    /// Process the swapchains in retirement.
    pub fn process_retirement<Vulkan, Surface>(
        &mut self,
        vulkan: &Vulkan,
        surface: &Surface,
    ) -> LabelledVkResult<()>
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
    {
        self.process_acquisitions(vulkan)?;
        self.recycle_garbage(vulkan)?;
        self.destroy_completed_swapchains(vulkan, surface)?;

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
        self.free_fences
            .iter()
            .for_each(|fence| unsafe { vulkan.device().destroy_fence(*fence, None) });
        self.free_fences.clear();

        // Destroy garbage fences
        self.garbage_fences
            .iter()
            .for_each(|fence| unsafe { vulkan.device().destroy_fence(*fence, None) });
        self.garbage_fences.clear();

        // Destroy acquisition fences
        self.tracked_acquisitions
            .iter()
            .for_each(|acquisition| unsafe {
                vulkan.device().destroy_fence(acquisition.fence, None)
            });
        self.tracked_acquisitions.clear();

        // destroy swapchains
        self.retired_swapchains
            .iter()
            .for_each(|swapchain| unsafe { swapchain.destroy(vulkan, surface) });
        self.retired_swapchains.clear();
    }
}
