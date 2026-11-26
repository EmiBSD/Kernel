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
 * Copyright (c) 2002 Cedric Berger
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 *    - Redistributions of source code must retain the above copyright
 *      notice, this list of conditions and the following disclaimer.
 *    - Redistributions in binary form must reproduce the above
 *      copyright notice, this list of conditions and the following
 *      disclaimer in the documentation and/or other materials provided
 *      with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT HOLDERS OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 *
 */
/* </LICENSES> */

/* <CODE> */
//! pf's address tables (`pfctl -t`, `<table>` in rules): sets of IPv4 and IPv6 addresses and
//! networks, kept in radix trees for longest-prefix matches.
//!
//! Upstream: sys/net/pf_table.c @ 3ce1f3f79392
//!
//! A table (`struct pfr_ktable`) is an item of `pfr_ktable_pl` in the global tree
//! `pfr_ktables`, ordered by name then anchor; it holds two radix trees (`pfrkt_ip4`,
//! `pfrkt_ip6`) of entries (`struct pfr_kentry`, items of `pfr_kentry_pl[type]`): since N6.7b1
//! the kernel's radix tree (`kernel::net::radix`, `PfrRadix` in `net/pfvar.rs`), which owns
//! its nodes and keys and carries each route's entry, so no leaf is cast back to the entry
//! that embedded it. Transactions build a
//! shadow table (`pfr_ina_define`) that `pfr_ina_commit` merges or swaps in; an anchored table
//! refers to the root table of the same name (`pfrkt_root`), which rules in the anchor fall back
//! to when the anchored table is not active.
//!
//! The ioctl functions take their arrays as a [`PfrBuf`]: the C's `COPYIN`/`COPYOUT` choose
//! `copyin`/`copyout` at a user address when the flags have `PFR_FLAG_USERIOCTL` and `bcopy` to
//! or from a kernel array otherwise; the buffer is the user address (`PfrBuf::User`) or the
//! kernel array (`PfrBuf::Kernel`) itself, and the flags still pick the path.
//!
//! ## Deviations
//! - `COPYIN`/`COPYOUT` through a [`PfrBuf`]: a buffer of the other kind than the flags ask for,
//!   or an index past the end of a kernel array, is `EFAULT` (the C would `bcopy` from the user
//!   address, or `copyin` from a kernel address, or run off the array).
//! - `pfr_walktree`'s `struct pfr_walktree` is [`PfrWalktree`], its union an enum; `rn_walktree`
//!   is the tree's cursor (`pfr_walk`), which lends the tree for each step only, the successor
//!   taken before `pfr_walktree` runs, as the C's. `PFRW_POOL_GET`'s "finish search" return of 1
//!   is `Err(EPERM)` (`EPERM` is 1), which `pfr_kentry_byidx` ignores as the C does.
//! - `pfr_kentry_all`'s `pfrke_rkif`/`pfrke_rifname` are reached through `pfr_kentry_route`, and
//!   the cost entry's weight through `pfr_kentry_cost`, after the entry's type.
//!   `pfr_fill_feedback` reads the weight of every entry as a `pfr_kentry_cost`, past the end of
//!   plain and route entries; here only cost entries report their weight, the others 0.
//! - The `sockaddr_in` and `sockaddr_in6` keys built on the stack (`tmp4`/`tmp6` in
//!   `pfr_kentry_byaddr`, `pfr_update_stats`, `pfr_pool_get`) are `pfsockaddr_union`s, so that
//!   the radix code may read them for its whole key length (`sizeof(struct sockaddr_in6)`).
//!   `pfr_pool_get`'s `addr`, a `struct pf_addr *` into `tmp4.sin_addr` or `tmp6.sin6_addr`, is
//!   a local address written into the key before each lookup.
//! - `RB_FIND` with a `struct pfr_table` cast to a `struct pfr_ktable` (`pfr_lookup_table`,
//!   the `key` tables of `pfr_add_tables`, `pfr_del_tables`, `pfr_ina_define`, ...) is a
//!   descent of `pfr_ktables` comparing the table's name and anchor as `pfr_ktable_compare`
//!   does; the key tables are heap copies of a `struct pfr_table`.
//! - Out-pointers: `int *` counts are `Option<&mut i32>` (NULL is `None`), in-out sizes
//!   `&mut i32`; `pfr_pool_get`'s `raddr`/`rmask` (pointers into the entry and into
//!   `pfr_mask`) are returned as copies, `Err(1)` and `Err(-1)` being the C's 1 and -1.
//! - Functions whose C `int` is 0 or -1 but not an errno (`pfr_validate_addr`,
//!   `pfr_route_kentry`, `pfr_unroute_kentry`) return `bool`, `true` for the C's 0; the
//!   predicates `pfr_skip_table`, `pfr_islinklocal` and `pfr_match_addr` return `true` where
//!   the C returns nonzero. `pfr_insert_kentry` returns `pfr_route_kentry`'s -1 as the C does,
//!   which is `ERESTART`'s value; like the C, it does not free the entry it could not route.
//! - `pfr_insert_kentry` and `pfr_remove_kentry` take `&PfrAddr` (pf.c's callers); the C's
//!   `pfr_create_kentry` writes the implicit weight into the caller's `pfr_addr`, here into a
//!   copy.
//! - Where the C dereferences a pointer it takes to be set (the shadow of a table being
//!   committed, the entry `rn_match` finds inside a block), a NULL one panics. A table's two
//!   trees are its own members, made with the table (the C's `rn_inithead` heads).
//! - Each of a table's trees is inside a lock of its own, which the C does not have
//!   (`hal::rwlock::Rwlocked`, "pfrktip4" and "pfrktip6"; the user's decision of 2026-11-24),
//!   because the C's readers race its writers: `pfr_update_stats` looks its entry up after
//!   `PF_UNLOCK`, under the shared net lock alone, on every softnet at once, while the state
//!   purge (`PF_LOCK`, no net lock) deletes entries and the packet path adds and deletes them
//!   (`pf_source_used` and `pf_source_rele` under `PF_LOCK`, `pf_src_connlimit`'s overload
//!   insert from `pf_test_state`, without it). Every lookup (`pfr_kentry_byaddr`, so
//!   `pfr_match_addr`, and `pfr_update_stats`, `pfr_lookup_addr`, `pfr_lookup_kentry`,
//!   `pfr_pool_get`, each step of a walk) takes it shared; `pfr_route_kentry`,
//!   `pfr_unroute_kentry`, the swap of `pfr_commit_ktable` and `pfr_destroy_ktable` take it
//!   alone; a new table's roots are made before the table is shared, without it. The lock is a
//!   leaf: held for the tree's own operation only, no other lock taken under it, never across a
//!   sleep, a copyout, a free, a walk's callback or `pf_unlock`; the one allocation under it is
//!   the tree's `M_NOWAIT` growth. A lookup costs the shared enter and exit (two atomic
//!   exchanges, no spl) the C does not pay.
//! - A lookup whose caller reads or counts on the entry it found keeps the tree's lock shared
//!   until that last use (`pfr_match_kentry`, `pfr_kentry_byaddr_with`): `pfr_update_stats`'s
//!   negation test and counters (their first `pfrke_counters` allocation is `PR_NOWAIT`, as
//!   the C's), `pfr_match_addr`'s negation, `pfr_states_increase` and `pfr_states_decrease`,
//!   `pfr_lookup_addr`'s and `pfr_lookup_kentry`'s network test, `pfr_pool_get`'s nested
//!   block. The C dereferences the entry after `rn_match` with nothing to keep it, so the
//!   purge (`PF_LOCK` only) may free an entry `pfr_update_stats` (shared net lock only) is
//!   counting on; here the delete waits for the lock. The debug lines are printed after it.
//!   What still outlives the lock is used under `pf_lock`, which every delete holds: the entry
//!   `pfr_kentry_byidx`'s walk answers `pfr_pool_get`, and `pfr_kentry_byaddr`'s answer to
//!   `pf_map_addr_sticky`, which only tests it.
//! - `pfr_kentry_pl` is an array of `PFRKE_MAX` pools, as in C; `pf_pool_limits` names its
//!   first pool (`&pfr_kentry_pl` in C is the array's address, `pfr_kentry_pl[PFRKE_PLAIN]`).

use core::cell::Cell;
use core::cmp::Ordering;
use core::mem::{offset_of, size_of};
use core::ptr::{self, NonNull};

use crate::kern::kern_tc::gettime;
use crate::kern::sched_bsd::preempt;
use crate::kern::subr_pool::{pool_get, pool_init, pool_put};
use crate::kern::subr_prf::panic;
use crate::machine::copy::{copyin, copyout};
use crate::machine::intr::IPL_SOFTNET;
use crate::net::if_::unhandled_af;
use crate::net::pf::{pf_addr_inc, pf_addrcpy, pf_match_addr, pf_poolmask};
use crate::net::pf_if::{pfi_kif_get, pfi_kif_ref, pfi_kif_unref};
use crate::net::pf_ruleset::{
    pf_find_or_create_ruleset, pf_find_ruleset, pf_main_ruleset, pf_remove_if_empty_ruleset,
};
use crate::net::pfvar::{
    PF_ADDR_DYNIFTL, PF_ADDR_TABLE, PF_DROP, PF_MATCH, PF_OUT, PF_PASS, PF_RESERVED_ANCHOR,
    PF_TABLE_NAME_SIZE, PFI_KIF_REF_ROUTE, PFR_DIR_MAX, PFR_FB_ADDED, PFR_FB_CHANGED,
    PFR_FB_CLEARED, PFR_FB_CONFLICT, PFR_FB_DELETED, PFR_FB_DUPLICATE, PFR_FB_MATCH,
    PFR_FB_NOCOUNT, PFR_FB_NONE, PFR_FB_NOTMATCH, PFR_FLAG_ADDRSTOO, PFR_FLAG_ALLMASK,
    PFR_FLAG_ALLRSETS, PFR_FLAG_CLSTATS, PFR_FLAG_DUMMY, PFR_FLAG_FEEDBACK, PFR_FLAG_REPLACE,
    PFR_FLAG_USERIOCTL, PFR_OP_ADDR_MAX, PFR_OP_BLOCK, PFR_OP_MATCH, PFR_OP_PASS, PFR_OP_TABLE_MAX,
    PFR_OP_XPASS, PFR_REFCNT_ANCHOR, PFR_REFCNT_RULE, PFR_TFLAG_ACTIVE, PFR_TFLAG_CONST,
    PFR_TFLAG_COUNTERS, PFR_TFLAG_INACTIVE, PFR_TFLAG_PERSIST, PFR_TFLAG_REFDANCHOR,
    PFR_TFLAG_REFERENCED, PFR_TFLAG_SETMASK, PFR_TFLAG_USRMASK, PFRKE_COST, PFRKE_FLAG_MARK,
    PFRKE_FLAG_NOT, PFRKE_MAX, PFRKE_PLAIN, PFRKE_ROUTE, PfAddr, PfPool, PfPoolItem, PfRuleset,
    PfiDynaddr, PfrAddr, PfrAstats, PfrKcounters, PfrKentry, PfrKentryCost, PfrKentryRoute,
    PfrKentryioq, PfrKentryworkq, PfrKtable, PfrKtableworkq, PfrRadix, PfrTable, PfrTstats,
    PfsockaddrUnion, pf_abi_zeroed, pf_azero, pf_cstr, pf_pool_get, pf_pool_init, pf_pool_put,
};
use crate::net::pfvar_priv::{PfGlobal, PfPdesc, pf_assert_locked, pf_lock, pf_unlock};
use crate::netinet::in_::{InAddr, SockaddrIn};
use crate::netinet6::in6::SockaddrIn6;
use crate::sys::errno::Errno;
use crate::sys::malloc::M_RTABLE;
use crate::sys::pool::{PR_LIMITFAIL, PR_NOWAIT, PR_WAITOK, PR_ZERO, Pool};
use crate::sys::queue::{SlistAdapter, SlistHead};
use crate::sys::sched::sched_pause;
use crate::sys::socket::{AF_INET, AF_UNSPEC};
use crate::sys::syslimits::PATH_MAX;
use crate::sys::syslog::{LOG_DEBUG, LOG_ERR, LOG_NOTICE};
use crate::sys::systm::{net_assert_locked, net_lock, net_unlock};
use crate::sys::tree::RbHead;
use crate::sys::types::{SaFamily, Time};
use hal::rwlock::{ReadGuard, Rwlocked};
use libkern::{strlcpy, strnlen};

#[cfg(feature = "inet6")]
use crate::{netinet6::in6::In6Addr, sys::socket::AF_INET6};

/// `NO_ADDRESSES`: a shadow table's `pfrkt_cnt` when the transaction gave no addresses.
const NO_ADDRESSES: i32 = -1;
/// `ENQUEUE_UNMARKED_ONLY`.
const ENQUEUE_UNMARKED_ONLY: bool = true;
/// `INVERT_NEG_FLAG`.
const INVERT_NEG_FLAG: bool = true;

/// A `struct pfr_addr *`, `struct pfr_table *`, ... array argument: a user address
/// (`PFR_FLAG_USERIOCTL`, `copyin`/`copyout`) or a kernel array (`bcopy`).
pub enum PfrBuf<'a, T> {
    /// The array at this user address.
    User(usize),
    /// This kernel array.
    Kernel(&'a mut [T]),
}

/// `enum pfrw_op`: what `pfr_walktree` does with each entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PfrwOp {
    /// `PFRW_MARK`.
    Mark,
    /// `PFRW_SWEEP`.
    Sweep,
    /// `PFRW_ENQUEUE`.
    Enqueue,
    /// `PFRW_GET_ADDRS`.
    GetAddrs,
    /// `PFRW_GET_ASTATS`.
    GetAstats,
    /// `PFRW_POOL_GET`.
    PoolGet,
    /// `PFRW_DYNADDR_UPDATE`.
    DynaddrUpdate,
}

