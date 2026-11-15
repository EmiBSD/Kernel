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
 * Copyright (c) 2012-2013 Henning Brauer <henning@openbsd.org>
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)if.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The kernel's network interface: `<net/if_var.h>`, `struct ifnet`, its addresses and the
//! interface's driver (`NetDriver`), cloner (`Cloner`) and softnet stack (`Netstack`).
//!
//! Upstream: sys/net/if_var.h @ 3ce1f3f79392
//! LZ: sys/net/if_var.rs@3c62ede68802
//!
//! Each interface accepts output datagrams of a specified maximum length, and provides higher
//! level routines with input datagrams received from its medium. Output occurs when the
//! routine `if_output` is called, with four parameters: `(*ifp->if_output)(ifp, m, dst, rt)`,
//! where `m` is the mbuf chain to be sent and `dst` is the destination address. The output
//! routine encapsulates the supplied datagram if necessary, and then transmits it on its
//! medium. On input, each interface unwraps the data received by it, and either places it on
//! the input queue of an internetwork datagram routine and posts the associated software
//! interrupt, or passes the datagram to a raw packet input routine. The routines live in
//! `net/if.c` ([`super::if_`]) and `net/route.c`.
//!
//! The locks named in the members' docs are the C's: \[I\] immutable after creation, \[d\]
//! the driver's, \[c\] ioctl or routing socket context (kernel lock), \[K\] kernel lock,
//! \[N\] net lock, \[T\] `if_tmplist_lock`, \[m\] `if_maddrlock`.
//!
//! ## Deviations
//! - `if_counters` (`struct cpumem *` of `counters_alloc(ifc_ncounters)`) is one array of
//!   atomics per interface, inside the interface, that every CPU bumps (LZ's): `hal`'s
//!   `percpu::Counters` holds `MAXCPUS` slots by value (32 KiB for an interface's twelve
//!   counters), which an interface embedded in a softc cannot carry; the cost is contention,
//!   not correctness. `if_counters_alloc` turns it on, `if_counters_free` off; the sums are
//!   the same.
//! - `if_carp_ptr`'s union is two members side by side (`if_carp`, the first pointer of
//!   carp(4)'s list, not configured, and `if_carpdevidx`), LZ's.
//! - One member the C has not: [`Ifnet::if_arpcom`], set by `ether_ifattach`, which names the
//!   `struct arpcom` an Ethernet interface is the first member of, so that the C's `(struct
//!   arpcom *)ifp` cast is one load (`kernel::net::if_ethersubr::arpcom_of`; N6.3c2, where
//!   LZ's checked cast and its mark `is_arpcom` went).
//! - `struct niqueue` and the `niq_*` macros are the kernel's since N6.4b ([`Niqueue`], over
//!   `kernel::net::mbuf::MbufQueue`; ARP's `arpinq` first); `bsd` keeps LZ's for its own
//!   queues (`ipintrq`, `ip6intrq`) until they move (N6.4c2, N6.5).
//!
//! ## Redesign
//! - LZ's `struct ifnet` of `Cell`s behind `unsafe impl Sync`, embedded in `bsd`'s zeroed
//!   softcs and lent as `&'static Ifnet` -> the kernel's [`Ifnet`], still embedded by value
//!   in the zeroed softcs (the embed idiom, docs/IDIOMS.md) and still lent `&'static` by
//!   `bsd`: true while the driver's memory lives, which `if_remove`'s protocol makes so (the
//!   list unlinked under the net lock, the index map's `smr::barrier`, the softnet barriers,
//!   the reference count finalized; docs/SCAFFOLDING.md, removed by N7's `NetDevice`). Every
//!   member outside [`Ifnet::rt`] is valid all zero, each by the `hal` test that proves its
//!   type so (`hal::link`, `hal::list`, `hal::once`, `hal::mutex`'s `an_all_zero_*`) or as an
//!   atomic; what is not (the queues' mutexes and tasks, the reference count's protocol, the
//!   timeout, the hooks' lists, the driver's table) is built by `if_attach_common` into the
//!   set-once [`IfRt`] (D-N6.3a-1). The zero-validity of the members:
//!
//!   | member | type | all zero is |
//!   |---|---|---|
//!   | `if_softc`, `if_mcast`, `if_mcast6`, `if_carp`, `if_pf_kif` | `PtrWord` | NULL (an `AtomicPtr`) |
//!   | `if_nd` | [`NdIfinfoCell`](crate::netinet6::nd6::NdIfinfoCell) | `nd6_ifattach` not run (two atomics) |
//!   | `if_bpf` | [`BpfTap`](crate::net::bpf::BpfTap) (`hal::smr::SmrArcWord`) | no tap (an `AtomicPtr`: `hal::smr`'s `an_all_zero_counted_word_is_null`) |
//!   | `if_xname`, `if_description` | [`AtomicBytes`] | the empty string |
//!   | `if_index`, `if_flags`, `if_xflags`, `if_type`, ..., `if_snd_maxlen` | [`AtomicCell`] | 0 |
//!   | `if_lastchange` | [`IfLastchange`] | `{ 0, 0 }` |
//!   | `if_data_counters`, `if_counters` | [`AtomicCell`]s, [`IfCountersSlot`] | 0, not allocated |
//!   | `if_list`, `if_tmplist` | `hal::list::ListEntry` | on no list (`an_all_zero_list_is_empty_and_an_all_zero_entry_unlinked`) |
//!   | `if_maddrlist` | `hal::list::List` | empty (the same test) |
//!   | `if_sadl` | `ReadMostly<Option<MBox<SockaddrDl>>>` | no reader, `None` (`hal::slot`'s `an_all_zero_cell_of_no_box_or_handle_holds_none`) |
//!   | `if_groups` | `ReadMostly<IfgSet>` (an `Option<MArc<MArray<..>>>`) | no reader, no group (the same test) |
//!   | `if_driver`, `if_link`, `rt` | `hal::once::OnceStatic` | empty (`an_all_zero_cell_is_an_empty_one`) |
//!   | `if_arpcom` | `hal::link::Link` | no `arpcom` (an `AtomicPtr`) |
//!   | `detached` | an atomic | attached |
//!
//! - The members `bsd` reads or writes under its detached net lock (the drivers' `*_start`
//!   test `IFF_RUNNING` with no lock at all, as the C's) are atomics with LZ's `get()`/`set()`
//!   ([`AtomicCell`]), links or lists, never a `Guarded` keyed to the net lock, whose witness
//!   a detached holding cannot make (`hal::netlock`; D-N6.3a-2): the 51 files that read
//!   `if_flags` compile unchanged.
//! - LZ's nine `Cell<Option<fn>>` hooks (`if_input`, `if_output`, `if_start`, `if_ioctl`,
//!   ...) and `if_rtrequest` -> [`NetDriver`], the interface's driver: one object per
//!   interface, `bsd`'s `LegacyNetDriver` over the table its drivers keep filling (registered
//!   before attach in [`Ifnet::if_driver`], D-N6.3a-8), the C's NULL defaults as the trait's
//!   default methods. `if_output` and `if_ll_output` stay in `bsd`'s table, out of the trait,
//!   with their callers, until routing moves (N6.3b; D-N6.3a-9): their arguments are `bsd`'s
//!   route and socket address, and the kernel calls neither.
//! - `if_ifqs`/`if_iqs` (a pointer to an array of pointers, the first slot the embedded
//!   queue's own one-element map) -> the queues in [`IfRt`]: `if_snd`/`if_rcv` and the
//!   `MBox`ed rest that `if_attach_queues`/`if_attach_iqueues` add, so [`Ifnet::ifq`] is one
//!   branch where the C loads `if_ifqs[i]`.
//! - `if_sadl` (a `malloc`ed `sockaddr_dl` the C rewrites in place under the net lock while
//!   others read it) -> one owned box in a `ReadMostly` cell: readers borrow it, `if_setlladdr`
//!   writes it alone (D-N6.3a-4; one compare-and-swap per read on the ARP/ND and routing
//!   socket paths, never per data packet, N12's row).
//! - `struct ifaddr`, which each protocol's address embeds first and casts back to
//!   (`ifatoia`), freed by `ifafree` at its last reference (`free(ifa, M_IFADDR, 0)`) ->
//!   (N6.5e) [`IfaddrRef`], a counted handle with one arm per family: `In`, `In6` (an
//!   `MArc` of the protocol's record, `kernel::netinet::in_var`'s and
//!   `kernel::netinet6::in6_var`'s, one `malloc(9)` block freed with its last handle) and
//!   `Linked` (enc(4)'s [`IfaddrLinked`], embedded in its softc, counted by its own
//!   `ifa_refcnt`). It derefs to the generic part ([`Ifaddr`]: the interface, the flags, the
//!   metric); the family is the arm, so `ifatoia`'s cast is a match and no record links to
//!   itself. `ifaref` is a clone, `ifafree` a drop: the handles' count is `ifa_refcnt`
//!   (D-5). The three addresses are read through the handle ([`IfaddrRef::ifa_addr`]): the
//!   family record's own words, or a linked address's `SaLink`.
//! - `if_addrlist` (a `TAILQ` of the records, `[N]`) -> [`IfaList`] in [`IfRt`]: a chain of
//!   handles in net cells (`hal::rwlock::Guarded::new_net`), the head's and each record's
//!   `ia_list`, read under a `hal::netlock::NetReader` and changed under the `NetWriter`
//!   (`NET_ASSERT_LOCKED()` and `NET_ASSERT_LOCKED_EXCLUSIVE()`, made in every build). The list
//!   holds the creation reference of each address; a walk lends `&'r IfaddrRef`s for the
//!   reader's life, counting nothing (the C's uncounted pointers). It is not in [`Ifnet`]: a
//!   `Guarded` is not valid all zero, and an interface is used only once attached (the C's
//!   `TAILQ_INIT` is `if_attach_common`'s too). Inserting at the tail and removing walk the
//!   chain (O(n), D-4; the C's `TAILQ` is O(1)): `ifa_add` and `ifa_del` are control paths.
//! - `ifma_addr` (a pointer to the multicast record's own `sockaddr`, which the C reads for
//!   the family alone) is dropped (N6.5e): the family test is the record's back-link
//!   (`Ifmaddr::ifma_in`, `ifma_in6`), and the record's address its own words.
//! - `struct netstack` is owned by its softnet thread and lent `&mut` ([`Netstack`]), where
//!   LZ's `&Netstack` with `Cell` lists needed `unsafe impl Sync`; `ns_route` (`route.c`'s
//!   `struct route`) is not cached per softnet until N6.3b: `bsd`'s `ip_input`/`ip6_input`
//!   use a route of the call (D-N6.3a-10's `SOFTNET_ROUTES` dropped: `bsd`'s `Route` is not
//!   `Sync`).
//! - `struct if_clone` with its two function pointers -> [`Cloner`].
//! - The C's one `if_output`/`if_input`/`if_rtrequest` slot, which `ether_ifattach` fills
//!   when the driver left it NULL (if_ethersubr.c:716-719) and the loopback fills itself
//!   (if_loop.c:164-168) -> [`LinkLayer`] in [`Ifnet::if_link`], behind [`NetDriver`]'s
//!   default `output`, `input` and `rtrequest` ([`link_output`], [`link_input`],
//!   [`link_rtrequest`], N6.3c2, D-N6.3c-5): a driver's own handler still wins, the link
//!   layer's is the default, one slot as in the C; a call costs one `OnceStatic` load and one
//!   `&dyn` call more than the C's pointer call. `if_ll_output` is dropped (D-N6.3c-6: only
//!   `#ifdef MPLS` writes it, nothing reads it).

