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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)vfs_lookup.c	8.6 (Berkeley) 11/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! Pathname translation: `namei` (copy the path in, pick the starting directory, call
//! `vfs_lookup`, expand symbolic links), `vfs_lookup` (walk the components, crossing mount
//! points and `..`), `vfs_relookup`, the realpath helpers `component_push`/`component_pop`
//! and `ndinitat`.
//!
//! Upstream: sys/kern/vfs_lookup.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_lookup.rs@3c62ede68802
//!
//! ## Deviations
//! - `NDINITAT`/`NDINIT` are [`ndinitat`]/[`ndinit`], returning the `nameidata`; the path's
//!   segment is the [`NiDirp`] variant (LZ's).
//! - `vfs_lookup`'s `goto bad`/`goto bad2` exits are the two error variants of an inner
//!   function (LZ's).
//! - No root file system yet: when there is no `rootvnode` (or no current directory for a
//!   relative path) `namei` fails with `ENOENT`, where the C cannot get there (`main`
//!   panics first when it cannot mount root) (LZ's).
//! - `KTRACE` is not configured; `NAMEI_DIAGNOSTIC` neither (LZ's).
//!
//! ## Redesign
//! - The pathname buffer is the componentname's own ([`super::namei::PathBuf`]): the walk is
//!   by index into it, `cn_nameptr` and `ni_next` are offsets, and the symbolic link's
//!   `memcpy(cp + linklen, ni_next, ni_pathlen)` is a copy between two owned buffers, the old
//!   one going back to its pool when it is replaced. LZ's eight `unsafe` (the buffer slices
//!   made from raw pointers, the thread detached from `ndp`, the link's copy) go.
//! - LZ's "`namei_pool` cannot sleep" deviation closes: `pool_get(PR_WAITOK)` sleeps.
//! - Every check of the C stays at its point, in its order: the pathname copy and
//!   `ENAMETOOLONG`, `ENOENT` for the empty path, `STRIPSLASHES`, the root (`fd_rdir`, or
//!   `rootvnode` for `KERNELPATH`), `BYPASSUNVEIL` only for `KERNELPATH` and otherwise
//!   `pledge_namei`, `unveil_start_relative` and `unveil_check_component` for a relative
//!   start, `unveil_check_component` for every component and every `..` step and at a link
//!   to an absolute path, `unveil_check_final`, `EJUSTRETURN` for `PLEDGE_UNVEIL` alone,
//!   `SYMLOOP_MAX`, and `BPU_LOCALTIME`/`BPU_ZONEINFO`'s three refusals. The pledge and
//!   unveil hooks are the glue's ([`ProcGlue::pledge_namei`], ...: `kern_pledge.c` and
//!   `kern_unveil.c` are `bsd`'s until N8). `pledge_namei` reads the pathname while it gets
//!   the `nameidata` mutably, so the buffer is lent to it out of the componentname (a move
//!   of the box, put back after). The C's `!cn_cred || !cn_proc` panic is a type: `cn_proc`
//!   is a reference and `namei` sets `cn_cred` from it.

use core::ptr;

use hal::sys::errno::Errno;
use hal::sys::fcntl::AT_FDCWD;
use hal::sys::mount::{VB_READ, VB_WAIT};
use hal::sys::pledge::PLEDGE_UNVEIL;
use hal::sys::syslimits::{NAME_MAX, SYMLOOP_MAX};
use hal::user::UserPtr;

use super::init::rootvnode;
use super::mount::{Mount, vfs_busy, vfs_unbusy};
use super::namei::{
    BPU_LOCALTIME, BPU_ZONEINFO, BYPASSUNVEIL, CREATE, Componentname, DELETE, EXECPATH, FOLLOW,
    HASBUF, ISDOTDOT, ISLASTCN, ISSYMLINK, KERNELPATH, LOCKLEAF, LOCKPARENT, MAKEENTRY, MAXPATHLEN,
    NO_PATHBUF, NOCACHE, NOCROSSMOUNT, Nameidata, NiDirp, PDIRUNLOCK, PathBuf, RDONLY, REALPATH,
    RENAME, REQUIREDIR, SAVENAME, SAVESTART, STRIPSLASHES, WANTPARENT, pathbuf_get,
};
use super::subr::{vput, vref, vrele};
use super::vnode::{LK_EXCLUSIVE, LK_RETRY, VDIR, VLNK, VREG, VROOT, Vnode, vn_lock};
use super::vops::{Cred, VFS_ROOT, vop_lookup, vop_readlink, vop_unlock};
use crate::assert::kpanic;
use crate::descrip::fd_getfile;
use crate::file::{DTYPE_VNODE, FileGlue};
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Iovec, Uio, UioSeg};

/// The pathname buffer of a componentname being walked (`namei` gave it one): `panic(9)`
/// without one.
fn pnbuf_of(b: Option<&PathBuf>) -> &[u8; MAXPATHLEN] {
    match b {
        Some(b) => b,
        None => kpanic(format_args!("namei: no cn_pnbuf")),
    }
}

/// The pathname buffer, to write.
fn pnbuf_of_mut(b: &mut Option<PathBuf>) -> &mut [u8; MAXPATHLEN] {
    match b.as_deref_mut() {
        Some(b) => b,
        None => kpanic(format_args!("namei: no cn_pnbuf")),
    }
}

/// The realpath buffer of a `REALPATH`/`EXECPATH` lookup: `panic(9)` without one (the
/// caller gives it, `sys___realpath`, exec).
fn rpbuf_of(b: &mut Option<PathBuf>) -> &mut [u8; MAXPATHLEN] {
    match b.as_deref_mut() {
        Some(b) => b,
        None => kpanic(format_args!("namei: REALPATH without cn_rpbuf")),
    }
}

/// `component_push` over the realpath buffer `rp` and its index.
fn push_into(rp: &mut [u8; MAXPATHLEN], rpi: &mut usize, component: &[u8]) -> bool {
    let len = component.len();
    if *rpi + len + 1 >= MAXPATHLEN {
        return false;
    }
    if *rpi > 1 {
        rp[*rpi] = b'/';
        *rpi += 1;
    }
    rp[*rpi..*rpi + len].copy_from_slice(component);
    *rpi += len;
    rp[*rpi] = 0;
    true
}

/// `component_pop` over the realpath buffer `rp` and its index.
fn pop_from(rp: &mut [u8; MAXPATHLEN], rpi: &mut usize) {
    while *rpi != 0 && rp[*rpi] != b'/' {
        *rpi -= 1;
    }
    if *rpi == 0 && rp[0] == b'/' {
        *rpi += 1;
    }
    rp[*rpi] = 0;
}

/// `component_push(cnp, component, len)`: appends `/component` to the realpath buffer;
/// `false` when it would not fit.
pub fn component_push<G: ProcGlue>(cnp: &mut Componentname<'_, G>, component: &[u8]) -> bool {
    push_into(rpbuf_of(&mut cnp.cn_rpbuf), &mut cnp.cn_rpi, component)
}

/// `component_pop(cnp)`: drops the last component of the realpath buffer.
pub fn component_pop<G: ProcGlue>(cnp: &mut Componentname<'_, G>) {
    pop_from(rpbuf_of(&mut cnp.cn_rpbuf), &mut cnp.cn_rpi);
}

/// The realpath buffer set to `"/"` (`cn_rpbuf[0] = '/'; cn_rpbuf[1] = '\0'; cn_rpi = 1`).
fn rp_root<G: ProcGlue>(cnp: &mut Componentname<'_, G>) {
    let rp = rpbuf_of(&mut cnp.cn_rpbuf);
    rp[0] = b'/';
    rp[1] = 0;
    cnp.cn_rpi = 1;
}

