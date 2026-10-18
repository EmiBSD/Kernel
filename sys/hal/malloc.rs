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
//! `MBox<T>`: a `T` in a `malloc(9)` block of the C's type, owned; its drop is the `free`.
//! `MArray<T>`: `n` of them, `mallocarray(9)`'s.
//!
//! The C writes `p = malloc(sizeof(*p), M_KEVENT, M_WAITOK)` and later `free(p, M_KEVENT,
//! sizeof(*p))`, the type and the size repeated at both ends; LZ keeps the shape. An `MBox<T>`
//! is made by [`MBox::new`] with the C's malloc type and wait flags (decision 19: kqueue's
//! timer is `MBox::new(M_KEVENT, M_WAITOK, filt_timer)`), so `vmstat -m` counts it in the
//! same row and `M_WAITOK` sleeps where the C sleeps; its drop drops the `T` and frees the
//! block with the same type and size. Rejected: `Box` (the global allocator is `M_TEMP,
//! M_NOWAIT`: another `vmstat -m` row, and a failure where the C sleeps).
//!
//! The blocks come from `sys/kern/kern_malloc.rs` through the [`Malloc`] bridge (bridges.toml;
//! paid by N9, when the old runtime files go), a safe trait like the other memory bridges
//! (decision 30): it returns a pointer and the size granted, and builds nothing unsafe.
//!
//! **Before registration** (early boot, host tests that register none) there is no
//! `kmem_map`: [`MBox::new`] answers `None` whatever the flags (the value is dropped).
//!
//! ## Redesign
//! - `malloc(size, type, flags)` + initialisation -> [`MBox::new`], `None` only when the C's
//!   `malloc` returns `NULL` (`M_NOWAIT`, `M_CANFAIL`); `free(p, type, size)` -> the drop. The
//!   type and the size are the box's, so the two ends cannot disagree.
//! - The block must hold a `T`: `malloc(9)` aligns every block to at least
//!   [`MALLOC_ALIGN`] (its smallest bucket, `1 << MINBUCKET`), which a `T` of a larger
//!   alignment refuses at compile time; a zero-sized `T` likewise (`malloc(0)` has no use).
//! - Writing the `T` into a block, and every reference to it, rest on the registration
//!   contract of the bridge (the kernel's boot path registers its own `malloc(9)`, once,
//!   before any user; `sys/hal/trust.rs`): a documented trust boundary, not a compile-time
//!   proof. `hal` checks every block (aligned for `T`, the size granted; a probe of the first
//!   and last byte under `diagnostic`) and panics naming the bridge when one fails. It goes
//!   with the bridge, at N9.
//! - N3.4a: `mallocarray(n, sizeof(T), ty, flags)` + a loop that fills it -> [`MArray::new`]
//!   with an initialiser per index, `Deref`/`DerefMut<Target = [T]>`; the drop drops each
//!   element and frees with the same type and size (`free(p, ty, n * sizeof(T))`). The C's
//!   overflow test is kept: `n * size_of::<T>()` past `SIZE_MAX` (and here past `isize::MAX`,
//!   the bound of a slice) is `None` under `M_CANFAIL` and `panic(9)` (`"mallocarray:
//!   overflow"`) otherwise. The amap's `am_ppref` and `am_buckets`, the aobj's `u_swslots`
//!   and the `hashinit(9)` tables (N3.4b/c).
//! - N3.5 S0: [`MBox::leak`], the box given up for a `&'static T` whose block is never freed
//!   (the three boot submaps of `uvm_km_suballoc`).

use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use crate::assert::hal_panic;
use crate::bridges;
use crate::trust::check_block;

/// `M_WAITOK`: `malloc` may sleep, and then never fails.
pub const M_WAITOK: i32 = 0x0001;
/// `M_NOWAIT`: `malloc` fails instead of sleeping.
pub const M_NOWAIT: i32 = 0x0002;
/// `M_CANFAIL`: an `M_WAITOK` request past the type's limit fails instead of panicking.
pub const M_CANFAIL: i32 = 0x0004;

