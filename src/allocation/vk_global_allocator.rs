#![allow(static_mut_refs)]

#[cfg(feature = "vk-global-allocator")]
pub use global_allocator::*;
#[cfg(not(feature = "vk-global-allocator"))]
pub use not_global_allocator::*;

#[cfg(not(feature = "vk-global-allocator"))]
mod not_global_allocator {
    /// Static reference to the Vulkan callbacks to the global allocator.
    pub static VK_GLOBAL_ALLOCATOR: Option<std::sync::LazyLock<ash::vk::AllocationCallbacks<'_>>> =
        None;
    /// Returns the memory usage tracked by the Vulkan allocator callbacks.
    pub fn get_memory_usage() -> usize {
        0
    }
}

#[cfg(feature = "vk-global-allocator")]
mod global_allocator {
    use alloc::collections::BTreeMap;
    use ash::vk;
    use core::alloc::Layout;
    use parking_lot::Mutex;

    /// Static reference to the Vulkan callbacks to the global allocator.
    pub static VK_GLOBAL_ALLOCATOR: Option<std::sync::LazyLock<vk::AllocationCallbacks<'_>>> =
        Some(std::sync::LazyLock::new(create_vk_global_allocator));

    static mut LAYOUT_MAP: std::sync::LazyLock<Mutex<BTreeMap<*mut u8, Layout>>> =
        std::sync::LazyLock::new(|| Mutex::new(BTreeMap::<*mut u8, Layout>::new()));

    /// Returns the memory usage tracked by the Vulkan allocator callbacks.
    pub fn get_memory_usage() -> usize {
        unsafe { LAYOUT_MAP.lock() }
            .values()
            .fold(0, |total, layout| total + layout.size())
    }

    /// # SAFETY
    /// This is **INCREDIBLY** unsafe, probably not a good idea to use this.
    fn create_vk_global_allocator() -> vk::AllocationCallbacks<'static> {
        vk::AllocationCallbacks::default()
            .pfn_allocation(Some(allocation))
            .pfn_free(Some(free))
            .pfn_internal_allocation(Some(internal_allocation_notification))
            .pfn_internal_free(Some(internal_free_notification))
            .pfn_reallocation(Some(reallocation))
    }

    unsafe extern "system" fn allocation(
        _p_user_data: *mut core::ffi::c_void,
        size: usize,
        alignment: usize,
        _allocation_scope: vk::SystemAllocationScope,
    ) -> *mut core::ffi::c_void {
        let layout = unsafe { Layout::from_size_align_unchecked(size, alignment) };
        let pointer = unsafe { alloc::alloc::alloc(layout) };

        unsafe { LAYOUT_MAP.lock().insert(pointer, layout) };

        pointer as *mut core::ffi::c_void
    }

    unsafe extern "system" fn free(
        _p_user_data: *mut core::ffi::c_void,
        p_memory: *mut core::ffi::c_void,
    ) {
        if p_memory.is_null() {
            return;
        }

        let maybe_layout = unsafe { LAYOUT_MAP.lock().remove(&(p_memory as *mut u8)) };

        match maybe_layout {
            Some(layout) => unsafe { alloc::alloc::dealloc(p_memory as *mut u8, layout) },
            None => panic!("Leaked memory with address {p_memory:?}"),
        };
    }

    unsafe extern "system" fn reallocation(
        _p_user_data: *mut core::ffi::c_void,
        p_original: *mut core::ffi::c_void,
        size: usize,
        alignment: usize,
        _allocation_scope: vk::SystemAllocationScope,
    ) -> *mut core::ffi::c_void {
        let maybe_layout = unsafe { LAYOUT_MAP.lock().remove(&(p_original as *mut u8)) };
        let layout = match maybe_layout {
            Some(layout) => layout,
            None => panic!("Leaked memory with address: {p_original:?}"),
        };

        let pointer = unsafe { alloc::alloc::realloc(p_original as *mut u8, layout, size) };
        let new_layout = unsafe { Layout::from_size_align_unchecked(size, alignment) };
        unsafe { LAYOUT_MAP.lock().insert(pointer, new_layout) };

        pointer as *mut core::ffi::c_void
    }

    unsafe extern "system" fn internal_allocation_notification(
        _p_user_data: *mut core::ffi::c_void,
        _size: usize,
        _allocation_type: vk::InternalAllocationType,
        _allocation_scope: vk::SystemAllocationScope,
    ) {
    }

    unsafe extern "system" fn internal_free_notification(
        _p_user_data: *mut core::ffi::c_void,
        _size: usize,
        _allocation_type: vk::InternalAllocationType,
        _allocation_scope: vk::SystemAllocationScope,
    ) {
    }
}
