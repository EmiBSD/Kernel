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
/* </LICENSES> */

/* <CODE> */
//! `TypedPool<T>` and `PoolArc<T>`: a `pool(9)` of `T`s whose items are owned, counted
//! handles; `BoxPool<T>` and `PoolBox<T>`: one whose items have exactly one owner.
//!
//! The C's `pool_get(pp, flags)` returns a `void *` the caller casts and initialises, and
//! `pool_put(pp, v)` takes it back, wherever the caller decides; LZ keeps the shape and hands
//! the item out as a `&'static T` (`pipe_pair_create`), which the pool frees under it. Here a
//! [`TypedPool<T>`] makes items holding a `T` and a count: [`TypedPool::get_shared`] returns
//! a [`PoolArc<T>`], an owned handle that `Clone` counts and whose last drop drops the `T` and
//! puts the item back. No `&'static T` is ever made from an item that can be freed (decision
//! 10): a reference borrows a handle, or the handle is given up for good, the item never put
//! back ([`PoolArc::leak`]).
//!
//! The items come from `bsd`'s `pool(9)` through the [`PoolBackend`] bridge (bridges.toml;
//! paid by N9, when the old runtime files go): each `TypedPool` is one pool of its own, named
//! and listed as the C's are, so `vmstat -m` and `systat pool` show it with its `wchan` as
//! they show every pool. The bridge is a safe trait (decision 30): it returns a pointer and
//! the size granted, and builds nothing unsafe.
//!
//! **Before registration** (early boot, host tests that register none) there is no pool
//! allocator: [`TypedPool::init`] fails with `ENOMEM` and every `get_shared` with `None`, as a
//! `pool_get` that cannot get a page does with `PR_NOWAIT`.
//!
//! ## Deviations
//! - An item is an `Inner<T>`: the `T` with its reference count and its pool, 16 bytes (and
//!   any alignment padding) more than the C's item, so the Size column of `vmstat -m` (and
//!   `systat pool`) differs for a pool made through `TypedPool` from the C's pool of that
//!   object. No `diff-openbsd` scenario prints `vmstat -m`; the first such pool is
//!   pipe(2)'s (N2.6), whose name and counts are the C's.
//!
//! ## Redesign
//! - `pool_init(&pp, sizeof(T), align, ipl, flags, wchan, NULL)` -> [`TypedPool::init`],
//!   sized and aligned by the type; `pool_get` + initialisation -> [`TypedPool::get_shared`]
//!   (the value written before anyone sees the item, `PR_ZERO` unnecessary); the matching
//!   `pool_put` -> the last [`PoolArc`]'s drop, after the `T`'s own drop. A double put or a
//!   put into the wrong pool cannot be written.
//! - Writing the `T` into an item, and every reference to it, rest on the registration
//!   contract of the bridge (the kernel's boot path registers its own `pool(9)`, once, before
//!   any user; `sys/hal/trust.rs`): a documented trust boundary, not a compile-time proof.
//!   `hal` checks every item it is given (aligned for the item, the size granted; a probe of
//!   the first and last byte under `diagnostic`) and panics naming the bridge when one
//!   fails. It goes with the bridge, at N9.
//! - No raw form (`into_raw`/`from_raw`) yet: its first user would be a type-erased slot, and
//!   the kernel's slots hold the handle itself (`FileData`, decision 14).
//! - N3.1 (D18): [`BoxPool<T>`] and [`PoolBox<T>`], the owned item without a count, for the
//!   pmaps' per-mapping records (amd64's `pv_entry`, pool `"pvpl"`; arm64's `pte_desc`, pool
//!   `"pted"`), which the C gets with `pool_get` and frees with `pool_put` from exactly one
//!   owner: the page's pv chain (`Option<PoolBox<PvEntry>>`, each entry owning the next) or
//!   the table slot that maps it. `BoxPool::get` writes the value before the handle exists;
//!   the handle derefs mutably (it is the only one) and its drop drops the `T` and puts the
//!   item back. It is its own pool type, not a mode of `TypedPool`: a `TypedPool` item is an
//!   `Inner<T>` (the count and the pool beside the value), and an uncounted item needs
//!   neither, so a `BoxPool` item is exactly a `T`, the C's item, and `vmstat -m` shows the
//!   C's size for it; the pool the item goes back to rides in the handle instead (the
//!   handle is two words: an `Option<PoolBox<T>>` is too, by the niche of its pointer).
//! - N3.1's review: [`SharedBox<T>`] ([`BoxPool::get_linked`]) is the owned item that raw
//!   links also reach (arm64's `pte_desc`, owned by its table slot and linked on its page's pv
//!   list): a `PoolBox` without `DerefMut`, so a `&mut` through the owner never coexists with
//!   a link's reader; `PoolBox` keeps `DerefMut` for an item that only its owner reaches
//!   (amd64's `pv_entry`, owned by its chain). [`PoolArc::from_value`] rebuilds a counted
//!   handle from a pointer to the value a record keeps (`pmap_reference(pve->pv_pmap)`).
//! - N3.1 commit 5: the raw form of a [`SharedBox`], for the type-erased slot the first item
//!   above waited for: arm64's level-3 table keeps its `pte_desc`s in a page of 512 words
//!   (the C's `struct pmapvp3`'s `vp[]`, beside the 512 entries the MMU reads), where a
//!   two-word handle does not fit. [`SharedBox::into_raw`] gives the handle up for the
//!   pointer (the slot owns the item then); [`SharedBox::from_raw`] takes it back, once, to
//!   free it. The pool rides in the handle, so `from_raw` names it.
//! - N3.4a: [`PoolArc::get_mut`] (`Arc::get_mut`: the value mutably while the handle is the
//!   only one; `amap_copy` fills its new amap so before installing it) and [`PoolArc::leak`]
//!   (the handle given up for a true `&'static T`: the C's vnode objects, never freed). A
//!   pool whose values records reach by address ([`PoolArc::from_value`], the pmaps) is made
//!   by [`TypedPool::init_recorded`] and never gives a value mutably; `from_value` refuses
//!   any other pool. `pool_sethiwat(9)` and `pool_cache_init(9)` are
//!   [`TypedPool::set_hiwat`]/[`TypedPool::cache_init`] (and [`BoxPool`]'s), two more
//!   methods of the [`RawPool`] bridge (`uvm_anon_init`, `amap_init`).
//! - N3.5 S0: the raw form of a [`PoolBox`] (`into_raw`/`from_raw`, crate-private), for the
//!   owning containers of `sys/hal/owned.rs`, which hold an element's item while it is linked
//!   and give its box back once. [`StaticArena<T, N>`] is a pool that is not a `pool(9)`: `N`
//!   slots in a `static` and, beyond them, pages given up for good ([`StaticArena::extend`]),
//!   carved into slots that are never freed (the C's `kernel_map_entry[MAX_KMAPENT]` and the
//!   `km_alloc`'d pages of `uvm_mapent_alloc`, which `vmstat -m` does not show either);
//!   [`BoxPool::init_arena`] makes a `BoxPool` hand out its slots, so an entry from the
//!   arena is the same `PoolBox` as one from `"vmmpepl"`. Its free list is a `Mutexed` at
//!   `IPL_VM` (`uvm_kmapent_mtx`).
//! - N4.1: [`PoolArc::as_pin`], the value pinned: a pool item never moves while a handle
//!   lives and is dropped in place before it goes back (the pinned lists' elements,
//!   `hal::pinlist`: a legacy timeout's node, a task queue). [`PoolArc::get_mut`], the one
//!   `&mut T`, is for `T: Unpin` only, so it can never move a pinned value.
//! - N4.2: [`PoolBox::as_pin`], as `PoolArc`'s (a thread on its process's lists while its one
//!   owner keeps it), its `DerefMut` for `T: Unpin` only; [`PoolArc::from_ref`], a counted
//!   handle back from a `&T` a list lends (`prfind`'s caller that keeps the process): a value
//!   of a recorded pool records its own place in a [`PoolHome`] field ([`pool_home!`]), set
//!   once by [`TypedPool::get_homed`] after the value is written into its item, so a value
//!   made anywhere else (proc0's `static`, a stack) has an empty home and gets `None`, never
//!   a handle to memory no pool owns. [`TypedPool::init_recorded`] is public for such pools.
//! - N4.3: three more forms of the count, for `struct file`'s `f_count` (`kernel::file`).
//!   [`PoolArc::try_last`] is `FRELE`'s decrement that tells the last one: it gives back a
//!   [`Last<T>`], the item at count 0 that only reads the value (`fdrop` calls the file's
//!   fallible close through it and returns its error) and frees it when dropped; a handle's
//!   own last drop is the same path. [`PoolArc::try_from_ref`] is `from_ref` answering `None`
//!   for a value at count 0 instead of stopping the kernel (`fd_iterfile` skips a file whose
//!   last `FRELE` is under way while `filehead` still lends it). [`PoolArc::forget`] and
//!   [`PoolArc::adopt`] park a count and take it back: a reference held by a word in legacy
//!   memory (a descriptor in flight in a unix-domain message, `syslogf`), counted by
//!   [`PoolArc::count`] as the C's `f_count` counts it, adopted once into a handle again. The
//!   word `Inner::refs` holds both: the references (handles and parked ones) in its low half,
//!   the parked ones among them in its high half, so an item never grows.

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, MaybeUninit, offset_of};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering, fence};

use crate::assert::hal_panic;
use crate::bridges;
use crate::intr::IPL_VM;
use crate::kmbuf::{KmBuf, PageKind};
use crate::link::Link;
use crate::mutex::Mutexed;
use crate::once::OnceStatic;
use crate::sys::errno::Errno;
use crate::trust::check_block;

/// `PR_WAITOK`: `pool_get` may sleep for an item.
pub const PR_WAITOK: i32 = 0x0001;
/// `PR_NOWAIT`: `pool_get` fails instead of sleeping.
pub const PR_NOWAIT: i32 = 0x0002;
/// `PR_LIMITFAIL`: `pool_get` fails at the hard limit even with `PR_WAITOK`.
pub const PR_LIMITFAIL: i32 = 0x0004;

/// Where the parked references start in `Inner::refs` ([`PoolArc::forget`]): the low half
/// counts every reference (handles and parked ones), the high half the parked ones among
/// them.
const PARK_SHIFT: u32 = usize::BITS / 2;

/// One parked reference in `Inner::refs`.
const PARKED_ONE: usize = 1 << PARK_SHIFT;

/// The references in `Inner::refs` (its low half).
const REFS_MASK: usize = PARKED_ONE - 1;

