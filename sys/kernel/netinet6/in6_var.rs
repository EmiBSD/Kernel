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
//! - The `ia_list` TAILQ entry is dropped: nothing links the records through it at the pin
//!   (no `in6_ifaddr` list head exists in netinet6). The record's `ia_list` member is the
//!   generic part's `ifa_list`.
//! - `struct in6_multi_mship` (a `malloc`'d list entry per membership) is not the kernel's:
//!   an address's memberships are an array of the records ([`In6Memberships`], the IPv4
//!   `imo_membership` shape, D-11), whose `M_IPMADDR` allocation `vmstat -m` counts instead
//!   of one entry each. `bsd`'s `ip6_setmoptions` keeps LZ's entries until N6.5c.
//!
//! ## Redesign
//! - LZ's `#[repr(C)]` records of `Cell`s cast from their generic part (`ifatoia6`,
//!   `ifmatoin6m`: pointer casts checked by the address family, one `unsafe` each, and three
//!   `unsafe impl Sync`) -> an address is held by a counted handle whose arm is its family
//!   (`kernel::net::if_var::IfaddrRef::In6`, N6.5e): [`ifatoia6`] and [`In6Ifaddr::of`] match
//!   the arm. A multicast record likewise (`kernel::net::if_var::IfmaddrRef::In6`, N6.5e's
//!   5e2): [`ifmatoin6m`] and [`In6Multi::of`].
//! - A multicast record is one `malloc(9)` block of `M_IPMADDR` (`hal::malloc::MArc`, N6.5e's
//!   5e2), made by [`in6_multi_alloc`] for its interface (its list cell keyed to that
//!   interface's `if_maddrlock`) and freed with its last handle (`in6_delmulti`'s `free`): the
//!   interface's list holds one, each membership one ([`In6Memberships`]' slots, nd6's proxy
//!   join none: its membership is counted in `in6m_refcnt` alone, as the C's discarded
//!   pointer). N6.5a's free list (and with it the high-water mark of `vmstat -m`'s
//!   `in_multi` row) and the generic part's back-link to the record are gone.
//! - An address is one `malloc(9)` block of `M_IFADDR` (`hal::malloc::MArc`, N6.5e), freed
//!   with its last handle (`ifafree`'s `free`): `vmstat -m`'s `ifaddr` row falls with the live
//!   addresses as the C's; the free list, its mutex and `in6_ifafree` are gone. The generic
//!   part's `ifa_list` and `ifa_tmplist` are the record's `ia_list` (a net cell) and
//!   `ia_tmplist` (keyed to `if_tmplist_lock`), `kernel::netinet::in_var`'s shape.
//! - The C's `struct sockaddr_in6` members -> [`Sin6Words`], the address in atomic words
//!   (`kernel::netinet::in_var`'s `SinWords` twin), read and written whole (`get`, `set`:
//!   LZ's `Cell` interface), by the address alone (`addr`, two word loads) or as the socket
//!   address the generic part names (`read`, through the handle). The C's NULL `ifa_dstaddr`
//!   of an address on neither a point-to-point nor a loopback interface is
//!   `ia_dstaddr_linked` false, set where `in6_update_ifa` sets the pointer (N6.5e; N6.5a's
//!   `SaLink::set_words`).
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
//! **Lock and spl transitions** against the C: an address's allocation and last release are
//! the C's `malloc` and `free`, and so are a multicast record's (N6.5e's 5e2: the free
//! list's mutex pair per allocation and per last release is gone); the memberships' mutex once
//! per join and once per purge (above the C's none: the net lock serves it).

use hal::intr::IPL_SOFTNET;
use hal::link::{Link, SaBytes, SaWords};
use hal::malloc::{M_NOWAIT, M_WAITOK, MArc, MArray};
use hal::mutex::Mutexed;
use hal::refcnt::Refcnt;
use hal::rwlock::Guarded;
use hal::sys::net_if::{IFF_LOOPBACK, IFF_POINTOPOINT};
use hal::sys::netinet6_in6::{In6Addr, SockaddrIn6};
use hal::sys::netinet6_in6_var::In6Addrlifetime;
use hal::sys::socket::AF_INET6;
use hal::sys::types::Time;

