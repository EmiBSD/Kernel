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
//! stays `bsd`'s is what names `bsd`'s types until their slice moves them: the interface
//! `ioctl`s (`ifioctl`, `ifconf`, the groups' requests: N6.3a2), the output helpers over
//! `bsd`'s route and socket address (`if_output_ml`, `if_output_tso`, `if_output_mq`:
//! N6.3b), the point-to-point link layer (`p2p_*`: N6.3b/c), the implementation of the
//! kernel's `NetDriver` for `bsd`'s drivers' tables ([`LegacyNetDriver`]) and of the `Inet`,
//! `Routing` and `Link` legs over the subsystems that have not moved.
//!
//! Upstream: sys/net/if.h @ 3ce1f3f79392
//! Upstream: sys/net/if.c @ 3ce1f3f79392
//!
//! `if` is a Rust keyword, so the module is `if_` (`docs/C_TO_RUST.md`).
//!
//! ## Deviations
//! - The kernel's deviations (`kernel::net::if_`) are this module's for what it forwards.
//! - `ifioctl`'s `pru_control` goes through the socket's protocol (`sys/protosw.rs`), as in
//!   C; the kernel's own requests come with a NULL socket and go to `in_ioctl` as privileged
//!   ones (the boot self-test configures an interface that way) (LZ's).
//! - `ifioctl`'s `struct socket *` is a raw pointer (NULL for the kernel's own requests) and
//!   `caddr_t data` a raw pointer: `ifioctl` and the `ioctl` helpers are `unsafe fn`s whose
//!   contract is `sys_ioctl`'s kernel copy of the argument (LZ's, until N6.3a2).
//!   `SIOCSIFXFLAGS`'s `goto forceup` into the `SIOCSIFFLAGS` case is a flag checked after
//!   the `match`.
//! - The functions whose C arguments are socket addresses (`ifa_ifwithaddr`,
//!   `ifa_ifwithdstaddr`, `ifaof_ifpforaddr`, `if_group_routechange`, `ifa_update_broadaddr`)
//!   keep LZ's `unsafe fn` signatures over `*const Sockaddr` for their callers: each views the
//!   address's `sa_len` bytes and forwards (the kernel compares bytes).
//! - Calls into files that are not ported report themselves with `unported!` and go on as
//!   the C would with an empty subsystem: `inet_ntop` for `AF_INET` (`ifa_print_all`;
//!   `AF_INET6` prints with nd6's `In6Ntop`).

use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::mem::{offset_of, size_of};
use core::pin::Pin;
use core::ptr;
use core::slice;
use core::sync::atomic::{AtomicU32, Ordering};

use libkern::strlcpy;

use crate::kern::init_main::PDEVINIT_DONE;
use crate::kern::kern_prot::suser;
use crate::kern::kern_rwlock::{rw_enter_write, rw_exit_write};
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::{Str, panic, printf};
use crate::kern::uipc_mbuf::{m_freem, ml_dequeue, ml_purge, mq_delist, mq_purge};
use crate::machine::copy::{copyin, copyinstr, copyout, copyoutstr};
use crate::machine::intr::{splnet, splx};
use crate::net::bpf::{bpf_mtap_af, bpf_mtap_ether, bpfdetach};
use crate::net::if_ethersubr::ether_brport_isset;
use crate::net::if_var::{IfnetHead, IfnetLegacy, LegacyNetDriver, Niqueue};
use crate::net::route::{
    RTF_BLACKHOLE, RTF_LLINFO, RTF_LOCAL, RTF_REJECT, RTLABEL_LEN, RTM_ADD, RTP_ANY, Rtentry,
    ifafree, ifaref, rt_if_track, rtlabel_id2name, rtlabel_name2id, rtlabel_unref,
};
use crate::net::rtable::{
    rtable_add, rtable_empty, rtable_exists, rtable_iterate, rtable_l2, rtable_l2set,
    rtable_loindex, rtable_lookup,
};
use crate::net::rtsock::{rtm_ifannounce, rtm_ifchg};
use crate::netinet::if_ether::{
    ETHER_ADDR_LEN, arpcom_of, arpinput, arpintr, ether_is_multicast, revarpinput,
};
use crate::netinet::igmp::rti_delete;
use crate::netinet::in_::{SockaddrIn, in_ifdetach, in_ioctl, sintosa};
use crate::netinet::ip_input::{ipintr, ipv4_input};
use crate::netinet::ip_output::{in_hdr_cksum_out, in_proto_cksum_out};
use crate::netinet::tcp_input::tcp_input_mlist;
use crate::netinet::tcp_output::tcp_if_output_tso;
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};
#[cfg(feature = "inet6")]
use crate::netinet6::in6::{in6_purgeaddr, in6ifa_ifpforlinklocal, satosin6_const, sin6tosa};
#[cfg(feature = "inet6")]
use crate::netinet6::in6_ifattach::{in6_ifattach, in6_ifdetach};
#[cfg(feature = "inet6")]
use crate::netinet6::in6_proto::IP6_FORWARDING;
#[cfg(feature = "inet6")]
use crate::netinet6::in6_var::{SIOCAIFADDR_IN6, SIOCDIFADDR_IN6};
#[cfg(feature = "inet6")]
use crate::netinet6::ip6_input::{ip6intr, ipv6_input};
#[cfg(feature = "inet6")]
use crate::netinet6::ip6_output::in6_proto_cksum_out;
#[cfg(feature = "inet6")]
use crate::netinet6::nd6::{In6Ntop, nd6_ifattach, nd6_ifdetach};
use crate::sys::errno::Errno;
use crate::sys::ioccom::iocparm_len;
use crate::sys::mbuf::{Mbuf, MbufList, MbufQueue, ml_len, mq_len, park, unpark};
use crate::sys::proc::Proc;
use crate::sys::socket::{AF_INET, AF_INET6, Sockaddr};
use crate::sys::socketvar::Socket;
use crate::sys::sockio::*;
use crate::sys::systm::{
    kernel_lock, kernel_unlock, net_lock, net_lock_shared, net_unlock, net_unlock_shared,
};
use crate::sys::task::Task;
use crate::sys::types::SaFamily;
use crate::{kassert, unported};

pub use hal::sys::net_if::*;
pub use kernel::net::if_::{
    ETHER_ADDR_LEN as IF_ETHER_ADDR_LEN, IFQ_CONGESTION, NET_TASKQ, Softnet, if_addgroup,
    if_alloc_sadl, if_attach_iqueues, if_attach_queues, if_clone_create, if_clone_destroy,
    if_congested, if_congestion, if_counters_alloc, if_counters_free, if_creategroup,
    if_createrdomain, if_deactivate, if_delgroup, if_detach, if_down, if_downall, if_free_sadl,
    if_get, if_getdata, if_group_egress_build, if_isconnected, if_link_state_change, if_linkstate,
    if_ref, if_rxr_get, if_rxr_init, if_rxr_livelocked, if_setlladdr, if_setrdomain, if_unit,
    if_up, ifa_add, ifa_del, ifg_head, ifnewlladdr, ifpromisc, ifsetlro, net_sn, net_tq,
    net_tq_barriers, softnet_count, softnet_init, softnet_percpu, unhandled_af,
};
pub use kernel::net::if_var::{counters_inc, counters_pkt};

/// `ifnetlist`: all attached interfaces (the kernel's list), in LZ's tuple shape.
pub static IFNETLIST: IfnetHead = IfnetHead(&kernel::net::if_::IFNETLIST);

/// `if_tmplist_lock`: serialises the users of `if_tmplist` (`ifconf`,
/// `if_getgroupmembers`: `bsd`'s until N6.3a2).
pub static IF_TMPLIST_LOCK: crate::sys::rwlock::Rwlock = crate::sys::rwlock::Rwlock::new("iftmplk");

/// The bytes of a NUL-terminated name up to its NUL.
fn cstr(s: &[u8]) -> &[u8] {
    let n = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    &s[..n]
}

/// `strcmp(a, b) == 0` of two NUL-terminated names.
fn name_eq(a: &[u8], b: &[u8]) -> bool {
    cstr(a) == cstr(b)
}

/// The bytes of a `#[repr(C)]` value (the requests this module copies out whole).
///
/// # Safety
///
/// `T` has no padding.
unsafe fn bytes_of<T>(v: &T) -> &[u8] {
    // SAFETY: the caller's contract; the slice covers `v` alone, for `v`'s lifetime.
    unsafe { slice::from_raw_parts(ptr::from_ref(v).cast::<u8>(), size_of::<T>()) }
}

