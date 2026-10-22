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
 * Copyright (c) 1989, 1993
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
 *	@(#)ucred.h	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/ucred.h>`: credentials.
//!
//! Upstream: sys/sys/ucred.h @ 3ce1f3f79392
//! LZ: sys/sys/ucred.rs@3c62ede68802
//!
//! `struct ucred` is the kernel crate's (`kernel::cred::Ucred`, whose members are atomic
//! cells read with `get()`; a placed credential is shared through the counted `CredRef`,
//! whose `Clone` is `crhold` and whose `Drop` is `crfree`); this header keeps `NOCRED`,
//! `FSCRED`, `struct xucred`, and [`CredSlot`], a holder's reference. `kern/kern_prot.rs`
//! holds the functions it declares.
//!
//! ## Deviations
//! - `NOCRED` and `FSCRED` stay the C's pointer sentinels: the VFS still passes credentials
//!   as `*const Ucred` (a thread's own copy, `Proc::ucred_ptr`, or a holder's `CredRef`)
//!   until N5.
//! - `cr_startcopy` (the `#define` naming where `crset` starts copying) is a comment: `crset`
//!   copies the members by name.
//!
//! ## Redesign
//! - LZ's `struct ucred` (here, `Cell` members after `cr_refcnt`, filled in through the
//!   shared pointer the processes held) moved to `kernel::cred` (N4.2), re-exported.
//! - The holders of a `struct ucred *` that `crhold` and `crfree` it by hand (a file's
//!   `f_cred`, a sigio's `sio_ucred`, an nfs node's, a quota's) hold a [`CredSlot`]: the
//!   counted reference in a mutex-guarded cell, given back by dropping it, never by a raw
//!   count; `get()` keeps the address the legacy VFS passes as `*const Ucred`.

use core::ptr;

use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;

use crate::sys::syslimits::NGROUPS_MAX;
use crate::sys::types::{Gid, Uid};

pub use kernel::cred::{CountCell, CredRef, IdCell, Ucred, UcredCell};

/// `NOCRED`: no credential available.
pub const NOCRED: *const Ucred = ptr::without_provenance(usize::MAX);
/// `FSCRED`: filesystem credential.
pub const FSCRED: *const Ucred = ptr::without_provenance(usize::MAX - 1);

/// A holder's counted reference to credentials: a `struct ucred *` the holder `crhold`s and
/// `crfree`s (a file's `f_cred`, a sigio's `sio_ucred`, an nfs node's, a quota's), kept in
/// a cell of its own so the reference is given back by dropping it, never by a raw count.
/// `get()` keeps LZ's reader, the address the legacy VFS passes (null when empty).
pub struct CredSlot(Mutexed<Option<CredRef>>);

impl CredSlot {
    /// No credentials (`NULL`).
    pub const fn new() -> Self {
        Self(Mutexed::new(IPL_NONE, None))
    }

    /// The credentials' address, null when there are none: valid while the slot holds
    /// them.
    pub fn get(&self) -> *const Ucred {
        self.0.lock().as_ref().map_or(ptr::null(), CredRef::as_ptr)
    }

    /// One more reference to the credentials (`crhold(holder->cred)`), `None` when there
    /// are none.
    pub fn cred(&self) -> Option<CredRef> {
        self.0.lock().clone()
    }

    /// `holder->cred = cr`: the slot holds `cr`; the reference it held before is dropped
    /// (`crfree`) after the slot's lock.
    pub fn set(&self, cr: CredRef) {
        let old = self.0.lock().replace(cr);
        drop(old);
    }

    /// The reference the slot held, the slot left empty (the holder's `crfree`).
    pub fn take(&self) -> Option<CredRef> {
        self.0.lock().take()
    }
}

impl Default for CredSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct xucred`: userspace version, for use in syscalls arguments.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Xucred {
    /// `cr_uid`: user id.
    pub cr_uid: Uid,
    /// `cr_gid`: group id.
    pub cr_gid: Gid,
    /// `cr_ngroups`: number of groups.
    pub cr_ngroups: i16,
    /// `cr_groups`: groups.
    pub cr_groups: [Gid; NGROUPS_MAX],
}

// A `CredRef` is one non-null pointer: `Option<CredRef>` takes its null niche, so an
// all-zero `CredSlot` (a zeroed softc's) holds `None`.
const _: () = {
    assert!(size_of::<Option<CredRef>>() == size_of::<usize>());
};
/* </CODE> */
