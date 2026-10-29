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
 * Copyright (c) 1994 Christopher G. Demetriou
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)kern_physio.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! physio(9): "physical I/O" on behalf of a user, directly between a raw (character) device
//! and the user's buffers, bypassing the buffer cache.
//!
//! Upstream: sys/kern/kern_physio.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_physio.rs@3c62ede68802
//!
//! The routines are described in Leffler, et al.: The Design and Implementation of the 4.3BSD
//! UNIX Operating System (Addison Welley, 1989), on pages 231-233. Comments in brackets are
//! from their pseudo-code implementation.
//!
//! A raw device's `d_read`/`d_write` (`rdread`, `sdread`, ...) calls [`physio`] with its
//! strategy routine and a `minphys` that bounds one transfer; physio wires each piece of the
//! user's buffer (`uvm_vslock_device`, which bounces it through DMA-reachable pages when the
//! device cannot reach them), maps it into the kernel (`vmapbuf`:
//! [`crate::vm::glue::DeviceLock::vmapbuf`]), hands one private buffer to the strategy
//! routine and waits for it.
//!
//! ## Deviations
//! - The strategy routine is a closure that takes the buffer's token (`bsd`'s adapter parks
//!   it and calls the driver's `fn(&'static Buf)`); `minphys` is a `fn(&Buf)`; `flags` is a
//!   `b_flags` value (`B_READ` or `B_WRITE`).
//! - `KASSERTMSG` is `kassert!` (the message is the C's comment).
//! - `pool_get(&bufpool, PR_WAITOK)` cannot fail in C; a `None` here panics.
//! - A thread without an address space (a kernel thread: the C would use `proc0`'s) has no
//!   user buffer to wire: `EFAULT`, as `sysctl_vslock`.
//!
//! ## Redesign
//! - LZ's buffer was a `bufpool` item written in place and named as `&'static Buf` until the
//!   `pool_put`; here it is a `bufpl` box on a list of physio's own (an `OwnedList`, never
//!   `bufhead`: `numbufs` is untouched, as the C's `bufpool` buffer is not the cache's), and
//!   physio holds its `B_BUSY` token, which the strategy routine consumes and `biodone` parks
//!   in the buffer for physio to take back at `B_DONE` (under the cache's lock, the C's
//!   `splbio`, so no wakeup is lost). The list's `take` frees the box only when no count is
//!   left on it (`hal::owned`'s I4): a driver that kept the buffer past `biodone` panics
//!   instead of writing into a freed item.
//! - `vmapbuf`/`vunmapbuf` (the machine's `vm_machdep.c`) are `kernel::vm::km::PhysWindow`,
//!   made by the `DeviceLock` that wired the range and borrowing it, so the window is gone
//!   before the pages are unwired (the C's `vunmapbuf`, then `uvm_vsunlock_device`); the
//!   window is shared with user space by design, so it gives `b_data`'s address, never a
//!   slice.

use core::ptr;

use hal::owned::{OwnedList, SecondaryRef};
use hal::pool::PR_WAITOK;
use hal::sleep::{INFSLP, PRIBIO};
use hal::sys::errno::Errno;
use hal::sys::mman::{PROT_READ, PROT_WRITE};
use hal::sys::types::{Daddr, Dev, Off};

use super::bio::{CACHE, DEV_BSHIFT, brelvp_buf};
use super::buf::{
    B_BUSY, B_DONE, B_ERROR, B_PHYS, B_RAW, B_READ, B_WRITE, BUFPL, Buf, BufRef, ByBufhead, MAXPHYS,
};
use crate::assert::{kassert, kpanic};
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;
use crate::vm::glue::{uvm_vslock_device, uvm_vsunlock_device};

/// `DEV_BSIZE`: the block size `b_blkno` counts in.
const DEV_BSIZE: Off = 1 << DEV_BSHIFT;

