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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! TCP compressed state engine: `netinet/tcp_input.c`'s SYN cache, which holds the state of
//! embryonic connections in `SYN_RECEIVED` (part 1: the sets, the removal, the insertion, the
//! lookup, the retransmit timer, the SYN-ACK, and the cleanup, reset and unreachable paths;
//! `syn_cache_add` and `syn_cache_get` follow at N6.6a4 with `tcp_input`).
//!
//! Upstream: sys/netinet/tcp_input.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_input.rs@3c62ede68802
//!
//! Two hash tables of entries, the active one and the one being drained: an entry goes into
//! the active set; when that set has been used `tcp_syn_use_limit` times and the other is
//! empty, they exchange their roles, and an empty set is reseeded (and resized) before its
//! next use. A full bucket or a full cache tosses its oldest entry. Each entry is on its
//! bucket and on its listener's list (`t_sc`), and its timer retransmits the SYN-ACK with a
//! fixed backoff until the keep alive time runs out.
//!
//! Locks used to protect global data and struct members: \[a\] atomic operations, \[N\] net
//! lock, \[S\] `syn_cache_mtx`, \[s\] the listen socket's `so_lock`.
//!
//! ## Deviations
//! - The functions take the addresses as `SynCacheSa` (`union syn_cache_sa`'s bytes) rather
//!   than `struct sockaddr *`, compared over `sa_len` bytes as the C's `bcmp`, bounded by the
//!   union's size (LZ's bound: a longer `sa_len` would read past the C's union).
//!   `syn_cache_lookup` does not return the bucket through `headp` (no caller reads it; LZ's
//!   choice) and returns a counted handle (the C's pointer, read under the mutex: here the
//!   mutex is let go by its caller with a handle in hand).
//! - `syn_cache_respond` returns `Result<(), Errno>` (the C's 0 or errno); without a listener
//!   it uses `ip6_defhlim`, what the C's `in6_selecthlim(NULL)` returns. Its `DIAGNOSTIC`
//!   check of `max_linkhdr` is the feature `diagnostic`'s.
//! - `syn_cache_respond` builds its options in a 40-byte buffer, the TCP header's option room:
//!   MSS 4, window scale 4, the signature 20 and the timestamps 12 fill it, so a signed entry
//!   with `SCF_SACK_PERMIT` (4 more) would not fit. The C relies on the same exclusion:
//!   `tcp_ctloutput` refuses `TCP_SACK_ENABLE` on a signed socket and `TCP_MD5SIG` on one with
//!   SACK (tcp_usrreq.c:396-398, :418-420), and `syn_cache_add` copies only what the
//!   listener allows; it is checked here (`kassert!`), and N6.6a5's `tcp_ctloutput` must keep
//!   both refusals.
//! - `syn_cache_unreach` takes the addresses as `SynCacheSa`s (LZ: raw pointers copied in by
//!   an `unsafe fn`); its caller (`tcp_ctlinput`, N6.6a3) copies them.
//!
//! ## Redesign
//! - An entry is a counted "syncache" item (`PoolArc`, the C's `sc_refcnt`) whose `[S]`
//!   members are a cell of the SYN cache's mutex and `[s]` members a cell of the listener's
//!   `so_lock` (`tcp_var::SynCache`). The counts: the two lists hold one handle, forgotten at
//!   [`syn_cache_insert`] and adopted back by [`syn_cache_rm`]; the timer holds one from the
//!   insertion (`Timeout::set_shared`), given back by `clear()` in `syn_cache_rm`; the
//!   listener is a counted handle in the `[S]` cell. `syn_cache_rm` answers what it took off
//!   ([`ScRemoved`]: the lists' handle and the listener's) for its caller to drop after the
//!   mutex, so the last `syn_cache_put` (the free of the IP options and the route) never runs
//!   under it where the C's callers let go first; `syn_cache_put` is that drop.
//! - The sets are the data of the glue's `Mutexed` (`tcp_var::SynCaches`); a call that walks
//!   or changes a bucket takes the set's `MArray` of buckets out of the guard and puts it back
//!   (the lists take the guard as their witness while the array is borrowed; the pcb table's
//!   shape): two word moves a use. The entry records its set and bucket by index where the C
//!   kept pointers (`sc_set`, `sc_buckethead`; LZ a raw pointer).
//! - `syn_cache_insert` sets the entry's timer (`Timeout::set_shared`, the wheel's lock) under
//!   `syn_cache_mtx`, where the C's `syn_cache_add` sets it bare before taking the mutex:
//!   N6.6a4's `syn_cache_add` may move the set before the mutex, leaving the insertion its
//!   `add_msec` alone.
//! - The listener's list (`t_sc`) is a `PinRing`, so an entry leaves it without naming the
//!   control block, as the C's `LIST_REMOVE`; a bucket is a `PinList` (`TAILQ`).

use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::malloc::{M_NOWAIT, M_WAITOK, MArray};
use hal::mutex::MutexedGuard;
use hal::netlock::{NETLOCK, NetHeld};
use hal::pool::{AnyArc, PR_NOWAIT, PoolArc};
use hal::rwlock::WitnessPart;
use hal::sys::endian::{htonl, htons, ntohl};
use hal::sys::errno::Errno;
use hal::sys::mbuf::MT_DATA;
use hal::sys::netinet_in::IPPROTO_TCP;
use hal::sys::netinet_ip::Ip;
use hal::sys::netinet_ip6::{IPV6_VERSION, IPV6_VERSION_MASK, Ip6Hdr};
use hal::sys::netinet_tcp::{
    TCPOLEN_SIGLEN, TCPOLEN_SIGNATURE, TCPOLEN_TSTAMP_APPA, TCPOLEN_WINDOW, TCPOPT_EOL,
    TCPOPT_MAXSEG, TCPOPT_NOP, TCPOPT_SACK_PERMIT_HDR, TCPOPT_SIGNATURE, TCPOPT_TSTAMP_HDR,
    TCPOPT_WINDOW, TH_ACK, TH_ECE, TH_SYN, Tcphdr,
};
use hal::sys::socket::{AF_INET, AF_INET6};

use crate::assert::{kassert, kpanic};
use crate::net::domain::net;
use crate::net::ipsec_hooks::{SOCKADDR_UNION_LEN, ipsec};
use crate::net::mbuf::{
    M_DONTWAIT, M_TCP_CSUM_OUT, MAX_LINKHDR, MCLBYTES, MHLEN, Mbuf, MbufBox, m_clget, m_gethdr,
};
use crate::net::rtable::rtable_l2;
use crate::netinet::in_pcb::{Inpcb, InpcbRef, in_pcbsolock};
use crate::netinet::ip_output::ip_output;
use crate::netinet::ip_var::{IP_DEFTTL, IP_MTUDISC, ip_mtudisc};
use crate::netinet::tcp_seq::{seq_gt, seq_lt};
use crate::netinet::tcp_subr::{TCP_DO_ECN, put_hdr, tcp_signature};
use crate::netinet::tcp_timer::{
    TCP_BACKOFF, TCP_KEEPINIT, TCP_MAXRXTSHIFT, TCPTV_MIN, TCPTV_REXMTMAX, TCPTV_SRTTDFLT,
    tcpt_rangeset,
};
use crate::netinet::tcp_var::{
    SCF_ECN_PERMIT, SCF_SACK_PERMIT, SCF_SIGNATURE, SCF_TIMESTAMP, SCF_UNREACH, ScListener, ScTpq,
    SynCache, SynCacheHead, SynCacheInit, SynCacheSa, SynCaches, TCP_SYN_BUCKET_SIZE,
    TCP_SYN_HASH_SIZE, TcpstatCounters, Tp, tcp_now, tcpstat_inc,
};
use crate::netinet6::in6_src::in6_selecthlim;
use crate::netinet6::ip6_output::ip6_output;
use crate::netinet6::ip6_var::IP6_DEFHLIM;
use crate::proc::ProcGlue;
use crate::timeout::{KCLOCK_NONE, TIMEOUT_MPSAFE, TIMEOUT_PROC};

