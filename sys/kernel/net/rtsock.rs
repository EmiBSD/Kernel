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
 * Copyright (c) 1988, 1991, 1993
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
 *	@(#)rtsock.c	8.6 (Berkeley) 2/11/95
 */
/* </LICENSES> */

/* <CODE> */
//! The routing socket: the route domain and its one protocol, the messages user processes
//! write to change and query the routing tables (`route_output`), the messages the kernel
//! announces to every routing socket (`rtm_send`, `rtm_miss`, `rtm_ifchg`, `rtm_addr`,
//! `rtm_ifannounce`, `rtm_proposal`) and the `net.route` sysctl (route dumps, interface lists
//! and names, the preferred sources, `rtstat`, table information).
//!
//! Upstream: sys/net/rtsock.c @ 3ce1f3f79392
//! LZ: sys/net/rtsock.rs@3c62ede68802
//!
//! A routing socket is a raw socket of `PF_ROUTE` whose control block, an [`Rtpcb`], hangs
//! in the socket's protocol slot and on the list of every routing socket. A message written
//! to it is a `struct rt_msghdr` followed by the socket addresses its `rtm_addrs` names
//! (`rtm_xaddrs`); `route_output` carries out the request (`rtm_output`: `RTM_ADD`,
//! `RTM_DELETE`, `RTM_CHANGE`, `RTM_GET`; also `RTM_PROPOSAL` and `RTM_SOURCE`) and hands
//! its answer to every listener (`route_input`), the writer included unless it cleared
//! `SO_USELOOPBACK`. The announcements build their message in an mbuf (`rtm_msg1`); the
//! sysctl builds its messages in a walk buffer (`rtm_msg2`) and copies them out one by one.
//!
//! The members' locks are the C's: \[I\] immutable after creation, \[s\] the socket's lock;
//! the order is `rtptable.rtp_lk` before the socket's lock.
//!
//! ## Deviations
//! - Not configured (LZ's): `BFD` (`rtm_bfd`, `RTAX_BFD`, the `RTF_BFD` changes of
//!   `RTM_CHANGE`) and `MPLS` (`RTAX_SRC` labels, `rt_mpls_set`/`rt_mpls_clear`); comments at
//!   their sites. `SMALL_KERNEL` is not defined: `NET_RT_STATS` and `NET_RT_TABLE` are
//!   answered. `rtm_80211info` has no caller without net80211.
//! - `route_input` reads the header fields it filters on (`rtm_type`, `rtm_tableid`,
//!   `rtm_priority`, `rtm_flags`) with `m_copydata`, not through `mtod`: the answer to a
//!   user's message is copied back into the user's mbuf chain, which may hold the header in
//!   more than one mbuf (LZ's).
//! - `rtm_xaddrs` gives each address as a view bounded by its `sa_len` and the message's end
//!   (the C's pointers into the message); the C's `break` after the first address with a
//!   string (a label, a static route list, a search list) is kept: the addresses after it
//!   are not checked (LZ's).
//! - `rtm_proposal` and `route_output`'s `RTM_CHANGE` read the C's pointers that may be NULL
//!   (`rti_info[RTAX_DNS]`, an `RTF_CACHED` route's gateway) as `panic(9)` and as a family
//!   mismatch (`EINVAL`) where the C would dereference NULL.
//!
//! ## Redesign
//! - `struct rtpcb` is [`Rtpcb`], an item of the pool `"rtpcb"` held by the socket's protocol
//!   slot (`so_pcb`, a type-erased handle, the UNIX domain's shape) and by its desync timeout's
//!   argument (`timeout_set_flags`'s `so`, here the control block: the cycle the detach
//!   breaks with `Timeout::clear`); `sotortpcb` is a borrow of the slot, never a clone per
//!   request. Its \[s\] members are one `Guarded` keyed to the socket's lock, opened with the
//!   socket lock's witness (`SoLocked`); `rop_socket` a counted handle of the socket.
//! - `struct rtptable` is [`RtsockTables`] of the glue (`NetTables`, as the UNIX domain's
//!   tables): `rtp_lk` a `RawRwlock`, `rtp_list` a `PinList` under its witness; `rtp_count` is
//!   the static [`RTP_COUNT`], which the announcements read without the lock (the C's).
//! - The announcements are not generic over the glue (route.c and if.c call them), so they
//!   reach the glue's listeners through the walk [`route_prinit`] registers
//!   ([`ROUTE_INPUT`], docs/SCAFFOLDING.md: the glue generic's hop, N9).
//! - The messages are byte buffers: `rtm_msg1` writes into its one mbuf or cluster,
//!   `rtm_msg2` into a slice or the walk buffer; `route_output` and `rtm_report` keep their
//!   message in an `MArray` of `M_RTABLE` and edit a copy of its `struct rt_msghdr` (the
//!   `AbiPod` image of `hal::sys::net_route`), written back before the message goes out.
//!   `struct rt_addrinfo`'s addresses are `SockaddrView`s, so no function here reads past an
//!   address: a gateway is copied out of its route whole (`sa_len` up to 255 bytes) before a
//!   message is built from it, the interface addresses are copies read under the net lock
//!   (`SaLink::read`, D-N6.3b-8).
//! - `struct walkarg`'s buffer is an `Option<MArray<u8>>` grown as the C's `w_tmem`; every
//!   copy-out is the prefix `rtm_msg2` has just built in it under the lock the C holds there
//!   (`rtable_read`'s, the shared net lock for the interface list), never more.
//! - `rmx_refcnt` of a dumped route is `rtable_read_refcnt(rt) - 1`, the C's
//!   `refcnt_read(&rt->rt_refcnt) - 1` inside `rtable_read`'s callback (the walk's own
//!   handle left out, N6.3b1's R22).
//! - The route domain is the kernel's ([`RouteDomain`], [`RouteProto`], [`RouteUsrreqs`]:
//!   `struct domain`, `struct protosw`, `struct pr_usrreqs`, decision 46), so its sockets are
//!   locked with a witness of their own lock; `route_attach`, `route_detach` and
//!   `route_output` let it go with `SoLocked::unlocked` where the C calls `sounlock`.
//! - Lock and spl transitions against the C (rtsock.c's lines):
//!
//!   | operation | the C | here |
//!   |---|---|---|
//!   | `route_attach` (:195-238) | `solock` held; `sounlock`, `rtp_lk` write, `solock` | the same (`SoLocked::unlocked`) |
//!   | `route_detach` (:241-269) | `sounlock`, `rtp_lk` write, the timeout's barrier, `solock` | the same; `Timeout::clear` after the barrier |
//!   | `route_input` (:481-569) | `rtp_lk` read; per listener `solock`, `sb_mtx` | the same |
//!   | `route_output(RTM_GET)` (:676-907) | `sounlock`; `NET_LOCK_SHARED` for `rtm_report`; `route_input`; `solock` | the same; the lookup's SMR read sections as `rtable_lookup`'s |
//!   | `rtm_output(RTM_ADD)` (:918-961) | `NET_LOCK` around `rtm_getifa` and `rtrequest` | the same |
//!   | `rtm_output(RTM_CHANGE)` (:1020-1187) | `NET_LOCK` around `rtm_getifa`, `rt_setgate` and the flags | the same |
//!   | `sysctl_rtable(NET_RT_DUMP)` (:2228-2242) | `NET_LOCK_SHARED`; `rtable_read`'s `r_lock` | the same |
//!   | `sysctl_ifnames` (:2114-2154) | `if_tmplist_lock` write, `NET_LOCK_SHARED` for the list | the same |

use core::pin::Pin;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray};
use hal::netlock::{NETLOCK, NetHeld};
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{AnyArc, PR_NOWAIT, PR_WAITOK, PoolArc, PoolHome, TypedPool};
use hal::rwlock::{Guarded, RawRwlock};
use hal::sys::errno::Errno;
use hal::sys::net_if::{
    IFF_POINTOPOINT, IfAnnouncemsghdr, IfData, IfIeee80211Data, IfIeee80211Msghdr, IfMsghdr,
    IfNameindexMsg, IfaMsghdr,
};
use hal::sys::net_route::{
    ROUTE_FLAGFILTER, ROUTE_MSGFILTER, ROUTE_PRIOFILTER, ROUTE_TABLEFILTER, RTA_DNS, RTA_IFA,
    RTA_NETMASK, RTA_SEARCH, RTA_STATIC, RTABLE_ANY, RTAX_BRD, RTAX_DNS, RTAX_DST, RTAX_GATEWAY,
    RTAX_GENMASK, RTAX_IFA, RTAX_IFP, RTAX_LABEL, RTAX_MAX, RTAX_NETMASK, RTAX_SEARCH, RTAX_SRC,
    RTAX_STATIC, RTDNS_LEN, RTF_ANNOUNCE, RTF_BROADCAST, RTF_CACHED, RTF_CLONED, RTF_CLONING,
    RTF_DONE, RTF_FMASK, RTF_GATEWAY, RTF_HOST, RTF_LLINFO, RTF_LOCAL, RTF_MPATH, RTF_MPLS,
    RTF_STATIC, RTLABEL_LEN, RTM_80211INFO, RTM_ADD, RTM_BFD, RTM_CHANGE, RTM_DELADDR, RTM_DELETE,
    RTM_DESYNC, RTM_GET, RTM_IFANNOUNCE, RTM_IFINFO, RTM_INVALIDATE, RTM_MAXSIZE, RTM_NEWADDR,
    RTM_PROPOSAL, RTM_RESOLVE, RTM_SOURCE, RTM_VERSION, RTP_ANY, RTP_DEFAULT, RTP_LOCAL, RTP_MASK,
    RTP_MAX, RTP_PROPOSAL_SOLICIT, RTSEARCH_LEN, RTSTATIC_LEN, RTV_EXPIRE, RTV_MTU, RtMetrics,
    RtMsghdr, RtTableinfo, Rtstat, SockaddrRtdns, SockaddrRtlabel, SockaddrRtsearch,
    SockaddrRtstatic,
};
use hal::sys::param::PAGE_SIZE;
use hal::sys::socket::{
    AF_INET, AF_INET6, AF_LINK, AF_MAX, AF_ROUTE, AF_UNSPEC, NET_RT_DUMP, NET_RT_FLAGS,
    NET_RT_IFLIST, NET_RT_IFNAMES, NET_RT_SOURCE, NET_RT_STATS, NET_RT_TABLE, PF_KEY, PF_ROUTE,
    SO_USELOOPBACK, SOCK_RAW,
};
use hal::sys::types::{Pid, SaFamily};
use hal::user::{AbiPod, UserPtr};

use crate::assert::kpanic;
use crate::net::domain::{Domain, net};
use crate::net::if_::{
    IF_TMPLIST_LOCK, IFNETLIST, if_get, if_getdata, if_group_routechange, if_put, if_ref,
    ifa_ifwithaddr, ifa_ifwithdstaddr, ifaof_ifpforaddr,
};
use crate::net::if_var::{IfTmplist, Ifaddr, Ifnet};
use crate::net::ipsec_hooks::ipsec;
use crate::net::mbuf::{
    M_COPYALL, M_DONTWAIT, M_EXT, M_PKTHDR, MCLBYTES, MHLEN, MSIZE, MT_DATA, Mbuf, MbufBox, m_adj,
    m_copyback, m_copydata, m_copym, m_gethdr, m_pullup, mclget,
};
use crate::net::proto::{
    PR_ADDR, PR_ATOMIC, PR_WANTRCVD, PRCO_GETOPT, PRCO_SETOPT, Protocol, SockaddrView, Usrreqs,
};
use crate::net::route::{
    RT_RESOLVE, RTCOUNTERS, RtAddrinfo, RtKmetrics, Rtentry, ifafree, ifaref, rt_getll,
    rt_if_linkstate_change, rt_plen2mask, rt_setgate, rt_timer_get_expire, rtalloc, rtlabel_id2sa,
    rtlabel_name2id, rtlabel_unref, rtrequest, rtrequest_delete, sadl_copy,
};
use crate::net::rtable::{
    M_RTABLE, SaCopy, rtable_add, rtable_exists, rtable_getsource, rtable_l2, rtable_lookup,
    rtable_match, rtable_read, rtable_read_refcnt, rtable_satoplen, rtable_setsource, rtable_walk,
};
use crate::net::sockbuf::{SS_CANTRCVMORE, SS_ISCONNECTED};
use crate::net::socket::{
    SoLocked, Socket, SocketRef, socantsendmore, soisconnected, soisdisconnected, solock, soref,
    soreserve, sorwakeup,
};
use crate::proc::{ProcGlue, suser};
use crate::sysctl::{SysctlReq, rdstruct};
use crate::timeout::{KCLOCK_NONE, TIMEOUT_MPSAFE, TIMEOUT_PROC, Timeout};
use hal::list::List;

/// `ROUTESNDQ`: a routing socket's send buffer.
pub const ROUTESNDQ: u64 = 8192;
/// `ROUTERCVQ`: a routing socket's receive buffer.
pub const ROUTERCVQ: u64 = 8192;

/// `ROUTECB_FLAG_DESYNC`: the routing socket ran out of memory (messages were lost).
const ROUTECB_FLAG_DESYNC: u32 = 0x1;
/// `ROUTECB_FLAG_FLUSH`: wait until the socket is empty before queueing more messages.
const ROUTECB_FLAG_FLUSH: u32 = 0x2;
/// `ROUTE_DESYNC_RESEND_TIMEOUT`: the `RTM_DESYNC` retry, in milliseconds.
const ROUTE_DESYNC_RESEND_TIMEOUT: u64 = 200;

/// `route_src`: the fake peer address of every routing message, `{ 2, PF_ROUTE }`.
const ROUTE_SRC: [u8; 2] = [2, PF_ROUTE];

/// `sizeof(socklen_t)`: what `rtm_xaddrs` needs left before an address.
const SOCKLEN: usize = 4;
/// `sizeof(struct sockaddr_in)`.
const SIN_LEN: usize = 16;
/// `sizeof(struct sockaddr_in6)`.
const SIN6_LEN: usize = 28;
/// `sizeof(struct sockaddr_dl)`.
const SDL_LEN: usize = size_of::<hal::sys::net_if::SockaddrDl>();
/// The longest socket address (`sa_len` is a byte).
const SA_MAXLEN: usize = 256;

/// `sizeof(struct rt_msghdr)`.
const RT_MSGHDR: usize = size_of::<RtMsghdr>();
/// `sizeof(struct ifa_msghdr)`.
const IFA_MSGHDR: usize = size_of::<IfaMsghdr>();
/// `sizeof(struct if_msghdr)`.
const IF_MSGHDR: usize = size_of::<IfMsghdr>();

/// `ROUNDUP(a)`: a socket address's room in a message, a multiple of `sizeof(long)` (8 for
/// an empty one).
const fn roundup_long(a: usize) -> usize {
    if a > 0 { 1 + ((a - 1) | 7) } else { 8 }
}

/// `ALIGN(len)`: to the next natural boundary (`_ALIGNBYTES` is 7 on LP64).
const fn align(len: usize) -> usize {
    (len + 7) & !7
}

/// A whole socket address copied out of the storage that holds it (a route's gateway), so a
/// message can be built from it after the storage's cell is let go.
struct SaBuf {
    /// The bytes; `buf[0]` is `sa_len`.
    buf: [u8; SA_MAXLEN],
}

impl SaBuf {
    /// The gateway of `rt`, whole (`None` without one).
    fn gateway(rt: &Rtentry) -> Option<Self> {
        rt.with_gateway(|g| {
            let g = g?;
            let mut s = Self {
                buf: [0; SA_MAXLEN],
            };
            s.buf[..g.len()].copy_from_slice(g);
            Some(s)
        })
    }

    /// The address.
    fn view(&self) -> Option<SockaddrView<'_>> {
        SockaddrView::new(&self.buf)
    }
}

/// `struct walkarg`: the state of a `net.route` sysctl walk.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct walkarg's w_* names so the C can be grepped"
)]
struct Walkarg {
    /// `w_op`: `NET_RT_*`.
    w_op: i32,
    /// `w_arg`: the operation's argument (flags, a priority, an interface index).
    w_arg: i32,
    /// `w_given`: the room at `w_where`.
    w_given: usize,
    /// `w_needed`: the bytes the answer takes.
    w_needed: usize,
    /// `w_where`: the user address the next message goes to (0: none, a size estimate).
    w_where: usize,
    /// `w_tmem`: the message being built (`w_tmemsize` its length).
    w_tmem: Option<MArray<u8>>,
    /// `curproc->p_p->ps_pid`, stamped on every route message.
    w_pid: Pid,
}

