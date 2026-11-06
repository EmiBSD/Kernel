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
/* </LICENSES> */

/* <CODE> */
//! The kernel cd9660's test fixtures (host tests and benches only): the records and the
//! 25 blocks of an image made by OpenBSD's makefs (`makefs -t cd9660 -o
//! rockridge,label=M10C` of a tree with `m10c-iso.txt`, holding "m10c-iso-42\n", and `sub/`
//! with a symbolic link `link -> ../m10c-iso.txt` and `Mixed_Case-name.long.txt`), moved
//! from LZ's `iso.rs` tests (N5.8c1), the real writer's cross-check; [`mkiso`], the writer
//! of the images no host tool gives (a Joliet tree, a session past block 0, an associated
//! file, a directory of two blocks, a record past 2 GiB, a device node, a symbolic link of
//! size 0: N5.8c2); the mounts of either over testffs's disk by `iso_mountfs`
//! ([`try_mount`], [`mount_root`]), a sparse disk for the image past 2 GiB
//! ([`attach_sparse`]); and, for the node-level tests that need no mount, a mount structure
//! of the makefs image and nodes of it with vnodes of a small table ([`TestIsoVops`]: the
//! lock, the block map and the strategy, through the device).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::sys::errno::Errno;
use hal::sys::mount::{IsoArgs, MNT_LOCAL, MNT_RDONLY, MOUNT_CD9660};
use hal::sys::types::Daddr;

use crate::fs::bio::biodone;
use crate::fs::buf::{B_READ, BufRef};
use crate::fs::isofs::cd9660::cd9660_bmap::cd9660_bmap;
use crate::fs::isofs::cd9660::cd9660_extern::{IsoFtype, IsoMnt, IsoMntRef};
use crate::fs::isofs::cd9660::cd9660_node::{
    IsoNode, IsoNodeRef, cd9660_inactive, cd9660_init, cd9660_reclaim, vtoi,
};
use crate::fs::isofs::cd9660::cd9660_vfsops::{Cd9660Vfs, cd9660_root, iso_mountfs};
use crate::fs::isofs::cd9660::iso::{Cdino, IsoDirectoryRecord};
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc};
use crate::fs::subr::{getnewvnode, vref};
use crate::fs::testfs::{TEST_CONF, TestGlue, set_bdev_strategy};
use crate::fs::ufs::ffs::testffs;
use crate::fs::vnode::{LK_RWFLAGS, VT_ISOFS, Vnode};
use crate::fs::vops::{Cred, IoBuf, Vnops, vop_strategy};
use crate::proc::Thread;

/// `DEV_BSIZE`.
const DEV_BSIZE: usize = 512;

/// The makefs image's records and blocks.
pub mod image {
    use std::vec::Vec;

    /// The root directory's `.` record (SP, ER and a CE to block 24).
    pub const ROOT_DOT: [u8; 254] = [
        0xfe, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x01, 0x01, 0x00, 0x53, 0x50, 0x07, 0x01, 0xbe, 0xef, 0x00, 0x45, 0x52, 0xb9, 0x01,
        0x0a, 0x49, 0x5e, 0x01, 0x49, 0x45, 0x45, 0x45, 0x5f, 0x50, 0x31, 0x32, 0x38, 0x32, 0x54,
        0x48, 0x45, 0x20, 0x49, 0x45, 0x45, 0x45, 0x20, 0x50, 0x31, 0x32, 0x38, 0x32, 0x20, 0x50,
        0x52, 0x4f, 0x54, 0x4f, 0x43, 0x4f, 0x4c, 0x20, 0x50, 0x52, 0x4f, 0x56, 0x49, 0x44, 0x45,
        0x53, 0x20, 0x53, 0x55, 0x50, 0x50, 0x4f, 0x52, 0x54, 0x20, 0x46, 0x4f, 0x52, 0x20, 0x50,
        0x4f, 0x53, 0x49, 0x58, 0x20, 0x46, 0x49, 0x4c, 0x45, 0x20, 0x53, 0x59, 0x53, 0x54, 0x45,
        0x4d, 0x20, 0x53, 0x45, 0x4d, 0x41, 0x4e, 0x54, 0x49, 0x43, 0x53, 0x2e, 0x50, 0x4c, 0x45,
        0x41, 0x53, 0x45, 0x20, 0x43, 0x4f, 0x4e, 0x54, 0x41, 0x43, 0x54, 0x20, 0x54, 0x48, 0x45,
        0x20, 0x49, 0x45, 0x45, 0x45, 0x20, 0x53, 0x54, 0x41, 0x4e, 0x44, 0x41, 0x52, 0x44, 0x53,
        0x20, 0x44, 0x45, 0x50, 0x41, 0x52, 0x54, 0x4d, 0x45, 0x4e, 0x54, 0x2c, 0x20, 0x50, 0x49,
        0x53, 0x43, 0x41, 0x54, 0x41, 0x57, 0x41, 0x59, 0x2c, 0x20, 0x4e, 0x4a, 0x2c, 0x20, 0x55,
        0x53, 0x41, 0x20, 0x46, 0x4f, 0x52, 0x20, 0x54, 0x48, 0x45, 0x20, 0x50, 0x31, 0x32, 0x38,
        0x32, 0x20, 0x53, 0x50, 0x45, 0x43, 0x49, 0x46, 0x49, 0x43, 0x41, 0x54, 0x49, 0x4f, 0x4e,
        0x2e, 0x43, 0x45, 0x1c, 0x01, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24,
    ];

