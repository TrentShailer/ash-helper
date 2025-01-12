use core::slice;
use std::time::Instant;

use ash::{util::Align, vk};
use ash_helper::{
    allocate_buffer, cmd_try_begin_label, cmd_try_end_label, create_shader_module_from_spv,
    onetime_command, try_name, VulkanContext,
};
use rand::Rng;
use rand_distr::Distribution;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use vulkan::Vulkan;

mod logger;
mod vulkan;

const TRY_DEBUG: bool = true;
// Maximum memory = (2^26 * 2) + (((2^26 / (64 * 4)) * (64 / 4)) * 2) = 142606336
const BUFFER_VALUES: u32 = 2u32.pow(26);
const COMMAND_BUFFER_COUNT: u32 = 3;

#[repr(C)]
struct PushConstants {
    pub input_length: u32,
}

fn n_dispatches(n_values_input: u32, subgroup_size: u32) -> u32 {
    let values_processed_per_dispatch = 64 * subgroup_size;

    n_values_input.div_ceil(values_processed_per_dispatch)
}

fn n_values_output(n_values_input: u32, subgroup_size: u32) -> u32 {
    let subgroups_per_dispatch = 64 / subgroup_size; // 16
    let values_processed_per_dispatch = 64 * subgroup_size; // 1024
    let values_produced_per_dispatch = subgroups_per_dispatch; // 16

    let number_of_dispatches = n_values_input as f64 / values_processed_per_dispatch as f64;

    (number_of_dispatches * values_produced_per_dispatch as f64).ceil() as u32
}

