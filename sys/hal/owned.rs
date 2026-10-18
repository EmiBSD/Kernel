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
 * Copyright 2002 Niels Provos <provos@citi.umich.edu>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 2016 David Gwynne <dlg@openbsd.org>
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
 * Copyright (c) 1991, 1993
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
 *	@(#)queue.h	8.5 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! `OwnedTree<A>` and `OwnedList<A>`: an intrusive red-black tree and tail queue that own
//! their elements, pool items freed explicitly; `SecondaryTree<S>` and `SecondaryRef<A>`:
//! links to elements another container owns, counted in the element.
//!
//! Upstream: sys/kern/subr_tree.c @ 3ce1f3f79392
//! Upstream: sys/sys/tree.h @ 3ce1f3f79392
//! Upstream: sys/sys/queue.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_tree.rs@3c62ede68802
//! LZ: sys/sys/tree.rs@3c62ede68802
//! LZ: sys/sys/queue.rs@3c62ede68802
//!
//! The behaviour of `RBT_INSERT`, `RBT_REMOVE`, `RBT_FIND`, `RBT_NFIND`, `RBT_MIN`/`MAX`,
//! `RBT_NEXT`/`PREV`, the `RBT_LEFT`/`RIGHT`/`PARENT`/`ROOT` walks and `RBT_AUGMENT` with
//! `RBT_GENERATE_AUGMENT` (`subr_tree.c`: the colourings of `rbe_insert_color` and
//! `rbe_remove_color`, `rbe_remove`'s replacement of a node with two children by its
//! successor, the rotations, and the augment callback at each place the C calls it), and of
//! the tail queue's `TAILQ_INSERT_TAIL`, `TAILQ_FIRST`, `TAILQ_REMOVE` of the head and the
//! `TAILQ_NEXT`/`PREV` walks, for elements that are pool items freed explicitly: uvm's map
//! entries (`struct vm_map_entry`), linked in their map's address tree (augmented with the
//! largest free space below each node), in the free tree of an address selector, and on a
//! dead queue that carries them out from under the map's lock (`uvm_map.c`, `uvm_addr.c`,
//! N3.5). [`crate::tree::Tree`] and [`crate::list::List`] take `&'static` elements and stay for
//! what is never freed (the pages, the pmemranges).
//!
//! **Ownership.** An owning container ([`OwnedTree`], [`OwnedList`]) takes an element as its
//! [`PoolBox`] ([`OwnedTree::insert`], [`OwnedList::push`]) and gives it back as one
//! ([`OwnedTree::take_by`], [`OwnedList::pop`]); [`OwnedTree::drain_into`] moves every element
//! of a tree onto a list in one pass (`uvm_map_teardown`). Meanwhile the container owns it:
//! its drop frees what it still holds, and every reference it hands out (`&T`, never `&mut
//! T`) borrows it, so no element can be freed or changed through a box while a reference is
//! alive. An element has one [`OwnedEntry`], shared by the tree and the list (the C's
//! `dfree` union: an entry is in the address tree or on a dead queue), and it records the
//! pool the element goes back to (`uvm_mapent_free`'s choice between the static entries,
//! `"vmmpekpl"` and `"vmmpepl"`).
//!
//! **Counted links.** A [`SecondaryTree`] (the selectors' free trees) and a [`SecondaryRef`]
//! (a pivot's entry, `uvm_addr.c`'s `up_pivot.entry`) reach elements an owning container
//! owns, through their own entry ([`SecondaryEntry`]) or pointer; each such link is counted in
//! the element's [`OwnedEntry`], and an owning container refuses to give an element back
//! (`take_by`, `pop`, its drop) while the count is not 0: the C's rule that an entry leaves
//! the free tree before the address tree (`uvm_mapent_free_remove` before
//! `uvm_mapent_addr_remove`) becomes a checked precondition.
//!
//! As with every lock-free structure of `hal`, the lock that keeps a container consistent is
//! its owner's: the containers change only through `&mut self` (the map's write lock, by the
//! guard it gives) and are read through `&self`.
//!
//! ## Soundness
//!
//! The derefs of this module (one for the owning containers, one for the counted links) and
//! the box rebuilt by an owning container rest on these invariants, which every function here
//! keeps:
//! - I1, ownership: an element whose [`PoolBox`] an owning container consumed is owned by that
//!   container until it gives a box back; a pool item has one box ([`PoolBox`]'s invariant),
//!   so it is owned by at most one container, and no box of it, hence no `&mut T` to it, exists
//!   meanwhile.
//! - I2, identity: every container has an identity drawn once from a global counter, never 0
//!   and never reused. An owned element has exactly one [`OwnedEntry`] that carries its
//!   owner's identity: the one its adapter named when it was linked. Every access checks that
//!   the entry the adapter names lies within the element's own bytes and carries the
//!   container's identity, and panics otherwise, so an adapter that names another element's
//!   entry, a static, or a different entry on different calls, is refused, never followed.
//! - I3, links: a container's head and the links of the entries carrying its identity name only
//!   elements it owns (a [`SecondaryTree`]: only elements some owning container owns, each
//!   counted once in that element's [`OwnedEntry`]; a [`SecondaryRef`]'s pointer likewise).
//! - I4, counting: an owning container gives no element back while its count is not 0; a
//!   secondary uncounts only what it unlinks. The count is read last after every call into
//!   an adapter of the removal (whose `entry` and `augment` see the element being removed and
//!   could link it from a secondary): only this module's code runs between that read and the
//!   box, and a late link panics with the container poisoned, the element never given back.
//! - I5, poisoning: a panic in the middle of a change (an adapter's `cmp` or `augment`, a
//!   refused entry, a broken shape) poisons the containers being changed, which then never
//!   read or change their links again (every method panics, the drop leaks).
//!
//! So a pointer read under I3 while the container is borrowed names a live element, which
//! does not move (a pool item) and which only shared references reach.
//!
//! ## Redesign
//! - `RBT_HEAD`/`RBT_ENTRY` and `TAILQ_HEAD`/`TAILQ_ENTRY` over pool items handed out as
//!   `&'static T` (LZ's `sys/sys/tree.rs`, `queue.rs`: every insert and remove an `unsafe`
//!   block whose comment says the element stays alive while linked, and `uvm_mapent_free`
//!   freeing an entry the caller still holds as `&'static`) -> containers that own the
//!   element's box: a use after free cannot be written.
//! - `RBT_REMOVE(name, head, elm)` with `elm` a pointer the caller found ->
//!   [`OwnedTree::take_by`], which finds and removes in one call (a reference that borrows the
//!   tree cannot be passed to a method that changes it); `RBT_FIND`/`RBT_NFIND` with an
//!   element built as the key -> [`OwnedTree::find_by`]/[`OwnedTree::nfind_by`].
//! - `RBT_GENERATE_AUGMENT(name, type, field, cmp, augment)`: the adapter's `augment`
//!   recomputes one node's summary from itself and its children and says whether it changed;
//!   wherever `subr_tree.c` calls the callback this module walks up from the node while the
//!   summary changes, which is what `uvm_map_addr_augment`, the C's only augment callback,
//!   does itself. [`OwnedTree::augment`] is `RBT_AUGMENT`.
//! - `uvm_map_teardown`'s breadth-first walk that queues every entry on the dead queue
//!   without unlinking it, then `RBT_INIT` -> [`OwnedTree::drain_into`]: the leaves are
//!   detached one by one, each moved onto the list, in linear time; the order is the walk's,
//!   not the C's breadth-first one (nothing outside the map sees the order the entries die in).
//! - The free trees' links (`dfree.rbtree`, in the same union as the dead queue's) are a
//!   [`SecondaryEntry`] of their own, so a counted link survives the move onto a dead list
//!   and is undone by the selector's own destruction ([`SecondaryTree`]'s drop).

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use core::sync::atomic::{
    AtomicBool, AtomicPtr, AtomicU8, AtomicU32, AtomicUsize, Ordering as Mem,
};

use crate::assert::hal_panic;
use crate::link::Link;
use crate::pool::{BoxPool, PoolBox};

/// `RB_BLACK`.
const BLACK: u8 = 0;
/// `RB_RED`.
const RED: u8 = 1;

/// Declares `$name`, the [`OwnedAdapter`] of `$elem`'s field `$field` ordered by `$cmp` (a
/// function or a closure without captures, `fn(&$elem, &$elem) -> Ordering`), augmented by
/// `$aug` (`fn(&$elem, Option<&$elem>, Option<&$elem>) -> bool`) when given:
/// `owned_adapter!(pub ByAddr: VmMapEntry, link, cmp_start, augment = augment_fspace);`.
#[macro_export]
macro_rules! owned_adapter {
    ($(#[$meta:meta])* $vis:vis $name:ident: $elem:ty, $field:ident, $cmp:expr, augment = $aug:expr) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::owned::OwnedAdapter for $name {
            type Elem = $elem;
            const AUGMENTED: bool = true;
            fn entry(e: &$elem) -> &$crate::owned::OwnedEntry<$elem> {
                &e.$field
            }
            fn cmp(a: &$elem, b: &$elem) -> ::core::cmp::Ordering {
                let f: fn(&$elem, &$elem) -> ::core::cmp::Ordering = $cmp;
                f(a, b)
            }
            fn augment(e: &$elem, l: ::core::option::Option<&$elem>, r: ::core::option::Option<&$elem>) -> bool {
                let f: fn(&$elem, ::core::option::Option<&$elem>, ::core::option::Option<&$elem>) -> bool = $aug;
                f(e, l, r)
            }
        }
    };
    ($(#[$meta:meta])* $vis:vis $name:ident: $elem:ty, $field:ident, $cmp:expr) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::owned::OwnedAdapter for $name {
            type Elem = $elem;
            fn entry(e: &$elem) -> &$crate::owned::OwnedEntry<$elem> {
                &e.$field
            }
            fn cmp(a: &$elem, b: &$elem) -> ::core::cmp::Ordering {
                let f: fn(&$elem, &$elem) -> ::core::cmp::Ordering = $cmp;
                f(a, b)
            }
        }
    };
}

/// Declares `$name`, the [`SecondaryAdapter`] of the field `$field` of the elements of the
/// owning adapter `$owner`, ordered by `$cmp`:
/// `secondary_adapter!(pub ByFspace: ByAddr, free, cmp_fspace);`.
#[macro_export]
macro_rules! secondary_adapter {
    ($(#[$meta:meta])* $vis:vis $name:ident: $owner:ty, $field:ident, $cmp:expr) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::owned::SecondaryAdapter for $name {
            type Owner = $owner;
            fn entry(
                e: &$crate::owned::Elem<Self>,
            ) -> &$crate::owned::SecondaryEntry<$crate::owned::Elem<Self>> {
                &e.$field
            }
            fn cmp(a: &$crate::owned::Elem<Self>, b: &$crate::owned::Elem<Self>) -> ::core::cmp::Ordering {
                let f: fn(&$crate::owned::Elem<Self>, &$crate::owned::Elem<Self>) -> ::core::cmp::Ordering = $cmp;
                f(a, b)
            }
        }
    };
}

/// The element type of a [`SecondaryAdapter`]: its owner adapter's.
pub type Elem<S> = <<S as SecondaryAdapter>::Owner as OwnedAdapter>::Elem;

/// An element's links in its owning container (`RBT_ENTRY` of the tree, or `TAILQ_ENTRY` of
/// the list: `left` is the previous element there and `right` the next), the identity of the
/// container that owns it, the pool it goes back to, and the count of the links to it from
/// secondaries.
pub struct OwnedEntry<T: 'static> {
    left: AtomicPtr<T>,
    right: AtomicPtr<T>,
    parent: AtomicPtr<T>,
    /// The identity of the owning container, 0 when the element is in none.
    owner: AtomicUsize,
    /// The pool the element goes back to, while it is owned.
    pool: Link<BoxPool<T>>,
    /// The links to the element from secondaries ([`SecondaryTree`], [`SecondaryRef`]).
    indexed: AtomicU32,
    color: AtomicU8,
}

