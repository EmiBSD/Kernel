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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)if.h	8.1 (Berkeley) 6/10/93
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
 *	@(#)if.c	8.3 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! Network interfaces for `bsd`'s network code: `net/if.c` is the kernel's since N6.3a
//! (`kernel::net::if_`, with `<net/if.h>` in `hal::sys::net_if`), re-exported here under LZ's
//! paths, with the forwards that turn `bsd`'s parked mbufs into the kernel's boxes; what
//! stays `bsd`'s is what names `bsd`'s types until their slice moves them: the output
//! helpers over `bsd`'s route and socket address (`if_output_ml`, `if_output_tso`,
//! `if_output_mq`: N6.3b), the point-to-point link layer (`p2p_*`: N6.3b/c), the
//! implementation of the kernel's `NetDriver` for `bsd`'s drivers' tables
//! ([`LegacyNetDriver`]) and of the `Inet`, `Routing` and `Link` legs over the subsystems
//! that have not moved, and the raw entry [`ifioctl`] of `bsd`'s callers that hold a request
//! as a structure of their own (D-N6.3a-12). The interface requests themselves (`ifioctl`,
//! `ifconf`, the groups', the cloners' and the rings' requests) are the kernel's since
//! N6.3a2, reached from the socket file directly (the `IfLegacy` bridge is gone).
//!
//! Upstream: sys/net/if.h @ 3ce1f3f79392
//! Upstream: sys/net/if.c @ 3ce1f3f79392
//!
//! `if` is a Rust keyword, so the module is `if_` (`docs/C_TO_RUST.md`).
//!
//! ## Deviations
//! - The kernel's deviations (`kernel::net::if_`) are this module's for what it forwards.
//! - [`ifioctl`] keeps LZ's raw form (`struct socket *` a raw pointer, NULL for the kernel's
//!   own requests, and `caddr_t data` a raw pointer) for `nfs_boot.rs` and `selftest.rs`
//!   until they move (N6.7a, N9): an `unsafe fn` whose contract is the caller's structure,
//!   forwarded as its `IOCPARM_LEN(cmd)` bytes to the kernel's (D-N6.3a-12).
//! - The functions whose C arguments are socket addresses (`ifa_ifwithaddr`,
//!   `ifa_ifwithdstaddr`, `ifaof_ifpforaddr`, `if_group_routechange`, `ifa_update_broadaddr`)
//!   keep LZ's `unsafe fn` signatures over `*const Sockaddr` for their callers: each views the
//!   address's `sa_len` bytes and forwards (the kernel compares bytes).
//! - The three lookups among them (`ifa_ifwithaddr`, `ifa_ifwithdstaddr`, `ifaof_ifpforaddr`)
//!   `panic(9)` at the start of their walk when the net lock is not held, where the C's
//!   `NET_ASSERT_LOCKED()` (a splassert) only prints and goes on: the kernel's copy of an
//!   address needs the proof (`hal::netlock::NetHeld`, N6.3a1 review 1 B2, review 2 S-a);
//!   every caller the C has holds the lock.
//! - Calls into files that are not ported report themselves with `unported!` and go on as
//!   the C would with an empty subsystem: `inet_ntop` for `AF_INET` (`ifa_print_all`;
//!   `AF_INET6` prints with nd6's `In6Ntop`).

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::slice;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::kern::init_main::PDEVINIT_DONE;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::{Str, panic, printf};
use crate::kern::uipc_mbuf::{m_freem, ml_dequeue, ml_purge, mq_delist, mq_purge};
use crate::net::bpf::{BpfTap, bpf_mtap_af, bpf_mtap_ether};
use crate::net::if_var::{IfnetHead, IfnetLegacy, LegacyNetDriver, Niqueue};
use crate::net::route::{Rtentry, as_static};
use crate::netinet::ip_input::{ipintr, ipv4_input};
use crate::netinet::tcp_input::tcp_input_mlist;
use crate::netinet::tcp_output::tcp_if_output_tso;
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};
#[cfg(feature = "inet6")]
use crate::netinet6::in6::{in6_purgeaddr, in6ifa_ifpforlinklocal, satosin6_const};
#[cfg(feature = "inet6")]
use crate::netinet6::in6_ifattach::{in6_ifattach, in6_ifdetach};
#[cfg(feature = "inet6")]
use crate::netinet6::in6_proto::IP6_FORWARDING;
#[cfg(feature = "inet6")]
use crate::netinet6::ip6_input::{ip6intr, ipv6_input};
#[cfg(feature = "inet6")]
use crate::netinet6::ip6_output::in6_proto_cksum_out;
#[cfg(feature = "inet6")]
use crate::netinet6::nd6::{In6Ntop, nd6_ifattach, nd6_ifdetach, nd6_resolve, nd6_rtrequest};
use crate::sys::errno::Errno;
use crate::sys::ioccom::iocparm_len;
use crate::sys::mbuf::{M_TCP_TSO, Mbuf, MbufList, MbufQueue, ml_len, mq_len, park, unpark};
use crate::sys::proc::Proc;
use crate::sys::socket::{AF_INET, AF_INET6, Sockaddr};
use crate::sys::socketvar::Socket;
use crate::sys::task::Task;
use crate::sys::types::SaFamily;
use crate::{kassert, unported};

pub use hal::sys::net_if::*;
pub use kernel::net::if_::{
    ETHER_ADDR_LEN as IF_ETHER_ADDR_LEN, IFQ_CONGESTION, NET_TASKQ, Softnet, if_addgroup,
    if_alloc_sadl, if_attach_iqueues, if_attach_queues, if_clone_create, if_clone_destroy,
    if_congested, if_congestion, if_counters_alloc, if_counters_free, if_creategroup,
    if_createrdomain, if_deactivate, if_delgroup, if_detach, if_downall, if_free_sadl, if_get,
    if_getdata, if_group_egress_build, if_isconnected, if_link_state_change, if_ref,
    if_rxhprio_l2_check, if_rxhprio_l3_check, if_rxr_get, if_rxr_info_ioctl, if_rxr_init,
    if_rxr_ioctl, if_rxr_livelocked, if_setlladdr, if_setrdomain, if_txhprio_l2_check,
    if_txhprio_l3_check, if_unit, ifa_add, ifa_del, ifg_head, ifnewlladdr, ifpromisc, ifsetlro,
    net_sn, net_tq, net_tq_barriers, softnet_count, softnet_init, softnet_percpu, unhandled_af,
};
pub use kernel::net::if_var::{counters_inc, counters_pkt};

/// `ifnetlist`: all attached interfaces (the kernel's list), in LZ's tuple shape.
pub static IFNETLIST: IfnetHead = IfnetHead(&kernel::net::if_::IFNETLIST);

pub use kernel::net::if_::IF_TMPLIST_LOCK;

/// `(*ifp->if_ioctl)(ifp, cmd, data)`: the driver's request handler (`ENODEV` once the
/// interface is detached, the C's `if_detached_ioctl`). A kernel driver (the loopback's,
/// N6.3c2) gets a zeroed image of the command's `IOCPARM_LEN(cmd)` bytes instead of `data`
/// (see [`kernel_driver_ioctl`]).
///
/// # Safety
///
/// `data` points at the request's structure for `cmd`, aligned for it, readable and writable
/// for `IOCPARM_LEN(cmd)` bytes and readable through the largest union member a handler may
/// read (an `in6_ifreq` read through an `ifreq`), as
/// [`IfIoctlFn`](crate::net::if_var::IfIoctlFn)'s contract says. The raw callers pass whole,
/// real structures (`in_.rs`'s `ifaddr`s, `in6.rs`'s `in6_ifreq`s), which satisfy it.
pub unsafe fn ifp_ioctl(ifp: &'static Ifnet, cmd: u64, data: *mut u8) -> Result<(), Errno> {
    if ifp.if_ioctl_detached() {
        return Err(Errno::ENODEV);
    }
    if ifp.if_driver.get().is_none() {
        // No handler at all (before attach): the C's NULL `if_ioctl` slot's answer here.
        return Err(Errno::ENOTTY);
    }
    let Some(l) = ifp.legacy_opt() else {
        return kernel_driver_ioctl(ifp, cmd);
    };
    match l.if_ioctl.get() {
        // SAFETY: the caller's contract is the handler's.
        Some(ioctl) => unsafe { ioctl(ifp, cmd, data) },
        None => Err(Errno::ENOTTY),
    }
}

