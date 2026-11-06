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
//! The FUSE vnode operations (`fusefs_vops`, [`FuseVnops`]): each one becomes a request to
//! the daemon (`fb_setup`, `fb_queue`); and the kqueue filters.
//!
//! Upstream: sys/miscfs/fuse/fuse_vnops.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fuse_vnops.rs@3c62ede68802
//!
//! FUSE file systems can maintain a file handle for each VFS file descriptor that is opened.
//! The OpenBSD VFS does not make file descriptors visible to us so we fake it by mapping
//! open flags to file handles. There is no way for FUSE to know which file descriptor is
//! being used by an application for a file operation. We only maintain 3 descriptors, one
//! each for `O_RDONLY`, `O_WRONLY` and `O_RDWR`. When reading and writing, the first open
//! descriptor is used and this may well not be the one that was set by FUSE open and may
//! have even been opened by another process.
//!
//! ## Deviations
//! - `vop_lookup` is `fuse_lookup`'s `fusefs_lookup`; the helpers the C installs
//!   (`vop_generic_abortop`, `vop_generic_bmap`, `spec_pathconf`) are the kernel's;
//!   `vop_revoke` and `vop_bwrite` are NULL (the trait's defaults), as in C.
//! - A `struct proc *` the C dereferences although it may be NULL (`a_p` of close, inactive
//!   and reclaim, `uio_procp`) is the given thread or else `curproc` ([`proc_or_cur`]), which
//!   panics without a thread where the C would follow NULL (LZ's).
//! - Credentials the C dereferences (`ap->a_cred` in `fusefs_access` and `fusefs_setattr`)
//!   go through `ucred`, which panics on `NOCRED`/`FSCRED` (LZ's).
//! - `fusefs_readdir` and `fusefs_mkdir` free the fusebuf on the two paths where the C
//!   forgets it (a dirent that does not fit the caller's buffer, `goto out`; a `mkdir` reply
//!   with inode 0 or `FUSE_ROOT_ID`) (LZ's).
//! - `fusefs_readdir` answers `EIO` to a reply that carries data but from which no entry
//!   could be taken (its first dirent partial, or fewer bytes than a dirent's head) while the
//!   caller's buffer has room: the C's outer loop asks the daemon again at the same offset,
//!   for ever (fuse_vnops.c:829, :861); a full buffer still ends the call as the C's `goto
//!   out`. A correct daemon (libfuse adds whole dirents only) never sends such a reply (the
//!   user's D-N5.9-U2).
//! - `fusefs_write` answers `EIO` to a `FUSE_WRITE` reply of `size` 0, once the uio is given
//!   the chunk back as the C's `diff` path does: the C's loop goes round again with the uio's
//!   residual and offset rewound but its iovec already past the chunk (the copy at
//!   fuse_vnops.c:1326, the give-back at :1336-1343), so it sends the bytes after the chunk
//!   and then runs past the iovecs (D-N5.9-U2).
//! - `fusefs_write` ends its loop after a `FUSE_WRITE` reply of fewer bytes than asked (but
//!   some), once the uio is given the rest of the chunk back (the C's two lines), so write(2)
//!   returns the bytes the daemon accepted: a POSIX short write, the file and the offset
//!   right. The C goes round again with the iovec past the chunk (fuse_vnops.c:1326,
//!   :1336-1343): its next request carries the wrong bytes at `offset + size`, and once the
//!   iovecs are spent `uiomove` runs past them (`KASSERT(uio->uio_iovcnt > 0)` under
//!   `DIAGNOSTIC`, or a stale copy). A correct daemon reaches it: libfuse replies with its
//!   write handler's count (lib/libfuse/fuse_ops.c:645-651) (the user's D-N5.9-U4).
//! - `fusefs_print`'s `#if defined(DEBUG) || defined(DIAGNOSTIC) || defined(VFSLCKDEBUG)` is
//!   the `debug`/`diagnostic` features (`VFSLCKDEBUG` has no feature) (LZ's).
//! - `fusefs_getattr`'s dummy attributes for another user read `f_ctime` under the mount's
//!   statistics lock (the one member with no unlocked word), where the C reads `mnt_stat`
//!   bare: one lock section on that refusal only (LZ's).
//!
//! Kept as the C, named (D-N5.9-9): the vnode lock is held across every request's sleep, so
//! a daemon that touches its own mount through a locked vnode deadlocks; `fusefs_symlink`,
//! `fusefs_mknod`, `fusefs_create` and `fusefs_mkdir` answer `EIO` to a reply whose `nodeid`
//! is 0 or `FUSE_ROOT_ID` (fuse_vnops.c:753, :1128, :1195).
//!
//! ## Redesign
//! - LZ's `FUSEFS_VOPS` (a `bsd` `Vops` of function pointers over argument structures) ->
//!   [`FuseVnops`], one `impl Vnops<G>` of the C's 34 slots (decision 46), the member
//!   `vops` of the glue's `FuseTables` (one address per glue, for `vops_id`; a byte, so that
//!   it is not zero-sized), tmpfs's shape. The operations are the C's functions over the
//!   kernel's arguments, generic over the glue; `curproc` is the operation's thread (`a_p`,
//!   `cn_proc`, `uio_procp`).
//! - The node and its mount are counted handles (`vtoi`, `i_fmp`), so nothing outlives the
//!   blocks it reads; LZ's `&'static` over raw `malloc` blocks are gone, and so is their
//!   `free` in `fusefs_reclaim`: the vnode's slot gives its handle up (`take_node`) after
//!   `fuse_ihashrem`, the last handle frees the node (the C's `free(ip, M_FUSEFS)`).
//! - A request's names go into its data block through `Fusebuf::dat_update` before it is
//!   queued; a reply's data is moved out of the request (`take_dat`) for the copy that can
//!   sleep (`fusefs_read`, `fusefs_readdir`, `fusefs_readlink`) and a write's is filled in
//!   the moved-out block and put back before the request is queued: LZ's five raw slices of
//!   the block (`fb_dat_slice`, 5 `unsafe`) are gone. The copies are `bsd`'s `uiomove`
//!   through the `FileGlue::uiomove` bridge (N7), the device's.
//! - The three `filterops` are `impl Filter<G>` for [`FusefsRead`], [`FusefsWrite`] and
//!   [`FusefsVnode`] with their tables `FusefsFiltops::<G>::READ`/`WRITE`/`VNODE`; a knote's
//!   vnode is its typed hook (`set_hook_vnode`, ufs's `kn_vnode`) where LZ cast `kn_hook` back
//!   (one `unsafe`); `filt_fusefsread` reads the node after its `NOTE_REVOKE` test (a
//!   reclaimed vnode has none).
//! - `fusefs_readdir` builds each `struct dirent` as the C's stack `de` and copies its
//!   `d_reclen` bytes with `fs::subr::dirent_uiomove` (msdosfs's and cd9660's): a second
//!   stack image of the entry the C does not make (ROADMAP N12's readdir row); LZ's
//!   `dirent_bytes` is that function.
//! - The handle kinds are `FufhKind`s (the C's `for (type = 0; type < FUFH_MAXTYPE; ..)` is
//!   `FufhKind::ALL`), so no slot index can be invalid.
//! - The helpers LZ wrote for this file are the kernel's existing ones: `vmount`
//!   (`ufs_lookup`), `ucred` and `pnbuf_free` (`ufs_vnops`), `kn_vnode` (`ufs_vnops`, the
//!   typed hook), `opt_eq` (`lookup`, the C's `==` on two pointers that may be NULL).

use core::marker::PhantomData;
use core::ptr;

use hal::sys::errno::Errno;
use hal::sys::fcntl::oflags;
use hal::sys::fcntl::{FREAD, FWRITE, Flock, O_CREAT, O_EXCL, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};
use hal::sys::mount::MNT_RDONLY;
use hal::sys::stat::{ALLPERMS, S_BLKSIZE, S_IFMT, S_IRUSR, S_IRWXU, S_ISTXT, S_IXUSR};
use hal::sys::types::{Daddr, Dev, Gid, Mode, Nlink, Register, Uid};
use hal::user::pod_from_bytes;

use crate::assert::kpanic;
use crate::event::{
    __EV_POLL, __EV_SELECT, EV_EOF, EV_ONESHOT, EVFILT_READ, EVFILT_VNODE, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_ATTRIB, NOTE_DELETE, NOTE_EOF, NOTE_LINK,
    NOTE_RENAME, NOTE_REVOKE, NOTE_WRITE,
};
use crate::file::{LockOwner, foffset};
use crate::fs::default;
use crate::fs::fuse::fuse_device::{fuse_device_queue_fbuf, uiomove};
use crate::fs::fuse::fuse_file::{fusefs_fd_get, fusefs_file_close, fusefs_file_open};
use crate::fs::fuse::fuse_ihash::fuse_ihashrem;
use crate::fs::fuse::fuse_lookup::fusefs_lookup;
use crate::fs::fuse::fusebuf::{FusebufRef, fb_delete, fb_queue, fb_setup};
use crate::fs::fuse::fusebuf_h::{
    FUSE_FATTR_ATIME, FUSE_FATTR_GID, FUSE_FATTR_MODE, FUSE_FATTR_MTIME, FUSE_FATTR_SIZE,
    FUSE_FATTR_UID, FUSE_FLUSH, FUSE_FORGET, FUSE_FSYNC, FUSE_GETATTR, FUSE_LINK, FUSE_MKDIR,
    FUSE_MKNOD, FUSE_NAME_OFFSET, FUSE_READ, FUSE_READDIR, FUSE_READLINK, FUSE_RENAME, FUSE_RMDIR,
    FUSE_ROOT_ID, FUSE_SETATTR, FUSE_SYMLINK, FUSE_UNLINK, FUSE_WRITE, FuseAttrOut, FuseDirent,
    FuseEntryOut, FuseFlushIn, FuseForgetIn, FuseFsyncIn, FuseLinkIn, FuseMkdirIn, FuseMknodIn,
    FuseReadIn, FuseRenameIn, FuseSetattrIn, FuseWriteIn, FuseWriteOut, fuse_dirent_size,
};
use crate::fs::fuse::fusefs::{
    UNDEF_FLUSH, UNDEF_FSYNC, UNDEF_LINK, UNDEF_MKDIR, UNDEF_MKNOD, UNDEF_READLINK, UNDEF_REMOVE,
    UNDEF_RENAME, UNDEF_RMDIR, UNDEF_SETATTR, UNDEF_SYMLINK,
};
use crate::fs::fuse::fusefs_node::{FUFH_INVALID, FUFH_RDWR, FUFH_WRONLY, FufhKind, vtoi};
use crate::fs::lockf::lf_advlock;
use crate::fs::lookup::opt_eq;
use crate::fs::namei::ISDOTDOT;
use crate::fs::spec::{BLKDEV_IOSIZE, spec_pathconf};
use crate::fs::subr::{dirent_uiomove, vaccess, vgone, vput, vrele};
use crate::fs::ufs::ufs::ufs_lookup::vmount;
use crate::fs::ufs::ufs::ufs_vnops::{kn_vnode, pnbuf_free, ucred};
use crate::fs::vnode::{
    IO_APPEND, LK_EXCLUSIVE, LK_RETRY, LK_RWFLAGS, VDIR, VLNK, VN_KNOTE, VNON, VNOVAL, VREG,
    VWRITE, Vattr, Vnode, iftovt, makeimode, vn_lock,
};
use crate::fs::vops::{Cn, Cred, IoBuf, VFS_VGET, Vnops, vop_abortop, vop_getattr, vop_unlock};
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};
use hal::sys::dirent::{Dirent, MAXNAMLEN, dirent_recsize};

