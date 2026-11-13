/*	$KAME: in6_src.c,v 1.36 2001/02/06 04:08:17 itojun Exp $	*/
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
 *	@(#)in_pcb.c	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 source address and route selection, the hop limit of a pcb, and the embedding of
//! scope zone ids in addresses: `netinet6/in6_src.c`, the kernel's since N6.5a
//! (`kernel::netinet6::in6_src`), with LZ's signatures for `bsd`'s callers.
//!
//! Upstream: sys/netinet6/in6_src.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - The kernel's functions take the two members of the options `in6_src.c` reads, the packet
//!   info and the multicast interface; these wrappers copy them out of `bsd`'s structures
//!   (`struct ip6_pktopts`' `ip6po_pktinfo`, the pcb's `inp_moptions6`) until the pcb's options
//!   are the kernel's (N6.5c), and lend the packet info whose address `in6_pcbselsrc`
//!   overrides. `in6_selectroute` answers the route as the `&'static Rtentry` LZ's callers
//!   keep (`as_static`).

use crate::net::if_::net_held;
use crate::net::route::{Route, Rtentry, as_static};
use crate::netinet::in_pcb::Inpcb;
use crate::netinet::in_pcb::InpcbLegacy as _;
use crate::netinet6::in6::{In6Addr, In6Pktinfo, SockaddrIn6};
use crate::netinet6::ip6_var::{Ip6Moptions, Ip6Pktopts};
use crate::sys::errno::Errno;

pub use kernel::netinet6::in6_src::{in6_clearscope, in6_recoverscope, in6_selecthlim};

/// `opts->ip6po_pktinfo`, lent to `f` for the call (`None` without options or packet info).
fn with_pktinfo<R>(opts: Option<&Ip6Pktopts>, f: impl FnOnce(Option<&mut In6Pktinfo>) -> R) -> R {
    let pi = opts.and_then(|o| o.ip6po_pktinfo);
    // SAFETY: `ip6po_pktinfo` is only ever set (`ip6_setpktopt`, `ip6_copypktopts`) to a
    // `malloc(M_IP6OPT)` block of an `In6Pktinfo` the options own, initialised whole and
    // freed only by `ip6_clearpktopts`, which needs the options mutably: `opts` is borrowed
    // here for the call, so the block lives through `f`. The block is reached only through
    // its options' pointer (no other copy of the pointer is kept: `ip6_copypktopts` makes a
    // new block), and `bsd`'s callers run the call under the socket lock that serialises
    // every user of the options, so the `&mut` is the only reference to it while `f` runs.
    f(pi.map(|p| unsafe { &mut *p.as_ptr() }))
}

/// `inp->inp_moptions6->im6o_ifidx`: the pcb's multicast interface, `None` without multicast
/// options.
fn moptions_ifidx(inp: &Inpcb) -> Option<u16> {
    // SAFETY: `inp_moptions6` is only ever set (`ip6_setmoptions`) to a live
    // `malloc(M_IPMOPTS)` block of an `Ip6Moptions`, initialised whole, freed only by
    // `ip6_freemoptions` once the word is cleared (`in_pcbdetach`, `ip6_setmoptions`), both
    // under the socket lock `bsd`'s callers hold for this call; the one byte-sized field is
    // read by value.
    inp.moptions6_ptr()
        .map(|p| unsafe { (*p.as_ptr()).im6o_ifidx })
}

/// `in6_pcbselsrc(&in6src, dstsock, inp, opts)`: the kernel's, its source stored in `in6src`,
/// under the net lock the caller holds.
pub fn in6_pcbselsrc(
    in6src: &mut In6Addr,
    dstsock: &SockaddrIn6,
    inp: &Inpcb,
    opts: Option<&Ip6Pktopts>,
) -> Result<(), Errno> {
    let ifidx = moptions_ifidx(inp);
    *in6src = with_pktinfo(opts, |pi| {
        kernel::netinet6::in6_src::in6_pcbselsrc(
            dstsock,
            inp,
            pi,
            ifidx,
            &net_held("in6_pcbselsrc"),
        )
    })?;
    Ok(())
}

/// `in6_selectsrc(&in6src, dstsock, mopts, rtableid)`: the kernel's, its source stored in
/// `in6src`.
pub fn in6_selectsrc(
    in6src: &mut In6Addr,
    dstsock: &SockaddrIn6,
    mopts: Option<&Ip6Moptions>,
    rtableid: u32,
) -> Result<(), Errno> {
    *in6src =
        kernel::netinet6::in6_src::in6_selectsrc(dstsock, mopts.map(|m| m.im6o_ifidx), rtableid)?;
    Ok(())
}

/// `in6_selectroute(dst, opts, ro, rtableid)`: the kernel's, the route as the `&'static
/// Rtentry` LZ's callers keep, under the net lock the caller holds.
pub fn in6_selectroute(
    dst: &In6Addr,
    opts: Option<&Ip6Pktopts>,
    ro: &mut Route,
    rtableid: u32,
) -> Option<&'static Rtentry> {
    let ifindex = with_pktinfo(opts, |pi| pi.map_or(0, |p| p.ipi6_ifindex));
    kernel::netinet6::in6_src::in6_selectroute(
        dst,
        ifindex,
        ro,
        rtableid,
        &net_held("in6_selectroute"),
    )
    .map(as_static)
}

