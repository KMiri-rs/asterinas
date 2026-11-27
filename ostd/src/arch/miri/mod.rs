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

pub(crate) unsafe fn late_init_on_bsp() {
    // SAFETY: We're on the BSP and this is called only once.
    let io_mem_builder = unsafe { io::construct_io_mem_allocator_builder() };

    irq::init();
    irq::init_ipi();
    timer::init();

    // SAFETY: We're on the BSP and we're ready to boot all APs.
    unsafe { crate::boot::smp::boot_all_aps() };

    // SAFETY: This arch drives no device through MMIO or port I/O itself (its
    // console is a KMiri shim), so there is no system device memory to remove,
    // and sensitive ports are collected from their linker section.
    unsafe { crate::io::init(io_mem_builder) };
}

///
pub fn read_random() -> Option<u64> {
    None
}

pub(crate) unsafe fn init_on_ap() {
    // On x86 this starts the AP's local timer. KMiri's timer interrupts are not
    // per CPU yet, so there is nothing to start.
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
    pub(crate) fn kern_miri_alloc_pages(paddr: usize, page_count: usize);

    /// Informs KernMiri to dealloc `page_count` pages at `paddr`.
    ///
    /// The kernel should only deallocate an address that
    /// has already been allocated by `kern_miri_alloc_pages`;
    /// otherwise, such behavior will be considered UB.
    pub(crate) fn kern_miri_dealloc_pages(paddr: usize, page_count: usize);

    /// Informs KernMiri that the `size`-byte heap object at `ptr` has been freed.
    pub(crate) fn kern_miri_dealloc_heap_slot(ptr: *mut u8, size: usize);

    /// Creates the KernMiri allocation for a `size`-byte heap object in the
    /// slot at `ptr` before the slot is returned to a caller.
    pub(crate) fn kern_miri_alloc_heap_slot(ptr: *mut u8, size: usize);

    /// Informs KernMiri to retype `page_count` pages at `paddr` to `page_type`.
    /// Each slot in the page has both size and alignment being `slot_size`.
    ///
    /// After retyping, the page will become a typed page,
    /// and the memory on the page will be converted into
    /// a contiguous series of slots of the same size.
    /// The kernel can only retype a page that has been allocated,
    /// and the same page cannot be retyped twice, otherwise,
    /// it will be considered UB.
    pub(crate) fn kern_miri_retype_pages(
        paddr: usize,
        page_count: usize,
        page_type: PageType,
        slot_size: usize,
    );

    /// Zero `page_count` of pages out. The first page starts at `paddr`.
    pub(crate) fn kern_miri_zero(paddr: usize, page_count: usize);

    // u8, u16, u32, u64 untyped read/write operation and untyped copy operation. If the operated `ptr` points to a unused or typed memory, this operation will be treated as UB.

    pub(crate) fn kern_miri_read_u8_untyped(ptr: *const u8) -> u8;

    pub(crate) fn kern_miri_write_u8_untyped(ptr: *mut u8, value: u8);

    pub(crate) fn kern_miri_copy_untyped(dst: *const u8, src: *const u8, len: usize);

    pub(crate) fn kern_miri_log(info: usize);

    pub(crate) fn kern_miri_copy(dst: *mut u8, src: *const u8, len: usize);

    pub(crate) fn kern_miri_get_cpu_local_va(cpu_local_va: usize) -> usize;

    /// Returns the number of CPUs of the machine.
    pub(crate) fn kern_miri_cpu_count() -> usize;

    /// Returns the hardware ID of the current CPU (`0..kern_miri_cpu_count()`).
    pub(crate) fn kern_miri_current_cpu() -> usize;

    /// Starts CPU `cpu`, which runs `func(arg)` on the stack that ends at
    /// `stack_end`. Everything the caller did before happens before the CPU's
    /// first step.
    pub(crate) fn kern_miri_start_cpu(
        cpu: usize,
        func: fn(usize),
        arg: usize,
        stack_end: usize,
        stack_size: usize,
    );

    /// Copies the CPU-local section to the virtual address `dst`, as a byte copy
    /// would, but keeping each variable a separate object.
    pub(crate) fn kern_miri_copy_cpu_local_section(dst: *mut u8);

    fn miri_write_to_stdout(bytes: &[u8]);

    pub fn kern_miri_record_time(index: usize);

    pub(crate) fn kern_miri_get_cpu_local_base() -> usize;

    pub(crate) fn kern_miri_set_cpu_local_base(base_vaddr: usize);

    /// Reads `size` (1, 2 or 4) bytes from the x86 I/O port `port` on
    /// KMiri's port bus.
    pub(crate) fn kern_miri_port_read(port: u16, size: usize) -> u32;

    /// Writes the low `size` (1, 2 or 4) bytes of `value` to the x86 I/O port
    /// `port` on KMiri's port bus.
    pub(crate) fn kern_miri_port_write(port: u16, size: usize, value: u32);

    /// Reads `size` (1, 2, 4 or 8) bytes of MMIO at `ptr`, zero-extended.
    ///
    /// RAM is read like any other memory. KernMiri models no device outside
    /// RAM, so there the bus floats and the read returns all ones.
    pub(crate) fn kern_miri_mmio_read(ptr: *const u8, size: usize) -> u64;

    /// Writes the low `size` (1, 2, 4 or 8) bytes of `value` to MMIO at `ptr`.
    ///
    /// RAM is written like any other memory. Outside RAM the write is dropped.
    pub(crate) fn kern_miri_mmio_write(ptr: *mut u8, size: usize, value: u64);

    /// Returns a pointer to the `index`-th static in the linker section
    /// `name`, or null past the last one.
    pub(crate) fn kern_miri_link_section_item(
        name: *const u8,
        name_len: usize,
        index: usize,
    ) -> *const ();
}

/// The type of the typed page, used to inform miri which
/// type of typed page the current page should be retyped into.
#[repr(usize)]
#[derive(Clone, Copy)]
pub(crate) enum PageType {
    Slab = 1,
    PageTable = 2,
    Stack = 3,
    Interpreter = 4,
}

pub(crate) struct MiriStdout;

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
