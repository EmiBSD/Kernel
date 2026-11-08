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
//! `Body` and `Bytes`: a pool item's storage lent as the typed values a tag says it holds, or
//! as bytes (N6.1, the mbuf).
//!
//! `struct mbuf` (`<sys/mbuf.h>`) is a header and two untagged unions: the data area holds a
//! packet header or data, and after the packet header the description of a cluster or data;
//! `m_flags`'s `M_PKTHDR` and `M_EXT` say which. Safe Rust has no untagged union, and an
//! `enum` over a 224-byte area costs a tag word that no niche can hide (any bytes are valid
//! data): `MLEN` would be 216 or `MHLEN` 136, where the C's are 224 and 144 and both are
//! behaviour (`m_pullup`'s limit, `MINCLSIZE`, how `sosend` packs). [`Body`] is the C's union
//! with the tag kept where the C keeps it: two bits of a flags word it owns, which only its
//! own methods set and clear, beside a few bytes of the caller's ([`Body::x`]) and the storage.
//! A typed value is lent while its bit is set ([`Body::p`], [`Body::e`]), the bytes around the
//! live values otherwise ([`Body::inline`]): no state is reachable in which a reference reads
//! bytes as a value they do not hold.
//!
//! [`Bytes`] is the storage of a cluster: `N` bytes of a pool item, lent shared or mutably by
//! the borrow of the handle, and by address ([`Bytes::as_raw`]) to `bsd`'s legacy code, which
//! writes packets in place through `mtod` (the C's convention: one owner of a packet at a
//! time), so the bytes are an `UnsafeCell`'s. [`crate::pool::BoxPool::get_bytes`] hands one
//! out as the C's `pool_get` does, unwritten.
//!
//! Both are written for the mbuf and have no second user: the C's equivalent is the union and
//! the cluster pool item themselves.

use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::slice;

/// `n` rounded up to a multiple of `a` (a power of two).
const fn round_up(n: usize, a: usize) -> usize {
    n.next_multiple_of(a)
}

/// The storage of a [`Body`]: `N` bytes aligned to 8, so a value of alignment up to 8 fits at
/// an offset that is a multiple of its alignment.
#[repr(C, align(8))]
struct Storage<const N: usize>([u8; N]);

/// The tagged overlay of a pool item: a flags word whose bits `PBIT` and `EBIT` say whether a
/// `P` (at offset 0 of the storage) and an `E` (at [`Body::OFF_E`]) are live, `X` (at most six
/// bytes of the owner's, alignment at most 2) beside it, and `N` bytes of storage; `8 + N`
/// bytes in all. The mbuf's: `Body<Pkthdr, Ext, MbufMeta, M_PKTHDR, M_EXT, MLEN>`.
///
/// Invariant: the storage's bytes are initialised everywhere except inside a live value (its
/// padding); the `P` region (`[0, size_of::<P>())`) holds an initialised `P` exactly while
/// `PBIT` is set in `flags`, and the `E` region (`[OFF_E, OFF_E + size_of::<E>())`) an
/// initialised `E` exactly while `EBIT` is set. The two bits are written only by the private
/// `put` and `out` (through [`Body::set_p`], [`Body::take_p`], [`Body::set_e`],
/// [`Body::take_e`] and the drop), which write or read the value at the same time; every other
/// flag write keeps them ([`Body::set_flags`]). `out` writes zeros over the region a value
/// leaves, so its padding bytes never become bytes [`Body::inline`] lends.
#[repr(C)]
pub struct Body<
    P: 'static,
    E: 'static,
    X: 'static,
    const PBIT: u16,
    const EBIT: u16,
    const N: usize,
> {
    flags: Cell<u16>,
    x: X,
    storage: UnsafeCell<Storage<N>>,
    /// The values the storage may hold: the overlay owns them (its drop drops them, its auto
    /// traits are theirs).
    _holds: PhantomData<(P, E)>,
}

