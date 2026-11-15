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
//! IPv6 interface addresses and multicast records, the kernel's half of
//! `<netinet6/in6_var.h>`: `struct in6_ifaddr`, `struct in6_multi`, their casts from the
//! generic records, the kernel's memberships of an address and the `_KERNEL` macros. The
//! userland half (`struct in6_ifreq`, `struct in6_aliasreq`, the lifetimes, the statistics,
//! the commands) is `hal::sys::netinet6_in6_var`; the prototypes are `kernel::netinet6::in6`'s
//! (`in6.c`).
//!
//! Upstream: sys/netinet6/in6_var.h @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_var.rs@3c62ede68802
//!
//! One `struct in6_ifaddr` is allocated for each interface with an IPv6 address. The `struct
//! ifaddr` it embeds first holds the protocol-independent part; its `ifa_addr`,
//! `ifa_dstaddr` (a point-to-point or loopback interface's only) and `ifa_netmask` name the
//! `sockaddr_in6`s the record reserves. Locks: \[I\] immutable after creation, \[m\] the
//! parent interface's `if_maddrlock`; the other members change under the net lock
//! (`in6_update_ifa`'s `NET_ASSERT_LOCKED`), as in the C.
//!
//! pltime/vltime are just for future reference (required to implement the 2 hour rule for
//! hosts); they are never modified by `nd6_timeout` or anywhere else. userland -> kernel:
//! accept pltime/vltime; kernel -> userland: throw up everything; in kernel: modify
//! preferred/expire only.
//!
//! ## Deviations
//! - A record is never freed: the last `ifafree` of an address ([`in6_ifafree`]) and the
//!   last `in6_delmulti` of a membership ([`in6_multi_free`]) put it on a free list, which
//!   the next allocation ([`in6_ifaddr_alloc`], [`in6_multi_alloc`]) takes from before a new
//!   `malloc(9)` (`kernel::netinet::in_var`'s shape, N6.4a's R1; D-2 of N6.5): `&'static` is
//!   then true for every holder (the interface's list, `rt_ifa`, DAD's `dad_ifa`, the pcbs'
//!   and pf's copies), and `vmstat -m`'s `ifaddr` and `ipmaddr` rows count the records ever
//!   alive at once (a high-water mark) where the C's fall after a delete. The counted
//!   handles in the lists are the user's slice (D-3).
//! - The `ia_list` TAILQ entry is dropped: nothing links the records through it at the pin
//!   (no `in6_ifaddr` list head exists in netinet6).
//! - `struct in6_multi_mship` (a `malloc`'d list entry per membership) is not the kernel's:
//!   an address's memberships are an array of the records ([`In6Memberships`], the IPv4
//!   `imo_membership` shape, D-11), whose `M_IPMADDR` allocation `vmstat -m` counts instead
//!   of one entry each. `bsd`'s `ip6_setmoptions` keeps LZ's entries until N6.5c.
//!
//! ## Redesign
//! - LZ's `#[repr(C)]` records of `Cell`s cast from their generic part (`ifatoia6`,
//!   `ifmatoin6m`: pointer casts checked by the address family, one `unsafe` each, and three
//!   `unsafe impl Sync`) -> records reached from the generic part through a back-link set at
//!   creation (`Ifaddr::ifa_in6`, `Ifmaddr::ifma_in6`: one load where the C casts, the
//!   cast's family test the link's presence: only `in6.c` makes `AF_INET6` records).
//! - The C's `struct sockaddr_in6` members -> [`Sin6Words`], the address in atomic words
//!   that the generic part's `SaLink`s name (`SaLink::set_words`, `kernel::netinet::in_var`'s
//!   `SinWords` twin), read and written whole (`get`, `set`: LZ's `Cell` interface) or by
//!   the address alone (`addr`, two word loads).
//! - `ia6_flags`, `ia6_updatetime`, `in6m_state`, `in6m_timer`: relaxed atomics; the
//!   lifetimes ([`In6AddrlifetimeCell`]) four of them, written whole by `in6_update_ifa`
//!   under the net lock (the C's struct assignment; a reader without it may see a mix, as
//!   the C's may).
//! - `ia6_memberships` (a `LIST_HEAD` the net lock guards) -> [`In6Memberships`] inside a
//!   `hal::mutex::Mutexed` of the record (IPL_SOFTNET): the record is shared (`&'static`)
//!   and the C's writers, `in6_update_ifa` and `in6_purgeaddr`, hold the net lock that a
//!   cell keyed to it cannot open for `bsd`'s detached holders. One mutex pair per join and
//!   one per purge, control path only (named in `kernel::netinet6::in6`'s lock table).
//!
//! **Lock and spl transitions** against the C: the free lists' mutex once per allocation
//! and per last release (the C's `malloc`/`free` take the allocator's); the memberships'
//! mutex once per join and once per purge (above the C's none: the net lock serves it).