use core::any::Any;
use core::ops::Deref;
use core::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI64, AtomicU8, AtomicU16, AtomicU32, AtomicU64,
    Ordering,
};

use hal::intr::IPL_NET;
use hal::link::{Link, PtrWord, SaBytes, SaLink};
use hal::list::{List, ListEntry};
use hal::malloc::{MArc, MArray, MBox};
use hal::mutex::Mutexed;
use hal::netlock::NetHeld;
use hal::once::OnceStatic;
use hal::pinlist::PinList;
use hal::refcnt::Refcnt;
use hal::rwlock::{Exclusive, Guarded, Witness};
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::net_if::{IFDESCRSIZE, IFNAMSIZ, IfRxring, SockaddrDl};

use crate::assert::kpanic;
use crate::net::ifq::{Ifiqueue, Ifqueue};
use crate::net::mbuf::{
    Mbuf, MbufBox, MbufList, MbufQueue, mq_delist, mq_dequeue, mq_drops, mq_len,
};
use crate::netinet::in_var::InIfaddr;
use crate::netinet6::in6_var::In6Ifaddr;
use crate::prf::Str;
use crate::task::{Task, TaskChain};
use crate::timeout::Timeout;

/// `IFNET_SLOWTIMO`: granularity is 1 second.
pub const IFNET_SLOWTIMO: i32 = 1;

/// `IF_TXMIT_MIN`.
pub const IF_TXMIT_MIN: u32 = 1;
/// `IF_TXMIT_DEFAULT`.
pub const IF_TXMIT_DEFAULT: u32 = 16;

// Default interface priorities.

/// `IF_WIRED_DEFAULT_PRIORITY`.
pub const IF_WIRED_DEFAULT_PRIORITY: u8 = 0;
/// `IF_WIRELESS_DEFAULT_PRIORITY`.
pub const IF_WIRELESS_DEFAULT_PRIORITY: u8 = 4;
/// `IF_WWAN_DEFAULT_PRIORITY`.
pub const IF_WWAN_DEFAULT_PRIORITY: u8 = 6;
/// `IF_CARP_DEFAULT_PRIORITY`.
pub const IF_CARP_DEFAULT_PRIORITY: u8 = 15;

/// `IFA_ROUTE`: auto-magically installed route.
pub const IFA_ROUTE: u32 = 0x01;

/// The seal of [`AtomicWord`]: only this module's integers.
mod sealed {
    /// Sealed.
    pub trait Sealed {}
}

/// An integer an [`AtomicCell`] holds, with its atomic (sealed: `u8`, `u16`, `i16`, `i32`,
/// `u32`, `i64`, `u64`).
pub trait AtomicWord: sealed::Sealed + Copy + 'static {
    /// The atomic that holds it.
    type Atom: Send + Sync;
    /// The atomic at 0.
    const ZERO: Self::Atom;
    /// A relaxed load.
    fn load(a: &Self::Atom) -> Self;
    /// A relaxed store.
    fn store(a: &Self::Atom, v: Self);
}

/// Implements [`AtomicWord`] for `$t` over `$a`.
macro_rules! atomic_word {
    ($($t:ty => $a:ty;)*) => {
        $(
            impl sealed::Sealed for $t {}

            impl AtomicWord for $t {
                type Atom = $a;
                const ZERO: $a = <$a>::new(0);
                #[inline]
                fn load(a: &$a) -> $t {
                    a.load(Ordering::Relaxed)
                }
                #[inline]
                fn store(a: &$a, v: $t) {
                    a.store(v, Ordering::Relaxed);
                }
            }
        )*
    };
}

atomic_word! {
    u8 => AtomicU8;
    u16 => AtomicU16;
    i16 => AtomicI16;
    i32 => AtomicI32;
    u32 => AtomicU32;
    i64 => AtomicI64;
    u64 => AtomicU64;
}

/// A scalar member the C writes under a lock and reads without one (`if_flags`, `if_mtu`,
/// `if_link_state`, ...): a relaxed atomic with LZ's `Cell` interface, `get` and `set` (a
/// relaxed load is a plain load on arm64 and amd64). Valid all zero.
#[repr(transparent)]
pub struct AtomicCell<T: AtomicWord>(T::Atom);

impl<T: AtomicWord> AtomicCell<T> {
    /// 0.
    pub const fn new() -> Self {
        Self(T::ZERO)
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> T {
        T::load(&self.0)
    }

    /// Stores `v`.
    #[inline]
    pub fn set(&self, v: T) {
        T::store(&self.0, v);
    }
}

impl<T: AtomicWord> Default for AtomicCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: AtomicWord + core::fmt::Debug> core::fmt::Debug for AtomicCell<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.get().fmt(f)
    }
}

/// A fixed string member (`if_xname`, `if_description`): `N` relaxed atomic bytes with LZ's
/// `Cell<[u8; N]>` interface (written whole by its one writer, the driver before attach or
/// the ioctl under the kernel lock; read whole). Valid all zero (the empty string).
pub struct AtomicBytes<const N: usize>([AtomicU8; N]);

impl<const N: usize> AtomicBytes<N> {
    /// All zero.
    pub const fn new() -> Self {
        Self([const { AtomicU8::new(0) }; N])
    }

    /// The bytes.
    pub fn get(&self) -> [u8; N] {
        core::array::from_fn(|i| self.0[i].load(Ordering::Relaxed))
    }

    /// Stores `v`.
    pub fn set(&self, v: [u8; N]) {
        for (c, b) in self.0.iter().zip(v) {
            c.store(b, Ordering::Relaxed);
        }
    }

    /// The bytes up to the first NUL (the C string).
    pub fn with_str<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        let b = self.get();
        let n = b.iter().position(|&c| c == 0).unwrap_or(N);
        f(&b[..n])
    }
}

impl<const N: usize> Default for AtomicBytes<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// `if_xname`: the interface's external name (name + unit), NUL-terminated.
pub type IfName = AtomicBytes<IFNAMSIZ>;

/// `if_description`.
pub type IfDescr = AtomicBytes<IFDESCRSIZE>;

/// `if_lastchange` (`struct timeval`): seconds and microseconds, each a relaxed atomic,
/// written by the link state and up/down paths, read by `if_getdata` (a torn read is the C's
/// own race, which copies the two words with no lock). Valid all zero.
pub struct IfLastchange {
    /// `tv_sec`.
    sec: AtomicI64,
    /// `tv_usec`.
    usec: AtomicI64,
}

impl IfLastchange {
    /// `{ 0, 0 }`.
    pub const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            usec: AtomicI64::new(0),
        }
    }

    /// `(tv_sec, tv_usec)`.
    pub fn get(&self) -> (i64, i64) {
        (
            self.sec.load(Ordering::Relaxed),
            self.usec.load(Ordering::Relaxed),
        )
    }

    /// Stores `(tv_sec, tv_usec)`.
    pub fn set(&self, sec: i64, usec: i64) {
        self.sec.store(sec, Ordering::Relaxed);
        self.usec.store(usec, Ordering::Relaxed);
    }
}

impl Default for IfLastchange {
    fn default() -> Self {
        Self::new()
    }
}

/// `enum if_counters`: the interface statistics, indices into `if_data_counters` and
/// `if_counters`.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IfCounters {
    /// `ifc_ipackets`: packets received on interface.
    IfcIpackets,
    /// `ifc_ierrors`: input errors on interface.
    IfcIerrors,
    /// `ifc_opackets`: packets sent on interface.
    IfcOpackets,
    /// `ifc_oerrors`: output errors on interface.
    IfcOerrors,
    /// `ifc_collisions`: collisions on csma interfaces.
    IfcCollisions,
    /// `ifc_ibytes`: total number of octets received.
    IfcIbytes,
    /// `ifc_obytes`: total number of octets sent.
    IfcObytes,
    /// `ifc_imcasts`: packets received via multicast.
    IfcImcasts,
    /// `ifc_omcasts`: packets sent via multicast.
    IfcOmcasts,
    /// `ifc_iqdrops`: dropped on input, this interface.
    IfcIqdrops,
    /// `ifc_oqdrops`: dropped on output, this interface.
    IfcOqdrops,
    /// `ifc_noproto`: destined for unsupported protocol.
    IfcNoproto,
    /// `ifc_ncounters`.
    IfcNcounters,
}

