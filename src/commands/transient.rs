use alloc::sync::Arc;
use core::slice;
use log::error;
use parking_lot::Mutex;
use std::thread;

use ash::vk;

use crate::{
    debug_utils::{queue_try_begin_label, queue_try_end_label, try_name},
    LabelledVkResult, MaybeMutex, VkError, VulkanContext,
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
        vulkan
            .device()
            .begin_command_buffer(command_buffer, &begin_info)
            .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;

        cmd_fn(vulkan, command_buffer);

        unsafe { vulkan.device().end_command_buffer(command_buffer) }
            .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
    }

    // Create fence
    let fence = {
        let fence_info = vk::FenceCreateInfo::default();

        let fence = unsafe { vulkan.device().create_fence(&fence_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateFence"))?;

        try_name(vulkan, fence, label);

        fence
    };

    // Submit
    {
        let submit_info =
            vk::SubmitInfo::default().command_buffers(slice::from_ref(&command_buffer));

        let (queue, _queue_guard) = queue.into().lock();

        queue_try_begin_label(vulkan, queue, label);

        unsafe {
            vulkan
                .device()
                .queue_submit(queue, slice::from_ref(&submit_info), fence)
                .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
        }

        queue_try_end_label(vulkan, queue);
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
        vulkan.device().destroy_fence(fence, None);

        let (pool, _pool_guard) = maybe_mutex_pool.lock();
        vulkan
            .device()
            .free_command_buffers(pool, slice::from_ref(&command_buffer))
    };

    Ok(())
}

/// Creates the resources to run a onetime command. Waiting for completion and cleaning up is
/// deferred to another thread. Useful for copies during setup. Works best when `command_pool` was
/// created with the `vk::CommandPoolCreateFlags::TRANSIENT`.
pub unsafe fn async_onetime_command<'m, Vulkan, CmdFn, Queue>(
    vulkan: Arc<Vulkan>,
    command_pool: Arc<Mutex<vk::CommandPool>>,
    queue: Queue,
    cmd_fn: CmdFn,
    label: &str,
) -> LabelledVkResult<()>
where
    Vulkan: VulkanContext + Send + Sync + 'static,
    CmdFn: FnOnce(&Vulkan, vk::CommandBuffer),
    Queue: Into<MaybeMutex<'m, vk::Queue>>,
{
    let pool = command_pool.lock();

    // Allocate command buffer
    let command_buffer = {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(*pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);

        unsafe { vulkan.device().allocate_command_buffers(&allocate_info) }
            .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0]
    };

    // Recording
    {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        vulkan
            .device()
            .begin_command_buffer(command_buffer, &begin_info)
            .map_err(|e| VkError::new(e, "vkBeginCommandBuffer"))?;

        cmd_fn(&vulkan, command_buffer);

        unsafe { vulkan.device().end_command_buffer(command_buffer) }
            .map_err(|e| VkError::new(e, "vkEndCommandBuffer"))?;
    }

    // Create fence
    let fence = {
        let fence_info = vk::FenceCreateInfo::default();

        let fence = unsafe { vulkan.device().create_fence(&fence_info, None) }
            .map_err(|e| VkError::new(e, "vkCreateFence"))?;

        try_name(vulkan.as_ref(), fence, label);

        fence
    };

    // Submit
    {
        let submit_info =
            vk::SubmitInfo::default().command_buffers(slice::from_ref(&command_buffer));

        let (queue, _queue_guard) = queue.into().lock();

        queue_try_begin_label(vulkan.as_ref(), queue, label);

        unsafe {
            vulkan
                .device()
                .queue_submit(queue, slice::from_ref(&submit_info), fence)
                .map_err(|e| VkError::new(e, "vkQueueSubmit"))?;
        }

        queue_try_end_label(vulkan.as_ref(), queue);
    }
    drop(pool);

    thread::spawn(move || {
        // Wait for submission to complete
        unsafe {
            if let Err(e) = vulkan
                .device()
                .wait_for_fences(slice::from_ref(&fence), true, u64::MAX)
                .map_err(|e| VkError::new(e, "vkWaitForFences"))
            {
                error!("Failed to wait for fence in async onetime command:\n{e}");
            };
        }

        // Cleanup
        unsafe {
            vulkan.device().destroy_fence(fence, None);

            let pool_guard = command_pool.lock();
            vulkan
                .device()
                .free_command_buffers(*pool_guard, slice::from_ref(&command_buffer))
        };
    });

    Ok(())
}