use hal::intr::IPL_SOFTNET;
use hal::link::{Link, SaWords};
use hal::list::{List, ListEntry};
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::mutex::Mutexed;
use hal::refcnt::Refcnt;
use hal::sys::net_if::{IFF_LOOPBACK, IFF_POINTOPOINT};
use hal::sys::netinet6_in6::{In6Addr, SockaddrIn6};
use hal::sys::netinet6_in6_var::In6Addrlifetime;
use hal::sys::socket::AF_INET6;
use hal::sys::types::Time;

use crate::assert::kpanic;
use crate::net::if_::M_IFADDR;
use crate::net::if_var::{AtomicCell, Ifaddr, Ifmaddr, Ifnet};
use crate::netinet::in_var::M_IPMADDR;

/// The size of a `struct sockaddr_in6`, its `sin6_len`.
pub const SIN6_LEN: u8 = 28;

/// The offset of `sin6_addr` in a `struct sockaddr_in6`: its words 1 and 2 hold it.
const SIN6_ADDR: usize = 8;

/// A `struct sockaddr_in6` the kernel owns: its bytes in atomic words (`hal::link::SaWords`)
/// that an interface record's `SaLink` names. Valid all zero.
#[derive(Debug, Default)]
pub struct Sin6Words(SaWords);

impl Sin6Words {
    /// An address of zeros.
    pub const fn new() -> Self {
        Self(SaWords::new())
    }

    /// The address, whole.
    #[inline]
    pub fn get(&self) -> SockaddrIn6 {
        self.0.get::<SockaddrIn6>()
    }

    /// The address stored whole (the C's structure assignment).
    #[inline]
    pub fn set(&self, sin6: SockaddrIn6) {
        self.0.set(&sin6);
    }

    /// `sin6_addr`: its bytes 8 to 24, the words 1 and 2.
    #[inline]
    pub fn addr(&self) -> In6Addr {
        let mut a = In6Addr::default();
        a.s6_addr[..8].copy_from_slice(&self.0.word(SIN6_ADDR / 8));
        a.s6_addr[8..].copy_from_slice(&self.0.word(SIN6_ADDR / 8 + 1));
        a
    }

    /// `sin6_len`: its first byte.
    #[inline]
    pub fn len(&self) -> u8 {
        self.0.word(0)[0]
    }

    /// Whether `sin6_len` is 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `sin6_family`: its second byte.
    #[inline]
    pub fn family(&self) -> u8 {
        self.0.word(0)[1]
    }