/// `ifc_ncounters`.
pub const IFC_NCOUNTERS: usize = IfCounters::IfcNcounters as usize;

/// What `counters_alloc(ifc_ncounters)` returns here: one counter per `if_counters`, bumped
/// with relaxed atomics (the module's deviations).
pub type IfCounterArray = [AtomicU64; IFC_NCOUNTERS];

/// `if_counters`: the interface's stack counters, off (`NULL`) until `if_counters_alloc`.
/// Valid all zero (off, every counter 0).
pub struct IfCountersSlot {
    /// Whether `if_counters_alloc` turned them on.
    on: AtomicBool,
    /// The counters.
    c: IfCounterArray,
}

impl IfCountersSlot {
    /// Off.
    pub const fn new() -> Self {
        Self {
            on: AtomicBool::new(false),
            c: [const { AtomicU64::new(0) }; IFC_NCOUNTERS],
        }
    }

    /// `ifp->if_counters`: the counters, `None` (`NULL`) while off.
    #[inline]
    pub fn get(&self) -> Option<&IfCounterArray> {
        self.on.load(Ordering::Acquire).then_some(&self.c)
    }

    /// `if_counters_alloc`'s and `if_counters_free`'s work: turned on at 0 (`counters_alloc`
    /// zeroes), or off.
    pub(crate) fn turn(&self, on: bool) {
        if on {
            for c in &self.c {
                c.store(0, Ordering::Relaxed);
            }
        }
        self.on.store(on, Ordering::Release);
    }
}

impl Default for IfCountersSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// `counters_inc(c, i)` over an interface's counters.
#[inline]
pub fn counters_inc(c: &IfCounterArray, i: IfCounters) {
    c[i as usize].fetch_add(1, Ordering::Relaxed);
}

/// `counters_pkt(c, pc, bc, v)`: one more packet in `pc`, `v` more bytes in `bc`.
#[inline]
pub fn counters_pkt(c: &IfCounterArray, pc: IfCounters, bc: IfCounters, v: u64) {
    c[pc as usize].fetch_add(1, Ordering::Relaxed);
    c[bc as usize].fetch_add(v, Ordering::Relaxed);
}

/// `struct netstack`: the packet lists a softnet thread works through (`if_input_process`).
/// Owned by its softnet thread, lent `&mut` to the input functions it calls (the module's
/// redesign); `ns_route`, `route.c`'s `struct route`, is the kernel's `Route` since N6.3b1
/// (`bsd`'s `ip_input`/`ip6_input` use it from N6.3b2).
pub struct Netstack {
    /// `ns_input`: packets to give to an interface's `if_input` again (`if_vinput`).
    pub ns_input: MbufList,
    /// `ns_proto`: packets for a protocol's input function (`if_input_proto`).
    pub ns_proto: MbufList,
    /// `ns_tcp_ml`: TCP segments gathered for `tcp_input_mlist`.
    pub ns_tcp_ml: MbufList,
    /// `ns_tcp6_ml`: the same for IPv6.
    pub ns_tcp6_ml: MbufList,
    /// The softnet this stack is.
    pub ns_idx: u32,
    /// `ns_route`: the softnet's route cache (`route.c`'s `struct route`, N6.3b1, D-N6.3b-7).
    pub ns_route: crate::net::route::Route,
}

impl Netstack {
    /// Four empty lists, of softnet `idx`.
    pub const fn new(idx: u32) -> Self {
        Self {
            ns_input: MbufList::new(),
            ns_proto: MbufList::new(),
            ns_tcp_ml: MbufList::new(),
            ns_tcp6_ml: MbufList::new(),
            ns_idx: idx,
            ns_route: crate::net::route::Route::new(),
        }
    }
}

/// `struct niqueue`: a network stack input queue, the packets a protocol's soft interrupt
/// takes (`if_netisr`'s `NETISR_*` bit `ni_isr`).
pub struct Niqueue {
    /// `ni_q`.
    pub ni_q: MbufQueue,
    /// `ni_isr`: the `NETISR_*` bit to schedule.
    pub ni_isr: i32,
}

impl Niqueue {
    /// `NIQUEUE_INITIALIZER(len, isr)`.
    pub const fn new(len: u32, isr: i32) -> Self {
        Self {
            ni_q: MbufQueue::new(len, IPL_NET),
            ni_isr: isr,
        }
    }
}

/// `niq_dequeue(q)`.
pub fn niq_dequeue(q: &Niqueue) -> Option<MbufBox> {
    mq_dequeue(&q.ni_q)
}

/// `niq_delist(q, ml)`: the queue's packets, taken.
pub fn niq_delist(q: &Niqueue) -> MbufList {
    mq_delist(&q.ni_q)
}

/// `niq_len(q)`.
pub fn niq_len(q: &Niqueue) -> u32 {
    mq_len(&q.ni_q)
}

/// `niq_drops(q)`.
pub fn niq_drops(q: &Niqueue) -> u32 {
    mq_drops(&q.ni_q)
}

/// The interface's driver: `if_var.h`'s procedure handles (`if_input`, `if_bpf_mtap`,
/// `if_output`, `if_ll_output`, `if_enqueue`, `if_start`, `if_ioctl`, `if_watchdog`,
/// `if_wol`, `if_qstart`), one object per interface, registered before `if_attach`
/// ([`Ifnet::if_driver`]). Each default method is what the C does with a NULL handle (or the
/// default `if_attach_common` installs). `if_rtrequest` is not one: the kernel never calls
/// it (`route.c`'s, N6.3b); nor are `if_output` and `if_ll_output` yet: their callers
/// (`if_output_ml`, `if_output_tso`, `if_output_mq`, the protocols' outputs) pass `bsd`'s
/// route and socket address and stay `bsd`'s with routing until N6.3b (D-N6.3a-9), so the
/// kernel calls neither in N6.3a. Decision 46: at N7 a native driver implements this trait (or
/// `NetDevice` absorbs it); until then `bsd`'s `LegacyNetDriver` does, over its drivers'
/// tables (bridges.toml's `NetDriver` row).
pub trait NetDriver: Sync + 'static {
    /// `if_start`: initiate output (a driver without `IFXF_MPSAFE`, under the kernel lock at
    /// `splnet`). Default: nothing (no `if_start`).
    fn start(&self, _ifp: &'static Ifnet) {}

    /// `if_qstart`: start the ring `ifq`. Default: `if_qstart_compat`, the driver's
    /// `if_start` under the kernel lock (what `if_attach_common` installs for a driver
    /// without `IFXF_MPSAFE`).
    fn qstart(&self, ifq: &'static Ifqueue) {
        crate::net::if_::if_qstart_compat(self, ifq);
    }

    /// `if_ioctl`: the request `cmd` on its argument's image (`IOCPARM_LEN(cmd)` bytes).
    fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, data: &mut [u8]) -> Result<(), Errno>;

    /// Whether the driver has an `if_watchdog` (the C's `if (ifp->if_watchdog)`).
    fn has_watchdog(&self) -> bool {
        false
    }

    /// `if_watchdog`: the timer routine (never called when [`NetDriver::has_watchdog`] says
    /// there is none).
    fn watchdog(&self, _ifp: &'static Ifnet) {}

    /// `if_wol`: wake on LAN on or off; `None` when the driver has none (NULL, which
    /// `ifioctl` refuses with `ENOTSUP`).
    fn wol(&self, _ifp: &'static Ifnet, _on: bool) -> Option<Result<(), Errno>> {
        None
    }

    /// `if_enqueue`: queue `m` for transmission. Default: `if_enqueue_ifq`.
    fn enqueue(&self, ifp: &'static Ifnet, m: MbufBox) -> Result<(), Errno> {
        crate::net::if_::if_enqueue_ifq(ifp, m)
    }

    /// `if_input`: a received packet, with the softnet's stack (`None`: the caller holds the
    /// net lock and wants it handled at once). Default: the interface's link layer's
    /// ([`link_input`]: `ether_input` as `ether_ifattach` fills the C's slot), `panic(9)`
    /// without one (the C calls through NULL).
    fn input(&self, ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
        link_input(ifp, m, ns);
    }

    /// `if_bpf_mtap`: tap `m` to the tap the interface's bpf(4) word `if_bpf` names; `true`
    /// drops it. Default: `bpf_mtap_ether` (what `if_attach_common` installs).
    fn bpf_mtap(&self, if_bpf: &crate::net::bpf::BpfTap, m: &Mbuf, dir: u32) -> bool {
        crate::net::bpf::bpf_mtap_ether(if_bpf, m, dir)
    }

    /// The object behind the trait, for `bsd`'s table to find itself again (its own type,
    /// named by downcasting); `None` for a native driver (docs/SCAFFOLDING.md, N7).
    fn as_any(&self) -> Option<&(dyn Any + Send + Sync)> {
        None
    }

    /// `if_rtrequest(ifp, req, rt)`: the interface's link layer told that route `rt` of the
    /// interface was added (`RTM_ADD`), cloned (`RTM_RESOLVE`) or deleted (`RTM_DELETE`),
    /// under the net lock (`net`) (N6.3b, D-N6.3b-11: `ether_rtrequest`, `lortrequest`,
    /// `p2p_rtrequest`). Default: the interface's link layer's ([`link_rtrequest`]:
    /// `ether_rtrequest`), nothing without one (the C's `if_rtrequest_dummy`).
    fn rtrequest(
        &self,
        ifp: &'static Ifnet,
        req: u8,
        rt: Option<&crate::net::route::Rtentry>,
        net: &hal::netlock::NetHeld<'_>,
    ) {
        link_rtrequest(ifp, req, rt, net);
    }

    /// `if_output(ifp, m, dst, rt)`: `m` sent toward `dst` through route `rt` (N6.3b,
    /// D-N6.3b-11: `ether_output`, `looutput`, the tunnels' outputs; `bsd`'s through
    /// `LegacyNetDriver` from N6.3b2). Default: the interface's link layer's
    /// ([`link_output`]: `ether_output`, which `ether_ifattach` installs when the driver set
    /// none), `panic(9)` without one (the C calls through NULL).
    fn output(
        &self,
        ifp: &'static Ifnet,
        m: MbufBox,
        dst: crate::net::proto::SockaddrView<'_>,
        rt: Option<&crate::net::route::Rtentry>,
    ) -> Result<(), Errno> {
        link_output(ifp, m, dst, rt)
    }
}

