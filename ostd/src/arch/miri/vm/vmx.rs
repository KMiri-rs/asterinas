// SPDX-License-Identifier: MPL-2.0

//! VMX on a CPU that does not support it.

use crate::{Error, prelude::*};

/// A guard that keeps VMX operation enabled.
///
/// VMX can never be enabled on the KMiri machine, so no guard exists.
pub(super) enum VmxGuard {}

impl VmxGuard {
    /// Enables VMX operation on all CPUs.
    ///
    /// Fails like native x86 on a CPU without VMX (`read_and_validate_capability`
    /// returns `Error::NotEnoughResources` when VMX is not enumerated).
    pub(super) fn acquire_vmx() -> Result<VmxGuard> {
        crate::task::atomic_mode::might_sleep();
        Err(Error::NotEnoughResources)
    }
}

pub(super) mod vmcs {
    use super::VmxGuard;
    use crate::prelude::*;

    /// The current VMCS on this CPU; it cannot exist without a [`VmxGuard`].
    pub(in crate::arch::vm) struct CurrentVmcs<'a> {
        pub(super) vmx_guard: &'a VmxGuard,
    }

    impl CurrentVmcs<'_> {
        pub(in crate::arch::vm) fn read(&self, _field: u32) -> Result<usize> {
            match *self.vmx_guard {}
        }
    }
}
