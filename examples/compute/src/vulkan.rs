use ash::vk;
use ash_helper::{vulkan_debug_callback, DebugUtils, VulkanContext};
use vp_ash::vp;

pub struct Vulkan {
    entry: ash::Entry,
    _vp_entry: vp_ash::Entry,
    capabilities: vp_ash::Capabilities,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue_family_index: u32,
    queue: vk::Queue,
    debug_utils: Option<DebugUtils>,
}

impl Vulkan {
    pub fn new(try_debug: bool) -> Self {
        // Setup objects.
        let entry = ash::Entry::linked();
        let vp_entry = vp_ash::Entry::linked();
        let capabilities = {
            let create_info = vp::CapabilitiesCreateInfo::default()
                .api_version(vk::make_api_version(0, 1, 2, 198))
                .flags(vp::CapabilitiesCreateFlags::STATIC);

            unsafe { vp_entry.create_capabilities(&create_info, None) }.unwrap()
        };

        // Profiles for this application.
        let core_profile = vp::ProfileProperties::default()
            .profile_name(c"VP_EXAMPLE_compute")
            .unwrap()
            .spec_version(1);
        let debug_profile = vp::ProfileProperties::default()
            .profile_name(c"VP_EXAMPLE_compute_debug")
            .unwrap()
            .spec_version(1);

        // Sanity check that profiles are present, if the instance in the build environment is missing
        // the required extensions, this will fail.
        {
            let profiles = unsafe { capabilities.get_profiles() }.unwrap();

            assert!(
                profiles.contains(&core_profile),
                "The build environment does not support the profiles."
            );
            assert!(
                profiles.contains(&debug_profile),
                "The build environment does not support the profiles."
            );
        };

        // Check for instance support.
        let supports_instance =
            unsafe { capabilities.get_instance_profile_support(None, &core_profile) }.unwrap();
        if !supports_instance {
            panic!("Your Vulkan Instance does not meet the requirements to run this application. Try updating your drivers.")
        }

        // If the instance supports debug and debug is wanted, then we should debug.
        let should_debug = {
            let supports_debug =
                unsafe { capabilities.get_instance_profile_support(None, &debug_profile) }.unwrap();

            try_debug && supports_debug
        };

        // Create the list of profiles to use.
        let mut enabled_profiles = vec![core_profile];
        if should_debug {
            enabled_profiles.push(debug_profile);
        }

        // Create instance.
        let instance = {
            let api_version = unsafe { capabilities.get_profile_api_version(&core_profile) };

            let app_info = vk::ApplicationInfo::default()
                .api_version(api_version)
                .application_name(c"Compute Example");

            let vk_create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

            let vp_create_info = vp::InstanceCreateInfo::default()
                .create_info(&vk_create_info)
                .enabled_full_profiles(&enabled_profiles);

            unsafe { capabilities.create_instance(&entry, &vp_create_info, None) }.unwrap()
        };

        // Select a physical device.
        let physical_device = {
            unsafe { instance.enumerate_physical_devices() }
            .unwrap()
            .into_iter()
            .filter(|&device| unsafe {
                let supported = capabilities
                    .get_physical_device_profile_support(&instance, device, &core_profile)
                    .unwrap();
                if !supported{
                    return  false;
                }

                let queue_properties =
                    instance.get_physical_device_queue_family_properties(device);

                queue_properties
                    .into_iter()
                    .any(| properties| {
                        properties.queue_count >= 1
                                && properties.queue_flags.contains(vk::QueueFlags::COMPUTE)
                    })
            }).min_by_key(|&device| {
                let properties = unsafe {instance.get_physical_device_properties(device)};

                match properties.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                    vk::PhysicalDeviceType::CPU => 3,
                    vk::PhysicalDeviceType::OTHER => 4,
                    _ => 5,
                }
            }).expect("No GPU in your system meets the requirements to run this application. Try updating your drivers.")
        };

        // Get the queue family index.
        let queue_family_index = {
            let queue_properties =
                unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

            queue_properties
                .into_iter()
                .position(|properties| {
                    properties.queue_count >= 1
                        && properties.queue_flags.contains(vk::QueueFlags::COMPUTE)
                })
                .unwrap() as u32
        };

        // Create logical device.
        let device = {
            let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_family_index)
                .queue_priorities(&[1.0; 1])];

            let vk_create_info =
                vk::DeviceCreateInfo::default().queue_create_infos(&queue_create_infos);

            let vp_create_info = vp::DeviceCreateInfo::default()
                .create_info(&vk_create_info)
                .enabled_full_profiles(&enabled_profiles);

            unsafe { capabilities.create_device(&instance, physical_device, &vp_create_info, None) }
                .unwrap()
        };

        // Retrieve the queue.
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        // Create debug utils if we should debug
        let debug_utils = if should_debug {
            let debug_utils =
                unsafe { DebugUtils::new(&entry, &instance, &device, Some(vulkan_debug_callback)) }
                    .unwrap();
            Some(debug_utils)
        } else {
            None
        };

        Self {
            entry,
            _vp_entry: vp_entry,
            capabilities,
            instance,
            physical_device,
            device,
            queue_family_index,
            queue,
            debug_utils,
        }
    }
}

impl VulkanContext for Vulkan {
    type QueuePurpose = ();

    #[inline]
    unsafe fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    #[inline]
    unsafe fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    #[inline]
    unsafe fn device(&self) -> &ash::Device {
        &self.device
    }

    #[inline]
    unsafe fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    #[inline]
    unsafe fn debug(&self) -> Option<&ash::ext::debug_utils::Device> {
        if let Some(debug_utils) = self.debug_utils.as_ref() {
            Some(&debug_utils.device)
        } else {
            None
        }
    }

    #[inline]
    fn queue_family_index(&self, _purpose: Self::QueuePurpose) -> Option<u32> {
        Some(self.queue_family_index)
    }

    #[inline]
    unsafe fn queue(&self, _purpose: Self::QueuePurpose) -> Option<vk::Queue> {
        Some(self.queue)
    }
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            if let Some(debug_utils) = self.debug_utils.as_ref() {
                debug_utils
                    .instance
                    .destroy_debug_utils_messenger(debug_utils.messenger, None);
            }
            self.instance.destroy_instance(None);
            self.capabilities.destroy_capabilities(None);
        }
    }
}
