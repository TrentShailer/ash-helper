use core::ffi::CStr;

use ash::{ext, vk};
use tracing::{debug, error, info, warn};

use crate::{LabelledVkResult, VK_GLOBAL_ALLOCATOR, VkError, VulkanContext};

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkSetDebugUtilsObjectNameEXT.html
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

/// https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkSetDebugUtilsObjectNameEXT.html
pub unsafe fn try_name_all<Vulkan, H>(vulkan: &Vulkan, handles: &[H], name: &str)
where
    Vulkan: VulkanContext,
    H: vk::Handle + Copy,
{
    if let Some(device) = unsafe { vulkan.debug() } {
        for (index, handle) in handles.iter().enumerate() {
            let name = alloc::format!("{name}_{index}\0");

            let name_info = vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(*handle)
                .object_name(CStr::from_bytes_until_nul(name.as_bytes()).unwrap());

            if let Err(e) = unsafe { device.set_debug_utils_object_name(&name_info) } {
                warn!("Failed to set the object name {name}:\n{e}");
            }
        }
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdBeginDebugUtilsLabelEXT.html>
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
pub unsafe fn cmd_try_end_label<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    command_buffer: vk::CommandBuffer,
) {
    if let Some(device) = unsafe { vulkan.debug() } {
        unsafe { device.cmd_end_debug_utils_label(command_buffer) };
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCmdInsertDebugUtilsLabelEXT.html>
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

    /// The Debug Utils Device.
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

        let messenger = unsafe {
            instance.create_debug_utils_messenger(&debug_info, VK_GLOBAL_ALLOCATOR.as_deref())
        }
        .map_err(|e| VkError::new(e, "vkCreateDebugUtilsMessengerEXT"))?;

        let device = ext::debug_utils::Device::new(vk_instance, vk_device);

        Ok(Self {
            instance,
            messenger,
            device,
        })
    }
}

/// Represents the data from a `vk::DebugUtilsMessengerCallbackDataEXT` with nice display.
#[derive(Debug)]
pub struct DebugMessage<'callback> {
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    id: i32,
    vuid: &'callback CStr,
    message: &'callback CStr,
    queue_labels: &'callback [vk::DebugUtilsLabelEXT<'callback>],
    command_buffer_labels: &'callback [vk::DebugUtilsLabelEXT<'callback>],
    object_names: &'callback [vk::DebugUtilsObjectNameInfoEXT<'callback>],
}

impl<'callback> DebugMessage<'callback> {
    /// Tries to convert a pointer to a `vk::DebugUtilsMessengerCallbackDataEXT` into self.
    ///
    /// # Safety
    /// * `p_callback_data` **MUST** be safe to dereference if it is not a null pointer.
    /// * All pointers in `DebugUtilsMessengerCallbackDataEXT` **MUST** be safe to dereference.
    /// * All `*const c_char` elements **MUST** be null terminated and safe to dereference.
    pub unsafe fn try_from(
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'callback>,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    ) -> Option<Self> {
        if p_callback_data.is_null() {
            return None;
        }

        let callback_data = unsafe { *p_callback_data };

        let id = callback_data.message_id_number;

        let message = if !callback_data.p_message.is_null() {
            unsafe { CStr::from_ptr(callback_data.p_message) }
        } else {
            c""
        };

        let vuid = if !callback_data.p_message_id_name.is_null() {
            unsafe { CStr::from_ptr(callback_data.p_message_id_name) }
        } else {
            c""
        };

        let queue_labels = unsafe {
            core::slice::from_raw_parts(
                callback_data.p_queue_labels,
                callback_data.queue_label_count as usize,
            )
        };

        let command_buffer_labels = unsafe {
            core::slice::from_raw_parts(
                callback_data.p_cmd_buf_labels,
                callback_data.cmd_buf_label_count as usize,
            )
        };

        let object_names = unsafe {
            core::slice::from_raw_parts(
                callback_data.p_objects,
                callback_data.object_count as usize,
            )
        };

        Some(Self {
            message_type,
            id,
            vuid,
            message,
            queue_labels,
            command_buffer_labels,
            object_names,
        })
    }
}

impl core::fmt::Display for DebugMessage<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Shader debug `printf`
        if self.id == 1985515673 {
            return write!(f, "{}", self.message.to_string_lossy());
        }

        // `[Vulkan validation] VUID-vkCmdSetScissor-firstScissor-00593`
        writeln!(
            f,
            "[Vulkan {}] {}",
            format!("{:?}", self.message_type).to_lowercase(),
            self.vuid.to_string_lossy()
        )?;

        // Message
        writeln!(f, "    {}", self.message.to_string_lossy())?;

        // Objects
        if !self.object_names.is_empty() {
            writeln!(f, "Objects:")?;

            let mut seen_objects = Vec::new();
            for object in self.object_names {
                if seen_objects.contains(&object.object_handle) {
                    continue;
                }
                seen_objects.push(object.object_handle);

                let name = unsafe { object.object_name_as_c_str() }.unwrap_or(c"");

                // `    [QUEUE] COMPUTE (0x5c11921d2d10)`
                writeln!(
                    f,
                    "    [{:?}] {} (0x{:0x})",
                    object.object_type,
                    name.to_string_lossy(),
                    object.object_handle
                )?;
            }
        }

        // Queue labels
        if !self.queue_labels.is_empty() {
            writeln!(f, "Queue labels:")?;

            let mut previous_label = c"";
            for queue in self.queue_labels.iter().rev() {
                let label = unsafe { queue.label_name_as_c_str() }.unwrap_or(c"");

                if previous_label == label {
                    continue;
                }
                previous_label = label;

                // `    HDR Scanner Read Result`
                writeln!(f, "    {}", label.to_string_lossy())?;
            }
        }

        // Command buffer labels
        if !self.command_buffer_labels.is_empty() {
            writeln!(f, "Command buffer labels:")?;

            let mut previous_label = c"";
            for command_buffer in self.command_buffer_labels.iter().rev() {
                let label = unsafe { command_buffer.label_name_as_c_str() }.unwrap_or(c"");

                if previous_label == label {
                    continue;
                }
                previous_label = label;

                // `    HdrScanner::scan`
                writeln!(f, "    {}", label.to_string_lossy())?;
            }
        }

        Ok(())
    }
}

/// Default messenger for Debug Utils.
pub unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut core::ffi::c_void,
) -> vk::Bool32 {
    let Some(message) = (unsafe { DebugMessage::try_from(p_callback_data, message_type) }) else {
        return vk::FALSE;
    };

    // Shader debug `printf`
    if message.id == 1985515673 {
        debug!("{message}",);
        return vk::FALSE;
    }

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            debug!("{message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("{message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("{message}")
        }

        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("{message}")
        }

        _ => {
            info!("{message}")
        }
    };

    vk::FALSE
}
