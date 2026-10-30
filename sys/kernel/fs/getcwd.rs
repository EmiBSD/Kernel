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
 * Copyright (c) 1999 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Bill Sommerfeld.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `getcwd(3)` in the kernel: the path from a directory up to the root, found through the
//! name cache's reverse map (`vfs_getcwd_getcache`) or, failing that, by looking `..` up and
//! scanning it for the child's file number (`vfs_getcwd_scandir`). Shared by
//! `sys___getcwd`, `vn_isunder` and `sys___realpath`.
//!
//! Upstream: sys/kern/vfs_getcwd.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_getcwd.rs@3c62ede68802
//!
//! ## Deviations
//! - The output buffer and the position in it are `Option<(&mut [u8], &mut usize)>`: the
//!   path is built backwards from the index, where the C moves a `char *` (`*bpp`) inside
//!   `bufp`. `vfs_getcwd_getcache` returns `Ok(false)` for the C's `-1` ("try the hard
//!   way") (LZ's).
//! - `DIRBLKSIZ` is `ufs/ufs/dir.h`'s (`DEV_BSIZE`) (LZ's).
//! - With no root file system (no `rootvnode`) or no current directory, the walk fails with
//!   `ENOENT`, as `namei` does (LZ's).
//! - `KTRACE` is not configured.
//!
//! ## Redesign
//! - The directory buffer and the path buffer are `MArray<u8>`s of `M_TEMP` (the C's
//!   `malloc(M_TEMP)`, freed by their drop); `MArray` initialises them (a Rust value has no
//!   uninitialised bytes): at most `MAXPATHLEN * 4` bytes per `__getcwd` and one directory
//!   block per `vfs_getcwd_scandir` pass, written once where the C writes nothing. LZ's two
//!   `unsafe` slices over `malloc`'s memory go.
//! - The `..` lookup of `vfs_getcwd_scandir` is [`Componentname::dotdot`] (the C's stack
//!   componentname with `cn_nameptr = ".."`, no pathname buffer).
//! - `sys___getcwd` is the kernel's ([`sys___getcwd`]) over a user address and a length;
//!   the system call's entry stays `bsd`'s (`Legacy`, a wrapper in `vfs_syscalls.rs`) until
//!   the VFS entries flip together (D-N5.3-6, N5.4).

use core::ptr;

use hal::malloc::{M_WAITOK, MArray};
use hal::sys::dirent::{Dirent, dirent_recsize};
use hal::sys::errno::Errno;
use hal::user::UserPtr;

use super::cache::cache_revlookup;
use super::init::rootvnode;
use super::mount::Mount;
use super::namei::{Componentname, MAXPATHLEN, Nameidata};
use super::subr::{vget, vput, vref, vrele};
use super::vnode::{
    GETCWD_CHECK_ACCESS, LK_EXCLUSIVE, LK_RETRY, VDIR, VEXEC, VREAD, VROOT, Vattr, Vnode, vn_lock,
};
use super::vops::{Cred, vop_access, vop_getattr, vop_lookup, vop_readdir, vop_unlock};
use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Iovec, Uio};

/// `M_TEMP` (`<sys/malloc.h>`): misc temporary data buffers.
const M_TEMP: i32 = 127;

/// `DIRBLKSIZ` (`ufs/ufs/dir.h`, `DEV_BSIZE`): only for the smallest directory buffer.
const DIRBLKSIZ: usize = 512;