/// `fusefs_vops`: the operations of a FUSE vnode (the glue's `FuseTables::vops`, one
/// address).
pub struct FuseVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> FuseVnops<G> {
    /// The table (the glue's `FuseTables::vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// `fusefsread_filtops`'s filter: the bytes past the file offset.
pub struct FusefsRead;

/// `fusefswrite_filtops`'s filter: a file is always writable.
pub struct FusefsWrite;

/// `fusefsvnode_filtops`'s filter: the vnode events (`NOTE_*`) the user asked for.
pub struct FusefsVnode;

/// The tables of the three filters (`kn_fop`'s targets), made at compile time.
struct FusefsFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> FusefsFiltops<G> {
    /// `fusefsread_filtops`.
    const READ: Filterops<G> = Filterops::of::<FusefsRead>();
    /// `fusefswrite_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<FusefsWrite>();
    /// `fusefsvnode_filtops`.
    const VNODE: Filterops<G> = Filterops::of::<FusefsVnode>();
}

/// A `struct proc *` the C dereferences although it may be NULL: `f` of the given thread, or
/// else of `curproc` (the module's deviations).
fn proc_or_cur<G: ProcGlue, R>(p: Option<&Thread<G>>, f: impl FnOnce(&Thread<G>) -> R) -> R {
    match p {
        Some(p) => f(p),
        None => G::with_curproc(|c| match c {
            Some(c) => f(c),
            None => kpanic(format_args!("fusefs: no curproc")),
        }),
    }
}

/// Copies `name` and a NUL into a fresh fusebuf's data at `off` (the C's `memcpy(fbuf->fb_dat
/// + off, name, len); fbuf->fb_dat[off + len] = '\0'`).
fn fb_put_name(fbuf: &FusebufRef, off: usize, name: &[u8]) {
    fbuf.dat_update(|d| {
        d[off..off + name.len()].copy_from_slice(name);
        d[off + name.len()] = 0;
    });
}

/// `fusefs_kqfilter` (`vop_kqfilter`): attach a knote to the vnode.
pub fn fusefs_kqfilter<G: ProcGlue>(vp: &'static Vnode<G>, kn: &Knote<G>) -> Result<(), Errno> {
    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&FusefsFiltops::<G>::READ)),
        EVFILT_WRITE => kn.kn_fop.set(Some(&FusefsFiltops::<G>::WRITE)),
        EVFILT_VNODE => kn.kn_fop.set(Some(&FusefsFiltops::<G>::VNODE)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = (caddr_t)vp: a fresh knote holds no hook yet (as `ufs_kqfilter`'s).
    let _ = kn.set_hook_vnode(vp);

    vp.klist.insert_legacy(kn);

    Ok(())
}

/// `filt_fusefsdetach`: unhooks the knote from the vnode.
fn filt_fusefsdetach<G: ProcGlue>(kn: &Knote<G>) {
    let vp = kn_vnode(kn);

    vp.klist.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for FusefsRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_fusefsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_fusefsdetach(kn);
    }

    /// `filt_fusefsread`: the bytes past the file offset; always ready for poll and select.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        let vp = kn_vnode(kn);

        // filesystem is gone, so set the EOF flag and schedule the knote for deletion. The
        // node is read past this test only (a reclaimed vnode has none).
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }
        let ip = vtoi(vp);

        kn.kn_data()
            .set(ip.filesize.get().wrapping_sub(foffset(&kn.fp())));
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

impl<G: ProcGlue> Filter<G> for FusefsWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_fusefsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_fusefsdetach(kn);
    }

    /// `filt_fusefswrite`: a file is always writable.
    fn event(kn: &Knote<G>, hint: i64) -> bool {
        // filesystem is gone, so set the EOF flag and schedule the knote for deletion
        if hint == i64::from(NOTE_REVOKE) {
            kn.set_flags(EV_EOF | EV_ONESHOT);
            return true;
        }

        kn.kn_data().set(0);
        true
    }
}