    /// The root directory's `..` record (PX).
    pub const ROOT_DOTDOT: [u8; 70] = [
        0x46, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x01, 0x01, 0x01, 0x50, 0x58, 0x24, 0x01, 0xed, 0x41, 0x00, 0x00, 0x00, 0x00, 0x41,
        0xed, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// `M10C_ISO.TXT;1`: PX, TF, NM "m10c-iso.txt".
    pub const FILE_REC: [u8; 128] = [
        0x80, 0x00, 0x17, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17, 0x0c, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x0c, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x01, 0x0e, 0x4d, 0x31, 0x30, 0x43, 0x5f, 0x49, 0x53, 0x4f, 0x2e, 0x54, 0x58, 0x54,
        0x3b, 0x31, 0x00, 0x50, 0x58, 0x24, 0x01, 0xa4, 0x81, 0x00, 0x00, 0x00, 0x00, 0x81, 0xa4,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54, 0x46, 0x1a, 0x01, 0x07, 0x7e,
        0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x7e, 0x0a,
        0x04, 0x07, 0x24, 0x30, 0xf4, 0x4e, 0x4d, 0x11, 0x01, 0x00, 0x6d, 0x31, 0x30, 0x63, 0x2d,
        0x69, 0x73, 0x6f, 0x2e, 0x74, 0x78, 0x74, 0x00,
    ];

    /// `SUB`: a directory with PX, TF, NM "sub".
    pub const SUB_REC: [u8; 106] = [
        0x6a, 0x00, 0x15, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x08, 0x00, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x02, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x01, 0x03, 0x53, 0x55, 0x42, 0x50, 0x58, 0x24, 0x01, 0xed, 0x41, 0x00, 0x00, 0x00,
        0x00, 0x41, 0xed, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54, 0x46, 0x1a,
        0x01, 0x07, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30,
        0xf4, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x4e, 0x4d, 0x08, 0x01, 0x00, 0x73, 0x75,
        0x62,
    ];

    /// `/SUB/LINK.;1`: PX, TF, SL "../m10c-iso.txt", NM "link".
    pub const LINK_REC: [u8; 132] = [
        0x84, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x16, 0x0f, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x0f, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x01, 0x07, 0x4c, 0x49, 0x4e, 0x4b, 0x2e, 0x3b, 0x31, 0x50, 0x58, 0x24, 0x01, 0xed,
        0xa1, 0x00, 0x00, 0x00, 0x00, 0xa1, 0xed, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x54, 0x46, 0x1a, 0x01, 0x07, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x7e, 0x0a,
        0x04, 0x07, 0x24, 0x30, 0xf4, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x53, 0x4c, 0x15,
        0x01, 0x00, 0x04, 0x00, 0x00, 0x0c, 0x6d, 0x31, 0x30, 0x63, 0x2d, 0x69, 0x73, 0x6f, 0x2e,
        0x74, 0x78, 0x74, 0x4e, 0x4d, 0x09, 0x01, 0x00, 0x6c, 0x69, 0x6e, 0x6b,
    ];

    /// `/SUB/MIXED_CASE_NAME.LONG_TXT;1`: NM "Mixed_Case-name.long.txt".
    pub const MIXED_REC: [u8; 152] = [
        0x98, 0x00, 0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x16, 0x06, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x06, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x00, 0x00, 0x00, 0x01, 0x00,
        0x00, 0x01, 0x1a, 0x4d, 0x49, 0x58, 0x45, 0x44, 0x5f, 0x43, 0x41, 0x53, 0x45, 0x5f, 0x4e,
        0x41, 0x4d, 0x45, 0x2e, 0x4c, 0x4f, 0x4e, 0x47, 0x5f, 0x54, 0x58, 0x54, 0x3b, 0x31, 0x00,
        0x50, 0x58, 0x24, 0x01, 0xa4, 0x81, 0x00, 0x00, 0x00, 0x00, 0x81, 0xa4, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54, 0x46, 0x1a, 0x01, 0x07, 0x7e, 0x0a, 0x04, 0x07,
        0x24, 0x30, 0xf4, 0x7e, 0x0a, 0x04, 0x07, 0x24, 0x30, 0xf4, 0x7e, 0x0a, 0x04, 0x07, 0x24,
        0x30, 0xf4, 0x4e, 0x4d, 0x1d, 0x01, 0x00, 0x4d, 0x69, 0x78, 0x65, 0x64, 0x5f, 0x43, 0x61,
        0x73, 0x65, 0x2d, 0x6e, 0x61, 0x6d, 0x65, 0x2e, 0x6c, 0x6f, 0x6e, 0x67, 0x2e, 0x74, 0x78,
        0x74, 0x00,
    ];

    /// The root `.` record's continuation area (block 24, 36 bytes: PX).
    pub const ROOT_CE: [u8; 36] = [
        0x50, 0x58, 0x24, 0x01, 0xed, 0x41, 0x00, 0x00, 0x00, 0x00, 0x41, 0xed, 0x04, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    /// The first 190 bytes of the primary volume descriptor.
    pub const PVD_HEAD: [u8; 190] = [
        0x01, 0x43, 0x44, 0x30, 0x30, 0x31, 0x01, 0x00, 0x4f, 0x70, 0x65, 0x6e, 0x42, 0x53, 0x44,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x4d, 0x31, 0x30, 0x43, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x08, 0x08, 0x00, 0x16, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x16, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x22, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x14, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x02, 0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x01, 0x00,
    ];

    /// The nonzero bytes of the image's first 25 blocks (its volume space), as runs at
    /// their byte offsets.
    pub const IMAGE_RUNS: &[(usize, &[u8])] = &[
        (
            32768,
            b"\x01CD001\x01\x00OpenBSD                         M10C                       \
          \x20    ",
        ),
        (32848, b"\x19\x00\x00\x00\x00\x00\x00\x19"),
        (
            32888,
            b"\x01\x00\x00\x01\x01\x00\x00\x01\x00\x08\x08\x00\x16\x00\x00\x00\x00\x00\x00\
          \x16\x12",
        ),
        (
            32919,
            b"\x13\x00\x00\x00\x00\x22\x00\x14\x00\x00\x00\x00\x00\x00\x14\x00\x08\x00\x00\
          \x00\x00\x08",
        ),
        (
            32949,
            b"\x02\x00\x00\x01\x00\x00\x01\x01\x00                                        \
          \x20                                                                        \
          \x20                                                                        \
          \x20                                                                        \
          \x20                                                                        \
          \x20                                                                        \
          \x20                                                                        \
          \x20                                                                        \
          \x20                                                                       2\
          026100407365200\xf42026100407365200\xf40000000000000000\x002026100407365200\
          \xf4\x01",
        ),
        (34816, b"\xffCD001\x01"),
        (
            36864,
            b"\x01\x00\x14\x00\x00\x00\x01\x00\x00\x00\x03\x00\x15\x00\x00\x00\x01\x00SUB",
        ),
        (
            38912,
            b"\x01\x00\x00\x00\x00\x14\x00\x01\x00\x00\x03\x00\x00\x00\x00\x15\x00\x01SUB",
        ),
        (
            40960,
            b"\xfe\x00\x14\x00\x00\x00\x00\x00\x00\x14\x00\x08\x00\x00\x00\x00\x08",
        ),
        (
            40985,
            b"\x02\x00\x00\x01\x00\x00\x01\x01\x00SP\x07\x01\xbe\xef\x00ER\xb9\x01\x0aI^\
          \x01IEEE_P1282THE IEEE P1282 PROTOCOL PROVIDES SUPPORT FOR POSIX FILE SYSTEM\
          \x20SEMANTICS.PLEASE CONTACT THE IEEE STANDARDS DEPARTMENT, PISCATAWAY, NJ, \
          USA FOR THE P1282 SPECIFICATION.CE\x1c\x01\x18\x00\x00\x00\x00\x00\x00\x18",
        ),
        (
            41206,
            b"$\x00\x00\x00\x00\x00\x00$F\x00\x14\x00\x00\x00\x00\x00\x00\x14\x00\x08\x00\
          \x00\x00\x00\x08",
        ),
        (
            41239,
            b"\x02\x00\x00\x01\x00\x00\x01\x01\x01PX$\x01\xedA\x00\x00\x00\x00A\xed\x04\
          \x00\x00\x00\x00\x00\x00\x04",
        ),
        (
            41284,
            b"\x80\x00\x17\x00\x00\x00\x00\x00\x00\x17\x0c\x00\x00\x00\x00\x00\x00\x0c~\
          \x0a\x04\x07$0\xf4\x00\x00\x00\x01\x00\x00\x01\x0eM10C_ISO.TXT;1\x00PX$\x01\
          \xa4\x81\x00\x00\x00\x00\x81\xa4\x01\x00\x00\x00\x00\x00\x00\x01",
        ),
        (
            41368,
            b"TF\x1a\x01\x07~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4NM\
          \x11\x01\x00m10c-iso.txt\x00j\x00\x15\x00\x00\x00\x00\x00\x00\x15\x00\x08\
          \x00\x00\x00\x00\x08\x00~\x0a\x04\x07$0\xf4\x02\x00\x00\x01\x00\x00\x01\x03S\
          UBPX$\x01\xedA\x00\x00\x00\x00A\xed\x04\x00\x00\x00\x00\x00\x00\x04",
        ),
        (
            41484,
            b"TF\x1a\x01\x07~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4NM\
          \x08\x01\x00sub",
        ),
        (
            43008,
            b"F\x00\x15\x00\x00\x00\x00\x00\x00\x15\x00\x08\x00\x00\x00\x00\x08\x00~\x0a\
          \x04\x07$0\xf4\x02\x00\x00\x01\x00\x00\x01\x01\x00PX$\x01\xedA\x00\x00\x00\
          \x00A\xed\x04\x00\x00\x00\x00\x00\x00\x04",
        ),
        (
            43078,
            b"F\x00\x14\x00\x00\x00\x00\x00\x00\x14\x00\x08\x00\x00\x00\x00\x08",
        ),
        (
            43103,
            b"\x02\x00\x00\x01\x00\x00\x01\x01\x01PX$\x01\xedA\x00\x00\x00\x00A\xed\x04\
          \x00\x00\x00\x00\x00\x00\x04",
        ),
        (
            43148,
            b"\x84\x00\x16\x00\x00\x00\x00\x00\x00\x16\x0f\x00\x00\x00\x00\x00\x00\x0f~\
          \x0a\x04\x07$0\xf4\x00\x00\x00\x01\x00\x00\x01\x07LINK.;1PX$\x01\xed\xa1\x00\
          \x00\x00\x00\xa1\xed\x01\x00\x00\x00\x00\x00\x00\x01",
        ),
        (
            43224,
            b"TF\x1a\x01\x07~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4SL\
          \x15\x01\x00\x04\x00\x00\x0cm10c-iso.txtNM\x09\x01\x00link\x98\x00\x16\x00\
          \x00\x00\x00\x00\x00\x16\x06\x00\x00\x00\x00\x00\x00\x06~\x0a\x04\x07$0\xf4\
          \x00\x00\x00\x01\x00\x00\x01\x1aMIXED_CASE_NAME.LONG_TXT;1\x00PX$\x01\xa4\
          \x81\x00\x00\x00\x00\x81\xa4\x01\x00\x00\x00\x00\x00\x00\x01",
        ),
        (
            43376,
            b"TF\x1a\x01\x07~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4~\x0a\x04\x07$0\xf4NM\
          \x1d\x01\x00Mixed_Case-name.long.txt",
        ),
        (45056, b"hello\x0a"),
        (47104, b"m10c-iso-42\x0a"),
        (
            49152,
            b"PX$\x01\xedA\x00\x00\x00\x00A\xed\x04\x00\x00\x00\x00\x00\x00\x04",
        ),
    ];
    /// The image's size: 25 blocks of 2048 bytes.
    pub const IMAGE_SIZE: usize = 25 * 2048;

    /// The image: its volume space, zeros but for the runs.
    pub fn image_bytes() -> Vec<u8> {
        let mut img = std::vec![0u8; IMAGE_SIZE];
        for &(off, b) in IMAGE_RUNS {
            img[off..off + b.len()].copy_from_slice(b);
        }
        img
    }
}

/// The vnode table of the fixture's nodes: the node's lock, the block map, the strategy
/// (mapped through `cd9660_bmap` to the device, `ufs_strategy`'s shape), `access` granted,
/// and cd9660's `inactive` and `reclaim`; every other slot the C's NULL.
pub struct TestIsoVops;

impl Vnops<TestGlue> for TestIsoVops {
    fn lock(&self, vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        vtoi(vp).i_lock.enter(flags & LK_RWFLAGS)
    }
    fn unlock(&self, vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        vtoi(vp).i_lock.exit();
        Ok(())
    }
    fn islocked(&self, vp: &'static Vnode<TestGlue>) -> i32 {
        vtoi(vp).i_lock.status()
    }
    fn access(
        &self,
        _vp: &'static Vnode<TestGlue>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<TestGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn inactive(
        &self,
        vp: &'static Vnode<TestGlue>,
        p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        cd9660_inactive(vp, p)
    }
    fn reclaim(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        cd9660_reclaim(vp)
    }
    fn bmap(
        &self,
        vp: &'static Vnode<TestGlue>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<TestGlue>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        cd9660_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, vp: &'static Vnode<TestGlue>, bp: IoBuf) -> Result<(), Errno> {
        let Some(t): Option<BufRef> = bp.into_token() else {
            std::panic!("testcd9660: a lent buffer");
        };
        let b = t.buf();
        if b.blkno() == b.lblkno() {
            let mut bn = 0;
            cd9660_bmap(vp, b.lblkno(), None, Some(&mut bn), None)?;
            b.set_blkno(bn);
        }
        let devvp = vtoi(vp).devvp();
        b.set_dev(devvp.rdev());
        vop_strategy(devvp, IoBuf::Token(t))
    }
}

/// The table, a `static` (one address for the registry, D-b2-10).
pub static TEST_ISO_VOPS: TestIsoVops = TestIsoVops;

/// `image` as the disk of testffs's block device, and cd9660's hash made: the memory guard
/// and the test thread.
pub fn attach_image(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = testffs::attach(image);
    if let Err(e) = cd9660_init::<TestGlue>() {
        std::panic!("testcd9660: cd9660_init: {e:?}");
    }
    (g, p)
}

/// The mount structure of the makefs image as `iso_mountfs` builds it (2048-byte blocks, 25
/// of them, the root at block 20, no Rock Ridge skip) on a fresh mount of testfs's type and
/// testffs's device, changed by `f`, published (not hung on the mount).
pub fn test_mnt(ftype: IsoFtype, f: impl FnOnce(&mut IsoMnt<TestGlue>)) -> IsoMntRef<TestGlue> {
    let devvp = testffs::devvp();
    let mut root = [0u8; 34];
    root.copy_from_slice(&image::PVD_HEAD[156..190]);
    let mut imp = IsoMnt {
        im_flags: 0,
        im_mountp: vfs_mount_alloc(None, &TEST_CONF[0]),
        im_dev: devvp.rdev(),
        im_devvp: devvp,
        logical_block_size: 2048,
        im_bshift: 11,
        im_bmask: 2047,
        volume_space_size: 25,
        root,
        root_extent: 20,
        root_size: 2048,
        iso_ftype: ftype,
        rr_skip: 0,
        rr_skip0: 0,
        joliet_level: 0,
    };
    f(&mut imp);
    imp.alloc()
}

/// A node of `imp` numbered `ino` with a vnode of its own of [`TEST_ISO_VOPS`] (referenced,
/// unlocked), its device vnode referenced, as `cd9660_vget_internal` links them; not hashed.
pub fn test_node(
    imp: &IsoMntRef<TestGlue>,
    ino: Cdino,
) -> (IsoNodeRef<TestGlue>, &'static Vnode<TestGlue>) {
    let vp = match getnewvnode(VT_ISOFS, Some(imp.im_mountp), &TEST_ISO_VOPS) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testcd9660: getnewvnode: {e:?}"),
    };
    let ip = IsoNode::new(vp, imp.im_dev, ino, imp.clone()).alloc();
    vp.set_node(hal::malloc::MArc::erase(ip.clone()));
    ip.i_devvp.set(Some(imp.im_devvp));
    vref(imp.im_devvp);
    (ip, vp)
}

/// A record view of a fixture.
pub fn rec(b: &[u8]) -> IsoDirectoryRecord<'_> {
    match IsoDirectoryRecord::new(b) {
        Some(r) => r,
        None => std::panic!("testcd9660: not a record"),
    }
}

