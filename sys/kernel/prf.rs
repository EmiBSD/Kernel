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

/*-
 * Copyright (c) 1986, 1988, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)subr_prf.c	8.3 (Berkeley) 1/21/94
 */

/*-
 * Copyright (c) 1990, 1993
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
 *	@(#)tprintf.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The kernel crate's `printf(9)`, `tprintf(9)` and its visible stubs
//! (`.claude/rules/scope-and-stubs.md`).
//!
//! Upstream: sys/kern/subr_prf.c @ 3ce1f3f79392
//! Upstream: sys/sys/tprintf.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_prf.rs@3c62ede68802
//! LZ: sys/sys/tprintf.rs@3c62ede68802
//!
//! [`printf`] is `hal::prf::printf`, and [`log`] `log(9)`, `bsd`'s `printf(9)` and `log(9)`
//! through the `Printf` bridge: the console and the message buffer, so a line the C prints
//! shows in `dmesg` as it did in LZ. [`unported!`] is `bsd`'s `unported!`
//! (`sys/kern/unported.rs`) for the code that moved here: the first time a site runs it prints
//! `unported: <name>` once, through [`printf`], and it evaluates to `Errno::ENOSYS`, so an
//! inherited gap stays as visible on the transcript as it was in LZ. [`tprintf_open`],
//! [`tprintf`] and [`tprintf_close`] are `subr_prf.c`'s (`NFSSERVER || NFSCLIENT`; nfs's
//! `nfs_msg` is their one caller): a message to a process's controlling terminal and to the
//! log.
//!
//! ## Deviations
//! - `tprintf` takes `fmt::Arguments` (the C's format and `va_list`).
//!
//! ## Redesign
//! - LZ's `tprintf_open`/`tprintf`/`tprintf_close` were `bsd`'s `subr_prf.rs`, over its
//!   `kprintf_tp` and `kputchar`, compiled with feature `nfsclient` or `nfsserver`; `tpr_t` was
//!   `sys/sys/tprintf.rs`'s `Tpr`. They are the kernel's (N6.7a0, D-4), over what is the
//!   kernel's already: the session's terminal (`kernel::tty::session_tty`), `ttycheckoutq`,
//!   `tputchar`, the message buffer and `logwakeup` (`kernel::log`). The bytes are the C's:
//!   `logpri(LOG_INFO)`'s `<6>` to the log only, then each byte to the terminal when
//!   `ttycheckoutq` allowed it and to the log but NUL, CR and DEL (`kputchar`'s `TOLOG` leg);
//!   the console legs of `kputchar` (`TOCONS`, `TODDB`) are never asked for here. Compiled
//!   always: the kernel crate has no feature for nfs's server, and unused they cost nothing.

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

use hal::klock;
pub use hal::prf::{log, printf};

use crate::log::{Buf, LOG_INFO, logwakeup, msgbuf_putchar, msgbufmapped};
use crate::proc::{PS_CONTROLT, ProcGlue, SessionRef, Thread};
use crate::sys::errno::Errno;
use crate::tty::{Tty, session_tty, tputchar, ttycheckoutq};

/// A C string as `printf(9)`'s `%s` prints it: its bytes up to the first NUL, if any, each as
/// a character (a byte above 0x7f prints as its Latin-1 character, as LZ's `Str` did). The one
/// printer of a name kept as bytes (an extent's, a task queue's, a device's).
pub struct Str<'a>(pub &'a [u8]);

impl core::fmt::Display for Str<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for &c in self.0.iter().take_while(|&&c| c != 0) {
            core::fmt::Write::write_char(f, char::from(c))?;
        }
        Ok(())
    }
}

/// Reports the gap `name` once per `once` flag and yields `ENOSYS`; [`unported!`] is the way
/// to call it.
pub fn report_unported(name: &str, once: &AtomicBool) -> Errno {
    if !once.swap(true, Ordering::Relaxed) {
        printf(format_args!("unported: {name}\n"));
    }
    Errno::ENOSYS
}

/// `tpr_t`: a [`tprintf_open`] handle, the session whose controlling terminal gets the
/// message, counted (`SESSHOLD`, its drop `SESSRELE`), or `None` (the C's NULL) when the
/// process cannot be printed to.
pub type Tpr<G> = Option<SessionRef<G>>;

/// `tprintf_open(p)`: a tprintf handle on the process of `p` (XXX change s/proc/process):
/// its session, held, when it has a controlling terminal (`PS_CONTROLT` and `s_ttyvp`), else
/// `None`. The session is read under the kernel lock (`[K]`).
pub fn tprintf_open<G: ProcGlue>(p: &Thread<G>) -> Tpr<G> {
    let pr = p.process();
    if pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT == 0 {
        return None;
    }
    let r = klock::reader();
    let sess = pr.session(&r)?;
    if sess.ttyvp.is_none() {
        return None;
    }
    // SESSHOLD: the group's counted reference to its session, cloned.
    pr.pgrp(&r).and_then(|pg| pg.pg_session.get(&r).clone())
}

/// `tprintf_close(sess)`: disposes of a [`tprintf_open`] handle (`SESSRELE`, the drop).
pub fn tprintf_close<G: ProcGlue>(sess: Tpr<G>) {
    drop(sess);
}

/// Whether `kputchar`'s `TOLOG` leg keeps the byte `c`: not NUL, CR or DEL.
const fn logs(c: u8) -> bool {
    c != 0 && c != b'\r' && c != 0o177
}

/// `kputchar(c, TOLOG | [TOTTY], tp)`: one byte to the terminal (when there is one) and to
/// the message buffer.
fn tpr_putchar<G: ProcGlue>(c: u8, tp: Option<&Tty<G>>) {
    if let Some(t) = tp {
        let _ = tputchar(i32::from(c), t);
    }
    if logs(c) && msgbufmapped() {
        msgbuf_putchar(Buf::Msgbuf, c);
    }
}

/// `kprintf`'s sink for [`tprintf`]: every byte through [`tpr_putchar`].
struct TprSink<'a, G: ProcGlue> {
    tp: Option<&'a Tty<G>>,
}

impl<G: ProcGlue> fmt::Write for TprSink<'_, G> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            tpr_putchar(c, self.tp);
        }
        Ok(())
    }
}

/// `tprintf(tpr, fmt, ...)`: given a [`tprintf_open`] handle, sends a message to the
/// controlling terminal of that process (when `ttycheckoutq` lets it, without waiting) and
/// always to the message buffer at `LOG_INFO` (`logpri`'s `<6>`), then wakes the log's
/// readers. The session's terminal is read under the kernel lock.
pub fn tprintf<G: ProcGlue>(tpr: &Tpr<G>, args: fmt::Arguments<'_>) {
    // logpri(LOG_INFO): the priority, to the log only.
    let mut log = TprSink::<G> { tp: None };
    let _ = fmt::Write::write_fmt(&mut log, format_args!("<{LOG_INFO}>"));
    let tp = match tpr {
        Some(sess) if !sess.ttyvp.is_none() => session_tty(sess).filter(|t| ttycheckoutq(t, false)),
        _ => None,
    };
    let mut sink = TprSink { tp: tp.as_deref() };
    let _ = fmt::Write::write_fmt(&mut sink, args);
    logwakeup::<G>();
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
    // Host tests of the visible stubs and of tprintf's byte routing (its log line runs
    // through bsd's process fixtures in `bsd`'s `subr_prf` tests).

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

    #[test]
    fn the_log_keeps_every_byte_but_nul_cr_and_del() {
        let cases = [
            (0u8, false),
            (b'\r', false),
            (0o177, false),
            (b'\n', true),
            (b'a', true),
            (0xff, true),
        ];
        for (c, kept) in cases {
            assert_eq!(logs(c), kept, "{c:#x}");
        }
    }
}
/* </TESTS> */
