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
 * Copyright (c) 1993, 1994
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
 *	@(#)cd9660_rrip.h	8.2 (Berkeley) 12/5/94
 */
/*-
 * Copyright (c) 1993, 1994
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
 *	@(#)cd9660_rrip.c	8.6 (Berkeley) 12/5/94
 */
/* </LICENSES> */

/* <CODE> */
//! Rock Ridge Interchange Protocol: `<isofs/cd9660/cd9660_rrip.h>` (the System Use Sharing
//! Protocol entries as they are on the disc) and `isofs/cd9660/cd9660_rrip.c` (the walk over
//! a directory record's entries, and the entries' readers: POSIX attributes, device
//! numbers, symbolic links, alternate names, relocated directories, time stamps,
//! continuation areas and the extension reference).
//!
//! Upstream: sys/isofs/cd9660/cd9660_rrip.h @ 3ce1f3f79392
//! Upstream: sys/isofs/cd9660/cd9660_rrip.c @ 3ce1f3f79392
//! LZ: sys/isofs/cd9660/cd9660_rrip.rs@3c62ede68802
//!
//! The entries are `hal::abi_pod!` images of byte arrays (as `iso.rs` has the volume
//! descriptors), read out of the entry's bytes with `at(entry, 0)`; `at` failing is the C's
//! `isonum_711(p->h.length) < sizeof(*p)`, since an entry's slice is exactly its length.
//!
//! ## Deviations
//! - `RRIP_TABLE` is an array without the C's empty sentinel; the `func2` walk stops at the
//!   first entry without one, as the C's `for (...; ptable->func2; ...)` does.
//! - The output name (`ana->outbuf`, `ana->outlen`) is the buffer, position and length of
//!   [`IsoRripAnalyze`] (`iso_rrip.rs`); no byte is written past the buffer, and a write
//!   that would not fit is treated as the C treats a name past `maxlen`.
//! - `cd9660_rrip_loop` reads a record's entries within the record and its buffer, and a
//!   continuation area within the block read for it: the block number, offset and length of
//!   a `CE` entry are read unsigned (`isonum_733`'s value zero-extended), so a corrupt one
//!   cannot point before the buffer or before the volume, which the C's `int` arithmetic
//!   (sign-extended into `daddr_t` and `off_t`) could.
//! - The `CE` chain, the user's decision D-N5.8-U1 of 2026-11-05: the C follows `CE` entries
//!   without a bound and `bread`s each continuation block without releasing the previous one
//!   (cd9660_rrip.c:581-636: every buffer but the last stays busy until the system needs it,
//!   and a `CE` naming a block the walk holds busy makes `getblk` wait for its own buffer, a
//!   thread hung for ever with its vnode locked). Here the previous continuation buffer is
//!   released before the next `bread`; a `CE` naming the block the caller holds busy on the
//!   device (`IsoRripAnalyze::held`: `cd9660_vget_internal`'s, `cd9660_readlink`'s and
//!   `iso_mountfs`'s record block; `cd9660_lookup` and `cd9660_readdir` hold the directory's
//!   block on the directory's own vnode, a buffer a read of the device never waits for) or
//!   the continuation block being walked ends the walk as the C's read error does (the
//!   remaining fields take their defaults), and so does a chain longer than
//!   [`ISO_RRIP_MAXCE`] (64) areas. A legitimate disc is read as the C reads it; a crafted
//!   one gets default Rock Ridge fields where OpenBSD hangs.
//! - `hostname` is read out of `kernel::sysctl`'s `hostname` (`hostnamelen` bytes) only for
//!   a `HOST` component, as the C; the `VOLROOT` component copies `f_mntonname` out of the
//!   mount.
//!
//! ## Redesign
//! - LZ's `from_bytes_impl!` views of the entries (an `unsafe` cast each) -> `hal::abi_pod!`
//!   images read by `fs::pod`'s `pod_view!` (a copy of 4 to 36 bytes per entry read, where
//!   the C reads in place: D-N5.8-5's class).
//! - LZ's lent continuation buffer (`b.data()`'s `unsafe` view, held as `&'static`) ->
//!   `fs::bio`'s `BufRef` token, owned by the walk and released before the next `bread` and
//!   at the end (U1 above).
//! - The four tables are associated constants of [`RripTables`] (a `static` generic over the
//!   glue cannot exist; they are fixed tables of function pointers, never keyed by address).
//! - The attribute readers write the analysis's copy of the node's attributes
//!   (`IsoRripAnalyze::inode`, stored back once by [`cd9660_rrip_analyze`]), where LZ changed
//!   the node's `Cell` per entry. 0 `unsafe` (LZ: 15).

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use hal::sys::mount::MNAMELEN;
use hal::sys::syslimits::NAME_MAX;
use hal::sys::types::{Daddr, Dev, Off, major, makedev, minor};

use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::fs::bio::{DEV_BSHIFT, bread, brelse};
use crate::fs::buf::BufRef;
use crate::fs::isofs::cd9660::cd9660_extern::IsoMnt;
use crate::fs::isofs::cd9660::cd9660_node::{
    IsoNode, cd9660_defattr, cd9660_deftstamp, cd9660_tstamp_conv7, cd9660_tstamp_conv17,
};
use crate::fs::isofs::cd9660::cd9660_util::{isochar, isofntrans};
use crate::fs::isofs::cd9660::iso::{
    Cdino, ISO_DIRECTORY_RECORD_SIZE, IsoDirectoryRecord, isodcl, isonum_711, isonum_733,
};
use crate::fs::isofs::cd9660::iso_rrip::{
    ISO_SUSP_ALTNAME, ISO_SUSP_ATTR, ISO_SUSP_CLINK, ISO_SUSP_CONT, ISO_SUSP_DEVICE,
    ISO_SUSP_EXTREF, ISO_SUSP_IDFLAG, ISO_SUSP_PLINK, ISO_SUSP_RELDIR, ISO_SUSP_SLINK,
    ISO_SUSP_STOP, ISO_SUSP_TSTAMP, IsoRripAnalyze,
};
use crate::fs::namei::MAXPATHLEN;
use crate::fs::pod::pod_view;
use crate::prf::printf;
use crate::proc::ProcGlue;
use crate::sysctl::{HOSTNAMELEN, MAXHOSTNAMELEN, with_hostname};

/// The longest chain of continuation areas a walk follows (the user's decision D-N5.8-U1:
/// 64 areas of a logical block each for one record is beyond any writer; makefs, mkhybrid
/// and xtask's writer emit one).
pub const ISO_RRIP_MAXCE: usize = 64;

/// `ISO_SUSP_HEADER`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoSuspHeader {
    /// `type`.
    pub type_: [u8; isodcl(0, 1)],
    /// `length`: 711.
    pub length: [u8; isodcl(2, 2)],
    /// `version`.
    pub version: [u8; isodcl(3, 3)],
}

hal::abi_pod!(IsoSuspHeader {
    type_: [u8; 2],
    length: [u8; 1],
    version: [u8; 1],
});
pod_view!(IsoSuspHeader);

/// `ISO_RRIP_ATTR`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripAttr {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `mode`: 733.
    pub mode: [u8; isodcl(4, 11)],
    /// `links`: 733.
    pub links: [u8; isodcl(12, 19)],
    /// `uid`: 733.
    pub uid: [u8; isodcl(20, 27)],
    /// `gid`: 733.
    pub gid: [u8; isodcl(28, 35)],
}

hal::abi_pod!(IsoRripAttr {
    h: IsoSuspHeader,
    mode: [u8; 8],
    links: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
});
pod_view!(IsoRripAttr);

/// `ISO_RRIP_DEVICE`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripDevice {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `dev_t_high`: 733.
    pub dev_t_high: [u8; isodcl(4, 11)],
    /// `dev_t_low`: 733.
    pub dev_t_low: [u8; isodcl(12, 19)],
}

hal::abi_pod!(IsoRripDevice {
    h: IsoSuspHeader,
    dev_t_high: [u8; 8],
    dev_t_low: [u8; 8],
});
pod_view!(IsoRripDevice);

/// `ISO_SUSP_CFLAG_CONTINUE`.
pub const ISO_SUSP_CFLAG_CONTINUE: u8 = 0x01;
/// `ISO_SUSP_CFLAG_CURRENT`.
pub const ISO_SUSP_CFLAG_CURRENT: u8 = 0x02;
/// `ISO_SUSP_CFLAG_PARENT`.
pub const ISO_SUSP_CFLAG_PARENT: u8 = 0x04;
/// `ISO_SUSP_CFLAG_ROOT`.
pub const ISO_SUSP_CFLAG_ROOT: u8 = 0x08;
/// `ISO_SUSP_CFLAG_VOLROOT`.
pub const ISO_SUSP_CFLAG_VOLROOT: u8 = 0x10;
/// `ISO_SUSP_CFLAG_HOST`.
pub const ISO_SUSP_CFLAG_HOST: u8 = 0x20;

