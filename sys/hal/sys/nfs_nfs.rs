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
 * Copyright (c) 1989, 1993, 1995
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfs.h	8.4 (Berkeley) 5/1/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfs.h>`'s part userland reads (an installed header): the tunables, the `nfssvc(2)`
//! flags, the `vfs.nfs` sysctl(3) identifiers and `struct nfsstats`, the counters
//! `vfs.nfs.nfsstats` copies out and back in (nfsstat(1)). Its image is `hal`'s (N6.7a0,
//! D-10); the counters themselves are the kernel's (`kernel::fs::nfs::nfs::NfsCounters`).
//!
//! Upstream: sys/nfs/nfs.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfs.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct nfsstats`' `uint64_t` members are `u64`, `rpccnt`/`srvrpccnt` arrays of
//!   `NFS_NPROCS`; the layout is the C's, 78 words with no hole (checked below).
//!
//! ## Redesign
//! - LZ's `Nfsstats` was the counters themselves, `#[repr(C)]` `AtomicU64`s read and written
//!   as 78 words through a raw cast (two `unsafe` slices): here it is the plain image the
//!   sysctl copies, an [`crate::abi_pod!`], and the kernel's counters convert to and from it
//!   member by member (`NfsCounters::snapshot`, `NfsCounters::restore`), no cast.
//! - The rest of `nfs.h` (the request, the server's socket, thread and descriptor, the RTT
//!   timers, `NFSINT_SIGMASK`, `nfsd_args`) is the kernel's or `bsd`'s, not userland's.

use crate::sys::nfs_nfsproto::NFS_NPROCS;
use crate::sys::sysctl::{CTLTYPE_INT, CTLTYPE_STRUCT, Ctlname};

/// `NFS_TICKINTVL`: desired time for a tick (msec).
pub const NFS_TICKINTVL: i32 = 5;
/// `NFS_TIMEOUTMUL`: timeout/delay multiplier.
pub const NFS_TIMEOUTMUL: i32 = 2;
/// `NFS_MAXREXMIT`: stop counting after this many.
pub const NFS_MAXREXMIT: i32 = 100;
/// `NFS_RETRANS`: num of retrans for soft mounts.
pub const NFS_RETRANS: i32 = 10;
/// `NFS_MAXGRPS`: max. size of groups list.
pub const NFS_MAXGRPS: i32 = 16;
/// `NFS_MINATTRTIMO`: attribute cache timeout in sec.
pub const NFS_MINATTRTIMO: i32 = 5;
/// `NFS_MAXATTRTIMO`.
pub const NFS_MAXATTRTIMO: i32 = 60;
/// `NFS_WSIZE`: def. write data size <= 8192.
pub const NFS_WSIZE: i32 = 8192;
/// `NFS_RSIZE`: def. read data size <= 8192.
pub const NFS_RSIZE: i32 = 8192;
/// `NFS_READDIRSIZE`: def. readdir size.
pub const NFS_READDIRSIZE: i32 = 8192;
/// `NFS_DEFRAHEAD`: def. read ahead # blocks.
pub const NFS_DEFRAHEAD: i32 = 1;
/// `NFS_MAXRAHEAD`: max. read ahead # blocks.
pub const NFS_MAXRAHEAD: i32 = 4;
/// `NFS_MAXASYNCDAEMON`: max. number async_daemons runable.
pub const NFS_MAXASYNCDAEMON: i32 = 20;

/// `NFS_DIRBLKSIZ`: must be a multiple of `DIRBLKSIZ`. Ideally it should be bigger, but I've
/// seen servers with broken NFS/ethernet drivers that won't work with anything bigger
/// (Linux..).
pub const NFS_DIRBLKSIZ: i32 = 1024;
/// `NFS_READDIRBLKSIZ`: size of read dir blocks. XXX
pub const NFS_READDIRBLKSIZ: i32 = 512;

/// `NFSSVC_NFSD`: flags for the `nfssvc()` system call.
pub const NFSSVC_NFSD: i32 = 0x004;
/// `NFSSVC_ADDSOCK`.
pub const NFSSVC_ADDSOCK: i32 = 0x008;

/// `NFS_NFSSTATS`: `vfs.nfs` sysctl(3) identifiers; struct: `struct nfsstats`.
pub const NFS_NFSSTATS: i32 = 1;
/// `NFS_NIOTHREADS`: number of i/o threads.
pub const NFS_NIOTHREADS: i32 = 2;
/// `NFS_MAXID`.
pub const NFS_MAXID: i32 = 3;