/// Do "physical I/O" on behalf of the user thread `p`: transfer `uio` (user space, at a
/// `DEV_BSIZE` multiple offset) to or from `dev` through `strategy`, at most `minphys` bytes
/// at a time.
///
/// `EINVAL` when the offset is not block aligned; otherwise the first transfer error (the
/// buffer's `b_error`, `EIO` without one) or a wiring failure. On return `uio` accounts for
/// what was transferred, including a short transfer at the end of the device, which ends
/// the I/O without an error.
pub fn physio<G: ProcGlue>(
    strategy: &mut dyn FnMut(BufRef),
    dev: Dev,
    flags: i64,
    minphys: fn(&Buf),
    uio: &mut Uio<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if uio.uio_offset % DEV_BSIZE != 0 {
        return Err(Errno::EINVAL);
    }
    let flags = flags & (B_READ | B_WRITE);

    // Create a buffer: a bufpl item of physio's own, its token held here.
    let mut own: OwnedList<ByBufhead> = OwnedList::new();
    let Some(b) = BUFPL.get(PR_WAITOK, Buf::new) else {
        kpanic(format_args!("physio: pool_get"));
    };
    own.push(b);
    let mut r = SecondaryRef::new();
    r.set(own.last());
    let mut tok = BufRef::acquire(&mut CACHE.lock(), r);

    // [set up the fixed part of the buffer for a transfer]
    // (b_vnbufs is on no list: a fresh header's.)
    {
        let bp = tok.buf();
        bp.set_dev(dev);
        bp.set_error(None);
        bp.set_proc_addr(ptr::from_ref(p).expose_provenance());
        bp.set_flags(B_BUSY);
    }
    let vm = p.vmspace_share();

    let mut error = Ok(());

    // [while there are data to transfer and no I/O error]
    'done: for i in 0..uio.uio_iovcnt() {
        while uio.uio_iov[i].iov_len > 0 {
            let iov_base = uio.uio_iov[i].iov_base.addr();
            let bp = tok.buf();

            // [mark the buffer busy for physical I/O] (i.e. set B_PHYS (because it's an I/O
            // to user memory), and B_RAW, because B_RAW is to be "Set by physio for raw
            // transfers.", in addition to the "busy" and read/write flag.)
            bp.clr(B_DONE | B_ERROR);
            bp.set(B_BUSY | B_PHYS | B_RAW | flags);

            // [set up the buffer for a maximum-sized transfer]
            bp.set_blkno((uio.uio_offset >> DEV_BSHIFT) as Daddr);
            bp.set_bcount(physio_bcount(uio.uio_iov[i].iov_len));

            // [call minphys to bound the transfer size] and remember the amount of data to
            // transfer, for later comparison.
            minphys(bp);
            let todo = bp.bcount();
            kassert!(todo >= 0); // minphys broken
            let todo = todo as usize;

            // [lock the part of the user address space involved in the transfer]
            let access = if flags & B_READ != 0 {
                PROT_READ | PROT_WRITE
            } else {
                PROT_READ
            };
            let locked = match &vm {
                Some(vm) => uvm_vslock_device(&vm.vm_map, iov_base, todo, access, Some(&p.p_ru)),
                None => Err(Errno::EFAULT),
            };
            let lock = match locked {
                Ok(lock) => lock,
                Err(e) => {
                    error = Err(e);
                    break 'done;
                }
            };
            // The C's `map`: the bounce buffer's kernel address; else vmapbuf, which keeps
            // the user's address in b_saveaddr and points b_data into the window.
            let window = if let Some(bounce) = lock.bounce_va() {
                bp.set_data_addr(bounce.as_usize());
                None
            } else {
                let w = lock.vmapbuf();
                bp.set_saveaddr(iov_base);
                bp.set_data_addr(lock.window_addr(&w));
                Some(w)
            };

            // [call strategy to start the transfer]; the buffer is reached through a counted
            // link meanwhile.
            let anchor = buf_anchor(&tok);
            strategy(tok);

            // Note that the raise/wait/lower/get error steps below would be done by
            // biowait(), but we want to unlock the address space before we lower the
            // priority.
            //
            // [raise the priority level to splbio] [wait for the transfer to complete]
            let back = {
                let Some(bp) = anchor.get() else {
                    kpanic(format_args!("physio: no buffer"));
                };
                let mut g = CACHE.lock();
                while !bp.isset(B_DONE) {
                    let _ = g.sleep_nsec(ptr::from_ref(bp), PRIBIO + 1, "physio", INFSLP);
                }
                let back = bp.take_parked_locked(&mut g);
                drop(g);
                back
            };
            drop(anchor);
            let Some(t) = back else {
                kpanic(format_args!("physio: buffer done without its token"));
            };
            tok = t;
            let bp = tok.buf();
            // Mark it busy again, so nobody else will use it. [lower the priority level]
            bp.set(B_BUSY);

            // [unlock the part of the address space previously locked]
            let mapped = window.is_some();
            drop(window);
            if mapped {
                // vunmapbuf: the window went, b_data is the user's address again.
                bp.set_data_addr(bp.saveaddr());
                bp.set_saveaddr(0);
            }
            uvm_vsunlock_device(lock, iov_base, todo);

            // remember error value (save a splbio/splx pair)
            if bp.isset(B_ERROR) {
                error = Err(bp.error().unwrap_or(Errno::EIO));
            }

            // [deduct the transfer size from the total number of data to transfer]
            let resid = bp.resid();
            kassert!(i64::try_from(resid).is_ok()); // strategy broken
            let done = bp.bcount() - resid as i64;
            kassert!(done >= 0); // strategy broken
            kassert!(done as usize <= todo); // strategy broken
            physio_advance(uio, i, done as usize);

            // Now, check for an error. Also, handle weird end-of-disk semantics.
            if error.is_err() || (done as usize) < todo {
                break 'done;
            }
        }
    }

    // done:
    // [clean up the state of the buffer]
    // XXXCDC: is this necessary?
    if tok.buf().vnode_bufs().is_some() {
        brelvp_buf(&tok);
    }
    let Some(r) = tok.into_counted(&mut CACHE.lock()) else {
        kpanic(format_args!("physio: a driver's buffer"));
    };
    drop(own.take(r));

    error
}

