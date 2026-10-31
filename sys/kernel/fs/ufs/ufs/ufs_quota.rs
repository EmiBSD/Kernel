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
//! Disk quotas (`option QUOTA`): the in-core quota of a user or group on a file system
//! (`struct dquot`), the charges every allocation and release of blocks and inodes makes
//! against it, the soft and hard limits with their grace times, and `quotactl(2)`'s
//! commands (`Q_QUOTAON`, `Q_QUOTAOFF`, `Q_GETQUOTA`, `Q_SETQUOTA`, `Q_SETUSE`, `Q_SYNC`).
//!
//! Upstream: sys/ufs/ufs/ufs_quota.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_quota.rs@3c62ede68802
//!
//! A quota file is an array of `struct dqblk` indexed by user or group id. `Q_QUOTAON` opens
//! it and keeps its vnode in `um_quotas[type]`; from then on every inode being written holds
//! a referenced dquot per quota type (`i_dquot[]`, set up by [`getinoquota`]), read from the
//! file by [`dqget`] and written back by [`dqsync`] through `VOP_READ`/`VOP_WRITE` on the
//! quota file's vnode. Unreferenced dquots stay cached in a SipHash-keyed hash and on a free
//! list from which [`dqget`] recycles them.
//!
//! ## Deviations
//! - `dqhashtbl`, `dqhash`, `dqhashkey`, `dqfreelist`, `numdquot` and `desireddquot` are one
//!   [`DqTables`] in the glue's VFS tables (`UfsTables::dq`), made by `ufs_quota_init` (from
//!   `ufs_init`, before any mount), so a host test's file systems start afresh
//!   (`ufs_test_reset` forgets it, the dquots of the old table with it, never freed).
//! - A hash chain is a `hal::list::List` (a tail queue's two links, 16 bytes) where the C's
//!   `LIST_HEAD` is 8, so the table `hashinit` makes in `M_DQUOT` is twice the C's and
//!   `vmstat -m`'s "dquot" MemUse grows by that much; no `diff-openbsd` scenario prints it.
//! - `ufs_quota_alloc_blocks2` and `ufs_quota_alloc_inode2` take the credential as
//!   `Option<&Ucred>`, `None` for the kernel's (`NOCRED`, `FSCRED`, NULL): no limit is
//!   checked for it. The C tests `NOCRED` only in the former and dereferences the pointer
//!   otherwise (LZ's deviation, kept).
//! - [`dqget`] answers `ENOMEM` when `malloc(M_WAITOK)` cannot be satisfied (the C cannot
//!   fail there).
//! - `VFSTOUFS(mp)` is the caller's (`quota.rs`'s deviations): `qsync`, `quotaon`,
//!   `quotaoff` and `ufs_quotactl` take the mount's `struct ufsmount` beside the mount.
//! - `KTRACE` is not configured: `ktrquota` is not called.
//! - The `DIAGNOSTIC` checks (`chkdquot`, the `vfs_isbusy` and `VOP_ISLOCKED` panics) are
//!   under feature `diagnostic`.
//!
//! ## Redesign
//! - LZ's `Dquot` of `Cell`s under one `unsafe impl Sync`, made by `malloc` and an `unsafe`
//!   write and deref (sys/ufs/ufs/ufs_quota.rs:1005-1020 at the pin) -> [`Dquot`]'s members
//!   as relaxed atomic words with LZ's names (the C changes them under the kernel lock, which
//!   every path here holds, and `DQ_LOCK` across the quota file's I/O), the `dqblk` as eight
//!   such words, `dq_vp` a `Link`, `dq_cred` a counted `CredRef` in a `ReadMostly` cell (the
//!   C's `crhold`/`crfree` made the reference's own count); the dquot an `MBox` of `M_DQUOT`
//!   given up for a `&'static` (`MBox::leak`): the C never frees one (ufs_quota.c has no
//!   `free(dq`; recycled through the free list).
//! - LZ's `sys/queue.rs` hash chains and free list in `StaticCell`s, linked in `unsafe`
//!   blocks whose argument was the kernel lock (:1092-1166) -> `hal::list::List`s of the
//!   `'static` dquots (atomic links, a broken precondition a panic): no witness, no lock and
//!   no interrupt level the C does not take; LZ's `dq_hashed` flag (a second removal through
//!   stale links after `dqget`'s read error) is the list entry's own `is_linked`.
//! - The quota file's `dqblk` is moved through a kernel buffer of its 32 bytes
//!   (`Uio::kernel_read`/`kernel_write`) and stored into the words after the read, where the
//!   C points the `iovec` at `dq_dqb`: the dquot is `DQ_LOCK`ed throughout, as the C's is.
//! - `chkdqchg` and `chkiqchg` share the limit decision [`chklimit`]; `setquota` and
//!   `setuse` compute their new `dqblk` in [`setquota_dqblk`] and [`setuse_dqblk`] (LZ's
//!   split, kept: the tables of the tests check them without a file system).

use core::ptr;
use core::sync::atomic::{AtomicI64, AtomicU16, AtomicU32, Ordering};

use hal::link::Link;
use hal::list::{List, ListEntry};
use hal::malloc::{M_WAITOK, MArray, MBox};
use hal::random::arc4random;
use hal::sleep::INFSLP;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::mount::{MNT_QUOTA, VB_NOWAIT, VB_READ};
use hal::sys::types::{Daddr, Off, Uid};
use hal::user::{UserPtr, bytes_of, pod_from_bytes};
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use super::inode::{Inode, vtoi};
use super::quota::{
    Dqblk, GRPQUOTA, INITQFNAMES, MAX_DQ_TIME, MAX_IQ_TIME, MAXQUOTAS, Q_GETQUOTA, Q_QUOTAOFF,
    Q_QUOTAON, Q_SETQUOTA, Q_SETUSE, Q_SYNC, SUBCMDMASK, SUBCMDSHIFT, UFS_QUOTA_FORCE, USRQUOTA,
    UfsQuotaFlags,
};
use super::ufsmount::{QTF_CLOSING, QTF_OPENING, Ufsmount};
use crate::assert::kpanic;
use crate::cred::{CredRef, Ucred};
use crate::file::FileGlue;
use crate::fs::lookup::ndinit;
use crate::fs::mount::{Mount, vfs_busy, vfs_unbusy};
use crate::fs::namei::NiDirp;
use crate::fs::subr::{vfs, vfs_mount_foreach_vnode, vget, vput};
use crate::fs::vnode::{LK_EXCLUSIVE, LK_NOWAIT, LK_RETRY, PINOD, VSYSTEM, Vnode, Vtype, vn_lock};
use crate::fs::vnops::{vn_close, vn_open};
use crate::fs::vops::{Cred, vop_read, vop_unlock, vop_write};
use crate::proc::{ProcGlue, Thread, suser};
use crate::sched::{tsleep_nsec, wakeup_all};
use crate::tc::gettime;
use crate::uio::{Iovec, Uio, UioRw};

/// `M_DQUOT`: the malloc type of the dquots and their hash table (`<sys/malloc.h>`;
/// `vmstat -m`'s "dquot").
pub const M_DQUOT: i32 = 27;

