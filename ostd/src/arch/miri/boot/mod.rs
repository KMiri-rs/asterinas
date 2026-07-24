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
    mm::{PAGE_SIZE, paddr_to_vaddr},
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

    let kernel_region = MemoryRegion::new(0, 4 * 1024 * PAGE_SIZE, MemoryRegionType::Kernel);
    let region = MemoryRegion::new(
        4 * 1024 * PAGE_SIZE,
        28 * 1024 * PAGE_SIZE,
        MemoryRegionType::Usable,
    );
    regions.push(region);

    // Add the kernel region.
    regions.push(kernel_region);

    regions.into_non_overlapping()
}

use crate::boot::{EARLY_INFO, EarlyBootInfo, start_kernel};

/// The entry point of the Rust code portion of Asterinas.
#[unsafe(no_mangle)]
pub fn miri_boot() {
    EARLY_INFO.call_once(|| EarlyBootInfo {
        bootloader_name: parse_bootloader_name(),
        kernel_cmdline: parse_kernel_commandline(),
        initramfs: parse_initramfs(),
        acpi_arg: parse_acpi_arg(),
        framebuffer_arg: parse_framebuffer_info(),
        memory_regions: parse_memory_regions(),
    });

    // Safety: miri should support this.
    unsafe { start_kernel() };
}
