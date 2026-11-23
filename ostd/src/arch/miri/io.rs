use alloc::vec::Vec;

use align_ext::AlignExt;

use crate::{boot::memory_region::MemoryRegionType, io::IoMemAllocatorBuilder};

pub mod io_mem;

/// Port I/O definition reference: <https://bochs.sourceforge.io/techspec/PORTS.LST>.
pub const MAX_IO_PORT: u16 = u16::MAX;

/// Initializes the allocatable MMIO area from the machine's memory map.
///
/// This follows the x86-64 arch (`arch/x86/io/mod.rs`): the area from the top
/// of low memory to 4 GiB and the area above the top of high memory are
/// available for MMIO. KMiri's machine is a PC with this memory layout and no
/// devices, so a driver can acquire a range, and finds nothing there.
///
/// # Safety
///
/// This function must be called only once in the boot context of the
/// bootstrapping processor, after the kernel page table is activated.
pub(super) unsafe fn construct_io_mem_allocator_builder() -> IoMemAllocatorBuilder {
    let regions = &crate::boot::EARLY_INFO.get().unwrap().memory_regions;
    let mut ranges = Vec::with_capacity(2);

    let reserved_filter = regions.iter().filter(|r| {
        r.typ() != MemoryRegionType::Unknown
            && r.typ() != MemoryRegionType::Reserved
            && r.typ() != MemoryRegionType::Framebuffer
    });

    const LOW_MMIO_TOP: usize = 0x1_0000_0000; // 4 GiB, 32 bits
    const LOW_MMIO_ALIGN: usize = 0x10_0000; // 1 MiB, 20 bits
    let (lower_half_base, lower_half_len) = reserved_filter
        .clone()
        .filter(|r| r.base() < LOW_MMIO_TOP)
        .max_by(|a, b| a.base().cmp(&b.base()))
        .map(|reg| (reg.base(), reg.len()))
        .unwrap();

    let mmio_start_addr = (lower_half_base + lower_half_len).align_up(LOW_MMIO_ALIGN);
    assert!(mmio_start_addr < LOW_MMIO_TOP);
    ranges.push(mmio_start_addr..LOW_MMIO_TOP);

    const HIGH_MMIO_TOP: usize = 0x10_0000_0000_0000; // 4 PiB, 52 bits
    const HIGH_MMIO_ALIGN: usize = 0x10_0000; // 1 MiB, 20 bits
    let (upper_half_base, upper_half_len) = reserved_filter
        .filter(|r| r.base() >= LOW_MMIO_TOP)
        .max_by(|a, b| a.base().cmp(&b.base()))
        .map(|reg| (reg.base(), reg.len()))
        .unwrap_or((LOW_MMIO_TOP, 0));

    let mmio_start_addr = (upper_half_base + upper_half_len).align_up(HIGH_MMIO_ALIGN);
    assert!(mmio_start_addr < HIGH_MMIO_TOP);
    ranges.push(mmio_start_addr..HIGH_MMIO_TOP);

    // SAFETY: The caller ensures that this is called once, after the kernel
    // page table is activated, and the ranges do not cover physical memory.
    unsafe { IoMemAllocatorBuilder::new(ranges) }
}
