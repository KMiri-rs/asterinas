// SPDX-License-Identifier: MPL-2.0

use super::{GuestInterrupt, GuestTimerInstant, VcpuMsrs, VcpuRegs, VcpuSregs, vmx::VmxGuard};
use crate::prelude::*;

/// A guest vCPU context.
///
/// Same public interface as the x86 `GuestContext`; creating one requires VMX,
/// which the KMiri machine does not have.
pub struct GuestContext {
    arch: VcpuArchState,
    _vmx_guard: VmxGuard,
}

impl GuestContext {
    /// Creates a guest vCPU context from initialized register state.
    ///
    /// # Panics
    ///
    /// Panics if called in [atomic mode](crate::task::atomic_mode).
    pub fn new(regs: VcpuRegs, sregs: VcpuSregs, msrs: VcpuMsrs) -> Result<Self> {
        let vmx_guard = VmxGuard::acquire_vmx()?;
        Ok(Self {
            arch: VcpuArchState {
                regs,
                sregs,
                msrs,
                pending_interrupt: None,
                timer_deadline: None,
            },
            _vmx_guard: vmx_guard,
        })
    }

    /// Returns the guest's general-purpose registers, instruction pointer, and flags.
    pub fn regs(&self) -> &VcpuRegs {
        &self.arch.regs
    }

    /// Returns a mutable reference to the guest's general-purpose registers,
    /// instruction pointer, and flags.
    pub fn regs_mut(&mut self) -> &mut VcpuRegs {
        &mut self.arch.regs
    }

    /// Returns the guest's special registers.
    pub fn sregs(&self) -> &VcpuSregs {
        &self.arch.sregs
    }

    /// Returns a mutable reference to the guest's special registers.
    pub fn sregs_mut(&mut self) -> &mut VcpuSregs {
        &mut self.arch.sregs
    }

    /// Returns the guest's MSRs that are not part of [`VcpuSregs`].
    pub fn msrs(&self) -> &VcpuMsrs {
        &self.arch.msrs
    }

    /// Returns a mutable reference to the guest's MSRs that are not part of [`VcpuSregs`].
    pub fn msrs_mut(&mut self) -> &mut VcpuMsrs {
        &mut self.arch.msrs
    }
}

/// The register state and execution requests of an x86 vCPU.
pub struct VcpuArchState {
    /// The general-purpose registers, instruction pointer, and flags.
    pub regs: VcpuRegs,
    /// The special registers and segment state.
    pub sregs: VcpuSregs,
    /// The model-specific registers.
    pub msrs: VcpuMsrs,
    /// An external interrupt awaiting injection by OSTD.
    pub pending_interrupt: Option<GuestInterrupt>,
    /// The guest TSC deadline for returning from guest execution.
    pub timer_deadline: Option<GuestTimerInstant>,
}
