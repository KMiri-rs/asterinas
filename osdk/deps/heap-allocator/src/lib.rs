// SPDX-License-Identifier: MPL-2.0

#![no_std]
// KMiri: the KMiri toolchain (rustc 1.99-dev, 2026-09-13) predates the stabilization of
// `core::alloc::AllocError`, which upstream relies on since nightly-2026-11-16.
#![cfg_attr(miri, feature(allocator_api))]
#![deny(unsafe_code)]

extern crate alloc;

use core::sync::atomic::AtomicIsize;

use ostd::cpu_local;

// Set this crate's log prefix for `ostd::log`.
macro_rules! __log_prefix {
    () => {
        "heap: "
    };
}

mod allocator;
mod cpu_local_allocator;
mod slab_cache;
mod slab_counter;
#[cfg(ktest)]
mod test;

pub use allocator::{HeapAllocator, type_from_layout};
pub use cpu_local_allocator::{CpuLocalBox, alloc_cpu_local};

cpu_local! {
    static LOCAL_TOTAL_SLAB_ALLOCATED: AtomicIsize = AtomicIsize::new(0);
}

/// Total size (in bytes) of physical memory committed to the slab caches.
pub(crate) static TOTAL_SLAB_ALLOCATED: slab_counter::SlabCounter =
    slab_counter::SlabCounter::new(&LOCAL_TOTAL_SLAB_ALLOCATED);

/// Returns the total size (in bytes) of memory committed to the slab caches.
pub fn load_total_slab_size() -> usize {
    TOTAL_SLAB_ALLOCATED.get()
}
