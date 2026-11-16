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
 * Copyright (c) 1985, 1986, 1993
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
 *	@(#)in_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Internet interface addresses and multicast records, the kernel's half of
//! `<netinet/in_var.h>`: `struct in_ifaddr`, `struct in_multi` and their casts from the
//! generic records. The userland half (`struct in_aliasreq`) is `hal::sys::netinet_in_var`;
//! the prototypes are `kernel::netinet::in_`'s (`in.c`).
//!
//! Upstream: sys/netinet/in_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/in_var.rs@3c62ede68802
//!
//! One `struct in_ifaddr` is allocated for each interface with an Internet address. The
//! `struct ifaddr` it embeds first holds the protocol-independent part; the record holds the
//! `sockaddr_in`s the C's `ifa_addr`, `ifa_dstaddr` and `ifa_netmask` point at, and the
//! generic part's list links. Locks: \[I\] immutable after creation, \[N\] the net lock,
//! \[T\] `if_tmplist_lock`, \[m\] the parent interface's `if_maddrlock`; the other members
//! change under the net lock, as in the C.
//!
//! ## Deviations
//! - The `ia_list` TAILQ entry (the C's list of every Internet address) is dropped: nothing
//!   links the records through it at the pin. The record's `ia_list` member is the generic
//!   part's `ifa_list`.
//!
//! ## Redesign
//! - LZ's `#[repr(C)]` records of `Cell`s cast from their generic part (`ifatoia`,
//!   `ifmatoinm`: pointer casts checked by the address family, one `unsafe` each) -> an
//!   address is held by a counted handle whose arm is its family
//!   (`kernel::net::if_var::IfaddrRef::In`, N6.5e): [`ifatoia`] and [`InIfaddr::of`] match
//!   the arm (`panic(9)` and `None` for another family's). A multicast record likewise
//!   (`kernel::net::if_var::IfmaddrRef::In`, N6.5e's 5e2): [`ifmatoinm`] and [`InMulti::of`].
//! - A multicast record is one `malloc(9)` block of `M_IPMADDR` (`hal::malloc::MArc`, N6.5e's
//!   5e2), made by [`in_multi_alloc`] and freed with its last handle (`in_delmulti`'s
//!   `free(inm, M_IPMADDR, ..)`): the interface's list holds one, each holder of a membership
//!   one (`ia_allhosts`, a socket's options, pfsync's), and `inm_refcnt` counts the
//!   memberships as the C's (D-8). N6.4a's free list (and with it the high-water mark of
//!   `vmstat -m`'s `in_multi` row) and the generic part's back-link to the record are gone;
//!   the record's list cell is keyed to its interface's `if_maddrlock`, so it is made for one
//!   interface ([`in_multi_alloc`] takes it).
//! - An address is one `malloc(9)` block of `M_IFADDR` (`hal::malloc::MArc`, N6.5e): the
//!   last handle's drop is `ifafree`'s `free(ifa, M_IFADDR, 0)`, so `vmstat -m`'s `ifaddr`
//!   row falls with the live addresses as the C's; LZ's (and N6.4a's) free list, its mutex and
//!   `in_ifafree` are gone. The generic part's `ifa_list` and `ifa_tmplist` are the record's
//!   `ia_list` (a net cell, `hal::rwlock::Guarded::new_net`) and `ia_tmplist` (keyed to
//!   `if_tmplist_lock`): a `Guarded` is not valid all zero, which enc(4)'s generic part, in a
//!   zeroed softc, must be.
//! - The C's `struct sockaddr_in` members -> [`SinWords`], the address in atomic words, read
//!   and written whole (`get`, `set`: LZ's `Cell` interface, so `bsd`'s readers are
//!   unchanged), by the address alone (`addr`, one word load) or as the socket address the
//!   generic part's `ifa_addr` names (`read`, through the handle:
//!   `kernel::net::if_var::IfaddrRef::ifa_addr`). No link from the generic part to them (N6.5e;
//!   N6.4a's `SaLink::set_words`).
//! - `ia_net`, `ia_netmask`, `inm_state`, `inm_timer`: relaxed atomics (the C's plain words
//!   under the net lock or `if_maddrlock`).

use hal::link::{Link, SaBytes, SaWords};
use hal::malloc::{M_WAITOK, MArc};
use hal::refcnt::Refcnt;
use hal::rwlock::Guarded;
use hal::slot::ReadMostly;
use hal::sys::net_if::IFF_BROADCAST;
use hal::sys::netinet_in::{InAddr, SockaddrIn};
use hal::sys::socket::AF_INET;

use crate::assert::kpanic;
use crate::net::if_::{IF_TMPLIST_LOCK, M_IFADDR};
use crate::net::if_var::{AtomicCell, Ifaddr, IfaddrRef, Ifmaddr, IfmaddrRef, Ifnet};

/// The size of a `struct sockaddr_in`, its `sin_len`.
const SIN_LEN: u8 = 16;

/// `M_IPMADDR`: the `malloc(9)` type of the multicast records (`<sys/malloc.h>`'s
/// `in_multi`).
pub const M_IPMADDR: i32 = 54;

/// A `struct sockaddr_in` the kernel owns: its bytes in atomic words (`hal::link::SaWords`).
/// Valid all zero.
#[derive(Debug, Default)]
pub struct SinWords(SaWords);

impl SinWords {
    /// An address of zeros.
    pub const fn new() -> Self {
        Self(SaWords::new())
    }

    /// The address, whole.
    #[inline]
    pub fn get(&self) -> SockaddrIn {
        self.0.get::<SockaddrIn>()
    }

    /// The address stored whole (the C's structure assignment).
    #[inline]
    pub fn set(&self, sin: SockaddrIn) {
        self.0.set(&sin);
    }

    /// `sin_addr`: its bytes 4 to 8, the second half of the first word.
    #[inline]
    pub fn addr(&self) -> InAddr {
        let w = self.0.word(0);
        InAddr {
            s_addr: u32::from_ne_bytes([w[4], w[5], w[6], w[7]]),
        }
    }

    /// `sin_len`: its first byte.
    #[inline]
    pub fn len(&self) -> u8 {
        self.0.word(0)[0]
    }

    /// Whether `sin_len` is 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `sin_family`: its second byte.
    #[inline]
    pub fn family(&self) -> u8 {
        self.0.word(0)[1]
    }

    /// A copy of the socket address (`sa_len` bytes, cut at a `sockaddr_in`): what the C's
    /// `ifa_addr`, `ifa_dstaddr` or `ifa_netmask` pointing here gives its readers.
    #[inline]
    pub fn read(&self) -> SaBytes {
        self.0.read(size_of::<SockaddrIn>())
    }
}

/// `struct in_ifaddr`: interface address, Internet version.
pub struct InIfaddr {
    /// `ia_ifa`: protocol-independent info.
    pub ia_ifa: Ifaddr,
    /// `ia_net`: network number of interface.
    pub ia_net: AtomicCell<u32>,
    /// `ia_netmask`: mask of net part.
    pub ia_netmask: AtomicCell<u32>,
    /// `ia_addr`: reserve space for interface name.
    pub ia_addr: SinWords,
    /// `ia_dstaddr`: reserve space for broadcast addr (`ia_broadaddr`).
    pub ia_dstaddr: SinWords,
    /// `ia_sockmask`: reserve space for general netmask.
    pub ia_sockmask: SinWords,
    /// `ia_allhosts`: multicast address record for the allhosts multicast group, the
    /// address's membership (a handle: `in_ifinit`'s `in_addmulti`, `in_purgeaddr`'s
    /// `in_delmulti`). Written under the net lock; a reader copies the answer out and never
    /// holds the cell across a sleep.
    pub ia_allhosts: ReadMostly<Option<MArc<InMulti>>>,
    /// \[N\] `ia_ifa.ifa_list`: the next address of the interface's list (a net cell,
    /// `kernel::net::if_var::IfaList`'s chain).
    pub(crate) ia_list: Guarded<Option<IfaddrRef>>,
    /// \[T\] `ia_ifa.ifa_tmplist`: the next address of `ifconf`'s temporary list.
    pub(crate) ia_tmplist: Guarded<Option<IfaddrRef>>,
}

impl InIfaddr {
    /// A record every member of which is zero (`M_ZERO`), on no list.
    const fn new() -> Self {
        Self {
            ia_ifa: Ifaddr::new(),
            ia_net: AtomicCell::new(),
            ia_netmask: AtomicCell::new(),
            ia_addr: SinWords::new(),
            ia_dstaddr: SinWords::new(),
            ia_sockmask: SinWords::new(),
            ia_allhosts: ReadMostly::new(None),
            ia_list: Guarded::new_net(None),
            ia_tmplist: Guarded::new_static(&IF_TMPLIST_LOCK, None),
        }
    }

    /// `ia_ifp` (`ia_ifa.ifa_ifp`).
    #[inline]
    pub fn ia_ifp(&self) -> &Link<Ifnet> {
        &self.ia_ifa.ifa_ifp
    }

    /// `ia_flags` (`ia_ifa.ifa_flags`).
    #[inline]
    pub fn ia_flags(&self) -> &AtomicCell<u32> {
        &self.ia_ifa.ifa_flags
    }

    /// `ia_broadaddr` (`ia_dstaddr`).
    #[inline]
    pub fn ia_broadaddr(&self) -> &SinWords {
        &self.ia_dstaddr
    }

    /// `ifatoia(ifa)` for an address that may not be an Internet one: `None` for another
    /// family's (the handle's arm).
    #[inline]
    pub fn of(ifa: &IfaddrRef) -> Option<&InIfaddr> {
        match ifa {
            IfaddrRef::In(ia) => Some(ia),
            _ => None,
        }
    }
}

/// `struct in_multi`: Internet multicast address structure. There is one of these for each
/// IP multicast group to which this host belongs on a given network interface. Held by
/// counted handles (`MArc`, the module's redesign).
pub struct InMulti {
    /// `inm_ifma`: protocol-independent info.
    pub inm_ifma: Ifmaddr,
    /// \[I\] `inm_sin`: IPv4 multicast address.
    pub inm_sin: SinWords,
    /// \[m\] `inm_state`: state of membership.
    pub inm_state: AtomicCell<u32>,
    /// \[m\] `inm_timer`: IGMP membership report.
    pub inm_timer: AtomicCell<u32>,
}

impl InMulti {
    /// A record of `ifp` every member of which is zero (`M_ZERO`), its list cell keyed to
    /// `ifp`'s `if_maddrlock`.
    fn new(ifp: &Ifnet) -> Self {
        Self {
            inm_ifma: Ifmaddr::new(ifp),
            inm_sin: SinWords::new(),
            inm_state: AtomicCell::new(),
            inm_timer: AtomicCell::new(),
        }
    }

    /// `inm_refcnt` (`inm_ifma.ifma_refcnt`).
    #[inline]
    pub fn inm_refcnt(&self) -> &Refcnt {
        &self.inm_ifma.ifma_refcnt
    }

    /// `inm_ifidx` (`inm_ifma.ifma_ifidx`).
    #[inline]
    pub fn inm_ifidx(&self) -> &AtomicCell<u32> {
        &self.inm_ifma.ifma_ifidx
    }

    /// `inm_addr` (`inm_sin.sin_addr`).
    #[inline]
    pub fn inm_addr(&self) -> InAddr {
        self.inm_sin.addr()
    }

    /// `ifmatoinm(ifma)` for a record that may not be an Internet one: its handle, `None`
    /// for another family's (the handle's arm).
    #[inline]
    pub fn of(ifma: &IfmaddrRef) -> Option<&MArc<InMulti>> {
        match ifma {
            IfmaddrRef::In(inm) => Some(inm),
            IfmaddrRef::In6(_) => None,
        }
    }
}

/// `ifatoia(ifa)`: the Internet address an `AF_INET` interface address is the first member
/// of; `panic(9)` for another family's (where the C would cast it).
#[inline]
pub fn ifatoia(ifa: &IfaddrRef) -> &InIfaddr {
    match InIfaddr::of(ifa) {
        Some(ia) => ia,
        None => kpanic(format_args!("ifatoia: not an inet address")),
    }
}

/// `ifmatoinm(ifma)`: the Internet multicast record an `AF_INET` multicast address is the
/// first member of; `panic(9)` for another family's (where the C would cast it).
#[inline]
pub fn ifmatoinm(ifma: &IfmaddrRef) -> &InMulti {
    match InMulti::of(ifma) {
        Some(inm) => inm,
        None => kpanic(format_args!("ifmatoinm: not an inet record")),
    }
}

/// `malloc(sizeof *ia, M_IFADDR, M_WAITOK | M_ZERO)` and the set-up `in_ioctl_set_ifaddr`
/// and `in_ioctl_change_ifaddr` give a new address of `ifp`: its address's family and length,
/// the mask's length, the broadcast address's family and length on a broadcast interface, its
/// interface. The handle returned is the creation reference (`refcnt_init`), which
/// `ifa_add` takes for the interface's list; the record is freed with the last handle
/// (`ifafree`'s `free(ifa, M_IFADDR, 0)`).
pub fn in_ifaddr_alloc(ifp: &'static Ifnet) -> MArc<InIfaddr> {
    // The C's `M_ZERO`, then its stores, made on the value `MArc::new` writes whole.
    let ia = InIfaddr::new();
    ia.ia_addr.set(SockaddrIn {
        sin_len: SIN_LEN,
        sin_family: AF_INET,
        ..SockaddrIn::default()
    });
    ia.ia_sockmask.set(SockaddrIn {
        sin_len: 8,
        ..SockaddrIn::default()
    });
    if ifp.if_flags.get() & IFF_BROADCAST != 0 {
        ia.ia_broadaddr().set(SockaddrIn {
            sin_len: SIN_LEN,
            sin_family: AF_INET,
            ..SockaddrIn::default()
        });
    }
    ia.ia_ifp().set(Some(ifp));
    match MArc::new(M_IFADDR, M_WAITOK, ia) {
        Some(ia) => ia,
        None => kpanic(format_args!("in_ifaddr_alloc: out of memory")),
    }
}

/// `malloc(sizeof(*inm), M_IPMADDR, M_WAITOK | M_ZERO)` (`in_addmulti`'s new record) for
/// a group of `ifp`: one handle, the block freed with the last (`free(inm, M_IPMADDR,
/// sizeof(*inm))`).
pub fn in_multi_alloc(ifp: &Ifnet) -> MArc<InMulti> {
    match MArc::new(M_IPMADDR, M_WAITOK, InMulti::new(ifp)) {
        Some(inm) => inm,
        None => kpanic(format_args!("in_multi_alloc: out of memory")),
    }
}

/// An Internet address of `ifp` for the tests and the benches, as `in_ioctl` makes one: its
/// address `addr`, and its mask and destination (broadcast) address when given; the creation
/// reference, for `ifa_add`.
#[cfg(any(test, feature = "doubles"))]
pub fn test_in_ifaddr(
    ifp: &'static Ifnet,
    addr: [u8; 4],
    mask: Option<[u8; 4]>,
    dst: Option<[u8; 4]>,
) -> IfaddrRef {
    let sin = |a: [u8; 4]| SockaddrIn {
        sin_len: SIN_LEN,
        sin_family: AF_INET,
        sin_addr: InAddr {
            s_addr: u32::from_ne_bytes(a),
        },
        ..SockaddrIn::default()
    };
    let ia = in_ifaddr_alloc(ifp);
    ia.ia_addr.set(sin(addr));
    if let Some(m) = mask {
        ia.ia_netmask.set(u32::from_ne_bytes(m));
        ia.ia_sockmask.set(sin(m));
    }
    if let Some(d) = dst {
        ia.ia_dstaddr.set(sin(d));
    }
    IfaddrRef::In(ia)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// `ifatoia` of another family's address: `panic(9)` (the C would cast it); the family is
    /// the handle's arm.
    #[test]
    #[should_panic(expected = "ifatoia: not an inet address")]
    fn ifatoia_of_another_family_is_panic() {
        let ifa: &'static crate::net::if_var::IfaddrLinked =
            std::boxed::Box::leak(std::boxed::Box::new(crate::net::if_var::IfaddrLinked::new()));
        ifa.ifa_refcnt.init();
        let l = IfaddrRef::Linked(crate::net::if_var::LinkedIfa::new(ifa));
        assert!(InIfaddr::of(&l).is_none());
        let _ = ifatoia(&l);
    }

    #[test]
    fn the_address_words_read_as_the_c_structure() {
        let w = SinWords::new();
        assert!(w.is_empty());
        let sin = SockaddrIn {
            sin_len: 16,
            sin_family: AF_INET,
            sin_port: 0x1234,
            sin_addr: InAddr {
                s_addr: u32::from_ne_bytes([10, 0, 2, 15]),
            },
            sin_zero: [0; 8],
        };
        w.set(sin);
        assert_eq!(w.get(), sin);
        assert_eq!(w.addr(), sin.sin_addr);
        assert_eq!((w.len(), w.family()), (16, AF_INET));
    }
}
/* </TESTS> */
