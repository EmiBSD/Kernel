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

/*-
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
 *	@(#)protosw.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The protocol switch: `struct protosw` and `struct pr_usrreqs` (`<sys/protosw.h>`) as
//! traits (decision 46, docs/ZERO_UNSAFE.md 0.11).
//!
//! Upstream: sys/sys/protosw.h @ 3ce1f3f79392
//! LZ: sys/sys/protosw.rs@3c62ede68802
//!
//! Each protocol has a handle initializing one of these structures, which is used for
//! protocol-protocol and system-protocol communication. A protocol is called through the
//! `pr_init` entry before any other. Thereafter it is called every 200ms through the
//! `pr_fasttimo` entry and every 500ms through the `pr_slowtimo` for timer based actions.
//! Control information passes up on `pr_ctlinput`; the socket options reach a protocol
//! through `pr_ctloutput`, the user requests through its [`Usrreqs`] (`pr_usrreqs`). The
//! protocol is responsible for the space occupied by any of the arguments to these entries
//! and must dispose of it.
//!
//! ## Deviations
//! - `pr_ctlinput`'s argument is the bytes of the ICMP message that carries the offending
//!   IPv4 header (`&icp->icmp_ip` in the C, from which TCP reads the message's
//!   `icmp_nextmtu` back); IPv6's `struct ip6ctlparam` arrives with ICMPv6 (N6.5).
//! - The `prurequests`/`prcrequests`/`prcorequests` name tables (behind `PRUREQUESTS`, ...)
//!   are not compiled, as in GENERIC.
//!
//! ## Redesign
//! - LZ's `struct protosw` and `struct pr_usrreqs` were structs of `Option<fn>` over
//!   `&'static Socket` and `&'static Mbuf`, with the `pru_*` inline wrappers checking the
//!   NULLs. Here [`Protocol`] and [`Usrreqs`] are traits: a NULL entry is a default method
//!   that answers what the C's wrapper answers (`EOPNOTSUPP` for `bind`, `listen`,
//!   `connect`, `accept`, `disconnect`, `control`, `rcvoob` and `connect2`, 0 for `sense`,
//!   the data and control freed and `EOPNOTSUPP` for `sendoob`); the entries the wrappers
//!   call unchecked (`attach`, `detach`, `shutdown`, `rcvd`, `send`, `abort`, `sockaddr`,
//!   `peeraddr`, `flowid`) have no default. Every request but `control` takes the socket and
//!   the witness of its lock ([`SoLocked`]: the C's "called with the socket locked";
//!   `soo_ioctl` calls `pru_control` unlocked); the mbufs are owned
//!   ([`MbufBox`]) where the protocol disposes of them and borrowed where the caller keeps
//!   them (the address of `connect`, `bind`, `send`).
//! - `bsd`'s protocols implement the traits over their old tables (`bsd`'s `protosw` and
//!   `pr_usrreqs` themselves, in `sys/kern/uipc_domain.rs`; the `Protocol` row of
//!   `bridges.toml`) until each
//!   moves: inet at N6.4-N6.6, route at N6.3b, pfkey with IPsec at N8.

use sysdefs::base::errno::Errno;
use sysdefs::file::stat::Stat;

use hal::sync::netlock::NetHeld;

use crate::assert::kpanic;
use crate::net::domain::Domain;
use crate::net::if_var::{Ifnet, Netstack};
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::socket::{SoLocked, Socket};
use crate::netinet6::icmp6::Ip6CtlParam;
use crate::proc::{ProcGlue, Thread};

/// 2 slow timeouts per second.
pub const PR_SLOWHZ: i32 = 2;
/// 5 fast timeouts per second.
pub const PR_FASTHZ: i32 = 5;

// Values for pr_flags. PR_ADDR requires PR_ATOMIC; PR_ADDR and PR_CONNREQUIRED are mutually
// exclusive.

/// Exchange atomic messages only.
pub const PR_ATOMIC: i16 = 0x0001;
/// Addresses given with messages.
pub const PR_ADDR: i16 = 0x0002;
/// Connection required by protocol.
pub const PR_CONNREQUIRED: i16 = 0x0004;
/// Want `PRU_RCVD` calls.
pub const PR_WANTRCVD: i16 = 0x0008;
/// Passes capabilities.
pub const PR_RIGHTS: i16 = 0x0010;
/// Abort on accept(2) to disconnected socket.
pub const PR_ABRTACPTDIS: i16 = 0x0020;
/// Socket splicing is possible.
pub const PR_SPLICE: i16 = 0x0040;
/// Input runs with shared netlock.
pub const PR_MPINPUT: i16 = 0x0080;
/// mp-safe sysctl(2) handler.
pub const PR_MPSYSCTL: i16 = 0x0200;

// The requests of pr_usrreqs (`PRU_*`), as the protocols' traces name them (`tcp_trace`'s
// `req`).

/// `PRU_ATTACH`: attach protocol to up.
pub const PRU_ATTACH: i32 = 0;
/// `PRU_DETACH`: detach protocol from up.
pub const PRU_DETACH: i32 = 1;
/// `PRU_BIND`: bind socket to address.
pub const PRU_BIND: i32 = 2;
/// `PRU_LISTEN`: listen for connection.
pub const PRU_LISTEN: i32 = 3;
/// `PRU_CONNECT`: establish connection to peer.
pub const PRU_CONNECT: i32 = 4;
/// `PRU_ACCEPT`: accept connection from peer.
pub const PRU_ACCEPT: i32 = 5;
/// `PRU_DISCONNECT`: disconnect from peer.
pub const PRU_DISCONNECT: i32 = 6;
/// `PRU_SHUTDOWN`: won't send any more data.
pub const PRU_SHUTDOWN: i32 = 7;
/// `PRU_RCVD`: have taken data; more room now.
pub const PRU_RCVD: i32 = 8;
/// `PRU_SEND`: send this data.
pub const PRU_SEND: i32 = 9;
/// `PRU_ABORT`: abort (fast DISCONNECT, DETATCH).
pub const PRU_ABORT: i32 = 10;
/// `PRU_CONTROL`: control operations on protocol.
pub const PRU_CONTROL: i32 = 11;
/// `PRU_SENSE`: return status into m.
pub const PRU_SENSE: i32 = 12;
/// `PRU_RCVOOB`: retrieve out of band data.
pub const PRU_RCVOOB: i32 = 13;
/// `PRU_SENDOOB`: send out of band data.
pub const PRU_SENDOOB: i32 = 14;
/// `PRU_SOCKADDR`: fetch socket's address.
pub const PRU_SOCKADDR: i32 = 15;
/// `PRU_PEERADDR`: fetch peer's address.
pub const PRU_PEERADDR: i32 = 16;

// The commands of pr_ctlinput (`PRC_*`).

/// Route changed.
pub const PRC_ROUTEDEAD: i32 = 1;
/// Mtu increased.
pub const PRC_MTUINC: i32 = 2;
/// Some said to slow down.
pub const PRC_QUENCH2: i32 = 3;
/// Some said to slow down.
pub const PRC_QUENCH: i32 = 4;
/// Message size forced drop.
pub const PRC_MSGSIZE: i32 = 5;
/// Host appears to be down.
pub const PRC_HOSTDEAD: i32 = 6;
/// Deprecated (use PRC_UNREACH_HOST).
pub const PRC_HOSTUNREACH: i32 = 7;
/// No route to network.
pub const PRC_UNREACH_NET: i32 = 8;
/// No route to host.
pub const PRC_UNREACH_HOST: i32 = 9;
/// Dst says bad protocol.
pub const PRC_UNREACH_PROTOCOL: i32 = 10;
/// Bad port #.
pub const PRC_UNREACH_PORT: i32 = 11;
/// (use PRC_MSGSIZE) 12 not used.
pub const PRC_UNREACH_NEEDFRAG: i32 = 12;
/// Source route failed.
pub const PRC_UNREACH_SRCFAIL: i32 = 13;
/// Net routing redirect.
pub const PRC_REDIRECT_NET: i32 = 14;
/// Host routing redirect.
pub const PRC_REDIRECT_HOST: i32 = 15;
/// Redirect for type of service & net.
pub const PRC_REDIRECT_TOSNET: i32 = 16;
/// Redirect for tos & host.
pub const PRC_REDIRECT_TOSHOST: i32 = 17;
/// Packet lifetime expired in transit.
pub const PRC_TIMXCEED_INTRANS: i32 = 18;
/// Lifetime expired on reass q.
pub const PRC_TIMXCEED_REASS: i32 = 19;
/// Header incorrect.
pub const PRC_PARAMPROB: i32 = 20;
/// The number of commands.
pub const PRC_NCMDS: usize = 21;

/// `PRC_IS_REDIRECT(cmd)`: whether control command `cmd` is a redirect.
#[inline]
pub const fn prc_is_redirect(cmd: i32) -> bool {
    cmd >= PRC_REDIRECT_NET && cmd <= PRC_REDIRECT_TOSHOST
}

// The requests of pr_ctloutput.

/// `PRCO_GETOPT`.
pub const PRCO_GETOPT: i32 = 0;
/// `PRCO_SETOPT`.
pub const PRCO_SETOPT: i32 = 1;

/// A socket address borrowed from the bytes that hold it (an `MT_SONAME` mbuf's data, a
/// `struct sockaddr` a caller built): at most `sa_len` of them, at least two (`sa_len`,
/// `sa_family`).
#[derive(Clone, Copy)]
pub struct SockaddrView<'a>(&'a [u8]);

impl<'a> SockaddrView<'a> {
    /// The address in `bytes`, cut to its `sa_len`; `None` when the bytes are shorter than
    /// the two header bytes or than `sa_len`.
    pub fn new(bytes: &'a [u8]) -> Option<Self> {
        let len = usize::from(*bytes.first()?);
        if bytes.len() < 2 {
            return None;
        }
        bytes.get(..len.max(2)).map(Self)
    }

    /// `sa_len`.
    pub fn len(&self) -> u8 {
        self.0[0]
    }

    /// Whether the address holds no byte past its header.
    pub fn is_empty(&self) -> bool {
        self.0.len() <= 2
    }

    /// `sa_family`.
    pub fn family(&self) -> u8 {
        self.0[1]
    }

    /// The address's bytes.
    pub fn bytes(&self) -> &'a [u8] {
        self.0
    }
}

/// The argument of a protocol's control input (`pr_ctlinput`'s `void *`, whose type the C
/// gives by family): nothing (the C's NULL: `pfctlinput`, a path MTU increase), the ICMP
/// message that returned an IPv4 header (from its type byte; the C's `&icp->icmp_ip` is
/// `ICMP_MINLEN` into it), or the ICMPv6 error's [`Ip6CtlParam`] (`struct ip6ctlparam`).
/// A protocol reads the argument of its own family only.
#[derive(Clone, Copy)]
pub enum CtlArg<'a> {
    /// NULL.
    None,
    /// An ICMP message quoting an IPv4 header (`AF_INET`'s argument).
    Ip(&'a [u8]),
    /// An ICMPv6 error's parameter (`AF_INET6`'s argument).
    Ip6(&'a Ip6CtlParam<'a>),
}

/// `struct protosw`: a protocol of a domain.
pub trait Protocol<G: ProcGlue>: Sync + 'static {
    /// `pr_domain`: domain protocol a member of.
    fn domain(&self) -> &'static dyn Domain<G>;
    /// `pr_type`: socket type used for.
    fn ty(&self) -> i16;
    /// `pr_protocol`: protocol number.
    fn protocol(&self) -> i16;
    /// `pr_flags`: see `PR_*`.
    fn flags(&self) -> i16;
    /// `pr_usrreqs`: the user requests; `None` for an entry sockets cannot be made of
    /// (`socreate`'s `EPROTONOSUPPORT`).
    fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>>;
    /// Whether the protocol's requests are `bsd`'s (its `struct protosw`'s impl; since N6.6b1
    /// only pfkey's, `bsd`'s raw rows answering with the kernel's raw IP): they release and
    /// take the socket lock again C-style (`pfkeyv2_send`), so the kernel holds a socket of the
    /// protocol by a detached holding of `so_lock`, never a witness. A socket records it at
    /// `soalloc` ([`Socket`]'s `legacy_lock`), for its life.
    /// The kernel's own protocols answer `false`: their sockets are held by a witness of
    /// `so_lock`, an internet one's under either holding of the net lock (D-1, N6.6a0).
    fn legacy(&self) -> bool {
        false
    }
    /// `pr_ctlinput(cmd, sa, rdomain, arg)`: control input (from below), the argument by its
    /// family ([`CtlArg`]), the caller's proof of the net lock `net` (its callers hold it:
    /// ICMP's and ICMPv6's input, `pfctlinput`; TCP's control input locks sockets under it,
    /// N6.6a5). Nothing when the C's entry is NULL.
    fn ctlinput(
        &self,
        _cmd: i32,
        _sa: SockaddrView<'_>,
        _rdomain: u32,
        _arg: CtlArg<'_>,
        _net: &NetHeld<'_>,
    ) {
    }
    /// `pr_input(mp, offp, proto, af, ns)`: the protocol's input of the packet `*mp` whose
    /// header is at `*offp`, under the net lock (`net`); `IPPROTO_DONE` once the packet is
    /// consumed, or the next protocol with `*mp` and `*offp` at its header. The C's NULL entry
    /// is never called (`ip_deliver` reaches only entries that have one): `panic(9)`.
    #[allow(
        clippy::too_many_arguments,
        reason = "the C's arguments and the net lock's proof"
    )]
    fn input(
        &self,
        _mp: &mut Option<MbufBox>,
        _offp: &mut i32,
        _proto: i32,
        _af: i32,
        _ns: Option<&mut Netstack>,
        _net: &NetHeld<'_>,
    ) -> i32 {
        kpanic(format_args!("pr_input of a protocol without one"))
    }
    /// Whether `pr_ctloutput` is set (`sosetopt`, `sogetopt`: `ENOPROTOOPT` without it, no
    /// lock taken).
    fn has_ctloutput(&self) -> bool {
        false
    }
    /// `pr_ctloutput(op, so, level, optname, m)`: control output (from above), the socket
    /// locked; `m` the option's mbuf (`None` for a `setsockopt` without a value).
    fn ctloutput(
        &self,
        _op: i32,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        _level: i32,
        _optname: i32,
        _m: Option<&mut Mbuf>,
    ) -> Result<(), Errno> {
        Err(Errno::ENOPROTOOPT)
    }
    /// Whether `pr_sysctl` is set (`net_sysctl`: the next protocol of the number is asked
    /// otherwise, `ENOPROTOOPT` after the last).
    fn has_sysctl(&self) -> bool {
        false
    }
    /// `pr_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the protocol's sysctl node
    /// (`net.<family>.<protocol>.*`), asked only of a protocol that has one.
    fn sysctl(
        &self,
        _name: &[i32],
        _req: &mut crate::base::sysctl::SysctlReq<'_>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pr_init`: initialization hook.
    fn init(&self) {}
    /// `pr_fasttimo`: fast timeout (200ms).
    fn fasttimo(&self) {}
    /// `pr_slowtimo`: slow timeout (500ms).
    fn slowtimo(&self) {}
}

/// `struct pr_usrreqs`: the user requests of a protocol, each called with the socket locked
/// (`w`, the witness of [`solock`](crate::net::socket::solock) or its shared form).
#[allow(clippy::too_many_arguments, reason = "the C's argument lists")]
pub trait Usrreqs<G: ProcGlue>: Sync + 'static {
    /// `pru_attach(so, proto, wait)`.
    fn attach(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        proto: i32,
        wait: i32,
    ) -> Result<(), Errno>;
    /// `pru_detach(so)`.
    fn detach(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno>;
    /// `pru_bind(so, nam, p)`.
    fn bind(
        &self,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        _nam: &Mbuf,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_listen(so)`.
    fn listen(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_connect(so, nam)`.
    fn connect(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>, _nam: &Mbuf) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_accept(so, nam)`: the peer's address into `nam`.
    fn accept(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>, _nam: &mut Mbuf) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_disconnect(so)`.
    fn disconnect(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_shutdown(so)`.
    fn shutdown(&self, so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno>;
    /// `pru_rcvd(so)`.
    fn rcvd(&self, so: &Socket<G>, w: &mut SoLocked<'_>);
    /// `pru_send(so, top, addr, control)`: the protocol disposes of the data and the control;
    /// the address is the caller's.
    fn send(
        &self,
        so: &Socket<G>,
        w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno>;
    /// `pru_abort(so)`.
    fn abort(&self, so: &Socket<G>, w: &mut SoLocked<'_>);
    /// The identity of `pru_send` (`sosplice`'s `pru_send != pru_send` test: two tables
    /// that share their send function splice into each other, tcp's and tcp6's). A protocol
    /// that splices (`PR_SPLICE`) answers its send function's address; the others are never
    /// asked.
    fn send_id(&self) -> usize {
        0
    }
    /// `pru_control(so, cmd, data, ifp)`: a protocol `ioctl`; `data` is the kernel copy of
    /// the argument, `ifp` the interface of an interface request (`ifioctl`'s; `None` from
    /// `soo_ioctl`). Called without the socket's lock, as `soo_ioctl` and `ifioctl` call it
    /// (`in_control` takes the net lock itself).
    fn control(
        &self,
        _so: &Socket<G>,
        _cmd: u64,
        _data: &mut [u8],
        _ifp: Option<&'static Ifnet>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_sense(so, ub)`.
    fn sense(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>, _ub: &mut Stat) -> Result<(), Errno> {
        Ok(())
    }
    /// `pru_rcvoob(so, m, flags)`.
    fn rcvoob(
        &self,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        _m: &mut Mbuf,
        _flags: i32,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_sendoob(so, top, addr, control)`: without the request the data and the control
    /// are freed (the C's wrapper).
    fn sendoob(
        &self,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        top: Option<MbufBox>,
        _addr: Option<&Mbuf>,
        control: Option<MbufBox>,
    ) -> Result<(), Errno> {
        drop(top);
        drop(control);
        Err(Errno::EOPNOTSUPP)
    }
    /// `pru_sockaddr(so, addr)`.
    fn sockaddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno>;
    /// `pru_peeraddr(so, addr)`.
    fn peeraddr(&self, so: &Socket<G>, w: &mut SoLocked<'_>, addr: &mut Mbuf) -> Result<(), Errno>;
    /// `pru_flowid(so)`: asked only of a protocol that splices (`PR_SPLICE`, `sosplice`'s
    /// task queue); the C calls the entry unchecked, so a protocol without one is never
    /// asked.
    fn flowid(&self, _so: &Socket<G>, _w: &mut SoLocked<'_>) -> i32 {
        kpanic(format_args!(
            "pru_flowid of a protocol that does not splice"
        ))
    }
    /// `pru_connect2(so1, so2)`, both locked (`solock_pair`).
    fn connect2(
        &self,
        _so: &Socket<G>,
        _w: &mut SoLocked<'_>,
        _so2: &Socket<G>,
        _w2: Option<&mut SoLocked<'_>>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::{assert, assert_eq};

    use super::*;
    use crate::net::domain::Domain;

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// The tests' family.
    const FAMILY: i32 = 9;

    struct TestDom;
    struct TestProto {
        ty: i16,
        protocol: i16,
    }

    static DGRAM: TestProto = TestProto { ty: 2, protocol: 0 };
    static STREAM: TestProto = TestProto { ty: 1, protocol: 6 };
    static RAW: TestProto = TestProto { ty: 3, protocol: 0 };
    static PROTOS: [&dyn Protocol<Procs>; 3] = [&DGRAM, &STREAM, &RAW];

    impl Domain<Procs> for TestDom {
        fn family(&self) -> i32 {
            FAMILY
        }
        fn name(&self) -> &'static str {
            "testdom"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<Procs>] {
            &PROTOS
        }
    }

    impl Protocol<Procs> for TestProto {
        fn domain(&self) -> &'static dyn Domain<Procs> {
            &TestDom
        }
        fn ty(&self) -> i16 {
            self.ty
        }
        fn protocol(&self) -> i16 {
            self.protocol
        }
        fn flags(&self) -> i16 {
            0
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<Procs>> {
            None
        }
    }

    #[test]
    fn a_protocol_s_absent_entries_answer_as_the_c_s_null_ones() {
        // pr_ctloutput NULL: no ctloutput; the timers and ctlinput do nothing.
        assert!(!STREAM.has_ctloutput());
        STREAM.init();
        STREAM.fasttimo();
        STREAM.slowtimo();
        assert_eq!(STREAM.domain().family(), FAMILY);
        assert!(STREAM.usrreqs().is_none());
        assert_eq!((DGRAM.ty(), RAW.ty(), STREAM.protocol()), (2, 3, 6));
    }
}
/* </TESTS> */
