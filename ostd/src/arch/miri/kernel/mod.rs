// SPDX-License-Identifier: MPL-2.0

//! Platform information for the KMiri arch.
//!
//! The KMiri boot environment provides no ACPI tables. This mirrors the part of
//! `arch::x86::kernel` that `aster-core` uses, filled with "not provided".

use spin::Once;

/// The platform information provided by the ACPI tables (none under KMiri).
#[derive(Debug)]
pub struct AcpiInfo {
    /// An I/O port to reset the machine by writing the specified value.
    pub reset_port_and_val: Option<(u16, u8)>,
}

/// The [`AcpiInfo`] singleton.
pub static ACPI_INFO: Once<AcpiInfo> = Once::new();

pub(in crate::arch) fn init() {
    ACPI_INFO.call_once(|| AcpiInfo {
        reset_port_and_val: None,
    });
}