/// The union `pfrw_1` of `struct pfr_walktree`; the array pointers are a buffer and the index
/// of the next element.
enum Pfrw1<'a, 'b> {
    /// Nothing (`bzero`).
    None,
    /// `pfrw_addr`.
    Addr(&'a mut PfrBuf<'b, PfrAddr>, usize),
    /// `pfrw_astats`.
    Astats(&'a mut PfrBuf<'b, PfrAstats>, usize),
    /// `pfrw_workq`.
    Workq(&'a SlistHead<PfrKentryworkq>),
    /// `pfrw_kentry`.
    Kentry(Option<&'static PfrKentry>),
    /// `pfrw_dyn`.
    Dyn(&'static PfiDynaddr),
}

/// `struct pfr_walktree`: the argument of `pfr_walktree`.
struct PfrWalktree<'a, 'b> {
    /// `pfrw_op`.
    pfrw_op: PfrwOp,
    /// `pfrw_1`.
    pfrw_1: Pfrw1<'a, 'b>,
    /// `pfrw_free`, also `pfrw_cnt`.
    pfrw_free: i32,
    /// `pfrw_flags`.
    pfrw_flags: i32,
}

impl PfrWalktree<'_, '_> {
    /// `bzero(&w, sizeof(w)); w.pfrw_op = op;`.
    fn new(op: PfrwOp) -> Self {
        Self {
            pfrw_op: op,
            pfrw_1: Pfrw1::None,
            pfrw_free: 0,
            pfrw_flags: 0,
        }
    }
}

crate::tree_adapter!(
    /// `RB_HEAD(pfr_ktablehead, pfr_ktable)`, `RB_GENERATE(pfr_ktablehead, ...)`.
    pub PfrKtablehead: PfrKtable, pfrkt_tree => crate::sys::tree::RbEntry<PfrKtable>,
    pfr_ktable_compare
);

/// `pfr_ktable_pl`.
pub static PFR_KTABLE_PL: Pool = Pool::new();
/// `pfr_kentry_pl[PFRKE_MAX]`: plain, route and cost entries.
pub static PFR_KENTRY_PL: [Pool; PFRKE_MAX as usize] = [Pool::new(), Pool::new(), Pool::new()];
/// `pfr_kcounters_pl`.
pub static PFR_KCOUNTERS_PL: Pool = Pool::new();

// SAFETY: list links, `Cell`s of integers, of a byte union and of `Option<&T>`: the all-zero
// pattern is valid.
unsafe impl PfPoolItem for PfrKentry {}
// SAFETY: as for `PfrKentry`, plus a `Cell<Option<&T>>` and a byte array.
unsafe impl PfPoolItem for PfrKentryRoute {}
// SAFETY: as for `PfrKentryRoute`, plus a `Cell<u16>`.
unsafe impl PfPoolItem for PfrKentryCost {}
// SAFETY: `Cell`s of integers.
unsafe impl PfPoolItem for PfrKcounters {}

/// `pfr_mask`: the mask `pfr_pool_get` hands out.
pub static PFR_MASK: PfGlobal<Cell<PfsockaddrUnion>> =
    PfGlobal(Cell::new(PfsockaddrUnion { bytes: [0; 28] }));
/// `pfr_ffaddr`: all ones, set by `pfr_initialize`.
pub static PFR_FFADDR: PfGlobal<Cell<PfAddr>> = PfGlobal(Cell::new(PfAddr::zeroed()));

/// `pfr_ktables`: every table, by name and anchor.
pub static PFR_KTABLES: PfGlobal<RbHead<PfrKtablehead>> = PfGlobal(RbHead::new());
/// `pfr_nulltable`: the empty description of the temporary tables.
pub static PFR_NULLTABLE: PfGlobal<PfrTable> = PfGlobal(PfrTable::zeroed());
/// `pfr_ktable_cnt`: the number of tables in `pfr_ktables`.
pub static PFR_KTABLE_CNT: PfGlobal<Cell<i32>> = PfGlobal(Cell::new(0));

/// An element type of a [`PfrBuf`]: moved as its bytes.
///
/// # Safety
///
/// Implement only for `#[repr(C)]` types without implicit padding whose every bit pattern is
/// valid (the promise of `AbiPod` and `PfAbi`).
pub unsafe trait PfrIo: Sized {}

// SAFETY: `PfrAddr: AbiPod`.
unsafe impl PfrIo for PfrAddr {}
// SAFETY: `PfrAstats: AbiPod`.
unsafe impl PfrIo for PfrAstats {}
// SAFETY: `PfrTable: PfAbi`.
unsafe impl PfrIo for PfrTable {}
// SAFETY: `PfrTstats: PfAbi`.
unsafe impl PfrIo for PfrTstats {}

/// `ACCEPT_FLAGS(flags, oklist)`: `EINVAL` for a flag outside `oklist`.
fn accept_flags(flags: i32, oklist: i32) -> Result<(), Errno> {
    if (flags & !oklist) & PFR_FLAG_ALLMASK != 0 {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// The bytes of `v`.
fn io_bytes<T: PfrIo>(v: &T) -> &[u8] {
    // SAFETY: `T: PfrIo` has no implicit padding, so every byte is initialised; nothing
    // changes `v` while the slice is borrowed (pf's locks, or the caller owns `v`).
    unsafe { core::slice::from_raw_parts(ptr::from_ref(v).cast::<u8>(), size_of::<T>()) }
}

/// The bytes of `v`, writable.
fn io_bytes_mut<T: PfrIo>(v: &mut T) -> &mut [u8] {
    // SAFETY: as for `io_bytes`; every bit pattern is a valid `T` (`PfrIo`), so any bytes
    // written leave a valid value.
    unsafe { core::slice::from_raw_parts_mut(ptr::from_mut(v).cast::<u8>(), size_of::<T>()) }
}

/// The user address of element `i` of the array at `base`.
fn io_uaddr<T>(base: usize, i: usize) -> Result<usize, Errno> {
    i.checked_mul(size_of::<T>())
        .and_then(|off| base.checked_add(off))
        .ok_or(Errno::EFAULT)
}

/// `COPYIN(from + i, to, sizeof(*to), flags)`.
fn pfr_copyin<T: PfrIo>(
    from: &PfrBuf<'_, T>,
    i: usize,
    to: &mut T,
    flags: i32,
) -> Result<(), Errno> {
    match (flags & PFR_FLAG_USERIOCTL != 0, from) {
        (true, PfrBuf::User(base)) => copyin(io_uaddr::<T>(*base, i)?, io_bytes_mut(to)),
        (false, PfrBuf::Kernel(a)) => {
            let src = a.get(i).ok_or(Errno::EFAULT)?;
            io_bytes_mut(to).copy_from_slice(io_bytes(src));
            Ok(())
        }
        _ => Err(Errno::EFAULT),
    }
}

/// `COPYOUT(from, to + i, sizeof(*from), flags)`.
fn pfr_copyout<T: PfrIo>(
    from: &T,
    to: &mut PfrBuf<'_, T>,
    i: usize,
    flags: i32,
) -> Result<(), Errno> {
    match (flags & PFR_FLAG_USERIOCTL != 0, to) {
        (true, PfrBuf::User(base)) => copyout(io_bytes(from), io_uaddr::<T>(*base, i)?),
        (false, PfrBuf::Kernel(a)) => {
            let dst = a.get_mut(i).ok_or(Errno::EFAULT)?;
            io_bytes_mut(dst).copy_from_slice(io_bytes(from));
            Ok(())
        }
        _ => Err(Errno::EFAULT),
    }
}

/// `YIELD(ok)`.
fn pfr_yield(ok: bool) {
    if ok {
        sched_pause(preempt);
    }
}

/// `flags & PFR_FLAG_USERIOCTL ? PR_WAITOK : PR_NOWAIT`.
fn pfr_wait(flags: i32) -> i32 {
    if flags & PFR_FLAG_USERIOCTL != 0 {
        PR_WAITOK
    } else {
        PR_NOWAIT
    }
}

/// `FILLIN_SIN(sin, addr)` on a zeroed union.
fn fillin_sin(su: &mut PfsockaddrUnion, addr: InAddr) {
    let mut sin = su.sin();
    sin.sin_len = size_of::<SockaddrIn>() as u8;
    sin.sin_family = AF_INET;
    sin.sin_addr = addr;
    su.set_sin(&sin);
}

/// `FILLIN_SIN6(sin6, addr)` on a zeroed union.
#[cfg(feature = "inet6")]
fn fillin_sin6(su: &mut PfsockaddrUnion, addr: In6Addr) {
    let mut sin6 = su.sin6();
    sin6.sin6_len = size_of::<SockaddrIn6>() as u8;
    sin6.sin6_family = AF_INET6;
    sin6.sin6_addr = addr;
    su.set_sin6(&sin6);
}

/// `SUNION2PF(su, af)`: the `pf_addr` at the union's address (`sin_addr` or `sin6_addr`).
fn sunion2pf(su: &PfsockaddrUnion, af: SaFamily) -> PfAddr {
    let off = if af == AF_INET {
        offset_of!(SockaddrIn, sin_addr)
    } else {
        offset_of!(SockaddrIn6, sin6_addr)
    };
    let mut a = PfAddr::zeroed();
    a.addr8.copy_from_slice(&su.bytes[off..off + 16]);
    a
}

/// `AF_BITS(af)`.
fn af_bits(af: u8) -> u8 {
    if af == AF_INET { 32 } else { 128 }
}

/// `ADDR_NETWORK(ad)`.
fn addr_network(ad: &PfrAddr) -> bool {
    ad.pfra_net < af_bits(ad.pfra_af)
}

/// `KENTRY_NETWORK(ke)`.
fn kentry_network(ke: &PfrKentry) -> bool {
    ke.pfrke_net.get() < af_bits(ke.pfrke_af.get())
}

/// `SLIST_FIRST` of a work queue of this file: a pool item (entry or table), which stays
/// allocated until `pfr_destroy_kentry`/`pfr_destroy_ktable`, after it is unlinked.
fn sl_first<A: SlistAdapter>(q: &SlistHead<A>) -> Option<&'static A::Elem>
where
    A::Elem: 'static,
{
    // SAFETY: see the function's doc: the element outlives every queue it is on.
    q.first().map(|e| unsafe { &*ptr::from_ref(e) })
}

/// `SLIST_FOREACH_SAFE` over a work queue of this file (the successor is read first).
fn sl_iter<A: SlistAdapter>(q: &SlistHead<A>) -> impl Iterator<Item = &'static A::Elem>
where
    A::Elem: 'static,
{
    let mut p = sl_first(q);
    core::iter::from_fn(move || {
        let e = p?;
        p = SlistHead::<A>::next(e);
        Some(e)
    })
}

/// `pfrke_rkif`/`pfrke_rifname`: the route part of a route or cost entry.
fn pfr_kentry_route(ke: &'static PfrKentry) -> &'static PfrKentryRoute {
    let t = ke.pfrke_type.get();
    if t != PFRKE_ROUTE && t != PFRKE_COST {
        panic(format_args!("pfr_kentry_route: type {t}"));
    }
    // SAFETY: an entry of type ROUTE or COST is an item of `pfr_kentry_pl[ROUTE]` or
    // `pfr_kentry_pl[COST]`, a `PfrKentryRoute` or a `PfrKentryCost`, both `#[repr(C)]` and
    // starting with the entry, then `kif` and `ifname` ("above overlaps with pfr_kentry
    // route").
    unsafe { &*ptr::from_ref(ke).cast::<PfrKentryRoute>() }
}

/// `(struct pfr_kentry_cost *)ke`: the cost entry `ke` is.
fn pfr_kentry_cost(ke: &'static PfrKentry) -> &'static PfrKentryCost {
    let t = ke.pfrke_type.get();
    if t != PFRKE_COST {
        panic(format_args!("pfr_kentry_cost: type {t}"));
    }
    // SAFETY: an entry of type COST is an item of `pfr_kentry_pl[COST]`, a `#[repr(C)]`
    // `PfrKentryCost` starting with the entry.
    unsafe { &*ptr::from_ref(ke).cast::<PfrKentryCost>() }
}

/// The key the radix trees hold for `ke`: its `pfrke_sa`, the bytes the tree copies.
fn ke_key(ke: &PfrKentry) -> [u8; size_of::<PfsockaddrUnion>()] {
    ke.pfrke_sa.get().bytes
}

/// The radix tree `h` of a table (`kt->pfrkt_ip4` or `pfrkt_ip6`), its lock taken shared for a
/// lookup or one step of a walk. The guard lives until the caller's last use of the entry the
/// tree answered, so no delete (`pfr_unroute_kentry`, which takes the lock alone before the
/// entry is freed) can free it meanwhile. The lock is a leaf: while the guard lives nothing
/// takes another lock (but an `M_NOWAIT` pool allocation's, as the tree's own growth),
/// sleeps, copies out or frees.
fn pfr_rnh(h: &Rwlocked<PfrRadix>) -> ReadGuard<'_, PfrRadix> {
    h.read()
}

/// The tree of the family `af` in `kt` (`pfrkt_ip4` or `pfrkt_ip6`), unlocked, for a caller that
/// prepares its key and mask before it takes the lock.
fn pfr_tree(kt: &'static PfrKtable, af: SaFamily) -> &'static Rwlocked<PfrRadix> {
    match af {
        AF_INET => &kt.pfrkt_ip4,
        #[cfg(feature = "inet6")]
        AF_INET6 => &kt.pfrkt_ip6,
        af => unhandled_af(i32::from(af)),
    }
}

/// An empty tree of `sockaddr_in` (`ip6`: `sockaddr_in6`) keys, compared from the address
/// (`rn_inithead(&kt->pfrkt_ip4, offsetof(struct sockaddr_in, sin_addr))`), its blocks of
/// `M_RTABLE`; nothing allocated until its roots are made.
fn pfr_radix(ip6: bool) -> PfrRadix {
    let off = if ip6 {
        offset_of!(SockaddrIn6, sin6_addr)
    } else {
        offset_of!(SockaddrIn, sin_addr)
    };
    PfrRadix::new(off, M_RTABLE)
}

/// `pfr_gcd`.
pub fn pfr_gcd(mut m: i32, mut n: i32) -> i32 {
    while m > 0 {
        let t = n % m;
        n = m;
        m = t;
    }
    n
}

/// `pfr_initialize`: the pools and `pfr_ffaddr`. The C's `rn_init(sizeof(struct
/// sockaddr_in6))` is each tree's own key length (`PfrRadix`, N6.7b1). A table is no
/// `PfPoolItem` (its trees are not valid all-zero): `pfr_create_ktable` writes them.
pub fn pfr_initialize() {
    pool_init(
        &PFR_KTABLE_PL,
        size_of::<PfrKtable>(),
        align_of::<PfrKtable>() as u32,
        IPL_SOFTNET,
        0,
        "pfrktable",
        None,
    );
    pf_pool_init::<PfrKentry>(
        &PFR_KENTRY_PL[PFRKE_PLAIN as usize],
        IPL_SOFTNET,
        0,
        "pfrke_plain",
    );
    pf_pool_init::<PfrKentryRoute>(
        &PFR_KENTRY_PL[PFRKE_ROUTE as usize],
        IPL_SOFTNET,
        0,
        "pfrke_route",
    );
    pf_pool_init::<PfrKentryCost>(
        &PFR_KENTRY_PL[PFRKE_COST as usize],
        IPL_SOFTNET,
        0,
        "pfrke_cost",
    );
    pf_pool_init::<PfrKcounters>(&PFR_KCOUNTERS_PL, IPL_SOFTNET, 0, "pfrkcounters");

    PFR_FFADDR.set(PfAddr { addr8: [0xff; 16] });
}

/// `pfr_clr_addrs`: removes every address of the table `tbl`; the count in `ndel`.
pub fn pfr_clr_addrs(tbl: &mut PfrTable, ndel: Option<&mut i32>, flags: i32) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKentryworkq>::new();

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    pfr_validate_table(tbl, 0, flags & PFR_FLAG_USERIOCTL != 0)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };
    if kt.pfrkt_flags().get() & PFR_TFLAG_CONST != 0 {
        return Err(Errno::EPERM);
    }
    pfr_enqueue_addrs(kt, &workq, ndel, false);

    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_remove_kentries(kt, &workq);
        if kt.pfrkt_cnt().get() != 0 {
            crate::dpfprintf!(
                LOG_NOTICE,
                "pfr_clr_addrs: corruption detected ({}).",
                kt.pfrkt_cnt().get()
            );
            kt.pfrkt_cnt().set(0);
        }
    }
    Ok(())
}

/// `pfr_fill_feedback`: the entry `ke` as `pfr_add_addrs` reports it back in `ad`.
pub fn pfr_fill_feedback(ke: &'static PfrKentry, ad: &mut PfrAddr) {
    ad.pfra_type = ke.pfrke_type.get();

    let t = ke.pfrke_type.get();
    if t == PFRKE_COST || t == PFRKE_ROUTE {
        if t == PFRKE_COST {
            pfr_kentry_cost(ke).weight.set(ad.pfra_weight);
        }
        let ifname = pfr_kentry_route(ke).ifname.get();
        if ifname[0] != 0 {
            strlcpy(&mut ad.pfra_ifname, &ifname);
        }
    }

    match ke.pfrke_af.get() {
        AF_INET => ad.pfra_u.set_v4(ke.pfrke_sa.get().sin().sin_addr),
        #[cfg(feature = "inet6")]
        AF_INET6 => ad.pfra_u.set_v6(ke.pfrke_sa.get().sin6().sin6_addr),
        af => unhandled_af(i32::from(af)),
    }
    ad.pfra_weight = if ke.pfrke_type.get() == PFRKE_COST {
        pfr_kentry_cost(ke).weight.get()
    } else {
        0
    };
    ad.pfra_af = ke.pfrke_af.get();
    ad.pfra_net = ke.pfrke_net.get();
    if ke.pfrke_flags.get() & PFRKE_FLAG_NOT != 0 {
        ad.pfra_not = 1;
    }
    ad.pfra_fback = ke.pfrke_fb.get();
}

/// `pfr_add_addrs`: adds `size` addresses from `addr` to the table `tbl`; the count of added
/// ones in `nadd`, per-address feedback written back into `addr` with `PFR_FLAG_FEEDBACK`.
pub fn pfr_add_addrs(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAddr>,
    size: i32,
    nadd: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKentryworkq>::new();
    let ioq = SlistHead::<PfrKentryioq>::new();
    let mut ad = PfrAddr::default();
    let mut xadd = 0;
    let tzero = gettime();
    let user = flags & PFR_FLAG_USERIOCTL != 0;

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_FEEDBACK)?;
    pfr_validate_table(tbl, 0, user)?;
    let tmpkt =
        pfr_create_ktable(&PFR_NULLTABLE, 0, false, pfr_wait(flags)).ok_or(Errno::ENOMEM)?;

    let rv: Result<(), Errno> = 'bad: {
        for i in 0..size.max(0) as usize {
            pfr_yield(user);
            if pfr_copyin(addr, i, &mut ad, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
            if !pfr_validate_addr(&ad) {
                break 'bad Err(Errno::EINVAL);
            }

            let Some(ke) = pfr_create_kentry_unlocked(&mut ad, flags) else {
                break 'bad Err(Errno::ENOMEM);
            };
            ke.pfrke_fb.set(PFR_FB_NONE);
            // SAFETY: a fresh entry, on no list; it stays allocated until destroyed after
            // leaving the ioq.
            unsafe { ioq.insert_head(ke) };
        }

        net_lock();
        pf_lock();
        let kt = pfr_lookup_table(tbl);
        let Some(kt) = kt.filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0) else {
            pf_unlock();
            net_unlock();
            break 'bad Err(Errno::ESRCH);
        };
        if kt.pfrkt_flags().get() & PFR_TFLAG_CONST != 0 {
            pf_unlock();
            net_unlock();
            break 'bad Err(Errno::EPERM);
        }
        for ke in sl_iter(&ioq) {
            pfr_kentry_kif_ref(ke);
            let p = pfr_lookup_kentry(kt, ke, true);
            let q = pfr_lookup_kentry(tmpkt, ke, true);
            if flags & PFR_FLAG_FEEDBACK != 0 {
                if q.is_some() {
                    ke.pfrke_fb.set(PFR_FB_DUPLICATE);
                } else if let Some(p) = p {
                    if (p.pfrke_flags.get() & PFRKE_FLAG_NOT)
                        != (ke.pfrke_flags.get() & PFRKE_FLAG_NOT)
                    {
                        ke.pfrke_fb.set(PFR_FB_CONFLICT);
                    } else {
                        ke.pfrke_fb.set(PFR_FB_NONE);
                    }
                } else {
                    ke.pfrke_fb.set(PFR_FB_ADDED);
                }
            }
            if p.is_none() && q.is_none() {
                if !pfr_route_kentry(tmpkt, ke) {
                    // defer destroy after feedback is processed
                    ke.pfrke_fb.set(PFR_FB_NONE);
                } else {
                    // mark entry as added to table, so we won't kill it with rest of the ioq
                    ke.pfrke_fb.set(PFR_FB_ADDED);
                    // SAFETY: a work queue of this call; the entry is on no other.
                    unsafe { workq.insert_head(ke) };
                    xadd += 1;
                }
            }
        }
        // remove entries, which we will insert from tmpkt
        pfr_clean_node_mask(tmpkt, &workq);
        if flags & PFR_FLAG_DUMMY == 0 {
            pfr_insert_kentries(kt, &workq, tzero);
        }

        pf_unlock();
        net_unlock();

        if flags & PFR_FLAG_FEEDBACK != 0 {
            let mut i = 0;
            while let Some(ke) = sl_first(&ioq) {
                pfr_yield(user);
                pfr_fill_feedback(ke, &mut ad);
                if pfr_copyout(&ad, addr, i, flags).is_err() {
                    break 'bad Err(Errno::EFAULT);
                }
                i += 1;
                // SAFETY: the ioq holds `ke` at its head.
                unsafe { ioq.remove_head() };
                match ke.pfrke_fb.get() {
                    PFR_FB_CONFLICT | PFR_FB_DUPLICATE | PFR_FB_NONE => pfr_destroy_kentry(ke),
                    PFR_FB_ADDED if flags & PFR_FLAG_DUMMY != 0 => pfr_destroy_kentry(ke),
                    _ => {}
                }
            }
        } else {
            pfr_destroy_ioq(&ioq, flags);
        }

        if let Some(nadd) = nadd {
            *nadd = xadd;
        }

        pfr_destroy_ktable(tmpkt, false);
        return Ok(());
    };
    pfr_destroy_ioq(&ioq, flags);
    if flags & PFR_FLAG_FEEDBACK != 0 {
        pfr_reset_feedback(addr, size, flags);
    }
    pfr_destroy_ktable(tmpkt, false);
    rv
}

/// `pfr_del_addrs`: removes `size` addresses of `addr` from the table `tbl`; the count of
/// removed ones in `ndel`.
pub fn pfr_del_addrs(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAddr>,
    size: i32,
    ndel: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKentryworkq>::new();
    let mut ad = PfrAddr::default();
    let mut xdel = 0;
    let mut log = 1;
    let user = flags & PFR_FLAG_USERIOCTL != 0;

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_FEEDBACK)?;
    pfr_validate_table(tbl, 0, user)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };
    if kt.pfrkt_flags().get() & PFR_TFLAG_CONST != 0 {
        return Err(Errno::EPERM);
    }
    // there are two algorithms to choose from here.
    // with:
    //   n: number of addresses to delete
    //   N: number of addresses in the table
    //
    // one is O(N) and is better for large 'n'
    // one is O(n*LOG(N)) and is better for small 'n'
    //
    // following code try to decide which one is best.
    let mut i = kt.pfrkt_cnt().get();
    while i > 0 {
        log += 1;
        i >>= 1;
    }
    if size > kt.pfrkt_cnt().get() / log {
        // full table scan
        pfr_mark_addrs(kt);
    } else {
        // iterate over addresses to delete
        for i in 0..size.max(0) as usize {
            pfr_yield(user);
            if pfr_copyin(addr, i, &mut ad, flags).is_err() {
                return Err(Errno::EFAULT);
            }
            if !pfr_validate_addr(&ad) {
                return Err(Errno::EINVAL);
            }
            if let Some(p) = pfr_lookup_addr(kt, &ad, true) {
                p.pfrke_flags.set(p.pfrke_flags.get() & !PFRKE_FLAG_MARK);
            }
        }
    }
    let rv: Result<(), Errno> = 'bad: {
        for i in 0..size.max(0) as usize {
            pfr_yield(user);
            if pfr_copyin(addr, i, &mut ad, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
            if !pfr_validate_addr(&ad) {
                break 'bad Err(Errno::EINVAL);
            }
            let p = pfr_lookup_addr(kt, &ad, true);
            if flags & PFR_FLAG_FEEDBACK != 0 {
                ad.pfra_fback = match p {
                    None => PFR_FB_NONE,
                    Some(p) if (p.pfrke_flags.get() & PFRKE_FLAG_NOT) != ad.pfra_not => {
                        PFR_FB_CONFLICT
                    }
                    Some(p) if p.pfrke_flags.get() & PFRKE_FLAG_MARK != 0 => PFR_FB_DUPLICATE,
                    Some(_) => PFR_FB_DELETED,
                };
            }
            if let Some(p) = p
                && (p.pfrke_flags.get() & PFRKE_FLAG_NOT) == ad.pfra_not
                && p.pfrke_flags.get() & PFRKE_FLAG_MARK == 0
            {
                p.pfrke_flags.set(p.pfrke_flags.get() | PFRKE_FLAG_MARK);
                // SAFETY: a work queue of this call; the mark keeps an entry from being
                // queued twice.
                unsafe { workq.insert_head(p) };
                xdel += 1;
            }
            if flags & PFR_FLAG_FEEDBACK != 0 && pfr_copyout(&ad, addr, i, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
        }
        if flags & PFR_FLAG_DUMMY == 0 {
            pfr_remove_kentries(kt, &workq);
        }
        if let Some(ndel) = ndel {
            *ndel = xdel;
        }
        return Ok(());
    };
    if flags & PFR_FLAG_FEEDBACK != 0 {
        pfr_reset_feedback(addr, size, flags);
    }
    rv
}

/// `pfr_set_addrs`: makes the table `tbl` hold exactly the `size` addresses of `addr`; the
/// counts in `nadd`, `ndel` and `nchange`. With `PFR_FLAG_FEEDBACK` and a nonzero `*size2`,
/// the deleted addresses are written after the given ones, `*size2` being the room for them
/// and set to the number needed.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn pfr_set_addrs(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAddr>,
    size: i32,
    mut size2: Option<&mut i32>,
    nadd: Option<&mut i32>,
    ndel: Option<&mut i32>,
    nchange: Option<&mut i32>,
    flags: i32,
    ignore_pfrt_flags: u32,
) -> Result<(), Errno> {
    let addq = SlistHead::<PfrKentryworkq>::new();
    let delq = SlistHead::<PfrKentryworkq>::new();
    let changeq = SlistHead::<PfrKentryworkq>::new();
    let mut ad = PfrAddr::default();
    let (mut xadd, mut xdel, mut xchange) = (0, 0, 0);
    let tzero = gettime();
    let user = flags & PFR_FLAG_USERIOCTL != 0;

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_FEEDBACK)?;
    pfr_validate_table(tbl, ignore_pfrt_flags, user)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };
    if kt.pfrkt_flags().get() & PFR_TFLAG_CONST != 0 {
        return Err(Errno::EPERM);
    }
    let tmpkt =
        pfr_create_ktable(&PFR_NULLTABLE, 0, false, pfr_wait(flags)).ok_or(Errno::ENOMEM)?;
    pfr_mark_addrs(kt);
    let rv: Result<(), Errno> = 'bad: {
        for i in 0..size.max(0) as usize {
            pfr_yield(user);
            if pfr_copyin(addr, i, &mut ad, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
            if !pfr_validate_addr(&ad) {
                break 'bad Err(Errno::EINVAL);
            }
            ad.pfra_fback = PFR_FB_NONE;
            'skip: {
                if let Some(p) = pfr_lookup_addr(kt, &ad, true) {
                    if p.pfrke_flags.get() & PFRKE_FLAG_MARK != 0 {
                        ad.pfra_fback = PFR_FB_DUPLICATE;
                        break 'skip;
                    }
                    p.pfrke_flags.set(p.pfrke_flags.get() | PFRKE_FLAG_MARK);
                    if (p.pfrke_flags.get() & PFRKE_FLAG_NOT) != ad.pfra_not {
                        // SAFETY: a work queue of this call; the mark keeps an entry from
                        // being queued twice.
                        unsafe { changeq.insert_head(p) };
                        ad.pfra_fback = PFR_FB_CHANGED;
                        xchange += 1;
                    }
                } else {
                    if pfr_lookup_addr(tmpkt, &ad, true).is_some() {
                        ad.pfra_fback = PFR_FB_DUPLICATE;
                        break 'skip;
                    }
                    let Some(p) = pfr_create_kentry(&mut ad) else {
                        break 'bad Err(Errno::ENOMEM);
                    };
                    if !pfr_route_kentry(tmpkt, p) {
                        pfr_destroy_kentry(p);
                        ad.pfra_fback = PFR_FB_NONE;
                        break 'skip;
                    }
                    // SAFETY: a fresh entry, on no list.
                    unsafe { addq.insert_head(p) };
                    ad.pfra_fback = PFR_FB_ADDED;
                    xadd += 1;
                    if p.pfrke_type.get() == PFRKE_COST {
                        kt.pfrkt_refcntcost.set(kt.pfrkt_refcntcost.get() + 1);
                    }
                    pfr_ktable_winfo_update(kt, p);
                }
            }
            // _skip:
            if flags & PFR_FLAG_FEEDBACK != 0 && pfr_copyout(&ad, addr, i, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
        }
        pfr_enqueue_addrs(kt, &delq, Some(&mut xdel), ENQUEUE_UNMARKED_ONLY);
        if flags & PFR_FLAG_FEEDBACK != 0
            && let Some(s2) = size2.as_deref_mut()
            && *s2 != 0
        {
            if *s2 < size + xdel {
                *s2 = size + xdel;
                break 'bad Ok(());
            }
            for (i, p) in sl_iter(&delq).enumerate() {
                pfr_copyout_addr(&mut ad, Some(p));
                ad.pfra_fback = PFR_FB_DELETED;
                if pfr_copyout(&ad, addr, size as usize + i, flags).is_err() {
                    break 'bad Err(Errno::EFAULT);
                }
            }
        }
        pfr_clean_node_mask(tmpkt, &addq);
        if flags & PFR_FLAG_DUMMY == 0 {
            pfr_insert_kentries(kt, &addq, tzero);
            pfr_remove_kentries(kt, &delq);
            pfr_clstats_kentries(&changeq, tzero, INVERT_NEG_FLAG);
        } else {
            pfr_destroy_kentries(&addq);
        }
        if let Some(nadd) = nadd {
            *nadd = xadd;
        }
        if let Some(ndel) = ndel {
            *ndel = xdel;
        }
        if let Some(nchange) = nchange {
            *nchange = xchange;
        }
        if flags & PFR_FLAG_FEEDBACK != 0
            && let Some(s2) = size2
        {
            *s2 = size + xdel;
        }
        pfr_destroy_ktable(tmpkt, false);
        return Ok(());
    };
    pfr_clean_node_mask(tmpkt, &addq);
    pfr_destroy_kentries(&addq);
    if flags & PFR_FLAG_FEEDBACK != 0 {
        pfr_reset_feedback(addr, size, flags);
    }
    pfr_destroy_ktable(tmpkt, false);
    rv
}

/// `pfr_tst_addrs`: tests which of the `size` host addresses of `addr` the table `tbl`
/// matches (feedback in each), the count of matches in `nmatch`; with `PFR_FLAG_REPLACE` each
/// address is replaced by the entry that matched it.
pub fn pfr_tst_addrs(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAddr>,
    size: i32,
    nmatch: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let mut ad = PfrAddr::default();
    let mut xmatch = 0;

    accept_flags(flags, PFR_FLAG_REPLACE)?;
    pfr_validate_table(tbl, 0, false)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };

    for i in 0..size.max(0) as usize {
        pfr_yield(flags & PFR_FLAG_USERIOCTL != 0);
        if pfr_copyin(addr, i, &mut ad, flags).is_err() {
            return Err(Errno::EFAULT);
        }
        if !pfr_validate_addr(&ad) {
            return Err(Errno::EINVAL);
        }
        if addr_network(&ad) {
            return Err(Errno::EINVAL);
        }
        let p = pfr_lookup_addr(kt, &ad, false);
        if flags & PFR_FLAG_REPLACE != 0 {
            pfr_copyout_addr(&mut ad, p);
        }
        ad.pfra_fback = match p {
            None => PFR_FB_NONE,
            Some(p) if p.pfrke_flags.get() & PFRKE_FLAG_NOT != 0 => PFR_FB_NOTMATCH,
            Some(_) => PFR_FB_MATCH,
        };
        if p.is_some_and(|p| p.pfrke_flags.get() & PFRKE_FLAG_NOT == 0) {
            xmatch += 1;
        }
        if pfr_copyout(&ad, addr, i, flags).is_err() {
            return Err(Errno::EFAULT);
        }
    }
    if let Some(nmatch) = nmatch {
        *nmatch = xmatch;
    }
    Ok(())
}