/// [`ifp_ioctl`] for an interface whose driver is the kernel's: the four requests the raw
/// callers make (`in_.rs`'s and `in6.rs`'s `SIOCSIFADDR` and `SIOCSIFDSTADDR`, whose `data` is
/// the protocol's `struct ifaddr`, not a request image; `in6.rs`'s `SIOCADDMULTI` and
/// `SIOCDELMULTI`, an `in6_ifreq`) reach the driver with a zeroed image of `IOCPARM_LEN(cmd)`
/// bytes, since the kernel's `NetDriver::ioctl` takes the request's bytes and `data` cannot be
/// read without `unsafe`. The one kernel driver they reach is the loopback's, which reads no
/// data for any of the four (if_loop.c:276-306); the image goes with `in_.rs` and `in6.rs`
/// (N6.4, N6.5: docs/SCAFFOLDING.md), which pass their own requests to the kernel's
/// `if_ioctl` then.
fn kernel_driver_ioctl(ifp: &'static Ifnet, cmd: u64) -> Result<(), Errno> {
    let mut image = [0u64; 8];
    let bytes = hal::user::bytes_of_mut(&mut image[..]);
    let n = (iocparm_len(cmd) as usize).min(bytes.len());
    kernel::net::if_::if_ioctl(ifp, cmd, &mut bytes[..n])
}

impl kernel::net::if_var::NetDriver for LegacyNetDriver {
    fn start(&self, ifp: &'static Ifnet) {
        if let Some(start) = self.if_start.get() {
            start(ifp);
        }
    }

    fn qstart(&self, ifq: &'static Ifqueue) {
        match self.if_qstart.get() {
            Some(qstart) => qstart(ifq),
            // if_attach_common's if_qstart_compat for a driver without IFXF_MPSAFE.
            None => kernel::net::if_::if_qstart_compat(self, ifq),
        }
    }

    fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
        // The handler reads and writes the command's structure, `IOCPARM_LEN(cmd)` bytes: a
        // shorter request is refused before the call (the C's callers always pass the
        // command's whole structure).
        if data.len() < iocparm_len(cmd) as usize {
            return Err(Errno::EINVAL);
        }
        let Some(ioctl) = self.if_ioctl.get() else {
            return Err(Errno::ENOTTY);
        };
        // The handlers view the request as its structure (an `Ifreq` is aligned to 8) and some
        // read past it (`ether_multiaddr` takes a `sockaddr_in6` out of a 32-byte `ifreq`, as
        // the C's handlers read `in6_ifreq`s through `struct ifreq *`): a request of up to 64
        // bytes runs on an aligned, zeroed copy of 64, copied back after; a longer one must be
        // aligned already (sys_ioctl's buffers are), or it is refused (review 2's B5-r).
        let mut copy = [0u64; 8];
        let small = data.len() <= size_of_val(&copy);
        let req = if small {
            let bytes = hal::user::bytes_of_mut(&mut copy[..]);
            bytes[..data.len()].copy_from_slice(data);
            bytes.as_mut_ptr()
        } else if data.as_ptr().addr().is_multiple_of(align_of::<u64>()) {
            data.as_mut_ptr()
        } else {
            return Err(Errno::EINVAL);
        };
        // SAFETY: `req` is aligned to 8 (a `[u64; 8]`, or `data` checked above) and names at
        // least `IOCPARM_LEN(cmd)` readable and writable bytes (`data`'s length is checked
        // above; the copy is 64 bytes, zero past `data`'s), followed, for a request of up to 64
        // bytes, by zeroed bytes up to 64, so a handler's view of the command's structure, or
        // of the longer address an `ifreq`'s union carries, stays inside it. Nothing else reads
        // or writes those bytes during the call (`copy` is this frame's, `data` is borrowed
        // exclusively). That is the handlers' contract (`IfIoctlFn`).
        let r = unsafe { ioctl(ifp, cmd, req) };
        if small {
            let n = data.len();
            data.copy_from_slice(&hal::user::bytes_of(&copy[..])[..n]);
        }
        r
    }

    fn has_watchdog(&self) -> bool {
        self.if_watchdog.get().is_some()
    }

    fn watchdog(&self, ifp: &'static Ifnet) {
        if let Some(watchdog) = self.if_watchdog.get() {
            watchdog(ifp);
        }
    }

    fn wol(&self, ifp: &'static Ifnet, on: bool) -> Option<Result<(), Errno>> {
        self.if_wol.get().map(|wol| wol(ifp, on))
    }

    fn enqueue(&self, ifp: &'static Ifnet, m: kernel::net::mbuf::MbufBox) -> Result<(), Errno> {
        match self.if_enqueue.get() {
            Some(enqueue) => enqueue(ifp, park(m)),
            None => kernel::net::if_::if_enqueue_ifq(ifp, m),
        }
    }

    fn input(&self, ifp: &'static Ifnet, m: kernel::net::mbuf::MbufBox, ns: Option<&mut Netstack>) {
        match self.if_input.get() {
            Some(input) => input(ifp, park(m), ns),
            // The link layer's (ether_input, set by ether_ifattach), else panic(9).
            None => kernel::net::if_var::link_input(ifp, m, ns),
        }
    }

    fn bpf_mtap(&self, if_bpf: &BpfTap, m: &Mbuf, dir: u32) -> bool {
        match self.if_bpf_mtap.get() {
            Some(mtap) => mtap(if_bpf, m, dir),
            None => bpf_mtap_ether(if_bpf, m, dir),
        }
    }

    fn rtrequest(
        &self,
        ifp: &'static Ifnet,
        req: u8,
        rt: Option<&kernel::net::route::Rtentry>,
        net: &hal::netlock::NetHeld<'_>,
    ) {
        match self.if_rtrequest.get() {
            Some(rtrequest) => rtrequest(ifp, i32::from(req), rt.map(as_static)),
            // The link layer's (ether_rtrequest), else nothing: if_rtrequest_dummy.
            None => kernel::net::if_var::link_rtrequest(ifp, req, rt, net),
        }
    }

    fn output(
        &self,
        ifp: &'static Ifnet,
        m: kernel::net::mbuf::MbufBox,
        dst: kernel::net::proto::SockaddrView<'_>,
        rt: Option<&kernel::net::route::Rtentry>,
    ) -> Result<(), Errno> {
        let Some(output) = self.if_output.get() else {
            // The link layer's (ether_output, which ether_ifattach installs when the driver
            // set none), else panic(9).
            return kernel::net::if_var::link_output(ifp, m, dst, rt);
        };
        let dst = dst.bytes();
        // SAFETY: `dst` is a `SockaddrView`'s bytes, at least its `sa_len` of them (the
        // view's invariant) and borrowed for the call, so the handler reads a socket address
        // valid for its `sa_len` bytes, `IfOutputFn`'s contract; the packet is parked for it
        // (`bsd`'s handlers take a `&'static Mbuf` they free or queue).
        unsafe { output(ifp, park(m), dst.as_ptr().cast(), rt.map(as_static)) }
    }

    fn as_any(&self) -> Option<&(dyn core::any::Any + Send + Sync)> {
        Some(self)
    }
}

/// The legs of the subsystems `if.c` and `if_ethersubr.c` call that are still `bsd`'s: the
/// internet protocols (`Inet`), registered by [`ifinit`] (the link layers are the kernel's
/// since N6.3c2).
pub struct BsdNetLegs;

/// The one [`BsdNetLegs`].
pub static BSD_NET_LEGS: BsdNetLegs = BsdNetLegs;

/// `ipv4_input` as the kernel's `if_input_proto` takes it.
pub fn ipv4_input_proto(
    ifp: &'static Ifnet,
    m: kernel::net::mbuf::MbufBox,
    ns: Option<&mut Netstack>,
) {
    ipv4_input(ifp, park(m), ns);
}

/// `ipv6_input` as the kernel's `if_input_proto` takes it.
#[cfg(feature = "inet6")]
pub fn ipv6_input_proto(
    ifp: &'static Ifnet,
    m: kernel::net::mbuf::MbufBox,
    ns: Option<&mut Netstack>,
) {
    ipv6_input(ifp, park(m), ns);
}

impl kernel::net::if_::Inet for BsdNetLegs {
    fn ipintr(&self) {
        ipintr();
    }

    fn ip6intr(&self) {
        #[cfg(feature = "inet6")]
        ip6intr();
    }

    fn tcp_input_mlist(&self, ml: &mut kernel::net::mbuf::MbufList, af: SaFamily) {
        let l = MbufList::new();
        l.change(|b| kernel::net::mbuf::ml_enlist(b, ml));
        tcp_input_mlist(&l, i32::from(af));
    }

    fn ipv4_input(&self) -> kernel::net::if_::ProtoFn {
        ipv4_input_proto
    }

    fn ipv6_input(&self) -> kernel::net::if_::ProtoFn {
        #[cfg(feature = "inet6")]
        return ipv6_input_proto;
        #[cfg(not(feature = "inet6"))]
        ipv4_input_proto
    }

