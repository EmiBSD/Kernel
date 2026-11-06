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
 * Copyright (c) 2005-2011 The NetBSD Foundation, Inc.
 * Copyright (c) 2013 Pedro Martelletto
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Julio M. Merino Vidal, developed as part of Google's Summer of Code
 * 2005 program, and by Mindaugas Rasiukevicius.
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
//! Efficient memory file system: interfaces for inode and directory entry construction,
//! destruction and manipulation, the attribute changes `tmpfs_setattr` makes, and the data
//! path of regular files through their anonymous UVM object.
//!
//! Upstream: sys/tmpfs/tmpfs_subr.c @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs_subr.rs@3c62ede68802
//!
//! # Reference counting
//!
//! The link count of inode (`tn_links`) is used as a reference counter. However, it has
//! slightly different semantics.
//!
//! For directories - link count represents directory entries, which refer to the
//! directories. In other words, it represents the count of sub-directories. It also takes
//! into account the virtual '.' entry (which has no real entry in the list). For files - link
//! count represents the hard links. Since only empty directories can be removed - link count
//! aligns the reference counting requirements enough. Note: to check whether directory is
//! not empty, the inode size (`tn_size`) can be used.
//!
//! The inode itself, as an object, gathers its first reference when directory entry is
//! attached via `tmpfs_dir_attach`. For instance, after regular `tmpfs_create()`, a file
//! would have a link count of 1, while directory after `tmpfs_mkdir()` would have 2 (due to
//! '.').
//!
//! # Reclamation
//!
//! It should be noted that tmpfs inodes rely on a combination of vnode reference counting
//! and link counting. That is, an inode can only be destroyed if its associated vnode is
//! inactive. The destruction is done on vnode reclamation i.e. `tmpfs_reclaim()`. It should
//! be noted that `tn_links` being 0 is a destruction criterion.
//!
//! If an inode has references within the file system (`tn_links > 0`) and its inactive vnode
//! gets reclaimed/recycled - then the association is broken in `tmpfs_reclaim()`. In such
//! case, an inode will always pass `tmpfs_lookup()` and thus `tmpfs_vnode_get()` to
//! associate a new vnode.
//!
//! # Lock order
//!
//! `tn_nlock` -> `v_vlock` (the node's `tn_vlock`) -> `v_interlock`; the kernel lock under
//! all (the witness of a directory's entries). `tm_lock` -> `tm_acc_lock`: both of the
//! unmount's rounds hold `tm_lock` as the list's witness and take `tm_acc_lock` under it (the
//! first round's `tmpfs_free_dirent` -> `tmpfs_mem_decr`, the second's freed nodes'
//! accounting), a nesting the C never makes (it walks the first round unlocked and takes
//! `tm_lock` and releases it inside each `tmpfs_free_node` before `tm_acc_lock`); only the
//! unmount runs there, so no other path can take the two the other way meanwhile.
//!
//! **Races the C has and keeps:** `tmpfs_vnode_get` waits in `vget` with `tn_nlock`
//! released and starts again (`goto again`) while `vget` answers `ENOENT` (the vnode being
//! reclaimed), `TMPFS_RECLAIMING_BIT` telling `tmpfs_reclaim` not to free the node meanwhile.
//!
//! ## Deviations
//! - Functions that fill a `**` out-parameter return the object (`Result<TmpfsNodeRef,
//!   Errno>`, ...); `tmpfs_dir_lookup`, `tmpfs_dir_cached` and `tmpfs_dir_lookupbyseq`
//!   return `Option` for the C's NULL, and `tmpfs_uio_cached` a `bool`.
//! - `tmpfs_alloc_node`/`tmpfs_alloc_file` take the symbolic link's target as an optional
//!   byte slice (ending at its first NUL or at the slice's end), not a `char *`.
//! - The `tmpfs_ch*` functions take the credential as `&Ucred`; the vnode operations turn
//!   their `a_cred` into one. Their `struct proc *` is kept for the signature (only
//!   `tmpfs_chtimes` uses it).
//! - `option FIFO` is in GENERIC but `miscfs/fifofs` is not ported: `tmpfs_vnode_get`
//!   refuses a fifo node with `EOPNOTSUPP` before taking a vnode, as `ffs_vinit` does
//!   (`tmpfs_fifovops` waits in `tmpfs_fifoops.rs`). `mknod(2)`/`mkfifo(2)` answer
//!   `EOPNOTSUPP` for a fifo before reaching the file system meanwhile (LZ's, kept).
//! - `KASSERT`s on pointers the C then dereferences (`dvp` in `tmpfs_dir_attach`, the
//!   entry's node, the parent of `..`) are panics: Rust cannot follow a NULL.
//! - `tmpfs_vnode_get`'s `rrw_init_flags(&node->tn_vlock, ..)` is not repeated: the node's
//!   lock was made with the node, unlocked, and is unlocked again whenever its vnode is gone
//!   (a fresh vnode finds it free), so the C's re-initialisation changes nothing here.
//!
//! ## Redesign
//! - The node, the entry and the mount are counted handles (`tmpfs.rs`'s notes): a function
//!   that the C gives a pointer takes a borrowed handle, one that keeps or frees what it is
//!   given takes the handle (`tmpfs_dir_attach` takes the entry and the node, linking them;
//!   `tmpfs_free_node`, `tmpfs_free_dirent` take theirs). A node's place on `tm_nodes` and an
//!   entry's on `tn_dir` hold the parked reference of the list (`PoolArc::forget` at the
//!   insert, `PoolArc::adopt` at the remove). LZ's `insert_head`, `insert_tail`, `remove` were
//!   `unsafe` (the list's raw links), and the name, the target and the window raw
//!   pointers (11 `unsafe` with `mapped_bytes`).
//! - The data path (`tmpfs_uiomove`, `tmpfs_zeropg`): the object's range is an
//!   [`ObjectWindow`] (`uvm_map_object_window`, the C's `uvm_map` and its `uvm_unmap` the
//!   drop), copied straight to the uio's iovecs by `uiomove_window`, one copy per byte as the
//!   C's `uiomove((void *)va + pgoff, sz, uio)`; the one-page cache (`tmpfs_uio_lookup`,
//!   `tmpfs_uio_cache`, the `tn_pgnum`/`tn_pgptr` pair) is [`TnReg::cached`], the window
//!   moved out of its cell for the copy and back (no reference over the copy's sleeps, no
//!   lock). The map entry takes the one handle clone it is given (LZ cloned the object's
//!   handle twice per map: `tn_uobj` and the `ObjRef`). `tmpfs_zeropg` zeroes through the
//!   window from a zero block (the C's `bzero`).
//! - Every walk of a directory's entries is the C's `TAILQ_FOREACH`/`TAILQ_NEXT` over the
//!   same elements: `PinList::iter`/`next` under `klock::reader()`, O(1) a step; a step that
//!   copies out (`tmpfs_dir_getdents`) holds the entry by its handle across the copy and
//!   takes the reader again for the next one (the reader is a per-CPU count, never held
//!   across a sleep). A change takes `klock::witness()` (an spl pair the C's vnode-locked
//!   `TAILQ_INSERT_TAIL`/`TAILQ_REMOVE` lack, on create and remove only: ROADMAP N12's line
//!   of msdosfs's hash, D-N5.3-2).
//! - A directory's size steps by [`TMPFS_DIRENT_CHARGE`] (D-N5.9-6: `st_size` OpenBSD's).
//! - `readdir`'s entries are copied out by `fs::subr::dirent_uiomove` (the structure's first
//!   `d_reclen` bytes, the C's `uiomove(&dent, dent.d_reclen, uio)`: the bytes past the name's
//!   NUL are what the previous entry left, as the C's; LZ zeroed them).

use core::sync::atomic::Ordering;

use hal::arch::current::param::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use hal::pool::PoolArc;
use hal::rwlock::WriteWitness;
use hal::sys::dirent::{
    DT_BLK, DT_CHR, DT_DIR, DT_FIFO, DT_LNK, DT_REG, DT_SOCK, Dirent, MAXNAMLEN, dirent_size,
};
use hal::sys::errno::Errno;
use hal::sys::mman::{MADV_NORMAL, MADV_SEQUENTIAL};
use hal::sys::mount::MNT_RDONLY;
use hal::sys::stat::{
    ALLPERMS, APPEND, IMMUTABLE, S_ISGID, S_ISTXT, SF_APPEND, SF_IMMUTABLE, SF_SETTABLE,
    UF_SETTABLE,
};
use hal::sys::syslimits::LINK_MAX;
use hal::sys::types::{Dev, Gid, Mode, Off, Uid, Vsize};
use hal::vm::param::{round_page, trunc_page};

