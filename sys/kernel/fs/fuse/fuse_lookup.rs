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
 * Copyright (c) 2012-2013 Sylvestre Gallon <ccna.syl@gmail.com>
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
//! `fusefs_lookup` (`vop_lookup`): looks a name up in a FUSE directory by asking the daemon
//! (`FUSE_LOOKUP`); `.` and `..` are answered in the kernel.
//!
//! Upstream: sys/miscfs/fuse/fuse_lookup.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fuse_lookup.rs@3c62ede68802
//!
//! ## Deviations
//! - The type the daemon reported (`nvtype`) is an `Option`: the C leaves it uninitialised
//!   for `.` and `..`, and for `..` under `RENAME` writes that garbage into the parent's
//!   `v_type`; the kernel leaves `v_type` as `VFS_VGET` found it there (LZ's).
//!
//! Kept as the C, named (D-N5.9-9): a `..` whose node turns out not to be a directory fails
//! with `EIO` without releasing the vnode `VFS_VGET` returned (a reference the C leaks); the
//! `..` step unlocks the directory before `VFS_VGET` of its parent ("race to get the inode")
//! and locks it again afterwards, so a daemon whose answer names an ancestor as a child
//! inverts the vnode lock order against a concurrent descent, as the C's (checking it would
//! walk `i_parent_cache` to the root on every lookup; an unanswering daemon already hangs
//! its callers by design).
//!
//! ## Redesign
//! - The operation takes the kernel's `vop_lookup` arguments (`Vnops::lookup`: the
//!   directory, the out-vnode, the component) where LZ took `bsd`'s `struct vop_lookup_args`;
//!   the node and the mount are `vtoi`'s and the node's counted handles, so nothing here
//!   outlives what it reads (LZ's `&'static` over raw blocks are gone).
//! - The name goes into the request's data block through `Fusebuf::dat_update` (a short
//!   write before the request is published) where LZ wrote a raw slice of the block through
//!   `unsafe`; the reply's `entry` is read at offset 0 of the `op` union (`op_get`), as the
//!   C's `fbuf->op.out.entry`. The `FUSE_FORGET` of the reclaim path is handed to the device's
//!   queue, which frees it once read (or at once when no device is open).

use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;
use hal::sys::types::Ino;

use crate::fs::fuse::fuse_device::fuse_device_queue_fbuf;
use crate::fs::fuse::fusebuf::{fb_delete, fb_queue, fb_setup};
use crate::fs::fuse::fusebuf_h::{
    FUSE_FORGET, FUSE_LOOKUP, FUSE_ROOT_ID, FuseEntryOut, FuseForgetIn,
};
use crate::fs::fuse::fusefs_node::vtoi;
use crate::fs::namei::{
    CREATE, DELETE, ISDOTDOT, ISLASTCN, LOCKPARENT, PDIRUNLOCK, RENAME, SAVENAME, WANTPARENT,
};
use crate::fs::subr::{vput, vref};
use crate::fs::ufs::ufs::ufs_lookup::vmount;
use crate::fs::vnode::{
    LK_EXCLUSIVE, LK_RETRY, VDIR, VEXEC, VWRITE, Vnode, Vtype, iftovt, vn_lock,
};
use crate::fs::vops::{Cn, VFS_VGET, vop_access, vop_unlock};
use crate::proc::ProcGlue;

