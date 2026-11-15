// SPDX-License-Identifier: MPL-2.0

//! Platform information for the KMiri arch.
//!
//! KernMiri's machine has no ACPI tables to parse. This mirrors
//! `arch::x86::kernel::acpi` and fills [`ACPI_INFO`] with what the FADT and
//! the MCFG of the machine would say: no CMOS RTC, no i8042 controller, no
//! reset register, and no PCI ECAM region.

use core::{num::NonZeroU8, time::Duration};

use spin::Once;

/// The platform information provided by the ACPI tables.
#[derive(Debug)]
pub struct AcpiInfo {
    /// The RTC CMOS RAM index to the century of data value; the "CENTURY" field in the FADT.
    pub century_register: Option<NonZeroU8>,
    /// IA-PC Boot Architecture Flags; the "IAPC_BOOT_ARCH" field in the FADT.
    pub boot_flags: Option<IaPcBootArchFlags>,
    /// An I/O port to reset the machine by writing the specified value.
    pub reset_port_and_val: Option<(u16, u8)>,
    /// A memory region that is stolen for PCI configuration space.
    pub pci_ecam_region: Option<PciEcamRegion>,
}

/// IA-PC Boot Architecture Flags.
///
/// This has the methods of `acpi::fadt::IaPcBootArchFlags` that drivers use;
/// that type cannot be constructed outside the `acpi` crate.
#[derive(Clone, Copy, Debug)]
pub struct IaPcBootArchFlags(u16);

impl IaPcBootArchFlags {
    /// Returns whether the motherboard contains an 8042 controller (bit 1).
    pub fn motherboard_implements_8042(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    /// Returns whether the CMOS RTC is either not implemented, or does not
    /// exist at the legacy addresses (bit 5).
    pub fn use_time_and_alarm_namespace_for_rtc(&self) -> bool {
        self.0 & (1 << 5) != 0
    }
}

/// A memory region that is stolen for PCI configuration space.
#[derive(Debug)]
pub struct PciEcamRegion {
    /// The base address of the memory region.
    pub base_address: u64,
    /// The start of the bus number.
    pub bus_start: u8,
    /// The end of the bus number.
    pub bus_end: u8,
}

/// The [`AcpiInfo`] singleton.
pub static ACPI_INFO: Once<AcpiInfo> = Once::new();

pub(in crate::arch) fn init() {
    ACPI_INFO.call_once(|| AcpiInfo {
        century_register: None,
        // No i8042 controller (bit 1 clear) and no CMOS RTC (bit 5 set).
        boot_flags: Some(IaPcBootArchFlags(1 << 5)),
        reset_port_and_val: None,
        pci_ecam_region: None,
    });
}

/// A consistent snapshot read from the KVM pvclock page.
///
/// This mirrors `arch::x86::kernel::kvm_clock::PvclockTimeSnapshot`.
#[derive(Clone, Copy, Debug)]
pub struct PvclockTimeSnapshot {
    /// The TSC value at the time the snapshot was taken.
    pub tsc_timestamp: u64,
    /// The system time in nanoseconds at `tsc_timestamp`.
    pub system_time: u64,
    /// The multiplier for converting TSC ticks to system time.
    pub tsc_to_system_mul: u32,
    /// The shift for converting TSC ticks to system time.
    pub tsc_shift: i8,
}

/// Returns a consistent snapshot read from the KVM pvclock page.
///
/// KernMiri's machine is not a KVM guest, so, as on x86 without KVM's
/// clocksource, there is no pvclock page and this returns `None`.
pub fn read_pvclock_time_snapshot() -> Option<PvclockTimeSnapshot> {
    None
}

/// Returns the wall-clock epoch recorded in the KVM wall-clock page.
///
/// KernMiri's machine is not a KVM guest, so, as on x86 without KVM's
/// clocksource, there is no wall-clock page and this returns `None`.
pub fn read_wall_clock_epoch() -> Option<Duration> {
    None
}
