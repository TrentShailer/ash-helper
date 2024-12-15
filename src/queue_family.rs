use std::sync::Arc;

use ash::vk;
use parking_lot::Mutex;

/// A transparent wrapper around a queue family and it's created
pub struct QueueFamily {
    /// The index of the queue family in the device's list of queue families.
    pub index: u32,

    /// The queue flags for this queue family.
    pub flags: vk::QueueFlags,

    /// The queues allocated from this queue family.
    pub queues: Vec<Arc<Mutex<vk::Queue>>>,
}
