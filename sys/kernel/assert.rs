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
//! The kernel crate's `KASSERT(9)` and `panic(9)`.
//!
//! The kernel crate names nothing of `bsd`, so it cannot call `panic()` of
//! `kern/subr_prf.rs`, and it does not need to: every kernel image has one panic handler (the
//! `#[panic_handler]` of the crate that links the image, `bsd`'s `lib.rs` today), and that
//! handler is `panic(9)` with the message it was given (`sys/hal/assert.rs` says the same of
//! `hal`). So [`kpanic`] is `core::panic!`: in the kernel the message reaches `panic(9)`
//! unchanged, on the host it is the unwinding panic `#[should_panic]` sees.
//!
//! [`kassert`] is `KASSERT(9)`: active under feature `diagnostic` (OpenBSD's `option
//! DIAGNOSTIC`, which `bsd`'s feature of the same name turns on here), with the message
//! `__assert` prints (`kernel diagnostic assertion "e" failed: file "f", line n`); without the
//! feature the expression is type-checked and not evaluated, as the C's empty macro. Every
//! module of the crate uses these two (`pipe`, `vm::km`).

use core::fmt;

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

/// `KDASSERT(e)`: as [`kassert`] under feature `debug` (OpenBSD's `option DEBUG`); otherwise
/// `e` is checked by the compiler and never evaluated.
macro_rules! kdassert {
    ($e:expr) => {
        #[cfg(feature = "debug")]
        {
            if !$e {
                $crate::assert::kpanic(::core::format_args!(
                    "kernel debugging assertion \"{}\" failed: file \"{}\", line {}",
                    ::core::stringify!($e),
                    ::core::file!(),
                    ::core::line!()
                ));
            }
        }
        #[cfg(not(feature = "debug"))]
        {
            let _ = || -> bool { $e };
        }
    };
}
pub(crate) use kdassert;

/// `__assert("diagnostic ", file, line, expr)`: the failed assertion's `panic(9)`.
#[cfg(feature = "diagnostic")]
#[cold]
#[inline(never)]
pub(crate) fn assert_failed(file: &str, line: u32, expr: &str) -> ! {
    kpanic(format_args!(
        "kernel diagnostic assertion \"{expr}\" failed: file \"{file}\", line {line}"
    ))
}

/// `panic(9)` from the kernel crate: a broken precondition the C panics on. The message is
/// what the C prints, formatted by the caller.
#[cold]
#[inline(never)]
#[allow(clippy::panic, reason = "panic(9), through the image's panic handler")]
pub(crate) fn kpanic(args: fmt::Arguments<'_>) -> ! {
    core::panic!("{args}")
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "km_free: unmanaged page at 0x1000")]
    fn kpanic_carries_the_message() {
        kpanic(format_args!("km_free: unmanaged page at {:#x}", 0x1000));
    }

    #[test]
    fn a_true_assertion_passes() {
        kassert!(1 + 1 == 2);
    }

    #[cfg(feature = "diagnostic")]
    #[test]
    #[should_panic(expected = "kernel diagnostic assertion \"1 + 1 == 3\" failed")]
    fn a_false_assertion_panics_under_diagnostic() {
        kassert!(1 + 1 == 3);
    }

    #[cfg(not(feature = "diagnostic"))]
    #[test]
    fn a_false_assertion_is_not_evaluated_without_diagnostic() {
        let mut evaluated = false;
        kassert!({
            evaluated = true;
            false
        });
        assert!(!evaluated);
    }
}
/* </TESTS> */
