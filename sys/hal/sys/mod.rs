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
//! The constant headers of OpenBSD `sys/sys/` that name nothing of `bsd` (the kernel's `errno.h`, `types.h`, `syscall.h`, ...).

pub mod endian;
pub mod errno;
pub mod exec_elf;
pub mod futex;
pub mod intrmap;
pub mod kernel;
pub mod limits;
pub mod mman;
pub mod pclock;
pub mod pledge;
pub mod reboot;
pub mod select;
pub mod softintr;
pub mod syscall;
pub mod types;
pub mod unistd;
pub mod uuid;
pub mod vmmeter;
pub mod wait;
/* </CODE> */
