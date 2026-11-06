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
//! The FUSE file-system-type operations (`fusefs_vfsops`, [`FusefsVfs`]): mount (attach the
//! daemon's fuse(4) device and send `FUSE_INIT`), unmount (`FUSE_DESTROY`), root, statfs
//! (`FUSE_STATFS`), vget (the inode hash, a new node and `FUSE_GETATTR`), init (the fusebuf
//! pool and the hash) and the `vfs.fuse` sysctls.
//!
//! Upstream: sys/miscfs/fuse/fuse_vfsops.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fuse_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - `fusefs_mount` reads its `struct fusefs_args` out of the kernel copy of the mount
//!   arguments (`FusefsArgs::from_bytes`), `EINVAL` when they are short; the C dereferences
//!   the pointer it is given (LZ's).
//! - `fusefs_mount` refuses `ENXIO` a character device that is not an open fuse(4) unit (no
//!   open unit of the vnode's whole `dev_t`, `fuse_lookup`'s match) and `EBUSY` a unit that
//!   already serves a mount, at the C's `VCHR` test: the C mounts the first (its later
//!   requests go nowhere and the first access of the mount point sleeps for ever, or reach a
//!   daemon of another device with that minor) and lets the second overwrite the unit's
//!   `fd_fmp`, so the first mount's requests answer through the second's session. libfuse
//!   opens the device and then mounts with that descriptor, so a correct daemon sees nothing
//!   (the user's D-N5.9-U1). The test sits where the C's tests are, before the mount's
//!   `M_WAITOK` `malloc` (a refusal there asks for no memory), and is made again right before
//!   the mount attaches to the unit (`fusefs_mount_attach`, with nothing that sleeps between
//!   that test and the attachment), since the unit may have closed or another mount attached
//!   it while this one slept in the `malloc`; a refusal there gives the mount's `M_FUSEFS`
//!   block back, one `vmstat -m` request on that race path only.
//! - `f_mntonname` (and the two `"fusefs"` names) are `strlcpy`'d with the C's `MNAMELEN`: a
//!   mount point of 89 characters is kept whole, where LZ's `MNAMELEN - 1` cut it to 88 (an
//!   LZ slip the C does not have, fuse_vfsops.c:126; D-N5.9-10).
//! - `fusefs_root` and `fusefs_vget` return the vnode (`Vfs`'s shape) (LZ's).
//! - `fusefs_vget`'s `curproc->p_ucred, curproc` is the glue's `curproc`, which panics
//!   without a thread (LZ's).
//! - `vfs_quotactl` is [`fusefs_quotactl`]; `vfs_checkexp`'s `fusefs_checkexp` (`EOPNOTSUPP`)
//!   is `VFS_CHECKEXP`'s arm in `bsd`, the NFS server's (the kernel's `Vfs` has no slot).
//!
//! Kept as the C, named: the vnode lock is held across `fusefs_vget`'s `FUSE_GETATTR` and
//! `fusefs_statfs`'s `FUSE_STATFS` sleeps, so a daemon that stats its own mount before
//! answering `FUSE_INIT` gets the dummies, as the C's comment says (D-N5.9-9).
//!
//! ## Redesign
//! - LZ's `FUSEFS_VFSOPS` (a `bsd` `Vfsops` of function pointers) -> `impl Vfs<G> for
//!   FusefsVfs<G>` and its constant [`FusefsVfs::VFS`] (tmpfs's and udf's shape), the
//!   functions the C's, generic over the glue, in its order.
//! - `struct fusefs_mnt` is a counted `malloc(M_FUSEFS)` block made at the C's place, after
//!   every refusal, and hung from the mount's typed slot where the C sets `mnt_data`; the
//!   device holds a handle from `fuse_device_set_fmp(fmp, 1)` to the unmount's
//!   `fuse_device_set_fmp(fmp, 0)`, every node one for its life; `fusefs_unmount` takes the
//!   slot's handle back (the C's `free(fmp, M_FUSEFS)` with the last handle). LZ wrote it
//!   into a raw block and freed it by hand (2 `unsafe`, with the node's in `fusefs_vget`).
//! - A node is a counted `malloc(M_FUSEFS)` block in the vnode's typed slot
//!   (`FusefsNode::new` the C's `M_ZERO` fill and `rrw_init_flags(.., "fuseinode",
//!   RWL_DUPOK | RWL_IS_VNODE)`), its handles marked invalid as the C's loop does.
//! - The C's `fusefs_fbuf_pool`, `stat_*` counters and the hash are the glue's `FuseTables`
//!   (`fusefs.rs`); `fusefs_vars[]`'s `&fusefs_fbuf_pool.pr_npages` is the pool's page count
//!   read at the request (`TypedPool::npages`, N5.9f2's `hal` accessor) where LZ refreshed a
//!   mirror atomic from bsd's pool before each lookup; the three counters are the table's
//!   atomics, served by `sysctl_bounded_arr` as the C's.

use core::marker::PhantomData;
use core::sync::atomic::AtomicI32;

use hal::intr::IPL_NONE;
use hal::malloc::{M_WAITOK, MArc};
use hal::pool::PR_WAITOK;
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, FusefsArgs, MNT_FORCE, MNT_UPDATE, Statfs};
use hal::sys::sysctl::{SYSCTL_INT_READONLY, SysctlBoundedArgs};
use hal::sys::types::{Ino, Uid};

