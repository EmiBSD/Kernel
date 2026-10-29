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
//! arm64's user and kernel copies: `copy.S` (`copyin`, `copyin32`, `copyout`, `kcopy`) and
//! `copystr.S` (`copyinstr`, `copyoutstr`), pulled in from the `.S` files next to this
//! module (they keep OpenBSD's licence blocks and layout; `{NAME}` placeholders are what
//! `assym.h` provides in C), behind `hal::user`'s typed copies.
//!
//! Upstream: sys/arch/arm64/arm64/copy.S @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/arm64/copystr.S @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/copy.rs@3c62ede68802
//! LZ: sys/arch/arm64/arm64/copystr.rs@3c62ede68802
//!
//! **What the assembly checks** (the C's, each kept): the user range's end does not overflow
//! (`adds` + `b.cs`) and lies in the user half (`tst #1 << 63`) before the first access, for
//! `copyin` and `copyout`; `copyin32`'s address is 4-byte aligned and in the user half;
//! `copyinstr` and `copyoutstr` test every user address they reach for the user half and stop
//! at `len` (`ENAMETOOLONG`); every user access is an unprivileged `ldtr`/`sttr` (the kernel's
//! own rights never reach user memory); `pcb_onfault` (the running thread's pcb, through
//! `ci_curpcb`) is saved, pointed at the routine's fault exit for the copy and restored on
//! every exit, so a fault on an unmapped page is `EFAULT` (`kdata_abort`'s recovery,
//! `trap.rs`). The kernel side is always a slice here, so only the user address can be
//! wrong, and the assembly turns that into `EFAULT`.
//!
//! ## Deviations
//! - No retguard.
//!
//! ## Redesign
//! - N4.8b: from `bsd` (`sys/arch/arm64/arm64/copy.rs`, `copystr.rs`, reached by `hal` through
//!   the `UserCopy` bridge) into `hal`, whose `user.rs` calls these directly: the bridge is
//!   gone. `kcopy` is `kcopy_in`/`kcopy_out`, `pub(crate)` and `unsafe`, the kernel side a
//!   slice: `hal` calls them for a `ForeignWindow`'s range and for the one `KcopyToken`
//!   (`hal::user`, the user's decision 13 of N4.8), never with a caller's raw pointers.

use core::arch::global_asm;
use core::mem::offset_of;

use super::include::pcb::Pcb;
use crate::cpu::CORE_CURPCB;
use crate::sys::errno::Errno;

global_asm!(
    include_str!("copy.S"),
    include_str!("copystr.S"),
    CI_CURPCB = const CORE_CURPCB,
    PCB_ONFAULT = const offset_of!(Pcb, pcb_onfault),
    EFAULT = const Errno::EFAULT.as_i32(),
    ENAMETOOLONG = const Errno::ENAMETOOLONG.as_i32(),
);

/// The C entry points; every one returns 0 or an errno.
mod sym {
    // SAFETY: the six items name the routines defined above in this crate (`copy.S`,
    // `copystr.S`) by their symbols, with the C prototypes they implement (AAPCS64: the
    // arguments in x0-x3, the `int` result in w0); each call site states what its arguments
    // must be.
    unsafe extern "C" {
        pub(super) fn copyin(uaddr: *const u8, kaddr: *mut u8, len: usize) -> i32;
        pub(super) fn copyin32(uaddr: *const u32, kaddr: *mut u32) -> i32;
        pub(super) fn copyout(kaddr: *const u8, uaddr: *mut u8, len: usize) -> i32;
        pub(super) fn kcopy(src: *const u8, dst: *mut u8, len: usize) -> i32;
        pub(super) fn copyinstr(
            uaddr: *const u8,
            kaddr: *mut u8,
            len: usize,
            done: *mut usize,
        ) -> i32;
        pub(super) fn copyoutstr(
            kaddr: *const u8,
            uaddr: *mut u8,
            len: usize,
            done: *mut usize,
        ) -> i32;
    }
}

/// An assembly routine's `int` as a `Result`.
fn errno(rc: i32) -> Result<(), Errno> {
    if rc == 0 {
        Ok(())
    } else {
        Err(Errno::from_raw(rc).unwrap_or(Errno::EFAULT))
    }
}

/// `copyin(uaddr, kbuf, kbuf.len())`: `EFAULT` when the user range is not mapped user memory.
pub(crate) fn copyin(uaddr: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
    let ua = core::ptr::with_exposed_provenance::<u8>(uaddr);
    // SAFETY: the kernel side is `kbuf`, valid for writes of its length and borrowed
    // exclusively for the call; the routine writes only inside it. The user side is checked
    // by the routine to lie in the user half without overflow before it is read, and read only
    // with `ldtr` (user rights), and any fault there exits through `pcb_onfault` with
    // `EFAULT` (the module's checks); user memory holds no Rust object, so no reference is
    // invalidated by the read.
    errno(unsafe { sym::copyin(ua, kbuf.as_mut_ptr(), kbuf.len()) })
}