/// `struct rtpcb`'s members under the socket's lock (\[s\]).
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct rtpcb's rop_* names so the C can be grepped"
)]
struct RopState {
    /// `rop_msgfilter`: the message types the socket wants (0: all).
    rop_msgfilter: u32,
    /// `rop_flagfilter`: the route flags whose messages the socket does not want.
    rop_flagfilter: u32,
    /// `rop_flags`: `ROUTECB_FLAG_*`.
    rop_flags: u32,
    /// `rop_rtableid`: the table the socket listens to (`RTABLE_ANY`: all).
    rop_rtableid: u32,
    /// `rop_priority`: the lowest priority the socket wants (0: all).
    rop_priority: u8,
}

/// `struct rtpcb`: a routing socket's control block, an item of the pool `"rtpcb"`.
pub struct Rtpcb<G: ProcGlue> {
    /// The item's place in `rtpcb_pool`.
    home: PoolHome<Self>,
    /// \[I\] `rop_socket`: its socket (counted; the detach clears the socket's slot).
    rop_socket: SocketRef<G>,
    /// `rop_list`: on `rtptable.rtp_list`, under `rtp_lk`.
    rop_list: PinEntry<Self>,
    /// `rop_timeout`: the `RTM_DESYNC` retry; its argument a handle of this block.
    rop_timeout: Timeout,
    /// \[s\] The members under the socket's lock.
    s: Guarded<RopState>,
    /// \[I\] `rop_proto`: the address family the socket was made for (`AF_UNSPEC`: all).
    rop_proto: u16,
}

hal::pool_home!(impl[G: ProcGlue] Rtpcb<G>, home);

hal::pin_adapter!(
    /// `rtptable.rtp_list`: every routing socket, through `rop_list`.
    impl[G: ProcGlue] pub RtpList<G>: Rtpcb<G>, rop_list
);

impl<G: ProcGlue> Rtpcb<G> {
    /// The \[s\] members, for the holder of the socket's lock.
    fn state<'a>(&'a self, w: &'a SoLocked<'_>) -> &'a RopState {
        match w.so_lock_ref() {
            Some(x) => self.s.get(x),
            None => kpanic(format_args!("rtpcb: a socket locked without its so_lock")),
        }
    }

    /// The \[s\] members, to change them.
    fn state_mut<'a>(&'a self, w: &'a mut SoLocked<'_>) -> &'a mut RopState {
        match w.so_lock() {
            Some(x) => self.s.get_mut(x),
            None => kpanic(format_args!("rtpcb: a socket locked without its so_lock")),
        }
    }
}

/// `struct rtptable` and `rtpcb_pool` of one glue (a member of its `NetTables`).
pub struct RtsockTables<G: ProcGlue> {
    /// `rtpcb_pool`.
    rtpcbpl: TypedPool<Rtpcb<G>>,
    /// `rtp_lk`: the list's lock, before any socket's.
    rtp_lk: RawRwlock,
    /// `rtp_list`: every routing socket.
    rtp_list: PinList<RtpList<G>>,
}

impl<G: ProcGlue> RtsockTables<G> {
    /// The empty tables, before `route_prinit`.
    pub const fn new() -> Self {
        Self {
            rtpcbpl: TypedPool::new(),
            rtp_lk: RawRwlock::new("rtsock"),
            rtp_list: PinList::new(),
        }
    }

    /// `rtpcb_pool`.
    pub fn pool(&self) -> &TypedPool<Rtpcb<G>> {
        &self.rtpcbpl
    }
}

impl<G: ProcGlue> Default for RtsockTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `routedomain`: the route domain (a type without data, one per glue).
pub struct RouteDomain<G>(core::marker::PhantomData<fn() -> G>);

impl<G: ProcGlue> RouteDomain<G> {
    /// `routedomain`.
    pub const NEW: Self = Self(core::marker::PhantomData);
    /// The domain, as its protocol names it.
    const DOMAIN: &'static dyn Domain<G> = &Self(core::marker::PhantomData);
    /// `routesw[]`: the route domain's one protocol.
    const PROTOSW: &'static [&'static dyn Protocol<G>] =
        &[&RouteProto::<G>(core::marker::PhantomData)];
}

/// `routesw[0]`: the raw routing protocol.
pub struct RouteProto<G>(core::marker::PhantomData<fn() -> G>);

/// `route_usrreqs`: the routing socket's requests.
pub struct RouteUsrreqs<G>(core::marker::PhantomData<fn() -> G>);

impl<G: ProcGlue> RouteUsrreqs<G> {
    /// `&route_usrreqs`.
    const USR: &'static dyn Usrreqs<G> = &Self(core::marker::PhantomData);
}

/// `rtptable.rtp_count`: the routing sockets open, changed under `rtp_lk` and read without
/// it by the announcements, as the C reads it.
pub static RTP_COUNT: AtomicU32 = AtomicU32::new(0);

/// The walk of the listeners of the glue whose `route_prinit` ran first: how the
/// announcements, which are not generic over the glue, reach `route_input` (the module's
/// redesign; docs/SCAFFOLDING.md).
static ROUTE_INPUT: OnceStatic<fn(MbufBox, SaFamily)> = OnceStatic::new();

/// The glue's routing socket tables.
#[inline]
fn tables<G: ProcGlue>() -> &'static RtsockTables<G> {
    &net::<G>().rtsock
}

/// `curproc->p_p->ps_pid` (0 without a process).
fn curpid<G: ProcGlue>() -> Pid {
    G::with_curproc(|p| p.map_or(0, |p| p.process().ps_pid))
}

/// `curproc->p_p->ps_rtableid` (0 without a process).
fn currtable<G: ProcGlue>() -> u32 {
    G::with_curproc(|p| p.map_or(0, |p| p.process().ps_rtableid.load(Ordering::Relaxed)))
}

/// The proof the running code holds the net lock, from its witness `w`.
fn net_held<W: hal::rwlock::Witness>(w: &W) -> NetHeld<'_> {
    match NetHeld::of(w) {
        Some(h) => h,
        None => kpanic(format_args!("not a witness of the net lock")),
    }
}

/// The 16-bit word at `off` of `b` (0 past its end), the C's `u_short` member.
fn u16_at(b: &[u8], off: usize) -> u16 {
    match b.get(off..off + 2) {
        Some(w) => u16::from_ne_bytes([w[0], w[1]]),
        None => 0,
    }
}

/// `satosdl_const(sa)->sdl_index`: the interface index of a link-level address.
fn sdl_index(sa: SockaddrView<'_>) -> u32 {
    u32::from(u16_at(sa.bytes(), 2))
}

/// A header `T` at the start of `buf` changed by `f` (copied out and written back).
fn hdr_edit<T: AbiPod>(buf: &mut [u8], f: impl FnOnce(&mut T)) {
    let Some(mut h) = hal::pod::at::<T>(buf, 0) else {
        kpanic(format_args!("routing message shorter than its header"));
    };
    f(&mut h);
    hal::pod::put(buf, 0, &h);
}

/// `route_prinit()`: the list's lock and the control blocks' pool; the glue's listeners
/// registered for the announcements (a second call, a test's, keeps the pool and the first
/// glue's walk).
pub fn route_prinit<G: ProcGlue>() {
    let t = tables::<G>();
    let _ = t.rtpcbpl.init_recorded(IPL_SOFTNET, PR_WAITOK, "rtpcb");
    let _ = ROUTE_INPUT.set(route_input_announce::<G>);
}

/// `route_attach(so, proto, wait)`: the socket's buffers and its control block, the socket
/// connected and looped back, then put on the list of routing sockets.
fn route_attach<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    proto: i32,
    wait: i32,
) -> Result<(), Errno> {
    soreserve(so, ROUTESNDQ, ROUTERCVQ)?;

    // Use the rawcb but allocate a rtpcb, this code does not care about the additional
    // fields and works directly on the raw socket.
    let t = tables::<G>();
    let flags = if wait == crate::net::mbuf::M_WAIT {
        PR_WAITOK
    } else {
        PR_NOWAIT
    };
    let rtableid = currtable::<G>();
    let Some(rop) = t.rtpcbpl.get_homed(flags, || Rtpcb {
        home: PoolHome::new(),
        rop_socket: soref(so),
        rop_list: PinEntry::new(),
        rop_timeout: Timeout::unset(),
        s: Guarded::new_keyed(
            so.so_lock.lock(),
            RopState {
                rop_msgfilter: 0,
                rop_flagfilter: 0,
                rop_flags: 0,
                rop_rtableid: rtableid,
                rop_priority: 0,
            },
        ),
        rop_proto: proto as u16,
    }) else {
        return Err(Errno::ENOBUFS);
    };
    drop(so.set_pcb(w, Some(PoolArc::erase(rop.clone()))));
    rop.rop_timeout.set_shared(
        rtm_senddesync_timer::<G>,
        PoolArc::erase(rop.clone()),
        KCLOCK_NONE,
        TIMEOUT_PROC | TIMEOUT_MPSAFE,
    );

    soisconnected(so);
    so.set_options(w, SO_USELOOPBACK, true);

    // Give up solock before taking rtp_lk for the lock ordering.
    w.unlocked(|| {
        let mut g = t.rtp_lk.write();
        t.rtp_list.insert_tail(&mut g, rop.as_pin());
        RTP_COUNT.fetch_add(1, Ordering::Relaxed);
    });

    Ok(())
}

/// `sotortpcb(so)` counted: the control block of a routing socket (`None` before its attach
/// and after its detach), for a holder that keeps it past the slot's read.
fn rtpcb_ref<G: ProcGlue>(so: &Socket<G>) -> Option<PoolArc<Rtpcb<G>>> {
    let pcb = so.pcb_read();
    let rop = pcb.as_ref()?.downcast_ref::<Rtpcb<G>>()?;
    PoolArc::from_ref(rop)
}

/// `route_detach(so)`: the control block off the list, its timeout stopped, then freed with
/// the socket's slot cleared; `EINVAL` without one.
fn route_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let Some(rop) = rtpcb_ref(so) else {
        return Err(Errno::EINVAL);
    };
    let t = tables::<G>();

    // Give up solock before taking rtp_lk for the lock ordering.
    w.unlocked(|| {
        let mut g = t.rtp_lk.write();
        RTP_COUNT.fetch_sub(1, Ordering::Relaxed);
        t.rtp_list.remove(&mut g, &rop);
        drop(g);

        // wait for all references to drop
        rop.rop_timeout.del_barrier();
        rop.rop_timeout.clear();
    });

    drop(so.set_pcb(w, None));
    crate::assert::kassert!(!so.has_state(crate::net::sockbuf::SS_NOFDREF));
    drop(rop);

    Ok(())
}

/// `route_rcvd(so)`: in a flush state, the flag cleared once the receive buffer is empty.
fn route_rcvd<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) {
    let pcb = so.pcb_read();
    let Some(rop) = pcb.as_ref().and_then(|a| a.downcast_ref::<Rtpcb<G>>()) else {
        kpanic(format_args!("route_rcvd: no control block"));
    };

    // If we are in a FLUSH state, check if the buffer is empty so that we can clear the
    // flag.
    let rg = so.so_rcv.lock();
    let st = rop.state_mut(w);
    if st.rop_flags & ROUTECB_FLAG_FLUSH != 0 && rg.space() == so.so_rcv.hiwat() as i64 {
        st.rop_flags &= !ROUTECB_FLAG_FLUSH;
    }
    drop(rg);
}

/// `route_send(so, m, nam, control)`: a message written to the socket, carried out by
/// `route_output`; `EOPNOTSUPP` with control data, `EISCONN` with an address.
fn route_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    nam: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    if control.as_ref().is_some_and(|c| c.m_len().get() != 0) {
        return Err(Errno::EOPNOTSUPP);
    }
    if nam.is_some() {
        return Err(Errno::EISCONN);
    }
    drop(control);
    route_output(m, so, w)
}

/// `route_peeraddr(so, nam)`: minimal support, just a fake peer address (`route_src`).
fn route_peeraddr(nam: &mut Mbuf) {
    nam.m_len().set(ROUTE_SRC.len() as u32);
    match nam.data_mut() {
        Some(d) => d.copy_from_slice(&ROUTE_SRC),
        None => kpanic(format_args!("route_peeraddr: a read-only address mbuf")),
    }
}

/// The `u_int` an option mbuf holds (`EINVAL` unless it holds exactly one).
fn opt_uint(m: Option<&mut Mbuf>) -> Result<u32, Errno> {
    let Some(m) = m else {
        return Err(Errno::EINVAL);
    };
    if m.m_len().get() as usize != size_of::<u32>() {
        return Err(Errno::EINVAL);
    }
    let d = m.data();
    Ok(u32::from_ne_bytes([d[0], d[1], d[2], d[3]]))
}