/// The most references a [`PoolArc`] counts: past it a clone panics, before the count could
/// reach the parked half and free a live item. `MAXCPUS` below the half's top, so the clones
/// in flight on the other CPUs when one panics cannot carry into it; a C-shaped limit below
/// this one (`FDUP_MAX_COUNT`, `UINT_MAX - 2 * MAXCPUS`) refuses before it panics.
const MAX_REFS: usize = REFS_MASK - crate::cpu::MAXCPUS;

const _: () = assert!(
    usize::BITS == 64,
    "the count's two halves need a 64-bit word"
);

/// One pool of `bsd`'s `pool(9)`, as [`PoolBackend::create`] returns it.
///
/// The registration contract (`sys/hal/trust.rs`): `get` returns `None` or an item and the
/// size granted (`pr_size`, at least the size the pool was created with), aligned to its
/// `align`, that nothing else references until `put` takes it back. `hal` checks the
/// alignment and the size of every item.
pub trait RawPool: Sync {
    /// `pool_get(pp, flags)` (`PR_WAITOK`, `PR_NOWAIT`, `PR_LIMITFAIL`): the item and the
    /// size granted.
    fn get(&self, flags: i32) -> Option<(NonNull<u8>, usize)>;
    /// `pool_put(pp, item)`: the contract's other half. The item given back is one this pool
    /// granted and no view of it is alive; `hal` alone calls it (from the last `PoolArc`'s
    /// drop), since the pools never leave `hal` (`TypedPool` keeps them, and
    /// `bridges::pool_backend` is crate-private).
    fn put(&self, item: NonNull<u8>);
    /// `pr_nout`: the items out of the pool now.
    fn nout(&self) -> u64;
    /// `pr_size`: the size of one item, the one `vmstat -m` reports for the pool (at least
    /// the size it was created with).
    fn size(&self) -> usize;
    /// `pool_sethiwat(pp, n)`: the pool keeps at most `n` idle items (its pages beyond them
    /// go back to the system).
    fn set_hiwat(&self, n: u32);
    /// `pool_cache_init(pp)`: `pool_get` and `pool_put` go through a cache per CPU from now
    /// on.
    fn cache_init(&self);
}

/// `bsd`'s `pool(9)`, as it registers it (`sys/kern/subr_pool.rs`).
///
/// The registration contract: `create` returns `None` or a pool, alive for the rest of the
/// kernel's life, whose items hold `size` bytes aligned to `align`.
pub trait PoolBackend: Sync {
    /// `pool_init(pp, size, align, ipl, flags, wchan, NULL)` of a new pool that lives for
    /// ever (`size` at least 1, `align` a power of two).
    fn create(
        &self,
        size: usize,
        align: usize,
        ipl: i32,
        flags: i32,
        wchan: &'static str,
    ) -> Option<&'static dyn RawPool>;
}

/// A pool of `T`s (`struct pool` sized for the type): a `static`, initialised once at boot.
pub struct TypedPool<T: 'static> {
    made: OnceStatic<Made>,
    _t: PhantomData<fn() -> T>,
}

/// A [`TypedPool`] once made: its `pool(9)`, and whether records reach its values by
/// address ([`TypedPool::init_recorded`]).
#[derive(Clone, Copy)]
struct Made {
    raw: &'static dyn RawPool,
    recorded: bool,
}

impl<T: Send + Sync + 'static> TypedPool<T> {
    /// A pool that `init` has not made yet.
    pub const fn new() -> Self {
        Self {
            made: OnceStatic::new(),
            _t: PhantomData,
        }
    }

    /// `pool_init(9)`: the pool of `T`s at `ipl`, with `flags` (`PR_WAITOK`), named `wchan`
    /// (the wait message and the name `vmstat -m` shows). `ENOMEM` when the pool cannot be
    /// made (or before the bridge is registered); `EBUSY` when it was already.
    pub fn init(&'static self, ipl: i32, flags: i32, wchan: &'static str) -> Result<(), Errno> {
        self.init_with(ipl, flags, wchan, false)
    }

    /// [`TypedPool::init`] for a pool whose values records reach by address and give back
    /// as handles ([`PoolArc::from_value`]: the pmaps, named by their pv entries). Such a
    /// pool never gives a value mutably ([`PoolArc::get_mut`] answers `None`), since a record
    /// may rebuild a handle to it at any time; the choice is made with the pool, before any
    /// item exists.
    pub fn init_recorded(
        &'static self,
        ipl: i32,
        flags: i32,
        wchan: &'static str,
    ) -> Result<(), Errno> {
        self.init_with(ipl, flags, wchan, true)
    }

    fn init_with(
        &'static self,
        ipl: i32,
        flags: i32,
        wchan: &'static str,
        recorded: bool,
    ) -> Result<(), Errno> {
        if self.made.is_set() {
            return Err(Errno::EBUSY);
        }
        let backend = bridges::pool_backend().ok_or(Errno::ENOMEM)?;
        let raw = backend
            .create(
                size_of::<Inner<T>>(),
                align_of::<Inner<T>>(),
                ipl,
                flags,
                wchan,
            )
            .ok_or(Errno::ENOMEM)?;
        self.made
            .set(Made { raw, recorded })
            .map_err(|_| Errno::EBUSY)
    }

    /// The pool's `pool(9)`, once made.
    fn raw(&self) -> Option<&'static dyn RawPool> {
        self.made.get().map(|m| m.raw)
    }

    /// Whether records reach the values ([`TypedPool::init_recorded`]); `false` before the
    /// pool is made (it has no item then).
    fn recorded(&self) -> bool {
        self.made.get().is_some_and(|m| m.recorded)
    }

    /// `pool_get(9)` and the item's initialisation: a handle to a new item holding `init()`,
    /// counted once. `None` when `pool_get` fails (`PR_NOWAIT`, `PR_LIMITFAIL`, no memory)
    /// or the pool was never made; `init` is not called then.
    pub fn get_shared(&'static self, flags: i32, init: impl FnOnce() -> T) -> Option<PoolArc<T>> {
        let raw = self.raw()?;
        let (block, granted) = raw.get(flags)?;
        check_block(
            "RawPool",
            block,
            granted,
            size_of::<Inner<T>>(),
            align_of::<Inner<T>>(),
        );
        let item = block.cast::<Inner<T>>();
        // A panic in `init` leaks the item (it never reaches a handle); nothing reads it.
        let inner = Inner {
            refs: AtomicUsize::new(1),
            pool: self,
            value: init(),
        };
        // SAFETY: `item` is aligned for `Inner<T>` and at least `size_of::<Inner<T>>()` bytes
        // (`check_block` above), and it is a fresh item of this pool that nothing else
        // references: the registration contract of `RawPool` (the kernel's boot path
        // registered its own `pool(9)`, once, before any user; `sys/hal/trust.rs`), a
        // documented trust boundary. Writing a whole value initialises it, before any handle
        // exists.
        unsafe { item.as_ptr().write(inner) };
        // The item's whole provenance, for `PoolArc::from_value`'s step back from the value.
        let _ = item.as_ptr().expose_provenance();
        Some(PoolArc {
            inner: item,
            _owns: PhantomData,
        })
    }

    /// `pr_nout`: the items out of the pool now (0 before `init`).
    pub fn nout(&self) -> u64 {
        self.raw().map_or(0, |r| r.nout())
    }

    /// `pr_size` of the pool once made, as `vmstat -m` reports it: an item holds the count
    /// and the pool's address beside the `T`. `None` before `init`.
    pub fn item_size(&self) -> Option<usize> {
        self.raw().map(|r| r.size())
    }

    /// `pool_sethiwat(9)`: the pool keeps at most `n` idle items (`uvm_anon_init`'s
    /// `uvmexp.free / 16`). Nothing before `init`: a pool never made holds no item.
    pub fn set_hiwat(&self, n: u32) {
        if let Some(raw) = self.raw() {
            raw.set_hiwat(n);
        }
    }

    /// `pool_cache_init(9)`: a cache per CPU in front of the pool from now on
    /// (`uvm_anon_init_percpu`). Nothing before `init`.
    pub fn cache_init(&self) {
        if let Some(raw) = self.raw() {
            raw.cache_init();
        }
    }
}

impl<T: Homed> TypedPool<T> {
    /// `pool_get(9)` and the item's initialisation for a pool of [`TypedPool::init_recorded`]
    /// whose values record their place ([`PoolHome`]): as [`TypedPool::get_shared`], then the
    /// value's home is set to the value's address, so a `&T` a list lends can be counted again
    /// ([`PoolArc::from_ref`]). A pool made by [`TypedPool::init`] is `panic(9)`: a handle
    /// rebuilt beside a `get_mut` would alias it.
    pub fn get_homed(&'static self, flags: i32, init: impl FnOnce() -> T) -> Option<PoolArc<T>> {
        if self.made.get().is_some_and(|m| !m.recorded) {
            hal_panic(format_args!(
                "TypedPool::get_homed: a pool not made by init_recorded"
            ));
        }
        let arc = self.get_shared(flags, init)?;
        arc.home().record(&arc);
        Some(arc)
    }
}

impl<T: Send + Sync + 'static> Default for TypedPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Where a value of a recorded pool is: the address of the value it was recorded for, set
/// once by [`TypedPool::get_homed`] right after the value is written into its item, never
/// changed after. A field of the value ([`pool_home!`] names it), so it is born empty with
/// the value ([`PoolHome::new`] is its only constructor; it is neither `Clone` nor `Copy`) and
/// only a value placed by `get_homed` carries its own address in it.
pub struct PoolHome<T> {
    /// The value's address, 0 while it is in no item.
    at: AtomicUsize,
    _t: PhantomData<fn() -> T>,
}

impl<T> PoolHome<T> {
    /// An empty home (a value not placed in a pool item yet, or never: a `static`).
    pub const fn new() -> Self {
        Self {
            at: AtomicUsize::new(0),
            _t: PhantomData,
        }
    }

    /// Whether this home lies within `value`'s own bytes.
    fn inside(&self, value: &T) -> bool {
        let start = ptr::from_ref(value).addr();
        let me = ptr::from_ref(self).addr();
        me >= start && me + size_of::<Self>() <= start + size_of::<T>()
    }

    /// Records `value`'s address, once (`get_homed`); a home outside its value or recorded
    /// twice is `panic(9)`.
    fn record(&self, value: &T) {
        if !self.inside(value) {
            hal_panic(format_args!("PoolHome: a home outside its value"));
        }
        let at = ptr::from_ref(value).addr();
        if self
            .at
            .compare_exchange(0, at, Ordering::Release, Ordering::Relaxed)
            .is_err()
        {
            hal_panic(format_args!("PoolHome: a value placed twice"));
        }
    }