use crate::assert::{kassert, kpanic};
use crate::clock::Timespec;
use crate::cred::{Ucred, groupmember, suser_ucred};
use crate::event::{NOTE_ATTRIB, NOTE_DELETE, NOTE_EXTEND, NOTE_LINK, NOTE_WRITE};
use crate::fs::mount::Mount;
use crate::fs::namei::{Componentname, MAXPATHLEN, SAVESTART};
use crate::fs::subr::{checkalias, dirent_uiomove, vfs, vget, vgone, vops_id, vrele};
use crate::fs::tmpfs::tmpfs::{
    TMPFS_DIRENT_CHARGE, TMPFS_DIRSEQ_DOT, TMPFS_DIRSEQ_DOTDOT, TMPFS_DIRSEQ_END, TMPFS_DIRSEQ_EOF,
    TMPFS_DIRSEQ_NONE, TMPFS_DIRSEQ_START, TMPFS_NODE_ACCESSED, TMPFS_NODE_CHANGED,
    TMPFS_NODE_GEN_MASK, TMPFS_NODE_MODIFIED, TMPFS_NODE_STATUSALL, TMPFS_RECLAIMING_BIT,
    TmpfsDirentRef, TmpfsMount, TmpfsNode, TmpfsNodeRef, TnDir, TnReg, TnSpec, tmpfs_dirseq_full,
    tmpfs_node_reclaiming, tmpfs_validate_dir, vfstotmpfs, vptodir, vptonode,
};
use crate::fs::tmpfs::tmpfs_mem::{
    tmpfs_dirent_get, tmpfs_dirent_put, tmpfs_mem_decr, tmpfs_mem_incr, tmpfs_node_get,
    tmpfs_node_put, tmpfs_strname_alloc, tmpfs_strname_free,
};
use crate::fs::ufs::ufs::ufs_vnops::ucred;
use crate::fs::vnode::{
    LK_EXCLUSIVE, LK_RETRY, VA_UTIMES_CHANGE, VA_UTIMES_NULL, VBAD, VBLK, VCHR, VDIR, VFIFO, VLNK,
    VN_KNOTE, VNON, VNOVAL, VREG, VROOT, VSOCK, VT_TMPFS, VTEXT, VWRITE, Vattr, Vnode, VopsId,
    Vtype, vn_lock,
};
use crate::fs::vops::{Cred, vop_access, vop_islocked};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::SECURELEVEL;
use crate::tc::{getnanotime, nanotime};
use crate::uio::{Uio, uiomove_window};
use crate::vm::aobj::{
    UAO_FLAG_CANFAIL, uao_create, uao_detach, uao_grow, uao_reference, uao_shrink,
};
use crate::vm::km::uvm_map_object_window;
use crate::vm::object::ObjRef;
use crate::vm::types::Voff;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

/// `TMPFS_UIO_MAXBYTES`: be gentle to kernel_map, don't allow more than 4MB in a single
/// transaction.
const TMPFS_UIO_MAXBYTES: usize = (1 << 22) - PAGE_SIZE;

/// A zero page for `tmpfs_zeropg`'s `bzero` through the window, one copy as the C's one
/// `bzero` (4 KiB of read-only data).
static ZEROS: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

/// Whether the node's vnode, if it has one, is locked (`VOP_ISLOCKED(node->tn_vnode)`).
fn vnode_islocked<G: ProcGlue>(node: &TmpfsNode<G>) -> bool {
    node.vnode().is_some_and(|vp| vop_islocked(vp) != 0)
}

/// `tmpfs_alloc_node`: allocate a new inode of a specified type and insert it into the list
/// of specified mount point.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn tmpfs_alloc_node<G: ProcGlue>(
    tmp: &TmpfsMount<G>,
    type_: Vtype,
    uid: Uid,
    gid: Gid,
    mode: Mode,
    target: Option<&[u8]>,
    rdev: Dev,
) -> Result<TmpfsNodeRef<G>, Errno> {
    // The type's case of tn_spec, cleared as the C's switch below leaves it.
    let spec = match type_ {
        VBLK | VCHR => {
            // Character/block special device.
            kassert!(rdev != VNOVAL as Dev);
            TnSpec::Dev(rdev)
        }
        VDIR => TnSpec::Dir(TnDir::new()),
        VFIFO | VSOCK => TnSpec::None,
        VLNK => TnSpec::Lnk(hal::once::OnceStatic::new()),
        VREG => TnSpec::Reg(TnReg::new()),
        VNON | VBAD => {
            kassert!(false);
            TnSpec::None
        }
    };
    // Initially, no references and no associations.
    let Some(nnode) = tmpfs_node_get(tmp, || TmpfsNode::new(type_, spec)) else {
        return Err(Errno::ENOSPC);
    };

    {
        let mut acc = tmp.tm_acc.write();
        acc.tm_highest_inode = acc.tm_highest_inode.wrapping_add(1);
        nnode.tn_id.set(acc.tm_highest_inode);
        if nnode.tn_id.get() == 0 {
            acc.tm_highest_inode = acc.tm_highest_inode.wrapping_sub(1);
            drop(acc);
            tmpfs_node_put(tmp, nnode);
            return Err(Errno::ENOSPC);
        }
    }

    // Generic initialization.
    nnode.tn_gen.store(
        TMPFS_NODE_GEN_MASK & u64::from(hal::random::arc4random()),
        Ordering::Relaxed,
    );

    let now = nanotime();
    nnode.tn_atime.set(now);
    nnode.tn_birthtime.set(now);
    nnode.tn_ctime.set(now);
    nnode.tn_mtime.set(now);

    kassert!(uid != VNOVAL as Uid && gid != VNOVAL as Gid && mode != VNOVAL as Mode);

    nnode.tn_uid.set(uid);
    nnode.tn_gid.set(gid);
    nnode.tn_mode.set(mode);

    // Type-specific initialization (`false`: ENOSPC, the node given back).
    let made = match &nnode.tn_spec {
        TnSpec::Dir(_) => {
            // Extra link count for the virtual '.' entry.
            nnode.tn_links.set(nnode.tn_links.get() + 1);
            true
        }
        TnSpec::Lnk(link) => {
            // Symbolic link. Target specifies the file name.
            kassert!(target.is_some());
            let target = target.unwrap_or(&[]);
            let len = target.iter().position(|&c| c == 0).unwrap_or(target.len());
            kassert!(len < MAXPATHLEN);

            nnode.tn_size.set(len as Off);
            if len == 0 {
                true
            } else if let Some(mut b) = tmpfs_strname_alloc(tmp, len) {
                b[..len].copy_from_slice(&target[..len]);
                let _ = link.set(b);
                true
            } else {
                false
            }
        }
        TnSpec::Reg(reg) => {
            // Regular file. Create an underlying UVM object.
            match uao_create(Vsize::new(0), UAO_FLAG_CANFAIL) {
                Some(ObjRef::Aobj(uobj)) => {
                    let _ = reg.tn_aobj.set(uobj);
                    true
                }
                _ => false,
            }
        }
        TnSpec::Dev(_) | TnSpec::None => true,
    };
    if !made {
        tmpfs_node_put(tmp, nnode);
        return Err(Errno::ENOSPC);
    }

    // rw_init(&nnode->tn_nlock, "tvlk"): made with the node.

    {
        let mut w = tmp.tm_lock.write();
        tmp.tm_nodes.insert_head(&mut w, nnode.as_pin());
    }
    // The list's place holds a reference until tmpfs_free_node takes it off.
    PoolArc::forget(PoolArc::clone(&nnode));

    Ok(nnode)
}

/// `tmpfs_free_node`: remove the inode from a list in the mount point and destroy the inode
/// structures; `node` is the caller's handle (the vnode's, from `tmpfs_reclaim`), which goes
/// with the list's: the item is freed with the last of them.
pub fn tmpfs_free_node<G: ProcGlue>(tmp: &TmpfsMount<G>, node: TmpfsNodeRef<G>) {
    {
        let mut w = tmp.tm_lock.write();
        tmp.tm_nodes.remove(&mut w, &node);
    }
    free_node_unlinked(tmp, node);
}

/// [`tmpfs_free_node`] after `LIST_REMOVE`: the list's reference taken back and the inode's
/// structures destroyed (`tmpfs_unmount`'s second round unlinks every node under one hold of
/// `tm_lock`, the list's witness).
pub(super) fn free_node_unlinked<G: ProcGlue>(tmp: &TmpfsMount<G>, node: TmpfsNodeRef<G>) {
    let listed = PoolArc::adopt(&*node);

    match &node.tn_spec {
        TnSpec::Lnk(_) => {
            if node.tn_size.get() > 0 {
                kassert!(usize::try_from(node.tn_size.get()).is_ok());
                // The block goes with the item (its last handle, below).
                tmpfs_strname_free(tmp, None, node.tn_size.get() as usize);
            }
        }
        TnSpec::Reg(reg) => {
            // Calculate the size of inode data, decrease the used-memory counter, and
            // destroy the underlying UVM object (if any).
            let objsz = PAGE_SIZE * reg.tn_aobj_pages.get() as usize;
            if objsz != 0 {
                tmpfs_mem_decr(tmp, objsz);
            }
            if let Some(uobj) = reg.tn_aobj.get() {
                uao_detach(uobj);
            }
        }
        TnSpec::Dir(dir) => {
            kassert!(dir.tn_dir.is_empty(&hal::klock::reader()));
            kassert!(
                dir.tn_parent.is_none()
                    || tmp.tm_root.get().is_some_and(|r| PoolArc::ptr_eq(r, &node))
            );
            // The root's own parent: the one cycle no detach breaks.
            drop(dir.tn_parent.set(None));
            drop(dir.tn_readdir_lastp.set(None));
        }
        TnSpec::Dev(_) | TnSpec::None => {}
    }

    {
        let mut acc = tmp.tm_acc.write();
        if node.tn_id.get() == acc.tm_highest_inode {
            acc.tm_highest_inode -= 1;
        }
    }

    // mutex_destroy(&node->tn_nlock): nothing to do for an rwlock.
    tmpfs_node_put(tmp, node);
    drop(listed);
}

