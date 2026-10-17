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
//! `List<A>`: the tail queue of `queue(3)` (`TAILQ_*`) over elements that live for ever.
//!
//! Upstream: sys/sys/queue.h @ 3ce1f3f79392
//! LZ: sys/sys/queue.rs@3c62ede68802
//!
//! The behaviour of `<sys/queue.h>`'s tail queue (`TAILQ_INSERT_HEAD`, `_TAIL`, `_BEFORE`,
//! `_AFTER`, `TAILQ_REMOVE`, `TAILQ_CONCAT`, the `TAILQ_FIRST`/`LAST`/`NEXT`/`PREV` walks),
//! re-expressed for the kernel's first users whose elements are never freed (the `vm_page`
//! array and the pmemranges, `kernel::vm`, N3.3). `bsd`'s `sys/sys/queue.rs` stays for the
//! legacy tree (the 50-module rule) until its last user moves.
//!
//! An element carries one [`ListEntry`] per list it can be on, and an adapter type (written
//! by [`list_adapter!`]) names which; the list's head holds the first and the last element.
//! Insertion takes a `&'static` element, so every link is a
//! [`Link`] to something that outlives the list and every operation is safe: a broken
//! precondition (inserting an element that is on a list, removing one that is not, removing
//! from the wrong list when it can be seen) panics, where the C would corrupt the list or
//! fault.
//!
//! What a list does not know is which lock keeps it consistent: the owner holds it (the C's
//! rule), usually as the data of a `Mutexed` or by a documented protocol. All the links are
//! atomics, so a race between two writers can scramble a list but never make a reference
//! dangle.
//!
//! ## Redesign
//! - `TAILQ_HEAD`/`TAILQ_ENTRY` with `tqe_prev` pointing into the previous element's entry
//!   (or into the head) -> a `prev` link to the previous *element* and `first`/`last` in the
//!   head: no pointer into a head, so a head may live anywhere (a static, a struct, a
//!   stack frame) and moves freely, and `TAILQ_INIT` has nothing to do (a new head is empty).
//! - `TAILQ_FOREACH_SAFE` is [`List::iter`]: the successor is read before an element is
//!   handed out, so the caller may remove it.
//! - `TAILQ_REMOVE` of an element on no list (a `NULL` dereference in the C) and
//!   `TAILQ_INSERT_*` of a linked one (a corrupted list) panic: the entry's `linked` flag
//!   says which.

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::assert::hal_panic;
use crate::link::Link;

/// Names the [`ListEntry`] of an element for one kind of list (`TAILQ_ENTRY(type) field`).
/// Implemented by [`list_adapter!`].
pub trait ListAdapter {
    /// The element type.
    type Elem: 'static + Sync;
    /// The element's entry for this kind of list.
    fn entry(e: &Self::Elem) -> &ListEntry<Self::Elem>;
}

/// Declares `$name`, the [`ListAdapter`] of `$elem`'s field `$field`:
/// `list_adapter!(pub Pageq: Page, pageq);`.
#[macro_export]
macro_rules! list_adapter {
    ($(#[$meta:meta])* $vis:vis $name:ident: $elem:ty, $field:ident) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::list::ListAdapter for $name {
            type Elem = $elem;
            fn entry(e: &$elem) -> &$crate::list::ListEntry<$elem> {
                &e.$field
            }
        }
    };
}

/// `TAILQ_ENTRY(type)`: an element's links on one list.
pub struct ListEntry<T: 'static> {
    next: Link<T>,
    prev: Link<T>,
    /// Whether the element is on a list through this entry.
    linked: AtomicBool,
}

impl<T: 'static> ListEntry<T> {
    /// An entry on no list.
    pub const fn new() -> Self {
        Self {
            next: Link::new(),
            prev: Link::new(),
            linked: AtomicBool::new(false),
        }
    }

    /// Whether the element is on a list through this entry.
    pub fn is_linked(&self) -> bool {
        self.linked.load(Ordering::Relaxed)
    }
}