/// `struct if_clone`: a "cloning" interface (`ifconfig lo3 create`): its name and its two
/// functions. A cloner is attached once at boot (`if_clone_attach`) and lives for ever.
pub trait Cloner: Sync + 'static {
    /// `ifc_name`: the name of the device, e.g. `gif` (no NUL).
    fn name(&self) -> &'static [u8];

    /// `ifc_create(ifc, unit)`: creates unit `unit` (the cloner itself is the C's first
    /// argument, a `'static`).
    fn create(&'static self, unit: i32) -> Result<(), Errno>;

    /// Whether the cloner has an `ifc_destroy` (the C's `ifc->ifc_destroy != NULL`;
    /// `if_clone_destroy` answers `EOPNOTSUPP` without one, before it looks the interface up).
    fn has_destroy(&self) -> bool;

    /// `ifc_destroy`: destroys the cloned interface `ifp` (only called when
    /// [`Cloner::has_destroy`]).
    fn destroy(&self, ifp: &'static Ifnet) -> Result<(), Errno>;
}

/// An interface's link layer (decision 46): the `if_output`, `if_input` and `if_rtrequest`
/// that `ether_ifattach` installs for every Ethernet driver (`ether_output`, keeping a
/// driver's own `if_output`; `ether_input`; `ether_rtrequest`) and the loopback sets for
/// itself (`looutput`, `loinput`, `lortrequest`). Set once, before the interface is used, in
/// [`Ifnet::if_link`]; [`NetDriver`]'s default `output`, `input` and `rtrequest` call it, so
/// a driver's own handler wins and the link layer's is the C's filled slot. The C's
/// `if_ll_output` has no method: only `#ifdef MPLS` code writes it, and nothing reads it.
pub trait LinkLayer: Sync + 'static {
    /// `if_output(ifp, m, dst, rt)` of the link layer: encapsulate `m` for `dst` and send it.
    fn output(
        &self,
        ifp: &'static Ifnet,
        m: MbufBox,
        dst: crate::net::proto::SockaddrView<'_>,
        rt: Option<&crate::net::route::Rtentry>,
    ) -> Result<(), Errno>;

    /// `if_input(ifp, m, ns)` of the link layer: a received frame to its protocol.
    fn input(&self, ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>);

    /// `if_rtrequest(ifp, req, rt)` of the link layer, under the net lock (`net`).
    fn rtrequest(
        &self,
        ifp: &'static Ifnet,
        req: u8,
        rt: Option<&crate::net::route::Rtentry>,
        net: &hal::netlock::NetHeld<'_>,
    );
}

/// [`NetDriver::output`]'s default: the interface's link layer's `output`, `panic(9)` for an
/// interface without one (the C's call through a NULL `if_output`).
pub fn link_output(
    ifp: &'static Ifnet,
    m: MbufBox,
    dst: crate::net::proto::SockaddrView<'_>,
    rt: Option<&crate::net::route::Rtentry>,
) -> Result<(), Errno> {
    if let Some(&l) = ifp.if_link.get() {
        return l.output(ifp, m, dst, rt);
    }
    drop(m);
    ifp.if_xname
        .with_str(|n| kpanic(format_args!("{}: no if_output", Str(n))))
}

/// [`NetDriver::input`]'s default: the interface's link layer's `input`, `panic(9)` without
/// one (the C's call through a NULL `if_input`).
pub fn link_input(ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
    if let Some(&l) = ifp.if_link.get() {
        l.input(ifp, m, ns);
        return;
    }
    drop(m);
    ifp.if_xname
        .with_str(|n| kpanic(format_args!("{}: no if_input", Str(n))));
}

/// [`NetDriver::rtrequest`]'s default: the interface's link layer's `rtrequest`, nothing
/// without one (`if_rtrequest_dummy`).
pub fn link_rtrequest(
    ifp: &'static Ifnet,
    req: u8,
    rt: Option<&crate::net::route::Rtentry>,
    net: &hal::netlock::NetHeld<'_>,
) {
    if let Some(&l) = ifp.if_link.get() {
        l.rtrequest(ifp, req, rt, net);
    }
}

/// The bits of [`Ifnet::detached`]: `if_detach` swapped the C's `if_qstart` and `if_ioctl` for
/// `if_detached_qstart` and `if_detached_ioctl`.
pub(crate) const DETACHED_QSTART: u8 = 1 << 0;
/// See [`DETACHED_QSTART`].
pub(crate) const DETACHED_IOCTL: u8 = 1 << 1;

/// What `if_attach_common` builds for an interface (the members of `struct ifnet` that are not
/// valid all zero: the queues' mutexes and tasks, the timeout, the hooks, the reference count's
/// protocol and the driver), set once into [`Ifnet::rt`] (D-N6.3a-1).
pub struct IfRt {
    /// `if_refcnt`.
    pub(crate) if_refcnt: Refcnt,
    /// `if_snd`: transmit queue (`if_ifqs[0]`).
    pub(crate) if_snd: Ifqueue,
    /// `if_rcv`: rx/input queue (`if_iqs[0]`).
    pub(crate) if_rcv: Ifiqueue,
    /// \[I\] `if_ifqs[1..]`: the other send queues, set by `if_attach_queues`.
    pub(crate) ifqs: OnceStatic<MArray<MBox<Ifqueue>>>,
    /// \[I\] `if_iqs[1..]`: the other input queues, set by `if_attach_iqueues`.
    pub(crate) iqs: OnceStatic<MArray<MBox<Ifiqueue>>>,
    /// \[N\] `if_addrlist`: list of addresses per if ([`Ifnet::if_addrlist`]).
    pub(crate) if_addrlist: IfaList,
    /// `if_maddrlock`: keeps `if_maddrlist`, taken C-style (`rw_enter_write` ..
    /// `rw_exit_write`) by `bsd`'s multicast code, its only user until N6.4/N6.5.
    pub if_maddrlock: hal::rwlock::SplitRwlock,
    /// \[I\] `if_slowtimo`: watchdog timeout (its argument the interface's index).
    pub(crate) if_slowtimo: Timeout,
    /// \[I\] `if_watchdogtask`: watchdog task (its argument the interface's index).
    pub(crate) if_watchdogtask: Task,
    /// \[I\] `if_linkstatetask`: task to do route updates (its argument the index).
    pub(crate) if_linkstatetask: Task,
    /// `if_hooks_mtx` (one per interface here: the C's one global mutex keeps per-interface
    /// lists, at the same level, taken by no other path): keeps the three hook lists.
    pub(crate) hooks_mtx: Mutexed<()>,
    /// \[I\] `if_addrhooks`: address change callbacks.
    pub(crate) if_addrhooks: PinList<TaskChain>,
    /// \[I\] `if_linkstatehooks`: link change callbacks.
    pub(crate) if_linkstatehooks: PinList<TaskChain>,
    /// \[I\] `if_detachhooks`: detach callbacks.
    pub(crate) if_detachhooks: PinList<TaskChain>,
    /// The driver (`if_attach`'s, from [`Ifnet::if_driver`]).
    pub(crate) driver: &'static dyn NetDriver,
}

