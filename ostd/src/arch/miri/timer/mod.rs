// SPDX-License-Identifier: MPL-2.0

//! The timer support.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::trap::TrapFrame;

/// The timer frequency (Hz). Here we choose 1000Hz since 1000Hz is easier for unit conversion and
/// convenient for timer. What's more, the frequency cannot be set too high or too low, 1000Hz is
/// a modest choice.
///
/// For system performance reasons, this rate cannot be set too high, otherwise most of the time
/// is spent executing timer code.
pub const TIMER_FREQ: u64 = 1000;

pub(crate) static TIMEBASE_FREQ: AtomicU64 = AtomicU64::new(1);

unsafe extern "Rust" {
    fn kern_miri_register_timer_irq(handler: extern "Rust" fn());
}

/// Registers the OSTD timer callback path with KMiri.
pub(crate) fn init() {
    // SAFETY: `miri_timer_irq_entry` is a valid, retained MIR function whose
    // callback dispatch stays within OSTD's normal timer path.
    unsafe { kern_miri_register_timer_irq(miri_timer_irq_entry) };
}

extern "Rust" fn miri_timer_irq_entry() {
    let trap_frame = TrapFrame::default();
    crate::irq::call_timer_callback_functions_from_miri(&trap_frame);
}

/// Queues the registered timer entry for an OSTD Miri integration test.
pub(crate) fn test_raise_irq() {
    crate::arch::irq::test_raise_irq(miri_timer_irq_entry);
}