/// Find parent vnode of `*lvpp`, return in `*uvpp`.
///
/// On return `*lvpp` is `None` (its reference is gone); `*uvpp` holds the parent, if found.
/// With `buf`, the child's name is copied just before the index, which moves back to its
/// start.
pub fn vfs_getcwd_scandir<G: ProcGlue>(
    lvpp: &mut Option<&'static Vnode<G>>,
    uvpp: &mut Option<&'static Vnode<G>>,
    buf: Option<(&mut [u8], &mut usize)>,
    p: &Thread<G>,
) -> Result<(), Errno>
where
    G::FileGlue: for<'c> FileGlue<Componentname<'c> = Componentname<'c, G>, Nameidata<'c> = Nameidata<'c, G>>,
{
    let Some(lvp) = *lvpp else {
        kpanic(format_args!("vfs_getcwd_scandir: no vnode"));
    };
    let cred = Cred::real(p.ucred());
    let mut va = Vattr::new();

    // If we want the filename, get some info we need while the current directory is still
    // locked.
    if buf.is_some()
        && let Err(error) = vop_getattr(lvp, &mut va, cred, p)
    {
        vput(lvp);
        *lvpp = None;
        *uvpp = None;
        return Err(error);
    }

    let mut cn = Componentname::dotdot(p, cred);

    // Get parent vnode using lookup of '..'
    if let Err(error) = vop_lookup(lvp, uvpp, &mut cn) {
        vput(lvp);
        *lvpp = None;
        *uvpp = None;
        return Err(error);
    }

    let Some(uvp) = *uvpp else {
        kpanic(format_args!("vfs_getcwd_scandir: no parent"));
    };

    let error = 'out: {
        // If we don't care about the pathname, we're done
        let Some((bufp, bpp)) = buf else {
            break 'out Ok(());
        };

        let fileno = va.va_fileid;

        let mut dirbuflen = DIRBLKSIZ;
        if (dirbuflen as i64) < va.va_blocksize {
            dirbuflen = va.va_blocksize as usize;
        }
        // XXX we need some limit for fuse, 1 MB should be enough
        if dirbuflen > 0xfffff {
            break 'out Err(Errno::EINVAL);
        }
        let Some(mut dirbuf) = MArray::new(M_TEMP, M_WAITOK, dirbuflen, |_| 0u8) else {
            break 'out Err(Errno::ENOMEM);
        };

        let mut off = 0;
        let mut tries = 0;

        loop {
            let mut iov = [Iovec::new()];
            let mut uio = Uio::kernel_read(&mut iov, &mut dirbuf[..], Some(p));
            uio.uio_offset = off;

            let mut eofflag = 0;

            // Call VOP_READDIR of parent
            let error = vop_readdir(uvp, &mut uio, cred, &mut eofflag);

            off = uio.uio_offset;
            let resid = uio.uio_resid;

            // Try again if NFS tosses its cookies
            if error == Err(Errno::EINVAL) && tries < 3 {
                tries += 1;
                off = 0;
                continue;
            } else if let Err(e) = error {
                break 'out Err(e); // Old userland getcwd() behaviour
            }

            let mut cpos = 0;
            tries = 0;

            // Scan directory page looking for matching vnode
            let mut len = (dirbuflen - resid) as isize;
            while len > 0 {
                let Some(dp) = Dirent::from_bytes(&dirbuf[cpos..]) else {
                    break 'out Err(Errno::EINVAL);
                };
                let reclen = usize::from(dp.d_reclen);

                // Check for malformed directory
                if reclen < dirent_recsize(1) || reclen as isize > len {
                    break 'out Err(Errno::EINVAL);
                }

                if dp.d_fileno == fileno {
                    let namlen = usize::from(dp.d_namlen);
                    if Dirent::NAME_OFFSET + namlen > reclen {
                        break 'out Err(Errno::EINVAL);
                    }
                    if *bpp <= namlen {
                        break 'out Err(Errno::ERANGE);
                    }
                    let bp = *bpp - namlen;

                    let name = &dirbuf[cpos + Dirent::NAME_OFFSET..][..namlen];
                    bufp[bp..bp + namlen].copy_from_slice(name);
                    *bpp = bp;

                    break 'out Ok(());
                }

                cpos += reclen;
                len -= reclen as isize;
            }

            if eofflag != 0 {
                break;
            }
        }

        Err(Errno::ENOENT)
    };

    // out: (the directory buffer goes with its drop)
    vrele(lvp);
    *lvpp = None;

    error
}

