use core::slice;

use ash::vk;

use crate::VulkanContext;

/// Transitions an image from an existing layout to a new layout.
///
/// # Supported Layouts
/// * `PREINITIALIZED`
/// * `UNDEFINED`
/// * `COLOR_ATTACHMENT_OPTIMAL`
/// * `SHADER_READ_ONLY_OPTIMAL`
/// * `TRANSFER_DST_OPTIMAL`
/// * `TRANSFER_SRC_OPTIMAL`
/// * `GENERAL`
/// * `PRESENT_SRC_KHR`
pub unsafe fn cmd_transition_image<Vulkan: VulkanContext>(
    vulkan: &Vulkan,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Option<()> {
    let (src_stage, src_access) = pipeline_stage_access_tuple(old_layout)?;
    let (dst_stage, dst_access) = pipeline_stage_access_tuple(new_layout)?;

    let image_barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .src_access_mask(src_access)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .new_layout(new_layout)
        .dst_access_mask(dst_access)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_array_layer(0)
                .base_mip_level(0)
                .layer_count(1)
                .level_count(1),
        );

    vulkan.device().cmd_pipeline_barrier(
        command_buffer,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        slice::from_ref(&image_barrier),
    );

    Some(())
}

fn pipeline_stage_access_tuple(
    layout: vk::ImageLayout,
) -> Option<(vk::PipelineStageFlags, vk::AccessFlags)> {
    let stage = match layout {
        vk::ImageLayout::PREINITIALIZED => vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::ImageLayout::UNDEFINED => vk::PipelineStageFlags::TOP_OF_PIPE,

        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        }

        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
            vk::PipelineStageFlags::FRAGMENT_SHADER
                | vk::PipelineStageFlags::COMPUTE_SHADER
                | vk::PipelineStageFlags::VERTEX_SHADER
                | vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER
                | vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER
                | vk::PipelineStageFlags::GEOMETRY_SHADER
                | vk::PipelineStageFlags::TASK_SHADER_EXT
                | vk::PipelineStageFlags::MESH_SHADER_EXT
        }

        vk::ImageLayout::TRANSFER_DST_OPTIMAL => vk::PipelineStageFlags::TRANSFER,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => vk::PipelineStageFlags::TRANSFER,

        vk::ImageLayout::GENERAL => {
            vk::PipelineStageFlags::COMPUTE_SHADER | vk::PipelineStageFlags::TRANSFER
        }

        vk::ImageLayout::PRESENT_SRC_KHR => vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,

        _ => return None,
    };

    let access = match layout {
        vk::ImageLayout::PREINITIALIZED => vk::AccessFlags::NONE,
        vk::ImageLayout::UNDEFINED => vk::AccessFlags::NONE,

        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => {
            vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE
        }

        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => vk::AccessFlags::SHADER_READ,

        vk::ImageLayout::TRANSFER_DST_OPTIMAL => vk::AccessFlags::TRANSFER_WRITE,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => vk::AccessFlags::TRANSFER_READ,

        vk::ImageLayout::GENERAL => vk::AccessFlags::TRANSFER_READ,

        vk::ImageLayout::PRESENT_SRC_KHR => vk::AccessFlags::NONE,

        _ => return None,
    };

    Some((stage, access))
}
