/*	$KAME: ip6_output.c,v 1.172 2001/03/25 09:55:56 itojun Exp $	*/
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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)ip_output.c	8.3 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 output as `bsd`'s callers know it: `ip6_output`, fragmentation, the output checksums
//! and the socket options (`ip6_ctloutput`, `ip6_raw_ctloutput`) with LZ's signatures over the
//! kernel's (`kernel::netinet6::ip6_output`, N6.5c); the generator of the fragment ids
//! (`ip6_id_ctx`, over `bsd`'s `idgen32`), which the kernel reaches through the `Inet` bridge's
//! `ip6_randomid` leg; and IPsec's half of `ip6_output` (`ip6_output_ipsec_lookup`,
//! `ip6_output_ipsec_send`, `ip6_output_ipsec_pmtu_update`), which the kernel reaches through
//! the `Ipsec` bridge's `ip6_output_lookup`/`ip6_output_send` legs until IPsec moves (N8).
//!
//! Upstream: sys/netinet6/ip6_output.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `ip6_output`'s `struct route *`, `struct ip6_pktopts *`, `struct ip6_moptions *` and
//!   `const struct ipsec_level *` are `Option`s; the error is a `Result`. The packet is a
//!   `&'static Mbuf`, handed to the kernel's `ip6_output` (unparked, consumed), the options
//!   lent as the kernel's view (`Ip6Pktopts::as_opts`), the net lock the caller holds proved
//!   through the `SaToken`.
//! - IPsec: `ip6_output_ipsec_lookup` returns the SA (`Ok(None)` for no IPsec) or the SPD's
//!   verdict (`SpdError::Drop` is the C's `-EINVAL`, a silent drop). `NPF` (pf(4)) is
//!   configured: `ip6_output_ipsec_send` runs `pf_test` on the SA's `enc(4)` interface. The
//!   IPv6 halves of `netinet/ip_spd.rs` and `netinet/ipsec_output.rs` are still comments
//!   there (phase 2 of the INET6 port), so an IPv6 flow finds no policy yet.
//! - `ip6_id_ctx` is behind a mutex of its own (a `hal::mutex::Mutexed`, IPL_SOFTNET; the C
//!   calls `idgen32` unlocked; here `ip6_randomid` can run on several CPUs under the shared net
//!   lock).

use core::mem::size_of;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::intr::IPL_SOFTNET;
use hal::mutex::Mutexed;

use crate::crypto::idgen::Idgen32Ctx;
use crate::kern::kern_tc::gettime;
use crate::kern::subr_prf::panic;
use crate::kern::uipc_mbuf::{m_freem, ml_dequeue, ml_enqueue, ml_init, ml_purge};
use crate::kern::uipc_mbuf2::{m_tag_first, m_tag_next};
use crate::net::if_::net_held;
use crate::net::if_enc::enc_getif;
use crate::net::if_var::Ifnet;
use crate::net::pf::pf_test;
use crate::net::pfvar::{PF_FWD, PF_OUT, PF_PASS};
use crate::net::route::{RT_RESOLVE, RTF_HOST, Route, as_static, rtfree};
use crate::netinet::in_pcb::sotoinpcb;
use crate::netinet::ip_input::ip_mtudisc;
use crate::netinet::ip_ipsp::{
    IPSP_DF_INHERIT, IPSP_DIRECTION_OUT, IpsecCounters, IpsecLevel, Tdb, TdbCounters, TdbIdent,
    ipsecstat_inc, tdb_unref, tdbstat_inc,
};
use crate::netinet::ip_spd::{SpdError, ipsp_spd_lookup};
use crate::netinet::ip6::Ip6Hdr;
use crate::netinet::ipsec_output::{ipsec_adjust_mtu, ipsp_process_packet};
use crate::netinet::tcp_output::tcp_softtso_chop;
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};
use crate::netinet6::icmp6::icmp6_mtudisc_clone;
use crate::netinet6::in6::{In6Addr, SockaddrIn6, in6_addr2scopeid, in6_are_addr_equal};
use crate::netinet6::in6_src::in6_embedscope;
use crate::netinet6::ip6_var::{Ip6Moptions, Ip6Pktopts, mtod_ip6};
use crate::sys::endian::{ntohl, ntohs};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{
    M_BCAST, M_IPV6_DF_OUT, M_MCAST, M_TCP_TSO, Mbuf, MbufList, PACKET_TAG_IPSEC_OUT_DONE, mut_of,
    unpark,
};
use crate::sys::socket::AF_INET6;
use crate::sys::socketvar::Socket;
use crate::sys::systm::{kernel_lock, kernel_unlock};