/// `mkiso`: an ISO 9660 writer of the tests' images, block by block: the volume
/// descriptors, directories of records packed as the standard packs them (no record across
/// a logical block), files, and the Rock Ridge entries a record's system use area holds.
/// It writes what the tests need and no release medium needs (xtask's `iso9660.rs` is that
/// writer, D-N5.8-8): no path tables (the kernel never reads them), no `TF` (a Rock Ridge
/// file without one takes the record's date).
pub mod mkiso {
    use std::collections::BTreeMap;
    use std::vec::Vec;

    /// A logical block.
    pub const BS: usize = 2048;

    /// The record date of every record: 2026-10-04 07:36:48 at GMT-3 (makefs's image's).
    const DATE: [u8; 7] = [126, 10, 4, 7, 36, 48, 0xf4];

    /// A 7.3.3 (both-endian) 32-bit number.
    pub fn n733(v: u32) -> [u8; 8] {
        let (l, b) = (v.to_le_bytes(), v.to_be_bytes());
        [l[0], l[1], l[2], l[3], b[0], b[1], b[2], b[3]]
    }

    /// A 7.2.3 (both-endian) 16-bit number.
    pub fn n723(v: u16) -> [u8; 4] {
        let (l, b) = (v.to_le_bytes(), v.to_be_bytes());
        [l[0], l[1], b[0], b[1]]
    }

