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
 * Copyright (c) 2001, 2002 Ian Dowse.  All rights reserved.
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! The hash-based lookup scheme for UFS directories (`option UFS_DIRHASH`, feature
//! `ufs_dirhash`) is the kernel's (`kernel::fs::ufs::ufs::ufs_dirhash`, N5.5a2): this module
//! keeps LZ's names for `bsd`'s UFS files and `ffs_vars[]`, and holds the integration tests
//! over an FFS image.
//!
//! Upstream: sys/ufs/ufs/ufs_dirhash.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - `ufsdirhash_lookup` returns the busy buffer's token (`BufRef`): `bsd`'s `ufs_lookup`
//!   reads the entry from it and releases it with the kernel's `brelse`.

pub use kernel::fs::ufs::ufs::ufs_dirhash::{
    M_DIRHASH, UFS_DIRHASHCHECK, UFS_DIRHASHMAXMEM, UFS_DIRHASHMEM, UFS_MINDIRHASHSIZE,
    ufsdirhash_add, ufsdirhash_build, ufsdirhash_checkblock, ufsdirhash_dirtrunc,
    ufsdirhash_enduseful, ufsdirhash_findfree, ufsdirhash_free, ufsdirhash_init, ufsdirhash_lookup,
    ufsdirhash_move, ufsdirhash_newblk, ufsdirhash_remove,
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the kernel's dirhash through `bsd`'s ffs (its slot and free-space
    // bookkeeping on a hash built by hand are the kernel module's tests): on an FFS image mounted on the host (the
    // `newfs` builder of the ffs tests), a directory of thousands of entries that is hashed,
    // looked up, shrunk, renamed into and grown with `ufsdirhash_checkblock` on, then checked
    // against a linear walk; and recycling under a small `ufs_dirhashmaxmem`.

    use core::ptr;
    use core::sync::atomic::Ordering;
    use std::collections::BTreeSet;
    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{format, vec};

    use super::*;
    use crate::sys::buf::BufLegacy;
    use crate::sys::errno::Errno;
    use crate::ufs::ufs::dir::DIRBLKSIZ;
    use crate::ufs::ufs::inode::Inode;

    /// `DIRBLKSIZ` as an `int`.
    const DIRBLK: i32 = DIRBLKSIZ as i32;
    use crate::kern::kern_descrip::sys_close;
    use crate::kern::subr_xxx::nullop;
    use crate::kern::vfs_init::{rootvnode, set_rootvnode, vfs_byname};
    use crate::kern::vfs_syscalls::{
        dounmount, sys_getdents, sys_link, sys_mkdir, sys_open, sys_rename, sys_stat, sys_sync,
        sys_unlink,
    };
    use crate::kern::vfs_vops::VOP_UNLOCK;
    use crate::kern::vfs_vops::vop_generic_bwrite;
    use crate::machine::Machine;
    use crate::machine::cpu::Cpu;
    use crate::machine::intr::{splbio, splx};
    use crate::sys::buf::{B_ERROR, B_READ};
    use crate::sys::buf::{biodone, bufinit};
    use crate::sys::fcntl::{O_CREAT, O_RDONLY, O_RDWR};
    use crate::sys::mount::{MNT_WAIT, Mount, VB_WAIT, VB_WRITE, VFS_ROOT, VFS_VGET};
    use crate::sys::param::DEV_BSIZE;
    use crate::sys::proc::Proc;
    use crate::sys::stat::Stat;
    use crate::sys::systm::{SyCall, SysArgs};
    use crate::sys::types::{Register, makedev};
    use crate::sys::vnode::{Vnode, VopFsyncArgs, VopInactiveArgs, VopStrategyArgs, Vops};
    use crate::sys::vnode::{
        bdevvp, vflushbuf, vfs_busy, vfs_mount_alloc, vfs_unbusy, vput, vref, vrele,
    };
    use crate::ufs::ffs::ffs_extern::FFS_DIRHASH_MEM;
    use crate::ufs::ffs::ffs_vfsops::tests::newfs;
    use crate::ufs::ffs::ffs_vfsops::{ffs_mountfs, ffs_statfs, ffs_sysctl};
    use crate::ufs::ufs::inode::vtoi;

    /// The disk the strategy below reads and writes.
    static DISK: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());

    /// The fake disk's strategy: a synchronous transfer between the buffer and the image.
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

    fn disk_fsync(ap: &mut VopFsyncArgs<'_>) -> Result<(), Errno> {
        vflushbuf(ap.a_vp, ap.a_waitfor == MNT_WAIT);
        Ok(())
    }

    fn disk_inactive(ap: &mut VopInactiveArgs<'_>) -> Result<(), Errno> {
        VOP_UNLOCK(ap.a_vp)
    }

    /// The fake disk's block device operations.
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

    /// Memory, the vfs, a fresh buffer cache, the image as the disk and the thread as
    /// `curproc` (as the ffs tests set up), with the dirhash checks on and no hash memory in use.
    fn setup(image: Vec<u8>) -> (MutexGuard<'static, ()>, &'static Proc) {
        let (g, p) = crate::kern::vfs_subr::tests::setup();
        Machine::set_curproc(Machine::curcpu(), Some(p));
        crate::kern::proc_glue::test_set_limits(p, 128);

        // A fresh buffer cache over this test's memory.
        kernel::fs::bio::bio_test_reset();
        kernel::fs::bufq::bufq_test_reset();
        bufinit();

        *DISK.lock().unwrap_or_else(|e| e.into_inner()) = image;
        // Hashes of earlier tests lived in the memory just reset (vfsinit ran ufsdirhash_init).
        UFS_DIRHASHMEM.store(0, Ordering::Relaxed);
        UFS_DIRHASHCHECK.store(1, Ordering::Relaxed);
        (g, p)
    }

    fn teardown() {
        UFS_DIRHASHCHECK.store(0, Ordering::Relaxed);
        Machine::set_curproc(Machine::curcpu(), None);
    }

    /// Mounts the disk at `/`, as the ffs tests do.
    fn mount_root(p: &'static Proc) -> &'static Mount {
        let devvp = bdevvp(makedev(17, 1)).unwrap().unwrap();
        crate::sys::vnode::set_vops(devvp, &DISK_VOPS);
        let mp = vfs_mount_alloc(None, vfs_byname(b"ffs").unwrap());
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
    fn unmount_root(p: &'static Proc, mp: &'static Mount) {
        if let Some(cdir) = p.fd().replace_cdir(None) {
            vrele(cdir);
        }
        if let Some(root) = rootvnode() {
            set_rootvnode(None);
            vrele(root);
        }
        vfs_busy(mp, VB_WRITE | VB_WAIT).unwrap();
        dounmount(mp, 0, p).unwrap();
    }

    /// A system call with up to six arguments; `retval[0]`.
    fn sys(f: SyCall, p: &Proc, args: &[usize]) -> Result<isize, Errno> {
        let mut v: SysArgs = [0; 6];
        for (slot, a) in v.iter_mut().zip(args) {
            *slot = *a as Register;
        }
        let mut rv = [0; 2];
        f(p, &v, &mut rv)?;
        Ok(rv[0])
    }

    /// `s` as a NUL-terminated path.
    fn cpath(s: &str) -> crate::kern::vfs_subr::tests::UserStr {
        crate::kern::vfs_subr::tests::user_str(s.as_bytes())
    }

    /// `stat(2)`: the inode number of `path`.
    fn ino_of(p: &Proc, path: &str) -> Result<u64, Errno> {
        let c = cpath(path);
        let ub = crate::kern::vfs_subr::tests::user_buf(&[0; Stat::SIZE]);
        sys(sys_stat, p, &[c.as_ptr() as usize, ub.addr()])?;
        Ok(ub.stat().st_ino)
    }

    fn link(p: &Proc, from: &str, to: &str) -> Result<isize, Errno> {
        let (a, b) = (cpath(from), cpath(to));
        sys(sys_link, p, &[a.as_ptr() as usize, b.as_ptr() as usize])
    }

    fn rename(p: &Proc, from: &str, to: &str) -> Result<isize, Errno> {
        let (a, b) = (cpath(from), cpath(to));
        sys(sys_rename, p, &[a.as_ptr() as usize, b.as_ptr() as usize])
    }

    fn unlink(p: &Proc, path: &str) -> Result<isize, Errno> {
        let c = cpath(path);
        sys(sys_unlink, p, &[c.as_ptr() as usize])
    }

    fn mkdir(p: &Proc, path: &str) {
        let c = cpath(path);
        sys(sys_mkdir, p, &[c.as_ptr() as usize, 0o755]).unwrap();
    }

    fn create(p: &Proc, path: &str) {
        let c = cpath(path);
        let fd = sys(
            sys_open,
            p,
            &[c.as_ptr() as usize, (O_RDWR | O_CREAT) as usize, 0o644],
        )
        .unwrap();
        sys(sys_close, p, &[fd as usize]).unwrap();
    }

    /// The names in directory `path` (without `.` and `..`) read with getdents(2), which walks
    /// the blocks linearly and never consults the hash.
    fn list_dir(p: &Proc, path: &str) -> BTreeSet<Vec<u8>> {
        let c = cpath(path);
        let fd = sys(sys_open, p, &[c.as_ptr() as usize, O_RDONLY as usize, 0]).unwrap();
        let mut buf = vec![0u8; 8192];
        let mut names = BTreeSet::new();
        loop {
            let n = sys(
                sys_getdents,
                p,
                &[fd as usize, buf.as_mut_ptr() as usize, buf.len()],
            )
            .unwrap();
            if n == 0 {
                break;
            }
            let mut off = 0;
            while off < n as usize {
                let reclen = u16::from_ne_bytes([buf[off + 16], buf[off + 17]]) as usize;
                let namlen = buf[off + 19] as usize;
                let nm = &buf[off + 24..off + 24 + namlen];
                if nm != b"." && nm != b".." {
                    assert!(names.insert(nm.to_vec()), "duplicate entry");
                }
                off += reclen;
            }
        }
        sys(sys_close, p, &[fd as usize]).unwrap();
        names
    }

    /// Runs `f` on the in-core inode of directory `path`.
    fn with_dir_inode<R>(
        p: &Proc,
        mp: &'static Mount,
        path: &str,
        f: impl FnOnce(&Inode) -> R,
    ) -> R {
        let ino = ino_of(p, path).unwrap();
        let vp: &'static Vnode = VFS_VGET(mp, ino).unwrap();
        let r = f(&vtoi(vp));
        vput(vp);
        r
    }

    /// Whether directory `path` has an intact hash.
    fn hashed(p: &Proc, mp: &'static Mount, path: &str) -> bool {
        with_dir_inode(p, mp, path, |ip| {
            ip.i_dirhash
                .read()
                .as_deref()
                .is_some_and(kernel::fs::ufs::ufs::dirhash::Dirhash::hashed)
        })
    }

    /// `sysctl vfs.ffs.dirhash_mem`.
    fn sysctl_dirhash_mem(p: &Proc) -> i32 {
        let mut v = 0i32;
        let mut len = size_of::<i32>();
        ffs_sysctl(
            &[FFS_DIRHASH_MEM],
            ptr::from_mut(&mut v) as usize,
            &mut len,
            0,
            0,
            p,
        )
        .unwrap();
        assert_eq!(len, size_of::<i32>());
        v
    }

    /// Every name in `names` resolves to `ino` in `dir`, and a few names not there do not.
    fn check_lookups(p: &Proc, dir: &str, names: &BTreeSet<Vec<u8>>, ino: u64, tag: &str) {
        for n in names {
            let path = format!("{dir}/{}", core::str::from_utf8(n).unwrap());
            assert_eq!(ino_of(p, &path), Ok(ino), "{path}");
        }
        for i in 0..20 {
            let path = format!("{dir}/absent-{tag}-{i}");
            assert_eq!(ino_of(p, &path), Err(Errno::ENOENT), "{path}");
        }
    }

    /// The name of entry `i` of the big directory: lengths vary from 1 to 30 bytes, so that the
    /// entries have different record sizes.
    fn big_name(i: usize) -> std::string::String {
        let pad = "x".repeat(i % 23);
        format!("n{i}{pad}")
    }

    #[test]
    fn a_large_directory_is_hashed_and_kept_in_step() {
        const N: usize = 3000;
        let (_g, p) = setup(newfs::Image::new(newfs::FFS2_4M).finish());
        let mp = mount_root(p);
        assert_eq!(UFS_MINDIRHASHSIZE.load(Ordering::Relaxed), 5 * DIRBLK);
        assert_eq!(sysctl_dirhash_mem(p), 0);

        mkdir(p, "/d");
        create(p, "/d/target");
        let target = ino_of(p, "/d/target").unwrap();
        let mut names: BTreeSet<Vec<u8>> = BTreeSet::new();
        names.insert(b"target".to_vec());
        for i in 0..N {
            let n = big_name(i);
            link(p, "/d/target", &format!("/d/{n}")).unwrap();
            names.insert(n.into_bytes());
        }
        // The creations past ufs_mindirhashsize went through the hash.
        assert!(hashed(p, mp, "/d"));
        let mem = sysctl_dirhash_mem(p);
        assert!(mem > 0);
        assert_eq!(mem, UFS_DIRHASHMEM.load(Ordering::Relaxed));
        check_lookups(p, "/d", &names, target, "a");
        // A name that exists cannot be created again.
        assert_eq!(
            link(p, "/d/target", &format!("/d/{}", big_name(7))),
            Err(Errno::EEXIST)
        );

        // Remove every third entry (DELETE lookups through the hash, with the previous
        // entry's offset), so that blocks get holes.
        for i in (0..N).step_by(3) {
            let n = big_name(i);
            unlink(p, &format!("/d/{n}")).unwrap();
            names.remove(n.as_bytes());
        }
        assert!(hashed(p, mp, "/d"));
        check_lookups(p, "/d", &names, target, "b");

        // Rename some entries to longer names inside the directory: removals, and creations
        // that reuse the holes (ufsdirhash_findfree, compaction with ufsdirhash_move).
        for i in (1..N).step_by(7) {
            if i % 3 == 0 {
                continue;
            }
            let (from, to) = (big_name(i), format!("renamed-{i}-{}", "y".repeat(i % 17)));
            rename(p, &format!("/d/{from}"), &format!("/d/{to}")).unwrap();
            names.remove(from.as_bytes());
            names.insert(to.into_bytes());
        }
        // And add more, past the end of the directory (ufsdirhash_newblk).
        for i in N..N + 1000 {
            let n = big_name(i);
            link(p, "/d/target", &format!("/d/{n}")).unwrap();
            names.insert(n.into_bytes());
        }
        assert!(hashed(p, mp, "/d"));
        check_lookups(p, "/d", &names, target, "c");

        // The directory as a linear walk sees it agrees with the names.
        assert_eq!(list_dir(p, "/d"), names);

        // Without the hash (the size threshold raised: ufsdirhash_build frees it), every name
        // still resolves by the linear search, and the memory goes back.
        UFS_MINDIRHASHSIZE.store(i32::MAX, Ordering::Relaxed);
        check_lookups(p, "/d", &names, target, "d");
        assert!(!hashed(p, mp, "/d"));
        assert_eq!(sysctl_dirhash_mem(p), 0);
        UFS_MINDIRHASHSIZE.store(5 * DIRBLK, Ordering::Relaxed);

        // Rebuilt from the blocks on the next lookup: a hash of what is on disk.
        check_lookups(p, "/d", &names, target, "e");
        assert!(hashed(p, mp, "/d"));

        // Empty the directory but for `target`: the truncations (ufsdirhash_dirtrunc) follow.
        let all: Vec<Vec<u8>> = names.iter().filter(|n| *n != b"target").cloned().collect();
        for n in &all {
            unlink(p, &format!("/d/{}", core::str::from_utf8(n).unwrap())).unwrap();
        }
        create(p, "/d/last");
        let mut left = BTreeSet::new();
        left.insert(b"target".to_vec());
        left.insert(b"last".to_vec());
        assert_eq!(list_dir(p, "/d"), left);

        sys(sys_sync, p, &[]).unwrap();
        unmount_root(p, mp);
        assert_eq!(
            UFS_DIRHASHMEM.load(Ordering::Relaxed),
            0,
            "reclaim freed the hash"
        );
        newfs::check(&DISK.lock().unwrap(), true);

        // Mounted again, the directory reads back.
        let mp = mount_root(p);
        assert_eq!(ino_of(p, "/d/last").map(|i| i != target), Ok(true));
        assert_eq!(ino_of(p, "/d/target"), Ok(target));
        unmount_root(p, mp);
        teardown();
    }

    #[test]
    fn recycling_steals_the_least_used_hash() {
        const N: usize = 400;
        let (_g, p) = setup(newfs::Image::new(newfs::FFS2_4M).finish());
        let mp = mount_root(p);

        create(p, "/target");
        let target = ino_of(p, "/target").unwrap();
        let mut names = BTreeSet::new();
        for i in 0..N {
            names.insert(format!("e{i:03}").into_bytes());
        }
        for d in ["/a", "/b", "/c"] {
            mkdir(p, d);
            for n in &names {
                link(
                    p,
                    "/target",
                    &format!("{d}/{}", core::str::from_utf8(n).unwrap()),
                )
                .unwrap();
            }
        }
        assert!(hashed(p, mp, "/a") && hashed(p, mp, "/b") && hashed(p, mp, "/c"));

        // Drop the hashes (they were built while the directories grew), then measure one built
        // at the final size: the three directories are the same size, so each costs that.
        UFS_MINDIRHASHSIZE.store(i32::MAX, Ordering::Relaxed);
        for d in ["/a", "/b", "/c"] {
            assert_eq!(ino_of(p, &format!("{d}/nothere")), Err(Errno::ENOENT));
        }
        assert_eq!(UFS_DIRHASHMEM.load(Ordering::Relaxed), 0);
        UFS_MINDIRHASHSIZE.store(5 * DIRBLK, Ordering::Relaxed);
        assert_eq!(ino_of(p, "/a/nothere2"), Err(Errno::ENOENT));
        let one = UFS_DIRHASHMEM.load(Ordering::Relaxed);
        assert!(one > 0);
        // Allow two and a half hashes.
        let maxmem = 2 * one + one / 2;
        UFS_DIRHASHMAXMEM.store(maxmem, Ordering::Relaxed);

        // Look-ups in /a and /b hash them; /c does not fit.
        check_lookups(p, "/a", &names, target, "a");
        check_lookups(p, "/b", &names, target, "b");
        assert!(hashed(p, mp, "/a") && hashed(p, mp, "/b"));
        assert_eq!(UFS_DIRHASHMEM.load(Ordering::Relaxed), 2 * one);

        // Every look-up in /c that tries to build its hash takes a point off the score of the
        // least recently used hash (/a, at the head of the list); at zero, /a's memory goes to
        // /c. Each look-up is of a new name, so that the name cache does not answer it.
        let mut tries = 0;
        while !hashed(p, mp, "/c") {
            assert_eq!(ino_of(p, &format!("/c/miss{tries}")), Err(Errno::ENOENT));
            assert!(UFS_DIRHASHMEM.load(Ordering::Relaxed) <= maxmem);
            tries += 1;
            assert!(tries < 200, "/c never got a hash");
        }
        assert!(tries > 1, "recycling waits for the score to run out");
        assert!(!hashed(p, mp, "/a"), "/a's hash was recycled");
        assert!(hashed(p, mp, "/b"));
        assert!(UFS_DIRHASHMEM.load(Ordering::Relaxed) <= maxmem);

        // Every directory still answers correctly, hashed or not.
        check_lookups(p, "/c", &names, target, "c");
        check_lookups(p, "/a", &names, target, "a2");
        check_lookups(p, "/b", &names, target, "b2");
        assert!(UFS_DIRHASHMEM.load(Ordering::Relaxed) <= maxmem);

        unmount_root(p, mp);
        assert_eq!(UFS_DIRHASHMEM.load(Ordering::Relaxed), 0);
        UFS_DIRHASHMAXMEM.store(5 * 1024 * 1024, Ordering::Relaxed);
        teardown();
    }
}
/* </TESTS> */
