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
 * Copyright (c) 1980, 1986, 1993
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
 *	@(#)route.h	8.3 (Berkeley) 4/19/94
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
 * Copyright (c) 1980, 1986, 1991, 1993
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
 *	@(#)route.c	8.2 (Berkeley) 11/15/93
 */

/*
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
//! Routes: the kernel half of `<net/route.h>` and `net/route.c`: the route entries, the
//! per-protocol route caches, the requests that add, clone and delete routes, the route
//! timers and the route labels.
//!
//! Upstream: sys/net/route.h @ 3ce1f3f79392
//! Upstream: sys/net/route.c @ 3ce1f3f79392
//! LZ: sys/net/route.rs@3c62ede68802
//!
//! Kernel resident routing tables. The routing tables are initialized when interface addresses
//! are set by making entries for all directly connected interfaces. Routes to hosts are
//! distinguished from routes to networks, preferring the former if available; for each route
//! the interface to use is inferred from the gateway address supplied when the route was
//! entered. Routes that forward packets through gateways are marked with `RTF_GATEWAY` so that
//! the output routines know to address the gateway rather than the ultimate destination. With
//! `RTF_GATEWAY` set, `rt_gwroute` names the route of the gateway (the next hop's link-layer
//! route); without it, `rt_cachecnt` counts the `RTF_GATEWAY` routes whose `rt_gwroute`
//! names this one.
//!
//! The locks named in the members' docs are the C's: \[I\] immutable after creation, \[N\] net
//! lock, \[X\] exclusive net lock (or shared net lock and kernel lock), \[R\] the rtable lock,
//! \[r\] the route's `rt_mtx`, \[L\] the ARP/ND lock for updates, net lock for reads, \[T\]
//! `rttimer_mtx`, \[a\] atomic operations. The licence block carries the NRL notice with its
//! advertising clause, accepted as BSD-4 (`.claude/rules/scope-and-stubs.md`).
//!
//! ## Deviations
//! - `MPLS` and `BFD` are not configured (LZ's): `rt_mpls_set`/`rt_mpls_clear`, `bfdinit` and
//!   `bfdclear` are comments at their sites; `SMALL_KERNEL` is not defined.
//! - `route6_cache` and `route6_mpath` (the C's `#ifdef INET6`) always compile, as LZ's.
//! - `rt_ifa_del` builds its masked destination in a copy on its frame where the C takes an
//!   `MT_SONAME` mbuf (`m_get(M_DONTWAIT)`, whose failure is `ENOBUFS`).
//! - `rt_setgwroute` answers `EHOSTUNREACH` for a cloned next hop without its parent (a route
//!   `rtrequest_delete` took it from), which the C dereferences.
//! - A C leak fixed, invisible to userland: the failure paths of `rtrequest` release the route
//!   label reference `rtlabel_name2id` took, where the C's keep it (the label's id stays
//!   allocated; no interface shows the count).
//! - `struct rt_addrinfo`'s `rti_rtm`, which no code at the pin reads, is not kept.
//! - `rtrequest(RTM_RESOLVE)` puts the clone's gateway (an empty link-level address) and its
//!   label in its own copy of the request: the C writes pointers to its own frame into the
//!   caller's `info`, which `rt_clone` reads after the return for its `RTM_MISS`; `rt_clone`
//!   builds that message with the same two addresses itself.
//!
//! ## Redesign
//! - `struct rtentry` is an item of the pool `"rtentry"` handled through
//!   [`PoolArc<Rtentry>`]: the count is the C's `rt_refcnt`, `rtref` a clone and `rtfree` a
//!   drop, and the C's last-`rtfree` work is the entry's [`Drop`]. Every C reference is one
//!   handle (D-N6.3b-3), so `rmx_refcnt` (the count less the table's reference) is the C's:
//!
//!   | the C's reference | its handle here |
//!   |---|---|
//!   | `rtable_insert`'s `rtref` (the table's) | the chain word (`an_value`, a predecessor's `rt_next`) |
//!   | `rt_putgwroute`'s `rt_gwroute` | `rt_gwroute`'s `Option<PoolArc>` |
//!   | `rtrequest`'s `rtref(*ret_nrt)` for `rt_parent` | `rt_parent`'s `Option<PoolArc>` |
//!   | `struct route`'s `ro_rt` | [`Route`]'s `ro_rt` |
//!   | `rtalloc`, `rtable_lookup`, `rtable_match`, `*ret_nrt` answers | the `PoolArc` answered |
//!   | `pool_get`'s first reference | the handle `rtrequest` holds until it answers |
//!   | (none: `rtt_rt` is a bare pointer) | a timer's `rtt_rt`, subtracted by [`rt_refcnt`] |
//!   | (none: an unlinked route's `rt_next` is a bare pointer) | the `rt_next` an unlinked route keeps, subtracted by [`rt_refcnt`] (`rt_backrefs`) |
//!   | (none: `rtfree` drops it at once) | a chain word's reference retired and not released yet, subtracted by [`rt_refcnt`] (`hal::smr::retired_count`) |
//!
//!   A timer holds a counted handle of its route (R11 of N6.3b1): kernel code cannot follow
//!   the C's bare back pointer. So a route's timers are removed when it leaves its table
//!   (`rtrequest_delete`), where the C's last `rtfree` usually follows, and [`rt_refcnt`]
//!   (`refcnt_read`) does not count them. A deleted route kept by a cache whose timer is
//!   added afterwards keeps it until the timer fires, where the C's timer dies with the
//!   route's last `rtfree`.
//! - The members are atomics, read-mostly cells and links by the C's legend: `rt_dest` an
//!   owned `M_RTABLE` copy, immutable; `rt_gateway`, `rt_gwroute` and `rt_parent`
//!   `ReadMostly` cells (\[X\]/\[N\]: no single witness expresses "exclusive net lock, or shared
//!   with the kernel lock", and `bsd`'s readers hold the net lock detached); `rt_flags`,
//!   `rt_ifidx`, `rt_labelid`, `rt_priority` relaxed atomics; `rt_ifa` a `Link` to the address
//!   (`bsd`'s storage until N6.4, D-N6.3a-3); `rt_llinfo` a word (D-N6.3b-10); `rt_cachecnt`
//!   inside `rt_mtx`'s `Mutexed`; `rt_timer` a `PinList` under `RTTIMER_MTX`'s witness.
//! - `struct route` is [`Route`], owned and changed through `&mut` (D-N6.3b-7): the C's
//!   `route_cache`/`route_mpath` are [`Route::cache4`] and [`Route::mpath4`] (`route6_*` the
//!   `6` twins); `Netstack::ns_route` holds one per softnet.
//! - `struct rt_addrinfo`'s addresses are [`SockaddrView`]s, bounded byte views, so no
//!   function here reads past a socket address.
//! - The timers: an [`Rttimer`] is an item of `"rttmr"` (a recorded pool), on two `PinList`s
//!   under the one `RTTIMER_MTX` (its queue's `rtq_head` and its route's `rt_timer`), with a
//!   reference parked for that membership and one held by its timeout's argument
//!   (`Timeout::set_shared`, the C's `timeout_set_proc`); `rt_timer_unlink`'s
//!   `timeout_del() == 0` branch is kept. The labels: one `Mutexed` chain of `M_RTABLE`
//!   boxes in the C's order, scanned linearly as the C's `TAILQ`.
//! - `rtcounters` is a `Counters` of `rts_ncounters`; `rttrash` and `rtgeneration` atomics
//!   (`rtgeneration` bumped with `Release`, read with `Acquire`: the C's `membar_producer`
//!   and `membar_consumer`).
//! - The final `rtfree` of a deleted route may run on the SMR thread (D-N6.3b-23): the
//!   table's reference is released after a grace period (`kernel::net::rtable`).
//! - Lock and spl transitions against the C (route.c's lines):
//!
//!   | operation | the C | here |
//!   |---|---|---|
//!   | `rtalloc` (:485-493) | `rtable_match`'s two read sections | the same; `rt_use` one atomic |
//!   | `rtrequest(RTM_ADD)` (:956-1130) | `r_lock` in `rtable_insert`, `rt_mtx` in `rt_putgwroute`, `KERNEL_LOCK` in `rt_setgate` | the same, and `rt_gateway`'s and `rt_gwroute`'s cells (an atomic each) |
//!   | `rtrequest_delete` (:894-954) | `r_lock` in `rtable_lookup`/`rtable_delete`, `rt_mtx`, `rttimer_mtx` at the last `rtfree` | the same; `RTTIMER_MTX` taken at the delete (R11) |
//!   | `rt_setgate` (:1132-1166) | `KERNEL_LOCK` | the same |
//!   | `rt_timer_timer` (:1693-1716) | `NET_LOCK`, `rttimer_mtx` | the same |
//!
//! - Costs against the C: `rtisvalid` of a gateway route reads `rt_gwroute`'s cell (one
//!   compare-and-swap, the `ReadMostly` row of N12); `Route::cache4`'s hit is the C's loads.

use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};

use hal::intr::{IPL_MPFLOOR, IPL_NET, IPL_SOFTNET};
use hal::klock;
use hal::link::{Link, PtrWord};
use hal::malloc::{M_NOWAIT, MArray, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::netlock::NetHeld;
use hal::once::OnceStatic;
use hal::percpu::Counters;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{AnyArc, PR_NOWAIT, PoolArc, PoolHome, TypedPool};
use hal::rwlock::Guarded;
use hal::slot::ReadMostly;
use hal::smr::{SmrArcLink, SmrArcWord, SmrCounted};
use hal::sys::errno::Errno;
use hal::sys::net_if::{IFF_BROADCAST, IFF_LOOPBACK, IFF_POINTOPOINT, IFF_UP, link_state_is_up};
use hal::sys::net_route::{
    RTAX_AUTHOR, RTAX_DST, RTAX_GATEWAY, RTAX_IFA, RTAX_IFP, RTAX_LABEL, RTAX_MAX, RTAX_NETMASK,
    RTF_BROADCAST, RTF_CACHED, RTF_CLONED, RTF_CLONING, RTF_CONNECTED, RTF_DYNAMIC, RTF_GATEWAY,
    RTF_HOST, RTF_LLINFO, RTF_LOCAL, RTF_MODIFIED, RTF_MPATH, RTF_STATIC, RTF_UP, RTLABEL_LEN,
    RTM_ADD, RTM_DELADDR, RTM_DELETE, RTM_MISS, RTM_NEWADDR, RTM_REDIRECT, RTM_RESOLVE,
    RTP_CONNECTED, RTP_DEFAULT, RTP_DOWN, RTP_LOCAL, RTP_MASK, RTP_NONE, RTP_STATIC, RTV_MTU,
    SockaddrRtlabel,
};
use hal::sys::socket::{AF_INET, AF_INET6, AF_LINK, AF_MAX, AF_UNSPEC};
use hal::sys::types::SaFamily;

use crate::assert::{kassert, kpanic};
use crate::net::if_::{self, if_get, if_group_routechange, if_put};
use crate::net::if_var::{AtomicCell, Ifaddr, Ifnet};
use crate::net::proto::SockaddrView;
use crate::net::rtable::{
    M_RTABLE, RTMAP_LIMIT, SA_COPY_MAX, SaCopy, rt_dom, rtable_delete, rtable_exists,
    rtable_insert, rtable_iterate, rtable_l2, rtable_lookup, rtable_match, rtable_mpath_capable,
    rtable_mpath_reprio, rtable_satoplen, rtable_walk,
};
use crate::net::rtsock;
use crate::timeout::{KCLOCK_NONE, TIMEOUT_PROC, Timeout};

/// `RT_RESOLVE`: `rtalloc`'s flag to add and resolve the link-layer route of a cloning one.
pub const RT_RESOLVE: i32 = 1;

/// `LABELID_MAX`: the largest route label id.
pub const LABELID_MAX: u16 = 50000;

/// `rts_badredirect`: bogus redirect calls.
pub const RTS_BADREDIRECT: usize = 0;
/// `rts_dynamic`: routes created by redirects.
pub const RTS_DYNAMIC: usize = 1;
/// `rts_newgateway`: routes modified by redirects.
pub const RTS_NEWGATEWAY: usize = 2;
/// `rts_unreach`: lookups which failed.
pub const RTS_UNREACH: usize = 3;
/// `rts_wildcard`: lookups satisfied by a wildcard.
pub const RTS_WILDCARD: usize = 4;
/// `rts_ncounters`.
pub const RTS_NCOUNTERS: usize = 5;

/// `struct sockaddr_in`'s size (netinet/in.h; N6.4 brings the type into `hal::sys`).
const SIN_LEN: usize = 16;
/// Where `sin_addr` starts in a `struct sockaddr_in`.
const SIN_ADDR: usize = 4;
/// `struct sockaddr_in6`'s size.
const SIN6_LEN: usize = 28;
/// Where `sin6_addr` starts in a `struct sockaddr_in6`.
const SIN6_ADDR: usize = 8;

/// `struct sockaddr_dl`'s size, `{ sizeof(sa_dl), AF_LINK }`'s length.
const SDL_LEN: usize = core::mem::size_of::<hal::sys::net_if::SockaddrDl>();

/// `ROUNDUP(a)`: `a` rounded up to a `long`'s size, `sizeof(long)` for 0.
#[inline]
const fn roundup_sa(a: usize) -> usize {
    if a > 0 {
        1 + ((a - 1) | (core::mem::size_of::<u64>() - 1))
    } else {
        core::mem::size_of::<u64>()
    }
}

/// `rt_flags`: the route's `RTF_*` bits, read without a lock and changed under the C's
/// locks (\[X\]; `RTF_CACHED` under `rt_mtx`), each change an atomic read-modify-write so two
/// writers under different locks lose no bit.
#[derive(Default)]
pub struct RtFlags(AtomicU32);

impl RtFlags {
    /// No bit.
    pub const fn new(v: u32) -> Self {
        Self(AtomicU32::new(v))
    }

    /// The bits.
    #[inline]
    pub fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }

    /// The bits set to `v`.
    #[inline]
    pub fn set(&self, v: u32) {
        self.0.store(v, Ordering::Relaxed);
    }

    /// `SET(rt->rt_flags, b)`.
    #[inline]
    pub fn set_bits(&self, b: u32) {
        self.0.fetch_or(b, Ordering::Relaxed);
    }

    /// `CLR(rt->rt_flags, b)`.
    #[inline]
    pub fn clear(&self, b: u32) {
        self.0.fetch_and(!b, Ordering::Relaxed);
    }

    /// `ISSET(rt->rt_flags, b)`: whether any of `b` is set.
    #[inline]
    pub fn isset(&self, b: u32) -> bool {
        self.get() & b != 0
    }
}

/// `struct rt_kmetrics`: the metrics used by the receiving protocols (`rmx_pksent` and
/// `rmx_mtu` changed without a lock, as the C's `rt_use++` and `[a]`).
#[derive(Default)]
pub struct RtKmetrics {
    /// `rmx_pksent`: packets sent using this route.
    pub rmx_pksent: AtomicU64,
    /// `rmx_expire`: lifetime for route.
    pub rmx_expire: AtomicI64,
    /// `rmx_locks`: kernel must leave these values.
    pub rmx_locks: AtomicCell<u32>,
    /// `rmx_mtu`: MTU for this path.
    pub rmx_mtu: AtomicU32,
}

impl RtKmetrics {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            rmx_pksent: AtomicU64::new(0),
            rmx_expire: AtomicI64::new(0),
            rmx_locks: AtomicCell::new(),
            rmx_mtu: AtomicU32::new(0),
        }
    }

    /// `rt->rt_rmx = from->rt_rmx`.
    pub fn assign(&self, from: &RtKmetrics) {
        self.rmx_pksent
            .store(from.rmx_pksent.load(Ordering::Relaxed), Ordering::Relaxed);
        self.rmx_expire
            .store(from.rmx_expire.load(Ordering::Relaxed), Ordering::Relaxed);
        self.rmx_locks.set(from.rmx_locks.get());
        self.rmx_mtu
            .store(from.rmx_mtu.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

/// `struct rtentry`: a route, an item of the pool `"rtentry"` (the module's redesign: the
/// count of its handles is `rt_refcnt`).
pub struct Rtentry {
    /// Where the item's pool recorded it (`PoolArc::from_ref`, `count_of`: `bsd`'s adapter and
    /// the routing socket's count).
    home: PoolHome<Rtentry>,
    /// The retirement link of the chain words' references (`an_value`, `rt_next`).
    link: SmrArcLink,
    /// \[I\] `rt_dest`: the destination, `dom_sasize` bytes of `M_RTABLE`.
    rt_dest: MArray<u8>,
    /// \[I\] `rt_plen`: the prefix length.
    rt_plen: u32,
    /// \[R\] `rt_next`: the next multipath route of the same destination.
    pub(crate) rt_next: SmrArcWord<Rtentry>,
    /// \[X\] `rt_gateway`: the gateway address, `ROUNDUP(sa_len)` bytes of `M_RTABLE`.
    rt_gateway: ReadMostly<Option<MArray<u8>>>,
    /// \[N\] `rt_ifa`: the interface address to use.
    pub rt_ifa: Link<Ifaddr>,
    /// \[L\] `rt_llinfo`: the link-level information (ARP's or ND's).
    pub rt_llinfo: PtrWord<core::ffi::c_void>,
    /// \[X\] `rt_gwroute`: the route of `rt_gateway`.
    rt_gwroute: ReadMostly<Option<PoolArc<Rtentry>>>,
    /// \[N\] `rt_parent`: the cloning route of a cloned one.
    rt_parent: ReadMostly<Option<PoolArc<Rtentry>>>,
    /// \[T\] `rt_timer`: the route's timers.
    rt_timer: PinList<RttLink>,
    /// The timers on `rt_timer`, each holding a handle (\[T\]; read by [`rt_refcnt`]).
    rt_ntimers: AtomicU32,
    /// The routes unlinked from this route's chain whose `rt_next` still names this one (a
    /// reference each, kept for their readers; read by [`rt_refcnt`]).
    pub(crate) rt_backrefs: AtomicU32,
    /// Whether this route left its chain keeping its `rt_next` (counted in the successor's
    /// `rt_backrefs`).
    pub(crate) rt_next_kept: AtomicBool,
    /// `rt_mtx` and the member it locks, \[r\] `rt_cachecnt`.
    rt_mtx: Mutexed<u32>,
    /// `rt_rmx`: the metrics.
    pub rt_rmx: RtKmetrics,
    /// \[N\] `rt_ifidx`: the interface to use.
    pub rt_ifidx: AtomicCell<u32>,
    /// \[X\] `rt_flags`.
    pub rt_flags: RtFlags,
    /// \[N\] `rt_labelid`: the route label id.
    pub rt_labelid: AtomicCell<u16>,
    /// \[N\] `rt_priority`: the routing priority.
    pub rt_priority: AtomicCell<u8>,
    /// Whether `rtrequest_delete` counted the route in `rttrash`.
    rt_trashed: AtomicBool,
}

hal::pool_home!(Rtentry, home);

impl SmrCounted for Rtentry {
    fn smr_link(&self) -> &SmrArcLink {
        &self.link
    }
}

impl Rtentry {
    /// `pool_get(&rtentry_pool, PR_NOWAIT | PR_ZERO)` and the members `rtrequest` sets before
    /// the route is linked: its destination `dest` (taken), its prefix length, flags and
    /// priority. `None` when the pool is empty or not made.
    fn alloc(dest: MArray<u8>, plen: u32, flags: u32, prio: u8) -> Option<PoolArc<Rtentry>> {
        RTENTRY_POOL
            .get_homed(PR_NOWAIT, || Rtentry {
                home: PoolHome::new(),
                link: SmrArcLink::new(),
                rt_dest: dest,
                rt_plen: plen,
                rt_next: SmrArcWord::new(),
                rt_gateway: ReadMostly::new(None),
                rt_ifa: Link::new(),
                rt_llinfo: PtrWord::new(),
                rt_gwroute: ReadMostly::new(None),
                rt_parent: ReadMostly::new(None),
                rt_timer: PinList::new(),
                rt_ntimers: AtomicU32::new(0),
                rt_backrefs: AtomicU32::new(0),
                rt_next_kept: AtomicBool::new(false),
                rt_mtx: Mutexed::new(IPL_SOFTNET, 0),
                rt_rmx: RtKmetrics::new(),
                rt_ifidx: AtomicCell::new(),
                rt_flags: RtFlags::new(flags),
                rt_labelid: AtomicCell::new(),
                rt_priority: AtomicCell::new(),
                rt_trashed: AtomicBool::new(false),
            })
            .inspect(|rt| rt.rt_priority.set(prio))
    }

    /// `rt_key(rt)`: the destination's bytes.
    #[inline]
    pub fn dest(&self) -> &[u8] {
        &self.rt_dest
    }

    /// `rt_key(rt)` as a socket address.
    pub fn dest_view(&self) -> SockaddrView<'_> {
        match SockaddrView::new(&self.rt_dest) {
            Some(v) => v,
            None => kpanic(format_args!("rtentry: a destination too short")),
        }
    }

    /// `rt_plen(rt)`.
    #[inline]
    pub fn rt_plen(&self) -> u32 {
        self.rt_plen
    }

    /// `rt->rt_ifa`, which every route `rtrequest` made has.
    pub fn ifa(&self) -> Option<&'static Ifaddr> {
        self.rt_ifa.get()
    }

    /// `f` of the gateway address's bytes (its `sa_len` of them), read under the cell.
    pub fn with_gateway<R>(&self, f: impl FnOnce(Option<&[u8]>) -> R) -> R {
        let g = self.rt_gateway.read();
        f(g.as_deref().map(|b| {
            let n = usize::from(b.first().copied().unwrap_or(0)).min(b.len());
            &b[..n.max(2).min(b.len())]
        }))
    }

    /// A copy of the gateway address (`None` without one or when longer than
    /// [`SA_COPY_MAX`]).
    pub fn gateway_copy(&self) -> Option<SaCopy> {
        self.with_gateway(|g| g.and_then(SockaddrView::new).and_then(SaCopy::of))
    }

    /// Whether the gateway is the socket address `gw` (`rt_gateway->sa_len == gw->sa_len &&
    /// memcmp(rt_gateway, gw, gw->sa_len) == 0`).
    pub fn gateway_is(&self, gw: &[u8]) -> bool {
        let n = usize::from(gw.first().copied().unwrap_or(0));
        self.with_gateway(|g| {
            g.is_some_and(|g| g.first() == gw.first() && g.get(..n) == gw.get(..n))
        })
    }

    /// `rt_use`: packets sent using this route.
    pub fn rt_use(&self) -> &AtomicU64 {
        &self.rt_rmx.rmx_pksent
    }

    /// `rt_expire`.
    pub fn rt_expire(&self) -> &AtomicI64 {
        &self.rt_rmx.rmx_expire
    }

    /// `rt_locks`.
    pub fn rt_locks(&self) -> &AtomicCell<u32> {
        &self.rt_rmx.rmx_locks
    }

    /// `rt_mtu`.
    pub fn rt_mtu(&self) -> &AtomicU32 {
        &self.rt_rmx.rmx_mtu
    }

    /// `rt_cachecnt`, read under `rt_mtx`.
    pub fn rt_cachecnt(&self) -> u32 {
        *self.rt_mtx.lock()
    }

    /// `rt->rt_parent`, a counted handle.
    pub fn parent(&self) -> Option<PoolArc<Rtentry>> {
        self.rt_parent.read().clone()
    }

    /// `rt->rt_gwroute`, a counted handle.
    pub fn gwroute(&self) -> Option<PoolArc<Rtentry>> {
        self.rt_gwroute.read().clone()
    }
}

impl Drop for Rtentry {
    /// The last `rtfree(rt)`: the route out of `rttrash`, its timers (none: each holds a
    /// handle) and its interface address and label references given back; its addresses are
    /// freed with their arrays.
    fn drop(&mut self) {
        kassert!(self.rt_flags.get() & RTF_UP == 0);
        if self.rt_trashed.load(Ordering::Relaxed) {
            RTTRASH.fetch_sub(1, Ordering::Relaxed);
        }
        if let Some(ifa) = self.rt_ifa.get() {
            ifafree(ifa);
        }
        rtlabel_unref(self.rt_labelid.get());
        // MPLS: rt_mpls_clear(rt), not configured.
        // A next hop still cached (a failed request's) is given back as `rt_putgwroute`
        // does.
        if let Some(o) = self.rt_gwroute.get_mut().take() {
            gwroute_unref(&o);
        }
        // The successor this route kept when it left its chain is no longer named by it.
        if self.rt_next_kept.load(Ordering::Relaxed) {
            let g = hal::smr::read();
            if let Some(n) = self.rt_next.load(&g) {
                n.rt_backrefs.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

/// `struct route`: a protocol's cache of the route to a destination, owned by its holder (a
/// softnet's `Netstack`, a protocol control block, a frame), its route handle the C's
/// `ro_rt` reference.
pub struct Route {
    /// `ro_rt`.
    pub ro_rt: Option<PoolArc<Rtentry>>,
    /// `ro_generation`: `rtgeneration` when the route was cached.
    pub ro_generation: u64,
    /// `ro_tableid`.
    pub ro_tableid: u32,
    /// `ro_dstsa`/`ro_dstsin`/`ro_dstsin6`: the destination, a `struct sockaddr_in6`'s bytes.
    pub ro_dst: [u8; SIN6_LEN],
    /// `ro_srcin`/`ro_srcin6`: the source address.
    pub ro_src: [u8; 16],
}

impl Route {
    /// An empty cache (all zero).
    pub const fn new() -> Self {
        Self {
            ro_rt: None,
            ro_generation: 0,
            ro_tableid: 0,
            ro_dst: [0; SIN6_LEN],
            ro_src: [0; 16],
        }
    }

    /// The cached route.
    #[inline]
    pub fn rt(&self) -> Option<&Rtentry> {
        self.ro_rt.as_deref()
    }

    /// The cached route's handle taken out.
    pub fn take_rt(&mut self) -> Option<PoolArc<Rtentry>> {
        self.ro_rt.take()
    }

    /// `ro_dstsa`: the destination as a socket address.
    pub fn dst(&self) -> SockaddrView<'_> {
        let n = usize::from(self.ro_dst[0]).clamp(2, SIN6_LEN);
        match SockaddrView::new(&self.ro_dst[..n]) {
            Some(v) => v,
            None => kpanic(format_args!("route: a destination too short")),
        }
    }

    /// `memset(ro, 0, sizeof(*ro))` then the generation and table of a new entry.
    fn reset(&mut self, generation: u64, rtableid: u32) {
        *self = Route::new();
        self.ro_generation = generation;
        self.ro_tableid = rtableid;
    }

    /// `route_cache(ro, dst, src, rtableid)`: `Ok` when the cached route still serves `dst`
    /// (from `src`) in table `rtableid` (`ipstat`'s cache hit); else (`Err(ESRCH)`, a miss)
    /// the cache reset to `dst`/`src` with no route.
    pub fn cache4(
        &mut self,
        dst: [u8; 4],
        src: Option<[u8; 4]>,
        rtableid: u32,
    ) -> Result<(), Errno> {
        let generation = RTGENERATION.load(Ordering::Acquire);

        if rtisvalid(self.rt())
            && self.ro_generation == generation
            && self.ro_tableid == rtableid
            && self.ro_dst[1] == AF_INET
            && self.ro_dst[SIN_ADDR..SIN_ADDR + 4] == dst
        {
            let srcin = &self.ro_src[..4];
            if src.is_none()
                || !ip_multipath()
                || !self.rt().is_some_and(|rt| rt.rt_flags.isset(RTF_MPATH))
                || (srcin != [0; 4] && src.is_some_and(|s| srcin == s))
            {
                inet_rtcache(true);
                return Ok(());
            }
        }

        inet_rtcache(false);
        self.reset(generation, rtableid);

        self.ro_dst[1] = AF_INET;
        self.ro_dst[0] = SIN_LEN as u8;
        self.ro_dst[SIN_ADDR..SIN_ADDR + 4].copy_from_slice(&dst);
        if let Some(s) = src {
            self.ro_src[..4].copy_from_slice(&s);
        }

        Err(Errno::ESRCH)
    }

    /// `route_mpath(ro, dst, src, rtableid)`: the cached route, or a new one (`rtalloc_mpath`,
    /// the source picking a multipath route) cached on a miss.
    pub fn mpath4(
        &mut self,
        dst: [u8; 4],
        src: Option<[u8; 4]>,
        rtableid: u32,
        net: &NetHeld<'_>,
    ) -> Option<&Rtentry> {
        if self.cache4(dst, src, rtableid).is_err() {
            let s = [u32::from_ne_bytes([
                self.ro_src[0],
                self.ro_src[1],
                self.ro_src[2],
                self.ro_src[3],
            ])];
            let src = (self.ro_src[..4] != [0; 4]).then_some(&s[..]);
            self.ro_rt = rtalloc_mpath(self.dst(), src, self.ro_tableid, net);
        }
        self.rt()
    }

    /// `route6_cache(ro, dst, src, rtableid)`: [`Route::cache4`] for IPv6.
    pub fn cache6(
        &mut self,
        dst: &[u8; 16],
        src: Option<&[u8; 16]>,
        rtableid: u32,
    ) -> Result<(), Errno> {
        let generation = RTGENERATION.load(Ordering::Acquire);

        if rtisvalid(self.rt())
            && self.ro_generation == generation
            && self.ro_tableid == rtableid
            && self.ro_dst[1] == AF_INET6
            && self.ro_dst[SIN6_ADDR..SIN6_ADDR + 16] == dst[..]
            && (src.is_none()
                || !ip6_multipath()
                || !self.rt().is_some_and(|rt| rt.rt_flags.isset(RTF_MPATH))
                || (self.ro_src != [0; 16] && src.is_some_and(|s| self.ro_src == *s)))
        {
            inet6_rtcache(true);
            return Ok(());
        }

        inet6_rtcache(false);
        self.reset(generation, rtableid);

        self.ro_dst[1] = AF_INET6;
        self.ro_dst[0] = SIN6_LEN as u8;
        self.ro_dst[SIN6_ADDR..SIN6_ADDR + 16].copy_from_slice(dst);
        if let Some(s) = src {
            self.ro_src = *s;
        }

        Err(Errno::ESRCH)
    }

    /// `route6_mpath(ro, dst, src, rtableid)`: [`Route::mpath4`] for IPv6.
    pub fn mpath6(
        &mut self,
        dst: &[u8; 16],
        src: Option<&[u8; 16]>,
        rtableid: u32,
        net: &NetHeld<'_>,
    ) -> Option<&Rtentry> {
        if self.cache6(dst, src, rtableid).is_err() {
            let s: [u32; 4] = core::array::from_fn(|i| {
                u32::from_ne_bytes([
                    self.ro_src[4 * i],
                    self.ro_src[4 * i + 1],
                    self.ro_src[4 * i + 2],
                    self.ro_src[4 * i + 3],
                ])
            });
            let src = (self.ro_src != [0; 16]).then_some(&s[..]);
            self.ro_rt = rtalloc_mpath(self.dst(), src, self.ro_tableid, net);
        }
        self.rt()
    }
}

impl Default for Route {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct rt_addrinfo`: the addresses and flags of a route request.
#[derive(Clone, Copy)]
pub struct RtAddrinfo<'a> {
    /// `rti_addrs`: the `RTA_*` bits of the addresses present.
    pub rti_addrs: i32,
    /// `rti_info`: the addresses, by `RTAX_*` index.
    pub rti_info: [Option<SockaddrView<'a>>; RTAX_MAX],
    /// `rti_flags`: the `RTF_*` bits.
    pub rti_flags: u32,
    /// `rti_ifa`: the interface address.
    pub rti_ifa: Option<&'static Ifaddr>,
    /// `rti_mpls`.
    pub rti_mpls: u8,
}

impl RtAddrinfo<'_> {
    /// An empty request (`memset(&info, 0, sizeof(info))`).
    pub const fn new() -> Self {
        Self {
            rti_addrs: 0,
            rti_info: [None; RTAX_MAX],
            rti_flags: 0,
            rti_ifa: None,
            rti_mpls: 0,
        }
    }
}

impl Default for RtAddrinfo<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// The function a timer queue calls for an expired timer's route (`rtq_func`).
pub type RtqFunc = fn(&PoolArc<Rtentry>, u32);

/// `struct rttimer_queue`: a queue of route timers of one timeout, embedded by its owner (a
/// protocol's `static`).
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct rttimer_queue's rtq_* names so the C can be grepped"
)]
pub struct RttimerQueue {
    /// \[T\] `rtq_head`: the timers, oldest first.
    rtq_head: PinList<RttNext>,
    /// \[I\] `rtq_func`: the callback (none: delete a dynamic host route).
    rtq_func: OnceStatic<Option<RtqFunc>>,
    /// \[T\] `rtq_count`.
    rtq_count: AtomicU64,
    /// \[T\] `rtq_timeout`: seconds.
    rtq_timeout: AtomicI32,
}

impl RttimerQueue {
    /// A queue before `rt_timer_queue_init`.
    pub const fn new() -> Self {
        Self {
            rtq_head: PinList::new(),
            rtq_func: OnceStatic::new(),
            rtq_count: AtomicU64::new(0),
            rtq_timeout: AtomicI32::new(0),
        }
    }
}

impl Default for RttimerQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct rttimer`: a route timer, an item of the recorded pool `"rttmr"`.
pub struct Rttimer {
    /// Where the pool recorded it.
    home: PoolHome<Rttimer>,
    /// \[T\] `rtt_next`: on its queue.
    rtt_next: PinEntry<Rttimer>,
    /// \[T\] `rtt_link`: on its route's `rt_timer`.
    rtt_link: PinEntry<Rttimer>,
    /// \[I\] `rtt_timeout`: its argument a handle of the timer (`timeout_set_proc`).
    rtt_timeout: Timeout,
    /// \[I\] `rtt_queue`.
    rtt_queue: &'static RttimerQueue,
    /// \[T\] `rtt_rt`: its route, cleared when the timer leaves the route.
    rtt_rt: Guarded<Option<PoolArc<Rtentry>>>,
    /// \[I\] `rtt_expire`: the route's expire time.
    rtt_expire: i64,
    /// \[I\] `rtt_tableid`: the route's table.
    rtt_tableid: u32,
}

hal::pool_home!(Rttimer, home);

hal::pin_adapter!(
    /// A queue's timers, through `Rttimer::rtt_next`.
    pub RttNext: Rttimer, rtt_next
);

hal::pin_adapter!(
    /// A route's timers, through `Rttimer::rtt_link`.
    pub RttLink: Rttimer, rtt_link
);

/// One route label (`struct rt_label`), a box of `M_RTABLE` on [`RT_LABELS`]' chain.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct rt_label's rtl_* names so the C can be grepped"
)]
struct RtLabel {
    /// \[L\] `rtl_entry`: the next label, in id order.
    rtl_next: Option<MBox<RtLabel>>,
    /// \[I\] `rtl_name`: NUL-terminated.
    rtl_name: [u8; RTLABEL_LEN],
    /// \[I\] `rtl_id`.
    rtl_id: u16,
    /// \[L\] `rtl_ref`.
    rtl_ref: i32,
}

impl RtLabel {
    /// The name up to its NUL.
    fn name(&self) -> &[u8] {
        cstr(&self.rtl_name)
    }
}

/// `rtentry_pool`.
static RTENTRY_POOL: TypedPool<Rtentry> = TypedPool::new();

/// `rttimer_pool`.
static RTTIMER_POOL: TypedPool<Rttimer> = TypedPool::new();

/// `rtcounters`: the routing statistics.
pub static RTCOUNTERS: Counters<RTS_NCOUNTERS> = Counters::new();

/// `rttrash`: routes not in a table but not freed.
pub static RTTRASH: AtomicI32 = AtomicI32::new(0);

/// `rtgeneration`: bumped when routes change.
pub static RTGENERATION: AtomicU64 = AtomicU64::new(0);

/// `rt_hashjitter`: the hash's jitter, to avoid synchronization between routers.
static RT_HASHJITTER: AtomicU32 = AtomicU32::new(0);

/// `rttimer_mtx`: the timers' lists.
static RTTIMER_MTX: Mutexed<()> = Mutexed::new(IPL_MPFLOOR, ());

/// `rt_labels` under `rtlabel_mtx`: the labels in id order.
static RT_LABELS: Mutexed<Option<MBox<RtLabel>>> = Mutexed::new(IPL_NET, None);

/// The `int` error of a request, as the routing messages carry it (0 for success).
fn errno_of(r: Result<(), Errno>) -> i32 {
    r.err().map_or(0, |e| e as i32)
}

/// `atomic_load_int(&ipmultipath)`, the internet protocols' (the `Inet` leg; off before
/// registration).
fn ip_multipath() -> bool {
    match if_::inet() {
        Some(i) => i.ip_multipath(),
        None => false,
    }
}

/// `atomic_load_int(&ip6_multipath)`.
fn ip6_multipath() -> bool {
    match if_::inet() {
        Some(i) => i.ip6_multipath(),
        None => false,
    }
}

/// `ipstat_inc(ips_rtcachehit/miss)`, the internet protocols' (the `Inet` leg).
fn inet_rtcache(hit: bool) {
    if let Some(i) = if_::inet() {
        i.ipstat_rtcache(hit);
    }
}

/// `ip6stat_inc(ip6s_rtcachehit/miss)`.
fn inet6_rtcache(hit: bool) {
    if let Some(i) = if_::inet() {
        i.ip6stat_rtcache(hit);
    }
}

/// `rtstat_inc(c)`.
#[inline]
pub fn rtstat_inc(c: usize) {
    RTCOUNTERS.inc(c);
}

/// The bytes before the first NUL.
fn cstr(b: &[u8]) -> &[u8] {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    &b[..n]
}

/// `strlcpy(dst, src, sizeof(dst))` of a name's bytes (up to a NUL).
fn strlcpy(dst: &mut [u8], src: &[u8]) {
    let src = cstr(src);
    let n = src.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&src[..n]);
    for b in &mut dst[n..] {
        *b = 0;
    }
}

/// `route_init()`: the counters, the `"rtentry"` pool and the hash's jitter. A second call
/// (a test's) leaves the pool as it is.
pub fn route_init() {
    let _ = RTENTRY_POOL.init_recorded(IPL_MPFLOOR, 0, "rtentry");

    while RT_HASHJITTER.load(Ordering::Relaxed) == 0 {
        RT_HASHJITTER.store(hal::random::arc4random(), Ordering::Relaxed);
    }

    // BFD: bfdinit(), not configured.
}

/// `rtisvalid(rt)`: whether a (cached) route is still usable: up, and for a gateway route
/// with its next hop up.
pub fn rtisvalid(rt: Option<&Rtentry>) -> bool {
    let Some(rt) = rt else {
        return false;
    };

    if !rt.rt_flags.isset(RTF_UP) {
        return false;
    }

    if rt.rt_flags.isset(RTF_GATEWAY) {
        let gw = rt.rt_gwroute.read();
        let Some(gw) = gw.as_deref() else {
            return false;
        };
        kassert!(!gw.rt_flags.isset(RTF_GATEWAY));
        if !gw.rt_flags.isset(RTF_UP) {
            return false;
        }
    }

    true
}

/// `rt_match(dst, src, flags, tableid)`: the best route to `dst` (a multipath one picked by
/// `src`), its cloned link-layer route made when it is a cloning route and `RT_RESOLVE` is
/// asked; `rts_unreach` counted for none. `rtalloc`'s body, not for direct use.
fn rt_match(
    dst: SockaddrView<'_>,
    src: Option<&[u32]>,
    flags: i32,
    tableid: u32,
    net: &NetHeld<'_>,
) -> Option<PoolArc<Rtentry>> {
    let Some(mut rt) = rtable_match(tableid, dst, src) else {
        rtstat_inc(RTS_UNREACH);
        return None;
    };

    if rt.rt_flags.isset(RTF_CLONING) && flags & RT_RESOLVE != 0 {
        let _ = rt_clone(&mut rt, dst, tableid, net);
    }

    rt.rt_use().fetch_add(1, Ordering::Relaxed);
    Some(rt)
}

/// `rt_clone(rtp, dst, rtableid)`: the host route of `dst` cloned from the cloning route
/// `*rtp`, which it replaces (listeners told with `RTM_ADD`); `RTM_MISS` sent on failure.
fn rt_clone(
    rtp: &mut PoolArc<Rtentry>,
    dst: SockaddrView<'_>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut info = RtAddrinfo::new();
    info.rti_info[RTAX_DST] = Some(dst);

    // The priority of cloned route should be different to avoid conflict with /32 cloning
    // routes. It should also be higher to let the ARP layer find cloned routes instead of
    // the cloning one.
    let prio = rtp.rt_priority.get().wrapping_sub(1);
    let mut ret = Some(PoolArc::clone(rtp));
    klock::kernel_lock();
    let error = rtrequest(RTM_RESOLVE, &mut info, prio, Some(&mut ret), rtableid, net);
    klock::kernel_unlock();
    match (error, ret) {
        (Ok(()), Some(rt)) => {
            // Inform listeners of the new route.
            rtsock::rtm_send(&rt, RTM_ADD, 0, rtableid, net);
            *rtp = rt;
            Ok(())
        }
        (e, _) => {
            // The request's gateway and label as RTM_RESOLVE set them (the module's
            // deviations).
            let sdl = empty_sdl();
            let mut sa_rl = SockaddrRtlabel::default();
            info.rti_info[RTAX_GATEWAY] = SockaddrView::new(&sdl);
            info.rti_info[RTAX_LABEL] = rtlabel_id2sa(rtp.rt_labelid.get(), &mut sa_rl);
            let e = e.and(Err(Errno::EINVAL));
            rtsock::rtm_miss(RTM_MISS, &info, 0, RTP_NONE, 0, errno_of(e), rtableid);
            e
        }
    }
}

/// `{ sizeof(sa_dl), AF_LINK }`: an empty link-level address's bytes.
fn empty_sdl() -> [u8; SDL_LEN] {
    let mut b = [0u8; SDL_LEN];
    b[0] = SDL_LEN as u8;
    b[1] = AF_LINK;
    b
}

/// `mix(a, b, c)`, originated from `bridge_hash()` in if_bridge.c.
#[inline]
fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 13);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 8);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 13);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 12);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 16);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 5);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 3);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 10);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 15);
}

/// The `i`th 32-bit word of `b` from `off`, as the C reads `s_addr` or `s6_addr32[i]` (in
/// memory order); 0 past the end.
#[inline]
fn word_at(b: &[u8], off: usize, i: usize) -> u32 {
    let at = off + 4 * i;
    b.get(at..at + 4)
        .map_or(0, |w| u32::from_ne_bytes([w[0], w[1], w[2], w[3]]))
}

/// `rt_hash(rt, dst, src)`: the hash of `dst` and the source `src` that picks a multipath
/// route, `None` (the C's -1) without a source, for an invalid or non-multipath route, or
/// with multipath off for the family.
pub fn rt_hash(rt: &Rtentry, dst: SockaddrView<'_>, src: Option<&[u32]>) -> Option<u32> {
    let src = src?;
    if !rtisvalid(Some(rt)) || !rt.rt_flags.isset(RTF_MPATH) {
        return None;
    }

    let mut a = 0x9e37_79b9u32;
    let mut b = a;
    let mut c = RT_HASHJITTER.load(Ordering::Relaxed);
    let d = dst.bytes();
    let s = |i: usize| src.get(i).copied().unwrap_or(0);

    match dst.family() {
        AF_INET => {
            if !ip_multipath() {
                return None;
            }
            a = a.wrapping_add(word_at(d, SIN_ADDR, 0));
            b = b.wrapping_add(s(0));
            mix(&mut a, &mut b, &mut c);
        }
        AF_INET6 => {
            if !ip6_multipath() {
                return None;
            }
            let w = |i| word_at(d, SIN6_ADDR, i);
            for (x, y, i) in [(0, 2, 0), (1, 3, 1), (2, 1, 2), (3, 0, 3)] {
                a = a.wrapping_add(w(x));
                b = b.wrapping_add(w(y));
                c = c.wrapping_add(s(i));
                mix(&mut a, &mut b, &mut c);
            }
        }
        _ => {}
    }

    Some(c & 0xffff)
}

/// `rtalloc_mpath(dst, src, rtableid)`: a route to `dst`, a multipath one picked by `src`,
/// its link-layer route resolved.
pub fn rtalloc_mpath(
    dst: SockaddrView<'_>,
    src: Option<&[u32]>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Option<PoolArc<Rtentry>> {
    rt_match(dst, src, RT_RESOLVE, rtableid, net)
}

/// `rtalloc(dst, flags, rtableid)`: the best route to `dst` in table `rtableid`
/// (`RT_RESOLVE`: its link-layer route made and resolved).
pub fn rtalloc(
    dst: SockaddrView<'_>,
    flags: i32,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Option<PoolArc<Rtentry>> {
    rt_match(dst, None, flags, rtableid, net)
}

/// `rt_setgwroute(rt, gate, rtableid)`: caches in the gateway route `rt` the route of its
/// reachable next hop `gate`: `ENOENT` without one, `EHOSTUNREACH` for one on another
/// interface that is no cloned link-layer route of a cloning route there, `ENETUNREACH` for
/// a cloning or gateway next hop. Under the net lock.
fn rt_setgwroute(
    rt: &Rtentry,
    gate: SockaddrView<'_>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let rdomain = rtable_l2(rtableid);

    // If we cannot find a valid next hop bail.
    let mut nhrt = rt_match(gate, None, RT_RESOLVE, rdomain, net).ok_or(Errno::ENOENT)?;

    // Next hop entry must be on the same interface.
    if nhrt.rt_ifidx.get() != rt.rt_ifidx.get() {
        if !nhrt.rt_flags.isset(RTF_LLINFO) || !nhrt.rt_flags.isset(RTF_CLONED) {
            return Err(Errno::EHOSTUNREACH);
        }

        // We found a L2 entry, so we might have multiple RTF_CLONING routes for the same
        // subnet. Query the first route of the multipath chain and iterate until we find the
        // correct one.
        let parent = nhrt.parent().ok_or(Errno::EHOSTUNREACH)?;
        let mut sa_mask = SaCopy::zeroed();
        let mask = rt_plen2mask(&parent, &mut sa_mask);
        let mut prt = rtable_lookup(
            rdomain,
            parent.dest_view(),
            mask,
            None,
            hal::sys::net_route::RTP_ANY,
        );
        drop(nhrt);

        while let Some(p) = prt.take() {
            if p.rt_ifidx.get() == rt.rt_ifidx.get() {
                prt = Some(p);
                break;
            }
            prt = rtable_iterate(p);
        }

        // We found nothing or a non-cloning MPATH route.
        let mut p = match prt {
            Some(p) if p.rt_flags.isset(RTF_CLONING) => p,
            _ => return Err(Errno::EHOSTUNREACH),
        };

        rt_clone(&mut p, gate, rdomain, net)?;
        nhrt = p;
    }

    // Next hop must be reachable, this also prevents rtentry loops for example when
    // rt->rt_gwroute points to rt.
    if nhrt.rt_flags.isset(RTF_CLONING | RTF_GATEWAY) {
        return Err(Errno::ENETUNREACH);
    }

    // If the MTU of next hop is 0, this will reset the MTU of the route to run PMTUD again
    // from scratch.
    if rt.rt_locks().get() & RTV_MTU == 0 {
        let mtu = rt.rt_mtu().load(Ordering::Relaxed);
        let nhmtu = nhrt.rt_mtu().load(Ordering::Relaxed);
        if mtu > nhmtu {
            let _ = rt
                .rt_mtu()
                .compare_exchange(mtu, nhmtu, Ordering::Relaxed, Ordering::Relaxed);
        }
    }

    // Commit.
    rt_putgwroute(rt, Some(nhrt), net);

    Ok(())
}

/// A next hop's cache reference given back: `rt_cachecnt` decremented under `rt_mtx`, and
/// `RTF_CACHED` cleared with the last.
fn gwroute_unref(onhrt: &Rtentry) {
    let mut c = onhrt.rt_mtx.lock();
    kassert!(*c > 0);
    kassert!(onhrt.rt_flags.isset(RTF_CACHED));
    *c = c.saturating_sub(1);
    if *c == 0 {
        onhrt.rt_flags.clear(RTF_CACHED);
    }
}

/// `rt_putgwroute(rt, nhrt)`: `nhrt` (or none) as the cached next hop of the gateway route
/// `rt`, the old one given back. To avoid reference counting problems when writing
/// link-layer addresses in an outgoing packet, the lifetime of a cached entry is greater
/// than the bigger lifetime of the gateway entries it is pointed by (`rt_cachecnt`,
/// `RTF_CACHED`). Under the net lock.
pub fn rt_putgwroute(rt: &Rtentry, nhrt: Option<PoolArc<Rtentry>>, _net: &NetHeld<'_>) {
    if !rt.rt_flags.isset(RTF_GATEWAY) {
        return;
    }

    if let Some(n) = &nhrt {
        let mut c = n.rt_mtx.lock();
        n.rt_flags.set_bits(RTF_CACHED);
        *c += 1;
    }

    let onhrt = rt.rt_gwroute.replace(nhrt);

    if let Some(o) = onhrt {
        gwroute_unref(&o);
        // rtfree(onhrt): the handle's drop.
    }
}

/// `ifaref(ifa)`: one more reference of the interface address (`rt_ifa`'s).
pub fn ifaref(ifa: &'static Ifaddr) -> &'static Ifaddr {
    ifa.ifa_refcnt.take();
    ifa
}

/// `ifafree(ifa)`: one reference of the interface address given back; with the last, the
/// address is freed (`free(ifa, M_IFADDR, 0)`, its protocol's storage: the `Inet` leg).
pub fn ifafree(ifa: &'static Ifaddr) {
    if !ifa.ifa_refcnt.rele() {
        return;
    }
    if let Some(i) = if_::inet() {
        i.ifa_free(ifa);
    }
}

/// `in_broadcast(in, rtableid)` (netinet/in.c, read here over the interface list until
/// N6.4 moves it): whether `addr` is the broadcast address of an IPv4 address of a
/// broadcast interface of `rtableid`'s domain, and not that address itself.
fn in_broadcast(addr: [u8; 4], rtableid: u32, net: &NetHeld<'_>) -> bool {
    let rdomain = rtable_l2(rtableid);
    for ifn in &if_::IFNETLIST {
        if ifn.if_rdomain.get() != rdomain || ifn.if_flags.get() & IFF_BROADCAST == 0 {
            continue;
        }
        for ifa in &ifn.if_addrlist {
            let Some(a) = ifa.ifa_addr.read(net) else {
                continue;
            };
            if a.family() != AF_INET {
                continue;
            }
            let own = a.get(SIN_ADDR..SIN_ADDR + 4) == Some(&addr[..]);
            let brd = ifa
                .ifa_broadaddr()
                .read(net)
                .is_some_and(|b| b.get(SIN_ADDR..SIN_ADDR + 4) == Some(&addr[..]));
            if !own && brd {
                return true;
            }
        }
    }
    false
}

/// `rtredirect(dst, gateway, src, rtp, rdomain)`: a routing redirect from `src` for `dst`
/// through `gateway` (from the network layer): a new dynamic host route, or the gateway of
/// the host route changed; the route made or changed given to `rtp` when asked for; the
/// counters and an `RTM_REDIRECT` message either way. Under the net lock.
pub fn rtredirect(
    dst: SockaddrView<'_>,
    gateway: SockaddrView<'_>,
    src: SockaddrView<'_>,
    rtp: Option<&mut Option<PoolArc<Rtentry>>>,
    rdomain: u32,
    net: &NetHeld<'_>,
) {
    let mut error: Result<(), Errno> = Ok(());
    let mut stat = RTS_NCOUNTERS;
    let mut ifidx = 0;
    let mut flags = RTF_GATEWAY | RTF_HOST;
    let mut prio = RTP_NONE;

    'out: {
        // The new gateway must be on a directly connected network.
        let ifa = match rtalloc(gateway, 0, rdomain, net) {
            Some(rt) if rtisvalid(Some(&rt)) && !rt.rt_flags.isset(RTF_GATEWAY) => {
                ifidx = rt.rt_ifidx.get();
                rt.rt_ifa.get()
            }
            _ => {
                error = Err(Errno::ENETUNREACH);
                break 'out;
            }
        };

        let mut rt = rtable_lookup(rdomain, dst, None, None, hal::sys::net_route::RTP_ANY);
        // A redirect is believed only from the router the route to `dst` uses now (anything
        // else is stale or forged), and never toward one of our own addresses or a broadcast
        // one, which would make a loop (an interface that just went down, say).
        if let Some(r) = &rt
            && (!r.gateway_is(src.bytes())
                || r.rt_ifa.get().map(core::ptr::from_ref) != ifa.map(core::ptr::from_ref))
        {
            error = Err(Errno::EINVAL);
        } else if if_::ifa_ifwithaddr(net, gateway.bytes(), rdomain).is_some()
            || (gateway.family() == AF_INET
                && gateway
                    .bytes()
                    .get(SIN_ADDR..SIN_ADDR + 4)
                    .is_some_and(|a| in_broadcast([a[0], a[1], a[2], a[3]], rdomain, net)))
        {
            error = Err(Errno::EHOSTUNREACH);
        }

        if error.is_ok() {
            // No route, or a gateway route to a network: a new dynamic host route (hosts that
            // learn their routes from the gateways' redirects build their tables this way). A
            // route straight to an interface is never redirected.
            let create = match &rt {
                None => true,
                // A network route stays as it is; the host gets a route of its own.
                Some(r) if r.rt_flags.isset(RTF_GATEWAY) => !r.rt_flags.isset(RTF_HOST),
                Some(_) => {
                    error = Err(Errno::EHOSTUNREACH);
                    false
                }
            };
            if error.is_ok() {
                if create {
                    drop(rt.take());
                    flags |= RTF_DYNAMIC;
                    let mut info = RtAddrinfo::new();
                    info.rti_info[RTAX_DST] = Some(dst);
                    info.rti_info[RTAX_GATEWAY] = Some(gateway);
                    info.rti_ifa = ifa;
                    info.rti_flags = flags;
                    error = rtrequest(RTM_ADD, &mut info, RTP_DEFAULT, Some(&mut rt), rdomain, net);
                    if error.is_ok()
                        && let Some(r) = &rt
                    {
                        flags = r.rt_flags.get();
                        prio = r.rt_priority.get();
                    }
                    stat = RTS_DYNAMIC;
                } else if let Some(r) = &rt {
                    // A host route through a gateway: its gateway replaced (the C does not
                    // check the mask either).
                    r.rt_flags.set_bits(RTF_MODIFIED);
                    flags |= RTF_MODIFIED;
                    prio = r.rt_priority.get();
                    stat = RTS_NEWGATEWAY;
                    let _ = rt_setgate(r, gateway, rdomain, net);
                }
            }
        }

        // done:
        if let Some(r) = rt
            && error.is_ok()
            && let Some(p) = rtp
        {
            *p = Some(r);
        }
    }

    if error.is_err() {
        rtstat_inc(RTS_BADREDIRECT);
    } else if stat != RTS_NCOUNTERS {
        rtstat_inc(stat);
    }
    let mut info = RtAddrinfo::new();
    info.rti_info[RTAX_DST] = Some(dst);
    info.rti_info[RTAX_GATEWAY] = Some(gateway);
    info.rti_info[RTAX_AUTHOR] = Some(src);
    rtsock::rtm_miss(
        RTM_REDIRECT,
        &info,
        flags,
        prio,
        ifidx,
        errno_of(error),
        rdomain,
    );
}

/// The bytes of interface `ifp`'s link-level address (`sdltosa(ifp->if_sadl)`), copied.
pub(crate) fn sadl_copy(ifp: &Ifnet) -> Option<SaCopy> {
    let sadl = ifp.if_sadl.read();
    let sdl = sadl.as_deref()?;
    let bytes = hal::user::bytes_of(core::slice::from_ref(sdl));
    let n = usize::from(sdl.sdl_len).clamp(2, bytes.len());
    SockaddrView::new(&bytes[..n]).and_then(SaCopy::of)
}

/// `rtdeletemsg(rt, ifp, tableid)`: deletes route `rt` of interface `ifp` and tells the
/// listeners (`RTM_DELETE`, with what the route was).
pub fn rtdeletemsg(
    rt: &PoolArc<Rtentry>,
    ifp: &'static Ifnet,
    tableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    kassert!(rt.rt_ifidx.get() == ifp.if_index.get());

    // Request the new route so that the entry is not actually deleted. That will allow the
    // information being reported to be accurate (and consistent with route_output()).
    let gw = rt.gateway_copy();
    let mut sa_mask = SaCopy::zeroed();
    let mut sa_rl = SockaddrRtlabel::default();
    let ifpaddr = sadl_copy(ifp);
    let ifaaddr = rt.rt_ifa.get().and_then(|a| a.ifa_addr.read(net));
    let mut info = RtAddrinfo::new();
    info.rti_info[RTAX_DST] = Some(rt.dest_view());
    info.rti_info[RTAX_GATEWAY] = gw.as_ref().map(SaCopy::view);
    if !rt.rt_flags.isset(RTF_HOST) {
        info.rti_info[RTAX_NETMASK] = rt_plen2mask(rt, &mut sa_mask);
    }
    info.rti_info[RTAX_LABEL] = rtlabel_id2sa(rt.rt_labelid.get(), &mut sa_rl);
    info.rti_flags = rt.rt_flags.get();
    info.rti_info[RTAX_IFP] = ifpaddr.as_ref().map(SaCopy::view);
    info.rti_info[RTAX_IFA] = ifaaddr.as_ref().and_then(|a| SockaddrView::new(a));

    let mut ret = None;
    klock::kernel_lock();
    let error = rtrequest_delete(
        &mut info,
        rt.rt_priority.get(),
        ifp,
        Some(&mut ret),
        tableid,
        net,
    );
    klock::kernel_unlock();
    rtsock::rtm_miss(
        RTM_DELETE,
        &info,
        info.rti_flags,
        rt.rt_priority.get(),
        rt.rt_ifidx.get(),
        errno_of(error),
        tableid,
    );
    // rtfree(rt) of the request's answer: the handle's drop.
    drop(ret);
    error
}

/// `rtequal(a, b)`: the same route, or routes of the same key and prefix length.
fn rtequal(a: &Rtentry, b: &Rtentry) -> bool {
    if core::ptr::eq(a, b) {
        return true;
    }
    let n = usize::from(a.dest()[0]).min(a.dest().len());
    a.dest().get(..n) == b.dest().get(..n) && a.rt_plen() == b.rt_plen()
}

/// `rtflushclone1(rt, cloningrt, id)`: `EEXIST` (stop and delete) for a route cloned from
/// `cloningrt` whose interface still exists and that no gateway caches (unless it is
/// `cloningrt`'s own child).
fn rtflushclone1(rt: &Rtentry, cloningrt: &Rtentry, _id: u32) -> Result<(), Errno> {
    if !rt.rt_flags.isset(RTF_CLONED) {
        return Ok(());
    }

    let parent = rt.rt_parent.read();
    let is_child = parent
        .as_deref()
        .is_some_and(|p| core::ptr::eq(p, cloningrt));

    // Cached route must stay alive as long as their parent are alive.
    if rt.rt_flags.isset(RTF_CACHED) && !is_child {
        return Ok(());
    }

    if !parent.as_deref().is_some_and(|p| rtequal(p, cloningrt)) {
        return Ok(());
    }
    drop(parent);

    // This happens when an interface with a RTF_CLONING route is being detached. In this
    // case it's safe to bail because all the routes are being purged by rt_ifa_purge().
    let Some(ifp) = if_get(rt.rt_ifidx.get()) else {
        return Ok(());
    };

    if_put(Some(ifp));
    Err(Errno::EEXIST)
}

/// `rtflushclone(parent, rtableid)`: deletes every route cloned from the cloning route
/// `parent`.
fn rtflushclone(parent: &Rtentry, rtableid: u32, net: &NetHeld<'_>) -> Result<(), Errno> {
    kassert!(parent.rt_flags.isset(RTF_CLONING));

    loop {
        let mut rt = None;
        let mut error = rtable_walk(
            rtableid,
            parent.dest_view().family(),
            Some(&mut rt),
            |r, id| rtflushclone1(r, parent, id),
        );
        if let Some(r) = &rt
            && error == Err(Errno::EEXIST)
        {
            error = match if_get(r.rt_ifidx.get()) {
                None => Err(Errno::EAGAIN),
                Some(ifp) => {
                    let e = match rtdeletemsg(r, ifp, rtableid, net) {
                        Ok(()) => Err(Errno::EAGAIN),
                        e => e,
                    };
                    if_put(Some(ifp));
                    e
                }
            };
        }
        drop(rt);
        if error != Err(Errno::EAGAIN) {
            return error;
        }
    }
}

/// `rtrequest_delete(info, prio, ifp, ret_nrt, tableid)`: deletes the route of
/// `info`'s destination, mask and gateway at priority `prio` on interface `ifp`: its next
/// hop cache, its clones and its parent given up, `RTF_UP` cleared, its timers removed, the
/// interface told; the deleted route given to `ret_nrt` when asked for. `EAFNOSUPPORT`
/// without the table, `ESRCH` for no such route on `ifp`. Under the net lock.
pub fn rtrequest_delete(
    info: &mut RtAddrinfo<'_>,
    prio: u8,
    ifp: &'static Ifnet,
    ret_nrt: Option<&mut Option<PoolArc<Rtentry>>>,
    tableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    if !rtable_exists(tableid) {
        return Err(Errno::EAFNOSUPPORT);
    }
    let dst = info.rti_info[RTAX_DST].ok_or(Errno::EINVAL)?;
    let rt = rtable_lookup(
        tableid,
        dst,
        info.rti_info[RTAX_NETMASK],
        info.rti_info[RTAX_GATEWAY],
        prio,
    )
    .ok_or(Errno::ESRCH)?;

    // Make sure that's the route the caller want to delete.
    if ifp.if_index.get() != rt.rt_ifidx.get() {
        return Err(Errno::ESRCH);
    }

    // BFD: bfdclear(rt) for RTF_BFD routes, not configured.

    if rtable_delete(tableid, dst, info.rti_info[RTAX_NETMASK], &rt).is_err() {
        return Err(Errno::ESRCH);
    }

    // Release next hop cache before flushing cloned entries.
    rt_putgwroute(&rt, None, net);

    // Clean up any cloned children.
    if rt.rt_flags.isset(RTF_CLONING) {
        let _ = rtflushclone(&rt, tableid, net);
    }

    drop(rt.rt_parent.replace(None));

    rt.rt_flags.clear(RTF_UP);

    // The timers the C removes at the last rtfree (the module's redesign, R11).
    rt_timer_remove_all(&rt);

    kassert!(ifp.if_index.get() == rt.rt_ifidx.get());
    if_rtrequest(ifp, RTM_DELETE, &rt, net);

    rt.rt_trashed.store(true, Ordering::Relaxed);
    RTTRASH.fetch_add(1, Ordering::Relaxed);

    if let Some(r) = ret_nrt {
        *r = Some(rt);
    }

    RTGENERATION.fetch_add(1, Ordering::Release);

    Ok(())
}

/// `ifp->if_rtrequest(ifp, req, rt)`: the interface's link layer told of a route change
/// (`if_rtrequest_dummy` for an interface without a driver).
fn if_rtrequest(ifp: &'static Ifnet, req: u8, rt: &Rtentry, net: &NetHeld<'_>) {
    if let Some(d) = ifp.if_driver.get() {
        d.rtrequest(ifp, req, Some(rt), net);
    }
}

/// `rtrequest(req, info, prio, ret_nrt, tableid)`: `RTM_ADD` adds the route `info`
/// describes at priority `prio` (the interface's static one for 0); `RTM_RESOLVE` clones the
/// cloning route `*ret_nrt` into a host route of `info`'s destination; the new route given
/// to `ret_nrt` when asked for. `EINVAL` for another request, a request without an address
/// or a resolve without a cloning route, `EAFNOSUPPORT` without the table, `ENOBUFS` without
/// memory, `EEXIST` when the table refuses it. Under the net lock.
pub fn rtrequest(
    req: u8,
    info: &mut RtAddrinfo<'_>,
    prio: u8,
    ret_nrt: Option<&mut Option<PoolArc<Rtentry>>>,
    tableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    if !rtable_exists(tableid) {
        return Err(Errno::EAFNOSUPPORT);
    }
    if info.rti_flags & RTF_HOST != 0 {
        info.rti_info[RTAX_NETMASK] = None;
    }
    match req {
        RTM_DELETE => Err(Errno::EINVAL),
        RTM_RESOLVE => {
            let Some(ret) = ret_nrt else {
                return Err(Errno::EINVAL);
            };
            let Some(cloning) = ret.clone() else {
                return Err(Errno::EINVAL);
            };
            if !cloning.rt_flags.isset(RTF_CLONING) {
                return Err(Errno::EINVAL);
            }
            info.rti_ifa = cloning.rt_ifa.get();
            info.rti_flags = cloning.rt_flags.get() | (RTF_CLONED | RTF_HOST);
            info.rti_flags &= !(RTF_CLONING | RTF_CONNECTED | RTF_STATIC);
            let sa_dl = empty_sdl();
            let mut sa_rl2 = SockaddrRtlabel::default();
            let mut addrs = info.rti_info;
            addrs[RTAX_GATEWAY] = SockaddrView::new(&sa_dl);
            addrs[RTAX_LABEL] = rtlabel_id2sa(cloning.rt_labelid.get(), &mut sa_rl2);
            rtrequest_add(
                req,
                info.rti_ifa,
                info.rti_flags,
                &addrs,
                prio,
                Some(ret),
                tableid,
                net,
            )
        }
        RTM_ADD => {
            let addrs = info.rti_info;
            rtrequest_add(
                req,
                info.rti_ifa,
                info.rti_flags,
                &addrs,
                prio,
                ret_nrt,
                tableid,
                net,
            )
        }
        _ => Ok(()),
    }
}

/// `rtrequest`'s `RTM_ADD` (and `RTM_RESOLVE`'s fall through): the route of `addrs` on the
/// interface of `ifa` made and linked.
#[allow(
    clippy::too_many_arguments,
    reason = "rtrequest's own arguments, split"
)]
fn rtrequest_add(
    req: u8,
    ifa: Option<&'static Ifaddr>,
    flags: u32,
    addrs: &[Option<SockaddrView<'_>>; RTAX_MAX],
    prio: u8,
    ret_nrt: Option<&mut Option<PoolArc<Rtentry>>>,
    tableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let ifa = ifa.ok_or(Errno::EINVAL)?;
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("rtrequest: an address without its interface"));
    };
    let mut prio = prio;
    if prio == 0 {
        prio = ifp.if_priority.get().wrapping_add(RTP_STATIC);
    }
    let dst = addrs[RTAX_DST].ok_or(Errno::EINVAL)?;
    let mask = addrs[RTAX_NETMASK];
    let gateway = addrs[RTAX_GATEWAY];

    let (ndst, plen) = rt_copysa(dst, mask)?;

    let rt = Rtentry::alloc(ndst, plen, flags | RTF_UP, prio).ok_or(Errno::ENOBUFS)?;
    // The route goes back with this handle on a failure below, `RTF_UP` cleared first (the
    // C's `pool_put` of a route never linked).
    let fail = |rt: PoolArc<Rtentry>, e: Errno| -> Result<(), Errno> {
        rt.rt_flags.clear(RTF_UP);
        drop(rt);
        Err(e)
    };

    // Check the link state if the table supports it.
    if rtable_mpath_capable(tableid, dst.family())
        && !rt.rt_flags.isset(RTF_LOCAL)
        && (!link_state_is_up(ifp.if_link_state.get()) || ifp.if_flags.get() & IFF_UP == 0)
    {
        rt.rt_flags.clear(RTF_UP);
        rt.rt_priority.set(rt.rt_priority.get() | RTP_DOWN);
    }

    if let Some(l) = addrs[RTAX_LABEL] {
        let name = l.bytes().get(2..).unwrap_or(&[]);
        rt.rt_labelid.set(rtlabel_name2id(name));
    }

    // MPLS: rt_mpls_set/rt_mpls_clear for RTF_MPLS routes, not configured.

    rt.rt_ifa.set(Some(ifaref(ifa)));
    rt.rt_ifidx.set(ifp.if_index.get());
    // Copy metrics and a back pointer from the cloned route's parent.
    if rt.rt_flags.isset(RTF_CLONED)
        && let Some(parent) = ret_nrt.as_deref().and_then(Option::as_ref)
    {
        rt.rt_rmx.assign(&parent.rt_rmx);
        drop(rt.rt_parent.replace(Some(PoolArc::clone(parent))));
    }

    // We must set rt->rt_gateway before adding the route to the routing table because the
    // MPATH code uses it to (re)order routes.
    let Some(gateway) = gateway else {
        return fail(rt, Errno::EINVAL);
    };
    if let Err(e) = rt_setgate(&rt, gateway, tableid, net) {
        rt_putgwroute(&rt, None, net);
        return fail(rt, e);
    }

    let mut error = rtable_insert(
        tableid,
        rt.dest_view(),
        mask,
        Some(gateway),
        rt.rt_priority.get(),
        &rt,
    );
    if error.is_err()
        && let Some(crt) = rtable_match(tableid, rt.dest_view(), None)
    {
        // Overwrite cloned route.
        if crt.rt_flags.isset(RTF_CLONED) && !crt.rt_flags.isset(RTF_CACHED) {
            let Some(cifp) = if_get(crt.rt_ifidx.get()) else {
                kpanic(format_args!("rtrequest: a cloned route of no interface"));
            };
            let _ = rtdeletemsg(&crt, cifp, tableid, net);
            if_put(Some(cifp));

            error = rtable_insert(
                tableid,
                rt.dest_view(),
                mask,
                Some(gateway),
                rt.rt_priority.get(),
                &rt,
            );
        }
    }
    if error.is_err() {
        rt_putgwroute(&rt, None, net);
        drop(rt.rt_parent.replace(None));
        return fail(rt, Errno::EEXIST);
    }
    if_rtrequest(ifp, req, &rt, net);

    if_group_routechange(dst.bytes(), mask.map(|m| m.bytes()));

    if let Some(r) = ret_nrt {
        *r = Some(rt);
    }

    RTGENERATION.fetch_add(1, Ordering::Release);

    Ok(())
}

/// `rt_setgate(rt, gate, rtableid)`: `gate` as route `rt`'s gateway (a copy of
/// `ROUNDUP(sa_len)` bytes), and for a gateway route its next hop cached
/// ([`rt_setgwroute`]'s error answered, the gateway kept); `ENOBUFS` without memory.
pub fn rt_setgate(
    rt: &Rtentry,
    gate: SockaddrView<'_>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let glen = roundup_sa(usize::from(gate.len()));
    let same = rt
        .rt_gateway
        .read()
        .as_deref()
        .is_some_and(|g| core::ptr::eq(g.as_ptr(), gate.bytes().as_ptr()));
    if same {
        // nop
        return Ok(());
    }

    let b = gate.bytes();
    let sa = MArray::new(M_RTABLE, M_NOWAIT, glen, |i| b.get(i).copied().unwrap_or(0))
        .ok_or(Errno::ENOBUFS)?;

    klock::kernel_lock(); // see [X] in route.h
    let osa = rt.rt_gateway.replace(Some(sa));
    let error = if rt.rt_flags.isset(RTF_GATEWAY) {
        rt_setgwroute(rt, gate, rtableid, net)
    } else {
        Ok(())
    };
    klock::kernel_unlock();

    drop(osa);

    error
}

/// `rt_getll(rt)`: `f` of the route holding the next hop's link-layer address of `rt` (its
/// cached next hop for a gateway route, possibly none; `rt` itself otherwise), lent under
/// the cell.
pub fn rt_getll<R>(rt: &Rtentry, f: impl FnOnce(Option<&Rtentry>) -> R) -> R {
    if rt.rt_flags.isset(RTF_GATEWAY) {
        // We may return NULL here.
        let g = rt.rt_gwroute.read();
        return f(g.as_deref());
    }
    f(Some(rt))
}

/// `rt_maskedcopy(src, dst, netmask)`: `src` masked by `netmask` into `dst` (its header
/// copied, the bytes past the mask or `src` zeroed).
pub fn rt_maskedcopy(src: &[u8], dst: &mut [u8], netmask: &[u8]) {
    let slen = usize::from(src.first().copied().unwrap_or(0)).min(dst.len());
    let mlen = usize::from(netmask.first().copied().unwrap_or(0));
    let byte = |b: &[u8], i: usize| b.get(i).copied().unwrap_or(0);
    // The header (`sa_len` and `sa_family`).
    for (i, d) in dst.iter_mut().enumerate().take(2) {
        *d = byte(src, i);
    }
    let lim = mlen.min(slen);
    for (i, d) in dst.iter_mut().enumerate().take(lim).skip(2) {
        *d = byte(src, i) & byte(netmask, i);
    }
    for d in dst.iter_mut().take(slen).skip(lim.max(2)) {
        *d = 0;
    }
}

/// `rt_copysa(src, mask)`: the routing table's copy of destination `src` masked by `mask`
/// (`dom_sasize` bytes of `M_RTABLE`, its address bits past the prefix zero) and the prefix
/// length; `EAFNOSUPPORT` for a family without a routing table, `EINVAL` for an address
/// shorter than the family's or a bad mask, `ENOBUFS` without memory.
fn rt_copysa(
    src: SockaddrView<'_>,
    mask: Option<SockaddrView<'_>>,
) -> Result<(MArray<u8>, u32), Errno> {
    const MASKARRAY: [u8; 8] = [0x0, 0x80, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0xfe];

    let dp = rt_dom(src.family()).ok_or(Errno::EAFNOSUPPORT)?;

    if u32::from(src.len()) < dp.sasize {
        return Err(Errno::EINVAL);
    }

    let plen = rtable_satoplen(src.family(), mask)?;

    let off = dp.rtoffset as usize;
    let full = (plen / 8) as usize;
    let s = src.bytes();
    let ndst = MArray::new(M_RTABLE, M_NOWAIT, dp.sasize as usize, |i| {
        if i == 0 {
            dp.sasize as u8
        } else if i == 1 {
            src.family()
        } else if i >= off && i < off + full {
            s.get(i).copied().unwrap_or(0)
        } else if i == off + full && plen % 8 != 0 {
            s.get(i).copied().unwrap_or(0) & MASKARRAY[(plen % 8) as usize]
        } else {
            0
        }
    })
    .ok_or(Errno::ENOBUFS)?;

    Ok((ndst, plen))
}

/// `rt_ifa_add(ifa, flags, dst, rdomain)`: the route of interface address `ifa` to `dst`
/// (its local, broadcast, connected or link-layer route by `flags`) added in routing domain
/// `rdomain`, and the listeners told (`RTM_NEWADDR` for a local route, `RTM_ADD`).
pub fn rt_ifa_add(
    ifa: &'static Ifaddr,
    flags: u32,
    dst: SockaddrView<'_>,
    rdomain: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("rt_ifa_add: an address without its interface"));
    };
    let mut prio = ifp.if_priority.get().wrapping_add(RTP_STATIC);

    kassert!(rdomain == rtable_l2(rdomain));

    let sadl = sadl_copy(ifp);
    let addr = ifa.ifa_addr.read(net);
    let netmask = ifa.ifa_netmask.read(net);
    let mut sa_rl = SockaddrRtlabel::default();
    let mut info = RtAddrinfo::new();
    info.rti_ifa = Some(ifa);
    info.rti_flags = flags;
    info.rti_info[RTAX_DST] = Some(dst);
    info.rti_info[RTAX_GATEWAY] = if flags & RTF_LLINFO != 0 {
        sadl.as_ref().map(SaCopy::view)
    } else {
        addr.as_ref().and_then(|a| SockaddrView::new(a))
    };
    info.rti_info[RTAX_LABEL] = rtlabel_id2sa(ifp.if_rtlabelid.get(), &mut sa_rl);

    // MPLS: MPLS_OP_POP for RTF_MPLS, not configured.

    if flags & RTF_HOST == 0 {
        info.rti_info[RTAX_NETMASK] = netmask.as_ref().and_then(|m| SockaddrView::new(m));
    }

    if flags & (RTF_LOCAL | RTF_BROADCAST) != 0 {
        prio = RTP_LOCAL;
    }

    if flags & RTF_CONNECTED != 0 {
        prio = ifp.if_priority.get().wrapping_add(RTP_CONNECTED);
    }

    let mut rt = None;
    let error = rtrequest(RTM_ADD, &mut info, prio, Some(&mut rt), rdomain, net);
    if error.is_ok()
        && let Some(rt) = rt
    {
        // A local route is created for every address configured on an interface, so use
        // this information to notify userland that a new address has been added.
        if flags & RTF_LOCAL != 0 {
            rtsock::rtm_addr(RTM_NEWADDR, ifa, net);
        }
        rtsock::rtm_send(&rt, RTM_ADD, 0, rdomain, net);
    }
    error
}

/// `rt_ifa_del(ifa, flags, dst, rdomain)`: the route [`rt_ifa_add`] made deleted (its
/// destination masked by the address's netmask for a network route), and the listeners told
/// (`RTM_DELETE`, `RTM_DELADDR` for a local route); the table's source address cleared when
/// it is the interface address's.
pub fn rt_ifa_del(
    ifa: &'static Ifaddr,
    flags: u32,
    dst: SockaddrView<'_>,
    rdomain: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("rt_ifa_del: an address without its interface"));
    };
    let mut prio = ifp.if_priority.get().wrapping_add(RTP_STATIC);

    kassert!(rdomain == rtable_l2(rdomain));

    let addr = ifa.ifa_addr.read(net);
    let netmask = ifa.ifa_netmask.read(net);
    let mut deldst = SaCopy::zeroed();
    let mut dst = dst;
    if flags & RTF_HOST == 0
        && let Some(m) = &netmask
    {
        rt_maskedcopy(dst.bytes(), deldst.buf_mut(), m);
        dst = deldst.view();
    }

    let mut sa_rl = SockaddrRtlabel::default();
    let mut info = RtAddrinfo::new();
    info.rti_ifa = Some(ifa);
    info.rti_flags = flags;
    info.rti_info[RTAX_DST] = Some(dst);
    if flags & RTF_LLINFO == 0 {
        info.rti_info[RTAX_GATEWAY] = addr.as_ref().and_then(|a| SockaddrView::new(a));
    }
    info.rti_info[RTAX_LABEL] = rtlabel_id2sa(ifp.if_rtlabelid.get(), &mut sa_rl);

    if flags & RTF_HOST == 0 {
        info.rti_info[RTAX_NETMASK] = netmask.as_ref().and_then(|m| SockaddrView::new(m));
    }

    if flags & (RTF_LOCAL | RTF_BROADCAST) != 0 {
        prio = RTP_LOCAL;
    }

    if flags & RTF_CONNECTED != 0 {
        prio = ifp.if_priority.get().wrapping_add(RTP_CONNECTED);
    }

    if let Some(a) = addr.as_ref().and_then(|a| SockaddrView::new(a)) {
        crate::net::rtable::rtable_clearsource(rdomain, a, net);
    }
    let mut rt = None;
    klock::kernel_lock();
    let error = rtrequest_delete(&mut info, prio, ifp, Some(&mut rt), rdomain, net);
    klock::kernel_unlock();
    if error.is_ok()
        && let Some(rt) = rt
    {
        rtsock::rtm_send(&rt, RTM_DELETE, 0, rdomain, net);
        if flags & RTF_LOCAL != 0 {
            rtsock::rtm_addr(RTM_DELADDR, ifa, net);
        }
    }

    error
}

/// Whether `a` (a socket address's bytes) is an unspecified internet address
/// (`INADDR_ANY`, `in6addr_any`), for which no local route is made.
fn is_any_addr(a: &[u8]) -> bool {
    match a.get(1).copied() {
        Some(AF_INET) => a.get(SIN_ADDR..SIN_ADDR + 4) == Some(&[0; 4][..]),
        Some(AF_INET6) => a.get(SIN6_ADDR..SIN6_ADDR + 16) == Some(&[0; 16][..]),
        _ => false,
    }
}

/// `rt_ifa_addlocal(ifa)`: the local host route of interface address `ifa` added unless one
/// exists (none for an unspecified address, which would corrupt the routing tree's default
/// routes).
pub fn rt_ifa_addlocal(ifa: &'static Ifaddr, net: &NetHeld<'_>) -> Result<(), Errno> {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!(
            "rt_ifa_addlocal: an address without its interface"
        ));
    };
    let mut flags = RTF_HOST | RTF_LOCAL;

    let Some(addr) = ifa.ifa_addr.read(net) else {
        return Ok(());
    };
    // If the configured address correspond to the magical "any" address do not add a local
    // route entry because that might corrupt the routing tree which uses this value for the
    // default routes.
    if is_any_addr(&addr) {
        return Ok(());
    }

    if ifp.if_flags.get() & (IFF_LOOPBACK | IFF_POINTOPOINT) == 0 {
        flags |= RTF_LLINFO;
    }

    let Some(a) = SockaddrView::new(&addr) else {
        return Ok(());
    };
    // If there is no local entry, allocate one.
    let rt = rtalloc(a, 0, ifp.if_rdomain.get(), net);
    if rt
        .as_ref()
        .is_none_or(|r| r.rt_flags.get() & flags != flags)
    {
        rt_ifa_add(ifa, flags | RTF_MPATH, a, ifp.if_rdomain.get(), net)
    } else {
        Ok(())
    }
}

/// `rt_ifa_dellocal(ifa)`: the local host route of interface address `ifa` deleted when it
/// surely exists (so an interface direct route of the same destination survives the removal
/// of a subnet-router anycast address).
pub fn rt_ifa_dellocal(ifa: &'static Ifaddr, net: &NetHeld<'_>) -> Result<(), Errno> {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!(
            "rt_ifa_dellocal: an address without its interface"
        ));
    };
    let mut flags = RTF_HOST | RTF_LOCAL;

    let Some(addr) = ifa.ifa_addr.read(net) else {
        return Ok(());
    };
    // We do not add local routes for such address, so do not bother removing them.
    if is_any_addr(&addr) {
        return Ok(());
    }

    if ifp.if_flags.get() & (IFF_LOOPBACK | IFF_POINTOPOINT) == 0 {
        flags |= RTF_LLINFO;
    }

    let Some(a) = SockaddrView::new(&addr) else {
        return Ok(());
    };
    // Before deleting, check if a corresponding local host route surely exists.
    let rt = rtalloc(a, 0, ifp.if_rdomain.get(), net);
    if rt
        .as_ref()
        .is_some_and(|r| r.rt_flags.get() & flags == flags)
    {
        rt_ifa_del(ifa, flags, a, ifp.if_rdomain.get(), net)
    } else {
        Ok(())
    }
}

/// `rt_ifa_purge(ifa)`: every route of interface address `ifa` deleted, in every table of its
/// interface's routing domain.
pub fn rt_ifa_purge(ifa: &'static Ifaddr, net: &NetHeld<'_>) {
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!(
            "rt_ifa_purge: an address without its interface"
        ));
    };
    let Some(af) = ifa.ifa_addr.read(net).map(|a| a.family()) else {
        return;
    };

    for rtableid in 0..RTMAP_LIMIT.load(Ordering::Acquire) {
        // Skip rtables that are not in the rdomain of the ifp.
        if rtable_l2(rtableid) != ifp.if_rdomain.get() {
            continue;
        }

        let mut error;
        loop {
            let mut rt = None;
            error = rtable_walk(rtableid, af, Some(&mut rt), |r, id| {
                rt_ifa_purge_walker(r, ifa, id)
            });
            if let Some(r) = &rt
                && error == Err(Errno::EEXIST)
            {
                error = match rtdeletemsg(r, ifp, rtableid, net) {
                    Ok(()) => Err(Errno::EAGAIN),
                    e => e,
                };
            }
            drop(rt);
            if error != Err(Errno::EAGAIN) {
                break;
            }
        }

        if error == Err(Errno::EAFNOSUPPORT) {
            error = Ok(());
        }

        if error.is_err() {
            break;
        }
    }
}

/// `rt_ifa_purge_walker(rt, ifa, rtableid)`: `EEXIST` for a route of `ifa`.
fn rt_ifa_purge_walker(rt: &Rtentry, ifa: &Ifaddr, _rtableid: u32) -> Result<(), Errno> {
    if rt.rt_ifa.points_to(ifa) {
        return Err(Errno::EEXIST);
    }
    Ok(())
}

/// `RTTIMER_CALLOUT(r)`: an expired timer's work on its route: its queue's function, or the
/// deletion of a dynamic host route whose interface exists.
fn rttimer_callout(r: &Rttimer, rt: &PoolArc<Rtentry>, net: &NetHeld<'_>) {
    if let Some(Some(f)) = r.rtt_queue.rtq_func.get() {
        f(rt, r.rtt_tableid);
    } else {
        let ifp = if_get(rt.rt_ifidx.get());
        if let Some(ifp) = ifp
            && rt.rt_flags.get() & (RTF_DYNAMIC | RTF_HOST) == (RTF_DYNAMIC | RTF_HOST)
        {
            let _ = rtdeletemsg(rt, ifp, r.rtt_tableid, net);
        }
        if_put(ifp);
    }
}

/// `rt_timer_init()`: the `"rttmr"` pool (a second call, a test's, leaves it).
pub fn rt_timer_init() {
    let _ = RTTIMER_POOL.init_recorded(IPL_MPFLOOR, 0, "rttmr");
}

/// `rt_timer_queue_init(rtq, timeout, func)`: queue `rtq` of `timeout` seconds calling
/// `func` (none: delete dynamic host routes) for its expired routes.
pub fn rt_timer_queue_init(rtq: &'static RttimerQueue, timeout: i32, func: Option<RtqFunc>) {
    rtq.rtq_timeout.store(timeout, Ordering::Relaxed);
    rtq.rtq_count.store(0, Ordering::Relaxed);
    let _ = rtq.rtq_func.set(func);
}

/// `rt_timer_queue_change(rtq, timeout)`: the queue's timeout for new timers.
pub fn rt_timer_queue_change(rtq: &RttimerQueue, timeout: i32) {
    let _g = RTTIMER_MTX.lock();
    rtq.rtq_timeout.store(timeout, Ordering::Relaxed);
}

/// The handle of timer `r` (its pool is recorded).
fn timer_arc(r: &Rttimer) -> PoolArc<Rttimer> {
    match PoolArc::from_ref(r) {
        Some(h) => h,
        None => kpanic(format_args!("rttimer: not a pool item")),
    }
}

/// A timer off both lists given back: its route's handle (none when the route already left
/// it), the reference parked for its membership taken back.
fn rttimer_unlinked(
    r: &Rttimer,
    g: &mut MutexedGuard<'_, ()>,
) -> (PoolArc<Rttimer>, Option<PoolArc<Rtentry>>) {
    let rt = r.rtt_rt.get_mut(g).take();
    if let Some(rt) = &rt {
        rt.rt_ntimers.fetch_sub(1, Ordering::Relaxed);
    }
    match PoolArc::adopt(r) {
        Some(h) => (h, rt),
        None => kpanic(format_args!("rttimer: no reference parked")),
    }
}

/// The last handle of a timer that neither its queue nor its wheel holds any more: its
/// timeout's argument given back, the item freed with the handle.
fn rttimer_free(r: PoolArc<Rttimer>) {
    r.rtt_timeout.clear();
    drop(r);
}

/// `rt_timer_queue_flush(rtq)`: every timer of queue `rtq` taken off its route and its work
/// done now. Under the net lock, as the C's: the timers are first moved, under the timers'
/// mutex, off their routes onto a list of this call (through their route entries, the C's
/// local `TAILQ`), each timeout deleted; a timer whose timeout already fired stays on its
/// queue for `rt_timer_timer`, which the caller's net lock holds back until this call is done
/// and which then finds it without its route (its work done here). Then each timer's work is
/// done with the mutex let go, and the ones this call took off their queue are freed.
pub fn rt_timer_queue_flush(rtq: &'static RttimerQueue, net: &NetHeld<'_>) {
    let local: PinList<RttLink> = PinList::new();
    {
        let mut g = RTTIMER_MTX.lock();
        let mut cur = rtq.rtq_head.first(&g).map(timer_arc);
        while let Some(h) = cur.take() {
            cur = rtq.rtq_head.next(&g, &h).map(timer_arc);
            if let Some(rt) = h.rtt_rt.get(&g).clone() {
                rt.rt_timer.remove(&mut g, &h);
            }
            if h.rtt_timeout.del() {
                // Pending: off its queue now (a fired one is rt_timer_timer's to take off).
                rtq.rtq_head.remove(&mut g, &h);
                kassert!(rtq.rtq_count.load(Ordering::Relaxed) > 0);
                rtq.rtq_count.fetch_sub(1, Ordering::Relaxed);
            }
            local.insert_tail(&mut g, h.as_pin());
        }
    }

    loop {
        let (h, rt, parked) = {
            let mut g = RTTIMER_MTX.lock();
            let Some(e) = local.first(&g) else {
                break;
            };
            let h = timer_arc(e);
            local.remove(&mut g, &h);
            let rt = h.rtt_rt.get_mut(&mut g).take();
            if let Some(rt) = &rt {
                rt.rt_ntimers.fetch_sub(1, Ordering::Relaxed);
            }
            // Off its queue: this call's to free (the membership's reference taken back).
            let parked = if h.rtt_next.is_linked() {
                None
            } else {
                match PoolArc::adopt(&*h) {
                    Some(p) => Some(p),
                    None => kpanic(format_args!("rttimer: no reference parked")),
                }
            };
            (h, rt, parked)
        };
        if let Some(rt) = &rt {
            rttimer_callout(&h, rt, net);
        }
        drop(rt);
        drop(h);
        if let Some(p) = parked {
            rttimer_free(p);
        }
    }
}

/// `rt_timer_queue_count(rtq)`: the timers on queue `rtq`.
pub fn rt_timer_queue_count(rtq: &RttimerQueue) -> u64 {
    rtq.rtq_count.load(Ordering::Relaxed)
}

/// `rt_timer_unlink(r)`: timer `r` off its route (its route handle given back); when its
/// timeout was still pending, off its queue too, and its handle answered for the caller to
/// free; `None` when the timeout fired already (`rt_timer_timer` does the cleanup).
fn rt_timer_unlink(
    r: &Rttimer,
    rt: &Rtentry,
    g: &mut MutexedGuard<'_, ()>,
) -> (Option<PoolArc<Rttimer>>, Option<PoolArc<Rtentry>>) {
    rt.rt_timer.remove(g, r);
    let old = r.rtt_rt.get_mut(g).take();
    if old.is_some() {
        rt.rt_ntimers.fetch_sub(1, Ordering::Relaxed);
    }

    if !r.rtt_timeout.del() {
        // Timeout fired, so rt_timer_timer will do the cleanup.
        return (None, old);
    }

    r.rtt_queue.rtq_head.remove(g, r);
    kassert!(r.rtt_queue.rtq_count.load(Ordering::Relaxed) > 0);
    r.rtt_queue.rtq_count.fetch_sub(1, Ordering::Relaxed);
    match PoolArc::adopt(r) {
        Some(h) => (Some(h), old),
        None => kpanic(format_args!("rttimer: no reference parked")),
    }
}

/// `rt_timer_remove_all(rt)`: every timer of route `rt` removed (a fired one left to
/// `rt_timer_timer`).
pub fn rt_timer_remove_all(rt: &Rtentry) {
    loop {
        let (r, old) = {
            let mut g = RTTIMER_MTX.lock();
            let Some(e) = rt.rt_timer.first(&g) else {
                break;
            };
            let h = timer_arc(e);
            let (r, old) = rt_timer_unlink(&h, rt, &mut g);
            drop(h);
            (r, old)
        };
        drop(old);
        if let Some(r) = r {
            rttimer_free(r);
        }
    }
}

/// `rt_timer_get_expire(rt)`: the earliest expire time of route `rt`'s timers (0 for none).
pub fn rt_timer_get_expire(rt: &Rtentry) -> i64 {
    let g = RTTIMER_MTX.lock();
    let mut expire = 0;
    for r in rt.rt_timer.iter(&g) {
        if expire == 0 || expire > r.rtt_expire {
            expire = r.rtt_expire;
        }
    }
    expire
}

/// `rt_timer_add(rt, queue, rtableid)`: a timer of `queue`'s timeout for route `rt` of table
/// `rtableid`, replacing the route's timer of that queue; `ENOBUFS` without memory.
pub fn rt_timer_add(
    rt: &PoolArc<Rtentry>,
    queue: &'static RttimerQueue,
    rtableid: u32,
) -> Result<(), Errno> {
    let rnew = RTTIMER_POOL
        .get_homed(PR_NOWAIT, || Rttimer {
            home: PoolHome::new(),
            rtt_next: PinEntry::new(),
            rtt_link: PinEntry::new(),
            rtt_timeout: Timeout::unset(),
            rtt_queue: queue,
            rtt_rt: Guarded::new_static_mutexed(&RTTIMER_MTX, None),
            rtt_expire: crate::tc::getuptime()
                + i64::from(queue.rtq_timeout.load(Ordering::Relaxed)),
            rtt_tableid: rtableid,
        })
        .ok_or(Errno::ENOBUFS)?;
    rnew.rtt_timeout.set_shared(
        rt_timer_timer,
        PoolArc::erase(PoolArc::clone(&rnew)),
        KCLOCK_NONE,
        TIMEOUT_PROC,
    );

    let (old, oldrt) = {
        let mut g = RTTIMER_MTX.lock();
        // If there's already a timer with this action, destroy it before we add a new one.
        let mut found = None;
        for r in rt.rt_timer.iter(&g) {
            if core::ptr::eq(r.rtt_queue, queue) {
                found = Some(timer_arc(r));
                break; // only one per list, so we can quit...
            }
        }
        let (old, oldrt) = match &found {
            Some(r) => rt_timer_unlink(r, rt, &mut g),
            None => (None, None),
        };
        drop(found);

        *rnew.rtt_rt.get_mut(&mut g) = Some(PoolArc::clone(rt));
        rt.rt_ntimers.fetch_add(1, Ordering::Relaxed);
        rt.rt_timer.insert_head(&mut g, rnew.as_pin());
        queue.rtq_head.insert_tail(&mut g, rnew.as_pin());
        // The membership's reference, taken back when the timer leaves its queue.
        PoolArc::forget(PoolArc::clone(&rnew));
        hal::pin::project(rnew.as_pin(), |r| &r.rtt_timeout)
            .add_sec(queue.rtq_timeout.load(Ordering::Relaxed));
        queue.rtq_count.fetch_add(1, Ordering::Relaxed);
        (old, oldrt)
    };

    drop(oldrt);
    if let Some(r) = old {
        rttimer_free(r);
    }

    Ok(())
}

/// `rt_timer_timer(arg)`: an expired timer (`arg`): off its queue and its route, its work
/// done on the route it still has, under the net lock; then freed.
fn rt_timer_timer(arg: &AnyArc) {
    let Some(r) = arg.downcast_ref::<Rttimer>() else {
        kpanic(format_args!("rt_timer_timer: not a route timer"));
    };
    let rtq = r.rtt_queue;

    let w = hal::netlock::NETLOCK.write();
    let (parked, rt) = {
        let mut g = RTTIMER_MTX.lock();

        if let Some(rt) = r.rtt_rt.get(&g).clone() {
            rt.rt_timer.remove(&mut g, r);
        }
        rtq.rtq_head.remove(&mut g, r);
        kassert!(rtq.rtq_count.load(Ordering::Relaxed) > 0);
        rtq.rtq_count.fetch_sub(1, Ordering::Relaxed);

        rttimer_unlinked(r, &mut g)
    };

    if let Some(rt) = &rt
        && let Some(net) = NetHeld::of(&w)
    {
        rttimer_callout(r, rt, &net);
    }
    drop(rt);
    drop(w);

    rttimer_free(parked);
}

/// `rtlabel_name2id(name)`: the id of route label `name` (a reference taken), a new one for
/// a new name (the first free id, the chain kept in id order); 0 for an empty name, past
/// `LABELID_MAX` or without memory.
pub fn rtlabel_name2id(name: &[u8]) -> u16 {
    let name = cstr(name);
    if name.is_empty() {
        return 0;
    }
    let name = &name[..name.len().min(RTLABEL_LEN - 1)];

    let mut g = RT_LABELS.lock();
    let mut l = g.as_deref_mut();
    while let Some(label) = l {
        if label.name() == name {
            label.rtl_ref += 1;
            return label.rtl_id;
        }
        l = label.rtl_next.as_deref_mut();
    }

    // To avoid fragmentation, we do a linear search from the beginning and take the first
    // free slot we find. If there is none or the list is empty, append a new entry at the
    // end.
    let mut new_id = 1u16;
    let mut p: &mut Option<MBox<RtLabel>> = &mut g;
    loop {
        if p.as_ref().is_none_or(|l| l.rtl_id != new_id) {
            break;
        }
        match p {
            Some(l) => {
                new_id = l.rtl_id + 1;
                p = &mut l.rtl_next;
            }
            None => return 0,
        }
    }
    if new_id > LABELID_MAX {
        return 0;
    }

    let mut rtl_name = [0u8; RTLABEL_LEN];
    strlcpy(&mut rtl_name, name);
    let next = p.take();
    let Some(label) = MBox::new(
        M_RTABLE,
        M_NOWAIT,
        RtLabel {
            rtl_next: next,
            rtl_name,
            rtl_id: new_id,
            rtl_ref: 1,
        },
    ) else {
        return 0;
    };
    // Insert the new entry before p, or at the end.
    *p = Some(label);
    new_id
}

/// Forgets every route label (never frees them), for the host tests, which number labels
/// from 1 and load fresh kernel memory per test.
#[cfg(any(test, feature = "doubles"))]
pub fn rtlabel_test_reset() {
    core::mem::forget(RT_LABELS.lock().take());
}

/// `rtlabel_id2name(id, buf)`: route label `id`'s name copied into `buf` (`strlcpy`), its
/// bytes answered; `None` for 0 or an unknown id.
pub fn rtlabel_id2name(id: u16, buf: &mut [u8]) -> Option<&[u8]> {
    if id == 0 {
        return None;
    }
    let g = RT_LABELS.lock();
    let label = rtlabel_find(g.as_deref(), id)?;
    strlcpy(buf, &label.rtl_name);
    drop(g);
    Some(cstr(buf))
}

/// `rtlabel_id2name_locked(id)`: the label of id `id` on the chain `g`.
fn rtlabel_find(first: Option<&RtLabel>, id: u16) -> Option<&RtLabel> {
    let mut l = first;
    while let Some(label) = l {
        if label.rtl_id == id {
            return Some(label);
        }
        l = label.rtl_next.as_deref();
    }
    None
}

/// `rtlabel_id2sa(labelid, sa_rl)`: route label `labelid` as a socket address built in
/// `sa_rl`; `None` for 0 or an unknown id.
pub fn rtlabel_id2sa(labelid: u16, sa_rl: &mut SockaddrRtlabel) -> Option<SockaddrView<'_>> {
    if labelid == 0 {
        return None;
    }

    {
        let g = RT_LABELS.lock();
        let label = rtlabel_find(g.as_deref(), labelid)?;
        *sa_rl = SockaddrRtlabel::default();
        sa_rl.sr_len = core::mem::size_of::<SockaddrRtlabel>() as u8;
        sa_rl.sr_family = AF_UNSPEC;
        strlcpy(&mut sa_rl.sr_label, &label.rtl_name);
    }

    SockaddrView::new(hal::user::bytes_of(core::slice::from_ref(sa_rl)))
}

/// `rtlabel_unref(id)`: one reference of route label `id` given back; with the last, the
/// label leaves the chain and is freed.
pub fn rtlabel_unref(id: u16) {
    if id == 0 {
        return;
    }

    let mut g = RT_LABELS.lock();
    let mut p: &mut Option<MBox<RtLabel>> = &mut g;
    loop {
        if p.as_ref().is_none_or(|l| l.rtl_id == id) {
            break;
        }
        match p {
            Some(l) => p = &mut l.rtl_next,
            None => return,
        }
    }
    let Some(l) = p.as_deref_mut() else {
        return;
    };
    l.rtl_ref -= 1;
    if l.rtl_ref == 0 {
        let next = l.rtl_next.take();
        let gone = core::mem::replace(p, next);
        drop(g);
        drop(gone);
    }
}

/// `rt_if_track(ifp)`: the routes of interface `ifp` brought up or down after a link state
/// change, in every table of its routing domain (its cloned and dynamic routes deleted when it
/// goes down).
pub fn rt_if_track(ifp: &'static Ifnet, net: &NetHeld<'_>) -> Result<(), Errno> {
    let mut error = Ok(());

    for rtableid in 0..RTMAP_LIMIT.load(Ordering::Acquire) {
        // Skip rtables that are not in the rdomain of the ifp.
        if rtable_l2(rtableid) != ifp.if_rdomain.get() {
            continue;
        }
        for i in 1..=AF_MAX {
            if !rtable_mpath_capable(rtableid, i) {
                continue;
            }

            loop {
                let mut rt = None;
                error = rtable_walk(rtableid, i, Some(&mut rt), |r, id| {
                    rt_if_linkstate_change(r, ifp, id)
                });
                if let Some(r) = &rt
                    && error == Err(Errno::EEXIST)
                {
                    error = match rtdeletemsg(r, ifp, rtableid, net) {
                        Ok(()) => Err(Errno::EAGAIN),
                        e => e,
                    };
                }
                drop(rt);
                if error != Err(Errno::EAGAIN) {
                    break;
                }
            }

            if error == Err(Errno::EAFNOSUPPORT) {
                error = Ok(());
            }

            if error.is_err() {
                break;
            }
        }
    }

    error
}

/// `rt_if_linkstate_change(rt, ifp, id)`: route `rt` of `ifp` up when the link is up and the
/// interface up, down otherwise (its priority moved by `RTP_DOWN`); `EEXIST` for a cloned or
/// dynamic route to delete on a down interface; the egress group and the generation updated.
pub fn rt_if_linkstate_change(rt: &Rtentry, ifp: &Ifnet, id: u32) -> Result<(), Errno> {
    if rt.rt_ifidx.get() != ifp.if_index.get() {
        return Ok(());
    }

    // Local routes are always usable.
    if rt.rt_flags.isset(RTF_LOCAL) {
        rt.rt_flags.set_bits(RTF_UP);
        return Ok(());
    }

    let error = if link_state_is_up(ifp.if_link_state.get()) && ifp.if_flags.get() & IFF_UP != 0 {
        if rt.rt_flags.isset(RTF_UP) {
            return Ok(());
        }

        // Bring route up.
        rt.rt_flags.set_bits(RTF_UP);
        rtable_mpath_reprio(
            id,
            rt.dest_view(),
            rt.rt_plen(),
            rt.rt_priority.get() & RTP_MASK,
            rt,
        )
    } else {
        // Remove redirected and cloned routes (mainly ARP) from down interfaces so we have a
        // chance to get new routes from a better source.
        if rt.rt_flags.isset(RTF_CLONED | RTF_DYNAMIC)
            && !rt.rt_flags.isset(RTF_CACHED | hal::sys::net_route::RTF_BFD)
        {
            return Err(Errno::EEXIST);
        }

        if !rt.rt_flags.isset(RTF_UP) {
            return Ok(());
        }

        // Take route down.
        rt.rt_flags.clear(RTF_UP);
        rtable_mpath_reprio(
            id,
            rt.dest_view(),
            rt.rt_plen(),
            rt.rt_priority.get() | RTP_DOWN,
            rt,
        )
    };
    let mut sa_mask = SaCopy::zeroed();
    let mask = rt_plen2mask(rt, &mut sa_mask);
    if_group_routechange(rt.dest(), mask.map(|m| m.bytes()));

    RTGENERATION.fetch_add(1, Ordering::Release);

    error
}

/// `in_prefixlen2mask` / `in6_prefixlen2mask`: the first `plen` bits of `out` set.
fn prefixlen2mask(out: &mut [u8], plen: u32) {
    for (i, b) in out.iter_mut().enumerate() {
        let bits = plen.saturating_sub(8 * i as u32).min(8);
        *b = if bits == 0 { 0 } else { 0xffu8 << (8 - bits) };
    }
}

/// `rt_plentosa(af, plen, sa_mask)`: the mask of prefix length `plen` (none for `None`, the
/// C's -1) as a socket address of family `af`, built in `sa_mask`; `None` for a family
/// without masks.
pub fn rt_plentosa(
    af: SaFamily,
    plen: Option<u32>,
    sa_mask: &mut SaCopy,
) -> Option<SockaddrView<'_>> {
    let plen = plen?;

    *sa_mask = SaCopy::zeroed();
    let b = sa_mask.buf_mut();

    match af {
        AF_INET => {
            b[1] = AF_INET;
            b[0] = SIN_LEN as u8;
            prefixlen2mask(&mut b[SIN_ADDR..SIN_ADDR + 4], plen);
        }
        AF_INET6 => {
            b[1] = AF_INET6;
            b[0] = SIN6_LEN as u8;
            prefixlen2mask(&mut b[SIN6_ADDR..SIN6_ADDR + 16], plen);
        }
        _ => return None,
    }

    Some(sa_mask.view())
}

/// `rt_plen2mask(rt, sa_mask)`: route `rt`'s mask, built in `sa_mask`.
pub fn rt_plen2mask<'a>(rt: &Rtentry, sa_mask: &'a mut SaCopy) -> Option<SockaddrView<'a>> {
    rt_plentosa(rt.dest_view().family(), Some(rt.rt_plen()), sa_mask)
}

/// `refcnt_read(&rt->rt_refcnt)`: the references of route `rt` the C counts. Left out: its
/// timers' handles, the references the routes unlinked before it keep in their `rt_next`, and
/// the retired references whose deferred release has not run (the C drops those at once).
pub fn rt_refcnt(rt: &Rtentry) -> usize {
    PoolArc::count_of(rt)
        .saturating_sub(rt.rt_ntimers.load(Ordering::Relaxed) as usize)
        .saturating_sub(rt.rt_backrefs.load(Ordering::Relaxed) as usize)
        .saturating_sub(hal::smr::retired_count(rt))
}

/// `db_print_sa(sa)`: a socket address's bytes, `[NULL]` for none.
pub fn db_print_sa(w: &mut dyn fmt::Write, sa: Option<&[u8]>) -> fmt::Result {
    let Some(sa) = sa else {
        return write!(w, "[NULL]");
    };
    let len = usize::from(sa.first().copied().unwrap_or(0)).min(sa.len());
    write!(w, "[")?;
    for (i, b) in sa[..len].iter().enumerate() {
        write!(w, "{b}")?;
        if i + 1 < len {
            write!(w, ",")?;
        }
    }
    writeln!(w, "]")
}

/// `db_print_ifa(ifa)`: an interface address's addresses, flags, references and metric.
pub fn db_print_ifa(
    w: &mut dyn fmt::Write,
    ifa: Option<&Ifaddr>,
    net: &NetHeld<'_>,
) -> fmt::Result {
    let Some(ifa) = ifa else {
        return Ok(());
    };
    write!(w, "  ifa_addr=")?;
    db_print_sa(w, ifa.ifa_addr.read(net).as_deref())?;
    write!(w, "  ifa_dsta=")?;
    db_print_sa(w, ifa.ifa_dstaddr.read(net).as_deref())?;
    write!(w, "  ifa_mask=")?;
    db_print_sa(w, ifa.ifa_netmask.read(net).as_deref())?;
    writeln!(
        w,
        "  flags=0x{:x}, refcnt={}, metric={}",
        ifa.ifa_flags.get(),
        ifa.ifa_refcnt.read(),
        ifa.ifa_metric.get()
    )
}

/// `db_show_rtentry(rt, w, id)`: one route, for `rtable_walk`.
pub fn db_show_rtentry(w: &mut dyn fmt::Write, rt: &Rtentry, net: &NetHeld<'_>) -> fmt::Result {
    write!(w, "rtentry={rt:p}")?;

    writeln!(
        w,
        " flags=0x{:x} refcnt={} use={} expire={}",
        rt.rt_flags.get(),
        rt_refcnt(rt),
        rt.rt_use().load(Ordering::Relaxed),
        rt.rt_expire().load(Ordering::Relaxed)
    )?;

    write!(w, " key=")?;
    db_print_sa(w, Some(rt.dest()))?;
    write!(w, " plen={}", rt.rt_plen())?;
    write!(w, " gw=")?;
    rt.with_gateway(|g| db_print_sa(w, g))?;
    write!(w, " ifidx={} ", rt.rt_ifidx.get())?;
    let ifa = rt.rt_ifa.get();
    writeln!(
        w,
        " ifa={:p}",
        ifa.map_or(core::ptr::null(), core::ptr::from_ref)
    )?;
    db_print_ifa(w, ifa, net)?;

    let gwroute = rt.gwroute();
    writeln!(
        w,
        " gwroute={:p} llinfo={:p} priority={}",
        gwroute
            .as_deref()
            .map_or(core::ptr::null(), core::ptr::from_ref),
        rt.rt_llinfo.get(),
        rt.rt_priority.get()
    )
}

/// `db_show_rtable(af, rtableid)`: every route of a table.
pub fn db_show_rtable(
    w: &mut dyn fmt::Write,
    af: SaFamily,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> fmt::Result {
    writeln!(w, "Route tree for af {af}, rtableid {rtableid}")?;
    let mut out = Ok(());
    let _ = rtable_walk(rtableid, af, None, |rt, _| {
        out = out.and_then(|()| db_show_rtentry(w, rt, net));
        Ok(())
    });
    out
}

const _: () = {
    assert!(SA_COPY_MAX >= SIN6_LEN && SA_COPY_MAX >= SDL_LEN);
    assert!(core::mem::size_of::<SockaddrRtlabel>() == 34);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for the routes: masked copies and prefix masks, the labels, the route caches,
    // requests that add, clone and delete routes over a test interface (the reference counts
    // against the C's at each step), the next hop cache, the timers, the link state tracking
    // and the routing socket's messages (a recording sink).

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::smr;
    use hal::sys::net_route::{RTM_ADD, RTP_ANY};

    use super::*;
    use crate::net::if_var::NetDriver;
    use crate::net::rtable::{RtDom, rtable_get, rtable_init_doms, rtable_test_reset};

    /// The routing domains of the tests: inet and inet6, as `domains[]` gives them.
    pub(crate) const ROUTE_DOMS: [RtDom; 2] = [
        RtDom {
            family: AF_INET,
            rtoffset: SIN_ADDR as u32,
            maxplen: 32,
            sasize: SIN_LEN as u32,
        },
        RtDom {
            family: AF_INET6,
            rtoffset: SIN6_ADDR as u32,
            maxplen: 128,
            sasize: SIN6_LEN as u32,
        },
    ];

    /// Fresh interfaces, memory and routing tables (table 0 of inet and inet6).
    pub(crate) fn testroutes_setup() -> MutexGuard<'static, ()> {
        let g = crate::net::if_::tests::setup();
        rtable_test_reset();
        rtlabel_test_reset();
        route_init();
        rtable_init_doms(&ROUTE_DOMS);
        MULTIPATH.store(false, Ordering::Relaxed);
        let _ = if_::register_inet(&TEST_INET);
        crate::net::rtsock::tap::start();
        g
    }

    /// Every release queued so far run.
    pub(crate) fn grace() {
        struct Alone;
        impl smr::CpuWalk for Alone {
            fn running(&self, _n: usize) -> bool {
                false
            }
            fn peg(&self, n: usize) {
                panic!("peg({n}) with no other CPU");
            }
            fn unpeg(&self) {}
        }
        loop {
            smr::idle();
            if smr::collect().run(&Alone) == 0 {
                break;
            }
        }
    }

    /// A `sockaddr_in` of `a`.
    pub(crate) fn sin(a: [u8; 4]) -> [u8; 16] {
        let mut s = [0u8; 16];
        s[0] = 16;
        s[1] = AF_INET;
        s[4..8].copy_from_slice(&a);
        s
    }

    /// A `sockaddr_in6` of `a`.
    pub(crate) fn sin6(a: [u8; 16]) -> [u8; 28] {
        let mut s = [0u8; 28];
        s[0] = 28;
        s[1] = AF_INET6;
        s[8..24].copy_from_slice(&a);
        s
    }

    fn v(b: &[u8]) -> SockaddrView<'_> {
        SockaddrView::new(b).expect("a view")
    }

    /// A route of `dst`/`plen` with gateway `gw`, made as `rtrequest` makes one but not
    /// linked anywhere (for the table's tests).
    pub(crate) fn entry(
        dst: &[u8],
        plen: u32,
        gw: Option<&[u8]>,
        flags: u32,
        prio: u8,
    ) -> PoolArc<Rtentry> {
        let (ndst, p) = rt_copysa(v(dst), mask_of(dst[1], plen).as_ref().map(SaCopy::view))
            .expect("a destination");
        assert_eq!(p, plen);
        let rt = Rtentry::alloc(ndst, plen, flags, prio).expect("an entry");
        if let Some(gw) = gw {
            let b = gw.to_vec();
            let sa = MArray::new(M_RTABLE, M_NOWAIT, roundup_sa(b.len()), |i| {
                b.get(i).copied().unwrap_or(0)
            })
            .expect("a gateway");
            drop(rt.rt_gateway.replace(Some(sa)));
        }
        rt
    }

    /// The mask of `plen` bits for `af` (none for a host route).
    fn mask_of(af: SaFamily, plen: u32) -> Option<SaCopy> {
        let max = if af == AF_INET { 32 } else { 128 };
        if plen == max {
            return None;
        }
        let mut m = SaCopy::zeroed();
        rt_plentosa(af, Some(plen), &mut m)?;
        Some(m)
    }

    /// The multipath switch of the test protocols.
    static MULTIPATH: AtomicBool = AtomicBool::new(false);

    pub(crate) fn set_multipath(on: bool) {
        MULTIPATH.store(on, Ordering::Relaxed);
    }

    /// The internet protocols of the tests: multipath on demand, the cache counters counted.
    struct TestInet {
        hits: AtomicU32,
        misses: AtomicU32,
        freed: AtomicU32,
    }

    impl if_::Inet for TestInet {
        fn arpintr(&self) {}
        fn ipintr(&self) {}
        fn ip6intr(&self) {}
        fn tcp_input_mlist(&self, _ml: &mut crate::net::mbuf::MbufList, _af: SaFamily) {}
        fn ipv4_input(&self) -> crate::net::if_::ProtoFn {
            crate::net::if_::test_proto_count
        }
        fn ipv6_input(&self) -> crate::net::if_::ProtoFn {
            crate::net::if_::test_proto_count
        }
        fn tcpstat_lro(&self, _hw: bool) {}
        fn nd6_ifattach(&self, _ifp: &'static Ifnet) {}
        fn nd6_ifdetach(&self, _ifp: &'static Ifnet) {}
        fn rti_delete(&self, _ifp: &'static Ifnet) {}
        fn revarp_ifdetach(&self, _ifp: &'static Ifnet) {}
        fn in_ifdetach(&self, _ifp: &'static Ifnet) {}
        fn in6_ifdetach(&self, _ifp: &'static Ifnet) {}
        fn in6_ifattach(&self, _ifp: &'static Ifnet) -> Result<(), Errno> {
            Ok(())
        }
        fn in_ioctl(
            &self,
            _cmd: u64,
            _data: &mut [u8],
            _ifp: &'static Ifnet,
            _privileged: bool,
        ) -> Result<(), Errno> {
            Err(Errno::EOPNOTSUPP)
        }
        fn ip6_forwarding(&self) -> bool {
            false
        }
        fn in6_purge_linklocal(&self, _ifp: &'static Ifnet) -> bool {
            false
        }
        fn ifa_free(&self, _ifa: &'static Ifaddr) {
            self.freed.fetch_add(1, Ordering::Relaxed);
        }
        fn ip_multipath(&self) -> bool {
            MULTIPATH.load(Ordering::Relaxed)
        }
        fn ip6_multipath(&self) -> bool {
            MULTIPATH.load(Ordering::Relaxed)
        }
        fn ipstat_rtcache(&self, hit: bool) {
            if hit {
                self.hits.fetch_add(1, Ordering::Relaxed);
            } else {
                self.misses.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn ip6stat_rtcache(&self, hit: bool) {
            self.ipstat_rtcache(hit);
        }
    }

    static TEST_INET: TestInet = TestInet {
        hits: AtomicU32::new(0),
        misses: AtomicU32::new(0),
        freed: AtomicU32::new(0),
    };

    /// The types of the routing messages the routing socket's tap saw.
    fn messages() -> Vec<u8> {
        crate::net::rtsock::tap::types()
    }

    /// A driver that counts the route requests it is told of.
    pub(crate) struct RtDriver {
        adds: AtomicU32,
        resolves: AtomicU32,
        deletes: AtomicU32,
        /// Packets output; the output fails at this count (0: never).
        outputs: AtomicU32,
        fail_at: AtomicU32,
    }

    impl NetDriver for RtDriver {
        fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            Ok(())
        }
        fn rtrequest(
            &self,
            _ifp: &'static Ifnet,
            req: u8,
            rt: Option<&Rtentry>,
            _net: &NetHeld<'_>,
        ) {
            assert!(rt.is_some());
            match req {
                RTM_ADD => &self.adds,
                RTM_RESOLVE => &self.resolves,
                _ => &self.deletes,
            }
            .fetch_add(1, Ordering::Relaxed);
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            m: crate::net::mbuf::MbufBox,
            dst: SockaddrView<'_>,
            rt: Option<&Rtentry>,
        ) -> Result<(), Errno> {
            assert_eq!(dst.family(), AF_INET);
            assert!(rt.is_some());
            drop(m);
            let n = self.outputs.fetch_add(1, Ordering::Relaxed) + 1;
            if n == self.fail_at.load(Ordering::Relaxed) {
                return Err(Errno::ENOBUFS);
            }
            Ok(())
        }
    }

    /// An interface up, its driver, and its IPv4 address `addr`/`mask`.
    pub(crate) fn iface(
        addr: [u8; 4],
        mask: [u8; 4],
    ) -> (&'static Ifnet, &'static RtDriver, &'static Ifaddr) {
        let d: &'static RtDriver = Box::leak(Box::new(RtDriver {
            adds: AtomicU32::new(0),
            resolves: AtomicU32::new(0),
            deletes: AtomicU32::new(0),
            outputs: AtomicU32::new(0),
            fail_at: AtomicU32::new(0),
        }));
        let ifp = crate::net::if_::test_ifnet_with(b"rt0", d);
        ifp.if_flags.set(IFF_UP | IFF_BROADCAST);
        crate::net::if_::if_alloc_sadl(ifp);
        let ifa: &'static Ifaddr = Box::leak(Box::new(Ifaddr::new()));
        ifa.ifa_refcnt.init();
        let token = crate::net::if_var::sa_test_token();
        let cell = |a: [u8; 4]| {
            let mut sa: hal::sys::socket::Sockaddr = hal::user::zeroed();
            sa.sa_len = 16;
            sa.sa_family = AF_INET;
            sa.sa_data[2..6].copy_from_slice(&a);
            &*Box::leak(Box::new(core::cell::Cell::new(sa)))
        };
        ifa.ifa_addr.set(token, Some(cell(addr)));
        ifa.ifa_netmask.set(token, Some(cell(mask)));
        ifa.ifa_ifp.set(Some(ifp));
        crate::net::if_::ifa_add(ifp, ifa);
        (ifp, d, ifa)
    }

    #[test]
    fn masked_copies_and_prefix_masks() {
        let src = sin([10, 0, 2, 15]);
        let mask = sin([255, 255, 255, 0]);
        let mut out = [0xaau8; 16];
        rt_maskedcopy(&src, &mut out, &mask);
        assert_eq!(out, sin([10, 0, 2, 0]));
        // A mask shorter than the address: the bytes past it zeroed.
        let mut short = mask;
        short[0] = 6;
        rt_maskedcopy(&src, &mut out, &short);
        assert_eq!(&out[..8], &[16, AF_INET, 0, 0, 10, 0, 0, 0]);

        let mut m = SaCopy::zeroed();
        let s = rt_plentosa(AF_INET, Some(20), &mut m).expect("a mask");
        assert_eq!(s.bytes(), &sin([255, 255, 240, 0])[..]);
        assert!(rt_plentosa(AF_INET, None, &mut m).is_none(), "-1");
        assert!(rt_plentosa(AF_LINK, Some(8), &mut m).is_none());
        let s = rt_plentosa(AF_INET6, Some(65), &mut m).expect("a mask");
        let mut want = [0u8; 16];
        want[..8].fill(0xff);
        want[8] = 0x80;
        assert_eq!(s.bytes(), &sin6(want)[..]);
        assert_eq!(roundup_sa(0), 8);
        assert_eq!(roundup_sa(16), 16);
        assert_eq!(roundup_sa(17), 24);
    }

    #[test]
    fn labels_are_named_counted_and_reused() {
        let _g = testroutes_setup();
        assert_eq!(rtlabel_name2id(b""), 0);
        let a = rtlabel_name2id(b"red\0");
        let b = rtlabel_name2id(b"blue");
        assert_eq!((a, b), (1, 2));
        assert_eq!(rtlabel_name2id(b"red"), a, "a reference more");
        let mut buf = [0u8; RTLABEL_LEN];
        assert_eq!(rtlabel_id2name(a, &mut buf), Some(&b"red"[..]));
        assert_eq!(rtlabel_id2name(0, &mut buf), None);
        assert_eq!(rtlabel_id2name(9, &mut buf), None);
        let mut sa = SockaddrRtlabel::default();
        let s = rtlabel_id2sa(b, &mut sa).expect("a label address");
        assert_eq!(s.len(), 34);
        assert_eq!(s.family(), AF_UNSPEC);
        assert_eq!(&s.bytes()[2..6], b"blue");
        // red goes with its second unref; its id is the first free one again.
        rtlabel_unref(a);
        assert_eq!(rtlabel_id2name(a, &mut buf), Some(&b"red"[..]));
        rtlabel_unref(a);
        assert_eq!(rtlabel_id2name(a, &mut buf), None);
        assert_eq!(rtlabel_name2id(b"green"), 1, "the gap reused");
        assert_eq!(rtlabel_name2id(b"black"), 3, "after blue");
        for id in [1, 2, 3] {
            rtlabel_unref(id);
        }
        assert_eq!(rtlabel_name2id(b"x"), 1);
        rtlabel_unref(1);
    }

    #[test]
    fn route_caches_hit_miss_and_reset() {
        let _g = testroutes_setup();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut ro = Route::new();
        let misses = TEST_INET.misses.load(Ordering::Relaxed);
        assert_eq!(ro.cache4([10, 0, 2, 2], None, 0), Err(Errno::ESRCH));
        assert_eq!(TEST_INET.misses.load(Ordering::Relaxed), misses + 1);
        assert_eq!(ro.dst().bytes(), &sin([10, 0, 2, 2])[..]);
        // No route to it: still a miss.
        assert!(ro.mpath4([10, 0, 2, 2], None, 0, &net).is_none());

        let rt = entry(
            &sin([10, 0, 2, 0]),
            24,
            Some(&sin([10, 0, 2, 1])),
            RTF_UP,
            8,
        );
        rtable_insert(
            0,
            v(&sin([10, 0, 2, 0])),
            mask_of(AF_INET, 24).as_ref().map(SaCopy::view),
            None,
            8,
            &rt,
        )
        .expect("insert");
        assert!(ro.mpath4([10, 0, 2, 2], None, 0, &net).is_some());
        assert_eq!(PoolArc::count(&rt), 3, "the table's, the cache's and ours");
        let hits = TEST_INET.hits.load(Ordering::Relaxed);
        assert_eq!(ro.cache4([10, 0, 2, 2], None, 0), Ok(()));
        assert_eq!(TEST_INET.hits.load(Ordering::Relaxed), hits + 1);
        // Another destination, another table, a newer generation: misses, the route given
        // back.
        assert_eq!(ro.cache4([10, 0, 2, 3], None, 0), Err(Errno::ESRCH));
        assert_eq!(PoolArc::count(&rt), 2);
        assert!(ro.mpath4([10, 0, 2, 3], None, 0, &net).is_some());
        RTGENERATION.fetch_add(1, Ordering::Release);
        assert_eq!(ro.cache4([10, 0, 2, 3], None, 0), Err(Errno::ESRCH));
        assert!(ro.rt().is_none());

        // The IPv6 twin resets its destination on a miss.
        let mut a = [0u8; 16];
        a[0] = 0x20;
        a[15] = 1;
        assert_eq!(ro.cache6(&a, None, 0), Err(Errno::ESRCH));
        assert_eq!(ro.dst().bytes(), &sin6(a)[..]);
        assert_eq!(ro.ro_generation, RTGENERATION.load(Ordering::Relaxed));
        drop(net);
        drop(w);
        rtable_delete(
            0,
            v(&sin([10, 0, 2, 0])),
            mask_of(AF_INET, 24).as_ref().map(SaCopy::view),
            &rt,
        )
        .expect("delete");
        rt.rt_flags.clear(RTF_UP);
        grace();
    }

    #[test]
    fn requests_add_clone_and_delete_with_the_c_counts() {
        let _g = testroutes_setup();
        let (ifp, d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let ifa_refs = ifa.ifa_refcnt.read();

        // The connected route of the subnet, cloning, as in_ifinit asks.
        rt_ifa_add(
            ifa,
            RTF_CLONING | RTF_CONNECTED,
            v(&sin([10, 0, 2, 0])),
            0,
            &net,
        )
        .expect("the subnet's route");
        assert_eq!(d.adds.load(Ordering::Relaxed), 1);
        assert_eq!(ifa.ifa_refcnt.read(), ifa_refs + 1, "rt_ifa's reference");
        let net24 = rtable_lookup(
            0,
            v(&sin([10, 0, 2, 0])),
            mask_of(AF_INET, 24).as_ref().map(SaCopy::view),
            None,
            RTP_ANY,
        )
        .expect("the /24");
        assert_eq!(rt_refcnt(&net24) - 1, 1, "rmx_refcnt: our lookup");
        assert!(net24.rt_flags.isset(RTF_UP | RTF_CLONING));
        assert_eq!(net24.rt_priority.get(), RTP_CONNECTED);
        assert!(messages().contains(&RTM_ADD));

        // A host of the subnet resolved: a clone of the /24, its parent counted.
        let host = rtalloc(v(&sin([10, 0, 2, 2])), RT_RESOLVE, 0, &net).expect("the host");
        assert_eq!(host.rt_plen(), 32);
        assert!(host.rt_flags.isset(RTF_CLONED | RTF_HOST));
        assert!(!host.rt_flags.isset(RTF_CLONING));
        assert!(host.parent().is_some_and(|p| PoolArc::ptr_eq(&p, &net24)));
        assert_eq!(d.resolves.load(Ordering::Relaxed), 1);
        assert_eq!(
            rt_refcnt(&net24) - 1,
            2,
            "our lookup and the clone's parent"
        );
        assert_eq!(rt_refcnt(&host) - 1, 1, "our answer");
        assert_eq!(host.rt_use().load(Ordering::Relaxed), 1);
        assert!(host.with_gateway(|g| g.is_some_and(|g| g[1] == AF_LINK)));

        // A second resolve finds the clone itself.
        let again = rtalloc(v(&sin([10, 0, 2, 2])), RT_RESOLVE, 0, &net).expect("the host");
        assert!(PoolArc::ptr_eq(&again, &host));
        drop(again);

        // Deleting the cloning route flushes its clone.
        drop(net24);
        let mut info = RtAddrinfo::new();
        let dst = sin([10, 0, 2, 0]);
        let m24 = mask_of(AF_INET, 24).expect("a mask");
        info.rti_info[RTAX_DST] = Some(v(&dst));
        info.rti_info[RTAX_NETMASK] = Some(m24.view());
        let mut gone = None;
        rtrequest_delete(&mut info, RTP_CONNECTED, ifp, Some(&mut gone), 0, &net).expect("delete");
        let gone = gone.expect("the deleted route");
        assert!(!gone.rt_flags.isset(RTF_UP));
        assert!(!host.rt_flags.isset(RTF_UP), "the clone flushed");
        assert!(host.parent().is_none());
        assert_eq!(d.deletes.load(Ordering::Relaxed), 2);
        assert!(rtable_lookup(0, v(&sin([10, 0, 2, 2])), None, None, RTP_ANY).is_none());
        assert_eq!(RTTRASH.load(Ordering::Relaxed), 2);
        drop(net);
        drop(w);
        drop((host, gone));
        grace();
        assert_eq!(RTTRASH.load(Ordering::Relaxed), 0, "both freed");
        assert_eq!(
            ifa.ifa_refcnt.read(),
            ifa_refs,
            "the routes' address references"
        );
        assert!(rtable_get(0, AF_INET).is_some());
    }

    #[test]
    fn gateways_are_cached_and_checked() {
        let _g = testroutes_setup();
        let (ifp, _d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rt_ifa_add(
            ifa,
            RTF_CLONING | RTF_CONNECTED,
            v(&sin([10, 0, 2, 0])),
            0,
            &net,
        )
        .expect("subnet");

        // A default route through 10.0.2.2: its next hop the clone, cached.
        let mut info = RtAddrinfo::new();
        let d0 = sin([0; 4]);
        let m0 = mask_of(AF_INET, 0).expect("a mask");
        let gw = sin([10, 0, 2, 2]);
        info.rti_info[RTAX_DST] = Some(v(&d0));
        info.rti_info[RTAX_NETMASK] = Some(m0.view());
        info.rti_info[RTAX_GATEWAY] = Some(v(&gw));
        info.rti_ifa = Some(ifa);
        info.rti_flags = RTF_GATEWAY | RTF_STATIC | RTF_UP;
        let mut rt = None;
        rtrequest(RTM_ADD, &mut info, 0, Some(&mut rt), 0, &net).expect("the default route");
        let rt = rt.expect("the route");
        let nh = rt.gwroute().expect("its next hop");
        assert!(nh.rt_flags.isset(RTF_CACHED | RTF_CLONED));
        assert_eq!(nh.rt_cachecnt(), 1);
        assert!(rtisvalid(Some(&rt)));
        assert!(rt_getll(&rt, |ll| ll.is_some_and(|l| core::ptr::eq(l, &*nh))));
        assert_eq!(
            rt.rt_priority.get(),
            RTP_STATIC,
            "the interface's static priority"
        );
        assert!(rt.gateway_is(&gw));
        let found = rtable_match(0, v(&sin([8, 8, 8, 8])), None).expect("a match");
        assert!(PoolArc::ptr_eq(&found, &rt));
        drop(found);

        // The same route again: EEXIST.
        let mut dup = info;
        assert_eq!(
            rtrequest(RTM_ADD, &mut dup, 0, None, 0, &net),
            Err(Errno::EEXIST)
        );

        // A gateway that is no reachable next hop: ENETUNREACH for a route through a
        // gateway, ENOENT for nothing.
        let far = sin([192, 168, 9, 9]);
        assert_eq!(
            rt_setgate(&rt, v(&far), 0, &net),
            Err(Errno::ENETUNREACH),
            "through the default"
        );
        assert_eq!(rt_setgate(&rt, v(&gw), 0, &net), Ok(()));

        // The cache given back at the delete.
        let mut del = RtAddrinfo::new();
        del.rti_info[RTAX_DST] = Some(v(&d0));
        del.rti_info[RTAX_NETMASK] = Some(m0.view());
        rtrequest_delete(&mut del, RTP_STATIC, ifp, None, 0, &net).expect("delete");
        assert!(rt.gwroute().is_none());
        assert_eq!(nh.rt_cachecnt(), 0);
        assert!(!nh.rt_flags.isset(RTF_CACHED));
        drop(net);
        drop(w);
        drop((nh, rt));
        grace();
    }

    static TIMEOUTQ: RttimerQueue = RttimerQueue::new();
    static FUNCQ: RttimerQueue = RttimerQueue::new();
    static CALLED: AtomicU32 = AtomicU32::new(0);

    fn called(_rt: &PoolArc<Rtentry>, _id: u32) {
        CALLED.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn timers_are_added_replaced_flushed_and_removed() {
        let _g = testroutes_setup();
        rt_timer_queue_init(&TIMEOUTQ, 600, None);
        rt_timer_queue_init(&FUNCQ, 30, Some(called));
        let rt = entry(&sin([10, 0, 9, 9]), 32, None, RTF_UP | RTF_HOST, 8);
        let base = rt_refcnt(&rt);
        rt_timer_add(&rt, &TIMEOUTQ, 0).expect("a timer");
        rt_timer_add(&rt, &FUNCQ, 0).expect("a timer");
        assert_eq!(rt_timer_queue_count(&TIMEOUTQ), 1);
        assert_eq!(rt_refcnt(&rt), base, "the timers' handles are not counted");
        assert_eq!(PoolArc::count(&rt), base + 2);
        let e = rt_timer_get_expire(&rt);
        assert!(e > 0);
        // A second timer of the same queue replaces the first.
        rt_timer_add(&rt, &TIMEOUTQ, 0).expect("a timer");
        assert_eq!(rt_timer_queue_count(&TIMEOUTQ), 1);
        assert_eq!(PoolArc::count(&rt), base + 2);

        // Flushing a queue runs its function now.
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let before = CALLED.load(Ordering::Relaxed);
        rt_timer_queue_flush(&FUNCQ, &net);
        assert_eq!(CALLED.load(Ordering::Relaxed), before + 1);
        assert_eq!(rt_timer_queue_count(&FUNCQ), 0);
        assert_eq!(PoolArc::count(&rt), base + 1);
        drop(net);
        drop(w);

        // The route's timers removed with it.
        rt_timer_remove_all(&rt);
        assert_eq!(rt_timer_queue_count(&TIMEOUTQ), 0);
        assert_eq!(PoolArc::count(&rt), base);
        assert_eq!(rt_timer_get_expire(&rt), 0);
        rt.rt_flags.clear(RTF_UP);
    }

    static FIREDQ: RttimerQueue = RttimerQueue::new();

    #[test]
    fn a_flush_leaves_a_fired_timer_to_its_timeout() {
        let _g = testroutes_setup();
        rt_timer_queue_init(&FIREDQ, 30, Some(called));
        let rt = entry(&sin([10, 0, 9, 8]), 32, None, RTF_UP | RTF_HOST, 8);
        let base = PoolArc::count(&rt);
        rt_timer_add(&rt, &FIREDQ, 0).expect("a timer");
        let timers = RTTIMER_POOL.nout();
        // The wheel takes the timeout off to run it: a fired timer whose rt_timer_timer is
        // still to come (held back by the flush's net lock).
        let t = {
            let g = RTTIMER_MTX.lock();
            timer_arc(FIREDQ.rtq_head.first(&g).expect("the timer"))
        };
        assert!(t.rtt_timeout.del(), "it was pending");

        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let before = CALLED.load(Ordering::Relaxed);
        rt_timer_queue_flush(&FIREDQ, &net);
        assert_eq!(
            CALLED.load(Ordering::Relaxed),
            before + 1,
            "its work done by the flush"
        );
        assert_eq!(rt_timer_queue_count(&FIREDQ), 1, "left on its queue");
        assert_eq!(PoolArc::count(&rt), base, "its route given back");
        assert_eq!(rt_timer_get_expire(&rt), 0, "off its route");
        drop(net);
        drop(w);

        // Its timeout runs: off its queue, no second callout, the timer freed.
        rt_timer_timer(&PoolArc::erase(PoolArc::clone(&t)));
        drop(t);
        assert_eq!(CALLED.load(Ordering::Relaxed), before + 1);
        assert_eq!(rt_timer_queue_count(&FIREDQ), 0);
        assert_eq!(RTTIMER_POOL.nout(), timers - 1, "freed");
        rt.rt_flags.clear(RTF_UP);
    }

    #[test]
    fn the_link_state_brings_routes_down_and_up() {
        let _g = testroutes_setup();
        let (ifp, _d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rt_ifa_add(ifa, RTF_CONNECTED, v(&sin([10, 0, 2, 0])), 0, &net).expect("subnet");
        let rt = rtable_match(0, v(&sin([10, 0, 2, 7])), None).expect("the subnet");
        assert!(rt.rt_flags.isset(RTF_UP));
        let generation = RTGENERATION.load(Ordering::Relaxed);

        ifp.if_flags.set(IFF_BROADCAST);
        rt_if_track(ifp, &net).expect("down");
        assert!(!rt.rt_flags.isset(RTF_UP));
        assert_eq!(rt.rt_priority.get() & RTP_DOWN, RTP_DOWN);
        assert!(RTGENERATION.load(Ordering::Relaxed) > generation);

        ifp.if_flags.set(IFF_UP | IFF_BROADCAST);
        rt_if_track(ifp, &net).expect("up");
        assert!(rt.rt_flags.isset(RTF_UP));
        assert_eq!(rt.rt_priority.get() & RTP_DOWN, 0);
        drop(rt);

        rt_ifa_del(ifa, RTF_CONNECTED, v(&sin([10, 0, 2, 0])), 0, &net).expect("delete");
        assert!(rtable_match(0, v(&sin([10, 0, 2, 7])), None).is_none());
        let ms = messages();
        assert!(ms.contains(&RTM_DELETE));
        drop(net);
        drop(w);
        grace();
    }

    #[test]
    fn local_routes_of_an_address_and_their_purge() {
        let _g = testroutes_setup();
        let (_ifp, _d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rt_ifa_addlocal(ifa, &net).expect("the local route");
        let local = rtalloc(v(&sin([10, 0, 2, 15])), 0, 0, &net).expect("the local route");
        assert!(local.rt_flags.isset(RTF_LOCAL | RTF_HOST | RTF_LLINFO));
        assert_eq!(local.rt_priority.get(), RTP_LOCAL);
        assert!(messages().contains(&RTM_NEWADDR));
        drop(local);
        // Twice: one route.
        rt_ifa_addlocal(ifa, &net).expect("again");
        rt_ifa_add(ifa, RTF_CONNECTED, v(&sin([10, 0, 2, 0])), 0, &net).expect("subnet");
        rt_ifa_purge(ifa, &net);
        assert!(crate::net::rtable::rtable_empty(0));
        drop(net);
        drop(w);
        grace();
    }

    #[test]
    fn the_hash_mixes_destination_source_and_jitter() {
        let _g = testroutes_setup();
        let rt = entry(
            &sin([0; 4]),
            0,
            Some(&sin([10, 0, 0, 1])),
            RTF_UP | RTF_MPATH,
            8,
        );
        let d = sin([8, 8, 8, 8]);
        assert_eq!(rt_hash(&rt, v(&d), None), None, "no source");
        assert_eq!(rt_hash(&rt, v(&d), Some(&[1])), None, "multipath off");
        set_multipath(true);
        let h = rt_hash(&rt, v(&d), Some(&[1])).expect("a hash");
        assert!(h <= 0xffff);
        assert_eq!(rt_hash(&rt, v(&d), Some(&[1])), Some(h), "the same flow");
        let other: Vec<_> = (2..40u32)
            .filter_map(|s| rt_hash(&rt, v(&d), Some(&[s])))
            .collect();
        assert!(other.iter().any(|&x| x != h));
        rt.rt_flags.clear(RTF_MPATH);
        assert_eq!(
            rt_hash(&rt, v(&d), Some(&[1])),
            None,
            "not a multipath route"
        );
        set_multipath(false);
        rt.rt_flags.clear(RTF_UP);
    }

    #[test]
    fn if_output_ml_sends_until_the_first_error() {
        let _g = testroutes_setup();
        let (ifp, d, _ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let rt = entry(&sin([10, 0, 2, 0]), 24, None, RTF_UP, 8);
        let dst = sin([10, 0, 2, 2]);
        let list = |n: usize| {
            let mut ml = crate::net::mbuf::MbufList::new();
            for _ in 0..n {
                crate::net::mbuf::ml_enqueue(&mut ml, crate::net::if_::test_packet(0, 0));
            }
            ml
        };
        let mut ml = list(3);
        assert_eq!(
            crate::net::if_::if_output_ml(ifp, &mut ml, v(&dst), Some(&rt)),
            Ok(())
        );
        assert_eq!(d.outputs.load(Ordering::Relaxed), 3);
        let mut ml = list(3);
        d.fail_at.store(5, Ordering::Relaxed);
        assert_eq!(
            crate::net::if_::if_output_ml(ifp, &mut ml, v(&dst), Some(&rt)),
            Err(Errno::ENOBUFS)
        );
        assert_eq!(d.outputs.load(Ordering::Relaxed), 5, "stopped at the error");
        assert_eq!(crate::net::mbuf::ml_len(&ml), 0, "the rest freed");
        rt.rt_flags.clear(RTF_UP);
    }

    #[test]
    fn a_point_to_point_local_route_loses_its_link_layer() {
        let _g = testroutes_setup();
        let (ifp, _d, ifa) = iface([10, 9, 9, 1], [255, 255, 255, 255]);
        let (lo0, _ld, _loa) = iface([127, 0, 0, 1], [255, 0, 0, 0]);
        klock::kernel_lock();
        crate::net::rtable::rtable_l2set(0, 0, lo0.if_index.get());
        klock::kernel_unlock();
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let local = |flags| {
            let rt = entry(&sin([10, 9, 9, 1]), 32, None, flags, RTP_LOCAL);
            rt.rt_ifa.set(Some(ifaref(ifa)));
            rt
        };
        let rt = local(RTF_UP | RTF_LOCAL | RTF_LLINFO | RTF_HOST);
        crate::net::if_::p2p_rtrequest(ifp, RTM_DELETE, &rt, &net);
        assert!(rt.rt_flags.isset(RTF_LLINFO), "only RTM_ADD");
        crate::net::if_::p2p_rtrequest(ifp, RTM_ADD, &rt, &net);
        assert!(!rt.rt_flags.isset(RTF_LLINFO), "lo0 has an IPv4 address");
        let other = local(RTF_UP | RTF_LLINFO | RTF_HOST);
        crate::net::if_::p2p_rtrequest(ifp, RTM_ADD, &other, &net);
        assert!(other.rt_flags.isset(RTF_LLINFO), "not a local route");
        drop(net);
        drop(w);
        rt.rt_flags.clear(RTF_UP);
        other.rt_flags.clear(RTF_UP);
    }

    #[test]
    fn ddb_prints_a_route() {
        let _g = testroutes_setup();
        let rt = entry(
            &sin([10, 0, 2, 0]),
            24,
            Some(&sin([10, 0, 2, 1])),
            RTF_UP,
            8,
        );
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut s = std::string::String::new();
        db_show_rtentry(&mut s, &rt, &net).expect("printed");
        assert!(
            s.contains(" key=[16,2,0,0,10,0,2,0,0,0,0,0,0,0,0,0]\n"),
            "{s}"
        );
        assert!(s.contains(" plen=24"), "{s}");
        let mut n = std::string::String::new();
        db_print_sa(&mut n, None).expect("printed");
        assert_eq!(n, "[NULL]");
        drop(net);
        drop(w);
        rt.rt_flags.clear(RTF_UP);
    }
}
/* </TESTS> */