/// The alignment of every `malloc(9)` block: its smallest bucket, `1 << MINBUCKET`.
pub const MALLOC_ALIGN: usize = 16;

/// `malloc(9)` and `free(9)`, as `bsd` registers them (`sys/kern/kern_malloc.rs`).
///
/// The registration contract (`sys/hal/trust.rs`): `malloc(size, ty, flags)` returns `None`
/// or a block and the size granted (at least `size`), aligned to at least [`MALLOC_ALIGN`],
/// counted under `ty`, that nothing else references until `free` gets it back with the same
/// `ty` and `size`. `hal` checks the alignment and the size of every block.
pub trait Malloc: Sync {
    /// `malloc(size, ty, flags)` (`M_WAITOK`, `M_NOWAIT`, `M_CANFAIL`): the block and the size
    /// granted.
    fn malloc(&self, size: usize, ty: i32, flags: i32) -> Option<(NonNull<u8>, usize)>;
    /// `free(ptr, ty, size)`: the contract's other half. The block given back is one this
    /// bridge granted, with its `ty` and `size`, and no view of it is alive; `hal` alone
    /// calls it (from `MBox`'s drop and `into_inner`), since the registered object never
    /// leaves `hal` (`bridges::malloc` is crate-private).
    fn free(&self, ptr: NonNull<u8>, ty: i32, size: usize);
}

/// A `T` in a block of `malloc(9)` of type `ty`, owned: `Deref`/`DerefMut`, the drop drops the
/// `T` and frees the block.
///
/// Invariant: `ptr` came from the registered [`Malloc`]'s `malloc(size_of::<T>(), ty, _)`,
/// holds an initialised `T`, and only this box references it.
pub struct MBox<T> {
    ptr: NonNull<T>,
    ty: i32,
    mem: &'static dyn Malloc,
    _owns: PhantomData<T>,
}

impl<T> MBox<T> {
    /// `malloc(sizeof(T), ty, flags)` holding `value`: `None` when `malloc` returns `NULL`
    /// (`M_NOWAIT`, `M_CANFAIL`) or before the bridge is registered; with `M_WAITOK` alone it
    /// sleeps for the memory, as the C does.
    pub fn new(ty: i32, flags: i32, value: T) -> Option<MBox<T>> {
        const {
            assert!(size_of::<T>() != 0, "MBox of a zero-sized type");
            assert!(
                align_of::<T>() <= MALLOC_ALIGN,
                "MBox of a type malloc cannot align"
            );
        }
        let mem = bridges::malloc()?;
        let (block, granted) = mem.malloc(size_of::<T>(), ty, flags)?;
        check_block("Malloc", block, granted, size_of::<T>(), align_of::<T>());
        let ptr = block.cast::<T>();
        // SAFETY: `ptr` is aligned for `T` and names at least `size_of::<T>()` bytes
        // (`check_block` above), a fresh block that nothing else references: the registration
        // contract of `Malloc` (the kernel's boot path registered its own `malloc(9)`, once,
        // before any user; `sys/hal/trust.rs`), a documented trust boundary. Writing the value
        // initialises it before the box exists.
        unsafe { ptr.as_ptr().write(value) };
        Some(MBox {
            ptr,
            ty,
            mem,
            _owns: PhantomData,
        })
    }

    /// The malloc type the block is counted under.
    pub fn ty(this: &Self) -> i32 {
        this.ty
    }