/// `ndinitat(ndp, op, flags, segflg, dirfd, namep, p)`: a `nameidata` for a lookup relative
/// to `dirfd`.
pub fn ndinitat<'a, G: ProcGlue>(
    op: u64,
    flags: u64,
    dirfd: i32,
    namep: NiDirp<'a>,
    p: &'a Thread<G>,
) -> Nameidata<'a, G> {
    Nameidata {
        ni_dirp: namep,
        ni_dirfd: dirfd,
        ni_segflg: match namep {
            NiDirp::User(_) => UioSeg::UIO_USERSPACE,
            NiDirp::Sys(_) => UioSeg::UIO_SYSSPACE,
        },
        ni_startdir: None,
        ni_rootdir: None,
        ni_pledge: 0,
        ni_unveil: 0,
        ni_vp: None,
        ni_dvp: None,
        ni_pathlen: 0,
        ni_next: 0,
        ni_loopcnt: 0,
        ni_unveil_match: Default::default(),
        // memset(0): no credentials until namei sets them.
        ni_cnd: Componentname::new(op, flags, p, Cred::NULL),
    }
}

/// `NDINIT(ndp, op, flags, segflg, namep, p)`: `ndinitat` relative to the current directory.
pub fn ndinit<'a, G: ProcGlue>(
    op: u64,
    flags: u64,
    namep: NiDirp<'a>,
    p: &'a Thread<G>,
) -> Nameidata<'a, G> {
    ndinitat(op, flags, AT_FDCWD, namep, p)
}

/// Gives the pathname buffer back and fails the lookup (`fail:` in C).
fn namei_fail<G: ProcGlue>(ndp: &mut Nameidata<'_, G>, error: Errno) -> Result<(), Errno> {
    drop(ndp.ni_cnd.take_pnbuf());
    ndp.ni_vp = None;
    Err(error)
}

/// The pathname up to its NUL.
fn pn_str(buf: &[u8]) -> &[u8] {
    &buf[..buf.iter().position(|&c| c == 0).unwrap_or(buf.len())]
}

/// `pledge_namei(p, ndp, cnp->cn_pnbuf)`: the glue's check, lent the pathname out of the
/// componentname for the call.
fn pledge_namei_of<G: ProcGlue>(p: &Thread<G>, ndp: &mut Nameidata<'_, G>) -> Result<(), Errno> {
    let buf = ndp.ni_cnd.take_pnbuf();
    let path = buf.as_deref().map_or(&[][..], |b| pn_str(b));
    let error = G::pledge_namei(p, ndp, path);
    ndp.ni_cnd.cn_pnbuf = buf;
    error
}

