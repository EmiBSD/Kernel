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
/* </LICENSES> */

/* <CODE> */
//! `Tree<A>`: the red-black tree of `RBT_*` (`<sys/tree.h>`, `kern/subr_tree.c`) over elements
//! that live for ever.
//!
//! Upstream: sys/kern/subr_tree.c @ 3ce1f3f79392
//! Upstream: sys/sys/tree.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_tree.rs@3c62ede68802
//! LZ: sys/sys/tree.rs@3c62ede68802
//!
//! The behaviour of `RBT_INSERT`, `RBT_REMOVE`, `RBT_FIND`, `RBT_NFIND`, `RBT_MIN`/`MAX`,
//! `RBT_NEXT`/`PREV` and the `RBT_LEFT`/`RIGHT`/`PARENT`/`ROOT` walks, re-expressed from
//! `subr_tree.c` and `tree.h` (red-black colouring, `rbe_insert_color`'s and
//! `rbe_remove_color`'s fix-ups, `rbe_remove`'s replacement of a node with two children by its
//! successor, the rotations), written for the kernel's first users whose elements are never
//! freed (`kernel::vm`'s pages and pmemranges, N3.3). `bsd`'s `sys/sys/tree.rs` and
//! `sys/kern/subr_tree.rs` stay for the legacy tree (the 50-module rule) until their last user
//! moves. An element carries a [`TreeEntry`]; an adapter type (written by [`tree_adapter!`])
//! names the entry and the order. Insertion takes a `&'static` element, so every link is a
//! [`Link`] to something that outlives the tree and every operation is safe; a broken
//! precondition (inserting an element that is in a tree, removing one that is not, a tree whose
//! shape the fix-ups find impossible) panics, where the C would corrupt the tree or fault.
//!
//! As with [`crate::list::List`], the lock that keeps a tree consistent is its owner's; the
//! links are atomics, so a race can scramble a tree but never make a reference dangle.
//!
//! ## Redesign
//! - `RBT_HEAD`/`RBT_ENTRY` of `struct rb_entry *` links reached by `container_of` from the
//!   element's offset (`rb_n2e`, `rb_e2n`) -> [`Link`]s to the elements themselves: no offset
//!   arithmetic and no `unsafe`; the adapter's `entry` is an ordinary field access.
//! - `RBT_FIND`/`RBT_NFIND` with an element built on the stack as the key -> [`Tree::find_by`]
//!   and [`Tree::nfind_by`], which take the comparison of the key with a node, so no key
//!   element is built (`uvm_pagelookup`'s on-stack `struct vm_page`). [`Tree::find`] and
//!   [`Tree::nfind`] keep the element-keyed form.
//! - The augment callbacks (`RBT_AUGMENT`) are not provided: no user of this tree has one.
//! - `RBT_INIT` has nothing to do: a new tree is empty.

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering as Mem};

use crate::assert::hal_panic;
use crate::link::Link;

/// `RB_BLACK`.
const BLACK: u8 = 0;
/// `RB_RED`.
const RED: u8 = 1;

/// Names the [`TreeEntry`] of an element for one kind of tree, and its order
/// (`RBT_PROTOTYPE(name, type, field, cmp)`). Implemented by [`tree_adapter!`].
pub trait TreeAdapter {
    /// The element type.
    type Elem: 'static + Sync;
    /// The element's entry for this kind of tree.
    fn entry(e: &Self::Elem) -> &TreeEntry<Self::Elem>;
    /// The order of the tree (`cmp(a, b)`: `a` before, equal to or after `b`).
    fn cmp(a: &Self::Elem, b: &Self::Elem) -> Ordering;
}

/// Declares `$name`, the [`TreeAdapter`] of `$elem`'s field `$field` ordered by `$cmp` (a
/// function or a closure without captures, `fn(&$elem, &$elem) -> Ordering`):
/// `tree_adapter!(pub ByOffset: Page, objt, |a, b| a.offset().cmp(&b.offset()));`.
#[macro_export]
macro_rules! tree_adapter {
    ($(#[$meta:meta])* $vis:vis $name:ident: $elem:ty, $field:ident, $cmp:expr) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::tree::TreeAdapter for $name {
            type Elem = $elem;
            fn entry(e: &$elem) -> &$crate::tree::TreeEntry<$elem> {
                &e.$field
            }
            fn cmp(a: &$elem, b: &$elem) -> ::core::cmp::Ordering {
                let f: fn(&$elem, &$elem) -> ::core::cmp::Ordering = $cmp;
                f(a, b)
            }
        }
    };
}

