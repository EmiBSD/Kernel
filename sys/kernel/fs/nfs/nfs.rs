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
//! `<nfs/nfs.h>` in the kernel: what the header declares for the client and the server
//! beyond userland's image (`hal::sys::nfs_nfs`, re-exported here). N6.7a0 brings the
//! statistics' counters; the request and the RTT timers come with the RPC engine (N6.7a1).
//!
//! Upstream: sys/nfs/nfs.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfs.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct nfsstats` is bumped racily from everywhere in the C (`nfsstats.rpccnt[p]++`):
//!   here each counter is an `AtomicU64` written `Relaxed`, the `uvmexp` idiom; a sysctl
//!   reads them into the C's structure one by one, so the copy is not one instant (the C's
//!   `copyout` of a structure other CPUs bump is not either).
//!
//! ## Redesign
//! - LZ's `Nfsstats` was both the counters and the ABI: `#[repr(C)]` atomics viewed as 78
//!   `u64` words through a raw cast (two `unsafe`). The ABI is `hal`'s plain image
//!   ([`Nfsstats`]); the counters are [`NfsCounters`], whose members have the C's names (so
//!   `NFSSTATS.rpccnt[procnum].fetch_add(1, Relaxed)` reads as the C), converted to and from
//!   the image member by member by [`NfsCounters::snapshot`] and [`NfsCounters::restore`]:
//!   the image is built by a structure literal and taken apart whole, so a member missing on
//!   either side does not compile.

use core::sync::atomic::{AtomicU64, Ordering};

pub use crate::sys::nfs_nfs::*;
use crate::sys::nfs_nfsproto::NFS_NPROCS;

/// The kernel's `nfsstats`: one atomic per member of `struct nfsstats`, in the C's order,
/// bumped without a lock as the C bumps its words.
pub struct NfsCounters {
    /// `attrcache_hits`.
    pub attrcache_hits: AtomicU64,
    /// `attrcache_misses`.
    pub attrcache_misses: AtomicU64,
    /// `lookupcache_hits`.
    pub lookupcache_hits: AtomicU64,
    /// `lookupcache_misses`.
    pub lookupcache_misses: AtomicU64,
    /// `direofcache_hits`.
    pub direofcache_hits: AtomicU64,
    /// `direofcache_misses`.
    pub direofcache_misses: AtomicU64,
    /// `biocache_reads`.
    pub biocache_reads: AtomicU64,
    /// `read_bios`.
    pub read_bios: AtomicU64,
    /// `read_physios`.
    pub read_physios: AtomicU64,
    /// `biocache_writes`.
    pub biocache_writes: AtomicU64,
    /// `write_bios`.
    pub write_bios: AtomicU64,
    /// `write_physios`.
    pub write_physios: AtomicU64,
    /// `biocache_readlinks`.
    pub biocache_readlinks: AtomicU64,
    /// `readlink_bios`.
    pub readlink_bios: AtomicU64,
    /// `biocache_readdirs`.
    pub biocache_readdirs: AtomicU64,
    /// `readdir_bios`.
    pub readdir_bios: AtomicU64,
    /// `rpccnt`: one per procedure.
    pub rpccnt: [AtomicU64; NFS_NPROCS],
    /// `rpcretries`.
    pub rpcretries: AtomicU64,
    /// `srvrpccnt`: one per procedure.
    pub srvrpccnt: [AtomicU64; NFS_NPROCS],
    /// `srvrpc_errs`.
    pub srvrpc_errs: AtomicU64,
    /// `srv_errs`.
    pub srv_errs: AtomicU64,
    /// `rpcrequests`.
    pub rpcrequests: AtomicU64,
    /// `rpctimeouts`.
    pub rpctimeouts: AtomicU64,
    /// `rpcunexpected`.
    pub rpcunexpected: AtomicU64,
    /// `rpcinvalid`.
    pub rpcinvalid: AtomicU64,
    /// `srvcache_inproghits`.
    pub srvcache_inproghits: AtomicU64,
    /// `srvcache_idemdonehits`.
    pub srvcache_idemdonehits: AtomicU64,
    /// `srvcache_nonidemdonehits`.
    pub srvcache_nonidemdonehits: AtomicU64,
    /// `srvcache_misses`.
    pub srvcache_misses: AtomicU64,
    /// `forcedsync`.
    pub forcedsync: AtomicU64,
    /// `srvnqnfs_leases`.
    pub srvnqnfs_leases: AtomicU64,
    /// `srvnqnfs_maxleases`.
    pub srvnqnfs_maxleases: AtomicU64,
    /// `srvnqnfs_getleases`.
    pub srvnqnfs_getleases: AtomicU64,
    /// `srvvop_writes`.
    pub srvvop_writes: AtomicU64,
}