/// `struct ifnet`: a network interface, providing a packet transport mechanism (ala level 0
/// of the PUP protocols). Embedded by value in a driver's zeroed softc and lent `&'static`
/// (the module's redesign): every member is valid all zero, and [`Ifnet::rt`] holds what
/// `if_attach` builds.
pub struct Ifnet {
    /// \[I\] `if_softc`: lower-level data for this if (the driver's word, never followed
    /// here).
    pub if_softc: PtrWord<core::ffi::c_void>,
    /// \[NK\] `if_list`: all struct ifnets are chained.
    pub if_list: ListEntry<Ifnet>,
    /// \[T\] `if_tmplist`: temporary list.
    pub if_tmplist: ListEntry<Ifnet>,
    /// \[m\] `if_maddrlist`: list of multicast records.
    pub if_maddrlist: List<IfMaddrlist>,
    /// \[I\] `if_xname`: external name (name + unit), NUL-terminated.
    pub if_xname: IfName,
    /// \[N\] `if_pcount`: # of promiscuous listeners.
    pub if_pcount: AtomicCell<i32>,
    /// \[K\] `if_bridgeidx`: used by bridge ports.
    pub if_bridgeidx: AtomicCell<u32>,
    /// `if_bpf`: packet filter structure (bpf(4)'s word: the tap while a descriptor listens).
    pub if_bpf: crate::net::bpf::BpfTap,
    /// `if_mcast`: used by multicast code (`struct vif *`, not configured).
    pub if_mcast: PtrWord<core::ffi::c_void>,
    /// `if_mcast6`: used by IPv6 multicast code (`struct mif6 *`, not configured).
    pub if_mcast6: PtrWord<core::ffi::c_void>,
    /// `if_pf_kif`: pf interface abstraction (pf's word).
    pub if_pf_kif: PtrWord<u8>,
    /// `if_carp` (`if_carp_ptr.carp_s`): carp if list (used by `IFT_ETHER`), its first
    /// pointer.
    pub if_carp: PtrWord<core::ffi::c_void>,
    /// `if_carpdevidx` (`if_carp_ptr.carp_idx`): index of carpdev (used by `IFT_CARP`).
    pub if_carpdevidx: AtomicCell<u32>,
    /// \[I\] `if_index`: unique index for this if.
    pub if_index: AtomicCell<u32>,
    /// `if_timer`: time 'til `if_watchdog` called.
    pub if_timer: AtomicCell<i16>,
    /// \[N\] `if_flags`: up/down, broadcast, etc. (`IFF_*`).
    pub if_flags: AtomicCell<i32>,
    /// \[N\] `if_xflags`: extra softnet flags (`IFXF_*`).
    pub if_xflags: AtomicCell<i32>,
    /// `if_type`: `IFT_*`.
    pub if_type: AtomicCell<u8>,
    /// `if_addrlen`.
    pub if_addrlen: AtomicCell<u8>,
    /// `if_hdrlen`.
    pub if_hdrlen: AtomicCell<u8>,
    /// `if_link_state`.
    pub if_link_state: AtomicCell<u8>,
    /// `if_mtu`.
    pub if_mtu: AtomicCell<u32>,
    /// `if_metric`.
    pub if_metric: AtomicCell<u32>,
    /// `if_baudrate`.
    pub if_baudrate: AtomicCell<u64>,
    /// `if_capabilities`: `IFCAP_*`.
    pub if_capabilities: AtomicCell<u32>,
    /// `if_rdomain`.
    pub if_rdomain: AtomicCell<u32>,
    /// \[c\] `if_lastchange`: last op. state change.
    pub if_lastchange: IfLastchange,
    /// `if_data_counters`.
    pub if_data_counters: [AtomicCell<u64>; IFC_NCOUNTERS],
    /// `if_counters`: the stack's counters (off until `if_counters_alloc`).
    pub if_counters: IfCountersSlot,
    /// \[d\] `if_hardmtu`: maximum MTU device supports.
    pub if_hardmtu: AtomicCell<u32>,
    /// \[c\] `if_description`: interface description.
    pub if_description: IfDescr,
    /// \[c\] `if_rtlabelid`: next route label.
    pub if_rtlabelid: AtomicCell<u16>,
    /// \[c\] `if_priority`: route priority offset.
    pub if_priority: AtomicCell<u8>,
    /// \[N\] `if_llprio`: link layer priority.
    pub if_llprio: AtomicCell<u8>,
    /// \[c\] `if_txmit`: txmitigation amount.
    pub if_txmit: AtomicCell<u32>,
    /// `if_snd.ifq_maxlen` as the driver sets it before attach (`ifq_init_maxlen`), which
    /// `ifq_init` keeps when it is not 0.
    pub if_snd_maxlen: AtomicCell<u32>,
    /// \[N\] `if_sadl`: our `sockaddr_dl` (`if_alloc_sadl`'s box).
    pub if_sadl: ReadMostly<Option<MBox<SockaddrDl>>>,
    /// \[N\] `if_nd`: IPv6 Neighbor Discovery info (`struct nd_ifinfo`, embedded: D-N6.5-6).
    pub if_nd: crate::netinet6::nd6::NdIfinfoCell,
    /// \[N\] `if_groups`: the groups the interface is a member of (a snapshot replaced whole
    /// by `if_addgroup` and `if_delgroup`; readers through [`Ifnet::groups`]).
    pub(crate) if_groups: ReadMostly<IfgSet>,
    /// The driver, registered before `if_attach` (`bsd`'s table, D-N6.3a-8).
    pub if_driver: OnceStatic<&'static dyn NetDriver>,
    /// The link layer (`ether_ifattach`'s, the loopback's): the C's `if_output`, `if_input`
    /// and `if_rtrequest` as the link layer fills them, which [`NetDriver`]'s defaults call.
    pub if_link: OnceStatic<&'static dyn LinkLayer>,
    /// What `if_attach_common` builds.
    pub rt: OnceStatic<IfRt>,
    /// [`DETACHED_QSTART`], [`DETACHED_IOCTL`]: what `if_detach` swapped.
    pub(crate) detached: AtomicU8,
    /// Not in the C: the `struct arpcom` whose `ac_if` this is, set by `ether_ifattach` (the
    /// module's deviations); lent `'static` from `bsd`'s softcs as the interface is.
    pub if_arpcom: Link<crate::net::if_ethersubr::Arpcom>,
}

/// Defines the `if_ipackets` .. `if_noproto` shorthands for `if_data_counters[ifc_*]`.
macro_rules! if_data_counter {
    ($($(#[$doc:meta])* $name:ident => $idx:ident;)*) => {
        $(
            $(#[$doc])*
            pub fn $name(&self) -> &AtomicCell<u64> {
                &self.if_data_counters[IfCounters::$idx as usize]
            }
        )*
    };
}

impl Ifnet {
    /// An interface not attached, every member at zero (what a zeroed softc holds).
    pub const fn new() -> Self {
        Self {
            if_softc: PtrWord::new(),
            if_list: ListEntry::new(),
            if_tmplist: ListEntry::new(),
            if_maddrlist: List::new(),
            if_xname: AtomicBytes::new(),
            if_pcount: AtomicCell::new(),
            if_bridgeidx: AtomicCell::new(),
            if_bpf: crate::net::bpf::BpfTap::new(),
            if_mcast: PtrWord::new(),
            if_mcast6: PtrWord::new(),
            if_pf_kif: PtrWord::new(),
            if_carp: PtrWord::new(),
            if_carpdevidx: AtomicCell::new(),
            if_index: AtomicCell::new(),
            if_timer: AtomicCell::new(),
            if_flags: AtomicCell::new(),
            if_xflags: AtomicCell::new(),
            if_type: AtomicCell::new(),
            if_addrlen: AtomicCell::new(),
            if_hdrlen: AtomicCell::new(),
            if_link_state: AtomicCell::new(),
            if_mtu: AtomicCell::new(),
            if_metric: AtomicCell::new(),
            if_baudrate: AtomicCell::new(),
            if_capabilities: AtomicCell::new(),
            if_rdomain: AtomicCell::new(),
            if_lastchange: IfLastchange::new(),
            if_data_counters: [const { AtomicCell::new() }; IFC_NCOUNTERS],
            if_counters: IfCountersSlot::new(),
            if_hardmtu: AtomicCell::new(),
            if_description: AtomicBytes::new(),
            if_rtlabelid: AtomicCell::new(),
            if_priority: AtomicCell::new(),
            if_llprio: AtomicCell::new(),
            if_txmit: AtomicCell::new(),
            if_snd_maxlen: AtomicCell::new(),
            if_sadl: ReadMostly::new(None),
            if_nd: crate::netinet6::nd6::NdIfinfoCell::new(),
            if_groups: ReadMostly::new(IfgSet::new()),
            if_driver: OnceStatic::new(),
            if_link: OnceStatic::new(),
            rt: OnceStatic::new(),
            detached: AtomicU8::new(0),
            if_arpcom: Link::new(),
        }
    }

    if_data_counter! {
        /// `if_ipackets`.
        if_ipackets => IfcIpackets;
        /// `if_ierrors`.
        if_ierrors => IfcIerrors;
        /// `if_opackets`.
        if_opackets => IfcOpackets;
        /// `if_oerrors`.
        if_oerrors => IfcOerrors;
        /// `if_collisions`.
        if_collisions => IfcCollisions;
        /// `if_ibytes`.
        if_ibytes => IfcIbytes;
        /// `if_obytes`.
        if_obytes => IfcObytes;
        /// `if_imcasts`.
        if_imcasts => IfcImcasts;
        /// `if_omcasts`.
        if_omcasts => IfcOmcasts;
        /// `if_iqdrops`.
        if_iqdrops => IfcIqdrops;
        /// `if_oqdrops`.
        if_oqdrops => IfcOqdrops;
        /// `if_noproto`.
        if_noproto => IfcNoproto;
    }

    /// What `if_attach_common` built; `panic(9)` before attach (the C would follow a NULL
    /// queue pointer).
    #[inline]
    pub fn rt(&self) -> &IfRt {
        match self.rt.get() {
            Some(rt) => rt,
            None => self
                .if_xname
                .with_str(|n| kpanic(format_args!("{}: used before if_attach", Str(n)))),
        }
    }

    /// `&ifp->if_addrlist`: the interface's addresses (in [`IfRt`], made by `if_attach`, as
    /// the C's `TAILQ_INIT`); `panic(9)` before attach.
    #[inline]
    pub fn if_addrlist(&self) -> &IfaList {
        &self.rt().if_addrlist
    }

    /// `&ifp->if_snd`: the first send queue.
    #[inline]
    pub fn if_snd(&self) -> &Ifqueue {
        &self.rt().if_snd
    }

    /// `&ifp->if_rcv`: the first input queue.
    #[inline]
    pub fn if_rcv(&self) -> &Ifiqueue {
        &self.rt().if_rcv
    }

    /// `if_nifqs`: number of output queues (1 until `if_attach_queues`).
    #[inline]
    pub fn if_nifqs(&self) -> u32 {
        self.rt().ifqs.get().map_or(1, |q| q.len() as u32 + 1)
    }

    /// `if_niqs`: number of input queues (1 until `if_attach_iqueues`).
    #[inline]
    pub fn if_niqs(&self) -> u32 {
        self.rt().iqs.get().map_or(1, |q| q.len() as u32 + 1)
    }

    /// `ifp->if_ifqs[i]`: send queue `i`, below `if_nifqs`; `panic(9)` past it (the C reads
    /// past the array).
    #[inline]
    pub fn ifq(&self, i: u32) -> &Ifqueue {
        let rt = self.rt();
        if i == 0 {
            return &rt.if_snd;
        }
        match rt.ifqs.get().and_then(|q| q.get(i as usize - 1)) {
            Some(q) => q,
            None => kpanic(format_args!("interface queue {i} of {}", self.if_nifqs())),
        }
    }

    /// `ifp->if_iqs[i]`: input queue `i`, below `if_niqs`.
    #[inline]
    pub fn ifiq(&self, i: u32) -> &Ifiqueue {
        let rt = self.rt();
        if i == 0 {
            return &rt.if_rcv;
        }
        match rt.iqs.get().and_then(|q| q.get(i as usize - 1)) {
            Some(q) => q,
            None => kpanic(format_args!("interface queue {i} of {}", self.if_niqs())),
        }
    }

    /// The interface's driver for the requests the C sends through `if_qstart` (the detached
    /// stub after `if_detach`'s swap, [`DETACHED_QSTART`]) or through the other handles.
    #[inline]
    pub(crate) fn driver(&self) -> &'static dyn NetDriver {
        self.rt().driver
    }

    /// Whether `if_detach` swapped the handle `bit` (`DETACHED_*`).
    #[inline]
    pub(crate) fn is_detached(&self, bit: u8) -> bool {
        self.detached.load(Ordering::Acquire) & bit != 0
    }

    /// `ifp->if_sadl`, borrowed for a reader that does not sleep with it (`hal::slot`'s
    /// rule): `None` before `if_alloc_sadl`.
    pub fn sadl(&self) -> hal::slot::ReadMostlyRef<'_, Option<MBox<SockaddrDl>>> {
        self.if_sadl.read()
    }

    /// `ifp->if_groups`, borrowed for a reader (the rule of [`IfgGroup::members`]).
    pub fn groups(&self) -> hal::slot::ReadMostlyRef<'_, IfgSet> {
        self.if_groups.read()
    }

    /// Whether `if_detach` swapped the C's `if_ioctl` for `if_detached_ioctl` (a request then
    /// answers `ENODEV`, which the kernel's `if_ioctl` checks itself).
    pub fn if_ioctl_detached(&self) -> bool {
        self.is_detached(DETACHED_IOCTL)
    }
}

