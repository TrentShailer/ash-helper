mod description;
mod extension;
mod queue_family;

use std::{ffi::CStr, fmt::Display};

pub use description::RequirementDescription;
pub use extension::RequiredExtension;
pub use queue_family::QueueFamilyRequirements;

use ash::vk;
use thiserror::Error;

/*
MVP output:
    Vulkan Instance did not meet the requirements:
    - Supports Vulkan 1.1

    Physical Device AMD Radeon(TM) Graphics did not meet the requirements:
    - Supports extension VK_KHR_get_memory_requirements2, promoted in Vulkan 1.1
    - Supports feature PhysicalDevice16BitStorageFeatures::storage_buffer16_bit_access
    - Supports limit maxPushConstantSize >= 256
    - Supports queue_family with flags COMPUTE, GRAPHICS, TRANSFER


*/

pub trait EntryRequirement {
    /// Validate if the Vulkan Entry meets the requirements.
    fn validate_entry(entry: &ash::Entry) -> ValidationResult;

    /// Returns the instance extension names to request.
    fn required_instance_extensions(entry: &ash::Entry) -> Vec<&'static CStr>;
}

pub trait DeviceRequirement {
    /// Validate if a given physical device meets the requirements.
    fn validate_device(instance: &ash::Instance, device: vk::PhysicalDevice) -> ValidationResult;

    /// Returns the device extension names to request.
    fn required_device_extensions(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> Vec<&'static CStr>;
}

pub trait RequiredFeatures {
    /// Sets this feature's required features.
    fn set_required_features(features: &mut vk::PhysicalDeviceFeatures);
}

pub trait RequiredFeatures2 {
    /// Sets this feature's required feature 2s.
    fn set_required_features2(features: &mut vk::PhysicalDeviceFeatures2);
}

pub trait QueueFamilyRequirement {
    /// Returns the queue family requiements for this feature.
    fn queue_family_requirements() -> QueueFamilyRequirements;
}

pub type ValidationResult = Result<ValidationOutcome, ValidationError>;

impl From<ValidationOutcome> for ValidationResult {
    fn from(value: ValidationOutcome) -> Self {
        Self::Ok(value)
    }
}

/// The outcome from a validation check.
#[non_exhaustive]
#[derive(Debug)]
pub enum ValidationOutcome {
    /// The requirements were met.
    Valid,

    /// The requirements were not met.
    Invalid(Vec<RequirementDescription>),
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("{1} call failed:\n{0}")]
    VulkanCall(#[source] vk::Result, &'static str),
}

impl ValidationOutcome {
    pub fn is_valid(&self) -> bool {
        match self {
            ValidationOutcome::Valid => true,
            ValidationOutcome::Invalid(_) => false,
        }
    }

    pub fn is_invalid(&self) -> bool {
        match self {
            ValidationOutcome::Valid => false,
            ValidationOutcome::Invalid(_) => true,
        }
    }

    /// Joins two validation outcomes.
    pub fn join(self, other: ValidationOutcome) -> ValidationOutcome {
        let mut unmet_requirements = vec![];

        if let ValidationOutcome::Invalid(mut requirements) = other {
            unmet_requirements.append(&mut requirements);
        }

        if let ValidationOutcome::Invalid(mut requirements) = self {
            unmet_requirements.append(&mut requirements);
        }

        if !unmet_requirements.is_empty() {
            return ValidationOutcome::Invalid(unmet_requirements);
        }

        ValidationOutcome::Valid
    }

    pub fn display_physical_device(instance: &ash::Instance, device: vk::PhysicalDevice) -> String {
        let properties = unsafe { instance.get_physical_device_properties(device) };

        format!(
            "Physical Device {:?} ",
            properties.device_name_as_c_str().unwrap_or(c"Invalid Name")
        )
    }
}

impl Display for ValidationOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationOutcome::Valid => write!(f, "meets the requirements."),
            ValidationOutcome::Invalid(unmet_requirements) => {
                write!(f, "does not meet the requirements:")?;
                for requirement in unmet_requirements {
                    writeln!(f, "- {}", requirement.0)?;
                }

                Ok(())
            }
        }
    }
}