/// `tmpfs_vnode_get`: allocate or reclaim a vnode for a specified inode.
///
/// Must be called with `tn_nlock` held (`nlock`, its witness), which it releases. Returns
/// the vnode locked.
pub fn tmpfs_vnode_get<'n, G: ProcGlue>(
    mp: &'static Mount<G>,
    node: &'n TmpfsNodeRef<G>,
    nlock: WriteWitness<'n>,
) -> Result<&'static Vnode<G>, Errno> {
    let mut nlock = nlock;
    // again:
    // If there is already a vnode, try to reclaim it.
    while let Some(vp) = node.vnode() {
        // atomic_or_ulong(&node->tn_gen, TMPFS_RECLAIMING_BIT);
        node.tn_gen
            .fetch_or(TMPFS_RECLAIMING_BIT, Ordering::Relaxed);
        drop(nlock);
        let error = vget(vp, LK_EXCLUSIVE);
        if error == Err(Errno::ENOENT) {
            nlock = node.tn_nlock.write();
            continue; // goto again
        }
        // atomic_and_ulong(&node->tn_gen, ~TMPFS_RECLAIMING_BIT);
        node.tn_gen
            .fetch_and(!TMPFS_RECLAIMING_BIT, Ordering::Relaxed);
        return error.map(|()| vp);
    }
    if tmpfs_node_reclaiming(node) {
        // atomic_and_ulong(&node->tn_gen, ~TMPFS_RECLAIMING_BIT);
        node.tn_gen
            .fetch_and(!TMPFS_RECLAIMING_BIT, Ordering::Relaxed);
    }

    // FIFO: vp->v_op = &tmpfs_fifovops below (miscfs/fifofs, not ported); refused here, as
    // ffs_vinit refuses one, before a vnode is taken.
    if node.tn_type == VFIFO {
        drop(nlock);
        return Err(Errno::EOPNOTSUPP);
    }

    // Get a new vnode and associate it with our inode. Share the lock with underlying UVM
    // object, if there is one (VREG case): `#if 0` in the C.
    let t = &vfs::<G>().tmpfs;
    let mut vp = match crate::fs::subr::getnewvnode(VT_TMPFS, Some(mp), &t.vops) {
        Ok(vp) => vp,
        Err(e) => {
            drop(nlock);
            return Err(e);
        }
    };

    // rrw_init_flags(&node->tn_vlock, "tnode", RWL_DUPOK | RWL_IS_VNODE): the module's
    // deviations.
    vp.set_vtype(node.tn_type);

    // Type-specific initialization.
    match node.tn_type {
        VBLK | VCHR => {
            vp.set_ops_id(vops_id::<G>(&t.specvops));
            if let Some(nvp) = checkalias(vp, node.rdev(), Some(mp)) {
                if let Some(data) = vp.take_node() {
                    nvp.set_node(data);
                }
                vp.set_ops_id(VopsId::SPEC);
                vrele(vp);
                vgone(vp);
                vp = nvp;
                node.tn_vnode.set(Some(vp));
            }
        }
        VDIR => {
            if node.dir().tn_parent.points_to(node) {
                vp.set_flag(vp.flag() | VROOT);
            }
        }
        VLNK | VREG | VSOCK | VFIFO => {}
        VNON | VBAD => {
            kassert!(false);
        }
    }

    uvm_vnp_setsize(vp, node.tn_size.get());
    vp.set_node(PoolArc::erase(PoolArc::clone(node)));
    node.tn_vnode.set(Some(vp));
    let _ = vn_lock(vp, LK_EXCLUSIVE | LK_RETRY);
    drop(nlock);

    kassert!(vop_islocked(vp) != 0);
    Ok(vp)
}

/// `tmpfs_alloc_file`: allocate a new file of specified type and adds it into the parent
/// directory. Credentials of the caller are used. Returns the new file's vnode, locked.
pub fn tmpfs_alloc_file<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vap: &Vattr,
    cnp: &mut Componentname<'_, G>,
    target: Option<&[u8]>,
) -> Result<&'static Vnode<G>, Errno> {
    let Some(mp) = dvp.mount() else {
        kpanic(format_args!("tmpfs_alloc_file: vnode {dvp:p} has no mount"));
    };
    let tmp = vfstotmpfs(mp);
    let dnode = vptodir(dvp);

    kassert!(vop_islocked(dvp) != 0);

    let error = 'out: {
        // Check for the maximum number of links limit.
        if vap.va_type == VDIR {
            // Check for maximum links limit.
            if dnode.tn_links.get() == LINK_MAX {
                break 'out Err(Errno::EMLINK);
            }
            kassert!(dnode.tn_links.get() < LINK_MAX);
        }

        if tmpfs_dirseq_full(&dnode) {
            break 'out Err(Errno::ENOSPC);
        }

        if dnode.tn_links.get() == 0 {
            break 'out Err(Errno::ENOENT);
        }

        // Allocate a node that represents the new file.
        let node = match tmpfs_alloc_node(
            &tmp,
            vap.va_type,
            ucred(cnp.cn_cred).cr_uid.get(),
            dnode.tn_gid.get(),
            vap.va_mode,
            target,
            vap.va_rdev,
        ) {
            Ok(node) => node,
            Err(e) => break 'out Err(e),
        };

        // Allocate a directory entry that points to the new file.
        let de = match tmpfs_alloc_dirent(&tmp, cnp.name()) {
            Ok(de) => de,
            Err(e) => {
                tmpfs_free_node(&tmp, node);
                break 'out Err(e);
            }
        };

        // Get a vnode for the new file.
        let vp = match tmpfs_vnode_get(mp, &node, node.tn_nlock.write()) {
            Ok(vp) => vp,
            Err(e) => {
                tmpfs_free_dirent(&tmp, de);
                tmpfs_free_node(&tmp, node);
                break 'out Err(e);
            }
        };

        // Associate inode and attach the entry into the directory.
        tmpfs_dir_attach(&dnode, de, node);
        Ok(vp)
    };

    // out:
    if error.is_ok() && cnp.cn_flags & SAVESTART == 0 {
        drop(cnp.take_pnbuf());
    }
    error
}

/// `tmpfs_alloc_dirent`: allocates a new directory entry for the inode. The directory entry
/// contains a path name component, `name`.
pub fn tmpfs_alloc_dirent<G: ProcGlue>(
    tmp: &TmpfsMount<G>,
    name: &[u8],
) -> Result<TmpfsDirentRef<G>, Errno> {
    let len = name.len() as u16;

    let Some(nde) = tmpfs_dirent_get(tmp) else {
        return Err(Errno::ENOSPC);
    };

    let Some(mut buf) = tmpfs_strname_alloc(tmp, usize::from(len)) else {
        tmpfs_dirent_put(tmp, nde);
        return Err(Errno::ENOSPC);
    };
    buf[..usize::from(len)].copy_from_slice(&name[..usize::from(len)]);
    let _ = nde.set_name(buf, len);
    nde.td_seq.set(TMPFS_DIRSEQ_NONE);

    Ok(nde)
}

/// `tmpfs_free_dirent`: free a directory entry (detached: the caller's handle is its last).
pub fn tmpfs_free_dirent<G: ProcGlue>(tmp: &TmpfsMount<G>, de: TmpfsDirentRef<G>) {
    kassert!(de.td_node_is_none());
    kassert!(de.td_seq.get() == TMPFS_DIRSEQ_NONE);
    if let Some(name) = de.take_name() {
        tmpfs_strname_free(tmp, Some(name), usize::from(de.td_namelen.get()));
    }
    tmpfs_dirent_put(tmp, de);
}

