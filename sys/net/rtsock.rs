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
 * Copyright (c) 1988, 1991, 1993
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
 *	@(#)rtsock.c	8.6 (Berkeley) 2/11/95
 */
/* </LICENSES> */

/* <CODE> */
//! The routing socket for `bsd`'s callers: the route domain, its messages and the `net.route`
//! sysctl are the kernel's (`kernel::net::rtsock`, N6.3b); this module is the adapter, of no
//! logic, of the announcements `bsd`'s protocols make (the net lock's proof made through the
//! `SaToken`, as `sys/net/route.rs` does), and the routing socket's end-to-end tests over
//! `bsd`'s protocols.
//!
//! Upstream: sys/net/rtsock.c @ 3ce1f3f79392

pub use kernel::net::rtsock::{ROUTERCVQ, ROUTESNDQ, rtm_ifannounce, rtm_ifchg, rtm_miss};

use crate::net::if_::net_held;
use crate::net::if_var::Ifaddr;
use crate::net::route::Rtentry;

/// `rtm_send(rt, cmd, error, rtableid)`.
pub fn rtm_send(rt: &Rtentry, cmd: u8, error: i32, rtableid: u32) {
    kernel::net::rtsock::rtm_send(rt, cmd, error, rtableid, &net_held("rtm_send"));
}

/// `rtm_addr(cmd, ifa)`.
pub fn rtm_addr(cmd: u8, ifa: &Ifaddr) {
    kernel::net::rtsock::rtm_addr(cmd, ifa, &net_held("rtm_addr"));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // A routing socket end to end over `bsd`'s socket layer and protocols (`socreate`,
    // `sosend` of `RTM_GET`, `RTM_ADD` and `RTM_DELETE`, the answers read back with
    // `soreceive`, the socket options through `sosetopt`/`sogetopt`, `soclose`): LZ's test of
    // the module, kept here because it needs `bsd`'s IPv4 and Ethernet; with N6.3b2's review
    // (S2) the kernel's `route_output` and `route_input` through it: `RTM_CHANGE`, the
    // superuser check (`EACCES`), `EMSGSIZE`, the listeners' filters (`ROUTE_PRIOFILTER`,
    // `ROUTE_TABLEFILTER`, `ROUTE_FLAGFILTER`), `SO_USELOOPBACK`, and a full listener
    // desynchronised then flushed. The message layout, the address checks and the
    // `net.route` sysctl are the kernel's tests (`kernel::net::rtsock`).
    //
    // The test runs as a thread with root credentials made `curproc` (the requests check
    // `suser` and stamp `rtm_pid`); it clears `curproc` before it returns.

    use core::mem::{offset_of, size_of};
    use core::sync::atomic::Ordering;
    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use crate::kern::kern_prot::crget;
    use crate::kern::proc_glue::{BsdProcGlue, procinit};
    use crate::kern::uipc_mbuf::{m_freem, m_get};
    use crate::kern::uipc_socket::{
        soclose, socreate, sogetopt, soinit, soreceive, sosend, sosetopt,
    };
    use crate::machine::Machine;
    use crate::machine::cpu::Cpu;
    use crate::net::route::{
        ROUTE_FLAGFILTER, ROUTE_MSGFILTER, ROUTE_PRIOFILTER, ROUTE_TABLEFILTER, RTA_DST,
        RTA_GATEWAY, RTA_LABEL, RTA_NETMASK, RTABLE_ANY, RTF_DONE, RTF_GATEWAY, RTF_HOST,
        RTF_LOCAL, RTF_STATIC, RTF_UP, RTM_ADD, RTM_CHANGE, RTM_DELETE, RTM_DESYNC, RTM_GET,
        RTM_IFINFO, RTM_MAXSIZE, RTM_VERSION, RTP_CONNECTED, RtMsghdr,
    };
    use crate::netinet::in_::SockaddrIn;
    use crate::netinet::ip_input::tests::{configure, sin, test_ether, unconfigure};
    use crate::sys::errno::Errno;
    use crate::sys::mbuf::{M_DONTWAIT, MT_SOOPTS, Mbuf, mtod};
    use crate::sys::socket::{
        AF_ROUTE, MSG_DONTWAIT, PF_ROUTE, SO_USELOOPBACK, SOCK_RAW, SOL_SOCKET,
    };
    use crate::sys::socketvar::{SS_ISCONNECTED, Socket};
    use crate::sys::systm::{net_lock, net_unlock};
    use crate::sys::uio::{Iovec, Uio, UioRw, UioSeg};

    type Guards = (MutexGuard<'static, ()>, MutexGuard<'static, ()>);

    /// The pid the test thread's process has.
    const PID: i32 = 42;

    /// The network setup (memory, mbufs, a routing table, IPv4) with an empty interface list,
    /// the socket and routing control block pools, and a root thread as `curproc`.
    fn setup() -> Guards {
        let g = crate::netinet::ip_input::tests::setup();
        // Interfaces and interface groups of earlier tests live in the memory just reset.
        kernel::net::if_::ifnet_test_reset();
        procinit();
        soinit();
        crate::kern::uipc_domain::net_tables_for_tests();
        kernel::net::rtsock::route_prinit::<BsdProcGlue>();
        kernel::net::rtsock::RTP_COUNT.store(0, Ordering::Relaxed);

        let p = crate::kern::proc_glue::test_proc(PID, crget());
        Machine::set_curproc(Machine::curcpu(), Some(p));
        g
    }

    /// Undoes what outlives the reset memory: `curproc`.
    fn teardown() {
        Machine::set_curproc(Machine::curcpu(), None);
    }

    /// The `struct rt_msghdr` at the start of `buf`; zeros past the end of a shorter message.
    fn hdr_at(buf: &[u8]) -> RtMsghdr {
        let mut h = [0u8; size_of::<RtMsghdr>()];
        let n = buf.len().min(h.len());
        h[..n].copy_from_slice(&buf[..n]);
        hal::pod::at::<RtMsghdr>(&h, 0).unwrap_or_default()
    }

    /// The IPv4 address of the `sockaddr_in` at `off` in `buf`.
    fn in_at(buf: &[u8], off: usize) -> [u8; 4] {
        let a = off + offset_of!(SockaddrIn, sin_addr);
        [buf[a], buf[a + 1], buf[a + 2], buf[a + 3]]
    }

    /// `sosend` of `bytes` from kernel space.
    fn send(so: &'static Socket, bytes: &[u8]) -> Result<(), Errno> {
        let mut iov = [Iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: bytes.len(),
        }];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: bytes.len(),
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: None,
        };
        sosend(so, None, Some(&mut uio), None, None, 0)
    }

    /// One message read without waiting, `EWOULDBLOCK` when there is none.
    fn recv(so: &'static Socket) -> Result<Vec<u8>, Errno> {
        let mut buf = vec![0u8; 1024];
        let len = buf.len();
        let mut iov = [Iovec {
            iov_base: buf.as_mut_ptr().cast(),
            iov_len: len,
        }];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_resid: len,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        };
        let mut flags = MSG_DONTWAIT;
        soreceive(so, None, &mut uio, None, None, Some(&mut flags), 0)?;
        let n = len - uio.uio_resid;
        buf.truncate(n);
        Ok(buf)
    }

    /// A request: a `struct rt_msghdr` of `type_` and `seq` with `addrs` after it.
    fn request(type_: u8, seq: i32, flags: u32, addrs: &[(i32, SockaddrIn)]) -> Vec<u8> {
        let mut rtm = RtMsghdr {
            rtm_version: RTM_VERSION,
            rtm_type: type_,
            rtm_hdrlen: size_of::<RtMsghdr>() as u16,
            rtm_seq: seq,
            rtm_flags: flags as i32,
            ..RtMsghdr::default()
        };
        let mut v = vec![0u8; size_of::<RtMsghdr>()];
        for (bit, sa) in addrs {
            rtm.rtm_addrs |= bit;
            v.extend_from_slice(crate::net::route::sa_of(sa));
        }
        rtm.rtm_msglen = v.len() as u16;
        assert!(hal::pod::put(&mut v, 0, &rtm));
        v
    }

    /// The answer to request `seq`, skipping the kernel's own announcements.
    fn answer(so: &'static Socket, seq: i32) -> (Vec<u8>, RtMsghdr) {
        loop {
            let m = recv(so).expect("an answer");
            let h = hdr_at(&m);
            if h.rtm_seq == seq && h.rtm_pid == PID {
                return (m, h);
            }
        }
    }

    /// An option mbuf holding `v`.
    fn opt(v: u32) -> &'static Mbuf {
        let m = m_get(M_DONTWAIT, MT_SOOPTS).expect("mbuf");
        m.m_len().set(4);
        // SAFETY: a fresh mbuf holds MLEN bytes.
        unsafe { mtod::<u32>(m).write_unaligned(v) };
        m
    }

    /// `setsockopt(so, level, optname, v)`, the option mbuf freed after (the caller's, as
    /// `sys_setsockopt`'s).
    fn setopt(so: &'static Socket, level: i32, optname: i32, v: Option<u32>) -> Result<(), Errno> {
        let m = v.map(opt);
        let e = sosetopt(so, level, optname, m);
        if let Some(m) = m {
            m_freem(m);
        }
        e
    }

    #[test]
    fn a_routing_socket_gets_adds_and_deletes_routes() {
        let _g = setup();
        let ifp = test_ether();
        configure(ifp, [10, 0, 2, 15], [255, 255, 255, 0]);

        let so = socreate(i32::from(PF_ROUTE), SOCK_RAW, 0).expect("socket(AF_ROUTE)");
        assert_eq!(kernel::net::rtsock::RTP_COUNT.load(Ordering::Relaxed), 1);
        assert!(so.has_state(SS_ISCONNECTED) && so.has_options(SO_USELOOPBACK));

        // RTM_GET of an address of the subnet: the /24, with its netmask.
        send(
            so,
            &request(RTM_GET, 1, 0, &[(RTA_DST, sin([10, 0, 2, 77]))]),
        )
        .expect("RTM_GET");
        let (m, h) = answer(so, 1);
        assert_eq!(h.rtm_type, RTM_GET);
        assert_eq!(h.rtm_errno, 0);
        assert_ne!(h.rtm_flags as u32 & RTF_DONE, 0);
        assert_eq!(u32::from(h.rtm_index), ifp.if_index.get());
        assert_ne!(h.rtm_addrs & RTA_NETMASK, 0);
        assert_eq!(h.rtm_addrs & RTA_LABEL, 0);
        assert_eq!(in_at(&m, size_of::<RtMsghdr>()), [10, 0, 2, 0]);

        // RTM_ADD of a default route through 10.0.2.2.
        let default = [
            (RTA_DST, sin([0, 0, 0, 0])),
            (RTA_GATEWAY, sin([10, 0, 2, 2])),
            (RTA_NETMASK, sin([0, 0, 0, 0])),
        ];
        let flags = RTF_UP | RTF_GATEWAY | RTF_STATIC;
        send(so, &request(RTM_ADD, 2, flags, &default)).expect("RTM_ADD");
        let (_, h) = answer(so, 2);
        assert_eq!((h.rtm_type, h.rtm_errno), (RTM_ADD, 0));
        assert_ne!(h.rtm_flags as u32 & RTF_GATEWAY, 0);

        // An address off the subnet now goes through it.
        send(so, &request(RTM_GET, 3, 0, &[(RTA_DST, sin([8, 8, 8, 8]))])).expect("RTM_GET");
        let (m, h) = answer(so, 3);
        assert_eq!(in_at(&m, size_of::<RtMsghdr>()), [0, 0, 0, 0]);
        assert_eq!(
            in_at(&m, size_of::<RtMsghdr>() + 16),
            [10, 0, 2, 2],
            "gateway"
        );
        assert_eq!(h.rtm_addrs & RTA_GATEWAY, RTA_GATEWAY);

        // RTM_DELETE, then RTM_GET fails with ESRCH, which the answer carries too.
        send(so, &request(RTM_DELETE, 4, 0, &default)).expect("RTM_DELETE");
        let (_, h) = answer(so, 4);
        assert_eq!((h.rtm_type, h.rtm_errno), (RTM_DELETE, 0));
        // As in C, deleting the default route leaves the egress group until the next rebuild;
        // rebuild it now (empty) so no group of this test outlives its memory.
        net_lock();
        crate::net::if_::if_group_egress_build().expect("egress");
        net_unlock();
        assert!(
            kernel::net::if_::ifg_head()
                .refs()
                .iter()
                .all(|g| !g.ifg_group.starts_with(b"egress"))
        );
        let get = request(RTM_GET, 5, 0, &[(RTA_DST, sin([8, 8, 8, 8]))]);
        assert_eq!(send(so, &get), Err(Errno::ESRCH));
        let (_, h) = answer(so, 5);
        assert_eq!(h.rtm_errno, Errno::ESRCH as i32);
        assert_eq!(h.rtm_flags as u32 & RTF_DONE, 0);

        // Malformed requests.
        let mut bad = get.clone();
        bad[offset_of!(RtMsghdr, rtm_version)] = 4;
        assert_eq!(send(so, &bad), Err(Errno::EPROTONOSUPPORT));
        let mut bad = get.clone();
        bad[offset_of!(RtMsghdr, rtm_type)] = RTM_IFINFO;
        assert_eq!(send(so, &bad), Err(Errno::EOPNOTSUPP));
        let mut bad = get.clone();
        bad[0] += 1;
        assert_eq!(
            send(so, &bad),
            Err(Errno::EINVAL),
            "rtm_msglen is not the length"
        );
        let local = request(RTM_ADD, 6, RTF_LOCAL, &default);
        assert_eq!(send(so, &local), Err(Errno::EINVAL), "a kernel-only flag");
        assert_eq!(
            recv(so),
            Err(Errno::EWOULDBLOCK),
            "no answers to malformed requests"
        );

        // The options: a message filter that only lets RTM_ADD through.
        let level = i32::from(AF_ROUTE);
        setopt(so, level, ROUTE_MSGFILTER, Some(1 << RTM_ADD)).expect("ROUTE_MSGFILTER");
        let m = opt(0);
        sogetopt(so, level, ROUTE_MSGFILTER, m).expect("get");
        // SAFETY: the option mbuf holds four bytes.
        assert_eq!(unsafe { mtod::<u32>(m).read_unaligned() }, 1 << RTM_ADD);
        m_freem(m);
        send(
            so,
            &request(RTM_GET, 7, 0, &[(RTA_DST, sin([10, 0, 2, 77]))]),
        )
        .expect("RTM_GET");
        assert_eq!(recv(so), Err(Errno::EWOULDBLOCK), "filtered out");
        assert_eq!(
            setopt(so, level, ROUTE_TABLEFILTER, Some(9)),
            Err(Errno::ENOENT)
        );
        assert_eq!(
            setopt(so, level, ROUTE_PRIOFILTER, Some(64)),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            setopt(so, level, ROUTE_FLAGFILTER, None),
            Err(Errno::EINVAL)
        );
        assert_eq!(setopt(so, level, 99, Some(0)), Err(Errno::ENOPROTOOPT));
        assert_eq!(setopt(so, 0, ROUTE_MSGFILTER, None), Err(Errno::EINVAL));

        soclose(so, 0).expect("close");
        assert_eq!(kernel::net::rtsock::RTP_COUNT.load(Ordering::Relaxed), 0);

        unconfigure(ifp, [10, 0, 2, 15]);
        teardown();
    }

    /// Every message `so` holds, read without waiting: (header, bytes) of each.
    fn drain(so: &'static Socket) -> Vec<(RtMsghdr, Vec<u8>)> {
        let mut out = Vec::new();
        loop {
            match recv(so) {
                Ok(m) => out.push((hdr_at(&m), m)),
                Err(Errno::EWOULDBLOCK) => return out,
                Err(e) => panic!("soreceive: {e:?}"),
            }
        }
    }

    /// Whether `so` holds the answer to request `seq` of the test's process (the kernel's
    /// own announcements skipped); everything it holds is read.
    fn got(so: &'static Socket, seq: i32) -> bool {
        drain(so)
            .iter()
            .any(|(h, _)| h.rtm_seq == seq && h.rtm_pid == PID)
    }

    /// Request `v` made for routing table `t` (`route -T<t>`).
    fn in_table(t: u16, mut v: Vec<u8>) -> Vec<u8> {
        let off = offset_of!(RtMsghdr, rtm_tableid);
        v[off..off + 2].copy_from_slice(&t.to_ne_bytes());
        v
    }

    /// `route -n get <dst>` as request `seq`.
    fn get(dst: [u8; 4], seq: i32) -> Vec<u8> {
        request(RTM_GET, seq, 0, &[(RTA_DST, sin(dst))])
    }

    #[test]
    fn a_routing_socket_changes_routes_and_filters_its_listeners() {
        let _g = setup();
        let ifp = test_ether();
        configure(ifp, [10, 0, 2, 15], [255, 255, 255, 0]);
        let level = i32::from(AF_ROUTE);

        // A writer that does not read its own messages, and a listener.
        let a = socreate(i32::from(PF_ROUTE), SOCK_RAW, 0).expect("socket(AF_ROUTE)");
        let b = socreate(i32::from(PF_ROUTE), SOCK_RAW, 0).expect("socket(AF_ROUTE)");
        assert_eq!(kernel::net::rtsock::RTP_COUNT.load(Ordering::Relaxed), 2);
        setopt(a, SOL_SOCKET, SO_USELOOPBACK, Some(0)).expect("SO_USELOOPBACK");
        assert!(!a.has_options(SO_USELOOPBACK));

        // RTM_ADD of a default route through 10.0.2.2: the answer reaches the listener only.
        let default = [
            (RTA_DST, sin([0, 0, 0, 0])),
            (RTA_GATEWAY, sin([10, 0, 2, 2])),
            (RTA_NETMASK, sin([0, 0, 0, 0])),
        ];
        let flags = RTF_UP | RTF_GATEWAY | RTF_STATIC;
        send(a, &request(RTM_ADD, 1, flags, &default)).expect("RTM_ADD");
        let (_, h) = answer(b, 1);
        assert_eq!((h.rtm_type, h.rtm_errno), (RTM_ADD, 0));
        assert!(
            !got(a, 1),
            "SO_USELOOPBACK cleared: no answer to the writer"
        );

        // RTM_CHANGE of its gateway to 10.0.2.3 (the route found without the new gateway,
        // the gateway resolved before the change), then RTM_GET shows it.
        let changed = [
            (RTA_DST, sin([0, 0, 0, 0])),
            (RTA_GATEWAY, sin([10, 0, 2, 3])),
            (RTA_NETMASK, sin([0, 0, 0, 0])),
        ];
        send(a, &request(RTM_CHANGE, 2, 0, &changed)).expect("RTM_CHANGE");
        let (m, h) = answer(b, 2);
        assert_eq!((h.rtm_type, h.rtm_errno), (RTM_CHANGE, 0));
        assert_ne!(h.rtm_flags as u32 & RTF_DONE, 0);
        assert_eq!(
            in_at(&m, size_of::<RtMsghdr>() + 16),
            [10, 0, 2, 3],
            "the new gateway"
        );
        send(a, &get([8, 8, 8, 8], 3)).expect("RTM_GET");
        let (m, _) = answer(b, 3);
        assert_eq!(in_at(&m, size_of::<RtMsghdr>() + 16), [10, 0, 2, 3]);
        // A route that does not exist: ESRCH, which its answer carries too.
        let none = [
            (RTA_DST, sin([192, 168, 7, 0])),
            (RTA_GATEWAY, sin([10, 0, 2, 3])),
            (RTA_NETMASK, sin([255, 255, 255, 0])),
        ];
        assert_eq!(
            send(a, &request(RTM_CHANGE, 4, 0, &none)),
            Err(Errno::ESRCH)
        );
        assert_eq!(answer(b, 4).1.rtm_errno, Errno::ESRCH as i32);

        // A message longer than RTM_MAXSIZE: EMSGSIZE, no answer.
        let mut big = get([10, 0, 2, 77], 5);
        big.resize(RTM_MAXSIZE + 8, 0);
        let len = big.len() as u16;
        big[..2].copy_from_slice(&len.to_ne_bytes());
        assert_eq!(send(a, &big), Err(Errno::EMSGSIZE));
        assert!(!got(b, 5));

        // Every request but RTM_GET is the superuser's: EACCES for another user.
        let root = crate::machine::cpu::curproc();
        let mut cr = crate::sys::ucred::Ucred::new();
        cr.cr_uid.set(1000);
        cr.cr_ruid.set(1000);
        let user = crate::kern::proc_glue::test_proc(PID + 1, kernel::cred::CredRef::new(cr));
        Machine::set_curproc(Machine::curcpu(), Some(user));
        assert_eq!(
            send(a, &request(RTM_ADD, 6, flags, &default)),
            Err(Errno::EACCES)
        );
        assert_eq!(
            send(a, &request(RTM_DELETE, 7, 0, &default)),
            Err(Errno::EACCES)
        );
        send(a, &get([8, 8, 8, 8], 8)).expect("RTM_GET is everyone's");
        Machine::set_curproc(Machine::curcpu(), root);
        let seen = drain(b);
        assert!(
            seen.iter()
                .any(|(h, _)| h.rtm_seq == 8 && h.rtm_pid == PID + 1 && h.rtm_errno == 0)
        );
        assert!(!seen.iter().any(|(h, _)| h.rtm_seq == 6 || h.rtm_seq == 7));

        // ROUTE_PRIOFILTER: priorities above the filter's are not wanted. The subnet's route
        // is RTP_CONNECTED, the static default route above it.
        setopt(b, level, ROUTE_PRIOFILTER, Some(u32::from(RTP_CONNECTED))).expect("prio");
        send(a, &get([10, 0, 2, 77], 9)).expect("RTM_GET");
        send(a, &get([8, 8, 8, 8], 10)).expect("RTM_GET");
        let seen = drain(b);
        assert!(seen.iter().any(|(h, _)| h.rtm_seq == 9));
        assert!(
            !seen.iter().any(|(h, _)| h.rtm_seq == 10),
            "filtered by priority"
        );
        setopt(b, level, ROUTE_PRIOFILTER, Some(0)).expect("prio");

        // ROUTE_FLAGFILTER: messages about RTF_GATEWAY routes are not wanted.
        setopt(b, level, ROUTE_FLAGFILTER, Some(RTF_GATEWAY)).expect("flags");
        send(a, &get([10, 0, 2, 77], 11)).expect("RTM_GET");
        send(a, &get([8, 8, 8, 8], 12)).expect("RTM_GET");
        let seen = drain(b);
        assert!(seen.iter().any(|(h, _)| h.rtm_seq == 11));
        assert!(
            !seen.iter().any(|(h, _)| h.rtm_seq == 12),
            "filtered by flags"
        );
        setopt(b, level, ROUTE_FLAGFILTER, Some(0)).expect("flags");

        // ROUTE_TABLEFILTER: table 0's messages only, so an RTM_ADD that makes table 1 (a
        // host route through the same gateway) is not seen; once table 1 exists, filtering
        // on it lets its messages through and table 0's not.
        setopt(b, level, ROUTE_TABLEFILTER, Some(0)).expect("table 0");
        let host = [
            (RTA_DST, sin([192, 168, 9, 9])),
            (RTA_GATEWAY, sin([10, 0, 2, 3])),
        ];
        let add1 = in_table(1, request(RTM_ADD, 13, flags | RTF_HOST, &host));
        send(a, &add1).expect("RTM_ADD in table 1");
        assert!(!got(b, 13), "table 1 filtered out");
        setopt(b, level, ROUTE_TABLEFILTER, Some(1)).expect("table 1");
        send(a, &in_table(1, get([192, 168, 9, 9], 14))).expect("RTM_GET in table 1");
        send(a, &get([8, 8, 8, 8], 15)).expect("RTM_GET in table 0");
        let seen = drain(b);
        assert!(
            seen.iter()
                .any(|(h, _)| h.rtm_seq == 14 && h.rtm_tableid == 1)
        );
        assert!(
            !seen.iter().any(|(h, _)| h.rtm_seq == 15),
            "table 0 filtered out"
        );
        setopt(b, level, ROUTE_TABLEFILTER, Some(RTABLE_ANY)).expect("any table");
        send(a, &in_table(1, request(RTM_DELETE, 16, 0, &host))).expect("RTM_DELETE in table 1");
        assert!(got(b, 16), "every table again");

        send(a, &request(RTM_DELETE, 17, 0, &changed)).expect("RTM_DELETE");
        net_lock();
        crate::net::if_::if_group_egress_build().expect("egress");
        net_unlock();
        soclose(a, 0).expect("close");
        soclose(b, 0).expect("close");
        assert_eq!(kernel::net::rtsock::RTP_COUNT.load(Ordering::Relaxed), 0);
        unconfigure(ifp, [10, 0, 2, 15]);
        teardown();
    }

    #[test]
    fn a_full_listener_is_desynchronised_then_flushed() {
        let _g = setup();
        let ifp = test_ether();
        configure(ifp, [10, 0, 2, 15], [255, 255, 255, 0]);

        let a = socreate(i32::from(PF_ROUTE), SOCK_RAW, 0).expect("socket(AF_ROUTE)");
        let b = socreate(i32::from(PF_ROUTE), SOCK_RAW, 0).expect("socket(AF_ROUTE)");
        setopt(a, SOL_SOCKET, SO_USELOOPBACK, Some(0)).expect("SO_USELOOPBACK");

        // More answers than the listener's ROUTERCVQ bytes hold, none read meanwhile.
        const SENT: i32 = 200;
        for seq in 1..=SENT {
            send(a, &get([10, 0, 2, 77], seq)).expect("RTM_GET");
        }
        // The listener holds the answers that fitted, in order with none missing, then the
        // RTM_DESYNC that told it it lost the next, then nothing: it flushes until drained.
        let seen = drain(b);
        let (last, answers) = seen.split_last().expect("messages");
        assert_eq!(last.0.rtm_type, RTM_DESYNC, "{:?}", last.0);
        assert!(!answers.is_empty() && answers.len() < SENT as usize);
        for (i, (h, _)) in answers.iter().enumerate() {
            assert_eq!((h.rtm_type, h.rtm_seq), (RTM_GET, i as i32 + 1));
        }
        // Drained, it takes messages again (route_rcvd cleared the flush).
        send(a, &get([10, 0, 2, 77], SENT + 1)).expect("RTM_GET");
        assert!(got(b, SENT + 1));

        soclose(a, 0).expect("close");
        soclose(b, 0).expect("close");
        unconfigure(ifp, [10, 0, 2, 15]);
        teardown();
    }
}
/* </TESTS> */
