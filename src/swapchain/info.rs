use ash::vk;

/// The swapchain info.
#[derive(Debug, Clone, Copy)]
pub struct SwapchainInfo {
    /// The number of images in the swapchain.
    pub image_count: usize,
    /// The extent of the swapchain.
    pub extent: vk::Extent2D,
    /// The surface format of the swapchain.
    pub format: vk::SurfaceFormatKHR,
    /// The swachain's composite alpha.
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    /// The swachain's present mode.
    pub present_mode: vk::PresentModeKHR,
    /// The swapchain's image layers
    pub image_layers: u32,
}

impl SwapchainInfo {
    /// Inherit swapchain info from create info and the swapchain image count.
    pub fn new(create_info: &vk::SwapchainCreateInfoKHR<'_>, image_count: usize) -> Self {
        Self {
            image_count,
            extent: create_info.image_extent,
            format: vk::SurfaceFormatKHR::default()
                .format(create_info.image_format)
                .color_space(create_info.image_color_space),
            composite_alpha: create_info.composite_alpha,
            present_mode: create_info.present_mode,
            image_layers: create_info.image_array_layers,
        }
    }
}