/// `RBT_ENTRY(type)`: an element's links in one tree.
pub struct TreeEntry<T: 'static> {
    left: Link<T>,
    right: Link<T>,
    parent: Link<T>,
    color: AtomicU8,
    /// Whether the element is in a tree through this entry.
    linked: AtomicBool,
}

impl<T: 'static> TreeEntry<T> {
    /// An entry in no tree.
    pub const fn new() -> Self {
        Self {
            left: Link::new(),
            right: Link::new(),
            parent: Link::new(),
            color: AtomicU8::new(BLACK),
            linked: AtomicBool::new(false),
        }
    }

    /// Whether the element is in a tree through this entry.
    pub fn is_linked(&self) -> bool {
        self.linked.load(Mem::Relaxed)
    }

    fn color(&self) -> u8 {
        self.color.load(Mem::Relaxed)
    }

    fn set_color(&self, c: u8) {
        self.color.store(c, Mem::Relaxed);
    }
}

impl<T: 'static> Default for TreeEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `RBT_HEAD(name, type)`: a red-black tree of `A::Elem`s.
pub struct Tree<A: TreeAdapter> {
    root: Link<A::Elem>,
    _adapter: PhantomData<fn() -> A>,
}

/// Whether `l` names `x` (`None`: whether `l` is empty), by address.
fn same<T: 'static>(l: &Link<T>, x: Option<&T>) -> bool {
    match x {
        Some(x) => l.points_to(x),
        None => l.is_none(),
    }
}

/// The tree's shape broke a red-black invariant the fix-ups rely on (a `NULL` sibling): only a
/// tree changed without its lock, or an element in two trees, gets here.
#[cold]
fn broken(what: &str) -> ! {
    hal_panic(format_args!("{what}: the tree is broken"))
}

impl<A: TreeAdapter> Tree<A> {
    /// `RBT_INITIALIZER`: an empty tree.
    pub const fn new() -> Self {
        Self {
            root: Link::new(),
            _adapter: PhantomData,
        }
    }

    fn e(x: &A::Elem) -> &TreeEntry<A::Elem> {
        A::entry(x)
    }

