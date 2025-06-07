use core::slice;

use ash::vk;

use crate::{
    LabelledVkResult, MaybeMutex, VK_GLOBAL_ALLOCATOR, VkError, VulkanContext,
    debug_utils::{queue_try_begin_label, queue_try_end_label, try_name},
};

/// Creates the resources to run a onetime command, waits for completion, then cleans up. Useful for
/// copies during setup. Works best when `command_pool` was created with the
/// `vk::CommandPoolCreateFlags::TRANSIENT`.
pub unsafe fn onetime_command<'m, Vulkan, CmdFn, Queue, Pool>(
    vulkan: &Vulkan,
    command_pool: Pool,
    queue: Queue,
    cmd_fn: CmdFn,
    label: &str,
) -> LabelledVkResult<()>
where
    Vulkan: VulkanContext,
    CmdFn: FnOnce(&Vulkan, vk::CommandBuffer),
    Queue: Into<MaybeMutex<'m, vk::Queue>>,
    Pool: Into<MaybeMutex<'m, vk::CommandPool>>,
{
    let maybe_mutex_pool = command_pool.into();
    let (pool, pool_guard) = maybe_mutex_pool.lock();

    // Allocate command buffer
    let command_buffer = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe { vulkan.device().allocate_command_buffers(&allocate_info) }
            .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0]
    };

    // Recording
    {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            vulkan
                .device()
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;
        }

        cmd_fn(vulkan, command_buffer);

        unsafe { vulkan.device().end_command_buffer(command_buffer) }
            .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
    }

    // Create fence
    let fence = {
        let fence_info = vk::FenceCreateInfo::default();

        let fence = unsafe {
            vulkan
                .device()
                .create_fence(&fence_info, VK_GLOBAL_ALLOCATOR.as_deref())
        }
        .map_err(|e| VkError::new(e, "vkCreateFence"))?;

        unsafe { try_name(vulkan, fence, label) };

        fence
    };

    // Submit
    {
        let submit_info =
            vk::SubmitInfo::default().command_buffers(slice::from_ref(&command_buffer));

        let (queue, _queue_guard) = queue.into().lock();

        unsafe { queue_try_begin_label(vulkan, queue, label) };

        unsafe {
            vulkan
                .device()
                .queue_submit(queue, slice::from_ref(&submit_info), fence)
                .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
        }

        unsafe { queue_try_end_label(vulkan, queue) };
    }

    #[allow(unused)]
    let pool = (); // Shadow pool to prevent usage after guard drop.
    drop(pool_guard);

    // Wait for submission to complete
    unsafe {
        vulkan
            .device()
            .wait_for_fences(slice::from_ref(&fence), true, u64::MAX)
            .map_err(|e| VkError::new(e, "vkWaitForFences"))?;
    }

    // Cleanup
    unsafe {
        vulkan
            .device()
            .destroy_fence(fence, VK_GLOBAL_ALLOCATOR.as_deref());

        let (pool, _pool_guard) = maybe_mutex_pool.lock();
        vulkan
            .device()
            .free_command_buffers(pool, slice::from_ref(&command_buffer))
    };

    Ok(())
}
