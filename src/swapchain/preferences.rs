use ash::vk;

use crate::{LabelledVkResult, SurfaceContext, VkError, VulkanContext};

/// Preferences for creating the swapchain.
#[derive(Default, Clone)]
pub struct SwapchainPreferences {
    /// The preferred number of images available.
    pub image_count: u32,

    /// The preferred image layout after creation.
    pub image_layout: vk::ImageLayout,

    /// The preferred swapchain format.
    pub format: Option<Vec<vk::Format>>,

    /// The preferred swapchain colour space.
    pub colour_space: Option<Vec<vk::ColorSpaceKHR>>,

    /// The preferred present mode.
    pub present_mode: Option<Vec<vk::PresentModeKHR>>,

    /// The preferred composite alpha.
    pub composite_alpha: Option<Vec<vk::CompositeAlphaFlagsKHR>>,
}

impl SwapchainPreferences {
    /// Sets the preferred number of images for the swapchain to have.
    pub fn image_count(mut self, frames: u32) -> Self {
        self.image_count = frames;
        self
    }

    /// Sets the image layout preference.
    ///
    /// Must be one of the layouts supported by [cmd_transition_image](crate::cmd_transition_image).
    pub fn image_layout(mut self, image_layout: vk::ImageLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    /// Sets the preferred format list.
    pub fn format(mut self, formats: Vec<vk::Format>) -> Self {
        self.format = Some(formats);
        self
    }

    /// Sets the preferred colour space list.
    pub fn colour_space(mut self, colour_space: Vec<vk::ColorSpaceKHR>) -> Self {
        self.colour_space = Some(colour_space);
        self
    }

    /// Sets the present mode preference list.
    pub fn present_mode(mut self, present_modes: Vec<vk::PresentModeKHR>) -> Self {
        self.present_mode = Some(present_modes);
        self
    }

    /// Sets the composite alpha preference list.
    pub fn composite_alpha(mut self, composite_alpha: Vec<vk::CompositeAlphaFlagsKHR>) -> Self {
        self.composite_alpha = Some(composite_alpha);
        self
    }

    /// Populates a swapchain create info based on preferences, device capabilities, and reasonable
    /// defaults.
    ///
    /// ## From capabilities:
    /// * `min_image_count`
    /// * `image_color_space`
    /// * `image_format`
    /// * `composite_alpha`
    /// * `present_mode`
    ///
    /// ## Reasonable defaults:
    /// Field                | Value
    /// ---------------------|------
    /// `surface`            | `surface.surface()`
    /// `image_extent`       | `capabilities.current_extent`
    /// `pre_transform`      | `capabilities.current_transform`
    /// `image_usage`        | `vk::ImageUsageFlags::COLOR_ATTACHMENT`
    /// `image_sharing_mode` | `vk::SharingMode::EXCLUSIVE`
    /// `clipped`            | `true`
    /// `image_array_layers` | `1`
    pub fn get_swapchain_create_info<Vulkan, Surface>(
        &self,
        vulkan: &Vulkan,
        surface: &Surface,
    ) -> LabelledVkResult<vk::SwapchainCreateInfoKHR<'_>>
    where
        Vulkan: VulkanContext,
        Surface: SurfaceContext,
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
            let format_position = if let Some(preferences) = self.format.as_ref() {
                preferences
                    .iter()
                    .position(|preference| *preference == format.format)
                    .unwrap_or(usize::MAX)
            } else {
                0
            };

            let colour_space_position = if let Some(preferences) = self.colour_space.as_ref() {
                preferences
                    .iter()
                    .position(|preference| *preference == format.color_space)
                    .unwrap_or(usize::MAX)
            } else {
                0
            };

            match format_position.checked_add(colour_space_position) {
                Some(value) => value,
                None => usize::MAX,
            }
        })
        .unwrap();

        // Select the present mode
        let present_mode = unsafe {
            surface
                .surface_instance()
                .get_physical_device_surface_present_modes(
                    vulkan.physical_device(),
                    surface.surface(),
                )
                .map_err(|e| VkError::new(e, "vkGetPhysicalDeviceSurfacePresentModesKHR"))?
                .into_iter()
                .min_by_key(|present_mode| {
                    let Some(preferences) = self.present_mode.as_ref() else {
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
            let first_supported = || {
                let supported = capabilities.supported_composite_alpha;
                if supported.contains(vk::CompositeAlphaFlagsKHR::OPAQUE) {
                    vk::CompositeAlphaFlagsKHR::OPAQUE
                } else if supported.contains(vk::CompositeAlphaFlagsKHR::INHERIT) {
                    vk::CompositeAlphaFlagsKHR::INHERIT
                } else if supported.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
                    vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
                } else if supported.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
                    vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
                } else {
                    vk::CompositeAlphaFlagsKHR::OPAQUE
                }
            };

            match self.composite_alpha.as_ref() {
                Some(preferences) => preferences
                    .iter()
                    .find(|&&preference| {
                        capabilities.supported_composite_alpha.contains(preference)
                    })
                    .copied()
                    .unwrap_or_else(first_supported),

                None => first_supported(),
            }
        };

        // Get the image count
        let image_count = {
            let max_image_count = if capabilities.max_image_count == 0 {
                self.image_count
            } else {
                capabilities.max_image_count
            };

            self.image_count
                .clamp(capabilities.min_image_count, max_image_count)
        };

        // Create swapchain info
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .min_image_count(image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .composite_alpha(composite_alpha)
            .present_mode(present_mode)
            .image_extent(capabilities.current_extent)
            .pre_transform(capabilities.current_transform)
            .surface(unsafe { surface.surface() })
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .clipped(true)
            .image_array_layers(1);

        Ok(create_info)
    }
}
