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
//! IPv4 output as `bsd`'s callers know it: `ip_output`, fragmentation, option insertion and
//! the output checksums with LZ's signatures over the kernel's (`kernel::netinet::ip_output`,
//! N6.4a); IPsec's half of `ip_output` (`ip_output_ipsec_lookup`, `ip_output_ipsec_send`,
//! `ip_output_ipsec_pmtu_update`), which the kernel reaches through the `Ipsec` bridge's legs
//! until IPsec moves (N8); and the socket option half.
//!
//! Upstream: sys/netinet/ip_output.c @ 3ce1f3f79392
//!
//! The socket option half (`ip_ctloutput`, the `pr_ctloutput` of UDP and, behind
//! `rip_ctloutput`, of raw IP) keeps a socket's IP options, header fields and multicast
//! options in its `inpcb`, until the pcb moves (N6.4c1).
//!
//! Status: `ported` (M7b output path, M9a socket options); the output path the kernel's
//! since N6.4a.
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
//! - `ip_ctloutput`'s option values are read and written unaligned in the option mbuf;
//!   `ip_pcbopts` builds the `struct ipoption` in a local buffer, large enough for what the
//!   C may write past `ipopt_list` before its final length check, and copies it into the mbuf.
//!   `ip_setmoptions`'s `malloc(M_WAITOK)` cannot fail in C: here its failure panics.
//! - `NPF` (pf(4)) is configured: `ip_output_ipsec_send` runs `pf_test` on the SA's `enc(4)`
//!   interface.

use core::cell::Cell;
use core::mem::size_of;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