/// A counted link to the token's buffer, which physio keeps while the strategy routine holds
/// the token.
fn buf_anchor(t: &BufRef) -> SecondaryRef<ByBufhead> {
    match t.counted() {
        Some(r) => r,
        None => kpanic(format_args!("physio: a driver's buffer")),
    }
}

/// The `b_bcount` physio starts a piece with: the iovec's length, limited to `LONG_MAX`
/// because `iov_len` is a `size_t` (unsigned) and `b_bcount` a `long` (signed), before the
/// provided minphys bounds it.
fn physio_bcount(iov_len: usize) -> i64 {
    i64::try_from(iov_len).unwrap_or(i64::MAX)
}

/// Accounts `done` transferred bytes of the iovec `i` of `uio`: the iovec shrinks and moves
/// on, the offset grows, the residual count shrinks.
fn physio_advance<G: ProcGlue>(uio: &mut Uio<'_, G>, i: usize, done: usize) {
    let iov = &mut uio.uio_iov[i];
    iov.iov_len -= done;
    iov.iov_base = iov.iov_base.wrapping_byte_add(done);
    uio.uio_offset += done as Off;
    uio.uio_resid -= done;
}

/// Leffler, et al., says on p. 231: "The minphys() routine is called by physio() to adjust
/// the size of each I/O transfer before the latter is passed to the strategy routine..."
///
/// so, just adjust the buffer's count accounting to `MAXPHYS` here.
pub fn minphys(bp: &Buf) {
    if bp.bcount() > MAXPHYS as i64 {
        bp.set_bcount(MAXPHYS as i64);
    }
}