fn main() {
    let vk = Vulkan::new(TRY_DEBUG);
    unsafe { try_name(&vk, vk.queue(()).unwrap(), "Main Queue") };

    // Create transient command pool
    let transient_command_pool = {
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(vk.queue_family_index(()).unwrap())
            .flags(vk::CommandPoolCreateFlags::TRANSIENT);
        let command_pool = unsafe { vk.device().create_command_pool(&create_info, None) }.unwrap();

        unsafe { try_name(&vk, command_pool, "Transient Command Pool") };

        command_pool
    };

    // Create executor command pools and buffers
    let command_objects: Vec<_> = {
        (0..COMMAND_BUFFER_COUNT)
            .map(|index| {
                let pool_create_info = vk::CommandPoolCreateInfo::default()
                    .queue_family_index(vk.queue_family_index(()).unwrap());
                let command_pool =
                    unsafe { vk.device().create_command_pool(&pool_create_info, None) }.unwrap();

                let command_buffer_info = vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1);
                let command_buffer =
                    unsafe { vk.device().allocate_command_buffers(&command_buffer_info) }.unwrap()
                        [0];

                // Debug: Name the objects.
                unsafe {
                    try_name(&vk, command_pool, &format!("Exec Pool {index}"));
                    try_name(&vk, command_buffer, &format!("Exec Buffer {index}"));
                }

                (command_pool, command_buffer)
            })
            .collect()
    };

    // Create timeline semaphore
    let semaphore = {
        let mut type_info = vk::SemaphoreTypeCreateInfo::default()
            .initial_value(0)
            .semaphore_type(vk::SemaphoreType::TIMELINE);
        let create_info = vk::SemaphoreCreateInfo::default().push_next(&mut type_info);

        unsafe { vk.device().create_semaphore(&create_info, None) }.unwrap()
    };

    // Create descriptor pool
    let descriptor_pool = {
        let pool_size = vk::DescriptorPoolSize::default()
            .descriptor_count(2)
            .ty(vk::DescriptorType::STORAGE_BUFFER);
        let create_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(2)
            .pool_sizes(slice::from_ref(&pool_size));

        unsafe { vk.device().create_descriptor_pool(&create_info, None) }.unwrap()
    };

    // Create descriptor sets
    let (descriptor_set_layout, descriptor_sets) = {
        let set_layout = {
            let bindings = [
                vk::DescriptorSetLayoutBinding::default()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
                vk::DescriptorSetLayoutBinding::default()
                    .binding(1)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::COMPUTE),
            ];
            let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

            unsafe { vk.device().create_descriptor_set_layout(&layout_info, None) }.unwrap()
        };

        let sets = {
            let layouts = slice::from_ref(&set_layout).repeat(2);
            let allocate_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts);

            unsafe { vk.device().allocate_descriptor_sets(&allocate_info) }.unwrap()
        };

        unsafe {
            try_name(&vk, sets[0], "Read-Write Set");
            try_name(&vk, sets[1], "Write-Read Set");
        }

        (set_layout, sets)
    };

    // Create pipeline
    let (pipeline_layout, shader_module, pipeline) = {
        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(size_of::<PushConstants>() as u32);

        let pipeline_layout = {
            let layout_info = vk::PipelineLayoutCreateInfo::default()
                .push_constant_ranges(slice::from_ref(&push_range))
                .set_layouts(slice::from_ref(&descriptor_set_layout));

            unsafe { vk.device().create_pipeline_layout(&layout_info, None) }.unwrap()
        };

        let shader_module = unsafe {
            create_shader_module_from_spv(&vk, include_bytes!("../shaders/maximum_reduction.spv"))
        }
        .unwrap();

        let pipeline = {
            let create_info = vk::ComputePipelineCreateInfo::default()
                .stage(
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::COMPUTE)
                        .module(shader_module)
                        .name(c"main"),
                )
                .layout(pipeline_layout);

            unsafe {
                vk.device().create_compute_pipelines(
                    vk::PipelineCache::null(),
                    slice::from_ref(&create_info),
                    None,
                )
            }
            .unwrap()[0]
        };

        (pipeline_layout, shader_module, pipeline)
    };

    // Get the subgroup size
    let subgroup_size = {
        let mut subgroup_properties = vk::PhysicalDeviceSubgroupProperties::default();
        let mut properties =
            vk::PhysicalDeviceProperties2::default().push_next(&mut subgroup_properties);
        unsafe {
            vk.instance()
                .get_physical_device_properties2(vk.physical_device(), &mut properties)
        };

        subgroup_properties.subgroup_size
    };

    // Setup buffer
    let data_size = BUFFER_VALUES as u64 * size_of::<i32>() as u64;
    let first_output_size =
        n_values_output(BUFFER_VALUES, subgroup_size) as u64 * size_of::<i32>() as u64;
    let (buffer, memory, _) = {
        let buffer_bytes = data_size + first_output_size;
        let queue_family = vk.queue_family_index(()).unwrap();

        let create_info = vk::BufferCreateInfo::default()
            .usage(
                vk::BufferUsageFlags::TRANSFER_SRC
                    | vk::BufferUsageFlags::TRANSFER_DST
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )
            .size(buffer_bytes)
            .queue_family_indices(slice::from_ref(&queue_family));

        unsafe {
            allocate_buffer(
                &vk,
                &create_info,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                "Main",
            )
        }
        .unwrap()
    };

    // Create data
    let data = {
        let true_max_index = rand::thread_rng().gen_range(0..BUFFER_VALUES as usize);

        let distribution = rand_distr::Uniform::new(i32::MIN, i32::MAX); // Excludes i32::MAX
        let data: Vec<_> = (0..BUFFER_VALUES as usize)
            .into_par_iter()
            .map_init(rand::thread_rng, |rng, index| {
                if index == true_max_index {
                    i32::MAX
                } else {
                    distribution.sample(rng)
                }
            })
            .collect();

        data
    };

    // Copy data to GPU
    {
        let buffer_bytes = data_size;
        let queue_family = vk.queue_family_index(()).unwrap();

        let (staging_buffer, staging_memory, _) = {
            let create_info = vk::BufferCreateInfo::default()
                .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                .size(buffer_bytes)
                .queue_family_indices(slice::from_ref(&queue_family));

            unsafe {
                allocate_buffer(
                    &vk,
                    &create_info,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    "Data Staging",
                )
            }
            .unwrap()
        };

        // Copy data to staging
        {
            let pointer = unsafe {
                vk.device()
                    .map_memory(staging_memory, 0, data_size, vk::MemoryMapFlags::empty())
            }
            .unwrap();

            let mut align: Align<i32> =
                unsafe { Align::new(pointer, align_of::<i32>() as u64, data_size) };
            align.copy_from_slice(&data);

            unsafe { vk.device().unmap_memory(staging_memory) };
        }

        unsafe {
            onetime_command(
                &vk,
                transient_command_pool,
                (),
                |command_buffer, device| {
                    let buffer_copy = vk::BufferCopy::default().size(data_size);

                    device.cmd_copy_buffer(
                        command_buffer,
                        staging_buffer,
                        buffer,
                        slice::from_ref(&buffer_copy),
                    );
                },
                "Copy Data to GPU",
            )
        }
        .unwrap();

        unsafe { vk.device().destroy_buffer(staging_buffer, None) };
        unsafe { vk.device().free_memory(staging_memory, None) };
    }

    // Update descriptor sets
    {
        let read_descriptor = vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .offset(0)
            .range(data_size);
        let write_descriptor = vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .offset(data_size)
            .range(first_output_size);

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(slice::from_ref(&read_descriptor)),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[0])
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(slice::from_ref(&write_descriptor)),
            // inverse
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[1])
                .dst_binding(0)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(slice::from_ref(&write_descriptor)),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_sets[1])
                .dst_binding(1)
                .descriptor_count(1)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(slice::from_ref(&read_descriptor)),
        ];

        unsafe { vk.device().update_descriptor_sets(&writes, &[]) };
    }

    println!("Completed Data Init");

    // ----- Data Init Complete -----

    let start = Instant::now();

    let mut input_length = BUFFER_VALUES;
    let mut dispatches = n_dispatches(input_length, subgroup_size);
    let mut output_length = n_values_output(input_length, subgroup_size);
    let mut data_in_read = true;
    let mut submission_index = 0;
    let mut current_wait_value = 0u64;
    let mut current_signal_value = 1u64;

    while input_length > 1 {
        let descriptor_set = if data_in_read {
            descriptor_sets[0]
        } else {
            descriptor_sets[1]
        };

        // Reset pool (buffer)
        let (command_pool, command_buffer) = command_objects[submission_index];
        unsafe {
            vk.device()
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
        }
        .unwrap();

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            vk.device()
                .begin_command_buffer(command_buffer, &begin_info)
        }
        .unwrap();

        unsafe {
            cmd_try_begin_label(
                &vk,
                command_buffer,
                &format!("Reduction Pass {submission_index}"),
            );

            vk.device().cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                &input_length.to_ne_bytes(),
            );

            vk.device().cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline_layout,
                0,
                slice::from_ref(&descriptor_set),
                &[],
            );

            vk.device()
                .cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);

            vk.device().cmd_dispatch(command_buffer, dispatches, 1, 1);

            cmd_try_end_label(&vk, command_buffer);
        }

        unsafe { vk.device().end_command_buffer(command_buffer) }.unwrap();

        let mut semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(slice::from_ref(&current_wait_value))
            .signal_semaphore_values(slice::from_ref(&current_signal_value));

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(slice::from_ref(&semaphore))
            .signal_semaphores(slice::from_ref(&semaphore))
            .command_buffers(slice::from_ref(&command_buffer))
            .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::COMPUTE_SHADER))
            .push_next(&mut semaphore_submit_info);

        unsafe {
            vk.device().queue_submit(
                vk.queue(()).unwrap(),
                slice::from_ref(&submit_info),
                vk::Fence::null(),
            )
        }
        .unwrap();

        println!(
            "Submission {} | Input {} | Dispatches {} | Output {} | Wait {} | Signal {}",
            submission_index,
            input_length,
            dispatches,
            output_length,
            current_wait_value,
            current_signal_value
        );

        input_length = output_length;
        dispatches = n_dispatches(input_length, subgroup_size);
        output_length = n_values_output(input_length, subgroup_size);
        data_in_read = !data_in_read;
        submission_index = (submission_index + 1) % COMMAND_BUFFER_COUNT as usize;

        current_wait_value = current_signal_value;
        current_signal_value += 1;
    }

    // wait for final submission
    {
        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(slice::from_ref(&semaphore))
            .values(slice::from_ref(&current_wait_value));
        unsafe { vk.device().wait_semaphores(&wait_info, u64::MAX) }.unwrap();
    }

    println!("Submissions completed");

    // Copy result to cpu
    let maximum = {
        let elements = 1;
        let staging_size = size_of::<i32>() as u64 * elements;
        let queue_family = vk.queue_family_index(()).unwrap();

        let (staging_buffer, staging_memory, _) = {
            let create_info = vk::BufferCreateInfo::default()
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .size(staging_size)
                .queue_family_indices(slice::from_ref(&queue_family));

            unsafe {
                allocate_buffer(
                    &vk,
                    &create_info,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    "Result Staging",
                )
            }
            .unwrap()
        };

        unsafe {
            onetime_command(
                &vk,
                transient_command_pool,
                (),
                |command_buffer, device| {
                    // find result buffer
                    let output_offset_bytes = if data_in_read { 0 } else { data_size };

                    let buffer_copy = vk::BufferCopy::default()
                        .size(staging_size)
                        .src_offset(output_offset_bytes);

                    device.cmd_copy_buffer(
                        command_buffer,
                        buffer,
                        staging_buffer,
                        slice::from_ref(&buffer_copy),
                    );
                },
                "Copy Result",
            )
        }
        .unwrap();

        // Copy data to cpu
        let maximum = {
            let pointer = unsafe {
                vk.device()
                    .map_memory(staging_memory, 0, staging_size, vk::MemoryMapFlags::empty())
            }
            .unwrap();

            let raw_output: &[i32] =
                unsafe { slice::from_raw_parts(pointer.cast(), elements as usize) };

            // dbg!(raw_output);

            let maximum = raw_output[0];

            unsafe { vk.device().unmap_memory(staging_memory) };

            maximum
        };

        unsafe {
            vk.device().destroy_buffer(staging_buffer, None);
            vk.device().free_memory(staging_memory, None)
        };

        maximum
    };

    assert_eq!(maximum, i32::MAX);

    println!(
        "GPU found max {} of {} values in {:.3}ms",
        maximum,
        BUFFER_VALUES,
        start.elapsed().as_secs_f32() * 1000.0
    );

    // Clean up
    unsafe {
        let device = vk.device();

        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);

        for (pool, _) in command_objects {
            device.destroy_command_pool(pool, None);
        }

        device.destroy_command_pool(transient_command_pool, None);

        device.destroy_semaphore(semaphore, None);

        device.destroy_descriptor_set_layout(descriptor_set_layout, None);
        device.destroy_descriptor_pool(descriptor_pool, None);

        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_pipeline(pipeline, None);

        device.destroy_shader_module(shader_module, None);
    }
}
