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
 * Copyright (c) 1989, 1990, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)mfs_vfsops.c	8.4 (Berkeley) 4/16/94
 */
/* </LICENSES> */

/* <CODE> */
//! The memory based file system's file-system-type operations: `mfs_vfsops` ([`Mfs::VFS`]),
//! `mfs_mount` (an FFS in the memory of the mounting process, on a made-up block device
//! vnode), `mfs_start` (the process stays in the kernel serving the file system's I/O),
//! `mfs_checkexp` and `mfs_init`.
//!
//! Upstream: sys/ufs/mfs/mfs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/ufs/mfs/mfs_vfsops.rs@3c62ede68802
//!
//! `mount_mfs(8)` (`newfs -DMFS`) builds the file system in its own memory, forks, and the
//! child calls `mount(2)` with `MOUNT_MFS` and `{base, size}`; `sys_mount` calls `VFS_START`
//! last, so the child sleeps in [`mfs_start`] until the file system is unmounted.
//!
//! ## Deviations
//! - `mfs_minor` is an atomic; `mfs_args` is `hal::sys::mount::MfsArgs`, read from the kernel
//!   copy of the arguments. A new mount whose arguments are missing (the C's NULL `data`)
//!   answers `EINVAL` where the C dereferences it (only an in-kernel caller passes none).
//! - `EXPORTMFS` (the update that only changes the export list) is not configured: an update
//!   with no device name does nothing, as in the C without it.
//! - `mfs_start`'s `p_siglist` update clears the signal's bit only when a signal was taken: a
//!   `cursig` of 0 (woken without one) clears nothing, where the C's `sigmask(0)` is a shift
//!   by -1 (LZ's).
//! - `mfs_checkexp` takes the mount alone: the export check is `bsd`'s (the NFS server's,
//!   `VFS_CHECKEXP`), which calls it for an mfs mount (docs/SCAFFOLDING.md, N6).
//!
//! ## Redesign
//! - LZ's `MFS_VFSOPS` (a `bsd` `Vfsops` of function pointers over `bsd`'s mount) -> `impl
//!   Vfs<G> for Mfs<G>` and its constant [`Mfs::VFS`], which `bsd`'s `vfsconflist[]` names
//!   (`&Mfs::<BsdProcGlue>::VFS`, nothing keys it by address); each slot is the C's function:
//!   this module's `mfs_mount`, `mfs_start` and `mfs_init`, the kernel's ffs's `ffs_unmount`,
//!   `ffs_statfs`, `ffs_sync`, `ffs_vget`, `ffs_fhtovp`, `ffs_vptofh` and `ffs_sysctl`, the
//!   UFS layer's `ufs_root` and `ufs_quotactl`.
//! - `mfs_mount` builds the node whole ([`Mfsnode::new`]: the counted `malloc(M_MFSNODE)`
//!   block) and hangs it in the device vnode's typed slot where the C sets `v_data` (LZ wrote
//!   a raw `malloc` with `ptr::write` and kept a `&'static` to it); the vnode's operations are
//!   the glue's `UfsTables::mfs_vops`. The mount-name copies are the kernel ffs's
//!   (`Superblock::set_fsmnt`, `mname_copy` over libkern's `strlcpy(9)`: LZ's two private
//!   helpers gone).
//! - `mfs_start` holds the node's handle while it serves (the C's local pointer, which the
//!   node's reclaim cannot free under it); the signal's unmount is [`mfs_signalled`], the
//!   C's block of the loop as a function, so it is tested alone.
//! - Cost against the C: `VFSTOUFS` and `VTOMFS` are a count pair each, once per
//!   `mfs_start` and per `mfs_mount`; the loop's sleep and wakeups are the C's (the channel the
//!   device vnode's address, `mfsidl`).

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::malloc::MArc;
use hal::sleep::{INFSLP, PCATCH, PWAIT};
use hal::sys::errno::Errno;
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_RDONLY, MNT_UPDATE, MNT_WANTRDWR, MfsArgs, Statfs, VB_NOWAIT,
    VB_WRITE,
};
use hal::sys::signal::{SIGKILL, sigmask};
use hal::sys::types::{Ino, Uid, makedev};
use hal::user::UserPtr;

