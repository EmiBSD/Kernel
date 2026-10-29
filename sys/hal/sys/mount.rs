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
 * Copyright (c) 1989, 1991, 1993
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
 *	@(#)mount.h	8.15 (Berkeley) 7/14/94
 */
/* </LICENSES> */

/* <CODE> */
//! The part of `<sys/mount.h>` the kernel and `bsd` share: the buffer cache statistics
//! (`struct bcachestats`), copied out whole by `vfs.generic.bcachestat`.
//!
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//! LZ: sys/sys/mount.rs@3c62ede68802
//!
//! ## Redesign
//! - N5.1: `struct bcachestats` is `hal`'s, so that `kernel::fs::bio` (the counters' owner,
//!   `bcstats`) and `bsd` (`vfs.generic.bcachestat`'s copy-out, `vfs_subr.rs`) name one copy;
//!   `bsd`'s `sys/sys/mount.rs` re-exports it. It is `AbiPod` through [`crate::abi_pod!`]
//!   (LZ declared it there, in `vfs_bio.rs`). The rest of the header moves with the VFS
//!   (N5.2, N5.4).

/// `struct bcachestats`: buffer cache statistics.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bcachestats {
    /// `numbufs`: number of buffers allocated.
    pub numbufs: i64,
    /// `numbufpages`: number of pages in buffer cache.
    pub numbufpages: i64,
    /// `numdirtypages`: number of dirty free pages.
    pub numdirtypages: i64,
    /// `numcleanpages`: number of clean free pages.
    pub numcleanpages: i64,
    /// `pendingwrites`: number of pending writes.
    pub pendingwrites: i64,
    /// `pendingreads`: number of pending reads.
    pub pendingreads: i64,
    /// `numwrites`: total writes started.
    pub numwrites: i64,
    /// `numreads`: total reads started.
    pub numreads: i64,
    /// `cachehits`: total reads found in cache.
    pub cachehits: i64,
    /// `busymapped`: number of busy and mapped buffers.
    pub busymapped: i64,
    /// `delwribufs`: delayed write buffers.
    pub delwribufs: i64,
    /// `kvaslots`: kva slots total.
    pub kvaslots: i64,
    /// `kvaslots_avail`: available kva slots.
    pub kvaslots_avail: i64,
}

// `#[repr(C)]`, thirteen `i64`s, no padding (`vfs.generic.bcachestat` copies it out).
crate::abi_pod!(Bcachestats {
    numbufs: i64,
    numbufpages: i64,
    numdirtypages: i64,
    numcleanpages: i64,
    pendingwrites: i64,
    pendingreads: i64,
    numwrites: i64,
    numreads: i64,
    cachehits: i64,
    busymapped: i64,
    delwribufs: i64,
    kvaslots: i64,
    kvaslots_avail: i64,
});

const _: () = assert!(size_of::<Bcachestats>() == 13 * 8);
/* </CODE> */
