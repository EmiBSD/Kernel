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

/*-
 * Copyright (C) 1994, 1995, 1997 Wolfgang Solfrank.
 * Copyright (C) 1994, 1995, 1997 TooLs GmbH.
 * All rights reserved.
 * Original code by Paul Popelka (paulp@uts.amdahl.com) (see below).
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by TooLs GmbH.
 * 4. The name of TooLs GmbH may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY TOOLS GMBH ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL TOOLS GMBH BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/*
 * Written by Paul Popelka (paulp@uts.amdahl.com)
 *
 * You can do anything you want with this software, just don't say you wrote
 * it, and don't remove this notice.
 *
 * This software is provided "as is".
 *
 * The author supplies this software to be publicly redistributed on the
 * understanding that the author is not responsible for the correct
 * functioning of this software in any circumstances and is not liable for
 * any damages caused by this software.
 *
 * October 1992
 */
/* </LICENSES> */

/* <CODE> */
//! The denode cache and the life of a denode: `msdosfs_init` (the hash table), `deget` (find
//! or make the denode of a directory entry, with its vnode), `deupdat` (write the times and
//! size back to the entry), `detrunc` and `deextend` (change a file's length), `reinsert`
//! (rehash after a rename), and the vnode operations `msdosfs_reclaim` and
//! `msdosfs_inactive`.
//!
//! Upstream: sys/msdosfs/msdosfs_denode.c @ 3ce1f3f79392
//! LZ: sys/msdosfs/msdosfs_denode.rs@3c62ede68802
//!
//! The chains are picked by a SipHash of the device and the entry's position under a random
//! key, so that an attacker cannot predict which denodes collide.
//!
//! ## Deviations
//! - `dehashtbl`, `dehash` (the table's size - 1) and `dehashkey` are one [`Dehash`] in the
//!   glue's VFS tables ([`MsdosfsTables`], `VfsTables::msdosfs`), made by `msdosfs_init` (from
//!   `vfsinit`, before any mount); `msdosfs_dehash` (`DEHASH`) is [`Dehash::msdosfs_dehash`],
//!   the table's own.
//! - The chains are pinned lists of the denodes (`hal::pinlist`, through `de_hash`); the C's
//!   `de_prev == NULL` (a denode on no chain) is the entry's own state, which
//!   `msdosfs_hashrem` clears always (the C only under `DIAGNOSTIC`), so that a denode
//!   unhashed twice (a `reinsert` whose `msdosfs_hashins` failed, then `msdosfs_reclaim`) is
//!   not unlinked twice. A denode remembers the chain it is on ([`Denode::de_chain`]), as the
//!   C's `de_prev` points into it: `msdosfs_rename` changes the entry's position before
//!   `reinsert` takes the denode off its old chain.
//! - `deget` returns the denode where the C fills `*depp` (and NULLs it on error). As in the
//!   C, a failing `readep` leaves the new vnode locked, referenced and hashed.
//! - The FAT12/16 root directory's manufactured size is `pm_rootdirsize * DEV_BSIZE`, the
//!   bytes the root occupies (`pm_rootdirsize` counts `DEV_BSIZE` blocks), where the C's
//!   `pm_rootdirsize * pm_BytesPerSec` overstates it by `pm_BytesPerSec / DEV_BSIZE` for
//!   sectors over 512 bytes: the user's decision D-N5.7-U4 of 2026-11-05 (FreeBSD's value).
//!   Every reader of the root's `de_FileSize` follows: `pcbmap`'s end of the root, so
//!   `msdosfs_lookup`, `createde`, `dosdirempty`, `uniqdosname` and `msdosfs_readdir` scan the
//!   root and stop at its real end; `msdosfs_create`'s and `msdosfs_mkdir`'s `ENOSPC` for a
//!   full root; `msdosfs_getattr`'s `va_size` (`st_size`) and `va_bytes`; `msdosfs_read` of
//!   the root and `filt_msdosfsread`'s `kn_data`. What userland sees on a FAT12/16 file system
//!   with sectors over 512 bytes: the root's real `st_size`, no phantom `getdents(2)` entries
//!   (the C lists the first data clusters' bytes as root entries), `ENOSPC` for a file created
//!   in a full root (the C writes the new entry into a data cluster); on 512-byte sectors
//!   (every smoke's image) nothing changes.
//! - The denode holds its mount structure from its making (`Denode::new`, msdosfs's
//!   `denode.rs`), where the C sets `de_pmp` only after `msdosfs_hashins`: the `EEXIST`
//!   retry's `vput` runs `msdosfs_inactive` on the fresh denode, whose `detrunc` computes
//!   `de_clcount(pmp, 0)` over the C's NULL `de_pmp` (a NULL dereference on a read-write
//!   mount, a race of two `deget`s of one entry); here that `detrunc` frees nothing (no
//!   cluster, no buffer) and the denode is reclaimed.
//! - The `MSDOSFS_DEBUG` `printf`s are left out: the option is not in GENERIC and has no
//!   feature here.
//! - `msdosfs_reclaim`'s and `msdosfs_inactive`'s `DIAGNOSTIC` `prtactive` reports are behind
//!   feature `diagnostic`.
//! - `msdosfs_vops` and `readep`/`readde` come from `msdosfs_vnops.rs` and
//!   `msdosfs_lookup.rs`.
//!
//! ## Redesign
//! - LZ's three `StaticCell`s and the chains of `sys/queue.rs` `ListHead`s, linked and
//!   unlinked in `unsafe` blocks whose argument was the kernel lock (`Dehashtbl`'s `unsafe
//!   impl Sync`/`Send`, the cells' writes in `msdosfs_init` and reads in `msdosfs_dehash`) ->
//!   [`Dehash`], `ufs_ihash.rs`'s `Ihash` shape (N5.5a1): a `hashinit` table of
//!   `hal::pinlist::PinList`s of the pinned denodes (`Pin<&Denode>` from the counted block,
//!   `hal::malloc::MArc::as_pin`) bound to the kernel lock, in a `ReadMostly` cell of the
//!   glue's tables (`MsdosfsTables::dehash`), so the host tests start each test afresh
//!   (`msdosfs_test_reset`). The adapter of the chains, LZ's `DeHash` (`queue_adapter!` in
//!   `denode.rs`), is [`DeHashLink`], declared here by the generic form of
//!   `hal::pin_adapter!` (no glue associated type, D-N5.7-3). A walk takes
//!   `klock::reader()` (a per-CPU count, no interrupt level), an insertion or a removal
//!   `klock::witness()`, which raises the interrupt level for its section: one spl pair per
//!   `msdosfs_hashins` and per `msdosfs_hashrem` of a hashed denode (once in a denode's life
//!   each, and around a rename's `reinsert`), where the C has none: docs/ROADMAP.md N12's row,
//!   as `ufs_ihash.rs`'s.
//! - `hashinit` and the key are the kernel's shared `fs::subr::hashinit` and `siphash_key`
//!   (`ufs_ihashinit` their other user, D-N5.7-4): the C's `arc4random_buf(&dehashkey, ..)` is
//!   four `arc4random()` words of the same generator.
//! - `msdosfs_hashget` finds the denode's vnode under the reader and drops it before `vget`
//!   (which may sleep), rechecks `v_id` as the C does, and answers the vnode's denode handle
//!   (`vtode`).
//! - `deget` builds the denode whole where the C fills its `malloc(M_ZERO)` block (the vnode,
//!   the device, the mount structure's handle, the entry's position), publishes it in a
//!   counted block (`Denode::alloc`) and hangs a handle from the vnode's typed slot where the
//!   C sets `v_data`; `msdosfs_reclaim` takes the slot's handle back where the C frees the
//!   denode, and the last handle frees the block (a handle an operation holds across a sleep
//!   never dangles). `de_devvp` is set after the insertion, as the C's, and given back by the
//!   reclaim (`vrele`).
//! - LZ's lent buffers and its `unsafe fn bdata` (five `unsafe`) -> `fs::bio`'s `BufRef`
//!   tokens: `readep`/`readde` lend the entry's block with the entry's offset, `deupdat`
//!   writes the entry back with `Direntry::put`, `detrunc` zeroes the cluster's tail in the
//!   token's bytes. 0 `unsafe` (LZ: 14).
//! - Every arithmetic over an on-disk value keeps the C's unsigned widths
//!   (`de_clcount(pmp, length) - 1` and `length - 1` wrap at 32 bits as the C's `uint32_t`).

