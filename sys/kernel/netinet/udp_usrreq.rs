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
//! The User Datagram Protocol, per RFC 768, August, 1980: `netinet/udp_usrreq.c`.
//!
//! Upstream: sys/netinet/udp_usrreq.c @ 3ce1f3f79392
//! LZ: sys/netinet/udp_usrreq.rs@3c62ede68802
//!
//! [`udp_input`] checks a datagram's length and checksum and appends it, with the sender's
//! address, to the socket bound to its destination: the one pf linked to the packet's state,
//! the connected one (`in_pcblookup`), else the listening one (`in_pcblookup_listen`); a
//! broadcast or multicast datagram goes to every matching socket, until one without
//! `SO_REUSEPORT` or `SO_REUSEADDR`. Without a socket the sender gets a port unreachable. A
//! datagram to IPsec's encapsulation port with a non-zero SPI is ESP, handed to IPsec.
//! [`udp_output`] prepends the UDP and IP headers (choosing the source address and, for an
//! unbound socket, a port; `IP_SENDSRCADDR` and `IP_IPSECFLOWINFO` control messages) and
//! sends with `ip_output`; the checksum is left to the interface or `in_proto_cksum_out`.
//! [`udp_ctlinput`] and [`udp6_ctlinput`] pass ICMP errors on to the sockets they concern.
//! The user requests are [`UdpUsrreqs`]; [`udp_sysctl`] is `net.inet.udp`.
//!
//! Locks used to protect data: \[a\] atomic.
//!
//! ## Deviations
//! - `udpcksum`, `udp_sendspace` and `udp_recvspace` are `AtomicI32`s, the type
//!   `sysctl_bounded_arr` takes (`u_int` for the spaces in C; the bounds keep them
//!   positive), as LZ's.
//! - `INET6` is configured: the IPv6 paths of `udp_input`, `udp_sbappend`
//!   (`ip6_savecontrol`, `IPV6_RECVDSTPORT`), `udp_output` (`udp6_output`) and the requests,
//!   `udb6table`, `udp6_usrreqs` and `udp6_ctlinput`. `NPF` (`pf_inp_lookup`, `pf_inp_link`,
//!   `pf_mbuf_link_inpcb`, through the `Pf` bridge), `NSTOEPLITZ` (the flow id) and `IPSEC`
//!   (through the `Ipsec` bridge: the inbound policy and its flow id, the UDP encapsulation of
//!   ESP and its path MTU) are configured; `PIPEX` is not, a comment at its sites.
//!   `SMALL_KERNEL` is not set: the sysctl handlers are compiled.
//! - The requests the C reaches through `sotoinpcb(so)` unchecked (`udp_bind`,
//!   `udp_connect`, `udp_disconnect`, `in_sockaddr`, `in_peeraddr`, `in_flowid`) are
//!   `panic(9)` on a socket without a pcb, where the C dereferences NULL, as LZ's; none is
//!   reached after the close that detaches it. `pru_rcvd` and `pru_abort`, NULL in the C's
//!   table and never called for UDP (no `PR_WANTRCVD`, no connection queue), are `panic(9)`,
//!   as `bsd`'s wrapper of a NULL entry was.
//! - With the `Ipsec` legs not registered (the C without `IPSEC`, the host tests' default)
//!   the encapsulation block is skipped and the datagram goes to the sockets, and
//!   `udp_ctlinput`'s path MTU block (`udpencap_pmtu`) is not taken: the control input goes
//!   on to the socket lookup; the kernel registers them at boot.
//!
//! ## Redesign
//! - N6.6b0: LZ's `bsd` module over `&'static Socket`, parked mbufs, raw header pointers
//!   (`unsafe fn uh_read`/`uh_set_sum`), the raw `pr_ctlinput` contract and `bsd`'s
//!   adapters (each send lending the net lock to `ip_output`) is the kernel's, with no
//!   `unsafe`. `udp_usrreqs`/`udp6_usrreqs` are [`UdpUsrreqs`] (decision 46: one type whose
//!   two values differ in `pru_control`, `pru_sockaddr` and `pru_peeraddr`, as the C's two
//!   tables share every other function; one `send_id`, so a v4 and a v6 socket splice, as the
//!   C's shared `udp_send`).
//! - A request holds the socket lock as a witness ([`SoLocked`], the C's `soassertlocked`;
//!   `udp_send`'s `soassertlocked_readonly` is the same witness under the shared net lock)
//!   and borrows the pcb from the socket's locked slot (`sotoinpcb_locked`, part `PCB`: the
//!   C's load of `so_pcb`, no count; N6.6a5's idiom). `udp_attach` fills the slot with
//!   `in_pcballoc`'s handle; `udp_detach` (and `udp_disconnect`'s detach of a socket without
//!   a file) takes it out and hands it to `in_pcbdetach`, so a request after the close finds
//!   no pcb (`udp_send`'s `EINVAL`).
//! - Input holds no socket lock, as the C (`soassertlocked_readonly` is `NET_ASSERT_LOCKED`
//!   for an internet socket, which the caller's proof `net` is; the append under the receive
//!   buffer's mutex): it reads the pcb's atomic members and the socket's options only. The pcb
//!   is the lookup's or the walk's counted handle (the C's `in_pcbref`), or pf's ([`InpHold`]).
//! - The UDP and IP headers are read once into copies through `hal::pod` (`ip6_exthdr_get`'s
//!   bytes for the UDP header); the packet is never written by `udp_input` but for the
//!   encapsulation's move of the IP header over the UDP header (one overlapping
//!   `copy_within`, the C's `bcopy`). The checksum the C stores through its pointer is stored
//!   in the copy (what the upcall sees); the packet keeps its own, so the C's restore of
//!   `*ip = save_ip` and `uh->uh_sum = savesum` before the ICMP error is the packet as it
//!   came: the error quotes the same bytes.
//! - The sender's address (`srcsa`, a union of `struct sockaddr_in` and `sockaddr_in6`) is
//!   its bytes, `sa_len` of them handed to `sbappendaddr`; `dstsa`, which the C fills and
//!   never reads, is not built.
//! - `udp_output` writes `struct udpiphdr` with `hal::pod::put` over the prepended space,
//!   its `uh_sum` bytes read first and kept (the C does not set them: the checksum is
//!   `in_proto_cksum_out`'s with `M_UDP_CSUM_OUT`, and without `udpcksum` the bytes are what
//!   the mbuf held). The control messages are walked over the control mbuf's bytes with the
//!   C's bounds in the C's integer types (`u_int clen`, `CMSG_ALIGN` in `unsigned long`); the
//!   4-byte values (`IP_IPSECFLOWINFO`, the `IP_SENDSRCADDR` address) read unaligned in native
//!   order, the C's `*(uint32_t *)CMSG_DATA(cm)`.
//! - `udp_ctlinput` takes the ICMP message (`CtlArg::Ip`, from its type byte) and reads the
//!   returned IP header and the UDP ports after it with their lengths checked (`icmp_input`
//!   hands on no shorter quote); its path MTU branch for UDP-encapsulated ESP compares
//!   `uh_sport` with `udpencap_port` **without `htons`, as the C does** (the 16-bit network
//!   value against the host-order port), then calls the `Ipsec` leg with the message. The
//!   matching socket is locked with `in_pcbsolock` under the caller's net lock, as the C.
//! - `inp_upcall` (wg(4)'s `wg_input`) is called with the packet, the header copies and the
//!   softnet's stack, the C's arguments; it answers the packet back or `None` when it took it.
//! - `udp_sysctl`'s port bitmaps go through `in_pcb::in_sysctl_ports` (the C's
//!   `M_SYSCTL` buffer, shared with TCP), closing LZ's stack-buffer deviation;
//!   `udp_sysctl_udpstat` builds `struct udpstat` (`hal::sys`'s image) from the per-CPU
//!   counters in their order.
//! - `udbtable` and `udb6table` are members of the glue's network tables (`NetTables`),
//!   reached by [`udbtable`] and [`udb6table`] (pf's socket lookup and `kern.file`'s walk,
//!   `bsd`'s).
//!
//! Costs against the C, per operation (the rest is the C's step for step):
//!
//! | operation | the C | the kernel |
//! |---|---|---|
//! | send, connected | `so_pcb` loaded; `inp_options`, `inp_route`, `inp_moptions` read | the slot's borrow (a load and a key compare); the options and route loans (two atomics each, N12's D-10) and the multicast options copied (two atomics) |
//! | send, unconnected | as connected, plus `in_pcbselsrc` and, unbound, `inpt_mtx` | the same |
//! | input, unicast | `ipsec_in_use` loaded; the lookup's `inpt_mtx` pair and count pair | a dynamic call each to `Pf::inp_lookup` and `Ipsec::spd_check_in_flow` (the latter loads `ipsec_in_use`); the same mutex and count |
//! | input, multicast | per pcb one iterator step, per match a count pair and the mutex dropped | the same (`step_over`, O(1) a step) |
//! | input, the upcall (wg) | an indirect call | the same (N6.6b0's bridge row `InpUpcall`) |

use core::marker::PhantomData;
use core::mem::{offset_of, size_of};
use core::sync::atomic::{AtomicI32, Ordering};

use hal::netlock::NetHeld;
use hal::pod;
use hal::pool::PoolArc;
use hal::rwlock::WitnessPart;
use hal::sys::errno::Errno;
use hal::sys::netinet_icmp6::{ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT};
use hal::sys::netinet_in::{
    INADDR_ANY, IP_IPSECFLOWINFO, IP_RECVDSTPORT, IP_SENDSRCADDR, IPPROTO_DONE, IPPROTO_IP,
    IPPROTO_IPV6, IPPROTO_UDP, InAddr, SockaddrIn,
};
use hal::sys::netinet_ip::{IP_MAXPACKET, Ip};
use hal::sys::netinet_ip_icmp::{ICMP_MINLEN, ICMP_UNREACH, ICMP_UNREACH_PORT};
use hal::sys::netinet_ip_var::Ipovly;
use hal::sys::netinet_ip6::Ip6Hdr;
use hal::sys::netinet_udp::Udphdr;
use hal::sys::netinet_udp_var::{
    UDPCTL_BADDYNAMIC, UDPCTL_CHECKSUM, UDPCTL_RECVSPACE, UDPCTL_ROOTONLY, UDPCTL_SENDSPACE,
    UDPCTL_STATS, Udpiphdr, Udpstat,
};
use hal::sys::netinet6_in6::{
    IPV6_RECVDSTPORT, In6Addr, SA6_ANY, SockaddrIn6, in6_are_addr_equal, in6_is_addr_unspecified,
};
use hal::sys::socket::{
    AF_INET, AF_INET6, CMSG_DATA_OFFSET, Cmsghdr, PF_INET6, SO_BROADCAST, SO_REUSEADDR,
    SO_REUSEPORT, SO_TIMESTAMP, cmsg_align, cmsg_len,
};
use hal::sys::sysctl::SysctlBoundedArgs;
use hal::user::{AbiPod, bytes_of};

use crate::assert::{kassert, kpanic};
use crate::net::domain::net;
use crate::net::if_::unhandled_af;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::ipsec_hooks::{ESP_ENABLE, Ipsec, UDPENCAP_ENABLE, UDPENCAP_PORT, ipsec};
use crate::net::mbuf::{
    M_BCAST, M_COPYALL, M_DONTWAIT, M_FLOWID, M_MCAST, M_UDP_CSUM_IN_BAD, M_UDP_CSUM_IN_OK,
    M_UDP_CSUM_OUT, Mbuf, MbufBox, PF_TAG_DIVERTED, m_adj, m_copydata, m_copym, m_prepend,
    m_pullup,
};
use crate::net::pf_hooks::pf;
use crate::net::proto::{
    CtlArg, PRC_HOSTDEAD, PRC_MSGSIZE, PRC_NCMDS, SockaddrView, Usrreqs, prc_is_redirect,
};
use crate::net::rtable::rtable_l2;
use crate::net::sockbuf::{SB_MAX, SS_CANTRCVMORE, SS_ISCONNECTED, SS_PRIV, sbcreatecontrol};
use crate::net::socket::{
    InetLocked, SoLocked, Socket, socantsendmore, soisconnected, soreserve, sorwakeup, sowwakeup,
};
use crate::netinet::in_::{in_control, in_nam2sin};
use crate::netinet::in_pcb::InpUpcall;
use crate::netinet::in_pcb::{
    IN6P_CONTROLOPTS, IN6P_RECVDSTPORT, INP_CONTROLOPTS, INP_IPSECFLOWINFO, INP_IPV6,
    INP_RECVDSTPORT, InpHold, Inpcb, InpcbRef, Inpcbtable, in_pcb_iterator, in_pcb_iterator_abort,
    in_pcbaddrisavail, in_pcballoc, in_pcbbind, in_pcbconnect, in_pcbdetach, in_pcbdisconnect,
    in_pcbinit, in_pcblookup, in_pcblookup_listen, in_pcbnotifyall, in_pcbrtchange, in_pcbselsrc,
    in_pcbsolock, in_pcbunset_laddr, in_setpeeraddr, in_setsockaddr, in_sysctl_ports,
    sotoinpcb_locked,
};
use crate::netinet::in4_cksum::in4_cksum;
use crate::netinet::ip_icmp::icmp_error;
use crate::netinet::ip_input::{INETCTLERRMAP, ip_savecontrol};
use crate::netinet::ip_output::{ip_ctloutput, ip_load, ip_output};
use crate::netinet::ip_var::{IP_DEFTTL, ip_mtudisc};
use crate::netinet::udp_var::{UDPCOUNTERS, UdpstatCounters, udpstat_inc};
use crate::netinet6::icmp6::{icmp6_error, icmp6_mtudisc_update};
use crate::netinet6::in6::{in6_addr2scopeid, in6_control};
use crate::netinet6::in6_cksum::in6_cksum;
use crate::netinet6::in6_pcb::{
    in6_pcblookup, in6_pcblookup_listen, in6_pcbnotify, in6_setpeeraddr, in6_setsockaddr,
};
use crate::netinet6::in6_src::{in6_embedscope, in6_recoverscope};
use crate::netinet6::ip6_input::{
    INET6CTLERRMAP, ip6_exthdr_bytes, ip6_exthdr_get, ip6_savecontrol,
};
use crate::netinet6::ip6_output::{ip6_ctloutput, ip6_load};
use crate::netinet6::ip6_var::IP6_DEFHLIM;
use crate::netinet6::udp6_output::udp6_output;
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::{SysctlReq, bounded_arr, rdstruct};

/// `UDB_INITIAL_HASH_SIZE`.
const UDB_INITIAL_HASH_SIZE: i32 = 128;

/// `AF_INET` as `pr_input`'s `af`.
const AF_INET_I32: i32 = AF_INET as i32;
/// `AF_INET6` as `pr_input`'s `af`.
const AF_INET6_I32: i32 = AF_INET6 as i32;