    fn tcpstat_lro(&self, hw: bool) {
        tcpstat_inc(if hw {
            TcpstatCounters::TcpsInhwlro
        } else {
            TcpstatCounters::TcpsInbadlro
        });
    }

    fn nd6_ifattach(&self, ifp: &'static Ifnet) {
        #[cfg(feature = "inet6")]
        nd6_ifattach(ifp);
        let _ = ifp;
    }

    fn nd6_ifdetach(&self, ifp: &'static Ifnet) {
        #[cfg(feature = "inet6")]
        nd6_ifdetach(ifp);
        let _ = ifp;
    }

    fn in6_ifdetach(&self, ifp: &'static Ifnet) {
        #[cfg(feature = "inet6")]
        in6_ifdetach(ifp);
        let _ = ifp;
    }

    fn in6_ifattach(&self, ifp: &'static Ifnet) -> Result<(), Errno> {
        #[cfg(feature = "inet6")]
        return in6_ifattach(ifp);
        #[cfg(not(feature = "inet6"))]
        {
            let _ = ifp;
            Err(Errno::EAFNOSUPPORT)
        }
    }

    fn ip6_forwarding(&self) -> bool {
        #[cfg(feature = "inet6")]
        return IP6_FORWARDING.load(Ordering::Relaxed) != 0;
        #[cfg(not(feature = "inet6"))]
        false
    }

    fn ifa_free(&self, ifa: &'static Ifaddr) {
        // free(ifa, M_IFADDR, 0): the protocol's in_ifaddr/in6_ifaddr whose ifaddr this is.
        crate::kern::kern_malloc::free(
            core::ptr::NonNull::from(ifa).cast(),
            crate::sys::malloc::M_IFADDR,
            0,
        );
    }

    fn ip6_multipath(&self) -> bool {
        #[cfg(feature = "inet6")]
        return crate::netinet6::in6_proto::IP6_MULTIPATH.load(Ordering::Relaxed) != 0;
        #[cfg(not(feature = "inet6"))]
        false
    }

    fn ip6stat_rtcache(&self, hit: bool) {
        #[cfg(feature = "inet6")]
        {
            use crate::netinet6::ip6_var::{Ip6statCounters, ip6stat_inc};
            ip6stat_inc(if hit {
                Ip6statCounters::Ip6sRtcachehit
            } else {
                Ip6statCounters::Ip6sRtcachemiss
            });
        }
        let _ = hit;
    }

    fn nd6_resolve(
        &self,
        ifp: &'static Ifnet,
        rt: Option<&kernel::net::route::Rtentry>,
        m: kernel::net::mbuf::MbufBox,
        dst: kernel::net::proto::SockaddrView<'_>,
        desten: &mut [u8; IF_ETHER_ADDR_LEN],
    ) -> Result<kernel::net::mbuf::MbufBox, Errno> {
        #[cfg(feature = "inet6")]
        {
            if dst.bytes().len() < size_of::<crate::netinet6::in6::SockaddrIn6>() {
                drop(m);
                return Err(Errno::EINVAL);
            }
            let m = park(m);
            // SAFETY: `dst`'s bytes, borrowed for the call, hold a whole `sockaddr_in6`
            // (checked above); `nd6_resolve` reads it once with `read_unaligned`
            // (`sys/netinet6/nd6.rs`), so the view's alignment of 1 is enough, and nothing
            // writes the bytes meanwhile. The packet is parked as for `arpresolve`.
            unsafe {
                nd6_resolve(
                    ifp,
                    rt.map(as_static),
                    m,
                    dst.bytes().as_ptr().cast(),
                    desten,
                )
            }?;
            Ok(unpark(m))
        }
        #[cfg(not(feature = "inet6"))]
        {
            let _ = (ifp, rt, dst, desten);
            drop(m);
            Err(Errno::EAFNOSUPPORT)
        }
    }

    fn nd6_rtrequest(&self, ifp: &'static Ifnet, req: u8, rt: &kernel::net::route::Rtentry) {
        #[cfg(feature = "inet6")]
        nd6_rtrequest(ifp, i32::from(req), as_static(rt));
        let _ = (ifp, req, rt);
    }

    fn in6_proto_cksum_out(&self, m: &Mbuf, ifp: &'static Ifnet) {
        #[cfg(feature = "inet6")]
        in6_proto_cksum_out(m, Some(ifp));
        let _ = (m, ifp);
    }

    fn tcp_if_output_tso(
        &self,
        ifp: &'static Ifnet,
        mp: &mut Option<kernel::net::mbuf::MbufBox>,
        dst: kernel::net::proto::SockaddrView<'_>,
        rt: Option<&kernel::net::route::Rtentry>,
        ifcap: u32,
        mtu: u32,
    ) -> Result<(), Errno> {
        // The C's first test (`caller must fail later or fragment`), made before the packet
        // is lent to `bsd`: a packet that asked for no TSO stays the kernel's.
        if mp
            .as_ref()
            .is_none_or(|m| m.m_pkthdr().csum_flags.get() & M_TCP_TSO == 0)
        {
            return Ok(());
        }
        let mut lent = mp.take().map(park);
        // SAFETY: `dst` is the kernel's view of a socket address, readable for its `sa_len`
        // bytes (the view's length, at least the two of its header) while the borrow lasts,
        // which outlives this call: `tcp_if_output_tso`'s contract (`if_output`'s).
        let error = unsafe {
            tcp_if_output_tso(
                ifp,
                &mut lent,
                dst.bytes().as_ptr().cast::<Sockaddr>(),
                rt.map(as_static),
                ifcap,
                mtu,
            )
        };
        *mp = lent.map(unpark);
        error
    }

    fn proto_outswcsum(&self, tcp: bool) {
        if tcp {
            tcpstat_inc(TcpstatCounters::TcpsOutswcsum);
        } else {
            crate::netinet::udp_var::udpstat_inc(
                crate::netinet::udp_var::UdpstatCounters::UdpsOutswcsum,
            );
        }
    }

    fn icmp_mtudisc_clone(
        &self,
        dst: [u8; 4],
        rtableid: u32,
        ipsec: bool,
    ) -> Option<hal::pool::PoolArc<kernel::net::route::Rtentry>> {
        let dst = crate::netinet::in_::InAddr {
            s_addr: u32::from_ne_bytes(dst),
        };
        crate::netinet::ip_icmp::icmp_mtudisc_clone(dst, rtableid, ipsec)
            .map(crate::net::route::unpark)
    }

    fn in6_purge_linklocal(&self, ifp: &'static Ifnet) -> bool {
        #[cfg(feature = "inet6")]
        if let Some(ia6) = in6ifa_ifpforlinklocal(ifp, 0) {
            in6_purgeaddr(&ia6.ia_ifa);
            return true;
        }
        let _ = ifp;
        false
    }
}

/// `ifinit`: the kernel's interface layer made (its index map) and the legs of `bsd`'s
/// subsystems registered (a second call, a test's, keeps the first).
pub fn ifinit() {
    let _ = kernel::net::if_::register_inet(&BSD_NET_LEGS);
    if let Ok(key) = kernel::net::ipsec_hooks::register_ipsec(&crate::net::if_enc::BSD_IPSEC) {
        let _ = crate::net::if_enc::BSD_TDB_KEY.set(key);
    }
    kernel::net::if_::ifinit();
}

/// `if_attach_common`'s check of `if_start`/`if_qstart` against `IFXF_MPSAFE`, before the
/// kernel attaches.
fn if_attach_legacy(ifp: &'static Ifnet) {
    // KASSERT(ifp->if_ioctl != NULL): every driver answers requests (review nit: a missing
    // handler answered ENOTTY where the C stops the attach).
    kassert!(ifp.if_ioctl().get().is_some());
    // if_rtrequest_dummy is the empty slot's answer (LegacyNetDriver::rtrequest), so the link
    // layer ether_ifattach sets later still gets the routes (N6.3c2).
    if ifp.if_xflags.get() & IFXF_MPSAFE == 0 {
        // KASSERTMSG: "%s: if_qstart set without MPSAFE set".
        kassert!(ifp.if_qstart().get().is_none());
    } else {
        // KASSERTMSG: "%s: if_start set with MPSAFE set".
        kassert!(ifp.if_start().get().is_none());
        // KASSERTMSG: "%s: if_qstart not set with MPSAFE set".
        kassert!(ifp.if_qstart().get().is_some());
    }
}

/// `if_attach(ifp)`: a network interface attached to the system (the kernel's, after
/// `bsd`'s table's defaults).
pub fn if_attach(ifp: &'static Ifnet) {
    if_attach_legacy(ifp);
    kernel::net::if_::if_attach(ifp);
}

