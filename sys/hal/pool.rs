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
//! puts the item back. No `&'static T` is ever made from an item (decision 10): a reference
//! borrows a handle.
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

use core::marker::PhantomData;
use core::mem::{ManuallyDrop, offset_of};
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering, fence};

use crate::assert::hal_panic;
use crate::bridges;
use crate::once::OnceStatic;
use crate::sys::errno::Errno;
use crate::trust::check_block;

/// `PR_WAITOK`: `pool_get` may sleep for an item.
pub const PR_WAITOK: i32 = 0x0001;
/// `PR_NOWAIT`: `pool_get` fails instead of sleeping.
pub const PR_NOWAIT: i32 = 0x0002;
/// `PR_LIMITFAIL`: `pool_get` fails at the hard limit even with `PR_WAITOK`.
pub const PR_LIMITFAIL: i32 = 0x0004;

/// The most references a [`PoolArc`] counts: past it a clone panics, before the count could
/// wrap and free a live item.
const MAX_REFS: usize = isize::MAX as usize;

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
    raw: OnceStatic<&'static dyn RawPool>,
    _t: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> TypedPool<T> {
    /// A pool that `init` has not made yet.
    pub const fn new() -> Self {
        Self {
            raw: OnceStatic::new(),
            _t: PhantomData,
        }
    }

    /// `pool_init(9)`: the pool of `T`s at `ipl`, with `flags` (`PR_WAITOK`), named `wchan`
    /// (the wait message and the name `vmstat -m` shows). `ENOMEM` when the pool cannot be
    /// made (or before the bridge is registered); `EBUSY` when it was already.
    pub fn init(&'static self, ipl: i32, flags: i32, wchan: &'static str) -> Result<(), Errno> {
        if self.raw.is_set() {
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
        self.raw.set(raw).map_err(|_| Errno::EBUSY)
    }

    /// `pool_get(9)` and the item's initialisation: a handle to a new item holding `init()`,
    /// counted once. `None` when `pool_get` fails (`PR_NOWAIT`, `PR_LIMITFAIL`, no memory)
    /// or the pool was never made; `init` is not called then.
    pub fn get_shared(&'static self, flags: i32, init: impl FnOnce() -> T) -> Option<PoolArc<T>> {
        let raw = *self.raw.get()?;
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
        self.raw.get().map_or(0, |r| r.nout())
    }
}

impl<T: Send + Sync + 'static> Default for TypedPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// What an item of a [`TypedPool<T>`] holds: the count, the pool it goes back to, the value.
struct Inner<T: 'static> {
    refs: AtomicUsize,
    pool: &'static TypedPool<T>,
    value: T,
}

/// An owned, counted handle to a pool item holding a `T`: `Deref<Target = T>`, `Clone` adds a
/// reference, the last drop drops the `T` and puts the item back into its pool.
///
/// Invariant: `inner` names an initialised `Inner<T>` of `inner.pool`, whose `refs` counts the
/// live handles to it, this one included; the item is put back only when that count reaches 0,
/// so it outlives every handle.
pub struct PoolArc<T: Send + Sync + 'static> {
    inner: NonNull<Inner<T>>,
    _owns: PhantomData<Inner<T>>,
}

impl<T: Send + Sync + 'static> PoolArc<T> {
    fn inner(&self) -> &Inner<T> {
        // SAFETY: the item is initialised and outlives every handle, this one included (the
        // invariant; that no one else touches it until it is put back is the registration
        // contract of `RawPool`, `sys/hal/trust.rs`); only shared references to it are ever
        // made while handles live (the count is atomic, the value is reached as `&T`), so
        // this one aliases no `&mut`.
        unsafe { self.inner.as_ref() }
    }

    /// Whether two handles name the same item.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.inner == b.inner
    }

    /// The handles alive now (a snapshot).
    pub fn count(this: &Self) -> usize {
        this.inner().refs.load(Ordering::Relaxed)
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
    /// `value` is the address of the value of a live item of a `TypedPool<T>` (one
    /// `get_shared` made), whose count is at least 1 and stays so until this call returns: the
    /// caller holds the lock that keeps the record of the pointer, and the record exists only
    /// while a handle does (a pmap's pv entries, its mappings: `hal::pmap`'s last drop panics
    /// while one is left).
    #[allow(
        dead_code,
        reason = "the pmap backends' (pmap_page_remove), N3.1 commits 4 and 5"
    )]
    pub(crate) unsafe fn from_value(value: NonNull<T>) -> Self {
        let addr = value.as_ptr().addr() - offset_of!(Inner<T>, value);
        let Some(inner) = NonNull::new(ptr::with_exposed_provenance_mut::<Inner<T>>(addr)) else {
            hal_panic(format_args!("PoolArc::from_value: a null item"));
        };
        // A handle the caller's guarantee makes valid, never dropped: cloning it counts the
        // new one (and refuses an overflow), leaving the count of the others as it was.
        let borrowed = ManuallyDrop::new(PoolArc {
            inner,
            _owns: PhantomData,
        });
        PoolArc::clone(&borrowed)
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
        if old >= MAX_REFS {
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
        let inner = self.inner();
        if inner.refs.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }
        // The last handle: every other handle's writes happen before what follows.
        fence(Ordering::Acquire);
        let pool = inner.pool;
        let Some(raw) = pool.raw.get() else {
            hal_panic(format_args!("PoolArc: an item of a pool never made"));
        };
        // SAFETY: the count reached 0, so no other handle, hence no other reference to the
        // item, exists (the invariant) and none can be made; the value is initialised and
        // dropped once, here, before its memory goes back to the pool it came from.
        unsafe { core::ptr::drop_in_place(&raw mut (*self.inner.as_ptr()).value) };
        raw.put(self.inner.cast());
    }
}

// SAFETY: a handle gives `&T` on any CPU (`T: Sync`) and the last one, on whichever CPU drops
// it, drops the `T` there (`T: Send`); the count is atomic.
unsafe impl<T: Send + Sync + 'static> Send for PoolArc<T> {}

// SAFETY: `&PoolArc<T>` gives `&T` and clones (an atomic increment); with `T: Sync` sharing it
// across CPUs is sound, and a clone moved elsewhere is covered by `Send` above.
unsafe impl<T: Send + Sync + 'static> Sync for PoolArc<T> {}

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

impl<T: 'static> DerefMut for PoolBox<T> {
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
    fn a_handle_is_rebuilt_from_its_value() {
        static P: TypedPool<Pair> = TypedPool::new();
        let pool = made(&P);
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
}

/* </TESTS> */
