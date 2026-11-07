// SPDX-License-Identifier: MPL-2.0

//! `/proc/kmiri_3871`: read-only observation of asterinas#3871.
//!
//! Prints a snapshot of the BSP `LOCAL_TSS` static. It does not trigger anything; the write it
//! observes happens in the unmodified userspace entry/exit assembly.

use aster_util::printer::VmPrinter;

use crate::{
    fs::{
        file::mkmod,
        procfs::template::{ProcFile, ProcFileOps},
        vfs::inode::Inode,
    },
    prelude::*,
};

/// Represents the inode at `/proc/kmiri_3871`.
pub(super) struct Kmiri3871FileOps;

impl Kmiri3871FileOps {
    pub(super) fn new_inode(parent: Weak<dyn Inode>) -> Arc<dyn Inode> {
        ProcFile::new(Self, parent, mkmod!(a+r))
    }
}

impl ProcFileOps for Kmiri3871FileOps {
    fn read_at(&self, offset: usize, writer: &mut VmWriter) -> Result<usize> {
        let mut printer = VmPrinter::new_skip(writer, offset);
        let s = ostd::arch::trap::kmiri_3871_snapshot();

        let verdict = match (s.changed_bytes != 0, s.stored_type_is_freeze) {
            (true, true) => "IMMUTABLE_STATIC_MUTATED",
            (true, false) => "MUTATED_THROUGH_UNSAFECELL",
            (false, _) => "UNCHANGED",
        };

        writeln!(printer, "static_addr=0x{:x}", s.static_addr)?;
        writeln!(printer, "gs_tss_addr=0x{:x}", s.gs_tss_addr)?;
        writeln!(
            printer,
            "gs_targets_static={}",
            (s.static_addr == s.gs_tss_addr) as u8
        )?;
        writeln!(printer, "stored_type={}", s.stored_type)?;
        writeln!(printer, "stored_type_is_freeze={}", s.stored_type_is_freeze as u8)?;
        match s.boot_sp0 {
            Some(v) => writeln!(printer, "boot_sp0=0x{:x}", v)?,
            None => writeln!(printer, "boot_sp0=unrecorded")?,
        }
        writeln!(printer, "init_sp0=0x{:x}", s.init_sp0)?;
        writeln!(printer, "now_sp0=0x{:x}", s.now_sp0)?;
        writeln!(printer, "now_sp1=0x{:x}", s.now_sp1)?;
        writeln!(printer, "changed_bytes={}", s.changed_bytes)?;
        writeln!(printer, "rust_view_sp0=0x{:x}", s.rust_view_sp0)?;
        writeln!(
            printer,
            "rust_view_matches_memory={}",
            (s.rust_view_sp0 == s.now_sp0) as u8
        )?;
        writeln!(printer, "verdict={}", verdict)?;
        Ok(printer.bytes_written())
    }
}