use core::pin::Pin;

use hal::klock;
use hal::malloc::{M_WAITOK, MArc, MArray};
use hal::pinlist::PinList;
use hal::sleep::INFSLP;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;
use hal::sys::types::{Daddr, Dev};
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use crate::assert::kpanic;
use crate::fs::bio::{bdwrite, bread, brelse, bwrite};
use crate::fs::cache::cache_purge;
use crate::fs::msdosfs::denode::{
    DE_MODIFIED, DE_UPDATE, Denode, DenodeRef, FC_LASTFC, MSDOSFSROOT_OFS, de_externalize,
    de_internalize, detimes, fc_setcache, vtode,
};
use crate::fs::msdosfs::direntry::{ATTR_DIRECTORY, DD_DAY_SHIFT, DD_MONTH_SHIFT, SLOT_DELETED};
use crate::fs::msdosfs::fat::{
    CLUST_END, CLUST_EOFE, DE_CLEAR, FAT_GET_AND_SET, MSDOSFSROOT, fat32, msdosfseof,
};
use crate::fs::msdosfs::msdosfs_fat::{extendfile, fatentry, fc_purge, freeclusterchain, pcbmap};
use crate::fs::msdosfs::msdosfs_lookup::{entry_at, entry_put, readde, readep};
use crate::fs::msdosfs::msdosfs_vnops::MsdosfsVnops;
use crate::fs::msdosfs::msdosfsmount::{
    M_MSDOSFSMNT, MsdosfsmountRef, cntobn, de_blk, de_bn2off, de_clcount, de_cluster, de_cn2off,
};
use crate::fs::subr::{
    getnewvnode, hashinit, siphash_key, vfs, vget, vinvalbuf, vput, vrecycle, vref, vrele,
};
use crate::fs::vnode::{
    IO_SYNC, LK_EXCLUSIVE, LK_RETRY, V_SAVE, V_SAVEMETA, VDIR, VREG, VROOT, VT_MSDOSFS, Vnode,
    vn_lock,
};
use crate::fs::vops::{Cred, vop_unlock};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::tc::getnanotime;
use crate::vm::vnode::{uvm_vnp_setsize, uvm_vnp_uncache};

