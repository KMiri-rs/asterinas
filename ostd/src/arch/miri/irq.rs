// SPDX-License-Identifier: MPL-2.0

//! Interrupts.

use alloc::{boxed::Box, fmt::Debug};

use spin::once::Once;

use crate::{
    arch::trap::TrapFrame,
    cpu::{CpuId, PinCurrentCpu},
    irq::IrqLine,
    smp,
};

// Intel(R) 64 and IA-32 rchitectures Software Developer's Manual,
// Volume 3A, Section 6.2 says "Vector numbers in the range 32 to 255
// are designated as user-defined interrupts and are not reserved by
// the Intel 64 and IA-32 architecture."
pub(crate) const IRQ_NUM_MIN: u8 = 32;
pub(crate) const IRQ_NUM_MAX: u8 = 255;

unsafe extern "Rust" {
    fn kern_miri_irq_is_enabled() -> bool;
    fn kern_miri_irq_disable();
    fn kern_miri_irq_enable();
    fn kern_miri_register_irq_entry(entry: extern "Rust" fn(u8));
    fn kern_miri_test_raise_irq(vector: u8);
    fn kern_miri_send_ipi(cpu: usize, vector: u8);
    fn kern_miri_halt();
}

pub(crate) fn enable_local() {
    // SAFETY: KMiri owns and validates the simulated per-CPU IRQ state.
    unsafe { kern_miri_irq_enable() };
}

pub(crate) fn disable_local() {
    // SAFETY: KMiri owns and validates the simulated per-CPU IRQ state.
    unsafe { kern_miri_irq_disable() };
}

pub(crate) fn is_local_enabled() -> bool {
    // SAFETY: This only reads KMiri's simulated per-CPU IRQ state.
    unsafe { kern_miri_irq_is_enabled() }
}

extern "Rust" fn miri_irq_entry(vector: u8) {
    crate::arch::trap::miri_dispatch_irq(vector);
}

pub(crate) fn init() {
    // SAFETY: Called once on the BSP, before any timer vector or test IRQ is set up.
    // The retained Rust entry dispatches the vector through the native trap path.
    unsafe { kern_miri_register_irq_entry(miri_irq_entry) };
}

pub(crate) fn test_raise_irq(vector: u8) {
    // SAFETY: KMiri validates the vector and invokes the registered Rust entry.
    unsafe { kern_miri_test_raise_irq(vector) };
}

pub struct CallbackElement {
    function: Box<dyn Fn(&TrapFrame) + Send + Sync + 'static>,
    id: usize,
}

impl CallbackElement {
    pub fn call(&self, element: &TrapFrame) {
        (*self.function)(element);
    }
}

impl Debug for CallbackElement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CallbackElement")
            .field("id", &self.id)
            .finish()
    }
}

/// An IRQ line with additional information that helps acknowledge the interrupt
/// on hardware.
///
/// On x86-64, it's the hardware (i.e., the I/O APIC and local APIC) that routes
/// the interrupt to the IRQ line. Therefore, the software does not need to
/// maintain additional information about the original hardware interrupt.
pub(crate) struct HwIrqLine {
    irq_num: u8,
}

impl HwIrqLine {
    pub(super) fn new(irq_num: u8) -> Self {
        Self { irq_num }
    }

    pub(crate) fn irq_num(&self) -> u8 {
        self.irq_num
    }

    pub(crate) fn ack(&self) {
        // debug_assert!(!crate::arch::cpu::context::CpuException::is_cpu_exception(
        //     self.irq_num as usize
        // ));
        // TODO: We're in the interrupt context, so `disable_preempt()` is not
        // really necessary here.
        // kernel::apic::get_or_init(&crate::task::disable_preempt() as _).eoi();
    }
}

pub(crate) struct IrqRemapping;

impl IrqRemapping {
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Initializes the remapping entry for the specific IRQ number.
    ///
    /// This will do nothing if the entry is already initialized or interrupt
    /// remapping is disabled or not supported by the architecture.
    pub(crate) fn init(&self, _irq_num: u8) {}

    /// Gets the remapping index of the IRQ line.
    ///
    /// This method will return `None` if interrupt remapping is disabled or
    /// not supported by the architecture.
    pub(crate) fn remapping_index(&self) -> Option<u16> {
        None
    }
}

static IPI_IRQ: Once<IrqLine> = Once::new();

/// Initializes global IPI state.
pub(super) fn init_ipi() {
    let mut irq = IrqLine::alloc().unwrap();
    // SAFETY: This will be called upon an inter-processor interrupt.
    irq.on_active(|f| unsafe { smp::do_inter_processor_call(f) });
    IPI_IRQ.call_once(|| irq);
}

/// Sends a general inter-processor interrupt (IPI) to the specified CPU.
pub(crate) fn send_ipi(cpu_id: HwCpuId, _guard: &dyn PinCurrentCpu) {
    let irq_num = IPI_IRQ.get().unwrap().num();
    // SAFETY: KMiri validates the target CPU and vector and delivers the IPI
    // to the registered IRQ entry on that CPU.
    unsafe { kern_miri_send_ipi(cpu_id.0 as usize, irq_num) };
}

/// Hardware-specific, architecture-dependent CPU ID.
///
/// This is the Local APIC ID in the x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HwCpuId(u32);

impl HwCpuId {
    pub(crate) fn read_current(guard: &dyn PinCurrentCpu) -> Self {
        // SAFETY: Reading the ID of the current CPU has no side effect.
        Self(unsafe { crate::arch::kern_miri_current_cpu() } as u32)
    }
}

pub(crate) fn enable_local_and_halt() {
    unsafe { kern_miri_halt() };
}

pub(crate) fn disable_local_and_halt() {
    disable_local();
}
