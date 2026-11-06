// SPDX-License-Identifier: MPL-2.0

//! Configure the Global Descriptor Table (GDT).

use alloc::boxed::Box;

use x86_64::{
    PrivilegeLevel, VirtAddr,
    instructions::tables::{lgdt, load_tss},
    registers::{
        model_specific::Star,
        segmentation::{CS, Segment},
    },
    structures::{
        DescriptorTablePointer,
        gdt::{Descriptor, SegmentSelector},
        tss::TaskStateSegment,
    },
};

use crate::cpu::local::{CpuLocal, StaticCpuLocal};

/// Initializes and loads the GDT and TSS.
///
/// The caller should only call this method once in the boot context for each available processor.
/// This is not a safety requirement, however, because calling this method again will do nothing
/// more than load the GDT and TSS with the same contents.
///
/// # Safety
///
/// The caller must ensure that no preemption can occur during the method, otherwise we may
/// accidentally load a wrong GDT and TSS that actually belongs to another CPU.
pub(super) unsafe fn init_on_cpu() {
    let tss_ptr = LOCAL_TSS.as_ptr();

    // FIXME: The segment limit in the descriptor created by `tss_segment_unchecked` does not
    // include the I/O port bitmap.

    // SAFETY: As a CPU-local variable, the TSS lives for `'static`.
    let tss_desc = unsafe { Descriptor::tss_segment_unchecked(tss_ptr) };
    let (tss0, tss1) = match tss_desc {
        Descriptor::SystemSegment(tss0, tss1) => (tss0, tss1),
        _ => unreachable!(),
    };

    // The kernel CS is considered a global invariant set by the boot GDT. This method is not
    // intended for switching to a new kernel CS.
    assert_eq!(CS::get_reg(), KERNEL_CS);

    // Allocate a new GDT with 9 entries.
    let gdt = Box::new([
        0, KCODE64, KDATA, /* KCODE32 (not used) */ 0, /* UCODE32 (not used) */ 0, UDATA,
        UCODE64, tss0, tss1,
    ]);
    let gdt = &*Box::leak(gdt);
    assert_eq!(gdt[KERNEL_CS.index() as usize], KCODE64);
    assert_eq!(gdt[KERNEL_SS.index() as usize], KDATA);
    assert_eq!(gdt[USER_CS.index() as usize], UCODE64);
    assert_eq!(gdt[USER_SS.index() as usize], UDATA);

    // Load the new GDT.
    let gdtr = DescriptorTablePointer {
        limit: (size_of_val(gdt) - 1) as u16,
        base: VirtAddr::new(gdt.as_ptr().addr() as u64),
    };
    // SAFETY: The GDT is valid to load because:
    //  - It lives for `'static`.
    //  - It contains correct entries at correct indexes: the kernel code/data segments, the user
    //    code/data segments, and the TSS segment.
    //  - Specifically, the TSS segment points to the CPU-local TSS of the current CPU.
    unsafe { lgdt(&gdtr) };

    // Load the TSS.
    let tss_sel = SegmentSelector::new(7, PrivilegeLevel::Ring0);
    assert_eq!(gdt[tss_sel.index() as usize], tss0);
    assert_eq!(gdt[(tss_sel.index() + 1) as usize], tss1);
    // SAFETY: The selector points to the TSS descriptors in the GDT.
    unsafe { load_tss(tss_sel) };

    // KMiri #3871 observation: remember the BSP TSS.sp0 before userspace ever runs.
    kmiri_3871_record_boot_sp0();

    // Set up the selectors for the `syscall` and `sysret` instructions.
    let sysret = SegmentSelector::new(4, PrivilegeLevel::Ring3);
    assert_eq!(gdt[(sysret.index() + 1) as usize], UDATA);
    assert_eq!(gdt[(sysret.index() + 2) as usize], UCODE64);
    let syscall = SegmentSelector::new(1, PrivilegeLevel::Ring0);
    assert_eq!(gdt[syscall.index() as usize], KCODE64);
    assert_eq!(gdt[(syscall.index() + 1) as usize], KDATA);
    // SAFETY: The selector points to correct kernel/user code/data descriptors in the GDT.
    unsafe { Star::write_raw(sysret.0, syscall.0) };
}

