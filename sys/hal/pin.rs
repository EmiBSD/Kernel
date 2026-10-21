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
//!   excludes every move and every free of the value it names. The module holds no `unsafe`
//!   (its `mod` line forbids it).

use core::pin::Pin;

/// `r`, pinned: a shared borrow that never ends excludes every move of the value it names
/// (`Pin::static_ref`).
#[inline]
pub const fn pin_static<T: ?Sized>(r: &'static T) -> Pin<&'static T> {
    Pin::static_ref(r)
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