/// `copyin32(uaddr, &v)`: the aligned 32-bit word at `uaddr`, read in one access.
pub(crate) fn copyin32(uaddr: usize) -> Result<u32, Errno> {
    let mut word = 0u32;
    let ua = core::ptr::with_exposed_provenance::<u32>(uaddr);
    // SAFETY: as for `copyin`; the kernel side is `word`, a local the routine writes once
    // (aligned, 4 bytes); the routine also refuses a misaligned user address.
    errno(unsafe { sym::copyin32(ua, &raw mut word) })?;
    Ok(word)
}

/// `copyout(kbuf, uaddr, kbuf.len())`.
pub(crate) fn copyout(kbuf: &[u8], uaddr: usize) -> Result<(), Errno> {
    let ua = core::ptr::with_exposed_provenance_mut::<u8>(uaddr);
    // SAFETY: as for `copyin`, the kernel side `kbuf` only read; the user side written only
    // with `sttr` after the same range checks, a fault being `EFAULT`.
    errno(unsafe { sym::copyout(kbuf.as_ptr(), ua, kbuf.len()) })
}

/// `copyinstr(uaddr, kbuf, kbuf.len(), &done)`: the bytes copied, the NUL included.
pub(crate) fn copyinstr(uaddr: usize, kbuf: &mut [u8]) -> Result<usize, Errno> {
    let mut done = 0usize;
    let ua = core::ptr::with_exposed_provenance::<u8>(uaddr);
    // SAFETY: as for `copyin`: the routine writes at most `kbuf.len()` bytes into `kbuf`
    // (its `len`), checks every user address it reads for the user half and reads it with
    // `ldtr`; `done` is a local it writes once.
    errno(unsafe { sym::copyinstr(ua, kbuf.as_mut_ptr(), kbuf.len(), &raw mut done) })?;
    Ok(done)
}

/// `copyoutstr(kbuf, uaddr, kbuf.len(), &done)`: the bytes copied, the NUL included.
pub(crate) fn copyoutstr(kbuf: &[u8], uaddr: usize) -> Result<usize, Errno> {
    let mut done = 0usize;
    let ua = core::ptr::with_exposed_provenance_mut::<u8>(uaddr);
    // SAFETY: as for `copyinstr`, with the kernel side `kbuf` only read (at most its length)
    // and the user side written with `sttr`.
    errno(unsafe { sym::copyoutstr(kbuf.as_ptr(), ua, kbuf.len(), &raw mut done) })?;
    Ok(done)
}

/// `kcopy(kva, buf, buf.len())`: the bytes at the kernel address `kva` into `buf`; a fault on
/// a page of the range is `EFAULT` (`pcb_onfault`) instead of a panic.
///
/// # Safety
///
/// `[kva, kva + buf.len())` holds no Rust object that something writes during the copy (a
/// page of it that is not mapped is allowed: that fault is the `EFAULT`): the foreign pages
/// of a `hal::user::ForeignWindow`, or what the `hal::user::KcopyToken`'s holder vouches for.
pub(crate) unsafe fn kcopy_in(kva: usize, buf: &mut [u8]) -> Result<(), Errno> {
    let s = core::ptr::with_exposed_provenance::<u8>(kva);
    // SAFETY: the routine reads the caller's range (its guarantee: no Rust object written
    // meanwhile; a fault exits through `pcb_onfault` with `EFAULT`) and writes `buf`, borrowed
    // exclusively for the call, never past its length.
    errno(unsafe { sym::kcopy(s, buf.as_mut_ptr(), buf.len()) })
}

/// `kcopy(buf, kva, buf.len())`: `buf` to the kernel address `kva`; a fault on a page of the
/// range is `EFAULT`.
///
/// # Safety
///
/// `[kva, kva + buf.len())` holds no Rust object (a page of it that is not mapped is allowed):
/// the foreign pages of a `hal::user::ForeignWindow`, or what the `hal::user::KcopyToken`'s
/// holder vouches for.
pub(crate) unsafe fn kcopy_out(buf: &[u8], kva: usize) -> Result<(), Errno> {
    let d = core::ptr::with_exposed_provenance_mut::<u8>(kva);
    // SAFETY: the routine reads `buf` (borrowed for the call, never past its length) and
    // writes the caller's range, which by its guarantee is no Rust object's memory; a fault
    // exits through `pcb_onfault` with `EFAULT`.
    errno(unsafe { sym::kcopy(buf.as_ptr(), d, buf.len()) })
}
/* </CODE> */
