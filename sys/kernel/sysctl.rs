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
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Mike Karels at Berkeley Software Design, Inc.
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
 *	@(#)kern_sysctl.c	8.4 (Berkeley) 4/14/94
 */
/* </LICENSES> */

/* <CODE> */
//! The sysctl tree's shape (`kern/kern_sysctl.c`): the [`SysctlNode`] a subsystem's
//! `*_sysctl` function is (the C's hand-written dispatch by `name[0]`), the request it
//! answers ([`SysctlReq`]) and the helpers of the read-only leaves `kern.tty` needs
//! (`sysctl_rdquad`, `sysctl_rdstruct`).
//!
//! Upstream: sys/kern/kern_sysctl.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_sysctl.rs@3c62ede68802
//!
//! ## Redesign
//! - Decision 46 (BRIEF-N4.7-AMEND-46, decision C): each `*_sysctl` the C dispatches by hand
//!   is a `SysctlNode` (`sysctl_tty` is `kernel::tty::TtyNode`), whose `sysctl` takes the
//!   name below the node and the request. The C's `oldp`/`oldlenp`/`newp`/`newlen` are a
//!   [`SysctlReq`] (moved here from `kernel::vm::meter`, which re-exports it): `*oldlenp` is
//!   in and out (a size query passes `oldp` 0 and gets the length back). No node of `kern`
//!   takes the process.
//! - N4.7b lands this module before the sysctl core's slice (N4.6b), which inherits it: the
//!   dispatch and the other helpers stay `bsd`'s (`kern_sysctl.rs`) until then.
//! - The copy-out is `hal::user::UserPtr`'s (the `UserCopy` bridge), not `bsd`'s `copyout`.

use hal::sys::errno::Errno;
use hal::user::UserPtr;

/// The user side of a sysctl request (`oldp`, `oldlenp`, `newp`, `newlen`).
pub struct SysctlReq<'a> {
    /// `oldp`: where the old value goes (0: nowhere).
    pub oldp: usize,
    /// `*oldlenp`: its room, then the value's length.
    pub oldlenp: &'a mut usize,
    /// `newp`: the new value (0: none).
    pub newp: usize,
    /// `newlen`.
    pub newlen: usize,
}

/// A node of the sysctl tree that answers for the names below it (a `*_sysctl` function of
/// the C: `sysctl_tty`, `sysctl_pty`, ...).
pub trait SysctlNode: Sync {
    /// The request `req` for the name `name` (the part below this node).
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno>;
}

/// `sysctl_rdquad(oldp, oldlenp, newp, val)`: a read-only 64-bit value.
pub fn rdquad(req: &mut SysctlReq<'_>, val: i64) -> Result<(), Errno> {
    rdstruct(req, &val.to_ne_bytes())
}

/// `sysctl_rdstruct(oldp, oldlenp, newp, sp, len)`: a read-only structure's bytes: `ENOMEM`
/// when the room is short, `EPERM` for a new value, the length always given back.
pub fn rdstruct(req: &mut SysctlReq<'_>, sp: &[u8]) -> Result<(), Errno> {
    let len = sp.len();
    if req.oldp != 0 && *req.oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    *req.oldlenp = len;
    if req.oldp != 0 {
        UserPtr::<u8>::new(req.oldp).copyout_slice(sp)?;
    }
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    /// A user buffer of the double's memory (its own range).
    const UBUF: usize = testutil::USER_BASE + 0x9000;

    #[test]
    fn rdquad_and_rdstruct_answer_as_the_c() {
        testutil::install();
        // A size query: no buffer, the length comes back.
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(rdquad(&mut req, 7), Ok(()));
        assert_eq!(len, 8);
        // Too little room.
        let mut len = 4;
        let mut req = SysctlReq {
            oldp: UBUF,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(rdquad(&mut req, 7), Err(Errno::ENOMEM));
        // A new value for a read-only leaf.
        let mut len = 8;
        let mut req = SysctlReq {
            oldp: UBUF,
            oldlenp: &mut len,
            newp: UBUF,
            newlen: 8,
        };
        assert_eq!(rdquad(&mut req, 7), Err(Errno::EPERM));
        // The value copied out.
        let mut len = 16;
        let mut req = SysctlReq {
            oldp: UBUF,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(rdstruct(&mut req, b"abc"), Ok(()));
        assert_eq!(len, 3);
        assert_eq!(testutil::user_bytes(UBUF, 3).unwrap(), b"abc");
    }
}
/* </TESTS> */