impl<P: 'static, E: 'static, X: 'static, const PBIT: u16, const EBIT: u16, const N: usize>
    Body<P, E, X, PBIT, EBIT, N>
{
    /// The two tag bits.
    const TAGS: u16 = PBIT | EBIT;

    /// Where the `E` region starts: after the `P` region, aligned for an `E`.
    pub const OFF_E: usize = round_up(size_of::<P>(), align_of::<E>());

    /// Where the bytes [`Body::inline`] lends start while a `P` is live: after it, aligned to 8.
    pub const INLINE_P: usize = round_up(size_of::<P>(), 8);

    /// The layout's checks, made when a `Body` is built (a compile-time error for a type
    /// that fails them).
    const CHECK: () = {
        assert!(
            PBIT.is_power_of_two() && EBIT.is_power_of_two() && PBIT != EBIT,
            "Body: the tags are two distinct bits"
        );
        assert!(
            size_of::<X>() <= 6 && align_of::<X>() <= 2,
            "Body: X fits beside the flags"
        );
        assert!(
            align_of::<P>() <= 8 && align_of::<E>() <= 8,
            "Body: the storage is aligned to 8"
        );
        assert!(
            Self::OFF_E + size_of::<E>() <= N && Self::INLINE_P <= N,
            "Body: both values fit in the storage"
        );
        assert!(N.is_multiple_of(8), "Body: no padding after the storage");
        assert!(size_of::<Self>() == 8 + N, "Body: 8 + N bytes");
    };

    /// An overlay holding no value, its storage zeroed, `x` beside it, `flags` without the
    /// tag bits.
    pub const fn new(x: X, flags: u16) -> Self {
        let () = Self::CHECK;
        Self {
            flags: Cell::new(flags & !Self::TAGS),
            x,
            storage: UnsafeCell::new(Storage([0; N])),
            _holds: PhantomData,
        }
    }

    /// The flags word, the tag bits included.
    #[inline]
    pub fn flags(&self) -> u16 {
        self.flags.get()
    }

    /// The flags word set to `v` but for the tag bits, which stay as they are: a caller cannot
    /// say a value is live that is not, nor drop one by forgetting its bit.
    #[inline]
    pub fn set_flags(&self, v: u16) {
        self.flags
            .set((v & !Self::TAGS) | (self.flags.get() & Self::TAGS));
    }

    /// The owner's bytes beside the flags.
    #[inline]
    pub fn x(&self) -> &X {
        &self.x
    }

    /// The storage's first byte, for what names the bytes by address (`bsd`'s `mtod`); never
    /// dereferenced here.
    #[inline]
    pub fn as_raw(&self) -> *mut u8 {
        self.storage.get().cast::<u8>()
    }

    /// The `P`, while one is live.
    #[inline]
    pub fn p(&self) -> Option<&P> {
        self.typed(PBIT, 0)
    }

    /// The `P`, to change it.
    #[inline]
    pub fn p_mut(&mut self) -> Option<&mut P> {
        self.typed_mut(PBIT, 0)
    }

    /// Makes `p` the live `P`, returning the one it replaces.
    #[inline]
    pub fn set_p(&mut self, p: P) -> Option<P> {
        self.put(PBIT, 0, p)
    }

    /// The live `P` moved out, its region zeroed.
    #[inline]
    pub fn take_p(&mut self) -> Option<P> {
        self.out(PBIT, 0)
    }

    /// The `E`, while one is live.
    #[inline]
    pub fn e(&self) -> Option<&E> {
        self.typed(EBIT, Self::OFF_E)
    }

    /// The `E`, to change it.
    #[inline]
    pub fn e_mut(&mut self) -> Option<&mut E> {
        self.typed_mut(EBIT, Self::OFF_E)
    }

    /// Makes `e` the live `E`, returning the one it replaces.
    #[inline]
    pub fn set_e(&mut self, e: E) -> Option<E> {
        self.put(EBIT, Self::OFF_E, e)
    }

    /// The live `E` moved out, its region zeroed.
    #[inline]
    pub fn take_e(&mut self) -> Option<E> {
        self.out(EBIT, Self::OFF_E)
    }

    /// Where the bytes [`Body::inline`] lends start in the storage: after the `P` while one is
    /// live ([`Body::INLINE_P`]), 0 otherwise.
    #[inline]
    pub fn inline_start(&self) -> usize {
        if self.flags.get() & PBIT != 0 {
            Self::INLINE_P
        } else {
            0
        }
    }

    /// The storage's bytes that hold no value, from [`Body::inline_start`] to the end; `None`
    /// while an `E` is live (the C uses no inline data beside `M_EXT`).
    #[inline]
    pub fn inline(&self) -> Option<&[u8]> {
        if self.flags.get() & EBIT != 0 {
            return None;
        }
        let start = self.inline_start();
        // SAFETY: `start..N` lies inside the storage (`INLINE_P <= N`, checked at compile time)
        // and holds no live value: the `E` is not live (checked above) and the `P` region ends
        // at or before `start` when its bit is set (`INLINE_P` rounds its size up). So the
        // bytes are initialised (the invariant: zeroed by `new` and by every `out`, or written
        // by the owner as bytes), and no `&mut` to them exists: `inline_mut`, `p_mut`,
        // `e_mut`, `set_*` and `take_*` all need `&mut self`, which this borrow of `self`
        // excludes for its lifetime. A live `P` may change through its own `Cell`s meanwhile,
        // in its own region only. What `as_raw`'s users write by address is theirs to keep
        // apart from this borrow (the owner's one-owner convention, `bsd`'s `unsafe`).
        Some(unsafe {
            slice::from_raw_parts(self.as_raw().wrapping_add(start).cast_const(), N - start)
        })
    }

    /// The bytes of [`Body::inline`], to write them.
    #[inline]
    pub fn inline_mut(&mut self) -> Option<&mut [u8]> {
        if self.flags.get() & EBIT != 0 {
            return None;
        }
        let start = self.inline_start();
        self.storage.get_mut().0.get_mut(start..)
    }

    /// The value of type `T` in the region at `off`, while `bit` is set. Called only with
    /// `(PBIT, 0)` for `P` and `(EBIT, OFF_E)` for `E`.
    #[inline]
    fn typed<T>(&self, bit: u16, off: usize) -> Option<&T> {
        if self.flags.get() & bit == 0 {
            return None;
        }
        // SAFETY: the two callers pass the pairs `(P, PBIT, 0)` and `(E, EBIT, OFF_E)`: the bit
        // is set, so the region at `off` holds an initialised `T` (the invariant), inside the
        // storage and aligned for it (the storage is aligned to 8, `off` is 0 or a multiple of
        // `align_of::<E>()`, both alignments at most 8: checked at compile time). The value
        // stays live while the reference does: it borrows `self`, and only `out` (through
        // `&mut self`) ends a value or clears its bit. No `&mut T` exists beside it
        // (`typed_mut` needs `&mut self`).
        Some(unsafe { &*self.as_raw().wrapping_add(off).cast::<T>() })
    }

    /// [`Body::typed`]'s value, to change it.
    #[inline]
    fn typed_mut<T>(&mut self, bit: u16, off: usize) -> Option<&mut T> {
        if self.flags.get() & bit == 0 {
            return None;
        }
        let at = self
            .storage
            .get_mut()
            .0
            .as_mut_ptr()
            .wrapping_add(off)
            .cast::<T>();
        // SAFETY: as for `typed`; the reference borrows `self` mutably, the only path to the
        // storage, so it is the only reference to the value while it lives.
        Some(unsafe { &mut *at })
    }

    /// Writes `v` into the region at `off` and sets `bit`, returning the value it replaces.
    /// Called only with `(PBIT, 0)` and `(EBIT, OFF_E)`.
    #[inline]
    fn put<T>(&mut self, bit: u16, off: usize, v: T) -> Option<T> {
        let old = self.out::<T>(bit, off);
        let at = self
            .storage
            .get_mut()
            .0
            .as_mut_ptr()
            .wrapping_add(off)
            .cast::<T>();
        // SAFETY: the region at `off` is inside the storage and aligned for `T` (as in
        // `typed`) and holds no live value now (`out` above ended the one there, if any, and
        // the other value's region is disjoint: `OFF_E` starts after the `P` region). `&mut
        // self` excludes every other reference to the storage. Writing a whole value
        // initialises the region; the bit set right after records it, so the invariant holds
        // again before any reference can be made.
        unsafe { at.write(v) };
        self.flags.set(self.flags.get() | bit);
        old
    }

    /// The value at `off` moved out and `bit` cleared, the region zeroed; `None` when `bit`
    /// is not set. Called only with `(PBIT, 0)` and `(EBIT, OFF_E)`.
    #[inline]
    fn out<T>(&mut self, bit: u16, off: usize) -> Option<T> {
        if self.flags.get() & bit == 0 {
            return None;
        }
        let at = self
            .storage
            .get_mut()
            .0
            .as_mut_ptr()
            .wrapping_add(off)
            .cast::<T>();
        // SAFETY: the bit is set, so the region holds an initialised `T` (the invariant),
        // aligned and inside the storage (as in `typed`); `&mut self` excludes every reference
        // to it. The read moves the value out, and clearing the bit at once makes the region
        // dead before anything can read it as a `T` again, so the value is not dropped twice.
        let v = unsafe { at.read() };
        self.flags.set(self.flags.get() & !bit);
        if let Some(region) = self.storage.get_mut().0.get_mut(off..off + size_of::<T>()) {
            region.fill(0);
        }
        Some(v)
    }
}

