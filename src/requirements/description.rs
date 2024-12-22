use crate::Version;

use super::RequiredExtension;

/// A human understandable requirement.
#[non_exhaustive]
#[derive(Debug)]
pub struct RequirementDescription(pub String);

impl RequirementDescription {
    /// Format a requirement for a device to be non-null.
    pub fn non_null_device() -> Self {
        Self("Is not null".to_string())
    }

    /// Format a requirement from an extension.
    /// ## Example
    /// ```
    /// let extension = RequiredExtension::new(khr::dedicated_allocation::NAME).promoted(Version::V1_1);
    /// RequirementDescription::extension(extension);
    /// ```
    pub fn extension(extension: &RequiredExtension) -> Self {
        let promoted_str = match extension.promoted {
            Some(version) => format!(", promoted in Vulkan {}.{}", version.major, version.minor),
            None => String::new(),
        };

        Self(format!(
            "Supports extension {:?}{}",
            extension.name, promoted_str
        ))
    }

    /// Format a requirement from a feature.
    /// ## Example
    /// ```
    /// RequirementDescription::feature("PhysicalDeviceFeatures", "geometry_shader");
    /// RequirementDescription::feature("PhysicalDevice16BitStorageFeatures", "storage_buffer16_bit_access");
    /// ```
    pub fn feature(feature: &'static str, field: &'static str) -> Self {
        Self(format!("Supports feature {feature}::{field}"))
    }

    /// Format a requirement from a version, only shows major and minor version.
    /// ## Example
    /// ```
    /// RequirementDescription::vulkan_version(Version::V1_1);
    /// ```
    pub fn vulkan_version(version: Version) -> Self {
        Self(format!(
            "Supports Vulkan {}.{}",
            version.major, version.minor
        ))
    }

    /// Format a requirement from a version, only shows major and minor version.
    /// ## Example
    /// ```
    /// RequirementDescription::limit("maxPushConstantsSize", ">= 256");
    /// ```
    pub fn limit(field: &'static str, requirement: &'static str) -> Self {
        Self(format!("Supports limit {field} {requirement}"))
    }
}
