use std::ffi::CStr;

use crate::Version;

pub struct VkExtension {
    pub name: &'static CStr,
    pub promoted: Option<Version>,
}

impl VkExtension {
    pub fn new(name: &'static CStr) -> Self {
        Self {
            name,
            promoted: None,
        }
    }

    pub fn promoted(mut self, version: Version) -> Self {
        self.promoted = Some(version);
        self
    }
}
