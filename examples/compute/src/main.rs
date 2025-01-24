use core::slice;
use std::time::Instant;

use ash::{util::Align, vk};
use ash_helper::{
    allocate_buffer, cmd_try_begin_label, cmd_try_end_label, create_shader_module_from_spv,
    onetime_command, queue_try_begin_label, queue_try_end_label, try_name, VulkanContext,
};
use log::info;
use logger::setup_logger;
use rand::Rng;
use rand_distr::Distribution;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use vulkan::Vulkan;

mod logger;
mod vulkan;

const TRY_DEBUG: bool = true;
const BUFFER_VALUES: u32 = 2u32.pow(26);
const COMMAND_BUFFER_COUNT: u32 = 3;

#[repr(C)]
struct PushConstants {
    pub input_length: u32,
}

#[inline]
fn n_dispatches(n_values_input: u32, subgroup_size: u32) -> u32 {
    let workgroup_size = 128;
    let values_per_wave = subgroup_size * subgroup_size;

    let waves_per_workgroup = workgroup_size / subgroup_size;

    let values_per_dispatch = waves_per_workgroup * values_per_wave;

    n_values_input.div_ceil(values_per_dispatch)
}

#[inline]
fn n_values_output(n_values_input: u32, subgroup_size: u32) -> u32 {
    let workgroup_size = 128;
    let values_per_wave = subgroup_size * subgroup_size;

    let waves_per_workgroup = workgroup_size / subgroup_size;

    let values_processed_per_dispatch = waves_per_workgroup * values_per_wave;
    let values_produced_per_dispatch = workgroup_size / subgroup_size;

    let number_of_dispatches = n_values_input as f64 / values_processed_per_dispatch as f64;

    (number_of_dispatches * values_produced_per_dispatch as f64).ceil() as u32
}

fn log_duration(action: &str, start: Instant) {
    info!(
        "{} in {:.3}ms",
        action,
        start.elapsed().as_secs_f32() * 1000.0
    )
}