    /// Gives the box up for a reference that lives for ever: the block is never freed and
    /// stays counted under its type (`uvm_km_suballoc`'s `uvm_map_create` of a submap in
    /// `M_VMMAP`, never freed: `kmem_map`, `exec_map`, `phys_map`), as
    /// [`crate::pool::PoolArc::leak`] does for a pool item.
    pub fn leak(this: Self) -> &'static T {
        let this = ManuallyDrop::new(this);
        // SAFETY: the box holds an initialised `T` that only it references (the invariant);
        // the box is never dropped (`ManuallyDrop`, not returned), so its block is never freed
        // and the value stays valid for the rest of the kernel's life; the box is gone, so the
        // shared reference made here is the only way to the value from now on, and no `&mut`
        // can be made of it again.
        unsafe { this.ptr.as_ref() }
    }

    /// The value, the block freed.
    pub fn into_inner(this: Self) -> T {
        let this = ManuallyDrop::new(this);
        // SAFETY: the box holds an initialised `T` that only it references (the invariant);
        // it is read out once, and `ManuallyDrop` keeps the box's drop from dropping it again,
        // so the block is freed below without touching the value.
        let value = unsafe { this.ptr.as_ptr().read() };
        this.mem.free(this.ptr.cast(), this.ty, size_of::<T>());
        value
    }
}

impl<T> Deref for MBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the box's `T` is initialised and only the box references it (the
        // invariant); the reference borrows the box, so it ends before the drop frees it.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for MBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as for `deref`, and `&mut self` makes this the only reference while it
        // lives.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for MBox<T> {
    fn drop(&mut self) {
        // SAFETY: the box's `T` is initialised and only the box references it (the
        // invariant); it is dropped once, here, before its block is freed.
        unsafe { core::ptr::drop_in_place(self.ptr.as_ptr()) };
        self.mem.free(self.ptr.cast(), self.ty, size_of::<T>());
    }
}

// SAFETY: the box owns its `T` outright (only it references the block), so sending the box
// sends the `T`; the bridge is `Sync`.
unsafe impl<T: Send> Send for MBox<T> {}

// SAFETY: `&MBox<T>` gives only `&T`, so sharing it is sharing `&T`.
unsafe impl<T: Sync> Sync for MBox<T> {}

/// `n` values of `T` in one block of `malloc(9)` of type `ty`, owned (`mallocarray(9)`):
/// `Deref`/`DerefMut<Target = [T]>`; the drop drops every element and frees the block.
///
/// Invariant: `ptr` came from the registered [`Malloc`]'s `malloc(len * size_of::<T>(), ty, _)`
/// (`check_block`ed for that size and `T`'s alignment, `len * size_of::<T>()` at most
/// `isize::MAX`), holds `len` initialised `T`s, and only this array references it.
pub struct MArray<T> {
    ptr: NonNull<T>,
    len: usize,
    ty: i32,
    mem: &'static dyn Malloc,
    _owns: PhantomData<T>,
}

impl<T> MArray<T> {
    /// `mallocarray(n, sizeof(T), ty, flags)`, element `i` set to `init(i)`: `None` when
    /// `malloc` returns `NULL` (`M_NOWAIT`, `M_CANFAIL`) or before the bridge is registered,
    /// and when `n * size_of::<T>()` overflows under `M_CANFAIL`; the overflow without it is
    /// `panic(9)`, as the C's. With `M_WAITOK` alone it sleeps for the memory. `n` may be 0
    /// (`malloc(0)`, a block of the smallest bucket, as the C asks for one). A panic in
    /// `init` leaks the block and the elements made so far.
    pub fn new(ty: i32, flags: i32, n: usize, mut init: impl FnMut(usize) -> T) -> Option<Self> {
        const {
            assert!(size_of::<T>() != 0, "MArray of a zero-sized type");
            assert!(
                align_of::<T>() <= MALLOC_ALIGN,
                "MArray of a type malloc cannot align"
            );
        }
        let Some(bytes) = n
            .checked_mul(size_of::<T>())
            .filter(|&b| b <= isize::MAX.unsigned_abs())
        else {
            if flags & M_CANFAIL != 0 {
                return None;
            }
            hal_panic(format_args!(
                "mallocarray: overflow {n} * {}",
                size_of::<T>()
            ));
        };
        let mem = bridges::malloc()?;
        let (block, granted) = mem.malloc(bytes, ty, flags)?;
        check_block("Malloc", block, granted, bytes, align_of::<T>());
        let ptr = block.cast::<T>();
        for i in 0..n {
            let value = init(i);
            // SAFETY: `ptr` is aligned for `T` and names at least `n * size_of::<T>()` bytes
            // (`check_block` above, `bytes` computed without overflow), so element `i < n` is
            // in the block; the block is fresh and nothing else references it (the
            // registration contract of `Malloc`, `sys/hal/trust.rs`, a documented trust
            // boundary). Writing a whole value initialises element `i`, once, before the
            // array exists.
            unsafe { ptr.add(i).write(value) };
        }
        Some(MArray {
            ptr,
            len: n,
            ty,
            mem,
            _owns: PhantomData,
        })
    }

