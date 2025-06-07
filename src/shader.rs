use ash::{ext, util::read_spv, vk};

use crate::{Context, LabelledVkResult, VK_GLOBAL_ALLOCATOR, VkError, VulkanContext, try_name};

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
    let shader_module = unsafe {
        vulkan
            .device()
            .create_shader_module(&shader_info, VK_GLOBAL_ALLOCATOR.as_deref())
    }
    .map_err(|e| VkError::new(e, "vkCreateShaderModule"))?;

    Ok(shader_module)
}

/// Creates linked shader objects, cleaning up any created shader objects on failure.
///
/// `next_stage` and `flags` are set automatically to link the shaders correctly.
///
/// Each shader is named: `{name} {stage:?} SHADER`. E.g., `MAXIMUM REDUCTION COMPUTE SHADER`.
pub unsafe fn link_shader_objects<Vulkan>(
    vulkan: &Vulkan,
    create_infos: &mut [vk::ShaderCreateInfoEXT<'_>],
    name: &str,
) -> Result<Vec<vk::ShaderEXT>, vk::Result>
where
    Vulkan: Context<ext::shader_object::Device>,
{
    // To set next_stage correctly, the following create_info is also required
    let mut iter = create_infos.iter_mut().peekable();
    while let Some(create_info) = iter.next() {
        let linked_create_info =
            create_info.flags(create_info.flags | vk::ShaderCreateFlagsEXT::LINK_STAGE);

        // Add next stage
        let linked_create_info = if let Some(next) = iter.peek() {
            linked_create_info.next_stage(next.stage)
        } else {
            linked_create_info
        };

        *create_info = linked_create_info;
    }

    unsafe { create_shader_objects(vulkan, create_infos, name) }
}

/// Creates shader objects, cleaning up any created shader objects on failure.
///
/// Each shader is named: `{name} {stage:?} SHADER`. E.g., `MAXIMUM REDUCTION COMPUTE SHADER`.
pub unsafe fn create_shader_objects<Vulkan>(
    vulkan: &Vulkan,
    create_infos: &[vk::ShaderCreateInfoEXT<'_>],
    name: &str,
) -> Result<Vec<vk::ShaderEXT>, vk::Result>
where
    Vulkan: Context<ext::shader_object::Device>,
{
    let device: &ext::shader_object::Device = unsafe { vulkan.context() };

    // Create the shaders
    let shaders =
        match unsafe { device.create_shaders(create_infos, VK_GLOBAL_ALLOCATOR.as_deref()) } {
            Ok(shaders) => shaders,

            Err((shaders, error)) => {
                // Cleanup any created shaders
                shaders.into_iter().for_each(|shader| unsafe {
                    device.destroy_shader(shader, VK_GLOBAL_ALLOCATOR.as_deref())
                });

                return Err(error);
            }
        };

    // Name the shaders
    shaders.iter().enumerate().for_each(|(index, shader)| {
        let info = create_infos[index];
        let stage = info.stage;

        unsafe { try_name(vulkan, *shader, &format!("{name} {stage:?} SHADER")) };
    });

    Ok(shaders)
}
