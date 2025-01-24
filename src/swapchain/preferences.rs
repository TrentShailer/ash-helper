use ash::vk;

/// Preferences for creating the swapchain.
#[derive(Default, Clone)]
pub struct SwapchainPreferences {
    /// The preferred number of images available.
    pub frames_in_flight: u32,

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
    /// Sets the preferred frames in flight.
    pub fn frames_in_flight(mut self, frames: u32) -> Self {
        self.frames_in_flight = frames;
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
}