    /// A directory record: the data's first block `extent` and its `size` in bytes, the
    /// `flags` (2 a directory, 4 an associated file), the ISO `name` (`[0]` for `.`, `[1]` for
    /// `..`) and the system use area `su`, padded as the standard pads them.
    pub fn record(extent: u32, size: u32, flags: u8, name: &[u8], su: &[u8]) -> Vec<u8> {
        let mut r = std::vec![0u8; 33];
        r[2..10].copy_from_slice(&n733(extent));
        r[10..18].copy_from_slice(&n733(size));
        r[18..25].copy_from_slice(&DATE);
        r[25] = flags;
        r[28..32].copy_from_slice(&n723(1));
        r[32] = name.len() as u8;
        r.extend_from_slice(name);
        if name.len() % 2 == 0 {
            r.push(0);
        }
        r.extend_from_slice(su);
        if r.len() % 2 == 1 {
            r.push(0);
        }
        r[0] = r.len() as u8;
        r
    }

    /// A SUSP entry: its signature, length, version 1 and `body`.
    pub fn susp(sig: &[u8; 2], body: &[u8]) -> Vec<u8> {
        let mut e = std::vec![sig[0], sig[1], (4 + body.len()) as u8, 1];
        e.extend_from_slice(body);
        e
    }

    /// `SP`: the SUSP indicator of the root's `.` (no bytes skipped).
    pub fn sp() -> Vec<u8> {
        susp(b"SP", &[0xbe, 0xef, 0])
    }