/// `(*ifp->if_ioctl)(ifp, cmd, data)`: the driver's request handler (`ENODEV` once the
/// interface is detached, the C's `if_detached_ioctl`).
///
/// # Safety
///
/// `data` points at the request's structure for `cmd` (`IOCPARM_LEN(cmd)` bytes, readable
/// and writable), as [`IfIoctlFn`](crate::net::if_var::IfIoctlFn)'s.
pub unsafe fn ifp_ioctl(ifp: &'static Ifnet, cmd: u64, data: *mut u8) -> Result<(), Errno> {
    if ifp.if_ioctl_detached() {
        return Err(Errno::ENODEV);
    }
    match ifp.if_ioctl().get() {
        // SAFETY: the caller's contract is the handler's.
        Some(ioctl) => unsafe { ioctl(ifp, cmd, data) },
        None => Err(Errno::ENOTTY),
    }
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
        match self.if_ioctl.get() {
            // SAFETY: `data` is the request's image of the command (the kernel's callers lend
            // an `Ifreq`, `ifioctl` the whole of `sys_ioctl`'s copy, `IOCPARM_LEN(cmd)` bytes
            // or more), readable and writable for the call: the handler's contract.
            Some(ioctl) => unsafe { ioctl(ifp, cmd, data.as_mut_ptr()) },
            None => Err(Errno::ENOTTY),
        }
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
            None => {
                drop(m);
                panic(format_args!("{}: no if_input", Str(&ifp.if_xname.get())));
            }
        }
    }

    fn bpf_mtap(&self, if_bpf: *const u8, m: &Mbuf, dir: u32) -> bool {
        match self.if_bpf_mtap.get() {
            Some(mtap) => mtap(if_bpf.cast_mut(), m, dir),
            None => bpf_mtap_ether(if_bpf.cast_mut(), m, dir),
        }
    }

    fn as_any(&self) -> Option<&(dyn core::any::Any + Send + Sync)> {
        Some(self)
    }
}

/// The legs of the subsystems `if.c` calls that are still `bsd`'s: the internet protocols
/// (`Inet`), routing (`Routing`) and the link layers (`Link`), registered by [`ifinit`].
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

/// `arpinput` as the kernel's `if_input_proto` takes it.
pub fn arpinput_proto(
    ifp: &'static Ifnet,
    m: kernel::net::mbuf::MbufBox,
    ns: Option<&mut Netstack>,
) {
    arpinput(ifp, park(m), ns);
}

/// `revarpinput` as the kernel's `if_input_proto` takes it.
pub fn revarpinput_proto(
    ifp: &'static Ifnet,
    m: kernel::net::mbuf::MbufBox,
    ns: Option<&mut Netstack>,
) {
    revarpinput(ifp, park(m), ns);
}

impl kernel::net::if_::Inet for BsdNetLegs {
    fn arpintr(&self) {
        arpintr();
    }

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

    fn rti_delete(&self, ifp: &'static Ifnet) {
        rti_delete(ifp);
    }

    fn in_ifdetach(&self, ifp: &'static Ifnet) {
        // NETHER > 0 && NFSCLIENT: revarp_ifidx is cleared.
        #[cfg(feature = "nfsclient")]
        if ifp.if_index.get() == crate::netinet::if_ether::REVARP_IFIDX.load(Ordering::Relaxed) {
            crate::netinet::if_ether::REVARP_IFIDX.store(0, Ordering::Relaxed);
        }
        in_ifdetach(ifp);
    }

    fn in6_ifdetach(&self, ifp: &'static Ifnet) {
        #[cfg(feature = "inet6")]
        in6_ifdetach(ifp);
        let _ = ifp;
    }

    fn in6_ifattach(&self, ifp: &'static Ifnet) {
        // The C ignores the result.
        #[cfg(feature = "inet6")]
        let _ = in6_ifattach(ifp);
        let _ = ifp;
    }

    fn ip6_forwarding(&self) -> bool {
        #[cfg(feature = "inet6")]
        return IP6_FORWARDING.load(Ordering::Relaxed) != 0;
        #[cfg(not(feature = "inet6"))]
        false
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

impl kernel::net::if_::Routing for BsdNetLegs {
    fn rtm_ifannounce(&self, ifp: &'static Ifnet, what: u16) {
        rtm_ifannounce(ifp, what);
    }

    fn rtm_ifchg(&self, ifp: &'static Ifnet) {
        rtm_ifchg(ifp);
    }

    fn rt_if_track(&self, ifp: &'static Ifnet) {
        // The C ignores rt_if_track's result in if_linkstate.
        let _ = rt_if_track(ifp);
    }

    fn rtable_l2(&self, rtableid: u32) -> u32 {
        rtable_l2(rtableid)
    }

    fn rtable_loindex(&self, rdomain: u32) -> u32 {
        rtable_loindex(rdomain)
    }

    fn rtable_exists(&self, rtableid: u32) -> bool {
        rtable_exists(rtableid)
    }

    fn rtable_add(&self, id: u32) -> Result<(), Errno> {
        rtable_add(id)
    }

    fn rtable_empty(&self, id: u32) -> bool {
        rtable_empty(id)
    }

    fn rtable_l2set(&self, rtableid: u32, rdomain: u32, loifidx: u32) {
        rtable_l2set(rtableid, rdomain, loifidx);
    }

    fn ifafree(&self, ifa: &'static Ifaddr) {
        ifafree(ifa);
    }

    fn default_routes(&self, af: SaFamily, f: &mut dyn FnMut(u32)) {
        let mut walk = |dst: *mut Sockaddr| {
            // SAFETY: `dst` is a local socket address of the family, the destination and the
            // mask (`0.0.0.0/0`, `::/0`).
            let mut rt = unsafe { rtable_lookup(0, dst, dst, ptr::null(), RTP_ANY) };
            while let Some(r) = rt {
                if r.rt_flags.get() & (RTF_REJECT | RTF_BLACKHOLE) == 0 {
                    f(r.rt_ifidx.get());
                }
                rt = rtable_iterate(r);
            }
        };
        match af {
            AF_INET => {
                let mut sa_in = SockaddrIn {
                    sin_len: size_of::<SockaddrIn>() as u8,
                    sin_family: AF_INET,
                    ..SockaddrIn::default()
                };
                walk(sintosa(&mut sa_in));
            }
            #[cfg(feature = "inet6")]
            AF_INET6 => {
                let mut sa_in6 = crate::netinet6::in6::SA6_ANY;
                walk(sin6tosa(&raw mut sa_in6));
            }
            _ => {}
        }
    }
}

impl kernel::net::if_::Link for BsdNetLegs {
    fn bpf_mtap_ether(&self, if_bpf: *const u8, m: &Mbuf, dir: u32) -> bool {
        bpf_mtap_ether(if_bpf.cast_mut(), m, dir)
    }

    fn bpf_queue_tap(&self, bpfp: &hal::link::PtrWord<*mut u8>) -> *const u8 {
        let p = bpfp.get();
        if p.is_null() {
            return ptr::null();
        }
        // SAFETY: a queue's `ifiq_bpfp` is NULL or points at its driver's per-queue tap word
        // (vmx's ring's `bpf`, a `PtrWord<u8>`, set before the ring runs), which lives as long
        // as the queue (LZ's `ifiq_input` read, moved here with the word); read atomically, as
        // bpf(4) writes it.
        unsafe { (*p.cast::<hal::link::PtrWord<u8>>()).get_mut() }
    }

    fn bpf_mtap_af(&self, if_bpf: *const u8, af: u32, m: &Mbuf, dir: u32) -> bool {
        bpf_mtap_af(if_bpf.cast_mut(), af, m, dir)
    }

    fn bpfdetach(&self, ifp: &'static Ifnet) {
        bpfdetach(ifp);
    }

    fn ether_brport_isset(&self, ifp: &'static Ifnet) -> bool {
        ether_brport_isset(ifp).is_err()
    }

    fn arpcom_set_enaddr(&self, ifp: &'static Ifnet, lladdr: &[u8; IF_ETHER_ADDR_LEN]) {
        arpcom_of(ifp).ac_enaddr.set(*lladdr);
    }
}

/// `ifinit`: the kernel's interface layer made (its index map) and the legs of `bsd`'s
/// subsystems registered (a second call, a test's, keeps the first).
pub fn ifinit() {
    let _ = kernel::net::if_::register_inet(&BSD_NET_LEGS);
    let _ = kernel::net::if_::register_routing(&BSD_NET_LEGS);
    let _ = kernel::net::if_::register_link(&BSD_NET_LEGS);
    kernel::net::if_::ifinit();
}

/// `if_attach_common`'s defaults for `bsd`'s own table (`if_rtrequest_dummy`) and its check
/// of `if_start`/`if_qstart` against `IFXF_MPSAFE`, before the kernel attaches.
fn if_attach_legacy(ifp: &'static Ifnet) {
    if ifp.if_rtrequest().get().is_none() {
        ifp.if_rtrequest().set(Some(if_rtrequest_dummy));
    }
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

/// `&ifr->ifr_addr` as the C's pointer into the caller's request: a protocol's request may
/// carry a socket address longer than the union (`struct in6_ifreq`'s `sockaddr_in6`), which
/// the callee reads through it (never dereferenced here).
pub fn ifr_addr_ptr(ifr: &Ifreq) -> *const Sockaddr {
    ptr::from_ref(ifr)
        .cast::<u8>()
        .wrapping_add(offset_of!(Ifreq, ifr_ifru))
        .cast::<Sockaddr>()
}

/// A socket address's `sa_len` bytes.
///
/// # Safety
///
/// `sa` points at a readable socket address of its `sa_len` bytes, which live for `'a`.
unsafe fn sa_bytes<'a>(sa: *const Sockaddr) -> &'a [u8] {
    // SAFETY: the caller's contract.
    unsafe { slice::from_raw_parts(sa.cast::<u8>(), usize::from((*sa).sa_len)) }
}

/// `ifa_ifwithaddr(addr, rtableid)`: locate an interface based on a complete address.
///
/// # Safety
///
/// `addr` points at a readable socket address of `sa_len` bytes.
pub unsafe fn ifa_ifwithaddr(addr: *const Sockaddr, rtableid: u32) -> Option<&'static Ifaddr> {
    // SAFETY: the caller's contract.
    kernel::net::if_::ifa_ifwithaddr(unsafe { sa_bytes(addr) }, rtableid)
}

/// `ifa_ifwithdstaddr(addr, rdomain)`: the point-to-point interface with a given
/// destination address.
///
/// # Safety
///
/// As for [`ifa_ifwithaddr`].
pub unsafe fn ifa_ifwithdstaddr(addr: *const Sockaddr, rdomain: u32) -> Option<&'static Ifaddr> {
    // SAFETY: the caller's contract.
    kernel::net::if_::ifa_ifwithdstaddr(unsafe { sa_bytes(addr) }, rdomain)
}

/// `ifaof_ifpforaddr(addr, ifp)`: the address of `ifp` that best matches `addr`.
///
/// # Safety
///
/// As for [`ifa_ifwithaddr`].
pub unsafe fn ifaof_ifpforaddr(addr: *const Sockaddr, ifp: &Ifnet) -> Option<&'static Ifaddr> {
    // SAFETY: the caller's contract.
    kernel::net::if_::ifaof_ifpforaddr(unsafe { sa_bytes(addr) }, ifp)
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

/// `(*ifp->if_output)(ifp, m, dst, rt)`.
///
/// # Safety
///
/// As for [`IfOutputFn`].
unsafe fn ifp_output(
    ifp: &'static Ifnet,
    m: &'static Mbuf,
    dst: *const Sockaddr,
    rt: Option<&'static Rtentry>,
) -> Result<(), Errno> {
    match ifp.if_output().get() {
        // SAFETY: the caller's contract is the hook's.
        Some(output) => unsafe { output(ifp, m, dst, rt) },
        None => panic(format_args!("{}: no if_output", Str(&ifp.if_xname.get()))),
    }
}

/// `if_output_tso`: sends a TCP packet with TSO, or chops it, or sends it whole when it fits
/// `mtu`; `*mp` is left set when the packet still has to be fragmented or dropped.
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
    // SAFETY: `dst` is readable per the contract.
    let family = unsafe { (*dst).sa_family };
    let ifcap: u32 = match family {
        AF_INET => IFCAP_TSOv4,
        #[cfg(feature = "inet6")]
        AF_INET6 => IFCAP_TSOv6,
        _ => unhandled_af(i32::from(family)),
    };

