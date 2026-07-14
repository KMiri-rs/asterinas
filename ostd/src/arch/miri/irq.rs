// SPDX-License-Identifier: MPL-2.0

//! Interrupts.

use alloc::{boxed::Box, fmt::Debug};

use spin::once::Once;

use crate::{
    arch::trap::TrapFrame,
    cpu::{CpuId, PinCurrentCpu},
};

// Intel(R) 64 and IA-32 rchitectures Software Developer's Manual,
// Volume 3A, Section 6.2 says "Vector numbers in the range 32 to 255
// are designated as user-defined interrupts and are not reserved by
// the Intel 64 and IA-32 architecture."
pub(crate) const IRQ_NUM_MIN: u8 = 32;
pub(crate) const IRQ_NUM_MAX: u8 = 255;

pub(crate) fn enable_local() {}

pub(crate) fn disable_local() {}

pub(crate) fn is_local_enabled() -> bool {
    true
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

/// Sends a general inter-processor interrupt (IPI) to the specified CPU.
///
/// # Safety
///
/// The caller must ensure that the CPU ID and the interrupt number corresponds
/// to a safe function to call.
pub(crate) fn send_ipi(cpu_id: HwCpuId, guard: &dyn PinCurrentCpu) {}

/// Hardware-specific, architecture-dependent CPU ID.
///
/// This is the Local APIC ID in the x86_64 architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HwCpuId(u32);

impl HwCpuId {
    pub(crate) fn read_current(guard: &dyn PinCurrentCpu) -> Self {
        // FIXME: should see x86_64's implementation
        Self(0)
    }
}

pub(crate) fn enable_local_and_halt() {}
pub(crate) fn disable_local_and_halt() {}
