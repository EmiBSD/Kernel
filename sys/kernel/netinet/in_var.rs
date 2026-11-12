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
//! `struct ifaddr` it embeds first holds the protocol-independent part; its `ifa_addr`,
//! `ifa_dstaddr` and `ifa_netmask` name the `sockaddr_in`s the record reserves. Locks: \[I\]
//! immutable after creation, \[m\] the parent interface's `if_maddrlock`; the other members
//! change under the net lock, as in the C.
//!
//! ## Deviations
//! - A record is never freed: the last `ifafree` of an address ([`in_ifafree`]) and the last
//!   `in_delmulti` of a membership ([`in_multi_free`]) put it on a free list, which the next
//!   allocation ([`in_ifaddr_alloc`], [`in_multi_alloc`]) takes from before a new `malloc(9)`
//!   (the mount's idiom, D-N5.2-2; N6.4a's R1): `&'static` is then true for every holder
//!   (the interface's list, `rt_ifa`, the pcbs' and pf's copies), and `vmstat -m`'s
//!   `ifaddr` and `ipmaddr` rows count the records ever alive at once (a high-water mark)
//!   where the C's fall after a delete. The alternative, counted handles in the lists, waits
//!   for both families (N6.5, docs/SCAFFOLDING.md).
//! - The `ia_list` TAILQ entry is dropped: nothing links the records through it at the pin.
//!
//! ## Redesign
//! - LZ's `#[repr(C)]` records of `Cell`s cast from their generic part (`ifatoia`,
//!   `ifmatoinm`: pointer casts checked by the address family, one `unsafe` each) -> records
//!   reached from the generic part through a back-link set at creation (`Ifaddr::ifa_in`,
//!   `Ifmaddr::ifma_in`: one load where the C casts, the cast's family test the link's
//!   presence: only `in.c` makes `AF_INET` records).
//! - The C's `struct sockaddr_in` members -> [`SinWords`], the address in atomic words that
//!   the generic part's `SaLink`s name (`SaLink::set_words`, no token: N6.4a's R2), read and
//!   written whole (`get`, `set`: LZ's `Cell` interface, so `bsd`'s readers are unchanged) or
//!   by the address alone (`addr`, one word load).
//! - `ia_net`, `ia_netmask`, `inm_state`, `inm_timer`: relaxed atomics (the C's plain words
//!   under the net lock or `if_maddrlock`).

use hal::intr::IPL_SOFTNET;
use hal::link::{Link, SaWords};
use hal::list::{List, ListEntry};
use hal::malloc::{M_WAITOK, MBox};
use hal::mutex::Mutexed;
use hal::refcnt::Refcnt;
use hal::sys::net_if::IFF_BROADCAST;
use hal::sys::netinet_in::{InAddr, SockaddrIn};
use hal::sys::socket::AF_INET;

use crate::assert::kpanic;
use crate::net::if_::M_IFADDR;
use crate::net::if_var::{AtomicCell, Ifaddr, Ifmaddr, Ifnet};

/// The size of a `struct sockaddr_in`, its `sin_len`.
const SIN_LEN: u8 = 16;

/// `M_IPMADDR`: the `malloc(9)` type of the multicast records (`<sys/malloc.h>`'s
/// `in_multi`).
pub const M_IPMADDR: i32 = 54;