/// Convert a pathname into a pointer to a vnode.
///
/// The `FOLLOW` flag is set when symbolic links are to be followed when they occur at the end
/// of the name translation process. Symbolic links are always followed for all other
/// pathname components other than the last.
///
/// If the `LOCKLEAF` flag is set, a locked vnode is returned.
///
/// The segflg defines whether the name is to be copied from user space or kernel space.
///
/// Overall outline of namei:
///
/// ```text
///     copy in name
///     get starting directory
///     while (!done && !error) {
///             call lookup to search path.
///             if symbolic link, massage name in buffer and continue
///     }
/// ```
pub fn namei<G: ProcGlue>(ndp: &mut Nameidata<'_, G>) -> Result<(), Errno> {
    let p = ndp.ni_cnd.cn_proc;
    ndp.ni_cnd.cn_cred = Cred::real(p.ucred());
    #[cfg(feature = "diagnostic")]
    {
        if ndp.ni_cnd.cn_nameiop & !super::namei::OPMASK != 0 {
            kpanic(format_args!("namei: nameiop contaminated with flags"));
        }
        if ndp.ni_cnd.cn_flags & super::namei::OPMASK != 0 {
            kpanic(format_args!("namei: flags contaminated with nameiops"));
        }
    }
    let fdp = p.fd();

    // Get a buffer for the name to be translated, and copy the name into the buffer.
    if ndp.ni_cnd.cn_flags & HASBUF == 0 {
        let Some(b) = pathbuf_get::<G>() else {
            ndp.ni_vp = None;
            return Err(NO_PATHBUF);
        };
        ndp.ni_cnd.cn_pnbuf = Some(b);
    }
    let copied = {
        let pn = pnbuf_of_mut(&mut ndp.ni_cnd.cn_pnbuf);
        match ndp.ni_dirp {
            NiDirp::Sys(path) => {
                // strlcpy(cn_pnbuf, ni_dirp, MAXPATHLEN)
                let path = pn_str(path);
                if path.len() >= MAXPATHLEN {
                    Err(Errno::ENAMETOOLONG)
                } else {
                    pn[..path.len()].copy_from_slice(path);
                    pn[path.len()] = 0;
                    Ok(path.len() + 1) // ni_pathlen includes NUL
                }
            }
            NiDirp::User(uaddr) => UserPtr::<u8>::new(uaddr).copyinstr(pn),
        }
    };
    ndp.ni_pathlen = match copied {
        // Fail on null pathnames
        Ok(1) => return namei_fail(ndp, Errno::ENOENT),
        Ok(len) => len,
        Err(e) => return namei_fail(ndp, e),
    };

    // KTRACE: not configured.

    // Strip trailing slashes, as requested
    if ndp.ni_cnd.cn_flags & STRIPSLASHES != 0 {
        let pn = pnbuf_of_mut(&mut ndp.ni_cnd.cn_pnbuf);
        let end = ndp.ni_pathlen as isize - 2;
        let mut cp = end;
        while cp >= 0 && pn[cp as usize] == b'/' {
            cp -= 1;
        }

        // Still some remaining characters in the buffer
        if cp >= 0 {
            ndp.ni_pathlen -= (end - cp) as usize;
            pn[(cp + 1) as usize] = 0;
        }
    }

    ndp.ni_loopcnt = 0;

    // Get starting point for the translation.
    ndp.ni_rootdir = fdp.rdir();
    if ndp.ni_rootdir.is_none() || ndp.ni_cnd.cn_flags & KERNELPATH != 0 {
        ndp.ni_rootdir = rootvnode::<G>();
    }
    let Some(rootdir) = ndp.ni_rootdir else {
        // No root file system (see the module's deviations).
        return namei_fail(ndp, Errno::ENOENT);
    };

    if ndp.ni_cnd.cn_flags & KERNELPATH != 0 {
        ndp.ni_cnd.cn_flags |= BYPASSUNVEIL;
    } else if let Err(e) = pledge_namei_of(p, ndp) {
        return namei_fail(ndp, e);
    }

    // Check if starting from root directory or current directory.
    let mut dp: &'static Vnode<G>;
    if pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref())[0] == b'/' {
        dp = rootdir;
        vref(dp);
        if ndp.ni_cnd.cn_flags & (REALPATH | EXECPATH) != 0 && ndp.ni_cnd.cn_rpi == 0 {
            rp_root(&mut ndp.ni_cnd);
        }
    } else if ndp.ni_dirfd == AT_FDCWD {
        let Some(cdir) = fdp.cdir() else {
            // No current directory before a root is mounted (see the module's deviations).
            return namei_fail(ndp, Errno::ENOENT);
        };
        dp = cdir;
        vref(dp);
        G::unveil_start_relative(p, ndp, dp);
        G::unveil_check_component(p, ndp, dp);
    } else {
        let Some(fp) = fd_getfile(fdp, ndp.ni_dirfd) else {
            return namei_fail(ndp, Errno::EBADF);
        };
        let dir = fp
            .data()
            .and_then(<G::FileGlue as FileGlue>::vnode_of)
            .filter(|vp| fp.f_type() == DTYPE_VNODE && vp.vtype() == VDIR);
        let Some(dir) = dir else {
            let _ = fp.frele(Some(p));
            return namei_fail(ndp, Errno::ENOTDIR);
        };
        dp = dir;
        vref(dp);
        G::unveil_start_relative(p, ndp, dp);
        G::unveil_check_component(p, ndp, dp);
        let _ = fp.frele(Some(p));
    }

    let error = loop {
        if dp.mount().is_none() {
            // Give up if the directory is no longer mounted
            vrele(dp);
            return namei_fail(ndp, Errno::ENOENT);
        }

        ndp.ni_cnd.set_name_at(0, 0);
        ndp.ni_startdir = Some(dp);
        if let Err(e) = vfs_lookup(ndp) {
            return namei_fail(ndp, e);
        }

        // If not a symbolic link, return search result.
        if ndp.ni_cnd.cn_flags & ISSYMLINK == 0 {
            if let Err(e) = G::unveil_check_final(p, ndp) {
                let cn_flags = ndp.ni_cnd.cn_flags;
                if cn_flags & LOCKPARENT != 0
                    && cn_flags & ISLASTCN != 0
                    && !opt_eq(ndp.ni_vp, ndp.ni_dvp)
                    && let Some(dvp) = ndp.ni_dvp
                {
                    vput(dvp);
                }
                if let Some(vp) = ndp.ni_vp {
                    if cn_flags & LOCKLEAF != 0 {
                        vput(vp);
                    } else {
                        vrele(vp);
                    }
                }
                return namei_fail(ndp, e);
            }
            if ndp.ni_cnd.cn_flags & (SAVENAME | SAVESTART) == 0 {
                drop(ndp.ni_cnd.take_pnbuf());
            } else {
                ndp.ni_cnd.cn_flags |= HASBUF;
            }
            return Ok(());
        }
        let (Some(vp), Some(dvp)) = (ndp.ni_vp, ndp.ni_dvp) else {
            kpanic(format_args!("namei: symlink without vnodes"));
        };
        if ndp.ni_cnd.cn_flags & LOCKPARENT != 0 && ndp.ni_cnd.cn_flags & ISLASTCN != 0 {
            let _ = vop_unlock(dvp);
        }
        let loopcnt = ndp.ni_loopcnt;
        ndp.ni_loopcnt += 1;
        if loopcnt >= SYMLOOP_MAX as u64 {
            break Errno::ELOOP;
        }
        // The link is read into a fresh buffer when the path goes on after it, into the
        // pathname buffer itself otherwise.
        let mut fresh: Option<PathBuf> = None;
        if ndp.ni_pathlen > 1 {
            match pathbuf_get::<G>() {
                Some(b) => fresh = Some(b),
                None => break NO_PATHBUF,
            }
        }
        let cred = ndp.ni_cnd.cn_cred;
        let resid = {
            let cp: &mut [u8] = match fresh.as_deref_mut() {
                Some(b) => b,
                None => pnbuf_of_mut(&mut ndp.ni_cnd.cn_pnbuf),
            };
            let mut aiov = [Iovec::new()];
            let mut auio = Uio::kernel_read(&mut aiov, &mut cp[..MAXPATHLEN], Some(p));
            if let Err(e) = vop_readlink(vp, &mut auio, cred) {
                // badlink: the fresh buffer goes back with its drop.
                break e;
            }
            auio.uio_resid
        };
        let linklen = MAXPATHLEN - resid;
        if linklen == 0 {
            break Errno::ENOENT;
        }
        if linklen + ndp.ni_pathlen >= MAXPATHLEN {
            break Errno::ENAMETOOLONG;
        }
        match fresh {
            Some(mut cp) => {
                let (next, len) = (ndp.ni_next, ndp.ni_pathlen);
                cp[linklen..linklen + len]
                    .copy_from_slice(&pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref())[next..next + len]);
                // The old buffer goes back to its pool.
                ndp.ni_cnd.cn_pnbuf = Some(cp);
            }
            None => pnbuf_of_mut(&mut ndp.ni_cnd.cn_pnbuf)[linklen] = 0,
        }
        ndp.ni_pathlen += linklen;
        if ndp.ni_cnd.cn_flags & BPU_LOCALTIME != 0 {
            // /etc/localtime can be a symbolic link but must point into /usr/share/zoneinfo/
            // without ..
            if !G::checkzoneinfopath(pn_str(pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref()))) {
                break Errno::EACCES;
            }
            ndp.ni_cnd.cn_flags &= !BPU_LOCALTIME;
            ndp.ni_cnd.cn_flags |= BPU_ZONEINFO;
        }
        vput(vp);
        dp = dvp;
        // Check if root directory should replace current directory.
        if pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref())[0] == b'/' {
            vrele(dp);
            dp = rootdir;
            vref(dp);
            ndp.ni_unveil_match = Default::default();
            G::unveil_check_component(p, ndp, dp);
            if ndp.ni_cnd.cn_flags & (REALPATH | EXECPATH) != 0 {
                rp_root(&mut ndp.ni_cnd);
            }
        } else if ndp.ni_cnd.cn_flags & (REALPATH | EXECPATH) != 0 {
            component_pop(&mut ndp.ni_cnd);
        }
    };
    if let Some(dvp) = ndp.ni_dvp {
        vrele(dvp);
    }
    if let Some(vp) = ndp.ni_vp {
        vput(vp);
    }
    namei_fail(ndp, error)
}

