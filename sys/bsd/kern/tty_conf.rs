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
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)tty_conf.c	8.4 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The line discipline switch, `kern/tty_conf.c`, for `bsd`'s drivers: `linesw[]` as the
//! table of functions they call (`(linesw(tp).l_rint)(c, tp)`), every entry forwarding to the
//! tty's discipline in the kernel (`kernel::tty::LineDiscipline`, N4.7b).
//!
//! Upstream: sys/kern/tty_conf.c @ 3ce1f3f79392
//! LZ: sys/kern/tty_conf.rs@3c62ede68802
//!
//! ## Deviations
//! - `pseudo-device ppp`, `nmea`, `msts` and `endrun` are not configured (their line
//!   disciplines are not ported): `kernel::tty::Discipline::TABLE` holds the defunct entries
//!   the C uses when `NPPP`, `NNMEA`, `NMSTS` or `NENDRUN` is 0 (LZ's).
//!
//! ## Redesign
//! - N4.7b: `struct linesw` is the kernel's `LineDiscipline` trait (decision 46). The drivers
//!   still call through a `struct linesw` of `fn`s (`crate::sys::conf::Linesw`, until N7):
//!   [`LINESW`] is that table, every row the same eight functions, which find the tty's
//!   discipline again from its `t_line` (`kernel::tty::linesw`): one more array index than the
//!   C's `linesw[tp->t_line]` per received character.

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::tty::TtyUio;
use crate::sys::conf::Linesw;
use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::tty::Tty;
use crate::sys::types::Dev;
use crate::sys::uio::Uio;

pub use kernel::tty::NLINESW;

/// `l_open` of every row: the tty's discipline's.
fn l_open(dev: Dev, tp: &Tty, p: &Proc) -> Result<(), Errno> {
    kernel::tty::linesw(tp).open(dev, tp, p)
}

/// `l_close`.
fn l_close(tp: &Tty, flag: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::tty::linesw(tp).close(tp, flag, p)
}

/// `l_read`.
fn l_read(tp: &Tty, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::tty::linesw(tp).read(tp, &mut TtyUio { uio }, flag)
}

/// `l_write`.
fn l_write(tp: &Tty, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::tty::linesw(tp).write(tp, &mut TtyUio { uio }, flag)
}

/// `l_ioctl`.
fn l_ioctl(tp: &Tty, cmd: u64, data: &mut [u8], flag: i32, p: &Proc) -> Result<bool, Errno> {
    kernel::tty::linesw(tp).ioctl(tp, cmd, data, flag, p)
}

/// `l_rint`.
fn l_rint(c: i32, tp: &Tty) -> i32 {
    kernel::tty::linesw(tp).rint(c, tp)
}

/// `l_start`: the C's `int` (0, or the defunct discipline's `ENODEV`).
fn l_start(tp: &Tty) -> i32 {
    match kernel::tty::linesw(tp).start(tp) {
        Ok(()) => 0,
        Err(e) => e as i32,
    }
}

/// `l_modem`.
fn l_modem(tp: &Tty, flag: i32) -> i32 {
    i32::from(kernel::tty::linesw(tp).modem(tp, flag != 0))
}

/// One row of [`LINESW`].
const FORWARD: Linesw = Linesw {
    l_open,
    l_close,
    l_read,
    l_write,
    l_ioctl,
    l_rint,
    l_start,
    l_modem,
};

/// `linesw[]`: the line disciplines, indexed by `t_line` (`TTYDISC`, ...): every row forwards
/// to the kernel's `Discipline::<BsdProcGlue>::TABLE`.
pub static LINESW: [Linesw; NLINESW] = [FORWARD; NLINESW];

/// `linesw[tp->t_line]`.
pub fn linesw(tp: &Tty) -> &'static Linesw {
    LINESW.get(usize::from(tp.t_line.get())).unwrap_or(&FORWARD)
}

/// `nullioctl`: do nothing specific version of line discipline specific ioctl command.
pub fn nullioctl(tp: &Tty, cmd: u64, data: &mut [u8], flags: i32, p: &Proc) -> Result<bool, Errno> {
    kernel::tty::nullioctl::<BsdProcGlue>(tp, cmd, data, flags, p)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_forwards_to_the_kernel_s_discipline() {
        assert_eq!(LINESW.len(), NLINESW);
        for row in &LINESW {
            assert!(core::ptr::fn_addr_eq(
                row.l_rint,
                l_rint as fn(i32, &Tty) -> i32
            ));
            assert!(core::ptr::fn_addr_eq(
                row.l_start,
                l_start as fn(&Tty) -> i32
            ));
        }
    }
}
/* </TESTS> */
