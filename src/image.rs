use ash::vk;
use thiserror::Error;

use crate::{find_memorytype_index, CoreVulkan};

/// Creates a basic image with bound backing memory, handles dedicated allocations when preferred or required.
///
/// # Requires
/// - Vulkan 1.1+ or `VK_KHR_get_memory_requirements2` device extension.
/// - Vulkan 1.1+ or `VK_KHR_dedicated_allocation` device extension.
pub unsafe fn create_image<Vk: CoreVulkan>(
    vk: &Vk,
    extent: vk::Extent2D,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    memory_flags: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory, vk::MemoryRequirements), Error> {
    let device = vk.vk_device();

    // Create image
    let image = {
        let image_create_info = vk::ImageCreateInfo::default()
            .extent(extent.into())
            .usage(usage)
            .format(format)
            .array_layers(1)
            .mip_levels(1)
            .image_type(vk::ImageType::TYPE_2D)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        device
            .create_image(&image_create_info, None)
            .map_err(|e| Error::VulkanCall(e, "vkCreateImage"))?
    };

    // Allocate memory
    let (memory, memory_requirements) = {
        let memory_requirements_info = vk::ImageMemoryRequirementsInfo2::default().image(image);
        let mut dedicated_allocation_requirements = vk::MemoryDedicatedRequirements::default();
        let mut memory_requirements =
            vk::MemoryRequirements2::default().push_next(&mut dedicated_allocation_requirements);

        device.get_image_memory_requirements2(&memory_requirements_info, &mut memory_requirements);

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
            let mut dedicated_allocation = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let allocate_info = allocate_info.push_next(&mut dedicated_allocation);
            device.allocate_memory(&allocate_info, None)
        } else {
            device.allocate_memory(&allocate_info, None)
        }
        .map_err(|e| Error::VulkanCall(e, "vkAllocateMemory"))?;

        (memory, memory_requirements)
    };

    // bind memory to image
    device
        .bind_image_memory(image, memory, 0)
        .map_err(|e| Error::VulkanCall(e, "vkBindImageMemory"))?;

    Ok((image, memory, memory_requirements))
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("{1} call failed:\n{0}")]
    VulkanCall(#[source] vk::Result, &'static str),

    #[error("No suitable memory type was available for the allocation.")]
    NoSuitableMemoryType,
}