/// `tmpfs_dir_attach`: associate directory entry with a specified inode, and attach the
/// entry into the directory, specified by vnode.
///
/// Increases link count on the associated node. Increases link count on directory node, if
/// our node is VDIR. It is caller's responsibility to check for the `LINK_MAX` limit.
/// Triggers kqueue events here. The entry's handle becomes the directory's (its place on
/// `tn_dir`), the node's the entry's (`td_node`).
pub fn tmpfs_dir_attach<G: ProcGlue>(
    dnode: &TmpfsNodeRef<G>,
    de: TmpfsDirentRef<G>,
    node: TmpfsNodeRef<G>,
) {
    let Some(dvp) = dnode.vnode() else {
        kpanic(format_args!(
            "tmpfs_dir_attach: directory {:p} has no vnode",
            &**dnode
        ));
    };
    let mut events = NOTE_WRITE;

    kassert!(vop_islocked(dvp) != 0);

    // Get a new sequence number.
    kassert!(de.td_seq.get() == TMPFS_DIRSEQ_NONE);
    de.td_seq.set(tmpfs_dir_getseq(dnode, &de));

    // Associate directory entry and the inode.
    let is_dir = node.tn_type == VDIR;
    kassert!(node.tn_links.get() < LINK_MAX);
    node.tn_links.set(node.tn_links.get() + 1);

    // Save the hint (might overwrite).
    drop(node.tn_dirent_hint.set(Some(PoolArc::clone(&de))));
    let child = if is_dir {
        Some(PoolArc::clone(&node))
    } else {
        None
    };
    drop(de.set_td_node(Some(node)));

    // Insert the entry to the directory (parent of inode).
    {
        let mut w = hal::klock::witness();
        dnode.dir().tn_dir.insert_tail(&mut w, de.as_pin());
    }
    // The directory's place holds the entry until tmpfs_dir_detach takes it off.
    PoolArc::forget(de);
    dnode
        .tn_size
        .set(dnode.tn_size.get() + TMPFS_DIRENT_CHARGE as Off);
    tmpfs_update(dnode, TMPFS_NODE_STATUSALL);
    uvm_vnp_setsize(dvp, dnode.tn_size.get());

    if let Some(node) = child {
        // Set parent.
        kassert!(node.dir().tn_parent.is_none());
        drop(node.dir().tn_parent.set(Some(PoolArc::clone(dnode))));

        // Increase the link count of parent.
        kassert!(dnode.tn_links.get() < LINK_MAX);
        dnode.tn_links.set(dnode.tn_links.get() + 1);
        events |= NOTE_LINK;

        tmpfs_validate_dir(&node);
    }
    VN_KNOTE(dvp, events);
}

/// `tmpfs_dir_detach`: disassociate directory entry and its inode, and detach the entry
/// from the directory, specified by vnode.
///
/// Decreases link count on the associated node. Decreases the link count on directory node,
/// if our node is VDIR. Triggers kqueue events here. The directory's reference of the entry
/// goes; `de` is the caller's, for `tmpfs_free_dirent` or another attach.
pub fn tmpfs_dir_detach<G: ProcGlue>(dnode: &TmpfsNodeRef<G>, de: &TmpfsDirentRef<G>) {
    let node = de.node();
    let dvp = dnode.vnode();
    let mut events = NOTE_WRITE;

    kassert!(dvp.is_none_or(|dvp| vop_islocked(dvp) != 0));

    // Deassociate the inode and entry.
    drop(de.set_td_node(None));
    drop(node.tn_dirent_hint.set(None));

    kassert!(node.tn_links.get() > 0);
    node.tn_links.set(node.tn_links.get() - 1);
    if let Some(vp) = node.vnode() {
        kassert!(vop_islocked(vp) != 0);
        VN_KNOTE(
            vp,
            if node.tn_links.get() != 0 {
                NOTE_LINK
            } else {
                NOTE_DELETE
            },
        );
    }

    // If directory - decrease the link count of parent.
    if node.tn_type == VDIR {
        kassert!(node.dir().tn_parent.points_to(dnode));
        drop(node.dir().tn_parent.set(None));

        kassert!(dnode.tn_links.get() > 0);
        dnode.tn_links.set(dnode.tn_links.get() - 1);
        events |= NOTE_LINK;
    }

    // Remove the entry from the directory.
    let dir = dnode.dir();
    if dir.tn_readdir_lastp.points_to(de) {
        drop(dir.tn_readdir_lastp.set(None));
    }
    {
        let mut w = hal::klock::witness();
        dir.tn_dir.remove(&mut w, de);
    }
    drop(PoolArc::adopt(&**de));

    dnode
        .tn_size
        .set(dnode.tn_size.get() - TMPFS_DIRENT_CHARGE as Off);
    tmpfs_update(dnode, TMPFS_NODE_MODIFIED | TMPFS_NODE_CHANGED);
    tmpfs_dir_putseq(dnode, de);
    if let Some(dvp) = dvp {
        tmpfs_update(dnode, 0);
        uvm_vnp_setsize(dvp, dnode.tn_size.get());
        VN_KNOTE(dvp, events);
    }
}

/// `tmpfs_dir_lookup`: find a directory entry in the specified inode, the C's linear scan
/// (the length first, then the bytes).
///
/// Note that the `.` and `..` components are not allowed as they do not physically exist
/// within directories.
pub fn tmpfs_dir_lookup<G: ProcGlue>(
    node: &TmpfsNode<G>,
    cnp: &Componentname<'_, G>,
) -> Option<TmpfsDirentRef<G>> {
    let name = cnp.name();
    let nlen = name.len();

    kassert!(vnode_islocked(node));
    kassert!(nlen != 1 || name[0] != b'.');
    kassert!(nlen != 2 || !(name[0] == b'.' && name[1] == b'.'));
    tmpfs_validate_dir(node);

    let de = {
        let r = hal::klock::reader();
        node.dir()
            .tn_dir
            .iter(&r)
            .find(|de| de.name_is(name))
            .and_then(PoolArc::from_ref)
    };
    tmpfs_update(node, TMPFS_NODE_ACCESSED);
    de
}

/// `tmpfs_dir_cached`: get a cached directory entry if it is valid. Used to avoid
/// unnecessary `tmpfs_dir_lookup()`. The vnode must be locked.
pub fn tmpfs_dir_cached<G: ProcGlue>(node: &TmpfsNode<G>) -> Option<TmpfsDirentRef<G>> {
    let de = node.tn_dirent_hint.get();

    kassert!(vnode_islocked(node));

    let de = de?;
    kassert!(de.td_node_is(node));

    // Directories always have a valid hint. For files, check if there are any hard links.
    // If there are - hint might be invalid.
    if node.tn_type != VDIR && node.tn_links.get() > 1 {
        None
    } else {
        Some(de)
    }
}

/// `tmpfs_dir_getseq`: get a per-directory sequence number for the entry.
pub fn tmpfs_dir_getseq<G: ProcGlue>(
    dnode: &TmpfsNode<G>,
    de: &crate::fs::tmpfs::tmpfs::TmpfsDirent<G>,
) -> u64 {
    let seq = de.td_seq.get();

    tmpfs_validate_dir(dnode);

    if seq != TMPFS_DIRSEQ_NONE {
        // Already set.
        kassert!(seq >= TMPFS_DIRSEQ_START);
        return seq;
    }

    // The "." and ".." and the end-of-directory have reserved numbers. The other sequence
    // numbers are allocated incrementally.

    let next = &dnode.dir().tn_next_seq;
    let seq = next.get();
    kassert!(seq >= TMPFS_DIRSEQ_START);
    kassert!(seq < TMPFS_DIRSEQ_END);
    next.set(seq + 1);
    seq
}

/// `tmpfs_dir_putseq`: give back the entry's sequence number; the directory's counter
/// steps back when it was the last one given, and restarts when the directory is empty.
pub fn tmpfs_dir_putseq<G: ProcGlue>(
    dnode: &TmpfsNode<G>,
    de: &crate::fs::tmpfs::tmpfs::TmpfsDirent<G>,
) {
    let seq = de.td_seq.get();

    tmpfs_validate_dir(dnode);
    kassert!(seq == TMPFS_DIRSEQ_NONE || seq >= TMPFS_DIRSEQ_START);
    kassert!(seq == TMPFS_DIRSEQ_NONE || seq < TMPFS_DIRSEQ_END);

    de.td_seq.set(TMPFS_DIRSEQ_NONE);

    // Empty?  We can reset.
    let next_seq = &dnode.dir().tn_next_seq;
    if dnode.tn_size.get() == 0 {
        next_seq.set(TMPFS_DIRSEQ_START);
    } else if seq != TMPFS_DIRSEQ_NONE && seq == next_seq.get() - 1 {
        next_seq.set(next_seq.get() - 1);
    }
}

/// `tmpfs_dir_lookupbyseq`: lookup a directory entry by the sequence number (the readdir
/// cache first, then the C's linear scan).
pub fn tmpfs_dir_lookupbyseq<G: ProcGlue>(
    node: &TmpfsNode<G>,
    seq: Off,
) -> Option<TmpfsDirentRef<G>> {
    let seq = seq as u64;
    let de = node.dir().tn_readdir_lastp.get();

    tmpfs_validate_dir(node);

    // First, check the cache. If does not match - perform a lookup.
    if let Some(de) = de
        && de.td_seq.get() == seq
    {
        kassert!(de.td_seq.get() >= TMPFS_DIRSEQ_START);
        kassert!(de.td_seq.get() != TMPFS_DIRSEQ_NONE);
        return Some(de);
    }
    let r = hal::klock::reader();
    node.dir()
        .tn_dir
        .iter(&r)
        .find(|de| {
            kassert!(de.td_seq.get() >= TMPFS_DIRSEQ_START);
            kassert!(de.td_seq.get() != TMPFS_DIRSEQ_NONE);
            de.td_seq.get() == seq
        })
        .and_then(PoolArc::from_ref)
}

