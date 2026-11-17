use alloc::vec::Vec;

use crate::io::IoMemAllocatorBuilder;

pub mod io_mem;

/// Port I/O definition reference: <https://bochs.sourceforge.io/techspec/PORTS.LST>.
pub const MAX_IO_PORT: u16 = u16::MAX;

/// Creates the MMIO allocator builder for KernMiri's machine.
///
/// The machine has no MMIO device, so there is no range to hand out to
/// drivers: every `IoMem::acquire` fails and a driver takes its own
/// "device absent" path.
///
/// # Safety
///
/// This function must be called only once in the boot context of the
/// bootstrapping processor.
pub(super) unsafe fn construct_io_mem_allocator_builder() -> IoMemAllocatorBuilder {
    // SAFETY: There are no ranges, so none of them belongs to physical memory.
    unsafe { IoMemAllocatorBuilder::new(Vec::new()) }
}