    /// The words, which a record's `SaLink` names.
    fn words(&'static self) -> &'static SaWords {
        &self.0
    }
}

/// `struct in6_addrlifetime` as a member of a shared record: four relaxed atomics with LZ's
/// `Cell` interface (`get`, `set` of the whole structure). Valid all zero.
#[derive(Debug, Default)]
pub struct In6AddrlifetimeCell {
    /// `ia6t_expire`.
    expire: AtomicCell<i64>,
    /// `ia6t_preferred`.
    preferred: AtomicCell<i64>,
    /// `ia6t_vltime`.
    vltime: AtomicCell<u32>,
    /// `ia6t_pltime`.
    pltime: AtomicCell<u32>,
}

impl In6AddrlifetimeCell {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            expire: AtomicCell::new(),
            preferred: AtomicCell::new(),
            vltime: AtomicCell::new(),
            pltime: AtomicCell::new(),
        }
    }

    /// The lifetimes, whole.
    #[inline]
    pub fn get(&self) -> In6Addrlifetime {
        In6Addrlifetime {
            ia6t_expire: self.expire.get(),
            ia6t_preferred: self.preferred.get(),
            ia6t_vltime: self.vltime.get(),
            ia6t_pltime: self.pltime.get(),
        }
    }

    /// The lifetimes stored whole (`in6_update_ifa`'s, under the net lock).
    #[inline]
    pub fn set(&self, lt: In6Addrlifetime) {
        self.expire.set(lt.ia6t_expire);
        self.preferred.set(lt.ia6t_preferred);
        self.vltime.set(lt.ia6t_vltime);
        self.pltime.set(lt.ia6t_pltime);
    }

    /// `ia6t_vltime` alone.
    #[inline]
    pub fn vltime(&self) -> u32 {
        self.vltime.get()
    }

    /// `ia6t_pltime` alone.
    #[inline]
    pub fn pltime(&self) -> u32 {
        self.pltime.get()
    }
}

/// The multicast groups the kernel joined for one owner (`LIST_HEAD(, in6_multi_mship)`:
/// an address's `ia6_memberships`, and N6.5c's socket options): the records, the first
/// [`len`](Self::len) slots of an `M_IPMADDR` array doubled when full (the module's
/// deviations). A join appends, a leave finds its member (the C's `LIST_FOREACH`, O(n)) and
/// closes the gap.
#[derive(Default)]
pub struct In6Memberships {
    /// The records; `None` before the first join.
    v: Option<MArray<Option<&'static In6Multi>>>,
    /// How many of `v`'s slots are in use.
    n: usize,
}

impl In6Memberships {
    /// No membership.
    pub const fn new() -> Self {
        Self { v: None, n: 0 }
    }

    /// How many groups are joined.
    #[inline]
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether no group is joined (`LIST_EMPTY`).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// The records, in the order they were joined.
    pub fn iter(&self) -> impl Iterator<Item = &'static In6Multi> + '_ {
        self.v
            .as_deref()
            .into_iter()
            .flat_map(|v| v.iter().take(self.n).copied().flatten())
    }

    /// The C's `imm = malloc(sizeof(*imm), M_IPMADDR, M_NOWAIT)` before the join: room for one
    /// more membership, the array allocated (4 slots) or doubled when full, under the
    /// owner's mutex (malloc(9) allows `M_NOWAIT` there, and the old array is freed there
    /// once copied); `Err(ENOBUFS)` when it cannot be had, before anything is joined.
    pub fn reserve(&mut self) -> Result<(), hal::sys::errno::Errno> {
        let cap = self.v.as_deref().map_or(0, <[_]>::len);
        if self.n < cap {
            return Ok(());
        }
        let old = self.v.as_deref().unwrap_or(&[]);
        let Some(nv) = MArray::new(M_IPMADDR, M_NOWAIT, (cap * 2).max(4), |i| {
            old.get(i).copied().flatten()
        }) else {
            return Err(hal::sys::errno::Errno::ENOBUFS);
        };
        // The old array freed once copied.
        self.v = Some(nv);
        Ok(())
    }

