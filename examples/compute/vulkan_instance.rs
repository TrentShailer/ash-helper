use std::sync::Arc;

use ash::{ext, khr, vk};
use ash_helper::{
    Buffer, CommandBuffer, DebugUtils, DeviceRequirement, EntryRequirement, QueueFamily,
    QueueFamilyRequirement, QueueFamilyRequirements, RequiredExtension, RequiredFeatures2,
    ValidationError, ValidationOutcome, Version,
};
use log::{error, info, warn};
use parking_lot::Mutex;
use thiserror::Error;

pub struct Vk {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue_family: QueueFamily,
    debug_utils: (ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT),
}

impl Vk {
    pub fn new() -> Result<Self, Error> {
        let entry = unsafe { ash::Entry::load().map_err(Error::LoadVulkanEntry)? };

        let entry_validation_outcome = ValidationOutcome::Valid
            .join(DebugUtils::validate_entry(&entry)?)
            .join(CommandBuffer::validate_entry(&entry)?);
        if entry_validation_outcome.is_invalid() {
            error!("Vulkan Instance {}", entry_validation_outcome);
            panic!("Vulkan Instance {}", entry_validation_outcome);
        }

        let instance = unsafe {
            let app_info = vk::ApplicationInfo::default()
                .application_name(c"ash_heler::examples::compute")
                .application_version(Version::cargo_package_version().as_vulkan_version())
                .api_version(Version::V1_3.as_vulkan_version());

            let validation_layer_name = c"VK_LAYER_KHRONOS_validation";
            let profiles_layer_name = c"VK_LAYER_KHRONOS_profiles";
            let layer_names = [validation_layer_name.as_ptr(), profiles_layer_name.as_ptr()];

            let mut extensions = vec![];
            extensions.append(&mut DebugUtils::required_instance_extensions(&entry));
            extensions.append(&mut CommandBuffer::required_instance_extensions(&entry));
            let extension_pointers: Vec<_> =
                extensions.into_iter().map(|ext| ext.as_ptr()).collect();

            let instance_create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_layer_names(&layer_names)
                .enabled_extension_names(extension_pointers.as_slice());

            entry
                .create_instance(&instance_create_info, None)
                .map_err(|e| Error::VulkanCall(e, "vkCreateInstance"))?
        };

        let debug_utils = DebugUtils::register(&entry, &instance)
            .map_err(|e| Error::VulkanCall(e, "Register Debug Utils"))?;

        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(|e| Error::VulkanCall(e, "vkEnumeratePhysicalDevices"))?
        };

        let queue_family_requirements =
            QueueFamilyRequirements::default() + CommandBuffer::queue_family_requirements();

        let maybe_physical_device = physical_devices
            .into_iter()
            .filter(|&device| {
                let validation_outcome = ValidationOutcome::Valid
                    .join(Buffer::validate_device(&instance, device).unwrap())
                    .join(CommandBuffer::validate_device(&instance, device).unwrap())
                    .join(
                        queue_family_requirements
                            .validate_device(&instance, device)
                            .unwrap(),
                    );

                if validation_outcome.is_invalid() {
                    warn!(
                        "{}{}",
                        ValidationOutcome::display_physical_device(&instance, device),
                        validation_outcome
                    );
                    return false;
                } else {
                    info!(
                        "{}{}",
                        ValidationOutcome::display_physical_device(&instance, device),
                        validation_outcome
                    );
                }

                true
            })
            .min_by_key(|&device| {
                let properties = unsafe { instance.get_physical_device_properties(device) };

                match properties.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                    _ => 2,
                }
            });

        let Some(physical_device) = maybe_physical_device else {
            return Err(Error::NoSuitableDevices);
        };

        let (queue_family_index, queue_family_properties) = queue_family_requirements
            .get_first_queue_family(&instance, physical_device)
            .unwrap();

        // Vk 1.1+
        let device = unsafe {
            // Extensions
            let mut extensions = vec![];
            let portability = RequiredExtension::new(khr::portability_subset::NAME);
            if RequiredExtension::validate_device(&[portability], &instance, physical_device)
                .unwrap()
                .is_valid()
            {
                extensions.push(khr::portability_subset::NAME);
            }
            extensions.append(&mut Buffer::required_device_extensions(
                &instance,
                physical_device,
            ));
            extensions.append(&mut CommandBuffer::required_device_extensions(
                &instance,
                physical_device,
            ));
            let extension_pointers: Vec<_> =
                extensions.into_iter().map(|ext| ext.as_ptr()).collect();

            // Features
            let mut features = vk::PhysicalDeviceFeatures2::default();
            CommandBuffer::set_required_features2(&mut features);
            CommandBuffer::set_required_features2(&mut features);

            // Queues
            let queue_priorities = vec![1.0; queue_family_requirements.queue_count as usize];
            let queue_info = vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&queue_priorities);

            // Device
            let device_create_info = vk::DeviceCreateInfo::default()
                .queue_create_infos(std::slice::from_ref(&queue_info))
                .enabled_extension_names(&extension_pointers)
                .push_next(&mut features);

            instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(|e| Error::VulkanCall(e, "vkCreateDevice"))?
        };

        let queue_family = unsafe {
            let queues: Vec<_> = (0..queue_family_requirements.queue_count)
                .map(|index| {
                    Arc::new(Mutex::new(
                        device.get_device_queue(queue_family_index, index),
                    ))
                })
                .collect();

            QueueFamily {
                index: queue_family_index,
                flags: queue_family_properties.queue_flags,
                queues,
            }
        };

        Ok(Self {
            entry,
            instance,
            physical_device,
            device,
            queue_family,
            debug_utils,
        })
    }
}

impl Drop for Vk {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.debug_utils
                .0
                .destroy_debug_utils_messenger(self.debug_utils.1, None);
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("LoadVulkanEntry call failed:\n{0}")]
    LoadVulkanEntry(#[source] ash::LoadingError),

    #[error("Validation call failed:\n{0}")]
    ValidationFailure(#[from] ValidationError),

    #[error("{1} call failed:\n{0}")]
    VulkanCall(#[source] vk::Result, &'static str),

    #[error("No suitable physical devices are available.")]
    NoSuitableDevices,
}
