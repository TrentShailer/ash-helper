use std::sync::Arc;

use ash::{ext, khr, vk};
use parking_lot::Mutex;
use thiserror::Error;

use crate::{
    requirements::RequiredFeatures2, CoreVulkan, DeviceRequirement, EntryRequirement,
    QueueFamilyRequirement, RequiredExtension, RequirementDescription, ValidationOutcome, Version,
};

pub struct CommandBuffer {
    pub command_buffer: vk::CommandBuffer,
    pub queue: Arc<Mutex<vk::Queue>>,
    pub fence: vk::Fence,
}

impl EntryRequirement for CommandBuffer {
    fn validate_entry(entry: &ash::Entry) -> crate::ValidationResult {
        let mut unmet_requirements = vec![];

        let required_extensions =
            [
                RequiredExtension::new(khr::get_physical_device_properties2::NAME)
                    .promoted(Version::V1_1),
            ];
        if let ValidationOutcome::Invalid(mut unmet_extensions) =
            RequiredExtension::validate_entry(&required_extensions, entry)?
        {
            unmet_requirements.append(&mut unmet_extensions);
        }

        if !unmet_requirements.is_empty() {
            return ValidationOutcome::Invalid(unmet_requirements).into();
        }
        ValidationOutcome::Valid.into()
    }

    fn required_instance_extensions(entry: &ash::Entry) -> Vec<&'static std::ffi::CStr> {
        let required_extensions =
            [
                RequiredExtension::new(khr::get_physical_device_properties2::NAME)
                    .promoted(Version::V1_1),
            ];

        RequiredExtension::request_for_instance(&required_extensions, entry)
    }
}

impl DeviceRequirement for CommandBuffer {
    fn validate_device(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> crate::ValidationResult {
        let mut unmet_requirements = vec![];

        // Extensions
        let extensions =
            [RequiredExtension::new(khr::synchronization2::NAME).promoted(Version::V1_3)];
        if let ValidationOutcome::Invalid(mut unmet_extensions) =
            RequiredExtension::validate_device(&extensions, instance, device)?
        {
            unmet_requirements.append(&mut unmet_extensions);
        }

        // Features
        let mut synchronization2 = vk::PhysicalDeviceSynchronization2Features::default();
        let mut device_features =
            vk::PhysicalDeviceFeatures2::default().push_next(&mut synchronization2);
        unsafe { instance.get_physical_device_features2(device, &mut device_features) };
        if !synchronization2.synchronization2 == vk::TRUE {
            unmet_requirements.push(RequirementDescription::feature(
                "PhysicalDeviceSynchronization2Features",
                "synchronization2",
            ));
        }

        // Return
        if !unmet_requirements.is_empty() {
            return ValidationOutcome::Invalid(unmet_requirements).into();
        }
        ValidationOutcome::Valid.into()
    }

    fn required_device_extensions(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> Vec<&'static std::ffi::CStr> {
        let extensions =
            [RequiredExtension::new(khr::synchronization2::NAME).promoted(Version::V1_3)];
        RequiredExtension::request_for_device(&extensions, instance, device)
    }
}

impl RequiredFeatures2 for CommandBuffer {
    fn set_required_features2(features: &mut vk::PhysicalDeviceFeatures2) {
        let synchronization2 = Box::leak(Box::new(
            vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true),
        ));

        *features = features.push_next(synchronization2);
    }
}

impl QueueFamilyRequirement for CommandBuffer {
    fn queue_family_requirements() -> crate::QueueFamilyRequirements {
        crate::QueueFamilyRequirements::default().queue_count(1)
    }
}

/// Records and submits this command buffer and waiting on its submission fence.
pub unsafe fn record_and_submit<F, Vk, const W: usize, const S: usize>(
    vk: &Vk,
    command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    queue: &Mutex<vk::Queue>,
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
