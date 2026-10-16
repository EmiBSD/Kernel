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
//! The runtime's assertions and its `panic(9)`.
//!
//! `hal` names nothing of the kernel, so it cannot call `panic()` of `kern/subr_prf.rs` the way
//! the legacy modules do. It does not need to: every kernel image has one panic handler (the
//! `#[panic_handler]` of the crate that links the image, today `bsd`'s `lib.rs`), and that
//! handler is `panic(9)` with the message it was given. So the runtime panics with
//! `core::panic!`: in the kernel the message reaches `panic(9)` unchanged, on the host (the
//! tests) it is the standard unwinding panic that `#[should_panic]` sees.
//!
//! [`kassert`] is `KASSERT(9)`: active under feature `diagnostic` (OpenBSD's `option
//! DIAGNOSTIC`, which `bsd`'s feature of the same name turns on here), with the message
//! `__assert` prints (`kernel diagnostic assertion "e" failed: file "f", line n`); without the
//! feature the expression is type-checked and not evaluated, as the C's empty macro.

/// `KASSERT(e)`: under feature `diagnostic`, panics with `__assert`'s message when `e` is
/// false; otherwise `e` is checked by the compiler and never evaluated.
macro_rules! kassert {
    ($e:expr) => {
        #[cfg(feature = "diagnostic")]
        {
            if !$e {
                $crate::assert::assert_failed(
                    ::core::file!(),
                    ::core::line!(),
                    ::core::stringify!($e),
                );
            }
        }
        #[cfg(not(feature = "diagnostic"))]
        {
            let _ = || -> bool { $e };
        }
    };
}
pub(crate) use kassert;

/// `__assert("diagnostic ", file, line, expr)`: the failed assertion's `panic(9)`.
#[cfg(feature = "diagnostic")]
#[cold]
#[inline(never)]
#[allow(
    clippy::panic,
    reason = "KASSERT(9) ends in panic(9), here the image's panic handler"
)]
pub(crate) fn assert_failed(file: &str, line: u32, expr: &str) -> ! {
    core::panic!("kernel diagnostic assertion \"{expr}\" failed: file \"{file}\", line {line}")
}

/// `panic(9)` from the runtime: a broken precondition the C would turn into a fault or a
/// panic. The message is what the C prints, formatted by the caller.
#[cold]
#[inline(never)]
#[allow(clippy::panic, reason = "panic(9), through the image's panic handler")]
pub(crate) fn hal_panic(args: core::fmt::Arguments<'_>) -> ! {
    core::panic!("{args}")
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "pool_put: double free")]
    fn hal_panic_carries_the_message() {
        hal_panic(format_args!("pool_put: double free"));
    }

    #[test]
    fn kassert_of_a_true_expression_does_nothing() {
        kassert!(1 + 1 == 2);
    }

    #[cfg(feature = "diagnostic")]
    #[test]
    #[should_panic(expected = "kernel diagnostic assertion \"1 == 2\" failed")]
    fn kassert_of_a_false_expression_panics_with_the_c_message() {
        kassert!(1 == 2);
    }

    #[cfg(not(feature = "diagnostic"))]
    #[test]
    fn kassert_does_not_evaluate_without_diagnostic() {
        let mut ran = false;
        kassert!({
            ran = true;
            ran
        });
        assert!(!ran);
    }
}
/* </TESTS> */
