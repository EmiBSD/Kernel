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
//! The kernel crate's `printf(9)` and its visible stubs (`.claude/rules/scope-and-stubs.md`).
//!
//! Not an OpenBSD file. [`printf`] is `hal::prf::printf`, and [`log`] `log(9)`, `bsd`'s
//! `printf(9)` and `log(9)` through the
//! `Printf` bridge: the console and the message buffer, so a line the C prints shows in
//! `dmesg` as it did in LZ. [`unported!`] is `bsd`'s `unported!` (`sys/kern/unported.rs`) for
//! the code that moved here: the first time a site runs it prints `unported: <name>` once,
//! through [`printf`], and it evaluates to `Errno::ENOSYS`, so an inherited gap stays as
//! visible on the transcript as it was in LZ.

use core::sync::atomic::{AtomicBool, Ordering};

pub use hal::prf::{log, printf};

use crate::sys::errno::Errno;

/// Reports the gap `name` once per `once` flag and yields `ENOSYS`; [`unported!`] is the way
/// to call it.
pub fn report_unported(name: &str, once: &AtomicBool) -> Errno {
    if !once.swap(true, Ordering::Relaxed) {
        printf(format_args!("unported: {name}\n"));
    }
    Errno::ENOSYS
}

/// `unported!("name")`: records that `name` is not ported yet (printed once per site) and
/// evaluates to `Errno::ENOSYS`.
macro_rules! unported {
    ($name:expr) => {{
        static ONCE: ::core::sync::atomic::AtomicBool =
            ::core::sync::atomic::AtomicBool::new(false);
        $crate::prf::report_unported($name, &ONCE)
    }};
}
pub(crate) use unported;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;
    use crate::testutil;

    #[test]
    fn a_gap_is_printed_once_per_site() {
        testutil::install();
        let site = || unported!("prf test gap");
        assert_eq!(site(), Errno::ENOSYS);
        assert_eq!(site(), Errno::ENOSYS);
        assert_eq!(testutil::printf_count("unported: prf test gap\n"), 1);
    }
}
/* </TESTS> */
