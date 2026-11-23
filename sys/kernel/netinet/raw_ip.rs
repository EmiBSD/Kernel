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
//! - `MROUTING` is not configured: the `MRT_*` options (`hal::sys::netinet_ip_mroute`)
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
//!   no count per request (N6.6a5's idiom, docs/IDIOMS.md); attach fills the slot with
//!   `in_pcballoc`'s handle and detach takes it out for `in_pcbdetach`. The proof of the
//!   net lock comes with the socket lock's parts ([`InetLocked`]) on requests and with the
//!   caller on input. `rawcbtable` is the glue's ([`rawcbtable`], `NetTables`).
//! - Input walks the table with `in_pcb_iterator` (one step and a count pair per pcb, an
//!   `m_copym` per extra match, the mutex let go around each append: the C's); the
//!   prototype's protocol is one word's load ([`IpProto::ip_p`]). The headers are read and
//!   written through `hal::pod` (`ip_load`, `ip_store`), never a pointer kept across an mbuf
//!   call.
//! - [`rip_chkhdr`] reads the options as the bytes of the pulled-up header, each length
//!   checked before the byte it covers is read; it answers the packet or `None` (freed).
//! - `rip_output` takes one loan of the pcb's IPv4 options for the whole send (the
//!   `IP_HDRINCL` refusal's test and `ip_output`'s argument), where LZ took two.
//! - LZ's `inpcb_of` (a `sotoinpcb` that panics) goes: a request without a pcb is
//!   `panic(9)` where the C dereferences NULL, `rip_detach` answers `EINVAL` as the C.

use core::marker::PhantomData;
use core::mem::size_of;
use core::pin::pin;

use hal::netlock::NetHeld;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::netinet_in::{
    INADDR_ANY, INADDR_BROADCAST, IP_HDRINCL, IPPROTO_DONE, IPPROTO_ICMP, IPPROTO_IP, IPPROTO_MAX,
    InAddr, SockaddrIn,
};
use hal::sys::netinet_ip::{
    IP_MAXPACKET, IPOPT_EOL, IPOPT_NOP, IPOPT_OLEN, IPOPT_OPTVAL, IPVERSION, Ip, MAXTTL,
};
use hal::sys::netinet_ip_icmp::{ICMP_UNREACH, ICMP_UNREACH_PROTOCOL};
use hal::sys::netinet_ip_mroute::{
    MRT_ADD_MFC, MRT_ADD_VIF, MRT_API_CONFIG, MRT_API_SUPPORT, MRT_ASSERT, MRT_DEL_MFC,
    MRT_DEL_VIF, MRT_DONE, MRT_INIT, MRT_VERSION,
};
use hal::sys::socket::{AF_INET, SO_BINDANY, SO_TIMESTAMP};
use hal::user::{AbiPod, bytes_of};

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
    InetLocked, SoLocked, Socket, socantsendmore, soisconnected, soisdisconnected, soreserve,
    sorwakeup,
};
use crate::netinet::in_::{in_broadcast, in_control, in_nam2sin};
use crate::netinet::in_pcb::{
    INP_CONTROLOPTS, INP_HDRINCL, INP_IPV6, Inpcb, InpcbRef, Inpcbtable, in_pcb_iterator,
    in_pcballoc, in_pcbdetach, in_pcbinit, in_pcbselsrc, in_peeraddr, in_sockaddr,
    sotoinpcb_locked,
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

/// The socket lock of a request on an internet socket of the kernel's, opened in its parts
/// (the C's `soassertlocked`): such a socket is always held by a witness of `so_lock` under a
/// proof of the net lock. `who` names the request in the `panic(9)` of a lock held another
/// way. The raw and divert requests share it.
pub(crate) fn inet<'w>(w: &'w mut SoLocked<'_>, who: &str) -> InetLocked<'w> {
    match w.inet() {
        Some(il) => il,
        None => kpanic(format_args!(
            "{who}: the socket lock is not held as a kernel protocol's"
        )),
    }
}

/// `sotoinpcb(so)` of a request the C makes on an attached socket (it dereferences the
/// pointer unchecked): the pcb borrowed from the socket's locked slot, `panic(9)` without one.
pub(crate) fn pcb_of<'a, G: ProcGlue>(
    so: &'a Socket<G>,
    il: &'a InetLocked<'_>,
    who: &str,
) -> &'a Inpcb<G> {
    match sotoinpcb_locked(so, &il.pcb) {
        Some(inp) => inp,
        None => kpanic(format_args!("{who}: socket {so:p} has no pcb")),
    }
}

/// `in_pcbdetach(sotoinpcb(so))` of a detach request: the pcb taken out of the socket's
/// locked slot (whose handle the attach put there) and detached; `false` when the socket has
/// none (the C's `inp == NULL`).
pub(crate) fn detach_slot<G: ProcGlue>(so: &Socket<G>, il: &mut InetLocked<'_>) -> bool {
    let Some(slot) = so.set_pcb_locked(&mut il.pcb, None) else {
        return false;
    };
    let Some(inp) = slot.downcast_ref::<Inpcb<G>>().and_then(PoolArc::from_ref) else {
        kpanic(format_args!(
            "detach: socket {so:p}: a slot of another type"
        ));
    };
    drop(slot);
    in_pcbdetach(inp, &il.net);
    true
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
    let (mut m, mut ip) = if !inp.has_flags(INP_HDRINCL) {
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
    } else {
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

/// `addr` written into the mbuf `m` (`m_len` its size): a `PRCO_GETOPT` answer or a name.
pub(crate) fn set_mbuf<T: AbiPod>(m: &mut Mbuf, addr: &T, who: &str) {
    let bytes = sa_bytes(addr);
    m.m_len().set(bytes.len() as u32);
    match m.data_mut() {
        Some(d) => d.copy_from_slice(bytes),
        None => kpanic(format_args!("{who}: a read-only mbuf")),
    }
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
    let il = inet(w, "rip_ctloutput");
    let inp = pcb_of(so, &il, "rip_ctloutput");

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
                    Some(m) => set_mbuf(m, &(inp.inp_flags.get() & INP_HDRINCL), "rip_ctloutput"),
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
    let mut il = inet(w, "rip_attach");
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// `rip_detach(so)`: the control block detached; `EINVAL` without one.
pub fn rip_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w, "rip_detach");

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
    let il = inet(w, "rip_bind");
    let inp = pcb_of(so, &il, "rip_bind");

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
    let il = inet(w, "rip_connect");
    let inp = pcb_of(so, &il, "rip_connect");

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
    let il = inet(w, "rip_disconnect");
    let inp = pcb_of(so, &il, "rip_disconnect");

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
    let _il = inet(w, "rip_shutdown");
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
    let il = inet(w, "rip_send");
    let inp = pcb_of(so, &il, "rip_send");

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
/* </CODE> */