impl NfsCounters {
    /// All counters zero.
    pub const fn new() -> Self {
        Self {
            attrcache_hits: AtomicU64::new(0),
            attrcache_misses: AtomicU64::new(0),
            lookupcache_hits: AtomicU64::new(0),
            lookupcache_misses: AtomicU64::new(0),
            direofcache_hits: AtomicU64::new(0),
            direofcache_misses: AtomicU64::new(0),
            biocache_reads: AtomicU64::new(0),
            read_bios: AtomicU64::new(0),
            read_physios: AtomicU64::new(0),
            biocache_writes: AtomicU64::new(0),
            write_bios: AtomicU64::new(0),
            write_physios: AtomicU64::new(0),
            biocache_readlinks: AtomicU64::new(0),
            readlink_bios: AtomicU64::new(0),
            biocache_readdirs: AtomicU64::new(0),
            readdir_bios: AtomicU64::new(0),
            rpccnt: [const { AtomicU64::new(0) }; NFS_NPROCS],
            rpcretries: AtomicU64::new(0),
            srvrpccnt: [const { AtomicU64::new(0) }; NFS_NPROCS],
            srvrpc_errs: AtomicU64::new(0),
            srv_errs: AtomicU64::new(0),
            rpcrequests: AtomicU64::new(0),
            rpctimeouts: AtomicU64::new(0),
            rpcunexpected: AtomicU64::new(0),
            rpcinvalid: AtomicU64::new(0),
            srvcache_inproghits: AtomicU64::new(0),
            srvcache_idemdonehits: AtomicU64::new(0),
            srvcache_nonidemdonehits: AtomicU64::new(0),
            srvcache_misses: AtomicU64::new(0),
            forcedsync: AtomicU64::new(0),
            srvnqnfs_leases: AtomicU64::new(0),
            srvnqnfs_maxleases: AtomicU64::new(0),
            srvnqnfs_getleases: AtomicU64::new(0),
            srvvop_writes: AtomicU64::new(0),
        }
    }

    /// The counters as `struct nfsstats`, what `vfs.nfs.nfsstats` copies out. The structure
    /// literal names every member of the image, so one missing here does not compile.
    pub fn snapshot(&self) -> Nfsstats {
        let ld = |c: &AtomicU64| c.load(Ordering::Relaxed);
        let lda = |a: &[AtomicU64; NFS_NPROCS]| core::array::from_fn(|i| ld(&a[i]));
        Nfsstats {
            attrcache_hits: ld(&self.attrcache_hits),
            attrcache_misses: ld(&self.attrcache_misses),
            lookupcache_hits: ld(&self.lookupcache_hits),
            lookupcache_misses: ld(&self.lookupcache_misses),
            direofcache_hits: ld(&self.direofcache_hits),
            direofcache_misses: ld(&self.direofcache_misses),
            biocache_reads: ld(&self.biocache_reads),
            read_bios: ld(&self.read_bios),
            read_physios: ld(&self.read_physios),
            biocache_writes: ld(&self.biocache_writes),
            write_bios: ld(&self.write_bios),
            write_physios: ld(&self.write_physios),
            biocache_readlinks: ld(&self.biocache_readlinks),
            readlink_bios: ld(&self.readlink_bios),
            biocache_readdirs: ld(&self.biocache_readdirs),
            readdir_bios: ld(&self.readdir_bios),
            rpccnt: lda(&self.rpccnt),
            rpcretries: ld(&self.rpcretries),
            srvrpccnt: lda(&self.srvrpccnt),
            srvrpc_errs: ld(&self.srvrpc_errs),
            srv_errs: ld(&self.srv_errs),
            rpcrequests: ld(&self.rpcrequests),
            rpctimeouts: ld(&self.rpctimeouts),
            rpcunexpected: ld(&self.rpcunexpected),
            rpcinvalid: ld(&self.rpcinvalid),
            srvcache_inproghits: ld(&self.srvcache_inproghits),
            srvcache_idemdonehits: ld(&self.srvcache_idemdonehits),
            srvcache_nonidemdonehits: ld(&self.srvcache_nonidemdonehits),
            srvcache_misses: ld(&self.srvcache_misses),
            forcedsync: ld(&self.forcedsync),
            srvnqnfs_leases: ld(&self.srvnqnfs_leases),
            srvnqnfs_maxleases: ld(&self.srvnqnfs_maxleases),
            srvnqnfs_getleases: ld(&self.srvnqnfs_getleases),
            srvvop_writes: ld(&self.srvvop_writes),
        }
    }

