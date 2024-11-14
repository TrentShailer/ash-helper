use std::fmt::Display;

use ash::{
    khr,
    prelude::VkResult,
    vk::{self, ExtensionProperties},
};
use thiserror::Error;

use crate::{ExtensionDetails, Version};

/// A Physical device and its properties.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PhysicalDevice {
    pub physical_device: vk::PhysicalDevice,
    pub api_version: Version,
    pub extension_properties: Vec<ExtensionProperties>,
    pub properties: vk::PhysicalDeviceProperties,
    pub queue_families: Vec<vk::QueueFamilyProperties>,
}

impl PhysicalDevice {
    /// Retrieves all current physical devices and their properties.
    ///
    /// # Feature Suport Checking
    /// ```
    /// fn supports_features(instance: &ash::Instance, device: vk::PhysicalDevice) -> bool {
    ///     let mut synchronization2 = vk::PhysicalDeviceSynchronization2Features::default();
    ///     let mut device_features = vk::PhysicalDeviceFeatures2::default()
    ///         .push_next(&mut synchronization2);
    ///
    ///     unsafe { instance.get_physical_device_features2(device, &mut device_features) };
    ///
    ///     synchronization2.synchronization2 == vk::TRUE
    /// }
    /// ```
    pub fn get_all(instance: &ash::Instance) -> Result<Vec<Self>, Error> {
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(Error::VkEnumeratePhysicalDevices)?
        };

        physical_devices
            .into_iter()
            .map(|device| Self::new(instance, device))
            .collect()
    }

    /// Retrieves the relevant properties from a physical device.
    pub fn new(instance: &ash::Instance, device: vk::PhysicalDevice) -> Result<Self, Error> {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let api_version = Version::from_vulkan_version(properties.api_version);

        let extension_properties = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .map_err(Error::VkEnumerateDeviceExtensionProperties)?
        };

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        Ok(Self {
            physical_device: device,
            api_version,
            extension_properties,
            properties,
            queue_families,
        })
    }

    /// Returns if the physical device supports a set of extensions either via direct support or extension pomotion.
    ///
    /// # Safety
    /// - All `name` values in `extensions` must be correct Vulkan extension names.
    /// - All `promoted` versions in `extensions` must be correct for the extension.
    /// - All parameters of `self` must be unmodified.
    pub unsafe fn supports_extensions(&self, extensions: &[ExtensionDetails]) -> bool {
        let supported_extensions: Vec<_> = self
            .extension_properties
            .iter()
            .filter_map(|extension| extension.extension_name_as_c_str().ok())
            .collect();

        extensions.iter().all(|extension| {
            if let Some(version) = extension.promoted {
                if self.api_version >= version {
                    return true;
                }
            };

            supported_extensions.contains(&extension.name)
        })
    }

    /// Finds the indicies of the queues that support the requirements.
    pub fn find_queue_family_indicies(
        &self,
        flags: vk::QueueFlags,
        queue_count: u32,
    ) -> Vec<(u32, vk::QueueFamilyProperties)> {
        self.queue_families
            .iter()
            .enumerate()
            .filter_map(|(index, &properties)| {
                if properties.queue_count >= queue_count && properties.queue_flags.contains(flags) {
                    Some((index as u32, properties))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Finds the indicies of the queue families that support the requirements and support the surface.
    ///
    /// # Requires
    /// - `VK_KHR_surface` instance extension.
    pub fn find_queue_family_indicies_with_surface(
        &self,
        flags: vk::QueueFlags,
        queue_count: u32,
        surface_loader: &khr::surface::Instance,
        surface: &vk::SurfaceKHR,
    ) -> VkResult<Vec<(u32, vk::QueueFamilyProperties)>> {
        self.queue_families
            .iter()
            .enumerate()
            .filter_map(|(index, &properties)| {
                let valid_properties =
                    properties.queue_count >= queue_count && properties.queue_flags.contains(flags);

                if !valid_properties {
                    return None;
                }

                let surface_supported_result = unsafe {
                    surface_loader.get_physical_device_surface_support(
                        self.physical_device,
                        index as u32,
                        *surface,
                    )
                };

                let supported = match surface_supported_result {
                    Ok(supported) => supported,

                    Err(e) => return Some(Err(e)),
                };

                if supported {
                    Some(Ok((index as u32, properties)))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Display for PhysicalDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self
            .properties
            .device_name_as_c_str()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|_| std::borrow::Cow::Borrowed("Invalid Device Name"));

        let device_type = match self.properties.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU",
            vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU",
            vk::PhysicalDeviceType::CPU => "CPU",
            vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual CPU",
            vk::PhysicalDeviceType::OTHER => "Other",
            _ => "Unknown Device Type",
        };

        // Dedicated GPU: NVIDIA RTX 3060 (Vk 1.3.0)
        write!(f, "{}: {} (Vk {})", device_type, name, self.api_version)
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("vkEnumerateDeviceExtensionProperties call failed:\n{0}")]
    VkEnumerateDeviceExtensionProperties(#[source] vk::Result),

    #[error("vkEnumeratePhysicalDevices call failed:\n{0}")]
    VkEnumeratePhysicalDevices(#[source] vk::Result),
}
