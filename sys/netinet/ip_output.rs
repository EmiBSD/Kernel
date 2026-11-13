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
//! IPv4 output as `bsd`'s callers know it: `ip_output`, fragmentation, option insertion, the
//! output checksums and the socket options (`ip_ctloutput`) with LZ's signatures over the
//! kernel's (`kernel::netinet::ip_output`, N6.4a and N6.4c1); and IPsec's half of
//! `ip_output` (`ip_output_ipsec_lookup`, `ip_output_ipsec_send`,
//! `ip_output_ipsec_pmtu_update`), which the kernel reaches through the `Ipsec` bridge's legs
//! until IPsec moves (N8).
//!
//! Upstream: sys/netinet/ip_output.c @ 3ce1f3f79392
//!
//! `ip_ctloutput` (the `pr_ctloutput` of UDP and, behind `rip_ctloutput` and
//! `tcp_ctloutput`, of raw IP and TCP) finds the socket's control block and lends the
//! kernel's the socket's IPv4 options, the parked mbuf this tree keeps in the pcb until the
//! transports move (N6.6).
//!
//! Status: `ported` (M7b output path, M9a socket options); the output path the kernel's
//! since N6.4a, the socket options since N6.4c1.
//!
//! ## Deviations
//! - `ip_output`'s `struct route *`, `struct ip_moptions *` and `const struct ipsec_level
//!   *seclevel` are `Option`s; the error is a `Result`. The packet (and `opt`) are
//!   `&'static Mbuf`s, handed to the kernel's `ip_output` (the packet unparked, consumed),
//!   with the multicast options' interface, TTL and loopback switch lent as the kernel's
//!   `IpMoptions`.
//! - IPsec (M9c): `ip_output_ipsec_lookup` returns the SA (`Ok(None)` for no IPsec) or the
//!   SPD's verdict (`SpdError::Drop` is the C's `-EINVAL`, a silent drop).
//! - The IP header is read and written as a copy (`ip_var.rs`'s `mtod_ip`/`mtod_ip_store`),
//!   since mbuf data has no 4-byte alignment guarantee.
//! - `NPF` (pf(4)) is configured: `ip_output_ipsec_send` runs `pf_test` on the SA's `enc(4)`
//!   interface.

use core::ptr::{self};
use core::sync::atomic::Ordering;

use crate::kern::kern_tc::gettime;
use crate::kern::subr_prf::panic;
use crate::kern::uipc_mbuf::{m_freem, ml_dequeue, ml_enqueue, ml_init, ml_purge};
use crate::kern::uipc_mbuf2::{m_tag_first, m_tag_next};
use crate::net::if_enc::enc_getif;
use crate::net::if_var::Ifnet;
use crate::net::pf::pf_test;
use crate::net::pfvar::{PF_FWD, PF_OUT, PF_PASS};
use crate::net::route::{RT_RESOLVE, RTF_HOST, Route, as_static, rtfree};
use crate::netinet::in_::InAddr;
use crate::netinet::in_pcb::InpcbLegacy as _;
use crate::netinet::in_pcb::sotoinpcb;
use crate::netinet::ip::IP_DF;
use crate::netinet::ip_icmp::icmp_mtudisc_clone;
use crate::netinet::ip_input::ip_mtudisc;
use crate::netinet::ip_ipsp::{
    IPSP_DF_INHERIT, IPSP_DIRECTION_OUT, IpsecCounters, IpsecLevel, Tdb, TdbCounters, TdbIdent,
    ipsecstat_inc, ipsp_ids_free, ipsp_ids_lookup, tdb_unref, tdbstat_inc,
};
use crate::netinet::ip_spd::{SpdError, ipsp_spd_lookup};
use crate::netinet::ip_var::{IpMoptions, mtod_ip};
use crate::netinet::ipsec_output::{ipsec_adjust_mtu, ipsp_process_packet};
use crate::netinet::tcp_output::tcp_softtso_chop;
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};
use crate::sys::endian::{htons, ntohl, ntohs};
use crate::sys::errno::Errno;
use crate::sys::mbuf::M_BCAST;
use crate::sys::mbuf::{
    M_IPV6_DF_OUT, M_MCAST, M_TCP_TSO, Mbuf, MbufList, PACKET_TAG_IPSEC_OUT_DONE, mut_of, park,
    unpark,
};
use crate::sys::socket::AF_INET;
use crate::sys::socketvar::Socket;
use crate::sys::systm::{kernel_lock, kernel_unlock};

