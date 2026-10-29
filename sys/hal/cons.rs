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
//! The console primitive: `cnputc`, `cngetc` and `cnpollc` of `cons(4)`, and a
//! `core::fmt::Write` over them.
//!
//! The console device and `cn_tab` stay in `bsd` (`sys/dev/cons.rs`, the drivers behind it,
//! N7), so they arrive through the [`Console`] bridge (bridges.toml; paid by N7), registered
//! by `bsd` once `consinit()` has run. Like the C's `cnputc`, the primitive takes no lock: it
//! must work after a panic, in ddb and from any interrupt level. The `'\n'` to `"\n\r"`
//! expansion and the NUL that is ignored are the bridge's, in `cnputc` of `dev/cons.rs`.
//!
//! **Before registration** (before `consinit()`, the host tests that register none) there is
//! no console, and the C's `cnputc` with `cn_tab == NULL` drops the character: so does this
//! one, counting it ([`dropped`]); `cngetc` answers 0, `cnpollc` does nothing.
//!
//! Not for any line the C prints with `printf(9)`: `Cons` bypasses `printf`'s lock and the
//! message buffer, so `dmesg` never sees what goes through it, and lines a smoke or
//! `diff-openbsd` compares keep going through `bsd`'s `printf` until it moves; the crates
//! above `bsd` reach it through [`crate::prf::printf`].
//!
//! ## Redesign
//! - Not a port: the functions are `dev/cons.rs`'s, reached through one trait object; what is
//!   new is [`Cons`], a `fmt::Write` sink, so the runtime and the kernel crate can format to
//!   the console without `printf(9)` (which stays `bsd`'s, with the message buffer).

use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::bridges;

/// The characters written before a console was registered.
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// The console, as `bsd` registers it (`cnputc`, `cngetc`, `cnpollc` of `sys/dev/cons.rs`).
pub trait Console: Sync {
    /// `cnputc(c)`: one character to the console (a `'\n'` followed by `'\r'`; a NUL ignored).
    fn cnputc(&self, c: u8);
    /// `cngetc()`: one character from the console, waiting for it.
    fn cngetc(&self) -> i32;
    /// `cnpollc(on)`: polling on or off, counted so nested users agree.
    fn cnpollc(&self, on: bool);
}

/// `cnputc(c)`: one character to the console; dropped (and counted) before one is registered.
pub fn cnputc(c: u8) {
    match bridges::console() {
        Some(b) => b.cnputc(c),
        None => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// The bytes of `s`, one `cnputc` each.
pub fn cnputs(s: &[u8]) {
    for &c in s {
        cnputc(c);
    }
}

/// `cngetc()`: one character from the console, waiting for it; 0 before one is registered.
pub fn cngetc() -> i32 {
    bridges::console().map_or(0, Console::cngetc)
}

/// `cnpollc(on)`: nothing before a console is registered.
pub fn cnpollc(on: bool) {
    if let Some(b) = bridges::console() {
        b.cnpollc(on);
    }
}

/// The characters dropped because no console was registered yet.
pub fn dropped() -> u64 {
    DROPPED.load(Ordering::Relaxed)
}

/// The console as a `fmt::Write` sink: `write!(Cons, "...")`. Never fails.
pub struct Cons;

impl fmt::Write for Cons {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        cnputs(s.as_bytes());
        Ok(())
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::fmt::Write;

    use super::*;
    use crate::testutil;

    #[test]
    fn formatted_output_reaches_the_console() {
        testutil::install();
        let _ = write!(Cons, "cons{} ok", 0);
        assert!(testutil::console_saw(b"cons0 ok"));
    }

    #[test]
    fn input_comes_from_the_console() {
        testutil::install();
        testutil::console_feed(b'x');
        assert_eq!(cngetc(), i32::from(b'x'));
        cnpollc(true);
        cnpollc(false);
    }
}
/* </TESTS> */
