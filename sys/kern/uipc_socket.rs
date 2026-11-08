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
 *	@(#)uipc_socket.c	8.3 (Berkeley) 4/15/94
 */
/* </LICENSES> */

/* <CODE> */
//! Socket operations: `kern/uipc_socket.c`, the kernel's (`kernel::net::socket`) under LZ's
//! names and signatures for `bsd`'s callers (nfs's kernel sockets, wg's and pflow's, the
//! system calls until N6.2b, `bsd`'s host tests).
//!
//! Upstream: sys/kern/uipc_socket.c @ 3ce1f3f79392
//!
//! A socket `bsd` makes with [`socreate`] is held by a reference parked on it (the C's
//! file reference of a socket a kernel subsystem owns), which [`soclose`] takes back after
//! the kernel's `soclose`. The mbufs `bsd` keeps by address are unparked into the kernel's
//! calls and parked again for the ones it hands back. The requests the C makes with the
//! socket locked (`sobind`, `soconnect`, `solisten`, `soaccept`) lend the caller's C-style
//! holding to the kernel (`solocked_legacy`).
//!
//! ## Deviations
//! - `socreate` and `sosetopt` act for `curproc` (the C's `socreate` uses it, `sosetopt`'s
//!   `SO_SPLICE` its thread's descriptors).

use kernel::net::mbuf::MbufBox;

pub use kernel::net::socket::{
    SOMAXCONN_VAR as somaxconn, SOMINCONN_VAR as sominconn, sohasoutofband, sorwakeup, sowwakeup,
};

pub use crate::sys::socketvar::sorele;

use crate::kern::proc_glue::BsdProcGlue as G;
use crate::kern::subr_prf::panic;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, mut_of, park, unpark};
use crate::sys::proc::Proc;
use crate::sys::socketvar::{Socket, as_static};
use crate::sys::uio::Uio;

/// `curproc`, which the calls below act for.
fn curproc_or_panic(func: &str) -> &'static Proc {
    match crate::machine::cpu::curproc() {
        Some(p) => p,
        None => panic(format_args!("{func}: no curproc")),
    }
}

/// `soinit`: the socket and splice pools.
pub fn soinit() {
    kernel::net::domain::soinit::<G>();
}

/// `soalloc(prp, wait)` of `bsd`'s protocol tests: a socket of `prp` of type `so_type` (the
/// kernel's sets the type at allocation, which LZ's tests wrote after), its existence
/// reference parked as [`socreate`]'s.
#[cfg(test)]
pub(crate) fn soalloc(
    prp: &'static crate::sys::protosw::Protosw,
    so_type: i32,
    wait: i32,
) -> Option<&'static Socket> {
    let so = kernel::net::socket::soalloc(
        prp,
        wait,
        kernel::net::socket::SoInit {
            so_type,
            state: 0,
            options: 0,
            linger: 0,
            timeo: 0,
            euid: 0,
            ruid: 0,
            egid: 0,
            rgid: 0,
            cpid: 0,
        },
    )?;
    let r = as_static(&so);
    hal::pool::PoolArc::forget(so);
    Some(r)
}

/// `socreate(dom, aso, type, proto)` for `curproc`: the socket, its reference parked on it
/// for [`soclose`].
pub fn socreate(dom: i32, type_: i32, proto: i32) -> Result<&'static Socket, Errno> {
    let p = curproc_or_panic("socreate");
    let so = kernel::net::socket::socreate(p, dom, type_, proto)?;
    let r = as_static(&so);
    hal::pool::PoolArc::forget(so);
    Ok(r)
}

/// `soclose(so, flags)` of a socket [`socreate`] made: the kernel's `soclose`, then the
/// parked reference given back.
pub fn soclose(so: &Socket, flags: i32) -> Result<(), Errno> {
    let error = kernel::net::socket::soclose(so, flags);
    match hal::pool::PoolArc::adopt(so) {
        Some(r) => drop(r),
        None => panic(format_args!("soclose: socket {so:p} not socreate'd")),
    }
    error
}

/// `sofree(so, keep_lock)` of `bsd`'s protocols (`in_pcbdetach`), which hold the socket's
/// lock C-style: `keep_lock` must be set (the only form `bsd` calls).
pub fn sofree(so: &Socket, keep_lock: bool) {
    if !keep_lock {
        panic(format_args!("sofree: bsd's caller lets the lock go"));
    }
    kernel::net::socket::sofree_legacy(so);
}