/// `rn_walktree(h, pfr_walktree, w)`: `pfr_walktree` on each entry of the tree `h`, the first
/// error ending the walk. The tree's lock is taken shared for each step only, to take the
/// successor and the entry before `pfr_walktree` runs (which may copy out to user memory or
/// queue the entry for a delete), and released before it; the walker's table id is the C's
/// `rnh_rtableid`, which nothing writes: 0.
fn pfr_walk(h: &Rwlocked<PfrRadix>, w: &mut PfrWalktree<'_, '_>) -> Result<(), Errno> {
    let mut c = pfr_rnh(h).first();
    while let Some(leaf) = c {
        let ke = {
            let t = pfr_rnh(h);
            c = t.next(leaf);
            t.get(leaf).copied()
        };
        if let Some(ke) = ke {
            pfr_walktree(ke, w, 0)?;
        }
    }
    Ok(())
}

/// `pfr_get_addrs`: copies the addresses of the table `tbl` out to `addr` (`copyout`, a user
/// buffer), when `*size` leaves room for them; `*size` is set to their number.
pub fn pfr_get_addrs(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAddr>,
    size: &mut i32,
    flags: i32,
) -> Result<(), Errno> {
    accept_flags(flags, 0)?;
    pfr_validate_table(tbl, 0, false)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };
    if kt.pfrkt_cnt().get() > *size {
        *size = kt.pfrkt_cnt().get();
        return Ok(());
    }

    let mut w = PfrWalktree::new(PfrwOp::GetAddrs);
    w.pfrw_1 = Pfrw1::Addr(addr, 0);
    w.pfrw_free = kt.pfrkt_cnt().get();
    w.pfrw_flags = flags;
    let mut rv = pfr_walk(&kt.pfrkt_ip4, &mut w);
    if rv.is_ok() {
        rv = pfr_walk(&kt.pfrkt_ip6, &mut w);
    }
    rv?;

    if w.pfrw_free != 0 {
        crate::dpfprintf!(
            LOG_ERR,
            "pfr_get_addrs: corruption detected ({})",
            w.pfrw_free
        );
        return Err(Errno::ENOTTY);
    }
    *size = kt.pfrkt_cnt().get();
    Ok(())
}

/// `pfr_get_astats`: copies the addresses of the table `tbl` with their counters out to
/// `addr`, when `*size` leaves room for them; `*size` is set to their number.
/// `PFR_FLAG_CLSTATS` clears the counters afterwards.
pub fn pfr_get_astats(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAstats>,
    size: &mut i32,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKentryworkq>::new();
    let tzero = gettime();

    pfr_validate_table(tbl, 0, false)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };
    if kt.pfrkt_cnt().get() > *size {
        *size = kt.pfrkt_cnt().get();
        return Ok(());
    }

    let mut w = PfrWalktree::new(PfrwOp::GetAstats);
    w.pfrw_1 = Pfrw1::Astats(addr, 0);
    w.pfrw_free = kt.pfrkt_cnt().get();
    w.pfrw_flags = flags;
    let mut rv = pfr_walk(&kt.pfrkt_ip4, &mut w);
    if rv.is_ok() {
        rv = pfr_walk(&kt.pfrkt_ip6, &mut w);
    }
    if rv.is_ok() && flags & PFR_FLAG_CLSTATS != 0 {
        pfr_enqueue_addrs(kt, &workq, None, false);
        pfr_clstats_kentries(&workq, tzero, false);
    }
    rv?;

    if w.pfrw_free != 0 {
        crate::dpfprintf!(
            LOG_ERR,
            "pfr_get_astats: corruption detected ({})",
            w.pfrw_free
        );
        return Err(Errno::ENOTTY);
    }
    *size = kt.pfrkt_cnt().get();
    Ok(())
}

/// `pfr_clr_astats`: clears the counters of the `size` addresses of `addr` in the table
/// `tbl`; the count in `nzero`.
pub fn pfr_clr_astats(
    tbl: &mut PfrTable,
    addr: &mut PfrBuf<'_, PfrAddr>,
    size: i32,
    nzero: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKentryworkq>::new();
    let mut ad = PfrAddr::default();
    let mut xzero = 0;

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_FEEDBACK)?;
    pfr_validate_table(tbl, 0, false)?;
    let Some(kt) =
        pfr_lookup_table(tbl).filter(|kt| kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0)
    else {
        return Err(Errno::ESRCH);
    };
    let rv: Result<(), Errno> = 'bad: {
        for i in 0..size.max(0) as usize {
            pfr_yield(flags & PFR_FLAG_USERIOCTL != 0);
            if pfr_copyin(addr, i, &mut ad, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
            if !pfr_validate_addr(&ad) {
                break 'bad Err(Errno::EINVAL);
            }
            let p = pfr_lookup_addr(kt, &ad, true);
            if flags & PFR_FLAG_FEEDBACK != 0 {
                ad.pfra_fback = if p.is_some() {
                    PFR_FB_CLEARED
                } else {
                    PFR_FB_NONE
                };
                if pfr_copyout(&ad, addr, i, flags).is_err() {
                    break 'bad Err(Errno::EFAULT);
                }
            }
            if let Some(p) = p {
                // SAFETY: a work queue of this call (an address given twice is queued twice,
                // as in C).
                unsafe { workq.insert_head(p) };
                xzero += 1;
            }
        }

        if flags & PFR_FLAG_DUMMY == 0 {
            pfr_clstats_kentries(&workq, gettime(), false);
        }
        if let Some(nzero) = nzero {
            *nzero = xzero;
        }
        return Ok(());
    };
    if flags & PFR_FLAG_FEEDBACK != 0 {
        pfr_reset_feedback(addr, size, flags);
    }
    rv
}

/// `pfr_validate_addr`: `true` (the C's 0) when `ad` is a valid table address; `false` (the
/// C's -1) for an unknown family, a prefix too long, host bits set, or bad flags.
pub fn pfr_validate_addr(ad: &PfrAddr) -> bool {
    match ad.pfra_af {
        AF_INET => {
            if ad.pfra_net > 32 {
                return false;
            }
        }
        #[cfg(feature = "inet6")]
        AF_INET6 => {
            if ad.pfra_net > 128 {
                return false;
            }
        }
        _ => return false,
    }
    let net = usize::from(ad.pfra_net);
    let bytes = &ad.pfra_u.addr8;
    if net < 128 && bytes[net / 8] & (0xff >> (net % 8)) != 0 {
        return false;
    }
    if bytes[net.div_ceil(8)..].iter().any(|&b| b != 0) {
        return false;
    }
    if ad.pfra_not != 0 && ad.pfra_not != 1 {
        return false;
    }
    if ad.pfra_fback != PFR_FB_NONE {
        return false;
    }
    if ad.pfra_type >= PFRKE_MAX {
        return false;
    }
    true
}

/// `pfr_enqueue_addrs`: queues the entries of `kt` (only the unmarked ones with `sweep`) on
/// `workq`; their number in `naddr`.
pub fn pfr_enqueue_addrs(
    kt: &'static PfrKtable,
    workq: &SlistHead<PfrKentryworkq>,
    naddr: Option<&mut i32>,
    sweep: bool,
) {
    workq.init();
    let mut w = PfrWalktree::new(if sweep {
        PfrwOp::Sweep
    } else {
        PfrwOp::Enqueue
    });
    w.pfrw_1 = Pfrw1::Workq(workq);
    if pfr_walk(&kt.pfrkt_ip4, &mut w).is_err() {
        crate::dpfprintf!(LOG_ERR, "pfr_enqueue_addrs: IPv4 walktree failed.");
    }
    if pfr_walk(&kt.pfrkt_ip6, &mut w).is_err() {
        crate::dpfprintf!(LOG_ERR, "pfr_enqueue_addrs: IPv6 walktree failed.");
    }
    if let Some(naddr) = naddr {
        *naddr = w.pfrw_free;
    }
}

/// `pfr_mark_addrs`: clears the mark of every entry of `kt`.
pub fn pfr_mark_addrs(kt: &'static PfrKtable) {
    let mut w = PfrWalktree::new(PfrwOp::Mark);
    if pfr_walk(&kt.pfrkt_ip4, &mut w).is_err() {
        crate::dpfprintf!(LOG_ERR, "pfr_mark_addrs: IPv4 walktree failed.");
    }
    if pfr_walk(&kt.pfrkt_ip6, &mut w).is_err() {
        crate::dpfprintf!(LOG_ERR, "pfr_mark_addrs: IPv6 walktree failed.");
    }
}

/// `pfr_lookup_addr`: the entry of `kt` for `ad`: for a network, the entry of exactly that
/// network; for a host, the best match (`None` with `exact` when that is a network).
pub fn pfr_lookup_addr(
    kt: &'static PfrKtable,
    ad: &PfrAddr,
    exact: bool,
) -> Option<&'static PfrKentry> {
    let mut sa = PfsockaddrUnion::default();
    let mut mask = PfsockaddrUnion::default();

    let head = match ad.pfra_af {
        AF_INET => {
            fillin_sin(&mut sa, ad.pfra_ip4addr());
            &kt.pfrkt_ip4
        }
        #[cfg(feature = "inet6")]
        AF_INET6 => {
            fillin_sin6(&mut sa, ad.pfra_ip6addr());
            &kt.pfrkt_ip6
        }
        af => unhandled_af(i32::from(af)),
    };
    if addr_network(ad) {
        pfr_prepare_network(&mut mask, ad.pfra_af, i32::from(ad.pfra_net));
        pfr_rnh(head).lookup(&sa.bytes, Some(&mask.bytes)).copied()
    } else {
        let head = pfr_rnh(head);
        let ke = head.matches(&sa.bytes).copied();
        ke.filter(|ke| !(exact && kentry_network(ke)))
    }
}

/// `pfr_lookup_kentry`: as `pfr_lookup_addr`, for the address of the entry `key`.
pub fn pfr_lookup_kentry(
    kt: &'static PfrKtable,
    key: &PfrKentry,
    exact: bool,
) -> Option<&'static PfrKentry> {
    let mut mask = PfsockaddrUnion::default();

    let head = pfr_tree(kt, key.pfrke_af.get());
    if kentry_network(key) {
        pfr_prepare_network(
            &mut mask,
            key.pfrke_af.get(),
            i32::from(key.pfrke_net.get()),
        );
        pfr_rnh(head)
            .lookup(&ke_key(key), Some(&mask.bytes))
            .copied()
    } else {
        let head = pfr_rnh(head);
        let ke = head.matches(&ke_key(key)).copied();
        ke.filter(|ke| !(exact && kentry_network(ke)))
    }
}

/// An item of `pfr_kentry_pl[type]` (zeroed), as its common part.
fn pfr_kentry_get(type_: u8, flags: i32) -> Option<&'static PfrKentry> {
    let pp = &PFR_KENTRY_PL[usize::from(type_)];
    match type_ {
        PFRKE_PLAIN => pf_pool_get::<PfrKentry>(pp, flags),
        PFRKE_ROUTE => pf_pool_get::<PfrKentryRoute>(pp, flags).map(|r| {
            // SAFETY: `PfrKentryRoute` is `#[repr(C)]` and starts with the entry.
            unsafe { &*ptr::from_ref(r).cast::<PfrKentry>() }
        }),
        _ => pf_pool_get::<PfrKentryCost>(pp, flags).map(|c| {
            // SAFETY: `PfrKentryCost` is `#[repr(C)]` and starts with the entry.
            unsafe { &*ptr::from_ref(c).cast::<PfrKentry>() }
        }),
    }
}

/// `pfr_create_kentry`: a new entry for `ad` (its weight made 1 when 0), with a reference on
/// its interface for route and cost entries.
pub fn pfr_create_kentry(ad: &mut PfrAddr) -> Option<&'static PfrKentry> {
    if ad.pfra_type >= PFRKE_MAX {
        panic(format_args!("unknown pfra_type {}", ad.pfra_type));
    }

    let ke = pfr_kentry_get(ad.pfra_type, PR_NOWAIT | PR_ZERO)?;

    ke.pfrke_type.set(ad.pfra_type);

    // set weight allowing implicit weights
    if ad.pfra_weight == 0 {
        ad.pfra_weight = 1;
    }

    match ke.pfrke_type.get() {
        PFRKE_PLAIN => {}
        t => {
            if t == PFRKE_COST {
                pfr_kentry_cost(ke).weight.set(ad.pfra_weight);
            }
            let kr = pfr_kentry_route(ke);
            if ad.pfra_ifname[0] != 0 {
                kr.kif.set(pfi_kif_get(&ad.pfra_ifname, None));
            }
            if let Some(kif) = kr.kif.get() {
                pfi_kif_ref(kif, PFI_KIF_REF_ROUTE);
            }
        }
    }

    let mut sa = ke.pfrke_sa.get();
    match ad.pfra_af {
        AF_INET => fillin_sin(&mut sa, ad.pfra_ip4addr()),
        #[cfg(feature = "inet6")]
        AF_INET6 => fillin_sin6(&mut sa, ad.pfra_ip6addr()),
        af => unhandled_af(i32::from(af)),
    }
    ke.pfrke_sa.set(sa);
    ke.pfrke_af.set(ad.pfra_af);
    ke.pfrke_net.set(ad.pfra_net);
    if ad.pfra_not != 0 {
        ke.pfrke_flags.set(ke.pfrke_flags.get() | PFRKE_FLAG_NOT);
    }
    Some(ke)
}

/// `pfr_create_kentry_unlocked`: as `pfr_create_kentry` without the net lock: the interface
/// name is only recorded, `pfr_kentry_kif_ref` looks it up later. Sleeps for memory with
/// `PFR_FLAG_USERIOCTL`.
pub fn pfr_create_kentry_unlocked(ad: &mut PfrAddr, flags: i32) -> Option<&'static PfrKentry> {
    let mut mflags = PR_ZERO;

    if ad.pfra_type >= PFRKE_MAX {
        panic(format_args!("unknown pfra_type {}", ad.pfra_type));
    }

    if flags & PFR_FLAG_USERIOCTL != 0 {
        mflags |= PR_WAITOK;
    } else {
        mflags |= PR_NOWAIT;
    }

    let ke = pfr_kentry_get(ad.pfra_type, mflags)?;

    ke.pfrke_type.set(ad.pfra_type);

    // set weight allowing implicit weights
    if ad.pfra_weight == 0 {
        ad.pfra_weight = 1;
    }

    match ke.pfrke_type.get() {
        PFRKE_PLAIN => {}
        t => {
            if t == PFRKE_COST {
                pfr_kentry_cost(ke).weight.set(ad.pfra_weight);
            }
            if ad.pfra_ifname[0] != 0 {
                let kr = pfr_kentry_route(ke);
                let mut ifname = kr.ifname.get();
                strlcpy(&mut ifname, &ad.pfra_ifname);
                kr.ifname.set(ifname);
            }
        }
    }

    let mut sa = ke.pfrke_sa.get();
    match ad.pfra_af {
        AF_INET => fillin_sin(&mut sa, ad.pfra_ip4addr()),
        #[cfg(feature = "inet6")]
        AF_INET6 => fillin_sin6(&mut sa, ad.pfra_ip6addr()),
        af => unhandled_af(i32::from(af)),
    }
    ke.pfrke_sa.set(sa);
    ke.pfrke_af.set(ad.pfra_af);
    ke.pfrke_net.set(ad.pfra_net);
    if ad.pfra_not != 0 {
        ke.pfrke_flags.set(ke.pfrke_flags.get() | PFRKE_FLAG_NOT);
    }
    Some(ke)
}

/// `pfr_kentry_kif_ref`: looks up the interface a route or cost entry made by
/// `pfr_create_kentry_unlocked` names, and references it.
pub fn pfr_kentry_kif_ref(ke: &'static PfrKentry) {
    net_assert_locked("pfr_kentry_kif_ref");
    let t = ke.pfrke_type.get();
    if t == PFRKE_COST || t == PFRKE_ROUTE {
        let kr = pfr_kentry_route(ke);
        let ifname = kr.ifname.get();
        if ifname[0] != 0 {
            kr.kif.set(pfi_kif_get(&ifname, None));
        }
        if let Some(kif) = kr.kif.get() {
            pfi_kif_ref(kif, PFI_KIF_REF_ROUTE);
        }
    }
}

/// `pfr_destroy_kentries`: destroys the entries of `workq`, emptying it.
pub fn pfr_destroy_kentries(workq: &SlistHead<PfrKentryworkq>) {
    while let Some(p) = sl_first(workq) {
        pfr_yield(true);
        // SAFETY: the queue holds `p` at its head.
        unsafe { workq.remove_head() };
        pfr_destroy_kentry(p);
    }
}

/// `pfr_destroy_ioq`: empties `ioq`, destroying the entries that did not make it to the table
/// (or all of them with `PFR_FLAG_DUMMY`).
pub fn pfr_destroy_ioq(ioq: &SlistHead<PfrKentryioq>, flags: i32) {
    while let Some(p) = sl_first(ioq) {
        pfr_yield(flags & PFR_FLAG_USERIOCTL != 0);
        // SAFETY: the queue holds `p` at its head.
        unsafe { ioq.remove_head() };
        // we destroy only those entries, which did not make it to table
        if p.pfrke_fb.get() != PFR_FB_ADDED || flags & PFR_FLAG_DUMMY != 0 {
            pfr_destroy_kentry(p);
        }
    }
}

/// `pfr_destroy_kentry`: frees the entry `ke`, its counters and its interface reference. The
/// entry is in no tree and no queue any more.
pub fn pfr_destroy_kentry(ke: &'static PfrKentry) {
    if let Some(c) = ke.pfrke_counters.get() {
        pf_pool_put(&PFR_KCOUNTERS_PL, c);
    }
    let t = ke.pfrke_type.get();
    if t == PFRKE_COST || t == PFRKE_ROUTE {
        pfi_kif_unref(pfr_kentry_route(ke).kif.get(), PFI_KIF_REF_ROUTE);
    }
    pf_pool_put(&PFR_KENTRY_PL[usize::from(t)], ke);
}

/// `pfr_insert_kentries`: routes the entries of `workq` into `kt` (stopping at the first that
/// fails), starting their statistics at `tzero`.
pub fn pfr_insert_kentries(kt: &'static PfrKtable, workq: &SlistHead<PfrKentryworkq>, tzero: Time) {
    let mut n = 0;

    for p in sl_iter(workq) {
        if !pfr_route_kentry(kt, p) {
            crate::dpfprintf!(
                LOG_ERR,
                "pfr_insert_kentries: cannot route entry (code={}).",
                -1
            );
            break;
        }
        p.pfrke_tzero.set(tzero);
        n += 1;
        if p.pfrke_type.get() == PFRKE_COST {
            kt.pfrkt_refcntcost.set(kt.pfrkt_refcntcost.get() + 1);
        }
        pfr_ktable_winfo_update(kt, p);
        pfr_yield(true);
    }
    kt.pfrkt_cnt().set(kt.pfrkt_cnt().get() + n);
}

/// `pfr_insert_kentry`: adds the address `ad` to `kt` unless it is there (pf's overload
/// tables and source limiters). `EINVAL` without memory; the C's -1 (`ERESTART`) when the
/// entry cannot be routed.
pub fn pfr_insert_kentry(kt: &'static PfrKtable, ad: &PfrAddr, tzero: Time) -> Result<(), Errno> {
    let mut ad = *ad;

    if pfr_lookup_addr(kt, &ad, true).is_some() {
        return Ok(());
    }
    let Some(p) = pfr_create_kentry(&mut ad) else {
        return Err(Errno::EINVAL);
    };

    if !pfr_route_kentry(kt, p) {
        // The C returns pfr_route_kentry's -1 and keeps the entry.
        return Err(Errno::ERESTART);
    }

    p.pfrke_tzero.set(tzero);
    if p.pfrke_type.get() == PFRKE_COST {
        kt.pfrkt_refcntcost.set(kt.pfrkt_refcntcost.get() + 1);
    }
    kt.pfrkt_cnt().set(kt.pfrkt_cnt().get() + 1);
    pfr_ktable_winfo_update(kt, p);

    Ok(())
}

/// `pfr_remove_kentry`: removes the address `ad` from `kt`; `ESRCH` when it is not there (or
/// is a negated entry).
pub fn pfr_remove_kentry(kt: &'static PfrKtable, ad: &PfrAddr) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKentryworkq>::new();

    let p = pfr_lookup_addr(kt, ad, true);
    let Some(p) = p.filter(|p| p.pfrke_flags.get() & PFRKE_FLAG_NOT == 0) else {
        return Err(Errno::ESRCH);
    };

    if p.pfrke_flags.get() & PFRKE_FLAG_MARK != 0 {
        return Ok(());
    }

    p.pfrke_flags.set(p.pfrke_flags.get() | PFRKE_FLAG_MARK);
    // SAFETY: a work queue of this call.
    unsafe { workq.insert_head(p) };
    pfr_remove_kentries(kt, &workq);

    Ok(())
}

/// `pfr_remove_kentries`: unroutes the entries of `workq` from `kt` and destroys them, then
/// recomputes the table's weights.
pub fn pfr_remove_kentries(kt: &'static PfrKtable, workq: &SlistHead<PfrKentryworkq>) {
    let addrq = SlistHead::<PfrKentryworkq>::new();
    let mut n = 0;

    for p in sl_iter(workq) {
        pfr_unroute_kentry(kt, p);
        n += 1;
        pfr_yield(true);
        if p.pfrke_type.get() == PFRKE_COST {
            kt.pfrkt_refcntcost
                .set(kt.pfrkt_refcntcost.get().wrapping_sub(1));
        }
    }
    kt.pfrkt_cnt().set(kt.pfrkt_cnt().get() - n);
    pfr_destroy_kentries(workq);

    // update maxweight and gcd for load balancing
    if kt.pfrkt_refcntcost.get() > 0 {
        kt.pfrkt_gcdweight.set(0);
        kt.pfrkt_maxweight.set(1);
        pfr_enqueue_addrs(kt, &addrq, None, false);
        for p in sl_iter(&addrq) {
            pfr_ktable_winfo_update(kt, p);
        }
    }
}

/// `pfr_clean_node_mask`: unroutes the entries of `workq` from `kt`.
pub fn pfr_clean_node_mask(kt: &'static PfrKtable, workq: &SlistHead<PfrKentryworkq>) {
    for p in sl_iter(workq) {
        pfr_unroute_kentry(kt, p);
    }
}

/// `pfr_clstats_kentries`: clears the counters of the entries of `workq`, flipping their
/// negation with `negchange`.
pub fn pfr_clstats_kentries(workq: &SlistHead<PfrKentryworkq>, tzero: Time, negchange: bool) {
    for p in sl_iter(workq) {
        if negchange {
            p.pfrke_flags.set(p.pfrke_flags.get() ^ PFRKE_FLAG_NOT);
        }
        if let Some(c) = p.pfrke_counters.get() {
            pf_pool_put(&PFR_KCOUNTERS_PL, c);
            p.pfrke_counters.set(None);
        }
        p.pfrke_tzero.set(tzero);
    }
}