/// `DQ_LOCK`: this quota locked (no MODS).
pub const DQ_LOCK: u16 = 0x01;
/// `DQ_WANT`: wakeup on unlock.
pub const DQ_WANT: u16 = 0x02;
/// `DQ_MOD`: this quota modified since read.
pub const DQ_MOD: u16 = 0x04;
/// `DQ_FAKE`: no limits here, just usage.
pub const DQ_FAKE: u16 = 0x08;
/// `DQ_BLKS`: has been warned about blk limit.
pub const DQ_BLKS: u16 = 0x10;
/// `DQ_INODS`: has been warned about inode limit.
pub const DQ_INODS: u16 = 0x20;

/// `DQUOTINC`: minimum free dquots desired.
const DQUOTINC: i64 = 5;

/// `struct dqblk` as eight relaxed words (`dq_dqb`), read and written whole.
struct DqbWords([AtomicU32; 8]);

impl DqbWords {
    /// All zero, as `malloc(M_ZERO)` returns it.
    const fn new() -> Self {
        Self([const { AtomicU32::new(0) }; 8])
    }

    /// The `dqblk`.
    fn get(&self) -> Dqblk {
        let [bh, bs, cb, ih, is, ci, bt, it] =
            core::array::from_fn(|i| self.0[i].load(Ordering::Relaxed));
        Dqblk {
            dqb_bhardlimit: bh,
            dqb_bsoftlimit: bs,
            dqb_curblocks: cb,
            dqb_ihardlimit: ih,
            dqb_isoftlimit: is,
            dqb_curinodes: ci,
            dqb_btime: bt,
            dqb_itime: it,
        }
    }

    /// `dq->dq_dqb = *b`.
    fn set(&self, b: &Dqblk) {
        let v = [
            b.dqb_bhardlimit,
            b.dqb_bsoftlimit,
            b.dqb_curblocks,
            b.dqb_ihardlimit,
            b.dqb_isoftlimit,
            b.dqb_curinodes,
            b.dqb_btime,
            b.dqb_itime,
        ];
        for (w, v) in self.0.iter().zip(v) {
            w.store(v, Ordering::Relaxed);
        }
    }
}

/// `struct dquot`: the disk usage of a user or group on a filesystem. There is one
/// allocated for each quota that exists on any filesystem for the current user or group. A
/// cache is kept of recently used entries.
///
/// Locks: every member is changed under the kernel lock, `dq_dqb` also under `DQ_LOCK`
/// while the quota file is read or written; the two list entries are the dquot hash's and
/// the free list's, under the kernel lock.
pub struct Dquot<G: ProcGlue> {
    /// `dq_hash`: hash list.
    pub dq_hash: ListEntry<Dquot<G>>,
    /// `dq_freelist`: free list.
    pub dq_freelist: ListEntry<Dquot<G>>,
    /// `dq_flags`: flags, see `DQ_*`.
    dq_flags: AtomicU16,
    /// `dq_type`: quota type of this dquot.
    dq_type: AtomicU16,
    /// `dq_cnt`: count of active references.
    dq_cnt: AtomicU32,
    /// `dq_id`: identifier this applies to.
    dq_id: AtomicU32,
    /// `dq_vp`: file backing this quota.
    dq_vp: Link<Vnode<G>>,
    /// `dq_cred`: credentials for writing file, the dquot's reference (`None` is `NOCRED`).
    dq_cred: ReadMostly<Option<CredRef>>,
    /// `dq_dqb`: actual usage & quotas.
    dq_dqb: DqbWords,
}

impl<G: ProcGlue> Dquot<G> {
    /// A zeroed dquot, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            dq_hash: ListEntry::new(),
            dq_freelist: ListEntry::new(),
            dq_flags: AtomicU16::new(0),
            dq_type: AtomicU16::new(0),
            dq_cnt: AtomicU32::new(0),
            dq_id: AtomicU32::new(0),
            dq_vp: Link::new(),
            dq_cred: ReadMostly::new(None),
            dq_dqb: DqbWords::new(),
        }
    }

    /// `dq_flags`.
    pub fn flags(&self) -> u16 {
        self.dq_flags.load(Ordering::Relaxed)
    }

    /// `dq->dq_flags = f`.
    fn set_flags(&self, f: u16) {
        self.dq_flags.store(f, Ordering::Relaxed);
    }

    /// `dq->dq_flags |= f`.
    fn setflag(&self, f: u16) {
        self.set_flags(self.flags() | f);
    }

    /// `dq->dq_flags &= ~f`.
    fn clrflag(&self, f: u16) {
        self.set_flags(self.flags() & !f);
    }

    /// `dq->dq_flags & f`.
    fn isset(&self, f: u16) -> bool {
        self.flags() & f != 0
    }

    /// `dq_cnt`.
    pub fn cnt(&self) -> u32 {
        self.dq_cnt.load(Ordering::Relaxed)
    }

    /// `dq_id`.
    pub fn id(&self) -> u32 {
        self.dq_id.load(Ordering::Relaxed)
    }

    /// `dq_type`.
    pub fn type_(&self) -> u16 {
        self.dq_type.load(Ordering::Relaxed)
    }

    /// `dq_dqb`: the usage and the limits.
    pub fn dqb(&self) -> Dqblk {
        self.dq_dqb.get()
    }

    /// Changes `dq_dqb` (the `dq_curblocks`, `dq_btime`, ... shorthands on the left of an
    /// assignment).
    fn update(&self, f: impl FnOnce(&mut Dqblk)) {
        let mut b = self.dq_dqb.get();
        f(&mut b);
        self.dq_dqb.set(&b);
    }

    /// The wait channel of the dquot (`tsleep_nsec(dq, ..)`, `wakeup(dq)`).
    fn chan(&self) -> usize {
        ptr::from_ref(self).addr()
    }
}

impl<G: ProcGlue> Default for Dquot<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// A dquot hash chain of the glue `G` (`dq_hash`, `ufs_glue_links!`).
type Chain<G> = List<<<G as ProcGlue>::FileGlue as FileGlue>::DqHash>;
/// The dquot free list of the glue `G` (`dq_freelist`, `ufs_glue_links!`).
type Freelist<G> = List<<<G as ProcGlue>::FileGlue as FileGlue>::DqFree>;

/// `dqhashtbl`, `dqhash`, `dqhashkey`, `dqfreelist`, `numdquot` and `desireddquot`.
pub struct DqTables<G: ProcGlue> {
    /// `dqhashtbl`: the hash chains.
    tbl: MArray<Chain<G>>,
    /// `dqhash`: size of the hash table - 1.
    mask: u64,
    /// `dqhashkey`.
    key: SiphashKey,
    /// `dqfreelist`: the dquot free list.
    freelist: Freelist<G>,
    /// `numdquot`: the dquots allocated.
    numdquot: AtomicI64,
    /// `desireddquot`: the dquots wanted before free ones are recycled.
    desireddquot: AtomicI64,
    /// `initialvnodes`, as `ufs_quota_init` saw it (`dqget`'s bound).
    initialvnodes: i64,
}

impl<G: ProcGlue> DqTables<G> {
    /// The hash chain of the quota file `dqvp` and the identifier `id`.
    fn chain(&self, dqvp: &Vnode<G>, id: u32) -> &Chain<G> {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&ptr::from_ref(dqvp).addr().to_ne_bytes());
        ctx.update(&u64::from(id).to_ne_bytes());
        &self.tbl[(ctx.end() & self.mask) as usize]
    }
}

/// What [`chklimit`] found for a charge against one pair of limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// The charge stays within the limits.
    Under,
    /// The charge crosses the soft limit now: the grace time starts, a warning is printed.
    SoftCrossed,
    /// The charge would exceed the hard limit.
    HardReached,
    /// Over the soft limit for longer than the grace time.
    GraceExpired,
}