/// `route_ctloutput(op, so, level, optname, m)`: the filters of a routing socket set or
/// read (`ROUTE_MSGFILTER`, `ROUTE_TABLEFILTER`, `ROUTE_PRIOFILTER`, `ROUTE_FLAGFILTER`).
fn route_ctloutput<G: ProcGlue>(
    op: i32,
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    if level != i32::from(AF_ROUTE) {
        return Err(Errno::EINVAL);
    }
    let pcb = so.pcb_read();
    let Some(rop) = pcb.as_ref().and_then(|a| a.downcast_ref::<Rtpcb<G>>()) else {
        kpanic(format_args!("route_ctloutput: no control block"));
    };

    match op {
        PRCO_SETOPT => match optname {
            ROUTE_MSGFILTER => {
                let v = opt_uint(m)?;
                rop.state_mut(w).rop_msgfilter = v;
                Ok(())
            }
            ROUTE_TABLEFILTER => {
                let tid = opt_uint(m)?;
                if tid != RTABLE_ANY && !rtable_exists(tid) {
                    return Err(Errno::ENOENT);
                }
                rop.state_mut(w).rop_rtableid = tid;
                Ok(())
            }
            ROUTE_PRIOFILTER => {
                let prio = opt_uint(m)?;
                if prio > u32::from(RTP_MAX) {
                    return Err(Errno::EINVAL);
                }
                rop.state_mut(w).rop_priority = prio as u8;
                Ok(())
            }
            ROUTE_FLAGFILTER => {
                let v = opt_uint(m)?;
                rop.state_mut(w).rop_flagfilter = v;
                Ok(())
            }
            _ => Err(Errno::ENOPROTOOPT),
        },
        PRCO_GETOPT => {
            let st = rop.state(w);
            let v = match optname {
                ROUTE_MSGFILTER => st.rop_msgfilter,
                ROUTE_TABLEFILTER => st.rop_rtableid,
                ROUTE_PRIOFILTER => u32::from(st.rop_priority),
                ROUTE_FLAGFILTER => st.rop_flagfilter,
                _ => return Err(Errno::ENOPROTOOPT),
            };
            let Some(m) = m else {
                kpanic(format_args!("route_ctloutput: PRCO_GETOPT without an mbuf"));
            };
            m.m_len().set(size_of::<u32>() as u32);
            match m.data_mut() {
                Some(d) => d.copy_from_slice(&v.to_ne_bytes()),
                None => kpanic(format_args!("route_ctloutput: a read-only option mbuf")),
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// `rtm_senddesync_timer(so)`: the `RTM_DESYNC` retry, under the socket's lock.
fn rtm_senddesync_timer<G: ProcGlue>(arg: &AnyArc) {
    let Some(rop) = arg.downcast_ref::<Rtpcb<G>>() else {
        kpanic(format_args!(
            "rtm_senddesync_timer: not a routing control block"
        ));
    };
    let Some(rop) = PoolArc::from_ref(rop) else {
        kpanic(format_args!("rtm_senddesync_timer: not in rtpcb_pool"));
    };
    let so = &*rop.rop_socket;
    let mut w = solock(so);
    rtm_senddesync(so, rop.as_pin(), &mut w);
    drop(w);
}

/// `rtm_senddesync(so)`: a desynchronised socket told so (`RTM_DESYNC`), or the retry armed
/// again when the message cannot be queued.
fn rtm_senddesync<G: ProcGlue>(so: &Socket<G>, rop: Pin<&Rtpcb<G>>, w: &mut SoLocked<'_>) {
    // Dying socket is disconnected by upper layer and there is no reason to send packet.
    // Also we shouldn't reschedule timeout(9), otherwise timeout_del_barrier(9) can't help
    // us.
    if !so.has_state(SS_ISCONNECTED) || so.so_rcv.has_state(SS_CANTRCVMORE) {
        return;
    }

    // If we are in a DESYNC state, try to send a RTM_DESYNC packet.
    if rop.state(w).rop_flags & ROUTECB_FLAG_DESYNC == 0 {
        return;
    }

    // If we fail to alloc memory or if sbappendaddr() fails, re-add timeout and try again.
    if let Some(desync) = rtm_msg1(RTM_DESYNC, None) {
        let mut rg = so.so_rcv.lock();
        let appended = rg.sbappendaddr(&ROUTE_SRC, Some(desync), None).is_ok();
        drop(rg);

        if appended {
            rop.state_mut(w).rop_flags &= !ROUTECB_FLAG_DESYNC;
            sorwakeup(so);
            return;
        }
    }
    // Re-add timeout to try sending msg again.
    hal::pin::project(rop, |r| &r.rop_timeout).add_msec(ROUTE_DESYNC_RESEND_TIMEOUT);
}

/// The header fields `route_input` filters on, copied out of the message (zeros past a
/// short one): `rtm_type`, `rtm_tableid`, `rtm_priority`, `rtm_flags`.
struct InputHdr {
    /// `rtm_type`.
    ty: u8,
    /// `rtm_tableid`.
    tableid: u16,
    /// `rtm_priority`.
    priority: u8,
    /// `rtm_flags`.
    flags: u32,
}

impl InputHdr {
    /// The fields of the message `m` (its first 20 bytes).
    fn of(m: &Mbuf) -> Self {
        let mut h = [0u8; 20];
        let n = (m.m_pkthdr().len.get().max(0) as usize).min(h.len());
        m_copydata(m, 0, &mut h[..n]);
        Self {
            ty: h[3],
            tableid: u16::from_ne_bytes([h[8], h[9]]),
            priority: h[10],
            flags: u32::from_ne_bytes([h[16], h[17], h[18], h[19]]),
        }
    }
}

/// An announcement handed to the listeners ([`ROUTE_INPUT`]); nothing before a glue
/// registers its walk, as with no routing socket open.
fn route_input_any(m: MbufBox, sa_family: SaFamily) {
    #[cfg(test)]
    tap::record(&m);
    if let Some(f) = ROUTE_INPUT.get() {
        f(m, sa_family);
    }
}

/// `route_input(m, NULL, sa_family)` of the glue `G`: the walk [`route_prinit`] registers.
fn route_input_announce<G: ProcGlue>(m: MbufBox, sa_family: SaFamily) {
    route_input::<G>(m, None, sa_family);
}

/// Whether a routing socket listens (`rtptable.rtp_count != 0`), read without the lock.
fn listeners() -> bool {
    #[cfg(test)]
    if tap::on() {
        return true;
    }
    RTP_COUNT.load(Ordering::Relaxed) != 0
}

/// `route_input(m0, so0, sa_family)`: the message `m0` copied to every routing socket that
/// wants it (its address family, its filters, its table), `so0` (the writer) excluded unless
/// it loops its messages back.
fn route_input<G: ProcGlue>(mut m: MbufBox, so0: Option<&Socket<G>>, sa_family: SaFamily) {
    // Ensure that we can access the rtm_type via mtod().
    if (m.m_len().get() as usize) < 4 {
        return;
    }
    let hdr = InputHdr::of(&m);

    let t = tables::<G>();
    let g = t.rtp_lk.read();
    for rop in t.rtp_list.iter_pinned(&g) {
        // If route socket is bound to an address family only send messages that match the
        // address family. Address family agnostic messages are always sent.
        if sa_family != AF_UNSPEC
            && rop.rop_proto != u16::from(AF_UNSPEC)
            && rop.rop_proto != u16::from(sa_family)
        {
            continue;
        }

        let so = &*rop.rop_socket;
        let mut w = solock(so);

        // Check to see if we don't want our own messages and if we can receive anything.
        if (so0.is_some_and(|s0| core::ptr::eq(s0, so)) && !so.has_options(SO_USELOOPBACK))
            || !so.has_state(SS_ISCONNECTED)
            || so.so_rcv.has_state(SS_CANTRCVMORE)
        {
            continue;
        }

        let st = rop.state(&w);
        // Filter messages that the process does not want, but RTM_DESYNC can't be
        // filtered.
        if hdr.ty != RTM_DESYNC {
            if st.rop_msgfilter != 0
                && st.rop_msgfilter & 1u32.checked_shl(u32::from(hdr.ty)).unwrap_or(0) == 0
            {
                continue;
            }
            if st.rop_flagfilter & hdr.flags != 0 {
                continue;
            }
        }
        match hdr.ty {
            RTM_IFANNOUNCE | RTM_DESYNC => {
                // no tableid
            }
            RTM_RESOLVE | RTM_NEWADDR | RTM_DELADDR | RTM_IFINFO | RTM_80211INFO | RTM_BFD => {
                // check against rdomain id
                if st.rop_rtableid != RTABLE_ANY
                    && rtable_l2(st.rop_rtableid) != u32::from(hdr.tableid)
                {
                    continue;
                }
            }
            _ => {
                if st.rop_priority != 0 && st.rop_priority < hdr.priority {
                    continue;
                }
                // check against rtable id
                if st.rop_rtableid != RTABLE_ANY && st.rop_rtableid != u32::from(hdr.tableid) {
                    continue;
                }
            }
        }

        // Check to see if the flush flag is set. If so, don't queue any more messages until
        // the flag is cleared.
        if st.rop_flags & ROUTECB_FLAG_FLUSH != 0 {
            continue;
        }

        let _ = rtm_sendup(so, rop, &mut w, &mut m);
    }
    drop(g);
}

/// `rtm_sendup(so, m0)`: a copy of `m0` queued on the socket; when it does not fit the socket
/// is flagged desynchronised (and flushing) and told so (`ENOBUFS`).
fn rtm_sendup<G: ProcGlue>(
    so: &Socket<G>,
    rop: Pin<&Rtpcb<G>>,
    w: &mut SoLocked<'_>,
    m0: &mut Mbuf,
) -> Result<(), Errno> {
    let Some(m) = m_copym(m0, 0, M_COPYALL, M_DONTWAIT) else {
        return Err(Errno::ENOMEM);
    };

    let mut rg = so.so_rcv.lock();
    let send_desync = if rg.space() < (2 * MSIZE) as i64 {
        drop(m);
        true
    } else {
        rg.sbappendaddr(&ROUTE_SRC, Some(m), None).is_err()
    };
    drop(rg);

    if send_desync {
        // Flag socket as desync'ed and flush required.
        rop.state_mut(w).rop_flags |= ROUTECB_FLAG_DESYNC | ROUTECB_FLAG_FLUSH;
        rtm_senddesync(so, rop, w);
        return Err(Errno::ENOBUFS);
    }

    sorwakeup(so);
    Ok(())
}

/// The copies `rtm_report`, `rtm_send` and `sysctl_dumpentry` read a route's addresses
/// into (`rt_key`, `rt_gateway`, `rt_plen2mask`, `rtlabel_id2sa`, the interface's and its
/// address's), under the net lock `net`.
struct RouteAddrs {
    /// `rt->rt_gateway`, whole.
    gateway: Option<SaBuf>,
    /// The netmask of `rt_plen`.
    mask: SaCopy,
    /// Whether there is a netmask (a host route has none).
    has_mask: bool,
    /// `sockaddr_rtlabel` of `rt_labelid`.
    label: SockaddrRtlabel,
    /// `sdltosa(ifp->if_sadl)` of the route's interface.
    ifp: Option<SaCopy>,
    /// `rtable_getsource(tableid, af)` or `rt->rt_ifa->ifa_addr`.
    ifa: Option<SaCopy>,
    /// `rt->rt_ifa->ifa_dstaddr` of a point-to-point interface.
    brd: Option<SaCopy>,
}

impl RouteAddrs {
    /// The addresses of `rt` in table `tableid`; the broadcast address only when `brd`.
    fn of(rt: &Rtentry, tableid: u32, brd: bool, net: &NetHeld<'_>) -> Self {
        let mut a = Self {
            gateway: SaBuf::gateway(rt),
            mask: SaCopy::zeroed(),
            has_mask: false,
            label: SockaddrRtlabel::default(),
            ifp: None,
            ifa: None,
            brd: None,
        };
        a.has_mask = rt_plen2mask(rt, &mut a.mask).is_some();
        let ifp = if_get(rt.rt_ifidx.get());
        if let Some(ifp) = ifp {
            a.ifp = sadl_copy(ifp);
            a.ifa = rtable_getsource(tableid, rt.dest_view().family(), net);
            if a.ifa.is_none() {
                a.ifa = rt
                    .ifa()
                    .and_then(|ifa| ifa.ifa_addr.read(net))
                    .and_then(|b| SockaddrView::new(&b).and_then(SaCopy::of));
            }
            if brd && ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
                a.brd = rt
                    .ifa()
                    .and_then(|ifa| ifa.ifa_dstaddr.read(net))
                    .and_then(|b| SockaddrView::new(&b).and_then(SaCopy::of));
            }
        }
        if_put(ifp);
        a
    }

    /// The request of these addresses for `rt`: `RTAX_DST`, `RTAX_GATEWAY`, `RTAX_NETMASK`
    /// (unless `!mask`), `RTAX_LABEL`, `RTAX_IFP`, `RTAX_IFA`, `RTAX_BRD`.
    fn info<'a>(&'a mut self, rt: &'a Rtentry, mask: bool) -> RtAddrinfo<'a> {
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = Some(rt.dest_view());
        info.rti_info[RTAX_GATEWAY] = self.gateway.as_ref().and_then(SaBuf::view);
        if mask && self.has_mask {
            info.rti_info[RTAX_NETMASK] = Some(self.mask.view());
        }
        info.rti_info[RTAX_LABEL] = rtlabel_id2sa(rt.rt_labelid.get(), &mut self.label);
        // BFD: RTAX_BFD (bfd2sa of an RTF_BFD route), not configured.
        // MPLS: RTAX_SRC and rti_mpls (an RTF_MPLS route's label), not configured.
        info.rti_info[RTAX_IFP] = self.ifp.as_ref().map(SaCopy::view);
        info.rti_info[RTAX_IFA] = self.ifa.as_ref().map(SaCopy::view);
        info.rti_info[RTAX_BRD] = self.brd.as_ref().map(SaCopy::view);
        info
    }
}

/// `rtm_report(rt, type, seq, tableid)`: the message answering a request about route `rt`
/// (its addresses, flags, priority and metrics), in a new `M_RTABLE` buffer, under the shared
/// net lock `net`.
fn rtm_report(
    rt: &Rtentry,
    ty: u8,
    seq: i32,
    tableid: u32,
    pid: Pid,
    net: &NetHeld<'_>,
) -> MArray<u8> {
    let mut a = RouteAddrs::of(rt, tableid, true, net);
    let mut info = a.info(rt, true);
    // RTAX_GENMASK, RTAX_AUTHOR, RTAX_SRCMASK ignored

    // build new route message
    let len = rtm_msg2(ty, &mut info, None, None);
    let Some(mut rtm) = MArray::new(M_RTABLE, M_WAITOK, len, |_| 0u8) else {
        kpanic(format_args!("rtm_report: no memory"));
    };
    rtm_msg2(ty, &mut info, Some(&mut rtm), None);
    let addrs = info.rti_addrs;
    hdr_edit::<RtMsghdr>(&mut rtm, |h| {
        h.rtm_type = ty;
        h.rtm_index = rt.rt_ifidx.get() as u16;
        h.rtm_tableid = tableid as u16;
        h.rtm_priority = rt.rt_priority.get() & RTP_MASK;
        h.rtm_flags = rt.rt_flags.get() as i32;
        h.rtm_pid = pid;
        h.rtm_seq = seq;
        rtm_getmetrics(rt, &mut h.rtm_rmx);
        h.rtm_addrs = addrs;
        // MPLS: rtm_mpls = rti_mpls, not configured.
    });
    rtm
}

/// `route_output(m, so)`: a message written to the routing socket `so` carried out (the
/// socket's lock let go meanwhile, as the C's `sounlock`), its answer handed to the
/// listeners; the request's error returned (and carried by the answer).
fn route_output<G: ProcGlue>(
    m: Option<MbufBox>,
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
) -> Result<(), Errno> {
    let Some(mut m) = m else {
        return Err(Errno::ENOBUFS);
    };
    if (m.m_len().get() as usize) < size_of::<i32>() {
        m = m_pullup(m, size_of::<i32>() as i32).ok_or(Errno::ENOBUFS)?;
    }
    if m.flags() & M_PKTHDR == 0 {
        kpanic(format_args!("route_output"));
    }

    let useloopback = so.has_options(SO_USELOOPBACK);

    // The socket can't be closed concurrently because the file descriptor reference is
    // still held.
    w.unlocked(|| route_output_unlocked::<G>(m, so, useloopback))
}

/// `route_output`'s body, with the socket unlocked.
fn route_output_unlocked<G: ProcGlue>(
    mut m: MbufBox,
    so: &Socket<G>,
    useloopback: bool,
) -> Result<(), Errno> {
    let len = m.m_pkthdr().len.get();
    let d = m.data();
    let msglen = i32::from(u16::from_ne_bytes([d[0], d[1]]));
    let vers = d[2];
    // offsetof(struct rt_msghdr, rtm_hdrlen) + sizeof(rtm->rtm_hdrlen)
    if len < 6 || len != msglen {
        return Err(Errno::EINVAL);
    }
    let len = len as usize;
    let mut rtm = match vers {
        RTM_VERSION => {
            if len < RT_MSGHDR {
                return Err(Errno::EINVAL);
            }
            if len > RTM_MAXSIZE {
                return Err(Errno::EMSGSIZE);
            }
            let Some(mut b) = MArray::new(M_RTABLE, M_WAITOK, len, |_| 0u8) else {
                kpanic(format_args!("route_output: no memory"));
            };
            m_copydata(&m, 0, &mut b);
            b
        }
        _ => return Err(Errno::EPROTONOSUPPORT),
    };
    let Some(mut hdr) = hal::pod::at::<RtMsghdr>(&rtm, 0) else {
        kpanic(format_args!("route_output: a short message"));
    };

    // Verify that the caller is sending an appropriate message early.
    match hdr.rtm_type {
        RTM_ADD | RTM_DELETE | RTM_GET | RTM_CHANGE | RTM_PROPOSAL | RTM_SOURCE => {}
        _ => return Err(Errno::EOPNOTSUPP),
    }
    // Verify that the header length is valid. All messages from userland start with a
    // struct rt_msghdr.
    if hdr.rtm_hdrlen == 0 {
        // old client
        hdr.rtm_hdrlen = RT_MSGHDR as u16;
    }
    let hdrlen = usize::from(hdr.rtm_hdrlen);
    if hdrlen < RT_MSGHDR || len < hdrlen {
        return Err(Errno::EINVAL);
    }

    let pid = curpid::<G>();
    hdr.rtm_pid = pid;

    // Verify that the caller has the appropriate privilege; RTM_GET is the only operation
    // the non-superuser is allowed.
    if hdr.rtm_type != RTM_GET && G::with_curproc(|p| p.is_none_or(|p| suser(p).is_err())) {
        return Err(Errno::EACCES);
    }
    let tableid = u32::from(hdr.rtm_tableid);
    if !rtable_exists(tableid) {
        if hdr.rtm_type == RTM_ADD {
            rtable_add(tableid)?;
        } else {
            return Err(Errno::EINVAL);
        }
    }

    // Do not let userland play with kernel-only flags.
    if hdr.rtm_flags as u32 & (RTF_LOCAL | RTF_BROADCAST) != 0 {
        return Err(Errno::EINVAL);
    }

    // make sure that kernel-only bits are not set
    hdr.rtm_priority &= RTP_MASK;
    hdr.rtm_flags &= !((RTF_DONE | RTF_CLONED | RTF_CACHED) as i32);
    hdr.rtm_fmask &= RTF_FMASK as i32;

    let prio = if hdr.rtm_priority != 0 {
        if hdr.rtm_priority > RTP_MAX || hdr.rtm_priority == RTP_LOCAL {
            return Err(Errno::EINVAL);
        }
        hdr.rtm_priority
    } else if hdr.rtm_type != RTM_ADD {
        RTP_ANY
    } else if hdr.rtm_flags as u32 & RTF_STATIC != 0 {
        0
    } else {
        RTP_DEFAULT
    };

    let mut af = AF_UNSPEC;
    let mut rt: Option<PoolArc<Rtentry>> = None;
    let mut reported = false;
    let error = {
        let mut info = RtAddrinfo::new();
        info.rti_addrs = hdr.rtm_addrs;
        rtm_xaddrs(&rtm[hdrlen..len], &mut info)?;

        info.rti_flags = hdr.rtm_flags as u32;

        if hdr.rtm_type != RTM_SOURCE
            && hdr.rtm_type != RTM_PROPOSAL
            && (info.rti_info[RTAX_DST].is_none_or(|d| d.family() >= AF_MAX)
                || info.rti_info[RTAX_GATEWAY].is_some_and(|g| g.family() >= AF_MAX)
                || info.rti_info[RTAX_GENMASK].is_some())
        {
            return Err(Errno::EINVAL);
        }
        // MPLS: info.rti_mpls = rtm->rtm_mpls, not configured.

        if info.rti_info[RTAX_GATEWAY].is_some_and(|g| g.family() == AF_LINK)
            && info.rti_flags & RTF_CLONING == 0
        {
            info.rti_flags |= RTF_LLINFO;
        }

        if let Some(d) = info.rti_info[RTAX_DST] {
            af = d.family();
        }

        if hdr.rtm_type == RTM_PROPOSAL {
            // Validate RTM_PROPOSAL and pass it along or error out.
            if rtm_validate_proposal(&info).is_err() {
                return Err(Errno::EINVAL);
            }
            // If this is a solicitation proposal forward request to all interfaces. Most
            // handlers will ignore it but at least umb(4) will send a response to this
            // event.
            if hdr.rtm_priority == RTP_PROPOSAL_SOLICIT {
                let nw = NETLOCK.write();
                let net = net_held(&nw);
                for ifp in &IFNETLIST {
                    if let Some(d) = ifp.if_driver.get() {
                        d.rtrequest(ifp, RTM_PROPOSAL, None, &net);
                    }
                }
                drop(nw);
            }
            Ok(())
        } else if hdr.rtm_type == RTM_SOURCE {
            let Some(ifa) = info.rti_info[RTAX_IFA] else {
                return Err(Errno::EINVAL);
            };
            let nw = NETLOCK.write();
            let r = rt_setsource(tableid, ifa, &net_held(&nw));
            drop(nw);
            r?;
            Ok(())
        } else {
            let r = rtm_output(&mut hdr, &mut rt, &mut info, prio, tableid);
            if r.is_ok() {
                let ty = hdr.rtm_type;
                let seq = hdr.rtm_seq;
                let Some(r) = rt.as_deref() else {
                    kpanic(format_args!("route_output: no route answered"));
                };
                let nr = NETLOCK.read();
                let report = rtm_report(r, ty, seq, tableid, pid, &net_held(&nr));
                drop(nr);
                rtm = report;
                reported = true;
            }
            r
        }
    };

    // rtfree(rt)
    drop(rt);
    // The C edits the request in place; its header copy goes back before the answer is
    // sent (the report is a message of its own).
    let len = if reported {
        usize::from(u16_at(&rtm, 0))
    } else {
        hal::pod::put(&mut rtm, 0, &hdr);
        len
    };
    match error {
        Err(e) => hdr_edit::<RtMsghdr>(&mut rtm, |h| h.rtm_errno = e as i32),
        Ok(()) => hdr_edit::<RtMsghdr>(&mut rtm, |h| h.rtm_flags |= RTF_DONE as i32),
    }

    // Check to see if we don't want our own messages.
    if !useloopback && RTP_COUNT.load(Ordering::Relaxed) == 0 {
        // no other listener and no loopback of messages
        return error;
    }
    let m = if m_copyback(&mut m, 0, &rtm[..len], M_NOWAIT).is_err() {
        drop(m);
        None
    } else {
        let plen = m.m_pkthdr().len.get();
        if plen > len as i32 {
            m_adj(&m, len as i32 - plen);
        }
        Some(m)
    };
    drop(rtm);
    if let Some(m) = m {
        route_input::<G>(m, Some(so), af);
    }

    error
}

/// `rtm_output(rtm, prt, info, prio, tableid)`: the request of header `rtm` with the
/// addresses `info` carried out at priority `prio` in table `tableid`; the route it concerns
/// left in `*prt` (also on some errors, as the C's).
fn rtm_output(
    rtm: &mut RtMsghdr,
    prt: &mut Option<PoolArc<Rtentry>>,
    info: &mut RtAddrinfo<'_>,
    prio: u8,
    tableid: u32,
) -> Result<(), Errno> {
    let mut ifp: Option<&'static Ifnet> = None;
    let mut rt: Option<PoolArc<Rtentry>> = prt.take();
    let Some(dst) = info.rti_info[RTAX_DST] else {
        return Err(Errno::EINVAL);
    };

    let error = 'req: {
        match rtm.rtm_type {
            RTM_ADD => {
                if info.rti_info[RTAX_GATEWAY].is_none() {
                    break 'req Err(Errno::EINVAL);
                }

                rt = rtable_match(tableid, dst, None);
                if let Err(e) = route_arp_conflict(rt.as_deref(), info) {
                    rt = None;
                    break 'req Err(e);
                }

                // We cannot go through a delete/create/insert cycle for cached route because
                // this can lead to races in the receive path. Instead we update the L2 cache.
                if let Some(r) = rt.as_deref()
                    && r.rt_flags.isset(RTF_CACHED)
                {
                    ifp = if_get(r.rt_ifidx.get());
                    let Some(i) = ifp else {
                        rt = None;
                        break 'req Err(Errno::ESRCH);
                    };
                    break 'req rtm_output_change(rtm, r, info, tableid, i, false);
                }

                rt = None;

                let nw = NETLOCK.write();
                let net = net_held(&nw);
                if let Err(e) = rtm_getifa(info, tableid, &net) {
                    break 'req Err(e);
                }
                let error = rtrequest(RTM_ADD, info, prio, Some(&mut rt), tableid, &net);
                drop(nw);
                if error.is_ok()
                    && let Some(r) = rt.as_deref()
                {
                    rtm_setmetrics(rtm.rtm_inits, &rtm.rtm_rmx, &r.rt_rmx);
                }
                error
            }
            RTM_DELETE => {
                rt = rtable_lookup(
                    tableid,
                    dst,
                    info.rti_info[RTAX_NETMASK],
                    info.rti_info[RTAX_GATEWAY],
                    prio,
                );
                let Some(r) = rt.as_deref() else {
                    break 'req Err(Errno::ESRCH);
                };

                // If we got multipath routes, we require users to specify a matching
                // gateway.
                if r.rt_flags.isset(RTF_MPATH) && info.rti_info[RTAX_GATEWAY].is_none() {
                    break 'req Err(Errno::ESRCH);
                }

                ifp = if_get(r.rt_ifidx.get());
                let Some(i) = ifp else {
                    rt = None;
                    break 'req Err(Errno::ESRCH);
                };

                // Invalidate the cache of automagically created and referenced L2 entries to
                // make sure that ``rt_gwroute'' pointer stays valid for other CPUs.
                if r.rt_flags.isset(RTF_CACHED) {
                    let nw = NETLOCK.write();
                    let net = net_held(&nw);
                    if let Some(d) = i.if_driver.get() {
                        d.rtrequest(i, RTM_INVALIDATE, Some(r), &net);
                    }
                    // Reset the MTU of the gateway route.
                    let _ = rtable_walk(tableid, r.dest_view().family(), None, |x, id| {
                        route_cleargateway(x, r, id);
                        Ok(())
                    });
                    drop(nw);
                    break 'req Ok(());
                }

                // Make sure that local routes are only modified by the kernel.
                if r.rt_flags.isset(RTF_LOCAL | RTF_BROADCAST) {
                    break 'req Err(Errno::EINVAL);
                }

                rt = None;

                let nw = NETLOCK.write();
                let net = net_held(&nw);
                let error = rtrequest_delete(info, prio, i, Some(&mut rt), tableid, &net);
                drop(nw);
                error
            }
            RTM_CHANGE => {
                rt = rtable_lookup(
                    tableid,
                    dst,
                    info.rti_info[RTAX_NETMASK],
                    info.rti_info[RTAX_GATEWAY],
                    prio,
                );
                // If we got multipath routes, we require users to specify a matching
                // gateway.
                if rt.as_deref().is_some_and(|r| r.rt_flags.isset(RTF_MPATH))
                    && info.rti_info[RTAX_GATEWAY].is_none()
                {
                    rt = None;
                }

                // If RTAX_GATEWAY is the argument we're trying to change, try to find a
                // compatible route.
                if rt.is_none() && info.rti_info[RTAX_GATEWAY].is_some() {
                    rt = rtable_lookup(tableid, dst, info.rti_info[RTAX_NETMASK], None, prio);
                    // Ensure we don't pick a multipath one.
                    if rt.as_deref().is_some_and(|r| r.rt_flags.isset(RTF_MPATH)) {
                        rt = None;
                    }
                }

                let Some(r) = rt.as_deref() else {
                    break 'req Err(Errno::ESRCH);
                };

                // Make sure that local routes are only modified by the kernel.
                if r.rt_flags.isset(RTF_LOCAL | RTF_BROADCAST) {
                    break 'req Err(Errno::EINVAL);
                }

                ifp = if_get(r.rt_ifidx.get());
                let Some(i) = ifp else {
                    rt = None;
                    break 'req Err(Errno::ESRCH);
                };

                // RTM_CHANGE needs a perfect match.
                let plen = rtable_satoplen(dst.family(), info.rti_info[RTAX_NETMASK]);
                if plen != Ok(r.rt_plen()) {
                    break 'req Err(Errno::ESRCH);
                }

                let newgate = info.rti_info[RTAX_GATEWAY].is_some_and(|g| !r.gateway_is(g.bytes()));
                // Check reachable gateway before changing the route. New gateway could
                // require new ifaddr, ifp; flags may also be different; ifp may be specified
                // by ll sockaddr when protocol address is ambiguous.
                if newgate || info.rti_info[RTAX_IFP].is_some() || info.rti_info[RTAX_IFA].is_some()
                {
                    let nw = NETLOCK.write();
                    let net = net_held(&nw);
                    if let Err(e) = rtm_getifa(info, tableid, &net) {
                        break 'req Err(e);
                    }
                    if let Some(ifa) = info.rti_ifa
                        && !r.ifa().is_some_and(|o| core::ptr::eq(o, ifa))
                    {
                        if let Some(d) = i.if_driver.get() {
                            d.rtrequest(i, RTM_DELETE, Some(r), &net);
                        }
                        if let Some(o) = r.ifa() {
                            ifafree(o);
                        }

                        r.rt_ifa.set(Some(ifaref(ifa)));
                        if let Some(ifp) = ifa.ifa_ifp.get() {
                            r.rt_ifidx.set(ifp.if_index.get());
                            // recheck link state after ifp change
                            let _ = rt_if_linkstate_change(r, ifp, tableid);
                        }
                    }
                    drop(nw);
                }
                rtm_output_change(rtm, r, info, tableid, i, newgate)
            }
            RTM_GET => {
                rt = rtable_lookup(
                    tableid,
                    dst,
                    info.rti_info[RTAX_NETMASK],
                    info.rti_info[RTAX_GATEWAY],
                    prio,
                );
                if rt.is_none() {
                    break 'req Err(Errno::ESRCH);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    };

    if_put(ifp);
    *prt = rt;
    error
}

/// `rtm_output`'s `change:` label: the gateway, the flags the user may toggle, the metrics,
/// the label and the locks of route `rt` (of interface `ifp`) changed as `rtm` and `info`
/// say; `newgate` when the gateway is new.
fn rtm_output_change(
    rtm: &mut RtMsghdr,
    rt: &Rtentry,
    info: &RtAddrinfo<'_>,
    tableid: u32,
    ifp: &'static Ifnet,
    newgate: bool,
) -> Result<(), Errno> {
    if let Some(gw) = info.rti_info[RTAX_GATEWAY] {
        // When updating the gateway, make sure it is valid.
        if !newgate && rt.with_gateway(|g| g.map(|g| g[1])) != Some(gw.family()) {
            return Err(Errno::EINVAL);
        }

        let nw = NETLOCK.write();
        let r = rt_setgate(rt, gw, tableid, &net_held(&nw));
        drop(nw);
        r?;
    }
    // MPLS: rt_mpls_set (RTF_MPLS) or rt_mpls_clear (a new gateway, or RTF_MPLS in the
    // mask), not configured.
    // BFD: bfdset (RTF_BFD) or bfdclear (RTF_BFD in the mask only), not configured.

    let nw = NETLOCK.write();
    let net = net_held(&nw);
    // Hack to allow some flags to be toggled.
    if rtm.rtm_fmask != 0 {
        // MPLS flag it is set by rt_mpls_set()
        rtm.rtm_fmask &= !(RTF_MPLS as i32);
        rtm.rtm_flags &= !(RTF_MPLS as i32);
        let fmask = rtm.rtm_fmask as u32;
        rt.rt_flags
            .set((rt.rt_flags.get() & !fmask) | (rtm.rtm_flags as u32 & fmask));
    }
    rtm_setmetrics(rtm.rtm_inits, &rtm.rtm_rmx, &rt.rt_rmx);

    if let Some(d) = ifp.if_driver.get() {
        d.rtrequest(ifp, RTM_ADD, Some(rt), &net);
    }

    if let Some(l) = info.rti_info[RTAX_LABEL] {
        let name = l.bytes().get(2..).unwrap_or(&[]);
        rtlabel_unref(rt.rt_labelid.get());
        rt.rt_labelid.set(rtlabel_name2id(name));
    }
    if let Some(dst) = info.rti_info[RTAX_DST] {
        if_group_routechange(dst.bytes(), info.rti_info[RTAX_NETMASK].map(|m| m.bytes()));
    }
    let locks = rt.rt_locks();
    locks.set(locks.get() & !rtm.rtm_inits);
    locks.set(locks.get() | (rtm.rtm_inits & rtm.rtm_rmx.rmx_locks));
    drop(nw);
    Ok(())
}

/// `ifa_ifwithroute(flags, dst, gateway, rtableid)`: the interface address a route to `dst`
/// through `gateway` should use, under the net lock `net`.
pub fn ifa_ifwithroute(
    flags: u32,
    dst: SockaddrView<'_>,
    gateway: SockaddrView<'_>,
    rtableid: u32,
    net: &NetHeld<'_>,
) -> Option<&'static Ifaddr> {
    let mut ifa = if flags & RTF_GATEWAY == 0 {
        // If we are adding a route to an interface, and the interface is a pt to pt link we
        // should search for the destination as our clue to the interface. Otherwise we can
        // use the local address.
        let mut ifa = None;
        if flags & RTF_HOST != 0 {
            ifa = ifa_ifwithdstaddr(net, dst.bytes(), rtableid);
        }
        if ifa.is_none() {
            ifa = ifa_ifwithaddr(net, gateway.bytes(), rtableid);
        }
        ifa
    } else {
        // If we are adding a route to a remote net or host, the gateway may still be on the
        // other end of a pt to pt link.
        ifa_ifwithdstaddr(net, gateway.bytes(), rtableid)
    };
    if ifa.is_none() {
        if gateway.family() == AF_LINK {
            let ifp = if_get(sdl_index(gateway));
            if let Some(i) = ifp {
                ifa = ifaof_ifpforaddr(net, dst.bytes(), i);
            }
            if_put(ifp);
        } else {
            let rt = rtalloc(gateway, RT_RESOLVE, rtable_l2(rtableid), net);
            ifa = rt.as_deref().and_then(Rtentry::ifa);
            drop(rt);
        }
    }
    let ifa = ifa?;
    if ifa.ifa_addr.read(net).map(|a| a.family()) != Some(dst.family()) {
        let oifa = ifa;
        return Some(
            ifa.ifa_ifp
                .get()
                .and_then(|ifp| ifaof_ifpforaddr(net, dst.bytes(), ifp))
                .unwrap_or(oifa),
        );
    }
    Some(ifa)
}

/// `rtm_getifa(info, rtid)`: the interface address of a request (`info->rti_ifa`), found
/// from its interface, address, gateway or destination; `ENETUNREACH` without one. The
/// address found lives while the net lock `net` is held.
fn rtm_getifa(info: &mut RtAddrinfo<'_>, rtid: u32, net: &NetHeld<'_>) -> Result<(), Errno> {
    // ifp may be specified by sockaddr_dl when protocol address is ambiguous
    let ifp = info.rti_info[RTAX_IFP].and_then(|sdl| if_get(sdl_index(sdl)));

    // IPSEC: if the destination is a PF_KEY address, we'll look for the existence of a
    // encap interface number or address in the options list of the gateway. By default,
    // we'll return enc0.
    if info.rti_info[RTAX_DST].is_some_and(|d| d.family() == PF_KEY)
        && let Some(i) = ipsec()
    {
        info.rti_ifa = i.enc_getifa(rtid, 0);
    }

    if info.rti_ifa.is_none()
        && let Some(a) = info.rti_info[RTAX_IFA]
    {
        info.rti_ifa = ifa_ifwithaddr(net, a.bytes(), rtid);
    }

    if info.rti_ifa.is_none() {
        let sa = info.rti_info[RTAX_IFA]
            .or(info.rti_info[RTAX_GATEWAY])
            .or(info.rti_info[RTAX_DST]);

        if let (Some(sa), Some(i)) = (sa, ifp) {
            info.rti_ifa = ifaof_ifpforaddr(net, sa.bytes(), i);
        } else if let (Some(d), Some(g)) = (info.rti_info[RTAX_DST], info.rti_info[RTAX_GATEWAY]) {
            info.rti_ifa = ifa_ifwithroute(info.rti_flags, d, g, rtid, net);
        } else if let Some(sa) = sa {
            info.rti_ifa = ifa_ifwithroute(info.rti_flags, sa, sa, rtid, net);
        }
    }

    if_put(ifp);

    if info.rti_ifa.is_none() {
        return Err(Errno::ENETUNREACH);
    }
    Ok(())
}

/// `route_cleargateway(rt, nhrt, rtableid)`: the MTU of a gateway route through `nhrt`
/// forgotten, unless locked.
fn route_cleargateway(rt: &Rtentry, nhrt: &Rtentry, _rtableid: u32) {
    if rt.rt_flags.isset(RTF_GATEWAY)
        && rt_getll(rt, |gw| gw.is_some_and(|g| core::ptr::eq(g, nhrt)))
        && rt.rt_locks().get() & RTV_MTU == 0
    {
        rt.rt_mtu().store(0, Ordering::Relaxed);
    }
}

/// `route_arp_conflict(rt, info)`: whether the user's request to insert an ARP entry
/// conflicts with existing ones (`EEXIST`); only two entries are allowed for a given IP
/// address, a private one (priv) and a public one (pub), the second forced multipath.
fn route_arp_conflict(rt: Option<&Rtentry>, info: &mut RtAddrinfo<'_>) -> Result<(), Errno> {
    let proxy = info.rti_flags & RTF_ANNOUNCE != 0;

    if info.rti_flags & RTF_LLINFO == 0
        || info.rti_info[RTAX_DST].is_none_or(|d| d.family() != AF_INET)
    {
        return Ok(());
    }

    let Some(rt) = rt else {
        return Ok(());
    };
    if !rt.rt_flags.isset(RTF_LLINFO) {
        return Ok(());
    }

    // If the entry is cached, it can be updated.
    if rt.rt_flags.isset(RTF_CACHED) {
        return Ok(());
    }

    // Same destination, not cached and both "priv" or "pub" conflict. If a second entry
    // exists, it always conflict.
    if rt.rt_flags.isset(RTF_ANNOUNCE) == proxy || rt.rt_flags.isset(RTF_MPATH) {
        return Err(Errno::EEXIST);
    }

    // No conflict but an entry exist so we need to force mpath.
    info.rti_flags |= RTF_MPATH;
    Ok(())
}

/// `rtm_setmetrics(which, in, out)`: the metrics `which` names of `in` set on `out` (an
/// expiry converted from wall-clock time to uptime).
pub fn rtm_setmetrics(which: u32, in_: &RtMetrics, out: &RtKmetrics) {
    if which & RTV_MTU != 0 {
        out.rmx_mtu.store(in_.rmx_mtu, Ordering::Relaxed);
    }
    if which & RTV_EXPIRE != 0 {
        let mut expire = in_.rmx_expire;
        if expire != 0 {
            expire -= crate::tc::gettime();
            expire += crate::tc::getuptime();
        }

        out.rmx_expire.store(expire, Ordering::Relaxed);
    }
}

/// `rtm_getmetrics(rt, out)`: route `rt`'s metrics for userland (an expiry converted from
/// uptime to wall-clock time; a timer's when the route has none).
pub fn rtm_getmetrics(rt: &Rtentry, out: &mut RtMetrics) {
    let in_ = &rt.rt_rmx;

    let mut expire = in_.rmx_expire.load(Ordering::Relaxed);
    if expire == 0 {
        expire = rt_timer_get_expire(rt);
    }
    if expire != 0 {
        expire -= crate::tc::getuptime();
        expire += crate::tc::gettime();
    }

    *out = RtMetrics::default();
    out.rmx_locks = in_.rmx_locks.get();
    out.rmx_mtu = in_.rmx_mtu.load(Ordering::Relaxed);
    out.rmx_expire = expire;
    out.rmx_pksent = in_.rmx_pksent.load(Ordering::Relaxed);
}

/// `rtm_xaddrs(cp, cplim, rtinfo)`: the addresses `rtinfo->rti_addrs` names split out of
/// `cp` (each a view bounded by its `sa_len` and `cp`'s end), then each checked against
/// its kind: a family that suits it (`EAFNOSUPPORT`), a size that fits the family and a
/// string NUL-terminated within it (`EINVAL`).
pub fn rtm_xaddrs<'a>(cp: &'a [u8], rtinfo: &mut RtAddrinfo<'a>) -> Result<(), Errno> {
    // Parse address bits, split address storage in chunks, and set info pointers. Use
    // sa_len for traversing the memory and check that we stay within in the limit.
    rtinfo.rti_info = [None; RTAX_MAX];
    let mut off = 0usize;
    for i in 0..32 {
        if (rtinfo.rti_addrs as u32) & (1u32 << i) == 0 {
            continue;
        }
        if i >= RTAX_MAX || off + SOCKLEN > cp.len() {
            return Err(Errno::EINVAL);
        }
        let sa_len = usize::from(cp[off]);
        if off + sa_len > cp.len() {
            return Err(Errno::EINVAL);
        }
        let Some(sa) = SockaddrView::new(&cp[off..]) else {
            return Err(Errno::EINVAL);
        };
        rtinfo.rti_info[i] = Some(sa);
        off += roundup_long(sa_len);
    }

    // Check that the address family is suitable for the route address type. Check that each
    // address has a size that fits its family and its length is within the size. Strings
    // within addresses must be NUL terminated.
    for i in 0..RTAX_MAX {
        let Some(sa) = rtinfo.rti_info[i] else {
            continue;
        };
        let mut maxlen = 0usize;
        let mut size = 0usize;
        match i {
            RTAX_DST | RTAX_GATEWAY | RTAX_SRC => match sa.family() {
                AF_INET => size = SIN_LEN,
                AF_LINK => size = SDL_LEN,
                AF_INET6 => size = SIN6_LEN,
                // MPLS: AF_MPLS, sizeof(struct sockaddr_mpls), not configured.
                _ => {}
            },
            RTAX_IFP => {
                if sa.family() != AF_LINK {
                    return Err(Errno::EAFNOSUPPORT);
                }
                // XXX Should be sizeof(struct sockaddr_dl), but route(8) has a bug and
                // provides less memory. arp(8) has another bug and uses sizeof pointer.
                size = 4;
            }
            RTAX_IFA => match sa.family() {
                AF_INET => size = SIN_LEN,
                AF_INET6 => size = SIN6_LEN,
                _ => return Err(Errno::EAFNOSUPPORT),
            },
            RTAX_LABEL => {
                if sa.family() != AF_UNSPEC {
                    return Err(Errno::EAFNOSUPPORT);
                }
                maxlen = RTLABEL_LEN;
                size = size_of::<SockaddrRtlabel>();
            }
            // BFD: RTAX_BFD (AF_UNSPEC, sizeof(struct sockaddr_bfd)), not configured.
            RTAX_DNS => {
                // more validation in rtm_validate_proposal
                if usize::from(sa.len()) > size_of::<SockaddrRtdns>() {
                    return Err(Errno::EINVAL);
                }
                if usize::from(sa.len()) < 2 {
                    return Err(Errno::EINVAL);
                }
                match sa.family() {
                    AF_INET | AF_INET6 => {}
                    _ => return Err(Errno::EAFNOSUPPORT),
                }
            }
            RTAX_STATIC => {
                match sa.family() {
                    AF_INET | AF_INET6 => {}
                    _ => return Err(Errno::EAFNOSUPPORT),
                }
                maxlen = RTSTATIC_LEN;
                size = size_of::<SockaddrRtstatic>();
            }
            RTAX_SEARCH => {
                if sa.family() != AF_UNSPEC {
                    return Err(Errno::EAFNOSUPPORT);
                }
                maxlen = RTSEARCH_LEN;
                size = size_of::<SockaddrRtsearch>();
            }
            _ => {}
        }
        // memory for the full struct must be provided
        if size != 0 && usize::from(sa.len()) < size {
            return Err(Errno::EINVAL);
        }
        if maxlen != 0 {
            // this should not happen
            if 2 + maxlen > size {
                return Err(Errno::EINVAL);
            }
            // strings must be NUL terminated within the struct
            let data = &sa.bytes()[2..2 + maxlen];
            let len = data.iter().position(|&c| c == 0).unwrap_or(maxlen);
            if len >= maxlen || 2 + len >= usize::from(sa.len()) {
                return Err(Errno::EINVAL);
            }
            // The C breaks out of the loop here: the addresses after the first string
            // address are not checked.
            break;
        }
    }
    Ok(())
}

/// The bytes of address `sa` a message carries: its `sa_len` of them (fewer when its view
/// holds fewer, an address shorter than its header).
fn sa_bytes<'a>(sa: &SockaddrView<'a>) -> &'a [u8] {
    let b = sa.bytes();
    &b[..usize::from(sa.len()).min(b.len())]
}

/// The addresses of `rtinfo` written after a header of `hlen` bytes in `cp`, each rounded up
/// to a long with zeros (`rti_addrs` set): the bytes written.
fn put_addrs(rtinfo: &mut RtAddrinfo<'_>, cp: &mut [u8], hlen: usize) -> usize {
    let mut len = hlen;
    for (i, sa) in rtinfo.rti_info.iter().enumerate() {
        let Some(sa) = sa else {
            continue;
        };
        rtinfo.rti_addrs |= 1 << i;
        let b = sa_bytes(sa);
        let dlen = roundup_long(usize::from(sa.len()));
        cp[len..len + b.len()].copy_from_slice(b);
        cp[len + b.len()..len + dlen].fill(0);
        len += dlen;
    }
    len
}

/// `rtm_msg1(type, rtinfo)`: a routing message of `type` in a new mbuf (a cluster when it is
/// longer than `MHLEN`): the header for its type, zeroed, followed by the addresses of
/// `rtinfo` (their bits set in `rti_addrs`); `None` without memory. The message fits its one
/// mbuf, so the addresses are written in place (the C's `m_copyback`).
fn rtm_msg1(ty: u8, rtinfo: Option<&mut RtAddrinfo<'_>>) -> Option<MbufBox> {
    let hlen = match ty {
        RTM_DELADDR | RTM_NEWADDR => IFA_MSGHDR,
        RTM_IFINFO => IF_MSGHDR,
        RTM_IFANNOUNCE => size_of::<IfAnnouncemsghdr>(),
        // BFD: RTM_BFD, sizeof(struct bfd_msghdr), not configured.
        RTM_80211INFO => size_of::<IfIeee80211Msghdr>(),
        _ => RT_MSGHDR,
    };
    let mut len = hlen;
    if let Some(info) = &rtinfo {
        for sa in info.rti_info.iter().flatten() {
            len += roundup_long(usize::from(sa.len()));
        }
    }
    if len > MCLBYTES {
        kpanic(format_args!("rtm_msg1"));
    }
    let mut m = m_gethdr(M_DONTWAIT, MT_DATA)?;
    if len > MHLEN {
        mclget(&mut m, M_DONTWAIT);
        if m.flags() & M_EXT == 0 {
            return None;
        }
    }
    m.m_pkthdr().len.set(len as i32);
    m.m_len().set(len as u32);
    m.m_pkthdr().ph_ifidx.set(0);
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("rtm_msg1: a read-only new mbuf"));
    };
    d.fill(0);
    let len = match rtinfo {
        Some(info) => put_addrs(info, d, hlen),
        None => hlen,
    };
    hdr_edit::<MsgHead>(d, |h| {
        h.msglen = len as u16;
        h.hdrlen = hlen as u16;
        h.version = RTM_VERSION;
        h.ty = ty;
    });
    Some(m)
}

