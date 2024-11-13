use std::sync::Arc;

use ash::vk;
use parking_lot::Mutex;
use thiserror::Error;

use crate::CoreVulkan;

/// Records and submits this command buffer and waiting on its submission fence.
pub unsafe fn record_and_submit<F, Vk, const W: usize, const S: usize>(
    vk: &Vk,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    queue: Arc<Mutex<vk::Queue>>,
    wait_semaphores: [vk::SemaphoreSubmitInfo; W],
    signal_semaphores: [vk::SemaphoreSubmitInfo; S],
    f: F,
) -> Result<(), Error>
where
    Vk: CoreVulkan,
    F: FnOnce(&ash::Device, vk::CommandBuffer),
{
    let device = vk.vk_device();

    // Wait for any previous submits to finish execution
    device
        .wait_for_fences(&[fence], true, u64::MAX)
        .map_err(Error::VkWaitForFences)?;
    device
        .reset_fences(&[fence])
        .map_err(Error::VkResetFences)?;

    // Reset command buffer state and release resources back to its pool.
    device
        .reset_command_buffer(
            command_buffer,
            vk::CommandBufferResetFlags::RELEASE_RESOURCES,
        )
        .map_err(Error::VkResetCommandBuffer)?;

    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device
        .begin_command_buffer(command_buffer, &begin_info)
        .map_err(Error::VkBeginCommandBuffer)?;

    // Add user commands to command buffer
    f(device, command_buffer);

    device
        .end_command_buffer(command_buffer)
        .map_err(Error::VkEndCommandBuffer)?;

    let submit_info = vk::CommandBufferSubmitInfo::default().command_buffer(command_buffer);
    let submit_infos = &[submit_info];

    let submit_info = vk::SubmitInfo2::default()
        .wait_semaphore_infos(&wait_semaphores)
        .signal_semaphore_infos(&signal_semaphores)
        .command_buffer_infos(submit_infos);

    let queue = queue.lock();

    device
        .queue_submit2(*queue, &[submit_info], fence)
        .map_err(Error::VkQueueSubmit2)?;

    Ok(())
}

/// Converts an array of semaphores and their pipline stages into an array of
/// `vk::SemaphoreSubmitInfo` that can be used in `CommandBuffer::record_and_submit`.
#[inline]
pub fn semaphore_submit_info_from_array<const L: usize>(
    semaphores: &[(vk::Semaphore, vk::PipelineStageFlags2); L],
) -> [vk::SemaphoreSubmitInfo; L] {
    semaphores.map(|(semaphore, stage)| {
        vk::SemaphoreSubmitInfo::default()
            .semaphore(semaphore)
            .stage_mask(stage)
    })
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("vkEndCommandBuffer call failed:\n{0}")]
    VkEndCommandBuffer(#[source] vk::Result),

    #[error("vkQueueSubmit2 call failed:\n{0}")]
    VkQueueSubmit2(#[source] vk::Result),

    #[error("vkBeginCommandBuffer call failed:\n{0}")]
    VkBeginCommandBuffer(#[source] vk::Result),

    #[error("vkResetFences call failed:\n{0}")]
    VkResetFences(#[source] vk::Result),

    #[error("vkWaitForFences call failed:\n{0}")]
    VkWaitForFences(#[source] vk::Result),

    #[error("vkResetCommandBuffer call failed:\n{0}")]
    VkResetCommandBuffer(#[source] vk::Result),
}