    /// `ER`: the Rock Ridge extension reference.
    pub fn er() -> Vec<u8> {
        let mut b = std::vec![10, 0, 0, 1];
        b.extend_from_slice(b"RRIP_1991A");
        susp(b"ER", &b)
    }

    /// `PX`: the POSIX mode, links, owner and group.
    pub fn px(mode: u32, links: u32, uid: u32, gid: u32) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [mode, links, uid, gid] {
            b.extend_from_slice(&n733(v));
        }
        susp(b"PX", &b)
    }

    /// `NM`: the alternate (POSIX) name.
    pub fn nm(name: &[u8]) -> Vec<u8> {
        let mut b = std::vec![0];
        b.extend_from_slice(name);
        susp(b"NM", &b)
    }

    /// `PN`: a device's number, its high and low halves.
    pub fn pn(high: u32, low: u32) -> Vec<u8> {
        let mut b = n733(high).to_vec();
        b.extend_from_slice(&n733(low));
        susp(b"PN", &b)
    }

    /// `CE`: a continuation area of `len` bytes at byte `off` of block `blk`.
    pub fn ce(blk: u32, off: u32, len: u32) -> Vec<u8> {
        let mut b = n733(blk).to_vec();
        b.extend_from_slice(&n733(off));
        b.extend_from_slice(&n733(len));
        susp(b"CE", &b)
    }

    /// `SL`: a symbolic link's components, each its flags (4 the parent, 2 the current
    /// directory, 0 a name) and its bytes.
    pub fn sl(components: &[(u8, &[u8])]) -> Vec<u8> {
        let mut b = std::vec![0];
        for (flags, name) in components {
            b.push(*flags);
            b.push(name.len() as u8);
            b.extend_from_slice(name);
        }
        susp(b"SL", &b)
    }

    /// A Joliet name: `name`'s UCS-2 big-endian bytes.
    pub fn ucs2(name: &[u8]) -> Vec<u8> {
        name.iter().flat_map(|&c| [0, c]).collect()
    }

    /// An image of `nblocks` logical blocks, written block by block (blocks never written
    /// read as zeros).
    pub struct Iso {
        /// The volume's size in blocks.
        pub nblocks: u32,
        /// The written blocks.
        pub blocks: BTreeMap<u32, [u8; BS]>,
    }

    impl Iso {
        /// An empty image.
        pub fn new(nblocks: u32) -> Self {
            Self {
                nblocks,
                blocks: BTreeMap::new(),
            }
        }

        /// `bytes` at byte `off` of block `blk` (across the following blocks).
        pub fn put(&mut self, blk: u32, off: usize, bytes: &[u8]) {
            for (i, &b) in bytes.iter().enumerate() {
                let at = off + i;
                let blk = blk + (at / BS) as u32;
                self.blocks.entry(blk).or_insert([0; BS])[at % BS] = b;
            }
        }

        /// A volume descriptor's head at block `at`: its type, `CD001`, version 1.
        fn vd(&mut self, at: u32, type_: u8) {
            self.put(at, 0, &[type_]);
            self.put(at, 1, b"CD001\x01");
        }

        /// The primary volume descriptor at block `at`: `volume_id` (space-padded to 32
        /// bytes), the volume's size `space` in blocks, 2,048-byte logical blocks, the root
        /// directory's record `root`.
        pub fn pvd(&mut self, at: u32, volume_id: &[u8], space: u32, root: &[u8]) {
            self.vd(at, 1);
            let mut id = [b' '; 32];
            id[..volume_id.len()].copy_from_slice(volume_id);
            self.put(at, 40, &id);
            self.put(at, 80, &n733(space));
            self.put(at, 120, &n723(1));
            self.put(at, 124, &n723(1));
            self.put(at, 128, &n723(BS as u16));
            self.put(at, 156, root);
        }

        /// A supplementary (Joliet) volume descriptor at block `at`: its `escape` sequence,
        /// `flags`, the volume's size and the root directory's record `root`.
        pub fn svd(&mut self, at: u32, escape: &[u8; 3], flags: u8, space: u32, root: &[u8]) {
            self.vd(at, 2);
            self.put(at, 7, &[flags]);
            self.put(at, 80, &n733(space));
            self.put(at, 88, escape);
            self.put(at, 128, &n723(BS as u16));
            self.put(at, 156, root);
        }

        /// The volume descriptor set terminator at block `at`.
        pub fn term(&mut self, at: u32) {
            self.vd(at, 255);
        }

        /// The directory `records` packed from block `at` (a record that does not fit in
        /// the rest of a block starts the next one); its size in bytes, whole blocks.
        pub fn dir(&mut self, at: u32, records: &[Vec<u8>]) -> u32 {
            let (mut blk, mut off) = (at, 0);
            for r in records {
                if off + r.len() > BS {
                    blk += 1;
                    off = 0;
                }
                self.put(blk, off, r);
                off += r.len();
            }
            self.blocks.entry(blk).or_insert([0; BS]);
            (blk - at + 1) * BS as u32
        }

        /// The image's bytes.
        pub fn finish(&self) -> Vec<u8> {
            let mut img = std::vec![0u8; self.nblocks as usize * BS];
            for (&b, data) in &self.blocks {
                let at = b as usize * BS;
                img[at..at + BS].copy_from_slice(data);
            }
            img
        }
    }
}

