// SPDX-License-Identifier: MPL-2.0

//! I/O memory utilities.

use crate::{arch::mm::__memcpy_fallible, mm::PodOnce};

/// Reads from a pointer with a non-tearing memory load.
///
/// This function is semantically equivalent to `core::ptr::read_volatile`
/// but is manually implemented with a single memory load instruction.
///
/// # Safety
///
/// Same as the safety requirement of `core::ptr::read_volatile`.
///
/// # Guarantee
///
/// The single-memory-load-instruction guarantee is particularly useful for
/// Confidential VMs (CVMs) such as Intel TDX and AMD SEV,
/// where memory loads may cause CPU exceptions (#VE and #VC, respectively)
/// and the kernel has to handle such exceptions
/// by decoding the faulting CPU instruction.
/// As such, the kernel must be compiled to emit simple load/store CPU instructions.
pub(crate) unsafe fn read_once<T: PodOnce>(ptr: *const T) -> T {
    debug_assert!(ptr.is_aligned());
    // Natively a single `mov` of `size_of::<T>()` bytes. Under KMiri a
    // volatile read is one access of that size, checked like any other.
    //
    // SAFETY: The caller guarantees `ptr` is valid for reads.
    unsafe { core::ptr::read_volatile(ptr) }
}

/// Writes to a pointer with a non-tearing memory store.
///
/// # Safety
///
/// Same as the safety requirement of `core::ptr::write_volatile`.
///
/// # Guarantee
///
/// Refer to the "Guarantee" section of [`read_once`].
pub(crate) unsafe fn write_once<T: PodOnce>(ptr: *mut T, val: T) {
    debug_assert!(ptr.is_aligned());
    // Natively a single `mov` of `size_of::<T>()` bytes (see `read_once`).
    //
    // SAFETY: The caller guarantees `ptr` is valid for writes.
    unsafe { core::ptr::write_volatile(ptr, val) }
}

// Natively `rep movsb`: a byte-wise forward copy. KMiri's kernel copy checks
// each side like the kernel's `memcpy` does (see `Memcpy<Infallible>`). It
// copies like `memmove`, which differs from `rep movsb` only when the two
// ranges overlap; one side is MMIO and the other is memory, so they never do.
macro_rules! do_copy_mmio_impl {
    (dst: $dst:ident, src: $src:ident, count: $count:ident) => {{
        if $count == 0 {
            return;
        }

        // SAFETY: The caller guarantees source and destination are valid for `count` bytes.
        unsafe { crate::arch::kern_miri_copy($dst, $src, $count) };
    }};
}

// Natively `rep stosb`: a byte-wise fill, i.e. `volatile_set_memory` on `u8`.
macro_rules! do_memset_mmio_impl {
    (dst: $dst:ident, value: $value:ident, count: $count:ident) => {{
        if $count == 0 {
            return;
        }

        // SAFETY: The caller guarantees destination pointer is valid for `count` bytes.
        unsafe { core::intrinsics::volatile_set_memory($dst, $value, $count) };
    }};
}

/// Copies from MMIO to regular memory.
///
/// # Safety
///
/// - `dst_ptr` must be valid for writes of `count` bytes.
/// - `src_io_ptr` must be valid for MMIO reads of `count` bytes.
pub(crate) unsafe fn copy_from_mmio(dst_ptr: *mut u8, src_io_ptr: *const u8, count: usize) {
    do_copy_mmio_impl!(dst: dst_ptr, src: src_io_ptr, count: count);
}

/// Copies from regular memory to MMIO.
///
/// # Safety
///
/// - `src_ptr` must be valid for reads of `count` bytes.
/// - `dst_io_ptr` must be valid for MMIO writes of `count` bytes.
pub(crate) unsafe fn copy_to_mmio(src_ptr: *const u8, dst_io_ptr: *mut u8, count: usize) {
    do_copy_mmio_impl!(dst: dst_io_ptr, src: src_ptr, count: count);
}

/// Copies from MMIO to fallible memory and returns bytes copied.
///
/// # Safety
///
/// - `dst_ptr` must be valid or in user space for writes of `count` bytes.
/// - `src_io_ptr` must be valid for MMIO reads of `count` bytes.
pub(crate) unsafe fn copy_from_mmio_fallible(
    dst_ptr: *mut u8,
    src_io_ptr: *const u8,
    count: usize,
) -> usize {
    // SAFETY: The caller guarantees the source and destination are valid.
    let failed_bytes = unsafe { __memcpy_fallible(dst_ptr, src_io_ptr, count) };
    count - failed_bytes
}

/// Copies from fallible memory to MMIO and returns bytes copied.
///
/// # Safety
///
/// - `src_ptr` must be valid or in user space for reads of `count` bytes.
/// - `dst_io_ptr` must be valid for MMIO writes of `count` bytes.
pub(crate) unsafe fn copy_to_mmio_fallible(
    src_ptr: *const u8,
    dst_io_ptr: *mut u8,
    count: usize,
) -> usize {
    // SAFETY: The caller guarantees the source and destination are valid.
    let failed_bytes = unsafe { __memcpy_fallible(dst_io_ptr, src_ptr, count) };
    count - failed_bytes
}

/// Fills MMIO with `value` for `count` bytes.
///
/// # Safety
///
/// - `dst_io_ptr` must be valid for MMIO writes of `count` bytes.
pub(crate) unsafe fn memset_mmio(dst_io_ptr: *mut u8, value: u8, count: usize) {
    do_memset_mmio_impl!(dst: dst_io_ptr, value: value, count: count);
}