// The linker script makes sure that the `.cpu_local_tss` section is at the beginning of the area
// that stores CPU-local variables. This is important because `trap.S` and `syscall.S` will assume
// this and treat the beginning of the CPU-local area as a TSS for loading and saving the kernel
// stack!
//
// No other special initialization is required because the kernel stack information is stored in
// the TSS when we start the userspace program. See `syscall.S` for details.
// SAFETY: This is properly handled in the linker script.
#[unsafe(link_section = ".cpu_local_tss")]
static LOCAL_TSS: StaticCpuLocal<TaskStateSegment> = {
    let tss = TaskStateSegment::new();
    // SAFETY: The `.cpu_local_tss` section is part of the CPU-local area.
    unsafe { CpuLocal::__new_static(tss) }
};

// Kernel code and data descriptors.
//
// These are the exact, unique values that satisfy the requirements of the `syscall` instruction.
// The Intel manual says: "It is the responsibility of OS software to ensure that the descriptors
// (in GDT or LDT) referenced by those selector values correspond to the fixed values loaded into
// the descriptor caches; the SYSCALL instruction does not ensure this correspondence."
pub(in crate::arch) const KCODE64: u64 = 0x00AF_9B00_0000_FFFF;
pub(in crate::arch) const KDATA: u64 = 0x00CF_9300_0000_FFFF;

// A 32-bit code descriptor that is used in the boot stage only. See `boot/bsp_boot.S`.
pub(in crate::arch) const KCODE32: u64 = 0x00CF_9B00_0000_FFFF;

// User code and data descriptors.
//
// These are the exact, unique values that satisfy the requirements of the `sysret` instruction.
// The Intel manual says: "It is the responsibility of OS software to ensure that the descriptors
// (in GDT or LDT) referenced by those selector values correspond to the fixed values loaded into
// the descriptor caches; the SYSRET instruction does not ensure this correspondence."
const UCODE64: u64 = 0x00AF_FB00_0000_FFFF;
const UDATA: u64 = 0x00CF_F300_0000_FFFF;

const KERNEL_CS: SegmentSelector = SegmentSelector::new(1, PrivilegeLevel::Ring0);
const KERNEL_SS: SegmentSelector = SegmentSelector::new(2, PrivilegeLevel::Ring0);

pub(super) const USER_CS: SegmentSelector = SegmentSelector::new(6, PrivilegeLevel::Ring3);
pub(super) const USER_SS: SegmentSelector = SegmentSelector::new(5, PrivilegeLevel::Ring3);

// ===== KMiri asterinas#3871 observation probe =====
//
// Read-only instrumentation. It is NOT part of the trigger: the write under test is the
// unmodified `mov gs:4, rsp` / `mov gs:12, rsp` in `syscall.S`, executed on every return to
// and entry from userspace. This probe only reads the bytes backing `LOCAL_TSS` and reports
// whether the Rust type stored in the static admits interior mutability (`Freeze`).

/// A snapshot of the BSP `LOCAL_TSS` static, exported through `/proc/kmiri_3871`.
#[derive(Clone, Copy, Debug)]
pub struct Kmiri3871Snapshot {
    /// Address of the Rust static `LOCAL_TSS` itself (the BSP storage).
    pub static_addr: usize,
    /// Address the assembly writes through (`GS.base`, i.e. `LOCAL_TSS.as_ptr()`).
    pub gs_tss_addr: usize,
    /// The Rust type stored in the static.
    pub stored_type: &'static str,
    /// Whether that type is `Freeze` (no `UnsafeCell`), i.e. its bytes must never change.
    pub stored_type_is_freeze: bool,
    /// `TSS.sp0` recorded at the end of `init_on_cpu`, before any userspace entry.
    pub boot_sp0: Option<u64>,
    /// `TSS.sp0` in the compile-time initializer `TaskStateSegment::new()`.
    pub init_sp0: u64,
    /// `TSS.sp0` currently in the static's bytes.
    pub now_sp0: u64,
    /// `TSS.sp1` currently in the static's bytes (user rsp scratch of the latest syscall).
    pub now_sp1: u64,
    /// Number of bytes that differ from the compile-time initializer.
    pub changed_bytes: usize,
}

mod kmiri_3871_freeze {
    use core::marker::{Freeze, PhantomData};

