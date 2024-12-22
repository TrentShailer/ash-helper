use std::sync::Arc;

use ash::{ext, vk};
use configuration::{Feature2Configuration, VulkanConfiguration};
use parking_lot::Mutex;

use crate::{QueueFamily, Version};

mod configuration;

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

    /// Gets the queue from the queue family.
    unsafe fn get_queue(&self, queue_index: usize) -> Option<Arc<Mutex<vk::Queue>>>;
}

// TODO given that I have separated these out, what about with window
// TODO etc.
// pub sturct VulkanInitializer?
// TODO how do I write the initialization such that it is reusable
// TODO I would like one function tbh and I don't like the generics.
// TODO is there any way to not have the generics? Box?
// TODO single function is definitly nicer.

pub unsafe fn initialize_vk_from_config_with_features2<Feat2Validator, Feat2Enabler>(
    entry: &ash::Entry,
    config: &VulkanConfiguration,
    features2: &Feature2Configuration<Feat2Validator, Feat2Enabler>,
) where
    Feat2Validator: Fn(&ash::Instance, vk::PhysicalDevice) -> bool,
    Feat2Enabler: Fn(&mut vk::PhysicalDeviceFeatures2),
{
}

pub unsafe fn initialize_vk_from_config(entry: &ash::Entry, config: &VulkanConfiguration) {
    // TODO validate entry for extensions

    let instance = {
        let app_info = vk::ApplicationInfo::default()
            .application_name(c"ash_heler::examples::compute")
            .application_version(Version::cargo_package_version().as_vulkan_version())
            .api_version(Version::V1_3.as_vulkan_version());

        let layer_names = if config.debug {
            let validation_layer_name = c"VK_LAYER_KHRONOS_validation";
            let profiles_layer_name = c"VK_LAYER_KHRONOS_profiles";
            vec![validation_layer_name.as_ptr(), profiles_layer_name.as_ptr()]
        } else {
            vec![]
        };

        // let mut extensions = config.ex;
        // TODO transform required extensions and optional extensions into extensions that should be requested

        // let extension_pointers: Vec<_> = extensions.into_iter().map(|ext| ext.as_ptr()).collect();

        // let instance_create_info = vk::InstanceCreateInfo::default()
        //     .application_info(&app_info)
        //     .enabled_layer_names(&layer_names)
        //     .enabled_extension_names(extension_pointers.as_slice());

        // TODO give name on debug
        // entry
        //     .create_instance(&instance_create_info, None)
        //     .map_err(|e| Error::VulkanCall(e, "vkCreateInstance"))?
    };
}