    // Try to send with TSO first. When forwarding LRO may set maximum segment size in mbuf
    // header. Chop TCP segment even if it would fit interface MTU to preserve maximum path
    // MTU.
    // SAFETY: the caller's contract is `if_output`'s, which `tcp_if_output_tso` passes on.
    unsafe { tcp_if_output_tso(ifp, mp, dst, rt, ifcap, mtu) }?;
    let Some(m) = *mp else {
        return Ok(());
    };

    if m.m_pkthdr().len.get() as u32 <= mtu {
        match family {
            AF_INET => {
                in_hdr_cksum_out(m, Some(ifp));
                in_proto_cksum_out(m, Some(ifp));
            }
            #[cfg(feature = "inet6")]
            AF_INET6 => in6_proto_cksum_out(m, Some(ifp)),
            _ => {}
        }
        // SAFETY: the caller's contract is `if_output`'s.
        let error = unsafe { ifp_output(ifp, m, dst, rt) };
        *mp = None;
        return error;
    }

    // mp still contains mbuf that has to be fragmented or dropped.
    Ok(())
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
/// `if_clone_list`: provide list of interface cloners to userspace.
pub fn if_clone_list(ifcr: &mut IfClonereq) -> Result<(), Errno> {
    let count_all = kernel::net::if_::if_clone_count();

    let mut dst = ifcr.ifcr_buffer as usize;
    if dst == 0 {
        // Just asking how many there are.
        ifcr.ifcr_total = count_all;
        return Ok(());
    }

    if ifcr.ifcr_count < 0 {
        return Err(Errno::EINVAL);
    }

    ifcr.ifcr_total = count_all;
    let mut count = count_all.min(ifcr.ifcr_count);

    let mut error = Ok(());
    for ifc in kernel::net::if_::if_cloners() {
        if count == 0 {
            break;
        }
        let mut outbuf = [0u8; IFNAMSIZ];
        let _ = strlcpy(&mut outbuf, ifc.name());
        error = copyout(&outbuf, dst);
        if error.is_err() {
            break;
        }
        count -= 1;
        dst += IFNAMSIZ;
    }

    error
}

// net/if.c, kept (LZ's shape until its slice).
/// `if_rtrequest_dummy`: the default `if_rtrequest`.
pub fn if_rtrequest_dummy(_ifp: &'static Ifnet, _req: i32, _rt: Option<&'static Rtentry>) {}

/// `p2p_rtrequest`: default action when installing a local route on a point-to-point
/// interface.
pub fn p2p_rtrequest(ifp: &'static Ifnet, req: i32, rt: Option<&'static Rtentry>) {
    let Some(rt) = rt else {
        return;
    };
    if i32::from(RTM_ADD) != req {
        // RTM_DELETE, RTM_RESOLVE, default: nothing.
        return;
    }
    if rt.rt_flags.get() & RTF_LOCAL == 0 {
        return;
    }

    let key = crate::net::rtable::rt_key(rt);
    let Some(ifa) = ifp.if_addrlist.iter().find(|ifa| {
        // SAFETY: a route's key and an interface's addresses are valid sockaddrs.
        unsafe {
            let len = usize::from((*key).sa_len);
            slice::from_raw_parts(key.cast::<u8>(), len)
                == slice::from_raw_parts(ifa.ifa_addr.get().cast::<u8>(), len)
        }
    }) else {
        return;
    };

    kassert!(rt.rt_ifa.get().is_some_and(|rifa| ptr::eq(rifa, ifa)));

    let lo0ifp = if_get(rtable_loindex(ifp.if_rdomain.get()));
    kassert!(lo0ifp.is_some());
    let family = {
        // SAFETY: as above.
        unsafe { (*ifa.ifa_addr.get()).sa_family }
    };
    let lo0ifa = lo0ifp.and_then(|lo0ifp| {
        lo0ifp
            .if_addrlist
            .iter()
            // SAFETY: as above.
            .find(|lo0ifa| unsafe { (*lo0ifa.ifa_addr.get()).sa_family } == family)
            .map(ptr::from_ref)
    });
    if_put(lo0ifp);

    if lo0ifa.is_none() {
        return;
    }

    rt.rt_flags.set(rt.rt_flags.get() & !RTF_LLINFO);
}

/// `p2p_bpf_mtap`: a point-to-point interface's tap, by the packet's address family.
pub fn p2p_bpf_mtap(if_bpf: *mut u8, m: &Mbuf, dir: u32) -> bool {
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

// net/if.c, kept (LZ's shape until its slice).
/// `pru_control(so, cmd, data, ifp)` (`<sys/protosw.h>`) of the socket of the request: the
/// protocol's `ioctl`. A request of the kernel itself (no socket) goes to `in_ioctl` as a
/// privileged one (see the module's deviations).
///
/// # Safety
///
/// As for [`ifioctl`].
unsafe fn pru_control(
    so: *const c_void,
    cmd: u64,
    data: *mut u8,
    ifp: &'static Ifnet,
) -> Result<(), Errno> {
    if so.is_null() {
        // SAFETY: the caller's contract.
        return unsafe { in_ioctl(cmd, data, Some(ifp), true) };
    }
    // SAFETY: a non-NULL `so` is the live socket of the request, which the caller's file
    // reference keeps for the call (the `fp_socket` idiom).
    let so: &'static Socket = unsafe { &*so.cast::<Socket>() };
    // SAFETY: `data` is the kernel copy of the request, as long as `cmd` encodes (the
    // caller's contract), exclusively ours for the call.
    let data = unsafe { slice::from_raw_parts_mut(data, iocparm_len(cmd) as usize) };
    // The bsd table's pru_control, which takes the interface (the kernel's Usrreqs::control
    // does not until N6.3a); a socket of the kernel's UNIX domain has none, as the C's.
    match crate::kern::uipc_domain::so_protosw(so)
        .and_then(|pr| pr.pr_usrreqs)
        .and_then(|u| u.pru_control)
    {
        Some(control) => control(so, cmd, data, Some(ifp)),
        None => Err(Errno::EOPNOTSUPP),
    }
}

/// `ifioctl`: interface ioctls (`SIOC*` on a socket).
///
/// # Safety
///
/// `data` points at the kernel copy of the request, aligned for and as long as the structure
/// `cmd` encodes; `so` is the live `struct socket` of the request, or NULL.
pub unsafe fn ifioctl(so: *const c_void, cmd: u64, data: *mut u8, p: &Proc) -> Result<(), Errno> {
    // SAFETY: every interface command's argument starts with the interface name (`ifreq`,
    // `ifgroupreq`, `if_afreq`), per the caller's contract.
    let ifr = unsafe { &mut *data.cast::<Ifreq>() };

    match cmd {
        SIOCIFCREATE => {
            suser(p)?;
            kernel_lock();
            let error = if_clone_create(&ifr.ifr_name, 0);
            kernel_unlock();
            return error;
        }
        SIOCIFDESTROY => {
            suser(p)?;
            kernel_lock();
            let error = if_clone_destroy(&ifr.ifr_name);
            kernel_unlock();
            return error;
        }
        SIOCSIFGATTR => {
            suser(p)?;
            kernel_lock();
            net_lock();
            // SAFETY: the caller's contract: an `ifgroupreq`.
            let error = unsafe { if_setgroupattribs(data) };
            net_unlock();
            kernel_unlock();
            return error;
        }
        SIOCGIFCONF | SIOCIFGCLONERS | SIOCGIFGMEMB | SIOCGIFGATTR | SIOCGIFGLIST
        | SIOCGIFFLAGS | SIOCGIFXFLAGS | SIOCGIFMETRIC | SIOCGIFMTU | SIOCGIFHARDMTU
        | SIOCGIFDATA | SIOCGIFDESCR | SIOCGIFRTLABEL | SIOCGIFPRIORITY | SIOCGIFRDOMAIN
        | SIOCGIFGROUP | SIOCGIFLLPRIO => {
            // SAFETY: the caller's contract.
            return unsafe { ifioctl_get(cmd, data) };
        }
        _ => {}
    }

    kernel_lock();

    let Some(ifp) = if_unit(&ifr.ifr_name) else {
        kernel_unlock();
        return Err(Errno::ENXIO);
    };
    let oif_flags = ifp.if_flags.get();
    let oif_xflags = ifp.if_xflags.get();

    // The C's `goto forceup` from SIOCSIFXFLAGS into SIOCSIFFLAGS.
    let mut forceup = false;
    let mut error: Result<(), Errno> = Ok(());

    match cmd {
        SIOCIFAFATTACH | SIOCIFAFDETACH => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            // SAFETY: the caller's contract: an `if_afreq`.
            let ifar = unsafe { &*data.cast::<IfAfreq>() };
            net_lock();
            match ifar.ifar_af {
                AF_INET => {
                    // attach is a noop for AF_INET
                    if cmd == SIOCIFAFDETACH {
                        in_ifdetach(ifp);
                    }
                }
                #[cfg(feature = "inet6")]
                AF_INET6 => {
                    if cmd == SIOCIFAFATTACH {
                        error = in6_ifattach(ifp);
                    } else {
                        in6_ifdetach(ifp);
                    }
                }
                _ => error = Err(Errno::EAFNOSUPPORT),
            }
            net_unlock();
        }

        SIOCSIFXFLAGS => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }

            net_lock();
            #[cfg(feature = "inet6")]
            {
                let rflags = i32::from(ifr.ifr_flags());
                let xflags = ifp.if_xflags.get();
                if rflags & (IFXF_AUTOCONF6 | IFXF_AUTOCONF6TEMP) != 0
                    && xflags & (IFXF_AUTOCONF6 | IFXF_AUTOCONF6TEMP) == 0
                    && let Err(e) = in6_ifattach(ifp)
                {
                    error = Err(e);
                    net_unlock();
                    break 'out;
                }

                if rflags & IFXF_INET6_NOSOII != 0 && ifp.if_xflags.get() & IFXF_INET6_NOSOII == 0 {
                    ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_INET6_NOSOII);
                }

                if rflags & IFXF_INET6_NOSOII == 0 && ifp.if_xflags.get() & IFXF_INET6_NOSOII != 0 {
                    ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_INET6_NOSOII);
                }
            }

            // MPLS: IFXF_MPLS swaps if_output with mpls_output; MPLS is not configured.

            // !SMALL_KERNEL
            let rflags = i32::from(ifr.ifr_flags());
            if ifp.if_capabilities.get() & IFCAP_WOL != 0 {
                if rflags & IFXF_WOL != 0 && ifp.if_xflags.get() & IFXF_WOL == 0 {
                    let s = splnet();
                    ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_WOL);
                    error = match ifp.if_wol().get() {
                        Some(wol) => wol(ifp, true),
                        None => Err(Errno::ENOTSUP),
                    };
                    splx(s);
                }
                if ifp.if_xflags.get() & IFXF_WOL != 0 && rflags & IFXF_WOL == 0 {
                    let s = splnet();
                    ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_WOL);
                    error = match ifp.if_wol().get() {
                        Some(wol) => wol(ifp, false),
                        None => Err(Errno::ENOTSUP),
                    };
                    splx(s);
                }
            } else if rflags & IFXF_WOL != 0 {
                ifr.set_ifr_flags((rflags & !IFXF_WOL) as i16);
                error = Err(Errno::ENOTSUP);
            }
            let rflags = i32::from(ifr.ifr_flags());
            if (rflags & IFXF_LRO != 0) != (ifp.if_xflags.get() & IFXF_LRO != 0) {
                error = ifsetlro(ifp, rflags & IFXF_LRO != 0);
            }

            if error.is_ok() {
                ifp.if_xflags
                    .set((ifp.if_xflags.get() & IFXF_CANTCHANGE) | (rflags & !IFXF_CANTCHANGE));
            }

            let xflags = ifp.if_xflags.get();
            if ifp.if_flags.get() & IFF_UP == 0
                && ((oif_xflags & IFXF_AUTOCONF4 == 0 && xflags & IFXF_AUTOCONF4 != 0)
                    || (oif_xflags & IFXF_AUTOCONF6 == 0 && xflags & IFXF_AUTOCONF6 != 0)
                    || (oif_xflags & IFXF_AUTOCONF6TEMP == 0 && xflags & IFXF_AUTOCONF6TEMP != 0))
            {
                ifr.set_ifr_flags((ifp.if_flags.get() | IFF_UP) as i16);
                forceup = true;
                break 'out;
            }

            net_unlock();
        }

        SIOCSIFFLAGS => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }

            net_lock();
            forceup = true;
        }

        SIOCSIFMETRIC => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            net_lock();
            ifp.if_metric.set(ifr.ifr_metric() as u32);
            net_unlock();
        }

        SIOCSIFMTU => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            net_lock();
            // SAFETY: the caller's contract is the hook's.
            error = unsafe { ifp_ioctl(ifp, cmd, data) };
            net_unlock();
            if error.is_ok() {
                rtm_ifchg(ifp);
            }
        }

        SIOCSIFDESCR => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            let mut ifdescrbuf = [0u8; IFDESCRSIZE];
            error = copyinstr(ifr.ifr_data() as usize, &mut ifdescrbuf).map(|_| ());
            if error.is_ok() {
                let mut descr = [0u8; IFDESCRSIZE];
                let _ = strlcpy(&mut descr, &ifdescrbuf);
                ifp.if_description.set(descr);
            }
        }

        SIOCSIFRTLABEL => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            let mut ifrtlabelbuf = [0u8; RTLABEL_LEN];
            error = copyinstr(ifr.ifr_data() as usize, &mut ifrtlabelbuf).map(|_| ());
            if error.is_ok() {
                rtlabel_unref(ifp.if_rtlabelid.get());
                ifp.if_rtlabelid.set(rtlabel_name2id(&ifrtlabelbuf));
            }
        }

        SIOCSIFPRIORITY => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            let metric = ifr.ifr_metric();
            if !(0..=15).contains(&metric) {
                error = Err(Errno::EINVAL);
                break 'out;
            }
            ifp.if_priority.set(metric as u8);
        }

        SIOCSIFRDOMAIN => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            error = if_createrdomain(ifr.ifr_rdomainid(), ifp);
            if matches!(error, Ok(()) | Err(Errno::EEXIST)) {
                net_lock();
                error = if_setrdomain(ifp, ifr.ifr_rdomainid());
                net_unlock();
            }
        }

        SIOCAIFGROUP => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            // SAFETY: the caller's contract: an `ifgroupreq`.
            let ifgr = unsafe { &*data.cast::<Ifgroupreq>() };
            net_lock();
            error = if_addgroup(ifp, ifgr.ifgr_group());
            if error.is_ok() {
                // SAFETY: the caller's contract is the hook's.
                error = unsafe { ifp_ioctl(ifp, cmd, data) };
                if error == Err(Errno::ENOTTY) {
                    error = Ok(());
                }
            }
            net_unlock();
        }

        SIOCDIFGROUP => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            // SAFETY: the caller's contract: an `ifgroupreq`.
            let ifgr = unsafe { &*data.cast::<Ifgroupreq>() };
            net_lock();
            // SAFETY: the caller's contract is the hook's.
            error = unsafe { ifp_ioctl(ifp, cmd, data) };
            if error == Err(Errno::ENOTTY) {
                error = Ok(());
            }
            if error.is_ok() {
                error = if_delgroup(ifp, ifgr.ifgr_group());
            }
            net_unlock();
        }

        SIOCSIFLLADDR => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            let sa = ifr.ifr_addr();
            let mut lladdr = [0u8; ETHER_ADDR_LEN];
            lladdr.copy_from_slice(&sa.sa_data[..ETHER_ADDR_LEN]);
            if ifp.sadl().is_none()
                || usize::from(sa.sa_len) != ETHER_ADDR_LEN
                || ether_is_multicast(&lladdr)
            {
                error = Err(Errno::EINVAL);
                break 'out;
            }
            net_lock();
            match ifp.if_type.get() {
                IFT_ETHER | IFT_CARP | IFT_XETHER | IFT_ISO88025 => {
                    // SAFETY: the caller's contract is the hook's.
                    error = unsafe { ifp_ioctl(ifp, cmd, data) };
                    if error == Err(Errno::ENOTTY) {
                        error = Ok(());
                    }
                    if error.is_ok() {
                        error = if_setlladdr(ifp, &lladdr);
                    }
                }
                _ => error = Err(Errno::ENODEV),
            }

            if error.is_ok() {
                ifnewlladdr(ifp);
            }
            net_unlock();
            if error.is_ok() {
                rtm_ifchg(ifp);
            }
        }

        SIOCSIFLLPRIO => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }
            let llprio = ifr.ifr_llprio();
            if llprio < IFQ_MINPRIO as i32 || llprio > IFQ_MAXPRIO as i32 {
                error = Err(Errno::EINVAL);
                break 'out;
            }
            net_lock();
            ifp.if_llprio.set(llprio as u8);
            net_unlock();
        }

        SIOCGIFSFFPAGE => 'out: {
            if let Err(e) = suser(p) {
                error = Err(e);
                break 'out;
            }

            // SAFETY: the caller's contract: an `if_sffpage`.
            error = unsafe { if_sffpage_check(data) };
            if error.is_err() {
                break 'out;
            }

            // don't take NET_LOCK because i2c reads take a long time
            // SAFETY: the caller's contract is the hook's.
            error = unsafe { ifp_ioctl(ifp, cmd, data) };
        }

        SIOCSIFMEDIA | SIOCGIFMEDIA => 'out: {
            if cmd == SIOCSIFMEDIA
                && let Err(e) = suser(p)
            {
                error = Err(e);
                break 'out;
            }
            // net lock is not needed
            // SAFETY: the caller's contract is the hook's.
            error = unsafe { ifp_ioctl(ifp, cmd, data) };
        }

        _ => 'out: {
            // These need the superuser before they go to the protocol and the driver
            // (NBRIDGE > 0 adds the SIOCBRDG* commands; bridge(4) is not configured).
            if matches!(
                cmd,
                SIOCSETKALIVE
                    | SIOCDIFPHYADDR
                    | SIOCSLIFPHYADDR
                    | SIOCSLIFPHYRTABLE
                    | SIOCSLIFPHYTTL
                    | SIOCSLIFPHYDF
                    | SIOCSLIFPHYECN
                    | SIOCADDMULTI
                    | SIOCDELMULTI
                    | SIOCSVNETID
                    | SIOCDVNETID
                    | SIOCSVNETFLOWID
                    | SIOCSTXHPRIO
                    | SIOCSRXHPRIO
                    | SIOCSIFPAIR
                    | SIOCSIFPARENT
                    | SIOCDIFPARENT
                    | SIOCSETMPWCFG
                    | SIOCSETLABEL
                    | SIOCDELLABEL
                    | SIOCSPWE3CTRLWORD
                    | SIOCSPWE3FAT
                    | SIOCSPWE3NEIGHBOR
                    | SIOCDPWE3NEIGHBOR
            ) && let Err(e) = suser(p)
            {
                error = Err(e);
                break 'out;
            }
            // SAFETY: the caller's contract.
            error = unsafe { pru_control(so, cmd, data, ifp) };
            if error != Err(Errno::EOPNOTSUPP) {
                break 'out;
            }
            error = match cmd {
                SIOCAIFADDR | SIOCDIFADDR | SIOCSIFADDR | SIOCSIFNETMASK | SIOCSIFDSTADDR
                | SIOCSIFBRDADDR => suser(p),
                #[cfg(feature = "inet6")]
                SIOCAIFADDR_IN6 | SIOCDIFADDR_IN6 => suser(p),
                _ => Ok(()),
            };
            if error.is_err() {
                break 'out;
            }
            net_lock();
            // SAFETY: the caller's contract is the hook's.
            error = unsafe { ifp_ioctl(ifp, cmd, data) };
            net_unlock();
        }
    }

    if forceup {
        // forceup: (SIOCSIFFLAGS, and SIOCSIFXFLAGS bringing an autoconf interface up)
        // The flags are an `unsigned short` in C: the assignment keeps the low sixteen bits.
        let rflags = i32::from(ifr.ifr_flags()) & 0xffff;
        ifp.if_flags
            .set((ifp.if_flags.get() & IFF_CANTCHANGE) | (rflags & !IFF_CANTCHANGE));
        // SAFETY: the caller's contract: `data` is the `ifreq`.
        error = unsafe { ifp_ioctl(ifp, SIOCSIFFLAGS, data) };
        if error.is_err() {
            ifp.if_flags.set(oif_flags);
            if cmd == SIOCSIFXFLAGS {
                ifp.if_xflags.set(oif_xflags);
            }
        } else if (oif_flags ^ ifp.if_flags.get()) & IFF_UP != 0 {
            let s = splnet();
            if ifp.if_flags.get() & IFF_UP != 0 {
                if_up(ifp);
            } else {
                if_down(ifp);
            }
            splx(s);
        }
        net_unlock();
    }

    if oif_flags != ifp.if_flags.get() || oif_xflags != ifp.if_xflags.get() {
        // if_up() and if_down() already sent an update, skip here
        if (oif_flags ^ ifp.if_flags.get()) & IFF_UP == 0 {
            rtm_ifchg(ifp);
        }
    }

    if (oif_flags ^ ifp.if_flags.get()) & IFF_UP != 0 {
        {
            let now = crate::kern::kern_tc::getmicrotime();
            ifp.if_lastchange.set(now.tv_sec, now.tv_usec);
        }
    }

    kernel_unlock();

    if_put(ifp);

    error
}

