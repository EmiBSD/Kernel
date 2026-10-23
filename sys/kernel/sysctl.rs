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
//! answers ([`SysctlReq`]) and the copy helpers of `kern_sysctl.c` the kernel's nodes need
//! (`sysctl_rdint`, `sysctl_int_bounded`, `sysctl_rdquad`, `sysctl_rdstruct`,
//! `sysctl_string`, `sysctl_rdstring`), with the C's checks in the same order and the same
//! errnos.
//!
//! Upstream: sys/kern/kern_sysctl.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_sysctl.rs@3c62ede68802
//!
//! ## Redesign
//! - Decision 46 (BRIEF-N4.7-AMEND-46, decision C; the N4.8 amendment): each `*_sysctl` the
//!   C dispatches by hand is a `SysctlNode` (`sysctl_tty` is `kernel::tty::TtyNode`,
//!   `sysctl_tc` `kernel::tc::TcNode`, `sysctl_clockintr` `kernel::clockintr::ClockintrNode`),
//!   whose `sysctl` takes the name below the node and the request. The C's
//!   `oldp`/`oldlenp`/`newp`/`newlen` are a [`SysctlReq`] (moved here from
//!   `kernel::vm::meter`, which re-exports it): `*oldlenp` is in and out (a size query passes
//!   `oldp` 0 and gets the length back). No node of `kern` takes the process.
//! - N4.7b and N4.8a land this module before the sysctl core's slice (N4.6b), which inherits
//!   it: the dispatch and the other helpers stay `bsd`'s (`kern_sysctl.rs`) until then.
//! - The copies are `hal::user::UserPtr`'s (the `UserCopy` bridge), not `bsd`'s
//!   `copyin`/`copyout`; a string variable is its whole buffer (`&mut [u8]`, the C's
//!   `maxlen`), its length up to the first NUL.

use core::sync::atomic::{AtomicI32, Ordering};

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
/// the C: `sysctl_tty`, `sysctl_tc`, `sysctl_clockintr`, ...).
pub trait SysctlNode: Sync {
    /// The request `req` for the name `name` (the part below this node).
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno>;
}

/// `copyout(src, oldp, len)` of bytes.
fn copyout_bytes(b: &[u8], uaddr: usize) -> Result<(), Errno> {
    UserPtr::<u8>::new(uaddr).copyout_slice(b)
}

/// `sysctl_rdint(oldp, oldlenp, newp, val)`: a read-only `int`.
pub fn rdint(req: &mut SysctlReq<'_>, val: i32) -> Result<(), Errno> {
    rdstruct(req, &val.to_ne_bytes())
}

/// `sysctl_int_bounded`: an `int` read and maybe set within `minimum..=maximum` (read-only
/// when `minimum > maximum`).
pub fn int_bounded(
    req: &mut SysctlReq<'_>,
    valp: &AtomicI32,
    minimum: i32,
    maximum: i32,
) -> Result<(), Errno> {
    // read only
    if req.newp != 0 && minimum > maximum {
        return Err(Errno::EPERM);
    }

    if req.oldp != 0 && *req.oldlenp < size_of::<i32>() {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen != size_of::<i32>() {
        return Err(Errno::EINVAL);
    }
    *req.oldlenp = size_of::<i32>();

    // copyin() may sleep, call it first
    let mut newval = 0;
    if req.newp != 0 {
        newval = UserPtr::<i32>::new(req.newp).copyin()?;
        // outside limits
        if newval < minimum || maximum < newval {
            return Err(Errno::EINVAL);
        }
    }
    if req.oldp != 0 {
        let oldval = if req.newp != 0 {
            valp.swap(newval, Ordering::Relaxed)
        } else {
            valp.load(Ordering::Relaxed)
        };
        // new value has been set although user gets error
        copyout_bytes(&oldval.to_ne_bytes(), req.oldp)?;
    } else if req.newp != 0 {
        valp.store(newval, Ordering::Relaxed);
    }

    Ok(())
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
        copyout_bytes(sp, req.oldp)?;
    }
    Ok(())
}

/// The length of the string in `s`: up to its first NUL or its end (`strnlen`).
fn strnlen(s: &[u8]) -> usize {
    s.iter().position(|&b| b == 0).unwrap_or(s.len())
}

/// `copyout(str, oldp, strlen(str) + 1)`: the bytes, then the NUL.
fn copyout_cstr(s: &[u8], uaddr: usize) -> Result<(), Errno> {
    copyout_bytes(s, uaddr)?;
    copyout_bytes(&[0], uaddr + s.len())
}

/// `sysctl_string`: a string read and maybe set; `str` is the variable's whole buffer (the
/// C's `maxlen`), a new value NUL-terminated in it.
pub fn string(req: &mut SysctlReq<'_>, str: &mut [u8]) -> Result<(), Errno> {
    let maxlen = str.len();
    let slen = strnlen(str);
    let len = slen + 1;
    if req.oldp != 0 && *req.oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 && req.newlen >= maxlen {
        return Err(Errno::EINVAL);
    }
    let mut error = Ok(());
    if req.oldp != 0 {
        error = copyout_cstr(&str[..slen], req.oldp);
    }
    *req.oldlenp = len;
    if error.is_ok() && req.newp != 0 {
        error = UserPtr::<u8>::new(req.newp).copyin_slice(&mut str[..req.newlen]);
        str[req.newlen] = 0;
    }
    error
}

