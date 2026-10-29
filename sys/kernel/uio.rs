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
 * Copyright (c) 1982, 1986, 1993, 1994
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
 *	@(#)uio.h	8.5 (Berkeley) 2/22/94
 */

/*
 * Copyright (c) 1996 Theo de Raadt
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)sys_generic.c	8.5 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/uio.h>`: scatter/gather I/O descriptions (`struct iovec`, `enum uio_rw`,
//! `enum uio_seg`, the kernel's `struct uio`), and `sys_generic.c`'s `iovec_copyin`, which
//! reads a user's iovec array for `readv(2)`, `writev(2)`, `preadv(2)` and `pwritev(2)`.
//!
//! Upstream: sys/sys/uio.h @ 3ce1f3f79392
//! Upstream: sys/kern/sys_generic.c @ 3ce1f3f79392
//! LZ: sys/sys/uio.rs@3c62ede68802
//! LZ: sys/kern/sys_generic.rs@3c62ede68802
//!
//! A [`Uio`] describes a transfer in progress: the iovecs not yet consumed, the file offset,
//! the bytes left, whose address space the iovecs name ([`UioSeg`]) and which way the bytes
//! flow ([`UioRw`]). The thread it is done for is the kernel's [`Thread`], generic over the
//! glue that carries `bsd`'s members ([`ProcGlue`]); `bsd` names the one instance it uses by
//! an alias (`sys/sys/uio.rs`), so its users write `Uio` as before. [`UioMove`] is the copy
//! the kernel's objects (a pipe, `uvm_io`'s window) ask of a uio: `uiomove(9)`, which stays
//! `bsd`'s (`kern_subr.rs`) while a `UIO_SYSSPACE` uio's iovecs are raw kernel addresses that
//! its builder vouches for.
//!
//! [`iovec_copyin`] copies `iovcnt` iovecs from userland into an [`IovBuf`]: the eight the
//! caller keeps on its stack, or a `malloc(M_IOV)` array past them (at most `IOV_MAX`), and
//! returns them with their total length, each length and the total bounded by `SSIZE_MAX`
//! (a read or write returns an `ssize_t`).
//!
//! ## Deviations
//! - `uio_iov`/`uio_iovcnt` are one mutable slice of iovecs: advancing to the next iovec
//!   (`uio->uio_iov++; uio->uio_iovcnt--`) shortens the slice, and [`Uio::uio_iovcnt`] is
//!   its length.
//! - `uio_procp` is `Option<&Thread>` (the C's NULL is `None`).
//! - [`Iovec::from_bytes`] is the reading of one user `struct iovec`; the structure is two
//!   words without padding, its `iov_base` an address word held and never followed here.
//! - `iovec_copyin`'s `mallocarray(M_WAITOK)` cannot fail in the C; here it is `ENOMEM` when
//!   `malloc(9)` answers nothing (LZ's, kept: before the allocator is registered).
//! - The userland prototypes (`readv`, `preadv`, ...) are not kernel material; `ureadc` and
//!   `uiomove` stay with `bsd`'s `kern_subr.rs`, `dofilereadv` and `dofilewritev` with the
//!   read and write paths.
//!
//! ## Redesign
//! - LZ's `sys/sys/uio.rs` named `bsd`'s `Proc` in `uio_procp`; the types are the kernel's
//!   now, `Uio<'a, G>` generic over the glue, and `bsd`'s `sys/sys/uio.rs` re-exports them
//!   with `type Uio<'a> = kernel::uio::Uio<'a, BsdProcGlue>`, so its 98 users (the 5 frozen
//!   amd64 files among them, decision 44) keep their paths, struct literals and fields.
//! - LZ's `iovec_copyin(uiov, aiov, iovcnt)` returned the caller's stack array or a raw
//!   `malloc(M_IOV)` array as a slice, which an `unsafe fn iovec_free` gave back on every
//!   path that got one (`readv`, `writev`, `preadv`, `pwritev`, and its own error paths). The
//!   array is an [`IovBuf`] now: the caller's eight iovecs and, past them, an owned
//!   `MArray<Iovec>`, freed when the buffer goes out of scope, so no path can leak it or free
//!   it twice and `iovec_free` has nothing left to do.
//! - The user array is copied in chunks of `UIO_SMALLIOV` iovecs (the C copies it in one
//!   `copyin`, LZ one iovec at a time) and only then checked, as in the C: an array that
//!   faults part-way is `EFAULT` even when an earlier length was too long (LZ checked each
//!   iovec as it copied it and answered `EINVAL` there).
//! - `UioMove`, declared by `kernel::pipe` at N2.6, lives here with the type it moves bytes
//!   through; `kernel::pipe` re-exports it.
//! - N4.7a: [`Uio::kernel_write`], a `UIO_SYSSPACE` write out of borrowed kernel bytes (the
//!   C's `aiov.iov_base = buf` over a kernel buffer, `dosendsyslog`'s): the uio borrows the
//!   bytes, so the address its iovec carries outlives every `uiomove` through it.
//! - N4.7b: [`UioMove::set_resid`], the terminal's adjustment of `uio_resid` (the bridge's
//!   fifth member).

use core::ffi::c_void;
use core::ptr;

use hal::malloc::{M_WAITOK, MArray};
use hal::sys::errno::Errno;
use hal::sys::limits::SSIZE_MAX;
use hal::sys::syslimits::IOV_MAX;
use hal::sys::types::Off;
use hal::user::UserPtr;

use crate::proc::{ProcGlue, Thread};

/// `UIO_SMALLIOV`: 8 on stack, else malloc.
pub const UIO_SMALLIOV: usize = 8;

/// `UIO_MAXIOV`: deprecated, use `IOV_MAX` instead.
pub const UIO_MAXIOV: usize = 1024;

/// `M_IOV`: the malloc type of large iovec arrays.
pub const M_IOV: i32 = 19;

/// `struct iovec`: one segment of a scatter/gather list.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Iovec {
    /// `iov_base`: base address (a user address under `UIO_USERSPACE`).
    pub iov_base: *mut c_void,
    /// `iov_len`: length.
    pub iov_len: usize,
}

impl Iovec {
    /// The structure's size in user space.
    pub const SIZE: usize = size_of::<Iovec>();

    /// An empty iovec.
    pub const fn new() -> Self {
        Self {
            iov_base: ptr::null_mut(),
            iov_len: 0,
        }
    }

    /// The iovec read from its user-space bytes (`copyin`).
    pub fn from_bytes(b: &[u8; Self::SIZE]) -> Self {
        let mut base = [0u8; size_of::<usize>()];
        let mut len = [0u8; size_of::<usize>()];
        base.copy_from_slice(&b[..size_of::<usize>()]);
        len.copy_from_slice(&b[size_of::<usize>()..]);
        Self {
            iov_base: ptr::without_provenance_mut(usize::from_ne_bytes(base)),
            iov_len: usize::from_ne_bytes(len),
        }
    }
}

impl Default for Iovec {
    fn default() -> Self {
        Self::new()
    }
}

/// `enum uio_rw`: the direction of a transfer.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)] // OpenBSD names, verbatim
pub enum UioRw {
    /// `UIO_READ`: from the kernel buffer to the iovecs.
    UIO_READ,
    /// `UIO_WRITE`: from the iovecs to the kernel buffer.
    UIO_WRITE,
}

/// `enum uio_seg`: segment flag values.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)] // OpenBSD names, verbatim
pub enum UioSeg {
    /// `UIO_USERSPACE`: from user data space.
    UIO_USERSPACE,
    /// `UIO_SYSSPACE`: from system space.
    UIO_SYSSPACE,
}

/// `struct uio`: a scatter/gather transfer in progress, for a thread of the glue `G`.
///
/// A `UIO_SYSSPACE` uio's iovecs are kernel buffers: whoever builds one vouches for them, as
/// in C (`uiomove` copies with `kcopy`, which catches a fault but not a wrong address).
pub struct Uio<'a, G: ProcGlue> {
    /// `uio_iov`/`uio_iovcnt`: the iovecs not yet consumed.
    pub uio_iov: &'a mut [Iovec],
    /// `uio_offset`: offset into file this uio corresponds to.
    pub uio_offset: Off,
    /// `uio_resid`: residual i/o count.
    pub uio_resid: usize,
    /// `uio_segflg`: see [`UioSeg`].
    pub uio_segflg: UioSeg,
    /// `uio_rw`: see [`UioRw`].
    pub uio_rw: UioRw,
    /// `uio_procp`: associated thread or NULL.
    pub uio_procp: Option<&'a Thread<G>>,
}

impl<G: ProcGlue> Uio<'_, G> {
    /// `uio_iovcnt`: number of iovecs left in the array.
    pub fn uio_iovcnt(&self) -> usize {
        self.uio_iov.len()
    }
}

impl<'a, G: ProcGlue> Uio<'a, G> {
    /// A `UIO_SYSSPACE` write out of the kernel bytes `buf` (`aiov.iov_base = buf; auio.uio_segflg
    /// = UIO_SYSSPACE; auio.uio_rw = UIO_WRITE`: `dosendsyslog`'s stashed message, its `"\r\n"`),
    /// its one iovec in `iov`, offset 0, all of `buf` to move. The uio borrows `buf` for as long
    /// as it lives, so the address the iovec carries names bytes that exist for every
    /// `uiomove` made through it; a write only reads them (`uiomove` copies out of a
    /// `UIO_WRITE` uio's iovecs), so a shared borrow is all it needs. The kernel's one way to
    /// build a `UIO_SYSSPACE` uio.
    pub fn kernel_write(
        iov: &'a mut [Iovec; 1],
        buf: &'a [u8],
        procp: Option<&'a Thread<G>>,
    ) -> Self {
        iov[0] = Iovec {
            iov_base: buf.as_ptr().cast_mut().cast::<c_void>(),
            iov_len: buf.len(),
        };
        Self {
            uio_iov: iov,
            uio_offset: 0,
            uio_resid: buf.len(),
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: procp,
        }
    }
}

impl<'a, G: ProcGlue> Uio<'a, G> {
    /// A `UIO_SYSSPACE` read into the kernel bytes `buf` (`aiov.iov_base = buf; auio.uio_segflg
    /// = UIO_SYSSPACE; auio.uio_rw = UIO_READ`: the vnode pager's `VOP_READ` of a page), its
    /// one iovec in `iov`, offset 0, all of `buf` to fill. The uio borrows `buf` mutably for as
    /// long as it lives, so the address the iovec carries names bytes that exist, and that
    /// nothing else reaches, for every `uiomove` made through it. The kernel's one way to
    /// build a reading `UIO_SYSSPACE` uio.
    pub fn kernel_read(
        iov: &'a mut [Iovec; 1],
        buf: &'a mut [u8],
        procp: Option<&'a Thread<G>>,
    ) -> Self {
        iov[0] = Iovec {
            iov_base: buf.as_mut_ptr().cast::<c_void>(),
            iov_len: buf.len(),
        };
        Self {
            uio_iov: iov,
            uio_offset: 0,
            uio_resid: buf.len(),
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: procp,
        }
    }
}

/// Where [`iovec_copyin`] puts a user's iovecs: the `UIO_SMALLIOV` the caller keeps on its
/// stack (the C's `aiov[UIO_SMALLIOV]`), or, past them, a `malloc(M_IOV)` array this buffer
/// owns and frees when it goes out of scope (the C's `iovec_free`).
pub struct IovBuf {
    small: [Iovec; UIO_SMALLIOV],
    big: Option<MArray<Iovec>>,
}

impl IovBuf {
    /// An empty buffer (nothing allocated until `iovec_copyin` needs it).
    pub const fn new() -> Self {
        Self {
            small: [Iovec::new(); UIO_SMALLIOV],
            big: None,
        }
    }
}

impl Default for IovBuf {
    fn default() -> Self {
        Self::new()
    }
}

/// The `uio` of a read or a write, as the kernel's objects move bytes through it (`bsd`'s
/// `Uio` and `uiomove`, through the glue). A bridge (`bridges.toml`).
pub trait UioMove {
    /// `uio_resid`: the bytes still to move.
    fn resid(&self) -> usize;
    /// `uiomove(buf, buf.len(), uio)`: moves `buf` (at most `resid` bytes of it) in the
    /// uio's direction: out of `buf` for a read, into it for a write. It may fault and
    /// sleep, so the pipe calls it with its lock released.
    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno>;
    /// `fp->f_flag & FNONBLOCK` of the file the I/O is on, read now: the pipe asks at every
    /// check, as the C reads `f_flag` each time round its loops (`fcntl(F_SETFL)` from
    /// another thread changes it while a reader or a writer sleeps).
    fn nonblock(&self) -> bool;
    /// `uio_rw == UIO_WRITE`: the bytes flow out of the uio (`uiomove` fills `buf`); a read
    /// moves them into it. `uvm_io` asks which way to copy through its window.
    fn is_write(&self) -> bool;
    /// `uio->uio_resid = resid`, the iovecs left as they are: `ttwrite`'s give-back of what it
    /// copied in but did not queue (`uio_resid += cc`, "the uio structure inconsistent ... it
    /// doesn't matter"), and its `uio_resid = 0` under `FLUSHO`.
    fn set_resid(&mut self, resid: usize);
}

/// `iovec_copyin(uiov, &iov, aiov, iovcnt, &resid)`: the `iovcnt` iovecs at `uiov` in
/// userland, copied into `buf` (its stack part when they fit, else an array of their own),
/// and their total length. `EINVAL` for no iovec, more than `IOV_MAX`, or a length (or the
/// total) past `SSIZE_MAX`; `EFAULT` when the array is not user memory.
pub fn iovec_copyin(
    uiov: UserPtr<Iovec>,
    buf: &mut IovBuf,
    iovcnt: u32,
) -> Result<(&mut [Iovec], usize), Errno> {
    let n = iovcnt as usize;
    let iov: &mut [Iovec] = if n > UIO_SMALLIOV {
        if n > IOV_MAX {
            return Err(Errno::EINVAL);
        }
        let Some(array) = MArray::new(M_IOV, M_WAITOK, n, |_| Iovec::new()) else {
            return Err(Errno::ENOMEM);
        };
        buf.big.insert(array)
    } else if n > 0 {
        &mut buf.small[..n]
    } else {
        return Err(Errno::EINVAL);
    };

    // The C's one copyin of the whole array, a chunk of user words at a time.
    let mut at = UserPtr::<[u8; Iovec::SIZE]>::new(uiov.addr());
    for chunk in iov.chunks_mut(UIO_SMALLIOV) {
        let mut words = [[0u8; Iovec::SIZE]; UIO_SMALLIOV];
        let words = &mut words[..chunk.len()];
        at.copyin_slice(words)?;
        for (slot, word) in chunk.iter_mut().zip(words.iter()) {
            *slot = Iovec::from_bytes(word);
        }
        at = at.checked_add(chunk.len()).ok_or(Errno::EFAULT)?;
    }

    // A read or a write returns an ssize_t (-1 on error), so a length and the total are
    // bounded by SSIZE_MAX; the sum cannot wrap, since SSIZE_MAX * 2 < SIZE_MAX.
    let max = SSIZE_MAX.unsigned_abs();
    let mut resid: usize = 0;
    for v in iov.iter() {
        if v.iov_len > max {
            return Err(Errno::EINVAL);
        }
        resid += v.iov_len;
        if resid > max {
            return Err(Errno::EINVAL);
        }
    }
    Ok((iov, resid))
}

const _: () = assert!(size_of::<Iovec>() == 2 * size_of::<usize>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    /// Where each test keeps its user iovec arrays (the double's user memory is one space,
    /// shared by the tests running in parallel: each test has its own range).
    fn user_at(off: usize) -> usize {
        testutil::USER_BASE + off
    }

    /// Writes `iovs` (base, len) as user `struct iovec`s at `uaddr`.
    fn put_iovecs(uaddr: usize, iovs: &[(usize, usize)]) {
        let words: Vec<[u8; Iovec::SIZE]> = iovs
            .iter()
            .map(|&(base, len)| {
                let mut w = [0u8; Iovec::SIZE];
                w[..8].copy_from_slice(&base.to_ne_bytes());
                w[8..].copy_from_slice(&len.to_ne_bytes());
                w
            })
            .collect();
        UserPtr::<[u8; Iovec::SIZE]>::new(uaddr)
            .copyout_slice(&words)
            .unwrap();
    }

    fn view(iov: &[Iovec]) -> Vec<(usize, usize)> {
        iov.iter().map(|v| (v.iov_base.addr(), v.iov_len)).collect()
    }

    #[test]
    fn an_iovec_reads_its_two_words() {
        let mut b = [0u8; Iovec::SIZE];
        b[..8].copy_from_slice(&0x1234_5678usize.to_ne_bytes());
        b[8..].copy_from_slice(&42usize.to_ne_bytes());
        let v = Iovec::from_bytes(&b);
        assert_eq!(v.iov_base.addr(), 0x1234_5678);
        assert_eq!(v.iov_len, 42);
        assert_eq!(Iovec::SIZE, 16);
        assert!(Iovec::default().iov_base.is_null());
    }

    #[test]
    fn up_to_eight_iovecs_stay_on_the_stack() {
        testutil::install();
        let at = user_at(0x4000);
        let want = [(0x1000, 3), (0x2000, 0), (0x3000, 10)];
        put_iovecs(at, &want);
        let before = testutil::malloc_out(M_IOV);
        let mut buf = IovBuf::new();
        let (iov, resid) = iovec_copyin(UserPtr::new(at), &mut buf, 3).unwrap();
        assert_eq!(view(iov), want);
        assert_eq!(resid, 13);
        assert!(buf.big.is_none());
        assert_eq!(testutil::malloc_out(M_IOV), before);

        let all: Vec<(usize, usize)> = (0..8).map(|i| (0x1000 * i, i)).collect();
        put_iovecs(at, &all);
        let (iov, resid) = iovec_copyin(UserPtr::new(at), &mut buf, 8).unwrap();
        assert_eq!(view(iov), all);
        assert_eq!(resid, 28);
        assert!(buf.big.is_none());
    }

    #[test]
    fn more_than_eight_take_an_array_freed_with_the_buffer() {
        testutil::install();
        let at = user_at(0x5000);
        let want: Vec<(usize, usize)> = (0..20).map(|i| (0x10_0000 + i * 64, i + 1)).collect();
        put_iovecs(at, &want);
        let before = testutil::malloc_out(M_IOV);
        let mut buf = IovBuf::new();
        let (iov, resid) = iovec_copyin(UserPtr::new(at), &mut buf, 20).unwrap();
        assert_eq!(view(iov), want);
        assert_eq!(resid, (1..=20).sum::<usize>());
        assert_eq!(
            testutil::malloc_out(M_IOV),
            before + (20 * Iovec::SIZE) as i64
        );
        drop(buf);
        assert_eq!(testutil::malloc_out(M_IOV), before);
    }

    #[test]
    fn the_count_is_checked_before_anything_is_read() {
        testutil::install();
        // A NULL array: the count alone decides.
        let null = UserPtr::new(0);
        let mut buf = IovBuf::new();
        assert_eq!(iovec_copyin(null, &mut buf, 0).err(), Some(Errno::EINVAL));
        let past = (IOV_MAX + 1) as u32;
        assert_eq!(
            iovec_copyin(null, &mut buf, past).err(),
            Some(Errno::EINVAL)
        );
        assert_eq!(
            iovec_copyin(null, &mut buf, u32::MAX).err(),
            Some(Errno::EINVAL)
        );
        // IOV_MAX itself is read.
        assert_eq!(
            iovec_copyin(null, &mut buf, IOV_MAX as u32).err(),
            Some(Errno::EFAULT)
        );
        assert_eq!(iovec_copyin(null, &mut buf, 1).err(), Some(Errno::EFAULT));
    }

    #[test]
    fn iov_max_iovecs_are_read_whole() {
        testutil::install();
        let at = user_at(0x8000);
        let want: Vec<(usize, usize)> = (0..IOV_MAX).map(|i| (i, 1)).collect();
        put_iovecs(at, &want);
        let mut buf = IovBuf::new();
        let (iov, resid) = iovec_copyin(UserPtr::new(at), &mut buf, IOV_MAX as u32).unwrap();
        assert_eq!(iov.len(), IOV_MAX);
        assert_eq!(view(iov), want);
        assert_eq!(resid, IOV_MAX);
    }

    #[test]
    fn a_length_or_the_total_past_ssize_max_is_einval() {
        testutil::install();
        let at = user_at(0x6000);
        let max = SSIZE_MAX.unsigned_abs();
        let mut buf = IovBuf::new();
        // The table of the C's checks: (lengths, answer).
        let cases: [(&[usize], Result<usize, Errno>); 6] = [
            (&[max], Ok(max)),
            (&[max + 1], Err(Errno::EINVAL)),
            (&[usize::MAX], Err(Errno::EINVAL)),
            (&[max / 2 + 1, max / 2 + 1], Err(Errno::EINVAL)),
            (&[max / 2, max / 2 + 1], Ok(max)),
            (&[1, 2, 3, 4, 5, 6, 7, 8, 9, max - 45], Ok(max)),
        ];
        for (lens, answer) in cases {
            let iovs: Vec<(usize, usize)> = lens.iter().map(|&l| (0x1000, l)).collect();
            put_iovecs(at, &iovs);
            let got =
                iovec_copyin(UserPtr::new(at), &mut buf, lens.len() as u32).map(|(_, resid)| resid);
            assert_eq!(got, answer, "{lens:?}");
        }
    }

    #[test]
    fn an_array_that_faults_part_way_is_efault_whatever_it_held() {
        testutil::install();
        // Ten iovecs whose last two words lie past the double's user memory, the first with
        // a length past SSIZE_MAX: the C copies the whole array first, so the fault wins.
        let at = testutil::USER_BASE + testutil::USER_LEN - 9 * Iovec::SIZE;
        let mut iovs: Vec<(usize, usize)> = (0..9).map(|_| (0x1000, 1)).collect();
        iovs[0].1 = usize::MAX;
        put_iovecs(at, &iovs);
        let mut buf = IovBuf::new();
        assert_eq!(
            iovec_copyin(UserPtr::new(at), &mut buf, 10).err(),
            Some(Errno::EFAULT)
        );
        // The nine that are there: the length decides.
        assert_eq!(
            iovec_copyin(UserPtr::new(at), &mut buf, 9).err(),
            Some(Errno::EINVAL)
        );
    }

    #[test]
    #[ignore = "reads reference/openbsd-src (just test-ref)"]
    fn constants_match_the_c() {
        let defs = crate::reftest::defines("sys/sys/uio.h");
        let _ = crate::reftest::assert_defines!(defs; UIO_SMALLIOV, UIO_MAXIOV);
        let defs = crate::reftest::defines("sys/sys/malloc.h");
        let _ = crate::reftest::assert_defines!(defs; M_IOV);
    }
}
/* </TESTS> */