    /// Whether this home, inside `value`, recorded `value`'s own address.
    fn names(&self, value: &T) -> bool {
        self.inside(value) && self.at.load(Ordering::Acquire) == ptr::from_ref(value).addr()
    }
}

impl<T> Default for PoolHome<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The seal of [`Homed`]: implemented by [`pool_home!`]'s expansion alone.
#[doc(hidden)]
pub mod home_sealed {
    /// The supertrait only [`crate::pool_home!`] implements.
    pub trait HomeSealed {}
}

/// A value that records its place in a pool item in one of its own fields
/// ([`PoolArc::from_ref`]). Implemented by [`pool_home!`] alone (sealed), whose `home` is a
/// direct field of the type, never a home elsewhere.
pub trait Homed: home_sealed::HomeSealed + Send + Sync + Sized + 'static {
    /// The value's home field.
    fn home(&self) -> &PoolHome<Self>;
}

/// Declares the [`Homed`] of a type whose field `$field` is its [`PoolHome`]:
/// `pool_home!(Uidinfo, home);`, `pool_home!(impl[G: Glue] Process<G>, home);`.
#[macro_export]
macro_rules! pool_home {
    (impl[$($gen:tt)*] $ty:ty, $field:ident) => {
        impl<$($gen)*> $crate::pool::home_sealed::HomeSealed for $ty {}

        impl<$($gen)*> $crate::pool::Homed for $ty {
            fn home(&self) -> &$crate::pool::PoolHome<Self> {
                &self.$field
            }
        }
    };
    ($ty:ty, $field:ident) => {
        impl $crate::pool::home_sealed::HomeSealed for $ty {}

        impl $crate::pool::Homed for $ty {
            fn home(&self) -> &$crate::pool::PoolHome<Self> {
                &self.$field
            }
        }
    };
}

/// What an item of a [`TypedPool<T>`] holds: the count, the pool it goes back to, the value.
struct Inner<T: 'static> {
    /// The references in the low half ([`REFS_MASK`]: the live handles and the parked
    /// references), the parked ones among them in the high half ([`PARKED_ONE`]); the parked
    /// count never exceeds the references.
    refs: AtomicUsize,
    pool: &'static TypedPool<T>,
    value: T,
}

/// An owned, counted handle to a pool item holding a `T`: `Deref<Target = T>`, `Clone` adds a
/// reference, the last drop drops the `T` and puts the item back into its pool.
///
/// Invariant: `inner` names an initialised `Inner<T>` of `inner.pool`, whose `refs` counts (in
/// its low half) the live handles to it, this one included, and the references parked by
/// [`PoolArc::forget`] (a handle given up by [`PoolArc::leak`] stays counted for ever); the
/// item is put back only when that count reaches 0, so it outlives every handle and every
/// parked reference. While handles live only shared references to the item are made, except the
/// `&mut T` of [`PoolArc::get_mut`], made only when its handle is the one counted and is
/// borrowed mutably, in a pool that no record reaches.
pub struct PoolArc<T: Send + Sync + 'static> {
    inner: NonNull<Inner<T>>,
    _owns: PhantomData<Inner<T>>,
}

impl<T: Send + Sync + 'static> PoolArc<T> {
    fn inner(&self) -> &Inner<T> {
        // SAFETY: the item is initialised and outlives every handle, this one included (the
        // invariant; that no one else touches it until it is put back is the registration
        // contract of `RawPool`, `sys/hal/trust.rs`); only shared references to it are made
        // while handles live, but for `get_mut`'s, which exists only while the one handle is
        // borrowed mutably (so `self` here cannot be it, and no other handle exists to be
        // `self`) in a pool whose values no record rebuilds a handle from: this one aliases
        // no `&mut`.
        unsafe { self.inner.as_ref() }
    }

    /// The value, mutably, when this handle is the only one (`Arc::get_mut`): `None` while
    /// other handles exist, and always for a pool of [`TypedPool::init_recorded`] (whose
    /// values a record may give a handle back from at any time: the pmaps). The value of an
    /// amap copy being filled before anyone else sees it (`amap_copy`).
    pub fn get_mut(this: &mut Self) -> Option<&mut T>
    where
        T: Unpin,
    {
        let inner = this.inner();
        // Acquire: the drops of the other handles (Release) happen before this access. The
        // whole word: one reference and none parked.
        if inner.refs.load(Ordering::Acquire) != 1 || inner.pool.recorded() {
            return None;
        }
        // SAFETY: the count is 1, so `this` is the only handle (the invariant), and it is
        // borrowed mutably for as long as the returned reference lives, so no clone can be
        // made of it, no `Deref` through it can run and no other handle exists to make a
        // reference; the pool is not recorded, so `from_value` (which refuses such a pool)
        // cannot make a handle either. The item is initialised and outlives the handle; the
        // shared reference `inner` above ended before this one is made. The reference is to
        // the value alone, which nothing else reaches while it lives.
        Some(unsafe { &mut (*this.inner.as_ptr()).value })
    }

    /// Gives the handle up for a reference that lives for ever: the item stays counted by
    /// this handle, never reaches 0 and is never put back (`uvm_vnode.c`'s vnode objects,
    /// never freed and kept in their vnode across recycling). Other handles may still be
    /// cloned and dropped; none ever gets the value mutably again (the count stays above 1
    /// while one exists).
    pub fn leak(this: Self) -> &'static T {
        let this = ManuallyDrop::new(this);
        // SAFETY: the handle is never dropped (`ManuallyDrop`, and it is not returned), so the
        // count it holds is never released: the count stays at least 1 and the item is never
        // put back (the invariant), so the initialised value is valid for the rest of the
        // kernel's life. Only shared references are made to it from now on: `get_mut` needs a
        // handle whose count is 1, and with this one counted for ever every other handle sees
        // 2 or more.
        unsafe { &(*this.inner.as_ptr()).value }
    }

    /// The value, pinned: it stays where it is until its last handle drops it in place.
    pub fn as_pin(&self) -> Pin<&T> {
        // SAFETY: `Pin`'s contract: the value is not moved, nor its memory invalidated or
        // reused, before its drop runs. The value lives in its pool item from `get_shared`'s
        // write until the last handle's drop, which drops it in place (`drop_in_place`)
        // before the item goes back to its pool (the invariant: the item outlives every
        // handle). Nothing in between can move it: handles give `&T` alone, but for
        // `get_mut`, which needs `T: Unpin` (so a value for which pinning means something is
        // never given mutably), and `leak`, which gives a `&'static T`; no API reads the value
        // out of its item.
        unsafe { Pin::new_unchecked(&self.inner().value) }
    }

    /// One more handle to the item holding `value`, counted as a clone: `refcnt_take` of a
    /// pointer a list lends (`prfind`'s caller that keeps the process while it sleeps). `None`
    /// for a value no recorded pool placed (its [`PoolHome`] empty: proc0's `static`, a value
    /// on a stack). A value whose last handle is gone, its drop under way, is `panic(9)`.
    pub fn from_ref(value: &T) -> Option<Self>
    where
        T: Homed,
    {
        Self::homed(value, Rebuild::Count)
    }

    /// [`PoolArc::from_ref`] for a value whose last handle may be gone: `None` then (its
    /// drop under way, the item about to go back), never a handle to it. `fd_iterfile`'s skip
    /// of a file whose `f_count` is 0 while `filehead` still lends it under `fhdlk`.
    pub fn try_from_ref(value: &T) -> Option<Self>
    where
        T: Homed,
    {
        Self::homed(value, Rebuild::CountUnlessLast)
    }

    /// Gives the handle up and parks its reference: the count stays (so [`PoolArc::count`]
    /// still includes it and the item stays alive), and one [`PoolArc::adopt`] of the value
    /// takes it back as a handle. The reference a word in legacy memory holds (a descriptor
    /// in flight in a unix-domain message, the C's `FREF` in `unp_internalize`); a parked
    /// reference never adopted keeps its item for ever, as a leaked handle does.
    pub fn forget(this: Self)
    where
        T: Homed,
    {
        let this = ManuallyDrop::new(this);
        // The parked count never exceeds the references, which `MAX_REFS` bounds below the
        // half's top, so this cannot carry out of the word.
        this.inner().refs.fetch_add(PARKED_ONE, Ordering::Relaxed);
    }

    /// A handle for one reference [`PoolArc::forget`] parked on the item holding `value`
    /// (the C's message whose descriptor `unp_externalize` installs or `unp_discard` drops):
    /// the count does not change. `None` when no reference is parked there, or for a value no
    /// recorded pool placed. Parked references are counted, not named: adopting one more
    /// than were parked answers `None`, never a second handle for one reference.
    pub fn adopt(value: &T) -> Option<Self>
    where
        T: Homed,
    {
        Self::homed(value, Rebuild::Adopt)
    }

    /// The references alive now on the item holding `value` (a snapshot, as
    /// [`PoolArc::count`]), read without a handle: the C's `fp->f_count` read where taking
    /// a reference could make its holder the last one (`unp_gc` under `unp_gc_lock`, whose
    /// last close would take that lock again). 0 for a value no recorded pool placed, and
    /// for one whose last handle is gone.
    pub fn count_of(value: &T) -> usize
    where
        T: Homed,
    {
        Self::homed_view(value).map_or(0, |v| Self::count(&v))
    }

    /// The handle [`PoolArc::from_ref`], [`PoolArc::try_from_ref`] and [`PoolArc::adopt`]
    /// rebuild for `value`, as `how` says.
    fn homed(value: &T, how: Rebuild) -> Option<Self>
    where
        T: Homed,
    {
        Self::take(&Self::homed_view(value)?, how)
    }

    /// A view of the item holding `value` (no reference of its own, never dropped), for
    /// [`PoolArc::homed`] and [`PoolArc::count_of`] to read or raise its count while `value`
    /// is borrowed; `None` for a value no recorded pool placed. The view is never kept past
    /// the caller's use of `value`.
    fn homed_view(value: &T) -> Option<ManuallyDrop<Self>>
    where
        T: Homed,
    {
        if !value.home().names(value) {
            return None;
        }
        let at = NonNull::from(value);
        // SAFETY: `view`'s contract. `at` is the address of the value of an item of a
        // `TypedPool<T>` made by `init_recorded`: the home checked above is a direct field of
        // `value` (`Homed` is implemented by `pool_home!` alone, sealed, and names a field of
        // the type) holding `value`'s own address, and the only store of a non-zero address
        // into a home is `get_homed`'s, into the home of the value it has just written into an
        // item of a recorded pool, of that value's address (a home is born empty with its
        // value, `PoolHome::new` being its only constructor, neither `Clone` nor `Copy`; a
        // recorded pool never gives its values mutably and no API moves a value out of an
        // item, so a home's bytes never leave the value they were recorded in). So `value`
        // is that value, still in its item; `from_value` checks the pool's flag again. The
        // item is not put back during the call: `value` is borrowed for it, and an item goes
        // back only after its value's drop, which no `&T` survives but one a pinned list
        // lends, whose element's drop stops the kernel while it is linked (hal::pinlist's
        // I1) before its memory goes back. The view lives no longer than the caller's
        // borrow of `value` (`homed` and `count_of` use it at once); a count of 0 is refused
        // by `take` (a panic or `None`, as `how` says), never taken back to 1; an adoption
        // takes a parked reference, which only exists while the count is above 0.
        Some(unsafe { Self::view(at) })
    }

    /// Whether two handles name the same item.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.inner == b.inner
    }

    /// The references alive now (a snapshot): the handles and the parked references
    /// ([`PoolArc::forget`]), as the C's `f_count` counts the slots, the holders and the
    /// messages in flight.
    pub fn count(this: &Self) -> usize {
        this.inner().refs.load(Ordering::Relaxed) & REFS_MASK
    }

    /// `FRELE`'s decrement: gives the handle up, and when it held the last reference returns
    /// the item at count 0 ([`Last`]), which the caller reads (a fallible close, `fdrop`'s
    /// `fo_close`) and drops to free it; `None` when other references remain.
    pub fn try_last(this: Self) -> Option<Last<T>> {
        let this = ManuallyDrop::new(this);
        this.release()
    }

    /// Releases this handle's reference; the last one returns the item at count 0. The
    /// caller does not use the handle after this (a drop, or a handle given up).
    fn release(&self) -> Option<Last<T>> {
        let old = self.inner().refs.fetch_sub(1, Ordering::Release);
        if old & REFS_MASK != 1 {
            return None;
        }
        if old >> PARK_SHIFT != 0 {
            hal_panic(format_args!(
                "PoolArc: a parked reference outlived the count"
            ));
        }
        // The last reference: every other handle's writes happen before what follows.
        fence(Ordering::Acquire);
        Some(Last {
            inner: self.inner,
            _owns: PhantomData,
        })
    }

    /// The value's address, for what reports it as a number (`kinfo_file`'s `f_data`); never
    /// to be dereferenced.
    pub fn as_ptr(this: &Self) -> *const T {
        &raw const this.inner().value
    }

    /// One more handle to the item whose value `value` points at, counted as a clone: the C's
    /// `pmap_reference` of a pointer a record keeps (`pve->pv_pmap`), for who must hold the
    /// object after dropping the lock that keeps the record. The item is reached back from
    /// the value by `offset_of!`, with the provenance `get_shared` exposed for the whole item.
    ///
    /// # Safety
    ///
    /// As [`PoolArc::view`]'s.
    #[cfg_attr(
        not(target_os = "none"),
        allow(dead_code, reason = "the pmap backends' alone; the host has none")
    )]
    pub(crate) unsafe fn from_value(value: NonNull<T>) -> Self {
        // SAFETY: the caller keeps `view`'s contract (this function's own); the view is used
        // at once, inside it.
        let view = unsafe { Self::view(value) };
        match Self::take(&view, Rebuild::Count) {
            Some(h) => h,
            None => hal_panic(format_args!("PoolArc::from_value: no handle rebuilt")),
        }
    }

    /// A view of the item whose value `value` points at: a `PoolArc` that holds no reference
    /// of its own and is never dropped, through which [`PoolArc::take`] makes a handle or the
    /// count is read. The item is reached back from the value by `offset_of!`, with the
    /// provenance `get_shared` exposed for the whole item.
    ///
    /// # Safety
    ///
    /// `value` is the address of the value of an item of a `TypedPool<T>` made by
    /// [`TypedPool::init_recorded`] (one `get_shared` made; such a pool never gives its values
    /// mutably, so no `&mut` from `get_mut` can be alive beside the handle made here; the
    /// pool's flag is checked, a panic, which catches a wrong pool) that has not been put back
    /// into its pool and is not put back during this call:
    /// the caller holds the lock that keeps the record of the pointer, and the object's drop
    /// refuses to finish while a record is left (a pmap with a pv entry: `hal::pmap`'s last
    /// drop panics, and a drop that does not finish never puts the item back). The count may
    /// be anything: a count of 0 (the last handle dropped, its drop under way) is refused by
    /// [`PoolArc::take`] with a panic or `None` (as its `how` says), never taken back to 1, so
    /// no second last drop can follow, and only read by [`PoolArc::count_of`]; a parked
    /// reference exists only while the count is above 0. The view is used only while those
    /// hold (the caller's lock or borrow), never kept.
    unsafe fn view(value: NonNull<T>) -> ManuallyDrop<Self> {
        let addr = value.as_ptr().addr() - offset_of!(Inner<T>, value);
        let Some(inner) = NonNull::new(ptr::with_exposed_provenance_mut::<Inner<T>>(addr)) else {
            hal_panic(format_args!("PoolArc::from_value: a null item"));
        };
        // A view the caller's guarantee makes valid (the item is not put back), never dropped:
        // its count is raised only from a live count, never from 0.
        let borrowed = ManuallyDrop::new(PoolArc {
            inner,
            _owns: PhantomData,
        });
        if !borrowed.inner().pool.recorded() {
            hal_panic(format_args!(
                "PoolArc::from_value: an item of a pool no record reaches"
            ));
        }
        borrowed
    }

    /// A handle to the item `view` names, as `how` says: one more reference (panicking at a
    /// count of 0, or answering `None` there), or a parked reference adopted (`None` when
    /// none is parked).
    fn take(view: &ManuallyDrop<Self>, how: Rebuild) -> Option<Self> {
        let inner = view.inner;
        let refs = &view.inner().refs;
        let taken = match how {
            Rebuild::Count | Rebuild::CountUnlessLast => {
                refs.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |c| {
                    let n = c & REFS_MASK;
                    (n != 0 && n < MAX_REFS).then(|| c + 1)
                })
            }
            // Acquire: the parking handle's writes happen before the adopter's reads.
            Rebuild::Adopt => refs.fetch_update(Ordering::Acquire, Ordering::Relaxed, |c| {
                (c >> PARK_SHIFT != 0).then(|| c - PARKED_ONE)
            }),
        };
        match (taken, how) {
            (Ok(_), _) => Some(PoolArc {
                inner,
                _owns: PhantomData,
            }),
            (Err(_), Rebuild::Adopt) => None,
            (Err(c), _) if c & REFS_MASK == 0 => match how {
                Rebuild::CountUnlessLast => None,
                _ => hal_panic(format_args!(
                    "PoolArc::from_value: an item whose last handle is gone"
                )),
            },
            (Err(_), _) => hal_panic(format_args!("PoolArc: reference count overflow")),
        }
    }
}

