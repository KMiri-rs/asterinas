// SPDX-License-Identifier: MPL-2.0

//! VirtIO-MMIO backend for KMiri.
//!
//! The KMiri boot environment describes no VirtIO-MMIO devices (no kernel
//! command line, no microvm layout) and its arch layer has no `IRQ_CHIP`, so
//! there is nothing to probe. This backend only exists so that the crate
//! compiles under `cfg(miri)`; it does not fabricate a device.

pub(super) use ostd::irq::IrqLine as MappedIrqLine;

pub(super) fn probe_for_device() {}