use super::mfs_vnops::mfs_serve;
use super::mfsnode::{Mfsnode, vtomfs};
use crate::assert::kpanic;
use crate::fs::mount::{Mount, Vfsconf, vfs_busy};
use crate::fs::subr::{checkalias, getnewvnode, vfs, vrele};
use crate::fs::syscalls::dounmount;
use crate::fs::ufs::ffs::ffs_vfsops::{
    ffs_fhtovp, ffs_flushfiles, ffs_init, ffs_mountfs, ffs_statfs, ffs_sync, ffs_sysctl,
    ffs_unmount, ffs_vget, ffs_vptofh, mname_copy,
};
use crate::fs::ufs::ffs::fs::Fs;
use crate::fs::ufs::ufs::quota::ufs_quotactl;
use crate::fs::ufs::ufs::ufs_vfsops::ufs_root;
use crate::fs::ufs::ufs::ufsmount::vfstoufs;
use crate::fs::vnode::{FORCECLOSE, VBLK, VT_MFS, Vnode, WRITECLOSE};
use crate::fs::vops::{Cred, Nd, Vfs};
use crate::proc::{ProcGlue, Thread};
use crate::sched::tsleep_nsec;
use crate::sig::{Sigctx, cursig};

/// The memory file system of the glue `G`: `mfs_vfsops` ([`Mfs::VFS`]).
pub struct Mfs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Mfs<G> {
    /// `mfs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"mfs"`).
    pub const VFS: Mfs<G> = Mfs(PhantomData);
}

/// `mfs_minor`: used for building internal dev_t.
static MFS_MINOR: AtomicU32 = AtomicU32::new(0);

/// `mfs_mount` (`vfs_mount`): mount system call. `data` is the kernel copy of the user's
/// `struct mfs_args` (empty for the C's NULL).
pub fn mfs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    _ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let args = MfsArgs::from_bytes(data);

    // If updating, check whether changing from read-only to read/write; if there is no device
    // name, that's all we do.
    if mp.flag() & MNT_UPDATE != 0 {
        let ump = vfstoufs(mp);
        let fs = ump.um_fs();
        if fs.fs_ronly() == 0 && mp.flag() & MNT_RDONLY != 0 {
            let mut flags = WRITECLOSE;
            if mp.flag() & MNT_FORCE != 0 {
                flags |= FORCECLOSE;
            }
            ffs_flushfiles(mp, flags, p)?;
        }
        if fs.fs_ronly() != 0 && mp.flag() & MNT_WANTRDWR != 0 {
            fs.set_fs_ronly(0);
        }
        // EXPORTMFS: not configured.
        return Ok(());
    }
    let Some(args) = args else {
        return Err(Errno::EINVAL);
    };
    let mut fspec = [0u8; MNAMELEN];
    UserPtr::<u8>::new(args.fspec).copyinstr(&mut fspec)?;
    let devvp = getnewvnode(VT_MFS, None, &vfs::<G>().ufs.mfs_vops)?;
    devvp.set_vtype(VBLK);
    let minor = MFS_MINOR.load(Ordering::Relaxed);
    if checkalias(devvp, makedev(255, minor), None).is_some() {
        kpanic(format_args!("mfs_mount: dup dev"));
    }
    MFS_MINOR.store(minor.wrapping_add(1), Ordering::Relaxed);
    // `mfs_size` is the C's `long`: the argument's bits, as the C's assignment keeps them.
    let mfsp = Mfsnode::new(devvp, args.base, args.size as i64, p.p_tid.get());
    devvp.set_node(MArc::erase(mfsp.clone()));
    if let Err(e) = ffs_mountfs(devvp, mp, p) {
        mfsp.mfs_shutdown.store(1, Ordering::Relaxed);
        drop(mfsp);
        vrele(devvp);
        return Err(e);
    }
    drop(mfsp);
    let ump = vfstoufs(mp);
    let fs = ump.um_fs();

    fs.set_fsmnt(path);
    mp.with_stat(|sp| {
        fs.read_bytes(offset_of!(Fs, fs_fsmnt), &mut sp.f_mntonname);
        mname_copy(&mut sp.f_mntfromname, &fspec);
        mname_copy(&mut sp.f_mntfromspec, &fspec);
        sp.mount_info.__align[..MfsArgs::SIZE].copy_from_slice(&data[..MfsArgs::SIZE]);
    });

    Ok(())
}