impl<T: 'static> OwnedEntry<T> {
    /// An entry in no container.
    pub const fn new() -> Self {
        Self {
            left: AtomicPtr::new(ptr::null_mut()),
            right: AtomicPtr::new(ptr::null_mut()),
            parent: AtomicPtr::new(ptr::null_mut()),
            owner: AtomicUsize::new(0),
            pool: Link::new(),
            indexed: AtomicU32::new(0),
            color: AtomicU8::new(BLACK),
        }
    }

    /// Whether the element is owned by a container through this entry.
    pub fn is_linked(&self) -> bool {
        self.owner.load(Mem::Relaxed) != 0
    }

    /// The links to the element from secondaries now (a snapshot, for assertions).
    pub fn indexed(&self) -> u32 {
        self.indexed.load(Mem::Relaxed)
    }

    #[inline]
    fn view(&self) -> View<'_, T> {
        View {
            left: &self.left,
            right: &self.right,
            parent: &self.parent,
            color: &self.color,
        }
    }

    /// No links, no owner, no pool: the entry of an element given back.
    fn clear(&self) {
        put(&self.left, None);
        put(&self.right, None);
        put(&self.parent, None);
        self.color.store(BLACK, Mem::Relaxed);
        self.owner.store(0, Mem::Relaxed);
        self.pool.set(None);
    }

    /// One more counted link; a count that would wrap panics before it changes.
    fn count_up(&self) {
        if self
            .indexed
            .fetch_update(Mem::Relaxed, Mem::Relaxed, |n| n.checked_add(1))
            .is_err()
        {
            hal_panic(format_args!("OwnedEntry: too many secondary links"));
        }
    }

    /// One counted link fewer; a count already 0 is a broken secondary, `panic`.
    fn count_down(&self) {
        if self
            .indexed
            .fetch_update(Mem::Relaxed, Mem::Relaxed, |n| n.checked_sub(1))
            .is_err()
        {
            hal_panic(format_args!(
                "OwnedEntry: a secondary link that was not counted"
            ));
        }
    }
}

impl<T: 'static> Default for OwnedEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// An element's links in one secondary tree (`RBT_ENTRY`: the selectors' `dfree.rbtree`) and
/// the identity of the tree it is in.
pub struct SecondaryEntry<T: 'static> {
    left: AtomicPtr<T>,
    right: AtomicPtr<T>,
    parent: AtomicPtr<T>,
    /// The identity of the secondary tree, 0 when the element is in none.
    owner: AtomicUsize,
    color: AtomicU8,
}

impl<T: 'static> SecondaryEntry<T> {
    /// An entry in no tree.
    pub const fn new() -> Self {
        Self {
            left: AtomicPtr::new(ptr::null_mut()),
            right: AtomicPtr::new(ptr::null_mut()),
            parent: AtomicPtr::new(ptr::null_mut()),
            owner: AtomicUsize::new(0),
            color: AtomicU8::new(BLACK),
        }
    }

    /// Whether the element is in a secondary tree through this entry.
    pub fn is_linked(&self) -> bool {
        self.owner.load(Mem::Relaxed) != 0
    }

    #[inline]
    fn view(&self) -> View<'_, T> {
        View {
            left: &self.left,
            right: &self.right,
            parent: &self.parent,
            color: &self.color,
        }
    }

    fn clear(&self) {
        put(&self.left, None);
        put(&self.right, None);
        put(&self.parent, None);
        self.color.store(BLACK, Mem::Relaxed);
        self.owner.store(0, Mem::Relaxed);
    }
}

impl<T: 'static> Default for SecondaryEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The links of one node as the red-black engine sees them.
#[derive(Clone, Copy)]
struct View<'a, T: 'static> {
    left: &'a AtomicPtr<T>,
    right: &'a AtomicPtr<T>,
    parent: &'a AtomicPtr<T>,
    color: &'a AtomicU8,
}

impl<T: 'static> View<'_, T> {
    #[inline]
    fn left(&self) -> Option<NonNull<T>> {
        get(self.left)
    }

    #[inline]
    fn right(&self) -> Option<NonNull<T>> {
        get(self.right)
    }

    #[inline]
    fn parent(&self) -> Option<NonNull<T>> {
        get(self.parent)
    }

    #[inline]
    fn color(&self) -> u8 {
        self.color.load(Mem::Relaxed)
    }

    #[inline]
    fn set_color(&self, c: u8) {
        self.color.store(c, Mem::Relaxed);
    }
}

