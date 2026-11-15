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
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 Neighbor Discovery: the neighbor cache, the per-interface ND information and the
//! address resolution, `netinet6/nd6.c`, and the kernel half of `<netinet6/nd6.h>` (its
//! userland half, the states and the requests ndp(8) reads, is `hal::sys::netinet6_nd6`).
//!
//! Upstream: sys/netinet6/nd6.h @ 3ce1f3f79392
//! Upstream: sys/netinet6/nd6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/nd6.rs@3c62ede68802
//!
//! As ARP does for IPv4 (`kernel::netinet::if_ether`), the neighbor cache lives in the routing
//! table: a cloning route of an on-link prefix makes a host route per neighbor
//! (`RTF_LLINFO`), whose gateway is a `sockaddr_dl` holding the neighbor's link-layer address
//! and whose `rt_llinfo` is a [`LlinfoNd6`] (the reachability state of RFC 2461 7.3.2, the
//! packets held while the address is resolved, the probes sent). [`nd6_resolve`] answers
//! `ether_resolve` from it or holds the packet and solicits, `kernel::netinet6::nd6_nbr`
//! fills it from solicitations and advertisements ([`nd6_cache_lladdr`], `nd6_na_input`), and
//! [`nd6_timer`] walks it through INCOMPLETE, REACHABLE, STALE, DELAY and PROBE.
//!
//! Locks used to protect struct members in this file: \[a\] atomic operations, \[I\] immutable
//! after creation, \[K\] kernel lock, \[m\] nd6 mutex, needed when net lock is shared, \[N\] net
//! lock.
//!
//! ## Deviations
//! - `inet_ntop` (`netinet/inet_ntop.c`) is not ported: the log lines print addresses through
//!   [`In6Ntop`], which writes what it writes for `AF_INET6` (LZ's).
//! - `nd6_free` reads the router flag only when the route still has its `llinfo_nd6` (the C
//!   dereferences `rt_llinfo` even on `nd6_cache_lladdr`'s `fail` path, where it may be NULL):
//!   its callers pass it, read under the mutex (LZ's).
//! - `nd6_cache_lladdr` takes the link-layer address as a slice (the C's pointer and
//!   `lladdrlen`, which the C never reads); the callers pass the bytes after the option header
//!   and checked their length against `if_addrlen` (LZ's).
//! - `nd6_options` takes the options as a byte slice and answers `bool` (the C's 0/-1);
//!   [`NdOpts`] holds the options' offsets in it (LZ's pointers into the packet).
//! - The casts and inline code the C repeats in `nd6.c`, `nd6_nbr.c` and `nd6_rtr.c` are
//!   crate-visible helpers here: [`rt_key_in6`] (`satosin6(rt_key(rt))->sin6_addr`),
//!   [`nd6_opt_lladdr`] (an option's address and `lladdrlen`), `nd6_llchanged`/`nd6_llstore`
//!   (the `bcmp`/`bcopy` of `if_addrlen` bytes against `LLADDR(sdl)`), `nd6_ether_sprintf`;
//!   `nd6_llsol` is the solicited-node address `nd6_rtrequest` computes twice (LZ's).
//! - `nd6_rtrequest`'s `RTM_DELETE` reads the proxy's solicited-node record under the
//!   interface's `if_maddrlock` (read), which `in6_lookupmulti` asserts and the C's caller
//!   does not take (the C's `KERNEL_LOCK` alone); the kernel lock then the multicast lock is
//!   the C's own order (`RTM_ADD`'s `in6_addmulti`).
//!
//! ## Redesign
//! - LZ's `struct llinfo_nd6` (`Cell`s and a `TailqEntry`, a `pool_get`'s raw item written in
//!   an `unsafe` block, freed by `pool_put` at its last `ln_refcnt`, a bare `&'static Rtentry`
//!   back pointer) hung on `rt_llinfo` as the untagged legacy address, cast back at every
//!   reader (`rt_ln`'s `unsafe`) -> a [`LlinfoNd6`] of the "nd6" [`BoxPool`], owned by the
//!   route's word as `hal::link::TaggedLink`'s second kind (D-5): the word lends the record
//!   only to a guard of [`ND6_MTX`] (the C's "llinfo_nd6 live time, rt_llinfo and RTF_LLINFO
//!   are protected by nd6_mtx", now `LlinfoNd6`'s `Tagged::LOCK`), gives the box back only to
//!   the whole guard borrowed mutably (`RTM_DELETE`), and no lent record can be made a box
//!   (N6.4b's B1: the box's raw form, never the park). Its one owner is the word, so
//!   `ln_refcnt` (the list's reference and the iterator's) goes: the list links the pinned
//!   record ([`ND6_LIST`], a `hal::pinlist::PinList` beside the mutex), and `nd6_timer` and
//!   `nd6_purge` hold no reference to a record across the mutex's release, only its route.
//! - `ln_rt`, the C's uncounted back pointer, is a counted handle (`PoolArc<Rtentry>`, \[I\];
//!   `None` only for a walk's marker): the walks clone it under the mutex where the C
//!   `rtref`s it, and the route and its record keep each other until `RTM_DELETE` takes the
//!   record out of the route. The C's pointer counts nothing, so `rt_refcnt` (`rmx_refcnt`,
//!   which `route(8)` prints) subtracts it while the word owns a record (`is_typed`, either
//!   kind: N6.4b's B2).
//! - `nd6_mtx` is [`ND6_MTX`], whose data is `nd6_inuse` (\[m\]); the \[m\] members of the
//!   record are atomics written under it, `ln_mq` the kernel's `MbufQueue` (its own mutex, a
//!   leaf, as the C's), `ln_saddr6` four words (`In6AddrCell`).
//! - `nd6_iterator` (a `struct llinfo_nd6_iterator` marker, the current record held by
//!   `ln_refcnt` while the mutex is released) -> `nd6_timer` and `nd6_purge` walk with a whole
//!   [`LlinfoNd6`] marker pinned on their stack (`ln_rt` `None`, the C's `ln_rt == NULL` test
//!   of a marker): the record after it is read and the marker steps over it
//!   (`PinList::step_over`, O(1)), the mutex is released with the route's handle alone, and
//!   the walk resumes from the marker, as the C's.
//! - `nd6_resolve`'s `TAILQ_REMOVE` + `TAILQ_INSERT_HEAD` of the record the route owns is
//!   `TaggedLink::move_b_to_head` (one call: the lent record borrows the witness the list's
//!   change needs mutably); `nd6_rtrequest`'s forced garbage collection rotates the list's
//!   last record to its head with `PinList::move_last_to_head` and reads it as the first. Both
//!   O(1), as the C's.
//! - `ifp->if_nd` (`malloc(M_IP6NDP)` of 8 bytes behind a word) -> the embedded
//!   [`NdIfinfoCell`] of every interface (D-6): two atomics, valid all zero (the
//!   `Ifnet`-in-a-zeroed-softc rule), set by `nd6_ifattach` and cleared by `nd6_ifdetach`; the
//!   image `SIOCGIFINFO_IN6` copies out stays `hal::sys::netinet6_nd6::NdIfinfo`. LZ's
//!   "an interface without one" deviation is gone: every interface has it.
//! - `nd6_resolve(ifp, rt, m, dst, desten)` takes the packet (`MbufBox`) and gives it back on
//!   success (`Err(EAGAIN)`: kept on the hold queue; any other error: freed), the destination a
//!   `SockaddrView`; the `Inet` bridge's leg and `bsd`'s `unsafe` over `dst` (decision 82's
//!   second block) go: `ether_resolve` calls it directly. Its solicitation asks the bridge for
//!   the net lock's proof (`Inet::with_net_held`, the leg `ether_resolve`'s broadcast copy
//!   uses: a driver's output carries none). `nd6_rtrequest` (`ether_rtrequest`'s), `nd6_ifattach`
//!   and `nd6_ifdetach` (the interface layer's) and DAD, the expiry timer and `nd6_purge`
//!   (in6's and in6_ifattach's, N6.5a's temporary legs) are called directly.
//! - The held packets an unanswered neighbor drops get their `icmp6_error` through the
//!   temporary `Inet::icmp6_error` leg until ICMPv6 moves (N6.5d).
//! - `nd6_ioctl(cmd, data, ifp)` over the request's bytes (`hal::pod::at`/`put` of the
//!   `struct in6_ndireq` and `struct in6_nbrinfo` images), called by the kernel's `in6_ioctl`.
//! - `nd6_cache_lladdr`'s and `nd6_na_input`'s one section also reads and writes the route's
//!   link-level gateway (`nd6_llchanged`, `nd6_llstore`: the `ReadMostly` cell's own pair), which
//!   the C does under the exclusive net lock alone: one mutex section in all, not one before
//!   and one after them; the log line, the routing message, `rt6_flush` and the output come
//!   after it.
//! - `nd6_timer` reads whether the list is empty at the end of its walk, under the mutex it
//!   holds there, where the C reads `TAILQ_EMPTY` under the net lock alone after letting the
//!   mutex go (the witness the list wants is held then; another pair the C has not, to read it
//!   later, would show nothing: an entry added meanwhile arms the timer itself).
//!
//! **Lock and spl transitions** against the C:
//!
//! | operation | the C | the kernel | above the C |
//! |---|---|---|---|
//! | `nd6_resolve` hit (nd6.c:1274-1308) | `nd6_mtx` for the record and the list move, released before the link address | `ND6_MTX` guard, `move_b_to_head`, `get_b`; the gateway's family, length and address read once before it (`with_gateway`, ARP's shape) | none |
//! | `nd6_resolve` miss (:1310-1354) | `nd6_mtx` then `ln_mq`'s mutex; the NS after both | the same; `ln_mq` the kernel's `MbufQueue` | none |
//! | `nd6_cache_lladdr` (:1013-1200), `nd6_na_input` (nd6_nbr.c:632-700) | the exclusive net lock alone reads the record | the exclusive net lock + one `ND6_MTX` pair to lend the record, the hold queue delisted under it, output after | one mutex pair per call (N6.4b's R5 class; N12 row) |
//! | `nd6_llinfo_timer` (:322-414) | the exclusive net lock alone reads the record | one `ND6_MTX` pair per expired entry, the NS and the ICMPv6 errors after it | one mutex pair per expired entry (the same class) |
//! | `nd6_timer` (:284-302), `nd6_purge` (:509-526) | `nd6_mtx`, dropped around `NET_LOCK` + the entry's timer or free with `rtref` held | the same with the route's `PoolArc` clone held across (ARP's `arptimer`) | none |
//! | `nd6_rtrequest` `RTM_ADD`/`RESOLVE` (:785-863), `RTM_DELETE` (:886-899) | the net lock; `nd6_mtx` for the word and the list; `KERNEL_LOCK` around the proxy's join and leave | the same: `TaggedLink::set_b` under the guard (`Err` = the race lost), `take_b` with the guard borrowed mutably; `klock` C-style | none (`RTM_DELETE`'s proxy leave: `if_maddrlock` read once, the deviations) |
//! | `nd6_invalidate` (:669-679), `nd6_lookup`'s create (:589-593) | `nd6_mtx` once | `ND6_MTX` once | none |
//! | `nd6_ioctl` (:941-985) | `NET_LOCK_SHARED`; `nd6_mtx` for `SIOCGNBRINFO_IN6` | a read witness; `ND6_MTX` once | none |
//! | `nd6_expire`, `nd6_slowtimo` (:465, :1210) | `NET_LOCK` | a write witness | none |
//!
//! **Holders** (`refcnt_take`/`rele`, `rtref`/`rtfree` of the C against the handles here):
//! `ln_refcnt`'s list reference = the route word's box (given to it by `TaggedLink::set_b` in
//! `nd6_rtrequest`, taken back by `TaggedLink::take_b` in `RTM_DELETE` and dropped there); the
//! iterator's reference = none (the walk holds the route, `ln_rt`'s clone, across the mutex's
//! release: the C's `rtref`); `ln_rt` = one route handle per record, dropped with it;
//! `nd6_lookup`'s reference = its `PoolArc` answer, dropped by its caller (`rtfree`).

use core::ffi::c_void;
use core::fmt;
use core::mem::size_of;
use core::pin::{Pin, pin};
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::link::{StaticMutex, Tagged};
use hal::mutex::Mutexed;
use hal::netlock::{NETLOCK, NetHeld};
use hal::pinlist::{PinEntry, PinList};
use hal::pod;
use hal::pool::{BoxPool, PR_NOWAIT, PoolArc, PoolBox, Pooled};
use hal::random::arc4random;
use hal::rwlock::{RW_READ, RW_WRITE};
use hal::sys::endian::{htonl, htons, ntohs};
use hal::sys::errno::Errno;
use hal::sys::net_if::{IFF_MULTICAST, IFT_CARP, IFT_ETHER, IFT_IEEE80211, SockaddrDl};
use hal::sys::net_route::{
    RTAX_DST, RTAX_GATEWAY, RTF_ANNOUNCE, RTF_CACHED, RTF_CLONING, RTF_GATEWAY, RTF_HOST,
    RTF_LLINFO, RTF_LOCAL, RTF_MPLS, RTF_MULTICAST, RTF_REJECT, RTF_STATIC, RTM_ADD, RTM_DELETE,
    RTM_INVALIDATE, RTM_RESOLVE, RTP_CONNECTED,
};
use hal::sys::netinet_icmp6::{
    ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADDR, Icmp6statCounters, ND_NEIGHBOR_SOLICIT,
    ND_OPT_DNSSL, ND_OPT_MTU, ND_OPT_PREFIX_INFORMATION, ND_OPT_RDNSS, ND_OPT_REDIRECTED_HEADER,
    ND_OPT_SOURCE_LINKADDR, ND_OPT_TARGET_LINKADDR, ND_REDIRECT, ND_REDIRECT_ROUTER,
    ND_ROUTER_ADVERT, ND_ROUTER_SOLICIT,
};
use hal::sys::netinet_if_ether::ETHER_ADDR_LEN;
use hal::sys::netinet6_in6::{
    In6Addr, SockaddrIn6, in6_is_addr_linklocal, in6_is_addr_mc_linklocal, in6_is_addr_unspecified,
};
use hal::sys::netinet6_in6_var::{
    IN6_IFF_AUTOCONF, IN6_IFF_DEPRECATED, SIOCGIFINFO_IN6, SIOCGNBRINFO_IN6,
};
use hal::sys::netinet6_nd6::{
    In6Nbrinfo, In6Ndireq, ND6_INFINITE_LIFETIME, ND6_LLINFO_DELAY, ND6_LLINFO_INCOMPLETE,
    ND6_LLINFO_NOSTATE, ND6_LLINFO_PROBE, ND6_LLINFO_PURGE, ND6_LLINFO_REACHABLE, ND6_LLINFO_STALE,
    NdIfinfo,
};
use hal::sys::socket::AF_LINK;

use crate::assert::{kassert, kpanic};
use crate::log::{LOG_DEBUG, LOG_INFO};
use crate::net::if_::{IFNETLIST, IfRef, if_output_ml, ifaof_ifpforaddr, inet, net_tq};
use crate::net::if_ethersubr::{ether_map_ipv6_multicast, ether_sprintf};
use crate::net::if_var::{AtomicCell, Ifnet};
use crate::net::mbuf::{
    M_MCAST, MbufBox, MbufList, MbufQueue, ml_dequeue, ml_len, mq_delist, mq_len, mq_purge, mq_push,
};
use crate::net::proto::SockaddrView;
use crate::net::route::{
    RT_RESOLVE, RtAddrinfo, Rtentry, rt_getll, rtalloc, rtdeletemsg, rtrequest,
};
use crate::netinet::if_ether::{
    SDL_ALEN, gw_alen_family, gw_family_len, gw_sdl_index, gw_set_type_index, if_sadl_copy,
    sdl_lladdr,
};
use crate::netinet::in_pcb::In6AddrCell;
use crate::netinet6::in6::{
    ifa6_is_deprecated, ifa6_is_invalid, in6_addmulti, in6_delmulti, in6_lookupmulti,
    in6_purgeaddr, in6ifa_ifpwithaddr,
};
use crate::netinet6::in6_var::{In6Ifaddr, in6_are_masked_addr_equal};
use crate::netinet6::ip6_var::{IP6_FORWARDING, IP6_NEIGHBORGCTHRESH, icmp6stat_inc};
use crate::netinet6::nd6_nbr::nd6_ns_output;
use crate::netinet6::nd6_rtr::rt6_flush;
use crate::prf::{Str, log, printf};
use crate::task::{Task, task_add};
use crate::tc::{gettime, getuptime};
use crate::timeout::{KCLOCK_NONE, TIMEOUT_PROC, Timeout};

/// `ND6_SLOWTIMER_INTERVAL`: 1 hour.
const ND6_SLOWTIMER_INTERVAL: i32 = 60 * 60;
/// `ND6_RECALC_REACHTM_INTERVAL`: 2 hours.
const ND6_RECALC_REACHTM_INTERVAL: i32 = 60 * 120;
/// `nd6_gctimer`: 1 day: garbage collection timer.
pub const ND6_GCTIMER: i32 = 60 * 60 * 24;

// protocol constants

/// 1sec.
pub const MAX_RTR_SOLICITATION_DELAY: i32 = 1;
/// 4sec.
pub const RTR_SOLICITATION_INTERVAL: i32 = 4;
/// `MAX_RTR_SOLICITATIONS`.
pub const MAX_RTR_SOLICITATIONS: i32 = 3;

/// `LN_HOLD_QUEUE`: packets held per neighbor until resolved.
pub const LN_HOLD_QUEUE: u32 = 10;
/// `LN_HOLD_TOTAL`: packets held in all of the nd6 queues.
pub const LN_HOLD_TOTAL: u32 = 100;

// node constants

/// msec.
pub const REACHABLE_TIME: u32 = 30000;
/// msec.
pub const RETRANS_TIMER: u32 = 1000;
/// 1024 * 0.5.
pub const MIN_RANDOM_FACTOR: u32 = 512;
/// 1024 * 1.5.
pub const MAX_RANDOM_FACTOR: u32 = 1536;

/// `M_IP6NDP` (`<sys/malloc.h>`): the `malloc(9)` type of ND's records (DAD's `struct dadq`).
pub const M_IP6NDP: i32 = 124;

/// The size of a `sockaddr_in6`.
const SIN6_SIZE: usize = size_of::<SockaddrIn6>();