/// `mfs_start` (`vfs_start`): used to grab the process and keep it in the kernel to service
/// memory filesystem I/O requests.
///
/// Loop servicing I/O requests. Copy the requested data into or out of the memory filesystem
/// address space.
pub fn mfs_start<G: ProcGlue>(
    mp: &'static Mount<G>,
    _flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let vp = vfstoufs(mp).devvp();
    let mfsp = vtomfs(vp);
    let mut sleepreturn: Result<(), Errno> = Ok(());

    loop {
        mfs_serve(&mfsp, || mfsp.shutdown());
        if mfsp.shutdown() {
            break;
        }

        // If a non-ignored signal is received, try to unmount. If that fails, clear the signal
        // (it has been "processed"), otherwise we will loop here, as tsleep will always return
        // EINTR/ERESTART.
        if sleepreturn.is_err() {
            mfs_signalled(mp, p);
            sleepreturn = Ok(());
            continue;
        }
        sleepreturn = tsleep_nsec::<G>(
            ptr::from_ref::<Vnode<G>>(vp).addr(),
            PWAIT | PCATCH,
            "mfsidl",
            INFSLP,
        );
    }
    Ok(())
}

/// `mfs_start`'s answer to a signal that woke it: the unmount (forced for `SIGKILL`), or,
/// when the mount is busy or the unmount fails, the signal cleared.
pub fn mfs_signalled<G: ProcGlue>(mp: &'static Mount<G>, p: &Thread<G>) {
    let mut ctx = Sigctx::default();
    let sig = cursig(p, &mut ctx, false);
    let failed = vfs_busy(mp, VB_WRITE | VB_NOWAIT).is_err()
        || dounmount(mp, if sig == SIGKILL { MNT_FORCE } else { 0 }, p).is_err();
    if failed && sig != 0 {
        p.p_siglist.fetch_and(!sigmask(sig), Ordering::Relaxed);
    }
}

/// `mfs_checkexp` (`vfs_checkexp`): check export permission, not supported.
pub fn mfs_checkexp<G: ProcGlue>(_mp: &Mount<G>) -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// `mfs_init` (`vfs_init`): memory based filesystem initialization.
pub fn mfs_init<G: ProcGlue>(vfsp: &'static Vfsconf<G>) -> Result<(), Errno> {
    ffs_init(vfsp)
}

/// `mfs_vfsops`: each slot the C's function of the same name.
impl<G: ProcGlue> Vfs<G> for Mfs<G> {
    /// `vfs_mount`: [`mfs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        mfs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: [`mfs_start`].
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        mfs_start(mp, flags, p)
    }

    /// `vfs_unmount`: `ffs_unmount`.
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        ffs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: `ufs_root`.
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        ufs_root(mp)
    }

    /// `vfs_quotactl`: `ufs_quotactl`.
    fn quotactl(
        &self,
        mp: &'static Mount<G>,
        cmds: i32,
        uid: Uid,
        arg: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ufs_quotactl(mp, cmds, uid, arg, p)
    }

    /// `vfs_statfs`: `ffs_statfs`.
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        ffs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: `ffs_sync`.
    fn sync(
        &self,
        mp: &'static Mount<G>,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ffs_sync(mp, waitfor, stall, cred, p)
    }

    /// `vfs_vget`: `ffs_vget`.
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        ffs_vget(mp, ino)
    }

    /// `vfs_fhtovp`: `ffs_fhtovp`.
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        ffs_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: `ffs_vptofh`.
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        ffs_vptofh(vp, fid)
    }

    /// `vfs_init`: [`mfs_init`].
    fn init(&self, vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        mfs_init(vfc)
    }

    /// `vfs_sysctl`: `ffs_sysctl`.
    fn sysctl(
        &self,
        name: &[i32],
        oldp: usize,
        oldlenp: &mut usize,
        newp: usize,
        newlen: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ffs_sysctl(name, oldp, oldlenp, newp, newlen, p)
    }
}

/// `mfs_minor` back to 0 (a host test's fresh memory: the device vnodes an earlier test made
/// are forgotten with the vnode lists, `fs::subr::vfs_test_reset`).
#[cfg(any(test, feature = "doubles"))]
pub fn mfs_test_reset() {
    MFS_MINOR.store(0, Ordering::Relaxed);
}