/// `ip_output(m, opt, ro, flags, imo, seclevel, ipsecflowinfo)`: IP output, the kernel's
/// (`kernel::netinet::ip_output::ip_output`) for `bsd`'s callers: the parked packet `m`
/// handed over (consumed), the socket's multicast options lent as the kernel's view, the net
/// lock the caller holds proved through the `SaToken`.
pub fn ip_output(
    m: &'static Mbuf,
    opt: Option<&'static Mbuf>,
    ro: Option<&mut Route>,
    flags: i32,
    imo: Option<&IpMoptions>,
    seclevel: Option<&IpsecLevel>,
    ipsecflowinfo: u32,
) -> Result<(), Errno> {
    kernel::netinet::ip_output::ip_output(
        unpark(m),
        opt.map(Mbuf::data),
        ro,
        flags,
        imo,
        seclevel,
        ipsecflowinfo,
        &crate::net::if_::net_held("ip_output"),
    )
}

/// `ip_output_ipsec_lookup`: the SA the policy wants applied to `m`, with a reference;
/// `None` when no IPsec is needed (no SA, or the packet already went through this one).
pub(crate) fn ip_output_ipsec_lookup(
    m: &Mbuf,
    hlen: i32,
    seclevel: Option<&IpsecLevel>,
    ipsecflowinfo: u32,
) -> Result<Option<&'static Tdb>, SpdError> {
    let mut tdb = None;

    // Do we have any pending SAs to apply ?
    let ids = if ipsecflowinfo != 0 {
        ipsp_ids_lookup(ipsecflowinfo)
    } else {
        None
    };
    let error = ipsp_spd_lookup(
        m,
        i32::from(AF_INET),
        hlen,
        IPSP_DIRECTION_OUT,
        None,
        seclevel,
        Some(&mut tdb),
        ids,
    );
    ipsp_ids_free(ids);
    error?;
    let Some(t) = tdb else {
        return Ok(None);
    };
    // Loop detection
    let mut mtag = m_tag_first(m);
    while let Some(tag) = mtag {
        if tag.m_tag_id.get() == PACKET_TAG_IPSEC_OUT_DONE {
            // SAFETY: `IPSEC_OUT_DONE` tags carry a `struct tdb_ident`.
            let tdbi = unsafe { TdbIdent::read(tag.data()) };
            if tdbi.spi == t.tdb_spi.get()
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

/// `ip_output_ipsec_pmtu_update`: stores the SA's MTU in a host route to `dst` (cloned when
/// needed), but not for transport mode SAs.
fn ip_output_ipsec_pmtu_update(tdb: &Tdb, ro: Option<&mut Route>, dst: InAddr, rtableid: u32) {
    let mut rt_mtucloned = false;
    let tdst = tdb.tdb_dst.get();
    let transportmode = tdst.sa_family() == AF_INET && tdst.sin_addr().s_addr == dst.s_addr;

    // Find a host route to store the mtu in
    let mut rt = ro.as_deref().and_then(Route::rt).map(as_static);
    // but don't add a PMTU route for transport mode SAs
    if transportmode {
        rt = None;
    } else if rt.is_none_or(|r| r.rt_flags.get() & RTF_HOST == 0) {
        rt = icmp_mtudisc_clone(dst, rtableid, true);
        rt_mtucloned = true;
    }
    crate::ipsec_dprintf!(
        "ip_output_ipsec_pmtu_update",
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
                &crate::net::if_::net_held("ip_output_ipsec_pmtu_update"),
            );
        }
        if rt_mtucloned {
            rtfree(Some(r));
        }
    }
}