/// A `struct sockaddr_in` the kernel owns: its bytes in atomic words (`hal::link::SaWords`)
/// that an interface record's `SaLink` names. Valid all zero.
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

    /// The words, which a record's `SaLink` names.
    fn words(&'static self) -> &'static SaWords {
        &self.0
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
    /// `ia_allhosts`: multicast address record for the allhosts multicast group.
    pub ia_allhosts: Link<InMulti>,
    /// The free list's entry (the module's deviations).
    ia_free: ListEntry<InIfaddr>,
}

impl InIfaddr {
    /// A record every member of which is zero (`M_ZERO`).
    const fn new() -> Self {
        Self {
            ia_ifa: Ifaddr::new(),
            ia_net: AtomicCell::new(),
            ia_netmask: AtomicCell::new(),
            ia_addr: SinWords::new(),
            ia_dstaddr: SinWords::new(),
            ia_sockmask: SinWords::new(),
            ia_allhosts: Link::new(),
            ia_free: ListEntry::new(),
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
    /// family's.
    #[inline]
    pub fn of(ifa: &Ifaddr) -> Option<&'static InIfaddr> {
        ifa.ifa_in.get()
    }

    /// A recycled record made as `malloc(M_ZERO)` leaves a new one: it is on no list and its
    /// last reference is gone, so no one reaches it; its links still name its own storage.
    fn reset(&self) {
        let ifa = &self.ia_ifa;
        if List::<crate::net::if_var::IfAddrlist>::is_linked(ifa) {
            kpanic(format_args!(
                "in_ifaddr_alloc: a recycled address on a list"
            ));
        }
        ifa.ifa_ifp.set(None);
        ifa.ifa_flags.set(0);
        ifa.ifa_metric.set(0);
        self.ia_net.set(0);
        self.ia_netmask.set(0);
        self.ia_addr.set(SockaddrIn::default());
        self.ia_dstaddr.set(SockaddrIn::default());
        self.ia_sockmask.set(SockaddrIn::default());
        self.ia_allhosts.set(None);
    }
}

hal::list_adapter!(
    /// The free list of Internet addresses (the module's deviations).
    IaFreeList: InIfaddr, ia_free
);

/// `struct in_multi`: Internet multicast address structure. There is one of these for each
/// IP multicast group to which this host belongs on a given network interface.
pub struct InMulti {
    /// `inm_ifma`: protocol-independent info.
    pub inm_ifma: Ifmaddr,
    /// \[I\] `inm_sin`: IPv4 multicast address.
    pub inm_sin: SinWords,
    /// \[m\] `inm_state`: state of membership.
    pub inm_state: AtomicCell<u32>,
    /// \[m\] `inm_timer`: IGMP membership report.
    pub inm_timer: AtomicCell<u32>,
    /// The free list's entry (the module's deviations).
    inm_free: ListEntry<InMulti>,
}

impl InMulti {
    /// A record every member of which is zero (`M_ZERO`).
    const fn new() -> Self {
        Self {
            inm_ifma: Ifmaddr::new(),
            inm_sin: SinWords::new(),
            inm_state: AtomicCell::new(),
            inm_timer: AtomicCell::new(),
            inm_free: ListEntry::new(),
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

    /// `ifmatoinm(ifma)` for a record that may not be an Internet one: `None` for another
    /// family's.
    #[inline]
    pub fn of(ifma: &Ifmaddr) -> Option<&'static InMulti> {
        ifma.ifma_in.get()
    }
}

hal::list_adapter!(
    /// The free list of multicast records (the module's deviations).
    InmFreeList: InMulti, inm_free
);

/// The Internet addresses no interface has any more, for [`in_ifaddr_alloc`].
static IA_FREE: Mutexed<List<IaFreeList>> = Mutexed::new(IPL_SOFTNET, List::new());

/// The multicast records no interface has any more, for [`in_multi_alloc`].
static INM_FREE: Mutexed<List<InmFreeList>> = Mutexed::new(IPL_SOFTNET, List::new());

/// `ifatoia(ifa)`: the Internet address an `AF_INET` interface address is the first member
/// of; `panic(9)` for another family's (where the C would cast it).
#[inline]
pub fn ifatoia(ifa: &Ifaddr) -> &'static InIfaddr {
    match InIfaddr::of(ifa) {
        Some(ia) => ia,
        None => kpanic(format_args!("ifatoia: not an inet address")),
    }
}

/// `ifmatoinm(ifma)`: the Internet multicast record an `AF_INET` multicast address is the
/// first member of; `panic(9)` for another family's.
#[inline]
pub fn ifmatoinm(ifma: &Ifmaddr) -> &'static InMulti {
    match InMulti::of(ifma) {
        Some(inm) => inm,
        None => kpanic(format_args!("ifmatoinm: not an inet record")),
    }
}

/// `malloc(sizeof *ia, M_IFADDR, M_WAITOK | M_ZERO)` and the set-up `in_ioctl_set_ifaddr`
/// and `in_ioctl_change_ifaddr` give a new address of `ifp`: its reference, its address's
/// family and length, the generic part's links to its storage, the mask's length, the
/// broadcast address's family and length on a broadcast interface, its interface. A record
/// off the free list first.
pub fn in_ifaddr_alloc(ifp: &'static Ifnet) -> &'static InIfaddr {
    let reused = IA_FREE.lock().pop_head();
    let ia: &'static InIfaddr = match reused {
        Some(ia) => {
            ia.reset();
            ia
        }
        // The C's `M_ZERO`: `MBox::new` writes the zeroed record whole.
        None => match MBox::new(M_IFADDR, M_WAITOK, InIfaddr::new()) {
            Some(b) => {
                let ia = MBox::leak(b);
                ia.ia_ifa.ifa_in.set(Some(ia));
                ia.ia_ifa
                    .ifa_addr
                    .set_words::<SockaddrIn>(Some(ia.ia_addr.words()));
                ia.ia_ifa
                    .ifa_dstaddr
                    .set_words::<SockaddrIn>(Some(ia.ia_dstaddr.words()));
                ia.ia_ifa
                    .ifa_netmask
                    .set_words::<SockaddrIn>(Some(ia.ia_sockmask.words()));
                ia
            }
            None => kpanic(format_args!("in_ifaddr_alloc: out of memory")),
        },
    };
    ia.ia_ifa.ifa_refcnt.init();
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
    ia
}

/// `free(ifa, M_IFADDR, 0)` of an Internet address whose last reference `ifafree` gave back:
/// the record onto the free list (the module's deviations).
pub fn in_ifafree(ia: &'static InIfaddr) {
    IA_FREE.lock().insert_tail(ia);
}

/// `malloc(sizeof(*inm), M_IPMADDR, M_WAITOK | M_ZERO)` (`in_addmulti`'s new record), a
/// record off the free list first.
pub fn in_multi_alloc() -> &'static InMulti {
    let reused = INM_FREE.lock().pop_head();
    match reused {
        Some(inm) => {
            let ifma = &inm.inm_ifma;
            if List::<crate::net::if_var::IfMaddrlist>::is_linked(ifma) {
                kpanic(format_args!("in_multi_alloc: a recycled record on a list"));
            }
            ifma.ifma_ifidx.set(0);
            inm.inm_sin.set(SockaddrIn::default());
            inm.inm_state.set(0);
            inm.inm_timer.set(0);
            inm
        }
        None => match MBox::new(M_IPMADDR, M_WAITOK, InMulti::new()) {
            Some(b) => {
                let inm = MBox::leak(b);
                inm.inm_ifma.ifma_in.set(Some(inm));
                inm.inm_ifma
                    .ifma_addr
                    .set_words::<SockaddrIn>(Some(inm.inm_sin.words()));
                inm
            }
            None => kpanic(format_args!("in_multi_alloc: out of memory")),
        },
    }
}

/// `free(inm, M_IPMADDR, sizeof(*inm))`: the record onto the free list (the module's
/// deviations).
pub fn in_multi_free(inm: &'static InMulti) {
    INM_FREE.lock().insert_tail(inm);
}

/// Forgets the free lists without touching their records, which an earlier test's memory
/// holds (`setup_real_memory`'s rule, testing.md).
#[cfg(any(test, feature = "doubles"))]
pub fn in_var_test_reset() {
    *IA_FREE.lock() = List::new();
    *INM_FREE.lock() = List::new();
}

/// The free lists' lengths, for the tests.
#[cfg(test)]
pub(crate) fn free_counts() -> (usize, usize) {
    (
        IA_FREE.lock().iter().count(),
        INM_FREE.lock().iter().count(),
    )
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

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