/// A red-black tree of `A::Elem`s that owns them (`RBT_HEAD` of uvm's address tree).
pub struct OwnedTree<A: OwnedAdapter> {
    root: AtomicPtr<A::Elem>,
    /// The tree's identity (I2), drawn at the first insertion; 0 before.
    id: usize,
    /// I5.
    poisoned: AtomicBool,
    /// The tree owns the elements' boxes: `Send` and `Sync` as they are.
    _owns: PhantomData<PoolBox<A::Elem>>,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: OwnedAdapter> OwnedTree<A> {
    /// `RBT_INITIALIZER`: an empty tree.
    pub const fn new() -> Self {
        Self {
            root: AtomicPtr::new(ptr::null_mut()),
            id: 0,
            poisoned: AtomicBool::new(false),
            _owns: PhantomData,
            _adapter: PhantomData,
        }
    }

    /// The element `p` names (a pointer of this tree, I3).
    #[inline]
    fn node(&self, p: NonNull<A::Elem>) -> &A::Elem {
        owned_ref(self, p)
    }

    /// `e`'s entry, checked to be this tree's (I2); `panic` otherwise.
    #[inline]
    fn ent<'e>(&self, e: &'e A::Elem, what: &str) -> &'e OwnedEntry<A::Elem> {
        let ent = entry_of::<A>(e);
        if self.id == 0 || ent.owner.load(Mem::Relaxed) != self.id {
            not_in_tree(what);
        }
        ent
    }

    #[inline]
    fn check(&self, what: &str) {
        if self.poisoned.load(Mem::Relaxed) {
            poisoned(what);
        }
    }

    /// `RBT_EMPTY`.
    pub fn is_empty(&self) -> bool {
        self.check("RBT_EMPTY");
        get(&self.root).is_none()
    }

    /// `RBT_ROOT`.
    pub fn root(&self) -> Option<&A::Elem> {
        self.check("RBT_ROOT");
        get(&self.root).map(|p| self.node(p))
    }

    /// Whether `e` is in this tree.
    pub fn contains(&self, e: &A::Elem) -> bool {
        self.check("OwnedTree::contains");
        self.id != 0 && entry_of::<A>(e).owner.load(Mem::Relaxed) == self.id
    }

    /// `RBT_LEFT(e)`; `e` must be in this tree.
    pub fn left(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("RBT_LEFT");
        get(&self.ent(e, "RBT_LEFT").left).map(|p| self.node(p))
    }

    /// `RBT_RIGHT(e)`; `e` must be in this tree.
    pub fn right(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("RBT_RIGHT");
        get(&self.ent(e, "RBT_RIGHT").right).map(|p| self.node(p))
    }

    /// `RBT_PARENT(e)`; `e` must be in this tree.
    pub fn parent(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("RBT_PARENT");
        get(&self.ent(e, "RBT_PARENT").parent).map(|p| self.node(p))
    }

    /// `RBT_MIN`: the first element in order.
    pub fn min(&self) -> Option<&A::Elem> {
        self.check("RBT_MIN");
        rb_min(self, get(&self.root)?).map(|p| self.node(p))
    }

    /// `RBT_MAX`: the last element in order.
    pub fn max(&self) -> Option<&A::Elem> {
        self.check("RBT_MAX");
        rb_max(self, get(&self.root)?).map(|p| self.node(p))
    }

    /// `RBT_NEXT(e)`: the element after `e` in order; `e` must be in this tree.
    pub fn next(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("RBT_NEXT");
        let v = self.ent(e, "RBT_NEXT").view();
        rb_next_from(self, NonNull::from(e), v).map(|p| self.node(p))
    }

    /// `RBT_PREV(e)`: the element before `e` in order; `e` must be in this tree.
    pub fn prev(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("RBT_PREV");
        let v = self.ent(e, "RBT_PREV").view();
        rb_prev_from(self, NonNull::from(e), v).map(|p| self.node(p))
    }

    /// `RBT_FIND(key)`: the element equal to `key` in the tree's order.
    pub fn find(&self, key: &A::Elem) -> Option<&A::Elem> {
        self.find_by(|n| A::cmp(key, n))
    }

    /// `RBT_NFIND(key)`: the first element not before `key`.
    pub fn nfind(&self, key: &A::Elem) -> Option<&A::Elem> {
        self.nfind_by(|n| A::cmp(key, n))
    }

    /// `RBT_FIND` without a key element: `key_vs(n)` is the key compared with node `n` (the
    /// C's `cmp(key, n)`).
    pub fn find_by(&self, key_vs: impl FnMut(&A::Elem) -> Ordering) -> Option<&A::Elem> {
        self.check("RBT_FIND");
        self.search(key_vs).map(|p| self.node(p))
    }

    /// `RBT_NFIND` without a key element (see [`OwnedTree::find_by`]).
    pub fn nfind_by(&self, mut key_vs: impl FnMut(&A::Elem) -> Ordering) -> Option<&A::Elem> {
        self.check("RBT_NFIND");
        let mut n = get(&self.root);
        let mut res = None;
        while let Some(x) = n {
            let v = self.view(x);
            n = match key_vs(self.node(x)) {
                Ordering::Less => {
                    res = Some(x);
                    v.left()
                }
                Ordering::Greater => v.right(),
                Ordering::Equal => return Some(self.node(x)),
            };
        }
        res.map(|p| self.node(p))
    }

    /// The node equal to the key, by `key_vs(n)`.
    fn search(&self, mut key_vs: impl FnMut(&A::Elem) -> Ordering) -> Option<NonNull<A::Elem>> {
        let mut n = get(&self.root);
        while let Some(x) = n {
            let v = self.view(x);
            n = match key_vs(self.node(x)) {
                Ordering::Less => v.left(),
                Ordering::Greater => v.right(),
                Ordering::Equal => return Some(x),
            };
        }
        None
    }

    /// The elements in order (`RBT_FOREACH`).
    pub fn iter(&self) -> Iter<'_, A> {
        self.check("RBT_FOREACH");
        Iter {
            tree: self,
            next: get(&self.root).and_then(|r| rb_min(self, r)),
        }
    }

    /// `RBT_INSERT(e)`: puts `e` in the tree and returns `None`, or gives `e` back when an
    /// element that compares equal is there (the C returns that element: [`OwnedTree::find`]
    /// of the box gives it).
    pub fn insert(&mut self, e: PoolBox<A::Elem>) -> Option<PoolBox<A::Elem>> {
        self.check("RBT_INSERT");
        let ent = entry_of::<A>(&e);
        if ent.is_linked() || ent.indexed() != 0 {
            hal_panic(format_args!(
                "RBT_INSERT: the element is in a container already"
            ));
        }
        // The search, before anything changes (the adapter's `cmp` may panic).
        let mut parent = None;
        let mut went = Ordering::Equal;
        let mut n = get(&self.root);
        while let Some(x) = n {
            parent = Some(x);
            went = A::cmp(&e, self.node(x));
            n = match went {
                Ordering::Less => self.view(x).left(),
                Ordering::Greater => self.view(x).right(),
                Ordering::Equal => return Some(e),
            };
        }
        if self.id == 0 {
            self.id = fresh_id();
        }
        let id = self.id;
        let (p, pool) = PoolBox::into_raw(e);
        let changing = Mutating(&self.poisoned);
        // The element is this tree's from here (I1): the box is consumed.
        let ent = entry_of::<A>(self.node(p));
        if ent.is_linked() {
            hal_panic(format_args!(
                "RBT_INSERT: the element is in a container already"
            ));
        }
        ent.owner.store(id, Mem::Relaxed);
        ent.pool.set(Some(pool));
        rb_link(self, p, parent, went);
        core::mem::forget(changing);
        None
    }

    /// `RBT_REMOVE` of the element equal to the key (`key_vs`, as [`OwnedTree::find_by`]):
    /// the element's box, `None` when no element is equal. Panics while a secondary links the
    /// element (I4: it must leave the free tree first).
    pub fn take_by(
        &mut self,
        key_vs: impl FnMut(&A::Elem) -> Ordering,
    ) -> Option<PoolBox<A::Elem>> {
        self.check("RBT_REMOVE");
        let x = self.search(key_vs)?;
        let ent = self.ent(self.node(x), "RBT_REMOVE");
        if ent.indexed() != 0 {
            hal_panic(format_args!(
                "RBT_REMOVE: the element is still linked from a secondary tree"
            ));
        }
        let changing = Mutating(&self.poisoned);
        rb_remove(self, x);
        // The removal ran the adapter's code (`entry`, and `augment` on the element itself:
        // `rbe_if_augment(parent)` with the successor `old`'s right child), which may have
        // linked the element from a secondary meanwhile. Looked at again with nothing run in
        // between from here to the box: a late link panics with the tree poisoned (I5), the
        // element never given back, so the link it holds stays valid (I4).
        if ent.indexed() != 0 {
            hal_panic(format_args!(
                "RBT_REMOVE: the element was linked from a secondary while being removed"
            ));
        }
        let pool = ent.pool.get();
        ent.clear();
        core::mem::forget(changing);
        let Some(pool) = pool else {
            hal_panic(format_args!(
                "RBT_REMOVE: an owned element without its pool"
            ));
        };
        Some(reclaim(x, pool))
    }

    /// `RBT_AUGMENT(e)`: `e`'s summary recomputed after its own value changed, and its
    /// ancestors' while theirs change; `e` must be in this tree. A tree that is not augmented
    /// does nothing.
    pub fn augment(&self, e: &A::Elem) {
        self.check("RBT_AUGMENT");
        self.ent(e, "RBT_AUGMENT");
        if A::AUGMENTED {
            let changing = Mutating(&self.poisoned);
            self.augment_walk(NonNull::from(e));
            core::mem::forget(changing);
        }
    }

    /// Moves every element of the tree onto the tail of `dead`, in linear time, without
    /// rebalancing (`uvm_map_teardown`'s walk and `RBT_INIT`): the tree is empty afterwards.
    /// Counted links stay counted: the elements are still owned, by the list.
    pub fn drain_into(&mut self, dead: &mut OwnedList<A>) {
        self.check("OwnedTree::drain_into");
        dead.check("OwnedTree::drain_into");
        if get(&self.root).is_none() {
            return;
        }
        if dead.id == 0 {
            dead.id = fresh_id();
        }
        let changing = Mutating(&self.poisoned);
        let changing_dead = Mutating(&dead.poisoned);
        let mut cur = get(&self.root);
        put(&self.root, None);
        while let Some(x) = cur {
            let v = self.view(x);
            if let Some(l) = v.left() {
                cur = Some(l);
                continue;
            }
            if let Some(r) = v.right() {
                cur = Some(r);
                continue;
            }
            // A leaf: detached from its parent, then moved onto the list.
            let p = v.parent();
            if let Some(p) = p {
                let pv = self.view(p);
                if pv.left() == Some(x) {
                    put(pv.left, None);
                } else {
                    put(pv.right, None);
                }
            }
            let ent = self.ent(self.node(x), "OwnedTree::drain_into");
            put(&ent.parent, None);
            ent.color.store(BLACK, Mem::Relaxed);
            ent.owner.store(dead.id, Mem::Relaxed);
            dead.link_tail(x);
            cur = p;
        }
        core::mem::forget(changing_dead);
        core::mem::forget(changing);
    }

    /// `rbe_augment` as uvm's callback does it: `x`'s summary, then its ancestors' while they
    /// change.
    fn augment_walk(&self, x: NonNull<A::Elem>) {
        let mut x = x;
        loop {
            let e = self.node(x);
            let v = self.view(x);
            let l = v.left().map(|p| self.node(p));
            let r = v.right().map(|p| self.node(p));
            if !A::augment(e, l, r) {
                return;
            }
            match v.parent() {
                Some(p) => x = p,
                None => return,
            }
        }
    }
}

impl<A: OwnedAdapter> Default for OwnedTree<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: OwnedAdapter> Drop for OwnedTree<A> {
    /// Frees every element still in the tree; panics on one a secondary still links (I4),
    /// which is then never freed. A poisoned tree leaks what it holds (I5).
    fn drop(&mut self) {
        if self.poisoned.load(Mem::Relaxed) || get(&self.root).is_none() {
            return;
        }
        let mut dead = OwnedList::new();
        self.drain_into(&mut dead);
    }
}

/// The iterator of [`OwnedTree::iter`].
pub struct Iter<'s, A: OwnedAdapter> {
    tree: &'s OwnedTree<A>,
    next: Option<NonNull<A::Elem>>,
}

impl<'s, A: OwnedAdapter> Iterator for Iter<'s, A> {
    type Item = &'s A::Elem;

    fn next(&mut self) -> Option<&'s A::Elem> {
        let x = self.next?;
        self.next = rb_next(self.tree, x);
        Some(self.tree.node(x))
    }
}

/// A tail queue of `A::Elem`s that owns them (`TAILQ_HEAD` of uvm's dead queue): `push` at
/// the tail, `pop` at the head, over the same [`OwnedEntry`] as the tree.
pub struct OwnedList<A: OwnedAdapter> {
    head: AtomicPtr<A::Elem>,
    tail: AtomicPtr<A::Elem>,
    /// The list's identity (I2), drawn at the first element; 0 before.
    id: usize,
    /// I5.
    poisoned: AtomicBool,
    /// The list owns the elements' boxes: `Send` and `Sync` as they are.
    _owns: PhantomData<PoolBox<A::Elem>>,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: OwnedAdapter> OwnedList<A> {
    /// `TAILQ_HEAD_INITIALIZER`: an empty list.
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(ptr::null_mut()),
            tail: AtomicPtr::new(ptr::null_mut()),
            id: 0,
            poisoned: AtomicBool::new(false),
            _owns: PhantomData,
            _adapter: PhantomData,
        }
    }

    fn node(&self, p: NonNull<A::Elem>) -> &A::Elem {
        owned_ref(self, p)
    }

    /// `e`'s entry, checked to be this list's (I2); `panic` otherwise.
    fn ent<'e>(&self, e: &'e A::Elem, what: &str) -> &'e OwnedEntry<A::Elem> {
        let ent = entry_of::<A>(e);
        if self.id == 0 || ent.owner.load(Mem::Relaxed) != self.id {
            hal_panic(format_args!("{what}: the element is not on this list"));
        }
        ent
    }

    fn check(&self, what: &str) {
        if self.poisoned.load(Mem::Relaxed) {
            poisoned(what);
        }
    }

    /// `TAILQ_EMPTY`.
    pub fn is_empty(&self) -> bool {
        self.check("TAILQ_EMPTY");
        get(&self.head).is_none()
    }

    /// `TAILQ_FIRST`.
    pub fn first(&self) -> Option<&A::Elem> {
        self.check("TAILQ_FIRST");
        get(&self.head).map(|p| self.node(p))
    }

    /// `TAILQ_LAST`.
    pub fn last(&self) -> Option<&A::Elem> {
        self.check("TAILQ_LAST");
        get(&self.tail).map(|p| self.node(p))
    }

    /// `TAILQ_NEXT(e)`; `e` must be on this list.
    pub fn next(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("TAILQ_NEXT");
        get(&self.ent(e, "TAILQ_NEXT").right).map(|p| self.node(p))
    }

    /// `TAILQ_PREV(e)`; `e` must be on this list.
    pub fn prev(&self, e: &A::Elem) -> Option<&A::Elem> {
        self.check("TAILQ_PREV");
        get(&self.ent(e, "TAILQ_PREV").left).map(|p| self.node(p))
    }

    /// Whether `e` is on this list.
    pub fn contains(&self, e: &A::Elem) -> bool {
        self.check("OwnedList::contains");
        self.id != 0 && entry_of::<A>(e).owner.load(Mem::Relaxed) == self.id
    }

    /// The elements from the head (`TAILQ_FOREACH`).
    pub fn iter(&self) -> ListIter<'_, A> {
        self.check("TAILQ_FOREACH");
        ListIter {
            list: self,
            next: get(&self.head),
        }
    }

    /// `TAILQ_INSERT_TAIL(e)` (`DEAD_ENTRY_PUSH`).
    pub fn push(&mut self, e: PoolBox<A::Elem>) {
        self.check("TAILQ_INSERT_TAIL");
        let ent = entry_of::<A>(&e);
        if ent.is_linked() || ent.indexed() != 0 {
            hal_panic(format_args!(
                "TAILQ_INSERT_TAIL: the element is in a container already"
            ));
        }
        if self.id == 0 {
            self.id = fresh_id();
        }
        let id = self.id;
        let (p, pool) = PoolBox::into_raw(e);
        let changing = Mutating(&self.poisoned);
        // The element is this list's from here (I1): the box is consumed.
        let ent = entry_of::<A>(self.node(p));
        if ent.is_linked() {
            hal_panic(format_args!(
                "TAILQ_INSERT_TAIL: the element is in a container already"
            ));
        }
        ent.owner.store(id, Mem::Relaxed);
        ent.pool.set(Some(pool));
        self.link_tail(p);
        core::mem::forget(changing);
    }

    /// `TAILQ_FIRST` and `TAILQ_REMOVE` of it: the head's box, `None` when the list is empty.
    /// Panics while a secondary links the element (I4).
    pub fn pop(&mut self) -> Option<PoolBox<A::Elem>> {
        self.check("TAILQ_REMOVE");
        let x = get(&self.head)?;
        let ent = self.ent(self.node(x), "TAILQ_REMOVE");
        if ent.indexed() != 0 {
            hal_panic(format_args!(
                "TAILQ_REMOVE: the element is still linked from a secondary tree"
            ));
        }
        let changing = Mutating(&self.poisoned);
        let next = get(&ent.right);
        put(&self.head, next);
        match next {
            Some(n) => put(&self.ent(self.node(n), "TAILQ_REMOVE").left, None),
            None => put(&self.tail, None),
        }
        // The adapter's `entry` ran on the next element: looked at again with nothing run in
        // between from here to the box (as `OwnedTree::take_by`).
        if ent.indexed() != 0 {
            hal_panic(format_args!(
                "TAILQ_REMOVE: the element was linked from a secondary while being removed"
            ));
        }
        let pool = ent.pool.get();
        ent.clear();
        core::mem::forget(changing);
        let Some(pool) = pool else {
            hal_panic(format_args!(
                "TAILQ_REMOVE: an owned element without its pool"
            ));
        };
        Some(reclaim(x, pool))
    }

    /// Links `x`, whose entry carries this list's identity already, at the tail.
    fn link_tail(&self, x: NonNull<A::Elem>) {
        let ent = self.ent(self.node(x), "TAILQ_INSERT_TAIL");
        let tail = get(&self.tail);
        put(&ent.left, tail);
        put(&ent.right, None);
        match tail {
            Some(t) => put(&self.ent(self.node(t), "TAILQ_INSERT_TAIL").right, Some(x)),
            None => put(&self.head, Some(x)),
        }
        put(&self.tail, Some(x));
    }
}