impl<P: 'static, E: 'static, X: 'static, const PBIT: u16, const EBIT: u16, const N: usize> Drop
    for Body<P, E, X, PBIT, EBIT, N>
{
    /// The live values are dropped with the overlay, the `P` first.
    fn drop(&mut self) {
        drop(self.take_p());
        drop(self.take_e());
    }
}

/// `N` bytes of a pool item (an mbuf cluster), aligned to 64 as the C's cluster pools are
/// (`m_pool_init(.., 64, ..)`), so `size_of` is the C's item size, `roundup(N, 64)`: the
/// `Size` column of `vmstat -m`. Lent by the borrow of whoever owns the item, and by address
/// to `bsd` ([`Bytes::as_raw`]).
#[repr(C, align(64))]
pub struct Bytes<const N: usize>(UnsafeCell<[u8; N]>);

impl<const N: usize> Bytes<N> {
    /// The bytes.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: the cell holds `N` bytes, all initialised (a `Bytes` is made only over a pool
        // item's RAM by `BoxPool::get_bytes`, every byte of which is a valid `u8`, and only
        // ever written as bytes), and safe code writes them only through `bytes_mut`, which
        // needs `&mut self`, excluded by this borrow for the slice's lifetime. What `as_raw`'s
        // users write by address is theirs to keep apart from this borrow (the owner's
        // one-owner convention, `bsd`'s `unsafe`).
        unsafe { &*self.0.get() }
    }

    /// The bytes, to write them.
    #[inline]
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.0.get_mut()
    }

    /// The first byte's address, for what names the bytes by address (`bsd`'s `mtod`, a
    /// driver's DMA map); never dereferenced here.
    #[inline]
    pub fn as_raw(&self) -> *mut u8 {
        self.0.get().cast::<u8>()
    }
}

