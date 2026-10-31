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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)quota.h	8.3 (Berkeley) 8/19/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/quota.h>`: the quota interface is the kernel's (`kernel::fs::ufs::ufs::quota`,
//! N5.5a2): this module keeps LZ's names and signatures for `bsd`'s file systems and the UFS
//! files still here, over the C's credential pointer and `struct mount`.
//!
//! Upstream: sys/ufs/ufs/quota.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - A charge's credential (`*const Ucred`, NULL, `NOCRED` or `FSCRED`) is the kernel's
//!   `Option<&Ucred>` through `cred_ref` (its one `unsafe`, moved here from `ufs_quota.rs`,
//!   until N5.5a2c's `Cred` carries the credential); a release's is `Cred::from_raw`.
//! - `qsync`, `quotaoff` and `ufs_quotactl` hand the kernel the mount's `struct ufsmount`
//!   (`vfstoufs`) beside the mount (the kernel's `quota.rs`, its deviations).

use kernel::fs::ufs::ufs::quota as kq;
use kernel::fs::vops::Cred;

use crate::sys::errno::Errno;
use crate::sys::mount::Mount;
use crate::sys::proc::Proc;
use crate::sys::types::{Daddr, Uid};
use crate::sys::ucred::Ucred;
use crate::sys::vnode::cred_ref;
use crate::ufs::ufs::inode::Inode;
use crate::ufs::ufs::ufsmount::vfstoufs;
pub use kq::{
    Dqblk, GRPQUOTA, INITQFNAMES, MAX_DQ_TIME, MAX_IQ_TIME, MAXQUOTAS, Q_GETQUOTA, Q_QUOTAOFF,
    Q_QUOTAON, Q_SETQUOTA, Q_SETUSE, Q_SYNC, QUOTAFILENAME, QUOTAGROUP, SUBCMDMASK, SUBCMDSHIFT,
    UFS_QUOTA_FORCE, UFS_QUOTA_NOGID, UFS_QUOTA_NOUID, USRQUOTA, UfsQuotaFlags, getinoquota, qcmd,
    ufs_quota_delete,
};

/// The credential a charge is checked against: `None` for the kernel's (`NOCRED`, `FSCRED`,
/// NULL).
fn chkcred<'a>(cred: *const Ucred) -> Option<&'a Ucred> {
    // SAFETY: the credential of a quota charge is the one of the vnode operation doing the
    // allocation, held by its caller for the operation's duration (`cred_ref`'s contract).
    unsafe { cred_ref(cred) }
}

/// `ufs_quota_alloc_blocks(i, c, cr)`.
pub fn ufs_quota_alloc_blocks(ip: &Inode, change: Daddr, cred: *const Ucred) -> Result<(), Errno> {
    kq::ufs_quota_alloc_blocks(ip, change, chkcred(cred))
}

/// `ufs_quota_free_blocks(i, c, cr)`.
pub fn ufs_quota_free_blocks(ip: &Inode, change: Daddr, cred: *const Ucred) -> Result<(), Errno> {
    kq::ufs_quota_free_blocks(ip, change, Cred::from_raw(cred))
}

/// `ufs_quota_alloc_inode(i, cr)`.
pub fn ufs_quota_alloc_inode(ip: &Inode, cred: *const Ucred) -> Result<(), Errno> {
    kq::ufs_quota_alloc_inode(ip, chkcred(cred))
}

/// `ufs_quota_alloc_blocks2`: charge `change` disk blocks.
pub fn ufs_quota_alloc_blocks2(
    ip: &Inode,
    change: Daddr,
    cred: *const Ucred,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    kq::ufs_quota_alloc_blocks2(ip, change, chkcred(cred), flags)
}

/// `ufs_quota_free_blocks2`: give back `change` disk blocks.
pub fn ufs_quota_free_blocks2(
    ip: &Inode,
    change: Daddr,
    cred: *const Ucred,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    kq::ufs_quota_free_blocks2(ip, change, Cred::from_raw(cred), flags)
}

/// `ufs_quota_alloc_inode2`: charge an inode.
pub fn ufs_quota_alloc_inode2(
    ip: &Inode,
    cred: *const Ucred,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    kq::ufs_quota_alloc_inode2(ip, chkcred(cred), flags)
}

/// `ufs_quota_free_inode2`: give back an inode.
pub fn ufs_quota_free_inode2(
    ip: &Inode,
    cred: *const Ucred,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    kq::ufs_quota_free_inode2(ip, Cred::from_raw(cred), flags)
}

/// `quotaoff`: turn off a quota type on a file system.
pub fn quotaoff(p: &Proc, mp: &'static Mount, type_: usize) -> Result<(), Errno> {
    kq::quotaoff(p, mp, vfstoufs(mp), type_)
}

/// `qsync`: write the file system's quota changes to disk.
pub fn qsync(mp: &'static Mount) -> Result<(), Errno> {
    kq::qsync(mp, vfstoufs(mp))
}

/// `ufs_quotactl` (`vfs_quotactl`): do operations associated with quotas.
pub fn ufs_quotactl(
    mp: &'static Mount,
    cmds: i32,
    uid: Uid,
    arg: usize,
    p: &Proc,
) -> Result<(), Errno> {
    kq::ufs_quotactl(mp, vfstoufs(mp), cmds, uid, arg, p)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ufs/quota.h");
        for (name, value) in [
            ("MAXQUOTAS", MAXQUOTAS as i64),
            ("USRQUOTA", USRQUOTA as i64),
            ("GRPQUOTA", GRPQUOTA as i64),
            ("SUBCMDMASK", i64::from(SUBCMDMASK)),
            ("SUBCMDSHIFT", i64::from(SUBCMDSHIFT)),
            ("Q_QUOTAON", i64::from(Q_QUOTAON)),
            ("Q_SYNC", i64::from(Q_SYNC)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
        assert_eq!(qcmd(Q_GETQUOTA, GRPQUOTA as i32), 0x30001);
        assert_eq!(size_of::<Dqblk>(), 32);
    }
}
/* </TESTS> */