/// `sysctl_rdstring`: a read-only string, `str` up to its first NUL or its end.
pub fn rdstring(req: &mut SysctlReq<'_>, str: &[u8]) -> Result<(), Errno> {
    let slen = strnlen(str);
    let len = slen + 1;
    if req.oldp != 0 && *req.oldlenp < len {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    *req.oldlenp = len;
    if req.oldp != 0 {
        copyout_cstr(&str[..slen], req.oldp)?;
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
    /// A second user range of this module's own in the user-copy double.
    const U: usize = testutil::USER_BASE + 0xC800;

    fn req(oldp: usize, oldlen: &mut usize, newp: usize, newlen: usize) -> SysctlReq<'_> {
        SysctlReq {
            oldp,
            oldlenp: oldlen,
            newp,
            newlen,
        }
    }

    #[test]
    fn rdquad_and_rdstruct_answer_as_the_c() {
        testutil::install();
        // A size query: no buffer, the length comes back.
        let mut len = 0;
        assert_eq!(rdquad(&mut req(0, &mut len, 0, 0), 7), Ok(()));
        assert_eq!(len, 8);
        // Too little room.
        let mut len = 4;
        assert_eq!(
            rdquad(&mut req(UBUF, &mut len, 0, 0), 7),
            Err(Errno::ENOMEM)
        );
        // A new value for a read-only leaf.
        let mut len = 8;
        assert_eq!(
            rdquad(&mut req(UBUF, &mut len, UBUF, 8), 7),
            Err(Errno::EPERM)
        );
        // The value copied out.
        let mut len = 16;
        assert_eq!(rdstruct(&mut req(UBUF, &mut len, 0, 0), b"abc"), Ok(()));
        assert_eq!(len, 3);
        assert_eq!(testutil::user_bytes(UBUF, 3).unwrap(), b"abc");
    }

    #[test]
    fn rdint_checks_the_room_and_refuses_a_write() {
        testutil::install();
        let mut len = 2;
        assert_eq!(rdint(&mut req(U, &mut len, 0, 0), 7), Err(Errno::ENOMEM));
        let mut len = 4;
        assert_eq!(rdint(&mut req(U, &mut len, U + 8, 4), 7), Err(Errno::EPERM));
        let mut len = 0;
        assert_eq!(rdint(&mut req(0, &mut len, 0, 0), 7), Ok(()));
        assert_eq!(len, 4, "a size query writes *oldlenp");
        let mut len = 4;
        assert_eq!(rdint(&mut req(U, &mut len, 0, 0), 7), Ok(()));
        assert_eq!(UserPtr::<i32>::new(U).copyin(), Ok(7));
    }

    #[test]
    fn int_bounded_refuses_out_of_range_and_returns_the_old_value() {
        testutil::install();
        static V: AtomicI32 = AtomicI32::new(0);
        assert_eq!(UserPtr::<i32>::new(U + 16).copyout(&2), Ok(()));
        let mut len = 4;
        assert_eq!(
            int_bounded(&mut req(U + 32, &mut len, U + 16, 4), &V, 0, 1),
            Err(Errno::EINVAL)
        );
        assert_eq!(UserPtr::<i32>::new(U + 16).copyout(&1), Ok(()));
        assert_eq!(
            int_bounded(&mut req(U + 32, &mut len, U + 16, 4), &V, 0, 1),
            Ok(())
        );
        assert_eq!(V.load(Ordering::Relaxed), 1);
        assert_eq!(UserPtr::<i32>::new(U + 32).copyin(), Ok(0), "the old value");
        assert_eq!(
            int_bounded(&mut req(0, &mut len, U + 16, 4), &V, 1, 0),
            Err(Errno::EPERM),
            "read-only"
        );
        assert_eq!(
            int_bounded(&mut req(0, &mut len, U + 16, 3), &V, 0, 1),
            Err(Errno::EINVAL),
            "newlen"
        );
    }

    #[test]
    fn strings_are_copied_with_their_nul() {
        testutil::install();
        let mut len = 16;
        assert_eq!(
            rdstring(&mut req(U + 64, &mut len, 0, 0), b"agtimer\0junk"),
            Ok(())
        );
        assert_eq!(len, 8);
        assert_eq!(
            testutil::user_bytes(U + 64, 8).as_deref(),
            Some(&b"agtimer\0"[..])
        );
        let mut len = 7;
        assert_eq!(
            rdstring(&mut req(U + 64, &mut len, 0, 0), b"agtimer"),
            Err(Errno::ENOMEM)
        );
        // A new value lands NUL-terminated in the buffer; one as long as it is refused.
        assert_eq!(UserPtr::<u8>::new(U + 96).copyout_slice(b"dummy"), Ok(()));
        let mut buf = [0u8; 8];
        buf[..3].copy_from_slice(b"old");
        let mut len = 8;
        assert_eq!(
            string(&mut req(U + 64, &mut len, U + 96, 5), &mut buf),
            Ok(())
        );
        assert_eq!(&buf[..6], b"dummy\0");
        assert_eq!(
            testutil::user_bytes(U + 64, 4).as_deref(),
            Some(&b"old\0"[..])
        );
        assert_eq!(
            string(&mut req(0, &mut len, U + 96, 8), &mut buf),
            Err(Errno::EINVAL)
        );
    }

    #[test]
    fn rdstruct_copies_the_bytes() {
        testutil::install();
        let mut len = 3;
        assert_eq!(
            rdstruct(&mut req(U, &mut len, 0, 0), &[1, 2, 3, 4]),
            Err(Errno::ENOMEM)
        );
        let mut len = 0;
        assert_eq!(rdstruct(&mut req(0, &mut len, 0, 0), &[1, 2, 3, 4]), Ok(()));
        assert_eq!(len, 4);
    }
}
/* </TESTS> */