use crate::assert::kpanic;
use crate::cred::suser_ucred;
use crate::descrip::fd_getfile;
use crate::file::{DTYPE_VNODE, FileGlue};
use crate::fs::fuse::fuse_device::{
    fuse_device_attach, fuse_device_cleanup, fuse_device_queue_fbuf, fuse_device_set_fmp,
    fuse_lookup,
};
use crate::fs::fuse::fuse_ihash::{fuse_ihashget, fuse_ihashinit, fuse_ihashins};
use crate::fs::fuse::fusebuf::{fb_delete, fb_queue, fb_setup};
use crate::fs::fuse::fusebuf_h::{
    FUSE_DESTROY, FUSE_INIT, FUSE_KERNEL_MINOR_VERSION, FUSE_KERNEL_VERSION, FUSE_ROOT_ID,
    FUSE_STATFS, FUSEBUFMAXSIZE, FuseInitIn, FuseStatfsOut,
};
use crate::fs::fuse::fusefs::{
    FUSEFS_INFBUFS, FUSEFS_OPENDEVS, FUSEFS_POOL_NBPAGES, FUSEFS_WAITFBUFS, FusefsMnt,
    FusefsMntRef, M_FUSEFS, vfstofusefs,
};
use crate::fs::fuse::fusefs_node::{FUFH_INVALID, FufhKind, FusefsFilehandle, FusefsNode, vtoi};
use crate::fs::mount::{Mount, Vfsconf};
use crate::fs::spec::BLKDEV_IOSIZE;
use crate::fs::subr::{copy_statfs_info, getnewvnode, vflush, vfs, vfs_getnewfsid, vrele};
use crate::fs::vnode::{FORCECLOSE, VCHR, VDIR, VROOT, VT_FUSEFS, Vattr, Vnode};
use crate::fs::vops::{Cred, Nd, VFS_VGET, Vfs, vop_getattr};
use crate::fs::word::{I32Word, U32Word};
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::{SysctlReq, bounded_arr, int_bounded};

/// `PENDING`: `FUSE_INIT` reply not yet received.
pub const PENDING: i32 = 2;

/// The FUSE file system of the glue `G`: `fusefs_vfsops` ([`FusefsVfs::VFS`]).
pub struct FusefsVfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> FusefsVfs<G> {
    /// `fusefs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"fuse"`).
    pub const VFS: FusefsVfs<G> = FusefsVfs(PhantomData);
}

/// `fusefs_mount` (`vfs_mount`): mount system call, made by the daemon: `data` is the kernel
/// copy of its `struct fusefs_args`, whose `fd` is its open fuse(4) device.
pub fn fusefs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if mp.flag() & MNT_UPDATE != 0 {
        return Err(Errno::EOPNOTSUPP);
    }

    let Some(args) = FusefsArgs::from_bytes(data) else {
        return Err(Errno::EINVAL);
    };

    let Some(fp) = fd_getfile(p.fd(), args.fd) else {
        return Err(Errno::EBADF);
    };

    let error = 'bad: {
        if fp.f_type() != DTYPE_VNODE {
            break 'bad Err(Errno::EINVAL);
        }

        let Some(vp) = fp.data().and_then(<G::FileGlue as FileGlue>::vnode_of) else {
            kpanic(format_args!("fusefs_mount: a vnode file without a vnode"));
        };
        if vp.vtype() != VCHR {
            break 'bad Err(Errno::EBADF);
        }

        // An open fuse(4) unit with no mount yet (the module's deviations, D-N5.9-U1).
        match fuse_lookup::<G>(vp.rdev()) {
            None => break 'bad Err(Errno::ENXIO),
            Some(fd) if fd.has_fmp() => break 'bad Err(Errno::EBUSY),
            Some(_) => {}
        }

        // Only root may specify allow_other.
        if args.allow_other != 0
            && let Err(e) = suser_ucred(p.ucred())
        {
            break 'bad Err(e);
        }

        let max_read = if args.max_read > 0 {
            args.max_read.min(FUSEBUFMAXSIZE as i32)
        } else {
            FUSEBUFMAXSIZE as i32
        };
        let Some(fmp) = MArc::new(
            M_FUSEFS,
            M_WAITOK,
            FusefsMnt {
                mp,
                undef_op: U32Word::new(0),
                max_read,
                // Initialise to a safe value just to be sure. This will be overwritten when
                // the file system responds to FUSE_INIT.
                max_write: I32Word::new(BLKDEV_IOSIZE),
                sess_init: I32Word::new(PENDING),
                allow_other: args.allow_other,
                dev: vp.rdev(),
            },
        ) else {
            kpanic(format_args!("fusefs_mount: malloc(M_WAITOK) failed"));
        };

        mp.set_node(MArc::erase(fmp.clone()));
        vfs_getnewfsid(mp);

        mp.with_stat(|sp| {
            sp.f_mntonname.fill(0);
            libkern::strlcpy(&mut sp.f_mntonname, path);
            sp.f_mntfromname.fill(0);
            libkern::strlcpy(&mut sp.f_mntfromname, b"fusefs");
            sp.f_mntfromspec.fill(0);
            libkern::strlcpy(&mut sp.f_mntfromspec, b"fusefs");
        });

        // fuse_device_set_fmp(fmp, 1), after U1's test made again.
        if let Err(e) = fusefs_mount_attach(mp, &fmp) {
            break 'bad Err(e);
        }
        let fbuf = fb_setup(0, 0, FUSE_INIT, p);
        fbuf.op_set(&FuseInitIn {
            major: FUSE_KERNEL_VERSION,
            minor: FUSE_KERNEL_MINOR_VERSION,
            max_readahead: 0,
            flags: 0, // OpenBSD supports nothing...
        });

        // cannot tsleep on mount: the device keeps the request (nobody waits for its reply)
        let _ = fuse_device_queue_fbuf::<G>(fmp.dev, fbuf);

        Ok(())
    };

    // bad:
    let _ = fp.frele(Some(p));
    error
}

/// `fuse_device_set_fmp(fmp, 1)` of `fusefs_mount`, after U1's test made again (the
/// module's deviations): `ENXIO` when no unit of the mount's `dev_t` is open any more, `EBUSY`
/// when another mount attached it meanwhile, the mount's slot then giving its block up (the
/// block goes with the caller's handle). Nothing sleeps between the test and the attachment.
fn fusefs_mount_attach<G: ProcGlue>(mp: &Mount<G>, fmp: &FusefsMntRef<G>) -> Result<(), Errno> {
    let error = match fuse_lookup::<G>(fmp.dev) {
        None => Err(Errno::ENXIO),
        Some(fd) if fd.has_fmp() => Err(Errno::EBUSY),
        Some(fd) => {
            fuse_device_attach(&fd, fmp);
            return Ok(());
        }
    };
    drop(mp.take_node());
    error
}

/// `fusefs_start` (`vfs_start`).
pub fn fusefs_start() -> Result<(), Errno> {
    Ok(())
}