/// `udp_usrreqs` and `udp6_usrreqs`: UDP's user requests, the two tables differing in
/// `pru_control`, `pru_sockaddr` and `pru_peeraddr` only.
pub struct UdpUsrreqs<G: ProcGlue> {
    /// The IPv6 table (`in6_control`, `in6_sockaddr`, `in6_peeraddr`).
    inet6: bool,
    /// The glue.
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> UdpUsrreqs<G> {
    /// `udp_usrreqs`.
    pub const INET: Self = Self {
        inet6: false,
        _g: PhantomData,
    };
    /// `udp6_usrreqs`.
    pub const INET6: Self = Self {
        inet6: true,
        _g: PhantomData,
    };
}

/// \[a\] `udpcksum`.
pub static UDPCKSUM: AtomicI32 = AtomicI32::new(1);
/// \[a\] `udp_sendspace`: really max datagram size.
pub static UDP_SENDSPACE: AtomicI32 = AtomicI32::new(9216);
/// \[a\] `udp_recvspace`: 40 1K datagrams.
pub static UDP_RECVSPACE: AtomicI32 = AtomicI32::new(40 * (1024 + size_of::<SockaddrIn>() as i32));

/// `udpctl_vars[]`.
static UDPCTL_VARS: [SysctlBoundedArgs; 3] = [
    SysctlBoundedArgs::new(UDPCTL_CHECKSUM, &UDPCKSUM, 0, 1),
    SysctlBoundedArgs::new(UDPCTL_RECVSPACE, &UDP_RECVSPACE, 0, SB_MAX as i32),
    SysctlBoundedArgs::new(UDPCTL_SENDSPACE, &UDP_SENDSPACE, 0, SB_MAX as i32),
];

/// The identity of `udp_send` for `sosplice` (`pru_send != pru_send`): both tables answer
/// this static's address.
static UDP_SEND_ID: u8 = 0;

/// `udbtable`: the glue's IPv4 UDP control blocks (pf's socket lookup and `kern.file`'s walk
/// read it).
pub fn udbtable<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().udbtable
}

/// `udb6table`: the glue's IPv6 UDP control blocks, as [`udbtable`].
pub fn udb6table<G: ProcGlue>() -> &'static Inpcbtable<G> {
    &net::<G>().udb6table
}

/// The socket lock of a request opened in its parts (the C's `soassertlocked`): a kernel UDP
/// socket is always held by a witness under a proof of the net lock.
fn inet<'w>(w: &'w mut SoLocked<'_>) -> InetLocked<'w> {
    match w.inet() {
        Some(il) => il,
        None => kpanic(format_args!(
            "udp: the socket lock is not held as a kernel protocol's"
        )),
    }
}

/// `sotoinpcb(so)` of the requests that dereference it unchecked: the pcb borrowed from the
/// socket's locked slot; `panic(9)` without one, where the C dereferences NULL.
fn udp_inp<'a, G: ProcGlue>(so: &'a Socket<G>, pcb: &'a WitnessPart<'_>) -> &'a Inpcb<G> {
    match sotoinpcb_locked(so, pcb) {
        Some(inp) => inp,
        None => kpanic(format_args!("udp socket {so:p}: no inpcb")),
    }
}

/// `curproc`, which the socket requests run as (`panic(9)` without one: a request reaches a
/// protocol from a system call or a kernel thread).
fn with_curproc<G: ProcGlue, R>(func: &str, f: impl FnOnce(&Thread<G>) -> R) -> R {
    G::with_curproc(|p| match p {
        Some(p) => f(p),
        None => kpanic(format_args!("{func}: no curproc")),
    })
}

/// `addr` written into the name mbuf `nam` (`m_len` its size).
fn set_nam<T: AbiPod>(nam: &mut Mbuf, addr: &T) {
    let bytes = bytes_of(core::slice::from_ref(addr));
    nam.m_len().set(bytes.len() as u32);
    match nam.data_mut() {
        Some(d) => d.copy_from_slice(bytes),
        None => kpanic(format_args!("udp: a read-only name mbuf")),
    }
}

/// `udp_init`: the two tables (`udpcounters` is a static).
pub fn udp_init<G: ProcGlue>() {
    in_pcbinit(udbtable::<G>(), UDB_INITIAL_HASH_SIZE);
    // INET6
    in_pcbinit(udb6table::<G>(), UDB_INITIAL_HASH_SIZE);
}

/// The sender's address of a datagram (the C's `srcsa`, a `struct sockaddr_in` or
/// `sockaddr_in6`), as the bytes `sbappendaddr` takes.
struct SrcAddr {
    /// The address's bytes, `sa_len` of them used.
    b: [u8; size_of::<SockaddrIn6>()],
}

impl SrcAddr {
    /// `bzero(&srcsa)` and the IPv4 sender `ip_src`:`sport`.
    fn inet(ip: &Ip, sport: u16) -> Self {
        let sin = SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_port: sport,
            sin_addr: ip.ip_src,
            ..SockaddrIn::default()
        };
        let mut b = [0u8; size_of::<SockaddrIn6>()];
        let _ = pod::put(&mut b, 0, &sin);
        Self { b }
    }

    /// `bzero(&srcsa)` and the IPv6 sender `ip6_src`:`sport`, its scope recovered (KAME
    /// hack: recover scopeid). The inbound flow information is not set (`#if 0` in the C).
    fn inet6(ip6: &Ip6Hdr, sport: u16) -> Self {
        let mut sin6 = SockaddrIn6 {
            sin6_port: sport,
            ..SockaddrIn6::with_addr(In6Addr::default())
        };
        in6_recoverscope(&mut sin6, &ip6.ip6_src);
        let mut b = [0u8; size_of::<SockaddrIn6>()];
        let _ = pod::put(&mut b, 0, &sin6);
        Self { b }
    }

    /// The address's `sa_len` bytes.
    fn bytes(&self) -> &[u8] {
        &self.b[..usize::from(self.b[0])]
    }
}

/// `udp_input`: a datagram from IP (`*mp`, its UDP header at `*offp`, of family `af`), under
/// the net lock (`net`): to the socket(s) bound to its destination, a port unreachable when
/// none is, or IPsec for a UDP-encapsulated ESP packet. `IPPROTO_DONE` once the packet is
/// consumed; IPsec's answer for an encapsulated packet (its inner protocol with `*mp`).
pub fn udp_input<G: ProcGlue>(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    _proto: i32,
    af: i32,
    ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let iphlen = *offp;
    // IPSEC: atomic_load_int(&udpencap_port), read once.
    let udpencap_port_local = UDPENCAP_PORT.load(Ordering::Relaxed);

    udpstat_inc(UdpstatCounters::UdpsIpackets);

    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let Some((m, at, o)) = ip6_exthdr_get(m, iphlen, size_of::<Udphdr>() as i32) else {
        udpstat_inc(UdpstatCounters::UdpsHdrops);
        return IPPROTO_DONE;
    };
    let Some(mut uh) =
        ip6_exthdr_bytes(&m, at, o, size_of::<Udphdr>()).and_then(|b| pod::at::<Udphdr>(b, 0))
    else {
        kpanic(format_args!(
            "udp_input: the header ip6_exthdr_get made contiguous"
        ));
    };

    // Each early end of the C's `goto bad`: the packet freed (no pcb is held yet).
    // Check for illegal destination port 0
    if uh.uh_dport == 0 {
        udpstat_inc(UdpstatCounters::UdpsNoport);
        return IPPROTO_DONE;
    }

    // Make mbuf data length reflect UDP length. If not enough data to reflect UDP length,
    // drop. `ip` (also the C's `save_ip`) or `ip6` is the packet's header, copied.
    let mut len = i32::from(u16::from_be(uh.uh_ulen));
    let plen = m.m_pkthdr().len.get() - iphlen;
    let (ip, ip6): (Option<Ip>, Option<Ip6Hdr>) = match af {
        AF_INET_I32 => {
            if plen != len {
                if len > plen || len < size_of::<Udphdr>() as i32 {
                    udpstat_inc(UdpstatCounters::UdpsBadlen);
                    return IPPROTO_DONE;
                }
                m_adj(&m, len - plen);
            }
            (Some(ip_load(&m)), None)
        }
        // INET6
        AF_INET6_I32 => {
            // jumbograms
            if len == 0 && plen > 0xffff {
                len = plen;
            }
            if len != plen {
                udpstat_inc(UdpstatCounters::UdpsBadlen);
                return IPPROTO_DONE;
            }
            (None, Some(ip6_load(&m)))
        }
        _ => unhandled_af(af),
    };

    // Checksum extended UDP header and data. from W.R.Stevens: check incoming udp cksums
    // even if udpcksum is not set.
    if uh.uh_sum == 0 {
        udpstat_inc(UdpstatCounters::UdpsNosum);
        // In IPv6, the UDP checksum is ALWAYS used.
        if ip6.is_some() {
            return IPPROTO_DONE;
        }
    } else if m.m_pkthdr().csum_flags.get() & M_UDP_CSUM_IN_OK == 0 {
        if m.m_pkthdr().csum_flags.get() & M_UDP_CSUM_IN_BAD != 0 {
            udpstat_inc(UdpstatCounters::UdpsBadsum);
            return IPPROTO_DONE;
        }
        udpstat_inc(UdpstatCounters::UdpsInswcsum);

        if ip.is_some() {
            uh.uh_sum = in4_cksum(&m, IPPROTO_UDP as u8, iphlen, len);
        } else if ip6.is_some() {
            uh.uh_sum = in6_cksum(&m, IPPROTO_UDP as u8, iphlen as u32, len as u32);
        }
        if uh.uh_sum != 0 {
            udpstat_inc(UdpstatCounters::UdpsBadsum);
            return IPPROTO_DONE;
        }
    }
    let ph = m.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() & !M_UDP_CSUM_OUT);

    // IPSEC: the UDP encapsulation of ESP (its legs registered: the C with IPSEC).
    if UDPENCAP_ENABLE.load(Ordering::Relaxed) != 0
        && udpencap_port_local != 0
        && ESP_ENABLE.load(Ordering::Relaxed) != 0
        // NPF > 0
        && m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED == 0
        && uh.uh_dport == (udpencap_port_local as u16).to_be()
        && let Some(legs) = ipsec()
    {
        let mut skip = iphlen + size_of::<Udphdr>() as i32;

        if m.m_pkthdr().len.get() - skip < size_of::<u32>() as i32 {
            // packet too short
            drop(m);
            return IPPROTO_DONE;
        }
        let mut spi = [0u8; 4];
        m_copydata(&m, skip, &mut spi);
        // decapsulate if the SPI is not zero, otherwise pass to userland
        if spi != [0; 4] {
            let Some(mut m) = m_pullup(m, skip) else {
                udpstat_inc(UdpstatCounters::UdpsHdrops);
                return IPPROTO_DONE;
            };

            // remove the UDP header: the IP header moved up over it (the first `skip` bytes
            // are the first mbuf's, `m_pullup`'s; `ip_input` made them writable).
            let Some(d) = m.data_mut() else {
                // A read-only first mbuf, which `m_pullup` gave back as it was: dropped as
                // its failure.
                udpstat_inc(UdpstatCounters::UdpsHdrops);
                return IPPROTO_DONE;
            };
            d.copy_within(..iphlen as usize, size_of::<Udphdr>());
            m_adj(&m, size_of::<Udphdr>() as i32);
            skip -= size_of::<Udphdr>() as i32;

            // espstat_inc(esps_udpencin): the leg's first step.
            let protoff = if af == AF_INET_I32 {
                offset_of!(Ip, ip_p)
            } else {
                offset_of!(Ip6Hdr, ip6_nxt)
            } as i32;
            *mp = Some(m);
            return legs.udpencap_input(mp, skip, protoff, af, ns);
        }
    }

    let srcsa = match (&ip, &ip6) {
        (Some(ip), _) => SrcAddr::inet(ip, uh.uh_sport),
        (None, Some(ip6)) => SrcAddr::inet6(ip6, uh.uh_sport),
        (None, None) => unhandled_af(af),
    };

    if m.m_flags().get() & (M_BCAST | M_MCAST) != 0 {
        // Deliver a multicast or broadcast datagram to *all* sockets for which the local and
        // remote addresses and ports match those of the incoming datagram. This allows more
        // than one process to receive multi/broadcasts on the same port. (This really ought
        // to be done for unicast datagrams as well, but that would cause problems with
        // existing applications that open both address-specific sockets and a wildcard
        // socket listening to the same port -- they would end up receiving duplicates of
        // every unicast datagram. Those applications open the multiple sockets to overcome an
        // inadequacy of the UDP socket interface, but for backwards compatibility we avoid the
        // problem here rather than fixing the interface. Maybe 4.5BSD will remedy this?)
        udp_input_mcast::<G>(m, ip.as_ref(), ip6.as_ref(), iphlen, &uh, &srcsa, ns);
        return IPPROTO_DONE;
    }

    // Locate pcb for datagram.
    // NPF > 0
    let mut inp: Option<InpHold<G>> = pf().and_then(|pf| pf.inp_lookup(&m)).and_then(InpHold::pf);
    let rtableid = m.m_pkthdr().ph_rtableid.get();
    if inp.is_none() {
        inp = match (&ip, &ip6) {
            (_, Some(ip6)) => in6_pcblookup(
                udb6table::<G>(),
                &ip6.ip6_src,
                uh.uh_sport,
                &ip6.ip6_dst,
                uh.uh_dport,
                rtableid,
            ),
            (Some(ip), _) => in_pcblookup(
                udbtable::<G>(),
                ip.ip_src,
                uh.uh_sport,
                ip.ip_dst,
                uh.uh_dport,
                rtableid,
            ),
            (None, None) => None,
        }
        .map(InpHold::Pcb);
    }
    if inp.is_none() {
        udpstat_inc(UdpstatCounters::UdpsPcbhashmiss);
        inp = match (&ip, &ip6) {
            (_, Some(ip6)) => in6_pcblookup_listen(
                udb6table::<G>(),
                &ip6.ip6_dst,
                uh.uh_dport,
                Some(&m),
                rtableid,
            ),
            (Some(ip), _) => {
                in_pcblookup_listen(udbtable::<G>(), ip.ip_dst, uh.uh_dport, Some(&m), rtableid)
            }
            (None, None) => None,
        }
        .map(InpHold::Pcb);
    }

    // IPSEC: while IPsec is in use, the inbound policy (the leg's sequence: the innermost
    // IPsec tag's SA, ipsp_spd_lookup, the SA's flow id, tdb_unref); a refusal drops the
    // datagram. The flow id is not modified after the SA's creation.
    let mut ipsecflowinfo: u32 = 0;
    if let Some(legs) = ipsec() {
        let seclevel = inp.as_deref().map(|i| i.inp_seclevel.get());
        match legs.spd_check_in_flow(&m, af, iphlen, seclevel.as_ref()) {
            Ok(flow) => ipsecflowinfo = flow,
            Err(_) => {
                udpstat_inc(UdpstatCounters::UdpsNosec);
                // bad: m_freem(m), in_pcbunref(inp)
                drop(m);
                drop(inp);
                return IPPROTO_DONE;
            }
        }
    }

    let Some(inp) = inp else {
        udpstat_inc(UdpstatCounters::UdpsNoport);
        if m.m_flags().get() & (M_BCAST | M_MCAST) != 0 {
            udpstat_inc(UdpstatCounters::UdpsNoportbcast);
            return IPPROTO_DONE;
        }
        // The packet's header and checksum are as they came (the C's `*ip = save_ip` and
        // `uh->uh_sum = savesum`: nothing above wrote them).
        if ip6.is_some() {
            icmp6_error(m, ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT, 0, net);
        } else {
            icmp_error(m, ICMP_UNREACH, ICMP_UNREACH_PORT, 0, 0, net);
        }
        return IPPROTO_DONE;
    };

    // soassertlocked_readonly(inp->inp_socket): the net lock (`net`).
    let minttl = inp.inp_ip_minttl.get();
    if let Some(ip6) = &ip6 {
        if minttl != 0 && minttl > ip6.ip6_hlim {
            return IPPROTO_DONE;
        }
    } else if let Some(ip) = &ip
        && minttl != 0
        && minttl > ip.ip_ttl
    {
        return IPPROTO_DONE;
    }

    // NPF > 0
    if inp.socket().has_state(SS_ISCONNECTED)
        && let Some(pf) = pf()
    {
        let a: &(dyn core::any::Any + Sync) = &*inp;
        pf.inp_link(&m, a);
    }

    // PIPEX: pipex_l2tp_lookup_session and pipex_l2tp_input; not configured.

    udp_sbappend(
        &inp,
        m,
        ip.as_ref(),
        ip6.as_ref(),
        iphlen,
        &uh,
        &srcsa,
        ipsecflowinfo,
        ns,
    );
    // in_pcbunref(inp)
    drop(inp);
    IPPROTO_DONE
}