    /// `LIST_INSERT_HEAD(&memberships, imm, i6mm_chain)` of a membership joined after
    /// [`reserve`](Self::reserve) made its room: `in6m` recorded in it. Only the
    /// `in6_update_ifa` call that made an address joins its groups (the C's `hostIsNew`),
    /// one after another under the net lock, and nothing else takes room (a purge only
    /// frees it), so the room is still there and the push cannot fail. Were it gone, the
    /// push reserves again (`Err(ENOBUFS)` if that fails, the caller then leaving the
    /// group): a fallback no caller reaches.
    pub fn push(&mut self, in6m: &'static In6Multi) -> Result<(), hal::sys::errno::Errno> {
        self.reserve()?;
        match self.v.as_deref_mut().and_then(|v| v.get_mut(self.n)) {
            Some(slot) => *slot = Some(in6m),
            None => kpanic(format_args!(
                "in6_memberships: slot {} out of the array",
                self.n
            )),
        }
        self.n += 1;
        Ok(())
    }

    /// `LIST_FIRST` and `LIST_REMOVE` of the list's head, the newest membership
    /// (`LIST_INSERT_HEAD`'s): its record, its slot emptied.
    pub fn pop_last(&mut self) -> Option<&'static In6Multi> {
        let n = self.n.checked_sub(1)?;
        let last = self.v.as_deref_mut()?.get_mut(n)?.take();
        self.n = n;
        last
    }

    /// `LIST_FOREACH` from the list's head, the newest first (`LIST_INSERT_HEAD`'s order),
    /// to the first membership `matches` accepts, then `LIST_REMOVE` of it: its record, the
    /// newer ones moved down one slot (the gap closed, the order kept). O(n) as the C's walk.
    pub fn take_newest(
        &mut self,
        mut matches: impl FnMut(&'static In6Multi) -> bool,
    ) -> Option<&'static In6Multi> {
        let n = self.n;
        let v = self.v.as_deref_mut()?;
        let i = (0..n).rev().find(|&i| v[i].is_some_and(&mut matches))?;
        let found = v[i].take();
        v.copy_within(i + 1..n, i);
        v[n - 1] = None;
        self.n = n - 1;
        found
    }
}

/// `struct in6_ifaddr`: interface address, Internet version 6.
pub struct In6Ifaddr {
    /// `ia_ifa`: protocol-independent info.
    pub ia_ifa: Ifaddr,
    /// `ia_addr`: interface address.
    pub ia_addr: Sin6Words,
    /// `ia_gwaddr`: router we learned address from.
    pub ia_gwaddr: Sin6Words,
    /// `ia_dstaddr`: space for destination addr.
    pub ia_dstaddr: Sin6Words,
    /// `ia_prefixmask`: prefix mask.
    pub ia_prefixmask: Sin6Words,
    /// `ia6_flags`: `IN6_IFF_*`.
    pub ia6_flags: AtomicCell<i32>,
    /// `ia6_lifetime`.
    pub ia6_lifetime: In6AddrlifetimeCell,
    /// `ia6_updatetime`.
    pub ia6_updatetime: AtomicCell<Time>,
    /// `ia6_memberships`: multicast addresses joined from the kernel (the module's
    /// redesign).
    pub ia6_memberships: Mutexed<In6Memberships>,
    /// The free list's entry (the module's deviations).
    ia_free: ListEntry<In6Ifaddr>,
}

