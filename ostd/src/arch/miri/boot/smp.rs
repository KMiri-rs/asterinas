// SPDX-License-Identifier: MPL-2.0

//! Multiprocessor Boot Support

use core::sync::atomic::{AtomicU32, Ordering};

use crate::{
    arch::{kern_miri_cpu_count, kern_miri_start_cpu},
    boot::smp::{AP_BOOT_STACK_SIZE, PerApRawInfo},
    prelude::Paddr,
};

pub(crate) fn count_processors() -> Option<u32> {
    // SAFETY: Reading the number of CPUs of the machine has no side effect.
    Some(unsafe { kern_miri_cpu_count() } as u32)
}

/// The next CPU ID to hand out to an AP that starts, like the counter in
/// `arch/x86/boot/ap_boot.S`: the APs take their IDs in the order they arrive.
static AP_NEXT_CPU_ID: AtomicU32 = AtomicU32::new(1);

/// Starts all APs. Each runs [`ap_trampoline`], which stands in for the
/// assembly in `arch/x86/boot/ap_boot.S`.
pub(crate) unsafe fn bringup_all_aps(info_ptr: *const PerApRawInfo, _pt_ptr: Paddr, num_cpus: u32) {
    for hw_cpu in 1..num_cpus as usize {
        // On x86 an AP takes its boot stack after it has taken its CPU ID. A
        // KMiri CPU needs its stack when it starts, so CPU `hw_cpu` runs on the
        // `hw_cpu`-th stack. Each boot stack is still used by exactly one AP.
        // SAFETY: `info_ptr` points to `num_cpus - 1` entries.
        let stack_top = unsafe { (*info_ptr.add(hw_cpu - 1)).stack_top } as usize;
        // SAFETY: The CPU has not been started, and its boot stack is not used
        // by anything else.
        unsafe {
            kern_miri_start_cpu(
                hw_cpu,
                ap_trampoline,
                info_ptr as usize,
                stack_top,
                AP_BOOT_STACK_SIZE,
            )
        };
    }
}

/// What `ap_boot.S` does after the mode switches: take a CPU ID, set the
/// CPU-local base to this CPU's copy of the CPU-local storage, and enter Rust.
fn ap_trampoline(info_ptr: usize) {
    let cpu_id = AP_NEXT_CPU_ID.fetch_add(1, Ordering::Relaxed);
    let info_ptr = info_ptr as *const PerApRawInfo;
    // SAFETY: There is one entry for each AP, and the CPU IDs of the APs are
    // `1..num_cpus`.
    let cpu_local = unsafe { (*info_ptr.add(cpu_id as usize - 1)).cpu_local } as u64;
    // SAFETY: This is this AP's copy of the CPU-local storage.
    unsafe { crate::arch::cpu::local::set_base(cpu_local) };
    // SAFETY: This is the boot context of an AP, and `cpu_id` is its ID.
    unsafe { crate::boot::smp::ap_early_entry(cpu_id) }
}