/// The entry after `de` on its directory's list (`TAILQ_NEXT`, O(1)), counted, the kernel
/// lock's reader taken for the step alone.
fn dir_next<G: ProcGlue>(
    dnode: &TmpfsNode<G>,
    de: &TmpfsDirentRef<G>,
) -> Option<TmpfsDirentRef<G>> {
    let r = hal::klock::reader();
    dnode.dir().tn_dir.next(&r, de).and_then(PoolArc::from_ref)
}

/// Sets `dp`'s name (NUL-terminated, the C's `strlcpy`/`memcpy` and the `'\0'`) and
/// `d_namlen`.
fn dirent_setname(dp: &mut Dirent, name: &[u8]) {
    dp.d_name[..name.len()].copy_from_slice(name);
    dp.d_name[name.len()] = 0;
    dp.d_namlen = name.len() as u8;
}

/// `tmpfs_dir_getdotents`: helper function for `tmpfs_readdir()` to get the dot meta
/// entries, that is, "." or "..". Copy it to the UIO space.
///
/// `Err(EJUSTRETURN)` when the entry does not fit in what is left of `uio`.
pub fn tmpfs_dir_getdotents<G: ProcGlue>(
    node: &TmpfsNode<G>,
    dp: &mut Dirent,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    let next = match uio.uio_offset as u64 {
        TMPFS_DIRSEQ_DOT => {
            dp.d_fileno = node.tn_id.get();
            dirent_setname(dp, b".");
            TMPFS_DIRSEQ_DOTDOT
        }
        TMPFS_DIRSEQ_DOTDOT => {
            let dir = node.dir();
            let Some(parent) = dir.tn_parent.get() else {
                kpanic(format_args!(
                    "tmpfs_dir_getdotents: directory {node:p} has no parent"
                ));
            };
            dp.d_fileno = parent.tn_id.get();
            dirent_setname(dp, b"..");
            let first = {
                let r = hal::klock::reader();
                dir.tn_dir.first(&r).and_then(PoolArc::from_ref)
            };
            match first {
                Some(de) => tmpfs_dir_getseq(node, &de),
                None => TMPFS_DIRSEQ_EOF,
            }
        }
        off => kpanic(format_args!("tmpfs_dir_getdotents: offset {off}")),
    };
    dp.d_type = DT_DIR;
    dp.d_reclen = dirent_size(dp) as u16;
    dp.d_off = next as Off;

    if usize::from(dp.d_reclen) > uio.uio_resid {
        return Err(Errno::EJUSTRETURN);
    }

    dirent_uiomove(dp, uio)?;

    uio.uio_offset = next as Off;
    Ok(())
}

/// `tmpfs_dir_getdents`: helper function for `tmpfs_readdir`.
///
/// Returns as much directory entries as can fit in the uio space. The read starts at
/// `uio.uio_offset`.
pub fn tmpfs_dir_getdents<G: ProcGlue>(
    node: &TmpfsNode<G>,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    kassert!(vnode_islocked(node));
    tmpfs_validate_dir(node);
    let mut dent = Dirent {
        d_fileno: 0,
        d_off: 0,
        d_reclen: 0,
        d_type: 0,
        d_namlen: 0,
        __d_padding: [0; 4],
        d_name: [0; MAXNAMLEN + 1],
    };

    let error = 'done: {
        if uio.uio_offset as u64 == TMPFS_DIRSEQ_DOT
            && let Err(e) = tmpfs_dir_getdotents(node, &mut dent, uio)
        {
            break 'done Err(e);
        }
        if uio.uio_offset as u64 == TMPFS_DIRSEQ_DOTDOT
            && let Err(e) = tmpfs_dir_getdotents(node, &mut dent, uio)
        {
            break 'done Err(e);
        }
        // Done if we reached the end.
        if uio.uio_offset as u64 == TMPFS_DIRSEQ_EOF {
            break 'done Ok(());
        }

        // Locate the directory entry given by the given sequence number.
        let Some(mut de) = tmpfs_dir_lookupbyseq(node, uio.uio_offset) else {
            break 'done Err(Errno::EINVAL);
        };

        // Read as many entries as possible; i.e., until we reach the end of the directory
        // or we exhaust UIO space.
        let mut error = Ok(());
        let cur = loop {
            let dnode = de.node();
            dent.d_fileno = dnode.tn_id.get();
            dent.d_type = match dnode.tn_type {
                VBLK => DT_BLK,
                VCHR => DT_CHR,
                VDIR => DT_DIR,
                VFIFO => DT_FIFO,
                VLNK => DT_LNK,
                VREG => DT_REG,
                VSOCK => DT_SOCK,
                VNON | VBAD => {
                    kassert!(false);
                    dent.d_type
                }
            };
            kassert!(usize::from(de.td_namelen.get()) < dent.d_name.len());
            de.with_name(|name| dirent_setname(&mut dent, name));
            dent.d_reclen = dirent_size(&dent) as u16;

            if dent.d_name[..usize::from(dent.d_namlen)].contains(&b'/') {
                error = Err(Errno::EINVAL);
                break Some(de);
            }

            let next_de = dir_next(node, &de);
            dent.d_off = match &next_de {
                None => TMPFS_DIRSEQ_EOF as Off,
                Some(next) => tmpfs_dir_getseq(node, next) as Off,
            };

            if usize::from(dent.d_reclen) > uio.uio_resid {
                // Exhausted UIO space.
                error = Err(Errno::EJUSTRETURN);
                break Some(de);
            }

            // Copy out the directory entry and continue.
            if let Err(e) = dirent_uiomove(&dent, uio) {
                error = Err(e);
                break Some(de);
            }
            match dir_next(node, &de) {
                Some(next) if uio.uio_resid > 0 => de = next,
                next => break next,
            }
        };

        // Cache the last entry or clear and mark EOF.
        uio.uio_offset = match &cur {
            Some(de) => tmpfs_dir_getseq(node, de) as Off,
            None => TMPFS_DIRSEQ_EOF as Off,
        };
        drop(node.dir().tn_readdir_lastp.set(cur));
        error
    };

    // done:
    tmpfs_update(node, TMPFS_NODE_ACCESSED);

    match error {
        // Exhausted UIO space - just return.
        Err(Errno::EJUSTRETURN) => Ok(()),
        error => error,
    }
}

/// `tmpfs_reg_resize`: resize the underlying UVM object associated with the specified
/// regular file.
pub fn tmpfs_reg_resize<G: ProcGlue>(vp: &'static Vnode<G>, newsize: Off) -> Result<(), Errno> {
    let Some(mp) = vp.mount() else {
        kpanic(format_args!("tmpfs_reg_resize: vnode {vp:p} has no mount"));
    };
    let tmp = vfstotmpfs(mp);
    let node = vptonode(vp);
    let reg = node.reg();
    let uobj = reg.aobj();

    kassert!(vp.vtype() == VREG);
    kassert!(newsize >= 0);

    let oldsize = node.tn_size.get();
    let oldpages = round_page(oldsize as usize) >> PAGE_SHIFT;
    let newpages = round_page(newsize as usize) >> PAGE_SHIFT;
    kassert!(oldpages as u64 == reg.tn_aobj_pages.get());

    if newpages > oldpages {
        // Increase the used-memory counter if getting extra pages.
        let bytes = (newpages - oldpages) << PAGE_SHIFT;
        if !tmpfs_mem_incr(&tmp, bytes) {
            return Err(Errno::ENOSPC);
        }
        let error = {
            let mut w = uobj.object().lock().write();
            uao_grow(uobj, &mut w, newpages as i32)
        };
        if error.is_err() {
            tmpfs_mem_decr(&tmp, bytes);
            return Err(Errno::ENOSPC);
        }
    }

    reg.tn_aobj_pages.set(newpages as u64);
    node.tn_size.set(newsize);
    uvm_vnp_setsize(vp, newsize);
    let _ = uvm_vnp_uncache(vp);

    // Free "backing store".
    if newpages < oldpages {
        if tmpfs_uio_cached(&node) {
            tmpfs_uio_uncache(&node);
        }
        {
            let mut w = uobj.object().lock().write();
            if uao_shrink(uobj, &mut w, newpages as i32).is_err() {
                kpanic(format_args!("shrink failed"));
            }
        }
        // Decrease the used-memory counter.
        tmpfs_mem_decr(&tmp, (oldpages - newpages) << PAGE_SHIFT);
    }
    if newsize > oldsize {
        if tmpfs_uio_cached(&node) {
            tmpfs_uio_uncache(&node);
        }
        let pgoff = oldsize as usize & PAGE_MASK;
        if pgoff != 0 {
            // Growing from an offset which is not at a page boundary; zero out unused bytes
            // in current page.
            if let Err(error) = tmpfs_zeropg(&node, trunc_page(oldsize as usize) as Voff, pgoff) {
                kpanic(format_args!("tmpfs_zeropg: error {}", error as i32));
            }
        }
        VN_KNOTE(vp, NOTE_EXTEND);
    }
    Ok(())
}

/// Whether the vnode's file system is mounted read-only (`vp->v_mount->mnt_flag &
/// MNT_RDONLY`).
fn rdonly<G: ProcGlue>(vp: &Vnode<G>) -> bool {
    vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0)
}