impl In6Ifaddr {
    /// A record every member of which is zero (`M_ZERO`).
    const fn new() -> Self {
        Self {
            ia_ifa: Ifaddr::new(),
            ia_addr: Sin6Words::new(),
            ia_gwaddr: Sin6Words::new(),
            ia_dstaddr: Sin6Words::new(),
            ia_prefixmask: Sin6Words::new(),
            ia6_flags: AtomicCell::new(),
            ia6_lifetime: In6AddrlifetimeCell::new(),
            ia6_updatetime: AtomicCell::new(),
            ia6_memberships: Mutexed::new(IPL_SOFTNET, In6Memberships::new()),
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

    /// `ifatoia6(ifa)` for an address that may not be an IPv6 one: `None` for another
    /// family's.
    #[inline]
    pub fn of(ifa: &Ifaddr) -> Option<&'static In6Ifaddr> {
        ifa.ifa_in6.get()
    }

    /// A recycled record made as `malloc(M_ZERO)` leaves a new one: it is on no list and its
    /// last reference is gone, so no one reaches it; its links still name its own storage.
    fn reset(&self) {
        let ifa = &self.ia_ifa;
        if List::<crate::net::if_var::IfAddrlist>::is_linked(ifa) {
            kpanic(format_args!(
                "in6_ifaddr_alloc: a recycled address on a list"
            ));
        }
        ifa.ifa_ifp.set(None);
        ifa.ifa_flags.set(0);
        ifa.ifa_metric.set(0);
        self.ia_addr.set(SockaddrIn6::default());
        self.ia_gwaddr.set(SockaddrIn6::default());
        self.ia_dstaddr.set(SockaddrIn6::default());
        self.ia_prefixmask.set(SockaddrIn6::default());
        self.ia6_flags.set(0);
        self.ia6_lifetime.set(In6Addrlifetime::default());
        self.ia6_updatetime.set(0);
        if !self.ia6_memberships.lock().is_empty() {
            kpanic(format_args!(
                "in6_ifaddr_alloc: a recycled address with memberships"
            ));
        }
    }
}

hal::list_adapter!(
    /// The free list of IPv6 addresses (the module's deviations).
    Ia6FreeList: In6Ifaddr, ia_free
);

/// `struct in6_multi`: IPv6 multicast address structure.
pub struct In6Multi {
    /// `in6m_ifma`: protocol-independent info.
    pub in6m_ifma: Ifmaddr,
    /// \[I\] `in6m_sin`: IPv6 multicast address.
    pub in6m_sin: Sin6Words,
    /// \[m\] `in6m_state`: state of membership.
    pub in6m_state: AtomicCell<u32>,
    /// \[m\] `in6m_timer`: MLD6 membership report.
    pub in6m_timer: AtomicCell<u32>,
    /// The free list's entry (the module's deviations).
    in6m_free: ListEntry<In6Multi>,
}

impl In6Multi {
    /// A record every member of which is zero (`M_ZERO`).
    const fn new() -> Self {
        Self {
            in6m_ifma: Ifmaddr::new(),
            in6m_sin: Sin6Words::new(),
            in6m_state: AtomicCell::new(),
            in6m_timer: AtomicCell::new(),
            in6m_free: ListEntry::new(),
        }
    }

    /// `in6m_refcnt` (`in6m_ifma.ifma_refcnt`).
    #[inline]
    pub fn in6m_refcnt(&self) -> &Refcnt {
        &self.in6m_ifma.ifma_refcnt
    }

    /// `in6m_ifidx` (`in6m_ifma.ifma_ifidx`).
    #[inline]
    pub fn in6m_ifidx(&self) -> &AtomicCell<u32> {
        &self.in6m_ifma.ifma_ifidx
    }

    /// `in6m_addr` (`in6m_sin.sin6_addr`).
    #[inline]
    pub fn in6m_addr(&self) -> In6Addr {
        self.in6m_sin.addr()
    }

    /// `ifmatoin6m(ifma)` for a record that may not be an IPv6 one: `None` for another
    /// family's.
    #[inline]
    pub fn of(ifma: &Ifmaddr) -> Option<&'static In6Multi> {
        ifma.ifma_in6.get()
    }
}

hal::list_adapter!(
    /// The free list of IPv6 multicast records (the module's deviations).
    Inm6FreeList: In6Multi, in6m_free
);

// The two free lists' mutexes are IPL_SOFTNET: every taker runs at or below the softnet
// level, never from a hardware interrupt: the allocations (`in6_update_ifa`, `in6_addmulti`,
// process context or the softnet tasks), `in6_ifafree` (the last `ifafree`, from `rtfree`,
// `ifa_del` and DAD's release) and `in6_multi_free` (`in6_delmulti`'s).

