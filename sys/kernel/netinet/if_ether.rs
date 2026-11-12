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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)if_ether.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)if_ether.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The Ethernet address resolution protocol, `netinet/if_ether.c`, and the kernel half of
//! `<netinet/if_ether.h>` it uses (the userland half, `struct ether_arp` and `struct
//! sockaddr_inarp` among it, is `hal::sys::netinet_if_ether`; `struct arpcom` and the
//! Ethernet layer are `kernel::net::if_ethersubr`).
//!
//! Upstream: sys/netinet/if_ether.h @ 3ce1f3f79392
//! Upstream: sys/netinet/if_ether.c @ 3ce1f3f79392
//! LZ: sys/netinet/if_ether.rs@3c62ede68802
//!
//! ARP keeps its cache in the routing table: a cloning route of a connected subnet makes a
//! host route per neighbour (`RTF_LLINFO`), whose gateway is a `sockaddr_dl` holding the
//! neighbour's hardware address and whose `rt_llinfo` is a `struct llinfo_arp` (the packets
//! held while the request is outstanding, the retry count). `arpresolve` answers from it or
//! sends a request, `in_arpinput` fills it from requests and replies (`arpcache`) and answers
//! requests for our addresses, and `arptimer` ages it. TODO in the C: add "inuse/lock" bit (or
//! ref. count) along with valid bit.
//!
//! Locks used to protect struct members in this file: \[a\] atomic operations, \[I\]
//! immutable after creation, \[K\] kernel lock, \[m\] arp mutex, needed when net lock is
//! shared, \[N\] net lock.
//!
//! ## Deviations
//! - `NCARP` is not configured (`carp_iamatch` for a request on a carp interface, the
//!   overwrite warning's carp exemption): a comment at each site, as LZ.
//! - `inet_ntop` (`netinet/inet_ntop.c`) is not ported: the log lines print IPv4 addresses
//!   with a dotted-quad `Display` adaptor, which is what it writes for `AF_INET` (LZ's).
//! - `NFSCLIENT` (the `revarp*` state and functions behind it) is feature `nfsclient`,
//!   forwarded by `bsd`'s of the same name.
//! - `arpresolve`'s `DIAGNOSTIC` print of an unresolved entry whose `rt_expire` is 0 is
//!   behind feature `diagnostic`, as LZ's.
//!
//! ## Redesign
//! - LZ's `struct llinfo_arp` (`Cell`s and a `ListEntry`, a `pool_get`'s raw item written in an
//!   `unsafe` block, freed by `pool_put` at its last `la_refcnt`) hung on `rt_llinfo` as a raw
//!   word, cast back at every reader (`rt_la`'s `unsafe`) -> a [`LlinfoArp`] of the "arp"
//!   [`BoxPool`] (112 bytes; LZ's 104, the C's 96: the kernel's `MbufQueue` is 56 with its
//!   unlocked length beside the list) owned by the route's word,
//!   `hal::link::TaggedLink<LlinfoArp>` (D-4; not `Parkable`: no lent record can be made a box
//!   by the park protocol, review 1's B1): the word
//!   lends the record only to a guard of [`ARP_MTX`] (the C's "rt_llinfo and RTF_LLINFO are
//!   protected by arp_mtx", now a type: `LlinfoArp`'s `Tagged::LOCK`), and gives the box back
//!   only to the whole guard borrowed mutably (`RTM_DELETE`). Its one owner is the word, so
//!   `la_refcnt` (the list's reference and the iterator's) goes: the list links the pinned
//!   record (`hal::pinlist::PinList`, `arp_list`), and `arptimer` holds no reference to a
//!   record across the mutex's release, only its route (below). The members are \[m\] as in
//!   the C: atomics written under the mutex (`la_refreshed`, `la_asked`), and `la_mq` the
//!   kernel's `MbufQueue` (its own mutex, a leaf, as the C's).
//! - `la_rt`, the C's uncounted back pointer, is a counted handle (`PoolArc<Rtentry>`, \[I\];
//!   `None` only for the iterator's marker): `arptimer` clones it under the mutex where the C
//!   `rtref`s it, and the route and its record keep each other until `RTM_DELETE` takes the
//!   record out of the route (every route deletion runs it: `rtrequest_delete`'s
//!   `if_rtrequest`), so the route outlives its record as the C assumes. The C's pointer
//!   counts nothing, so `rt_refcnt` (`rmx_refcnt`, which `route(8)` and `netstat -r` print)
//!   subtracts this handle while the route's word owns the record (review 1's B2).
//! - `arpiterator` (a `struct llinfo_arp_iterator` marker on `arptimer`'s stack, the current
//!   record held by `la_refcnt` while the mutex is released) -> `arptimer` walks with a whole
//!   [`LlinfoArp`] marker pinned on its stack (`la_rt` `None`, the C's `la_rt == NULL` test of
//!   a marker): the record after it is read and the marker steps over it
//!   (`PinList::step_over`, O(1), COLLECTIONS's row), the mutex is released with the route's
//!   handle alone, and the walk resumes from the marker, as the C's. LZ's static marker
//!   (only that timeout walks) is the C's stack one again.
//! - `arpresolve(ifp, rt, m, dst, desten)` takes the packet (`MbufBox`) and gives it back on
//!   success (`Err(EAGAIN)`: kept on the hold queue; any other error: freed), the destination
//!   a `SockaddrView`; the `Inet` bridge's leg and `bsd`'s `unsafe` around LZ's `unsafe fn`
//!   (decision 82's block) go: `ether_resolve` calls it directly. The link-level gateway is
//!   read once per call (`with_gateway`: length, family and address together), where LZ's
//!   helpers read the cell twice.
//! - The receive entries `arpinput` and `revarpinput` hand packets to `arpinq`, whose soft
//!   interrupt the glue's task queue runs (`schednetisr::<G>`): they are generic in `G`, and
//!   `arpinit::<G>` publishes them in [`ARPINPUT`] and [`REVARPINPUT`] for `ether_input`, which
//!   has no glue (`BPF_REVOKE`'s shape; docs/SCAFFOLDING.md, removed with `G` at N9).
//!   `arpintr` runs under `if_netisr`'s write witness of the net lock (`net`), which
//!   `arpcache`'s `NET_ASSERT_LOCKED_EXCLUSIVE` checks.
//! - `arpinq` is the kernel's `Niqueue` (`kernel::net::if_var`), `net.inet.ip.arpq`'s
//!   `sysctl_niq` the kernel's; `if_output_mq` over `la_mq` is `arpcache`'s own delist and
//!   `if_output_ml`.
//! - The ARP packet in an mbuf is read and written through `hal::pod` at offset 0 (the data
//!   has no alignment guarantee: a copy, 28 bytes, once per packet as LZ's); the Ethernet
//!   header `arprequest`/`arpreply` hand to the interface's output in a `pseudo_AF_HDRCMPLT`
//!   socket address is written into its `sa_data`, as in C.
//! - `revarpwhoarewe` sleeps on `revarp_myip`'s address through `hal::sleep` (no glue), as
//!   the C's `tsleep_nsec(&revarp_myip, PSOCK, "revarp", ...)`.
//!
//! **Lock and spl transitions** against the C: `arpresolve`'s hit takes none (`arp_mtx` once
//! when the entry nears expiry, as the C); its miss `arp_mtx` once and `la_mq`'s mutex once
//! inside it (the C's `mq_push`), the kernel lock C-style around an `RTF_REJECT` change, as
//! the C; `arp_rtrequest` `arp_mtx` once (`RTM_DELETE`: and `la_mq`'s once for the purge, as
//! the C's `mq_purge`); `arpinvalidate` the same; `arptimer` `arp_mtx` for the walk, dropped
//! around the net lock and `arptfree` for each expired entry, as the C. **One above the C:**
//! `arpcache` takes `arp_mtx` once to reach the record, which the C reads under the exclusive
//! net lock alone (its lifetime then rests on `RTM_DELETE` needing the net lock, which no
//! type here can see; an either-lock read of the slot, D-5's shape at N6.4c1, would pay it:
//! the N12 row). Its "back in queue" re-check of the C's `if_output_mq` takes `arp_mtx` again
//! only when the hold total moved during the output (the C's unlocked `mq_len` of a record the
//! exclusive net lock keeps).
//!
//! **Holders** (`refcnt_take`/`rele` of the C against the handles here): `la_refcnt`'s list
//! reference = the route word's box (given to it by `TaggedLink::set` in `arp_rtrequest`, taken
//! back by `TaggedLink::take` in `RTM_DELETE` and dropped there); the iterator's reference = none
//! (the walk holds the route, `la_rt`'s clone, across the mutex's release: the C's `rtref`);
//! `la_rt` = one route handle per record, dropped with it.

use core::mem::size_of;
use core::pin::{Pin, pin};
#[cfg(feature = "nfsclient")]
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::link::{StaticMutex, Tagged};
use hal::mutex::Mutexed;
use hal::netlock::{NETLOCK, NetHeld};
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pod;
use hal::pool::{BoxPool, PR_NOWAIT, PoolArc, Pooled};
use hal::rwlock::RW_WRITE;
use hal::sys::errno::Errno;
#[cfg(feature = "nfsclient")]
use hal::sys::net_ethertypes::ETHERTYPE_REVARP;
use hal::sys::net_ethertypes::{ETHERTYPE_ARP, ETHERTYPE_IP};
use hal::sys::net_if::{IFF_NOARP, IFF_STATICARP, SDL_DATA_OFFSET, SockaddrDl};
#[cfg(feature = "nfsclient")]
use hal::sys::net_if_arp::ARPOP_REVREQUEST;
use hal::sys::net_if_arp::{ARPHRD_ETHER, ARPOP_REPLY, ARPOP_REQUEST, ARPOP_REVREPLY, Arphdr};
use hal::sys::net_route::{
    RTF_ANNOUNCE, RTF_BROADCAST, RTF_CACHED, RTF_CLONING, RTF_GATEWAY, RTF_LLINFO, RTF_LOCAL,
    RTF_MPLS, RTF_MULTICAST, RTF_REJECT, RTF_STATIC, RTM_ADD, RTM_DELETE, RTM_INVALIDATE,
    RTM_RESOLVE,
};
use hal::sys::netinet_if_ether::{
    ETHER_ADDR_LEN, EtherArp, EtherHeader, RTF_PERMANENT_ARP, SIN_PROXY, SockaddrInarp,
    ether_is_broadcast, ether_is_multicast,
};
use hal::sys::netinet_in::{INADDR_ANY, InAddr, SockaddrIn};
use hal::sys::socket::{AF_INET, AF_LINK, pseudo_AF_HDRCMPLT};

