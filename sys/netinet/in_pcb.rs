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
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)in_pcb.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1991, 1993
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
//! Internet protocol control blocks: `<netinet/in_pcb.h>` and `netinet/in_pcb.c`, as `bsd`'s
//! transports reach them.
//!
//! Upstream: sys/netinet/in_pcb.h @ 3ce1f3f79392
//! Upstream: sys/netinet/in_pcb.c @ 3ce1f3f79392
//!
//! The control blocks are the kernel's since N6.4c1 (`kernel::netinet::in_pcb`): this module
//! re-exports them for `bsd`'s importers and keeps the shape LZ's callers use until they move
//! (tcp, udp, raw IP and divert at N6.6, pf at N8): a control block is a `&'static Inpcb` held
//! by a count of the kernel's pcb (`in_pcbref`/`in_pcbunref`, the C's `inp_refcnt`), the
//! kernel's counted answers (lookups, the iterator) become such references here, the
//! `INP_IPV6` pcbs go to `netinet6/in6_pcb.rs`'s half before the kernel's IPv4 functions run,
//! and the members `bsd` keeps (the IPv4 options, the IPv6 options and multicast options, the
//! ICMPv6 filter) are read here; the IPv4 multicast options are the kernel's since N6.4c1.
//!
//! ## Deviations
//! - `sotoinpcb` panics on a socket of neither `inetdomain` nor `inet6domain` (the C macro
//!   casts `so_pcb` blindly).
//! - `in_pcbunref`'s three `KASSERT`s that the links are cleared are the kernel's pinned
//!   lists' own check (an element dropped while linked is `panic(9)`).
//! - The `DIAGNOSTIC` `in_pcbnotifymiss` printfs are not made.

use core::mem::size_of;
use core::pin::Pin;
use core::ptr::{self, NonNull};

use hal::once::OnceStatic;
use kernel::netinet::in_pcb as k;

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::kern::uipc_mbuf::m_freem;
use crate::net::if_::net_held;
use crate::net::route::{Rtentry, as_static};
use crate::net::toeplitz::stoeplitz_ip4port;
use crate::netinet::icmp6::Icmp6Filter;
use crate::netinet::in_::{InAddr, SockaddrIn};
use crate::netinet::ip_ipsp::SockaddrUnion;
#[cfg(feature = "inet6")]
use crate::netinet6::in6_pcb::{
    in6_pcbbind_locked, in6_pcbconnect, in6_pcbrtentry, in6_pcbset_addr, in6_setpeeraddr,
    in6_setsockaddr,
};
#[cfg(feature = "inet6")]
use crate::netinet6::ip6_output::{ip6_freemoptions, ip6_freepcbopts};
use crate::netinet6::ip6_var::{Ip6Moptions, Ip6Pktopts};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, mtod, of_parked};
use crate::sys::proc::Proc;
use crate::sys::socket::AF_INET;
#[cfg(feature = "inet6")]
use crate::sys::socket::AF_INET6;
use crate::sys::socketvar::Socket;

pub use kernel::netinet::in_pcb::{
    Addr6, BADDYNAMICPORTS, Baddynamicports, DEFBADDYNAMICPORTS_TCP, DEFBADDYNAMICPORTS_UDP,
    DEFROOTONLYPORTS_TCP, DEFROOTONLYPORTS_UDP, DP_MAPBITS, DP_MAPSIZE, Hdr6, IN_PCBLOCK_GRAB,
    IN_PCBLOCK_HOLD, IN6P_ANONPORT, IN6P_AUTOFLOWLABEL, IN6P_CONTROLOPTS, IN6P_DSTOPTS,
    IN6P_HIGHPORT, IN6P_HOPLIMIT, IN6P_HOPOPTS, IN6P_LOWPORT, IN6P_MINMTU, IN6P_MTU, IN6P_PKTINFO,
    IN6P_RECVDSTPORT, IN6P_RFC2292, IN6P_RTHDR, IN6P_TCLASS, INP_CONTROLOPTS, INP_HDRINCL,
    INP_HIGHPORT, INP_HOPLIMIT, INP_IPSECFLOWINFO, INP_IPV6, INP_LOWPORT, INP_RECVDSTADDR,
    INP_RECVDSTPORT, INP_RECVIF, INP_RECVOPTS, INP_RECVRETOPTS, INP_RECVRTABLE, INP_RECVTTL,
    INP_RXDSTOPTS, INP_RXHOPOPTS, INP_RXINFO, INP_RXSRCRT, INPLOOKUP_IPV6, INPLOOKUP_SETLOCAL,
    INPLOOKUP_WILDCARD, IPPORT_FIRSTAUTO, IPPORT_LASTAUTO, Inpaddru, InpcbRef, LegacyKey,
    ROOTONLYPORTS, ZEROIN_ADDR, ZEROIN46_ADDR, dp_clr, dp_isset, dp_set, in_baddynamic,
    in_pcb_is_iterator, in_pcbhash, in_pcblhash, in_rootonly, ipport_hifirstauto,
    ipport_hilastauto,
};

/// `struct inpcb` of `bsd`'s sockets.
pub type Inpcb = k::Inpcb<BsdProcGlue>;
/// `struct inpcbtable` of `bsd`'s transports.
pub type Inpcbtable = k::Inpcbtable<BsdProcGlue>;
/// The table mutex held.
pub type InpGuard<'a> = k::InpGuard<'a, BsdProcGlue>;

/// `inp_upcall`: a hook that sees UDP datagrams before the socket buffer
/// (`(arg, m, ip, ip6, uh, hlen, ns)`); returns the packet to append, or `None` when it took
/// it.
///
/// # Safety
///
/// `arg` is the pcb's `inp_upcall_arg`; `ip` (or `ip6`) points at the packet's IP header and
/// `uh` at its UDP header, both readable for the call.
pub type InpUpcallFn = unsafe fn(
    *mut core::ffi::c_void,
    &'static Mbuf,
    *const crate::netinet::ip::Ip,
    *const core::ffi::c_void,
    *const core::ffi::c_void,
    i32,
    Option<&mut crate::net::if_var::Netstack>,
) -> Option<&'static Mbuf>;

/// An [`InpUpcallFn`] as the kernel's pcb keeps it (`Inpcb::set_upcall`'s erased function):
/// a `static` of its setter's (wg(4)'s `WG_UPCALL`).
pub struct InpUpcallEntry(pub InpUpcallFn);

/// The per-pcb hook of `in_pcbnotifyall` and the protocols' `ctlinput`s (`udp_notify`,
/// `in_pcbrtchange`): the control block and the errno of the event (`None` for 0).
pub type InpNotifyFn = fn(&'static Inpcb, Option<Errno>);

/// The [`LegacyKey`] `in_init` took at boot.
static LEGACY_KEY: OnceStatic<&'static LegacyKey> = OnceStatic::new();

/// The kernel pcb's key for `bsd`'s counts, taken by the first `in_init` (the boot's; the
/// host tests, which have no boot, by their first setup). `panic(9)` when another took it.
pub(crate) fn legacy_key() -> &'static LegacyKey {
    if let Some(k) = LEGACY_KEY.get() {
        return k;
    }
    if let Some(k) = LegacyKey::take() {
        let _ = LEGACY_KEY.set(k);
    }
    for _ in 0..(1u32 << 20) {
        if let Some(k) = LEGACY_KEY.get() {
            return k;
        }
        core::hint::spin_loop();
    }
    panic(format_args!(
        "in_pcb: the kernel's legacy key was taken outside bsd"
    ))
}