/// The IPv6 addresses no interface has any more, for [`in6_ifaddr_alloc`].
static IA6_FREE: Mutexed<List<Ia6FreeList>> = Mutexed::new(IPL_SOFTNET, List::new());

/// The IPv6 multicast records no interface has any more, for [`in6_multi_alloc`].
static INM6_FREE: Mutexed<List<Inm6FreeList>> = Mutexed::new(IPL_SOFTNET, List::new());

/// `ifatoia6(ifa)`: the IPv6 address an `AF_INET6` interface address is the first member of;
/// `panic(9)` for another family's (where the C would cast it).
#[inline]
pub fn ifatoia6(ifa: &Ifaddr) -> &'static In6Ifaddr {
    match In6Ifaddr::of(ifa) {
        Some(ia6) => ia6,
        None => kpanic(format_args!("ifatoia6: not an inet6 address")),
    }
}

/// `ifmatoin6m(ifma)`: the IPv6 multicast record an `AF_INET6` multicast address is the
/// first member of; `panic(9)` for another family's.
#[inline]
pub fn ifmatoin6m(ifma: &Ifmaddr) -> &'static In6Multi {
    match In6Multi::of(ifma) {
        Some(in6m) => in6m,
        None => kpanic(format_args!("ifmatoin6m: not an inet6 record")),
    }
}

/// `IA6_IN6(ia)`: the interface address.
#[inline]
pub fn ia6_in6(ia: &In6Ifaddr) -> In6Addr {
    ia.ia_addr.addr()
}

/// `IA6_DSTIN6(ia)`: the destination address.
#[inline]
pub fn ia6_dstin6(ia: &In6Ifaddr) -> In6Addr {
    ia.ia_dstaddr.addr()
}

/// `IA6_MASKIN6(ia)`: the prefix mask.
#[inline]
pub fn ia6_maskin6(ia: &In6Ifaddr) -> In6Addr {
    ia.ia_prefixmask.addr()
}

/// `IA6_SIN6(ia)`: the interface address as a socket address.
#[inline]
pub fn ia6_sin6(ia: &In6Ifaddr) -> SockaddrIn6 {
    ia.ia_addr.get()
}

/// `IA6_DSTSIN6(ia)`: the destination address as a socket address.
#[inline]
pub fn ia6_dstsin6(ia: &In6Ifaddr) -> SockaddrIn6 {
    ia.ia_dstaddr.get()
}

/// `IFA_IN6(x)`: the IPv6 address of an `AF_INET6` interface address (its record's).
#[inline]
pub fn ifa_in6(x: &Ifaddr) -> In6Addr {
    ifatoia6(x).ia_addr.addr()
}

/// `IFA_DSTIN6(x)`: the IPv6 destination address of an `AF_INET6` interface address.
#[inline]
pub fn ifa_dstin6(x: &Ifaddr) -> In6Addr {
    ifatoia6(x).ia_dstaddr.addr()
}

/// `IN6_ARE_MASKED_ADDR_EQUAL(d, a, m)`: whether `d` and `a` agree under mask `m`.
#[inline]
pub const fn in6_are_masked_addr_equal(d: &In6Addr, a: &In6Addr, m: &In6Addr) -> bool {
    let mut i = 0;
    while i < 4 {
        if (d.s6_addr32(i) ^ a.s6_addr32(i)) & m.s6_addr32(i) != 0 {
            return false;
        }
        i += 1;
    }
    true
}

/// `IN6_ARE_SCOPE_CMP(a, b)`.
#[inline]
pub const fn in6_are_scope_cmp(a: i32, b: i32) -> i32 {
    a - b
}

/// `IN6_ARE_SCOPE_EQUAL(a, b)`.
#[inline]
pub const fn in6_are_scope_equal(a: i32, b: i32) -> bool {
    a == b
}