impl<A: OwnedAdapter> Default for OwnedList<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: OwnedAdapter> Drop for OwnedList<A> {
    /// Frees every element still on the list, from the head; panics on one a secondary still
    /// links (I4), which is then never freed, nor are those after it. A poisoned list leaks
    /// what it holds (I5).
    fn drop(&mut self) {
        if self.poisoned.load(Mem::Relaxed) {
            return;
        }
        while let Some(b) = self.pop() {
            drop(b);
        }
    }
}

/// The iterator of [`OwnedList::iter`].
pub struct ListIter<'s, A: OwnedAdapter> {
    list: &'s OwnedList<A>,
    next: Option<NonNull<A::Elem>>,
}

impl<'s, A: OwnedAdapter> Iterator for ListIter<'s, A> {
    type Item = &'s A::Elem;

    fn next(&mut self) -> Option<&'s A::Elem> {
        let x = self.next?;
        let e = self.list.node(x);
        self.next = get(&self.list.ent(e, "TAILQ_FOREACH").right);
        Some(e)
    }
}

/// A red-black tree of elements that owning containers own (`uaddr_free_rbtree`: a
/// selector's free tree): it links them through their [`SecondaryEntry`] and counts each link
/// in their [`OwnedEntry`], so their owner keeps them while they are here (I4).
pub struct SecondaryTree<S: SecondaryAdapter> {
    root: AtomicPtr<Elem<S>>,
    /// The tree's identity (I2), drawn at the first insertion; 0 before.
    id: usize,
    /// I5.
    poisoned: AtomicBool,
    _adapter: PhantomData<fn() -> S>,
}

impl<S: SecondaryAdapter> SecondaryTree<S> {
    /// `RBT_INITIALIZER`: an empty tree.
    pub const fn new() -> Self {
        Self {
            root: AtomicPtr::new(ptr::null_mut()),
            id: 0,
            poisoned: AtomicBool::new(false),
            _adapter: PhantomData,
        }
    }

    #[inline]
    fn node(&self, p: NonNull<Elem<S>>) -> &Elem<S> {
        counted_ref(self, p)
    }

    /// `e`'s secondary entry, checked to be this tree's (I2); `panic` otherwise.
    #[inline]
    fn ent<'e>(&self, e: &'e Elem<S>, what: &str) -> &'e SecondaryEntry<Elem<S>> {
        let ent = secondary_entry_of::<S>(e);
        if self.id == 0 || ent.owner.load(Mem::Relaxed) != self.id {
            not_in_tree(what);
        }
        ent
    }

    #[inline]
    fn check(&self, what: &str) {
        if self.poisoned.load(Mem::Relaxed) {
            poisoned(what);
        }
    }

    /// `RBT_EMPTY`.
    pub fn is_empty(&self) -> bool {
        self.check("RBT_EMPTY");
        get(&self.root).is_none()
    }

    /// `RBT_ROOT`.
    pub fn root(&self) -> Option<&Elem<S>> {
        self.check("RBT_ROOT");
        get(&self.root).map(|p| self.node(p))
    }

    /// Whether `e` is in this tree.
    pub fn contains(&self, e: &Elem<S>) -> bool {
        self.check("SecondaryTree::contains");
        self.id != 0 && secondary_entry_of::<S>(e).owner.load(Mem::Relaxed) == self.id
    }

    /// `RBT_LEFT(e)`; `e` must be in this tree.
    pub fn left(&self, e: &Elem<S>) -> Option<&Elem<S>> {
        self.check("RBT_LEFT");
        get(&self.ent(e, "RBT_LEFT").left).map(|p| self.node(p))
    }

    /// `RBT_RIGHT(e)`; `e` must be in this tree.
    pub fn right(&self, e: &Elem<S>) -> Option<&Elem<S>> {
        self.check("RBT_RIGHT");
        get(&self.ent(e, "RBT_RIGHT").right).map(|p| self.node(p))
    }

    /// `RBT_PARENT(e)`; `e` must be in this tree.
    pub fn parent(&self, e: &Elem<S>) -> Option<&Elem<S>> {
        self.check("RBT_PARENT");
        get(&self.ent(e, "RBT_PARENT").parent).map(|p| self.node(p))
    }

    /// `RBT_MIN`.
    pub fn min(&self) -> Option<&Elem<S>> {
        self.check("RBT_MIN");
        rb_min(self, get(&self.root)?).map(|p| self.node(p))
    }

    /// `RBT_MAX`.
    pub fn max(&self) -> Option<&Elem<S>> {
        self.check("RBT_MAX");
        rb_max(self, get(&self.root)?).map(|p| self.node(p))
    }

    /// `RBT_NEXT(e)`; `e` must be in this tree.
    pub fn next(&self, e: &Elem<S>) -> Option<&Elem<S>> {
        self.check("RBT_NEXT");
        let v = self.ent(e, "RBT_NEXT").view();
        rb_next_from(self, NonNull::from(e), v).map(|p| self.node(p))
    }

    /// `RBT_PREV(e)`; `e` must be in this tree.
    pub fn prev(&self, e: &Elem<S>) -> Option<&Elem<S>> {
        self.check("RBT_PREV");
        let v = self.ent(e, "RBT_PREV").view();
        rb_prev_from(self, NonNull::from(e), v).map(|p| self.node(p))
    }

    /// `RBT_FIND` without a key element (see [`OwnedTree::find_by`]).
    pub fn find_by(&self, mut key_vs: impl FnMut(&Elem<S>) -> Ordering) -> Option<&Elem<S>> {
        self.check("RBT_FIND");
        let mut n = get(&self.root);
        while let Some(x) = n {
            let v = self.view(x);
            n = match key_vs(self.node(x)) {
                Ordering::Less => v.left(),
                Ordering::Greater => v.right(),
                Ordering::Equal => return Some(self.node(x)),
            };
        }
        None
    }

    /// `RBT_NFIND` without a key element (see [`OwnedTree::find_by`]).
    pub fn nfind_by(&self, mut key_vs: impl FnMut(&Elem<S>) -> Ordering) -> Option<&Elem<S>> {
        self.check("RBT_NFIND");
        let mut n = get(&self.root);
        let mut res = None;
        while let Some(x) = n {
            let v = self.view(x);
            n = match key_vs(self.node(x)) {
                Ordering::Less => {
                    res = Some(x);
                    v.left()
                }
                Ordering::Greater => v.right(),
                Ordering::Equal => return Some(self.node(x)),
            };
        }
        res.map(|p| self.node(p))
    }

    /// The elements in order (`RBT_FOREACH`).
    pub fn iter(&self) -> SecondaryIter<'_, S> {
        self.check("RBT_FOREACH");
        SecondaryIter {
            tree: self,
            next: get(&self.root).and_then(|r| rb_min(self, r)),
        }
    }

    /// `RBT_INSERT(e)`: links `e`, which an owning container must own, and counts the link;
    /// `None`, or the element already there that compares equal (`e` left out, uncounted).
    pub fn insert(&mut self, e: &Elem<S>) -> Option<&Elem<S>> {
        self.check("RBT_INSERT");
        let sent = secondary_entry_of::<S>(e);
        if sent.is_linked() {
            hal_panic(format_args!(
                "RBT_INSERT: the element is in a secondary tree already"
            ));
        }
        let owner = entry_of::<S::Owner>(e);
        if !owner.is_linked() {
            hal_panic(format_args!(
                "RBT_INSERT: a secondary tree links only an element a container owns"
            ));
        }
        let mut parent = None;
        let mut went = Ordering::Equal;
        let mut n = get(&self.root);
        while let Some(x) = n {
            parent = Some(x);
            went = S::cmp(e, self.node(x));
            n = match went {
                Ordering::Less => self.view(x).left(),
                Ordering::Greater => self.view(x).right(),
                Ordering::Equal => return Some(self.node(x)),
            };
        }
        if self.id == 0 {
            self.id = fresh_id();
        }
        let changing = Mutating(&self.poisoned);
        owner.count_up();
        sent.owner.store(self.id, Mem::Relaxed);
        rb_link(self, NonNull::from(e), parent, went);
        core::mem::forget(changing);
        None
    }

    /// `RBT_REMOVE(e)`: unlinks `e` and uncounts the link; `e` must be in this tree.
    pub fn remove(&mut self, e: &Elem<S>) {
        self.check("RBT_REMOVE");
        let ent = self.ent(e, "RBT_REMOVE");
        let changing = Mutating(&self.poisoned);
        rb_remove(self, NonNull::from(e));
        ent.clear();
        entry_of::<S::Owner>(e).count_down();
        core::mem::forget(changing);
    }

    /// Unlinks every element and uncounts its link, in linear time (`RBT_INIT` of a
    /// selector's tree when it is destroyed): the tree is empty afterwards.
    pub fn clear(&mut self) {
        self.check("SecondaryTree::clear");
        let changing = Mutating(&self.poisoned);
        let mut cur = get(&self.root);
        put(&self.root, None);
        while let Some(x) = cur {
            let v = self.view(x);
            if let Some(l) = v.left() {
                cur = Some(l);
                continue;
            }
            if let Some(r) = v.right() {
                cur = Some(r);
                continue;
            }
            let p = v.parent();
            if let Some(p) = p {
                let pv = self.view(p);
                if pv.left() == Some(x) {
                    put(pv.left, None);
                } else {
                    put(pv.right, None);
                }
            }
            let e = self.node(x);
            self.ent(e, "SecondaryTree::clear").clear();
            entry_of::<S::Owner>(e).count_down();
            cur = p;
        }
        core::mem::forget(changing);
    }
}

