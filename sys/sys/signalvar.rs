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

/*
 * Copyright (c) 1991, 1993
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
 *	@(#)signalvar.h	8.3 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/signalvar.h>`: kernel signal definitions and data structures, not exported to user
//! programs: `kernel::sig`'s since N4.5b, named here for `bsd`'s callers.
//!
//! Upstream: sys/sys/signalvar.h @ 3ce1f3f79392
//! LZ: sys/sys/signalvar.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.5b: `struct sigacts`, `SAS_*`, `SIG_CATCH`/`SIG_HOLD`, `SIGPENDING`, the `SA_*`
//!   signal properties, `sigcantmask`, `enum signal_type` and `struct sigctx` are
//!   `kernel::sig`'s (`sys/kernel/sig.rs`): `struct sigacts`'s `[m]` members are a cell of
//!   the process's `ps_mtx` there (LZ: `Cell`s under an `unsafe impl Sync`), opened with its
//!   guard. This module re-exports them (0 `unsafe`, LZ 1).

pub use kernel::sig::{
    SA_CANTMASK, SA_CONT, SA_CORE, SA_IGNORE, SA_KILL, SA_STOP, SA_TTYSTOP, SAS_NOCLDSTOP,
    SAS_NOCLDWAIT, SIG_CATCH, SIG_HOLD, Sigacts, Sigctx, SignalType, sigcantmask, sigpending,
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cant_mask_set_is_kill_and_stop() {
        assert_eq!(sigcantmask(), 0x1_0100);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/signalvar.h");
        let ours: &[(&str, i64)] = &[
            ("SAS_NOCLDSTOP", SAS_NOCLDSTOP as i64),
            ("SAS_NOCLDWAIT", SAS_NOCLDWAIT as i64),
            ("SA_KILL", SA_KILL as i64),
            ("SA_CORE", SA_CORE as i64),
            ("SA_STOP", SA_STOP as i64),
            ("SA_TTYSTOP", SA_TTYSTOP as i64),
            ("SA_IGNORE", SA_IGNORE as i64),
            ("SA_CONT", SA_CONT as i64),
            ("SA_CANTMASK", SA_CANTMASK as i64),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