hal::pin_adapter!(
    /// The denode hash chains (`de_next`/`de_prev`), through `de_hash`.
    impl[G: ProcGlue] pub DeHashLink<G>: Denode<G>, de_hash
);

/// A chain of the denode hash.
type Chain<G> = PinList<DeHashLink<G>>;

/// `dehashtbl`, `dehash` and `dehashkey`: the hash chains, the table's size - 1, the key.
pub struct Dehash<G: ProcGlue> {
    tbl: MArray<Chain<G>>,
    mask: u64,
    key: SiphashKey,
}

impl<G: ProcGlue> Dehash<G> {
    /// `msdosfs_dehash(dev, dirclust, diroff)` (`DEHASH`): the index of the chain of a
    /// directory entry.
    pub fn msdosfs_dehash(&self, dev: Dev, dirclust: u32, diroff: u32) -> u32 {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&dev.to_ne_bytes());
        ctx.update(&dirclust.to_ne_bytes());
        ctx.update(&diroff.to_ne_bytes());
        // `& dehash`, the table's size - 1, a power of two below 2^31 (`hashinit` of an
        // `int`): the index fits the C's `u_int`.
        (ctx.end() & self.mask) as u32
    }

    /// `&dehashtbl[i]`.
    fn chain(&self, i: u32) -> &Chain<G> {
        &self.tbl[i as usize]
    }
}

/// msdosfs's globals of a glue: `msdosfs_denode.c`'s denode hash and `msdosfs_vnops.c`'s
/// `msdosfs_vops` (a static generic over the glue cannot exist, and these are the tables of
/// the glue's denodes and vnodes).
pub struct MsdosfsTables<G: ProcGlue> {
    /// The denode hash, once `msdosfs_init` made it.
    pub(super) dehash: ReadMostly<Option<Dehash<G>>>,
    /// `msdosfs_vops`: the vnode operations vector of msdos vnodes. A member of the glue's
    /// tables (a `static`), so the table has one address per glue, which the VFS's registry
    /// keys it by (`vops_id`): a promoted generic constant has none of its own (D-b2-10).
    pub vops: MsdosfsVnops<G>,
}

impl<G: ProcGlue> MsdosfsTables<G> {
    /// Nothing initialised.
    pub const fn new() -> Self {
        Self {
            dehash: ReadMostly::new(None),
            vops: MsdosfsVnops::new(),
        }
    }
}

impl<G: ProcGlue> Default for MsdosfsTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The table, when `msdosfs_init` has made it: `panic(9)` before.
fn with_dehash<G: ProcGlue, R>(f: impl FnOnce(&Dehash<G>) -> R) -> R {
    let t = vfs::<G>().msdosfs.dehash.read();
    match t.as_ref() {
        Some(dh) => f(dh),
        None => kpanic(format_args!("msdosfs_dehash: no table")),
    }
}

/// `msdosfs_init(vfsp)` (`vfs_init`): set up the denode hash table, `hashinit(initialvnodes /
/// 2, M_MSDOSFSMNT, M_WAITOK, &dehash)`, and a random key.
pub fn msdosfs_init<G: ProcGlue>() -> Result<(), Errno> {
    let initialvnodes = vfs::<G>()
        .initialvnodes
        .load(core::sync::atomic::Ordering::Relaxed);
    let Some((tbl, mask)) = hashinit::<DeHashLink<G>>(initialvnodes / 2, M_MSDOSFSMNT, M_WAITOK)
    else {
        kpanic(format_args!("msdosfs_init: no memory"));
    };
    let dh = Dehash {
        tbl,
        mask,
        key: siphash_key(),
    };
    let old = vfs::<G>().msdosfs.dehash.replace(Some(dh));
    if old.is_some() {
        kpanic(format_args!("msdosfs_init: twice"));
    }
    Ok(())
}