    /// `RBT_EMPTY`.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// `RBT_ROOT`.
    pub fn root(&self) -> Option<&'static A::Elem> {
        self.root.get()
    }

    /// `RBT_LEFT(e)`.
    pub fn left(e: &A::Elem) -> Option<&'static A::Elem> {
        Self::e(e).left.get()
    }

    /// `RBT_RIGHT(e)`.
    pub fn right(e: &A::Elem) -> Option<&'static A::Elem> {
        Self::e(e).right.get()
    }

    /// `RBT_PARENT(e)`.
    pub fn parent(e: &A::Elem) -> Option<&'static A::Elem> {
        Self::e(e).parent.get()
    }

    /// Whether `e` is in a tree through this tree's entry.
    pub fn is_linked(e: &A::Elem) -> bool {
        Self::e(e).is_linked()
    }

    /// `RBT_MIN`: the first element in order.
    pub fn min(&self) -> Option<&'static A::Elem> {
        let mut n = self.root.get()?;
        while let Some(l) = Self::left(n) {
            n = l;
        }
        Some(n)
    }

    /// `RBT_MAX`: the last element in order.
    pub fn max(&self) -> Option<&'static A::Elem> {
        let mut n = self.root.get()?;
        while let Some(r) = Self::right(n) {
            n = r;
        }
        Some(n)
    }

    /// `RBT_NEXT(e)`: the element after `e` in order.
    pub fn next(e: &A::Elem) -> Option<&'static A::Elem> {
        if let Some(mut n) = Self::right(e) {
            while let Some(l) = Self::left(n) {
                n = l;
            }
            return Some(n);
        }
        // Up while we come from a right child; the first parent reached from a left child.
        let mut child: &A::Elem = e;
        let mut p = Self::parent(e);
        while let Some(pp) = p {
            if Self::e(pp).left.points_to(child) {
                return Some(pp);
            }
            child = pp;
            p = Self::parent(pp);
        }
        None
    }

    /// `RBT_PREV(e)`: the element before `e` in order.
    pub fn prev(e: &A::Elem) -> Option<&'static A::Elem> {
        if let Some(mut n) = Self::left(e) {
            while let Some(r) = Self::right(n) {
                n = r;
            }
            return Some(n);
        }
        let mut child: &A::Elem = e;
        let mut p = Self::parent(e);
        while let Some(pp) = p {
            if Self::e(pp).right.points_to(child) {
                return Some(pp);
            }
            child = pp;
            p = Self::parent(pp);
        }
        None
    }

    /// `RBT_FIND(key)`: the element equal to `key` in the tree's order.
    pub fn find(&self, key: &A::Elem) -> Option<&'static A::Elem> {
        self.find_by(|n| A::cmp(key, n))
    }

    /// `RBT_NFIND(key)`: the first element not before `key`.
    pub fn nfind(&self, key: &A::Elem) -> Option<&'static A::Elem> {
        self.nfind_by(|n| A::cmp(key, n))
    }

    /// `RBT_FIND` without a key element: `key_vs(n)` is the key compared with node `n` (the
    /// C's `cmp(key, n)`).
    pub fn find_by(&self, key_vs: impl Fn(&A::Elem) -> Ordering) -> Option<&'static A::Elem> {
        let mut n = self.root.get();
        while let Some(x) = n {
            n = match key_vs(x) {
                Ordering::Less => Self::left(x),
                Ordering::Greater => Self::right(x),
                Ordering::Equal => return Some(x),
            };
        }
        None
    }

    /// `RBT_NFIND` without a key element (see [`Tree::find_by`]).
    pub fn nfind_by(&self, key_vs: impl Fn(&A::Elem) -> Ordering) -> Option<&'static A::Elem> {
        let mut n = self.root.get();
        let mut res = None;
        while let Some(x) = n {
            n = match key_vs(x) {
                Ordering::Less => {
                    res = Some(x);
                    Self::left(x)
                }
                Ordering::Greater => Self::right(x),
                Ordering::Equal => return Some(x),
            };
        }
        res
    }

    /// The elements in order (`RBT_FOREACH_SAFE`: each successor is read before its element is
    /// handed out, so the caller may remove the element it holds).
    pub fn iter(&self) -> Iter<A> {
        Iter {
            next: self.min(),
            _adapter: PhantomData,
        }
    }

    /// `RBT_INSERT(e)`: puts `e` in the tree and returns `None`, or returns the element already
    /// there that compares equal (and leaves `e` out). Panics when `e` is in a tree.
    pub fn insert(&self, e: &'static A::Elem) -> Option<&'static A::Elem> {
        let ent = Self::e(e);
        // One swap claims the entry, as `List::claim` does: two inserts of the same element
        // cannot both see it unlinked.
        if ent.linked.swap(true, Mem::Relaxed) {
            hal_panic(format_args!("RBT_INSERT: the element is in a tree already"));
        }
        let mut parent = None;
        let mut went = Ordering::Equal;
        let mut n = self.root.get();
        while let Some(x) = n {
            parent = Some(x);
            went = A::cmp(e, x);
            n = match went {
                Ordering::Less => Self::left(x),
                Ordering::Greater => Self::right(x),
                Ordering::Equal => {
                    // An equal element is there: `e` stays out, its claim released.
                    ent.linked.store(false, Mem::Relaxed);
                    return Some(x);
                }
            };
        }
        ent.parent.set(parent);
        ent.left.set(None);
        ent.right.set(None);
        ent.set_color(RED);
        match parent {
            Some(p) if went == Ordering::Less => Self::e(p).left.set(Some(e)),
            Some(p) => Self::e(p).right.set(Some(e)),
            None => self.root.set(Some(e)),
        }
        self.insert_color(e);
        None
    }

    /// `RBT_REMOVE(e)`: takes `e` out of this tree and clears its links. Panics when `e` is in
    /// no tree, or in another tree when that shows (a root that is not this tree's), before
    /// anything is changed.
    pub fn remove(&self, e: &A::Elem) {
        let ent = Self::e(e);
        if !ent.is_linked() {
            hal_panic(format_args!("RBT_REMOVE: the element is in no tree"));
        }
        if ent.parent.is_none() && !self.root.points_to(e) {
            hal_panic(format_args!("RBT_REMOVE: the element is in another tree"));
        }
        let (child, parent, color) = match (ent.left.get(), ent.right.get()) {
            (None, c) | (c, None) => {
                let parent = ent.parent.get();
                if let Some(c) = c {
                    Self::e(c).parent.set(parent);
                }
                self.replace_child(parent, e, c);
                (c, parent, ent.color())
            }
            (Some(_), Some(right)) => {
                // Two children: the successor takes `e`'s place, and the fix-up starts where
                // the successor was.
                let mut s = right;
                while let Some(l) = Self::left(s) {
                    s = l;
                }
                let sent = Self::e(s);
                let s_child = sent.right.get();
                // The successor is below `e`, so it has a parent (checked before any change).
                let Some(sp) = sent.parent.get() else {
                    broken("RBT_REMOVE")
                };
                let color = sent.color();
                if let Some(c) = s_child {
                    Self::e(c).parent.set(Some(sp));
                }
                if Self::e(sp).left.points_to(s) {
                    Self::e(sp).left.set(s_child);
                } else {
                    Self::e(sp).right.set(s_child);
                }
                let fix_parent = if core::ptr::eq(sp, e) {
                    Some(s)
                } else {
                    Some(sp)
                };
                // The successor takes `e`'s links and colour (read after the detach above,
                // which may have changed `e`'s right child).
                let (l, r, p) = (ent.left.get(), ent.right.get(), ent.parent.get());
                sent.left.set(l);
                sent.right.set(r);
                sent.parent.set(p);
                sent.set_color(ent.color());
                self.replace_child(p, e, Some(s));
                match l {
                    Some(l) => Self::e(l).parent.set(Some(s)),
                    None => broken("RBT_REMOVE"),
                }
                if let Some(r) = r {
                    Self::e(r).parent.set(Some(s));
                }
                (s_child, fix_parent, color)
            }
        };
        if color == BLACK {
            self.remove_color(parent, child);
        }
        ent.left.set(None);
        ent.right.set(None);
        ent.parent.set(None);
        ent.linked.store(false, Mem::Relaxed);
    }

    /// `parent`'s link to `old` (the root when `parent` is `None`) now names `new`. Panics when
    /// `old` is a root but not this tree's (it is in another tree).
    fn replace_child(
        &self,
        parent: Option<&'static A::Elem>,
        old: &A::Elem,
        new: Option<&'static A::Elem>,
    ) {
        match parent {
            Some(p) if Self::e(p).left.points_to(old) => Self::e(p).left.set(new),
            Some(p) => Self::e(p).right.set(new),
            None => {
                if !self.root.points_to(old) {
                    hal_panic(format_args!("RBT_REMOVE: the element is in another tree"));
                }
                self.root.set(new);
            }
        }
    }

    /// `rbe_rotate_left(x)`: `x`'s right child takes its place, `x` becoming its left child.
    fn rotate_left(&self, x: &'static A::Elem) {
        let xe = Self::e(x);
        let Some(y) = xe.right.get() else {
            broken("RBT rotate")
        };
        let ye = Self::e(y);
        let yl = ye.left.get();
        xe.right.set(yl);
        if let Some(yl) = yl {
            Self::e(yl).parent.set(Some(x));
        }
        let p = xe.parent.get();
        ye.parent.set(p);
        match p {
            Some(p) if Self::e(p).left.points_to(x) => Self::e(p).left.set(Some(y)),
            Some(p) => Self::e(p).right.set(Some(y)),
            None => self.root.set(Some(y)),
        }
        ye.left.set(Some(x));
        xe.parent.set(Some(y));
    }

    /// `rbe_rotate_right(x)`: the mirror of [`Tree::rotate_left`].
    fn rotate_right(&self, x: &'static A::Elem) {
        let xe = Self::e(x);
        let Some(y) = xe.left.get() else {
            broken("RBT rotate")
        };
        let ye = Self::e(y);
        let yr = ye.right.get();
        xe.left.set(yr);
        if let Some(yr) = yr {
            Self::e(yr).parent.set(Some(x));
        }
        let p = xe.parent.get();
        ye.parent.set(p);
        match p {
            Some(p) if Self::e(p).left.points_to(x) => Self::e(p).left.set(Some(y)),
            Some(p) => Self::e(p).right.set(Some(y)),
            None => self.root.set(Some(y)),
        }
        ye.right.set(Some(x));
        xe.parent.set(Some(y));
    }

    /// Whether `x` is red (an absent node is black).
    fn is_red(x: Option<&A::Elem>) -> bool {
        x.is_some_and(|x| Self::e(x).color() == RED)
    }

    /// `rbe_insert_color`: restores the colouring after `e`, red, was linked as a leaf.
    fn insert_color(&self, e: &'static A::Elem) {
        let mut x = e;
        while let Some(parent) = Self::parent(x)
            && Self::e(parent).color() == RED
        {
            // A red parent is never the root, so the grandparent exists.
            let Some(gparent) = Self::parent(parent) else {
                broken("RBT_INSERT")
            };
            let ge = Self::e(gparent);
            let parent_is_left = ge.left.points_to(parent);
            let uncle = if parent_is_left {
                ge.right.get()
            } else {
                ge.left.get()
            };
            if let Some(u) = uncle
                && Self::e(u).color() == RED
            {
                Self::e(u).set_color(BLACK);
                Self::e(parent).set_color(BLACK);
                ge.set_color(RED);
                x = gparent;
                continue;
            }
            let mut parent = parent;
            if parent_is_left {
                if Self::e(parent).right.points_to(x) {
                    self.rotate_left(parent);
                    core::mem::swap(&mut parent, &mut x);
                }
                Self::e(parent).set_color(BLACK);
                ge.set_color(RED);
                self.rotate_right(gparent);
            } else {
                if Self::e(parent).left.points_to(x) {
                    self.rotate_right(parent);
                    core::mem::swap(&mut parent, &mut x);
                }
                Self::e(parent).set_color(BLACK);
                ge.set_color(RED);
                self.rotate_left(gparent);
            }
        }
        if let Some(r) = self.root.get() {
            Self::e(r).set_color(BLACK);
        }
    }

    /// `rbe_remove_color`: restores the colouring after a black node left the tree; `x` (maybe
    /// absent) took its place under `parent`.
    fn remove_color(&self, parent: Option<&'static A::Elem>, x: Option<&'static A::Elem>) {
        let mut x = x;
        let mut parent = parent;
        while !Self::is_red(x) && !same(&self.root, x) {
            let Some(p) = parent else {
                broken("RBT_REMOVE")
            };
            let pe = Self::e(p);
            if same(&pe.left, x) {
                let Some(mut w) = pe.right.get() else {
                    broken("RBT_REMOVE")
                };
                if Self::e(w).color() == RED {
                    Self::e(w).set_color(BLACK);
                    pe.set_color(RED);
                    self.rotate_left(p);
                    let Some(nw) = pe.right.get() else {
                        broken("RBT_REMOVE")
                    };
                    w = nw;
                }
                let we = Self::e(w);
                if !Self::is_red(we.left.get()) && !Self::is_red(we.right.get()) {
                    we.set_color(RED);
                    x = Some(p);
                    parent = pe.parent.get();
                } else {
                    if !Self::is_red(we.right.get()) {
                        if let Some(wl) = we.left.get() {
                            Self::e(wl).set_color(BLACK);
                        }
                        we.set_color(RED);
                        self.rotate_right(w);
                        let Some(nw) = pe.right.get() else {
                            broken("RBT_REMOVE")
                        };
                        w = nw;
                    }
                    let we = Self::e(w);
                    we.set_color(pe.color());
                    pe.set_color(BLACK);
                    if let Some(wr) = we.right.get() {
                        Self::e(wr).set_color(BLACK);
                    }
                    self.rotate_left(p);
                    x = self.root.get();
                    break;
                }
            } else {
                let Some(mut w) = pe.left.get() else {
                    broken("RBT_REMOVE")
                };
                if Self::e(w).color() == RED {
                    Self::e(w).set_color(BLACK);
                    pe.set_color(RED);
                    self.rotate_right(p);
                    let Some(nw) = pe.left.get() else {
                        broken("RBT_REMOVE")
                    };
                    w = nw;
                }
                let we = Self::e(w);
                if !Self::is_red(we.left.get()) && !Self::is_red(we.right.get()) {
                    we.set_color(RED);
                    x = Some(p);
                    parent = pe.parent.get();
                } else {
                    if !Self::is_red(we.left.get()) {
                        if let Some(wr) = we.right.get() {
                            Self::e(wr).set_color(BLACK);
                        }
                        we.set_color(RED);
                        self.rotate_left(w);
                        let Some(nw) = pe.left.get() else {
                            broken("RBT_REMOVE")
                        };
                        w = nw;
                    }
                    let we = Self::e(w);
                    we.set_color(pe.color());
                    pe.set_color(BLACK);
                    if let Some(wl) = we.left.get() {
                        Self::e(wl).set_color(BLACK);
                    }
                    self.rotate_right(p);
                    x = self.root.get();
                    break;
                }
            }
        }
        if let Some(x) = x {
            Self::e(x).set_color(BLACK);
        }
    }
}