impl Default for Ifnet {
    fn default() -> Self {
        Self::new()
    }
}

hal::list_adapter!(
    /// `TAILQ_HEAD(ifnet_head, ifnet)` through `if_list`: `ifnetlist`.
    pub IfnetList: Ifnet, if_list
);

hal::list_adapter!(
    /// `TAILQ_ENTRY(ifnet) if_tmplist`: the temporary lists of `ifconf` and
    /// `if_getgroupmembers`.
    pub IfTmplist: Ifnet, if_tmplist
);

/// `struct ifaddr`'s generic part: information about one address of an interface. The
/// addresses are maintained by the different address families, are allocated and attached
/// when an address is set, and are linked together so all addresses for an interface can be
/// located. A protocol's record (`kernel::netinet::in_var::InIfaddr`,
/// `kernel::netinet6::in6_var::In6Ifaddr`) embeds it first and holds the C's other members
/// (the addresses, the list links); enc(4)'s [`IfaddrLinked`] embeds it in a zeroed softc.
/// Reached through an [`IfaddrRef`] (`Deref`). Valid all zero.
pub struct Ifaddr {
    /// `ifa_ifp`: back-pointer to interface.
    pub ifa_ifp: Link<Ifnet>,
    /// `ifa_flags`: interface flags, see below.
    pub ifa_flags: AtomicCell<u32>,
    /// `ifa_metric`: cost of going out this interface.
    pub ifa_metric: AtomicCell<i32>,
}

impl Ifaddr {
    /// An address on no interface, every member zero.
    pub const fn new() -> Self {
        Self {
            ifa_ifp: Link::new(),
            ifa_flags: AtomicCell::new(),
            ifa_metric: AtomicCell::new(),
        }
    }
}

impl Default for Ifaddr {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct ifaddr` whole, for an address no protocol record holds: enc(4)'s fake `AF_LINK`
/// address (`sc_ifa`), embedded in its zeroed softc, whose memory the softc owns. Its
/// references are counted in `ifa_refcnt`: the softc's own (`refcnt_init`) and one per
/// [`LinkedIfa`]; `enc_clone_destroy` frees the softc only after its own release was the
/// last (`panic(9)` otherwise), so no handle outlives it. Its addresses are `SaLink`s to
/// `bsd`'s storage, published with the `SaToken`. Never on an interface's list (enc(4)'s is
/// not, if_enc.c:103-106). Valid all zero: every member is a `hal` link, a count or an
/// atomic.
pub struct IfaddrLinked {
    /// The generic part.
    pub ifa: Ifaddr,
    /// `ifa_refcnt`: the owner's reference and the handles'.
    pub ifa_refcnt: Refcnt,
    /// `ifa_addr`: address of interface.
    pub ifa_addr: SaLink,
    /// `ifa_dstaddr`: other end of p-to-p link (`ifa_broadaddr`: broadcast address).
    pub ifa_dstaddr: SaLink,
    /// `ifa_netmask`: used to determine subnet.
    pub ifa_netmask: SaLink,
}

impl IfaddrLinked {
    /// An address on no interface: its owner's one reference (`refcnt_init`), every other
    /// member zero.
    pub const fn new() -> Self {
        Self {
            ifa: Ifaddr::new(),
            ifa_refcnt: Refcnt::new(),
            ifa_addr: SaLink::new(),
            ifa_dstaddr: SaLink::new(),
            ifa_netmask: SaLink::new(),
        }
    }
}

impl Default for IfaddrLinked {
    fn default() -> Self {
        Self::new()
    }
}

/// A counted reference of an [`IfaddrLinked`]: made by `ifaref` (`refcnt_take`), given back
/// by its drop (`ifafree`'s `refcnt_rele`). The last reference is the owner's, released by
/// the owner before it frees the record (`enc_clone_destroy`); a handle that sees the count
/// reach zero is `panic(9)` (the C would `free` a part of the softc).
pub struct LinkedIfa(&'static IfaddrLinked);

impl LinkedIfa {
    /// `ifaref(ifa)` of a linked address: one more reference, for its owner's lookup
    /// (`enc_getifa`).
    pub fn new(ifa: &'static IfaddrLinked) -> Self {
        ifa.ifa_refcnt.take();
        Self(ifa)
    }
}

impl Clone for LinkedIfa {
    fn clone(&self) -> Self {
        Self::new(self.0)
    }
}

impl Drop for LinkedIfa {
    fn drop(&mut self) {
        if self.0.ifa_refcnt.rele() {
            kpanic(format_args!(
                "ifafree: the last reference of a linked address is its owner's"
            ));
        }
    }
}

/// A counted reference of an interface address (a `struct ifaddr *` the C holds with
/// `ifaref`), one arm per family. The family records are `malloc(9)` blocks of `M_IFADDR`
/// (`hal::malloc::MArc`), freed with their last handle (`ifafree`'s `free(ifa, M_IFADDR,
/// 0)`); enc(4)'s is linked ([`LinkedIfa`]). It derefs to the generic part ([`Ifaddr`]).
/// `Clone` is `ifaref`, the drop `ifafree` ([`crate::net::route::ifaref`],
/// [`crate::net::route::ifafree`]): the handles' count is the C's `ifa_refcnt`
/// ([`IfaddrRef::count`]).
#[derive(Clone)]
pub enum IfaddrRef {
    /// An Internet address (`AF_INET`, `struct in_ifaddr`).
    In(MArc<InIfaddr>),
    /// An IPv6 address (`AF_INET6`, `struct in6_ifaddr`).
    In6(MArc<In6Ifaddr>),
    /// An address its owner embeds (enc(4)'s).
    Linked(LinkedIfa),
}

impl Deref for IfaddrRef {
    type Target = Ifaddr;

    #[inline]
    fn deref(&self) -> &Ifaddr {
        match self {
            IfaddrRef::In(ia) => &ia.ia_ifa,
            IfaddrRef::In6(ia6) => &ia6.ia_ifa,
            IfaddrRef::Linked(l) => &l.0.ifa,
        }
    }
}

impl IfaddrRef {
    /// Whether this handle names the address `ifa` (the C's comparison of the pointers).
    #[inline]
    pub fn is(&self, ifa: &Ifaddr) -> bool {
        let me: &Ifaddr = self;
        core::ptr::eq(me, ifa)
    }

    /// The references of the address alive now (`ifa_refcnt`, a snapshot), for ddb and the
    /// tests.
    pub fn count(&self) -> usize {
        match self {
            IfaddrRef::In(ia) => MArc::count(ia),
            IfaddrRef::In6(ia6) => MArc::count(ia6),
            IfaddrRef::Linked(l) => l.0.ifa_refcnt.read() as usize,
        }
    }

    /// `ifa->ifa_addr`: a copy of the address (`sa_len` bytes cut at its storage), `None`
    /// where the C's pointer is NULL. A family record's is its own words; a linked address's
    /// is read under the net lock `net` proves.
    #[inline]
    pub fn ifa_addr(&self, net: &NetHeld<'_>) -> Option<SaBytes> {
        match self {
            IfaddrRef::In(ia) => Some(ia.ia_addr.read()),
            IfaddrRef::In6(ia6) => Some(ia6.ia_addr.read()),
            IfaddrRef::Linked(l) => l.0.ifa_addr.read(net),
        }
    }

    /// `ifa->ifa_dstaddr` (`ifa_broadaddr`), as [`IfaddrRef::ifa_addr`]: an Internet
    /// address's always (its `ia_dstaddr`), an IPv6 address's only on a point-to-point or
    /// loopback interface (`in6_update_ifa`'s NULL elsewhere).
    #[inline]
    pub fn ifa_dstaddr(&self, net: &NetHeld<'_>) -> Option<SaBytes> {
        match self {
            IfaddrRef::In(ia) => Some(ia.ia_dstaddr.read()),
            IfaddrRef::In6(ia6) => ia6.ia_dstaddr_linked.then(|| ia6.ia_dstaddr.read()),
            IfaddrRef::Linked(l) => l.0.ifa_dstaddr.read(net),
        }
    }

    /// `ifa->ifa_broadaddr`: the `ifa_dstaddr` member.
    #[inline]
    pub fn ifa_broadaddr(&self, net: &NetHeld<'_>) -> Option<SaBytes> {
        self.ifa_dstaddr(net)
    }

    /// `ifa->ifa_netmask`, as [`IfaddrRef::ifa_addr`].
    #[inline]
    pub fn ifa_netmask(&self, net: &NetHeld<'_>) -> Option<SaBytes> {
        match self {
            IfaddrRef::In(ia) => Some(ia.ia_sockmask.read()),
            IfaddrRef::In6(ia6) => Some(ia6.ia_prefixmask.read()),
            IfaddrRef::Linked(l) => l.0.ifa_netmask.read(net),
        }
    }

    /// `ifa_list`: the cell of the next address on the interface's list, the record's own;
    /// `panic(9)` for a linked address, which is on no list.
    #[inline]
    pub(crate) fn list_cell(&self) -> &Guarded<Option<IfaddrRef>> {
        match self {
            IfaddrRef::In(ia) => &ia.ia_list,
            IfaddrRef::In6(ia6) => &ia6.ia_list,
            IfaddrRef::Linked(_) => kpanic(format_args!("ifa_list: a linked address")),
        }
    }

    /// `ifa_tmplist`: the cell of the next address on `ifconf`'s temporary list (\[T\], keyed
    /// to `if_tmplist_lock`); `panic(9)` for a linked address, which is on no interface.
    #[inline]
    pub(crate) fn tmplist_cell(&self) -> &Guarded<Option<IfaddrRef>> {
        match self {
            IfaddrRef::In(ia) => &ia.ia_tmplist,
            IfaddrRef::In6(ia6) => &ia6.ia_tmplist,
            IfaddrRef::Linked(_) => kpanic(format_args!("ifa_tmplist: a linked address")),
        }
    }
}

/// `TAILQ_HEAD(, ifaddr) if_addrlist`: an interface's addresses in the order they were added
/// (the order `ifconf`, the routing socket and every walk see). A chain of counted handles in
/// net cells (`hal::netlock`): the head's cell holds the first address's, each address's own
/// `ia_list` the next one's. The list holds one reference of each address, its creation
/// reference ([`crate::net::if_::ifa_add`] takes it, [`crate::net::if_::ifa_del`] gives it
/// back). A walk lends `&'r IfaddrRef`s for the life of its witness
/// (`hal::netlock::NetHeld::reader`), uncounted as the C's pointers, O(1) a step; a change
/// takes the writer, which no reader can be out beside.
///
/// An address a walk lends cannot outlive the reader it borrows (a reference past its
/// record's last handle cannot be written):
///
/// ```compile_fail,E0515
/// use hal::netlock::{NETLOCK, NetHeld};
/// use kernel::net::if_var::{IfaList, IfaddrRef};
/// fn kept(list: &IfaList) -> Option<&IfaddrRef> {
///     let w = NETLOCK.read();
///     let net = NetHeld::of(&w)?;
///     let r = net.reader();
///     list.first(&r)
/// }
/// ```
pub struct IfaList {
    /// `tqh_first`.
    head: Guarded<Option<IfaddrRef>>,
}

impl IfaList {
    /// An empty list (`TAILQ_INIT`).
    pub const fn new() -> Self {
        Self {
            head: Guarded::new_net(None),
        }
    }

    /// `TAILQ_FIRST`: the first address, for the life of the witness `r`.
    #[inline]
    pub fn first<'r, W: Witness + ?Sized>(&'r self, r: &'r W) -> Option<&'r IfaddrRef> {
        self.head.get(r).as_ref()
    }

    /// `TAILQ_NEXT(ifa, ifa_list)`: the address after `ifa`, for the life of `r`.
    #[inline]
    pub fn next<'r, W: Witness + ?Sized>(r: &'r W, ifa: &'r IfaddrRef) -> Option<&'r IfaddrRef> {
        ifa.list_cell().get(r).as_ref()
    }

    /// `TAILQ_FOREACH`: the addresses, first to last, for the life of `r`; O(1) a step.
    #[inline]
    pub fn iter<'r, W: Witness + ?Sized>(
        &'r self,
        r: &'r W,
    ) -> impl Iterator<Item = &'r IfaddrRef> {
        core::iter::successors(self.first(r), move |a| Self::next(r, a))
    }

