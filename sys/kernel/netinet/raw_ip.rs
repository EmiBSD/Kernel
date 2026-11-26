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
//! Raw IP sockets: `netinet/raw_ip.c`. A `SOCK_RAW` socket of the inet domain (ping(8)'s
//! `IPPROTO_ICMP` one, or any protocol number) has a control block in `rawcbtable`.
//! `rip_input`, the input of every protocol without a handler of its own and the end of
//! `icmp_input` and `igmp_input`, hands a copy of each datagram, IP header included, to every
//! raw socket whose protocol and addresses match; `rip_output` prepends an IP header from
//! the control block's prototype (or, with `IP_HDRINCL`, checks the one the user wrote,
//! [`rip_chkhdr`], which divert(4) reuses) and sends the datagram with `ip_output`.
//!
//! Upstream: sys/netinet/raw_ip.c @ 3ce1f3f79392
//! LZ: sys/netinet/raw_ip.rs@3c62ede68802
//!
//! Locks: the requests run with the socket locked (a witness of `so_lock` under the net lock,
//! [`SoLocked::inet`]) and borrow the pcb from the socket's locked slot
//! (`sotoinpcb_locked`); the addresses are `[t]`, written under `rawcbtable`'s mutex. Input
//! holds no socket lock, as the C: the walk reads the `[I]`, `[t]` and atomic members of each
//! pcb under the table's mutex and appends under the receive buffer's.
//!
//! ## Deviations
//! - `MROUTING` is not configured: the `MRT_*` options (`sysdefs::netinet::ip_mroute`)
//!   answer `EOPNOTSUPP`, the C's `#else`, and `rip_detach` has no `ip_mrouter_done`.
//! - `rip_sendspace`/`rip_recvspace` (`u_long`s no sysctl names) are the constants
//!   [`RIP_SENDSPACE`] and [`RIP_RECVSPACE`].
//! - `rip_send` without a packet (which `sosend` never does: it hands every request one,
//!   empty or not) fails with `EINVAL` (LZ's).
//! - `ipcounters`' `counters_enter` pair (`ips_noproto` up, `ips_delivered` down) is two
//!   counter updates.
//! - `NPF` is configured (the divert-to key, `pf_mbuf_link_inpcb`); so is `IPSEC`:
//!   `rip_output` passes the socket's `inp_seclevel` to `ip_output`.
//!
//! ## Redesign
//! - N6.6b1: LZ's raw IP was `bsd`'s, over `&'static` sockets and pcbs, parked mbufs and the
//!   `bsd` adapters that lent the net lock's proof per call (`ip_output`, `in_pcbselsrc`,
//!   `in_broadcast`). Here it is the kernel's, generic over the glue: the requests are
//!   [`RipUsrreqs`] (`rip_usrreqs`), each borrowing the pcb from the socket's locked slot,
//!   no count per request (N6.6a5's idiom, docs/IDIOMS.md; the socket lock opened by
//!   `in_pcb`'s shared `inet`); attach fills the slot with
//!   `in_pcballoc`'s handle and detach takes it out for `in_pcbdetach`. The proof of the
//!   net lock comes with the socket lock's parts ([`InetLocked`]) on requests and with the
//!   caller on input. `rawcbtable` is the glue's ([`rawcbtable`], `NetTables`).
//! - Input walks the table with `in_pcb_iterator` (one step and a count pair per pcb, an
//!   `m_copym` per extra match, the mutex let go around each append: the C's); the
//!   prototype's protocol is one word's load ([`IpProto::ip_p`]). The headers are read and
//!   written through `hal::mem::pod` (`ip_load`, `ip_store`), never a pointer kept across an mbuf
//!   call.
//! - [`rip_chkhdr`] reads the options as the bytes of the pulled-up header, each length
//!   checked before the byte it covers is read; it answers the packet or `None` (freed).
//! - `rip_output` takes one loan of the pcb's IPv4 options for the whole send (the
//!   `IP_HDRINCL` refusal's test and `ip_output`'s argument), where LZ took two.
//! - LZ's `inpcb_of` (a `sotoinpcb` that panics) goes: a request without a pcb is
//!   `panic(9)` where the C dereferences NULL, `rip_detach` answers `EINVAL` as the C.
//! - Cost against the C (measured at the N6 close by the `rip.*` bench rows, decision 50):
//!   a send pays the pcb's options and route loans (two atomics each, D-10 of N6.6b, N12)
//!   and the multicast options' copy (`moptions_out`), where the C reads the members under
//!   the socket lock, and a connected socket's packet a dynamic call to pf's
//!   `mbuf_link_inpcb_of`; input pays the header's copy (`ip_load`, 20 bytes) and a dynamic
//!   call to pf's `find_divert` where the C reads through a pointer and calls pf directly
//!   (pf moves at N8); the walk, its count pair per pcb and the `m_copym` per extra match are
//!   the C's. Since N6.6b1 no lend of the net lock is left on these paths, but on IPsec's
//!   tail to the raw sockets (`bsd`'s `ipsec_input_disabled`, one per packet, N8).

use core::marker::PhantomData;
use core::mem::size_of;
use core::pin::pin;

use hal::mem::pool::PoolArc;
use hal::sync::netlock::NetHeld;
use hal::user::bytes_of;
use sysdefs::base::errno::Errno;
use sysdefs::base::pod::AbiPod;
use sysdefs::net::socket::{AF_INET, SO_BINDANY, SO_TIMESTAMP};
use sysdefs::netinet::in_::{
    INADDR_ANY, INADDR_BROADCAST, IP_HDRINCL, IPPROTO_DONE, IPPROTO_ICMP, IPPROTO_IP, IPPROTO_MAX,
    InAddr, SockaddrIn,
};
use sysdefs::netinet::ip::{
    IP_MAXPACKET, IPOPT_EOL, IPOPT_NOP, IPOPT_OLEN, IPOPT_OPTVAL, IPVERSION, Ip, MAXTTL,
};
use sysdefs::netinet::ip_icmp::{ICMP_UNREACH, ICMP_UNREACH_PROTOCOL};
use sysdefs::netinet::ip_mroute::{
    MRT_ADD_MFC, MRT_ADD_VIF, MRT_API_CONFIG, MRT_API_SUPPORT, MRT_ASSERT, MRT_DEL_MFC,
    MRT_DEL_VIF, MRT_DONE, MRT_INIT, MRT_VERSION,
};

