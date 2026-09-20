// SPDX-License-Identifier: MPL-2.0
//
// The original source code is from [trapframe-rs](https://github.com/rcore-os/trapframe-rs),
// which is released under the following license:
//
// SPDX-License-Identifier: MIT
//
// Copyright (c) 2020 - 2024 Runji Wang
//
// We make the following new changes:
// * Implement the `trap_handler` of Asterinas.
//
// These changes are released under the following license:
//
// SPDX-License-Identifier: MPL-2.0

//! Handles trap.

use spin::Once;

use crate::{
    arch::{
        cpu::context::{CpuException, GeneralRegs},
        irq::{HwIrqLine, disable_local, enable_local},
    },
    cpu::PrivilegeLevel,
    irq::call_irq_callback_functions,
    mm::{
        MAX_USERSPACE_VADDR,
        fault::{TrapFrameApi, handle_user_page_fault},
    },
};

/// Trap frame of kernel interrupt
///
/// # Trap handler
///
/// You need to define a handler function like this:
///
/// ```
/// // SAFETY: The name does not collide with other symbols.
/// #[unsafe(no_mangle)]
/// extern "sysv64" fn trap_handler(tf: &mut TrapFrame) {
///     match tf.trap_num {
///         3 => {
///             println!("TRAP: BreakPoint");
///             tf.rip += 1;
///         }
///         _ => panic!("TRAP: {:#x?}", tf),
///     }
/// }
/// ```
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
#[expect(missing_docs)]
pub struct TrapFrame {
    // Pushed by 'trap.S'
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rbp: usize,
    pub rsp: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
    pub _pad: usize,

    pub trap_num: usize,
    pub error_code: usize,

    // Pushed by CPU
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
}

impl TrapFrameApi for TrapFrame {
    fn set_instruction_pointer(&mut self, ip: usize) {
        self.rip = ip;
    }

    fn instruction_pointer(&self) -> usize {
        self.rip
    }
}

/// Userspace context.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct RawUserContext {
    pub(super) general: GeneralRegs,
    pub(super) rflags: usize,
    pub(super) trap_num: usize,
    pub(super) error_code: usize,
}

impl RawUserContext {
    pub(in crate::arch) fn run(&mut self) {}
}

/// Handle traps (only from kernel).
// SAFETY: The name does not collide with other symbols.
#[unsafe(no_mangle)]
unsafe extern "sysv64" fn trap_handler(f: &mut TrapFrame) {
    fn enable_local_if(cond: bool) {
        if cond {
            enable_local();
        }
    }

    fn disable_local_if(cond: bool) {
        if cond {
            disable_local();
        }
    }

    // The IRQ state before trapping. We need to ensure that the IRQ state
    // during exception handling is consistent with the state before the trap.
    let was_irq_enabled =
        f.rflags as u64 & x86_64::registers::rflags::RFlags::INTERRUPT_FLAG.bits() > 0;

    let cpu_exception = CpuException::new(f.trap_num, f.error_code);
    match cpu_exception {
        #[cfg(feature = "cvm_guest")]
        Some(CpuException::VirtualizationException) => {}
        Some(page_fault @ CpuException::PageFault(raw_page_fault_info)) => {
            enable_local_if(was_irq_enabled);
            crate::mm::fault::handle_user_page_fault(f, &page_fault, raw_page_fault_info.addr);
            disable_local_if(was_irq_enabled);
        }
        Some(exception) => {
            enable_local_if(was_irq_enabled);
            panic!(
                "Cannot handle kernel CPU exception: {:#x?}; trapframe: {:#x?}",
                exception, f
            );
        }
        None => {
            call_irq_callback_functions(
                f,
                &HwIrqLine::new(f.trap_num as u8),
                PrivilegeLevel::Kernel,
            );
        }
    }
}

#[expect(clippy::type_complexity)]
static USER_PAGE_FAULT_HANDLER: Once<fn(&CpuException) -> Result<(), ()>> = Once::new();

/// Injects a custom handler for page faults that occur in the kernel and
/// are caused by user-space address.
pub fn inject_user_page_fault_handler(handler: fn(info: &CpuException) -> Result<(), ()>) {
    USER_PAGE_FAULT_HANDLER.call_once(|| handler);
}

pub(crate) unsafe fn init_on_cpu() {}
