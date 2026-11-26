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

/*
 * Copyright (c) 1995 Jason R. Thorpe.  All rights reserved.
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This software was developed by the Computer Systems Engineering group
 * at Lawrence Berkeley Laboratory under DARPA contract BG 91-66 and
 * contributed to Berkeley.
 *
 * All advertising materials mentioning features or use of this software
 * must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Lawrence Berkeley Laboratory.
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
 * from: Header: disk.h,v 1.5 92/11/19 04:33:03 torek Exp  (LBL)
 *
 *	@(#)disk.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/disk.h>`'s part userland sees and the drivers share: the disk name length, `struct
//! diskstats` (`hw.diskstats`), the disk flags (`DKF_*`), the label states (`DK_*`) and the
//! disk map flags (`DM_*`).
//!
//! Upstream: sys/sys/disk.h @ 3ce1f3f79392
//! LZ: sys/sys/disk.rs@e8249449d905
//!
//! `struct disk` itself, `disklist` and the functions the header declares are the kernel's
//! (`kernel::dev::disk`).
//!
//! ## Deviations
//! - `struct diskstats` has an explicit `ds_pad0` where the C compiler inserts four bytes of
//!   padding before `ds_rxfer`: the layout is the C's (checked below) with no uninitialised
//!   bytes, so `hw.diskstats` copies it out as bytes. Its three `struct timeval`s are
//!   [`IfTimeval`], `hal`'s image of the kernel's `Timeval` (16 bytes on LP64).
//!
//! ## Redesign
//! - N7.1d: the header's constants and `struct diskstats` are `hal`'s, `AbiPod` through
//!   [`crate::abi_pod!`] here (it was in `bsd`'s `sys/sysctl.rs`), so the kernel's
//!   `hw.diskstats` (`kernel::base::sysctl`) and `bsd`'s drivers name one copy; `struct disk`
//!   moved into the kernel as a counted block (`kernel::dev::disk::Disk`) and `bsd`'s
//!   `sys/sys/disk.rs` is the drivers' shim over it.

use crate::sys::net_if::IfTimeval;

/// `DS_DISKNAMELEN`.
pub const DS_DISKNAMELEN: usize = 16;

/// `struct diskstats`: what `hw.diskstats` reports per disk.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Diskstats {
    /// `ds_name`.
    pub ds_name: [u8; DS_DISKNAMELEN],
    /// `ds_busy`: busy counter.
    pub ds_busy: i32,
    /// The four bytes of padding the C compiler puts here to align `ds_rxfer`.
    pub ds_pad0: u32,
    /// `ds_rxfer`: total number of read transfers.
    pub ds_rxfer: u64,
    /// `ds_wxfer`: total number of write transfers.
    pub ds_wxfer: u64,
    /// `ds_seek`: total independent seek operations.
    pub ds_seek: u64,
    /// `ds_rbytes`: total bytes read.
    pub ds_rbytes: u64,
    /// `ds_wbytes`: total bytes written.
    pub ds_wbytes: u64,
    /// `ds_attachtime`: time disk was attached.
    pub ds_attachtime: IfTimeval,
    /// `ds_timestamp`: time of first busy or any unbusy.
    pub ds_timestamp: IfTimeval,
    /// `ds_time`: total time spent busy.
    pub ds_time: IfTimeval,
}

crate::abi_pod!(Diskstats {
    ds_name: [u8; DS_DISKNAMELEN],
    ds_busy: i32,
    ds_pad0: u32,
    ds_rxfer: u64,
    ds_wxfer: u64,
    ds_seek: u64,
    ds_rbytes: u64,
    ds_wbytes: u64,
    ds_attachtime: IfTimeval,
    ds_timestamp: IfTimeval,
    ds_time: IfTimeval,
});

/// `DKF_CONSTRUCTED`.
pub const DKF_CONSTRUCTED: i32 = 0x0001;
/// `DKF_OPENED`.
pub const DKF_OPENED: i32 = 0x0002;
/// `DKF_NOLABELREAD`.
pub const DKF_NOLABELREAD: i32 = 0x0004;

// states

/// `DK_CLOSED`: drive is closed.
pub const DK_CLOSED: i32 = 0;
/// `DK_WANTOPEN`: drive being opened.
pub const DK_WANTOPEN: i32 = 1;
/// `DK_WANTOPENRAW`: drive being opened.
pub const DK_WANTOPENRAW: i32 = 2;
/// `DK_RDLABEL`: label being read.
pub const DK_RDLABEL: i32 = 3;
/// `DK_OPEN`: label read, drive open.
pub const DK_OPEN: i32 = 4;
/// `DK_OPENRAW`: open without label.
pub const DK_OPENRAW: i32 = 5;

// Disk map flags.

/// `DM_OPENPART`: Open raw partition.
pub const DM_OPENPART: i32 = 0x1;
/// `DM_OPENBLCK`: Open block device.
pub const DM_OPENBLCK: i32 = 0x2;

// The C layout (amd64 and arm64 are both LP64) `hw.diskstats` copies out.
const _: () = {
    use core::mem::{offset_of, size_of};
    assert!(size_of::<Diskstats>() == 112);
    assert!(offset_of!(Diskstats, ds_rxfer) == 24);
    assert!(offset_of!(Diskstats, ds_attachtime) == 64);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/disk.h");
        let ours: &[(&str, i64)] = &[
            ("DS_DISKNAMELEN", DS_DISKNAMELEN as i64),
            ("DKF_CONSTRUCTED", DKF_CONSTRUCTED.into()),
            ("DKF_OPENED", DKF_OPENED.into()),
            ("DKF_NOLABELREAD", DKF_NOLABELREAD.into()),
            ("DK_CLOSED", DK_CLOSED.into()),
            ("DK_OPENRAW", DK_OPENRAW.into()),
            ("DM_OPENPART", DM_OPENPART.into()),
            ("DM_OPENBLCK", DM_OPENBLCK.into()),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