/// A control block as the `&'static Inpcb` LZ's callers keep.
pub fn inp_static(inp: &Inpcb) -> &'static Inpcb {
    // SAFETY: `inp` is the value of an item of the kernel's "inpcb" pool (a `PoolArc` item:
    // its value stays in place until its last handle's drop), which `bsd` reaches only while
    // a count keeps it: a count of `inp_refcnt` (the socket's, until `in_pcbdetach`, or one
    // `in_pcbref` took), whose anchor handle the item holds while the count is above zero, or
    // a handle of the kernel's lent for the call (a lookup's answer, the iterator's). Every
    // `bsd` caller uses the reference under those conventions, the C's: no further than its
    // `in_pcbunref`, or than the socket lock it holds while the socket is attached.
    unsafe { &*ptr::from_ref(inp) }
}

/// A kernel answer held as `bsd` holds a control block: one count of `inp_refcnt` taken for
/// it with the table's guard `g` still held (the C's `in_pcbref` before `mtx_leave`), the
/// handle dropped.
pub(crate) fn legacy_hold(h: InpcbRef<BsdProcGlue>, g: &InpGuard<'_>) -> &'static Inpcb {
    h.legacy_ref_locked(g);
    inp_static(&h)
}

/// `in_init`: the kernel's `inpcb` pool, and the key `bsd`'s `in_pcbunref` needs.
pub fn in_init() {
    k::in_init::<BsdProcGlue>();
    let _ = legacy_key();
}

/// `sotoinpcb(so)`: the control block of an Internet socket, `None` once detached.
pub fn sotoinpcb(so: &Socket) -> Option<&'static Inpcb> {
    let family = so.dom_family();
    #[cfg(feature = "inet6")]
    let inet6 = family == i32::from(AF_INET6);
    #[cfg(not(feature = "inet6"))]
    let inet6 = false;
    if family != i32::from(AF_INET) && !inet6 {
        panic(format_args!(
            "sotoinpcb: socket {:p} of family {}",
            so,
            so.dom_family()
        ));
    }
    let p = ptr::with_exposed_provenance::<Inpcb>(so.legacy_pcb());
    // SAFETY: an `inetdomain` or `inet6domain` socket's `so_pcb` is 0 or the exposed address
    // of the kernel pcb `in_pcballoc` made for it, whose `inp_refcnt` keeps the socket's count
    // (and so the item, through its anchor) until `in_pcbdetach` clears the word first; the
    // callers hold the socket lock, which `in_pcbdetach`'s caller holds too.
    unsafe { p.as_ref() }
}

/// `in_pcbinit`: sets up `table` with hash tables of `hashsize` buckets and fresh keys.
pub fn in_pcbinit(table: &Inpcbtable, hashsize: i32) {
    k::in_pcbinit(table, hashsize);
}

/// `in_pcballoc`: a new control block for `so` in `table`.
pub fn in_pcballoc(
    so: &'static Socket,
    table: &'static Inpcbtable,
    wait: i32,
) -> Result<(), Errno> {
    k::in_pcballoc(so, table, wait).map(drop)
}

/// The members `bsd` keeps in the kernel's pcb (its words), read and written here.
pub trait InpcbLegacy {
    /// `inp->inp_options`: the IPv4 options, a parked mbuf chain.
    fn options(&self) -> Option<&'static Mbuf>;
    /// `inp->inp_options = m` (the old chain is the caller's to free).
    fn set_options(&self, m: Option<&'static Mbuf>);
    /// `inp->inp_moptions6`'s pointer.
    fn moptions6_ptr(&self) -> Option<NonNull<Ip6Moptions>>;
    /// `inp->inp_moptions6 = imo`.
    fn set_moptions6_ptr(&self, imo: Option<NonNull<Ip6Moptions>>);
    /// `inp->inp_outputopts6`'s pointer.
    fn outputopts6_ptr(&self) -> Option<NonNull<Ip6Pktopts>>;
    /// `inp->inp_outputopts6 = o`.
    fn set_outputopts6_ptr(&self, o: Option<NonNull<Ip6Pktopts>>);
    /// `inp->inp_icmp6filt`'s pointer.
    fn icmp6filt_ptr(&self) -> Option<NonNull<Icmp6Filter>>;
    /// `inp->inp_icmp6filt = f`.
    fn set_icmp6filt_ptr(&self, f: Option<NonNull<Icmp6Filter>>);
    /// `inp->inp_upcall`, `inp->inp_upcall_arg` (wg(4)'s, set once).
    fn upcall_pair(&self) -> Option<(InpUpcallFn, *mut core::ffi::c_void)>;
}

impl InpcbLegacy for Inpcb {
    fn options(&self) -> Option<&'static Mbuf> {
        NonNull::new(self.inp_options.get_mut().cast::<Mbuf>()).map(of_parked)
    }

    fn set_options(&self, m: Option<&'static Mbuf>) {
        self.inp_options
            .set(m.map_or(ptr::null(), |m| ptr::from_ref(m).cast()));
    }

    fn moptions6_ptr(&self) -> Option<NonNull<Ip6Moptions>> {
        NonNull::new(self.inp_moptions6.get_mut().cast::<Ip6Moptions>())
    }

    fn set_moptions6_ptr(&self, imo: Option<NonNull<Ip6Moptions>>) {
        self.inp_moptions6
            .set(imo.map_or(ptr::null(), |p| p.as_ptr().cast_const().cast()));
    }

    fn outputopts6_ptr(&self) -> Option<NonNull<Ip6Pktopts>> {
        NonNull::new(self.inp_outputopts6.get_mut().cast::<Ip6Pktopts>())
    }

    fn set_outputopts6_ptr(&self, o: Option<NonNull<Ip6Pktopts>>) {
        self.inp_outputopts6
            .set(o.map_or(ptr::null(), |p| p.as_ptr().cast_const().cast()));
    }

    fn icmp6filt_ptr(&self) -> Option<NonNull<Icmp6Filter>> {
        NonNull::new(self.inp_icmp6filt.get_mut().cast::<Icmp6Filter>())
    }

    fn set_icmp6filt_ptr(&self, f: Option<NonNull<Icmp6Filter>>) {
        self.inp_icmp6filt
            .set(f.map_or(ptr::null(), |p| p.as_ptr().cast_const().cast()));
    }

    fn upcall_pair(&self) -> Option<(InpUpcallFn, *mut core::ffi::c_void)> {
        let u = self.upcall()?;
        let f = u.f.downcast_ref::<InpUpcallEntry>()?;
        Some((
            f.0,
            ptr::with_exposed_provenance_mut::<core::ffi::c_void>(u.arg),
        ))
    }
}

