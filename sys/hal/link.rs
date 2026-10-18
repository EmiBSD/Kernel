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
//! `Link<T>`: a settable reference to something that lives for ever.
//!
//! Not an OpenBSD file. The C links the objects that are never freed (the `vm_page` array and
//! the pmemranges stolen at boot, `uvm_page_init`) with plain pointers, `NULL` for none; LZ
//! spells them `Cell<*const T>` or `Cell<Option<NonNull<T>>>`, each read in its own `unsafe`
//! block whose comment says the target is still there. A `Link<T>` holds only a `&'static T`
//! (or nothing): [`Link::set`] takes the reference, so whatever it names outlives every
//! reader, and [`Link::get`] gives it back. That is the whole contract, and the one `unsafe`
//! of the type, in `get`, rests on it alone: no lock and no caller's promise is involved, so
//! the links of [`crate::list::List`] and [`crate::tree::Tree`], which are `Link`s, have safe
//! mutators.
//!
//! The pointer is an atomic (`Release` on store, `Acquire` on load), so a value built and then
//! published through a link is seen whole by a CPU that follows it. Which lock keeps a set of
//! links consistent (a list's, a tree's) is the owner's business: a race between two writers
//! can make a list wrong, never a reference dangle.
//!
//! ## Redesign
//! - `Cell<*const T>` with "valid while linked" -> `Link<T>` over `'static` targets: the
//!   validity is the type's, so it is checked by the compiler at `set`, not promised at every
//!   read. `Sync` by composition (an atomic pointer and a `PhantomData<&'static T>`), so a
//!   link may be shared exactly when `T` may.

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

/// A reference to a `T` that lives for ever, or none (`NULL`), settable through `&self`.
///
/// Invariant: `ptr` is null or the address of a `&'static T` given to [`Link::set`]; nothing
/// else ever writes it.
pub struct Link<T: 'static> {
    ptr: AtomicPtr<T>,
    /// The link stands for an `Option<&'static T>`: `Send` and `Sync` exactly when that is.
    _ref: PhantomData<&'static T>,
}

impl<T: 'static> Link<T> {
    /// No target (`NULL`).
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::null_mut()),
            _ref: PhantomData,
        }
    }

    /// A link to `to` from the start.
    pub const fn to(to: &'static T) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr::from_ref(to).cast_mut()),
            _ref: PhantomData,
        }
    }

    /// The target, `None` when the link is empty.
    pub fn get(&self) -> Option<&'static T> {
        let p = self.ptr.load(Ordering::Acquire);
        if p.is_null() {
            return None;
        }
        // SAFETY: by the type's invariant `p` is the address of a `&'static T` that `set` (or
        // `to`) was given, the only writers: it is aligned, points to a `T` valid for the
        // whole run of the kernel, and the shared reference made here is exactly the one
        // that was handed in, so it aliases only shared references. The `Acquire` load pairs
        // with the `Release` store of `set`, so the `T` as it was built before it was linked
        // is visible here.
        Some(unsafe { &*p })
    }

    /// Points the link at `to` (`None` empties it).
    pub fn set(&self, to: Option<&'static T>) {
        let p = to.map_or(ptr::null_mut(), |r| ptr::from_ref(r).cast_mut());
        self.ptr.store(p, Ordering::Release);
    }

    /// Whether the link is empty (`== NULL`).
    pub fn is_none(&self) -> bool {
        self.ptr.load(Ordering::Relaxed).is_null()
    }

    /// Whether the link names `x` (an address comparison, nothing followed).
    pub fn points_to(&self, x: &T) -> bool {
        ptr::eq(self.ptr.load(Ordering::Relaxed), ptr::from_ref(x))
    }
}

impl<T: 'static> Default for Link<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> core::fmt::Debug for Link<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Link({:p})", self.ptr.load(Ordering::Relaxed))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::thread;

    use super::*;

    static A: u32 = 7;
    static B: u32 = 9;

    #[test]
    fn empty_then_set_then_emptied() {
        let l = Link::<u32>::new();
        assert!(l.is_none());
        assert_eq!(l.get(), None);
        l.set(Some(&A));
        assert_eq!(l.get(), Some(&7));
        assert!(l.points_to(&A));
        assert!(!l.points_to(&B));
        l.set(Some(&B));
        assert_eq!(l.get().copied(), Some(9));
        l.set(None);
        assert!(l.is_none());
        assert_eq!(Link::to(&A).get(), Some(&7));
    }

    #[test]
    fn a_value_published_on_one_cpu_is_seen_whole_on_another() {
        static L: Link<[u64; 4]> = Link::new();
        let v: &'static [u64; 4] = Box::leak(Box::new([1, 2, 3, 4]));
        let t = thread::spawn(move || L.set(Some(v)));
        t.join().unwrap();
        assert_eq!(L.get(), Some(&[1, 2, 3, 4]));
    }
}
/* </TESTS> */
