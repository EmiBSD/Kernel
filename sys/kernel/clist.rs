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
 * Copyright (c) 1993, 1994 Theo de Raadt
 * All rights reserved.
 *
 * Per Lindqvist <pgd@compuram.bbt.se> supplied an almost fully working
 * set of true clist functions that this is very loosely based on.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*-
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)tty.h	8.6 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The character lists of the terminals: `kern/tty_subr.c` and `struct clist` of
//! `<sys/tty.h>`.
//!
//! Upstream: sys/kern/tty_subr.c @ 3ce1f3f79392
//! Upstream: sys/sys/tty.h @ 3ce1f3f79392
//! LZ: sys/kern/tty_subr.rs@3c62ede68802
//! LZ: sys/sys/tty.rs@3c62ede68802
//!
//! Clists are really ring buffers. The `c_cc`, `c_cf`, `c_cl` fields have exactly the same
//! behaviour as in true clists. A queue allocated without quoting has no `TTY_QUOTE`
//! functionality, which saves memory and CPU time: a line discipline that does not need it
//! (SLIP wanting a 32K ring) sets it up without the quote bitmap.
//!
//! ## Deviations
//! - The cursors are indices into the ring (LZ's): `firstc`/`nextc` hand out `Option<usize>`
//!   where the C passes `u_char *` (NULL is `None`), and the `int *c`, `int *cc`
//!   out-parameters are `&mut i32`. `c_ce` is not kept (it is `c_cs + c_cn`).
//! - `getc` and `unputc` return `-1` for an empty queue, as in C (a character is never
//!   negative, `TTY_QUOTE` included); `b_to_q`/`q_to_b` take the buffer as a slice and
//!   return `usize`.
//! - A bit past the quote bitmap (`ndqb`'s `isset(c_cq, i)` after the last character) reads
//!   as clear instead of reading past the allocation (LZ's).
//! - `clalloc` panics if `malloc(M_WAITOK)` fails (`malloc(9)` cannot sleep for every size
//!   yet; `M_WAITOK` cannot fail in C) (LZ's).
//!
//! ## Redesign
//! - LZ's `struct clist` was six `Cell`s, its ring and quote bitmap raw `malloc(M_TTYS)`
//!   pointers reached as `&[Cell<u8>]` through `unsafe` (`ring`, `quote`) and scrubbed by two
//!   volatile `unsafe` stores in `clfree`, every operation bracketed by `spltty()`/`splx()`.
//!   Here a [`Clist`] owns its ring: the counts, the cursors and the two `MArray<u8>` blocks
//!   (`M_TTYS`, counted in `vmstat -m`'s `ttys` row as the C's) are a `Ring` inside a
//!   `hal::mutex::Mutexed` at `IPL_TTY`, the C's per-operation `spltty()` made a lock that
//!   owns what it guards; `clfree` scrubs with `libkern::explicit_bzero` and frees by
//!   dropping the blocks.
//! - `c_cc` and `c_cn` are also kept as relaxed words beside the ring ([`Clist::c_cc`],
//!   [`Clist::c_cn`]), written under the ring's lock by every operation, for the readers the
//!   C lets test them without `spltty()` (the drivers' `t_outq.c_cc`, the kqueue filters,
//!   `ttwrite`'s high-water tests): the same race the C has, on an atomic word.
//! - The pty sleeps on `&tp->t_outq.c_cf` and `&tp->t_rawq.c_cf` as channels distinct from
//!   the queue's own address: [`Clist::cf_chan`] is the address of the `c_cc` word, inside the
//!   `#[repr(C)]` structure past the ring, so never the structure's own.
//! - `catq` and the queue exchange of `TIOCSETA` (`tmp = *a; *a = *b; *b = tmp`, LZ's
//!   `Clist::swap`) take both rings' locks, the lower address first, and move the rings (or
//!   the characters) without letting either go between; `ttypend`'s header copy over the
//!   live ring (LZ's `copy_header`) has no counterpart: the tty requeues in place.
//! - [`Clist::peek_with`] is the drivers' "`ndqb(q, 0)` then read from `c_cf`" (ucom's and
//!   wsdisplay's start routines, LZ's two `unsafe` slices over `c_cs`): the contiguous run at
//!   the head of the queue, lent under the ring's lock.
//! - A `Clist` that is all zero bytes (a zeroed softc embedding one: `uhid`, `ugen`) is a
//!   valid, unallocated queue: its lock is a `Mutexed` at `IPL_NONE` (hal's documented zero),
//!   its counts and cursors 0, its blocks `None` (the `MArray`'s null pointer niche), its
//!   words 0. Those two drivers keep the C's exclusion (their own `splusb()` and the kernel
//!   lock) until N7 gives them [`Clist::new_at`] with their level.