impl<T: 'static> Default for ListEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `TAILQ_HEAD(name, type)`: a tail queue of `A::Elem`s.
pub struct List<A: ListAdapter> {
    first: Link<A::Elem>,
    last: Link<A::Elem>,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: ListAdapter> List<A> {
    /// `TAILQ_HEAD_INITIALIZER`: an empty list.
    pub const fn new() -> Self {
        Self {
            first: Link::new(),
            last: Link::new(),
            _adapter: PhantomData,
        }
    }

    /// `TAILQ_EMPTY`.
    pub fn is_empty(&self) -> bool {
        self.first.is_none()
    }

    /// `TAILQ_FIRST`.
    pub fn first(&self) -> Option<&'static A::Elem> {
        self.first.get()
    }

    /// `TAILQ_LAST`.
    pub fn last(&self) -> Option<&'static A::Elem> {
        self.last.get()
    }

    /// `TAILQ_NEXT(e)`: the element after `e`, `None` at the end (or when `e` is on no list).
    pub fn next(e: &A::Elem) -> Option<&'static A::Elem> {
        A::entry(e).next.get()
    }

    /// `TAILQ_PREV(e)`: the element before `e`, `None` at the start.
    pub fn prev(e: &A::Elem) -> Option<&'static A::Elem> {
        A::entry(e).prev.get()
    }

    /// Whether `e` is on a list through this list's entry.
    pub fn is_linked(e: &A::Elem) -> bool {
        A::entry(e).is_linked()
    }

    /// Marks `e`'s entry linked; panics when it was (`what` names the operation).
    fn claim(e: &A::Elem, what: &str) {
        if A::entry(e).linked.swap(true, Ordering::Relaxed) {
            hal_panic(format_args!("{what}: the element is on a list already"));
        }
    }

    /// `TAILQ_INSERT_HEAD`: `e` becomes the first element. Panics when `e` is on a list.
    pub fn insert_head(&self, e: &'static A::Elem) {
        Self::claim(e, "TAILQ_INSERT_HEAD");
        let ent = A::entry(e);
        let old = self.first.get();
        ent.next.set(old);
        ent.prev.set(None);
        match old {
            Some(f) => A::entry(f).prev.set(Some(e)),
            None => self.last.set(Some(e)),
        }
        self.first.set(Some(e));
    }

    /// `TAILQ_INSERT_TAIL`: `e` becomes the last element. Panics when `e` is on a list.
    pub fn insert_tail(&self, e: &'static A::Elem) {
        Self::claim(e, "TAILQ_INSERT_TAIL");
        let ent = A::entry(e);
        let old = self.last.get();
        ent.prev.set(old);
        ent.next.set(None);
        match old {
            Some(l) => A::entry(l).next.set(Some(e)),
            None => self.first.set(Some(e)),
        }
        self.last.set(Some(e));
    }

    /// `TAILQ_INSERT_BEFORE(at, e)`: `e` goes right before `at`, which must be on this list.
    /// Panics when `e` is on a list or `at` is not.
    pub fn insert_before(&self, at: &'static A::Elem, e: &'static A::Elem) {
        if !A::entry(at).is_linked() {
            hal_panic(format_args!(
                "TAILQ_INSERT_BEFORE: the position is on no list"
            ));
        }
        Self::claim(e, "TAILQ_INSERT_BEFORE");
        let ent = A::entry(e);
        let p = A::entry(at).prev.get();
        ent.prev.set(p);
        ent.next.set(Some(at));
        match p {
            Some(p) => A::entry(p).next.set(Some(e)),
            None => {
                self.check_end(self.first.points_to(at), "TAILQ_INSERT_BEFORE");
                self.first.set(Some(e));
            }
        }
        A::entry(at).prev.set(Some(e));
    }

    /// `TAILQ_INSERT_AFTER(at, e)`: `e` goes right after `at`, which must be on this list.
    /// Panics when `e` is on a list or `at` is not.
    pub fn insert_after(&self, at: &'static A::Elem, e: &'static A::Elem) {
        if !A::entry(at).is_linked() {
            hal_panic(format_args!(
                "TAILQ_INSERT_AFTER: the position is on no list"
            ));
        }
        Self::claim(e, "TAILQ_INSERT_AFTER");
        let ent = A::entry(e);
        let n = A::entry(at).next.get();
        ent.next.set(n);
        ent.prev.set(Some(at));
        match n {
            Some(n) => A::entry(n).prev.set(Some(e)),
            None => {
                self.check_end(self.last.points_to(at), "TAILQ_INSERT_AFTER");
                self.last.set(Some(e));
            }
        }
        A::entry(at).next.set(Some(e));
    }

    /// An end of an element that has no neighbour on that side must be this list's end: when
    /// it is not, the element is on another list (the one wrong use that can be seen).
    fn check_end(&self, ok: bool, what: &str) {
        if !ok {
            hal_panic(format_args!("{what}: the element is on another list"));
        }
    }

    /// `TAILQ_REMOVE(e)`: takes `e` off this list and clears its links. Panics when `e` is on
    /// no list, or on another list when that shows (an end that is not this list's).
    pub fn remove(&self, e: &A::Elem) {
        let ent = A::entry(e);
        if !ent.is_linked() {
            hal_panic(format_args!("TAILQ_REMOVE: the element is on no list"));
        }
        let n = ent.next.get();
        let p = ent.prev.get();
        match n {
            Some(n) => A::entry(n).prev.set(p),
            None => {
                self.check_end(self.last.points_to(e), "TAILQ_REMOVE");
                self.last.set(p);
            }
        }
        match p {
            Some(p) => A::entry(p).next.set(n),
            None => {
                self.check_end(self.first.points_to(e), "TAILQ_REMOVE");
                self.first.set(n);
            }
        }
        ent.next.set(None);
        ent.prev.set(None);
        ent.linked.store(false, Ordering::Relaxed);
    }

    /// The first element, taken off the list (`TAILQ_FIRST` then `TAILQ_REMOVE`).
    pub fn pop_head(&self) -> Option<&'static A::Elem> {
        let e = self.first.get()?;
        self.remove(e);
        Some(e)
    }

    /// `TAILQ_CONCAT(self, other)`: every element of `other`, in order, appended to this
    /// list; `other` is left empty.
    pub fn concat(&self, other: &Self) {
        let Some(of) = other.first.get() else {
            return;
        };
        let ol = other.last.get();
        match self.last.get() {
            Some(l) => {
                A::entry(l).next.set(Some(of));
                A::entry(of).prev.set(Some(l));
            }
            None => self.first.set(Some(of)),
        }
        self.last.set(ol);
        other.first.set(None);
        other.last.set(None);
    }

    /// `TAILQ_FOREACH_SAFE`: the elements in order, each successor read before its element
    /// is handed out (so the caller may remove the element it holds).
    pub fn iter(&self) -> Iter<A> {
        Iter {
            next: self.first.get(),
            _adapter: PhantomData,
        }
    }
}

