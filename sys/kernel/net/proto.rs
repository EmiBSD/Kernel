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
//! - `pr_input` is not a method yet: its arguments are `struct netstack` and the packet
//!   path, N6.3a/N6.4's types, and no kernel code calls it before N6.4; `bsd`'s `ip_input`
//!   dispatches through its own tables (D-N6.2-4). `pr_ctlinput`'s family argument (`struct
//!   ip *`, `ip6ctlparam`) arrives with its callers at N6.4 too: the one kernel use,
//!   `pfctlinput`, passes NULL.
//! - `pru_control`'s `struct ifnet *` is always NULL from `soo_ioctl` and the type is
//!   N6.3a's: the parameter comes with it.
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
//! - `bsd`'s protocols implement the traits over their old tables (`bsd`'s
//!   `LegacyProtocol`/`LegacyUsrreqs`, the `Protocol` row of `bridges.toml`) until each
//!   moves: inet at N6.4-N6.6, route at N6.3b, pfkey with IPsec at N8.

use hal::sys::errno::Errno;
use hal::sys::stat::Stat;

use crate::assert::kpanic;
use crate::net::domain::Domain;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::socket::{SoLocked, Socket};
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
    /// `pr_ctlinput(cmd, sa, rdomain, NULL)`: control input (from below); nothing when the C's
    /// entry is NULL.
    fn ctlinput(&self, _cmd: i32, _sa: SockaddrView<'_>, _rdomain: u32) {}
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
    /// `pru_control(so, cmd, data, NULL)`: a protocol `ioctl`; `data` is the kernel copy of
    /// the argument. Called without the socket's lock, as `soo_ioctl` calls it (`in_control`
    /// takes the net lock itself).
    fn control(&self, _so: &Socket<G>, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
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
