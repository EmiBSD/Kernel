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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)subr_log.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `bsd`'s side of the message buffer and `/dev/klog` (`kern/subr_log.c`), which are the
//! kernel crate's (`kernel::log`): the names `bsd`'s callers and the frozen machine code use,
//! the device switch's entry points of `/dev/klog` (character major 7), `sendsyslog(2)`'s
//! `sysent[]` row, and the console's half of the device switch the kernel writes through
//! (`kernel::log::TtyDevsw`).
//!
//! Upstream: sys/kern/subr_log.c @ 3ce1f3f79392
//! LZ: sys/kern/subr_log.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.7a: the buffers, `logsoftc`, the stash and `dosendsyslog` moved into `kernel::log`
//!   (its module documentation has the shape and the deviations); LZ's twelve `unsafe` (the
//!   `Msgbuf` overlay, the `StaticCell`s, the stash's raw blocks, `syslogf`'s pointer) are
//!   gone with them. This module keeps the names its callers know (`init_static_msgbuf`,
//!   called by both machines' early init, amd64's frozen; `initconsbuf`, `logwakeup`,
//!   `msgbufmapped`, `sys_sendsyslog`) over the kernel's functions bound to `bsd`'s glue,
//!   and adds what `/dev/klog` needs from `bsd` now that it is live: the five `cdevsw`
//!   entry points (`logopen` .. `logkqfilter`, arm64's `cdevsw[7]`), and the console's
//!   `cnwrite` and `constty || cn_devvp` for the kernel's `TtyDevsw` bridge, registered by
//!   `initconsbuf`.
//! - `log_open` is [`log_open`], a function (LZ: the `LOG_OPEN` static): the flag is the
//!   kernel's, in the glue's tables.
//! - `msgbufp()`/`consbufp()` and `msgbuf_putchar(mbp, c)` are `kernel::log`'s
//!   `msgbuf_putchar(which, c)` and readers (`msgbuf_header`, `msgbuf_read`): no reference
//!   to a buffer leaves `log_mtx`.

use kernel::log::TtyDevsw;
pub use kernel::log::{Buf, msgbuf_putchar, msgbufmapped};
use kernel::uio::UioMove;

use crate::dev::cons::{cn_devvp_set, cnwrite, constty};
use crate::kern::kern_subr::uiomove;
use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::proc::Proc;
use crate::sys::syscallargs::SysSendsyslogArgs;
use crate::sys::systm::{SysArgs, sysargs};
use crate::sys::types::{Dev, Register};
use crate::sys::uio::{Uio, UioRw};

/// `bsd`'s console for the kernel's log: `dev/cons.c`'s `cnwrite` and what `dosendsyslog`
/// tests in front of it.
struct BsdTtyDevsw;

impl TtyDevsw<BsdProcGlue> for BsdTtyDevsw {
    fn console_tty(&self) -> bool {
        constty().is_some() || cn_devvp_set()
    }

    fn console_write(&self, uio: &mut Uio<'_>) -> Result<(), Errno> {
        cnwrite(0, uio, 0)
    }
}

/// The registered console.
static BSD_TTYDEVSW: BsdTtyDevsw = BsdTtyDevsw;

/// A device read's uio, as the kernel's log device moves bytes into it (`uiomove(9)`).
struct LogUio<'a, 'b> {
    uio: &'a mut Uio<'b>,
}

impl UioMove for LogUio<'_, '_> {
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
}

/// Hands the machine's static buffer to `initmsgbuf` (both machines' early init).
pub fn init_static_msgbuf() {
    kernel::log::initmsgbuf();
}

/// `initconsbuf`: sets up the buffer that collects `/dev/console` output (`main`), and
/// registers the console the kernel's `sendsyslog(2)` writes through.
pub fn initconsbuf() {
    kernel::log::initconsbuf();
    kernel::log::register_ttydevsw::<BsdProcGlue>(&BSD_TTYDEVSW);
}

/// `log_open`: whether `/dev/klog` is open (`log()` also prints to the console when not).
pub fn log_open() -> bool {
    kernel::log::log_open::<BsdProcGlue>()
}

/// `logwakeup`: asks for the `/dev/klog` readers to be woken at the next tick.
pub fn logwakeup() {
    kernel::log::logwakeup::<BsdProcGlue>();
}

/// `logopen` (`cdevsw[7].d_open`).
pub fn logopen(_dev: Dev, _flags: i32, _mode: i32, _p: &Proc) -> Result<(), Errno> {
    kernel::log::logopen::<BsdProcGlue>()
}

/// `logclose` (`cdevsw[7].d_close`).
pub fn logclose(_dev: Dev, _flag: i32, _mode: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::log::logclose::<BsdProcGlue>(p)
}

/// `logread` (`cdevsw[7].d_read`).
pub fn logread(_dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::log::logread::<BsdProcGlue>(&mut LogUio { uio }, flag)
}

/// `logioctl` (`cdevsw[7].d_ioctl`).
pub fn logioctl(_dev: Dev, com: u64, data: &mut [u8], _flag: i32, p: &Proc) -> Result<(), Errno> {
    kernel::log::logioctl::<BsdProcGlue>(com, data, p)
}

/// `logkqfilter` (`cdevsw[7].d_kqfilter`).
pub fn logkqfilter(_dev: Dev, kn: &Knote) -> Result<(), Errno> {
    kernel::log::logkqfilter::<BsdProcGlue>(kn)
}

/// `sendsyslog(2)` (`sysent[112]`, `NOLOCK`): the kernel's, for the calling thread.
pub fn sys_sendsyslog(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysSendsyslogArgs = sysargs(v);
    kernel::log::sys_sendsyslog::<BsdProcGlue>(
        p,
        uap.buf.get() as usize,
        uap.nbyte.get(),
        uap.flags.get(),
    )
}
/* </CODE> */