/// `malloc(sizeof(*ia6), M_IFADDR, M_WAITOK | M_ZERO)` and the set-up `in6_update_ifa` gives
/// a new address of `ifp`: its reference, its address's family and length, the generic part's
/// links to its storage (`ifa_dstaddr` only on a point-to-point or loopback interface:
/// "some functions expect that ifa_dstaddr is not NULL for p2p interfaces"), its time stamp
/// and its interface. A record off the free list first.
pub fn in6_ifaddr_alloc(ifp: &'static Ifnet, now: Time) -> &'static In6Ifaddr {
    let reused = IA6_FREE.lock().pop_head();
    let ia6: &'static In6Ifaddr = match reused {
        Some(ia6) => {
            ia6.reset();
            ia6
        }
        // The C's `M_ZERO`: `MBox::new` writes the zeroed record whole.
        None => match MBox::new(M_IFADDR, M_WAITOK, In6Ifaddr::new()) {
            Some(b) => {
                let ia6 = MBox::leak(b);
                ia6.ia_ifa.ifa_in6.set(Some(ia6));
                ia6.ia_ifa
                    .ifa_addr
                    .set_words::<SockaddrIn6>(Some(ia6.ia_addr.words()));
                ia6.ia_ifa
                    .ifa_netmask
                    .set_words::<SockaddrIn6>(Some(ia6.ia_prefixmask.words()));
                ia6
            }
            None => kpanic(format_args!("in6_ifaddr: no memory")),
        },
    };
    ia6.ia_ifa.ifa_refcnt.init();
    // Initialize the address and masks, and put time stamp
    ia6.ia_addr.set(SockaddrIn6 {
        sin6_len: SIN6_LEN,
        sin6_family: AF_INET6,
        ..SockaddrIn6::default()
    });
    ia6.ia6_updatetime.set(now);
    if ifp.if_flags.get() & (IFF_POINTOPOINT | IFF_LOOPBACK) != 0 {
        // XXX: some functions expect that ifa_dstaddr is not NULL for p2p interfaces.
        ia6.ia_ifa
            .ifa_dstaddr
            .set_words::<SockaddrIn6>(Some(ia6.ia_dstaddr.words()));
    } else {
        ia6.ia_ifa.ifa_dstaddr.clear();
    }
    ia6.ia_ifp().set(Some(ifp));
    ia6
}

/// `free(ifa, M_IFADDR, 0)` of an IPv6 address whose last reference `ifafree` gave back: the
/// record onto the free list (the module's deviations).
pub(crate) fn in6_ifafree(ia6: &'static In6Ifaddr) {
    IA6_FREE.lock().insert_tail(ia6);
}

/// `malloc(sizeof(*in6m), M_IPMADDR, M_NOWAIT | M_ZERO)` (`in6_addmulti`'s new record), a
/// record off the free list first; `None` when the allocator has none (the C's `ENOBUFS`).
pub fn in6_multi_alloc() -> Option<&'static In6Multi> {
    let reused = INM6_FREE.lock().pop_head();
    match reused {
        Some(in6m) => {
            let ifma = &in6m.in6m_ifma;
            if List::<crate::net::if_var::IfMaddrlist>::is_linked(ifma) {
                kpanic(format_args!("in6_multi_alloc: a recycled record on a list"));
            }
            ifma.ifma_ifidx.set(0);
            in6m.in6m_sin.set(SockaddrIn6::default());
            in6m.in6m_state.set(0);
            in6m.in6m_timer.set(0);
            Some(in6m)
        }
        None => {
            let in6m = MBox::leak(MBox::new(M_IPMADDR, M_NOWAIT, In6Multi::new())?);
            in6m.in6m_ifma.ifma_in6.set(Some(in6m));
            in6m.in6m_ifma
                .ifma_addr
                .set_words::<SockaddrIn6>(Some(in6m.in6m_sin.words()));
            Some(in6m)
        }
    }
}

/// `free(in6m, M_IPMADDR, sizeof(*in6m))`: the record onto the free list (the module's
/// deviations).
pub(crate) fn in6_multi_free(in6m: &'static In6Multi) {
    INM6_FREE.lock().insert_tail(in6m);
}

