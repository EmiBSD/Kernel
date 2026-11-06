/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

/*-
 * Copyright (c) 1994
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley
 * by Pace Willisson (pace@blitz.com).  The Rock Ridge Extension
 * Support code is derived from software contributed to Berkeley
 * by Atsushi Murai (amurai@spec.co.jp).
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)iso.h	8.4 (Berkeley) 12/5/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<isofs/cd9660/cd9660_extern.h>`: definitions used in the kernel for cd9660 file system
//! support: the CD-ROM format type, the per-mount data (`struct iso_mnt`) and the block
//! arithmetic macros.
//!
//! Upstream: sys/isofs/cd9660/cd9660_extern.h @ 3ce1f3f79392
//! LZ: sys/isofs/cd9660/cd9660_extern.rs@3c62ede68802
//!
//! ## Deviations
//! - `im_export` (`struct netexport`) is not a member: the export list of every kernel file
//!   system's mount is the mount's glue member (`MountG`, `bsd`'s `BsdMountG.export`, the UFS
//!   mounts' since N5.5b0, msdosfs's since N5.7c3), which `cd9660_mount` updates through
//!   `FileGlue::vfs_export` and `iso_mountfs` starts empty through `FileGlue::ufs_export_init`
//!   (N5.8c2); the C keeps it here whether or not `NFSSERVER` is configured.
//! - `root_extent` and `root_size` are `u32`, as `isonum_733` reads them (the C's `int` only
//!   differs past 2^31 blocks).
//! - `blkoff`, `lblktosize`, `lblkno` and `blksize` are functions over `i64` offsets and
//!   blocks (the C's `off_t`/`daddr_t`).
//! - The prototypes are the functions of `cd9660_vfsops.rs`, `cd9660_util.rs`,
//!   `cd9660_node.rs` and `cd9660_vnops.rs`.
//!
//! ## Redesign
//! - LZ's `IsoMnt` (plain members, `malloc(M_ISOFSMNT)` written into `mnt_data` and cast
//!   back by `vfstoisofs`'s `unsafe` deref behind a `vfc_name` check; each node holding a
//!   `&'static`) -> an [`IsoMnt`] built whole by `iso_mountfs` in its locals (D-N5.8-6: the C
//!   writes no member after the mount, cd9660_vfsops.c:339-420), every member plain, and
//!   published once in a counted `malloc(M_ISOFSMNT)` block ([`IsoMntRef`],
//!   `hal::malloc::MArc`, [`IsoMnt::alloc`]; `vmstat -m`'s `ISOFS mount` row, 16 bytes more
//!   per block for the count and the type, as the `UFS mount` row) erased into the mount's
//!   typed slot where the C sets `mnt_data`; the unmount takes it back after `vflush`, the
//!   last handle frees the block (docs/IDIOMS.md, the row of a C in-core structure filled
//!   member by member and hung); the requests of a refused mount follow the C's through
//!   N5.8c2's shim (D-N5.7-U5).

//! - `VFSTOISOFS(mp)` ([`vfstoisofs`]) is the slot's typed read, a counted handle (the type
//!   replaces LZ's name check; `panic(9)` on another file system's mount). Cost against the
//!   C's one load: the slot's read and a count pair, once per `Vfs` call (ROADMAP N12's
//!   row); a node holds one handle for its life and borrows it (`IsoNode::mnt`, one load,
//!   the C's `ip->i_mnt`). 0 `unsafe` (LZ: 1).

use hal::malloc::{M_WAITOK, MArc};
use hal::sys::types::Dev;

use crate::assert::kpanic;
use crate::fs::isofs::cd9660::iso::isodcl;
use crate::fs::mount::Mount;
use crate::fs::vnode::Vnode;
use crate::proc::ProcGlue;

/// `M_ISOFSMNT`: ISOFS mount structures (`<sys/malloc.h>`; `bsd`'s `sys/malloc.rs` until N7
/// moves the header).
pub const M_ISOFSMNT: i32 = 57;

/// `enum ISO_FTYPE`: CD-ROM format type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)] // OpenBSD names, verbatim
pub enum IsoFtype {
    /// Plain ISO 9660 with version numbers stripped and associated files merged.
    ISO_FTYPE_DEFAULT,
    /// Plain ISO 9660 names as they are (`ISOFSMNT_GENS`).
    ISO_FTYPE_9660,
    /// Rock Ridge.
    ISO_FTYPE_RRIP,
    /// ECMA-168.
    ISO_FTYPE_ECMA,
}

pub use IsoFtype::{ISO_FTYPE_9660, ISO_FTYPE_DEFAULT, ISO_FTYPE_ECMA, ISO_FTYPE_RRIP};

/// `ISOFSMNT_ROOT`.
pub const ISOFSMNT_ROOT: i32 = 0;

/// `struct iso_mnt`: a mounted ISO 9660 file system, built whole by `iso_mountfs` and not
/// changed afterwards (see the module's redesign).
pub struct IsoMnt<G: ProcGlue> {
    /// `im_flags`: the `ISOFSMNT_*` mount flags in effect.
    pub im_flags: i32,
    /// `im_mountp`.
    pub im_mountp: &'static Mount<G>,
    /// `im_dev`.
    pub im_dev: Dev,
    /// `im_devvp`.
    pub im_devvp: &'static Vnode<G>,
    /// `logical_block_size`.
    pub logical_block_size: i32,
    /// `im_bshift`.
    pub im_bshift: i32,
    /// `im_bmask`.
    pub im_bmask: i32,
    /// `volume_space_size`.
    pub volume_space_size: i32,
    /// `root`: a copy of the root directory record.
    pub root: [u8; isodcl(157, 190)],
    /// `root_extent`.
    pub root_extent: u32,
    /// `root_size`.
    pub root_size: u32,
    /// `iso_ftype`.
    pub iso_ftype: IsoFtype,
    /// `rr_skip`: the bytes to skip before the SUSP entries of a record.
    pub rr_skip: i32,
    /// `rr_skip0`: the same, for the root directory's `.` entry.
    pub rr_skip0: i32,
    /// `joliet_level`.
    pub joliet_level: i32,
}

/// A counted handle of a published [`IsoMnt`]: the mount's slot holds one, each node one.
pub type IsoMntRef<G> = MArc<IsoMnt<G>>;

impl<G: ProcGlue> IsoMnt<G> {
    /// Publishes the structure `iso_mountfs` built in a counted `malloc(M_ISOFSMNT,
    /// M_WAITOK)` block (which cannot fail: `panic(9)` if it does).
    pub fn alloc(self) -> IsoMntRef<G> {
        match MArc::new(M_ISOFSMNT, M_WAITOK, self) {
            Some(imp) => imp,
            None => kpanic(format_args!("cd9660: malloc(M_ISOFSMNT) failed")),
        }
    }
}

/// `VFSTOISOFS(mp)`: the ISO 9660 data of a mounted cd9660 file system, a counted handle
/// (`panic(9)` on a mount that has none: not a mounted cd9660).
pub fn vfstoisofs<G: ProcGlue>(mp: &Mount<G>) -> IsoMntRef<G> {
    match mp.node_arc::<IsoMnt<G>>() {
        Some(imp) => imp,
        None => kpanic(format_args!(
            "VFSTOISOFS: mount {mp:p} is not a mounted cd9660"
        )),
    }
}

/// `blkoff(imp, loc)`.
pub fn blkoff<G: ProcGlue>(imp: &IsoMnt<G>, loc: i64) -> i64 {
    loc & i64::from(imp.im_bmask)
}

/// `lblktosize(imp, blk)`.
pub fn lblktosize<G: ProcGlue>(imp: &IsoMnt<G>, blk: i64) -> i64 {
    blk << imp.im_bshift
}

/// `lblkno(imp, loc)`.
pub fn lblkno<G: ProcGlue>(imp: &IsoMnt<G>, loc: i64) -> i64 {
    loc >> imp.im_bshift
}

/// `blksize(imp, ip, lbn)`: every block is a logical block.
pub fn blksize<G: ProcGlue>(imp: &IsoMnt<G>) -> i32 {
    imp.logical_block_size
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::isofs::cd9660::testcd9660::{attach_image, image, test_mnt};

    #[test]
    fn the_block_macros_use_the_logical_block() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        assert_eq!(blkoff(&imp, 4097), 1);
        assert_eq!(lblktosize(&imp, 3), 6144);
        assert_eq!(lblkno(&imp, 6143), 2);
        // a negative offset is an extended attribute record before the data
        assert_eq!(lblkno(&imp, -2048), -1);
        assert_eq!(blksize(&imp), 2048);
    }

    #[test]
    fn the_mount_slot_hands_out_the_structure() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_RRIP, |m| m.joliet_level = 2);
        imp.im_mountp.set_node(MArc::erase(imp.clone()));
        let got = vfstoisofs(imp.im_mountp);
        assert!(core::ptr::eq(&*got, &*imp));
        assert_eq!(got.joliet_level, 2);
        assert_eq!(got.iso_ftype, ISO_FTYPE_RRIP);
        drop(imp.im_mountp.take_node());
    }

    #[test]
    fn vfstoisofs_refuses_a_mount_without_the_structure() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        let mp = imp.im_mountp;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| vfstoisofs(mp)));
        assert!(r.is_err());
    }
}
/* </TESTS> */