/// `FS_NFS_NAMES`.
pub const FS_NFS_NAMES: [Ctlname; NFS_MAXID as usize] = [
    Ctlname::NONE,
    Ctlname::new(b"nfsstats", CTLTYPE_STRUCT),
    Ctlname::new(b"iothreads", CTLTYPE_INT),
];

/// `struct nfsstats`: stats structure, the `vfs.nfs.nfsstats` sysctl(3) ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nfsstats {
    /// `attrcache_hits`.
    pub attrcache_hits: u64,
    /// `attrcache_misses`.
    pub attrcache_misses: u64,
    /// `lookupcache_hits`.
    pub lookupcache_hits: u64,
    /// `lookupcache_misses`.
    pub lookupcache_misses: u64,
    /// `direofcache_hits`.
    pub direofcache_hits: u64,
    /// `direofcache_misses`.
    pub direofcache_misses: u64,
    /// `biocache_reads`.
    pub biocache_reads: u64,
    /// `read_bios`.
    pub read_bios: u64,
    /// `read_physios`.
    pub read_physios: u64,
    /// `biocache_writes`.
    pub biocache_writes: u64,
    /// `write_bios`.
    pub write_bios: u64,
    /// `write_physios`.
    pub write_physios: u64,
    /// `biocache_readlinks`.
    pub biocache_readlinks: u64,
    /// `readlink_bios`.
    pub readlink_bios: u64,
    /// `biocache_readdirs`.
    pub biocache_readdirs: u64,
    /// `readdir_bios`.
    pub readdir_bios: u64,
    /// `rpccnt`.
    pub rpccnt: [u64; NFS_NPROCS],
    /// `rpcretries`.
    pub rpcretries: u64,
    /// `srvrpccnt`.
    pub srvrpccnt: [u64; NFS_NPROCS],
    /// `srvrpc_errs`.
    pub srvrpc_errs: u64,
    /// `srv_errs`.
    pub srv_errs: u64,
    /// `rpcrequests`.
    pub rpcrequests: u64,
    /// `rpctimeouts`.
    pub rpctimeouts: u64,
    /// `rpcunexpected`.
    pub rpcunexpected: u64,
    /// `rpcinvalid`.
    pub rpcinvalid: u64,
    /// `srvcache_inproghits`.
    pub srvcache_inproghits: u64,
    /// `srvcache_idemdonehits`.
    pub srvcache_idemdonehits: u64,
    /// `srvcache_nonidemdonehits`.
    pub srvcache_nonidemdonehits: u64,
    /// `srvcache_misses`.
    pub srvcache_misses: u64,
    /// `forcedsync`.
    pub forcedsync: u64,
    /// `srvnqnfs_leases`.
    pub srvnqnfs_leases: u64,
    /// `srvnqnfs_maxleases`.
    pub srvnqnfs_maxleases: u64,
    /// `srvnqnfs_getleases`.
    pub srvnqnfs_getleases: u64,
    /// `srvvop_writes`.
    pub srvvop_writes: u64,
}

impl Nfsstats {
    /// The number of `uint64_t` words in the structure.
    pub const NWORDS: usize = size_of::<Nfsstats>() / size_of::<u64>();

    /// All counters zero.
    pub const fn new() -> Self {
        Self {
            attrcache_hits: 0,
            attrcache_misses: 0,
            lookupcache_hits: 0,
            lookupcache_misses: 0,
            direofcache_hits: 0,
            direofcache_misses: 0,
            biocache_reads: 0,
            read_bios: 0,
            read_physios: 0,
            biocache_writes: 0,
            write_bios: 0,
            write_physios: 0,
            biocache_readlinks: 0,
            readlink_bios: 0,
            biocache_readdirs: 0,
            readdir_bios: 0,
            rpccnt: [0; NFS_NPROCS],
            rpcretries: 0,
            srvrpccnt: [0; NFS_NPROCS],
            srvrpc_errs: 0,
            srv_errs: 0,
            rpcrequests: 0,
            rpctimeouts: 0,
            rpcunexpected: 0,
            rpcinvalid: 0,
            srvcache_inproghits: 0,
            srvcache_idemdonehits: 0,
            srvcache_nonidemdonehits: 0,
            srvcache_misses: 0,
            forcedsync: 0,
            srvnqnfs_leases: 0,
            srvnqnfs_maxleases: 0,
            srvnqnfs_getleases: 0,
            srvvop_writes: 0,
        }
    }
}