use crate::assert::kassert;
use crate::log::{LOG_DEBUG, LOG_ERR, LOG_INFO, LOG_WARNING};
use crate::net::if_::{IfRef, ProtoFn, if_isconnected, if_output_ml, niq_enqueue};
use crate::net::if_ethersubr::{
    ETHERBROADCASTADDR, arpcom_of, ether_map_ip_multicast, ether_sprintf,
};
use crate::net::if_var::{AtomicCell, Ifnet, Netstack, Niqueue, niq_delist};
use crate::net::mbuf::{
    M_BCAST, M_DONTWAIT, M_MCAST, MT_DATA, MbufBox, MbufList, MbufQueue, m_align, m_gethdr,
    m_pullup, m_resethdr, ml_dequeue, ml_len, mq_delist, mq_len, mq_purge, mq_push,
};
use crate::net::netisr::NETISR_ARP;
use crate::net::proto::SockaddrView;
use crate::net::route::{RT_RESOLVE, Rtentry, rt_getll, rtalloc, rtdeletemsg, rtisvalid};
use crate::net::rtable::{rtable_iterate, rtable_l2};
use crate::net::rtsock::rtm_send;
use crate::netinet::in_var::ifatoia;
use crate::prf::{Str, log};
use crate::proc::ProcGlue;
use crate::tc::getuptime;
use crate::timeout::{KCLOCK_NONE, TIMEOUT_MPSAFE, TIMEOUT_PROC, Timeout};

/// `LA_HOLD_QUEUE`: packets held per unresolved entry.
pub const LA_HOLD_QUEUE: u32 = 10;
/// `LA_HOLD_TOTAL`: packets held by all entries.
pub const LA_HOLD_TOTAL: u32 = 100;

/// \[I\] `arp_maxtries`: arp requests before set to rejected.
const ARP_MAXTRIES: i32 = 5;

/// \[I\] `arpt_prune`: walk list every 5 minutes.
const ARPT_PRUNE: i32 = 5 * 60;

/// `revarpwhoarewe`'s wait for a reply: 500 ms a request (`MSEC_TO_NSEC(500)`).
#[cfg(feature = "nfsclient")]
const REVARP_WAIT_NSEC: u64 = 500 * 1_000_000;

/// `PSOCK` (`<sys/param.h>`): the priority of a socket's sleeps, `revarp`'s.
#[cfg(feature = "nfsclient")]
const PSOCK: i32 = 24;

/// `struct llinfo_arp`: the ARP state of a route (its `rt_llinfo`), an item of the pool
/// "arp". Locks: \[m\] `arp_mtx`, \[I\] immutable after creation. The marker `arptimer` walks
/// with is one with no route (the C's `struct llinfo_arp_iterator`, whose first two members
/// are these).
pub struct LlinfoArp {
    /// \[m\] `la_list`: global `arp_list`.
    la_list: PinEntry<LlinfoArp>,
    /// \[I\] `la_rt`: the route (a counted handle; `None` for an iterator marker).
    la_rt: Option<PoolArc<Rtentry>>,
    /// `la_mq`: packet hold queue (its own mutex, inside `arp_mtx`).
    la_mq: MbufQueue,
    /// \[m\] `la_refreshed`: when was refresh sent.
    la_refreshed: AtomicCell<i64>,
    /// \[m\] `la_asked`: number of queries sent.
    la_asked: AtomicCell<i32>,
}

impl LlinfoArp {
    /// A record of `rt` (`pool_get(&arp_pool, PR_NOWAIT | PR_ZERO)`, `refcnt_init`, `mq_init`
    /// and `la_rt`), or `arptimer`'s marker with none.
    fn new(rt: Option<PoolArc<Rtentry>>) -> Self {
        Self {
            la_list: PinEntry::new(),
            la_rt: rt,
            la_mq: MbufQueue::new(LA_HOLD_QUEUE, IPL_SOFTNET),
            la_refreshed: AtomicCell::new(),
            la_asked: AtomicCell::new(),
        }
    }

    /// `la_asked`.
    pub fn la_asked(&self) -> i32 {
        self.la_asked.get()
    }

    /// `mq_len(&la->la_mq)`: the packets held.
    pub fn held(&self) -> u32 {
        mq_len(&self.la_mq)
    }
}

impl Pooled for LlinfoArp {
    const POOL: &'static BoxPool<LlinfoArp> = &ARP_POOL;
}

impl Tagged for LlinfoArp {
    const LOCK: StaticMutex = StaticMutex::of(&ARP_MTX);
}

hal::pin_adapter!(
    /// `LIST_HEAD(, llinfo_arp)` through `la_list`: `arp_list`.
    pub LaList: LlinfoArp, la_list
);

/// A network-order IPv4 address printed as a dotted quad (what `inet_ntop(AF_INET, ...)`
/// writes; see the module's deviations).
struct InAddrFmt(InAddr);

impl core::fmt::Display for InAddrFmt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let b = self.0.s_addr.to_ne_bytes();
        write!(f, "{}.{}.{}.{}", b[0], b[1], b[2], b[3])
    }
}

/// \[a\] `arpt_keep`: once resolved, cache for 20 minutes (`net.inet.ip.arptimeout`).
pub static ARPT_KEEP: AtomicI32 = AtomicI32::new(20 * 60);
/// \[a\] `arpt_down`: once declared down, don't send for 20 secs (`net.inet.ip.arpdown`).
pub static ARPT_DOWN: AtomicI32 = AtomicI32::new(20);

/// `arpinq` (`net.inet.ip.arpq`).
pub static ARPINQ: Niqueue = Niqueue::new(50, NETISR_ARP);

/// `arp_mtx`: llinfo_arp live time, `rt_llinfo` and `RTF_LLINFO` are protected by it; its
/// guard lends a route's record (`TaggedLink::get`) and walks [`ARP_LIST`].
pub static ARP_MTX: Mutexed<()> = Mutexed::new(IPL_SOFTNET, ());

/// \[m\] `arp_list`: list of `llinfo_arp` structures.
static ARP_LIST: PinList<LaList> = PinList::new();

/// \[I\] `arp_pool`: pool for `llinfo_arp` structures.
static ARP_POOL: BoxPool<LlinfoArp> = BoxPool::new();

/// \[a\] `la_hold_total`: packets currently in the arp queue (`net.inet.ip.arpq.len`'s
/// neighbour, `IPCTL_ARPQUEUED`).
#[allow(non_upper_case_globals)] // the C name; `LA_HOLD_TOTAL` is the limit beside it
pub static la_hold_total: AtomicU32 = AtomicU32::new(0);

/// `arptimer_to`: `arpinit`'s timeout.
static ARPTIMER_TO: Timeout = Timeout::new_flags(
    arptimer,
    core::ptr::null_mut(),
    KCLOCK_NONE,
    TIMEOUT_PROC | TIMEOUT_MPSAFE,
);

/// `arpinput` for the glue `arpinit` was given: `ether_input`'s entry for `ETHERTYPE_ARP`
/// (unset: the frame is dropped, the C without `INET`).
pub static ARPINPUT: OnceStatic<ProtoFn> = OnceStatic::new();
/// `revarpinput` for that glue: `ether_input`'s entry for `ETHERTYPE_REVARP`.
pub static REVARPINPUT: OnceStatic<ProtoFn> = OnceStatic::new();

/// `revarp_myip`: the client address a RARP reply gave (`in_addr`, network order); its
/// address is `revarpwhoarewe`'s wait channel. NFSCLIENT.
#[cfg(feature = "nfsclient")]
pub static REVARP_MYIP: AtomicU32 = AtomicU32::new(0);
/// `revarp_srvip`: the address of the server that answered. NFSCLIENT.
#[cfg(feature = "nfsclient")]
pub static REVARP_SRVIP: AtomicU32 = AtomicU32::new(0);
/// `revarp_finished`: a reply has been taken. NFSCLIENT.
#[cfg(feature = "nfsclient")]
pub static REVARP_FINISHED: AtomicBool = AtomicBool::new(false);
/// `revarp_ifidx`: the interface a RARP request is out on (0: none); `if_detach` clears it
/// for the interface that goes. NFSCLIENT.
#[cfg(feature = "nfsclient")]
pub static REVARP_IFIDX: AtomicU32 = AtomicU32::new(0);

/// `LLADDR(satosdl(rt->rt_gateway))` read for `ETHER_ADDR_LEN` bytes, as the C's callers
/// read the address an ARP or ND route's link-level gateway holds (zeros past a short
/// gateway).
pub fn gw_lladdr(rt: &Rtentry) -> [u8; ETHER_ADDR_LEN] {
    rt.with_gateway(|g| g.map_or([0; ETHER_ADDR_LEN], sdl_lladdr))
}

/// The `ETHER_ADDR_LEN` bytes after the name of the `sockaddr_dl` whose bytes `g` holds
/// (zeros past its end).
fn sdl_lladdr(g: &[u8]) -> [u8; ETHER_ADDR_LEN] {
    let mut en = [0u8; ETHER_ADDR_LEN];
    let start = SDL_DATA_OFFSET + usize::from(g.get(5).copied().unwrap_or(0));
    if let Some(a) = g.get(start..) {
        let n = a.len().min(ETHER_ADDR_LEN);
        en[..n].copy_from_slice(&a[..n]);
    }
    en
}

/// `satosdl(rt->rt_gateway)->sdl_alen` and `sdl_family` of an ARP or ND route.
pub fn gw_alen_family(rt: &Rtentry) -> (u8, u8) {
    rt.with_gateway(|g| g.map_or((0, 0), |g| (g.get(SDL_ALEN).copied().unwrap_or(0), g[1])))
}

/// `rt->rt_gateway->sa_family` and `sa_len` (zeros without a gateway).
pub fn gw_family_len(rt: &Rtentry) -> (u8, usize) {
    rt.with_gateway(|g| g.map_or((0, 0), |g| (g[1], usize::from(g[0]))))
}

/// `satosdl(rt->rt_gateway)->sdl_index` of a link-level gateway (`None` for another).
pub fn gw_sdl_index(rt: &Rtentry) -> Option<u32> {
    rt.with_gateway(|g| {
        let g = g?;
        (g[1] == AF_LINK).then(|| {
            u32::from(u16::from_ne_bytes([
                g.get(SDL_INDEX).copied().unwrap_or(0),
                g.get(SDL_INDEX + 1).copied().unwrap_or(0),
            ]))
        })
    })
}

/// A link-layer route's `sockaddr_dl` gateway given the interface's type and index
/// (`arp_rtrequest`'s and `nd6_rtrequest`'s, which checked its size).
pub fn gw_set_type_index(rt: &Rtentry, ifp: &Ifnet) {
    rt.update_gateway(|g| {
        if let Some(g) = g {
            g[SDL_TYPE] = ifp.if_type.get();
            g[SDL_INDEX..SDL_INDEX + 2].copy_from_slice(&(ifp.if_index.get() as u16).to_ne_bytes());
        }
    });
}

/// A link-layer route's `sockaddr_dl` gateway given the address `lladdr` (`sdl_alen` set to
/// its length first, then `LLADDR(sdl)` written), or forgotten with an empty one.
pub fn gw_set_lladdr(rt: &Rtentry, lladdr: &[u8]) {
    rt.update_gateway(|g| {
        if let Some(g) = g {
            g[SDL_ALEN] = lladdr.len() as u8;
            if let Some(a) = SockaddrDl::lladdr_mut(g) {
                a.copy_from_slice(lladdr);
            }
        }
    });
}

/// `*satosdl(rt->rt_gateway)` of a link-level gateway, copied (zeros past a short one;
/// `None` for another family).
pub fn gw_sdl_copy(rt: &Rtentry) -> Option<SockaddrDl> {
    rt.with_gateway(|g| {
        let g = g.filter(|g| g[1] == AF_LINK)?;
        let mut d = [0u8; size_of::<SockaddrDl>()];
        let n = g.len().min(d.len());
        d[..n].copy_from_slice(&g[..n]);
        pod::at::<SockaddrDl>(&d, 0)
    })
}