impl<T: Send + Sync + 'static> Deref for PoolArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T: Send + Sync + 'static> Clone for PoolArc<T> {
    fn clone(&self) -> Self {
        let old = self.inner().refs.fetch_add(1, Ordering::Relaxed);
        if old & REFS_MASK >= MAX_REFS {
            hal_panic(format_args!("PoolArc: reference count overflow"));
        }
        PoolArc {
            inner: self.inner,
            _owns: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> Drop for PoolArc<T> {
    fn drop(&mut self) {
        // The last reference frees the item (`Last`'s drop); `self` is not used after.
        drop(self.release());
    }
}

// SAFETY: a handle gives `&T` on any CPU (`T: Sync`) and the last one, on whichever CPU drops
// it, drops the `T` there (`T: Send`); the count is atomic.
unsafe impl<T: Send + Sync + 'static> Send for PoolArc<T> {}

// SAFETY: `&PoolArc<T>` gives `&T` and clones (an atomic increment); with `T: Sync` sharing it
// across CPUs is sound, and a clone moved elsewhere is covered by `Send` above.
unsafe impl<T: Send + Sync + 'static> Sync for PoolArc<T> {}

/// How [`PoolArc::take`] makes its handle.
#[derive(Clone, Copy)]
enum Rebuild {
    /// One more reference; a count of 0 is `panic(9)`.
    Count,
    /// One more reference; `None` at a count of 0.
    CountUnlessLast,
    /// A parked reference taken back; `None` when none is parked.
    Adopt,
}

/// An item of a [`TypedPool<T>`] whose last reference is gone ([`PoolArc::try_last`]): the
/// value can still be read (`Deref`), and dropping this drops the `T` in place and puts the
/// item back. `fdrop`'s `struct file` between its last `FRELE` and `pool_put`: its close
/// runs through it and its error goes back to the caller.
///
/// Invariant: `inner` names an initialised `Inner<T>` of `inner.pool` whose count is 0 and
/// that nothing parks a reference on: no handle to it exists and none can be made
/// ([`PoolArc::take`] refuses a count of 0, an adoption needs a parked reference), so
/// this is its one owner until its drop; only shared references to the value are made from
/// it.
pub struct Last<T: Send + Sync + 'static> {
    inner: NonNull<Inner<T>>,
    _owns: PhantomData<Inner<T>>,
}

impl<T: Send + Sync + 'static> Last<T> {
    /// The whole item, shared: its count, its pool, its value.
    fn item(&self) -> &Inner<T> {
        // SAFETY: the item is initialised and is not put back before this `Last` is dropped
        // (the invariant: it is the item's one owner, and only its drop frees it), and that
        // drop cannot run while the returned reference, which borrows `self`, lives. Only
        // shared references to the item are made while it is at count 0: this one, and the
        // ones to the value a pinned list lends its walkers, which end with the unlink the
        // owner makes before dropping (hal::pinlist's I1 stops the kernel otherwise); none is
        // `&mut`.
        unsafe { self.inner.as_ref() }
    }
}

impl<T: Send + Sync + 'static> Deref for Last<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.item().value
    }
}

impl<T: Send + Sync + 'static> Drop for Last<T> {
    fn drop(&mut self) {
        let Some(raw) = self.item().pool.raw() else {
            hal_panic(format_args!("PoolArc: an item of a pool never made"));
        };
        // SAFETY: the count is 0 and this is the item's one owner (the invariant): no handle,
        // hence no reference made through one, exists and none can be made; the references
        // `item` made borrowed `self` and have ended (the one above, once `raw` was read),
        // and a pinned list's walkers no longer reach the value (its element is unlinked
        // before the value's drop, hal::pinlist's I1). The value is initialised and dropped
        // once, here, before its memory goes back to the pool it came from.
        unsafe { core::ptr::drop_in_place(&raw mut (*self.inner.as_ptr()).value) };
        raw.put(self.inner.cast());
    }
}