/// `M_SYNCACHE`: the SYN cache's bucket arrays (`malloc.h`).
const M_SYNCACHE: i32 = 139;

/// The guard of `syn_cache_mtx`: the witness of the `[S]` cells and the lists.
pub type ScGuard<'a, G> = MutexedGuard<'a, SynCaches<G>>;

/// What [`syn_cache_rm`] took off an entry: the lists' handle and the listener's, for the
/// caller to drop once it let `syn_cache_mtx` go (`syn_cache_put`'s last drop frees the
/// entry, its IP options and its route).
#[must_use = "the handles are dropped after syn_cache_mtx is let go"]
pub struct ScRemoved<G: ProcGlue> {
    /// The lists' handle, adopted back.
    list: Option<PoolArc<SynCache<G>>>,
    /// `sc_inplisten`, taken out (the C's `in_pcbunref`).
    listener: Option<InpcbRef<G>>,
}

impl<G: ProcGlue> Drop for ScRemoved<G> {
    /// `in_pcbunref(sc->sc_inplisten)`, then the lists' reference (`syn_cache_put`).
    fn drop(&mut self) {
        drop(self.listener.take());
        drop(self.list.take());
    }
}

/// \[a\] `tcp_syn_cache_limit`: global entry limit.
pub static TCP_SYN_CACHE_LIMIT: AtomicI32 = AtomicI32::new(TCP_SYN_HASH_SIZE * TCP_SYN_BUCKET_SIZE);
/// \[a\] `tcp_syn_bucket_limit`: per bucket limit.
pub static TCP_SYN_BUCKET_LIMIT: AtomicI32 = AtomicI32::new(3 * TCP_SYN_BUCKET_SIZE);

/// `sa_family` of a `union syn_cache_sa`.
#[inline]
fn sa_family(sa: &SynCacheSa) -> u8 {
    sa[1]
}

/// `sa_len` of a `union syn_cache_sa`, bounded by the union's size (see the deviations).
#[inline]
fn sa_len(sa: &SynCacheSa) -> usize {
    usize::from(sa[0]).min(SOCKADDR_UNION_LEN)
}

/// The port of a `sockaddr_in` or `sockaddr_in6` (the same offset), network order.
#[inline]
fn sa_port(sa: &SynCacheSa) -> u16 {
    u16::from_ne_bytes([sa[2], sa[3]])
}

/// The 32-bit word `i` of the bytes at `off` (`sin_addr.s_addr`, `s6_addr32[i]`).
#[inline]
fn sa_word(sa: &SynCacheSa, off: usize) -> u32 {
    u32::from_ne_bytes([sa[off], sa[off + 1], sa[off + 2], sa[off + 3]])
}

/// The 16 bytes of `sin6_addr`.
#[inline]
fn sa_addr6(sa: &SynCacheSa) -> [u8; 16] {
    let mut a = [0; 16];
    a.copy_from_slice(&sa[8..24]);
    a
}

/// `syn_cache_hash`: the bucket hash of a connection from `src` to `dst` under the set's seed
/// `rand`.
#[inline]
fn syn_cache_hash(src: &SynCacheSa, dst: &SynCacheSa, rand: &[u32; 5]) -> u32 {
    let src_port = u32::from(sa_port(src));
    let dst_port = u32::from(sa_port(dst));
    match sa_family(src) {
        AF_INET => (((dst_port << 16).wrapping_add(src_port)) ^ rand[4])
            .wrapping_mul(sa_word(src, 4) ^ rand[0]),
        AF_INET6 => (((dst_port << 16).wrapping_add(src_port)) ^ rand[4])
            .wrapping_mul(sa_word(src, 8) ^ rand[0])
            .wrapping_mul(sa_word(src, 12) ^ rand[1])
            .wrapping_mul(sa_word(src, 16) ^ rand[2])
            .wrapping_mul(sa_word(src, 20) ^ rand[3]),
        family => kpanic(format_args!("syn_cache_hash: unhandled af {family}")),
    }
}

/// `mallocarray(n, sizeof(struct syn_cache_head), M_SYNCACHE, flags|M_ZERO)` and the
/// `TAILQ_INIT` of every bucket.
fn syn_cache_buckets<G: ProcGlue>(n: i32, flags: i32) -> Option<MArray<SynCacheHead<G>>> {
    MArray::new(M_SYNCACHE, flags, usize::try_from(n).ok()?, |_| {
        SynCacheHead::new()
    })
}

/// Runs `f` with the bucket array of set `set` out of the guard, so the buckets' lists can
/// take the guard as their witness while the array is borrowed; puts it back. `panic(9)`
/// before `syn_cache_init`.
fn with_buckets<'a, G: ProcGlue, R>(
    g: &mut ScGuard<'a, G>,
    set: usize,
    f: impl FnOnce(&mut MArray<SynCacheHead<G>>, &mut ScGuard<'a, G>) -> R,
) -> R {
    let Some(mut b) = g.sets[set].scs_buckethead.take() else {
        kpanic(format_args!("syn cache: set {set} has no buckets"));
    };
    let r = f(&mut b, g);
    g.sets[set].scs_buckethead = Some(b);
    r
}

/// `syn_cache_rm`: unlinks `sc` from its bucket and its listener's list, stops its timer and
/// gives its handles back ([`ScRemoved`]: the caller drops them after the mutex).
pub fn syn_cache_rm<G: ProcGlue>(g: &mut ScGuard<'_, G>, sc: &SynCache<G>) -> ScRemoved<G> {
    // MUTEX_ASSERT_LOCKED(&syn_cache_mtx): the guard is the mutex's (the cells check it).
    let (set, bucket) = {
        let s = sc.sc_s.get(&*g);
        (usize::from(s.sc_set), s.sc_bucket as usize)
    };
    with_buckets(g, set, |b, g| {
        b[bucket].sch_bucket.remove(g, sc);
        b[bucket].sch_length -= 1;
    });
    let listener = sc.sc_s.get_mut(g).sc_inplisten.take();
    hal::pinlist::PinRing::<ScTpq<G>>::remove(g, sc);
    // The lists' handle (both memberships hold one), and the timer's: deleted, its argument
    // given back (the C's refcnt_rele of a pending timer; a running one has its own clone).
    let list = PoolArc::adopt(sc);
    let _ = sc.sc_timer.del();
    sc.sc_timer.clear();
    g.sets[set].scs_count -= 1;
    ScRemoved { list, listener }
}

/// `mtx_enter(&syn_cache_mtx)`: the glue's SYN cache, locked (`IPL_SOFTNET`).
#[inline]
pub fn syn_cache_lock<G: ProcGlue>() -> ScGuard<'static, G> {
    net::<G>().tcp.syn_cache.lock()
}

/// `pool_get(&syn_cache_pool, PR_NOWAIT|PR_ZERO)` and `refcnt_init` of `syn_cache_add`: a new
/// entry of the listener `inp` with the `[I]` members `init` and the `[s]` ones `l`, its
/// listener counted (`sc_inplisten = in_pcbref(tp->t_inpcb)`); `None` when the pool is empty
/// (or `inp` no pool item).
pub fn syn_cache_alloc<G: ProcGlue>(
    inp: &Inpcb<G>,
    init: SynCacheInit,
    l: ScListener,
) -> Option<PoolArc<SynCache<G>>> {
    let t = &net::<G>().tcp;
    let listen = inp.socket().so_lock.lock().id();
    let listener = PoolArc::from_ref(inp)?;
    t.syn_cache_pool.get_homed(PR_NOWAIT, || {
        SynCache::new(t.syn_cache.id(), listen, init, l, Some(listener))
    })
}