/// `ifioctl_get`: the read-only interface ioctls.
///
/// # Safety
///
/// As for [`ifioctl`].
unsafe fn ifioctl_get(cmd: u64, data: *mut u8) -> Result<(), Errno> {
    match cmd {
        // SAFETY: the caller's contract: an `ifconf`.
        SIOCGIFCONF => return unsafe { ifconf(data) },
        SIOCIFGCLONERS => {
            // SAFETY: the caller's contract: an `if_clonereq`.
            return if_clone_list(unsafe { &mut *data.cast::<IfClonereq>() });
        }
        // SAFETY: the caller's contract: an `ifgroupreq`.
        SIOCGIFGMEMB => return unsafe { if_getgroupmembers(data) },
        SIOCGIFGATTR => {
            net_lock_shared();
            // SAFETY: the caller's contract: an `ifgroupreq`.
            let error = unsafe { if_getgroupattribs(data) };
            net_unlock_shared();
            return error;
        }
        // SAFETY: the caller's contract: an `ifgroupreq`.
        SIOCGIFGLIST => return unsafe { if_getgrouplist(data) },
        _ => {}
    }

    // SAFETY: the caller's contract: an `ifreq`.
    let ifr = unsafe { &mut *data.cast::<Ifreq>() };

    kernel_lock();
    let ifp = if_unit(&ifr.ifr_name);
    kernel_unlock();

    let Some(ifp) = ifp else {
        return Err(Errno::ENXIO);
    };

    let mut error = Ok(());
    match cmd {
        SIOCGIFFLAGS => {
            let mut flags = ifp.if_flags.get();
            if ifq_is_oactive(ifp.if_snd()) {
                flags |= IFF_OACTIVE;
            }
            ifr.set_ifr_flags(flags as i16);
        }

        SIOCGIFXFLAGS => {
            ifr.set_ifr_flags((ifp.if_xflags.get() & !(IFXF_MPSAFE | IFXF_CLONED)) as i16);
        }

        SIOCGIFMETRIC => ifr.set_ifr_metric(ifp.if_metric.get() as i32),

        SIOCGIFMTU => ifr.set_ifr_mtu(ifp.if_mtu.get() as i32),

        SIOCGIFHARDMTU => ifr.set_ifr_hardmtu(ifp.if_hardmtu.get() as i32),

        SIOCGIFDATA => {
            let mut ifdata = MaybeUninit::<IfData>::zeroed();
            // SAFETY: all-zero is a valid `if_data` (integers and a timeval).
            let ifdata = unsafe { ifdata.assume_init_mut() };

            net_lock_shared();
            kernel_lock();
            if_getdata(ifp, ifdata);
            kernel_unlock();
            net_unlock_shared();

            // SAFETY: zero-filled, then written member by member.
            error = copyout(unsafe { bytes_of(ifdata) }, ifr.ifr_data() as usize);
        }

        SIOCGIFDESCR => {
            let mut ifdescrbuf = [0u8; IFDESCRSIZE];
            kernel_lock();
            let _ = strlcpy(&mut ifdescrbuf, &ifp.if_description.get());
            kernel_unlock();

            error = copyoutstr(&ifdescrbuf, ifr.ifr_data() as usize).map(|_| ());
        }
        SIOCGIFRTLABEL => {
            let rtlabelid = ifp.if_rtlabelid.get();

            let mut ifrtlabelbuf = [0u8; RTLABEL_LEN];
            error = if rtlabelid != 0 && rtlabel_id2name(rtlabelid, &mut ifrtlabelbuf).is_some() {
                copyoutstr(&ifrtlabelbuf, ifr.ifr_data() as usize).map(|_| ())
            } else {
                Err(Errno::ENOENT)
            };
        }
        SIOCGIFPRIORITY => ifr.set_ifr_metric(i32::from(ifp.if_priority.get())),

        SIOCGIFRDOMAIN => ifr.set_ifr_rdomainid(ifp.if_rdomain.get() as i32),

        // SAFETY: the caller's contract: an `ifgroupreq`.
        SIOCGIFGROUP => error = unsafe { if_getgroup(data, ifp) },

        SIOCGIFLLPRIO => ifr.set_ifr_llprio(i32::from(ifp.if_llprio.get())),

        _ => panic(format_args!("invalid ioctl {cmd}")),
    }

    if_put(ifp);

    error
}