/// `udp_input`'s broadcast and multicast delivery: a copy of `m` to every matching socket of
/// the family's table but the last, which gets `m`; the walk stops after a socket without
/// `SO_REUSEPORT` or `SO_REUSEADDR`. The table mutex is dropped around each append, the
/// previous match counted meanwhile (the C's `in_pcbref(last)`). Without a match the packet
/// is discarded (no port unreachable for a broadcast or multicast datagram).
fn udp_input_mcast<G: ProcGlue>(
    mut m: MbufBox,
    ip: Option<&Ip>,
    ip6: Option<&Ip6Hdr>,
    iphlen: i32,
    uh: &Udphdr,
    srcsa: &SrcAddr,
    mut ns: Option<&mut Netstack>,
) {
    let table = if ip6.is_some() {
        udb6table::<G>()
    } else {
        udbtable::<G>()
    };
    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());

    let iter = core::pin::pin!(Inpcb::<G>::marker());
    let mut last: Option<InpcbRef<G>> = None;
    let mut inp: Option<InpcbRef<G>> = None;
    let mut g = table.lock();
    while let Some(i) = in_pcb_iterator(table, &mut g, inp.take(), iter.as_ref()) {
        kassert!(ip6.is_some() == i.has_flags(INP_IPV6));

        let so = i.socket();
        let matches = !so.so_rcv.has_state(SS_CANTRCVMORE)
            && rtable_l2(i.inp_rtableid.get()) == rdomain
            && i.inp_lport.get() == uh.uh_dport
            && match (ip, ip6) {
                (_, Some(ip6)) => {
                    let minhlim = i.inp_ip6_minhlim().get();
                    let laddr6 = i.inp_laddr6.get::<In6Addr>();
                    let faddr6 = i.inp_faddr6.get::<In6Addr>();
                    !(minhlim != 0 && minhlim > ip6.ip6_hlim)
                        && (in6_is_addr_unspecified(&laddr6)
                            || in6_are_addr_equal(&laddr6, &ip6.ip6_dst))
                        && (in6_is_addr_unspecified(&faddr6)
                            || (in6_are_addr_equal(&faddr6, &ip6.ip6_src)
                                && i.inp_fport.get() == uh.uh_sport))
                }
                (Some(ip), None) => {
                    let minttl = i.inp_ip_minttl.get();
                    let laddr = i.inp_laddr.get().s_addr;
                    let faddr = i.inp_faddr.get().s_addr;
                    !(minttl != 0 && minttl > ip.ip_ttl)
                        && (laddr == INADDR_ANY || laddr == ip.ip_dst.s_addr)
                        && (faddr == INADDR_ANY
                            || (faddr == ip.ip_src.s_addr && i.inp_fport.get() == uh.uh_sport))
                }
                (None, None) => false,
            };
        if !matches {
            inp = Some(i);
            continue;
        }

        if let Some(l) = last.take() {
            drop(g);

            if let Some(n) = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT) {
                udp_sbappend(&l, n, ip, ip6, iphlen, uh, srcsa, 0, ns.as_deref_mut());
            }
            // in_pcbunref(last)
            drop(l);

            g = table.lock();
        }
        // in_pcbref(inp)
        last = Some(i.clone());

        // Don't look for additional matches if this one does not have either the
        // SO_REUSEPORT or SO_REUSEADDR socket options set. This heuristic avoids searching
        // through all pcbs in the common case of a non-shared port. It assumes that an
        // application will never clear these options after setting them.
        if !i.socket().has_options(SO_REUSEPORT | SO_REUSEADDR) {
            in_pcb_iterator_abort(table, &mut g, Some(i), iter.as_ref());
            break;
        }
        inp = Some(i);
    }
    drop(g);

    let Some(last) = last else {
        // No matching pcb found; discard datagram. (No need to send an ICMP Port Unreachable
        // for a broadcast or multicast datagram.)
        udpstat_inc(UdpstatCounters::UdpsNoportbcast);
        drop(m);
        return;
    };

    udp_sbappend(&last, m, ip, ip6, iphlen, uh, srcsa, 0, ns);
    // in_pcbunref(last)
    drop(last);
}

/// The control message `n` put at the end of the chain `opts` (the C's walk to the last
/// `m_next`; a NULL `n` appends nothing).
fn opts_append(opts: &mut Option<MbufBox>, n: Option<MbufBox>) {
    let Some(n) = n else {
        return;
    };
    let Some(mut t) = opts.as_deref_mut() else {
        *opts = Some(n);
        return;
    };
    while t.m_next.is_some() {
        t = match t.next_mut() {
            Some(x) => x,
            None => kpanic(format_args!("udp: a control chain's link")),
        };
    }
    drop(t.m_next.set(n));
}

/// `udp_sbappend`: appends datagram `m` from `srcaddr` (its IP header `ip` or IPv6 header
/// `ip6`, `hlen` bytes, and UDP header `uh` still in front) to the receive buffer of `inp`'s
/// socket, with the control messages the socket asked for; the upcall, if any, sees it
/// first. No socket lock: the append is under the receive buffer's mutex.
#[allow(clippy::too_many_arguments, reason = "the C's signature")]
fn udp_sbappend<G: ProcGlue>(
    inp: &Inpcb<G>,
    m: MbufBox,
    ip: Option<&Ip>,
    ip6: Option<&Ip6Hdr>,
    hlen: i32,
    uh: &Udphdr,
    srcaddr: &SrcAddr,
    ipsecflowinfo: u32,
    ns: Option<&mut Netstack>,
) {
    let so = inp.socket();
    let mut m = m;
    let mut opts: Option<MbufBox> = None;

    let hlen = hlen + size_of::<Udphdr>() as i32;

    if let Some(InpUpcall { f, arg }) = inp.upcall() {
        match f(arg, m, ip, ip6, uh, hlen, ns) {
            Some(n) => m = n,
            None => return,
        }
    }

    // INET6
    if ip6.is_some() && (inp.has_flags(IN6P_CONTROLOPTS) || so.has_options(SO_TIMESTAMP)) {
        opts = ip6_savecontrol(inp, &m);
    }
    if let Some(ip) = ip
        && (inp.has_flags(INP_CONTROLOPTS) || so.has_options(SO_TIMESTAMP))
    {
        opts = ip_savecontrol(inp, ip, &m);
    }
    // INET6
    if ip6.is_some() && inp.has_flags(IN6P_RECVDSTPORT) {
        let n = sbcreatecontrol(&uh.uh_dport.to_ne_bytes(), IPV6_RECVDSTPORT, IPPROTO_IPV6);
        opts_append(&mut opts, n);
    }
    if ip.is_some() && inp.has_flags(INP_RECVDSTPORT) {
        let n = sbcreatecontrol(&uh.uh_dport.to_ne_bytes(), IP_RECVDSTPORT, IPPROTO_IP);
        opts_append(&mut opts, n);
    }
    // IPSEC
    if ipsecflowinfo != 0 && inp.has_flags(INP_IPSECFLOWINFO) {
        let n = sbcreatecontrol(&ipsecflowinfo.to_ne_bytes(), IP_IPSECFLOWINFO, IPPROTO_IP);
        opts_append(&mut opts, n);
    }
    m_adj(&m, hlen);

    let mut sb = so.so_rcv.lock();
    if let Err((m, opts)) = sb.sbappendaddr(srcaddr.bytes(), Some(m), opts) {
        drop(sb);
        udpstat_inc(UdpstatCounters::UdpsFullsock);
        // m_freem(m), m_freem(opts)
        drop(m);
        drop(opts);
        return;
    }
    drop(sb);

    sorwakeup(so);
}

/// `udp_notify`: notify a udp user of an asynchronous error; just wake up so that he can
/// collect error status. The socket `w` locks (the C's walks lock it).
pub fn udp_notify<G: ProcGlue>(inp: &Inpcb<G>, _w: &mut SoLocked<'_>, errno: Option<Errno>) {
    let so = inp.socket();
    so.set_error(errno);
    sorwakeup(so);
    sowwakeup(so);
}

/// `in_pcbrtchange` as the walks' notify (the socket's holding unused).
fn rtchange_notify<G: ProcGlue>(inp: &Inpcb<G>, _w: &mut SoLocked<'_>, errno: Option<Errno>) {
    in_pcbrtchange(inp, errno);
}

/// The two ports of a returned UDP header (`struct udp_portonly`, network order), from its
/// four bytes.
fn ports_of(b: [u8; 4]) -> (u16, u16) {
    (
        u16::from_ne_bytes([b[0], b[1]]),
        u16::from_ne_bytes([b[2], b[3]]),
    )
}

/// `udp6_ctlinput`: an ICMPv6 error (`cmd`) about a datagram to `sa`; `arg` is the error's
/// [`Ip6CtlParam`](crate::netinet6::icmp6::Ip6CtlParam), or none; the caller's proof of the
/// net lock `net`. A path MTU change (`PRC_MSGSIZE`) updates the route if a connected socket
/// matches; then every matching socket is notified (`in6_pcbnotify`). `INET6`.
pub fn udp6_ctlinput<G: ProcGlue>(
    cmd: i32,
    sa: SockaddrView<'_>,
    rdomain: u32,
    arg: CtlArg<'_>,
    net: &NetHeld<'_>,
) {
    if sa.family() != AF_INET6 || usize::from(sa.len()) != size_of::<SockaddrIn6>() {
        return;
    }
    let Some(sa_sin6) = pod::at::<SockaddrIn6>(sa.bytes(), 0) else {
        return;
    };
    // The argument of this family only (an IPv4 quote is never handed to an IPv6 entry).
    let mut d = match arg {
        CtlArg::Ip6(p) => Some(p),
        CtlArg::None => None,
        CtlArg::Ip(_) => return,
    };

    if cmd as u32 as usize >= PRC_NCMDS {
        return;
    }
    // A redirect's notify is in_pcbrtchange, which `in6_pcbnotify` chooses itself for one.
    if prc_is_redirect(cmd) || cmd == PRC_HOSTDEAD {
        d = None;
    } else if cmd == PRC_MSGSIZE {
        // special code is present, see below
    } else if INET6CTLERRMAP[cmd as usize].is_none() {
        return;
    }

    // if the parameter is from icmp6, decode it; the address in internal form (XXX:
    // translate addresses into internal form; `in6_embedscope` "should be impossible" to
    // fail).
    let mut sa6 = sa_sin6;
    let Some(p) = d else {
        let mut a = sa6.sin6_addr;
        if in6_embedscope(&mut a, &sa_sin6, 0, 0).is_err() {
            return;
        }
        sa6.sin6_addr = a;
        let mut notify = udp_notify::<G>;
        in6_pcbnotify(
            udb6table::<G>(),
            &sa6,
            0,
            Some(&SA6_ANY),
            0,
            rdomain,
            cmd,
            net,
            Some(&mut notify),
        );
        return;
    };

    // XXX: assuming M is valid in this case. The final destination's zone embedded in the
    // parameter itself, as the C writes `*ip6c_finaldst`; `sa6` keeps the address as given,
    // with its scope id.
    let ifidx = p.m.m_pkthdr().ph_ifidx.get();
    let finaldst = p.finaldst.get();
    sa6 = SockaddrIn6::with_addr(finaldst);
    sa6.sin6_scope_id = in6_addr2scopeid(ifidx, &finaldst) as u32;
    let mut embedded = finaldst;
    if in6_embedscope(&mut embedded, &sa6, 0, 0).is_err() {
        // should be impossible
        return;
    }
    p.finaldst.set(embedded);

    // XXX: We assume that when IPV6 is non NULL, M and OFF are valid.
    let (m, off) = (p.m, p.off);

    // check if we can safely examine src and dst ports (`struct udp_portonly`; the C's
    // `int` against the `size_t` sum, unsigned)
    if (m.m_pkthdr().len.get() as u64) < (off as u64).wrapping_add(4) {
        return;
    }
    let mut b = [0u8; 4];
    m_copydata(m, off, &mut b);
    let (uh_sport, uh_dport) = ports_of(b);

    let src = p.ip6.ip6_src;
    let mut sa6_src = SockaddrIn6::with_addr(src);
    sa6_src.sin6_scope_id = in6_addr2scopeid(ifidx, &src) as u32;
    let mut a = sa6_src.sin6_addr;
    if in6_embedscope(&mut a, &sa6_src, 0, 0).is_err() {
        // should be impossible
        return;
    }
    sa6_src.sin6_addr = a;

    if cmd == PRC_MSGSIZE {
        // Check to see if we have a valid UDP socket corresponding to the address in the
        // ICMPv6 message payload.
        let inp = in6_pcblookup(
            udb6table::<G>(),
            &sa6.sin6_addr,
            uh_dport,
            &sa6_src.sin6_addr,
            uh_sport,
            rdomain,
        );
        // (#if 0 in the C: as the use of sendto(2) is fairly popular, we may want to allow
        // non-connected pcb too. But it could be too weak against attacks... We should at
        // least check if the local address (= s) is really ours.)

        // Depending on the value of "valid" and routing table size (mtudisc_{hi,lo}wat), we
        // will:
        // - recalculate the new MTU and create the corresponding routing entry, or
        // - ignore the MTU change notification.
        icmp6_mtudisc_update(p, inp.is_some(), net);
        // in_pcbunref(inp)
        drop(inp);

        // regardless of if we called icmp6_mtudisc_update(), we need to call
        // in6_pcbnotify(), to notify path MTU change to the userland (2292bis-02), because
        // some unconnected sockets may share the same destination and want to know the path
        // MTU.
    }

    let mut notify = udp_notify::<G>;
    in6_pcbnotify(
        udb6table::<G>(),
        &sa6,
        u32::from(uh_dport),
        Some(&sa6_src),
        u32::from(uh_sport),
        rdomain,
        cmd,
        net,
        Some(&mut notify),
    );
}

