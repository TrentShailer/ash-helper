use std::sync::Arc;

use ash::vk;
use parking_lot::Mutex;

/// A trait for a structure that has the core Vulkan objects.
pub trait CoreVulkan {
    /// Gets a reference to the vulkan entry.
    unsafe fn vk_entry(&self) -> &ash::Entry;

    /// Gets a reference to the vulkan instance.
    unsafe fn vk_instance(&self) -> &ash::Instance;

    /// Gets a reference to the logical device.
    unsafe fn vk_device(&self) -> &ash::Device;

    /// Gets a copy of the physical device.
    unsafe fn vk_physical_device(&self) -> vk::PhysicalDevice;

    /// Gets the queue from a given queue family.
    unsafe fn get_queue(
        &self,
        family_index: u32,
        queue_index: usize,
    ) -> Option<Arc<Mutex<vk::Queue>>>;

    /// Tries to find the queue family indicies of the queue families that meets the requirements.
    /// Should only return queue family indicies that have queues created on device creation.
    unsafe fn find_queue_family_indicies(
        &self,
        flags: vk::QueueFlags,
        queue_count: u32,
    ) -> Option<Vec<u32>>;
}