/// `in_pcbbind_locked`: binds `inp` to the address in `nam` (or `laddr` and a port picked
/// here when `nam` is `None`), with the table mutex held (`g`).
pub fn in_pcbbind_locked(
    g: &mut InpGuard<'_>,
    inp: &'static Inpcb,
    nam: Option<&Mbuf>,
    laddr: &Inpaddru,
    p: &Proc,
) -> Result<(), Errno> {
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        return in6_pcbbind_locked(g, inp, nam, laddr, p);
    }
    k::in_pcbbind_locked(g, inp, nam, laddr, p, &net_held("in_pcbbind"))
}

/// `in_pcbbind`: binds `inp` to `nam`, or to a port picked here.
pub fn in_pcbbind(inp: &'static Inpcb, nam: Option<&Mbuf>, p: &Proc) -> Result<(), Errno> {
    // keep lookup, modification, and rehash in sync
    let mut g = inp.table().lock();
    in_pcbbind_locked(&mut g, inp, nam, &ZEROIN46_ADDR, p)
}

/// `in_pcbaddrisavail`: whether `inp` may bind to `sin`, taking the table mutex.
pub fn in_pcbaddrisavail(
    inp: &Inpcb,
    sin: &mut SockaddrIn,
    wild: i32,
    _p: &Proc,
) -> Result<(), Errno> {
    k::in_pcbaddrisavail(inp, sin, wild, &net_held("in_pcbaddrisavail"))
}

/// The flow id of a connected pcb (`NSTOEPLITZ`), after the table mutex as the C.
fn set_flowid4(inp: &Inpcb) {
    inp.inp_flowid.set(stoeplitz_ip4port(
        inp.inp_faddr.get().s_addr,
        inp.inp_laddr.get().s_addr,
        inp.inp_fport.get(),
        inp.inp_lport.get(),
    ));
}

/// `in_pcbconnect`: connects `inp` to the address in `nam`; both address and port must be
/// specified. If the socket has no local address yet, one is picked.
pub fn in_pcbconnect(inp: &'static Inpcb, nam: &Mbuf) -> Result<(), Errno> {
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        return in6_pcbconnect(inp, nam);
    }
    let p = curproc_or_panic("in_pcbconnect");
    k::in_pcbconnect(inp, nam, p, &net_held("in_pcbconnect"))?;
    set_flowid4(inp);
    Ok(())
}

/// `curproc`, which the socket requests run as.
fn curproc_or_panic(func: &str) -> &'static Proc {
    match crate::machine::cpu::curproc() {
        Some(p) => p,
        None => panic(format_args!("{}: no curproc", func)),
    }
}

/// `in_pcbdisconnect`: forgets the flow; a socket without a file reference goes too.
pub fn in_pcbdisconnect(inp: &'static Inpcb) {
    if k::in_pcbdisconnect(inp) {
        in_pcbdetach(inp);
    }
}

/// What `in_pcbdetach` frees of `bsd`'s members (the options), at the C's point.
fn free_legacy_opts(inp: &Inpcb) {
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        let (opts, mopts) = (inp.outputopts6_ptr(), inp.moptions6_ptr());
        inp.set_outputopts6_ptr(None);
        inp.set_moptions6_ptr(None);
        // SAFETY: the options are this control block's own allocations
        // (`ip6_setpktopts`, `ip6_setmoptions`), taken out of it here and never used again.
        unsafe {
            ip6_freepcbopts(opts);
            ip6_freemoptions(mopts);
        }
        return;
    }
    let opts = inp.options();
    inp.set_options(None);
    m_freem(opts);
}

/// `in_pcbdetach`: unlinks `inp` from its socket and its table and drops the socket's
/// reference.
pub fn in_pcbdetach(inp: &'static Inpcb) {
    let Some(h) = hal::pool::PoolArc::from_ref(inp) else {
        panic(format_args!("in_pcbdetach: inpcb {inp:p} not a pool item"));
    };
    k::in_pcbdetach(h, &net_held("in_pcbdetach"), free_legacy_opts);
}

/// `in_pcbsolock`: locks the socket of `inp`; `None` when it is detached.
pub fn in_pcbsolock(inp: &'static Inpcb) -> Option<&'static Socket> {
    k::in_pcbsolock(inp)
}

/// `in_pcbsounlock`: unlocks what `in_pcbsolock` locked.
pub fn in_pcbsounlock(inp: Option<&Inpcb>, so: Option<&Socket>) {
    k::in_pcbsounlock(inp, so);
}

/// `in_pcbref`: takes a reference to `inp`.
pub fn in_pcbref(inp: Option<&'static Inpcb>) -> Option<&'static Inpcb> {
    let inp = inp?;
    inp.legacy_ref();
    Some(inp)
}

/// `in_pcbunref`: drops a reference to `inp`; the last one releases the socket and the
/// control block.
pub fn in_pcbunref(inp: Option<&Inpcb>) {
    let Some(inp) = inp else {
        return;
    };
    drop(inp.legacy_rele(legacy_key()));
}

/// `in_pcb_iterator`: the control block after `iter`'s place in `table` (the first one when
/// the walk starts), referenced, with `iter` marking the place; drops the reference to the
/// previous answer `inp`. `None` ends the walk. `g` is the table mutex, held around every
/// call; `iter` is a pinned `Inpcb::marker()` that lives until the walk ends.
pub fn in_pcb_iterator(
    table: &Inpcbtable,
    g: &mut InpGuard<'_>,
    inp: Option<&Inpcb>,
    iter: Pin<&Inpcb>,
) -> Option<&'static Inpcb> {
    let next = k::in_pcb_iterator(table, g, None, iter);
    in_pcbunref(inp);
    next.map(|h| legacy_hold(h, g))
}

/// `in_pcb_iterator_abort`: ends a walk before `in_pcb_iterator` returned `None`.
pub fn in_pcb_iterator_abort(
    table: &Inpcbtable,
    g: &mut InpGuard<'_>,
    inp: Option<&Inpcb>,
    iter: Pin<&Inpcb>,
) {
    k::in_pcb_iterator_abort(table, g, None, iter);
    in_pcbunref(inp);
}

/// Writes `sin` as the address in `nam`.
fn set_nam(nam: &Mbuf, sin: SockaddrIn) {
    nam.m_len().set(size_of::<SockaddrIn>() as u32);
    // SAFETY: `nam` is an `MT_SONAME` mbuf of `MLEN` bytes, more than a `sockaddr_in`;
    // written unaligned (mbuf data need not be aligned).
    unsafe { mtod::<SockaddrIn>(nam).write_unaligned(sin) };
}

/// `in_setsockaddr`: the local address of `inp` into `nam`.
pub fn in_setsockaddr(inp: &Inpcb, nam: &Mbuf) {
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        in6_setsockaddr(inp, nam);
        return;
    }
    set_nam(nam, k::in_setsockaddr(inp));
}

/// `in_setpeeraddr`: the foreign address of `inp` into `nam`.
pub fn in_setpeeraddr(inp: &Inpcb, nam: &Mbuf) {
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        in6_setpeeraddr(inp, nam);
        return;
    }
    set_nam(nam, k::in_setpeeraddr(inp));
}