/// Do a lookup in the vnode-to-name reverse map. `Ok(false)` is the C's `-1`: not found
/// there, the caller tries the hard way (with `*lvpp` still held and locked).
pub fn vfs_getcwd_getcache<G: ProcGlue>(
    lvpp: &mut Option<&'static Vnode<G>>,
    uvpp: &mut Option<&'static Vnode<G>>,
    buf: Option<(&mut [u8], &mut usize)>,
) -> Result<bool, Errno> {
    let Some(lvp) = *lvpp else {
        kpanic(format_args!("vfs_getcwd_getcache: no vnode"));
    };
    // Save original position to restore to on error
    let (bufp, bpp) = match buf {
        Some((bufp, bpp)) => (Some(bufp), Some(bpp)),
        None => (None, None),
    };
    let obp = bpp.as_deref().copied();

    let mut bp = obp.unwrap_or(0);
    let uvp = match cache_revlookup(lvp, &mut bp, bufp) {
        Err(error) => {
            vput(lvp);
            *lvpp = None;
            *uvpp = None;
            return Err(error);
        }
        Ok(None) => return Ok(false),
        Ok(Some(uvp)) => uvp,
    };
    *uvpp = Some(uvp);
    let restore = bpp.map(|bpp| {
        *bpp = bp;
        (bpp, obp.unwrap_or(0))
    });
    let vpid = uvp.id();

    // Release current lock before acquiring the parent lock
    let _ = vop_unlock(lvp);

    let mut error = vget(uvp, LK_EXCLUSIVE | LK_RETRY);
    if error.is_err() {
        *uvpp = None;
    }

    // Verify that vget() succeeded, and check that vnode capability didn't change while we
    // were waiting for the lock.
    if error.is_err() || vpid != uvp.id() {
        // Try to get our lock back. If that works, tell the caller to try things the hard
        // way, otherwise give up.
        if error.is_ok() {
            vput(uvp);
        }

        *uvpp = None;

        error = vn_lock(lvp, LK_EXCLUSIVE | LK_RETRY);
        if error.is_ok() {
            if let Some((bpp, obp)) = restore {
                *bpp = obp; // restore the buffer
            }
            return Ok(false);
        }
    }

    vrele(lvp);
    *lvpp = None;

    error.map(|()| true)
}

/// Common routine shared by `sys___getcwd()` and `vn_isunder()` and `sys___realpath()`: the
/// path from `lvp` up to `rvp` (the thread's root when `None`), written backwards into `buf`
/// before its index, at most `limit` vnodes up; `GETCWD_CHECK_ACCESS` checks search (then
/// read) access on the way.
pub fn vfs_getcwd_common<G: ProcGlue>(
    lvp: &'static Vnode<G>,
    rvp: Option<&'static Vnode<G>>,
    mut buf: Option<(&mut [u8], &mut usize)>,
    mut limit: i32,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno>
where
    G::FileGlue: for<'c> FileGlue<Componentname<'c> = Componentname<'c, G>, Nameidata<'c> = Nameidata<'c, G>>,
{
    let fdp = p.fd();
    let cred = Cred::real(p.ucred());
    let mut perms = VEXEC;

    let Some(rvp) = rvp.or_else(|| fdp.rdir()).or_else(rootvnode::<G>) else {
        // No root file system (see the module's deviations).
        return Err(Errno::ENOENT);
    };

    let first = lvp;
    vref(rvp);
    vref(first);

    let mut lvp: Option<&'static Vnode<G>> = Some(first);
    let mut uvp: Option<&'static Vnode<G>> = None;

    let error = 'out: {
        if let Err(error) = vn_lock(first, LK_EXCLUSIVE | LK_RETRY) {
            vrele(first);
            lvp = None;
            break 'out Err(error);
        }

        if ptr::eq(first, rvp) {
            if let Some((bufp, bpp)) = buf.as_mut() {
                **bpp -= 1;
                bufp[**bpp] = b'/';
            }
            break 'out Ok(());
        }

        // This loop will terminate when we hit the root, VOP_READDIR() or VOP_LOOKUP() fails,
        // or we run out of space in the user buffer.
        loop {
            let Some(mut cur) = lvp else {
                break 'out Err(Errno::ENOENT);
            };
            if cur.vtype() != VDIR {
                break 'out Err(Errno::ENOTDIR);
            }

            // Check for access if caller cares
            if flags & GETCWD_CHECK_ACCESS != 0 {
                if let Err(error) = vop_access(cur, perms, cred, p) {
                    break 'out Err(error);
                }
                perms = VEXEC | VREAD;
            }

            // Step up if we're a covered vnode
            while cur.flag() & VROOT != 0 {
                if ptr::eq(cur, rvp) {
                    break 'out Ok(());
                }

                let tvp = cur;
                let covered = cur.mount().and_then(Mount::vnodecovered);

                vput(tvp);

                let Some(covered) = covered else {
                    lvp = None;
                    break 'out Err(Errno::ENOENT);
                };
                cur = covered;
                lvp = Some(cur);

                vref(cur);

                if let Err(error) = vn_lock(cur, LK_EXCLUSIVE | LK_RETRY) {
                    vrele(cur);
                    lvp = None;
                    break 'out Err(error);
                }
            }

            // Look in the name cache
            let reborrow = buf.as_mut().map(|(b, i)| (&mut **b, &mut **i));
            let error = match vfs_getcwd_getcache(&mut lvp, &mut uvp, reborrow) {
                // If that fails, look in the directory
                Ok(false) => {
                    let reborrow = buf.as_mut().map(|(b, i)| (&mut **b, &mut **i));
                    vfs_getcwd_scandir(&mut lvp, &mut uvp, reborrow, p)
                }
                Ok(true) => Ok(()),
                Err(e) => Err(e),
            };

            if let Err(e) = error {
                break 'out Err(e);
            }

            #[cfg(feature = "diagnostic")]
            {
                if lvp.is_some() {
                    kpanic(format_args!("getcwd: oops, forgot to null lvp"));
                }
                if let Some((_, bpp)) = buf.as_ref()
                    && **bpp == 0
                {
                    kpanic(format_args!("getcwd: oops, went back too far"));
                }
            }

            if let Some((bufp, bpp)) = buf.as_mut() {
                **bpp -= 1;
                bufp[**bpp] = b'/';
            }

            lvp = uvp.take();
            limit -= 1;

            if lvp.is_some_and(|l| ptr::eq(l, rvp)) || limit <= 0 {
                break;
            }
        }
        Ok(())
    };

    // out:
    if let Some(uvp) = uvp {
        vput(uvp);
    }

    if let Some(lvp) = lvp {
        vput(lvp);
    }

    vrele(rvp);

    error
}