/// `msdosfs_hashget(dev, dirclust, diroff)`: the denode of the entry, its vnode referenced and
/// locked, when it is in the cache.
fn msdosfs_hashget<G: ProcGlue>(dev: Dev, dirclust: u32, diroff: u32) -> Option<DenodeRef<G>> {
    loop {
        let found = with_dehash::<G, _>(|dh| {
            let r = klock::reader();
            dh.chain(dh.msdosfs_dehash(dev, dirclust, diroff))
                .iter(&r)
                .find(|dep| {
                    dirclust == dep.de_dirclust.get()
                        && diroff == dep.de_diroffset.get()
                        && dev == dep.de_dev
                        && dep.de_refcnt.get() != 0
                })
                .map(|dep| {
                    let vp = dep.detov();
                    (vp, vp.id())
                })
        });
        let (vp, vpid) = found?;
        // The walk starts over (the C's `break` back into its `for (;;)`) when `vget` fails
        // or the vnode was recycled meanwhile.
        if vget(vp, LK_EXCLUSIVE).is_ok() {
            if vpid != vp.id() {
                vput(vp);
                continue;
            }
            return Some(vtode(vp));
        }
    }
}

/// `msdosfs_hashins(dep)`: put the denode on its chain; `EEXIST` when a live denode of the
/// same entry is there already.
fn msdosfs_hashins<G: ProcGlue>(dep: Pin<&Denode<G>>) -> Result<(), Errno> {
    with_dehash::<G, _>(|dh| {
        let mut w = klock::witness();
        let i = dh.msdosfs_dehash(dep.de_dev, dep.de_dirclust.get(), dep.de_diroffset.get());
        let chain = dh.chain(i);
        if chain.iter(&w).any(|deq| {
            dep.de_dirclust.get() == deq.de_dirclust.get()
                && dep.de_diroffset.get() == deq.de_diroffset.get()
                && dep.de_dev == deq.de_dev
                && deq.de_refcnt.get() != 0
        }) {
            return Err(Errno::EEXIST);
        }
        dep.de_chain.set(i);
        chain.insert_head(&mut w, dep);
        Ok(())
    })
}

/// `msdosfs_hashrem(dep)`: take the denode off its chain, if it is on one.
fn msdosfs_hashrem<G: ProcGlue>(dep: &Denode<G>) {
    with_dehash::<G, _>(|dh| {
        let mut w = klock::witness();
        if dep.de_hash.is_linked() {
            dh.chain(dep.de_chain.get()).remove(&mut w, dep);
        }
    });
}