impl<S: SecondaryAdapter> Default for SecondaryTree<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: SecondaryAdapter> Drop for SecondaryTree<S> {
    /// Unlinks and uncounts every element ([`SecondaryTree::clear`]). A poisoned tree keeps
    /// its counts, so the owners of its elements never free them (I4, I5).
    fn drop(&mut self) {
        if !self.poisoned.load(Mem::Relaxed) {
            self.clear();
        }
    }
}

/// The iterator of [`SecondaryTree::iter`].
pub struct SecondaryIter<'s, S: SecondaryAdapter> {
    tree: &'s SecondaryTree<S>,
    next: Option<NonNull<Elem<S>>>,
}

impl<'s, S: SecondaryAdapter> Iterator for SecondaryIter<'s, S> {
    type Item = &'s Elem<S>;

    fn next(&mut self) -> Option<&'s Elem<S>> {
        let x = self.next?;
        self.next = rb_next(self.tree, x);
        Some(self.tree.node(x))
    }
}

/// A counted pointer to an element an owning container owns (`up_pivot.entry`, the pivot's
/// entry): set, the element's [`OwnedEntry`] counts it, so its owner keeps it (I4); unset or
/// dropped, the count goes back.
pub struct SecondaryRef<A: OwnedAdapter> {
    to: AtomicPtr<A::Elem>,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: OwnedAdapter> SecondaryRef<A> {
    /// A pointer to nothing (`NULL`).
    pub const fn new() -> Self {
        Self {
            to: AtomicPtr::new(ptr::null_mut()),
            _adapter: PhantomData,
        }
    }

    /// The element pointed at, `None` for nothing.
    pub fn get(&self) -> Option<&A::Elem> {
        get(&self.to).map(|p| counted_ref(self, p))
    }

    /// Whether it points at something.
    pub fn is_some(&self) -> bool {
        get(&self.to).is_some()
    }

    /// Whether it points at `e` (an address comparison).
    pub fn points_to(&self, e: &A::Elem) -> bool {
        get(&self.to) == Some(NonNull::from(e))
    }

    /// Points at `to` (`None`: at nothing), which an owning container must own; the old
    /// target's count goes back.
    pub fn set(&mut self, to: Option<&A::Elem>) {
        if let Some(e) = to {
            let ent = entry_of::<A>(e);
            if !ent.is_linked() {
                hal_panic(format_args!(
                    "SecondaryRef: points only at an element a container owns"
                ));
            }
            ent.count_up();
        }
        if let Some(old) = get(&self.to) {
            let old = counted_ref(self, old);
            let ent = entry_of::<A>(old);
            if !ent.is_linked() {
                hal_panic(format_args!("SecondaryRef: its element is in no container"));
            }
            ent.count_down();
        }
        put(&self.to, to.map(NonNull::from));
    }
}

impl<A: OwnedAdapter> Default for SecondaryRef<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: OwnedAdapter> Drop for SecondaryRef<A> {
    fn drop(&mut self) {
        self.set(None);
    }
}

/// The source of container identities (I2): never 0, never reused.
static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Names the [`OwnedEntry`] of an element for its owning containers, the tree's order and
/// its augment (`RBT_PROTOTYPE(name, type, field, cmp)` and `RBT_GENERATE_AUGMENT`).
/// Implemented by [`owned_adapter!`].
pub trait OwnedAdapter: 'static {
    /// The element type.
    type Elem: Send + Sync + 'static;
    /// Whether the tree is augmented (`RBT_GENERATE_AUGMENT`): `augment` is called only then.
    const AUGMENTED: bool = false;
    /// The element's entry.
    fn entry(e: &Self::Elem) -> &OwnedEntry<Self::Elem>;
    /// The order of the tree (`cmp(a, b)`: `a` before, equal to or after `b`).
    fn cmp(a: &Self::Elem, b: &Self::Elem) -> Ordering;
    /// The augment callback for one node: recomputes `e`'s summary from its own value and its
    /// children's summaries and stores it; whether it changed (`uvm_map_addr_augment`'s step).
    fn augment(e: &Self::Elem, left: Option<&Self::Elem>, right: Option<&Self::Elem>) -> bool {
        let _ = (e, left, right);
        false
    }
}

/// Names the [`SecondaryEntry`] of an element for one kind of secondary tree, the owning
/// adapter whose entry counts its links, and the order (`RBT_PROTOTYPE` of a second tree of
/// the same type: `uaddr_free_rbtree`). Implemented by [`secondary_adapter!`].
pub trait SecondaryAdapter: 'static {
    /// The adapter of the element's owning containers, whose [`OwnedEntry`] counts the links.
    type Owner: OwnedAdapter;
    /// The element's entry for this kind of secondary tree.
    fn entry(e: &Elem<Self>) -> &SecondaryEntry<Elem<Self>>;
    /// The order of the tree.
    fn cmp(a: &Elem<Self>, b: &Elem<Self>) -> Ordering;
}

/// What the red-black engine needs from a tree: its root, a node's links (checked to be
/// this tree's, I2), and the augment of a node.
trait Rb {
    type T: 'static;
    /// Whether `augment_walk` does anything.
    const AUGMENTED: bool;
    fn root_link(&self) -> &AtomicPtr<Self::T>;
    fn view(&self, x: NonNull<Self::T>) -> View<'_, Self::T>;
    fn augment_walk(&self, x: NonNull<Self::T>);
}

impl<A: OwnedAdapter> Rb for OwnedTree<A> {
    type T = A::Elem;
    const AUGMENTED: bool = A::AUGMENTED;

    fn root_link(&self) -> &AtomicPtr<A::Elem> {
        &self.root
    }

    #[inline]
    fn view(&self, x: NonNull<A::Elem>) -> View<'_, A::Elem> {
        self.ent(self.node(x), "RBT").view()
    }

    fn augment_walk(&self, x: NonNull<A::Elem>) {
        OwnedTree::augment_walk(self, x);
    }
}

impl<S: SecondaryAdapter> Rb for SecondaryTree<S> {
    type T = Elem<S>;
    const AUGMENTED: bool = false;

    fn root_link(&self) -> &AtomicPtr<Elem<S>> {
        &self.root
    }

    #[inline]
    fn view(&self, x: NonNull<Elem<S>>) -> View<'_, Elem<S>> {
        self.ent(self.node(x), "RBT").view()
    }

    fn augment_walk(&self, _x: NonNull<Elem<S>>) {}
}

/// The poison of I5: dropped only when a panic unwinds out of a change (the change's normal
/// end forgets it).
struct Mutating<'a>(&'a AtomicBool);

impl Drop for Mutating<'_> {
    fn drop(&mut self) {
        self.0.store(true, Mem::Relaxed);
    }
}

/// A pointer read from a link.
#[inline]
fn get<T>(a: &AtomicPtr<T>) -> Option<NonNull<T>> {
    NonNull::new(a.load(Mem::Relaxed))
}

/// A link set (`None`: `NULL`).
#[inline]
fn put<T>(a: &AtomicPtr<T>, v: Option<NonNull<T>>) {
    a.store(v.map_or(ptr::null_mut(), NonNull::as_ptr), Mem::Relaxed);
}

/// A fresh container identity (I2).
fn fresh_id() -> usize {
    let id = NEXT_ID.fetch_add(1, Mem::Relaxed);
    if id == 0 || id == usize::MAX {
        hal_panic(format_args!("owned: container identities exhausted"));
    }
    id
}

#[cold]
fn poisoned(what: &str) -> ! {
    hal_panic(format_args!(
        "{what}: the container was being changed when a panic cut the change short"
    ))
}

/// Whether `inner` lies within the bytes of `outer`.
#[inline]
fn within<O, I>(outer: &O, inner: &I) -> bool {
    let base = ptr::from_ref(outer).addr();
    ptr::from_ref(inner)
        .addr()
        .checked_sub(base)
        .and_then(|off| off.checked_add(size_of::<I>()))
        .is_some_and(|end| end <= size_of::<O>())
}

/// The [`OwnedEntry`] `A` names for `e`, checked to lie within `e` (I2).
#[inline]
fn entry_of<A: OwnedAdapter>(e: &A::Elem) -> &OwnedEntry<A::Elem> {
    let ent = A::entry(e);
    if !within(e, ent) {
        not_own_entry("OwnedAdapter");
    }
    ent
}

/// The [`SecondaryEntry`] `S` names for `e`, checked to lie within `e` (I2).
#[inline]
fn secondary_entry_of<S: SecondaryAdapter>(e: &Elem<S>) -> &SecondaryEntry<Elem<S>> {
    let ent = S::entry(e);
    if !within(e, ent) {
        not_own_entry("SecondaryAdapter");
    }
    ent
}

/// I2's refusal of an element whose entry carries another container's identity (`what`
/// names the operation): kept out of the walks' line.
#[cold]
#[inline(never)]
fn not_in_tree(what: &str) -> ! {
    hal_panic(format_args!("{what}: the element is not in this tree"))
}

/// I2's refusal of an adapter whose entry is not within the element (`which` names the
/// adapter trait).
#[cold]
#[inline(never)]
fn not_own_entry(which: &str) -> ! {
    hal_panic(format_args!("{which}: the entry is not the element's own"))
}

/// The element an owning container's pointer names, borrowed for as long as the container.
///
/// Its callers, all in this module, pass only a pointer read from `_by`'s head or from a link
/// of an entry `_by` checked to carry its identity, the pointer `PoolBox::into_raw` just gave
/// for a box `_by` consumed, the pointer of an element just moved to `_by` from the tree that
/// owned it, its entry given `_by`'s identity (`OwnedTree::drain_into`), or the address of an
/// element its caller holds by reference and `_by` checked to carry its identity
/// (`OwnedTree::next(&self, e)`), used only while that reference lives.
#[inline]
fn owned_ref<T: 'static, C>(_by: &C, p: NonNull<T>) -> &T {
    // SAFETY: `p` names an element `_by` owns: the pointer of a box it consumed (I1), of an
    // element another owning container handed it without a box in between (the drain: the
    // element stays owned throughout), one read from its head or from an entry carrying its
    // identity, which name only elements it owns (I3, kept by every change here; a change cut
    // short poisons the container and nothing reads its links again, I5), or the address of
    // an element of `_by` (its entry carries `_by`'s identity, I2) that the caller holds by a
    // shared reference, alive for the call. An owned element is a live pool item,
    // initialised by `BoxPool::get` before its box existed, which no box frees while it is
    // owned (I1) and which never moves; no `&mut` to it exists while it is owned (no box
    // exists, and the containers and secondaries make only shared references). The reference
    // borrows `_by`, which gives an element back (a box, and so a possible `&mut` or free)
    // only through `&mut self`: no reference made here outlives its ownership. A pointer
    // stored in a link has the item's whole provenance (`PoolBox::into_raw`); one taken from
    // the caller's reference is that reference's, a shared one, and the reference made from
    // it here is shared too and dropped before the call returns.
    unsafe { p.as_ref() }
}

