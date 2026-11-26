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
//! Group `base` of `sysdefs`: the basic types and the constants every header leans on.

#[forbid(unsafe_code)]
pub mod endian;
#[forbid(unsafe_code)]
pub mod errno;
#[forbid(unsafe_code)]
pub mod ioccom;
#[forbid(unsafe_code)]
pub mod kernel;
#[forbid(unsafe_code)]
pub mod limits;
#[forbid(unsafe_code)]
pub mod pod;
#[forbid(unsafe_code)]
pub mod syslimits;
#[forbid(unsafe_code)]
pub mod types;
#[forbid(unsafe_code)]
pub mod uuid;

/// `<sys/param.h>`'s page size for the headers moved here (`ioccom.h`'s `IOCPARM_MAX`): the
/// machine's (`<machine/param.h>`), so `crate::base::param::PAGE_SIZE` names the same constant
/// in both crates.
#[forbid(unsafe_code)]
pub mod param {
    /// `PAGE_SIZE`: 4096 on arm64, amd64 and the host (`hal::vm::param` asserts the machine's equal).
    pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
    /// `PAGE_SHIFT`: log2 of `PAGE_SIZE`.
    pub const PAGE_SHIFT: usize = 12;
    /// `PAGE_MASK`: the offset bits of an address in a page.
    pub const PAGE_MASK: usize = PAGE_SIZE - 1;
}
#[cfg(test)]
#[allow(dead_code, unused_macros, unused_imports)]
pub(crate) mod reftest;
/* </CODE> */