pub use kernel::netinet6::ip6_output::{ip6_clearpktopts, ip6_setpktopts, ro_sin6};

/// `ip6_id_ctx`: context for non-repeating IDs, behind its mutex (the module's deviations).
static IP6_ID_CTX: Mutexed<Idgen32Ctx> = Mutexed::new(IPL_SOFTNET, Idgen32Ctx::zeroed());

/// `ip6_output(m, opt, ro, flags, im6o, seclevel)`: IP6 output, the kernel's
/// (`kernel::netinet6::ip6_output::ip6_output`) for `bsd`'s callers: the parked packet `m`
/// handed over (consumed), the options lent as the kernel's view, the net lock the caller
/// holds proved through the `SaToken`.
pub fn ip6_output(
    m: &'static Mbuf,
    opt: Option<&Ip6Pktopts>,
    ro: Option<&mut Route>,
    flags: i32,
    im6o: Option<&Ip6Moptions>,
    seclevel: Option<&IpsecLevel>,
) -> Result<(), Errno> {
    kernel::netinet6::ip6_output::ip6_output(
        unpark(m),
        opt.map(Ip6Pktopts::as_opts),
        ro,
        flags,
        im6o,
        seclevel,
        net_held("ip6_output").net(),
    )
}

/// `ip6_fragment(m0, ml, hlen, nextproto, mtu)`: splits `m0` into fragments for `mtu` on `ml`
/// (the kernel's, over `bsd`'s list); consumes the packet, on error `ml` is purged.
pub fn ip6_fragment(
    m0: &'static Mbuf,
    ml: &MbufList,
    hlen: i32,
    nextproto: u8,
    mtu: u64,
) -> Result<(), Errno> {
    ml.change(|l| kernel::netinet6::ip6_output::ip6_fragment(unpark(m0), l, hlen, nextproto, mtu))
}

/// `ip6_ctloutput`: IP6 socket option processing (the kernel's, `pr_ctloutput` of UDP6 and,
/// behind `rip6_ctloutput` and `tcp_ctloutput`, of raw IPv6 and TCP), on the socket's
/// control block.
pub fn ip6_ctloutput(
    op: i32,
    so: &'static Socket,
    level: i32,
    optname: i32,
    m: Option<&'static Mbuf>,
) -> Result<(), Errno> {
    let Some(inp) = sotoinpcb(so) else {
        panic(format_args!("ip6_ctloutput: socket {:p} without inpcb", so));
    };
    kernel::netinet6::ip6_output::ip6_ctloutput(
        op,
        inp,
        level,
        optname,
        m.map(mut_of),
        net_held("ip6_ctloutput").net(),
    )
}

/// `ip6_raw_ctloutput`: the raw-socket IPv6 options (`IPV6_CHECKSUM`, the kernel's), on the
/// socket's control block.
pub fn ip6_raw_ctloutput(
    op: i32,
    so: &'static Socket,
    level: i32,
    optname: i32,
    m: Option<&'static Mbuf>,
) -> Result<(), Errno> {
    let Some(inp) = sotoinpcb(so) else {
        panic(format_args!(
            "ip6_raw_ctloutput: socket {:p} without inpcb",
            so
        ));
    };
    kernel::netinet6::ip6_output::ip6_raw_ctloutput(op, inp, level, optname, m.map(mut_of))
}

/// `idgen32(&ip6_id_ctx)`: the next fragment id of `bsd`'s generator, the kernel's
/// `ip6_randomid` through the `Inet` leg (`BsdNetLegs::ip6_randomid`).
pub fn ip6_id_generate() -> u32 {
    IP6_ID_CTX.lock().generate()
}

/// `ip6_randomid_init`: seeds the IPv6 fragment identification generator.
pub fn ip6_randomid_init() {
    IP6_ID_CTX.lock().init();
}

/// `in6_proto_cksum_out`: the upper-layer checksum of an outgoing packet (the kernel's, on the
/// packet `bsd`'s caller owns).
pub fn in6_proto_cksum_out(m: &Mbuf, ifp: Option<&Ifnet>) {
    kernel::netinet6::ip6_output::in6_proto_cksum_out(mut_of(m), ifp);
}