impl Default for Nfsstats {
    fn default() -> Self {
        Self::new()
    }
}

crate::abi_pod!(Nfsstats {
    attrcache_hits: u64,
    attrcache_misses: u64,
    lookupcache_hits: u64,
    lookupcache_misses: u64,
    direofcache_hits: u64,
    direofcache_misses: u64,
    biocache_reads: u64,
    read_bios: u64,
    read_physios: u64,
    biocache_writes: u64,
    write_bios: u64,
    write_physios: u64,
    biocache_readlinks: u64,
    readlink_bios: u64,
    biocache_readdirs: u64,
    readdir_bios: u64,
    rpccnt: [u64; NFS_NPROCS],
    rpcretries: u64,
    srvrpccnt: [u64; NFS_NPROCS],
    srvrpc_errs: u64,
    srv_errs: u64,
    rpcrequests: u64,
    rpctimeouts: u64,
    rpcunexpected: u64,
    rpcinvalid: u64,
    srvcache_inproghits: u64,
    srvcache_idemdonehits: u64,
    srvcache_nonidemdonehits: u64,
    srvcache_misses: u64,
    forcedsync: u64,
    srvnqnfs_leases: u64,
    srvnqnfs_maxleases: u64,
    srvnqnfs_getleases: u64,
    srvvop_writes: u64,
});

const _: () = {
    assert!(Nfsstats::NWORDS == 78);
    assert!(size_of::<Nfsstats>() == 624);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `<nfs/nfs.h>`'s userland part: the statistics' layout and, against the
    // C header, the constants.

    use core::mem::offset_of;

    use super::*;

    #[test]
    fn nfsstats_members_sit_at_the_c_offsets() {
        assert_eq!(offset_of!(Nfsstats, attrcache_hits), 0);
        assert_eq!(offset_of!(Nfsstats, rpccnt), 16 * 8);
        assert_eq!(offset_of!(Nfsstats, rpcretries), (16 + NFS_NPROCS) * 8);
        assert_eq!(offset_of!(Nfsstats, srvrpccnt), (17 + NFS_NPROCS) * 8);
        assert_eq!(offset_of!(Nfsstats, srvvop_writes), 77 * 8);
        assert_eq!(Nfsstats::new(), Nfsstats::default());
    }

    #[test]
    fn the_sysctl_names_are_the_c_table() {
        assert_eq!(
            FS_NFS_NAMES[NFS_NFSSTATS as usize].ctl_name,
            Some(&b"nfsstats"[..])
        );
        assert_eq!(FS_NFS_NAMES[NFS_NIOTHREADS as usize].ctl_type, CTLTYPE_INT);
        assert_eq!(FS_NFS_NAMES[0], Ctlname::NONE);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/nfs/nfs.h");
        let ours: &[(&str, i64)] = &[
            ("NFS_TICKINTVL", NFS_TICKINTVL.into()),
            ("NFS_TIMEOUTMUL", NFS_TIMEOUTMUL.into()),
            ("NFS_MAXREXMIT", NFS_MAXREXMIT.into()),
            ("NFS_RETRANS", NFS_RETRANS.into()),
            ("NFS_MAXGRPS", NFS_MAXGRPS.into()),
            ("NFS_MINATTRTIMO", NFS_MINATTRTIMO.into()),
            ("NFS_MAXATTRTIMO", NFS_MAXATTRTIMO.into()),
            ("NFS_WSIZE", NFS_WSIZE.into()),
            ("NFS_RSIZE", NFS_RSIZE.into()),
            ("NFS_READDIRSIZE", NFS_READDIRSIZE.into()),
            ("NFS_DEFRAHEAD", NFS_DEFRAHEAD.into()),
            ("NFS_MAXRAHEAD", NFS_MAXRAHEAD.into()),
            ("NFS_MAXASYNCDAEMON", NFS_MAXASYNCDAEMON.into()),
            ("NFS_DIRBLKSIZ", NFS_DIRBLKSIZ.into()),
            ("NFS_READDIRBLKSIZ", NFS_READDIRBLKSIZ.into()),
            ("NFSSVC_NFSD", NFSSVC_NFSD.into()),
            ("NFSSVC_ADDSOCK", NFSSVC_ADDSOCK.into()),
            ("NFS_NFSSTATS", NFS_NFSSTATS.into()),
            ("NFS_NIOTHREADS", NFS_NIOTHREADS.into()),
            ("NFS_MAXID", NFS_MAXID.into()),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
