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
//! The runtime's error: an `errno(2)` number, carried as it is.
//!
//! The runtime's fallible operations fail with the errors the C returns from the same
//! operations: a lock taken with `RW_NOSLEEP` that is busy (`EBUSY`), an interruptible sleep a
//! signal ends (`EINTR`, `ERESTART`), a timed sleep that expires (`EWOULDBLOCK`), a user copy
//! that faults (`EFAULT`) or a user string that does not fit (`ENAMETOOLONG`). Several of them
//! come from a bridge (the sleep queues and the user copy are still `bsd`'s), so the value is
//! whatever number the implementation returned: [`Errno`] holds any `errno(2)` value, not a
//! closed list, and names the few the runtime itself produces. `bsd` converts it with its own
//! `Errno::from_raw` (`sys/sys/errno.rs`), the kernel crate and the ABI module with
//! [`Errno::get`].
//!
//! ## Redesign
//! - Interim, by design one definition: `<sys/errno.h>` moves into `hal` as the inherited
//!   `hal::sys::errno` (the user's decision of 2026-10-16, a parallel branch), and after that
//!   merge this file becomes `pub use crate::sys::errno::Errno;` in one commit. So the runtime
//!   names the type only as `crate::errno::Errno` and uses only what the legacy enum also has
//!   (the `E*` names, `EWOULDBLOCK`, `from_raw`, `Debug`); [`Errno::get`] is used by this
//!   file's own tests alone. Until then it is a transparent number, so no conversion can
//!   lose a value on the way up from a bridge; the constants are the C's values, checked
//!   against the header by a reference-backed test.

/// An `errno(2)` value, as the runtime's fallible operations return it. Never 0.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Errno(i32);

impl Errno {
    /// `EINTR`: interrupted system call.
    pub const EINTR: Errno = Errno(4);
    /// `ENOMEM`: cannot allocate memory.
    pub const ENOMEM: Errno = Errno(12);
    /// `EFAULT`: bad address.
    pub const EFAULT: Errno = Errno(14);
    /// `EBUSY`: device busy (a lock asked for without sleeping, held).
    pub const EBUSY: Errno = Errno(16);
    /// `EAGAIN`: resource temporarily unavailable.
    pub const EAGAIN: Errno = Errno(35);
    /// `EWOULDBLOCK`: `EAGAIN`, the timed sleep's expiry.
    pub const EWOULDBLOCK: Errno = Errno::EAGAIN;
    /// `ENAMETOOLONG`: a user string longer than its buffer.
    pub const ENAMETOOLONG: Errno = Errno(63);
    /// `ERESTART`: the kernel's "restart the system call" (the sleep of a restartable signal).
    pub const ERESTART: Errno = Errno(-1);
    /// `EJUSTRETURN`: the kernel's "return without touching the registers".
    pub const EJUSTRETURN: Errno = Errno(-2);

    /// The error whose number is `raw`; `None` for 0, which is success.
    pub const fn from_raw(raw: i32) -> Option<Errno> {
        if raw == 0 { None } else { Some(Errno(raw)) }
    }

    /// The number.
    pub const fn get(self) -> i32 {
        self.0
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::string::String;

    use super::*;

    #[test]
    fn zero_is_not_an_error_and_numbers_round_trip() {
        assert_eq!(Errno::from_raw(0), None);
        assert_eq!(Errno::from_raw(4), Some(Errno::EINTR));
        assert_eq!(Errno::from_raw(-1), Some(Errno::ERESTART));
        assert_eq!(Errno::EWOULDBLOCK, Errno::EAGAIN);
        assert_eq!(Errno::from_raw(77).map(Errno::get), Some(77));
    }

    /// The `#define NAME value` of `<sys/errno.h>`, read from `$OPENBSD_SRC`.
    fn define(header: &str, name: &str) -> Option<i32> {
        header.lines().find_map(|l| {
            let mut w = l.split_whitespace();
            (w.next() == Some("#define") && w.next() == Some(name))
                .then(|| w.next().and_then(|v| v.parse().ok()))
                .flatten()
        })
    }

    #[test]
    #[ignore = "reads $OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let src = std::env::var("OPENBSD_SRC").unwrap();
        let header: String = std::fs::read_to_string(src + "/sys/sys/errno.h").unwrap();
        for (name, e) in [
            ("EINTR", Errno::EINTR),
            ("ENOMEM", Errno::ENOMEM),
            ("EFAULT", Errno::EFAULT),
            ("EBUSY", Errno::EBUSY),
            ("EAGAIN", Errno::EAGAIN),
            ("ENAMETOOLONG", Errno::ENAMETOOLONG),
            ("ERESTART", Errno::ERESTART),
            ("EJUSTRETURN", Errno::EJUSTRETURN),
        ] {
            assert_eq!(define(&header, name), Some(e.get()), "{name}");
        }
    }
}
/* </TESTS> */