    /// `TAILQ_EMPTY`.
    #[inline]
    pub fn is_empty<W: Witness + ?Sized>(&self, r: &W) -> bool {
        self.head.get(r).is_none()
    }

    /// `TAILQ_INSERT_TAIL`: `ifa`'s handle linked last, the list's reference. The walk to the
    /// last address is O(n) (D-4); that address's cell is opened through a clone of its handle
    /// (T3: a reference lent by `get` cannot be held while the witness opens a cell with
    /// `get_mut`), dropped before the return.
    pub fn insert_tail<W: Exclusive + ?Sized>(&self, w: &mut W, ifa: IfaddrRef) {
        let last = self.iter(&*w).last().cloned(); // T3
        match last {
            None => *self.head.get_mut(w) = Some(ifa),
            Some(l) => *l.list_cell().get_mut(w) = Some(ifa),
        }
    }

    /// `TAILQ_REMOVE`: the address `ifa` unlinked and its handle, the list's reference,
    /// returned; `None` when it is not on the list. The walk to its predecessor is O(n) (D-4);
    /// that one's cell is opened through a clone of its handle (T3). The removed address's
    /// own cell is emptied: it holds no reference of its successor any more.
    pub fn remove<W: Exclusive + ?Sized>(&self, w: &mut W, ifa: &Ifaddr) -> Option<IfaddrRef> {
        let pred = {
            let r: &W = &*w;
            let mut pred: Option<&IfaddrRef> = None;
            let mut at = self.first(r);
            loop {
                let a = at?;
                if a.is(ifa) {
                    break pred.cloned(); // T3
                }
                pred = Some(a);
                at = Self::next(r, a);
            }
        };
        let cell = match &pred {
            Some(p) => p.list_cell(),
            None => &self.head,
        };
        let removed = cell.get_mut(w).take()?;
        let after = removed.list_cell().get_mut(w).take();
        *cell.get_mut(w) = after;
        Some(removed)
    }

    /// `TAILQ_FOREACH_SAFE`: `f` of each address, under the net lock `net` proves, for a body
    /// that may take the address it is given off the list (`ifa_del` takes the writer). No
    /// reader lives while `f` runs: the address and its successor are cloned under a reader of
    /// their own before it (T2), dropped after it. A body that takes the successor off the
    /// list is still given it next (the walk's clone keeps it alive), and the walk ends after
    /// it (its cell was emptied), where the C would read freed memory.
    pub fn for_each_safe(&self, net: &NetHeld<'_>, mut f: impl FnMut(&IfaddrRef)) {
        let mut cur = {
            let r = net.reader();
            self.first(&r).cloned() // T2
        };
        while let Some(c) = cur {
            let next = {
                let r = net.reader();
                Self::next(&r, &c).cloned() // T2
            };
            f(&c);
            cur = next;
        }
    }
}

impl Default for IfaList {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct ifmaddr`: interface multicast address. A protocol's record (`struct in_multi`,
/// `bsd`'s) embeds it first, hence `#[repr(C)]`; valid all zero.
#[repr(C)]
pub struct Ifmaddr {
    /// \[m\] `ifma_list`: per-interface list.
    pub ifma_list: ListEntry<Ifmaddr>,
    /// `ifma_refcnt`: count of references.
    pub ifma_refcnt: Refcnt,
    /// \[I\] `ifma_ifidx`: index of the interface.
    pub ifma_ifidx: AtomicCell<u32>,
    /// \[I\] The Internet multicast record this is the generic part of (`ifmatoinm`'s cast,
    /// a link set when `kernel::netinet::in_var` makes the record).
    pub ifma_in: Link<crate::netinet::in_var::InMulti>,
    /// \[I\] The IPv6 multicast record this is the generic part of (`ifmatoin6m`'s cast, a
    /// link set when `kernel::netinet6::in6_var` makes the record).
    pub ifma_in6: Link<crate::netinet6::in6_var::In6Multi>,
}

impl Ifmaddr {
    /// A record on no list, every member zero.
    pub const fn new() -> Self {
        Self {
            ifma_list: ListEntry::new(),
            ifma_refcnt: Refcnt::new(),
            ifma_ifidx: AtomicCell::new(),
            ifma_in: Link::new(),
            ifma_in6: Link::new(),
        }
    }
}

impl Default for Ifmaddr {
    fn default() -> Self {
        Self::new()
    }
}

hal::list_adapter!(
    /// `TAILQ_HEAD(, ifmaddr) if_maddrlist`.
    pub IfMaddrlist: Ifmaddr, ifma_list
);

/// `struct ifg_group`: an interface group (`ifconfig em0 group foo`), with its member
/// interfaces. Held by counted references ([`IfgRef`]) from the groups' list (`ifg_head`) and
/// from each member's `if_groups`; freed with the last one (the C's `ifgroup_icrele` and its
/// temporary references, which the snapshots make unnecessary).
pub struct IfgGroup {
    /// `ifg_group`: the group's name, NUL-padded.
    pub ifg_group: [u8; IFNAMSIZ],
    /// \[N\] `ifg_refcnt`: the members, the creator's one counted until the first joins.
    pub(crate) ifg_refcnt: AtomicU32,
    /// `ifg_pf_kif`: pf's kif of the group (pf's word).
    pub ifg_pf_kif: PtrWord<u8>,
    /// `ifg_carp_demoted`.
    pub ifg_carp_demoted: AtomicCell<i32>,
    /// \[N\] `ifg_members`: the member interfaces, a snapshot replaced whole by each change.
    pub(crate) ifg_members: ReadMostly<IfgMembers>,
}

impl IfgGroup {
    /// The group's name without its NUL padding.
    pub fn name(&self) -> &[u8] {
        let n = self
            .ifg_group
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(IFNAMSIZ);
        &self.ifg_group[..n]
    }

    /// `ifg_members`, borrowed for a reader. A reader that sleeps with it holds the net lock,
    /// which every writer holds exclusively (the C's protocol), so no writer waits for it.
    pub fn members(&self) -> hal::slot::ReadMostlyRef<'_, IfgMembers> {
        self.ifg_members.read()
    }

