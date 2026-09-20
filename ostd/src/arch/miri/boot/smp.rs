// SPDX-License-Identifier: MPL-2.0

//! Multiprocessor Boot Support
use crate::{boot::smp::PerApRawInfo, prelude::Paddr};

pub(crate) fn count_processors() -> Option<u32> {
    Some(1)
}

pub(crate) unsafe fn bringup_all_aps(info_ptr: *const PerApRawInfo, pt_ptr: Paddr, num_cpus: u32) {
    // TODO
}