use hal::intr::IPL_TTY;
use hal::malloc::{M_WAITOK, MArray};
use hal::mutex::{Mutexed, MutexedGuard};

use crate::assert::kpanic;
use crate::proc::I32Word;

/// `TTY_QUOTE` (`<sys/tty.h>`): character quoted.
pub const TTY_QUOTE: i32 = 0x0000_0100;

/// `M_TTYS`: allocated tty structures (`vmstat -m`'s `ttys`).
pub const M_TTYS: i32 = 62;

/// Bits per byte (`NBBY`, `<sys/select.h>`).
const NBBY: usize = u8::BITS as usize;

/// `QMEM(n)`: the bytes of a quote bitmap for `n` characters.
pub const fn qmem(n: usize) -> usize {
    ((n - 1) / NBBY) + 1
}

/// A queue's ring buffer and its cursors, under the queue's lock.
#[derive(Default)]
struct Ring {
    /// `c_cc`: count of characters in queue.
    cc: i32,
    /// `c_cn`: total ring buffer length.
    cn: i32,
    /// `c_cf`: index of the first character (meaningful while `cc > 0`).
    cf: usize,
    /// `c_cl`: index of the next open character (meaningful while `cc > 0`).
    cl: usize,
    /// `c_cs`: the ring buffer (`c_cn` bytes), `None` before `clalloc`.
    cs: Option<MArray<u8>>,
    /// `c_cq`: the quote bitmap (`QMEM(c_cn)` bytes), `None` when the queue has none.
    cq: Option<MArray<u8>>,
}

impl Ring {
    /// `c_cs` as a slice, empty before `clalloc`.
    fn cs(&mut self) -> &mut [u8] {
        match &mut self.cs {
            Some(a) => a,
            None => &mut [],
        }
    }

    /// `isset(c_cq, i)`; a bit past the map (or no map) is clear.
    fn qisset(&self, i: usize) -> bool {
        self.cq
            .as_ref()
            .and_then(|m| m.get(i / NBBY))
            .is_some_and(|b| b & (1 << (i % NBBY)) != 0)
    }

    /// `setbit(c_cq, i)` or `clrbit(c_cq, i)` when the queue has a map.
    fn qput(&mut self, i: usize, set: bool) {
        if let Some(m) = &mut self.cq {
            let b = &mut m[i / NBBY];
            if set {
                *b |= 1 << (i % NBBY);
            } else {
                *b &= !(1 << (i % NBBY));
            }
        }
    }

    /// `clrbits(c_cq, off, len)` when the queue has a map.
    fn qclear(&mut self, off: usize, len: usize) {
        if let Some(m) = &mut self.cq {
            clrbits(m, off, len);
        }
    }

    /// The contiguous run from `c_cf`: to `c_cl`, or to the ring's end when it wraps (the
    /// `cc` of the copy loops).
    fn run(&self) -> usize {
        if self.cf >= self.cl {
            self.cn as usize - self.cf
        } else {
            self.cl - self.cf
        }
    }

    /// The empty queue's cursors (`c_cf = c_cl = NULL`).
    fn reset_if_empty(&mut self) {
        if self.cc == 0 {
            self.cf = 0;
            self.cl = 0;
        }
    }

    /// `getc` under the lock.
    fn getc(&mut self) -> i32 {
        if self.cc == 0 {
            return -1;
        }
        let cf = self.cf;
        let cs = self.cs();
        let mut c = i32::from(cs[cf]);
        cs[cf] = 0;
        if self.qisset(cf) {
            c |= TTY_QUOTE;
        }
        self.qput(cf, false);
        self.cf = if cf + 1 == self.cn as usize {
            0
        } else {
            cf + 1
        };
        self.cc -= 1;
        self.reset_if_empty();
        c
    }

