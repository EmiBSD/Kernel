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
 * Copyright (c) 1989, 1993
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
 *	@(#)nfs_srvcache.c	8.3 (Berkeley) 3/30/95
 */

/*
 * Copyright (c) 1989, 1993
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
 *	@(#)nfsrvcache.h	8.3 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS server's recent request cache, the duplicate request cache (N6.7b2,
//! `nfs_srvcache.c` and `nfsrvcache.h`): requests of datagram transports are remembered by
//! transaction id, procedure and client address, so that a retransmission of a request still
//! in progress is dropped, one of a non-idempotent request that has completed gets the saved
//! reply, and one of an idempotent request is simply done again. The server's since N6.7b4
//! (`nfs_init` makes it, after `nfsrv_init(0)`).
//!
//! Upstream: sys/nfs/nfs_srvcache.c @ 3ce1f3f79392
//! Upstream: sys/nfs/nfsrvcache.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_srvcache.rs@e8249449d905
//! LZ: sys/nfs/nfsrvcache.rs@e8249449d905
//!
//! Reference: Chet Juszczak, "Improving the Performance and Correctness of an NFS Server",
//! in Proc. Winter 1989 USENIX Conference, pages 53-63. San Diego, February 1989.
//!
//! `option NFSSERVER` only: the module is behind the kernel's `nfsserver` feature.
//!
//! ## Deviations
//! - `nfsrv_getcache` answers the action and the reply (the C's `*repp`). When no memory is
//!   to be had where the C's `M_WAIT` cannot fail (here a pool cannot grow past its limit), a
//!   reply that cannot be copied drops the request (`RC_DROPIT`; the client retransmits), an
//!   entry that cannot be allocated lets the request through uncached (`RC_DOIT`), and
//!   `nfsrv_updatecache` leaves the entry without a saved reply (a retransmission is then
//!   done again); the C would store a null reply and fault on the next hit (LZ's).
//! - A full cache whose LRU list is empty (a `desirednfsrvcache` of 0) lets the request
//!   through uncached, where the C dereferences `TAILQ_FIRST` of the empty list.
//! - `nfsrv_cleanentry` frees a copied address once: the C leaves `RC_NAM` set after its
//!   `m_free`, so the next clean of the entry frees the mbuf again.
//! - `vmstat -m`: the entries are items of a pool of their own, "nfsrvcpl", where the C
//!   `malloc`s them as `M_NFSD` (D-12's rule for what only `vmstat -m` shows).
//! - `union nethostaddr` is the entry's two members side by side (`rc_inetaddr`, `rc_nam`),
//!   the C's flag (`RC_INETADDR`, `RC_NAM`) saying which one holds the key.
//!
//! ## Redesign
//! - LZ's entries were `malloc(9)` blocks cast to `&'static` with `Cell` members, linked on a
//!   `TailqHead` and on hash chains of a `StaticCell` table through raw links (eleven
//!   `unsafe`: the table and key, the entry's making and freeing, every insert and remove,
//!   three `unsafe impl`). Here an entry is a `PoolBox` the LRU list owns (an `OwnedList`)
//!   and a counted member of its hash chain (a `SecondaryList` per bucket, the chains an
//!   `MArray` of `M_NFSD`, the key `libkern`'s SipHash of the xid under a random key); the
//!   table is one cell of the kernel lock, the C's lock for the cache: a lookup walks its
//!   chain under the lock's reader (no spl), the list changes take its witness. Its members
//!   are relaxed atomics, and the saved reply and the copied address `LoanSlot`s (an mbuf is
//!   `Send`, not `Sync`). `RC_LOCKED` and `RC_WANTED` and the sleep on the entry ("nfsrc")
//!   are the C's, the chain walked again after it; the holder of a locked entry keeps it by
//!   a counted `SecondaryRef` across the copies that may sleep (`m_copym(M_WAIT)`,
//!   `nfs_rephead`), and the list refuses to give back a counted entry, so no entry is freed
//!   under its holder. A new or recycled entry is filled while it is on no list and linked
//!   on both at once.

use core::ptr;
use core::sync::atomic::Ordering::Relaxed;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU8, AtomicU16, AtomicU32};

use hal::klock;
use hal::malloc::{M_WAITOK, MArray};
use hal::owned::{
    Member, OwnedAdapter, OwnedEntry, OwnedList, SecondaryAdapter, SecondaryEntry, SecondaryList,
    SecondaryRef,
};
use hal::pool::{BoxPool, PR_WAITOK, PoolBox};
use hal::rwlock::Guarded;
use hal::sleep::{INFSLP, tsleep_nsec, wakeup};
use hal::slot::LoanSlot;
use libkern::siphash::{SipHash24, SiphashKey};

use super::nfs_srvsocket::nfs_rephead;
use super::nfs_srvsubs::{netaddr_match, sockaddr_in_of};
use super::nfs_subs::{NFSSTATS, NFSV2_PROCID};
use super::nfsproto::NFS_NPROCS;
use super::nfssvc::{M_NFSD, ND_NFSV3, NfsrvDescript, NfssvcSock};
use crate::assert::kpanic;
use crate::fs::subr::{siphash_key, vfs};
use crate::net::mbuf::{M_COPYALL, M_WAIT, Mbuf, MbufBox, m_copym};
use crate::proc::ProcGlue;
use crate::sched::PZERO;
use crate::sys::socket::{AF_INET, AF_UNSPEC};
use crate::sys::types::SaFamily;

/// `NFSRVCACHESIZ`: the entries the cache holds at most (`desirednfsrvcache`'s default).
pub const NFSRVCACHESIZ: usize = 2048;

/// `RC_UNUSED`: cache entry states.
pub const RC_UNUSED: u8 = 0;
/// `RC_INPROG`.
pub const RC_INPROG: u8 = 1;
/// `RC_DONE`.
pub const RC_DONE: u8 = 2;

/// `RC_DROPIT`: return values.
pub const RC_DROPIT: i32 = 0;
/// `RC_REPLY`.
pub const RC_REPLY: i32 = 1;
/// `RC_DOIT`.
pub const RC_DOIT: i32 = 2;
/// `RC_CHECKIT`.
pub const RC_CHECKIT: i32 = 3;

/// `RC_LOCKED`: flag bits.
pub const RC_LOCKED: u8 = 0x01;
/// `RC_WANTED`.
pub const RC_WANTED: u8 = 0x02;
/// `RC_REPSTATUS`.
pub const RC_REPSTATUS: u8 = 0x04;
/// `RC_REPMBUF`.
pub const RC_REPMBUF: u8 = 0x08;
/// `RC_INETADDR`.
pub const RC_INETADDR: u8 = 0x20;
/// `RC_NAM`.
pub const RC_NAM: u8 = 0x40;

/// `nonidempotent[]`: which nfs rpc's are nonidempotent.
const NONIDEMPOTENT: [bool; NFS_NPROCS] = {
    let mut t = [false; NFS_NPROCS];
    t[2] = true;
    let mut i = 7;
    while i <= 15 {
        t[i] = true;
        i += 1;
    }
    t
};

/// `nfsv2_repstat[]`: true iff the rpc reply is an nfs status ONLY! (version 2 numbers).
const NFSV2_REPSTAT: [bool; NFS_NPROCS] = {
    let mut t = [false; NFS_NPROCS];
    t[10] = true;
    t[11] = true;
    t[12] = true;
    t[13] = true;
    t[15] = true;
    t
};

/// `struct nfsrvcache`: an entry of the cache (the module's Redesign).
pub struct NfsrvCache {
    /// `rc_lru`: LRU chain, which owns the entry.
    rc_lru: OwnedEntry<NfsrvCache>,
    /// `rc_hash`: hash chain.
    rc_hash: SecondaryEntry<NfsrvCache>,
    /// `rc_xid`: rpc id number (host order, as `nd_retxid`).
    pub rc_xid: AtomicU32,
    /// `rc_reply` (`rc_un.ru_repmb`): reply mbuf list (`RC_REPMBUF`).
    pub rc_reply: LoanSlot<Option<MbufBox>>,
    /// `rc_status` (`rc_un.ru_repstat`): reply status (`RC_REPSTATUS`).
    pub rc_status: AtomicI32,
    /// `rc_inetaddr` (`rc_haddr.had_inetaddr`, `RC_INETADDR`).
    pub rc_inetaddr: AtomicU32,
    /// `rc_nam` (`rc_haddr.had_nam`, `RC_NAM`): a copy of the client's address.
    pub rc_nam: LoanSlot<Option<MbufBox>>,
    /// `rc_proc`: rpc proc number.
    pub rc_proc: AtomicU16,
    /// `rc_state`: current state of request.
    pub rc_state: AtomicU8,
    /// `rc_flag`: flag bits.
    pub rc_flag: AtomicU8,
}

impl NfsrvCache {
    /// An entry with every member cleared (`malloc(M_ZERO)`).
    pub const fn new() -> Self {
        Self {
            rc_lru: OwnedEntry::new(),
            rc_hash: SecondaryEntry::new(),
            rc_xid: AtomicU32::new(0),
            rc_reply: LoanSlot::new(None),
            rc_status: AtomicI32::new(0),
            rc_inetaddr: AtomicU32::new(0),
            rc_nam: LoanSlot::new(None),
            rc_proc: AtomicU16::new(0),
            rc_state: AtomicU8::new(RC_UNUSED),
            rc_flag: AtomicU8::new(0),
        }
    }

    /// `rp->rc_flag`.
    #[inline]
    pub fn flag(&self) -> u8 {
        self.rc_flag.load(Relaxed)
    }

    /// `rp->rc_state`.
    #[inline]
    pub fn state(&self) -> u8 {
        self.rc_state.load(Relaxed)
    }
}

impl Default for NfsrvCache {
    fn default() -> Self {
        Self::new()
    }
}

/// `TAILQ_HEAD(nfsrvlru, nfsrvcache)`: the cache's LRU list, through `rc_lru`.
pub struct RcLru;

impl OwnedAdapter for RcLru {
    type Elem = NfsrvCache;
    fn entry(e: &NfsrvCache) -> &OwnedEntry<NfsrvCache> {
        &e.rc_lru
    }
    /// A list has no order of its own.
    fn cmp(_a: &NfsrvCache, _b: &NfsrvCache) -> core::cmp::Ordering {
        core::cmp::Ordering::Equal
    }
}

/// `LIST_HEAD(nfsrvhash, nfsrvcache)`: a hash chain of the cache, through `rc_hash`.
pub struct RcHash;

impl SecondaryAdapter for RcHash {
    type Owner = RcLru;
    fn entry(e: &NfsrvCache) -> &SecondaryEntry<NfsrvCache> {
        &e.rc_hash
    }
    /// A list has no order of its own.
    fn cmp(_a: &NfsrvCache, _b: &NfsrvCache) -> core::cmp::Ordering {
        core::cmp::Ordering::Equal
    }
}

/// The cache's lists, under the kernel lock (`nfsrvhashtbl`, `nfsrvhash`, `nfsrvhashkey`,
/// `nfsrvlruhead`).
pub struct NfsrvCacheTable {
    /// `nfsrvhashtbl`: the hash chains (`nfsrv_initcache`'s `hashinit`), none before it.
    nfsrvhashtbl: Option<MArray<SecondaryList<RcHash>>>,
    /// `nfsrvhash`: the mask of a chain's index.
    nfsrvhash: u64,
    /// `nfsrvhashkey`.
    nfsrvhashkey: SiphashKey,
    /// `nfsrvlruhead`: every entry, least recently used first.
    nfsrvlruhead: OwnedList<RcLru>,
}

impl NfsrvCacheTable {
    /// No chains, no entry.
    pub const fn new() -> Self {
        Self {
            nfsrvhashtbl: None,
            nfsrvhash: 0,
            nfsrvhashkey: SiphashKey { k0: 0, k1: 0 },
            nfsrvlruhead: OwnedList::new(),
        }
    }

    /// `NFSRCHASH(xid)`: the index of `xid`'s chain.
    fn chain_of(&self, xid: u32) -> usize {
        (SipHash24(&self.nfsrvhashkey, &xid.to_ne_bytes()) & self.nfsrvhash) as usize
    }

    /// `xid`'s chain (`None` before `nfsrv_initcache`).
    fn chain(&self, xid: u32) -> Option<&SecondaryList<RcHash>> {
        let i = self.chain_of(xid);
        self.nfsrvhashtbl.as_deref().and_then(|t| t.get(i))
    }

    /// The entries, least recently used first.
    pub fn lru(&self) -> &OwnedList<RcLru> {
        &self.nfsrvlruhead
    }
}

impl Default for NfsrvCacheTable {
    fn default() -> Self {
        Self::new()
    }
}

/// The cache's state in the server's tables (`numnfsrvcache`, `desirednfsrvcache`, the
/// lists and the entries' pool).
pub struct NfsrvCacheState {
    /// The lists, a cell of the kernel lock.
    pub table: Guarded<NfsrvCacheTable>,
    /// `numnfsrvcache`: the entries allocated.
    pub numnfsrvcache: AtomicI64,
    /// `desirednfsrvcache`: the entries allocated before the least recently used is
    /// recycled.
    pub desirednfsrvcache: AtomicI64,
    /// The entries ("nfsrvcpl"; the C's `M_NFSD` blocks).
    pub nfsrvcachepl: BoxPool<NfsrvCache>,
}

impl NfsrvCacheState {
    /// An empty cache, `NFSRVCACHESIZ` entries desired, the pool not made.
    pub const fn new() -> Self {
        Self {
            table: Guarded::new_kernel(NfsrvCacheTable::new()),
            numnfsrvcache: AtomicI64::new(0),
            desirednfsrvcache: AtomicI64::new(NFSRVCACHESIZ as i64),
            nfsrvcachepl: BoxPool::new(),
        }
    }
}

impl Default for NfsrvCacheState {
    fn default() -> Self {
        Self::new()
    }
}

/// `NETFAMILY(rp)`: the family of an entry's key.
fn netfamily(rp: &NfsrvCache) -> SaFamily {
    if rp.flag() & RC_INETADDR != 0 {
        AF_INET
    } else {
        AF_UNSPEC
    }
}

/// `rp->rc_flag &= ~RC_LOCKED`, and the sleepers on the entry woken if any wanted it.
fn unlock_entry(rp: &NfsrvCache) {
    rp.rc_flag.fetch_and(!RC_LOCKED, Relaxed);
    if rp.flag() & RC_WANTED != 0 {
        rp.rc_flag.fetch_and(!RC_WANTED, Relaxed);
        wakeup(ptr::from_ref(rp));
    }
}

/// `nfsrv_cleanentry(rp)`: the saved reply and the copied address freed, the reply flags
/// cleared.
pub fn nfsrv_cleanentry(rp: &NfsrvCache) {
    if rp.flag() & RC_REPMBUF != 0 {
        drop(rp.rc_reply.take());
    }
    if rp.flag() & RC_NAM != 0 {
        drop(rp.rc_nam.take());
    }
    rp.rc_flag.fetch_and(!(RC_REPSTATUS | RC_REPMBUF), Relaxed);
}

/// `nfsrv_initcache()`: initialize the server request cache list: the hash chains (a
/// `hashinit(desirednfsrvcache, M_NFSD, M_WAITOK)`, its size the power of two at or above),
/// their random key, the LRU list empty, and the entries' pool made (once).
pub fn nfsrv_initcache<G: ProcGlue>() {
    let c = &vfs::<G>().nfsrv.cache;
    let _ = c
        .nfsrvcachepl
        .init(hal::intr::IPL_NONE, PR_WAITOK, "nfsrvcpl");
    let desired = c.desirednfsrvcache.load(Relaxed);
    if desired <= 0 {
        kpanic(format_args!("hashinit: bad cnt"));
    }
    let mut size = 1usize;
    while (size as i64) < desired {
        size <<= 1;
    }
    let Some(tbl) = MArray::new(M_NFSD, M_WAITOK, size, |_| SecondaryList::new()) else {
        kpanic(format_args!(
            "nfsrv_initcache: no memory for the hash table"
        ));
    };
    let key = siphash_key();
    let old = {
        let mut w = klock::witness();
        let t = c.table.get_mut(&mut w);
        t.nfsrvhashkey = key;
        t.nfsrvhash = (size - 1) as u64;
        t.nfsrvhashtbl.replace(tbl)
    };
    drop(old);
}

/// `nfsrv_getcache(nd, slp, repp)`: looks for the request in the cache. If found, returns the
/// action and optionally the reply, else inserts it in the cache. The rules are as follows:
/// - if in progress, return DROP request
/// - if completed within DELAY of the current time, return DROP it
/// - if completed a longer time ago return REPLY if the reply was cached or return DOIT
///
/// Update/add new request at end of lru list. Requests of reliable transport protocols
/// (no `nd_nam2`) are not cached (maybe we should for the case of a reconnect, but..).
pub fn nfsrv_getcache<G: ProcGlue>(
    nd: &mut NfsrvDescript,
    slp: &NfssvcSock<G>,
) -> (i32, Option<MbufBox>) {
    if nd.nd_nam2.is_none() {
        return (RC_DOIT, None);
    }
    let c = &vfs::<G>().nfsrv.cache;

    if let Some(held) = nfsrv_lookupcache::<G>(nd) {
        let Some(rp) = held.get() else {
            kpanic(format_args!("nfsrv cache: a lookup's entry is gone"));
        };
        {
            // If not at end of LRU chain, move it there
            let mut w = klock::witness();
            let t = c.table.get_mut(&mut w);
            if t.nfsrvlruhead.next(rp).is_some() {
                t.nfsrvlruhead.move_to_tail(rp);
            }
        }
        let state = rp.state();
        let (ret, reply) = if state == RC_UNUSED {
            kpanic(format_args!("nfsrv cache"));
        } else if state == RC_INPROG {
            NFSSTATS.srvcache_inproghits.fetch_add(1, Relaxed);
            (RC_DROPIT, None)
        } else if rp.flag() & RC_REPSTATUS != 0 {
            NFSSTATS.srvcache_nonidemdonehits.fetch_add(1, Relaxed);
            let status = rp.rc_status.load(Relaxed);
            (RC_REPLY, Some(nfs_rephead(0, nd, slp, status).into_chain()))
        } else if rp.flag() & RC_REPMBUF != 0 {
            NFSSTATS.srvcache_nonidemdonehits.fetch_add(1, Relaxed);
            let mut saved = rp.rc_reply.lend();
            match saved
                .as_deref_mut()
                .and_then(|m| m_copym(m, 0, M_COPYALL, M_WAIT))
            {
                Some(copy) => (RC_REPLY, Some(copy)),
                None => (RC_DROPIT, None),
            }
        } else {
            NFSSTATS.srvcache_idemdonehits.fetch_add(1, Relaxed);
            rp.rc_state.store(RC_INPROG, Relaxed);
            (RC_DOIT, None)
        };
        unlock_entry(rp);
        drop(held);
        return (ret, reply);
    }

    NFSSTATS.srvcache_misses.fetch_add(1, Relaxed);
    let rp = if c.numnfsrvcache.load(Relaxed) < c.desirednfsrvcache.load(Relaxed) {
        let Some(rp) = c.nfsrvcachepl.get(PR_WAITOK, NfsrvCache::new) else {
            return (RC_DOIT, None);
        };
        c.numnfsrvcache.fetch_add(1, Relaxed);
        rp.rc_flag.store(RC_LOCKED, Relaxed);
        rp
    } else {
        let Some(rp) = nfsrv_recycle::<G>() else {
            return (RC_DOIT, None);
        };
        nfsrv_cleanentry(&rp);
        rp.rc_flag.fetch_and(RC_LOCKED | RC_WANTED, Relaxed);
        rp
    };
    rp.rc_state.store(RC_INPROG, Relaxed);
    rp.rc_xid.store(nd.nd_retxid, Relaxed);
    let inet = nd
        .nam()
        .and_then(sockaddr_in_of)
        .filter(|sin| sin.sin_family == AF_INET);
    match inet {
        Some(sin) => {
            rp.rc_flag.fetch_or(RC_INETADDR, Relaxed);
            rp.rc_inetaddr.store(sin.sin_addr.s_addr, Relaxed);
        }
        None => {
            rp.rc_flag.fetch_or(RC_NAM, Relaxed);
            let copy = nd
                .nd_nam2
                .as_deref_mut()
                .and_then(|m| m_copym(m, 0, M_COPYALL, M_WAIT));
            drop(rp.rc_nam.replace(copy));
        }
    }
    // `nd_procnum < NFS_NPROCS` (23): a `u_int16_t`.
    rp.rc_proc.store(nd.nd_procnum as u16, Relaxed);
    // Linked on both lists at once, then unlocked; a waiter woken after the section.
    let wake = {
        let mut w = klock::witness();
        let t = c.table.get_mut(&mut w);
        let i = t.chain_of(nd.nd_retxid);
        t.nfsrvlruhead.push(rp);
        let Some(e) = t.nfsrvlruhead.last() else {
            kpanic(format_args!("nfsrv cache: an entry pushed is not last"));
        };
        if let Some(chain) = t.nfsrvhashtbl.as_deref_mut().and_then(|h| h.get_mut(i)) {
            chain.push_head(Member::counted(e));
        }
        e.rc_flag.fetch_and(!RC_LOCKED, Relaxed);
        let wanted = e.flag() & RC_WANTED != 0;
        if wanted {
            e.rc_flag.fetch_and(!RC_WANTED, Relaxed);
        }
        wanted.then(|| ptr::from_ref(e))
    };
    if let Some(chan) = wake {
        wakeup(chan);
    }
    (RC_DOIT, None)
}

/// The least recently used entry taken for a new request (the C's loop over `TAILQ_FIRST`:
/// a locked one waited for, then locked, off its chain and off the list), or `None` when the
/// list is empty.
fn nfsrv_recycle<G: ProcGlue>() -> Option<PoolBox<NfsrvCache>> {
    let c = &vfs::<G>().nfsrv.cache;
    loop {
        let chan = {
            let mut w = klock::witness();
            let t = c.table.get_mut(&mut w);
            let first = t.nfsrvlruhead.first()?;
            if first.flag() & RC_LOCKED == 0 {
                first.rc_flag.fetch_or(RC_LOCKED, Relaxed);
                let i = t.chain_of(first.rc_xid.load(Relaxed));
                if let Some(chain) = t.nfsrvhashtbl.as_deref_mut().and_then(|h| h.get_mut(i)) {
                    drop(chain.remove(first));
                }
                return t.nfsrvlruhead.pop();
            }
            first.rc_flag.fetch_or(RC_WANTED, Relaxed);
            ptr::from_ref(first)
        };
        let _ = tsleep_nsec(chan, PZERO - 1, "nfsrc", INFSLP);
    }
}

/// `nfsrv_updatecache(nd, repvalid, repmbuf)`: updates a request cache entry after the rpc
/// has been done: done, and, for a non-idempotent procedure with a valid reply, the reply
/// saved (version 2's status-only replies as the status).
pub fn nfsrv_updatecache<G: ProcGlue>(
    nd: &NfsrvDescript,
    repvalid: bool,
    repmbuf: Option<&mut Mbuf>,
) {
    if nd.nd_nam2.is_none() {
        return;
    }
    let Some(held) = nfsrv_lookupcache::<G>(nd) else {
        return;
    };
    let Some(rp) = held.get() else {
        kpanic(format_args!("nfsrv cache: a lookup's entry is gone"));
    };
    nfsrv_cleanentry(rp);
    rp.rc_state.store(RC_DONE, Relaxed);
    // If we have a valid reply update status and save the reply for non-idempotent rpc's.
    if repvalid && NONIDEMPOTENT.get(nd.nd_procnum).copied().unwrap_or(false) {
        let v2status = nd.nd_flag & ND_NFSV3 == 0
            && NFSV2_PROCID
                .get(nd.nd_procnum)
                .and_then(|&p| NFSV2_REPSTAT.get(p))
                .copied()
                .unwrap_or(false);
        if v2status {
            rp.rc_status.store(nd.nd_repstat, Relaxed);
            rp.rc_flag.fetch_or(RC_REPSTATUS, Relaxed);
        } else if let Some(copy) = repmbuf.and_then(|m| m_copym(m, 0, M_COPYALL, M_WAIT)) {
            drop(rp.rc_reply.replace(Some(copy)));
            rp.rc_flag.fetch_or(RC_REPMBUF, Relaxed);
        }
    }
    unlock_entry(rp);
    drop(held);
}

/// `nfsrv_cleancache()`: clean out the cache; called when the last nfsd terminates. The
/// entries are freed after the lists' section.
pub fn nfsrv_cleancache<G: ProcGlue>() {
    let c = &vfs::<G>().nfsrv.cache;
    let mut dead = OwnedList::<RcLru>::new();
    {
        let mut w = klock::witness();
        let t = c.table.get_mut(&mut w);
        while let Some(first) = t.nfsrvlruhead.first() {
            let xid = first.rc_xid.load(Relaxed);
            let i = t.chain_of(xid);
            if let Some(chain) = t.nfsrvhashtbl.as_deref_mut().and_then(|h| h.get_mut(i)) {
                drop(chain.remove(first));
            }
            let Some(b) = t.nfsrvlruhead.pop() else { break };
            dead.push(b);
        }
    }
    while let Some(rp) = dead.pop() {
        nfsrv_cleanentry(&rp);
        drop(rp);
    }
    c.numnfsrvcache.store(0, Relaxed);
}

/// `nfsrv_lookupcache(nd)`: the entry of the request (its transaction id, procedure and
/// client), locked and held; a locked entry is waited for ("nfsrc") and the chain walked
/// again. `None` when there is none.
pub fn nfsrv_lookupcache<G: ProcGlue>(nd: &NfsrvDescript) -> Option<SecondaryRef<RcLru>> {
    let c = &vfs::<G>().nfsrv.cache;
    let nam = nd.nam()?;
    loop {
        let chan = {
            let r = klock::reader();
            let t = c.table.get(&r);
            let chain = t.chain(nd.nd_retxid)?;
            let rp = chain.iter().find(|rp| {
                nd.nd_retxid == rp.rc_xid.load(Relaxed)
                    && nd.nd_procnum == usize::from(rp.rc_proc.load(Relaxed))
                    && netaddr_match(netfamily(rp), rp.rc_inetaddr.load(Relaxed), nam)
            })?;
            if rp.flag() & RC_LOCKED == 0 {
                rp.rc_flag.fetch_or(RC_LOCKED, Relaxed);
                let mut held = SecondaryRef::new();
                held.set(Some(rp));
                return Some(held);
            }
            rp.rc_flag.fetch_or(RC_WANTED, Relaxed);
            ptr::from_ref(rp)
        };
        let _ = tsleep_nsec(chan, PZERO - 1, "nfsrc", INFSLP);
    }
}

/// The host tests' reset of the cache of a test glue (testing.md): the lists forgotten
/// with what they hold (the memory behind them is gone with the last test's), the counts
/// back to the C's start.
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn nfsrv_cache_test_reset<G: ProcGlue>() {
    let c = &vfs::<G>().nfsrv.cache;
    {
        let mut w = klock::witness();
        core::mem::forget(core::mem::replace(
            c.table.get_mut(&mut w),
            NfsrvCacheTable::new(),
        ));
    }
    c.numnfsrvcache.store(0, Relaxed);
    c.desirednfsrvcache.store(NFSRVCACHESIZ as i64, Relaxed);
}

const _: () = {
    assert!(NONIDEMPOTENT.len() == NFS_NPROCS && NFSV2_REPSTAT.len() == NFS_NPROCS);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the duplicate request cache, LZ's eleven re-expressed: the C's two
    // tables, a retransmission in progress dropped, an idempotent request done again, a
    // non-idempotent reply saved and copied, version 2's status-only reply, the key (xid,
    // procedure, client), reliable transports not cached, the LRU recycle and its order, a
    // recycled reply freed, a non-inet address kept as a copy, the cache cleaned; then an
    // entry slept on by a second request (RC_LOCKED, RC_WANTED, "nfsrc").

    use std::vec::Vec;

    use super::*;
    use crate::fs::nfs::nfsm_subs::{test_bytes, test_chain};
    use crate::fs::nfs::nfsproto::{
        NFSPROC_GETATTR, NFSPROC_REMOVE, NFSPROC_SETATTR, NFSPROC_WRITE,
    };
    use crate::fs::testfs::TestGlue;
    use crate::net::mbuf::{MT_SONAME, m_devget};
    use hal::sys::errno::Errno;

    type G = TestGlue;

    /// Memory, mbufs, the kernel lock, and an empty cache of at most `desired` entries.
    fn fresh(desired: i64) -> std::sync::MutexGuard<'static, ()> {
        let g = crate::net::mbuf::tests::setup();
        hal::klock::kernel_lock();
        nfsrv_cache_test_reset::<G>();
        vfs::<G>()
            .nfsrv
            .cache
            .desirednfsrvcache
            .store(desired, Relaxed);
        nfsrv_initcache::<G>();
        g
    }

    /// The test's end: the cache emptied with what its entries hold (the saved replies and
    /// address copies are mbufs, counted for the whole process: testing.md), the lock given
    /// back.
    fn done() {
        nfsrv_cleancache::<G>();
        hal::klock::kernel_unlock();
    }

    /// An `MT_SONAME` mbuf of `family` holding the address `a`, port `port`.
    fn nam(family: u8, a: [u8; 4], port: u16) -> MbufBox {
        let mut sin = [0u8; 16];
        sin[0] = 16;
        sin[1] = family;
        sin[2..4].copy_from_slice(&port.to_be_bytes());
        sin[4..8].copy_from_slice(&a);
        let m = m_devget(&sin, 0).expect("an mbuf");
        m.m_type().set(MT_SONAME as i16);
        m
    }

    /// A datagram request: transaction id, procedure, client.
    fn req(xid: u32, procnum: usize, client: [u8; 4], v3: bool) -> NfsrvDescript {
        NfsrvDescript {
            nd_retxid: xid,
            nd_procnum: procnum,
            nd_nam2: Some(nam(AF_INET, client, 700)),
            nd_flag: if v3 { ND_NFSV3 } else { 0 },
            ..NfsrvDescript::default()
        }
    }

    fn sock() -> NfssvcSock<G> {
        NfssvcSock::new(None)
    }

    fn num() -> i64 {
        vfs::<G>().nfsrv.cache.numnfsrvcache.load(Relaxed)
    }

    /// The xids on the LRU list, least recently used first.
    fn lru_xids() -> Vec<u32> {
        let r = hal::klock::reader();
        vfs::<G>()
            .nfsrv
            .cache
            .table
            .get(&r)
            .lru()
            .iter()
            .map(|rp| rp.rc_xid.load(Relaxed))
            .collect()
    }

    /// `f` on the entry of `nd`, held and then unlocked.
    fn with_entry<R>(nd: &NfsrvDescript, f: impl FnOnce(&NfsrvCache) -> R) -> Option<R> {
        let held = nfsrv_lookupcache::<G>(nd)?;
        let rp = held.get()?;
        let r = f(rp);
        unlock_entry(rp);
        Some(r)
    }

    #[test]
    fn the_tables_match_the_c_arrays() {
        let non: Vec<usize> = (0..NFS_NPROCS).filter(|&i| NONIDEMPOTENT[i]).collect();
        assert_eq!(non, [2, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
        let st: Vec<usize> = (0..NFS_NPROCS).filter(|&i| NFSV2_REPSTAT[i]).collect();
        assert_eq!(st, [10, 11, 12, 13, 15]);
    }

    #[test]
    fn a_new_request_is_cached_and_a_retransmission_in_progress_is_dropped() {
        let _g = fresh(8);
        let s = sock();
        let mut nd = req(0x1234, NFSPROC_GETATTR, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        assert_eq!(num(), 1);
        assert_eq!(lru_xids(), [0x1234]);
        let before = NFSSTATS.srvcache_inproghits.load(Relaxed);
        let mut again = req(0x1234, NFSPROC_GETATTR, [10, 0, 2, 9], true);
        let (ret, reply) = nfsrv_getcache(&mut again, &s);
        assert_eq!(ret, RC_DROPIT);
        assert!(reply.is_none());
        assert_eq!(NFSSTATS.srvcache_inproghits.load(Relaxed), before + 1);
        assert_eq!(num(), 1);
        done();
    }

    #[test]
    fn an_idempotent_request_is_done_again_after_it_completed() {
        let _g = fresh(8);
        let s = sock();
        let mut nd = req(7, NFSPROC_GETATTR, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        let mut reply = test_chain(&[&[1, 2, 3, 4]]);
        nfsrv_updatecache::<G>(&nd, true, Some(&mut reply));
        assert_eq!(
            with_entry(&nd, |rp| (rp.state(), rp.flag() & RC_REPMBUF)),
            Some((RC_DONE, 0))
        );
        let mut again = req(7, NFSPROC_GETATTR, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut again, &s).0, RC_DOIT);
        assert_eq!(with_entry(&nd, NfsrvCache::state), Some(RC_INPROG));
        done();
    }

    #[test]
    fn a_non_idempotent_reply_is_saved_and_copied_for_a_retransmission() {
        let _g = fresh(8);
        let s = sock();
        let mut nd = req(9, NFSPROC_WRITE, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        let mut reply = test_chain(&[&[5, 6], &[7, 8]]);
        nfsrv_updatecache::<G>(&nd, true, Some(&mut reply));
        let mut again = req(9, NFSPROC_WRITE, [10, 0, 2, 9], true);
        let (ret, copy) = nfsrv_getcache(&mut again, &s);
        assert_eq!(ret, RC_REPLY);
        assert_eq!(test_bytes(&copy.expect("the saved reply")), [5, 6, 7, 8]);
        // An invalid reply is not saved: done again.
        let mut nd = req(10, NFSPROC_SETATTR, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        nfsrv_updatecache::<G>(&nd, false, None);
        let mut again = req(10, NFSPROC_SETATTR, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut again, &s).0, RC_DOIT);
        done();
    }

    #[test]
    fn a_version_2_status_only_reply_saves_the_status() {
        let _g = fresh(8);
        let s = sock();
        // v3 REMOVE (12) is v2 REMOVE (10): its reply is the status only.
        let mut nd = req(11, NFSPROC_REMOVE, [10, 0, 2, 9], false);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        nd.nd_repstat = Errno::ENOENT.as_i32();
        nfsrv_updatecache::<G>(&nd, true, None);
        assert_eq!(
            with_entry(&nd, |rp| (
                rp.flag() & (RC_REPSTATUS | RC_REPMBUF),
                rp.rc_status.load(Relaxed)
            )),
            Some((RC_REPSTATUS, Errno::ENOENT.as_i32()))
        );
        let mut again = req(11, NFSPROC_REMOVE, [10, 0, 2, 9], false);
        let (ret, reply) = nfsrv_getcache(&mut again, &s);
        assert_eq!(ret, RC_REPLY);
        // The reply header with ENOENT mapped (NFSERR_NOENT, 2).
        let b = test_bytes(&reply.expect("a header"));
        assert_eq!(&b[b.len() - 4..], &2u32.to_be_bytes());
        done();
    }

    #[test]
    fn requests_differ_by_xid_procedure_and_client() {
        let _g = fresh(8);
        let s = sock();
        let mut nd = req(5, NFSPROC_WRITE, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        for mut other in [
            req(6, NFSPROC_WRITE, [10, 0, 2, 9], true),
            req(5, NFSPROC_SETATTR, [10, 0, 2, 9], true),
            req(5, NFSPROC_WRITE, [10, 0, 2, 8], true),
        ] {
            assert_eq!(nfsrv_getcache(&mut other, &s).0, RC_DOIT);
        }
        assert_eq!(num(), 4);
        done();
    }

    #[test]
    fn requests_on_a_reliable_transport_are_not_cached() {
        let _g = fresh(8);
        let s = sock();
        let mut nd = req(1, NFSPROC_WRITE, [10, 0, 2, 9], true);
        nd.nd_nam = nd.nd_nam2.take();
        let (ret, reply) = nfsrv_getcache(&mut nd, &s);
        assert_eq!(ret, RC_DOIT);
        assert!(reply.is_none());
        nfsrv_updatecache::<G>(&nd, true, None);
        assert_eq!(num(), 0);
        assert!(lru_xids().is_empty());
        done();
    }

    #[test]
    fn a_full_cache_recycles_the_least_recently_used_entry() {
        let _g = fresh(3);
        let s = sock();
        let mk = |xid| req(xid, NFSPROC_WRITE, [10, 0, 2, 9], true);
        let (mut a, mut b, mut c, mut d) = (mk(1), mk(2), mk(3), mk(4));
        for nd in [&mut a, &mut b, &mut c] {
            assert_eq!(nfsrv_getcache(nd, &s).0, RC_DOIT);
        }
        let mut reply = test_chain(&[&[1, 2]]);
        nfsrv_updatecache::<G>(&a, true, Some(&mut reply));
        assert_eq!(lru_xids(), [1, 2, 3]);
        // A hit moves the entry to the end of the LRU list.
        assert_eq!(nfsrv_getcache(&mut mk(1), &s).0, RC_REPLY);
        assert_eq!(lru_xids(), [2, 3, 1]);
        // The fourth request recycles xid 2 (the least recently used): same count, new key.
        assert_eq!(nfsrv_getcache(&mut d, &s).0, RC_DOIT);
        assert_eq!(num(), 3);
        assert_eq!(lru_xids(), [3, 1, 4]);
        assert!(with_entry(&b, |_| ()).is_none());
        assert_eq!(nfsrv_getcache(&mut b, &s).0, RC_DOIT);
        assert_eq!(lru_xids(), [1, 4, 2]);
        assert!(with_entry(&c, |_| ()).is_none());
        // Xid 1's entry kept its saved reply meanwhile.
        assert_eq!(
            with_entry(&a, |rp| rp.flag() & RC_REPMBUF),
            Some(RC_REPMBUF)
        );
        done();
    }

    #[test]
    fn recycling_an_entry_with_a_reply_frees_it() {
        let _g = fresh(1);
        let s = sock();
        let mut a = req(1, NFSPROC_WRITE, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut a, &s).0, RC_DOIT);
        let mut reply = test_chain(&[&[1, 2]]);
        nfsrv_updatecache::<G>(&a, true, Some(&mut reply));
        let mut b = req(2, NFSPROC_WRITE, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut b, &s).0, RC_DOIT);
        assert_eq!(num(), 1);
        assert_eq!(
            with_entry(&b, |rp| (
                rp.flag() & (RC_REPMBUF | RC_REPSTATUS),
                rp.rc_reply.take().is_none()
            )),
            Some((0, true))
        );
        done();
    }

    #[test]
    fn an_address_that_is_not_inet_is_kept_as_a_copy_and_never_matches() {
        let _g = fresh(4);
        let s = sock();
        let mut nd = req(3, NFSPROC_WRITE, [10, 0, 2, 9], true);
        nd.nd_nam2 = Some(nam(0, [1, 2, 3, 4], 9));
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        {
            let r = hal::klock::reader();
            let t = vfs::<G>().nfsrv.cache.table.get(&r);
            let rp = t.lru().first().expect("an entry");
            assert_eq!(rp.flag() & (RC_NAM | RC_INETADDR), RC_NAM);
            let copy = rp.rc_nam.take().expect("a copy");
            assert_eq!(copy.data()[4..8], [1, 2, 3, 4]);
            drop(rp.rc_nam.replace(Some(copy)));
        }
        // NETFAMILY of such an entry is AF_UNSPEC: a retransmission is not found.
        let mut again = req(3, NFSPROC_WRITE, [10, 0, 2, 9], true);
        again.nd_nam2 = Some(nam(0, [1, 2, 3, 4], 9));
        assert_eq!(nfsrv_getcache(&mut again, &s).0, RC_DOIT);
        assert_eq!(num(), 2);
        // Cleaned twice, the copy is freed once (the C frees it again).
        {
            let r = hal::klock::reader();
            let t = vfs::<G>().nfsrv.cache.table.get(&r);
            let rp = t.lru().first().expect("an entry");
            nfsrv_cleanentry(rp);
            nfsrv_cleanentry(rp);
            assert!(rp.rc_nam.take().is_none());
        }
        done();
    }

    #[test]
    fn cleancache_empties_everything() {
        let _g = fresh(8);
        let s = sock();
        for xid in 1..=5 {
            let mut nd = req(xid, NFSPROC_WRITE, [10, 0, 2, 9], true);
            assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
            let mut reply = test_chain(&[&[9]]);
            nfsrv_updatecache::<G>(&nd, true, Some(&mut reply));
        }
        assert_eq!(num(), 5);
        nfsrv_cleancache::<G>();
        assert_eq!(num(), 0);
        assert!(lru_xids().is_empty());
        assert!(with_entry(&req(3, NFSPROC_WRITE, [10, 0, 2, 9], true), |_| ()).is_none());
        done();
    }

    #[test]
    fn a_locked_entry_is_slept_on_and_found_again() {
        let _g = fresh(8);
        let s = sock();
        let mut nd = req(21, NFSPROC_WRITE, [10, 0, 2, 9], true);
        assert_eq!(nfsrv_getcache(&mut nd, &s).0, RC_DOIT);
        // Held by a first lookup: a second one marks it wanted and sleeps; the host's sleep
        // returns at once, so the second walk finds it still locked until it is unlocked.
        let held = nfsrv_lookupcache::<G>(&nd).expect("the entry");
        let rp = held.get().expect("held");
        assert_eq!(rp.flag() & RC_LOCKED, RC_LOCKED);
        let chain_has_it = {
            let r = hal::klock::reader();
            let t = vfs::<G>().nfsrv.cache.table.get(&r);
            t.chain(21)
                .is_some_and(|c| c.iter().any(|e| ptr::eq(e, rp)))
        };
        assert!(chain_has_it);
        rp.rc_flag.fetch_or(RC_WANTED, Relaxed);
        unlock_entry(rp);
        assert_eq!(rp.flag() & (RC_LOCKED | RC_WANTED), 0);
        drop(held);
        // Unlocked, the next lookup takes it at once.
        assert!(with_entry(&nd, |_| ()).is_some());
        done();
    }
}
/* </TESTS> */
