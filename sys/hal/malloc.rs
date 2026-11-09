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
//! `MArray<T>`: `n` of them, `mallocarray(9)`'s. `MArc<T>`: a `T` in a `malloc(9)` block, counted.
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
//! ## Deviations
//! - An [`MArc<T>`]'s block is the `T` with its reference count and its malloc type beside it
//!   (`MInner<T>`: 16 bytes, more when `T`'s alignment pads it), so a structure the C
//!   `malloc`s and frees from one place, and that is shared by handles here (the UFS mount
//!   structure, `M_UFSMNT`), adds those bytes to its type's `vmstat -m` row (the in-use and
//!   high-water bytes; the request counts are the C's). No `diff-openbsd` scenario prints
//!   `vmstat -m`; `TypedPool`'s items carry the same 16 bytes (`sys/hal/pool.rs`).
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
//! - N4.1: [`MBox::as_pin`], the value pinned (an extent on the list of all extents,
//!   `hal::pinlist`): the block never moves and the value is dropped in place before it is
//!   freed; the two ways to a `T` that could move it, `DerefMut` and [`MBox::into_inner`],
//!   are for `T: Unpin` only.
//! - N5.5b0: [`MArc<T>`], a `malloc(9)`'d structure many holders reach and the last frees
//!   (`struct ufsmount`: `malloc(M_UFSMNT)` by `ffs_mountfs`, hung from `mnt_data`, reached by
//!   every inode's `i_ump`, `free`d by `ffs_unmount` after `vflush`): the count beside the
//!   value, `Clone` counts, the last drop drops the `T` and frees the block. It is erased into
//!   the [`AnyArc`] of a typed slot (the mount's, the vnode's) as a [`crate::pool::PoolArc`]
//!   is, and taken back as a handle by naming its type ([`AnyArc::downcast_arc`]). Made over
//!   [`MBox`] (the block, its checks and its type; [`MBox::into_raw`] gives the box up to the
//!   counted handle), so the only new `unsafe` is the handle's: the shared reference to the
//!   block, the last drop's free, the `Send`/`Sync` of a counted shared value, and the erased
//!   release. `PoolArc`'s proof, for a block of `malloc(9)` instead of a pool item. The erased
//!   handle's fields are private to `pool.rs` (N5.5b0's review): `MArc::erase` builds it
//!   through `AnyArc::from_raw` and `MErased::VT` through `ErasedVt::new` (two `unsafe` with
//!   their contracts), and `AnyArc::downcast_arc` takes the block back through the safe
//!   `AnyArc::into_raw_if`, which checks the kind and the type.
//! - N5.7c2: [`MArc::as_pin`], the value of a counted block pinned (a denode on its hash
//!   chain, `hal::pinlist`, while the vnode's typed slot holds it): [`MBox::as_pin`]'s
//!   argument for a block the last of many handles drops in place, one `unsafe`. No `MArc`
//!   API moves the value or lends it mutably (no `DerefMut`, `get_mut` or `into_inner`).

use core::any::TypeId;
use core::marker::PhantomData;
use core::mem::{ManuallyDrop, offset_of};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering, fence};

use crate::assert::hal_panic;
use crate::bridges;
use crate::pool::{AnyArc, ErasedKind, ErasedVt, MAX_REFS, REFS_MASK};
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
    /// `nkmempages`: the pages of `kmem_map` (`vm.nkmempages`, N4.6b); 0 for an allocator
    /// without one (the tests' doubles).
    fn nkmempages(&self) -> i32 {
        0
    }
}

