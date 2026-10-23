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
/* </LICENSES> */

/* <CODE> */
//! The character lists of the terminals, `kern/tty_subr.c`, for `bsd`: the kernel's
//! (`kernel::clist`, N4.7b) under the free functions' names the drivers call
//! (`getc(&tp->t_outq)`, `q_to_b(&sc->sc_q, buf)`, ...).
//!
//! Upstream: sys/kern/tty_subr.c @ 3ce1f3f79392
//! LZ: sys/kern/tty_subr.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.7b: a queue owns its ring inside its lock (`kernel::clist::Clist`, whose documentation
//!   has the shape); each function here is one call of the queue's method. LZ's four `unsafe`
//!   (the ring and the bitmap as raw slices, `clfree`'s two volatile scrubs) are gone with
//!   the raw pointers.

use crate::sys::tty::Clist;

pub use kernel::clist::qmem;

/// `clalloc`: initialize a particular clist (a ring of `size` characters, with or without
/// quoting).
pub fn clalloc(clp: &Clist, size: i32, quot: bool) {
    clp.clalloc(size, quot);
}

/// `clfree`: releases a clist's ring and bitmap, scrubbed first.
pub fn clfree(clp: &Clist) {
    clp.clfree();
}

/// `getc`: get a character from a clist; -1 when it is empty.
pub fn getc(clp: &Clist) -> i32 {
    clp.getc()
}

/// `q_to_b`: copy clist to buffer. Return number of bytes moved.
pub fn q_to_b(clp: &Clist, cp: &mut [u8]) -> usize {
    clp.q_to_b(cp)
}

/// `ndqb`: return count of contiguous characters in clist. Stop counting if
/// `flag & character` is non-null.
pub fn ndqb(clp: &Clist, flag: i32) -> i32 {
    clp.ndqb(flag)
}

/// `ndflush`: flush `count` bytes from clist.
pub fn ndflush(clp: &Clist, count: i32) {
    clp.ndflush(count);
}

/// `putc`: put a character into the output queue; -1 when it is full.
pub fn putc(c: i32, clp: &Clist) -> i32 {
    clp.putc(c)
}

/// `clrbits`: optimized version of `for (i = 0; i < len; i++) clrbit(cp, off + i);`.
pub fn clrbits(cp: &mut [u8], off: usize, len: usize) {
    kernel::clist::clrbits(cp, off, len);
}

/// `b_to_q`: copy buffer to clist. Return number of bytes not transferred.
pub fn b_to_q(cp: &[u8], clp: &Clist) -> usize {
    clp.b_to_q(cp)
}

/// `nextc`: given a cursor into the clist return the cursor of the next character in the
/// list, or `None` if no more chars; `*c` is set to that character.
pub fn nextc(clp: &Clist, cp: Option<usize>, c: &mut i32, ccp: &mut i32) -> Option<usize> {
    clp.nextc(cp, c, ccp)
}

/// `firstc`: return the cursor of the first character in the list, or `None` if there is
/// none; `*c` is set to that character.
pub fn firstc(clp: &Clist, c: &mut i32, ccp: &mut i32) -> Option<usize> {
    clp.firstc(c, ccp)
}

/// `unputc`: remove the last character in the clist and return it; -1 when it is empty.
pub fn unputc(clp: &Clist) -> i32 {
    clp.unputc()
}

/// `catq`: put the chars in the `from` queue on the end of the `to` queue.
pub fn catq(from: &Clist, to: &Clist) {
    from.catq(to);
}
/* </CODE> */
