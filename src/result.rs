use core::fmt::Display;

use ash::vk;
use thiserror::Error;

/// A shortcut for `Result<T, VkError>`.
pub type LabelledVkResult<T> = Result<T, VkError>;

/// A Vulkan Result wrapped with some context for the call that triggered the error.
#[derive(Debug, Error)]
pub struct VkError {
    vk_call: &'static str,
    #[source]
    source: vk::Result,
}

impl VkError {
    /// Create a VkError from a `vk::Result` and a label.
    pub fn new(source: vk::Result, vk_call: &'static str) -> Self {
        Self { vk_call, source }
    }
}

impl Display for VkError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} call failed:\n{}", self.vk_call, self.source)
    }
}