/// `fusefs_unmount` (`vfs_unmount`): flushes the vnodes, tells a live daemon
/// (`FUSE_DESTROY`), fails what is still queued and detaches the device.
pub fn fusefs_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let fmp = vfstofusefs(mp);
    let mut flags = 0;

    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }

    vflush(mp, None, flags)?;

    if fmp.sess_init.get() != 0 && fmp.sess_init.get() != PENDING {
        let fbuf = fb_setup(0, 0, FUSE_DESTROY, p);

        let _error = fb_queue::<G>(fmp.dev, &fbuf);
        // if (error) DPRINTF("error %d on destroy\n", error);

        fb_delete(fbuf);
    }
    fmp.sess_init.set(0);

    fuse_device_cleanup::<G>(fmp.dev);
    fuse_device_set_fmp(&fmp, false);
    // free(fmp, M_FUSEFS, sizeof(*fmp)); mp->mnt_data = NULL: the slot's handle goes, the
    // last one (`fmp`, below) frees the block.
    drop(mp.take_node());

    Ok(())
}

/// `fusefs_root` (`vfs_root`): the root directory's vnode (`FUSE_ROOT_ID`), locked.
pub fn fusefs_root<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    let nvp = VFS_VGET(mp, FUSE_ROOT_ID)?;

    nvp.set_vtype(VDIR);

    Ok(nvp)
}

/// `fusefs_quotactl` (`vfs_quotactl`).
pub fn fusefs_quotactl() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `fusefs_statfs` (`vfs_statfs`): the daemon's `FUSE_STATFS` answer once the session is
/// up, zeros before.
pub fn fusefs_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let fmp = vfstofusefs(mp);

    // Deny other users unless allow_other mount option was specified.
    if fmp.allow_other == 0 && p.ucred().cr_uid.get() != mp.owner() {
        return Err(Errno::EPERM);
    }

    copy_statfs_info(sbp, mp);

    // Both FUSE_INIT and FUSE_STATFS are sent to the FUSE file system daemon when it is
    // mounted. However, the daemon is the process that called mount(2) so to prevent a
    // deadlock return dummy values until the response to FUSE_INIT init is received. All
    // other VFS syscalls are queued.
    if fmp.sess_init.get() == 0 || fmp.sess_init.get() == PENDING {
        sbp.f_bavail = 0;
        sbp.f_bfree = 0;
        sbp.f_blocks = 0;
        sbp.f_ffree = 0;
        sbp.f_favail = 0;
        sbp.f_files = 0;
        sbp.f_bsize = 0;
        sbp.f_iosize = 0;
        sbp.f_namemax = 0;
    } else {
        let fbuf = fb_setup(0, FUSE_ROOT_ID, FUSE_STATFS, p);

        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            fb_delete(fbuf);
            return Err(error);
        }

        let st = fbuf.op_get::<FuseStatfsOut>().st;
        sbp.f_blocks = st.blocks;
        sbp.f_bfree = st.bfree;
        sbp.f_bavail = st.bavail as i64;
        sbp.f_files = st.files;
        sbp.f_ffree = st.ffree;
        sbp.f_favail = st.ffree as i64;
        sbp.f_bsize = st.frsize;
        sbp.f_namemax = st.namelen;
        sbp.f_iosize = st.bsize;

        // Programs use this to allocate an i/o buffer so ensure it's sane.
        //
        // XXX Should this be larger?
        if sbp.f_iosize > BLKDEV_IOSIZE as u32 {
            sbp.f_iosize = BLKDEV_IOSIZE as u32;
        }

        fb_delete(fbuf);
    }

    Ok(())
}

/// `fusefs_sync` (`vfs_sync`).
pub fn fusefs_sync() -> Result<(), Errno> {
    Ok(())
}

/// `fusefs_vget` (`vfs_vget`): the vnode of inode `ino`, referenced and locked: from the
/// inode hash (one more lookup), or a new node whose size `FUSE_GETATTR` gives.
pub fn fusefs_vget<G: ProcGlue>(
    mp: &'static Mount<G>,
    ino: Ino,
) -> Result<&'static Vnode<G>, Errno> {
    loop {
        // retry:
        let fmp = vfstofusefs(mp);

        // check if vnode is in hash.
        if let Some(vp) = fuse_ihashget(&fmp, ino) {
            let ip = vtoi(vp);
            ip.nlookup.set(ip.nlookup.get() + 1);
            return Ok(vp);
        }

        // if not create it
        let nvp = getnewvnode(VT_FUSEFS, Some(mp), &vfs::<G>().fuse.vops)?;
        // if error: DPRINTF("getnewvnode error: %d\n", error);

        // malloc(M_WAITOK | M_ZERO), rrw_init_flags(&ip->i_lock, "fuseinode", RWL_DUPOK |
        // RWL_IS_VNODE), i_vnode, i_fmp and i_number: `FusefsNode::new`.
        let Some(ip) = MArc::new(M_FUSEFS, M_WAITOK, FusefsNode::new(nvp, fmp.clone(), ino)) else {
            kpanic(format_args!("fusefs_vget: malloc(M_WAITOK) failed"));
        };
        nvp.set_node(MArc::erase(ip.clone()));
        ip.nlookup.set(1);

        for t in FufhKind::ALL {
            ip.set_fufh(
                t,
                FusefsFilehandle {
                    fh_id: ip.fufh(t).fh_id,
                    fh_type: FUFH_INVALID,
                },
            );
        }

        if let Err(error) = fuse_ihashins(ip.as_pin()) {
            vrele(nvp);

            if error == Errno::EEXIST {
                continue;
            }

            return Err(error);
        }

        if ino == FUSE_ROOT_ID {
            nvp.flag_set(VROOT);
        } else {
            // Initialise the file size so that file size changes can be detected during
            // file operations.
            let mut vattr = Vattr::new();
            let error = G::with_curproc(|p| {
                let Some(p) = p else {
                    kpanic(format_args!("fusefs_vget: no curproc"));
                };
                vop_getattr(nvp, &mut vattr, Cred::real(p.ucred()), p)
            });
            if let Err(error) = error {
                vrele(nvp);
                return Err(error);
            }
            ip.filesize.set(vattr.va_size as i64);
        }

        return Ok(nvp);
    }
}

/// `fusefs_fhtovp` (`vfs_fhtovp`): FUSE has no file handles.
pub fn fusefs_fhtovp<G: ProcGlue>() -> Result<&'static Vnode<G>, Errno> {
    Err(Errno::EINVAL)
}

