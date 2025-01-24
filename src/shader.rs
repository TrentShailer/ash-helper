use ash::{util::read_spv, vk};

use crate::{LabelledVkResult, VkError, VulkanContext};

/// Creates a shader module from some SPV bytes.
///
/// # Panics
/// - If the `read_spv` call fails on `bytes`.
///
/// # Safety
/// - `bytes` **must** be valid SPV according to <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkShaderModuleCreateInfo.html>.
pub unsafe fn create_shader_module_from_spv<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    bytes: &[u8],
) -> LabelledVkResult<vk::ShaderModule> {
    let mut cursor = std::io::Cursor::new(bytes);
    let shader_code = read_spv(&mut cursor).expect("Failed to read spv");

    let shader_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);
    let shader_module = unsafe { vulkan.device().create_shader_module(&shader_info, None) }
        .map_err(|e| VkError::new(e, "vkCreateShaderModule"))?;

    Ok(shader_module)
}