impl<A: TreeAdapter> IntoIterator for &Tree<A> {
    type Item = &'static A::Elem;
    type IntoIter = Iter<A>;

    fn into_iter(self) -> Iter<A> {
        self.iter()
    }
}

impl<A: TreeAdapter> Default for Tree<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// The iterator of [`Tree::iter`].
pub struct Iter<A: TreeAdapter> {
    next: Option<&'static A::Elem>,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: TreeAdapter> Iterator for Iter<A> {
    type Item = &'static A::Elem;

    fn next(&mut self) -> Option<&'static A::Elem> {
        let e = self.next?;
        self.next = Tree::<A>::next(e);
        Some(e)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::collections::BTreeSet;
    use std::vec::Vec;

    use super::*;

    struct Node {
        key: u32,
        t: TreeEntry<Node>,
    }

    crate::tree_adapter!(ByKey: Node, t, |a, b| a.key.cmp(&b.key));

    fn node(key: u32) -> &'static Node {
        Box::leak(Box::new(Node {
            key,
            t: TreeEntry::new(),
        }))
    }

    type T = Tree<ByKey>;

    /// The red-black invariants of the subtree under `n` (parent links, order, no red node
    /// with a red child); its black height.
    fn check(n: Option<&'static Node>, parent: Option<&'static Node>) -> usize {
        let Some(x) = n else {
            return 1;
        };
        assert!(same(&x.t.parent, parent), "parent link of {}", x.key);
        assert!(x.t.is_linked());
        let (l, r) = (T::left(x), T::right(x));
        if x.t.color() == RED {
            assert!(
                !T::is_red(l) && !T::is_red(r),
                "red {} has a red child",
                x.key
            );
        }
        if let Some(l) = l {
            assert!(l.key < x.key);
        }
        if let Some(r) = r {
            assert!(r.key > x.key);
        }
        let (hl, hr) = (check(l, Some(x)), check(r, Some(x)));
        assert_eq!(hl, hr, "black heights under {}", x.key);
        hl + usize::from(x.t.color() == BLACK)
    }

    fn valid(t: &T, oracle: &BTreeSet<u32>) {
        if let Some(r) = t.root() {
            assert_eq!(r.t.color(), BLACK, "the root is black");
        }
        check(t.root(), None);
        let keys: Vec<u32> = t.iter().map(|n| n.key).collect();
        let want: Vec<u32> = oracle.iter().copied().collect();
        assert_eq!(keys, want);
        let back: Vec<u32> = core::iter::successors(t.max(), |n| T::prev(n))
            .map(|n| n.key)
            .collect();
        assert_eq!(back, want.iter().rev().copied().collect::<Vec<_>>());
    }

    /// A small deterministic generator (a 64-bit LCG, Knuth's constants).
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) as u32
        }
    }

    #[test]
    fn against_a_btreeset_over_random_keys() {
        let t = T::new();
        let mut oracle = BTreeSet::new();
        let mut nodes: Vec<&'static Node> = Vec::new();
        let mut rng = Lcg(0x5eed);
        for _ in 0..1000 {
            let key = rng.next() % 2000;
            let n = node(key);
            let dupe = t.insert(n);
            assert_eq!(dupe.is_some(), !oracle.insert(key), "insert {key}");
            if let Some(d) = dupe {
                assert_eq!(d.key, key);
                assert!(!n.t.is_linked(), "a refused element stays out");
            } else {
                nodes.push(n);
            }
            // Remove one element in three, chosen at random.
            if rng.next().is_multiple_of(3) && !nodes.is_empty() {
                let i = rng.next() as usize % nodes.len();
                let gone = nodes.swap_remove(i);
                t.remove(gone);
                assert!(!gone.t.is_linked());
                assert!(oracle.remove(&gone.key));
            }
            valid(&t, &oracle);
        }
        for probe in [0, 1, 999, 1000, 1999, 2000, 5000] {
            assert_eq!(
                t.find_by(|n| probe.cmp(&n.key)).map(|n| n.key),
                oracle.get(&probe).copied()
            );
            assert_eq!(
                t.nfind_by(|n| probe.cmp(&n.key)).map(|n| n.key),
                oracle.range(probe..).next().copied()
            );
        }
        assert_eq!(t.min().map(|n| n.key), oracle.first().copied());
        assert_eq!(t.max().map(|n| n.key), oracle.last().copied());
        // Drain in insertion order of what is left.
        for n in nodes {
            t.remove(n);
            oracle.remove(&n.key);
            valid(&t, &oracle);
        }
        assert!(t.is_empty());
    }

    #[test]
    fn find_and_nfind_with_a_key_element() {
        let t = T::new();
        for k in [10, 20, 30] {
            assert!(t.insert(node(k)).is_none());
        }
        assert_eq!(t.find(node(20)).map(|n| n.key), Some(20));
        assert!(t.find(node(25)).is_none());
        assert_eq!(t.nfind(node(25)).map(|n| n.key), Some(30));
        assert!(t.nfind(node(31)).is_none());
    }

    #[test]
    fn iterating_allows_removing_the_current_element() {
        let t = T::new();
        let mut oracle = BTreeSet::new();
        for k in 0..64 {
            t.insert(node(k));
            oracle.insert(k);
        }
        for n in t.iter() {
            if n.key % 3 != 0 {
                t.remove(n);
                oracle.remove(&n.key);
            }
        }
        valid(&t, &oracle);
    }

    #[test]
    #[should_panic(expected = "in a tree already")]
    fn inserting_a_linked_element_panics() {
        let t = T::new();
        let n = node(1);
        t.insert(n);
        t.insert(n);
    }

    #[test]
    #[should_panic(expected = "in no tree")]
    fn removing_an_unlinked_element_panics() {
        T::new().remove(node(1));
    }

    #[test]
    #[should_panic(expected = "in another tree")]
    fn removing_from_the_wrong_tree_panics() {
        let (a, b) = (T::new(), T::new());
        let n = node(1);
        a.insert(n);
        b.insert(node(2));
        b.remove(n);
    }

    #[test]
    fn a_refused_removal_changes_nothing() {
        // The root of `a`, with two children: removing it from `b` is refused before the
        // successor is detached or any colour changes.
        let (a, b) = (T::new(), T::new());
        let mut oracle = BTreeSet::new();
        for k in [20, 10, 30] {
            a.insert(node(k));
            oracle.insert(k);
        }
        b.insert(node(99));
        let root = a.root().expect("a root");
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.remove(root)));
        assert!(r.is_err(), "refused");
        valid(&a, &oracle);
        valid(&b, &BTreeSet::from([99]));
    }

    #[test]
    fn a_duplicate_is_left_out_and_unclaimed() {
        let t = T::new();
        let first = node(7);
        assert!(t.insert(first).is_none());
        let dup = node(7);
        assert!(t.insert(dup).is_some_and(|x| core::ptr::eq(x, first)));
        assert!(!T::is_linked(dup), "the duplicate's claim is released");
        // So it may go into another tree.
        let other = T::new();
        assert!(other.insert(dup).is_none());
        assert!(T::is_linked(dup));
    }
}
/* </TESTS> */
