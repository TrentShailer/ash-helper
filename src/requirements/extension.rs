use std::ffi::CStr;

use ash::vk::{self, Handle};

use crate::Version;

use super::{RequirementDescription, ValidationError, ValidationOutcome, ValidationResult};

pub struct RequiredExtension {
    pub name: &'static CStr,
    pub promoted: Option<Version>,
}

impl RequiredExtension {
    pub fn new(name: &'static CStr) -> Self {
        Self {
            name,
            promoted: None,
        }
    }

    pub fn promoted(mut self, version: Version) -> Self {
        self.promoted = Some(version);
        self
    }

    /// Returns the list of extensions that must be requested. Does not check if the entry supports
    /// the exensions.
    pub fn request_for_instance(
        extensions: &[RequiredExtension],
        entry: &ash::Entry,
    ) -> Vec<&'static CStr> {
        let mut extensions_to_request = vec![];

        // Errors on OOHM
        let instance_version = match unsafe { entry.try_enumerate_instance_version().unwrap() } {
            Some(version) => Version::from_vulkan_version(version),
            None => Version::V1_0,
        };

        for extension in extensions {
            if let Some(promoted) = extension.promoted {
                if instance_version >= promoted {
                    continue;
                }
            }

            extensions_to_request.push(extension.name);
        }

        extensions_to_request
    }

    /// Returns the list of extensions that must be requested. Does not check if the device supports
    /// the exensions.
    pub fn request_for_device(
        extensions: &[RequiredExtension],
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> Vec<&'static CStr> {
        let mut extensions_to_request = vec![];

        let properties = unsafe { instance.get_physical_device_properties(device) };
        let device_version = Version::from_vulkan_version(properties.api_version);

        for extension in extensions {
            if let Some(promoted) = extension.promoted {
                if device_version >= promoted {
                    continue;
                }
            }

            extensions_to_request.push(extension.name);
        }

        extensions_to_request
    }

    /// Validates if a device supports all listed extensions.
    pub fn validate_device(
        extensions: &[RequiredExtension],
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> ValidationResult {
        let mut unmet_requirements = vec![];
        if device.is_null() {
            unmet_requirements.push(RequirementDescription::non_null_device());
            return ValidationOutcome::Invalid(unmet_requirements).into();
        }

        let properties = unsafe { instance.get_physical_device_properties(device) };
        let device_version = Version::from_vulkan_version(properties.api_version);

        // Provided by vk1.0, errors on OOHM, OODM.
        let extension_properties = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .map_err(|e| {
                    ValidationError::VulkanCall(e, "vkEnumerateDeviceExtensionProperties")
                })?
        };
        let supported_extensions: Vec<_> = extension_properties
            .iter()
            .filter_map(|extension| extension.extension_name_as_c_str().ok())
            .collect();

        for extension in extensions {
            if let Some(promoted) = extension.promoted {
                if device_version >= promoted {
                    continue;
                }
            }

            if !supported_extensions.contains(&extension.name) {
                unmet_requirements.push(RequirementDescription::extension(extension));
            }
        }

        if !unmet_requirements.is_empty() {
            return ValidationOutcome::Invalid(unmet_requirements).into();
        }
        ValidationOutcome::Valid.into()
    }

    ///Validates if the Vulkan Entry supports all listed extensions
    pub fn validate_entry(
        extensions: &[RequiredExtension],
        entry: &ash::Entry,
    ) -> ValidationResult {
        let mut unmet_requirements = vec![];

        let instance_version = match unsafe {
            entry
                .try_enumerate_instance_version()
                .map_err(|e| ValidationError::VulkanCall(e, "vkEnumerateInstanceVersion"))?
        } {
            Some(version) => Version::from_vulkan_version(version),
            None => Version::V1_0,
        };

        let extension_properties = unsafe {
            entry
                .enumerate_instance_extension_properties(None)
                .map_err(|e| {
                    ValidationError::VulkanCall(e, "vkEnumerateInstanceExtensionProperties")
                })?
        };

        let supported_extensions: Vec<_> = extension_properties
            .iter()
            .filter_map(|extension| extension.extension_name_as_c_str().ok())
            .collect();

        for extension in extensions {
            if let Some(promoted) = extension.promoted {
                if instance_version >= promoted {
                    continue;
                }
            }

            if !supported_extensions.contains(&extension.name) {
                unmet_requirements.push(RequirementDescription::extension(extension));
            }
        }

        if !unmet_requirements.is_empty() {
            return ValidationOutcome::Invalid(unmet_requirements).into();
        }
        ValidationOutcome::Valid.into()
    }
}