    // Autoref-based dispatch: `(&&Probe<T>).is_freeze()` picks `ViaFreeze` iff `T: Freeze`.
    pub(super) struct Probe<T>(pub(super) PhantomData<T>);
    pub(super) trait ViaFreeze {
        fn is_freeze(&self) -> bool {
            true
        }
    }
    impl<T: Freeze> ViaFreeze for &Probe<T> {}
    pub(super) trait ViaAny {
        fn is_freeze(&self) -> bool {
            false
        }
    }
    impl<T> ViaAny for Probe<T> {}
}

static KMIRI_3871_BOOT_SP0: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
static KMIRI_3871_BOOT_RECORDED: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

fn kmiri_3871_phantom<T>(_: &StaticCpuLocal<T>) -> core::marker::PhantomData<T> {
    core::marker::PhantomData
}

fn kmiri_3871_static_addr() -> usize {
    core::ptr::from_ref(&LOCAL_TSS) as usize
}

fn kmiri_3871_gs_addr() -> usize {
    LOCAL_TSS.as_ptr() as usize
}

/// Reads `len` bytes at `addr` with volatile byte loads (the field at offset 4 is unaligned).
fn kmiri_3871_read_u64(base: *const u8, offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    for (i, b) in bytes.iter_mut().enumerate() {
        // SAFETY: `base` points to the `LOCAL_TSS` static and `offset + 8 <= size_of::<TSS>()`.
        *b = unsafe { core::ptr::read_volatile(base.add(offset + i)) };
    }
    u64::from_le_bytes(bytes)
}

const KMIRI_3871_SP0_OFFSET: usize = 4;
const KMIRI_3871_SP1_OFFSET: usize = 12;

fn kmiri_3871_record_boot_sp0() {
    use core::sync::atomic::Ordering;
    // Only the BSP's TSS is the Rust static itself; APs use dynamically allocated storage.
    if kmiri_3871_gs_addr() != kmiri_3871_static_addr() {
        return;
    }
    let base = core::ptr::from_ref(&LOCAL_TSS).cast::<u8>();
    let sp0 = kmiri_3871_read_u64(base, KMIRI_3871_SP0_OFFSET);
    KMIRI_3871_BOOT_SP0.store(sp0, Ordering::Relaxed);
    KMIRI_3871_BOOT_RECORDED.store(true, Ordering::Release);
}

/// Takes a read-only snapshot of the BSP `LOCAL_TSS` static.
pub fn kmiri_3871_snapshot() -> Kmiri3871Snapshot {
    use core::sync::atomic::Ordering;

    use kmiri_3871_freeze::{Probe, ViaAny as _, ViaFreeze as _};

    fn type_name_of<T>(_: &StaticCpuLocal<T>) -> &'static str {
        core::any::type_name::<T>()
    }

    let probe = Probe(kmiri_3871_phantom(&LOCAL_TSS));
    let stored_type_is_freeze = (&&probe).is_freeze();

    let base = core::ptr::from_ref(&LOCAL_TSS).cast::<u8>();
    let init = TaskStateSegment::new();
    let init_base = core::ptr::from_ref(&init).cast::<u8>();
    let size = size_of::<TaskStateSegment>();
    let mut changed_bytes = 0;
    for i in 0..size {
        // SAFETY: Both pointers cover `size_of::<TaskStateSegment>()` readable bytes.
        let (now, old) = unsafe {
            (
                core::ptr::read_volatile(base.add(i)),
                core::ptr::read_volatile(init_base.add(i)),
            )
        };
        if now != old {
            changed_bytes += 1;
        }
    }

    Kmiri3871Snapshot {
        static_addr: kmiri_3871_static_addr(),
        gs_tss_addr: kmiri_3871_gs_addr(),
        stored_type: type_name_of(&LOCAL_TSS),
        stored_type_is_freeze,
        boot_sp0: KMIRI_3871_BOOT_RECORDED
            .load(Ordering::Acquire)
            .then(|| KMIRI_3871_BOOT_SP0.load(Ordering::Relaxed)),
        init_sp0: kmiri_3871_read_u64(init_base, KMIRI_3871_SP0_OFFSET),
        now_sp0: kmiri_3871_read_u64(base, KMIRI_3871_SP0_OFFSET),
        now_sp1: kmiri_3871_read_u64(base, KMIRI_3871_SP1_OFFSET),
        changed_bytes,
    }
}
