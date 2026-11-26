// SPDX-License-Identifier: MPL-2.0

//! The RISC-V boot module defines the entrypoints of Asterinas.

pub mod smp;

use alloc::{string::String, vec::Vec};
use core::{arch::global_asm, mem};

use spin::Once;

use crate::{
    boot::{
        BootloaderAcpiArg, BootloaderFramebufferArg,
        memory_region::{MemoryRegion, MemoryRegionArray, MemoryRegionType},
    },
    early_println,
    mm::paddr_to_vaddr,
};

fn parse_bootloader_name() -> &'static str {
    "Unknown"
}

fn parse_kernel_commandline() -> &'static str {
    ""
}

fn parse_initramfs() -> Option<&'static [u8]> {
    None
}

fn parse_acpi_arg() -> BootloaderAcpiArg {
    BootloaderAcpiArg::NotProvided
}

fn parse_framebuffer_info() -> Option<BootloaderFramebufferArg> {
    None
}

fn parse_memory_regions() -> MemoryRegionArray {
    let mut regions = MemoryRegionArray::new();

    unsafe extern "Rust" {
        /// The number of entries in the machine's firmware memory map.
        fn kern_miri_memory_map_len() -> usize;
        /// Reads the `index`-th firmware memory map entry (`kind`: 1 = Usable, 2 = Reserved).
        fn kern_miri_memory_map_entry(
            index: usize,
            base: *mut usize,
            len: *mut usize,
            kind: *mut u32,
        ) -> bool;
    }

    // Report the firmware memory map provided by KMiri; any omitted ranges below
    // the highest region are filled as `MemoryRegionType::Unknown` by
    // `into_non_overlapping()`, matching native boot behavior.
    // SAFETY: The KMiri firmware memory map primitives have no preconditions,
    // and the out pointers are valid for writes.
    let count = unsafe { kern_miri_memory_map_len() };
    for i in 0..count {
        let mut base = 0usize;
        let mut len = 0usize;
        let mut kind = 0u32;
        let ok = unsafe { kern_miri_memory_map_entry(i, &mut base, &mut len, &mut kind) };
        if ok && len > 0 {
            let typ = match kind {
                1 => MemoryRegionType::Usable,
                2 => MemoryRegionType::Reserved,
                _ => MemoryRegionType::Unknown,
            };
            regions.push(MemoryRegion::new(base, len, typ));
        }
    }

    // Add the kernel region.
    let kernel_region = MemoryRegion::kernel();
    regions.push(kernel_region);

    regions.into_non_overlapping()
}

use crate::boot::{EARLY_INFO, EarlyBootInfo};

/// The entry point of the Rust code portion of Asterinas.
#[unsafe(no_mangle)]
pub fn miri_boot() {
    // Point the BSP's CPU-local base register at the `.cpu_local` section before
    // any CPU-local variable is used, as `bsp_boot.S` does on x86. KMiri starts the
    // register at its reset value 0 and provides the section symbols.
    unsafe extern "C" {
        fn __cpu_local_start();
    }
    // SAFETY: This is the first code on the BSP; no CPU-local variable is in use.
    unsafe { crate::arch::cpu::local::set_base(__cpu_local_start as *const () as u64) };

    EARLY_INFO.call_once(|| EarlyBootInfo {
        bootloader_name: parse_bootloader_name(),
        kernel_cmdline: parse_kernel_commandline(),
        initramfs: parse_initramfs(),
        acpi_arg: parse_acpi_arg(),
        framebuffer_arg: parse_framebuffer_info(),
        memory_regions: parse_memory_regions(),
    });

    // SAFETY: The function is called only once on the BSP.
    unsafe { crate::init() };

    // SAFETY: `miri_main` should define this.
    unsafe extern "Rust" {
        fn __ostd_main();
    }
    unsafe { __ostd_main() };
}