/// `syn_cache_init`: the two sets' buckets and the "syncache" pool (at boot, from `tcp_init`).
/// A second call (a test glue's) leaves them as they are.
pub fn syn_cache_init<G: ProcGlue>() {
    let t = &net::<G>().tcp;
    let size = t.syn_cache.lock().tcp_syn_hash_size;
    // Initialize the hash buckets: allocated (M_WAITOK) before the mutex is taken.
    let mut arrays = [
        syn_cache_buckets::<G>(size, M_WAITOK),
        syn_cache_buckets::<G>(size, M_WAITOK),
    ];
    let mut g = t.syn_cache.lock();
    for (set, a) in g.sets.iter_mut().zip(&mut arrays) {
        if set.scs_buckethead.is_none() {
            set.scs_buckethead = a.take();
            set.scs_size = size;
        }
    }
    drop(g);
    // A second call's arrays go back, unused, outside the mutex.
    drop(arrays);

    // Initialize the syn cache pool.
    let _ = t
        .syn_cache_pool
        .init_recorded(hal::intr::IPL_SOFTNET, 0, "syncache");
}

/// `syn_cache_insert`: puts the new entry `sc` (its `[I]` members and its listener set) into
/// the active set and onto the list of the listener `tp`, its timer started: the C's
/// sequence (a reseed of an empty, used-up set, the bucket and the cache limits that toss an
/// oldest entry, the set's swap). The tossed entries' handles come back in `tossed` for the
/// caller to drop after the mutex.
pub fn syn_cache_insert<G: ProcGlue>(
    g: &mut ScGuard<'_, G>,
    sc: &PoolArc<SynCache<G>>,
    tp: &Tp<'_, '_, G>,
    tossed: &mut Option<ScRemoved<G>>,
) {
    // NET_ASSERT_LOCKED(): `tp` carries its proof (`tp.net`).
    let active = g.active;

    // If there are no entries in the hash table, reinitialize the hash secrets. To avoid
    // useless cache swaps and reinitialization, use it until the limit is reached. An empty
    // cache is also the opportunity to resize the hash.
    if g.sets[active].scs_count == 0 && g.sets[active].scs_use <= 0 {
        g.sets[active].scs_use = i64::from(g.tcp_syn_use_limit);
        let hash_size = g.tcp_syn_hash_size;
        if g.sets[active].scs_size != hash_size {
            match syn_cache_buckets::<G>(hash_size, M_NOWAIT) {
                // Try again next time.
                None => g.sets[active].scs_use = 0,
                Some(scp) => {
                    // The old array's buckets are empty (the set is): its free is the
                    // C's free(9).
                    drop(g.sets[active].scs_buckethead.replace(scp));
                    g.sets[active].scs_size = hash_size;
                }
            }
        }
        let mut seed = [0u8; 20];
        hal::random::arc4random_buf(&mut seed);
        for (w, b) in g.sets[active]
            .scs_random
            .iter_mut()
            .zip(seed.chunks_exact(4))
        {
            *w = u32::from_ne_bytes([b[0], b[1], b[2], b[3]]);
        }
        // The seed's copy on the stack wiped: the set's words are its storage.
        libkern::explicit_bzero(&mut seed);
        tcpstat_inc(TcpstatCounters::TcpsScSeedrandom);
    }

    let hash = syn_cache_hash(&sc.sc_i.sc_src, &sc.sc_i.sc_dst, &g.sets[active].scs_random);
    // The index is below `scs_size`, an `int`: it fits the entry's `u32`.
    let bucket32 = hash % g.sets[active].scs_size as u32;
    let bucket = bucket32 as usize;
    {
        let s = sc.sc_s.get_mut(&mut *g);
        s.sc_hash = hash;
        s.sc_bucket = bucket32;
    }

    // Make sure that we don't overflow the per-bucket limit or the total cache size limit.
    let length = with_buckets(g, active, |b, _| b[bucket].sch_length);
    if i32::from(length) >= TCP_SYN_BUCKET_LIMIT.load(Ordering::Relaxed) {
        tcpstat_inc(TcpstatCounters::TcpsScBucketoverflow);
        // Someone might attack our bucket hash function. Reseed with random as soon as the
        // passive syn cache gets empty.
        g.sets[active].scs_use = 0;
        // The bucket is full. Toss the oldest element in the bucket. This will be the first
        // entry in the bucket.
        let sc2 = with_buckets(g, active, |b, g| {
            b[bucket].sch_bucket.first(&*g).and_then(PoolArc::from_ref)
        });
        // This should never happen; we should always find an entry in our bucket.
        let Some(sc2) = sc2 else {
            kpanic(format_args!("syn_cache_insert: bucketoverflow: impossible"));
        };
        *tossed = Some(syn_cache_rm(g, &sc2));
    } else if g.sets[active].scs_count >= TCP_SYN_CACHE_LIMIT.load(Ordering::Relaxed) {
        tcpstat_inc(TcpstatCounters::TcpsScOverflowed);
        // The cache is full. Toss the oldest entry in the first non-empty bucket we can find.
        //
        // XXX We would really like to toss the oldest entry in the cache, but we hope that
        // this condition doesn't happen very often.
        let sc2 = with_buckets(g, active, |b, g| {
            let n = b.len();
            let mut i = bucket;
            if b[i].sch_bucket.is_empty(&*g) {
                i = (bucket + 1) % n;
                while i != bucket && b[i].sch_bucket.is_empty(&*g) {
                    i = (i + 1) % n;
                }
            }
            b[i].sch_bucket.first(&*g).and_then(PoolArc::from_ref)
        });
        // This should never happen; we should always find a non-empty bucket.
        let Some(sc2) = sc2 else {
            kpanic(format_args!("syn_cache_insert: cacheoverflow: impossible"));
        };
        *tossed = Some(syn_cache_rm(g, &sc2));
    }

    // Initialize the entry's timer. We don't estimate RTT with SYNs, so each packet starts
    // with the default RTT and each timer step has a fixed timeout value.
    let rxtcur = {
        let s = sc.sc_s.get_mut(&mut *g);
        s.sc_rxttot = 0;
        s.sc_rxtshift = 0;
        s.sc_rxtcur =
            tcpt_rangeset(TCPTV_SRTTDFLT * TCP_BACKOFF[0], TCPTV_MIN, TCPTV_REXMTMAX) as u32;
        s.sc_rxtcur
    };
    // The timer's handle, for as long as the entry is in the cache (`syn_cache_rm` gives it
    // back): set here where the C's `syn_cache_add` sets the timer, before it is added.
    sc.sc_timer.set_shared(
        syn_cache_timer::<G>,
        PoolArc::erase(sc.clone()),
        KCLOCK_NONE,
        TIMEOUT_PROC | TIMEOUT_MPSAFE,
    );
    let _ = hal::pin::project(sc.as_pin(), |s| &s.sc_timer).add_msec(u64::from(rxtcur));

    // Link it from tcpcb entry: the lists' handle, forgotten until syn_cache_rm adopts it.
    PoolArc::forget(sc.clone());
    hal::pin::project(tp.cb, |c| &c.t_sc).insert_head(g, sc.as_pin());

    // Put it into the bucket.
    with_buckets(g, active, |b, g| {
        b[bucket].sch_bucket.insert_tail(g, sc.as_pin());
        b[bucket].sch_length += 1;
    });
    // `active` is 0 or 1 (`tcp_syn_cache_active`).
    sc.sc_s.get_mut(&mut *g).sc_set = active as u8;
    g.sets[active].scs_count += 1;
    g.sets[active].scs_use -= 1;

    tcpstat_inc(TcpstatCounters::TcpsScAdded);

    // If the active cache has exceeded its use limit and the passive syn cache is empty,
    // exchange their roles.
    if g.sets[active].scs_use <= 0 && g.sets[active ^ 1].scs_count == 0 {
        g.active = active ^ 1;
    }
}

