use core::slice;

use ash::vk;

use crate::{
    debug_utils::{queue_try_begin_label, queue_try_end_label, try_name},
    LabelledVkResult, VkError, VulkanContext,
};

/// Creates the resources to run a onetime command, waits for completion, then cleans up. Useful for
/// copies during setup. Works best when `command_pool` was created with the
/// `vk::CommandPoolCreateFlags::TRANSIENT`.
pub unsafe fn onetime_command<Vk, Fn>(
    vk: &Vk,
    command_pool: vk::CommandPool,
    queue_purpose: Vk::QueuePurpose,
    record_callback: Fn,
    label: &str,
) -> LabelledVkResult<()>
where
    Vk: VulkanContext,
    Fn: FnOnce(&ash::Device, vk::CommandBuffer),
{
    // Allocate command buffer
    let command_buffer = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe { vk.device().allocate_command_buffers(&allocate_info) }
            .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0]
    };

    // Recording
    {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        vk.device()
            .begin_command_buffer(command_buffer, &begin_info)
            .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;

        record_callback(vk.device(), command_buffer);

        unsafe { vk.device().end_command_buffer(command_buffer) }
            .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
    }

    // Create fence
    let fence = {
        let fence_info = vk::FenceCreateInfo::default();

        let fence = unsafe { vk.device().create_fence(&fence_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateFence"))?;

        try_name(vk, fence, label);

        fence
    };

    // Submit
    {
        let submit_info =
            vk::SubmitInfo::default().command_buffers(slice::from_ref(&command_buffer));

        let queue = vk.queue(queue_purpose).lock();
        queue_try_begin_label(vk, *queue, label);

        unsafe {
            vk.device()
                .queue_submit(*queue, slice::from_ref(&submit_info), fence)
                .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
        }

        queue_try_end_label(vk, *queue);
    }

    // Wait for submission to complete
    unsafe {
        vk.device()
            .wait_for_fences(slice::from_ref(&fence), true, u64::MAX)
            .map_err(|e| VkError::new(e, "vkWaitForFences"))?;
    }

    // Cleanup
    unsafe {
        vk.device().destroy_fence(fence, None);
        vk.device()
            .free_command_buffers(command_pool, slice::from_ref(&command_buffer))
    };

    Ok(())
}