use crate::assert::{kassert, kpanic};
use crate::net::domain::net;
use crate::net::if_::ifa_ifwithaddr;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::mbuf::{
    M_COPYALL, M_DONTWAIT, Mbuf, MbufBox, PF_TAG_DIVERTED, m_copym, m_prepend, m_pullup,
};
use crate::net::pf_hooks::{PF_DIVERT_REPLY, PF_DIVERT_TO, pf};
use crate::net::proto::{PRCO_SETOPT, Usrreqs};
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::{SS_CANTRCVMORE, SS_ISCONNECTED, SS_PRIV};
use crate::net::socket::{
    SoLocked, Socket, socantsendmore, soisconnected, soisdisconnected, soreserve, sorwakeup,
};
use crate::netinet::in_::{in_broadcast, in_control, in_nam2sin};
use crate::netinet::in_pcb::{
    INP_CONTROLOPTS, INP_HDRINCL, INP_IPV6, Inpcb, InpcbRef, Inpcbtable, detach_slot,
    in_pcb_iterator, in_pcballoc, in_pcbinit, in_pcbselsrc, in_peeraddr, in_sockaddr, inet, pcb_of,
    set_nam,
};
use crate::netinet::ip_icmp::icmp_error;
use crate::netinet::ip_id::ip_randomid;
use crate::netinet::ip_input::ip_savecontrol;
use crate::netinet::ip_output::{ip_ctloutput, ip_load, ip_output, ip_store};
use crate::netinet::ip_var::{
    IP_ALLOWBROADCAST, IP_RAWOUTPUT, IpstatCounters, ipstat_dec, ipstat_inc,
};
use crate::proc::{ProcGlue, Thread};

/// `RIPSNDQ`: nominal space allocated to a raw ip socket.
const RIPSNDQ: u64 = 8192;
/// `RIPRCVQ`.
const RIPRCVQ: u64 = 8192;

/// `sizeof(struct ip)`.
const IP_HDR_LEN: usize = size_of::<Ip>();

/// `rip_sendspace`.
pub const RIP_SENDSPACE: u64 = RIPSNDQ;
/// `rip_recvspace`.
pub const RIP_RECVSPACE: u64 = RIPRCVQ;

/// `rip_usrreqs`: raw interface to IP protocol (the raw entries', ICMP's, IGMP's, IPsec's and
/// pfsync's user requests).
pub struct RipUsrreqs<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> RipUsrreqs<G> {
    /// `rip_usrreqs`.
    pub const NEW: Self = Self(PhantomData);
}

/// `rawcbtable`: the glue's raw IPv4 control blocks (`NetTables`; `kern.file`'s walk reads
/// it).
pub fn rawcbtable<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().rawcbtable
}

/// The bytes of a `sockaddr_in` (or any other ABI image) the caller built.
#[inline]
pub(crate) fn sa_bytes<T: AbiPod>(sa: &T) -> &[u8] {
    bytes_of(core::slice::from_ref(sa))
}

/// `rip_init`: initialize raw connection block q.
pub fn rip_init<G: ProcGlue>() {
    in_pcbinit(rawcbtable::<G>(), 1);
}

/// `rip_input(mp, offp, proto, af, ns)`: hands a copy of datagram `*mp` to every raw socket
/// that matches it, under the net lock (`net`); without one, an ICMP message is dropped and
/// any other protocol is answered with a protocol unreachable.
#[allow(
    clippy::too_many_arguments,
    reason = "pr_input's arguments and the net lock's proof"
)]
pub fn rip_input<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    _offp: &mut i32,
    _proto: i32,
    af: i32,
    _ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(mut m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let ip = ip_load(&m);
    let table = rawcbtable::<G>();
    let iter = pin!(Inpcb::<G>::marker());

    kassert!(af == i32::from(AF_INET));

    let ripsrc = SockaddrIn {
        sin_family: AF_INET,
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_addr: ip.ip_src,
        ..SockaddrIn::default()
    };

    let mut key = ip.ip_dst;
    if m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0 {
        let divert = pf().and_then(|p| p.find_divert(&m));
        kassert!(divert.is_some());
        if let Some(divert) = divert {
            match divert.type_ {
                PF_DIVERT_TO => {
                    let mut a = [0u8; 4];
                    a.copy_from_slice(&divert.addr[..4]);
                    key = InAddr {
                        s_addr: u32::from_ne_bytes(a),
                    };
                }
                PF_DIVERT_REPLY => {}
                t => kpanic(format_args!(
                    "rip_input: unknown divert type {t}, mbuf {:p}",
                    &*m
                )),
            }
        }
    }

    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());
    let mut g = table.lock();
    let mut inp: Option<InpcbRef<G>> = None;
    let mut last: Option<InpcbRef<G>> = None;
    while let Some(i) = in_pcb_iterator(table, &mut g, inp.take(), iter.as_ref()) {
        kassert!(!i.has_flags(INP_IPV6));
        let so = i.socket();

        // Packet must not be inserted after disconnected wakeup call. To avoid race, check
        // again when holding receive buffer mutex.
        let skip = so.so_rcv.has_state(SS_CANTRCVMORE)
            || rtable_l2(i.inp_rtableid.get()) != rdomain
            || {
                let p = i.inp_ip.ip_p();
                p != 0 && p != ip.ip_p
            }
            || {
                let l = i.inp_laddr.get().s_addr;
                l != 0 && l != key.s_addr
            }
            || {
                let f = i.inp_faddr.get().s_addr;
                f != 0 && f != ip.ip_src.s_addr
            };
        if skip {
            inp = Some(i);
            continue;
        }

        if let Some(l) = last.take() {
            drop(g);

            if let Some(n) = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT) {
                rip_sbappend(&l, n, &ip, &ripsrc);
            }
            drop(l); // in_pcbunref(last)

            g = table.lock();
        }
        last = Some(i.clone()); // in_pcbref(inp)
        inp = Some(i);
    }
    drop(g);

    let Some(last) = last else {
        if i32::from(ip.ip_p) == IPPROTO_ICMP {
            drop(m);
        } else {
            icmp_error(m, ICMP_UNREACH, ICMP_UNREACH_PROTOCOL, 0, 0, net);
        }
        ipstat_inc(IpstatCounters::IpsNoproto);
        ipstat_dec(IpstatCounters::IpsDelivered);

        return IPPROTO_DONE;
    };

    rip_sbappend(&last, m, &ip, &ripsrc);
    drop(last); // in_pcbunref(last)

    IPPROTO_DONE
}

/// `rip_sbappend`: queues datagram `m` (whose header is `ip`) from `ripsrc`, with the control
/// messages its socket asked for, on the receive buffer of `inp`'s socket; the mutex is let
/// go before a refused datagram is freed.
fn rip_sbappend<G: ProcGlue>(inp: &Inpcb<G>, m: MbufBox, ip: &Ip, ripsrc: &SockaddrIn) {
    let so = inp.socket();
    let opts = if inp.has_flags(INP_CONTROLOPTS) || so.has_options(SO_TIMESTAMP) {
        ip_savecontrol(inp, ip, &m)
    } else {
        None
    };

    let mut rg = so.so_rcv.lock();
    let r = if so.so_rcv.has_state(SS_CANTRCVMORE) {
        Err((Some(m), opts))
    } else {
        rg.sbappendaddr(sa_bytes(ripsrc), Some(m), opts)
    };
    drop(rg);

    match r {
        Ok(()) => sorwakeup(so),
        Err((m, opts)) => {
            drop(m);
            drop(opts);
            ipstat_inc(IpstatCounters::IpsNoproto);
        }
    }
}