/// A pool of `T`s whose items have one owner each (`struct pool` sized for the type): a
/// `static`, initialised once at boot. Its items are exactly `T`s (no count, no back
/// pointer), handed out as [`PoolBox<T>`].
pub struct BoxPool<T: 'static> {
    raw: OnceStatic<&'static dyn RawPool>,
    _t: PhantomData<fn() -> T>,
}

impl<T: 'static> BoxPool<T> {
    /// A pool that `init` has not made yet.
    pub const fn new() -> Self {
        Self {
            raw: OnceStatic::new(),
            _t: PhantomData,
        }
    }

    /// `pool_init(9)`: the pool of `T`s at `ipl`, with `flags`, named `wchan` (the name
    /// `vmstat -m` shows). `ENOMEM` when the pool cannot be made (or before the bridge is
    /// registered); `EBUSY` when it was already.
    pub fn init(&'static self, ipl: i32, flags: i32, wchan: &'static str) -> Result<(), Errno> {
        if self.raw.is_set() {
            return Err(Errno::EBUSY);
        }
        let backend = bridges::pool_backend().ok_or(Errno::ENOMEM)?;
        let raw = backend
            .create(size_of::<T>().max(1), align_of::<T>(), ipl, flags, wchan)
            .ok_or(Errno::ENOMEM)?;
        self.raw.set(raw).map_err(|_| Errno::EBUSY)
    }

    /// `pool_get(9)` and the item's initialisation: the one handle to a new item holding
    /// `init()`. `None` when `pool_get` fails (`PR_NOWAIT`, `PR_LIMITFAIL`, no memory) or the
    /// pool was never made; `init` is not called then.
    pub fn get(&'static self, flags: i32, init: impl FnOnce() -> T) -> Option<PoolBox<T>> {
        let raw = *self.raw.get()?;
        let (block, granted) = raw.get(flags)?;
        check_block("RawPool", block, granted, size_of::<T>(), align_of::<T>());
        let item = block.cast::<T>();
        // A panic in `init` leaks the item (it never reaches a handle); nothing reads it.
        let value = init();
        // SAFETY: `item` is aligned for `T` and at least `size_of::<T>()` bytes (`check_block`
        // above), and it is a fresh item of this pool that nothing else references: the
        // registration contract of `RawPool` (the kernel's boot path registered its own
        // `pool(9)`, once, before any user; `sys/hal/trust.rs`), a documented trust boundary.
        // Writing a whole value initialises it, before the handle exists.
        unsafe { item.as_ptr().write(value) };
        Some(PoolBox {
            item,
            pool: self,
            _owns: PhantomData,
        })
    }

    /// `pr_nout`: the items out of the pool now (0 before `init`).
    pub fn nout(&self) -> u64 {
        self.raw.get().map_or(0, |r| r.nout())
    }

    /// `pr_size` of the pool once made, as `vmstat -m` reports it (an item is exactly a
    /// `T`; an arena's slot carries its free-list link too). `None` before `init`.
    pub fn item_size(&self) -> Option<usize> {
        self.raw.get().map(|r| r.size())
    }

    /// `pool_sethiwat(9)`: the pool keeps at most `n` idle items (`amap_init`'s 4096 chunks).
    /// Nothing before `init`: a pool never made holds no item.
    pub fn set_hiwat(&self, n: u32) {
        if let Some(raw) = self.raw.get() {
            raw.set_hiwat(n);
        }
    }

    /// `pool_cache_init(9)`: a cache per CPU in front of the pool from now on. Nothing
    /// before `init`.
    pub fn cache_init(&self) {
        if let Some(raw) = self.raw.get() {
            raw.cache_init();
        }
    }
}

impl<T: 'static> Default for BoxPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The one owner of a pool item holding a `T`: `Deref`/`DerefMut<Target = T>`; its drop drops
/// the `T` and puts the item back into the pool it came from. Not `Clone`.
///
/// Invariant: `item` names an initialised `T` in an item of `pool`, which nothing but this
/// handle references, except when the handle is the private field of a [`SharedBox`]: that
/// one never takes `&mut T`, and raw links may read the item beside its shared references
/// (`SharedBox`'s contract). It is put back exactly once, by this handle's drop.
pub struct PoolBox<T: 'static> {
    item: NonNull<T>,
    pool: &'static BoxPool<T>,
    _owns: PhantomData<T>,
}

impl<T: 'static> PoolBox<T> {
    /// The value's address, for what compares items by identity (a pv chain's entry, a table
    /// slot's descriptor); never to be dereferenced outside the handle.
    pub fn as_ptr(this: &Self) -> *const T {
        this.item.as_ptr().cast_const()
    }

    /// Whether two handles name the same item (only possible through a broken pool).
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.item == b.item
    }

    /// The value, pinned: it stays where it is until this handle drops it in place (a thread
    /// on its process's lists while its one owner keeps its box).
    pub fn as_pin(&self) -> Pin<&T> {
        // SAFETY: `Pin`'s contract: the value is not moved, nor its memory invalidated or
        // reused, before its drop runs. It lives in its pool item from `BoxPool::get`'s write
        // until the drop of the handle that owns the item, which drops it in place before the
        // item goes back (the invariant). Nothing in between moves it: `DerefMut` exists only
        // for `T: Unpin` (for which pinning promises nothing), no API reads the value out of
        // its item, and the raw forms (`into_raw`/`from_raw`, `SharedBox::into_raw`) hand the
        // same item, unmoved, to an owner that gives it back as a handle or drops it in place
        // the same way (`hal::owned`'s containers give a removed element back as its box).
        unsafe { Pin::new_unchecked(&**self) }
    }

    /// Gives the handle up for the item's pointer (with the item's whole provenance) and its
    /// pool: whoever keeps them owns the item from now on and gives it back as a handle by
    /// [`PoolBox::from_raw`]. The item is not freed here; dropped, the pair leaks it. The
    /// owning containers of `sys/hal/owned.rs` are the one user: an element linked in one
    /// is owned by the container, not by a handle.
    pub(crate) fn into_raw(this: Self) -> (NonNull<T>, &'static BoxPool<T>) {
        let this = ManuallyDrop::new(this);
        (this.item, this.pool)
    }

    /// The handle back from what [`PoolBox::into_raw`] returned.
    ///
    /// # Safety
    ///
    /// `raw` and `pool` were returned together by `into_raw`, and no handle was rebuilt from
    /// them since (each `into_raw` is answered by at most one `from_raw`, so the item has one
    /// owner again and is put back once); no reference to the value made while the pointer
    /// was kept is alive or used after this call (the handle may give `&mut T`).
    pub(crate) unsafe fn from_raw(raw: NonNull<T>, pool: &'static BoxPool<T>) -> Self {
        PoolBox {
            item: raw,
            pool,
            _owns: PhantomData,
        }
    }
}

impl<T: 'static> Deref for PoolBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the item is initialised and alive until this handle's drop (the invariant;
        // that no one else frees or writes it until it is put back is the registration
        // contract of `RawPool`, `sys/hal/trust.rs`); the shared reference borrows the handle,
        // so no `&mut` from `deref_mut` can be alive beside it, and a `SharedBox`'s links only
        // read.
        unsafe { self.item.as_ref() }
    }
}

impl<T: Unpin + 'static> DerefMut for PoolBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as for `deref`, and the handle is borrowed mutably, so no other reference
        // through it is alive; this `PoolBox` is not a `SharedBox`'s (whose field is private
        // and which never calls this), so no link to the item exists either: this is the
        // only reference to the value while it lives.
        unsafe { self.item.as_mut() }
    }
}

impl<T: 'static> Drop for PoolBox<T> {
    fn drop(&mut self) {
        let Some(raw) = self.pool.raw.get() else {
            hal_panic(format_args!("PoolBox: an item of a pool never made"));
        };
        // SAFETY: this handle is the item's only owner (the invariant) and is being dropped, so
        // no reference through it is alive and none can be made (a `SharedBox`'s links are
        // unlinked before its owner drops it, under the lock their readers hold: its
        // contract); the value is initialised and dropped once, here, before its memory goes
        // back to the pool it came from.
        unsafe { core::ptr::drop_in_place(self.item.as_ptr()) };
        raw.put(self.item.cast());
    }
}

// SAFETY: the handle owns its `T` as a `Box` does: moving it to another CPU moves the `T`
// (`T: Send`), which is dropped wherever the handle is; the pool it goes back to is `Sync`.
unsafe impl<T: Send + 'static> Send for PoolBox<T> {}

// SAFETY: `&PoolBox<T>` gives only `&T` (`DerefMut` needs `&mut PoolBox<T>`), so sharing it
// across CPUs is sharing `&T`, sound with `T: Sync`.
unsafe impl<T: Sync + 'static> Sync for PoolBox<T> {}

/// An owned pool item that others may also reach by raw links: one owner (this handle, in
/// a table slot) frees it, and `&T` from it may coexist with the links' readers (a page's pv
/// list, arm64's `pte_desc`). No `DerefMut`: a field changed after the item is linked is an
/// atomic or a `Cell` under the lock that guards it.
///
/// The contract its users keep (the links' derefs are theirs, each with its own `SAFETY:`):
/// a link is made from [`SharedBox::link`] and read only under the lock that also guards the
/// link's removal, and the owner drops the box only after every link to it was removed under
/// that lock; links never take `&mut T`.
pub struct SharedBox<T: 'static>(PoolBox<T>);

impl<T: 'static> SharedBox<T> {
    /// A pointer to the value, with the item's whole provenance, for a link; never written
    /// through.
    pub fn link(this: &Self) -> NonNull<T> {
        this.0.item
    }

    /// Gives the handle up for the item's pointer, with the item's whole provenance: whoever
    /// keeps the pointer (a word of a table page) owns the item from now on, and frees it by
    /// [`SharedBox::from_raw`]. The item is not freed here; dropped, the pointer leaks it.
    pub fn into_raw(this: Self) -> NonNull<T> {
        let this = ManuallyDrop::new(this);
        this.0.item
    }

    /// The handle back from what [`SharedBox::into_raw`] returned, to free the item or keep
    /// it as a handle again.
    ///
    /// # Safety
    ///
    /// `raw` was returned by `into_raw` for an item of `pool` (`get_linked` on that pool), and
    /// no handle was rebuilt from it since: each `into_raw` is answered by at most one
    /// `from_raw`, so the item has one owner again and is put back once. Every link to the
    /// item is still read only under the lock that guards its removal (the type's contract).
    #[cfg_attr(
        not(all(target_os = "none", target_arch = "aarch64")),
        allow(dead_code, reason = "arm64's level-3 table slots alone keep raw items")
    )]
    pub(crate) unsafe fn from_raw(pool: &'static BoxPool<T>, raw: NonNull<T>) -> Self {
        SharedBox(PoolBox {
            item: raw,
            pool,
            _owns: PhantomData,
        })
    }
}