/// The fields every routing message starts with (`rtm_msglen`, `rtm_version`, `rtm_type`,
/// `rtm_hdrlen`: the same in `if_msghdr`, `ifa_msghdr`, `if_announcemsghdr`, ...).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MsgHead {
    /// `rtm_msglen`.
    msglen: u16,
    /// `rtm_version`.
    version: u8,
    /// `rtm_type`.
    ty: u8,
    /// `rtm_hdrlen`.
    hdrlen: u16,
}

hal::abi_pod!(MsgHead {
    msglen: u16,
    version: u8,
    ty: u8,
    hdrlen: u16,
});

/// `rtm_msg2(type, vers, rtinfo, cp, w)`: the length of a routing message of `type` with
/// the addresses of `rtinfo` (their bits set in `rti_addrs`), written into `cp` when given,
/// or into the walk's buffer when the walk has room for it (the walk's need counted; its
/// buffer grown, or the walk's copy-out stopped without memory).
fn rtm_msg2(
    ty: u8,
    rtinfo: &mut RtAddrinfo<'_>,
    cp: Option<&mut [u8]>,
    w: Option<&mut Walkarg>,
) -> usize {
    rtinfo.rti_addrs = 0;
    let hlen = match ty {
        RTM_DELADDR | RTM_NEWADDR => IFA_MSGHDR,
        RTM_IFINFO => IF_MSGHDR,
        _ => RT_MSGHDR,
    };
    let mut len = hlen;
    for (i, sa) in rtinfo.rti_info.iter().enumerate() {
        let Some(sa) = sa else {
            continue;
        };
        rtinfo.rti_addrs |= 1 << i;
        len += roundup_long(usize::from(sa.len()));
    }
    // align message length to the next natural boundary
    let len = align(len);

    if let Some(cp) = cp {
        let n = put_addrs(rtinfo, cp, hlen);
        cp[n..len].fill(0);
        msg2_head(cp, ty, len, hlen);
        return len;
    }
    let Some(w) = w else {
        return len;
    };
    w.w_needed += len;
    if w.w_needed <= w.w_given && w.w_where != 0 {
        if w.w_tmem.as_ref().map_or(0, |t| t.len()) < len {
            w.w_tmem = None;
            w.w_tmem = MArray::new(M_RTABLE, M_NOWAIT, len, |_| 0u8);
        }
        match w.w_tmem.as_mut() {
            Some(t) => {
                // clear the message header
                t[..hlen].fill(0);
                let n = put_addrs(rtinfo, t, hlen);
                t[n..len].fill(0);
                msg2_head(t, ty, len, hlen);
            }
            None => w.w_where = 0,
        }
    }
    len
}

