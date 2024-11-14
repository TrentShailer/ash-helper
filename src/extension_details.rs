use std::ffi::CStr;

use crate::Version;

/// The details about a Vulkan extension.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ExtensionDetails {
    /// The extension name
    pub name: &'static CStr,

    /// The vulkan version the extension was promoted to core.
    pub promoted: Option<Version>,
}

impl ExtensionDetails {
    /// Constructs a new ExtensionDetails.
    ///
    /// # Usage
    /// ```
    /// RequiredExtension::new(khr::get_physical_device_properties2::NAME, Some(Version::V1_1));
    /// ```
    pub const fn new(name: &'static CStr, promoted: Option<Version>) -> Self {
        Self { name, promoted }
    }
}