/// `rip_output(m, so, dst, control)`: generates an IP header and passes the packet to
/// `ip_output`, with the options the user may have set up with control calls; `dst` is the
/// destination, which an `IP_HDRINCL` header overrides. `inp` is the socket's pcb, borrowed
/// under its lock; `net` the net lock's proof.
pub fn rip_output<G: ProcGlue>(
    m: MbufBox,
    inp: &Inpcb<G>,
    dst: &mut SockaddrIn,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut flags = IP_ALLOWBROADCAST;
    // inp->inp_options, lent once for the whole send.
    let opts = inp.options();

    // If the user handed us a complete IP packet, use it. Otherwise, allocate an mbuf for a
    // header and fill it in.
    let (mut m, mut ip) = if inp.has_flags(INP_HDRINCL) {
        if m.m_pkthdr().len.get() as usize > IP_MAXPACKET {
            return Err(Errno::EMSGSIZE);
        }

        let Some(m) = rip_chkhdr(m, opts.is_some()) else {
            return Err(Errno::EINVAL);
        };

        let mut ip = ip_load(&m);
        if ip.ip_id == 0 {
            ip.ip_id = ip_randomid().to_be();
        }
        dst.sin_addr = ip.ip_dst;

        // XXX prevent ip_output from overwriting header fields
        flags |= IP_RAWOUTPUT;
        ipstat_inc(IpstatCounters::IpsRawout);
        (m, ip)
    } else {
        if m.m_pkthdr().len.get() as usize + IP_HDR_LEN > IP_MAXPACKET {
            return Err(Errno::EMSGSIZE);
        }
        let Some(m) = m_prepend(m, IP_HDR_LEN as i32, M_DONTWAIT) else {
            return Err(Errno::ENOBUFS);
        };
        let proto = inp.inp_ip.get();
        let ip = Ip {
            ip_tos: proto.ip_tos,
            ip_off: 0u16.to_be(),
            ip_p: proto.ip_p,
            ip_len: (m.m_pkthdr().len.get() as u16).to_be(),
            ip_src: InAddr { s_addr: INADDR_ANY },
            ip_dst: dst.sin_addr,
            ip_ttl: if proto.ip_ttl != 0 {
                proto.ip_ttl
            } else {
                MAXTTL
            },
            ..ip_load(&m)
        };
        (m, ip)
    };

    if ip.ip_src.s_addr == INADDR_ANY {
        ip.ip_src = in_pcbselsrc(dst, inp, net)?;
    }
    if !ip_store(&mut m, &ip) {
        return Err(Errno::ENOBUFS);
    }

    // A thought: Even though raw IP shouldn't be able to set IPv6 multicast options, if it
    // does, the last parameter to ip_output should be guarded against v6/v4 problems.

    // force routing table
    m.m_pkthdr().ph_rtableid.set(inp.inp_rtableid.get());

    if inp.socket().has_state(SS_ISCONNECTED)
        && i32::from(ip.ip_p) != IPPROTO_ICMP
        && let Some(pf) = pf()
    {
        let a: &(dyn core::any::Any + Sync) = inp;
        pf.mbuf_link_inpcb_of(&m, a);
    }

    let seclevel = inp.inp_seclevel.get();
    let imo = inp.moptions_out();
    let mut ro = inp.route();
    ip_output(
        m,
        opts.as_deref().map(Mbuf::data),
        Some(&mut *ro),
        flags,
        imo.as_ref(),
        Some(&seclevel),
        0,
        net,
    )
}

/// `rip_chkhdr(m, options)`: checks the IP header (and its options) a user handed over with
/// `IP_HDRINCL` (or divert(4) reinjects): a whole header whose length the packet's matches,
/// version 4, and, with options, no socket options beside them (`options`) and each option's
/// length within the header. The packet, its header pulled up, or `None` once freed.
pub fn rip_chkhdr(m: MbufBox, options: bool) -> Option<MbufBox> {
    if (m.m_pkthdr().len.get() as usize) < IP_HDR_LEN {
        return None;
    }

    let m = m_pullup(m, IP_HDR_LEN as i32)?;

    let ip = ip_load(&m);
    let hlen = usize::from(ip.ip_hl()) << 2;
    let ip_len = usize::from(u16::from_be(ip.ip_len));

    // Don't allow packet length sizes that will crash.
    if hlen < IP_HDR_LEN
        || ip_len < hlen
        || i64::try_from(ip_len).ok() != Some(i64::from(m.m_pkthdr().len.get()))
    {
        return None;
    }
    let m = m_pullup(m, hlen as i32)?;

    if ip_load(&m).ip_v() != IPVERSION {
        return None;
    }

    // Don't allow both user specified and setsockopt options. If options are present verify
    // them.
    if hlen != IP_HDR_LEN {
        if options {
            return None;
        }
        let opts = m.data().get(IP_HDR_LEN..hlen)?;
        let mut cp = 0usize;
        let mut cnt = opts.len();
        while cnt > 0 {
            let opt = *opts.get(cp + IPOPT_OPTVAL)?;
            if opt == IPOPT_EOL {
                break;
            }
            let optlen = if opt == IPOPT_NOP {
                1
            } else {
                if cnt < IPOPT_OLEN + 1 {
                    return None;
                }
                let optlen = usize::from(*opts.get(cp + IPOPT_OLEN)?);
                if optlen < IPOPT_OLEN + 1 || optlen > cnt {
                    return None;
                }
                optlen
            };
            cnt -= optlen;
            cp += optlen;
        }
    }

    Some(m)
}

/// The `int` an option mbuf carries (`*mtod(m, int *)`); `None` for a missing or short one
/// (the C's `m == NULL || m->m_len < sizeof(int)`).
pub(crate) fn opt_int(m: Option<&Mbuf>) -> Option<i32> {
    let d = m?.data();
    let b: [u8; size_of::<i32>()] = d.get(..size_of::<i32>())?.try_into().ok()?;
    Some(i32::from_ne_bytes(b))
}

/// `rip_ctloutput(op, so, level, optname, m)`: raw IP socket option processing: `IP_HDRINCL`
/// (set from an `int`, read back as the flag's bit, the C's `inp_flags & INP_HDRINCL`), the
/// multicast routing options (`EOPNOTSUPP` without `MROUTING`), then `ip_ctloutput`; another
/// level than `IPPROTO_IP` is `EINVAL`.
pub fn rip_ctloutput<G: ProcGlue>(
    op: i32,
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip_ctloutput");

    if level != IPPROTO_IP {
        return Err(Errno::EINVAL);
    }

    match optname {
        IP_HDRINCL => {
            if op == PRCO_SETOPT {
                match opt_int(m.as_deref()) {
                    None => return Err(Errno::EINVAL),
                    Some(0) => inp.clear_flags(INP_HDRINCL),
                    Some(_) => inp.set_flags(INP_HDRINCL),
                }
            } else {
                match m {
                    Some(m) => set_nam(m, &(inp.inp_flags.get() & INP_HDRINCL)),
                    None => kpanic(format_args!("rip_ctloutput: no mbuf for PRCO_GETOPT")),
                }
            }
            Ok(())
        }

        MRT_INIT | MRT_DONE | MRT_ADD_VIF | MRT_DEL_VIF | MRT_ADD_MFC | MRT_DEL_MFC
        | MRT_VERSION | MRT_ASSERT | MRT_API_SUPPORT | MRT_API_CONFIG => {
            // MROUTING: ip_mrouter_set / ip_mrouter_get; not configured.
            Err(Errno::EOPNOTSUPP)
        }

        _ => {
            let mut options = inp.options();
            ip_ctloutput(op, inp, &mut options, level, optname, m, &il.net)
        }
    }
}