/// The four fields `rtm_msg2` stamps on the message it wrote at `cp`.
fn msg2_head(cp: &mut [u8], ty: u8, len: usize, hlen: usize) {
    hdr_edit::<MsgHead>(cp, |h| {
        h.version = RTM_VERSION;
        h.ty = ty;
        h.msglen = len as u16;
        h.hdrlen = hlen as u16;
    });
}

/// `rtm_send(rt, cmd, error, rtableid)`: the listeners told of a change `cmd` to route `rt`
/// (`rtm_miss` of its addresses), under the net lock `net`.
pub fn rtm_send(rt: &Rtentry, cmd: u8, error: i32, rtableid: u32, net: &NetHeld<'_>) {
    let mut a = RouteAddrs::of(rt, rtableid, false, net);
    let info = a.info(rt, !rt.rt_flags.isset(RTF_HOST));

    rtm_miss(
        cmd,
        &info,
        rt.rt_flags.get(),
        rt.rt_priority.get(),
        rt.rt_ifidx.get(),
        error,
        rtableid,
    );
}

/// `rtm_miss(type, rtinfo, flags, prio, ifidx, error, tableid)`: a message from the routing
/// socket indicating that a redirect has occurred, a routing lookup has failed, or that a
/// protocol has detected timeouts to a particular destination.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's rtm_miss and its arguments"
)]
pub fn rtm_miss(
    ty: u8,
    rtinfo: &RtAddrinfo<'_>,
    flags: u32,
    prio: u8,
    ifidx: u32,
    error: i32,
    tableid: u32,
) {
    if !listeners() {
        return;
    }
    let mut info = *rtinfo;
    let Some(mut m) = rtm_msg1(ty, Some(&mut info)) else {
        return;
    };
    let af = info.rti_info[RTAX_DST].map_or(AF_UNSPEC, |d| d.family());
    if let Some(d) = m.data_mut() {
        hdr_edit::<RtMsghdr>(d, |h| {
            h.rtm_flags = (RTF_DONE | flags) as i32;
            h.rtm_priority = prio;
            h.rtm_errno = error;
            h.rtm_tableid = tableid as u16;
            h.rtm_addrs = info.rti_addrs;
            h.rtm_index = ifidx as u16;
        });
    }
    route_input_any(m, af);
}

/// `rtm_ifchg(ifp)`: a message from the routing socket indicating that the status of a
/// network interface has changed.
pub fn rtm_ifchg(ifp: &Ifnet) {
    if !listeners() {
        return;
    }
    let sadl = sadl_copy(ifp);
    let mut info = RtAddrinfo::new();
    info.rti_info[RTAX_IFP] = sadl.as_ref().map(SaCopy::view);
    let Some(mut m) = rtm_msg1(RTM_IFINFO, Some(&mut info)) else {
        return;
    };
    let mut data = IfData::default();
    if_getdata(ifp, &mut data);
    if let Some(d) = m.data_mut() {
        hdr_edit::<IfMsghdr>(d, |h| {
            h.ifm_index = ifp.if_index.get() as u16;
            h.ifm_tableid = ifp.if_rdomain.get() as u16;
            h.ifm_flags = ifp.if_flags.get();
            h.ifm_xflags = ifp.if_xflags.get();
            h.ifm_data = data;
            h.ifm_addrs = info.rti_addrs;
        });
    }
    route_input_any(m, AF_UNSPEC);
}

