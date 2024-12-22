use std::ffi::CStr;

use ash::vk;

pub use extension::VkExtension;

mod extension;

/// Generates a Validator and Enabler closure for validating and enabing PhysicalDeviceFeature2s.
///
/// - `feat` feature type.
/// - `name` unique name for the feature.
/// - `require` the flag(s) that are required, can be repeated.
///
/// ## Example
/// ```
/// features2! {
///     feat vk::PhysicalDevice16BitStorageFeatures,
///     name storage16bit,
///     require storage_buffer16_bit_access,
///     require uniform_and_storage_buffer16_bit_access;
///
///     feat vk::PhysicalDeviceSynchronization2Features,
///     name synchronization2,
///     require synchronization2
/// };
/// ```
#[macro_export]
macro_rules! features2 {
    ($(feat $struc:ty, name $name:ident, $(require $feat:ident),+);+) => {
    (
        // Validator
        |instance: &ash::Instance, device: ash::vk::PhysicalDevice| {
            $(
                let mut $name = <$struc>::default()$(.$feat(true))+;
            )+

            let mut device_features = ash::vk::PhysicalDeviceFeatures2::default()
            $(
                .push_next(&mut $name)
            )+;

            unsafe { instance.get_physical_device_features2(device, &mut device_features) };

            $(
                $(
                    $name.$feat == ash::vk::TRUE &&
                )+
            )+ true
        },
        // Enabler
        |features: &mut ash::vk::PhysicalDeviceFeatures2| {
            $(
                let $name = Box::leak(Box::new(
                    <$struc>::default()
                    $(
                        .$feat(true)
                    )+
                ));
            )+

            *features = features
            $(
                .push_next($name)
            )+;
        }
    )
    };
}

#[derive(Default)]
#[non_exhaustive]
pub struct VulkanConfiguration {
    pub app_name: &'static CStr,

    pub debug: bool,

    pub required_instance_extensions: Vec<VkExtension>,
    pub optional_instance_extensions: Vec<VkExtension>,

    pub required_device_extensions: Vec<VkExtension>,
    pub optional_device_extensions: Vec<VkExtension>,

    pub queue_flags: vk::QueueFlags,
    pub queue_count: u32,

    pub features: vk::PhysicalDeviceFeatures,
}

pub struct Feature2Configuration<Feat2Validator, Feat2Enabler>
where
    Feat2Validator: Fn(&ash::Instance, vk::PhysicalDevice) -> bool,
    Feat2Enabler: Fn(&mut vk::PhysicalDeviceFeatures2),
{
    /// The PhysicalDeviceFeature2s to request.
    ///
    /// Use the `features2!` macro
    ///
    /// ## Example
    /// ```
    /// features2! {
    ///     feat vk::PhysicalDevice16BitStorageFeatures,
    ///     name storage16bit,
    ///     require storage_buffer16_bit_access,
    ///     require uniform_and_storage_buffer16_bit_access;
    ///
    ///     feat vk::PhysicalDeviceSynchronization2Features,
    ///     name synchronization2,
    ///     require synchronization2
    /// };
    /// ```
    pub features2: (Feat2Validator, Feat2Enabler),
}

impl<Feat2Validator, Feat2Enabler> Feature2Configuration<Feat2Validator, Feat2Enabler>
where
    Feat2Validator: Fn(&ash::Instance, vk::PhysicalDevice) -> bool,
    Feat2Enabler: Fn(&mut vk::PhysicalDeviceFeatures2),
{
    /// The PhysicalDeviceFeature2s to request.
    ///
    /// Use the `features2!` macro
    ///
    /// ## Example
    /// ```
    /// features2! {
    ///     feat vk::PhysicalDevice16BitStorageFeatures,
    ///     name storage16bit,
    ///     require storage_buffer16_bit_access,
    ///     require uniform_and_storage_buffer16_bit_access;
    ///
    ///     feat vk::PhysicalDeviceSynchronization2Features,
    ///     name synchronization2,
    ///     require synchronization2
    /// };
    /// ```
    pub fn new(features2: (Feat2Validator, Feat2Enabler)) -> Self {
        Self { features2 }
    }
}

impl VulkanConfiguration {
    pub fn app_name(mut self, name: &'static CStr) -> Self {
        self.app_name = name;
        self
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn required_instance_extensions(mut self, extensions: Vec<VkExtension>) -> Self {
        self.required_instance_extensions = extensions;
        self
    }

    pub fn optional_instance_extensions(mut self, extensions: Vec<VkExtension>) -> Self {
        self.optional_instance_extensions = extensions;
        self
    }

    pub fn required_device_extensions(mut self, extensions: Vec<VkExtension>) -> Self {
        self.required_device_extensions = extensions;
        self
    }

    pub fn optional_device_extensions(mut self, extensions: Vec<VkExtension>) -> Self {
        self.optional_device_extensions = extensions;
        self
    }

    pub fn queue_flags(mut self, flags: vk::QueueFlags) -> Self {
        self.queue_flags = flags;
        self
    }

    pub fn queue_count(mut self, count: u32) -> Self {
        self.queue_count = count;
        self
    }

    pub fn features(mut self, features: vk::PhysicalDeviceFeatures) -> Self {
        self.features = features;
        self
    }
}
