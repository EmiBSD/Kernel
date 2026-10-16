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
//! handles.
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

use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::NonNull;
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
    /// `pool_put(pp, item)`.
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
    }
}
/* </TESTS> */