/// `deget(pmp, dirclust, diroffset, depp)`: the denode of the directory entry at
/// (`dirclust`, `diroffset`), returned with its vnode locked.
///
/// - `pmp`: the msdosfsmount structure of the filesystem containing the denode of interest.
///   The `pm_dev` field and the address of the msdosfsmount structure are used.
/// - `dirclust`: which cluster bp contains; if `dirclust` is 0 (root directory) `diroffset` is
///   relative to the beginning of the root directory, otherwise it is cluster relative.
/// - `diroffset`: offset past begin of cluster of denode we want
pub fn deget<G: ProcGlue>(
    pmp: &MsdosfsmountRef<G>,
    mut dirclust: u32,
    diroffset: u32,
) -> Result<DenodeRef<G>, Errno> {
    // On FAT32 filesystems, root is a (more or less) normal directory.
    if fat32(pmp) && dirclust == MSDOSFSROOT {
        dirclust = pmp.pm_rootdirblk;
    }

    // See if the denode is in the denode cache. Use the location of the directory entry to
    // compute the hash value. For subdir use address of "." entry. For root dir (if not FAT32)
    // use cluster MSDOSFSROOT, offset MSDOSFSROOT_OFS.
    //
    // NOTE: The check for de_refcnt > 0 below insures the denode being examined does not
    // represent an unlinked but still open file. These files are not to be accessible even
    // when the directory entry that represented the file happens to be reused while the
    // deleted file is still open.
    let (ldep, nvp) = loop {
        // retry:
        if let Some(ldep) = msdosfs_hashget::<G>(pmp.pm_dev, dirclust, diroffset) {
            return Ok(ldep);
        }

        // Directory entry was not in cache, have to create a vnode and copy it from the passed
        // disk buffer.
        // getnewvnode() does a vref() on the vnode
        let nvp = getnewvnode(VT_MSDOSFS, Some(pmp.pm_mountp), &vfs::<G>().msdosfs.vops)?;
        let d = Denode::new(Some(nvp), pmp.pm_dev, pmp.clone(), dirclust, diroffset);
        fc_purge(&d, 0); // init the fat cache for this denode
        let ldep = d.alloc();
        nvp.set_node(MArc::erase(ldep.clone()));

        // Insert the denode into the hash queue and lock the denode so it can't be accessed
        // until we've read it in and have done what we need to it.
        let _ = vn_lock(nvp, LK_EXCLUSIVE | LK_RETRY);
        match msdosfs_hashins(ldep.as_pin()) {
            Ok(()) => break (ldep, nvp),
            Err(error) => {
                drop(ldep);
                vput(nvp);

                if error == Errno::EEXIST {
                    continue;
                }

                return Err(error);
            }
        }
    };

    ldep.de_devvp.set(Some(pmp.pm_devvp));
    ldep.de_refcnt.set(1);
    // Copy the directory entry into the denode area of the vnode.
    if (dirclust == MSDOSFSROOT || (fat32(pmp) && dirclust == pmp.pm_rootdirblk))
        && diroffset == MSDOSFSROOT_OFS
    {
        // Directory entry for the root directory. There isn't one, so we manufacture one. We
        // should probably rummage through the root directory and find a label entry (if it
        // exists), and then use the time and date from that entry as the time and date for
        // the root denode.
        nvp.flag_set(VROOT); // should be further down XXX

        ldep.de_Attributes.set(ATTR_DIRECTORY);
        if fat32(pmp) {
            ldep.de_StartCluster.set(pmp.pm_rootdirblk);
            // de_FileSize will be filled in further down
        } else {
            ldep.de_StartCluster.set(MSDOSFSROOT);
            // The root's bytes, `pm_rootdirsize` blocks of `DEV_BSIZE` (the module's
            // deviations, D-N5.7-U4): the C multiplies by `pm_BytesPerSec`.
            ldep.de_FileSize.set(de_bn2off(pmp, pmp.pm_rootdirsize));
        }
        // fill in time and date so that dos2unixtime() doesn't spit up when called from
        // msdosfs_getattr() with root denode
        ldep.de_CTime.set(0x0000); // 00:00:00
        ldep.de_CTimeHundredth.set(0);
        // Year 0 (`0 << DD_YEAR_SHIFT`), month 1, day 1: Jan 1, 1980.
        ldep.de_CDate
            .set((1 << DD_MONTH_SHIFT | 1 << DD_DAY_SHIFT) as u16);
        ldep.de_ADate.set(ldep.de_CDate.get());
        ldep.de_MTime.set(ldep.de_CTime.get());
        ldep.de_MDate.set(ldep.de_CDate.get());
        // leave the other fields as garbage
    } else {
        let (bp, off) = readep(pmp, dirclust, diroffset)?;
        de_internalize(&ldep, &entry_at(bp.bytes(), off));
        brelse(bp);
    }

    // Fill in a few fields of the vnode and finish filling in the denode. Then return the
    // address of the found denode.
    if ldep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        // Since DOS directory entries that describe directories have 0 in the filesize field,
        // we take this opportunity to find out the length of the directory and plug it into
        // the denode structure.
        nvp.set_vtype(VDIR);
        if ldep.de_StartCluster.get() != MSDOSFSROOT {
            let mut size = 0;
            match pcbmap(&ldep, CLUST_END, None, Some(&mut size), None) {
                Err(Errno::E2BIG) => ldep.de_FileSize.set(de_cn2off(pmp, size)),
                Err(error) => {
                    printf(format_args!("deget(): pcbmap returned {}\n", error as i32));
                    return Err(error);
                }
                Ok(()) => {}
            }
        }
    } else {
        nvp.set_vtype(VREG);
    }
    vref(pmp.pm_devvp);
    Ok(ldep)
}

/// `deupdat(dep, waitfor)`: write the denode's times, attributes and size back to its
/// directory entry, synchronously when `waitfor` is nonzero.
pub fn deupdat<G: ProcGlue>(dep: &Denode<G>, waitfor: i32) -> Result<(), Errno> {
    if dep
        .detov()
        .mount()
        .is_some_and(|mp| mp.flag() & MNT_RDONLY != 0)
    {
        return Ok(());
    }
    let ts = getnanotime();
    detimes(dep, &ts, &ts, &ts);
    if dep.de_flag.get() & DE_MODIFIED == 0 {
        return Ok(());
    }
    dep.clr_flag(DE_MODIFIED);
    if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        return Ok(());
    }
    if dep.de_refcnt.get() <= 0 {
        return Ok(());
    }
    let (mut bp, off) = readde(dep)?;
    let mut dirp = entry_at(bp.bytes(), off);
    de_externalize(&mut dirp, dep);
    entry_put(bp.data(), off, &dirp);
    if waitfor != 0 {
        bwrite(bp)
    } else {
        bdwrite(bp);
        Ok(())
    }
}