/// `syn_cache_timer`: walk the timer queues, looking for SYN,ACKs that need to be
/// retransmitted. If we have retransmitted an entry the maximum number of times, expire that
/// entry. The argument is the entry's handle the timer holds, lent for the call (the wheel's
/// clone).
pub fn syn_cache_timer<G: ProcGlue>(arg: &AnyArc) {
    let Some(sc) = arg.downcast_ref::<SynCache<G>>() else {
        kpanic(format_args!("syn_cache_timer: not a syn cache entry"));
    };
    let t = &net::<G>().tcp;

    let mut g = t.syn_cache.lock();
    // in_pcbref(sc->sc_inplisten): the listener counted under the mutex, never after.
    let Some(inp) = sc.sc_s.get(&g).sc_inplisten.clone() else {
        // freeit: removed meanwhile; the wheel drops its clone after the call.
        return;
    };

    let dropit = {
        let s = sc.sc_s.get_mut(&mut g);
        if s.sc_rxtshift == TCP_MAXRXTSHIFT as u32 {
            // Drop it -- too many retransmissions.
            true
        } else {
            // Compute the total amount of time this entry has been on a queue. If this entry
            // has been on longer than the keep alive timer would allow, expire it.
            s.sc_rxttot = s.sc_rxttot.wrapping_add(s.sc_rxtcur);
            // The C compares `u_int` with `int`: unsigned.
            if s.sc_rxttot >= TCP_KEEPINIT.load(Ordering::Relaxed) as u32 {
                true
            } else {
                // Advance the timer back-off.
                s.sc_rxtshift += 1;
                s.sc_rxtcur = tcpt_rangeset(
                    TCPTV_SRTTDFLT * TCP_BACKOFF[s.sc_rxtshift as usize],
                    TCPTV_MIN,
                    TCPTV_REXMTMAX,
                ) as u32;
                false
            }
        }
    };
    if dropit {
        tcpstat_inc(TcpstatCounters::TcpsScTimedOut);
        let removed = syn_cache_rm(&mut g, sc);
        drop(g);
        drop(removed);
        drop(inp);
        return;
    }
    let rxtcur = sc.sc_s.get(&g).sc_rxtcur;
    // The re-arm under the mutex, as the C's: the item is pinned in its pool item, reached
    // through a counted handle for the projection.
    if let Some(h) = PoolArc::from_ref(sc) {
        let _ = hal::pin::project(h.as_pin(), |s| &s.sc_timer).add_msec(u64::from(rxtcur));
    }
    drop(g);

    let w = NETLOCK.read();
    let Some(net) = NetHeld::of(&w) else {
        kpanic(format_args!(
            "syn_cache_timer: the net lock's witness proves no holding"
        ));
    };
    if let Some(mut so) = in_pcbsolock(&inp, &net)
        && let Some(mut il) = so.inet()
    {
        let now = tcp_now();
        let do_ecn = TCP_DO_ECN.load(Ordering::Relaxed) != 0;
        let _ = syn_cache_respond(sc, None, now, do_ecn, &mut il.syn, &il.net);
        tcpstat_inc(TcpstatCounters::TcpsScRetransmitted);
    }
    // in_pcbsounlock, NET_UNLOCK_SHARED by the drops; in_pcbunref(inp) after them; the
    // wheel drops its clone of the entry (syn_cache_put) after the call.
}

/// `syn_cache_cleanup`: remove syn cache created by the specified tcb entry, because this does
/// not make sense to keep them (if there's no tcb entry, syn cache entry will never be used).
pub fn syn_cache_cleanup<G: ProcGlue>(tp: &Tp<'_, '_, G>) {
    // NET_ASSERT_LOCKED(): `tp` carries its proof.
    let ring = hal::pin::project(tp.cb, |c| &c.t_sc);
    let mut g = net::<G>().tcp.syn_cache.lock();
    // LIST_FOREACH_SAFE: the first entry taken off until none is left, its handle counted
    // first (the ring lends it under the guard the removal takes).
    while let Some(sc) = ring.first(&g).and_then(PoolArc::from_ref) {
        kassert!(
            sc.sc_s
                .get(&g)
                .sc_inplisten
                .as_ref()
                .is_some_and(|l| core::ptr::eq::<Inpcb<G>>(&**l, tp.inp))
        );
        let removed = syn_cache_rm(&mut g, &sc);
        // syn_cache_put under the mutex, as the C's.
        drop(removed);
        drop(sc);
    }
    drop(g);
}