/// `ISO_RRIP_SLINK_COMPONENT`: one component of a symbolic link (its name follows).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripSlinkComponent {
    /// `cflag`.
    pub cflag: [u8; isodcl(1, 1)],
    /// `clen`.
    pub clen: [u8; isodcl(2, 2)],
    /// `name`: XXX.
    pub name: [u8; 1],
}

hal::abi_pod!(IsoRripSlinkComponent {
    cflag: [u8; 1],
    clen: [u8; 1],
    name: [u8; 1],
});

/// `ISO_RRIP_SLSIZ`: the header of a component.
pub const ISO_RRIP_SLSIZ: usize = 2;

/// `ISO_RRIP_SLINK`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripSlink {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `flags`.
    pub flags: [u8; isodcl(4, 4)],
    /// `component`.
    pub component: [u8; isodcl(5, 5)],
}

hal::abi_pod!(IsoRripSlink {
    h: IsoSuspHeader,
    flags: [u8; 1],
    component: [u8; 1],
});
pod_view!(IsoRripSlink);

/// `ISO_RRIP_ALTNAME`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripAltname {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `flags`.
    pub flags: [u8; isodcl(4, 4)],
}

hal::abi_pod!(IsoRripAltname {
    h: IsoSuspHeader,
    flags: [u8; 1],
});
pod_view!(IsoRripAltname);

/// `ISO_RRIP_CLINK`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripClink {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `dir_loc`: 733.
    pub dir_loc: [u8; isodcl(4, 11)],
}

hal::abi_pod!(IsoRripClink {
    h: IsoSuspHeader,
    dir_loc: [u8; 8],
});
pod_view!(IsoRripClink);

/// `ISO_RRIP_PLINK`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripPlink {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `dir_loc`: 733.
    pub dir_loc: [u8; isodcl(4, 11)],
}

hal::abi_pod!(IsoRripPlink {
    h: IsoSuspHeader,
    dir_loc: [u8; 8],
});
pod_view!(IsoRripPlink);

/// `ISO_RRIP_RELDIR`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripReldir {
    /// `h`.
    pub h: IsoSuspHeader,
}

hal::abi_pod!(IsoRripReldir { h: IsoSuspHeader });
pod_view!(IsoRripReldir);

/// `ISO_SUSP_TSTAMP_FORM17`.
pub const ISO_SUSP_TSTAMP_FORM17: u8 = 0x80;
/// `ISO_SUSP_TSTAMP_FORM7`.
pub const ISO_SUSP_TSTAMP_FORM7: u8 = 0x00;
/// `ISO_SUSP_TSTAMP_CREAT`.
pub const ISO_SUSP_TSTAMP_CREAT: u8 = 0x01;
/// `ISO_SUSP_TSTAMP_MODIFY`.
pub const ISO_SUSP_TSTAMP_MODIFY: u8 = 0x02;
/// `ISO_SUSP_TSTAMP_ACCESS`.
pub const ISO_SUSP_TSTAMP_ACCESS: u8 = 0x04;
/// `ISO_SUSP_TSTAMP_ATTR`.
pub const ISO_SUSP_TSTAMP_ATTR: u8 = 0x08;
/// `ISO_SUSP_TSTAMP_BACKUP`.
pub const ISO_SUSP_TSTAMP_BACKUP: u8 = 0x10;
/// `ISO_SUSP_TSTAMP_EXPIRE`.
pub const ISO_SUSP_TSTAMP_EXPIRE: u8 = 0x20;
/// `ISO_SUSP_TSTAMP_EFFECT`.
pub const ISO_SUSP_TSTAMP_EFFECT: u8 = 0x40;

/// `ISO_RRIP_TSTAMP`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripTstamp {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `flags`.
    pub flags: [u8; isodcl(4, 4)],
    /// `time`.
    pub time: [u8; isodcl(5, 5)],
}

hal::abi_pod!(IsoRripTstamp {
    h: IsoSuspHeader,
    flags: [u8; 1],
    time: [u8; 1],
});
pod_view!(IsoRripTstamp);

/// `ISO_RRIP_IDFLAG`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripIdflag {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `flags`.
    pub flags: [u8; isodcl(4, 4)],
}

hal::abi_pod!(IsoRripIdflag {
    h: IsoSuspHeader,
    flags: [u8; 1],
});
pod_view!(IsoRripIdflag);

/// `ISO_RRIP_EXTREF`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripExtref {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `len_id`.
    pub len_id: [u8; isodcl(4, 4)],
    /// `len_des`.
    pub len_des: [u8; isodcl(5, 5)],
    /// `len_src`.
    pub len_src: [u8; isodcl(6, 6)],
    /// `version`.
    pub version: [u8; isodcl(7, 7)],
}

hal::abi_pod!(IsoRripExtref {
    h: IsoSuspHeader,
    len_id: [u8; 1],
    len_des: [u8; 1],
    len_src: [u8; 1],
    version: [u8; 1],
});
pod_view!(IsoRripExtref);

/// `ISO_RRIP_OFFSET`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripOffset {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `check`.
    pub check: [u8; isodcl(4, 5)],
    /// `skip`.
    pub skip: [u8; isodcl(6, 6)],
}

hal::abi_pod!(IsoRripOffset {
    h: IsoSuspHeader,
    check: [u8; 2],
    skip: [u8; 1],
});
pod_view!(IsoRripOffset);

/// `ISO_RRIP_CONT`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IsoRripCont {
    /// `h`.
    pub h: IsoSuspHeader,
    /// `location`.
    pub location: [u8; isodcl(4, 11)],
    /// `offset`.
    pub offset: [u8; isodcl(12, 19)],
    /// `length`.
    pub length: [u8; isodcl(20, 27)],
}

hal::abi_pod!(IsoRripCont {
    h: IsoSuspHeader,
    location: [u8; 8],
    offset: [u8; 8],
    length: [u8; 8],
});
pod_view!(IsoRripCont);

/// The reader of one entry type: the entry's bytes and the analysis; returns the
/// `ISO_SUSP_*` bit it found, or 0.
type RripFunc<G> = fn(&[u8], &mut IsoRripAnalyze<'_, G>) -> i32;
/// The default of one entry type, run over the record when no entry of the type was found.
type RripFunc2<G> = fn(&IsoDirectoryRecord<'_>, &mut IsoRripAnalyze<'_, G>);

/// `RRIP_TABLE`: one entry type an analysis looks for.
struct RripTable<G: ProcGlue> {
    /// `type`.
    type_: [u8; 2],
    /// `func`.
    func: RripFunc<G>,
    /// `func2`.
    func2: Option<RripFunc2<G>>,
    /// `result`.
    result: i32,
}

/// The analyses' tables (the C's `static RRIP_TABLE`s), for the glue `G`.
struct RripTables<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> RripTables<G> {
    /// `rrip_table_analyze`: get attributes.
    const RRIP_TABLE_ANALYZE: [RripTable<G>; 6] = [
        RripTable {
            type_: *b"PX",
            func: cd9660_rrip_attr::<G>,
            func2: Some(cd9660_rrip_defattr::<G>),
            result: ISO_SUSP_ATTR,
        },
        RripTable {
            type_: *b"TF",
            func: cd9660_rrip_tstamp::<G>,
            func2: Some(cd9660_rrip_deftstamp::<G>),
            result: ISO_SUSP_TSTAMP,
        },
        RripTable {
            type_: *b"PN",
            func: cd9660_rrip_device::<G>,
            func2: None,
            result: ISO_SUSP_DEVICE,
        },
        RripTable {
            type_: *b"RR",
            func: cd9660_rrip_idflag::<G>,
            func2: None,
            result: ISO_SUSP_IDFLAG,
        },
        RripTable {
            type_: *b"CE",
            func: cd9660_rrip_cont::<G>,
            func2: None,
            result: ISO_SUSP_CONT,
        },
        RripTable {
            type_: *b"ST",
            func: cd9660_rrip_stop::<G>,
            func2: None,
            result: ISO_SUSP_STOP,
        },
    ];

    /// `rrip_table_getname`: get alternate name.
    const RRIP_TABLE_GETNAME: [RripTable<G>; 7] = [
        RripTable {
            type_: *b"NM",
            func: cd9660_rrip_altname::<G>,
            func2: Some(cd9660_rrip_defname::<G>),
            result: ISO_SUSP_ALTNAME,
        },
        RripTable {
            type_: *b"CL",
            func: cd9660_rrip_pclink::<G>,
            func2: None,
            result: ISO_SUSP_CLINK | ISO_SUSP_PLINK,
        },
        RripTable {
            type_: *b"PL",
            func: cd9660_rrip_pclink::<G>,
            func2: None,
            result: ISO_SUSP_CLINK | ISO_SUSP_PLINK,
        },
        RripTable {
            type_: *b"RE",
            func: cd9660_rrip_reldir::<G>,
            func2: None,
            result: ISO_SUSP_RELDIR,
        },
        RripTable {
            type_: *b"RR",
            func: cd9660_rrip_idflag::<G>,
            func2: None,
            result: ISO_SUSP_IDFLAG,
        },
        RripTable {
            type_: *b"CE",
            func: cd9660_rrip_cont::<G>,
            func2: None,
            result: ISO_SUSP_CONT,
        },
        RripTable {
            type_: *b"ST",
            func: cd9660_rrip_stop::<G>,
            func2: None,
            result: ISO_SUSP_STOP,
        },
    ];

    /// `rrip_table_getsymname`: get symbolic link.
    const RRIP_TABLE_GETSYMNAME: [RripTable<G>; 4] = [
        RripTable {
            type_: *b"SL",
            func: cd9660_rrip_slink::<G>,
            func2: None,
            result: ISO_SUSP_SLINK,
        },
        RripTable {
            type_: *b"RR",
            func: cd9660_rrip_idflag::<G>,
            func2: None,
            result: ISO_SUSP_IDFLAG,
        },
        RripTable {
            type_: *b"CE",
            func: cd9660_rrip_cont::<G>,
            func2: None,
            result: ISO_SUSP_CONT,
        },
        RripTable {
            type_: *b"ST",
            func: cd9660_rrip_stop::<G>,
            func2: None,
            result: ISO_SUSP_STOP,
        },
    ];

    /// `rrip_table_extref`: the extension reference.
    const RRIP_TABLE_EXTREF: [RripTable<G>; 3] = [
        RripTable {
            type_: *b"ER",
            func: cd9660_rrip_extref::<G>,
            func2: None,
            result: ISO_SUSP_EXTREF,
        },
        RripTable {
            type_: *b"CE",
            func: cd9660_rrip_cont::<G>,
            func2: None,
            result: ISO_SUSP_CONT,
        },
        RripTable {
            type_: *b"ST",
            func: cd9660_rrip_stop::<G>,
            func2: None,
            result: ISO_SUSP_STOP,
        },
    ];
}

/// `ana->inop`, which the attribute analysis sets.
fn inop<'a, G: ProcGlue>(ana: &IsoRripAnalyze<'a, G>) -> &'a IsoNode<G> {
    match ana.inop {
        Some(ip) => ip,
        None => kpanic(format_args!("cd9660_rrip: analysis without a node")),
    }
}