/// `a == b` for two optional vnodes.
fn opt_eq<G: ProcGlue>(a: Option<&'static Vnode<G>>, b: Option<&'static Vnode<G>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// The directory `vfs_lookup` is searching, and whether it is already unlocked.
struct Walk<G: ProcGlue> {
    dp: &'static Vnode<G>,
    dpunlocked: bool,
}

/// The two error exits of `vfs_lookup`.
enum LookupError {
    /// `goto bad`: release the directory being searched.
    Bad(Errno),
    /// `goto bad2`: release the parent first.
    Bad2(Errno),
}

/// Search a pathname. This is a very central and rather complicated routine.
///
/// The pathname is pointed to by `ni_cnd.cn_nameptr` and is of length `ni_pathlen`. The
/// starting directory is taken from `ni_startdir`. The pathname is descended until done, or a
/// symbolic link is encountered. If the path is completed the flag `ISLASTCN` is set in
/// `ni_cnd.cn_flags`. If a symbolic link need interpretation is encountered, the flag
/// `ISSYMLINK` is set in `ni_cnd.cn_flags`.
///
/// The flag argument is `LOOKUP`, `CREATE`, `RENAME`, or `DELETE` depending on whether the
/// name is to be looked up, created, renamed, or deleted. When `CREATE`, `RENAME`, or
/// `DELETE` is specified, information usable in creating, renaming, or deleting a directory
/// entry may be calculated. If flag has `LOCKPARENT` or'ed into it, the parent directory is
/// returned locked. If flag has `WANTPARENT` or'ed into it, the parent directory is returned
/// unlocked. Otherwise the parent directory is not returned. If the target of the pathname
/// exists and `LOCKLEAF` is or'ed into the flag the target is returned locked, otherwise it
/// is returned unlocked. When creating or renaming and `LOCKPARENT` is specified, the target
/// may not be ".". When deleting and `LOCKPARENT` is specified, the target may be ".".
///
/// Overall outline of lookup:
///
/// ```text
/// dirloop:
///     identify next component of name at ndp->ni_ptr
///     handle degenerate case where name is null string
///     if .. and crossing mount points and on mounted filesys, find parent
///     call VOP_LOOKUP routine for next component name
///         directory vnode returned in ni_dvp, unlocked unless LOCKPARENT set
///         component vnode returned in ni_vp (if it exists), locked.
///     if result vnode is mounted on and crossing mount points,
///         find mounted on vnode
///     if more components of name, do next level at dirloop
///     return the answer in ni_vp, locked if LOCKLEAF set
///         if LOCKPARENT set, return locked parent in ni_dvp
///         if WANTPARENT set, return unlocked parent in ni_dvp
/// ```
pub fn vfs_lookup<G: ProcGlue>(ndp: &mut Nameidata<'_, G>) -> Result<(), Errno> {
    // Setup: break out flag bits into variables.
    let wantparent = ndp.ni_cnd.cn_flags & (LOCKPARENT | WANTPARENT) != 0;
    let mut docache = (ndp.ni_cnd.cn_flags & NOCACHE) ^ NOCACHE != 0;
    if ndp.ni_cnd.cn_nameiop == DELETE || (wantparent && ndp.ni_cnd.cn_nameiop != CREATE) {
        docache = false;
    }
    let rdonly = ndp.ni_cnd.cn_flags & RDONLY != 0;
    ndp.ni_dvp = None;
    ndp.ni_cnd.cn_flags &= !ISSYMLINK;
    let Some(dp) = ndp.ni_startdir.take() else {
        kpanic(format_args!("vfs_lookup: no ni_startdir"));
    };
    let _ = vn_lock(dp, LK_EXCLUSIVE | LK_RETRY);

    let mut st = Walk {
        dp,
        dpunlocked: false,
    };
    match lookup_walk(ndp, &mut st, wantparent, docache, rdonly) {
        Ok(()) => Ok(()),
        Err(err) => {
            let error = match err {
                LookupError::Bad2(error) => {
                    let flags = ndp.ni_cnd.cn_flags;
                    if let Some(dvp) = ndp.ni_dvp {
                        if flags & LOCKPARENT != 0
                            && flags & ISLASTCN != 0
                            && flags & PDIRUNLOCK == 0
                        {
                            let _ = vop_unlock(dvp);
                        }
                        vrele(dvp);
                    }
                    error
                }
                LookupError::Bad(error) => error,
            };
            if st.dpunlocked {
                vrele(st.dp);
            } else {
                vput(st.dp);
            }
            ndp.ni_vp = None;
            Err(error)
        }
    }
}

/// The body of `vfs_lookup`, from the leading slashes to `terminal:`.
#[allow(
    clippy::too_many_lines,
    reason = "the C's vfs_lookup, one function between its labels"
)]
fn lookup_walk<G: ProcGlue>(
    ndp: &mut Nameidata<'_, G>,
    st: &mut Walk<G>,
    wantparent: bool,
    docache: bool,
    rdonly: bool,
) -> Result<(), LookupError> {
    use LookupError::{Bad, Bad2};

    let p = ndp.ni_cnd.cn_proc;
    let mut name = ndp.ni_cnd.name_off().unwrap_or(0);

    // If we have a leading string of slashes, remove them, and just make sure the current
    // node is a directory.
    let mut terminal = false;
    if pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref())[name] == b'/' {
        let pn = pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref());
        let mut cp = name;
        while pn[cp] == b'/' {
            cp += 1;
        }
        ndp.ni_pathlen -= cp - name;
        name = cp;
        ndp.ni_cnd.set_name_at(name, 0);

        if st.dp.vtype() != VDIR {
            return Err(Bad(Errno::ENOTDIR));
        }

        // If we've exhausted the path name, then just return the current node. If the caller
        // requested the parent node (i.e. it's a CREATE, DELETE, or RENAME), and we don't have
        // one (because this is the root directory), then we must fail.
        if pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref())[name] == 0 {
            if ndp.ni_dvp.is_none() && wantparent {
                return Err(Bad(Errno::EISDIR));
            }
            ndp.ni_vp = Some(st.dp);
            ndp.ni_cnd.cn_flags |= ISLASTCN;
            terminal = true;
        }
    }

    if !terminal {
        // dirloop: Search a new directory.
        //
        // The last component of the filename is left accessible via cnp->cn_nameptr for
        // callers that need the name. Callers needing the name set the SAVENAME flag. When
        // done, they assume responsibility for freeing the pathname buffer.
        loop {
            ndp.ni_cnd.cn_consume = 0;

            // XXX: Figure out the length of the last component.
            let mut cp = name;
            {
                let pn = pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref());
                while pn[cp] != 0 && pn[cp] != b'/' {
                    cp += 1;
                }
            }
            let namelen = cp - name;
            ndp.ni_cnd.set_name_at(name, namelen);
            if namelen > NAME_MAX {
                return Err(Bad(Errno::ENAMETOOLONG));
            }

            if ndp.ni_cnd.cn_flags & (REALPATH | EXECPATH) != 0 {
                let cn = &mut ndp.ni_cnd;
                let pn = pnbuf_of(cn.cn_pnbuf.as_ref());
                let component = &pn[name..cp];
                if component == b".." {
                    pop_from(rpbuf_of(&mut cn.cn_rpbuf), &mut cn.cn_rpi);
                } else if component != b"."
                    && !push_into(rpbuf_of(&mut cn.cn_rpbuf), &mut cn.cn_rpi, component)
                {
                    if cn.cn_flags & REALPATH != 0 {
                        return Err(Bad(Errno::ENAMETOOLONG));
                    }
                    // EXECPATH: give up building path
                    cn.cn_rpi = 0;
                    cn.cn_flags &= !EXECPATH;
                }
            }

            ndp.ni_pathlen -= namelen;
            let mut next = cp;
            // If this component is followed by a slash, then move the pointer to the next
            // component forward, and remember that this component must be a directory.
            let slashes;
            let at_end;
            {
                let pn = pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref());
                if pn[cp] == b'/' {
                    while pn[cp] == b'/' {
                        cp += 1;
                    }
                    slashes = cp - next;
                    ndp.ni_pathlen -= slashes;
                    next = cp;
                    ndp.ni_cnd.cn_flags |= REQUIREDIR;
                } else {
                    slashes = 0;
                    ndp.ni_cnd.cn_flags &= !REQUIREDIR;
                }
                at_end = pn[cp] == 0;
            }
            ndp.ni_next = next;
            // We do special processing on the last component, whether or not it's a
            // directory. Cache all intervening lookups, but not the final one.
            if at_end {
                if docache {
                    ndp.ni_cnd.cn_flags |= MAKEENTRY;
                } else {
                    ndp.ni_cnd.cn_flags &= !MAKEENTRY;
                }
                ndp.ni_cnd.cn_flags |= ISLASTCN;
            } else {
                ndp.ni_cnd.cn_flags |= MAKEENTRY;
                ndp.ni_cnd.cn_flags &= !ISLASTCN;
            }
            if ndp.ni_cnd.name() == b".." {
                ndp.ni_cnd.cn_flags |= ISDOTDOT;
            } else {
                ndp.ni_cnd.cn_flags &= !ISDOTDOT;
            }

            'nextname: {
                // Handle "..": two special cases.
                // 1. If at root directory (e.g. after chroot) or at absolute root directory
                //    then ignore it so can't get out.
                // 2. If this vnode is the root of a mounted filesystem, then replace it with
                //    the vnode which was mounted on so we take the .. in the other file
                //    system.
                if ndp.ni_cnd.cn_flags & ISDOTDOT != 0 {
                    loop {
                        G::unveil_check_component(p, ndp, st.dp);
                        if opt_eq(Some(st.dp), ndp.ni_rootdir)
                            || opt_eq(Some(st.dp), rootvnode::<G>())
                        {
                            ndp.ni_dvp = Some(st.dp);
                            ndp.ni_vp = Some(st.dp);
                            vref(st.dp);
                            ndp.ni_unveil_match = Default::default();
                            break 'nextname;
                        }
                        if st.dp.flag() & VROOT == 0 || ndp.ni_cnd.cn_flags & NOCROSSMOUNT != 0 {
                            break;
                        }
                        let tdp = st.dp;
                        let Some(covered) = st.dp.mount().and_then(Mount::vnodecovered) else {
                            break;
                        };
                        st.dp = covered;
                        vput(tdp);
                        vref(st.dp);
                        G::unveil_check_component(p, ndp, st.dp);
                        let _ = vn_lock(st.dp, LK_EXCLUSIVE | LK_RETRY);
                    }
                }

                // We now have a segment name to search for, and a directory to search.
                ndp.ni_dvp = Some(st.dp);
                ndp.ni_vp = None;
                ndp.ni_cnd.cn_flags &= !PDIRUNLOCK;
                G::unveil_check_component(p, ndp, st.dp);

                if let Err(lookup_error) = vop_lookup(st.dp, &mut ndp.ni_vp, &mut ndp.ni_cnd) {
                    #[cfg(feature = "diagnostic")]
                    if ndp.ni_vp.is_some() {
                        kpanic(format_args!("leaf should be empty"));
                    }
                    let mut error = lookup_error;
                    // Allow for unveiling a file in a directory which we cannot create
                    // ourselves.
                    if ndp.ni_pledge == PLEDGE_UNVEIL
                        && matches!(error, Errno::EPERM | Errno::EACCES | Errno::EROFS)
                    {
                        error = Errno::EJUSTRETURN;
                    }

                    if error != Errno::EJUSTRETURN {
                        return Err(Bad(error));
                    }
                    // If this was not the last component, or there were trailing slashes,
                    // then the name must exist.
                    if ndp.ni_cnd.cn_flags & REQUIREDIR != 0 {
                        return Err(Bad(Errno::ENOENT));
                    }
                    // If creating and at end of pathname, then can consider allowing file to
                    // be created. Check for a read only filesystem and disallow this unless
                    // we are unveil'ing
                    if ndp.ni_pledge != PLEDGE_UNVEIL && (rdonly || mnt_rdonly(ndp.ni_dvp)) {
                        return Err(Bad(Errno::EROFS));
                    }
                    // We return with ni_vp NULL to indicate that the entry doesn't currently
                    // exist, leaving a pointer to the (possibly locked) directory vnode in
                    // ndp->ni_dvp.
                    if ndp.ni_cnd.cn_flags & SAVESTART != 0 {
                        ndp.ni_startdir = ndp.ni_dvp;
                        if let Some(sd) = ndp.ni_startdir {
                            vref(sd);
                        }
                    }
                    return Ok(());
                }

                // Take into account any additional components consumed by the underlying
                // filesystem. This will include any trailing slashes after the last
                // component consumed.
                if ndp.ni_cnd.cn_consume > 0 {
                    let consume = ndp.ni_cnd.cn_consume as usize;
                    if consume >= slashes {
                        ndp.ni_cnd.cn_flags &= !REQUIREDIR;
                    }

                    ndp.ni_pathlen = ndp.ni_pathlen + slashes - consume;
                    next = next + consume - slashes;
                    ndp.ni_next = next;
                    ndp.ni_cnd.cn_consume = 0;
                    if pnbuf_of(ndp.ni_cnd.cn_pnbuf.as_ref())[next] == 0 {
                        ndp.ni_cnd.cn_flags |= ISLASTCN;
                    }
                }

                let Some(found) = ndp.ni_vp else {
                    kpanic(format_args!("vfs_lookup: no vnode found"));
                };
                st.dp = found;
                // Check to see if the vnode has been mounted on; if so find the root of the
                // mounted file system.
                while st.dp.vtype() == VDIR
                    && let Some(mp) = st.dp.mountedhere()
                    && ndp.ni_cnd.cn_flags & NOCROSSMOUNT == 0
                {
                    if vfs_busy(mp, VB_READ | VB_WAIT).is_err() {
                        continue;
                    }
                    let _ = vop_unlock(st.dp);
                    let root = VFS_ROOT(mp);
                    vfs_unbusy(mp);
                    match root {
                        Err(error) => {
                            st.dpunlocked = true;
                            return Err(Bad2(error));
                        }
                        Ok(tdp) => {
                            vrele(st.dp);
                            st.dp = tdp;
                            ndp.ni_vp = Some(tdp);
                        }
                    }
                }

                // Check for symbolic link. Back up over any slashes that we skipped, as we
                // will need them again.
                if st.dp.vtype() == VLNK && ndp.ni_cnd.cn_flags & (FOLLOW | REQUIREDIR) != 0 {
                    ndp.ni_pathlen += slashes;
                    ndp.ni_next = next - slashes;
                    ndp.ni_cnd.cn_flags |= ISSYMLINK;
                    if ndp.ni_cnd.cn_flags & BPU_ZONEINFO != 0 {
                        // /usr/share/zoneinfo prohibits symbolic links
                        return Err(Bad2(Errno::ELOOP));
                    }
                    return Ok(());
                }

                // Check for directory, if the component was followed by a series of slashes.
                if st.dp.vtype() != VDIR && ndp.ni_cnd.cn_flags & REQUIREDIR != 0 {
                    return Err(Bad2(Errno::ENOTDIR));
                }
            }

            // nextname: Not a symbolic link. If this was not the last component, then
            // continue at the next component, else return.
            if ndp.ni_cnd.cn_flags & ISLASTCN == 0 {
                name = ndp.ni_next;
                ndp.ni_cnd.set_name_at(name, 0);
                if let Some(dvp) = ndp.ni_dvp {
                    vrele(dvp);
                }
                continue;
            }
            break;
        }
    }

    // terminal:
    // __pledge_open() only opens regular files in /usr/share/zoneinfo
    if ndp.ni_cnd.cn_flags & BPU_ZONEINFO != 0 && st.dp.vtype() != VREG {
        return Err(Bad2(Errno::EACCES));
    }
    // Check for read-only file systems.
    if ndp.ni_cnd.cn_nameiop == DELETE || ndp.ni_cnd.cn_nameiop == RENAME {
        // Disallow directory write attempts on read-only file systems.
        if rdonly || mnt_rdonly(Some(st.dp)) || (wantparent && mnt_rdonly(ndp.ni_dvp)) {
            return Err(Bad2(Errno::EROFS));
        }
    }
    if let Some(dvp) = ndp.ni_dvp {
        if ndp.ni_cnd.cn_flags & SAVESTART != 0 {
            ndp.ni_startdir = Some(dvp);
            vref(dvp);
        }
        if !wantparent {
            vrele(dvp);
        }
    }
    if ndp.ni_cnd.cn_flags & LOCKLEAF == 0 {
        let _ = vop_unlock(st.dp);
    }
    Ok(())
}