/// `syn_cache_lookup`: find an entry in the syn cache, the active set first; its handle
/// (see the deviations).
pub fn syn_cache_lookup<G: ProcGlue>(
    g: &mut ScGuard<'_, G>,
    src: &SynCacheSa,
    dst: &SynCacheSa,
    rtableid: u32,
) -> Option<PoolArc<SynCache<G>>> {
    // NET_ASSERT_LOCKED(), MUTEX_ASSERT_LOCKED(&syn_cache_mtx): the guard is the mutex's.
    let srclen = sa_len(src);
    let dstlen = sa_len(dst);
    // Check the active cache first, the passive cache is likely empty.
    let active = g.active;
    for set in [active, active ^ 1] {
        if g.sets[set].scs_count == 0 {
            continue;
        }
        let hash = syn_cache_hash(src, dst, &g.sets[set].scs_random);
        let bucket = (hash % g.sets[set].scs_size as u32) as usize;
        let found = with_buckets(g, set, |b, g| {
            b[bucket]
                .sch_bucket
                .iter(&*g)
                .find(|sc| {
                    sc.sc_s.get(&*g).sc_hash == hash
                        && sc.sc_i.sc_src[..srclen] == src[..srclen]
                        && sc.sc_i.sc_dst[..dstlen] == dst[..dstlen]
                        && rtable_l2(rtableid) == rtable_l2(sc.sc_i.sc_rtableid)
                })
                .and_then(PoolArc::from_ref)
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

/// `syn_cache_reset`: called when we get a RST for a non-existent connection, so that we can
/// see if the connection is in the syn cache. If it is, zap it. `th` in host order.
pub fn syn_cache_reset<G: ProcGlue>(
    src: &SynCacheSa,
    dst: &SynCacheSa,
    th: &Tcphdr,
    rtableid: u32,
    _net: &NetHeld<'_>,
) {
    let mut g = net::<G>().tcp.syn_cache.lock();
    let Some(sc) = syn_cache_lookup(&mut g, src, dst, rtableid) else {
        return;
    };
    let irs = sc.sc_i.sc_irs;
    if seq_lt(th.th_seq, irs) || seq_gt(th.th_seq, irs.wrapping_add(1)) {
        return;
    }
    let removed = syn_cache_rm(&mut g, &sc);
    drop(g);
    tcpstat_inc(TcpstatCounters::TcpsScReset);
    drop(removed);
    drop(sc);
}

/// `syn_cache_unreach`: an ICMP unreachable for the embryonic connection from `src` to `dst`
/// whose returned TCP header is `th` (network order); the second one after three
/// retransmissions removes the entry.
pub fn syn_cache_unreach<G: ProcGlue>(
    src: &SynCacheSa,
    dst: &SynCacheSa,
    th: &Tcphdr,
    rtableid: u32,
    _net: &NetHeld<'_>,
) {
    let mut g = net::<G>().tcp.syn_cache.lock();
    let Some(sc) = syn_cache_lookup(&mut g, src, dst, rtableid) else {
        return;
    };
    // If the sequence number != sc_iss, then it's a bogus ICMP msg
    if ntohl(th.th_seq) != sc.sc_i.sc_iss {
        return;
    }

    // If we've retransmitted 3 times and this is our second error, we remove the entry.
    // Otherwise, we allow it to continue on. This prevents us from incorrectly nuking an
    // entry during a spurious network outage.
    //
    // See tcp_notify().
    {
        let s = sc.sc_s.get_mut(&mut g);
        if s.sc_dynflags & SCF_UNREACH == 0 || s.sc_rxtshift < 3 {
            s.sc_dynflags |= SCF_UNREACH;
            return;
        }
    }

    let removed = syn_cache_rm(&mut g, &sc);
    drop(g);
    tcpstat_inc(TcpstatCounters::TcpsScUnreach);
    drop(removed);
    drop(sc);
}

/// `syn_cache_respond`: sends the SYN-ACK of the entry `sc` (the incoming segment `m`, if any,
/// freed first), its `[s]` members opened with the listen socket's part `SYN` (`lsyn`), the
/// net lock proved by `net`. `ENOBUFS` without an mbuf, `EPERM` without the SA of a signed
/// connection, `EINVAL` when the signature cannot be made, or `ip_output`'s error.
pub fn syn_cache_respond<G: ProcGlue>(
    sc: &SynCache<G>,
    m: Option<MbufBox>,
    now: u64,
    do_ecn: bool,
    lsyn: &mut WitnessPart<'_>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let i = &sc.sc_i;
    let af = sa_family(&i.sc_src);
    let hlen = match af {
        AF_INET => size_of::<Ip>(),
        AF_INET6 => size_of::<Ip6Hdr>(),
        _ => {
            drop(m);
            return Err(Errno::EAFNOSUPPORT);
        }
    };

    // Compute the size of the TCP options.
    let optlen = 4
        + if i.sc_request_r_scale != 15 { 4 } else { 0 }
        + if i.sc_fixflags & SCF_SACK_PERMIT != 0 {
            4
        } else {
            0
        }
        + if i.sc_fixflags & SCF_SIGNATURE != 0 {
            usize::from(TCPOLEN_SIGLEN)
        } else {
            0
        }
        + if i.sc_fixflags & SCF_TIMESTAMP != 0 {
            usize::from(TCPOLEN_TSTAMP_APPA)
        } else {
            0
        };

    let tlen = hlen + size_of::<Tcphdr>() + optlen;

    // Create the IP+TCP header from scratch.
    drop(m);
    let max_linkhdr = MAX_LINKHDR.load(Ordering::Relaxed) as usize;
    if cfg!(feature = "diagnostic") && max_linkhdr + tlen > MCLBYTES {
        return Err(Errno::ENOBUFS);
    }
    let mut m = m_gethdr(M_DONTWAIT, MT_DATA);
    if let Some(mm) = m.as_mut()
        && max_linkhdr + tlen > MHLEN
        && !m_clget(mm, M_DONTWAIT, (max_linkhdr + tlen) as u32)
    {
        m = None;
    }
    let Some(mut m) = m else {
        return Err(Errno::ENOBUFS);
    };

    // Fixup the mbuf.
    m.data_add(max_linkhdr as isize);
    m.m_len().set(tlen as u32);
    let ph = m.m_pkthdr();
    ph.len.set(tlen as i32);
    ph.ph_ifidx.set(0);
    ph.ph_rtableid.set(i.sc_rtableid);

    let mut th = Tcphdr::default();
    let mut ip = Ip::default();
    let mut ip6 = Ip6Hdr::default();
    if af == AF_INET {
        ip.ip_dst.s_addr = sa_word(&i.sc_src, 4);
        ip.ip_src.s_addr = sa_word(&i.sc_dst, 4);
        ip.ip_p = IPPROTO_TCP as u8;
    } else {
        ip6.ip6_dst.s6_addr = sa_addr6(&i.sc_src);
        ip6.ip6_src.s6_addr = sa_addr6(&i.sc_dst);
        ip6.ip6_nxt = IPPROTO_TCP as u8;
    }
    th.th_dport = sa_port(&i.sc_src);
    th.th_sport = sa_port(&i.sc_dst);

    th.th_seq = htonl(i.sc_iss);
    th.th_ack = htonl(i.sc_irs.wrapping_add(1));
    th.set_th_off(((size_of::<Tcphdr>() + optlen) >> 2) as u8);
    th.th_flags = TH_SYN | TH_ACK;
    // Set ECE for SYN-ACK if peer supports ECN.
    if do_ecn && i.sc_fixflags & SCF_ECN_PERMIT != 0 {
        th.th_flags |= TH_ECE;
    }
    th.th_win = htons(i.sc_win as u16);
    // th_sum already 0, th_urp already 0

    // Tack on the TCP options, after the headers (zeroed, as the C's memset). They fit the 40
    // bytes of a TCP header's option room because a signed connection does not permit SACK
    // (the module's deviations: tcp_ctloutput refuses the pair).
    kassert!(i.sc_fixflags & SCF_SIGNATURE == 0 || i.sc_fixflags & SCF_SACK_PERMIT == 0);
    let mut opts = [0u8; 40];
    let mut o = 0usize;
    opts[..4].copy_from_slice(&[
        TCPOPT_MAXSEG,
        4,
        (i.sc_ourmaxseg >> 8) as u8,
        i.sc_ourmaxseg as u8,
    ]);
    o += 4;

    // Include SACK_PERMIT_HDR option if peer has already done so.
    if i.sc_fixflags & SCF_SACK_PERMIT != 0 {
        opts[o..o + 4].copy_from_slice(&htonl(TCPOPT_SACK_PERMIT_HDR).to_ne_bytes());
        o += 4;
    }

    if i.sc_request_r_scale != 15 {
        let w = u32::from(TCPOPT_NOP) << 24
            | u32::from(TCPOPT_WINDOW) << 16
            | u32::from(TCPOLEN_WINDOW) << 8
            | u32::from(i.sc_request_r_scale);
        opts[o..o + 4].copy_from_slice(&htonl(w).to_ne_bytes());
        o += 4;
    }

    let l = sc.sc_l.get_mut(lsyn);
    if i.sc_fixflags & SCF_TIMESTAMP != 0 {
        // Form timestamp option as shown in appendix A of RFC 1323.
        let words = [
            htonl(TCPOPT_TSTAMP_HDR),
            htonl((now as u32).wrapping_add(i.sc_modulate)),
            htonl(l.sc_timestamp as u32),
        ];
        for w in words {
            opts[o..o + 4].copy_from_slice(&w.to_ne_bytes());
            o += 4;
        }
    }

    let Some(d) = m.data_mut() else {
        kpanic(format_args!("syn_cache_respond: a new mbuf is read-only"));
    };
    d.fill(0);
    if af == AF_INET {
        put_hdr(d, 0, &ip);
    } else {
        put_hdr(d, 0, &ip6);
    }

    if i.sc_fixflags & SCF_SIGNATURE != 0 {
        // The SA between the addresses the reply carries: two unions zeroed but for their
        // length, family and address.
        let mut src = [0u8; SOCKADDR_UNION_LEN];
        let mut dst = [0u8; SOCKADDR_UNION_LEN];
        src[..2].copy_from_slice(&i.sc_src[..2]);
        dst[..2].copy_from_slice(&i.sc_dst[..2]);
        if af == AF_INET {
            src[4..8].copy_from_slice(&ip.ip_src.s_addr.to_ne_bytes());
            dst[4..8].copy_from_slice(&ip.ip_dst.s_addr.to_ne_bytes());
        } else {
            src[8..24].copy_from_slice(&ip6.ip6_src.s6_addr);
            dst[8..24].copy_from_slice(&ip6.ip6_dst.s6_addr);
        }
        let Some(tdb) =
            ipsec().and_then(|s| s.tcp_signature_tdb(rtable_l2(i.sc_rtableid), &src, &dst))
        else {
            return Err(Errno::EPERM);
        };

        // Send signature option
        opts[o] = TCPOPT_SIGNATURE;
        opts[o + 1] = TCPOLEN_SIGNATURE;
        o += 2;

        let mut sig = [0u8; 16];
        let ok = tcp_signature(&tdb, i32::from(af), &m, &th, hlen as i32, false, &mut sig);
        if let Some(s) = ipsec() {
            s.tdb_unref(tdb);
        }
        if !ok {
            return Err(Errno::EINVAL);
        }
        opts[o..o + 16].copy_from_slice(&sig);
        o += 16;

        // Pad options list to the next 32 bit boundary and terminate it.
        opts[o] = TCPOPT_NOP;
        opts[o + 1] = TCPOPT_EOL;
        o += 2;
    }
    if o != optlen {
        kpanic(format_args!(
            "syn_cache_respond: {o} option bytes of {optlen}"
        ));
    }

    let Some(d) = m.data_mut() else {
        kpanic(format_args!("syn_cache_respond: a new mbuf is read-only"));
    };
    put_hdr(d, hlen, &th);
    d[hlen + size_of::<Tcphdr>()..tlen].copy_from_slice(&opts[..optlen]);

    let ph = m.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_OUT);

    // use IPsec policy and ttl from listening socket, on SYN ACK
    let inp = {
        let g = crate::net::domain::net::<G>().tcp.syn_cache.lock();
        sc.sc_s.get(&g).sc_inplisten.clone()
    };

    // Fill in some straggling IP bits. Note the stack expects ip_len to be in host order, for
    // convenience.
    let seclevel = inp.as_ref().map(|p| p.inp_seclevel.get());
    let error = if af == AF_INET {
        ip.ip_len = htons(tlen as u16);
        match &inp {
            Some(p) => {
                let pip = p.inp_ip.get();
                ip.ip_ttl = pip.ip_ttl;
                ip.ip_tos = pip.ip_tos;
            }
            None => ip.ip_ttl = IP_DEFTTL.load(Ordering::Relaxed) as u8,
        }
        if let Some(d) = m.data_mut() {
            put_hdr(d, 0, &ip);
        }
        ip_output(
            m,
            l.sc_ipopts.as_deref().map(Mbuf::data),
            Some(&mut l.sc_route),
            if ip_mtudisc.load(Ordering::Relaxed) != 0 {
                IP_MTUDISC
            } else {
                0
            },
            None,
            seclevel.as_ref(),
            0,
            net,
        )
    } else {
        ip6.set_ip6_vfc((ip6.ip6_vfc() & !IPV6_VERSION_MASK) | IPV6_VERSION);
        // ip6_plen will be updated in ip6_output()
        ip6.ip6_hlim = match &inp {
            Some(p) => in6_selecthlim(p),
            None => IP6_DEFHLIM.load(Ordering::Relaxed),
        } as u8;
        // leave flowlabel = 0, it is legal and require no state mgmt
        if let Some(d) = m.data_mut() {
            put_hdr(d, 0, &ip6);
        }
        ip6_output(
            m,
            None, // XXX
            Some(&mut l.sc_route),
            0,
            None,
            seclevel.as_ref(),
            net,
        )
    };
    drop(inp);
    error
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the SYN cache: the hash as the C's formula, an entry's life through
    // insert, lookup, reset, unreach and the listener's close, the two limits that toss the
    // oldest entry, the sets' swap, the retransmit timer and its two expiries, the pool back
    // at its baseline.

    use std::vec::Vec;

    use hal::pod;
    use hal::sys::netinet_tcp::TCPOPT_MAXSEG;

    use super::*;
    use crate::netinet::in_pcb::testinet;
    use crate::netinet::tcp_subr::testtcp;
    use crate::netinet::tcp_var::TCPCOUNTERS;

    type G = testtcp::G;

    /// A `sockaddr_in` of `a`:`port` as the union's bytes.
    fn sa(a: [u8; 4], port: u16) -> SynCacheSa {
        let mut s = [0u8; SOCKADDR_UNION_LEN];
        s[0] = 16;
        s[1] = AF_INET;
        s[2..4].copy_from_slice(&port.to_be_bytes());
        s[4..8].copy_from_slice(&a);
        s
    }

    #[test]
    fn hash_mixes_ports_and_source() {
        let src = sa([10, 0, 0, 1], 1234);
        let dst = sa([10, 0, 0, 2], 80);
        let rand = [0x1111_1111, 2, 3, 4, 0x5555_5555];
        let sport = u32::from(1234u16.to_be());
        let dport = u32::from(80u16.to_be());
        let addr = u32::from_ne_bytes([10, 0, 0, 1]);
        assert_eq!(
            syn_cache_hash(&src, &dst, &rand),
            (((dport << 16).wrapping_add(sport)) ^ rand[4]).wrapping_mul(addr ^ rand[0])
        );
        // Only the source address enters the product.
        let other = sa([10, 0, 0, 3], 80);
        assert_eq!(
            syn_cache_hash(&src, &dst, &rand),
            syn_cache_hash(&src, &other, &rand)
        );
        // IPv6: the four words of the source address.
        let mut s6 = [0u8; SOCKADDR_UNION_LEN];
        s6[0] = 28;
        s6[1] = AF_INET6;
        s6[2..4].copy_from_slice(&1234u16.to_be_bytes());
        s6[8..24].copy_from_slice(&[0x20, 1, 0xd, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let w = |i: usize| {
            u32::from_ne_bytes([s6[8 + 4 * i], s6[9 + 4 * i], s6[10 + 4 * i], s6[11 + 4 * i]])
        };
        let mut d6 = s6;
        d6[2..4].copy_from_slice(&80u16.to_be_bytes());
        let want = (((dport << 16).wrapping_add(sport)) ^ rand[4])
            .wrapping_mul(w(0) ^ rand[0])
            .wrapping_mul(w(1) ^ rand[1])
            .wrapping_mul(w(2) ^ rand[2])
            .wrapping_mul(w(3) ^ rand[3]);
        assert_eq!(syn_cache_hash(&s6, &d6, &rand), want);
    }

    /// A fresh entry for `src -> dst` of the listener `inp`, as `syn_cache_add` fills it.
    fn entry(inp: &Inpcb<G>, src: SynCacheSa, dst: SynCacheSa, irs: u32) -> PoolArc<SynCache<G>> {
        let init = SynCacheInit {
            sc_src: src,
            sc_dst: dst,
            sc_irs: irs,
            sc_iss: irs ^ 0x5a5a_0000,
            sc_win: 16384,
            sc_ourmaxseg: 1460,
            sc_request_r_scale: 15,
            ..SynCacheInit::default()
        };
        syn_cache_alloc(inp, init, ScListener::default()).expect("a syn cache entry")
    }

    /// `syn_cache_insert` of `sc` on the listener `inp`, the tossed entries dropped after.
    fn insert(inp: &Inpcb<G>, sc: &PoolArc<SynCache<G>>) {
        testtcp::with_tp(inp, |tp| {
            let mut tossed = None;
            let mut g = net::<G>().tcp.syn_cache.lock();
            syn_cache_insert(&mut g, sc, &tp, &mut tossed);
            drop(g);
            drop(tossed);
            ((), None)
        });
    }

    /// `syn_cache_lookup` under the mutex.
    fn lookup(src: &SynCacheSa, dst: &SynCacheSa) -> Option<PoolArc<SynCache<G>>> {
        let mut g = net::<G>().tcp.syn_cache.lock();
        syn_cache_lookup(&mut g, src, dst, 0)
    }

    /// A counter of `tcpcounters`.
    fn count(c: TcpstatCounters) -> u64 {
        TCPCOUNTERS.read()[c as usize]
    }

    /// The items out of the "syncache" pool.
    fn entries() -> u64 {
        net::<G>().tcp.syn_cache_pool.nout()
    }

    #[test]
    fn an_entry_lives_from_insert_to_reset_unreach_and_close() {
        let (_g, _p) = testtcp::setup();
        let base = entries();
        let (so, inp) = testtcp::conn();
        let dst = sa([10, 0, 0, 1], 80);
        let (s1, s2, s3) = (
            sa([10, 0, 0, 2], 1001),
            sa([10, 0, 0, 2], 1002),
            sa([10, 0, 0, 2], 1003),
        );
        let added = count(TcpstatCounters::TcpsScAdded);
        for (s, irs) in [(s1, 100), (s2, 200), (s3, 300)] {
            let sc = entry(&inp, s, dst, irs);
            insert(&inp, &sc);
            assert!(sc.sc_timer.pending(), "the retransmit timer runs");
        }
        assert_eq!(count(TcpstatCounters::TcpsScAdded) - added, 3);
        assert_eq!(entries(), base + 3);
        let found = lookup(&s2, &dst).expect("the second entry");
        assert_eq!(found.sc_i.sc_irs, 200);
        assert!(
            lookup(&sa([10, 0, 0, 9], 1002), &dst).is_none(),
            "another source"
        );
        // The same hash (sin_zero is not hashed), another source: compared over sa_len.
        let mut zeroed = s2;
        zeroed[8] = 1;
        assert!(lookup(&zeroed, &dst).is_none(), "the source compared whole");
        drop(found);

        let net_w = hal::netlock::NETLOCK.read();
        let held = NetHeld::of(&net_w).expect("the net lock");
        // A RST outside [irs, irs + 1] leaves the entry; inside, it zaps it.
        let reset = count(TcpstatCounters::TcpsScReset);
        let th = |seq: u32| Tcphdr {
            th_seq: seq,
            ..Tcphdr::default()
        };
        syn_cache_reset::<G>(&s1, &dst, &th(99), 0, &held);
        syn_cache_reset::<G>(&s1, &dst, &th(102), 0, &held);
        assert!(lookup(&s1, &dst).is_some());
        syn_cache_reset::<G>(&s1, &dst, &th(101), 0, &held);
        assert!(lookup(&s1, &dst).is_none());
        assert_eq!(count(TcpstatCounters::TcpsScReset) - reset, 1);

        // An unreachable for another sequence is bogus; the first real one only marks the
        // entry; a second one after three retransmissions removes it.
        let unreach = count(TcpstatCounters::TcpsScUnreach);
        let iss = 200 ^ 0x5a5a_0000u32;
        syn_cache_unreach::<G>(&s2, &dst, &th(htonl(iss + 1)), 0, &held);
        syn_cache_unreach::<G>(&s2, &dst, &th(htonl(iss)), 0, &held);
        // A second error before three retransmissions keeps the entry.
        syn_cache_unreach::<G>(&s2, &dst, &th(htonl(iss)), 0, &held);
        let sc2 = lookup(&s2, &dst).expect("still cached");
        {
            let mut g = net::<G>().tcp.syn_cache.lock();
            let s = sc2.sc_s.get_mut(&mut g);
            let marked = s.sc_dynflags & SCF_UNREACH != 0;
            s.sc_rxtshift = 3;
            drop(g);
            assert!(marked, "SCF_UNREACH");
        }
        drop(sc2);
        syn_cache_unreach::<G>(&s2, &dst, &th(htonl(iss)), 0, &held);
        assert!(lookup(&s2, &dst).is_none());
        assert_eq!(count(TcpstatCounters::TcpsScUnreach) - unreach, 1);
        drop(held);
        drop(net_w);

        // The listener's close removes what is left (syn_cache_cleanup).
        assert!(lookup(&s3, &dst).is_some());
        testtcp::close(so, inp);
        assert!(lookup(&s3, &dst).is_none());
        assert_eq!(entries(), base, "every entry back in the pool");
    }

    /// What the limits test changes, put back by its drop (an assertion that fails unwinds
    /// through it).
    struct Restore;

    impl Drop for Restore {
        fn drop(&mut self) {
            TCP_SYN_BUCKET_LIMIT.store(3 * TCP_SYN_BUCKET_SIZE, Ordering::Relaxed);
            TCP_SYN_CACHE_LIMIT.store(TCP_SYN_HASH_SIZE * TCP_SYN_BUCKET_SIZE, Ordering::Relaxed);
            let mut g = syn_cache_lock::<G>();
            g.tcp_syn_hash_size = TCP_SYN_HASH_SIZE;
            for s in &mut g.sets {
                s.scs_use = 0;
            }
        }
    }

    #[test]
    fn the_limits_toss_the_oldest_and_the_sets_swap() {
        let (_g, _p) = testtcp::setup();
        let _restore = Restore;
        let base = entries();
        let (so, inp) = testtcp::conn();
        let dst = sa([10, 0, 0, 1], 80);
        // One bucket per set from the next reseed (an empty, used-up set is resized), and a
        // bucket limit of two.
        let first = {
            let mut g = net::<G>().tcp.syn_cache.lock();
            g.tcp_syn_hash_size = 1;
            for s in &mut g.sets {
                s.scs_use = 0;
            }
            g.active
        };
        TCP_SYN_BUCKET_LIMIT.store(2, Ordering::Relaxed);
        let over = count(TcpstatCounters::TcpsScBucketoverflow);
        let e: Vec<SynCacheSa> = (0..3).map(|i| sa([10, 0, 0, 2], 2000 + i)).collect();
        for (i, s) in e.iter().enumerate() {
            insert(&inp, &entry(&inp, *s, dst, 10 * i as u32));
        }
        TCP_SYN_BUCKET_LIMIT.store(3 * TCP_SYN_BUCKET_SIZE, Ordering::Relaxed);
        assert_eq!(count(TcpstatCounters::TcpsScBucketoverflow) - over, 1);
        assert!(
            lookup(&e[0], &dst).is_none(),
            "the oldest of the bucket tossed"
        );
        assert!(lookup(&e[1], &dst).is_some() && lookup(&e[2], &dst).is_some());
        // The overflow zeroed the set's uses: used up, it gave the active role to the empty
        // passive set (whose entries a lookup still finds in the other).
        let second = net::<G>().tcp.syn_cache.lock().active;
        assert_ne!(second, first, "the sets swapped");

        // The cache limit: the oldest entry of the first non-empty bucket goes.
        TCP_SYN_CACHE_LIMIT.store(1, Ordering::Relaxed);
        let overflowed = count(TcpstatCounters::TcpsScOverflowed);
        let (f0, f1) = (sa([10, 0, 0, 2], 2100), sa([10, 0, 0, 2], 2101));
        insert(&inp, &entry(&inp, f0, dst, 7));
        insert(&inp, &entry(&inp, f1, dst, 8));
        TCP_SYN_CACHE_LIMIT.store(TCP_SYN_HASH_SIZE * TCP_SYN_BUCKET_SIZE, Ordering::Relaxed);
        assert_eq!(count(TcpstatCounters::TcpsScOverflowed) - overflowed, 1);
        assert!(lookup(&f0, &dst).is_none() && lookup(&f1, &dst).is_some());
        assert!(
            lookup(&e[1], &dst).is_some(),
            "the passive set's entries stay"
        );
        testtcp::close(so, inp);
        assert_eq!(entries(), base);
    }

    #[test]
    fn the_timer_retransmits_then_expires() {
        let (_g, _p) = testtcp::setup();
        let (_ifp, d) = testinet::iface(b"tsyn0", [10, 9, 3, 1]);
        let base = entries();
        let (so, inp) = testtcp::conn();
        let src = sa([10, 9, 3, 2], 4321);
        let dst = sa([10, 9, 3, 1], 80);
        let sc = entry(&inp, src, dst, 1000);
        insert(&inp, &sc);
        let arg = PoolArc::erase(sc.clone());

        // A retransmission: the backoff advances, the SYN-ACK goes out again.
        let rexmit = count(TcpstatCounters::TcpsScRetransmitted);
        let sent = d.sent.load(Ordering::Relaxed);
        syn_cache_timer::<G>(&arg);
        assert_eq!(count(TcpstatCounters::TcpsScRetransmitted) - rexmit, 1);
        assert_eq!(d.sent.load(Ordering::Relaxed), sent + 1);
        let (shift, cur, tot) = {
            let g = net::<G>().tcp.syn_cache.lock();
            let s = sc.sc_s.get(&g);
            (s.sc_rxtshift, s.sc_rxtcur, s.sc_rxttot)
        };
        assert_eq!((shift, cur, tot), (1, 6000, 3000));
        let p = d.last.lock().expect("the capture").clone();
        let ip: Ip = pod::at(&p, 0).expect("the IP header");
        assert_eq!(ip.ip_dst.s_addr, u32::from_ne_bytes([10, 9, 3, 2]));
        let th: Tcphdr = pod::at(&p, 20).expect("the TCP header");
        assert_eq!(th.th_flags, TH_SYN | TH_ACK);
        assert_eq!(u32::from_be(th.th_seq), 1000 ^ 0x5a5a_0000);
        assert_eq!(u32::from_be(th.th_ack), 1001);
        assert_eq!(
            (u16::from_be(th.th_sport), u16::from_be(th.th_dport)),
            (80, 4321)
        );
        assert_eq!(th.th_off(), 6, "the MSS option alone");
        assert_eq!(&p[40..44], &[TCPOPT_MAXSEG, 4, 0x05, 0xb4]);
        assert_eq!(u16::from_be(th.th_win), 16384);

        // Past the keep alive time: timed out, removed; a late fire finds it removed.
        let timed_out = count(TcpstatCounters::TcpsScTimedOut);
        {
            let mut g = net::<G>().tcp.syn_cache.lock();
            sc.sc_s.get_mut(&mut g).sc_rxttot =
                crate::netinet::tcp_timer::TCP_KEEPINIT.load(Ordering::Relaxed) as u32;
        }
        syn_cache_timer::<G>(&arg);
        assert_eq!(count(TcpstatCounters::TcpsScTimedOut) - timed_out, 1);
        assert!(lookup(&src, &dst).is_none());
        assert!(!sc.sc_timer.pending(), "deleted with the entry");
        syn_cache_timer::<G>(&arg);
        assert_eq!(count(TcpstatCounters::TcpsScTimedOut) - timed_out, 1);

        // Too many retransmissions expire an entry too.
        let sc2 = entry(&inp, sa([10, 9, 3, 2], 4322), dst, 5);
        insert(&inp, &sc2);
        {
            let mut g = net::<G>().tcp.syn_cache.lock();
            sc2.sc_s.get_mut(&mut g).sc_rxtshift = TCP_MAXRXTSHIFT as u32;
        }
        syn_cache_timer::<G>(&PoolArc::erase(sc2.clone()));
        assert_eq!(count(TcpstatCounters::TcpsScTimedOut) - timed_out, 2);
        drop((arg, sc, sc2));
        testtcp::close(so, inp);
        assert_eq!(entries(), base);
    }

    #[test]
    fn the_syn_ack_carries_the_options_the_entry_allows() {
        let (_g, _p) = testtcp::setup();
        let (_ifp, d) = testinet::iface(b"tsyo0", [10, 9, 4, 1]);
        let base = entries();
        let (so, inp) = testtcp::conn();
        let init = SynCacheInit {
            sc_src: sa([10, 9, 4, 2], 5555),
            sc_dst: sa([10, 9, 4, 1], 80),
            sc_irs: 77,
            sc_iss: 88,
            sc_win: 1000,
            sc_modulate: 10,
            sc_ourmaxseg: 1400,
            sc_request_r_scale: 3,
            sc_fixflags: SCF_SACK_PERMIT | SCF_TIMESTAMP | SCF_ECN_PERMIT,
            ..SynCacheInit::default()
        };
        let l = || ScListener {
            sc_timestamp: 0x0102_0304,
            ..ScListener::default()
        };
        let sc = syn_cache_alloc(&inp, init, l()).expect("an entry");
        let sent = d.sent.load(Ordering::Relaxed);
        let r = testtcp::with_lock(&inp, |_, il| {
            syn_cache_respond(&sc, None, 500, true, &mut il.syn, &il.net)
        });
        assert_eq!(r, Ok(()));
        assert_eq!(d.sent.load(Ordering::Relaxed), sent + 1);
        let p = d.last.lock().expect("the capture").clone();
        // MSS 4, SACK permitted 4, window scale 4, timestamps 12: 24 option bytes.
        assert_eq!(p.len(), 20 + 20 + 24);
        let ip: Ip = pod::at(&p, 0).expect("the IP header");
        assert_eq!(u16::from_be(ip.ip_len), 64);
        assert_eq!(ip.ip_src.s_addr, u32::from_ne_bytes([10, 9, 4, 1]));
        assert_eq!(ip.ip_ttl, inp.inp_ip.get().ip_ttl, "the listener's ttl");
        let th: Tcphdr = pod::at(&p, 20).expect("the TCP header");
        assert_eq!(th.th_off(), 11);
        assert_eq!(
            th.th_flags,
            TH_SYN | TH_ACK | TH_ECE,
            "ECE: the peer does ECN"
        );
        assert_eq!((u32::from_be(th.th_seq), u32::from_be(th.th_ack)), (88, 78));
        assert_eq!(u16::from_be(th.th_win), 1000);
        assert_eq!(&p[40..44], &[TCPOPT_MAXSEG, 4, 0x05, 0x78]);
        assert_eq!(
            &p[44..48],
            &[TCPOPT_NOP, TCPOPT_NOP, 4, 2],
            "SACK permitted"
        );
        assert_eq!(&p[48..52], &[TCPOPT_NOP, TCPOPT_WINDOW, TCPOLEN_WINDOW, 3]);
        assert_eq!(
            &p[52..56],
            &[TCPOPT_NOP, TCPOPT_NOP, 8, 10],
            "the timestamps"
        );
        assert_eq!(&p[56..60], &510u32.to_be_bytes(), "now + sc_modulate");
        assert_eq!(&p[60..64], &0x0102_0304u32.to_be_bytes(), "sc_timestamp");

        // Without ECN asked, no ECE; with no option but the MSS, th_off 6.
        let bare = syn_cache_alloc(
            &inp,
            SynCacheInit {
                sc_fixflags: SCF_ECN_PERMIT,
                sc_request_r_scale: 15,
                ..init
            },
            l(),
        )
        .expect("an entry");
        let r = testtcp::with_lock(&inp, |_, il| {
            syn_cache_respond(&bare, None, 500, false, &mut il.syn, &il.net)
        });
        assert_eq!(r, Ok(()));
        let p = d.last.lock().expect("the capture").clone();
        let th: Tcphdr = pod::at(&p, 20).expect("the TCP header");
        assert_eq!((th.th_off(), th.th_flags), (6, TH_SYN | TH_ACK));

        // A signed entry with no IPsec to give its SA: EPERM, nothing sent (the kernel's
        // tests register no Ipsec legs, so the EINVAL and tdb_unref path has no double).
        let signed = syn_cache_alloc(
            &inp,
            SynCacheInit {
                sc_fixflags: SCF_SIGNATURE | SCF_TIMESTAMP,
                ..init
            },
            l(),
        )
        .expect("an entry");
        let sent = d.sent.load(Ordering::Relaxed);
        let r = testtcp::with_lock(&inp, |_, il| {
            syn_cache_respond(&signed, None, 500, false, &mut il.syn, &il.net)
        });
        assert_eq!(r, Err(Errno::EPERM));
        assert_eq!(d.sent.load(Ordering::Relaxed), sent);
        drop((sc, bare, signed));
        testtcp::close(so, inp);
        assert_eq!(entries(), base);
    }
}
/* </TESTS> */