fn main() {
    setup_logger().unwrap();

    let vulkan = {
        let start = Instant::now();

        let vk = Vulkan::new(TRY_DEBUG);

        log_duration("Initialised Vulkan", start);

        vk
    };

    unsafe { try_name(&vulkan, vulkan.queue(), "Main Queue") };

    // Create Vulkan Objects
    let start = Instant::now();

    // Create transient command pool
    let transient_pool = {
        let create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(vulkan.queue_family_index())
            .flags(vk::CommandPoolCreateFlags::TRANSIENT);
        let command_pool =
            unsafe { vulkan.device().create_command_pool(&create_info, None) }.unwrap();

        unsafe { try_name(&vulkan, command_pool, "Transient Command Pool") };

        command_pool
    };

    // Create executor command pools and buffers
    let command_objects: Vec<_> = {
        (0..COMMAND_BUFFER_COUNT)
            .map(|index| {
                let pool_create_info = vk::CommandPoolCreateInfo::default()
                    .queue_family_index(vulkan.queue_family_index());
                let command_pool =
                    unsafe { vulkan.device().create_command_pool(&pool_create_info, None) }
                        .unwrap();

                let command_buffer_info = vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1);
                let command_buffer = unsafe {
                    vulkan
                        .device()
                        .allocate_command_buffers(&command_buffer_info)
                }
                .unwrap()[0];

                // Debug: Name the objects.
                unsafe {
                    try_name(&vulkan, command_pool, &format!("Exec Pool {index}"));
                    try_name(&vulkan, command_buffer, &format!("Exec Buffer {index}"));
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

        unsafe { vulkan.device().create_semaphore(&create_info, None) }.unwrap()
    };

    // Create descriptor layout
    let descriptor_set_layout = {
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
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR);

        unsafe {
            vulkan
                .device()
                .create_descriptor_set_layout(&layout_info, None)
        }
        .unwrap()
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

            unsafe { vulkan.device().create_pipeline_layout(&layout_info, None) }.unwrap()
        };

        let shader_module = unsafe {
            create_shader_module_from_spv(
                &vulkan,
                include_bytes!("../shaders/maximum_reduction.spv"),
            )
            .unwrap()
        };

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
                vulkan
                    .device()
                    .create_compute_pipelines(
                        vk::PipelineCache::null(),
                        slice::from_ref(&create_info),
                        None,
                    )
                    .unwrap()[0]
            }
        };

        (pipeline_layout, shader_module, pipeline)
    };

    // Get the subgroup size
    let subgroup_size = {
        let mut subgroup_properties = vk::PhysicalDeviceSubgroupProperties::default();
        let mut properties =
            vk::PhysicalDeviceProperties2::default().push_next(&mut subgroup_properties);
        unsafe {
            vulkan
                .instance()
                .get_physical_device_properties2(vulkan.physical_device(), &mut properties)
        };

        subgroup_properties.subgroup_size
    };

    // Setup buffer
    let data_size = BUFFER_VALUES as u64 * size_of::<i32>() as u64;
    let first_output_size =
        n_values_output(BUFFER_VALUES, subgroup_size) as u64 * size_of::<i32>() as u64;
    let (buffer, memory, _) = {
        let buffer_bytes = data_size + first_output_size;
        let queue_family = vulkan.queue_family_index();

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
                &vulkan,
                &create_info,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                "Main",
            )
            .unwrap()
        }
    };

    // Setup staging buffer
    // This wastes GPU memory, as it is reused for reading the result back which takes far less
    // memory than the copy to the GPU. However, it avoids another allocation.
    let (staging_buffer, staging_memory, _) = {
        let queue_family = vulkan.queue_family_index();

        let create_info = vk::BufferCreateInfo::default()
            .usage(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST)
            .size(data_size)
            .queue_family_indices(slice::from_ref(&queue_family));

        unsafe {
            allocate_buffer(
                &vulkan,
                &create_info,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                "Staging",
            )
            .unwrap()
        }
    };

    log_duration("Setup Vulkan Objects", start);

    // Create data
    let data = {
        let start = Instant::now();

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

        log_duration("Created Data", start);

        data
    };

    // Copy data to GPU
    {
        let start = Instant::now();

        // Copy data to staging
        {
            let pointer = unsafe {
                vulkan
                    .device()
                    .map_memory(staging_memory, 0, data_size, vk::MemoryMapFlags::empty())
                    .unwrap()
            };

            let mut align: Align<i32> =
                unsafe { Align::new(pointer, align_of::<i32>() as u64, data_size) };
            align.copy_from_slice(&data);

            unsafe { vulkan.device().unmap_memory(staging_memory) };
        }

        unsafe {
            onetime_command(
                &vulkan,
                transient_pool,
                vulkan.queue(),
                |vk, command_buffer| {
                    cmd_try_begin_label(vk, command_buffer, "Copy Data to GPU");

                    let buffer_copy = vk::BufferCopy::default().size(data_size);

                    vk.device().cmd_copy_buffer(
                        command_buffer,
                        staging_buffer,
                        buffer,
                        slice::from_ref(&buffer_copy),
                    );

                    cmd_try_end_label(vk, command_buffer);
                },
                "Copy Data to GPU",
            )
            .unwrap();
        }

        log_duration("Copied data to GPU", start);
    }

    // ----- Data Init Complete -----
    let read_descriptor = vk::DescriptorBufferInfo::default()
        .buffer(buffer)
        .offset(0)
        .range(data_size);

    let write_descriptor = vk::DescriptorBufferInfo::default()
        .buffer(buffer)
        .offset(data_size)
        .range(first_output_size);

    let whole_time = Instant::now();

    let mut input_length = BUFFER_VALUES;
    let mut dispatches = n_dispatches(input_length, subgroup_size);
    let mut output_length = n_values_output(input_length, subgroup_size);
    let mut data_in_read = true;
    let mut submission_index = 0;
    let mut current_wait_value = 0u64;
    let mut current_signal_value = 1u64;
    let mut submission_count = 0;

    while input_length > 1 {
        let start = Instant::now();

        // Wait for any work on the command buffer we want to use, to have completed.
        'cb_guard: {
            if COMMAND_BUFFER_COUNT as u64 > current_signal_value {
                break 'cb_guard;
            }

            let wait_value = current_signal_value - COMMAND_BUFFER_COUNT as u64;
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(slice::from_ref(&semaphore))
                .values(slice::from_ref(&wait_value));

            unsafe { vulkan.device().wait_semaphores(&wait_info, u64::MAX) }.unwrap();
        }

        // Reset pool (buffer)
        let (command_pool, command_buffer) = command_objects[submission_index];
        unsafe {
            vulkan
                .device()
                .reset_command_pool(command_pool, vk::CommandPoolResetFlags::empty())
                .unwrap();
        }

        // Write commands.
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            vulkan
                .device()
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();

            cmd_try_begin_label(
                &vulkan,
                command_buffer,
                &format!("Reduction Pass {submission_count}"),
            );

            vulkan.device().cmd_push_constants(
                command_buffer,
                pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                &input_length.to_ne_bytes(),
            );

            // Setup descriptor writes
            {
                let read_binding = if data_in_read { 0 } else { 1 };
                let write_binding = if data_in_read { 1 } else { 0 };

                let descriptor_writes = [
                    // Read buffer
                    vk::WriteDescriptorSet::default()
                        .dst_set(vk::DescriptorSet::null())
                        .descriptor_count(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .dst_binding(read_binding)
                        .buffer_info(slice::from_ref(&read_descriptor)),
                    // Write buffer
                    vk::WriteDescriptorSet::default()
                        .dst_set(vk::DescriptorSet::null())
                        .descriptor_count(1)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .dst_binding(write_binding)
                        .buffer_info(slice::from_ref(&write_descriptor)),
                ];

                vulkan.push_descriptor_device().cmd_push_descriptor_set(
                    command_buffer,
                    vk::PipelineBindPoint::COMPUTE,
                    pipeline_layout,
                    0,
                    &descriptor_writes,
                );
            }

            vulkan.device().cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                pipeline,
            );

            vulkan
                .device()
                .cmd_dispatch(command_buffer, dispatches, 1, 1);

            cmd_try_end_label(&vulkan, command_buffer);

            vulkan.device().end_command_buffer(command_buffer).unwrap();
        }

        // Submit
        {
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
                vulkan
                    .device()
                    .queue_submit(
                        vulkan.queue(),
                        slice::from_ref(&submit_info),
                        vk::Fence::null(),
                    )
                    .unwrap();
            }
        }

        info!(
            "Submission {} | Index {} | Input {} | Dispatches {} | Output {} | Wait {} | Signal {} | Elapsed {:.3}ms",
            submission_count,
            submission_index,
            input_length,
            dispatches,
            output_length,
            current_wait_value,
            current_signal_value,
            start.elapsed().as_secs_f32() * 1000.0
        );

        input_length = output_length;
        dispatches = n_dispatches(input_length, subgroup_size);
        output_length = n_values_output(input_length, subgroup_size);
        data_in_read = !data_in_read;
        submission_index = (submission_index + 1) % COMMAND_BUFFER_COUNT as usize;
        submission_count += 1;

        current_wait_value = current_signal_value;
        current_signal_value += 1;
    }

    // Copy result to cpu
    let maximum = {
        let start = Instant::now();

        // Allocate command buffer
        let command_buffer = {
            let allocate_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(transient_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            unsafe { vulkan.device().allocate_command_buffers(&allocate_info) }.unwrap()[0]
        };

        // find result buffer
        let output_offset_bytes = if data_in_read { 0 } else { data_size };

        // Recording
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            vulkan
                .device()
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();

            let buffer_copy = vk::BufferCopy::default()
                .size(size_of::<i32>() as u64)
                .src_offset(output_offset_bytes)
                .dst_offset(0);

            vulkan.device().cmd_copy_buffer(
                command_buffer,
                buffer,
                staging_buffer,
                slice::from_ref(&buffer_copy),
            );

            vulkan.device().end_command_buffer(command_buffer).unwrap();
        }

        // Submit
        {
            let mut semaphore_submit_info = vk::TimelineSemaphoreSubmitInfo::default()
                .wait_semaphore_values(slice::from_ref(&current_wait_value))
                .signal_semaphore_values(slice::from_ref(&current_signal_value));

            let submit_info = vk::SubmitInfo::default()
                .command_buffers(slice::from_ref(&command_buffer))
                .wait_semaphores(slice::from_ref(&semaphore))
                .signal_semaphores(slice::from_ref(&semaphore))
                .push_next(&mut semaphore_submit_info)
                .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::TRANSFER));

            unsafe {
                queue_try_begin_label(&vulkan, vulkan.queue(), "Copy to CPU");

                vulkan
                    .device()
                    .queue_submit(
                        vulkan.queue(),
                        slice::from_ref(&submit_info),
                        vk::Fence::null(),
                    )
                    .unwrap();

                queue_try_end_label(&vulkan, vulkan.queue())
            }
        }

        // Wait for submission to complete
        unsafe {
            let wait_info = vk::SemaphoreWaitInfo::default()
                .values(slice::from_ref(&current_signal_value))
                .semaphores(slice::from_ref(&semaphore));

            vulkan
                .device()
                .wait_semaphores(&wait_info, u64::MAX)
                .unwrap();
        }

        // Copy data to cpu
        let maximum = {
            let pointer = unsafe {
                vulkan
                    .device()
                    .map_memory(
                        staging_memory,
                        0,
                        size_of::<i32>() as u64,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap()
            };

            let raw_output: &[i32] = unsafe { slice::from_raw_parts(pointer.cast(), 1) };
            let maximum = raw_output[0];

            unsafe { vulkan.device().unmap_memory(staging_memory) };

            maximum
        };

        log_duration("Waited and Copied to CPU", start);

        unsafe {
            vulkan
                .device()
                .free_command_buffers(transient_pool, slice::from_ref(&command_buffer))
        };

        maximum
    };

    assert_eq!(maximum, i32::MAX);

    info!(
        "GPU found max {} in {} values in {:.3}ms",
        maximum,
        BUFFER_VALUES,
        whole_time.elapsed().as_secs_f32() * 1000.0
    );

    // Clean up
    unsafe {
        let start = Instant::now();
        let device = vulkan.device();

        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_memory, None);

        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);

        for (pool, _) in command_objects {
            device.destroy_command_pool(pool, None);
        }

        device.destroy_command_pool(transient_pool, None);

        device.destroy_semaphore(semaphore, None);

        device.destroy_descriptor_set_layout(descriptor_set_layout, None);

        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_pipeline(pipeline, None);

        device.destroy_shader_module(shader_module, None);

        drop(vulkan);
        log_duration("Cleaned up", start);
    }
}