/// `sotoinpcb(so)` of a socket the C knows to be attached.
fn inpcb_of(so: &Socket) -> &'static Inpcb {
    match sotoinpcb(so) {
        Some(inp) => inp,
        None => panic(format_args!("socket {:p}: no inpcb", so)),
    }
}

/// `in_sockaddr`: the `pru_sockaddr` of the Internet protocols.
pub fn in_sockaddr(so: &'static Socket, nam: &'static Mbuf) -> Result<(), Errno> {
    in_setsockaddr(inpcb_of(so), nam);

    Ok(())
}

/// `in_peeraddr`: the `pru_peeraddr` of the Internet protocols.
pub fn in_peeraddr(so: &'static Socket, nam: &'static Mbuf) -> Result<(), Errno> {
    in_setpeeraddr(inpcb_of(so), nam);

    Ok(())
}

/// `in_flowid`: the `pru_flowid` of the Internet protocols.
pub fn in_flowid(so: &'static Socket) -> i32 {
    match sotoinpcb(so) {
        Some(inp) => i32::from(inp.inp_flowid.get()),
        None => 0,
    }
}

/// `in_pcbnotifyall`: passes some notification to all connections of a protocol associated
/// with address `dst`, calling `notify` for each matching socket with the socket locked.
pub fn in_pcbnotifyall(
    table: &Inpcbtable,
    dst: &SockaddrIn,
    rtable: u32,
    errno: Option<Errno>,
    notify: Option<InpNotifyFn>,
) {
    let Some(notify) = notify else {
        return;
    };
    k::in_pcbnotifyall(table, dst, rtable, errno, |inp, e| {
        notify(inp_static(inp), e)
    });
}

/// `in_losing`: invalidates the cached route (a dynamic one is deleted).
pub fn in_losing(inp: &Inpcb) {
    k::in_losing(inp, &net_held("in_losing"));
}

/// `in_pcbrtchange`: after a routing change, flushes the old route.
pub fn in_pcbrtchange(inp: &'static Inpcb, errno: Option<Errno>) {
    crate::kern::uipc_socket2::soassertlocked(inp.socket());
    k::in_pcbrtchange(inp, errno);
}

/// `in_pcblookup_local_lock`: the control block of `table` bound to local port `lport` and
/// address `laddrp` with the fewest wildcards, referenced; `IN_PCBLOCK_GRAB` only (a caller
/// holding the table mutex calls the kernel's `in_pcblookup_local` with its guard).
pub fn in_pcblookup_local_lock(
    table: &Inpcbtable,
    laddrp: &Inpaddru,
    lport: u16,
    flags: i32,
    rtable: u32,
    lock: i32,
) -> Option<&'static Inpcb> {
    if lock != IN_PCBLOCK_GRAB {
        panic(format_args!(
            "in_pcblookup_local_lock: lock {lock} without a guard"
        ));
    }
    // The answer's count taken before the mutex is let go (in_pcb.c's order).
    let mut g = table.lock();
    let inp = k::in_pcblookup_local(&mut g, table, laddrp, lport, flags, rtable);
    let inp = inp.map(|h| legacy_hold(h, &g));
    drop(g);
    inp
}

/// `in_pcbrtentry`: the route to the foreign address of `inp`, from its cache.
pub fn in_pcbrtentry(inp: &Inpcb) -> Option<&'static Rtentry> {
    crate::kern::uipc_socket2::soassertlocked(inp.socket());
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        return in6_pcbrtentry(inp);
    }
    // The cache keeps its own handle while `bsd`'s caller uses the route (`as_static`).
    k::in_pcbrtentry(inp, &net_held("in_pcbrtentry")).map(|rt| as_static(&rt))
}

/// `in_pcbselsrc`: the IPv4 address most appropriate as the source for destination
/// `dstsock`, into `insrc`.
pub fn in_pcbselsrc(insrc: &mut InAddr, dstsock: &SockaddrIn, inp: &Inpcb) -> Result<(), Errno> {
    *insrc = k::in_pcbselsrc(dstsock, inp, &net_held("in_pcbselsrc"))?;
    Ok(())
}

/// `in_pcblookup`: the connected control block for `faddr.fport <-> laddr.lport`,
/// referenced.
pub fn in_pcblookup(
    table: &Inpcbtable,
    faddr: InAddr,
    fport: u16,
    laddr: InAddr,
    lport: u16,
    rtable: u32,
) -> Option<&'static Inpcb> {
    // The answer's count taken before the mutex is let go (in_pcb.c:1220-1221): a pcb a
    // concurrent detach and last in_pcbunref would free is still held then.
    // The hash before the mutex (in_pcb.c:1208-1212).
    let rdomain = kernel::net::rtable::rtable_l2(rtable);
    let hash = k::in_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    let mut g = table.lock();
    let inp = k::in_pcbhash_lookup(&mut g, hash, rdomain, &faddr, fport, &laddr, lport);
    let inp = inp.map(|h| legacy_hold(h, &g));
    drop(g);
    inp
}

/// `in_pcblookup_listen`: the listening control block for `laddr.lport`, referenced.
pub fn in_pcblookup_listen(
    table: &Inpcbtable,
    laddr: InAddr,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<&'static Inpcb> {
    // pf's divert key before the mutex, the count before it is let go (in_pcb.c:1304-1305).
    // pf's divert key and the first hash before the mutex (in_pcb.c:1292-1294).
    let l = k::in_pcblookup_listen_keys(table, laddr, lport_arg, m, rtable)?;
    let mut g = table.lock();
    let inp = k::in_pcblookup_listen_held(&mut g, table, &l);
    let inp = inp.map(|h| legacy_hold(h, &g));
    drop(g);
    inp
}

/// `in_pcbset_rtableid`: moves an unbound `inp` to routing table `rtableid`.
pub fn in_pcbset_rtableid(inp: &'static Inpcb, rtableid: u32) -> Result<(), Errno> {
    k::in_pcbset_rtableid(inp, rtableid)
}

/// `in_pcbset_addr`: gives `inp` both addresses at once (a connection accepted from a
/// listener), unless another control block has them.
pub fn in_pcbset_addr(
    inp: &'static Inpcb,
    fsa: &SockaddrUnion,
    lsa: &SockaddrUnion,
    rtableid: u32,
) -> Result<(), Errno> {
    #[cfg(feature = "inet6")]
    if inp.has_flags(INP_IPV6) {
        crate::kassert!(fsa.sa_family() == AF_INET6);
        crate::kassert!(lsa.sa_family() == AF_INET6);
        return in6_pcbset_addr(inp, &su_sin6(fsa), &su_sin6(lsa), rtableid);
    }
    crate::kassert!(fsa.sa_family() == AF_INET);
    crate::kassert!(lsa.sa_family() == AF_INET);
    k::in_pcbset_addr(inp, &fsa.sin(), &lsa.sin(), rtableid)?;
    set_flowid4(inp);
    Ok(())
}