use crate::kassert;
use crate::kern::kern_malloc::{free, malloc, mallocarray};
use crate::kern::kern_prot::suser;
use crate::kern::kern_tc::gettime;
use crate::kern::subr_prf::panic;
use crate::kern::uipc_mbuf::{m_freem, m_get, ml_dequeue, ml_enqueue, ml_init, ml_purge};
use crate::kern::uipc_mbuf2::{m_tag_first, m_tag_next};
use crate::machine::cpu::curproc;
use crate::net::if_::{IFF_MULTICAST, if_get, if_put, ifa_ifwithaddr};
use crate::net::if_enc::enc_getif;
use crate::net::if_var::Ifnet;
use crate::net::pf::pf_test;
use crate::net::pfvar::{PF_FWD, PF_OUT, PF_PASS};
use crate::net::route::{
    RT_RESOLVE, RTF_HOST, RTF_LOCAL, Route, as_static, rtalloc, rtfree, rtisvalid, sa_of,
};
use crate::net::rtable::rtable_l2;
use crate::netinet::in_::{
    INADDR_ANY, IP_ADD_MEMBERSHIP, IP_AUTH_LEVEL, IP_DEFAULT_MULTICAST_LOOP,
    IP_DEFAULT_MULTICAST_TTL, IP_DROP_MEMBERSHIP, IP_ESP_NETWORK_LEVEL, IP_ESP_TRANS_LEVEL,
    IP_IPCOMP_LEVEL, IP_IPDEFTTL, IP_IPSEC_LOCAL_ID, IP_IPSEC_REMOTE_ID, IP_IPSECFLOWINFO,
    IP_MAX_MEMBERSHIPS, IP_MIN_MEMBERSHIPS, IP_MINTTL, IP_MULTICAST_IF, IP_MULTICAST_LOOP,
    IP_MULTICAST_TTL, IP_OPTIONS, IP_PIPEX, IP_PORTRANGE, IP_PORTRANGE_DEFAULT, IP_PORTRANGE_HIGH,
    IP_PORTRANGE_LOW, IP_RECVDSTADDR, IP_RECVDSTPORT, IP_RECVIF, IP_RECVOPTS, IP_RECVRETOPTS,
    IP_RECVRTABLE, IP_RECVTTL, IP_RETOPTS, IP_TOS, IP_TTL, IPPROTO_IP, IPSEC_AUTH_LEVEL_DEFAULT,
    IPSEC_ESP_NETWORK_LEVEL_DEFAULT, IPSEC_ESP_TRANS_LEVEL_DEFAULT, IPSEC_IPCOMP_LEVEL_DEFAULT,
    IPSEC_LEVEL_BYPASS, IPSEC_LEVEL_UNIQUE, InAddr, IpMreq, IpMreqn, SockaddrIn, in_addmulti,
    in_delmulti, in_ifp2ia, in_multicast, sintosa,
};
use crate::netinet::in_pcb::{
    INP_HIGHPORT, INP_IPSECFLOWINFO, INP_LOWPORT, INP_RECVDSTADDR, INP_RECVDSTPORT, INP_RECVIF,
    INP_RECVOPTS, INP_RECVRETOPTS, INP_RECVRTABLE, INP_RECVTTL, in_pcbset_rtableid, sotoinpcb,
};
use crate::netinet::in_var::{InMulti, ifatoia};
use crate::netinet::ip::{
    IP_DF, IPOPT_EOL, IPOPT_LSRR, IPOPT_MINOFF, IPOPT_NOP, IPOPT_OFFSET, IPOPT_OLEN, IPOPT_OPTVAL,
    IPOPT_SSRR, MAXTTL,
};
use crate::netinet::ip_icmp::icmp_mtudisc_clone;
use crate::netinet::ip_input::{IP_DEFTTL, ip_mtudisc};
use crate::netinet::ip_ipsp::{
    IPSP_DF_INHERIT, IPSP_DIRECTION_OUT, IpsecCounters, IpsecLevel, Tdb, TdbCounters, TdbIdent,
    ipsecstat_inc, ipsp_ids_free, ipsp_ids_lookup, tdb_unref, tdbstat_inc,
};
use crate::netinet::ip_spd::{SpdError, ipsp_spd_lookup};
use crate::netinet::ip_var::{IpMoptions, Ipoption, MAX_IPOPTLEN, mtod_ip};
use crate::netinet::ipsec_output::{ipsec_adjust_mtu, ipsp_process_packet};
use crate::netinet::tcp_output::tcp_softtso_chop;
use crate::netinet::tcp_var::{TcpstatCounters, tcpstat_inc};
use crate::sys::endian::{htons, ntohl, ntohs};
use crate::sys::errno::Errno;
use crate::sys::malloc::{M_IPMOPTS, M_NOWAIT, M_WAITOK, M_ZERO};
use crate::sys::mbuf::M_BCAST;
use crate::sys::mbuf::{
    M_IPV6_DF_OUT, M_MCAST, M_TCP_TSO, MT_SOOPTS, Mbuf, MbufList, PACKET_TAG_IPSEC_OUT_DONE, mtod,
    mut_of, park, unpark,
};
use crate::sys::protosw::{PRCO_GETOPT, PRCO_SETOPT};
use crate::sys::socket::{AF_INET, SO_RTABLE};
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
    let view = imo.map(|imo| kernel::netinet::ip_var::IpMoptions {
        imo_ifidx: imo.imo_ifidx,
        imo_ttl: imo.imo_ttl,
        imo_loop: imo.imo_loop,
    });
    kernel::netinet::ip_output::ip_output(
        unpark(m),
        opt.map(Mbuf::data),
        ro,
        flags,
        view.as_ref(),
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

// ip_ctloutput, ip_pcbopts, ip_multicast_if, ip_setmoptions, ip_getmoptions: the socket
// option side, which needs the socket layer (see the module's deviations).

/// The `int` at the start of an option mbuf (read unaligned: mbuf data need not be aligned).
fn mtod_int(m: &Mbuf) -> i32 {
    // SAFETY: the callers checked `m_len` is at least `sizeof(int)`.
    unsafe { mtod::<i32>(m).read_unaligned() }
}

/// Stores `v` as an `int` option of `m`.
fn set_mtod_int(m: &Mbuf, v: i32) {
    m.m_len().set(size_of::<i32>() as u32);
    // SAFETY: an option mbuf holds `MLEN` bytes, more than an `int`.
    unsafe { mtod::<i32>(m).write_unaligned(v) };
}

/// The first `m_len` bytes of an option mbuf.
fn mtod_bytes(m: &Mbuf) -> &[u8] {
    // SAFETY: the first mbuf holds `m_len` bytes at `m_data` (an option mbuf is one mbuf),
    // read while the caller borrows it.
    unsafe { core::slice::from_raw_parts(mtod::<u8>(m), m.m_len().get() as usize) }
}

/// `ip_ctloutput`: IP socket option processing.
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
    let Some(p) = curproc() else {
        panic(format_args!("ip_ctloutput: no curproc")); // XXX
    };
    let mut error = Ok(());

    if level != IPPROTO_IP {
        return Err(Errno::EINVAL);
    }

    let rtableid = p.process().ps_rtableid.load(Ordering::Relaxed);

    match op {
        PRCO_SETOPT => match optname {
            IP_OPTIONS => return ip_pcbopts(&inp.inp_options, m),

            IP_TOS | IP_TTL | IP_MINTTL | IP_RECVOPTS | IP_RECVRETOPTS | IP_RECVDSTADDR
            | IP_RECVIF | IP_RECVTTL | IP_RECVDSTPORT | IP_RECVRTABLE | IP_IPSECFLOWINFO => {
                let Some(m) = m.filter(|m| m.m_len().get() as usize == size_of::<i32>()) else {
                    return Err(Errno::EINVAL);
                };
                let optval = mtod_int(m);
                let optset = |bit: i32| {
                    if optval != 0 {
                        inp.set_flags(bit);
                    } else {
                        inp.clear_flags(bit);
                    }
                };
                let mut ip = inp.inp_ip.get();
                match optname {
                    IP_TOS => ip.ip_tos = optval as u8,

                    IP_TTL => {
                        if optval > 0 && optval <= i32::from(MAXTTL) {
                            ip.ip_ttl = optval as u8;
                        } else if optval == -1 {
                            ip.ip_ttl = IP_DEFTTL.load(Ordering::Relaxed) as u8;
                        } else {
                            error = Err(Errno::EINVAL);
                        }
                    }

                    IP_MINTTL => {
                        if optval >= 0 && optval <= i32::from(MAXTTL) {
                            inp.inp_ip_minttl.set(optval as u8);
                        } else {
                            error = Err(Errno::EINVAL);
                        }
                    }

                    IP_RECVOPTS => optset(INP_RECVOPTS),
                    IP_RECVRETOPTS => optset(INP_RECVRETOPTS),
                    IP_RECVDSTADDR => optset(INP_RECVDSTADDR),
                    IP_RECVIF => optset(INP_RECVIF),
                    IP_RECVTTL => optset(INP_RECVTTL),
                    IP_RECVDSTPORT => optset(INP_RECVDSTPORT),
                    IP_RECVRTABLE => optset(INP_RECVRTABLE),
                    _ => optset(INP_IPSECFLOWINFO), // IP_IPSECFLOWINFO
                }
                inp.inp_ip.set(ip);
            }

            IP_MULTICAST_IF | IP_MULTICAST_TTL | IP_MULTICAST_LOOP | IP_ADD_MEMBERSHIP
            | IP_DROP_MEMBERSHIP => {
                error = ip_setmoptions(optname, &inp.inp_moptions, m, inp.inp_rtableid.get());
            }

            IP_PORTRANGE => {
                let Some(m) = m.filter(|m| m.m_len().get() as usize == size_of::<i32>()) else {
                    return Err(Errno::EINVAL);
                };
                match mtod_int(m) {
                    IP_PORTRANGE_DEFAULT => {
                        inp.clear_flags(INP_LOWPORT);
                        inp.clear_flags(INP_HIGHPORT);
                    }

                    IP_PORTRANGE_HIGH => {
                        inp.clear_flags(INP_LOWPORT);
                        inp.set_flags(INP_HIGHPORT);
                    }

                    IP_PORTRANGE_LOW => {
                        inp.clear_flags(INP_HIGHPORT);
                        inp.set_flags(INP_LOWPORT);
                    }

                    _ => error = Err(Errno::EINVAL),
                }
            }
            IP_AUTH_LEVEL | IP_ESP_TRANS_LEVEL | IP_ESP_NETWORK_LEVEL | IP_IPCOMP_LEVEL => 'level: {
                let Some(m) = m.filter(|m| m.m_len().get() as usize == size_of::<i32>()) else {
                    error = Err(Errno::EINVAL);
                    break 'level;
                };
                let optval = mtod_int(m);

                if !(IPSEC_LEVEL_BYPASS..=IPSEC_LEVEL_UNIQUE).contains(&optval) {
                    error = Err(Errno::EINVAL);
                    break 'level;
                }

                let default = match optname {
                    IP_AUTH_LEVEL => IPSEC_AUTH_LEVEL_DEFAULT,
                    IP_ESP_TRANS_LEVEL => IPSEC_ESP_TRANS_LEVEL_DEFAULT,
                    IP_ESP_NETWORK_LEVEL => IPSEC_ESP_NETWORK_LEVEL_DEFAULT,
                    _ => IPSEC_IPCOMP_LEVEL_DEFAULT, // IP_IPCOMP_LEVEL
                };
                if optval < default && suser(p).is_err() {
                    error = Err(Errno::EACCES);
                    break 'level;
                }
                let mut sl = inp.inp_seclevel.get();
                let level = optval as u8;
                match optname {
                    IP_AUTH_LEVEL => sl.sl_auth = level,
                    IP_ESP_TRANS_LEVEL => sl.sl_esp_trans = level,
                    IP_ESP_NETWORK_LEVEL => sl.sl_esp_network = level,
                    _ => sl.sl_ipcomp = level, // IP_IPCOMP_LEVEL
                }
                inp.inp_seclevel.set(sl);
            }

            IP_IPSEC_LOCAL_ID | IP_IPSEC_REMOTE_ID => error = Err(Errno::EOPNOTSUPP),
            SO_RTABLE => {
                let Some(m) = m.filter(|m| m.m_len().get() as usize >= size_of::<u32>()) else {
                    return Err(Errno::EINVAL);
                };
                let rtid = mtod_int(m) as u32;
                if inp.inp_rtableid.get() != rtid {
                    // needs privileges to switch when already set
                    if rtableid != rtid && rtableid != 0 {
                        suser(p)?;
                    }
                    error = in_pcbset_rtableid(inp, rtid);
                }
            }
            IP_PIPEX => match m.filter(|m| m.m_len().get() as usize == size_of::<i32>()) {
                Some(m) => inp.inp_pipex.set(mtod_int(m)),
                None => error = Err(Errno::EINVAL),
            },

            _ => error = Err(Errno::ENOPROTOOPT),
        },

        PRCO_GETOPT => {
            // sogetopt always hands over an mbuf to fill.
            let Some(m) = m else {
                return Err(Errno::EINVAL);
            };
            match optname {
                IP_OPTIONS | IP_RETOPTS => {
                    if let Some(opts) = inp.inp_options.get() {
                        let len = opts.m_len().get();
                        m.m_len().set(len);
                        // SAFETY: both mbufs hold `len` bytes (`ip_pcbopts` made the options
                        // at most a `struct ipoption`, smaller than `MLEN`); distinct mbufs.
                        unsafe {
                            ptr::copy_nonoverlapping(mtod::<u8>(opts), mtod::<u8>(m), len as usize)
                        };
                    } else {
                        m.m_len().set(0);
                    }
                }

                IP_TOS | IP_TTL | IP_MINTTL | IP_RECVOPTS | IP_RECVRETOPTS | IP_RECVDSTADDR
                | IP_RECVIF | IP_RECVTTL | IP_RECVDSTPORT | IP_RECVRTABLE | IP_IPSECFLOWINFO
                | IP_IPDEFTTL => {
                    let optbit = |bit: i32| i32::from(inp.has_flags(bit));
                    let optval = match optname {
                        IP_TOS => i32::from(inp.inp_ip.get().ip_tos),
                        IP_TTL => i32::from(inp.inp_ip.get().ip_ttl),
                        IP_MINTTL => i32::from(inp.inp_ip_minttl.get()),
                        IP_IPDEFTTL => IP_DEFTTL.load(Ordering::Relaxed),
                        IP_RECVOPTS => optbit(INP_RECVOPTS),
                        IP_RECVRETOPTS => optbit(INP_RECVRETOPTS),
                        IP_RECVDSTADDR => optbit(INP_RECVDSTADDR),
                        IP_RECVIF => optbit(INP_RECVIF),
                        IP_RECVTTL => optbit(INP_RECVTTL),
                        IP_RECVDSTPORT => optbit(INP_RECVDSTPORT),
                        IP_RECVRTABLE => optbit(INP_RECVRTABLE),
                        _ => optbit(INP_IPSECFLOWINFO), // IP_IPSECFLOWINFO
                    };
                    set_mtod_int(m, optval);
                }

                IP_MULTICAST_IF | IP_MULTICAST_TTL | IP_MULTICAST_LOOP | IP_ADD_MEMBERSHIP
                | IP_DROP_MEMBERSHIP => error = ip_getmoptions(optname, inp.moptions(), m),

                IP_PORTRANGE => {
                    let optval = if inp.has_flags(INP_HIGHPORT) {
                        IP_PORTRANGE_HIGH
                    } else if inp.has_flags(INP_LOWPORT) {
                        IP_PORTRANGE_LOW
                    } else {
                        0
                    };
                    set_mtod_int(m, optval);
                }

                IP_AUTH_LEVEL | IP_ESP_TRANS_LEVEL | IP_ESP_NETWORK_LEVEL | IP_IPCOMP_LEVEL => {
                    let sl = inp.inp_seclevel.get();
                    let optval = match optname {
                        IP_AUTH_LEVEL => sl.sl_auth,
                        IP_ESP_TRANS_LEVEL => sl.sl_esp_trans,
                        IP_ESP_NETWORK_LEVEL => sl.sl_esp_network,
                        _ => sl.sl_ipcomp, // IP_IPCOMP_LEVEL
                    };
                    set_mtod_int(m, i32::from(optval));
                }
                IP_IPSEC_LOCAL_ID | IP_IPSEC_REMOTE_ID => error = Err(Errno::EOPNOTSUPP),
                SO_RTABLE => set_mtod_int(m, inp.inp_rtableid.get() as i32),
                IP_PIPEX => set_mtod_int(m, inp.inp_pipex.get()),
                _ => error = Err(Errno::ENOPROTOOPT),
            }
        }

        _ => {}
    }
    error
}

