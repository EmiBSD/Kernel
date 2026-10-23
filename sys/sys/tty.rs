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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)tty.h	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/tty.h>` for `bsd`: the kernel's terminal (`kernel::tty`, `kernel::clist`, N4.7b)
//! under the path the drivers name, bound to `bsd`'s glue, with the sysctl names table that
//! `bsd`'s `kern_sysctl` prints.
//!
//! Upstream: sys/sys/tty.h @ 3ce1f3f79392
//! LZ: sys/sys/tty.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.7b: `struct tty` and `struct clist` are the kernel's (`kernel::tty::Tty`,
//!   `kernel::clist::Clist`: their modules' documentation has the shape); `Tty` here is the
//!   kernel's bound to `BsdProcGlue`, so the drivers' `&Tty`, `tp.t_state_isset(..)`,
//!   `tp.t_cflag()`, `tp.t_dev.get()`, `tp.t_outq.c_cc.get()` keep their meaning. LZ's six
//!   `unsafe` (`Ptmget`'s and `Itty`'s marker impls, the `Sync` of `Tty` and of the list head,
//!   the two raw-pointer derefs of `t_pgrp`/`t_session`) are gone with the types.
//! - `isctty` and `isbackground` are the kernel's, for `bsd`'s `Process`.

use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::proc::Process;
use crate::sys::sysctl::{CTLTYPE_QUAD, CTLTYPE_STRUCT, Ctlname};

pub use kernel::clist::Clist;
pub use kernel::tty::{
    BACKSPACE, CONTROL, DMBIC, DMBIS, DMGET, DMSET, Itty, KERN_TTY_INFO, KERN_TTY_MAXID,
    KERN_TTY_TKCANCC, KERN_TTY_TKNIN, KERN_TTY_TKNOUT, KERN_TTY_TKRAWCC, NEWLINE, OBUFSIZ,
    ORDINARY, PATH_PTMDEV, PTMGET, Ptmget, RETURN, Speedtab, TAB, TS_ASLEEP, TS_ASYNC, TS_BKSL,
    TS_BUSY, TS_CARR_ON, TS_CNTTB, TS_ERASE, TS_FLUSH, TS_ISOPEN, TS_LNCH, TS_LOCAL, TS_TBLOCK,
    TS_TIMEOUT, TS_TSTAMPCTSCLR, TS_TSTAMPCTSSET, TS_TSTAMPDCDCLR, TS_TSTAMPDCDSET, TS_TTSTOP,
    TS_TYPEN, TS_WOPEN, TS_XCLUDE, TTHIWATMINSPACE, TTIPRI, TTMAXLOWAT, TTMINHIWAT, TTMINLOWAT,
    TTOPRI, TTY_CHARMASK, TTY_ERRORMASK, TTY_FE, TTY_GID, TTY_PE, TTY_QUOTE, VTAB,
};

/// `struct tty` of `bsd`'s glue.
pub type Tty = kernel::tty::Tty<BsdProcGlue>;
/// `t_oproc`: start output.
pub type TOproc = kernel::tty::TOproc<BsdProcGlue>;
/// `t_param`: set hardware state.
pub type TParam = kernel::tty::TParam<BsdProcGlue>;
/// `t_hwiflow`: set hardware flow control; nonzero when it took effect.
pub type THwiflow = kernel::tty::THwiflow<BsdProcGlue>;

/// `CTL_KERN_TTY_NAMES`.
pub const CTL_KERN_TTY_NAMES: [Ctlname; KERN_TTY_MAXID] = [
    Ctlname::NONE,
    Ctlname::new(b"tk_nin", CTLTYPE_QUAD),
    Ctlname::new(b"tk_nout", CTLTYPE_QUAD),
    Ctlname::new(b"tk_rawcc", CTLTYPE_QUAD),
    Ctlname::new(b"tk_cancc", CTLTYPE_QUAD),
    Ctlname::new(b"ttyinfo", CTLTYPE_STRUCT),
    Ctlname::new(b"gap", 0),
    Ctlname::new(b"gap", 0),
];

/// `TTYHOG(tp)`: the input queue limit.
pub fn ttyhog(tp: &Tty) -> i32 {
    tp.ttyhog()
}

/// `isctty(pr, tp)`: is `tp` controlling terminal for `pr`? Under the kernel lock.
pub fn isctty(pr: &Process, tp: &Tty) -> bool {
    kernel::tty::isctty(pr, tp)
}

/// `isbackground(pr, tp)`: is `pr` in background of `tp`?
pub fn isbackground(pr: &Process, tp: &Tty) -> bool {
    kernel::tty::isbackground(pr, tp)
}
/* </CODE> */