/// `detrunc(dep, length, flags, cred, p)`: truncate the file described by `dep` to the length
/// specified by `length`.
pub fn detrunc<G: ProcGlue>(
    dep: &Denode<G>,
    length: u32,
    flags: i32,
    cred: Cred<'_>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let isadir = dep.de_Attributes.get() & ATTR_DIRECTORY != 0;
    let pmp = dep.pmp();
    let vp = dep.detov();
    let mut chaintofree = 0;

    // Disallow attempts to truncate the root directory since it is of fixed size. That's
    // just the way dos filesystems are. We use the VROOT bit in the vnode because checking for
    // the directory bit and a startcluster of 0 in the denode is not adequate to recognize the
    // root directory at this point in a file or directory's life.
    if vp.flag() & VROOT != 0 && !fat32(pmp) {
        printf(format_args!(
            "detrunc(): can't truncate root directory, clust {}, offset {}\n",
            dep.de_dirclust.get(),
            dep.de_diroffset.get()
        ));
        return Err(Errno::EINVAL);
    }

    uvm_vnp_setsize(vp, i64::from(length));

    if dep.de_FileSize.get() < length {
        return deextend(dep, length, cred);
    }

    // If the desired length is 0 then remember the starting cluster of the file and set the
    // StartCluster field in the directory entry to 0. If the desired length is not zero, then
    // get the number of the last cluster in the shortened file. Then get the number of the
    // first cluster in the part of the file that is to be freed. Then set the next cluster
    // pointer in the last cluster of the file to CLUST_EOFE.
    let eofentry = if length == 0 {
        chaintofree = dep.de_StartCluster.get();
        dep.de_StartCluster.set(0);
        !0
    } else {
        let mut eofentry = 0;
        pcbmap(
            dep,
            de_clcount(pmp, length).wrapping_sub(1),
            None,
            Some(&mut eofentry),
            None,
        )?;
        eofentry
    };

    fc_purge(dep, de_clcount(pmp, length));

    // If the new length is not a multiple of the cluster size then we must zero the tail end
    // of the new last cluster in case it becomes part of the file again because of a seek.
    let boff = length & pmp.pm_crbomask;
    if boff != 0 {
        let (mut bp, error) = if isadir {
            let bn = cntobn(pmp, eofentry);
            bread(&pmp.pm_devvp.bufs, Daddr::from(bn), pmp.pm_bpcluster as i32)
        } else {
            let bn = de_blk(pmp, length);
            bread(&vp.bufs, Daddr::from(bn), pmp.pm_bpcluster as i32)
        };
        if let Err(error) = error {
            brelse(bp);
            return Err(error);
        }
        let _ = uvm_vnp_uncache(vp);
        // is this the right place for it?
        bp.data()[boff as usize..pmp.pm_bpcluster as usize].fill(0);
        if flags & IO_SYNC != 0 {
            let _ = bwrite(bp);
        } else {
            bdwrite(bp);
        }
    }

    // Write out the updated directory entry. Even if the update fails we free the trailing
    // clusters.
    dep.de_FileSize.set(length);
    if !isadir {
        dep.set_flag(DE_UPDATE | DE_MODIFIED);
    }
    let vflags = (if length > 0 { V_SAVE } else { 0 }) | V_SAVEMETA;
    let _ = vinvalbuf(vp, vflags, cred, p, 0, INFSLP);
    let allerror = deupdat(dep, 1);

    // If we need to break the cluster chain for the file then do it now.
    if eofentry != !0 {
        fatentry(
            FAT_GET_AND_SET,
            pmp,
            eofentry,
            Some(&mut chaintofree),
            CLUST_EOFE,
        )?;
        fc_setcache(
            dep,
            FC_LASTFC,
            de_cluster(pmp, length.wrapping_sub(1)),
            eofentry,
        );
    }

    // Now free the clusters removed from the file because of the truncation.
    if chaintofree != 0 && !msdosfseof(pmp, chaintofree) {
        let _ = freeclusterchain(pmp, chaintofree);
    }

    allerror
}

