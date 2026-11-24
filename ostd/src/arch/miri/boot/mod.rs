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
        /// The size of the machine's RAM, which starts at physical address 0.
        fn kern_miri_ram_size() -> usize;
    }

    let kernel_region = MemoryRegion::kernel();
    // Like a firmware memory map, report all RAM as usable; the kernel region
    // below is carved out of it.
    // SAFETY: The KMiri primitive has no preconditions.
    let ram_size = unsafe { kern_miri_ram_size() };
    let region = MemoryRegion::new(0, ram_size, MemoryRegionType::Usable);
    regions.push(region);

    // Add the kernel region.
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
