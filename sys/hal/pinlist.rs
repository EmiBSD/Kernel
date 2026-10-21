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

/*
 * Copyright (c) 2001 Thomas Nordin <nordin@openbsd.org>
 * Copyright (c) 2000-2001 Artur Grabowski <art@openbsd.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES,
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY
 * AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
 * THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL  DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `PinList<A>` and `PinRing<A>`: intrusive lists of pinned elements whose owners free them,
//! the tail queue of `queue(3)` (`TAILQ_*`) and `kern_timeout.c`'s circular queue (`CIRCQ_*`).
//!
//! Upstream: sys/sys/queue.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_timeout.c @ 3ce1f3f79392
//! LZ: sys/sys/queue.rs@3c62ede68802
//! LZ: sys/kern/kern_timeout.rs@3c62ede68802
//!
//! The C links a `struct timeout` or a `struct task` that its owner embeds anywhere (a softc,
//! a stack frame, a pool item) into a list a lock keeps, and frees it when it pleases, trusting
//! itself to unlink it first. [`crate::list::List`] takes `&'static` elements and
//! [`crate::owned::OwnedTree`] owns its elements' boxes; neither fits an element its owner
//! keeps by value. These lists take the element **pinned** (`Pin<&T>`): its memory is not
//! reused before its `Drop` has run, and the drop of its [`PinEntry`] panics while it is linked,
//! so a link never names freed memory. What the C writes as a convention ("timeout_del before
//! free") becomes a checked precondition that stops the kernel instead of corrupting it.
//!
//! **Two shapes.** [`PinList`] is the tail queue with the first and last element in its head:
//! no element points at the head, so a head may move freely (a field of a pool item); removal
//! names the list, and an element at an end that is not this list's end is refused.
//! [`PinRing`] is the C's circular queue with a sentinel: the head is a node of the ring, so an
//! element leaves without naming its list ([`PinRing::remove`], `CIRCQ_REMOVE`), which is what
//! `timeout_del` needs (a timeout sits in one of 2052 queues it does not record); the head
//! must therefore not move once an element points at it ([`Pin`] of the head, a `static` by
//! [`crate::pin::pin_static`]).
//!
//! **One lock per family.** Every operation takes a witness of the lock that keeps the lists
//! (`hal::rwlock::Witness`: a `MutexedGuard`, a reader-writer lock's witness): reads take any
//! witness, changes the whole exclusive one (`Divisible`) mutably, so no reference an earlier
//! read handed out is alive across a change. A list is bound to the identity of the first
//! lock whose witness it sees and refuses any other from then on; an element records the
//! identity of the lock of the list it is on. Lists bound to one lock share their elements'
//! identity, so moving an element or splicing a whole list ([`PinRing::concat`],
//! `CIRCQ_CONCAT`, `MOVEBUCKET`) between lists of one owner retags nothing: `concat` and
//! `remove` are a few pointer writes, no walk.
//!
//! ## Soundness
//!
//! The two derefs of this module (a link to the entry it names, an entry to the element that
//! holds it) and the `Send`/`Sync` of [`PinEntry`] rest on these invariants, which every
//! function here keeps:
//! - I1, pinning: an element is linked only through `Pin<&T>`, its entry checked to lie within
//!   the element's own bytes (so it is dropped with the element), and an element type is not
//!   `Unpin` ([`pin_adapter!`] refuses one at compile time: the adapters are written only
//!   through it, as `hal`'s other adapters are). By `Pin`'s drop guarantee the element's
//!   memory is not invalidated or reused before its drop runs, and [`PinEntry`]'s drop panics
//!   while the entry is linked (`panic(9)` in a kernel image, which never returns: the memory
//!   is never given back), so a linked element is alive. A ring's head is pinned likewise and
//!   its drop panics while the ring is not empty.
//! - I2, the lock: a list's head cells and the link cells of the elements on it are read and
//!   written only by holders of a witness of the lock the list is bound to (checked once per
//!   operation, against the list's identity and, for an element named by the caller, against
//!   the identity in its entry); an element's neighbours are on the same list. So the cells
//!   are never touched by two CPUs at once, but by readers under a shared witness, which no
//!   writer accompanies. An element's identity moves from 0 to a lock's under a
//!   compare-and-swap (`Acquire`) and back to 0 last of all (`Release`), so the cells it hands
//!   over from one lock's holders to another's are seen whole.
//! - I3, links: every link of a linked element, and of a head, names a linked element's entry
//!   on the same list (or, in a ring, the ring's own head); an entry's `this` names the element
//!   it was linked for, with that element's provenance.
//!
//! So a link read under I2 names a live entry (I1, I3), and a reference handed out borrows the
//! list and the witness: the element cannot be unlinked (a change needs the witness mutably)
//! nor, linked, freed while it lives.
//!
//! What stays a convention: an element is linked or unlinked by the owner of its memory
//! before that memory goes, and a type whose values unlink themselves does it in its own
//! `Drop` (which runs before its fields' drops) under the lock, as `kernel::timeout::Timeout`
//! does; the panic of a forgotten unlink is the last line, not the protocol. A thread that
//! exits while a value pinned on its stack is linked breaks `Pin`'s guarantee in the exit
//! primitive, not here: kernel code unlinks its stack nodes before calling it.
//!
//! ## Redesign
//! - `TAILQ_ENTRY`/`struct circq` with raw `next`/`prev` and `tqe_prev` into the head, linked
//!   and unlinked in `unsafe` blocks whose comment says the element stays alive (LZ's
//!   `sys/sys/queue.rs`, `kern_timeout.rs`'s `circq_*`) -> [`PinEntry`], linked through
//!   `Pin<&T>` and checked unlinked at its drop: a use after free cannot be written in safe
//!   code. `timeout_from_circq`'s cast is the entry's `this`, made from the pinned reference.
//! - `CIRCQ_INIT` at run time (`timeout_startup`) -> a constant head whose empty links mean
//!   "itself": nothing to initialise, and the head's own address is taken only once it is
//!   pinned.
//! - The lock that keeps the lists, a comment in the C (`[T]`), is a witness every operation
//!   takes and checks.

use core::cell::Cell;
use core::marker::{PhantomData, PhantomPinned};
use core::pin::Pin;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::assert::hal_panic;
use crate::rwlock::{Divisible, Witness};

/// Names the [`PinEntry`] of an element for one family of lists (`TAILQ_ENTRY(type) field`).
/// Implemented by [`pin_adapter!`] alone, which refuses an `Unpin` element.
pub trait PinAdapter {
    /// The element type: shared between CPUs through the lists, so `Sync`.
    type Elem: Sync;
    /// The element's entry for this family of lists.
    fn entry(e: &Self::Elem) -> &PinEntry<Self::Elem>;
}

/// Declares `$name`, the [`PinAdapter`] of `$elem`'s field `$field`, and refuses at compile
/// time an `$elem` that is `Unpin` (the module's I1): `pin_adapter!(pub ToList: Timeout,
/// entry);`.
#[macro_export]
macro_rules! pin_adapter {
    ($(#[$meta:meta])* $vis:vis $name:ident: $elem:ty, $field:ident) => {
        $(#[$meta])*
        $vis struct $name;

        impl $crate::pinlist::PinAdapter for $name {
            type Elem = $elem;
            fn entry(e: &$elem) -> &$crate::pinlist::PinEntry<$elem> {
                &e.$field
            }
        }

        // An `Unpin` element makes the call below ambiguous (two impls apply): refused.
        const _: fn() = || {
            trait NotUnpin<M> {
                fn not_unpin() {}
            }
            impl<T: ?Sized> NotUnpin<()> for T {}
            impl<T: ?Sized + ::core::marker::Unpin> NotUnpin<u8> for T {}
            <$elem as NotUnpin<_>>::not_unpin();
        };
    };
}

/// An element's links on one list of a family (`TAILQ_ENTRY`, `struct circq`); also a ring's
/// head. Not `Unpin`: the element that holds it is pinned while it is linked.
pub struct PinEntry<T> {
    /// The next node (`None` past a tail queue's end, or in an empty ring's head).
    next: Cell<Option<NonNull<PinEntry<T>>>>,
    /// The previous node.
    prev: Cell<Option<NonNull<PinEntry<T>>>>,
    /// The element this entry was linked for (`None` for a head, or unlinked).
    this: Cell<Option<NonNull<T>>>,
    /// The identity of the lock of the list the element is on; 0 when it is on none.
    owner: AtomicUsize,
    _pinned: PhantomPinned,
}

impl<T> PinEntry<T> {
    /// An entry on no list (`{ NULL, NULL }`).
    pub const fn new() -> Self {
        Self {
            next: Cell::new(None),
            prev: Cell::new(None),
            this: Cell::new(None),
            owner: AtomicUsize::new(0),
            _pinned: PhantomPinned,
        }
    }

    /// Whether the element is on a list through this entry (a snapshot).
    pub fn is_linked(&self) -> bool {
        self.owner.load(Ordering::Relaxed) != 0
    }

    /// The address of this entry, as the links name it.
    #[inline]
    fn addr(&self) -> NonNull<PinEntry<T>> {
        NonNull::from(self)
    }

    /// The links cleared and the entry given up (I2: the identity goes back to 0 last).
    #[inline]
    fn release(&self) {
        self.next.set(None);
        self.prev.set(None);
        self.this.set(None);
        self.owner.store(0, Ordering::Release);
    }
}

impl<T> Default for PinEntry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for PinEntry<T> {
    /// The last line of I1: an element dropped while linked stops the kernel.
    fn drop(&mut self) {
        if self.owner.load(Ordering::Acquire) != 0 {
            linked_drop();
        }
    }
}

// SAFETY: the cells of an entry are read and written only by holders of a witness of the lock
// whose identity the entry records (an element's) or its list is bound to (a head's), I2, so
// two CPUs never touch them at once but as readers; `this` names a `T` that other CPUs reach
// only shared (`T: Sync`), and the identity is an atomic. Sharing `&PinEntry<T>` across CPUs
// is therefore sound for `T: Sync`.
unsafe impl<T: Sync> Sync for PinEntry<T> {}

// SAFETY: moving an entry to another CPU moves its links and `this`, which name entries and an
// element shared between CPUs (`T: Sync`); a linked entry never moves (I1: its element is
// pinned), and an unlinked one carries no link. The drop on another CPU reads the atomic
// identity alone.
unsafe impl<T: Sync> Send for PinEntry<T> {}

/// The entry a link names, for as long as the caller's borrows of the list and the witness
/// (I2, I3).
#[inline]
fn entry_at<'a, T>(p: NonNull<PinEntry<T>>) -> &'a PinEntry<T> {
    // SAFETY: `p` was read from a link (a head's or a linked entry's) by a holder of the
    // list's lock (I2), so it names a linked element's entry or a pinned ring head (I3), which
    // is alive (I1) and, being pinned, still where the link was made. Only shared references
    // to entries are ever made (their cells are `Cell`s and an atomic), so this one aliases no
    // `&mut`. Every caller ties `'a` to its borrows of the list and of the witness: while they
    // last no change can unlink the node (a change takes the witness mutably), so it stays
    // linked, hence alive, for `'a`.
    unsafe { p.as_ref() }
}

/// The element a linked entry was linked for, for as long as the caller's borrows of the list
/// and the witness.
#[inline]
fn elem_of<'a, T>(n: &PinEntry<T>) -> &'a T {
    let Some(t) = n.this.get() else {
        hal_panic(format_args!(
            "PinList: a link names a head or an unlinked entry"
        ));
    };
    // SAFETY: `this` was set by `link` from the `Pin<&T>` the element was linked through, with
    // its whole provenance (I3), and the entry is still linked (it was reached through a link
    // under the lock, I2), so the element is alive and in place (I1). Only shared references
    // to elements are made here; `'a` is tied by every caller to its borrows of the list and
    // of the witness, during which the element stays linked.
    unsafe { t.as_ref() }
}

/// The element's entry claimed for the lock `id`, `this` set: panics when the adapter names an
/// entry outside the element (I1) or the element is on a list already.
#[inline]
fn link<A: PinAdapter>(e: Pin<&A::Elem>, id: usize, what: &str) -> NonNull<PinEntry<A::Elem>> {
    match try_link::<A>(e, id, what) {
        Some(n) => n,
        None => hal_panic(format_args!("{what}: the element is on a list already")),
    }
}

/// [`link`], `None` (nothing changed) when the element is on a list already; panics when the
/// adapter names an entry outside the element (I1).
#[inline]
fn try_link<A: PinAdapter>(
    e: Pin<&A::Elem>,
    id: usize,
    what: &str,
) -> Option<NonNull<PinEntry<A::Elem>>> {
    let e = e.get_ref();
    let ent = A::entry(e);
    let ea = ptr::from_ref(ent).addr();
    let ta = ptr::from_ref(e).addr();
    let inside = ea >= ta
        && ea
            .checked_add(size_of::<PinEntry<A::Elem>>())
            .is_some_and(|end| end <= ta.saturating_add(size_of::<A::Elem>()));
    if !inside {
        hal_panic(format_args!("{what}: the entry is not within the element"));
    }
    if ent
        .owner
        .compare_exchange(0, id, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return None;
    }
    ent.this.set(Some(NonNull::from(e)));
    Some(ent.addr())
}

/// The entry of `e`, which the caller names as being on a list of the lock `id`: panics
/// otherwise (on no list, or on a list of another lock, whose cells this caller may not touch).
#[inline]
fn linked_entry<'e, A: PinAdapter>(e: &'e A::Elem, id: usize, what: &str) -> &'e PinEntry<A::Elem> {
    let ent = A::entry(e);
    if ent.owner.load(Ordering::Relaxed) != id {
        hal_panic(format_args!(
            "{what}: the element is not on a list of this lock"
        ));
    }
    ent
}

/// Binds a list to the lock `id` on first use; panics when it is bound to another (I2).
#[inline]
fn bind(key: &AtomicUsize, id: usize, what: &str) {
    let k = key.load(Ordering::Relaxed);
    if k == id {
        return;
    }
    if k == 0
        && key
            .compare_exchange(0, id, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    {
        return;
    }
    another_lock(what);
}

/// [`bind`]'s refusal, out of the operations' line.
#[cold]
#[inline(never)]
fn another_lock(what: &str) -> ! {
    hal_panic(format_args!("{what}: a list of another lock"))
}

/// [`PinEntry`]'s drop of a linked element, out of line.
#[cold]
#[inline(never)]
fn linked_drop() -> ! {
    hal_panic(format_args!("PinEntry: an element dropped while linked"))
}

/// The identity of `w`'s lock, as lists and entries record it (never 0).
#[inline]
fn id_of<W: Witness + ?Sized>(w: &W) -> usize {
    w.lock_id().raw()
}

/// `TAILQ_HEAD(name, type)` of pinned elements: the first and the last element in the head, no
/// element pointing at it, so the head moves freely. Removal names the list
/// ([`PinList::remove`]).
pub struct PinList<A: PinAdapter> {
    /// `next` is the first element's entry, `prev` the last's; `this` and the identity unused.
    head: PinEntry<A::Elem>,
    /// The identity of the lock the list is bound to; 0 before its first use.
    key: AtomicUsize,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: PinAdapter> PinList<A> {
    /// `TAILQ_HEAD_INITIALIZER`: an empty list, bound to no lock yet.
    pub const fn new() -> Self {
        Self {
            head: PinEntry::new(),
            key: AtomicUsize::new(0),
            _adapter: PhantomData,
        }
    }

    /// `TAILQ_EMPTY`, for a holder of the list's lock.
    pub fn is_empty<W: Witness + ?Sized>(&self, w: &W) -> bool {
        bind(&self.key, id_of(w), "TAILQ_EMPTY");
        self.head.next.get().is_none()
    }

    /// `TAILQ_FIRST`: borrows the list and the witness, so the element stays linked while the
    /// reference lives.
    pub fn first<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a A::Elem> {
        bind(&self.key, id_of(w), "TAILQ_FIRST");
        self.head.next.get().map(|p| elem_of(entry_at(p)))
    }

    /// `TAILQ_LAST`.
    pub fn last<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a A::Elem> {
        bind(&self.key, id_of(w), "TAILQ_LAST");
        self.head.prev.get().map(|p| elem_of(entry_at(p)))
    }

    /// `TAILQ_NEXT(e)`: the element after `e`, which must be on a list of this lock.
    pub fn next<'a, W: Witness + ?Sized>(
        &'a self,
        w: &'a W,
        e: &'a A::Elem,
    ) -> Option<&'a A::Elem> {
        let id = id_of(w);
        bind(&self.key, id, "TAILQ_NEXT");
        linked_entry::<A>(e, id, "TAILQ_NEXT")
            .next
            .get()
            .map(|p| elem_of(entry_at(p)))
    }

    /// `TAILQ_FOREACH`: the elements in order.
    pub fn iter<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> ListIter<'a, A> {
        bind(&self.key, id_of(w), "TAILQ_FOREACH");
        ListIter {
            next: self.head.next.get(),
            _borrow: PhantomData,
        }
    }

    /// `TAILQ_INSERT_HEAD`: `e` becomes the first element. Panics when it is on a list.
    pub fn insert_head<W: Divisible + ?Sized>(&self, w: &mut W, e: Pin<&A::Elem>) {
        let id = id_of(w);
        bind(&self.key, id, "TAILQ_INSERT_HEAD");
        let n = link::<A>(e, id, "TAILQ_INSERT_HEAD");
        let ent = entry_at(n);
        let old = self.head.next.get();
        ent.next.set(old);
        ent.prev.set(None);
        match old {
            Some(f) => entry_at(f).prev.set(Some(n)),
            None => self.head.prev.set(Some(n)),
        }
        self.head.next.set(Some(n));
    }

    /// `TAILQ_INSERT_TAIL`: `e` becomes the last element. Panics when it is on a list.
    pub fn insert_tail<W: Divisible + ?Sized>(&self, w: &mut W, e: Pin<&A::Elem>) {
        let id = id_of(w);
        bind(&self.key, id, "TAILQ_INSERT_TAIL");
        let n = link::<A>(e, id, "TAILQ_INSERT_TAIL");
        self.append(n);
    }

    /// `TAILQ_INSERT_TAIL` of an element that may be on a list already (of this lock, or of
    /// another whose holder linked it meanwhile): `false`, nothing changed, when it is. The
    /// test that decides is the claim of its entry itself, so it races with no other insert
    /// (task(9)'s `task_add` on two queues at once, which the C would corrupt).
    pub fn try_insert_tail<W: Divisible + ?Sized>(&self, w: &mut W, e: Pin<&A::Elem>) -> bool {
        let id = id_of(w);
        bind(&self.key, id, "TAILQ_INSERT_TAIL");
        match try_link::<A>(e, id, "TAILQ_INSERT_TAIL") {
            Some(n) => {
                self.append(n);
                true
            }
            None => false,
        }
    }

    /// Links the claimed entry `n` last.
    #[inline]
    fn append(&self, n: NonNull<PinEntry<A::Elem>>) {
        let ent = entry_at(n);
        let old = self.head.prev.get();
        ent.prev.set(old);
        ent.next.set(None);
        match old {
            Some(l) => entry_at(l).next.set(Some(n)),
            None => self.head.next.set(Some(n)),
        }
        self.head.prev.set(Some(n));
    }

    /// `TAILQ_REMOVE(e)`: takes `e` off this list. Panics when it is on no list of this lock, or
    /// at an end that is not this list's (on another list), before anything is changed.
    pub fn remove<W: Divisible + ?Sized>(&self, w: &mut W, e: &A::Elem) {
        let id = id_of(w);
        bind(&self.key, id, "TAILQ_REMOVE");
        let ent = linked_entry::<A>(e, id, "TAILQ_REMOVE");
        self.unlink(ent);
    }

    /// `TAILQ_FIRST` then `TAILQ_REMOVE`: whether there was a first element to take off.
    pub fn remove_first<W: Divisible + ?Sized>(&self, w: &mut W) -> bool {
        bind(&self.key, id_of(w), "TAILQ_REMOVE");
        match self.head.next.get() {
            Some(f) => {
                self.unlink(entry_at(f));
                true
            }
            None => false,
        }
    }

    /// Unlinks `ent`, which is linked under this list's lock, checking its ends first.
    fn unlink(&self, ent: &PinEntry<A::Elem>) {
        let me = Some(ent.addr());
        let n = ent.next.get();
        let p = ent.prev.get();
        // Both ends first: a refused removal leaves every list as it was.
        if (n.is_none() && self.head.prev.get() != me)
            || (p.is_none() && self.head.next.get() != me)
        {
            hal_panic(format_args!("TAILQ_REMOVE: the element is on another list"));
        }
        match n {
            Some(n) => entry_at(n).prev.set(p),
            None => self.head.prev.set(p),
        }
        match p {
            Some(p) => entry_at(p).next.set(n),
            None => self.head.next.set(n),
        }
        ent.release();
    }

    /// `TAILQ_CONCAT(self, other)`: `other`'s elements appended, in order; `other` left empty.
    /// A few pointer writes: both lists are bound to one lock, whose identity the elements
    /// already carry. Panics when `other` is this list.
    pub fn concat<W: Divisible + ?Sized>(&self, w: &mut W, other: &Self) {
        let id = id_of(w);
        bind(&self.key, id, "TAILQ_CONCAT");
        bind(&other.key, id, "TAILQ_CONCAT");
        if ptr::eq(self, other) {
            hal_panic(format_args!("TAILQ_CONCAT: a list onto itself"));
        }
        let Some(of) = other.head.next.get() else {
            return;
        };
        let ol = other.head.prev.get();
        match self.head.prev.get() {
            Some(l) => {
                entry_at(l).next.set(Some(of));
                entry_at(of).prev.set(Some(l));
            }
            None => self.head.next.set(Some(of)),
        }
        self.head.prev.set(ol);
        other.head.next.set(None);
        other.head.prev.set(None);
    }
}

impl<A: PinAdapter> Default for PinList<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: PinAdapter> Drop for PinList<A> {
    /// A list dropped with elements on it would strand them linked (their drops would panic
    /// later): refused now.
    fn drop(&mut self) {
        if self.head.next.get().is_some() {
            hal_panic(format_args!("PinList: dropped with elements on it"));
        }
    }
}

/// The iterator of [`PinList::iter`].
pub struct ListIter<'a, A: PinAdapter> {
    next: Option<NonNull<PinEntry<A::Elem>>>,
    _borrow: PhantomData<&'a A::Elem>,
}

impl<'a, A: PinAdapter> Iterator for ListIter<'a, A> {
    type Item = &'a A::Elem;

    fn next(&mut self) -> Option<&'a A::Elem> {
        let ent = entry_at::<A::Elem>(self.next?);
        self.next = ent.next.get();
        Some(elem_of(ent))
    }
}

/// `struct circq` as a list head: the C's circular queue of pinned elements, whose head is a
/// node of the ring. An element leaves without naming its list ([`PinRing::remove`]); the head
/// must be pinned to take elements.
pub struct PinRing<A: PinAdapter> {
    /// The sentinel: its links name the first and the last element, or are empty (`None`, or
    /// the head itself) when the ring is.
    head: PinEntry<A::Elem>,
    /// The identity of the lock the ring is bound to; 0 before its first use.
    key: AtomicUsize,
    _adapter: PhantomData<fn() -> A>,
}

impl<A: PinAdapter> PinRing<A> {
    /// An empty ring (the C's `CIRCQ_INIT` is not needed: empty links mean "itself").
    pub const fn new() -> Self {
        Self {
            head: PinEntry::new(),
            key: AtomicUsize::new(0),
            _adapter: PhantomData,
        }
    }

    /// The first node after the head, when the ring is not empty.
    #[inline]
    fn first_node(&self) -> Option<NonNull<PinEntry<A::Elem>>> {
        self.head.next.get().filter(|&n| n != self.head.addr())
    }

    /// The last node before the head, when the ring is not empty.
    #[inline]
    fn last_node(&self) -> Option<NonNull<PinEntry<A::Elem>>> {
        self.head.prev.get().filter(|&n| n != self.head.addr())
    }

    /// `CIRCQ_EMPTY`.
    pub fn is_empty<W: Witness + ?Sized>(&self, w: &W) -> bool {
        bind(&self.key, id_of(w), "CIRCQ_EMPTY");
        self.first_node().is_none()
    }

    /// `CIRCQ_FIRST` as an element: borrows the ring and the witness.
    pub fn first<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> Option<&'a A::Elem> {
        bind(&self.key, id_of(w), "CIRCQ_FIRST");
        self.first_node().map(|p| elem_of(entry_at(p)))
    }

    /// `CIRCQ_FOREACH`: the elements in order.
    pub fn iter<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> RingIter<'a, A> {
        bind(&self.key, id_of(w), "CIRCQ_FOREACH");
        RingIter {
            next: self.first_node(),
            head: self.head.addr(),
            _borrow: PhantomData,
        }
    }

    /// Links `n` between `p` and `q` (`p.next == q`).
    #[inline]
    fn splice_in(
        n: NonNull<PinEntry<A::Elem>>,
        p: NonNull<PinEntry<A::Elem>>,
        q: NonNull<PinEntry<A::Elem>>,
    ) {
        let ent = entry_at(n);
        ent.prev.set(Some(p));
        ent.next.set(Some(q));
        entry_at(p).next.set(Some(n));
        entry_at(q).prev.set(Some(n));
    }

    /// `CIRCQ_INSERT_HEAD`: `e` becomes the first element. Panics when it is on a list.
    pub fn insert_head<W: Divisible + ?Sized>(self: Pin<&Self>, w: &mut W, e: Pin<&A::Elem>) {
        let id = id_of(w);
        bind(&self.key, id, "CIRCQ_INSERT_HEAD");
        let n = link::<A>(e, id, "CIRCQ_INSERT_HEAD");
        let h = self.head.addr();
        Self::splice_in(n, h, self.first_node().unwrap_or(h));
    }

    /// `CIRCQ_INSERT_TAIL`: `e` becomes the last element. Panics when it is on a list.
    pub fn insert_tail<W: Divisible + ?Sized>(self: Pin<&Self>, w: &mut W, e: Pin<&A::Elem>) {
        let id = id_of(w);
        bind(&self.key, id, "CIRCQ_INSERT_TAIL");
        let n = link::<A>(e, id, "CIRCQ_INSERT_TAIL");
        let h = self.head.addr();
        Self::splice_in(n, self.last_node().unwrap_or(h), h);
    }

    /// Unlinks the linked entry `ent` from whichever ring holds it; its identity kept when
    /// `keep` (the element moves on under the same lock).
    #[inline]
    fn unlink(ent: &PinEntry<A::Elem>, keep: bool) {
        let (Some(n), Some(p)) = (ent.next.get(), ent.prev.get()) else {
            hal_panic(format_args!("CIRCQ_REMOVE: the element is on no ring"));
        };
        entry_at(p).next.set(Some(n));
        entry_at(n).prev.set(Some(p));
        if keep {
            ent.next.set(None);
            ent.prev.set(None);
        } else {
            ent.release();
        }
    }

    /// `CIRCQ_REMOVE(e)`: takes `e` off the ring it is on, which need not be named (the
    /// neighbours are its own). Panics when it is on no ring of this lock.
    pub fn remove<W: Divisible + ?Sized>(w: &mut W, e: &A::Elem) {
        let ent = linked_entry::<A>(e, id_of(w), "CIRCQ_REMOVE");
        Self::unlink(ent, false);
    }

    /// `CIRCQ_FIRST` then `CIRCQ_REMOVE`: whether there was a first element to take off.
    pub fn remove_first<W: Divisible + ?Sized>(&self, w: &mut W) -> bool {
        bind(&self.key, id_of(w), "CIRCQ_REMOVE");
        match self.first_node() {
            Some(f) => {
                Self::unlink(entry_at(f), false);
                true
            }
            None => false,
        }
    }

    /// The first element moved to the tail of `dst` (`CIRCQ_REMOVE` then `CIRCQ_INSERT_TAIL`,
    /// the element never unlinked from the lock in between): whether there was one.
    pub fn move_first<W: Divisible + ?Sized>(&self, w: &mut W, dst: Pin<&Self>) -> bool {
        let id = id_of(w);
        bind(&self.key, id, "CIRCQ_INSERT_TAIL");
        bind(&dst.key, id, "CIRCQ_INSERT_TAIL");
        let Some(f) = self.first_node() else {
            return false;
        };
        Self::unlink(entry_at(f), true);
        let h = dst.head.addr();
        Self::splice_in(f, dst.last_node().unwrap_or(h), h);
        true
    }

    /// `CIRCQ_CONCAT(self, other)`: `other`'s elements appended, in order; `other` left empty.
    /// Four pointer writes, no walk: both rings are bound to one lock, whose identity the
    /// elements already carry. Panics when `other` is this ring.
    pub fn concat<W: Divisible + ?Sized>(self: Pin<&Self>, w: &mut W, other: &Self) {
        let id = id_of(w);
        bind(&self.key, id, "CIRCQ_CONCAT");
        bind(&other.key, id, "CIRCQ_CONCAT");
        if ptr::eq(self.get_ref(), other) {
            hal_panic(format_args!("CIRCQ_CONCAT: a ring onto itself"));
        }
        let (Some(of), Some(ol)) = (other.first_node(), other.last_node()) else {
            return;
        };
        let h = self.head.addr();
        let sl = self.last_node().unwrap_or(h);
        entry_at(sl).next.set(Some(of));
        entry_at(of).prev.set(Some(sl));
        entry_at(ol).next.set(Some(h));
        self.head.prev.set(Some(ol));
        other.head.next.set(None);
        other.head.prev.set(None);
    }
}

impl<A: PinAdapter> Default for PinRing<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: PinAdapter> Drop for PinRing<A> {
    /// Its elements point at the head: a ring dropped while not empty is refused (I1).
    fn drop(&mut self) {
        if self.first_node().is_some() {
            hal_panic(format_args!("PinRing: dropped with elements on it"));
        }
    }
}

/// The iterator of [`PinRing::iter`].
pub struct RingIter<'a, A: PinAdapter> {
    next: Option<NonNull<PinEntry<A::Elem>>>,
    head: NonNull<PinEntry<A::Elem>>,
    _borrow: PhantomData<&'a A::Elem>,
}

impl<'a, A: PinAdapter> Iterator for RingIter<'a, A> {
    type Item = &'a A::Elem;

    fn next(&mut self) -> Option<&'a A::Elem> {
        let ent = entry_at::<A::Elem>(self.next?);
        self.next = ent.next.get().filter(|&n| n != self.head);
        Some(elem_of(ent))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::vec::Vec;

    use super::*;
    use crate::intr::IPL_NONE;
    use crate::mutex::Mutexed;
    use crate::pin::pin_static;
    use crate::testutil;

    struct Node {
        v: u32,
        e: PinEntry<Node>,
    }

    impl Node {
        const fn new(v: u32) -> Self {
            Self {
                v,
                e: PinEntry::new(),
            }
        }
    }

    crate::pin_adapter!(ByE: Node, e);

    fn values<'a>(it: impl Iterator<Item = &'a Node>) -> Vec<u32> {
        it.map(|n| n.v).collect()
    }

    #[test]
    fn a_tail_queue_inserts_removes_and_walks_in_order() {
        testutil::install();
        let m = Mutexed::new(IPL_NONE, ());
        let mut g = m.lock();
        let l: PinList<ByE> = PinList::new();
        let a = core::pin::pin!(Node::new(1));
        let b = core::pin::pin!(Node::new(2));
        let c = core::pin::pin!(Node::new(3));
        l.insert_tail(&mut g, a.as_ref());
        l.insert_tail(&mut g, b.as_ref());
        l.insert_head(&mut g, c.as_ref());
        assert_eq!(values(l.iter(&g)), [3, 1, 2]);
        assert_eq!(l.first(&g).map(|n| n.v), Some(3));
        assert_eq!(l.last(&g).map(|n| n.v), Some(2));
        assert_eq!(l.next(&g, &a).map(|n| n.v), Some(2));
        l.remove(&mut g, &a);
        assert!(!a.e.is_linked());
        assert_eq!(values(l.iter(&g)), [3, 2]);
        assert!(l.remove_first(&mut g));
        l.remove(&mut g, &b);
        assert!(l.is_empty(&g));
        assert!(!l.remove_first(&mut g));
    }

    #[test]
    #[should_panic(expected = "on another list")]
    fn removing_an_end_through_another_list_is_refused() {
        testutil::install();
        let m = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let mut g = m.lock();
        let l1: &PinList<ByE> = Box::leak(Box::new(PinList::new()));
        let l2: &PinList<ByE> = Box::leak(Box::new(PinList::new()));
        let a: Pin<&Node> = pin_static(Box::leak(Box::new(Node::new(1))));
        l1.insert_tail(&mut g, a);
        l2.remove(&mut g, &a);
    }

    #[test]
    #[should_panic(expected = "a list of another lock")]
    fn a_list_refuses_the_witness_of_another_lock() {
        testutil::install();
        let m1 = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let m2 = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let l: &PinList<ByE> = Box::leak(Box::new(PinList::new()));
        let a: Pin<&Node> = pin_static(Box::leak(Box::new(Node::new(1))));
        l.insert_tail(&mut m1.lock(), a);
        let _ = l.is_empty(&m2.lock());
    }

    #[test]
    #[should_panic(expected = "dropped while linked")]
    fn an_element_dropped_while_linked_panics() {
        testutil::install();
        let m = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let mut g = m.lock();
        // The list outlives the panic (leaked): it is never touched again.
        let l: &PinList<ByE> = Box::leak(Box::new(PinList::new()));
        let a = Box::pin(Node::new(1));
        l.insert_tail(&mut g, a.as_ref());
        drop(a);
    }

    #[test]
    fn a_thousand_ring_elements_splice_and_leave_from_the_middle() {
        testutil::install();
        let m = Mutexed::new(IPL_NONE, ());
        let mut g = m.lock();
        let r1 = core::pin::pin!(PinRing::<ByE>::new());
        let r2 = core::pin::pin!(PinRing::<ByE>::new());
        let nodes: Vec<Pin<Box<Node>>> = (0..1000).map(|i| Box::pin(Node::new(i))).collect();
        for n in &nodes[..500] {
            r1.as_ref().insert_tail(&mut g, n.as_ref());
        }
        for n in &nodes[500..] {
            r2.as_ref().insert_tail(&mut g, n.as_ref());
        }
        r1.as_ref().concat(&mut g, &r2);
        assert!(r2.is_empty(&g));
        assert_eq!(values(r1.iter(&g)), (0..1000).collect::<Vec<_>>());
        // The middle leaves without naming its ring (CIRCQ_REMOVE).
        PinRing::<ByE>::remove(&mut g, &nodes[700]);
        PinRing::<ByE>::remove(&mut g, &nodes[0]);
        PinRing::<ByE>::remove(&mut g, &nodes[999]);
        let left = values(r1.iter(&g));
        assert_eq!(left.len(), 997);
        assert_eq!(left.first(), Some(&1));
        assert_eq!(left.last(), Some(&998));
        assert!(!left.contains(&700));
        // Emptied by moving everything onto the second ring, then by removal.
        while r1.move_first(&mut g, r2.as_ref()) {}
        assert_eq!(values(r2.iter(&g)).len(), 997);
        while r2.remove_first(&mut g) {}
        assert!(nodes.iter().all(|n| !n.e.is_linked()));
    }

    #[test]
    fn a_ring_head_and_insert_head_keep_the_c_order() {
        testutil::install();
        let m = Mutexed::new(IPL_NONE, ());
        let mut g = m.lock();
        let r = core::pin::pin!(PinRing::<ByE>::new());
        let a = core::pin::pin!(Node::new(1));
        let b = core::pin::pin!(Node::new(2));
        let c = core::pin::pin!(Node::new(3));
        r.as_ref().insert_tail(&mut g, a.as_ref());
        r.as_ref().insert_tail(&mut g, b.as_ref());
        r.as_ref().insert_head(&mut g, c.as_ref());
        assert_eq!(r.first(&g).map(|n| n.v), Some(3));
        assert_eq!(values(r.iter(&g)), [3, 1, 2]);
        PinRing::<ByE>::remove(&mut g, &a);
        assert_eq!(values(r.iter(&g)), [3, 2]);
        while r.remove_first(&mut g) {}
        assert!(r.is_empty(&g));
    }

    #[test]
    #[should_panic(expected = "not on a list of this lock")]
    fn a_ring_element_of_another_lock_is_refused() {
        testutil::install();
        let m1 = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let m2 = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let r: Pin<&PinRing<ByE>> = pin_static(Box::leak(Box::new(PinRing::new())));
        let a: Pin<&Node> = pin_static(Box::leak(Box::new(Node::new(1))));
        r.insert_tail(&mut m1.lock(), a);
        PinRing::<ByE>::remove(&mut m2.lock(), &a);
    }

    #[test]
    #[should_panic(expected = "on a list already")]
    fn linking_a_linked_element_again_is_refused() {
        testutil::install();
        let m = Box::leak(Box::new(Mutexed::new(IPL_NONE, ())));
        let mut g = m.lock();
        let l: &PinList<ByE> = Box::leak(Box::new(PinList::new()));
        let a: Pin<&Node> = pin_static(Box::leak(Box::new(Node::new(1))));
        l.insert_tail(&mut g, a);
        l.insert_head(&mut g, a);
    }

    #[test]
    fn a_tail_queue_concat_is_a_splice() {
        testutil::install();
        let m = Mutexed::new(IPL_NONE, ());
        let mut g = m.lock();
        let l1: PinList<ByE> = PinList::new();
        let l2: PinList<ByE> = PinList::new();
        let nodes: Vec<Pin<Box<Node>>> = (0..6).map(|i| Box::pin(Node::new(i))).collect();
        for n in &nodes[..3] {
            l1.insert_tail(&mut g, n.as_ref());
        }
        for n in &nodes[3..] {
            l2.insert_tail(&mut g, n.as_ref());
        }
        l1.concat(&mut g, &l2);
        assert!(l2.is_empty(&g));
        assert_eq!(values(l1.iter(&g)), [0, 1, 2, 3, 4, 5]);
        l1.remove(&mut g, &nodes[5]);
        assert_eq!(l1.last(&g).map(|n| n.v), Some(4));
        while l1.remove_first(&mut g) {}
    }

    #[test]
    fn try_insert_tail_refuses_an_element_on_a_list_of_another_lock() {
        testutil::install();
        let m1 = Mutexed::new(IPL_NONE, ());
        let m2 = Mutexed::new(IPL_NONE, ());
        let l1: PinList<ByE> = PinList::new();
        let l2: PinList<ByE> = PinList::new();
        let a = core::pin::pin!(Node::new(1));
        assert!(l1.try_insert_tail(&mut m1.lock(), a.as_ref()));
        assert!(
            !l1.try_insert_tail(&mut m1.lock(), a.as_ref()),
            "on this list"
        );
        assert!(
            !l2.try_insert_tail(&mut m2.lock(), a.as_ref()),
            "on another lock's"
        );
        assert!(l2.is_empty(&m2.lock()));
        l1.remove(&mut m1.lock(), &a);
        assert!(l2.try_insert_tail(&mut m2.lock(), a.as_ref()), "free again");
        l2.remove(&mut m2.lock(), &a);
    }
}
/* </TESTS> */