/// `udp_ctlinput`: an ICMP error (`cmd`) about a datagram to `sa`: `arg` the ICMP message
/// that returned its IP header (from its type byte), or none; the caller's proof of the net
/// lock `net`. Notifies the socket that sent it, or every socket talking to `sa`; a path MTU
/// message about UDP-encapsulated ESP goes to IPsec.
pub fn udp_ctlinput<G: ProcGlue>(
    cmd: i32,
    sa: SockaddrView<'_>,
    rdomain: u32,
    arg: CtlArg<'_>,
    net: &NetHeld<'_>,
) {
    let mut notify: fn(&Inpcb<G>, &mut SoLocked<'_>, Option<Errno>) = udp_notify::<G>;

    if sa.family() != AF_INET || usize::from(sa.len()) != size_of::<SockaddrIn>() {
        return;
    }
    let Some(sin) = pod::at::<SockaddrIn>(sa.bytes(), 0) else {
        return;
    };
    if sin.sin_addr.s_addr == INADDR_ANY {
        return;
    }
    // The argument of this family only (an IPv6 parameter is never handed to an IPv4 entry).
    let mut ip = match arg {
        CtlArg::Ip(msg) => Some(msg),
        CtlArg::None => None,
        CtlArg::Ip6(_) => return,
    };

    if cmd as u32 as usize >= PRC_NCMDS {
        return;
    }
    let errno = INETCTLERRMAP[cmd as usize];
    if prc_is_redirect(cmd) {
        notify = rtchange_notify::<G>;
        ip = None;
    } else if cmd == PRC_HOSTDEAD {
        ip = None;
    } else if errno.is_none() {
        return;
    }

    let Some(msg) = ip else {
        in_pcbnotifyall(udbtable::<G>(), &sin, rdomain, errno, net, notify);
        return;
    };

    // The returned IP header and the UDP ports after it (`uhp`, at the header's length),
    // read where the quote holds them (`icmp_input` hands on no shorter one).
    let Some(iph) = pod::at::<Ip>(msg, ICMP_MINLEN) else {
        return;
    };
    let at = ICMP_MINLEN + (usize::from(iph.ip_hl()) << 2);
    let Some(&[b0, b1, b2, b3]) = msg.get(at..at + 4) else {
        return;
    };
    let (uh_sport, uh_dport) = ports_of([b0, b1, b2, b3]);
    if udpencap_pmtu(ipsec(), cmd, uh_sport, rdomain, msg) {
        return;
    }
    let inp = in_pcblookup(
        udbtable::<G>(),
        iph.ip_dst,
        uh_dport,
        iph.ip_src,
        uh_sport,
        rdomain,
    );
    if let Some(i) = inp.as_deref()
        && let Some(mut w) = in_pcbsolock(i, net)
    {
        notify(i, &mut w, errno);
    }
    // in_pcbsounlock (the holding's drop above), in_pcbunref
    drop(inp);
}

/// `udp_ctlinput`'s `#ifdef IPSEC` block, PMTU discovery for udpencap: a path MTU message
/// whose returned datagram left from `udpencap_port` (`uh_sport`, the 16-bit network-order
/// value, against the host-order variable: the C's comparison, no `htons`) goes to IPsec's
/// leg with the ICMP message `msg`, and `true` ends the control input. `false` without the
/// legs (`legs` is `ipsec()`; the C without `IPSEC` has no such block and goes on to the
/// socket lookup) or for any other message.
fn udpencap_pmtu(
    legs: Option<&dyn Ipsec>,
    cmd: i32,
    uh_sport: u16,
    rdomain: u32,
    msg: &[u8],
) -> bool {
    let Some(legs) = legs else {
        return false;
    };
    // atomic_load_int(&udpencap_port)
    let udpencap_port_local = UDPENCAP_PORT.load(Ordering::Relaxed);
    if cmd == PRC_MSGSIZE
        && ip_mtudisc.load(Ordering::Relaxed) != 0
        && UDPENCAP_ENABLE.load(Ordering::Relaxed) != 0
        && udpencap_port_local != 0
        && i32::from(uh_sport) == udpencap_port_local
    {
        legs.udpencap_ctlinput(rdomain, msg);
        return true;
    }
    false
}

/// `udp_output`: sends datagram `m` from `inp` to `addr` (or its peer), with the source
/// address an `IP_SENDSRCADDR` control message may choose and the flow an
/// `IP_IPSECFLOWINFO` one gives, the socket lock held (`net` its proof of the net lock). The
/// datagram and the control are consumed.
pub fn udp_output<G: ProcGlue>(
    inp: &Inpcb<G>,
    m: MbufBox,
    addr: Option<&Mbuf>,
    control: Option<MbufBox>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    // INET6
    if inp.has_flags(INP_IPV6) {
        return udp6_output(inp, m, addr, control, net);
    }

    let len = m.m_pkthdr().len.get();
    let mut ipsecflowinfo: u32 = 0;
    let mut src_sin = SockaddrIn::default();

    // Compute the packet length of the IP header, and punt if the length looks bogus (the
    // C's `int` converted to the `size_t` sum).
    if (len as usize).wrapping_add(size_of::<Udpiphdr>()) > IP_MAXPACKET {
        // release: m_freem(m), m_freem(control)
        return Err(Errno::EMSGSIZE);
    }

    if let Some(control) = control.as_deref() {
        // XXX: Currently, we assume all the optional information is stored in a single mbuf.
        if control.m_next.is_some() {
            return Err(Errno::EINVAL);
        }

        let cmsgs = control.data();
        let mut clen: u32 = control.m_len().get();
        let mut at = 0usize;
        loop {
            if (clen as usize) < cmsg_len(0) {
                return Err(Errno::EINVAL);
            }
            let Some(cm) = pod::at::<Cmsghdr>(cmsgs, at) else {
                return Err(Errno::EINVAL);
            };
            let cmlen = cm.cmsg_len as usize;
            if cmlen < cmsg_len(0) || cmsg_align(cmlen) > clen as usize {
                return Err(Errno::EINVAL);
            }
            let data = at + CMSG_DATA_OFFSET;
            // IPSEC
            if inp.has_flags(INP_IPSECFLOWINFO)
                && cmlen == cmsg_len(size_of::<u32>())
                && cm.cmsg_level == IPPROTO_IP
                && cm.cmsg_type == IP_IPSECFLOWINFO
            {
                ipsecflowinfo = pod::at::<u32>(cmsgs, data).unwrap_or(0);
            } else if cmlen == cmsg_len(size_of::<InAddr>())
                && cm.cmsg_level == IPPROTO_IP
                && cm.cmsg_type == IP_SENDSRCADDR
            {
                src_sin.sin_addr = pod::at::<InAddr>(cmsgs, data).unwrap_or_default();
                src_sin.sin_family = AF_INET;
                src_sin.sin_len = size_of::<SockaddrIn>() as u8;
                // no check on reuse when sin->sin_port == 0
                in_pcbaddrisavail(inp, &mut src_sin, 0, net)?;
            }
            clen -= cmsg_align(cmlen) as u32;
            at += cmsg_align(cmlen);
            if clen == 0 {
                break;
            }
        }
    }

    let (laddr, dst, dport) = if let Some(addr) = addr {
        let sin = in_nam2sin(addr)?;
        if sin.sin_port == 0 {
            return Err(Errno::EADDRNOTAVAIL);
        }
        if inp.inp_faddr.get().s_addr != INADDR_ANY {
            return Err(Errno::EISCONN);
        }
        let mut laddr = in_pcbselsrc(&sin, inp, net)?;

        if inp.inp_lport.get() == 0 {
            with_curproc::<G, _>("udp_output", |p| in_pcbbind(inp, None, p, net))?;
        }

        if src_sin.sin_len > 0
            && src_sin.sin_addr.s_addr != INADDR_ANY
            && src_sin.sin_addr.s_addr != inp.inp_laddr.get().s_addr
        {
            src_sin.sin_port = inp.inp_lport.get();
            if inp.inp_laddr.get().s_addr != INADDR_ANY {
                in_pcbaddrisavail(inp, &mut src_sin, 0, net)?;
            }
            laddr = src_sin.sin_addr;
        }
        (laddr, sin.sin_addr, sin.sin_port)
    } else {
        if inp.inp_faddr.get().s_addr == INADDR_ANY {
            return Err(Errno::ENOTCONN);
        }
        (
            inp.inp_laddr.get(),
            inp.inp_faddr.get(),
            inp.inp_fport.get(),
        )
    };

    // Calculate data length and get a mbuf for UDP and IP headers.
    let Some(mut m) = m_prepend(m, size_of::<Udpiphdr>() as i32, M_DONTWAIT) else {
        // bail: m_freem(control)
        return Err(Errno::ENOBUFS);
    };

    // Fill in mbuf with extended UDP header and addresses and length put into network
    // format; the checksum's bytes as they are.
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("udp_output: a new header mbuf is read-only"));
    };
    let uh_sum = pod::at::<Udpiphdr>(d, 0).map_or(0, |ui| ui.ui_u.uh_sum);
    let ui_len = (len as u16)
        .wrapping_add(size_of::<Udphdr>() as u16)
        .to_be();
    let ui = Udpiphdr {
        ui_i: Ipovly {
            ih_x1: [0; 9],
            ih_pr: IPPROTO_UDP as u8,
            ih_len: ui_len,
            ih_src: laddr,
            ih_dst: dst,
        },
        ui_u: Udphdr {
            uh_sport: inp.inp_lport.get(),
            uh_dport: dport,
            uh_ulen: ui_len,
            uh_sum,
        },
    };
    // The IP header over the overlay: its length, TTL and TOS from the pcb's prototype.
    if !pod::put(d, 0, &ui) {
        kpanic(format_args!("udp_output: no room for the headers"));
    }
    let Some(mut ip) = pod::at::<Ip>(d, 0) else {
        kpanic(format_args!("udp_output: no room for the headers"));
    };
    ip.ip_len = ((size_of::<Udpiphdr>() as i32 + len) as u16).to_be();
    let proto = inp.inp_ip.get();
    ip.ip_ttl = proto.ip_ttl;
    ip.ip_tos = proto.ip_tos;
    let _ = pod::put(d, 0, &ip);

    let ph = m.m_pkthdr();
    if UDPCKSUM.load(Ordering::Relaxed) != 0 {
        ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_OUT);
    }

    udpstat_inc(UdpstatCounters::UdpsOpackets);

    // force routing table
    ph.ph_rtableid.set(inp.inp_rtableid.get());

    let so = inp.socket();
    if so.has_state(SS_ISCONNECTED) {
        // NPF > 0: the packet names its pcb (pf's own reference, pf's leg).
        if let Some(pf) = pf() {
            let a: &(dyn core::any::Any + Sync) = inp;
            pf.mbuf_link_inpcb_of(&m, a);
        }
        // NSTOEPLITZ
        let ph = m.m_pkthdr();
        ph.ph_flowid.set(inp.inp_flowid.get());
        ph.csum_flags.set(ph.csum_flags.get() | M_FLOWID);
    }

    // The options and the route cache lent for the call, the multicast options copied (the
    // socket lock is held).
    let imo = inp.moptions_out();
    let opts = inp.options();
    let mut ro = inp.route();
    let error = ip_output(
        m,
        opts.as_deref().map(Mbuf::data),
        Some(&mut *ro),
        so.options() & SO_BROADCAST,
        imo.as_ref(),
        Some(&inp.inp_seclevel.get()),
        ipsecflowinfo,
        net,
    );
    drop(ro);
    drop(opts);

    // bail: m_freem(control)
    drop(control);
    error
}

/// `udp_attach`: a control block in `udbtable` (or `udb6table`), the default TTL or hop
/// limit, the buffer sizes; the pcb in the socket's locked slot for the requests.
pub fn udp_attach<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    _proto: i32,
    wait: i32,
) -> Result<(), Errno> {
    if so.has_pcb() {
        return Err(Errno::EINVAL);
    }

    soreserve(
        so,
        u64::from(UDP_SENDSPACE.load(Ordering::Relaxed) as u32),
        u64::from(UDP_RECVSPACE.load(Ordering::Relaxed) as u32),
    )?;

    // INET6
    let table = if so.dom_family() == i32::from(PF_INET6) {
        udb6table::<G>()
    } else {
        udbtable::<G>()
    };
    let inp = in_pcballoc(so, table, wait)?;
    if inp.has_flags(INP_IPV6) {
        let mut ip6: Ip6Hdr = inp.inp_ipv6.get();
        ip6.ip6_hlim = IP6_DEFHLIM.load(Ordering::Relaxed) as u8;
        inp.inp_ipv6.set(ip6);
    } else {
        let mut ip = inp.inp_ip.get();
        ip.ip_ttl = IP_DEFTTL.load(Ordering::Relaxed) as u8;
        inp.inp_ip.set(ip);
    }
    // so->so_pcb for the requests (`sotoinpcb_locked`): in_pcballoc's handle, kept by the
    // socket's locked slot until udp_detach.
    let mut il = inet(w);
    drop(so.set_pcb_locked(&mut il.pcb, Some(PoolArc::erase(inp))));
    Ok(())
}

/// The pcb taken out of the socket's locked slot and detached (`in_pcbdetach`): the C's
/// `in_pcbdetach(inp)`, which clears `so_pcb`; `false` when the slot is empty.
fn udp_pcbdetach<G: ProcGlue>(so: &Socket<G>, il: &mut InetLocked<'_>) -> bool {
    let Some(slot) = so.set_pcb_locked(&mut il.pcb, None) else {
        return false;
    };
    let Some(h) = slot.downcast_ref::<Inpcb<G>>().and_then(PoolArc::from_ref) else {
        kpanic(format_args!("udp: socket {so:p}: its slot holds no pcb"));
    };
    in_pcbdetach(h, &il.net);
    drop(slot);
    true
}

/// `udp_detach`: the control block detached; `EINVAL` without one.
pub fn udp_detach<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);
    if !udp_pcbdetach(so, &mut il) {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `udp_bind`: the socket's local address (or a port picked).
pub fn udp_bind<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    addr: &Mbuf,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = udp_inp(so, &il.pcb);
    in_pcbbind(inp, Some(addr), p, &il.net)
}

/// `udp_connect`: the socket's peer; `EISCONN` when it has one.
pub fn udp_connect<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    addr: &Mbuf,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = udp_inp(so, &il.pcb);

    // INET6
    let connected = if inp.has_flags(INP_IPV6) {
        !in6_is_addr_unspecified(&inp.inp_faddr6.get::<In6Addr>())
    } else {
        inp.inp_faddr.get().s_addr != INADDR_ANY
    };
    if connected {
        return Err(Errno::EISCONN);
    }
    with_curproc::<G, _>("udp_connect", |p| in_pcbconnect(inp, addr, p, &il.net))?;

    soisconnected(so);
    Ok(())
}

/// `udp_disconnect`: the socket's peer and local address forgotten; `ENOTCONN` without a
/// peer.
pub fn udp_disconnect<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut il = inet(w);
    let detach = {
        let inp = udp_inp(so, &il.pcb);

        // INET6
        let unconnected = if inp.has_flags(INP_IPV6) {
            in6_is_addr_unspecified(&inp.inp_faddr6.get::<In6Addr>())
        } else {
            inp.inp_faddr.get().s_addr == INADDR_ANY
        };
        if unconnected {
            return Err(Errno::ENOTCONN);
        }
        in_pcbunset_laddr(inp);
        in_pcbdisconnect(inp)
    };
    if detach {
        // in_pcbdisconnect's in_pcbdetach of a socket without a file.
        udp_pcbdetach(so, &mut il);
    }
    so.st_clear(SS_ISCONNECTED); // XXX

    Ok(())
}