    /// `putc` under the lock.
    fn putc(&mut self, c: i32) -> i32 {
        if self.cc == self.cn {
            return -1;
        }
        if self.cc == 0 {
            if self.cs.is_none() {
                kpanic(format_args!("putc: tty has no clist"));
            }
            self.cf = 0;
            self.cl = 0;
        }
        let i = self.cl;
        self.cs()[i] = (c & 0xff) as u8;
        self.qput(i, c & TTY_QUOTE != 0);
        self.cc += 1;
        self.cl = if i + 1 == self.cn as usize { 0 } else { i + 1 };
        0
    }

    /// `ndflush` under the lock.
    fn ndflush(&mut self, count: i32) {
        if count == self.cc {
            self.cc = 0;
            self.cf = 0;
            self.cl = 0;
            return;
        }
        let mut count = count;
        // optimize this while loop
        while count > 0 && self.cc > 0 {
            let cc = (self.run() as i32).min(count);
            count -= cc;
            self.cc -= cc;
            self.cf += cc as usize;
            if self.cf == self.cn as usize {
                self.cf = 0;
            }
        }
        self.reset_if_empty();
    }
}

/// `struct clist`: a ring buffer of characters, with an optional bit per character for
/// `TTY_QUOTE`. Its lock (`IPL_TTY`, the C's `spltty()`) owns the ring; the counts are also
/// readable without it ([`Clist::c_cc`]).
#[repr(C)]
pub struct Clist {
    /// The ring, its cursors and its blocks, under the queue's lock.
    ring: Mutexed<Ring>,
    /// `c_cc`, read without the lock (the drivers', the filters'); its address is the
    /// queue's second channel ([`Clist::cf_chan`]).
    pub c_cc: I32Word,
    /// `c_cn`, read without the lock.
    pub c_cn: I32Word,
}

impl Clist {
    /// An unallocated queue, locked at `IPL_TTY`.
    pub const fn new() -> Self {
        Self::new_at(IPL_TTY)
    }

    /// An unallocated queue whose lock blocks interrupts up to `ipl` (a queue outside a tty:
    /// `uhid`'s and `ugen`'s at `IPL_USB`).
    pub const fn new_at(ipl: i32) -> Self {
        Self {
            ring: Mutexed::new(
                ipl,
                Ring {
                    cc: 0,
                    cn: 0,
                    cf: 0,
                    cl: 0,
                    cs: None,
                    cq: None,
                },
            ),
            c_cc: I32Word::new(0),
            c_cn: I32Word::new(0),
        }
    }