/// The dquot tables, when `ufs_quota_init` has made them: `panic(9)` before. `f` must not
/// sleep (the cell's reader).
fn with_dq<G: ProcGlue, R>(f: impl FnOnce(&DqTables<G>) -> R) -> R {
    let t = vfs::<G>().ufs.dq.read();
    match t.as_ref() {
        Some(dt) => f(dt),
        None => kpanic(format_args!("dqget: no table")),
    }
}

/// `dqref`: obtain a reference to a dquot.
pub fn dqref<G: ProcGlue>(dq: &Dquot<G>) {
    dq.dq_cnt.store(dq.cnt() + 1, Ordering::Relaxed);
}

/// `getinoquota`: set up the quotas for an inode.
///
/// This routine completely defines the semantics of quotas. If other criterion want to be
/// used to establish quotas, the `MAXQUOTAS` value in quota.h should be increased, and the
/// additional dquots set up here.
pub fn getinoquota<G: ProcGlue>(ip: &Inode<G>) -> Result<(), Errno> {
    let ump = ip.ump();
    let vp = ip.itov();

    // Set up the user quota based on file uid, then the group quota based on file gid.
    // EINVAL means that quotas are not enabled.
    for (type_, id) in [(USRQUOTA, ip.dip_uid()), (GRPQUOTA, ip.dip_gid())] {
        if ip.i_dquot[type_].is_none() {
            match dqget(Some(vp), id, ump, type_) {
                Ok(dq) => ip.i_dquot[type_].set(Some(dq)),
                Err(Errno::EINVAL) => {}
                Err(e) => return Err(e),
            }
        }
    }
    Ok(())
}

/// Sleeps until nobody holds `dq`'s `DQ_LOCK` (the C's `while (dq->dq_flags & DQ_LOCK)`
/// loops).
fn dqwait<G: ProcGlue>(dq: &Dquot<G>, pri: i32, wmesg: &'static str) {
    while dq.isset(DQ_LOCK) {
        dq.setflag(DQ_WANT);
        let _ = tsleep_nsec::<G>(dq.chan(), pri, wmesg, INFSLP);
    }
}

/// Whether `flags` excludes quota type `i` (`UFS_QUOTA_NOUID`, `UFS_QUOTA_NOGID`).
fn skiptype(flags: UfsQuotaFlags, i: usize) -> bool {
    flags & (1 << i) != 0
}

/// `ufs_quota_alloc_blocks2`: update disk usage, and take corrective action.
pub fn ufs_quota_alloc_blocks2<G: ProcGlue>(
    ip: &Inode<G>,
    change: Daddr,
    cred: Option<&Ucred>,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    chkdquot(ip);

    if change == 0 {
        return Ok(());
    }

    if flags & UFS_QUOTA_FORCE == 0
        && let Some(c) = cred.filter(|c| c.cr_uid.get() != 0)
    {
        for i in 0..MAXQUOTAS {
            if skiptype(flags, i) || ip.i_dquot[i].is_none() {
                continue;
            }
            chkdqchg(ip, change, c, i)?;
        }
    }
    for i in 0..MAXQUOTAS {
        if skiptype(flags, i) {
            continue;
        }
        let Some(dq) = ip.i_dquot[i].get() else {
            continue;
        };
        dqwait(dq, PINOD + 1, "chkdq");
        dq.update(|b| b.dqb_curblocks = (i64::from(b.dqb_curblocks) + change) as u32);
        dq.setflag(DQ_MOD);
    }
    Ok(())
}

/// `ufs_quota_free_blocks2`: give back `change` disk blocks.
pub fn ufs_quota_free_blocks2<G: ProcGlue>(
    ip: &Inode<G>,
    change: Daddr,
    _cred: Cred<'_>,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if crate::fs::vops::vop_islocked(ip.itov()) == 0 {
        kpanic(format_args!("ufs_quota_free_blocks2: vnode is not locked"));
    }

    if change == 0 {
        return Ok(());
    }

    for i in 0..MAXQUOTAS {
        if skiptype(flags, i) {
            continue;
        }
        let Some(dq) = ip.i_dquot[i].get() else {
            continue;
        };
        dqwait(dq, PINOD + 1, "chkdq");
        dq.update(|b| {
            b.dqb_curblocks = if i64::from(b.dqb_curblocks) >= change {
                (i64::from(b.dqb_curblocks) - change) as u32
            } else {
                0
            };
        });
        dq.clrflag(DQ_BLKS);
        dq.setflag(DQ_MOD);
    }
    Ok(())
}

/// The limit test of `chkdqchg` and `chkiqchg`: charging `change` to a usage of `cur` with
/// a `hard` and a `soft` limit (0: none), whose grace time ends at `deadline`, at `now`.
pub fn chklimit(cur: u32, change: i64, hard: u32, soft: u32, deadline: u32, now: i64) -> Limit {
    let ncur = i64::from(cur) + change;

    // If user would exceed their hard limit, disallow the allocation.
    if ncur >= i64::from(hard) && hard != 0 {
        return Limit::HardReached;
    }
    // If user is over their soft limit for too long, disallow the allocation. Reset time
    // limit as they cross their soft limit.
    if ncur >= i64::from(soft) && soft != 0 {
        if cur < soft {
            return Limit::SoftCrossed;
        }
        if now > i64::from(deadline) {
            return Limit::GraceExpired;
        }
    }
    Limit::Under
}

/// Which of `chkdqchg`'s and `chkiqchg`'s three messages.
#[derive(Clone, Copy)]
enum Msg {
    /// "write failed, ... limit reached".
    Reached,
    /// "warning, ... quota exceeded".
    Warning,
    /// "write failed, ... quota exceeded for too long".
    TooLong,
}

/// `uprintf(9)` of a quota message on the running thread's terminal, the mount point's name
/// of `ip` first (`ITOV(ip)->v_mount->mnt_stat.f_mntonname`); `what` names the limit ("disk",
/// "inode").
fn quotamsg<G: ProcGlue>(ip: &Inode<G>, msg: Msg, type_: usize, what: &str) {
    let (name, len) = ip.mount().mntonname();
    let n = core::str::from_utf8(&name[..len]).unwrap_or("?");
    let q = INITQFNAMES[type_];
    G::with_curproc(|p| {
        let Some(p) = p else {
            return;
        };
        let pr = p.process();
        match msg {
            Msg::Reached => {
                crate::tty::uprintf(
                    pr,
                    format_args!("\n{n}: write failed, {q} {what} limit reached\n"),
                );
            }
            Msg::Warning => {
                crate::tty::uprintf(
                    pr,
                    format_args!("\n{n}: warning, {q} {what} quota exceeded\n"),
                );
            }
            Msg::TooLong => crate::tty::uprintf(
                pr,
                format_args!("\n{n}: write failed, {q} {what} quota exceeded for too long\n"),
            ),
        }
    });
}