/// `if_sffpage_check`.
///
/// # Safety
///
/// `data` points at an `if_sffpage`.
unsafe fn if_sffpage_check(data: *const u8) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let sff = unsafe { &*data.cast::<IfSffpage>() };

    match sff.sff_addr {
        IFSFF_ADDR_EEPROM | IFSFF_ADDR_DDM => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_txhprio_l2_check`: a valid layer 2 transmit header priority setting.
pub fn if_txhprio_l2_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_txhprio_l3_check`: a valid layer 3 transmit header priority setting.
pub fn if_txhprio_l3_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET | IF_HDRPRIO_PAYLOAD => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_rxhprio_l2_check`: a valid layer 2 receive header priority setting.
pub fn if_rxhprio_l2_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET | IF_HDRPRIO_OUTER => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_rxhprio_l3_check`: a valid layer 3 receive header priority setting.
pub fn if_rxhprio_l3_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET | IF_HDRPRIO_PAYLOAD | IF_HDRPRIO_OUTER => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `ifconf`: return interface configuration of system. List may be used in later ioctl's
/// (above) to get other information.
///
/// # Safety
///
/// `data` points at an `ifconf` whose `ifc_req` is a user address.
unsafe fn ifconf(data: *mut u8) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifc = unsafe { &mut *data.cast::<Ifconf>() };
    let mut space = ifc.ifc_len as isize;
    let ifr_size = size_of::<Ifreq>() as isize;
    let sa_size = size_of::<Sockaddr>();

    // If ifc->ifc_len is 0, fill it in with the needed size and return.
    if space == 0 {
        net_lock_shared();
        for ifp in IFNETLIST.0.iter() {
            if ifp.if_addrlist.is_empty() {
                space += ifr_size;
            } else {
                for ifa in ifp.if_addrlist.iter() {
                    // SAFETY: an interface's addresses are valid sockaddrs.
                    let sa_len = usize::from(unsafe { (*ifa.ifa_addr.get()).sa_len });
                    if sa_len > sa_size {
                        space += (sa_len - sa_size) as isize;
                    }
                    space += ifr_size;
                }
            }
        }
        net_unlock_shared();
        ifc.ifc_len = space as i32;
        return Ok(());
    }

    let if_tmplist: hal::list::List<kernel::net::if_var::IfTmplist> = hal::list::List::new();
    let addr_tmplist: hal::list::List<kernel::net::if_var::IfaTmplist> = hal::list::List::new();

    let mut ifrp = ifc.ifc_req() as usize;
    let mut ifr = Ifreq::zeroed();
    let mut error = Ok(());

    rw_enter_write(&IF_TMPLIST_LOCK);
    net_lock_shared();
    for ifp in IFNETLIST.0.iter() {
        let _ = if_ref(ifp);
        // `if_tmplist` is free under `IF_TMPLIST_LOCK`; the reference keeps the interface
        // until it is taken off below.
        if_tmplist.insert_tail(ifp);
    }
    net_unlock_shared();

    'free: for ifp in if_tmplist.iter() {
        if space < ifr_size {
            break 'free;
        }
        ifr.ifr_name = ifp.if_xname.get();

        net_lock_shared();
        for ifa in ifp.if_addrlist.iter() {
            let _ = ifaref(ifa);
            // `ifa_tmplist` is free under `IF_TMPLIST_LOCK`; the reference keeps the address
            // until it is taken off below.
            addr_tmplist.insert_tail(ifa);
        }
        net_unlock_shared();

        if addr_tmplist.is_empty() {
            *ifr.ifr_addr_mut() = Sockaddr {
                sa_len: 0,
                sa_family: 0,
                sa_data: [0; 14],
            };
            // SAFETY: an `Ifreq::zeroed` written member by member.
            error = copyout(unsafe { bytes_of(&ifr) }, ifrp);
            if error.is_err() {
                break 'free;
            }

            space -= ifr_size;
            ifrp += ifr_size as usize;
        } else {
            for ifa in addr_tmplist.iter() {
                let sa = ifa.ifa_addr.get();
                // SAFETY: an interface's addresses are valid sockaddrs of `sa_len` bytes.
                let sa_len = usize::from(unsafe { (*sa).sa_len });
                // SAFETY: as above.
                let sa_bytes = unsafe { slice::from_raw_parts(sa.cast::<u8>(), sa_len) };

                if space < ifr_size {
                    break 'free;
                }
                if sa_len <= sa_size {
                    let mut addr = [0u8; 16];
                    addr[..sa_len].copy_from_slice(sa_bytes);
                    // SAFETY: a `Sockaddr` is 16 bytes of plain data.
                    *ifr.ifr_addr_mut() =
                        unsafe { core::mem::transmute::<[u8; 16], Sockaddr>(addr) };
                    // SAFETY: an `Ifreq::zeroed` written member by member.
                    error = copyout(unsafe { bytes_of(&ifr) }, ifrp);
                    if error.is_err() {
                        break 'free;
                    }

                    space -= ifr_size;
                    ifrp += ifr_size as usize;
                } else {
                    let total = (IFNAMSIZ + sa_len) as isize;

                    if space < total {
                        break 'free;
                    }
                    error = copyout(&ifr.ifr_name, ifrp);
                    if error.is_err() {
                        break 'free;
                    }
                    error = copyout(sa_bytes, ifrp + offset_of!(Ifreq, ifr_ifru));
                    if error.is_err() {
                        break 'free;
                    }

                    space -= total;
                    ifrp += total as usize;
                }
            }
        }

        while let Some(ifa) = addr_tmplist.pop_head() {
            ifafree(ifa);
        }
    }

    // free:
    while let Some(ifa) = addr_tmplist.pop_head() {
        ifafree(ifa);
    }
    while let Some(ifp) = if_tmplist.pop_head() {
        if_put(ifp);
    }
    rw_exit_write(&IF_TMPLIST_LOCK);

    if error.is_ok() {
        ifc.ifc_len -= space as i32;
    }
    error
}