    /// `ifg_refcnt`.
    pub fn refcnt(&self) -> u32 {
        self.ifg_refcnt.load(Ordering::Relaxed)
    }
}

/// A counted reference to a group (`struct ifg_list`'s `ifgl_group`, `ifg_head`'s links).
pub type IfgRef = hal::malloc::MArc<IfgGroup>;

/// A list of groups (`TAILQ_HEAD(, ifg_list) if_groups`, `ifg_head`), as one counted array
/// replaced whole by each change (the module's redesign), empty as `None`. A reader that
/// copies out (and may sleep) keeps a [`IfgSet::snapshot`] in place of the C's temporary
/// list and references (`ifg_tmplist`, `ifgroup_icref`).
#[derive(Default)]
pub struct IfgSet(pub(crate) Option<hal::malloc::MArc<MArray<IfgRef>>>);

impl IfgSet {
    /// The empty list.
    pub const fn new() -> Self {
        Self(None)
    }

    /// The same array, one more reference to it (no copy): it stays as it is while the
    /// cell it came from is replaced.
    #[must_use]
    pub fn snapshot(&self) -> Self {
        Self(self.0.clone())
    }

    /// The groups, in the order they were added (the C's `TAILQ_INSERT_TAIL`).
    pub fn iter(&self) -> impl Iterator<Item = &IfgGroup> {
        self.refs().iter().map(|g| &**g)
    }

    /// The counted references, in order.
    pub fn refs(&self) -> &[IfgRef] {
        self.0.as_deref().map_or(&[], |a| a)
    }

    /// The number of groups.
    pub fn len(&self) -> usize {
        self.refs().len()
    }

    /// No group.
    pub fn is_empty(&self) -> bool {
        self.refs().is_empty()
    }
}

/// `TAILQ_HEAD(, ifg_member) ifg_members`: a group's members, in the order they joined, as one
/// array replaced whole by each change, empty as `None`.
#[derive(Default)]
pub struct IfgMembers(pub(crate) Option<hal::malloc::MArc<MArray<&'static Ifnet>>>);

impl IfgMembers {
    /// The empty list.
    pub const fn new() -> Self {
        Self(None)
    }

    /// The same array, one more reference to it (as [`IfgSet::snapshot`]).
    #[must_use]
    pub fn snapshot(&self) -> Self {
        Self(self.0.clone())
    }

    /// The members, in order.
    pub fn iter(&self) -> impl Iterator<Item = &'static Ifnet> + '_ {
        self.as_slice().iter().copied()
    }

    /// The members as a slice.
    pub fn as_slice(&self) -> &[&'static Ifnet] {
        self.0.as_deref().map_or(&[], |a| a)
    }
}

/// `if_rxr_put(r, c)`: `c` slots of the ring were consumed.
pub fn if_rxr_put(r: &mut IfRxring, c: u32) {
    r.rxr_alive -= c;
}

/// `if_rxr_needrefill(r)`: the ring is below its low watermark.
pub fn if_rxr_needrefill(r: &IfRxring) -> bool {
    r.rxr_alive < r.rxr_lwm
}

/// `if_rxr_inuse(r)`.
pub fn if_rxr_inuse(r: &IfRxring) -> u32 {
    r.rxr_alive
}

/// `if_rxr_cwm(r)`.
pub fn if_rxr_cwm(r: &IfRxring) -> u32 {
    r.rxr_cwm
}

/// `hal`'s one `SaToken`, for the kernel's tests (whose binary has no `bsd` to take it at
/// boot): they publish their interface addresses as `bsd`'s protocols do.
#[cfg(test)]
pub(crate) fn sa_test_token() -> &'static hal::link::SaToken {
    static TOKEN: std::sync::OnceLock<&'static hal::link::SaToken> = std::sync::OnceLock::new();
    TOKEN.get_or_init(|| match hal::link::SaToken::take() {
        Some(t) => t,
        None => panic!("the SaToken was taken outside sa_test_token"),
    })
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_atomic_cell_is_a_cell_of_its_integer() {
        let c = AtomicCell::<i32>::new();
        assert_eq!(c.get(), 0);
        c.set(0x43);
        assert_eq!(c.get(), 0x43);
        let u = AtomicCell::<u8>::new();
        u.set(255);
        assert_eq!(u.get(), 255);
        fn shared<T: Send + Sync>() {}
        shared::<AtomicCell<u64>>();
        shared::<Ifnet>();
        shared::<Ifaddr>();
        shared::<Ifmaddr>();
    }

    #[test]
    fn a_name_is_written_and_read_whole() {
        let n = IfName::new();
        assert_eq!(n.with_str(<[u8]>::len), 0);
        let mut b = [0u8; IFNAMSIZ];
        b[..4].copy_from_slice(b"vio0");
        n.set(b);
        assert_eq!(n.get(), b);
        assert_eq!(n.with_str(<[u8]>::to_vec), b"vio0");
        let full = [b'x'; IFNAMSIZ];
        n.set(full);
        assert_eq!(n.with_str(<[u8]>::len), IFNAMSIZ);
    }

    #[test]
    fn counters_are_off_until_allocated_and_zeroed_when_turned_on() {
        let ifp = Ifnet::new();
        assert!(ifp.if_counters.get().is_none());
        ifp.if_counters.turn(true);
        let c = ifp.if_counters.get().unwrap();
        counters_pkt(c, IfCounters::IfcIpackets, IfCounters::IfcIbytes, 1500);
        counters_inc(c, IfCounters::IfcIerrors);
        assert_eq!(
            c[IfCounters::IfcIpackets as usize].load(Ordering::Relaxed),
            1
        );
        assert_eq!(
            c[IfCounters::IfcIbytes as usize].load(Ordering::Relaxed),
            1500
        );
        ifp.if_counters.turn(false);
        assert!(ifp.if_counters.get().is_none());
        ifp.if_counters.turn(true);
        let c = ifp.if_counters.get().unwrap();
        assert_eq!(
            c[IfCounters::IfcIpackets as usize].load(Ordering::Relaxed),
            0
        );
        ifp.if_ierrors().set(ifp.if_ierrors().get() + 1);
        assert_eq!(
            ifp.if_data_counters[IfCounters::IfcIerrors as usize].get(),
            1
        );
        ifp.if_lastchange.set(10, 20);
        assert_eq!(ifp.if_lastchange.get(), (10, 20));
    }

    /// An address its owner embeds (enc(4)'s): counted by its own `ifa_refcnt` (the owner's
    /// reference and one per handle), its address read through its link under the net lock.
    #[test]
    fn a_linked_address_is_counted_by_its_own_refcnt() {
        let ifa: &'static IfaddrLinked =
            std::boxed::Box::leak(std::boxed::Box::new(IfaddrLinked::new()));
        ifa.ifa_refcnt.init();
        let l = IfaddrRef::Linked(LinkedIfa::new(ifa));
        assert_eq!(l.count(), 2, "the owner's and the handle's");
        let c = l.clone();
        assert_eq!(l.count(), 3);
        drop(c);
        assert_eq!(l.count(), 2);
        assert!(l.is(&ifa.ifa));
        let sdl: &'static core::cell::Cell<[u8; 8]> =
            std::boxed::Box::leak(std::boxed::Box::new(core::cell::Cell::new([
                8, 18, 1, 0, 0, 0, 0, 0,
            ])));
        ifa.ifa_addr.set(sa_test_token(), Some(sdl));
        {
            let w = hal::netlock::NETLOCK.read();
            let net = NetHeld::of(&w).expect("the net lock's witness");
            assert_eq!(
                l.ifa_addr(&net).map(|b| (b.family(), b.len())),
                Some((18, 8))
            );
            assert!(l.ifa_netmask(&net).is_none() && l.ifa_dstaddr(&net).is_none());
        }
        drop(l);
        assert_eq!(ifa.ifa_refcnt.read(), 1, "the owner's alone");
    }

    /// A handle that sees a linked address's count reach zero: `panic(9)` (the owner gives
    /// its reference back last, `enc_clone_destroy`; the C would `free` a part of a softc).
    #[test]
    #[should_panic(expected = "the last reference of a linked address")]
    fn a_linked_address_s_last_reference_is_its_owner_s() {
        let ifa: &'static IfaddrLinked =
            std::boxed::Box::leak(std::boxed::Box::new(IfaddrLinked::new()));
        ifa.ifa_refcnt.init();
        let l = LinkedIfa::new(ifa);
        assert!(!ifa.ifa_refcnt.rele(), "the handle's left");
        drop(l);
    }

    #[test]
    fn a_new_interface_is_not_attached() {
        let ifp = Ifnet::new();
        assert!(ifp.rt.get().is_none() && ifp.if_driver.get().is_none());
        assert!(ifp.sadl().is_none() && ifp.if_arpcom.get().is_none());
        assert!(ifp.if_link.get().is_none());
        assert!(!List::<IfnetList>::is_linked(&ifp));
        let ifa = IfaddrLinked::new();
        assert!(ifa.ifa_addr.is_none() && ifa.ifa_dstaddr.is_none());
        assert!(ifa.ifa_netmask.is_none() && ifa.ifa.ifa_ifp.get().is_none());
        // The owner's one reference (`refcnt_init`, enc_clone_create's), no flags.
        assert_eq!((ifa.ifa.ifa_flags.get(), ifa.ifa_refcnt.read()), (0, 1));
    }

    #[test]
    fn the_rx_ring_helpers_read_the_ring() {
        let mut r = IfRxring {
            rxr_alive: 10,
            rxr_lwm: 4,
            rxr_cwm: 8,
            ..IfRxring::default()
        };
        assert!(!if_rxr_needrefill(&r));
        if_rxr_put(&mut r, 7);
        assert!(if_rxr_needrefill(&r));
        assert_eq!((if_rxr_inuse(&r), if_rxr_cwm(&r)), (3, 8));
    }
}
/* </TESTS> */