/// The messages and flags of a charge [`chklimit`] judged: `what` names the limit ("disk",
/// "inode"), `warned` the flag that says the owner was told (`DQ_BLKS`, `DQ_INODS`), and
/// `set_deadline` starts the grace time.
#[allow(clippy::too_many_arguments)] // the C's two copies of this body, joined
fn chkchg<G: ProcGlue>(
    ip: &Inode<G>,
    dq: &Dquot<G>,
    limit: Limit,
    owner: bool,
    type_: usize,
    what: &str,
    warned: u16,
    set_deadline: impl FnOnce(&mut Dqblk),
) -> Result<(), Errno> {
    match limit {
        Limit::Under => Ok(()),
        Limit::HardReached => {
            if !dq.isset(warned) && owner {
                quotamsg(ip, Msg::Reached, type_, what);
                dq.setflag(warned);
            }
            Err(Errno::EDQUOT)
        }
        Limit::SoftCrossed => {
            dq.update(set_deadline);
            if owner {
                quotamsg(ip, Msg::Warning, type_, what);
            }
            Ok(())
        }
        Limit::GraceExpired => {
            if !dq.isset(warned) && owner {
                quotamsg(ip, Msg::TooLong, type_, what);
                dq.setflag(warned);
            }
            Err(Errno::EDQUOT)
        }
    }
}

/// `chkdqchg`: check for a valid change to a users allocation. Issue an error message if
/// appropriate.
fn chkdqchg<G: ProcGlue>(
    ip: &Inode<G>,
    change: i64,
    cred: &Ucred,
    type_: usize,
) -> Result<(), Errno> {
    let Some(dq) = ip.i_dquot[type_].get() else {
        return Ok(());
    };
    let b = dq.dqb();
    let now = gettime();
    let limit = chklimit(
        b.dqb_curblocks,
        change,
        b.dqb_bhardlimit,
        b.dqb_bsoftlimit,
        b.dqb_btime,
        now,
    );
    let grace = ip.ump().um_btime[type_].load(Ordering::Relaxed);
    let owner = ip.dip_uid() == cred.cr_uid.get();
    chkchg(ip, dq, limit, owner, type_, "disk", DQ_BLKS, |b| {
        b.dqb_btime = (now + grace) as u32;
    })
}

/// `ufs_quota_alloc_inode2`: check the inode limit, applying corrective action.
pub fn ufs_quota_alloc_inode2<G: ProcGlue>(
    ip: &Inode<G>,
    cred: Option<&Ucred>,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    chkdquot(ip);

    if flags & UFS_QUOTA_FORCE == 0
        && let Some(c) = cred.filter(|c| c.cr_uid.get() != 0)
    {
        for i in 0..MAXQUOTAS {
            if skiptype(flags, i) || ip.i_dquot[i].is_none() {
                continue;
            }
            chkiqchg(ip, 1, c, i)?;
        }
    }
    for i in 0..MAXQUOTAS {
        if skiptype(flags, i) {
            continue;
        }
        let Some(dq) = ip.i_dquot[i].get() else {
            continue;
        };
        dqwait(dq, PINOD + 1, "chkiq");
        dq.update(|b| b.dqb_curinodes = b.dqb_curinodes.wrapping_add(1));
        dq.setflag(DQ_MOD);
    }
    Ok(())
}

/// `ufs_quota_free_inode2`: give back an inode.
pub fn ufs_quota_free_inode2<G: ProcGlue>(
    ip: &Inode<G>,
    _cred: Cred<'_>,
    flags: UfsQuotaFlags,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if crate::fs::vops::vop_islocked(ip.itov()) == 0 {
        kpanic(format_args!("ufs_quota_free_blocks2: vnode is not locked"));
    }

    for i in 0..MAXQUOTAS {
        if skiptype(flags, i) {
            continue;
        }
        let Some(dq) = ip.i_dquot[i].get() else {
            continue;
        };
        dqwait(dq, PINOD + 1, "chkiq");
        dq.update(|b| {
            if b.dqb_curinodes > 0 {
                b.dqb_curinodes -= 1;
            }
        });
        dq.clrflag(DQ_INODS);
        dq.setflag(DQ_MOD);
    }
    Ok(())
}

/// `chkiqchg`: check for a valid change to a users allocation. Issue an error message if
/// appropriate.
fn chkiqchg<G: ProcGlue>(
    ip: &Inode<G>,
    change: i64,
    cred: &Ucred,
    type_: usize,
) -> Result<(), Errno> {
    let Some(dq) = ip.i_dquot[type_].get() else {
        return Ok(());
    };
    let b = dq.dqb();
    let now = gettime();
    let limit = chklimit(
        b.dqb_curinodes,
        change,
        b.dqb_ihardlimit,
        b.dqb_isoftlimit,
        b.dqb_itime,
        now,
    );
    let grace = ip.ump().um_itime[type_].load(Ordering::Relaxed);
    let owner = ip.dip_uid() == cred.cr_uid.get();
    chkchg(ip, dq, limit, owner, type_, "inode", DQ_INODS, |b| {
        b.dqb_itime = (now + grace) as u32;
    })
}

/// `chkdquot`: on filesystems with quotas enabled, it is an error for a file to change size
/// and not to have a dquot structure associated with it.
#[cfg(feature = "diagnostic")]
fn chkdquot<G: ProcGlue>(ip: &Inode<G>) {
    let ump = ip.ump();
    let vp = ip.itov();

    if crate::fs::vops::vop_islocked(vp) == 0 {
        kpanic(format_args!("chkdquot: vnode is not locked"));
    }

    for i in 0..MAXQUOTAS {
        if ump.um_quotas[i].is_none()
            || ump.um_qflags[i].load(Ordering::Relaxed) & (QTF_OPENING | QTF_CLOSING) != 0
        {
            continue;
        }
        if ip.i_dquot[i].is_none() {
            crate::fs::subr::vprint(Some("chkdquot: missing dquot"), vp);
            kpanic(format_args!("missing dquot"));
        }
    }
}

// Code to process quotactl commands.

/// `quotaon_vnode`: give a vnode open for writing its dquots.
fn quotaon_vnode<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if vp.vtype() == Vtype::VNON || vp.writecount() == 0 {
        return Ok(());
    }

    if vget(vp, LK_EXCLUSIVE).is_err() {
        return Ok(());
    }

    let error = getinoquota(&vtoi(vp));
    vput(vp);

    error
}

/// The credential a quota file is read and written with: the holder's, or `NOCRED`.
fn cred_of(c: Option<&CredRef>) -> Cred<'_> {
    c.map_or(Cred::NOCRED, |c| Cred::real(c))
}

