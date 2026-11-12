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
 *	@(#)if_loop.c	8.1 (Berkeley) 6/10/93
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
//! The loopback interface, lo(4): `net/if_loop.c`, a pseudo-device for protocol testing and
//! timing. `lo0` is made at boot (`loopattach`, a pseudo-device of `pdevinit[]`), more are
//! cloned (`ifconfig lo1 create`), one per routing domain.
//!
//! Upstream: sys/net/if_loop.c @ 3ce1f3f79392
//! LZ: sys/net/if_loop.rs@3c62ede68802
//!
//! A packet sent on a loopback (`looutput`) is queued on one of its input queues
//! (`if_output_local`) rather than handed up at once, which keeps the stack shallow and lets a
//! TCP handshake over the loopback complete; the softnet task gives it back to `loinput`,
//! which passes it to its protocol through `if_input_local`. bpf(4) sees it once, on output,
//! from `if_input_local` (`lo_bpf_mtap` refuses the input tap).
//!
//! ## Deviations
//! - A destroyed loopback's `struct ifnet` is not freed (the C frees it, if_loop.c:216): the
//!   interface list and the index map lend interfaces as `&'static Ifnet`, and no owner type
//!   gives the block back safely until N7's `Device` (D-N6.3c-18; docs/SCAFFOLDING.md). Only
//!   `vmstat -m`'s `devbuf` shows it, one block per `ifconfig lo<n> destroy`; `lo0` is never
//!   destroyed.
//!
//! ## Redesign
//! - LZ's `IfClone` of two function pointers and its nine handle slots set one by one on a
//!   `malloc`ed, zeroed `Ifnet` cast from raw memory (`unsafe`) -> [`LoopCloner`] (the
//!   kernel's first native [`Cloner`]) and [`Loop`], the loopback's [`NetDriver`] and
//!   [`LinkLayer`] at once, set in the interface's `if_driver` and `if_link`: the C's
//!   `if_ioctl`/`if_bpf_mtap` are the driver's methods, `if_output`/`if_input`/`if_rtrequest`
//!   the link layer's (`NetDriver`'s defaults reach them). The interface is an `MBox` of
//!   `M_DEVBUF` made from `Ifnet::new()` and leaked (the deviation above), no raw cast.
//! - `looutput` takes the destination as a `SockaddrView` (its family, a byte) where LZ's
//!   `unsafe fn` read a raw `*const Sockaddr`; `loioctl` reads `SIOCSIFMTU`'s `struct ifreq`
//!   as a copy out of the request's bytes (`hal::pod::at`), never a cast; its `if_up` takes
//!   the caller's detached net lock as a lent proof (`hal::netlock::lend_detached`: every
//!   caller of `SIOCSIFADDR`, `in_ifinit` and `in6_ifinit`, holds `NET_LOCK()`).
//! - `loop_clone_destroy`'s walk of `ifnetlist` runs under the net lock's shared witness, as
//!   the C's `NET_LOCK_SHARED()` section.
//!
//! **Lock and spl transitions** against the C: `loop_clone_create` takes the kernel lock's
//! witness inside `bpfattach` (one spl pair the C's `KERNEL_ASSERT_LOCKED` lacks, bpf.rs's
//! row); `loop_clone_destroy` the C's one shared net lock section; `looutput`, `loinput` and
//! `loioctl` none beyond the C's (`if_output_local`'s queue mutex, `if_up`'s).

use hal::malloc::{M_WAITOK, MBox};
use hal::netlock::NETLOCK;
use hal::sys::errno::Errno;
use hal::sys::net_bpf::DLT_LOOP;
use hal::sys::net_if::{
    IFCAP_CSUM_IPv4, IFCAP_CSUM_TCPv4, IFCAP_CSUM_TCPv6, IFCAP_CSUM_UDPv4, IFCAP_CSUM_UDPv6,
    IFCAP_LRO, IFCAP_TSOv4, IFCAP_TSOv6, IFF_LOOPBACK, IFF_MULTICAST, IFF_RUNNING, IFT_LOOP,
    IFXF_CLONED, IFXF_LRO, Ifreq,
};
use hal::sys::net_route::{RTF_BLACKHOLE, RTF_HOST, RTF_REJECT};
use hal::sys::sockio::{SIOCADDMULTI, SIOCDELMULTI, SIOCSIFADDR, SIOCSIFFLAGS, SIOCSIFMTU};

use crate::assert::kpanic;
use crate::net::bpf::{BpfTap, bpfattach};
use crate::net::if_::{
    IFNETLIST, M_DEVBUF, if_addgroup, if_alloc_sadl, if_attach, if_attach_iqueues,
    if_attach_queues, if_attachhead, if_clone_attach, if_counters_alloc, if_detach, if_input_local,
    if_output_local, if_up, lo_name, softnet_count,
};
use crate::net::if_var::{Cloner, IfCounters, Ifnet, LinkLayer, NetDriver, Netstack, counters_inc};
use crate::net::mbuf::{M_PKTHDR, Mbuf, MbufBox};
use crate::net::proto::SockaddrView;
use crate::net::route::Rtentry;
use crate::net::rtable::{rtable_l2set, rtable_loindex};

/// `LOMTU`: the loopback's MTU.
pub const LOMTU: u32 = 32768;

/// `sizeof(u_int32_t)`: the loopback's link header, the address family (`DLT_LOOP`).
const LOHDRLEN: u32 = 4;

/// `struct if_clone loop_cloner`: the `lo` cloner.
pub struct LoopCloner;

/// `loop_cloner`.
pub static LOOP_CLONER: LoopCloner = LoopCloner;

/// The loopback's driver and link layer: `loioctl`, `lo_bpf_mtap`, `looutput`, `loinput` and
/// `lortrequest`, one object for every loopback (the C's handles carry no state).
pub struct Loop;

/// The one [`Loop`], every loopback's `if_driver` and `if_link`.
pub static LOOP: Loop = Loop;

impl Cloner for LoopCloner {
    fn name(&self) -> &'static [u8] {
        b"lo"
    }

    fn create(&'static self, unit: i32) -> Result<(), Errno> {
        loop_clone_create(self, unit)
    }

    fn has_destroy(&self) -> bool {
        true
    }

    fn destroy(&self, ifp: &'static Ifnet) -> Result<(), Errno> {
        loop_clone_destroy(ifp)
    }
}