/// `in6_embedscope(in6, sin6, outputopts6, moptions6)`: the kernel's, over the options'
/// interfaces.
pub fn in6_embedscope(
    in6: &mut In6Addr,
    sin6: &SockaddrIn6,
    outputopts6: Option<&Ip6Pktopts>,
    moptions6: Option<&Ip6Moptions>,
) -> Result<(), Errno> {
    let pi = with_pktinfo(outputopts6, |pi| pi.map_or(0, |p| p.ipi6_ifindex));
    kernel::netinet6::in6_src::in6_embedscope(in6, sin6, pi, moptions6.map_or(0, |m| m.im6o_ifidx))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `in6_src.c`: scope embedding and recovery, the source address of
    // link-local and multicast destinations, route selection with a packet info.

    use core::ptr;

    use super::*;
    use crate::netinet6::in6::IN6ADDR_ANY;
    use crate::netinet6::in6::tests::{a6, setup, test_ia6, test_if};
    use crate::netinet6::in6_var::IN6_IFF_TENTATIVE;
    use crate::netinet6::in6_var::ia6_in6;
    use crate::sys::endian::htons;
    use crate::sys::systm::{net_lock, net_unlock};

    fn sin6(a: &str, scope_id: u32) -> SockaddrIn6 {
        let mut s = SockaddrIn6::with_addr(a6(a));
        s.sin6_scope_id = scope_id;
        s
    }

    fn pktopts(ifindex: u32, addr: In6Addr) -> (Ip6Pktopts, &'static mut In6Pktinfo) {
        let pi: &'static mut In6Pktinfo = std::boxed::Box::leak(std::boxed::Box::new(In6Pktinfo {
            ipi6_addr: addr,
            ipi6_ifindex: ifindex,
        }));
        let opts = Ip6Pktopts {
            ip6po_pktinfo: ptr::NonNull::new(ptr::from_mut(pi)),
            ..Ip6Pktopts::default()
        };
        (opts, pi)
    }

    #[test]
    fn scope_is_embedded_recovered_and_cleared() {
        let _g = setup();
        let ifp = test_if(b"tsc0");
        let idx = ifp.if_index.get();

        // sin6_scope_id of a link-local address goes into s6_addr16[1].
        let mut out = In6Addr::default();
        in6_embedscope(&mut out, &sin6("fe80::1", idx), None, None).expect("embed");
        assert_eq!(out.s6_addr16(1), htons(idx as u16));
        assert_eq!(&out.s6_addr[..2], [0xfe, 0x80]);
        // A global address is copied as it is, whatever the scope id.
        in6_embedscope(&mut out, &sin6("2001:db8::1", idx), None, None).expect("embed");
        assert_eq!(out, a6("2001:db8::1"));
        // An unknown interface.
        assert_eq!(
            in6_embedscope(&mut out, &sin6("fe80::1", 4000), None, None),
            Err(Errno::ENXIO)
        );
        // No scope id: the zone is left alone.
        in6_embedscope(&mut out, &sin6("fe80::1", 0), None, None).expect("embed");
        assert_eq!(out, a6("fe80::1"));

        // The packet info's interface wins over sin6_scope_id; for a multicast address, the
        // multicast options' wins when there is no packet info.
        let other = test_if(b"tsc1");
        let (opts, _pi) = pktopts(other.if_index.get(), IN6ADDR_ANY);
        in6_embedscope(&mut out, &sin6("fe80::1", idx), Some(&opts), None).expect("embed");
        assert_eq!(out.s6_addr16(1), htons(other.if_index.get() as u16));
        let mopts = Ip6Moptions {
            im6o_memberships: crate::sys::queue::ListHead::new(),
            im6o_ifidx: other.if_index.get() as u16,
            im6o_hlim: 1,
            im6o_loop: 1,
        };
        in6_embedscope(&mut out, &sin6("ff02::1", idx), None, Some(&mopts)).expect("embed");
        assert_eq!(out.s6_addr16(1), htons(other.if_index.get() as u16));
        // ... but not for a unicast address.
        in6_embedscope(&mut out, &sin6("fe80::1", idx), None, Some(&mopts)).expect("embed");
        assert_eq!(out.s6_addr16(1), htons(idx as u16));

        // And back.
        let mut sa = SockaddrIn6::zeroed();
        in6_recoverscope(&mut sa, &out);
        assert_eq!(sa.sin6_addr, a6("fe80::1"));
        assert_eq!(sa.sin6_scope_id, idx);
        in6_recoverscope(&mut sa, &a6("2001:db8::1"));
        assert_eq!(sa.sin6_addr, a6("2001:db8::1"));
        assert_eq!(sa.sin6_scope_id, 0);
        // Not embedded (zero zone): nothing to recover.
        in6_recoverscope(&mut sa, &a6("fe80::9"));
        assert_eq!(sa.sin6_scope_id, 0);

        let mut ll = a6("fe80:5::1");
        in6_clearscope(&mut ll);
        assert_eq!(ll, a6("fe80::1"));
        let mut g = a6("2001:5::1");
        in6_clearscope(&mut g);
        assert_eq!(g, a6("2001:5::1"));
        let mut m = a6("ff02:5::1");
        in6_clearscope(&mut m);
        assert_eq!(m, a6("ff02::1"));
        let mut m = a6("ff01:5::1");
        in6_clearscope(&mut m);
        assert_eq!(m, a6("ff01::1"));
    }

    #[test]
    fn source_for_a_scoped_or_multicast_destination() {
        let _g = setup();
        let if1 = test_if(b"tss0");
        let if2 = test_if(b"tss1");
        let ll1 = test_ia6(if1, a6("fe80:1::1"), 0);
        let bare = test_if(b"tss2");
        let ll2 = test_ia6(if2, a6("fe80:2::1"), 0);
        test_ia6(if2, a6("2001:db8::5"), 0);
        net_lock();
        let mut src = In6Addr::default();

        // A link-local destination with a scope id: the address of that interface.
        in6_selectsrc(&mut src, &sin6("fe80::99", if2.if_index.get()), None, 0).expect("src");
        assert_eq!(src, ia6_in6(ll2));
        in6_selectsrc(&mut src, &sin6("ff02::1", if1.if_index.get()), None, 0).expect("src");
        assert_eq!(src, ia6_in6(ll1));
        assert_eq!(
            in6_selectsrc(&mut src, &sin6("fe80::99", 4000), None, 0),
            Err(Errno::ENXIO)
        );
        assert_eq!(
            in6_selectsrc(&mut src, &sin6("fe80::99", bare.if_index.get()), None, 0),
            Err(Errno::EADDRNOTAVAIL)
        );

        // A multicast destination: the interface of the multicast options.
        let mopts = Ip6Moptions {
            im6o_memberships: crate::sys::queue::ListHead::new(),
            im6o_ifidx: if2.if_index.get() as u16,
            im6o_hlim: 1,
            im6o_loop: 1,
        };
        in6_selectsrc(&mut src, &sin6("ff05::1", 0), Some(&mopts), 0).expect("src");
        assert_eq!(src, a6("2001:db8::5"));
        // None given: nothing to choose from.
        assert_eq!(
            in6_selectsrc(&mut src, &sin6("ff05::1", 0), None, 0),
            Err(Errno::EADDRNOTAVAIL)
        );
        // A unicast global destination is the route's business.
        assert_eq!(
            in6_selectsrc(&mut src, &sin6("2001:db8::99", 0), Some(&mopts), 0),
            Err(Errno::EADDRNOTAVAIL)
        );
        net_unlock();
    }

    #[test]
    fn a_tentative_address_is_not_a_source() {
        let _g = setup();
        let ifp = test_if(b"tst0");
        test_ia6(ifp, a6("fe80:1::1"), IN6_IFF_TENTATIVE);
        net_lock();
        let mut src = In6Addr::default();
        assert_eq!(
            in6_selectsrc(&mut src, &sin6("fe80::99", ifp.if_index.get()), None, 0),
            Err(Errno::EADDRNOTAVAIL)
        );
        net_unlock();
    }
}
/* </TESTS> */
