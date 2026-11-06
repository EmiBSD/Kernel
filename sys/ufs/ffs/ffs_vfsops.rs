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
 * Copyright (c) 1989, 1991, 1993, 1994
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
 *	@(#)ffs_vfsops.c	8.14 (Berkeley) 11/28/94
 */
/* </LICENSES> */

/* <CODE> */
//! The fast file system's file-system-type operations are the kernel's
//! (`kernel::fs::ufs::ffs::ffs_vfsops`, N5.5b2): `vfsconflist[]` names its `Ffs::VFS`. This
//! module keeps `ffs_mountroot` (`subr_disk.c`'s `setroot` calls it) over the kernel's, with
//! `bsd`'s `rootdev` and the machine's `swapdev`, and in its tests the fixture `bsd`'s nfs,
//! quota and dirhash tests mount an image with (mfs's are the kernel's since N5.5b3).
//!
//! Upstream: sys/ufs/ffs/ffs_vfsops.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `rootdev` is `sys/systm.rs`'s `ROOTDEV` and `swapdev` the machine's (`conf.c`) until N7
//!   (docs/SCAFFOLDING.md); `rootvp` and `swapdev_vp` stay `init_main`'s statics, which the
//!   kernel's `ffs_mountroot` reaches through `FileGlue::rootvp`, `set_rootvp` and
//!   `set_swapdev_vp` (`kern_descrip.rs`'s `BsdFileGlue`, docs/SCAFFOLDING.md, N7).

use core::sync::atomic::Ordering;

use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::conf::swapdev;
use crate::sys::errno::Errno;
use crate::sys::systm::ROOTDEV;

/// `ffs_mountroot`: called by `main()` when ufs is going to be mounted as root: the kernel's,
/// over `rootdev` and `swapdev`.
pub fn ffs_mountroot() -> Result<(), Errno> {
    kernel::fs::ufs::ffs::ffs_vfsops::ffs_mountroot::<BsdProcGlue>(
        ROOTDEV.load(Ordering::Relaxed),
        swapdev(),
    )
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // The fixture of `bsd`'s tests that mount an FFS image (nfs's, quota's, dirhash's):
    // `newfs`, the kernel's image builder, a block device vnode whose strategy reads and writes
    // the image, and the root mount through the kernel's `ffs_mountfs`; with the tests of
    // `ffs_mount`'s export update (the exports are `bsd`'s until N6), of the kernel msdosfs's
    // and cd9660's (`VFS_CHECKEXP`'s arms), and of the CD label `subr_disk.rs`'s shell spoofs
    // over the kernel's parser (N5.8c2). The image tests of the
    // file system itself are the kernel's (`kernel::fs::ufs::ffs::ffs_vfsops`'s tests, over
    // its test glue), and so are decision 43's ffs pairs since N5.5b3 (`benches/fs.rs`'s
    // `ffs.*` rows, which replace this module's `ufs_bench`).

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use crate::kern::kern_descrip::sys_close;
    use crate::kern::subr_xxx::nullop;
    use crate::kern::sys_generic::sys_read;
    use crate::kern::vfs_init::{set_rootvnode, vfs_byname};
    use crate::kern::vfs_syscalls::{dounmount, sys_open};
    use crate::kern::vfs_vops::{VOP_UNLOCK, vop_generic_bwrite};
    use crate::machine::Machine;
    use crate::machine::cpu::Cpu;
    use crate::machine::intr::{splbio, splx};
    use crate::sys::buf::{B_ERROR, B_READ, BufLegacy, biodone, bufinit};
    use crate::sys::errno::Errno;
    use crate::sys::fcntl::O_RDONLY;
    use crate::sys::mount::{MNT_RDONLY, MNT_WAIT, Mount, VB_WAIT, VB_WRITE, VFS_ROOT};
    use crate::sys::param::DEV_BSIZE;
    use crate::sys::proc::Proc;
    use crate::sys::systm::{SyCall, SysArgs};
    use crate::sys::types::{Register, makedev};
    use crate::sys::vnode::{
        Vnode, VopFsyncArgs, VopInactiveArgs, VopStrategyArgs, Vops, bdevvp, vflushbuf, vfs_busy,
        vfs_mount_alloc, vfs_unbusy, vref, vrele,
    };
    use kernel::fs::ufs::ffs::ffs_vfsops::{ffs_mountfs, ffs_statfs};

    /// `newfs`: the kernel's FFS image builder (`kernel::fs::ufs::ffs::testffs::newfs`).
    pub(crate) mod newfs {
        pub(crate) use kernel::fs::ufs::ffs::testffs::newfs::*;
    }

    /// The disk the strategy below reads and writes.
    pub(crate) static DISK: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

    /// `diskvn`'s strategy: [`transfer`].
    fn disk_strategy(ap: &mut VopStrategyArgs) -> Result<(), Errno> {
        transfer(ap.a_bp);
        Ok(())
    }

    /// The disk's `d_strategy`: a synchronous transfer between the buffer and the image at
    /// `b_blkno`, then `biodone`.
    fn transfer(bp: &'static crate::sys::buf::Buf) {
        let off = bp.blkno() as usize * DEV_BSIZE;
        let len = bp.bcount() as usize;
        {
            let mut d = DISK.lock().unwrap_or_else(|e| e.into_inner());
            if off + len > d.len() {
                bp.set_error(Some(Errno::EIO));
                bp.set(B_ERROR);
            } else {
                // SAFETY: the buffer is busy for this transfer and mapped.
                let data = unsafe { bp.data() };
                if bp.isset(B_READ) {
                    data.copy_from_slice(&d[off..off + len]);
                } else {
                    d[off..off + len].copy_from_slice(data);
                }
                bp.set_resid(0);
            }
        }
        let s = splbio();
        biodone(bp);
        splx(s);
    }

    /// The fake disk's fsync: `vflushbuf`, as `spec_fsync` does.
    fn disk_fsync(ap: &mut VopFsyncArgs<'_>) -> Result<(), Errno> {
        vflushbuf(ap.a_vp, ap.a_waitfor == MNT_WAIT);
        Ok(())
    }

    fn disk_inactive(ap: &mut VopInactiveArgs<'_>) -> Result<(), Errno> {
        VOP_UNLOCK(ap.a_vp)
    }

    /// The operations of the fake disk's block device vnode: the device switch's open, close
    /// and strategy over `DISK`.
    static DISK_VOPS: Vops = Vops {
        vop_open: Some(|_| nullop()),
        vop_close: Some(|_| nullop()),
        vop_lock: Some(|_| nullop()),
        vop_unlock: Some(|_| nullop()),
        vop_islocked: Some(|_| 0),
        vop_inactive: Some(disk_inactive),
        vop_reclaim: Some(|_| nullop()),
        vop_strategy: Some(disk_strategy),
        vop_bwrite: Some(vop_generic_bwrite),
        vop_fsync: Some(disk_fsync),
        ..Vops::EMPTY
    };

    /// The device number of the fake disk (`rd0a`-like).
    const DISKDEV: i32 = makedev(17, 0);

    /// Memory, the vfs and a fresh buffer cache, the image as the disk, and the thread as
    /// `curproc`.
    pub(crate) fn setup(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Proc) {
        let (g, p) = crate::kern::vfs_subr::tests::setup();
        Machine::set_curproc(Machine::curcpu(), Some(p));
        // No resource limits (write(2) checks RLIMIT_FSIZE).
        crate::kern::proc_glue::test_set_limits(p, 128);

        // A fresh buffer cache over this test's memory.
        kernel::fs::bio::bio_test_reset();
        kernel::fs::bufq::bufq_test_reset();
        bufinit();

        *DISK.lock().unwrap_or_else(|e| e.into_inner()) = image;
        (g, p)
    }

    pub(crate) fn teardown() {
        Machine::set_curproc(Machine::curcpu(), None);
    }

    /// The fake disk's block device vnode, referenced.
    fn diskvp() -> &'static Vnode {
        let vp = bdevvp(DISKDEV).unwrap().unwrap();
        crate::sys::vnode::set_vops(vp, &DISK_VOPS);
        vp
    }

    /// Mounts the disk at `/` the way `ffs_mount` and `main` do: `ffs_mountfs`, the mount list,
    /// the root vnode and the thread's current directory.
    pub(crate) fn mount_root(p: &'static Proc, ronly: bool) -> &'static Mount {
        let devvp = diskvp();
        let mp = vfs_mount_alloc(None, vfs_byname(b"ffs").unwrap());
        if ronly {
            mp.set_flag(mp.flag() | MNT_RDONLY);
        }
        mp.with_stat(|sp| sp.f_mntonname[0] = b'/');
        ffs_mountfs(devvp, mp, p).unwrap();
        let mut st = mp.stat();
        ffs_statfs(mp, &mut st, p).unwrap();
        mp.with_stat(|sp| *sp = st);
        vfs_unbusy(mp);
        crate::sys::mount::mountlist_insert_tail(mp);
        let root = VFS_ROOT(mp).unwrap();
        set_rootvnode(Some(root));
        p.fd().set_cdir(Some(root));
        vref(root);
        let _ = VOP_UNLOCK(root);
        mp
    }

    /// Undoes `mount_root` and unmounts.
    pub(crate) fn unmount_root(p: &'static Proc, mp: &'static Mount) {
        if let Some(cdir) = p.fd().replace_cdir(None) {
            vrele(cdir);
        }
        if let Some(root) = crate::kern::vfs_init::rootvnode() {
            set_rootvnode(None);
            vrele(root);
        }
        vfs_busy(mp, VB_WRITE | VB_WAIT).unwrap();
        dounmount(mp, 0, p).unwrap();
    }

    /// A system call with up to six arguments; `retval[0]`.
    pub(crate) fn sys(f: SyCall, p: &Proc, args: &[usize]) -> Result<isize, Errno> {
        let mut v: SysArgs = [0; 6];
        for (slot, a) in v.iter_mut().zip(args) {
            *slot = *a as Register;
        }
        let mut rv = [0; 2];
        f(p, &v, &mut rv)?;
        Ok(rv[0])
    }

    /// A NUL-terminated path as a "user" address (the host's copyin reads it directly).
    pub(crate) fn path(s: &'static [u8]) -> usize {
        assert_eq!(s.last(), Some(&0));
        crate::kern::vfs_subr::tests::user_str(s).addr()
    }

    /// The whole contents of the file at `name`.
    pub(crate) fn read_file(p: &Proc, name: &'static [u8]) -> Result<Vec<u8>, Errno> {
        let fd = sys(sys_open, p, &[path(name), O_RDONLY as usize, 0])?;
        let mut out = Vec::new();
        let mut buf = vec![0u8; 3000];
        loop {
            let n = sys(
                sys_read,
                p,
                &[fd as usize, buf.as_mut_ptr() as usize, buf.len()],
            )?;
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        sys(sys_close, p, &[fd as usize])?;
        Ok(out)
    }

    /// `mount -u` with `ufs_args` whose `fspec` is NULL and whose `export_info` names a network, as
    /// `mountd(8)` does it: `mount(2)` reaches `ffs_mount`, which hands the export list to
    /// `vfs_export`; `ufs_check_export` then answers the flags and the anonymous credentials for a
    /// client's address, and refuses one outside the list.
    #[cfg(feature = "nfsserver")]
    #[test]
    fn mount_update_exports_the_file_system() {
        use crate::kern::uipc_mbuf::tests::mbinit_again;
        use crate::kern::vfs_subr::tests::exports::{args, check_export, sin};
        use crate::kern::vfs_syscalls::sys_mount;
        use crate::sys::mount::{MNT_DELEXPORT, MNT_EXPORTED, MNT_EXRDONLY, MNT_UPDATE, UfsArgs};

        let img = newfs::Image::new(newfs::FFS2_4M);
        let (_g, p) = setup(img.finish());
        mbinit_again();
        let mp = mount_root(p, false);

        let net = sin(2, [10, 0, 2, 0]);
        let mask = sin(2, [255, 255, 255, 0]);
        let mut args = UfsArgs {
            fspec: 0,
            export_info: args(MNT_EXPORTED | MNT_EXRDONLY, 32767, Some(net), Some(mask)),
        };
        let check = |a: [u8; 4]| check_export(mp, a);

        // Not exported yet: every client is refused.
        assert_eq!(check([10, 0, 2, 9]), Err(Errno::EACCES));

        // The type and the arguments in the double's user memory, where the kernel's
        // `sys_mount` copies them in.
        let update = |args: &mut UfsArgs| {
            let ub = crate::kern::vfs_subr::tests::user_buf(hal::user::bytes_of(
                core::slice::from_ref(args),
            ));
            sys(
                sys_mount,
                p,
                &[path(b"ffs\0"), path(b"/\0"), MNT_UPDATE as usize, ub.addr()],
            )
        };
        update(&mut args).unwrap();
        assert!(mp.flag() & MNT_EXPORTED != 0);
        assert_eq!(mp.flag() & MNT_UPDATE, 0, "the update flag is gone");
        assert_eq!(
            check([10, 0, 2, 9]),
            Ok((MNT_EXPORTED | MNT_EXRDONLY, 32767))
        );
        assert_eq!(check([10, 0, 3, 9]), Err(Errno::EACCES));

        // The same network again is refused, and the mount keeps its flags.
        assert_eq!(update(&mut args), Err(Errno::EPERM));
        assert!(mp.flag() & MNT_EXPORTED != 0);

        // mountd deletes the list before it loads another.
        args.export_info.ex_flags = MNT_DELEXPORT;
        update(&mut args).unwrap();
        assert_eq!(mp.flag() & MNT_EXPORTED, 0);
        assert_eq!(check([10, 0, 2, 9]), Err(Errno::EACCES));

        unmount_root(p, mp);
        teardown();
    }

    /// The export list is the mount's glue member (N5.5b0) and mounts are recycled, where the
    /// C's list lives in a `struct ufsmount` that `ffs_mountfs` `malloc(M_ZERO)`s afresh: an
    /// export left by an unmount (no `MNT_DELEXPORT`) is gone from the recycled mount after
    /// `ffs_mountfs`.
    #[cfg(feature = "nfsserver")]
    #[test]
    fn a_recycled_mount_starts_with_no_exports() {
        use crate::kern::uipc_mbuf::tests::mbinit_again;
        use crate::kern::vfs_subr::tests::exports::{args, sin};
        use crate::kern::vfs_syscalls::sys_mount;
        use crate::sys::mount::{MNT_EXPORTED, MNT_UPDATE, UfsArgs};

        let img = newfs::Image::new(newfs::FFS2_4M);
        let (_g, p) = setup(img.finish());
        mbinit_again();
        let mp = mount_root(p, false);
        let net = sin(2, [10, 0, 2, 0]);
        let mask = sin(2, [255, 255, 255, 0]);
        let args = UfsArgs {
            fspec: 0,
            export_info: args(MNT_EXPORTED, 0, Some(net), Some(mask)),
        };
        let ub = crate::kern::vfs_subr::tests::user_buf(hal::user::bytes_of(
            core::slice::from_ref(&args),
        ));
        sys(
            sys_mount,
            p,
            &[path(b"ffs\0"), path(b"/\0"), MNT_UPDATE as usize, ub.addr()],
        )
        .unwrap();
        assert!(mp.g.export.ne_rtable_inet.get().is_some(), "exported");

        // Unmounted with its list still hanging (the C's free(ump) forgets it), then mounted
        // again: vfs_mount_alloc hands the same mount out of its free list.
        unmount_root(p, mp);
        assert!(
            mp.g.export.ne_rtable_inet.get().is_some(),
            "the unmount leaves the list, as the C's free(ump) does"
        );
        let again = mount_root(p, false);
        assert!(core::ptr::eq(again, mp), "the mount was recycled");
        assert!(again.g.export.ne_rtable_inet.get().is_none());
        assert_eq!(again.flag() & MNT_EXPORTED, 0);

        unmount_root(p, again);
        teardown();
    }

    /// msdosfs's export list (N5.7c3: `bsd`'s msdosfs test, moved here with the switch, as the
    /// kernel's msdosfs reaches `bsd`'s export lists only through this crate's glue): `mount
    /// -u` of a FAT file system with `msdosfs_args` whose `fspec` is NULL reaches the kernel's
    /// `msdosfs_mount`, which hands the list to `vfs_export`; `VFS_CHECKEXP`'s `MOUNT_MSDOS`
    /// arm (`ufs_check_export` over the mount's list) then answers the listed client and
    /// refuses the others.
    #[cfg(all(feature = "nfsserver", feature = "msdosfs"))]
    #[test]
    fn mount_update_exports_a_fat_file_system() {
        use crate::kern::uipc_mbuf::tests::mbinit_again;
        use crate::kern::vfs_subr::tests::exports::{args, check_export, sin};
        use crate::kern::vfs_syscalls::sys_mount;
        use crate::sys::mount::{MNT_EXPORTED, MNT_EXRDONLY, MNT_UPDATE};
        use kernel::fs::msdosfs::msdosfs_vfsops::{msdosfs_mountfs, msdosfs_statfs};
        use kernel::fs::msdosfs::testmsdosfs::{mkfat, noargs};

        let mut img = mkfat::Image::new(mkfat::FAT12_1M);
        let root = img.root();
        img.add_file(root, None, b"README  TXT", &[b'r'; 3000]);
        img.mkdir(root, b"SUBDIR     ", 1);
        let (_g, p) = setup(img.finish());
        mbinit_again();
        // The FAT image at `/`, as mount_root mounts an FFS one.
        let mp = vfs_mount_alloc(None, vfs_byname(b"msdos").unwrap());
        mp.with_stat(|sp| sp.f_mntonname[0] = b'/');
        msdosfs_mountfs(diskvp(), mp, p, &noargs()).unwrap();
        let mut st = mp.stat();
        msdosfs_statfs(mp, &mut st, p).unwrap();
        mp.with_stat(|sp| *sp = st);
        vfs_unbusy(mp);
        crate::sys::mount::mountlist_insert_tail(mp);
        let rootvp = VFS_ROOT(mp).unwrap();
        set_rootvnode(Some(rootvp));
        p.fd().set_cdir(Some(rootvp));
        vref(rootvp);
        let _ = VOP_UNLOCK(rootvp);

        let ro = MNT_EXPORTED | MNT_EXRDONLY;
        assert_eq!(check_export(mp, [10, 0, 0, 5]), Err(Errno::EACCES));
        let net = sin(2, [10, 0, 0, 0]);
        let mask = sin(2, [255, 255, 255, 0]);
        let mut a = noargs();
        a.export_info = args(ro, 32767, Some(net), Some(mask));
        let ub =
            crate::kern::vfs_subr::tests::user_buf(hal::user::bytes_of(core::slice::from_ref(&a)));
        sys(
            sys_mount,
            p,
            &[
                path(b"msdos\0"),
                path(b"/\0"),
                MNT_UPDATE as usize,
                ub.addr(),
            ],
        )
        .unwrap();
        assert!(mp.flag() & MNT_EXPORTED != 0);
        assert_eq!(check_export(mp, [10, 0, 0, 5]), Ok((ro, 32767)));
        assert_eq!(check_export(mp, [10, 0, 1, 5]), Err(Errno::EACCES));

        unmount_root(p, mp);
        teardown();
    }

    /// `mount -u` of an ISO 9660 disc (the kernel's cd9660 over this module's disk and `bsd`'s
    /// glue, N5.8c2): mount(2)'s `MNT_UPDATE` without a device name reaches the kernel's
    /// `cd9660_mount` and `bsd`'s `vfs_export`, and `VFS_CHECKEXP`'s `MOUNT_CD9660` arm (the
    /// C's `cd9660_check_export`, `ufs_check_export`'s body over the mount's list) answers the
    /// listed client and refuses the others; without the arm the match's last arm panics.
    #[cfg(all(feature = "nfsserver", feature = "cd9660"))]
    #[test]
    fn mount_update_exports_an_iso_9660_disc() {
        use crate::kern::uipc_mbuf::tests::mbinit_again;
        use crate::kern::vfs_subr::tests::exports::{args, check_export, sin};
        use crate::kern::vfs_syscalls::sys_mount;
        use crate::sys::mount::{MNT_EXPORTED, MNT_EXRDONLY, MNT_UPDATE};
        use kernel::fs::isofs::cd9660::cd9660_vfsops::{cd9660_statfs, iso_mountfs};
        use kernel::fs::isofs::cd9660::testcd9660::{self, image};

        let (_g, p) = setup(image::image_bytes());
        mbinit_again();
        // The disc at `/`, read-only, as mount_root mounts an FFS image.
        let mp = vfs_mount_alloc(None, vfs_byname(b"cd9660").unwrap());
        mp.set_flag(mp.flag() | MNT_RDONLY);
        mp.with_stat(|sp| sp.f_mntonname[0] = b'/');
        iso_mountfs(diskvp(), mp, p, &mut testcd9660::args(0, 0)).unwrap();
        let mut st = mp.stat();
        cd9660_statfs(mp, &mut st, p).unwrap();
        mp.with_stat(|sp| *sp = st);
        vfs_unbusy(mp);
        crate::sys::mount::mountlist_insert_tail(mp);
        let rootvp = VFS_ROOT(mp).unwrap();
        set_rootvnode(Some(rootvp));
        p.fd().set_cdir(Some(rootvp));
        vref(rootvp);
        let _ = VOP_UNLOCK(rootvp);
        assert_eq!(read_file(p, b"/m10c-iso.txt\0").unwrap(), b"m10c-iso-42\n");

        let ro = MNT_EXPORTED | MNT_EXRDONLY;
        assert_eq!(check_export(mp, [10, 0, 0, 5]), Err(Errno::EACCES));
        let net = sin(2, [10, 0, 0, 0]);
        let mask = sin(2, [255, 255, 255, 0]);
        let mut a = testcd9660::args(0, 0);
        a.export_info = args(ro, 32767, Some(net), Some(mask));
        let ub =
            crate::kern::vfs_subr::tests::user_buf(hal::user::bytes_of(core::slice::from_ref(&a)));
        sys(
            sys_mount,
            p,
            &[
                path(b"cd9660\0"),
                path(b"/\0"),
                (MNT_UPDATE | MNT_RDONLY) as usize,
                ub.addr(),
            ],
        )
        .unwrap();
        assert!(mp.flag() & MNT_EXPORTED != 0);
        assert_eq!(check_export(mp, [10, 0, 0, 5]), Ok((ro, 32767)));
        assert_eq!(check_export(mp, [10, 0, 1, 5]), Err(Errno::EACCES));

        unmount_root(p, mp);
        teardown();
    }

    /// `iso_disklabelspoof` (`subr_disk.rs`'s shell over the kernel's `iso_volume_label`,
    /// N5.8c2) through a driver's strategy over this module's disk: the whole disc in `a` and
    /// `c`, the volume id's halves the type and pack names, the checksum; a disc without a
    /// primary descriptor is `EINVAL`, its label left as it was.
    #[cfg(feature = "cd9660")]
    #[test]
    fn a_cd_gets_a_disk_label() {
        use crate::kern::subr_disk::{dkcksum, iso_disklabelspoof};
        use crate::sys::disklabel::{
            DISKMAGIC, Disklabel, FS_ISO9660, MAXPARTITIONS, RAW_PART, dl_getpoffset, dl_getpsize,
        };
        use kernel::fs::isofs::cd9660::testcd9660::image;

        let (_g, _p) = setup(image::image_bytes());
        let mut lp = Disklabel::zeroed();
        lp.d_secsize = 512;
        lp.d_secperunit = 100;
        iso_disklabelspoof(DISKDEV, transfer, &mut lp).unwrap();
        assert_eq!(&lp.d_typename, b"M10C            ");
        assert_eq!(&lp.d_packname, b"                ");
        for i in [0, RAW_PART as usize] {
            assert_eq!(lp.d_partitions[i].p_fstype, FS_ISO9660);
            assert_eq!(dl_getpsize(&lp.d_partitions[i]), 100);
            assert_eq!(dl_getpoffset(&lp.d_partitions[i]), 0);
        }
        assert_eq!(usize::from(lp.d_npartitions), MAXPARTITIONS);
        assert_eq!(
            (lp.d_magic, lp.d_magic2, lp.d_version),
            (DISKMAGIC, DISKMAGIC, 1)
        );
        assert_eq!(dkcksum(&lp), 0);

        // not a CD
        *DISK.lock().unwrap_or_else(|e| e.into_inner()) = vec![0u8; image::IMAGE_SIZE];
        let mut lp = Disklabel::zeroed();
        assert_eq!(
            iso_disklabelspoof(DISKDEV, transfer, &mut lp),
            Err(Errno::EINVAL)
        );
        assert_eq!((lp.d_magic, lp.d_npartitions), (0, 0));
        teardown();
    }
}
/* </TESTS> */