/// `fusefs_vptofh` (`vfs_vptofh`).
pub fn fusefs_vptofh() -> Result<(), Errno> {
    Err(Errno::EINVAL)
}

/// `fusefs_init` (`vfs_init`): the fusebuf pool and the inode hash; a second call (a host
/// test's) keeps the pool made first (its `init` answers `EBUSY`).
pub fn fusefs_init<G: ProcGlue>() -> Result<(), Errno> {
    match vfs::<G>()
        .fuse
        .fbuf_pool
        .init_recorded(IPL_NONE, PR_WAITOK, "fmsg")
    {
        Ok(()) | Err(Errno::EBUSY) => {}
        Err(e) => return Err(e),
    }
    fuse_ihashinit::<G>();

    Ok(())
}

/// `fusefs_sysctl` (`vfs_sysctl`): `vfs.fuse.*`, `fusefs_vars[]` (all read-only): the open
/// devices, the requests to read, those waiting for a reply, the `"fmsg"` pool's pages.
pub fn fusefs_sysctl<G: ProcGlue>(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let t = &vfs::<G>().fuse;
    let mut req = SysctlReq {
        oldp,
        oldlenp,
        newp,
        newlen,
    };
    // `&fusefs_fbuf_pool.pr_npages`: the count as the pool has it now.
    if name == [FUSEFS_POOL_NBPAGES] {
        let npages = AtomicI32::new(t.fbuf_pool.npages() as i32);
        return int_bounded(
            &mut req,
            &npages,
            SYSCTL_INT_READONLY.0,
            SYSCTL_INT_READONLY.1,
        );
    }
    let vars = [
        SysctlBoundedArgs::readonly(FUSEFS_OPENDEVS, &t.stat_opened_fusedev),
        SysctlBoundedArgs::readonly(FUSEFS_INFBUFS, &t.stat_fbufs_in),
        SysctlBoundedArgs::readonly(FUSEFS_WAITFBUFS, &t.stat_fbufs_wait),
    ];
    bounded_arr(&vars, name, &mut req)
}

/// `fusefs_vfsops`: each slot the C's function of the same name; `vfs_checkexp` is
/// `VFS_CHECKEXP`'s arm (the module's deviations).
impl<G: ProcGlue> Vfs<G> for FusefsVfs<G> {
    /// `vfs_mount`: [`fusefs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`fusefs_start`].
    fn start(&self, _mp: &'static Mount<G>, _flags: i32, _p: &Thread<G>) -> Result<(), Errno> {
        fusefs_start()
    }

