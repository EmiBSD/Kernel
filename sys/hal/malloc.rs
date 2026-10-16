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

use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

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
}
/* </TESTS> */