    /// Overwrites the counters with `v`: what the `vfs.nfs.nfsstats` sysctl does when it is
    /// written (`nfs_sysctl`'s `copyin`). The image is destructured whole, so a member left
    /// out here does not compile.
    pub fn restore(&self, v: &Nfsstats) {
        let st = |c: &AtomicU64, w: u64| c.store(w, Ordering::Relaxed);
        let sta = |a: &[AtomicU64; NFS_NPROCS], w: [u64; NFS_NPROCS]| {
            for (c, w) in a.iter().zip(w) {
                st(c, w);
            }
        };
        let Nfsstats {
            attrcache_hits,
            attrcache_misses,
            lookupcache_hits,
            lookupcache_misses,
            direofcache_hits,
            direofcache_misses,
            biocache_reads,
            read_bios,
            read_physios,
            biocache_writes,
            write_bios,
            write_physios,
            biocache_readlinks,
            readlink_bios,
            biocache_readdirs,
            readdir_bios,
            rpccnt,
            rpcretries,
            srvrpccnt,
            srvrpc_errs,
            srv_errs,
            rpcrequests,
            rpctimeouts,
            rpcunexpected,
            rpcinvalid,
            srvcache_inproghits,
            srvcache_idemdonehits,
            srvcache_nonidemdonehits,
            srvcache_misses,
            forcedsync,
            srvnqnfs_leases,
            srvnqnfs_maxleases,
            srvnqnfs_getleases,
            srvvop_writes,
        } = *v;
        st(&self.attrcache_hits, attrcache_hits);
        st(&self.attrcache_misses, attrcache_misses);
        st(&self.lookupcache_hits, lookupcache_hits);
        st(&self.lookupcache_misses, lookupcache_misses);
        st(&self.direofcache_hits, direofcache_hits);
        st(&self.direofcache_misses, direofcache_misses);
        st(&self.biocache_reads, biocache_reads);
        st(&self.read_bios, read_bios);
        st(&self.read_physios, read_physios);
        st(&self.biocache_writes, biocache_writes);
        st(&self.write_bios, write_bios);
        st(&self.write_physios, write_physios);
        st(&self.biocache_readlinks, biocache_readlinks);
        st(&self.readlink_bios, readlink_bios);
        st(&self.biocache_readdirs, biocache_readdirs);
        st(&self.readdir_bios, readdir_bios);
        sta(&self.rpccnt, rpccnt);
        st(&self.rpcretries, rpcretries);
        sta(&self.srvrpccnt, srvrpccnt);
        st(&self.srvrpc_errs, srvrpc_errs);
        st(&self.srv_errs, srv_errs);
        st(&self.rpcrequests, rpcrequests);
        st(&self.rpctimeouts, rpctimeouts);
        st(&self.rpcunexpected, rpcunexpected);
        st(&self.rpcinvalid, rpcinvalid);
        st(&self.srvcache_inproghits, srvcache_inproghits);
        st(&self.srvcache_idemdonehits, srvcache_idemdonehits);
        st(&self.srvcache_nonidemdonehits, srvcache_nonidemdonehits);
        st(&self.srvcache_misses, srvcache_misses);
        st(&self.forcedsync, forcedsync);
        st(&self.srvnqnfs_leases, srvnqnfs_leases);
        st(&self.srvnqnfs_maxleases, srvnqnfs_maxleases);
        st(&self.srvnqnfs_getleases, srvnqnfs_getleases);
        st(&self.srvvop_writes, srvvop_writes);
    }
}

impl Default for NfsCounters {
    fn default() -> Self {
        Self::new()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the counters and their image: every member reaches its C word.

    use super::*;

    #[test]
    fn snapshot_is_in_c_order() {
        let st = NfsCounters::new();
        st.attrcache_hits.store(1, Ordering::Relaxed);
        st.rpccnt[0].store(2, Ordering::Relaxed);
        st.rpccnt[NFS_NPROCS - 1].store(3, Ordering::Relaxed);
        st.rpcretries.store(4, Ordering::Relaxed);
        st.srvvop_writes.store(5, Ordering::Relaxed);
        let w = st.snapshot();
        let words: &[u8] = hal::user::bytes_of(core::slice::from_ref(&w));
        let word = |i: usize| u64::from_ne_bytes(words[i * 8..i * 8 + 8].try_into().unwrap());
        assert_eq!(word(0), 1);
        assert_eq!(word(16), 2);
        assert_eq!(word(16 + NFS_NPROCS - 1), 3);
        assert_eq!(word(16 + NFS_NPROCS), 4);
        assert_eq!(word(Nfsstats::NWORDS - 1), 5);
    }

    #[test]
    fn restore_then_snapshot_is_the_identity_on_every_word() {
        // Each of the 78 words gets its own value, so a member mapped to the wrong one (or
        // left out of `restore`) shows.
        let mut img = Nfsstats::new();
        let bytes = hal::user::bytes_of_mut(core::slice::from_mut(&mut img));
        for (i, w) in bytes.chunks_exact_mut(8).enumerate() {
            w.copy_from_slice(&(i as u64 * 0x0101_0101 + 7).to_ne_bytes());
        }
        let st = NfsCounters::new();
        st.restore(&img);
        assert_eq!(st.snapshot(), img);
        assert_eq!(st.srvrpccnt[3].load(Ordering::Relaxed), img.srvrpccnt[3]);
    }
}
/* </TESTS> */