/// The bytes of the interface's link-level address (`sdltosa(ifp->if_sadl)`), copied.
pub fn if_sadl_copy(ifp: &Ifnet) -> SockaddrDl {
    ifp.sadl().as_deref().copied().unwrap_or_default()
}

/// `LLADDR(ifp->if_sadl)` read for `ETHER_ADDR_LEN` bytes: the interface's hardware address.
fn if_lladdr(ifp: &Ifnet) -> [u8; ETHER_ADDR_LEN] {
    let sdl = if_sadl_copy(ifp);
    let start = usize::from(sdl.sdl_nlen);
    let mut en = [0u8; ETHER_ADDR_LEN];
    if let Some(a) = sdl.sdl_data.get(start..start + ETHER_ADDR_LEN) {
        en.copy_from_slice(a);
    }
    en
}

/// The offset of `sdl_index` in a `sockaddr_dl`.
const SDL_INDEX: usize = core::mem::offset_of!(SockaddrDl, sdl_index);
/// The offset of `sdl_type`.
const SDL_TYPE: usize = core::mem::offset_of!(SockaddrDl, sdl_type);
/// The offset of `sdl_alen`.
pub const SDL_ALEN: usize = core::mem::offset_of!(SockaddrDl, sdl_alen);

/// The `sin_addr` of the `sockaddr_in` whose bytes `sa` holds (0 for a shorter one).
fn sin_addr_of(sa: &[u8]) -> InAddr {
    let a = sa.get(4..8).map_or([0; 4], |b| [b[0], b[1], b[2], b[3]]);
    InAddr {
        s_addr: u32::from_ne_bytes(a),
    }
}

/// The IPv4 address of a route's interface address (`satosin(rt->rt_ifa->ifa_addr)`).
fn rt_ifa_sin(rt: &Rtentry) -> u32 {
    rt.ifa().map_or(0, |ifa| ifatoia(ifa).ia_addr.addr().s_addr)
}

/// `arptimer`: timeout routine. Age arp table entries periodically.
fn arptimer(_arg: *mut core::ffi::c_void) {
    let marker = pin!(LlinfoArp::new(None));
    let marker: Pin<&LlinfoArp> = marker.as_ref();

    let uptime = getuptime();
    let _ = Pin::static_ref(&ARPTIMER_TO).add_sec(ARPT_PRUNE);

    let mut g = ARP_MTX.lock();
    ARP_LIST.insert_head(&mut g, marker);
    loop {
        // The record after the marker (a marker of another walk has no route: stepped over).
        let rt = match ARP_LIST.next(&g, marker.get_ref()) {
            None => break,
            Some(la) => la.la_rt.clone(),
        };
        let _ = ARP_LIST.step_over(&mut g, marker.get_ref());
        let Some(rt) = rt else {
            continue;
        };
        let expire = rt.rt_expire().get();
        if expire != 0 && expire < uptime {
            // The route's handle is the C's timer reference (`rtref`).
            drop(g);
            {
                let w = NETLOCK.write();
                if let Some(net) = NetHeld::of(&w) {
                    arptfree(&rt, &net); // timer has expired; clear
                }
            }
            drop(rt);
            g = ARP_MTX.lock();
        }
    }
    ARP_LIST.remove(&mut g, marker.get_ref());
}

/// `arpinit()`: the entry pool, the aging timeout, and the receive entries of the glue `G`
/// for `ether_input` ([`ARPINPUT`], [`REVARPINPUT`]; set once, a second glue's ignored).
pub fn arpinit<G: ProcGlue>() {
    let _ = ARP_POOL.init(IPL_SOFTNET, 0, "arp");
    let _ = ARPINPUT.set(arpinput::<G> as ProtoFn);
    let _ = REVARPINPUT.set(revarpinput::<G> as ProtoFn);

    let _ = Pin::static_ref(&ARPTIMER_TO).add_sec(ARPT_PRUNE);
}

/// `arp_rtrequest(ifp, req, rt)`: the ARP side of a route change on an Ethernet interface: a
/// record for a new link-layer route, its removal, an invalidation. Under the net lock (`net`,
/// the C's `NET_ASSERT_LOCKED`).
pub fn arp_rtrequest(ifp: &'static Ifnet, req: u8, rt: &Rtentry, net: &NetHeld<'_>) {
    let _ = net;
    if rt
        .rt_flags
        .isset(RTF_GATEWAY | RTF_BROADCAST | RTF_MULTICAST | RTF_MPLS)
    {
        return;
    }

    let uptime = getuptime();
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
                // Announce a new entry if requested or warn the user if another station has
                // this IP address.
                if rt.rt_flags.isset(RTF_ANNOUNCE | RTF_LOCAL) {
                    // An ARP route's key is a `sockaddr_in` and its gateway a `sockaddr_dl`
                    // holding the interface's address.
                    let a = sin_addr_of(rt.dest()).s_addr;
                    arprequest(ifp, a, a, &gw_lladdr(rt));
                }
            }
            // FALLTHROUGH (RTM_RESOLVE)
            let (gfam, glen) = gw_family_len(rt);
            if gfam != AF_LINK || glen < size_of::<SockaddrDl>() {
                log(
                    LOG_DEBUG,
                    format_args!(
                        "arp_rtrequest: bad gateway value: {}\n",
                        Str(&ifp.if_xname.get())
                    ),
                );
                return;
            }
            gw_set_type_index(rt, ifp);
            // Case 2: This route may come from cloning, or a manual route add with a LL
            // address.
            // The record's counted handle of its route (`la_rt`); a route the routing table
            // gave is always an item of its recorded pool, so this answers; a value outside
            // it (none exists) would get no record, quietly, as the C has no such case.
            let Some(handle) = PoolArc::from_ref(rt) else {
                return;
            };
            let Some(la) = ARP_POOL.get(PR_NOWAIT, || LlinfoArp::new(Some(handle))) else {
                log(LOG_DEBUG, format_args!("arp_rtrequest: pool get failed\n"));
                return;
            };

            let mut g = ARP_MTX.lock();
            if rt.rt_llinfo.is_set() {
                // we lost the race, another thread has entered it
                drop(g);
                drop(la);
                return;
            }
            ARP_LIST.insert_head(&mut g, la.as_pin());
            if let Err(la) = rt.rt_llinfo.set(&g, la) {
                // a legacy owner stored meanwhile: not ours
                ARP_LIST.remove(&mut g, &la);
                drop(g);
                drop(la);
                return;
            }
            rt.rt_flags.set_bits(RTF_LLINFO);
            if !rt.rt_flags.isset(RTF_LOCAL) {
                rt.rt_expire().set(uptime);
            }
        }
        RTM_DELETE => {
            let mut g = ARP_MTX.lock();
            let Some(la) = rt.rt_llinfo.take(&mut g) else {
                // we lost the race, another thread has removed it
                return;
            };
            ARP_LIST.remove(&mut g, &la);
            rt.rt_flags.clear(RTF_LLINFO);
            la_hold_total.fetch_sub(mq_purge(&la.la_mq), Ordering::Relaxed);
            drop(g);
            // The last reference (the C's `refcnt_rele` and `pool_put`): the record and its
            // route handle go.
            drop(la);
        }
        RTM_INVALIDATE => {
            if !rt.rt_flags.isset(RTF_LOCAL) {
                arpinvalidate(rt);
            }
        }
        _ => {}
    }
}

/// An ARP packet `ea` in a new mbuf for `ifp`, `m_align`ed, broadcast or not.
fn arp_packet(ifp: &Ifnet, ea: &EtherArp) -> Option<MbufBox> {
    let mut m = m_gethdr(M_DONTWAIT, MT_DATA)?;
    let len = size_of::<EtherArp>();
    m.m_len().set(len as u32);
    m.m_pkthdr().len.set(len as i32);
    m.m_pkthdr().ph_rtableid.set(ifp.if_rdomain.get());
    m.m_pkthdr().pf.prio.set(ifp.if_llprio.get());
    m_align(&m, len as i32);
    let data = m.data_mut()?;
    if !pod::put(data, 0, ea) {
        return None;
    }
    Some(m)
}

/// `ifp->if_output(ifp, m, &sa, NULL)` of an ARP packet whose whole Ethernet header `eh` is
/// in the `pseudo_AF_HDRCMPLT` socket address's `sa_data`.
fn arp_output(ifp: &'static Ifnet, m: MbufBox, eh: &EtherHeader) {
    let mut sa = [0u8; 16];
    sa[0] = 16;
    sa[1] = pseudo_AF_HDRCMPLT;
    // `eh = (struct ether_header *)sa.sa_data`: 14 bytes, the data's size.
    let _ = pod::put(&mut sa, 2, eh);
    let Some(dst) = SockaddrView::new(&sa) else {
        return;
    };
    let mut ml = MbufList::new();
    crate::net::mbuf::ml_enqueue(&mut ml, m);
    let _ = if_output_ml(ifp, &mut ml, dst, None);
}

/// `arprequest(ifp, sip, tip, enaddr)`: broadcasts an ARP request for `tip` from `sip` and
/// `enaddr` (network-order addresses).
pub fn arprequest(ifp: &'static Ifnet, sip: u32, tip: u32, enaddr: &[u8; ETHER_ADDR_LEN]) {
    let ea = EtherArp {
        ea_hdr: Arphdr {
            ar_hrd: ARPHRD_ETHER.to_be(),
            ar_pro: ETHERTYPE_IP.to_be(),
            ar_hln: ETHER_ADDR_LEN as u8, // hardware address length
            ar_pln: 4,                    // protocol address length
            ar_op: ARPOP_REQUEST.to_be(),
        },
        arp_sha: *enaddr,
        arp_spa: sip.to_ne_bytes(),
        arp_tha: [0; ETHER_ADDR_LEN],
        arp_tpa: tip.to_ne_bytes(),
    };
    let Some(m) = arp_packet(ifp, &ea) else {
        return;
    };
    let eh = EtherHeader {
        ether_dhost: ETHERBROADCASTADDR,
        ether_shost: *enaddr,
        ether_type: ETHERTYPE_ARP.to_be(), // if_output will not swap
    };
    m.m_flags().set(m.flags() | M_BCAST);
    arp_output(ifp, m, &eh);
}

/// `arpreply(ifp, m, sip, eaddr, rdomain)`: turns request `m` into the reply that `sip` is at
/// `eaddr` and sends it.
pub fn arpreply(
    ifp: &'static Ifnet,
    mut m: MbufBox,
    sip: InAddr,
    eaddr: &[u8; ETHER_ADDR_LEN],
    rdomain: u32,
) {
    m_resethdr(&mut m);
    m.m_pkthdr().ph_rtableid.set(rdomain);

    let Some(mut ea) = pod::at::<EtherArp>(m.data(), 0) else {
        return;
    };
    ea.ea_hdr.ar_op = ARPOP_REPLY.to_be();
    ea.ea_hdr.ar_pro = ETHERTYPE_IP.to_be(); // let's be sure!

    // We're replying to a request.
    ea.arp_tha = ea.arp_sha;
    ea.arp_tpa = ea.arp_spa;

    ea.arp_sha = *eaddr;
    ea.arp_spa = sip.s_addr.to_ne_bytes();
    let Some(data) = m.data_mut() else {
        return;
    };
    let _ = pod::put(data, 0, &ea);

    let eh = EtherHeader {
        ether_dhost: ea.arp_tha,
        ether_shost: *eaddr,
        ether_type: ETHERTYPE_ARP.to_be(),
    };
    arp_output(ifp, m, &eh);
}