/// `struct nd_ifinfo` of an interface (`ifp->if_nd`), embedded in it (D-6): \[N\] both, as
/// the C's, atomics so that the image `SIOCGIFINFO_IN6` copies is read whole under a shared
/// net lock. Valid all zero (an interface `nd6_ifattach` has not seen).
pub struct NdIfinfoCell {
    /// \[N\] `reachable`: Reachable Time.
    reachable: AtomicU32,
    /// \[N\] `recalctm`: BaseReachable recalc timer.
    recalctm: AtomicI32,
}

impl NdIfinfoCell {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            reachable: AtomicU32::new(0),
            recalctm: AtomicI32::new(0),
        }
    }

    /// `*ifp->if_nd`: the image.
    #[inline]
    pub fn get(&self) -> NdIfinfo {
        NdIfinfo {
            reachable: self.reachable.load(Ordering::Relaxed),
            recalctm: self.recalctm.load(Ordering::Relaxed),
        }
    }

    /// `ifp->if_nd->reachable`.
    #[inline]
    pub fn reachable(&self) -> u32 {
        self.reachable.load(Ordering::Relaxed)
    }

    /// Stores `v` (under the net lock).
    #[inline]
    fn set(&self, v: NdIfinfo) {
        self.reachable.store(v.reachable, Ordering::Relaxed);
        self.recalctm.store(v.recalctm, Ordering::Relaxed);
    }
}

impl Default for NdIfinfoCell {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for NdIfinfoCell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

/// `struct llinfo_nd6`: the link-level information of an `AF_INET6` route (a neighbor cache
/// entry), an item of the pool "nd6" owned by the route's `rt_llinfo` (the word's second kind).
/// Locks: \[m\] `nd6_mtx`, \[I\] immutable after creation. A walk's marker is one with no route
/// (the C's `struct llinfo_nd6_iterator`, whose first two members are these).
pub struct LlinfoNd6 {
    /// \[m\] `ln_list`: global `nd6_list`.
    pub(crate) ln_list: PinEntry<LlinfoNd6>,
    /// \[I\] `ln_rt`: the route (a counted handle; `None` for an iterator).
    pub(crate) ln_rt: Option<PoolArc<Rtentry>>,
    /// `ln_mq`: hold packets until resolved (its own mutex, inside `nd6_mtx`).
    pub(crate) ln_mq: MbufQueue,
    /// \[m\] `ln_saddr6`: source of prompting packet.
    pub(crate) ln_saddr6: In6AddrCell,
    /// \[m\] `ln_asked`: number of queries already sent for addr.
    pub(crate) ln_asked: AtomicCell<i64>,
    /// \[m\] `ln_state`: reachability state.
    pub(crate) ln_state: AtomicCell<i16>,
    /// \[m\] `ln_router`: 2^0: ND6 router bit.
    pub(crate) ln_router: AtomicCell<i16>,
}

impl LlinfoNd6 {
    /// A record of `rt` (`pool_get(&nd6_pool, PR_NOWAIT | PR_ZERO)`, `refcnt_init`, `mq_init`
    /// and `ln_rt`), or a walk's marker with none.
    fn new(rt: Option<PoolArc<Rtentry>>) -> Self {
        Self {
            ln_list: PinEntry::new(),
            ln_rt: rt,
            ln_mq: MbufQueue::new(LN_HOLD_QUEUE, IPL_SOFTNET),
            ln_saddr6: In6AddrCell::new(),
            ln_asked: AtomicCell::new(),
            ln_state: AtomicCell::new(),
            ln_router: AtomicCell::new(),
        }
    }

    /// `ln_state`.
    pub fn ln_state(&self) -> i16 {
        self.ln_state.get()
    }

    /// `ln_asked`.
    pub fn ln_asked(&self) -> i64 {
        self.ln_asked.get()
    }

    /// `ln_router`.
    pub fn ln_router(&self) -> i16 {
        self.ln_router.get()
    }

    /// `ln_saddr6`.
    pub fn ln_saddr6(&self) -> In6Addr {
        self.ln_saddr6.get::<In6Addr>()
    }

    /// `mq_len(&ln->ln_mq)`: the packets held.
    pub fn held(&self) -> u32 {
        mq_len(&self.ln_mq)
    }