/// `ana->outbuf -= *ana->outlen; *ana->outlen = 0`: drop the name gathered so far.
fn reset_out<G: ProcGlue>(ana: &mut IsoRripAnalyze<'_, G>) {
    ana.outpos = ana.outpos.saturating_sub(usize::from(ana.outlen));
    ana.outlen = 0;
}

/// `hostnamelen`, at most the size of `hostname`.
fn hostnamelen() -> usize {
    HOSTNAMELEN
        .load(Ordering::Relaxed)
        .clamp(0, MAXHOSTNAMELEN as i32) as usize
}

/// Where the bytes of one piece of a gathered name come from.
enum Src<'v> {
    /// Bytes of the entry (a component) or a constant (`.`, `..`).
    Bytes(&'v [u8]),
    /// `hostname`, read when the piece is copied.
    Host,
    /// `f_mntonname` of the mount, copied out of it.
    Mnt([u8; MNAMELEN]),
}

impl Src<'_> {
    /// Copies the piece's first `dst.len()` bytes into `dst`.
    fn copy_to(&self, dst: &mut [u8]) {
        let n = dst.len();
        match self {
            Src::Bytes(b) => dst.copy_from_slice(&b[..n]),
            Src::Host => with_hostname(|h| dst.copy_from_slice(&h[..n])),
            Src::Mnt(m) => dst.copy_from_slice(&m[..n]),
        }
    }
}

/// `cd9660_rrip_attr`: POSIX file attribute.
fn cd9660_rrip_attr<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let Some(p) = IsoRripAttr::at(v, 0) else {
        return 0;
    };

    // The C's `int` values stored into the members' narrower types.
    ana.inode.iso_mode = isonum_733(&p.mode) as u16;
    ana.inode.iso_uid = isonum_733(&p.uid);
    ana.inode.iso_gid = isonum_733(&p.gid);
    ana.inode.iso_links = isonum_733(&p.links) as i16;
    ana.fields &= !ISO_SUSP_ATTR;
    ISO_SUSP_ATTR
}

/// `cd9660_rrip_defattr`.
fn cd9660_rrip_defattr<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    ana: &mut IsoRripAnalyze<'_, G>,
) {
    // But this is a required field!
    printf(format_args!("RRIP without PX field?\n"));
    let ip = inop(ana);
    cd9660_defattr(isodir, ip, &mut ana.inode, None);
}

/// `cd9660_rrip_slink`: symbolic links.
fn cd9660_rrip_slink<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    /// The C's `goto invalid`.
    fn invalid<G: ProcGlue>(ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
        ana.cont = 1;
        ana.fields = 0;
        reset_out(ana);
        0
    }

    let Some(p) = IsoRripSlink::at(v, 0) else {
        return invalid(ana);
    };
    let pcompe = v.len();
    let mut pcomp = core::mem::offset_of!(IsoRripSlink, component);
    let mut len = usize::from(ana.outlen);
    let mut outpos = ana.outpos;
    let mut cont = ana.cont;
    let maxlen = usize::from(ana.maxlen);

    // Gathering a Symbolic name from each component with path
    while pcomp < pcompe {
        if pcomp + ISO_RRIP_SLSIZ > pcompe {
            return invalid(ana);
        }
        let cflag = v[pcomp];
        let clen = usize::from(v[pcomp + 1]);
        if pcomp + ISO_RRIP_SLSIZ + clen > pcompe {
            return invalid(ana);
        }

        if cont == 0 && len < maxlen {
            let Some(b) = ana.outbuf.get_mut(outpos) else {
                return invalid(ana);
            };
            *b = b'/';
            outpos += 1;
            len += 1;
        }
        cont = 0;

        let (src, wlen) = match cflag {
            ISO_SUSP_CFLAG_CURRENT => (Src::Bytes(b".."), 1), // Inserting Current
            ISO_SUSP_CFLAG_PARENT => (Src::Bytes(b".."), 2),  // Inserting Parent
            ISO_SUSP_CFLAG_ROOT => {
                // Inserting slash for ROOT; start over from beginning(?)
                outpos = outpos.saturating_sub(len);
                len = 0;
                (Src::Bytes(b""), 0)
            }
            ISO_SUSP_CFLAG_VOLROOT => {
                // Inserting a mount point i.e. "/cdrom"; same as above
                outpos = outpos.saturating_sub(len);
                len = 0;
                let (m, n) = ana.imp.im_mountp.mntonname();
                (Src::Mnt(m), n)
            }
            ISO_SUSP_CFLAG_HOST => (Src::Host, hostnamelen()), // Inserting hostname
            ISO_SUSP_CFLAG_CONTINUE | 0 => {
                if cflag == ISO_SUSP_CFLAG_CONTINUE {
                    cont = 1;
                }
                // Inserting component
                let at = pcomp + ISO_RRIP_SLSIZ;
                (Src::Bytes(&v[at..at + clen]), clen)
            }
            _ => {
                printf(format_args!("RRIP with incorrect flags?"));
                (Src::Bytes(b""), maxlen + 1)
            }
        };

        if len + wlen > maxlen {
            // indicate error to caller
            return invalid(ana);
        }
        let Some(dst) = ana.outbuf.get_mut(outpos..outpos + wlen) else {
            return invalid(ana);
        };
        src.copy_to(dst);
        outpos += wlen;
        len += wlen;

        pcomp += ISO_RRIP_SLSIZ + clen;
    }
    ana.outpos = outpos;
    // `len` is at most `maxlen`, a `u_short`.
    ana.outlen = len as u16;
    ana.cont = cont;

    if isonum_711(&p.flags) == 0 {
        ana.fields &= !ISO_SUSP_SLINK;
        return ISO_SUSP_SLINK;
    }
    0
}

/// `cd9660_rrip_altname`: alternate name.
fn cd9660_rrip_altname<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    /// The C's `goto invalid`: treat as no name field.
    fn invalid<G: ProcGlue>(ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
        ana.fields &= !ISO_SUSP_ALTNAME;
        reset_out(ana);
        0
    }

    let len = v.len();
    let Some(p) = IsoRripAltname::at(v, 0) else {
        return invalid(ana);
    };
    let mut cont = 0;

    let (src, wlen) = match p.flags[0] {
        ISO_SUSP_CFLAG_CURRENT => (Src::Bytes(b".."), 1), // Inserting Current
        ISO_SUSP_CFLAG_PARENT => (Src::Bytes(b".."), 2),  // Inserting Parent
        ISO_SUSP_CFLAG_HOST => (Src::Host, hostnamelen()), // Inserting hostname
        f @ (ISO_SUSP_CFLAG_CONTINUE | 0) => {
            if f == ISO_SUSP_CFLAG_CONTINUE {
                cont = 1;
            }
            // Inserting component
            (
                Src::Bytes(&v[IsoRripAltname::SIZE..]),
                len - IsoRripAltname::SIZE,
            )
        }
        _ => {
            printf(format_args!("RRIP with incorrect NM flags?\n"));
            (Src::Bytes(b""), usize::from(ana.maxlen) + 1)
        }
    };

    let outlen = usize::from(ana.outlen);
    let maxlen = usize::from(ana.maxlen);
    if outlen > maxlen || wlen > maxlen - outlen {
        return invalid(ana);
    }
    let Some(dst) = ana.outbuf.get_mut(ana.outpos..ana.outpos + wlen) else {
        return invalid(ana);
    };
    src.copy_to(dst);
    // The sum is at most `maxlen`, a `u_short`.
    ana.outlen += wlen as u16;
    ana.outpos += wlen;

    if cont == 0 {
        ana.fields &= !ISO_SUSP_ALTNAME;
        return ISO_SUSP_ALTNAME;
    }
    0
}