use crate::assert::kpanic;
use crate::net::if_::{IF_TMPLIST_LOCK, M_IFADDR};
use crate::net::if_var::{AtomicCell, Ifaddr, IfaddrRef, Ifmaddr, IfmaddrRef, Ifnet};
use crate::netinet::in_var::M_IPMADDR;

/// The size of a `struct sockaddr_in6`, its `sin6_len`.
pub const SIN6_LEN: u8 = 28;

/// The offset of `sin6_addr` in a `struct sockaddr_in6`: its words 1 and 2 hold it.
const SIN6_ADDR: usize = 8;

/// A `struct sockaddr_in6` the kernel owns: its bytes in atomic words (`hal::link::SaWords`).
/// Valid all zero.
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

    /// A copy of the socket address (`sa_len` bytes, cut at a `sockaddr_in6`): what the C's
    /// `ifa_addr`, `ifa_dstaddr` or `ifa_netmask` pointing here gives its readers.
    #[inline]
    pub fn read(&self) -> SaBytes {
        self.0.read(size_of::<SockaddrIn6>())
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
/// closes the gap. Each slot holds the membership's handle of its record (`in6_addmulti`'s,
/// which `in6_delmulti` takes back).
#[derive(Default)]
pub struct In6Memberships {
    /// The records' handles; `None` before the first join.
    v: Option<MArray<Option<MArc<In6Multi>>>>,
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
    pub fn iter(&self) -> impl Iterator<Item = &In6Multi> + '_ {
        self.v
            .as_deref()
            .into_iter()
            .flat_map(|v| v.iter().take(self.n).flatten().map(|m| &**m))
    }

    /// The C's `imm = malloc(sizeof(*imm), M_IPMADDR, M_NOWAIT)` before the join: room for one
    /// more membership, the array allocated (4 slots) or doubled when full, under the
    /// owner's mutex (malloc(9) allows `M_NOWAIT` there, and the old array is freed there
    /// once its handles moved); `Err(ENOBUFS)` when it cannot be had, before anything is
    /// joined (and nothing moved: `MArray::new` fills the array only once it has it).
    pub fn reserve(&mut self) -> Result<(), hal::sys::errno::Errno> {
        let cap = self.v.as_deref().map_or(0, <[_]>::len);
        if self.n < cap {
            return Ok(());
        }
        let old = self.v.as_deref_mut().unwrap_or(&mut []);
        let Some(nv) = MArray::new(M_IPMADDR, M_NOWAIT, (cap * 2).max(4), |i| {
            old.get_mut(i).and_then(Option::take)
        }) else {
            return Err(hal::sys::errno::Errno::ENOBUFS);
        };
        // The old array, emptied, freed.
        self.v = Some(nv);
        Ok(())
    }

    /// `LIST_INSERT_HEAD(&memberships, imm, i6mm_chain)` of a membership joined after
    /// [`reserve`](Self::reserve) made its room: `in6m` recorded in it. Two joiners take
    /// room, each on its own set: the `in6_update_ifa` call that made an address joins its
    /// groups (the C's `hostIsNew`), one after another under the net lock, and
    /// `ip6_setmoptions`' `IPV6_JOIN_GROUP` joins a socket's group under the socket's lock
    /// (`kernel::netinet6::ip6_output`'s `join_group`, the C's `in6_joingroup` and
    /// `LIST_INSERT_HEAD` of `im6o_memberships`). Between a joiner's reserve and its push
    /// nothing else takes room in its set (a purge or a leave only frees it), so the room is
    /// still there and the push cannot fail. Were it gone, the push reserves again (the
    /// handle given back if that fails, the C's `ENOBUFS`, the caller then leaving the group
    /// with it): a fallback no caller reaches.
    pub fn push(&mut self, in6m: MArc<In6Multi>) -> Result<(), MArc<In6Multi>> {
        if self.reserve().is_err() {
            return Err(in6m);
        }
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
    /// (`LIST_INSERT_HEAD`'s): its record's handle, its slot emptied.
    pub fn pop_last(&mut self) -> Option<MArc<In6Multi>> {
        let n = self.n.checked_sub(1)?;
        let last = self.v.as_deref_mut()?.get_mut(n)?.take();
        self.n = n;
        last
    }

    /// `LIST_FOREACH` from the list's head, the newest first (`LIST_INSERT_HEAD`'s order),
    /// to the first membership `matches` accepts, then `LIST_REMOVE` of it: its record's
    /// handle, the newer ones moved down one slot (the gap closed, the order kept). O(n) as
    /// the C's walk.
    pub fn take_newest(
        &mut self,
        mut matches: impl FnMut(&In6Multi) -> bool,
    ) -> Option<MArc<In6Multi>> {
        let n = self.n;
        let v = self.v.as_deref_mut()?;
        let i = (0..n)
            .rev()
            .find(|&i| v[i].as_deref().is_some_and(&mut matches))?;
        let found = v[i].take();
        // The emptied slot moves to the end of the ones in use.
        v[i..n].rotate_left(1);
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
    /// \[I\] Whether the generic part's `ifa_dstaddr` names `ia_dstaddr` (the C's pointer:
    /// NULL off a point-to-point or loopback interface).
    pub(crate) ia_dstaddr_linked: bool,
    /// \[N\] `ia_ifa.ifa_list`: the next address of the interface's list (a net cell,
    /// `kernel::net::if_var::IfaList`'s chain).
    pub(crate) ia_list: Guarded<Option<IfaddrRef>>,
    /// \[T\] `ia_ifa.ifa_tmplist`: the next address of `ifconf`'s temporary list.
    pub(crate) ia_tmplist: Guarded<Option<IfaddrRef>>,
}

impl In6Ifaddr {
    /// A record every member of which is zero (`M_ZERO`), on no list.
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
            ia_dstaddr_linked: false,
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

    /// `ifatoia6(ifa)` for an address that may not be an IPv6 one: `None` for another
    /// family's (the handle's arm).
    #[inline]
    pub fn of(ifa: &IfaddrRef) -> Option<&In6Ifaddr> {
        match ifa {
            IfaddrRef::In6(ia6) => Some(ia6),
            _ => None,
        }
    }
}

/// `struct in6_multi`: IPv6 multicast address structure. Held by counted handles (`MArc`,
/// the module's redesign).
pub struct In6Multi {
    /// `in6m_ifma`: protocol-independent info.
    pub in6m_ifma: Ifmaddr,
    /// \[I\] `in6m_sin`: IPv6 multicast address.
    pub in6m_sin: Sin6Words,
    /// \[m\] `in6m_state`: state of membership.
    pub in6m_state: AtomicCell<u32>,
    /// \[m\] `in6m_timer`: MLD6 membership report.
    pub in6m_timer: AtomicCell<u32>,
}

impl In6Multi {
    /// A record of `ifp` every member of which is zero (`M_ZERO`), its list cell keyed to
    /// `ifp`'s `if_maddrlock`.
    fn new(ifp: &Ifnet) -> Self {
        Self {
            in6m_ifma: Ifmaddr::new(ifp),
            in6m_sin: Sin6Words::new(),
            in6m_state: AtomicCell::new(),
            in6m_timer: AtomicCell::new(),
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

    /// `ifmatoin6m(ifma)` for a record that may not be an IPv6 one: its handle, `None` for
    /// another family's (the handle's arm).
    #[inline]
    pub fn of(ifma: &IfmaddrRef) -> Option<&MArc<In6Multi>> {
        match ifma {
            IfmaddrRef::In6(in6m) => Some(in6m),
            IfmaddrRef::In(_) => None,
        }
    }
}

/// `ifatoia6(ifa)`: the IPv6 address an `AF_INET6` interface address is the first member of;
/// `panic(9)` for another family's (where the C would cast it).
#[inline]
pub fn ifatoia6(ifa: &IfaddrRef) -> &In6Ifaddr {
    match In6Ifaddr::of(ifa) {
        Some(ia6) => ia6,
        None => kpanic(format_args!("ifatoia6: not an inet6 address")),
    }
}

/// `ifmatoin6m(ifma)`: the IPv6 multicast record an `AF_INET6` multicast address is the
/// first member of; `panic(9)` for another family's (where the C would cast it).
#[inline]
pub fn ifmatoin6m(ifma: &IfmaddrRef) -> &In6Multi {
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
pub fn ifa_in6(x: &IfaddrRef) -> In6Addr {
    ifatoia6(x).ia_addr.addr()
}

/// `IFA_DSTIN6(x)`: the IPv6 destination address of an `AF_INET6` interface address.
#[inline]
pub fn ifa_dstin6(x: &IfaddrRef) -> In6Addr {
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
/// a new address of `ifp`: its address's family and length, its time stamp, its interface,
/// and the generic part's `ifa_dstaddr` only on a point-to-point or loopback interface ("some
/// functions expect that ifa_dstaddr is not NULL for p2p interfaces"). The handle returned is
/// the creation reference (`refcnt_init`), which `ifa_add` takes for the interface's list;
/// the record is freed with the last handle (`ifafree`'s `free(ifa, M_IFADDR, 0)`).
pub fn in6_ifaddr_alloc(ifp: &'static Ifnet, now: Time) -> MArc<In6Ifaddr> {
    // The C's `M_ZERO`, then its stores, made on the value `MArc::new` writes whole.
    let mut ia6 = In6Ifaddr::new();
    // Initialize the address and masks, and put time stamp
    ia6.ia_addr.set(SockaddrIn6 {
        sin6_len: SIN6_LEN,
        sin6_family: AF_INET6,
        ..SockaddrIn6::default()
    });
    ia6.ia6_updatetime.set(now);
    // XXX: some functions expect that ifa_dstaddr is not NULL for p2p interfaces.
    ia6.ia_dstaddr_linked = ifp.if_flags.get() & (IFF_POINTOPOINT | IFF_LOOPBACK) != 0;
    ia6.ia_ifp().set(Some(ifp));
    match MArc::new(M_IFADDR, M_WAITOK, ia6) {
        Some(ia6) => ia6,
        None => kpanic(format_args!("in6_ifaddr: no memory")),
    }
}

/// `malloc(sizeof(*in6m), M_IPMADDR, M_NOWAIT | M_ZERO)` (`in6_addmulti`'s new record) for
/// a group of `ifp`: one handle, the block freed with the last (`free(in6m, M_IPMADDR,
/// sizeof(*in6m))`); `None` when the allocator has none (the C's `ENOBUFS`).
pub fn in6_multi_alloc(ifp: &Ifnet) -> Option<MArc<In6Multi>> {
    MArc::new(M_IPMADDR, M_NOWAIT, In6Multi::new(ifp))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_records_sizes() {
        use crate::net::if_var::{Ifaddr, IfaddrLinked, IfaddrRef, Ifmaddr};
        use crate::netinet::in_var::{InIfaddr, InMulti};
        // vmstat -m's M_IFADDR and M_IPMADDR and the rtentry pool's item (D-6 of N6.5e, the
        // sizes taken and flagged, ROADMAP's N12 row). On LP64 the C's struct in_ifaddr is 288
        // bytes, struct in6_ifaddr 256, struct ifaddr 80, struct in6_multi 80, a pointer 8.
        // Before N6.5e: In6Ifaddr 416, InIfaddr 296, Ifaddr 160 (three 24-byte SaLinks, the
        // list links, the count, two back-links), Ifmaddr 72, the multicast records 136.
        // N6.5e: an address is an MArc block (16 bytes of count and type before it); the
        // generic part keeps the interface, the flags and the metric (16); each family record
        // holds the chain's two cells (Guarded<Option<IfaddrRef>>: a 16-byte key and the
        // 16-byte handle) where the C has two TAILQ entries (32), and its addresses' words
        // with no link to them; IPv6's NULL ifa_dstaddr is one bool. `rt_ifa` grows from a
        // pointer to a cell of the handle: +24 on the route. Ifmaddr loses its unread
        // ifma_addr (24). 5e2: a multicast record is an MArc block of M_IPMADDR too; its
        // generic part holds the chain's cell (32, a TAILQ entry's 16 in the C), the count and
        // the index, no back-links and no free list's entry: InMulti 112 -> 80 (block 96),
        // In6Multi 112 -> 80 (block 96, the C's in6_multi 80); InIfaddr's `ia_allhosts` is a
        // `ReadMostly` of the membership's handle (16 where the Link was 8): 192 -> 200 (block
        // 216, the same 256 bucket).
        let sizes = [
            size_of::<Ifaddr>(),
            size_of::<IfaddrRef>(),
            size_of::<Option<IfaddrRef>>(),
            size_of::<IfaddrLinked>(),
            size_of::<InIfaddr>(),
            size_of::<In6Ifaddr>(),
            size_of::<Ifmaddr>(),
            size_of::<InMulti>(),
            size_of::<In6Multi>(),
            size_of::<crate::net::route::Rtentry>(),
        ];
        assert_eq!(sizes[..3], [16, 16, 16]);
        assert_eq!(sizes[3], 72, "IfaddrLinked");
        assert_eq!((sizes[4], sizes[5]), (200, 312), "InIfaddr, In6Ifaddr");
        assert_eq!(sizes[6..9], [40, 80, 80], "Ifmaddr, InMulti, In6Multi");
        assert_eq!(
            sizes[9], 328,
            "Rtentry (304 before: rt_ifa was an 8-byte Link)"
        );
        assert_eq!(size_of::<hal::rwlock::Guarded<Option<IfaddrRef>>>(), 32);
        assert_eq!(size_of::<hal::malloc::MArc<InIfaddr>>(), 8);
        assert_eq!(size_of::<crate::net::if_var::IfmaddrRef>(), 16);
        assert_eq!(size_of::<Option<crate::net::if_var::IfmaddrRef>>(), 16);
        assert_eq!(size_of::<Mutexed<In6Memberships>>(), 64);
    }

    /// `In6Memberships` over the memberships' handles (N6.5e's 5e2): a join past the array's
    /// four slots doubles it, moving the handles (none lost, none added); the newest
    /// matching one is taken with the gap closed in the order of the joins; the rest come off
    /// the newest first, and every handle is the test's again at the end.
    #[test]
    fn memberships_grow_by_moving_their_handles() {
        let _g = crate::net::if_::tests::setup();
        static D: crate::net::if_::TestDriver = crate::net::if_::TestDriver::new();
        let ifp = crate::net::if_::test_ifnet_with(b"tms0", &D);
        let recs: std::vec::Vec<MArc<In6Multi>> = (0..6u8)
            .map(|n| {
                let m = in6_multi_alloc(ifp).expect("a record");
                let mut a = In6Addr::new([0xff, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                a.s6_addr[15] = n;
                m.in6m_sin.set(SockaddrIn6::with_addr(a));
                m
            })
            .collect();
        let last = |m: &In6Multi| m.in6m_addr().s6_addr[15];
        let mut set = In6Memberships::new();
        for m in &recs {
            set.reserve().expect("room");
            assert!(set.push(m.clone()).is_ok());
        }
        assert_eq!(set.len(), 6);
        assert_eq!(
            set.iter().map(last).collect::<std::vec::Vec<_>>(),
            [0, 1, 2, 3, 4, 5]
        );
        assert!(
            recs.iter().all(|m| MArc::count(m) == 2),
            "moved, not copied"
        );

        let two = set.take_newest(|m| last(m) == 2).expect("member 2");
        assert!(MArc::ptr_eq(&two, &recs[2]));
        assert_eq!(
            set.iter().map(last).collect::<std::vec::Vec<_>>(),
            [0, 1, 3, 4, 5]
        );
        assert!(set.take_newest(|m| last(m) == 2).is_none());
        drop(two);
        let mut order = std::vec::Vec::new();
        while let Some(m) = set.pop_last() {
            order.push(last(&m));
        }
        assert_eq!(order, [5, 4, 3, 1, 0], "the newest first");
        assert!(set.is_empty());
        assert!(recs.iter().all(|m| MArc::count(m) == 1));
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