impl<G: ProcGlue> Filter<G> for FusefsVnode {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_fusefsdetach`.
    fn detach(kn: &Knote<G>) {
        filt_fusefsdetach(kn);
    }

    /// `filt_fusefsvnode`: records the vnode events (`NOTE_*`) the user asked for.
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

/// `fusefs_open` (`vop_open`): makes sure the node has a daemon handle of the open's kind.
pub fn fusefs_open<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;
    let mut fufh_type = FufhKind::RdOnly;

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }

    let isdir = if vp.vtype() == VDIR {
        true
    } else {
        if mode & FREAD != 0 && mode & FWRITE != 0 {
            fufh_type = FufhKind::RdWr;
        } else if mode & FWRITE != 0 {
            fufh_type = FufhKind::WrOnly;
        }

        // Due to possible attribute caching, there is no reliable way to determine if the
        // file was modified externally (e.g. network file system) so clear the UVM cache to
        // ensure that it is not stale. The file can still become stale later on read but
        // this will satisfy most situations.
        uvm_vnp_uncache(vp);
        false
    };

    // already open i think all is ok
    if ip.fufh(fufh_type).fh_type != FUFH_INVALID {
        return Ok(());
    }

    // The file has already been created and/or truncated so FUSE dictates that no creation
    // and truncation flags are passed to open.
    let flags = oflags(mode) & !(O_CREAT | O_EXCL | O_TRUNC);
    fusefs_file_open(fmp, &ip, fufh_type, flags, isdir, p)
}

/// `fusefs_close` (`vop_close`): asks the daemon to flush a writable file (`FUSE_FLUSH`);
/// handles are released on `VOP_INACTIVE`.
pub fn fusefs_close<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fflag: i32,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    if fmp.sess_init.get() == 0 {
        return Ok(());
    }

    // The file or directory may have been opened more than once so there is no reliable
    // way to determine when to ask the FUSE daemon to release its file descriptor. For
    // files, ask the daemon to flush any buffers to disk now. All open file descriptors
    // will be released on VOP_INACTIVE(9).

    if vp.vtype() == VDIR {
        return Ok(());
    }

    // Implementing flush is optional so don't error.
    if fmp.undef_op.get() & UNDEF_FLUSH != 0 {
        return Ok(());
    }

    // Only flush writeable file descriptors.
    let fufh_type = if fflag & FREAD != 0 && fflag & FWRITE != 0 {
        FufhKind::RdWr
    } else if fflag & FWRITE != 0 {
        FufhKind::WrOnly
    } else {
        return Ok(());
    };

    if ip.fufh(fufh_type).fh_type == FUFH_INVALID {
        return Err(Errno::EBADF);
    }

    let fbuf = proc_or_cur(p, |p| fb_setup(0, ip.i_number, FUSE_FLUSH, p));
    fbuf.op_set(&FuseFlushIn {
        fh: ip.fufh(fufh_type).fh_id,
        ..FuseFlushIn::default()
    });
    let error = fb_queue::<G>(fmp.dev, &fbuf);
    fb_delete(fbuf);
    if error == Err(Errno::ENOSYS) {
        fmp.undef_op.set(fmp.undef_op.get() | UNDEF_FLUSH);

        // Implementing flush is optional so don't error.
        return Ok(());
    }

    error
}

/// `fusefs_access` (`vop_access`): only the user who mounted the file system (or anyone
/// with `allow_other`), then the permissions `FUSE_GETATTR` reports.
pub fn fusefs_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    mode: i32,
    acred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let cred = p.ucred();
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    // Only user that mounted the file system can access it unless allow_other mount option
    // was specified.
    if fmp.allow_other == 0 && cred.cr_uid.get() != fmp.mp.owner() {
        return Err(Errno::EACCES);
    }

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }

    // Disallow write attempts on filesystems mounted read-only; unless the file is a
    // socket, fifo, or a block or character device resident on the filesystem.
    if mode & VWRITE != 0 && fmp.mp.flag() & MNT_RDONLY != 0 {
        let t = vp.vtype();
        if t == VREG || t == VDIR || t == VLNK {
            return Err(Errno::EROFS);
        }
    }

    let mut vattr = Vattr::new();
    vop_getattr(vp, &mut vattr, Cred::real(cred), p)?;

    vaccess(
        vp.vtype(),
        vattr.va_mode & ALLPERMS,
        vattr.va_uid,
        vattr.va_gid,
        mode,
        ucred(acred),
    )
}

/// `fusefs_getattr` (`vop_getattr`): the daemon's `FUSE_GETATTR` answer; dummy values for
/// users other than the one who mounted, unless `allow_other`.
pub fn fusefs_getattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &mut Vattr,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let cred = p.ucred();
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    // Only user that mounted the file system can access it unless allow_other mount option
    // was specified. Return dummy values for the root inode in this situation.
    let owner = fmp.mp.owner();
    if fmp.allow_other == 0 && cred.cr_uid.get() != owner {
        *vap = Vattr::new();
        vap.va_type = VNON;
        if vmount(vp).flag() & MNT_RDONLY != 0 {
            vap.va_mode = S_IRUSR | S_IXUSR;
        } else {
            vap.va_mode = S_IRWXU;
        }
        vap.va_nlink = 2;
        vap.va_uid = owner;
        vap.va_gid = owner;
        vap.va_fsid = i64::from(fmp.mp.fsid().val[0]);
        vap.va_fileid = ip.i_number;
        vap.va_size = S_BLKSIZE as u64;
        vap.va_blocksize = S_BLKSIZE as i64;
        let ctime = fmp.mp.with_stat(|sp| sp.f_ctime);
        vap.va_atime.tv_sec = ctime as i64;
        vap.va_mtime.tv_sec = ctime as i64;
        vap.va_ctime.tv_sec = ctime as i64;
        vap.va_rdev = fmp.dev;
        vap.va_bytes = S_BLKSIZE as u64;
        return Ok(());
    }

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }

    let fbuf = fb_setup(0, ip.i_number, FUSE_GETATTR, p);

    if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
        fb_delete(fbuf);
        return Err(error);
    }

    let mut st = fbuf.op_get::<FuseAttrOut>().attr;

    // opendir(3) expects blocksize to be greater than zero.
    if st.blksize == 0 {
        st.blksize = S_BLKSIZE as u32;
    } else if st.blksize > BLKDEV_IOSIZE as u32 {
        st.blksize = BLKDEV_IOSIZE as u32;
    }

    // calculate blocks of held disk space if fs didn't do it
    if st.blocks == 0 && st.size > 0 {
        st.blocks = st.size.div_ceil(S_BLKSIZE as u64);
    }

    *vap = Vattr::new();
    vap.va_type = iftovt(st.mode);
    vap.va_mode = st.mode & !S_IFMT;
    vap.va_nlink = st.nlink as Nlink;
    vap.va_uid = st.uid as Uid;
    vap.va_gid = st.gid as Gid;
    vap.va_fsid = i64::from(fmp.mp.fsid().val[0]);
    vap.va_fileid = st.ino;
    vap.va_size = st.size;
    vap.va_blocksize = i64::from(st.blksize);
    vap.va_atime.tv_sec = st.atime as i64;
    vap.va_atime.tv_nsec = i64::from(st.atimensec);
    vap.va_mtime.tv_sec = st.mtime as i64;
    vap.va_mtime.tv_nsec = i64::from(st.mtimensec);
    vap.va_ctime.tv_sec = st.ctime as i64;
    vap.va_ctime.tv_nsec = i64::from(st.ctimensec);
    vap.va_rdev = st.rdev as Dev;
    vap.va_bytes = st.blocks.wrapping_mul(S_BLKSIZE as u64);

    fb_delete(fbuf);
    Ok(())
}

/// `fusefs_setattr` (`vop_setattr`): `FUSE_SETATTR` with the attributes that are set.
pub fn fusefs_setattr<G: ProcGlue>(
    vp: &'static Vnode<G>,
    vap: &Vattr,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    // Setting of flags is not supported.
    if vap.va_flags != VNOVAL as u64 {
        return Err(Errno::EOPNOTSUPP);
    }

    // Check for unsettable attributes.
    if vap.va_type != VNON
        || vap.va_nlink != VNOVAL as Nlink
        || vap.va_fsid != i64::from(VNOVAL)
        || vap.va_fileid != VNOVAL as u64
        || vap.va_blocksize != i64::from(VNOVAL)
        || vap.va_rdev != VNOVAL
        || vap.va_bytes as i32 != VNOVAL
        || vap.va_gen != VNOVAL as u64
    {
        return Err(Errno::EINVAL);
    }

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }

    if fmp.undef_op.get() & UNDEF_SETATTR != 0 {
        return Err(Errno::ENOSYS);
    }

    let fbuf = fb_setup(0, ip.i_number, FUSE_SETATTR, p);
    let mut setattr = FuseSetattrIn::default();
    let rdonly = vmount(vp).flag() & MNT_RDONLY != 0;

    let error = 'out: {
        if vap.va_uid != VNOVAL as Uid {
            if rdonly {
                break 'out Err(Errno::EROFS);
            }
            setattr.uid |= vap.va_uid;
            setattr.valid |= FUSE_FATTR_UID;
        }

        if vap.va_gid != VNOVAL as Gid {
            if rdonly {
                break 'out Err(Errno::EROFS);
            }
            setattr.gid |= vap.va_gid;
            setattr.valid |= FUSE_FATTR_GID;
        }

        if vap.va_size != VNOVAL as u64 {
            // Disallow write attempts on read-only file systems; unless the file is a
            // socket, fifo, or a block or character device resident on the file system.
            match vp.vtype() {
                VDIR => break 'out Err(Errno::EISDIR),
                VLNK | VREG if rdonly => break 'out Err(Errno::EROFS),
                _ => {}
            }

            setattr.size |= vap.va_size;
            setattr.valid |= FUSE_FATTR_SIZE;
        }

        if vap.va_atime.tv_nsec != i64::from(VNOVAL) {
            if rdonly {
                break 'out Err(Errno::EROFS);
            }
            setattr.atime = vap.va_atime.tv_sec as u64;
            setattr.atimensec = vap.va_atime.tv_nsec as u32;
            setattr.valid |= FUSE_FATTR_ATIME;
        }

        if vap.va_mtime.tv_nsec != i64::from(VNOVAL) {
            if rdonly {
                break 'out Err(Errno::EROFS);
            }
            setattr.mtime = vap.va_mtime.tv_sec as u64;
            setattr.mtimensec = vap.va_mtime.tv_nsec as u32;
            setattr.valid |= FUSE_FATTR_MTIME;
        }
        // XXX should set a flag if (vap->va_vaflags & VA_UTIMES_CHANGE)

        if vap.va_mode != VNOVAL as Mode {
            if rdonly {
                break 'out Err(Errno::EROFS);
            }

            // chmod returns EFTYPE if the effective user ID is not the super-user, the mode
            // includes the sticky bit (S_ISVTX), and path does not refer to a directory
            if ucred(cred).cr_uid.get() != 0 && vp.vtype() != VDIR && vap.va_mode & S_ISTXT != 0 {
                break 'out Err(Errno::EFTYPE);
            }

            setattr.mode = vap.va_mode & ALLPERMS;
            setattr.valid |= FUSE_FATTR_MODE;
        }

        if setattr.valid == 0 {
            break 'out Ok(());
        }

        fbuf.op_set(&setattr);
        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_SETATTR);
            }
            break 'out Err(error);
        }

        // truncate was successful, let uvm know
        if vap.va_size != VNOVAL as u64 && vap.va_size as i64 != ip.filesize.get() {
            ip.filesize.set(vap.va_size as i64);
            uvm_vnp_setsize(vp, vap.va_size as i64);
        }

        VN_KNOTE(vp, NOTE_ATTRIB);
        Ok(())
    };

    fb_delete(fbuf);
    error
}

/// `fusefs_ioctl` (`vop_ioctl`).
pub fn fusefs_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `fusefs_link` (`vop_link`): `FUSE_LINK`.
pub fn fusefs_link<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let p = cnp.proc();

    let ip = vtoi(vp);
    let dip = vtoi(dvp);
    let fmp = &ip.i_fmp;

    let error = 'out2: {
        if fmp.sess_init.get() == 0 {
            let _ = vop_abortop(dvp, cnp);
            break 'out2 Err(Errno::ENXIO);
        }
        if fmp.undef_op.get() & UNDEF_LINK != 0 {
            let _ = vop_abortop(dvp, cnp);
            break 'out2 Err(Errno::ENOSYS);
        }
        if !ptr::eq(dvp, vp)
            && let Err(e) = vn_lock(vp, LK_EXCLUSIVE)
        {
            let _ = vop_abortop(dvp, cnp);
            break 'out2 Err(e);
        }

        let name = cnp.name();
        let fbuf = fb_setup(name.len() + 1, dip.i_number, FUSE_LINK, p);

        fbuf.op_set(&FuseLinkIn {
            oldnodeid: ip.i_number,
        });
        fb_put_name(&fbuf, 0, name);

        let error = fb_queue::<G>(fmp.dev, &fbuf);

        if let Err(e) = error {
            if e == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_LINK);
            }

            fb_delete(fbuf);
        } else {
            fb_delete(fbuf);
            VN_KNOTE(vp, NOTE_LINK);
            VN_KNOTE(dvp, NOTE_WRITE);
        }

        // out1:
        pnbuf_free(cnp);
        if !ptr::eq(dvp, vp) {
            let _ = vop_unlock(vp);
        }
        error
    };

    // out2:
    vput(dvp);
    error
}

/// `fusefs_symlink` (`vop_symlink`): `FUSE_SYMLINK` with the name and the target.
pub fn fusefs_symlink<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    target: &[u8],
) -> Result<(), Errno> {
    let p = cnp.proc();

    let dp = vtoi(dvp);
    let fmp = &dp.i_fmp;

    let error = 'bad: {
        if fmp.sess_init.get() == 0 {
            break 'bad Err(Errno::ENXIO);
        }

        if fmp.undef_op.get() & UNDEF_SYMLINK != 0 {
            break 'bad Err(Errno::ENOSYS);
        }

        // strlen(target) + 1: the target and its NUL.
        let target = &target[..target.iter().position(|&c| c == 0).unwrap_or(target.len())];
        let len = target.len() + 1;

        let name = cnp.name();
        let fbuf = fb_setup(len + name.len() + 1, dp.i_number, FUSE_SYMLINK, p);

        fb_put_name(&fbuf, 0, name);
        fb_put_name(&fbuf, name.len() + 1, target);

        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_SYMLINK);
            }

            fb_delete(fbuf);
            break 'bad Err(error);
        }

        // symlink returns a fuse_entry_out with the ino of the new link
        let nodeid = fbuf.op_get::<FuseEntryOut>().nodeid;
        if nodeid == 0 || nodeid == FUSE_ROOT_ID {
            fb_delete(fbuf);
            break 'bad Err(Errno::EIO);
        }

        let tdp = match VFS_VGET(fmp.mp, nodeid) {
            Ok(tdp) => tdp,
            Err(e) => {
                fb_delete(fbuf);
                break 'bad Err(e);
            }
        };

        tdp.set_vtype(VLNK);
        VN_KNOTE(dvp, NOTE_WRITE);

        *vpp = Some(tdp);
        fb_delete(fbuf);
        vput(tdp);
        Ok(())
    };

    // bad:
    pnbuf_free(cnp);
    vput(dvp);
    error
}

/// `fusefs_readdir` (`vop_readdir`): `FUSE_READDIR` replies of at most `max_read` bytes,
/// each `struct fuse_dirent` checked and converted into a `struct dirent`.
pub fn fusefs_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    eofflagp: &mut i32,
) -> Result<(), Errno> {
    let mut foffset = uio.uio_offset;
    let mut eofflag = 0;
    let mut diropen = false;

    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }

    // Some file systems expect that the kernel always uses the same read buffer size. In
    // the event that uio_resid is a multiple of max_read, we need to ensure that we use a
    // consistent buffer size in the loop.
    let read_size = uio.uio_resid.min(fmp.max_read as usize);

    // Basic check to ensure buffer is large enough for at least one dirent with maximum
    // allowed name length.
    if read_size < size_of::<Dirent>() {
        return Err(Errno::EINVAL);
    }

    proc_or_cur(uio.uio_procp, |p| {
        if ip.fufh(FufhKind::RdOnly).fh_type == FUFH_INVALID {
            fusefs_file_open(fmp, &ip, FufhKind::RdOnly, O_RDONLY, true, p)?;
            diropen = true;
        }

        let mut error: Result<(), Errno> = Ok(());

        // loop until we run out of buffer space
        'out: while error.is_ok() && uio.uio_resid >= read_size {
            let fbuf = fb_setup(0, ip.i_number, FUSE_READDIR, p);
            fbuf.op_set(&FuseReadIn {
                fh: ip.fufh(FufhKind::RdOnly).fh_id,
                offset: foffset as u64,
                size: read_size as u32,
                ..FuseReadIn::default()
            });

            if let Err(e) = fb_queue::<G>(fmp.dev, &fbuf) {
                error = Err(e);
                fb_delete(fbuf);
                break;
            }

            // Ack end of readdir. Only used by getcwd(3), getdents(3) relies on a repeat call
            // that returns no entries.
            if fbuf.fb_len() == 0 {
                eofflag = 1;
                fb_delete(fbuf);
                break;
            }

            // validate and convert the returned dirents
            let dat = fbuf.take_dat();
            let dat: &[u8] = dat.as_deref().unwrap_or_default();
            let mut fresid = fbuf.fb_len() as u32;
            let mut pos = 0usize;
            let mut taken = 0usize;

            while uio.uio_resid > 0 && fresid as usize > FUSE_NAME_OFFSET {
                let Some(fdp) = dat.get(pos..).and_then(pod_from_bytes::<FuseDirent>) else {
                    break;
                };

                // get the size of the FUSE dirent
                let freclen = fuse_dirent_size(&fdp);

                // check for partial dirent
                if (fresid as usize) < freclen {
                    break;
                }

                // check for sane name length
                if fdp.namelen == 0 || fdp.namelen as usize > MAXNAMLEN {
                    error = Err(Errno::EIO);
                    break;
                }

                let at = pos + FUSE_NAME_OFFSET;
                let Some(name) = dat.get(at..at + fdp.namelen as usize) else {
                    break;
                };

                // check for illegal character in file name
                if name.contains(&b'/') {
                    error = Err(Errno::EIO);
                    break;
                }

                // copy FUSE dirent into struct dirent
                let d_namlen = fdp.namelen as u8;
                let mut de = Dirent {
                    // d_off is used by telldir, seekdir, readdir libc functions and expect the
                    // file system's offset. This will be passed back to this function so needs
                    // to be the FUSE offset.
                    d_fileno: fdp.ino,
                    d_off: fdp.off as i64,
                    d_reclen: dirent_recsize(usize::from(d_namlen)) as u16,
                    d_type: fdp.r#type as u8,
                    d_namlen,
                    __d_padding: [0; 4],
                    // pad with NUL
                    d_name: [0; MAXNAMLEN + 1],
                };
                if uio.uio_resid < usize::from(de.d_reclen) {
                    fb_delete(fbuf);
                    break 'out;
                }
                de.d_name[..name.len()].copy_from_slice(name);

                if let Err(e) = dirent_uiomove(&de, uio) {
                    error = Err(e);
                    break;
                }

                // advance to next FUSE dirent
                fresid -= freclen as u32;
                foffset = fdp.off as i64;
                pos += freclen;
                taken += 1;
            }

            // A reply with data from which no entry could be taken while the buffer has room
            // would be asked for again at the same offset, for ever (the module's deviations,
            // D-N5.9-U2).
            if error.is_ok() && taken == 0 {
                error = Err(Errno::EIO);
            }

            fb_delete(fbuf);
        }

        // out:
        uio.uio_offset = foffset;

        if error.is_ok() {
            *eofflagp = eofflag;
        }

        if diropen {
            let _ = fusefs_file_close(fmp, &ip, FufhKind::RdOnly, O_RDONLY, true, p);
        }

        error
    })
}

/// `fusefs_inactive` (`vop_inactive`): releases every daemon handle of the node, unlocks it.
pub fn fusefs_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    // Close all open file handles.
    for t in FufhKind::ALL {
        if ip.fufh(t).fh_type != FUFH_INVALID {
            // FUSE file systems expect the same flags to be sent on release that were sent
            // on open. We don't have a record of them so make a best guess.
            let flags = match t {
                FufhKind::RdOnly => O_RDONLY,
                FufhKind::WrOnly => O_WRONLY,
                FufhKind::RdWr => O_RDWR,
            };

            let _ = proc_or_cur(p, |p| {
                fusefs_file_close(fmp, &ip, t, flags, vp.vtype() == VDIR, p)
            });
        }
    }

    let _ = vop_unlock(vp);

    // Don't return error to prevent kernel panic in vclean(9).
    Ok(())
}

/// `fusefs_readlink` (`vop_readlink`): `FUSE_READLINK`.
pub fn fusefs_readlink<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }

    if fmp.undef_op.get() & UNDEF_READLINK != 0 {
        return Err(Errno::ENOSYS);
    }

    let fbuf = proc_or_cur(uio.uio_procp, |p| {
        fb_setup(0, ip.i_number, FUSE_READLINK, p)
    });

    if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
        if error == Errno::ENOSYS {
            fmp.undef_op.set(fmp.undef_op.get() | UNDEF_READLINK);
        }

        fb_delete(fbuf);
        return Err(error);
    }

    let mut dat = fbuf.take_dat();
    let dat = dat.as_deref_mut().unwrap_or_default();
    if dat.contains(&0) {
        // DPRINTF("symbolic link contains embedded NUL: %s\n", fbuf->fb_dat);

        fb_delete(fbuf);
        return Err(Errno::EIO);
    }

    let error = uiomove(dat, uio);
    fb_delete(fbuf);

    error
}

/// `fusefs_reclaim` (`vop_reclaim`): releases the handles, tells the daemon to forget the
/// node (`FUSE_FORGET` with its lookup count), unhashes it and gives the vnode's handle up.
pub fn fusefs_reclaim<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    proc_or_cur(p, |p| {
        // Close opened files.
        for t in FufhKind::ALL {
            let fufh = ip.fufh(t);
            if fufh.fh_type != FUFH_INVALID {
                // DPRINTF("vnode being reclaimed is valid\n");
                let _ = fusefs_file_close(fmp, &ip, t, t as i32, vp.vtype() == VDIR, p);
            }
        }

        // If the fuse connection is opened ask libfuse to free the vnodes.
        if fmp.sess_init.get() != 0 && ip.i_number != FUSE_ROOT_ID {
            let fbuf = fb_setup(0, ip.i_number, FUSE_FORGET, p);
            fbuf.op_set(&FuseForgetIn {
                nlookup: ip.nlookup.get(),
            });
            let _ = fuse_device_queue_fbuf::<G>(fmp.dev, fbuf);
            // FUSE_FORGET has no response
        }
    });

    // Remove the inode from its hash chain.
    fuse_ihashrem(&ip);

    // free(ip, M_FUSEFS, sizeof(*ip)); vp->v_data = NULL: the slot's handle goes, the last
    // one (`ip`, below) frees the node.
    drop(vp.take_node());

    // Must return success otherwise kernel panic in vclean(9).
    Ok(())
}

/// `fusefs_print` (`vop_print`): describe the node (`DEBUG`/`DIAGNOSTIC` kernels only).
pub fn fusefs_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        let ip = vtoi(vp);

        // Complete the information given by vprint().
        crate::prf::printf(format_args!("tag VT_FUSE, hash id {} ", ip.i_number));
        crate::prf::printf(format_args!("\n"));
    }
    #[cfg(not(any(feature = "debug", feature = "diagnostic")))]
    let _ = vp;
    Ok(())
}

/// `fusefs_create` (`vop_create`): `FUSE_MKNOD` of a regular file.
pub fn fusefs_create<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let p = cnp.proc();

    let ip = vtoi(dvp);
    let fmp = &ip.i_fmp;
    let mode = makeimode(vap.va_type, vap.va_mode);

    if fmp.sess_init.get() == 0 {
        let _ = vop_abortop(dvp, cnp);
        return Err(Errno::ENXIO);
    }

    if fmp.undef_op.get() & UNDEF_MKNOD != 0 {
        let _ = vop_abortop(dvp, cnp);
        return Err(Errno::ENOSYS);
    }

    let name = cnp.name();
    let fbuf = fb_setup(name.len() + 1, ip.i_number, FUSE_MKNOD, p);

    fbuf.op_set(&FuseMknodIn {
        mode,
        umask: p.fd().cmask(),
        ..FuseMknodIn::default()
    });
    fb_put_name(&fbuf, 0, name);

    let error = 'out: {
        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_MKNOD);
            }

            break 'out Err(error);
        }

        // mknod returns a fuse_entry_out with the ino of the new file
        let nodeid = fbuf.op_get::<FuseEntryOut>().nodeid;
        if nodeid == 0 || nodeid == FUSE_ROOT_ID {
            break 'out Err(Errno::EIO);
        }

        let tdp = match VFS_VGET(fmp.mp, nodeid) {
            Ok(tdp) => tdp,
            Err(e) => break 'out Err(e),
        };

        tdp.set_vtype(VREG);

        *vpp = Some(tdp);
        VN_KNOTE(dvp, NOTE_WRITE);
        Ok(())
    };

    // out:
    fb_delete(fbuf);
    pnbuf_free(cnp);
    error
}

/// `fusefs_mknod` (`vop_mknod`): `FUSE_MKNOD`; the new node is dropped from the cache so
/// that it is reloaded (and checked for an alias) by `VFS_VGET`.
pub fn fusefs_mknod<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let p = cnp.proc();

    let ip = vtoi(dvp);
    let fmp = &ip.i_fmp;

    if fmp.sess_init.get() == 0 {
        let _ = vop_abortop(dvp, cnp);
        return Err(Errno::ENXIO);
    }

    if fmp.undef_op.get() & UNDEF_MKNOD != 0 {
        let _ = vop_abortop(dvp, cnp);
        return Err(Errno::ENOSYS);
    }

    let name = cnp.name();
    let fbuf = fb_setup(name.len() + 1, ip.i_number, FUSE_MKNOD, p);

    let mut mknod = FuseMknodIn {
        mode: makeimode(vap.va_type, vap.va_mode),
        umask: p.fd().cmask(),
        ..FuseMknodIn::default()
    };
    if vap.va_rdev != VNOVAL {
        mknod.rdev = vap.va_rdev as u32;
    }
    fbuf.op_set(&mknod);
    fb_put_name(&fbuf, 0, name);

    let error = 'out: {
        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_MKNOD);
            }

            break 'out Err(error);
        }

        // mknod returns a fuse_entry_out with the ino of the new file
        let nodeid = fbuf.op_get::<FuseEntryOut>().nodeid;
        if nodeid == 0 || nodeid == FUSE_ROOT_ID {
            break 'out Err(Errno::EIO);
        }

        let tdp = match VFS_VGET(fmp.mp, nodeid) {
            Ok(tdp) => tdp,
            Err(e) => break 'out Err(e),
        };

        // Don't trust the type returned by the file system and assume it created what it
        // was asked.
        tdp.set_vtype(vap.va_type);

        *vpp = Some(tdp);
        VN_KNOTE(dvp, NOTE_WRITE);

        // Remove inode so that it will be reloaded by VFS_VGET and checked to see if it is
        // an alias of an existing entry in the inode cache.
        vput(tdp);
        tdp.set_vtype(VNON);
        vgone(tdp);
        *vpp = None;
        Ok(())
    };

    // out:
    fb_delete(fbuf);
    pnbuf_free(cnp);
    error
}

/// `fusefs_read` (`vop_read`): `FUSE_READ`s of at most `max_read` bytes until the request
/// is done or the daemon returns less than asked.
pub fn fusefs_read<G: ProcGlue>(vp: &'static Vnode<G>, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }
    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }

    proc_or_cur(uio.uio_procp, |p| {
        let mut fbuf = None;
        let mut error = Ok(());
        while uio.uio_resid > 0 {
            let f = fbuf.insert(fb_setup(0, ip.i_number, FUSE_READ, p));

            let size = uio.uio_resid.min(fmp.max_read as usize);
            f.op_set(&FuseReadIn {
                fh: fusefs_fd_get(&ip, FufhKind::RdOnly),
                offset: uio.uio_offset as u64,
                size: size as u32,
                ..FuseReadIn::default()
            });

            error = fb_queue::<G>(fmp.dev, f);

            if error.is_err() {
                break;
            }

            let n = (size as u64).min(f.fb_len()) as usize;
            let mut dat = f.take_dat();
            let dat = dat.as_deref_mut().unwrap_or_default();
            error = uiomove(&mut dat[..n], uio);
            if error.is_err() {
                break;
            }

            if f.fb_len() < size as u64 {
                break;
            }

            fb_delete(fbuf.take());
        }

        fb_delete(fbuf);
        error
    })
}

/// `fusefs_write` (`vop_write`): `FUSE_WRITE`s of at most `max_write` bytes; the daemon may
/// write less than asked, which the uio gets back.
pub fn fusefs_write<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    ioflag: i32,
) -> Result<(), Errno> {
    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    // XXX fmp->max_write will not be set to the value wanted by the file system if
    // sess_init == PENDING. It is not currently possible for this to be the case since a
    // write operation cannot take place on a vnode until after VOP_LOOKUP(9) has
    // successfully returned. At which point, the file system must have responded to
    // FUSE_INIT and sess_init will be 1 and max_write will be correctly set.
    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }
    if uio.uio_resid == 0 {
        return Ok(());
    }

    proc_or_cur(uio.uio_procp, |p| {
        if ioflag & IO_APPEND != 0 {
            let cred = p.ucred();
            let mut vattr = Vattr::new();
            vop_getattr(vp, &mut vattr, Cred::real(cred), p)?;

            uio.uio_offset = vattr.va_size as i64;
        }

        let mut fbuf = None;
        let mut error = Ok(());
        while uio.uio_resid > 0 {
            let len = uio.uio_resid.min(fmp.max_write.get() as usize);
            let f = fbuf.insert(fb_setup(len, ip.i_number, FUSE_WRITE, p));

            f.op_set(&FuseWriteIn {
                fh: fusefs_fd_get(&ip, FufhKind::WrOnly),
                offset: uio.uio_offset as u64,
                size: len as u32,
                ..FuseWriteIn::default()
            });

            let mut dat = f.take_dat();
            error = uiomove(dat.as_deref_mut().unwrap_or_default(), uio);
            f.set_dat(dat);
            if error.is_err() {
                // DPRINTF("uio error %i\n", error);
                break;
            }

            error = fb_queue::<G>(fmp.dev, f);

            if error.is_err() {
                break;
            }

            let written = f.op_get::<FuseWriteOut>().size as usize;
            let diff = len.wrapping_sub(written);
            if written > len {
                error = Err(Errno::EINVAL);
                break;
            }

            uio.uio_resid += diff;
            uio.uio_offset -= diff as i64;

            // A reply that wrote nothing: EIO (the module's deviations, D-N5.9-U2).
            if written == 0 {
                error = Err(Errno::EIO);
                break;
            }

            if uio.uio_offset > ip.filesize.get() {
                ip.filesize.set(uio.uio_offset);
                uvm_vnp_setsize(vp, uio.uio_offset);
            }
            uvm_vnp_uncache(vp);

            fb_delete(fbuf.take());

            // A short write ends the call: the uio's iovec is past the chunk the daemon did
            // not all take (the module's deviations, D-N5.9-U4).
            if diff > 0 {
                break;
            }
        }

        fb_delete(fbuf);
        error
    })
}

/// The C's `abortit:` sequence of `fusefs_rename`: release everything the rename was given.
fn fusefs_rename_abort<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) {
    let _ = vop_abortop(tdvp, tcnp); // XXX, why not in NFS?
    if opt_eq(Some(tdvp), tvp) {
        vrele(tdvp);
    } else {
        vput(tdvp);
    }
    if let Some(tvp) = tvp {
        vput(tvp);
    }
    let _ = vop_abortop(fdvp, fcnp); // XXX, why not in NFS?
    vrele(fdvp);
    vrele(fvp);
}

/// `fusefs_rename` (`vop_rename`): `FUSE_RENAME` with both names and the target directory.
pub fn fusefs_rename<G: ProcGlue>(
    fdvp: &'static Vnode<G>,
    fvp: &'static Vnode<G>,
    fcnp: &mut Cn<'_, G>,
    tdvp: &'static Vnode<G>,
    tvp: Option<&'static Vnode<G>>,
    tcnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let p = fcnp.proc();

    #[cfg(feature = "diagnostic")]
    {
        use crate::fs::namei::HASBUF;
        if tcnp.cn_flags & HASBUF == 0 || fcnp.cn_flags & HASBUF == 0 {
            kpanic(format_args!("fusefs_rename: no name"));
        }
    }

    // Check for cross-device rename.
    if !opt_eq(fvp.mount(), tdvp.mount())
        || tvp.is_some_and(|tvp| !opt_eq(fvp.mount(), tvp.mount()))
    {
        fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(Errno::EXDEV);
    }

    // If source and dest are the same, do nothing.
    if opt_eq(tvp, Some(fvp)) {
        fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Ok(());
    }

    if let Err(error) = vn_lock(fvp, LK_EXCLUSIVE | LK_RETRY) {
        fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(error);
    }
    let dp = vtoi(fdvp);
    let ip = vtoi(fvp);
    let fmp = &ip.i_fmp;

    // Be sure we are not renaming ".", "..", or an alias of ".". This leads to a crippled
    // directory tree. It's pretty tough to do a "ls" or "pwd" with the "." directory entry
    // missing, and "cd .." doesn't work if the ".." entry is missing.
    if fvp.vtype() == VDIR {
        // Avoid ".", "..", and aliases of "." for obvious reasons.
        if fcnp.name() == b"."
            || ptr::eq(ptr::from_ref(&*dp), ptr::from_ref(&*ip))
            || fcnp.cn_flags & ISDOTDOT != 0
            || tcnp.cn_flags & ISDOTDOT != 0
        {
            let _ = vop_unlock(fvp);
            fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
            return Err(Errno::EINVAL);
        }
    }
    VN_KNOTE(fdvp, NOTE_WRITE); // XXX right place?

    if fmp.sess_init.get() == 0 {
        let _ = vop_unlock(fvp);
        fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(Errno::ENXIO);
    }

    if fmp.undef_op.get() & UNDEF_RENAME != 0 {
        let _ = vop_unlock(fvp);
        fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(Errno::ENOSYS);
    }

    let fname = fcnp.name();
    let tname = tcnp.name();
    let fbuf = fb_setup(fname.len() + tname.len() + 2, dp.i_number, FUSE_RENAME, p);

    fb_put_name(&fbuf, 0, fname);
    fb_put_name(&fbuf, fname.len() + 1, tname);
    fbuf.op_set(&FuseRenameIn {
        newdir: vtoi(tdvp).i_number,
    });

    if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
        if error == Errno::ENOSYS {
            fmp.undef_op.set(fmp.undef_op.get() | UNDEF_RENAME);
        }

        fb_delete(fbuf);
        let _ = vop_unlock(fvp);
        fusefs_rename_abort(fdvp, fvp, fcnp, tdvp, tvp, tcnp);
        return Err(error);
    }

    fb_delete(fbuf);
    VN_KNOTE(fvp, NOTE_RENAME);

    let _ = vop_unlock(fvp);
    if opt_eq(Some(tdvp), tvp) {
        vrele(tdvp);
    } else {
        vput(tdvp);
    }
    if let Some(tvp) = tvp {
        vput(tvp);
    }
    vrele(fdvp);
    vrele(fvp);

    Ok(())
}

/// `fusefs_mkdir` (`vop_mkdir`): `FUSE_MKDIR`.
pub fn fusefs_mkdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
    vap: &Vattr,
) -> Result<(), Errno> {
    let p = cnp.proc();

    let ip = vtoi(dvp);
    let fmp = &ip.i_fmp;

    let error = 'out: {
        if fmp.sess_init.get() == 0 {
            break 'out Err(Errno::ENXIO);
        }

        if fmp.undef_op.get() & UNDEF_MKDIR != 0 {
            break 'out Err(Errno::ENOSYS);
        }

        let name = cnp.name();
        let fbuf = fb_setup(name.len() + 1, ip.i_number, FUSE_MKDIR, p);

        fbuf.op_set(&FuseMkdirIn {
            mode: makeimode(vap.va_type, vap.va_mode),
            umask: p.fd().cmask(),
        });
        fb_put_name(&fbuf, 0, name);

        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_MKDIR);
            }

            fb_delete(fbuf);
            break 'out Err(error);
        }

        // mkdir returns a fuse_entry_out with the ino of the new directory
        let nodeid = fbuf.op_get::<FuseEntryOut>().nodeid;
        if nodeid == 0 || nodeid == FUSE_ROOT_ID {
            fb_delete(fbuf);
            break 'out Err(Errno::EIO);
        }

        let tdp = match VFS_VGET(fmp.mp, nodeid) {
            Ok(tdp) => tdp,
            Err(e) => {
                fb_delete(fbuf);
                break 'out Err(e);
            }
        };

        tdp.set_vtype(VDIR);

        *vpp = Some(tdp);
        VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
        fb_delete(fbuf);
        Ok(())
    };

    // out:
    pnbuf_free(cnp);
    vput(dvp);
    error
}

/// `fusefs_rmdir` (`vop_rmdir`): `FUSE_RMDIR`.
pub fn fusefs_rmdir<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let p = cnp.proc();
    let mut held = Some(dvp);

    let ip = vtoi(vp);
    let dp = vtoi(dvp);
    let fmp = &ip.i_fmp;

    let error = 'out: {
        if fmp.sess_init.get() == 0 {
            break 'out Err(Errno::ENXIO);
        }

        if fmp.undef_op.get() & UNDEF_RMDIR != 0 {
            break 'out Err(Errno::ENOSYS);
        }

        // Don't delete parent since it's clearly not empty.
        let name = cnp.name();
        if name == b".." {
            break 'out Err(Errno::ENOTEMPTY);
        }

        VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);

        let fbuf = fb_setup(name.len() + 1, dp.i_number, FUSE_RMDIR, p);
        fb_put_name(&fbuf, 0, name);

        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_RMDIR);
            }
            if error != Errno::ENOTEMPTY {
                VN_KNOTE(dvp, NOTE_WRITE | NOTE_LINK);
            }

            fb_delete(fbuf);
            break 'out Err(error);
        }

        vput(dvp);
        held = None;

        fb_delete(fbuf);
        Ok(())
    };

    // out:
    if let Some(dvp) = held {
        vput(dvp);
    }
    VN_KNOTE(vp, NOTE_DELETE);
    pnbuf_free(cnp);
    vput(vp);
    error
}

/// `fusefs_remove` (`vop_remove`): `FUSE_UNLINK`.
pub fn fusefs_remove<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: &'static Vnode<G>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let p = cnp.proc();

    let ip = vtoi(vp);
    let dp = vtoi(dvp);
    let fmp = &ip.i_fmp;

    let error = 'out: {
        if fmp.sess_init.get() == 0 {
            break 'out Err(Errno::ENXIO);
        }

        if fmp.undef_op.get() & UNDEF_REMOVE != 0 {
            break 'out Err(Errno::ENOSYS);
        }

        let name = cnp.name();
        let fbuf = fb_setup(name.len() + 1, dp.i_number, FUSE_UNLINK, p);
        fb_put_name(&fbuf, 0, name);

        if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
            if error == Errno::ENOSYS {
                fmp.undef_op.set(fmp.undef_op.get() | UNDEF_REMOVE);
            }

            fb_delete(fbuf);
            break 'out Err(error);
        }

        VN_KNOTE(vp, NOTE_DELETE);
        VN_KNOTE(dvp, NOTE_WRITE);
        fb_delete(fbuf);
        Ok(())
    };

    // out:
    pnbuf_free(cnp);
    error
}

/// `fusefs_strategy` (`vop_strategy`): FUSE has no buffers.
pub fn fusefs_strategy() -> Result<(), Errno> {
    Ok(())
}

/// `fusefs_lock` (`vop_lock`): take the node's lock.
pub fn fusefs_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    vtoi(vp).i_lock.enter(flags & LK_RWFLAGS)
}

/// `fusefs_unlock` (`vop_unlock`).
pub fn fusefs_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    vtoi(vp).i_lock.exit();
    Ok(())
}

/// `fusefs_islocked` (`vop_islocked`).
pub fn fusefs_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    vtoi(vp).i_lock.status()
}

/// `fusefs_advlock` (`vop_advlock`): advisory record locking on the node.
pub fn fusefs_advlock<G: ProcGlue>(
    vp: &'static Vnode<G>,
    id: LockOwner,
    op: i32,
    fl: &mut Flock,
    flags: i32,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    lf_advlock::<G>(&ip.i_lockf, ip.filesize.get(), id, op, fl, flags)
}

/// `fusefs_fsync` (`vop_fsync`): `FUSE_FSYNC` of every writable handle.
pub fn fusefs_fsync<G: ProcGlue>(vp: &'static Vnode<G>, p: &Thread<G>) -> Result<(), Errno> {
    let mut error = Ok(());

    // Can't write to directory file handles so no need to fsync. FUSE has fsyncdir but it
    // doesn't make sense on OpenBSD.
    if vp.vtype() == VDIR {
        return Ok(());
    }

    let ip = vtoi(vp);
    let fmp = &ip.i_fmp;

    if fmp.sess_init.get() == 0 {
        return Err(Errno::ENXIO);
    }

    // Implementing fsync is optional so don't error.
    if fmp.undef_op.get() & UNDEF_FSYNC != 0 {
        return Ok(());
    }

    // Sync all writeable file descriptors.
    for t in FufhKind::ALL {
        let fufh = ip.fufh(t);
        if fufh.fh_type == FUFH_WRONLY || fufh.fh_type == FUFH_RDWR {
            let fbuf = fb_setup(0, ip.i_number, FUSE_FSYNC, p);
            // fdatasync(2) is just a wrapper around fsync(2) so datasync is always false.
            fbuf.op_set(&FuseFsyncIn {
                fh: fufh.fh_id,
                fsync_flags: 0,
                ..FuseFsyncIn::default()
            });

            // Always behave as if ap->a_waitfor = MNT_WAIT.
            error = fb_queue::<G>(fmp.dev, &fbuf);
            fb_delete(fbuf);
            if error.is_err() {
                break;
            }
        }
    }

    if error == Err(Errno::ENOSYS) {
        fmp.undef_op.set(fmp.undef_op.get() | UNDEF_FSYNC);

        // Implementing fsync is optional so don't error.
        return Ok(());
    }

    error
}

/// `fusefs_vops`: each slot the C's function of the same name; `vop_revoke` and `vop_bwrite`
/// NULL (the trait's defaults), `vop_abortop`, `vop_bmap` and `vop_pathconf` the generic
/// helpers the C installs.
impl<G: ProcGlue> Vnops<G> for FuseVnops<G> {
    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        fusefs_lookup(dvp, vpp, cnp)
    }
    fn create(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        fusefs_create(dvp, vpp, cnp, vap)
    }
    fn mknod(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        fusefs_mknod(dvp, vpp, cnp, vap)
    }
    fn open(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        _cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_open(vp, mode, p)
    }
    fn close(
        &self,
        vp: &'static Vnode<G>,
        fflag: i32,
        _cred: Cred<'_>,
        p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        fusefs_close(vp, fflag, p)
    }
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_access(vp, mode, cred, p)
    }
    fn getattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        _cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_getattr(vp, vap, p)
    }
    fn setattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_setattr(vp, vap, cred, p)
    }
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        fusefs_read(vp, uio)
    }
    fn write(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        fusefs_write(vp, uio, ioflag)
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
        fusefs_ioctl()
    }
    fn kqfilter(&self, vp: &'static Vnode<G>, _fflag: i32, kn: &Knote<G>) -> Result<(), Errno> {
        fusefs_kqfilter(vp, kn)
    }
    fn fsync(
        &self,
        vp: &'static Vnode<G>,
        _cred: Cred<'_>,
        _waitfor: i32,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        fusefs_fsync(vp, p)
    }
    fn remove(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        fusefs_remove(dvp, vp, cnp)
    }
    fn link(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        fusefs_link(dvp, vp, cnp)
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
        fusefs_rename(fdvp, fvp, fcnp, tdvp, tvp, tcnp)
    }
    fn mkdir(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        vap: &mut Vattr,
    ) -> Result<(), Errno> {
        fusefs_mkdir(dvp, vpp, cnp, vap)
    }
    fn rmdir(
        &self,
        dvp: &'static Vnode<G>,
        vp: &'static Vnode<G>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        fusefs_rmdir(dvp, vp, cnp)
    }
    fn symlink(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
        _vap: &mut Vattr,
        target: &[u8],
    ) -> Result<(), Errno> {
        fusefs_symlink(dvp, vpp, cnp, target)
    }
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        fusefs_readdir(vp, uio, eofflag)
    }
    fn readlink(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        fusefs_readlink(vp, uio)
    }
    fn abortop(&self, _dvp: &'static Vnode<G>, cnp: &mut Cn<'_, G>) -> Result<(), Errno> {
        default::vop_generic_abortop(cnp)
    }
    fn inactive(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
        fusefs_inactive(vp, p)
    }
    fn reclaim(&self, vp: &'static Vnode<G>, p: Option<&Thread<G>>) -> Result<(), Errno> {
        fusefs_reclaim(vp, p)
    }
    fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        fusefs_lock(vp, flags)
    }
    fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        fusefs_unlock(vp)
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
        fusefs_strategy()
    }
    fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        fusefs_print(vp)
    }
    fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
        fusefs_islocked(vp)
    }
    fn pathconf(
        &self,
        _vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        spec_pathconf(name, retval)
    }
    fn advlock(
        &self,
        vp: &'static Vnode<G>,
        id: LockOwner,
        op: i32,
        fl: &mut Flock,
        flags: i32,
    ) -> Result<(), Errno> {
        fusefs_advlock(vp, id, op, fl, flags)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the vnode operations over testfuse's mount and scripted daemon (each
    // request's bytes checked as the daemon reads them): create, mknod, mkdir and symlink
    // with the C's `EIO` for a 0 or root `nodeid`; link, rename, remove and rmdir; open and
    // close (`FUSE_FLUSH`, an optional operation the daemon may lack), fsync, readlink (an
    // embedded NUL is `EIO`), read and write in chunks; the user's D-N5.9-U2 bounds: a
    // readdir reply no entry can be taken from and a write reply of size 0 answer `EIO`
    // where the C loops for ever (run under bounded.pl: a hang is the detection by
    // mutation); readdir's `goto out` for a full buffer and its name checks; the generic
    // slots, the lock and the filters.

    use core::ptr;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne, vec};

    use hal::sys::types::{Dev, Off};

    use super::*;
    use crate::event::{EVFILT_USER, kqueue_alloc, kqueue_init, test_knote, test_knote_drop};
    use crate::fs::fuse::fuse_device::fuseclose;
    use crate::fs::fuse::fuse_vfsops::fusefs_unmount;
    use crate::fs::fuse::fusebuf_h::{
        FUSE_LOOKUP, FUSE_OPEN, FUSE_OPENDIR, FUSE_RELEASE, FUSE_RELEASEDIR, FuseOpenIn,
        FuseOpenOut, FuseReleaseIn,
    };
    use crate::fs::fuse::testfuse::{
        Answer, content, dirents, hello_attr, hello_fs, mounted, rdwr, readdir, script, scripted,
        set_daemon, take_ops, take_seen,
    };
    use crate::fs::mount::Mount;
    use crate::fs::namei::{CREATE, Componentname, ISLASTCN, LOCKPARENT, LOOKUP};
    use crate::fs::subr::vfs;
    use crate::fs::subr::{vattr_null, vref};
    use crate::fs::testfs::TestGlue;
    use crate::fs::vnode::VCHR;
    use crate::fs::vops::{
        vop_close, vop_create, vop_fsync, vop_ioctl, vop_islocked, vop_link, vop_lookup, vop_mkdir,
        vop_mknod, vop_open, vop_pathconf, vop_readlink, vop_remove, vop_rename, vop_rmdir,
        vop_symlink,
    };
    use crate::uio::{Iovec, UioRw, UioSeg};
    use hal::sys::unistd::_PC_LINK_MAX;

    type P = &'static Thread<TestGlue>;
    type V = &'static Vnode<TestGlue>;

    /// A component name for `name`.
    fn cn(p: P, op: u64, name: &'static [u8], flags: u64) -> Componentname<'static, TestGlue> {
        let mut cnp = Componentname::new(op, flags, p, Cred::real(p.ucred()));
        cnp.set_name_lit(name);
        cnp
    }

    /// `hello`'s vnode, looked up in the root and unlocked (one reference).
    fn hello(p: P, root: V) -> V {
        let mut vpp = None;
        let mut cnp = cn(p, LOOKUP, b"hello", LOCKPARENT | ISLASTCN);
        vop_lookup(root, &mut vpp, &mut cnp).expect("lookup hello");
        let vp = vpp.expect("hello");
        let _ = vop_unlock(vp);
        vp
    }

    /// The directory vnode `dvp` referenced and locked once more, for an operation that
    /// `vput`s it (the namei's reference and lock in the C).
    fn lent(dvp: V) -> V {
        vref(dvp);
        vn_lock(dvp, LK_EXCLUSIVE).expect("lock");
        dvp
    }

    /// The first request of opcode `op` the daemon read since the last look.
    fn op_seen(op: u32) -> crate::fs::fuse::testfuse::Req {
        match take_seen().into_iter().find(|r| r.hdr.opcode == op) {
            Some(r) => r,
            None => std::panic!("no request of opcode {op}"),
        }
    }

    /// Unmounts (the root given back) and closes the device of [`mounted`].
    fn done(p: P, dev: Dev, mp: &'static Mount<TestGlue>, root: V) {
        vput(root);
        fusefs_unmount(mp, 0, p).expect("unmount");
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }

    /// The daemon answers `op` with an entry of inode `ino`, and `FUSE_GETATTR` of it.
    fn entry(op: u32, ino: u64) {
        script(op, move |_| {
            Answer::out(&FuseEntryOut {
                nodeid: ino,
                attr: hello_attr(ino),
                ..FuseEntryOut::default()
            })
        });
    }

    #[test]
    fn create_mknod_mkdir_symlink_and_their_eio() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"");
        let mut va = Vattr::new();
        vattr_null(&mut va);
        va.va_type = VREG;
        va.va_mode = 0o644;

        // create: FUSE_MKNOD of S_IFREG | mode, the umask, the name and its NUL.
        entry(FUSE_MKNOD, 4);
        let mut vpp = None;
        let mut cnp = cn(p, CREATE, b"f", LOCKPARENT | ISLASTCN);
        vop_create(root, &mut vpp, &mut cnp, &mut va).expect("create");
        let f = vpp.expect("f");
        assert_eq!(f.vtype(), VREG);
        let seen = take_seen();
        let r = &seen[0];
        assert_eq!(
            (r.hdr.opcode, r.hdr.nodeid, &r.data[..]),
            (FUSE_MKNOD, 1, &b"f\0"[..])
        );
        let m = r.op::<FuseMknodIn>();
        assert_eq!((m.mode, m.umask), (0o100644, p.fd().cmask()));
        vput(f);

        // mknod: the given rdev; the vnode gone (VFS_VGET reloads it), no vnode returned.
        va.va_type = VCHR;
        va.va_rdev = 0x0102;
        entry(FUSE_MKNOD, 5);
        let mut vpp = None;
        let mut cnp = cn(p, CREATE, b"c", LOCKPARENT | ISLASTCN);
        vop_mknod(root, &mut vpp, &mut cnp, &mut va).expect("mknod");
        assert!(vpp.is_none());
        let r = &take_seen()[0];
        assert_eq!(r.op::<FuseMknodIn>().rdev, 0x0102);
        assert_eq!(r.op::<FuseMknodIn>().mode, 0o020644);

        // mkdir: FUSE_MKDIR; the directory's vnode (vput as the C's).
        va.va_type = VDIR;
        va.va_mode = 0o755;
        entry(FUSE_MKDIR, 6);
        let mut vpp = None;
        let mut cnp = cn(p, CREATE, b"d", LOCKPARENT | ISLASTCN);
        vop_mkdir(lent(root), &mut vpp, &mut cnp, &mut va).expect("mkdir");
        let d = vpp.expect("d");
        assert_eq!(d.vtype(), VDIR);
        assert_eq!(op_seen(FUSE_MKDIR).op::<FuseMkdirIn>().mode, 0o040755);
        vput(d);

        // symlink: the name, its NUL, the target, its NUL.
        entry(FUSE_SYMLINK, 7);
        let mut vpp = None;
        let mut cnp = cn(p, CREATE, b"l", LOCKPARENT | ISLASTCN);
        vop_symlink(lent(root), &mut vpp, &mut cnp, &mut va, b"/target\0").expect("symlink");
        assert!(vpp.is_some_and(|v| v.vtype() == VLNK));
        assert_eq!(op_seen(FUSE_SYMLINK).data, b"l\0/target\0");

        // A 0 or root nodeid is EIO for all four (fuse_vnops.c:753, :1128, :1195), each
        // request freed.
        let out = vfs::<TestGlue>().fuse.fbuf_pool.nout();
        for ino in [0, FUSE_ROOT_ID] {
            for op in [FUSE_MKNOD, FUSE_MKDIR, FUSE_SYMLINK] {
                entry(op, ino);
            }
            va.va_type = VREG;
            let mut vpp = None;
            let mut cnp = cn(p, CREATE, b"x", LOCKPARENT | ISLASTCN);
            assert_eq!(
                vop_create(root, &mut vpp, &mut cnp, &mut va),
                Err(Errno::EIO)
            );
            let mut cnp = cn(p, CREATE, b"x", LOCKPARENT | ISLASTCN);
            assert_eq!(
                vop_mknod(root, &mut vpp, &mut cnp, &mut va),
                Err(Errno::EIO)
            );
            let mut cnp = cn(p, CREATE, b"x", LOCKPARENT | ISLASTCN);
            assert_eq!(
                vop_mkdir(lent(root), &mut vpp, &mut cnp, &mut va),
                Err(Errno::EIO)
            );
            let mut cnp = cn(p, CREATE, b"x", LOCKPARENT | ISLASTCN);
            assert_eq!(
                vop_symlink(lent(root), &mut vpp, &mut cnp, &mut va, b"t"),
                Err(Errno::EIO)
            );
            assert!(vpp.is_none());
        }
        assert_eq!(
            vfs::<TestGlue>().fuse.fbuf_pool.nout(),
            out,
            "every refused request freed (mkdir's too, where the C leaks it)"
        );

        // A daemon without MKNOD: ENOSYS once, then refused in the kernel.
        script(FUSE_MKNOD, |_| Answer::err(Errno::ENOSYS));
        let mut vpp = None;
        let mut cnp = cn(p, CREATE, b"y", LOCKPARENT | ISLASTCN);
        assert_eq!(
            vop_create(root, &mut vpp, &mut cnp, &mut va),
            Err(Errno::ENOSYS)
        );
        let _ = take_seen();
        let mut cnp = cn(p, CREATE, b"y", LOCKPARENT | ISLASTCN);
        assert_eq!(
            vop_create(root, &mut vpp, &mut cnp, &mut va),
            Err(Errno::ENOSYS)
        );
        assert!(take_seen().is_empty(), "UNDEF_MKNOD");
        done(p, dev, mp, root);
    }

    #[test]
    fn link_rename_remove_rmdir() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"hi");
        let h = hello(p, root);
        script(FUSE_LINK, |r| {
            Answer::out(&FuseEntryOut {
                nodeid: r.op::<FuseLinkIn>().oldnodeid,
                attr: hello_attr(2),
                ..FuseEntryOut::default()
            })
        });
        script(FUSE_RENAME, |_| Answer::ok());
        script(FUSE_UNLINK, |_| Answer::ok());
        script(FUSE_RMDIR, |r| {
            if r.data == b"full\0" {
                Answer::err(Errno::ENOTEMPTY)
            } else {
                Answer::ok()
            }
        });
        let _ = take_seen();

        // link: FUSE_LINK on the directory with the old node and the new name; dvp vput.
        let mut cnp = cn(p, CREATE, b"h2", LOCKPARENT | ISLASTCN);
        vop_link(lent(root), h, &mut cnp).expect("link");
        let r = &op_seen(FUSE_LINK);
        assert_eq!(
            (
                r.hdr.opcode,
                r.hdr.nodeid,
                r.op::<FuseLinkIn>().oldnodeid,
                &r.data[..]
            ),
            (FUSE_LINK, 1, 2, &b"h2\0"[..])
        );
        assert_eq!(vop_islocked(h), 0, "the node unlocked again");

        // rename hello -> h3 in the root: both names, the target directory.
        vref(h);
        let mut fcn = cn(p, crate::fs::namei::DELETE, b"hello", ISLASTCN);
        let mut tcn = cn(p, crate::fs::namei::RENAME, b"h3", ISLASTCN);
        vref(root);
        vop_rename(root, h, &mut fcn, lent(root), None, &mut tcn).expect("rename");
        let r = &op_seen(FUSE_RENAME);
        assert_eq!(
            (r.hdr.opcode, r.op::<FuseRenameIn>().newdir, &r.data[..]),
            (FUSE_RENAME, 1, &b"hello\0h3\0"[..])
        );
        // The same node as source and target: nothing asked.
        vref(h);
        vref(h);
        vref(root);
        vn_lock(h, LK_EXCLUSIVE).expect("lock");
        let mut fcn = cn(p, crate::fs::namei::DELETE, b"h3", ISLASTCN);
        let mut tcn = cn(p, crate::fs::namei::RENAME, b"h2", ISLASTCN);
        vop_rename(root, h, &mut fcn, lent(root), Some(h), &mut tcn).expect("same");
        assert!(take_seen().is_empty());

        // remove: FUSE_UNLINK of the name on the directory.
        let mut cnp = cn(p, crate::fs::namei::DELETE, b"h2", ISLASTCN);
        vop_remove(lent(root), lent(h), &mut cnp).expect("remove");
        assert_eq!(op_seen(FUSE_UNLINK).data, b"h2\0");

        // rmdir: "..", ENOTEMPTY in the kernel; the daemon's ENOTEMPTY passed on; success.
        // Both vnodes vput.
        let dir = |ino: u64| {
            entry(FUSE_LOOKUP, ino);
            let mut vpp = None;
            let mut cnp = cn(p, LOOKUP, b"d", LOCKPARENT | ISLASTCN);
            vop_lookup(root, &mut vpp, &mut cnp).expect("lookup d");
            vpp.expect("d")
        };
        let d = dir(8);
        let mut cnp = cn(p, crate::fs::namei::DELETE, b"..", ISLASTCN);
        assert_eq!(vop_rmdir(lent(root), d, &mut cnp), Err(Errno::ENOTEMPTY));
        let d = dir(8);
        let mut cnp = cn(p, crate::fs::namei::DELETE, b"full", ISLASTCN);
        let _ = take_seen();
        assert_eq!(vop_rmdir(lent(root), d, &mut cnp), Err(Errno::ENOTEMPTY));
        assert_eq!(take_ops(), [FUSE_RMDIR]);
        let d = dir(8);
        let mut cnp = cn(p, crate::fs::namei::DELETE, b"d", ISLASTCN);
        vop_rmdir(lent(root), d, &mut cnp).expect("rmdir");

        vrele(h);
        done(p, dev, mp, root);
    }

    #[test]
    fn open_close_fsync_readlink() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"hi");
        let h = hello(p, root);
        let cred = Cred::real(p.ucred());

        // Open read-write: FUSE_OPEN without O_CREAT | O_EXCL | O_TRUNC; a second open of
        // the kind asks nothing.
        let _ = take_seen();
        vop_open(h, FREAD | FWRITE | O_TRUNC, cred, p).expect("open");
        let r = &take_seen()[0];
        assert_eq!(r.hdr.opcode, FUSE_OPEN);
        assert_eq!(r.op::<FuseOpenIn>().flags as i32, O_RDWR);
        let ip = vtoi(h);
        assert_eq!(ip.fufh(FufhKind::RdWr).fh_type, FUFH_RDWR);
        vop_open(h, FREAD | FWRITE, cred, p).expect("again");
        assert!(take_seen().is_empty());

        // fsync: FUSE_FSYNC of the writable handle; a daemon without it is fine.
        script(FUSE_FSYNC, |_| Answer::err(Errno::ENOSYS));
        vop_fsync(h, cred, 0, p).expect("fsync");
        let r = &take_seen()[0];
        assert_eq!(
            (r.hdr.opcode, r.op::<FuseFsyncIn>().fh),
            (FUSE_FSYNC, 10 + O_RDWR as u64)
        );
        vop_fsync(h, cred, 0, p).expect("UNDEF_FSYNC");
        assert!(take_seen().is_empty());

        // close: FUSE_FLUSH of a writable handle, ENOSYS taken as success once and then not
        // asked; a read-only close asks nothing; a handle of a kind never opened is EBADF.
        script(FUSE_FLUSH, |_| Answer::err(Errno::ENOSYS));
        assert_eq!(vop_close(h, FWRITE, cred, Some(p)), Err(Errno::EBADF));
        vop_close(h, FREAD, cred, Some(p)).expect("read-only");
        assert!(take_seen().is_empty());
        vop_close(h, FREAD | FWRITE, cred, Some(p)).expect("flush");
        assert_eq!(take_ops(), [FUSE_FLUSH]);
        vop_close(h, FREAD | FWRITE, cred, None).expect("UNDEF_FLUSH");
        assert!(take_seen().is_empty());

        // readlink: the daemon's bytes; an embedded NUL is EIO; a 0-byte uio asks nothing.
        let mut buf = [0u8; 16];
        let link = |reply: &'static [u8], buf: &mut [u8]| {
            script(FUSE_READLINK, move |_| Answer::data(reply));
            let len = buf.len();
            let mut iov = [Iovec {
                iov_base: buf.as_mut_ptr().cast(),
                iov_len: len,
            }];
            let mut uio = Uio {
                uio_iov: &mut iov,
                uio_offset: 0,
                uio_resid: len,
                uio_segflg: UioSeg::UIO_SYSSPACE,
                uio_rw: UioRw::UIO_READ,
                uio_procp: Some(p),
            };
            vop_readlink(h, &mut uio, cred).map(|()| len - uio.uio_resid)
        };
        assert_eq!(link(b"/a/b", &mut buf), Ok(4));
        assert_eq!(&buf[..4], b"/a/b");
        assert_eq!(link(b"/a\0b", &mut buf), Err(Errno::EIO));
        let _ = take_seen();
        assert_eq!(link(b"x", &mut []), Ok(0));
        assert!(take_seen().is_empty());

        // The release of every handle at the last reference.
        vrele(h);
        assert_eq!(take_ops(), [FUSE_RELEASE]);
        done(p, dev, mp, root);
    }

    /// `VOP_WRITE` of `data` at `off`: the result, the residual and the offset it left.
    fn write(p: P, vp: V, data: &[u8], off: Off) -> (Result<(), Errno>, usize, Off) {
        let mut buf = data.to_vec();
        let len = buf.len();
        let mut iov = [Iovec {
            iov_base: buf.as_mut_ptr().cast(),
            iov_len: len,
        }];
        let mut uio = Uio {
            uio_iov: &mut iov,
            uio_offset: off,
            uio_resid: len,
            uio_segflg: UioSeg::UIO_SYSSPACE,
            uio_rw: UioRw::UIO_WRITE,
            uio_procp: Some(p),
        };
        let r = fusefs_write(vp, &mut uio, 0);
        (r, uio.uio_resid, uio.uio_offset)
    }

    #[test]
    fn read_and_write_in_chunks_and_the_write_bounds() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"0123456789");
        let h = hello(p, root);
        vop_open(h, FREAD | FWRITE, Cred::real(p.ucred()), p).expect("open");
        let fmp = vtoi(h).i_fmp.clone();

        // Reads of at most max_read (4096 here): a short reply ends the call.
        let _ = take_seen();
        let mut buf = vec![0u8; 5000];
        assert_eq!(rdwr(h, p, &mut buf, 0, false), Ok(10));
        assert_eq!(&buf[..10], b"0123456789");
        let sizes: Vec<u32> = take_seen()
            .iter()
            .map(|r| r.op::<FuseReadIn>().size)
            .collect();
        assert_eq!(sizes, [4096], "the reply was short");
        let mut b = [0u8; 4];
        assert_eq!(rdwr(h, p, &mut b, -1, false), Err(Errno::EINVAL));

        // Writes of at most max_write bytes.
        fmp.max_write.set(4);
        assert_eq!(write(p, h, b"abcdefghij", 0), (Ok(()), 0, 10));
        let sizes: Vec<u32> = take_seen()
            .iter()
            .map(|r| r.op::<FuseWriteIn>().size)
            .collect();
        assert_eq!(sizes, [4, 4, 2]);
        assert_eq!(content(), b"abcdefghij");

        // D-N5.9-U4: a daemon that takes 3 of a 4-byte chunk ends a two-chunk write after
        // one request: Ok, the residual and the offset what it did not take, where the C
        // sends the wrong bytes at offset 3 and runs past the iovecs.
        script(FUSE_WRITE, |r| {
            Answer::out(&FuseWriteOut {
                size: r.op::<FuseWriteIn>().size.min(3),
                ..FuseWriteOut::default()
            })
        });
        let _ = take_seen();
        assert_eq!(write(p, h, b"ABCDEFGH", 0), (Ok(()), 5, 3));
        let seen = take_seen();
        assert_eq!(seen.len(), 1, "one request");
        let w = seen[0].op::<FuseWriteIn>();
        assert_eq!((w.offset, w.size, &seen[0].data[..]), (0, 4, &b"ABCD"[..]));
        assert_eq!(vtoi(h).filesize.get(), 10, "the file is no shorter");

        // More than asked: EINVAL (the C's).
        script(FUSE_WRITE, |_| {
            Answer::out(&FuseWriteOut {
                size: 9,
                ..FuseWriteOut::default()
            })
        });
        assert_eq!(write(p, h, b"xy", 0).0, Err(Errno::EINVAL));

        // D-N5.9-U2: a reply of size 0 is EIO, the chunk given back to the uio, where the C
        // goes round again with the iovec past the chunk.
        script(FUSE_WRITE, |_| Answer::out(&FuseWriteOut::default()));
        let _ = take_seen();
        assert_eq!(write(p, h, b"abcdef", 2), (Err(Errno::EIO), 6, 2));
        assert_eq!(take_ops().len(), 1, "asked once");

        fmp.max_write.set(65536);
        drop(fmp);
        vrele(h);
        done(p, dev, mp, root);
    }

    #[test]
    fn readdir_takes_whole_entries_and_bounds_a_reply_it_cannot_take() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"");
        let long: &'static [u8] = &[b'n'; 250];
        std::thread_local! {
            static REPLY: core::cell::RefCell<Vec<u8>> = const { core::cell::RefCell::new(Vec::new()) };
        }
        let set = |b: Vec<u8>| REPLY.with(|r| *r.borrow_mut() = b);
        script(FUSE_READDIR, |r| {
            let rd = r.op::<FuseReadIn>();
            REPLY.with(|b| {
                let b = b.borrow();
                // the whole reply at offset 0, nothing past it (the end)
                Answer::data(if rd.offset == 0 { &b } else { &[] })
            })
        });
        let size = size_of::<Dirent>();

        // A full buffer ends the call at the last whole entry (the C's `goto out`), the
        // request freed.
        set(dirents(&[(4, b"a", 8), (5, long, 8)], 0, 4096));
        let out = vfs::<TestGlue>().fuse.fbuf_pool.nout();
        let (e, end, eof) = readdir(root, p, 0, size).expect("readdir");
        assert_eq!(e, [(b"a".to_vec(), 8, 4, 1)]);
        assert_eq!((end, eof), (1, 0));
        assert_eq!(vfs::<TestGlue>().fuse.fbuf_pool.nout(), out);

        // D-N5.9-U2: a reply no entry can be taken from: EIO, asked once.
        let mut partial = dirents(&[(5, long, 8)], 0, 4096);
        partial.truncate(100);
        for bad in [vec![0u8; 10], partial] {
            set(bad);
            let _ = take_seen();
            assert_eq!(readdir(root, p, 0, 2 * size).err(), Some(Errno::EIO));
            assert_eq!(
                take_ops(),
                [FUSE_OPENDIR, FUSE_READDIR, FUSE_RELEASEDIR],
                "one READDIR"
            );
        }

        // The names the C refuses: empty, past MAXNAMLEN (a 328-byte reply of a 300-byte
        // name), with a '/'.
        let mut empty = dirents(&[(5, b"e", 8)], 0, 4096);
        empty[16..20].copy_from_slice(&0u32.to_ne_bytes());
        set(empty);
        assert_eq!(readdir(root, p, 0, 2 * size).err(), Some(Errno::EIO));
        let huge: &'static [u8] = &[b'n'; 300];
        let reply = dirents(&[(5, huge, 8)], 0, 4096);
        assert_eq!(reply.len(), 328);
        set(reply);
        assert_eq!(readdir(root, p, 0, 2 * size).err(), Some(Errno::EIO));
        set(dirents(&[(5, b"a/b", 8)], 0, 4096));
        assert_eq!(readdir(root, p, 0, 2 * size).err(), Some(Errno::EIO));

        // A buffer under one dirent: EINVAL before any request.
        let _ = take_seen();
        assert_eq!(readdir(root, p, 0, size - 1).err(), Some(Errno::EINVAL));
        assert!(take_seen().is_empty());
        done(p, dev, mp, root);
    }

    #[test]
    fn the_generic_slots_the_lock_and_the_filters() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"hi");
        let h = hello(p, root);
        let cred = Cred::real(p.ucred());

        let mut data = [0u8; 4];
        assert_eq!(vop_ioctl(h, 0, &mut data, 0, cred, p), Err(Errno::ENOTTY));
        let mut v = 0;
        vop_pathconf(h, _PC_LINK_MAX, &mut v).expect("spec_pathconf");
        assert_eq!(fusefs_strategy(), Ok(()));
        assert_eq!(fusefs_print(h), Ok(()));

        // The lock is the node's recursive rwlock.
        assert_eq!(vop_islocked(h), 0);
        vn_lock(h, LK_EXCLUSIVE).expect("lock");
        assert_ne!(vop_islocked(h), 0);
        let _ = vop_unlock(h);

        // The filters: the vnode's events, the write filter always ready, the read filter's
        // revoke; other filters EINVAL; detach unhooks from the vnode's klist.
        let _ = kqueue_init::<TestGlue>();
        let kq = kqueue_alloc::<TestGlue>(p.process().fd_ref()).expect("kqueue");
        let hold = test_knote(&kq, 1);
        let kn = hold.kn();
        kn.kn_kevent.filter.set(EVFILT_USER);
        assert_eq!(fusefs_kqfilter(h, kn), Err(Errno::EINVAL));
        kn.kn_kevent.filter.set(EVFILT_VNODE);
        kn.kn_sfflags.set(NOTE_WRITE | NOTE_DELETE);
        fusefs_kqfilter(h, kn).expect("kqfilter");
        assert!(ptr::eq(kn_vnode(kn), h));
        assert!(!<FusefsVnode as Filter<TestGlue>>::event(
            kn,
            i64::from(NOTE_RENAME)
        ));
        assert!(<FusefsVnode as Filter<TestGlue>>::event(
            kn,
            i64::from(NOTE_WRITE)
        ));
        assert_eq!(kn.kn_fflags().get(), NOTE_WRITE);
        <FusefsVnode as Filter<TestGlue>>::detach(kn);
        assert!(h.klist.is_empty());
        test_knote_drop(&kq, hold);
        let hold = test_knote(&kq, 2);
        let kn = hold.kn();
        kn.kn_kevent.filter.set(EVFILT_WRITE);
        fusefs_kqfilter(h, kn).expect("kqfilter");
        assert!(<FusefsWrite as Filter<TestGlue>>::event(kn, 0));
        assert!(<FusefsRead as Filter<TestGlue>>::event(
            kn,
            i64::from(NOTE_REVOKE)
        ));
        assert!(kn.has_flags(EV_EOF | EV_ONESHOT));
        <FusefsWrite as Filter<TestGlue>>::detach(kn);
        test_knote_drop(&kq, hold);

        vrele(h);
        done(p, dev, mp, root);
    }

    #[test]
    fn the_last_reference_releases_and_reclaim_forgets() {
        let (_g, p, dev, mp, root) = mounted();
        hello_fs(b"hi");
        let h = hello(p, root);
        script(FUSE_OPEN, |r| {
            Answer::out(&FuseOpenOut {
                fh: 30 + u64::from(r.op::<FuseOpenIn>().flags),
                ..FuseOpenOut::default()
            })
        });
        let cred = Cred::real(p.ucred());
        vop_open(h, FREAD, cred, p).expect("open");
        vop_open(h, FWRITE, cred, p).expect("open");
        let _ = take_seen();
        // inactive: a FUSE_RELEASE per handle with the kind's flags, the handles invalid.
        vrele(h);
        let seen = take_seen();
        let rel: Vec<(u64, u32)> = seen
            .iter()
            .map(|r| {
                let i = r.op::<FuseReleaseIn>();
                (i.fh, i.flags)
            })
            .collect();
        assert_eq!(rel, [(30, O_RDONLY as u32), (31, O_WRONLY as u32)]);
        // reclaim (the unmount's vflush): FUSE_FORGET with the lookup count, the node off
        // the hash and out of the vnode's slot.
        let ip = vtoi(h);
        let nl = ip.nlookup.get();
        drop(ip);
        vput(root);
        fusefs_unmount(mp, 0, p).expect("unmount");
        let seen = take_seen();
        let f = seen
            .iter()
            .find(|r| r.hdr.opcode == FUSE_FORGET)
            .expect("FORGET");
        assert_eq!((f.hdr.nodeid, f.op::<FuseForgetIn>().nlookup), (2, nl));
        assert!(
            h.node_arc::<crate::fs::fuse::fusefs_node::FusefsNode<TestGlue>>()
                .is_none()
        );
        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
        let _ = scripted;
    }
}
/* </TESTS> */