/// `pfr_reset_feedback`: clears the feedback of the `size` addresses of `addr` (stopping at
/// the first copy that fails).
pub fn pfr_reset_feedback(addr: &mut PfrBuf<'_, PfrAddr>, size: i32, flags: i32) {
    let mut ad = PfrAddr::default();

    for i in 0..size.max(0) as usize {
        pfr_yield(flags & PFR_FLAG_USERIOCTL != 0);
        if pfr_copyin(addr, i, &mut ad, flags).is_err() {
            break;
        }
        ad.pfra_fback = PFR_FB_NONE;
        if pfr_copyout(&ad, addr, i, flags).is_err() {
            break;
        }
    }
}

/// `pfr_prepare_network`: the netmask of a `net`-bit prefix of family `af` in `sa`.
pub fn pfr_prepare_network(sa: &mut PfsockaddrUnion, af: SaFamily, net: i32) {
    *sa = PfsockaddrUnion::default();
    match af {
        AF_INET => {
            let mut sin = SockaddrIn {
                sin_len: size_of::<SockaddrIn>() as u8,
                sin_family: AF_INET,
                ..SockaddrIn::default()
            };
            sin.sin_addr.s_addr = if net != 0 {
                (u32::MAX << (32 - net)).to_be()
            } else {
                0
            };
            sa.set_sin(&sin);
        }
        #[cfg(feature = "inet6")]
        AF_INET6 => {
            let mut sin6 = SockaddrIn6 {
                sin6_len: size_of::<SockaddrIn6>() as u8,
                sin6_family: AF_INET6,
                ..SockaddrIn6::zeroed()
            };
            let mut net = net;
            for i in 0..4 {
                if net <= 32 {
                    sin6.sin6_addr.set_s6_addr32(
                        i,
                        if net != 0 {
                            (u32::MAX << (32 - net)).to_be()
                        } else {
                            0
                        },
                    );
                    break;
                }
                sin6.sin6_addr.set_s6_addr32(i, 0xffff_ffff);
                net -= 32;
            }
            sa.set_sin6(&sin6);
        }
        _ => unhandled_af(i32::from(af)),
    }
}

/// `pfr_route_kentry`: adds the entry `ke` to the radix tree of its family in `kt`. `true`
/// (the C's 0) when added, `false` (the C's -1) when the radix code refuses it (the key is
/// there, or no memory). The tree's lock is taken alone for the add only, the key and the mask
/// made before it; the tree's growth under it is `M_NOWAIT`.
pub fn pfr_route_kentry(kt: &'static PfrKtable, ke: &'static PfrKentry) -> bool {
    let mut mask = PfsockaddrUnion::default();

    // The C's bzero(ke->pfrke_node): the tree makes the route's nodes.
    let head = pfr_tree(kt, ke.pfrke_af.get());
    let key = ke_key(ke);

    let rn = if kentry_network(ke) {
        pfr_prepare_network(&mut mask, ke.pfrke_af.get(), i32::from(ke.pfrke_net.get()));
        head.write().add(&key, Some(&mask.bytes), ke)
    } else {
        head.write().add(&key, None, ke)
    };

    rn.is_ok()
}

/// `pfr_unroute_kentry`: removes the entry `ke` from the radix tree of its family in `kt`.
/// `true` (the C's 0) when removed, `false` (the C's -1) when it was not there. The tree's lock
/// is taken alone for the delete only.
pub fn pfr_unroute_kentry(kt: &'static PfrKtable, ke: &'static PfrKentry) -> bool {
    let mut mask = PfsockaddrUnion::default();

    let head = pfr_tree(kt, ke.pfrke_af.get());
    let key = ke_key(ke);

    let rn = if kentry_network(ke) {
        pfr_prepare_network(&mut mask, ke.pfrke_af.get(), i32::from(ke.pfrke_net.get()));
        head.write().delete(&key, Some(&mask.bytes))
    } else {
        head.write().delete(&key, None)
    };

    if rn.is_none() {
        crate::dpfprintf!(LOG_ERR, "pfr_unroute_kentry: delete failed.\n");
        return false;
    }
    true
}

/// `pfr_copyout_addr`: the entry `ke` as a `pfr_addr` in `ad` (all zero without an entry).
pub fn pfr_copyout_addr(ad: &mut PfrAddr, ke: Option<&'static PfrKentry>) {
    *ad = PfrAddr::default();
    let Some(ke) = ke else {
        return;
    };
    ad.pfra_af = ke.pfrke_af.get();
    ad.pfra_net = ke.pfrke_net.get();
    ad.pfra_type = ke.pfrke_type.get();
    if ke.pfrke_flags.get() & PFRKE_FLAG_NOT != 0 {
        ad.pfra_not = 1;
    }

    match ad.pfra_af {
        AF_INET => ad.pfra_u.set_v4(ke.pfrke_sa.get().sin().sin_addr),
        #[cfg(feature = "inet6")]
        AF_INET6 => ad.pfra_u.set_v6(ke.pfrke_sa.get().sin6().sin6_addr),
        af => unhandled_af(i32::from(af)),
    }
    if let Some(c) = ke.pfrke_counters.get() {
        ad.pfra_states = c.states.get() as u32;
    }
    let t = ke.pfrke_type.get();
    if t == PFRKE_COST || t == PFRKE_ROUTE {
        if t == PFRKE_COST {
            ad.pfra_weight = pfr_kentry_cost(ke).weight.get();
        }
        if let Some(kif) = pfr_kentry_route(ke).kif.get() {
            strlcpy(&mut ad.pfra_ifname, &kif.pfik_name);
        }
    }
}

/// `pfr_walktree`: what the table walks do with each entry, after `w.pfrw_op`.
fn pfr_walktree(
    ke: &'static PfrKentry,
    w: &mut PfrWalktree<'_, '_>,
    _id: u32,
) -> Result<(), Errno> {
    let mut mask = PfsockaddrUnion::default();
    let flags = w.pfrw_flags;

    match w.pfrw_op {
        PfrwOp::Mark => {
            ke.pfrke_flags.set(ke.pfrke_flags.get() & !PFRKE_FLAG_MARK);
        }
        PfrwOp::Sweep | PfrwOp::Enqueue => {
            if w.pfrw_op == PfrwOp::Sweep && ke.pfrke_flags.get() & PFRKE_FLAG_MARK != 0 {
                return Ok(());
            }
            if let Pfrw1::Workq(workq) = w.pfrw_1 {
                // SAFETY: the queue being built by `pfr_enqueue_addrs`; each entry is visited
                // once.
                unsafe { workq.insert_head(ke) };
            }
            w.pfrw_free += 1;
        }
        PfrwOp::GetAddrs => {
            let free = w.pfrw_free;
            w.pfrw_free -= 1;
            if free > 0
                && let Pfrw1::Addr(buf, i) = &mut w.pfrw_1
            {
                let mut ad = PfrAddr::default();

                pfr_copyout_addr(&mut ad, Some(ke));
                // The C copies out with copyout(9), whatever the flags.
                match buf {
                    PfrBuf::User(base) => {
                        copyout(io_bytes(&ad), io_uaddr::<PfrAddr>(*base, *i)?)
                            .map_err(|_| Errno::EFAULT)?;
                    }
                    PfrBuf::Kernel(_) => return Err(Errno::EFAULT),
                }
                *i += 1;
            }
        }
        PfrwOp::GetAstats => {
            let free = w.pfrw_free;
            w.pfrw_free -= 1;
            if free > 0
                && let Pfrw1::Astats(buf, i) = &mut w.pfrw_1
            {
                let mut as_ = PfrAstats::default();

                pfr_copyout_addr(&mut as_.pfras_a, Some(ke));

                if let Some(c) = ke.pfrke_counters.get() {
                    for d in 0..PFR_DIR_MAX {
                        for o in 0..PFR_OP_ADDR_MAX {
                            as_.pfras_packets[d][o] = c.pfrkc_packets[d][o].get();
                            as_.pfras_bytes[d][o] = c.pfrkc_bytes[d][o].get();
                        }
                    }
                } else {
                    as_.pfras_packets = [[0; PFR_OP_ADDR_MAX]; PFR_DIR_MAX];
                    as_.pfras_bytes = [[0; PFR_OP_ADDR_MAX]; PFR_DIR_MAX];
                    as_.pfras_a.pfra_fback = PFR_FB_NOCOUNT;
                }
                as_.pfras_tzero = ke.pfrke_tzero.get();

                if pfr_copyout(&as_, buf, *i, flags).is_err() {
                    return Err(Errno::EFAULT);
                }
                *i += 1;
            }
        }
        PfrwOp::PoolGet => {
            if ke.pfrke_flags.get() & PFRKE_FLAG_NOT != 0 {
                return Ok(()); // negative entries are ignored
            }
            let cnt = w.pfrw_free;
            w.pfrw_free -= 1;
            if cnt == 0 {
                w.pfrw_1 = Pfrw1::Kentry(Some(ke));
                return Err(Errno::EPERM); // finish search (the C's 1)
            }
        }
        PfrwOp::DynaddrUpdate => {
            if let Pfrw1::Dyn(dyn_) = w.pfrw_1 {
                match ke.pfrke_af.get() {
                    AF_INET => {
                        let n = dyn_.pfid_acnt4.get();
                        dyn_.pfid_acnt4.set(n + 1);
                        if n > 0 {
                            return Ok(());
                        }
                        pfr_prepare_network(&mut mask, AF_INET, i32::from(ke.pfrke_net.get()));
                        dyn_.pfid_addr4.set(sunion2pf(&ke.pfrke_sa.get(), AF_INET));
                        dyn_.pfid_mask4.set(sunion2pf(&mask, AF_INET));
                    }
                    #[cfg(feature = "inet6")]
                    AF_INET6 => {
                        let n = dyn_.pfid_acnt6.get();
                        dyn_.pfid_acnt6.set(n + 1);
                        if n > 0 {
                            return Ok(());
                        }
                        pfr_prepare_network(&mut mask, AF_INET6, i32::from(ke.pfrke_net.get()));
                        dyn_.pfid_addr6.set(sunion2pf(&ke.pfrke_sa.get(), AF_INET6));
                        dyn_.pfid_mask6.set(sunion2pf(&mask, AF_INET6));
                    }
                    af => unhandled_af(i32::from(af)),
                }
            }
        }
    }
    Ok(())
}

/// `pfr_clr_tables`: deactivates the tables `filter` selects (its anchor, or every ruleset
/// with `PFR_FLAG_ALLRSETS`), but those of the reserved anchor; the count in `ndel`.
pub fn pfr_clr_tables(
    filter: &mut PfrTable,
    ndel: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let mut xdel = 0;

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_ALLRSETS)?;
    pfr_fix_anchor(&mut filter.pfrt_anchor)?;
    if pfr_table_count(filter, flags) < 0 {
        return Err(Errno::ENOENT);
    }

    for p in PFR_KTABLES.iter() {
        if pfr_skip_table(filter, p, flags) {
            continue;
        }
        if pf_cstr(p.pfrkt_anchor()) == PF_RESERVED_ANCHOR {
            continue;
        }
        if p.pfrkt_flags().get() & PFR_TFLAG_ACTIVE == 0 {
            continue;
        }
        p.pfrkt_nflags
            .set((p.pfrkt_flags().get() & !PFR_TFLAG_ACTIVE) as i32);
        // SAFETY: a work queue of this call; each table is visited once.
        unsafe { workq.insert_head(p) };
        xdel += 1;
    }
    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_setflags_ktables(&workq);
    }
    if let Some(ndel) = ndel {
        *ndel = xdel;
    }
    Ok(())
}

/// `pfr_add_tables`: creates (or reactivates) the `size` tables of `tbl`, and the root tables
/// of anchored ones; the count of new tables in `nadd`.
pub fn pfr_add_tables(
    tbl: &PfrBuf<'_, PfrTable>,
    size: i32,
    nadd: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let addq = SlistHead::<PfrKtableworkq>::new();
    let changeq = SlistHead::<PfrKtableworkq>::new();
    let auxq = SlistHead::<PfrKtableworkq>::new();
    let mut key = pf_abi_zeroed::<PfrTable>();
    let mut xadd = 0;
    let tzero = gettime();
    let user = flags & PFR_FLAG_USERIOCTL != 0;

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    let rv: Result<(), Errno> = 'bad: {
        // pre-allocate all memory outside of locks
        for i in 0..size.max(0) as usize {
            pfr_yield(user);
            if pfr_copyin(tbl, i, &mut key, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
            if pfr_validate_table(&mut key, PFR_TFLAG_USRMASK, user).is_err() {
                break 'bad Err(Errno::EINVAL);
            }
            key.pfrt_flags.set(key.pfrt_flags.get() | PFR_TFLAG_ACTIVE);
            let Some(p) = pfr_create_ktable(&key, tzero, false, pfr_wait(flags)) else {
                break 'bad Err(Errno::ENOMEM);
            };

            // Note: we also pre-allocate a root table here. We keep it at ->pfrkt_root,
            // which we must not forget about.
            key.pfrt_flags.set(0);
            key.pfrt_anchor.fill(0);
            p.pfrkt_root
                .set(pfr_create_ktable(&key, 0, false, pfr_wait(flags)));
            if p.pfrkt_root.get().is_none() {
                pfr_destroy_ktable(p, false);
                break 'bad Err(Errno::ENOMEM);
            }

            if sl_iter(&auxq).any(|q| pfr_ktable_compare(p, q) == Ordering::Equal) {
                // We need no lock here, because `p` is empty, there are no rules or shadow
                // tables attached.
                if let Some(root) = p.pfrkt_root.get() {
                    pfr_destroy_ktable(root, false);
                }
                p.pfrkt_root.set(None);
                pfr_destroy_ktable(p, false);
                continue;
            }

            // SAFETY: a fresh table, on no queue.
            unsafe { auxq.insert_head(p) };
        }

        // auxq contains freshly allocated tables with no dups. also note there are no
        // rulesets attached, because the attach operation requires PF_LOCK().
        net_lock();
        pf_lock();
        for n in sl_iter(&auxq) {
            match PFR_KTABLES.find(n) {
                None => {
                    // SAFETY: `n` is on `auxq`; it goes onto `addq` through the same link.
                    unsafe {
                        auxq.remove(n);
                        addq.insert_head(n);
                    }
                    xadd += 1;
                }
                Some(p) => {
                    if flags & PFR_FLAG_DUMMY == 0 && p.pfrkt_flags().get() & PFR_TFLAG_ACTIVE == 0
                    {
                        p.pfrkt_nflags.set(
                            ((p.pfrkt_flags().get() & !PFR_TFLAG_USRMASK)
                                | (n.pfrkt_flags().get() & PFR_TFLAG_USRMASK)
                                | PFR_TFLAG_ACTIVE) as i32,
                        );
                        // SAFETY: a work queue of this call; an existing table is found once
                        // (the new ones have no duplicates).
                        unsafe { changeq.insert_head(p) };
                    }
                }
            }
        }

        if flags & PFR_FLAG_DUMMY == 0 {
            // addq contains tables we have to insert and attach rules to them
            //
            // changeq contains tables we need to update
            //
            // auxq contains pre-allocated tables, we won't use and we must free them
            for p in sl_iter(&addq) {
                p.pfrkt_rs.set(pf_find_or_create_ruleset(p.pfrkt_anchor()));
                let Some(rs) = p.pfrkt_rs.get() else {
                    xadd -= 1;
                    // SAFETY: `p` is on `addq`; it moves to `auxq` through the same link.
                    unsafe {
                        addq.remove(p);
                        auxq.insert_head(p);
                    }
                    continue;
                };
                rs.tables.set(rs.tables.get() + 1);

                let Some(q) = p.pfrkt_root.get() else {
                    continue;
                };
                if p.pfrkt_anchor()[0] == 0 {
                    p.pfrkt_root.set(None);
                    // SAFETY: the pre-allocated root is on no queue.
                    unsafe { auxq.insert_head(q) };
                    continue;
                }

                // use pre-allocated root table as a key
                p.pfrkt_root.set(None);
                if let Some(r) = PFR_KTABLES.find(q) {
                    p.pfrkt_root.set(Some(r));
                    // SAFETY: as above.
                    unsafe { auxq.insert_head(q) };
                    continue;
                }
                // there is a chance we could create root table in earlier iteration. such
                // table may exist in addq only then.
                if let Some(r) =
                    sl_iter(&addq).find(|r| pfr_ktable_compare(r, q) == Ordering::Equal)
                {
                    // `r` is our root table we've found earlier, `q` can get dropped.
                    p.pfrkt_root.set(Some(r));
                    // SAFETY: as above.
                    unsafe { auxq.insert_head(q) };
                    continue;
                }

                q.pfrkt_rs.set(pf_find_or_create_ruleset(q.pfrkt_anchor()));
                // root tables are attached to main ruleset, because ->pfrkt_anchor[0] == '\0'
                crate::kassert!(
                    q.pfrkt_rs
                        .get()
                        .is_some_and(|rs| ptr::eq(rs, pf_main_ruleset()))
                );
                if let Some(rs) = q.pfrkt_rs.get() {
                    rs.tables.set(rs.tables.get() + 1);
                }
                p.pfrkt_root.set(Some(q));
                // SAFETY: the root is on no queue; inserted at the head, behind the walk.
                unsafe { addq.insert_head(q) };
            }

            pfr_insert_ktables(&addq);
            pfr_setflags_ktables(&changeq);
        }
        pf_unlock();
        net_unlock();

        pfr_destroy_ktables_aux(&auxq);
        if flags & PFR_FLAG_DUMMY != 0 {
            pfr_destroy_ktables_aux(&addq);
        }

        if let Some(nadd) = nadd {
            *nadd = xadd;
        }
        return Ok(());
    };
    pfr_destroy_ktables_aux(&auxq);
    rv
}

/// `pfr_del_tables`: deactivates the `size` tables of `tbl`; the count in `ndel`.
pub fn pfr_del_tables(
    tbl: &PfrBuf<'_, PfrTable>,
    size: i32,
    ndel: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let mut key = pf_abi_zeroed::<PfrTable>();
    let mut xdel = 0;
    let user = flags & PFR_FLAG_USERIOCTL != 0;

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    for i in 0..size.max(0) as usize {
        pfr_yield(user);
        if pfr_copyin(tbl, i, &mut key, flags).is_err() {
            return Err(Errno::EFAULT);
        }
        pfr_validate_table(&mut key, 0, user)?;
        if let Some(p) = pfr_lookup_table(&key)
            && p.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0
        {
            if sl_iter(&workq).any(|q| pfr_ktable_compare(p, q) == Ordering::Equal) {
                continue; // _skip
            }
            p.pfrkt_nflags
                .set((p.pfrkt_flags().get() & !PFR_TFLAG_ACTIVE) as i32);
            // SAFETY: a work queue of this call; duplicates were skipped.
            unsafe { workq.insert_head(p) };
            xdel += 1;
        }
    }

    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_setflags_ktables(&workq);
    }
    if let Some(ndel) = ndel {
        *ndel = xdel;
    }
    Ok(())
}

/// `pfr_get_tables`: copies the tables `filter` selects out to `tbl`, when `*size` leaves room
/// for them; `*size` is set to their number.
pub fn pfr_get_tables(
    filter: &mut PfrTable,
    tbl: &mut PfrBuf<'_, PfrTable>,
    size: &mut i32,
    flags: i32,
) -> Result<(), Errno> {
    accept_flags(flags, PFR_FLAG_ALLRSETS)?;
    pfr_fix_anchor(&mut filter.pfrt_anchor)?;
    let nn = pfr_table_count(filter, flags);
    let mut n = nn;
    if n < 0 {
        return Err(Errno::ENOENT);
    }
    if n > *size {
        *size = n;
        return Ok(());
    }
    let mut i = 0;
    for p in PFR_KTABLES.iter() {
        if pfr_skip_table(filter, p, flags) {
            continue;
        }
        let left = n;
        n -= 1;
        if left <= 0 {
            continue;
        }
        if pfr_copyout(p.pfrkt_t(), tbl, i, flags).is_err() {
            return Err(Errno::EFAULT);
        }
        i += 1;
    }
    if n != 0 {
        crate::dpfprintf!(LOG_ERR, "pfr_get_tables: corruption detected ({}).", n);
        return Err(Errno::ENOTTY);
    }
    *size = nn;
    Ok(())
}

/// `pfr_get_tstats`: copies the tables `filter` selects, with their counters, out to `tbl`,
/// when `*size` leaves room for them; `*size` is set to their number.
pub fn pfr_get_tstats(
    filter: &mut PfrTable,
    tbl: &mut PfrBuf<'_, PfrTstats>,
    size: &mut i32,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let tzero = gettime();

    // XXX PFR_FLAG_CLSTATS disabled
    accept_flags(flags, PFR_FLAG_ALLRSETS)?;
    pfr_fix_anchor(&mut filter.pfrt_anchor)?;
    let nn = pfr_table_count(filter, flags);
    let mut n = nn;
    if n < 0 {
        return Err(Errno::ENOENT);
    }
    if n > *size {
        *size = n;
        return Ok(());
    }
    let mut i = 0;
    for p in PFR_KTABLES.iter() {
        if pfr_skip_table(filter, p, flags) {
            continue;
        }
        let left = n;
        n -= 1;
        if left <= 0 {
            continue;
        }
        if pfr_copyout(&p.pfrkt_ts, tbl, i, flags).is_err() {
            return Err(Errno::EFAULT);
        }
        i += 1;
        // SAFETY: a work queue of this call; each table is visited once.
        unsafe { workq.insert_head(p) };
    }
    if flags & PFR_FLAG_CLSTATS != 0 {
        pfr_clstats_ktables(&workq, tzero, flags & PFR_FLAG_ADDRSTOO != 0);
    }
    if n != 0 {
        crate::dpfprintf!(LOG_ERR, "pfr_get_tstats: corruption detected ({}).", n);
        return Err(Errno::ENOTTY);
    }
    *size = nn;
    Ok(())
}

/// `pfr_clr_tstats`: clears the counters of the `size` tables of `tbl` (and of their
/// addresses with `PFR_FLAG_ADDRSTOO`); the count in `nzero`.
pub fn pfr_clr_tstats(
    tbl: &PfrBuf<'_, PfrTable>,
    size: i32,
    nzero: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let mut key = pf_abi_zeroed::<PfrTable>();
    let mut xzero = 0;
    let tzero = gettime();

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_ADDRSTOO)?;
    for i in 0..size.max(0) as usize {
        pfr_yield(flags & PFR_FLAG_USERIOCTL != 0);
        if pfr_copyin(tbl, i, &mut key, flags).is_err() {
            return Err(Errno::EFAULT);
        }
        pfr_validate_table(&mut key, 0, false)?;
        if let Some(p) = pfr_lookup_table(&key) {
            // SAFETY: a work queue of this call (a table given twice is queued twice, as in
            // C).
            unsafe { workq.insert_head(p) };
            xzero += 1;
        }
    }
    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_clstats_ktables(&workq, tzero, flags & PFR_FLAG_ADDRSTOO != 0);
    }
    if let Some(nzero) = nzero {
        *nzero = xzero;
    }
    Ok(())
}