/// `cd9660_rrip_defname`: the ISO 9660 name, when there is no `NM` entry.
fn cd9660_rrip_defname<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    ana: &mut IsoRripAnalyze<'_, G>,
) {
    // strlcpy(ana->outbuf, "..", ana->maxlen - *ana->outlen)
    let size = i32::from(ana.maxlen) - i32::from(ana.outlen);
    if size > 0 {
        let n = (size - 1).min(2) as usize;
        let out = ana.outbuf.get_mut(ana.outpos..).unwrap_or(&mut []);
        for (i, &c) in b".."[..n].iter().chain(b"\0").enumerate() {
            if let Some(b) = out.get_mut(i) {
                *b = c;
            }
        }
    }
    match isodir.name0() {
        0 => ana.outlen = 1,
        1 => ana.outlen = 2,
        _ => {
            let name = isodir.name();
            let namelen = usize::from(isonum_711(isodir.name_len())).min(name.len());
            let out = ana.outbuf.get_mut(ana.outpos..).unwrap_or(&mut []);
            isofntrans(
                &name[..namelen],
                out,
                &mut ana.outlen,
                true,
                isonum_711(isodir.flags()) & 4 != 0,
                ana.imp.joliet_level,
            );
        }
    }
}

/// `cd9660_rrip_pclink`: parent or child link.
fn cd9660_rrip_pclink<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let Some(p) = IsoRripClink::at(v, 0) else {
        return 0;
    };
    // The C's `int` shifted into the `cdino_t`: the same bits as this `u32` shift.
    let ino: Cdino = isonum_733(&p.dir_loc) << ana.imp.im_bshift;
    if let Some(inump) = ana.inump.as_deref_mut() {
        *inump = ino;
    }
    ana.fields &= !(ISO_SUSP_CLINK | ISO_SUSP_PLINK);
    if p.h.type_[0] == b'C' {
        ISO_SUSP_CLINK
    } else {
        ISO_SUSP_PLINK
    }
}

/// `cd9660_rrip_reldir`: relocated directory.
fn cd9660_rrip_reldir<G: ProcGlue>(_v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    // special hack to make caller aware of RE field
    ana.outlen = 0;
    ana.fields = 0;
    ISO_SUSP_RELDIR | ISO_SUSP_ALTNAME | ISO_SUSP_CLINK | ISO_SUSP_PLINK
}

/// `cd9660_rrip_tstamp`: time stamps. As in C, `need` counts the whole `ISO_RRIP_TSTAMP`
/// (one byte of `time` included) before the stamps, so a `TF` entry must be a byte longer
/// than its stamps to be read.
fn cd9660_rrip_tstamp<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let len = v.len();
    let Some(p) = IsoRripTstamp::at(v, 0) else {
        return 0;
    };
    let flags = p.flags[0];
    let tlen = if flags & ISO_SUSP_TSTAMP_FORM17 != 0 {
        17
    } else {
        7
    };
    let mut need = IsoRripTstamp::SIZE;
    for f in [
        ISO_SUSP_TSTAMP_CREAT,
        ISO_SUSP_TSTAMP_MODIFY,
        ISO_SUSP_TSTAMP_ACCESS,
        ISO_SUSP_TSTAMP_ATTR,
    ] {
        if flags & f != 0 {
            need += tlen;
        }
    }
    if need > len {
        return 0;
    }
    let mut ptime = core::mem::offset_of!(IsoRripTstamp, time);
    let conv: fn(&[u8], &mut Timespec) -> bool = if flags & ISO_SUSP_TSTAMP_FORM17 == 0 {
        cd9660_tstamp_conv7
    } else {
        cd9660_tstamp_conv17
    };

    // Check a format of time stamp (7bytes/17bytes)
    let ino = &mut ana.inode;
    if flags & ISO_SUSP_TSTAMP_CREAT != 0 {
        ptime += tlen;
    }

    if flags & ISO_SUSP_TSTAMP_MODIFY != 0 {
        conv(&v[ptime..], &mut ino.iso_mtime);
        ptime += tlen;
    } else {
        ino.iso_mtime = Timespec::new(0, 0);
    }

    if flags & ISO_SUSP_TSTAMP_ACCESS != 0 {
        conv(&v[ptime..], &mut ino.iso_atime);
        ptime += tlen;
    } else {
        ino.iso_atime = ino.iso_mtime;
    }

    if flags & ISO_SUSP_TSTAMP_ATTR != 0 {
        conv(&v[ptime..], &mut ino.iso_ctime);
    } else {
        ino.iso_ctime = ino.iso_mtime;
    }
    ana.fields &= !ISO_SUSP_TSTAMP;
    ISO_SUSP_TSTAMP
}

/// `cd9660_rrip_deftstamp`.
fn cd9660_rrip_deftstamp<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    ana: &mut IsoRripAnalyze<'_, G>,
) {
    let ip = inop(ana);
    cd9660_deftstamp(isodir, ip, &mut ana.inode, None);
}

/// `cd9660_rrip_device`: POSIX device modes.
fn cd9660_rrip_device<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let Some(p) = IsoRripDevice::at(v, 0) else {
        return 0;
    };
    let high = isonum_733(&p.dev_t_high);
    // The C's `int` into the `dev_t`: the same bits.
    let low = isonum_733(&p.dev_t_low) as Dev;

    ana.inode.iso_rdev = if high == 0 {
        makedev(major(low), minor(low))
    } else {
        makedev(high, minor(low))
    };
    ana.fields &= !ISO_SUSP_DEVICE;
    ISO_SUSP_DEVICE
}

/// `cd9660_rrip_idflag`: flag indicating which fields are recorded.
fn cd9660_rrip_idflag<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let Some(p) = IsoRripIdflag::at(v, 0) else {
        return 0;
    };
    // don't touch high bits
    ana.fields &= i32::from(isonum_711(&p.flags)) | !0xff;
    // special handling of RE field
    if ana.fields & ISO_SUSP_RELDIR != 0 {
        return cd9660_rrip_reldir(v, ana);
    }

    ISO_SUSP_IDFLAG
}

/// `cd9660_rrip_cont`: continuation pointer.
fn cd9660_rrip_cont<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let Some(p) = IsoRripCont::at(v, 0) else {
        return 0;
    };
    ana.iso_ce_blk = Daddr::from(isonum_733(&p.location));
    ana.iso_ce_off = Off::from(isonum_733(&p.offset));
    // The C's `int`, which a length past 2^31 makes negative (the walk refuses it).
    ana.iso_ce_len = isonum_733(&p.length) as i32;
    ISO_SUSP_CONT
}

/// `cd9660_rrip_stop`: System Use end.
fn cd9660_rrip_stop<G: ProcGlue>(_v: &[u8], _ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    ISO_SUSP_STOP
}

/// `cd9660_rrip_extref`: extension reference.
fn cd9660_rrip_extref<G: ProcGlue>(v: &[u8], ana: &mut IsoRripAnalyze<'_, G>) -> i32 {
    let len = v.len();
    let Some(p) = IsoRripExtref::at(v, 0) else {
        return 0;
    };
    if isonum_711(&p.version) != 1 {
        return 0;
    }
    let idlen = usize::from(isonum_711(&p.len_id));
    if idlen != 9 && idlen != 10 {
        return 0;
    }
    if len < IsoRripExtref::SIZE + idlen {
        return 0;
    }
    let id = &v[8..8 + idlen];
    if idlen == 9 && id != b"IEEE_1282" {
        return 0;
    }
    if idlen == 10 && id != b"IEEE_P1282" && id != b"RRIP_1991A" {
        return 0;
    }
    ana.fields &= !ISO_SUSP_EXTREF;
    ISO_SUSP_EXTREF
}