/// `rtm_addr(cmd, ifa)`: a message from the routing socket indicating a network interface
/// has had addresses associated with it, read under the net lock `net`. If we ever reverse
/// the logic and replace messages TO the routing socket indicate a request to configure
/// interfaces, then it will be unnecessary as the routing socket will automatically
/// generate copies of it.
pub fn rtm_addr(cmd: u8, ifa: &Ifaddr, net: &NetHeld<'_>) {
    if !listeners() {
        return;
    }
    let Some(ifp) = ifa.ifa_ifp.get() else {
        kpanic(format_args!("rtm_addr: an address without its interface"));
    };

    let addr = ifa.ifa_addr.read(net);
    let mask = ifa.ifa_netmask.read(net);
    let brd = ifa.ifa_dstaddr.read(net);
    let sadl = sadl_copy(ifp);
    let mut info = RtAddrinfo::new();
    info.rti_info[RTAX_IFA] = addr.as_ref().and_then(|a| SockaddrView::new(a));
    info.rti_info[RTAX_IFP] = sadl.as_ref().map(SaCopy::view);
    info.rti_info[RTAX_NETMASK] = mask.as_ref().and_then(|a| SockaddrView::new(a));
    info.rti_info[RTAX_BRD] = brd.as_ref().and_then(|a| SockaddrView::new(a));
    let Some(mut m) = rtm_msg1(cmd, Some(&mut info)) else {
        return;
    };
    if let Some(d) = m.data_mut() {
        hdr_edit::<IfaMsghdr>(d, |h| {
            h.ifam_index = ifp.if_index.get() as u16;
            h.ifam_metric = ifa.ifa_metric.get();
            h.ifam_flags = ifa.ifa_flags.get() as i32;
            h.ifam_addrs = info.rti_addrs;
            h.ifam_tableid = ifp.if_rdomain.get() as u16;
        });
    }

    route_input_any(m, addr.map_or(AF_UNSPEC, |a| a.family()));
}

/// `rtm_ifannounce(ifp, what)`: routing socket messages indicating network interface
/// arrival and departure.
pub fn rtm_ifannounce(ifp: &Ifnet, what: u16) {
    if !listeners() {
        return;
    }
    let Some(mut m) = rtm_msg1(RTM_IFANNOUNCE, None) else {
        return;
    };
    let mut name = [0u8; hal::sys::net_if::IFNAMSIZ];
    ifp.if_xname.with_str(|n| {
        let k = n.len().min(name.len() - 1);
        name[..k].copy_from_slice(&n[..k]);
    });
    if let Some(d) = m.data_mut() {
        hdr_edit::<IfAnnouncemsghdr>(d, |h| {
            h.ifan_index = ifp.if_index.get() as u16;
            h.ifan_name = name;
            h.ifan_what = what;
        });
    }
    route_input_any(m, AF_UNSPEC);
}

/// `rtm_80211info(ifp, ifie)`: routing socket messages indicating the state of an ieee80211
/// interface (net80211 is not configured: no caller).
pub fn rtm_80211info(ifp: &Ifnet, ifie: &IfIeee80211Data) {
    if !listeners() {
        return;
    }
    let Some(mut m) = rtm_msg1(RTM_80211INFO, None) else {
        return;
    };
    if let Some(d) = m.data_mut() {
        hdr_edit::<IfIeee80211Msghdr>(d, |h| {
            h.ifim_index = ifp.if_index.get() as u16;
            h.ifim_tableid = ifp.if_rdomain.get() as u16;
            h.ifim_ifie = *ifie;
        });
    }
    route_input_any(m, AF_UNSPEC);
}

/// `rtm_proposal(ifp, rtinfo, flags, prio)`: routing socket messages indicating the address
/// selection proposal from an interface.
pub fn rtm_proposal(ifp: &Ifnet, rtinfo: &mut RtAddrinfo<'_>, flags: u32, prio: u8) {
    let Some(mut m) = rtm_msg1(RTM_PROPOSAL, Some(rtinfo)) else {
        return;
    };
    if let Some(d) = m.data_mut() {
        hdr_edit::<RtMsghdr>(d, |h| {
            h.rtm_flags = (RTF_DONE | flags) as i32;
            h.rtm_priority = prio;
            h.rtm_tableid = ifp.if_rdomain.get() as u16;
            h.rtm_index = ifp.if_index.get() as u16;
            h.rtm_addrs = rtinfo.rti_addrs;
        });
    }
    let af = match rtinfo.rti_info[RTAX_DNS] {
        Some(d) => d.family(),
        None => kpanic(format_args!("rtm_proposal: no RTAX_DNS")),
    };
    route_input_any(m, af);
}

/// `sysctl_dumpentry(rt, w, id)`: route `rt` of table `id` as an `RTM_GET` message of the
/// dump (a route the walk's operation filters out skipped), copied out when the answer has
/// room; under `rtable_read`'s lock and the shared net lock `net`.
fn sysctl_dumpentry(
    rt: &Rtentry,
    w: &mut Walkarg,
    id: u32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    if w.w_op == NET_RT_FLAGS && rt.rt_flags.get() & w.w_arg as u32 == 0 {
        return Ok(());
    }
    if w.w_op == NET_RT_DUMP && w.w_arg != 0 {
        let mut prio = (w.w_arg as u8) & RTP_MASK;
        if w.w_arg < 0 {
            prio = (w.w_arg.wrapping_neg() as u8) & RTP_MASK;
            // Show all routes that are not this priority
            if prio == rt.rt_priority.get() & RTP_MASK {
                return Ok(());
            }
        } else if prio != rt.rt_priority.get() & RTP_MASK && prio != RTP_ANY {
            return Ok(());
        }
    }
    let mut a = RouteAddrs::of(rt, id, true, net);
    let mut info = a.info(rt, true);
    // BFD: RTAX_BFD of an RTF_BFD route, not configured.
    // MPLS: RTAX_SRC and rti_mpls of an RTF_MPLS route, not configured.

    let size = rtm_msg2(RTM_GET, &mut info, None, Some(w));
    if w.w_where != 0
        && w.w_needed <= w.w_given
        && let Some(tmem) = w.w_tmem.as_mut()
    {
        let addrs = info.rti_addrs;
        hdr_edit::<RtMsghdr>(tmem, |h| {
            h.rtm_pid = w.w_pid;
            h.rtm_flags = (RTF_DONE | rt.rt_flags.get()) as i32;
            h.rtm_priority = rt.rt_priority.get() & RTP_MASK;
            rtm_getmetrics(rt, &mut h.rtm_rmx);
            // Do not account the routing table's reference.
            h.rtm_rmx.rmx_refcnt = (rtable_read_refcnt(rt) as u32).wrapping_sub(1);
            h.rtm_index = rt.rt_ifidx.get() as u16;
            h.rtm_addrs = addrs;
            h.rtm_tableid = id as u16;
            // MPLS: rtm_mpls = rti_mpls, not configured.
        });
        match UserPtr::<u8>::new(w.w_where).copyout_slice(&tmem[..size]) {
            Err(e) => {
                w.w_where = 0;
                return Err(e);
            }
            Ok(()) => w.w_where += size,
        }
    }
    Ok(())
}

/// `sysctl_rtable_rtstat(oldp, oldlenp, newp)`: `struct rtstat`, one 32-bit word per
/// counter.
fn sysctl_rtable_rtstat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let counters = RTCOUNTERS.read();
    let w = |i: usize| counters[i] as u32;
    let rtstat = Rtstat {
        rts_badredirect: w(0),
        rts_dynamic: w(1),
        rts_newgateway: w(2),
        rts_unreach: w(3),
        rts_wildcard: w(4),
    };
    rdstruct(req, hal::user::bytes_of(core::slice::from_ref(&rtstat)))
}

/// `sysctl_iflist(af, w)`: every interface (or the one `w_arg` names) as an `RTM_IFINFO`
/// message, each followed by its addresses of family `af` (any for 0) as `RTM_NEWADDR`
/// messages; under the shared net lock `net`.
fn sysctl_iflist(af: u8, w: &mut Walkarg, net: &NetHeld<'_>) -> Result<(), Errno> {
    for ifp in &IFNETLIST {
        if w.w_arg != 0 && w.w_arg as u32 != ifp.if_index.get() {
            continue;
        }
        // Copy the link-layer address first
        let sadl = sadl_copy(ifp);
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_IFP] = sadl.as_ref().map(SaCopy::view);
        let len = rtm_msg2(RTM_IFINFO, &mut info, None, Some(w));
        if w.w_where != 0
            && w.w_needed <= w.w_given
            && let Some(tmem) = w.w_tmem.as_mut()
        {
            let mut data = IfData::default();
            if_getdata(ifp, &mut data);
            let addrs = info.rti_addrs;
            hdr_edit::<IfMsghdr>(tmem, |h| {
                h.ifm_index = ifp.if_index.get() as u16;
                h.ifm_tableid = ifp.if_rdomain.get() as u16;
                h.ifm_flags = ifp.if_flags.get();
                h.ifm_data = data;
                h.ifm_addrs = addrs;
            });
            UserPtr::<u8>::new(w.w_where).copyout_slice(&tmem[..len])?;
            w.w_where += len;
        }
        for ifa in &ifp.if_addrlist {
            let addr = ifa.ifa_addr.read(net);
            crate::assert::kassert!(addr.is_some_and(|a| a.family() != AF_LINK));
            if af != 0 && addr.is_none_or(|a| a.family() != af) {
                continue;
            }
            let mask = ifa.ifa_netmask.read(net);
            let brd = ifa.ifa_dstaddr.read(net);
            let mut info = RtAddrinfo::new();
            info.rti_info[RTAX_IFA] = addr.as_ref().and_then(|a| SockaddrView::new(a));
            info.rti_info[RTAX_NETMASK] = mask.as_ref().and_then(|a| SockaddrView::new(a));
            info.rti_info[RTAX_BRD] = brd.as_ref().and_then(|a| SockaddrView::new(a));
            let len = rtm_msg2(RTM_NEWADDR, &mut info, None, Some(w));
            if w.w_where != 0
                && w.w_needed <= w.w_given
                && let Some(tmem) = w.w_tmem.as_mut()
            {
                let addrs = info.rti_addrs;
                let index = ifa.ifa_ifp.get().map_or(0, |i| i.if_index.get());
                hdr_edit::<IfaMsghdr>(tmem, |h| {
                    h.ifam_index = index as u16;
                    h.ifam_flags = ifa.ifa_flags.get() as i32;
                    h.ifam_metric = ifa.ifa_metric.get();
                    h.ifam_addrs = addrs;
                });
                UserPtr::<u8>::new(w.w_where).copyout_slice(&tmem[..len])?;
                w.w_where += len;
            }
        }
    }
    Ok(())
}

/// `sysctl_ifnames(w)`: every interface (or the one `w_arg` names) as a
/// `struct if_nameindex_msg`, copied out from a list of references taken under the shared
/// net lock (the C's `if_tmplist`).
fn sysctl_ifnames(w: &mut Walkarg) -> Result<(), Errno> {
    let mut error = Ok(());

    let tmp = IF_TMPLIST_LOCK.write();
    let if_tmplist: List<IfTmplist> = List::new();
    {
        let _net = NETLOCK.read();
        // XXX ignore tableid for now
        for ifp in &IFNETLIST {
            if w.w_arg != 0 && w.w_arg as u32 != ifp.if_index.get() {
                continue;
            }
            if_tmplist.insert_tail(if_ref(ifp));
        }
    }

    for ifp in &if_tmplist {
        w.w_needed += size_of::<IfNameindexMsg>();
        if w.w_where != 0 && w.w_needed <= w.w_given {
            let mut ifn = IfNameindexMsg {
                if_index: ifp.if_index.get(),
                ..IfNameindexMsg::default()
            };
            ifp.if_xname.with_str(|n| {
                let k = n.len().min(ifn.if_name.len() - 1);
                ifn.if_name[..k].copy_from_slice(&n[..k]);
            });
            if let Err(e) = UserPtr::<IfNameindexMsg>::new(w.w_where).copyout(&ifn) {
                error = Err(e);
                break;
            }
            w.w_where += size_of::<IfNameindexMsg>();
        }
    }

    while let Some(ifp) = if_tmplist.pop_head() {
        if_put(Some(ifp));
    }
    drop(tmp);

    error
}

/// `sysctl_source(af, tableid, w)`: the preferred source address of family `af` of table
/// `tableid`, when one is set.
fn sysctl_source(af: SaFamily, tableid: u32, w: &mut Walkarg) -> Result<(), Errno> {
    let src = {
        let nr = NETLOCK.read();
        rtable_getsource(tableid, af, &net_held(&nr)).filter(|sa| {
            matches!(
                (sa.view().family(), sa.bytes().len()),
                (AF_INET, SIN_LEN..) | (AF_INET6, SIN6_LEN..)
            )
        })
    };

    if let Some(sa) = src {
        let size = match sa.view().family() {
            AF_INET => SIN_LEN,
            _ => SIN6_LEN,
        };
        w.w_needed += size;
        if w.w_where != 0 && w.w_needed <= w.w_given {
            UserPtr::<u8>::new(w.w_where).copyout_slice(&sa.bytes()[..size])?;
            w.w_where += size;
        }
    }
    Ok(())
}

/// `sysctl_rtable(name, namelen, where, given, new, newlen)`: the `net.route` tree
/// (`name` = family, operation, argument and an optional table): `EPERM` for a new value,
/// `EINVAL` for a malformed name, `ENOENT` for a table that does not exist, `ENOMEM` when
/// the answer does not fit (the room needed then in `*given`, as it is for a size estimate:
/// what is needed plus a tenth, at least 1024 bytes, in pages).
pub fn sysctl_rtable<G: ProcGlue>(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    if name.len() < 3 || name.len() > 4 {
        return Err(Errno::EINVAL);
    }
    let af = name[0] as u8;
    let where_ = req.oldp;
    let mut w = Walkarg {
        w_op: name[1],
        w_arg: name[2],
        w_given: *req.oldlenp,
        w_needed: 0,
        w_where: where_,
        w_tmem: None,
        w_pid: curpid::<G>(),
    };

    let tableid = if name.len() == 4 {
        let t = name[3] as u32;
        if !rtable_exists(t) {
            return Err(Errno::ENOENT);
        }
        t
    } else {
        currtable::<G>()
    };

    let mut error = Err(Errno::EINVAL);
    match w.w_op {
        NET_RT_DUMP | NET_RT_FLAGS => {
            let nr = NETLOCK.read();
            let net = net_held(&nr);
            for i in 1..=AF_MAX {
                if af != 0 && af != i {
                    continue;
                }

                error = rtable_read(tableid, i, |rt, id| sysctl_dumpentry(rt, &mut w, id, &net));
                if error == Err(Errno::EAFNOSUPPORT) {
                    error = Ok(());
                }
                if error.is_err() {
                    break;
                }
            }
            drop(nr);
        }
        NET_RT_STATS => return sysctl_rtable_rtstat(req),
        NET_RT_TABLE => {
            let tableid = w.w_arg as u32;
            if rtable_exists(tableid) {
                let tableinfo = RtTableinfo {
                    rti_tableid: tableid as u16,
                    rti_domainid: rtable_l2(tableid) as u16,
                };
                return rdstruct(req, hal::user::bytes_of(core::slice::from_ref(&tableinfo)));
            }
            return Err(Errno::ENOENT);
        }
        NET_RT_SOURCE => {
            let tableid = w.w_arg as u32;
            if !rtable_exists(tableid) {
                return Err(Errno::ENOENT);
            }
            for i in 1..=AF_MAX {
                if af != 0 && af != i {
                    continue;
                }

                error = sysctl_source(i, tableid, &mut w);
                if error == Err(Errno::EAFNOSUPPORT) {
                    error = Ok(());
                }
                if error.is_err() {
                    break;
                }
            }
        }
        NET_RT_IFLIST => {
            let nr = NETLOCK.read();
            error = sysctl_iflist(af, &mut w, &net_held(&nr));
            drop(nr);
        }
        NET_RT_IFNAMES => {
            error = sysctl_ifnames(&mut w);
        }
        _ => {}
    }
    drop(w.w_tmem.take());
    if where_ != 0 {
        *req.oldlenp = w.w_where.wrapping_sub(where_);
        if w.w_needed > w.w_given {
            return Err(Errno::ENOMEM);
        }
    } else if w.w_needed == 0 {
        *req.oldlenp = 0;
    } else {
        *req.oldlenp = (w.w_needed + (w.w_needed / 10).max(1024)).next_multiple_of(PAGE_SIZE);
    }
    error
}