/// `pfr_set_tflags`: sets `setflag` and clears `clrflag` (user flags) on the `size` active
/// tables of `tbl`; the counts of changed and deleted (no longer persistent) tables in
/// `nchange` and `ndel`.
pub fn pfr_set_tflags(
    tbl: &PfrBuf<'_, PfrTable>,
    size: i32,
    setflag: i32,
    clrflag: i32,
    nchange: Option<&mut i32>,
    ndel: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let mut key = pf_abi_zeroed::<PfrTable>();
    let (mut xchange, mut xdel) = (0, 0);
    let user = flags & PFR_FLAG_USERIOCTL != 0;
    let (setflag, clrflag) = (setflag as u32, clrflag as u32);

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    if (setflag & !PFR_TFLAG_USRMASK) != 0
        || (clrflag & !PFR_TFLAG_USRMASK) != 0
        || (setflag & clrflag) != 0
    {
        return Err(Errno::EINVAL);
    }
    for i in 0..size.max(0) as usize {
        pfr_yield(user);
        if pfr_copyin(tbl, i, &mut key, flags).is_err() {
            return Err(Errno::EFAULT);
        }
        pfr_validate_table(&mut key, 0, user)?;
        if let Some(p) = pfr_lookup_table(&key)
            && p.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0
        {
            p.pfrkt_nflags
                .set(((p.pfrkt_flags().get() | setflag) & !clrflag) as i32);
            if p.pfrkt_nflags.get() as u32 == p.pfrkt_flags().get() {
                continue; // _skip
            }
            if sl_iter(&workq).any(|q| pfr_ktable_compare(p, q) == Ordering::Equal) {
                continue; // _skip
            }
            // SAFETY: a work queue of this call; duplicates were skipped.
            unsafe { workq.insert_head(p) };
            if p.pfrkt_flags().get() & PFR_TFLAG_PERSIST != 0
                && clrflag & PFR_TFLAG_PERSIST != 0
                && p.pfrkt_flags().get() & PFR_TFLAG_REFERENCED == 0
            {
                xdel += 1;
            } else {
                xchange += 1;
            }
        }
    }
    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_setflags_ktables(&workq);
    }
    if let Some(nchange) = nchange {
        *nchange = xchange;
    }
    if let Some(ndel) = ndel {
        *ndel = xdel;
    }
    Ok(())
}

/// `pfr_ina_begin`: opens a table transaction on the anchor of `trs`, dropping the inactive
/// tables a previous one left; the new ticket in `ticket`, the count of dropped tables in
/// `ndel`.
pub fn pfr_ina_begin(
    trs: &PfrTable,
    ticket: Option<&mut u32>,
    ndel: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let mut xdel = 0;

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    let rs = pf_find_or_create_ruleset(&trs.pfrt_anchor).ok_or(Errno::ENOMEM)?;
    for p in PFR_KTABLES.iter() {
        if p.pfrkt_flags().get() & PFR_TFLAG_INACTIVE == 0 || pfr_skip_table(trs, p, 0) {
            continue;
        }
        p.pfrkt_nflags
            .set((p.pfrkt_flags().get() & !PFR_TFLAG_INACTIVE) as i32);
        // SAFETY: a work queue of this call; each table is visited once.
        unsafe { workq.insert_head(p) };
        xdel += 1;
    }
    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_setflags_ktables(&workq);
        rs.tticket.set(rs.tticket.get().wrapping_add(1));
        if let Some(ticket) = ticket {
            *ticket = rs.tticket.get();
        }
        rs.topen.set(1);
    } else {
        pf_remove_if_empty_ruleset(rs);
    }
    if let Some(ndel) = ndel {
        *ndel = xdel;
    }
    Ok(())
}

/// `pfr_ina_define`: defines the table `tbl` in the open transaction `ticket`, with the
/// `size` addresses of `addr` (`PFR_FLAG_ADDRSTOO`) in its shadow; the counts of new tables and
/// of addresses in `nadd` and `naddr`.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn pfr_ina_define(
    tbl: &mut PfrTable,
    addr: &PfrBuf<'_, PfrAddr>,
    size: i32,
    nadd: Option<&mut i32>,
    naddr: Option<&mut i32>,
    ticket: u32,
    flags: i32,
) -> Result<(), Errno> {
    let tableq = SlistHead::<PfrKtableworkq>::new();
    let addrq = SlistHead::<PfrKentryworkq>::new();
    let mut ad = PfrAddr::default();
    let (mut xadd, mut xaddr) = (0, 0);
    let user = flags & PFR_FLAG_USERIOCTL != 0;

    accept_flags(flags, PFR_FLAG_DUMMY | PFR_FLAG_ADDRSTOO)?;
    if size != 0 && flags & PFR_FLAG_ADDRSTOO == 0 {
        return Err(Errno::EINVAL);
    }
    pfr_validate_table(tbl, PFR_TFLAG_USRMASK, user)?;
    let rs = pf_find_ruleset(&tbl.pfrt_anchor);
    if !rs.is_some_and(|rs| rs.topen.get() != 0 && ticket == rs.tticket.get()) {
        return Err(Errno::EBUSY);
    }
    tbl.pfrt_flags
        .set(tbl.pfrt_flags.get() | PFR_TFLAG_INACTIVE);
    let kt = match pfr_lookup_table(tbl) {
        None => {
            let kt = pfr_create_ktable(tbl, 0, true, pfr_wait(flags)).ok_or(Errno::ENOMEM)?;
            // SAFETY: a fresh table, on no queue.
            unsafe { tableq.insert_head(kt) };
            xadd += 1;
            'skip: {
                if tbl.pfrt_anchor[0] == 0 {
                    break 'skip;
                }

                // find or create root table
                let mut key = pf_abi_zeroed::<PfrTable>();
                strlcpy(&mut key.pfrt_name, &tbl.pfrt_name);
                if let Some(rt) = pfr_lookup_table(&key) {
                    kt.pfrkt_root.set(Some(rt));
                    break 'skip;
                }
                let Some(rt) = pfr_create_ktable(&key, 0, true, pfr_wait(flags)) else {
                    pfr_destroy_ktables(&tableq, false);
                    return Err(Errno::ENOMEM);
                };
                // SAFETY: a fresh table, on no queue.
                unsafe { tableq.insert_head(rt) };
                kt.pfrkt_root.set(Some(rt));
            }
            kt
        }
        Some(kt) => {
            if kt.pfrkt_flags().get() & PFR_TFLAG_INACTIVE == 0 {
                xadd += 1;
            }
            kt
        }
    };
    // _skip:
    let Some(shadow) = pfr_create_ktable(tbl, 0, false, pfr_wait(flags)) else {
        pfr_destroy_ktables(&tableq, false);
        return Err(Errno::ENOMEM);
    };
    let rv: Result<(), Errno> = 'bad: {
        for i in 0..size.max(0) as usize {
            pfr_yield(user);
            if pfr_copyin(addr, i, &mut ad, flags).is_err() {
                break 'bad Err(Errno::EFAULT);
            }
            if !pfr_validate_addr(&ad) {
                break 'bad Err(Errno::EINVAL);
            }
            if pfr_lookup_addr(shadow, &ad, true).is_some() {
                continue;
            }
            let Some(p) = pfr_create_kentry(&mut ad) else {
                break 'bad Err(Errno::ENOMEM);
            };
            if !pfr_route_kentry(shadow, p) {
                pfr_destroy_kentry(p);
                continue;
            }
            // SAFETY: a fresh entry, on no queue.
            unsafe { addrq.insert_head(p) };
            xaddr += 1;
            if p.pfrke_type.get() == PFRKE_COST {
                kt.pfrkt_refcntcost.set(kt.pfrkt_refcntcost.get() + 1);
            }
            pfr_ktable_winfo_update(kt, p);
        }
        if flags & PFR_FLAG_DUMMY == 0 {
            if let Some(old) = kt.pfrkt_shadow.get() {
                pfr_destroy_ktable(old, true);
            }
            kt.pfrkt_flags()
                .set(kt.pfrkt_flags().get() | PFR_TFLAG_INACTIVE);
            pfr_insert_ktables(&tableq);
            shadow.pfrkt_cnt().set(if flags & PFR_FLAG_ADDRSTOO != 0 {
                xaddr
            } else {
                NO_ADDRESSES
            });
            kt.pfrkt_shadow.set(Some(shadow));
        } else {
            pfr_clean_node_mask(shadow, &addrq);
            pfr_destroy_ktable(shadow, false);
            pfr_destroy_ktables(&tableq, false);
            pfr_destroy_kentries(&addrq);
        }
        if let Some(nadd) = nadd {
            *nadd = xadd;
        }
        if let Some(naddr) = naddr {
            *naddr = xaddr;
        }
        return Ok(());
    };
    pfr_destroy_ktable(shadow, false);
    pfr_destroy_ktables(&tableq, false);
    pfr_destroy_kentries(&addrq);
    rv
}

/// `pfr_ina_rollback`: drops the inactive tables of the transaction `ticket` on the anchor of
/// `trs` and closes it; the count in `ndel`. A stale ticket is not an error.
pub fn pfr_ina_rollback(
    trs: &PfrTable,
    ticket: u32,
    ndel: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let mut xdel = 0;

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    let rs = pf_find_ruleset(&trs.pfrt_anchor);
    let Some(rs) = rs.filter(|rs| rs.topen.get() != 0 && ticket == rs.tticket.get()) else {
        return Ok(());
    };
    for p in PFR_KTABLES.iter() {
        if p.pfrkt_flags().get() & PFR_TFLAG_INACTIVE == 0 || pfr_skip_table(trs, p, 0) {
            continue;
        }
        p.pfrkt_nflags
            .set((p.pfrkt_flags().get() & !PFR_TFLAG_INACTIVE) as i32);
        // SAFETY: a work queue of this call; each table is visited once.
        unsafe { workq.insert_head(p) };
        xdel += 1;
    }
    if flags & PFR_FLAG_DUMMY == 0 {
        pfr_setflags_ktables(&workq);
        rs.topen.set(0);
        pf_remove_if_empty_ruleset(rs);
    }
    if let Some(ndel) = ndel {
        *ndel = xdel;
    }
    Ok(())
}

/// `pfr_ina_commit`: makes the inactive tables of the transaction `ticket` on the anchor of
/// `trs` active and closes it; the counts of new and changed tables in `nadd` and `nchange`.
pub fn pfr_ina_commit(
    trs: &PfrTable,
    ticket: u32,
    nadd: Option<&mut i32>,
    nchange: Option<&mut i32>,
    flags: i32,
) -> Result<(), Errno> {
    let workq = SlistHead::<PfrKtableworkq>::new();
    let (mut xadd, mut xchange) = (0, 0);
    let tzero = gettime();

    accept_flags(flags, PFR_FLAG_DUMMY)?;
    let rs = pf_find_ruleset(&trs.pfrt_anchor);
    let Some(rs) = rs.filter(|rs| rs.topen.get() != 0 && ticket == rs.tticket.get()) else {
        return Err(Errno::EBUSY);
    };

    for p in PFR_KTABLES.iter() {
        if p.pfrkt_flags().get() & PFR_TFLAG_INACTIVE == 0 || pfr_skip_table(trs, p, 0) {
            continue;
        }
        // SAFETY: a work queue of this call; each table is visited once.
        unsafe { workq.insert_head(p) };
        if p.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0 {
            xchange += 1;
        } else {
            xadd += 1;
        }
    }

    if flags & PFR_FLAG_DUMMY == 0 {
        for p in sl_iter(&workq) {
            pfr_commit_ktable(p, tzero);
        }
        rs.topen.set(0);
        pf_remove_if_empty_ruleset(rs);
    }
    if let Some(nadd) = nadd {
        *nadd = xadd;
    }
    if let Some(nchange) = nchange {
        *nchange = xchange;
    }

    Ok(())
}

/// `pfr_commit_ktable`: makes the shadow of `kt` its content: merged address by address when
/// `kt` is active (it might contain addresses), swapped in otherwise.
pub fn pfr_commit_ktable(kt: &'static PfrKtable, tzero: Time) {
    let Some(shadow) = kt.pfrkt_shadow.get() else {
        panic(format_args!("pfr_commit_ktable: no shadow"));
    };

    if shadow.pfrkt_cnt().get() == NO_ADDRESSES {
        if kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE == 0 {
            pfr_clstats_ktable(kt, tzero, true);
        }
    } else if kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0 {
        // kt might contain addresses
        let addrq = SlistHead::<PfrKentryworkq>::new();
        let addq = SlistHead::<PfrKentryworkq>::new();
        let changeq = SlistHead::<PfrKentryworkq>::new();
        let delq = SlistHead::<PfrKentryworkq>::new();
        let garbageq = SlistHead::<PfrKentryworkq>::new();
        let mut ad = PfrAddr::default();

        pfr_enqueue_addrs(shadow, &addrq, None, false);
        pfr_mark_addrs(kt);
        pfr_clean_node_mask(shadow, &addrq);
        while let Some(p) = sl_first(&addrq) {
            // SAFETY: the queue holds `p` at its head.
            unsafe { addrq.remove_head() };
            pfr_copyout_addr(&mut ad, Some(p));
            if let Some(q) = pfr_lookup_addr(kt, &ad, true) {
                if (q.pfrke_flags.get() & PFRKE_FLAG_NOT) != (p.pfrke_flags.get() & PFRKE_FLAG_NOT)
                {
                    // SAFETY: a work queue of this call; the mark below keeps `q` from being
                    // queued twice.
                    unsafe { changeq.insert_head(q) };
                }
                q.pfrke_flags.set(q.pfrke_flags.get() | PFRKE_FLAG_MARK);
                // SAFETY: `p` just left `addrq`.
                unsafe { garbageq.insert_head(p) };
            } else {
                p.pfrke_tzero.set(tzero);
                // SAFETY: `p` just left `addrq`.
                unsafe { addq.insert_head(p) };
            }
        }
        pfr_enqueue_addrs(kt, &delq, None, ENQUEUE_UNMARKED_ONLY);
        pfr_insert_kentries(kt, &addq, tzero);
        pfr_remove_kentries(kt, &delq);
        pfr_clstats_kentries(&changeq, tzero, INVERT_NEG_FLAG);
        pfr_destroy_kentries(&garbageq);
    } else {
        // kt cannot contain addresses
        pfr_swap_tree(&kt.pfrkt_ip4, &shadow.pfrkt_ip4, false);
        pfr_swap_tree(&kt.pfrkt_ip6, &shadow.pfrkt_ip6, true);
        kt.pfrkt_cnt().swap(shadow.pfrkt_cnt());
        pfr_clstats_ktable(kt, tzero, true);
    }
    let nflags = ((shadow.pfrkt_flags().get() & PFR_TFLAG_USRMASK)
        | (kt.pfrkt_flags().get() & PFR_TFLAG_SETMASK)
        | PFR_TFLAG_ACTIVE)
        & !PFR_TFLAG_INACTIVE;
    pfr_destroy_ktable(shadow, false);
    kt.pfrkt_shadow.set(None);
    pfr_setflags_ktable(kt, nflags);
}

/// The C's swap of `kt->pfrkt_ip4` (or `pfrkt_ip6`) and the shadow's in `pfr_commit_ktable`:
/// `kt`'s tree becomes the shadow's and the shadow's `kt`'s. The locks are leaves, so one is
/// held at a time: the shadow's tree is taken out (the shadow is the ioctl's own, reached only
/// through `pfrkt_shadow` under the exclusive net lock and `pf_lock`, so nobody sees it empty),
/// swapped into `kt` in one section of `kt`'s lock (a lookup sees the old tree or the new one),
/// and `kt`'s old tree put in the shadow, which `pfr_destroy_ktable` frees.
fn pfr_swap_tree(kt: &Rwlocked<PfrRadix>, shadow: &Rwlocked<PfrRadix>, ip6: bool) {
    let new = core::mem::replace(&mut *shadow.write(), pfr_radix(ip6));
    let old = core::mem::replace(&mut *kt.write(), new);
    let empty = core::mem::replace(&mut *shadow.write(), old);
    drop(empty);
}

/// `pfr_validate_table`: checks the name, anchor (rewritten by `pfr_fix_anchor`) and flags of
/// `tbl`; `no_reserved` refuses the reserved anchor.
pub fn pfr_validate_table(
    tbl: &mut PfrTable,
    allowedflags: u32,
    no_reserved: bool,
) -> Result<(), Errno> {
    if tbl.pfrt_name[0] == 0 {
        return Err(Errno::EINVAL);
    }
    if no_reserved && pf_cstr(&tbl.pfrt_anchor) == PF_RESERVED_ANCHOR {
        return Err(Errno::EINVAL);
    }
    if strnlen(&tbl.pfrt_name, PF_TABLE_NAME_SIZE) >= PF_TABLE_NAME_SIZE {
        return Err(Errno::ENAMETOOLONG);
    }
    pfr_fix_anchor(&mut tbl.pfrt_anchor)?;
    if tbl.pfrt_flags.get() & !allowedflags != 0 {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `pfr_fix_anchor`: rewrite anchors referenced by tables to remove slashes and check for
/// validity.
pub fn pfr_fix_anchor(anchor: &mut [u8; PATH_MAX]) -> Result<(), Errno> {
    let siz = PATH_MAX;

    if strnlen(anchor, PATH_MAX) >= PATH_MAX {
        return Err(Errno::ENAMETOOLONG);
    }
    if anchor[0] == b'/' {
        let mut off = 1;
        while anchor[off] == b'/' {
            off += 1;
        }
        anchor.copy_within(off.., 0);
        anchor[siz - off..].fill(0);
    }
    Ok(())
}

/// `pfr_table_count`: the number of tables `filter` selects, -1 for an unknown anchor.
pub fn pfr_table_count(filter: &PfrTable, flags: i32) -> i32 {
    if flags & PFR_FLAG_ALLRSETS != 0 {
        return PFR_KTABLE_CNT.get();
    }
    if filter.pfrt_anchor[0] != 0 {
        let rs = pf_find_ruleset(&filter.pfrt_anchor);
        return rs.map_or(-1, |rs| rs.tables.get());
    }
    pf_main_ruleset().tables.get()
}

/// `pfr_skip_table`: `true` when `filter` does not select `kt` (another anchor, without
/// `PFR_FLAG_ALLRSETS`).
pub fn pfr_skip_table(filter: &PfrTable, kt: &PfrKtable, flags: i32) -> bool {
    if flags & PFR_FLAG_ALLRSETS != 0 {
        return false;
    }
    pf_cstr(&filter.pfrt_anchor) != pf_cstr(kt.pfrkt_anchor())
}

/// `pfr_insert_ktables`: inserts the tables of `workq` into `pfr_ktables`.
pub fn pfr_insert_ktables(workq: &SlistHead<PfrKtableworkq>) {
    for p in sl_iter(workq) {
        pfr_insert_ktable(p);
    }
}

/// `pfr_insert_ktable`: inserts `kt` into `pfr_ktables`, referencing its root table.
pub fn pfr_insert_ktable(kt: &'static PfrKtable) {
    // SAFETY: `kt` is a pool item in no tree; it stays allocated until removed again
    // (`pfr_setflags_ktable`).
    let _ = unsafe { PFR_KTABLES.insert(kt) };
    PFR_KTABLE_CNT.set(PFR_KTABLE_CNT.get() + 1);
    if let Some(root) = kt.pfrkt_root.get() {
        let r = &root.pfrkt_refcnt()[PFR_REFCNT_ANCHOR];
        let old = r.get();
        r.set(old + 1);
        if old == 0 {
            pfr_setflags_ktable(root, root.pfrkt_flags().get() | PFR_TFLAG_REFDANCHOR);
        }
    }
}

/// `pfr_setflags_ktables`: gives each table of `workq` its `pfrkt_nflags`.
pub fn pfr_setflags_ktables(workq: &SlistHead<PfrKtableworkq>) {
    for p in sl_iter(workq) {
        pfr_setflags_ktable(p, p.pfrkt_nflags.get() as u32);
    }
}

/// `pfr_setflags_ktable`: gives `kt` the flags `newf`, destroying it when it is no longer
/// referenced, persistent, active or inactive, its addresses when it is no longer active, and
/// its shadow when it is no longer inactive.
pub fn pfr_setflags_ktable(kt: &'static PfrKtable, mut newf: u32) {
    let addrq = SlistHead::<PfrKentryworkq>::new();

    if newf & PFR_TFLAG_REFERENCED == 0
        && newf & PFR_TFLAG_REFDANCHOR == 0
        && newf & PFR_TFLAG_PERSIST == 0
    {
        newf &= !PFR_TFLAG_ACTIVE;
    }
    if newf & PFR_TFLAG_ACTIVE == 0 {
        newf &= !PFR_TFLAG_USRMASK;
    }
    if newf & PFR_TFLAG_SETMASK == 0 {
        // SAFETY: a table with flags in `PFR_TFLAG_SETMASK` is in `pfr_ktables`.
        unsafe { PFR_KTABLES.remove(kt) };
        if let Some(root) = kt.pfrkt_root.get() {
            let r = &root.pfrkt_refcnt()[PFR_REFCNT_ANCHOR];
            r.set(r.get() - 1);
            if r.get() == 0 {
                pfr_setflags_ktable(root, root.pfrkt_flags().get() & !PFR_TFLAG_REFDANCHOR);
            }
        }
        pfr_destroy_ktable(kt, true);
        PFR_KTABLE_CNT.set(PFR_KTABLE_CNT.get() - 1);
        return;
    }
    if newf & PFR_TFLAG_ACTIVE == 0 && kt.pfrkt_cnt().get() != 0 {
        pfr_enqueue_addrs(kt, &addrq, None, false);
        pfr_remove_kentries(kt, &addrq);
    }
    if newf & PFR_TFLAG_INACTIVE == 0
        && let Some(shadow) = kt.pfrkt_shadow.get()
    {
        pfr_destroy_ktable(shadow, true);
        kt.pfrkt_shadow.set(None);
    }
    kt.pfrkt_flags().set(newf);
}

/// `pfr_clstats_ktables`: clears the counters of the tables of `workq`.
pub fn pfr_clstats_ktables(workq: &SlistHead<PfrKtableworkq>, tzero: Time, recurse: bool) {
    for p in sl_iter(workq) {
        pfr_clstats_ktable(p, tzero, recurse);
    }
}

/// `pfr_clstats_ktable`: clears the counters of `kt` (and of its entries with `recurse`).
pub fn pfr_clstats_ktable(kt: &'static PfrKtable, tzero: Time, recurse: bool) {
    let addrq = SlistHead::<PfrKentryworkq>::new();

    if recurse {
        pfr_enqueue_addrs(kt, &addrq, None, false);
        pfr_clstats_kentries(&addrq, tzero, false);
    }
    for d in 0..PFR_DIR_MAX {
        for o in 0..PFR_OP_TABLE_MAX {
            kt.pfrkt_packets()[d][o].set(0);
            kt.pfrkt_bytes()[d][o].set(0);
        }
    }
    kt.pfrkt_match().set(0);
    kt.pfrkt_nomatch().set(0);
    kt.pfrkt_tzero().set(tzero);
}

/// `pfr_create_ktable`: a new table described by `tbl`, with empty radix trees, attached to
/// the ruleset of its anchor with `attachruleset` (under `pf_lock`).
pub fn pfr_create_ktable(
    tbl: &PfrTable,
    tzero: Time,
    attachruleset: bool,
    wait: i32,
) -> Option<&'static PfrKtable> {
    let p = pool_get(&PFR_KTABLE_PL, wait | PR_ZERO | PR_LIMITFAIL)?
        .cast::<PfrKtable>()
        .as_ptr();
    // rn_inithead of both heads: the trees' roots (M_RTABLE, M_NOWAIT), made while the trees
    // are this call's own values, so no lock is taken; the C makes them after attaching the
    // ruleset, and a failure is still answered there, after it, as the C's.
    let mut ip4 = pfr_radix(false);
    let mut ip6 = pfr_radix(true);
    let heads = ip4.inithead() && ip6.inithead();
    // SAFETY: `p` is a fresh, zeroed item of `pfr_ktable_pl`, sized and aligned for a
    // `PfrKtable` (`pfr_initialize`'s `pool_init`), which nothing else refers to yet. All-zero
    // is a valid value of every member but the two locked radix trees (integers, `Cell`s of them
    // and of `Option<&T>`, tree and list links, the byte copy of a `PfAbi` table); the two
    // `Rwlocked` trees are written here through the raw pointer, which neither reads nor drops the
    // zero bytes under them, and so is the table description (`pfrkt_t`, whose name and anchor are not `Cell`s),
    // all before the first shared reference is made. The item stays allocated until
    // `pfr_destroy_ktable` gives it back, after emptying both trees.
    let kt: &'static PfrKtable = unsafe {
        ptr::copy_nonoverlapping(
            ptr::from_ref(tbl),
            ptr::addr_of_mut!((*p).pfrkt_ts.pfrts_t),
            1,
        );
        ptr::addr_of_mut!((*p).pfrkt_ip4).write(Rwlocked::new("pfrktip4", ip4));
        ptr::addr_of_mut!((*p).pfrkt_ip6).write(Rwlocked::new("pfrktip6", ip6));
        &*p
    };

    if attachruleset {
        pf_assert_locked();
        let Some(rs) = pf_find_or_create_ruleset(&tbl.pfrt_anchor) else {
            pfr_destroy_ktable(kt, false);
            return None;
        };
        kt.pfrkt_rs.set(Some(rs));
        rs.tables.set(rs.tables.get() + 1);
    }

    // rn_inithead's failure: no table.
    if !heads {
        pfr_destroy_ktable(kt, false);
        return None;
    }
    kt.pfrkt_tzero().set(tzero);
    kt.pfrkt_refcntcost.set(0);
    kt.pfrkt_gcdweight.set(0);
    kt.pfrkt_maxweight.set(1);

    Some(kt)
}

