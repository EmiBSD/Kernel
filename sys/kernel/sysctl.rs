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
//! The sysctl(2) dispatch as the kernel crate's nodes implement it: [`SysctlNode`], one per
//! `*_sysctl` function the C dispatches by hand, the request ([`SysctlReq`], the user's
//! `oldp`, `*oldlenp`, `newp`, `newlen`) and the copy helpers of `kern_sysctl.c` the nodes
//! need (`sysctl_rdint`, `sysctl_int_bounded`, `sysctl_rdstruct`, `sysctl_string`,
//! `sysctl_rdstring`), over `hal::user`'s typed copy.
//!
//! Not an OpenBSD file of its own: the helpers re-express `kern_sysctl.c`'s (whose port is
//! still `bsd`'s `kern_sysctl.rs`, with the dispatch), with the same checks in the same order
//! and the same errnos. N4.6's sysctl core owns this module once it lands (the N4.7 and
//! N4.8 amendments: the first lander defines it); `kern.timecounter` and `kern.clockintr`
//! (N4.8a) are its first nodes.

use core::sync::atomic::{AtomicI32, Ordering};

use hal::user::UserPtr;

use crate::sys::errno::Errno;
pub use crate::vm::meter::SysctlReq;

/// A sysctl subtree the C serves with a `*_sysctl(name, namelen, oldp, oldlenp, newp,
/// newlen)` function: `name` is what is left of the MIB below the node.
pub trait SysctlNode: Sync {
    /// Serves the request for `name`.
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno>;
}

/// `copyout(src, oldp, len)` of bytes.
fn copyout_bytes(b: &[u8], uaddr: usize) -> Result<(), Errno> {
    UserPtr::<u8>::new(uaddr).copyout_slice(b)
}

/// `sysctl_rdint`: a read-only `int`.
pub fn rdint(req: &mut SysctlReq<'_>, val: i32) -> Result<(), Errno> {
    if req.oldp != 0 && *req.oldlenp < size_of::<i32>() {
        return Err(Errno::ENOMEM);
    }
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    *req.oldlenp = size_of::<i32>();
    if req.oldp != 0 {
        copyout_bytes(&val.to_ne_bytes(), req.oldp)?;
    }
    Ok(())
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

/// `sysctl_rdstruct`: a read-only structure's bytes.
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

    /// A user range of this module's own in the user-copy double.
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