/// `tmpfs_chflags`: change flags of the given vnode.
pub fn tmpfs_chflags<G: ProcGlue>(
    vp: &'static Vnode<G>,
    flags: u32,
    cred: &Ucred,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let node = vptonode(vp);

    kassert!(vop_islocked(vp) != 0);

    // Disallow this operation if the file system is mounted read-only.
    if rdonly(vp) {
        return Err(Errno::EROFS);
    }

    if cred.cr_uid.get() != node.tn_uid.get() {
        suser_ucred(cred)?;
    }

    if cred.cr_uid.get() == 0 {
        if node.tn_flags.get() & (SF_IMMUTABLE | SF_APPEND) != 0
            && SECURELEVEL.load(Ordering::Relaxed) > 0
        {
            return Err(Errno::EPERM);
        }
        node.tn_flags.set(flags);
    } else {
        if node.tn_flags.get() & (SF_IMMUTABLE | SF_APPEND) != 0 || flags & UF_SETTABLE != flags {
            return Err(Errno::EPERM);
        }
        node.tn_flags.set(node.tn_flags.get() & SF_SETTABLE);
        node.tn_flags
            .set(node.tn_flags.get() | (flags & UF_SETTABLE));
    }

    tmpfs_update(&node, TMPFS_NODE_CHANGED);
    VN_KNOTE(vp, NOTE_ATTRIB);
    Ok(())
}

/// `tmpfs_chmod`: change access mode on the given vnode.
pub fn tmpfs_chmod<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: Mode,
    cred: &Ucred,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let node = vptonode(vp);

    kassert!(vop_islocked(vp) != 0);

    // Disallow this operation if the file system is mounted read-only.
    if rdonly(vp) {
        return Err(Errno::EROFS);
    }

    // Immutable or append-only files cannot be modified, either.
    if node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }

    if cred.cr_uid.get() != node.tn_uid.get() {
        suser_ucred(cred)?;
    }
    if cred.cr_uid.get() != 0 {
        if vp.vtype() != VDIR && mode & S_ISTXT != 0 {
            return Err(Errno::EFTYPE);
        }
        if !groupmember(node.tn_gid.get(), cred) && mode & S_ISGID != 0 {
            return Err(Errno::EPERM);
        }
    }

    node.tn_mode.set(mode & ALLPERMS);
    tmpfs_update(&node, TMPFS_NODE_CHANGED);
    if vp.flag() & VTEXT != 0 && node.tn_mode.get() & S_ISTXT == 0 {
        let _ = uvm_vnp_uncache(vp);
    }
    VN_KNOTE(vp, NOTE_ATTRIB);
    Ok(())
}

/// `tmpfs_chown`: change ownership of the given vnode.
///
/// At least one of uid or gid must be different than `VNOVAL`. Attribute is unchanged for
/// `VNOVAL` case.
pub fn tmpfs_chown<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uid: Uid,
    gid: Gid,
    cred: &Ucred,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let node = vptonode(vp);

    kassert!(vop_islocked(vp) != 0);

    // Assign default values if they are unknown.
    kassert!(uid != VNOVAL as Uid || gid != VNOVAL as Gid);
    let uid = if uid == VNOVAL as Uid {
        node.tn_uid.get()
    } else {
        uid
    };
    let gid = if gid == VNOVAL as Gid {
        node.tn_gid.get()
    } else {
        gid
    };

    // Disallow this operation if the file system is mounted read-only.
    if rdonly(vp) {
        return Err(Errno::EROFS);
    }

    // Immutable or append-only files cannot be modified, either.
    if node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }

    if cred.cr_uid.get() != node.tn_uid.get()
        || uid != node.tn_uid.get()
        || (gid != node.tn_gid.get() && !groupmember(gid, cred))
    {
        suser_ucred(cred)?;
    }

    node.tn_uid.set(uid);
    node.tn_gid.set(gid);
    tmpfs_update(&node, TMPFS_NODE_CHANGED);
    VN_KNOTE(vp, NOTE_ATTRIB);
    Ok(())
}

/// `tmpfs_chsize`: change size of the given vnode.
pub fn tmpfs_chsize<G: ProcGlue>(
    vp: &'static Vnode<G>,
    size: u64,
    _cred: &Ucred,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let node = vptonode(vp);

    kassert!(vop_islocked(vp) != 0);

    // Decide whether this is a valid operation based on the file type.
    match vp.vtype() {
        VDIR => return Err(Errno::EISDIR),
        VREG => {
            if rdonly(vp) {
                return Err(Errno::EROFS);
            }
        }
        VBLK | VCHR | VFIFO => {
            // Allow modifications of special files even if in the file system is mounted
            // read-only (we are not modifying the files themselves, but the objects they
            // represent).
            return Ok(());
        }
        _ => return Err(Errno::EOPNOTSUPP),
    }

    // Immutable or append-only files cannot be modified, either.
    if node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }

    // Note: tmpfs_truncate() will raise NOTE_EXTEND and NOTE_ATTRIB.
    tmpfs_truncate(vp, size as Off)
}

/// `tmpfs_chtimes`: change access and modification times for vnode.
pub fn tmpfs_chtimes<G: ProcGlue>(
    vp: &'static Vnode<G>,
    atime: &Timespec,
    mtime: &Timespec,
    vaflags: u32,
    cred: &Ucred,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let node = vptonode(vp);

    kassert!(vop_islocked(vp) != 0);

    // Disallow this operation if the file system is mounted read-only.
    if rdonly(vp) {
        return Err(Errno::EROFS);
    }

    // Immutable or append-only files cannot be modified, either.
    if node.tn_flags.get() & (IMMUTABLE | APPEND) != 0 {
        return Err(Errno::EPERM);
    }

    if cred.cr_uid.get() != node.tn_uid.get()
        && let Err(error) = suser_ucred(cred)
    {
        if vaflags & VA_UTIMES_NULL == 0 {
            return Err(error);
        }
        vop_access(vp, VWRITE, Cred::real(cred), p)?;
    }

    if atime.tv_nsec != i64::from(VNOVAL) {
        node.tn_atime.set(*atime);
    }

    if mtime.tv_nsec != i64::from(VNOVAL) {
        node.tn_mtime.set(*mtime);
    }

    if mtime.tv_nsec != i64::from(VNOVAL) || vaflags & VA_UTIMES_CHANGE != 0 {
        tmpfs_update(&node, TMPFS_NODE_CHANGED);
    }

    VN_KNOTE(vp, NOTE_ATTRIB);

    Ok(())
}

/// `tmpfs_update`: update timestamps, et al.
pub fn tmpfs_update<G: ProcGlue>(node: &TmpfsNode<G>, flags: i32) {
    let nowtm = getnanotime();

    if flags & TMPFS_NODE_ACCESSED != 0 {
        node.tn_atime.set(nowtm);
    }
    if flags & TMPFS_NODE_MODIFIED != 0 {
        node.tn_mtime.set(nowtm);
    }
    if flags & TMPFS_NODE_CHANGED != 0 {
        node.tn_ctime.set(nowtm);
    }
}

/// `tmpfs_truncate`: set a regular file's size.
pub fn tmpfs_truncate<G: ProcGlue>(vp: &'static Vnode<G>, length: Off) -> Result<(), Errno> {
    let node = vptonode(vp);

    if length < 0 {
        return Err(Errno::EINVAL);
    }
    if node.tn_size.get() == length {
        return Ok(());
    }
    let error = tmpfs_reg_resize(vp, length);
    if error.is_ok() {
        tmpfs_update(&node, TMPFS_NODE_CHANGED | TMPFS_NODE_MODIFIED);
    }
    error
}

/// `tmpfs_uio_cached`: whether the node keeps one page of its object mapped.
pub fn tmpfs_uio_cached<G: ProcGlue>(node: &TmpfsNode<G>) -> bool {
    node.reg().cached.read().is_some()
}

/// `tmpfs_uio_uncache`: drop the cached mapping (its window's drop is the C's `uvm_unmap`
/// of the page, which detaches the entry's object reference).
pub fn tmpfs_uio_uncache<G: ProcGlue>(node: &TmpfsNode<G>) {
    let cached = node.reg().cached.replace(None);
    kassert!(cached.is_some());
    drop(cached);
}