/// Forgets the free lists without touching their records, which an earlier test's memory
/// holds (`setup_real_memory`'s rule, testing.md).
#[cfg(any(test, feature = "doubles"))]
pub fn in6_var_test_reset() {
    *IA6_FREE.lock() = List::new();
    *INM6_FREE.lock() = List::new();
}

/// The free lists' lengths, for the tests.
#[cfg(test)]
pub(crate) fn free_counts() -> (usize, usize) {
    (
        IA6_FREE.lock().iter().count(),
        INM6_FREE.lock().iter().count(),
    )
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_records_sizes() {
        // vmstat -m's M_IFADDR and M_IPMADDR (D-4, the sizes taken and flagged). On LP64 the
        // C's struct in6_ifaddr is 256 bytes and struct in6_multi 80, LZ's #[repr(C)] mirrors
        // the same; these are 416 and 136. Of the growth, the generic parts are 80 and 32
        // (Ifaddr 160 for the C's 80, Ifmaddr 72 for the C's 40: the SaLinks, the counted
        // references and the list links are wider, and since N6.5a each carries an 8-byte
        // link to its IPv6 record, ifa_in6 or ifma_in6, which the IPv4 records pay too:
        // in_ifaddr 288 -> 296, in_multi 128 -> 136); the memberships' Mutexed is 64 for the
        // C's 8-byte LIST_HEAD.
        assert_eq!(size_of::<In6Ifaddr>(), 416);
        assert_eq!(size_of::<In6Multi>(), 136);
        assert_eq!(size_of::<crate::net::if_var::Ifaddr>(), 160);
        assert_eq!(size_of::<crate::net::if_var::Ifmaddr>(), 72);
        assert_eq!(size_of::<crate::netinet::in_var::InIfaddr>(), 296);
        assert_eq!(size_of::<crate::netinet::in_var::InMulti>(), 136);
        assert_eq!(size_of::<Mutexed<In6Memberships>>(), 64);
    }

    #[test]
    fn the_address_words_read_as_the_c_structure() {
        let w = Sin6Words::new();
        assert!(w.is_empty());
        let sin6 = SockaddrIn6 {
            sin6_len: SIN6_LEN,
            sin6_family: AF_INET6,
            sin6_port: 0x1234,
            sin6_flowinfo: 7,
            sin6_addr: In6Addr::new([0xfd, 0, 0, 0x77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
            sin6_scope_id: 3,
        };
        w.set(sin6);
        assert_eq!(w.get(), sin6);
        assert_eq!(w.addr(), sin6.sin6_addr);
        assert_eq!((w.len(), w.family()), (SIN6_LEN, AF_INET6));
    }

    #[test]
    fn the_lifetimes_are_stored_whole() {
        let c = In6AddrlifetimeCell::new();
        let lt = In6Addrlifetime {
            ia6t_expire: 100,
            ia6t_preferred: 50,
            ia6t_vltime: 3600,
            ia6t_pltime: 1800,
        };
        c.set(lt);
        assert_eq!(c.get(), lt);
        assert_eq!((c.vltime(), c.pltime()), (3600, 1800));
    }

    #[test]
    fn the_masked_comparison_is_the_cs() {
        let a = In6Addr::new([0x20, 1, 0xd, 0xb8, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1]);
        let b = In6Addr::new([0x20, 1, 0xd, 0xb8, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2]);
        let m64 = hal::sys::netinet6_in6::IN6MASK64;
        assert!(in6_are_masked_addr_equal(&a, &b, &m64));
        assert!(!in6_are_masked_addr_equal(
            &a,
            &b,
            &hal::sys::netinet6_in6::IN6MASK128
        ));
        assert_eq!(in6_are_scope_cmp(2, 5), -3);
        assert!(in6_are_scope_equal(2, 2));
    }
}
/* </TESTS> */
