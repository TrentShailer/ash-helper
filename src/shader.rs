use std::io::{self, Cursor};

use ash::{util::read_spv, vk};
use thiserror::Error;

use crate::CoreVulkan;

/// Creates a shader module from some SPV bytes.
///
/// # Usage
/// ```
/// create_shader_module_from_spv(vk.as_ref(), include_bytes!("shaders/shader.spv")).unwrap();
/// ```
///
/// # Safety
/// - `bytes` **must** be valid SPV according to <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkShaderModuleCreateInfo.html>.
pub unsafe fn create_shader_module_from_spv<Vk>(
    vk: &Vk,
    bytes: &[u8],
) -> Result<vk::ShaderModule, Error>
where
    Vk: CoreVulkan,
{
    let mut cursor = Cursor::new(bytes);
    let shader_code = read_spv(&mut cursor).map_err(Error::ReadSpv)?;
    let shader_info = vk::ShaderModuleCreateInfo::default().code(&shader_code);
    let shader_module = vk
        .vk_device()
        .create_shader_module(&shader_info, None)
        .map_err(Error::VkCreateShaderModule)?;

    Ok(shader_module)
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("Failed to read SPV:\n{0}")]
    ReadSpv(#[source] io::Error),

    #[error("vkCreateShaderModule call failed:\n{0}")]
    VkCreateShaderModule(#[source] vk::Result),
}