/// `tmpfs_uiomove`: move up to `len` bytes between the file's object and `uio`, at
/// `uio.uio_offset`, through a kernel window of the object (one page of it is kept mapped
/// for the next small transfer): the C's one `uvm_map`, faults, one `uiomove` and one
/// `uvm_unmap` or none, or on a hit of the kept page the one `uiomove`.
pub fn tmpfs_uiomove<G: ProcGlue>(
    node: &TmpfsNode<G>,
    uio: &mut Uio<'_, G>,
    len: usize,
) -> Result<(), Errno> {
    let reg = node.reg();
    let pgnum = trunc_page(uio.uio_offset as usize) as Voff;
    let pgoff = uio.uio_offset as usize & PAGE_MASK;

    // The kept page, out of its cell for the copy (tmpfs_uio_lookup).
    let cached = reg.cached.replace(None);
    if pgoff + len < PAGE_SIZE
        && let Some((n, w)) = &cached
        && *n == pgnum
    {
        let r = uiomove_window(w.window(), pgoff, len, uio);
        let _ = reg.cached.replace(cached);
        return r;
    }

    let (sz, adv) = if len >= TMPFS_UIO_MAXBYTES {
        (TMPFS_UIO_MAXBYTES, MADV_NORMAL)
    } else {
        (len, MADV_SEQUENTIAL)
    };

    // if (tmpfs_uio_cached(node)) tmpfs_uio_uncache(node);
    drop(cached);

    let uobj = reg.aobj();
    uao_reference(uobj);

    let w = match uvm_map_object_window(PoolArc::clone(uobj), pgnum, round_page(pgoff + sz), adv) {
        Ok(w) => w,
        Err(error) => {
            uao_detach(uobj); // Drop reference.
            return Err(error);
        }
    };

    let error = uiomove_window(w.window(), pgoff, sz, uio);
    if error.is_ok() && pgoff + sz < PAGE_SIZE {
        // tmpfs_uio_cache(node, pgnum, va)
        let _ = reg.cached.replace(Some((pgnum, w)));
    } else {
        drop(w); // uvm_unmap(kernel_map, va, va + round_page(pgoff + sz))
    }

    error
}