/// `if_attachhead(ifp)`: as [`if_attach`], at the head of `ifnetlist` (`lo0`).
pub fn if_attachhead(ifp: &'static Ifnet) {
    if_attach_legacy(ifp);
    kernel::net::if_::if_attachhead(ifp);
}

/// `if_attach_ifq(ifp, newops, args)`: the first send queue's conditioner switched to one
/// of pf's (`ifq_priq_ops` is `ifq_attach_priq`).
pub fn if_attach_ifq(ifp: &Ifnet, newops: &'static IfqOps, args: *mut c_void) {
    crate::net::ifq::ifq_attach(ifp.if_snd(), newops, args);
}

/// `if_start(ifp)`: the start routine of a driver without `IFXF_MPSAFE`.
pub fn if_start(ifp: &'static Ifnet) {
    kernel::net::if_::if_start(ifp);
}

/// `if_enqueue(ifp, m)`: a finished packet handed to the interface for transmission.
pub fn if_enqueue(ifp: &'static Ifnet, m: &'static Mbuf) -> Result<(), Errno> {
    kernel::net::if_::if_enqueue(ifp, unpark(m))
}

/// `if_enqueue_ifq(ifp, m)`: the default `if_enqueue`.
pub fn if_enqueue_ifq(ifp: &'static Ifnet, m: &'static Mbuf) -> Result<(), Errno> {
    kernel::net::if_::if_enqueue_ifq(ifp, unpark(m))
}

/// `if_input(ifp, ml)`: a driver's received packets, onto its first input queue.
pub fn if_input(ifp: &'static Ifnet, ml: &MbufList) {
    ml.change(|l| kernel::net::if_::if_input(ifp, l));
}

/// `if_input_local(ifp, m, af, ns)`: a packet for a local address received again.
pub fn if_input_local(
    ifp: &'static Ifnet,
    m: &'static Mbuf,
    af: SaFamily,
    ns: Option<&mut Netstack>,
) -> Result<(), Errno> {
    kernel::net::if_::if_input_local(ifp, unpark(m), af, ns)
}

/// `if_output_local(ifp, m, af)`: a packet queued for local delivery on `ifp`.
pub fn if_output_local(ifp: &'static Ifnet, m: &'static Mbuf, af: SaFamily) -> Result<(), Errno> {
    kernel::net::if_::if_output_local(ifp, unpark(m), af)
}

/// `if_vinput(ifp, m, ns)`: a packet a virtual interface received from its parent.
pub fn if_vinput(ifp: &'static Ifnet, m: &'static Mbuf, ns: Option<&mut Netstack>) {
    kernel::net::if_::if_vinput(ifp, unpark(m), ns);
}

/// `if_input_proto(ifp, m, input, ns)`: `m` for the protocol function `input`.
pub fn if_input_proto(
    ifp: &'static Ifnet,
    m: &'static Mbuf,
    input: kernel::net::if_::ProtoFn,
    ns: Option<&mut Netstack>,
) {
    kernel::net::if_::if_input_proto(ifp, unpark(m), input, ns);
}

/// `if_get_smr(index)`: the interface at `index` for a caller inside an SMR read section,
/// no reference taken.
pub fn if_get_smr(index: u32) -> Option<&'static Ifnet> {
    kernel::net::if_::if_get_smr(index, &hal::smr::read())
}

/// `if_put(ifp)`: one reference less (NULL is nothing).
pub fn if_put<'a>(ifp: impl Into<Option<&'a Ifnet>>) {
    kernel::net::if_::if_put(ifp.into());
}

/// `if_clone_attach(ifc)`: a cloner registered, by `main()` while the pseudo-devices attach.
pub fn if_clone_attach(ifc: &'static IfClone) {
    kassert!(!PDEVINIT_DONE.load(Ordering::Relaxed));
    kernel::net::if_::if_clone_attach(ifc);
}

/// `if_clone_lookup(name)`: the cloner of a name and the unit it asks for.
pub fn if_clone_lookup(name: &[u8]) -> Option<(&'static dyn kernel::net::if_var::Cloner, i32)> {
    kernel::net::if_::if_clone_lookup(name)
}

/// `if_addrhook_add(ifp, t)`: an address change hook (`t` stays linked until removed).
pub fn if_addrhook_add(ifp: &Ifnet, t: &'static Task) {
    kernel::net::if_::if_addrhook_add(ifp, Pin::static_ref(&t.t_node));
}

/// `if_addrhook_del(ifp, t)`.
pub fn if_addrhook_del(ifp: &Ifnet, t: &Task) {
    kernel::net::if_::if_addrhook_del(ifp, &t.t_node);
}

/// `if_addrhooks_run(ifp)`.
pub fn if_addrhooks_run(ifp: &Ifnet) {
    kernel::net::if_::if_addrhooks_run(ifp);
}

/// `if_linkstatehook_add(ifp, t)`.
pub fn if_linkstatehook_add(ifp: &Ifnet, t: &'static Task) {
    kernel::net::if_::if_linkstatehook_add(ifp, Pin::static_ref(&t.t_node));
}

/// `if_linkstatehook_del(ifp, t)`.
pub fn if_linkstatehook_del(ifp: &Ifnet, t: &Task) {
    kernel::net::if_::if_linkstatehook_del(ifp, &t.t_node);
}

/// `if_detachhook_add(ifp, t)`.
pub fn if_detachhook_add(ifp: &Ifnet, t: &'static Task) {
    kernel::net::if_::if_detachhook_add(ifp, Pin::static_ref(&t.t_node));
}

/// `if_detachhook_del(ifp, t)`.
pub fn if_detachhook_del(ifp: &Ifnet, t: &Task) {
    kernel::net::if_::if_detachhook_del(ifp, &t.t_node);
}

/// A socket address's `sa_len` bytes.
///
/// Its two header bytes at least, whatever `sa_len` says: the C reads `sa_family` of an
/// address of `sa_len` 0 or 1 too, and `equal()` then compares no byte (review nit).
///
/// # Safety
///
/// `sa` points at a readable socket address of its two header bytes and its `sa_len`
/// bytes, which live for `'a`.
unsafe fn sa_bytes<'a>(sa: *const Sockaddr) -> &'a [u8] {
    // SAFETY: the caller's contract.
    unsafe { slice::from_raw_parts(sa.cast::<u8>(), usize::from((*sa).sa_len).max(2)) }
}

/// The proof that `bsd`'s caller holds the net lock (detached), for one walk of the kernel's
/// address lookups: `hal`'s check through the `SaToken` (the C's `NET_ASSERT_LOCKED()`);
/// `panic(9)` without it, where the C's splassert would only print.
pub(crate) fn net_held(what: &str) -> hal::netlock::NetHeld<'static> {
    match crate::net::if_var::sa_token().net_held() {
        Some(h) => h,
        None => panic(format_args!("{what}: the net lock is not held")),
    }
}

/// `ifa_ifwithaddr(addr, rtableid)` of a socket address's bytes (a routing table's preferred
/// source, a copy): the kernel's, under `bsd`'s net lock.
pub fn ifa_ifwithaddr_sa(addr: &[u8], rtableid: u32) -> Option<&'static Ifaddr> {
    kernel::net::if_::ifa_ifwithaddr(&net_held("ifa_ifwithaddr"), addr, rtableid)
}

/// `ifa_ifwithaddr(addr, rtableid)`: locate an interface based on a complete address.
///
/// # Safety
///
/// `addr` points at a readable socket address of `sa_len` bytes.
pub unsafe fn ifa_ifwithaddr(addr: *const Sockaddr, rtableid: u32) -> Option<&'static Ifaddr> {
    // SAFETY: the caller's contract.
    kernel::net::if_::ifa_ifwithaddr(
        &net_held("ifa_ifwithaddr"),
        unsafe { sa_bytes(addr) },
        rtableid,
    )
}

/// `ifa_ifwithdstaddr(addr, rdomain)`: the point-to-point interface with a given
/// destination address.
///
/// # Safety
///
/// As for [`ifa_ifwithaddr`].
pub unsafe fn ifa_ifwithdstaddr(addr: *const Sockaddr, rdomain: u32) -> Option<&'static Ifaddr> {
    // SAFETY: the caller's contract.
    kernel::net::if_::ifa_ifwithdstaddr(
        &net_held("ifa_ifwithdstaddr"),
        unsafe { sa_bytes(addr) },
        rdomain,
    )
}