impl NetDriver for Loop {
    fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
        loioctl(ifp, cmd, data)
    }

    fn bpf_mtap(&self, if_bpf: &BpfTap, m: &Mbuf, dir: u32) -> bool {
        lo_bpf_mtap(if_bpf, m, dir)
    }
}

impl LinkLayer for Loop {
    fn output(
        &self,
        ifp: &'static Ifnet,
        m: MbufBox,
        dst: SockaddrView<'_>,
        rt: Option<&Rtentry>,
    ) -> Result<(), Errno> {
        looutput(ifp, m, dst, rt)
    }

    fn input(&self, ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
        loinput(ifp, m, ns);
    }

    fn rtrequest(
        &self,
        ifp: &'static Ifnet,
        req: u8,
        rt: Option<&Rtentry>,
        _net: &hal::netlock::NetHeld<'_>,
    ) {
        lortrequest(ifp, req, rt);
    }
}

/// `loopattach(n)`: the pseudo-device's attach: `lo0` made and the cloner registered.
pub fn loopattach(_n: i32) {
    if loop_clone_create(&LOOP_CLONER, 0).is_err() {
        kpanic(format_args!("unable to create lo0"));
    }

    if_clone_attach(&LOOP_CLONER);
}

/// `loop_clone_create(ifc, unit)`: `lo<unit>` made and attached (`lo0` at the head of the
/// interface list, in the `lo` group, the default routing domain's loopback).
pub fn loop_clone_create(ifc: &'static LoopCloner, unit: i32) -> Result<(), Errno> {
    let Some(b) = MBox::new(M_DEVBUF, M_WAITOK, Ifnet::new()) else {
        kpanic(format_args!("loop_clone_create: out of memory"));
    };
    let ifp: &'static Ifnet = MBox::leak(b);

    ifp.if_xname.set(lo_name(u32::try_from(unit).unwrap_or(0)));
    // if_softc = NULL: Ifnet::new()'s.
    ifp.if_mtu.set(LOMTU);
    ifp.if_flags.set(IFF_LOOPBACK | IFF_MULTICAST);
    ifp.if_xflags.set(IFXF_CLONED | IFXF_LRO);
    ifp.if_capabilities.set(
        IFCAP_CSUM_IPv4
            | IFCAP_CSUM_TCPv4
            | IFCAP_CSUM_UDPv4
            | IFCAP_CSUM_TCPv6
            | IFCAP_CSUM_UDPv6
            | IFCAP_LRO
            | IFCAP_TSOv4
            | IFCAP_TSOv6,
    );
    // if_bpf_mtap, if_ioctl: the driver; if_rtrequest, if_input, if_output: the link layer.
    if ifp.if_driver.set(&LOOP).is_err() || ifp.if_link.set(&LOOP).is_err() {
        kpanic(format_args!(
            "loop_clone_create: a fresh interface with handles"
        ));
    }
    ifp.if_type.set(IFT_LOOP);
    ifp.if_hdrlen.set(LOHDRLEN as u8);
    if_counters_alloc(ifp);
    if unit == 0 {
        if_attachhead(ifp);
        let _ = if_addgroup(ifp, ifc.name());
        rtable_l2set(0, 0, ifp.if_index.get());
    } else {
        if_attach(ifp);
    }
    if_attach_queues(ifp, softnet_count());
    if_attach_iqueues(ifp, softnet_count());
    if_alloc_sadl(ifp);
    bpfattach(&ifp.if_bpf, ifp, DLT_LOOP, LOHDRLEN);
    Ok(())
}