impl<T: 'static> Deref for SharedBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: 'static> BoxPool<T> {
    /// `pool_get(9)` and the item's initialisation, as [`BoxPool::get`], for an item that will
    /// also be linked: the owner's [`SharedBox`].
    pub fn get_linked(&'static self, flags: i32, init: impl FnOnce() -> T) -> Option<SharedBox<T>> {
        self.get(flags, init).map(SharedBox)
    }
}

impl<T: Send + 'static> BoxPool<T> {
    /// Makes this pool hand out the slots of `arena` instead of a `pool(9)`'s items: the
    /// static entries' pool (`uvm_mapent_alloc`'s `VM_MAP_INTRSAFE || cold` case). `EBUSY`
    /// when the pool was made already. Nothing shows in `vmstat -m`, as for the C's static
    /// entries.
    pub fn init_arena<const N: usize>(
        &'static self,
        arena: &'static StaticArena<T, N>,
    ) -> Result<(), Errno> {
        self.raw.set(&arena.core).map_err(|_| Errno::EBUSY)
    }
}

/// A pool of `T`s that is not a `pool(9)`: `N` slots in the arena itself (a `static`) and
/// those of the pages [`StaticArena::extend`] gives it, none ever freed; a [`BoxPool`] hands
/// them out ([`BoxPool::init_arena`]). The C's `kernel_map_entry[MAX_KMAPENT]`, its
/// `uvm.kentry_free` list under `uvm_kmapent_mtx`, and the never-freed pages
/// `uvm_mapent_alloc` carves when the list is empty.
pub struct StaticArena<T: 'static, const N: usize> {
    core: ArenaCore<T, N>,
}

impl<T: Send + 'static, const N: usize> StaticArena<T, N> {
    /// An arena of `N` free slots.
    pub const fn new() -> Self {
        Self {
            core: ArenaCore {
                slots: [const { Slot::new() }; N],
                free: Mutexed::new(
                    IPL_VM,
                    Free {
                        head: None,
                        fresh: 0,
                    },
                ),
                nout: AtomicU64::new(0),
            },
        }
    }

    /// Gives `page` up for good and adds its slots to the free list (`uvm_mapent_alloc`'s
    /// `km_alloc(PAGE_SIZE, &kv_page, &kp_dirty, &kd_nowait)` carved into entries): how many.
    /// The page is never freed. It must be wired (`kp_dirty`'s): the slots are written under
    /// the arena's mutex at `IPL_VM` and from `INTRSAFE` paths, where a fault on a pageable
    /// page cannot be taken; a pageable one panics.
    pub fn extend(&self, page: KmBuf) -> usize {
        if page.kind() != PageKind::Wired {
            hal_panic(format_args!(
                "StaticArena::extend: a pageable page (the slots are used at IPL_VM)"
            ));
        }
        let slots = page.leak_slice(|_| Slot::new());
        let mut f = self.core.free.lock();
        for s in slots {
            s.next.set(f.head);
            f.head = Some(s);
        }
        slots.len()
    }

    /// The slots out now (handed out and not put back).
    pub fn nout(&self) -> u64 {
        self.core.nout.load(Ordering::Relaxed)
    }

    /// Whether a slot is free now (a snapshot): when not, [`BoxPool::get`] on the arena's
    /// pool answers `None` until [`StaticArena::extend`] or a put.
    pub fn has_free(&self) -> bool {
        let f = self.core.free.lock();
        f.head.is_some() || f.fresh < N
    }
}

impl<T: Send + 'static, const N: usize> Default for StaticArena<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// What a [`StaticArena`] holds; the `RawPool` its `BoxPool` reaches. Private, so nothing
/// outside this module can call its `put`.
struct ArenaCore<T: 'static, const N: usize> {
    /// The arena's own slots, handed out first, in order.
    slots: [Slot<T>; N],
    /// The free list and how many of `slots` were ever handed out.
    free: Mutexed<Free<T>>,
    /// `pr_nout`'s count.
    nout: AtomicU64,
}

/// A [`StaticArena`]'s free list (`uvm.kentry_free`).
struct Free<T: 'static> {
    /// The first free slot put back or carved from a page.
    head: Option<&'static Slot<T>>,
    /// The first of the arena's own slots never handed out.
    fresh: usize,
}

/// Room for one `T`, and the link of the free list while it is free. `#[repr(C)]` with the
/// value first, so a slot's address is its item's.
#[repr(C)]
struct Slot<T: 'static> {
    value: UnsafeCell<MaybeUninit<T>>,
    next: Link<Slot<T>>,
}

impl<T: 'static> Slot<T> {
    const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::zeroed()),
            next: Link::new(),
        }
    }
}

// SAFETY: a slot's `value` is reached only through the one item pointer `ArenaCore::get`
// handed out for it, by the `PoolBox` that holds it, until that box's drop puts it back
// (the free list hands a slot out once, under its mutex, and takes it back once); `&Slot`
// shared between CPUs gives nothing else access to the value (no method reads it) and its
// `next` is an atomic `Link`. So sharing a slot never shares its `T`; the `T` is made on one
// CPU and dropped on whichever CPU drops the box, which `T: Send` allows.
unsafe impl<T: Send + 'static> Sync for Slot<T> {}

