use core::slice;

pub use preferences::SwapchainPreferences;
pub use resources::FrameResources;

use ash::vk;

use crate::{
    cmd_transition_image, onetime_command, try_name, LabelledVkResult, MaybeMutex, SurfaceContext,
    VkError, VulkanContext,
};

mod preferences;
mod resources;

/// A swapchain and associated resources and details.
pub struct Swapchain {
    /// Flag if the swapchain needs to be recreated.
    pub needs_to_rebuild: bool,

    /// The number of images in the swapchain.
    pub max_frames_in_flight: u32,
    /// The extent of the swapchain.
    pub extent: vk::Extent2D,
    /// The surface format of the swapchain.
    pub format: vk::SurfaceFormatKHR,
    /// The swachain's composite alpha.
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    /// The swachain's present mode.
    pub present_mode: vk::PresentModeKHR,

    /// The swapchain.
    pub swapchain: vk::SwapchainKHR,

    /// The index of the current frame resources.
    pub current_resources: u32,

    /// The swapchain images.
    pub images: Vec<vk::Image>,
    /// The swapchain images' views.
    pub views: Vec<vk::ImageView>,
    /// The resources for each frame.
    pub resources: Vec<FrameResources>,
}

impl Swapchain {
    /// Create a new swapchain for the surface with preferences.
    ///
    /// Images are created with the `COLOR_ATTACHMENT` usage.
    pub unsafe fn new<'m, Vulkan, Surface, Pool, Queue>(
        vulkan: &Vulkan,
        surface: &Surface,
        transition_pool: Pool,
        transition_queue: Queue,
        old_swapchain: Option<vk::SwapchainKHR>,
        preferences: &SwapchainPreferences,
    ) -> LabelledVkResult<Self>
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
        Queue: Into<MaybeMutex<'m, vk::Queue>>,
        Pool: Into<MaybeMutex<'m, vk::CommandPool>>,
    {
        // Get surface capabilities
        let capabilities = unsafe {
            surface
                .surface_instance()
                .get_physical_device_surface_capabilities(
                    vulkan.physical_device(),
                    surface.surface(),
                )
                .map_err(|e| VkError::new(e, "vkGetPhysicalDeviceSurfaceCapabilitiesKHR"))?
        };

        // Select surface format
        let surface_format = unsafe {
            surface
                .surface_instance()
                .get_physical_device_surface_formats(vulkan.physical_device(), surface.surface())
                .map_err(|e| VkError::new(e, "vkGetPhysicalDeviceSurfaceFormatsKHR"))?
        }
        .into_iter()
        .min_by_key(|format| {
            let format_position = if let Some(preferences) = preferences.format.as_ref() {
                preferences
                    .iter()
                    .position(|preference| *preference == format.format)
                    .unwrap_or(usize::MAX)
            } else {
                0
            };

            let colour_space_position = if let Some(preferences) = preferences.colour_space.as_ref()
            {
                preferences
                    .iter()
                    .position(|preference| *preference == format.color_space)
                    .unwrap_or(usize::MAX)
            } else {
                0
            };

            format_position + colour_space_position
        })
        .unwrap();

        // Select the present mode
        let present_mode = {
            surface
                .surface_instance()
                .get_physical_device_surface_present_modes(
                    vulkan.physical_device(),
                    surface.surface(),
                )
                .map_err(|e| VkError::new(e, "vkGetPhysicalDeviceSurfacePresentModesKHR"))?
                .into_iter()
                .min_by_key(|present_mode| {
                    let Some(preferences) = preferences.present_mode.as_ref() else {
                        return 0;
                    };

                    preferences
                        .iter()
                        .position(|preference| preference == present_mode)
                        .unwrap_or(usize::MAX)
                })
                .unwrap()
        };

        // Select the composite alpha
        let composite_alpha = {
            match preferences.composite_alpha.as_ref() {
                Some(preferences) => preferences
                    .iter()
                    .find(|&&preference| {
                        capabilities.supported_composite_alpha.contains(preference)
                    })
                    .unwrap_or(&vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .to_owned(),

                None => vk::CompositeAlphaFlagsKHR::OPAQUE,
            }
        };

        // Get the image count
        let image_count = {
            let max_image_count = if capabilities.max_image_count == 0 {
                preferences.frames_in_flight
            } else {
                capabilities.max_image_count
            };

            preferences
                .frames_in_flight
                .clamp(capabilities.min_image_count, max_image_count)
        };

        // Create swapchain
        let swapchain = {
            let create_info = vk::SwapchainCreateInfoKHR::default()
                .surface(surface.surface())
                .min_image_count(image_count)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(capabilities.current_extent)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(capabilities.current_transform)
                .composite_alpha(composite_alpha)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);

            let create_info = if let Some(old_swapchain) = old_swapchain {
                create_info.old_swapchain(old_swapchain)
            } else {
                create_info
            };

            surface
                .swapchain_device()
                .create_swapchain(&create_info, None)
                .map_err(|e| VkError::new(e, "vkCreateSwapchainKHR"))?
        };

        try_name(vulkan, swapchain, "Swapchain");

        // Retrieve images

        let images = unsafe { surface.swapchain_device().get_swapchain_images(swapchain) }
            .map_err(|e| VkError::new(e, "vkGetSwapchainImagesKHR"))?;

        // Create image views
        let image_views = {
            let create_info = vk::ImageViewCreateInfo::default()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(surface_format.format)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1)
                        .level_count(1),
                );

            (0..image_count)
                .map(|index| {
                    let image = images[index as usize];
                    try_name(vulkan, image, &format!("Swapchain Image {index}"));

                    let create_info = create_info.image(image);

                    let image_view =
                        unsafe { vulkan.device().create_image_view(&create_info, None) }
                            .map_err(|e| VkError::new(e, "vkCreateImageView"))?;
                    try_name(vulkan, image_view, &format!("Swapchain Image View {index}"));

                    Ok(image_view)
                })
                .collect::<Result<Vec<_>, VkError>>()?
        };

        // Create frame resources
        let frame_resources = (0..image_count)
            .map(|index| FrameResources::new(vulkan, index))
            .collect::<Result<Vec<_>, VkError>>()?;

        // Transition images
        unsafe {
            onetime_command(
                vulkan,
                transition_pool,
                transition_queue,
                |vk, command_buffer| {
                    for image in &images {
                        cmd_transition_image(
                            vk,
                            command_buffer,
                            *image,
                            vk::ImageLayout::UNDEFINED,
                            preferences.image_layout,
                        )
                        .unwrap();
                    }
                },
                "Transition Swapchain Images",
            )
        }?;

        Ok(Self {
            needs_to_rebuild: false,
            current_resources: 0,

            swapchain,

            images,
            views: image_views,

            resources: frame_resources,

            present_mode,
            composite_alpha,
            format: surface_format,
            max_frames_in_flight: image_count,
            extent: capabilities.current_extent,
        })
    }

    /// Returns a copy of the resources for the current frame. Waits for the resorces to be free.
    pub fn current_resources<Vk: VulkanContext>(
        &self,
        vk: &Vk,
    ) -> LabelledVkResult<FrameResources> {
        let resouces = self.resources[self.current_resources as usize];
        unsafe {
            vk.device()
                .wait_for_fences(slice::from_ref(&resouces.in_flight_fence), true, u64::MAX)
                .map_err(|e| VkError::new(e, "vkWaitForFences"))?;
        }

        Ok(resouces)
    }

    /// Converts a physical position to a position in Vulkan space.
    pub fn screen_space(&self, physical: [f32; 2]) -> [f32; 2] {
        [
            (physical[0] / self.extent.width as f32) * 2.0 - 1.0,
            (physical[1] / self.extent.height as f32) * 2.0 - 1.0,
        ]
    }

    /// Destroys the Vulkan resources created for the swapchain.
    pub unsafe fn destroy<Vk: VulkanContext, Surface: SurfaceContext>(
        &self,
        vk: &Vk,
        surface: &Surface,
    ) {
        unsafe {
            surface
                .swapchain_device()
                .destroy_swapchain(self.swapchain, None)
        };

        for frame_resource in &self.resources {
            unsafe { frame_resource.destroy(vk) };
        }

        for &image_view in &self.views {
            unsafe { vk.device().destroy_image_view(image_view, None) };
        }
    }
}
