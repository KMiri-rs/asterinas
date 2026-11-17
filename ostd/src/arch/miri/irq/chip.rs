// SPDX-License-Identifier: MPL-2.0

//! The IRQ chip of KernMiri's machine.
//!
//! The machine has one I/O APIC with 24 pins, for the global system interrupts
//! (GSIs) 0 to 23. ISA IRQs map to the GSIs with the same numbers (there is no
//! interrupt source override). This provides the API of `arch::x86::irq::chip`,
//! so that drivers map their IRQ pins as they do on x86. No device raises an
//! interrupt through a pin yet; the chip tracks which pin is mapped to which
//! IRQ line.

use core::{
    fmt,
    ops::{Deref, DerefMut},
};

use spin::Once;

use crate::{Error, Result, irq::IrqLine, sync::SpinLock};

/// The number of pins of the I/O APIC.
const NR_PINS: usize = 24;

/// An IRQ chip.
///
/// This abstracts the hardware IRQ chips (or IRQ controllers), allowing the bus or device drivers
/// to enable [`IrqLine`]s (via, e.g., [`map_gsi_pin_to`]) regardless of the specifics of the IRQ chip.
///
/// [`map_gsi_pin_to`]: Self::map_gsi_pin_to
pub struct IrqChip {
    /// The IRQ number that each pin is mapped to, like the vector field of an
    /// I/O APIC redirection entry.
    pins: SpinLock<[Option<u8>; NR_PINS]>,
}

impl IrqChip {
    /// Maps an IRQ pin specified by a GSI number to an IRQ line.
    ///
    /// Returns [`Error::InvalidArgs`] if the GSI does not exist and
    /// [`Error::AccessDenied`] if the pin is already mapped, as on x86.
    pub fn map_gsi_pin_to(
        &'static self,
        irq_line: IrqLine,
        gsi_index: u32,
    ) -> Result<MappedIrqLine> {
        let mut pins = self.pins.lock();
        let pin = pins.get_mut(gsi_index as usize).ok_or(Error::InvalidArgs)?;
        if pin.is_some() {
            return Err(Error::AccessDenied);
        }
        *pin = Some(irq_line.num());
        drop(pins);

        Ok(MappedIrqLine {
            irq_line,
            gsi_index,
            irq_chip: self,
        })
    }

    fn disable_gsi(&self, gsi_index: u32) {
        self.pins.lock()[gsi_index as usize] = None;
    }

    /// Maps an IRQ pin specified by an ISA interrupt number to an IRQ line.
    pub fn map_isa_pin_to(
        &'static self,
        irq_line: IrqLine,
        isa_index: u8,
    ) -> Result<MappedIrqLine> {
        self.map_gsi_pin_to(irq_line, isa_index as u32)
    }

    /// Counts the number of I/O APICs.
    pub fn count_io_apics(&self) -> usize {
        1
    }
}

/// An [`IrqLine`] mapped to an IRQ pin managed by an [`IrqChip`].
///
/// When the object is dropped, the IRQ line will be unmapped by the IRQ chip.
pub struct MappedIrqLine {
    irq_line: IrqLine,
    gsi_index: u32,
    irq_chip: &'static IrqChip,
}

impl fmt::Debug for MappedIrqLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MappedIrqLine")
            .field("irq_line", &self.irq_line)
            .field("gsi_index", &self.gsi_index)
            .finish_non_exhaustive()
    }
}

impl Deref for MappedIrqLine {
    type Target = IrqLine;

    fn deref(&self) -> &Self::Target {
        &self.irq_line
    }
}

impl DerefMut for MappedIrqLine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.irq_line
    }
}

impl Drop for MappedIrqLine {
    fn drop(&mut self) {
        self.irq_chip.disable_gsi(self.gsi_index)
    }
}

/// The [`IrqChip`] singleton.
pub static IRQ_CHIP: Once<IrqChip> = Once::new();

pub(super) fn init() {
    IRQ_CHIP.call_once(|| IrqChip {
        pins: SpinLock::new([None; NR_PINS]),
    });
}