    /// `ND6_LLINFO_PERMANENT(ln)`: whether the entry's route never expires (`panic(9)` for a
    /// walk's marker, which has none: the C would dereference NULL).
    pub(crate) fn permanent(&self) -> bool {
        match &self.ln_rt {
            Some(rt) => rt.rt_expire().get() == 0,
            None => kpanic(format_args!("ND6_LLINFO_PERMANENT: no route")),
        }
    }
}

impl Pooled for LlinfoNd6 {
    const POOL: &'static BoxPool<LlinfoNd6> = &ND6_POOL;
}

impl Tagged for LlinfoNd6 {
    const LOCK: StaticMutex = StaticMutex::of(&ND6_MTX);
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(llinfo_nd6_head, llinfo_nd6)` through `ln_list`: `nd6_list`.
    pub LnList: LlinfoNd6, ln_list
);

/// `struct nd_opts`: the options `nd6_options` found in a message, as their offsets in the
/// options it was given (the C's pointers into the packet).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NdOpts {
    /// `nd_opts_src_lladdr`.
    pub nd_opts_src_lladdr: Option<usize>,
    /// `nd_opts_tgt_lladdr`.
    pub nd_opts_tgt_lladdr: Option<usize>,
}

/// An IPv6 address printed as `inet_ntop(AF_INET6, ...)` writes it: lower-case hex words,
/// the first longest run of two or more zero words as `::`, and the IPv4 tail of a
/// compatible or mapped address as a dotted quad (see the module's deviations).
pub struct In6Ntop(pub In6Addr);

impl fmt::Display for In6Ntop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0.s6_addr;
        let words: [u16; 8] =
            core::array::from_fn(|i| u16::from_be_bytes([b[2 * i], b[2 * i + 1]]));

        // Find the longest run of 0x00's in words[] for :: shorthanding.
        let mut best: Option<(usize, usize)> = None;
        let mut cur: Option<(usize, usize)> = None;
        for (i, w) in words.iter().enumerate() {
            if *w == 0 {
                cur = Some(match cur {
                    None => (i, 1),
                    Some((base, len)) => (base, len + 1),
                });
            } else if let Some(c) = cur.take()
                && best.is_none_or(|b| c.1 > b.1)
            {
                best = Some(c);
            }
        }
        if let Some(c) = cur
            && best.is_none_or(|b| c.1 > b.1)
        {
            best = Some(c);
        }
        let best = best.filter(|b| b.1 >= 2);

        for (i, w) in words.iter().enumerate() {
            // Are we inside the best run of 0x00's?
            if let Some((base, len)) = best
                && i >= base
                && i < base + len
            {
                if i == base {
                    f.write_str(":")?;
                }
                continue;
            }
            // Are we following an initial run of 0x00s or any real hex?
            if i != 0 {
                f.write_str(":")?;
            }
            // Is this address an encapsulated IPv4?
            if i == 6
                && let Some((0, len)) = best
                && (len == 6 || (len == 5 && words[5] == 0xffff))
            {
                return write!(f, "{}.{}.{}.{}", b[12], b[13], b[14], b[15]);
            }
            write!(f, "{w:x}")?;
        }
        // Was it a trailing run of 0x00's?
        if let Some((base, len)) = best
            && base + len == 8
        {
            f.write_str(":")?;
        }
        Ok(())
    }
}

/// \[N\] `nd6_timer_next`: at which uptime `nd6_timer` runs.
static ND6_TIMER_NEXT: AtomicI64 = AtomicI64::new(-1);
/// `nd6_expire_next`: at which uptime `nd6_expire` runs.
static ND6_EXPIRE_NEXT: AtomicI64 = AtomicI64::new(-1);

/// \[a\] `nd6_delay`: delay first probe time 5 second (`net.inet6.icmp6.nd6_delay`).
pub static ND6_DELAY: AtomicI32 = AtomicI32::new(5);
/// \[a\] `nd6_umaxtries`: maximum unicast query (`net.inet6.icmp6.nd6_umaxtries`).
pub static ND6_UMAXTRIES: AtomicI32 = AtomicI32::new(3);
/// \[a\] `nd6_mmaxtries`: maximum multicast query (`net.inet6.icmp6.nd6_mmaxtries`).
pub static ND6_MMAXTRIES: AtomicI32 = AtomicI32::new(3);

/// `nd6_maxndopt`: max # of ND options allowed (preventing too many loops in ND option
/// parsing).
static ND6_MAXNDOPT: AtomicI32 = AtomicI32::new(10);

/// `nd6_mtx`: llinfo_nd6 live time, `rt_llinfo` and `RTF_LLINFO` are protected by it; its
/// guard lends a route's record (`TaggedLink::get_b`) and walks [`ND6_LIST`]. Its data is
/// \[m\] `nd6_inuse`: limit neighbor discovery routes.
pub static ND6_MTX: Mutexed<i32> = Mutexed::new(IPL_SOFTNET, 0);

/// \[m\] `nd6_list`: list of `llinfo_nd6` structures.
static ND6_LIST: PinList<LnList> = PinList::new();

/// \[I\] `nd6_pool`: pool for `llinfo_nd6` structures.
static ND6_POOL: BoxPool<LlinfoNd6> = BoxPool::new();

/// \[a\] `ln_hold_total`: packets currently in the nd6 queue (`net.inet6.icmp6.nd6_queued`).
#[allow(non_upper_case_globals)] // the C name; `LN_HOLD_TOTAL` is the limit beside it
pub static ln_hold_total: AtomicU32 = AtomicU32::new(0);

/// `nd6_timer_to`.
static ND6_TIMER_TO: Timeout =
    Timeout::new_flags(nd6_timer, ptr::null_mut(), KCLOCK_NONE, TIMEOUT_PROC);
/// `nd6_slowtimo_ch`.
static ND6_SLOWTIMO_CH: Timeout =
    Timeout::new_flags(nd6_slowtimo, ptr::null_mut(), KCLOCK_NONE, TIMEOUT_PROC);
/// `nd6_expire_timeout`.
static ND6_EXPIRE_TIMEOUT: Timeout = Timeout::new(nd6_expire_timer, ptr::null_mut());
/// `nd6_expire_task`.
static ND6_EXPIRE_TASK: Task = Task::new(nd6_expire, ptr::null_mut());

/// `ND_COMPUTE_RTIME(x)`: a random reachable time around `x` milliseconds (between 0.5 and
/// 1.5 times it), in seconds.
pub fn nd_compute_rtime(x: u32) -> u32 {
    (MIN_RANDOM_FACTOR * (x >> 10)
        + (arc4random() & ((MAX_RANDOM_FACTOR - MIN_RANDOM_FACTOR) * (x >> 10))))
        / 1000
}

/// `satosin6(rt_key(rt))->sin6_addr`: the destination of an inet6 route.
pub fn rt_key_in6(rt: &Rtentry) -> In6Addr {
    // The key of a route in an inet6 table is a `sockaddr_in6`: its address at offset 8.
    pod::at::<In6Addr>(rt.dest(), 8).unwrap_or_default()
}

/// `NET_ASSERT_LOCKED()`.
#[inline]
fn net_assert_locked() {
    kassert!(matches!(hal::netlock::status(), RW_WRITE | RW_READ));
}

/// `NET_ASSERT_LOCKED_EXCLUSIVE()`.
#[inline]
fn net_assert_locked_exclusive() {
    kassert!(hal::netlock::status() == RW_WRITE);
}

/// `icmp6_error(m, type, code, param)`: through the `Inet` leg until ICMPv6 moves (N6.5d);
/// the packet freed without it.
fn icmp6_error(m: MbufBox, type_: u8, code: u8, param: i32) {
    match inet() {
        Some(i) => i.icmp6_error(m, type_, code, param),
        None => drop(m),
    }
}

/// `nd6_init`: the neighbor cache pool, and the slow timer started (the timeouts and the
/// expiry task are set by their `static`s).
pub fn nd6_init() {
    let _ = ND6_POOL.init(IPL_SOFTNET, 0, "nd6");

    // start timer
    let _ = Pin::static_ref(&ND6_SLOWTIMO_CH).add_sec(ND6_SLOWTIMER_INTERVAL);
}

/// `nd6_ifattach(ifp)`: the Neighbor Discovery information of `ifp`, its random reachable
/// time.
pub fn nd6_ifattach(ifp: &Ifnet) {
    ifp.if_nd.set(NdIfinfo {
        reachable: nd_compute_rtime(REACHABLE_TIME),
        recalctm: 0,
    });
}

/// `nd6_ifdetach(ifp)`: the C frees the record; here it is cleared, as a fresh interface's.
pub fn nd6_ifdetach(ifp: &Ifnet) {
    ifp.if_nd.set(NdIfinfo::default());
}

/// `nd6_options(opt, icmp6len, ndopts)`: parses multiple ND options in `opt` (the rest of the
/// message) into `ndopts`; `false` for an invalid option (the C's -1). This function is much
/// easier to use, for ND routines that do not need multiple options of the same type.
pub fn nd6_options(opt: &[u8], ndopts: &mut NdOpts) -> bool {
    *ndopts = NdOpts::default();

    if opt.is_empty() {
        return true;
    }

    let last_opt = opt.len();
    let mut next_opt = Some(0usize);
    let mut i = 0;

    while let Some(off) = next_opt {
        // make sure nd_opt_len is inside the buffer
        let Some(&len) = opt.get(off + 1) else {
            return nd6_options_invalid(ndopts);
        };

        // every option must have a length greater than zero
        let olen = usize::from(len) << 3;
        if olen == 0 {
            return nd6_options_invalid(ndopts);
        }

        let next = off + olen;
        if next > last_opt {
            // option overruns the end of buffer
            return nd6_options_invalid(ndopts);
        }
        // reached the end of options chain at `last_opt`
        next_opt = (next != last_opt).then_some(next);

        match opt[off] {
            ND_OPT_SOURCE_LINKADDR => {
                if ndopts.nd_opts_src_lladdr.is_none() {
                    ndopts.nd_opts_src_lladdr = Some(off);
                }
            }
            ND_OPT_TARGET_LINKADDR => {
                if ndopts.nd_opts_tgt_lladdr.is_none() {
                    ndopts.nd_opts_tgt_lladdr = Some(off);
                }
            }
            ND_OPT_MTU
            | ND_OPT_REDIRECTED_HEADER
            | ND_OPT_PREFIX_INFORMATION
            | ND_OPT_DNSSL
            | ND_OPT_RDNSS => {
                // Don't warn, not used by kernel
            }
            _ => {
                // Unknown options must be silently ignored, to accommodate future extension
                // to the protocol.
            }
        }

        i += 1;
        if i > ND6_MAXNDOPT.load(Ordering::Relaxed) {
            icmp6stat_inc(Icmp6statCounters::Icp6sNdToomanyopt);
            break;
        }
    }

    true
}

/// `nd6_options`'s `invalid:` label.
fn nd6_options_invalid(ndopts: &mut NdOpts) -> bool {
    *ndopts = NdOpts::default();
    icmp6stat_inc(Icmp6statCounters::Icp6sNdBadopt);
    false
}

/// The link-layer address the option at offset `off` of `opt` carries, as `nd6_options`
/// found it there: the bytes after its header (`(char *)(opt + 1)`), and the C's `lladdrlen`
/// (the whole option's length, `nd_opt_len << 3`). Bounded by `opt` (an offset no option of
/// it starts at gives what fits).
pub fn nd6_opt_lladdr(opt: &[u8], off: usize) -> (&[u8], i32) {
    let len = opt.get(off + 1).map_or(0, |l| usize::from(*l) << 3);
    let end = (off + len).min(opt.len());
    (opt.get(off + 2..end).unwrap_or(&[]), len as i32)
}

/// `nd6_llinfo_settimer(ln, secs)`: ND6 timer routine to handle ND6 entries: (re)arms the
/// neighbor cache timer of `ln` in `secs` seconds. Under the net lock (and the mutex that lent
/// `ln`).
pub fn nd6_llinfo_settimer(ln: &LlinfoNd6, secs: u32) {
    let expire = getuptime() + i64::from(secs);

    net_assert_locked();
    let Some(rt) = ln.ln_rt.as_deref() else {
        kpanic(format_args!("nd6_llinfo_settimer: an iterator"));
    };
    kassert!(!rt.rt_flags.isset(RTF_LOCAL));

    rt.rt_expire().set(expire);
    if !ND6_TIMER_TO.pending() || expire < ND6_TIMER_NEXT.load(Ordering::Relaxed) {
        ND6_TIMER_NEXT.store(expire, Ordering::Relaxed);
        let _ = Pin::static_ref(&ND6_TIMER_TO).add_sec(secs as i32);
    }
}

/// `nd6_timer`: the neighbor cache timeout: runs the state machine of every entry whose timer
/// expired, then rearms itself for the next one.
fn nd6_timer(_unused: *mut c_void) {
    let marker = pin!(LlinfoNd6::new(None));
    let marker: Pin<&LlinfoNd6> = marker.as_ref();
    let i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;

    let uptime = getuptime();
    let mut expire = uptime + i64::from(ND6_GCTIMER);

    let mut g = ND6_MTX.lock();
    ND6_LIST.insert_head(&mut g, marker);
    loop {
        // The record after the marker: its route's handle when its timer expired (the C's
        // `rtref`), the earliest pending expiry noted otherwise; a marker of another walk has
        // no route and is stepped over.
        let due = match ND6_LIST.next(&g, marker.get_ref()) {
            None => break,
            Some(ln) => match &ln.ln_rt {
                None => None,
                Some(rt) => {
                    let e = rt.rt_expire().get();
                    if e != 0 && e <= uptime {
                        Some(rt.clone())
                    } else {
                        if e != 0 && e < expire {
                            expire = e;
                        }
                        None
                    }
                }
            },
        };
        let _ = ND6_LIST.step_over(&mut g, marker.get_ref());
        let Some(rt) = due else {
            continue;
        };
        drop(g);
        {
            let w = NETLOCK.write();
            if let Some(net) = NetHeld::of(&w)
                && !nd6_llinfo_timer(&rt, i_am_router, &net)
            {
                let e = rt.rt_expire().get();
                if e != 0 && e < expire {
                    expire = e;
                }
            }
        }
        drop(rt); // rtfree
        g = ND6_MTX.lock();
    }
    ND6_LIST.remove(&mut g, marker.get_ref());
    // TAILQ_EMPTY(&nd6_list), read while the mutex is held (the module's redesign).
    let listed = !ND6_LIST.is_empty(&g);
    drop(g);

    let secs = (expire - uptime).max(1) as i32;

    let _w = NETLOCK.write();
    if listed {
        ND6_TIMER_NEXT.store(uptime + i64::from(secs), Ordering::Relaxed);
        let _ = Pin::static_ref(&ND6_TIMER_TO).add_sec(secs);
    }
}

/// What `nd6_llinfo_timer` does once it let the mutex go.
enum TimerStep {
    /// Nothing more.
    Done,
    /// A solicitation, to `daddr6` (`None`: the solicited-node group) from `saddr6`.
    Solicit(Option<In6Addr>, In6Addr),
    /// The held packets dropped with an ICMPv6 error each, then the entry freed (its router
    /// bit with it).
    Drop(MbufList, bool),
    /// The entry freed, its router bit with it.
    Free(bool),
}

/// `nd6_llinfo_timer(rt, i_am_router)`: ND timer state handling. Returns `true` if `rt` should
/// no longer be used. Under the exclusive net lock (`net`); the record under one `ND6_MTX`
/// section (the module's lock table), the solicitation and the errors after it.
pub fn nd6_llinfo_timer(rt: &PoolArc<Rtentry>, i_am_router: bool, net: &NetHeld<'_>) -> bool {
    net_assert_locked_exclusive();

    // might have been freed between leave nd6_mtx and enter net lock
    if !rt.rt_flags.isset(RTF_LLINFO) {
        return false;
    }

    let Some(ifp) = IfRef::get(rt.rt_ifidx.get()) else {
        return true;
    };
    let dst = rt_key_in6(rt);

    let step = {
        let g = ND6_MTX.lock();
        // RTF_LLINFO and the record are set and cleared together under the mutex.
        let Some(ln) = rt.rt_llinfo.get_b(&g) else {
            return false;
        };
        let router = ln.ln_router.get() != 0;
        match ln.ln_state.get() {
            ND6_LLINFO_INCOMPLETE => {
                if ln.ln_asked.get() < i64::from(ND6_MMAXTRIES.load(Ordering::Relaxed)) {
                    ln.ln_asked.set(ln.ln_asked.get() + 1);
                    nd6_llinfo_settimer(ln, RETRANS_TIMER / 1000);
                    TimerStep::Solicit(None, ln.ln_saddr6())
                } else {
                    TimerStep::Drop(mq_delist(&ln.ln_mq), router)
                }
            }

            ND6_LLINFO_REACHABLE => {
                if !ln.permanent() {
                    ln.ln_state.set(ND6_LLINFO_STALE);
                    nd6_llinfo_settimer(ln, ND6_GCTIMER as u32);
                }
                TimerStep::Done
            }

            ND6_LLINFO_STALE | ND6_LLINFO_PURGE => {
                // Garbage Collection(RFC 2461 5.3)
                if ln.permanent() {
                    TimerStep::Done
                } else {
                    TimerStep::Free(router)
                }
            }

            ND6_LLINFO_DELAY => {
                // We need NUD
                ln.ln_asked.set(1);
                ln.ln_state.set(ND6_LLINFO_PROBE);
                nd6_llinfo_settimer(ln, RETRANS_TIMER / 1000);
                TimerStep::Solicit(Some(dst), ln.ln_saddr6())
            }

            ND6_LLINFO_PROBE => {
                if ln.ln_asked.get() < i64::from(ND6_UMAXTRIES.load(Ordering::Relaxed)) {
                    ln.ln_asked.set(ln.ln_asked.get() + 1);
                    nd6_llinfo_settimer(ln, RETRANS_TIMER / 1000);
                    TimerStep::Solicit(Some(dst), ln.ln_saddr6())
                } else {
                    TimerStep::Free(router)
                }
            }

            _ => TimerStep::Done,
        }
    };

    match step {
        TimerStep::Done => false,
        TimerStep::Solicit(daddr6, saddr6) => {
            nd6_ns_output(ifp.ifp(), daddr6.as_ref(), &dst, Some(&saddr6), false, net);
            false
        }
        TimerStep::Drop(mut ml, router) => {
            let len = ml_len(&ml);
            while let Some(m) = ml_dequeue(&mut ml) {
                // Fake rcvif to make the ICMP error more helpful in diagnosing for the
                // receiver.
                // XXX: should we consider older rcvif?
                m.m_pkthdr().ph_ifidx.set(rt.rt_ifidx.get());

                icmp6_error(m, ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADDR, 0);
            }
            // XXXSMP we also discard if other CPU enqueues: a packet back in the queue is
            // purged and counted by nd6_free's nd6_invalidate, below (the C purges it here,
            // and nd6_invalidate finds the queue empty then).
            ln_hold_total.fetch_sub(len, Ordering::Relaxed);

            nd6_free(rt, ifp.ifp(), i_am_router, router, net);
            true
        }
        TimerStep::Free(router) => {
            nd6_free(rt, ifp.ifp(), i_am_router, router, net);
            true
        }
    }
}

/// `nd6_expire_timer_update(ia6)`: rearms the address lifetime timer for `ia6`.
pub fn nd6_expire_timer_update(ia6: &In6Ifaddr) {
    let lt = ia6.ia6_lifetime.get();
    let mut expire_time = i64::MAX;

    if lt.ia6t_vltime != ND6_INFINITE_LIFETIME {
        expire_time = lt.ia6t_expire;
    }

    if ia6.ia6_flags.get() & IN6_IFF_DEPRECATED == 0
        && lt.ia6t_pltime != ND6_INFINITE_LIFETIME
        && expire_time > lt.ia6t_preferred
    {
        expire_time = lt.ia6t_preferred;
    }

    if expire_time == i64::MAX {
        return;
    }

    // IFA6_IS_INVALID() and IFA6_IS_DEPRECATED() check for uptime greater than ia6t_expire or
    // ia6t_preferred, not greater or equal. Schedule timeout one second later so that either
    // IFA6_IS_INVALID() or IFA6_IS_DEPRECATED() is true.
    expire_time += 1;

    if !ND6_EXPIRE_TIMEOUT.pending() || ND6_EXPIRE_NEXT.load(Ordering::Relaxed) > expire_time {
        // The C's `int secs = expire_time - getuptime()`, then `if (secs < 0) secs = 0`: the
        // difference truncated to an int first, then clamped, so a lifetime past INT_MAX
        // seconds (root's SIOCAIFADDR_IN6 may give 3000000000) schedules at 0, never a
        // negative `timeout_add_sec`.
        let secs = ((expire_time - getuptime()) as i32).max(0);

        let _ = Pin::static_ref(&ND6_EXPIRE_TIMEOUT).add_sec(secs);
        ND6_EXPIRE_NEXT.store(expire_time, Ordering::Relaxed);
    }
}

/// `nd6_expire`: expire interface addresses.
fn nd6_expire(_unused: *mut c_void) {
    let w = NETLOCK.write();
    let Some(net) = NetHeld::of(&w) else {
        return;
    };

    for ifp in IFNETLIST.iter() {
        // TAILQ_FOREACH_SAFE: the walk reads each successor first (in6_purgeaddr removes the
        // address it is given).
        for ifa in ifp.if_addrlist.iter() {
            let Some(ia6) = In6Ifaddr::of(ifa) else {
                continue;
            };
            // check address lifetime
            if ifa6_is_invalid(ia6) {
                in6_purgeaddr(&ia6.ia_ifa, &net);
            } else {
                if ifa6_is_deprecated(ia6) {
                    ia6.ia6_flags.set(ia6.ia6_flags.get() | IN6_IFF_DEPRECATED);
                }
                nd6_expire_timer_update(ia6);
            }
        }
    }
}

/// `nd6_expire_timer`: the address lifetime timeout: runs `nd6_expire` on the network task
/// queue.
fn nd6_expire_timer(_unused: *mut c_void) {
    if let Some(tq) = net_tq(0) {
        let _ = task_add(tq, Pin::static_ref(&ND6_EXPIRE_TASK));
    }
}

/// `nd6_purge(ifp)`: nuke neighbor cache/prefix/default router management table, right before
/// `ifp` goes away. Under the exclusive net lock (`net`).
pub fn nd6_purge(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    let marker = pin!(LlinfoNd6::new(None));
    let marker: Pin<&LlinfoNd6> = marker.as_ref();
    let i_am_router = IP6_FORWARDING.load(Ordering::Relaxed) != 0;

    // Nuke neighbor cache entries for the ifp.
    let mut g = ND6_MTX.lock();
    ND6_LIST.insert_head(&mut g, marker);
    loop {
        // A record of the interface (its link-level gateway's index): its route's handle and
        // its router bit, for nd6_free once the mutex is let go.
        let ours = match ND6_LIST.next(&g, marker.get_ref()) {
            None => break,
            Some(ln) => ln
                .ln_rt
                .as_ref()
                .filter(|rt| gw_sdl_index(rt) == Some(ifp.if_index.get()))
                .map(|rt| (rt.clone(), ln.ln_router.get() != 0)),
        };
        let _ = ND6_LIST.step_over(&mut g, marker.get_ref());
        let Some((rt, router)) = ours else {
            continue;
        };
        drop(g);
        nd6_free(&rt, ifp, i_am_router, router, net);
        drop(rt); // rtfree
        g = ND6_MTX.lock();
    }
    ND6_LIST.remove(&mut g, marker.get_ref());
}

/// The `sockaddr_in6` image of `addr`, for the routes' requests.
#[inline]
fn sin6_bytes(addr: &In6Addr) -> [u8; SIN6_SIZE] {
    let mut b = [0u8; SIN6_SIZE];
    let _ = pod::put(&mut b, 0, &SockaddrIn6::with_addr(*addr));
    b
}

/// `nd6_lookup(addr6, create, ifp, rtableid)`: the neighbor cache entry of `addr6` (on `ifp`
/// when given) in `rtableid`, with a reference; with `create`, a host route made for it when
/// there is none. Under the net lock (`net`).
pub fn nd6_lookup(
    addr6: &In6Addr,
    create: bool,
    ifp: Option<&Ifnet>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Option<PoolArc<Rtentry>> {
    let sin6 = sin6_bytes(addr6);
    let flags = if create { RT_RESOLVE } else { 0 };

    let dst = SockaddrView::new(&sin6)?;
    let mut rt = rtalloc(dst, flags, rtableid, net);
    if let Some(r) = &rt
        && !r.rt_flags.isset(RTF_LLINFO)
    {
        // This is the case for the default route. If we want to create a neighbor cache for
        // the address, we should free the route for the destination and allocate an interface
        // route.
        if create {
            rt = None;
        }
    }
    let rt = match rt {
        Some(r) => r,
        None => {
            let ifp = ifp.filter(|_| create)?;
            // If no route is available and create is set, we allocate a host route for the
            // destination and treat it like an interface route. This hack is necessary for a
            // neighbor which can't be covered by our own prefix.
            let ifa = ifaof_ifpforaddr(net, &sin6, ifp)?;

            // Create a new route. RTF_LLINFO is necessary to create a Neighbor Cache entry for
            // the destination in nd6_rtrequest which will be called in rtrequest.
            let mut sdl = [0u8; size_of::<SockaddrDl>()];
            let _ = pod::put(&mut sdl, 0, &if_sadl_copy(ifp));
            let mut info = RtAddrinfo::new();
            info.rti_ifa = Some(ifa);
            info.rti_flags = RTF_HOST | RTF_LLINFO;
            info.rti_info[RTAX_DST] = Some(dst);
            info.rti_info[RTAX_GATEWAY] = SockaddrView::new(&sdl);
            let mut nrt = None;
            if rtrequest(
                RTM_ADD,
                &mut info,
                RTP_CONNECTED,
                Some(&mut nrt),
                rtableid,
                net,
            )
            .is_err()
            {
                return None;
            }
            let r = nrt?;
            {
                let g = ND6_MTX.lock();
                if let Some(ln) = r.rt_llinfo.get_b(&g) {
                    ln.ln_state.set(ND6_LLINFO_NOSTATE);
                }
            }
            r
        }
    };
    // Validation for the entry. Note that the check for rt_llinfo is necessary because a
    // cloned route from a parent route that has the L flag (e.g. the default route to a p2p
    // interface) may have the flag, too, while the destination is not actually a neighbor.
    if rt.rt_flags.isset(RTF_GATEWAY)
        || !rt.rt_flags.isset(RTF_LLINFO)
        || gw_family_len(&rt).0 != AF_LINK
        || !rt.rt_llinfo.is_set()
        || ifp.is_some_and(|ifp| rt.rt_ifidx.get() != ifp.if_index.get())
    {
        return None; // rtfree(rt): the handle's drop
    }
    Some(rt)
}

/// `nd6_is_addr_neighbor(addr, ifp)`: whether a given IPv6 address identifies a neighbor on a
/// given link. XXX: should take care of the destination of a p2p link? Under the net lock
/// (`net`).
pub fn nd6_is_addr_neighbor(addr: &SockaddrIn6, ifp: &Ifnet, net: &NetHeld<'_>) -> bool {
    // A link-local address is always a neighbor.
    // XXX: we should use the sin6_scope_id field rather than the embedded interface index.
    // XXX: a link does not necessarily specify a single interface.
    if in6_is_addr_linklocal(&addr.sin6_addr)
        && u32::from(ntohs(addr.sin6_addr.s6_addr16(1))) == ifp.if_index.get()
    {
        return true;
    }

    for ia6 in ifp.if_addrlist.iter().filter_map(In6Ifaddr::of) {
        // Prefix check down below.
        if ia6.ia6_flags.get() & IN6_IFF_AUTOCONF != 0 {
            continue;
        }

        if in6_are_masked_addr_equal(
            &addr.sin6_addr,
            &ia6.ia_addr.addr(),
            &ia6.ia_prefixmask.addr(),
        ) {
            return true;
        }
    }

    // Even if the address matches none of our addresses, it might be in the neighbor cache.
    nd6_lookup(&addr.sin6_addr, false, Some(ifp), ifp.if_rdomain.get(), net).is_some()
}

/// `nd6_invalidate(rt)`: forgets the link-layer address of neighbor route `rt` and its held
/// packets; the entry goes back to INCOMPLETE.
pub fn nd6_invalidate(rt: &Rtentry) {
    let g = ND6_MTX.lock();
    let Some(ln) = rt.rt_llinfo.get_b(&g) else {
        return;
    };
    ln_hold_total.fetch_sub(mq_purge(&ln.ln_mq), Ordering::Relaxed);
    // A neighbor route's gateway is a `sockaddr_dl` (`nd6_rtrequest` checked it): its address
    // forgotten.
    rt.update_gateway(|g| {
        if let Some(g) = g {
            g[SDL_ALEN] = 0;
        }
    });
    ln.ln_state.set(ND6_LLINFO_INCOMPLETE);
    ln.ln_asked.set(0);
}

/// `nd6_free(rt, ifp, i_am_router)`: free an nd6 llinfo entry: forgets the neighbor and
/// deletes its route. `router` is the entry's `ln_router`, which its caller read under the
/// mutex (false without a record: the module's deviations). Under the exclusive net lock
/// (`net`).
pub fn nd6_free(
    rt: &PoolArc<Rtentry>,
    ifp: &'static Ifnet,
    i_am_router: bool,
    router: bool,
    net: &NetHeld<'_>,
) {
    let in6 = rt_key_in6(rt);

    net_assert_locked_exclusive();

    if !i_am_router && router {
        // rt6_flush must be called whether or not the neighbor is in the Default Router List.
        // See a corresponding comment in nd6_na_input().
        let _ = rt6_flush(&in6, ifp, net);
    }

    kassert!(!rt.rt_flags.isset(RTF_LOCAL));
    nd6_invalidate(rt);

    // Detach the route from the routing tree and the list of neighbor caches, and disable the
    // route entry not to be used in already cached routes.
    if !rt.rt_flags.isset(RTF_STATIC | RTF_CACHED) {
        let _ = rtdeletemsg(rt, ifp, ifp.if_rdomain.get(), net);
    }
}

/// The solicited-node multicast address of `addr` with the interface index of `ifp` embedded
/// (`ff02:<ifidx>::1:ffXX:XXXX`), as `nd6_rtrequest` computes it inline.
fn nd6_llsol(addr: &In6Addr, ifp: &Ifnet) -> In6Addr {
    let mut llsol = *addr;
    llsol.set_s6_addr16(0, htons(0xff02));
    llsol.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
    llsol.set_s6_addr32(1, 0);
    llsol.set_s6_addr32(2, htonl(1));
    llsol.s6_addr[12] = 0xff;
    llsol
}

/// `nd6_rtrequest(ifp, req, rt)`: the `AF_INET6` part of `ether_rtrequest`: sets up and tears
/// down the neighbor cache entries of routes. Under the net lock (`net`).
pub fn nd6_rtrequest(ifp: &'static Ifnet, req: u8, rt: &Rtentry, net: &NetHeld<'_>) {
    if rt.rt_flags.isset(RTF_GATEWAY | RTF_MULTICAST | RTF_MPLS) {
        return;
    }

    if !nd6_need_cache(ifp) && !rt.rt_flags.isset(RTF_HOST) {
        // This is probably an interface direct route for a link which does not need neighbor
        // caches (e.g. fe80::%lo0/64). We do not need special treatment below for such a
        // route. Moreover, the RTF_LLINFO flag which would be set below would annoy the
        // ndp(8) command.
        return;
    }

    if req == RTM_RESOLVE && !nd6_need_cache(ifp) {
        // For routing daemons like ospf6d we allow neighbor discovery based on the cloning
        // route only. This allows us to send packets directly into a network without having an
        // address with matching prefix on the interface. If the cloning route is used for an
        // 6to4 interface, we would mistakenly make a neighbor cache for the host route, and
        // would see strange neighbor solicitation for the corresponding destination. In order
        // to avoid confusion, we check if the interface is suitable for neighbor discovery,
        // and stop the process if not. Additionally, we remove the LLINFO flag so that ndp(8)
        // will not try to get the neighbor information of the destination.
        rt.rt_flags.clear(RTF_LLINFO);
        return;
    }

    match req {
        RTM_ADD | RTM_RESOLVE => {
            if req == RTM_ADD {
                if rt.rt_flags.isset(RTF_CLONING) {
                    rt.rt_expire().set(0);
                    return;
                }
                if rt.rt_flags.isset(RTF_LOCAL) && !rt.rt_llinfo.is_set() {
                    rt.rt_expire().set(0);
                }
            }
            // FALLTHROUGH (RTM_RESOLVE)
            nd6_rtrequest_add(ifp, req, rt, net);
        }
        RTM_DELETE => {
            let la = {
                let mut g = ND6_MTX.lock();
                let Some(ln) = rt.rt_llinfo.take_b(&mut g) else {
                    // we lost the race, another thread has removed it
                    return;
                };
                *g -= 1;
                ND6_LIST.remove(&mut g, &ln);
                rt.rt_expire().set(0);
                rt.rt_flags.clear(RTF_LLINFO);
                ln_hold_total.fetch_sub(mq_purge(&ln.ln_mq), Ordering::Relaxed);
                ln
            };
            // The last reference (the C's `refcnt_rele` and `pool_put`): the record and its
            // route handle go.
            drop(la);

            // leave from solicited node multicast for proxy ND
            if rt.rt_flags.isset(RTF_ANNOUNCE) && ifp.if_flags.get() & IFF_MULTICAST != 0 {
                let llsol = nd6_llsol(&rt_key_in6(rt), ifp);

                hal::klock::kernel_lock();
                // in6_lookupmulti's if_maddrlock, read (the module's deviations).
                let lock = &ifp.rt().if_maddrlock;
                let _ = lock.enter(RW_READ);
                let in6m = in6_lookupmulti(&llsol, ifp);
                lock.exit();
                if let Some(in6m) = in6m {
                    in6_delmulti(in6m, net);
                }
                hal::klock::kernel_unlock();
            }
        }
        RTM_INVALIDATE if !rt.rt_flags.isset(RTF_LOCAL) => {
            nd6_invalidate(rt);
        }
        _ => {}
    }
}

/// `nd6_rtrequest`'s `RTM_ADD` and `RTM_RESOLVE` from the gateway's check on: the route's
/// record made, linked and given to the route's word, the forced garbage collection of the
/// least recently used entries, our own or a proxied address made reachable for good, and the
/// proxy's solicited-node group joined.
fn nd6_rtrequest_add(ifp: &'static Ifnet, req: u8, rt: &Rtentry, net: &NetHeld<'_>) {
    let (gfam, glen) = gw_family_len(rt);
    if gfam != AF_LINK || glen < size_of::<SockaddrDl>() {
        log(
            LOG_DEBUG,
            format_args!(
                "nd6_rtrequest: bad gateway value: {}\n",
                Str(&ifp.if_xname.get())
            ),
        );
        return;
    }
    gw_set_type_index(rt, ifp);
    // Case 2: This route may come from cloning, or a manual route add with a LL address.
    // The record's counted handle of its route (`ln_rt`); a route the routing table gave is
    // always an item of its recorded pool, so this answers; a value outside it (none exists)
    // would get no record, quietly, as the C has no such case.
    let Some(handle) = PoolArc::from_ref(rt) else {
        return;
    };
    let Some(ln) = ND6_POOL.get(PR_NOWAIT, || LlinfoNd6::new(Some(handle))) else {
        log(LOG_DEBUG, format_args!("nd6_rtrequest: pool get failed\n"));
        return;
    };
    let ln_addr = PoolBox::as_ptr(&ln).addr();

    let key = rt_key_in6(rt);
    let ifa6;
    {
        let mut g = ND6_MTX.lock();
        if rt.rt_llinfo.is_set() {
            // we lost the race, another thread has entered it
            drop(g);
            drop(ln);
            return;
        }
        *g += 1;
        ND6_LIST.insert_head(&mut g, ln.as_pin());
        if let Err(ln) = rt.rt_llinfo.set_b(&g, ln) {
            // another owner stored meanwhile: not ours
            *g -= 1;
            ND6_LIST.remove(&mut g, &ln);
            drop(g);
            drop(ln);
            return;
        }
        rt.rt_flags.set_bits(RTF_LLINFO);
        if let Some(ln) = rt.rt_llinfo.get_b(&g) {
            // this is required for "ndp" command. - shin
            if req == RTM_ADD {
                // gate should have some valid AF_LINK entry, and ln expire should have some
                // lifetime which is specified by ndp command.
                ln.ln_state.set(ND6_LLINFO_REACHABLE);
            } else {
                // When req == RTM_RESOLVE, rt is created and initialized in rtrequest(), so
                // rt_expire is 0.
                ln.ln_state.set(ND6_LLINFO_NOSTATE);
                nd6_llinfo_settimer(ln, 0);
            }
        }

        // If we have too many cache entries, initiate immediate purging for some "less
        // recently used" entries. Note that we cannot directly call nd6_free() here because it
        // would cause re-entering rtable related routines triggering lock-order-reversal
        // problems.
        if *g >= IP6_NEIGHBORGCTHRESH.load(Ordering::Relaxed) {
            for _ in 0..10 {
                let stop = match ND6_LIST.last(&g) {
                    None => true,
                    // the new entry, or an iterator: cannot move the iterator, try next time
                    Some(e) => ptr::from_ref(e).addr() == ln_addr || e.ln_rt.is_none(),
                };
                if stop {
                    break;
                }

                // Move this entry to the head
                let _ = ND6_LIST.move_last_to_head(&mut g);
                let Some(ln_end) = ND6_LIST.first(&g) else {
                    break;
                };

                if ln_end.permanent() {
                    continue;
                }

                if ln_end.ln_state.get() > ND6_LLINFO_INCOMPLETE {
                    ln_end.ln_state.set(ND6_LLINFO_STALE);
                } else {
                    ln_end.ln_state.set(ND6_LLINFO_PURGE);
                }
                nd6_llinfo_settimer(ln_end, 0);
            }
        }

        // check if rt_key(rt) is one of my address assigned to the interface.
        ifa6 = in6ifa_ifpwithaddr(ifp, &key);
        if ifa6.is_some() || rt.rt_flags.isset(RTF_ANNOUNCE) {
            if let Some(ln) = rt.rt_llinfo.get_b(&g) {
                ln.ln_state.set(ND6_LLINFO_REACHABLE);
            }
            rt.rt_expire().set(0);
        }
    }

    // join solicited node multicast for proxy ND
    if ifa6.is_none() && rt.rt_flags.isset(RTF_ANNOUNCE) && ifp.if_flags.get() & IFF_MULTICAST != 0
    {
        let llsol = nd6_llsol(&key, ifp);

        hal::klock::kernel_lock();
        let _ = in6_addmulti(&llsol, ifp, net);
        hal::klock::kernel_unlock();
    }
}

/// `nd6_ioctl(cmd, data, ifp)`: `SIOCGIFINFO_IN6` and `SIOCGNBRINFO_IN6` over `data`, the
/// request's bytes (`in6_ioctl` checked their length); `EINVAL` for a neighbor without an
/// entry.
pub fn nd6_ioctl(cmd: u64, data: &mut [u8], ifp: &'static Ifnet) -> Result<(), Errno> {
    match cmd {
        SIOCGIFINFO_IN6 => {
            let Some(mut ndi) = pod::at::<In6Ndireq>(data, 0) else {
                return Err(Errno::EINVAL);
            };
            {
                let _r = NETLOCK.read();
                ndi.ndi = ifp.if_nd.get();
            }
            let _ = pod::put(data, 0, &ndi);
            Ok(())
        }
        SIOCGNBRINFO_IN6 => {
            let Some(mut nbi) = pod::at::<In6Nbrinfo>(data, 0) else {
                return Err(Errno::EINVAL);
            };
            let mut nb_addr = nbi.addr; // make local for safety

            {
                let r = NETLOCK.read();
                let Some(net) = NetHeld::of(&r) else {
                    return Err(Errno::EINVAL);
                };
                // XXX: KAME specific hack for scoped addresses
                //      XXXX: for other scopes than link-local?
                if (in6_is_addr_linklocal(&nb_addr) || in6_is_addr_mc_linklocal(&nb_addr))
                    && nb_addr.s6_addr16(1) == 0
                {
                    nb_addr.set_s6_addr16(1, htons(ifp.if_index.get() as u16));
                }

                let rt = nd6_lookup(&nb_addr, false, Some(ifp), ifp.if_rdomain.get(), &net);
                let g = ND6_MTX.lock();
                let Some((rt, ln)) = rt
                    .as_ref()
                    .and_then(|r| r.rt_llinfo.get_b(&g).map(|ln| (r, ln)))
                else {
                    return Err(Errno::EINVAL);
                };
                let mut expire = rt.rt_expire().get();
                if expire != 0 {
                    expire -= getuptime();
                    expire += gettime();
                }

                nbi.state = i32::from(ln.ln_state.get());
                nbi.asked = ln.ln_asked.get();
                nbi.isrouter = i32::from(ln.ln_router.get());
                nbi.expire = expire;
            }
            let _ = pod::put(data, 0, &nbi);
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The first `if_addrlen` bytes of a link-layer address (all of it when shorter).
fn lladdr_bytes<'a>(lladdr: &'a [u8], ifp: &Ifnet) -> &'a [u8] {
    &lladdr[..usize::from(ifp.if_addrlen.get()).min(lladdr.len())]
}

/// A link-layer address as `ether_sprintf` prints it.
pub fn nd6_ether_sprintf(lladdr: &[u8]) -> [u8; ETHER_ADDR_LEN * 3] {
    let mut ea = [0u8; ETHER_ADDR_LEN];
    let n = lladdr.len().min(ETHER_ADDR_LEN);
    ea[..n].copy_from_slice(&lladdr[..n]);
    ether_sprintf(&ea)
}

/// Whether `ll` differs from the link-layer address in `sdl` (the C's
/// `bcmp(lladdr, LLADDR(sdl), ifp->if_addrlen)`). The route's link-level gateway is a
/// `sockaddr_dl` with room for the interface's address (`nd6_rtrequest` checked its size); one
/// too short for it compares as changed.
pub fn nd6_llchanged(ll: &[u8], rt: &Rtentry, ifp: &Ifnet) -> bool {
    let l = lladdr_bytes(ll, ifp);
    rt.with_gateway(|g| {
        let cur = g.and_then(|g| {
            let start = hal::sys::net_if::SDL_DATA_OFFSET + usize::from(*g.get(5)?);
            g.get(start..start + l.len())
        });
        cur != Some(l)
    })
}

/// `satosdl(rt->rt_gateway)->sdl_alen`, 0 without a gateway.
pub fn nd6_lladdr_alen(rt: &Rtentry) -> u8 {
    gw_alen_family(rt).0
}

/// Records `ll` in the route's link-level gateway (the C's `sdl->sdl_alen = ifp->if_addrlen;
/// bcopy(lladdr, LLADDR(sdl), ifp->if_addrlen)`).
pub fn nd6_llstore(ll: &[u8], rt: &Rtentry, ifp: &Ifnet) {
    let l = lladdr_bytes(ll, ifp);
    rt.update_gateway(|g| {
        if let Some(g) = g {
            g[SDL_ALEN] = ifp.if_addrlen.get();
            if let Some(a) = SockaddrDl::lladdr_mut(g) {
                let n = a.len().min(l.len());
                a[..n].copy_from_slice(&l[..n]);
            }
        }
    });
}

/// What `nd6_cache_lladdr`'s section decided, for after the mutex.
enum CacheStep {
    /// Nothing more.
    Done,
    /// The entry is invalid (the C's `fail:`): freed, its router bit with it.
    Fail(bool),
    /// The held packets to send (the entry became STALE: `if_output_mq`).
    Output(MbufList),
}

/// `nd6_cache_lladdr(ifp, from, lladdr, lladdrlen, type, code, i_am_router)`: create neighbor
/// cache entry and cache link-layer address, on reception of inbound ND6 packets
/// (RS/RA/NS/redirect). `type_` is the ICMP6 type, `code` the type dependent information,
/// `i_am_router` whether we forward. Under the exclusive net lock (`net`); the record under one
/// `ND6_MTX` section (the module's lock table), the held packets sent after it.
pub fn nd6_cache_lladdr(
    ifp: &'static Ifnet,
    from: &In6Addr,
    lladdr: Option<&[u8]>,
    type_: i32,
    code: i32,
    i_am_router: bool,
    net: &NetHeld<'_>,
) {
    net_assert_locked_exclusive();

    // nothing must be updated for unspecified address
    if in6_is_addr_unspecified(from) {
        return;
    }

    // Validation about ifp->if_addrlen and lladdrlen must be done in the caller.
    //
    // XXX If the link does not have link-layer address, what should we do?
    // (ifp->if_addrlen == 0) Spec says nothing in sections for RA, RS and NA. There's small
    // description on it in NS section (RFC 2461 7.2.3).

    let (rt, is_newentry) = match nd6_lookup(from, false, Some(ifp), ifp.if_rdomain.get(), net) {
        None => (
            nd6_lookup(from, true, Some(ifp), ifp.if_rdomain.get(), net),
            true,
        ),
        Some(rt) => {
            // do not overwrite local or static entry
            if rt.rt_flags.isset(RTF_STATIC | RTF_LOCAL) {
                return;
            }
            (Some(rt), false)
        }
    };

    let Some(rt) = rt else {
        return;
    };
    let ok = rt.rt_flags.get() & (RTF_GATEWAY | RTF_LLINFO) == RTF_LLINFO
        && gw_family_len(&rt).0 == AF_LINK;

    let mut overwritten = false;
    let step = {
        let g = ND6_MTX.lock();
        let ln = rt.rt_llinfo.get_b(&g);
        match ln.filter(|_| ok) {
            None => CacheStep::Fail(ln.is_some_and(|ln| ln.ln_router.get() != 0)),
            Some(ln) => {
                // An `AF_LINK` gateway of a neighbor route is a `sockaddr_dl` with room for the
                // interface's address (`nd6_rtrequest` checked its size).
                let olladdr = nd6_lladdr_alen(&rt) != 0;
                let llchange = match lladdr {
                    Some(l) if olladdr => nd6_llchanged(l, &rt, ifp),
                    _ => false,
                };

                // newentry olladdr  lladdr  llchange	(*=record)
                //	0	n	n	--	(1)
                //	0	y	n	--	(2)
                //	0	n	y	--	(3) * STALE
                //	0	y	y	n	(4) *
                //	0	y	y	y	(5) * STALE
                //	1	--	n	--	(6)   NOSTATE(= PASSIVE)
                //	1	--	y	--	(7) * STALE

                overwritten = llchange && lladdr.is_some();
                if let Some(l) = lladdr {
                    // (3-5) and (7)
                    // Record source link-layer address
                    // XXX is it dependent to ifp->if_type?
                    nd6_llstore(l, &rt, ifp);
                }

                let (do_update, newstate) = if !is_newentry {
                    if (!olladdr && lladdr.is_some()) // (3)
                        || (olladdr && lladdr.is_some() && llchange)
                    // (5)
                    {
                        (true, ND6_LLINFO_STALE)
                    } else {
                        // (1-2,4)
                        (false, ND6_LLINFO_INCOMPLETE)
                    }
                } else if lladdr.is_none() {
                    // (6)
                    (true, ND6_LLINFO_NOSTATE)
                } else {
                    // (7)
                    (true, ND6_LLINFO_STALE)
                };

                let mut step = CacheStep::Done;
                if do_update {
                    // Update the state of the neighbor cache.
                    ln.ln_state.set(newstate);

                    if newstate == ND6_LLINFO_STALE {
                        // Since nd6_resolve() in ifp->if_output() will cause state transition
                        // to DELAY and reset the timer, we must set the timer now, although it
                        // is actually meaningless.
                        nd6_llinfo_settimer(ln, ND6_GCTIMER as u32);
                        // if_output_mq: the held packets, sent after the mutex.
                        step = CacheStep::Output(mq_delist(&ln.ln_mq));
                    } else if newstate == ND6_LLINFO_INCOMPLETE {
                        // probe right away
                        nd6_llinfo_settimer(ln, 0);
                    }
                }

                // ICMP6 type dependent behavior.
                //
                // NS: clear IsRouter if new entry
                // RS: clear IsRouter
                // RA: set IsRouter if there's lladdr
                // redir: clear IsRouter if new entry
                //
                // RA case, (1):
                // The spec says that we must set IsRouter in the following cases:
                // - If lladdr exist, set IsRouter.  This means (1-5).
                // - If it is old entry (!newentry), set IsRouter.  This means (7).
                // So, based on the spec, in (1-5) and (7) cases we must set IsRouter.
                // A question arises for (1) case.  (1) case has no lladdr in the neighbor
                // cache, this is similar to (6). This case is rare but we figured that we MUST
                // NOT set IsRouter.
                //
                // newentry olladdr  lladdr  llchange	    NS  RS  RA	redir
                //							D R
                //	0	n	n	--	(1)	c   ?     s
                //	0	y	n	--	(2)	c   s     s
                //	0	n	y	--	(3)	c   s     s
                //	0	y	y	n	(4)	c   s     s
                //	0	y	y	y	(5)	c   s     s
                //	1	--	n	--	(6) c	c	c s
                //	1	--	y	--	(7) c	c   s	c s
                //
                //					(c=clear s=set)
                match (type_ & 0xff) as u8 {
                    ND_NEIGHBOR_SOLICIT => {
                        // New entry must have is_router flag cleared.
                        if is_newentry {
                            // (6-7)
                            ln.ln_router.set(0);
                        }
                    }
                    ND_REDIRECT => {
                        // If the icmp is a redirect to a better router, always set the
                        // is_router flag. Otherwise, if the entry is newly created, clear the
                        // flag. [RFC 2461, sec 8.3]
                        if code == i32::from(ND_REDIRECT_ROUTER) {
                            ln.ln_router.set(1);
                        } else if is_newentry {
                            // (6-7)
                            ln.ln_router.set(0);
                        }
                    }
                    ND_ROUTER_SOLICIT => {
                        // is_router flag must always be cleared.
                        ln.ln_router.set(0);
                    }
                    // Mark an entry with lladdr as a router: (2-5) and (7).
                    ND_ROUTER_ADVERT
                        if (!is_newentry && (olladdr || lladdr.is_some()))
                            || (is_newentry && lladdr.is_some()) =>
                    {
                        ln.ln_router.set(1);
                    }
                    _ => {}
                }
                step
            }
        }
    };

    if overwritten && let Some(l) = lladdr {
        let s = nd6_ether_sprintf(l);
        log(
            LOG_INFO,
            format_args!(
                "ndp info overwritten for {} by {} on {}\n",
                In6Ntop(*from),
                Str(&s),
                Str(&ifp.if_xname.get())
            ),
        );
    }

    match step {
        CacheStep::Done => {}
        CacheStep::Fail(router) => {
            // fail:
            nd6_free(&rt, ifp, i_am_router, router, net);
        }
        CacheStep::Output(ml) => nd6_output_held(ifp, &rt, ml),
    }
    // rtfree(rt): the handle's drop
}

/// `if_output_mq(ifp, &ln->ln_mq, &ln_hold_total, rt_key(rt), rt)` of the packets `ml` the
/// record's hold queue gave up under the mutex: sent, the hold total lowered by their count,
/// and, as the C's "XXXSMP we also discard if other CPU enqueues", a packet back in the queue
/// dropped. Only a hold of the output itself moves the total meanwhile (the exclusive net lock
/// keeps every other `nd6_resolve` out), so the record is reached again only then (ARP's
/// `arpcache`'s shape).
pub(crate) fn nd6_output_held(ifp: &'static Ifnet, rt: &PoolArc<Rtentry>, mut ml: MbufList) {
    let len = ml_len(&ml);
    let before = ln_hold_total.load(Ordering::Relaxed);
    let _ = if_output_ml(ifp, &mut ml, rt.dest_view(), Some(&**rt));
    let mut purged = 0;
    if ln_hold_total.load(Ordering::Relaxed) != before {
        let g = ND6_MTX.lock();
        if let Some(ln) = rt.rt_llinfo.get_b(&g)
            && mq_len(&ln.ln_mq) > 0
        {
            // mbuf is back in queue. Discard.
            purged = mq_purge(&ln.ln_mq);
        }
    }
    ln_hold_total.fetch_sub(len + purged, Ordering::Relaxed);
}

/// `nd6_slowtimo`: recomputes the random Reachable Time of every interface every few hours.
fn nd6_slowtimo(_ignored_arg: *mut c_void) {
    let _w = NETLOCK.write();

    let _ = Pin::static_ref(&ND6_SLOWTIMO_CH).add_sec(ND6_SLOWTIMER_INTERVAL);

    for ifp in IFNETLIST.iter() {
        let mut nd = ifp.if_nd.get();
        nd.recalctm -= ND6_SLOWTIMER_INTERVAL;
        if nd.recalctm <= 0 {
            // Since reachable time rarely changes by router advertisements, we SHOULD insure
            // that a new random value gets recomputed at least once every few hours. (RFC
            // 2461, 6.3.4)
            nd.recalctm = ND6_RECALC_REACHTM_INTERVAL;
            nd.reachable = nd_compute_rtime(REACHABLE_TIME);
        }
        ifp.if_nd.set(nd);
    }
}

/// `nd6_resolve(ifp, rt0, m, dst, desten)`: the link-layer address of `dst` (a
/// `sockaddr_in6`) for `m` in `desten`, the packet back to be sent; `Err(EAGAIN)` when the
/// packet was queued until the neighbor answers, any other error once it is freed
/// (`ether_resolve`'s).
pub fn nd6_resolve(
    ifp: &'static Ifnet,
    rt0: Option<&Rtentry>,
    m: MbufBox,
    dst: SockaddrView<'_>,
    desten: &mut [u8; ETHER_ADDR_LEN],
) -> Result<MbufBox, Errno> {
    // A `sockaddr_in6` (the IPv6 output's destination; the bridge leg's check, kept).
    if dst.bytes().len() < SIN6_SIZE {
        return Err(Errno::EINVAL);
    }

    if m.flags() & M_MCAST != 0 {
        *desten = ether_map_ipv6_multicast(&sin6_addr_of(dst));
        return Ok(m);
    }

    let uptime = getuptime();
    let Some(rt0) = rt0 else {
        return Err(Errno::EHOSTDOWN);
    };
    rt_getll(rt0, |rt| {
        let Some(rt) = rt.filter(|r| {
            !(r.rt_flags.isset(RTF_REJECT)
                && (r.rt_expire().get() == 0 || r.rt_expire().get() > uptime))
        }) else {
            return Err(if rt.is_some_and(|r| ptr::eq(r, rt0)) {
                Errno::EHOSTDOWN
            } else {
                Errno::EHOSTUNREACH
            });
        };
        nd6_resolve_ll(ifp, rt, m, dst, desten)
    })
}

/// `satosin6(dst)->sin6_addr`: the address of a `sockaddr_in6`'s bytes (offset 8), read where
/// the C reads it (the multicast map, the log, the solicitation), never on the hit path.
#[inline]
fn sin6_addr_of(dst: SockaddrView<'_>) -> In6Addr {
    pod::at::<In6Addr>(dst.bytes(), 8).unwrap_or_default()
}

/// `nd6_resolve`'s body over the link-layer route `rt` (`rt_getll(rt0)`, not rejected).
fn nd6_resolve_ll(
    ifp: &'static Ifnet,
    rt: &Rtentry,
    m: MbufBox,
    dst: SockaddrView<'_>,
    desten: &mut [u8; ETHER_ADDR_LEN],
) -> Result<MbufBox, Errno> {
    // Address resolution or Neighbor Unreachability Detection for the next hop. At this point,
    // the destination of the packet must be a unicast or an anycast address(i.e. not a
    // multicast).
    if !rt.rt_flags.isset(RTF_LLINFO) {
        log(
            LOG_DEBUG,
            format_args!(
                "nd6_resolve: {}: route contains no ND information\n",
                In6Ntop(rt_key_in6(rt))
            ),
        );
        return Err(Errno::EINVAL); // bad: m freed
    }

    // An `RTF_LLINFO` route's gateway: its family, length and address read at once (ARP's
    // shape: the C reads `rt_gateway` twice; its writers hold the exclusive net lock, which
    // the output's holding keeps out).
    let (family, alen, en) = rt.with_gateway(|g| {
        g.map_or((0, 0, [0; ETHER_ADDR_LEN]), |g| {
            (g[1], g.get(SDL_ALEN).copied().unwrap_or(0), sdl_lladdr(g))
        })
    });
    if family != AF_LINK {
        printf(format_args!("nd6_resolve: something odd happens\n"));
        return Err(Errno::EINVAL);
    }

    let mut g = ND6_MTX.lock();
    // Move this entry to the head of the queue so that it is less likely for this entry to be
    // a target of forced garbage collection (see nd6_rtrequest()).
    if !rt.rt_llinfo.move_b_to_head(&mut g, &ND6_LIST) {
        return Err(Errno::EINVAL);
    }
    let Some(ln) = rt.rt_llinfo.get_b(&g) else {
        return Err(Errno::EINVAL);
    };

    // The first time we send a packet to a neighbor whose entry is STALE, we have to change
    // the state to DELAY and set a timer to expire in DELAY_FIRST_PROBE_TIME seconds to ensure
    // we do neighbor unreachability detection on expiration. (RFC 2461 7.3.3)
    if ln.ln_state.get() == ND6_LLINFO_STALE {
        ln.ln_asked.set(0);
        ln.ln_state.set(ND6_LLINFO_DELAY);
        nd6_llinfo_settimer(ln, ND6_DELAY.load(Ordering::Relaxed) as u32);
    }

    // If the neighbor cache entry has a state other than INCOMPLETE (i.e. its link-layer
    // address is already resolved), just send the packet.
    if ln.ln_state.get() > ND6_LLINFO_INCOMPLETE {
        drop(g);

        if usize::from(alen) != ETHER_ADDR_LEN {
            log(
                LOG_DEBUG,
                format_args!(
                    "nd6_resolve: {}: incorrect nd6 information\n",
                    In6Ntop(sin6_addr_of(dst))
                ),
            );
            return Err(Errno::EINVAL);
        }

        *desten = en;
        return Ok(m);
    }

    // There is a neighbor cache entry, but no ethernet address response yet. Insert mbuf in
    // hold queue if below limit. If above the limit free the queue without queuing the new
    // packet.
    if ln.ln_state.get() == ND6_LLINFO_NOSTATE {
        ln.ln_state.set(ND6_LLINFO_INCOMPLETE);
    }
    // source address of prompting packet is needed by nd6_ns_output()
    if m.m_len().get() as usize >= IP6_HDR_LEN
        && let Some(src) = pod::at::<In6Addr>(m.data(), 8)
    {
        ln.ln_saddr6.set_bytes(src.s6_addr);
    }
    if ln_hold_total.fetch_add(1, Ordering::Relaxed) < LN_HOLD_TOTAL {
        if mq_push(&ln.ln_mq, m) {
            ln_hold_total.fetch_sub(1, Ordering::Relaxed);
        }
    } else {
        ln_hold_total.fetch_sub(mq_purge(&ln.ln_mq) + 1, Ordering::Relaxed);
        drop(m);
    }

    // If there has been no NS for the neighbor after entering the INCOMPLETE state, send the
    // first solicitation.
    let mut solicit = None;
    if !ln.permanent() && ln.ln_asked.get() == 0 {
        ln.ln_asked.set(ln.ln_asked.get() + 1);
        nd6_llinfo_settimer(ln, RETRANS_TIMER / 1000);
        solicit = Some(ln.ln_saddr6());
    }
    drop(g);

    if let Some(saddr6) = solicit {
        // A driver's output carries no proof of the net lock: the bridge's (the module's
        // redesign).
        let dst6 = sin6_addr_of(dst);
        with_net_held(|net| nd6_ns_output(ifp, None, &dst6, Some(&saddr6), false, net));
    }
    Err(Errno::EAGAIN)
}

/// The length of an IPv6 header.
const IP6_HDR_LEN: usize = 40;

/// `f` with the proof of the net lock the caller holds without one (`Inet::with_net_held`:
/// a driver's output carries none, until N7's).
fn with_net_held(f: impl FnOnce(&NetHeld<'_>)) {
    let mut f = Some(f);
    let mut call = |net: &NetHeld<'_>| {
        if let Some(f) = f.take() {
            f(net);
        }
    };
    match inet() {
        Some(i) => i.with_net_held(&mut call),
        None => kpanic(format_args!("NET_ASSERT_LOCKED: no proof of the net lock")),
    }
}

/// `nd6_need_cache(ifp)`: whether `ifp` needs a neighbor cache. RFC2893 says: unidirectional
/// tunnels needs no ND.
pub fn nd6_need_cache(ifp: &Ifnet) -> bool {
    matches!(ifp.if_type.get(), IFT_ETHER | IFT_IEEE80211 | IFT_CARP)
}

const _: () = {
    assert!(LN_HOLD_QUEUE > 0 && LN_HOLD_TOTAL >= LN_HOLD_QUEUE);
    assert!(SIN6_SIZE == 28);
};

/// Forgets the neighbor cache for the host tests (`setup_real_memory`'s rule, testing.md):
/// the list emptied without touching its records (each still owned by its route's word, in an
/// earlier test's memory), `nd6_inuse` and the hold total zeroed, the three timeouts taken off
/// the wheel.
#[cfg(any(test, feature = "doubles"))]
pub fn nd6_test_reset() {
    {
        let mut g = ND6_MTX.lock();
        while ND6_LIST.remove_first(&mut g) {}
        *g = 0;
    }
    ln_hold_total.store(0, Ordering::Relaxed);
    // Off the wheel as fresh ones, without counting a deletion (the timeout tests, which run
    // beside these, read the wheel's counters).
    ND6_TIMER_TO.set_fresh(Some(nd6_timer), ptr::null_mut(), KCLOCK_NONE, TIMEOUT_PROC);
    ND6_SLOWTIMO_CH.set_fresh(
        Some(nd6_slowtimo),
        ptr::null_mut(),
        KCLOCK_NONE,
        TIMEOUT_PROC,
    );
    ND6_EXPIRE_TIMEOUT.set_fresh(Some(nd6_expire_timer), ptr::null_mut(), KCLOCK_NONE, 0);
    ND6_TIMER_NEXT.store(-1, Ordering::Relaxed);
    ND6_EXPIRE_NEXT.store(-1, Ordering::Relaxed);
}

/// The records on the list, for the tests (markers included).
#[cfg(test)]
pub(crate) fn nd6_listed() -> usize {
    let g = ND6_MTX.lock();
    ND6_LIST.iter(&g).count()
}

/// The record of a route, for the tests: its state, asked count, router bit, held packets and
/// prompting source.
#[cfg(any(test, feature = "doubles"))]
pub fn nd6_entry(rt: &Rtentry) -> Option<(i16, i64, i16, u32, In6Addr)> {
    let g = ND6_MTX.lock();
    rt.rt_llinfo.get_b(&g).map(|ln| {
        (
            ln.ln_state.get(),
            ln.ln_asked.get(),
            ln.ln_router.get(),
            ln.held(),
            ln.ln_saddr6(),
        )
    })
}

/// Sets the state of a route's record (and, with `secs`, its timer), for the tests: an entry as
/// another path would have left it. Under the net lock.
#[cfg(any(test, feature = "doubles"))]
pub fn nd6_entry_set_for_tests(rt: &Rtentry, state: i16, secs: Option<u32>) {
    let g = ND6_MTX.lock();
    if let Some(ln) = rt.rt_llinfo.get_b(&g) {
        ln.ln_state.set(state);
        if let Some(s) = secs {
            nd6_llinfo_settimer(ln, s);
        }
    }
}

/// Runs the neighbor cache timeout once, for the tests (its timer fired).
#[cfg(any(test, feature = "doubles"))]
pub fn nd6_timer_for_tests() {
    nd6_timer(ptr::null_mut());
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests of Neighbor Discovery over a test Ethernet interface (an `Arpcom` with a
    // driver that leaves its frames on its send queue) with fd00:77::1/64 configured through
    // `in6_ioctl` (its link-local companion and DAD with it): LZ's nd6.rs's (the option parser,
    // the address printer, an entry walked through its states, a neighbor that never answers),
    // LZ's nd6_nbr.rs's (the NS and NA rejection tables over real packets through `nd6_msg`, one
    // case per check, each failing when its check goes; DAD's counts and its second start, a
    // down interface, an anycast address, `nd6_ifptomac`) and the brief's (the hold queue's and
    // the hold total's bounds, the timer's walk past entries it frees, solicitations and
    // advertisements with and without the link-layer option, DAD's three outcomes and its
    // records reused, the route's count without the record's handle, the ioctls, the records'
    // sizes).

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::sys::net_if::{
        IFF_BROADCAST, IFF_MULTICAST, IFF_RUNNING, IFF_SIMPLEX, IFF_UP, IFNAMSIZ, IFT_LOOP,
        IFXF_MPSAFE,
    };
    use hal::sys::netinet_icmp6::{
        ND_NA_FLAG_OVERRIDE, ND_NA_FLAG_SOLICITED, ND_NEIGHBOR_ADVERT, NdNeighborAdvert,
    };
    use hal::sys::netinet_ip6::{IPV6_VERSION, Ip6Hdr};
    use hal::sys::netinet6_in6_var::{
        IN6_IFF_ANYCAST, IN6_IFF_DUPLICATED, IN6_IFF_TENTATIVE, In6Aliasreq,
    };

    use super::*;
    use crate::net::if_::SIOCAIFADDR_IN6;
    use crate::net::if_::{TestDriver, if_attach};
    use crate::net::if_ethersubr::{Arpcom, ether_ifattach};
    use crate::net::ifq::ifq_dequeue;
    use crate::net::mbuf::{M_DONTWAIT, MT_DATA, m_copydata, m_devget, m_get, m_gethdr};
    use crate::net::route::tests::{take_icmp6_errors, take_ip6_sent, testroutes_setup};
    use crate::netinet6::in6::in6_ioctl;
    use crate::netinet6::in6::tests::{a6, add, aliasreq, zoned};
    use crate::netinet6::in6_var::In6Ifaddr;
    use crate::netinet6::ip6_var::{ICMP6COUNTERS, IP6_DAD_COUNT, IP6_DAD_PENDING};
    use crate::netinet6::nd6_nbr::{
        Dadq, dad_counts, dad_fire, dad_free_counts, dad_reap, dad_tries, nd6_dad_start,
        nd6_dad_stop, nd6_ifptomac, nd6_na_input, nd6_ns_input,
    };
    use crate::tc::TIME_UPTIME;

    const OURS: [u8; 6] = [0x52, 0x54, 0x00, 0xbb, 0x00, 0x01];
    const PEER: [u8; 6] = [0x52, 0x54, 0x00, 0xbb, 0x00, 0x02];

    /// `fd00:77::1`, our address.
    fn ours6() -> In6Addr {
        a6("fd00:77::1")
    }

    /// `fd00:77::2`, the neighbor's.
    fn peer6() -> In6Addr {
        a6("fd00:77::2")
    }

    /// The value of an ICMPv6 counter.
    fn icmp6stat(c: Icmp6statCounters) -> u64 {
        ICMP6COUNTERS.read()[c as usize]
    }

    /// The routing tests' fixture, ND's state and timers, the uptime past 0 (a route cloned at
    /// 0 would read as a permanent entry), and nothing left of an earlier test's output.
    fn setup() -> MutexGuard<'static, ()> {
        let g = testroutes_setup();
        nd6_init();
        TIME_UPTIME.fetch_max(1000, Ordering::Relaxed);
        let _ = take_ip6_sent();
        let _ = take_icmp6_errors();
        g
    }

    /// An Ethernet interface `name` (a zeroed `struct arpcom` with a test driver that leaves
    /// its frames queued), up and running, with fd00:77::1/64 (its link-local companion and
    /// both DADs started: their solicitations taken).
    fn ether(name: &[u8]) -> &'static Ifnet {
        let ac: &'static Arpcom = Box::leak(Box::new(Arpcom::new()));
        let d: &'static TestDriver = Box::leak(Box::new(TestDriver::new()));
        let ifp: &'static Ifnet = &ac.ac_if;
        let mut n = [0u8; IFNAMSIZ];
        n[..name.len()].copy_from_slice(name);
        ifp.if_xname.set(n);
        assert!(ifp.if_driver.set(d).is_ok());
        ifp.if_xflags.set(IFXF_MPSAFE);
        ifp.if_flags
            .set(IFF_BROADCAST | IFF_SIMPLEX | IFF_MULTICAST | IFF_RUNNING | IFF_UP);
        ac.ac_enaddr.set(OURS);
        hal::klock::kernel_lock();
        if_attach(ifp);
        ether_ifattach(ac);
        hal::klock::kernel_unlock();
        add(ifp, ours6(), 64).expect("SIOCAIFADDR_IN6");
        let _ = sent(ifp);
        let _ = take_ip6_sent();
        ifp
    }