/// `pfr_destroy_ktables`: destroys the tables of `workq`, emptying it.
pub fn pfr_destroy_ktables(workq: &SlistHead<PfrKtableworkq>, flushaddr: bool) {
    while let Some(p) = sl_first(workq) {
        // SAFETY: the queue holds `p` at its head.
        unsafe { workq.remove_head() };
        pfr_destroy_ktable(p, flushaddr);
    }
}

/// `pfr_destroy_ktables_aux`: destroys the pre-allocated tables of `auxq` and their root
/// tables, emptying it.
pub fn pfr_destroy_ktables_aux(auxq: &SlistHead<PfrKtableworkq>) {
    while let Some(p) = sl_first(auxq) {
        // SAFETY: the queue holds `p` at its head.
        unsafe { auxq.remove_head() };
        // There must be no extra data (rules, shadow tables, ...) attached, because auxq
        // holds just empty memory to be initialized. Therefore we can also be called with no
        // lock.
        if let Some(root) = p.pfrkt_root.get() {
            crate::kassert!(root.pfrkt_rs.get().is_none());
            crate::kassert!(root.pfrkt_shadow.get().is_none());
            crate::kassert!(root.pfrkt_root.get().is_none());
            pfr_destroy_ktable(root, false);
            p.pfrkt_root.set(None);
        }
        crate::kassert!(p.pfrkt_rs.get().is_none());
        crate::kassert!(p.pfrkt_shadow.get().is_none());
        pfr_destroy_ktable(p, false);
    }
}

/// `pfr_destroy_ktable`: frees `kt`, its radix heads (its entries too with `flushaddr`) and
/// its shadow, and drops its ruleset's table count. `kt` is out of `pfr_ktables`.
pub fn pfr_destroy_ktable(kt: &'static PfrKtable, flushaddr: bool) {
    let addrq = SlistHead::<PfrKentryworkq>::new();

    if flushaddr {
        pfr_enqueue_addrs(kt, &addrq, None, false);
        pfr_clean_node_mask(kt, &addrq);
        pfr_destroy_kentries(&addrq);
    }
    // free(kt->pfrkt_ip4, M_RTABLE) and pfrkt_ip6's: each tree's memory given back, so the
    // item owns nothing when it goes back to its pool (which runs no drop). Each tree is taken
    // out under its lock alone and freed after the lock is released.
    let ip4 = core::mem::replace(&mut *kt.pfrkt_ip4.write(), pfr_radix(false));
    drop(ip4);
    let ip6 = core::mem::replace(&mut *kt.pfrkt_ip6.write(), pfr_radix(true));
    drop(ip6);
    if let Some(shadow) = kt.pfrkt_shadow.get() {
        pfr_destroy_ktable(shadow, flushaddr);
    }
    if let Some(rs) = kt.pfrkt_rs.get() {
        rs.tables.set(rs.tables.get() - 1);
        pf_remove_if_empty_ruleset(rs);
    }
    pool_put(&PFR_KTABLE_PL, NonNull::from(kt).cast());
}

/// The order of two table descriptions: name (at most `PF_TABLE_NAME_SIZE` bytes), then
/// anchor.
fn pfr_table_compare(p: &PfrTable, q: &PfrTable) -> Ordering {
    pf_cstr(&p.pfrt_name)
        .cmp(pf_cstr(&q.pfrt_name))
        .then_with(|| pf_cstr(&p.pfrt_anchor).cmp(pf_cstr(&q.pfrt_anchor)))
}

/// `pfr_ktable_compare`: tables are ordered by name, then anchor.
pub fn pfr_ktable_compare(p: &PfrKtable, q: &PfrKtable) -> Ordering {
    pfr_table_compare(p.pfrkt_t(), q.pfrkt_t())
}

/// `pfr_lookup_table`: the table named and anchored as `tbl`.
pub fn pfr_lookup_table(tbl: &PfrTable) -> Option<&'static PfrKtable> {
    // struct pfr_ktable start like a struct pfr_table
    let mut n = PFR_KTABLES.root();
    while let Some(kt) = n {
        n = match pfr_table_compare(tbl, kt.pfrkt_t()) {
            Ordering::Less => RbHead::<PfrKtablehead>::left(kt),
            Ordering::Greater => RbHead::<PfrKtablehead>::right(kt),
            Ordering::Equal => return Some(kt),
        };
    }
    None
}

/// `pfr_match_addr`: whether the active table `kt` (or its root) holds `a` in a positive
/// entry; counts the match or miss.
pub fn pfr_match_addr(kt: &'static PfrKtable, a: &PfAddr, af: SaFamily) -> bool {
    // The negation is read under the tree's lock.
    let m = pfr_kentry_byaddr_with(kt, a, af, false, |ke| {
        ke.is_some_and(|ke| ke.pfrke_flags.get() & PFRKE_FLAG_NOT == 0)
    });
    if m {
        kt.pfrkt_match().set(kt.pfrkt_match().get().wrapping_add(1));
    } else {
        kt.pfrkt_nomatch()
            .set(kt.pfrkt_nomatch().get().wrapping_add(1));
    }

    m
}

/// The radix key of the IPv4 address in the first word of `a` (`tmp4`).
fn pfr_sin_key(a: &PfAddr) -> PfsockaddrUnion {
    let mut tmp4 = PfsockaddrUnion::default();
    fillin_sin(
        &mut tmp4,
        InAddr {
            s_addr: a.addr32(0),
        },
    );
    tmp4
}

/// The `sockaddr_in6` key (`tmp6`) of the address `a`.
#[cfg(feature = "inet6")]
fn pfr_sin6_key(a: &PfAddr) -> PfsockaddrUnion {
    let mut tmp6 = PfsockaddrUnion::default();
    fillin_sin6(&mut tmp6, a.v6());
    tmp6
}

/// `rn_match` of the address `a` (`tmp4` or `tmp6`) in the tree of family `af` of the table
/// `kt`, with `f` run on the entry found while the tree's lock is still held shared: a delete
/// cannot unlink and free the entry while `f` uses it, where the C dereferences the entry after
/// `rn_match` with nothing to keep it. `f` keeps the lock a leaf ([`pfr_rnh`]).
fn pfr_match_kentry<R>(
    kt: &'static PfrKtable,
    a: &PfAddr,
    af: SaFamily,
    f: impl FnOnce(Option<&'static PfrKentry>) -> R,
) -> R {
    match af {
        AF_INET => {
            let tmp4 = pfr_sin_key(a);
            let head = pfr_rnh(&kt.pfrkt_ip4);
            f(head.matches(&tmp4.bytes).copied())
        }
        #[cfg(feature = "inet6")]
        AF_INET6 => {
            let tmp6 = pfr_sin6_key(a);
            let head = pfr_rnh(&kt.pfrkt_ip6);
            f(head.matches(&tmp6.bytes).copied())
        }
        _ => unhandled_af(i32::from(af)),
    }
}

/// [`pfr_kentry_byaddr`]'s answer given to `f` under the tree's lock ([`pfr_match_kentry`]);
/// `f(None)` when neither `kt` nor its root is active.
fn pfr_kentry_byaddr_with<R>(
    kt: &'static PfrKtable,
    a: &PfAddr,
    af: SaFamily,
    exact: bool,
    f: impl FnOnce(Option<&'static PfrKentry>) -> R,
) -> R {
    match pfr_ktable_select_active(kt) {
        Some(kt) => pfr_match_kentry(kt, a, af, |ke| {
            f(ke.filter(|ke| !(exact && kentry_network(ke))))
        }),
        None => f(None),
    }
}

/// `pfr_kentry_byaddr`: the entry of the active table `kt` (or its root) that best matches
/// `a` (`None` with `exact` when that is a network). The entry outlives the tree's lock: for a
/// caller that only tests it (`pf_map_addr_sticky`) or holds `pf_lock`, which every delete
/// holds; the others use [`pfr_kentry_byaddr_with`].
pub fn pfr_kentry_byaddr(
    kt: &'static PfrKtable,
    a: &PfAddr,
    af: SaFamily,
    exact: bool,
) -> Option<&'static PfrKentry> {
    pfr_kentry_byaddr_with(kt, a, af, exact, |ke| ke)
}

/// `pfr_update_stats`: counts the packet `pd` against the table `kt` and the entry matching
/// `a`, as rule action `op`; `notrule` says the rule negated the table.
pub fn pfr_update_stats(kt: &'static PfrKtable, a: &PfAddr, pd: &PfPdesc, op: u8, notrule: bool) {
    let af = pd.af;
    let len = pd.tot_len;
    let dir_idx = usize::from(pd.dir == PF_OUT);

    let Some(kt) = pfr_ktable_select_active(kt) else {
        return;
    };

    // The entry is read and counted on under the tree's lock, so the purge cannot free it
    // meanwhile; the first count's `pfrke_counters` is `PR_NOWAIT`, as the C's. The debug line
    // and the table's own counters come after the lock.
    let (op_idx, failed) = pfr_match_kentry(kt, a, af, |ke| {
        let mut op_idx = match op {
            PF_PASS => PFR_OP_PASS,
            PF_MATCH => PFR_OP_MATCH,
            PF_DROP => PFR_OP_BLOCK,
            _ => panic(format_args!("unhandled op")),
        };
        let mut failed = false;
        if ke.is_none_or(|ke| ke.pfrke_flags.get() & PFRKE_FLAG_NOT != 0) != notrule {
            failed = op_idx != PFR_OP_PASS;
            op_idx = PFR_OP_XPASS;
        }
        if let Some(ke) = ke
            && op_idx != PFR_OP_XPASS
            && kt.pfrkt_flags().get() & PFR_TFLAG_COUNTERS != 0
        {
            if ke.pfrke_counters.get().is_none() {
                ke.pfrke_counters.set(pf_pool_get::<PfrKcounters>(
                    &PFR_KCOUNTERS_PL,
                    PR_NOWAIT | PR_ZERO,
                ));
            }
            if let Some(c) = ke.pfrke_counters.get() {
                let p = &c.pfrkc_packets[dir_idx][op_idx];
                p.set(p.get().wrapping_add(1));
                let b = &c.pfrkc_bytes[dir_idx][op_idx];
                b.set(b.get().wrapping_add(len));
            }
        }
        (op_idx, failed)
    });
    if failed {
        crate::dpfprintf!(LOG_DEBUG, "pfr_update_stats: assertion failed.");
    }
    let c = &kt.pfrkt_packets()[dir_idx][op_idx];
    c.set(c.get().wrapping_add(1));
    let c = &kt.pfrkt_bytes()[dir_idx][op_idx];
    c.set(c.get().wrapping_add(len));
}

/// `pfr_attach_table`: the table `name` of the ruleset `rs` (created, with its root table for
/// an anchor, when missing), referenced by one more rule.
pub fn pfr_attach_table(
    rs: &'static PfRuleset,
    name: &[u8],
    wait: i32,
) -> Option<&'static PfrKtable> {
    let ac = rs.anchor.get();

    let mut tbl = pf_abi_zeroed::<PfrTable>();
    strlcpy(&mut tbl.pfrt_name, name);
    if let Some(ac) = ac {
        strlcpy(&mut tbl.pfrt_anchor, &ac.path);
    }
    let kt = match pfr_lookup_table(&tbl) {
        Some(kt) => kt,
        None => {
            // Hold rs across the table creation below. A new table takes a reference on the
            // ruleset, and pfr_destroy_ktable() drops it again on the failure paths, which
            // lets pf_remove_if_empty_ruleset() free the anchor rs points into while our
            // caller still holds rs.
            rs.tables.set(rs.tables.get() + 1);
            let Some(kt) = pfr_create_ktable(&tbl, gettime(), true, wait) else {
                rs.tables.set(rs.tables.get() - 1);
                return None;
            };
            if ac.is_some() {
                tbl.pfrt_anchor.fill(0);
                let rt = match pfr_lookup_table(&tbl) {
                    Some(rt) => rt,
                    None => {
                        let Some(rt) = pfr_create_ktable(&tbl, 0, true, wait) else {
                            pfr_destroy_ktable(kt, false);
                            rs.tables.set(rs.tables.get() - 1);
                            return None;
                        };
                        pfr_insert_ktable(rt);
                        rt
                    }
                };
                kt.pfrkt_root.set(Some(rt));
            }
            pfr_insert_ktable(kt);
            rs.tables.set(rs.tables.get() - 1);
            kt
        }
    };
    let r = &kt.pfrkt_refcnt()[PFR_REFCNT_RULE];
    let old = r.get();
    r.set(old + 1);
    if old == 0 {
        pfr_setflags_ktable(kt, kt.pfrkt_flags().get() | PFR_TFLAG_REFERENCED);
    }
    Some(kt)
}

/// `pfr_detach_table`: drops a rule's reference on `kt`.
pub fn pfr_detach_table(kt: &'static PfrKtable) {
    let r = &kt.pfrkt_refcnt()[PFR_REFCNT_RULE];
    if r.get() <= 0 {
        crate::dpfprintf!(LOG_NOTICE, "pfr_detach_table: refcount = {}.", r.get());
    } else {
        r.set(r.get() - 1);
        if r.get() == 0 {
            pfr_setflags_ktable(kt, kt.pfrkt_flags().get() & !PFR_TFLAG_REFERENCED);
        }
    }
}

/// `pfr_islinklocal`: whether `addr` is an IPv6 link-local address.
pub fn pfr_islinklocal(af: SaFamily, addr: &PfAddr) -> bool {
    #[cfg(feature = "inet6")]
    if af == AF_INET6 && crate::netinet6::in6::in6_is_addr_linklocal(&addr.v6()) {
        return true;
    }
    #[cfg(not(feature = "inet6"))]
    let _ = (af, addr);
    false
}

/// The kernel `pf_pool`'s table (`PF_ADDR_TABLE`) or dynamic address table
/// (`PF_ADDR_DYNIFTL`).
fn pfr_pool_table(rpool: &'static PfPool) -> Result<Option<&'static PfrKtable>, i32> {
    match rpool.addr.type_.get() {
        PF_ADDR_TABLE => Ok(rpool.addr.tbl()),
        PF_ADDR_DYNIFTL => Ok(rpool.addr.dyn_().and_then(|d| d.pfid_kt.get())),
        _ => Err(-1),
    }
}

/// Sets the pool's position and per-address state after it settled on the entry `ke` at
/// index `idx`, address `addr`.
fn pfr_pool_settle(
    rpool: &'static PfPool,
    kt: &'static PfrKtable,
    ke: &'static PfrKentry,
    idx: i32,
    addr: &PfAddr,
    af: SaFamily,
) {
    let mut counter = rpool.counter.get();
    pf_addrcpy(&mut counter, addr, af);
    rpool.counter.set(counter);
    rpool.tblidx.set(idx);
    kt.pfrkt_match().set(kt.pfrkt_match().get().wrapping_add(1));
    rpool.states.set(0);
    if let Some(c) = ke.pfrke_counters.get() {
        rpool.states.set(c.states.get());
    }
    match ke.pfrke_type.get() {
        t @ (PFRKE_COST | PFRKE_ROUTE) => {
            if t == PFRKE_COST {
                rpool.weight.set(pfr_kentry_cost(ke).weight.get());
            }
            rpool.set_kif(pfr_kentry_route(ke).kif.get());
        }
        _ => rpool.weight.set(1),
    }
}

/// `pfr_pool_get`: the next address of the round-robin pool `rpool` over its table, in
/// family `af`; returns the block the address is in (`*raddr`, `*rmask`) and leaves the
/// address in `rpool.counter`. `Err(1)` when the table has no usable address, `Err(-1)` when
/// the pool is not over a table.
pub fn pfr_pool_get(rpool: &'static PfPool, af: SaFamily) -> Result<(PfAddr, PfAddr), i32> {
    let mut tmp4 = PfsockaddrUnion::default();
    let mut addr = PfAddr::zeroed();
    let mut mask = PfsockaddrUnion::default();
    let mut loop_ = 0;
    let mut use_counter = false;

    match af {
        AF_INET => fillin_sin(&mut tmp4, InAddr { s_addr: 0 }),
        #[cfg(feature = "inet6")]
        AF_INET6 => fillin_sin6(&mut tmp4, In6Addr::default()),
        _ => unhandled_af(i32::from(af)),
    }

    let kt = pfr_pool_table(rpool)?;
    let Some(kt) = kt.and_then(pfr_ktable_select_active) else {
        return Err(-1);
    };

    let mut idx = rpool.tblidx.get();
    if idx < 0 || idx >= kt.pfrkt_cnt().get() {
        idx = 0;
    } else {
        use_counter = true;
    }
    let startidx = idx;

    // _next_block:
    loop {
        if loop_ != 0 && startidx == idx {
            kt.pfrkt_nomatch()
                .set(kt.pfrkt_nomatch().get().wrapping_add(1));
            return Err(1);
        }

        let ke = match pfr_kentry_byidx(kt, idx, af) {
            Some(ke) => ke,
            None => {
                // we don't have this idx, try looping
                let ke = if loop_ != 0 {
                    None
                } else {
                    pfr_kentry_byidx(kt, 0, af)
                };
                let Some(ke) = ke else {
                    kt.pfrkt_nomatch()
                        .set(kt.pfrkt_nomatch().get().wrapping_add(1));
                    return Err(1);
                };
                idx = 0;
                loop_ += 1;
                ke
            }
        };

        // Get current weight for weighted round-robin
        if idx == 0 && use_counter && kt.pfrkt_refcntcost.get() > 0 {
            rpool
                .curweight
                .set(rpool.curweight.get() - i32::from(kt.pfrkt_gcdweight.get()));

            if rpool.curweight.get() < 1 {
                rpool.curweight.set(i32::from(kt.pfrkt_maxweight.get()));
            }
        }

        let mut pmask = PfsockaddrUnion::default();
        pfr_prepare_network(&mut pmask, af, i32::from(ke.pfrke_net.get()));
        PFR_MASK.set(pmask);
        let raddr = sunion2pf(&ke.pfrke_sa.get(), af);
        let rmask = sunion2pf(&PFR_MASK.get(), af);

        let counter = rpool.counter.get();
        if use_counter && !pf_azero(&counter, af) {
            // is supplied address within block?
            if !pf_match_addr(0, &raddr, &rmask, &counter, af) {
                // no, go to next block in table
                idx += 1;
                use_counter = false;
                continue;
            }
            pf_addrcpy(&mut addr, &counter, af);
        } else {
            // use first address of block
            pf_addrcpy(&mut addr, &raddr, af);
        }

        if !kentry_network(ke) {
            // this is a single IP address - no possible nested block
            if rpool.addr.type_.get() == PF_ADDR_DYNIFTL && pfr_islinklocal(af, &addr) {
                idx += 1;
                continue;
            }
            pfr_pool_settle(rpool, kt, ke, idx, &addr, af);
            return Ok((raddr, rmask));
        }
        loop {
            // we don't want to use a nested block
            let head = match af {
                AF_INET => {
                    let mut sin = tmp4.sin();
                    sin.sin_addr = InAddr {
                        s_addr: addr.addr32(0),
                    };
                    tmp4.set_sin(&sin);
                    &kt.pfrkt_ip4
                }
                #[cfg(feature = "inet6")]
                AF_INET6 => {
                    // tmp6, in the same union.
                    let mut sin6 = tmp4.sin6();
                    sin6.sin6_addr = addr.v6();
                    tmp4.set_sin6(&sin6);
                    &kt.pfrkt_ip6
                }
                _ => unhandled_af(i32::from(af)),
            };
            // The entry found is compared and its prefix read under the tree's lock.
            let (same, net2) = {
                let head = pfr_rnh(head);
                let ke2 = head.matches(&tmp4.bytes).copied();
                (
                    ke2.is_some_and(|ke2| ptr::eq(ke2, ke)),
                    ke2.map(|ke2| ke2.pfrke_net.get()),
                )
            };
            if same && !(rpool.addr.type_.get() == PF_ADDR_DYNIFTL && pfr_islinklocal(af, &addr)) {
                // lookup return the same block - perfect
                pfr_pool_settle(rpool, kt, ke, idx, &addr, af);
                return Ok((raddr, rmask));
            }
            // _next_entry:
            let Some(net2) = net2 else {
                panic(format_args!("pfr_pool_get: address outside its block"));
            };
            // we need to increase the counter past the nested block
            pfr_prepare_network(&mut mask, af, i32::from(net2));
            let cur = addr;
            pf_poolmask(
                &mut addr,
                &cur,
                &sunion2pf(&mask, af),
                &PFR_FFADDR.get(),
                af,
            );
            pf_addr_inc(&mut addr, af);
            if !pf_match_addr(0, &raddr, &rmask, &addr, af) {
                // ok, we reached the end of our main block
                // go to next block in table
                idx += 1;
                use_counter = false;
                break;
            }
        }
    }
}

/// `pfr_kentry_byidx`: the `idx`th positive entry of family `af` in `kt`.
pub fn pfr_kentry_byidx(
    kt: &'static PfrKtable,
    idx: i32,
    af: SaFamily,
) -> Option<&'static PfrKentry> {
    let mut w = PfrWalktree::new(PfrwOp::PoolGet);
    w.pfrw_free = idx;

    match af {
        AF_INET => {
            let _ = pfr_walk(&kt.pfrkt_ip4, &mut w);
            match w.pfrw_1 {
                Pfrw1::Kentry(ke) => ke,
                _ => None,
            }
        }
        #[cfg(feature = "inet6")]
        AF_INET6 => {
            let _ = pfr_walk(&kt.pfrkt_ip6, &mut w);
            match w.pfrw_1 {
                Pfrw1::Kentry(ke) => ke,
                _ => None,
            }
        }
        _ => None,
    }
}