/// The element a secondary's pointer names, borrowed for as long as the secondary.
///
/// Its callers, all in this module, pass only a pointer read from a [`SecondaryTree`]'s root
/// or from a [`SecondaryEntry`] it checked to carry its identity, a [`SecondaryRef`]'s, or
/// the address of an element its caller holds by reference and the tree checked to carry its
/// identity (`SecondaryTree::next(&self, e)`, `remove`), used only while that reference lives.
#[inline]
fn counted_ref<T: 'static, C>(_by: &C, p: NonNull<T>) -> &T {
    // SAFETY: `p` names an element that an owning container owns and whose `OwnedEntry`
    // counts this link (I3: a secondary links only an owned element and counts it before it
    // stores the pointer, and uncounts only after it removed it); an owning container never
    // gives back, so never frees, a counted element (I4), and a container dropped with one
    // panics before it frees it; so the element is a live, initialised pool item that does
    // not move, reached only by shared references (it is owned, so no box and no `&mut`
    // exists, I1). The reference borrows `_by`, which uncounts the link only through `&mut
    // self`. Every stored pointer was taken from a shared reference to the element (the
    // insert's, the `set`'s), as is one passed by the caller, so the reference made here is
    // a shared one of the same element.
    unsafe { p.as_ref() }
}

/// The element of a box an owning container gives back.
fn reclaim<T: 'static>(p: NonNull<T>, pool: &'static BoxPool<T>) -> PoolBox<T> {
    // SAFETY: `p` and `pool` are the pair `PoolBox::into_raw` gave when the container that
    // calls this consumed the element's box (the pool kept in its entry since), and the
    // container rebuilds the box once: it has just unlinked the element and cleared its
    // entry, so no container reaches it any more. Its count is 0, checked by the caller after
    // the last code of an adapter ran (the unlinking's `entry` and `augment` calls, which may
    // link the element from a secondary) and with nothing but this module's own code between
    // that check and here (I4), so no secondary reaches it; every reference the container
    // made borrowed it and ended before the `&mut self` of the caller, the only way here. So
    // no reference to the value is alive or used after this.
    unsafe { PoolBox::from_raw(p, pool) }
}

/// `RBT_MIN` from `x`: its leftmost descendant.
#[inline]
fn rb_min<R: Rb>(t: &R, x: NonNull<R::T>) -> Option<NonNull<R::T>> {
    let mut n = x;
    while let Some(l) = t.view(n).left() {
        n = l;
    }
    Some(n)
}

/// `RBT_MAX` from `x`: its rightmost descendant.
#[inline]
fn rb_max<R: Rb>(t: &R, x: NonNull<R::T>) -> Option<NonNull<R::T>> {
    let mut n = x;
    while let Some(r) = t.view(n).right() {
        n = r;
    }
    Some(n)
}

/// `RBT_NEXT(x)`.
#[inline]
fn rb_next<R: Rb>(t: &R, x: NonNull<R::T>) -> Option<NonNull<R::T>> {
    rb_next_from(t, x, t.view(x))
}

/// `RBT_NEXT(x)` from `x`'s view `xv` (its entry checked by the caller): each node of the
/// walk is viewed, so checked, once.
#[inline]
fn rb_next_from<R: Rb>(t: &R, x: NonNull<R::T>, xv: View<'_, R::T>) -> Option<NonNull<R::T>> {
    if let Some(r) = xv.right() {
        return rb_min(t, r);
    }
    let mut child = x;
    let mut p = xv.parent();
    while let Some(pp) = p {
        let pv = t.view(pp);
        if pv.left() == Some(child) {
            return Some(pp);
        }
        child = pp;
        p = pv.parent();
    }
    None
}

/// `RBT_PREV(x)` from `x`'s view `xv`, as [`rb_next_from`].
#[inline]
fn rb_prev_from<R: Rb>(t: &R, x: NonNull<R::T>, xv: View<'_, R::T>) -> Option<NonNull<R::T>> {
    if let Some(l) = xv.left() {
        return rb_max(t, l);
    }
    let mut child = x;
    let mut p = xv.parent();
    while let Some(pp) = p {
        let pv = t.view(pp);
        if pv.right() == Some(child) {
            return Some(pp);
        }
        child = pp;
        p = pv.parent();
    }
    None
}

/// The tree's shape broke an invariant the fix-ups rely on (a `NULL` sibling): only a tree
/// changed outside this module could, and none can.
#[cold]
fn broken(what: &str) -> ! {
    hal_panic(format_args!("{what}: the tree is broken"))
}

/// `rbe_if_augment(x)`.
fn rb_augment<R: Rb>(t: &R, x: NonNull<R::T>) {
    if R::AUGMENTED {
        t.augment_walk(x);
    }
}

/// `_rb_insert` after the search: `x` (its entry carrying the tree's identity, links unset)
/// linked as a red leaf under `parent` on the side `went` says, then `rbe_insert_color`.
fn rb_link<R: Rb>(t: &R, x: NonNull<R::T>, parent: Option<NonNull<R::T>>, went: Ordering) {
    let xv = t.view(x);
    put(xv.parent, parent);
    put(xv.left, None);
    put(xv.right, None);
    xv.set_color(RED);
    match parent {
        Some(p) => {
            let pv = t.view(p);
            if went == Ordering::Less {
                put(pv.left, Some(x));
            } else {
                put(pv.right, Some(x));
            }
            rb_augment(t, p);
        }
        None => put(t.root_link(), Some(x)),
    }
    rb_insert_color(t, x);
}

/// `parent`'s link to `old` (the root when `parent` is `None`) now names `new`.
fn rb_replace<R: Rb>(
    t: &R,
    parent: Option<NonNull<R::T>>,
    old: NonNull<R::T>,
    new: Option<NonNull<R::T>>,
) {
    match parent {
        Some(p) => {
            let pv = t.view(p);
            if pv.left() == Some(old) {
                put(pv.left, new);
            } else {
                put(pv.right, new);
            }
        }
        None => put(t.root_link(), new),
    }
}

/// `rbe_rotate_left(x)`: `x`'s right child takes its place, `x` becoming its left child.
fn rb_rotate_left<R: Rb>(t: &R, x: NonNull<R::T>) {
    let xv = t.view(x);
    let Some(y) = xv.right() else {
        broken("RBT rotate")
    };
    let yv = t.view(y);
    let yl = yv.left();
    put(xv.right, yl);
    if let Some(yl) = yl {
        put(t.view(yl).parent, Some(x));
    }
    let p = xv.parent();
    put(yv.parent, p);
    rb_replace(t, p, x, Some(y));
    put(yv.left, Some(x));
    put(xv.parent, Some(y));
    if R::AUGMENTED {
        t.augment_walk(x);
        t.augment_walk(y);
        if let Some(p) = yv.parent() {
            t.augment_walk(p);
        }
    }
}

/// `rbe_rotate_right(x)`: the mirror of [`rb_rotate_left`].
fn rb_rotate_right<R: Rb>(t: &R, x: NonNull<R::T>) {
    let xv = t.view(x);
    let Some(y) = xv.left() else {
        broken("RBT rotate")
    };
    let yv = t.view(y);
    let yr = yv.right();
    put(xv.left, yr);
    if let Some(yr) = yr {
        put(t.view(yr).parent, Some(x));
    }
    let p = xv.parent();
    put(yv.parent, p);
    rb_replace(t, p, x, Some(y));
    put(yv.right, Some(x));
    put(xv.parent, Some(y));
    if R::AUGMENTED {
        t.augment_walk(x);
        t.augment_walk(y);
        if let Some(p) = yv.parent() {
            t.augment_walk(p);
        }
    }
}

/// Whether `x` is red (an absent node is black).
fn is_red<R: Rb>(t: &R, x: Option<NonNull<R::T>>) -> bool {
    x.is_some_and(|x| t.view(x).color() == RED)
}

/// `rbe_insert_color`: restores the colouring after `x`, red, was linked as a leaf.
fn rb_insert_color<R: Rb>(t: &R, x: NonNull<R::T>) {
    let mut x = x;
    while let Some(parent) = t.view(x).parent()
        && let pv = t.view(parent)
        && pv.color() == RED
    {
        // A red parent is never the root, so the grandparent exists.
        let Some(gparent) = pv.parent() else {
            broken("RBT_INSERT")
        };
        let gv = t.view(gparent);
        let parent_is_left = gv.left() == Some(parent);
        let uncle = if parent_is_left {
            gv.right()
        } else {
            gv.left()
        };
        if let Some(u) = uncle
            && let uv = t.view(u)
            && uv.color() == RED
        {
            uv.set_color(BLACK);
            pv.set_color(BLACK);
            gv.set_color(RED);
            x = gparent;
            continue;
        }
        let mut parent = parent;
        if parent_is_left {
            if pv.right() == Some(x) {
                rb_rotate_left(t, parent);
                core::mem::swap(&mut parent, &mut x);
            }
            t.view(parent).set_color(BLACK);
            gv.set_color(RED);
            rb_rotate_right(t, gparent);
        } else {
            if pv.left() == Some(x) {
                rb_rotate_right(t, parent);
                core::mem::swap(&mut parent, &mut x);
            }
            t.view(parent).set_color(BLACK);
            gv.set_color(RED);
            rb_rotate_left(t, gparent);
        }
    }
    if let Some(r) = get(t.root_link()) {
        t.view(r).set_color(BLACK);
    }
}

/// `rbe_remove(old)`: takes `old` out of the tree (its own links are left for the caller to
/// clear), with the augment calls and `rbe_remove_color` of the C.
fn rb_remove<R: Rb>(t: &R, old: NonNull<R::T>) {
    let ov = t.view(old);
    let (child, parent, color) = match (ov.left(), ov.right()) {
        (None, c) | (c, None) => {
            let parent = ov.parent();
            let color = ov.color();
            if let Some(c) = c {
                put(t.view(c).parent, parent);
            }
            rb_replace(t, parent, old, c);
            if let Some(p) = parent {
                rb_augment(t, p);
            }
            (c, parent, color)
        }
        (Some(_), Some(right)) => {
            // Two children: the successor takes `old`'s place.
            let Some(s) = rb_min(t, right) else {
                broken("RBT_REMOVE")
            };
            let sv = t.view(s);
            let child = sv.right();
            // The successor is below `old`, so it has a parent.
            let Some(sp) = sv.parent() else {
                broken("RBT_REMOVE")
            };
            let color = sv.color();
            if let Some(c) = child {
                put(t.view(c).parent, Some(sp));
            }
            rb_replace(t, Some(sp), s, child);
            rb_augment(t, sp);
            let parent = if sp == old { s } else { sp };
            // `*rbe = *old`: read after the detach above, which may have changed `old`'s
            // right child.
            let (l, r, p) = (ov.left(), ov.right(), ov.parent());
            put(sv.left, l);
            put(sv.right, r);
            put(sv.parent, p);
            sv.set_color(ov.color());
            rb_replace(t, p, old, Some(s));
            if let Some(p) = p {
                rb_augment(t, p);
            }
            match l {
                Some(l) => put(t.view(l).parent, Some(s)),
                None => broken("RBT_REMOVE"),
            }
            if let Some(r) = r {
                put(t.view(r).parent, Some(s));
            }
            if R::AUGMENTED {
                let mut tmp = Some(parent);
                while let Some(x) = tmp {
                    t.augment_walk(x);
                    tmp = t.view(x).parent();
                }
            }
            (child, Some(parent), color)
        }
    };
    if color == BLACK {
        rb_remove_color(t, parent, child);
    }
}