/// `deextend(dep, length, cred)`: extend the file described by `dep` to length specified by
/// `length`.
pub fn deextend<G: ProcGlue>(dep: &Denode<G>, length: u32, cred: Cred<'_>) -> Result<(), Errno> {
    let pmp = dep.pmp();

    // The root of a DOS filesystem cannot be extended.
    if dep.detov().flag() & VROOT != 0 && !fat32(pmp) {
        return Err(Errno::EINVAL);
    }

    // Directories cannot be extended.
    if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        return Err(Errno::EISDIR);
    }

    if length <= dep.de_FileSize.get() {
        kpanic(format_args!("deextend: file too large"));
    }

    // Compute the number of clusters to allocate.
    let count = de_clcount(pmp, length).wrapping_sub(de_clcount(pmp, dep.de_FileSize.get()));
    if count > 0 {
        if count > pmp.pm_freeclustercount.get() {
            return Err(Errno::ENOSPC);
        }
        if let Err(error) = extendfile(dep, count, None, None, DE_CLEAR) {
            // truncate the added clusters away again
            let _ = G::with_curproc(|p| detrunc(dep, dep.de_FileSize.get(), 0, cred, p));
            return Err(error);
        }
    }

    dep.de_FileSize.set(length);
    dep.set_flag(DE_UPDATE | DE_MODIFIED);
    deupdat(dep, 1)
}

/// `reinsert(dep)`: move a denode to its correct hash queue after the file it represents has
/// been moved to a new directory.
pub fn reinsert<G: ProcGlue>(dep: Pin<&Denode<G>>) {
    // Fix up the denode cache. If the denode is for a directory, there is nothing to do since
    // the hash is based on the starting cluster of the directory file and that hasn't
    // changed. If for a file the hash is based on the location of the directory entry, so we
    // must remove it from the cache and re-enter it with the hash based on the new location
    // of the directory entry.
    if dep.de_Attributes.get() & ATTR_DIRECTORY != 0 {
        return;
    }
    msdosfs_hashrem(&dep);
    let _ = msdosfs_hashins(dep);
}

/// `msdosfs_reclaim` (`vop_reclaim`): unhash the denode and free it, so that the vnode can
/// be used for other purposes. The vnode's handle goes (`vp->v_data = NULL`); the last handle
/// frees the block.
pub fn msdosfs_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let dep = vtode(vp);

    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "msdosfs_reclaim(): pushing active");

    // Remove the denode from its hash chain.
    msdosfs_hashrem::<G>(&dep);
    // Purge old data structures associated with the denode.
    cache_purge(vp);
    if let Some(devvp) = dep.de_devvp.get() {
        vrele(devvp);
        dep.de_devvp.set(None);
    }
    // `#if 0 /* XXX */ dep->de_flag = 0;` in the C.
    drop(dep);
    drop(vp.take_node());
    Ok(())
}

/// `msdosfs_inactive` (`vop_inactive`): last reference to a denode. Truncate a removed file
/// and write the times back; recycle the vnode of a deleted entry at once. The vnode is
/// unlocked on return, as the C's.
pub fn msdosfs_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let dep = vtode(vp);
    let mut error = Ok(());

    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "msdosfs_inactive(): pushing active");

    // Get rid of denodes related to stale file handles.
    if dep.de_Name[0].get() != SLOT_DELETED {
        // If the file has been deleted and it is on a read/write filesystem, then truncate the
        // file, and mark the directory slot as empty. (This may not be necessary for the dos
        // filesystem.)
        let rdonly = vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0);
        if dep.de_refcnt.get() <= 0 && !rdonly {
            error = detrunc(&dep, 0, 0, Cred::NOCRED, p);
            dep.de_Name[0].set(SLOT_DELETED);
        }
        let _ = deupdat(&dep, 0);
    }

    // out:
    let _ = vop_unlock(vp);
    // If we are done with the denode, reclaim it so that it can be reused immediately.
    let deleted = dep.de_Name[0].get() == SLOT_DELETED;
    drop(dep);
    if deleted {
        let _ = vrecycle(vp, p);
    }
    error
}

