use std::{borrow::Cow, ffi::CStr};

use ash::{ext, prelude::VkResult, vk};
use log::{debug, error, info, warn};

/// Registers Vulkan's debug utils and messenger to receive [`log`] messages from any Vulkan debug 
/// calls.
///
/// # Requires
/// - Instance extension `VK_EXT_debug_utils`.
pub fn register_debug_utils(
    entry: &ash::Entry,
    instance: &ash::Instance,
) -> VkResult<(ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)> {
    unsafe {
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
            .pfn_user_callback(Some(vulkan_debug_callback));

        let debug_utils_loader = ext::debug_utils::Instance::new(entry, instance);
        let debug_messenger = debug_utils_loader.create_debug_utils_messenger(&debug_info, None)?;

        Ok((debug_utils_loader, debug_messenger))
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;

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

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!("[{message_type:?}] [{message_id_name} ({message_id_number})] | {message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("[{message_type:?}] [{message_id_name} ({message_id_number})] | {message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("[{message_type:?}] [{message_id_name} ({message_id_number})] | {message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("[{message_type:?}] [{message_id_name} ({message_id_number})] | {message}")
        }

        _ => {
            info!("[{message_type:?}] [{message_id_name} ({message_id_number})] | {message}")
        }
    };

    vk::FALSE
}