/// `loop_clone_destroy(ifp)`: a cloned loopback detached, unless it is a routing domain's:
/// `EPERM` for the default domain's, `EBUSY` while another interface is in its domain.
pub fn loop_clone_destroy(ifp: &'static Ifnet) -> Result<(), Errno> {
    let mut rdomain = 0;

    if ifp.if_index.get() == rtable_loindex(ifp.if_rdomain.get()) {
        // rdomain 0 always needs a loopback
        if ifp.if_rdomain.get() == 0 {
            return Err(Errno::EPERM);
        }

        // if there is any other interface in this rdomain, deny
        let busy = {
            let _net = NETLOCK.read();
            IFNETLIST.iter().any(|p| {
                p.if_rdomain.get() == ifp.if_rdomain.get() && p.if_index.get() != ifp.if_index.get()
            })
        };
        if busy {
            return Err(Errno::EBUSY);
        }

        rdomain = ifp.if_rdomain.get();
    }

    if_detach(ifp);

    // free(ifp, M_DEVBUF, sizeof(*ifp)): kept until N7 (the module's deviations).

    if rdomain != 0 {
        rtable_l2set(rdomain, 0, 0);
    }
    Ok(())
}

/// `lo_bpf_mtap`: the loopback taps on output only (`if_input_local`), so the input tap
/// keeps every packet and sees none.
pub fn lo_bpf_mtap(_if_bpf: &BpfTap, _m: &Mbuf, _dir: u32) -> bool {
    false
}

/// `loinput(ifp, m, ns)`: a looped packet, from the input queue, to its protocol; a
/// protocol that refuses it counts an input error.
pub fn loinput(ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
    if m.flags() & M_PKTHDR == 0 {
        kpanic(format_args!("loinput: no header mbuf"));
    }

    let af = m.m_pkthdr().ph_family.get();
    if if_input_local(ifp, m, af, ns).is_err()
        && let Some(c) = ifp.if_counters.get()
    {
        counters_inc(c, IfCounters::IfcIerrors);
    }
}

/// `looutput(ifp, m, dst, rt)`: a packet for `dst` queued for local delivery; a reject or
/// blackhole route drops it (`EHOSTUNREACH` for a host route, `ENETUNREACH` for a net,
/// success for a blackhole).
pub fn looutput(
    ifp: &'static Ifnet,
    m: MbufBox,
    dst: SockaddrView<'_>,
    rt: Option<&Rtentry>,
) -> Result<(), Errno> {
    if m.flags() & M_PKTHDR == 0 {
        kpanic(format_args!("looutput: no header mbuf"));
    }

    if let Some(rt) = rt {
        let flags = rt.rt_flags.get();
        if flags & (RTF_REJECT | RTF_BLACKHOLE) != 0 {
            drop(m);
            return if flags & RTF_BLACKHOLE != 0 {
                Ok(())
            } else if flags & RTF_HOST != 0 {
                Err(Errno::EHOSTUNREACH)
            } else {
                Err(Errno::ENETUNREACH)
            };
        }
    }

    // Do not call if_input_local() directly. Queue the packet to avoid stack overflow and
    // make TCP handshake over loopback work.
    if_output_local(ifp, m, dst.family())
}

