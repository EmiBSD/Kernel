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
//! - Not a port: a name of the crate's for `core::pin::Pin::static_ref` (stable since Rust
//!   1.61), whose premise is the one this module stated: a shared borrow that never ends
//!   excludes every move and every free of the value it names.
//! - N4.5a: [`project`], the projection of a pinned value onto one of its own fields (the
//!   thread's sleep timeout, `p_sleep_to`, which `timeout_add` takes pinned), with one
//!   `unsafe`: the field is checked to lie within the value's bytes, so it is pinned with it.

use core::pin::Pin;

use crate::assert::hal_panic;

/// `r`, pinned: a shared borrow that never ends excludes every move of the value it names
/// (`Pin::static_ref`).
#[inline]
pub const fn pin_static<T: ?Sized>(r: &'static T) -> Pin<&'static T> {
    Pin::static_ref(r)
}
/// `p`'s field `f` returns, pinned: the field lies within `p`'s own bytes (checked: anything
/// else, a value behind a pointer the field holds, is `panic(9)`), so it does not move and
/// its memory is not reused before `p`'s drop, which drops the field in place.
///
/// `f` is a function pointer: it captures nothing, so the reference it returns is reached
/// from `&T` alone (no guard or witness lent to it), for as long as `p`.
pub fn project<'a, T, F>(p: Pin<&'a T>, f: fn(&T) -> &F) -> Pin<&'a F> {
    let t: &'a T = p.get_ref();
    let field: &'a F = f(t);
    let base = core::ptr::from_ref(t).addr();
    let at = core::ptr::from_ref(field).addr();
    let inside = at >= base
        && at
            .checked_add(size_of::<F>())
            .is_some_and(|end| end <= base.saturating_add(size_of::<T>()));
    if !inside {
        hal_panic(format_args!(
            "pin::project: the field is not within the value"
        ));
    }
    // SAFETY: `Pin`'s contract for the field: it is not moved, nor its memory invalidated or
    // reused, before its drop runs. `p` gives that for `T`'s whole bytes until `T`'s drop, and
    // the field lies within them (checked above), so its memory is `T`'s: not moved and not
    // reused until `T`'s drop, which drops the field in place, as fields are, before the
    // memory goes. What could still move the field is a `&mut T`: none is reached through a
    // pinned `T` that is not `Unpin` (and a field that is not `Unpin` makes `T` so, no
    // `impl Unpin` being allowed outside `hal`, `cargo xtask hal-check`), except by `T`'s own
    // `Drop`, which the types this serves do not write to move a field out (a linked entry
    // moved out of its element stops the kernel at its drop, `hal::pinlist`'s I1). If `F` is
    // `Unpin`, pinning it promises nothing.
    unsafe { Pin::new_unchecked(field) }
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

    struct Holder {
        a: u32,
        b: AtomicU32,
        out: &'static AtomicU32,
    }

    #[test]
    fn a_field_of_a_pinned_value_is_projected_pinned() {
        let h = core::pin::pin!(Holder {
            a: 1,
            b: AtomicU32::new(2),
            out: &VALUE,
        });
        let b: Pin<&AtomicU32> = project(h.as_ref(), |h| &h.b);
        assert!(core::ptr::eq(b.get_ref(), &h.b));
        assert_eq!(b.load(Ordering::Relaxed), 2);
        assert_eq!(h.a, 1);
    }

    #[test]
    #[should_panic(expected = "not within the value")]
    fn a_reference_out_of_the_value_is_refused() {
        let h = core::pin::pin!(Holder {
            a: 1,
            b: AtomicU32::new(2),
            out: &VALUE,
        });
        let _ = project(h.as_ref(), |h| h.out);
    }
}
/* </TESTS> */