const _: () = assert!(B_WRITE == 0);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::ffi::c_void;
    use std::sync::Mutex;
    use std::vec::Vec;

    use hal::intr::IPL_BIO;
    use hal::sys::mman::{MADV_NORMAL, MAP_INHERIT_COPY};
    use hal::sys::param::PAGE_SIZE;

    use super::*;
    use crate::event::tests::Kg;
    use crate::fs::bio::biodone;
    use crate::fs::buf::B_ASYNC;
    use crate::uio::{Iovec, UioRw, UioSeg};
    use crate::vm::map::tests::{setup, thread, user_vmspace};
    use crate::vm::map::uvm_mapanon;
    use crate::vm::types::{UVM_FLAG_COPYONW, UVM_FLAG_FIXED, uvm_mapflag};

    fn uio_over<'a>(iov: &'a mut [Iovec], offset: Off) -> Uio<'a, Kg> {
        let resid = iov.iter().map(|v| v.iov_len).sum();
        Uio {
            uio_iov: iov,
            uio_offset: offset,
            uio_resid: resid,
            uio_segflg: UioSeg::UIO_USERSPACE,
            uio_rw: UioRw::UIO_READ,
            uio_procp: None,
        }
    }

    fn iov(base: usize, len: usize) -> Iovec {
        Iovec {
            iov_base: ptr::without_provenance_mut::<c_void>(base),
            iov_len: len,
        }
    }

    #[test]
    fn minphys_clamps_to_maxphys() {
        let bp = Buf::new();
        bp.set_bcount(3 * MAXPHYS as i64 + 1);
        minphys(&bp);
        assert_eq!(bp.bcount(), MAXPHYS as i64);

        bp.set_bcount(MAXPHYS as i64);
        minphys(&bp);
        assert_eq!(bp.bcount(), MAXPHYS as i64);

        bp.set_bcount(512);
        minphys(&bp);
        assert_eq!(bp.bcount(), 512);
    }

    #[test]
    fn bcount_is_limited_to_long_max() {
        assert_eq!(physio_bcount(0), 0);
        assert_eq!(physio_bcount(8192), 8192);
        assert_eq!(physio_bcount(i64::MAX as usize), i64::MAX);
        assert_eq!(physio_bcount(usize::MAX), i64::MAX);
    }

    #[test]
    fn advance_moves_the_iovec_the_offset_and_the_residual() {
        let (a, b) = (0x10_0000usize, 0x20_0000usize);
        let mut v = [iov(a, 4096), iov(b, 1024)];
        let mut uio = uio_over(&mut v, 2048);

        // A full MAXPHYS-bounded piece, then a short one (the end of the disk).
        physio_advance(&mut uio, 0, 3072);
        assert_eq!(uio.uio_iov[0].iov_len, 1024);
        assert_eq!(uio.uio_iov[0].iov_base.addr(), a + 3072);
        assert_eq!(uio.uio_offset, 2048 + 3072);
        assert_eq!(uio.uio_resid, 5120 - 3072);

        physio_advance(&mut uio, 0, 1024);
        physio_advance(&mut uio, 1, 512);
        assert_eq!(uio.uio_iov[0].iov_len, 0);
        assert_eq!(uio.uio_iov[1].iov_len, 512);
        assert_eq!(uio.uio_iov[1].iov_base.addr(), b + 512);
        assert_eq!(uio.uio_offset, 2048 + 4608);
        assert_eq!(uio.uio_resid, 512);
    }

    /// What the test strategy saw of one piece.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Piece {
        blkno: Daddr,
        bcount: i64,
        data: usize,
        flags: i64,
    }

    /// The user thread of `pid` with `len` bytes of anonymous memory mapped at `va` in a
    /// vmspace of its own, and bufpl ready.
    fn user_thread(pid: i32, va: usize, len: usize) -> crate::event::tests::T {
        let p = thread(pid);
        let vm = user_vmspace();
        let mut a = va;
        let flags = uvm_mapflag(
            PROT_READ | PROT_WRITE,
            PROT_READ | PROT_WRITE,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
        );
        uvm_mapanon(&vm.vm_map, &mut a, len, 0, flags).expect("mapanon");
        drop(p.set_vmspace(Some(vm)));
        let _ = BUFPL.init(IPL_BIO, 0, "bufpl");
        p
    }

    #[test]
    fn misaligned_offsets_are_einval_before_any_io() {
        let _g = setup();
        let p = thread(5101);
        for offset in [1, 511, 513, -1] {
            let mut v = [iov(0x5100_0000, 512)];
            let mut uio = uio_over(&mut v, offset);
            let mut strategy = |_t: BufRef| panic!("physio called the strategy routine");
            assert_eq!(
                physio(&mut strategy, 0, B_READ, minphys, &mut uio, p),
                Err(Errno::EINVAL),
                "offset {offset}"
            );
            assert_eq!(uio.uio_resid, 512);
            assert_eq!(uio.uio_offset, offset);
        }
    }

    /// A transfer in pieces of `minphys` (a quarter of MAXPHYS here) over two iovecs, each
    /// piece through the window at the user's address (no MMU on the host: the window is the
    /// address itself), the token back after each `biodone`, then the buffer freed.
    #[test]
    fn a_transfer_goes_in_pieces_through_the_window() {
        let _g = setup();
        let va = 0x5200_0000;
        let p = user_thread(5102, va, 8 * PAGE_SIZE);
        fn quarter(bp: &Buf) {
            if bp.bcount() > MAXPHYS as i64 / 4 {
                bp.set_bcount(MAXPHYS as i64 / 4);
            }
        }
        let q = MAXPHYS / 4;
        let seen = Mutex::new(Vec::new());
        let mut strategy = |t: BufRef| {
            let bp = t.buf();
            seen.lock().unwrap().push(Piece {
                blkno: bp.blkno(),
                bcount: bp.bcount(),
                data: bp.data_addr(),
                flags: bp.flags(),
            });
            assert!(bp.is_held());
            bp.set_resid(0);
            biodone(t);
        };
        let first = q + 512;
        let second = 1024;
        let base2 = va + 2 * PAGE_SIZE + 512;
        let mut v = [iov(va + 512, first), iov(base2, second)];
        let mut uio = uio_over(&mut v, 4096);
        // Only B_READ and B_WRITE of the flags count.
        physio(&mut strategy, 7, B_READ | B_ASYNC, quarter, &mut uio, p).expect("physio");

        let seen = seen.into_inner().unwrap();
        let rw = B_BUSY | B_PHYS | B_RAW | B_READ;
        assert_eq!(
            seen,
            [
                Piece {
                    blkno: 8,
                    bcount: q as i64,
                    data: va + 512,
                    flags: rw
                },
                Piece {
                    blkno: 8 + (q as Daddr >> DEV_BSHIFT),
                    bcount: 512,
                    data: va + 512 + q,
                    flags: rw
                },
                Piece {
                    blkno: 8 + ((q + 512) as Daddr >> DEV_BSHIFT),
                    bcount: second as i64,
                    data: base2,
                    flags: rw
                },
            ]
        );
        assert_eq!(uio.uio_resid, 0);
        assert_eq!(uio.uio_offset, 4096 + (first + second) as Off);
    }

    /// A short piece (the end of the device) ends the I/O without an error, with what was
    /// done accounted; an error is the buffer's `b_error`, `EIO` without one.
    #[test]
    fn a_short_piece_ends_the_io_and_errors_are_returned() {
        let _g = setup();
        let va = 0x5300_0000;
        let p = user_thread(5103, va, 2 * PAGE_SIZE);

        let mut calls = 0;
        let mut short = |t: BufRef| {
            calls += 1;
            t.buf().set_resid(512);
            biodone(t);
        };
        let mut v = [iov(va, 2048)];
        let mut uio = uio_over(&mut v, 0);
        assert_eq!(physio(&mut short, 0, B_WRITE, minphys, &mut uio, p), Ok(()));
        assert_eq!(calls, 1);
        assert_eq!(uio.uio_resid, 512);
        assert_eq!(uio.uio_offset, 1536);

        for (e, want) in [(Some(Errno::ENXIO), Errno::ENXIO), (None, Errno::EIO)] {
            let mut failing = |t: BufRef| {
                let bp = t.buf();
                bp.set(B_ERROR);
                bp.set_error(e);
                bp.set_resid(usize::try_from(bp.bcount()).unwrap());
                biodone(t);
            };
            let mut v = [iov(va, 1024)];
            let mut uio = uio_over(&mut v, 0);
            assert_eq!(
                physio(&mut failing, 0, B_READ, minphys, &mut uio, p),
                Err(want)
            );
            assert_eq!(uio.uio_resid, 1024, "nothing was transferred");
        }
    }

    /// A user range that is not mapped fails the wiring (`uvm_vslock_device`) before the
    /// strategy routine runs (the buffer goes back all the same: the list's `take`).
    #[test]
    fn an_unmapped_buffer_fails_before_the_strategy() {
        let _g = setup();
        let p = user_thread(5104, 0x5400_0000, PAGE_SIZE);
        let mut v = [iov(0x5480_0000, 512)];
        let mut uio = uio_over(&mut v, 0);
        let mut strategy = |_t: BufRef| panic!("physio called the strategy routine");
        assert!(physio(&mut strategy, 0, B_READ, minphys, &mut uio, p).is_err());
        assert_eq!(uio.uio_resid, 512);
    }

    /// A strategy that parks the token (a driver's queue) and completes later, from another
    /// thread: physio sleeps on the buffer until `biodone`. The driver lets its link go
    /// before `biodone`, as a driver must (a link kept past it would make physio's free
    /// panic).
    #[test]
    fn physio_waits_for_a_later_biodone() {
        use std::sync::mpsc;
        let _g = setup();
        let va = 0x5500_0000;
        let p = user_thread(5105, va, PAGE_SIZE);
        let (tx, rx) = mpsc::channel::<SecondaryRef<ByBufhead>>();
        let driver = std::thread::spawn(move || {
            let r = rx.recv().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            let t = r.get().and_then(Buf::take_parked).expect("parked");
            drop(r);
            t.buf().set_resid(0);
            biodone(t);
        });
        let mut strategy = |t: BufRef| {
            let r = t.counted().expect("physio's buffer is counted");
            Buf::park(t);
            tx.send(r).unwrap();
        };
        let mut v = [iov(va, 512)];
        let mut uio = uio_over(&mut v, 0);
        physio(&mut strategy, 0, B_READ, minphys, &mut uio, p).expect("physio");
        driver.join().unwrap();
        assert_eq!(uio.uio_resid, 0);
    }
}
/* </TESTS> */
