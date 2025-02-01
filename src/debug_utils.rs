use alloc::borrow::Cow;
use core::ffi::CStr;

use ash::{ext, vk};
use tracing::{debug, error, info, warn};

use crate::{LabelledVkResult, VkError, VulkanContext};

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkSetDebugUtilsObjectNameEXT.html
#[inline]
pub unsafe fn try_name<Vulkan, H>(vulkan: &Vulkan, handle: H, name: &str)
where
    Vulkan: VulkanContext,
    H: vk::Handle,
{
    if let Some(device) = unsafe { vulkan.debug() } {
        let name = alloc::format!("{name}\0");

        let name_info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(handle)
            .object_name(CStr::from_bytes_until_nul(name.as_bytes()).unwrap());

        if let Err(e) = unsafe { device.set_debug_utils_object_name(&name_info) } {
            warn!("Failed to set the object name {name}:\n{e}");
        }
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdBeginDebugUtilsLabelEXT.html>
#[inline]
pub unsafe fn cmd_try_begin_label<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    command_buffer: vk::CommandBuffer,
    label: &str,
) {
    if let Some(device) = unsafe { vulkan.debug() } {
        let label = alloc::format!("{label}\0");

        let label_info = vk::DebugUtilsLabelEXT::default()
            .label_name(CStr::from_bytes_until_nul(label.as_bytes()).unwrap());

        unsafe { device.cmd_begin_debug_utils_label(command_buffer, &label_info) };
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdEndDebugUtilsLabelEXT.html>
#[inline]
pub unsafe fn cmd_try_end_label<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    command_buffer: vk::CommandBuffer,
) {
    if let Some(device) = unsafe { vulkan.debug() } {
        unsafe { device.cmd_end_debug_utils_label(command_buffer) };
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdInsertDebugUtilsLabelEXT.html>
#[inline]
pub unsafe fn cmd_try_insert_label<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    command_buffer: vk::CommandBuffer,
    label: &str,
) {
    if let Some(device) = unsafe { vulkan.debug() } {
        let label = alloc::format!("{label}\0");

        let label_info = vk::DebugUtilsLabelEXT::default()
            .label_name(CStr::from_bytes_until_nul(label.as_bytes()).unwrap());

        unsafe { device.cmd_insert_debug_utils_label(command_buffer, &label_info) };
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkQueueBeginDebugUtilsLabelEXT.html>
#[inline]
pub unsafe fn queue_try_begin_label<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    queue: vk::Queue,
    label: &str,
) {
    if let Some(device) = unsafe { vulkan.debug() } {
        let label = alloc::format!("{label}\0");

        let label_info = vk::DebugUtilsLabelEXT::default()
            .label_name(CStr::from_bytes_until_nul(label.as_bytes()).unwrap());

        unsafe { device.queue_begin_debug_utils_label(queue, &label_info) };
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkQueueInsertDebugUtilsLabelEXT.html>
#[inline]
pub unsafe fn queue_try_insert_label<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    queue: vk::Queue,
    label: &str,
) {
    if let Some(device) = unsafe { vulkan.debug() } {
        let label = alloc::format!("{label}\0");

        let label_info = vk::DebugUtilsLabelEXT::default()
            .label_name(CStr::from_bytes_until_nul(label.as_bytes()).unwrap());

        unsafe { device.queue_insert_debug_utils_label(queue, &label_info) };
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkQueueEndDebugUtilsLabelEXT.html>
#[inline]
pub unsafe fn queue_try_end_label<Vulkan: VulkanContext>(vulkan: &Vulkan, queue: vk::Queue) {
    if let Some(device) = unsafe { vulkan.debug() } {
        unsafe { device.queue_end_debug_utils_label(queue) };
    }
}

/// Wrapper around `VK_EXT_debug_utils` objects for debugging.
pub struct DebugUtils {
    /// The Debug Utils Instance.
    pub instance: ext::debug_utils::Instance,

    /// The Debug Utils Messenger.
    pub messenger: vk::DebugUtilsMessengerEXT,

    /// The Debug Utils Devoce.
    pub device: ext::debug_utils::Device,
}

impl DebugUtils {
    /// Registers Vulkan's debug utils and messenger to receive [`log`] messages from any Vulkan
    /// debug calls.
    pub unsafe fn new(
        entry: &ash::Entry,
        vk_instance: &ash::Instance,
        vk_device: &ash::Device,
        message_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
    ) -> LabelledVkResult<Self> {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(message_callback);

        let instance = ext::debug_utils::Instance::new(entry, vk_instance);

        let messenger = unsafe { instance.create_debug_utils_messenger(&debug_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateDebugUtilsMessengerEXT"))?;

        let device = ext::debug_utils::Device::new(vk_instance, vk_device);

        Ok(Self {
            instance,
            messenger,
            device,
        })
    }
}

/// Default messenger for Debug Utils.
pub unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut core::ffi::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let _message_id_number = callback_data.message_id_number;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    // Debug
    if callback_data.message_id_number == 1985515673 {
        debug!("{}", message);
        return vk::FALSE;
    }

    let message = message.replace(" | ", "\n");

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!("[{message_type:?}] [{message_id_name}]\n{message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("[{message_type:?}] [{message_id_name}]\n{message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("[{message_type:?}] [{message_id_name}]\n{message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("[{message_type:?}] [{message_id_name}]\n{message}")
        }

        _ => {
            info!("[{message_type:?}] [{message_id_name}]\n{message}")
        }
    };

    vk::FALSE
}
