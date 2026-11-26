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
 * Copyright (c) 1982, 1986, 1990, 1993, 1995
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Robert Elz at The University of Melbourne.
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
 *	@(#)ufs_quota.c	8.5 (Berkeley) 8/19/94
 */
/* </LICENSES> */

/* <CODE> */
//! Disk quotas (`option QUOTA`) are the kernel's (`kernel::fs::ufs::ufs::ufs_quota`,
//! N5.5a2): this module keeps LZ's names for `bsd`'s glue and holds the integration test over
//! an FFS image. The functions `bsd`'s file systems call are `quota.rs`'s.
//!
//! Upstream: sys/ufs/ufs/ufs_quota.c @ 3ce1f3f79392

pub use kernel::fs::ufs::ufs::ufs_quota::{
    DQ_BLKS, DQ_FAKE, DQ_INODS, DQ_LOCK, DQ_MOD, DQ_WANT, Limit, M_DQUOT, chklimit, dqget, dqref,
    dqrele, dqsync, quotaon, setquota_dqblk, setuse_dqblk,
};

/// `struct dquot` of `bsd`'s mounts: the kernel's.
pub type Dquot = kernel::fs::ufs::ufs::ufs_quota::Dquot<crate::kern::proc_glue::BsdProcGlue>;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the kernel's disk quotas through `bsd`'s ffs (the limit and grace logic of
    // `chkdqchg`/`chkiqchg`, `setquota` and `setuse` are the kernel module's tests):
    // `quotactl(2)` on an FFS image mounted on the host, with a
    // quota file laid out as `quotacheck(8)` leaves it: a user writes until `EDQUOT`, the
    // usage is read back with `Q_GETQUOTA`, and it survives `Q_SYNC`, an unmount with quotas
    // on, `Q_QUOTAOFF` and a remount.

    use std::vec;
    use std::vec::Vec;

    use crate::kern::kern_descrip::sys_close;

    use crate::kern::sys_generic::sys_write;
    use crate::kern::vfs_subr::tests::user_buf;
    use crate::kern::vfs_syscalls::{sys_chown, sys_open, sys_quotactl, sys_unlink};
    use crate::sys::errno::Errno;
    use crate::sys::fcntl::{O_CREAT, O_RDWR};
    use crate::sys::mount::MNT_QUOTA;
    use crate::sys::proc::Proc;
    use crate::sys::types::Uid;
    use crate::sys::ucred::CredRef;
    use crate::sys::ucred::Ucred;
    use crate::ufs::ffs::ffs_vfsops::tests::{
        DISK as DISKIMG, mount_root, newfs, path, read_file, setup, sys, teardown, unmount_root,
    };
    use crate::ufs::ufs::quota::{
        Dqblk, Q_GETQUOTA, Q_QUOTAOFF, Q_QUOTAON, Q_SETQUOTA, Q_SETUSE, Q_SYNC, USRQUOTA, qcmd,
    };

    fn dqblk(bhard: u32, bsoft: u32, curb: u32, ihard: u32, isoft: u32, curi: u32) -> Dqblk {
        Dqblk {
            dqb_bhardlimit: bhard,
            dqb_bsoftlimit: bsoft,
            dqb_curblocks: curb,
            dqb_ihardlimit: ihard,
            dqb_isoftlimit: isoft,
            dqb_curinodes: curi,
            dqb_btime: 0,
            dqb_itime: 0,
        }
    }

    /// The user whose quota the file system test sets.
    const USER: Uid = 1000;
    /// Their block hard limit, in `DEV_BSIZE` units (50 KB).
    const BHARD: u32 = 100;

    /// A user quota file as `quotacheck(8)` leaves it for a file system where nothing belongs
    /// to `USER` yet, with `edquota(8)`'s hard limit for `USER`: one `dqblk` per id up to
    /// `USER`.
    fn quota_file() -> Vec<u8> {
        let mut f = Vec::new();
        for id in 0..=USER {
            let b = if id == USER {
                dqblk(BHARD, 0, 0, 0, 0, 0)
            } else {
                Dqblk::default()
            };
            for w in [
                b.dqb_bhardlimit,
                b.dqb_bsoftlimit,
                b.dqb_curblocks,
                b.dqb_ihardlimit,
                b.dqb_isoftlimit,
                b.dqb_curinodes,
                b.dqb_btime,
                b.dqb_itime,
            ] {
                f.extend_from_slice(&w.to_ne_bytes());
            }
        }
        f
    }

    /// The `dqblk` of `id` in a quota file's bytes.
    fn entry(file: &[u8], id: Uid) -> Dqblk {
        let off = id as usize * size_of::<Dqblk>();
        let w = |i: usize| {
            let o = off + 4 * i;
            u32::from_ne_bytes([file[o], file[o + 1], file[o + 2], file[o + 3]])
        };
        Dqblk {
            dqb_bhardlimit: w(0),
            dqb_bsoftlimit: w(1),
            dqb_curblocks: w(2),
            dqb_ihardlimit: w(3),
            dqb_isoftlimit: w(4),
            dqb_curinodes: w(5),
            dqb_btime: w(6),
            dqb_itime: w(7),
        }
    }

    /// `quotactl("/", QCMD(cmd, USRQUOTA), uid, addr)`.
    fn quotactl(p: &Proc, cmd: i32, uid: Uid, addr: usize) -> Result<isize, Errno> {
        sys(
            sys_quotactl,
            p,
            &[
                path(b"/\0"),
                qcmd(cmd, USRQUOTA as i32) as usize,
                uid as usize,
                addr,
            ],
        )
    }

    /// `Q_GETQUOTA` of `uid`'s user quota.
    fn getq(p: &Proc, uid: Uid) -> Result<Dqblk, Errno> {
        let u = user_buf(&[0u8; size_of::<Dqblk>()]);
        quotactl(p, Q_GETQUOTA, uid, u.addr())?;
        Ok(hal::user::pod_from_bytes(&u.bytes()).unwrap())
    }

    /// Runs the thread as `uid` (a new credential) until the returned one is put back.
    fn become_user(p: &Proc, uid: Uid) -> Option<CredRef> {
        let mut cr = Ucred::new();
        for c in [&mut cr.cr_uid, &mut cr.cr_ruid, &mut cr.cr_svuid] {
            c.set(uid);
        }
        for c in [&mut cr.cr_gid, &mut cr.cr_rgid, &mut cr.cr_svgid] {
            c.set(uid);
        }
        p.set_cred(CredRef::new(cr))
    }

    #[test]
    fn quotas_on_an_ffs_image() {
        let mut img = newfs::Image::new(newfs::FFS2_4M);
        img.add_file(b"quota.user", &quota_file());
        let (_g, p) = setup(img.finish());

        let mp = mount_root(p, false);
        // Not on yet: no quota to get.
        assert_eq!(getq(p, USER), Err(Errno::EINVAL));
        quotactl(p, Q_QUOTAON, 0, path(b"/quota.user\0")).unwrap();
        assert_ne!(mp.flag() & MNT_QUOTA, 0);
        let b = getq(p, USER).unwrap();
        assert_eq!(
            (b.dqb_bhardlimit, b.dqb_curblocks, b.dqb_curinodes),
            (BHARD, 0, 0)
        );

        // A file for the user: chown moves the inode's charge to them.
        let fd = sys(
            sys_open,
            p,
            &[path(b"/f\0"), (O_RDWR | O_CREAT) as usize, 0o644],
        )
        .unwrap();
        sys(sys_close, p, &[fd as usize]).unwrap();
        sys(sys_chown, p, &[path(b"/f\0"), USER as usize, 0]).unwrap();
        assert_eq!(getq(p, USER).unwrap().dqb_curinodes, 1);

        // The user writes 8 KB blocks until the hard limit refuses one: six blocks are 96
        // sectors, a seventh would make 112 >= 100.
        let root = become_user(p, USER);
        let fd = sys(sys_open, p, &[path(b"/f\0"), O_RDWR as usize, 0]).unwrap();
        let chunk = vec![0x5au8; 8192];
        let mut written = 0;
        let error = loop {
            match sys(
                sys_write,
                p,
                &[fd as usize, chunk.as_ptr() as usize, chunk.len()],
            ) {
                Ok(n) => written += n as usize,
                Err(e) => break e,
            }
            assert!(
                written <= 1 << 20,
                "the hard limit never stopped the writes"
            );
        };
        assert_eq!(error, Errno::EDQUOT);
        assert_eq!(written, 6 * 8192);
        sys(sys_close, p, &[fd as usize]).unwrap();
        // A user may read their own quota, not somebody else's.
        let b = getq(p, USER).unwrap();
        assert_eq!((b.dqb_curblocks, b.dqb_curinodes), (96, 1));
        assert_eq!(getq(p, 0), Err(Errno::EPERM));
        assert_eq!(quotactl(p, Q_QUOTAOFF, 0, 0), Err(Errno::EPERM));
        // `become_user`'s credential goes as the thread takes root's back.
        drop(p.set_cred(root.expect("root's credentials")));

        // Q_SYNC writes the usage into the quota file.
        quotactl(p, Q_SYNC, 0, 0).unwrap();
        let e = entry(&read_file(p, b"/quota.user\0").unwrap(), USER);
        assert_eq!(
            (e.dqb_bhardlimit, e.dqb_curblocks, e.dqb_curinodes),
            (BHARD, 96, 1)
        );

        // Unmounting with quotas on turns them off (ffs_flushfiles); a remount reads the usage
        // back from the file.
        unmount_root(p, mp);
        newfs::check(&DISKIMG.lock().unwrap(), true);
        let mp = mount_root(p, false);
        assert_eq!(mp.flag() & MNT_QUOTA, 0);
        quotactl(p, Q_QUOTAON, 0, path(b"/quota.user\0")).unwrap();
        let b = getq(p, USER).unwrap();
        assert_eq!(
            (b.dqb_bhardlimit, b.dqb_curblocks, b.dqb_curinodes),
            (BHARD, 96, 1)
        );

        // Q_SETQUOTA raises the limit (the usage stays the kernel's), Q_SETUSE sets the usage.
        let lim = dqblk(BHARD * 2, 0, 12345, 0, 0, 999);
        let u = user_buf(hal::user::bytes_of(core::slice::from_ref(&lim)));
        quotactl(p, Q_SETQUOTA, USER, u.addr()).unwrap();
        let b = getq(p, USER).unwrap();
        assert_eq!(
            (b.dqb_bhardlimit, b.dqb_curblocks, b.dqb_curinodes),
            (BHARD * 2, 96, 1)
        );
        let usage = dqblk(0, 0, 90, 0, 0, 1);
        let u = user_buf(hal::user::bytes_of(core::slice::from_ref(&usage)));
        quotactl(p, Q_SETUSE, USER, u.addr()).unwrap();
        assert_eq!(getq(p, USER).unwrap().dqb_curblocks, 90);

        // Removing the file gives the blocks and the inode back.
        sys(sys_unlink, p, &[path(b"/f\0")]).unwrap();
        let b = getq(p, USER).unwrap();
        assert_eq!((b.dqb_curblocks, b.dqb_curinodes), (0, 0));

        // Q_QUOTAOFF writes the dquots back and closes the file; a remount and Q_QUOTAON find
        // the new limit on disk.
        quotactl(p, Q_QUOTAOFF, 0, 0).unwrap();
        assert_eq!(mp.flag() & MNT_QUOTA, 0);
        assert_eq!(getq(p, USER), Err(Errno::EINVAL));
        unmount_root(p, mp);
        let mp = mount_root(p, false);
        quotactl(p, Q_QUOTAON, 0, path(b"/quota.user\0")).unwrap();
        let b = getq(p, USER).unwrap();
        assert_eq!(
            (b.dqb_bhardlimit, b.dqb_curblocks, b.dqb_curinodes),
            (BHARD * 2, 0, 0)
        );
        unmount_root(p, mp);
        newfs::check(&DISKIMG.lock().unwrap(), true);
        teardown();
    }
}
/* </TESTS> */
