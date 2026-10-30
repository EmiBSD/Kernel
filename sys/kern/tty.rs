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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)tty.c	8.8 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The terminal line discipline, `kern/tty.c`, for `bsd`: the kernel's (`kernel::tty`,
//! N4.7b) bound to `bsd`'s glue under the names the drivers, the device switch and the
//! frozen machine code call, and `bsd`'s half of the terminal's bridge (`TtyDevsw`: the
//! device switch's `d_stop`, the console's `cnwrite`, `/dev/console`'s access check, the
//! session's controlling vnode).
//!
//! Upstream: sys/kern/tty.c @ 3ce1f3f79392
//! LZ: sys/kern/tty.rs@3c62ede68802
//!
//! ## Deviations
//! - `ttyfree` retires the tty (`kernel::tty::ttyfree`: off the list, its timeout stopped,
//!   its knotes told EOF, its rings freed) and leaves its pool item allocated: the drivers
//!   hold it as `&'static` ([`ttymalloc`] leaks the handle), until they move (N7, decision 5
//!   of N4.7).
//!
//! ## Redesign
//! - N4.7b: the line discipline, the tty list, `kern.tty` and the filters are the kernel's
//!   (`kernel::tty`'s documentation has the shape); LZ's eleven `unsafe` (the raw tty block,
//!   the list's insert and removal, `kn_tty`, the two timeout arguments, the `itty` array) are
//!   gone with them. What stays here is the glue: each frozen name is one call of the
//!   kernel's for `BsdProcGlue`; `ttymalloc` hands the drivers a `&'static Tty` (the counted
//!   handle given up, `PoolArc::leak`); `ttkqfilter(dev, kn)` finds the tty through the
//!   device switch's `d_tty`; `sysctl_tty` turns `kern_sysctl`'s arguments into a
//!   `SysctlReq`; a driver's `struct uio` reaches the kernel's read and write as a
//!   [`TtyUio`] (`uiomove(9)`).
//! - `TTY_COUNT` is [`tty_count`] (the count is the kernel's, in the glue's tables).

use core::ffi::c_void;

use kernel::tty::TtyDevsw;
use kernel::uio::UioMove;

use crate::dev::cons::{cn_devvp_set, cn_tab, cnwrite, constty};
use crate::kern::kern_subr::uiomove;
use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::conf::cdevsw;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::proc::Proc;
use crate::sys::tty::{Speedtab, Tty};
use crate::sys::types::{Dev, major};
use crate::sys::uio::{Uio, UioRw};

pub use kernel::tty::{
    CCLASSMASK, CHAR_TYPE, TK_CANCC, TK_NIN, TK_NOUT, TK_RAWCC, TTCLOS, TTOPEN, TTYBG, TTYIN,
    TTYOUT,
};

/// A driver's `struct uio` as the kernel's terminal moves bytes through it (`uiomove(9)`).
pub struct TtyUio<'a, 'b> {
    /// The uio.
    pub uio: &'a mut Uio<'b>,
}

impl UioMove for TtyUio<'_, '_> {
    fn resid(&self) -> usize {
        self.uio.uio_resid
    }

    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
        uiomove(buf, self.uio)
    }

    fn nonblock(&self) -> bool {
        false
    }

    fn is_write(&self) -> bool {
        self.uio.uio_rw == UioRw::UIO_WRITE
    }

    fn set_resid(&mut self, resid: usize) {
        self.uio.uio_resid = resid;
    }
}

/// `bsd`'s half of the terminal: the device switch, the console, the vnode layer.
struct BsdTtyDevsw;

impl TtyDevsw<BsdProcGlue> for BsdTtyDevsw {
    fn console_tty(&self) -> bool {
        constty().is_some() || cn_devvp_set()
    }

    fn console_write(&self, uio: &mut Uio<'_>) -> Result<(), Errno> {
        cnwrite(0, uio, 0)
    }

    fn stop(&self, tp: &Tty, rw: i32) {
        let _ = (cdevsw(major(tp.t_dev.get())).d_stop)(tp, rw);
    }

