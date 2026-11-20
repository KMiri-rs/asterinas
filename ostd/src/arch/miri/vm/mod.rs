// SPDX-License-Identifier: MPL-2.0

//! Hardware virtualization support on the KMiri machine.
//!
//! The KMiri machine is an x86 machine whose CPUs do not enumerate VMX. On
//! such a CPU, the native x86 code (`arch/x86/vm`) fails every attempt to
//! enable VMX: `VmxGuard::acquire_vmx` returns `Error::NotEnoughResources`
//! (`read_and_validate_capability`). Every guest object ([`GuestContext`],
//! `GuestPhysMemSpace`, `GuestMode`) holds a `VmxGuard`, so none of them can
//! be created. This module models exactly that: [`vmx::VmxGuard`] is
//! uninhabited and acquiring it fails with the same error.
//!
//! The architecture-independent parts are the x86 files themselves: the guest
//! register types, the VM-exit types, and the EPT page-table format.

mod context;
#[path = "../../x86/vm/exit.rs"]
#[allow(dead_code)] // `exit_info` needs a current VMCS, which cannot exist here.
mod exit;
pub(crate) mod guest_mode;
pub(crate) mod memory;
#[path = "../../x86/vm/types.rs"]
mod types;
mod vmx;

/// Initializes hardware-virtualization state on the current CPU.
///
/// The CPU has no VMX, so there is nothing to initialize (as in
/// `arch/x86/vm/vmx::init_feature_control` when VMX is not enumerated).
#[allow(dead_code)]
pub(super) fn init() {}

pub use self::{
    context::{GuestContext, VcpuArchState},
    exit::{GuestExitInfo, VmxExitReason},
    types::{
        GuestInterrupt, GuestTimerInstant, VcpuDescTable, VcpuMsrs, VcpuRegs, VcpuSegment,
        VcpuSregs,
    },
};