/// `rbe_remove_color`: restores the colouring after a black node left the tree; `x` (maybe
/// absent) took its place under `parent`.
fn rb_remove_color<R: Rb>(t: &R, parent: Option<NonNull<R::T>>, x: Option<NonNull<R::T>>) {
    let mut x = x;
    let mut parent = parent;
    while !is_red(t, x) && x != get(t.root_link()) {
        let Some(p) = parent else {
            broken("RBT_REMOVE")
        };
        let pv = t.view(p);
        if pv.left() == x {
            let Some(mut w) = pv.right() else {
                broken("RBT_REMOVE")
            };
            if t.view(w).color() == RED {
                t.view(w).set_color(BLACK);
                pv.set_color(RED);
                rb_rotate_left(t, p);
                let Some(nw) = pv.right() else {
                    broken("RBT_REMOVE")
                };
                w = nw;
            }
            let wv = t.view(w);
            if !is_red(t, wv.left()) && !is_red(t, wv.right()) {
                wv.set_color(RED);
                x = Some(p);
                parent = pv.parent();
            } else {
                if !is_red(t, wv.right()) {
                    if let Some(wl) = wv.left() {
                        t.view(wl).set_color(BLACK);
                    }
                    wv.set_color(RED);
                    rb_rotate_right(t, w);
                    let Some(nw) = pv.right() else {
                        broken("RBT_REMOVE")
                    };
                    w = nw;
                }
                let wv = t.view(w);
                wv.set_color(pv.color());
                pv.set_color(BLACK);
                if let Some(wr) = wv.right() {
                    t.view(wr).set_color(BLACK);
                }
                rb_rotate_left(t, p);
                x = get(t.root_link());
                break;
            }
        } else {
            let Some(mut w) = pv.left() else {
                broken("RBT_REMOVE")
            };
            if t.view(w).color() == RED {
                t.view(w).set_color(BLACK);
                pv.set_color(RED);
                rb_rotate_right(t, p);
                let Some(nw) = pv.left() else {
                    broken("RBT_REMOVE")
                };
                w = nw;
            }
            let wv = t.view(w);
            if !is_red(t, wv.left()) && !is_red(t, wv.right()) {
                wv.set_color(RED);
                x = Some(p);
                parent = pv.parent();
            } else {
                if !is_red(t, wv.left()) {
                    if let Some(wr) = wv.right() {
                        t.view(wr).set_color(BLACK);
                    }
                    wv.set_color(RED);
                    rb_rotate_left(t, w);
                    let Some(nw) = pv.left() else {
                        broken("RBT_REMOVE")
                    };
                    w = nw;
                }
                let wv = t.view(w);
                wv.set_color(pv.color());
                pv.set_color(BLACK);
                if let Some(wl) = wv.left() {
                    t.view(wl).set_color(BLACK);
                }
                rb_rotate_right(t, p);
                x = get(t.root_link());
                break;
            }
        }
    }
    if let Some(x) = x {
        t.view(x).set_color(BLACK);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::Arc;
    use std::sync::Once;
    use std::sync::atomic::{AtomicU64, AtomicUsize};
    use std::vec::Vec;

    use super::*;
    use crate::pool::PR_WAITOK;
    use crate::testutil;

    /// A map entry's shape: a key, a free space, its subtree's largest free space, the owner
    /// link, a free-tree link, and a count of the drops.
    struct Node {
        key: u64,
        fspace: AtomicU64,
        augment: AtomicU64,
        link: OwnedEntry<Node>,
        free: SecondaryEntry<Node>,
        other: OwnedEntry<Node>,
        drops: Option<Arc<AtomicUsize>>,
        explode: bool,
    }

    impl Drop for Node {
        fn drop(&mut self) {
            if let Some(d) = &self.drops {
                d.fetch_add(1, Mem::Relaxed);
            }
        }
    }

    /// `uvm_map_addr_augment`'s step: the largest free space of the node and its children's
    /// subtrees.
    fn augment_fspace(e: &Node, l: Option<&Node>, r: Option<&Node>) -> bool {
        let explodes = |c: Option<&Node>| c.is_some_and(|c| c.explode);
        assert!(
            !e.explode && !explodes(l) && !explodes(r),
            "the augment callback panicked"
        );
        let mut m = e.fspace.load(Mem::Relaxed);
        for c in [l, r].into_iter().flatten() {
            m = m.max(c.augment.load(Mem::Relaxed));
        }
        if e.augment.load(Mem::Relaxed) == m {
            return false;
        }
        e.augment.store(m, Mem::Relaxed);
        true
    }

    owned_adapter!(ByKey: Node, link, |a, b| a.key.cmp(&b.key), augment = augment_fspace);
    owned_adapter!(ByKeyOther: Node, other, |a, b| a.key.cmp(&b.key));
    secondary_adapter!(ByFree: ByKey, free, |a, b| {
        (a.fspace.load(Mem::Relaxed), a.key).cmp(&(b.fspace.load(Mem::Relaxed), b.key))
    });

    /// An adapter that names a static entry, not the element's own.
    struct Stray;

    static STRAY: OwnedEntry<Node> = OwnedEntry::new();

    impl OwnedAdapter for Stray {
        type Elem = Node;
        fn entry(_e: &Node) -> &OwnedEntry<Node> {
            &STRAY
        }
        fn cmp(a: &Node, b: &Node) -> Ordering {
            a.key.cmp(&b.key)
        }
    }

    static POOL: BoxPool<Node> = BoxPool::new();
    static MADE: Once = Once::new();

    fn pool() -> &'static BoxPool<Node> {
        MADE.call_once(|| {
            testutil::install();
            POOL.init(0, PR_WAITOK, "ownedpl").unwrap();
        });
        &POOL
    }

    fn node(key: u64, fspace: u64, drops: &Arc<AtomicUsize>) -> PoolBox<Node> {
        pool()
            .get(PR_WAITOK, || Node {
                key,
                fspace: AtomicU64::new(fspace),
                augment: AtomicU64::new(fspace),
                link: OwnedEntry::new(),
                free: SecondaryEntry::new(),
                other: OwnedEntry::new(),
                drops: Some(Arc::clone(drops)),
                explode: false,
            })
            .unwrap()
    }

    /// A xorshift: the tests' random keys, reproducible.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }

    /// The red-black invariants and the augment below `x`: its black height and the largest
    /// free space of its subtree.
    fn check_subtree(
        t: &OwnedTree<ByKey>,
        x: &Node,
        lo: Option<u64>,
        hi: Option<u64>,
    ) -> (u32, u64) {
        assert!(
            lo.is_none_or(|lo| x.key > lo) && hi.is_none_or(|hi| x.key < hi),
            "order"
        );
        let mut max = x.fspace.load(Mem::Relaxed);
        let mut heights = [0u32; 2];
        for (i, c) in [t.left(x), t.right(x)].into_iter().enumerate() {
            if let Some(c) = c {
                assert!(ptr::eq(t.parent(c).unwrap(), x), "parent link");
                if x.link.color.load(Mem::Relaxed) == RED {
                    assert_eq!(c.link.color.load(Mem::Relaxed), BLACK, "red under red");
                }
                let (lo, hi) = if i == 0 {
                    (lo, Some(x.key))
                } else {
                    (Some(x.key), hi)
                };
                let (h, m) = check_subtree(t, c, lo, hi);
                heights[i] = h;
                max = max.max(m);
            }
        }
        assert_eq!(heights[0], heights[1], "black heights");
        assert_eq!(x.augment.load(Mem::Relaxed), max, "augment of {}", x.key);
        let own = u32::from(x.link.color.load(Mem::Relaxed) == BLACK);
        (heights[0] + own, max)
    }

    fn check_tree(t: &OwnedTree<ByKey>, oracle: &BTreeSet<u64>) {
        if let Some(r) = t.root() {
            assert_eq!(r.link.color.load(Mem::Relaxed), BLACK, "black root");
            assert!(t.parent(r).is_none());
            check_subtree(t, r, None, None);
        }
        let keys: Vec<u64> = t.iter().map(|n| n.key).collect();
        assert_eq!(keys, oracle.iter().copied().collect::<Vec<_>>());
        let back: Vec<u64> = {
            let mut v = Vec::new();
            let mut n = t.max();
            while let Some(x) = n {
                v.push(x.key);
                n = t.prev(x);
            }
            v
        };
        assert_eq!(back, oracle.iter().rev().copied().collect::<Vec<_>>());
    }

    #[test]
    fn the_tree_matches_a_btreeset_oracle() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        let mut oracle = BTreeSet::new();
        let mut rng = Rng(0x2545_f491_4f6c_dd1d);
        let mut made = 0usize;
        for i in 0..1000 {
            let key = rng.next() % 512;
            if rng.next() % 3 == 0 {
                let got = t.take_by(|n| key.cmp(&n.key));
                assert_eq!(got.is_some(), oracle.remove(&key));
                if let Some(b) = got {
                    assert_eq!(b.key, key);
                    assert!(!b.link.is_linked());
                }
            } else {
                made += 1;
                let refused = t.insert(node(key, rng.next() % 4096, &drops));
                assert_eq!(refused.is_some(), !oracle.insert(key));
            }
            if i % 10 == 0 || i > 990 {
                check_tree(&t, &oracle);
            }
            assert_eq!(
                t.nfind_by(|n| key.cmp(&n.key)).map(|n| n.key),
                oracle.range(key..).next().copied()
            );
        }
        check_tree(&t, &oracle);
        assert_eq!(t.min().map(|n| n.key), oracle.first().copied());
        assert_eq!(t.find_by(|n| 9999u64.cmp(&n.key)).map(|n| n.key), None);
        drop(t);
        assert_eq!(drops.load(Mem::Relaxed), made);
    }

    #[test]
    fn augment_follows_a_changed_value() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        let mut oracle = BTreeSet::new();
        for k in 0..64u64 {
            assert!(t.insert(node(k, k % 7, &drops)).is_none());
            oracle.insert(k);
        }
        check_tree(&t, &oracle);
        let e = t.find_by(|n| 33u64.cmp(&n.key)).unwrap();
        e.fspace.store(1_000_000, Mem::Relaxed);
        t.augment(e);
        assert_eq!(t.root().unwrap().augment.load(Mem::Relaxed), 1_000_000);
        check_tree(&t, &oracle);
        let e = t.find_by(|n| 33u64.cmp(&n.key)).unwrap();
        e.fspace.store(0, Mem::Relaxed);
        t.augment(e);
        assert_eq!(t.root().unwrap().augment.load(Mem::Relaxed), 6);
        check_tree(&t, &oracle);
    }

    #[test]
    fn an_equal_key_is_given_back() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        assert!(t.insert(node(5, 0, &drops)).is_none());
        let b = t.insert(node(5, 1, &drops)).unwrap();
        assert!(!b.link.is_linked());
        assert_eq!(t.find(&b).unwrap().fspace.load(Mem::Relaxed), 0);
        drop(b);
        assert_eq!(drops.load(Mem::Relaxed), 1);
        assert!(t.contains(t.root().unwrap()));
    }

    #[test]
    fn drain_into_moves_every_element_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        for k in 0..100u64 {
            assert!(t.insert(node(k * 3, 0, &drops)).is_none());
        }
        let mut dead = OwnedList::<ByKey>::new();
        t.drain_into(&mut dead);
        assert!(t.is_empty());
        let mut seen: Vec<u64> = dead.iter().map(|n| n.key).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..100u64).map(|k| k * 3).collect::<Vec<_>>());
        // The tree works again afterwards.
        assert!(t.insert(node(1, 0, &drops)).is_none());
        let mut n = 0;
        while let Some(b) = dead.pop() {
            assert!(!b.link.is_linked());
            n += 1;
        }
        assert_eq!(n, 100);
        assert_eq!(drops.load(Mem::Relaxed), 100);
    }

    #[test]
    fn the_list_is_first_in_first_out() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut l = OwnedList::<ByKey>::new();
        assert!(l.is_empty() && l.first().is_none() && l.pop().is_none());
        for k in 0..10u64 {
            l.push(node(k, 0, &drops));
        }
        assert_eq!(
            l.iter().map(|n| n.key).collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
        let third = l.next(l.next(l.first().unwrap()).unwrap()).unwrap();
        assert_eq!(third.key, 2);
        assert_eq!(l.prev(third).unwrap().key, 1);
        assert_eq!(l.last().unwrap().key, 9);
        assert!(l.contains(third));
        assert_eq!(l.pop().unwrap().key, 0);
        assert_eq!(l.first().unwrap().key, 1);
        assert!(l.prev(l.first().unwrap()).is_none());
        drop(l);
        assert_eq!(drops.load(Mem::Relaxed), 10);
    }

    #[test]
    fn a_secondary_tree_orders_and_counts() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        for k in 0..32u64 {
            assert!(t.insert(node(k, (k * 7) % 11, &drops)).is_none());
        }
        let mut free = SecondaryTree::<ByFree>::new();
        for e in t.iter() {
            assert!(free.insert(e).is_none());
            assert_eq!(e.link.indexed(), 1);
        }
        let order: Vec<(u64, u64)> = free
            .iter()
            .map(|n| (n.fspace.load(Mem::Relaxed), n.key))
            .collect();
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(order, sorted);
        let e = t.find_by(|n| 4u64.cmp(&n.key)).unwrap();
        assert!(free.contains(e));
        free.remove(e);
        assert_eq!(e.link.indexed(), 0);
        assert!(!free.contains(e));
        // A best fit: the first free space of at least 9.
        let fit = free.nfind_by(|n| (9u64, 0u64).cmp(&(n.fspace.load(Mem::Relaxed), n.key)));
        assert!(fit.is_some_and(|n| n.fspace.load(Mem::Relaxed) >= 9));
        // Element 4 is uncounted: it can be given back while the others are linked.
        assert_eq!(t.take_by(|n| 4u64.cmp(&n.key)).unwrap().key, 4);
        free.clear();
        assert!(free.is_empty());
        assert!(t.iter().all(|n| n.link.indexed() == 0));
        drop(t);
        assert_eq!(drops.load(Mem::Relaxed), 32);
    }

    #[test]
    fn a_secondary_ref_counts_its_element() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        for k in 0..4u64 {
            assert!(t.insert(node(k, 0, &drops)).is_none());
        }
        let mut r = SecondaryRef::<ByKey>::new();
        assert!(r.get().is_none());
        let two = t.find_by(|n| 2u64.cmp(&n.key)).unwrap();
        r.set(Some(two));
        assert_eq!(two.link.indexed(), 1);
        assert!(r.points_to(two));
        assert_eq!(r.get().unwrap().key, 2);
        let three = t.find_by(|n| 3u64.cmp(&n.key)).unwrap();
        r.set(Some(three));
        assert_eq!((two.link.indexed(), three.link.indexed()), (0, 1));
        r.set(Some(three));
        assert_eq!(three.link.indexed(), 1);
        drop(r);
        assert_eq!(three.link.indexed(), 0);
        assert!(t.take_by(|n| 3u64.cmp(&n.key)).is_some());
    }

    #[test]
    #[should_panic(expected = "still linked from a secondary tree")]
    fn take_by_refuses_a_counted_element() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        assert!(t.insert(node(1, 0, &drops)).is_none());
        let mut r = SecondaryRef::<ByKey>::new();
        r.set(t.root());
        let _ = t.take_by(|n| 1u64.cmp(&n.key));
    }

    #[test]
    #[should_panic(expected = "still linked from a secondary tree")]
    fn pop_refuses_a_counted_element() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut l = OwnedList::<ByKey>::new();
        l.push(node(1, 0, &drops));
        let mut r = SecondaryRef::<ByKey>::new();
        r.set(l.first());
        let _ = l.pop();
    }

    #[test]
    #[should_panic(expected = "still linked from a secondary tree")]
    fn dropping_a_tree_with_a_counted_element_panics() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut free = SecondaryTree::<ByFree>::new();
        let mut t = OwnedTree::<ByKey>::new();
        for k in 0..8u64 {
            assert!(t.insert(node(k, k, &drops)).is_none());
        }
        assert!(free.insert(t.root().unwrap()).is_none());
        drop(t);
    }

    #[test]
    #[should_panic(expected = "only an element a container owns")]
    fn a_secondary_refuses_an_element_in_no_container() {
        let drops = Arc::new(AtomicUsize::new(0));
        let b = node(1, 0, &drops);
        let mut free = SecondaryTree::<ByFree>::new();
        let _ = free.insert(&b);
    }

    #[test]
    #[should_panic(expected = "points only at an element a container owns")]
    fn a_secondary_ref_refuses_an_element_in_no_container() {
        let drops = Arc::new(AtomicUsize::new(0));
        let b = node(1, 0, &drops);
        let mut r = SecondaryRef::<ByKey>::new();
        r.set(Some(&b));
    }

    #[test]
    #[should_panic(expected = "not in this tree")]
    fn an_element_of_another_tree_is_refused() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut a = OwnedTree::<ByKey>::new();
        let mut b = OwnedTree::<ByKey>::new();
        assert!(a.insert(node(1, 0, &drops)).is_none());
        assert!(b.insert(node(2, 0, &drops)).is_none());
        let _ = b.next(a.root().unwrap());
    }

    #[test]
    #[should_panic(expected = "not in this tree")]
    fn an_element_on_a_list_is_not_in_a_tree() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        assert!(t.insert(node(1, 0, &drops)).is_none());
        let mut l = OwnedList::<ByKey>::new();
        l.push(node(2, 0, &drops));
        let _ = t.left(l.first().unwrap());
    }

    #[test]
    #[should_panic(expected = "only an element a container owns")]
    fn a_secondary_counts_only_its_owner_adapters_entry() {
        // Owned through `other`: the free tree's owner adapter names `link`, which is in no
        // container, so the link would be counted where no owner looks.
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKeyOther>::new();
        assert!(t.insert(node(1, 0, &drops)).is_none());
        let mut free = SecondaryTree::<ByFree>::new();
        let _ = free.insert(t.root().unwrap());
    }

    #[test]
    #[should_panic(expected = "the entry is not the element's own")]
    fn an_adapter_naming_another_entry_is_refused() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<Stray>::new();
        let _ = t.insert(node(1, 0, &drops));
    }

    #[test]
    fn a_panic_in_a_change_poisons_the_tree() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKey>::new();
        for k in 0..8u64 {
            assert!(t.insert(node(k, 0, &drops)).is_none());
        }
        let bomb = pool()
            .get(PR_WAITOK, || Node {
                key: 100,
                fspace: AtomicU64::new(0),
                augment: AtomicU64::new(0),
                link: OwnedEntry::new(),
                free: SecondaryEntry::new(),
                other: OwnedEntry::new(),
                drops: Some(Arc::clone(&drops)),
                explode: true,
            })
            .unwrap();
        let cut = catch_unwind(AssertUnwindSafe(|| t.insert(bomb)));
        assert!(cut.is_err());
        let again = catch_unwind(AssertUnwindSafe(|| t.is_empty()));
        assert!(again.is_err());
        // Dropped poisoned: everything leaks, nothing is freed.
        drop(t);
        assert_eq!(drops.load(Mem::Relaxed), 0);
    }

    #[test]
    fn identities_are_distinct() {
        let a = fresh_id();
        let b = fresh_id();
        assert!(a != 0 && b != 0 && a != b);
    }

    #[test]
    fn the_entries_are_as_large_as_lzs_links() {
        // LZ's entry had two `RbtEntry`s, a `SlistEntry` and a `TailqEntry`: 88 bytes.
        assert_eq!(size_of::<OwnedEntry<Node>>(), 48);
        assert_eq!(size_of::<SecondaryEntry<Node>>(), 40);
        assert_eq!(size_of::<SecondaryRef<ByKey>>(), 8);
    }

    /// Armed, the next augment of key 2 links that element from [`TRAP`]: what a kernel
    /// adapter could do while its element is being removed.
    static TRAP_ARMED: AtomicBool = AtomicBool::new(false);
    static TRAP: crate::mutex::Mutexed<SecondaryRef<ByKeyTrap>> =
        crate::mutex::Mutexed::new(0, SecondaryRef::new());

    fn augment_trap(e: &Node, l: Option<&Node>, r: Option<&Node>) -> bool {
        if e.key == 2 && TRAP_ARMED.swap(false, Mem::Relaxed) {
            TRAP.lock().set(Some(e));
        }
        augment_fspace(e, l, r)
    }

    owned_adapter!(ByKeyTrap: Node, link, |a, b| a.key.cmp(&b.key), augment = augment_trap);

    #[test]
    #[should_panic(
        expected = "RBT_REMOVE: the element was linked from a secondary while being removed"
    )]
    fn an_augment_that_links_the_element_being_removed_is_caught() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut t = OwnedTree::<ByKeyTrap>::new();
        for k in [2u64, 1, 3] {
            assert!(t.insert(node(k, k, &drops)).is_none());
        }
        // 2 at the root, 3 its right child without a left one: removing 2, the successor is
        // 3 and `rbe_if_augment(parent)` runs on 2 itself, after `take_by`'s first look at
        // the count. The box must not come back: the trap's link would dangle.
        assert_eq!(t.root().unwrap().key, 2);
        assert!(t.left(t.root().unwrap()).is_some_and(|n| n.key == 1));
        TRAP_ARMED.store(true, Mem::Relaxed);
        let _ = t.take_by(|n| 2u64.cmp(&n.key));
    }
}
/* </TESTS> */