/// `arpresolve(ifp, rt, m, dst, desten)`: resolves an IP address into an ethernet address.
/// If success, `desten` is filled in and the packet comes back to be sent. If there is no
/// entry in arptab, set one up and broadcast a request for the IP address; hold onto this
/// mbuf and resend it once the address is finally resolved: `Err(EAGAIN)`, the packet taken
/// over here. Any other error indicates an error (and the packet was freed).
pub fn arpresolve(
    ifp: &'static Ifnet,
    rt0: Option<&Rtentry>,
    m: MbufBox,
    dst: SockaddrView<'_>,
    desten: &mut [u8; ETHER_ADDR_LEN],
) -> Result<MbufBox, Errno> {
    if dst.bytes().len() < size_of::<SockaddrIn>() {
        return Err(Errno::EINVAL);
    }
    let dstin = sin_addr_of(dst.bytes());
    if m.flags() & M_BCAST != 0 {
        // broadcast
        *desten = ETHERBROADCASTADDR;
        return Ok(m);
    }
    if m.flags() & M_MCAST != 0 {
        // multicast
        *desten = ether_map_ip_multicast(&dstin);
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
            return Err(if rt.is_some_and(|r| core::ptr::eq(r, rt0)) {
                Errno::EHOSTDOWN
            } else {
                Errno::EHOSTUNREACH
            });
        };
        arpresolve_ll(ifp, rt, m, dstin, desten, uptime)
    })
}

/// `arpresolve`'s body over the link-layer route `rt` (`rt_getll(rt0)`, not rejected).
fn arpresolve_ll(
    ifp: &'static Ifnet,
    rt: &Rtentry,
    m: MbufBox,
    dstin: InAddr,
    desten: &mut [u8; ETHER_ADDR_LEN],
    uptime: i64,
) -> Result<MbufBox, Errno> {
    if !rt.rt_flags.isset(RTF_LLINFO) {
        // A route's key in the inet table is a `sockaddr_in`.
        log(
            LOG_DEBUG,
            format_args!(
                "arpresolve: {}: route contains no arp information\n",
                InAddrFmt(sin_addr_of(rt.dest()))
            ),
        );
        return Err(Errno::EINVAL);
    }

    // An `RTF_LLINFO` route's gateway is a `sockaddr_dl` (`arp_rtrequest` checked it): its
    // length, family and address read at once.
    let (alen, family, en) = rt.with_gateway(|g| {
        g.map_or((0, 0, [0; ETHER_ADDR_LEN]), |g| {
            (g.get(SDL_ALEN).copied().unwrap_or(0), g[1], sdl_lladdr(g))
        })
    });
    if alen > 0 && usize::from(alen) != ETHER_ADDR_LEN {
        log(
            LOG_DEBUG,
            format_args!(
                "arpresolve: {}: incorrect arp information\n",
                InAddrFmt(dstin)
            ),
        );
        return Err(Errno::EINVAL);
    }

    // Check the address family and length is valid, the address is resolved; otherwise, try
    // to resolve.
    let expire = rt.rt_expire().get();
    if (expire == 0 || expire > uptime) && family == AF_LINK && alen != 0 {
        *desten = en;

        // refresh ARP entry when timeout gets close
        let mut refresh = false;
        if expire != 0 && expire - i64::from(ARPT_KEEP.load(Ordering::Relaxed) / 8) < uptime {
            let g = ARP_MTX.lock();
            if let Some(la) = rt.rt_llinfo.get(&g)
                && la.la_refreshed.get() + 30 < uptime
            {
                la.la_refreshed.set(uptime);
                refresh = true;
            }
        }
        if refresh {
            // A route's address is an `AF_INET` interface address.
            let sip = rt_ifa_sin(rt);
            arprequest(ifp, sip, dstin.s_addr, &arpcom_of(ifp).ac_enaddr.get());
        }
        return Ok(m);
    }

    if ifp.if_flags.get() & (IFF_NOARP | IFF_STATICARP) != 0 {
        return Err(Errno::EINVAL);
    }

    let mut refresh = false;
    // ~RTF_REJECT, RTF_REJECT or neither.
    let mut reject: Option<bool> = None;
    {
        let g = ARP_MTX.lock();
        let Some(la) = rt.rt_llinfo.get(&g) else {
            return Err(Errno::EINVAL);
        };

        // There is an arptab entry, but no ethernet address response yet. Insert mbuf in hold
        // queue if below limit. If above the limit free the queue without queuing the new
        // packet.
        if la_hold_total.fetch_add(1, Ordering::Relaxed) < LA_HOLD_TOTAL {
            if mq_push(&la.la_mq, m) {
                la_hold_total.fetch_sub(1, Ordering::Relaxed);
            }
        } else {
            la_hold_total.fetch_sub(mq_purge(&la.la_mq) + 1, Ordering::Relaxed);
            drop(m);
        }

        // Re-send the ARP request when appropriate.
        #[cfg(feature = "diagnostic")]
        if rt.rt_expire().get() == 0 {
            // This should never happen. (Should it? -gwr)
            crate::prf::printf(format_args!("arpresolve: unresolved and rt_expire == 0\n"));
            // Set expiration time to now (expired).
            rt.rt_expire().set(uptime);
        }
        if rt.rt_expire().get() != 0 {
            reject = Some(false);
            if la.la_asked.get() == 0 || rt.rt_expire().get() != uptime {
                rt.rt_expire().set(uptime);
                let asked = la.la_asked.get();
                la.la_asked.set(asked + 1);
                if asked < ARP_MAXTRIES {
                    refresh = true;
                } else {
                    reject = Some(true);
                    rt.rt_expire()
                        .set(rt.rt_expire().get() + i64::from(ARPT_DOWN.load(Ordering::Relaxed)));
                    la.la_asked.set(0);
                    la.la_refreshed.set(0);
                    la_hold_total.fetch_sub(mq_purge(&la.la_mq), Ordering::Relaxed);
                }
            }
        }
    }

    // The route flags are changed under the kernel lock.
    if reject == Some(true) && !rt.rt_flags.isset(RTF_REJECT) {
        hal::klock::kernel_lock();
        rt.rt_flags.set_bits(RTF_REJECT);
        hal::klock::kernel_unlock();
    }
    if reject == Some(false) && rt.rt_flags.isset(RTF_REJECT) {
        hal::klock::kernel_lock();
        rt.rt_flags.clear(RTF_REJECT);
        hal::klock::kernel_unlock();
    }
    if refresh {
        let sip = rt_ifa_sin(rt);
        arprequest(ifp, sip, dstin.s_addr, &arpcom_of(ifp).ac_enaddr.get());
    }
    Err(Errno::EAGAIN)
}

/// `arppullup(m)`: the common length and type checks of an ARP packet; `None` (freed) if
/// bad.
fn arppullup(m: MbufBox) -> Option<MbufBox> {
    #[cfg(feature = "diagnostic")]
    if m.pkthdr().is_none() {
        crate::assert::kpanic(format_args!("arp without packet header"));
    }

    let mut m = m;
    let mut len = size_of::<Arphdr>();
    if (m.m_len().get() as usize) < len {
        m = m_pullup(m, len as i32)?;
    }

    let ar = pod::at::<Arphdr>(m.data(), 0)?;
    if u16::from_be(ar.ar_hrd) != ARPHRD_ETHER
        || u16::from_be(ar.ar_pro) != ETHERTYPE_IP
        || usize::from(ar.ar_hln) != ETHER_ADDR_LEN
        || usize::from(ar.ar_pln) != size_of::<InAddr>()
    {
        return None;
    }

    len += 2 * (usize::from(ar.ar_hln) + usize::from(ar.ar_pln));
    if (m.m_len().get() as usize) < len {
        m = m_pullup(m, len as i32)?;
    }

    Some(m)
}

/// `arpinput(ifp, m, ns)`: common length and type checks are done here, then the packet is
/// queued for the protocol-specific routine.
pub fn arpinput<G: ProcGlue>(_ifp: &'static Ifnet, m: MbufBox, _ns: Option<&mut Netstack>) {
    let Some(m) = arppullup(m) else {
        return;
    };
    let _ = niq_enqueue::<G>(&ARPINQ, m);
}

/// `arpintr()`: the ARP soft interrupt: processes the queued packets, under `if_netisr`'s
/// exclusive holding of the net lock (`net`).
pub fn arpintr(net: &NetHeld<'_>) {
    let mut ml = niq_delist(&ARPINQ);

    while let Some(m) = ml_dequeue(&mut ml) {
        match IfRef::get(m.m_pkthdr().ph_ifidx.get()) {
            Some(ifp) => in_arpinput(ifp.ifp(), m, net),
            None => drop(m),
        }
    }
}

/// `in_arpinput(ifp, m)`: ARP for Internet protocols on Ethernet, RFC 826. In addition, a
/// sanity check is performed on the sender protocol address, to catch impersonators.
pub fn in_arpinput(ifp: &'static Ifnet, m: MbufBox, net: &NetHeld<'_>) {
    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());

    let Some(ea) = pod::at::<EtherArp>(m.data(), 0) else {
        return;
    };
    let op = u16::from_be(ea.arp_op());
    if op != ARPOP_REQUEST && op != ARPOP_REPLY {
        return;
    }

    let mut itaddr = InAddr {
        s_addr: u32::from_ne_bytes(ea.arp_tpa),
    };
    let isaddr = InAddr {
        s_addr: u32::from_ne_bytes(ea.arp_spa),
    };

    if ether_is_multicast(&ea.arp_sha) && ether_is_broadcast(&ea.arp_sha) {
        log(
            LOG_ERR,
            format_args!(
                "arp: ether address is broadcast for IP address {}!\n",
                InAddrFmt(isaddr)
            ),
        );
        return;
    }

    let ours = if_lladdr(ifp);
    if ea.arp_sha == ours {
        return; // it's from me, ignore it.
    }

    // Check target against our interface addresses.
    let sin = SockaddrIn {
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_family: AF_INET,
        sin_addr: itaddr,
        ..SockaddrIn::default()
    };
    let mut target = false;
    let b = sa16(&sin);
    if let Some(v) = SockaddrView::new(&b) {
        let r = rtalloc(v, 0, rdomain, net);
        if let Some(x) = r.as_deref()
            && rtisvalid(Some(x))
            && x.rt_flags.isset(RTF_LOCAL)
            && x.rt_ifidx.get() == ifp.if_index.get()
        {
            target = true;
        }
    }

    // NCARP > 0: carp_iamatch for a request on a carp interface; not configured.

    // Do we have an ARP cache for the sender? Create if we are target.
    let mut rt = arplookup(isaddr, target, false, rdomain, net);

    // Check sender against our interface addresses.
    if let Some(r) = rt.as_deref()
        && rtisvalid(Some(r))
        && r.rt_flags.isset(RTF_LOCAL)
        && r.rt_ifidx.get() == ifp.if_index.get()
        && isaddr.s_addr != INADDR_ANY
    {
        let s = ether_sprintf(&ea.arp_sha);
        log(
            LOG_ERR,
            format_args!(
                "duplicate IP address {} sent from ethernet address {}\n",
                InAddrFmt(isaddr),
                Str(&s)
            ),
        );
        itaddr = isaddr;
    } else if let Some(r) = rt.as_ref()
        && !arpcache(ifp, &ea, r, net)
    {
        return;
    }

    if op == ARPOP_REQUEST {
        let eaddr = if target {
            // We already have all info for the reply
            ours
        } else {
            drop(rt.take());
            rt = arplookup(itaddr, false, true, rdomain, net);
            // Protect from possible duplicates, only owner should respond
            let Some(r) = rt
                .as_deref()
                .filter(|r| r.rt_ifidx.get() == ifp.if_index.get())
            else {
                return;
            };
            // A proxy ARP route's gateway is a `sockaddr_dl` with an address.
            gw_lladdr(r)
        };
        arpreply(ifp, m, itaddr, &eaddr, rdomain);
    }
}