/// `satosin6_const(sa)` of a generic socket address image, read out of it.
#[cfg(feature = "inet6")]
fn su_sin6(su: &SockaddrUnion) -> crate::netinet6::in6::SockaddrIn6 {
    // SAFETY: the union is `sizeof(struct sockaddr_in6)` initialised bytes; `SockaddrIn6` is
    // plain old data of that size, read unaligned.
    unsafe {
        su.as_bytes()
            .as_ptr()
            .cast::<crate::netinet6::in6::SockaddrIn6>()
            .read_unaligned()
    }
}

/// `in_pcbunset_faddr`: forgets the foreign address.
pub fn in_pcbunset_faddr(inp: &'static Inpcb) {
    k::in_pcbunset_faddr(inp);
}

/// `in_pcbunset_laddr`: forgets both addresses (the local port stays).
pub fn in_pcbunset_laddr(inp: &'static Inpcb) {
    k::in_pcbunset_laddr(inp);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for the Internet control blocks: the port bitmaps, binding (explicit ports,
    // picked ports, `EADDRINUSE` and `SO_REUSEPORT`), the local port, connection and listen
    // lookups, a table that grows past its load factor, the iterator and detaching.
    //
    // [`setup`] is the network setup of `ip_input`'s tests plus a thread with credentials made
    // `curproc` (sockets read it), the socket pool and `in_init`; the UDP and raw IP tests use it
    // too. [`teardown`] clears `curproc`.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use core::sync::atomic::AtomicU32;

    use super::*;
    use crate::kern::kern_prot::crget;
    use crate::kern::uipc_mbuf::m_get;
    use crate::kern::uipc_socket::{soalloc, soinit, sorele};
    use crate::kern::uipc_socket2::{solock, sounlock};
    use crate::machine::Machine;
    use crate::machine::cpu::Cpu;
    use crate::netinet::in_::{IPPORT_RESERVED, IPPORT_USERRESERVED, IPPROTO_TCP, IPPROTO_UDP};
    use crate::netinet::in_proto::INETSW;
    use crate::sys::endian::htons;
    use crate::sys::mbuf::M_WAIT;
    use crate::sys::mbuf::{M_DONTWAIT, MT_SONAME};
    use crate::sys::socket::{SO_REUSEPORT, SOCK_DGRAM};
    use crate::sys::socketvar::{SS_NOFDREF, soref};

    /// The network setup, a `curproc` with credentials, sockets and control blocks.
    pub(crate) fn setup() -> (
        MutexGuard<'static, ()>,
        MutexGuard<'static, ()>,
        &'static Proc,
    ) {
        let (g, t) = crate::netinet::ip_input::tests::setup();
        crate::kern::proc_glue::procinit();
        let p = crate::kern::proc_glue::test_proc(1, crget());
        Machine::set_curproc(Machine::curcpu(), Some(p));
        soinit();
        crate::kern::uipc_domain::net_tables_for_tests();
        in_init();
        (g, t, p)
    }

    /// Undoes what outlives the reset memory: `curproc`.
    pub(crate) fn teardown() {
        Machine::set_curproc(Machine::curcpu(), None);
    }

    /// An address mbuf holding `sin`.
    pub(crate) fn nam(sin: SockaddrIn) -> &'static Mbuf {
        let m = m_get(M_DONTWAIT, MT_SONAME).expect("mbuf");
        m.m_len().set(size_of::<SockaddrIn>() as u32);
        // SAFETY: a fresh mbuf of `MLEN` bytes.
        unsafe { mtod::<SockaddrIn>(m).write_unaligned(sin) };
        m
    }

    /// `addr:port` as a `sockaddr_in`.
    fn sin(addr: [u8; 4], port: u16) -> SockaddrIn {
        SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_port: htons(port),
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes(addr),
            },
            ..SockaddrIn::default()
        }
    }

    /// A UDP socket with a control block in `table`.
    fn pcb(table: &'static Inpcbtable) -> &'static Inpcb {
        let so = soalloc(&INETSW[1], SOCK_DGRAM, M_WAIT).expect("socket");
        in_pcballoc(so, table, M_WAIT).expect("in_pcballoc");
        sotoinpcb(so).expect("attached")
    }

    /// Detaches the control block and lets the socket go, as `udp_detach` and `soclose` do
    /// (holding a reference of their own across the unlock).
    fn release(inp: &'static Inpcb) {
        let so = inp.socket();
        let _ = soref(Some(so));
        solock(so);
        so.set_state_legacy(SS_NOFDREF);
        in_pcbdetach(inp);
        sounlock(so);
        sorele(so);
    }

    /// A table of `hashsize` buckets.
    fn table(hashsize: i32) -> &'static Inpcbtable {
        let t: &'static Inpcbtable = Box::leak(Box::new(Inpcbtable::new()));
        in_pcbinit(t, hashsize);
        t
    }

    #[test]
    fn the_port_bitmaps_and_their_defaults() {
        let (_g, _t, _p) = setup();
        let m: Box<[AtomicU32; DP_MAPSIZE]> = Box::new([const { AtomicU32::new(0) }; DP_MAPSIZE]);
        assert_eq!(DP_MAPSIZE, 2048);
        dp_set(&m, 65535);
        dp_set(&m, 33);
        assert!(dp_isset(&m, 65535) && dp_isset(&m, 33) && !dp_isset(&m, 32));
        dp_clr(&m, 33);
        assert!(!dp_isset(&m, 33));

        // ip_init filled the defaults.
        assert!(in_baddynamic(2049, IPPROTO_TCP as u16));
        assert!(in_baddynamic(7784, IPPROTO_UDP as u16));
        assert!(!in_baddynamic(7784, IPPROTO_TCP as u16));
        assert!(!in_baddynamic(2049, 1));
        assert!(in_rootonly(80, IPPROTO_UDP as u16));
        assert!(in_rootonly(2049, IPPROTO_UDP as u16));
        assert!(!in_rootonly(5353, IPPROTO_UDP as u16));
        teardown();
    }

    /// `f` with the net lock held, as the socket layer's `solock` calls `in_pcbbind` (its
    /// address check reads the interfaces' addresses under it).
    fn locked<R>(f: impl FnOnce() -> R) -> R {
        crate::sys::systm::net_lock();
        let r = f();
        crate::sys::systm::net_unlock();
        r
    }

    #[test]
    fn bind_lookups_and_the_iterator() {
        let (_g, _t, p) = setup();
        let t = table(1);

        // An explicit port on the wildcard address.
        let a = pcb(t);
        let n = nam(sin([0; 4], 5000));
        locked(|| in_pcbbind(a, Some(n), p)).expect("bind");
        assert_eq!(a.inp_lport.get(), htons(5000));
        let found = in_pcblookup_local_lock(t, &ZEROIN46_ADDR, htons(5000), 0, 0, IN_PCBLOCK_GRAB);
        assert!(found.is_some_and(|f| ptr::eq(f, a)));
        in_pcbunref(found);
        let any = Inpaddru::from_addr(sin([10, 0, 2, 15], 0).sin_addr);
        assert!(in_pcblookup_local_lock(t, &any, htons(5000), 0, 0, IN_PCBLOCK_GRAB).is_none());
        let wild =
            in_pcblookup_local_lock(t, &any, htons(5000), INPLOOKUP_WILDCARD, 0, IN_PCBLOCK_GRAB);
        assert!(wild.is_some_and(|f| ptr::eq(f, a)));
        in_pcbunref(wild);
        // Bound already.
        assert_eq!(locked(|| in_pcbbind(a, Some(n), p)), Err(Errno::EINVAL));

        // The same port again (a second socket grows the table past its load factor).
        let b = pcb(t);
        assert!(t.size() > 1, "resized");
        assert_eq!(locked(|| in_pcbbind(b, Some(n), p)), Err(Errno::EADDRINUSE));
        // ... unless both sockets allow it.
        a.socket().set_options_legacy(SO_REUSEPORT, true);
        b.socket().set_options_legacy(SO_REUSEPORT, true);
        locked(|| in_pcbbind(b, Some(n), p)).expect("SO_REUSEPORT bind");
        // An address that is not ours.
        let c = pcb(t);
        let other = nam(sin([192, 0, 2, 1], 5001));
        assert_eq!(
            locked(|| in_pcbbind(c, Some(other), p)),
            Err(Errno::EADDRNOTAVAIL)
        );
        // A picked port, from the default range.
        locked(|| in_pcbbind(c, None, p)).expect("anonymous bind");
        let port = i32::from(u16::from_be(c.inp_lport.get()));
        assert!(
            (IPPORT_RESERVED..=IPPORT_USERRESERVED).contains(&port),
            "port {port}"
        );

        // A connection's quadruple, then the exact and the listen lookups.
        let fsin = sin([10, 0, 2, 2], 53);
        let lsin = sin([10, 0, 2, 15], 6000);
        let d = pcb(t);
        let (fsu, lsu) = (
            SockaddrUnion::from_sin(&fsin),
            SockaddrUnion::from_sin(&lsin),
        );
        in_pcbset_addr(d, &fsu, &lsu, 0).expect("set_addr");
        assert_eq!(
            in_pcbset_addr(pcb(t), &fsu, &lsu, 0),
            Err(Errno::EADDRINUSE)
        );
        let hit = in_pcblookup(
            t,
            fsin.sin_addr,
            fsin.sin_port,
            lsin.sin_addr,
            lsin.sin_port,
            0,
        );
        assert!(hit.is_some_and(|h| ptr::eq(h, d)));
        in_pcbunref(hit);
        let listen = in_pcblookup_listen(t, lsin.sin_addr, htons(5000), None, 0);
        assert!(listen.is_some_and(|l| ptr::eq(l, a) || ptr::eq(l, b)));
        in_pcbunref(listen);
        assert!(in_pcblookup_listen(t, lsin.sin_addr, htons(5002), None, 0).is_none());

        // in_pcbunset_laddr forgets the quadruple; the exact lookup misses then.
        in_pcbunset_laddr(d);
        assert!(
            in_pcblookup(
                t,
                fsin.sin_addr,
                fsin.sin_port,
                lsin.sin_addr,
                lsin.sin_port,
                0
            )
            .is_none()
        );

        // The iterator sees every control block once, with an aborted walk in between.
        let iter = core::pin::pin!(Inpcb::marker());
        let mut seen = Vec::new();
        let mut inp = None;
        let mut g = t.lock();
        while let Some(i) = in_pcb_iterator(t, &mut g, inp, iter.as_ref()) {
            inp = Some(i);
            seen.push(ptr::from_ref(i));
        }
        let abort = core::pin::pin!(Inpcb::marker());
        let first = in_pcb_iterator(t, &mut g, None, abort.as_ref());
        in_pcb_iterator_abort(t, &mut g, first, abort.as_ref());
        let count = g.inpt_count;
        drop(g);
        assert_eq!(seen.len() as i32, count);

        for i in [a, b, c, d] {
            release(i);
        }
        assert_eq!(t.count(), 1, "the refused one is left");
        teardown();
    }

    #[test]
    fn kern_file_reports_an_inet_socket() {
        // kern.file's Internet fields of a socket over the kernel's pcb (`fill_file_socket`
        // through the accessors): the addresses, ports and table always, the pcb's address
        // (`so_pcb`) and the per-protocol pcb's (`inp_ppcb`) only under show_pointers, and no
        // count of the pcb anywhere.
        use crate::kern::kern_sysctl::fill_file;
        use crate::sys::sysctl::KinfoFile;

        let (_g, _t, p) = setup();
        let t = table(16);
        let inp = pcb(t);
        let so = inp.socket();
        {
            let g = t.lock();
            inp.set_laddr(&g, sin([10, 0, 2, 15], 0).sin_addr);
            inp.set_lport(&g, htons(5353));
            inp.set_faddr(&g, sin([10, 0, 2, 2], 0).sin_addr);
            inp.set_fport(&g, htons(53));
        }
        // A stand-in for the address of a tcpcb (`inp_ppcb`).
        static PPCB: u64 = 0;
        inp.inp_ppcb.set(ptr::from_ref(&PPCB).cast());

        solock(so);
        let mut kf = KinfoFile::zeroed();
        fill_file(&mut kf, None, None, 0, None, None, p, Some(so), false);
        let mut kfp = KinfoFile::zeroed();
        fill_file(&mut kfp, None, None, 0, None, None, p, Some(so), true);
        sounlock(so);

        assert_eq!(kf.so_family, u32::from(AF_INET));
        assert_eq!(kf.so_protocol, IPPROTO_UDP as u32);
        assert_eq!(kf.inp_lport, u32::from(htons(5353)));
        assert_eq!(kf.inp_laddru[0], u32::from_ne_bytes([10, 0, 2, 15]));
        assert_eq!(kf.inp_fport, u32::from(htons(53)));
        assert_eq!(kf.inp_faddru[0], u32::from_ne_bytes([10, 0, 2, 2]));
        assert_eq!(kf.inp_rtableid, 0);
        assert_eq!(kf.so_pcb, -1i64 as u64, "no pointers without show_pointers");
        assert_eq!(kf.inp_ppcb, 0);
        assert_eq!(
            kfp.so_pcb,
            ptr::from_ref(inp).addr() as u64,
            "so_pcb: the pcb"
        );
        assert_eq!(kfp.inp_ppcb, ptr::from_ref(&PPCB).addr() as u64);
        assert_eq!(inp.legacy_count(), 1, "the socket's count only");

        inp.inp_ppcb.set(ptr::null());
        release(inp);
        teardown();
    }

    /// IPv6 helpers shared by the UDP and TCP tests (a test interface with `fd00:77::1/64` whose
    /// output the tests read, address mbufs, packets from `fd00:77::2`), and the `INP_IPV6`
    /// branches of binding, port picking, connecting and the lookups.
    #[cfg(feature = "inet6")]
    pub(crate) mod inet6 {
        use std::sync::Mutex as StdMutex;
        use std::{assert, assert_eq, assert_ne, vec, vec::Vec};

        use core::mem::size_of;
        use core::ptr;
        use core::sync::atomic::Ordering;

        use super::{locked, setup, teardown};
        use crate::kern::uipc_mbuf::{m_freem, m_get};
        use crate::kern::uipc_socket::{soclose, socreate};
        use crate::kern::uipc_socket2::{solock, sounlock};
        use crate::net::if_::tests::{test_ifnet, test_packet};
        use crate::net::if_var::Ifnet;
        use crate::net::route::Rtentry;
        use crate::netinet::in_pcb::*;
        use crate::netinet6::in6::tests::a6;
        use crate::netinet6::in6::{
            IN6ADDR_ANY, In6Addr, SockaddrIn6, in6_prefixlen2mask, in6ifa_ifpwithaddr,
        };
        use crate::netinet6::in6_cksum::in6_cksum;
        use crate::netinet6::in6_pcb::in6_pcblookup;
        use crate::netinet6::in6_var::{IN6_IFF_TENTATIVE, In6Aliasreq, SIOCAIFADDR_IN6};
        use crate::netinet6::nd6::ND6_INFINITE_LIFETIME;
        use crate::sys::endian::htons;
        use crate::sys::errno::Errno;
        use crate::sys::mbuf::{M_DONTWAIT, MT_SONAME, Mbuf, mtod};
        use crate::sys::socket::{AF_INET6, SOCK_DGRAM, Sockaddr};

        /// What the IPv6 test interface was asked to send, as bytes from the IPv6 header on.
        pub(crate) static SENT6: StdMutex<Vec<Vec<u8>>> = StdMutex::new(Vec::new());

        /// Takes what the test interface sent so far.
        pub(crate) fn take_sent6() -> Vec<Vec<u8>> {
            core::mem::take(&mut *SENT6.lock().unwrap_or_else(|e| e.into_inner()))
        }

        /// A driver `ioctl` that accepts the address and multicast requests (and brings the
        /// interface up on the first address).
        ///
        /// # Safety
        ///
        /// `IfIoctlFn`'s contract.
        unsafe fn accepting_ioctl(
            ifp: &'static Ifnet,
            cmd: u64,
            _data: *mut u8,
        ) -> Result<(), Errno> {
            use crate::net::if_::{IFF_RUNNING, IFF_UP};
            use crate::sys::sockio::{SIOCADDMULTI, SIOCDELMULTI, SIOCSIFADDR};
            match cmd {
                SIOCSIFADDR => {
                    ifp.if_flags.set(ifp.if_flags.get() | IFF_UP | IFF_RUNNING);
                    Ok(())
                }
                SIOCADDMULTI | SIOCDELMULTI => Ok(()),
                _ => Err(Errno::ENOTTY),
            }
        }

        /// A driver output that keeps the bytes of what it is handed.
        ///
        /// # Safety
        ///
        /// `IfOutputFn`'s contract.
        unsafe fn capturing_output(
            _ifp: &'static Ifnet,
            m: &'static Mbuf,
            _dst: *const Sockaddr,
            _rt: Option<&'static Rtentry>,
        ) -> Result<(), Errno> {
            let b = crate::netinet::ip_input::tests::bytes(m);
            SENT6.lock().unwrap_or_else(|e| e.into_inner()).push(b);
            m_freem(m);
            Ok(())
        }

        /// An attached Ethernet-like interface `name` with `fd00:77::1/64`, usable at once (its
        /// duplicate address detection skipped), whose output lands in [`SENT6`].
        pub(crate) fn test_if6(name: &[u8]) -> &'static Ifnet {
            let ifp = test_ifnet(name);
            crate::net::if_var::IfnetLegacy::if_ioctl(ifp).set(Some(accepting_ioctl));
            crate::net::if_var::IfnetLegacy::if_output(ifp).set(Some(capturing_output));
            ifp.if_type.set(crate::net::if_types::IFT_ETHER);
            ifp.if_flags.set(crate::net::if_::IFF_MULTICAST);
            ifp.if_mtu.set(1500);
            let mut sdl = crate::net::if_dl::SockaddrDl {
                sdl_alen: 6,
                ..Default::default()
            };
            sdl.sdl_data[..6].copy_from_slice(&crate::netinet::ip_input::tests::OURS);
            crate::net::if_var::if_sadl_test_set(ifp, sdl);
            crate::net::if_::if_attach(ifp);

            let mut ifra = In6Aliasreq::zeroed();
            ifra.ifra_name = ifp.if_xname.get();
            *ifra.ifra_addr_mut() = SockaddrIn6::with_addr(a6("fd00:77::1"));
            ifra.ifra_prefixmask = SockaddrIn6::with_addr(IN6ADDR_ANY);
            in6_prefixlen2mask(&mut ifra.ifra_prefixmask.sin6_addr, 64);
            ifra.ifra_lifetime.ia6t_vltime = ND6_INFINITE_LIFETIME;
            ifra.ifra_lifetime.ia6t_pltime = ND6_INFINITE_LIFETIME;
            // ifconfig's SIOCAIFADDR_IN6 through the kernel's in6_ioctl, over the request's
            // bytes as in6_control hands them.
            let mut req = [0u8; size_of::<In6Aliasreq>()];
            assert!(hal::pod::put(&mut req, 0, &ifra));
            kernel::netinet6::in6::in6_ioctl(SIOCAIFADDR_IN6, &mut req, Some(ifp), true)
                .expect("SIOCAIFADDR_IN6");
            let ia = in6ifa_ifpwithaddr(ifp, &a6("fd00:77::1")).expect("the address");
            ia.ia6_flags.set(ia.ia6_flags.get() & !IN6_IFF_TENTATIVE);
            let _ = take_sent6();
            ifp
        }

        /// `[addr]:port` (host order port).
        pub(crate) fn sin6(addr: In6Addr, port: u16) -> SockaddrIn6 {
            SockaddrIn6 {
                sin6_port: htons(port),
                ..SockaddrIn6::with_addr(addr)
            }
        }

        /// An address mbuf holding `sin6`.
        pub(crate) fn nam6(sin6: SockaddrIn6) -> &'static Mbuf {
            let m = m_get(M_DONTWAIT, MT_SONAME).expect("mbuf");
            m.m_len().set(size_of::<SockaddrIn6>() as u32);
            // SAFETY: a fresh mbuf of `MLEN` bytes.
            unsafe { mtod::<SockaddrIn6>(m).write_unaligned(sin6) };
            m
        }

        /// The packet `fd00:77::2 -> fd00:77::1` of next header `nxt` carrying `payload` (a
        /// transport header and its data), received on `ifp`; the transport checksum at
        /// `cksum_off` in `payload` is filled in unless `cksum_off` is `None`.
        pub(crate) fn packet6(
            ifp: &Ifnet,
            nxt: u8,
            payload: &[u8],
            cksum_off: Option<usize>,
        ) -> &'static Mbuf {
            let mut v = vec![0x60, 0, 0, 0];
            v.extend_from_slice(&(payload.len() as u16).to_be_bytes());
            v.extend_from_slice(&[nxt, 64]);
            v.extend_from_slice(&a6("fd00:77::2").s6_addr);
            v.extend_from_slice(&a6("fd00:77::1").s6_addr);
            v.extend_from_slice(payload);
            if let Some(off) = cksum_off {
                let m = test_packet(&v);
                let sum = in6_cksum(m, nxt, 40, payload.len() as u32);
                m_freem(m);
                v[40 + off..42 + off].copy_from_slice(&sum.to_ne_bytes());
            }
            let m = test_packet(&v);
            m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
            m
        }

        #[test]
        fn inet6_bind_pick_connect_and_lookups() {
            let (_g, _t, p) = setup();
            crate::netinet::udp_usrreq::udp_init();
            let _ifp = test_if6(b"tpcb6");
            let ours = a6("fd00:77::1");

            let so = socreate(i32::from(AF_INET6), SOCK_DGRAM, 0).expect("socket");
            let inp = sotoinpcb(so).expect("attached");
            assert!(inp.has_flags(INP_IPV6), "in_pcballoc marks a PF_INET6 pcb");
            assert_eq!(inp.inp_cksum6.get(), -1);
            let t = inp.table();

            // An explicit port on our address, then the local port lookups.
            locked(|| in_pcbbind(inp, Some(nam6(sin6(ours, 5000))), p)).expect("bind");
            assert_eq!(inp.inp_laddr6.get::<In6Addr>(), ours);
            assert_eq!(inp.inp_lport.get(), htons(5000));
            let found = in_pcblookup_local_lock(
                t,
                &Inpaddru::from_addr6(ours),
                htons(5000),
                INPLOOKUP_IPV6,
                0,
                IN_PCBLOCK_GRAB,
            );
            assert!(found.is_some_and(|f| ptr::eq(f, inp)));
            in_pcbunref(found);
            let any = Inpaddru::from_addr6(IN6ADDR_ANY);
            let lookup = |flags| {
                let f = in_pcblookup_local_lock(t, &any, htons(5000), flags, 0, IN_PCBLOCK_GRAB);
                let hit = f.is_some_and(|f| ptr::eq(f, inp));
                in_pcbunref(f);
                hit
            };
            assert!(!lookup(INPLOOKUP_IPV6), "an exact match only");
            assert!(lookup(INPLOOKUP_IPV6 | INPLOOKUP_WILDCARD));
            assert_eq!(
                locked(|| in_pcbbind(inp, Some(nam6(sin6(ours, 5001))), p)),
                Err(Errno::EINVAL),
                "bound already"
            );

            // The same address and port again; an address that is not ours.
            let so2 = socreate(i32::from(AF_INET6), SOCK_DGRAM, 0).expect("socket");
            let inp2 = sotoinpcb(so2).expect("attached");
            assert_eq!(
                locked(|| in_pcbbind(inp2, Some(nam6(sin6(ours, 5000))), p)),
                Err(Errno::EADDRINUSE)
            );
            assert_eq!(
                locked(|| in_pcbbind(inp2, Some(nam6(sin6(a6("fd00:77::9"), 5000))), p)),
                Err(Errno::EADDRNOTAVAIL)
            );
            // A picked port from the default range, on the unspecified address.
            locked(|| in_pcbbind(inp2, None, p)).expect("anonymous bind");
            let port = i32::from(u16::from_be(inp2.inp_lport.get()));
            assert!(
                (IPPORT_FIRSTAUTO.load(Ordering::Relaxed)
                    ..=IPPORT_LASTAUTO.load(Ordering::Relaxed))
                    .contains(&port),
                "port {port}"
            );
            assert_eq!(inp2.inp_laddr6.get::<In6Addr>(), IN6ADDR_ANY);

            // connect(2) of an unbound socket: in6_pcbconnect binds it to the selected source
            // and a picked port (in_pcbbind_locked with an IPv6 local address).
            let so3 = socreate(i32::from(AF_INET6), SOCK_DGRAM, 0).expect("socket");
            let inp3 = sotoinpcb(so3).expect("attached");
            let peer = a6("fd00:77::2");
            solock(so3);
            in_pcbconnect(inp3, nam6(sin6(peer, 53))).expect("connect");
            assert!(
                in_pcbrtentry(inp3).is_some(),
                "in6_pcbrtentry: the route to the peer"
            );
            sounlock(so3);
            assert_eq!(
                inp3.inp_laddr6.get::<In6Addr>(),
                ours,
                "the source in6_pcbselsrc picked"
            );
            assert_ne!(inp3.inp_lport.get(), 0);
            assert_eq!(inp3.inp_faddr6.get::<In6Addr>(), peer);
            let hit = in6_pcblookup(t, &peer, htons(53), &ours, inp3.inp_lport.get(), 0);
            assert!(hit.is_some_and(|h| ptr::eq(h, inp3)));
            in_pcbunref(hit);

            // getpeername(2) and getsockname(2) of an INP_IPV6 pcb.
            let m = m_get(M_DONTWAIT, MT_SONAME).expect("mbuf");
            in_setpeeraddr(inp3, m);
            // SAFETY: `in6_setpeeraddr` wrote a `sockaddr_in6`.
            let got = unsafe { mtod::<SockaddrIn6>(m).read_unaligned() };
            assert_eq!((got.sin6_family, got.sin6_port), (AF_INET6, htons(53)));
            assert_eq!(got.sin6_addr, peer);
            in_setsockaddr(inp3, m);
            // SAFETY: as above, `in6_setsockaddr`'s.
            let got = unsafe { mtod::<SockaddrIn6>(m).read_unaligned() };
            assert_eq!(got.sin6_addr, ours);
            m_freem(Some(m));

            // in_pcbunset_laddr forgets both IPv6 addresses.
            in_pcbunset_laddr(inp3);
            assert_eq!(
                (inp3.inp_laddr6.get::<In6Addr>(), inp3.inp_faddr6.get()),
                (IN6ADDR_ANY, IN6ADDR_ANY)
            );

            for so in [so, so2, so3] {
                soclose(so, 0).expect("close");
            }
            let _ = take_sent6();
            teardown();
        }

        #[test]
        fn kern_file_reports_an_inet6_socket() {
            use crate::kern::kern_sysctl::fill_file;
            use crate::netinet::udp_usrreq::{udp_bind, udp_init};
            use crate::sys::sysctl::KinfoFile;

            let (_g, _t, p) = setup();
            udp_init();
            let _ifp = test_if6(b"tkf6");
            let so = socreate(i32::from(AF_INET6), SOCK_DGRAM, 0).expect("socket");
            solock(so);
            udp_bind(so, nam6(sin6(a6("fd00:77::1"), 5353)), p).expect("bind");
            let mut kf = KinfoFile::zeroed();
            fill_file(&mut kf, None, None, 0, None, None, p, Some(so), false);
            sounlock(so);

            assert_eq!(kf.so_family, u32::from(AF_INET6));
            assert_eq!(kf.inp_lport, u32::from(htons(5353)));
            let ours = a6("fd00:77::1");
            for (i, w) in kf.inp_laddru.iter().enumerate() {
                assert_eq!(*w, ours.s6_addr32(i), "inp_laddru[{i}]");
            }
            assert_eq!(kf.inp_faddru, [0; 4]);
            assert_eq!(kf.inp_ppcb, 0, "no pointers without show_pointers");

            soclose(so, 0).expect("close");
            teardown();
        }
    }
}
/* </TESTS> */