    /// The malloc type the block is counted under.
    pub fn ty(this: &Self) -> i32 {
        this.ty
    }

    /// Gives the block up, as `Box::into_raw`: its first element and its count. The elements
    /// stay initialised and nothing frees them here; whoever takes the block frees it with
    /// `free(ptr, ty, len * size_of::<T>())` of the same `malloc(9)` (`pinsyscalls(2)`'s
    /// table, which the process keeps until `exit`/`exec` free it as the C's
    /// `free(pn_pins, M_PINSYSCALL, ..)`), or leaks it.
    pub fn into_raw(this: Self) -> (NonNull<T>, usize) {
        let this = core::mem::ManuallyDrop::new(this);
        (this.ptr, this.len)
    }
}

impl<T> Deref for MArray<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        // SAFETY: `ptr` holds `len` initialised `T`s, aligned, in one block of at most
        // `isize::MAX` bytes that only this array references (the invariant); the slice
        // borrows the array, so it ends before the drop frees the block.
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for MArray<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: as for `deref`, and `&mut self` makes this the only reference to the
        // elements while it lives.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for MArray<T> {
    fn drop(&mut self) {
        let elements = core::ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len);
        // SAFETY: the `len` elements are initialised and only this array references them (the
        // invariant); each is dropped once, here, before the block is freed, and nothing
        // reads them afterwards.
        unsafe { core::ptr::drop_in_place(elements) };
        self.mem
            .free(self.ptr.cast(), self.ty, self.len * size_of::<T>());
    }
}

// SAFETY: the array owns its elements outright (only it references the block), so sending
// the array sends the `T`s; the bridge is `Sync`.
unsafe impl<T: Send> Send for MArray<T> {}