/// `tmpfs_zeropg`: zero the bytes of the page at `pgnum` from `pgoff` to its end.
pub fn tmpfs_zeropg<G: ProcGlue>(
    node: &TmpfsNode<G>,
    pgnum: Voff,
    pgoff: usize,
) -> Result<(), Errno> {
    kassert!(!tmpfs_uio_cached(node));

    let uobj = node.reg().aobj();
    uao_reference(uobj);

    let w = match uvm_map_object_window(PoolArc::clone(uobj), pgnum, PAGE_SIZE, MADV_NORMAL) {
        Ok(w) => w,
        Err(error) => {
            uao_detach(uobj); // Drop reference.
            return Err(error);
        }
    };

    // bzero((void *)va + pgoff, PAGE_SIZE - pgoff)
    w.window().write_at(pgoff, &ZEROS[pgoff..])?;
    drop(w); // uvm_unmap(kernel_map, va, va + PAGE_SIZE)

    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests over a mount of the fixture (testtmpfs: no device, the fixture's vnode
    // table): the directory sequence numbers through attach and detach (LZ's, moved from hand
    // linked lists), the times, the attributes, and the data path, host-tested for the first
    // time (D-N5.9-11: the window over the file's aobj faulted in through the host's
    // kdata_abort): a write and a read through kernel and user iovecs, the one-page cache's
    // hit and miss, a growth zeroing the tail of its page, a fault on the user side, and no
    // kernel_map range left behind on any path. User memory: testfs's UBUF (every fixture
    // holds MEMORY).

    use std::vec::Vec;

    use super::*;
    use crate::fs::subr::vput;
    use crate::fs::testfs::{TestGlue, UBUF};
    use crate::fs::tmpfs::testtmpfs;
    use crate::fs::tmpfs::tmpfs_vfsops::{tmpfs_root, tmpfs_unmount};
    use crate::uio::{Iovec, UioRw, UioSeg};
    use crate::vm::km::kernel_map;
    use hal::user::UserPtr;

    type P = &'static Thread<TestGlue>;

    /// A component name for `name`, with the thread's credentials.
    fn cn(p: P, name: &'static [u8]) -> Componentname<'static, TestGlue> {
        let mut cnp = Componentname::new(0, 0, p, Cred::real(p.ucred()));
        cnp.set_name_lit(name);
        cnp
    }

    /// The attributes `VOP_CREATE`/`VOP_MKDIR` pass for a new node.
    fn vattr(type_: Vtype, mode: Mode) -> Vattr {
        let mut va = Vattr::new();
        va.va_type = type_;
        va.va_mode = mode;
        va.va_rdev = VNOVAL as Dev;
        va
    }

    /// A mount of 1 MiB at most and its root vnode, locked.
    fn mounted(p: P) -> (&'static Mount<TestGlue>, &'static Vnode<TestGlue>) {
        let mut data = testtmpfs::args(1 << 20, 0, 0, 0o1777);
        let mp = testtmpfs::mount(p, &mut data).expect("mount");
        (mp, tmpfs_root(mp).expect("the root"))
    }

    /// A regular file `name` of the root `dvp`, its vnode locked.
    fn create(
        dvp: &'static Vnode<TestGlue>,
        p: P,
        name: &'static [u8],
    ) -> &'static Vnode<TestGlue> {
        tmpfs_alloc_file(dvp, &vattr(VREG, 0o644), &mut cn(p, name), None).expect("create")
    }

    /// `tmpfs_remove`'s work on the entry of `name`: detach and free it.
    fn remove(dvp: &'static Vnode<TestGlue>, p: P, name: &'static [u8]) {
        let dnode = vptodir(dvp);
        let Some(mp) = dvp.mount() else {
            panic!("no mount")
        };
        let de = tmpfs_dir_lookup(&dnode, &cn(p, name)).expect("the entry");
        tmpfs_dir_detach(&dnode, &de);
        tmpfs_free_dirent(&vfstotmpfs(mp), de);
    }

    /// `len` bytes of the file at `off`, read through a kernel uio.
    fn read(node: &TmpfsNode<TestGlue>, off: Off, len: usize) -> Vec<u8> {
        let mut buf = std::vec![0u8; len];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<TestGlue>::kernel_read(&mut iov, &mut buf, None);
        uio.uio_offset = off;
        tmpfs_uiomove(node, &mut uio, len).expect("read");
        assert_eq!(uio.uio_resid, 0);
        buf
    }

    /// `src` written into the file at `off` through a kernel uio (the file grown first, as
    /// tmpfs_write does).
    fn write(vp: &'static Vnode<TestGlue>, off: Off, src: &[u8]) {
        let node = vptonode(vp);
        let end = off + src.len() as Off;
        if end > node.tn_size.get() {
            tmpfs_reg_resize(vp, end).expect("grow");
        }
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<TestGlue>::kernel_write(&mut iov, src, None);
        uio.uio_offset = off;
        tmpfs_uiomove(&node, &mut uio, src.len()).expect("write");
        assert_eq!(uio.uio_offset, end);
    }

    /// A user uio of `len` bytes at `uaddr`, file offset `off`.
    fn user_uio(
        iov: &mut [Iovec; 1],
        uaddr: usize,
        len: usize,
        off: Off,
        rw: UioRw,
        p: P,
    ) -> Uio<'_, TestGlue> {
        iov[0] = Iovec {
            iov_base: core::ptr::without_provenance_mut(uaddr),
            iov_len: len,
        };
        Uio {
            uio_iov: iov,
            uio_offset: off,
            uio_resid: len,
            uio_segflg: UioSeg::UIO_USERSPACE,
            uio_rw: rw,
            uio_procp: Some(p),
        }
    }

    #[test]
    fn sequence_numbers_start_after_the_reserved_ones_and_step_back() {
        let (_g, p) = testtmpfs::setup();
        let (mp, dvp) = mounted(p);
        let d = vptodir(dvp);
        let vps = [
            create(dvp, p, b"a"),
            create(dvp, p, b"b"),
            create(dvp, p, b"c"),
        ];
        let seq = |n: &'static [u8]| tmpfs_dir_lookup(&d, &cn(p, n)).map(|de| de.td_seq.get());
        assert_eq!(
            [seq(b"a"), seq(b"b"), seq(b"c")],
            [Some(3), Some(4), Some(5)]
        );
        assert_eq!(
            d.tn_size.get(),
            3 * TMPFS_DIRENT_CHARGE as Off,
            "the C's sizeof each"
        );

        // Removing the last one given steps the counter back; a hole in the middle does not.
        remove(dvp, p, b"c");
        assert_eq!(d.dir().tn_next_seq.get(), 5);
        remove(dvp, p, b"a");
        assert_eq!(d.dir().tn_next_seq.get(), 5);
        let vd = create(dvp, p, b"d");
        assert_eq!(seq(b"d"), Some(5));

        // Lookups by number: the readdir cache first, then the list.
        let b = tmpfs_dir_lookup(&d, &cn(p, b"b")).expect("b");
        assert!(tmpfs_dir_lookupbyseq(&d, 4).is_some_and(|de| PoolArc::ptr_eq(&de, &b)));
        drop(d.dir().tn_readdir_lastp.set(Some(PoolArc::clone(&b))));
        assert!(tmpfs_dir_lookupbyseq(&d, 4).is_some_and(|de| PoolArc::ptr_eq(&de, &b)));
        assert!(tmpfs_dir_lookupbyseq(&d, 3).is_none());
        assert!(!tmpfs_dirseq_full(&d));
        drop(b);

        // An emptied directory restarts its numbers.
        remove(dvp, p, b"b");
        remove(dvp, p, b"d");
        assert_eq!(d.tn_size.get(), 0);
        assert_eq!(d.dir().tn_next_seq.get(), TMPFS_DIRSEQ_START);
        assert!(
            d.dir().tn_readdir_lastp.is_none(),
            "the detach cleared the cache"
        );
        d.dir().tn_next_seq.set(TMPFS_DIRSEQ_END);
        assert!(
            tmpfs_dirseq_full(&d),
            "a full directory takes no more entries"
        );
        d.dir().tn_next_seq.set(TMPFS_DIRSEQ_START);

        drop(d);
        for vp in vps {
            vput(vp);
        }
        vput(vd);
        vput(dvp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
    }

    #[test]
    fn update_sets_the_requested_times() {
        let node = TmpfsNode::<TestGlue>::new(VREG, TnSpec::None);
        let never = Timespec {
            tv_sec: -1,
            tv_nsec: 0,
        };
        node.tn_atime.set(never);
        node.tn_mtime.set(never);
        node.tn_ctime.set(never);
        tmpfs_update(&node, TMPFS_NODE_MODIFIED);
        assert_eq!(node.tn_atime.get(), never);
        assert_eq!(node.tn_ctime.get(), never);
        assert_ne!(node.tn_mtime.get(), never);
        tmpfs_update(&node, TMPFS_NODE_STATUSALL);
        assert_eq!(node.tn_atime.get(), node.tn_ctime.get());
        assert_eq!(node.tn_atime.get(), node.tn_mtime.get());
    }

    #[test]
    fn a_file_is_written_and_read_through_its_objects_window() {
        let (_g, p) = testtmpfs::setup();
        let (mp, dvp) = mounted(p);
        let fvp = create(dvp, p, b"data");
        let node = vptonode(fvp);
        let base = kernel_map().size();

        // Across two pages: one window of two pages, unmapped after (not cached).
        let pattern: Vec<u8> = (0..6000).map(|i| (i % 253) as u8).collect();
        write(fvp, 0, &pattern);
        assert_eq!(node.reg().tn_aobj_pages.get(), 2);
        assert!(!tmpfs_uio_cached(&node));
        assert_eq!(kernel_map().size(), base, "no range left behind");
        assert_eq!(read(&node, 0, 6000), pattern);

        // A user iovec: 100 bytes from inside the second page, straight to user memory.
        let mut iov = [Iovec::new()];
        let mut uio = user_uio(&mut iov, UBUF, 100, 4500, UioRw::UIO_READ, p);
        tmpfs_uiomove(&node, &mut uio, 100).expect("a user read");
        assert_eq!(
            crate::testutil::user_bytes(UBUF, 100).as_deref(),
            Some(&pattern[4500..4600])
        );

        // A user write into the first page.
        UserPtr::<[u8; 5]>::new(UBUF + 512)
            .copyout(b"tmpfs")
            .unwrap();
        let mut iov = [Iovec::new()];
        let mut uio = user_uio(&mut iov, UBUF + 512, 5, 10, UioRw::UIO_WRITE, p);
        tmpfs_uiomove(&node, &mut uio, 5).expect("a user write");
        assert_eq!(
            read(&node, 8, 9),
            [8, 9, b't', b'm', b'p', b'f', b's', 15, 16]
        );

        if tmpfs_uio_cached(&node) {
            tmpfs_uio_uncache(&node);
        }
        assert_eq!(kernel_map().size(), base);
        drop(node);
        vput(fvp);
        vput(dvp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
    }

    #[test]
    fn a_small_transfer_keeps_its_page_and_the_next_one_in_it_hits() {
        let (_g, p) = testtmpfs::setup();
        let (mp, dvp) = mounted(p);
        let fvp = create(dvp, p, b"small");
        let node = vptonode(fvp);
        let base = kernel_map().size();
        write(fvp, 0, &[7u8; 8192]);
        assert_eq!(kernel_map().size(), base);

        // A small read: its window of one page is kept (tmpfs_uio_cache).
        assert_eq!(read(&node, 100, 10), [7u8; 10]);
        assert!(tmpfs_uio_cached(&node));
        assert_eq!(kernel_map().size(), base + PAGE_SIZE);
        let kept = node.reg().cached.read().as_ref().map(|(n, w)| (*n, w.va()));
        // Two more in the same page, a write and a read: the kept window, no new range, and
        // no change of kernel_map at all (every uvm_map and uvm_unmap bumps its timestamp).
        let ts = kernel_map().timestamp();
        write(fvp, 200, b"hit");
        assert_eq!(read(&node, 199, 5), [7, b'h', b'i', b't', 7]);
        assert_eq!(
            kernel_map().timestamp(),
            ts,
            "the hits map and unmap nothing"
        );
        assert_eq!(
            node.reg().cached.read().as_ref().map(|(n, w)| (*n, w.va())),
            kept,
            "the same page, the same window"
        );
        assert_eq!(kernel_map().size(), base + PAGE_SIZE);
        // One in the next page: a miss, the kept page replaced.
        assert_eq!(read(&node, PAGE_SIZE as Off + 1, 3), [7u8; 3]);
        assert_eq!(
            node.reg().cached.read().as_ref().map(|(n, _)| *n),
            Some(PAGE_SIZE as Voff)
        );
        assert_eq!(kernel_map().size(), base + PAGE_SIZE);
        // A transfer that ends on the page's last byte is not kept (pgoff + sz < PAGE_SIZE).
        assert_eq!(read(&node, 4000, 96), [7u8; 96]);
        assert!(!tmpfs_uio_cached(&node));
        assert_eq!(kernel_map().size(), base);

        // A shrink drops a kept page (tmpfs_reg_resize).
        let _ = read(&node, 10, 1);
        assert!(tmpfs_uio_cached(&node));
        tmpfs_truncate(fvp, 100).expect("shrink");
        assert!(!tmpfs_uio_cached(&node));
        assert_eq!(node.reg().tn_aobj_pages.get(), 1);
        assert_eq!(kernel_map().size(), base);

        drop(node);
        vput(fvp);
        vput(dvp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
    }

    #[test]
    fn growing_from_inside_a_page_zeroes_its_tail_and_a_fault_leaves_no_range() {
        let (_g, p) = testtmpfs::setup();
        let (mp, dvp) = mounted(p);
        let fvp = create(dvp, p, b"grow");
        let node = vptonode(fvp);
        let base = kernel_map().size();
        write(fvp, 0, &[0xaau8; PAGE_SIZE]);
        // Shrink inside the page (no page goes), then grow back: tmpfs_zeropg zeroes 100..4096.
        tmpfs_truncate(fvp, 100).expect("shrink");
        assert_eq!(node.reg().tn_aobj_pages.get(), 1);
        tmpfs_truncate(fvp, PAGE_SIZE as Off).expect("grow");
        let back = read(&node, 0, PAGE_SIZE);
        assert_eq!(&back[..100], &[0xaau8; 100][..]);
        assert!(back[100..].iter().all(|&b| b == 0), "the tail zeroed");
        if tmpfs_uio_cached(&node) {
            tmpfs_uio_uncache(&node);
        }
        assert_eq!(kernel_map().size(), base);

        // A user iovec outside user memory: EFAULT, the window unmapped, nothing kept.
        let mut iov = [Iovec::new()];
        let mut uio = user_uio(&mut iov, 8, 16, 0, UioRw::UIO_READ, p);
        assert_eq!(tmpfs_uiomove(&node, &mut uio, 16), Err(Errno::EFAULT));
        assert_eq!(uio.uio_resid, 16);
        assert!(!tmpfs_uio_cached(&node));
        assert_eq!(kernel_map().size(), base);
        // The object's one reference is its node's: none of a window's is left.
        assert_eq!(node.reg().aobj().object().refs(), 1);

        drop(node);
        vput(fvp);
        vput(dvp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
    }

    #[test]
    fn attributes_change_as_root_and_flags_refuse_changes() {
        let (_g, p) = testtmpfs::setup();
        let (mp, dvp) = mounted(p);
        let fvp = create(dvp, p, b"attr");
        let node = vptonode(fvp);
        let cred = p.ucred();
        tmpfs_chmod(fvp, 0o4600, cred, p).expect("chmod");
        assert_eq!(node.tn_mode.get(), 0o4600);
        tmpfs_chown(fvp, 7, VNOVAL as Gid, cred, p).expect("chown");
        assert_eq!((node.tn_uid.get(), node.tn_gid.get()), (7, 0));
        tmpfs_chflags(fvp, SF_IMMUTABLE, cred, p).expect("chflags");
        assert_eq!(tmpfs_chmod(fvp, 0o600, cred, p), Err(Errno::EPERM));
        assert_eq!(tmpfs_chsize(fvp, 0, cred, p), Err(Errno::EPERM));
        tmpfs_chflags(fvp, 0, cred, p).expect("chflags");
        assert_eq!(tmpfs_chsize(dvp, 0, cred, p), Err(Errno::EISDIR));
        assert_eq!(tmpfs_truncate(fvp, -1), Err(Errno::EINVAL));
        drop(node);
        vput(fvp);
        vput(dvp);
        tmpfs_unmount(mp, 0, p).expect("unmount");
    }
}
/* </TESTS> */
