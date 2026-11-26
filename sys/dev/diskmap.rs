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
 * Copyright (c) 2009, 2010 Joel Sing <jsing@openbsd.org>
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
//! diskmap(4)'s `cdevsw[90]` entry points as `bsd`'s device switch names them: the kernel's
//! (`kernel::dev::diskmap`, N7.1d) over `bsd`'s glue.
//!
//! Upstream: sys/dev/diskmap.c @ 3ce1f3f79392
//!
//! Each is one concrete function, so that the address `cdevsw[90].d_ioctl` holds and the
//! one `pledge_ioctl` compares it with (`kern_pledge.rs`'s `DIOCMAP` arm) are the same
//! function's (a generic's instances may differ between codegen units).
//! docs/SCAFFOLDING.md: until the device switch is the kernel's (N7.3c).

use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::errno::Errno;
use crate::sys::proc::Proc;
use crate::sys::types::Dev;
use crate::sys::uio::Uio;

/// `diskmapopen`.
pub fn diskmapopen(dev: Dev, flag: i32, fmt: i32, p: &Proc) -> Result<(), Errno> {
    kernel::dev::diskmap::diskmapopen::<BsdProcGlue>(dev, flag, fmt, p)
}

/// `diskmapclose`.
pub fn diskmapclose(dev: Dev, flag: i32, fmt: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::dev::diskmap::diskmapclose::<BsdProcGlue>(dev, flag, fmt, p)
}

/// `diskmapioctl`: `DIOCMAP`.
pub fn diskmapioctl(dev: Dev, cmd: u64, addr: &mut [u8], flag: i32, p: &Proc) -> Result<(), Errno> {
    kernel::dev::diskmap::diskmapioctl(dev, cmd, addr, flag, p)
}

/// `diskmapread`.
pub fn diskmapread(dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::dev::diskmap::diskmapread::<BsdProcGlue>(dev, uio, flag)
}

/// `diskmapwrite`.
pub fn diskmapwrite(dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::dev::diskmap::diskmapwrite::<BsdProcGlue>(dev, uio, flag)
}
/* </CODE> */
