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
 * Copyright (c) 2005, 2006 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Julio M. Merino Vidal, developed as part of Google's Summer of Code
 * 2005 program.
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

/*
 * Copyright (c) 2005, 2006, 2007, 2012 The NetBSD Foundation, Inc.
 * Copyright (c) 2013 Pedro Martelletto
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Julio M. Merino Vidal, developed as part of Google's Summer of Code
 * 2005 program, and by Taylor R Campbell.
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
//! `<tmpfs/tmpfs_vnops.h>` / `tmpfs_vnops.c`: the tmpfs vnode interface: the operations
//! vector used for files stored in a tmpfs file system (`tmpfs_vops`, [`TmpfsVnops`]), its
//! operations, the rename machinery and the kqueue filters.
//!
//! Upstream: sys/tmpfs/tmpfs_vnops.h @ 3ce1f3f79392
//! Upstream: sys/tmpfs/tmpfs_vnops.c @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs_vnops.rs@3c62ede68802
//!
//! The declarations of `tmpfs_vnops.h` are the `pub` items here and in `tmpfs_specops.rs`
//! and `tmpfs_fifoops.rs`.
//!
//! # The vnode lock
//!
//! A tmpfs vnode's lock is its node's `tn_vlock`, a recursive rwlock (`rrwlock`), as ufs's
//! is its inode's `i_lock`: `tmpfs_lock` is `rrw_enter(&tn_vlock, a_flags & LK_RWFLAGS)`.
//! A thread that holds it exclusively and asks for it again gets it again (the count goes
//! up and `tmpfs_unlock` takes it down; the last unlock releases it), unless it passes
//! `LK_RECURSEFAIL`, which fails with `EDEADLK`. vnd(4) relies on this: `VNDIOCSET` reads
//! the backing file through `vn_rdwr` (which `vn_lock`s it) while the lock `vn_open`
//! returned it with is still held.
//!
//! ## Deviations
//! - `tmpfs_print`'s `fifo_printinfo` (`option FIFO`) waits for `miscfs/fifofs` (as fifo
//!   nodes do: `tmpfs_vnode_get` refuses them, see `tmpfs_subr.rs`) (LZ's).
//! - `tmpfs_lookup` passes on an error of `cache_lookup` other than `ENOENT` (it failed to
//!   lock the directory again); the C returns 0 there without a vnode, which its caller
//!   would follow as a NULL pointer (LZ's).
//! - Credentials the C dereferences (`cred->cr_uid`, the `tmpfs_ch*` calls of
//!   `tmpfs_setattr`) go through `ucred`, which panics on `NOCRED`/`FSCRED`, where the C
//!   would follow a bad pointer. `KASSERT`s on pointers the C then dereferences (an entry's
//!   node, a node's vnode, the source entry `tmpfs_rename_lock` found) are panics too (LZ's).
//! - `tmpfs_readlink` of a kernel uio (`UIO_SYSSPACE`: `namei`'s symbolic link, the NFS
//!   server's) copies the target to a `MAXPATHLEN` buffer on its stack for `bsd`'s `uiomove`,
//!   which takes a mutable slice (LZ's; D-N5.9-17); a user uio (`readlink(2)`) takes it
//!   straight from the link's block, as the C's `uiomove` does.
//! - The rename helpers return what the C stores through out-parameters
//!   (`RenameEntered`, `RenameLocked`, the intermediate node); their `goto fail<n>`
//!   ladders are nested labelled blocks (LZ's).
//! - `tmpfs_strategy`, `tmpfs_ioctl` and `tmpfs_bwrite` take no arguments (LZ's).
//!
//! ## Redesign
//! - LZ's `TMPFS_VOPS` (a `bsd` `Vops` of function pointers over argument structures) ->
//!   [`TmpfsVnops`], one `impl Vnops<G>` of the C's 34 slots (decision 46), a member of the
//!   glue's `TmpfsTables` (one address per glue, for `vops_id`; a byte, so that it is not
//!   zero-sized); the nine slots the three tables share are one macro
//!   (`tmpfs_common_slots!`, cd9660's shape). The operations are the C's functions over
//!   the kernel's arguments, generic over the glue; `curproc` is the operation's thread
//!   (`a_p`, `cn_proc`, `uio_procp`).
//! - The node is `vptonode`'s counted handle (`tmpfs.rs`), an entry `tmpfs_dir_lookup`'s;
//!   where LZ followed `&'static` pool items (`td_node`, `tn_vnode`, raw pointers made
//!   references by convention) the rename helpers hold handles across their sleeps, so no
//!   node or entry can go under them; a node's vnode is its `Link`, read as the C reads it.
//!   `tmpfs_rename_genealogy`'s walk up `tn_parent` takes one handle per level (O(depth), a
//!   count pair a step: the C's pointer walk plus the count, ROADMAP N12's line of
//!   `vptonode`).
//! - The three `filterops` are `impl Filter<G>` for [`TmpfsRead`], [`TmpfsWrite`] and
//!   [`TmpfsVnode`] with their tables `TmpfsFiltops::<G>::READ`/`WRITE`/`VNODE` (cd9660's
//!   shape); a knote's vnode is its typed hook (`set_hook_vnode`, `hook_vnode`) where LZ cast
//!   `kn_hook` back (one `unsafe`); `filt_tmpfsread` reads the node after its `NOTE_REVOKE`
//!   test, where the C dereferences it (a reclaimed vnode has none).
//! - `tmpfs_rename`'s new name is a fresh `tmpfs_strname_alloc` block filled where the C
//!   `memcpy`s it, the old one freed through `tmpfs_strname_free` (LZ's raw copy and
//!   pointer swap: one `unsafe`). 0 `unsafe` (LZ: 3).

use core::marker::PhantomData;
use core::ptr;

use hal::arch::current::param::PAGE_SIZE;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FWRITE, Flock, O_APPEND};
use hal::sys::mount::{MNT_NOATIME, MNT_RDONLY};
use hal::sys::stat::{APPEND, IMMUTABLE, S_ISTXT};
use hal::sys::syslimits::LINK_MAX;
use hal::sys::types::{Daddr, Gid, Mode, Nlink, Off, Register, Uid};
use hal::sys::unistd::{
    _PC_CHOWN_RESTRICTED, _PC_FILESIZEBITS, _PC_LINK_MAX, _PC_NAME_MAX, _PC_NO_TRUNC,
    _PC_TIMESTAMP_RESOLUTION,
};
use hal::vm::param::round_page;

use crate::assert::{kassert, kpanic};
use crate::clock::Timespec;
use crate::event::{
    __EV_POLL, __EV_SELECT, EV_EOF, EV_ONESHOT, EVFILT_READ, EVFILT_VNODE, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_EOF, NOTE_EXTEND, NOTE_LINK, NOTE_RENAME,
    NOTE_REVOKE, NOTE_WRITE,
};
use crate::file::{FileGlue, LockOwner, foffset};
use crate::fs::cache::{cache_enter, cache_lookup, cache_purge};
use crate::fs::default;
use crate::fs::lockf::lf_advlock;
use crate::fs::mount::Mount;
use crate::fs::namei::{
    CREATE, DELETE, HASBUF, ISDOTDOT, ISLASTCN, LOCKPARENT, MAKEENTRY, MAXPATHLEN, PDIRUNLOCK,
    RENAME, SAVENAME,
};
use crate::fs::subr::{vaccess, vattr_null, vput, vrecycle, vref, vrele};
use crate::fs::tmpfs::tmpfs::{
    TMPFS_DIRSEQ_EOF, TMPFS_MAXNAMLEN, TMPFS_NODE_ACCESSED, TMPFS_NODE_CHANGED,
    TMPFS_NODE_MODIFIED, TMPFS_NODE_STATUSALL, TmpfsDirentRef, TmpfsMount, TmpfsNode, TmpfsNodeRef,
    tmpfs_dirseq_full, tmpfs_node_gen, tmpfs_node_reclaiming, vfstotmpfs, vptodir, vptonode,
};
use crate::fs::tmpfs::tmpfs_mem::{tmpfs_strname_alloc, tmpfs_strname_free, tmpfs_strname_neqlen};
use crate::fs::tmpfs::tmpfs_subr::{
    tmpfs_alloc_dirent, tmpfs_alloc_file, tmpfs_chflags, tmpfs_chmod, tmpfs_chown, tmpfs_chsize,
    tmpfs_chtimes, tmpfs_dir_attach, tmpfs_dir_cached, tmpfs_dir_detach, tmpfs_dir_getdents,
    tmpfs_dir_lookup, tmpfs_free_dirent, tmpfs_free_node, tmpfs_reg_resize, tmpfs_uio_cached,
    tmpfs_uio_uncache, tmpfs_uiomove, tmpfs_update, tmpfs_vnode_get,
};
use crate::fs::ufs::ufs::ufs_vnops::ucred;
use crate::fs::vnode::{
    IO_APPEND, LK_EXCLUSIVE, LK_RETRY, LK_RWFLAGS, VA_UTIMES_CHANGE, VBLK, VCHR, VDIR, VEXEC,
    VFIFO, VLNK, VN_KNOTE, VNON, VNOVAL, VREG, VSOCK, VWRITE, Vattr, Vnode, vn_lock,
};
use crate::fs::vnops::vn_fsizechk;
use crate::fs::vops::{Cn, Cred, IoBuf, Vnops, vop_abortop, vop_access, vop_islocked, vop_unlock};
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Uio, UioRw, UioSeg, uiomove_user_out};
use crate::vm::vnode::uvm_vnp_uncache;

/// `tmpfs_vops`: vnode operations vector used for files stored in a tmpfs file system (the
/// glue's `TmpfsTables::vops`, one address).
pub struct TmpfsVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> TmpfsVnops<G> {
    /// The table (the glue's `TmpfsTables::vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// What `tmpfs_rename_enter` looked up and locked: the C's `*fde_ret`, `*fvp_ret`,
/// `*tde_ret` and `*tvp_ret`.
struct RenameEntered<G: ProcGlue> {
    /// The source entry.
    fde: TmpfsDirentRef<G>,
    /// The source vnode, locked and referenced.
    fvp: &'static Vnode<G>,
    /// The target entry, when the target exists.
    tde: Option<TmpfsDirentRef<G>>,
    /// The target vnode, locked and referenced, when the target exists.
    tvp: Option<&'static Vnode<G>>,
}

/// What `tmpfs_rename_lock` looked up and locked: the C's `*a_dirent_ret`, `*a_vp_ret`,
/// `*b_dirent_ret` and `*b_vp_ret`.
struct RenameLocked<G: ProcGlue> {
    /// The entry of `a_cnp` in directory a, if any.
    a_dirent: Option<TmpfsDirentRef<G>>,
    /// Its vnode, locked and referenced.
    a_vp: Option<&'static Vnode<G>>,
    /// The entry of `b_cnp` in directory b, if any.
    b_dirent: Option<TmpfsDirentRef<G>>,
    /// Its vnode, locked and referenced (`a_vp` once more when both name one node).
    b_vp: Option<&'static Vnode<G>>,
}

/// `tmpfsread_filtops`'s filter: the bytes past the file offset.
pub struct TmpfsRead;

/// `tmpfswrite_filtops`'s filter: a file is always writable.
pub struct TmpfsWrite;

/// `tmpfsvnode_filtops`'s filter: the vnode events (`NOTE_*`) the user asked for.
pub struct TmpfsVnode;

/// The tables of the three filters (`kn_fop`'s targets), made at compile time.
pub(super) struct TmpfsFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> TmpfsFiltops<G> {
    /// `tmpfsread_filtops`.
    pub(super) const READ: Filterops<G> = Filterops::of::<TmpfsRead>();
    /// `tmpfswrite_filtops`.
    pub(super) const WRITE: Filterops<G> = Filterops::of::<TmpfsWrite>();
    /// `tmpfsvnode_filtops`.
    pub(super) const VNODE: Filterops<G> = Filterops::of::<TmpfsVnode>();
}

/// The vnode's mount, which a tmpfs vnode always has.
fn vmount<G: ProcGlue>(vp: &Vnode<G>) -> &'static Mount<G> {
    match vp.mount() {
        Some(mp) => mp,
        None => kpanic(format_args!("tmpfs: vnode {vp:p} without a mount")),
    }
}

/// `node->tn_vnode` where the C dereferences it: a node the rename locked has its vnode.
fn tn_vnode<G: ProcGlue>(node: &TmpfsNode<G>) -> &'static Vnode<G> {
    match node.vnode() {
        Some(vp) => vp,
        None => kpanic(format_args!("tmpfs: node {node:p} without a vnode")),
    }
}

/// The C's `a == b` on two pointers that may be NULL (vnodes, mounts).
fn same<T>(a: Option<&T>, b: Option<&T>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// The C's `a == b` on a node and a node pointer that may be NULL.
fn is_node<G: ProcGlue>(a: &TmpfsNode<G>, b: Option<&TmpfsNodeRef<G>>) -> bool {
    b.is_some_and(|b| ptr::eq(a, PoolArc::as_ptr(b)))
}

/// Whether `cnp` names `.` or `..`.
fn is_dot_or_dotdot<G: ProcGlue>(cnp: &Cn<'_, G>) -> bool {
    let name = cnp.name();
    name == b"." || name == b".."
}

/// `tmpfs_lookup` (`vop_lookup`): path name traversal routine.
///
/// Arguments: `dvp` (directory being searched), `vpp` (result), `cnp` (component name -
/// path).
///
/// - Caller holds a reference and lock on `dvp`.
/// - We return looked-up vnode (`vpp`) locked, with a reference held.
pub fn tmpfs_lookup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let cred = cnp.cn_cred;
    let p = cnp.cn_proc;
    let lastcn = cnp.cn_flags & ISLASTCN != 0;
    let lockparent = cnp.cn_flags & LOCKPARENT != 0;

    kassert!(vop_islocked(dvp) != 0);

    let dnode = vptodir(dvp);
    cnp.cn_flags &= !PDIRUNLOCK;
    *vpp = None;

    let error: Result<(), Errno> = 'out: {
        // Check accessibility of directory.
        if let Err(e) = vop_access(dvp, VEXEC, cred, p) {
            break 'out Err(e);
        }

        // If requesting the last path component on a read-only file system with a write
        // operation, deny it.
        if lastcn
            && vmount(dvp).flag() & MNT_RDONLY != 0
            && (cnp.cn_nameiop == DELETE || cnp.cn_nameiop == RENAME)
        {
            break 'out Err(Errno::EROFS);
        }

        // Avoid doing a linear scan of the directory if the requested directory/name couple
        // is already in the cache. A negative hit is ENOENT.
        if let Some(vp) = cache_lookup(dvp, cnp)? {
            *vpp = Some(vp);
            return Ok(()); // Found in cache.
        }

        let error: Result<(), Errno> = 'done: {
            if cnp.cn_flags & ISDOTDOT != 0 {
                // Lookup of ".." case.
                if lastcn {
                    if cnp.cn_nameiop == RENAME {
                        break 'out Err(Errno::EINVAL);
                    }
                    if cnp.cn_nameiop == DELETE {
                        // Keep the name for tmpfs_rmdir().
                        cnp.cn_flags |= SAVENAME;
                    }
                }
                kassert!(dnode.tn_type == VDIR);
                let Some(pnode) = dnode.dir().tn_parent.get() else {
                    break 'out Err(Errno::ENOENT);
                };

                // Lock the parent tn_nlock before releasing the vnode lock, and thus
                // prevents parent from disappearing.
                let nlock = pnode.tn_nlock.write();
                let _ = vop_unlock(dvp);

                // Get a vnode of the '..' entry and re-acquire the lock. Release the
                // tn_nlock.
                let error = tmpfs_vnode_get(vmount(dvp), &pnode, nlock).map(|vp| *vpp = Some(vp));
                let _ = vn_lock(dvp, LK_EXCLUSIVE | LK_RETRY);
                break 'out error;
            } else if cnp.name() == b"." {
                // Lookup of "." case.
                if lastcn && cnp.cn_nameiop == RENAME {
                    break 'out Err(Errno::EISDIR);
                }
                vref(dvp);
                *vpp = Some(dvp);
                break 'done Ok(());
            }

            // Other lookup cases: perform directory scan.
            let Some(de) = tmpfs_dir_lookup(&dnode, cnp) else {
                // The entry was not found in the directory. This is valid if we are
                // creating or renaming an entry and are working on the last component of
                // the path name.
                if lastcn && (cnp.cn_nameiop == CREATE || cnp.cn_nameiop == RENAME) {
                    if let Err(e) = vop_access(dvp, VWRITE, cred, p) {
                        break 'out Err(e);
                    }
                    // We are creating an entry in the file system, so save its name for
                    // further use by tmpfs_create().
                    cnp.cn_flags |= SAVENAME;
                    break 'done Err(Errno::EJUSTRETURN);
                }
                break 'done Err(Errno::ENOENT);
            };

            let tnode = de.node();
            drop(de);

            // If it is not the last path component and found a non-directory or non-link
            // entry (which may itself be pointing to a directory), raise an error.
            if !lastcn && tnode.tn_type != VDIR && tnode.tn_type != VLNK {
                break 'out Err(Errno::ENOTDIR);
            }

            // Check the permissions.
            if lastcn && (cnp.cn_nameiop == DELETE || cnp.cn_nameiop == RENAME) {
                if let Err(e) = vop_access(dvp, VWRITE, cred, p) {
                    break 'out Err(e);
                }

                // If not root and directory is sticky, check for permission on directory or
                // on file. This implements append-only directories.
                if dnode.tn_mode.get() & S_ISTXT != 0 {
                    let uid = ucred(cred).cr_uid.get();
                    if uid != 0 && uid != dnode.tn_uid.get() && uid != tnode.tn_uid.get() {
                        break 'out Err(Errno::EPERM);
                    }
                }

                // XXX pedro: We might need cn_nameptr later in tmpfs_remove() or
                // tmpfs_rmdir() for a tmpfs_dir_lookup(). We should really get rid of
                // SAVENAME at some point.
                if cnp.cn_nameiop == DELETE {
                    cnp.cn_flags |= SAVENAME;
                }
            }

            // Get a vnode for the matching entry.
            let nlock = tnode.tn_nlock.write();
            tmpfs_vnode_get(vmount(dvp), &tnode, nlock).map(|vp| *vpp = Some(vp))
        };
        // done:
        // Cache the result, unless request was for creation (as it does not improve the
        // performance).
        if cnp.cn_flags & MAKEENTRY != 0 && cnp.cn_nameiop != CREATE {
            cache_enter(dvp, *vpp, cnp.name());
        }
        error
    };

    // out:
    // If (1) we succeeded, (2) found a distinct vnode != .. to return and (3) were either
    // explicitly told to keep the parent locked or are in the middle of a lookup, unlock the
    // parent vnode.
    if matches!(error, Ok(()) | Err(Errno::EJUSTRETURN)) // (1)
        && (!same(*vpp, Some(dvp)) || cnp.cn_flags & ISDOTDOT != 0) // (2)
        && (!lockparent || !lastcn)
    // (3)
    {
        let _ = vop_unlock(dvp);
        cnp.cn_flags |= PDIRUNLOCK;
    } else {
        kassert!(vop_islocked(dvp) != 0);
    }

    kassert!(vpp.is_some_and(|vp| vop_islocked(vp) != 0) || error.is_err());

    error
}

/// `tmpfs_create` (`vop_create`): create a regular file (or a socket's node).
pub fn tmpfs_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    kassert!(vop_islocked(dvp) != 0);
    kassert!(cnp.cn_flags & HASBUF != 0);
    kassert!(vap.va_type == VREG || vap.va_type == VSOCK);
    let r = tmpfs_alloc_file(dvp, vap, cnp, None);
    *vpp = r.as_ref().ok().copied();
    r.map(|_| ())
}

/// `tmpfs_mknod` (`vop_mknod`): make a device special file or a fifo.
pub fn tmpfs_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let vt = vap.va_type;

    if vt != VBLK && vt != VCHR && vt != VFIFO {
        return Err(Errno::EINVAL);
    }

    let r = tmpfs_alloc_file(dvp, vap, cnp, None);
    *vpp = r.as_ref().ok().copied();
    let vp = r?;
    vput(vp);
    Ok(())
}

/// `tmpfs_open` (`vop_open`): refuse a file all of whose names are gone, and writes to an
/// append-only file that are not appends.
pub fn tmpfs_open<G: ProcGlue>(vp: &'static Vnode<G>, mode: i32) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);

    let node = vptonode(vp);
    if node.tn_links.get() < 1 {
        // The file is still active, but all its names have been removed (e.g. by a "rmdir
        // $(pwd)"). It cannot be opened any more, as it is about to be destroyed.
        return Err(Errno::ENOENT);
    }

    // If the file is marked append-only, deny write requests.
    if node.tn_flags.get() & APPEND != 0 && mode & (FWRITE | O_APPEND) == FWRITE {
        return Err(Errno::EPERM);
    }
    Ok(())
}

/// `tmpfs_close` (`vop_close`): nothing to do.
pub fn tmpfs_close<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    // DIAGNOSTIC:
    kassert!(vop_islocked(vp) != 0);
    Ok(())
}

/// `tmpfs_access` (`vop_access`).
pub fn tmpfs_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let node = vptonode(vp);
    let writing = mode & VWRITE != 0;

    kassert!(vop_islocked(vp) != 0);

    // Possible?
    match vp.vtype() {
        VDIR | VLNK | VREG => {
            if writing && vmount(vp).flag() & MNT_RDONLY != 0 {
                return Err(Errno::EROFS);
            }
        }
        VBLK | VCHR | VSOCK | VFIFO => {}
        _ => return Err(Errno::EINVAL),
    }
    if writing && node.tn_flags.get() & IMMUTABLE != 0 {
        return Err(Errno::EPERM);
    }

    vaccess(
        vp.vtype(),
        node.tn_mode.get(),
        node.tn_uid.get(),
        node.tn_gid.get(),
        mode,
        ucred(cred),
    )
}

/// `tmpfs_getattr` (`vop_getattr`).
pub fn tmpfs_getattr<G: ProcGlue>(vp: &'static Vnode<G>, vap: &mut Vattr) -> Result<(), Errno> {
    let node = vptonode(vp);

    vattr_null(vap);

    vap.va_type = vp.vtype();
    vap.va_mode = node.tn_mode.get();
    vap.va_nlink = node.tn_links.get();
    vap.va_uid = node.tn_uid.get();
    vap.va_gid = node.tn_gid.get();
    vap.va_fsid = i64::from(vmount(vp).fsid().val[0]);
    vap.va_fileid = node.tn_id.get();
    vap.va_size = node.tn_size.get() as u64;
    vap.va_blocksize = PAGE_SIZE as i64;
    vap.va_atime = node.tn_atime.get();
    vap.va_mtime = node.tn_mtime.get();
    vap.va_ctime = node.tn_ctime.get();
    // vap->va_birthtime = node->tn_birthtime;
    vap.va_gen = tmpfs_node_gen(&node);
    vap.va_flags = u64::from(node.tn_flags.get());
    vap.va_rdev = if vp.vtype() == VBLK || vp.vtype() == VCHR {
        node.rdev()
    } else {
        VNOVAL as hal::sys::types::Dev
    };
    vap.va_bytes = round_page(node.tn_size.get() as usize) as u64;
    vap.va_filerev = VNOVAL as u64;
    vap.va_vaflags = 0;
    vap.va_spare = i64::from(VNOVAL); // XXX

    Ok(())
}

/// `GOODTIME(tv)`: whether a time of a `vattr` is set.
fn goodtime(tv: &Timespec) -> bool {
    tv.tv_nsec != i64::from(VNOVAL)
}

/// `tmpfs_setattr` (`vop_setattr`).
///
/// XXX Should this operation be atomic? I think it should, but code in other places (e.g.,
/// ufs) doesn't seem to be...
pub fn tmpfs_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);

    // Abort if any unsettable attribute is given.
    if vap.va_type != VNON
        || vap.va_nlink != VNOVAL as Nlink
        || vap.va_fsid != i64::from(VNOVAL)
        || vap.va_fileid != VNOVAL as u64
        || vap.va_blocksize != i64::from(VNOVAL)
        || goodtime(&vap.va_ctime)
        || vap.va_gen != VNOVAL as u64
        || vap.va_rdev != VNOVAL as hal::sys::types::Dev
        || vap.va_bytes != VNOVAL as u64
    {
        return Err(Errno::EINVAL);
    }
    if vap.va_flags != VNOVAL as u64 {
        tmpfs_chflags(vp, vap.va_flags as u32, ucred(cred), p)?;
    }

    if vap.va_size != VNOVAL as u64 {
        tmpfs_chsize(vp, vap.va_size, ucred(cred), p)?;
    }

    if vap.va_uid != VNOVAL as Uid || vap.va_gid != VNOVAL as Gid {
        tmpfs_chown(vp, vap.va_uid, vap.va_gid, ucred(cred), p)?;
    }

    if vap.va_mode != VNOVAL as Mode {
        tmpfs_chmod(vp, vap.va_mode, ucred(cred), p)?;
    }

    if vap.va_vaflags & VA_UTIMES_CHANGE != 0 || goodtime(&vap.va_atime) || goodtime(&vap.va_mtime)
    {
        tmpfs_chtimes(
            vp,
            &vap.va_atime,
            &vap.va_mtime,
            vap.va_vaflags,
            ucred(cred),
            p,
        )?;
    }

    Ok(())
}

/// `tmpfs_read` (`vop_read`): read a regular file through its object.
pub fn tmpfs_read<G: ProcGlue>(vp: &'static Vnode<G>, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);

    if vp.vtype() != VREG {
        return Err(Errno::EISDIR);
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }

    let node = vptonode(vp);
    let mut error = Ok(());

    while error.is_ok() && uio.uio_resid > 0 {
        if node.tn_size.get() <= uio.uio_offset {
            break;
        }
        let len = (node.tn_size.get() - uio.uio_offset).min(uio.uio_resid as Off);
        if len == 0 {
            break;
        }
        error = tmpfs_uiomove(&node, uio, len as usize);
    }

    if vmount(vp).flag() & MNT_NOATIME == 0 {
        tmpfs_update(&node, TMPFS_NODE_ACCESSED);
    }

    error
}

/// `tmpfs_write` (`vop_write`): write a regular file through its object, growing it first
/// when the write goes past its end.
pub fn tmpfs_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);

    let node = vptonode(vp);
    let oldsize = node.tn_size.get();

    if vp.vtype() != VREG {
        return Err(Errno::EINVAL);
    }

    if uio.uio_resid == 0 {
        return Ok(());
    }

    if ioflag & IO_APPEND != 0 {
        uio.uio_offset = node.tn_size.get();
    }

    if uio.uio_offset < 0 || uio.uio_offset as u64 + uio.uio_resid as u64 > i64::MAX as u64 {
        return Err(Errno::EFBIG);
    }

    // do the filesize rlimit check
    let p = uio.uio_procp.and_then(G::thread_pin);
    let overrun = vn_fsizechk(vp, uio, ioflag, p)?;

    let extended = uio.uio_offset + uio.uio_resid as Off > node.tn_size.get();
    let error: Result<(), Errno> = 'out: {
        if extended && let Err(e) = tmpfs_reg_resize(vp, uio.uio_offset + uio.uio_resid as Off) {
            break 'out Err(e);
        }

        let mut error = Ok(());
        while error.is_ok() && uio.uio_resid > 0 {
            let _ = uvm_vnp_uncache(vp);
            let len = (node.tn_size.get() - uio.uio_offset).min(uio.uio_resid as Off);
            if len == 0 {
                break;
            }
            error = tmpfs_uiomove(&node, uio, len as usize);
        }
        if error.is_err() {
            let _ = tmpfs_reg_resize(vp, oldsize);
        }

        tmpfs_update(&node, TMPFS_NODE_MODIFIED | TMPFS_NODE_CHANGED);
        if extended {
            VN_KNOTE(vp, NOTE_WRITE | NOTE_EXTEND);
        } else {
            VN_KNOTE(vp, NOTE_WRITE);
        }
        error
    };
    // out:
    if error.is_err() {
        kassert!(oldsize == node.tn_size.get());
    } else {
        kassert!(uio.uio_resid == 0);

        // correct the result for writes clamped by vn_fsizechk()
        uio.uio_resid += overrun;
    }
    error
}

/// `tmpfs_fsync` (`vop_fsync`): nothing to do.
pub fn tmpfs_fsync<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    // DIAGNOSTIC: Nothing to do. Just update.
    kassert!(vop_islocked(vp) != 0);
    Ok(())
}

/// `tmpfs_remove` (`vop_remove`): unlink a file.
///
/// - Both directory (`dvp`) and file (`vp`) are locked.
/// - `VOP_REMOVE` unlocks and drops the reference on both.
pub fn tmpfs_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    kassert!(cnp.cn_flags & HASBUF != 0);

    let error: Result<(), Errno> = 'out: {
        if vp.vtype() == VDIR {
            break 'out Err(Errno::EPERM);
        }

        let dnode = vptonode(dvp);
        let node = vptonode(vp);

        // Files marked as immutable or append-only cannot be deleted.
        if node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
            break 'out Err(Errno::EPERM);
        }

        // Likewise, files residing on directories marked as append-only cannot be deleted.
        if dnode.tn_flags.get() & APPEND != 0 {
            break 'out Err(Errno::EPERM);
        }

        // Lookup the directory entry (check the cached hint first).
        let de = tmpfs_dir_cached(&node).or_else(|| tmpfs_dir_lookup(&dnode, cnp));

        kassert!(de.as_ref().is_some_and(|de| de.td_node_is(&node)));
        let Some(de) = de else {
            kpanic(format_args!("tmpfs_remove: no entry for node {:p}", &*node));
        };

        // Remove the entry from the directory (drops the link count) and destroy it.
        // Note: the inode referred by it will not be destroyed until the vnode is
        // reclaimed/recycled.
        tmpfs_dir_detach(&dnode, &de);
        tmpfs_free_dirent(&vfstotmpfs(vmount(vp)), de);
        if node.tn_links.get() > 0 {
            // We removed a hard link.
            tmpfs_update(&node, TMPFS_NODE_CHANGED);
        }
        Ok(())
    };
    // out: pool_put(&namei_pool, cnp->cn_pnbuf)
    drop(cnp.take_pnbuf());
    error
}

/// `tmpfs_link` (`vop_link`): create a hard link.
pub fn tmpfs_link<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    kassert!(vop_islocked(dvp) != 0);
    kassert!(!ptr::eq(dvp, vp));

    let dnode = vptodir(dvp);
    let node = vptonode(vp);

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);

    let error: Result<(), Errno> = 'out: {
        // Check for maximum number of links limit.
        if node.tn_links.get() == LINK_MAX {
            break 'out Err(Errno::EMLINK);
        }
        kassert!(node.tn_links.get() < LINK_MAX);

        // We cannot create links of files marked immutable or append-only.
        if node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
            break 'out Err(Errno::EPERM);
        }

        if tmpfs_dirseq_full(&dnode) {
            break 'out Err(Errno::ENOSPC);
        }

        // Allocate a new directory entry to represent the inode.
        let de = match tmpfs_alloc_dirent(&vfstotmpfs(vmount(vp)), cnp.name()) {
            Ok(de) => de,
            Err(e) => break 'out Err(e),
        };

        // Insert the entry into the directory. It will increase the inode link count.
        tmpfs_dir_attach(&dnode, de, TmpfsNodeRef::clone(&node));

        // Update the timestamps and trigger the event.
        if let Some(nvp) = node.vnode() {
            VN_KNOTE(nvp, NOTE_LINK);
        }
        tmpfs_update(&node, TMPFS_NODE_CHANGED);
        Ok(())
    };
    // out:
    drop(cnp.take_pnbuf());
    let _ = vop_unlock(vp);
    vput(dvp);
    error
}

/// `tmpfs_mkdir` (`vop_mkdir`).
pub fn tmpfs_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    kassert!(vap.va_type == VDIR);
    let r = tmpfs_alloc_file(dvp, vap, cnp, None);
    *vpp = r.as_ref().ok().copied();
    vput(dvp);
    r.map(|_| ())
}

/// `tmpfs_rmdir` (`vop_rmdir`): remove an empty directory.
pub fn tmpfs_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let tmp = vfstotmpfs(vmount(dvp));
    let dnode = vptodir(dvp);
    let node = vptodir(vp);

    kassert!(vop_islocked(dvp) != 0);
    kassert!(vop_islocked(vp) != 0);
    kassert!(cnp.cn_flags & HASBUF != 0);

    let error: Result<(), Errno> = 'out: {
        if cnp.name() == b".." {
            break 'out Err(Errno::ENOTEMPTY);
        }

        kassert!(node.dir().tn_parent.points_to(&dnode));

        // Directories with more than two entries ('.' and '..') cannot be removed.
        if node.tn_size.get() > 0 && !node.dir().tn_dir.is_empty(&hal::klock::reader()) {
            break 'out Err(Errno::ENOTEMPTY);
        }

        // Lookup the directory entry (check the cached hint first).
        let de = tmpfs_dir_cached(&node).or_else(|| tmpfs_dir_lookup(&dnode, cnp));

        kassert!(de.as_ref().is_some_and(|de| de.td_node_is(&node)));
        let Some(de) = de else {
            kpanic(format_args!("tmpfs_rmdir: no entry for node {:p}", &*node));
        };

        // Check flags to see if we are allowed to remove the directory.
        if dnode.tn_flags.get() & APPEND != 0 || node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
            break 'out Err(Errno::EPERM);
        }

        // Decrement the link count for the virtual '.' entry.
        node.tn_links.set(node.tn_links.get() - 1);
        tmpfs_update(&node, TMPFS_NODE_STATUSALL);

        // Detach the directory entry from the directory.
        tmpfs_dir_detach(&dnode, &de);

        // Purge the cache for parent.
        cache_purge(dvp);

        // Destroy the directory entry. Note: the inode referred by it will not be destroyed
        // until the vnode is reclaimed.
        tmpfs_free_dirent(&tmp, de);
        kassert!(node.dir().tn_dir.is_empty(&hal::klock::reader()));

        kassert!(node.tn_links.get() == 0);
        Ok(())
    };
    // out:
    drop(cnp.take_pnbuf());
    // Release the nodes.
    vput(dvp);
    vput(vp);
    error
}

/// `tmpfs_symlink` (`vop_symlink`).
pub fn tmpfs_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &mut Vattr,
    target: &[u8],
) -> Result<(), Errno> {
    kassert!(vap.va_type == VNON);
    vap.va_type = VLNK;

    let r = tmpfs_alloc_file(dvp, vap, cnp, Some(target));
    vput(dvp);
    let vp = r?;
    *vpp = Some(vp);
    vput(vp);

    Ok(())
}

/// `tmpfs_readdir` (`vop_readdir`).
pub fn tmpfs_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);

    // This operation only makes sense on directory nodes.
    if vp.vtype() != VDIR {
        return Err(Errno::ENOTDIR);
    }
    let node = vptodir(vp);
    // Retrieve the directory entries, unless it is being destroyed.
    let error = if node.tn_links.get() != 0 {
        tmpfs_dir_getdents(&node, uio)
    } else {
        Ok(())
    };

    *eofflag = i32::from(error.is_ok() && uio.uio_offset == TMPFS_DIRSEQ_EOF as Off);
    error
}

/// `tmpfs_readlink` (`vop_readlink`): the target, `MIN(tn_size, uio_resid)` bytes of it.
pub fn tmpfs_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);
    kassert!(uio.uio_offset == 0);
    kassert!(vp.vtype() == VLNK);

    let node = vptonode(vp);
    let link = node.link();
    let len = link.len().min(uio.uio_resid);
    let error = if uio.uio_segflg == UioSeg::UIO_USERSPACE && uio.uio_rw == UioRw::UIO_READ {
        // Straight from the link's block (immutable for the node's life).
        uiomove_user_out(&link[..len], uio)
    } else {
        // A kernel uio: through bsd's uiomove, from a copy (the module's deviations).
        let mut buf = [0u8; MAXPATHLEN];
        buf[..len].copy_from_slice(&link[..len]);
        <G::FileGlue as FileGlue>::uiomove(&mut buf[..len], uio)
    };

    if vmount(vp).flag() & MNT_NOATIME == 0 {
        tmpfs_update(&node, TMPFS_NODE_ACCESSED);
    }

    error
}

/// `tmpfs_inactive` (`vop_inactive`): the last use of the vnode went; drop the cached
/// mapping of a regular file and, if the node has no links left, reclaim the vnode at once
/// so that the node can be freed and reused immediately.
pub fn tmpfs_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    kassert!(vop_islocked(vp) != 0);

    let node = vptonode(vp);

    if vp.vtype() == VREG && tmpfs_uio_cached(&node) {
        tmpfs_uio_uncache(&node);
    }

    let _ = vop_unlock(vp);

    // If we are done with the node, reclaim it so that it can be reused immediately.
    if node.tn_links.get() == 0 {
        drop(node);
        let _ = vrecycle(vp, p);
    }

    Ok(())
}

/// `tmpfs_reclaim` (`vop_reclaim`): disassociate the node from the vnode, and destroy the
/// node if it has no links (unless `tmpfs_vnode_get` is about to give it a new vnode). The
/// vnode's slot handle goes to `tmpfs_free_node`, which drops it with the list's.
pub fn tmpfs_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let tmp = vfstotmpfs(vmount(vp));
    let node = vptonode(vp);

    // Disassociate inode from vnode.
    let racing = {
        let _nlock = node.tn_nlock.write();
        node.tn_vnode.set(None);
        drop(vp.take_node()); // vp->v_data = NULL
        // Check if tmpfs_vnode_get() is racing with us.
        tmpfs_node_reclaiming(&node)
    };

    cache_purge(vp);

    // If inode is not referenced, i.e. no links, then destroy it. Note: if racing - inode
    // is about to get a new vnode, leave it.
    if node.tn_links.get() == 0 && !racing {
        tmpfs_free_node(&tmp, node);
    }
    Ok(())
}

/// `tmpfs_pathconf` (`vop_pathconf`).
pub fn tmpfs_pathconf(name: i32, retval: &mut Register) -> Result<(), Errno> {
    *retval = match name {
        _PC_LINK_MAX => LINK_MAX as Register,
        _PC_NAME_MAX => TMPFS_MAXNAMLEN as Register,
        _PC_CHOWN_RESTRICTED | _PC_NO_TRUNC | _PC_TIMESTAMP_RESOLUTION => 1,
        _PC_FILESIZEBITS => 64,
        _ => return Err(Errno::EINVAL),
    };
    Ok(())
}

/// `tmpfs_advlock` (`vop_advlock`): advisory record locking on the node.
pub fn tmpfs_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let node = vptonode(vp);

    lf_advlock::<G>(&node.tn_lockf, node.tn_size.get(), id, op, fl, flags)
}

/// `tmpfs_print` (`vop_print`): describe the node (`DEBUG`/`DIAGNOSTIC` kernels only).
pub fn tmpfs_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        let node = vptonode(vp);

        crate::prf::printf(format_args!(
            "tag VT_TMPFS, tmpfs_node {:p}, flags 0x{:x}, links {}\n\tmode 0{:o}, owner {}, group {}, size {}",
            &*node,
            node.tn_flags.get(),
            node.tn_links.get(),
            node.tn_mode.get(),
            node.tn_uid.get(),
            node.tn_gid.get(),
            node.tn_size.get()
        ));
        // FIFO: fifo_printinfo(vp) for a VFIFO (miscfs/fifofs, not ported).
        crate::prf::printf(format_args!("\n"));
    }
    #[cfg(not(any(feature = "debug", feature = "diagnostic")))]
    let _ = vp;
    Ok(())
}

/// `tmpfs_bwrite` (`vop_bwrite`): a null op.
pub fn tmpfs_bwrite() -> Result<(), Errno> {
    Ok(())
}

/// `tmpfs_strategy` (`vop_strategy`): tmpfs has no buffers.
pub fn tmpfs_strategy() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `tmpfs_ioctl` (`vop_ioctl`).
pub fn tmpfs_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `tmpfs_lock` (`vop_lock`): take the node's vnode lock.
pub fn tmpfs_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vptonode(vp).tn_vlock.enter(flags & LK_RWFLAGS)
}

/// `tmpfs_unlock` (`vop_unlock`).
pub fn tmpfs_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vptonode(vp).tn_vlock.exit();
    Ok(())
}

/// `tmpfs_islocked` (`vop_islocked`).
pub fn tmpfs_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vptonode(vp).tn_vlock.status()
}

/// `tmpfs_rename` (`vop_rename`): rename routine, the hairiest system call, with the insane
/// API.
///
/// Arguments: `fdvp` (from-parent vnode), `fvp` (from-leaf), `tdvp` (to-parent) and `tvp`
/// (to-leaf), if exists (NULL if not).
///
/// - Caller holds a reference on `fdvp` and `fvp`, they are unlocked. Note: `fdvp` and
///   `fvp` can refer to the same object (i.e. when it is root).
/// - Both `tdvp` and `tvp` are referenced and locked. It is our responsibility to release
///   the references and unlock them (or destroy).
pub fn tmpfs_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    kassert!(fcnp.has_name());
    // KASSERT(VOP_ISLOCKED(fdvp) != LK_EXCLUSIVE);
    // KASSERT(VOP_ISLOCKED(fvp) != LK_EXCLUSIVE);
    kassert!(fdvp.vtype() == VDIR);
    kassert!(tdvp.vtype() == VDIR);
    kassert!(fcnp.cn_flags & HASBUF != 0);
    kassert!(tcnp.cn_flags & HASBUF != 0);

    let cred = fcnp.cn_cred;
    kassert!(tcnp.cn_cred == cred);

    // Check for cross-device rename. Also don't allow renames of mount points.
    if !same(fvp.mount(), tdvp.mount())
        || !same(fdvp.mount(), fvp.mount())
        || tvp.is_some_and(|tvp| !same(fvp.mount(), tvp.mount()))
    {
        tmpfs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(Errno::EXDEV);
    }

    // Can't check the locks on these until we know they're on the same FS, as not all FS do
    // locking the same way.
    kassert!(vop_islocked(tdvp) == LK_EXCLUSIVE);
    kassert!(tvp.is_none_or(|tvp| vop_islocked(tvp) == LK_EXCLUSIVE));

    // Reject renaming '.' and '..'.
    if is_dot_or_dotdot(fcnp) {
        tmpfs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(Errno::EINVAL);
    }

    // Sanitize our world from the VFS insanity. Unlock the target directory and node,
    // which are locked. Release the children, which are referenced. Check for
    // rename("x", "y/."), which it is our responsibility to reject, not the caller's. (But
    // the caller does reject rename("x/.", "y"). Go figure.)
    let _ = vop_unlock(tdvp);
    if let Some(tvp) = tvp
        && !ptr::eq(tvp, tdvp)
    {
        let _ = vop_unlock(tvp);
    }

    vrele(fvp);
    if let Some(tvp) = tvp {
        vrele(tvp);
    }

    let error = if same(tvp, Some(tdvp)) {
        Err(Errno::EINVAL)
    } else {
        tmpfs_sane_rename(fdvp, fcnp, tdvp, tcnp, cred, false)
    };

    // out: All done, whether with success or failure. Release the directory nodes now, as
    // the caller expects from the VFS protocol.
    vrele(fdvp);
    vrele(tdvp);

    error
}

/// `tmpfs_sane_rename`: rename routine, the hairiest system call, with the sane API.
///
/// Arguments: `fdvp` (from directory vnode), `fcnp` (from component name), `tdvp` (to
/// directory vnode), and `tcnp` (to component name).
///
/// `fdvp` and `tdvp` must be referenced and unlocked.
fn tmpfs_sane_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fcnp: &Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tcnp: &Cn<'_, G>,
    cred: Cred<'_>,
    posixly_correct: bool,
) -> Result<(), Errno> {
    let p = fcnp.cn_proc;
    // KASSERT(VOP_ISLOCKED(fdvp) != LK_EXCLUSIVE);
    // KASSERT(VOP_ISLOCKED(tdvp) != LK_EXCLUSIVE);
    kassert!(fdvp.vtype() == VDIR);
    kassert!(tdvp.vtype() == VDIR);
    kassert!(same(fdvp.mount(), tdvp.mount()));
    kassert!(fcnp.cn_flags & ISDOTDOT == 0);
    kassert!(tcnp.cn_flags & ISDOTDOT == 0);
    kassert!(!is_dot_or_dotdot(fcnp));
    kassert!(!is_dot_or_dotdot(tcnp));

    // Pull out the tmpfs data structures.
    let fdnode = vptonode(fdvp);
    let tdnode = vptonode(tdvp);
    kassert!(same(fdnode.vnode(), Some(fdvp)));
    kassert!(same(tdnode.vnode(), Some(tdvp)));
    kassert!(fdnode.tn_type == VDIR);
    kassert!(tdnode.tn_type == VDIR);

    let mount = vmount(fdvp);
    kassert!(same(Some(mount), tdvp.mount()));
    // XXX How can we be sure this stays true? (Not that you're likely to mount a tmpfs
    // read-only...)
    kassert!(mount.flag() & MNT_RDONLY == 0);
    let tmpfs = vfstotmpfs(mount);

    // Decide whether we need a new name, and allocate memory for it if so. Do this before
    // locking anything or taking destructive actions so that we can back out safely and
    // sleep safely. XXX Is sleeping an issue here? Can this just be moved into
    // tmpfs_rename_attachdetach?
    let tlen = tcnp.name().len();
    let mut newname = if tmpfs_strname_neqlen(fcnp, tcnp) {
        match tmpfs_strname_alloc(&tmpfs, tlen) {
            Some(name) => Some(name),
            None => return Err(Errno::ENOSPC), // out_unlocked, with nothing to free
        }
    } else {
        None
    };

    let error: Result<(), Errno> = 'out_unlocked: {
        // Lock and look up everything. GCC is not very clever.
        let RenameEntered { fde, fvp, tde, tvp } = match tmpfs_rename_enter(
            mount, &tmpfs, cred, p, fdvp, &fdnode, fcnp, tdvp, &tdnode, tcnp,
        ) {
            Ok(entered) => entered,
            Err(e) => break 'out_unlocked Err(e),
        };

        // Check that everything is locked and looks right.
        let fnode = fde.node();
        kassert!(same(fnode.vnode(), Some(fvp)));
        kassert!(fnode.tn_type == fvp.vtype());
        kassert!(tde.is_none() == tvp.is_none());
        kassert!(tde.as_ref().is_none_or(|tde| !tde.td_node_is_none()));
        kassert!(tde.as_ref().is_none_or(|tde| same(tde.node().vnode(), tvp)));
        kassert!(
            tde.as_ref()
                .is_none_or(|tde| tvp.is_some_and(|tvp| tde.node().tn_type == tvp.vtype()))
        );
        kassert!(vop_islocked(fdvp) == LK_EXCLUSIVE);
        kassert!(vop_islocked(tdvp) == LK_EXCLUSIVE);
        kassert!(vop_islocked(fvp) == LK_EXCLUSIVE);
        kassert!(tvp.is_none_or(|tvp| vop_islocked(tvp) == LK_EXCLUSIVE));

        let error: Result<(), Errno> = 'out_locked: {
            if same(Some(fvp), tvp) {
                // If the source and destination are the same object, we need only at most
                // delete the source entry.
                kassert!(tvp.is_some());
                if fnode.tn_type == VDIR {
                    // XXX How can this possibly happen?
                    break 'out_locked Err(Errno::EINVAL);
                }
                let same_entry = tde
                    .as_ref()
                    .is_some_and(|tde| TmpfsDirentRef::ptr_eq(&fde, tde));
                if !posixly_correct && !same_entry {
                    // XXX Doesn't work because of locking.
                    // error = VOP_REMOVE(fdvp, fvp);
                    if let Err(e) = tmpfs_do_remove(&tmpfs, fdvp, &fdnode, fde, fvp, cred, p) {
                        break 'out_locked Err(e);
                    }
                }
                // goto success
            } else {
                kassert!(
                    tde.as_ref()
                        .is_none_or(|tde| !TmpfsDirentRef::ptr_eq(&fde, tde))
                );
                kassert!(!same(Some(fvp), tvp));

                // If the target exists, refuse to rename a directory over a non-directory
                // or vice versa, or to clobber a non-empty directory.
                if let Some(tvp) = tvp {
                    kassert!(tde.as_ref().is_some_and(|tde| !tde.td_node_is_none()));
                    let fdir = fvp.vtype() == VDIR;
                    let tdir = tvp.vtype() == VDIR;
                    let error = match (fdir, tdir) {
                        (true, true)
                            if tde.as_ref().is_some_and(|tde| tde.node().tn_size.get() > 0) =>
                        {
                            Err(Errno::ENOTEMPTY)
                        }
                        (true, true) | (false, false) => Ok(()),
                        (true, false) => Err(Errno::ENOTDIR),
                        (false, true) => Err(Errno::EISDIR),
                    };
                    if let Err(e) = error {
                        break 'out_locked Err(e);
                    }
                    kassert!(fdir == tdir);
                }

                // Authorize the rename.
                let tnode = tde.as_ref().map(|tde| tde.node());
                if let Err(e) =
                    tmpfs_rename_check_possible(&fdnode, &fnode, &tdnode, tnode.as_ref())
                {
                    break 'out_locked Err(e);
                }
                if let Err(e) =
                    tmpfs_rename_check_permitted(cred, p, &fdnode, &fnode, &tdnode, tnode.as_ref())
                {
                    break 'out_locked Err(e);
                }
                drop(tnode);

                // Everything is hunky-dory. Shuffle the directory entries.
                tmpfs_rename_attachdetach(&tmpfs, fdvp, &fde, fvp, tdvp, tde, tvp);

                // Update the directory entry's name necessary, and flag metadata updates. A
                // memory allocation failure here is not OK because we've already committed
                // some changes that we can't back out at this point, and we have things
                // locked so we can't sleep, hence the early allocation above.
                if let Some(mut name) = newname.take() {
                    kassert!(tlen <= TMPFS_MAXNAMLEN);

                    let oldlen = usize::from(fde.td_namelen.get());
                    name[..tlen].copy_from_slice(tcnp.name());
                    // Commit newname and don't free it on the way out.
                    let old = fde.set_name(name, tlen as u16);
                    tmpfs_strname_free(&tmpfs, old, oldlen);

                    tmpfs_update(&fde.node(), TMPFS_NODE_CHANGED);
                    tmpfs_update(&tdnode, TMPFS_NODE_MODIFIED);
                }
            }

            // success:
            VN_KNOTE(fvp, NOTE_RENAME);
            tmpfs_rename_cache_purge(fdvp, fvp, tdvp, tvp);
            Ok(())
        };

        // out_locked:
        tmpfs_rename_exit(fdvp, fvp, tdvp, tvp);
        error
    };

    // out_unlocked:
    // KASSERT(VOP_ISLOCKED(fdvp) != LK_EXCLUSIVE);
    // KASSERT(VOP_ISLOCKED(tdvp) != LK_EXCLUSIVE);
    // KASSERT((fvp == NULL) || (VOP_ISLOCKED(fvp) != LK_EXCLUSIVE));
    // KASSERT((tvp == NULL) || (VOP_ISLOCKED(tvp) != LK_EXCLUSIVE));

    if let Some(name) = newname {
        tmpfs_strname_free(&tmpfs, Some(name), tlen);
    }

    error
}

/// `tmpfs_rename_enter`: look up `fcnp` in `fdnode`/`fdvp` and return its directory entry
/// and the associated vnode; fail if not found. Look up `tcnp` in `tdnode`/`tdvp` and
/// return its directory entry and the associated vnode, or none if not found. Fail if
/// anything has been mounted on any of the nodes involved.
///
/// `fdvp` and `tdvp` must be referenced.
///
/// On entry, nothing is locked.
///
/// On success, everything is locked, and `fvp`, and `tvp` if any, are referenced. The only
/// pairs of vnodes that may be identical are {`fdvp`, `tdvp`} and {`fvp`, `tvp`}.
///
/// On failure, everything remains as was.
///
/// Locking everything including the source and target nodes is necessary to make sure that,
/// e.g., link count updates are OK. The locking order is, in general, ancestor-first,
/// matching the order you need to use to look up a descendant anyway.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's parameters and the thread"
)]
fn tmpfs_rename_enter<G: ProcGlue>(
    mount: &'static Mount<G>,
    tmpfs: &TmpfsMount<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
    fdvp: &'static Vnode<G>,
    fdnode: &TmpfsNodeRef<G>,
    fcnp: &Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tdnode: &TmpfsNodeRef<G>,
    tcnp: &Cn<'_, G>,
) -> Result<RenameEntered<G>, Errno> {
    kassert!(same(fdnode.vnode(), Some(fdvp)));
    kassert!(same(tdnode.vnode(), Some(tdvp)));
    kassert!(fdnode.tn_type == VDIR);
    kassert!(tdnode.tn_type == VDIR);

    let entered = if ptr::eq(fdvp, tdvp) {
        kassert!(TmpfsNodeRef::ptr_eq(fdnode, tdnode));
        tmpfs_rename_enter_common(mount, tmpfs, cred, p, fdvp, fdnode, fcnp, tcnp)?
    } else {
        kassert!(!TmpfsNodeRef::ptr_eq(fdnode, tdnode));
        tmpfs_rename_enter_separate(mount, cred, p, fdvp, fdnode, fcnp, tdvp, tdnode, tcnp)?
    };

    let RenameEntered { fde, fvp, tde, tvp } = &entered;
    kassert!(tde.is_none() == tvp.is_none());
    kassert!(tde.as_ref().is_none_or(|tde| !tde.td_node_is_none()));
    kassert!(
        tde.as_ref()
            .is_none_or(|tde| same(tde.node().vnode(), *tvp))
    );
    kassert!(!fde.td_node_is_none());
    kassert!(vop_islocked(fdvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(fvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tdvp) == LK_EXCLUSIVE);
    kassert!(tvp.is_none_or(|tvp| vop_islocked(tvp) == LK_EXCLUSIVE));
    kassert!(!ptr::eq(*fvp, fdvp));
    kassert!(!ptr::eq(*fvp, tdvp));
    kassert!(!same(*tvp, Some(fdvp)));
    kassert!(!same(*tvp, Some(tdvp)));
    Ok(entered)
}

/// `tmpfs_rename_enter_common`: lock and look up with a common source/target directory.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's parameters and the thread"
)]
fn tmpfs_rename_enter_common<G: ProcGlue>(
    mount: &'static Mount<G>,
    _tmpfs: &TmpfsMount<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
    dvp: &'static Vnode<G>,
    dnode: &TmpfsNodeRef<G>,
    fcnp: &Cn<'_, G>,
    tcnp: &Cn<'_, G>,
) -> Result<RenameEntered<G>, Errno> {
    tmpfs_rename_lock_directory(dvp, dnode)?; // fail0

    let parent = dnode.dir().tn_parent.get();
    let entered: Result<RenameEntered<G>, Errno> = 'fail1: {
        // Did we lose a race with mount?
        if dvp.mountedhere().is_some() {
            break 'fail1 Err(Errno::EBUSY);
        }

        // Make sure the caller may read the directory.
        if let Err(e) = vop_access(dvp, VEXEC, cred, p) {
            break 'fail1 Err(e);
        }

        // The order in which we lock the source and target nodes is irrelevant because
        // there can only be one rename on this directory in flight at a time, and we have
        // it locked.

        let Some(fde) = tmpfs_dir_lookup(dnode, fcnp) else {
            break 'fail1 Err(Errno::ENOENT);
        };

        let fnode = fde.node();
        // We ruled out `.' earlier.
        kassert!(!TmpfsNodeRef::ptr_eq(&fnode, dnode));
        // We ruled out `..' earlier.
        kassert!(!is_node(&fnode, parent.as_ref()));
        let nlock = fnode.tn_nlock.write();
        let fvp = match tmpfs_vnode_get(mount, &fnode, nlock) {
            Ok(fvp) => fvp,
            Err(e) => break 'fail1 Err(e),
        };
        kassert!(vop_islocked(fvp) == LK_EXCLUSIVE);
        kassert!(!ptr::eq(fvp, dvp));
        kassert!(same(fvp.mount(), Some(mount)));

        let entered: Result<RenameEntered<G>, Errno> = 'fail2: {
            // Refuse to rename a mount point.
            if fvp.vtype() == VDIR && fvp.mountedhere().is_some() {
                break 'fail2 Err(Errno::EBUSY);
            }

            let tde = tmpfs_dir_lookup(dnode, tcnp);
            let tvp = match &tde {
                None => None,
                Some(tde) => {
                    let tnode = tde.node();
                    // We ruled out `.' earlier.
                    kassert!(!TmpfsNodeRef::ptr_eq(&tnode, dnode));
                    // We ruled out `..' earlier.
                    kassert!(!is_node(&tnode, parent.as_ref()));
                    let tvp = if TmpfsNodeRef::ptr_eq(&tnode, &fnode) {
                        vref(fvp);
                        fvp
                    } else {
                        let nlock = tnode.tn_nlock.write();
                        let tvp = match tmpfs_vnode_get(mount, &tnode, nlock) {
                            Ok(tvp) => tvp,
                            Err(e) => break 'fail2 Err(e),
                        };
                        kassert!(same(tvp.mount(), Some(mount)));
                        // Refuse to rename over a mount point.
                        if tvp.vtype() == VDIR && tvp.mountedhere().is_some() {
                            // fail3: tvp is not fvp here.
                            vput(tvp);
                            break 'fail2 Err(Errno::EBUSY);
                        }
                        tvp
                    };
                    kassert!(vop_islocked(tvp) == LK_EXCLUSIVE);
                    Some(tvp)
                }
            };
            kassert!(!same(tvp, Some(dvp)));

            Ok(RenameEntered { fde, fvp, tde, tvp })
        };
        if entered.is_err() {
            // fail2:
            vput(fvp);
        }
        entered
    };
    if entered.is_err() {
        // fail1:
        let _ = vop_unlock(dvp);
    }
    entered
}

/// `tmpfs_rename_enter_separate`: lock and look up with separate source and target
/// directories.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's parameters and the thread"
)]
fn tmpfs_rename_enter_separate<G: ProcGlue>(
    mount: &'static Mount<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
    fdvp: &'static Vnode<G>,
    fdnode: &TmpfsNodeRef<G>,
    fcnp: &Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tdnode: &TmpfsNodeRef<G>,
    tcnp: &Cn<'_, G>,
) -> Result<RenameEntered<G>, Errno> {
    kassert!(!ptr::eq(fdvp, tdvp));
    kassert!(!TmpfsNodeRef::ptr_eq(fdnode, tdnode));

    // #if 0 XXX: mutex_enter(&tmpfs->tm_rename_lock);

    let intermediate_node = tmpfs_rename_genealogy(fdnode, tdnode)?;

    // intermediate_node == NULL means fdnode is not an ancestor of tdnode.
    let (fde, fvp, tde, tvp) = if intermediate_node.is_none() {
        let l = tmpfs_rename_lock(
            mount,
            cred,
            p,
            Errno::ENOTEMPTY,
            (tdvp, tdnode, tcnp, true),
            (fdvp, fdnode, fcnp, false),
        )?;
        (l.b_dirent, l.b_vp, l.a_dirent, l.a_vp)
    } else {
        let l = tmpfs_rename_lock(
            mount,
            cred,
            p,
            Errno::EINVAL,
            (fdvp, fdnode, fcnp, false),
            (tdvp, tdnode, tcnp, true),
        )?;
        (l.a_dirent, l.a_vp, l.b_dirent, l.b_vp)
    };

    // The source was not missing-ok: tmpfs_rename_lock found it, with its vnode.
    let (Some(fde), Some(fvp)) = (fde, fvp) else {
        kpanic(format_args!("tmpfs_rename_enter_separate: no source"));
    };
    kassert!(!fde.td_node_is_none());

    // Reject rename("foo/bar", "foo/bar/baz/quux/zot").
    if intermediate_node
        .as_ref()
        .is_some_and(|i| fde.td_node_is(i))
    {
        tmpfs_rename_exit(fdvp, fvp, tdvp, tvp);
        return Err(Errno::EINVAL);
    }

    Ok(RenameEntered { fde, fvp, tde, tvp })

    // fail: #if 0 XXX: mutex_exit(&tmpfs->tm_rename_lock);
}

/// `tmpfs_rename_exit`: unlock everything we locked for rename.
///
/// `fdvp` and `tdvp` must be referenced.
///
/// On entry, everything is locked, and `fvp` and `tvp` referenced.
///
/// On exit, everything is unlocked, and `fvp` and `tvp` are released.
fn tmpfs_rename_exit<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
) {
    kassert!(!ptr::eq(fdvp, fvp));
    kassert!(!same(Some(fdvp), tvp));
    kassert!(!same(Some(tdvp), tvp));
    kassert!(!ptr::eq(tdvp, fvp));
    kassert!(vop_islocked(fdvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tdvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(fvp) == LK_EXCLUSIVE);
    kassert!(tvp.is_none_or(|tvp| vop_islocked(tvp) == LK_EXCLUSIVE));

    if let Some(tvp) = tvp {
        if ptr::eq(tvp, fvp) {
            vrele(tvp);
        } else {
            vput(tvp);
        }
    }
    let _ = vop_unlock(tdvp);
    vput(fvp);
    if !ptr::eq(fdvp, tdvp) {
        let _ = vop_unlock(fdvp);
    }

    // #if 0 XXX: if (fdvp != tdvp) mutex_exit(&tmpfs->tm_rename_lock);
}

/// `tmpfs_rename_lock_directory`: lock a directory, but fail if it has been rmdir'd.
///
/// `vp` must be referenced.
fn tmpfs_rename_lock_directory<G: ProcGlue>(
    vp: &'static Vnode<G>,
    node: &TmpfsNode<G>,
) -> Result<(), Errno> {
    kassert!(same(node.vnode(), Some(vp)));
    kassert!(node.tn_type == VDIR);

    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    if node.dir().tn_parent.is_none() {
        let _ = vop_unlock(vp);
        return Err(Errno::ENOENT);
    }

    Ok(())
}

/// `tmpfs_rename_genealogy`: analyze the genealogy of the source and target nodes.
///
/// On success, returns either the child of `fdnode` of which `tdnode` is a descendant, or
/// none if `tdnode` is not a descendant of `fdnode` at all.
///
/// `fdnode` and `tdnode` must be unlocked and referenced. The file system's rename lock
/// must also be held, to exclude concurrent changes to the file system's genealogy other
/// than rmdir.
///
/// XXX This causes an extra lock/unlock of `tdnode` in the case when we're just about to
/// lock it again before locking anything else. However, changing that requires
/// reorganizing the code to make it even more horrifically obscure.
fn tmpfs_rename_genealogy<G: ProcGlue>(
    fdnode: &TmpfsNodeRef<G>,
    tdnode: &TmpfsNodeRef<G>,
) -> Result<Option<TmpfsNodeRef<G>>, Errno> {
    kassert!(!TmpfsNodeRef::ptr_eq(fdnode, tdnode));
    kassert!(fdnode.vnode().is_some());
    kassert!(tdnode.vnode().is_some());
    kassert!(fdnode.tn_type == VDIR);
    kassert!(tdnode.tn_type == VDIR);

    // We need to provisionally lock tdnode->tn_vnode to keep rmdir from deleting it -- or
    // any ancestor -- at an inopportune moment.
    tmpfs_rename_lock_directory(tn_vnode(tdnode), tdnode)?;

    let mut node = TmpfsNodeRef::clone(tdnode);
    let intermediate_node = loop {
        let Some(parent) = node.dir().tn_parent.get() else {
            kpanic(format_args!(
                "tmpfs_rename_genealogy: directory {:p} without a parent",
                &*node
            ));
        };
        kassert!(parent.tn_type == VDIR);

        // Did we hit the root without finding fdnode?
        if TmpfsNodeRef::ptr_eq(&parent, &node) {
            break None;
        }

        // Did we find that fdnode is an ancestor?
        if TmpfsNodeRef::ptr_eq(&parent, fdnode) {
            break Some(node);
        }

        // Neither -- keep ascending the family tree.
        node = parent;
    };

    let _ = vop_unlock(tn_vnode(tdnode));
    Ok(intermediate_node)
}

/// `tmpfs_rename_lock`: lock directories a and b, which must be distinct, and look up and
/// lock nodes a and b. Do a first and then b. Directory b may not be an ancestor of
/// directory a, although directory a may be an ancestor of directory b. Fail with
/// `overlap_error` if node a is directory b. Neither componentname may be `.` or `..`.
/// Each side is the C's `(x_dvp, x_dnode, x_cnp, x_missing_ok)`.
///
/// `a_dvp` and `b_dvp` must be referenced.
///
/// On entry, `a_dvp` and `b_dvp` are unlocked.
///
/// On success,
/// - `a_dvp` and `b_dvp` are locked,
/// - `a_dirent` is either none or a directory entry whose node is locked and referenced,
/// - `a_vp` is the corresponding vnode,
/// - `b_dirent` is either none or a directory entry whose node is locked and referenced,
/// - `b_vp` is either none or the corresponding vnode, and
/// - the only pair of vnodes that may be identical is `a_vp` and `b_vp`.
///
/// On failure, `a_dvp` and `b_dvp` are left unlocked and nothing is returned.
#[allow(
    clippy::type_complexity,
    reason = "the C's four parameters of each side"
)]
fn tmpfs_rename_lock<G: ProcGlue>(
    mount: &'static Mount<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
    overlap_error: Errno,
    (a_dvp, a_dnode, a_cnp, a_missing_ok): (&'static Vnode<G>, &TmpfsNodeRef<G>, &Cn<'_, G>, bool),
    (b_dvp, b_dnode, b_cnp, b_missing_ok): (&'static Vnode<G>, &TmpfsNodeRef<G>, &Cn<'_, G>, bool),
) -> Result<RenameLocked<G>, Errno> {
    kassert!(!ptr::eq(a_dvp, b_dvp));
    kassert!(!TmpfsNodeRef::ptr_eq(a_dnode, b_dnode));
    kassert!(same(a_dnode.vnode(), Some(a_dvp)));
    kassert!(same(b_dnode.vnode(), Some(b_dvp)));
    kassert!(a_dnode.tn_type == VDIR);
    kassert!(b_dnode.tn_type == VDIR);
    kassert!(a_missing_ok != b_missing_ok);

    tmpfs_rename_lock_directory(a_dvp, a_dnode)?; // fail0

    let locked: Result<RenameLocked<G>, Errno> = 'fail1: {
        // Did we lose a race with mount?
        if a_dvp.mountedhere().is_some() {
            break 'fail1 Err(Errno::EBUSY);
        }

        // Make sure the caller may read the directory.
        if let Err(e) = vop_access(a_dvp, VEXEC, cred, p) {
            break 'fail1 Err(e);
        }

        let a_dirent = tmpfs_dir_lookup(a_dnode, a_cnp);
        let a_node = a_dirent.as_ref().map(|de| de.node());
        let a_vp = match &a_node {
            Some(a_node) => {
                // We ruled out `.' earlier.
                kassert!(!TmpfsNodeRef::ptr_eq(a_node, a_dnode));
                // We ruled out `..' earlier.
                kassert!(!a_dnode.dir().tn_parent.points_to(a_node));
                if TmpfsNodeRef::ptr_eq(a_node, b_dnode) {
                    break 'fail1 Err(overlap_error);
                }
                let nlock = a_node.tn_nlock.write();
                let a_vp = match tmpfs_vnode_get(mount, a_node, nlock) {
                    Ok(a_vp) => a_vp,
                    Err(e) => break 'fail1 Err(e),
                };
                kassert!(same(a_vp.mount(), Some(mount)));
                // Refuse to rename (over) a mount point.
                if a_vp.vtype() == VDIR && a_vp.mountedhere().is_some() {
                    // fail2:
                    kassert!(vop_islocked(a_vp) == LK_EXCLUSIVE);
                    vput(a_vp);
                    break 'fail1 Err(Errno::EBUSY);
                }
                Some(a_vp)
            }
            None if !a_missing_ok => break 'fail1 Err(Errno::ENOENT),
            None => None,
        };
        kassert!(!same(a_vp, Some(a_dvp)));
        kassert!(!same(a_vp, Some(b_dvp)));

        let locked: Result<RenameLocked<G>, Errno> = 'fail2: {
            if let Err(e) = tmpfs_rename_lock_directory(b_dvp, b_dnode) {
                break 'fail2 Err(e);
            }

            let locked: Result<RenameLocked<G>, Errno> = 'fail3: {
                // Did we lose a race with mount?
                if b_dvp.mountedhere().is_some() {
                    break 'fail3 Err(Errno::EBUSY);
                }

                // Make sure the caller may read the directory.
                if let Err(e) = vop_access(b_dvp, VEXEC, cred, p) {
                    break 'fail3 Err(e);
                }

                let b_dirent = tmpfs_dir_lookup(b_dnode, b_cnp);
                let b_vp = match &b_dirent {
                    Some(b_de) => {
                        let b_node = b_de.node();
                        // We ruled out `.' earlier.
                        kassert!(!TmpfsNodeRef::ptr_eq(&b_node, b_dnode));
                        // We ruled out `..' earlier.
                        kassert!(!b_dnode.dir().tn_parent.points_to(&b_node));
                        // b is not an ancestor of a.
                        kassert!(!TmpfsNodeRef::ptr_eq(&b_node, a_dnode));
                        // But the source and target nodes might be the same.
                        if is_node(&b_node, a_node.as_ref()) {
                            // a_dirent names the node, which tmpfs_vnode_get gave a vnode.
                            let Some(a_vp) = a_vp else {
                                kpanic(format_args!("tmpfs_rename_lock: no vnode for a"));
                            };
                            vref(a_vp);
                            Some(a_vp)
                        } else {
                            let nlock = b_node.tn_nlock.write();
                            let b_vp = match tmpfs_vnode_get(mount, &b_node, nlock) {
                                Ok(b_vp) => b_vp,
                                Err(e) => break 'fail3 Err(e),
                            };
                            kassert!(same(b_vp.mount(), Some(mount)));
                            kassert!(!same(a_vp, Some(b_vp)));
                            // Refuse to rename (over) a mount point.
                            if b_vp.vtype() == VDIR && b_vp.mountedhere().is_some() {
                                // fail4: b_vp is not a_vp here.
                                kassert!(vop_islocked(b_vp) == LK_EXCLUSIVE);
                                vput(b_vp);
                                break 'fail3 Err(Errno::EBUSY);
                            }
                            Some(b_vp)
                        }
                    }
                    None if !b_missing_ok => break 'fail3 Err(Errno::ENOENT),
                    None => None,
                };
                kassert!(!same(b_vp, Some(a_dvp)));
                kassert!(!same(b_vp, Some(b_dvp)));

                kassert!(vop_islocked(a_dvp) == LK_EXCLUSIVE);
                kassert!(vop_islocked(b_dvp) == LK_EXCLUSIVE);
                kassert!(a_missing_ok || a_dirent.is_some());
                kassert!(b_missing_ok || b_dirent.is_some());
                kassert!(
                    a_dirent
                        .as_ref()
                        .is_none_or(|de| same(de.node().vnode(), a_vp))
                );
                kassert!(
                    b_dirent
                        .as_ref()
                        .is_none_or(|de| same(de.node().vnode(), b_vp))
                );
                kassert!(a_vp.is_none_or(|vp| vop_islocked(vp) == LK_EXCLUSIVE));
                kassert!(b_vp.is_none_or(|vp| vop_islocked(vp) == LK_EXCLUSIVE));

                Ok(RenameLocked {
                    a_dirent,
                    a_vp,
                    b_dirent,
                    b_vp,
                })
            };
            if locked.is_err() {
                // fail3:
                kassert!(vop_islocked(b_dvp) == LK_EXCLUSIVE);
                let _ = vop_unlock(b_dvp);
            }
            locked
        };
        if locked.is_err()
            && let Some(a_vp) = a_vp
        {
            // fail2:
            kassert!(vop_islocked(a_vp) == LK_EXCLUSIVE);
            vput(a_vp);
        }
        locked
    };
    if locked.is_err() {
        // fail1:
        kassert!(vop_islocked(a_dvp) == LK_EXCLUSIVE);
        let _ = vop_unlock(a_dvp);
    }

    // fail0:
    // KASSERT(VOP_ISLOCKED(a_dvp) != LK_EXCLUSIVE);
    // KASSERT(VOP_ISLOCKED(b_dvp) != LK_EXCLUSIVE);
    // KASSERT((a_vp == NULL) || (VOP_ISLOCKED(a_vp) != LK_EXCLUSIVE));
    // KASSERT((b_vp == NULL) || (VOP_ISLOCKED(b_vp) != LK_EXCLUSIVE));
    locked
}

/// `tmpfs_rename_attachdetach`: shuffle the directory entries to move `fvp` from the
/// directory `fdvp` into the directory `tdvp`. `fde` is `fvp`'s directory entry in `fdvp`.
/// If we are overwriting a target node, it is `tvp`, and `tde` is its directory entry in
/// `tdvp`, which goes.
///
/// `fdvp`, `fvp`, `tdvp`, and `tvp` must all be locked and referenced.
fn tmpfs_rename_attachdetach<G: ProcGlue>(
    tmpfs: &TmpfsMount<G>,
    fdvp: &'static Vnode<G>,
    fde: &TmpfsDirentRef<G>,
    fvp: &'static Vnode<G>,
    tdvp: &'static Vnode<G>,
    tde: Option<TmpfsDirentRef<G>>,
    tvp: Option<&'static Vnode<G>>,
) {
    kassert!(!fde.td_node_is_none());
    kassert!(same(fde.node().vnode(), Some(fvp)));
    kassert!(tde.is_none() == tvp.is_none());
    kassert!(tde.as_ref().is_none_or(|tde| !tde.td_node_is_none()));
    kassert!(tde.as_ref().is_none_or(|tde| same(tde.node().vnode(), tvp)));
    kassert!(vop_islocked(fdvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tdvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(fvp) == LK_EXCLUSIVE);
    kassert!(tvp.is_none_or(|tvp| vop_islocked(tvp) == LK_EXCLUSIVE));

    // If we are moving from one directory to another, detach the source entry and reattach
    // it to the target directory.
    if !ptr::eq(fdvp, tdvp) {
        // tmpfs_dir_detach clobbers fde->td_node, so save it.
        let fnode = fde.node();
        let fdnode = vptodir(fdvp);
        let tdnode = vptodir(tdvp);
        tmpfs_dir_detach(&fdnode, fde);
        tmpfs_dir_attach(&tdnode, TmpfsDirentRef::clone(fde), fnode);
    } else if tvp.is_none() {
        // We are changing the directory. tmpfs_dir_attach and tmpfs_dir_detach note the
        // events for us, but for this case we don't call them, so we must note the event
        // explicitly.
        VN_KNOTE(fdvp, NOTE_WRITE);
    }

    // If we are replacing an existing target entry, delete it.
    if let Some(tde) = tde {
        let tdnode = vptodir(tdvp);
        let tnode = tde.node();
        kassert!(tvp.is_some_and(|tvp| (fvp.vtype() == VDIR) == (tvp.vtype() == VDIR)));
        if tnode.tn_type == VDIR {
            kassert!(tnode.tn_size.get() == 0);
            kassert!(tnode.tn_links.get() == 2);
            // Decrement the extra link count for `.' so the vnode will be recycled when
            // released.
            tnode.tn_links.set(tnode.tn_links.get() - 1);
        }
        tmpfs_dir_detach(&tdnode, &tde);
        tmpfs_free_dirent(tmpfs, tde);
    }
}

/// `tmpfs_do_remove`: remove the entry `de` for the non-directory `vp` from the directory
/// `dvp`.
///
/// Everything must be locked and referenced.
fn tmpfs_do_remove<G: ProcGlue>(
    tmpfs: &TmpfsMount<G>,
    dvp: &'static Vnode<G>,
    dnode: &TmpfsNodeRef<G>,
    de: TmpfsDirentRef<G>,
    vp: &'static Vnode<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    kassert!(same(dnode.vnode(), Some(dvp)));
    kassert!(!de.td_node_is_none());
    kassert!(same(de.node().vnode(), Some(vp)));
    kassert!(vop_islocked(dvp) == LK_EXCLUSIVE);
    kassert!(vop_islocked(vp) == LK_EXCLUSIVE);

    let node = de.node();
    tmpfs_remove_check_possible(dnode, &node)?;

    tmpfs_remove_check_permitted(cred, p, dnode, &node)?;

    // If not root and directory is sticky, check for permission on directory or on file.
    // This implements append-only directories.
    if dnode.tn_mode.get() & S_ISTXT != 0 {
        let uid = ucred(cred).cr_uid.get();
        if uid != 0 && uid != dnode.tn_uid.get() && uid != node.tn_uid.get() {
            return Err(Errno::EPERM);
        }
    }

    tmpfs_dir_detach(dnode, &de);
    tmpfs_free_dirent(tmpfs, de);

    Ok(())
}

/// `tmpfs_rename_check_possible`: check whether a rename is possible independent of
/// credentials.
///
/// Everything must be locked and referenced.
fn tmpfs_rename_check_possible<G: ProcGlue>(
    fdnode: &TmpfsNodeRef<G>,
    fnode: &TmpfsNodeRef<G>,
    tdnode: &TmpfsNodeRef<G>,
    tnode: Option<&TmpfsNodeRef<G>>,
) -> Result<(), Errno> {
    kassert!(!TmpfsNodeRef::ptr_eq(fdnode, fnode));
    kassert!(!is_node(tdnode, tnode));
    kassert!(!is_node(fnode, tnode));
    kassert!(fdnode.vnode().is_some());
    kassert!(fnode.vnode().is_some());
    kassert!(tdnode.vnode().is_some());
    kassert!(tnode.is_none_or(|tnode| tnode.vnode().is_some()));
    kassert!(vop_islocked(tn_vnode(fdnode)) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tn_vnode(fnode)) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tn_vnode(tdnode)) == LK_EXCLUSIVE);
    kassert!(tnode.is_none_or(|tnode| vop_islocked(tn_vnode(tnode)) == LK_EXCLUSIVE));

    // If fdnode is immutable, we can't write to it. If fdnode is append-only, the only
    // change we can make is to add entries to it. If fnode is immutable, we can't change
    // the links to it. If fnode is append-only...well, this is what UFS does.
    if (fdnode.tn_flags.get() | fnode.tn_flags.get()) & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }

    // If tdnode is immutable, we can't write to it. If tdnode is append-only, we can add
    // entries, but we can't change existing entries.
    let append = if tnode.is_some() { APPEND } else { 0 };
    if tdnode.tn_flags.get() & (IMMUTABLE | append) != 0 {
        return Err(Errno::EPERM);
    }

    // If tnode is immutable, we can't replace links to it. If tnode is append-only...well,
    // this is what UFS does.
    if let Some(tnode) = tnode
        && tnode.tn_flags.get() & (IMMUTABLE | APPEND) != 0
    {
        return Err(Errno::EPERM);
    }

    Ok(())
}

/// `tmpfs_rename_check_permitted`: check whether a rename is permitted given our
/// credentials.
///
/// Everything must be locked and referenced.
fn tmpfs_rename_check_permitted<G: ProcGlue>(
    cred: Cred<'_>,
    p: &Thread<G>,
    fdnode: &TmpfsNodeRef<G>,
    fnode: &TmpfsNodeRef<G>,
    tdnode: &TmpfsNodeRef<G>,
    tnode: Option<&TmpfsNodeRef<G>>,
) -> Result<(), Errno> {
    kassert!(!TmpfsNodeRef::ptr_eq(fdnode, fnode));
    kassert!(!is_node(tdnode, tnode));
    kassert!(!is_node(fnode, tnode));
    kassert!(fdnode.vnode().is_some());
    kassert!(fnode.vnode().is_some());
    kassert!(tdnode.vnode().is_some());
    kassert!(tnode.is_none_or(|tnode| tnode.vnode().is_some()));
    kassert!(vop_islocked(tn_vnode(fdnode)) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tn_vnode(fnode)) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tn_vnode(tdnode)) == LK_EXCLUSIVE);
    kassert!(tnode.is_none_or(|tnode| vop_islocked(tn_vnode(tnode)) == LK_EXCLUSIVE));

    // We need to remove or change an entry in the source directory.
    vop_access(tn_vnode(fdnode), VWRITE, cred, p)?;

    // If we are changing directories, then we need to write to the target directory to add
    // or change an entry. Also, if fnode is a directory, we need to write to it to change
    // its `..' entry.
    if !TmpfsNodeRef::ptr_eq(fdnode, tdnode) {
        vop_access(tn_vnode(tdnode), VWRITE, cred, p)?;
        if fnode.tn_type == VDIR {
            vop_access(tn_vnode(fnode), VWRITE, cred, p)?;
        }
    }

    tmpfs_check_sticky(cred, fdnode, Some(fnode))?;

    if tmpfs_dirseq_full(tdnode) {
        return Err(Errno::ENOSPC);
    }

    tmpfs_check_sticky(cred, tdnode, tnode)?;

    Ok(())
}

/// `tmpfs_remove_check_possible`: check whether removing `node`'s entry in `dnode` is
/// possible independent of credentials.
///
/// Everything must be locked and referenced.
fn tmpfs_remove_check_possible<G: ProcGlue>(
    dnode: &TmpfsNodeRef<G>,
    node: &TmpfsNodeRef<G>,
) -> Result<(), Errno> {
    kassert!(dnode.vnode().is_some());
    kassert!(!TmpfsNodeRef::ptr_eq(dnode, node));
    kassert!(vop_islocked(tn_vnode(dnode)) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tn_vnode(node)) == LK_EXCLUSIVE);

    // We want to delete the entry. If dnode is immutable, we can't write to it to delete
    // the entry. If dnode is append-only, the only change we can make is to add entries, so
    // we can't delete entries. If node is immutable, we can't change the links to it, so we
    // can't delete the entry. If node is append-only...well, this is what UFS does.
    if (dnode.tn_flags.get() | node.tn_flags.get()) & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }

    Ok(())
}

/// `tmpfs_remove_check_permitted`: check whether removing `node`'s entry in `dnode` is
/// permitted given our credentials.
///
/// Everything must be locked and referenced.
fn tmpfs_remove_check_permitted<G: ProcGlue>(
    cred: Cred<'_>,
    p: &Thread<G>,
    dnode: &TmpfsNodeRef<G>,
    node: &TmpfsNodeRef<G>,
) -> Result<(), Errno> {
    kassert!(dnode.vnode().is_some());
    kassert!(!TmpfsNodeRef::ptr_eq(dnode, node));
    kassert!(vop_islocked(tn_vnode(dnode)) == LK_EXCLUSIVE);
    kassert!(vop_islocked(tn_vnode(node)) == LK_EXCLUSIVE);

    // Check whether we are permitted to write to the source directory in order to delete an
    // entry from it.
    vop_access(tn_vnode(dnode), VWRITE, cred, p)?;

    tmpfs_check_sticky(cred, dnode, Some(node))?;

    Ok(())
}

/// `tmpfs_check_sticky`: check whether we may change an entry in a sticky directory. If the
/// directory is sticky, the user must own either the directory or, if it exists, the node,
/// in order to change the entry.
///
/// Everything must be locked and referenced.
fn tmpfs_check_sticky<G: ProcGlue>(
    cred: Cred<'_>,
    dnode: &TmpfsNode<G>,
    node: Option<&TmpfsNodeRef<G>>,
) -> Result<(), Errno> {
    kassert!(dnode.vnode().is_some());
    kassert!(vop_islocked(tn_vnode(dnode)) == LK_EXCLUSIVE);
    kassert!(node.is_none_or(|node| node.vnode().is_some()));
    kassert!(node.is_none() || vop_islocked(tn_vnode(dnode)) == LK_EXCLUSIVE);

    let Some(node) = node else {
        return Ok(());
    };

    if dnode.tn_mode.get() & S_ISTXT != 0 {
        let uid = ucred(cred).cr_uid.get();
        if uid != 0 && uid != dnode.tn_uid.get() && uid != node.tn_uid.get() {
            return Err(Errno::EPERM);
        }
    }

    Ok(())
}

/// `tmpfs_rename_cache_purge`: purge the name cache of the vnodes a rename changed.
fn tmpfs_rename_cache_purge<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
) {
    kassert!(!ptr::eq(fdvp, fvp));
    kassert!(!same(Some(fdvp), tvp));
    kassert!(!ptr::eq(tdvp, fvp));
    kassert!(!same(Some(tdvp), tvp));
    kassert!(!same(Some(fvp), tvp));
    kassert!(fdvp.vtype() == VDIR);
    kassert!(tdvp.vtype() == VDIR);

    // XXX What actually needs to be purged?

    cache_purge(fdvp);

    if fvp.vtype() == VDIR {
        cache_purge(fvp);
    }

    if !ptr::eq(tdvp, fdvp) {
        cache_purge(tdvp);
    }

    if let Some(tvp) = tvp
        && tvp.vtype() == VDIR
    {
        cache_purge(tvp);
    }
}

/// `tmpfs_rename_abort`: abort both lookups of a rename and release every vnode.
fn tmpfs_rename_abort<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) {
    let _ = vop_abortop(tdvp, tcnp);
    if same(Some(tdvp), tvp) {
        vrele(tdvp);
    } else {
        vput(tdvp);
    }
    if let Some(tvp) = tvp {
        vput(tvp);
    }
    let _ = vop_abortop(fdvp, fcnp);
    vrele(fdvp);
    vrele(fvp);
}

/// `tmpfs_kqfilter` (`vop_kqfilter`): attach a knote to the vnode.
pub fn tmpfs_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&TmpfsFiltops::<G>::READ)),
        EVFILT_WRITE => kn.kn_fop.set(Some(&TmpfsFiltops::<G>::WRITE)),
        EVFILT_VNODE => kn.kn_fop.set(Some(&TmpfsFiltops::<G>::VNODE)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = (caddr_t)vp: a fresh knote holds no hook yet (as `ufs_kqfilter`'s).
    let _ = kn.set_hook_vnode(vp);

    vp.klist.insert_legacy(kn);

    Ok(())
}

/// `kn->kn_hook` of a tmpfs knote: its vnode.
fn kn_vnode<G: ProcGlue>(kn: &Knote<G>) -> &'static Vnode<G> {
    match kn.hook_vnode() {
        Some(vp) => vp,
        None => kpanic(format_args!("knote {kn:p}: no vnode")),
    }
}

/// `filt_tmpfsdetach`: unhooks the knote from the vnode.
fn filt_tmpfsdetach<G: ProcGlue>(kn: &Knote<G>) {
    let vp = kn_vnode(kn);

    vp.klist.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for TmpfsRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_tmpfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_tmpfsdetach(kn);
    }

    /// `filt_tmpfsread`: the bytes past the file offset; always ready for poll and select.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let vp = kn_vnode(kn);

        // filesystem is gone, so set the EOF flag and schedule the knote for deletion. The
        // vnode has no node any more (the module's redesign): it is reached only past this
        // test.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }
        let node = vptonode(vp);

        kn.kn_data()
            .set(node.tn_size.get().wrapping_sub(foffset(&kn.fp())));
        if kn.kn_data().get() == 0 && kn.kn_sfflags.get() & NOTE_EOF != 0 {
            kn.kn_fflags().set(kn.kn_fflags().get() | NOTE_EOF);
            return true;
        }

        if kn.has_flags(__EV_POLL | __EV_SELECT) {
            return true;
        }

        kn.kn_data().get() != 0
    }
}

impl<G: ProcGlue> Filter<G> for TmpfsWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_tmpfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_tmpfsdetach(kn);
    }

    /// `filt_tmpfswrite`: a file is always writable.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        // filesystem is gone, so set the EOF flag and schedule the knote for deletion.
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }

        kn.kn_data().set(0);
        true
    }
}

impl<G: ProcGlue> Filter<G> for TmpfsVnode {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_tmpfsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_tmpfsdetach(kn);
    }

    /// `filt_tmpfsvnode`: records the vnode events (`NOTE_*`) the user asked for.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let hint32 = hint as u32;
        if kn.kn_sfflags.get() & hint32 != 0 {
            kn.kn_fflags().set(kn.kn_fflags().get() | hint32);
        }
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF);
            return true;
        }
        kn.kn_fflags().get() != 0
    }
}

/// The nine slots `tmpfs_vops`, `tmpfs_specvops` and `tmpfs_fifovops` share: `access`,
/// `getattr`, `setattr`, `inactive`, `reclaim`, `lock`, `unlock`, `print` and `islocked` are
/// tmpfs's in the three (`impl Vnops<G>` bodies of a glue `G: ProcGlue`).
macro_rules! tmpfs_common_slots {
    () => {
        fn access(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
            mode: i32,
            cred: crate::fs::vops::Cred<'_>,
            _p: &crate::proc::Thread<G>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_access(vp, mode, cred)
        }
        fn getattr(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
            vap: &mut crate::fs::vnode::Vattr,
            _cred: crate::fs::vops::Cred<'_>,
            _p: &crate::proc::Thread<G>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_getattr(vp, vap)
        }
        fn setattr(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
            vap: &mut crate::fs::vnode::Vattr,
            cred: crate::fs::vops::Cred<'_>,
            p: &crate::proc::Thread<G>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_setattr(vp, vap, cred, p)
        }
        fn inactive(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
            p: Option<&crate::proc::Thread<G>>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_inactive(vp, p)
        }
        fn reclaim(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
            _p: Option<&crate::proc::Thread<G>>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_reclaim(vp)
        }
        fn lock(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
            flags: i32,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_lock(vp, flags)
        }
        fn unlock(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_unlock(vp)
        }
        fn print(
            &self,
            vp: &'static crate::fs::vnode::Vnode<G>,
        ) -> Result<(), hal::sys::errno::Errno> {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_print(vp)
        }
        fn islocked(&self, vp: &'static crate::fs::vnode::Vnode<G>) -> i32 {
            crate::fs::tmpfs::tmpfs_vnops::tmpfs_islocked(vp)
        }
    };
}
pub(super) use tmpfs_common_slots;

/// `tmpfs_vops`: each slot the C's (34).
impl<G: ProcGlue> Vnops<G> for TmpfsVnops<G> {
    tmpfs_common_slots!();

    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        tmpfs_lookup(dvp, vpp, cnp)
    }
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        tmpfs_create(dvp, vpp, cnp, vap)
    }
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        tmpfs_mknod(dvp, vpp, cnp, vap)
    }
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_open(vp, mode)
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        tmpfs_close(vp)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_read(vp, uio)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_write(vp, uio, ioflag)
    }
    fn ioctl(
        &self,
        _vp: &'static Vnode<G>,
        _command: u64,
        _data: &mut [u8],
        _fflag: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_ioctl()
    }
    fn kqfilter(&self, vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        tmpfs_kqfilter(vp, kn)
    }
    fn revoke(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        default::vop_generic_revoke(vp, flags)
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        _waitfor: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        tmpfs_fsync(vp)
    }
    fn remove(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        tmpfs_remove(dvp, vp, cnp)
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        tmpfs_link(dvp, vp, cnp)
    }
    fn rename(
        &self,
        fdvp: &'static Vnode<G>,
        fvp: &'static Vnode<G>,
        fcnp: &mut Cn<'_, G>,
        tdvp: &'static Vnode<G>,
        tvp: Option<&'static Vnode<G>>,
        tcnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        tmpfs_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
    }
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        tmpfs_mkdir(dvp, vpp, cnp, vap)
    }
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        tmpfs_rmdir(dvp, vp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        tmpfs_symlink(dvp, vpp, cnp, vap, target)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        tmpfs_readdir(vp, uio, eofflag)
    }
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        tmpfs_readlink(vp, uio)
    }
    fn abortop(&self, _dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_abortop(cnp)
    }
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        default::vop_generic_bmap(vp, bn, vpp, bnp, runp)
    }
    fn strategy(&self, _vp: &'static Vnode<G>, _bp: IoBuf) -> Result<(), Errno> {
        tmpfs_strategy()
    }
    fn pathconf(
        &self,
        _vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        tmpfs_pathconf(name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        tmpfs_advlock(vp, id, op, fl, flags)
    }
    fn bwrite(&self, _bp: IoBuf) -> Result<(), Errno> {
        tmpfs_bwrite()
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the vnode operations, through the system calls' own paths (`namei`,
    // `vn_open`, `sys_mkdir`, `sys_rename`, ...) over a tmpfs made the root (testtmpfs's
    // `setup_root`): LZ's eight moved (a file's life, the arguments' checks, directories,
    // renames, hard links, symbolic links, the recursive lock under vnd's read, pathconf and
    // the filters), and, new since the window (D-N5.9-11), the bytes of files through
    // `read(2)`/`write(2)` and `vn_rdwr`; readdir's cookies; `readlink(2)`'s user leg and
    // `namei`'s kernel one; the tables' addresses, a device node's table, the fifo refusal
    // and the fifo table's gaps; `statfs`'s counts of the C's sizes; the filters through
    // `kevent(2)`. User memory: testfs's path slots and buffer, under the guard.

    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::dirent::Dirent;
    use hal::sys::fcntl::{AT_FDCWD, AT_REMOVEDIR, FREAD, O_CREAT, O_RDWR};
    use hal::sys::mount::Statfs;
    use hal::sys::stat::{S_IFCHR, S_IFIFO};
    use hal::sys::types::makedev;
    use hal::user::UserPtr;

    use super::*;
    use crate::descrip::{sys_close, sys_read, sys_write};
    use crate::event::{EV_ADD, Kevent, NOTE_DELETE, kqueue_init, sys_kevent, sys_kqueue};
    use crate::fs::lookup::{namei, ndinit};
    use crate::fs::namei::{FOLLOW, LOCKLEAF, LOOKUP, NiDirp};
    use crate::fs::subr::{vfs, vops_id};
    use crate::fs::syscalls::{
        dounlinkat, sys_link, sys_mkdir, sys_mkfifo, sys_mknod, sys_open, sys_readlink, sys_rename,
        sys_symlink, sys_unlink,
    };
    use crate::fs::testfs::{TestGlue, UBUF, up};
    use crate::fs::tmpfs::testtmpfs::{self, setup_root};
    use crate::fs::tmpfs::tmpfs::{TMPFS_DIRENT_CHARGE, TMPFS_NODE_CHARGE};
    use crate::fs::tmpfs::tmpfs_subr::tmpfs_alloc_node;
    use crate::fs::tmpfs::tmpfs_vfsops::tmpfs_statfs;
    use crate::fs::vnode::LK_RECURSEFAIL;
    use crate::fs::vnops::{RdwrBuf, vn_close, vn_open, vn_rdwr};
    use crate::fs::vops::{
        vop_getattr, vop_kqfilter, vop_open, vop_pathconf, vop_readdir, vop_setattr,
    };
    use crate::uio::Iovec;

    type V = &'static Vnode<TestGlue>;
    type P = &'static Thread<TestGlue>;

    /// `s` as the first path argument (testfs's slot 0).
    fn c(s: &str) -> UserPtr<u8> {
        up(0, s.as_bytes())
    }

    /// `s` as the second path argument (slot 1).
    fn c2(s: &str) -> UserPtr<u8> {
        up(1, s.as_bytes())
    }

    /// `open(path, O_RDWR | O_CREAT, mode)`: the vnode, unlocked and referenced.
    fn create(p: P, path: &[u8], mode: Mode) -> V {
        let mut nd = ndinit(0, 0, NiDirp::Sys(path), p);
        vn_open(&mut nd, FREAD | FWRITE | O_CREAT, mode).expect("create");
        let vp = nd.ni_vp.expect("a vnode");
        let _ = vop_unlock(vp);
        vp
    }

    /// `close` of a vnode `create` gave.
    fn close(p: P, vp: V) {
        vn_close(vp, FREAD | FWRITE, Cred::real(p.ucred()), Some(p)).expect("close");
    }

    /// The vnode `path` names, unlocked and referenced (symbolic links followed).
    fn lookup(p: P, path: &[u8]) -> Result<V, Errno> {
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd)?;
        Ok(nd.ni_vp.expect("a vnode"))
    }

    /// `lstat(path)`: the attributes of the node `path` names.
    fn stat(p: P, path: &[u8]) -> Result<Vattr, Errno> {
        let mut nd = ndinit(LOOKUP, LOCKLEAF, NiDirp::Sys(path), p);
        namei(&mut nd)?;
        let vp = nd.ni_vp.expect("a vnode");
        let mut va = Vattr::new();
        let error = vop_getattr(vp, &mut va, Cred::real(p.ucred()), p);
        vput(vp);
        error.map(|()| va)
    }

    /// One `VOP_READDIR` of the directory `path` from the cookie `offset` into `size`
    /// bytes: the result, each entry's name and `d_off`, the new offset, the EOF flag.
    fn readdir(
        p: P,
        path: &[u8],
        offset: Off,
        size: usize,
    ) -> (Result<(), Errno>, Vec<(Vec<u8>, Off)>, Off, i32) {
        let mut nd = ndinit(LOOKUP, LOCKLEAF | FOLLOW, NiDirp::Sys(path), p);
        namei(&mut nd).expect("lookup");
        let vp = nd.ni_vp.expect("a vnode");
        let mut buf = vec![0u8; size];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut buf, Some(p));
        uio.uio_offset = offset;
        let mut eof = 0;
        let r = vop_readdir(vp, &mut uio, Cred::real(p.ucred()), &mut eof);
        let (off, used) = (uio.uio_offset, size - uio.uio_resid);
        vput(vp);
        let mut out = Vec::new();
        let mut at = 0;
        while at < used {
            let d = Dirent::from_bytes(&buf[at..]).expect("a dirent");
            let name = &buf[at + Dirent::NAME_OFFSET..][..usize::from(d.d_namlen)];
            out.push((name.to_vec(), d.d_off));
            at += usize::from(d.d_reclen);
        }
        (r, out, off, eof)
    }

    /// The names `getdents` lists in the directory `path`, read in one call to its end.
    fn names(p: P, path: &[u8]) -> Vec<Vec<u8>> {
        let (r, ents, _, eof) = readdir(p, path, 0, 4096);
        assert_eq!((r, eof), (Ok(()), 1), "one call reads a small directory");
        ents.into_iter().map(|(n, _)| n).collect()
    }

    /// `truncate(vp, size)` through `VOP_SETATTR`.
    fn truncate(p: P, vp: V, size: u64) -> Result<(), Errno> {
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_size = size;
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
        let error = vop_setattr(vp, &mut va, Cred::real(p.ucred()), p);
        let _ = vop_unlock(vp);
        error
    }

    /// `vn_rdwr` of `buf` at `off` (a kernel buffer), the bytes not moved.
    fn rdwr(p: P, rw: UioRw, vp: V, buf: &mut [u8], off: Off) -> Result<usize, Errno> {
        let mut resid = 0;
        let b = if rw == UioRw::UIO_WRITE {
            RdwrBuf::KernelRead(&buf[..])
        } else {
            RdwrBuf::Kernel(&mut buf[..])
        };
        let procp = if rw == UioRw::UIO_WRITE {
            None
        } else {
            Some(p)
        };
        vn_rdwr(
            rw,
            vp,
            b,
            off,
            0,
            Cred::real(p.ucred()),
            Some(&mut resid),
            procp,
        )?;
        Ok(resid)
    }

    /// LZ's first test and the bytes: a file is created, written and read through
    /// `write(2)`/`read(2)` (the window's user legs) and `vn_rdwr` (its kernel legs), resized
    /// by whole and partial pages (the tail zeroed), refused an unsettable attribute,
    /// opened twice as one vnode, and removed while open.
    #[test]
    fn a_file_is_created_written_read_resized_and_removed() {
        let (_g, p, _mp) = setup_root();
        let vp = create(p, b"/f", 0o640);

        let va = stat(p, b"/f").expect("stat");
        assert_eq!(va.va_type, VREG);
        assert_eq!(va.va_mode, 0o640);
        assert_eq!((va.va_nlink, va.va_size, va.va_bytes), (1, 0, 0));
        assert_eq!(va.va_fileid, vptonode(vp).tn_id.get());
        assert_eq!(va.va_blocksize, PAGE_SIZE as i64);
        assert_eq!(va.va_rdev, VNOVAL as hal::sys::types::Dev);
        assert_eq!(
            stat(p, b"/").expect("stat /").va_size,
            TMPFS_DIRENT_CHARGE as u64,
            "one entry of the C's size"
        );

        // write(2) and read(2) through a descriptor: user iovecs.
        let fd = sys_open(p, c("/f"), O_RDWR, 0).expect("open");
        UserPtr::<u8>::new(UBUF)
            .copyout_slice(b"hello, tmpfs")
            .unwrap();
        assert_eq!(sys_write(p, fd, UBUF, 12), Ok(12));
        let mut back = [0u8; 12];
        assert_eq!(rdwr(p, UioRw::UIO_READ, vp, &mut back, 0), Ok(0));
        assert_eq!(&back, b"hello, tmpfs");
        // A kernel write across a page boundary, read back by read(2).
        let mut pat: Vec<u8> = (0..300).map(|i| (i % 251) as u8).collect();
        let at = PAGE_SIZE as Off - 100;
        assert_eq!(rdwr(p, UioRw::UIO_WRITE, vp, &mut pat, at), Ok(0));
        assert_eq!(stat(p, b"/f").expect("stat").va_size, at as u64 + 300);
        sys_close(p, fd).expect("close");
        let fd = sys_open(p, c("/f"), O_RDWR, 0).expect("open");
        let mut n = 0;
        loop {
            let got = sys_read(p, fd, UBUF + n, 1000).expect("read");
            if got == 0 {
                break;
            }
            n += got;
        }
        assert_eq!(n, at as usize + 300);
        let mut all = vec![0u8; n];
        UserPtr::<u8>::new(UBUF).copyin_slice(&mut all).unwrap();
        assert_eq!(&all[..12], b"hello, tmpfs");
        assert!(
            all[12..at as usize].iter().all(|&b| b == 0),
            "a hole reads zeros"
        );
        assert_eq!(&all[at as usize..], &pat[..]);
        sys_close(p, fd).expect("close");

        // A partial page is zeroed when the file shrinks into it and grows again.
        truncate(p, vp, 5).expect("shrink");
        truncate(p, vp, 3 * PAGE_SIZE as u64).expect("grow");
        let va = stat(p, b"/f").expect("stat");
        assert_eq!(
            (va.va_size, va.va_bytes),
            (3 * PAGE_SIZE as u64, 3 * PAGE_SIZE as u64)
        );
        let mut head = [0xffu8; 16];
        assert_eq!(rdwr(p, UioRw::UIO_READ, vp, &mut head, 0), Ok(0));
        assert_eq!(&head[..5], b"hello");
        assert!(head[5..].iter().all(|&b| b == 0));
        truncate(p, vp, PAGE_SIZE as u64).expect("shrink");
        assert_eq!(stat(p, b"/f").expect("stat").va_size, PAGE_SIZE as u64);

        // unsettable attributes
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_nlink = 3;
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
        assert_eq!(
            vop_setattr(vp, &mut va, Cred::real(p.ucred()), p),
            Err(Errno::EINVAL)
        );
        let _ = vop_unlock(vp);

        // a second open finds the same vnode; O_CREAT on an existing file opens it
        let again = create(p, b"/f", 0o600);
        assert!(ptr::eq(again, vp));
        close(p, again);

        assert_eq!(names(p, b"/"), [&b"."[..], b"..", b"f"]);
        sys_unlink(p, c("/f")).expect("unlink");
        assert_eq!(stat(p, b"/f").err(), Some(Errno::ENOENT));
        assert_eq!(names(p, b"/"), [&b"."[..], b".."]);
        assert_eq!(vptonode(vp).tn_links.get(), 0, "open but nameless");
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
        assert_eq!(
            vop_open(vp, FREAD, Cred::real(p.ucred()), p),
            Err(Errno::ENOENT),
            "a file without names cannot be opened again"
        );
        let _ = vop_unlock(vp);
        close(p, vp);
    }

    /// LZ's: empty reads and writes, `EFBIG` past the largest offset, a directory neither
    /// read (`EISDIR`) nor written (`EINVAL`); and a negative offset `EINVAL`.
    #[test]
    fn reads_and_writes_check_their_arguments() {
        let (_g, p, _mp) = setup_root();
        let vp = create(p, b"/f", 0o644);
        let mut byte = [0u8; 1];
        assert_eq!(rdwr(p, UioRw::UIO_READ, vp, &mut byte, 0), Ok(1));
        assert_eq!(rdwr(p, UioRw::UIO_WRITE, vp, &mut byte[..0], 0), Ok(0));
        assert_eq!(
            rdwr(p, UioRw::UIO_WRITE, vp, &mut byte, i64::MAX),
            Err(Errno::EFBIG)
        );
        assert_eq!(vptonode(vp).tn_size.get(), 0);
        // A negative offset.
        let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
        let mut iov = [Iovec::new()];
        let mut uio = Uio::kernel_read(&mut iov, &mut byte, Some(p));
        uio.uio_offset = -1;
        assert_eq!(tmpfs_read(vp, &mut uio), Err(Errno::EINVAL));
        let _ = vop_unlock(vp);
        close(p, vp);

        let root = lookup(p, b"/").expect("lookup /");
        assert_eq!(
            rdwr(p, UioRw::UIO_READ, root, &mut byte, 0),
            Err(Errno::EISDIR)
        );
        assert_eq!(
            rdwr(p, UioRw::UIO_WRITE, root, &mut byte, 0),
            Err(Errno::EINVAL)
        );
        vrele(root);
    }

    /// LZ's directories (made, listed, refused, removed, the link counts and `..`), and
    /// readdir's cookies: `.` points at `..` (1), `..` at the first entry's sequence number,
    /// each entry at the next and the last at `TMPFS_DIRSEQ_EOF`; a read from a cookie
    /// resumes there; a cookie no entry has is `EINVAL`; a buffer too small for an entry
    /// returns what fitted.
    #[test]
    fn directories_are_made_listed_and_removed() {
        let (_g, p, _mp) = setup_root();
        assert_eq!(stat(p, b"/").expect("stat /").va_nlink, 2);

        sys_mkdir(p, c("/d"), 0o755).expect("mkdir /d");
        sys_mkdir(p, c("/d/e"), 0o700).expect("mkdir /d/e");
        assert_eq!(sys_mkdir(p, c("/d"), 0o755), Err(Errno::EEXIST));
        let d = stat(p, b"/d").expect("stat /d");
        assert_eq!((d.va_type, d.va_mode, d.va_nlink), (VDIR, 0o755, 3));
        assert_eq!(stat(p, b"/").expect("stat /").va_nlink, 3);
        assert_eq!(names(p, b"/d"), [&b"."[..], b"..", b"e"]);
        assert_eq!(
            stat(p, b"/d/e/..").expect("..").va_fileid,
            d.va_fileid,
            "`..` is the parent"
        );

        // The cookies.
        close(p, create(p, b"/d/f", 0o644));
        let (r, ents, off, eof) = readdir(p, b"/d", 0, 4096);
        assert_eq!((r, eof, off), (Ok(()), 1, TMPFS_DIRSEQ_EOF as Off));
        assert_eq!(ents.len(), 4);
        assert_eq!(ents[0], (b".".to_vec(), 1));
        let (e_seq, f_seq) = (ents[1].1, ents[2].1);
        assert!(e_seq >= 3 && f_seq > e_seq);
        assert_eq!(ents[2].0, b"e");
        assert_eq!(ents[3], (b"f".to_vec(), TMPFS_DIRSEQ_EOF as Off));
        let (r, ents, _, eof) = readdir(p, b"/d", f_seq, 4096);
        assert_eq!((r, eof), (Ok(()), 1));
        assert_eq!(ents, [(b"f".to_vec(), TMPFS_DIRSEQ_EOF as Off)]);
        let (r, ents, _, eof) = readdir(p, b"/d", TMPFS_DIRSEQ_EOF as Off, 4096);
        assert_eq!((r, ents.len(), eof), (Ok(()), 0, 1));
        let (r, _, _, eof) = readdir(p, b"/d", f_seq + 1000, 4096);
        assert_eq!((r, eof), (Err(Errno::EINVAL), 0));
        let (r, ents, off, eof) = readdir(p, b"/d", 0, 40);
        assert_eq!(
            (r, ents.len(), off, eof),
            (Ok(()), 1, 1, 0),
            "only `.` fits"
        );
        let (r, ents, off, _) = readdir(p, b"/d", 0, 30);
        assert_eq!((r, ents.len(), off), (Ok(()), 0, 0), "nothing fits");
        sys_unlink(p, c("/d/f")).expect("unlink");

        assert_eq!(
            dounlinkat(p, AT_FDCWD, c("/d"), AT_REMOVEDIR),
            Err(Errno::ENOTEMPTY)
        );
        assert_eq!(
            dounlinkat(p, AT_FDCWD, c("/d"), 0),
            Err(Errno::EPERM),
            "unlink(2) of a directory"
        );
        dounlinkat(p, AT_FDCWD, c("/d/e"), AT_REMOVEDIR).expect("rmdir /d/e");
        assert_eq!(stat(p, b"/d").expect("stat /d").va_nlink, 2);
        dounlinkat(p, AT_FDCWD, c("/d"), AT_REMOVEDIR).expect("rmdir /d");
        assert_eq!(stat(p, b"/d").err(), Some(Errno::ENOENT));
        assert_eq!(stat(p, b"/").expect("stat /").va_nlink, 2);
        assert_eq!(names(p, b"/"), [&b"."[..], b".."]);
    }

    /// LZ's renames: within a directory (a longer name: a new block), across directories,
    /// over an existing file, a directory into another (the link counts and `..` follow),
    /// a directory over a non-empty one, a file over a directory, a directory into its own
    /// subtree (the genealogy's `EINVAL`), an empty directory over an empty one; and two
    /// names of one node (rename(2) changes nothing: dorenameat's `fvp == tvp` returns before
    /// `VOP_RENAME`, D-t2-5), `.` refused.
    #[test]
    fn renames_move_entries_within_and_across_directories() {
        let (_g, p, _mp) = setup_root();
        close(p, create(p, b"/a", 0o644));
        let id = stat(p, b"/a").expect("stat /a").va_fileid;
        sys_mkdir(p, c("/d"), 0o755).expect("mkdir /d");
        sys_mkdir(p, c("/d2"), 0o755).expect("mkdir /d2");

        // within a directory: a new name, a longer one (past one name quantum)
        let long = "/a-name-longer-than-thirty-two-bytes";
        sys_rename(p, c("/a"), c2(long)).expect("rename");
        assert_eq!(stat(p, b"/a").err(), Some(Errno::ENOENT));
        assert_eq!(stat(p, long.as_bytes()).expect("stat").va_fileid, id);
        sys_rename(p, c(long), c2("/bee")).expect("rename");
        assert_eq!(stat(p, b"/bee").expect("stat /bee").va_fileid, id);

        // across directories
        sys_rename(p, c("/bee"), c2("/d/c")).expect("rename");
        assert_eq!(stat(p, b"/d/c").expect("stat /d/c").va_fileid, id);
        assert_eq!(names(p, b"/"), [&b"."[..], b"..", b"d", b"d2"]);
        assert_eq!(names(p, b"/d"), [&b"."[..], b"..", b"c"]);

        // over an existing file, which goes
        close(p, create(p, b"/d/x", 0o644));
        sys_rename(p, c("/d/c"), c2("/d/x")).expect("rename");
        assert_eq!(stat(p, b"/d/x").expect("stat /d/x").va_fileid, id);
        assert_eq!(names(p, b"/d"), [&b"."[..], b"..", b"x"]);

        // two names of one node: rename(2) does nothing (dorenameat's `fvp == tvp`)
        sys_link(p, c("/d/x"), c2("/d/y")).expect("link");
        sys_rename(p, c("/d/y"), c2("/d/x")).expect("rename");
        assert_eq!(names(p, b"/d"), [&b"."[..], b"..", b"x", b"y"]);
        assert_eq!(stat(p, b"/d/x").expect("stat").va_nlink, 2);
        sys_unlink(p, c("/d/y")).expect("unlink");

        // a directory into another one: the link counts and `..` follow it
        sys_rename(p, c("/d"), c2("/d2/d")).expect("rename");
        let d2 = stat(p, b"/d2").expect("stat /d2");
        assert_eq!(d2.va_nlink, 3);
        assert_eq!(stat(p, b"/").expect("stat /").va_nlink, 3);
        assert_eq!(stat(p, b"/d2/d/..").expect("..").va_fileid, d2.va_fileid);
        assert_eq!(stat(p, b"/d2/d/x").expect("stat").va_fileid, id);

        // a directory over a non-empty one, a file over a directory, a directory into itself
        sys_mkdir(p, c("/e"), 0o755).expect("mkdir /e");
        assert_eq!(sys_rename(p, c("/e"), c2("/d2")), Err(Errno::ENOTEMPTY));
        assert_eq!(sys_rename(p, c("/d2/d/x"), c2("/e")), Err(Errno::EISDIR));
        assert_eq!(sys_rename(p, c("/e"), c2("/d2/d/x")), Err(Errno::ENOTDIR));
        assert_eq!(sys_rename(p, c("/d2"), c2("/d2/d/z")), Err(Errno::EINVAL));
        assert_eq!(sys_rename(p, c("/d2/d/."), c2("/q")), Err(Errno::EINVAL));
        // an empty directory over an empty one
        sys_mkdir(p, c("/f"), 0o755).expect("mkdir /f");
        sys_rename(p, c("/e"), c2("/f")).expect("rename");
        assert_eq!(names(p, b"/"), [&b"."[..], b"..", b"d2", b"f"]);
        assert_eq!(stat(p, b"/").expect("stat /").va_nlink, 4);
    }

    /// Rename's `fvp == tvp` branch (the C's tmpfs_vnops.c:1478-1494), which rename(2) never
    /// reaches (dorenameat returns first) and a `link(2)` racing between namei and the rename
    /// locks does: through `tmpfs_sane_rename` itself, two names of one file in a directory
    /// lose the source name (`tmpfs_do_remove`, the count 2 -> 1); a name renamed onto itself
    /// (`tde == fde`) keeps both names; a directory onto itself is `EINVAL`. The C's
    /// `tmpfs_rename_cache_purge` asserts `fvp != tvp` after the first two (a `DIAGNOSTIC`
    /// kernel panics there, the C's and the kernel's alike); the host tests run without it.
    #[test]
    fn rename_of_one_node_onto_itself_is_the_c_s() {
        let (_g, p, _mp) = setup_root();
        sys_mkdir(p, c("/d"), 0o755).expect("mkdir /d");
        sys_mkdir(p, c("/d/sub"), 0o755).expect("mkdir /d/sub");
        close(p, create(p, b"/d/x", 0o644));
        sys_link(p, c("/d/x"), c2("/d/y")).expect("link");
        assert_eq!(stat(p, b"/d/x").expect("stat").va_nlink, 2);
        let dvp = lookup(p, b"/d").expect("lookup /d");
        let cred = Cred::real(p.ucred());
        let rename = |from: &'static [u8], to: &'static [u8]| {
            let mut fcnp = Cn::<TestGlue>::new(RENAME, 0, p, cred);
            fcnp.set_name_lit(from);
            let mut tcnp = Cn::<TestGlue>::new(RENAME, 0, p, cred);
            tcnp.set_name_lit(to);
            tmpfs_sane_rename(dvp, &fcnp, dvp, &tcnp, cred, false)
        };

        // Two names of one file: the source name goes.
        assert_eq!(rename(b"y", b"x"), Ok(()));
        assert_eq!(names(p, b"/d"), [&b"."[..], b"..", b"sub", b"x"]);
        assert_eq!(stat(p, b"/d/x").expect("stat").va_nlink, 1);
        // One name onto itself: both stay.
        sys_link(p, c("/d/x"), c2("/d/y")).expect("link");
        assert_eq!(rename(b"x", b"x"), Ok(()));
        assert_eq!(names(p, b"/d"), [&b"."[..], b"..", b"sub", b"x", b"y"]);
        assert_eq!(stat(p, b"/d/x").expect("stat").va_nlink, 2);
        // A directory onto itself.
        assert_eq!(rename(b"sub", b"sub"), Err(Errno::EINVAL));
        assert_eq!(stat(p, b"/d/sub").expect("stat").va_type, VDIR);
        assert_eq!(vop_islocked(dvp), 0, "the rename unlocked everything");
        vrele(dvp);
    }

    /// LZ's hard links: a second name, `EEXIST`, none to a directory, the count down as
    /// names go.
    #[test]
    fn hard_links_count_names() {
        let (_g, p, _mp) = setup_root();
        close(p, create(p, b"/f", 0o644));
        sys_mkdir(p, c("/d"), 0o755).expect("mkdir /d");

        sys_link(p, c("/f"), c2("/d/g")).expect("link");
        let f = stat(p, b"/f").expect("stat /f");
        assert_eq!(f.va_nlink, 2);
        assert_eq!(stat(p, b"/d/g").expect("stat /d/g").va_fileid, f.va_fileid);
        assert_eq!(sys_link(p, c("/f"), c2("/d/g")), Err(Errno::EEXIST));
        assert_eq!(
            sys_link(p, c("/d"), c2("/h")),
            Err(Errno::EPERM),
            "no links to directories"
        );

        sys_unlink(p, c("/f")).expect("unlink /f");
        assert_eq!(stat(p, b"/d/g").expect("stat /d/g").va_nlink, 1);
        sys_unlink(p, c("/d/g")).expect("unlink /d/g");
        assert_eq!(names(p, b"/d"), [&b"."[..], b".."]);
    }

    /// LZ's symbolic links, through both legs of `tmpfs_readlink`: `readlink(2)` (a user
    /// uio, straight from the link's block, a short buffer truncating) and `namei`'s
    /// following (a kernel uio, the stack copy); a long target.
    #[test]
    fn symbolic_links_read_back_and_are_followed() {
        let (_g, p, _mp) = setup_root();
        sys_mkdir(p, c("/d"), 0o755).expect("mkdir /d");
        sys_symlink(p, c("d"), c2("/l")).expect("symlink");

        let l = stat(p, b"/l").expect("lstat /l");
        assert_eq!((l.va_type, l.va_size), (VLNK, 1));
        let ubuf = UBUF + 0x800;
        assert_eq!(sys_readlink(p, c("/l"), ubuf, 16), Ok(1));
        let mut b = [0u8; 1];
        UserPtr::<u8>::new(ubuf).copyin_slice(&mut b).unwrap();
        assert_eq!(&b, b"d");

        // followed in the middle of a path (namei's kernel uio)
        let d = stat(p, b"/d").expect("stat /d").va_fileid;
        assert_eq!(stat(p, b"/l/.").expect("stat /l/.").va_fileid, d);

        // a long target, read whole and truncated
        let target = "x".repeat(200);
        sys_symlink(p, c(&target), c2("/long")).expect("symlink");
        assert_eq!(sys_readlink(p, c("/long"), ubuf, 300), Ok(200));
        let mut back = vec![0u8; 200];
        UserPtr::<u8>::new(ubuf).copyin_slice(&mut back).unwrap();
        assert_eq!(back, target.as_bytes());
        assert_eq!(sys_readlink(p, c("/long"), ubuf, 7), Ok(7));
        sys_unlink(p, c("/long")).expect("unlink");
    }

    /// LZ's: the vnode lock is recursive as ufs's (`LK_RECURSEFAIL` is `EDEADLK`), and
    /// vnd(4)'s read through `vn_rdwr` under `vn_open`'s lock leaves it at one level.
    #[test]
    fn the_vnode_lock_recurses_as_ufs_does_and_vnd_reads_under_it() {
        let (_g, p, _mp) = setup_root();
        let mut nd = ndinit(0, 0, NiDirp::Sys(b"/new.img"), p);
        vn_open(&mut nd, FREAD | FWRITE | O_CREAT, 0o600).expect("open");
        let vp = nd.ni_vp.expect("a vnode");
        let node = vptonode(vp);
        let lock = &node.tn_vlock;

        // vn_open returns the vnode locked by this thread
        assert_eq!(vop_islocked(vp), LK_EXCLUSIVE);
        assert_eq!(lock.depth(), 1);

        // the same thread takes it again: rrw_enter counts, as ufs_lock does
        vn_lock(vp, LK_EXCLUSIVE | LK_RETRY).expect("recursive lock");
        assert_eq!(lock.depth(), 2);
        assert_eq!(
            vn_lock(vp, LK_EXCLUSIVE | LK_RECURSEFAIL),
            Err(Errno::EDEADLK)
        );
        let _ = vop_unlock(vp);
        assert_eq!(lock.depth(), 1);
        assert_eq!(vop_islocked(vp), LK_EXCLUSIVE, "still held once");

        // vnd(4)'s VNDIOCSET: vndsetcred reads the file through vn_rdwr (which locks it
        // again) while vn_open's lock is held, and the lock is back to one level afterwards
        let mut buf = [0u8; 512];
        assert_eq!(rdwr(p, UioRw::UIO_READ, vp, &mut buf, 0), Ok(512));
        assert_eq!(lock.depth(), 1);
        assert_eq!(vop_islocked(vp), LK_EXCLUSIVE);

        let _ = vop_unlock(vp);
        assert_eq!(vop_islocked(vp), 0);
        drop(node);
        close(p, vp);
    }

    /// LZ's pathconf and filters, and the filters through `kevent(2)` (tmpfs_kqfilter hooks
    /// each on the vnode's klist): the read filter's bytes past the offset, the write
    /// filter always ready, the vnode filter's `NOTE_WRITE | NOTE_EXTEND` of a growing
    /// write; an unknown filter refused; revocation ends each.
    #[test]
    fn pathconf_and_kqueue_filters() {
        let (_g, p, _mp) = setup_root();
        let vp = create(p, b"/f", 0o644);
        let mut v: Register = 0;
        vop_pathconf(vp, _PC_NAME_MAX, &mut v).expect("pathconf");
        assert_eq!(v, TMPFS_MAXNAMLEN as Register);
        vop_pathconf(vp, _PC_LINK_MAX, &mut v).expect("pathconf");
        assert_eq!(v, LINK_MAX as Register);
        vop_pathconf(vp, _PC_FILESIZEBITS, &mut v).expect("pathconf");
        assert_eq!(v, 64);
        assert_eq!(vop_pathconf(vp, 9999, &mut v), Err(Errno::EINVAL));

        let mut data = vec![1u8; 100];
        assert_eq!(rdwr(p, UioRw::UIO_WRITE, vp, &mut data, 0), Ok(0));
        let _ = kqueue_init::<TestGlue>();
        let fd = sys_open(p, c("/f"), O_RDWR, 0).expect("open");
        assert_eq!(sys_read(p, fd, UBUF, 30), Ok(30));
        let kfd = sys_kqueue(p).expect("kqueue");
        let changes = [
            Kevent {
                ident: fd as usize,
                filter: EVFILT_READ,
                flags: EV_ADD,
                ..Kevent::default()
            },
            Kevent {
                ident: fd as usize,
                filter: EVFILT_WRITE,
                flags: EV_ADD,
                ..Kevent::default()
            },
            Kevent {
                ident: fd as usize,
                filter: EVFILT_VNODE,
                flags: EV_ADD,
                fflags: NOTE_WRITE | NOTE_EXTEND | NOTE_DELETE,
                ..Kevent::default()
            },
        ];
        let events = UserPtr::<Kevent>::new(UBUF + 0x100);
        let get = |n: i32| {
            let r = sys_kevent(p, kfd, UserPtr::new(UBUF), n, events, 3, 0);
            let mut got = [Kevent::default(); 3];
            events.copyin_slice(&mut got).unwrap();
            (r, got)
        };
        UserPtr::<Kevent>::new(UBUF)
            .copyout_slice(&changes)
            .unwrap();
        let of = |got: &[Kevent; 3], n: usize, f: i16| {
            *got[..n].iter().find(|e| e.filter == f).expect("an event")
        };
        let (r, got) = get(3);
        assert_eq!(r, Ok(2));
        assert_eq!(of(&got, 2, EVFILT_READ).data, 70);
        assert_eq!(of(&got, 2, EVFILT_WRITE).data, 0);
        // A write past the end (30..130): NOTE_WRITE | NOTE_EXTEND; the offset is then at
        // the end, so the read filter has nothing.
        UserPtr::<u8>::new(UBUF + 0x200)
            .copyout_slice(&[7u8; 100])
            .unwrap();
        assert_eq!(sys_write(p, fd, UBUF + 0x200, 100), Ok(100));
        let (r, got) = get(0);
        assert_eq!(r, Ok(2));
        assert_eq!(of(&got, 2, EVFILT_VNODE).fflags, NOTE_WRITE | NOTE_EXTEND);
        assert_eq!(of(&got, 2, EVFILT_WRITE).data, 0);
        sys_close(p, kfd).expect("close kq");
        sys_close(p, fd).expect("close");
        assert!(vp.klist.is_empty(), "the knotes came off the vnode's list");

        // An unknown filter is refused; the events on detached knotes.
        let bad = Knote::<TestGlue>::detached();
        bad.kn_filter().set(-100);
        assert_eq!(vop_kqfilter(vp, 0, &bad), Err(Errno::EINVAL));
        let kn = Knote::<TestGlue>::detached();
        kn.kn_sfflags.set(NOTE_WRITE | NOTE_RENAME);
        assert!(!<TmpfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_LINK)
        ));
        assert!(<TmpfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_RENAME)
        ));
        assert_eq!(kn.kn_fflags().get(), NOTE_RENAME);
        assert!(<TmpfsVnode as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF));
        let kn = Knote::<TestGlue>::detached();
        assert!(<TmpfsWrite as Filter<TestGlue>>::event(&kn, 0) && kn.kn_data().get() == 0);
        assert!(<TmpfsWrite as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF) && kn.has_flags(EV_ONESHOT));
        let kn = Knote::<TestGlue>::detached();
        let _ = kn.set_hook_vnode(vp);
        assert!(<TmpfsRead as Filter<TestGlue>>::event(
            &kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF) && kn.has_flags(EV_ONESHOT));
        assert_eq!(TmpfsFiltops::<TestGlue>::READ.f_flags, FILTEROP_ISFD);
        close(p, vp);
    }

    /// The three tables have one registered address each, distinct and not zero-sized; a
    /// file's vnode has the file table, a device node's the special files' (its times kept
    /// by `tmpfs_spec_read`'s and `getattr`'s `rdev`); a fifo node is refused before a vnode
    /// is taken (`tmpfs_vnode_get`'s `EOPNOTSUPP`, `mkfifo(2)`'s answer before the file
    /// system), and the fifo table's gaps answer `ENOSYS`.
    #[test]
    fn the_tables_device_nodes_and_the_fifo_refusal() {
        let (_g, p, mp) = setup_root();
        let t = &vfs::<TestGlue>().tmpfs;
        let (f, s, q) = (
            vops_id::<TestGlue>(&t.vops),
            vops_id::<TestGlue>(&t.specvops),
            vops_id::<TestGlue>(&t.fifovops),
        );
        assert!(f != s && s != q && f != q);
        assert_ne!(size_of::<TmpfsVnops<TestGlue>>(), 0);
        let root = lookup(p, b"/").expect("lookup /");
        assert_eq!(root.ops_id(), f);

        let dev = makedev(250, 7);
        sys_mknod(p, c("/c"), S_IFCHR | 0o600, dev).expect("mknod");
        let va = stat(p, b"/c").expect("stat /c");
        assert_eq!((va.va_type, va.va_rdev), (VCHR, dev));
        let cvp = lookup(p, b"/c").expect("lookup /c");
        assert_eq!(cvp.ops_id(), s);
        vrele(cvp);

        assert_eq!(
            sys_mkfifo(p, c("/fifo"), S_IFIFO | 0o600),
            Err(Errno::EOPNOTSUPP)
        );
        let tmp = vfstotmpfs(mp);
        let node = tmpfs_alloc_node(
            &tmp,
            VFIFO,
            0,
            0,
            0o600,
            None,
            VNOVAL as hal::sys::types::Dev,
        )
        .expect("a fifo node");
        assert_eq!(
            tmpfs_vnode_get(mp, &node, node.tn_nlock.write()).err(),
            Some(Errno::EOPNOTSUPP)
        );
        assert!(node.vnode().is_none());
        tmpfs_free_node(&tmp, node);

        let mut v: Register = 0;
        assert_eq!(
            t.fifovops.pathconf(root, _PC_NAME_MAX, &mut v),
            Err(Errno::ENOSYS)
        );
        assert_eq!(tmpfs_bwrite(), Ok(()));
        vrele(root);
    }

    /// `statfs`'s node counts divide the free memory by the C's `sizeof(tmpfs_node_t)`
    /// (D-N5.9-6): a mount whose memory, not its node limit, decides them.
    #[test]
    fn statfs_counts_nodes_by_the_c_s_size() {
        let (_g, p) = testtmpfs::setup();
        let mut data = testtmpfs::args(64 << 10, 100_000, 0, 0o755);
        let mp = testtmpfs::mount(p, &mut data).expect("mount");
        let mut sb = Statfs::default();
        tmpfs_statfs(mp, &mut sb, p).expect("statfs");
        let free = sb.f_bfree * PAGE_SIZE as u64 / TMPFS_NODE_CHARGE as u64;
        assert!(free < 100_000);
        assert_eq!(sb.f_ffree, free);
        assert_eq!(sb.f_files, 1 + free, "the root and the free ones");
        assert_eq!(sb.f_bsize, PAGE_SIZE as u32);
    }
}
/* </TESTS> */