/// `ifaof_ifpforaddr(addr, ifp)`: the address of `ifp` that best matches `addr`.
///
/// # Safety
///
/// As for [`ifa_ifwithaddr`].
pub unsafe fn ifaof_ifpforaddr(addr: *const Sockaddr, ifp: &Ifnet) -> Option<&'static Ifaddr> {
    // SAFETY: the caller's contract.
    kernel::net::if_::ifaof_ifpforaddr(
        &net_held("ifaof_ifpforaddr"),
        unsafe { sa_bytes(addr) },
        ifp,
    )
}

/// `if_group_routechange(dst, mask)`: the egress group built again after a default route
/// changed.
///
/// # Safety
///
/// `dst` and, when not NULL, `mask` point at readable socket addresses of their `sa_len`
/// bytes.
pub unsafe fn if_group_routechange(dst: *const Sockaddr, mask: *const Sockaddr) {
    // SAFETY: the caller's contract.
    let (dst, mask) = unsafe { (sa_bytes(dst), (!mask.is_null()).then(|| sa_bytes(mask))) };
    kernel::net::if_::if_group_routechange(dst, mask);
}

/// `niq_enqueue`: network stack input queues: queues the packet and schedules its software
/// interrupt; `true` (the C's nonzero) when the queue was full and the packet dropped.
pub fn niq_enqueue(niq: &Niqueue, m: &'static Mbuf) -> bool {
    let rv = crate::kern::uipc_mbuf::mq_enqueue(&niq.ni_q, m);
    if !rv {
        kernel::net::netisr::schednetisr::<BsdProcGlue>(niq.ni_isr);
    } else {
        if_congestion();
    }

    rv
}

pub use crate::net::if_var::{IfClone, Ifaddr, Ifnet, Netstack};
pub use crate::net::ifq::{IfqOps, Ifqueue, ifq_is_oactive};

// net/if.c, kept (LZ's shape until its slice).
/// `if_output_ml`: sends a list of packets through `if_output`; on the first error the rest
/// are freed.
///
/// # Safety
///
/// As for [`IfOutputFn`].
pub unsafe fn if_output_ml(
    ifp: &'static Ifnet,
    ml: &MbufList,
    dst: *const Sockaddr,
    rt: Option<&'static Rtentry>,
) -> Result<(), Errno> {
    let mut error = Ok(());

    while let Some(m) = ml_dequeue(ml) {
        // SAFETY: the caller's contract is `if_output`'s.
        error = unsafe { ifp_output(ifp, m, dst, rt) };
        if error.is_err() {
            break;
        }
    }
    if error.is_err() {
        let _ = ml_purge(ml);
    }

    error
}

/// `(*ifp->if_output)(ifp, m, dst, rt)`: through the interface's driver (`bsd`'s table, a
/// kernel driver, or the link layer behind either: the loopback's, N6.3c2), the packet given
/// to the kernel.
///
/// # Safety
///
/// As for [`IfOutputFn`]: `dst` points at a readable socket address of its `sa_len` bytes.
pub(crate) unsafe fn ifp_output(
    ifp: &'static Ifnet,
    m: &'static Mbuf,
    dst: *const Sockaddr,
    rt: Option<&'static Rtentry>,
) -> Result<(), Errno> {
    // SAFETY: the caller's contract: `dst` is readable for its `sa_len` bytes (at least the
    // two of its header), borrowed for the call.
    let sa = unsafe { sa_bytes(dst) };
    let Some(dst) = kernel::net::proto::SockaddrView::new(sa) else {
        panic(format_args!(
            "{}: if_output to no address",
            Str(&ifp.if_xname.get())
        ));
    };
    let Some(&d) = ifp.if_driver.get() else {
        panic(format_args!("{}: no if_output", Str(&ifp.if_xname.get())));
    };
    d.output(ifp, unpark(m), dst, rt.map(|r| r as &_))
}

/// `if_output_tso`: sends a TCP packet with TSO, or chops it, or sends it whole when it
/// fits `mtu`; `*mp` is left set when the packet still has to be fragmented or dropped (the
/// kernel's, N6.4a: the parked packet lent to it and parked again if it comes back).
///
/// # Safety
///
/// As for [`IfOutputFn`].
pub unsafe fn if_output_tso(
    ifp: &'static Ifnet,
    mp: &mut Option<&'static Mbuf>,
    dst: *const Sockaddr,
    rt: Option<&'static Rtentry>,
    mtu: u32,
) -> Result<(), Errno> {
    // SAFETY: the caller's contract: `dst` is readable for its `sa_len` bytes (at least the
    // two of its header), borrowed for the call.
    let sa = unsafe { sa_bytes(dst) };
    let Some(dst) = kernel::net::proto::SockaddrView::new(sa) else {
        panic(format_args!(
            "{}: if_output to no address",
            Str(&ifp.if_xname.get())
        ));
    };
    let mut lent = mp.take().map(unpark);
    let error = kernel::net::if_::if_output_tso(ifp, &mut lent, dst, rt.map(|r| r as &_), mtu);
    *mp = lent.map(park);
    error
}

/// `if_output_mq`: sends the packets of a queue; `total` (the packets accounted to the queue
/// elsewhere) drops by what was sent and by what was discarded.
///
/// # Safety
///
/// As for [`IfOutputFn`].
pub unsafe fn if_output_mq(
    ifp: &'static Ifnet,
    mq: &MbufQueue,
    total: &AtomicU32,
    dst: *const Sockaddr,
    rt: Option<&'static Rtentry>,
) -> Result<(), Errno> {
    let ml = MbufList::new();

    mq_delist(mq, &ml);
    let len = ml_len(&ml);
    // SAFETY: the caller's contract.
    let error = unsafe { if_output_ml(ifp, &ml, dst, rt) };

    // XXXSMP we also discard if other CPU enqueues
    if mq_len(mq) > 0 {
        // mbuf is back in queue. Discard.
        total.fetch_sub(len + mq_purge(mq), Ordering::Relaxed);
    } else {
        total.fetch_sub(len, Ordering::Relaxed);
    }

    error
}

// net/if.c, kept (LZ's shape until its slice).

/// `p2p_rtrequest`: default action when installing a local route on a point-to-point
/// interface (the kernel's, under `bsd`'s net lock).
pub fn p2p_rtrequest(ifp: &'static Ifnet, req: i32, rt: Option<&'static Rtentry>) {
    if let Some(rt) = rt {
        kernel::net::if_::p2p_rtrequest(ifp, req as u8, rt, &net_held("p2p_rtrequest"));
    }
}

/// `if_up(ifp)` under `bsd`'s net lock (the kernel's, the proof made through the token).
pub fn if_up(ifp: &'static Ifnet) {
    kernel::net::if_::if_up(ifp, &net_held("if_up"));
}

/// `if_down(ifp)` under `bsd`'s net lock.
pub fn if_down(ifp: &'static Ifnet) {
    kernel::net::if_::if_down(ifp, &net_held("if_down"));
}

/// `if_linkstate(ifp)` under `bsd`'s net lock.
pub fn if_linkstate(ifp: &'static Ifnet) {
    kernel::net::if_::if_linkstate(ifp, &net_held("if_linkstate"));
}

/// `p2p_bpf_mtap`: a point-to-point interface's tap, by the packet's address family.
pub fn p2p_bpf_mtap(if_bpf: &BpfTap, m: &Mbuf, dir: u32) -> bool {
    bpf_mtap_af(if_bpf, u32::from(m.m_pkthdr().ph_family.get()), m, dir)
}

/// `p2p_input`: a point-to-point interface's input, by the packet's address family.
pub fn p2p_input(ifp: &'static Ifnet, m: &'static Mbuf, ns: Option<&mut Netstack>) {
    match m.m_pkthdr().ph_family.get() {
        AF_INET => if_input_proto(ifp, m, ipv4_input_proto, ns),
        #[cfg(feature = "inet6")]
        AF_INET6 => if_input_proto(ifp, m, ipv6_input_proto, ns),
        // MPLS: mpls_input; MPLS is not configured.
        _ => {
            m_freem(m);
        }
    }
}

/// `ifioctl(so, cmd, data, p)` for `bsd`'s callers that hold the request as a structure of
/// their own and the socket as a raw pointer (`nfs_boot_init`'s, the boot self-test's; NULL:
/// a request of the kernel itself): the kernel's [`kernel::net::if_::ifioctl`] on the
/// request's `IOCPARM_LEN(cmd)` bytes (D-N6.3a-12, until those callers move).
///
/// # Safety
///
/// `data` points at `IOCPARM_LEN(cmd)` readable and writable bytes (the command's structure,
/// which the caller owns and nothing else uses during the call); `so` is NULL or a socket
/// that lives across the call.
pub unsafe fn ifioctl(so: *const c_void, cmd: u64, data: *mut u8, p: &Proc) -> Result<(), Errno> {
    // SAFETY: the caller's contract: `so` is NULL (`as_ref` answers `None`) or a live socket,
    // borrowed shared for the call as the kernel's own references to it are; `data` names
    // `IOCPARM_LEN(cmd)` bytes the caller owns and lends exclusively for the call, so the
    // mutable slice over them is the only reference to them while it lives (the kernel's
    // `ifioctl` keeps none past the call).
    let (so, data) = unsafe {
        (
            so.cast::<Socket>().as_ref(),
            slice::from_raw_parts_mut(data, iocparm_len(cmd) as usize),
        )
    };
    kernel::net::if_::ifioctl::<BsdProcGlue>(so, cmd, data, p)
}