// SAFETY: a shared `Bytes` gives only `&[u8]` (`bytes`) and a raw pointer (`as_raw`), which
// safe code cannot write through; every write by safe code needs `&mut Bytes`. So sharing one
// across CPUs shares a read-only byte array, as `[u8; N]` (which is `Sync`) would; the raw
// writes `bsd` makes are under its own `unsafe` and its one-owner convention, as in LZ, where
// a cluster was memory behind a raw pointer.
unsafe impl<const N: usize> Sync for Bytes<N> {}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;
    use std::vec::Vec;

    /// A packet header's stand-in: 80 bytes, its drops counted.
    #[repr(C)]
    struct P80 {
        words: [u64; 9],
        drops: Rc<Cell<u32>>,
    }

    impl Drop for P80 {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    /// A cluster description's stand-in: 24 bytes, its drops counted.
    #[repr(C)]
    struct E24 {
        words: [u64; 2],
        drops: Rc<Cell<u32>>,
    }

    impl Drop for E24 {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    /// The owner's six bytes (an mbuf's type and mark).
    #[repr(C)]
    struct X6 {
        kind: Cell<i16>,
        _r: [u8; 4],
    }

    type B = Body<P80, E24, X6, 0x0002, 0x0001, 224>;

    fn body() -> B {
        B::new(
            X6 {
                kind: Cell::new(1),
                _r: [0; 4],
            },
            0,
        )
    }

    fn p80(drops: &Rc<Cell<u32>>, v: u64) -> P80 {
        P80 {
            words: [v; 9],
            drops: drops.clone(),
        }
    }

    fn e24(drops: &Rc<Cell<u32>>, v: u64) -> E24 {
        E24 {
            words: [v; 2],
            drops: drops.clone(),
        }
    }

    #[test]
    fn the_overlay_is_eight_bytes_and_its_storage() {
        assert_eq!(size_of::<P80>(), 80);
        assert_eq!(size_of::<E24>(), 24);
        assert_eq!(size_of::<B>(), 232);
        assert_eq!(B::OFF_E, 80);
        assert_eq!(B::INLINE_P, 80);
        assert_eq!(size_of::<Bytes<2050>>(), 2112);
        assert_eq!(align_of::<Bytes<2048>>(), 64);
    }

    #[test]
    fn values_are_dropped_once_through_every_transition() {
        let dp = Rc::new(Cell::new(0));
        let de = Rc::new(Cell::new(0));
        let mut b = body();
        assert!(b.p().is_none() && b.e().is_none());
        assert!(b.set_p(p80(&dp, 7)).is_none());
        assert_eq!(b.p().map(|p| p.words[8]), Some(7));
        assert_eq!(b.flags() & 0x0002, 0x0002);
        let old = b.set_p(p80(&dp, 8)).expect("the value replaced");
        assert_eq!(old.words[0], 7);
        drop(old);
        assert_eq!(dp.get(), 1);
        assert!(b.set_e(e24(&de, 3)).is_none());
        if let Some(e) = b.e_mut() {
            e.words[1] = 4;
        }
        assert_eq!(b.e().map(|e| e.words[1]), Some(4));
        let p = b.take_p().expect("the header");
        assert_eq!((p.words[0], b.flags() & 0x0002), (8, 0));
        drop(p);
        assert_eq!(dp.get(), 2);
        assert!(b.take_p().is_none());
        assert!(b.set_p(p80(&dp, 9)).is_none());
        drop(b);
        assert_eq!(
            (dp.get(), de.get()),
            (3, 1),
            "the drop dropped both live values"
        );
    }

    #[test]
    fn inline_bytes_follow_the_live_values() {
        let d = Rc::new(Cell::new(0));
        let mut b = body();
        assert_eq!(b.inline().map(<[u8]>::len), Some(224));
        assert_eq!(b.inline_start(), 0);
        b.inline_mut().expect("the bytes")[100] = 0x5a;
        b.set_p(p80(&d, u64::MAX));
        assert_eq!(b.inline_start(), 80);
        let inl = b.inline().expect("the bytes after the header");
        assert_eq!((inl.len(), inl[20]), (144, 0x5a));
        b.set_e(e24(&d, 1));
        assert!(b.inline().is_none() && b.inline_mut().is_none());
        drop(b.take_e());
        drop(b.take_p());
        let all = b.inline().expect("every byte");
        assert!(
            all[..80].iter().all(|&x| x == 0),
            "the header's region zeroed"
        );
        assert!(
            all[80..104].iter().all(|&x| x == 0),
            "the description's region zeroed"
        );
        assert_eq!(all[100], 0, "the bytes under a value are the value's");
        assert_eq!(b.x().kind.get(), 1);
    }

    #[test]
    fn set_flags_cannot_flip_a_tag() {
        let d = Rc::new(Cell::new(0));
        let mut b = body();
        b.set_flags(0xffff);
        assert_eq!(b.flags(), 0xfffc, "no tag bit from set_flags");
        assert!(b.p().is_none() && b.e().is_none());
        b.set_p(p80(&d, 1));
        b.set_flags(0);
        assert_eq!(b.flags(), 0x0002, "the live header's bit kept");
        assert!(b.p().is_some());
        let c = B::new(
            X6 {
                kind: Cell::new(0),
                _r: [0; 4],
            },
            0x0003,
        );
        assert_eq!(c.flags(), 0, "new clears the tags");
    }

    #[test]
    fn bytes_lend_what_was_written() {
        let mut v: Vec<Bytes<64>> = Vec::new();
        v.push(Bytes(UnsafeCell::new([0; 64])));
        let b = &mut v[0];
        b.bytes_mut()[63] = 9;
        assert_eq!((b.bytes().len(), b.bytes()[63]), (64, 9));
        assert_eq!(b.as_raw().addr() % 64, 0);
    }

    #[test]
    fn a_byte_buffer_comes_from_its_pool_and_goes_back() {
        static BUFS: crate::pool::BoxPool<Bytes<4096>> = crate::pool::BoxPool::new();
        crate::testutil::install();
        let _ = BUFS.init(0, crate::pool::PR_WAITOK, "bytespl");
        let mut b = BUFS.get_bytes(crate::pool::PR_WAITOK).expect("a buffer");
        assert_eq!(BUFS.nout(), 1);
        assert_eq!(BUFS.item_size(), Some(4096));
        b.bytes_mut().fill(7);
        assert!(b.bytes().iter().all(|&x| x == 7));
        assert_eq!(b.as_raw().addr() % 64, 0);
        drop(b);
        assert_eq!(BUFS.nout(), 0);
    }
}
/* </TESTS> */