/// `ip_pcbopts`: sets up IP options in the pcb for insertion in output packets: stored in an
/// mbuf in `pcbopt`, with a pseudo-option holding the first hop if source routed.
pub fn ip_pcbopts(pcbopt: &Cell<Option<&'static Mbuf>>, m: Option<&Mbuf>) -> Result<(), Errno> {
    // turn off any old options
    m_freem(pcbopt.take());
    let Some(m) = m.filter(|m| m.m_len().get() != 0) else {
        // Only turning off any previous options.
        return Ok(());
    };

    let mlen = m.m_len().get() as usize;
    if !mlen.is_multiple_of(size_of::<i32>()) || mlen > MAX_IPOPTLEN + size_of::<InAddr>() {
        return Err(Errno::EINVAL);
    }

    // Don't sleep because NET_LOCK() is hold.
    let Some(n) = m_get(M_NOWAIT, MT_SOOPTS) else {
        return Err(Errno::ENOBUFS);
    };
    // The struct ipoption, zeroed (0 = IPOPT_EOL, needed for padding): the first-hop address
    // then the options. The list is as long as the input can fill (the C writes past
    // ipopt_list into the mbuf before the final length check).
    let mut p = [0u8; size_of::<InAddr>() + MAX_IPOPTLEN + size_of::<InAddr>()];
    let (dst, list) = p.split_at_mut(size_of::<InAddr>());
    let mut nlen = size_of::<InAddr>();

    let cp_all = mtod_bytes(m);
    let mut cp = 0usize;
    let mut off = 0usize;
    let mut cnt = mlen as i32;

    let bad = |n: &Mbuf| {
        m_freem(n);
        Err(Errno::EINVAL)
    };

    while cnt > 0 {
        let opt = cp_all[cp + IPOPT_OPTVAL];

        let mut optlen = if opt == IPOPT_NOP || opt == IPOPT_EOL {
            1
        } else {
            if cnt < (IPOPT_OLEN + 1) as i32 {
                return bad(n);
            }
            let optlen = i32::from(cp_all[cp + IPOPT_OLEN]);
            if optlen < (IPOPT_OLEN + 1) as i32 || optlen > cnt {
                return bad(n);
            }
            optlen
        };
        match opt {
            IPOPT_LSRR | IPOPT_SSRR => {
                // user process specifies route as:
                //	->A->B->C->D
                // D must be our final destination (but we can't check that since we may not
                // have connected yet). A is first hop destination, which doesn't appear in
                // actual IP option, but is stored before the options.
                if optlen < i32::from(IPOPT_MINOFF) - 1 + size_of::<InAddr>() as i32 {
                    return bad(n);
                }

                // Optlen is smaller because first address is popped. Cnt and cp will be
                // adjusted a bit later to reflect this.
                optlen -= size_of::<InAddr>() as i32;
                list[off + IPOPT_OPTVAL] = opt;
                list[off + IPOPT_OLEN] = optlen as u8;

                // Move first hop before start of options.
                dst.copy_from_slice(&cp_all[cp + IPOPT_OFFSET..cp + IPOPT_OFFSET + 4]);
                cp += size_of::<InAddr>();
                cnt -= size_of::<InAddr>() as i32;
                // Then copy rest of options
                let rest = optlen as usize - IPOPT_OFFSET;
                list[off + IPOPT_OFFSET..off + IPOPT_OFFSET + rest]
                    .copy_from_slice(&cp_all[cp + IPOPT_OFFSET..cp + IPOPT_OFFSET + rest]);
            }
            _ => {
                let l = optlen as usize;
                list[off..off + l].copy_from_slice(&cp_all[cp..cp + l]);
            }
        }
        off += optlen as usize;
        cp += optlen as usize;
        cnt -= optlen;

        if opt == IPOPT_EOL {
            break;
        }
    }
    // pad options to next word, since p was zeroed just adjust off
    off = (off + size_of::<i32>() - 1) & !(size_of::<i32>() - 1);
    nlen += off;
    if nlen > size_of::<Ipoption>() {
        return bad(n);
    }
    n.m_len().set(nlen as u32);
    // SAFETY: `n` is a fresh mbuf of `MLEN` bytes, more than a `struct ipoption`.
    unsafe { ptr::copy_nonoverlapping(p.as_ptr(), mtod::<u8>(n), size_of::<Ipoption>()) };

    pcbopt.set(Some(n));
    Ok(())
}

