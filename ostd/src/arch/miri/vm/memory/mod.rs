// SPDX-License-Identifier: MPL-2.0

//! Architecture-specific guest physical memory support.

#[path = "../../../x86/vm/memory/ept.rs"]
mod ept;

pub(crate) use ept::{PageTableEntry, PagingConsts};

use super::vmx::VmxGuard;
use crate::{
    mm::{Frame, frame::meta::AnyFrameMeta},
    prelude::*,
    smp::PendingIpis,
    sync::RcuDrop,
};

/// Architecture-specific state of a guest physical memory space.
pub(crate) struct ArchGuestPhysMemSpace {
    vmx_guard: VmxGuard,
}

impl ArchGuestPhysMemSpace {
    /// Creates the architecture-specific state of a guest physical memory space.
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            vmx_guard: VmxGuard::acquire_vmx()?,
        })
    }

    /// Invalidates guest translations and retains the frames until every CPU has flushed.
    pub(crate) fn invalidate(&self, _frames: &[RcuDrop<Frame<dyn AnyFrameMeta>>]) -> PendingIpis {
        match self.vmx_guard {}
    }
}
