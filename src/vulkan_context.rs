use ash::{ext, khr, vk};

/// This trait provides standard ways to access the Vulkan Context.
pub trait VulkanContext {
    /// Gets a reference to the Vulkan entry.
    unsafe fn entry(&self) -> &ash::Entry;

    /// Gets a reference to the Vulkan instance.
    unsafe fn instance(&self) -> &ash::Instance;

    /// Gets a reference to the logical device.
    unsafe fn device(&self) -> &ash::Device;

    /// Gets a copy of the physical device.
    unsafe fn physical_device(&self) -> vk::PhysicalDevice;

    /// Returns Some if this Vulkan instance wants other functions to debug.
    unsafe fn debug(&self) -> Option<&ext::debug_utils::Device>;

    /// Returns the queue family index.
    fn queue_family_index(&self) -> u32;

    /// Returns the queue family index as a slice.
    fn queue_family_index_as_slice(&self) -> &[u32];
}

/// This trait provides standard ways to access the Vulkan Surface Context.
pub trait SurfaceContext {
    /// Gets a reference to the surface instance.
    unsafe fn surface_instance(&self) -> &khr::surface::Instance;

    /// Gets a reference to the swapchain device.
    unsafe fn swapchain_device(&self) -> &khr::swapchain::Device;

    /// Gets a reference to the surface.
    unsafe fn surface(&self) -> vk::SurfaceKHR;
}

/// The Vulkan Context implements additional context.
pub trait Context<T>: VulkanContext {
    /// Gets the context associated object.
    unsafe fn context(&self) -> &T;
}