// net/if.c, kept (LZ's shape until its slice).
/// Copies one `ifg_req` naming `name` (a group's or a member's: one union) out to the user
/// array.
fn ifgrq_copyout(name: &[u8], ifgp: usize) -> Result<(), Errno> {
    let mut ifgrq = IfgReq {
        ifgrq_ifgrqu: [0; IFNAMSIZ],
    };
    let _ = strlcpy(&mut ifgrq.ifgrq_ifgrqu, name);
    copyout(&ifgrq.ifgrq_ifgrqu, ifgp)
}

/// The group named `name` (a reference taken), under the net lock's shared hold.
fn ifg_named(name: &[u8]) -> Option<kernel::net::if_var::IfgRef> {
    ifg_head()
        .refs()
        .iter()
        .find(|ifg| name_eq(&ifg.ifg_group, name))
        .cloned()
}

/// `if_getgroup`: stores all groups from an interface in memory pointed to by `data`. The
/// groups are a snapshot taken under the net lock and copied out without it (the C's
/// `ifg_tmplist` with its temporary references).
///
/// # Safety
///
/// `data` points at an `ifgroupreq` whose `ifgr_groups` is a user address.
unsafe fn if_getgroup(data: *mut u8, ifp: &Ifnet) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifgr = unsafe { &mut *data.cast::<Ifgroupreq>() };
    let req = size_of::<IfgReq>();

    if ifgr.ifgr_len == 0 {
        net_lock_shared();
        ifgr.ifgr_len += (ifp.groups().len() * req) as u32;
        net_unlock_shared();
        return Ok(());
    }

    let mut len = ifgr.ifgr_len as usize;
    let mut ifgp = ifgr.ifgr_groups() as usize;
    let mut error = Ok(());

    net_lock_shared();
    let groups = ifp.groups().snapshot();
    net_unlock_shared();

    for ifg in groups.iter() {
        if len < req {
            error = Err(Errno::EINVAL);
            break;
        }
        error = ifgrq_copyout(&ifg.ifg_group, ifgp);
        if error.is_err() {
            break;
        }
        len -= req;
        ifgp += req;
    }

    error
}