/// `rip_attach(so, proto, wait)`: a control block for a privileged raw socket of protocol
/// `proto` (`EACCES` unprivileged, `EPROTONOSUPPORT` outside `[0, IPPROTO_MAX)`), its buffers
/// reserved, its handle in the socket's locked slot.
pub fn rip_attach<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    proto: i32,
    wait: i32,
) -> Result<(), Errno> {
    if so.has_pcb() {
        kpanic(format_args!("rip_attach"));
    }
    if !so.has_state(SS_PRIV) {
        return Err(Errno::EACCES);
    }
    if !(0..IPPROTO_MAX).contains(&proto) {
        return Err(Errno::EPROTONOSUPPORT);
    }

    soreserve(so, RIP_SENDSPACE, RIP_RECVSPACE)?;
    let inp = in_pcballoc(so, rawcbtable::<G>(), wait)?;
    let mut ip = inp.inp_ip.get();
    ip.ip_p = proto as u8;
    inp.inp_ip.set(ip);
    // so->so_pcb for the requests: in_pcballoc's handle, kept by the locked slot until detach.
    let mut il = inet(w);
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// `rip_detach(so)`: the control block detached; `EINVAL` without one.
pub fn rip_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);

    // MROUTING: ip_mrouter_done(so); not configured.
    if !detach_slot(so, &mut il) {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `rip_bind(so, nam, p)`: a local address for `so`: one of ours, a broadcast or the
/// wildcard one, or any with `SO_BINDANY` (`EADDRNOTAVAIL` otherwise).
pub fn rip_bind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip_bind");

    let addr = in_nam2sin(nam)?;
    let rtableid = inp.inp_rtableid.get();

    if !(so.has_options(SO_BINDANY)
        || addr.sin_addr.s_addr == INADDR_ANY
        || addr.sin_addr.s_addr == INADDR_BROADCAST
        || in_broadcast(addr.sin_addr, rtableid, &il.net)
        || ifa_ifwithaddr(&il.net, &il.net.reader(), sa_bytes(&addr), rtableid).is_some())
    {
        return Err(Errno::EADDRNOTAVAIL);
    }

    let g = inp.table().lock();
    inp.set_laddr(&g, addr.sin_addr);
    drop(g);

    Ok(())
}

/// `rip_connect(so, nam)`: the foreign address of `so`.
pub fn rip_connect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    nam: &Mbuf,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip_connect");

    let addr = in_nam2sin(nam)?;

    let g = inp.table().lock();
    inp.set_faddr(&g, addr.sin_addr);
    drop(g);
    soisconnected(so);

    Ok(())
}

/// `rip_disconnect(so)`: `ENOTCONN` unless connected; the foreign address back to the
/// wildcard.
pub fn rip_disconnect<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip_disconnect");

    if !so.has_state(SS_ISCONNECTED) {
        return Err(Errno::ENOTCONN);
    }

    soisdisconnected(so);
    let g = inp.table().lock();
    inp.set_faddr(&g, InAddr { s_addr: INADDR_ANY });
    drop(g);

    Ok(())
}

/// `rip_shutdown(so)`: marks the connection as being incapable of further input.
pub fn rip_shutdown<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let _il = inet(w);
    socantsendmore(so);

    Ok(())
}

/// `rip_send(so, m, nam, control)`: ships a packet out to the connected address
/// (`EISCONN` with an address) or to `nam` (`ENOTCONN` without one); `rip_output` handles any
/// massaging necessary. The data and the control are disposed of here.
pub fn rip_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    nam: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = pcb_of(so, &il.pcb, "rip_send");

    let mut dst = SockaddrIn {
        sin_family: AF_INET,
        sin_len: size_of::<SockaddrIn>() as u8,
        ..SockaddrIn::default()
    };
    if so.has_state(SS_ISCONNECTED) {
        if nam.is_some() {
            return Err(Errno::EISCONN);
        }
        dst.sin_addr = inp.inp_faddr.get();
    } else {
        let Some(nam) = nam else {
            return Err(Errno::ENOTCONN);
        };
        dst.sin_addr = in_nam2sin(nam)?.sin_addr;
    }
    // XXX Find an IPsec TDB
    // sosend always hands over a packet (a pkthdr mbuf, empty or not).
    let Some(m) = m else {
        return Err(Errno::EINVAL);
    };
    let error = rip_output(m, inp, &mut dst, &il.net);
    drop(control);
    error
}

impl<G: ProcGlue> Usrreqs<G> for RipUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        rip_attach(so, w, proto, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        rip_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        rip_bind(so, w, nam, p)
    }
    fn connect(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        rip_connect(so, w, nam)
    }
    fn disconnect(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        rip_disconnect(so, w)
    }
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        rip_shutdown(so, w)
    }
    /// No `pru_rcvd` (the entries are not `PR_WANTRCVD`): never asked.
    fn rcvd(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_rcvd of raw IP"));
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        rip_send(so, w, top, addr, control)
    }
    /// No `pru_abort` (a raw socket is never on a listen queue): never asked.
    fn abort(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("pru_abort of raw IP"));
    }
    /// `in_control`.
    fn control(
        &self,
        so: &Socket<G>,
        cmd: u64,
        data: &mut [u8],
        ifp: Option<&'static Ifnet>,
    ) -> Result<(), Errno> {
        in_control(so.has_state(SS_PRIV), cmd, data, ifp)
    }
    /// `in_sockaddr`.
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        in_sockaddr(so, w, addr)
    }
    /// `in_peeraddr`.
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        in_peeraddr(so, w, addr)
    }
}

/// The host tests' raw and divert sockets (N6.6b1): protocols of `testinet`'s domains whose
/// requests are the kernel's (`RipUsrreqs`, `Rip6Usrreqs`, `DivertUsrreqs`,
/// `Divert6Usrreqs`), the four tables initialised, sockets made by `soalloc` and attached by
/// the real request, packets and names built, records read back.
#[cfg(test)]
pub(crate) mod testraw {
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::sync::netlock::NETLOCK;
    use sysdefs::net::socket::{AF_INET6, SOCK_RAW};
    use sysdefs::netinet6::in6::In6Addr;