/// `quotaon` (`Q_QUOTAON`): set up a quota file for a particular file system. `fname` is
/// the user address of the quota file's path.
pub fn quotaon<G: ProcGlue>(
    p: &Thread<G>,
    mp: &'static Mount<G>,
    ump: &'static Ufsmount<G>,
    type_: usize,
    fname: usize,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if !crate::fs::mount::vfs_isbusy(mp) {
        kpanic(format_args!("quotaon: mount point not busy"));
    }

    let mut nd = ndinit(0, 0, NiDirp::User(fname), p);
    vn_open(&mut nd, FREAD | FWRITE, 0)?;
    let Some(vp) = nd.ni_vp else {
        kpanic(format_args!("quotaon: vn_open returned no vnode"));
    };
    let _ = vop_unlock(vp);
    if vp.vtype() != Vtype::VREG {
        let _ = vn_close(vp, FREAD | FWRITE, Cred::real(p.ucred()), Some(p));
        return Err(Errno::EACCES);
    }

    // Update the vnode and ucred for quota file updates.
    if !ump.um_quotas[type_].points_to(vp) {
        let _ = quotaoff(p, mp, ump, type_);
        ump.um_quotas[type_].set(Some(vp));
        drop(ump.um_cred[type_].replace(Some(p.cred_ref())));
    } else {
        let ocred = ump.um_cred[type_].read().clone();

        let _ = vn_close(vp, FREAD | FWRITE, cred_of(ocred.as_ref()), Some(p));
        let mine = p.cred_ref();
        if !ocred.is_some_and(|o| CredRef::ptr_eq(&o, &mine)) {
            // The slot's old reference goes as it is replaced (`crfree(ocred)`).
            drop(ump.um_cred[type_].replace(Some(mine)));
        }
    }

    ump.um_qflags[type_].fetch_or(QTF_OPENING, Ordering::Relaxed);
    mp.flag_set(MNT_QUOTA);
    vp.flag_set(VSYSTEM);

    // Set up the time limits for this quota.
    ump.um_btime[type_].store(MAX_DQ_TIME, Ordering::Relaxed);
    ump.um_itime[type_].store(MAX_IQ_TIME, Ordering::Relaxed);
    if let Ok(dq) = dqget(None, 0, ump, type_) {
        let b = dq.dqb();
        if b.dqb_btime > 0 {
            ump.um_btime[type_].store(i64::from(b.dqb_btime), Ordering::Relaxed);
        }
        if b.dqb_itime > 0 {
            ump.um_itime[type_].store(i64::from(b.dqb_itime), Ordering::Relaxed);
        }
        dqrele(None, Some(dq));
    }

    // Search vnodes associated with this mount point, adding references to quota file being
    // opened. NB: only need to add dquot's for inodes being modified.
    let error = vfs_mount_foreach_vnode(mp, &mut |vp| quotaon_vnode(vp));

    ump.um_qflags[type_].fetch_and(!QTF_OPENING, Ordering::Relaxed);
    if error.is_err() {
        let _ = quotaoff(p, mp, ump, type_);
    }
    error
}

/// `quotaoff_vnode`: drop a vnode's dquot of the type being turned off.
fn quotaoff_vnode<G: ProcGlue>(vp: &'static Vnode<G>, type_: usize) -> Result<(), Errno> {
    if vp.vtype() == Vtype::VNON {
        return Ok(());
    }

    if vget(vp, LK_EXCLUSIVE).is_err() {
        return Ok(());
    }
    let ip = vtoi(vp);
    let dq = ip.i_dquot[type_].get();
    ip.i_dquot[type_].set(None);
    dqrele(Some(vp), dq);
    drop(ip);
    vput(vp);
    Ok(())
}

/// `quotaoff` (`Q_QUOTAOFF`): turn off disk quotas for a filesystem.
pub fn quotaoff<G: ProcGlue>(
    p: &Thread<G>,
    mp: &'static Mount<G>,
    ump: &'static Ufsmount<G>,
    type_: usize,
) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if !crate::fs::mount::vfs_isbusy(mp) {
        kpanic(format_args!("quotaoff: mount point not busy"));
    }
    let Some(qvp) = ump.um_quotas[type_].get() else {
        return Ok(());
    };
    ump.um_qflags[type_].fetch_or(QTF_CLOSING, Ordering::Relaxed);
    // Search vnodes associated with this mount point, deleting any references to quota file
    // being closed.
    let _ = vfs_mount_foreach_vnode(mp, &mut |vp| quotaoff_vnode(vp, type_));

    let error = vn_close(qvp, FREAD | FWRITE, Cred::real(p.ucred()), Some(p));
    ump.um_quotas[type_].set(None);
    // crfree(um_cred[type]); um_cred[type] = NOCRED: the mount's reference goes.
    drop(ump.um_cred[type_].replace(None));
    ump.um_qflags[type_].fetch_and(!QTF_CLOSING, Ordering::Relaxed);
    if ump.um_quotas.iter().all(Link::is_none) {
        mp.flag_clr(MNT_QUOTA);
    }
    error
}

/// `getquota` (`Q_GETQUOTA`): return current values in a dqblk structure at the user
/// address `addr`.
fn getquota<G: ProcGlue>(
    ump: &'static Ufsmount<G>,
    id: u32,
    type_: usize,
    addr: usize,
) -> Result<(), Errno> {
    let dq = dqget(None, id, ump, type_)?;
    let error = UserPtr::<Dqblk>::new(addr).copyout(&dq.dqb());
    // KTRACE (KTR_STRUCT, ktrquota): not configured.

    dqrele(None, Some(dq));
    error
}

/// `setquota`'s new `dq_dqb`: all of `newlim` but the current values (and the grace times
/// of an id other than 0), with a grace time started if the user had no soft limit or was
/// under it, and is now over the new one; and the new `dq_flags`.
pub fn setquota_dqblk(
    old: Dqblk,
    flags: u16,
    id: u32,
    mut newlim: Dqblk,
    now: i64,
    bgrace: i64,
    igrace: i64,
) -> (Dqblk, u16) {
    // Copy all but the current values. Reset time limit if previously had no soft limit or
    // were under it, but now have a soft limit and are over it.
    newlim.dqb_curblocks = old.dqb_curblocks;
    newlim.dqb_curinodes = old.dqb_curinodes;
    if id != 0 {
        newlim.dqb_btime = old.dqb_btime;
        newlim.dqb_itime = old.dqb_itime;
    }
    if newlim.dqb_bsoftlimit != 0
        && old.dqb_curblocks >= newlim.dqb_bsoftlimit
        && (old.dqb_bsoftlimit == 0 || old.dqb_curblocks < old.dqb_bsoftlimit)
    {
        newlim.dqb_btime = (now + bgrace) as u32;
    }
    if newlim.dqb_isoftlimit != 0
        && old.dqb_curinodes >= newlim.dqb_isoftlimit
        && (old.dqb_isoftlimit == 0 || old.dqb_curinodes < old.dqb_isoftlimit)
    {
        newlim.dqb_itime = (now + igrace) as u32;
    }
    let b = newlim;
    let mut flags = flags;
    if b.dqb_curblocks < b.dqb_bsoftlimit {
        flags &= !DQ_BLKS;
    }
    if b.dqb_curinodes < b.dqb_isoftlimit {
        flags &= !DQ_INODS;
    }
    if b.dqb_isoftlimit == 0
        && b.dqb_bsoftlimit == 0
        && b.dqb_ihardlimit == 0
        && b.dqb_bhardlimit == 0
    {
        flags |= DQ_FAKE;
    } else {
        flags &= !DQ_FAKE;
    }
    (b, flags | DQ_MOD)
}

/// `setquota` (`Q_SETQUOTA`): assign an entire dqblk structure, read from the user address
/// `addr`.
fn setquota<G: ProcGlue>(
    ump: &'static Ufsmount<G>,
    id: u32,
    type_: usize,
    addr: usize,
) -> Result<(), Errno> {
    let newlim = UserPtr::<Dqblk>::new(addr).copyin()?;
    // KTRACE (KTR_STRUCT, ktrquota): not configured.

    let dq = dqget(None, id, ump, type_)?;
    dqwait(dq, PINOD + 1, "setquota");
    let (b, flags) = setquota_dqblk(
        dq.dqb(),
        dq.flags(),
        dq.id(),
        newlim,
        gettime(),
        ump.um_btime[type_].load(Ordering::Relaxed),
        ump.um_itime[type_].load(Ordering::Relaxed),
    );
    dq.dq_dqb.set(&b);
    dq.set_flags(flags);
    dqrele(None, Some(dq));
    Ok(())
}