    /// `vfs_unmount`: [`fusefs_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        fusefs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: [`fusefs_root`].
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        fusefs_root(mp)
    }

    /// `vfs_quotactl`: [`fusefs_quotactl`].
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_quotactl()
    }

    /// `vfs_statfs`: [`fusefs_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        fusefs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`fusefs_sync`].
    fn sync(
        &self,
        _mp: &'static Mount<G>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_sync()
    }

    /// `vfs_vget`: [`fusefs_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        fusefs_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`fusefs_fhtovp`].
    fn fhtovp(&self, _mp: &'static Mount<G>, _fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        fusefs_fhtovp()
    }

    /// `vfs_vptofh`: [`fusefs_vptofh`].
    fn vptofh(&self, _vp: &'static Vnode<G>, _fid: &mut Fid) -> Result<(), Errno> {
        fusefs_vptofh()
    }

    /// `vfs_init`: [`fusefs_init`].
    fn init(&self, _vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        fusefs_init::<G>()
    }

    /// `vfs_sysctl`: [`fusefs_sysctl`].
    fn sysctl(
        &self,
        name: &[i32],
        oldp: usize,
        oldlenp: &mut usize,
        newp: usize,
        newlen: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_sysctl(name, oldp, oldlenp, newp, newlen, p)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for a FUSE mount driven end to end by a simulated daemon (LZ's two moved,
    // over testfuse's scripted daemon and `hello_fs`): `fusefs_mount` (the `FUSE_INIT` it
    // queues, answered on the first request), `statfs` before and after the session starts,
    // the root, lookups through the inode hash, `getattr`, `open`, `read`, `write`,
    // `setattr`, `readdir` in two calls, the vnode release (`FUSE_RELEASE*`, `FUSE_FORGET` on
    // reclaim) and `fusefs_unmount` (`FUSE_DESTROY`); a dead daemon. New at N5.9f2: the
    // mount's refusals with the user's D-N5.9-U1 (`ENXIO` for a character device that is not
    // an open fuse unit, `EBUSY` for a unit already mounted), `f_mntonname`'s 89 characters
    // (D-N5.9-10), `FUSE_INIT`'s version checks, an unmount with requests in flight, the
    // `vfs.fuse` sysctls (the pool's pages through `TypedPool::npages`), the other user's
    // `statfs`.

    use core::ptr;
    use std::{assert, assert_eq};

    use hal::sys::mount::MNAMELEN;
    use hal::sys::types::{Dev, makedev};
    use hal::user::bytes_of;

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::fs::fuse::fuse_device::fuseclose;
    use crate::fs::fuse::fusebuf_h::{
        FUSE_FORGET, FUSE_GETATTR, FUSE_LOOKUP, FUSE_OPENDIR, FUSE_READDIR, FUSE_RELEASE,
        FUSE_RELEASEDIR, FuseInitOut,
    };
    use crate::fs::fuse::fusefs_node::{FUFH_RDONLY, FUFH_WRONLY};
    use crate::fs::fuse::testfuse::{
        args, content, hello_fs, install_dev_fd, install_vnode_fd, mount, open_dev, rdwr, read_req,
        readdir, reply, scripted, set_daemon, setup, stats, take_ops, take_seen,
    };
    use crate::fs::namei::{Componentname, ISLASTCN, LOCKPARENT, LOOKUP};
    use crate::fs::subr::{cdevvp, vattr_null, vput};
    use crate::fs::testfs::{TestGlue, UBUF};
    use crate::fs::vnode::VREG;
    use crate::fs::vops::{VFS_ROOT, vop_getattr, vop_lookup, vop_open, vop_setattr, vop_unlock};
    use hal::sys::dirent::{DT_DIR, DT_REG};
    use hal::sys::fcntl::{FREAD, FWRITE, O_RDONLY, O_WRONLY};
    use hal::sys::types::Off;
    use hal::user::UserPtr;

    type P = &'static Thread<TestGlue>;
    type M = &'static Mount<TestGlue>;

    /// The file's initial contents.
    const HELLO: &[u8] = b"hello, world\n";

    /// A component name for `name`, with the thread's credentials and no pathname buffer.
    fn cn(p: P, name: &'static [u8], flags: u64) -> Componentname<'static, TestGlue> {
        let mut cnp = Componentname::new(LOOKUP, flags, p, Cred::real(p.ucred()));
        cnp.set_name_lit(name);
        cnp
    }

    /// `vfs.fuse.<mib>`: the value, or the error.
    fn sysctl(p: P, name: &[i32]) -> Result<i32, Errno> {
        let mut len = size_of::<i32>();
        fusefs_sysctl(name, UBUF, &mut len, 0, 0, p)?;
        assert_eq!(len, size_of::<i32>());
        let mut b = [0u8; 4];
        UserPtr::<u8>::new(UBUF)
            .copyin_slice(&mut b)
            .expect("copyin");
        Ok(i32::from_ne_bytes(b))
    }

    /// A mount of `/dev/fuse<unit>` (opened, its descriptor installed, the scripted daemon
    /// set), `FUSE_INIT` left unanswered: the device and the mount.
    fn mount_unit(p: P, unit: u32) -> (Dev, M) {
        let dev = open_dev(p, unit);
        let fd = install_dev_fd(p, dev);
        set_daemon(Some(scripted));
        let mp = mount(p, b"/mnt", &mut args(fd, 0, 0)).expect("mount");
        (dev, mp)
    }

    #[test]
    fn a_mount_served_by_a_simulated_daemon() {
        let (_g, p) = setup();
        hello_fs(HELLO);
        let dev = open_dev(p, 0);
        let fd = install_dev_fd(p, dev);
        set_daemon(Some(scripted));

        // mount(MOUNT_FUSEFS, "/mnt", 0, &fargs), as libfuse's fuse_mount does.
        let mut short = [0u8; 8];
        assert_eq!(mount(p, b"/mnt", &mut short).err(), Some(Errno::EINVAL));
        assert_eq!(
            mount(p, b"/mnt", &mut args(fd + 9, 0, 0)).err(),
            Some(Errno::EBADF)
        );
        let mp = mount(p, b"/mnt", &mut args(fd, 0, 0)).expect("mount");
        let fmp = vfstofusefs(mp);
        assert_eq!(fmp.dev, dev);
        assert_eq!(fmp.max_read, FUSEBUFMAXSIZE as i32);
        assert_eq!(fmp.max_write.get(), BLKDEV_IOSIZE);
        assert_eq!(fmp.sess_init.get(), PENDING);
        assert_eq!(&mp.stat().f_mntfromname[..7], b"fusefs\0");
        assert_eq!(&mp.stat().f_mntfromspec[..7], b"fusefs\0");
        assert_eq!(&mp.stat().f_mntonname[..5], b"/mnt\0");
        assert_eq!(stats().1, 1, "FUSE_INIT waits on the device");

        // Before FUSE_INIT is answered statfs returns zeros (the daemon is in mount(2)).
        let mut sb = Statfs::new();
        fusefs_statfs(mp, &mut sb, p).expect("statfs");
        assert_eq!((sb.f_blocks, sb.f_bsize, sb.f_namemax), (0, 0, 0));
        assert!(take_seen().is_empty());

        // The root needs no request.
        let root = VFS_ROOT(mp).expect("root");
        assert_eq!(root.vtype(), VDIR);
        assert!(root.flag() & VROOT != 0);

        // lookup "hello": access (GETATTR of the root, after INIT), LOOKUP, then the new
        // node's GETATTR.
        let mut vpp = None;
        let mut cnp = cn(p, b"hello", LOCKPARENT | ISLASTCN);
        vop_lookup(root, &mut vpp, &mut cnp).expect("lookup");
        let vp = vpp.expect("a vnode");
        assert_eq!(fmp.sess_init.get(), 1, "INIT answered");
        assert_eq!(fmp.max_write.get(), 65536);
        assert_eq!(vp.vtype(), VREG);
        let ip = vtoi(vp);
        assert_eq!(ip.i_number, 2);
        assert_eq!(ip.filesize.get(), HELLO.len() as Off);
        assert_eq!(ip.nlookup.get(), 1);
        assert_eq!(
            take_ops(),
            [FUSE_INIT, FUSE_GETATTR, FUSE_LOOKUP, FUSE_GETATTR]
        );
        let _ = vop_unlock(vp);

        // The second lookup finds the node in the inode hash.
        let mut vpp2 = None;
        let mut cnp = cn(p, b"hello", LOCKPARENT | ISLASTCN);
        vop_lookup(root, &mut vpp2, &mut cnp).expect("lookup");
        assert!(vpp2.is_some_and(|v| ptr::eq(v, vp)));
        assert_eq!(ip.nlookup.get(), 2);
        vput(vp);

        let mut vpp3 = None;
        let mut cnp = cn(p, b"nothere", LOCKPARENT | ISLASTCN);
        assert_eq!(vop_lookup(root, &mut vpp3, &mut cnp), Err(Errno::ENOENT));
        assert!(vpp3.is_none());

        // getattr
        let mut va = Vattr::new();
        vop_getattr(vp, &mut va, Cred::real(p.ucred()), p).expect("getattr");
        assert_eq!(va.va_type, VREG);
        assert_eq!(va.va_mode, 0o644);
        assert_eq!(va.va_size, HELLO.len() as u64);
        assert_eq!(va.va_blocksize, 512, "a zero blksize becomes S_BLKSIZE");
        assert_eq!(va.va_bytes, 512, "blocks computed from the size");
        assert_eq!(
            (va.va_mtime.tv_sec, va.va_mtime.tv_nsec),
            (1_700_000_000, 5)
        );
        assert_eq!(va.va_fileid, 2);

        // open for reading, read
        vop_open(vp, FREAD, Cred::real(p.ucred()), p).expect("open");
        assert_eq!(ip.fufh(FufhKind::RdOnly).fh_id, 10 + O_RDONLY as u64);
        assert_eq!(ip.fufh(FufhKind::RdOnly).fh_type, FUFH_RDONLY);
        let mut buf = [0u8; 64];
        let n = rdwr(vp, p, &mut buf, 0, false).expect("read");
        assert_eq!(&buf[..n], HELLO);
        let n = rdwr(vp, p, &mut buf, 7, false).expect("read");
        assert_eq!(&buf[..n], b"world\n");

        // open for writing, write
        vop_open(vp, FWRITE, Cred::real(p.ucred()), p).expect("open");
        assert_eq!(ip.fufh(FufhKind::WrOnly).fh_id, 10 + O_WRONLY as u64);
        assert_eq!(ip.fufh(FufhKind::WrOnly).fh_type, FUFH_WRONLY);
        let mut hi = *b"HELLO";
        assert_eq!(rdwr(vp, p, &mut hi, 0, true), Ok(5));
        let mut more = *b"!!";
        assert_eq!(rdwr(vp, p, &mut more, HELLO.len() as Off, true), Ok(2));
        assert_eq!(ip.filesize.get(), 15, "a write past the end grows the file");
        assert_eq!(content(), b"HELLO, world\n!!");

        // truncate
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_size = 5;
        vop_setattr(vp, &mut va, Cred::real(p.ucred()), p).expect("setattr");
        assert_eq!(ip.filesize.get(), 5);
        vattr_null(&mut va);
        va.va_flags = 1;
        assert_eq!(
            vop_setattr(vp, &mut va, Cred::real(p.ucred()), p),
            Err(Errno::EOPNOTSUPP)
        );
        vattr_null(&mut va);
        va.va_rdev = 0;
        assert_eq!(
            vop_setattr(vp, &mut va, Cred::real(p.ucred()), p),
            Err(Errno::EINVAL)
        );

        // readdir in two calls: the entries, then the end.
        let _ = take_seen();
        let (entries, end, eof) = readdir(root, p, 0, 512).expect("readdir");
        assert_eq!(
            entries,
            [
                (b".".to_vec(), DT_DIR, 1, 1),
                (b"..".to_vec(), DT_DIR, 1, 2),
                (b"hello".to_vec(), DT_REG, 2, 3)
            ]
        );
        assert_eq!((end, eof), (3, 0));
        let (entries, end, eof) = readdir(root, p, end, 512).expect("readdir");
        assert!(entries.is_empty());
        assert_eq!((end, eof), (3, 1));
        assert_eq!(
            take_ops(),
            [
                FUSE_OPENDIR,
                FUSE_READDIR,
                FUSE_RELEASEDIR,
                FUSE_OPENDIR,
                FUSE_READDIR,
                FUSE_RELEASEDIR
            ]
        );

        // statfs, now that the session is up
        let mut sb = Statfs::new();
        fusefs_statfs(mp, &mut sb, p).expect("statfs");
        assert_eq!((sb.f_blocks, sb.f_bfree, sb.f_bavail), (100, 50, 40));
        assert_eq!((sb.f_files, sb.f_ffree, sb.f_favail), (10, 5, 5));
        assert_eq!((sb.f_bsize, sb.f_namemax), (512, 255));
        assert_eq!(sb.f_iosize, BLKDEV_IOSIZE as u32, "capped");

        // The sysctls.
        assert_eq!(sysctl(p, &[FUSEFS_OPENDEVS]), Ok(1));

        // Last reference: VOP_INACTIVE releases both handles; unmount reclaims (FORGET with
        // the two lookups) and says FUSE_DESTROY.
        let _ = take_seen();
        vrele(vp);
        vput(root);
        fusefs_unmount(mp, 0, p).expect("unmount");
        let seen = take_seen();
        assert_eq!(
            seen.iter()
                .map(|r| r.hdr.opcode)
                .collect::<std::vec::Vec<_>>(),
            [FUSE_RELEASE, FUSE_RELEASE, FUSE_FORGET, FUSE_DESTROY]
        );
        let forget = &seen[2];
        assert_eq!(
            (
                forget.hdr.nodeid,
                forget
                    .op::<crate::fs::fuse::fusebuf_h::FuseForgetIn>()
                    .nlookup
            ),
            (2, 2)
        );
        assert!(
            mp.node_arc::<FusefsMnt<TestGlue>>().is_none(),
            "mnt_data = NULL"
        );
        assert!(
            !crate::fs::fuse::fuse_device::fuse_lookup::<TestGlue>(dev)
                .is_some_and(|fd| fd.has_fmp()),
            "the device serves no mount"
        );

        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn a_dead_daemon_fails_the_vnode_operations() {
        let (_g, p) = setup();
        hello_fs(HELLO);
        let dev = open_dev(p, 0);
        let fd = install_dev_fd(p, dev);
        set_daemon(Some(scripted));

        let mp = mount(p, b"/mnt", &mut args(fd, 4096, 0)).expect("mount");
        let fmp = vfstofusefs(mp);
        assert_eq!(fmp.max_read, 4096);
        let root = VFS_ROOT(mp).expect("root");
        let mut va = Vattr::new();
        vop_getattr(root, &mut va, Cred::real(p.ucred()), p).expect("getattr");
        assert_eq!(va.va_type, VDIR);

        // The daemon exits: its descriptor's close fails what is queued and ends the
        // session.
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
        assert_eq!(fmp.sess_init.get(), 0);
        assert_eq!(
            vop_getattr(root, &mut va, Cred::real(p.ucred()), p),
            Err(Errno::ENXIO)
        );
        let mut vpp = None;
        let mut cnp = cn(p, b"hello", LOCKPARENT | ISLASTCN);
        assert_eq!(vop_lookup(root, &mut vpp, &mut cnp), Err(Errno::ENXIO));

        vput(root);
        fusefs_unmount(mp, 0, p).expect("unmount without a daemon");
    }

    #[test]
    fn the_mount_refuses_a_device_that_is_not_an_open_unit_or_is_mounted() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let fd = install_dev_fd(p, dev);
        set_daemon(Some(scripted));

        // Not a character device: EBADF (the C's).
        let blk = cdevvp::<TestGlue>(makedev(92, 7))
            .expect("cdevvp")
            .expect("a vnode");
        blk.set_vtype(crate::fs::vnode::VBLK);
        let fd_blk = install_vnode_fd(p, blk);
        assert_eq!(
            mount(p, b"/mnt", &mut args(fd_blk, 0, 0)).err(),
            Some(Errno::EBADF)
        );

        // D-N5.9-U1: a fuse minor no unit has open, and the open unit's minor under another
        // major, are no fuse device: ENXIO, where the C mounts and hangs the first access.
        let closed = install_dev_fd(p, makedev(92, 5));
        assert_eq!(
            mount(p, b"/mnt", &mut args(closed, 0, 0)).err(),
            Some(Errno::ENXIO)
        );
        let other = install_dev_fd(p, makedev(3, 0));
        assert_eq!(
            mount(p, b"/mnt", &mut args(other, 0, 0)).err(),
            Some(Errno::ENXIO)
        );
        let before = vfs::<TestGlue>().fuse.fbuf_pool.nout();
        assert_eq!(stats().1, 0, "no FUSE_INIT queued by a refusal");

        // The unit itself mounts; a second mount of it is EBUSY, where the C cross-wires the
        // two sessions.
        let mp = mount(p, b"/mnt", &mut args(fd, 0, 0)).expect("mount");
        let fmp = vfstofusefs(mp);
        let second = install_dev_fd(p, dev);
        assert_eq!(
            mount(p, b"/mnt2", &mut args(second, 0, 0)).err(),
            Some(Errno::EBUSY)
        );
        assert_eq!(
            mount(p, b"/mnt2", &mut args(fd, 0, 0)).err(),
            Some(Errno::EBUSY)
        );
        assert_eq!(
            vfs::<TestGlue>().fuse.fbuf_pool.nout(),
            before + 1,
            "the first mount's FUSE_INIT alone"
        );
        let fd_dev = crate::fs::fuse::fuse_device::fuse_lookup::<TestGlue>(dev).expect("open");
        assert!(
            fd_dev
                .fd_fmp
                .read()
                .as_ref()
                .is_some_and(|m| ptr::eq(&**m, &*fmp)),
            "the unit serves the first mount still"
        );
        drop(fd_dev);

        fusefs_unmount(mp, 0, p).expect("unmount");
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn the_attachment_tests_the_unit_again() {
        // S1 of N5.9f2's review: U1's test made again right before the attachment, for a
        // unit that closed, or that another mount attached, while the mount slept in its
        // M_WAITOK malloc (here: the change made between the two tests by hand).
        let (_g, p) = setup();
        let attach = |fmp: &FusefsMntRef<TestGlue>| {
            fmp.mp.set_node(MArc::erase(fmp.clone()));
            let r = fusefs_mount_attach(fmp.mp, fmp);
            (r, fmp.mp.node_arc::<FusefsMnt<TestGlue>>().is_some())
        };

        // The unit closed meanwhile: ENXIO, the slot given up (the block goes with `gone`).
        let dev = open_dev(p, 0);
        let gone = crate::fs::fuse::testfuse::fake_fmp(dev, 0);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
        assert_eq!(attach(&gone), (Err(Errno::ENXIO), false));

        // Another mount attached the unit meanwhile: EBUSY, the first mount still served.
        let dev = open_dev(p, 1);
        let first = crate::fs::fuse::testfuse::mount_fmp(dev, 0);
        let second = crate::fs::fuse::testfuse::fake_fmp(dev, 0);
        assert_eq!(attach(&second), (Err(Errno::EBUSY), false));
        let fd = crate::fs::fuse::fuse_device::fuse_lookup::<TestGlue>(dev).expect("open");
        assert!(
            fd.fd_fmp
                .read()
                .as_ref()
                .is_some_and(|m| ptr::eq(&**m, &*first))
        );
        drop(fd);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");

        // An open unit with no mount: attached, the slot kept.
        let dev = open_dev(p, 2);
        let ok = crate::fs::fuse::testfuse::fake_fmp(dev, 0);
        assert_eq!(attach(&ok), (Ok(()), true));
        let fd = crate::fs::fuse::fuse_device::fuse_lookup::<TestGlue>(dev).expect("open");
        assert!(fd.has_fmp());
        drop(fd);
        drop(ok.mp.take_node());
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn the_mount_point_keeps_the_cs_mnamelen() {
        // D-N5.9-10: strlcpy(f_mntonname, path, MNAMELEN), 89 characters kept, where LZ's
        // MNAMELEN - 1 kept 88.
        let (_g, p) = setup();
        let mut path = std::vec![b'/'];
        path.resize(MNAMELEN - 1, b'm');
        for (unit, len) in [(0, MNAMELEN - 1), (1, MNAMELEN + 5)] {
            path.resize(len, b'm');
            let dev = open_dev(p, unit);
            let fd = install_dev_fd(p, dev);
            set_daemon(Some(scripted));
            let mp = mount(p, &path, &mut args(fd, 0, 0)).expect("mount");
            let on = mp.stat().f_mntonname;
            assert_eq!(&on[..MNAMELEN - 1], &path[..MNAMELEN - 1], "89 characters");
            assert_eq!(on[MNAMELEN - 1], 0, "and the NUL");
            fusefs_unmount(mp, 0, p).expect("unmount");
            set_daemon(None);
            fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
        }
    }

    #[test]
    fn fuse_init_checks_the_daemons_version() {
        let (_g, p) = setup();
        // A reply to FUSE_INIT of major `major`, minor `minor`, max_write `mw`: what the
        // device answers the daemon and the session it leaves.
        let mut unit = 0;
        let mut init = |major: u32, minor: u32, mw: u32| {
            let dev = open_dev(p, unit);
            unit += 1;
            let fd = install_dev_fd(p, dev);
            let mp = mount(p, b"/mnt", &mut args(fd, 0, 0)).expect("mount");
            let req = read_req(dev).expect("FUSE_INIT");
            assert_eq!(req.hdr.opcode, FUSE_INIT);
            let r = req.op::<FuseInitIn>();
            assert_eq!(
                (r.major, r.minor, r.max_readahead, r.flags),
                (FUSE_KERNEL_VERSION, FUSE_KERNEL_MINOR_VERSION, 0, 0)
            );
            let out = FuseInitOut {
                major,
                minor,
                max_write: mw,
                ..FuseInitOut::default()
            };
            let w = reply(
                dev,
                req.hdr.unique,
                0,
                bytes_of(core::slice::from_ref(&out)),
                &[],
            );
            let fmp = vfstofusefs(mp);
            let s = (w, fmp.sess_init.get(), fmp.max_write.get());
            // A session up has the unmount's FUSE_DESTROY answered.
            set_daemon(Some(scripted));
            fusefs_unmount(mp, 0, p).expect("unmount");
            set_daemon(None);
            fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
            s
        };
        let pend = BLKDEV_IOSIZE;
        assert_eq!(init(6, 19, 0), (Err(Errno::EINVAL), PENDING, pend));
        assert_eq!(init(7, 8, 0), (Err(Errno::EINVAL), PENDING, pend));
        assert_eq!(init(7, 9, 8192), (Ok(()), 1, 8192));
        assert_eq!(
            init(7, 19, 0),
            (Ok(()), 1, FUSEBUFMAXSIZE as i32),
            "0 means the largest"
        );
        assert_eq!(init(7, 19, u32::MAX), (Ok(()), 1, FUSEBUFMAXSIZE as i32));
    }

    #[test]
    fn an_unmount_fails_and_frees_the_requests_in_flight() {
        let (_g, p) = setup();
        let out0 = vfs::<TestGlue>().fuse.fbuf_pool.nout();
        let (dev, mp) = mount_unit(p, 0);
        let fmp = vfstofusefs(mp);
        // FUSE_INIT unanswered on the input queue, and a FUSE_FORGET beside it.
        let f = fb_setup(0, 9, crate::fs::fuse::fusebuf_h::FUSE_FORGET, p);
        assert!(fuse_device_queue_fbuf::<TestGlue>(dev, f));
        assert_eq!(stats().1, 2);
        assert_eq!(vfs::<TestGlue>().fuse.fbuf_pool.nout(), out0 + 2);
        // PENDING: no FUSE_DESTROY is sent (nothing would answer it).
        fusefs_unmount(mp, 0, p).expect("unmount");
        assert_eq!(stats().1, 0, "the input queue emptied");
        assert_eq!(
            vfs::<TestGlue>().fuse.fbuf_pool.nout(),
            out0,
            "both freed with the queue's references"
        );
        assert_eq!(fmp.sess_init.get(), 0);
        let mut b = [0u8; 64];
        assert_eq!(
            crate::fs::fuse::testfuse::dev_read(dev, &mut b, crate::fs::vnode::IO_NDELAY),
            Err(Errno::ENODEV),
            "the device serves no mount"
        );
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }

    #[test]
    fn the_vfs_fuse_sysctls() {
        let (_g, p) = setup();
        assert_eq!(sysctl(p, &[FUSEFS_OPENDEVS]), Ok(0));
        let (dev, mp) = mount_unit(p, 0);
        assert_eq!(sysctl(p, &[FUSEFS_OPENDEVS]), Ok(1));
        assert_eq!(sysctl(p, &[FUSEFS_INFBUFS]), Ok(1), "FUSE_INIT");
        assert_eq!(sysctl(p, &[FUSEFS_WAITFBUFS]), Ok(0));
        let pages = vfs::<TestGlue>().fuse.fbuf_pool.npages();
        assert!(pages > 0, "the pool holds the request's page");
        assert_eq!(sysctl(p, &[FUSEFS_POOL_NBPAGES]), Ok(pages as i32));
        // The daemon reads FUSE_INIT: it waits for the reply.
        let req = read_req(dev).expect("FUSE_INIT");
        assert_eq!(
            (sysctl(p, &[FUSEFS_INFBUFS]), sysctl(p, &[FUSEFS_WAITFBUFS])),
            (Ok(0), Ok(1))
        );
        let out = FuseInitOut {
            major: 7,
            minor: 19,
            ..FuseInitOut::default()
        };
        reply(
            dev,
            req.hdr.unique,
            0,
            bytes_of(core::slice::from_ref(&out)),
            &[],
        )
        .expect("reply");
        assert_eq!(sysctl(p, &[FUSEFS_WAITFBUFS]), Ok(0));
        assert_eq!(
            sysctl(p, &[FUSEFS_POOL_NBPAGES]),
            Ok(vfs::<TestGlue>().fuse.fbuf_pool.npages() as i32)
        );

        // Read-only, one level, the C's four names.
        let mut len = size_of::<i32>();
        for mib in [FUSEFS_OPENDEVS, FUSEFS_POOL_NBPAGES] {
            assert_eq!(
                fusefs_sysctl(&[mib], 0, &mut len, UBUF, 4, p),
                Err(Errno::EPERM)
            );
        }
        assert_eq!(sysctl(p, &[5]), Err(Errno::EOPNOTSUPP));
        assert_eq!(sysctl(p, &[FUSEFS_OPENDEVS, 1]), Err(Errno::ENOTDIR));

        fusefs_unmount(mp, 0, p).expect("unmount");
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
        assert_eq!(sysctl(p, &[FUSEFS_OPENDEVS]), Ok(0));
    }

    #[test]
    fn another_user_gets_no_statfs_and_dummy_attributes() {
        let (_g, p, dev, mp, root) = crate::fs::fuse::testfuse::mounted();
        hello_fs(HELLO);
        let mut cr = Ucred::new();
        cr.cr_uid.set(1000);
        cr.cr_gid.set(1000);
        drop(p.set_cred(CredRef::new(cr)));

        let mut sb = Statfs::new();
        assert_eq!(fusefs_statfs(mp, &mut sb, p), Err(Errno::EPERM));
        let mut va = Vattr::new();
        vop_getattr(root, &mut va, Cred::real(p.ucred()), p).expect("dummies");
        assert_eq!(
            (va.va_type, va.va_mode, va.va_nlink, va.va_uid, va.va_fileid),
            (crate::fs::vnode::VNON, 0o700, 2, 0, FUSE_ROOT_ID)
        );
        assert_eq!((va.va_size, va.va_blocksize, va.va_bytes), (512, 512, 512));
        assert_eq!(va.va_rdev, dev);
        assert_eq!(
            crate::fs::vops::vop_access(root, 0, Cred::real(p.ucred()), p),
            Err(Errno::EACCES)
        );
        assert!(take_seen().is_empty(), "nothing asked of the daemon");

        drop(p.set_cred(CredRef::new(Ucred::new())));
        vput(root);
        fusefs_unmount(mp, 0, p).expect("unmount");
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }
}
/* </TESTS> */
