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
 *	@(#)uipc_domain.c	8.2 (Berkeley) 10/18/93
 */

/*
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
 *	@(#)domain.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The communication domains: `struct domain` (`<sys/domain.h>`) as a trait, the table of
//! the configured ones, `domaininit`, the protocol lookups and the protocol timeouts
//! (`kern/uipc_domain.c`).
//!
//! Upstream: sys/kern/uipc_domain.c @ 3ce1f3f79392
//! Upstream: sys/sys/domain.h @ 3ce1f3f79392
//! LZ: sys/kern/uipc_domain.rs@3c62ede68802
//! LZ: sys/sys/domain.rs@3c62ede68802
//!
//! A domain is an address family (`AF_INET`, `AF_ROUTE`, ...) with its table of protocols
//! and the routing table's view of its addresses (`dom_rtoffset`, `dom_maxplen`).
//! `domaininit` runs every domain's `dom_init` and every protocol's `pr_init`, sizes the link
//! and protocol headers the mbuf code reserves (`max_linkhdr`, `max_hdr`) and starts the
//! protocols' fast (200 ms) and slow (500 ms) timeouts.
//!
//! ## Deviations
//! - `domains[]` is the slice the glue hands [`domaininit`], in the C's order (`pfkey`,
//!   `inet6`, `inet`, `unix`, `route`; `MPLS` and `NAF_FRAME` not configured), without the
//!   C's NULL terminator.
//! - `net_sysctl` (`net.*`) is `bsd`'s until the socket layer's switch moves the net node
//!   (the protocols' `pr_sysctl` are `bsd`'s tables until each moves).
//!
//! ## Redesign
//! - LZ's `struct domain` was a struct of `Option<fn>` and a `&'static [Protosw]`; here
//!   [`Domain`] is a trait whose NULL `dom_externalize`/`dom_dispose` are default methods
//!   answering as the C's callers do when the entry is NULL, and whose protocols are
//!   `&'static dyn Protocol` (decision 46, docs/ZERO_UNSAFE.md 0.11).
//! - `domains[]`, the socket pools and the protocol timeouts live in [`NetTables`], one per
//!   glue (`FileGlue::net()`, D-N6.2-7): the socket is generic over the glue, so its pools
//!   cannot be `static`s. The timeouts are made in the tables' `const fn` with the glue's
//!   function and armed through `pin_static` of the glue's `'static` tables, so no
//!   argument word is followed.

use core::ptr;
use core::sync::atomic::Ordering;

use hal::intr::IPL_SOFTNET;
use hal::once::OnceStatic;
use hal::pin::pin_static;
use hal::pool::{BoxPool, TypedPool};
use hal::sys::errno::Errno;
use hal::sys::socket::{PF_UNSPEC, SOCK_RAW};

use crate::assert::kassert;
use crate::net::mbuf::{MAX_HDR, MAX_LINKHDR, MAX_PROTOHDR, Mbuf, MbufBox};
use crate::net::proto::{Protocol, SockaddrView};
use crate::net::socket::{Socket, Sosplice};
use crate::proc::ProcGlue;
use crate::timeout::{KCLOCK_NONE, TIMEOUT_MPSAFE, TIMEOUT_PROC, Timeout};

/// `struct domain`.
pub trait Domain<G: ProcGlue>: Sync + 'static {
    /// `dom_family`: `AF_xxx`.
    fn family(&self) -> i32;
    /// `dom_name`.
    fn name(&self) -> &'static str;
    /// `dom_init`: initialize domain data structures.
    fn init(&self) {}
    /// `dom_externalize` and `dom_dispose`, the domain's access rights in control messages:
    /// both or neither, `None` for a domain without them (the C's NULL entries: `soreceive`
    /// then hands the message over as it is, lets no mutex go around them, and disposes of
    /// nothing).
    fn rights(&self) -> Option<&'static dyn DomRights> {
        None
    }
    /// `dom_protosw` .. `dom_protoswNPROTOSW`: the protocols.
    fn protocols(&self) -> &'static [&'static dyn Protocol<G>];
    /// `SO_PEERCRED`'s answer for a socket of this domain, the socket locked
    /// (`sotounpcb(so)->unp_connid` when `UNP_FEIDS` is set); `None` (`ENOTCONN`) for a
    /// domain without peer credentials.
    fn peercred(
        &self,
        _so: &Socket<G>,
        _w: &crate::net::socket::SoLocked<'_>,
    ) -> Option<hal::sys::socket::Sockpeercred> {
        None
    }
    /// Whether the domain's protocols are `bsd`'s (its `struct domain`'s impl): their requests
    /// release and take the socket lock again C-style (`route_attach`, `route_output`,
    /// `pfkeyv2_send`), so the kernel holds a socket of the domain by a detached holding of
    /// `so_lock`, never a witness.
    fn legacy(&self) -> bool {
        false
    }
    /// `dom_sasize`: size of sockaddr structure.
    fn sasize(&self) -> u32 {
        0
    }
    /// `dom_rtoffset`: offset of the key, in bytes (0: no routing table).
    fn rtoffset(&self) -> u32 {
        0
    }
    /// `dom_maxplen`: maximum prefix length, in bits.
    fn maxplen(&self) -> u32 {
        0
    }
}

/// The network's tables of one glue: the configured domains, the socket pools and the
/// protocol timeouts (`uipc_domain.c`'s and `uipc_socket.c`'s globals).
pub struct NetTables<G: ProcGlue> {
    /// `domains[]`, set once by [`domaininit`].
    domains: OnceStatic<&'static [&'static dyn Domain<G>]>,
    /// `socket_pool`.
    pub(crate) sockpl: TypedPool<Socket<G>>,
    /// `sosplice_pool`.
    pub(crate) sosppl: BoxPool<Sosplice<G>>,
    /// `pffast_timeout`: the 200 ms protocol timeout.
    pffast: Timeout,
    /// `pfslow_timeout`: the 500 ms protocol timeout.
    pfslow: Timeout,
    /// The registered `so_upcall` function (nfs's, `bsd`'s until N6.7b).
    pub(crate) upcall: OnceStatic<crate::net::socket::UpcallFn<G>>,
    /// The UNIX domain's globals (`uipc_usrreq.c`'s: the control blocks' pool, `unp_head`,
    /// the files in flight, the collector).
    pub(crate) unix: crate::net::unix::UnixTables<G>,
    /// `if_input_task_locked`: `if_netisr`, the protocols' software interrupt (N6.3a: here,
    /// since its yield is the glue's scheduler's).
    pub(crate) if_input_task_locked: crate::task::Task,
}

impl<G: ProcGlue> NetTables<G> {
    /// The empty tables, before [`domaininit`].
    pub const fn new() -> Self {
        Self {
            domains: OnceStatic::new(),
            sockpl: TypedPool::new(),
            sosppl: BoxPool::new(),
            pffast: Timeout::new_flags(
                pffasttimo::<G>,
                ptr::null_mut(),
                KCLOCK_NONE,
                TIMEOUT_PROC | TIMEOUT_MPSAFE,
            ),
            pfslow: Timeout::new_flags(
                pfslowtimo::<G>,
                ptr::null_mut(),
                KCLOCK_NONE,
                TIMEOUT_PROC | TIMEOUT_MPSAFE,
            ),
            upcall: OnceStatic::new(),
            unix: crate::net::unix::UnixTables::new(),
            if_input_task_locked: crate::task::Task::new(
                crate::net::netisr::if_netisr::<G>,
                ptr::null_mut(),
            ),
        }
    }

    /// `domains[]`: the configured domains (none before [`domaininit`]).
    pub fn domains(&self) -> &'static [&'static dyn Domain<G>] {
        self.domains.get().copied().unwrap_or(&[])
    }
}

impl<G: ProcGlue> Default for NetTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The glue's network tables.
#[inline]
pub(crate) fn net<G: ProcGlue>() -> &'static NetTables<G> {
    <G::FileGlue as crate::file::FileGlue>::net()
}

/// `soinit`: the socket and splice pools (`sockpl`, `sosppl`, `IPL_SOFTNET`). A second call
/// (a test glue's) leaves them as they are.
pub fn soinit<G: ProcGlue>() {
    let t = net::<G>();
    let _ = t.sockpl.init_recorded(IPL_SOFTNET, 0, "sockpl");
    let _ = t.sosppl.init(IPL_SOFTNET, 0, "sosppl");
}

/// `domaininit`: records the configured domains and initialises them and their protocols.
pub fn domaininit<G: ProcGlue>(domains: &'static [&'static dyn Domain<G>]) {
    let t = net::<G>();
    if t.domains.set(domains).is_err() {
        return;
    }
    for dp in domains {
        dp.init();
        for pr in dp.protocols() {
            pr.init();
        }
    }

    // max_linkhdr of 64 was chosen to encompass tunnelling traffic in IP payloads, eg, by
    // etherip(4) or gif(4), without needing to prepend an mbuf to fit those headers.
    if MAX_LINKHDR.load(Ordering::Relaxed) < 64 {
        MAX_LINKHDR.store(64, Ordering::Relaxed);
    }

    MAX_HDR.store(
        MAX_LINKHDR.load(Ordering::Relaxed) + MAX_PROTOHDR.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    // The kernel's own host tests share one process-wide timeout wheel, which other tests
    // tick (event.rs's timers) and whose counters others compare (timeout.rs): the protocol
    // timeouts, which re-add themselves, are not armed there (testing.md's isolation rule;
    // no test of the kernel's domains has work for them).
    #[cfg(not(test))]
    {
        pin_static(&t.pffast).add(1);
        pin_static(&t.pfslow).add(1);
    }
}

/// The domain table set without `domaininit`'s initialisations, the first call for the
/// process (`bsd`'s protocol tests, whose setups run each protocol's init over each test's
/// fresh memory themselves).
pub fn domains_set<G: ProcGlue>(domains: &'static [&'static dyn Domain<G>]) {
    let _ = net::<G>().domains.set(domains);
}

/// `pffinddomain(family)`: the domain of address family `family`.
pub fn pffinddomain<G: ProcGlue>(family: i32) -> Option<&'static dyn Domain<G>> {
    net::<G>()
        .domains()
        .iter()
        .copied()
        .find(|dp| dp.family() == family)
}

/// `pffindtype(family, type)`: the first protocol of `family` with socket type `type_`.
pub fn pffindtype<G: ProcGlue>(family: i32, type_: i32) -> Option<&'static dyn Protocol<G>> {
    let dp = pffinddomain::<G>(family)?;

    dp.protocols()
        .iter()
        .copied()
        .find(|pr| pr.ty() != 0 && i32::from(pr.ty()) == type_)
}

/// `pffindproto(family, protocol, type)`: the protocol `protocol` of `family` with socket
/// type `type_`; for raw sockets, the domain's raw wildcard when no protocol matches.
pub fn pffindproto<G: ProcGlue>(
    family: i32,
    protocol: i32,
    type_: i32,
) -> Option<&'static dyn Protocol<G>> {
    let mut maybe = None;

    if family == i32::from(PF_UNSPEC) {
        return None;
    }

    let dp = pffinddomain::<G>(family)?;

    for &pr in dp.protocols() {
        if i32::from(pr.protocol()) == protocol && i32::from(pr.ty()) == type_ {
            return Some(pr);
        }

        if type_ == SOCK_RAW
            && i32::from(pr.ty()) == SOCK_RAW
            && pr.protocol() == 0
            && maybe.is_none()
        {
            maybe = Some(pr);
        }
    }
    maybe
}

/// `pfctlinput(cmd, sa)`: hands control input `cmd` about `sa` to every protocol that takes
/// it (`pr_ctlinput(cmd, sa, 0, NULL)`), the net lock held.
pub fn pfctlinput<G: ProcGlue>(cmd: i32, sa: SockaddrView<'_>) {
    kassert!(hal::netlock::status() != 0);

    for dp in net::<G>().domains() {
        for pr in dp.protocols() {
            pr.ctlinput(cmd, sa, 0);
        }
    }
}

/// `pfslowtimo`: the 500 ms protocol timeout.
fn pfslowtimo<G: ProcGlue>(_arg: *mut core::ffi::c_void) {
    let t = net::<G>();
    for dp in t.domains() {
        for pr in dp.protocols() {
            pr.slowtimo();
        }
    }
    pin_static(&t.pfslow).add_msec(500);
}

/// `pffasttimo`: the 200 ms protocol timeout.
fn pffasttimo<G: ProcGlue>(_arg: *mut core::ffi::c_void) {
    let t = net::<G>();
    for dp in t.domains() {
        for pr in dp.protocols() {
            pr.fasttimo();
        }
    }
    pin_static(&t.pffast).add_msec(200);
}

/// A domain's access rights in control messages ([`Domain::rights`]).
pub trait DomRights: Sync {
    /// `dom_externalize(m, controllen, flags)`: externalize access rights of the control
    /// message `m` for the receiving process.
    fn externalize(&self, m: &mut Mbuf, controllen: u32, flags: i32) -> Result<(), Errno>;
    /// `dom_dispose(m)`: dispose of internalized rights in the records `m` (records by
    /// `m_nextpkt`, each a chain by `m_next`).
    fn dispose(&self, m: &mut Mbuf);
}

/// `dom_dispose` of a domain, then the records freed (`sorele`'s and `sorflush`'s
/// `if (pr->pr_flags & PR_RIGHTS && dom_dispose) (*dom_dispose)(m); m_purge(m)`).
pub(crate) fn dispose_and_purge<G: ProcGlue>(pr: &dyn Protocol<G>, m: Option<MbufBox>) {
    let Some(mut m) = m else {
        return;
    };
    if pr.flags() & crate::net::proto::PR_RIGHTS != 0
        && let Some(rights) = pr.domain().rights()
    {
        rights.dispose(&mut m);
    }
    crate::net::mbuf::m_purge(Some(m));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::{assert, assert_eq};

    use super::*;
    use crate::net::proto::Usrreqs;

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
    static DOMS: [&dyn Domain<Procs>; 1] = [&TestDom];

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

    /// The protocol's address, to compare the trait objects found.
    fn addr(p: &dyn Protocol<Procs>) -> *const () {
        core::ptr::from_ref(p).cast()
    }

    #[test]
    fn the_finders_walk_the_domain_table_as_the_c_does() {
        domains_set::<Procs>(&DOMS);
        // A second table set is refused: the first stays (domaininit's once).
        domains_set::<Procs>(&[]);
        assert_eq!(
            pffinddomain::<Procs>(FAMILY).map(|d| d.name()),
            Some("testdom")
        );
        assert!(pffinddomain::<Procs>(FAMILY + 1).is_none());

        // pffindtype: the first protocol of the type.
        let st = pffindtype::<Procs>(FAMILY, 1).expect("the stream protocol");
        assert_eq!(addr(st), addr(&STREAM));
        assert!(pffindtype::<Procs>(FAMILY, 5).is_none());

        // pffindproto: protocol and type; for SOCK_RAW the domain's raw wildcard.
        let tcp = pffindproto::<Procs>(FAMILY, 6, 1).expect("protocol 6");
        assert_eq!(addr(tcp), addr(&STREAM));
        let raw = pffindproto::<Procs>(FAMILY, 17, 3).expect("the raw wildcard");
        assert_eq!(addr(raw), addr(&RAW));
        assert!(pffindproto::<Procs>(FAMILY, 17, 2).is_none());
        assert!(pffindproto::<Procs>(i32::from(PF_UNSPEC), 6, 1).is_none());

        // A domain without the optional entries: the C's NULLs.
        assert!(TestDom.rights().is_none() && !TestDom.legacy());
    }
}
/* </TESTS> */