/// `nkmempages` of the registered allocator (`vm.nkmempages`); 0 before registration.
pub fn nkmempages() -> i32 {
    bridges::malloc().map_or(0, Malloc::nkmempages)
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

    /// The value, pinned: it stays where it is until the box drops it in place.
    pub fn as_pin(&self) -> Pin<&T> {
        // SAFETY: `Pin`'s contract: the value is not moved, nor its memory invalidated or
        // reused, before its drop runs. The value sits in the box's block from `new`'s write
        // until the box's drop, which drops it in place before freeing the block (the
        // invariant: only the box references the block). Nothing in between can move it: the
        // box gives `&T`, `&mut T` (`DerefMut`) and the value by value (`into_inner`) only for
        // `T: Unpin`, so a value for which pinning means something never leaves the block or
        // meets a `&mut`; `leak` gives a `&'static T` and never frees the block.
        unsafe { Pin::new_unchecked(self.ptr.as_ref()) }
    }

    /// The value, the block freed (`T: Unpin`: a pinned value never leaves its block).
    pub fn into_inner(this: Self) -> T
    where
        T: Unpin,
    {
        let this = ManuallyDrop::new(this);
        // SAFETY: the box holds an initialised `T` that only it references (the invariant);
        // it is read out once, and `ManuallyDrop` keeps the box's drop from dropping it again,
        // so the block is freed below without touching the value.
        let value = unsafe { this.ptr.as_ptr().read() };
        this.mem.free(this.ptr.cast(), this.ty, size_of::<T>());
        value
    }

    /// Gives the box up, as `Box::into_raw`: the block, holding its initialised `T`, which
    /// nothing frees here. Its taker frees it with the registered `malloc(9)`'s `free(ptr, ty,
    /// size_of::<T>())` after dropping the `T`, or leaks it ([`MArc`], whose block is an
    /// `MBox<MInner<T>>` given up so).
    pub(crate) fn into_raw(this: Self) -> NonNull<T> {
        ManuallyDrop::new(this).ptr
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

impl<T: Unpin> DerefMut for MBox<T> {
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

/// What an [`MArc<T>`]'s block holds: the count, the malloc type, the value.
struct MInner<T> {
    /// The live handles (the `MArc`s and the [`AnyArc`]s erased from them), in the low half
    /// as [`crate::pool::PoolArc`]'s count ([`REFS_MASK`], [`MAX_REFS`]); none is parked.
    refs: AtomicUsize,
    /// The malloc type the block is counted under.
    ty: i32,
    /// The value.
    value: T,
}

/// A `T` in a block of `malloc(9)` of type `ty`, shared: `Deref<Target = T>`, `Clone` adds a
/// reference, the last drop drops the `T` and frees the block (`free(p, ty, size)`). The
/// structure the C `malloc`s at one place, hangs where many reach it, and `free`s once they
/// are gone (`struct ufsmount`, `mnt_data`, every inode's `i_ump`).
///
/// Invariant: `inner` is the block of an `MBox<MInner<T>>` that [`MArc::new`] made and gave up
/// ([`MBox::into_raw`]): an initialised `MInner<T>` in a block of the registered [`Malloc`] of
/// `size_of::<MInner<T>>()` bytes counted under `inner.ty`, whose `refs` counts (in its low
/// half) the live handles to it, this one included, and the [`AnyArc`]s erased from them; the
/// value is dropped and the block freed only when that count reaches 0, so the block outlives
/// every handle. Only shared references to the block are made while handles live. The value
/// never leaves its block and is never lent mutably while handles live (`as_pin`); a move-out
/// or `&mut` API would need `T: Unpin`.
pub struct MArc<T: Send + Sync + 'static> {
    inner: NonNull<MInner<T>>,
    _owns: PhantomData<MInner<T>>,
}

impl<T: Send + Sync + 'static> MArc<T> {
    /// `malloc(sizeof(T), ty, flags)` holding `value`, one handle: `None` when `malloc`
    /// returns `NULL` (`M_NOWAIT`, `M_CANFAIL`) or before the bridge is registered; with
    /// `M_WAITOK` alone it sleeps for the memory, as the C does. The block holds the count and
    /// the type beside the value (the module's deviations).
    pub fn new(ty: i32, flags: i32, value: T) -> Option<Self> {
        let b = MBox::new(
            ty,
            flags,
            MInner {
                refs: AtomicUsize::new(1),
                ty,
                value,
            },
        )?;
        Some(MArc {
            inner: MBox::into_raw(b),
            _owns: PhantomData,
        })
    }

    fn inner(&self) -> &MInner<T> {
        // SAFETY: the block is initialised and outlives every handle, this one included (the
        // invariant: it is freed only when the count this handle holds, among the others,
        // reaches 0, which needs this handle's drop, so not while `&self` is borrowed); only
        // shared references to the block are made while handles live (no API gives `&mut`), so
        // this one aliases no `&mut`.
        unsafe { self.inner.as_ref() }
    }

    /// The malloc type the block is counted under.
    pub fn ty(this: &Self) -> i32 {
        this.inner().ty
    }

    /// Whether two handles name the same block.
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        a.inner == b.inner
    }

    /// The value, pinned: it stays where it is until the last handle drops it in place (a
    /// denode linked on its hash chain while its vnode's slot holds a handle).
    pub fn as_pin(&self) -> Pin<&T> {
        let value = &self.inner().value;
        // SAFETY: `Pin`'s contract: the value is not moved, nor its memory invalidated or
        // reused, before its drop runs. The value is written into its block once, by `new`
        // (inside the `MInner` the `MBox` placed), and the block is freed only by the drop of
        // the last handle, which drops the `MInner`, the value with it, in place first (the
        // invariant: the count of live handles and erased handles reaches 0 only there).
        // Nothing in between can move it or reach it mutably: every way to the value, from a
        // handle (`Deref`), from an erased one (`AnyArc::downcast_ref`) or from a handle taken
        // back (`AnyArc::downcast_arc`), gives `&T` only; no `DerefMut`, `get_mut`,
        // `into_inner` or `try_unwrap` exists, so whatever pinning promises for a `T` that is
        // not `Unpin` holds from here to that drop, even after this borrow ends.
        unsafe { Pin::new_unchecked(value) }
    }

    /// The handle with its type erased ([`AnyArc`]): the same reference, counted as before,
    /// lent back as a `&T` to who names `T` ([`AnyArc::downcast_ref`]) and taken back as a
    /// handle by [`AnyArc::downcast_arc`]: a mount's typed slot (`mnt_data`).
    pub fn erase(this: Self) -> AnyArc {
        let this = ManuallyDrop::new(this);
        // SAFETY: `from_raw`'s contract: `this.inner` is the block of this live handle over a
        // `T: Send + Sync + 'static` (the type's invariant), whose reference is given over here
        // (`ManuallyDrop`: the handle is not dropped); `MErased::<T>::VT` is made for that `T`
        // and for `MArc`'s blocks (`ErasedKind::Malloc`).
        unsafe { AnyArc::from_raw(this.inner.cast(), &MErased::<T>::VT) }
    }
}

