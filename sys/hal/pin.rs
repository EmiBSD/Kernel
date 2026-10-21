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
//! `pin_static`: a value that lives for ever, pinned.
//!
//! Not an OpenBSD file. The lists of [`crate::pinlist`] take their elements pinned (`Pin<&T>`)
//! and a ring takes its head pinned. A value pinned on a stack (`core::pin::pin!`) or in a
//! pool item ([`crate::pool::PoolArc::as_pin`]) has its own way in; a `static`, or what a
//! legacy caller hands over as `&'static` (the `struct task`s of the softcs, `task_add`'s
//! argument since LZ), needs this one.
//!
//! ## Redesign
//! - Not a port: `Pin::new_unchecked` written once here, over the one premise that makes it
//!   sound for every caller, instead of at each site.

use core::pin::Pin;

/// `r`, pinned: a shared borrow that never ends excludes every move of the value it names.
pub fn pin_static<T: ?Sized>(r: &'static T) -> Pin<&'static T> {
    // SAFETY: `Pin`'s contract for `Pin<&T>`: the `T` is not moved, nor its memory invalidated
    // or reused, before its drop runs. A `&'static T` is a shared borrow that lasts for the rest
    // of the program, so for as long as the program runs no `&mut T` to the same value may
    // exist and its owner cannot use it by value (both conflict with a live shared borrow),
    // which are the only ways to move a `T` out of its place or to free it: it stays where it
    // is, and its memory is never given back (whatever owns it is itself borrowed for ever:
    // a `static`, a leaked allocation). Interior mutability (`Cell::replace`, a lock's guard)
    // moves the contents of a field, never the `T`: `hal`'s primitives project no pin through
    // such a field (the lists reach an element's entry as a plain field, `pinlist`'s I1).
    unsafe { Pin::new_unchecked(r) }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    static VALUE: AtomicU32 = AtomicU32::new(7);

    #[test]
    fn a_static_is_pinned_where_it_is() {
        let p = pin_static(&VALUE);
        assert!(core::ptr::eq(p.get_ref(), &VALUE));
        p.store(8, Ordering::Relaxed);
        assert_eq!(VALUE.load(Ordering::Relaxed), 8);
    }

    #[test]
    fn a_leaked_value_is_pinned_too() {
        let r: &'static [u32] = std::boxed::Box::leak(std::vec![1, 2, 3].into_boxed_slice());
        let p = pin_static(r);
        assert_eq!(&*p, &[1, 2, 3]);
    }
}
/* </TESTS> */