/// `ip_multicast_if`: the interface for the request in `mreq`: its index if given, else the
/// interface of the route to the group (no local address) or of the local address.
pub fn ip_multicast_if(mreq: &IpMreqn, rtableid: u32, ifidx: &mut u32) -> Result<(), Errno> {
    // In case userland provides the imr_ifindex use this as interface. If no interface
    // address was provided, use the interface of the route to the given multicast address.
    if mreq.imr_ifindex != 0 {
        *ifidx = mreq.imr_ifindex as u32;
    } else {
        let (addr, flags) = if mreq.imr_address.s_addr == INADDR_ANY {
            (mreq.imr_multiaddr, RT_RESOLVE)
        } else {
            (mreq.imr_address, 0)
        };
        let sin = SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_addr: addr,
            ..SockaddrIn::default()
        };
        let rt = rtalloc(sa_of(&sin), flags, rtableid);
        if !rtisvalid(rt) || (flags == 0 && rt.is_some_and(|rt| rt.rt_flags.get() & RTF_LOCAL == 0))
        {
            rtfree(rt);
            return Err(Errno::EADDRNOTAVAIL);
        }
        *ifidx = rt.map_or(0, |rt| rt.rt_ifidx.get());
        rtfree(rt);
    }

    Ok(())
}

/// The `ip_mreqn` an `ip_mreq` or `ip_mreqn` option mbuf holds (a short one zero-extended).
fn mtod_mreqn(m: &Mbuf) -> IpMreqn {
    let mut mreqn = IpMreqn::default();
    let len = (m.m_len().get() as usize).min(size_of::<IpMreqn>());
    // SAFETY: the callers checked `m_len` is the size of an `ip_mreq` or an `ip_mreqn`; the
    // local is `#[repr(C)]` plain data.
    unsafe { ptr::copy_nonoverlapping(mtod::<u8>(m), ptr::from_mut(&mut mreqn).cast::<u8>(), len) };
    mreqn
}

