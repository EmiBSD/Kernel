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

/*
 * Copyright (c) 1982, 1986, 1990, 1993, 1995
 *	Regents of the University of California.  All rights reserved.
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
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 protocol control blocks: binding, connecting, lookups and notifications of
//! `INP_IPV6` pcbs: `netinet6/in6_pcb.c` (prototypes in `<netinet/in_pcb.h>`), the kernel's
//! since N6.5c (`kernel::netinet6::in6_pcb`), with LZ's signatures for `bsd`'s callers.
//!
//! Upstream: sys/netinet6/in6_pcb.c @ 3ce1f3f79392
//!
//! The licence block carries the NRL notice with its advertising clause, accepted as
//! BSD-4 (`.claude/rules/scope-and-stubs.md`).
//!
//! The lookups and the IPv6 hash are the kernel's since N6.4c1 (D-17's first half), the rest
//! (the address checks, connecting, the addresses of `getsockname(2)`/`getpeername(2)`, the
//! notifications, the route, the accepted connection's addresses) since N6.5c, with
//! `in_pcb.c`'s `#ifdef INET6` branches (`netinet/in_pcb.rs`'s adapters call the kernel's
//! `in_pcbbind_locked`, `in_pcbconnect` and `in_pcbrtentry` for both families). These
//! adapters take `bsd`'s shapes: the `&'static Inpcb` held by a count, the name mbuf, the
//! net lock's proof made through the `SaToken` (the hash's, the address check's and the
//! notification's went with their last callers, raw IPv6 and divert6, at N6.6b1: their tests
//! call the kernel's).
//!
//! ## Deviations
//! - `NSTOEPLITZ` is configured: the flow id of a connected or accepted pcb is the kernel's
//!   (`in6_pcb_newflow` at the end of `in6_pcbconnect` and `in6_pcbset_addr`, N6.6a1).

use core::mem::size_of;

use crate::kern::subr_prf::panic;
use crate::netinet::in_pcb::{Inpcb, Inpcbtable};
use crate::netinet6::in6::{In6Addr, SockaddrIn6};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, mut_of};
use kernel::netinet::in_pcb as k;
use kernel::netinet6::in6_pcb as k6;

pub use kernel::netinet6::in6_pcb::ZEROIN6_ADDR;

impl k::Hdr6 for crate::netinet::ip6::Ip6Hdr {
    fn from_bytes(b: [u8; 40]) -> Self {
        let mut src = [0; 16];
        let mut dst = [0; 16];
        src.copy_from_slice(&b[8..24]);
        dst.copy_from_slice(&b[24..40]);
        Self {
            ip6_flow: u32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
            ip6_plen: u16::from_ne_bytes([b[4], b[5]]),
            ip6_nxt: b[6],
            ip6_hlim: b[7],
            ip6_src: In6Addr::new(src),
            ip6_dst: In6Addr::new(dst),
        }
    }
    fn to_bytes(&self) -> [u8; 40] {
        let mut b = [0; 40];
        b[0..4].copy_from_slice(&self.ip6_flow.to_ne_bytes());
        b[4..6].copy_from_slice(&self.ip6_plen.to_ne_bytes());
        b[6] = self.ip6_nxt;
        b[7] = self.ip6_hlim;
        b[8..24].copy_from_slice(&self.ip6_src.s6_addr);
        b[24..40].copy_from_slice(&self.ip6_dst.s6_addr);
        b
    }
}

/// Writes `sin6` as the address in `nam` (`nam->m_len = sizeof(struct sockaddr_in6)`, the
/// structure at `mtod(nam)`).
fn set_nam6(nam: &Mbuf, sin6: SockaddrIn6) {
    let nam = mut_of(nam);
    nam.m_len().set(size_of::<SockaddrIn6>() as u32);
    // An `MT_SONAME` mbuf of `MLEN` bytes, writable: the C's store cannot fail.
    let written = nam.data_mut().is_some_and(|d| hal::pod::put(d, 0, &sin6));
    if !written {
        panic(format_args!("set_nam6: a name mbuf without room"));
    }
}

/// `in6_setsockaddr(inp, nam)`: the kernel's local address of `inp`, into `nam`. This
/// services the getsockname(2) call.
pub fn in6_setsockaddr(inp: &Inpcb, nam: &Mbuf) {
    set_nam6(nam, k6::in6_setsockaddr(inp));
}

/// `in6_setpeeraddr(inp, nam)`: the kernel's foreign address of `inp`, into `nam`. This
/// services the getpeername(2) call.
pub fn in6_setpeeraddr(inp: &Inpcb, nam: &Mbuf) {
    set_nam6(nam, k6::in6_setpeeraddr(inp));
}

/// `in6_pcblookup`: the connected control block for `faddr.fport <-> laddr.lport`,
/// referenced (the kernel's lookup, held as `bsd` holds a pcb).
pub fn in6_pcblookup(
    table: &Inpcbtable,
    faddr: &In6Addr,
    fport: u16,
    laddr: &In6Addr,
    lport: u16,
    rtable: u32,
) -> Option<&'static Inpcb> {
    // The answer's count taken before the mutex is let go (in6_pcb.c:692-693).
    // The hash before the mutex (in6_pcb.c:680-683).
    let rdomain = kernel::net::rtable::rtable_l2(rtable);
    let (f, l) = (faddr.s6_addr, laddr.s6_addr);
    let hash = k6::in6_pcbhash(table, rdomain, &f, fport, &l, lport);
    let mut g = table.lock();
    let inp = k6::in6_pcbhash_lookup(&mut g, hash, rdomain, &f, fport, &l, lport);
    let inp = inp.map(|h| crate::netinet::in_pcb::legacy_hold(h, &g));
    drop(g);
    inp
}

/// `in6_pcblookup_listen`: the listening control block for `laddr.lport_arg`, referenced.
pub fn in6_pcblookup_listen(
    table: &Inpcbtable,
    laddr: &In6Addr,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<&'static Inpcb> {
    // pf's divert key before the mutex, the count before it is let go.
    let l = k6::in6_pcblookup_listen_keys(table, laddr, lport_arg, m, rtable)?;
    let mut g = table.lock();
    let inp = k6::in6_pcblookup_listen_held(&mut g, table, &l);
    let inp = inp.map(|h| crate::netinet::in_pcb::legacy_hold(h, &g));
    drop(g);
    inp
}

/// `in6_pcbset_addr(inp, fsin6, lsin6, rtableid)`: the kernel's (which sets the flow id).
pub fn in6_pcbset_addr(
    inp: &'static Inpcb,
    fsin6: &SockaddrIn6,
    lsin6: &SockaddrIn6,
    rtableid: u32,
) -> Result<(), Errno> {
    k6::in6_pcbset_addr(inp, fsin6, lsin6, rtableid)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests for the IPv6 control blocks: the spread of `in6_pcbhash`, the exact and
    // listening lookups over a table of bound, connected and wildcard IPv6 control blocks,
    // `in6_pcbset_addr`'s conflict check, the addresses `getsockname(2)`/`getpeername(2)`
    // return (the scope zone recovered), `in6_pcbnotify`'s matching and the checks of
    // `in6_pcbaddrisavail`.
    //
    // The control blocks are placed in their hash chains by hand (`place6`), with the hash
    // `in_pcbrehash` computes for `INP_IPV6` ones (`in6_pcbhash`).

    use std::boxed::Box;
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use crate::netinet::in_pcb::sotoinpcb;
    use crate::sys::socketvar::Socket;

    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    use core::ptr;

    use super::*;
    use crate::kern::uipc_mbuf::m_get;
    use crate::kern::uipc_socket::soalloc;
    use crate::kern::uipc_socket::sorele;
    use crate::kern::uipc_socket2::{solock, sounlock};
    use crate::net::route::{RTF_HOST, route6_mpath};
    use crate::netinet::in_pcb::INP_IPV6;
    use crate::netinet::in_pcb::in_pcbunref;
    use crate::netinet::in_pcb::tests::{setup, teardown};
    use crate::netinet::in_pcb::{in_pcballoc, in_pcbdetach, in_pcbinit};
    use crate::netinet6::in6::IN6ADDR_ANY;
    use crate::netinet6::in6::tests::a6;
    use crate::sys::endian::htons;
    use crate::sys::mbuf::mtod;
    use crate::sys::mbuf::{M_DONTWAIT, M_WAIT, MT_SONAME};
    use crate::sys::protosw::{PRC_HOSTDEAD, PRC_MSGSIZE, PRC_UNREACH_PORT};
    use crate::sys::socket::{AF_INET6, SO_BINDANY, SOCK_RAW};
    use crate::sys::socketvar::{SS_NOFDREF, soref};

    /// A table of `hashsize` buckets.
    fn table(hashsize: i32) -> &'static Inpcbtable {
        let t: &'static Inpcbtable = Box::leak(Box::new(Inpcbtable::new()));
        in_pcbinit(t, hashsize);
        t
    }

    /// A data-only raw IPv6 row of the inet6 domain for these tests, by their row and not by
    /// an index of `INET6SW` (N6.6b0); a raw row (`pr_rip`, N6.6b1), its sockets held by a
    /// witness as every internet socket's.
    static RAW6_ROW: crate::sys::protosw::Protosw = crate::sys::protosw::Protosw {
        pr_type: SOCK_RAW as i16,
        pr_protocol: crate::netinet::in_::IPPROTO_RAW as i16,
        pr_flags: crate::sys::protosw::PR_ATOMIC | crate::sys::protosw::PR_ADDR,
        pr_rip: true,
        ..crate::sys::protosw::Protosw::new(&crate::netinet6::in6_proto::INET6DOMAIN)
    };

    /// An `INP_IPV6` control block of a raw IPv6 socket in `table`.
    fn pcb6(table: &'static Inpcbtable) -> &'static Inpcb {
        let so = soalloc(&RAW6_ROW, SOCK_RAW, M_WAIT).expect("socket");
        in_pcballoc(so, table, M_WAIT).expect("in_pcballoc");
        let inp = pcb_of(so);
        assert!(inp.has_flags(INP_IPV6), "in_pcballoc marks PF_INET6 pcbs");
        inp
    }

    /// The control block `in_pcballoc` gave `so` (`sotoinpcb(so)`, read without its family
    /// check).
    pub(crate) fn pcb_of(so: &Socket) -> &'static Inpcb {
        sotoinpcb(so).expect("attached")
    }

    /// Gives `inp` the quadruple `laddr.lport <-> faddr.fport` (host order ports) and moves it
    /// to the hash chain `in6_pcbhash` picks.
    fn place6(inp: &'static Inpcb, laddr: In6Addr, lport: u16, faddr: In6Addr, fport: u16) {
        let t = inp.table();
        let mut g = t.lock();
        inp.set_laddr6(&g, laddr);
        inp.set_lport(&g, htons(lport));
        inp.set_faddr6(&g, faddr);
        inp.set_fport(&g, htons(fport));
        kernel::netinet::in_pcb::in_pcbrehash(&mut g, inp);
    }

    /// Detaches the control block and lets the socket go, as `rip6_detach` and `soclose` do.
    fn release(inp: &'static Inpcb) {
        let so = inp.socket();
        let _ = soref(Some(so));
        solock(so);
        so.set_state_legacy(SS_NOFDREF);
        in_pcbdetach(inp);
        sounlock(so);
        sorele(so);
    }

    /// `addr.port` (host order) as a `sockaddr_in6`.
    fn sin6(addr: In6Addr, port: u16) -> SockaddrIn6 {
        SockaddrIn6 {
            sin6_port: htons(port),
            ..SockaddrIn6::with_addr(addr)
        }
    }

    /// Whether the lookup found `want`; drops the reference it took.
    fn found(hit: Option<&'static Inpcb>, want: &Inpcb) -> bool {
        let r = hit.is_some_and(|h| ptr::eq(h, want));
        in_pcbunref(hit);
        r
    }

    #[test]
    fn in6_pcbhash_spreads_quadruples_over_the_buckets() {
        let (_g, _t, _p) = setup();
        let t = table(64);
        let buckets = ((t.size() as u64 - 1) + 1) as usize;
        let mut count = std::vec![0u32; buckets];
        let l = a6("fd00:77::1");
        for i in 0..4096u32 {
            let mut f = a6("fd00:77::2");
            f.set_s6_addr32(3, i / 64);
            let h = hash6(t, 0, &f, htons(443), &l, htons(1024 + (i % 64) as u16));
            count[(h & (t.size() as u64 - 1)) as usize] += 1;
        }
        let mean = 4096 / buckets as u32;
        let (min, max) = (count.iter().min(), count.iter().max());
        assert!(min.is_some_and(|&m| m >= mean / 3), "{count:?}");
        assert!(max.is_some_and(|&m| m <= mean * 3), "{count:?}");

        // The hash is a function of the whole quadruple and the routing domain.
        let f = a6("fd00:77::2");
        let h = hash6(t, 0, &f, htons(1), &l, htons(2));
        assert_eq!(h, hash6(t, 0, &f, htons(1), &l, htons(2)));
        assert_ne!(h, hash6(t, 1, &f, htons(1), &l, htons(2)));
        assert_ne!(h, hash6(t, 0, &l, htons(1), &f, htons(2)));
        assert_ne!(h, hash6(t, 0, &f, htons(2), &l, htons(1)));
        teardown();
    }

    #[test]
    fn exact_and_listen_lookups_over_bound_connected_and_wildcard_pcbs() {
        let (_g, _t, _p) = setup();
        let t = table(8);
        let (l, f, g) = (a6("fd00:77::1"), a6("fd00:77::2"), a6("fd00:77::3"));

        let conn = pcb6(t);
        place6(conn, l, 1000, f, 2000);
        let bound = pcb6(t);
        place6(bound, l, 80, IN6ADDR_ANY, 0);
        let wild80 = pcb6(t);
        place6(wild80, IN6ADDR_ANY, 80, IN6ADDR_ANY, 0);
        let wild22 = pcb6(t);
        place6(wild22, IN6ADDR_ANY, 22, IN6ADDR_ANY, 0);

        // The exact lookup: the whole quadruple, nothing else.
        let hit = in6_pcblookup(t, &f, htons(2000), &l, htons(1000), 0);
        assert!(found(hit, conn));
        assert!(in6_pcblookup(t, &f, htons(2001), &l, htons(1000), 0).is_none());
        assert!(in6_pcblookup(t, &g, htons(2000), &l, htons(1000), 0).is_none());
        assert!(in6_pcblookup(t, &f, htons(2000), &IN6ADDR_ANY, htons(1000), 0).is_none());
        // Not the listeners either.
        assert!(in6_pcblookup(t, &f, htons(2000), &l, htons(80), 0).is_none());

        // The listen lookup: the bound address first, then the wildcard.
        assert!(found(
            in6_pcblookup_listen(t, &l, htons(80), None, 0),
            bound
        ));
        assert!(found(
            in6_pcblookup_listen(t, &g, htons(80), None, 0),
            wild80
        ));
        assert!(found(
            in6_pcblookup_listen(t, &l, htons(22), None, 0),
            wild22
        ));
        assert!(in6_pcblookup_listen(t, &l, htons(23), None, 0).is_none());
        // A connected control block is no listener.
        assert!(in6_pcblookup_listen(t, &l, htons(1000), None, 0).is_none());

        // in6_pcbset_addr refuses a quadruple that is taken.
        let other = pcb6(t);
        assert_eq!(
            in6_pcbset_addr(other, &sin6(f, 2000), &sin6(l, 1000), 0),
            Err(Errno::EADDRINUSE)
        );

        for i in [conn, bound, wild80, wild22, other] {
            release(i);
        }
        assert_eq!(t.count(), 0);
        teardown();
    }

    #[test]
    fn getsockname_and_getpeername_recover_the_scope_zone() {
        let (_g, _t, _p) = setup();
        let t = table(1);
        let inp = pcb6(t);
        // fe80::1 on the interface of index 3, as the kernel keeps it (the zone embedded).
        let mut ll = a6("fe80::1");
        ll.set_s6_addr16(1, htons(3));
        let g = inp.table().lock();
        inp.set_laddr6(&g, ll);
        inp.set_lport(&g, htons(546));
        inp.set_faddr6(&g, a6("fd00:77::2"));
        inp.set_fport(&g, htons(547));
        drop(g);

        let nam = m_get(M_DONTWAIT, MT_SONAME).expect("mbuf");
        // in6_sockaddr(so, nam), without its sotoinpcb.
        in6_setsockaddr(inp, nam);
        assert_eq!(nam.m_len().get() as usize, size_of::<SockaddrIn6>());
        // SAFETY: in6_setsockaddr wrote a `sockaddr_in6`.
        let s = unsafe { mtod::<SockaddrIn6>(nam).read_unaligned() };
        assert_eq!((s.sin6_len, s.sin6_family), (28, AF_INET6));
        assert_eq!(s.sin6_port, htons(546));
        assert_eq!(s.sin6_addr, a6("fe80::1"));
        assert_eq!(s.sin6_scope_id, 3);

        in6_setpeeraddr(inp, nam);
        // SAFETY: in6_setpeeraddr wrote a `sockaddr_in6`.
        let s = unsafe { mtod::<SockaddrIn6>(nam).read_unaligned() };
        assert_eq!(s.sin6_port, htons(547));
        assert_eq!(s.sin6_addr, a6("fd00:77::2"));
        assert_eq!(s.sin6_scope_id, 0);
        crate::kern::uipc_mbuf::m_freem(nam);
        release(inp);
        teardown();
    }

    /// The kernel's `in6_pcbhash` of the quadruple, over the addresses' bytes.
    fn hash6(
        t: &Inpcbtable,
        rdomain: u32,
        faddr: &In6Addr,
        fport: u16,
        laddr: &In6Addr,
        lport: u16,
    ) -> u64 {
        k6::in6_pcbhash(t, rdomain, &faddr.s6_addr, fport, &laddr.s6_addr, lport)
    }

    /// The kernel's `in6_pcbnotify` under the caller's net lock (its proof lent), `notify`
    /// handed each matching control block as `bsd` holds one (`inp_static`); `cmdarg` is not
    /// used, as in the C.
    #[allow(clippy::too_many_arguments)] // in6_pcbnotify's arguments
    fn notify6(
        table: &Inpcbtable,
        dst: &SockaddrIn6,
        fport_arg: u32,
        src: Option<&SockaddrIn6>,
        lport_arg: u32,
        rtable: u32,
        cmd: i32,
        _cmdarg: Option<u32>,
        notify: Option<fn(&'static Inpcb, Option<Errno>)>,
    ) {
        let lend = crate::net::if_::net_held("in6_pcbnotify");
        match notify {
            Some(n) => {
                let mut f =
                    |inp: &Inpcb, _w: &mut kernel::net::socket::SoLocked<'_>, e: Option<Errno>| {
                        n(crate::netinet::in_pcb::inp_static(inp), e)
                    };
                k6::in6_pcbnotify(
                    table,
                    dst,
                    fport_arg,
                    src,
                    lport_arg,
                    rtable,
                    cmd,
                    lend.net(),
                    Some(&mut f),
                );
            }
            None => k6::in6_pcbnotify(
                table,
                dst,
                fport_arg,
                src,
                lport_arg,
                rtable,
                cmd,
                lend.net(),
                None,
            ),
        }
    }

    /// What `record` saw: the number of calls, the local port and errno of the last one.
    static NOTIFIED: AtomicUsize = AtomicUsize::new(0);
    static LAST_LPORT: AtomicU32 = AtomicU32::new(0);
    static LAST_ERRNO: AtomicU32 = AtomicU32::new(0);

    /// An `InpNotifyFn` that records its calls.
    fn record(inp: &'static Inpcb, errno: Option<Errno>) {
        NOTIFIED.fetch_add(1, Ordering::Relaxed);
        LAST_LPORT.store(
            u32::from(u16::from_be(inp.inp_lport.get())),
            Ordering::Relaxed,
        );
        LAST_ERRNO.store(errno.map_or(0, |e| e as u32), Ordering::Relaxed);
    }

    #[test]
    fn in6_pcbnotify_reaches_the_matching_connections() {
        let (_g, _t, _p) = setup();
        let t = table(4);
        let (l, f, g) = (a6("fd00:77::1"), a6("fd00:77::2"), a6("fd00:77::3"));
        let a = pcb6(t);
        place6(a, l, 1000, f, 2000);
        let b = pcb6(t);
        place6(b, l, 1001, f, 2000);
        let c = pcb6(t);
        place6(c, l, 1000, g, 2000);
        let pcbs: Vec<_> = std::vec![a, b, c];

        // The ctlinput callers' net lock (in_pcbsolock asserts it, as in_pcb.c:626).
        crate::sys::systm::net_lock();
        // A message about one connection: its ports and addresses select it.
        NOTIFIED.store(0, Ordering::Relaxed);
        notify6(
            t,
            &sin6(f, 0),
            u32::from(htons(2000)),
            Some(&sin6(l, 0)),
            u32::from(htons(1000)),
            0,
            PRC_UNREACH_PORT,
            None,
            Some(record),
        );
        assert_eq!(NOTIFIED.load(Ordering::Relaxed), 1);
        assert_eq!(LAST_LPORT.load(Ordering::Relaxed), 1000);
        assert_eq!(
            LAST_ERRNO.load(Ordering::Relaxed),
            Errno::ECONNREFUSED as u32
        );

        // A dead host: every connection to it, whatever the ports.
        NOTIFIED.store(0, Ordering::Relaxed);
        notify6(
            t,
            &sin6(f, 0),
            0,
            None,
            0,
            0,
            PRC_HOSTDEAD,
            None,
            Some(record),
        );
        assert_eq!(NOTIFIED.load(Ordering::Relaxed), 2);
        assert_eq!(LAST_ERRNO.load(Ordering::Relaxed), Errno::EHOSTDOWN as u32);

        // Nothing for the unspecified or a v4-mapped destination, nor without a notifier.
        NOTIFIED.store(0, Ordering::Relaxed);
        for dst in [IN6ADDR_ANY, a6("::ffff:a00:202")] {
            notify6(
                t,
                &sin6(dst, 0),
                0,
                None,
                0,
                0,
                PRC_MSGSIZE,
                None,
                Some(record),
            );
        }
        notify6(t, &sin6(f, 0), 0, None, 0, 0, PRC_MSGSIZE, None, None);
        assert_eq!(NOTIFIED.load(Ordering::Relaxed), 0);
        crate::sys::systm::net_unlock();

        for i in pcbs {
            release(i);
        }
        teardown();
    }

    /// `route add -inet6 default <gw>` on `ifp`, as icmp6's tests add it.
    fn add_default_route(ifp: &'static crate::net::if_var::Ifnet, gw: In6Addr) {
        use crate::net::route::{
            RTAX_DST, RTAX_GATEWAY, RTAX_NETMASK, RTF_GATEWAY, RTF_STATIC, RTM_ADD, RtAddrinfo,
            rtfree, rtrequest, sa_of, sa_view,
        };
        use crate::netinet6::in6::sin6tosa;

        let mut dst = SockaddrIn6::with_addr(IN6ADDR_ANY);
        let mut mask = SockaddrIn6::with_addr(IN6ADDR_ANY);
        let mut gate = SockaddrIn6::with_addr(gw);
        for s in [&mut dst, &mut mask, &mut gate] {
            s.sin6_len = core::mem::size_of::<SockaddrIn6>() as u8;
        }
        let gate_copy = gate;
        let mut info = RtAddrinfo::new();
        info.rti_info[RTAX_DST] = Some(sa_view(sa_of(&dst)));
        info.rti_info[RTAX_NETMASK] = Some(sa_view(sa_of(&mask)));
        info.rti_info[RTAX_GATEWAY] = Some(sa_view(sa_of(&gate_copy)));
        info.rti_flags = RTF_GATEWAY | RTF_STATIC;
        crate::sys::systm::net_lock();
        // SAFETY: a local `sockaddr_in6`.
        let ifa = unsafe { crate::net::if_::ifaof_ifpforaddr(sin6tosa(&mut gate), ifp) };
        info.rti_ifa = ifa.as_ref();
        let mut rt = None;
        rtrequest(RTM_ADD, &mut info, 0, Some(&mut rt), 0).expect("default route");
        rtfree(rt);
        crate::sys::systm::net_unlock();
    }

    /// B1 of N6.3b2's first review: `in6_pcbnotify` runs in the softnet's context (the shared
    /// net lock, no socket's lock) and reads an unconnected pcb's route cache only under that
    /// pcb's socket lock. A walk that read it under the table mutex alone would trip
    /// `Inpcb::route`'s check (the cache is another socket's), so this test fails then.
    #[test]
    fn in6_pcbnotify_reads_a_route_cache_under_its_socket_lock() {
        use crate::kern::uipc_socket::{soclose, socreate};
        use crate::netinet::in_pcb::tests::inet6::{take_sent6, test_if6};
        use crate::sys::protosw::PRC_REDIRECT_HOST;
        use crate::sys::socket::SOCK_DGRAM;
        use crate::sys::systm::{net_lock_shared, net_unlock_shared};

        let (_g, _t, _p) = setup();
        kernel::netinet::udp_usrreq::udp_init::<crate::kern::proc_glue::BsdProcGlue>();
        let ifp = test_if6(b"tnfy6");
        add_default_route(ifp, a6("fd00:77::fe"));

        // An unconnected UDP socket whose cache holds the default route towards `far`, filled
        // as udp6_output fills it: under the socket lock.
        let so = socreate(i32::from(AF_INET6), SOCK_DGRAM, 0).expect("socket");
        let inp = sotoinpcb(so).expect("attached");
        let far = a6("2001:db8::5");
        assert!(!inp.route_locked(), "no lock held: no loan of the cache");
        solock(so);
        assert!(inp.route_locked());
        let rt = route6_mpath(&mut inp.route(), &far, None, 0).expect("the default route");
        assert_eq!(rt.rt_flags.get() & RTF_HOST, 0, "a network route");
        sounlock(so);

        // A redirect about another destination keeps the cache; one about `far` reaches the
        // pcb by its cached route alone (no address of the pcb matches) and flushes it.
        for (dst, kept) in [(a6("2001:db8::6"), true), (far, false)] {
            net_lock_shared();
            notify6(
                inp.table(),
                &sin6(dst, 0),
                0,
                None,
                0,
                0,
                PRC_REDIRECT_HOST,
                None,
                Some(record),
            );
            net_unlock_shared();
            solock(so);
            assert_eq!(inp.route().rt().is_some(), kept, "redirect about {dst:?}");
            sounlock(so);
        }

        soclose(so, 0).expect("close");
        let _ = take_sent6();
        teardown();
    }

    /// The kernel's `in6_pcbaddrisavail(inp, sin6, 0)` under the net lock, as the socket
    /// layer's `solock` calls the pcb functions (their address checks read the interfaces'
    /// addresses under it).
    fn avail(inp: &Inpcb, sin6: &mut SockaddrIn6) -> Result<(), Errno> {
        crate::sys::systm::net_lock();
        let r = k6::in6_pcbaddrisavail(
            inp,
            sin6,
            0,
            crate::net::if_::net_held("in6_pcbaddrisavail").net(),
        );
        crate::sys::systm::net_unlock();
        r
    }

    #[test]
    fn in6_pcbaddrisavail_checks_the_address() {
        let (_g, _t, _p) = setup();
        let t = table(1);
        let inp = pcb6(t);

        // The wildcard without a port is always available.
        let mut s = sin6(IN6ADDR_ANY, 0);
        assert_eq!(avail(inp, &mut s), Ok(()));
        // No IPv4-mapped addresses.
        let mut s = sin6(a6("::ffff:a00:20f"), 0);
        assert_eq!(avail(inp, &mut s), Err(Errno::EADDRNOTAVAIL));
        // Not an address of ours, unless SO_BINDANY.
        let mut s = sin6(a6("2001:db8::1"), 0);
        assert_eq!(avail(inp, &mut s), Err(Errno::EADDRNOTAVAIL));
        inp.socket().set_options_legacy(SO_BINDANY, true);
        let mut s = sin6(a6("2001:db8::1"), 0);
        assert_eq!(avail(inp, &mut s), Ok(()));
        // A multicast group is available too, the scope id cleared.
        let mut s = sin6(a6("ff02::1"), 0);
        s.sin6_scope_id = 0;
        assert_eq!(avail(inp, &mut s), Ok(()));
        assert_eq!(s.sin6_scope_id, 0);
        release(inp);
        teardown();
    }
}
/* </TESTS> */