/// `setuse`'s new `dq_dqb` and `dq_flags`: the current values of `usage`, with a grace time
/// started if the user has a soft limit, was under it, and is now over it.
pub fn setuse_dqblk(
    old: Dqblk,
    flags: u16,
    usage: Dqblk,
    now: i64,
    bgrace: i64,
    igrace: i64,
) -> (Dqblk, u16) {
    let mut b = old;
    // Reset time limit if have a soft limit and were previously under it, but are now over
    // it.
    if b.dqb_bsoftlimit != 0
        && b.dqb_curblocks < b.dqb_bsoftlimit
        && usage.dqb_curblocks >= b.dqb_bsoftlimit
    {
        b.dqb_btime = (now + bgrace) as u32;
    }
    if b.dqb_isoftlimit != 0
        && b.dqb_curinodes < b.dqb_isoftlimit
        && usage.dqb_curinodes >= b.dqb_isoftlimit
    {
        b.dqb_itime = (now + igrace) as u32;
    }
    b.dqb_curblocks = usage.dqb_curblocks;
    b.dqb_curinodes = usage.dqb_curinodes;
    let mut flags = flags;
    if b.dqb_curblocks < b.dqb_bsoftlimit {
        flags &= !DQ_BLKS;
    }
    if b.dqb_curinodes < b.dqb_isoftlimit {
        flags &= !DQ_INODS;
    }
    (b, flags | DQ_MOD)
}

/// `setuse` (`Q_SETUSE`): set current inode and block usage, read from the user address
/// `addr`.
fn setuse<G: ProcGlue>(
    ump: &'static Ufsmount<G>,
    id: u32,
    type_: usize,
    addr: usize,
) -> Result<(), Errno> {
    let usage = UserPtr::<Dqblk>::new(addr).copyin()?;
    // KTRACE (KTR_STRUCT, ktrquota): not configured.

    let dq = dqget(None, id, ump, type_)?;
    dqwait(dq, PINOD + 1, "setuse");
    let (b, flags) = setuse_dqblk(
        dq.dqb(),
        dq.flags(),
        usage,
        gettime(),
        ump.um_btime[type_].load(Ordering::Relaxed),
        ump.um_itime[type_].load(Ordering::Relaxed),
    );
    dq.dq_dqb.set(&b);
    dq.set_flags(flags);
    dqrele(None, Some(dq));
    Ok(())
}

/// `qsync_vnode`: write a vnode's modified dquots.
fn qsync_vnode<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if vp.vtype() == Vtype::VNON {
        return Ok(());
    }

    if vget(vp, LK_EXCLUSIVE | LK_NOWAIT).is_err() {
        return Ok(());
    }

    let ip = vtoi(vp);
    for i in 0..MAXQUOTAS {
        if let Some(dq) = ip.i_dquot[i].get()
            && dq.isset(DQ_MOD)
        {
            let _ = dqsync(Some(vp), dq);
        }
    }
    drop(ip);
    vput(vp);
    Ok(())
}

/// `qsync` (`Q_SYNC`): sync quota files to disk.
pub fn qsync<G: ProcGlue>(mp: &'static Mount<G>, ump: &'static Ufsmount<G>) -> Result<(), Errno> {
    // Check if the mount point has any quotas. If not, simply return.
    if ump.um_quotas.iter().all(Link::is_none) {
        return Ok(());
    }
    // Search vnodes associated with this mount point, synchronizing any modified dquot
    // structures.
    let _ = vfs_mount_foreach_vnode(mp, &mut |vp| qsync_vnode(vp));
    Ok(())
}

// Code pertaining to management of the in-core dquot data structures.

/// `ufs_quota_init`: initialize the quota system: `hashinit(initialvnodes, M_DQUOT,
/// M_WAITOK, &dqhash)`, a random key and an empty free list.
pub fn ufs_quota_init<G: ProcGlue>(initialvnodes: i32) {
    if initialvnodes <= 0 {
        kpanic(format_args!("hashinit: bad cnt"));
    }
    let n = (initialvnodes as usize).next_power_of_two();
    let Some(tbl) = MArray::new(M_DQUOT, M_WAITOK, n, |_| List::new()) else {
        kpanic(format_args!("ufs_quota_init: no memory"));
    };
    // arc4random_buf(&dqhashkey, sizeof(dqhashkey)): the key's 16 bytes, a word at a time.
    let word = || u64::from(arc4random()) << 32 | u64::from(arc4random());
    let dt = DqTables {
        tbl,
        mask: n as u64 - 1,
        key: SiphashKey {
            k0: word(),
            k1: word(),
        },
        freelist: List::new(),
        numdquot: AtomicI64::new(0),
        desireddquot: AtomicI64::new(DQUOTINC),
        initialvnodes: i64::from(initialvnodes),
    };
    if vfs::<G>().ufs.dq.replace(Some(dt)).is_some() {
        kpanic(format_args!("ufs_quota_init: twice"));
    }
}

/// Whether `vp` is `dqvp` (the caller already holds the quota file's lock).
fn same<G: ProcGlue>(vp: Option<&Vnode<G>>, dqvp: &Vnode<G>) -> bool {
    vp.is_some_and(|vp| ptr::eq(vp, dqvp))
}

/// A transfer of `dq_dqb` to or from the quota file at `dq_id`'s slot: the error and the
/// bytes left (`auio.uio_resid`).
fn dqio<G: ProcGlue>(
    dqvp: &'static Vnode<G>,
    dq: &Dquot<G>,
    rw: UioRw,
) -> (Result<(), Errno>, usize) {
    let mut buf = [0u8; size_of::<Dqblk>()];
    buf.copy_from_slice(bytes_of(&[dq.dqb()]));
    let offset = Off::from(dq.id()) * size_of::<Dqblk>() as Off;
    let cred = dq.dq_cred.read().clone();
    let cred = cred_of(cred.as_ref());
    let mut aiov = [Iovec::new()];
    let (error, resid) = if rw == UioRw::UIO_READ {
        let mut auio = Uio::<G>::kernel_read(&mut aiov, &mut buf, None);
        auio.uio_offset = offset;
        let e = vop_read(dqvp, &mut auio, 0, cred);
        (e, auio.uio_resid)
    } else {
        let mut auio = Uio::<G>::kernel_write(&mut aiov, &buf, None);
        auio.uio_offset = offset;
        let e = vop_write(dqvp, &mut auio, 0, cred);
        (e, auio.uio_resid)
    };
    if rw == UioRw::UIO_READ
        && let Some(b) = pod_from_bytes::<Dqblk>(&buf)
    {
        dq.dq_dqb.set(&b);
    }
    (error, resid)
}

