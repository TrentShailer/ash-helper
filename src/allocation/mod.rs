pub use buffer::allocate_buffer;
pub use image::allocate_image;
pub use memory::{allocate_buffer_memory, allocate_image_memory, find_memorytype_index};
pub use slice::{BufferAlignment, BufferUsageFlags};
pub use vk_global_allocator::VK_GLOBAL_ALLOCATOR;

use crate::VkError;
use thiserror::Error;

mod buffer;
mod image;
mod memory;
mod slice;
/// Utilities for using the Rust global allocator with Vulkan.
pub mod vk_global_allocator;

/// Allocation failure reason.
#[derive(Debug, Error)]
pub enum AllocationError {
    /// The allocation failed at a Vulkan call.
    #[error(transparent)]
    VkError(#[from] VkError),

    /// The allocation failed because the device didn't have a memory type to match the allocation.
    #[error("The device had no suitable memory type for the allocation")]
    NoSuitableMemoryType,
}