    /// Our address's record on `ifp`.
    fn ours_ia6(ifp: &Ifnet) -> &'static In6Ifaddr {
        in6ifa_ifpwithaddr(ifp, &ours6()).expect("our address")
    }

    /// DAD of our address run to its end with no answer: the address preferred (its NA to the
    /// routers taken).
    fn dad_done(ifp: &'static Ifnet) {
        let ia6 = ours_ia6(ifp);
        while dad_counts(&ia6.ia_ifa).is_some() {
            dad_fire(&ia6.ia_ifa);
        }
        dad_reap();
        assert_eq!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE, 0, "preferred");
        let _ = take_ip6_sent();
    }

    /// The frames the interface was given since the last call, as bytes.
    fn sent(ifp: &'static Ifnet) -> Vec<Vec<u8>> {
        let mut v = Vec::new();
        while let Some(m) = ifq_dequeue(ifp.if_snd()) {
            let mut b = std::vec![0u8; m.m_pkthdr().len.get() as usize];
            m_copydata(&m, 0, &mut b);
            v.push(b);
        }
        v
    }

    /// The route to `a` in table 0, cloned from the prefix's cloning route with `resolve`.
    fn route6(a: In6Addr, resolve: bool) -> Option<PoolArc<Rtentry>> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let s = sin6_bytes(&a);
        let flags = if resolve { RT_RESOLVE } else { 0 };
        rtalloc(SockaddrView::new(&s).expect("a view"), flags, 0, &net)
    }

    /// The entry of the route to `a`: state, asked, router, held, prompting source.
    fn entry(a: In6Addr) -> Option<(i16, i64, i16, u32, In6Addr)> {
        let rt = route6(a, false)?;
        nd6_entry(&rt)
    }

    /// A 46-byte IPv6 packet from us to `dst` (the header and 6 bytes).
    fn packet_to(dst: In6Addr) -> MbufBox {
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(46);
        m.m_pkthdr().len.set(46);
        let mut ip6 = Ip6Hdr {
            ip6_plen: htons(6),
            ip6_nxt: 59, // IPPROTO_NONE
            ip6_hlim: 64,
            ip6_src: ours6(),
            ip6_dst: dst,
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        assert!(pod::put(m.data_mut().expect("room"), 0, &ip6));
        m
    }

    /// `nd6_resolve` of a packet to `dst` through `rt`, under the net lock held detached (the
    /// solicitation asks the bridge for its proof).
    fn resolve(ifp: &'static Ifnet, rt: &Rtentry, dst: In6Addr) -> Result<[u8; 6], Errno> {
        let s = sin6_bytes(&dst);
        let mut desten = [0u8; ETHER_ADDR_LEN];
        hal::netlock::lock();
        let r = nd6_resolve(
            ifp,
            Some(rt),
            packet_to(dst),
            SockaddrView::new(&s).expect("a view"),
            &mut desten,
        );
        hal::netlock::unlock();
        r.map(|m| {
            drop(m);
            desten
        })
    }

    /// An ICMPv6 ND message from `src` to `dst` with hop limit `hlim`: the fixed part `msg`
    /// and the options `opts`, received on `ifp`.
    fn nd_packet(
        ifp: &Ifnet,
        src: In6Addr,
        dst: In6Addr,
        hlim: u8,
        msg: &[u8],
        opts: &[u8],
    ) -> (MbufBox, i32) {
        let len = msg.len() + opts.len();
        let mut ip6 = Ip6Hdr {
            ip6_plen: htons(len as u16),
            ip6_nxt: 58, // IPPROTO_ICMPV6
            ip6_hlim: hlim,
            ip6_src: src,
            ip6_dst: dst,
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        let mut b = std::vec![0u8; IP6_HDR_LEN];
        assert!(pod::put(&mut b, 0, &ip6));
        b.extend_from_slice(msg);
        b.extend_from_slice(opts);
        let m = m_devget(&b, 0).expect("an mbuf");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        (m, len as i32)
    }

    /// The fixed part of a solicitation (`flags` 0) or advertisement (network-order `flags`)
    /// for `target`.
    fn nd_msg(type_: u8, flags: u32, target: In6Addr) -> Vec<u8> {
        let mut b = std::vec![type_, 0, 0, 0];
        b.extend_from_slice(&flags.to_ne_bytes());
        b.extend_from_slice(&target.s6_addr);
        b
    }

    /// A link-layer address option of `type_` with `mac`.
    fn lla(type_: u8, mac: [u8; 6]) -> [u8; 8] {
        [type_, 1, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]]
    }

    /// The solicited-node group of `a` (zone of `ifp` embedded).
    fn solicited_node(ifp: &Ifnet, a: In6Addr) -> In6Addr {
        nd6_llsol(&a, ifp)
    }

    /// `nd6_na_input` of an advertisement for `target` (from `target`, to us) with `flags`
    /// and the target link-layer option `mac`, under the exclusive net lock.
    fn advertise(ifp: &'static Ifnet, target: In6Addr, flags: u32, mac: Option<[u8; 6]>) {
        let opt = mac.map(|m| lla(ND_OPT_TARGET_LINKADDR, m));
        let (m, len) = nd_packet(
            ifp,
            target,
            ours6(),
            255,
            &nd_msg(ND_NEIGHBOR_ADVERT, flags, target),
            opt.as_ref().map_or(&[][..], |o| &o[..]),
        );
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        nd6_na_input(m, IP6_HDR_LEN as i32, len, &net);
    }

    /// `nd6_ns_input` of a solicitation from `src` to `dst` for `target`, with the source
    /// link-layer option `mac`, under the exclusive net lock.
    fn solicit(
        ifp: &'static Ifnet,
        src: In6Addr,
        dst: In6Addr,
        target: In6Addr,
        mac: Option<[u8; 6]>,
    ) {
        let opt = mac.map(|m| lla(ND_OPT_SOURCE_LINKADDR, m));
        let (m, len) = nd_packet(
            ifp,
            src,
            dst,
            255,
            &nd_msg(ND_NEIGHBOR_SOLICIT, 0, target),
            opt.as_ref().map_or(&[][..], |o| &o[..]),
        );
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        nd6_ns_input(m, IP6_HDR_LEN as i32, len, &net);
    }

    /// The ICMPv6 type, the IPv6 destination and the ND target of a packet the output leg
    /// was given.
    fn nd_sent(b: &[u8]) -> (u8, In6Addr, In6Addr) {
        (
            b[IP6_HDR_LEN],
            pod::at::<In6Addr>(b, 24).expect("dst"),
            pod::at::<In6Addr>(b, IP6_HDR_LEN + 8).expect("target"),
        )
    }

    /// `nd6_timer` run at uptime `t`.
    fn tick(t: i64) {
        TIME_UPTIME.store(t, Ordering::Relaxed);
        nd6_timer(ptr::null_mut());
    }

    #[test]
    fn nd6_options_finds_the_link_layer_addresses() {
        let _g = setup();
        // A source option, an MTU option, a second source option (ignored), a target option.
        let opt: [u8; 32] = [
            1, 1, 0x52, 0x55, 0x0a, 0, 2, 2, // source link-layer address
            5, 1, 0, 0, 0, 0, 0x05, 0xdc, // MTU
            1, 1, 1, 2, 3, 4, 5, 6, // duplicate source: the first one wins
            2, 1, 0x52, 0x54, 0, 0x12, 0x34, 0x56, // target link-layer address
        ];
        let mut ndopts = NdOpts::default();
        assert!(nd6_options(&opt, &mut ndopts));
        let src = ndopts.nd_opts_src_lladdr.expect("source option");
        let tgt = ndopts.nd_opts_tgt_lladdr.expect("target option");
        assert_eq!((src, tgt), (0, 24));
        let ((sl, sn), (tl, tn)) = (nd6_opt_lladdr(&opt, src), nd6_opt_lladdr(&opt, tgt));
        assert_eq!((&sl[..6], sn), (&[0x52, 0x55, 0x0a, 0, 2, 2][..], 8));
        assert_eq!((&tl[..6], tn), (&[0x52, 0x54, 0, 0x12, 0x34, 0x56][..], 8));

        // No options at all is valid.
        assert!(nd6_options(&[], &mut ndopts));
        assert_eq!(ndopts, NdOpts::default());
        // An offset past the options gives nothing (no option starts there).
        assert_eq!(nd6_opt_lladdr(&opt, 40), (&[][..], 0));
    }

    #[test]
    fn nd6_options_rejects_bad_lengths_and_stops_after_nd6_maxndopt() {
        let _g = setup();
        let bad = || icmp6stat(Icmp6statCounters::Icp6sNdBadopt);
        let mut ndopts = NdOpts::default();
        for opt in [
            &[1u8, 0, 0, 0, 0, 0, 0, 0][..],     // zero-length option
            &[1, 1, 0, 0, 0, 0][..],             // option overruns the buffer (truncated)
            &[1][..],                            // no room for nd_opt_len
            &[1, 1, 0, 0, 0, 0, 0, 0, 2][..],    // a trailing byte after a good option
            &[1, 2, 0, 0, 0, 0, 0, 0, 0, 0][..], // length says 16, 10 bytes there
            &[2, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0][..], // a good one, then 0
        ] {
            let before = bad();
            assert!(!nd6_options(opt, &mut ndopts), "{opt:?}");
            assert_eq!(ndopts, NdOpts::default(), "cleared on error");
            assert_eq!(bad(), before + 1);
        }

        // Eleven options (nd6_maxndopt 10): parsing stops after the eleventh, the message is
        // still valid, a source option past the stop is not seen.
        let mut many = [[3u8, 1, 0, 0, 0, 0, 0, 0]; 12].concat();
        many[11 * 8] = ND_OPT_SOURCE_LINKADDR;
        let before = icmp6stat(Icmp6statCounters::Icp6sNdToomanyopt);
        assert!(nd6_options(&many, &mut ndopts));
        assert_eq!(icmp6stat(Icmp6statCounters::Icp6sNdToomanyopt), before + 1);
        assert_eq!(ndopts.nd_opts_src_lladdr, None);
        // Ten options: all of them read, no count.
        let ten = [[3u8, 1, 0, 0, 0, 0, 0, 0]; 10].concat();
        assert!(nd6_options(&ten, &mut ndopts));
        assert_eq!(icmp6stat(Icmp6statCounters::Icp6sNdToomanyopt), before + 1);
    }

    #[test]
    fn in6_ntop_writes_what_inet_ntop_writes() {
        let a = |s: [u16; 8]| {
            let mut b = [0u8; 16];
            for (i, w) in s.iter().enumerate() {
                b[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
            }
            std::format!("{}", In6Ntop(In6Addr { s6_addr: b }))
        };
        assert_eq!(a([0; 8]), "::");
        assert_eq!(a([0, 0, 0, 0, 0, 0, 0, 1]), "::1");
        assert_eq!(a([0xfe80, 0, 0, 0, 0, 0, 0, 1]), "fe80::1");
        assert_eq!(a([0xfd00, 0x77, 0, 0, 0, 0, 0, 2]), "fd00:77::2");
        assert_eq!(a([0xff02, 0, 0, 0, 0, 1, 0xff00, 2]), "ff02::1:ff00:2");
        assert_eq!(a([1, 0, 0, 2, 0, 0, 0, 3]), "1:0:0:2::3");
        assert_eq!(a([1, 0, 0, 2, 0, 0, 3, 4]), "1::2:0:0:3:4");
        assert_eq!(a([1, 0, 2, 3, 4, 5, 6, 7]), "1:0:2:3:4:5:6:7");
        assert_eq!(a([1, 2, 3, 4, 5, 6, 7, 0]), "1:2:3:4:5:6:7:0");
        assert_eq!(a([1, 2, 3, 4, 5, 0, 0, 0]), "1:2:3:4:5::");
        assert_eq!(
            a([0, 0, 0, 0, 0, 0xffff, 0x0a00, 0x0201]),
            "::ffff:10.0.2.1"
        );
        assert_eq!(a([0, 0, 0, 0, 0, 0, 0x0a00, 0x0201]), "::10.0.2.1");
    }

    #[test]
    fn an_entry_walks_incomplete_reachable_stale_delay_probe() {
        let _g = setup();
        let ifp = ether(b"nd0");
        dad_done(ifp);
        let rt = route6(peer6(), true).expect("the cloned neighbor route");
        assert!(rt.rt_flags.isset(RTF_LLINFO) && rt.rt_llinfo.is_b());
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_NOSTATE);
        let held = ln_hold_total.load(Ordering::Relaxed);

        // A packet to the neighbor: held, the entry INCOMPLETE, a solicitation sent to the
        // neighbor's solicited-node group, from the prompting packet's source.
        assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        let (state, asked, _, n, saddr6) = nd6_entry(&rt).expect("the entry");
        assert_eq!((state, asked, n), (ND6_LLINFO_INCOMPLETE, 1, 1));
        assert_eq!(saddr6, ours6(), "the prompting packet's source");
        assert_eq!(ln_hold_total.load(Ordering::Relaxed), held + 1);
        let uptime = TIME_UPTIME.load(Ordering::Relaxed);
        assert_eq!(rt.rt_expire().get(), uptime + 1, "RETRANS_TIMER");
        let ns = take_ip6_sent();
        assert_eq!(ns.len(), 1);
        let (ty, dst, target) = nd_sent(&ns[0].bytes);
        assert_eq!((ty, target), (ND_NEIGHBOR_SOLICIT, peer6()));
        assert_eq!(dst, solicited_node(ifp, peer6()));
        assert_eq!((ns[0].ifidx, ns[0].hlim), (ifp.if_index.get() as u16, 255));
        // The prompting packet's source is no link-local one for a target that is not either:
        // the C's source selection, the route to the solicited-node group, whose address is
        // the interface's link-local one.
        let ll = crate::netinet6::in6::in6ifa_ifpforlinklocal(ifp, 0).expect("link-local");
        assert_eq!(
            pod::at::<In6Addr>(&ns[0].bytes, 8),
            Some(ll.ia_addr.addr()),
            "the source the C selects"
        );
        assert_eq!(&ns[0].bytes[64..66], &[ND_OPT_SOURCE_LINKADDR, 1]);
        assert_eq!(&ns[0].bytes[66..72], &OURS);

        // The solicited advertisement: REACHABLE for the interface's reachable time, the held
        // packet sent to the neighbor's address.
        advertise(
            ifp,
            peer6(),
            ND_NA_FLAG_SOLICITED | ND_NA_FLAG_OVERRIDE,
            Some(PEER),
        );
        let (state, asked, _, n, _) = nd6_entry(&rt).expect("the entry");
        assert_eq!((state, asked, n), (ND6_LLINFO_REACHABLE, 0, 0));
        assert_eq!(gw_lladdr_of(&rt), Some(PEER));
        let frames = sent(ifp);
        assert_eq!(frames.len(), 1, "the held packet");
        assert_eq!(&frames[0][..6], &PEER, "to the neighbor's hardware address");
        assert_eq!(&frames[0][14 + 24..14 + 40], &peer6().s6_addr);
        assert_eq!(ln_hold_total.load(Ordering::Relaxed), held);
        let reachable = ifp.if_nd.reachable();
        assert!((14..=44).contains(&reachable), "ND_COMPUTE_RTIME(30000)");
        assert_eq!(rt.rt_expire().get(), uptime + i64::from(reachable));

        // Its timer runs out: STALE for a day.
        let t = rt.rt_expire().get();
        tick(t);
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_STALE);
        assert_eq!(rt.rt_expire().get(), t + i64::from(ND6_GCTIMER));

        // A packet to a STALE neighbor goes out at once and starts the DELAY timer.
        assert_eq!(resolve(ifp, &rt, peer6()), Ok(PEER));
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_DELAY);
        assert_eq!(
            rt.rt_expire().get(),
            t + i64::from(ND6_DELAY.load(Ordering::Relaxed))
        );

        // No confirmation within the delay: PROBE with unicast solicitations, one a second.
        let t = rt.rt_expire().get();
        tick(t);
        let (state, asked, ..) = nd6_entry(&rt).expect("the entry");
        assert_eq!((state, asked), (ND6_LLINFO_PROBE, 1));
        let ns = take_ip6_sent();
        assert_eq!(nd_sent(&ns[0].bytes).1, peer6(), "unicast");
        assert_eq!(ns[0].ifidx, 0, "no multicast interface for a unicast probe");
        tick(t + 1);
        let (state, asked, ..) = nd6_entry(&rt).expect("the entry");
        assert_eq!((state, asked), (ND6_LLINFO_PROBE, 2));
        let _ = take_ip6_sent();

        // An unsolicited advertisement with the same address and no override changes nothing;
        // a solicited one makes the entry REACHABLE again.
        advertise(ifp, peer6(), 0, Some(PEER));
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_PROBE);
        advertise(ifp, peer6(), ND_NA_FLAG_SOLICITED, Some(PEER));
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_REACHABLE);
        // A new address without override: REACHABLE becomes STALE, the address is kept.
        let other = [1, 2, 3, 4, 5, 6];
        advertise(ifp, peer6(), 0, Some(other));
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_STALE);
        assert_eq!(gw_lladdr_of(&rt), Some(PEER));
        // With override it is recorded.
        advertise(ifp, peer6(), ND_NA_FLAG_OVERRIDE, Some(other));
        assert_eq!(gw_lladdr_of(&rt), Some(other));
        let _ = sent(ifp);
    }

    /// The link-layer address a neighbor route's gateway holds, if resolved.
    fn gw_lladdr_of(rt: &Rtentry) -> Option<[u8; 6]> {
        (nd6_lladdr_alen(rt) == 6).then(|| crate::netinet::if_ether::gw_lladdr(rt))
    }

    #[test]
    fn a_neighbor_that_never_answers_gets_its_packets_dropped_and_its_entry_freed() {
        let _g = setup();
        let ifp = ether(b"nd1");
        dad_done(ifp);
        let rt = route6(peer6(), true).expect("the cloned neighbor route");
        let held = ln_hold_total.load(Ordering::Relaxed);

        for _ in 0..3 {
            assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        }
        let (_, asked, _, n, _) = nd6_entry(&rt).expect("the entry");
        assert_eq!((asked, n), (1, 3), "one solicitation for the three packets");
        assert_eq!(take_ip6_sent().len(), 1);

        // nd6_mmaxtries solicitations in all, one per second.
        for asked in 2..=3 {
            tick(rt.rt_expire().get());
            let (state, a, ..) = nd6_entry(&rt).expect("the entry");
            assert_eq!((state, a), (ND6_LLINFO_INCOMPLETE, asked));
        }
        assert_eq!(take_ip6_sent().len(), 2);

        // Then the held packets are dropped with an ICMPv6 unreachable each, and the entry and
        // its route go (the timer's nd6_free).
        tick(rt.rt_expire().get());
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_ADDR); 3]
        );
        assert_eq!(ln_hold_total.load(Ordering::Relaxed), held);
        assert!(nd6_entry(&rt).is_none(), "the record went with RTM_DELETE");
        assert!(!rt.rt_flags.isset(RTF_LLINFO));
        assert!(
            route6(peer6(), false).is_none_or(|r| !r.rt_flags.isset(RTF_LLINFO)),
            "the route deleted"
        );
    }

    #[test]
    fn the_hold_queue_and_the_hold_total_are_bounded() {
        let _g = setup();
        let ifp = ether(b"nd2");
        dad_done(ifp);
        let rt = route6(peer6(), true).expect("the cloned neighbor route");
        let base = ln_hold_total.load(Ordering::Relaxed);

        // LN_HOLD_QUEUE packets per entry: the twelfth and the eleventh push out the oldest,
        // the total follows.
        for _ in 0..12 {
            assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        }
        assert_eq!(nd6_entry(&rt).expect("the entry").3, LN_HOLD_QUEUE);
        assert_eq!(ln_hold_total.load(Ordering::Relaxed), base + LN_HOLD_QUEUE);

        // LN_HOLD_TOTAL in all the queues: over it, the entry's queue is purged and the new
        // packet dropped.
        ln_hold_total.store(LN_HOLD_TOTAL, Ordering::Relaxed);
        assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        assert_eq!(nd6_entry(&rt).expect("the entry").3, 0, "purged");
        assert_eq!(
            ln_hold_total.load(Ordering::Relaxed),
            LN_HOLD_TOTAL - LN_HOLD_QUEUE
        );
        ln_hold_total.store(base, Ordering::Relaxed);
        let _ = take_ip6_sent();
    }

    #[test]
    fn the_timer_walks_past_the_entries_it_frees() {
        let _g = setup();
        let ifp = ether(b"nd3");
        dad_done(ifp);
        let listed = nd6_listed();
        let peers = [a6("fd00:77::10"), a6("fd00:77::11"), a6("fd00:77::12")];
        let rts: Vec<PoolArc<Rtentry>> = peers
            .iter()
            .map(|a| route6(*a, true).expect("a neighbor"))
            .collect();
        assert_eq!(nd6_listed(), listed + 3);
        // Each in STALE (garbage collected when its timer expires), all due now.
        let now = TIME_UPTIME.load(Ordering::Relaxed);
        {
            let _w = NETLOCK.write();
            for rt in &rts {
                nd6_entry_set_for_tests(rt, ND6_LLINFO_STALE, Some(0));
            }
        }
        tick(now + 1);
        for rt in &rts {
            assert!(nd6_entry(rt).is_none(), "freed by the walk");
        }
        assert_eq!(nd6_listed(), listed, "no marker left, the others kept");
        {
            let g = ND6_MTX.lock();
            assert!(ND6_LIST.iter(&g).all(|ln| ln.ln_rt.is_some()));
        }
    }

    #[test]
    fn solicitations_are_answered_and_teach_the_solicitor_s_address() {
        let _g = setup();
        let ifp = ether(b"nd4");
        dad_done(ifp);

        // To our solicited-node group with the source option: an entry for the solicitor
        // (STALE, its address recorded) and a solicited advertisement with our address in the
        // target option, override set.
        solicit(
            ifp,
            peer6(),
            solicited_node(ifp, ours6()),
            ours6(),
            Some(PEER),
        );
        let (state, ..) = entry(peer6()).expect("the solicitor's entry");
        assert_eq!(state, ND6_LLINFO_STALE);
        let rt = route6(peer6(), false).expect("the route");
        assert_eq!(gw_lladdr_of(&rt), Some(PEER));
        let na = take_ip6_sent();
        assert_eq!(na.len(), 1);
        let (ty, dst, target) = nd_sent(&na[0].bytes);
        assert_eq!((ty, dst, target), (ND_NEIGHBOR_ADVERT, peer6(), ours6()));
        let adv = pod::at::<NdNeighborAdvert>(&na[0].bytes, IP6_HDR_LEN).expect("the NA");
        assert_eq!(
            adv.nd_na_flags_reserved(),
            ND_NA_FLAG_SOLICITED | ND_NA_FLAG_OVERRIDE
        );
        assert_eq!(&na[0].bytes[64..66], &[ND_OPT_TARGET_LINKADDR, 1]);
        assert_eq!(&na[0].bytes[66..72], &OURS);
        drop(rt);

        // Unicast to us without the option: no target option in the answer (RFC 2461 7.2.4),
        // so no override either; the entry is not overwritten.
        solicit(ifp, peer6(), ours6(), ours6(), None);
        let na = take_ip6_sent();
        assert_eq!(na.len(), 1);
        let adv = pod::at::<NdNeighborAdvert>(&na[0].bytes, IP6_HDR_LEN).expect("the NA");
        assert_eq!(adv.nd_na_flags_reserved(), ND_NA_FLAG_SOLICITED);
        assert_eq!(
            na[0].bytes.len(),
            IP6_HDR_LEN + size_of::<NdNeighborAdvert>()
        );

        // A solicitation for an address we do not have: silently ignored.
        solicit(
            ifp,
            peer6(),
            solicited_node(ifp, a6("fd00:77::99")),
            a6("fd00:77::99"),
            Some(PEER),
        );
        assert!(take_ip6_sent().is_empty());
        // A bad one (hop limit 254) counted.
        let before = icmp6stat(Icmp6statCounters::Icp6sBadns);
        let (m, len) = nd_packet(
            ifp,
            peer6(),
            ours6(),
            254,
            &nd_msg(ND_NEIGHBOR_SOLICIT, 0, ours6()),
            &[],
        );
        {
            let w = NETLOCK.write();
            nd6_ns_input(m, 40, len, &NetHeld::of(&w).expect("the net lock"));
        }
        assert_eq!(icmp6stat(Icmp6statCounters::Icp6sBadns), before + 1);
        let _ = sent(ifp);
    }

    #[test]
    fn an_advertisement_without_the_option_leaves_an_incomplete_entry_alone() {
        let _g = setup();
        let ifp = ether(b"nd5");
        dad_done(ifp);
        let rt = route6(peer6(), true).expect("the neighbor");
        assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        // The link has addresses: an advertisement without one changes nothing.
        advertise(ifp, peer6(), ND_NA_FLAG_SOLICITED, None);
        let (state, _, _, n, _) = nd6_entry(&rt).expect("the entry");
        assert_eq!((state, n), (ND6_LLINFO_INCOMPLETE, 1));
        // A bad advertisement (multicast target) counted.
        let before = icmp6stat(Icmp6statCounters::Icp6sBadna);
        advertise(ifp, IN6ADDR_LINKLOCAL_ALLNODES_TEST, 0, Some(PEER));
        assert_eq!(icmp6stat(Icmp6statCounters::Icp6sBadna), before + 1);
        let _ = take_ip6_sent();
        let _ = sent(ifp);
    }

    /// ff02::1, a multicast target.
    const IN6ADDR_LINKLOCAL_ALLNODES_TEST: In6Addr = In6Addr {
        s6_addr: [0xff, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    };

    #[test]
    fn dad_makes_a_silent_address_preferred_and_reuses_its_record() {
        let _g = setup();
        let ifp = ether(b"nd6");
        let ia6 = ours_ia6(ifp);
        assert_ne!(
            ia6.ia6_flags.get() & IN6_IFF_TENTATIVE,
            0,
            "tentative at first"
        );
        assert_eq!(dad_counts(&ia6.ia_ifa), Some((1, 0, 0)), "one probe out");
        let refs = ia6.ia_ifa.ifa_refcnt.read();

        // ip6_dad_count probes, no answer: preferred, the routers told (RFC 9131), the record
        // destroyed, then reaped: the address's reference back, the record on the free list.
        dad_fire(&ia6.ia_ifa);
        assert_eq!(IP6_DAD_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(
            ia6.ia6_flags.get() & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED),
            0
        );
        let na = take_ip6_sent();
        let (ty, dst, target) = nd_sent(&na[0].bytes);
        assert_eq!(ty, ND_NEIGHBOR_ADVERT);
        assert_eq!(target, ours6());
        assert_eq!(dst, zoned(a6("ff02::2"), ifp.if_index.get()));
        assert!(dad_counts(&ia6.ia_ifa).is_none());
        let (reap0, free0) = dad_free_counts();
        assert!(reap0 >= 1);
        dad_reap();
        assert_eq!(dad_free_counts(), (0, free0 + reap0));
        assert_eq!(
            ia6.ia_ifa.ifa_refcnt.read(),
            refs - 1,
            "DAD's reference given back"
        );

        // A second address: its DAD takes a record off the free list.
        add(ifp, a6("fd00:88::1"), 64).expect("SIOCAIFADDR_IN6");
        assert_eq!(dad_free_counts(), (0, free0 + reap0 - 1), "a record reused");
        let ia2 = in6ifa_ifpwithaddr(ifp, &a6("fd00:88::1")).expect("the address");
        assert_eq!(dad_counts(&ia2.ia_ifa), Some((1, 0, 0)));
        // Its probe: from :: (IPV6_UNSPECSRC), no source link-layer option.
        let probes: Vec<_> = take_ip6_sent()
            .into_iter()
            .filter(|p| p.bytes[IP6_HDR_LEN] == ND_NEIGHBOR_SOLICIT)
            .collect();
        assert_eq!(probes.len(), 1);
        assert_eq!(nd_sent(&probes[0].bytes).2, a6("fd00:88::1"));
        assert_eq!(probes[0].flags, crate::netinet6::ip6_var::IPV6_UNSPECSRC);
        assert_eq!(
            pod::at::<In6Addr>(&probes[0].bytes, 8),
            Some(In6Addr::default())
        );
        assert_eq!(probes[0].bytes.len(), IP6_HDR_LEN + 24, "no option");
    }

    #[test]
    fn dad_finds_a_duplicate_by_a_probe_or_an_advertisement() {
        let _g = setup();
        let ifp = ether(b"nd7");
        let ia6 = ours_ia6(ifp);

        // Another node probes for our tentative address after our probe: counted, decided at
        // the timer: duplicated.
        solicit(
            ifp,
            In6Addr::default(),
            solicited_node(ifp, ours6()),
            ours6(),
            None,
        );
        assert_eq!(dad_counts(&ia6.ia_ifa), Some((1, 1, 0)));
        assert_ne!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE, 0);
        dad_fire(&ia6.ia_ifa);
        assert_eq!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE, 0);
        assert_ne!(ia6.ia6_flags.get() & IN6_IFF_DUPLICATED, 0);
        assert!(dad_counts(&ia6.ia_ifa).is_none());

        // An advertisement for another tentative address of ours: duplicated at once.
        add(ifp, a6("fd00:99::1"), 64).expect("SIOCAIFADDR_IN6");
        let ia2 = in6ifa_ifpwithaddr(ifp, &a6("fd00:99::1")).expect("the address");
        assert_ne!(ia2.ia6_flags.get() & IN6_IFF_TENTATIVE, 0);
        advertise(ifp, a6("fd00:99::1"), ND_NA_FLAG_OVERRIDE, Some(PEER));
        assert_ne!(ia2.ia6_flags.get() & IN6_IFF_DUPLICATED, 0);
        assert!(dad_counts(&ia2.ia_ifa).is_none());
        dad_reap();
        let _ = take_ip6_sent();
    }

    #[test]
    fn a_dad_timer_of_a_destroyed_record_finds_nothing() {
        let _g = setup();
        let ifp = ether(b"nd8");
        let ia6 = ours_ia6(ifp);
        let addr = DADQ_ADDR(&ia6.ia_ifa);
        {
            let w = NETLOCK.write();
            crate::netinet6::nd6_nbr::nd6_dad_stop(&ia6.ia_ifa, &NetHeld::of(&w).expect("net"));
        }
        // The timeout that fires late (it waited for the net lock while the address went)
        // names a record no longer on the list: logged, nothing done.
        crate::netinet6::nd6_nbr::dad_timer_at(addr);
        assert_ne!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE, 0, "untouched");
        dad_reap();
    }

    /// The address of `ifa`'s DAD record (its timeout's argument).
    #[allow(non_snake_case)]
    fn DADQ_ADDR(ifa: &crate::net::if_var::Ifaddr) -> usize {
        crate::netinet6::nd6_nbr::dad_record_addr(ifa).expect("a DAD record")
    }

    #[test]
    fn a_neighbor_route_held_by_the_table_alone_dumps_refcnt_0() {
        let _g = setup();
        let ifp = ether(b"nd9");
        dad_done(ifp);
        // A neighbor (UHLc) and our local route (UHLl): each owns a record whose ln_rt counts
        // its route, as the C's bare pointer does not.
        let rt = route6(peer6(), true).expect("the neighbor");
        assert!(rt.rt_llinfo.is_b());
        assert_eq!(
            crate::net::rtable::rtable_read_refcnt(&rt) - 1,
            0,
            "route(8) shows Refs 0"
        );
        let local = route6(ours6(), false).expect("our local route");
        assert!(local.rt_flags.isset(RTF_LOCAL) && local.rt_llinfo.is_b());
        assert_eq!(crate::net::rtable::rtable_read_refcnt(&local) - 1, 0);
        // A second holder is counted, as netstat -r shows it.
        let held = rt.clone();
        assert_eq!(crate::net::rtable::rtable_read_refcnt(&rt) - 1, 1);
        drop(held);
    }

    #[test]
    fn the_ioctls_answer_the_interface_and_a_neighbor() {
        let _g = setup();
        let ifp = ether(b"nda");
        dad_done(ifp);
        let mut ndi = [0u8; size_of::<In6Ndireq>()];
        nd6_ioctl(SIOCGIFINFO_IN6, &mut ndi, ifp).expect("SIOCGIFINFO_IN6");
        let got = pod::at::<In6Ndireq>(&ndi, 0).expect("the request");
        assert_eq!(got.ndi, ifp.if_nd.get());
        assert!(got.ndi.reachable > 0);

        let mut nbi = In6Nbrinfo {
            addr: peer6(),
            ..In6Nbrinfo::default()
        };
        let mut b = [0u8; size_of::<In6Nbrinfo>()];
        assert!(pod::put(&mut b, 0, &nbi));
        assert_eq!(nd6_ioctl(SIOCGNBRINFO_IN6, &mut b, ifp), Err(Errno::EINVAL));
        let rt = route6(peer6(), true).expect("the neighbor");
        assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        nd6_ioctl(SIOCGNBRINFO_IN6, &mut b, ifp).expect("SIOCGNBRINFO_IN6");
        nbi = pod::at::<In6Nbrinfo>(&b, 0).expect("the answer");
        assert_eq!(
            (nbi.state, nbi.asked, nbi.isrouter),
            (i32::from(ND6_LLINFO_INCOMPLETE), 1, 0)
        );
        assert!(nbi.expire != 0, "a wall-clock expiry");
        let _ = take_ip6_sent();
    }

    /// A crafted ND message the input must drop, and the one counter it must move.
    struct Rejected {
        /// The check it breaks (the C's line).
        what: &'static str,
        src: In6Addr,
        dst: In6Addr,
        hlim: u8,
        /// The fixed part.
        msg: Vec<u8>,
        /// The options.
        opts: Vec<u8>,
        /// The `icmp6len` given, when not the message's own length.
        len: Option<i32>,
        /// The counter that moves by one; the others of [`ND_COUNTERS`] stay.
        counter: Icmp6statCounters,
    }

    /// The counters an ND input's rejection may move.
    const ND_COUNTERS: [Icmp6statCounters; 4] = [
        Icmp6statCounters::Icp6sBadns,
        Icmp6statCounters::Icp6sBadna,
        Icmp6statCounters::Icp6sNdBadopt,
        Icmp6statCounters::Icp6sTooshort,
    ];

    /// An ND input (`nd6_ns_input`, `nd6_na_input`).
    type NdInput = fn(MbufBox, i32, i32, &NetHeld<'_>);

    /// Each case of `cases` given to `input` as a packet received on `ifp`, under the exclusive
    /// net lock: its counter, alone of [`ND_COUNTERS`], moved by one; nothing sent, no entry
    /// made for its source.
    fn rejected(ifp: &'static Ifnet, input: NdInput, cases: &[Rejected]) {
        for c in cases {
            let before = ND_COUNTERS.map(icmp6stat);
            let (m, len) = nd_packet(ifp, c.src, c.dst, c.hlim, &c.msg, &c.opts);
            {
                let w = NETLOCK.write();
                let net = NetHeld::of(&w).expect("the net lock");
                input(m, IP6_HDR_LEN as i32, c.len.unwrap_or(len), &net);
            }
            let after = ND_COUNTERS.map(icmp6stat);
            for (i, k) in ND_COUNTERS.iter().enumerate() {
                let moved = u64::from(*k == c.counter);
                assert_eq!(after[i], before[i] + moved, "{}: {k:?}", c.what);
            }
            assert!(take_ip6_sent().is_empty(), "{}: answered", c.what);
            assert!(entry(c.src).is_none(), "{}: an entry made", c.what);
        }
    }

    /// `b` (an IPv6 header and its payload) as a two-mbuf chain, the header mbuf holding the
    /// first `at` bytes, received on `ifp`: what `nd6_msg` must pull up.
    fn chained(ifp: &Ifnet, b: &[u8], at: usize) -> MbufBox {
        let mut m = m_devget(&b[..at], 0).expect("an mbuf");
        let mut n = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        n.m_len().set((b.len() - at) as u32);
        n.data_mut().expect("room").copy_from_slice(&b[at..]);
        m.m_pkthdr().len.set(b.len() as i32);
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        drop(m.m_next.set(n));
        m
    }

    /// The bytes of a packet `nd_packet` makes.
    fn packet_bytes(src: In6Addr, dst: In6Addr, msg: &[u8], opts: &[u8]) -> Vec<u8> {
        let mut ip6 = Ip6Hdr {
            ip6_plen: htons((msg.len() + opts.len()) as u16),
            ip6_nxt: 58, // IPPROTO_ICMPV6
            ip6_hlim: 255,
            ip6_src: src,
            ip6_dst: dst,
            ..Ip6Hdr::default()
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        let mut b = std::vec![0u8; IP6_HDR_LEN];
        assert!(pod::put(&mut b, 0, &ip6));
        b.extend_from_slice(msg);
        b.extend_from_slice(opts);
        b
    }

    #[test]
    fn bad_solicitations_are_counted_and_dropped() {
        // LZ's table (nd6_nbr.rs's), over real packets through nd6_msg: each case passes every
        // check before the one it breaks, so a check removed lets its case through.
        let _g = setup();
        let ifp = ether(b"ndr0");
        dad_done(ifp);
        let any = In6Addr::default();
        let snm = solicited_node(ifp, ours6());
        let ns = nd_msg(ND_NEIGHBOR_SOLICIT, 0, ours6());
        let src_lla = lla(ND_OPT_SOURCE_LINKADDR, PEER).to_vec();
        // A source option of 16 bytes for an Ethernet address (8).
        let mut long_lla = std::vec![ND_OPT_SOURCE_LINKADDR, 2];
        long_lla.extend_from_slice(&PEER);
        long_lla.extend_from_slice(&[0; 8]);
        let bad = |what, src, dst, hlim, msg: &[u8], opts: &[u8]| Rejected {
            what,
            src,
            dst,
            hlim,
            msg: msg.to_vec(),
            opts: opts.to_vec(),
            len: None,
            counter: Icmp6statCounters::Icp6sBadns,
        };
        let none = Vec::new();
        let mut cases = std::vec![
            bad("nd6_nbr.c:126, hop limit 254", any, snm, 254, &ns, &none),
            bad(
                "nd6_nbr.c:131-137, from :: to a unicast address",
                any,
                ours6(),
                255,
                &ns,
                &none
            ),
            bad(
                "nd6_nbr.c:142, a source that is no neighbor",
                a6("fd00:99::5"),
                ours6(),
                255,
                &ns,
                &src_lla
            ),
            bad(
                "nd6_nbr.c:148, a multicast target",
                any,
                snm,
                255,
                &nd_msg(ND_NEIGHBOR_SOLICIT, 0, snm),
                &none
            ),
            bad(
                "nd6_nbr.c:165, a DAD probe with a source option",
                any,
                snm,
                255,
                &ns,
                &src_lla
            ),
            bad(
                "nd6_nbr.c:243, a source option not 8 bytes",
                peer6(),
                ours6(),
                255,
                &ns,
                &long_lla
            ),
        ];
        // A zero-length option: dropped by nd6_options, counted there alone.
        cases.push(Rejected {
            counter: Icmp6statCounters::Icp6sNdBadopt,
            ..bad(
                "nd6_options, a zero-length option",
                peer6(),
                snm,
                255,
                &ns,
                &std::vec![ND_OPT_SOURCE_LINKADDR, 0, 0, 0, 0, 0, 0, 0],
            )
        });
        // Shorter than a solicitation, and a length past what the chain holds.
        cases.push(Rejected {
            msg: ns[..20].to_vec(),
            counter: Icmp6statCounters::Icp6sTooshort,
            ..bad("nd6_nbr.c:117, 20 bytes", peer6(), snm, 255, &none, &none)
        });
        cases.push(Rejected {
            len: Some(32),
            counter: Icmp6statCounters::Icp6sTooshort,
            ..bad(
                "nd6_nbr.c:117, 32 bytes said, 24 there",
                peer6(),
                snm,
                255,
                &ns,
                &none,
            )
        });
        rejected(ifp, nd6_ns_input, &cases);

        // The same solicitation, good, its message split over two mbufs (nd6_msg's pullup):
        // answered, the solicitor's entry made.
        let b = packet_bytes(peer6(), snm, &ns, &src_lla);
        let m = chained(ifp, &b, IP6_HDR_LEN + 10);
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            nd6_ns_input(m, IP6_HDR_LEN as i32, (b.len() - IP6_HDR_LEN) as i32, &net);
        }
        let na = take_ip6_sent();
        assert_eq!(na.len(), 1, "answered");
        assert_eq!(
            nd_sent(&na[0].bytes),
            (ND_NEIGHBOR_ADVERT, peer6(), ours6())
        );
        assert_eq!(
            entry(peer6()).expect("the solicitor's entry").0,
            ND6_LLINFO_STALE
        );
        let _ = sent(ifp);
    }

    #[test]
    fn bad_advertisements_are_counted_and_dropped() {
        let _g = setup();
        let ifp = ether(b"ndr1");
        dad_done(ifp);
        let all_nodes = IN6ADDR_LINKLOCAL_ALLNODES_TEST;
        let tlla = lla(ND_OPT_TARGET_LINKADDR, PEER).to_vec();
        let mut long_tlla = std::vec![ND_OPT_TARGET_LINKADDR, 2];
        long_tlla.extend_from_slice(&PEER);
        long_tlla.extend_from_slice(&[0; 8]);
        let solicited = nd_msg(ND_NEIGHBOR_ADVERT, ND_NA_FLAG_SOLICITED, peer6());
        let bad = |what, dst, hlim, msg: &[u8], opts: &[u8]| Rejected {
            what,
            src: peer6(),
            dst,
            hlim,
            msg: msg.to_vec(),
            opts: opts.to_vec(),
            len: None,
            counter: Icmp6statCounters::Icp6sBadna,
        };
        let none = Vec::new();
        let mut cases = std::vec![
            bad(
                "nd6_nbr.c:537, hop limit 64",
                ours6(),
                64,
                &solicited,
                &tlla
            ),
            bad(
                "nd6_nbr.c:555, a multicast target",
                ours6(),
                255,
                &nd_msg(ND_NEIGHBOR_ADVERT, 0, all_nodes),
                &tlla
            ),
            bad(
                "nd6_nbr.c:557, solicited, to a group",
                all_nodes,
                255,
                &solicited,
                &tlla
            ),
            bad(
                "nd6_nbr.c:566, to a group without the target option",
                all_nodes,
                255,
                &nd_msg(ND_NEIGHBOR_ADVERT, ND_NA_FLAG_OVERRIDE, peer6()),
                &none
            ),
            bad(
                "nd6_nbr.c:616, a target option not 8 bytes",
                ours6(),
                255,
                &solicited,
                &long_tlla
            ),
        ];
        cases.push(Rejected {
            counter: Icmp6statCounters::Icp6sNdBadopt,
            ..bad(
                "nd6_options, a zero-length option",
                ours6(),
                255,
                &solicited,
                &std::vec![ND_OPT_TARGET_LINKADDR, 0, 0, 0, 0, 0, 0, 0],
            )
        });
        cases.push(Rejected {
            counter: Icmp6statCounters::Icp6sTooshort,
            ..bad(
                "nd6_nbr.c:540, 20 bytes",
                ours6(),
                255,
                &solicited[..20],
                &none,
            )
        });
        cases.push(Rejected {
            len: Some(32),
            counter: Icmp6statCounters::Icp6sTooshort,
            ..bad(
                "nd6_nbr.c:540, 32 bytes said, 24 there",
                ours6(),
                255,
                &solicited,
                &none,
            )
        });
        rejected(ifp, nd6_na_input, &cases);

        // A good unsolicited advertisement for a neighbor a host has no entry for: ignored, not
        // counted as bad.
        let before = ND_COUNTERS.map(icmp6stat);
        let (m, len) = nd_packet(
            ifp,
            peer6(),
            all_nodes,
            255,
            &nd_msg(ND_NEIGHBOR_ADVERT, ND_NA_FLAG_OVERRIDE, peer6()),
            &tlla,
        );
        {
            let w = NETLOCK.write();
            nd6_na_input(m, IP6_HDR_LEN as i32, len, &NetHeld::of(&w).expect("net"));
        }
        assert_eq!(ND_COUNTERS.map(icmp6stat), before);
        assert!(entry(peer6()).is_none(), "a host makes no entry");

        // A solicited one for an INCOMPLETE entry, split over two mbufs (nd6_msg's pullup):
        // REACHABLE.
        let rt = route6(peer6(), true).expect("the neighbor");
        assert_eq!(resolve(ifp, &rt, peer6()), Err(Errno::EAGAIN));
        let _ = take_ip6_sent();
        let b = packet_bytes(peer6(), ours6(), &solicited, &tlla);
        let m = chained(ifp, &b, IP6_HDR_LEN + 6);
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            nd6_na_input(m, IP6_HDR_LEN as i32, (b.len() - IP6_HDR_LEN) as i32, &net);
        }
        assert_eq!(nd6_entry(&rt).expect("the entry").0, ND6_LLINFO_REACHABLE);
        assert_eq!(gw_lladdr_of(&rt), Some(PEER));
        let _ = sent(ifp);
    }

    #[test]
    fn dad_counts_its_probes_once_and_holds_its_address_until_reaped() {
        let _g = setup();
        let ifp = ether(b"ndd0");
        let ia6 = ours_ia6(ifp);
        // ether()'s address: DAD started once, its one probe sent.
        let pending = IP6_DAD_PENDING.load(Ordering::Relaxed);
        assert!(pending >= 1);
        let count = IP6_DAD_COUNT.load(Ordering::Relaxed);
        assert_eq!(dad_tries(&ia6.ia_ifa), Some((count, 1, 1)));
        let addr = DADQ_ADDR(&ia6.ia_ifa);
        let refs = ia6.ia_ifa.ifa_refcnt.read();

        // A second start for the same address does nothing: no record, no probe, no reference.
        {
            let w = NETLOCK.write();
            nd6_dad_start(&ia6.ia_ifa, &NetHeld::of(&w).expect("the net lock"));
        }
        assert_eq!(IP6_DAD_PENDING.load(Ordering::Relaxed), pending);
        assert_eq!(DADQ_ADDR(&ia6.ia_ifa), addr);
        assert_eq!(dad_tries(&ia6.ia_ifa), Some((count, 1, 1)));
        assert!(take_ip6_sent().is_empty(), "no second probe");
        assert_eq!(ia6.ia_ifa.ifa_refcnt.read(), refs);

        // The verdict: the record off the list, one DAD less pending; the address's reference
        // is DAD's until the reaper runs.
        dad_fire(&ia6.ia_ifa);
        assert!(dad_tries(&ia6.ia_ifa).is_none());
        assert_eq!(IP6_DAD_PENDING.load(Ordering::Relaxed), pending - 1);
        assert_eq!(ia6.ia_ifa.ifa_refcnt.read(), refs, "DAD's until reaped");
        dad_reap();
        assert_eq!(ia6.ia_ifa.ifa_refcnt.read(), refs - 1);
        let _ = take_ip6_sent();
    }

    #[test]
    fn dad_on_a_down_interface_counts_tries_and_loses_to_any_probe() {
        let _g = setup();
        let ifp = ether(b"ndd1");
        let ia6 = ours_ia6(ifp);
        {
            let w = NETLOCK.write();
            nd6_dad_stop(&ia6.ia_ifa, &NetHeld::of(&w).expect("the net lock"));
        }
        dad_reap();
        let _ = take_ip6_sent();

        // Down: the try counts, no probe goes out.
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_UP);
        assert_ne!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE, 0);
        {
            let w = NETLOCK.write();
            nd6_dad_start(&ia6.ia_ifa, &NetHeld::of(&w).expect("the net lock"));
        }
        let count = IP6_DAD_COUNT.load(Ordering::Relaxed);
        assert_eq!(dad_tries(&ia6.ia_ifa), Some((count, 1, 0)));
        assert!(take_ip6_sent().is_empty());

        // A probe from someone else before ours: it wins at once (nd6_dad_ns_input).
        solicit(
            ifp,
            In6Addr::default(),
            solicited_node(ifp, ours6()),
            ours6(),
            None,
        );
        assert_eq!(ia6.ia6_flags.get() & IN6_IFF_TENTATIVE, 0);
        assert_ne!(ia6.ia6_flags.get() & IN6_IFF_DUPLICATED, 0);
        assert!(dad_tries(&ia6.ia_ifa).is_none());
        dad_reap();
        ifp.if_flags.set(ifp.if_flags.get() | IFF_UP);
        let _ = take_ip6_sent();
    }

    #[test]
    fn an_anycast_address_skips_dad() {
        let _g = setup();
        let ifp = ether(b"ndd2");
        dad_done(ifp);
        let pending = IP6_DAD_PENDING.load(Ordering::Relaxed);
        let any6 = a6("fd00:77::aa");
        let mut r = aliasreq(ifp, any6, 64, IN6_IFF_ANYCAST);
        in6_ioctl(SIOCAIFADDR_IN6, &mut r, Some(ifp), true).expect("SIOCAIFADDR_IN6");
        let ia = in6ifa_ifpwithaddr(ifp, &any6).expect("the anycast address");
        let flags = ia.ia6_flags.get();
        assert_ne!(flags & IN6_IFF_ANYCAST, 0);
        assert_eq!(
            flags & (IN6_IFF_TENTATIVE | IN6_IFF_DUPLICATED),
            0,
            "usable at once"
        );
        assert!(dad_tries(&ia.ia_ifa).is_none());
        assert_eq!(IP6_DAD_PENDING.load(Ordering::Relaxed), pending);
        assert!(
            !take_ip6_sent()
                .iter()
                .any(|p| p.bytes[IP6_HDR_LEN] == ND_NEIGHBOR_SOLICIT),
            "no probe"
        );
    }

    #[test]
    fn nd6_ifptomac_is_the_ethernet_address() {
        let _g = setup();
        let ifp = ether(b"ndd3");
        assert_eq!(nd6_ifptomac(ifp), Some(OURS));
        // An Ethernet-typed interface without its arpcom, and a loopback: none.
        let bare: &'static Ifnet = Box::leak(Box::new(Ifnet::new()));
        bare.if_type.set(IFT_ETHER);
        assert_eq!(nd6_ifptomac(bare), None);
        bare.if_type.set(IFT_LOOP);
        assert_eq!(nd6_ifptomac(bare), None);
    }

    #[test]
    fn a_lifetime_past_int_max_seconds_schedules_the_expiry_at_once() {
        // nd6.c:446-450: the difference to the expiry truncated to an int, then clamped at 0.
        // Root may give finite lifetimes of 3000000000 seconds: the C schedules at 0 (one tick).
        let _g = setup();
        let ifp = ether(b"nde0");
        dad_done(ifp);
        let addr = a6("fd00:66::1");
        let mut r = aliasreq(ifp, addr, 64, 0);
        let mut ifra = pod::at::<In6Aliasreq>(&r, 0).expect("the request");
        ifra.ifra_lifetime.ia6t_vltime = 3_000_000_000;
        ifra.ifra_lifetime.ia6t_pltime = 3_000_000_000;
        assert!(pod::put(&mut r, 0, &ifra));
        assert!(
            !ND6_EXPIRE_TIMEOUT.pending(),
            "ether()'s lifetimes are infinite"
        );
        let now = crate::clock::ticks();
        in6_ioctl(SIOCAIFADDR_IN6, &mut r, Some(ifp), true).expect("SIOCAIFADDR_IN6");
        assert!(ND6_EXPIRE_TIMEOUT.pending());
        let due = ND6_EXPIRE_TIMEOUT.time().wrapping_sub(now);
        assert!(
            (1..100).contains(&due),
            "due in {due} ticks: timeout_add_sec(0)"
        );
        assert_eq!(
            ND6_EXPIRE_NEXT.load(Ordering::Relaxed),
            TIME_UPTIME.load(Ordering::Relaxed) + 3_000_000_001
        );
        let _ = take_ip6_sent();
    }

    #[test]
    fn the_records_sizes() {
        // The pool item (vmstat -m's "nd6") and DAD's malloc(M_IP6NDP), against the C's on
        // LP64 (replicas of struct llinfo_nd6 and struct dadq) and LZ's (D-4, flagged).
        #[repr(C)]
        struct CMutex(*const u8, i32, i32);
        #[repr(C)]
        struct CMbufQueue(CMutex, *const u8, *const u8, u32, u32, u32);
        #[repr(C)]
        struct CLlinfoNd6(
            [*const u8; 2],
            *const u8,
            [u32; 2],
            CMbufQueue,
            [u32; 4],
            i64,
            i16,
            i16,
        );
        #[repr(C)]
        struct CTimeout(
            [*const u8; 2],
            [i64; 2],
            *const u8,
            *const u8,
            *const u8,
            [i32; 3],
        );
        #[repr(C)]
        struct CDadq([*const u8; 2], *const u8, [i32; 5], CTimeout);
        std::println!(
            "nd6 item: C {} kernel {} (pool {:?}); dadq: C {} kernel {}",
            size_of::<CLlinfoNd6>(),
            size_of::<LlinfoNd6>(),
            ND6_POOL.item_size(),
            size_of::<CDadq>(),
            size_of::<Dadq>()
        );
        std::println!(
            "timeout: C {} kernel {}",
            size_of::<CTimeout>(),
            size_of::<Timeout>()
        );
        // nd6: C 112, LZ 120 (its LlinfoNd6 over bsd's types, measured once at N6.5b's base),
        // kernel 128: the 32-byte PinEntry for the C's 16-byte TAILQ_ENTRY (+16), the 56-byte
        // MbufQueue for the C's 48 (+8), no ln_refcnt (-8). dadq: C 120, kernel 200 (the
        // kernel's 120-byte Timeout for the C's 72-byte struct timeout, a second list entry:
        // the reaper's and the free list's).
        assert_eq!(size_of::<CLlinfoNd6>(), 112);
        assert_eq!(size_of::<LlinfoNd6>(), 128);
        assert_eq!(ND6_POOL.item_size().unwrap_or(128), 128);
        assert_eq!(size_of::<CDadq>(), 120);
        assert_eq!(size_of::<Dadq>(), 200);
    }
}
/* </TESTS> */