    fn console_dev(&self) -> Option<Dev> {
        cn_tab().map(|cp| cp.cn_dev.get())
    }
}

/// The registered half.
static BSD_TTYDEVSW: BsdTtyDevsw = BsdTtyDevsw;

/// Registers `bsd`'s half of the terminal's bridge (`initconsbuf`, before the first tty).
pub fn register_devsw() {
    kernel::tty::register_ttydevsw::<BsdProcGlue>(&BSD_TTYDEVSW);
}

/// `ttyopen`: initial open of tty, or (re)entry to standard tty line discipline.
pub fn ttyopen(device: Dev, tp: &Tty, p: &Proc) -> Result<(), Errno> {
    kernel::tty::ttyopen(device, tp, p)
}

/// `ttyclose`: handle close() on a tty line.
pub fn ttyclose(tp: &Tty) -> Result<(), Errno> {
    kernel::tty::ttyclose(tp)
}

/// `ttyinput`: process input of a single character received on a tty.
pub fn ttyinput(c: i32, tp: &Tty) -> i32 {
    kernel::tty::ttyinput(c, tp)
}

/// `ttyoutput`: output a single character on a tty.
pub fn ttyoutput(c: i32, tp: &Tty) -> i32 {
    kernel::tty::ttyoutput(c, tp)
}

/// `ttioctl`: ioctls for all tty devices; `Ok(false)` where the C returns `-1`.
pub fn ttioctl(tp: &Tty, cmd: u64, data: &mut [u8], flag: i32, p: &Proc) -> Result<bool, Errno> {
    kernel::tty::ttioctl(tp, cmd, data, flag, p)
}

/// `ttkqfilter`: attaches a kqueue filter to the tty of `dev` (`cdevsw[major(dev)].d_tty`).
pub fn ttkqfilter(dev: Dev, kn: &Knote) -> Result<(), Errno> {
    let Some(tp) = cdevsw(major(dev)).d_tty.and_then(|t| t(dev)) else {
        return Err(Errno::ENXIO);
    };
    kernel::tty::ttkqfilter(tp, kn)
}

/// `ttywait_nsec`: wait for output to drain, or if this times out, flush it.
pub fn ttywait_nsec(tp: &Tty, nsecs: u64) -> Result<(), Errno> {
    kernel::tty::ttywait_nsec(tp, nsecs)
}

/// `ttywait`: wait for output to drain.
pub fn ttywait(tp: &Tty) -> Result<(), Errno> {
    kernel::tty::ttywait(tp)
}

/// `ttywflush`: flush if successfully wait.
pub fn ttywflush(tp: &Tty) -> Result<(), Errno> {
    kernel::tty::ttywflush(tp)
}

/// `ttyflush`: flush tty read and/or write queues, notifying anyone waiting.
pub fn ttyflush(tp: &Tty, rw: i32) {
    kernel::tty::ttyflush(tp, rw);
}

/// `ttychars`: copy in the default termios characters.
pub fn ttychars(tp: &Tty) {
    kernel::tty::ttychars(tp);
}

/// `ttstart`: start output through the driver's `t_oproc`.
pub fn ttstart(tp: &Tty) -> i32 {
    kernel::tty::ttstart(tp)
}

/// `ttylclose`: "close" a line discipline.
pub fn ttylclose(tp: &Tty, flag: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::tty::ttylclose(tp, flag, p)
}

/// `ttymodem`: handle modem control transition on a tty.
pub fn ttymodem(tp: &Tty, flag: i32) -> i32 {
    kernel::tty::ttymodem(tp, flag)
}

/// `nullmodem`: default modem control routine (for other line disciplines).
pub fn nullmodem(tp: &Tty, flag: i32) -> i32 {
    kernel::tty::nullmodem(tp, flag)
}

/// `ttypend`: reinput pending characters after state switch.
pub fn ttypend(tp: &Tty) {
    kernel::tty::ttypend(tp);
}

/// `ttread`: process a read call on a tty device.
pub fn ttread(tp: &Tty, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::tty::ttread(tp, &mut TtyUio { uio }, flag)
}

