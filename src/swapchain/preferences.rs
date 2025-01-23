use ash::vk;

/// Preferences for creating the swapchain.
#[derive(Default)]
pub struct SwapchainPreferences<'a> {
    /// The preferred number of images available.
    pub frames_in_flight: u32,

    /// The preferred image layout after creation.
    pub image_layout: vk::ImageLayout,

    /// The preferred swapchain format.
    pub format: Option<&'a [vk::SurfaceFormatKHR]>,

    /// The preferred present mode.
    pub present_mode: Option<&'a [vk::PresentModeKHR]>,

    /// The preferred composite alpha.
    pub composite_alpha: Option<&'a [vk::CompositeAlphaFlagsKHR]>,
}

impl<'a> SwapchainPreferences<'a> {
    /// Sets the preferred frames in flight.
    pub fn frames_in_flight(mut self, frames: u32) -> Self {
        self.frames_in_flight = frames;
        self
    }

    /// Sets the preferred format list.
    pub fn format(mut self, formats: &'a [vk::SurfaceFormatKHR]) -> Self {
        self.format = Some(formats);
        self
    }

    /// Sets the present mode preference list.
    pub fn present_mode(mut self, present_modes: &'a [vk::PresentModeKHR]) -> Self {
        self.present_mode = Some(present_modes);
        self
    }

    /// Sets the composite alpha preference list.
    pub fn composite_alpha(mut self, composite_alpha: &'a [vk::CompositeAlphaFlagsKHR]) -> Self {
        self.composite_alpha = Some(composite_alpha);
        self
    }

    /// Sets the image layout preference.
    ///
    /// Must be one of the layouts supported by [cmd_transition_image](crate::cmd_transition_image).
    pub fn image_layout(mut self, image_layout: vk::ImageLayout) -> Self {
        self.image_layout = image_layout;
        self
    }
}