/// The bytes of a 16-byte socket address image (`sockaddr_in`, `sockaddr_inarp`), for the
/// route lookups.
fn sa16<T: hal::user::AbiPod>(sa: &T) -> [u8; 16] {
    let mut b = [0u8; 16];
    let _ = pod::put(&mut b, 0, sa);
    b
}

/// `arpcache(ifp, ea, rt)`: records the sender of `ea` in ARP route `rt` and sends the
/// packets held for it; `false` (the C's -1) when the entry may not be changed (the caller
/// drops the packet). Under the exclusive net lock (`NET_ASSERT_LOCKED_EXCLUSIVE`).
pub fn arpcache(
    ifp: &'static Ifnet,
    ea: &EtherArp,
    rt: &PoolArc<Rtentry>,
    net: &NetHeld<'_>,
) -> bool {
    let spa = InAddr {
        s_addr: u32::from_ne_bytes(ea.arp_spa),
    };
    let mut changed = false;

    kassert!(hal::netlock::status() == RW_WRITE);
    kassert!(rt.with_gateway(|g| g.is_some()));

    // This can happen if the entry has been deleted by another CPU after we found it.
    if !rt.rt_llinfo.is_set() {
        return true;
    }

    let uptime = getuptime();
    // An ARP route's gateway is a `sockaddr_dl` with room for an Ethernet address.
    let (alen, _) = gw_alen_family(rt);
    if alen > 0 {
        let cur = gw_lladdr(rt);
        let n = usize::from(alen).min(ETHER_ADDR_LEN);
        if ea.arp_sha[..n] != cur[..n] {
            if rt.rt_flags.isset(RTF_PERMANENT_ARP | RTF_LOCAL) {
                let s = ether_sprintf(&ea.arp_sha);
                log(
                    LOG_WARNING,
                    format_args!(
                        "arp: attempt to overwrite permanent entry for {} by {} on {}\n",
                        InAddrFmt(spa),
                        Str(&s),
                        Str(&ifp.if_xname.get())
                    ),
                );
                return false;
            } else if rt.rt_ifidx.get() != ifp.if_index.get() {
                // NCARP > 0: no warning for a carp interface; not configured.
                let Some(rifp) = IfRef::get(rt.rt_ifidx.get()) else {
                    return false;
                };
                let s = ether_sprintf(&ea.arp_sha);
                log(
                    LOG_WARNING,
                    format_args!(
                        "arp: attempt to overwrite entry for {} on {} by {} on {}\n",
                        InAddrFmt(spa),
                        Str(&rifp.if_xname.get()),
                        Str(&s),
                        Str(&ifp.if_xname.get())
                    ),
                );
                return false;
            } else {
                let s = ether_sprintf(&ea.arp_sha);
                log(
                    LOG_INFO,
                    format_args!(
                        "arp info overwritten for {} by {} on {}\n",
                        InAddrFmt(spa),
                        Str(&s),
                        Str(&ifp.if_xname.get())
                    ),
                );
                rt.rt_expire().set(1); // no longer static
            }
            changed = true;
        }
    } else if !if_isconnected(ifp, rt.rt_ifidx.get()) {
        let Some(rifp) = IfRef::get(rt.rt_ifidx.get()) else {
            return false;
        };
        let s = ether_sprintf(&ea.arp_sha);
        log(
            LOG_WARNING,
            format_args!(
                "arp: attempt to add entry for {} on {} by {} on {}\n",
                InAddrFmt(spa),
                Str(&rifp.if_xname.get()),
                Str(&s),
                Str(&ifp.if_xname.get())
            ),
        );
        return false;
    }
    // The gateway's address set: its length, then its bytes.
    gw_set_lladdr(rt, &ea.arp_sha);
    if rt.rt_expire().get() != 0 {
        rt.rt_expire()
            .set(uptime + i64::from(ARPT_KEEP.load(Ordering::Relaxed)));
    }
    rt.rt_flags.clear(RTF_REJECT);

    // The record, under `arp_mtx` (see the module's lock table): its question count read and
    // reset, its held packets taken.
    let (asked, mut ml) = {
        let g = ARP_MTX.lock();
        let Some(la) = rt.rt_llinfo.get(&g) else {
            // deleted by another CPU meanwhile, as above
            return true;
        };
        let asked = la.la_asked.get();
        la.la_asked.set(0);
        la.la_refreshed.set(0);
        (asked, mq_delist(&la.la_mq))
    };

    // Notify userland that an ARP resolution has been done.
    if asked != 0 || changed {
        rtm_send(rt, RTM_RESOLVE, 0, ifp.if_rdomain.get(), net);
    }

    // if_output_mq(ifp, &la->la_mq, &la_hold_total, rt_key(rt), rt)
    let len = ml_len(&ml);
    let before = la_hold_total.load(Ordering::Relaxed);
    let _ = if_output_ml(ifp, &mut ml, rt.dest_view(), Some(&**rt));
    // XXXSMP we also discard if other CPU enqueues: a packet back in the queue (the output
    // held it again) is dropped. Only a hold of the output moves the total meanwhile (the
    // exclusive net lock keeps every other `arpresolve` out), so the record is reached again
    // only then.
    let mut purged = 0;
    if la_hold_total.load(Ordering::Relaxed) != before {
        let g = ARP_MTX.lock();
        if let Some(la) = rt.rt_llinfo.get(&g)
            && mq_len(&la.la_mq) > 0
        {
            purged = mq_purge(&la.la_mq);
        }
    }
    la_hold_total.fetch_sub(len + purged, Ordering::Relaxed);

    true
}

/// `arpinvalidate(rt)`: forgets the hardware address of ARP route `rt` and its held packets.
pub fn arpinvalidate(rt: &Rtentry) {
    let g = ARP_MTX.lock();
    let Some(la) = rt.rt_llinfo.get(&g) else {
        return;
    };
    la_hold_total.fetch_sub(mq_purge(&la.la_mq), Ordering::Relaxed);
    // An ARP route's gateway is a `sockaddr_dl`: its address forgotten.
    rt.update_gateway(|g| {
        if let Some(g) = g {
            g[SDL_ALEN] = 0;
        }
    });
    la.la_asked.set(0);
}

/// `arptfree(rt)`: frees an arp entry, under the exclusive net lock.
pub fn arptfree(rt: &PoolArc<Rtentry>, net: &NetHeld<'_>) {
    kassert!(hal::netlock::status() == RW_WRITE);

    // might have been freed between leave arp_mtx and enter net lock
    if !rt.rt_flags.isset(RTF_LLINFO) {
        return;
    }

    kassert!(!rt.rt_flags.isset(RTF_LOCAL));
    arpinvalidate(rt);

    let Some(ifp) = IfRef::get(rt.rt_ifidx.get()) else {
        return;
    };
    if !rt.rt_flags.isset(RTF_STATIC | RTF_CACHED) {
        let _ = rtdeletemsg(rt, ifp.ifp(), ifp.if_rdomain.get(), net);
    }
}

/// `arplookup(inp, create, proxy, tableid)`: looks up (or, with `create`, enters) an address
/// in arptab; with `proxy`, the published (`RTF_ANNOUNCE`) entry. The route comes with a
/// reference.
pub fn arplookup(
    inp: InAddr,
    create: bool,
    proxy: bool,
    tableid: u32,
    net: &NetHeld<'_>,
) -> Option<PoolArc<Rtentry>> {
    let sin = SockaddrInarp {
        sin_len: size_of::<SockaddrInarp>() as u8,
        sin_family: AF_INET,
        sin_addr: inp,
        sin_other: if proxy { SIN_PROXY } else { 0 },
        ..SockaddrInarp::default()
    };
    let flags = if create { RT_RESOLVE } else { 0 };

    // A `sockaddr_inarp`, a `sockaddr_in` with more after the address.
    let b = sa16(&sin);
    let mut rt = rtalloc(SockaddrView::new(&b)?, flags, tableid, net).filter(|r| {
        rtisvalid(Some(r))
            && !r.rt_flags.isset(RTF_GATEWAY)
            && r.rt_flags.isset(RTF_LLINFO)
            && r.with_gateway(|g| g.is_some_and(|g| g[1] == AF_LINK))
    })?;

    if proxy && !rt.rt_flags.isset(RTF_ANNOUNCE) {
        loop {
            rt = rtable_iterate(rt)?;
            if rt.rt_flags.isset(RTF_ANNOUNCE) {
                break;
            }
        }
    }

    Some(rt)
}

/// `arpproxy(in, rtableid)`: whether we do proxy ARP for this address and we point to
/// ourselves.
pub fn arpproxy(in_: InAddr, rtableid: u32, net: &NetHeld<'_>) -> bool {
    let Some(rt) = arplookup(in_, false, true, rtableid, net).filter(|r| rtisvalid(Some(r))) else {
        return false;
    };

    // Check that arp information are correct.
    if usize::from(gw_alen_family(&rt).0) != ETHER_ADDR_LEN {
        return false;
    }

    let Some(ifp) = IfRef::get(rt.rt_ifidx.get()) else {
        return false;
    };

    gw_lladdr(&rt) == if_lladdr(&ifp)
}

/// `revarpinput(ifp, m, ns)`: called from Ethernet interrupt handlers when ether packet type
/// `ETHERTYPE_REVARP` is received. Common length and type checks are done here, then the
/// protocol-specific routine is called.
pub fn revarpinput<G: ProcGlue>(ifp: &'static Ifnet, m: MbufBox, _ns: Option<&mut Netstack>) {
    let Some(m) = arppullup(m) else {
        return;
    };
    in_revarpinput::<G>(ifp, m);
}

/// `in_revarpinput(ifp, m)`: RARP for Internet protocols on Ethernet. Algorithm is that given
/// in RFC 903. We are only using for bootstrap purposes to get an ip address for one of our
/// interfaces. Thus we support no user-interface. Since the contents of the RARP reply are
/// specific to the interface that sent the request, this code must ensure that they are
/// properly associated. Note: also supports ARP via RARP packets, per the RFC.
pub fn in_revarpinput<G: ProcGlue>(ifp: &'static Ifnet, m: MbufBox) {
    let Some(ar) = pod::at::<EtherArp>(m.data(), 0) else {
        return;
    };
    match u16::from_be(ar.arp_op()) {
        ARPOP_REQUEST | ARPOP_REPLY => {
            // per RFC
            let _ = niq_enqueue::<G>(&ARPINQ, m);
        }
        ARPOP_REVREPLY => {
            // NFSCLIENT: the reply to revarpwhoarewe; without it the reply is dropped.
            #[cfg(feature = "nfsclient")]
            revarp_reply(ifp, &m, &ar);
            #[cfg(not(feature = "nfsclient"))]
            let _ = ifp;
        }
        // ARPOP_REVREQUEST: handled by rarpd(8); default: dropped.
        _ => {}
    }
}