/// Whether the vnode's file system is mounted read-only.
fn mnt_rdonly<G: ProcGlue>(vp: Option<&'static Vnode<G>>) -> bool {
    vp.and_then(Vnode::mount)
        .is_some_and(|mp| mp.flag() & hal::sys::mount::MNT_RDONLY != 0)
}

/// Reacquire a path name component.
pub fn vfs_relookup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Componentname<'_, G>,
) -> Result<(), Errno> {
    // Setup: break out flag bits into variables.
    let wantparent = cnp.cn_flags & (LOCKPARENT | WANTPARENT) != 0;
    let rdonly = cnp.cn_flags & RDONLY != 0;
    cnp.cn_flags &= !ISSYMLINK;
    let dp = dvp;
    let _ = vn_lock(dp, LK_EXCLUSIVE | LK_RETRY);

    // dirloop: Search a new directory.
    //
    // The last component of the filename is left accessible via cnp->cn_nameptr for callers
    // that need the name. Callers needing the name set the SAVENAME flag. When done, they
    // assume responsibility for freeing the pathname buffer.

    // NAMEI_DIAGNOSTIC: not configured.

    // Check for degenerate name (e.g. / or "") which is a way of talking about a directory,
    // e.g. like "/." or ".".
    if cnp.name().first().is_none_or(|&c| c == 0) {
        kpanic(format_args!("relookup: null name"));
    }

    if cnp.cn_flags & ISDOTDOT != 0 {
        kpanic(format_args!("relookup: lookup on dot-dot"));
    }

    // We now have a segment name to search for, and a directory to search.
    if let Err(error) = vop_lookup(dp, vpp, cnp) {
        #[cfg(feature = "diagnostic")]
        if vpp.is_some() {
            kpanic(format_args!("leaf should be empty"));
        }
        if error != Errno::EJUSTRETURN {
            vput(dp);
            *vpp = None;
            return Err(error);
        }
        // If creating and at end of pathname, then can consider allowing file to be created.
        if rdonly || mnt_rdonly(Some(dvp)) {
            vput(dp);
            *vpp = None;
            return Err(Errno::EROFS);
        }
        // ASSERT(dvp == ndp->ni_startdir)
        if cnp.cn_flags & SAVESTART != 0 {
            vref(dvp);
        }
        // We return with ni_vp NULL to indicate that the entry doesn't currently exist,
        // leaving a pointer to the (possibly locked) directory vnode in ndp->ni_dvp.
        return Ok(());
    }
    let Some(dp) = *vpp else {
        kpanic(format_args!("relookup: no vnode found"));
    };

    #[cfg(feature = "diagnostic")]
    if dp.vtype() == VLNK && cnp.cn_flags & FOLLOW != 0 {
        kpanic(format_args!("relookup: symlink found."));
    }

    // Check for read-only file systems.
    if (cnp.cn_nameiop == DELETE || cnp.cn_nameiop == RENAME)
        && (rdonly || mnt_rdonly(Some(dp)) || (wantparent && mnt_rdonly(Some(dvp))))
    {
        // Disallow directory write attempts on read-only file systems.
        if cnp.cn_flags & LOCKPARENT != 0 && cnp.cn_flags & ISLASTCN != 0 {
            let _ = vop_unlock(dvp);
        }
        vrele(dvp);
        vput(dp);
        *vpp = None;
        return Err(Errno::EROFS);
    }
    // ASSERT(dvp == ndp->ni_startdir)
    if cnp.cn_flags & SAVESTART != 0 {
        vref(dvp);
    }
    if !wantparent {
        vrele(dvp);
    }
    if cnp.cn_flags & LOCKLEAF == 0 {
        let _ = vop_unlock(dp);
    }
    Ok(())
}
/* </CODE> */
/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `namei`/`vfs_lookup` over the kernel's test file system: the splitting
    // of a path into components (leading, repeated and trailing slashes, `.` and `..`, the
    // root as a barrier, `..` across a mount), the locking and reference protocol of
    // `LOCKLEAF`/`LOCKPARENT`/`WANTPARENT`, `CREATE` of a missing last component, `REALPATH`,
    // the errors, the pledge and unveil hooks at the C's points (and `PLEDGE_UNVEIL`'s
    // `EJUSTRETURN`), and that every lookup leaves the use counts as it found them. The
    // symbolic links are `bsd`'s tests over its glue (the test glue's `uiomove` cannot fill
    // the link's buffer, `fs::testfs`'s notes).

    use std::string::String;
    use std::vec::Vec;

    use super::*;
    use crate::fs::namei::{CREATE, LOOKUP, MAXPATHLEN, Nameidata, SAVENAME};
    use crate::fs::testfs::{
        self, A, B, C, D, LONG, N, NAMES, ROOT, TestGlue, any_locked, node_of, test_thread,
        usecounts,
    };
    use crate::fs::vops::{vop_abortop, vop_islocked};
    use crate::testutil::{file_log, set_namei_pledge, set_unveil_final};

    type G = TestGlue;

    /// `namei` of a kernel path.
    fn lookup(
        p: &'static Thread<G>,
        path: &[u8],
        op: u64,
        flags: u64,
    ) -> Result<Nameidata<'static, G>, Errno> {
        let path: &'static [u8] = std::boxed::Box::leak(path.to_vec().into_boxed_slice());
        let mut nd = ndinit(op, flags, NiDirp::Sys(path), p);
        namei(&mut nd).map(|()| nd)
    }

    /// The node a vnode stands for.
    fn idx(vp: &Vnode<G>) -> usize {
        node_of(vp).map_or(usize::MAX, |n| n.idx)
    }

    /// Looks `path` up with `LOOKUP` and `flags`, checks it found node `want`, releases it and
    /// checks nothing leaked.
    fn finds(p: &'static Thread<G>, path: &[u8], flags: u64, want: usize) {
        let before = usecounts();
        let nd = lookup(p, path, LOOKUP, flags).unwrap_or_else(|e| {
            panic!("{:?}: error {e:?}", core::str::from_utf8(path));
        });
        let vp = nd.ni_vp.unwrap();
        assert_eq!(idx(vp), want, "{:?}", core::str::from_utf8(path));
        if flags & LOCKLEAF != 0 {
            assert_ne!(vop_islocked(vp), 0);
            vput(vp);
        } else {
            assert!(
                !any_locked(),
                "{:?} left a lock",
                core::str::from_utf8(path)
            );
            vrele(vp);
        }
        assert!(!any_locked());
        assert!(nd.ni_cnd.pnbuf().is_none(), "the buffer went back");
        // Nodes looked up for the first time now have a vnode on the free list (count 0).
        let after = usecounts();
        for i in 0..N {
            assert_eq!(after[i], before[i], "use count of node {i} after {path:?}");
        }
    }

    #[test]
    fn absolute_and_relative_paths() {
        let (_g, p) = test_thread();
        finds(p, b"/", FOLLOW, ROOT);
        finds(p, b"/a", FOLLOW, A);
        finds(p, b"/a/b", FOLLOW, B);
        finds(p, b"/a/b", FOLLOW | LOCKLEAF, B);
        finds(p, b"a/c", FOLLOW, C);
        finds(p, b"./a/./c/.", FOLLOW, C);
        finds(p, b"/a/c/d", FOLLOW | LOCKLEAF, D);
    }

    #[test]
    fn slashes_split_components() {
        let (_g, p) = test_thread();
        finds(p, b"//a///c", FOLLOW, C);
        finds(p, b"/a/c/", FOLLOW, C);
        finds(p, b"/a/c//", FOLLOW | LOCKLEAF, C);
        // A trailing slash requires a directory.
        assert_eq!(
            lookup(p, b"/a/b/", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOTDIR)
        );
        // A component under a file.
        assert_eq!(
            lookup(p, b"/a/b/x", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOTDIR)
        );
        assert!(!any_locked());
    }

    #[test]
    fn dot_dot_climbs_and_stops_at_the_root() {
        let (_g, p) = test_thread();
        finds(p, b"/a/c/..", FOLLOW, A);
        finds(p, b"/a/../a/b", FOLLOW, B);
        finds(p, b"/..", FOLLOW, ROOT);
        finds(p, b"../../a", FOLLOW, A);
        finds(p, b"/a/c/../../a/c", FOLLOW | LOCKLEAF, C);
    }

    #[test]
    fn dot_dot_crosses_back_over_a_mount() {
        let (_g, p) = test_thread();
        let mp = crate::fs::init::rootvnode::<G>()
            .and_then(Vnode::mount)
            .unwrap();
        let cvp = testfs::vget_node(mp, C).unwrap();
        let _ = vop_unlock(cvp);
        let mp2 = testfs::mount_on(cvp);
        // /a/c is the second mount's root: its d is that file system's /a/c/d, absent.
        let nd = lookup(p, b"/a/c/a/b", LOOKUP, FOLLOW).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert!(vp.mount().is_some_and(|m| ptr::eq(m, mp2)) && idx(vp) == B);
        vrele(vp);
        // `..` of the mounted root is taken in the covered vnode's file system: /a.
        let nd = lookup(p, b"/a/c/..", LOOKUP, FOLLOW).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert!(vp.mount().is_some_and(|m| ptr::eq(m, mp)) && idx(vp) == A);
        vrele(vp);
        // NOCROSSMOUNT stays on the covered vnode.
        let nd = lookup(p, b"/a/c", LOOKUP, FOLLOW | NOCROSSMOUNT).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert!(ptr::eq(vp, cvp));
        vrele(vp);
        assert!(!any_locked());
        vrele(cvp);
    }

    #[test]
    fn errors() {
        let (_g, p) = test_thread();
        let before = usecounts();
        assert_eq!(lookup(p, b"", LOOKUP, FOLLOW).err(), Some(Errno::ENOENT));
        assert_eq!(
            lookup(p, b"/nope", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOENT)
        );
        assert_eq!(
            lookup(p, b"/a/nope/b", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOENT)
        );
        let long = [b'y'; 300];
        assert_eq!(
            lookup(p, &long, LOOKUP, FOLLOW).err(),
            Some(Errno::ENAMETOOLONG)
        );
        let toolong = [b'z'; MAXPATHLEN + 10];
        assert_eq!(
            lookup(p, &toolong, LOOKUP, FOLLOW).err(),
            Some(Errno::ENAMETOOLONG)
        );
        // A name longer than NAMECACHE_MAXLEN is looked up, not cached.
        finds(p, &[b"/a/", NAMES[LONG].0].concat(), FOLLOW, LONG);
        // A user path at an address that is not mapped.
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::User(0x10), p);
        assert_eq!(namei(&mut nd), Err(Errno::EFAULT));
        assert!(!any_locked());
        assert_eq!(usecounts()[ROOT], before[ROOT]);
    }

    #[test]
    fn lockparent_returns_both_vnodes_locked() {
        let (_g, p) = test_thread();
        let before = usecounts();
        let nd = lookup(p, b"/a/b", LOOKUP, LOCKPARENT | LOCKLEAF).unwrap();
        let (vp, dvp) = (nd.ni_vp.unwrap(), nd.ni_dvp.unwrap());
        assert_eq!((idx(dvp), idx(vp)), (A, B));
        assert!(vop_islocked(dvp) != 0 && vop_islocked(vp) != 0);
        vput(vp);
        vput(dvp);
        assert!(!any_locked());
        assert_eq!(usecounts()[ROOT], before[ROOT]);

        // WANTPARENT: the parent comes back referenced but unlocked.
        let nd = lookup(p, b"/a/c", LOOKUP, WANTPARENT).unwrap();
        let (vp, dvp) = (nd.ni_vp.unwrap(), nd.ni_dvp.unwrap());
        assert_eq!((idx(dvp), idx(vp)), (A, C));
        assert!(!any_locked());
        vrele(vp);
        vrele(dvp);
    }

    #[test]
    fn create_of_a_missing_name_returns_the_locked_parent() {
        let (_g, p) = test_thread();
        let before = usecounts();
        let mut nd = lookup(p, b"/a/new", CREATE, LOCKPARENT | SAVENAME).unwrap();
        assert!(nd.ni_vp.is_none());
        let dvp = nd.ni_dvp.unwrap();
        assert_eq!(idx(dvp), A);
        assert_ne!(vop_islocked(dvp), 0);
        // SAVENAME keeps the pathname buffer and the last component for the caller.
        assert!(nd.ni_cnd.cn_flags & HASBUF != 0);
        assert_eq!(nd.ni_cnd.name(), b"new");
        assert_eq!(nd.ni_cnd.name_off(), Some(3));
        let _ = vop_abortop(dvp, &mut nd.ni_cnd);
        assert!(nd.ni_cnd.pnbuf().is_none(), "abortop gives the buffer back");
        vput(dvp);
        assert!(!any_locked());
        assert_eq!(usecounts()[A], before[A]);

        // A missing intermediate directory is an error, not a creation.
        assert_eq!(
            lookup(p, b"/nope/new", CREATE, LOCKPARENT).err(),
            Some(Errno::ENOENT)
        );
        // Neither is a trailing slash on a missing name.
        assert_eq!(
            lookup(p, b"/a/new/", CREATE, LOCKPARENT).err(),
            Some(Errno::ENOENT)
        );
        // The root has no parent to create in.
        assert_eq!(
            lookup(p, b"/", CREATE, LOCKPARENT).err(),
            Some(Errno::EISDIR)
        );
        // A directory that refuses the creation (EACCES): an error for a lookup...
        assert_eq!(
            lookup(p, b"/a/c/d/new", CREATE, LOCKPARENT).err(),
            Some(Errno::EACCES)
        );
        // ...that unveil(2) turns into "not there yet" (EJUSTRETURN), and only unveil.
        let path: &'static [u8] = b"/a/c/d/new";
        let mut nd = ndinit(CREATE, LOCKPARENT, NiDirp::Sys(path), p);
        nd.ni_pledge = PLEDGE_UNVEIL;
        namei(&mut nd).unwrap();
        assert!(nd.ni_vp.is_none());
        let dvp = nd.ni_dvp.unwrap();
        assert_eq!(idx(dvp), D);
        vput(dvp);
        assert!(!any_locked());
    }

    #[test]
    fn realpath_builds_the_canonical_name() {
        let (_g, p) = test_thread();
        let path: &'static [u8] = b"/a/./c/../c/d/..";
        let mut nd = ndinit(LOOKUP, FOLLOW | SAVENAME | REALPATH, NiDirp::Sys(path), p);
        nd.ni_cnd.cn_rpbuf = Some(pathbuf_get::<G>().unwrap());
        nd.ni_cnd.cn_rpi = 0;
        namei(&mut nd).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert_eq!(idx(vp), C);
        let rp = nd.ni_cnd.rpbuf().unwrap();
        let len = rp.iter().position(|&c| c == 0).unwrap();
        assert_eq!(&rp[..len], b"/a/c");
        assert_eq!(nd.ni_cnd.cn_rpi, 4);
        vrele(vp);
    }

    #[test]
    fn stripslashes_drops_the_trailing_slashes_first() {
        let (_g, p) = test_thread();
        let nd = lookup(p, b"/a/b///", LOOKUP, FOLLOW | STRIPSLASHES).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert_eq!(idx(vp), B);
        vrele(vp);
    }

    #[test]
    fn no_root_file_system_is_enoent() {
        let (_g, p) = test_thread();
        let root = crate::fs::init::rootvnode::<G>();
        crate::fs::init::set_rootvnode::<G>(None);
        let cdir = p.fd().replace_cdir(None);
        assert_eq!(lookup(p, b"/a", LOOKUP, FOLLOW).err(), Some(Errno::ENOENT));
        assert_eq!(lookup(p, b"a", LOOKUP, FOLLOW).err(), Some(Errno::ENOENT));
        crate::fs::init::set_rootvnode::<G>(root);
        p.fd().set_cdir(cdir);
    }

    #[test]
    fn component_push_and_pop() {
        let (_g, p) = test_thread();
        let mut cn = Componentname::new(LOOKUP, 0, p, Cred::NULL);
        cn.cn_rpbuf = Some(pathbuf_get::<G>().unwrap());
        cn.cn_rpbuf.as_deref_mut().unwrap()[0] = b'/';
        cn.cn_rpi = 1;
        assert!(component_push(&mut cn, b"usr"));
        assert!(component_push(&mut cn, b"share"));
        assert_eq!(&cn.rpbuf().unwrap()[..cn.cn_rpi + 1], b"/usr/share\0");
        component_pop(&mut cn);
        assert_eq!(&cn.rpbuf().unwrap()[..cn.cn_rpi + 1], b"/usr\0");
        component_pop(&mut cn);
        assert_eq!(&cn.rpbuf().unwrap()[..cn.cn_rpi + 1], b"/\0");
        // A component that would not fit is refused.
        cn.cn_rpi = MAXPATHLEN - 3;
        assert!(!component_push(&mut cn, b"xy"));
    }

    /// The hooks' calls this thread logged since the last read, `pledge_namei`'s path cut.
    fn hooks() -> Vec<String> {
        file_log()
            .into_iter()
            .filter(|l| l.starts_with("pledge_namei") || l.starts_with("unveil"))
            .collect()
    }

    #[test]
    fn the_pledge_and_unveil_hooks_run_at_the_cs_points() {
        let (_g, p) = test_thread();
        let _ = file_log();
        // A relative path: pledge, the start (relative, then the start directory's
        // component), one check per component, the final check.
        finds(p, b"a/c", FOLLOW, C);
        assert_eq!(
            hooks(),
            [
                "pledge_namei a/c",
                "unveil_start_relative",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_final"
            ]
        );
        // An absolute path: no start of its own; `..` checks before and after its climb.
        finds(p, b"/a/..", FOLLOW, ROOT);
        assert_eq!(
            hooks(),
            [
                "pledge_namei /a/..",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_final"
            ]
        );
        // KERNELPATH: no pledge check, BYPASSUNVEIL set for the unveil hooks.
        let path: &'static [u8] = b"/a";
        let mut nd = ndinit(LOOKUP, FOLLOW | KERNELPATH, NiDirp::Sys(path), p);
        namei(&mut nd).unwrap();
        assert!(nd.ni_cnd.cn_flags & BYPASSUNVEIL != 0);
        assert!(!hooks().iter().any(|h| h.starts_with("pledge_namei")));
        vrele(nd.ni_vp.unwrap());
    }

    /// A symbolic link (vfs_lookup.c:279-345): an absolute target restarts the walk at the
    /// root with the unveil match cleared and `unveil_check_component` at the root before the
    /// target's first component; a relative one goes on in the link's directory with the
    /// match it had; REALPATH's buffer starts over at the root for an absolute target. The
    /// test glue counts the match (`unveil_check_component` adds one, the C's NULL is 0) and
    /// copies the link's bytes (`set_copy_kernel`).
    #[test]
    fn a_symbolic_link_restarts_at_the_root_with_its_unveil_check() {
        let (_g, p) = test_thread();
        crate::testutil::set_copy_kernel(true);
        let _ = file_log();
        // /a/abs -> /f: `a`, `abs`, then the root (the match cleared first), then `f`.
        finds(p, b"/a/abs", FOLLOW, testfs::F);
        assert_eq!(
            hooks(),
            [
                "pledge_namei /a/abs",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_final"
            ]
        );
        assert_eq!(
            crate::testutil::unveil_final_match(),
            Some(2),
            "cleared at the root"
        );
        // /lnk -> a/c: `lnk`, then `a` and `c` from the link's directory, no clearing.
        finds(p, b"/lnk", FOLLOW, C);
        assert_eq!(
            hooks(),
            [
                "pledge_namei /lnk",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_component",
                "unveil_check_final"
            ]
        );
        assert_eq!(crate::testutil::unveil_final_match(), Some(3), "kept");
        // Without FOLLOW the last component is the link itself.
        finds(p, b"/a/abs", 0, testfs::ABS);
        let _ = file_log();
        // REALPATH: an absolute target's name starts over at the root.
        let path: &'static [u8] = b"/a/abs";
        let mut nd = ndinit(LOOKUP, FOLLOW | SAVENAME | REALPATH, NiDirp::Sys(path), p);
        nd.ni_cnd.cn_rpbuf = Some(pathbuf_get::<G>().unwrap());
        nd.ni_cnd.cn_rpi = 0;
        namei(&mut nd).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert_eq!(idx(vp), testfs::F);
        let rp = nd.ni_cnd.rpbuf().unwrap();
        let len = rp.iter().position(|&c| c == 0).unwrap();
        assert_eq!(&rp[..len], b"/f");
        vrele(vp);
        crate::testutil::set_copy_kernel(false);
        assert!(!any_locked());
    }

    #[test]
    fn a_refusing_hook_fails_the_lookup_and_releases_everything() {
        let (_g, p) = test_thread();
        let before = usecounts();
        set_namei_pledge(Some(Errno::EPERM));
        assert_eq!(lookup(p, b"/a/b", LOOKUP, FOLLOW).err(), Some(Errno::EPERM));
        set_namei_pledge(None);
        set_unveil_final(Some(Errno::ENOENT));
        assert_eq!(
            lookup(p, b"/a/b", LOOKUP, LOCKPARENT | LOCKLEAF).err(),
            Some(Errno::ENOENT)
        );
        assert_eq!(
            lookup(p, b"/a/c", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOENT)
        );
        set_unveil_final(None);
        assert!(!any_locked());
        let after = usecounts();
        for i in 0..N {
            assert_eq!(after[i], before[i], "use count of node {i}");
        }
    }

    #[test]
    fn a_lookup_relative_to_a_descriptor() {
        let (_g, p) = test_thread();
        let mp = crate::fs::init::rootvnode::<G>()
            .and_then(Vnode::mount)
            .unwrap();
        let avp = testfs::vget_node(mp, A).unwrap();
        let _ = vop_unlock(avp);
        let bvp = testfs::vget_node(mp, B).unwrap();
        let _ = vop_unlock(bvp);
        let mut g = crate::descrip::fdplock(p.fd());
        let (fp, fd) = crate::descrip::falloc(p, &mut g).unwrap();
        fp.install(
            crate::file::DTYPE_VNODE,
            &crate::fs::vnops::VnodeFileOps,
            crate::testutil::TestFileData {
                kvnode: Some(avp),
                ..Default::default()
            },
        );
        crate::descrip::fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        let (fp2, fd2) = {
            let mut g = crate::descrip::fdplock(p.fd());
            let (fp2, fd2) = crate::descrip::falloc(p, &mut g).unwrap();
            fp2.install(
                crate::file::DTYPE_VNODE,
                &crate::fs::vnops::VnodeFileOps,
                crate::testutil::TestFileData {
                    kvnode: Some(bvp),
                    ..Default::default()
                },
            );
            crate::descrip::fdinsert(&mut g, fd2, 0, &fp2);
            (fp2, fd2)
        };
        let path: &'static [u8] = b"c";
        let mut nd = ndinitat(LOOKUP, FOLLOW, fd, NiDirp::Sys(path), p);
        namei(&mut nd).unwrap();
        assert_eq!(idx(nd.ni_vp.unwrap()), C);
        vrele(nd.ni_vp.unwrap());
        // A descriptor of a file, and one that is not open.
        let mut nd = ndinitat(LOOKUP, FOLLOW, fd2, NiDirp::Sys(path), p);
        assert_eq!(namei(&mut nd), Err(Errno::ENOTDIR));
        let mut nd = ndinitat(LOOKUP, FOLLOW, 99, NiDirp::Sys(path), p);
        assert_eq!(namei(&mut nd), Err(Errno::EBADF));
        drop((fp, fp2));
        assert!(!any_locked());
    }
}
/* </TESTS> */
