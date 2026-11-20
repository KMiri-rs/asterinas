// SPDX-License-Identifier: MPL-2.0

use super::{GuestContext, GuestExitInfo, vmx::VmxGuard};
use crate::{
    prelude::*,
    vm::{GuestModeHooks, GuestPhysMemSpace},
};

/// An execution object that keeps VMX operation enabled.
pub(crate) struct ArchGuestMode {
    vmx_guard: VmxGuard,
}

impl ArchGuestMode {
    /// Creates a guest execution object without entering a guest.
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            vmx_guard: VmxGuard::acquire_vmx()?,
        })
    }

    /// Attempts one guest entry and returns an exit that needs higher-level handling.
    pub(crate) fn execute_once<H: GuestModeHooks + ?Sized>(
        &self,
        _context: &mut GuestContext,
        _guest_mem: &GuestPhysMemSpace,
        _hooks: &H,
    ) -> Result<Option<GuestExitInfo>> {
        match self.vmx_guard {}
    }
}
