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
//! The host double of the machine's copies (`copy.S`, `copystr.S`; `hal::user`'s backend on
//! the host), the one the host tests of every crate copy through since the `UserCopy` bridge
//! went (N4.8b; before, `hal`'s and `kernel`'s tests registered a double each).
//!
//! - User memory: one address space of [`USER_LEN`] bytes at [`USER_BASE`]; anything outside
//!   it is `EFAULT`, as an unmapped user page is. The tests read and write it through
//!   [`user_bytes`] and `hal::user::UserPtr`.
//! - Foreign memory (a `hal::user::ForeignWindow`'s pages): one run of [`FOREIGN_LEN`] bytes at
//!   [`FOREIGN_BASE`] ([`foreign_bytes`], [`set_foreign_bytes`]). The doubles' addresses lie
//!   below 4 GiB ([`DOUBLE_LIMIT`]), where no host memory is (macOS keeps the low 4 GiB of a
//!   64-bit process unmapped, `__PAGEZERO`): a kernel copy there outside the foreign run faults
//!   (`EFAULT`), as a window onto a page that went away does.
//! - A kernel address at or above [`DOUBLE_LIMIT`] is the host's own memory: the
//!   `hal::user::KcopyToken`'s holder (`bsd`'s `uiomove`, whose host tests build
//!   `UIO_SYSSPACE` uios over their own buffers) vouches for it, as on the machine.

// The host double alone links std outside the tests (rust-kernel.md: the host doubles).
extern crate std;

use std::sync::{Mutex, MutexGuard};
use std::vec::Vec;

use crate::sys::errno::Errno;

/// Where the double's user address space starts.
pub const USER_BASE: usize = 0x1000_0000;
/// Its size.
pub const USER_LEN: usize = 64 * 1024;
/// Where the double's foreign kernel memory starts (a `ForeignWindow`'s side of `kcopy`).
pub const FOREIGN_BASE: usize = 0x3000_0000;
/// Its size.
pub const FOREIGN_LEN: usize = 64 * 1024;

/// The user address space of the double.
static USER: Mutex<[u8; USER_LEN]> = Mutex::new([0; USER_LEN]);
/// The double's foreign kernel memory.
static FOREIGN: Mutex<[u8; FOREIGN_LEN]> = Mutex::new([0; FOREIGN_LEN]);