/// Membership slot `i` of `imo` (of `imo_max_memberships` at `imo_membership`).
fn imo_slot(imo: &IpMoptions, i: usize) -> *mut Option<&'static InMulti> {
    imo.imo_membership.wrapping_add(i)
}

/// The group of membership `i` of `imo`, one of the first `imo_num_memberships` (in use).
fn imo_member(imo: &IpMoptions, i: usize) -> Option<&'static InMulti> {
    kassert!(i < usize::from(imo.imo_num_memberships));
    // SAFETY: a slot in use of the vector `ip_setmoptions` allocated.
    unsafe { *imo_slot(imo, i) }
}

/// `ip_setmoptions`: sets the IP multicast options in response to user setsockopt().
pub fn ip_setmoptions(
    optname: i32,
    imop: &Cell<Option<NonNull<IpMoptions>>>,
    m: Option<&Mbuf>,
    rtableid: u32,
) -> Result<(), Errno> {
    let mut error = Ok(());

    let imo_ptr = match imop.get() {
        Some(imo) => imo,
        None => {
            // No multicast option buffer attached to the pcb; allocate one and initialize to
            // default values.
            let (Some(imo), Some(immp)) = (
                malloc(size_of::<IpMoptions>(), M_IPMOPTS, M_WAITOK | M_ZERO),
                mallocarray(
                    usize::from(IP_MIN_MEMBERSHIPS),
                    size_of::<Option<&InMulti>>(),
                    M_IPMOPTS,
                    M_WAITOK | M_ZERO,
                ),
            ) else {
                panic(format_args!("ip_setmoptions: malloc(M_WAITOK)"));
            };
            let imo = imo.cast::<IpMoptions>();
            // SAFETY: a fresh allocation of the structure's size, written once.
            unsafe {
                imo.as_ptr().write(IpMoptions {
                    imo_membership: immp.cast::<Option<&'static InMulti>>().as_ptr(),
                    imo_ifidx: 0,
                    imo_ttl: IP_DEFAULT_MULTICAST_TTL,
                    imo_loop: IP_DEFAULT_MULTICAST_LOOP,
                    imo_num_memberships: 0,
                    imo_max_memberships: IP_MIN_MEMBERSHIPS,
                })
            };
            imop.set(Some(imo));
            imo
        }
    };
    // SAFETY: the options are the control block's own allocation, changed here and in
    // ip_freemoptions only, under the exclusive net lock the socket option calls hold.
    let imo = unsafe { &mut *imo_ptr.as_ptr() };

    'out: {
        match optname {
            IP_MULTICAST_IF => {
                // Select the interface for outgoing multicast packets.
                let Some(m) = m else {
                    error = Err(Errno::EINVAL);
                    break 'out;
                };
                let len = m.m_len().get() as usize;
                let addr = if len == size_of::<InAddr>() {
                    // SAFETY: the mbuf holds an `in_addr`.
                    unsafe { mtod::<InAddr>(m).read_unaligned() }
                } else if len == size_of::<IpMreq>() || len == size_of::<IpMreqn>() {
                    let mreqn = mtod_mreqn(m);

                    // If an interface index is given use this index to set the imo_ifidx but
                    // check first that the interface actually exists. In the other case just
                    // set the addr to the imr_address and fall through to the regular code.
                    if mreqn.imr_ifindex != 0 {
                        let ifp = if_get(mreqn.imr_ifindex as u32);
                        match ifp {
                            Some(ifp) if ifp.if_rdomain.get() == rtable_l2(rtableid) => {
                                imo.imo_ifidx = ifp.if_index.get() as u16;
                            }
                            _ => error = Err(Errno::EADDRNOTAVAIL),
                        }
                        if_put(ifp);
                        break 'out;
                    }
                    mreqn.imr_address
                } else {
                    error = Err(Errno::EINVAL);
                    break 'out;
                };
                // INADDR_ANY is used to remove a previous selection. When no interface is
                // selected, a default one is chosen every time a multicast packet is sent.
                if addr.s_addr == INADDR_ANY {
                    imo.imo_ifidx = 0;
                    break 'out;
                }
                // The selected interface is identified by its local IP address. Find the
                // interface and confirm that it supports multicasting.
                let mut sin = SockaddrIn {
                    sin_len: size_of::<SockaddrIn>() as u8,
                    sin_family: AF_INET,
                    sin_addr: addr,
                    ..SockaddrIn::default()
                };
                // SAFETY: a local `sockaddr_in`, read for the call.
                let ifa = unsafe { ifa_ifwithaddr(sintosa(&mut sin), rtableid) };
                let ifp = ifa.map(ifatoia).and_then(|ia| ia.ia_ifp().get());
                match ifp {
                    Some(ifp) if ifp.if_flags.get() & IFF_MULTICAST != 0 => {
                        imo.imo_ifidx = ifp.if_index.get() as u16;
                    }
                    _ => error = Err(Errno::EADDRNOTAVAIL),
                }
            }

            IP_MULTICAST_TTL => {
                // Set the IP time-to-live for outgoing multicast packets.
                match m.filter(|m| m.m_len().get() == 1) {
                    Some(m) => imo.imo_ttl = mtod_bytes(m)[0],
                    None => error = Err(Errno::EINVAL),
                }
            }

            IP_MULTICAST_LOOP => {
                // Set the loopback flag for outgoing multicast packets. Must be zero or one.
                match m.filter(|m| m.m_len().get() == 1).map(|m| mtod_bytes(m)[0]) {
                    Some(lp) if lp <= 1 => imo.imo_loop = lp,
                    _ => error = Err(Errno::EINVAL),
                }
            }

            IP_ADD_MEMBERSHIP => {
                // Add a multicast group membership. Group must be a valid IP multicast
                // address.
                let Some(m) = m.filter(|m| {
                    let len = m.m_len().get() as usize;
                    len == size_of::<IpMreq>() || len == size_of::<IpMreqn>()
                }) else {
                    error = Err(Errno::EINVAL);
                    break 'out;
                };
                let mreqn = mtod_mreqn(m);
                if !in_multicast(mreqn.imr_multiaddr.s_addr) {
                    error = Err(Errno::EINVAL);
                    break 'out;
                }

                let mut ifidx = 0;
                if let Err(e) = ip_multicast_if(&mreqn, rtableid, &mut ifidx) {
                    error = Err(e);
                    break 'out;
                }

                // See if we found an interface, and confirm that it supports multicast.
                let ifp = if_get(ifidx);
                let Some(ifp) = ifp.filter(|ifp| {
                    ifp.if_rdomain.get() == rtable_l2(rtableid)
                        && ifp.if_flags.get() & IFF_MULTICAST != 0
                }) else {
                    error = Err(Errno::EADDRNOTAVAIL);
                    if_put(ifp);
                    break 'out;
                };

                // See if the membership already exists or if all the membership slots are
                // full.
                let num = usize::from(imo.imo_num_memberships);
                let i = (0..num)
                    .find(|&i| {
                        imo_member(imo, i).is_some_and(|inm| {
                            inm.inm_ifidx().get() == ifidx
                                && inm.inm_addr().s_addr == mreqn.imr_multiaddr.s_addr
                        })
                    })
                    .unwrap_or(num);
                if i < num {
                    error = Err(Errno::EADDRINUSE);
                    if_put(Some(ifp));
                    break 'out;
                }
                if imo.imo_num_memberships == imo.imo_max_memberships {
                    // Resize the vector to next power-of-two minus 1. If the size would exceed
                    // the maximum then we know we've really run out of entries. Otherwise, we
                    // reallocate the vector.
                    let mut nmships = None;
                    let omships = imo.imo_membership;
                    let oldmax = usize::from(imo.imo_max_memberships);
                    let newmax = ((oldmax + 1) * 2) - 1;
                    if newmax <= usize::from(IP_MAX_MEMBERSHIPS) {
                        nmships = mallocarray(
                            newmax,
                            size_of::<Option<&InMulti>>(),
                            M_IPMOPTS,
                            M_NOWAIT | M_ZERO,
                        );
                        if let Some(n) = nmships {
                            let n = n.cast::<Option<&'static InMulti>>().as_ptr();
                            // SAFETY: the old vector has `oldmax` slots, the new one more;
                            // distinct allocations.
                            unsafe { ptr::copy_nonoverlapping(omships, n, oldmax) };
                            if let Some(o) = NonNull::new(omships) {
                                free(o.cast(), M_IPMOPTS, size_of::<Option<&InMulti>>() * oldmax);
                            }
                            imo.imo_membership = n;
                            imo.imo_max_memberships = newmax as u16;
                        }
                    }
                    if nmships.is_none() {
                        error = Err(Errno::ENOBUFS);
                        if_put(Some(ifp));
                        break 'out;
                    }
                }
                // Everything looks good; add a new record to the multicast address list for
                // the given interface.
                let Some(inm) = in_addmulti(&mreqn.imr_multiaddr, ifp) else {
                    error = Err(Errno::ENOBUFS);
                    if_put(Some(ifp));
                    break 'out;
                };
                // SAFETY: `i` (== imo_num_memberships) is below imo_max_memberships: a slot
                // of the vector.
                unsafe { *imo_slot(imo, i) = Some(inm) };
                imo.imo_num_memberships += 1;
                if_put(Some(ifp));
            }

            IP_DROP_MEMBERSHIP => {
                // Drop a multicast group membership. Group must be a valid IP multicast
                // address.
                let Some(m) = m.filter(|m| {
                    let len = m.m_len().get() as usize;
                    len == size_of::<IpMreq>() || len == size_of::<IpMreqn>()
                }) else {
                    error = Err(Errno::EINVAL);
                    break 'out;
                };
                let mreqn = mtod_mreqn(m);
                if !in_multicast(mreqn.imr_multiaddr.s_addr) {
                    error = Err(Errno::EINVAL);
                    break 'out;
                }

                // If an interface address was specified, get a pointer to its ifnet
                // structure.
                let mut ifidx = 0;
                if let Err(e) = ip_multicast_if(&mreqn, rtableid, &mut ifidx) {
                    error = Err(e);
                    break 'out;
                }

                // Find the membership in the membership array.
                let num = usize::from(imo.imo_num_memberships);
                let Some(i) = (0..num).find(|&i| {
                    imo_member(imo, i).is_some_and(|inm| {
                        (ifidx == 0 || inm.inm_ifidx().get() == ifidx)
                            && inm.inm_addr().s_addr == mreqn.imr_multiaddr.s_addr
                    })
                }) else {
                    error = Err(Errno::EADDRNOTAVAIL);
                    break 'out;
                };
                // Give up the multicast address record to which the membership points.
                if let Some(inm) = imo_member(imo, i) {
                    in_delmulti(inm);
                }
                // Remove the gap in the membership array.
                for j in i + 1..num {
                    // SAFETY: `j - 1` and `j` are slots in use.
                    unsafe { *imo_slot(imo, j - 1) = *imo_slot(imo, j) };
                }
                imo.imo_num_memberships -= 1;
            }

            _ => error = Err(Errno::EOPNOTSUPP),
        }
    }

    // If all options have default values, no need to keep the data.
    if imo.imo_ifidx == 0
        && imo.imo_ttl == IP_DEFAULT_MULTICAST_TTL
        && imo.imo_loop == IP_DEFAULT_MULTICAST_LOOP
        && imo.imo_num_memberships == 0
    {
        if let Some(mships) = NonNull::new(imo.imo_membership) {
            free(
                mships.cast(),
                M_IPMOPTS,
                usize::from(imo.imo_max_memberships) * size_of::<Option<&InMulti>>(),
            );
        }
        free(imo_ptr.cast(), M_IPMOPTS, size_of::<IpMoptions>());
        imop.set(None);
    }

    error
}

