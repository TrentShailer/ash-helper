use core::slice;

use ash::vk;

use crate::{LabelledVkResult, SurfaceContext, VkError, VulkanContext};

use super::{FrameResources, Swapchain};

impl Swapchain {
    /// Acquire the next image from this swapchain and the resources to use.
    pub fn acquire_next_image<Vulkan, Surface>(
        &mut self,
        vulkan: &Vulkan,
        surface: &Surface,
    ) -> LabelledVkResult<Option<(u32, vk::Image, vk::ImageView, FrameResources)>>
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
    {
        // Get the resources
        let resources = self.next_resources(vulkan)?;

        // Get the image index
        let image_index = {
            let acquire_result = unsafe {
                surface.swapchain_device().acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    resources.acquire_semaphore,
                    vk::Fence::null(),
                )
            };

            // If out of date, flag rebuild.
            let (image_index, suboptimal) = match acquire_result {
                Ok(v) => v,
                Err(e) => match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.needs_to_rebuild = true;
                        return Ok(None);
                    }

                    vk::Result::NOT_READY => return Ok(None),

                    e => return Err(VkError::new(e, "vkAcquireNextImageKHR")),
                },
            };

            // If suboptimal, flag rebuild.
            if suboptimal {
                self.needs_to_rebuild = true;
            }

            image_index
        };

        // Get the image
        let image = self.images[image_index as usize];

        // Get or create the image view
        let view = {
            if self.views[image_index as usize] == vk::ImageView::null() {
                let view = Self::create_view(
                    vulkan,
                    image_index,
                    image,
                    self.info.format.format,
                    self.info.image_layers,
                )?;

                self.views[image_index as usize] = view;

                view
            } else {
                self.views[image_index as usize]
            }
        };

        // Track the present history for this swapchain
        if !self.presented_images.contains(&image_index) {
            self.presented_images.push(image_index);
        }

        Ok(Some((image_index, image, view, resources)))
    }

    /// Returns a copy of the next resources in the circular buffer. Waits for the resources to be
    /// free.
    pub fn next_resources<Vulkan: VulkanContext>(
        &mut self,
        vulkan: &Vulkan,
    ) -> LabelledVkResult<FrameResources> {
        let resources = self.resources[self.next_resources];

        unsafe {
            vulkan
                .device()
                .wait_for_fences(slice::from_ref(&resources.render_fence), true, u64::MAX)
                .map_err(|e| VkError::new(e, "vkWaitForFences"))?;
        };

        self.next_resources = (self.next_resources + 1) % self.resources.len();

        Ok(resources)
    }
}
