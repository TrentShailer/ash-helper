use std::panic::catch_unwind;

use logger::setup_logger;
use vulkan_instance::Vk;

mod logger;
mod vulkan_instance;

pub fn main() {
    setup_logger().unwrap();
    let _vk = Vk::new().unwrap();
}