/// The fixture's configuration entry: the kernel's cd9660 as `vfsconflist[]` has it
/// (`"cd9660"`, type 14, `struct iso_args`, `MNT_LOCAL`).
pub static TEST_CD9660_CONF: Vfsconf<TestGlue> = Vfsconf::new(
    &Cd9660Vfs::<TestGlue>::VFS,
    MOUNT_CD9660,
    14,
    MNT_LOCAL,
    IsoArgs::SIZE,
);

/// Mount arguments of `flags` (`ISOFSMNT_*`) and session `sess`, no device name.
pub fn args(flags: i32, sess: i32) -> IsoArgs {
    let Some(mut a) = IsoArgs::from_bytes(&[0u8; IsoArgs::SIZE]) else {
        std::panic!("testcd9660: no iso_args");
    };
    a.flags = flags;
    a.sess = sess;
    a
}

/// A new read-only mount of the disk by `iso_mountfs` with the arguments `a` (whose flags it
/// changes as the C's), as `cd9660_mount` makes one: the statistics, the mount unbusied, not
/// on the mount list; `Err` freed the mount (`testffs::mount_conf`).
pub fn try_mount(p: &Thread<TestGlue>, a: &mut IsoArgs) -> Result<&'static Mount<TestGlue>, Errno> {
    testffs::mount_conf(p, &TEST_CD9660_CONF, b"/cd", MNT_RDONLY, |devvp, mp| {
        iso_mountfs(devvp, mp, p, a)
    })
}

