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
//! arm64's general registers by name, as a system call convention reads them
//! (`hal::syscall::Gpr`, decision 53): `x0` to `x29`, the ones `struct trapframe` keeps in
//! `tf_x` (`<machine/frame.h>`; `x30` is `tf_lr` and the stack pointer `tf_sp`, neither of
//! them here). The host double compiles this same file (`hal::arch::host`), so the ABI's
//! arm64 convention is tested on the host under these names.

/// How many registers [`Gpr`] names: `tf_x`'s length.
pub(crate) const NGPR: usize = 30;

/// A general register of arm64, `x0` to `x29`: its number is its index in `tf_x`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Gpr {
    /// `x0`: the first argument register, and the return register of the C ABI and of a
    /// system call.
    X0 = 0,
    /// `x1`.
    X1 = 1,
    /// `x2`.
    X2 = 2,
    /// `x3`.
    X3 = 3,
    /// `x4`.
    X4 = 4,
    /// `x5`.
    X5 = 5,
    /// `x6`.
    X6 = 6,
    /// `x7`.
    X7 = 7,
    /// `x8`: the C ABI's indirect result register, which OpenBSD's system call convention
    /// loads with the number.
    X8 = 8,
    /// `x9`.
    X9 = 9,
    /// `x10`.
    X10 = 10,
    /// `x11`.
    X11 = 11,
    /// `x12`.
    X12 = 12,
    /// `x13`.
    X13 = 13,
    /// `x14`.
    X14 = 14,
    /// `x15`.
    X15 = 15,
    /// `x16`.
    X16 = 16,
    /// `x17`.
    X17 = 17,
    /// `x18`.
    X18 = 18,
    /// `x19`.
    X19 = 19,
    /// `x20`.
    X20 = 20,
    /// `x21`.
    X21 = 21,
    /// `x22`.
    X22 = 22,
    /// `x23`.
    X23 = 23,
    /// `x24`.
    X24 = 24,
    /// `x25`.
    X25 = 25,
    /// `x26`.
    X26 = 26,
    /// `x27`.
    X27 = 27,
    /// `x28`.
    X28 = 28,
    /// `x29`: the frame pointer of the C ABI.
    X29 = 29,
}

impl Gpr {
    /// The register's index in `tf_x` (its number).
    #[inline(always)]
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

// Every register indexes `tf_x` and the host double's array of the same length.
const _: () = assert!(Gpr::X29.index() + 1 == NGPR);
/* </CODE> */