/// `if_getgroupmembers`: stores all members of a group in memory pointed to by `data` (a
/// snapshot of the members, references taken as the C's `if_tmplist` does).
///
/// # Safety
///
/// `data` points at an `ifgroupreq` whose `ifgr_groups` is a user address.
unsafe fn if_getgroupmembers(data: *mut u8) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifgr = unsafe { &mut *data.cast::<Ifgroupreq>() };
    let req = size_of::<IfgReq>();
    let mut error = Ok(());

    net_lock_shared();

    let Some(ifg) = ifg_named(&ifgr.ifgr_name) else {
        net_unlock_shared();
        return Err(Errno::ENOENT);
    };

    if ifgr.ifgr_len == 0 {
        ifgr.ifgr_len += (ifg.members().as_slice().len() * req) as u32;
        net_unlock_shared();
        return Ok(());
    }

    let members = ifg.members().snapshot();
    for ifp in members.iter() {
        let _ = if_ref(ifp);
    }
    net_unlock_shared();

    let mut len = ifgr.ifgr_len as usize;
    let mut ifgp = ifgr.ifgr_groups() as usize;

    for ifp in members.iter() {
        if len < req {
            error = Err(Errno::EINVAL);
            break;
        }
        error = ifgrq_copyout(&ifp.if_xname.get(), ifgp);
        if error.is_err() {
            break;
        }
        len -= req;
        ifgp += req;
    }

    for ifp in members.iter() {
        if_put(ifp);
    }

    error
}

/// `if_getgroupattribs`: the group's attributes (`carp(4)`'s demotion counter).
///
/// # Safety
///
/// `data` points at an `ifgroupreq`.
unsafe fn if_getgroupattribs(data: *mut u8) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifgr = unsafe { &mut *data.cast::<Ifgroupreq>() };

    let Some(ifg) = ifg_named(&ifgr.ifgr_name) else {
        return Err(Errno::ENOENT);
    };

    ifgr.set_ifgr_attrib(IfgAttrib {
        ifg_carp_demoted: ifg.ifg_carp_demoted.get(),
    });

    Ok(())
}

/// `if_setgroupattribs`: the group's demotion counter changed by the request's (`EINVAL` past
/// 0..255), each member told.
///
/// # Safety
///
/// `data` points at an `ifgroupreq`.
unsafe fn if_setgroupattribs(data: *mut u8) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifgr = unsafe { &*data.cast::<Ifgroupreq>() };

    let Some(ifg) = ifg_named(&ifgr.ifgr_name) else {
        return Err(Errno::ENOENT);
    };

    let demote = ifgr.ifgr_attrib().ifg_carp_demoted;
    let sum = demote + ifg.ifg_carp_demoted.get();
    if !(0..=0xff).contains(&sum) {
        return Err(Errno::EINVAL);
    }

    ifg.ifg_carp_demoted.set(sum);

    let members = ifg.members().snapshot();
    for ifp in members.iter() {
        // SAFETY: `data` is the `ifgroupreq` of `SIOCSIFGATTR` (the caller's contract), what
        // the driver's handler reads for it.
        let _ = unsafe { ifp_ioctl(ifp, SIOCSIFGATTR, data) };
    }

    Ok(())
}