/// `fusefs_lookup` (`vop_lookup`): the vnode `cnp` names in the directory `vdp`, into `vpp`.
pub fn fusefs_lookup<G: ProcGlue>(
    vdp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let dp = vtoi(vdp); // inode for directory being searched
    let fmp = dp.i_fmp.clone(); // file system that directory is in
    let p = cnp.proc();
    let cred = cnp.cn_cred;
    let nameiop = cnp.cn_nameiop;

    let flags = cnp.cn_flags;
    *vpp = None;
    let lockparent = flags & LOCKPARENT != 0;
    let wantparent = flags & (LOCKPARENT | WANTPARENT) != 0;

    vop_access(vdp, VEXEC, cred, p)?;

    if flags & ISLASTCN != 0
        && vmount(vdp).flag() & MNT_RDONLY != 0
        && (nameiop == DELETE || nameiop == RENAME)
    {
        return Err(Errno::EROFS);
    }

    // FUSE doesn't send . or .. lookups to userland so they must be handled here. The
    // parent node id is only cached for directories and will be refreshed below the next
    // time the directory is looked up by name.
    let nid: Ino;
    let mut nvtype: Option<Vtype> = None;
    if cnp.name() == b"." {
        nid = dp.i_number;
    } else if flags & ISDOTDOT != 0 {
        nid = dp.i_parent_cache.get();
    } else {
        if fmp.sess_init.get() == 0 {
            return Err(Errno::ENOENT);
        }

        // got a real entry
        let name = cnp.name();
        let fbuf = fb_setup(name.len() + 1, dp.i_number, FUSE_LOOKUP, p);
        fbuf.dat_update(|d| {
            d[..name.len()].copy_from_slice(name);
            d[name.len()] = 0;
        });

        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            fb_delete(fbuf);

            // file system is dead
            if error == Errno::ENXIO {
                return Err(error);
            }

            if (nameiop == CREATE || nameiop == RENAME) && flags & ISLASTCN != 0 {
                // Access for write is interpreted as allowing creation of files in the
                // directory.
                vop_access(vdp, VWRITE, cred, p)?;

                cnp.cn_flags |= SAVENAME;

                if !lockparent {
                    let _ = vop_unlock(vdp);
                    cnp.cn_flags |= PDIRUNLOCK;
                }

                return Err(Errno::EJUSTRETURN);
            }

            return Err(Errno::ENOENT);
        }

        let entry: FuseEntryOut = fbuf.op_get();
        nid = entry.nodeid;
        nvtype = Some(iftovt(entry.attr.mode));
        fb_delete(fbuf);

        // An error of ENOENT or an inode value of 0 mean the entry was not found. The
        // difference is that 0 indicates that the result may be cached. We don't support
        // caching yet so just return.
        if nid == 0 {
            return Err(Errno::ENOENT);
        }
    }

    let error = 'reclaim: {
        if nameiop == DELETE && flags & ISLASTCN != 0 {
            // Write access to directory required to delete files.
            if let Err(e) = vop_access(vdp, VWRITE, cred, p) {
                break 'reclaim e;
            }

            cnp.cn_flags |= SAVENAME;
        }

        if nameiop == RENAME && wantparent && flags & ISLASTCN != 0 {
            // Write access to directory required to delete files.
            if let Err(e) = vop_access(vdp, VWRITE, cred, p) {
                break 'reclaim e;
            }

            if nid == dp.i_number {
                return Err(Errno::EISDIR);
            }

            let tdp = match VFS_VGET(fmp.mp, nid) {
                Ok(tdp) => tdp,
                Err(e) => break 'reclaim e,
            };

            if let Some(t) = nvtype {
                tdp.set_vtype(t);
            }
            vtoi(tdp).i_parent_cache.set(dp.i_number);
            *vpp = Some(tdp);
            cnp.cn_flags |= SAVENAME;

            return Ok(());
        }

        if flags & ISDOTDOT != 0 {
            let _ = vop_unlock(vdp); // race to get the inode
            cnp.cn_flags |= PDIRUNLOCK;

            let tdp = match VFS_VGET(fmp.mp, nid) {
                // DPRINTF("%s: parent not dir: %s\n", __func__, cnp->cn_nameptr): the
                // vnode is not released (the module's "kept as the C").
                Ok(tdp) if tdp.vtype() != VDIR => Err(Errno::EIO),
                r => r,
            };
            let tdp = match tdp {
                Ok(tdp) => tdp,
                Err(e) => {
                    if vn_lock(vdp, LK_EXCLUSIVE | LK_RETRY).is_ok() {
                        cnp.cn_flags &= !PDIRUNLOCK;
                    }

                    break 'reclaim e;
                }
            };

            if lockparent && flags & ISLASTCN != 0 {
                if let Err(e) = vn_lock(vdp, LK_EXCLUSIVE) {
                    vput(tdp);
                    return Err(e);
                }
                cnp.cn_flags &= !PDIRUNLOCK;
            }
            *vpp = Some(tdp);

            // Didn't actually make a call but vget increments lookup
            let tip = vtoi(tdp);
            tip.nlookup.set(tip.nlookup.get().wrapping_sub(1));
        } else if nid == dp.i_number {
            vref(vdp);
            *vpp = Some(vdp);
        } else {
            let tdp = match VFS_VGET(fmp.mp, nid) {
                Ok(tdp) => tdp,
                Err(e) => break 'reclaim e,
            };

            if let Some(t) = nvtype {
                tdp.set_vtype(t);
            }
            let tip = vtoi(tdp);
            tip.i_parent_cache.set(dp.i_number);

            // Cache the parent if it's a directory so that we can resolve any .. lookups
            // later.
            if tdp.vtype() == VDIR {
                tip.i_parent_cache.set(dp.i_number);
            }

            if !lockparent || flags & ISLASTCN == 0 {
                let _ = vop_unlock(vdp);
                cnp.cn_flags |= PDIRUNLOCK;
            }

            *vpp = Some(tdp);
        }

        return Ok(());
    };

    // reclaim:
    if nid != dp.i_number && nid != FUSE_ROOT_ID {
        let fbuf = fb_setup(0, nid, FUSE_FORGET, p);
        fbuf.op_set(&FuseForgetIn { nlookup: 1 });
        // no response: the caller does not sleep, whatever the answer
        let _ = fuse_device_queue_fbuf::<G>(fmp.dev, fbuf);
    }
    Err(error)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the lookup over testfuse's mount and scripted daemon: a name asked of
    // the daemon (its bytes and NUL), `.` and `..` in the kernel (`..` unlocking the
    // directory, the parent's `nlookup` given back, the child locked again under
    // LOCKPARENT), the refusals (`ENOENT` for an error or a 0 inode, `EJUSTRETURN` for a
    // creation, `EROFS`, `EISDIR`, a `..` that is no directory with the `FUSE_FORGET` of the
    // reclaim path), and a session not up.

    use std::{assert, assert_eq, assert_ne};

    use hal::sys::mount::MNT_RDONLY;
    use hal::sys::types::Dev;

    use super::*;
    use crate::fs::fuse::fuse_device::fuseclose;
    use crate::fs::fuse::fuse_vfsops::fusefs_unmount;
    use crate::fs::fuse::fusebuf_h::{FUSE_GETATTR, FuseAttrOut};
    use crate::fs::fuse::testfuse::{
        Answer, hello_attr, hello_fs, mounted, script, scripted, set_daemon, take_ops, take_seen,
    };
    use crate::fs::mount::Mount;
    use crate::fs::namei::{Componentname, LOOKUP};
    use crate::fs::subr::vrele;
    use crate::fs::testfs::TestGlue;
    use crate::fs::vops::{Cred, vop_islocked};
    use crate::proc::Thread;

    type P = &'static Thread<TestGlue>;
    type V = &'static Vnode<TestGlue>;

    /// `VOP_LOOKUP(dvp, name)` for `op` with `flags`: the result, the vnode and the flags the
    /// lookup left.
    fn look(
        p: P,
        dvp: V,
        name: &'static [u8],
        op: u64,
        flags: u64,
    ) -> (Result<(), Errno>, Option<V>, u64) {
        let mut cnp = Componentname::new(op, flags, p, Cred::real(p.ucred()));
        cnp.set_name_lit(name);
        let mut vpp = None;
        let r = fusefs_lookup(dvp, &mut vpp, &mut cnp);
        (r, vpp, cnp.cn_flags)
    }

    /// The daemon's tree: the root (1) holding `hello` (2) and the directory `d` (3), whose
    /// `self` names `d` again and whose `zero` answers inode 0.
    fn with_dir() {
        hello_fs(b"x");
        script(FUSE_LOOKUP, |r| {
            let (ino, mode) = match (r.hdr.nodeid, &r.data[..]) {
                (1, b"hello\0") => (2, 0o100644),
                (1, b"d\0") | (3, b"self\0") => (3, 0o040755),
                (3, b"zero\0") => (0, 0),
                _ => return Answer::err(Errno::ENOENT),
            };
            let mut attr = hello_attr(ino);
            attr.mode = mode;
            Answer::out(&FuseEntryOut {
                nodeid: ino,
                attr,
                ..FuseEntryOut::default()
            })
        });
        script(FUSE_GETATTR, |r| {
            let mut attr = hello_attr(r.hdr.nodeid);
            if r.hdr.nodeid == 3 {
                attr.mode = 0o040755;
            }
            Answer::out(&FuseAttrOut {
                attr,
                ..FuseAttrOut::default()
            })
        });
    }

    /// Unmounts (the root given back) and closes the device of [`mounted`].
    fn done(p: P, dev: Dev, mp: &'static Mount<TestGlue>, root: V) {
        vput(root);
        fusefs_unmount(mp, 0, p).expect("unmount");
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn names_dot_and_dotdot() {
        let (_g, p, dev, mp, root) = mounted();
        with_dir();

        // A name: FUSE_LOOKUP with the name and its NUL, then the node's GETATTR; the
        // directory stays locked (LOCKPARENT on the last component).
        let (r, vp, flags) = look(p, root, b"d", LOOKUP, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Ok(()));
        let d = vp.expect("a vnode");
        let seen = take_seen();
        assert_eq!(
            seen.iter()
                .map(|r| r.hdr.opcode)
                .collect::<std::vec::Vec<_>>(),
            [FUSE_GETATTR, FUSE_LOOKUP, FUSE_GETATTR],
            "the access check, the name, the new node"
        );
        assert_eq!((seen[1].hdr.nodeid, &seen[1].data[..]), (1, &b"d\0"[..]));
        assert_eq!(flags & PDIRUNLOCK, 0);
        assert_eq!(d.vtype(), VDIR);
        let di = vtoi(d);
        assert_eq!(
            (di.i_number, di.i_parent_cache.get(), di.nlookup.get()),
            (3, 1, 1)
        );

        // ".": the directory itself, referenced, no request but the access check.
        let (r, vp, _) = look(p, d, b".", LOOKUP, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Ok(()));
        assert!(vp.is_some_and(|v| core::ptr::eq(v, d)));
        vrele(d);
        assert_eq!(take_ops(), [FUSE_GETATTR]);

        // "..": the parent from i_parent_cache; the directory unlocked across VFS_VGET
        // (PDIRUNLOCK, no LOCKPARENT), the parent's lookup count given back.
        let rn = vtoi(root).nlookup.get();
        let _ = vop_unlock(root);
        let (r, vp, flags) = look(p, d, b"..", LOOKUP, ISDOTDOT);
        assert_eq!(r, Ok(()));
        assert!(vp.is_some_and(|v| core::ptr::eq(v, root)));
        assert_ne!(flags & PDIRUNLOCK, 0);
        assert_eq!(vop_islocked(d), 0, "the child left unlocked");
        assert_eq!(vtoi(root).nlookup.get(), rn, "vget's lookup given back");
        vput(root);

        // ".." under LOCKPARENT | ISLASTCN locks the child again.
        vn_lock(d, LK_EXCLUSIVE).expect("lock");
        let (r, vp, flags) = look(p, d, b"..", LOOKUP, ISDOTDOT | LOCKPARENT | ISLASTCN);
        assert_eq!(r, Ok(()));
        assert_eq!(flags & PDIRUNLOCK, 0);
        assert_ne!(vop_islocked(d), 0);
        assert!(vp.is_some_and(|v| core::ptr::eq(v, root)));
        vrele(root);
        vput(d);
        let _ = take_seen();
        done(p, dev, mp, root);
    }

    #[test]
    fn the_refusals() {
        let (_g, p, dev, mp, root) = mounted();
        with_dir();

        // The daemon's ENOENT, and its "0: may be cached".
        let (r, vp, _) = look(p, root, b"nothere", LOOKUP, LOCKPARENT | ISLASTCN);
        assert_eq!((r, vp.is_none()), (Err(Errno::ENOENT), true));
        let (r, d, _) = look(p, root, b"d", LOOKUP, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Ok(()));
        let d = d.expect("d");
        let (r, _, _) = look(p, d, b"zero", LOOKUP, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Err(Errno::ENOENT));

        // CREATE of a missing last component: EJUSTRETURN, SAVENAME, the directory unlocked
        // without LOCKPARENT.
        let (r, _, flags) = look(p, d, b"new", CREATE, ISLASTCN);
        assert_eq!(r, Err(Errno::EJUSTRETURN));
        assert_ne!(flags & SAVENAME, 0);
        assert_ne!(flags & PDIRUNLOCK, 0);
        assert_eq!(vop_islocked(d), 0);
        vn_lock(d, LK_EXCLUSIVE).expect("lock");

        // DELETE saves the name.
        let (r, h, flags) = look(p, root, b"hello", DELETE, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Ok(()));
        assert_ne!(flags & SAVENAME, 0);
        let h = h.expect("hello");
        vput(h);

        // RENAME's target naming the directory itself: EISDIR, no FORGET.
        let _ = take_seen();
        let (r, _, _) = look(p, d, b"self", RENAME, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Err(Errno::EISDIR));
        scripted(dev);
        assert!(!take_ops().contains(&FUSE_FORGET));

        // A read-only mount refuses DELETE and RENAME of the last component.
        mp.set_flag(mp.flag() | MNT_RDONLY);
        let (r, _, _) = look(p, root, b"hello", DELETE, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Err(Errno::EROFS));
        let (r, _, _) = look(p, root, b"hello", RENAME, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Err(Errno::EROFS));
        mp.set_flag(mp.flag() & !MNT_RDONLY);

        // A ".." whose node is no directory: EIO, the directory locked again, the node
        // forgotten (reclaim:); the vnode VFS_VGET gave is not given back (the C's), so the
        // test gives it back.
        vtoi(d).i_parent_cache.set(2);
        let _ = take_seen();
        let (r, vp, flags) = look(p, d, b"..", LOOKUP, ISDOTDOT);
        assert_eq!((r, vp.is_none()), (Err(Errno::EIO), true));
        assert_eq!(flags & PDIRUNLOCK, 0, "locked again");
        assert_ne!(vop_islocked(d), 0);
        scripted(dev); // the FORGET waits on the device
        let seen = take_seen();
        let forget = seen
            .iter()
            .find(|r| r.hdr.opcode == FUSE_FORGET)
            .expect("FORGET");
        assert_eq!(
            (forget.hdr.nodeid, forget.op::<FuseForgetIn>().nlookup),
            (2, 1)
        );
        vput(h);
        vput(d);

        // The session not up: the access check refuses first (ENXIO), no request.
        let fmp = vtoi(root).i_fmp.clone();
        fmp.sess_init.set(0);
        let (r, _, _) = look(p, root, b"hello", LOOKUP, LOCKPARENT | ISLASTCN);
        assert_eq!(r, Err(Errno::ENXIO));
        assert!(take_seen().is_empty());
        fmp.sess_init.set(1);
        drop(fmp);
        done(p, dev, mp, root);
    }
}
/* </TESTS> */