/// The `ARPOP_REVREPLY` case of `in_revarpinput` (`NFSCLIENT`): a reply to our request, if
/// it is on the interface that asked and for our hardware address.
#[cfg(feature = "nfsclient")]
fn revarp_reply(ifp: &Ifnet, m: &MbufBox, ar: &EtherArp) {
    let ifidx = REVARP_IFIDX.load(Ordering::Relaxed);
    if ifidx == 0 {
        return;
    }
    if ifidx != m.m_pkthdr().ph_ifidx.get() {
        // !same interface
        return;
    }
    if !REVARP_FINISHED.load(Ordering::Relaxed) {
        if ar.arp_tha != if_lladdr(ifp) {
            return;
        }
        REVARP_SRVIP.store(u32::from_ne_bytes(ar.arp_spa), Ordering::Relaxed);
        REVARP_MYIP.store(u32::from_ne_bytes(ar.arp_tpa), Ordering::Relaxed);
        REVARP_FINISHED.store(true, Ordering::Relaxed);
    }
    // wake: Do wakeup every time in case it was missed.
    hal::sleep::wakeup(&REVARP_MYIP);
}

/// `revarprequest(ifp)`: send a RARP request for the ip address of the specified interface.
/// The request should be RFC 903-compliant.
#[cfg(feature = "nfsclient")]
pub fn revarprequest(ifp: &'static Ifnet) {
    let enaddr = arpcom_of(ifp).ac_enaddr.get();
    let ea = EtherArp {
        ea_hdr: Arphdr {
            ar_hrd: ARPHRD_ETHER.to_be(),
            ar_pro: ETHERTYPE_IP.to_be(),
            ar_hln: ETHER_ADDR_LEN as u8, // hardware address length
            ar_pln: 4,                    // protocol address length
            ar_op: ARPOP_REVREQUEST.to_be(),
        },
        arp_sha: enaddr,
        arp_spa: [0; 4],
        arp_tha: enaddr,
        arp_tpa: [0; 4],
    };
    let Some(m) = arp_packet(ifp, &ea) else {
        return;
    };
    let eh = EtherHeader {
        ether_dhost: ETHERBROADCASTADDR,
        ether_shost: enaddr,
        ether_type: ETHERTYPE_REVARP.to_be(), // if_output will not swap
    };
    m.m_flags().set(m.flags() | M_BCAST);
    arp_output(ifp, m, &eh);
}

/// `revarpwhoarewe(ifp, serv_in, clnt_in)`: RARP for the ip address of the specified
/// interface, but also save the ip address of the server that sent the answer. Timeout if
/// no response is received. Returns the server's and the client's addresses.
#[cfg(feature = "nfsclient")]
pub fn revarpwhoarewe(ifp: &'static Ifnet) -> Result<(InAddr, InAddr), Errno> {
    let mut count = 20;

    if REVARP_FINISHED.load(Ordering::Relaxed) {
        return Err(Errno::EIO);
    }

    REVARP_IFIDX.store(ifp.if_index.get(), Ordering::Relaxed);
    while count > 0 {
        count -= 1;
        revarprequest(ifp);
        let result = hal::sleep::tsleep_nsec(&REVARP_MYIP, PSOCK, "revarp", REVARP_WAIT_NSEC);
        if result != Err(Errno::EWOULDBLOCK) {
            break;
        }
    }
    REVARP_IFIDX.store(0, Ordering::Relaxed);
    if !REVARP_FINISHED.load(Ordering::Relaxed) {
        return Err(Errno::ENETUNREACH);
    }

    let serv = InAddr {
        s_addr: REVARP_SRVIP.load(Ordering::Relaxed),
    };
    let clnt = InAddr {
        s_addr: REVARP_MYIP.load(Ordering::Relaxed),
    };
    Ok((serv, clnt))
}

/// `revarpwhoami(in, ifp)`: for compatibility: only saves the interface address.
#[cfg(feature = "nfsclient")]
pub fn revarpwhoami(ifp: &'static Ifnet) -> Result<InAddr, Errno> {
    let (_server, clnt) = revarpwhoarewe(ifp)?;
    Ok(clnt)
}

/// `if_detach`'s `NETHER > 0 && NFSCLIENT` block: the reverse ARP interface forgotten
/// (`revarp_ifidx = 0`) when it is `ifp`.
pub fn revarp_ifdetach(ifp: &Ifnet) {
    #[cfg(feature = "nfsclient")]
    if ifp.if_index.get() == REVARP_IFIDX.load(Ordering::Relaxed) {
        REVARP_IFIDX.store(0, Ordering::Relaxed);
    }
    let _ = ifp;
}

/// The host tests' reset of ARP's state (testing.md: a global that holds kernel memory):
/// the records on `arp_list` unlinked (their memory, owned by routes of an earlier test's
/// tables, never freed), the queued packets and the hold total forgotten, the revarp state
/// cleared.
#[cfg(any(test, feature = "doubles"))]
pub fn arp_test_reset() {
    {
        let mut g = ARP_MTX.lock();
        while ARP_LIST.remove_first(&mut g) {}
    }
    core::mem::forget(niq_delist(&ARPINQ));
    la_hold_total.store(0, Ordering::Relaxed);
    #[cfg(feature = "nfsclient")]
    {
        REVARP_IFIDX.store(0, Ordering::Relaxed);
        REVARP_FINISHED.store(false, Ordering::Relaxed);
        REVARP_MYIP.store(0, Ordering::Relaxed);
        REVARP_SRVIP.store(0, Ordering::Relaxed);
    }
}