/// `dqget`: obtain a dquot structure for the specified identifier and quota file reading
/// the information from the file if necessary. `vp` is the vnode the caller holds locked,
/// if any. `EINVAL` when the quota type is not enabled.
pub fn dqget<G: ProcGlue>(
    vp: Option<&'static Vnode<G>>,
    id: u32,
    ump: &'static Ufsmount<G>,
    type_: usize,
) -> Result<&'static Dquot<G>, Errno> {
    let Some(dqvp) = ump.um_quotas[type_].get() else {
        return Err(Errno::EINVAL);
    };
    if ump.um_qflags[type_].load(Ordering::Relaxed) & QTF_CLOSING != 0 {
        return Err(Errno::EINVAL);
    }
    // Check the cache first.
    let hit = with_dq::<G, _>(|dt| {
        let dq = dt
            .chain(dqvp, id)
            .iter()
            .find(|dq| dq.id() == id && dq.dq_vp.points_to(dqvp))?;
        // Cache hit with no references. Take the structure off the free list.
        if dq.cnt() == 0 {
            dt.freelist.remove(dq);
        }
        dqref(dq);
        Some(dq)
    });
    if let Some(dq) = hit {
        return Ok(dq);
    }
    // Not in cache, allocate a new one.
    let fresh = with_dq::<G, _>(|dt| {
        let numdquot = dt.numdquot.load(Ordering::Relaxed);
        if dt.freelist.is_empty() && numdquot < MAXQUOTAS as i64 * dt.initialvnodes {
            dt.desireddquot.fetch_add(DQUOTINC, Ordering::Relaxed);
        }
        numdquot < dt.desireddquot.load(Ordering::Relaxed)
    });
    let dq: &'static Dquot<G> = if fresh {
        // malloc(sizeof *dq, M_DQUOT, M_WAITOK | M_ZERO), never freed (may sleep: outside
        // the tables' cell).
        let Some(b) = MBox::new(M_DQUOT, M_WAITOK, Dquot::new()) else {
            return Err(Errno::ENOMEM);
        };
        with_dq::<G, _>(|dt| dt.numdquot.fetch_add(1, Ordering::Relaxed));
        MBox::leak(b)
    } else {
        let recycled = with_dq::<G, _>(|dt| {
            let dq = dt.freelist.first()?;
            if dq.cnt() != 0 || dq.isset(DQ_MOD) {
                kpanic(format_args!("free dquot isn't"));
            }
            dt.freelist.remove(dq);
            if dq.dq_hash.is_linked() {
                let ovp = dq.dq_vp.get();
                if let Some(ovp) = ovp {
                    dt.chain(ovp, dq.id()).remove(dq);
                }
            }
            Some(dq)
        });
        let Some(dq) = recycled else {
            <G::FileGlue as FileGlue>::tablefull("dquot");
            return Err(Errno::EUSERS);
        };
        // crfree(dq->dq_cred); dq->dq_cred = NOCRED.
        drop(dq.dq_cred.replace(None));
        dq
    };
    // Initialize the contents of the dquot structure.
    if !same(vp, dqvp) {
        let _ = vn_lock(dqvp, LK_EXCLUSIVE | LK_RETRY);
    }
    with_dq::<G, _>(|dt| dt.chain(dqvp, id).insert_head(dq));
    dqref(dq);
    dq.set_flags(DQ_LOCK);
    dq.dq_id.store(id, Ordering::Relaxed);
    dq.dq_vp.set(Some(dqvp));
    dq.dq_type.store(type_ as u16, Ordering::Relaxed);
    let cr = ump.um_cred[type_].read().clone();
    drop(dq.dq_cred.replace(cr));
    let (error, resid) = dqio(dqvp, dq, UioRw::UIO_READ);
    if resid == size_of::<Dqblk>() && error.is_ok() {
        dq.dq_dqb.set(&Dqblk::default());
    }
    if !same(vp, dqvp) {
        let _ = vop_unlock(dqvp);
    }
    if dq.isset(DQ_WANT) {
        wakeup_all::<G>(dq.chan());
    }
    dq.set_flags(0);
    // I/O error in reading quota file, release quota structure and reflect problem to
    // caller.
    if let Err(e) = error {
        with_dq::<G, _>(|dt| dt.chain(dqvp, id).remove(dq));
        dqrele(vp, Some(dq));
        return Err(e);
    }
    // Check for no limit to enforce. Initialize time values if necessary.
    let b = dq.dqb();
    if b.dqb_isoftlimit == 0
        && b.dqb_bsoftlimit == 0
        && b.dqb_ihardlimit == 0
        && b.dqb_bhardlimit == 0
    {
        dq.setflag(DQ_FAKE);
    }
    if dq.id() != 0 {
        let now = gettime();
        dq.update(|b| {
            if b.dqb_btime == 0 {
                b.dqb_btime = (now + ump.um_btime[type_].load(Ordering::Relaxed)) as u32;
            }
            if b.dqb_itime == 0 {
                b.dqb_itime = (now + ump.um_itime[type_].load(Ordering::Relaxed)) as u32;
            }
        });
    }
    Ok(dq)
}

/// `dqrele`: release a reference to a dquot. `vp` is the vnode the caller holds locked, if
/// any.
pub fn dqrele<G: ProcGlue>(vp: Option<&'static Vnode<G>>, dq: Option<&'static Dquot<G>>) {
    let Some(dq) = dq else {
        return;
    };
    if dq.cnt() > 1 {
        dq.dq_cnt.store(dq.cnt() - 1, Ordering::Relaxed);
        return;
    }
    if dq.isset(DQ_MOD) {
        let _ = dqsync(vp, dq);
    }
    dq.dq_cnt.store(dq.cnt() - 1, Ordering::Relaxed);
    if dq.cnt() > 0 {
        return;
    }
    with_dq::<G, _>(|dt| dt.freelist.insert_tail(dq));
}

/// `dqsync`: update the disk quota in the quota file. `vp` is the vnode the caller holds
/// locked, if any.
pub fn dqsync<G: ProcGlue>(
    vp: Option<&'static Vnode<G>>,
    dq: &'static Dquot<G>,
) -> Result<(), Errno> {
    if !dq.isset(DQ_MOD) {
        return Ok(());
    }
    let Some(dqvp) = dq.dq_vp.get() else {
        kpanic(format_args!("dqsync: file"));
    };

    if !same(vp, dqvp) {
        let _ = vn_lock(dqvp, LK_EXCLUSIVE | LK_RETRY);
    }
    while dq.isset(DQ_LOCK) {
        dq.setflag(DQ_WANT);
        let _ = tsleep_nsec::<G>(dq.chan(), PINOD + 2, "dqsync", INFSLP);
        if !dq.isset(DQ_MOD) {
            if !same(vp, dqvp) {
                let _ = vop_unlock(dqvp);
            }
            return Ok(());
        }
    }
    dq.setflag(DQ_LOCK);
    let (mut error, resid) = dqio(dqvp, dq, UioRw::UIO_WRITE);
    if resid != 0 && error.is_ok() {
        error = Err(Errno::EIO);
    }
    if dq.isset(DQ_WANT) {
        wakeup_all::<G>(dq.chan());
    }
    dq.clrflag(DQ_MOD | DQ_LOCK | DQ_WANT);
    if !same(vp, dqvp) {
        let _ = vop_unlock(dqvp);
    }
    error
}

/// `ufs_quota_delete`: release the inode's dquots.
pub fn ufs_quota_delete<G: ProcGlue>(ip: &Inode<G>) -> Result<(), Errno> {
    let vp = ip.itov();
    for i in 0..MAXQUOTAS {
        if let Some(dq) = ip.i_dquot[i].get() {
            ip.i_dquot[i].set(None);
            dqrele(Some(vp), Some(dq));
        }
    }

    Ok(())
}

