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

#[forbid(unsafe_code)]
pub mod endian;
#[forbid(unsafe_code)]
pub mod errno;
#[forbid(unsafe_code)]
pub mod exec_elf;
#[forbid(unsafe_code)]
pub mod futex;
#[forbid(unsafe_code)]
pub mod intrmap;
#[forbid(unsafe_code)]
pub mod kernel;
#[forbid(unsafe_code)]
pub mod limits;
#[forbid(unsafe_code)]
pub mod mman;
#[forbid(unsafe_code)]
pub mod pclock;
#[forbid(unsafe_code)]
pub mod pledge;
#[forbid(unsafe_code)]
pub mod reboot;
#[forbid(unsafe_code)]
pub mod select;
#[forbid(unsafe_code)]
pub mod softintr;
#[forbid(unsafe_code)]
pub mod syscall;
#[forbid(unsafe_code)]
pub mod types;
#[forbid(unsafe_code)]
pub mod unistd;
#[forbid(unsafe_code)]
pub mod uuid;
#[forbid(unsafe_code)]
pub mod vmmeter;
#[forbid(unsafe_code)]
pub mod wait;
/* </CODE> */