/// The host tests' fixture (`mfs_vfsops.rs` and `mfs_vnops.rs`): a test thread over fresh VFS
/// tables with page memory and a buffer cache, an FFS image in the user memory of the host's
/// copy double (the memory of the "process" that mounts it), and the mount as `sys_mount`
/// makes it.
#[cfg(test)]
pub(super) mod fixture {
    extern crate std;

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::sys::errno::Errno;
    use hal::sys::fcntl::{O_CREAT, O_RDONLY, O_RDWR};
    use hal::sys::mount::{MNT_LOCAL, MOUNT_MFS, MfsArgs, VB_WAIT, VB_WRITE};
    use hal::user::UserPtr;

    use super::{Mfs, mfs_init, mfs_mount};
    use crate::descrip::{sys_close, sys_read, sys_write};
    use crate::fs::init::{rootvnode, set_rootvnode};
    use crate::fs::lookup::ndinit;
    use crate::fs::mount::{
        Mount, Vfsconf, mountlist_insert_tail, vfs_busy, vfs_mount_alloc, vfs_unbusy,
    };
    use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
    use crate::fs::subr::{vref, vrele};
    use crate::fs::syscalls::{dounmount, sys_fsync, sys_open};
    use crate::fs::testfs::{TestGlue, test_thread_cached};
    use crate::fs::vops::{VFS_ROOT, VFS_STATFS, vop_unlock};
    use crate::proc::Thread;
    use crate::testutil::{USER_BASE, USER_LEN, user_bytes};

    /// The fixture's configuration entry: mfs as `vfsconflist[]` has it (`"mfs"`, type 3,
    /// `struct mfs_args`).
    pub static TEST_MFS_CONF: Vfsconf<TestGlue> = Vfsconf::new(
        &Mfs::<TestGlue>::VFS,
        MOUNT_MFS,
        3,
        MNT_LOCAL,
        MfsArgs::SIZE,
    );

    /// Where the image lies in the user memory (the 64 KiB below hold the tests' arguments).
    pub const UIMAGE: usize = USER_BASE + 0x1_0000;
    /// The name `newfs -DMFS` hands the kernel as `fspec` (`mfs:<pid>`), in user memory.
    pub const UFSPEC: usize = USER_BASE + 0xa000;
    /// The path slots: four of 256 bytes.
    const UPATH: usize = USER_BASE + 0xb000;
    /// The buffer, to USER_BASE + 0xd000.
    const UBUF: usize = USER_BASE + 0xb400;
    /// The buffer's length.
    const UBUF_LEN: usize = 0xd000 - 0xb400;

    /// The fspec's text.
    pub const FSPEC: &[u8] = b"mfs:1234";