const _: () = {
    assert!(size_of::<EtherArp>() == 28);
    assert!(size_of::<SockaddrInarp>() == 16);
    assert!(ARP_MAXTRIES > 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of ARP over a test Ethernet interface (an `Arpcom` with a driver that keeps
    // what it is given on its send queue) with 10.0.2.15/24 configured through `in_ioctl`:
    // LZ's two (an entry made, resolved, answered and aged; the RARP client), and the brief's
    // (a miss held and flushed by the reply, the hold queue's and the hold total's bounds,
    // `arp_maxtries` rejecting the entry for `arpt_down`, proxy ARP, `arptimer`'s walk past
    // entries it removes, the record's size).

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::sys::net_if::{
        IFF_BROADCAST, IFF_MULTICAST, IFF_RUNNING, IFF_SIMPLEX, IFF_UP, IFNAMSIZ, IFXF_MPSAFE,
    };
    use hal::sys::net_route::{RTAX_DST, RTAX_GATEWAY, RTF_HOST, RTF_UP, RTP_STATIC};
    use hal::sys::sockio::SIOCAIFADDR;

    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::net::if_::{TestDriver, if_attach};
    use crate::net::if_ethersubr::{Arpcom, ether_ifattach};
    use crate::net::ifq::ifq_dequeue;
    use crate::net::mbuf::{M_DONTWAIT, m_copydata, m_devget};
    use crate::net::route::tests::{sin, testroutes_setup};
    use crate::net::route::{RtAddrinfo, rtrequest};
    use crate::tc::TIME_UPTIME;

    const OURS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    const PEER: [u8; 6] = [0x52, 0x54, 0x00, 0xaa, 0xbb, 0xcc];
    const ADDR: [u8; 4] = [10, 0, 2, 15];
    const GATEWAY: [u8; 4] = [10, 0, 2, 2];

    /// The routing tests' fixture and ARP's entries for the test glue; the uptime past 0 (a
    /// route cloned at 0 would read as a permanent entry).
    fn setup() -> MutexGuard<'static, ()> {
        let g = testroutes_setup();
        arp_test_reset();
        arpinit::<TestGlue>();
        TIME_UPTIME.fetch_max(1000, Ordering::Relaxed);
        g
    }

    /// An Ethernet interface `name` (a zeroed `struct arpcom` with a test driver that leaves
    /// its frames queued), up and running, with `ADDR`/24 (its local route's announcement
    /// taken off the queue and returned).
    fn ether(name: &[u8]) -> (&'static Ifnet, Vec<Vec<u8>>) {
        ether_at(name, ADDR)
    }

    /// [`ether`] with `addr`/24.
    fn ether_at(name: &[u8], addr: [u8; 4]) -> (&'static Ifnet, Vec<Vec<u8>>) {
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

        // ifconfig <name> inet 10.0.2.15/24
        let mut req = [0u8; 64];
        req[16..18].copy_from_slice(&[16, AF_INET]);
        req[20..24].copy_from_slice(&addr);
        req[48..50].copy_from_slice(&[16, AF_INET]);
        req[52..56].copy_from_slice(&[255, 255, 255, 0]);
        crate::netinet::in_::in_ioctl(SIOCAIFADDR, &mut req, Some(ifp), true).expect("the address");
        (ifp, sent(ifp))
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

    /// An ARP packet of `op` from `sha`/`spa` to `tha`/`tpa`, as bytes.
    fn arp_bytes(op: u16, sha: [u8; 6], spa: [u8; 4], tha: [u8; 6], tpa: [u8; 4]) -> [u8; 28] {
        let ea = EtherArp {
            ea_hdr: Arphdr {
                ar_hrd: ARPHRD_ETHER.to_be(),
                ar_pro: ETHERTYPE_IP.to_be(),
                ar_hln: 6,
                ar_pln: 4,
                ar_op: op.to_be(),
            },
            arp_sha: sha,
            arp_spa: spa,
            arp_tha: tha,
            arp_tpa: tpa,
        };
        sa16_28(&ea)
    }

    fn sa16_28(ea: &EtherArp) -> [u8; 28] {
        let mut b = [0u8; 28];
        assert!(pod::put(&mut b, 0, ea));
        b
    }

    /// `ARP` packet bytes received on `ifp` (the Ethernet header trimmed, as `ether_input`
    /// hands them on) processed as `arpintr` does, under the exclusive net lock.
    fn receive(ifp: &'static Ifnet, pkt: &[u8]) {
        let m = m_devget(pkt, 0).expect("an mbuf");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        let m = arppullup(m).expect("a valid ARP packet");
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        in_arpinput(ifp, m, &net);
    }

    /// The route to `a` in table 0, cloned from the subnet's cloning route with `resolve`.
    fn route(a: [u8; 4], resolve: bool) -> Option<PoolArc<Rtentry>> {
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let s = sin(a);
        let flags = if resolve { RT_RESOLVE } else { 0 };
        rtalloc(SockaddrView::new(&s).expect("a view"), flags, 0, &net)
    }

    /// A 46-byte IPv4 packet for `arpresolve`.
    fn packet() -> MbufBox {
        let m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(46);
        m.m_pkthdr().len.set(46);
        m
    }

    /// `arpresolve` of a packet to `a` through `rt`, under the shared net lock.
    fn resolve(ifp: &'static Ifnet, rt: &Rtentry, a: [u8; 4]) -> Result<[u8; 6], Errno> {
        let s = sin(a);
        let mut desten = [0u8; 6];
        let w = NETLOCK.read();
        let r = arpresolve(
            ifp,
            Some(rt),
            packet(),
            SockaddrView::new(&s).expect("a view"),
            &mut desten,
        );
        drop(w);
        r.map(|m| {
            drop(m);
            desten
        })
    }

    /// The record of `rt`: `(held, la_asked)`, `None` without one.
    fn record(rt: &Rtentry) -> Option<(u32, i32)> {
        let g = ARP_MTX.lock();
        rt.rt_llinfo.get(&g).map(|la| (la.held(), la.la_asked()))
    }

    /// `TIME_UPTIME` moved by `d` seconds.
    fn later(d: i64) {
        TIME_UPTIME.fetch_add(d, Ordering::Relaxed);
    }

    #[test]
    fn an_arp_entry_is_made_resolved_answered_and_aged_out() {
        let _g = setup();
        let (ifp, announced) = ether(b"art0");
        // The local route announced our address (a gratuitous request, sender = target).
        assert!(announced.iter().any(|f| {
            f[0..6] == [0xff; 6]
                && f[12..14] == ETHERTYPE_ARP.to_be_bytes()
                && f[28..32] == ADDR
                && f[38..42] == ADDR
        }));

        // The gateway asks for our address: we learn its, and answer, through ether_input.
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0xff; 6]);
        frame.extend_from_slice(&PEER);
        frame.extend_from_slice(&ETHERTYPE_ARP.to_be_bytes());
        frame.extend_from_slice(&arp_bytes(ARPOP_REQUEST, PEER, GATEWAY, [0; 6], ADDR));
        let before = crate::net::if_var::niq_len(&ARPINQ);
        // (`if_input` marks a frame with its interface before the link layer sees it.)
        let m = m_devget(&frame, 0).expect("a frame");
        m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
        crate::net::if_ethersubr::ether_input(ifp, m, None);
        assert_eq!(
            crate::net::if_var::niq_len(&ARPINQ),
            before + 1,
            "on arpinq"
        );
        {
            let w = NETLOCK.write();
            arpintr(&NetHeld::of(&w).expect("the net lock"));
        }
        let reply = sent(ifp);
        assert_eq!(reply.len(), 1);
        let b = &reply[0];
        assert_eq!(&b[0..6], &PEER, "to the asker");
        assert_eq!(&b[20..22], &ARPOP_REPLY.to_be_bytes());
        assert_eq!(&b[22..28], &OURS);
        assert_eq!(&b[28..32], &ADDR);
        assert_eq!(&b[32..38], &PEER);
        assert_eq!(&b[38..42], &GATEWAY);

        let rt = route(GATEWAY, false).expect("a route");
        assert!(rt.rt_flags.isset(RTF_LLINFO), "a cloned ARP entry");
        assert_eq!(gw_alen_family(&rt), (6, AF_LINK));
        assert_eq!(gw_lladdr(&rt), PEER, "resolved to the gateway's address");
        let expire = rt.rt_expire().get();
        assert_eq!(
            expire,
            getuptime() + i64::from(ARPT_KEEP.load(Ordering::Relaxed))
        );
        assert_eq!(resolve(ifp, &rt, GATEWAY), Ok(PEER), "a hit");
        drop(rt);

        // Past arpt_keep, arptimer removes the entry: the subnet's cloning route is left.
        let now = getuptime();
        TIME_UPTIME.store(expire + 1, Ordering::Relaxed);
        arptimer(core::ptr::null_mut());
        TIME_UPTIME.store(now, Ordering::Relaxed);
        let rt = route(GATEWAY, false).expect("the subnet's route");
        assert!(!rt.rt_flags.isset(RTF_LLINFO), "the entry is gone");
        assert!(rt.rt_flags.isset(RTF_CLONING));
    }

    #[test]
    fn a_miss_holds_the_packets_and_the_reply_sends_them() {
        let _g = setup();
        let (ifp, _) = ether(b"art1");
        const HOST: [u8; 4] = [10, 0, 2, 99];
        let rt = route(HOST, true).expect("a cloned route");
        assert!(rt.rt_flags.isset(RTF_LLINFO) && record(&rt) == Some((0, 0)));

        // The first packet is held and a request broadcast.
        assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
        let req = sent(ifp);
        assert_eq!(req.len(), 1);
        assert_eq!(&req[0][0..6], &[0xff; 6]);
        assert_eq!(&req[0][20..22], &ARPOP_REQUEST.to_be_bytes());
        assert_eq!(&req[0][28..32], &ADDR, "from our address");
        assert_eq!(&req[0][38..42], &HOST);
        // Eleven more within the same second: no new request, the queue keeps the last 10.
        for _ in 0..11 {
            assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
        }
        assert!(sent(ifp).is_empty(), "asked once a second");
        assert_eq!(record(&rt), Some((LA_HOLD_QUEUE, 1)));
        assert_eq!(la_hold_total.load(Ordering::Relaxed), LA_HOLD_QUEUE);

        // The reply resolves it: the held packets go out to the host, the count drops.
        let peer2 = [0x52, 0x54, 0x00, 0x00, 0x00, 0x99];
        receive(ifp, &arp_bytes(ARPOP_REPLY, peer2, HOST, OURS, ADDR));
        let out = sent(ifp);
        assert_eq!(out.len(), LA_HOLD_QUEUE as usize);
        assert!(
            out.iter()
                .all(|f| f[0..6] == peer2 && f[12..14] == ETHERTYPE_IP.to_be_bytes())
        );
        assert_eq!(record(&rt), Some((0, 0)));
        assert_eq!(la_hold_total.load(Ordering::Relaxed), 0);
        assert_eq!(resolve(ifp, &rt, HOST), Ok(peer2));

        // An invalidation forgets the address; the entry asks again.
        arpinvalidate(&rt);
        assert_eq!(gw_alen_family(&rt).0, 0);
        assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
        assert_eq!(sent(ifp).len(), 1, "asked again");

        // Above LA_HOLD_TOTAL every queue is purged and the packet dropped.
        la_hold_total.store(LA_HOLD_TOTAL, Ordering::Relaxed);
        assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
        assert_eq!(record(&rt).map(|r| r.0), Some(0));
        assert_eq!(la_hold_total.load(Ordering::Relaxed), LA_HOLD_TOTAL - 1);
        la_hold_total.store(0, Ordering::Relaxed);
    }

    #[test]
    fn unanswered_requests_reject_the_entry_for_arpt_down() {
        let _g = setup();
        let (ifp, _) = ether(b"art2");
        const HOST: [u8; 4] = [10, 0, 2, 77];
        let rt = route(HOST, true).expect("a cloned route");
        let now = getuptime();
        // One request a second, arp_maxtries of them.
        for i in 0..ARP_MAXTRIES {
            later(1);
            assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
            assert_eq!(record(&rt).map(|r| r.1), Some(i + 1));
        }
        assert_eq!(sent(ifp).len(), ARP_MAXTRIES as usize);
        // The next one gives up: rejected for arpt_down, the queue purged, no request.
        later(1);
        assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
        assert!(rt.rt_flags.isset(RTF_REJECT));
        assert_eq!(record(&rt), Some((0, 0)));
        assert!(sent(ifp).is_empty());
        assert_eq!(
            rt.rt_expire().get(),
            getuptime() + i64::from(ARPT_DOWN.load(Ordering::Relaxed))
        );
        // Meanwhile the route answers "host down" (the route is the link-level one).
        assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EHOSTDOWN));
        // Past arpt_down it asks again and the rejection is lifted.
        later(i64::from(ARPT_DOWN.load(Ordering::Relaxed)) + 1);
        assert_eq!(resolve(ifp, &rt, HOST), Err(Errno::EAGAIN));
        assert!(!rt.rt_flags.isset(RTF_REJECT));
        assert_eq!(sent(ifp).len(), 1);
        TIME_UPTIME.store(now, Ordering::Relaxed);
        // The held packet given back (the mbuf tests count every one out).
        arpinvalidate(&rt);
        assert_eq!(la_hold_total.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_published_entry_is_answered_for() {
        let _g = setup();
        let (ifp, _) = ether(b"art3");
        const PUB: [u8; 4] = [10, 0, 2, 50];
        // arp -s 10.0.2.50 <our address> pub
        let dst = sin(PUB);
        let mut gw = [0u8; 32];
        gw[0] = 32;
        gw[1] = AF_LINK;
        gw[2..4].copy_from_slice(&(ifp.if_index.get() as u16).to_ne_bytes());
        gw[4] = ifp.if_type.get();
        gw[6] = 6;
        gw[8..14].copy_from_slice(&OURS);
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = SockaddrView::new(&dst);
        info.rti_info[RTAX_GATEWAY] = SockaddrView::new(&gw);
        info.rti_flags = RTF_HOST | RTF_LLINFO | RTF_STATIC | RTF_ANNOUNCE | RTF_UP;
        info.rti_ifa = route(ADDR, false).and_then(|r| r.ifa());
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            rtrequest(RTM_ADD, &mut info, RTP_STATIC, None, 0, &net).expect("the entry");
            assert!(arpproxy(
                InAddr {
                    s_addr: u32::from_ne_bytes(PUB)
                },
                0,
                &net
            ));
            assert!(!arpproxy(
                InAddr {
                    s_addr: u32::from_ne_bytes(GATEWAY)
                },
                0,
                &net
            ));
        }
        let _ = sent(ifp);
        // A request for it is answered with our address.
        receive(ifp, &arp_bytes(ARPOP_REQUEST, PEER, GATEWAY, [0; 6], PUB));
        let reply = sent(ifp);
        assert_eq!(reply.len(), 1);
        assert_eq!(&reply[0][20..22], &ARPOP_REPLY.to_be_bytes());
        assert_eq!(&reply[0][22..28], &OURS);
        assert_eq!(&reply[0][28..32], &PUB);
        assert_eq!(&reply[0][32..38], &PEER);
    }

    #[test]
    fn arptimer_walks_past_the_entries_it_removes() {
        let _g = setup();
        let (ifp, _) = ether(b"art4");
        let hosts = [[10, 0, 2, 31], [10, 0, 2, 32], [10, 0, 2, 33]];
        for h in hosts {
            let rt = route(h, true).expect("a cloned route");
            assert_eq!(resolve(ifp, &rt, h), Err(Errno::EAGAIN));
            rt.rt_expire().set(getuptime() + 10);
        }
        // The middle one stays.
        let keep = route(hosts[1], false).expect("the entry");
        keep.rt_expire().set(getuptime() + 10_000);
        let now = getuptime();
        TIME_UPTIME.store(now + 100, Ordering::Relaxed);
        arptimer(core::ptr::null_mut());
        TIME_UPTIME.store(now, Ordering::Relaxed);
        let left: Vec<bool> = hosts
            .iter()
            .map(|&h| route(h, false).is_some_and(|r| r.rt_flags.isset(RTF_LLINFO)))
            .collect();
        assert_eq!(left, [false, true, false]);
        let _ = sent(ifp); // the three requests
        assert_eq!(record(&keep), Some((1, 1)));
        arpinvalidate(&keep);
        assert_eq!(
            la_hold_total.load(Ordering::Relaxed),
            0,
            "each removal gave its packet back"
        );
        {
            let g = ARP_MTX.lock();
            assert!(
                ARP_LIST.iter(&g).all(|la| la.la_rt.is_some()),
                "no marker left on the list"
            );
        }
        la_hold_total.store(0, Ordering::Relaxed);
    }

    /// The dump's `rmx_refcnt` of a route the test holds one handle of (`sysctl_dumpentry`'s
    /// `rtable_read_refcnt(rt) - 1`, the test's handle standing for the walk's).
    fn dumped_refcnt(rt: &Rtentry) -> usize {
        crate::net::rtable::rtable_read_refcnt(rt) - 1
    }

    #[test]
    fn an_arp_entry_held_by_the_table_alone_dumps_refcnt_0() {
        let _g = setup();
        let (ifp, _) = ether(b"art6");
        // A resolved neighbour (UHLc) and our local route (UHLl): each owns a record whose
        // la_rt counts its route, as the C's bare pointer does not.
        receive(ifp, &arp_bytes(ARPOP_REPLY, PEER, GATEWAY, OURS, ADDR));
        let _ = sent(ifp);
        for a in [GATEWAY, ADDR] {
            let rt = route(a, false).expect("the entry");
            assert!(rt.rt_flags.isset(RTF_LLINFO) && rt.rt_llinfo.is_typed());
            assert_eq!(dumped_refcnt(&rt), 0, "route(8) shows Refs 0 for {a:?}");
        }
        // A second holder is counted, as netstat -r shows it.
        let rt = route(GATEWAY, false).expect("the entry");
        let held = rt.clone();
        assert_eq!(dumped_refcnt(&rt), 1);
        drop(held);
    }

    #[test]
    fn requests_from_a_broadcast_address_or_from_ourselves_are_ignored() {
        let _g = setup();
        let (ifp, _) = ether(b"art7");
        const HOST: [u8; 4] = [10, 0, 2, 40];
        // A broadcast sender address: logged and dropped, nothing learned, no answer.
        receive(
            ifp,
            &arp_bytes(ARPOP_REQUEST, [0xff; 6], HOST, [0; 6], ADDR),
        );
        assert!(sent(ifp).is_empty());
        assert!(!route(HOST, false).is_some_and(|r| r.rt_flags.isset(RTF_LLINFO)));
        // Our own hardware address: our echo, ignored.
        receive(ifp, &arp_bytes(ARPOP_REQUEST, OURS, HOST, [0; 6], ADDR));
        assert!(sent(ifp).is_empty());
        assert!(!route(HOST, false).is_some_and(|r| r.rt_flags.isset(RTF_LLINFO)));
    }

    #[test]
    fn a_station_claiming_our_address_is_logged_and_answered() {
        let _g = setup();
        let (ifp, _) = ether(b"art8");
        // Another station asks for our address with it as its own: the duplicate is logged,
        // our local entry is not overwritten, and we answer for the address.
        receive(ifp, &arp_bytes(ARPOP_REQUEST, PEER, ADDR, [0; 6], ADDR));
        let reply = sent(ifp);
        assert_eq!(reply.len(), 1);
        let b = &reply[0];
        assert_eq!(&b[0..6], &PEER);
        assert_eq!(&b[20..22], &ARPOP_REPLY.to_be_bytes());
        assert_eq!(&b[22..28], &OURS);
        assert_eq!(&b[28..32], &ADDR, "our address, from our hardware address");
        let local = route(ADDR, false).expect("the local route");
        assert!(local.rt_flags.isset(RTF_LOCAL));
        assert_eq!(gw_lladdr(&local), OURS, "not overwritten");
    }

    #[test]
    fn a_permanent_entry_is_not_overwritten() {
        let _g = setup();
        let (ifp, _) = ether(b"art9");
        const PERM: [u8; 4] = [10, 0, 2, 60];
        const PERM_MAC: [u8; 6] = [0x52, 0x54, 0x00, 0x00, 0x00, 0x60];
        // arp -s 10.0.2.60 <mac> permanent
        let dst = sin(PERM);
        let mut gw = [0u8; 32];
        gw[0] = 32;
        gw[1] = AF_LINK;
        gw[2..4].copy_from_slice(&(ifp.if_index.get() as u16).to_ne_bytes());
        gw[4] = ifp.if_type.get();
        gw[6] = 6;
        gw[8..14].copy_from_slice(&PERM_MAC);
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = SockaddrView::new(&dst);
        info.rti_info[RTAX_GATEWAY] = SockaddrView::new(&gw);
        info.rti_flags = RTF_HOST | RTF_LLINFO | RTF_STATIC | RTF_PERMANENT_ARP | RTF_UP;
        info.rti_ifa = route(ADDR, false).and_then(|r| r.ifa());
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            rtrequest(RTM_ADD, &mut info, RTP_STATIC, None, 0, &net).expect("the entry");
        }
        let _ = sent(ifp);
        // Another station's reply for it: refused (the warning), the packet dropped.
        receive(ifp, &arp_bytes(ARPOP_REPLY, PEER, PERM, OURS, ADDR));
        let rt = route(PERM, false).expect("the entry");
        assert_eq!(gw_lladdr(&rt), PERM_MAC, "not overwritten");
        assert!(sent(ifp).is_empty());
    }

    #[test]
    fn another_interface_may_neither_overwrite_nor_add_an_entry() {
        let _g = setup();
        let (ifp, _) = ether(b"arta");
        let (other, _) = ether_at(b"artb", [10, 0, 3, 15]);
        const HOST: [u8; 4] = [10, 0, 2, 70];
        // A resolved entry on the first interface: a reply on the other one, from another
        // hardware address, does not overwrite it.
        receive(ifp, &arp_bytes(ARPOP_REPLY, PEER, GATEWAY, OURS, ADDR));
        let _ = sent(ifp);
        let peer2 = [0x52, 0x54, 0x00, 0x00, 0x00, 0x02];
        receive(
            other,
            &arp_bytes(ARPOP_REPLY, peer2, GATEWAY, OURS, [10, 0, 3, 15]),
        );
        let rt = route(GATEWAY, false).expect("the entry");
        assert_eq!(gw_lladdr(&rt), PEER, "still the first interface's answer");
        // An entry being asked for on the first interface (no address yet): a reply that
        // comes in on the other, unconnected interface does not fill it.
        let asked = route(HOST, true).expect("a cloned entry");
        assert_eq!(gw_alen_family(&asked).0, 0);
        receive(
            other,
            &arp_bytes(ARPOP_REPLY, peer2, HOST, OURS, [10, 0, 3, 15]),
        );
        assert_eq!(
            gw_alen_family(&asked).0,
            0,
            "not added from an unconnected interface"
        );
        assert!(sent(ifp).is_empty() && sent(other).is_empty());
    }

    /// The RARP client (`NFSCLIENT`): `revarprequest` broadcasts an RFC 903 request, and
    /// `in_revarpinput` takes the reply only while a `revarpwhoarewe` is out on that interface
    /// and only for our hardware address.
    #[cfg(feature = "nfsclient")]
    #[test]
    fn a_rarp_request_goes_out_and_the_reply_is_taken_once() {
        let _g = setup();
        let (ifp, _) = ether(b"art5");

        // The request: broadcast, from us, asking for the address of our own hardware address.
        revarprequest(ifp);
        let req = sent(ifp);
        assert_eq!(req.len(), 1);
        let b = &req[0];
        assert_eq!(&b[0..6], &[0xff; 6], "to everyone");
        assert_eq!(&b[6..12], &OURS);
        assert_eq!(&b[12..14], &ETHERTYPE_REVARP.to_be_bytes());
        assert_eq!(
            &b[14..20],
            &[0, 1, 8, 0, 6, 4],
            "ethernet, ip, 6 and 4 bytes"
        );
        assert_eq!(&b[20..22], &ARPOP_REVREQUEST.to_be_bytes());
        assert_eq!(&b[22..28], &OURS, "sender hardware address");
        assert_eq!(&b[28..32], &[0; 4]);
        assert_eq!(&b[32..38], &OURS, "target hardware address");
        assert_eq!(&b[38..42], &[0; 4]);

        let deliver = |tha: [u8; 6]| {
            let m =
                m_devget(&arp_bytes(ARPOP_REVREPLY, PEER, GATEWAY, tha, ADDR), 0).expect("an mbuf");
            m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
            revarpinput::<TestGlue>(ifp, m, None);
        };
        // Nobody is waiting: the reply is dropped.
        deliver(OURS);
        assert!(!REVARP_FINISHED.load(Ordering::Relaxed));

        // A request is out on this interface; a reply for somebody else is not ours.
        REVARP_IFIDX.store(ifp.if_index.get(), Ordering::Relaxed);
        deliver(PEER);
        assert!(!REVARP_FINISHED.load(Ordering::Relaxed));
        // One that came in on another interface is not either.
        REVARP_IFIDX.store(ifp.if_index.get() + 1, Ordering::Relaxed);
        deliver(OURS);
        assert!(!REVARP_FINISHED.load(Ordering::Relaxed));

        // Ours: the server and our address are kept.
        REVARP_IFIDX.store(ifp.if_index.get(), Ordering::Relaxed);
        deliver(OURS);
        assert!(REVARP_FINISHED.load(Ordering::Relaxed));
        assert_eq!(REVARP_SRVIP.load(Ordering::Relaxed).to_ne_bytes(), GATEWAY);
        assert_eq!(REVARP_MYIP.load(Ordering::Relaxed).to_ne_bytes(), ADDR);

        // A later reply does not change what was taken.
        REVARP_MYIP.store(0, Ordering::Relaxed);
        deliver(OURS);
        assert_eq!(REVARP_MYIP.load(Ordering::Relaxed), 0);

        // Asking again after an answer is an error; the detach of the interface clears it.
        REVARP_IFIDX.store(ifp.if_index.get(), Ordering::Relaxed);
        revarp_ifdetach(ifp);
        assert_eq!(REVARP_IFIDX.load(Ordering::Relaxed), 0);
        assert_eq!(revarpwhoarewe(ifp).err(), Some(Errno::EIO));
        assert_eq!(revarpwhoami(ifp).err(), Some(Errno::EIO));
        arp_test_reset();
    }

    #[test]
    fn bad_packets_and_the_record_s_size() {
        let _g = setup();
        // arppullup refuses another hardware or protocol, or another length.
        let mut bad = arp_bytes(ARPOP_REQUEST, PEER, GATEWAY, [0; 6], ADDR);
        bad[1] = 6; // ARPHRD_IEEE802
        assert!(arppullup(m_devget(&bad, 0).expect("an mbuf")).is_none());
        let mut bad = arp_bytes(ARPOP_REQUEST, PEER, GATEWAY, [0; 6], ADDR);
        bad[5] = 16;
        assert!(arppullup(m_devget(&bad, 0).expect("an mbuf")).is_none());
        assert!(
            arppullup(m_devget(&bad[..6], 0).expect("an mbuf")).is_none(),
            "short"
        );
        // The pool item (vmstat -m's "arp"): the C's struct llinfo_arp is 96 bytes on LP64,
        // LZ's LlinfoArp 104, this one 112 (its 56-byte MbufQueue; the park mark review 1's B1
        // removed sat in padding) (docs/SCAFFOLDING.md's pool sizes row).
        std::println!("arp item: {} bytes", size_of::<LlinfoArp>());
        assert_eq!(size_of::<LlinfoArp>(), 112);
        assert_eq!(size_of::<MbufQueue>(), 56);
    }
}
/* </TESTS> */