/// `msdosfs_test_reset`: forgets the denode hash of a test thread's tables, without freeing
/// it (the memory behind it is gone with the last test's, testing.md), as `ufs_test_reset`
/// forgets the inode hash.
#[cfg(any(test, feature = "doubles"))]
pub fn msdosfs_test_reset<G: ProcGlue>() {
    core::mem::forget(vfs::<G>().msdosfs.dehash.replace(None));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::testfs::TestGlue;

    /// A table over a fixed key and four chains, as `hashinit` and `siphash_key` would make
    /// it.
    fn table(key: SiphashKey) -> Dehash<TestGlue> {
        let Some((tbl, mask)) = hashinit::<DeHashLink<TestGlue>>(3, M_MSDOSFSMNT, M_WAITOK) else {
            std::panic!("no table");
        };
        Dehash { tbl, mask, key }
    }

    /// `msdosfs_dehash` is SipHash-2-4 of the device, the cluster and the offset in their
    /// native bytes, masked to the table: the C's (`SipHash24_Update` of each in turn).
    #[test]
    fn dehash_is_the_siphash_of_the_entry_s_position_masked_to_the_table() {
        crate::testutil::install();
        let key = || SiphashKey {
            k0: 0x0706_0504_0302_0100,
            k1: 0x0f0e_0d0c_0b0a_0908,
        };
        let dh = table(key());
        assert_eq!(dh.mask, 3, "hashinit(3): four chains");
        for (dev, cl, off) in [
            (0x400, 0, MSDOSFSROOT_OFS),
            (0x400, 2, 32),
            (7, 0x0fff_fff6, 0),
        ] {
            let mut ctx = SipHash24Ctx::new(&key());

            let mut bytes = std::vec::Vec::new();
            bytes.extend_from_slice(&u32::to_ne_bytes(dev as u32));
            bytes.extend_from_slice(&u32::to_ne_bytes(cl));
            bytes.extend_from_slice(&u32::to_ne_bytes(off));
            ctx.update(&bytes);
            assert_eq!(dh.msdosfs_dehash(dev, cl, off), (ctx.end() & 3) as u32);
        }
        // A position that differs in any of the three words picks its chain afresh.
        let a = dh.msdosfs_dehash(0x400, 2, 32);
        assert!((0..64u32).any(|o| dh.msdosfs_dehash(0x400, 2, 32 * o) != a));
    }

    /// The FAT12/16 root's manufactured size is its blocks' bytes, `pm_rootdirsize *
    /// DEV_BSIZE` (D-N5.7-U4), on sectors of 1 and 4 KiB, where the C's `pm_rootdirsize *
    /// pm_BytesPerSec` is 2 and 8 times that.
    #[test]
    fn the_fat16_root_s_size_is_its_blocks_bytes() {
        use crate::fs::msdosfs::msdosfs_lookup::test_full_root;
        for (bps, spc, ents, bytes) in [(1024, 2, 208, 6656), (4096, 1, 512, 16384)] {
            let (_g, _p, pmp) = test_full_root(bps, spc, ents);
            let root = deget(&pmp, MSDOSFSROOT, MSDOSFSROOT_OFS).unwrap();
            let vp = root.detov();
            assert_eq!(root.de_FileSize.get(), bytes);
            assert_ne!(root.de_FileSize.get(), pmp.pm_rootdirsize * u32::from(bps));
            assert_ne!(vp.flag() & VROOT, 0);
            assert_eq!(vp.vtype(), VDIR);
            assert_eq!(
                (root.de_Attributes.get(), root.de_StartCluster.get()),
                (ATTR_DIRECTORY, MSDOSFSROOT)
            );
            drop(root);
            vput(vp);
        }
    }

    /// `deget` makes the denode of an entry once: the cache answers it again while it lives,
    /// with its vnode locked (`vget`); the reclaim takes it off its chain and the next `deget`
    /// makes a new one from the entry.
    #[test]
    fn deget_hashes_the_denode_until_its_reclaim() {
        use crate::fs::msdosfs::msdosfs_lookup::test_full_root;
        use crate::fs::subr::vgone;
        let (_g, _p, pmp) = test_full_root(1024, 2, 208);
        let dep = deget(&pmp, MSDOSFSROOT, 32).unwrap();
        let vp = dep.detov();
        assert_eq!(&dep.name(), b"F0000001TXT");
        assert_eq!((vp.vtype(), dep.de_refcnt.get()), (VREG, 1));
        assert!(dep.de_hash.is_linked());
        assert_eq!(
            dep.de_devvp.get().map(core::ptr::from_ref),
            Some(core::ptr::from_ref(pmp.pm_devvp))
        );
        // The cache's: the same denode, its vnode referenced again (and locked, recursively).
        let again = deget(&pmp, MSDOSFSROOT, 32).unwrap();
        assert!(MArc::ptr_eq(&dep, &again));
        assert_eq!(vp.usecount(), 2);
        drop(again);
        vput(vp);
        // A reclaim unhashes it and gives the device's reference back.
        let devrefs = pmp.pm_devvp.usecount();
        let _ = vop_unlock(vp);
        vgone(vp);
        assert!(!dep.de_hash.is_linked());
        assert_eq!(pmp.pm_devvp.usecount(), devrefs - 1);
        assert!(!vp.has_node());
        let fresh = deget(&pmp, MSDOSFSROOT, 32).unwrap();
        assert!(!MArc::ptr_eq(&dep, &fresh));
        drop(dep);
        let fvp = fresh.detov();
        drop(fresh);
        vput(fvp);
    }
}
/* </TESTS> */