// net/if.c, kept (LZ's shape until its slice).
/// `ifa_update_broadaddr`: replaces the broadcast address in place.
///
/// # Safety
///
/// `sa` points at a readable socket address; the address's broadcast address is a valid one.
pub unsafe fn ifa_update_broadaddr(_ifp: &Ifnet, ifa: &Ifaddr, sa: *const Sockaddr) {
    let broad = ifa.ifa_broadaddr().get();
    // SAFETY: the caller's contract.
    unsafe {
        if (*broad).sa_len != (*sa).sa_len {
            panic(format_args!(
                "ifa_update_broadaddr does not support dynamic length"
            ));
        }
        ptr::copy(
            sa.cast::<u8>(),
            broad.cast::<u8>(),
            usize::from((*sa).sa_len),
        );
    }
}

/// `ifa_print_all` (`DDB`): debug function, can be called from ddb>.
pub fn ifa_print_all() {
    for ifp in IFNETLIST.0.iter() {
        for ifa in ifp.if_addrlist.iter() {
            // SAFETY: an interface's addresses are valid sockaddrs.
            match unsafe { (*ifa.ifa_addr.get()).sa_family } {
                AF_INET => {
                    // printf("%s", inet_ntop(AF_INET, &satosin(ifa->ifa_addr)->sin_addr, ...)):
                    // inet_ntop (lib/libkern) is not ported.
                    let _ = unported!("inet_ntop");
                }
                #[cfg(feature = "inet6")]
                AF_INET6 => {
                    // SAFETY: an AF_INET6 address is a `sockaddr_in6`; read unaligned.
                    let a = unsafe {
                        ptr::read_unaligned(
                            &raw const (*satosin6_const(ifa.ifa_addr.get())).sin6_addr,
                        )
                    };
                    printf(format_args!("{}", In6Ntop(a)));
                }
                _ => {}
            }
            printf(format_args!(" on {}\n", Str(&ifp.if_xname.get())));
        }
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use core::mem::MaybeUninit;

    use super::*;
    use crate::reftest::{assert_complete, assert_defines};
    use crate::sys::mbuf::mtod;
    use crate::sys::sockio::*;
    use crate::sys::systm::{net_lock, net_unlock};

    #[test]
    fn ifreq_union_accessors() {
        let mut ifr = Ifreq::zeroed();
        ifr.ifr_name[..4].copy_from_slice(b"vio0");
        ifr.set_ifr_flags((IFF_UP | IFF_BROADCAST) as i16);
        assert_eq!(ifr.ifr_flags(), 0x3);
        ifr.set_ifr_mtu(1500);
        assert_eq!(ifr.ifr_metric(), 1500);
        assert_eq!(ifr.ifr_hardmtu(), 1500);
        ifr.ifr_addr_mut().sa_len = 16;
        ifr.ifr_addr_mut().sa_family = 2;
        assert_eq!(ifr.ifr_dstaddr().sa_family, 2);
        assert_eq!(ifr.ifr_broadaddr().sa_len, 16);
        // The union starts right after the name, as in C.
        assert_eq!(core::mem::offset_of!(Ifreq, ifr_ifru), IFNAMSIZ);
        assert_eq!(core::mem::offset_of!(Ifaliasreq, ifra_dstaddr), 32);
        assert_eq!(core::mem::offset_of!(Ifgroupreq, ifgr_ifgru), 24);
    }

    #[test]
    fn link_state_and_speed_helpers() {
        assert!(link_state_is_up(LINK_STATE_UNKNOWN));
        assert!(link_state_is_up(LINK_STATE_FULL_DUPLEX));
        assert!(!link_state_is_up(LINK_STATE_DOWN));
        let carp_backup = LINK_STATE_DESCRIPTIONS
            .iter()
            .find(|d| link_state_desc_match(d, IFT_CARP, LINK_STATE_DOWN));
        assert_eq!(carp_backup.map(|d| d.ifs_string), Some(&b"backup"[..]));
        let ether_up = LINK_STATE_DESCRIPTIONS
            .iter()
            .find(|d| link_state_desc_match(d, IFT_ETHER, LINK_STATE_UP));
        assert_eq!(ether_up.map(|d| d.ifs_string), Some(&b"active"[..]));
        assert_eq!(if_gbps(1), 1_000_000_000);
        assert_eq!(ifq_prio2tos(IFQ_DEFPRIO), 0x60);
        assert_eq!(ifq_tos2prio(0x60), IFQ_DEFPRIO);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/net/if.h");
        let ifq = assert_defines!(defs; IFQ_NQUEUES, IFQ_MINPRIO, IFQ_MAXPRIO, IFQ_DEFPRIO);
        assert_complete(&defs, "IFQ_", &ifq);
        let link = assert_defines!(defs;
        LINK_STATE_UNKNOWN, LINK_STATE_INVALID, LINK_STATE_DOWN, LINK_STATE_KALIVE_DOWN,
        LINK_STATE_UP, LINK_STATE_HALF_DUPLEX, LINK_STATE_FULL_DUPLEX);
        // LINK_STATE_DESCRIPTIONS spans lines in the C.
        assert_complete(
            &defs,
            "LINK_STATE_",
            &[&link[..], &["LINK_STATE_DESCRIPTIONS"]].concat(),
        );
        let iff = assert_defines!(defs;
        IFF_UP, IFF_BROADCAST, IFF_DEBUG, IFF_LOOPBACK, IFF_POINTOPOINT, IFF_STATICARP,
        IFF_RUNNING, IFF_NOARP, IFF_PROMISC, IFF_ALLMULTI, IFF_OACTIVE, IFF_SIMPLEX, IFF_LINK0,
        IFF_LINK1, IFF_LINK2, IFF_MULTICAST);
        // IFF_CANTCHANGE spans lines in the C.
        assert_eq!(IFF_CANTCHANGE, 0x8e5a);
        assert_complete(&defs, "IFF_", &[&iff[..], &["IFF_CANTCHANGE"]].concat());
        let ifxf = assert_defines!(defs;
        IFXF_MPSAFE, IFXF_CLONED, IFXF_AUTOCONF6TEMP, IFXF_MPLS, IFXF_WOL, IFXF_AUTOCONF6,
        IFXF_INET6_NOSOII, IFXF_AUTOCONF4, IFXF_MONITOR, IFXF_LRO, IFXF_MBUF_64BIT);
        assert_eq!(IFXF_CANTCHANGE, 0x3);
        assert_complete(&defs, "IFXF_", &[&ifxf[..], &["IFXF_CANTCHANGE"]].concat());
        let ifcap = assert_defines!(defs;
        IFCAP_CSUM_IPv4, IFCAP_CSUM_TCPv4, IFCAP_CSUM_UDPv4, IFCAP_VLAN_MTU,
        IFCAP_VLAN_HWTAGGING, IFCAP_VLAN_HWOFFLOAD, IFCAP_CSUM_TCPv6, IFCAP_CSUM_UDPv6,
        IFCAP_TSOv4, IFCAP_TSOv6, IFCAP_LRO, IFCAP_WOL);
        assert_eq!(IFCAP_CSUM_MASK, 0x187);
        assert_complete(
            &defs,
            "IFCAP_",
            &[&ifcap[..], &["IFCAP_CSUM_MASK"]].concat(),
        );
        let ifqctl = assert_defines!(defs;
        IFQCTL_LEN, IFQCTL_MAXLEN, IFQCTL_DROPS, IFQCTL_CONGESTION, IFQCTL_MAXID);
        assert_complete(&defs, "IFQCTL_", &ifqctl);
        let ifan = assert_defines!(defs; IFAN_ARRIVAL, IFAN_DEPARTURE);
        assert_complete(&defs, "IFAN_", &ifan);
        let hdrprio = assert_defines!(defs;
        IF_HDRPRIO_MIN, IF_HDRPRIO_MAX, IF_HDRPRIO_PACKET, IF_HDRPRIO_PAYLOAD,
        IF_HDRPRIO_OUTER, IF_PWE3_ETHERNET, IF_PWE3_IP, IF_NAMESIZE, IF_MAX_VECTORS);
        assert_complete(&defs, "IF_", &hdrprio);
        let sff = assert_defines!(defs; IFSFF_ADDR_EEPROM, IFSFF_ADDR_DDM, IFSFF_DATA_LEN);
        assert_complete(&defs, "IFSFF_", &sff);
        assert_defines!(defs; MCLPOOLS, IFNAMSIZ, IFDESCRSIZE, IFLR_PREFIX);
        assert_eq!(defs["IFG_ALL"], "\"all\"");
        assert_eq!(defs["IFG_EGRESS"], "\"egress\"");
    }

    // net/if.c

    use std::boxed::Box;
    use std::sync::MutexGuard;

    use crate::kern::uipc_mbuf::m_gethdr;
    use crate::sys::mbuf::{M_DONTWAIT, MT_DATA};

    /// Real memory, `mbinit`, a fresh index map (`ifinit`), and no interfaces, groups or pf kifs:
    /// everything from earlier tests lives in their own (leaked) memory, so nothing may be grown
    /// from it.
    pub(crate) fn setup_net() -> MutexGuard<'static, ()> {
        let guard = crate::kern::uipc_mbuf::tests::setup();
        ifinit();
        // A fresh index map, no interfaces or groups from earlier tests (the kernel's), nor pf
        // kifs (pfi_attach_ifgroup).
        kernel::net::if_::if_test_reset();
        crate::net::pf_if::pfi_test_reset();
        // Nor enc(4) interfaces or bpf(4) taps of interfaces that are gone.
        crate::net::if_enc::enc_reset();
        crate::net::bpf::bpf_test_reset();
        // Nor the kernel's free address and membership records, nor IGMP's router records
        // (N6.4a): a later test that detaches the same index would free an earlier test's
        // record (`rti_delete`) into this test's memory.
        kernel::netinet::in_var::in_var_test_reset();
        kernel::netinet::igmp::igmp_test_reset();
        kernel::netinet::if_ether::arp_test_reset();
        guard
    }

    /// An `ioctl` that accepts nothing: what `if_attach` needs to see.
    ///
    /// # Safety
    ///
    /// Never dereferences anything.
    pub(crate) unsafe fn test_ioctl(
        _ifp: &'static Ifnet,
        _cmd: u64,
        _data: *mut u8,
    ) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }

    /// A zero-filled, leaked `T`, as `malloc(M_ZERO)` gives a softc.
    ///
    /// # Safety
    ///
    /// The all-zero bit pattern is a valid `T`.
    pub(crate) unsafe fn zeroed_static<T>() -> &'static T {
        // SAFETY: the caller's contract.
        Box::leak(Box::new(unsafe {
            MaybeUninit::<T>::zeroed().assume_init()
        }))
    }

    /// A zero-filled interface named `name`, with a test `if_ioctl`.
    pub(crate) fn test_ifnet(name: &[u8]) -> &'static Ifnet {
        // SAFETY: the all-zero `Ifnet` is valid (`net/if_var.rs`).
        let ifp: &'static Ifnet = unsafe { zeroed_static() };
        let mut xname = [0u8; IFNAMSIZ];
        xname[..name.len()].copy_from_slice(name);
        ifp.if_xname.set(xname);
        ifp.if_ioctl().set(Some(test_ioctl));
        ifp
    }

    /// A packet header mbuf holding `bytes`.
    pub(crate) fn test_packet(bytes: &[u8]) -> &'static Mbuf {
        let m = m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf");
        assert!(bytes.len() <= crate::sys::mbuf::MHLEN);
        // SAFETY: a fresh packet header mbuf has MHLEN bytes at m_data.
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), mtod::<u8>(m), bytes.len()) };
        m.m_len().set(bytes.len() as u32);
        m.m_pkthdr().len.set(bytes.len() as i32);
        m
    }

    /// The packets [`keep_output`] was given.
    #[cfg(feature = "inet6")]
    static KEPT: std::sync::Mutex<std::vec::Vec<usize>> =
        std::sync::Mutex::new(std::vec::Vec::new());

    /// An `if_output` that keeps the packet (its address in [`KEPT`]) for the test to free.
    ///
    /// # Safety
    ///
    /// `IfOutputFn`'s contract.
    #[cfg(feature = "inet6")]
    unsafe fn keep_output(
        _ifp: &'static Ifnet,
        m: &'static Mbuf,
        _dst: *const Sockaddr,
        _rt: Option<&'static Rtentry>,
    ) -> Result<(), Errno> {
        KEPT.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(ptr::from_ref(m).addr());
        Ok(())
    }

    #[test]
    #[cfg(feature = "inet6")]
    fn output_tso_sends_an_ipv6_packet_with_its_checksum() {
        use crate::netinet::in_::IPPROTO_UDP;
        use crate::netinet6::in6::{In6Addr, SockaddrIn6, sin6tosa_const};
        use crate::netinet6::in6_cksum::in6_cksum;
        use crate::sys::mbuf::M_UDP_CSUM_OUT;

        let _g = setup_net();
        let ifp = test_ifnet(b"ttso6");
        ifp.if_output().set(Some(keep_output));
        // IPv6 from fe80::1 to fe80::2, UDP 1234 -> 53 with four bytes and no checksum yet.
        let mut p = std::vec![0x60, 0, 0, 0, 0, 12, IPPROTO_UDP as u8, 64];
        p.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        p.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        p.extend_from_slice(&[0x04, 0xd2, 0, 53, 0, 12, 0, 0, 1, 2, 3, 4]);
        let m = test_packet(&p);
        m.m_pkthdr().csum_flags.set(M_UDP_CSUM_OUT);
        let dst = SockaddrIn6::with_addr(In6Addr::new([
            0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ]));

        let mut mp = Some(m);
        // SAFETY: a local `sockaddr_in6`; the interface keeps the packet.
        let r = unsafe { if_output_tso(ifp, &mut mp, sin6tosa_const(&dst), None, 1500) };
        assert_eq!(r, Ok(()));
        assert!(mp.is_none(), "sent");
        assert_eq!(
            *KEPT.lock().unwrap_or_else(|e| e.into_inner()),
            [ptr::from_ref(m).addr()]
        );
        // No IFCAP_CSUM_UDPv6: in6_proto_cksum_out computed the checksum in software.
        assert_eq!(m.m_pkthdr().csum_flags.get() & M_UDP_CSUM_OUT, 0);
        assert_eq!(
            in6_cksum(m, IPPROTO_UDP as u8, 40, 12),
            0,
            "a valid checksum"
        );
        m_freem(m);
    }

    #[test]
    #[cfg(feature = "inet6")]
    fn if_up_gives_the_default_loopback_its_ipv6_addresses() {
        use crate::net::if_loop::{LOOP_CLONER, loop_clone_create};
        use crate::net::rtable::rtable_loindex;
        use crate::netinet6::in6::{IN6ADDR_LOOPBACK, in6ifa_ifpforlinklocal, in6ifa_ifpwithaddr};

        let _g = crate::netinet::ip_input::tests::setup();
        // The C holds the kernel lock across an attach (bpf_ifattach asserts it).
        hal::klock::kernel_lock();
        loop_clone_create(&LOOP_CLONER, 0).expect("lo0");
        loop_clone_create(&LOOP_CLONER, 1).expect("lo1");
        hal::klock::kernel_unlock();
        let lo0 = if_get(rtable_loindex(0)).expect("lo0");
        let lo1 = if_unit(b"lo1").expect("lo1");
        assert!(in6ifa_ifpwithaddr(lo0, &IN6ADDR_LOOPBACK).is_none());

        net_lock();
        if_up(lo0);
        if_up(lo1);
        net_unlock();
        assert!(
            in6ifa_ifpwithaddr(lo0, &IN6ADDR_LOOPBACK).is_some(),
            "::1 on lo0"
        );
        assert!(in6ifa_ifpforlinklocal(lo0, 0).is_some(), "fe80::1%lo0");
        // Only the default loopback of the rdomain gets ::1.
        assert!(in6ifa_ifpwithaddr(lo1, &IN6ADDR_LOOPBACK).is_none());
        if_put(lo0);
        if_put(lo1);
    }

    #[test]
    fn the_raw_entry_reaches_the_kernel_s_ifioctl_with_its_socket() {
        use crate::sys::sockio::{SIOCGIFADDR, SIOCGIFFLAGS, SIOCGLIFPHYADDR};
        let (_g, _t, p) = crate::netinet::in_pcb::tests::setup();
        let ifp = test_ifnet(b"traw0");
        ifp.if_flags.set(IFF_UP);
        if_attach(ifp);
        let so = crate::kern::uipc_socket::soalloc(
            &crate::netinet::in_proto::INETSW[1],
            crate::sys::socket::SOCK_DGRAM,
            crate::sys::mbuf::M_WAIT,
        )
        .unwrap();
        let sop = ptr::from_ref(so).cast::<c_void>();

        // A request longer than STK_PARAMS (536 bytes, sys_ioctl's heap buffer) on an
        // interface that is not there.
        let mut laddr = [0u64; 68];
        hal::user::bytes_of_mut(&mut laddr[..])[..8].copy_from_slice(b"nosuch0\0");
        // SAFETY: 544 writable bytes of a local, ours for the call; `so` lives across it.
        let r = unsafe { ifioctl(sop, SIOCGLIFPHYADDR, laddr.as_mut_ptr().cast(), p) };
        assert_eq!(r, Err(Errno::ENXIO));

        // A request of the kernel itself (no socket) on the interface.
        let mut ifr = Ifreq::zeroed();
        ifr.ifr_name[..5].copy_from_slice(b"traw0");
        // SAFETY: a local `ifreq`, SIOCGIFFLAGS's structure, ours for the call.
        let r = unsafe { ifioctl(ptr::null(), SIOCGIFFLAGS, ptr::from_mut(&mut ifr).cast(), p) };
        assert_eq!(r, Ok(()));
        assert_eq!(i32::from(ifr.ifr_flags()) & IFF_UP, IFF_UP);

        // Through the socket's protocol, which gets the interface (`in_control`): it has no
        // address.
        // SAFETY: as above, for SIOCGIFADDR.
        let r = unsafe { ifioctl(sop, SIOCGIFADDR, ptr::from_mut(&mut ifr).cast(), p) };
        assert_eq!(r, Err(Errno::EADDRNOTAVAIL));
        crate::netinet::in_pcb::tests::teardown();
    }

    #[test]
    #[cfg(feature = "inet6")]
    fn the_kernel_s_copies_of_the_inet6_requests_are_in6_var_s() {
        use crate::netinet6::in6_var::{SIOCAIFADDR_IN6, SIOCDIFADDR_IN6};
        assert_eq!(kernel::net::if_::SIOCAIFADDR_IN6, SIOCAIFADDR_IN6);
        assert_eq!(kernel::net::if_::SIOCDIFADDR_IN6, SIOCDIFADDR_IN6);
    }

    #[test]
    fn a_driver_sets_its_send_queue_length_before_attach() {
        // vio, em, vmx, re, dc, fxp and pcn: `ifq_init_maxlen(&ifp->if_snd, n)` before
        // `if_attach`, which makes the queue with it.
        let _g = setup_net();
        let ifp = test_ifnet(b"tml0");
        ifp.if_snd_maxlen.set(7);
        if_attach(ifp);
        assert_eq!(ifp.if_snd().ifq_maxlen.get(), 7);
        let other = test_ifnet(b"tml1");
        if_attach(other);
        assert_eq!(other.if_snd().ifq_maxlen.get(), crate::net::ifq::IFQ_MAXLEN);
    }

    static ALIGNED_SEEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

    /// A handler that checks the request it is given is aligned for its `ifreq`, counts the
    /// calls that were, and answers `SIOCGIFFLAGS` with flags 0x1234.
    ///
    /// # Safety
    ///
    /// `IfIoctlFn`'s contract.
    unsafe fn aligned_ioctl(_ifp: &'static Ifnet, cmd: u64, data: *mut u8) -> Result<(), Errno> {
        if !data.addr().is_multiple_of(8) {
            return Err(Errno::EFAULT);
        }
        ALIGNED_SEEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if cmd == SIOCGIFFLAGS {
            // SAFETY: the caller's contract: an aligned `ifreq`.
            unsafe { (*data.cast::<Ifreq>()).set_ifr_flags(0x1234) };
        }
        Ok(())
    }

    #[test]
    fn a_handler_gets_an_aligned_request_whatever_the_caller_s() {
        // Review 2's B5-r: a request the kernel's `if_ioctl` is given at any address reaches
        // the handler aligned for its structure, and the handler's answer comes back.
        let _g = setup_net();
        let ifp = test_ifnet(b"tal0");
        ifp.if_ioctl().set(Some(aligned_ioctl));
        if_attach(ifp);
        let mut buf = [0u64; 5];
        let bytes = hal::user::bytes_of_mut(&mut buf[..]);
        let before = ALIGNED_SEEN.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(
            kernel::net::if_::if_ioctl(ifp, SIOCGIFFLAGS, &mut bytes[1..33]),
            Ok(())
        );
        let ifr: Ifreq = hal::pod::at(&bytes[1..33], 0).expect("an ifreq");
        assert_eq!(ifr.ifr_flags(), 0x1234);
        // A request longer than the copy must be aligned already: refused at byte 1, taken
        // at byte 0 (SIOCGLIFPHYADDR's `if_laddrreq`, 536 bytes).
        let mut long = [0u64; 68];
        let long = hal::user::bytes_of_mut(&mut long[..]);
        assert_eq!(
            kernel::net::if_::if_ioctl(ifp, SIOCGLIFPHYADDR, &mut long[1..537]),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            kernel::net::if_::if_ioctl(ifp, SIOCGLIFPHYADDR, &mut long[..536]),
            Ok(())
        );
        assert_eq!(
            ALIGNED_SEEN.load(std::sync::atomic::Ordering::Relaxed),
            before + 2
        );
    }

    #[test]
    fn an_inet6_multicast_request_on_ethernet_reads_no_byte_past_it() {
        // Review 2's B5-r (b): `ether_multiaddr` takes the `sockaddr_in6` of SIOCADDMULTI out
        // of a 32-byte `ifreq`, bytes 24..40. Through the kernel's `if_ioctl` the handler reads
        // the zeroed copy past the request: ff02:0:0:0 in the request and zeros after it is
        // the group ff02::, Ethernet 33:33:00:00:00:00. The caller's buffer goes on with 0xaa
        // bytes past the request, which a handler run in place would read (review 3).
        let _g = setup_net();
        let ifp = crate::netinet::ip_input::tests::test_ether();
        let ac = crate::netinet::if_ether::arpcom_of(ifp);
        let mut ifr = Ifreq::zeroed();
        ifr.ifr_name[..5].copy_from_slice(b"tvio0");
        let sa = ifr.ifr_addr_mut();
        sa.sa_len = 28;
        sa.sa_family = AF_INET6;
        // sin6_port (2) and sin6_flowinfo (4), then the first 8 bytes of sin6_addr.
        sa.sa_data[6..8].copy_from_slice(&[0xff, 0x02]);
        let mut buf = [u64::from_ne_bytes([0xaa; 8]); 6];
        let bytes = hal::user::bytes_of_mut(&mut buf[..]);
        bytes[..size_of::<Ifreq>()]
            .copy_from_slice(hal::user::bytes_of(core::slice::from_ref(&ifr)));
        let req = &mut bytes[..size_of::<Ifreq>()];
        let r = kernel::net::if_::if_ioctl(ifp, SIOCADDMULTI, req);
        assert!(matches!(r, Ok(()) | Err(Errno::ENETRESET)), "{r:?}");
        assert_eq!(ac.ac_multicnt.get(), 1);
        // The group's low 32 bits (sin6_addr bytes 12..16, request bytes 36..40) are past the
        // 32-byte request: the entry 33:33:00:00:00:00 proves they were read as the copy's
        // zeros, not as the caller's 0xaa bytes that follow the request.
        {
            let g = ac.multiaddrs();
            let enm = g.iter().next().expect("added");
            assert_eq!(enm.enm_addrlo, [0x33, 0x33, 0, 0, 0, 0]);
            assert_eq!(enm.enm_addrhi, enm.enm_addrlo);
        }
        let r = kernel::net::if_::if_ioctl(ifp, SIOCDELMULTI, req);
        assert!(matches!(r, Ok(()) | Err(Errno::ENETRESET)), "{r:?}");
        assert_eq!(ac.ac_multicnt.get(), 0);
    }

    #[test]
    fn a_short_request_never_reaches_the_driver() {
        let _g = setup_net();
        let ifp = test_ifnet(b"tio0");
        if_attach(ifp);
        // SIOCGIFFLAGS takes a 32-byte struct ifreq: 4 bytes are refused before the driver.
        let mut short = [0u8; 4];
        assert_eq!(
            kernel::net::if_::if_ioctl(ifp, SIOCGIFFLAGS, &mut short),
            Err(Errno::EINVAL)
        );
        let mut ifr = [0u8; 32];
        assert_eq!(
            kernel::net::if_::if_ioctl(ifp, SIOCGIFFLAGS, &mut ifr),
            Err(Errno::ENOTTY)
        );
    }
}
/* </TESTS> */
