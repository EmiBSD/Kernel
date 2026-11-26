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
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * William Jolitz.
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
 *	@(#)swapgeneric.c	5.5 (Berkeley) 5/9/91
 */
/* </LICENSES> */

/* <CODE> */
//! The root and swap configuration `config(8)` generates for a kernel: `swapgeneric.c` for
//! `config bsd swap generic` (GENERIC), and, as a deviation, the values its `swapbsd.c` would
//! hold for `config bsd root on rd0a swap on rd0b` (arm64's RAMDISK line; amd64's adds
//! `wd0b` and `sd0b`, drivers not ported).
//!
//! Upstream: sys/conf/swapgeneric.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - OpenBSD builds two kernels, `bsd` (GENERIC, "swap generic") and `bsd.rd` (RAMDISK,
//!   "root on rd0a"), and boot(8) loads one. Here one kernel serves both: the statics hold
//!   the generic values, and `swapconf_rdroot` switches them to the RAMDISK values when the
//!   boot loader hands over a ramdisk image (`sys/stand`, before `main`), as booting
//!   `bsd.rd` would. Without a ramdisk the kernel stays generic.
//! - `swdevt[]` has room for the RAMDISK configuration's one swap device; `uvm_swap.c`, its
//!   reader, is not ported.
//! - N7.1d: `rootdev`, `dumpdev`, `swdevt[]` and `mountroot` are the kernel's
//!   (`kernel::dev::disk`, the root choice's owner), re-exported here; `mountroot` is a
//!   `Mountroot` (`dk_mountroot`, `nfs_mountroot`) and `swapconf_rdroot` is safe.

use core::sync::atomic::Ordering::Relaxed;

pub use kernel::dev::disk::{DUMPDEV, ROOTDEV, SWDEVT};
use kernel::dev::disk::{Mountroot, mountroot_set};

use crate::sys::disklabel::makediskdev;
use crate::sys::param::NODEV;

/// The `rd(4)` block major on both architectures (`nam2blk[]` of their `autoconf.c`).
const RD_BMAJOR: u32 = 17;

/// What `swapbsd.c` holds for `config bsd root on rd0a swap on rd0b`: `rootdev` rd0a,
/// `dumpdev` and `swdevt[0]` rd0b, `mountroot` `dk_mountroot`. Before `main` reads
/// `mountroot` (the boot CPU's hand-over).
pub fn swapconf_rdroot() {
    ROOTDEV.store(makediskdev(RD_BMAJOR, 0, 0), Relaxed);
    DUMPDEV.store(makediskdev(RD_BMAJOR, 0, 1), Relaxed);
    SWDEVT[0].store(makediskdev(RD_BMAJOR, 0, 1), Relaxed);
    SWDEVT[1].store(NODEV, Relaxed);
    mountroot_set(Some(Mountroot::Disk));
}
/* </CODE> */
