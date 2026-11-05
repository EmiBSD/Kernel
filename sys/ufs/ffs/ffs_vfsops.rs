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
//! quota, dirhash and mfs tests mount an image with.
//!
//! Upstream: sys/ufs/ffs/ffs_vfsops.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `rootdev` is `sys/systm.rs`'s `ROOTDEV` and `swapdev` the machine's (`conf.c`) until N7
//!   (docs/SCAFFOLDING.md); `rootvp` and `swapdev_vp` are the kernel's VFS tables'.

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
    // The fixture of `bsd`'s tests that mount an FFS image (nfs's, quota's, dirhash's, mfs's):
    // `newfs`, the kernel's image builder, a block device vnode whose strategy reads and writes
    // the image, and the root mount through the kernel's `ffs_mountfs`; with the tests of
    // `ffs_mount`'s export update (the exports are `bsd`'s until N6) and decision 43's ufs
    // bench. The image tests of the file system itself are the kernel's
    // (`kernel::fs::ufs::ffs::ffs_vfsops`'s tests, over its test glue).

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use crate::kern::kern_descrip::sys_close;
    use crate::kern::subr_xxx::nullop;
    use crate::kern::sys_generic::sys_read;
    use crate::kern::vfs_init::{set_rootvnode, vfs_byname};
    use crate::kern::vfs_syscalls::{dounmount, sys_getdents, sys_open, sys_unlink};
    use crate::kern::vfs_vops::{VOP_UNLOCK, vop_generic_bwrite};
    use crate::machine::Machine;
    use crate::machine::cpu::Cpu;
    use crate::machine::intr::{splbio, splx};
    use crate::sys::buf::{B_ERROR, B_READ, BufLegacy, biodone, bufinit};
    use crate::sys::errno::Errno;
    use crate::sys::fcntl::{O_CREAT, O_RDONLY, O_RDWR};
    use crate::sys::mount::{MNT_RDONLY, MNT_WAIT, Mount, VB_WAIT, VB_WRITE, VFS_ROOT};
    use crate::sys::param::DEV_BSIZE;
    use crate::sys::proc::Proc;
    use crate::sys::systm::{SyCall, SysArgs};
    use crate::sys::types::{Register, makedev};
    use crate::sys::vnode::{
        Vnode, VopFsyncArgs, VopInactiveArgs, VopStrategyArgs, Vops, bdevvp, vflushbuf, vfs_busy,
        vfs_mount_alloc, vfs_unbusy, vref, vrele,
    };
    use crate::ufs::ufs::inode::vtoi;
    use kernel::fs::ufs::ffs::ffs_vfsops::{ffs_mountfs, ffs_statfs};

    /// `newfs`: the kernel's FFS image builder (`kernel::fs::ufs::ffs::testffs::newfs`), and
    /// the clean flag of an image.
    pub(crate) mod newfs {
        pub(crate) use kernel::fs::ufs::ffs::testffs::newfs::*;

        /// The super-block's `fs_clean` in an image.
        pub(crate) fn clean(disk: &[u8], ufs2: bool) -> i8 {
            super_block(disk, ufs2).fs_clean
        }
    }

    /// The disk the strategy below reads and writes.
    pub(crate) static DISK: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

    /// `diskvn`'s strategy: a synchronous transfer between the buffer and the image at
    /// `b_blkno`, then `biodone`.
    fn disk_strategy(ap: &mut VopStrategyArgs) -> Result<(), Errno> {
        let bp = ap.a_bp;
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
        Ok(())
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

    /// Decision 43's ufs pairs (brief N5.5a 7.4b): the hot operations of the UFS layer over an
    /// FFS2 image `newfs` builds, through `bsd`'s system calls and `namei` over the kernel's
    /// ffs (N5.5b2; `bsd`'s ffs before), as `bsd`'s threads reach
    /// the UFS code. A sub-slice runs it on its base and on its tip, dev and release,
    /// interleaved, and puts the medians in its `Bench:` lines:
    ///
    /// `cargo test -p bsd --lib [--release] ufs_bench -- --ignored --nocapture --test-threads 1`
    ///
    /// (`UFSBENCH=<row>` runs the rows whose name contains it). Each row prints
    /// `UFSBENCH <row> median <ns> ns/op`, the median of seven samples, each the median of
    /// 200 timed batches. `#[ignore]`: a measurement, not a check.
    #[test]
    #[ignore = "decision 43's ufs bench: run by hand, --release, --test-threads 1"]
    fn ufs_bench() {
        use std::boxed::Box;
        use std::hint::black_box;
        use std::time::Instant;

        use crate::kern::vfs_syscalls::{sys_lseek, sys_stat};
        use crate::sys::namei::{FOLLOW, LOOKUP, NOCACHE, NiDirp, namei, ndinit};

        const REPS: usize = 7;
        const ROUNDS: usize = 200;

        let mut img = newfs::Image::new(newfs::FFS2_4M);
        img.add_file(b"f", b"hello, world\n");
        for n in [
            &b"passwd"[..],
            b"group",
            b"hosts",
            b"fstab",
            b"rc",
            b"rc.conf",
            b"motd",
            b"services",
        ] {
            img.add_file(n, b"x");
        }
        let (_g, p) = setup(img.finish());
        let mp = mount_root(p, false);

        let p_f = path(b"/f\0");
        let p_n = path(b"/n\0");
        let p_root = path(b"/\0");
        let ub = crate::kern::vfs_subr::tests::user_buf(&[0u8; 256]);
        let mut dbuf = vec![0u8; 4096];
        let fd_root = sys(sys_open, p, &[p_root, O_RDONLY as usize, 0]).unwrap() as usize;
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/f"), p);
        namei(&mut nd).unwrap();
        let fvp = nd.ni_vp.unwrap();

        let mut rows: Vec<(&str, usize, Box<dyn FnMut()>)> = Vec::new();
        rows.push((
            "ffs.vtoi.dip",
            1000,
            Box::new(|| {
                let ip = vtoi(black_box(fvp));
                black_box(ip.dip_size());
            }),
        ));
        rows.push((
            "ffs.lookup.hit.1",
            100,
            Box::new(|| {
                let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/f"), p);
                namei(&mut nd).unwrap();
                vrele(nd.ni_vp.unwrap());
            }),
        ));
        rows.push((
            "ffs.lookup.miss",
            100,
            Box::new(|| {
                let mut nd = ndinit(LOOKUP, FOLLOW | NOCACHE, NiDirp::Sys(b"/f"), p);
                namei(&mut nd).unwrap();
                vrele(nd.ni_vp.unwrap());
            }),
        ));
        rows.push((
            "ffs.readdir.1blk",
            100,
            Box::new(|| {
                sys(sys_lseek, p, &[fd_root, 0, 0]).unwrap();
                let n = sys(
                    sys_getdents,
                    p,
                    &[fd_root, dbuf.as_mut_ptr() as usize, dbuf.len()],
                )
                .unwrap();
                black_box(n);
            }),
        ));
        rows.push((
            "ffs.create.remove",
            20,
            Box::new(|| {
                let fd = sys(sys_open, p, &[p_n, (O_RDWR | O_CREAT) as usize, 0o644]).unwrap();
                sys(sys_close, p, &[fd as usize]).unwrap();
                sys(sys_unlink, p, &[p_n]).unwrap();
            }),
        ));
        rows.push((
            "ffs.stat",
            100,
            Box::new(|| {
                sys(sys_stat, p, &[p_f, ub.addr()]).unwrap();
            }),
        ));

        let only = std::env::var("UFSBENCH").ok();
        for (name, batch, f) in rows.iter_mut() {
            if only.as_deref().is_some_and(|o| !name.contains(o)) {
                continue;
            }
            for _ in 0..*batch {
                f();
            }
            let mut med = Vec::new();
            for _ in 0..REPS {
                let mut samples = Vec::new();
                for _ in 0..ROUNDS {
                    let t = Instant::now();
                    for _ in 0..*batch {
                        f();
                    }
                    samples.push(t.elapsed().as_nanos() as f64 / *batch as f64);
                }
                samples.sort_by(f64::total_cmp);
                med.push(samples[ROUNDS / 2]);
            }
            med.sort_by(f64::total_cmp);
            std::println!("UFSBENCH {name} median {:.1} ns/op", med[REPS / 2]);
        }
        drop(rows);
        vrele(fvp);
        sys(sys_close, p, &[fd_root]).unwrap();
        unmount_root(p, mp);
        teardown();
    }
}
/* </TESTS> */
