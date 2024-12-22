use ash::{
    khr,
    vk::{self, Handle},
};
use thiserror::Error;

use crate::{
    find_memorytype_index, CoreVulkan, DeviceRequirement, RequiredExtension,
    RequirementDescription, ValidationOutcome, ValidationResult, Version,
};

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub memory_requirements: vk::MemoryRequirements,
}

impl DeviceRequirement for Buffer {
    fn validate_device(instance: &ash::Instance, device: vk::PhysicalDevice) -> ValidationResult {
        let mut unmet_requirements = vec![];

        // Extensions
        let required_extensions = [
            RequiredExtension::new(khr::get_memory_requirements2::NAME).promoted(Version::V1_1), // Buffer::new
            RequiredExtension::new(khr::dedicated_allocation::NAME).promoted(Version::V1_1), // Buffer::new
        ];
        if let ValidationOutcome::Invalid(mut unmet_extensions) =
            RequiredExtension::validate_device(&required_extensions, instance, device)?
        {
            unmet_requirements.append(&mut unmet_extensions);
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
        let required_extensions = [
            RequiredExtension::new(khr::get_memory_requirements2::NAME).promoted(Version::V1_1), // Buffer::new
            RequiredExtension::new(khr::dedicated_allocation::NAME).promoted(Version::V1_1), // Buffer::new
        ];
        RequiredExtension::request_for_device(&required_extensions, instance, device)
    }
}

impl Buffer {
    /// Creates a basic buffer with bound backing memory, handles dedicated allocations when preferred
    /// or required.
    pub unsafe fn new<Vk: CoreVulkan>(
        vk: &Vk,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_flags: vk::MemoryPropertyFlags,
    ) -> Result<Self, Error> {
        let device = vk.vk_device();

        // Create buffer
        let buffer = {
            let create_info = vk::BufferCreateInfo::default()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
            device
                .create_buffer(&create_info, None)
                .map_err(|e| Error::VulkanCall(e, "vkCreateBuffer"))?
        };

        // Allocate Memory
        let (memory, memory_requirements) = {
            let memory_requirements_info =
                vk::BufferMemoryRequirementsInfo2::default().buffer(buffer);
            let mut dedicated_allocation_requirements = vk::MemoryDedicatedRequirements::default();
            let mut memory_requirements = vk::MemoryRequirements2::default()
                .push_next(&mut dedicated_allocation_requirements);

            device.get_buffer_memory_requirements2(
                &memory_requirements_info,
                &mut memory_requirements,
            );

            let memory_requirements = memory_requirements.memory_requirements;
            let should_be_dedicated =
                dedicated_allocation_requirements.prefers_dedicated_allocation == vk::TRUE;

            let memory_index = find_memorytype_index(vk, memory_requirements, memory_flags)
                .ok_or(Error::NoSuitableMemoryType)?;

            let allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(memory_requirements.size)
                .memory_type_index(memory_index);

            // Handle allocation or dedicated allocation
            let memory = if should_be_dedicated {
                let mut dedicated_allocation =
                    vk::MemoryDedicatedAllocateInfo::default().buffer(buffer);
                let allocate_info = allocate_info.push_next(&mut dedicated_allocation);
                device.allocate_memory(&allocate_info, None)
            } else {
                device.allocate_memory(&allocate_info, None)
            }
            .map_err(|e| Error::VulkanCall(e, "vkAllocateMemory"))?;

            (memory, memory_requirements)
        };

        // bind buffer and memory
        device
            .bind_buffer_memory(buffer, memory, 0)
            .map_err(|e| Error::VulkanCall(e, "vkBindBufferMemory"))?;

        Ok(Self {
            buffer,
            memory,
            memory_requirements,
        })
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("{1} call failed:\n{0}")]
    VulkanCall(#[source] vk::Result, &'static str),

    #[error("No suitable memory type was available for the allocation.")]
    NoSuitableMemoryType,
}