/// A double's memory, a panicking test's poison ignored.
fn lock<T>(m: &'static Mutex<T>) -> MutexGuard<'static, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The range `[addr - base, addr - base + len)` of a region of `size` bytes at `base`,
/// `EFAULT` outside it.
fn range(
    base: usize,
    size: usize,
    addr: usize,
    len: usize,
) -> Result<core::ops::Range<usize>, Errno> {
    let start = addr.checked_sub(base).ok_or(Errno::EFAULT)?;
    let end = start.checked_add(len).ok_or(Errno::EFAULT)?;
    if end > size {
        return Err(Errno::EFAULT);
    }
    Ok(start..end)
}

/// The range of the user address space at `uaddr`, `EFAULT` outside it.
fn user_range(uaddr: usize, len: usize) -> Result<core::ops::Range<usize>, Errno> {
    range(USER_BASE, USER_LEN, uaddr, len)
}

/// Below this the addresses are the doubles' (user and foreign memory, and the faults around
/// them); host memory lies above it.
pub const DOUBLE_LIMIT: usize = 1 << 32;

/// Whether `kva` is one of the doubles' addresses (a kernel copy there is the double's, an
/// `EFAULT` outside the foreign run).
fn is_foreign(kva: usize) -> bool {
    kva < DOUBLE_LIMIT
}

/// `copyin`: `EFAULT` outside the user address space.
pub(crate) fn copyin(uaddr: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
    let r = user_range(uaddr, kbuf.len())?;
    kbuf.copy_from_slice(&lock(&USER)[r]);
    Ok(())
}

/// `copyout`.
pub(crate) fn copyout(kbuf: &[u8], uaddr: usize) -> Result<(), Errno> {
    let r = user_range(uaddr, kbuf.len())?;
    lock(&USER)[r].copy_from_slice(kbuf);
    Ok(())
}

/// `copyin32`: the aligned word, `EFAULT` when misaligned or outside.
pub(crate) fn copyin32(uaddr: usize) -> Result<u32, Errno> {
    if !uaddr.is_multiple_of(4) {
        return Err(Errno::EFAULT);
    }
    let r = user_range(uaddr, 4)?;
    let mut w = [0u8; 4];
    w.copy_from_slice(&lock(&USER)[r]);
    Ok(u32::from_ne_bytes(w))
}

/// `copyinstr`: up to the NUL (copied), at most `kbuf.len()` bytes, `ENAMETOOLONG` without
/// one; `EFAULT` past the address space.
pub(crate) fn copyinstr(uaddr: usize, kbuf: &mut [u8]) -> Result<usize, Errno> {
    let user = lock(&USER);
    for (i, b) in kbuf.iter_mut().enumerate() {
        let r = user_range(uaddr.wrapping_add(i), 1)?;
        *b = user[r.start];
        if *b == 0 {
            return Ok(i + 1);
        }
    }
    Err(Errno::ENAMETOOLONG)
}

/// `copyoutstr`: `kbuf` up to its NUL (copied), at most `kbuf.len()` bytes, `ENAMETOOLONG`
/// without one; `EFAULT` past the address space.
pub(crate) fn copyoutstr(kbuf: &[u8], uaddr: usize) -> Result<usize, Errno> {
    let mut user = lock(&USER);
    for (i, &b) in kbuf.iter().enumerate() {
        let r = user_range(uaddr.wrapping_add(i), 1)?;
        user[r.start] = b;
        if b == 0 {
            return Ok(i + 1);
        }
    }
    Err(Errno::ENAMETOOLONG)
}

/// `kcopy(kva, buf, buf.len())`: the foreign region's bytes (`EFAULT` past its end), or the
/// host's own memory at `kva`.
///
/// # Safety
///
/// As the machine's (`arch/arm64/copy.rs`): `[kva, kva + buf.len())` outside the foreign
/// region is the caller's memory to read, no Rust object written during the copy.
pub(crate) unsafe fn kcopy_in(kva: usize, buf: &mut [u8]) -> Result<(), Errno> {
    if is_foreign(kva) {
        let r = range(FOREIGN_BASE, FOREIGN_LEN, kva, buf.len())?;
        buf.copy_from_slice(&lock(&FOREIGN)[r]);
        return Ok(());
    }
    let src = core::ptr::with_exposed_provenance::<u8>(kva);
    // SAFETY: the caller's guarantee: the range is readable host memory (the token holder's
    // own buffer, on the host its test's) that nothing writes during the copy; `buf` is a
    // distinct, exclusively borrowed slice of the same length.
    unsafe { core::ptr::copy(src, buf.as_mut_ptr(), buf.len()) };
    Ok(())
}

/// `kcopy(buf, kva, buf.len())`: into the foreign region (`EFAULT` past its end), or the
/// host's own memory at `kva`.
///
/// # Safety
///
/// As the machine's: `[kva, kva + buf.len())` outside the foreign region is the caller's
/// memory to write, no Rust object referencing it during the copy.
pub(crate) unsafe fn kcopy_out(buf: &[u8], kva: usize) -> Result<(), Errno> {
    if is_foreign(kva) {
        let r = range(FOREIGN_BASE, FOREIGN_LEN, kva, buf.len())?;
        lock(&FOREIGN)[r].copy_from_slice(buf);
        return Ok(());
    }
    let dst = core::ptr::with_exposed_provenance_mut::<u8>(kva);
    // SAFETY: the caller's guarantee: the range is writable host memory no Rust reference
    // covers during the copy; `buf` is only read, for its length.
    unsafe { core::ptr::copy(buf.as_ptr(), dst, buf.len()) };
    Ok(())
}

/// `len` bytes of the double's user memory at `uaddr`, `None` outside it.
pub fn user_bytes(uaddr: usize, len: usize) -> Option<Vec<u8>> {
    let r = user_range(uaddr, len).ok()?;
    Some(lock(&USER)[r].to_vec())
}

/// `len` bytes of the double's foreign kernel memory at `kva`, `None` outside it.
pub fn foreign_bytes(kva: usize, len: usize) -> Option<Vec<u8>> {
    let r = range(FOREIGN_BASE, FOREIGN_LEN, kva, len).ok()?;
    Some(lock(&FOREIGN)[r].to_vec())
}

/// Writes `b` into the double's foreign kernel memory at `kva`; `false` outside it.
pub fn set_foreign_bytes(kva: usize, b: &[u8]) -> bool {
    match range(FOREIGN_BASE, FOREIGN_LEN, kva, b.len()) {
        Ok(r) => {
            lock(&FOREIGN)[r].copy_from_slice(b);
            true
        }
        Err(_) => false,
    }
}
/* </CODE> */
