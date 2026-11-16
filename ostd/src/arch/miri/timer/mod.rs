// SPDX-License-Identifier: MPL-2.0

//! The timer support.

use core::sync::atomic::{AtomicU64, Ordering};

use spin::Once;

use crate::irq::IrqLine;

/// The timer frequency (Hz). Here we choose 1000Hz since 1000Hz is easier for unit conversion and
/// convenient for timer. What's more, the frequency cannot be set too high or too low, 1000Hz is
/// a modest choice.
///
/// For system performance reasons, this rate cannot be set too high, otherwise most of the time
/// is spent executing timer code.
pub const TIMER_FREQ: u64 = 1000;

pub(crate) static TIMEBASE_FREQ: AtomicU64 = AtomicU64::new(1);

/// The timer IRQ line, allocated like `arch/x86/timer/apic.rs::init_on_bsp` so
/// that the timer callback lives in the native `top_half` INNERS table and the
/// timer has a real interrupt vector.
static TIMER_IRQ: Once<IrqLine> = Once::new();

unsafe extern "Rust" {
    fn kern_miri_set_timer_vector(vector: u8);
}

/// Registers the OSTD timer callback path with KMiri.
pub(crate) fn init() {
    let mut timer_irq = IrqLine::alloc().unwrap();
    timer_irq.on_active(|trap_frame| crate::timer::call_timer_callback_functions(trap_frame));
    let vector = timer_irq.num();
    TIMER_IRQ.call_once(|| timer_irq);

    // SAFETY: The allocated IRQ line has an external-interrupt vector, and its
    // callback is installed before automatic timer delivery can begin.
    unsafe { kern_miri_set_timer_vector(vector) };
}

/// Raises the timer vector for an OSTD Miri integration test.
pub(crate) fn test_raise_irq() {
    crate::arch::irq::test_raise_irq(TIMER_IRQ.get().unwrap().num());
}