/// `ip_output_ipsec_send`: hands `m` to the SA `tdb` (after the path MTU check), as many
/// packets as TSO chopping makes. Consumes the packet.
pub(crate) fn ip_output_ipsec_send(
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
    if pf_test(AF_INET, if fwd { PF_FWD } else { PF_OUT }, e, &mut mp) != PF_PASS {
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
    let ip = mtod_ip(m);
    let len: u32 = if m.m_pkthdr().csum_flags.get() & M_TCP_TSO != 0
        && u32::from(m.m_pkthdr().ph_mss.get()) <= tdb.tdb_mtu.get()
    {
        tso = true;
        u32::from(m.m_pkthdr().ph_mss.get())
    } else {
        u32::from(ntohs(ip.ip_len))
    };

    // Check if we are allowed to fragment
    let dst = ip.ip_dst;
    if ip_mtudisc_local != 0
        && ip.ip_off & htons(IP_DF) != 0
        && tdb.tdb_mtu.get() != 0
        && len > tdb.tdb_mtu.get()
        && tdb.tdb_mtutimeout.get() > gettime() as u64
    {
        ip_output_ipsec_pmtu_update(tdb, ro, dst, rtableid);
        ipsec_adjust_mtu(m, tdb.tdb_mtu.get());
        m_freem(m);
        return Err(Errno::EMSGSIZE);
    }
    // propagate IP_DF for v4-over-v6
    if ip_mtudisc_local != 0 && ip.ip_off & htons(IP_DF) != 0 {
        m.m_pkthdr()
            .csum_flags
            .set(m.m_pkthdr().csum_flags.get() | M_IPV6_DF_OUT);
    }

    // Clear these -- they'll be set in the recursive invocation as needed.
    m.m_flags().set(m.m_flags().get() & !(M_MCAST | M_BCAST));

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
            in_proto_cksum_out(m, encif);
            ml_init(&ml);
            ml_enqueue(&ml, m);
        }

        kernel_lock();
        while let Some(m) = ml_dequeue(&ml) {
            // Callee frees mbuf
            error = ipsp_process_packet(m, tdb, i32::from(AF_INET), false, IPSP_DF_INHERIT);
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
        ip_output_ipsec_pmtu_update(tdb, ro, dst, rtableid);
    }
    error
}

/// `ip_fragment`: splits `m0` into fragments for `mtu` on `ml`, the first (trimmed) one
/// first; on failure every fragment is freed (the kernel's, over `bsd`'s list).
pub fn ip_fragment(m0: &'static Mbuf, ml: &MbufList, ifp: &Ifnet, mtu: u64) -> Result<(), Errno> {
    // `u_long mtu`: the interfaces' and the routes' MTUs are `u_int`s.
    let mtu = u32::try_from(mtu).unwrap_or(u32::MAX);
    ml.change(|l| kernel::netinet::ip_output::ip_fragment(unpark(m0), l, ifp, mtu))
}

/// `ip_insertoptions`: inserts IP options `opt` (a `struct ipoption`) into a preformed packet
/// (the kernel's: the parked packet handed over and the result parked).
pub fn ip_insertoptions(m: &'static Mbuf, opt: &Mbuf, phlen: &mut i32) -> &'static Mbuf {
    park(kernel::netinet::ip_output::ip_insertoptions(
        unpark(m),
        opt.data(),
        phlen,
    ))
}

/// `ip_ctloutput`: IP socket option processing (the kernel's, `pr_ctloutput` of UDP and,
/// behind `rip_ctloutput` and `tcp_ctloutput`, of raw IP and TCP), on the socket's control
/// block, with its IPv4 options (`inp_options`, the parked mbuf `bsd` keeps until the
/// transports move) lent for the call.
pub fn ip_ctloutput(
    op: i32,
    so: &'static Socket,
    level: i32,
    optname: i32,
    m: Option<&'static Mbuf>,
) -> Result<(), Errno> {
    let Some(inp) = sotoinpcb(so) else {
        panic(format_args!("ip_ctloutput: socket {:p} without inpcb", so));
    };
    let mut options = inp.options().map(unpark);
    inp.set_options(None);
    let r = kernel::netinet::ip_output::ip_ctloutput(
        op,
        inp,
        &mut options,
        level,
        optname,
        m.map(mut_of),
        &crate::net::if_::net_held("ip_ctloutput"),
    );
    inp.set_options(options.map(park));
    r
}

/// `in_hdr_cksum_out`: the IP header checksum, by the interface if it can or in software (the
/// kernel's, on the packet `bsd`'s caller owns).
pub fn in_hdr_cksum_out(m: &Mbuf, ifp: Option<&Ifnet>) {
    kernel::netinet::ip_output::in_hdr_cksum_out(mut_of(m), ifp);
}

/// `in_proto_cksum_out`: the transport checksum of an outgoing packet (the kernel's, on the
/// packet `bsd`'s caller owns).
pub fn in_proto_cksum_out(m: &Mbuf, ifp: Option<&Ifnet>) {
    kernel::netinet::ip_output::in_proto_cksum_out(mut_of(m), ifp);
}

/// `in_ifcap_cksum`: whether `ifp` computes checksum `ifcap` for `m` in hardware.
pub use kernel::netinet::ip_output::in_ifcap_cksum;
/* </CODE> */