    /// The ring, locked; the words are refreshed by [`Clist::unlock`].
    fn lock(&self) -> MutexedGuard<'_, Ring> {
        self.ring.lock()
    }

    /// Publishes the ring's count in the unlocked word, then lets the lock go.
    fn unlock(&self, g: MutexedGuard<'_, Ring>) {
        self.c_cc.set(g.cc);
        drop(g);
    }

    /// [`Clist::unlock`] after an operation that may have changed the ring's size too.
    fn unlock_all(&self, g: MutexedGuard<'_, Ring>) {
        self.c_cn.set(g.cn);
        self.unlock(g);
    }

    /// The channel the pty sleeps on as `&q->c_cf` (distinct from the queue's own address).
    pub fn cf_chan(&self) -> *const () {
        core::ptr::from_ref(&self.c_cc).cast()
    }

    /// `clalloc`: initialize a particular clist. Ok, they are really ring buffers, of the
    /// specified length, with/without quoting support. A queue set up before is freed first
    /// (the C would leak it).
    pub fn clalloc(&self, size: i32, quot: bool) {
        let n = usize::try_from(size).unwrap_or(0);
        let Some(cs) = MArray::new(M_TTYS, M_WAITOK, n, |_| 0u8) else {
            kpanic(format_args!("clalloc: out of memory"));
        };
        let cq = if quot {
            let Some(cq) = MArray::new(M_TTYS, M_WAITOK, qmem(n), |_| 0u8) else {
                kpanic(format_args!("clalloc: out of memory"));
            };
            Some(cq)
        } else {
            None
        };
        let mut g = self.lock();
        let old = core::mem::replace(
            &mut *g,
            Ring {
                cc: 0,
                cn: size,
                cf: 0,
                cl: 0,
                cs: Some(cs),
                cq,
            },
        );
        self.unlock_all(g);
        drop(scrub(old));
    }

    /// `clfree`: releases a clist's ring and bitmap, scrubbed first (`explicit_bzero`).
    pub fn clfree(&self) {
        let mut g = self.lock();
        // The C keeps c_cn and the count as they were; the blocks go.
        let old = Ring {
            cs: g.cs.take(),
            cq: g.cq.take(),
            ..Ring::default()
        };
        self.unlock(g);
        drop(scrub(old));
    }

    /// `getc`: get a character from a clist; -1 when it is empty.
    pub fn getc(&self) -> i32 {
        let mut g = self.lock();
        let c = g.getc();
        self.unlock(g);
        c
    }

    /// `q_to_b`: copy clist to buffer. Return number of bytes moved.
    pub fn q_to_b(&self, cp: &mut [u8]) -> usize {
        let mut g = self.lock();
        let r = &mut *g;
        let mut count = cp.len();
        let mut p = 0;
        // optimize this while loop
        while count > 0 && r.cc > 0 {
            let cc = r.run().min(count);
            let cf = r.cf;
            let src = &mut r.cs()[cf..cf + cc];
            cp[p..p + cc].copy_from_slice(src);
            src.fill(0);
            r.qclear(cf, cc);
            count -= cc;
            p += cc;
            r.cc -= cc as i32;
            r.cf = if cf + cc == r.cn as usize { 0 } else { cf + cc };
        }
        r.reset_if_empty();
        self.unlock(g);
        p
    }

    /// `ndqb`: return count of contiguous characters in clist. Stop counting if
    /// `flag & character` is non-null.
    pub fn ndqb(&self, flag: i32) -> i32 {
        let mut g = self.lock();
        let r = &mut *g;
        let count = 'out: {
            if r.cc == 0 {
                break 'out 0;
            }
            if flag == 0 {
                let count = r.cl as i32 - r.cf as i32;
                break 'out if count <= 0 {
                    (r.cn as usize - r.cf) as i32
                } else {
                    count
                };
            }
            let cn = r.cn as usize;
            let mut count = 0;
            let mut cc = r.cc;
            let mut i = r.cf;
            while cc > 0 {
                cc -= 1;
                let ch = i32::from(r.cs()[i]);
                i += 1;
                // With TTY_QUOTE the bit tested is the one after the character, as the C's
                // `i++` makes it.
                let stop = if flag & TTY_QUOTE != 0 {
                    ch & (flag & !TTY_QUOTE) != 0 || r.qisset(i)
                } else {
                    ch & flag != 0
                };
                if stop {
                    break;
                }
                count += 1;
                if i == cn {
                    break;
                }
            }
            count
        };
        drop(g);
        count
    }

    /// `ndflush`: flush `count` bytes from clist.
    pub fn ndflush(&self, count: i32) {
        let mut g = self.lock();
        g.ndflush(count);
        self.unlock(g);
    }

    /// `putc`: put a character into the output queue; -1 when it is full.
    pub fn putc(&self, c: i32) -> i32 {
        let mut g = self.lock();
        let r = g.putc(c);
        self.unlock(g);
        r
    }

    /// `b_to_q`: copy buffer to clist. Return number of bytes not transferred.
    pub fn b_to_q(&self, cp: &[u8]) -> usize {
        let mut count = cp.len();
        if count == 0 {
            return 0;
        }
        let mut g = self.lock();
        let r = &mut *g;
        let mut p = 0;
        'out: {
            if r.cc == r.cn {
                break 'out;
            }
            if r.cc == 0 {
                if r.cs.is_none() {
                    kpanic(format_args!("b_to_q: tty has no clist"));
                }
                r.cf = 0;
                r.cl = 0;
            }
            // optimize this while loop
            while count > 0 && r.cc < r.cn {
                let cn = r.cn as usize;
                let cl = r.cl;
                let room = if r.cf > cl { r.cf - cl } else { cn - cl };
                let cc = room.min(count);
                r.cs()[cl..cl + cc].copy_from_slice(&cp[p..p + cc]);
                r.qclear(cl, cc);
                p += cc;
                count -= cc;
                r.cc += cc as i32;
                r.cl = if cl + cc == cn { 0 } else { cl + cc };
            }
        }
        self.unlock(g);
        count
    }

    /// `nextc`: given a cursor into the clist return the cursor of the next character in the
    /// list, or `None` if no more chars; `*c` is set to that character.
    ///
    /// Callers must not allow `getc`s to happen between `firstc`s and `nextc`s so that the
    /// cursor becomes invalid. Note that interrupts are NOT masked.
    pub fn nextc(&self, cp: Option<usize>, c: &mut i32, ccp: &mut i32) -> Option<usize> {
        let mut g = self.lock();
        let r = &mut *g;
        if r.cc != 0 && cp == Some(r.cf) {
            // First time initialization.
            *ccp = r.cc;
        }
        let cp = cp?;
        if *ccp == 0 {
            return None;
        }
        *ccp -= 1;
        if *ccp == 0 {
            return None;
        }
        let cp = if cp + 1 == r.cn as usize { 0 } else { cp + 1 };
        *c = i32::from(r.cs()[cp]);
        if r.qisset(cp) {
            *c |= TTY_QUOTE;
        }
        Some(cp)
    }

    /// `firstc`: return the cursor of the first character in the list, or `None` if there is
    /// none; `*c` is set to that character.
    pub fn firstc(&self, c: &mut i32, ccp: &mut i32) -> Option<usize> {
        let mut g = self.lock();
        let r = &mut *g;
        *ccp = r.cc;
        if *ccp == 0 {
            return None;
        }
        let cp = r.cf;
        *c = i32::from(r.cs()[cp]);
        if r.qisset(cp) {
            *c |= TTY_QUOTE;
        }
        Some(cp)
    }

    /// `unputc`: remove the last character in the clist and return it; -1 when it is empty.
    pub fn unputc(&self) -> i32 {
        let mut g = self.lock();
        let r = &mut *g;
        let c = 'out: {
            if r.cc == 0 {
                break 'out -1;
            }
            let cl = if r.cl == 0 {
                r.cn as usize - 1
            } else {
                r.cl - 1
            };
            r.cl = cl;
            r.cc -= 1;
            let cs = r.cs();
            let mut c = i32::from(cs[cl]);
            cs[cl] = 0;
            if r.qisset(cl) {
                c |= TTY_QUOTE;
            }
            r.qput(cl, false);
            r.reset_if_empty();
            c
        };
        self.unlock(g);
        c
    }

    /// Both queues' rings locked, the lower address first (a fixed order, so two CPUs never
    /// wait on each other); the guards in `(self, other)` order.
    fn lock_pair<'a>(
        &'a self,
        other: &'a Self,
    ) -> (MutexedGuard<'a, Ring>, MutexedGuard<'a, Ring>) {
        if core::ptr::from_ref(self).addr() < core::ptr::from_ref(other).addr() {
            let a = self.lock();
            let b = other.lock();
            (a, b)
        } else {
            let b = other.lock();
            let a = self.lock();
            (a, b)
        }
    }

    /// `catq(from = self, to)`: put the chars in the `from` queue on the end of the `to`
    /// queue. If `to` is empty and the queues are the same max size, it is more efficient to
    /// just swap the clist structures.
    pub fn catq(&self, to: &Self) {
        if core::ptr::eq(self, to) {
            return;
        }
        let (mut f, mut t) = self.lock_pair(to);
        if f.cc != 0 {
            if t.cc == 0 && f.cn == t.cn {
                core::mem::swap(&mut *f, &mut *t);
            } else {
                loop {
                    let c = f.getc();
                    if c == -1 {
                        break;
                    }
                    let _ = t.putc(c);
                }
            }
        }
        self.unlock_all(f);
        to.unlock_all(t);
    }

    /// `tmp = *self; *self = *other; *other = tmp` (`TIOCSETA`'s exchange of the raw and
    /// canonical queues): the rings, their cursors and their counts change places.
    pub fn swap(&self, other: &Self) {
        if core::ptr::eq(self, other) {
            return;
        }
        let (mut a, mut b) = self.lock_pair(other);
        core::mem::swap(&mut *a, &mut *b);
        self.unlock_all(a);
        other.unlock_all(b);
    }

    /// `f` of the contiguous characters at the head of the queue (`ndqb(q, 0)` of them from
    /// `c_cf`: up to `c_cl`, or to the ring's end when the queue wraps), at most `n`, lent
    /// under the queue's lock; empty when the queue is.
    pub fn peek_with<R>(&self, n: usize, f: impl FnOnce(&[u8]) -> R) -> R {
        let mut g = self.lock();
        let len = if g.cc == 0 { 0 } else { g.run().min(n) };
        let cf = g.cf;
        f(&g.cs()[cf..cf + len])
    }
}