/// `udp_shutdown`: no more sending.
pub fn udp_shutdown<G: ProcGlue>(so: &Socket<G>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
    socantsendmore(so);
    Ok(())
}

/// `udp_send`: `udp_output` of the datagram `m` to `addr` (or the peer) with `control`,
/// the socket locked (the C's `soassertlocked_readonly`); `EINVAL` once the pcb is gone (PCB
/// could be destroyed, but socket still spliced). The data and the control are consumed.
pub fn udp_send<G: ProcGlue>(
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    m: Option<MbufBox>,
    addr: Option<&Mbuf>,
    control: Option<MbufBox>,
) -> Result<(), Errno> {
    let il = inet(w);
    let Some(inp) = sotoinpcb_locked(so, &il.pcb) else {
        // PCB could be destroyed, but socket still spliced.
        return Err(Errno::EINVAL);
    };

    // PIPEX: pipex_l2tp_userland_lookup_session and pipex_l2tp_userland_output; not
    // configured.

    // sosend always hands over a packet (a pkthdr mbuf, empty or not).
    let Some(m) = m else {
        return Err(Errno::EINVAL);
    };
    udp_output(inp, m, addr, control, &il.net)
}

/// `udp_sysctl`: sysctl for udp variables.
pub fn udp_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    // All sysctl names at this level are terminal.
    let [n] = *name else {
        return Err(Errno::ENOTDIR);
    };

    match n {
        UDPCTL_ROOTONLY | UDPCTL_BADDYNAMIC => {
            in_sysctl_ports(req, IPPROTO_UDP as u16, n == UDPCTL_ROOTONLY)
        }
        UDPCTL_STATS => {
            if req.newp != 0 {
                return Err(Errno::EPERM);
            }

            udp_sysctl_udpstat(req)
        }
        _ => bounded_arr(&UDPCTL_VARS, name, req),
    }
}

/// `udp_sysctl_udpstat`: `net.inet.udp.stats`, the per-CPU counters summed into a `struct
/// udpstat` in its members' order.
fn udp_sysctl_udpstat(req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    use UdpstatCounters as C;

    let c = UDPCOUNTERS.read();
    let udpstat = Udpstat {
        udps_ipackets: c[C::UdpsIpackets as usize],
        udps_hdrops: c[C::UdpsHdrops as usize],
        udps_badsum: c[C::UdpsBadsum as usize],
        udps_nosum: c[C::UdpsNosum as usize],
        udps_badlen: c[C::UdpsBadlen as usize],
        udps_noport: c[C::UdpsNoport as usize],
        udps_noportbcast: c[C::UdpsNoportbcast as usize],
        udps_nosec: c[C::UdpsNosec as usize],
        udps_fullsock: c[C::UdpsFullsock as usize],
        udps_pcbhashmiss: c[C::UdpsPcbhashmiss as usize],
        udps_inswcsum: c[C::UdpsInswcsum as usize],
        udps_opackets: c[C::UdpsOpackets as usize],
        udps_outswcsum: c[C::UdpsOutswcsum as usize],
    };

    rdstruct(req, bytes_of(core::slice::from_ref(&udpstat)))
}

/// `pr_ctloutput` of UDP's rows: the C's rows name `ip_ctloutput` (`ip6_ctloutput` for
/// `inet6sw[]`'s), which reach the pcb through `sotoinpcb(so)`; here the pcb borrowed from
/// the socket's locked slot (`panic(9)` without one, the C's NULL dereference) and, for IPv4,
/// its options lent, the socket lock held (`w`).
pub fn udp_ctloutput<G: ProcGlue>(
    op: i32,
    so: &Socket<G>,
    w: &mut SoLocked<'_>,
    level: i32,
    optname: i32,
    m: Option<&mut Mbuf>,
) -> Result<(), Errno> {
    let il = inet(w);
    let inp = udp_inp(so, &il.pcb);
    if inp.has_flags(INP_IPV6) {
        return ip6_ctloutput(op, inp, level, optname, m, &il.net);
    }
    let mut options = inp.options();
    ip_ctloutput(op, inp, &mut options, level, optname, m, &il.net)
}

impl<G: ProcGlue> Usrreqs<G> for UdpUsrreqs<G> {
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno> {
        udp_attach(so, w, proto, wait)
    }
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        udp_detach(so, w)
    }
    fn bind(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        nam: &Mbuf,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        udp_bind(so, w, nam, p)
    }
    fn connect(&self, so: &Socket<G>, w: &mut SoLocked<'_>, nam: &Mbuf) -> Result<(), Errno> {
        udp_connect(so, w, nam)
    }
    fn disconnect(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        udp_disconnect(so, w)
    }
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
        udp_shutdown(so, w)
    }
    /// NULL in the C's table: never called (UDP has no `PR_WANTRCVD`).
    fn rcvd(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("udp: pru_rcvd of a protocol without one"));
    }
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        udp_send(so, w, top, addr, control)
    }
    /// NULL in the C's table: never called (no connection queue holds a UDP socket).
    fn abort(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) {
        kpanic(format_args!("udp: pru_abort of a protocol without one"));
    }
    fn send_id(&self) -> usize {
        core::ptr::from_ref(&UDP_SEND_ID).addr()
    }
    fn control(
        &self,
        so: &Socket<G>,
        cmd: u64,
        data: &mut [u8],
        ifp: Option<&'static Ifnet>,
    ) -> Result<(), Errno> {
        if self.inet6 {
            in6_control(so.has_state(SS_PRIV), cmd, data, ifp)
        } else {
            in_control(so.has_state(SS_PRIV), cmd, data, ifp)
        }
    }
    /// `in_sockaddr` or `in6_sockaddr`.
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        let il = inet(w);
        let inp = udp_inp(so, &il.pcb);
        if self.inet6 {
            set_nam(addr, &in6_setsockaddr(inp));
        } else {
            set_nam(addr, &in_setsockaddr(inp));
        }
        Ok(())
    }
    /// `in_peeraddr` or `in6_peeraddr`.
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno> {
        let il = inet(w);
        let inp = udp_inp(so, &il.pcb);
        if self.inet6 {
            set_nam(addr, &in6_setpeeraddr(inp));
        } else {
            set_nam(addr, &in_setpeeraddr(inp));
        }
        Ok(())
    }
    /// `in_flowid`.
    fn flowid(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> i32 {
        let il = inet(w);
        i32::from(udp_inp(so, &il.pcb).inp_flowid.get())
    }
}
/// The host tests' UDP sockets: protocols of the test domains (`testinet`'s `InetDom` and
/// `Inet6Dom`) whose requests are [`UdpUsrreqs`], the tables of `udp_init`, the helpers the
/// tests of this module and of `udp6_output` share.
#[cfg(test)]
pub(crate) mod testudp {
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::netlock::{NETLOCK, NetHeld};
    use hal::sys::mbuf::{MT_CONTROL, MT_DATA, MT_SONAME};
    use hal::sys::socket::SOCK_DGRAM;

