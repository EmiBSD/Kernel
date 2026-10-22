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
 * Copyright (c) 1990, 1993
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
 *	@(#)filedesc.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/filedesc.h>` for `bsd`: the kernel's descriptor table (`kernel::descrip`) bound to
//! `bsd`'s glue.
//!
//! Upstream: sys/sys/filedesc.h @ 3ce1f3f79392
//! LZ: sys/sys/filedesc.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.3: `struct filedesc` and `struct filedesc0` moved into `kernel::descrip` (a pool item
//!   whose slots and bitmaps sit inside the locks that guard them; `filedesc0`'s internal
//!   arrays are its first storage); this module names its one instance (`Filedesc`,
//!   `FdRef`, `FdGuard` over `BsdFileGlue`) and re-exports the constants and the lock calls.
//!   `p->p_fd` and `pr->ps_fd` are the kernel's `Thread::fd` and `Process::fd`.

pub use kernel::descrip::{
    FD_ADVLOCK, NDENTRIES, NDENTRYMASK, NDENTRYSHIFT, NDEXTENT, NDFILE, UF_EXCLOSE, UF_FORKCLOSE,
    UF_PLEDGED, UF_PLEDGEOPEN, fdpassertlocked, fdplock, fdpunlock, ndhislots, ndloslots, ndreduce,
};

use crate::kern::kern_descrip::BsdFileGlue;

/// `struct filedesc` of `bsd`'s processes.
pub type Filedesc = kernel::descrip::Filedesc<BsdFileGlue>;

/// A counted reference to a `struct filedesc` (`ps_fd`).
pub type FdRef = kernel::descrip::FdRef<BsdFileGlue>;

/// `fdplock(fdp)` held.
pub type FdGuard<'a> = kernel::descrip::FdGuard<'a, BsdFileGlue>;

/// `OFILESIZE`: storage required per open file descriptor.
pub const OFILESIZE: usize = size_of::<*const ()>() + size_of::<u8>();

// N4.3's probe (BRIEF-N4.3 section 9, commit 3), kept: what the kernel's file and table hold
// of `bsd`'s is `Send + Sync`, as their pools need.
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<crate::kern::kern_descrip::BsdFileData>();
    send_sync::<kernel::cred::CredRef>();
    send_sync::<crate::kern::kern_descrip::BsdFdKq>();
};
/* </CODE> */