    use super::*;
    use crate::net::domain::Domain;
    use crate::net::mbuf::{M_WAIT, MT_CONTROL, MT_SONAME, m_devget, m_get};
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, Protocol};
    use crate::net::sockbuf::SS_NOFDREF;
    use crate::net::socket::{SoInit, SocketRef, soalloc, solock};
    use crate::netinet::in_pcb::{sotoinpcb, testinet};

    /// The fixture's glue.
    pub(crate) type G = testinet::G;

    /// The family and requests of a [`TestProto`].
    #[derive(Clone, Copy)]
    pub(crate) enum Kind {
        /// Raw IP.
        Raw,
        /// Raw IPv6.
        Raw6,
        /// divert(4).
        Divert,
        /// IPv6 divert.
        Divert6,
    }

    /// A raw protocol of the test domains with the kernel's requests and options.
    pub(crate) struct TestProto {
        kind: Kind,
        proto: i16,
    }

    /// Raw IP's `IPPROTO_RAW` row.
    pub(crate) static RAW: TestProto = TestProto {
        kind: Kind::Raw,
        proto: 255,
    };
    /// Raw IPv6's `IPPROTO_RAW` row.
    pub(crate) static RAW6: TestProto = TestProto {
        kind: Kind::Raw6,
        proto: 255,
    };
    /// ICMPv6's row (raw IPv6's requests, `pr_protocol` `IPPROTO_ICMPV6`).
    pub(crate) static ICMP6: TestProto = TestProto {
        kind: Kind::Raw6,
        proto: 58,
    };
    /// divert(4)'s row.
    pub(crate) static DIVERT: TestProto = TestProto {
        kind: Kind::Divert,
        proto: 258,
    };
    /// IPv6 divert's row.
    pub(crate) static DIVERT6: TestProto = TestProto {
        kind: Kind::Divert6,
        proto: 258,
    };

    impl Protocol<G> for TestProto {
        fn domain(&self) -> &'static dyn Domain<G> {
            match self.kind {
                Kind::Raw | Kind::Divert => &testinet::InetDom,
                Kind::Raw6 | Kind::Divert6 => &testinet::Inet6Dom,
            }
        }
        fn ty(&self) -> i16 {
            SOCK_RAW as i16
        }
        fn protocol(&self) -> i16 {
            self.proto
        }
        fn flags(&self) -> i16 {
            PR_ATOMIC | PR_ADDR
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
            Some(match self.kind {
                Kind::Raw => &RipUsrreqs::<G>::NEW,
                Kind::Raw6 => &crate::netinet6::raw_ip6::Rip6Usrreqs::<G>::NEW,
                Kind::Divert => &crate::netinet::ip_divert::DivertUsrreqs::<G>::NEW,
                Kind::Divert6 => &crate::netinet6::ip6_divert::Divert6Usrreqs::<G>::NEW,
            })
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
            match self.kind {
                Kind::Raw | Kind::Divert => rip_ctloutput(op, so, w, level, optname, m),
                Kind::Raw6 | Kind::Divert6 => {
                    crate::netinet6::raw_ip6::rip6_ctloutput(op, so, w, level, optname, m)
                }
            }
        }
    }

    /// `testinet::setup` and the four tables initialised.
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, &'static Thread<G>) {
        let r = testinet::setup();
        rip_init::<G>();
        crate::netinet6::raw_ip6::rip6_init::<G>();
        crate::netinet::ip_divert::divert_init::<G>();
        crate::netinet6::ip6_divert::divert6_init::<G>();
        r
    }

    /// `f` with `so` locked (`solock`); its answer is checked after the unlock.
    pub(crate) fn locked<R>(so: &Socket<G>, f: impl FnOnce(&mut SoLocked<'_>) -> R) -> R {
        let mut w = solock(so);
        f(&mut w)
    }

    /// A socket of `p` attached with protocol number `proto` (`socreate`'s two steps), by a
    /// privileged process (`SS_PRIV`) or not.
    pub(crate) fn socket(
        p: &'static TestProto,
        proto: i32,
        privileged: bool,
    ) -> Result<SocketRef<G>, Errno> {
        let so = soalloc(
            p,
            M_WAIT,
            SoInit {
                so_type: SOCK_RAW,
                state: if privileged { SS_PRIV } else { 0 },
                options: 0,
                linger: 0,
                timeo: 0,
                euid: 0,
                ruid: 0,
                egid: 0,
                rgid: 0,
                cpid: 1,
            },
        )
        .expect("a socket");
        let r = locked(&so, |w| match p.usrreqs() {
            Some(u) => u.attach(&so, w, proto, M_WAIT),
            None => Err(Errno::EPROTONOSUPPORT),
        });
        r.map(|()| so)
    }

    /// The socket's pcb, counted.
    pub(crate) fn pcb(so: &Socket<G>) -> InpcbRef<G> {
        sotoinpcb(so).expect("an attached socket")
    }

    /// `soclose`: the protocol's detach, the socket let go (no file holds it).
    pub(crate) fn close(so: SocketRef<G>) {
        so.st_set(SS_NOFDREF);
        let p = so.proto();
        let r = locked(&so, |w| match p.usrreqs() {
            Some(u) => u.detach(&so, w),
            None => Ok(()),
        });
        assert_eq!(r, Ok(()));
        drop(so);
    }

    /// `f` with a proof of the shared net lock (input's holding).
    pub(crate) fn with_net<R>(f: impl FnOnce(&NetHeld<'_>) -> R) -> R {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        f(&net)
    }

    /// `addr` in a name mbuf (`MT_SONAME`).
    pub(crate) fn nam<T: AbiPod>(addr: &T) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
        set_nam(&mut m, addr);
        m
    }

    /// A `sockaddr_in` of `addr` (port 0) in a name mbuf.
    pub(crate) fn nam4(addr: [u8; 4]) -> MbufBox {
        nam(&sin(addr, 0))
    }

    /// A `sockaddr_in` of `addr`:`port` (host order).
    pub(crate) fn sin(addr: [u8; 4], port: u16) -> SockaddrIn {
        SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_port: port.to_be(),
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(addr),
            },
            ..SockaddrIn::default()
        }
    }

    /// A `sockaddr_in6` of `addr` in a name mbuf.
    pub(crate) fn nam6(addr: In6Addr) -> MbufBox {
        let sin6 = sysdefs::netinet6::in6::SockaddrIn6 {
            sin6_len: 28,
            sin6_family: AF_INET6,
            sin6_addr: addr,
            ..Default::default()
        };
        nam(&sin6)
    }

    /// An IPv4 packet from `src` to `dst` of protocol `p`, its header (20 bytes, the length
    /// and version set) and `payload`.
    pub(crate) fn ip4(src: [u8; 4], dst: [u8; 4], p: u8, payload: &[u8]) -> Vec<u8> {
        let len = 20 + payload.len();
        let mut ip = Ip {
            ip_len: (len as u16).to_be(),
            ip_ttl: 64,
            ip_p: p,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes(src),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes(dst),
            },
            ..Ip::default()
        };
        ip.set_ip_v(4);
        ip.set_ip_hl(5);
        let mut b = std::vec![0u8; len];
        assert!(hal::mem::pod::put(&mut b, 0, &ip));
        b[20..].copy_from_slice(payload);
        b
    }

    /// `bytes` as a packet (one header mbuf, or a chain past `MHLEN`).
    pub(crate) fn pkt(bytes: &[u8]) -> MbufBox {
        m_devget(bytes, 0).expect("a packet")
    }

    /// The first record of `so`'s receive buffer taken: its address's and its data's bytes
    /// (the control messages skipped); `None` when the buffer is empty.
    pub(crate) fn recv(so: &Socket<G>) -> Option<(Vec<u8>, Vec<u8>)> {
        let rec = so.so_rcv.lock().take_first_record()?;
        let (mut name, mut data) = (Vec::new(), Vec::new());
        for m in rec.iter() {
            match i32::from(m.m_type().get()) {
                MT_SONAME => name.extend_from_slice(m.data()),
                MT_CONTROL => {}
                _ => data.extend_from_slice(m.data()),
            }
        }
        Some((name, data))
    }

    /// The IPv4 statistic `c`.
    pub(crate) fn ips(c: IpstatCounters) -> u64 {
        crate::netinet::ip_var::IPCOUNTERS.read()[c as usize]
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use sysdefs::netinet::in_::{IP_TTL, IPPROTO_TCP, IPPROTO_UDP};

    use super::testraw::{
        self, G, RAW, close, ip4, ips, locked, nam, nam4, pkt, recv, socket, with_net,
    };
    use super::*;
    use crate::net::mbuf::{M_WAIT, MT_SOOPTS, m_get};
    use crate::net::proto::PRCO_GETOPT;
    use crate::net::socket::socantrcvmore;
    use crate::netinet::in_pcb::testinet;

    /// An option mbuf of `bytes`.
    fn optm(bytes: &[u8]) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_SOOPTS).expect("an mbuf");
        m.m_len().set(bytes.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        m
    }

    /// `setsockopt(so, level, opt, &v, sizeof(int))` through `rip_ctloutput`.
    fn set(so: &Socket<G>, level: i32, opt: i32, v: i32) -> Result<(), Errno> {
        let mut m = optm(&v.to_ne_bytes());
        locked(so, |w| {
            rip_ctloutput(PRCO_SETOPT, so, w, level, opt, Some(&mut m))
        })
    }

    /// `getsockopt(so, level, opt)` through `rip_ctloutput`: the answer's bytes.
    fn get(so: &Socket<G>, level: i32, opt: i32) -> Result<Vec<u8>, Errno> {
        let mut m = optm(&[]);
        locked(so, |w| {
            rip_ctloutput(PRCO_GETOPT, so, w, level, opt, Some(&mut m))
        })
        .map(|()| m.data().to_vec())
    }

    /// `rip_input` of `bytes` (received on interface `ifidx`) under the shared net lock.
    fn deliver(bytes: &[u8], ifidx: u32) {
        let m = pkt(bytes);
        m.m_pkthdr().ph_ifidx.set(ifidx);
        let p = i32::from(bytes[9]);
        with_net(|net| {
            let mut mp = Some(m);
            let mut off = 20;
            assert_eq!(
                rip_input::<G>(&mut mp, &mut off, p, i32::from(AF_INET), None, net),
                IPPROTO_DONE
            );
            assert!(mp.is_none(), "the packet consumed");
        });
    }

    /// `rip_send(so, m, nam, NULL)` of `bytes`.
    fn send(so: &Socket<G>, bytes: &[u8], to: Option<[u8; 4]>) -> Result<(), Errno> {
        let n = to.map(nam4);
        locked(so, |w| {
            rip_send(so, w, Some(pkt(bytes)), n.as_deref(), None)
        })
    }

    /// The last packet `d` output.
    fn last(d: &testinet::InetDriver) -> Vec<u8> {
        d.last.lock().expect("the wire").clone()
    }

    #[test]
    fn rip_attach_needs_privilege_and_a_protocol_in_range() {
        let (_g, _p) = testraw::setup();
        assert_eq!(socket(&RAW, 1, false).err(), Some(Errno::EACCES));
        assert_eq!(
            socket(&RAW, IPPROTO_MAX, true).err(),
            Some(Errno::EPROTONOSUPPORT)
        );
        assert_eq!(socket(&RAW, -1, true).err(), Some(Errno::EPROTONOSUPPORT));

        let so = socket(&RAW, IPPROTO_UDP, true).expect("attached");
        let inp = testraw::pcb(&so);
        assert_eq!(i32::from(inp.inp_ip.ip_p()), IPPROTO_UDP);
        assert!(!inp.has_flags(INP_IPV6 | INP_HDRINCL));
        assert!(core::ptr::eq(inp.table(), rawcbtable::<G>()));
        drop(inp);

        // The slot's borrow answers the pcb until the detach takes it; a second detach finds
        // none (the C's EINVAL).
        let r = locked(&so, |w| {
            let il = inet(w);
            crate::netinet::in_pcb::sotoinpcb_locked(&so, &il.pcb).is_some()
        });
        assert!(r);
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        assert_eq!(locked(&so, |w| rip_detach(&so, w)), Ok(()));
        assert_eq!(locked(&so, |w| rip_detach(&so, w)), Err(Errno::EINVAL));
        drop(so);
    }

    #[test]
    fn rip_input_hands_a_copy_to_every_matching_socket() {
        let (_g, p) = testraw::setup();
        let (ifp, _d) = testinet::iface(b"rip0", [10, 9, 61, 1]);
        let ifidx = ifp.if_index.get();
        let any = socket(&RAW, IPPROTO_ICMP, true).expect("attached");
        let bound = socket(&RAW, IPPROTO_ICMP, true).expect("attached");
        assert_eq!(
            locked(&bound, |w| rip_bind(&bound, w, &nam4([10, 9, 61, 1]), p)),
            Ok(())
        );
        let elsewhere = socket(&RAW, IPPROTO_ICMP, true).expect("attached");
        assert_eq!(
            locked(&elsewhere, |w| rip_connect(
                &elsewhere,
                w,
                &nam4([10, 9, 61, 9])
            )),
            Ok(())
        );
        let udp = socket(&RAW, IPPROTO_UDP, true).expect("attached");
        let wild = socket(&RAW, 0, true).expect("attached");
        let closed = socket(&RAW, IPPROTO_ICMP, true).expect("attached");
        socantrcvmore(&closed);
        let noproto = ips(IpstatCounters::IpsNoproto);

        let echo = ip4([10, 9, 61, 2], [10, 9, 61, 1], 1, &[8, 0, 0, 0, 0, 1, 0, 1]);
        deliver(&echo, ifidx);
        for so in [&any, &bound, &wild] {
            let (name, data) = recv(so).expect("a copy");
            assert_eq!(data, echo, "the whole datagram, its header included");
            let src = hal::mem::pod::at::<SockaddrIn>(&name, 0).expect("a sockaddr_in");
            assert_eq!(src.sin_addr.s_addr, u32::from_ne_bytes([10, 9, 61, 2]));
            assert_eq!(src.sin_len, 16);
        }
        assert!(recv(&elsewhere).is_none(), "another foreign address");
        assert!(recv(&udp).is_none(), "another protocol");
        assert!(recv(&closed).is_none(), "SS_CANTRCVMORE");

        let dgram = ip4([10, 9, 61, 9], [10, 9, 61, 1], 17, &[0; 8]);
        deliver(&dgram, ifidx);
        assert_eq!(recv(&udp).map(|r| r.1), Some(dgram.clone()));
        assert_eq!(recv(&wild).map(|r| r.1), Some(dgram));
        assert!(recv(&any).is_none() && recv(&elsewhere).is_none());
        assert_eq!(ips(IpstatCounters::IpsNoproto), noproto);

        for so in [any, bound, elsewhere, udp, wild, closed] {
            close(so);
        }
    }

    #[test]
    fn rip_input_without_a_socket_counts_and_answers_protocol_unreachable() {
        use crate::netinet::icmp_var::{ICMPCOUNTERS, IcmpstatCounters};
        let icps = |c: IcmpstatCounters| ICMPCOUNTERS.read()[c as usize];
        let unreach = || ICMPCOUNTERS.read()[IcmpstatCounters::IcpsOuthist as usize + 3];

        let (_g, _p) = testraw::setup();
        let (ifp, _d) = testinet::iface(b"rip1", [10, 9, 62, 1]);
        let ifidx = ifp.if_index.get();
        let noproto = ips(IpstatCounters::IpsNoproto);
        let delivered = ips(IpstatCounters::IpsDelivered);
        let (error, out) = (icps(IcmpstatCounters::IcpsError), unreach());

        // ICMP: dropped quietly.
        deliver(
            &ip4([10, 9, 62, 2], [10, 9, 62, 1], 1, &[8, 0, 0, 0]),
            ifidx,
        );
        assert_eq!((icps(IcmpstatCounters::IcpsError), unreach()), (error, out));
        assert_eq!(ips(IpstatCounters::IpsNoproto), noproto + 1);
        assert_eq!(ips(IpstatCounters::IpsDelivered), delivered.wrapping_sub(1));

        // Another protocol: a protocol unreachable made for the sender (icmp_error; its
        // output is ip_send's task, later).
        deliver(&ip4([10, 9, 62, 2], [10, 9, 62, 1], 99, &[0; 8]), ifidx);
        assert_eq!(icps(IcmpstatCounters::IcpsError), error + 1);
        assert_eq!(unreach(), out + 1, "an ICMP_UNREACH sent");
        assert_eq!(ips(IpstatCounters::IpsNoproto), noproto + 2);
        assert_eq!(ips(IpstatCounters::IpsDelivered), delivered.wrapping_sub(2));
    }

    /// The header of `rip_chkhdr`'s cases: version `v`, length `hl` words, `ip_len` `len`, the
    /// options `opts` (`hl` - 5 words), the packet `len` bytes long unless `pad` differs.
    fn hdr(v: u8, hl: u8, len: u16, opts: &[u8], pktlen: usize) -> Vec<u8> {
        let mut b = ip4([10, 9, 0, 2], [10, 9, 0, 1], 17, &[]);
        b[0] = (v << 4) | hl;
        b[2..4].copy_from_slice(&len.to_be_bytes());
        b.extend_from_slice(opts);
        b.resize(pktlen, 0);
        b
    }

    #[test]
    #[rustfmt::skip]
    fn rip_chkhdr_keeps_every_bound_of_the_c() {
        let (_g, _p) = testraw::setup();
        let cases: [(&str, Vec<u8>, bool, bool); 13] = [
            ("short", std::vec![0x45; 19], false, false),
            ("hl < 5", hdr(4, 4, 20, &[], 20), false, false),
            ("ip_len < hlen", hdr(4, 6, 20, &[1, 1, 1, 1], 24), false, false),
            ("ip_len != len", hdr(4, 5, 28, &[], 24), false, false),
            ("version 6", hdr(6, 5, 20, &[], 20), false, false),
            ("both options", hdr(4, 6, 24, &[1, 1, 1, 0], 24), true, false),
            ("no options, setsockopt's", hdr(4, 5, 20, &[], 20), true, true),
            ("NOPs, EOL", hdr(4, 6, 24, &[1, 1, 1, 0], 24), false, true),
            ("an option of 4", hdr(4, 6, 24, &[7, 4, 0, 0], 24), false, true),
            ("optlen < 2", hdr(4, 6, 24, &[7, 1, 0, 0], 24), false, false),
            ("optlen > cnt", hdr(4, 6, 24, &[7, 5, 0, 0], 24), false, false),
            ("no length byte", hdr(4, 6, 24, &[1, 1, 1, 7], 24), false, false),
            ("an EOL ends the walk", hdr(4, 6, 24, &[0, 7, 9, 9], 24), false, true),
        ];
        for (what, bytes, options, ok) in cases {
            assert_eq!(rip_chkhdr(pkt(&bytes), options).is_some(), ok, "{what}");
        }
    }

    /// A reference reading of `rip_chkhdr`'s rule over `b` (no setsockopt options).
    fn reference(b: &[u8]) -> bool {
        if b.len() < 20 {
            return false;
        }
        let hlen = usize::from(b[0] & 0xf) * 4;
        let len = usize::from(u16::from_be_bytes([b[2], b[3]]));
        if hlen < 20 || len < hlen || len != b.len() || b[0] >> 4 != 4 {
            return false;
        }
        let mut cp = 20;
        while cp < hlen {
            match b[cp] {
                0 => return true,
                1 => cp += 1,
                _ => {
                    let Some(&l) = b.get(cp + 1).filter(|_| cp + 1 < hlen) else {
                        return false;
                    };
                    if l < 2 || cp + usize::from(l) > hlen {
                        return false;
                    }
                    cp += usize::from(l);
                }
            }
        }
        true
    }

    #[test]
    fn rip_chkhdr_never_reads_past_the_bytes_it_is_given() {
        let (_g, _p) = testraw::setup();
        // A property test over 4000 headers of a xorshift stream (no crate): every answer is
        // the reference's, and nothing panics (an option walk past the header would).
        let mut s: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        for _ in 0..4000 {
            let r = next();
            let hl = 5 + (r % 11) as u8;
            let pktlen = (r >> 8) as usize % 64;
            let mut b: Vec<u8> = (0..pktlen.max(1)).map(|_| (next() % 9) as u8).collect();
            if b.len() >= 4 {
                b[0] = if r & (1 << 40) != 0 {
                    0x40 | hl
                } else {
                    0x60 | hl
                };
                let len = if r & (1 << 41) != 0 {
                    b.len() as u16
                } else {
                    (r >> 48) as u16 % 64
                };
                b[2..4].copy_from_slice(&len.to_be_bytes());
            }
            assert_eq!(rip_chkhdr(pkt(&b), false).is_some(), reference(&b), "{b:?}");
        }
    }

    #[test]
    fn ip_hdrincl_output_keeps_the_user_s_header() {
        let (_g, _p) = testraw::setup();
        let (_ifp, d) = testinet::iface(b"rip2", [10, 9, 63, 1]);
        let so = socket(&RAW, IPPROTO_UDP, true).expect("attached");
        assert_eq!(set(&so, IPPROTO_IP, IP_HDRINCL, 1), Ok(()));
        let rawout = ips(IpstatCounters::IpsRawout);

        // Source 0 is selected, id 0 drawn, the destination the header's (the name's is
        // ignored), the rest the user's.
        let mut h = ip4([0, 0, 0, 0], [10, 9, 63, 7], 17, &[1, 2, 3, 4, 5, 6, 7, 8]);
        h[1] = 0x10; // tos
        h[8] = 3; // ttl
        assert_eq!(send(&so, &h, Some([10, 9, 63, 99])), Ok(()));
        let out = last(d);
        assert_eq!(out[16..20], [10, 9, 63, 7]);
        assert_eq!(out[12..16], [10, 9, 63, 1]);
        assert_ne!(out[4..6], [0, 0], "ip_id drawn");
        assert_eq!((out[1], out[8], out[9]), (0x10, 3, 17));
        assert_eq!(out[20..], [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(ips(IpstatCounters::IpsRawout), rawout + 1);

        // A header rip_chkhdr refuses (version 6), and one past IP_MAXPACKET.
        let mut bad = h.clone();
        bad[0] = 0x65;
        assert_eq!(send(&so, &bad, Some([10, 9, 63, 7])), Err(Errno::EINVAL));
        let huge = std::vec![0x45; IP_MAXPACKET + 1];
        assert_eq!(send(&so, &huge, Some([10, 9, 63, 7])), Err(Errno::EMSGSIZE));
        assert_eq!(ips(IpstatCounters::IpsRawout), rawout + 1);
        close(so);
    }

    #[test]
    fn rip_output_builds_the_header_from_the_prototype() {
        let (_g, _p) = testraw::setup();
        let (_ifp, d) = testinet::iface(b"rip3", [10, 9, 64, 1]);
        let so = socket(&RAW, 253, true).expect("attached");
        assert_eq!(set(&so, IPPROTO_IP, IP_TTL, 7), Ok(()));

        assert_eq!(send(&so, &[0xa5; 4], Some([10, 9, 64, 7])), Ok(()));
        let out = last(d);
        assert_eq!(out.len(), 24);
        assert_eq!(out[0] >> 4, 4);
        assert_eq!(u16::from_be_bytes([out[2], out[3]]), 24);
        assert_eq!(
            (out[8], out[9]),
            (7, 253),
            "the prototype's TTL and protocol"
        );
        assert_eq!(out[12..16], [10, 9, 64, 1]);
        assert_eq!(out[16..20], [10, 9, 64, 7]);
        assert_eq!(out[20..], [0xa5; 4]);

        // `m_pkthdr.len + sizeof(struct ip) > IP_MAXPACKET`.
        let big = std::vec![0; IP_MAXPACKET - 19];
        assert_eq!(send(&so, &big, Some([10, 9, 64, 7])), Err(Errno::EMSGSIZE));
        close(so);
    }

    #[test]
    fn rip_ctloutput_answers_as_the_c() {
        let (_g, _p) = testraw::setup();
        let so = socket(&RAW, 1, true).expect("attached");
        assert_eq!(
            get(&so, IPPROTO_IP, IP_HDRINCL),
            Ok(0i32.to_ne_bytes().to_vec())
        );
        assert_eq!(set(&so, IPPROTO_IP, IP_HDRINCL, 1), Ok(()));
        // The bit, INP_HDRINCL (8), not 1: the C's `inp_flags & INP_HDRINCL`.
        assert_eq!(
            get(&so, IPPROTO_IP, IP_HDRINCL),
            Ok(8i32.to_ne_bytes().to_vec())
        );
        assert_eq!(set(&so, IPPROTO_IP, IP_HDRINCL, 0), Ok(()));
        assert_eq!(
            get(&so, IPPROTO_IP, IP_HDRINCL),
            Ok(0i32.to_ne_bytes().to_vec())
        );
        // A value shorter than an int, or none.
        let mut short = optm(&[1, 0]);
        assert_eq!(
            locked(&so, |w| rip_ctloutput(
                PRCO_SETOPT,
                &so,
                w,
                IPPROTO_IP,
                IP_HDRINCL,
                Some(&mut short)
            )),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            locked(&so, |w| rip_ctloutput(
                PRCO_SETOPT,
                &so,
                w,
                IPPROTO_IP,
                IP_HDRINCL,
                None
            )),
            Err(Errno::EINVAL)
        );
        // MROUTING is not configured; another level is refused; IP's options go on.
        assert_eq!(set(&so, IPPROTO_IP, MRT_INIT, 1), Err(Errno::EOPNOTSUPP));
        assert_eq!(get(&so, IPPROTO_IP, MRT_API_CONFIG), Err(Errno::EOPNOTSUPP));
        assert_eq!(set(&so, IPPROTO_TCP, IP_HDRINCL, 1), Err(Errno::EINVAL));
        assert_eq!(set(&so, IPPROTO_IP, IP_TTL, 9), Ok(()));
        assert_eq!(
            get(&so, IPPROTO_IP, IP_TTL),
            Ok(9i32.to_ne_bytes().to_vec())
        );
        close(so);
    }

    #[test]
    fn rip_bind_takes_ours_a_broadcast_or_any() {
        let (_g, p) = testraw::setup();
        let _ = testinet::iface(b"rip4", [10, 9, 65, 1]);
        let so = socket(&RAW, 1, true).expect("attached");
        let bind = |a: [u8; 4]| locked(&so, |w| rip_bind(&so, w, &nam4(a), p));
        assert_eq!(bind([10, 9, 65, 1]), Ok(()), "ours");
        assert_eq!(bind([10, 9, 65, 255]), Ok(()), "the subnet's broadcast");
        assert_eq!(bind([255, 255, 255, 255]), Ok(()), "INADDR_BROADCAST");
        assert_eq!(bind([0, 0, 0, 0]), Ok(()), "INADDR_ANY");
        assert_eq!(bind([192, 0, 2, 1]), Err(Errno::EADDRNOTAVAIL));
        assert_eq!(
            testraw::pcb(&so).inp_laddr.get().s_addr,
            0,
            "a refused bind leaves the address"
        );
        locked(&so, |w| so.set_options(w, SO_BINDANY, true));
        assert_eq!(bind([192, 0, 2, 1]), Ok(()), "SO_BINDANY");
        assert_eq!(
            testraw::pcb(&so).inp_laddr.get().s_addr,
            u32::from_ne_bytes([192, 0, 2, 1])
        );
        close(so);
    }

    #[test]
    fn rip_send_needs_an_address_unless_connected() {
        let (_g, _p) = testraw::setup();
        let (_ifp, d) = testinet::iface(b"rip5", [10, 9, 66, 1]);
        let so = socket(&RAW, 253, true).expect("attached");
        assert_eq!(send(&so, &[1], None), Err(Errno::ENOTCONN));
        assert_eq!(
            locked(&so, |w| rip_disconnect(&so, w)),
            Err(Errno::ENOTCONN)
        );
        assert_eq!(
            locked(&so, |w| rip_connect(&so, w, &nam4([10, 9, 66, 7]))),
            Ok(())
        );
        assert!(so.has_state(SS_ISCONNECTED));
        assert_eq!(send(&so, &[1], Some([10, 9, 66, 8])), Err(Errno::EISCONN));
        assert_eq!(send(&so, &[1], None), Ok(()));
        assert_eq!(last(d)[16..20], [10, 9, 66, 7]);
        assert_eq!(
            locked(&so, |w| rip_send(&so, w, None, None, None)),
            Err(Errno::EINVAL),
            "no packet (sosend never)"
        );
        assert_eq!(locked(&so, |w| rip_disconnect(&so, w)), Ok(()));
        assert_eq!(testraw::pcb(&so).inp_faddr.get().s_addr, 0);
        assert_eq!(
            locked(&so, |w| rip_send(
                &so,
                w,
                Some(pkt(&[1])),
                Some(&nam(&testraw::sin([10, 9, 66, 9], 0))),
                None
            )),
            Ok(())
        );
        close(so);
    }
}
/* </TESTS> */