/// Find pathname of a process's current directory: `__getcwd(buf, len)`, the path copied
/// out to the user address `buf` (at most `len` bytes there, `MAXPATHLEN * 4` at most).
#[allow(non_snake_case)] // the C name: sys___getcwd
pub fn sys___getcwd<G: ProcGlue>(p: &Thread<G>, buf: usize, len: usize) -> Result<(), Errno>
where
    G::FileGlue: for<'c> FileGlue<Componentname<'c> = Componentname<'c, G>, Nameidata<'c> = Nameidata<'c, G>>,
{
    let mut len = len;
    if len > MAXPATHLEN * 4 {
        len = MAXPATHLEN * 4;
    } else if len < 2 {
        return Err(Errno::ERANGE);
    }

    let Some(mut path) = MArray::new(M_TEMP, M_WAITOK, len, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };

    let mut bp = len - 1;
    path[bp] = 0;

    // 5th argument here is "max number of vnodes to traverse". Since each entry takes up at
    // least 2 bytes in the output buffer, limit it to N/2 vnodes for an N byte buffer.
    let Some(cdir) = p.fd().cdir() else {
        return Err(Errno::ENOENT); // no root file system yet
    };
    vfs_getcwd_common(
        cdir,
        None,
        Some((&mut path[..], &mut bp)),
        (len / 2) as i32,
        GETCWD_CHECK_ACCESS,
        p,
    )?;

    // Put the result into user buffer
    let end = len.min(bp + MAXPATHLEN);
    UserPtr::<u8>::new(buf).copyoutstr(&path[bp..end])?;

    // KTRACE: not configured.
    Ok(())
}
/* </CODE> */
/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `getcwd` over the kernel's test file system: the path through the name
    // cache's reverse map (`vfs_getcwd_getcache` at every step), the root, `ERANGE` for a
    // buffer too small, `__getcwd`'s copy to user space and its bounds, `vn_isunder`. The
    // directory scan (`vfs_getcwd_scandir`) is `bsd`'s tests over its glue (the test glue's
    // `uiomove` cannot fill a kernel buffer with a directory's entries, `fs::testfs`'s
    // notes): here it gets as far as `VOP_READDIR`, which the test file system lacks.

    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::fs::lookup::{namei, ndinit};
    use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
    use crate::fs::subr::vfs;
    use crate::fs::testfs::{C, TestGlue, any_locked, test_thread, vget_node};
    use crate::fs::vnops::vn_isunder;
    use crate::testutil::USER_BASE;

    type G = TestGlue;

    /// The vnode of `path`, referenced, every name on the way now in the cache.
    fn walk(p: &'static Thread<G>, path: &'static [u8]) -> &'static Vnode<G> {
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd).unwrap();
        nd.ni_vp.unwrap()
    }

    /// `vfs_getcwd_common` of `vp` into a buffer of `len` bytes, as `__getcwd` calls it.
    fn path_of(vp: &'static Vnode<G>, len: usize, p: &Thread<G>) -> Result<Vec<u8>, Errno> {
        let mut buf = vec![0u8; len];
        let mut bp = len - 1;
        buf[bp] = 0;
        vfs_getcwd_common(
            vp,
            None,
            Some((&mut buf[..], &mut bp)),
            (len / 2) as i32,
            GETCWD_CHECK_ACCESS,
            p,
        )?;
        Ok(buf[bp..len - 1].to_vec())
    }

    #[test]
    fn getcwd_through_the_reverse_cache() {
        let (_g, p) = test_thread();
        let dvp = walk(p, b"/a/c/d");
        let hits = vfs::<G>()
            .nchstats
            .ncs_revhits
            .load(core::sync::atomic::Ordering::SeqCst);
        assert_eq!(path_of(dvp, 64, p).unwrap(), b"/a/c/d");
        assert_eq!(
            vfs::<G>()
                .nchstats
                .ncs_revhits
                .load(core::sync::atomic::Ordering::SeqCst),
            hits + 3
        );
        // The root is "/".
        let root = crate::fs::init::rootvnode::<G>().unwrap();
        assert_eq!(path_of(root, 8, p).unwrap(), b"/");
        // A buffer too small for the path: ERANGE.
        assert_eq!(path_of(dvp, 6, p), Err(Errno::ERANGE));
        // The walk stops at the limit (N/2 vnodes for N bytes).
        let mut buf = [0u8; 64];
        let mut bp = 63;
        vfs_getcwd_common(dvp, None, Some((&mut buf[..], &mut bp)), 1, 0, p).unwrap();
        assert_eq!(&buf[bp..63], b"/d");
        assert!(!any_locked());
        vrele(dvp);
    }

    #[test]
    fn a_directory_the_cache_does_not_name_is_scanned() {
        let (_g, p) = test_thread();
        let mp = crate::fs::init::rootvnode::<G>()
            .and_then(Vnode::mount)
            .unwrap();
        // C looked up only through VFS_VGET: its name is in no cache; the scan looks `..` up
        // and reads the parent, which the test file system cannot (`EOPNOTSUPP`).
        let cvp = vget_node(mp, C).unwrap();
        let _ = vop_unlock(cvp);
        assert_eq!(path_of(cvp, 64, p), Err(Errno::EOPNOTSUPP));
        assert!(!any_locked());
        vrele(cvp);
    }

    #[test]
    fn getcwd_copies_out_the_current_directory() {
        let (_g, p) = test_thread();
        let dvp = walk(p, b"/a/c/d");
        let old = p.fd().replace_cdir(Some(dvp));
        let ubuf = USER_BASE + 0x6000;
        sys___getcwd(p, ubuf, 64).unwrap();
        let mut out = [0u8; 7];
        UserPtr::<u8>::new(ubuf).copyin_slice(&mut out).unwrap();
        assert_eq!(&out, b"/a/c/d\0");
        // Bounds: below 2 bytes ERANGE, a path longer than the buffer ERANGE.
        assert_eq!(sys___getcwd(p, ubuf, 1), Err(Errno::ERANGE));
        assert_eq!(sys___getcwd(p, ubuf, 4), Err(Errno::ERANGE));
        // The C's limit (len / 2 vnodes) can stop the walk first: 5 bytes give 2 vnodes, the
        // tail of the path ("/c/d"), as the C answers.
        sys___getcwd(p, ubuf, 5).unwrap();
        let mut out = [0u8; 5];
        UserPtr::<u8>::new(ubuf).copyin_slice(&mut out).unwrap();
        assert_eq!(&out, b"/c/d\0");
        let cur = p.fd().replace_cdir(old);
        assert!(cur.is_some_and(|c| ptr::eq(c, dvp)));
        vrele(dvp);
    }

    #[test]
    fn isunder_answers_ancestry() {
        let (_g, p) = test_thread();
        let dvp = walk(p, b"/a/c/d");
        let avp = walk(p, b"/a");
        assert!(vn_isunder(dvp, avp, p));
        assert!(vn_isunder(dvp, dvp, p));
        assert!(!vn_isunder(avp, dvp, p));
        assert!(!any_locked());
        vrele(avp);
        vrele(dvp);
    }
}
/* </TESTS> */