/// `ip6_output_ipsec_lookup`: the SA the policy wants applied to `m`, with a reference;
/// `None` when no IPsec is needed (no SA, or the packet already went through this one).
/// Checks whether there was an outgoing SA bound to the flow from a transport protocol.
pub fn ip6_output_ipsec_lookup(
    m: &Mbuf,
    seclevel: Option<&IpsecLevel>,
) -> Result<Option<&'static Tdb>, SpdError> {
    let mut tdb = None;

    // Do we have any pending SAs to apply ?
    ipsp_spd_lookup(
        m,
        i32::from(AF_INET6),
        size_of::<Ip6Hdr>() as i32,
        IPSP_DIRECTION_OUT,
        None,
        seclevel,
        Some(&mut tdb),
        None,
    )?;
    let Some(t) = tdb else {
        return Ok(None);
    };
    // Loop detection
    let mut mtag = m_tag_first(m);
    while let Some(tag) = mtag {
        if tag.m_tag_id.get() == PACKET_TAG_IPSEC_OUT_DONE {
            // `IPSEC_OUT_DONE` tags carry a `struct tdb_ident`: a copy of the tag's data
            // (`PACKET_TAG_MAXSIZE` bytes, more than the structure), read through `hal::pod`.
            if let Some(tdbi) = hal::pod::at::<TdbIdent>(&tag.bytes(), 0)
                && tdbi.spi == t.tdb_spi.get()
                && tdbi.proto == t.tdb_sproto.get()
                && tdbi.rdomain == t.tdb_rdomain.get()
                && tdbi.dst == t.tdb_dst.get()
            {
                // no IPsec needed
                tdb_unref(Some(t));
                return Ok(None);
            }
        }
        mtag = m_tag_next(m, tag);
    }
    Ok(Some(t))
}

/// `ip6_output_ipsec_pmtu_update`: stores the SA's MTU in a host route to `dst` (cloned
/// when needed, the scope of `dst` taken from interface `ifidx`), but not for transport
/// mode SAs.
fn ip6_output_ipsec_pmtu_update(
    tdb: &Tdb,
    ro: Option<&mut Route>,
    dst: &mut In6Addr,
    ifidx: u32,
    rtableid: u32,
    transportmode: bool,
) -> Result<(), Errno> {
    let mut rt_mtucloned = false;

    // Find a host route to store the mtu in
    let mut rt = ro.as_deref().and_then(Route::rt).map(as_static);
    // but don't add a PMTU route for transport mode SAs
    if transportmode {
        rt = None;
    } else if rt.is_none_or(|r| r.rt_flags.get() & RTF_HOST == 0) {
        let mut sin6 = SockaddrIn6::with_addr(*dst);
        sin6.sin6_scope_id = in6_addr2scopeid(ifidx, dst) as u32;
        // should be impossible to fail
        in6_embedscope(dst, &sin6, None, None)?;
        rt = icmp6_mtudisc_clone(&sin6, rtableid, true);
        rt_mtucloned = true;
    }
    crate::ipsec_dprintf!(
        "ip6_output_ipsec_pmtu_update",
        "spi {:08x} mtu {} rt {:p} cloned {}",
        ntohl(tdb.tdb_spi.get()),
        tdb.tdb_mtu.get(),
        rt.map_or(ptr::null(), ptr::from_ref),
        rt_mtucloned
    );
    if let Some(r) = rt {
        r.rt_mtu().store(tdb.tdb_mtu.get(), Ordering::Relaxed);
        if let Some(ro) = ro
            && ro.rt().is_some()
        {
            ro.ro_rt = None;
            ro.ro_tableid = rtableid;
            ro.ro_rt = kernel::net::route::rtalloc(
                ro.dst(),
                RT_RESOLVE,
                rtableid,
                net_held("ip6_output_ipsec_pmtu_update").net(),
            );
        }
        if rt_mtucloned {
            rtfree(Some(r));
        }
    }
    Ok(())
}

