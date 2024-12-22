use std::ops::{Add, AddAssign};

use ash::vk::{self, Handle};

use crate::requirements::{RequirementDescription, ValidationOutcome};

use super::ValidationResult;

// TODO presentation

/// The requirements for a valid queue family.
#[derive(Debug, Default)]
pub struct QueueFamilyRequirements {
    /// The queue flags that are required.
    pub flags: vk::QueueFlags,

    /// The number of queues that are required, may be shared.
    pub queue_count: u32,
}

impl QueueFamilyRequirements {
    pub fn flags(mut self, flags: vk::QueueFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn queue_count(mut self, count: u32) -> Self {
        self.queue_count = count;
        self
    }

    /// Validate if a device has a queue family that meets the requirements.
    pub fn validate_device(
        &self,
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> ValidationResult {
        if device.is_null() {
            return ValidationOutcome::Invalid(vec![RequirementDescription::non_null_device()])
                .into();
        }

        let mut unmet_requirements = vec![];

        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        if !queue_families
            .iter()
            .any(|family| family.queue_count >= self.queue_count)
        {
            unmet_requirements.push(RequirementDescription(format!(
                "Supports queue_family with queue_count >= {}",
                self.queue_count
            )));
        }

        // Reporting TRANSFER is optional if the queue supports GRAPHICS or COMPUTE.
        let transfer_flags =
            vk::QueueFlags::TRANSFER | vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE;
        let wants_transfer = self.flags.contains(vk::QueueFlags::TRANSFER);

        let required_flags = if wants_transfer {
            self.flags & !vk::QueueFlags::TRANSFER
        } else {
            self.flags
        };

        if !queue_families.iter().any(|family| {
            if wants_transfer && !family.queue_flags.intersects(transfer_flags) {
                return false;
            }

            family.queue_flags.contains(required_flags)
        }) {
            // Generate requirement description

            let flag_name_map = [
                (vk::QueueFlags::GRAPHICS, "GRAPHICS"),
                (vk::QueueFlags::COMPUTE, "COMPUTE"),
                (vk::QueueFlags::TRANSFER, "TRANSFER"),
                (vk::QueueFlags::SPARSE_BINDING, "SPARSE_BINDING"),
                (vk::QueueFlags::PROTECTED, "PROTECTED"),
                (vk::QueueFlags::VIDEO_DECODE_KHR, "VIDEO_DECODE_KHR"),
                (vk::QueueFlags::VIDEO_ENCODE_KHR, "VIDEO_ENCODE_KHR"),
                (vk::QueueFlags::OPTICAL_FLOW_NV, "OPTICAL_FLOW_NV"),
            ];

            let mut required_flags = vec![];
            for name_map in flag_name_map {
                if self.flags.contains(name_map.0) {
                    required_flags.push(name_map.1);
                }
            }

            let flag_string = required_flags.join(", ");

            unmet_requirements.push(RequirementDescription(format!(
                "Supports queue_family with flags {}",
                flag_string
            )));
        }

        if !unmet_requirements.is_empty() {
            return ValidationOutcome::Invalid(unmet_requirements).into();
        }
        ValidationOutcome::Valid.into()
    }

    pub fn get_first_queue_family(
        &self,
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
    ) -> Option<(u32, vk::QueueFamilyProperties)> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        let queue_families: Vec<_> = queue_families
            .iter()
            .enumerate()
            .filter_map(|(index, &properties)| {
                if properties.queue_count < self.queue_count {
                    return None;
                }

                let transfer_flags =
                    vk::QueueFlags::TRANSFER | vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE;
                let wants_transfer = self.flags.contains(vk::QueueFlags::TRANSFER);
                let required_flags = if wants_transfer {
                    self.flags & !vk::QueueFlags::TRANSFER
                } else {
                    self.flags
                };
                if wants_transfer && !properties.queue_flags.intersects(transfer_flags) {
                    return None;
                }
                if !properties.queue_flags.contains(required_flags) {
                    return None;
                }

                Some((index as u32, properties))
            })
            .collect();

        queue_families.first().copied()
    }
}

impl Add for QueueFamilyRequirements {
    type Output = Self;

    /// Creates the minimal superset of the two sets of requirements.
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            flags: self.flags | rhs.flags,
            queue_count: self.queue_count.max(rhs.queue_count),
        }
    }
}

impl AddAssign for QueueFamilyRequirements {
    /// Creates the minimal superset of the two sets of requirements.
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: Self) {
        self.flags |= rhs.flags;
        self.queue_count = self.queue_count.max(rhs.queue_count);
    }
}