/// `pfr_states_increase`: counts one more state on the entry for `addr` (added for load
/// balancing state counter use); its count, -1 without such an entry or memory.
pub fn pfr_states_increase(kt: &'static PfrKtable, addr: &PfAddr, af: SaFamily) -> i32 {
    // Under the tree's lock, as `pfr_update_stats`.
    pfr_kentry_byaddr_with(kt, addr, af, true, |ke| {
        let Some(ke) = ke else {
            return -1;
        };

        if ke.pfrke_counters.get().is_none() {
            ke.pfrke_counters.set(pf_pool_get::<PfrKcounters>(
                &PFR_KCOUNTERS_PL,
                PR_NOWAIT | PR_ZERO,
            ));
        }
        let Some(c) = ke.pfrke_counters.get() else {
            return -1;
        };

        c.states.set(c.states.get().wrapping_add(1));
        c.states.get() as i32
    })
}

/// `pfr_states_decrease`: counts one state less on the entry for `addr` (added for load
/// balancing state counter use); its count, -1 without such an entry or memory.
pub fn pfr_states_decrease(kt: &'static PfrKtable, addr: &PfAddr, af: SaFamily) -> i32 {
    // Under the tree's lock, as `pfr_update_stats`; the debug line after it.
    let (n, below) = pfr_kentry_byaddr_with(kt, addr, af, true, |ke| {
        let Some(ke) = ke else {
            return (-1, false);
        };

        if ke.pfrke_counters.get().is_none() {
            ke.pfrke_counters.set(pf_pool_get::<PfrKcounters>(
                &PFR_KCOUNTERS_PL,
                PR_NOWAIT | PR_ZERO,
            ));
        }
        let Some(c) = ke.pfrke_counters.get() else {
            return (-1, false);
        };

        let below = c.states.get() == 0;
        if !below {
            c.states.set(c.states.get() - 1);
        }
        (c.states.get() as i32, below)
    });
    if below {
        crate::dpfprintf!(LOG_DEBUG, "pfr_states_decrease: states-- when states <= 0");
    }
    n
}

/// `pfr_dynaddr_update`: sets the first address and mask of each family of the table `kt` in
/// the dynamic address `dyn_`, and counts the addresses.
pub fn pfr_dynaddr_update(kt: &'static PfrKtable, dyn_: &'static PfiDynaddr) {
    let mut w = PfrWalktree::new(PfrwOp::DynaddrUpdate);
    w.pfrw_1 = Pfrw1::Dyn(dyn_);

    dyn_.pfid_acnt4.set(0);
    dyn_.pfid_acnt6.set(0);
    match dyn_.pfid_af.get() {
        AF_UNSPEC => {
            // look up all both addresses IPv4 + IPv6
            let _ = pfr_walk(&kt.pfrkt_ip4, &mut w);
            let _ = pfr_walk(&kt.pfrkt_ip6, &mut w);
        }
        AF_INET => {
            let _ = pfr_walk(&kt.pfrkt_ip4, &mut w);
        }
        #[cfg(feature = "inet6")]
        AF_INET6 => {
            let _ = pfr_walk(&kt.pfrkt_ip6, &mut w);
        }
        af => unhandled_af(i32::from(af)),
    }
}

/// `pfr_ktable_winfo_update`: with cost entries, the table's weight gcd (needed for
/// round-robin) and maximum account for the entry `p`.
pub fn pfr_ktable_winfo_update(kt: &'static PfrKtable, p: &'static PfrKentry) {
    // If cost flag is set, gcdweight is needed for round-robin.
    if kt.pfrkt_refcntcost.get() > 0 {
        let weight: u16 = if p.pfrke_type.get() == PFRKE_COST {
            pfr_kentry_cost(p).weight.get()
        } else {
            1
        };

        if kt.pfrkt_gcdweight.get() == 0 {
            kt.pfrkt_gcdweight.set(weight);
        }

        kt.pfrkt_gcdweight
            .set(pfr_gcd(i32::from(weight), i32::from(kt.pfrkt_gcdweight.get())) as u16);

        if kt.pfrkt_maxweight.get() < weight {
            kt.pfrkt_maxweight.set(weight);
        }
    }
}