/// The inner loop of `cd9660_rrip_loop`: the SUSP entries of `area` from byte `phead`, each
/// one `table` knows given to its reader while `ana.fields` still wants one, until a `ST`
/// entry, a malformed length or the area's end.
fn cd9660_rrip_area<G: ProcGlue>(
    area: &[u8],
    mut phead: usize,
    ana: &mut IsoRripAnalyze<'_, G>,
    table: &[RripTable<G>],
    result: &mut i32,
) {
    // Note: "pend" should be more than one SUSP header
    while phead + IsoSuspHeader::SIZE <= area.len() {
        let len = usize::from(area[phead + 2]);
        if len < IsoSuspHeader::SIZE || phead + len > area.len() {
            break;
        }
        let entry = &area[phead..phead + len];
        if entry[3] == 1 {
            if let Some(t) = table.iter().find(|t| entry[..2] == t.type_) {
                *result |= (t.func)(entry, ana);
            }
            if ana.fields == 0 {
                break;
            }
        }
        if *result & ISO_SUSP_STOP != 0 {
            *result &= !ISO_SUSP_STOP;
            break;
        }
        // move to next SUSP; hopefully this works with newer versions, too
        phead += len;
    }
}

/// `cd9660_rrip_loop`: walk the SUSP entries of `isodir` (and of its continuation areas),
/// giving each entry `table` knows to its reader while `ana.fields` still wants one; then
/// run the defaults of the types not found. Returns the `ISO_SUSP_*` bits found.
fn cd9660_rrip_loop<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    ana: &mut IsoRripAnalyze<'_, G>,
    table: &[RripTable<G>],
) -> i32 {
    let imp = ana.imp;
    let rec = isodir.bytes();

    // Note: If name length is odd, it will be padded by 1 byte after the name
    let name_len = usize::from(isonum_711(isodir.name_len()));
    let mut pwhead = ISO_DIRECTORY_RECORD_SIZE + name_len;
    if name_len & 1 == 0 {
        pwhead += 1;
    }
    let mut c = 0u8;
    isochar(
        isodir.name(),
        pwhead - ISO_DIRECTORY_RECORD_SIZE,
        imp.joliet_level,
        &mut c,
    );

    // If it's not the '.' entry of the root dir obey SP field
    let skip = if c != 0 || isonum_733(isodir.extent()) != imp.root_extent {
        imp.rr_skip
    } else {
        imp.rr_skip0
    };
    let phead = pwhead.saturating_add_signed(skip as isize);
    let area = &rec[..usize::from(isonum_711(isodir.length())).min(rec.len())];

    let mut result = 0;
    ana.iso_ce_len = 0;
    cd9660_rrip_area(area, phead, ana, table, &mut result);

    // The continuation area being walked: its buffer and its device block.
    let mut cont: Option<(BufRef, Daddr)> = None;
    let mut areas = 0;
    while ana.fields != 0 && ana.iso_ce_len != 0 {
        let ce_len = ana.iso_ce_len;
        let ce_off = ana.iso_ce_off;
        if ana.iso_ce_blk >= i64::from(imp.volume_space_size)
            || ce_len < 0
            || ce_off + i64::from(ce_len) > i64::from(imp.logical_block_size)
        {
            // what to do now?
            break;
        }
        let blk = ana.iso_ce_blk << (imp.im_bshift - DEV_BSHIFT as i32);
        // D-N5.8-U1: a block the walk holds busy, or one area too many, ends it.
        if ana.held == Some(blk)
            || cont.as_ref().is_some_and(|(_, b)| *b == blk)
            || areas == ISO_RRIP_MAXCE
        {
            break;
        }
        areas += 1;
        if let Some((t, _)) = cont.take() {
            brelse(t);
        }
        let (t, error) = bread(&imp.im_devvp.bufs, blk, imp.logical_block_size);
        if error.is_err() {
            brelse(t);
            break;
        }
        // Within the block by the check above (`ce_off + ce_len <= logical_block_size`, the
        // buffer's size).
        ana.iso_ce_len = 0;
        if let Some(area) = t
            .bytes()
            .get(ce_off as usize..(ce_off + i64::from(ce_len)) as usize)
        {
            cd9660_rrip_area(area, 0, ana, table, &mut result);
        }
        cont = Some((t, blk));
    }
    if let Some((t, _)) = cont {
        brelse(t);
    }

    // If we don't find the Basic SUSP stuffs, just set default value (attribute/time
    // stamp)
    for t in table {
        let Some(func2) = t.func2 else {
            break;
        };
        if t.result & result == 0 {
            func2(isodir, ana);
        }
    }

    result
}

/// `cd9660_rrip_analyze`: get attributes (POSIX attributes, time stamps, device number)
/// of the node `inop` from its record; `held` is the device block the caller holds busy, if
/// it holds one (D-N5.8-U1).
pub fn cd9660_rrip_analyze<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    inop: &IsoNode<G>,
    imp: &IsoMnt<G>,
    held: Option<Daddr>,
) -> i32 {
    let mut analyze = IsoRripAnalyze::new(imp, ISO_SUSP_ATTR | ISO_SUSP_TSTAMP | ISO_SUSP_DEVICE);
    analyze.inop = Some(inop);
    analyze.inode = inop.inode();
    analyze.held = held;

    let result = cd9660_rrip_loop(isodir, &mut analyze, &RripTables::<G>::RRIP_TABLE_ANALYZE);
    inop.set_inode(&analyze.inode);
    result
}

/// `cd9660_rrip_getname`: get alternate name into `outbuf` (`NAME_MAX` bytes), its length
/// into `outlen`; a relocated directory's location goes to `inump`.
pub fn cd9660_rrip_getname<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    outbuf: &mut [u8],
    outlen: &mut u16,
    inump: &mut Cdino,
    imp: &IsoMnt<G>,
    held: Option<Daddr>,
) -> i32 {
    let mut analyze = IsoRripAnalyze::new(
        imp,
        ISO_SUSP_ALTNAME | ISO_SUSP_RELDIR | ISO_SUSP_CLINK | ISO_SUSP_PLINK,
    );
    analyze.outbuf = outbuf;
    analyze.maxlen = NAME_MAX as u16;
    analyze.inump = Some(inump);
    analyze.held = held;

    let namelen = usize::from(isonum_711(isodir.name_len()));
    let mut c = 0u8;
    isochar(isodir.name(), namelen, imp.joliet_level, &mut c);
    let mut tab = &RripTables::<G>::RRIP_TABLE_GETNAME[..];
    if c == 0 || c == 1 {
        cd9660_rrip_defname(isodir, &mut analyze);

        analyze.fields &= !ISO_SUSP_ALTNAME;
        tab = &tab[1..];
    }

    let result = cd9660_rrip_loop(isodir, &mut analyze, tab);
    *outlen = analyze.outlen;
    result
}

/// `cd9660_rrip_getsymname`: get symbolic link into `outbuf` (`MAXPATHLEN` bytes), its
/// length into `outlen`; non-zero when a whole link was found.
pub fn cd9660_rrip_getsymname<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    outbuf: &mut [u8],
    outlen: &mut u16,
    imp: &IsoMnt<G>,
    held: Option<Daddr>,
) -> i32 {
    let mut analyze = IsoRripAnalyze::new(imp, ISO_SUSP_SLINK);
    analyze.outbuf = outbuf;
    analyze.maxlen = MAXPATHLEN as u16;
    analyze.cont = 1; // don't start with a slash
    analyze.held = held;

    let result = cd9660_rrip_loop(
        isodir,
        &mut analyze,
        &RripTables::<G>::RRIP_TABLE_GETSYMNAME,
    ) & ISO_SUSP_SLINK;
    *outlen = analyze.outlen;
    result
}

/// `cd9660_rrip_offset`: check for Rock Ridge Extension and return offset of its fields
/// (the `SP` entry's skip), -1 when there is none. Note: We insist on the ER field.
pub fn cd9660_rrip_offset<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    imp: &mut IsoMnt<G>,
    held: Option<Daddr>,
) -> i32 {
    const SP: &[u8; 6] = b"SP\x07\x01\xbe\xef";

    imp.rr_skip0 = 0;
    let rec = isodir.bytes();
    let pend = usize::from(isonum_711(isodir.length())).min(rec.len());
    let mut p = ISO_DIRECTORY_RECORD_SIZE + 1;
    if p + IsoRripOffset::SIZE > pend {
        return -1;
    }
    if rec[p..p + 6] != *SP {
        // Maybe, it's a CDROM XA disc?
        imp.rr_skip0 = 15;
        p += 15;
        if p + IsoRripOffset::SIZE > pend {
            return -1;
        }
        if rec[p..p + 6] != *SP {
            return -1;
        }
    }

    let mut analyze = IsoRripAnalyze::new(imp, ISO_SUSP_EXTREF);
    analyze.held = held;
    if cd9660_rrip_loop(isodir, &mut analyze, &RripTables::<G>::RRIP_TABLE_EXTREF) & ISO_SUSP_EXTREF
        == 0
    {
        return -1;
    }

    i32::from(rec[p + core::mem::offset_of!(IsoRripOffset, skip)])
}

