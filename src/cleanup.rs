use ash::vk;

use crate::{VK_GLOBAL_ALLOCATOR, VulkanContext};

/// Destroy a Vulkan object.
#[allow(private_bounds)]
pub unsafe fn vk_destroy<'a, Vulkan: VulkanContext, T: Into<Target<'a>>>(
    vulkan: &Vulkan,
    target: T,
) {
    unsafe {
        match target.into() {
            Target::Image(image) => vulkan
                .device()
                .destroy_image(image, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::DescriptorLayouts(descriptor_set_layouts) => {
                descriptor_set_layouts.iter().for_each(|layout| {
                    vulkan
                        .device()
                        .destroy_descriptor_set_layout(*layout, VK_GLOBAL_ALLOCATOR.as_deref())
                })
            }
            Target::ImageView(image_view) => vulkan
                .device()
                .destroy_image_view(image_view, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::Sampler(sampler) => vulkan
                .device()
                .destroy_sampler(sampler, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::Buffer(buffer) => vulkan
                .device()
                .destroy_buffer(buffer, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::DeviceMemory(device_memory) => vulkan
                .device()
                .free_memory(device_memory, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::DescriptorPool(descriptor_pool) => vulkan
                .device()
                .destroy_descriptor_pool(descriptor_pool, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::PipelineLayout(pipeline_layout) => vulkan
                .device()
                .destroy_pipeline_layout(pipeline_layout, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::Pipeline(pipeline) => vulkan
                .device()
                .destroy_pipeline(pipeline, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::ShaderModule(shader_module) => vulkan
                .device()
                .destroy_shader_module(shader_module, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::CommandPool(command_pool) => vulkan
                .device()
                .destroy_command_pool(command_pool, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::Semaphore(semaphore) => vulkan
                .device()
                .destroy_semaphore(semaphore, VK_GLOBAL_ALLOCATOR.as_deref()),
            Target::Fence(fence) => vulkan
                .device()
                .destroy_fence(fence, VK_GLOBAL_ALLOCATOR.as_deref()),
        }
    }
}

enum Target<'a> {
    Image(vk::Image),
    ImageView(vk::ImageView),
    Sampler(vk::Sampler),
    Buffer(vk::Buffer),
    DeviceMemory(vk::DeviceMemory),
    DescriptorLayouts(&'a [vk::DescriptorSetLayout]),
    DescriptorPool(vk::DescriptorPool),
    PipelineLayout(vk::PipelineLayout),
    Pipeline(vk::Pipeline),
    ShaderModule(vk::ShaderModule),
    CommandPool(vk::CommandPool),
    Semaphore(vk::Semaphore),
    Fence(vk::Fence),
}

impl From<vk::Image> for Target<'_> {
    fn from(value: vk::Image) -> Self {
        Self::Image(value)
    }
}
impl From<vk::ImageView> for Target<'_> {
    fn from(value: vk::ImageView) -> Self {
        Self::ImageView(value)
    }
}
impl From<vk::Sampler> for Target<'_> {
    fn from(value: vk::Sampler) -> Self {
        Self::Sampler(value)
    }
}
impl From<vk::Buffer> for Target<'_> {
    fn from(value: vk::Buffer) -> Self {
        Self::Buffer(value)
    }
}
impl From<vk::DeviceMemory> for Target<'_> {
    fn from(value: vk::DeviceMemory) -> Self {
        Self::DeviceMemory(value)
    }
}
impl<'a> From<&'a [vk::DescriptorSetLayout]> for Target<'a> {
    fn from(value: &'a [vk::DescriptorSetLayout]) -> Self {
        Self::DescriptorLayouts(value)
    }
}
impl From<vk::DescriptorPool> for Target<'_> {
    fn from(value: vk::DescriptorPool) -> Self {
        Self::DescriptorPool(value)
    }
}
impl From<vk::PipelineLayout> for Target<'_> {
    fn from(value: vk::PipelineLayout) -> Self {
        Self::PipelineLayout(value)
    }
}
impl From<vk::Pipeline> for Target<'_> {
    fn from(value: vk::Pipeline) -> Self {
        Self::Pipeline(value)
    }
}
impl From<vk::ShaderModule> for Target<'_> {
    fn from(value: vk::ShaderModule) -> Self {
        Self::ShaderModule(value)
    }
}
impl From<vk::CommandPool> for Target<'_> {
    fn from(value: vk::CommandPool) -> Self {
        Self::CommandPool(value)
    }
}
impl From<vk::Semaphore> for Target<'_> {
    fn from(value: vk::Semaphore) -> Self {
        Self::Semaphore(value)
    }
}
impl From<vk::Fence> for Target<'_> {
    fn from(value: vk::Fence) -> Self {
        Self::Fence(value)
    }
}