/// `rtm_validate_proposal(info)`: the addresses an `RTM_PROPOSAL` may carry, of the sizes
/// their families have (`Err(EINVAL)` for the C's -1).
fn rtm_validate_proposal(info: &RtAddrinfo<'_>) -> Result<(), Errno> {
    if info.rti_addrs & !(RTA_NETMASK | RTA_IFA | RTA_DNS | RTA_STATIC | RTA_SEARCH) != 0 {
        return Err(Errno::EINVAL);
    }

    // A netmask and an interface address: an IPv4 or IPv6 socket address of its exact size.
    for (bit, idx) in [(RTA_NETMASK, RTAX_NETMASK), (RTA_IFA, RTAX_IFA)] {
        if info.rti_addrs & bit == 0 {
            continue;
        }
        let Some(sa) = info.rti_info[idx] else {
            return Err(Errno::EINVAL);
        };
        match sa.family() {
            AF_INET if usize::from(sa.len()) == SIN_LEN => {}
            AF_INET6 if usize::from(sa.len()) == SIN6_LEN => {}
            _ => return Err(Errno::EINVAL),
        }
    }

    if info.rti_addrs & RTA_DNS != 0 {
        let Some(rtdns) = info.rti_info[RTAX_DNS] else {
            return Err(Errno::EINVAL);
        };
        let sr_len = usize::from(rtdns.len());
        if sr_len > size_of::<SockaddrRtdns>() {
            return Err(Errno::EINVAL);
        }
        // offsetof(struct sockaddr_rtdns, sr_dns)
        if sr_len < 2 {
            return Err(Errno::EINVAL);
        }
        let unit = match rtdns.family() {
            AF_INET => 4,
            AF_INET6 => 16,
            _ => return Err(Errno::EINVAL),
        };
        if (sr_len - 2) % unit != 0 {
            return Err(Errno::EINVAL);
        }
    }

    // A static route list and a search list: no longer than their structure, longer than
    // its header.
    for (bit, idx, max) in [
        (RTA_STATIC, RTAX_STATIC, size_of::<SockaddrRtstatic>()),
        (RTA_SEARCH, RTAX_SEARCH, size_of::<SockaddrRtsearch>()),
    ] {
        if info.rti_addrs & bit == 0 {
            continue;
        }
        let Some(sa) = info.rti_info[idx] else {
            return Err(Errno::EINVAL);
        };
        let sr_len = usize::from(sa.len());
        if sr_len > max || sr_len <= 2 {
            return Err(Errno::EINVAL);
        }
    }

    Ok(())
}

/// `rt_setsource(rtableid, src)`: the preferred source address of table `rtableid` set to
/// `src`, an address of an interface of its routing domain (`EINVAL` otherwise); the
/// unspecified address goes back to automatic source selection; `EAFNOSUPPORT` for another
/// family. Under the exclusive net lock `net`.
fn rt_setsource(rtableid: u32, src: SockaddrView<'_>, net: &NetHeld<'_>) -> Result<(), Errno> {
    // If source address is 0.0.0.0 or :: use automatic source selection
    let b = src.bytes();
    match src.family() {
        AF_INET => {
            if b.get(4..8).is_none_or(|a| a == [0; 4]) {
                return rtable_setsource(rtableid, AF_INET, None, net);
            }
        }
        AF_INET6 => {
            if b.get(8..24).is_none_or(|a| a == [0; 16]) {
                return rtable_setsource(rtableid, AF_INET6, None, net);
            }
        }
        _ => return Err(Errno::EAFNOSUPPORT),
    }

    // Check if source address is assigned to an interface in the same rdomain
    let Some(ifa) = ifa_ifwithaddr(net, b, rtableid) else {
        return Err(Errno::EINVAL);
    };

    let addr = ifa.ifa_addr.read(net);
    rtable_setsource(
        rtableid,
        src.family(),
        addr.as_ref().and_then(|a| SockaddrView::new(a)),
        net,
    )
}

impl<G: ProcGlue> Domain<G> for RouteDomain<G> {
    fn family(&self) -> i32 {
        i32::from(PF_ROUTE)
    }
    fn name(&self) -> &'static str {
        "route"
    }
    fn init(&self) {
        crate::net::route::route_init();
    }
    fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
        Self::PROTOSW
    }
}

impl<G: ProcGlue> Protocol<G> for RouteProto<G> {
    fn domain(&self) -> &'static dyn Domain<G> {
        RouteDomain::<G>::DOMAIN
    }
    fn ty(&self) -> i16 {
        SOCK_RAW as i16
    }
    fn protocol(&self) -> i16 {
        0
    }
    fn flags(&self) -> i16 {
        PR_ATOMIC | PR_ADDR | PR_WANTRCVD
    }
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
        Some(RouteUsrreqs::<G>::USR)
    }
    fn has_ctloutput(&self) -> bool {
        true
    }
    fn ctloutput(
        &self,
        op: i32,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        level: i32,
        optname: i32,
        m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        route_ctloutput(op, so, w, level, optname, m)
    }
    fn init(&self) {
        route_prinit::<G>();
    }
}

impl<G: ProcGlue> Usrreqs<G> for RouteUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        route_attach(so, w, proto, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        route_detach(so, w)
    }
    fn disconnect(&self, so: &Socket<G>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        soisdisconnected(so);
        Ok(())
    }
    fn shutdown(&self, so: &Socket<G>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        socantsendmore(so);
        Ok(())
    }
    fn rcvd(&self, so: &Socket<G>, w: &mut SoLocked<'_>) {
        route_rcvd(so, w);
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        route_send(so, w, top, addr, control)
    }
    fn abort(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        // route_usrreqs has no pru_abort: the C's call through NULL.
        kpanic(format_args!("pru_abort of a routing socket"));
    }
    fn sockaddr(
        &self,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        _addr: &mut Mbuf,
    ) -> Result<(), Errno> {
        Err(Errno::EINVAL)
    }
    fn peeraddr(
        &self,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        addr: &mut Mbuf,
    ) -> Result<(), Errno> {
        route_peeraddr(addr);
        Ok(())
    }
}

const _: () = {
    assert!(size_of::<MsgHead>() == 6);
    assert!(RT_MSGHDR == 96);
    assert!(SDL_LEN == 32);
    assert!(size_of::<SockaddrRtdns>() == 2 + RTDNS_LEN);
};

/// The routing socket's test tap: the announcements recorded as if a socket listened (the
/// routes' tests watch their messages without a socket layer).
#[cfg(test)]
pub(crate) mod tap {
    use std::sync::Mutex;
    use std::vec::Vec;

    use super::*;

    /// The messages seen while the tap is on, whole (`None`: off).
    static TAP: Mutex<Option<Vec<Vec<u8>>>> = Mutex::new(None);

    /// The tap on, empty.
    pub(crate) fn start() {
        if let Ok(mut t) = TAP.lock() {
            *t = Some(Vec::new());
        }
    }

    /// The tap off; the messages it saw.
    pub(crate) fn stop() -> Vec<Vec<u8>> {
        TAP.lock()
            .ok()
            .and_then(|mut t| t.take())
            .unwrap_or_default()
    }

    /// The types of the messages seen so far.
    pub(crate) fn types() -> Vec<u8> {
        TAP.lock()
            .ok()
            .and_then(|t| {
                t.as_ref()
                    .map(|v| v.iter().map(|m| m.get(3).copied().unwrap_or(0)).collect())
            })
            .unwrap_or_default()
    }

    /// Whether the tap is on.
    pub(crate) fn on() -> bool {
        TAP.lock().is_ok_and(|t| t.is_some())
    }

    /// `m` recorded when the tap is on.
    pub(crate) fn record(m: &Mbuf) {
        let Ok(mut t) = TAP.lock() else {
            return;
        };
        let Some(v) = t.as_mut() else {
            return;
        };
        let mut b = std::vec![0u8; m.m_pkthdr().len.get().max(0) as usize];
        m_copydata(m, 0, &mut b);
        v.push(b);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the routing socket: the layout `rtm_msg2` builds, the address parsing
    // and checks of `rtm_xaddrs` (the C's cases, and every truncation of a message), the
    // proposals' checks, the announcements' headers (through the test tap), and the
    // `net.route` sysctl over a table of the routes' test interface: the dump with the C's
    // `rmx_refcnt`, its filters and size estimate, a short room, a copy-out fault, the
    // argument checks, the interface lists and names, the table information, the statistics
    // and the preferred sources. The user copies go to `USER_BASE + 0x3_8000`, 32 KiB no
    // other test names (D-N6.3b-20).

    use std::vec;
    use std::vec::Vec;

    use hal::sys::net_if::IFAN_ARRIVAL;
    use hal::sys::net_route::{
        RTA_BRD, RTA_DST, RTA_GATEWAY, RTA_IFP, RTA_LABEL, RTF_CONNECTED, RTP_STATIC,
    };

    use super::*;
    use crate::fs::testfs::TestGlue as G;
    use crate::net::route::tests::{grace, iface, sin, testroutes_setup};
    use crate::net::route::{rt_ifa_add, rt_ifa_addlocal, rt_ifa_purge};
    use crate::testutil::{USER_BASE, USER_LEN, user_bytes};

    /// Where the sysctl answers go in the user-copy double.
    const UDUMP: usize = USER_BASE + 0x3_8000;
    /// Its room.
    const UDUMP_LEN: usize = 0x8000;

    fn v(b: &[u8]) -> SockaddrView<'_> {
        SockaddrView::new(b).expect("a view")
    }

    /// The `struct rt_msghdr` at `off` of `buf`; zeros past the end of a shorter message (the
    /// interface messages share only the first fields).
    fn hdr_at(buf: &[u8], off: usize) -> RtMsghdr {
        let mut h = [0u8; RT_MSGHDR];
        let n = (buf.len() - off).min(h.len());
        h[..n].copy_from_slice(&buf[off..off + n]);
        hal::pod::at(&h, 0).expect("a header")
    }

    /// The messages of an answer: (offset, header) of each, by `rtm_msglen`.
    fn messages(buf: &[u8]) -> Vec<(usize, RtMsghdr)> {
        let mut out = Vec::new();
        let mut off = 0;
        while off < buf.len() {
            let h = hdr_at(buf, off);
            assert_eq!(h.rtm_version, RTM_VERSION);
            assert!(usize::from(h.rtm_msglen) >= usize::from(h.rtm_hdrlen));
            out.push((off, h));
            off += usize::from(h.rtm_msglen);
        }
        assert_eq!(off, buf.len(), "messages end at the answer's end");
        out
    }