    /// A test thread with its `uiomove` copying kernel iovecs, `mfs_init` run (ffs's pools,
    /// the UFS layer), `image` in the user memory at [`UIMAGE`] and the fspec at [`UFSPEC`].
    pub fn attach(image: &[u8]) -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
        let (g, p) = test_thread_cached(1 << 22, 192);
        crate::testutil::set_copy_kernel(true);
        if let Err(e) = mfs_init(&TEST_MFS_CONF) {
            std::panic!("mfs fixture: mfs_init: {e:?}");
        }
        // The image, and zeros to the end of the user memory: an earlier test's image there
        // is gone (the double's memory is the process's, not the test's).
        let mut mem = vec![0u8; USER_BASE + USER_LEN - UIMAGE];
        mem[..image.len()].copy_from_slice(image);
        UserPtr::<u8>::new(UIMAGE).copyout_slice(&mem).unwrap();
        let mut s = FSPEC.to_vec();
        s.push(0);
        UserPtr::<u8>::new(UFSPEC).copyout_slice(&s).unwrap();
        (g, p)
    }

    /// The image as the user memory holds it now.
    pub fn image(len: usize) -> Vec<u8> {
        user_bytes(UIMAGE, len).unwrap()
    }

    /// `struct mfs_args` as `sys_mount` copies it in: `fspec`, the (zeroed) export arguments,
    /// `base` and `size`.
    pub fn mfs_args(fspec: usize, base: usize, size: usize) -> [u8; MfsArgs::SIZE] {
        let mut b = [0u8; MfsArgs::SIZE];
        b[..8].copy_from_slice(&fspec.to_ne_bytes());
        b[MfsArgs::SIZE - 16..MfsArgs::SIZE - 8].copy_from_slice(&base.to_ne_bytes());
        b[MfsArgs::SIZE - 8..].copy_from_slice(&(size as u64).to_ne_bytes());
        b
    }

    /// A new mount of mfs, busy, as `sys_mount` allocates it before `VFS_MOUNT`.
    pub fn new_mount(flags: i32) -> &'static Mount<TestGlue> {
        let mp = vfs_mount_alloc(None, &TEST_MFS_CONF);
        mp.set_flag(mp.flag() | flags);
        mp
    }

    /// `mfs_mount` of the image (`len` bytes at [`UIMAGE`]) on a new mount: `Ok` with the mount
    /// unbusied and its statistics read (as `sys_mount` leaves it), or the error with the
    /// mount freed.
    pub fn mount(
        p: &'static Thread<TestGlue>,
        len: usize,
        flags: i32,
    ) -> Result<&'static Mount<TestGlue>, Errno> {
        let mp = new_mount(flags);
        let mut args = mfs_args(UFSPEC, UIMAGE, len);
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/"), p);
        let r = mfs_mount(mp, b"/mfs\0", &mut args, &mut nd, p);
        if r.is_ok() {
            let mut st = mp.stat();
            let _ = VFS_STATFS(mp, &mut st, p);
            mp.with_stat(|sp| *sp = st);
        }
        vfs_unbusy(mp);
        match r {
            Ok(()) => Ok(mp),
            Err(e) => {
                crate::fs::mount::vfs_mount_free(mp);
                Err(e)
            }
        }
    }

    /// [`mount`] at `/`: the mount list, the root vnode and the thread's current directory.
    pub fn mount_root(
        p: &'static Thread<TestGlue>,
        len: usize,
        flags: i32,
    ) -> &'static Mount<TestGlue> {
        let mp = mount(p, len, flags).unwrap_or_else(|e| std::panic!("mfs_mount: {e:?}"));
        mountlist_insert_tail(mp);
        let vp = VFS_ROOT(mp).unwrap();
        if let Some(old) = rootvnode::<TestGlue>() {
            vrele(old);
        }
        set_rootvnode(Some(vp));
        vref(vp);
        if let Some(old) = p.fd().replace_cdir(Some(vp)) {
            vrele(old);
        }
        let _ = vop_unlock(vp);
        mp
    }

    /// [`mount_root`] undone through `dounmount` (`unmount(2)`'s path).
    pub fn unmount_root(p: &'static Thread<TestGlue>, mp: &'static Mount<TestGlue>) {
        if let Some(cdir) = p.fd().replace_cdir(None) {
            vrele(cdir);
        }
        if let Some(vp) = rootvnode::<TestGlue>() {
            set_rootvnode::<TestGlue>(None);
            vrele(vp);
        }
        vfs_busy(mp, VB_WRITE | VB_WAIT).unwrap();
        dounmount(mp, 0, p).unwrap();
    }

    /// The NUL-terminated `s` in user memory, in slot `i`.
    pub fn up(i: usize, s: &[u8]) -> UserPtr<u8> {
        assert!(s.len() < 256 && i < 4);
        let at = UPATH + i * 256;
        let mut b = s.to_vec();
        b.push(0);
        UserPtr::<u8>::new(at).copyout_slice(&b).unwrap();
        UserPtr::new(at)
    }

    /// The whole contents of the file at `name`.
    pub fn read_file(p: &Thread<TestGlue>, name: &[u8]) -> Result<Vec<u8>, Errno> {
        let fd = sys_open(p, up(0, name), O_RDONLY, 0)?;
        let mut out = Vec::new();
        loop {
            let n = sys_read(p, fd, UBUF, 3000)?;
            if n == 0 {
                break;
            }
            let mut b = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
            out.extend_from_slice(&b);
        }
        sys_close(p, fd)?;
        Ok(out)
    }

    /// Creates `name` with `data` and `fsync`s it.
    pub fn write_file(p: &Thread<TestGlue>, name: &[u8], data: &[u8]) -> Result<(), Errno> {
        let fd = sys_open(p, up(0, name), O_RDWR | O_CREAT, 0o644)?;
        for chunk in data.chunks(5001.min(UBUF_LEN)) {
            UserPtr::<u8>::new(UBUF).copyout_slice(chunk).unwrap();
            let n = sys_write(p, fd, UBUF, chunk.len())?;
            assert_eq!(n, chunk.len());
        }
        sys_fsync(p, fd)?;
        sys_close(p, fd)?;
        Ok(())
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the memory file system: an FFS image built by the kernel ffs's `newfs`,
    // laid in the user memory of the host's copy double (the memory of the "process" that
    // mounts it), mounted by `mfs_mount` (the test thread is the file system's process, so its
    // I/O is done at once), read and written through the system calls, unmounted clean.
    // `mfs_start` sleeps until the unmount, which one host thread cannot do: it is tested for
    // the shutdown it answers at once and for the signal's unmount (`mfs_signalled`); the
    // smoke runs the rest end to end (`mount_mfs -s 8m swap /mfs`, smoke-ufsopts).

    extern crate std;

    use std::vec::Vec;

    use hal::sys::mount::MNT_RDONLY;
    use hal::sys::signal::SIGTERM;

    use super::fixture::{
        FSPEC, TEST_MFS_CONF, UFSPEC, UIMAGE, attach, image, mfs_args, mount, mount_root,
        new_mount, read_file, unmount_root, write_file,
    };
    use super::*;
    use crate::fs::lookup::ndinit;
    use crate::fs::mount::{vfs_mount_free, vfs_unbusy};
    use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
    use crate::fs::subr::vgone;
    use crate::fs::syscalls::sys_sync;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::fs::FS_UFS1_MAGIC;
    use crate::fs::ufs::ffs::testffs::newfs::{FFS1_4M, Image, Params, check};
    use crate::fs::ufs::mfs::mfsnode::{M_MFSNODE, mfstov};
    use crate::testutil::malloc_out;
    use hal::sys::types::major;

    /// FFS1, 128 KB in one cylinder group: what fits the user memory of the copy double (the
    /// super-block search reads 64 KiB in first, so an image needs 72 KiB at least).
    const FFS1_128K: Params = Params {
        size: 128 << 10,
        fpg: 256,
        ipg: 64,
        ..FFS1_4M
    };

    fn pattern(n: usize, seed: u8) -> Vec<u8> {
        (0..n)
            .map(|i| (i as u8).wrapping_mul(29).wrapping_add(seed) ^ (i >> 9) as u8)
            .collect()
    }

    fn nd(p: &Thread<TestGlue>) -> Nd<'_, TestGlue> {
        ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/"), p)
    }

    #[test]
    fn mfs_args_are_read_from_the_kernel_copy() {
        let a = mfs_args(0x1000, 0x7000_0000, 8 << 20);
        let args = MfsArgs::from_bytes(&a).unwrap();
        assert_eq!(args.fspec, 0x1000);
        assert_eq!(args.base, 0x7000_0000);
        assert_eq!(args.size, 8 << 20);
        assert_eq!(args.export_info.ex_flags, 0);
        assert!(MfsArgs::from_bytes(&a[..MfsArgs::SIZE - 1]).is_none());
        assert!(MfsArgs::from_bytes(&[]).is_none());
        assert_eq!(TEST_MFS_CONF.vfc_datasize, MfsArgs::SIZE);
    }

    #[test]
    fn mfs_mount_read_write_and_unmount() {
        let mut img = Image::new(FFS1_128K);
        img.add_file(b"motd", b"hello, memory\n");
        let big = pattern(30000, 3);
        img.add_file(b"big", &big);
        let len = FFS1_128K.size;
        let (_g, p) = attach(&img.finish());
        let nodes = malloc_out(M_MFSNODE);

        let mp = mount_root(p, len, 0);
        let ump = vfstoufs(mp);
        let fs = ump.um_fs();
        assert_eq!(fs.fs_magic(), FS_UFS1_MAGIC);
        assert_eq!(fs.fsmnt().as_bytes(), b"/mfs");
        assert_eq!(fs.fs_ronly(), 0);

        // The "device" is a made-up block device vnode of the mfs type, and the mount knows
        // what it was mounted from, on, and with.
        let devvp = ump.devvp();
        drop(ump);
        assert_eq!(devvp.tag(), VT_MFS);
        assert_eq!(devvp.vtype(), VBLK);
        assert_eq!(major(devvp.rdev()), 255);
        let mfsp = vtomfs(devvp);
        assert_eq!(mfsp.mfs_size, len as i64);
        assert_eq!(mfsp.mfs_baseoff, UIMAGE);
        assert_eq!(mfsp.mfs_tid, p.p_tid.get());
        assert!(core::ptr::eq(mfstov(&mfsp), devvp));
        drop(mfsp);
        assert!(
            malloc_out(M_MFSNODE) > nodes,
            "the node is malloc(M_MFSNODE)'s"
        );
        let st = mp.stat();
        assert_eq!(&st.f_mntonname[..5], b"/mfs\0");
        assert_eq!(&st.f_mntfromname[..9], b"mfs:1234\0");
        assert_eq!(&st.f_mntfromspec[..9], b"mfs:1234\0");
        assert_eq!(&st.f_fstypename[..4], b"mfs\0");
        assert_eq!(st.f_fsid.val[0], devvp.rdev());
        assert_eq!(&st.mount_info.__align[..8], &UFSPEC.to_ne_bytes());
        assert_eq!(
            &st.mount_info.__align[MfsArgs::SIZE - 16..MfsArgs::SIZE - 8],
            &UIMAGE.to_ne_bytes()
        );

        // What newfs left in the memory reads back; what is written lands in it.
        assert_eq!(read_file(p, b"/motd").unwrap(), b"hello, memory\n");
        assert_eq!(read_file(p, b"/big").unwrap(), big);
        let note = pattern(20000, 9);
        write_file(p, b"/note", &note).unwrap();
        assert_eq!(read_file(p, b"/note").unwrap(), note);
        sys_sync(p).unwrap();

        // mfs_start answers a shutdown at once.
        let mfsp = vtomfs(devvp);
        mfsp.mfs_shutdown.store(1, Ordering::Relaxed);
        mfs_start(mp, 0, p).unwrap();
        mfsp.mfs_shutdown.store(0, Ordering::Relaxed);

        // Unmount: the close of the device tells the process to exit, the image is clean.
        unmount_root(p, mp);
        assert!(mfsp.shutdown());
        drop(mfsp);
        let disk = image(len);
        check(&disk, false);
        assert_eq!(
            crate::fs::ufs::ffs::testffs::newfs::super_block(&disk, false).fs_clean,
            1
        );

        // Recycling the device vnode frees the mfsnode (`mfs_reclaim`).
        vgone(devvp);
        assert!(!devvp.has_node());
        assert_eq!(malloc_out(M_MFSNODE), nodes, "the node's block was freed");

        // The same memory mounts again (a new "process"): the file written is still there.
        let mp = mount_root(p, len, MNT_RDONLY);
        assert_eq!(read_file(p, b"/note").unwrap(), note);
        assert_eq!(read_file(p, b"/big").unwrap(), big);
        assert_eq!(vfstoufs(mp).um_fs().fs_ronly(), 1);
        unmount_root(p, mp);
    }

    #[test]
    fn mfs_mount_failures_release_the_device() {
        let (_g, p) = attach(&[0u8; 4096]);

        // No arguments.
        let mp = new_mount(0);
        assert_eq!(
            mfs_mount(mp, b"/\0", &mut [], &mut nd(p), p),
            Err(Errno::EINVAL)
        );

        // A name that cannot be copied in.
        let mut args = mfs_args(0, UIMAGE, 1 << 17);
        assert_eq!(
            mfs_mount(mp, b"/\0", &mut args, &mut nd(p), p),
            Err(Errno::EFAULT)
        );

        // Memory with no file system in it: ffs_mountfs fails, the mfsnode is told to shut
        // down, the device vnode is released and the mount holds no UFS structure. The C's
        // answer is EFAULT: the search's last slot (256 KiB in) lies past the end, whose read
        // faults (an image of the copy double cannot reach the 264 KiB of an EINVAL).
        let minor = MFS_MINOR.load(Ordering::Relaxed);
        let mut args = mfs_args(UFSPEC, UIMAGE, 1 << 17);
        assert_eq!(
            mfs_mount(mp, b"/\0", &mut args, &mut nd(p), p),
            Err(Errno::EFAULT)
        );
        assert_eq!(MFS_MINOR.load(Ordering::Relaxed), minor + 1);
        assert!(!mp.has_node(), "no UFS structure left in the mount's slot");

        // Memory that ends before the first slot of the super-block search (64 KiB in): the
        // read past the end fails, as the C's copyin of that length does.
        let mut args = mfs_args(UFSPEC, UIMAGE, 32 << 10);
        assert_eq!(
            mfs_mount(mp, b"/\0", &mut args, &mut nd(p), p),
            Err(Errno::EFAULT)
        );
        vfs_unbusy(mp);
        vfs_mount_free(mp);
    }

    #[test]
    fn a_file_system_that_ends_inside_the_first_slot_mounts_from_the_second() {
        // 68 KiB: the read 64 KiB in returns 4 KiB, short of a super-block; FFS1's at 8 KiB is
        // found next.
        let params = Params {
            size: 68 << 10,
            fpg: 136,
            ipg: 32,
            ..FFS1_4M
        };
        let mut img = Image::new(params);
        img.add_file(b"f", b"small\n");
        let (_g, p) = attach(&img.finish());
        let mp = mount_root(p, params.size, 0);
        assert_eq!(read_file(p, b"/f").unwrap(), b"small\n");
        unmount_root(p, mp);
        check(&image(params.size), false);
    }

    #[test]
    fn mfs_update_flushes_and_reenables_writes() {
        let (_g, p) = attach(&Image::new(FFS1_128K).finish());
        let mp = mount_root(p, FFS1_128K.size, 0);
        let ump = vfstoufs(mp);
        let fs = ump.um_fs();

        // An update to read-only flushes the files open for writing; the C leaves `fs_ronly`
        // to the generic code, and so does this.
        mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RDONLY);
        mfs_mount(mp, b"/\0", &mut [], &mut nd(p), p).unwrap();
        assert_eq!(fs.fs_ronly(), 0);

        // A read-only file system asked to be read-write.
        fs.set_fs_ronly(1);
        mp.set_flag((mp.flag() & !MNT_RDONLY) | MNT_UPDATE | MNT_WANTRDWR);
        mfs_mount(mp, b"/\0", &mut [], &mut nd(p), p).unwrap();
        assert_eq!(fs.fs_ronly(), 0);

        mp.set_flag(mp.flag() & !(MNT_UPDATE | MNT_WANTRDWR));
        drop(ump);
        unmount_root(p, mp);
    }

    #[test]
    fn a_signal_unmounts_the_file_system_or_is_cleared() {
        let (_g, p) = attach(&Image::new(FFS1_128K).finish());
        crate::sig::test_sigacts(p.process());
        let mp = mount(p, FFS1_128K.size, 0).unwrap();
        crate::fs::mount::mountlist_insert_tail(mp);
        let devvp = vfstoufs(mp).devvp();

        // The mount is busy (a reader in it): the unmount cannot start, the signal is
        // "processed".
        vfs_busy(mp, hal::sys::mount::VB_READ | hal::sys::mount::VB_WAIT).unwrap();
        p.p_siglist.fetch_or(sigmask(SIGTERM), Ordering::Relaxed);
        mfs_signalled(mp, p);
        assert_eq!(p.p_siglist.load(Ordering::Relaxed) & sigmask(SIGTERM), 0);
        assert!(mp.has_node(), "still mounted");
        vfs_unbusy(mp);

        // Free: the signal unmounts it, and the close of the device shuts the server down.
        let mfsp = vtomfs(devvp);
        p.p_siglist.fetch_or(sigmask(SIGTERM), Ordering::Relaxed);
        mfs_signalled(mp, p);
        assert!(!mp.has_node(), "unmounted");
        assert!(mfsp.shutdown());
    }

    #[test]
    fn exports_are_not_supported() {
        let (_g, _p) = attach(FSPEC);
        let mp = new_mount(0);
        assert_eq!(mfs_checkexp(mp), Err(Errno::EOPNOTSUPP), "no EXPORTMFS");
        vfs_unbusy(mp);
        vfs_mount_free(mp);
    }
}
/* </TESTS> */
