use core::{fmt, slice};

pub use acquire::Frame;
pub use info::SwapchainInfo;
pub use preferences::SwapchainPreferences;
pub use resources::FrameResources;
pub use retirement::SwapchainRetirement;

use ash::vk;

use crate::{
    LabelledVkResult, MaybeMutex, SurfaceContext, VkError, VulkanContext, try_name, try_name_all,
};

mod acquire;
mod info;
mod preferences;
mod resources;
mod retirement;

/// A swapchain and associated resources and details.
pub struct Swapchain {
    /// Flag if the swapchain needs to be recreated.
    pub needs_to_rebuild: bool,
    /// The swapchain info.
    pub info: SwapchainInfo,

    /// The swapchain.
    pub swapchain: vk::SwapchainKHR,
    /// The swapchain images.
    pub images: Vec<vk::Image>,
    /// The swapchain images' views.
    pub views: Vec<vk::ImageView>,

    /// The index of the current frame resources.
    pub next_resources: usize,
    /// The resources for each frame.
    pub resources: Vec<FrameResources>,

    /// The image indicies this swapchain has acquired.
    pub acquired_images: Vec<u32>,
    /// The image indicies this swapchain has presented.
    pub presented_images: Vec<u32>,
}

impl Swapchain {
    /// Create a new swapchain for the surface with preferences.
    pub unsafe fn new<Vulkan, Surface>(
        vulkan: &Vulkan,
        surface: &Surface,
        old_swapchain: Option<&mut Self>,
        swapchain_create_info: vk::SwapchainCreateInfoKHR<'_>,
    ) -> LabelledVkResult<Self>
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
    {
        let swapchain_create_info = if let Some(swapchain) = old_swapchain.as_ref() {
            swapchain_create_info.old_swapchain(swapchain.swapchain)
        } else {
            swapchain_create_info
        };

        // Create swapchain
        let swapchain = unsafe {
            surface
                .swapchain_device()
                .create_swapchain(&swapchain_create_info, None)
                .map_err(|e| VkError::new(e, "vkCreateSwapchainKHR"))?
        };
        unsafe { try_name(vulkan, swapchain, "Swapchain") };

        // Retrieve images
        let images = {
            let images = unsafe { surface.swapchain_device().get_swapchain_images(swapchain) }
                .map_err(|e| VkError::new(e, "vkGetSwapchainImagesKHR"))?;

            unsafe { try_name_all(vulkan, &images, "Swapchain Image") };

            images
        };
        let image_count = images.len();

        // Create image views
        let image_views = {
            // Image views can only be created when the image is backed with memory.
            if swapchain_create_info
                .flags
                .contains(vk::SwapchainCreateFlagsKHR::DEFERRED_MEMORY_ALLOCATION_EXT)
            {
                vec![vk::ImageView::null(); image_count]
            } else {
                images
                    .iter()
                    .enumerate()
                    .map(|(index, &image)| {
                        Self::create_view(
                            vulkan,
                            index as u32,
                            image,
                            swapchain_create_info.image_format,
                            swapchain_create_info.image_array_layers,
                        )
                    })
                    .collect::<Result<Vec<_>, VkError>>()?
            }
        };

        // Create frame resources
        let (resources, next_resources) = {
            let existing_count = old_swapchain
                .as_ref()
                .map(|swapchain| swapchain.resources.len())
                .unwrap_or(0);

            let new_resources = if image_count > existing_count {
                (existing_count..image_count)
                    .map(|index| unsafe { FrameResources::new(vulkan, index) })
                    .collect::<Result<Vec<_>, VkError>>()?
            } else {
                vec![]
            };

            match old_swapchain {
                Some(swapchain) => {
                    let mut resources: Vec<_> = swapchain.resources.drain(..).collect();
                    resources.extend(new_resources.iter());
                    (resources, swapchain.next_resources)
                }
                None => (new_resources, 0),
            }
        };

        let info = SwapchainInfo::new(&swapchain_create_info, image_count);

        Ok(Self {
            needs_to_rebuild: false,
            info,

            swapchain,
            images,
            views: image_views,

            next_resources,
            resources,

            acquired_images: vec![],
            presented_images: vec![],
        })
    }

    /// Queue a present operation for this swapchain.
    pub fn queue_present<'m, Surface, Queue>(
        &mut self,
        surface: &Surface,
        image_index: u32,
        wait_semaphore: vk::Semaphore,
        queue: Queue,
    ) -> LabelledVkResult<()>
    where
        Surface: SurfaceContext,
        Queue: Into<MaybeMutex<'m, vk::Queue>>,
    {
        // Track the present history for this swapchain
        if !self.presented_images.contains(&image_index) {
            self.presented_images.push(image_index);
        }

        // Queue present
        let result = {
            let present_info = vk::PresentInfoKHR::default()
                .image_indices(slice::from_ref(&image_index))
                .swapchains(slice::from_ref(&self.swapchain))
                .wait_semaphores(slice::from_ref(&wait_semaphore));

            let (queue, _queue_guard) = queue.into().lock();
            unsafe {
                surface
                    .swapchain_device()
                    .queue_present(queue, &present_info)
            }
        };

        // Flag swapchain as needing to rebuild.
        let suboptimal = match result {
            Ok(suboptimal) => suboptimal,

            Err(e) => match e {
                vk::Result::ERROR_OUT_OF_DATE_KHR => true,

                e => return Err(VkError::new(e, "vkQueuePresentKHR")),
            },
        };

        if suboptimal {
            self.needs_to_rebuild = true;
        }

        Ok(())
    }

    /// Converts a physical position to a position in Vulkan space.
    pub fn screen_to_vulkan_space(&self, physical: [f32; 2]) -> [f32; 2] {
        [
            (physical[0] / self.info.extent.width as f32) * 2.0 - 1.0,
            (physical[1] / self.info.extent.height as f32) * 2.0 - 1.0,
        ]
    }

    /// Destroys the Vulkan resources created for the swapchain.
    pub unsafe fn destroy<Vulkan: VulkanContext, Surface: SurfaceContext>(
        &self,
        vulkan: &Vulkan,
        surface: &Surface,
    ) {
        unsafe {
            surface
                .swapchain_device()
                .destroy_swapchain(self.swapchain, None)
        };

        for &image_view in &self.views {
            unsafe { vulkan.device().destroy_image_view(image_view, None) };
        }

        for resource in &self.resources {
            unsafe { resource.destroy(vulkan) };
        }
    }

    /// Creates the view for a swapchain image. `DEFERRED_MEMORY_ALLOCATION` requires this is called
    /// only after the image has been acquired.
    fn create_view<Vulkan>(
        vulkan: &Vulkan,
        image_index: u32,
        image: vk::Image,
        format: vk::Format,
        layers: u32,
    ) -> LabelledVkResult<vk::ImageView>
    where
        Vulkan: VulkanContext,
    {
        let create_info = vk::ImageViewCreateInfo::default()
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .base_array_layer(0)
                    .layer_count(layers)
                    .level_count(1),
            )
            .image(image);

        let image_view = unsafe { vulkan.device().create_image_view(&create_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateImageView"))?;

        unsafe {
            try_name(
                vulkan,
                image_view,
                &format!("Swapchain Image View {image_index}"),
            );
        };

        Ok(image_view)
    }
}

impl fmt::Debug for Swapchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Swapchain")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}
