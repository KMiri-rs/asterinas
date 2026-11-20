// SPDX-License-Identifier: MPL-2.0

//! Platform-specific code for the RISC-V platform.

pub mod boot;
pub mod cpu;
pub mod device;
pub mod io;
pub mod iommu;
pub(crate) mod irq;
pub mod mm;
pub(crate) mod pci;
pub mod power;
pub mod qemu;
pub mod serial;
pub mod task;
pub mod timer;
pub mod trap;
pub mod vm;

use core::{
    fmt::{self, Arguments, Write},
    sync::atomic::Ordering,
};

use crate::task::Task;

pub(crate) unsafe fn late_init_on_bsp() {
    irq::init();
    timer::init();
    // SAFETY: We're on the BSP and we're ready to boot all APs.
    unsafe { crate::boot::smp::boot_all_aps() };
}

///
pub fn read_random() -> Option<u64> {
    None
}

pub(crate) unsafe fn init_on_ap() {
    unimplemented!()
}

/// Return the frequency of TSC. The unit is Hz.
pub fn tsc_freq() -> u64 {
    timer::TIMEBASE_FREQ.load(Ordering::Relaxed)
}

/// Reads the current value of the processor’s time-stamp counter (TSC).
pub fn read_tsc() -> u64 {
    unimplemented!()
}

pub(crate) fn enable_cpu_features() {
    // Unimplemented, no-op
}

unsafe extern "Rust" {
    /// Informs KernMiri to alloc `page_count` pages at `paddr`.
    ///
    /// Kernel should not allocate a page at the same address twice.
    /// If an address that has already been allocated is allocated again
    /// before being deallocated, KernMiri will treat it as UB.
    pub fn kern_miri_alloc_pages(paddr: usize, page_count: usize);

    /// Informs KernMiri to dealloc `page_count` pages at `paddr`.
    ///
    /// The kernel should only deallocate an address that
    /// has already been allocated by `kern_miri_alloc_pages`;
    /// otherwise, such behavior will be considered UB.
    pub fn kern_miri_dealloc_pages(paddr: usize, page_count: usize);

    /// Informs KernMiri that the `size`-byte heap object at `ptr` has been freed.
    pub fn kern_miri_dealloc_heap_slot(ptr: *mut u8, size: usize);

    /// Creates the KernMiri allocation for a `size`-byte heap object in the
    /// slot at `ptr` before the slot is returned to a caller.
    pub fn kern_miri_alloc_heap_slot(ptr: *mut u8, size: usize);

    /// Informs KernMiri to retype `page_count` pages at `paddr` to `page_type`.
    /// Each slot in the page has both size and alignment being `slot_size`.
    ///
    /// After retyping, the page will become a typed page,
    /// and the memory on the page will be converted into
    /// a contiguous series of slots of the same size.
    /// The kernel can only retype a page that has been allocated,
    /// and the same page cannot be retyped twice, otherwise,
    /// it will be considered UB.
    pub fn kern_miri_retype_pages(
        paddr: usize,
        page_count: usize,
        page_type: PageType,
        slot_size: usize,
    );

    /// Zero `page_count` of pages out. The first page starts at `paddr`.
    pub fn kern_miri_zero(paddr: usize, page_count: usize);

    // u8, u16, u32, u64 untyped read/write operation and untyped copy operation. If the operated `ptr` points to a unused or typed memory, this operation will be treated as UB.

    pub fn kern_miri_read_u8_untyped(ptr: *const u8) -> u8;

    pub fn kern_miri_write_u8_untyped(ptr: *mut u8, value: u8);

    pub fn kern_miri_copy_untyped(dst: *const u8, src: *const u8, len: usize);

    pub fn kern_miri_log(info: usize);

    pub fn kern_miri_copy(dst: usize, src: usize, len: usize);

    pub fn kern_miri_get_cpu_local_va(cpu_local_va: usize) -> usize;

    pub fn kern_miri_init_ap(
        cpu_id: usize,
        func: fn(usize),
        arg: usize,
        task: &Task,
        stack_end: usize,
        stack_size: usize,
    ) -> usize;

    fn miri_write_to_stdout(bytes: &[u8]);

    pub fn kern_miri_record_time(index: usize);

    pub fn kern_miri_get_cpu_local_base() -> usize;

    pub fn kern_miri_set_cpu_local_base(base_vaddr: usize);
}

/// The type of the typed page, used to inform miri which
/// type of typed page the current page should be retyped into.
#[repr(usize)]
#[derive(Clone, Copy)]
pub enum PageType {
    Slab = 1,
    PageTable = 2,
    Stack = 3,
    Interpreter = 4,
    /// A slab page of the kernel heap. The objects in its slots are reported
    /// with `kern_miri_alloc_heap_slot` and `kern_miri_dealloc_heap_slot`.
    Heap = 5,
}

pub struct MiriStdout;

impl Write for MiriStdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            miri_write_to_stdout(s.as_bytes());
        }
        Ok(())
    }
}

/// Prints the formatted arguments to the standard output.
pub fn _miri_print(args: Arguments) {
    MiriStdout.write_fmt(args).unwrap();
}

/// Copied from Rust std: <https://github.com/rust-lang/rust/blob/master/library/std/src/macros.rs>
#[macro_export]
macro_rules! miri_print {
    ($($arg:tt)*) => {{
        $crate::arch::miri::_miri_print(format_args!($($arg)*));
    }};
}

/// Inserts a TDX-specific code block.
///
/// The KMiri machine is an x86 machine without TDX, so this is the x86 macro
/// (`arch/x86/mod.rs`) with `::tdx_guest::tdx_is_enabled()` fixed to `false`:
/// the `else_block`, if any, is executed and the `if_block` never is. This is
/// what a native x86 kernel does when the `cvm_guest` feature is enabled but
/// TDX is not detected at runtime.
#[macro_export]
macro_rules! if_tdx_enabled {
    // Match when there is an else block
    ($if_block:block else $else_block:block) => {{ $else_block }};
    // Match when there is no else block
    ($if_block:block) => {{}};
}

pub use if_tdx_enabled;