impl<T: Send + 'static, const N: usize> RawPool for ArenaCore<T, N> {
    fn get(&self, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        let mut f = self.free.lock();
        let slot = if let Some(s) = f.head {
            f.head = s.next.get();
            s.next.set(None);
            NonNull::from(s)
        } else {
            // With no slot left, `None` and no sleep for one: the caller extends the arena
            // (`kd_nowait`).
            let s = self.slots.get(f.fresh)?;
            f.fresh += 1;
            NonNull::from(s)
        };
        drop(f);
        self.nout.fetch_add(1, Ordering::Relaxed);
        Some((slot.cast::<u8>(), size_of::<Slot<T>>()))
    }

    fn put(&self, item: NonNull<u8>) {
        // SAFETY: `RawPool::put`'s contract: `item` is an item this pool granted and has not
        // taken back, and nothing references it any more; only `PoolBox`'s drop calls it,
        // with the pointer `get` gave (this type is private, so nothing outside this module
        // can call it, and `BoxPool::init_arena` binds the arena to its own `BoxPool`). Such a
        // pointer is a slot's address, with the slot's provenance (`NonNull::from` of the
        // slot), of a slot in `slots` (this arena's, which lives for ever: `init_arena` takes
        // it `&'static`) or in a page `extend` gave up for good: so it names a live, aligned
        // `Slot<T>` that lives for ever. The value was dropped by the box and is a
        // `MaybeUninit`, so the shared reference makes no claim on it; the slot is `Sync`,
        // and shared references are all that is ever made of it.
        let slot: &'static Slot<T> = unsafe { item.cast::<Slot<T>>().as_ref() };
        let mut f = self.free.lock();
        slot.next.set(f.head);
        f.head = Some(slot);
        drop(f);
        self.nout.fetch_sub(1, Ordering::Relaxed);
    }

    fn nout(&self) -> u64 {
        self.nout.load(Ordering::Relaxed)
    }

    fn size(&self) -> usize {
        size_of::<Slot<T>>()
    }

    /// An arena keeps every slot: nothing to give back.
    fn set_hiwat(&self, _n: u32) {}

    /// An arena has no per-CPU cache: its free list is one mutex, as `uvm_kmapent_mtx`.
    fn cache_init(&self) {}
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU32;
    use std::thread;
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    struct Pair {
        a: u64,
        drops: Arc<AtomicU32>,
    }

    impl Drop for Pair {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A pool of the test's own (the tests run in parallel and count items out).
    fn made(pool: &'static TypedPool<Pair>) -> &'static TypedPool<Pair> {
        testutil::install();
        assert_eq!(pool.init(0, PR_WAITOK, "testpl"), Ok(()));
        pool
    }

    #[test]
    fn an_item_is_freed_once_by_the_last_handle() {
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
        let drops = Arc::new(AtomicU32::new(0));
        let before = pool.nout();
        let one = pool
            .get_shared(PR_WAITOK, || Pair {
                a: 7,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        let two = one.clone();
        assert!(PoolArc::ptr_eq(&one, &two));
        assert_eq!(PoolArc::count(&one), 2);
        assert_eq!(two.a, 7);
        assert_eq!(pool.nout(), before + 1);
        drop(one);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(pool.nout(), before + 1);
        drop(two);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(pool.nout(), before);
    }

    #[test]
    fn handles_cross_cpus_and_the_last_one_frees() {
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
        let drops = Arc::new(AtomicU32::new(0));
        let h = pool
            .get_shared(PR_WAITOK, || Pair {
                a: 1,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let h = h.clone();
                thread::spawn(move || h.a)
            })
            .collect();
        drop(h);
        for t in threads {
            assert_eq!(t.join().unwrap(), 1);
        }
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(pool.nout(), 0);
    }

    #[test]
    fn a_second_init_is_refused() {
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
        assert_eq!(pool.init(0, PR_WAITOK, "testpl"), Err(Errno::EBUSY));
    }

    #[test]
    fn a_box_drops_its_value_and_puts_the_item_back() {
        static P: BoxPool<Pair> = BoxPool::new();
        testutil::install();
        assert_eq!(P.init(0, 0, "testbx"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let mut b = P
            .get(PR_NOWAIT, || Pair {
                a: 3,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        assert_eq!(P.nout(), 1);
        b.a += 1;
        assert_eq!(b.a, 4);
        drop(b);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(P.nout(), 0);
    }

    /// The pv chain's shape: each entry owns the next, so dropping the head frees the chain.
    struct Link {
        n: u32,
        next: Option<PoolBox<Link>>,
    }

    #[test]
    fn an_owned_chain_frees_every_link_with_its_head() {
        static P: BoxPool<Link> = BoxPool::new();
        testutil::install();
        assert_eq!(P.init(0, 0, "testch"), Ok(()));
        let mut head: Option<PoolBox<Link>> = None;
        for n in 0..5 {
            let next = head.take();
            head = Some(P.get(PR_NOWAIT, || Link { n, next }).unwrap());
        }
        assert_eq!(P.nout(), 5);
        // Unlink the entry holding 2 through a cursor, as pmap_remove_pv does.
        let mut cur = &mut head;
        while cur.as_ref().is_some_and(|e| e.n != 2) {
            cur = &mut cur.as_mut().unwrap().next;
        }
        let mut gone = cur.take().unwrap();
        *cur = gone.next.take();
        drop(gone);

        assert_eq!(P.nout(), 4);
        let mut seen = Vec::new();
        let mut at = head.as_deref();
        while let Some(e) = at {
            seen.push(e.n);
            at = e.next.as_deref();
        }
        assert_eq!(seen, [4, 3, 1, 0]);
        drop(head);
        assert_eq!(P.nout(), 0);
    }

    #[test]
    fn a_box_item_is_exactly_its_value() {
        assert_eq!(size_of::<Option<PoolBox<Pair>>>(), 2 * size_of::<usize>());
        static P: BoxPool<u64> = BoxPool::new();
        testutil::install();
        assert_eq!(P.init(0, 0, "testsz"), Ok(()));
        let b = P.get(PR_NOWAIT, || 7).unwrap();
        assert_eq!(*b, 7);
        assert!(PoolBox::as_ptr(&b).addr().is_multiple_of(align_of::<u64>()));
    }

    #[test]
    fn the_item_size_is_the_pool_s_own() {
        // `pr_size` as the pool reports it (vmstat -m): a box's item is its value, a counted
        // item adds the count and the pool's address; nothing before `init`.
        static B: BoxPool<[u64; 3]> = BoxPool::new();
        static T: TypedPool<[u64; 3]> = TypedPool::new();
        testutil::install();
        assert_eq!(B.item_size(), None);
        assert_eq!(T.item_size(), None);
        assert_eq!(B.init(0, 0, "testbsz"), Ok(()));
        assert_eq!(T.init(0, 0, "testtsz"), Ok(()));
        assert_eq!(B.item_size(), Some(24));
        assert_eq!(T.item_size(), Some(24 + 2 * size_of::<usize>()));
    }

    #[test]
    fn a_handle_is_rebuilt_from_its_value() {
        static P: TypedPool<Pair> = TypedPool::new();
        testutil::install();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "testrec"), Ok(()));
        let pool = &P;
        let drops = Arc::new(AtomicU32::new(0));
        let one = pool
            .get_shared(PR_WAITOK, || Pair {
                a: 9,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        let value = NonNull::from(&*one);
        // SAFETY: `value` is the value of `one`'s item, alive with a count of 1 for the call.
        let two = unsafe { PoolArc::from_value(value) };
        assert!(PoolArc::ptr_eq(&one, &two));
        assert_eq!(PoolArc::count(&one), 2);
        drop(one);
        assert_eq!(two.a, 9);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(two);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(pool.nout(), 0);
    }

    #[test]
    fn a_shared_box_reads_through_its_link_and_frees_once() {
        static P: BoxPool<u64> = BoxPool::new();
        testutil::install();
        assert_eq!(P.init(0, 0, "testsb"), Ok(()));
        let b = P.get_linked(PR_NOWAIT, || 42).unwrap();
        let link = SharedBox::link(&b);
        assert_eq!(*b, 42);
        assert_eq!(link.as_ptr().cast_const(), &raw const *b);
        assert_eq!(P.nout(), 1);
        drop(b);
        assert_eq!(P.nout(), 0);
    }

    #[test]
    fn a_shared_box_given_up_for_its_pointer_is_freed_once_back() {
        static P: BoxPool<u64> = BoxPool::new();
        testutil::install();
        assert_eq!(P.init(0, 0, "testraw"), Ok(()));
        let b = P.get_linked(PR_NOWAIT, || 7).unwrap();
        let link = SharedBox::link(&b);
        let raw = SharedBox::into_raw(b);
        assert_eq!(raw, link);
        // The item stays out while only the pointer owns it.
        assert_eq!(P.nout(), 1);
        // SAFETY: `raw` is what `into_raw` returned for an item of `P`, rebuilt this once.
        let back = unsafe { SharedBox::from_raw(&P, raw) };
        assert_eq!(*back, 7);
        drop(back);
        assert_eq!(P.nout(), 0);
    }

    #[test]
    #[should_panic(expected = "a pool no record reaches")]
    fn from_value_refuses_a_pool_records_do_not_reach() {
        static P: TypedPool<u64> = TypedPool::new();
        testutil::install();
        assert_eq!(P.init(0, PR_WAITOK, "testnorec"), Ok(()));
        let one = P.get_shared(PR_WAITOK, || 3).unwrap();
        // SAFETY: `one`'s value, alive for the call; the pool is not recorded, which the call
        // refuses before it makes a handle.
        let _two = unsafe { PoolArc::from_value(NonNull::from(&*one)) };
    }

    #[test]
    fn get_mut_answers_only_for_the_one_handle() {
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
        let drops = Arc::new(AtomicU32::new(0));
        let mut one = pool
            .get_shared(PR_WAITOK, || Pair {
                a: 1,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        PoolArc::get_mut(&mut one).unwrap().a = 2;
        let two = one.clone();
        assert!(PoolArc::get_mut(&mut one).is_none());
        assert_eq!(two.a, 2);
        // The other handle dropped on another CPU: its drop happens before the access.
        thread::spawn(move || drop(two)).join().unwrap();
        PoolArc::get_mut(&mut one).unwrap().a += 1;
        assert_eq!(one.a, 3);
        drop(one);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_recorded_pool_never_gives_a_value_mutably() {
        static P: TypedPool<u64> = TypedPool::new();
        testutil::install();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "testrecmut"), Ok(()));
        let mut one = P.get_shared(PR_WAITOK, || 7).unwrap();
        assert_eq!(PoolArc::count(&one), 1);
        assert!(PoolArc::get_mut(&mut one).is_none());
    }

    #[test]
    fn a_leaked_handle_keeps_its_item_for_ever() {
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
        let drops = Arc::new(AtomicU32::new(0));
        let one = pool
            .get_shared(PR_WAITOK, || Pair {
                a: 5,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        let mut other = one.clone();
        let forever: &'static Pair = PoolArc::leak(one);
        assert_eq!(forever.a, 5);
        // The leaked handle stays counted: the other is never the only one.
        assert_eq!(PoolArc::count(&other), 2);
        assert!(PoolArc::get_mut(&mut other).is_none());
        drop(other);
        assert_eq!(pool.nout(), 1);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(forever.a, 5);
    }

    #[test]
    fn the_high_water_mark_and_the_cache_reach_the_pool() {
        static P: TypedPool<u64> = TypedPool::new();
        static B: BoxPool<u64> = BoxPool::new();
        static NEVER: TypedPool<u64> = TypedPool::new();
        testutil::install();
        // A pool never made has nothing to bound.
        NEVER.set_hiwat(1);
        NEVER.cache_init();
        assert_eq!(P.init(0, PR_WAITOK, "testhiwat"), Ok(()));
        assert_eq!(testutil::pool_knobs("testhiwat"), Some((0, false)));
        P.set_hiwat(16);
        P.cache_init();
        assert_eq!(testutil::pool_knobs("testhiwat"), Some((16, true)));
        assert_eq!(B.init(0, 0, "testbxhiwat"), Ok(()));
        B.set_hiwat(4096);
        assert_eq!(testutil::pool_knobs("testbxhiwat"), Some((4096, false)));
    }

    #[test]
    fn a_pool_never_made_gives_nothing() {
        static NEVER: TypedPool<u32> = TypedPool::new();
        let mut called = false;
        assert!(
            NEVER
                .get_shared(PR_NOWAIT, || {
                    called = true;
                    0
                })
                .is_none()
        );
        assert!(!called);
        assert_eq!(NEVER.nout(), 0);
        static NEVER_BOX: BoxPool<u32> = BoxPool::new();
        assert!(NEVER_BOX.get(PR_NOWAIT, || 0).is_none());
        assert_eq!(NEVER_BOX.nout(), 0);
    }

    /// A static entry's shape: a key and a count of the drops.
    struct Entry {
        key: u64,
        drops: Arc<AtomicU32>,
    }

    impl Drop for Entry {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn an_arena_hands_out_its_slots_then_the_pages_it_is_given() {
        testutil::install();
        static ARENA: StaticArena<Entry, 4> = StaticArena::new();
        static P: BoxPool<Entry> = BoxPool::new();
        assert_eq!(P.init_arena(&ARENA), Ok(()));
        assert_eq!(P.init_arena(&ARENA), Err(Errno::EBUSY));
        let drops = Arc::new(AtomicU32::new(0));
        let get = |key| {
            P.get(PR_NOWAIT, || Entry {
                key,
                drops: Arc::clone(&drops),
            })
        };
        let mut out: Vec<PoolBox<Entry>> = (0..4).map(|k| get(k).unwrap()).collect();
        assert_eq!((ARENA.nout(), P.nout()), (4, 4));
        assert!(!ARENA.has_free());
        // Out of slots: `None`, and `init` is not called.
        assert!(get(99).is_none());
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        // A slot put back is handed out again.
        let addr = PoolBox::as_ptr(&out[1]);
        drop(out.remove(1));
        assert_eq!((drops.load(Ordering::Relaxed), ARENA.nout()), (1, 3));
        let again = get(10).unwrap();
        assert_eq!(PoolBox::as_ptr(&again), addr);
        assert_eq!(again.key, 10);
        out.push(again);
        // A page gives more slots, never freed.
        let page = KmBuf::alloc_wired(testutil::page_memory(), 4096, false).unwrap();
        let added = ARENA.extend(page);
        assert_eq!(added, 4096 / size_of::<Slot<Entry>>());
        for k in 0..added {
            out.push(get(100 + k as u64).unwrap());
        }
        assert!(get(1000).is_none());
        let keys: Vec<u64> = out.iter().map(|e| e.key).collect();
        assert_eq!(keys.len(), 4 + added);
        let n = out.len();
        drop(out);
        assert_eq!(drops.load(Ordering::Relaxed), 1 + n as u32);
        assert_eq!(ARENA.nout(), 0);
        assert!(ARENA.has_free());
    }

    #[test]
    fn an_owned_box_round_trips_through_its_raw_form() {
        static P: BoxPool<Pair> = BoxPool::new();
        testutil::install();
        assert_eq!(P.init(0, PR_WAITOK, "rawpl"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let b = P
            .get(PR_WAITOK, || Pair {
                a: 3,
                drops: Arc::clone(&drops),
            })
            .unwrap();
        let (raw, pool) = PoolBox::into_raw(b);
        assert_eq!(P.nout(), 1);
        // SAFETY: `raw` and `pool` came from `into_raw` just above, once.
        let mut b = unsafe { PoolBox::from_raw(raw, pool) };
        b.a += 1;
        assert_eq!(b.a, 4);
        drop(b);
        assert_eq!((drops.load(Ordering::Relaxed), P.nout()), (1, 0));
    }

    #[test]
    #[should_panic(expected = "StaticArena::extend: a pageable page")]
    fn an_arena_refuses_a_pageable_page() {
        testutil::install();
        static ARENA: StaticArena<Entry, 1> = StaticArena::new();
        let page = KmBuf::alloc_pageable(testutil::page_memory(), 4096).unwrap();
        let _ = ARENA.extend(page);
    }

    /// A value that must stay where it is (`PhantomPinned`): `get_mut` does not exist for it.
    struct Fixed {
        v: u32,
        _pin: core::marker::PhantomPinned,
    }

    #[test]
    fn a_pool_value_is_pinned_where_its_handles_see_it() {
        testutil::install();
        static P: TypedPool<Fixed> = TypedPool::new();
        assert_eq!(P.init(0, PR_WAITOK, "pinpl"), Ok(()));
        let h = P
            .get_shared(PR_WAITOK, || Fixed {
                v: 3,
                _pin: core::marker::PhantomPinned,
            })
            .unwrap();
        let other = h.clone();
        let pinned = other.as_pin();
        assert!(core::ptr::eq(pinned.get_ref(), &*h));
        assert_eq!(pinned.v, 3);
    }

    /// A value that records its place: `PoolArc::from_ref` counts it again.
    struct Homey {
        v: u32,
        home: PoolHome<Homey>,
        _pin: core::marker::PhantomPinned,
    }

    crate::pool_home!(Homey, home);

    #[test]
    fn a_lent_reference_is_counted_again_only_for_a_placed_value() {
        testutil::install();
        static P: TypedPool<Homey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "homepl"), Ok(()));
        let h = P
            .get_homed(PR_WAITOK, || Homey {
                v: 5,
                home: PoolHome::new(),
                _pin: core::marker::PhantomPinned,
            })
            .unwrap();
        let lent: &Homey = &h;
        let two = PoolArc::from_ref(lent).unwrap();
        assert!(PoolArc::ptr_eq(&h, &two));
        assert_eq!(PoolArc::count(&h), 2);
        drop(h);
        assert_eq!(two.v, 5);
        drop(two);
        assert_eq!(P.nout(), 0);
        // A value made outside a pool has an empty home.
        let local = Homey {
            v: 1,
            home: PoolHome::new(),
            _pin: core::marker::PhantomPinned,
        };
        assert!(PoolArc::from_ref(&local).is_none());
        // A value written by get_shared (not get_homed) has one too.
        static Q: TypedPool<Homey> = TypedPool::new();
        assert_eq!(Q.init_recorded(0, PR_WAITOK, "homeq"), Ok(()));
        let plain = Q
            .get_shared(PR_WAITOK, || Homey {
                v: 2,
                home: PoolHome::new(),
                _pin: core::marker::PhantomPinned,
            })
            .unwrap();
        assert!(PoolArc::from_ref(&*plain).is_none());
    }

    #[test]
    #[should_panic(expected = "a pool not made by init_recorded")]
    fn a_homed_value_needs_a_recorded_pool() {
        testutil::install();
        static P: TypedPool<Homey> = TypedPool::new();
        assert_eq!(P.init(0, PR_WAITOK, "homeplain"), Ok(()));
        let _ = P.get_homed(PR_WAITOK, || Homey {
            v: 0,
            home: PoolHome::new(),
            _pin: core::marker::PhantomPinned,
        });
    }

    #[test]
    fn a_box_value_is_pinned_where_its_owner_sees_it() {
        testutil::install();
        static P: BoxPool<Fixed> = BoxPool::new();
        assert_eq!(P.init(0, PR_WAITOK, "pinbox"), Ok(()));
        let b = P
            .get(PR_WAITOK, || Fixed {
                v: 4,
                _pin: core::marker::PhantomPinned,
            })
            .unwrap();
        let pinned = b.as_pin();
        assert!(core::ptr::eq(pinned.get_ref(), &*b));
        assert_eq!(pinned.v, 4);
    }

    /// A homed value that counts its drops (`struct file` in `kernel::file`'s shape).
    struct Filey {
        v: u32,
        home: PoolHome<Filey>,
        drops: Arc<AtomicU32>,
    }

    impl Drop for Filey {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    crate::pool_home!(Filey, home);

    fn filey(pool: &'static TypedPool<Filey>, v: u32, drops: &Arc<AtomicU32>) -> PoolArc<Filey> {
        pool.get_homed(PR_WAITOK, || Filey {
            v,
            home: PoolHome::new(),
            drops: drops.clone(),
        })
        .unwrap()
    }

    #[test]
    fn the_last_release_hands_the_item_over_and_its_drop_frees_it() {
        testutil::install();
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
        let drops = Arc::new(AtomicU32::new(0));
        let one = pool
            .get_shared(PR_WAITOK, || Pair {
                a: 9,
                drops: drops.clone(),
            })
            .unwrap();
        let two = one.clone();
        // Not the last: the reference goes, nothing else.
        assert!(PoolArc::try_last(one).is_none());
        assert_eq!(PoolArc::count(&two), 1);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        // The last: the value is still there to read (a close), then the drop frees it.
        let last = PoolArc::try_last(two).unwrap();
        assert_eq!(last.a, 9);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(pool.nout(), 1);
        drop(last);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(pool.nout(), 0);
    }

    #[test]
    fn a_value_at_count_zero_is_skipped_never_counted_again() {
        testutil::install();
        static P: TypedPool<Filey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "lastpl"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let h = filey(&P, 1, &drops);
        // A live value: one more reference.
        let again = PoolArc::try_from_ref(&*h).unwrap();
        assert_eq!(PoolArc::count(&h), 2);
        drop(again);
        // Its last reference gone, the item still readable through the `Last` (as a list
        // under its lock lends it): `fd_iterfile` skips it.
        let last = PoolArc::try_last(h).unwrap();
        assert!(PoolArc::try_from_ref(&*last).is_none());
        assert!(PoolArc::adopt(&*last).is_none());
        drop(last);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(P.nout(), 0);
        // A value no pool placed: nothing.
        let local = Filey {
            v: 0,
            home: PoolHome::new(),
            drops: drops.clone(),
        };
        assert!(PoolArc::try_from_ref(&local).is_none());
        assert!(PoolArc::adopt(&local).is_none());
    }

    /// `count_of` reads the count through the value with no handle made: the count does not
    /// move, a parked reference is counted, and a value at count 0 or of no pool reads 0.
    #[test]
    fn count_of_reads_the_count_without_a_reference() {
        testutil::install();
        static P: TypedPool<Filey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "countpl"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let h = filey(&P, 4, &drops);
        assert_eq!(PoolArc::count_of(&*h), 1);
        assert_eq!(PoolArc::count(&h), 1);
        let two = h.clone();
        PoolArc::forget(two);
        assert_eq!(PoolArc::count_of(&*h), 2);
        let back = PoolArc::adopt(&*h).unwrap();
        drop(back);
        assert_eq!(PoolArc::count_of(&*h), 1);
        // The last handle's drop is never started by the read: the value is still there.
        let last = PoolArc::try_last(h).unwrap();
        assert_eq!(PoolArc::count_of(&*last), 0);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(last);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        let local = Filey {
            v: 0,
            home: PoolHome::new(),
            drops: drops.clone(),
        };
        assert_eq!(PoolArc::count_of(&local), 0);
    }

    #[test]
    #[should_panic(expected = "an item whose last handle is gone")]
    fn counting_a_value_at_count_zero_again_stops_the_kernel() {
        testutil::install();
        static P: TypedPool<Filey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "lastpanic"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let last = PoolArc::try_last(filey(&P, 2, &drops)).unwrap();
        let _ = PoolArc::from_ref(&*last);
    }

    #[test]
    fn a_parked_reference_is_counted_and_adopted_once() {
        testutil::install();
        static P: TypedPool<Filey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "parkpl"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        // The C's f_count through a message: the slot's reference, FREF'd into the message
        // (unp_internalize), the slot closed, the message received (unp_externalize).
        let slot = filey(&P, 3, &drops);
        assert_eq!(PoolArc::count(&slot), 1);
        PoolArc::forget(slot.clone());
        assert_eq!(PoolArc::count(&slot), 2);
        let other = slot.clone();
        drop(slot);
        assert_eq!(PoolArc::count(&other), 2);
        let received = PoolArc::adopt(&*other).unwrap();
        assert!(PoolArc::ptr_eq(&received, &other));
        assert_eq!(PoolArc::count(&other), 2);
        // One reference was parked: a second adoption finds none.
        assert!(PoolArc::adopt(&*other).is_none());
        assert_eq!(PoolArc::count(&other), 2);
        drop(other);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(received.v, 3);
        drop(received);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(P.nout(), 0);
    }

    #[test]
    fn a_parked_reference_keeps_its_item_after_every_handle() {
        testutil::install();
        static P: TypedPool<Filey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "parkkeep"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let h = filey(&P, 4, &drops);
        let lent = PoolArc::leak(h.clone());
        PoolArc::forget(h);
        // The leaked handle and the parked reference.
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        let back = PoolArc::adopt(lent).unwrap();
        assert_eq!(PoolArc::count(&back), 2);
        assert!(PoolArc::try_last(back).is_none());
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(P.nout(), 1);
    }

    #[test]
    fn parked_references_cross_cpus_and_the_last_release_frees_once() {
        testutil::install();
        static P: TypedPool<Filey> = TypedPool::new();
        assert_eq!(P.init_recorded(0, PR_WAITOK, "parkmp"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let h = filey(&P, 5, &drops);
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let mine = h.clone();
                thread::spawn(move || {
                    for _ in 0..1000 {
                        PoolArc::forget(mine.clone());
                        let back = PoolArc::adopt(&*mine).unwrap();
                        assert!(PoolArc::count(&back) >= 2);
                        drop(back);
                    }
                    PoolArc::try_last(mine).is_some()
                })
            })
            .collect();
        let lasts = workers
            .into_iter()
            .map(|w| w.join().unwrap())
            .filter(|&last| last)
            .count();
        assert_eq!(lasts, 0);
        assert_eq!(PoolArc::count(&h), 1);
        assert!(PoolArc::adopt(&*h).is_none());
        drop(h);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(P.nout(), 0);
    }
}

/* </TESTS> */
