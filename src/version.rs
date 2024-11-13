use std::cmp::{self, Ordering};

use ash::vk;

/// A basic version for support checking.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub const V1_0: Version = Version::new(1, 0, 0);
    pub const V1_1: Version = Version::new(1, 1, 0);
    pub const V1_2: Version = Version::new(1, 2, 0);
    pub const V1_3: Version = Version::new(1, 3, 0);

    /// Converts a Vulkan version to a Version.
    pub fn from_vulkan_version(version: u32) -> Self {
        let major = vk::api_version_major(version);
        let minor = vk::api_version_minor(version);
        let patch = vk::api_version_patch(version);

        Version::new(major, minor, patch)
    }

    /// Converts a Version to a Vulkan version.
    pub fn as_vulkan_version(&self) -> u32 {
        vk::make_api_version(0, self.major, self.minor, self.patch)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => self.patch.cmp(&other.patch),
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
}