/// `pfr_ktable_select_active`: `kt` if active, else its root table if that is active.
pub fn pfr_ktable_select_active(kt: &'static PfrKtable) -> Option<&'static PfrKtable> {
    let mut kt = kt;
    if kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE == 0
        && let Some(root) = kt.pfrkt_root.get()
    {
        kt = root;
    }
    if kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE == 0 {
        return None;
    }

    Some(kt)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for pf's tables: creating tables (also anchored ones and their root tables),
    // adding, testing and deleting IPv4 addresses and networks through kernel buffers,
    // longest-prefix matches with negated entries, table and entry counters, a transaction
    // (`pfr_ina_begin`/`define`/`commit`) and round-robin address pools.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;
    use crate::net::pf_ruleset::{PF_ANCHORS, pf_find_anchor};
    use crate::net::pfvar::PfRuleset;

    fn setup() -> MutexGuard<'static, ()> {
        let guard = crate::net::pf_ruleset::tests::setup();
        pfr_initialize();
        PFR_KTABLES.init();
        PFR_KTABLE_CNT.set(0);
        guard
    }

    /// A table description `name` in `anchor`, with `flags`.
    fn table(name: &str, anchor: &str, flags: u32) -> PfrTable {
        let mut t = PfrTable::zeroed();
        strlcpy(&mut t.pfrt_name, name.as_bytes());
        strlcpy(&mut t.pfrt_anchor, anchor.as_bytes());
        t.pfrt_flags.set(flags);
        t
    }

    /// An IPv4 table address `a/net`, negated with `not`.
    fn addr4(a: [u8; 4], net: u8, not: bool) -> PfrAddr {
        PfrAddr {
            pfra_u: pf4(a),
            pfra_af: AF_INET,
            pfra_net: net,
            pfra_not: u8::from(not),
            ..PfrAddr::default()
        }
    }

    /// An address's first four bytes and its feedback code.
    type Feedback = ([u8; 4], u8);

    fn pf4(a: [u8; 4]) -> PfAddr {
        PfAddr::from_v4(InAddr {
            s_addr: u32::from_ne_bytes(a),
        })
    }

    /// Adds the tables of `t` (kernel buffer), returning the count added.
    fn add_tables(t: Vec<PfrTable>) -> i32 {
        let mut t = t;
        let n = t.len() as i32;
        let mut nadd = -1;
        pfr_add_tables(&PfrBuf::Kernel(&mut t), n, Some(&mut nadd), 0).expect("tables added");
        nadd
    }

    /// Adds `addrs` to `tbl` with feedback, returning the count and what the buffer holds
    /// afterwards: each address with its feedback code. The C reports the entries in the order of
    /// its `ioq`, built by inserting at the head: the reverse of the input.
    fn add_addrs(tbl: &mut PfrTable, addrs: &[PfrAddr]) -> Result<(i32, Vec<Feedback>), Errno> {
        let mut v = addrs.to_vec();
        let mut nadd = -1;
        let n = v.len() as i32;
        pfr_add_addrs(
            tbl,
            &mut PfrBuf::Kernel(&mut v),
            n,
            Some(&mut nadd),
            PFR_FLAG_FEEDBACK,
        )?;
        let fb = v
            .iter()
            .map(|a| {
                let b = &a.pfra_u.addr8;
                ([b[0], b[1], b[2], b[3]], a.pfra_fback)
            })
            .collect();
        Ok((nadd, fb))
    }

    #[test]
    fn tables_addresses_and_matches() {
        let _g = setup();

        assert_eq!(add_tables(std::vec![table("t", "", 0)]), 1);
        assert_eq!(PFR_KTABLE_CNT.get(), 1);
        // Adding it again adds nothing.
        assert_eq!(add_tables(std::vec![table("t", "", 0)]), 0);
        let mut tbl = table("t", "", 0);
        let kt = pfr_lookup_table(&tbl).expect("table");
        assert!(kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0);
        assert!(ptr::eq(kt.pfrkt_rs.get().unwrap(), pf_main_ruleset()));
        assert_eq!(pf_main_ruleset().tables.get(), 1);

        let (nadd, fb) = add_addrs(
            &mut tbl,
            &[
                addr4([10, 0, 0, 0], 8, false),
                addr4([10, 1, 0, 0], 16, true),
                addr4([10, 1, 2, 3], 32, false),
                addr4([192, 168, 1, 1], 32, false),
                addr4([10, 1, 2, 3], 32, false),
            ],
        )
        .expect("added");
        assert_eq!(nadd, 4);
        assert_eq!(
            fb,
            [
                ([10, 1, 2, 3], PFR_FB_ADDED),
                ([192, 168, 1, 1], PFR_FB_ADDED),
                ([10, 1, 2, 3], PFR_FB_DUPLICATE),
                ([10, 1, 0, 0], PFR_FB_ADDED),
                ([10, 0, 0, 0], PFR_FB_ADDED),
            ]
        );
        assert_eq!(kt.pfrkt_cnt().get(), 4);

        // Longest prefix wins; the negated /16 hides the /8 under it.
        assert!(pfr_match_addr(kt, &pf4([10, 2, 3, 4]), AF_INET));
        assert!(!pfr_match_addr(kt, &pf4([10, 1, 5, 5]), AF_INET));
        assert!(pfr_match_addr(kt, &pf4([10, 1, 2, 3]), AF_INET));
        assert!(pfr_match_addr(kt, &pf4([192, 168, 1, 1]), AF_INET));
        assert!(!pfr_match_addr(kt, &pf4([192, 168, 1, 2]), AF_INET));
        assert_eq!(kt.pfrkt_match().get(), 3);
        assert_eq!(kt.pfrkt_nomatch().get(), 2);

        // An address already there, and a conflicting negation, are not added.
        let (nadd, fb) = add_addrs(
            &mut tbl,
            &[
                addr4([10, 0, 0, 0], 8, false),
                addr4([192, 168, 1, 1], 32, true),
            ],
        )
        .unwrap();
        assert_eq!(nadd, 0);
        assert_eq!(
            fb,
            [
                ([192, 168, 1, 1], PFR_FB_CONFLICT),
                ([10, 0, 0, 0], PFR_FB_NONE)
            ]
        );
        // Host bits under the mask, and unknown tables, are refused.
        assert_eq!(
            add_addrs(&mut tbl, &[addr4([10, 0, 0, 1], 8, false)]),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            add_addrs(&mut table("nope", "", 0), &[addr4([1, 2, 3, 4], 32, false)]),
            Err(Errno::ESRCH)
        );

        // pfr_tst_addrs.
        let mut v = std::vec![
            addr4([10, 1, 5, 5], 32, false),
            addr4([10, 9, 9, 9], 32, false),
            addr4([1, 2, 3, 4], 32, false),
        ];
        let mut nmatch = -1;
        pfr_tst_addrs(
            &mut tbl,
            &mut PfrBuf::Kernel(&mut v),
            3,
            Some(&mut nmatch),
            0,
        )
        .unwrap();
        assert_eq!(nmatch, 1);
        assert_eq!(v[0].pfra_fback, PFR_FB_NOTMATCH);
        assert_eq!(v[1].pfra_fback, PFR_FB_MATCH);
        assert_eq!(v[2].pfra_fback, PFR_FB_NONE);
        // With PFR_FLAG_REPLACE each address becomes the entry it matched.
        let mut v = std::vec![addr4([10, 9, 9, 9], 32, false)];
        pfr_tst_addrs(
            &mut tbl,
            &mut PfrBuf::Kernel(&mut v),
            1,
            None,
            PFR_FLAG_REPLACE,
        )
        .unwrap();
        assert_eq!(v[0].pfra_net, 8);
        assert_eq!(v[0].pfra_u.addr8[..4], [10, 0, 0, 0]);
        // Networks cannot be tested.
        let mut v = std::vec![addr4([10, 0, 0, 0], 8, false)];
        assert_eq!(
            pfr_tst_addrs(&mut tbl, &mut PfrBuf::Kernel(&mut v), 1, None, 0),
            Err(Errno::EINVAL)
        );

        // pfr_del_addrs.
        let mut v = std::vec![
            addr4([10, 1, 2, 3], 32, false),
            addr4([1, 1, 1, 1], 32, false)
        ];
        let mut ndel = -1;
        pfr_del_addrs(
            &mut tbl,
            &mut PfrBuf::Kernel(&mut v),
            2,
            Some(&mut ndel),
            PFR_FLAG_FEEDBACK,
        )
        .unwrap();
        assert_eq!(ndel, 1);
        assert_eq!(v[0].pfra_fback, PFR_FB_DELETED);
        assert_eq!(v[1].pfra_fback, PFR_FB_NONE);
        assert_eq!(kt.pfrkt_cnt().get(), 3);
        assert!(
            !pfr_match_addr(kt, &pf4([10, 1, 2, 3]), AF_INET),
            "under the negated /16"
        );

        // The size query of pfr_get_addrs (its copy is copyout(9) to a user buffer).
        let mut size = 0;
        pfr_get_addrs(&mut tbl, &mut PfrBuf::Kernel(&mut []), &mut size, 0).unwrap();
        assert_eq!(size, 3);

        // pfr_set_addrs replaces the content.
        let mut v = std::vec![
            addr4([10, 0, 0, 0], 8, false),
            addr4([172, 16, 0, 0], 12, false)
        ];
        let (mut nadd, mut ndel, mut nchange) = (-1, -1, -1);
        pfr_set_addrs(
            &mut tbl,
            &mut PfrBuf::Kernel(&mut v),
            2,
            None,
            Some(&mut nadd),
            Some(&mut ndel),
            Some(&mut nchange),
            0,
            0,
        )
        .unwrap();
        assert_eq!((nadd, ndel, nchange), (1, 2, 0));
        assert_eq!(kt.pfrkt_cnt().get(), 2);
        assert!(pfr_match_addr(kt, &pf4([172, 20, 1, 1]), AF_INET));
        assert!(!pfr_match_addr(kt, &pf4([192, 168, 1, 1]), AF_INET));

        // pfr_clr_addrs empties it.
        let mut ndel = -1;
        pfr_clr_addrs(&mut tbl, Some(&mut ndel), 0).unwrap();
        assert_eq!(ndel, 2);
        assert_eq!(kt.pfrkt_cnt().get(), 0);
        assert_eq!(PFR_KENTRY_PL[PFRKE_PLAIN as usize].pr_nout.get(), 0);

        // pfr_del_tables destroys the table.
        let mut ts = std::vec![table("t", "", 0)];
        let mut ndel = -1;
        pfr_del_tables(&PfrBuf::Kernel(&mut ts), 1, Some(&mut ndel), 0).unwrap();
        assert_eq!(ndel, 1);
        assert!(pfr_lookup_table(&tbl).is_none());
        assert_eq!(PFR_KTABLE_CNT.get(), 0);
        assert_eq!(pf_main_ruleset().tables.get(), 0);
        assert_eq!(PFR_KTABLE_PL.pr_nout.get(), 0);
    }

    #[test]
    fn anchored_tables_and_their_root() {
        let _g = setup();

        assert_eq!(add_tables(std::vec![table("t2", "/an", 0)]), 1);
        // The anchor was rewritten without its slash, and the root table made.
        let kt = pfr_lookup_table(&table("t2", "an", 0)).expect("anchored table");
        let root = pfr_lookup_table(&table("t2", "", 0)).expect("root table");
        assert!(ptr::eq(kt.pfrkt_root.get().unwrap(), root));
        assert_eq!(PFR_KTABLE_CNT.get(), 2);
        assert_eq!(root.pfrkt_refcnt()[PFR_REFCNT_ANCHOR].get(), 1);
        assert!(root.pfrkt_flags().get() & PFR_TFLAG_REFDANCHOR != 0);
        assert!(root.pfrkt_flags().get() & PFR_TFLAG_ACTIVE == 0);
        let an = pf_find_anchor(b"an").expect("the table's anchor");
        assert_eq!(an.ruleset.tables.get(), 1);
        assert_eq!(pfr_table_count(&table("", "an", 0), 0), 1);
        assert_eq!(pfr_table_count(&table("", "", 0), PFR_FLAG_ALLRSETS), 2);
        assert_eq!(pfr_table_count(&table("", "nowhere", 0), 0), -1);

        // An inactive anchored table falls back on an active root.
        let mut rt = table("t2", "", 0);
        assert_eq!(add_tables(std::vec![table("t2", "", 0)]), 0);
        assert!(
            root.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0,
            "reactivated"
        );
        add_addrs(&mut rt, &[addr4([8, 8, 8, 8], 32, false)]).unwrap();
        assert!(pfr_match_addr(root, &pf4([8, 8, 8, 8]), AF_INET));
        kt.pfrkt_flags()
            .set(kt.pfrkt_flags().get() & !PFR_TFLAG_ACTIVE);
        assert!(ptr::eq(pfr_ktable_select_active(kt).unwrap(), root));
        assert!(pfr_kentry_byaddr(kt, &pf4([8, 8, 8, 8]), AF_INET, true).is_some());
        kt.pfrkt_flags()
            .set(kt.pfrkt_flags().get() | PFR_TFLAG_ACTIVE);

        // pfr_get_tables with an anchor filter.
        let mut out: Vec<PfrTable> = (0..2).map(|_| PfrTable::zeroed()).collect();
        let mut size = 2;
        pfr_get_tables(
            &mut table("", "an", 0),
            &mut PfrBuf::Kernel(&mut out),
            &mut size,
            0,
        )
        .unwrap();
        assert_eq!(size, 1);
        assert_eq!(pf_cstr(&out[0].pfrt_name), b"t2");
        assert_eq!(pf_cstr(&out[0].pfrt_anchor), b"an");

        // Deleting the anchored table drops the root's anchor reference: the root, active but
        // neither persistent nor referenced, goes too, and so does the emptied anchor.
        let mut ts = std::vec![table("t2", "an", 0)];
        pfr_del_tables(&PfrBuf::Kernel(&mut ts), 1, None, 0).unwrap();
        assert!(pfr_lookup_table(&table("t2", "an", 0)).is_none());
        assert!(pfr_lookup_table(&table("t2", "", 0)).is_none());
        assert!(pf_find_anchor(b"an").is_none());
        assert!(PF_ANCHORS.is_empty());
        assert_eq!(PFR_KTABLE_CNT.get(), 0);
        assert_eq!(PFR_KTABLE_PL.pr_nout.get(), 0);
    }

    #[test]
    fn counters_follow_the_packets() {
        let _g = setup();
        add_tables(std::vec![table("c", "", PFR_TFLAG_COUNTERS)]);
        let mut tbl = table("c", "", 0);
        add_addrs(&mut tbl, &[addr4([10, 0, 0, 0], 8, false)]).unwrap();
        let kt = pfr_lookup_table(&tbl).unwrap();

        let mut pd = PfPdesc::new();
        pd.af = AF_INET;
        pd.tot_len = 100;
        pd.dir = PF_OUT;
        pfr_update_stats(kt, &pf4([10, 2, 3, 4]), &pd, PF_PASS, false);
        pfr_update_stats(kt, &pf4([10, 2, 3, 4]), &pd, PF_PASS, false);
        pd.dir = crate::net::pfvar::PF_IN;
        pfr_update_stats(kt, &pf4([10, 9, 9, 9]), &pd, PF_DROP, false);
        // Not in the table, though the rule said it was: counted as XPASS.
        pfr_update_stats(kt, &pf4([1, 1, 1, 1]), &pd, PF_PASS, false);

        assert_eq!(kt.pfrkt_packets()[1][PFR_OP_PASS].get(), 2);
        assert_eq!(kt.pfrkt_bytes()[1][PFR_OP_PASS].get(), 200);
        assert_eq!(kt.pfrkt_packets()[0][PFR_OP_BLOCK].get(), 1);
        assert_eq!(kt.pfrkt_packets()[0][PFR_OP_XPASS].get(), 1);

        let mut st = std::vec![PfrAstats::default(); 1];
        let mut size = 1;
        pfr_get_astats(&mut tbl, &mut PfrBuf::Kernel(&mut st), &mut size, 0).unwrap();
        assert_eq!(size, 1);
        assert_eq!(st[0].pfras_a.pfra_net, 8);
        assert_eq!(st[0].pfras_packets[1][PFR_OP_PASS], 2);
        assert_eq!(st[0].pfras_bytes[1][PFR_OP_PASS], 200);
        assert_eq!(st[0].pfras_packets[0][PFR_OP_BLOCK], 1);

        // pfr_clr_astats drops the entry's counters.
        let mut v = std::vec![addr4([10, 0, 0, 0], 8, false)];
        let mut nzero = -1;
        pfr_clr_astats(
            &mut tbl,
            &mut PfrBuf::Kernel(&mut v),
            1,
            Some(&mut nzero),
            0,
        )
        .unwrap();
        assert_eq!(nzero, 1);
        pfr_get_astats(&mut tbl, &mut PfrBuf::Kernel(&mut st), &mut size, 0).unwrap();
        assert_eq!(st[0].pfras_a.pfra_fback, PFR_FB_NOCOUNT);
        assert_eq!(PFR_KCOUNTERS_PL.pr_nout.get(), 0);

        // States of load balancing.
        assert_eq!(
            pfr_states_increase(kt, &pf4([10, 0, 0, 0]), AF_INET),
            -1,
            "not exact"
        );
        let mut v = std::vec![addr4([10, 0, 0, 7], 32, false)];
        pfr_add_addrs(&mut tbl, &mut PfrBuf::Kernel(&mut v), 1, None, 0).unwrap();
        assert_eq!(pfr_states_increase(kt, &pf4([10, 0, 0, 7]), AF_INET), 1);
        assert_eq!(pfr_states_increase(kt, &pf4([10, 0, 0, 7]), AF_INET), 2);
        assert_eq!(pfr_states_decrease(kt, &pf4([10, 0, 0, 7]), AF_INET), 1);

        // Table counters cleared.
        let mut ts = std::vec![table("c", "", 0)];
        let mut nzero = -1;
        pfr_clr_tstats(&PfrBuf::Kernel(&mut ts), 1, Some(&mut nzero), 0).unwrap();
        assert_eq!(nzero, 1);
        assert_eq!(kt.pfrkt_packets()[1][PFR_OP_PASS].get(), 0);
    }

    #[test]
    fn transaction_define_and_commit() {
        let _g = setup();
        let trs = table("", "", 0);

        net_lock();
        pf_lock();
        let mut ticket = 0;
        pfr_ina_begin(&trs, Some(&mut ticket), None, 0).unwrap();
        assert!(pf_main_ruleset().topen.get() != 0);

        let mut tbl = table("def", "", 0);
        let a = PfrBuf::Kernel(&mut []);
        assert_eq!(
            pfr_ina_define(&mut tbl, &a, 0, None, None, ticket + 1, 0),
            Err(Errno::EBUSY)
        );
        let mut v = std::vec![addr4([172, 16, 0, 0], 12, false)];
        let (mut nadd, mut naddr) = (-1, -1);
        let mut tbl = table("def", "", PFR_TFLAG_PERSIST);
        pfr_ina_define(
            &mut tbl,
            &PfrBuf::Kernel(&mut v),
            1,
            Some(&mut nadd),
            Some(&mut naddr),
            ticket,
            PFR_FLAG_ADDRSTOO,
        )
        .unwrap();
        assert_eq!((nadd, naddr), (1, 1));
        let kt = pfr_lookup_table(&table("def", "", 0)).expect("inactive table");
        assert!(kt.pfrkt_flags().get() & PFR_TFLAG_INACTIVE != 0);
        assert!(
            !pfr_match_addr(kt, &pf4([172, 20, 1, 1]), AF_INET),
            "not active yet"
        );

        let (mut nadd, mut nchange) = (-1, -1);
        pfr_ina_commit(&trs, ticket, Some(&mut nadd), Some(&mut nchange), 0).unwrap();
        assert_eq!((nadd, nchange), (1, 0));
        assert!(kt.pfrkt_flags().get() & PFR_TFLAG_ACTIVE != 0);
        assert!(kt.pfrkt_flags().get() & PFR_TFLAG_INACTIVE == 0);
        assert!(kt.pfrkt_shadow.get().is_none());
        assert!(pfr_match_addr(kt, &pf4([172, 20, 1, 1]), AF_INET));
        assert_eq!(pf_main_ruleset().topen.get(), 0);

        // A second transaction merges into the active table.
        pfr_ina_begin(&trs, Some(&mut ticket), None, 0).unwrap();
        let mut v = std::vec![
            addr4([192, 0, 2, 0], 24, false),
            addr4([172, 16, 0, 0], 12, true)
        ];
        let mut tbl = table("def", "", PFR_TFLAG_PERSIST);
        pfr_ina_define(
            &mut tbl,
            &PfrBuf::Kernel(&mut v),
            2,
            Some(&mut nadd),
            Some(&mut naddr),
            ticket,
            PFR_FLAG_ADDRSTOO,
        )
        .unwrap();
        // The C counts an active table being redefined as added.
        assert_eq!((nadd, naddr), (1, 2));
        pfr_ina_commit(&trs, ticket, Some(&mut nadd), Some(&mut nchange), 0).unwrap();
        assert_eq!((nadd, nchange), (0, 1));
        assert!(kt.pfrkt_flags().get() & PFR_TFLAG_PERSIST != 0);
        assert!(pfr_match_addr(kt, &pf4([192, 0, 2, 5]), AF_INET));
        assert!(
            !pfr_match_addr(kt, &pf4([172, 20, 1, 1]), AF_INET),
            "now negated"
        );
        assert_eq!(kt.pfrkt_cnt().get(), 2);

        // A rolled back transaction leaves the table alone.
        pfr_ina_begin(&trs, Some(&mut ticket), None, 0).unwrap();
        let mut tbl = table("def", "", 0);
        pfr_ina_define(&mut tbl, &PfrBuf::Kernel(&mut []), 0, None, None, ticket, 0).unwrap();
        let mut ndel = -1;
        pfr_ina_rollback(&trs, ticket, Some(&mut ndel), 0).unwrap();
        assert_eq!(ndel, 1);
        assert!(kt.pfrkt_shadow.get().is_none());
        assert!(pfr_match_addr(kt, &pf4([192, 0, 2, 5]), AF_INET));
        pf_unlock();
        net_unlock();
    }

    #[test]
    fn attach_and_detach_tables() {
        let _g = setup();

        pf_lock();
        let rs: &'static PfRuleset = pf_main_ruleset();
        let kt = pfr_attach_table(rs, b"rt", PR_WAITOK).expect("table");
        assert_eq!(kt.pfrkt_refcnt()[PFR_REFCNT_RULE].get(), 1);
        assert!(kt.pfrkt_flags().get() & PFR_TFLAG_REFERENCED != 0);
        assert!(ptr::eq(pfr_attach_table(rs, b"rt", PR_WAITOK).unwrap(), kt));
        pfr_detach_table(kt);
        assert!(pfr_lookup_table(&table("rt", "", 0)).is_some());
        pfr_detach_table(kt);
        assert!(
            pfr_lookup_table(&table("rt", "", 0)).is_none(),
            "unreferenced, destroyed"
        );
        assert_eq!(PFR_KTABLE_CNT.get(), 0);
        pf_unlock();

        // pfr_insert_kentry / pfr_remove_kentry, as pf's overload tables use them.
        add_tables(std::vec![table("ov", "", 0)]);
        let kt = pfr_lookup_table(&table("ov", "", 0)).unwrap();
        pfr_insert_kentry(kt, &addr4([203, 0, 113, 9], 32, false), 0).unwrap();
        pfr_insert_kentry(kt, &addr4([203, 0, 113, 9], 32, false), 0).unwrap();
        assert_eq!(kt.pfrkt_cnt().get(), 1);
        assert!(pfr_match_addr(kt, &pf4([203, 0, 113, 9]), AF_INET));
        pfr_remove_kentry(kt, &addr4([203, 0, 113, 9], 32, false)).unwrap();
        assert_eq!(
            pfr_remove_kentry(kt, &addr4([203, 0, 113, 9], 32, false)),
            Err(Errno::ESRCH)
        );
        assert_eq!(kt.pfrkt_cnt().get(), 0);
    }

    /// A kernel pool over the table `kt`.
    fn rr_pool(kt: &'static PfrKtable) -> &'static PfPool {
        let p: &'static PfPool = Box::leak(Box::new(PfPool::zeroed()));
        p.addr.type_.set(PF_ADDR_TABLE);
        p.addr.set_tbl(Some(kt));
        p
    }

    #[test]
    fn pool_get_round_robin() {
        let _g = setup();
        add_tables(std::vec![table("rr", "", 0)]);
        let mut tbl = table("rr", "", 0);
        add_addrs(
            &mut tbl,
            &[
                addr4([10, 0, 0, 1], 32, false),
                addr4([10, 0, 0, 2], 32, false),
                addr4([10, 0, 0, 8], 31, false),
                addr4([10, 0, 0, 5], 32, true),
            ],
        )
        .unwrap();
        let kt = pfr_lookup_table(&tbl).unwrap();
        let rpool = rr_pool(kt);

        let mut got = Vec::new();
        for _ in 0..5 {
            let (raddr, rmask) = pfr_pool_get(rpool, AF_INET).expect("an address");
            let c = rpool.counter.get();
            assert!(
                pf_match_addr(0, &raddr, &rmask, &c, AF_INET),
                "inside its block"
            );
            got.push(c.addr8[3]);
            // pf_lb steps the counter past the address it used.
            let mut c = c;
            pf_addr_inc(&mut c, AF_INET);
            rpool.counter.set(c);
        }
        // The negated 10.0.0.5 is skipped; the /31 gives both its addresses; then around.
        assert_eq!(got, [1, 2, 8, 9, 1]);
        assert_eq!(rpool.weight.get(), 1);

        // An empty table has nothing to give; a pool not over a table is -1.
        add_tables(std::vec![table("empty", "", 0)]);
        let empty = pfr_lookup_table(&table("empty", "", 0)).unwrap();
        assert_eq!(pfr_pool_get(rr_pool(empty), AF_INET), Err(1));
        let other: &'static PfPool = Box::leak(Box::new(PfPool::zeroed()));
        assert_eq!(pfr_pool_get(other, AF_INET), Err(-1));
    }

    #[test]
    fn validate_and_fix() {
        let _g = setup();
        assert!(pfr_validate_addr(&addr4([10, 0, 0, 0], 8, false)));
        assert!(pfr_validate_addr(&addr4([0, 0, 0, 0], 0, false)));
        assert!(
            !pfr_validate_addr(&addr4([10, 0, 0, 1], 8, false)),
            "host bits"
        );
        assert!(
            !pfr_validate_addr(&addr4([10, 0, 0, 0], 33, false)),
            "prefix"
        );
        let mut a = addr4([10, 0, 0, 0], 8, false);
        a.pfra_not = 2;
        assert!(!pfr_validate_addr(&a));
        let mut a = addr4([10, 0, 0, 0], 8, false);
        a.pfra_af = 0;
        assert!(!pfr_validate_addr(&a));

        let mut anchor = [0u8; PATH_MAX];
        anchor[..6].copy_from_slice(b"///a/b");
        pfr_fix_anchor(&mut anchor).unwrap();
        assert_eq!(pf_cstr(&anchor), b"a/b");
        assert!(anchor[PATH_MAX - 3..].iter().all(|&b| b == 0));
        let mut anchor = [b'x'; PATH_MAX];
        assert_eq!(pfr_fix_anchor(&mut anchor), Err(Errno::ENAMETOOLONG));

        let mut t = table("", "", 0);
        assert_eq!(pfr_validate_table(&mut t, 0, false), Err(Errno::EINVAL));
        let mut t = table("x", "_pf", 0);
        assert_eq!(pfr_validate_table(&mut t, 0, true), Err(Errno::EINVAL));
        assert_eq!(pfr_validate_table(&mut t, 0, false), Ok(()));
        let mut t = table("x", "", PFR_TFLAG_PERSIST);
        assert_eq!(pfr_validate_table(&mut t, 0, false), Err(Errno::EINVAL));
        assert_eq!(pfr_validate_table(&mut t, PFR_TFLAG_USRMASK, false), Ok(()));

        assert_eq!(pfr_gcd(12, 18), 6);
        assert_eq!(pfr_gcd(0, 5), 5);
    }

    /// An IPv6 address from its eight 16-bit groups.
    #[cfg(feature = "inet6")]
    fn pf6(g: [u16; 8]) -> PfAddr {
        let mut a = PfAddr::zeroed();
        for (i, w) in g.iter().enumerate() {
            a.addr8[2 * i..2 * i + 2].copy_from_slice(&w.to_be_bytes());
        }
        a
    }

    #[cfg(feature = "inet6")]
    #[test]
    fn ipv6_networks_and_hosts() {
        let _g = setup();

        assert_eq!(add_tables(std::vec![table("t6", "", 0)]), 1);
        let mut tbl = table("t6", "", 0);
        let kt = pfr_lookup_table(&tbl).expect("table");

        let a6 = |g, net, not: bool| PfrAddr {
            pfra_u: pf6(g),
            pfra_af: AF_INET6,
            pfra_net: net,
            pfra_not: u8::from(not),
            ..PfrAddr::default()
        };
        let net64 = a6([0x2001, 0xdb8, 1, 2, 0, 0, 0, 0], 64, false);
        let host = a6([0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x99], 128, false);
        // Host bits past the prefix make the address invalid.
        let bad = a6([0x2001, 0xdb8, 1, 2, 0, 0, 0, 1], 64, false);
        assert!(!pfr_validate_addr(&bad));
        assert!(pfr_validate_addr(&net64));
        assert!(!pfr_validate_addr(&a6([0; 8], 129, false)));

        let (nadd, _) = add_addrs(&mut tbl, &[net64, host]).expect("added");
        assert_eq!(nadd, 2);
        assert_eq!(kt.pfrkt_cnt().get(), 2);

        assert!(pfr_match_addr(
            kt,
            &pf6([0x2001, 0xdb8, 1, 2, 0xa, 0xb, 0xc, 0xd]),
            AF_INET6
        ));
        assert!(!pfr_match_addr(
            kt,
            &pf6([0x2001, 0xdb8, 1, 3, 0, 0, 0, 1]),
            AF_INET6
        ));
        assert!(pfr_match_addr(
            kt,
            &pf6([0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x99]),
            AF_INET6
        ));
        assert!(!pfr_match_addr(
            kt,
            &pf6([0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x98]),
            AF_INET6
        ));
        // An IPv4 lookup does not see the IPv6 entries.
        assert!(!pfr_match_addr(kt, &pf4([32, 1, 13, 184]), AF_INET));

        // The /64 entry reads back with its prefix.
        let ke = pfr_lookup_addr(kt, &net64, true).expect("exact /64");
        let mut ad = PfrAddr::default();
        pfr_copyout_addr(&mut ad, Some(ke));
        assert_eq!((ad.pfra_af, ad.pfra_net), (AF_INET6, 64));
        assert_eq!(ad.pfra_ip6addr(), net64.pfra_ip6addr());
    }
    /// xorshift64*: the model test's generator, deterministic from its seed.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            x.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// An entry of the model: the network `a/net` (a host at 32), negated or not.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct Model {
        a: [u8; 4],
        net: u8,
        not: bool,
    }

    /// The mask of a `net`-bit prefix.
    fn prefix_mask(net: u8) -> u32 {
        if net == 0 {
            0
        } else {
            u32::MAX << (32 - u32::from(net))
        }
    }

    /// An address of a small universe, so that networks nest and share prefixes.
    fn model_addr(rng: &mut Rng) -> [u8; 4] {
        [
            10,
            [0u8, 1, 2, 3][rng.below(4)],
            [0u8, 1, 128, 255][rng.below(4)],
            [0u8, 1, 77, 255][rng.below(4)],
        ]
    }

    /// The model's answer for `x`: the most specific entry holding it (the longest prefix,
    /// a host before every network), matched unless negated.
    fn model_match(model: &[Model], x: [u8; 4]) -> bool {
        let x = u32::from_be_bytes(x);
        model
            .iter()
            .filter(|e| x & prefix_mask(e.net) == u32::from_be_bytes(e.a))
            .max_by_key(|e| e.net)
            .is_some_and(|e| !e.not)
    }

    /// N6.7b1 (pf's tables on the kernel's radix tree): random adds, deletes and lookups of
    /// IPv4 hosts and nested networks, some negated, through pf's own ioctl functions; at every
    /// step `pfr_match_addr` answers as a linear model of the longest prefix and the negation,
    /// the table's counters follow, and its walks (a delete's full scan, the enqueue, the
    /// clear) visit every entry once.
    #[test]
    fn table_matches_answer_as_the_longest_prefix_model() {
        let _g = setup();
        assert_eq!(add_tables(std::vec![table("model", "", 0)]), 1);
        let mut tbl = table("model", "", 0);
        let kt = pfr_lookup_table(&tbl).expect("table");
        let (mut matched, mut missed) = (0u64, 0u64);
        let mut steps = 0;
        for seed in 1..=20u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
            let mut model: Vec<Model> = Vec::new();
            for _ in 0..300 {
                steps += 1;
                match rng.below(20) {
                    0..=8 => {
                        let net = [0u8, 8, 9, 15, 16, 17, 24, 25, 31, 32, 32, 32][rng.below(12)];
                        let a = (u32::from_be_bytes(model_addr(&mut rng)) & prefix_mask(net))
                            .to_be_bytes();
                        let e = Model {
                            a,
                            net,
                            not: rng.below(4) == 0,
                        };
                        let there = model.iter().any(|m| m.a == e.a && m.net == e.net);
                        let (nadd, _) =
                            add_addrs(&mut tbl, &[addr4(e.a, e.net, e.not)]).expect("added");
                        assert_eq!(nadd, i32::from(!there), "{e:?}");
                        if !there {
                            model.push(e);
                        }
                    }
                    9..=11 => {
                        // A live entry (its negation flipped now and then: a conflict, kept),
                        // or an absent one.
                        let (e, flip) = if !model.is_empty() && rng.below(4) != 0 {
                            (model[rng.below(model.len())], rng.below(5) == 0)
                        } else {
                            let net = [8u8, 16, 24, 32][rng.below(4)];
                            let a = (u32::from_be_bytes(model_addr(&mut rng)) & prefix_mask(net))
                                .to_be_bytes();
                            (Model { a, net, not: false }, false)
                        };
                        let mut v = std::vec![addr4(e.a, e.net, e.not != flip)];
                        let mut ndel = -1;
                        pfr_del_addrs(&mut tbl, &mut PfrBuf::Kernel(&mut v), 1, Some(&mut ndel), 0)
                            .expect("deleted");
                        let i = model
                            .iter()
                            .position(|m| m.a == e.a && m.net == e.net && m.not == (e.not != flip));
                        assert_eq!(ndel, i32::from(i.is_some()), "{e:?} flip {flip}");
                        if let Some(i) = i {
                            model.swap_remove(i);
                        }
                    }
                    12..=18 => {
                        let x = model_addr(&mut rng);
                        let want = model_match(&model, x);
                        let (m0, n0) = (kt.pfrkt_match().get(), kt.pfrkt_nomatch().get());
                        assert_eq!(pfr_match_addr(kt, &pf4(x), AF_INET), want, "{x:?}");
                        assert_eq!(kt.pfrkt_match().get() - m0, u64::from(want));
                        assert_eq!(kt.pfrkt_nomatch().get() - n0, u64::from(!want));
                        if want {
                            matched += 1;
                        } else {
                            missed += 1;
                        }
                    }
                    _ => {
                        let workq = SlistHead::<PfrKentryworkq>::new();
                        let mut n = -1;
                        pfr_enqueue_addrs(kt, &workq, Some(&mut n), false);
                        assert_eq!(n as usize, model.len());
                        assert_eq!(sl_iter(&workq).count(), model.len());
                        assert_eq!(kt.pfrkt_cnt().get() as usize, model.len());
                    }
                }
            }
            let mut ndel = -1;
            pfr_clr_addrs(&mut tbl, Some(&mut ndel), 0).expect("cleared");
            assert_eq!(ndel as usize, model.len());
            assert_eq!(kt.pfrkt_cnt().get(), 0);
            assert!(!pfr_match_addr(kt, &pf4([10, 0, 0, 1]), AF_INET));
        }
        assert_eq!(PFR_KENTRY_PL[PFRKE_PLAIN as usize].pr_nout.get(), 0);
        std::println!("pf table model: {steps} steps, {matched} matches, {missed} misses");
        assert!(matched > 500 && missed > 500, "both answers seen");
    }

    /// N6.7b1: a destroyed table gives its trees' memory back before its pool item goes
    /// (the pool runs no drop), and a table made again in that item starts empty.
    #[test]
    fn a_destroyed_table_gives_its_trees_back() {
        let _g = setup();
        let rtable = kernel::net::rtable::M_RTABLE;
        // The pfrktable pool's item (vmstat -m's size, D-12): the table and its two trees.
        std::eprintln!(
            "pfrktable item: {} bytes, a tree {} bytes",
            size_of::<PfrKtable>(),
            size_of::<PfrRadix>()
        );
        let before = kernel::testutil::malloc_out(rtable);
        assert_eq!(add_tables(std::vec![table("gone", "", 0)]), 1);
        let mut tbl = table("gone", "", 0);
        let v: Vec<PfrAddr> = (0..100u8)
            .map(|i| addr4([10, 9, i, 0], 24, false))
            .collect();
        assert_eq!(add_addrs(&mut tbl, &v).expect("added").0, 100);
        assert!(kernel::testutil::malloc_out(rtable) > before);
        let mut ts = std::vec![table("gone", "", 0)];
        let mut ndel = -1;
        pfr_del_tables(&PfrBuf::Kernel(&mut ts), 1, Some(&mut ndel), 0).expect("deleted");
        assert_eq!(ndel, 1);
        assert_eq!(
            kernel::testutil::malloc_out(rtable),
            before,
            "the trees' blocks"
        );
        assert_eq!(PFR_KTABLE_PL.pr_nout.get(), 0);

        // A table made again (in a pool item the old one may have been): empty, then its own
        // entries only.
        assert_eq!(add_tables(std::vec![table("again", "", 0)]), 1);
        let mut tbl = table("again", "", 0);
        let kt = pfr_lookup_table(&tbl).expect("table");
        assert_eq!(kt.pfrkt_cnt().get(), 0);
        assert!(!pfr_match_addr(kt, &pf4([10, 9, 3, 1]), AF_INET));
        add_addrs(&mut tbl, &[addr4([10, 9, 3, 0], 24, false)]).expect("added");
        assert!(pfr_match_addr(kt, &pf4([10, 9, 3, 1]), AF_INET));
        assert!(!pfr_match_addr(kt, &pf4([10, 9, 4, 1]), AF_INET));
    }

    /// N6.7b1 B1 (a table's trees under their own lock, the user's decision of 2026-11-24): two
    /// holders look a table up at once; a writer (`pfr_insert_kentry`) that comes while they
    /// hold it waits, and a lookup that comes after the writer waits behind it; then two threads
    /// look up while this one adds and deletes hosts in the same tree, and every answer is the
    /// table's. The `RefCell` this replaced had a borrow counter that concurrent lookups wrote
    /// without a lock. What the host cannot show: the softnets' and the purge task's own
    /// contexts, spl, or a lost update of the old counter (a race, not a sequence).
    #[test]
    fn lookups_share_a_table_while_a_writer_changes_it() {
        use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use std::time::{Duration, Instant};

        let _g = setup();
        assert_eq!(add_tables(std::vec![table("shared", "", 0)]), 1);
        let mut tbl = table("shared", "", 0);
        let kt = pfr_lookup_table(&tbl).expect("table");
        let nets: Vec<PfrAddr> = (0..64u8).map(|i| addr4([10, i, 0, 0], 24, false)).collect();
        assert_eq!(add_addrs(&mut tbl, &nets).expect("added").0, 64);
        let net_of = |a: [u8; 4]| {
            pfr_kentry_byaddr(kt, &pf4(a), AF_INET, false).map(|ke| ke.pfrke_net.get())
        };
        let host = |k: u32| addr4([192, 168, (k >> 8) as u8, k as u8], 32, false);

        // Two holders at once, and a writer between them and the next reader.
        let first = kt.pfrkt_ip4.read();
        std::thread::scope(|sc| {
            sc.spawn(|| {
                let second = kt
                    .pfrkt_ip4
                    .try_read()
                    .expect("shared with the first holder");
                let key = pfr_sin_key(&pf4([10, 3, 0, 9]));
                assert_eq!(
                    second.matches(&key.bytes).map(|ke| ke.pfrke_net.get()),
                    Some(24)
                );
                assert!(
                    kt.pfrkt_ip4.try_write().is_none(),
                    "no writer while two hold it"
                );
            })
            .join()
            .expect("the second holder");
            let writer = sc.spawn(|| pfr_insert_kentry(kt, &host(1), 0));
            // The writer spins, then sleeps on the lock's writers' channel.
            let until = Instant::now() + Duration::from_secs(10);
            while kt.pfrkt_ip4.try_read().map(drop).is_some() {
                assert!(Instant::now() < until, "the writer never waited");
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(!writer.is_finished(), "the writer waits for the holder");
            drop(first);
            assert_eq!(writer.join().expect("the writer"), Ok(()));
        });
        assert_eq!(net_of([192, 168, 0, 1]), Some(32));
        assert_eq!(net_of([10, 63, 0, 200]), Some(24));
        assert_eq!(kt.pfrkt_cnt().get(), 65);

        // Two lookers while this thread adds and deletes hosts beside their networks.
        let done = AtomicBool::new(false);
        let looked = AtomicU64::new(0);
        std::thread::scope(|sc| {
            for t in 0..2u8 {
                let (done, looked) = (&done, &looked);
                sc.spawn(move || {
                    let mut i = 0u32;
                    while !done.load(Ordering::Relaxed) || i < 2_000 {
                        let n = (i % 64) as u8;
                        assert_eq!(net_of([10, n, 0, t.wrapping_add(i as u8)]), Some(24));
                        assert_eq!(net_of([172, 16, n, 1]), None);
                        i += 1;
                    }
                    looked.fetch_add(u64::from(i), Ordering::Relaxed);
                });
            }
            for round in 0..20u32 {
                for k in 2..202u32 {
                    assert_eq!(pfr_insert_kentry(kt, &host(k), 0), Ok(()));
                }
                for k in (2..202u32).filter(|k| (k + round) % 3 != 0) {
                    assert_eq!(pfr_remove_kentry(kt, &host(k)), Ok(()));
                }
                for k in (2..202u32).filter(|k| (k + round) % 3 == 0) {
                    assert_eq!(pfr_remove_kentry(kt, &host(k)), Ok(()));
                }
            }
            done.store(true, Ordering::Relaxed);
        });
        assert!(looked.load(Ordering::Relaxed) >= 4_000);
        assert_eq!(kt.pfrkt_cnt().get(), 65);
        assert_eq!(net_of([192, 168, 0, 2]), None);
        assert_eq!(net_of([192, 168, 0, 1]), Some(32));
        std::eprintln!(
            "pf table lock: {} lookups beside 8,000 adds and deletes",
            looked.load(Ordering::Relaxed) * 2
        );
    }
}
/* </TESTS> */