impl<T: Send + Sync + 'static> Deref for MArc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T: Send + Sync + 'static> Clone for MArc<T> {
    fn clone(&self) -> Self {
        let old = self.inner().refs.fetch_add(1, Ordering::Relaxed);
        if old & REFS_MASK >= MAX_REFS {
            hal_panic(format_args!("MArc: reference count overflow"));
        }
        MArc {
            inner: self.inner,
            _owns: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static> Drop for MArc<T> {
    fn drop(&mut self) {
        // Release: this handle's accesses happen before the last one's free.
        if self.inner().refs.fetch_sub(1, Ordering::Release) & REFS_MASK != 1 {
            return;
        }
        // The last reference: every other handle's accesses happen before the free.
        fence(Ordering::Acquire);
        let ty = self.inner().ty;
        let Some(mem) = bridges::malloc() else {
            hal_panic(format_args!("MArc: a block of a malloc never registered"));
        };
        // SAFETY: the count reached 0 with this handle's release, so no other handle, and no
        // `AnyArc` erased from one, exists, and none can be made (a clone needs a live handle);
        // the shared references `inner` made above borrowed `self` and have ended. The block
        // holds an initialised `MInner<T>` (the invariant), dropped once, here, before the
        // block goes back, whole, to the `Malloc` that granted it (registered once, so the
        // one `bridges::malloc` answers now), with its type and its size, as
        // `MBox::into_raw`'s taker must.
        unsafe { core::ptr::drop_in_place(self.inner.as_ptr()) };
        mem.free(self.inner.cast(), ty, size_of::<MInner<T>>());
    }
}

// SAFETY: a handle gives `&T` on any CPU (`T: Sync`) and the last one, on whichever CPU drops
// it, drops the `T` there (`T: Send`); the count is atomic.
unsafe impl<T: Send + Sync + 'static> Send for MArc<T> {}

// SAFETY: `&MArc<T>` gives `&T` and clones (an atomic increment); with `T: Sync` sharing it
// across CPUs is sound, and a clone moved elsewhere is covered by `Send` above.
unsafe impl<T: Send + Sync + 'static> Sync for MArc<T> {}

/// The [`ErasedVt`] of an [`MArc<T>`].
struct MErased<T>(PhantomData<T>);

impl<T: Send + Sync + 'static> MErased<T> {
    // SAFETY: `ErasedVt::new`'s contract, for `MArc<T>`'s blocks: every block is an
    // initialised `MInner<T>` (`MArc`'s invariant) whose `refs` (an `AtomicUsize` counted in
    // its low half, a clone adding one, as `MArc::clone` does) and `value` (a `T: Send + Sync
    // + 'static`, only ever lent shared) sit at the offsets given; `TypeId::of::<T>` is the
    // `T`'s; `merased_release::<T>` drops one handle's reference (the last frees the block);
    // `ErasedKind::Malloc` is used by no other layout.
    const VT: ErasedVt = unsafe {
        ErasedVt::new(
            TypeId::of::<T>,
            ErasedKind::Malloc,
            offset_of!(MInner<T>, refs),
            offset_of!(MInner<T>, value),
            merased_release::<T>,
        )
    };
}

impl AnyArc {
    /// The handle back, when the erased value is an [`MArc<T>`]'s `T`: the same reference,
    /// counted as before; `None` (the erased handle dropped) for another type or for a pool
    /// item ([`crate::pool::PoolArc::erase`]'s), whose block is not a malloc'd `MInner<T>`.
    pub fn downcast_arc<T: Send + Sync + 'static>(self) -> Option<MArc<T>> {
        // `Err` is the erased handle, dropped here (its reference released).
        let item = self.into_raw_if::<T>(ErasedKind::Malloc).ok()?;
        // The kind and the type checked: `item` is the block of an `MArc<T>` whose reference
        // the erased handle owned and gave over, now this handle's.
        Some(MArc {
            inner: item.cast(),
            _owns: PhantomData,
        })
    }
}

/// One reference fewer to the `MInner<T>` at `item` ([`AnyArc`]'s drop of an erased
/// [`MArc<T>`]); the last one drops the `T` and frees the block.
///
/// # Safety
///
/// `item` names the `MInner<T>` of a block holding a reference the caller owns and gives up
/// here, once.
unsafe fn merased_release<T: Send + Sync + 'static>(item: NonNull<()>) {
    drop(MArc::<T> {
        inner: item.cast(),
        _owns: PhantomData,
    });
}

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