/// [`try_mount`] with `flags` at `/`, made the root as `main` mounts it
/// (`testffs::make_root`; `testffs::unmount_root` undoes it): the mount and the flags
/// `iso_mountfs` left.
pub fn mount_root(p: &'static Thread<TestGlue>, flags: i32) -> (&'static Mount<TestGlue>, i32) {
    let mut a = args(flags, 0);
    let mp = match try_mount(p, &mut a) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testcd9660: iso_mountfs: {e:?}"),
    };
    testffs::make_root(p, mp);
    (mp, a.flags)
}

/// The root directory's vnode of `mp`, locked and referenced (`vput` it): `VFS_ROOT`.
pub fn root(mp: &'static Mount<TestGlue>) -> &'static Vnode<TestGlue> {
    match cd9660_root(mp) {
        Ok(vp) => vp,
        Err(e) => std::panic!("testcd9660: no root: {e:?}"),
    }
}

std::thread_local! {
    /// The sparse disk's written logical blocks.
    static SPARSE: RefCell<BTreeMap<u32, [u8; mkiso::BS]>> = const { RefCell::new(BTreeMap::new()) };
}

/// The sparse disk's `d_strategy`: a read of the blocks [`attach_sparse`] wrote, zeros
/// elsewhere, then `biodone` (no write: a cd9660 writes nothing).
fn sparse_strategy(mut t: BufRef) {
    if !t.buf().isset(B_READ) {
        std::panic!("testcd9660: a write to the sparse disk");
    }
    let at = t.buf().blkno() as usize * DEV_BSIZE;
    let n = t.buf().bcount() as usize;
    let data = t.data();
    SPARSE.with(|s| {
        let s = s.borrow();
        for (i, b) in data[..n].iter_mut().enumerate() {
            let off = at + i;
            *b = s
                .get(&((off / mkiso::BS) as u32))
                .map_or(0, |blk| blk[off % mkiso::BS]);
        }
    });
    biodone(t);
}

/// `image`'s blocks as a sparse disk of testffs's block device (an image past 2 GiB, whose
/// bytes no test could hold), cd9660's hash made: [`attach_image`] with the sparse
/// strategy.
pub fn attach_sparse(image: &mkiso::Iso) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let r = attach_image(Vec::new());
    SPARSE.with(|s| *s.borrow_mut() = image.blocks.clone());
    set_bdev_strategy(Some(sparse_strategy));
    r
}
/* </CODE> */