/// `if_getgrouplist`: stores all groups in memory pointed to by `data` (a snapshot of
/// `ifg_head`, the C's `ifg_tmplist`).
///
/// # Safety
///
/// `data` points at an `ifgroupreq` whose `ifgr_groups` is a user address.
unsafe fn if_getgrouplist(data: *mut u8) -> Result<(), Errno> {
    // SAFETY: the caller's contract.
    let ifgr = unsafe { &mut *data.cast::<Ifgroupreq>() };
    let req = size_of::<IfgReq>();

    if ifgr.ifgr_len == 0 {
        net_lock_shared();
        ifgr.ifgr_len += (ifg_head().len() * req) as u32;
        net_unlock_shared();
        return Ok(());
    }

    let mut len = ifgr.ifgr_len as usize;
    let mut ifgp = ifgr.ifgr_groups() as usize;
    let mut error = Ok(());

    net_lock_shared();
    let groups = ifg_head().snapshot();
    net_unlock_shared();

    for ifg in groups.iter() {
        if len < req {
            error = Err(Errno::EINVAL);
            break;
        }
        error = ifgrq_copyout(&ifg.ifg_group, ifgp);
        if error.is_err() {
            break;
        }
        len -= req;
        ifgp += req;
    }

    error
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

// net/if.c, kept (LZ's shape until its slice).
/// `if_rxr_info_ioctl`: copies `e` out for `SIOCGIFRXR`; `uifri` is the user's
/// `if_rxrinfo`.
pub fn if_rxr_info_ioctl(uifri: usize, t: u32, e: &[IfRxringInfo]) -> Result<(), Errno> {
    let mut kifri = MaybeUninit::<IfRxrinfo>::zeroed();
    // SAFETY: an `if_rxrinfo` is an integer and a pointer, valid for any bytes; the slice is
    // the structure's bytes.
    let kifri = unsafe {
        let bytes =
            slice::from_raw_parts_mut(kifri.as_mut_ptr().cast::<u8>(), size_of::<IfRxrinfo>());
        copyin(uifri, bytes)?;
        kifri.assume_init_mut()
    };

    let n = t.min(kifri.ifri_total) as usize;
    kifri.ifri_total = t;

    if n > 0 {
        // SAFETY: `if_rxring_info` has no padding (16 + 4 + 20 bytes of integers).
        let bytes = unsafe {
            slice::from_raw_parts(
                e.as_ptr().cast::<u8>(),
                size_of::<IfRxringInfo>() * n.min(e.len()),
            )
        };
        copyout(bytes, kifri.ifri_entries as usize)?;
    }

    // SAFETY: zero-filled, then written member by member.
    copyout(unsafe { bytes_of(kifri) }, uifri)
}

/// `if_rxr_ioctl`: `SIOCGIFRXR` for a driver with one ring.
pub fn if_rxr_ioctl(
    ifri: usize,
    name: Option<&[u8]>,
    size: u32,
    rxr: &IfRxring,
) -> Result<(), Errno> {
    let mut ifr = IfRxringInfo {
        ifr_name: [0; 16],
        ifr_size: size,
        ifr_info: *rxr,
    };

    if let Some(name) = name {
        let _ = strlcpy(&mut ifr.ifr_name, name);
    }

    if_rxr_info_ioctl(ifri, 1, slice::from_ref(&ifr))
}

// net/if.c, kept (LZ's shape until its slice).
/// The bytes `ifioctl` works on for `cmd`: the structure the command names
/// (`IOCPARM_LEN(cmd)`, up to `IOCPARM_MAX`, which `sys_ioctl` copied whole) and at least the
/// `struct ifreq` every interface request starts with (its name is read first).
fn ifioctl_need(cmd: u64) -> usize {
    (hal::sys::ioccom::iocparm_len(cmd) as usize).max(size_of::<Ifreq>())
}

/// Where `ifioctl` works on `data` for `cmd`: `None` for `data` itself, when it holds
/// [`ifioctl_need`] bytes aligned for `u64` (the kernel's stack buffer and its heap ones of
/// the right length); else a zeroed heap copy of `data` at least that long, `u64`-aligned,
/// whose first `data.len()` bytes go back to `data` after the call. Never a shorter one.
fn ifioctl_buffer(data: &[u8], cmd: u64) -> Result<Option<hal::malloc::MArray<u64>>, Errno> {
    let need = ifioctl_need(cmd);
    if data.len() >= need && data.as_ptr().align_offset(align_of::<u64>()) == 0 {
        return Ok(None);
    }
    let words = need.max(data.len()).div_ceil(size_of::<u64>());
    let Some(mut c) = hal::malloc::MArray::new(
        crate::sys::malloc::M_IOCTLOPS,
        crate::sys::malloc::M_WAITOK,
        words,
        |_| 0u64,
    ) else {
        return Err(Errno::ENOMEM);
    };
    hal::user::bytes_of_mut(&mut c[..])[..data.len()].copy_from_slice(data);
    Ok(Some(c))
}

/// The interface layer's legs of the kernel's socket layer (the `IfLegacy` row of
/// `bridges.toml`): `if.c`'s `ifioctl` and `net_tq`, this file's until N6.3a (N6.2b moved
/// them here from `sys/kern/sys_socket.rs`, which went).
pub struct BsdIfLegacy;

/// The one [`BsdIfLegacy`], registered by `domaininit`.
pub static IFLEGACY: BsdIfLegacy = BsdIfLegacy;

impl kernel::net::domain::IfLegacy<crate::kern::proc_glue::BsdProcGlue> for BsdIfLegacy {
    /// `ifioctl(so, cmd, data, p)` on `sys_ioctl`'s copy of the request, in place when it is
    /// as long as `ifioctl` works on and aligned for it (the kernel's stack buffer is), else
    /// through a heap copy of that length ([`ifioctl_buffer`]).
    fn ifioctl(
        &self,
        so: &crate::sys::socketvar::Socket,
        cmd: u64,
        data: &mut [u8],
        p: &Proc,
    ) -> Result<(), Errno> {
        let mut copy = ifioctl_buffer(data, cmd)?;
        let bytes = match copy.as_mut() {
            Some(c) => hal::user::bytes_of_mut(&mut c[..]).as_mut_ptr(),
            None => data.as_mut_ptr(),
        };
        // SAFETY: `bytes` points at `ifioctl_need(cmd)` or more bytes aligned for `u64`
        // (`ifioctl_buffer`'s promise: `data` itself, checked for both, or the heap copy it
        // made that long), so the request structure `ifioctl` casts it to (`IOCPARM_LEN(cmd)`
        // bytes, at least the `struct ifreq` every interface request starts with) is in
        // bounds and aligned, and nothing else uses the bytes during the call (`data` is
        // ours exclusively, the copy local); `ifioctl` keeps no pointer past the call; `so`
        // is the caller's socket, alive across it.
        let error = unsafe { ifioctl(core::ptr::from_ref(so).cast(), cmd, bytes, p) };
        if let Some(c) = &copy {
            data.copy_from_slice(&hal::user::bytes_of(&c[..])[..data.len()]);
        }
        error
    }

    /// `net_tq(flowid)`.
    fn net_tq(&self, flowid: u32) -> &'static kernel::task::Taskq {
        match net_tq(flowid) {
            Some(tq) => tq,
            None => panic(format_args!("net_tq: no softnet queue")),
        }
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};
    use crate::sys::mbuf::mtod;

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

    #[test]
    fn header_priority_checks() {
        assert_eq!(if_txhprio_l2_check(IF_HDRPRIO_PACKET), Ok(()));
        assert_eq!(if_txhprio_l2_check(IF_HDRPRIO_PAYLOAD), Err(Errno::EINVAL));
        assert_eq!(if_txhprio_l3_check(IF_HDRPRIO_PAYLOAD), Ok(()));
        assert_eq!(if_rxhprio_l2_check(IF_HDRPRIO_OUTER), Ok(()));
        assert_eq!(if_rxhprio_l3_check(IF_HDRPRIO_OUTER), Ok(()));
        assert_eq!(if_txhprio_l3_check(IF_HDRPRIO_OUTER), Err(Errno::EINVAL));
        for prio in IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX {
            assert_eq!(if_rxhprio_l3_check(prio), Ok(()));
        }
        assert_eq!(if_txhprio_l2_check(IF_HDRPRIO_MAX + 1), Err(Errno::EINVAL));
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
        use crate::netinet6::in6::{IN6ADDR_LOOPBACK, in6ifa_ifpforlinklocal, in6ifa_ifpwithaddr};

        let _g = crate::netinet::ip_input::tests::setup();
        loop_clone_create(&LOOP_CLONER, 0).expect("lo0");
        loop_clone_create(&LOOP_CLONER, 1).expect("lo1");
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
    fn ifioctl_works_on_the_whole_request() {
        use crate::sys::sockio::SIOCGLIFPHYADDR;
        let (_g, _t, p) = crate::netinet::in_pcb::tests::setup();
        // A request longer than STK_PARAMS (536 bytes): sys_ioctl's heap buffer.
        let need = ifioctl_need(SIOCGLIFPHYADDR);
        assert!(need > 128, "{need}");
        let mut words: std::vec::Vec<u64> = std::vec![0; need.div_ceil(8) + 1];
        let bytes = hal::user::bytes_of_mut(&mut words[..]);
        // Aligned and long enough: worked on in place.
        assert!(
            ifioctl_buffer(&bytes[..need], SIOCGLIFPHYADDR)
                .unwrap()
                .is_none()
        );
        // Misaligned: a copy, never shorter than the request.
        let c = ifioctl_buffer(&bytes[1..1 + need], SIOCGLIFPHYADDR)
            .unwrap()
            .expect("a copy");
        assert!(c.len() * 8 >= need);
        // A void command's 8 bytes: a copy as long as a struct ifreq.
        let c = ifioctl_buffer(&bytes[..8], 0x2000_6900)
            .unwrap()
            .expect("a copy");
        assert!(c.len() * 8 >= size_of::<Ifreq>());

        // Through the bridge, both ways, on an interface that is not there.
        let so = crate::kern::uipc_socket::soalloc(
            &crate::netinet::in_proto::INETSW[1],
            crate::sys::socket::SOCK_DGRAM,
            crate::sys::mbuf::M_WAIT,
        )
        .unwrap();
        for at in [0usize, 1] {
            let data = &mut bytes[at..at + need];
            data[..8].copy_from_slice(b"nosuch0\0");
            let r = kernel::net::domain::IfLegacy::ifioctl(&IFLEGACY, so, SIOCGLIFPHYADDR, data, p);
            assert_eq!(r, Err(Errno::ENXIO), "at {at}");
            assert_eq!(&data[..8], b"nosuch0\0");
        }
        crate::netinet::in_pcb::tests::teardown();
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
}
/* </TESTS> */