/// `ufs_quotactl` (`vfs_quotactl`): do operations associated with quotas. `arg` is a user
/// address.
pub fn ufs_quotactl<G: ProcGlue>(
    mp: &'static Mount<G>,
    ump: &'static Ufsmount<G>,
    cmds: i32,
    uid: Uid,
    arg: usize,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ruid = p.ucred().cr_ruid.get();
    let uid = if uid == Uid::MAX { ruid } else { uid };
    let cmd = cmds >> SUBCMDSHIFT;

    match cmd {
        Q_SYNC => {}
        Q_GETQUOTA if uid == ruid => {}
        _ => suser(p)?,
    }

    let type_ = (cmds & SUBCMDMASK) as u32 as usize;
    if type_ >= MAXQUOTAS {
        return Err(Errno::EINVAL);
    }

    if vfs_busy(mp, VB_READ | VB_NOWAIT).is_err() {
        return Ok(());
    }

    let error = match cmd {
        Q_QUOTAON => quotaon(p, mp, ump, type_, arg),
        Q_QUOTAOFF => quotaoff(p, mp, ump, type_),
        Q_SETQUOTA => setquota(ump, uid, type_, arg),
        Q_SETUSE => setuse(ump, uid, type_, arg),
        Q_GETQUOTA => getquota(ump, uid, type_, arg),
        Q_SYNC => qsync(mp, ump),
        _ => Err(Errno::EINVAL),
    };

    vfs_unbusy(mp);
    error
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;
    const WEEK: i64 = 7 * 24 * 60 * 60;

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

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn dquot_flags_match_the_c_file() {
        let defs = crate::reftest::defines("sys/ufs/ufs/ufs_quota.c");
        for (name, value) in [
            ("DQ_LOCK", DQ_LOCK),
            ("DQ_WANT", DQ_WANT),
            ("DQ_MOD", DQ_MOD),
            ("DQ_FAKE", DQ_FAKE),
            ("DQ_BLKS", DQ_BLKS),
            ("DQ_INODS", DQ_INODS),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value)),
                "{name}"
            );
        }
        assert_eq!(crate::reftest::int(&defs, "DQUOTINC"), Some(DQUOTINC));
    }

    #[test]
    fn chklimit_hard_soft_and_grace() {
        // No limits: anything goes.
        assert_eq!(chklimit(1000, 1000, 0, 0, 0, NOW), Limit::Under);
        // The hard limit is reached when the new usage gets to it (">=").
        assert_eq!(chklimit(90, 9, 100, 0, 0, NOW), Limit::Under);
        assert_eq!(chklimit(90, 10, 100, 0, 0, NOW), Limit::HardReached);
        assert_eq!(chklimit(0, 200, 100, 0, 0, NOW), Limit::HardReached);
        // Crossing the soft limit starts the grace time (allowed, with a warning).
        assert_eq!(chklimit(40, 10, 100, 50, 0, NOW), Limit::SoftCrossed);
        assert_eq!(chklimit(40, 9, 100, 50, 0, NOW), Limit::Under);
        // Over it already: allowed until the deadline, refused after it.
        let deadline = (NOW + 10) as u32;
        assert_eq!(chklimit(60, 1, 100, 50, deadline, NOW), Limit::Under);
        assert_eq!(chklimit(60, 1, 100, 50, deadline, NOW + 10), Limit::Under);
        assert_eq!(
            chklimit(60, 1, 100, 50, deadline, NOW + 11),
            Limit::GraceExpired
        );
        // The hard limit wins over the soft one.
        assert_eq!(chklimit(60, 50, 100, 50, deadline, NOW), Limit::HardReached);
        // Giving back is never refused by the soft limit's grace when under it.
        assert_eq!(
            chklimit(60, -20, 100, 50, deadline, NOW + 100),
            Limit::Under
        );
    }

    #[test]
    fn setquota_keeps_usage_and_starts_grace() {
        // The current values come from the old dquot, not from the caller.
        let old = dqblk(0, 0, 70, 0, 0, 5);
        let (b, f) = setquota_dqblk(old, 0, 1000, dqblk(100, 50, 0, 10, 4, 0), NOW, WEEK, WEEK);
        assert_eq!((b.dqb_curblocks, b.dqb_curinodes), (70, 5));
        assert_eq!((b.dqb_bhardlimit, b.dqb_bsoftlimit), (100, 50));
        // Over both new soft limits with none before: both grace times start now.
        assert_eq!(b.dqb_btime, (NOW + WEEK) as u32);
        assert_eq!(b.dqb_itime, (NOW + WEEK) as u32);
        assert_eq!(f, DQ_MOD);

        // Already over the old soft limit: the old grace time is kept for an id but 0.
        let mut old = dqblk(100, 50, 70, 0, 0, 0);
        old.dqb_btime = 1234;
        let (b, _) = setquota_dqblk(old, DQ_BLKS, 7, dqblk(100, 60, 0, 0, 0, 0), NOW, WEEK, WEEK);
        assert_eq!(b.dqb_btime, 1234);
        // Id 0 holds the grace times themselves: the caller's are taken.
        let mut lim = dqblk(0, 0, 0, 0, 0, 0);
        lim.dqb_btime = 99;
        lim.dqb_itime = 98;
        let (b, f) = setquota_dqblk(old, DQ_BLKS, 0, lim, NOW, WEEK, WEEK);
        assert_eq!((b.dqb_btime, b.dqb_itime), (99, 98));
        // No limits at all: just usage (DQ_FAKE); a soft limit of 0 clears no warning (the
        // C's `curblocks < bsoftlimit`), one the usage is under clears it.
        assert_eq!(f, DQ_BLKS | DQ_MOD | DQ_FAKE);
        let (_, f) = setquota_dqblk(
            dqblk(0, 0, 10, 0, 0, 1),
            DQ_BLKS | DQ_INODS | DQ_FAKE,
            3,
            dqblk(100, 50, 0, 10, 5, 0),
            NOW,
            WEEK,
            WEEK,
        );
        assert_eq!(f, DQ_MOD);
    }

    #[test]
    fn setuse_sets_usage_and_starts_grace_on_crossing() {
        let old = dqblk(100, 50, 40, 10, 5, 2);
        // Crossing both soft limits: both grace times start.
        let (b, f) = setuse_dqblk(
            old,
            DQ_BLKS | DQ_INODS,
            dqblk(0, 0, 60, 0, 0, 7),
            NOW,
            WEEK,
            2 * WEEK,
        );
        assert_eq!((b.dqb_curblocks, b.dqb_curinodes), (60, 7));
        assert_eq!(b.dqb_btime, (NOW + WEEK) as u32);
        assert_eq!(b.dqb_itime, (NOW + 2 * WEEK) as u32);
        assert_eq!((b.dqb_bhardlimit, b.dqb_isoftlimit), (100, 5));
        assert_eq!(f, DQ_BLKS | DQ_INODS | DQ_MOD);
        // Back under the soft limits: the warnings clear, the times stay.
        let (b2, f) = setuse_dqblk(b, f, dqblk(0, 0, 10, 0, 0, 1), NOW + 5, WEEK, WEEK);
        assert_eq!((b2.dqb_btime, b2.dqb_itime), (b.dqb_btime, b.dqb_itime));
        assert_eq!(f, DQ_MOD);
    }

    #[test]
    fn the_dqblk_words_hold_the_record_whole() {
        let w = DqbWords::new();
        assert_eq!(w.get(), Dqblk::default());
        let mut b = dqblk(1, 2, 3, 4, 5, 6);
        b.dqb_btime = 7;
        b.dqb_itime = u32::MAX;
        w.set(&b);
        assert_eq!(w.get(), b);
        let dq = Dquot::<crate::fs::testfs::TestGlue>::new();
        dq.update(|b| b.dqb_curinodes = 9);
        assert_eq!(dq.dqb().dqb_curinodes, 9);
        dq.setflag(DQ_MOD | DQ_LOCK);
        dq.clrflag(DQ_LOCK);
        assert!(dq.isset(DQ_MOD) && !dq.isset(DQ_LOCK));
    }
}
/* </TESTS> */