// SAFETY: `&MArray<T>` gives only `&[T]`, so sharing it is sharing `&T`s.
unsafe impl<T: Sync> Sync for MArray<T> {}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::testutil;

    /// `M_KEVENT`, a type of the C's table.
    const M_KEVENT: i32 = 137;

    struct Timer {
        ticks: u64,
        drops: Arc<AtomicU32>,
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn the_box_holds_the_value_counted_under_its_type_and_frees_it() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let before = testutil::malloc_out(M_KEVENT);
        let mut b = MBox::new(
            M_KEVENT,
            M_WAITOK,
            Timer {
                ticks: 5,
                drops: Arc::clone(&drops),
            },
        )
        .unwrap();
        assert_eq!(MBox::ty(&b), M_KEVENT);
        b.ticks += 1;
        assert_eq!(b.ticks, 6);
        assert_eq!(
            testutil::malloc_out(M_KEVENT),
            before + size_of::<Timer>() as isize
        );
        drop(b);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(testutil::malloc_out(M_KEVENT), before);
    }

    #[test]
    fn into_inner_frees_the_block_and_keeps_the_value() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let before = testutil::malloc_out(M_KEVENT);
        let b = MBox::new(
            M_KEVENT,
            M_NOWAIT,
            Timer {
                ticks: 1,
                drops: Arc::clone(&drops),
            },
        )
        .unwrap();
        let t = MBox::into_inner(b);
        assert_eq!(testutil::malloc_out(M_KEVENT), before);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(t.ticks, 1);
        drop(t);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    /// `M_UVMAMAP`, a type of the C's table.
    const M_UVMAMAP: i32 = 105;

    #[test]
    fn an_array_holds_its_elements_under_its_type_and_drops_each() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let before = testutil::malloc_out(M_UVMAMAP);
        let mut a = MArray::new(M_UVMAMAP, M_WAITOK, 5, |i| Timer {
            ticks: i as u64 * 10,
            drops: Arc::clone(&drops),
        })
        .unwrap();
        assert_eq!(MArray::ty(&a), M_UVMAMAP);
        assert_eq!(a.len(), 5);
        assert_eq!(
            a.iter().map(|t| t.ticks).collect::<std::vec::Vec<_>>(),
            [0, 10, 20, 30, 40]
        );
        a[4].ticks += 1;
        assert_eq!(a[4].ticks, 41);
        assert_eq!(
            testutil::malloc_out(M_UVMAMAP),
            before + (5 * size_of::<Timer>()) as isize
        );
        drop(a);
        assert_eq!(drops.load(Ordering::Relaxed), 5);
        assert_eq!(testutil::malloc_out(M_UVMAMAP), before);
    }

    #[test]
    fn into_raw_hands_the_block_over_counted_and_unfreed() {
        // pinsyscalls(2)'s table: the process keeps the block and exit/exec free it with
        // the C's free(pn_pins, M_PINSYSCALL, npins * sizeof(u_int)).
        testutil::install();
        let before = testutil::malloc_out(M_UVMAMAP);
        let a = MArray::new(M_UVMAMAP, M_WAITOK, 6, |i| i as u32 * 3).unwrap();
        let (ptr, n) = MArray::into_raw(a);
        assert_eq!(n, 6);
        assert_eq!(
            testutil::malloc_out(M_UVMAMAP),
            before + (6 * size_of::<u32>()) as isize,
            "still allocated"
        );
        bridges::malloc()
            .unwrap()
            .free(ptr.cast(), M_UVMAMAP, n * size_of::<u32>());
        assert_eq!(testutil::malloc_out(M_UVMAMAP), before);
    }

    #[test]
    fn an_array_of_ints_and_an_empty_one() {
        testutil::install();
        let before = testutil::malloc_out(M_UVMAMAP);
        let mut ppref = MArray::new(M_UVMAMAP, M_NOWAIT, 16, |_| 0i32).unwrap();
        ppref[3] = -2;
        assert_eq!(ppref.iter().sum::<i32>(), -2);
        let none = MArray::new(M_UVMAMAP, M_NOWAIT, 0, |_| 0u64).unwrap();
        assert!(none.is_empty());
        drop((ppref, none));
        assert_eq!(testutil::malloc_out(M_UVMAMAP), before);
    }

    #[test]
    fn an_overflowing_array_fails_under_m_canfail() {
        testutil::install();
        let mut called = false;
        let a = MArray::new(M_UVMAMAP, M_WAITOK | M_CANFAIL, usize::MAX / 2, |_| {
            called = true;
            0u32
        });
        assert!(a.is_none() && !called);
        // Past isize::MAX bytes but not past SIZE_MAX: a slice cannot hold it either.
        let a = MArray::new(
            M_UVMAMAP,
            M_CANFAIL,
            isize::MAX.unsigned_abs() / 4 + 1,
            |_| 0u32,
        );
        assert!(a.is_none());
    }

    #[test]
    #[should_panic(expected = "mallocarray: overflow")]
    fn an_overflowing_array_panics_without_m_canfail() {
        testutil::install();
        let _ = MArray::new(M_UVMAMAP, M_WAITOK, usize::MAX / 2, |_| 0u32);
    }

    #[test]
    fn a_leaked_box_stays_counted_and_is_never_freed() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let before = testutil::malloc_out(M_KEVENT);
        let b = MBox::new(
            M_KEVENT,
            M_WAITOK,
            Timer {
                ticks: 9,
                drops: Arc::clone(&drops),
            },
        )
        .unwrap();
        let r: &'static Timer = MBox::leak(b);
        assert_eq!(r.ticks, 9);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(
            testutil::malloc_out(M_KEVENT),
            before + size_of::<Timer>() as isize
        );
    }
}
/* </TESTS> */
