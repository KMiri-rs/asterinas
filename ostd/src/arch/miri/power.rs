// SPDX-License-Identifier: MPL-2.0

//! Power management.
//!
//! The KMiri machine has no QEMU isa-debug-exit device (the x86 code only uses
//! it after detecting QEMU), so, like native x86 without that device, both
//! operations do nothing and return; the caller then halts the machine.

/// Attempts to power off the system using an architecture-specific mechanism.
///
/// On the KMiri machine, this function does nothing and returns.
pub fn try_poweroff(_code: crate::power::ExitCode) {}

/// Attempts to restart the system using an architecture-specific mechanism.
///
/// On the KMiri machine, this function does nothing and returns.
pub fn try_restart(_code: crate::power::ExitCode) {}