// `None` of a box or a counted handle is a niche of its pointer, no tag (review 2's nit): so
// zero bytes, a null pointer, are `None`, which `hal::slot`'s
// `an_all_zero_cell_of_no_box_or_handle_holds_none` relies on for the interface's cells in
// zeroed softcs.
const _: () = {
    assert!(size_of::<Option<MBox<u64>>>() == size_of::<MBox<u64>>());
    assert!(size_of::<Option<MArc<u64>>>() == size_of::<MArc<u64>>());
};
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

    #[test]
    fn a_boxed_value_is_pinned_in_its_block() {
        testutil::install();
        struct Fixed {
            v: u32,
            _pin: core::marker::PhantomPinned,
        }
        let b = MBox::new(
            M_KEVENT,
            M_WAITOK,
            Fixed {
                v: 9,
                _pin: core::marker::PhantomPinned,
            },
        )
        .unwrap();
        let p = b.as_pin();
        assert!(core::ptr::eq(p.get_ref(), &*b));
        assert_eq!(p.v, 9);
    }

    /// `M_UFSMNT`, a type of the C's table no other test of this crate uses (the tests run in
    /// parallel and compare a type's bytes out).
    const M_UFSMNT: i32 = 28;

    /// The bytes an `MArc<Timer>` takes: the timer and the 16-byte header.
    const ARC_BYTES: isize = (size_of::<Timer>() + 16) as isize;

    fn timer(ticks: u64, drops: &Arc<AtomicU32>) -> Timer {
        Timer {
            ticks,
            drops: Arc::clone(drops),
        }
    }

    #[test]
    fn a_counted_block_is_freed_by_its_last_handle() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let before = testutil::malloc_out(M_UFSMNT);
        let a = MArc::new(M_UFSMNT, M_WAITOK, timer(3, &drops)).unwrap();
        assert_eq!(MArc::ty(&a), M_UFSMNT);
        assert_eq!(a.ticks, 3);
        assert_eq!(testutil::malloc_out(M_UFSMNT), before + ARC_BYTES);
        let b = a.clone();
        assert!(MArc::ptr_eq(&a, &b));
        drop(a);
        assert_eq!(drops.load(Ordering::Relaxed), 0, "a handle is left");
        assert_eq!(b.ticks, 3);
        let c = b.clone();
        let crossed = std::thread::spawn(move || c.ticks).join().unwrap();
        assert_eq!(crossed, 3);
        assert_eq!(
            drops.load(Ordering::Relaxed),
            0,
            "a clone dropped on another CPU"
        );
        drop(b);
        assert_eq!(
            drops.load(Ordering::Relaxed),
            1,
            "the last drop drops the value"
        );
        assert_eq!(
            testutil::malloc_out(M_UFSMNT),
            before,
            "and frees the block"
        );
    }

    #[test]
    fn an_erased_counted_block_lends_its_type_and_gives_its_handle_back() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let before = testutil::malloc_out(M_UFSMNT);
        let a = MArc::new(M_UFSMNT, M_WAITOK, timer(7, &drops)).unwrap();
        let kept = a.clone();
        let e = MArc::erase(a);
        assert_eq!(e.downcast_ref::<Timer>().map(|t| t.ticks), Some(7));
        assert!(e.downcast_ref::<u64>().is_none(), "another type is refused");
        let f = e.clone();
        assert!(AnyArc::ptr_eq(&e, &f));
        assert!(
            e.clone().downcast_arc::<u64>().is_none(),
            "and its handle too"
        );
        let back = e.downcast_arc::<Timer>().unwrap();
        assert!(MArc::ptr_eq(&back, &kept));
        drop((back, kept));
        assert_eq!(
            drops.load(Ordering::Relaxed),
            0,
            "the erased clone keeps the block"
        );
        assert_eq!(f.downcast_ref::<Timer>().map(|t| t.ticks), Some(7));
        drop(f);
        assert_eq!(
            drops.load(Ordering::Relaxed),
            1,
            "the last erased drop frees it"
        );
        assert_eq!(testutil::malloc_out(M_UFSMNT), before);
    }

    /// `into_raw_if` refuses another type or kind without touching the count: the erased
    /// handle comes back whole, still lending its value.
    #[test]
    fn a_refused_raw_take_gives_the_erased_handle_back_counted() {
        testutil::install();
        let drops = Arc::new(AtomicU32::new(0));
        let e = MArc::erase(MArc::new(M_UFSMNT, M_WAITOK, timer(4, &drops)).unwrap());
        let e = e.into_raw_if::<u64>(ErasedKind::Malloc).unwrap_err();
        let e = e.into_raw_if::<Timer>(ErasedKind::Pool).unwrap_err();
        assert_eq!(e.downcast_ref::<Timer>().map(|t| t.ticks), Some(4));
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        drop(e);
        assert_eq!(drops.load(Ordering::Relaxed), 1, "still the last reference");
    }

    #[test]
    fn a_pool_item_is_never_taken_back_as_a_counted_block() {
        use crate::pool::{PR_WAITOK, PoolArc, TypedPool};
        static P: TypedPool<Timer> = TypedPool::new();
        testutil::install();
        assert_eq!(P.init(0, PR_WAITOK, "marcpl"), Ok(()));
        let drops = Arc::new(AtomicU32::new(0));
        let h = P.get_shared(PR_WAITOK, || timer(9, &drops)).unwrap();
        let e = PoolArc::erase(h);
        assert_eq!(e.downcast_ref::<Timer>().map(|t| t.ticks), Some(9));
        assert!(
            e.downcast_arc::<Timer>().is_none(),
            "a pool's Timer is no MArc"
        );
        assert_eq!(
            drops.load(Ordering::Relaxed),
            1,
            "the refused handle was dropped"
        );
        assert_eq!(P.nout(), 0);
    }

    /// A counted value with a list entry: the shape of a denode on its hash chain.
    struct Chained<T> {
        v: T,
        e: crate::pinlist::PinEntry<Chained<T>>,
    }

    crate::pin_adapter!(impl[T: Send + Sync + 'static] ByChained<T>: Chained<T>, e);

    /// `as_pin` lends the block's value in place: linked through it, the list names the value
    /// every handle reaches, the other handles keep the block while it is linked, and the
    /// last drop after the unlink drops it in place and frees the block.
    #[test]
    fn a_counted_value_is_pinned_where_every_handle_reaches_it() {
        use crate::intr::IPL_NONE;
        use crate::mutex::Mutexed;
        use crate::pinlist::{PinEntry, PinList};

        testutil::install();
        let before = testutil::malloc_out(M_UFSMNT);
        let a = MArc::new(
            M_UFSMNT,
            M_WAITOK,
            Chained {
                v: 11u64,
                e: PinEntry::new(),
            },
        )
        .unwrap();
        let p = a.as_pin();
        assert!(core::ptr::eq(p.get_ref(), &*a), "the value, not a copy");
        let m = Mutexed::new(IPL_NONE, ());
        let mut g = m.lock();
        let l: PinList<ByChained<u64>> = PinList::new();
        l.insert_head(&mut g, a.as_pin());
        let erased = MArc::erase(a.clone());
        drop(a);
        let back = erased.downcast_arc::<Chained<u64>>().unwrap();
        let first = l.first(&g).unwrap();
        assert!(
            core::ptr::eq(first, &*back),
            "the list names the block's value"
        );
        assert_eq!(first.v, 11);
        l.remove(&mut g, &back);
        assert!(l.is_empty(&g));
        drop(g);
        drop(back);
        assert_eq!(
            testutil::malloc_out(M_UFSMNT),
            before,
            "freed by the last handle"
        );
    }
}
/* </TESTS> */
