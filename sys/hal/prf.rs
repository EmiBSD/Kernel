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
//! `printf(9)` for the crates above `bsd`: the kernel's lines go where the C's `printf` puts
//! them, the console and the message buffer (`dmesg`).
//!
//! Not an OpenBSD file. `printf(9)`, its lock and the message buffer stay in `bsd`
//! (`sys/kern/subr_prf.rs`, deleted at N9), so they arrive through the [`Printf`] bridge
//! (bridges.toml; paid by N9), registered by `bsd`'s boot path before `main()`
//! (`hal_bridges_register`). [`crate::cons::Cons`] is the console primitive and skips the
//! message buffer; a line the C prints with `printf` goes through [`printf`] instead.
//!
//! **Before registration** (the host tests that register none) a line is dropped and
//! counted ([`dropped`]), as the console drops a character before `consinit()`.
//!
//! ## Redesign
//! - Not a port: `printf(9)` is `bsd`'s, reached through one trait object that takes the
//!   formatted arguments (`format_args!`), so nothing is formatted twice and no buffer is
//!   needed here.

use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::bridges;

/// The lines printed before `printf(9)` was registered.
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// `printf(9)`, as `bsd` registers it (`printf` of `sys/kern/subr_prf.rs`: the console and
/// the message buffer, under `printf`'s lock).
pub trait Printf: Sync {
    /// `printf(fmt, ...)`: `args` formatted onto the console and into the message buffer.
    fn printf(&self, args: fmt::Arguments<'_>);

    /// `log(level, fmt, ...)`: `args` into the message buffer at syslog priority `level` (and
    /// onto the console while no `syslogd` reads `/dev/klog`). The default prints: a double
    /// with no log.
    fn log(&self, level: i32, args: fmt::Arguments<'_>) {
        let _ = level;
        self.printf(args);
    }
}

/// `printf(9)`: `args` to the console and the message buffer; dropped (and counted) before
/// `printf` is registered.
pub fn printf(args: fmt::Arguments<'_>) {
    match bridges::printf() {
        Some(p) => p.printf(args),
        None => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// `log(9)`: `args` to the message buffer at priority `level`; dropped (and counted) before
/// `printf` is registered.
pub fn log(level: i32, args: fmt::Arguments<'_>) {
    match bridges::printf() {
        Some(p) => p.log(level, args),
        None => {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// The lines dropped because `printf(9)` was not registered yet.
pub fn dropped() -> u64 {
    DROPPED.load(Ordering::Relaxed)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn a_line_reaches_printf() {
        testutil::install();
        printf(format_args!("prf{} ok\n", 7));
        assert!(testutil::printf_saw("prf7 ok\n"));
    }
}
/* </TESTS> */