impl Default for Clist {
    fn default() -> Self {
        Self::new()
    }
}

/// `explicit_bzero` of a ring's blocks before they are freed (by the caller's drop).
fn scrub(mut r: Ring) -> Ring {
    if let Some(cs) = &mut r.cs {
        libkern::explicit_bzero(cs);
    }
    if let Some(cq) = &mut r.cq {
        libkern::explicit_bzero(cq);
    }
    r
}

/// `clrbits`: optimized version of `for (i = 0; i < len; i++) clrbit(cp, off + i);`.
pub fn clrbits(cp: &mut [u8], off: usize, len: usize) {
    if len == 1 {
        cp[off / NBBY] &= !(1 << (off % NBBY));
        return;
    }

    let mut sby = off / NBBY;
    let sbi = off % NBBY;
    let eby = (off + len) / NBBY;
    let ebi = (off + len) % NBBY;
    if sby == eby {
        let mask = (((1u32 << (ebi - sbi)) - 1) << sbi) as u8;
        cp[sby] &= !mask;
    } else {
        let mask = ((1u32 << sbi) - 1) as u8;
        cp[sby] &= mask;
        sby += 1;

        cp[sby..eby].fill(0x00);

        let mask = ((1u32 << ebi) - 1) as u8;
        if mask != 0 {
            // if no mask, eby may be 1 too far
            cp[eby] &= !mask;
        }
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The clists over the doubles' malloc: the ring wrapping, the quote bits, the cursors,
    // the queue exchanges, the unlocked counts, and the drivers' head-of-queue view.

    use std::boxed::Box;
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    /// A queue of `size` characters, set up by `clalloc` (leaked for the run).
    fn q(size: i32, quot: bool) -> &'static Clist {
        testutil::install();
        let q: &'static Clist = Box::leak(Box::new(Clist::new()));
        q.clalloc(size, quot);
        q
    }

    fn drain(q: &Clist) -> Vec<i32> {
        let mut out = Vec::new();
        loop {
            let c = q.getc();
            if c == -1 {
                return out;
            }
            out.push(c);
        }
    }

    /// The unlocked words follow the ring after every operation.
    fn words(q: &Clist) -> (i32, i32) {
        (q.c_cc.get(), q.c_cn.get())
    }

    #[test]
    fn putc_getc_wrap_around_and_keep_quotes() {
        let q = q(4, true);
        assert_eq!(q.getc(), -1);
        assert_eq!(q.putc(i32::from(b'a')), 0);
        assert_eq!(q.putc(i32::from(b'b') | TTY_QUOTE), 0);
        assert_eq!(q.getc(), i32::from(b'a'));
        // Wrap: three more fit (one slot was freed).
        for c in [b'c', b'd', b'e'] {
            assert_eq!(q.putc(i32::from(c)), 0);
        }
        assert_eq!(q.putc(i32::from(b'f')), -1, "full");
        assert_eq!(words(q), (4, 4));
        assert_eq!(
            drain(q),
            [
                i32::from(b'b') | TTY_QUOTE,
                i32::from(b'c'),
                i32::from(b'd'),
                i32::from(b'e')
            ]
        );
        assert_eq!(words(q), (0, 4));
    }

    #[test]
    fn queues_without_quoting_drop_the_bit() {
        let q = q(8, false);
        q.putc(i32::from(b'x') | TTY_QUOTE);
        assert_eq!(q.getc(), i32::from(b'x'));
    }

    #[test]
    fn b_to_q_and_q_to_b_move_what_fits() {
        let q = q(8, true);
        assert_eq!(q.b_to_q(b""), 0);
        assert_eq!(q.b_to_q(b"hello"), 0);
        let mut buf = [0u8; 3];
        assert_eq!(q.q_to_b(&mut buf), 3);
        assert_eq!(&buf, b"hel");
        // 2 left; 6 free, wrapping at the end of the ring.
        assert_eq!(q.b_to_q(b"0123456789"), 4, "four did not fit");
        assert_eq!(words(q), (8, 8));
        let mut all = [0u8; 16];
        assert_eq!(q.q_to_b(&mut all), 8);
        assert_eq!(&all[..8], b"lo012345");
        assert_eq!(q.q_to_b(&mut all), 0);
        // A quote bit written by putc is cleared by b_to_q over the same slot.
        q.putc(i32::from(b'q') | TTY_QUOTE);
        assert_eq!(q.getc(), i32::from(b'q') | TTY_QUOTE);
        q.b_to_q(b"r");
        assert_eq!(q.getc(), i32::from(b'r'), "the slot's quote bit is gone");
    }

    #[test]
    fn ndqb_counts_contiguous_characters() {
        let q = q(8, true);
        assert_eq!(q.ndqb(0), 0);
        q.b_to_q(b"abcdef");
        let mut buf = [0u8; 4];
        q.q_to_b(&mut buf); // c_cf = 4, "ef" left
        q.b_to_q(b"ghij"); // wraps: "gh" at the end, "ij" at the start
        assert_eq!(q.c_cc.get(), 6);
        assert_eq!(q.ndqb(0), 4, "up to the end of the ring");
        // Stop at a character with the bit set ('g' = 0x67 has 0x40).
        assert_eq!(q.ndqb(0x40), 0, "'e' has 0x40 too");
        q.ndflush(2);
        assert_eq!(q.ndqb(0x80), 2, "stops at the end of the ring");
        assert_eq!(drain(q), [0x67, 0x68, 0x69, 0x6a]);
    }

    #[test]
    fn ndqb_with_quote_tests_the_next_bit() {
        let q = q(16, true);
        q.putc(i32::from(b'a'));
        q.putc(i32::from(b'b') | TTY_QUOTE);
        q.putc(i32::from(b'c'));
        // The C tests the quote bit after the character it counts: 'a' is counted only if
        // 'b' is not quoted.
        assert_eq!(q.ndqb(TTY_QUOTE), 0);
        q.getc();
        q.getc();
        assert_eq!(q.ndqb(TTY_QUOTE), 1, "'c', the bit after it clear");
    }

    #[test]
    fn unputc_and_ndflush() {
        let q = q(4, true);
        assert_eq!(q.unputc(), -1);
        q.b_to_q(b"abc");
        q.putc(i32::from(b'd') | TTY_QUOTE);
        assert_eq!(q.unputc(), i32::from(b'd') | TTY_QUOTE);
        assert_eq!(q.unputc(), i32::from(b'c'));
        q.getc();
        q.getc();
        // Empty again; the next putc starts over at the ring's start.
        q.b_to_q(b"xyz");
        q.ndflush(1);
        assert_eq!(drain(q), [i32::from(b'y'), i32::from(b'z')]);
        q.b_to_q(b"12");
        q.ndflush(2);
        assert_eq!(words(q), (0, 4));
        // Flushing more than the queue holds stops at its end.
        q.b_to_q(b"3");
        q.ndflush(7);
        assert_eq!(q.c_cc.get(), 0);
    }

    #[test]
    fn firstc_nextc_walk_the_queue() {
        let q = q(4, true);
        let (mut c, mut cc) = (0, 0);
        assert_eq!(q.firstc(&mut c, &mut cc), None);
        q.b_to_q(b"ab");
        q.getc();
        q.getc();
        q.b_to_q(b"cd");
        q.putc(i32::from(b'e') | TTY_QUOTE); // wraps to slot 0
        let mut seen = Vec::new();
        let mut cp = q.firstc(&mut c, &mut cc);
        while cp.is_some() {
            seen.push(c);
            cp = q.nextc(cp, &mut c, &mut cc);
        }
        assert_eq!(
            seen,
            [
                i32::from(b'c'),
                i32::from(b'd'),
                i32::from(b'e') | TTY_QUOTE
            ]
        );
        assert_eq!(q.c_cc.get(), 3, "walking does not consume");
    }

    #[test]
    fn catq_swaps_or_copies_and_swap_exchanges_rings() {
        let from = q(8, true);
        let to = q(8, true);
        from.b_to_q(b"abc");
        from.catq(to);
        assert_eq!(words(from), (0, 8), "same size, empty target: swapped");
        assert_eq!(words(to), (3, 8));
        let small = q(4, false);
        small.putc(i32::from(b'z'));
        to.catq(small);
        assert_eq!(drain(small), [i32::from(b'z'), 0x61, 0x62, 0x63]);
        assert_eq!(words(to), (0, 8));
        // A queue onto itself moves nothing.
        to.b_to_q(b"s");
        to.catq(to);
        assert_eq!(words(to), (1, 8));

        // swap: the rings and their counts change places, quote bits with them.
        let a = q(8, true);
        let b = q(16, true);
        a.putc(i32::from(b'x') | TTY_QUOTE);
        b.b_to_q(b"yz");
        a.swap(b);
        assert_eq!((words(a), words(b)), ((2, 16), (1, 8)));
        assert_eq!(drain(a), [0x79, 0x7a]);
        assert_eq!(drain(b), [0x78 | TTY_QUOTE]);
    }

    #[test]
    fn opposite_pairs_from_two_threads_do_not_deadlock() {
        let a = q(64, false);
        let b = q(64, false);
        std::thread::scope(|s| {
            s.spawn(|| {
                for _ in 0..2000 {
                    a.b_to_q(b"ab");
                    a.catq(b);
                    b.swap(a);
                }
            });
            s.spawn(|| {
                for _ in 0..2000 {
                    b.ndflush(b.c_cc.get());
                    b.catq(a);
                    a.swap(b);
                    a.ndflush(a.c_cc.get());
                }
            });
        });
    }

    #[test]
    fn clalloc_and_clfree() {
        // An unallocated queue holds nothing and refuses nothing it cannot hold.
        testutil::install();
        let q: &'static Clist = Box::leak(Box::new(Clist::new()));
        assert_eq!(q.getc(), -1);
        assert_eq!(q.ndqb(0), 0);
        assert_eq!(words(q), (0, 0));
        q.clalloc(16, true);
        assert_eq!(words(q), (0, 16));
        q.b_to_q(b"secret");
        // clfree scrubs and frees the blocks; the count stays as the C leaves it.
        q.clfree();
        assert_eq!(q.c_cn.get(), 16);
        // An allocation again starts empty.
        q.clalloc(8, false);
        assert_eq!(words(q), (0, 8));
        assert_eq!(q.getc(), -1);
        // A free of a queue never allocated does nothing.
        let r: &'static Clist = Box::leak(Box::new(Clist::new_at(hal::intr::IPL_NONE)));
        r.clfree();
        assert_eq!(words(r), (0, 0));
    }

    #[test]
    fn peek_with_lends_the_contiguous_head() {
        let q = q(8, false);
        assert_eq!(q.peek_with(8, <[u8]>::to_vec), b"");
        q.b_to_q(b"abcdef");
        assert_eq!(q.peek_with(4, <[u8]>::to_vec), b"abcd");
        let mut buf = [0u8; 5];
        q.q_to_b(&mut buf); // c_cf = 5, "f" left
        q.b_to_q(b"ghij"); // "fgh" at the end, "ij" at the start
        assert_eq!(q.peek_with(8, <[u8]>::to_vec), b"fgh", "to the ring's end");
        assert_eq!(q.peek_with(8, <[u8]>::len), q.ndqb(0) as usize);
        q.ndflush(3);
        assert_eq!(q.peek_with(8, <[u8]>::to_vec), b"ij");
    }

    #[test]
    fn the_second_channel_is_not_the_queue() {
        let q = q(4, false);
        assert_ne!(q.cf_chan(), core::ptr::from_ref(q).cast());
    }

    #[test]
    fn clrbits_clears_exactly_the_range() {
        let mut map = [0xffu8; 4];
        clrbits(&mut map, 3, 1);
        assert_eq!(map[0], 0xf7);
        clrbits(&mut map, 4, 2);
        assert_eq!(map[0], 0xc7);
        clrbits(&mut map, 6, 12);
        assert_eq!(map, [0x07, 0x00, 0xfc, 0xff]);
        // Ending on a byte boundary must not touch the byte after.
        let mut map = [0xffu8; 2];
        clrbits(&mut map, 4, 12);
        assert_eq!(map, [0x0f, 0x00]);
    }

    #[test]
    fn qmem_rounds_up_to_bytes() {
        assert_eq!([qmem(1), qmem(8), qmem(9), qmem(1024)], [1, 1, 2, 128]);
    }
}
/* </TESTS> */