/// `ip6_output_ipsec_send`: hands `m` to the SA `tdb` (after the path MTU check), as many
/// packets as TSO chopping makes; `rtableid`: the packet's original routing table, `fwd`:
/// forwarded. Consumes the packet.
pub fn ip6_output_ipsec_send(
    tdb: &'static Tdb,
    m: &'static Mbuf,
    ro: Option<&mut Route>,
    rtableid: u32,
    fwd: bool,
) -> Result<(), Errno> {
    let ml = MbufList::new();
    let ip_mtudisc_local = ip_mtudisc.load(Ordering::Relaxed);
    let mut tso = false;

    // Packet filter
    let encif = enc_getif(tdb.tdb_rdomain.get(), tdb.tdb_tap.get());
    let mut mp = Some(m);
    let Some(e) = encif else {
        m_freem(m);
        return Err(Errno::EACCES);
    };
    if pf_test(AF_INET6, if fwd { PF_FWD } else { PF_OUT }, e, &mut mp) != PF_PASS {
        m_freem(mp);
        return Err(Errno::EACCES);
    }
    let Some(m) = mp else {
        return Ok(());
    };
    // PF_TAG_REROUTE handling or not... Packet is entering IPsec so the routing is already
    // overruled by the IPsec policy. Until now the change was not reconsidered. What's the
    // behaviour?

    // Check if we can chop the TCP packet
    let ip6 = mtod_ip6(m);
    let len: u32 = if m.m_pkthdr().csum_flags.get() & M_TCP_TSO != 0
        && u32::from(m.m_pkthdr().ph_mss.get()) <= tdb.tdb_mtu.get()
    {
        tso = true;
        u32::from(m.m_pkthdr().ph_mss.get())
    } else {
        size_of::<Ip6Hdr>() as u32 + u32::from(ntohs(ip6.ip6_plen))
    };

    // Check if we are allowed to fragment
    let mut dst = ip6.ip6_dst;
    let ifidx = m.m_pkthdr().ph_ifidx.get();
    if ip_mtudisc_local != 0
        && tdb.tdb_mtu.get() != 0
        && len > tdb.tdb_mtu.get()
        && tdb.tdb_mtutimeout.get() > gettime() as u64
    {
        let tdst = tdb.tdb_dst.get();
        // SAFETY: the union is the 28 bytes of a `sockaddr_in6`, read as one.
        let tsin6 = unsafe { ptr::read_unaligned(tdst.as_sockaddr_ptr().cast::<SockaddrIn6>()) };
        let transportmode =
            tdst.sa().sa_family == AF_INET6 && in6_are_addr_equal(&tsin6.sin6_addr, &dst);
        if let Err(e) =
            ip6_output_ipsec_pmtu_update(tdb, ro, &mut dst, ifidx, rtableid, transportmode)
        {
            ipsecstat_inc(IpsecCounters::IpsecOdrops);
            tdbstat_inc(tdb, TdbCounters::TdbOdrops);
            m_freem(m);
            return Err(e);
        }
        ipsec_adjust_mtu(m, tdb.tdb_mtu.get());
        m_freem(m);
        return Err(Errno::EMSGSIZE);
    }
    // propagate don't fragment for v6-over-v6
    if ip_mtudisc_local != 0 {
        m.m_pkthdr()
            .csum_flags
            .set(m.m_pkthdr().csum_flags.get() | M_IPV6_DF_OUT);
    }

    // Clear these -- they'll be set in the recursive invocation as needed.
    m.m_flags().set(m.m_flags().get() & !(M_BCAST | M_MCAST));

    let mut error: Result<(), Errno> = Ok(());
    'done: {
        if tso {
            error = tcp_softtso_chop(&ml, m, e, len);
            if error.is_err() {
                break 'done;
            }
        } else {
            m.m_pkthdr()
                .csum_flags
                .set(m.m_pkthdr().csum_flags.get() & !M_TCP_TSO);
            in6_proto_cksum_out(m, encif);
            ml_init(&ml);
            ml_enqueue(&ml, m);
        }

        kernel_lock();
        while let Some(m) = ml_dequeue(&ml) {
            // Callee frees mbuf
            error = ipsp_process_packet(m, tdb, i32::from(AF_INET6), false, IPSP_DF_INHERIT);
            if error.is_err() {
                break;
            }
        }
        kernel_unlock();
    }
    // done:
    if error.is_err() {
        ml_purge(&ml);
        ipsecstat_inc(IpsecCounters::IpsecOdrops);
        tdbstat_inc(tdb, TdbCounters::TdbOdrops);
    }
    if error.is_ok() && tso {
        tcpstat_inc(TcpstatCounters::TcpsOutswtso);
    }
    if ip_mtudisc_local != 0 && error == Err(Errno::EMSGSIZE) {
        let _ = ip6_output_ipsec_pmtu_update(tdb, ro, &mut dst, ifidx, rtableid, false);
    }
    error
}
/* </CODE> */