/// `ttyunblock`.
pub fn ttyunblock(tp: &Tty) {
    kernel::tty::ttyunblock(tp);
}

/// `ttycheckoutq`: check the output queue on `tp` for space for a kernel message.
pub fn ttycheckoutq(tp: &Tty, wait: bool) -> bool {
    kernel::tty::ttycheckoutq(tp, wait)
}

/// `ttwrite`: process a write call on a tty device.
pub fn ttwrite(tp: &Tty, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::tty::ttwrite(tp, &mut TtyUio { uio }, flag)
}

/// `ttyrub`: rubout one character from the rawq of tp as cleanly as possible.
pub fn ttyrub(c: i32, tp: &Tty) -> i32 {
    kernel::tty::ttyrub(c, tp)
}

/// `ttyretype`: reprint the rawq line.
pub fn ttyretype(tp: &Tty) -> i32 {
    kernel::tty::ttyretype(tp)
}

/// `ttwakeupwr`: wakeup any writers if necessary.
pub fn ttwakeupwr(tp: &Tty) {
    kernel::tty::ttwakeupwr(tp);
}

/// `ttwakeup`: wake up any readers on a tty.
pub fn ttwakeup(tp: &Tty) {
    kernel::tty::ttwakeup(tp);
}

/// `ttspeedtab`: look up a code for a specified speed in a conversion table.
pub fn ttspeedtab(speed: i32, table: &[Speedtab]) -> i32 {
    kernel::tty::ttspeedtab(speed, table)
}

/// `ttsetwater`: set tty hi and low water marks.
pub fn ttsetwater(tp: &Tty) {
    kernel::tty::ttsetwater(tp);
}

/// `ttyinfo`: report on state of foreground process group.
pub fn ttyinfo(tp: &Tty) {
    kernel::tty::ttyinfo(tp);
}

/// `tputchar`: output char to tty; console putchar style.
pub fn tputchar(c: i32, tp: &Tty) -> i32 {
    kernel::tty::tputchar(c, tp)
}

/// `ttysleep`: sleep on `chan`, `ERESTART` if the tty changed while we napped.
pub fn ttysleep(tp: &Tty, chan: *const c_void, pri: i32, wmesg: &'static str) -> Result<(), Errno> {
    kernel::tty::ttysleep(tp, chan.cast(), pri, wmesg)
}

/// `ttysleep_nsec`: [`ttysleep`] with a timeout.
pub fn ttysleep_nsec(
    tp: &Tty,
    chan: *const c_void,
    pri: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    kernel::tty::ttysleep_nsec(tp, chan.cast(), pri, wmesg, nsecs)
}

/// `tty_init`: initialise the global tty list.
pub fn tty_init() {
    kernel::tty::tty_init::<BsdProcGlue>();
}

/// `ttymalloc`: allocate a tty structure and its associated buffers, and attach it to the
/// tty list; the driver keeps it for the kernel's life (see the module's deviations).
pub fn ttymalloc(baud: i32) -> &'static Tty {
    hal::pool::PoolArc::leak(kernel::tty::ttymalloc::<BsdProcGlue>(baud))
}

/// `ttyfree`: retire a tty (see the module's deviations).
pub fn ttyfree(tp: &Tty) {
    kernel::tty::ttyfree(tp);
}

/// `tty_count`: number of ttys in global ttylist.
pub fn tty_count() -> i32 {
    kernel::tty::tty_count::<BsdProcGlue>()
}

/// `sysctl_tty`: return tty-related information (`kern.tty`).
pub fn sysctl_tty(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let mut req = kernel::sysctl::SysctlReq {
        oldp,
        oldlenp,
        newp,
        newlen,
    };
    kernel::tty::sysctl_tty::<BsdProcGlue>(name, &mut req)
}

/// `ttytstamp`: record the time of a modem line transition the user asked for.
pub fn ttytstamp(tp: &Tty, octs: i32, ncts: i32, odcd: i32, ndcd: i32) {
    kernel::tty::ttytstamp(tp, octs, ncts, odcd, ndcd);
}
/* </CODE> */