/// `lortrequest(ifp, cmd, rt)`: a route over the loopback takes the loopback's MTU when it
/// has none.
pub fn lortrequest(_ifp: &'static Ifnet, _cmd: u8, rt: Option<&Rtentry>) {
    if let Some(rt) = rt {
        let _ = rt.rt_rmx.rmx_mtu.compare_exchange(
            0,
            LOMTU,
            core::sync::atomic::Ordering::Relaxed,
            core::sync::atomic::Ordering::Relaxed,
        );
    }
}

/// `loioctl(ifp, cmd, data)`: the loopback's requests: `SIOCSIFFLAGS` (TSO follows LRO),
/// `SIOCSIFADDR` (running and up), the multicast requests (nothing to filter) and
/// `SIOCSIFMTU`; `ENOTTY` for the rest.
pub fn loioctl(ifp: &'static Ifnet, cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
    match cmd {
        SIOCSIFFLAGS => {
            let caps = ifp.if_capabilities.get();
            if ifp.if_xflags.get() & IFXF_LRO != 0 {
                ifp.if_capabilities.set(caps | IFCAP_TSOv4 | IFCAP_TSOv6);
            } else {
                ifp.if_capabilities.set(caps & !(IFCAP_TSOv4 | IFCAP_TSOv6));
            }
            Ok(())
        }

        SIOCSIFADDR => {
            ifp.if_flags.set(ifp.if_flags.get() | IFF_RUNNING);
            // send up RTM_IFINFO, under the caller's NET_LOCK()
            hal::netlock::lend_detached(|net| if_up(ifp, net));
            // Everything else is done at a higher level.
            Ok(())
        }

        SIOCADDMULTI | SIOCDELMULTI => Ok(()),

        SIOCSIFMTU => {
            let ifr = hal::pod::at::<Ifreq>(data, 0).ok_or(Errno::EINVAL)?;
            ifp.if_mtu.set(ifr.ifr_mtu() as u32);
            Ok(())
        }

        _ => Err(Errno::ENOTTY),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::sync::MutexGuard;

    use core::sync::atomic::Ordering;

    use hal::sys::net_route::RTP_DEFAULT;
    use hal::sys::socket::AF_INET;

    use super::*;
    use crate::net::if_::{
        TEST_PROTO_INPUTS, TestDriver, if_clone_destroy, if_input_process, if_put, if_unit,
        test_ifnet_fresh, test_packet,
    };
    use crate::net::ifq::ifiq_test_take;
    use crate::net::route::tests::{entry, sin, testroutes_setup};

    /// The routing tests' fixture (table 0 of inet and inet6, their `Inet` double) and the
    /// cloner registered once.
    fn setup() -> MutexGuard<'static, ()> {
        let g = testroutes_setup();
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| if_clone_attach(&LOOP_CLONER));
        g
    }

    /// The interface's stack count of packets sent (`ifc_opackets`).
    fn opackets_of(ifp: &Ifnet) -> u64 {
        ifp.if_counters.get().expect("counters")[IfCounters::IfcOpackets as usize]
            .load(Ordering::Relaxed)
    }

    /// `loop_clone_create` under the kernel lock, as `loopattach` and `SIOCIFCREATE` run.
    fn create(unit: i32) -> &'static Ifnet {
        hal::klock::kernel_lock();
        assert_eq!(LOOP_CLONER.create(unit), Ok(()));
        let name = lo_name(unit as u32);
        let ifp = if_unit(&name).expect("created");
        if_put(Some(ifp));
        hal::klock::kernel_unlock();
        ifp
    }

    #[test]
    fn lo0_is_the_default_domain_s_loopback_and_is_never_destroyed() {
        let _g = setup();
        let ifp = create(0);
        assert_eq!(ifp.if_xname.get()[..4], *b"lo0\0");
        assert_eq!(ifp.if_mtu.get(), LOMTU);
        assert_eq!(ifp.if_flags.get(), IFF_LOOPBACK | IFF_MULTICAST);
        assert_eq!(ifp.if_xflags.get(), IFXF_CLONED | IFXF_LRO);
        assert_eq!(ifp.if_type.get(), IFT_LOOP);
        assert_eq!(ifp.if_hdrlen.get(), 4);
        assert_ne!(ifp.if_capabilities.get() & IFCAP_TSOv6, 0);
        assert!(ifp.sadl().is_some(), "a link name");
        assert_eq!(rtable_loindex(0), ifp.if_index.get());
        assert!(ifp.groups().iter().any(|g| g.name() == b"lo"));
        assert!(ifp.if_counters.get().is_some());
        // The driver and the link layer are the loopback's.
        assert!(!lo_bpf_mtap(&ifp.if_bpf, &test_packet(0, 60), 0));
        hal::klock::kernel_lock();
        assert_eq!(loop_clone_destroy(ifp), Err(Errno::EPERM));
        assert_eq!(if_clone_destroy(b"lo0"), Err(Errno::EPERM));
        hal::klock::kernel_unlock();
    }

    #[test]
    fn a_domain_s_loopback_is_kept_while_another_interface_is_in_its_domain() {
        let _g = setup();
        let lo0 = create(0);
        let lo2 = create(2);
        assert_eq!(lo2.if_xname.get()[..4], *b"lo2\0");
        assert_ne!(rtable_loindex(0), lo2.if_index.get(), "not lo0's domain");
        hal::klock::kernel_lock();
        assert_eq!(crate::net::rtable::rtable_add(2), Ok(()));
        rtable_l2set(2, 2, lo2.if_index.get());
        lo2.if_rdomain.set(2);
        let d: &'static TestDriver = Box::leak(Box::new(TestDriver::new()));
        let other = test_ifnet_fresh(b"tst0", d);
        if_attach(other);
        other.if_rdomain.set(2);
        assert_eq!(loop_clone_destroy(lo2), Err(Errno::EBUSY));
        other.if_rdomain.set(0);
        assert_eq!(
            crate::net::if_::tests::with_smr_thread(|| loop_clone_destroy(lo2)),
            Ok(())
        );
        assert_eq!(rtable_loindex(2), 0, "the domain lost its loopback");
        assert!(if_unit(b"lo2").is_none());
        // A loopback that is no domain's goes at once.
        let lo1 = create(1);
        assert_eq!(
            crate::net::if_::tests::with_smr_thread(|| if_clone_destroy(b"lo1")),
            Ok(())
        );
        assert!(if_unit(b"lo1").is_none());
        let _ = lo1;
        hal::klock::kernel_unlock();
        assert_eq!(rtable_loindex(0), lo0.if_index.get());
    }

    #[test]
    fn a_packet_sent_on_the_loopback_comes_back_to_its_protocol() {
        let _g = setup();
        let ifp = create(0);
        let dst = sin([127, 0, 0, 1]);
        let view = SockaddrView::new(&dst).expect("an address");
        let opackets = opackets_of(ifp);
        TEST_PROTO_INPUTS.store(0, Ordering::Relaxed);
        assert_eq!(looutput(ifp, test_packet(0, 60), view, None), Ok(()));
        // Queued, not handed up: the softnet gives it back to loinput.
        let mut ml = ifiq_test_take(ifp.if_rcv());
        assert_eq!(crate::net::mbuf::ml_len(&ml), 1);
        assert_eq!(TEST_PROTO_INPUTS.load(Ordering::Relaxed), 0);
        if_input_process(ifp, &mut ml, 0);
        assert_eq!(TEST_PROTO_INPUTS.load(Ordering::Relaxed), 1);
        assert_eq!(opackets_of(ifp), opackets + 1, "if_input_local counts it");
        // Through the interface's driver and link layer, as ip_output reaches it.
        assert_eq!(
            ifp.driver().output(ifp, test_packet(0, 60), view, None),
            Ok(())
        );
        let ml = ifiq_test_take(ifp.if_rcv());
        assert_eq!(crate::net::mbuf::ml_len(&ml), 1);
        let m = crate::net::mbuf::ml_dequeue(&mut { ml }).expect("the packet");
        assert_eq!(m.m_pkthdr().ph_family.get(), AF_INET);
    }

    #[test]
    fn a_reject_or_blackhole_route_drops_the_packet_with_the_c_s_answer() {
        let _g = setup();
        let ifp = create(0);
        let dst = sin([127, 0, 0, 2]);
        let view = SockaddrView::new(&dst).expect("an address");
        let cases = [
            (RTF_REJECT | RTF_HOST, Err(Errno::EHOSTUNREACH)),
            (RTF_REJECT, Err(Errno::ENETUNREACH)),
            (RTF_BLACKHOLE, Ok(())),
            (RTF_BLACKHOLE | RTF_HOST, Ok(())),
        ];
        for (flags, want) in cases {
            let rt = entry(&dst, 32, None, flags, RTP_DEFAULT);
            assert_eq!(looutput(ifp, test_packet(0, 60), view, Some(&rt)), want);
            let ml = ifiq_test_take(ifp.if_rcv());
            assert_eq!(crate::net::mbuf::ml_len(&ml), 0, "dropped, never queued");
        }
        // An ordinary route queues it.
        let rt = entry(&dst, 32, None, RTF_HOST, RTP_DEFAULT);
        assert_eq!(looutput(ifp, test_packet(0, 60), view, Some(&rt)), Ok(()));
        assert_eq!(crate::net::mbuf::ml_len(&ifiq_test_take(ifp.if_rcv())), 1);
    }

    #[test]
    fn a_route_over_the_loopback_takes_its_mtu_when_it_has_none() {
        let _g = setup();
        let ifp = create(0);
        let dst = sin([127, 0, 0, 3]);
        let rt = entry(&dst, 32, None, RTF_HOST, RTP_DEFAULT);
        lortrequest(ifp, 0, Some(&rt));
        assert_eq!(rt.rt_rmx.rmx_mtu.load(Ordering::Relaxed), LOMTU);
        rt.rt_rmx.rmx_mtu.store(1500, Ordering::Relaxed);
        lortrequest(ifp, 0, Some(&rt));
        assert_eq!(rt.rt_rmx.rmx_mtu.load(Ordering::Relaxed), 1500, "kept");
        lortrequest(ifp, 0, None);
    }

    #[test]
    fn the_loopback_s_requests() {
        let _g = setup();
        let ifp = create(0);
        let mut req = [0u8; 32];
        // SIOCSIFFLAGS: TSO follows LRO.
        ifp.if_xflags.set(IFXF_CLONED);
        assert_eq!(loioctl(ifp, SIOCSIFFLAGS, &mut req), Ok(()));
        assert_eq!(ifp.if_capabilities.get() & (IFCAP_TSOv4 | IFCAP_TSOv6), 0);
        ifp.if_xflags.set(IFXF_CLONED | IFXF_LRO);
        assert_eq!(loioctl(ifp, SIOCSIFFLAGS, &mut req), Ok(()));
        assert_eq!(
            ifp.if_capabilities.get() & (IFCAP_TSOv4 | IFCAP_TSOv6),
            IFCAP_TSOv4 | IFCAP_TSOv6
        );
        // SIOCSIFMTU: the request's MTU.
        let mut ifr = Ifreq::default();
        ifr.set_ifr_mtu(16384);
        assert!(hal::pod::put(&mut req, 0, &ifr));
        assert_eq!(loioctl(ifp, SIOCSIFMTU, &mut req), Ok(()));
        assert_eq!(ifp.if_mtu.get(), 16384);
        // The multicast requests have nothing to do; the rest is not the loopback's.
        assert_eq!(loioctl(ifp, SIOCADDMULTI, &mut req), Ok(()));
        assert_eq!(loioctl(ifp, SIOCDELMULTI, &mut req), Ok(()));
        assert_eq!(
            loioctl(ifp, hal::sys::sockio::SIOCGIFMEDIA, &mut req),
            Err(Errno::ENOTTY)
        );
        // SIOCSIFADDR: running and up, under the caller's net lock.
        hal::netlock::lock();
        assert_eq!(
            crate::net::if_::if_ioctl(ifp, SIOCSIFADDR, &mut req),
            Ok(())
        );
        hal::netlock::unlock();
        assert_ne!(ifp.if_flags.get() & IFF_RUNNING, 0);
        assert_ne!(ifp.if_flags.get() & hal::sys::net_if::IFF_UP, 0);
    }
}
/* </TESTS> */
