use ash::vk;

use crate::{LabelledVkResult, VkError, VulkanContext, try_name};

/// The resources for rendering and presenting an individual frame.
#[derive(Clone, Copy)]
pub struct FrameResources {
    /// Semaphore that signals when the image is available.
    pub image_available_semaphore: vk::Semaphore,
    /// Semaphore that signals when the render has finished.
    pub render_finished_semaphore: vk::Semaphore,
    /// Fence that signals when these resources are no longer in flight.
    pub in_flight_fence: vk::Fence,
    /// This frame's command pool.
    pub command_pool: vk::CommandPool,
    /// That command pool's command buffer.
    pub command_buffer: vk::CommandBuffer,
}

impl FrameResources {
    /// Create new resources for a given frame.
    pub unsafe fn new<Vulkan: VulkanContext>(
        vulkan: &Vulkan,
        index: u32,
    ) -> LabelledVkResult<Self> {
        let image_available_semaphore = {
            let create_info = vk::SemaphoreCreateInfo::default();

            let semaphore = unsafe { vulkan.device().create_semaphore(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateSemaphore"))?;
            unsafe {
                try_name(
                    vulkan,
                    semaphore,
                    &format!("Image Available Semaphore {index}"),
                )
            };

            semaphore
        };

        let render_finished_semaphore = {
            let create_info = vk::SemaphoreCreateInfo::default();

            let semaphore = unsafe { vulkan.device().create_semaphore(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateSemaphore"))?;
            unsafe {
                try_name(
                    vulkan,
                    semaphore,
                    &format!("Render Finished Semaphore {index}"),
                )
            };

            semaphore
        };

        let command_pool = {
            let create_info = vk::CommandPoolCreateInfo::default()
                .queue_family_index(vulkan.queue_family_index());

            let command_pool = unsafe { vulkan.device().create_command_pool(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateCommandPool"))?;
            unsafe {
                try_name(
                    vulkan,
                    command_pool,
                    &format!("Render Command Pool {index}"),
                )
            };

            command_pool
        };

        let command_buffer = {
            let allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_buffer_count(1)
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY);

            let command_buffer =
                unsafe { vulkan.device().allocate_command_buffers(&allocate_info) }
                    .map_err(|e| VkError::new(e, "vkAllocateCommandBuffers"))?[0];
            unsafe {
                try_name(
                    vulkan,
                    command_pool,
                    &format!("Render Command Buffer {index}"),
                )
            };

            command_buffer
        };

        let fence = {
            let create_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let fence = unsafe { vulkan.device().create_fence(&create_info, None) }
                .map_err(|e| VkError::new(e, "vkCreateFence"))?;

            unsafe {
                try_name(vulkan, fence, &format!("Render Fence {index}"));
            }

            fence
        };

        Ok(Self {
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence: fence,
            command_pool,
            command_buffer,
        })
    }

    /// Destroy the Vulkan resources for this frame.
    pub unsafe fn destroy<Vk: VulkanContext>(&self, vk: &Vk) {
        unsafe {
            vk.device().destroy_fence(self.in_flight_fence, None);
            vk.device()
                .destroy_semaphore(self.image_available_semaphore, None);
            vk.device()
                .destroy_semaphore(self.render_finished_semaphore, None);
            vk.device().destroy_command_pool(self.command_pool, None);
        }
    }
}