/// `ip_getmoptions`: returns the IP multicast options in response to user getsockopt().
pub fn ip_getmoptions(optname: i32, imo: Option<&IpMoptions>, m: &Mbuf) -> Result<(), Errno> {
    match optname {
        IP_MULTICAST_IF => {
            m.m_len().set(size_of::<InAddr>() as u32);
            let ifp = imo.and_then(|imo| if_get(u32::from(imo.imo_ifidx)));
            let addr = match ifp {
                None => INADDR_ANY,
                Some(ifp) => {
                    let a =
                        in_ifp2ia(ifp).map_or(INADDR_ANY, |ia| ia.ia_addr.get().sin_addr.s_addr);
                    if_put(Some(ifp));
                    a
                }
            };
            // SAFETY: an option mbuf holds `MLEN` bytes, more than an `in_addr`.
            unsafe { mtod::<InAddr>(m).write_unaligned(InAddr { s_addr: addr }) };
            Ok(())
        }

        IP_MULTICAST_TTL => {
            m.m_len().set(1);
            let ttl = imo.map_or(IP_DEFAULT_MULTICAST_TTL, |imo| imo.imo_ttl);
            // SAFETY: an option mbuf holds at least one byte.
            unsafe { mtod::<u8>(m).write(ttl) };
            Ok(())
        }

        IP_MULTICAST_LOOP => {
            m.m_len().set(1);
            let lp = imo.map_or(IP_DEFAULT_MULTICAST_LOOP, |imo| imo.imo_loop);
            // SAFETY: as above.
            unsafe { mtod::<u8>(m).write(lp) };
            Ok(())
        }

        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `ip_freemoptions`: frees a set of multicast options and leaves its groups.
///
/// # Safety
///
/// `imo` was allocated by `ip_setmoptions` (`malloc(M_IPMOPTS)`), with
/// `imo_max_memberships` slots at `imo_membership`, and is not used afterwards.
pub unsafe fn ip_freemoptions(imo: Option<NonNull<IpMoptions>>) {
    let Some(imo) = imo else {
        return;
    };
    // SAFETY: the caller's contract.
    let o = unsafe { imo.as_ref() };
    for i in 0..usize::from(o.imo_num_memberships) {
        // SAFETY: the first `imo_num_memberships` slots hold joined groups.
        if let Some(inm) = unsafe { *o.imo_membership.add(i) } {
            crate::netinet::in_::in_delmulti(inm);
        }
    }
    if let Some(m) = NonNull::new(o.imo_membership) {
        free(
            m.cast(),
            M_IPMOPTS,
            usize::from(o.imo_max_memberships)
                * size_of::<Option<&crate::netinet::in_var::InMulti>>(),
        );
    }
    free(imo.cast(), M_IPMOPTS, size_of::<IpMoptions>());
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

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the IP socket options (the output half's are the kernel's since N6.4a).

    use super::*;
    use crate::kern::uipc_mbuf::m_freem;
    use crate::net::if_::tests::setup_net;
    use crate::netinet::ip::{IPOPT_LSRR, IPOPT_RR};
    use crate::sys::mbuf::M_DONTWAIT;

    /// An option mbuf holding `opts`.
    fn optm(opts: &[u8]) -> &'static Mbuf {
        let m = m_get(M_DONTWAIT, MT_SOOPTS).expect("mbuf");
        m.m_len().set(opts.len() as u32);
        // SAFETY: a fresh mbuf of `MLEN` bytes, more than any option list here.
        unsafe { ptr::copy_nonoverlapping(opts.as_ptr(), mtod::<u8>(m), opts.len()) };
        m
    }

    #[test]
    fn ip_pcbopts_pops_the_first_hop_of_a_source_route() {
        let _g = setup_net();
        let pcbopt = Cell::new(None);

        // NOP, then LSRR through 10.0.0.1 (first hop) and 10.0.0.2 as the user writes it (the
        // addresses where the pointer byte goes), then EOL: the first hop moves before the
        // options and the route shrinks by one address.
        let m = optm(&[
            IPOPT_NOP, IPOPT_LSRR, 10, 10, 0, 0, 1, 10, 0, 0, 2, IPOPT_EOL,
        ]);
        ip_pcbopts(&pcbopt, Some(m)).expect("options");
        let n = pcbopt.get().expect("stored");
        assert_eq!(
            mtod_bytes(n),
            &[
                10, 0, 0, 1, IPOPT_NOP, IPOPT_LSRR, 6, 10, 0, 0, 2, IPOPT_EOL
            ]
        );

        // A length that is not a multiple of 4, or an option longer than the list, is refused
        // (and the old options are gone either way).
        assert_eq!(
            ip_pcbopts(&pcbopt, Some(optm(&[IPOPT_NOP; 3]))),
            Err(Errno::EINVAL)
        );
        assert!(pcbopt.get().is_none());
        assert_eq!(
            ip_pcbopts(&pcbopt, Some(optm(&[IPOPT_RR, 9, 4, 0]))),
            Err(Errno::EINVAL)
        );
        // No mbuf only turns the options off.
        ip_pcbopts(&pcbopt, Some(m)).expect("options");
        ip_pcbopts(&pcbopt, None).expect("off");
        assert!(pcbopt.get().is_none());
        m_freem(m);
    }

    #[test]
    fn ip_getmoptions_answers_the_defaults_without_options() {
        let _g = setup_net();
        let m = optm(&[0; 4]);
        ip_getmoptions(IP_MULTICAST_TTL, None, m).expect("ttl");
        assert_eq!(mtod_bytes(m), &[IP_DEFAULT_MULTICAST_TTL]);
        ip_getmoptions(IP_MULTICAST_LOOP, None, m).expect("loop");
        assert_eq!(mtod_bytes(m), &[IP_DEFAULT_MULTICAST_LOOP]);
        ip_getmoptions(IP_MULTICAST_IF, None, m).expect("if");
        assert_eq!(mtod_bytes(m), &[0, 0, 0, 0]);
        assert_eq!(ip_getmoptions(IP_TTL, None, m), Err(Errno::EOPNOTSUPP));
        m_freem(m);
    }
}
/* </TESTS> */