const _: () = {
    assert!(size_of::<IsoSuspHeader>() == 4);
    assert!(size_of::<IsoRripAttr>() == 36);
    assert!(size_of::<IsoRripDevice>() == 20);
    assert!(size_of::<IsoRripSlinkComponent>() == 3);
    assert!(size_of::<IsoRripSlink>() == 6);
    assert!(size_of::<IsoRripAltname>() == 5);
    assert!(size_of::<IsoRripClink>() == 12);
    assert!(size_of::<IsoRripPlink>() == 12);
    assert!(size_of::<IsoRripReldir>() == 4);
    assert!(size_of::<IsoRripTstamp>() == 6);
    assert!(size_of::<IsoRripIdflag>() == 5);
    assert!(size_of::<IsoRripExtref>() == 8);
    assert!(size_of::<IsoRripOffset>() == 7);
    assert!(size_of::<IsoRripCont>() == 28);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the Rock Ridge walk: the extension check of the root, alternate names,
    // symbolic links, attributes, time stamps, device numbers, relocation entries, the SUSP
    // control entries (`RR`, `CE`, `ST`) and the bounds of the `CE` chain (D-N5.8-U1), over
    // the records of a makefs image, records built here and continuation areas on testffs's
    // disk.

    use std::vec::Vec;

    use super::*;
    use crate::fs::bio::BCSTATS;
    use crate::fs::isofs::cd9660::cd9660_extern::{ISO_FTYPE_RRIP, IsoMntRef};
    use crate::fs::isofs::cd9660::testcd9660::{attach_image, image, rec, test_mnt, test_node};
    use crate::fs::testfs::TestGlue;
    use hal::sys::stat::{S_IFCHR, S_IFDIR, S_IFLNK, S_IFREG};

    /// A 733 field: the value little-endian, then big-endian.
    fn both32(v: u32) -> [u8; 8] {
        let mut b = [0u8; 8];
        b[..4].copy_from_slice(&v.to_le_bytes());
        b[4..].copy_from_slice(&v.to_be_bytes());
        b
    }

    /// A SUSP entry of version 1.
    fn susp(t: &[u8; 2], body: &[u8]) -> Vec<u8> {
        let mut e = std::vec![t[0], t[1], (4 + body.len()) as u8, 1];
        e.extend_from_slice(body);
        e
    }

    /// A `PX` entry (RRIP 1.10: no file serial number).
    fn px(mode: u32, links: u32, uid: u32, gid: u32) -> Vec<u8> {
        let mut b = Vec::new();
        for v in [mode, links, uid, gid] {
            b.extend_from_slice(&both32(v));
        }
        susp(b"PX", &b)
    }

    /// A `CE` entry: the continuation area at byte `off` of logical block `blk`, `len` long.
    fn ce(blk: u32, off: u32, len: u32) -> Vec<u8> {
        susp(b"CE", &[both32(blk), both32(off), both32(len)].concat())
    }

    /// A directory record at extent 30 named `name`, with the system use entries `entries`.
    fn record(name: &[u8], dir: bool, entries: &[Vec<u8>]) -> Vec<u8> {
        let mut r = std::vec![0u8; ISO_DIRECTORY_RECORD_SIZE];
        r[2..10].copy_from_slice(&both32(30));
        r[18..25].copy_from_slice(&[126, 10, 4, 7, 36, 48, (-12i8) as u8]);
        r[25] = if dir { 2 } else { 0 };
        r[28..32].copy_from_slice(&[1, 0, 0, 1]);
        r[32] = name.len() as u8;
        r.extend_from_slice(name);
        if name.len() % 2 == 0 {
            r.push(0);
        }
        for e in entries {
            r.extend_from_slice(e);
        }
        r[0] = r.len() as u8;
        r
    }

    /// The makefs image's mount, Rock Ridge with no skip, changed by `f`.
    fn rrip_mnt(f: impl FnOnce(&mut IsoMnt<TestGlue>)) -> IsoMntRef<TestGlue> {
        test_mnt(ISO_FTYPE_RRIP, f)
    }

    /// `cd9660_rrip_getname`: the name, its length, the inode number and the result.
    fn getname(r: &[u8], imp: &IsoMnt<TestGlue>) -> (Vec<u8>, i32, Cdino) {
        let mut out = [0u8; NAME_MAX + 1];
        let mut len = 0u16;
        let mut ino: Cdino = 77;
        let res = cd9660_rrip_getname(&rec(r), &mut out, &mut len, &mut ino, imp, None);
        (out[..usize::from(len)].to_vec(), res, ino)
    }

    /// `cd9660_rrip_getsymname`: the link and the result.
    fn getsymname(r: &[u8], imp: &IsoMnt<TestGlue>) -> (Vec<u8>, i32) {
        let mut out = [0u8; MAXPATHLEN];
        let mut len = 0u16;
        let res = cd9660_rrip_getsymname(&rec(r), &mut out, &mut len, imp, None);
        (out[..usize::from(len)].to_vec(), res)
    }

    /// The busy and mapped buffers of the cache.
    fn busy() -> i64 {
        BCSTATS.busymapped.load(Ordering::Relaxed)
    }

    #[test]
    fn the_root_announces_rock_ridge() {
        let (_g, _p) = attach_image(image::image_bytes());
        let mut imp = test_mnt_owned();
        assert_eq!(
            cd9660_rrip_offset(&rec(&image::ROOT_DOT), &mut imp, None),
            0
        );
        assert_eq!(imp.rr_skip0, 0);

        // no SP entry: not Rock Ridge (after looking for the CD-ROM XA layout too)
        assert_eq!(
            cd9660_rrip_offset(&rec(&image::FILE_REC), &mut imp, None),
            -1
        );
        assert_eq!(imp.rr_skip0, 15);

        // CD-ROM XA: 15 bytes of XA data before the SP entry; the ER may say RRIP_1991A
        let mut sp_er = std::vec![0x58u8; 15];
        sp_er.extend_from_slice(b"SP\x07\x01\xbe\xef\x00");
        let mut er = std::vec![10, 0, 0, 1];
        er.extend_from_slice(b"RRIP_1991A");
        sp_er.extend_from_slice(&susp(b"ER", &er));
        let mut r = image::ROOT_DOT[..34].to_vec();
        r.extend_from_slice(&sp_er);
        r[0] = r.len() as u8;
        assert_eq!(cd9660_rrip_offset(&rec(&r), &mut imp, None), 0);
        assert_eq!(imp.rr_skip0, 15);

        // an extension reference to something else is not Rock Ridge
        let pos = r.len() - 10;
        r[pos..].copy_from_slice(b"SOMETHING!");
        assert_eq!(cd9660_rrip_offset(&rec(&r), &mut imp, None), -1);
    }

    /// The structure `test_mnt` builds, unpublished (`cd9660_rrip_offset` changes
    /// `rr_skip0` before `iso_mountfs` publishes it).
    fn test_mnt_owned() -> IsoMnt<TestGlue> {
        let imp = rrip_mnt(|_| {});
        IsoMnt {
            im_flags: imp.im_flags,
            im_mountp: imp.im_mountp,
            im_dev: imp.im_dev,
            im_devvp: imp.im_devvp,
            logical_block_size: imp.logical_block_size,
            im_bshift: imp.im_bshift,
            im_bmask: imp.im_bmask,
            volume_space_size: imp.volume_space_size,
            root: imp.root,
            root_extent: imp.root_extent,
            root_size: imp.root_size,
            iso_ftype: imp.iso_ftype,
            rr_skip: imp.rr_skip,
            rr_skip0: imp.rr_skip0,
            joliet_level: imp.joliet_level,
        }
    }

    #[test]
    fn names_come_from_nm_entries() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let (name, res, ino) = getname(&image::FILE_REC, &imp);
        assert_eq!(name, b"m10c-iso.txt");
        assert_ne!(res & ISO_SUSP_ALTNAME, 0);
        assert_eq!(ino, 77);
        assert_eq!(
            getname(&image::MIXED_REC, &imp).0,
            b"Mixed_Case-name.long.txt"
        );
        assert_eq!(getname(&image::SUB_REC, &imp).0, b"sub");
        assert_eq!(getname(&image::LINK_REC, &imp).0, b"link");
        // '.' and '..' are named by the record, not by NM
        assert_eq!(getname(&image::ROOT_DOTDOT, &imp).0, b"..");
        // the root's '.' continues in block 24 (read from the disc: PX only)
        assert_eq!(getname(&image::ROOT_DOT, &imp).0, b".");
        // a mount whose last block is 23 does not reach it: the walk stops
        let short = rrip_mnt(|m| m.volume_space_size = 24);
        assert_eq!(getname(&image::ROOT_DOT, &short).0, b".");
    }

    #[test]
    fn names_without_nm_are_the_iso_names() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let r = record(b"README.TXT;1", false, &[px(0o100444, 1, 0, 0)]);
        let (name, res, _) = getname(&r, &imp);
        assert_eq!(name, b"README.TXT;1");
        assert_eq!(res & ISO_SUSP_ALTNAME, 0);
    }

    #[test]
    fn nm_entries_continue_and_name_dots() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let r = record(
            b"A",
            false,
            &[susp(b"NM", b"\x01abc"), susp(b"NM", b"\x00def")],
        );
        assert_eq!(getname(&r, &imp).0, b"abcdef");
        let r = record(b"A", false, &[susp(b"NM", &[ISO_SUSP_CFLAG_PARENT])]);
        assert_eq!(getname(&r, &imp).0, b"..");
        let r = record(b"A", false, &[susp(b"NM", &[ISO_SUSP_CFLAG_CURRENT])]);
        assert_eq!(getname(&r, &imp).0, b".");
        // an NM with bad flags counts as no NM: the ISO name stands
        let r = record(b"A", false, &[susp(b"NM", b"\x40xyz")]);
        assert_eq!(getname(&r, &imp).0, b"A");
    }

    #[test]
    fn relocation_entries() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        // CL: a relocated directory's real location
        let r = record(
            b"D",
            true,
            &[susp(b"CL", &both32(40)), susp(b"NM", b"\x00d")],
        );
        let (name, res, ino) = getname(&r, &imp);
        assert_eq!((name.as_slice(), ino), (b"d".as_slice(), 40 << 11));
        assert_ne!(res & ISO_SUSP_CLINK, 0);
        // RE: the relocated directory itself, which is not listed
        let r = record(b"D", true, &[susp(b"RE", &[]), susp(b"NM", b"\x00d")]);
        let (name, res, _) = getname(&r, &imp);
        assert!(name.is_empty());
        assert_ne!(res & ISO_SUSP_RELDIR, 0);
    }

    #[test]
    fn symbolic_links_gather_their_components() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let (link, res) = getsymname(&image::LINK_REC, &imp);
        assert_eq!(link, b"../m10c-iso.txt");
        assert_eq!(res, ISO_SUSP_SLINK);

        let sl = |body: &[u8]| record(b"L", false, &[susp(b"SL", body)]);
        let r = sl(b"\x00\x08\x00\x00\x03etc\x00\x06passwd");
        assert_eq!(getsymname(&r, &imp).0, b"/etc/passwd");
        let r = sl(b"\x00\x02\x00\x00\x03bin");
        assert_eq!(getsymname(&r, &imp).0, b"./bin");
        // a component continued in the next SL entry
        let r = record(
            b"L",
            false,
            &[
                susp(b"SL", b"\x01\x01\x03abc"),
                susp(b"SL", b"\x00\x00\x03def"),
            ],
        );
        assert_eq!(getsymname(&r, &imp), (b"abcdef".to_vec(), ISO_SUSP_SLINK));
        // the mount point
        imp.im_mountp
            .with_stat(|sp| sp.f_mntonname[..6].copy_from_slice(b"/cdrom"));
        let r = sl(b"\x00\x10\x00\x00\x03etc");
        assert_eq!(getsymname(&r, &imp).0, b"/cdrom/etc");
        // a component running past the entry is invalid
        let r = sl(b"\x00\x00\x09abc");
        assert_eq!(getsymname(&r, &imp), (b"".to_vec(), 0));
    }

    #[test]
    fn symbolic_links_longer_than_maxpathlen_are_refused() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let mut body = std::vec![0u8];
        for _ in 0..40 {
            body.extend_from_slice(&[0, 30]);
            body.extend_from_slice(&[b'x'; 30]);
        }
        // more than one entry's worth: split over SL entries of at most 250 bytes
        let entries: Vec<Vec<u8>> = body[1..]
            .chunks(32 * 7)
            .map(|c| {
                let mut e = std::vec![1u8];
                e.extend_from_slice(c);
                susp(b"SL", &e)
            })
            .collect();
        // a record would be too long for its length byte: walk the entries as the loop does
        let mut out = [0u8; MAXPATHLEN];
        let mut ana = IsoRripAnalyze::new(&imp, ISO_SUSP_SLINK);
        ana.outbuf = &mut out;
        ana.maxlen = MAXPATHLEN as u16;
        ana.cont = 1;
        let mut res = 0;
        for e in &entries {
            res |= cd9660_rrip_slink(e, &mut ana);
            if ana.fields == 0 {
                break;
            }
        }
        assert_eq!(res, 0);
        assert_eq!(ana.outlen, 0);
        assert_eq!(ana.fields, 0);
    }

    #[test]
    fn attributes_come_from_px_and_pn() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let (ip, _vp) = test_node(&imp, 23 << 11);
        let res = cd9660_rrip_analyze(&rec(&image::FILE_REC), &ip, &imp, None);
        let ino = ip.inode();
        assert_eq!(u32::from(ino.iso_mode), S_IFREG | 0o644);
        assert_eq!((ino.iso_links, ino.iso_uid, ino.iso_gid), (1, 0, 0));
        assert_ne!(res & ISO_SUSP_ATTR, 0);
        // makefs writes TF entries without the byte that ISO_RRIP_TSTAMP's size counts, so
        // they are not read: the times are the record's
        assert_eq!(res & ISO_SUSP_TSTAMP, 0);
        assert_eq!(ino.iso_mtime, Timespec::new(1_791_110_208, 0));

        let (ip, _vp) = test_node(&imp, 22 << 11);
        cd9660_rrip_analyze(&rec(&image::LINK_REC), &ip, &imp, None);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFLNK | 0o755);

        let (ip, _vp) = test_node(&imp, 30 << 11);
        let r = record(
            b"TTY",
            false,
            &[
                px(S_IFCHR | 0o620, 1, 0, 4),
                susp(b"PN", &[both32(0), both32(makedev(5, 3) as u32)].concat()),
            ],
        );
        let res = cd9660_rrip_analyze(&rec(&r), &ip, &imp, None);
        assert_ne!(res & ISO_SUSP_DEVICE, 0);
        assert_eq!(ip.inode().iso_rdev, makedev(5, 3));
        assert_eq!(ip.inode().iso_gid, 4);
        let r = record(
            b"TTY",
            false,
            &[susp(
                b"PN",
                &[both32(7), both32(makedev(5, 3) as u32)].concat(),
            )],
        );
        cd9660_rrip_analyze(&rec(&r), &ip, &imp, None);
        assert_eq!(ip.inode().iso_rdev, makedev(7, 3));
    }

    #[test]
    fn time_stamps_come_from_tf() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let (ip, _vp) = test_node(&imp, 30 << 11);
        // modify and access, 7-byte form, plus the byte the C's size check counts
        let mut tf = std::vec![ISO_SUSP_TSTAMP_MODIFY | ISO_SUSP_TSTAMP_ACCESS];
        tf.extend_from_slice(&[99, 12, 31, 23, 59, 59, 0]);
        tf.extend_from_slice(&[70, 1, 1, 0, 0, 1, 0]);
        tf.push(0);
        let r = record(
            b"F",
            false,
            &[px(S_IFREG | 0o444, 1, 0, 0), susp(b"TF", &tf)],
        );
        let res = cd9660_rrip_analyze(&rec(&r), &ip, &imp, None);
        assert_ne!(res & ISO_SUSP_TSTAMP, 0);
        let ino = ip.inode();
        assert_eq!(ino.iso_mtime, Timespec::new(946_684_799, 0));
        assert_eq!(ino.iso_atime, Timespec::new(1, 0));
        assert_eq!(ino.iso_ctime, ino.iso_mtime);

        // 17-byte form with the creation time first
        let mut tf =
            std::vec![ISO_SUSP_TSTAMP_FORM17 | ISO_SUSP_TSTAMP_CREAT | ISO_SUSP_TSTAMP_ATTR];
        tf.extend_from_slice(b"2000010100000000\x00");
        tf.extend_from_slice(b"2026100407364800\xf4");
        tf.push(0);
        let r = record(
            b"F",
            false,
            &[px(S_IFREG | 0o444, 1, 0, 0), susp(b"TF", &tf)],
        );
        cd9660_rrip_analyze(&rec(&r), &ip, &imp, None);
        let ino = ip.inode();
        assert_eq!(ino.iso_ctime, Timespec::new(1_791_110_208, 0));
        assert_eq!(ino.iso_mtime, Timespec::new(0, 0));
        assert_eq!(ino.iso_atime, ino.iso_mtime);
    }

    #[test]
    fn rr_st_and_missing_px() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        let (ip, _vp) = test_node(&imp, 30 << 11);
        // RR says only PX is recorded: the walk ends after it, the times are the record's
        let mut tf = std::vec![ISO_SUSP_TSTAMP_MODIFY];
        tf.extend_from_slice(&[99, 12, 31, 23, 59, 59, 0, 0]);
        let r = record(
            b"F",
            false,
            &[
                susp(b"RR", &[0x01]),
                px(S_IFREG | 0o600, 1, 1000, 10),
                susp(b"TF", &tf),
            ],
        );
        let res = cd9660_rrip_analyze(&rec(&r), &ip, &imp, None);
        assert_eq!(res & ISO_SUSP_TSTAMP, 0);
        assert_eq!(ip.inode().iso_uid, 1000);
        assert_eq!(ip.inode().iso_mtime, Timespec::new(1_791_110_208, 0));

        // ST ends the system use area: the PX after it is not read, the defaults apply
        let (ip, _vp) = test_node(&imp, 31 << 11);
        let r = record(
            b"D",
            true,
            &[susp(b"ST", &[]), px(S_IFREG | 0o600, 1, 1000, 10)],
        );
        let res = cd9660_rrip_analyze(&rec(&r), &ip, &imp, None);
        assert_eq!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFDIR | 0o555);

        // the root's '.' keeps its PX in a continuation area this mount cannot reach
        let short = rrip_mnt(|m| m.volume_space_size = 24);
        let (ip, _vp) = test_node(&short, 20 << 11);
        let res = cd9660_rrip_analyze(&rec(&image::ROOT_DOT), &ip, &short, None);
        assert_eq!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFDIR | 0o555);
    }

    #[test]
    fn the_continuation_area_holds_the_root_attributes() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = rrip_mnt(|_| {});
        // the CE target of the root's '.' entry, walked as the loop walks it after the read
        let mut ana = IsoRripAnalyze::new(&imp, ISO_SUSP_ATTR | ISO_SUSP_TSTAMP);
        assert_eq!(cd9660_rrip_attr(&image::ROOT_CE, &mut ana), ISO_SUSP_ATTR);
        assert_eq!(u32::from(ana.inode.iso_mode), S_IFDIR | 0o755);
        assert_eq!(ana.inode.iso_links, 4);
        let mut ana = IsoRripAnalyze::new(&imp, ISO_SUSP_ATTR);
        let c = &image::ROOT_DOT[image::ROOT_DOT.len() - 28..];
        assert_eq!(cd9660_rrip_cont(c, &mut ana), ISO_SUSP_CONT);
        assert_eq!(
            (ana.iso_ce_blk, ana.iso_ce_off, ana.iso_ce_len),
            (24, 0, 36)
        );
        // and the whole walk reads it from the disc
        let (ip, _vp) = test_node(&imp, 20 << 11);
        let before = busy();
        let res = cd9660_rrip_analyze(&rec(&image::ROOT_DOT), &ip, &imp, None);
        assert_ne!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFDIR | 0o755);
        assert_eq!(busy(), before);
    }

    /// An image of `nblk` blocks of 2048 bytes: the makefs image's blocks, then zeros.
    fn big_image(nblk: usize) -> Vec<u8> {
        let mut img = image::image_bytes();
        img.resize(nblk * 2048, 0);
        img
    }

    /// Writes the area `entries` at the start of block `blk` of `img`.
    fn put_area(img: &mut [u8], blk: u32, entries: &[Vec<u8>]) -> u32 {
        let area: Vec<u8> = entries.concat();
        let at = blk as usize * 2048;
        img[at..at + area.len()].copy_from_slice(&area);
        area.len() as u32
    }

    /// A chain of `n` continuation areas in blocks 30.. of `img`, the last holding a PX of
    /// mode 0o100600: the record's CE (to block 30) and the image.
    fn chain(n: u32) -> (Vec<u8>, Vec<u8>) {
        let mut img = big_image(30 + n as usize + 1);
        let pxe = px(S_IFREG | 0o600, 1, 0, 0);
        let mut len = put_area(&mut img, 30 + n - 1, std::slice::from_ref(&pxe));
        for k in (0..n - 1).rev() {
            len = put_area(&mut img, 30 + k, &[ce(30 + k + 1, 0, len)]);
        }
        (ce(30, 0, len), img)
    }

    #[test]
    fn a_two_area_chain_is_followed_and_released() {
        let (c, img) = chain(2);
        let n = img.len() / 2048;
        let (_g, _p) = attach_image(img);
        let imp = rrip_mnt(|m| m.volume_space_size = n as i32);
        let (ip, _vp) = test_node(&imp, 30 << 11);
        let before = busy();
        let res = cd9660_rrip_analyze(&rec(&record(b"F", false, &[c])), &ip, &imp, None);
        assert_ne!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFREG | 0o600);
        // both continuation buffers were released (the C leaks the first)
        assert_eq!(busy(), before);
    }

    #[test]
    fn a_chain_ends_after_iso_rrip_maxce_areas() {
        let (c, img) = chain(ISO_RRIP_MAXCE as u32);
        let n = img.len() / 2048;
        let (_g, _p) = attach_image(img);
        let imp = rrip_mnt(|m| m.volume_space_size = n as i32);
        let (ip, _vp) = test_node(&imp, 30 << 11);
        let before = busy();
        let res = cd9660_rrip_analyze(&rec(&record(b"F", false, &[c])), &ip, &imp, None);
        // 64 areas: the PX in the last one is read
        assert_ne!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFREG | 0o600);
        assert_eq!(busy(), before);
        drop(ip);
        drop(imp);
    }

    #[test]
    fn a_chain_of_65_areas_gives_the_defaults() {
        let (c, img) = chain(ISO_RRIP_MAXCE as u32 + 1);
        let n = img.len() / 2048;
        let (_g, _p) = attach_image(img);
        let imp = rrip_mnt(|m| m.volume_space_size = n as i32);
        let (ip, _vp) = test_node(&imp, 30 << 11);
        let before = busy();
        let res = cd9660_rrip_analyze(&rec(&record(b"F", false, &[c])), &ip, &imp, None);
        // the 65th area is not read: the record's defaults
        assert_eq!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFREG | 0o555);
        assert_eq!(busy(), before);
    }

    #[test]
    fn a_ce_naming_a_block_the_walk_holds_ends_it() {
        let mut img = big_image(40);
        // a continuation area in block 31 whose CE names block 31 itself
        let len = put_area(&mut img, 31, &[ce(31, 0, 28)]);

        // the record, in block 32, whose CE names block 32 (its own, held by the caller)
        let r = record(b"F", false, &[ce(32, 0, 28)]);
        img[32 * 2048..32 * 2048 + r.len()].copy_from_slice(&r);
        let r2 = record(b"G", false, &[ce(31, 0, len)]);
        let (_g, _p) = attach_image(img);
        let imp = rrip_mnt(|m| m.volume_space_size = 40);
        let devvp = imp.im_devvp;

        // the record read as cd9660_vget_internal reads it: its block busy meanwhile
        let (t, e) = bread(&devvp.bufs, 32 << 2, 2048);
        e.expect("read");
        let (ip, _vp) = test_node(&imp, 32 << 11);
        let before = busy();
        let res = cd9660_rrip_analyze(&rec(&t.bytes()[..r.len()]), &ip, &imp, Some(32 << 2));
        // no hang: the CE is not followed, the defaults apply
        assert_eq!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFREG | 0o555);
        assert_eq!(busy(), before);
        brelse(t);

        // a continuation area that names itself: read once, then the walk ends
        let (ip, _vp) = test_node(&imp, 33 << 11);
        let before = busy();
        let res = cd9660_rrip_analyze(&rec(&r2), &ip, &imp, None);
        assert_eq!(res & ISO_SUSP_ATTR, 0);
        assert_eq!(u32::from(ip.inode().iso_mode), S_IFREG | 0o555);
        assert_eq!(busy(), before);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/isofs/cd9660/cd9660_rrip.h");
        for (name, value) in [
            ("ISO_SUSP_CFLAG_CONTINUE", ISO_SUSP_CFLAG_CONTINUE),
            ("ISO_SUSP_CFLAG_CURRENT", ISO_SUSP_CFLAG_CURRENT),
            ("ISO_SUSP_CFLAG_PARENT", ISO_SUSP_CFLAG_PARENT),
            ("ISO_SUSP_CFLAG_ROOT", ISO_SUSP_CFLAG_ROOT),
            ("ISO_SUSP_CFLAG_VOLROOT", ISO_SUSP_CFLAG_VOLROOT),
            ("ISO_SUSP_CFLAG_HOST", ISO_SUSP_CFLAG_HOST),
            ("ISO_RRIP_SLSIZ", ISO_RRIP_SLSIZ as u8),
            ("ISO_SUSP_TSTAMP_FORM17", ISO_SUSP_TSTAMP_FORM17),
            ("ISO_SUSP_TSTAMP_FORM7", ISO_SUSP_TSTAMP_FORM7),
            ("ISO_SUSP_TSTAMP_CREAT", ISO_SUSP_TSTAMP_CREAT),
            ("ISO_SUSP_TSTAMP_MODIFY", ISO_SUSP_TSTAMP_MODIFY),
            ("ISO_SUSP_TSTAMP_ACCESS", ISO_SUSP_TSTAMP_ACCESS),
            ("ISO_SUSP_TSTAMP_ATTR", ISO_SUSP_TSTAMP_ATTR),
            ("ISO_SUSP_TSTAMP_BACKUP", ISO_SUSP_TSTAMP_BACKUP),
            ("ISO_SUSP_TSTAMP_EXPIRE", ISO_SUSP_TSTAMP_EXPIRE),
            ("ISO_SUSP_TSTAMP_EFFECT", ISO_SUSP_TSTAMP_EFFECT),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value)),
                "{name}"
            );
        }
    }
}
/* </TESTS> */
