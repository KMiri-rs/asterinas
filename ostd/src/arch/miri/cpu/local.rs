// SPDX-License-Identifier: MPL-2.0

//! Architecture dependent CPU-local information utilities.

use crate::{
    arch::{kern_miri_get_cpu_local_base, kern_miri_set_cpu_local_base},
    mm::PAGE_SIZE,
};

pub(crate) unsafe fn set_base(addr: u64) {
    kern_miri_set_cpu_local_base(addr as usize)
}

pub(crate) fn get_base() -> u64 {
    unsafe { kern_miri_get_cpu_local_base() as u64 }
    //0xffff_ffff_8000_0000 + 4080 * PAGE_SIZE as u64
}
