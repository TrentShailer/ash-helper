#![allow(missing_docs)]

use ash::vk;

use crate::VulkanContext;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Buffer usages that have an alignment restriction.
pub struct BufferUsageFlags(pub(crate) vk::Flags);

impl BufferUsageFlags {
    /// The buffer is used in a memory map operation.
    pub const MEMORY_MAP: Self = Self(1 << 0);

    /// The buffer is used as a storage buffer.
    pub const STORAGE_BUFFER: Self = Self(1 << 1);

    /// The buffer is used as a texel buffer.
    pub const TEXEL_BUFFER: Self = Self(1 << 2);

    /// The buffer is used as a uniform buffer.
    pub const UNIFORM_BUFFER: Self = Self(1 << 3);
}

ash::vk_bitflags_wrapped!(BufferUsageFlags, vk::Flags);

/// Stores the buffer alignment requiremetns for the different buffer usages.
pub struct BufferAlignment {
    memory_map: u64,
    storage_buffer: u64,
    texel_buffer: u64,
    uniform_buffer: u64,
}

impl BufferAlignment {
    /// Create a new buffer alignment object.
    pub fn new<Vulkan: VulkanContext>(vulkan: &Vulkan) -> Self {
        let properties = unsafe {
            vulkan
                .instance()
                .get_physical_device_properties(vulkan.physical_device())
        };

        Self {
            memory_map: properties.limits.min_memory_map_alignment as u64,
            storage_buffer: properties.limits.min_storage_buffer_offset_alignment,
            texel_buffer: properties.limits.min_texel_buffer_offset_alignment,
            uniform_buffer: properties.limits.min_uniform_buffer_offset_alignment,
        }
    }

    /// Calculate the start and end points of a slice of a buffer.
    /// * Slice offsets are aligned to the minimum alignment for it's usage.
    /// * Elements are aligned to `element_alignment`.
    pub fn calc_slice(
        &self,
        previous_end: u64,
        element_alignment: u64,
        element_size: u64,
        count: u64,
        usage: BufferUsageFlags,
    ) -> (u64, u64) {
        let minimum_offset_alignment = {
            let memory_map = if usage.contains(BufferUsageFlags::MEMORY_MAP) {
                self.memory_map
            } else {
                0
            };

            let storage_buffer = if usage.contains(BufferUsageFlags::STORAGE_BUFFER) {
                self.storage_buffer
            } else {
                0
            };

            let texel_buffer = if usage.contains(BufferUsageFlags::TEXEL_BUFFER) {
                self.texel_buffer
            } else {
                0
            };

            let uniform_buffer = if usage.contains(BufferUsageFlags::UNIFORM_BUFFER) {
                self.uniform_buffer
            } else {
                0
            };

            memory_map
                .max(storage_buffer)
                .max(texel_buffer)
                .max(uniform_buffer)
        };

        let start_padding = (minimum_offset_alignment - previous_end % minimum_offset_alignment)
            % minimum_offset_alignment;

        let offset = previous_end + start_padding;

        let element_padding = (element_alignment - offset % element_alignment) % element_alignment;
        let element_size = element_size + element_padding;

        let end = offset + element_size * count;

        (offset, end)
    }
}