    use super::*;
    use crate::net::mbuf::{M_WAIT, m_get, m_gethdr};
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, PR_SPLICE, Protocol};
    use crate::net::sockbuf::SS_NOFDREF;
    use crate::net::socket::{SoInit, SocketRef, soalloc, solock};
    use crate::netinet::in_pcb::testinet;

    /// The fixture's glue.
    pub(crate) type G = testinet::G;

    /// UDP over the IPv4 test domain.
    pub(crate) struct UdpProto;
    /// UDP over the IPv6 test domain.
    pub(crate) struct Udp6Proto;

    impl Protocol<G> for UdpProto {
        fn domain(&self) -> &'static dyn crate::net::domain::Domain<G> {
            &testinet::InetDom
        }
        fn ty(&self) -> i16 {
            SOCK_DGRAM as i16
        }
        fn protocol(&self) -> i16 {
            IPPROTO_UDP as i16
        }
        fn flags(&self) -> i16 {
            PR_ATOMIC | PR_ADDR | PR_SPLICE
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
            Some(&UdpUsrreqs::<G>::INET)
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
            udp_ctloutput(op, so, w, level, optname, m)
        }
    }

    impl Protocol<G> for Udp6Proto {
        fn domain(&self) -> &'static dyn crate::net::domain::Domain<G> {
            &testinet::Inet6Dom
        }
        fn ty(&self) -> i16 {
            SOCK_DGRAM as i16
        }
        fn protocol(&self) -> i16 {
            IPPROTO_UDP as i16
        }
        fn flags(&self) -> i16 {
            PR_ATOMIC | PR_ADDR | PR_SPLICE
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
            Some(&UdpUsrreqs::<G>::INET6)
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
            udp_ctloutput(op, so, w, level, optname, m)
        }
    }

    /// `testinet::setup` and `udp_init` (the tables made again for the test).
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, &'static Thread<G>) {
        let r = testinet::setup();
        udp_init::<G>();
        r
    }

    /// Runs `f` with `so` locked as `solock` locks it (the exclusive net lock and the socket's
    /// witness); the answer is checked by the caller after the unlock.
    pub(crate) fn locked<R>(so: &Socket<G>, f: impl FnOnce(&mut SoLocked<'_>) -> R) -> R {
        let mut w = solock(so);
        f(&mut w)
    }

    /// A socket of `proto` (root's, `SS_PRIV`) with `udp_attach` done.
    pub(crate) fn socket(proto: &'static dyn Protocol<G>) -> SocketRef<G> {
        let so = soalloc(
            proto,
            M_WAIT,
            SoInit {
                so_type: SOCK_DGRAM,
                state: SS_PRIV,
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
        let r = locked(&so, |w| udp_attach(&so, w, 0, M_WAIT));
        r.expect("udp_attach");
        so
    }

    /// `udp_detach` of `so` and its release (no file holds it).
    pub(crate) fn close(so: SocketRef<G>) {
        so.st_set(SS_NOFDREF);
        let r = locked(&so, |w| udp_detach(&so, w));
        r.expect("udp_detach");
        drop(so);
    }

    /// `addr` in a name mbuf (`MT_SONAME`).
    pub(crate) fn nam<T: AbiPod>(addr: &T) -> MbufBox {
        let mut m = m_get(M_WAIT, MT_SONAME).expect("an mbuf");
        set_nam(&mut m, addr);
        m
    }

    /// The `sockaddr_in` of `a`:`port`.
    pub(crate) fn sin(a: [u8; 4], port: u16) -> SockaddrIn {
        SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_port: port.to_be(),
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(a),
            },
            ..SockaddrIn::default()
        }
    }

    /// The `sockaddr_in6` of `a`:`port`.
    pub(crate) fn sin6(a: In6Addr, port: u16) -> SockaddrIn6 {
        SockaddrIn6 {
            sin6_port: port.to_be(),
            ..SockaddrIn6::with_addr(a)
        }
    }

    /// `bytes` in a packet header mbuf (`MHLEN` at most).
    pub(crate) fn pkt(bytes: &[u8]) -> MbufBox {
        let mut m = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(bytes.len() as u32);
        m.m_pkthdr().len.set(bytes.len() as i32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        m
    }

    /// `udp_input` of `m` (its UDP header at `off`, family `af`) under the shared net lock,
    /// as `ip_deliver` calls it; the answer.
    pub(crate) fn input(m: MbufBox, off: i32, af: i32) -> i32 {
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut mp = Some(m);
        let mut off = off;
        let r = udp_input::<G>(&mut mp, &mut off, IPPROTO_UDP, af, None, &net);
        drop(w);
        r
    }

    /// A record of a receive buffer: the sender's address, the control messages (level,
    /// type and data of each) and the data.
    #[derive(Debug, Default, PartialEq, Eq)]
    pub(crate) struct Rec {
        /// The `MT_SONAME` mbuf's bytes.
        pub(crate) name: Vec<u8>,
        /// The `MT_CONTROL` mbufs: `(cmsg_level, cmsg_type, data)`.
        pub(crate) ctl: Vec<(i32, i32, Vec<u8>)>,
        /// The data mbufs' bytes.
        pub(crate) data: Vec<u8>,
    }

    /// The records of the receive buffer of `so`.
    pub(crate) fn records(so: &Socket<G>) -> Vec<Rec> {
        let g = so.so_rcv.lock();
        let mut v = Vec::new();
        for r in g.data().records() {
            let mut rec = Rec::default();
            for m in r {
                let d = m.data();
                match i32::from(m.m_type().get()) {
                    MT_SONAME => rec.name.extend_from_slice(d),
                    MT_CONTROL => {
                        let cm: Cmsghdr = pod::at(d, 0).expect("a cmsghdr");
                        let end = (cm.cmsg_len as usize).min(d.len());
                        rec.ctl.push((
                            cm.cmsg_level,
                            cm.cmsg_type,
                            d[CMSG_DATA_OFFSET..end].to_vec(),
                        ));
                    }
                    _ => rec.data.extend_from_slice(d),
                }
            }
            v.push(rec);
        }
        v
    }

    /// The one's complement sum of `b` (16-bit words, network order).
    fn sum16(b: &[u8], mut s: u32) -> u32 {
        for c in b.chunks(2) {
            let w = u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)]);
            s += u32::from(w);
        }
        s
    }

    /// The UDP checksum of the IPv4 packet `p` verifies (0 over the pseudo-header and the
    /// datagram), or the checksum is 0 (none).
    pub(crate) fn cksum4_ok(p: &[u8]) -> bool {
        let hl = usize::from(p[0] & 0x0f) * 4;
        let u = &p[hl..];
        if u[6..8] == [0, 0] {
            return true;
        }
        let mut s = sum16(&p[12..20], 0) + 17 + u.len() as u32;
        s = sum16(u, s);
        while s > 0xffff {
            s = (s & 0xffff) + (s >> 16);
        }
        s == 0xffff
    }

    /// The UDP checksum of the IPv6 packet `p` (no extension header) verifies.
    pub(crate) fn cksum6_ok(p: &[u8]) -> bool {
        let u = &p[40..];
        let mut s = sum16(&p[8..40], 0) + 17 + u.len() as u32;
        s = sum16(u, s);
        while s > 0xffff {
            s = (s & 0xffff) + (s >> 16);
        }
        s == 0xffff
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of UDP over the test interfaces (`testudp`'s sockets, whose requests are
    // these): bsd's four tests re-expressed (input to a bound socket and the errors, output
    // with its control messages and errors, the IPsec levels and the encapsulation port,
    // IPv6 input and its port unreachable), the pcb's slot; and N6.6b0's: the multicast
    // fan-out and its stop, the minimum TTL and a locked receiver, both control inputs, ESP in
    // UDP and the inbound policy on IPsec's test legs (`testtcp::sig_legs`), the upcall, the
    // sysctl names, bounds, statistics and bitmaps.

    use std::vec::Vec;

    use hal::sys::netinet_icmp6::Icmp6Hdr;
    use hal::sys::netinet_ip_icmp::ICMP_UNREACH_PORT;

    use super::testudp::{
        self, G, Rec, Udp6Proto, UdpProto, cksum4_ok, close, input, locked, nam, pkt, records, sin,
        sin6,
    };
    use super::*;
    use crate::net::mbuf::{M_WAIT, m_get, ml_dequeue};
    use crate::net::proto::{PRCO_GETOPT, PRCO_SETOPT, Protocol};
    use crate::net::socket::socantrcvmore;
    use crate::netinet::in_pcb::{in_baddynamic, sotoinpcb, testinet};
    use crate::netinet::ip_input::ip_send_test_take;
    use crate::netinet::tcp_subr::testtcp::{a6, iface6};
    use crate::netinet6::ip6_input::ip6_send_test_take;
    use hal::user::UserPtr;

    /// The counter `c`'s sum.
    fn stat(c: UdpstatCounters) -> u64 {
        UDPCOUNTERS.read()[c as usize]
    }

    /// An IPv4 datagram from `src`:`sport` to `dst`:`dport` with `payload`, its UDP length
    /// `ulen` (None: the true one) and checksum `sum` (network order); verified by the
    /// interface (`M_UDP_CSUM_IN_OK`) unless `sum` asks for the software check.
    fn dgram4(
        src: [u8; 4],
        sport: u16,
        dst: [u8; 4],
        dport: u16,
        payload: &[u8],
        ulen: Option<u16>,
        sum: Option<u16>,
    ) -> MbufBox {
        let len = 28 + payload.len();
        let mut ip = Ip {
            ip_len: (len as u16).to_be(),
            ip_ttl: 64,
            ip_p: IPPROTO_UDP as u8,
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
        let uh = Udphdr {
            uh_sport: sport.to_be(),
            uh_dport: dport.to_be(),
            uh_ulen: ulen.unwrap_or((8 + payload.len()) as u16).to_be(),
            uh_sum: sum.unwrap_or(0x1234).to_be(),
        };
        let mut p = std::vec![0u8; len];
        assert!(pod::put(&mut p, 0, &ip));
        assert!(pod::put(&mut p, 20, &uh));
        p[28..].copy_from_slice(payload);
        let m = pkt(&p);
        if sum.is_none() {
            let ph = m.m_pkthdr();
            ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_IN_OK);
        }
        m
    }

    /// The bytes of each packet `ip_send` queued since the last call.
    fn sent4() -> Vec<Vec<u8>> {
        let mut ml = ip_send_test_take();
        let mut v = Vec::new();
        while let Some(m) = ml_dequeue(&mut ml) {
            let mut b = std::vec![0u8; m.m_pkthdr().len.get() as usize];
            m_copydata(&m, 0, &mut b);
            v.push(b);
        }
        v
    }

    /// `bind(2)` of `so` to `n`.
    fn bind(so: &Socket<G>, n: &Mbuf) -> Result<(), Errno> {
        G::with_curproc(|p| {
            let p = p.expect("a curproc");
            locked(so, |w| udp_bind(so, w, n, p))
        })
    }

    /// `sendto(2)` of `data` on `so` to `to` with `control`.
    fn send(
        so: &Socket<G>,
        data: &[u8],
        to: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        locked(so, |w| udp_send(so, w, Some(pkt(data)), to, control))
    }

    /// One control message (`cmsg_len` the header's and `data`'s, aligned) in an mbuf.
    fn cmsg(level: i32, ty: i32, data: &[u8]) -> MbufBox {
        let cm = Cmsghdr {
            cmsg_len: cmsg_len(data.len()) as u32,
            cmsg_level: level,
            cmsg_type: ty,
        };
        let mut b = std::vec![0u8; cmsg_align(cmsg_len(data.len()))];
        assert!(pod::put(&mut b, 0, &cm));
        b[CMSG_DATA_OFFSET..CMSG_DATA_OFFSET + data.len()].copy_from_slice(data);
        let mut m = m_get(M_WAIT, hal::sys::mbuf::MT_CONTROL).expect("an mbuf");
        m.m_len().set(b.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(&b);
        m
    }

    /// The UDP header of the IPv4 packet `p`.
    fn udp_of(p: &[u8]) -> Udphdr {
        pod::at(p, usize::from(p[0] & 0x0f) * 4).expect("a UDP header")
    }

    #[test]
    fn datagrams_in_to_a_bound_socket_and_errors_for_the_others() {
        let (_g, _p) = testudp::setup();
        let n = 200;
        let (_ifp, _d) = testinet::iface(b"udpi0", [10, 9, n, 1]);
        let so = testudp::socket(&UdpProto);
        bind(&so, &nam(&sin([10, 9, n, 1], 5000))).expect("bind");
        let _ = sent4();

        // To the bound socket: the sender's address and the data, nothing else.
        let ipackets = stat(UdpstatCounters::UdpsIpackets);
        let r = input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5000,
                b"hello",
                None,
                None,
            ),
            20,
            AF_INET_I32,
        );
        assert_eq!(r, IPPROTO_DONE);
        assert_eq!(stat(UdpstatCounters::UdpsIpackets), ipackets + 1);
        let recs = records(&so);
        assert_eq!(recs.len(), 1);
        let from = bytes_of(core::slice::from_ref(&sin([10, 9, n, 2], 6000))).to_vec();
        assert_eq!(
            recs[0],
            Rec {
                name: from,
                ctl: Vec::new(),
                data: b"hello".to_vec(),
            }
        );

        // Destination port 0: no port, dropped.
        let noport = stat(UdpstatCounters::UdpsNoport);
        input(
            dgram4([10, 9, n, 2], 6000, [10, 9, n, 1], 0, b"x", None, None),
            20,
            AF_INET_I32,
        );
        assert_eq!(stat(UdpstatCounters::UdpsNoport), noport + 1);

        // A UDP length past the packet, or shorter than the header: bad length.
        let badlen = stat(UdpstatCounters::UdpsBadlen);
        input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5000,
                b"x",
                Some(20),
                None,
            ),
            20,
            AF_INET_I32,
        );
        input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5000,
                b"x",
                Some(7),
                None,
            ),
            20,
            AF_INET_I32,
        );
        assert_eq!(stat(UdpstatCounters::UdpsBadlen), badlen + 2);

        // A shorter UDP length trims the packet: "hel" delivered.
        input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5000,
                b"hello",
                Some(11),
                None,
            ),
            20,
            AF_INET_I32,
        );
        assert_eq!(records(&so)[1].data, b"hel".to_vec());

        // A wrong checksum checked in software: counted and dropped; the interface's verdict
        // of a bad one as well.
        let (badsum, inswcsum) = (
            stat(UdpstatCounters::UdpsBadsum),
            stat(UdpstatCounters::UdpsInswcsum),
        );
        input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5000,
                b"x",
                None,
                Some(0x1234),
            ),
            20,
            AF_INET_I32,
        );
        let bad = dgram4(
            [10, 9, n, 2],
            6000,
            [10, 9, n, 1],
            5000,
            b"x",
            None,
            Some(0x1234),
        );
        let ph = bad.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_IN_BAD);
        input(bad, 20, AF_INET_I32);
        assert_eq!(stat(UdpstatCounters::UdpsBadsum), badsum + 2);
        assert_eq!(stat(UdpstatCounters::UdpsInswcsum), inswcsum + 1);
        // No checksum at all over IPv4: counted, delivered.
        let nosum = stat(UdpstatCounters::UdpsNosum);
        input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5000,
                b"z",
                None,
                Some(0),
            ),
            20,
            AF_INET_I32,
        );
        assert_eq!(stat(UdpstatCounters::UdpsNosum), nosum + 1);
        assert_eq!(records(&so).len(), 3);

        // No socket on the port: a port unreachable quoting the datagram as it came.
        let (noport, miss) = (
            stat(UdpstatCounters::UdpsNoport),
            stat(UdpstatCounters::UdpsPcbhashmiss),
        );
        input(
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                5001,
                b"lost",
                None,
                None,
            ),
            20,
            AF_INET_I32,
        );
        assert_eq!(stat(UdpstatCounters::UdpsNoport), noport + 1);
        assert_eq!(stat(UdpstatCounters::UdpsPcbhashmiss), miss + 1);
        let out = sent4();
        assert_eq!(out.len(), 1, "one ICMP error");
        let icmp = &out[0][20..];
        assert_eq!((icmp[0], icmp[1]), (ICMP_UNREACH, ICMP_UNREACH_PORT));
        let quoted = udp_of(&icmp[8..]);
        assert_eq!(u16::from_be(quoted.uh_dport), 5001);
        assert_eq!(
            u16::from_be(quoted.uh_sum),
            0x1234,
            "the checksum as it came"
        );

        close(so);
    }

    #[test]
    fn datagrams_out_options_and_errors() {
        let (_g, _p) = testudp::setup();
        let n = 201;
        let (_ifp, d) = testinet::iface(b"udpo0", [10, 9, n, 1]);
        let so = testudp::socket(&UdpProto);
        let opackets = stat(UdpstatCounters::UdpsOpackets);

        // An unbound socket's sendto(2): a port picked, our address the source, the checksum
        // filled by in_proto_cksum_out.
        send(&so, b"ping", Some(&nam(&sin([10, 9, n, 2], 53))), None).expect("sendto");
        assert_eq!(stat(UdpstatCounters::UdpsOpackets), opackets + 1);
        let p = d.last.lock().expect("the capture").clone();
        let ip: Ip = pod::at(&p, 0).expect("an IP header");
        let uh = udp_of(&p);
        assert_eq!(ip.ip_src.s_addr.to_ne_bytes(), [10, 9, n, 1]);
        assert_eq!(ip.ip_dst.s_addr.to_ne_bytes(), [10, 9, n, 2]);
        assert_eq!(ip.ip_p, IPPROTO_UDP as u8);
        assert_eq!(ip.ip_ttl, IP_DEFTTL.load(Ordering::Relaxed) as u8);
        assert_eq!(u16::from_be(uh.uh_dport), 53);
        assert_eq!(u16::from_be(uh.uh_ulen), 12);
        let lport = uh.uh_sport;
        assert_ne!(lport, 0, "a port picked");
        let inp = sotoinpcb(&so).expect("its pcb");
        assert_eq!(inp.inp_lport.get(), lport);
        assert!(cksum4_ok(&p));
        assert_eq!(&p[28..], b"ping");

        // IP_SENDSRCADDR: an address not ours is refused; ours is the source.
        let r = send(
            &so,
            b"a",
            Some(&nam(&sin([10, 9, n, 2], 53))),
            Some(cmsg(IPPROTO_IP, IP_SENDSRCADDR, &[10, 9, n, 9])),
        );
        assert_eq!(r, Err(Errno::EADDRNOTAVAIL));
        send(
            &so,
            b"b",
            Some(&nam(&sin([10, 9, n, 2], 53))),
            Some(cmsg(IPPROTO_IP, IP_SENDSRCADDR, &[10, 9, n, 1])),
        )
        .expect("sendto with a source");

        // The control walk's refusals: two mbufs, a short header, a length past the buffer.
        let mut two = cmsg(IPPROTO_IP, IP_SENDSRCADDR, &[10, 9, n, 1]);
        drop(
            two.m_next
                .set(cmsg(IPPROTO_IP, IP_SENDSRCADDR, &[10, 9, n, 1])),
        );
        assert_eq!(
            send(&so, b"c", Some(&nam(&sin([10, 9, n, 2], 53))), Some(two)),
            Err(Errno::EINVAL)
        );
        let short = cmsg(IPPROTO_IP, IP_SENDSRCADDR, &[10, 9, n, 1]);
        short.m_len().set(8);
        assert_eq!(
            send(&so, b"c", Some(&nam(&sin([10, 9, n, 2], 53))), Some(short)),
            Err(Errno::EINVAL)
        );
        let mut long = cmsg(IPPROTO_IP, IP_SENDSRCADDR, &[10, 9, n, 1]);
        if let Some(d) = long.data_mut() {
            d[..4].copy_from_slice(&64u32.to_ne_bytes());
        }
        assert_eq!(
            send(&so, b"c", Some(&nam(&sin([10, 9, n, 2], 53))), Some(long)),
            Err(Errno::EINVAL)
        );

        // Port 0, a datagram too long, no destination unconnected.
        assert_eq!(
            send(&so, b"d", Some(&nam(&sin([10, 9, n, 2], 0))), None),
            Err(Errno::EADDRNOTAVAIL)
        );
        let big = pkt(b"e");
        big.m_pkthdr().len.set(65535 - 28 + 1);
        assert_eq!(
            locked(&so, |w| udp_send(&so, w, Some(big), None, None)),
            Err(Errno::EMSGSIZE)
        );
        assert_eq!(send(&so, b"f", None, None), Err(Errno::ENOTCONN));

        // Connected: no address needed, and one refused.
        locked(&so, |w| udp_connect(&so, w, &nam(&sin([10, 9, n, 2], 53)))).expect("connect");
        assert!(so.has_state(SS_ISCONNECTED));
        send(&so, b"g", None, None).expect("send");
        assert_eq!(udp_of(&d.last.lock().expect("the capture")).uh_sport, lport);
        assert_eq!(
            send(&so, b"h", Some(&nam(&sin([10, 9, n, 2], 53))), None),
            Err(Errno::EISCONN)
        );
        assert_eq!(
            locked(&so, |w| udp_connect(&so, w, &nam(&sin([10, 9, n, 3], 53)))),
            Err(Errno::EISCONN)
        );
        locked(&so, |w| udp_disconnect(&so, w)).expect("disconnect");
        assert!(!so.has_state(SS_ISCONNECTED));
        assert_eq!(
            locked(&so, |w| udp_disconnect(&so, w)),
            Err(Errno::ENOTCONN)
        );

        close(so);
    }

    #[test]
    fn ipsec_levels_and_the_udpencap_port() {
        use hal::sys::netinet_in::{
            IP_AUTH_LEVEL, IP_ESP_TRANS_LEVEL, IPSEC_LEVEL_BYPASS, IPSEC_LEVEL_DEFAULT,
            IPSEC_LEVEL_REQUIRE,
        };

        let (_g, _p) = testudp::setup();
        let so = testudp::socket(&UdpProto);
        let int_opt = |op: i32, name: i32, v: i32| -> Result<i32, Errno> {
            let mut m = m_get(M_WAIT, hal::sys::mbuf::MT_SOOPTS).expect("an mbuf");
            m.m_len().set(4);
            m.data_mut()
                .expect("writable")
                .copy_from_slice(&v.to_ne_bytes());
            locked(&so, |w| {
                UdpProto.ctloutput(op, &so, w, IPPROTO_IP, name, Some(&mut m))
            })?;
            Ok(pod::at::<i32>(m.data(), 0).unwrap_or(-1))
        };

        // in_pcballoc's defaults.
        assert_eq!(
            int_opt(PRCO_GETOPT, IP_AUTH_LEVEL, 0),
            Ok(IPSEC_LEVEL_DEFAULT)
        );
        // Root may set any level, the bypass included.
        for level in [IPSEC_LEVEL_REQUIRE, IPSEC_LEVEL_BYPASS] {
            int_opt(PRCO_SETOPT, IP_ESP_TRANS_LEVEL, level).expect("set");
            assert_eq!(int_opt(PRCO_GETOPT, IP_ESP_TRANS_LEVEL, 0), Ok(level));
        }
        let inp = sotoinpcb(&so).expect("attached");
        assert_eq!(
            i32::from(inp.inp_seclevel.get().sl_esp_trans),
            IPSEC_LEVEL_BYPASS
        );
        assert_eq!(
            int_opt(PRCO_SETOPT, IP_ESP_TRANS_LEVEL, 5),
            Err(Errno::EINVAL)
        );
        drop(inp);

        // udpencap_port is never handed out as a dynamic port.
        assert!(in_baddynamic(4500, IPPROTO_UDP as u16));
        assert!(!in_baddynamic(4501, IPPROTO_UDP as u16));

        close(so);
    }

    #[test]
    fn inet6_datagrams_in_to_a_bound_socket_and_a_port_unreachable() {
        let (_g, _p) = testudp::setup();
        let n = 202;
        let _d = iface6(b"udp6i0", n);
        let so = testudp::socket(&Udp6Proto);
        bind(&so, &nam(&sin6(a6(n, 1), 5000))).expect("bind");
        let mut ml = ip6_send_test_take();
        while ml_dequeue(&mut ml).is_some() {}

        let dgram6 = |dport: u16, sum: u16, payload: &[u8]| {
            let ip6 = Ip6Hdr {
                ip6_flow: 0x6000_0000u32.to_be(),
                ip6_plen: ((8 + payload.len()) as u16).to_be(),
                ip6_nxt: IPPROTO_UDP as u8,
                ip6_hlim: 64,
                ip6_src: a6(n, 2),
                ip6_dst: a6(n, 1),
            };
            let uh = Udphdr {
                uh_sport: 6000u16.to_be(),
                uh_dport: dport.to_be(),
                uh_ulen: ((8 + payload.len()) as u16).to_be(),
                uh_sum: sum.to_be(),
            };
            let mut p = std::vec![0u8; 48 + payload.len()];
            assert!(pod::put(&mut p, 0, &ip6));
            assert!(pod::put(&mut p, 40, &uh));
            p[48..].copy_from_slice(payload);
            let m = pkt(&p);
            let ph = m.m_pkthdr();
            ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_IN_OK);
            m
        };

        // To the bound socket, the sender's sockaddr_in6 as its name.
        input(dgram6(5000, 0x5555, b"six"), 40, AF_INET6_I32);
        let recs = records(&so);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].data, b"six".to_vec());
        let from: SockaddrIn6 = pod::at(&recs[0].name, 0).expect("a sockaddr_in6");
        assert_eq!(
            (from.sin6_family, u16::from_be(from.sin6_port)),
            (AF_INET6, 6000)
        );
        assert_eq!(from.sin6_addr, a6(n, 2));

        // In IPv6 the checksum is always used: none is dropped.
        let nosum = stat(UdpstatCounters::UdpsNosum);
        input(dgram6(5000, 0, b"none"), 40, AF_INET6_I32);
        assert_eq!(stat(UdpstatCounters::UdpsNosum), nosum + 1);
        assert_eq!(records(&so).len(), 1);

        // No socket: ICMPv6's port unreachable.
        let noport = stat(UdpstatCounters::UdpsNoport);
        input(dgram6(5001, 0x5555, b"lost"), 40, AF_INET6_I32);
        assert_eq!(stat(UdpstatCounters::UdpsNoport), noport + 1);
        let mut ml = ip6_send_test_take();
        let m = ml_dequeue(&mut ml).expect("an ICMPv6 error");
        let mut b = std::vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(&m, 0, &mut b);
        let icmp6: Icmp6Hdr = pod::at(&b, 40).expect("an ICMPv6 header");
        assert_eq!(
            (icmp6.icmp6_type, icmp6.icmp6_code),
            (ICMP6_DST_UNREACH, ICMP6_DST_UNREACH_NOPORT)
        );

        close(so);
    }

    #[test]
    fn the_requests_borrow_the_pcb_from_the_locked_slot() {
        let (_g, _p) = testudp::setup();
        let so = testudp::socket(&UdpProto);
        let inp = sotoinpcb(&so).expect("its pcb");
        assert_eq!(
            inp.inp_ip.get().ip_ttl,
            IP_DEFTTL.load(Ordering::Relaxed) as u8
        );
        let slot = |so: &Socket<G>| {
            locked(so, |w| {
                let il = w.inet().expect("a kernel socket");
                sotoinpcb_locked(so, &il.pcb).map(|i| core::ptr::from_ref(i).addr())
            })
        };
        assert_eq!(
            slot(&so),
            Some(core::ptr::from_ref::<Inpcb<G>>(&inp).addr())
        );
        // A second attach is refused.
        assert_eq!(
            locked(&so, |w| udp_attach(&so, w, 0, M_WAIT)),
            Err(Errno::EINVAL)
        );
        drop(inp);

        // After the detach no request finds the pcb: send's EINVAL, detach's EINVAL.
        so.st_set(crate::net::sockbuf::SS_NOFDREF);
        locked(&so, |w| udp_detach(&so, w)).expect("detach");
        assert_eq!(slot(&so), None);
        assert!(sotoinpcb(&so).is_none());
        assert_eq!(send(&so, b"x", None, None), Err(Errno::EINVAL));
        assert_eq!(locked(&so, |w| udp_detach(&so, w)), Err(Errno::EINVAL));
    }

    /// `ip_input`'s M_BCAST datagram (`dgram4` with the flag set).
    fn bcast(m: MbufBox) -> MbufBox {
        m.m_flags().set(m.m_flags().get() | M_BCAST);
        m
    }

    /// `setsockopt(so, SOL_SOCKET, opt, on)` (the socket's options word).
    fn sockopt(so: &Socket<G>, opt: i32, on: bool) {
        locked(so, |w| so.set_options(w, opt, on));
    }

    #[test]
    fn broadcast_reaches_every_sharing_socket_until_one_without_reuse() {
        let (_g, _p) = testudp::setup();
        let n = 203;
        // Three sockets sharing port 7000 (SO_REUSEPORT before each bind); the table walks
        // the last made first (in_pcballoc inserts at the head).
        let socks: Vec<_> = (0..3)
            .map(|_| {
                let so = testudp::socket(&UdpProto);
                sockopt(&so, SO_REUSEPORT, true);
                bind(&so, &nam(&sin([0; 4], 7000))).expect("bind");
                so
            })
            .collect();
        let d = || {
            dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 255],
                7000,
                b"all",
                None,
                None,
            )
        };

        input(bcast(d()), 20, AF_INET_I32);
        for so in &socks {
            let r = records(so);
            assert_eq!(r.len(), 1, "every sharing socket");
            assert_eq!(r[0].data, b"all".to_vec());
        }

        // The first socket the walk meets stops sharing: it alone gets the next one.
        sockopt(&socks[2], SO_REUSEPORT, false);
        input(bcast(d()), 20, AF_INET_I32);
        assert_eq!(records(&socks[2]).len(), 2, "the walk's first match");
        assert_eq!(
            records(&socks[1]).len(),
            1,
            "not reached after a socket without reuse"
        );
        assert_eq!(records(&socks[0]).len(), 1);

        // A socket that can receive no more is skipped; with no match the datagram is
        // counted and dropped, no ICMP error.
        let lone = testudp::socket(&UdpProto);
        bind(&lone, &nam(&sin([0; 4], 7001))).expect("bind");
        socantrcvmore(&lone);
        let (bcastmiss, noport) = (
            stat(UdpstatCounters::UdpsNoportbcast),
            stat(UdpstatCounters::UdpsNoport),
        );
        let _ = sent4();
        input(
            bcast(dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 255],
                7001,
                b"no",
                None,
                None,
            )),
            20,
            AF_INET_I32,
        );
        assert!(records(&lone).is_empty());
        assert_eq!(stat(UdpstatCounters::UdpsNoportbcast), bcastmiss + 1);
        assert_eq!(stat(UdpstatCounters::UdpsNoport), noport);
        assert!(sent4().is_empty());

        close(lone);
        for so in socks {
            close(so);
        }
    }

    #[test]
    fn the_minimum_ttl_and_a_locked_receiver() {
        let (_g, _p) = testudp::setup();
        let n = 204;
        let so = testudp::socket(&UdpProto);
        bind(&so, &nam(&sin([0; 4], 7002))).expect("bind");
        let inp = sotoinpcb(&so).expect("its pcb");

        // IP_MINTTL above the datagram's TTL (64): dropped, not counted as no port.
        inp.inp_ip_minttl.set(65);
        let noport = stat(UdpstatCounters::UdpsNoport);
        input(
            dgram4([10, 9, n, 2], 6000, [10, 9, n, 1], 7002, b"x", None, None),
            20,
            AF_INET_I32,
        );
        assert!(records(&so).is_empty());
        assert_eq!(stat(UdpstatCounters::UdpsNoport), noport);
        // And the multicast walk applies it too.
        input(
            bcast(dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 255],
                7002,
                b"x",
                None,
                None,
            )),
            20,
            AF_INET_I32,
        );
        assert!(records(&so).is_empty());
        inp.inp_ip_minttl.set(64);
        drop(inp);

        // Input takes no socket lock: a datagram arrives while this thread holds the
        // receiver's lock as a send does (the sender's own broadcast, looped back
        // synchronously in the C), the input under that holding's shared net lock.
        let held = crate::net::socket::solock_shared(&so);
        let r = {
            let net = held.net_proof().expect("the shared net lock");
            let mut mp = Some(dgram4(
                [10, 9, n, 2],
                6000,
                [10, 9, n, 1],
                7002,
                b"mine",
                None,
                None,
            ));
            let mut off = 20;
            udp_input::<G>(&mut mp, &mut off, IPPROTO_UDP, AF_INET_I32, None, &net)
        };
        drop(held);
        assert_eq!(r, IPPROTO_DONE);
        assert_eq!(records(&so)[0].data, b"mine".to_vec());

        close(so);
    }

    /// An ICMP message (type `ty`, code `code`, `icmp_nextmtu` `mtu`) returning the IPv4
    /// header of a datagram from `src`:`sport` (its bytes `sport_raw` as they are) to
    /// `dst`:`dport`, and the UDP ports: `CtlArg::Ip`'s bytes.
    fn quote(src: [u8; 4], sport_raw: [u8; 2], dst: [u8; 4], dport: u16, mtu: u16) -> Vec<u8> {
        let mut ip = Ip {
            ip_len: 28u16.to_be(),
            ip_ttl: 64,
            ip_p: IPPROTO_UDP as u8,
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
        let mut b = std::vec![3u8, 3, 0, 0, 0, 0];
        b.extend_from_slice(&mtu.to_be_bytes());
        let mut h = [0u8; 20];
        assert!(pod::put(&mut h, 0, &ip));
        b.extend_from_slice(&h);
        b.extend_from_slice(&sport_raw);
        b.extend_from_slice(&dport.to_be_bytes());
        b.extend_from_slice(&[0, 8, 0, 0]);
        b
    }

    /// `pr_ctlinput` of UDP under the shared net lock, as ICMP's input calls it.
    fn ctl4(cmd: i32, to: &SockaddrIn, arg: CtlArg<'_>) {
        let w = hal::netlock::NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let sa = bytes_of(core::slice::from_ref(to));
        udp_ctlinput::<G>(
            cmd,
            SockaddrView::new(sa).expect("an address"),
            0,
            arg,
            &net,
        );
    }

    #[test]
    fn the_control_inputs_tell_the_sockets_they_concern() {
        use crate::net::proto::PRC_UNREACH_PORT;

        let (_g, _p) = testudp::setup();
        let legs = crate::netinet::tcp_subr::testtcp::sig_legs();
        let n = 205;
        let (_ifp, _d) = testinet::iface(b"udpc0", [10, 9, n, 1]);
        let so = testudp::socket(&UdpProto);
        locked(&so, |w| udp_connect(&so, w, &nam(&sin([10, 9, n, 2], 53)))).expect("connect");
        let lport = sotoinpcb(&so).expect("its pcb").inp_lport.get();
        let peer = sin([10, 9, n, 2], 53);

        // A port unreachable quoting our datagram: the connected socket gets ECONNREFUSED,
        // collected by its next request; nothing else is locked twice (input holds no socket
        // lock, the error is set under in_pcbsolock's).
        let q = quote([10, 9, n, 1], lport.to_ne_bytes(), [10, 9, n, 2], 53, 0);
        ctl4(PRC_UNREACH_PORT, &peer, CtlArg::Ip(&q));
        assert_eq!(so.error(), Some(Errno::ECONNREFUSED));
        so.set_error(None);
        // A quote of another port reaches nobody.
        let other = quote([10, 9, n, 1], 4444u16.to_be_bytes(), [10, 9, n, 2], 53, 0);
        ctl4(PRC_UNREACH_PORT, &peer, CtlArg::Ip(&other));
        assert_eq!(so.error(), None);
        // A dead host: every socket talking to it (no quote).
        ctl4(PRC_HOSTDEAD, &peer, CtlArg::None);
        assert_eq!(so.error(), Some(Errno::EHOSTDOWN));
        so.set_error(None);
        // A short quote: nothing read past the message, nothing told.
        ctl4(PRC_UNREACH_PORT, &peer, CtlArg::Ip(&q[..30]));
        assert_eq!(so.error(), None);

        // UDP-encapsulated ESP's path MTU: the quote's source port compared with
        // udpencap_port as the C compares it, the 16-bit network value against the host's
        // (4500 = 0x1194: its bytes as they are in memory match, its network order does not).
        let before = legs.udpencap_ctl.load(Ordering::Relaxed);
        let raw = quote(
            [10, 9, n, 1],
            4500u16.to_ne_bytes(),
            [10, 9, n, 2],
            4500,
            1300,
        );
        ctl4(PRC_MSGSIZE, &peer, CtlArg::Ip(&raw));
        assert_eq!(
            legs.udpencap_ctl.load(Ordering::Relaxed),
            before + 1,
            "IPsec's leg"
        );
        let net = quote(
            [10, 9, n, 1],
            4500u16.to_be_bytes(),
            [10, 9, n, 2],
            4500,
            1300,
        );
        ctl4(PRC_MSGSIZE, &peer, CtlArg::Ip(&net));
        assert_eq!(
            legs.udpencap_ctl.load(Ordering::Relaxed),
            before + 1,
            "htons(port): the socket lookup instead"
        );

        close(so);
    }

    #[test]
    fn the_path_mtu_branch_is_ipsecs_only_with_its_legs() {
        let (_g, _p) = testudp::setup();
        let legs = crate::netinet::tcp_subr::testtcp::sig_legs();
        let n = 207;
        let raw = quote(
            [10, 9, n, 1],
            4500u16.to_ne_bytes(),
            [10, 9, n, 2],
            4500,
            1300,
        );
        // The quote's source port as udp_ctlinput reads it (ports_of: its bytes as they are).
        let sport = 4500u16;
        let before = legs.udpencap_ctl.load(Ordering::Relaxed);
        // Without the legs (the C without IPSEC has no such block): not taken, so
        // udp_ctlinput goes on to the socket lookup.
        assert!(!udpencap_pmtu(None, PRC_MSGSIZE, sport, 0, &raw));
        // With them: IPsec's, once; another message or another port is not.
        assert!(udpencap_pmtu(ipsec(), PRC_MSGSIZE, sport, 0, &raw));
        assert!(!udpencap_pmtu(ipsec(), PRC_HOSTDEAD, sport, 0, &raw));
        assert!(!udpencap_pmtu(
            ipsec(),
            PRC_MSGSIZE,
            4500u16.to_be(),
            0,
            &raw
        ));
        assert_eq!(legs.udpencap_ctl.load(Ordering::Relaxed), before + 1);
    }

    #[test]
    fn udp6_ctlinput_tells_the_connected_socket() {
        use crate::net::proto::PRC_UNREACH_PORT;
        use crate::netinet6::icmp6::Ip6CtlParam;

        let (_g, _p) = testudp::setup();
        let n = 206;
        let _d = iface6(b"udp6c0", n);
        let so = testudp::socket(&Udp6Proto);
        locked(&so, |w| udp_connect(&so, w, &nam(&sin6(a6(n, 2), 53)))).expect("connect");
        let lport = sotoinpcb(&so).expect("its pcb").inp_lport.get();

        // The quoted packet: our IPv6 header and the UDP ports after it.
        let ip6 = Ip6Hdr {
            ip6_flow: 0x6000_0000u32.to_be(),
            ip6_plen: 8u16.to_be(),
            ip6_nxt: IPPROTO_UDP as u8,
            ip6_hlim: 64,
            ip6_src: a6(n, 1),
            ip6_dst: a6(n, 2),
        };
        let mut b = std::vec![0u8; 48];
        assert!(pod::put(&mut b, 0, &ip6));
        b[40..42].copy_from_slice(&lport.to_ne_bytes());
        b[42..44].copy_from_slice(&53u16.to_be_bytes());
        let m = pkt(&b);
        let p = Ip6CtlParam {
            m: &m,
            icmp6: Default::default(),
            ip6,
            off: 40,
            src: sin6(a6(n, 1), 0),
            finaldst: core::cell::Cell::new(a6(n, 2)),
            cmdarg: None,
            nxt: IPPROTO_UDP as u8,
        };
        let to = sin6(a6(n, 2), 0);
        {
            let w = hal::netlock::NETLOCK.read();
            let net = NetHeld::of(&w).expect("the net lock");
            let sa = bytes_of(core::slice::from_ref(&to));
            let v = SockaddrView::new(sa).expect("an address");
            udp6_ctlinput::<G>(PRC_UNREACH_PORT, v, 0, CtlArg::Ip6(&p), &net);
            // An IPv4 quote is never this entry's: nothing.
            udp6_ctlinput::<G>(PRC_UNREACH_PORT, v, 0, CtlArg::Ip(&[0; 36]), &net);
        }
        assert_eq!(so.error(), Some(Errno::ECONNREFUSED));

        close(so);
    }

    /// A datagram to the encapsulation port 4500 carrying `body` (an SPI first).
    fn esp_in_udp(src: [u8; 4], body: &[u8]) -> MbufBox {
        dgram4(src, 4500, [10, 9, 72, 1], 4500, body, None, None)
    }

    #[test]
    fn esp_in_udp_goes_to_ipsec_unless_its_spi_is_zero() {
        let (_g, _p) = testudp::setup();
        let legs = crate::netinet::tcp_subr::testtcp::sig_legs();
        let so = testudp::socket(&UdpProto);
        bind(&so, &nam(&sin([0; 4], 4500))).expect("bind");
        let src = [10, 9, 72, 2];

        // A non-zero SPI: the UDP header removed (the IP header moved up over it), the ESP
        // packet handed to IPsec's leg at the IP header's end, protocol byte at ip_p.
        let before = legs.udpencap_in.lock().expect("the record").len();
        input(esp_in_udp(src, &[0, 0, 1, 0, 0xee, 0xee]), 20, AF_INET_I32);
        let got = legs.udpencap_in.lock().expect("the record")[before..].to_vec();
        assert_eq!(got.len(), 1, "IPsec's leg");
        let (skip, protoff, af, head) = got[0].clone();
        assert_eq!((skip, protoff, af), (20, 9, AF_INET_I32));
        assert_eq!(head[0], 0x45, "the IP header first");
        assert_eq!(&head[20..26], &[0, 0, 1, 0, 0xee, 0xee], "the SPI after it");
        assert!(records(&so).is_empty());

        // SPI 0: for the socket.
        input(esp_in_udp(src, &[0, 0, 0, 0, 1]), 20, AF_INET_I32);
        assert_eq!(records(&so).len(), 1);
        // Shorter than an SPI: dropped.
        input(esp_in_udp(src, &[1, 2, 3]), 20, AF_INET_I32);
        assert_eq!(records(&so).len(), 1);
        // pf diverted it: not IPsec's, whatever the SPI; the socket lookup instead (pf's
        // divert-to key, which the tests' kernel has no pf to give: no socket then).
        let m = esp_in_udp(src, &[0, 0, 1, 0, 2]);
        m.m_pkthdr().pf.flags.set(PF_TAG_DIVERTED);
        let miss = stat(UdpstatCounters::UdpsPcbhashmiss);
        input(m, 20, AF_INET_I32);
        assert_eq!(
            stat(UdpstatCounters::UdpsPcbhashmiss),
            miss + 1,
            "the lookup"
        );
        assert_eq!(records(&so).len(), 1);
        // ESP disabled: for the socket.
        ESP_ENABLE.store(0, Ordering::Relaxed);
        input(esp_in_udp(src, &[0, 0, 1, 0, 3]), 20, AF_INET_I32);
        ESP_ENABLE.store(1, Ordering::Relaxed);
        assert_eq!(records(&so).len(), 2);
        assert_eq!(
            legs.udpencap_in.lock().expect("the record").len(),
            before + 1
        );

        close(so);
    }

    #[test]
    fn the_inbound_policy_drops_or_gives_its_flow() {
        use crate::netinet::tcp_subr::testtcp::{SPD_DENY, SPD_FLOW, SPD_FLOW_ID};

        let (_g, _p) = testudp::setup();
        let _legs = crate::netinet::tcp_subr::testtcp::sig_legs();
        let so = testudp::socket(&UdpProto);
        bind(&so, &nam(&sin([0; 4], 7003))).expect("bind");
        let d = |src: [u8; 4]| dgram4(src, 6000, [10, 9, 72, 1], 7003, b"p", None, None);

        // Refused: counted, dropped.
        let nosec = stat(UdpstatCounters::UdpsNosec);
        input(d(SPD_DENY), 20, AF_INET_I32);
        assert_eq!(stat(UdpstatCounters::UdpsNosec), nosec + 1);
        assert!(records(&so).is_empty());

        // An SA's flow: a control message only for a socket that asked (IP_IPSECFLOWINFO).
        input(d(SPD_FLOW), 20, AF_INET_I32);
        assert!(records(&so)[0].ctl.is_empty());
        sotoinpcb(&so)
            .expect("its pcb")
            .set_flags(INP_IPSECFLOWINFO);
        input(d(SPD_FLOW), 20, AF_INET_I32);
        assert_eq!(
            records(&so)[1].ctl,
            std::vec![(
                IPPROTO_IP,
                IP_IPSECFLOWINFO,
                SPD_FLOW_ID.to_ne_bytes().to_vec()
            )]
        );
        // No SA, no flow: nothing.
        input(d([10, 9, 72, 3]), 20, AF_INET_I32);
        assert!(records(&so)[2].ctl.is_empty());

        close(so);
    }

    /// What the test upcall saw: its argument, the headers' length, the UDP source port and
    /// the IP source.
    static UPCALLS: std::sync::Mutex<Vec<(usize, i32, u16, [u8; 4])>> =
        std::sync::Mutex::new(Vec::new());

    /// A test `inp_upcall`: records its call, takes the packet for argument 1, gives it back
    /// for any other.
    fn upcall(
        arg: usize,
        m: MbufBox,
        ip: Option<&Ip>,
        _ip6: Option<&Ip6Hdr>,
        uh: &Udphdr,
        hlen: i32,
        _ns: Option<&mut Netstack>,
    ) -> Option<MbufBox> {
        let src = ip.map_or([0; 4], |ip| ip.ip_src.s_addr.to_ne_bytes());
        if let Ok(mut u) = UPCALLS.lock() {
            u.push((arg, hlen, u16::from_be(uh.uh_sport), src));
        }
        (arg != 1).then_some(m)
    }

    #[test]
    fn the_upcall_sees_the_datagram_first() {
        let (_g, _p) = testudp::setup();
        let n = 207;
        let taker = testudp::socket(&UdpProto);
        bind(&taker, &nam(&sin([0; 4], 7004))).expect("bind");
        let giver = testudp::socket(&UdpProto);
        bind(&giver, &nam(&sin([0; 4], 7005))).expect("bind");
        sotoinpcb(&taker)
            .expect("its pcb")
            .set_upcall(upcall, 1)
            .expect("set");
        sotoinpcb(&giver)
            .expect("its pcb")
            .set_upcall(upcall, 2)
            .expect("set");
        assert_eq!(
            sotoinpcb(&giver).expect("its pcb").set_upcall(upcall, 3),
            Err(Errno::EBUSY),
            "set once"
        );
        UPCALLS.lock().expect("the record").clear();

        input(
            dgram4([10, 9, n, 2], 6001, [10, 9, n, 1], 7004, b"t", None, None),
            20,
            AF_INET_I32,
        );
        input(
            dgram4([10, 9, n, 2], 6002, [10, 9, n, 1], 7005, b"g", None, None),
            20,
            AF_INET_I32,
        );
        assert_eq!(
            *UPCALLS.lock().expect("the record"),
            std::vec![(1, 28, 6001, [10, 9, n, 2]), (2, 28, 6002, [10, 9, n, 2])]
        );
        assert!(records(&taker).is_empty(), "taken by the upcall");
        assert_eq!(records(&giver)[0].data, b"g".to_vec(), "given back");

        close(taker);
        close(giver);
    }

    /// `net.inet.udp.<name>` as an `int`, `new` written first when given.
    fn int_sysctl(name: i32, u: usize, new: Option<i32>) -> Result<i32, Errno> {
        let mut len = size_of::<i32>();
        if let Some(v) = new {
            UserPtr::<i32>::new(u + 8).copyout(&v).expect("copyout");
        }
        let mut r = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: if new.is_some() { u + 8 } else { 0 },
            newlen: if new.is_some() { size_of::<i32>() } else { 0 },
        };
        udp_sysctl(&[name], &mut r)?;
        Ok(UserPtr::<i32>::new(u).copyin().expect("copyin"))
    }

    #[test]
    fn sysctl_names_bounds_stats_and_the_port_bitmaps() {
        use crate::netinet::in_pcb::{BADDYNAMICPORTS, DP_MAPSIZE, ROOTONLYPORTS};
        use crate::sysctl::SECURELEVEL;

        let (_g, _p) = testudp::setup();
        let u = crate::testutil::USER_BASE + 0x3_a000;

        // The bounded integers.
        assert_eq!(int_sysctl(UDPCTL_CHECKSUM, u, None), Ok(1));
        assert_eq!(int_sysctl(UDPCTL_CHECKSUM, u, Some(2)), Err(Errno::EINVAL));
        assert_eq!(int_sysctl(UDPCTL_CHECKSUM, u, Some(0)), Ok(1));
        assert_eq!(UDPCKSUM.load(Ordering::Relaxed), 0);
        assert_eq!(int_sysctl(UDPCTL_CHECKSUM, u, Some(1)), Ok(0));
        let rs = UDP_RECVSPACE.load(Ordering::Relaxed);
        assert_eq!(int_sysctl(UDPCTL_RECVSPACE, u, None), Ok(rs));
        assert_eq!(
            int_sysctl(UDPCTL_RECVSPACE, u, Some(SB_MAX as i32 + 1)),
            Err(Errno::EINVAL)
        );
        assert_eq!(int_sysctl(UDPCTL_SENDSPACE, u, None), Ok(9216));
        assert_eq!(int_sysctl(99, u, None), Err(Errno::EOPNOTSUPP));
        // Not terminal below a name.
        let mut len = 4;
        let mut r = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(udp_sysctl(&[UDPCTL_STATS, 1], &mut r), Err(Errno::ENOTDIR));

        // The statistics: struct udpstat, read only.
        udpstat_inc(UdpstatCounters::UdpsOpackets);
        let mut len = size_of::<Udpstat>();
        let mut r = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        udp_sysctl(&[UDPCTL_STATS], &mut r).expect("stats");
        assert_eq!(len, 104);
        let s: Udpstat = UserPtr::<Udpstat>::new(u).copyin().expect("copyin");
        assert_eq!(s.udps_opackets, stat(UdpstatCounters::UdpsOpackets));
        let mut r = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: u,
            newlen: 104,
        };
        assert_eq!(udp_sysctl(&[UDPCTL_STATS], &mut r), Err(Errno::EPERM));

        // The bitmaps: UDP's half read whole, a new one written under the exclusive net lock
        // (TCP's half untouched); rootonly refuses a new value at securelevel > 0.
        let buflen = DP_MAPSIZE * size_of::<u32>();
        let map = |name: i32, oldp: usize, newp: usize| -> Result<(), Errno> {
            let mut len = buflen;
            let mut r = SysctlReq {
                oldp,
                oldlenp: &mut len,
                newp,
                newlen: if newp != 0 { buflen } else { 0 },
            };
            udp_sysctl(&[name], &mut r)
        };
        let w0 = BADDYNAMICPORTS.udp[0].load(Ordering::Relaxed);
        let t0 = BADDYNAMICPORTS.tcp[0].load(Ordering::Relaxed);
        assert_eq!(map(UDPCTL_BADDYNAMIC, u, 0), Ok(()));
        assert_eq!(UserPtr::<u32>::new(u).copyin(), Ok(w0));
        UserPtr::<u32>::new(u)
            .copyout(&0x8000_0001)
            .expect("copyout");
        assert_eq!(map(UDPCTL_BADDYNAMIC, 0, u), Ok(()));
        assert_eq!(BADDYNAMICPORTS.udp[0].load(Ordering::Relaxed), 0x8000_0001);
        assert_eq!(
            BADDYNAMICPORTS.tcp[0].load(Ordering::Relaxed),
            t0,
            "TCP's half untouched"
        );
        UserPtr::<u32>::new(u).copyout(&w0).expect("copyout");
        assert_eq!(map(UDPCTL_BADDYNAMIC, 0, u), Ok(()));
        assert_eq!(BADDYNAMICPORTS.udp[0].load(Ordering::Relaxed), w0);
        let r0 = ROOTONLYPORTS.udp[0].load(Ordering::Relaxed);
        let level = SECURELEVEL.swap(1, Ordering::Relaxed);
        let refused = map(UDPCTL_ROOTONLY, 0, u);
        let read = map(UDPCTL_ROOTONLY, u, 0);
        SECURELEVEL.store(level, Ordering::Relaxed);
        assert_eq!((refused, read), (Err(Errno::EPERM), Ok(())));
        assert_eq!(ROOTONLYPORTS.udp[0].load(Ordering::Relaxed), r0);
    }
}
/* </TESTS> */