    /// `sysctl(name)` with `room` bytes at `oldp`: the result and the length given back.
    fn sysctl(name: &[i32], oldp: usize, room: usize) -> (Result<(), Errno>, usize) {
        let mut len = room;
        let mut req = SysctlReq {
            oldp,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        let r = sysctl_rtable::<G>(name, &mut req);
        (r, len)
    }

    /// `sysctl(name)` the way libc does it: the size first, then the answer.
    fn answer(name: &[i32]) -> Vec<u8> {
        let (r, size) = sysctl(name, 0, 0);
        r.expect("the size estimate");
        let (r, len) = sysctl(name, UDUMP, size.min(UDUMP_LEN));
        r.expect("the answer");
        user_bytes(UDUMP, len).expect("the answer's bytes")
    }

    /// The IPv4 address of the `sockaddr_in` at `off` of `buf`.
    fn in_at(buf: &[u8], off: usize) -> [u8; 4] {
        [buf[off + 4], buf[off + 5], buf[off + 6], buf[off + 7]]
    }

    #[test]
    fn rtm_msg2_lays_out_the_header_and_the_rounded_addresses() {
        let dst = sin([10, 0, 2, 0]);
        let mut mask = sin([255, 255, 255, 0]);
        mask[0] = 7; // trimmed, as rt_plen2mask leaves it
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = Some(v(&dst));
        info.rti_info[RTAX_NETMASK] = Some(v(&mask));

        let len = rtm_msg2(RTM_GET, &mut info, None, None);
        assert_eq!(len, RT_MSGHDR + 16 + 8);
        assert_eq!(info.rti_addrs, RTA_DST | RTA_NETMASK);

        let mut buf = vec![0xffu8; len];
        assert_eq!(rtm_msg2(RTM_GET, &mut info, Some(&mut buf), None), len);
        let h = hdr_at(&buf, 0);
        assert_eq!(usize::from(h.rtm_msglen), len);
        assert_eq!(h.rtm_version, RTM_VERSION);
        assert_eq!(h.rtm_type, RTM_GET);
        assert_eq!(usize::from(h.rtm_hdrlen), RT_MSGHDR);
        // Without a walk the rest of the caller's header is left alone.
        assert_eq!(buf[6], 0xff, "rtm_index untouched");
        let a = RT_MSGHDR;
        assert_eq!(&buf[a..a + 16], &dst[..]);
        assert_eq!(&buf[a + 16..a + 23], &mask[..7]);
        assert_eq!(buf[a + 23], 0, "rounded up with zeros");

        // The interface messages have their own headers; an empty address takes a long.
        let mut info = RtAddrinfo::new();
        assert_eq!(rtm_msg2(RTM_IFINFO, &mut info, None, None), IF_MSGHDR);
        assert_eq!(rtm_msg2(RTM_NEWADDR, &mut info, None, None), IFA_MSGHDR);
        assert_eq!(info.rti_addrs, 0);
        let empty = [0u8, AF_INET];
        info.rti_info[RTAX_GATEWAY] = Some(v(&empty));
        assert_eq!(rtm_msg2(RTM_GET, &mut info, None, None), RT_MSGHDR + 8);
        assert_eq!(roundup_long(17), 24);
    }

    /// A message's addresses: `dst`, `gw` and a label `blue`, each rounded up.
    fn three_addresses() -> (Vec<u8>, usize) {
        let mut label = [0u8; size_of::<SockaddrRtlabel>()];
        label[0] = label.len() as u8;
        label[1] = AF_UNSPEC;
        label[2..6].copy_from_slice(b"blue");
        let mut b = vec![0u8; 128];
        b[..16].copy_from_slice(&sin([10, 0, 2, 0]));
        b[16..32].copy_from_slice(&sin([10, 0, 2, 2]));
        b[32..32 + label.len()].copy_from_slice(&label);
        (b, 32 + roundup_long(label.len()))
    }

    #[test]
    fn rtm_xaddrs_splits_and_checks_the_addresses() {
        let (mut bytes, end) = three_addresses();
        let parse = |b: &[u8], addrs: i32, len: usize| -> Result<[usize; 3], Errno> {
            let mut info = RtAddrinfo::new();
            info.rti_addrs = addrs;
            rtm_xaddrs(&b[..len], &mut info)?;
            let at = |i: usize| {
                info.rti_info[i].map_or(usize::MAX, |s| {
                    s.bytes().as_ptr() as usize - b.as_ptr() as usize
                })
            };
            Ok([at(RTAX_DST), at(RTAX_GATEWAY), at(RTAX_LABEL)])
        };

        assert_eq!(
            parse(&bytes, RTA_DST | RTA_GATEWAY | RTA_LABEL, end),
            Ok([0, 16, 32])
        );
        assert_eq!(parse(&bytes, RTA_DST, end), Ok([0, usize::MAX, usize::MAX]));

        // An address past the end, a bit past RTAX_MAX.
        assert_eq!(parse(&bytes, RTA_DST | RTA_GATEWAY, 24), Err(Errno::EINVAL));
        assert_eq!(parse(&bytes, RTA_DST | (1 << 20), end), Err(Errno::EINVAL));
        // A sockaddr_in as the interface (which must be AF_LINK), as a label, as a DNS
        // proposal of the wrong family is fine but too long is not.
        assert_eq!(
            parse(&bytes, RTA_DST | RTA_IFP, end),
            Err(Errno::EAFNOSUPPORT)
        );
        assert_eq!(parse(&bytes, RTA_LABEL, end), Err(Errno::EAFNOSUPPORT));

        // A short sockaddr_in as the destination.
        bytes[0] = 8;
        assert_eq!(parse(&bytes, RTA_DST, end), Err(Errno::EINVAL));
        bytes[0] = 16;

        // A label without its NUL.
        bytes[34..34 + RTLABEL_LEN].fill(b'x');
        assert_eq!(
            parse(&bytes, RTA_DST | RTA_GATEWAY | RTA_LABEL, end),
            Err(Errno::EINVAL)
        );
        // The C's break after the first string: a bad label is caught, but an address after
        // a good one is not checked (an AF_INET search list).
        let (mut bytes, _) = three_addresses();
        let mut search = [0u8; 16];
        search[0] = 16;
        search[1] = AF_INET;
        bytes[72..88].copy_from_slice(&search);
        let mut info = RtAddrinfo::new();
        info.rti_addrs = RTA_LABEL | hal::sys::net_route::RTA_SEARCH;
        assert_eq!(rtm_xaddrs(&bytes[32..88], &mut info), Ok(()));
    }

    #[test]
    fn rtm_xaddrs_never_reads_past_a_truncated_message() {
        let (bytes, end) = three_addresses();
        let all = RTA_DST | RTA_GATEWAY | RTA_LABEL;
        for len in 0..=end {
            let mut info = RtAddrinfo::new();
            info.rti_addrs = all;
            let r = rtm_xaddrs(&bytes[..len], &mut info);
            // The last address ends at 66: shorter messages cannot hold it.
            assert_eq!(r.is_ok(), len >= 66, "length {len}");
            for sa in info.rti_info.iter().flatten() {
                let off = sa.bytes().as_ptr() as usize - bytes.as_ptr() as usize;
                assert!(off + sa.bytes().len() <= len, "a view past the message");
            }
        }
        // Every address length at the end of a 24-byte message.
        for sa_len in 0..=255u8 {
            let mut b = [0u8; 24];
            b[0] = sa_len;
            b[1] = AF_INET;
            let mut info = RtAddrinfo::new();
            info.rti_addrs = RTA_GATEWAY;
            let r = rtm_xaddrs(&b, &mut info);
            assert_eq!(r.is_ok(), (16..=24).contains(&sa_len), "sa_len {sa_len}");
        }
    }

    #[test]
    fn proposals_are_validated() {
        let ifa = sin([10, 0, 2, 15]);
        let mut dns = [0u8; 2 + 8];
        dns[0] = dns.len() as u8;
        dns[1] = AF_INET;
        let mut info = RtAddrinfo::new();
        info.rti_addrs = RTA_IFA | RTA_DNS;
        info.rti_info[RTAX_IFA] = Some(v(&ifa));
        info.rti_info[RTAX_DNS] = Some(v(&dns));
        assert_eq!(rtm_validate_proposal(&info), Ok(()));

        // Two and a half IPv4 addresses.
        let mut odd = dns;
        odd[0] = 8;
        info.rti_info[RTAX_DNS] = Some(v(&odd));
        assert_eq!(rtm_validate_proposal(&info), Err(Errno::EINVAL));
        info.rti_info[RTAX_DNS] = Some(v(&dns));
        // A bit the proposals do not carry.
        info.rti_addrs |= RTA_DST;
        assert_eq!(rtm_validate_proposal(&info), Err(Errno::EINVAL));
        info.rti_addrs &= !RTA_DST;
        // A short interface address, a missing one.
        let mut short = ifa;
        short[0] = 8;
        info.rti_info[RTAX_IFA] = Some(v(&short));
        assert_eq!(rtm_validate_proposal(&info), Err(Errno::EINVAL));
        info.rti_info[RTAX_IFA] = None;
        assert_eq!(rtm_validate_proposal(&info), Err(Errno::EINVAL));
        // A search list no longer than its header.
        let search = [2u8, AF_UNSPEC];
        let mut info = RtAddrinfo::new();
        info.rti_addrs = hal::sys::net_route::RTA_SEARCH;
        info.rti_info[RTAX_SEARCH] = Some(v(&search));
        assert_eq!(rtm_validate_proposal(&info), Err(Errno::EINVAL));
    }

    #[test]
    fn announcements_carry_their_headers_and_addresses() {
        let _g = testroutes_setup();
        let (ifp, _d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        tap::start();

        rtm_ifannounce(ifp, IFAN_ARRIVAL);
        rtm_ifchg(ifp);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rtm_addr(RTM_NEWADDR, ifa, &net);
        drop(w);
        let mut info = RtAddrinfo::new();
        let dst = sin([8, 8, 8, 8]);
        info.rti_info[RTAX_DST] = Some(v(&dst));
        rtm_miss(
            hal::sys::net_route::RTM_MISS,
            &info,
            0,
            3,
            7,
            Errno::EHOSTUNREACH as i32,
            0,
        );
        let ms = tap::stop();

        assert_eq!(ms.len(), 4);
        let an: IfAnnouncemsghdr = hal::pod::at(&ms[0], 0).expect("an announcement");
        assert_eq!(
            (an.ifan_type, usize::from(an.ifan_msglen), an.ifan_what),
            (RTM_IFANNOUNCE, size_of::<IfAnnouncemsghdr>(), IFAN_ARRIVAL)
        );
        assert_eq!(u32::from(an.ifan_index), ifp.if_index.get());
        ifp.if_xname
            .with_str(|n| assert_eq!(&an.ifan_name[..n.len()], n));

        let ifm: IfMsghdr = hal::pod::at(&ms[1], 0).expect("an interface message");
        assert_eq!((ifm.ifm_type, ifm.ifm_addrs), (RTM_IFINFO, RTA_IFP));
        assert_eq!(ifm.ifm_flags, ifp.if_flags.get());
        assert_eq!(ms[1][IF_MSGHDR + 1], AF_LINK);

        let ifam: IfaMsghdr = hal::pod::at(&ms[2], 0).expect("an address message");
        assert_eq!(ifam.ifam_type, RTM_NEWADDR);
        assert_eq!(ifam.ifam_addrs, RTA_NETMASK | RTA_IFP | RTA_IFA);
        // RTAX_NETMASK, RTAX_IFP, then RTAX_IFA, each rounded up.
        let mask_len = roundup_long(usize::from(ms[2][IFA_MSGHDR]));
        let sdl_len = roundup_long(usize::from(ms[2][IFA_MSGHDR + mask_len]));
        assert_eq!(
            in_at(&ms[2], IFA_MSGHDR + mask_len + sdl_len),
            [10, 0, 2, 15]
        );

        let h = hdr_at(&ms[3], 0);
        assert_eq!(h.rtm_type, hal::sys::net_route::RTM_MISS);
        assert_eq!((h.rtm_priority, h.rtm_index), (3, 7));
        assert_eq!(h.rtm_errno, Errno::EHOSTUNREACH as i32);
        assert_eq!(h.rtm_flags as u32 & RTF_DONE, RTF_DONE);
        assert_eq!(in_at(&ms[3], RT_MSGHDR), [8, 8, 8, 8]);

        // No listener, no message.
        rtm_ifchg(ifp);
        assert!(tap::stop().is_empty());
    }

    #[test]
    fn sysctl_dumps_the_routes_with_the_c_reference_counts() {
        let _g = testroutes_setup();
        let (ifp, _d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rt_ifa_addlocal(ifa, &net).expect("the local route");
        rt_ifa_add(ifa, RTF_CONNECTED, v(&sin([10, 0, 2, 0])), 0, &net).expect("the subnet");
        drop(w);

        // The size estimate: what is needed plus a tenth (at least 1024), in pages.
        let (r, size) = sysctl(&[0, NET_RT_DUMP, 0, 0], 0, 0);
        assert_eq!(r, Ok(()));
        assert!(size > 0 && size % PAGE_SIZE == 0);

        let buf = answer(&[0, NET_RT_DUMP, 0, 0]);
        let msgs = messages(&buf);
        assert_eq!(msgs.len(), 2, "{msgs:?}");
        let mut seen = Vec::new();
        for &(off, h) in &msgs {
            assert_eq!(h.rtm_type, RTM_GET);
            assert_eq!(usize::from(h.rtm_hdrlen), RT_MSGHDR);
            assert_ne!(h.rtm_flags as u32 & RTF_DONE, 0);
            assert_eq!(u32::from(h.rtm_index), ifp.if_index.get());
            assert_eq!(h.rtm_tableid, 0);
            let want = RTA_DST | RTA_GATEWAY | RTA_IFP | RTA_IFA;
            assert_eq!(h.rtm_addrs & want, want, "addresses of {h:?}");
            assert_eq!(h.rtm_addrs & (RTA_LABEL | RTA_BRD), 0);
            // Only the table holds them: the walk's own handle is left out.
            assert_eq!(h.rtm_rmx.rmx_refcnt, 0, "{h:?}");
            seen.push((
                in_at(&buf, off + usize::from(h.rtm_hdrlen)),
                h.rtm_flags as u32,
            ));
        }
        let has = |a: [u8; 4], flag: u32| seen.iter().any(|&(d, f)| d == a && f & flag != 0);
        assert!(has([10, 0, 2, 15], RTF_LOCAL), "local route: {seen:?}");
        assert!(has([10, 0, 2, 0], RTF_CONNECTED), "prefix route: {seen:?}");

        // A reference held elsewhere is counted, as `netstat -r` shows it.
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        let held = rtalloc(v(&sin([10, 0, 2, 15])), 0, 0, &net).expect("the local route");
        drop(w);
        let buf = answer(&[0, NET_RT_FLAGS, RTF_LOCAL as i32, 0]);
        let local = messages(&buf);
        assert_eq!(local.len(), 1, "NET_RT_FLAGS keeps the local route");
        assert_eq!(local[0].1.rtm_rmx.rmx_refcnt, 1);
        drop(held);

        // A priority filter: none of these is static; the negative filter keeps both.
        assert!(answer(&[0, NET_RT_DUMP, i32::from(RTP_STATIC), 0]).is_empty());
        assert_eq!(
            messages(&answer(&[0, NET_RT_DUMP, -i32::from(RTP_STATIC), 0])).len(),
            2
        );
        // Another family: nothing.
        assert!(answer(&[i32::from(AF_INET6), NET_RT_DUMP, 0, 0]).is_empty());

        // A room too small for the first message: nothing copied, ENOMEM.
        assert_eq!(
            sysctl(&[0, NET_RT_DUMP, 0, 0], UDUMP, 50),
            (Err(Errno::ENOMEM), 0)
        );
        // Room for the first message only: it is copied, ENOMEM for the rest.
        let first = usize::from(msgs[0].1.rtm_msglen);
        assert_eq!(
            sysctl(&[0, NET_RT_DUMP, 0, 0], UDUMP, first + 8),
            (Err(Errno::ENOMEM), first)
        );
        // A copy-out that faults: its error, and the C's length (where's NULL less where).
        let edge = USER_BASE + USER_LEN - 16;
        let (r, len) = sysctl(&[0, NET_RT_DUMP, 0, 0], edge, 4096);
        assert_eq!((r, len), (Err(Errno::EFAULT), 0usize.wrapping_sub(edge)));

        // The argument checks.
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: UDUMP,
            newlen: 4,
        };
        assert_eq!(
            sysctl_rtable::<G>(&[0, NET_RT_DUMP, 0, 0], &mut req),
            Err(Errno::EPERM)
        );
        assert_eq!(sysctl(&[0, NET_RT_DUMP], 0, 0).0, Err(Errno::EINVAL));
        assert_eq!(
            sysctl(&[0, NET_RT_DUMP, 0, 0, 0], 0, 0).0,
            Err(Errno::EINVAL)
        );
        assert_eq!(sysctl(&[0, NET_RT_DUMP, 0, 9], 0, 0).0, Err(Errno::ENOENT));
        assert_eq!(sysctl(&[0, 99, 0, 0], 0, 0).0, Err(Errno::EINVAL));

        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rt_ifa_purge(ifa, &net);
        drop(w);
        grace();
    }

    #[test]
    fn sysctl_lists_interfaces_names_tables_stats_and_sources() {
        let _g = testroutes_setup();
        let (ifp, _d, ifa) = iface([10, 0, 2, 15], [255, 255, 255, 0]);
        let index = ifp.if_index.get() as i32;
        // On the interface list, as if_attach puts it (the fixture attaches it to the map).
        IFNETLIST.insert_tail(ifp);

        // NET_RT_IFLIST: the interface, then its address.
        let buf = answer(&[0, NET_RT_IFLIST, index]);
        let msgs = messages(&buf);
        assert_eq!(msgs.len(), 2, "{msgs:?}");
        let (off, h) = msgs[0];
        assert_eq!(h.rtm_type, RTM_IFINFO);
        assert_eq!(usize::from(h.rtm_hdrlen), IF_MSGHDR);
        let ifm: IfMsghdr = hal::pod::at(&buf, off).expect("an interface message");
        assert_eq!(i32::from(ifm.ifm_index), index);
        assert_eq!(ifm.ifm_addrs, RTA_IFP);
        assert_eq!(ifm.ifm_flags, ifp.if_flags.get());
        assert_eq!(buf[off + IF_MSGHDR + 1], AF_LINK);
        let (off, h) = msgs[1];
        assert_eq!(h.rtm_type, RTM_NEWADDR);
        let ifam: IfaMsghdr = hal::pod::at(&buf, off).expect("an address message");
        assert_eq!(i32::from(ifam.ifam_index), index);
        assert_eq!(ifam.ifam_addrs, RTA_IFA | RTA_NETMASK);
        // RTAX_NETMASK comes first, then RTAX_IFA.
        let mask_len = roundup_long(usize::from(buf[off + IFA_MSGHDR]));
        assert_eq!(in_at(&buf, off + IFA_MSGHDR + mask_len), [10, 0, 2, 15]);
        // Only the link message for a family without addresses.
        assert_eq!(
            messages(&answer(&[i32::from(AF_INET6), NET_RT_IFLIST, index])).len(),
            1
        );
        // A short room: the estimate given back, ENOMEM.
        assert_eq!(
            sysctl(&[0, NET_RT_IFLIST, index], UDUMP, 8),
            (Err(Errno::ENOMEM), 0)
        );

        // NET_RT_IFNAMES: the index and the name.
        let names = answer(&[0, NET_RT_IFNAMES, index]);
        assert_eq!(names.len(), size_of::<IfNameindexMsg>());
        let ifn: IfNameindexMsg = hal::pod::at(&names, 0).expect("a name");
        assert_eq!(ifn.if_index, index as u32);
        ifp.if_xname.with_str(|n| {
            assert_eq!(&ifn.if_name[..n.len()], n);
            assert_eq!(ifn.if_name[n.len()], 0);
        });

        // NET_RT_TABLE: table 0 in routing domain 0; table 9 does not exist.
        assert_eq!(sysctl(&[0, NET_RT_TABLE, 0], UDUMP, 4), (Ok(()), 4));
        assert_eq!(user_bytes(UDUMP, 4), Some(vec![0; 4]));
        assert_eq!(sysctl(&[0, NET_RT_TABLE, 9], 0, 0).0, Err(Errno::ENOENT));

        // NET_RT_STATS: struct rtstat, one word per counter.
        let before = RTCOUNTERS.read()[3];
        RTCOUNTERS.inc(3);
        assert_eq!(sysctl(&[0, NET_RT_STATS, 0], UDUMP, 20), (Ok(()), 20));
        let s: Rtstat = hal::pod::at(&user_bytes(UDUMP, 20).expect("rtstat"), 0).expect("rtstat");
        assert_eq!(u64::from(s.rts_unreach), before + 1);

        // NET_RT_SOURCE: none set, then the interface's address, then none again.
        assert_eq!(sysctl(&[0, NET_RT_SOURCE, 0], 0, 0), (Ok(()), 0));
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        assert_eq!(
            rt_setsource(0, v(&sin([10, 0, 2, 99])), &net),
            Err(Errno::EINVAL)
        );
        rt_setsource(0, v(&sin([10, 0, 2, 15])), &net).expect("a source");
        drop(w);
        assert_eq!(answer(&[0, NET_RT_SOURCE, 0]), sin([10, 0, 2, 15]).to_vec());
        assert_eq!(sysctl(&[0, NET_RT_SOURCE, 9], 0, 0).0, Err(Errno::ENOENT));
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        rt_setsource(0, v(&sin([0, 0, 0, 0])), &net).expect("automatic");
        assert_eq!(
            rt_setsource(0, v(&[2, AF_LINK]), &net),
            Err(Errno::EAFNOSUPPORT)
        );
        drop(w);
        assert_eq!(sysctl(&[0, NET_RT_SOURCE, 0], 0, 0), (Ok(()), 0));
        let _ = ifa;
        IFNETLIST.remove(ifp);
    }

    #[test]
    fn the_route_domain_is_one_raw_protocol() {
        let d = RouteDomain::<G>::NEW;
        assert_eq!((d.family(), d.name()), (i32::from(PF_ROUTE), "route"));
        assert_eq!((d.rtoffset(), d.sasize(), d.legacy()), (0, 0, false));
        let p = d.protocols();
        assert_eq!(p.len(), 1);
        assert_eq!((p[0].ty(), p[0].protocol()), (SOCK_RAW as i16, 0));
        assert_eq!(p[0].flags(), PR_ATOMIC | PR_ADDR | PR_WANTRCVD);
        assert!(p[0].has_ctloutput() && p[0].usrreqs().is_some());
        assert_eq!(p[0].domain().family(), i32::from(PF_ROUTE));
    }
}
/* </TESTS> */
