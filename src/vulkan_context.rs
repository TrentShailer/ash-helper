use ash::{ext, vk};

/// This trait provides standard ways to access the Vulkan Context.
pub trait VulkanContext {
    /// Type containing an identifier for a purpose of a queue/queue family.
    ///
    /// ## Example
    /// ```
    /// pub enum QueuePurpose {
    ///     ComputeFamily(u32),
    ///     GraphicsFamily(u32),
    /// }
    /// ```
    type QueuePurpose;

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

    /// Returns the queue family index that was created for `purpose` if it exists.
    fn queue_family_index(&self, purpose: Self::QueuePurpose) -> Option<u32>;

    /// Returns the queue at the index if it exists with the given purpose.
    unsafe fn queue(&self, purpose: Self::QueuePurpose) -> Option<vk::Queue>;
}
