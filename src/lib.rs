//! Various helper functions, wrappers, and traits for working with Vulkan through `ash`.

#![warn(
    clippy::alloc_instead_of_core,
    clippy::use_self,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    deprecated_in_future,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications,
    missing_docs
)]
#![allow(
    clippy::missing_safety_doc,
    clippy::missing_transmute_annotations,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms
)]

extern crate alloc;

pub use allocation::*;
pub use commands::*;
pub use debug_utils::*;
pub use result::*;
pub use shader::*;
pub use vulkan_context::*;

mod allocation;
mod commands;
mod debug_utils;
mod result;
mod shader;
mod vulkan_context;