impl<A: ListAdapter> Default for List<A> {
    fn default() -> Self {
        Self::new()
    }
}

/// The iterator of [`List::iter`].
pub struct Iter<A: ListAdapter> {
    next: Option<&'static A::Elem>,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: ListAdapter> Iterator for Iter<A> {
    type Item = &'static A::Elem;

    fn next(&mut self) -> Option<&'static A::Elem> {
        let e = self.next?;
        self.next = A::entry(e).next.get();
        Some(e)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::vec::Vec;

    use super::*;

    struct Item {
        v: u32,
        q: ListEntry<Item>,
        r: ListEntry<Item>,
    }

    crate::list_adapter!(Q: Item, q);
    crate::list_adapter!(R: Item, r);

    fn item(v: u32) -> &'static Item {
        Box::leak(Box::new(Item {
            v,
            q: ListEntry::new(),
            r: ListEntry::new(),
        }))
    }

    fn values(l: &List<Q>) -> Vec<u32> {
        l.iter().map(|i| i.v).collect()
    }

    #[test]
    fn inserts_at_both_ends_and_around() {
        let l = List::<Q>::new();
        assert!(l.is_empty());
        let (a, b, c, d, e) = (item(1), item(2), item(3), item(4), item(5));
        l.insert_tail(b);
        l.insert_head(a);
        l.insert_tail(d);
        l.insert_before(d, c);
        l.insert_after(d, e);
        assert_eq!(values(&l), [1, 2, 3, 4, 5]);
        assert!(core::ptr::eq(l.first().unwrap(), a));
        assert!(core::ptr::eq(l.last().unwrap(), e));
        assert_eq!(List::<Q>::prev(c).map(|i| i.v), Some(2));
        assert_eq!(List::<Q>::next(c).map(|i| i.v), Some(4));
        let back: Vec<u32> = core::iter::successors(l.last(), |i| List::<Q>::prev(i))
            .map(|i| i.v)
            .collect();
        assert_eq!(back, [5, 4, 3, 2, 1]);
    }

    #[test]
    fn removal_anywhere_and_pop() {
        let l = List::<Q>::new();
        let items: Vec<&'static Item> = (0..5).map(item).collect();
        for &i in &items {
            l.insert_tail(i);
        }
        l.remove(items[2]);
        assert!(!List::<Q>::is_linked(items[2]));
        assert_eq!(List::<Q>::next(items[2]).map(|i| i.v), None);
        l.remove(items[0]);
        l.remove(items[4]);
        assert_eq!(values(&l), [1, 3]);
        assert_eq!(l.pop_head().map(|i| i.v), Some(1));
        assert_eq!(l.pop_head().map(|i| i.v), Some(3));
        assert!(l.pop_head().is_none());
        assert!(l.is_empty() && l.last().is_none());
        // Off the list, an element goes onto it again.
        l.insert_head(items[2]);
        assert_eq!(values(&l), [2]);
    }

    #[test]
    fn iterating_allows_removing_the_current_element() {
        let l = List::<Q>::new();
        for v in 0..6 {
            l.insert_tail(item(v));
        }
        for i in l.iter() {
            if i.v % 2 == 0 {
                l.remove(i);
            }
        }
        assert_eq!(values(&l), [1, 3, 5]);
    }

    #[test]
    fn one_element_on_two_kinds_of_list_and_three_static_lists() {
        static L1: List<Q> = List::new();
        static L2: List<Q> = List::new();
        static L3: List<R> = List::new();
        let (a, b) = (item(10), item(20));
        L1.insert_tail(a);
        L2.insert_tail(b);
        L3.insert_tail(a);
        L3.insert_tail(b);
        L1.concat(&L2);
        assert!(L2.is_empty());
        assert_eq!(values(&L1), [10, 20]);
        assert_eq!(L3.iter().map(|i| i.v).collect::<Vec<_>>(), [10, 20]);
        L3.remove(a);
        assert!(List::<Q>::is_linked(a), "the other entry is untouched");
        assert_eq!(L1.pop_head().map(|i| i.v), Some(10));
        assert_eq!(L1.pop_head().map(|i| i.v), Some(20));
        assert_eq!(L3.pop_head().map(|i| i.v), Some(20));
    }

    #[test]
    #[should_panic(expected = "on a list already")]
    fn inserting_a_linked_element_panics() {
        let l = List::<Q>::new();
        let a = item(1);
        l.insert_tail(a);
        l.insert_head(a);
    }

    #[test]
    #[should_panic(expected = "on no list")]
    fn removing_an_unlinked_element_panics() {
        let l = List::<Q>::new();
        l.remove(item(1));
    }

    #[test]
    #[should_panic(expected = "on another list")]
    fn removing_from_the_wrong_list_panics() {
        let (l1, l2) = (List::<Q>::new(), List::<Q>::new());
        let a = item(1);
        l1.insert_tail(a);
        l2.insert_tail(item(2));
        l2.remove(a);
    }

    #[test]
    #[should_panic(expected = "the position is on no list")]
    fn inserting_before_an_unlinked_element_panics() {
        let l = List::<Q>::new();
        l.insert_before(item(1), item(2));
    }
}
/* </TESTS> */