/// `sobind(so, nam, p)`, the socket locked by the caller.
pub fn sobind(so: &Socket, nam: &Mbuf, p: &Proc) -> Result<(), Errno> {
    let mut w = kernel::net::socket::solocked_legacy(so);
    kernel::net::socket::sobind(so, &mut w, nam, p)
}

/// `solisten(so, backlog)`, the socket locked by the caller.
pub fn solisten(so: &Socket, backlog: i32) -> Result<(), Errno> {
    let mut w = kernel::net::socket::solocked_legacy(so);
    kernel::net::socket::solisten(so, &mut w, backlog)
}

/// `soaccept(so, nam)`, the socket locked by the caller.
pub fn soaccept(so: &Socket, nam: &Mbuf) -> Result<(), Errno> {
    let mut w = kernel::net::socket::solocked_legacy(so);
    kernel::net::socket::soaccept(so, &mut w, mut_of(nam))
}

/// `soconnect(so, nam)`, the socket locked by the caller.
pub fn soconnect(so: &Socket, nam: &Mbuf) -> Result<(), Errno> {
    let mut w = kernel::net::socket::solocked_legacy(so);
    kernel::net::socket::soconnect(so, &mut w, nam)
}

/// `soconnect2(so1, so2)`.
pub fn soconnect2(so1: &Socket, so2: &Socket) -> Result<(), Errno> {
    kernel::net::socket::soconnect2(so1, so2)
}

/// `sosend(so, addr, uio, top, control, flags)`: the data, the control and the address are
/// the caller's parked mbufs; the data and the control are freed on return.
pub fn sosend(
    so: &Socket,
    addr: Option<&'static Mbuf>,
    uio: Option<&mut Uio<'_>>,
    top: Option<&'static Mbuf>,
    control: Option<&'static Mbuf>,
    flags: i32,
) -> Result<(), Errno> {
    kernel::net::socket::sosend(so, addr, uio, top.map(unpark), control.map(unpark), flags)
}

/// The mbuf the kernel handed out into the caller's slot, parked.
fn park_into(slot: Option<&mut Option<&'static Mbuf>>, m: Option<MbufBox>) {
    match slot {
        Some(s) => *s = m.map(park),
        None => drop(m),
    }
}

/// `soreceive(so, paddr, uio, mp0, controlp, flagsp, controllen)`: the address, the data and
/// the control handed out parked into the caller's slots.
#[allow(clippy::too_many_arguments, reason = "the C's argument list")]
pub fn soreceive(
    so: &Socket,
    paddr: Option<&mut Option<&'static Mbuf>>,
    uio: &mut Uio<'_>,
    mp0: Option<&mut Option<&'static Mbuf>>,
    controlp: Option<&mut Option<&'static Mbuf>>,
    flagsp: Option<&mut i32>,
    controllen: u32,
) -> Result<(), Errno> {
    let (want_addr, want_mp, want_ctl) = (paddr.is_some(), mp0.is_some(), controlp.is_some());
    let mut addr = None;
    let mut mp = None;
    let mut ctl = None;
    let error = kernel::net::socket::soreceive(
        so,
        want_addr.then_some(&mut addr),
        uio,
        want_mp.then_some(&mut mp),
        want_ctl.then_some(&mut ctl),
        flagsp,
        controllen,
    );
    park_into(paddr, addr);
    park_into(mp0, mp);
    park_into(controlp, ctl);
    error
}

/// `soshutdown(so, how)`.
pub fn soshutdown(so: &Socket, how: i32) -> Result<(), Errno> {
    kernel::net::socket::soshutdown(so, how)
}

/// `sosetopt(so, level, optname, m)` for `curproc`; `m` is the caller's (parked) and stays
/// the caller's.
pub fn sosetopt(
    so: &Socket,
    level: i32,
    optname: i32,
    m: Option<&'static Mbuf>,
) -> Result<(), Errno> {
    let p = curproc_or_panic("sosetopt");
    kernel::net::socket::sosetopt(so, p, level, optname, m.map(mut_of))
}

/// `sogetopt(so, level, optname, m)`: the value into the caller's (parked) mbuf.
pub fn sogetopt(so: &Socket, level: i32, optname: i32, m: &'static Mbuf) -> Result<(), Errno> {
    kernel::net::socket::sogetopt(so, level, optname, mut_of(m))
}

/// `so_print(so, pr)` of ddb.
pub fn so_print(so: &Socket, pr: crate::machine::db_machdep::PrFn) {
    struct Out(crate::machine::db_machdep::PrFn);
    impl core::fmt::Write for Out {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            (self.0)(format_args!("{s}"));
            Ok(())
        }
    }
    let _ = kernel::net::socket::so_print(so, &mut Out(pr));
}
/* </CODE> */
