// SPDX-License-Identifier: MPL-2.0

//! CPUID for the KMiri arch.

use core::arch::x86_64::CpuidResult;

/// Miri cannot execute the CPUID instruction, so every leaf is reported as
/// unsupported. Callers already handle `None`.
pub fn cpuid(_leaf: u32, _subleaf: u32) -> Option<CpuidResult> {
    None
}
