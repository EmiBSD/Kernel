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
 * Copyright (c) 1994
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley
 * by Pace Willisson (pace@blitz.com).  The Rock Ridge Extension
 * Support code is derived from software contributed to Berkeley
 * by Atsushi Murai (amurai@spec.co.jp).
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
 *	@(#)cd9660_node.h	8.4 (Berkeley) 12/5/94
 */
/*-
 * Copyright (c) 1982, 1986, 1989, 1994
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley
 * by Pace Willisson (pace@blitz.com).  The Rock Ridge Extension
 * Support code is derived from software contributed to Berkeley
 * by Atsushi Murai (amurai@spec.co.jp).
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
 *	@(#)cd9660_node.c	8.5 (Berkeley) 12/5/94
 */
/* </LICENSES> */

/* <CODE> */
//! The in-core ISO 9660 node: `<isofs/cd9660/cd9660_node.h>` (`struct iso_node`, its
//! attributes `ISO_RRIP_INODE`, `VTOI`/`ITOV`) and `isofs/cd9660/cd9660_node.c` (the node
//! hash, `cd9660_inactive`/`cd9660_reclaim`, the attributes and time stamps of a plain
//! ISO 9660 directory record, and the conversion of its dates).
//!
//! Upstream: sys/isofs/cd9660/cd9660_node.h @ 3ce1f3f79392
//! Upstream: sys/isofs/cd9660/cd9660_node.c @ 3ce1f3f79392
//! LZ: sys/isofs/cd9660/cd9660_node.rs@3c62ede68802
//!
//! The chains are picked by a SipHash of the device and the inode number under a random key,
//! so that an attacker cannot predict which nodes collide.
//!
//! ## Deviations
//! - The hash chain links `i_next`/`i_prev` are a pinned list entry (`i_hash`) of the hash's
//!   chains, with the chain's index (`i_chain`, what `i_prev` knows of its chain); the C's
//!   `i_prev == NULL` (a node that was never hashed) is the entry's own state.
//! - `isohashtbl`, `isohash` (the table's size - 1) and `isohashkey` are one [`Isohash`] in
//!   the glue's VFS tables ([`IsofsTables`], `VfsTables::cd9660`), made by `cd9660_init` (from
//!   `vfsinit`, before any mount); `cd9660_isohash` (`INOHASH`) is [`Isohash::cd9660_isohash`],
//!   the table's own.
//! - `cd9660_defattr` and `cd9660_deftstamp` take the record as an [`IsoDirectoryRecord`],
//!   the attributes they fill as a `&mut IsoRripInode` (the C's `inop->inode`, which the
//!   caller stores in the node: [`IsoNode::update_inode`]) and the extended attribute buffer
//!   as an `Option`; `cd9660_tstamp_conv7`/`17` return `bool` (the C's 1 and 0) and read the
//!   date from a byte slice. The date arithmetic wraps as the C's `int` does in practice.
//! - `prtactive`'s `vprint` under `DIAGNOSTIC` is behind feature `diagnostic`.
//!
//! ## Redesign
//! - LZ's `IsoNode` (`Cell` members under an `unsafe impl Sync`, `malloc(M_ISOFSNODE)` hung
//!   from `v_data` and reached by `vtoi`'s `unsafe` deref behind a tag check as a `&'static
//!   IsoNode`, freed by `cd9660_reclaim`) -> an [`IsoNode`] made where the C fills its
//!   `malloc(M_ZERO)` block ([`IsoNode::new`]: the identity `i_vnode`, `i_dev`, `i_number` and
//!   the mount structure's handle `i_mnt` given, the rest zero) and published in a counted
//!   `malloc(M_ISOFSNODE)` block ([`IsoNodeRef`], `hal::malloc::MArc`; `vmstat -m`'s `ISOFS
//!   node` row, the C's requests; a typed pool would add a pool row userland sees, D-N5.7-2,
//!   D-N5.8-7) erased into the vnode's typed slot (`Vnode::set_node` in
//!   `cd9660_vget_internal`, N5.8c2, where the C sets `v_data`; `take_node` in
//!   [`cd9660_reclaim`]): the last handle frees the block (msdosfs's denode's shape). [`vtoi`]
//!   is the slot's typed read and a count (the type replaces LZ's tag check; cost against the
//!   C's one load: the slot's read and a count pair per operation, ROADMAP N12's row).
//! - What the C writes after the node is hashed and hung (`cd9660_vget_internal` reads the
//!   record only then, and `checkalias` moves the node to a device alias) are relaxed words
//!   (`fs::word`, docs/IDIOMS.md's counters' row: every cd9660 path is `KERNEL_LOCK`ed):
//!   `iso_extent`, `i_size`, `iso_start`, and what the C changes under the node's lock (`i_flag`, the lookup's directory offsets
//!   `i_endoff`/`i_diroff`/`i_offset`/`i_ino`, the read-ahead's `ci_lastr`); `i_vnode` and
//!   `i_devvp` are `hal::link::Link`s (the alias move; `vref`'d after the hashing and given
//!   back by the reclaim). `i_mnt` is a counted handle of the mount structure held for the
//!   node's life ([`IsoNode::mnt`] borrows it: one load, the C's `ip->i_mnt`). `i_lock` is a
//!   `hal::rwlock::RecursiveRwlock` (the C's `RWL_DUPOK | RWL_IS_VNODE` are `WITNESS` flags,
//!   as the denode's).
//! - The attributes (`inode`, the C's `ISO_RRIP_INODE`, filled once by `cd9660_vget_internal`
//!   from the record and read whole by `getattr`, `access` and `inactive`) are a
//!   `hal::slot::ReadMostly<IsoRripInode>` ([`IsoNode::inode`] a copy out, [`IsoNode::set_inode`]
//!   the one write; `ufs_vfsops.rs`'s tables the precedent): a read is one compare-and-swap and
//!   a release, where the C loads the members it needs; review 1 of N5.8c1.
//! - LZ's three `StaticCell`s and the chains of `sys/queue.rs` `ListHead`s linked and
//!   unlinked in `unsafe` blocks whose argument was the kernel lock -> [`Isohash`], the
//!   denode hash's shape (N5.7c2): a `hashinit` table of `hal::pinlist::PinList`s of the
//!   pinned nodes (`MArc::as_pin`), its adapter [`IsoHashLink`] from `hal::pin_adapter!`'s
//!   generic form (LZ's `IsoHash`, a `queue_adapter!`), in a `ReadMostly` cell of the glue's
//!   tables. **The C has no lock** ("XXX locking"): the kernel lock is the lock, as LZ's: a
//!   walk takes `klock::reader()` (a per-CPU count), an insertion or a removal
//!   `klock::witness()`, whose interrupt-level raise is one spl pair per `cd9660_ihashins`
//!   and per `cd9660_ihashrem` of a hashed node (once in a node's life each), where the C has
//!   none: docs/ROADMAP.md N12's row, as `ufs_ihash.rs`'s and msdosfs's. `hashinit` and the
//!   key are the kernel's shared `fs::subr::hashinit` and `siphash_key`.
//! - `cd9660_ihashget` finds the node's vnode under the reader and drops it before `vget`
//!   (which may sleep), rechecks `v_id` as the C does; `cd9660_ihashins` keeps the C's
//!   duplicate scan and `EEXIST`, and locks the vnode after the insertion as the C does.
//! - LZ's lent buffers (`extattr_v1`'s `unsafe` view of `b_data`) -> `fs::bio`'s `BufRef`
//!   tokens: the extended attribute record is read out of the token's bytes with
//!   `IsoExtendedAttributes::at` (a 250-byte copy, under `ISOFSMNT_EXTATT` only). 0 `unsafe`
//!   (LZ: 6).

use core::pin::Pin;

use hal::klock;
use hal::link::Link;
use hal::malloc::{M_WAITOK, MArc, MArray};
use hal::pinlist::PinList;
use hal::rwlock::RecursiveRwlock;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::mount::ISOFSMNT_EXTATT;
use hal::sys::stat::{S_IFDIR, S_IFREG, S_IRGRP, S_IROTH, S_IRUSR, S_IXGRP, S_IXOTH, S_IXUSR};
use hal::sys::types::{Dev, Gid, Uid};
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::fs::bio::brelse;
use crate::fs::buf::BufRef;
use crate::fs::cache::cache_purge;
use crate::fs::isofs::cd9660::cd9660_extern::{IsoMnt, IsoMntRef, M_ISOFSMNT};
use crate::fs::isofs::cd9660::cd9660_lookup::cd9660_bufatoff;
use crate::fs::isofs::cd9660::iso::{
    Cdino, IsoDirectoryRecord, IsoExtendedAttributes, isonum_711, isonum_723, isonum_733,
};
use crate::fs::subr::{hashinit, siphash_key, vfs, vget, vput, vrecycle, vrele};
use crate::fs::vnode::{LK_EXCLUSIVE, VT_ISOFS, Vnode};
use crate::fs::vops::{vop_lock, vop_unlock};
use crate::fs::word::{I64Word, U32Word, U64Word};
use crate::proc::{ProcGlue, Thread};

/// `M_ISOFSNODE`: ISOFS vnode private part (`<sys/malloc.h>`; `bsd`'s `sys/malloc.rs` until N7
/// moves the header).
pub const M_ISOFSNODE: i32 = 58;

/// `doff_t`.
pub type Doff = u64;

/// `ISO_RRIP_INODE`: the attributes of a node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IsoRripInode {
    /// `iso_atime`: time of last access.
    pub iso_atime: Timespec,
    /// `iso_mtime`: time of last modification.
    pub iso_mtime: Timespec,
    /// `iso_ctime`: time file changed.
    pub iso_ctime: Timespec,
    /// `iso_mode`: files access mode and type.
    pub iso_mode: u16,
    /// `iso_uid`: owner user id.
    pub iso_uid: Uid,
    /// `iso_gid`: owner group id.
    pub iso_gid: Gid,
    /// `iso_links`: links of file.
    pub iso_links: i16,
    /// `iso_rdev`: Major/Minor number for special.
    pub iso_rdev: Dev,
}

impl IsoRripInode {
    /// All zero, as `M_ZERO` leaves them.
    pub const ZERO: IsoRripInode = IsoRripInode {
        iso_atime: Timespec::ZERO,
        iso_mtime: Timespec::ZERO,
        iso_ctime: Timespec::ZERO,
        iso_mode: 0,
        iso_uid: 0,
        iso_gid: 0,
        iso_links: 0,
        iso_rdev: 0,
    };
}

/// `struct iso_node`: the in-core node of a file of a mounted ISO 9660 file system (see the
/// module's redesign for which members are words).
pub struct IsoNode<G: ProcGlue> {
    /// `i_next`, `i_prev`: hash chain.
    pub i_hash: hal::pinlist::PinEntry<IsoNode<G>>,
    /// The index of the chain the node is on (what the C's `i_prev` knows of it).
    pub i_chain: U32Word,
    /// `i_vnode`: vnode associated with this inode.
    pub i_vnode: Link<Vnode<G>>,
    /// `i_devvp`: vnode for block I/O.
    pub i_devvp: Link<Vnode<G>>,
    /// `i_flag`: see below.
    pub i_flag: U32Word,
    /// `i_dev`: device where inode resides.
    pub i_dev: Dev,
    /// `i_number`: the identity of the inode; we use the actual starting block of the
    /// file.
    pub i_number: Cdino,
    /// `i_mnt`: filesystem associated with this inode.
    pub i_mnt: IsoMntRef<G>,
    /// `i_endoff`: end of useful stuff in directory.
    pub i_endoff: U64Word,
    /// `i_diroff`: offset in dir, where we found last entry.
    pub i_diroff: U64Word,
    /// `i_offset`: offset of free space in directory.
    pub i_offset: U64Word,
    /// `i_ino`: inode number of found directory.
    pub i_ino: U32Word,
    /// `i_lock`: node lock.
    pub i_lock: RecursiveRwlock,
    /// `iso_extent`: extent of file.
    pub iso_extent: U64Word,
    /// `i_size`.
    pub i_size: U64Word,
    /// `iso_start`: actual start of data file (may be different from `iso_extent`, if the
    /// file has extended attributes).
    pub iso_start: U64Word,
    /// `inode`: the attributes, filled once by `cd9660_vget_internal` (see the module's
    /// redesign).
    inode: ReadMostly<IsoRripInode>,
    /// `i_ci.ci_lastr`: the last block read, for the read-ahead (the only member of `struct
    /// cluster_info` cd9660 uses).
    pub i_ci_lastr: I64Word,
}

/// A counted handle of a published [`IsoNode`]: the vnode's slot holds one.
pub type IsoNodeRef<G> = MArc<IsoNode<G>>;

impl<G: ProcGlue> IsoNode<G> {
    /// A node of the file `number` of the mount `imp` on the device `dev`, whose vnode is
    /// `vp`, as `cd9660_vget_internal` fills its `malloc(M_ZERO)` block: the rest zero,
    /// unhashed, unlocked.
    pub fn new(vp: &'static Vnode<G>, dev: Dev, number: Cdino, imp: IsoMntRef<G>) -> Self {
        Self {
            i_hash: hal::pinlist::PinEntry::new(),
            i_chain: U32Word::new(0),
            i_vnode: Link::to(vp),
            i_devvp: Link::new(),
            i_flag: U32Word::new(0),
            i_dev: dev,
            i_number: number,
            i_mnt: imp,
            i_endoff: U64Word::new(0),
            i_diroff: U64Word::new(0),
            i_offset: U64Word::new(0),
            i_ino: U32Word::new(0),
            i_lock: RecursiveRwlock::new("isoinode"),
            iso_extent: U64Word::new(0),
            i_size: U64Word::new(0),
            iso_start: U64Word::new(0),
            inode: ReadMostly::new(IsoRripInode::ZERO),
            i_ci_lastr: I64Word::new(0),
        }
    }

    /// Publishes the node in a counted `malloc(M_ISOFSNODE, M_WAITOK)` block (which cannot
    /// fail: `panic(9)` if it does).
    pub fn alloc(self) -> IsoNodeRef<G> {
        match MArc::new(M_ISOFSNODE, M_WAITOK, self) {
            Some(ip) => ip,
            None => kpanic(format_args!("cd9660: malloc(M_ISOFSNODE) failed")),
        }
    }

    /// `ITOV(ip)`: the node's vnode.
    pub fn itov(&self) -> &'static Vnode<G> {
        match self.i_vnode.get() {
            Some(vp) => vp,
            None => kpanic(format_args!("iso_node {self:p}: no vnode")),
        }
    }

    /// `ip->i_mnt`, borrowed.
    #[inline]
    pub fn mnt(&self) -> &IsoMnt<G> {
        &self.i_mnt
    }

    /// `ip->i_devvp`, which `cd9660_vget_internal` sets after the hashing.
    pub fn devvp(&self) -> &'static Vnode<G> {
        match self.i_devvp.get() {
            Some(vp) => vp,
            None => kpanic(format_args!("iso_node {self:p}: no device vnode")),
        }
    }

    /// `ip->inode`: the attributes, copied out.
    pub fn inode(&self) -> IsoRripInode {
        *self.inode.read()
    }

    /// `ip->inode = v`.
    pub fn set_inode(&self, v: &IsoRripInode) {
        self.inode.update(|i| *i = *v);
    }

    /// Changes `ip->inode` through `f` (the C's writes of `inop->inode.*`), and stores it:
    /// `f` runs on a copy, outside the cell, so it may read the disc (`cd9660_defattr`'s
    /// extended attribute record) without holding the cell's writer bit across the sleep.
    pub fn update_inode(&self, f: impl FnOnce(&mut IsoRripInode)) {
        let mut ino = self.inode();
        f(&mut ino);
        self.set_inode(&ino);
    }
}

hal::pin_adapter!(
    /// The node hash chains (`i_next`/`i_prev`), through `i_hash`.
    impl[G: ProcGlue] pub IsoHashLink<G>: IsoNode<G>, i_hash
);

/// A chain of the node hash.
type Chain<G> = PinList<IsoHashLink<G>>;

/// `IN_ACCESS`: inode access time to be updated.
pub const IN_ACCESS: u32 = 0x0020;

/// `isohashtbl`, `isohash` and `isohashkey`: the hash chains, the table's size - 1, the key.
pub struct Isohash<G: ProcGlue> {
    /// `isohashtbl`: structures associated with `iso_node` caching.
    tbl: MArray<Chain<G>>,
    /// `isohash`: size of hash table - 1.
    mask: u64,
    /// `isohashkey`.
    key: SiphashKey,
}

impl<G: ProcGlue> Isohash<G> {
    /// `cd9660_isohash(device, inum)` (`INOHASH`): the index of the chain of the pair.
    pub fn cd9660_isohash(&self, device: Dev, inum: Cdino) -> u32 {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&device.to_ne_bytes());
        ctx.update(&inum.to_ne_bytes());
        // `& isohash`, the table's size - 1, a power of two below 2^31 (`hashinit` of an
        // `int`): the index fits the C's `u_int`.
        (ctx.end() & self.mask) as u32
    }

    /// `&isohashtbl[i]`.
    fn chain(&self, i: u32) -> &Chain<G> {
        &self.tbl[i as usize]
    }
}

/// cd9660's globals of a glue: `cd9660_node.c`'s node hash (a static generic over the glue
/// cannot exist, and these are the tables of the glue's nodes); N5.8c2 adds the vnode tables
/// `cd9660_vops` and `cd9660_specvops`.
pub struct IsofsTables<G: ProcGlue> {
    /// The node hash, once `cd9660_init` made it.
    pub(super) isohash: ReadMostly<Option<Isohash<G>>>,
}

impl<G: ProcGlue> IsofsTables<G> {
    /// Nothing initialised.
    pub const fn new() -> Self {
        Self {
            isohash: ReadMostly::new(None),
        }
    }
}

impl<G: ProcGlue> Default for IsofsTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `VTOI(vp)`: the node of a cd9660 vnode, a counted handle (`panic(9)` on a vnode that has
/// none, LZ's `VT_ISOFS` check made by the slot's type).
pub fn vtoi<G: ProcGlue>(vp: &Vnode<G>) -> IsoNodeRef<G> {
    match vp.node_arc::<IsoNode<G>>() {
        Some(ip) if vp.tag() == VT_ISOFS => ip,
        _ => kpanic(format_args!("VTOI: vnode {vp:p} has no iso_node")),
    }
}

/// The table, when `cd9660_init` has made it: `panic(9)` before.
fn with_isohash<G: ProcGlue, R>(f: impl FnOnce(&Isohash<G>) -> R) -> R {
    let t = vfs::<G>().cd9660.isohash.read();
    match t.as_ref() {
        Some(h) => f(h),
        None => kpanic(format_args!("cd9660_isohash: no table")),
    }
}

/// `cd9660_init(vfsp)` (`vfs_init`): initialize hash links for inodes and dnodes:
/// `hashinit(initialvnodes, M_ISOFSMNT, M_WAITOK, &isohash)` and a random key.
pub fn cd9660_init<G: ProcGlue>() -> Result<(), Errno> {
    let initialvnodes = vfs::<G>()
        .initialvnodes
        .load(core::sync::atomic::Ordering::Relaxed);
    let Some((tbl, mask)) = hashinit::<IsoHashLink<G>>(initialvnodes, M_ISOFSMNT, M_WAITOK) else {
        kpanic(format_args!("cd9660_init: no memory"));
    };
    let h = Isohash {
        tbl,
        mask,
        key: siphash_key(),
    };
    let old = vfs::<G>().cd9660.isohash.replace(Some(h));
    if old.is_some() {
        kpanic(format_args!("cd9660_init: twice"));
    }
    Ok(())
}

/// `cd9660_ihashget(dev, inum)`: use the device/inum pair to find the incore inode, and
/// return its vnode, referenced and locked. If it is in core, but locked, wait for it.
pub fn cd9660_ihashget<G: ProcGlue>(dev: Dev, inum: Cdino) -> Option<&'static Vnode<G>> {
    loop {
        // XXX locking lock hash list?
        let found = with_isohash::<G, _>(|h| {
            let r = klock::reader();
            h.chain(h.cd9660_isohash(dev, inum))
                .iter(&r)
                .find(|ip| inum == ip.i_number && dev == ip.i_dev)
                .map(|ip| {
                    let vp = ip.itov();
                    (vp, vp.id())
                })
        });
        // XXX locking unlock hash list?
        let (vp, vpid) = found?;
        // The walk starts over (the C's `goto loop`) when `vget` fails or the vnode was
        // recycled meanwhile.
        if vget(vp, LK_EXCLUSIVE).is_err() {
            continue;
        }
        if vpid != vp.id() {
            vput(vp);
            continue;
        }
        return Some(vp);
    }
}

/// `cd9660_ihashins(ip)`: insert the inode into the hash table, and return it locked;
/// `EEXIST` (nothing inserted, nothing locked) when the pair is hashed already.
pub fn cd9660_ihashins<G: ProcGlue>(ip: Pin<&IsoNode<G>>) -> Result<(), Errno> {
    with_isohash::<G, _>(|h| {
        // XXX locking lock hash list?
        let mut w = klock::witness();
        let i = h.cd9660_isohash(ip.i_dev, ip.i_number);
        let chain = h.chain(i);
        if chain
            .iter(&w)
            .any(|iq| iq.i_dev == ip.i_dev && iq.i_number == ip.i_number)
        {
            return Err(Errno::EEXIST);
        }
        ip.i_chain.set(i);
        chain.insert_head(&mut w, ip);
        // XXX locking unlock hash list?
        Ok(())
    })?;

    let _ = vop_lock(ip.itov(), LK_EXCLUSIVE);

    Ok(())
}

/// `cd9660_ihashrem(ip)`: remove the inode from the hash table, if it is on it.
pub fn cd9660_ihashrem<G: ProcGlue>(ip: &IsoNode<G>) {
    with_isohash::<G, _>(|h| {
        // XXX locking lock hash list?
        let mut w = klock::witness();
        if ip.i_hash.is_linked() {
            h.chain(ip.i_chain.get()).remove(&mut w, ip);
        }
        // XXX locking unlock hash list?
    });
}

/// `cd9660_inactive` (`vop_inactive`): last reference to an inode; if it is not valid,
/// reclaim it so that it can be reused immediately.
pub fn cd9660_inactive<G: ProcGlue>(
    vp: &'static Vnode<G>,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let ip = vtoi(vp);

    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "cd9660_inactive: pushing active");

    ip.i_flag.set(0);
    let _ = vop_unlock(vp);
    // If we are done with the inode, reclaim it so that it can be reused immediately.
    let done = ip.inode().iso_mode == 0;
    drop(ip);
    if done {
        let _ = vrecycle(vp, p);
    }

    Ok(())
}

/// `cd9660_reclaim` (`vop_reclaim`): reclaim an inode so that it can be used for other
/// purposes.
pub fn cd9660_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let ip = vtoi(vp);

    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "cd9660_reclaim: pushing active");

    // Remove the inode from its hash chain.
    cd9660_ihashrem::<G>(&ip);
    // Purge old data structures associated with the inode.
    cache_purge(vp);
    if let Some(devvp) = ip.i_devvp.get() {
        vrele(devvp);
        ip.i_devvp.set(None);
    }
    // `free(vp->v_data, M_ISOFSNODE)`: the slot's handle, and with ours the last one.
    drop(ip);
    drop(vp.take_node());
    Ok(())
}

/// The extended attribute record at the start of a buffer, when it is version 1 (the only
/// one the C reads). The buffer is a logical block, at least `DEV_BSIZE` bytes (`iso_mountfs`
/// refuses a smaller one), so the record's 250 bytes are always there.
fn extattr_v1(t: &BufRef) -> Option<IsoExtendedAttributes> {
    IsoExtendedAttributes::at(t.bytes(), 0).filter(|ap| isonum_711(&ap.version) == 1)
}

/// The buffer of a node's extended attribute record, when the mount reads them and the
/// record has one (`cd9660_bufatoff` at the negative offset of the record; a read error is
/// no buffer, as the C's `bp2` stays `NULL`).
fn extattr_buf<G: ProcGlue>(isodir: &IsoDirectoryRecord<'_>, inop: &IsoNode<G>) -> Option<BufRef> {
    let imp = inop.mnt();
    if imp.im_flags & ISOFSMNT_EXTATT == 0 {
        return None;
    }
    let off = isonum_711(isodir.ext_attr_length());
    if off == 0 {
        return None;
    }
    cd9660_bufatoff(inop, -(i64::from(off) << imp.im_bshift))
        .ok()
        .map(|(t, _)| t)
}

/// `cd9660_defattr`: file attributes of a plain ISO 9660 record (and of its extended
/// attribute record, in `bp` or read here), into `ino` (the C's `inop->inode`).
pub fn cd9660_defattr<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    inop: &IsoNode<G>,
    ino: &mut IsoRripInode,
    bp: Option<&BufRef>,
) {
    // If we return 2, fts() will assume there are no subdirectories (just links for the
    // path and .), so instead we return 1.
    ino.iso_mode = if isonum_711(isodir.flags()) & 2 != 0 {
        S_IFDIR as u16
    } else {
        S_IFREG as u16
    };
    ino.iso_links = 1;
    let bp2 = if bp.is_none() {
        extattr_buf(isodir, inop)
    } else {
        None
    };
    if let Some(ap) = bp.or(bp2.as_ref()).and_then(extattr_v1) {
        if ap.perm[1] & 0x10 == 0 {
            ino.iso_mode |= S_IRUSR as u16;
        }
        if ap.perm[1] & 0x40 == 0 {
            ino.iso_mode |= S_IXUSR as u16;
        }
        if ap.perm[0] & 0x01 == 0 {
            ino.iso_mode |= S_IRGRP as u16;
        }
        if ap.perm[0] & 0x04 == 0 {
            ino.iso_mode |= S_IXGRP as u16;
        }
        if ap.perm[0] & 0x10 == 0 {
            ino.iso_mode |= S_IROTH as u16;
        }
        if ap.perm[0] & 0x40 == 0 {
            ino.iso_mode |= S_IXOTH as u16;
        }
        ino.iso_uid = Uid::from(isonum_723(&ap.owner)); // what about 0?
        ino.iso_gid = Gid::from(isonum_723(&ap.group)); // what about 0?
    } else {
        ino.iso_mode |= (S_IRUSR | S_IXUSR | S_IRGRP | S_IXGRP | S_IROTH | S_IXOTH) as u16;
        ino.iso_uid = 0;
        ino.iso_gid = 0;
    }
    if let Some(bp2) = bp2 {
        brelse(bp2);
    }
}

/// `cd9660_deftstamp`: time stamps of a plain ISO 9660 record (and of its extended
/// attribute record, in `bp` or read here), into `ino`.
pub fn cd9660_deftstamp<G: ProcGlue>(
    isodir: &IsoDirectoryRecord<'_>,
    inop: &IsoNode<G>,
    ino: &mut IsoRripInode,
    bp: Option<&BufRef>,
) {
    let bp2 = if bp.is_none() {
        extattr_buf(isodir, inop)
    } else {
        None
    };
    if let Some(ap) = bp.or(bp2.as_ref()).and_then(extattr_v1) {
        if !cd9660_tstamp_conv17(&ap.ftime, &mut ino.iso_atime) {
            cd9660_tstamp_conv17(&ap.ctime, &mut ino.iso_atime);
        }
        if !cd9660_tstamp_conv17(&ap.ctime, &mut ino.iso_ctime) {
            ino.iso_ctime = ino.iso_atime;
        }
        if !cd9660_tstamp_conv17(&ap.mtime, &mut ino.iso_mtime) {
            ino.iso_mtime = ino.iso_ctime;
        }
    } else {
        cd9660_tstamp_conv7(isodir.date(), &mut ino.iso_ctime);
        ino.iso_atime = ino.iso_ctime;
        ino.iso_mtime = ino.iso_ctime;
    }

    if let Some(bp2) = bp2 {
        brelse(bp2);
    }
}

/// `cd9660_tstamp_conv7`: a 7-byte date (years since 1900, month, day, hour, minute,
/// second, offset from GMT in 15-minute units) to a time; `false` and the epoch before
/// 1970.
pub fn cd9660_tstamp_conv7(pi: &[u8], pu: &mut Timespec) -> bool {
    let y = i32::from(pi[0]) + 1900;
    let m = i32::from(pi[1]);
    let d = i32::from(pi[2]);
    let hour = i32::from(pi[3]);
    let minute = i32::from(pi[4]);
    let second = i32::from(pi[5]);
    let tz = pi[6] as i8;

    if y < 1970 {
        pu.tv_sec = 0;
        pu.tv_nsec = 0;
        return false;
    }

    // ORIGINAL: computes day number relative to Sept. 19th,1989; don't even *THINK* about
    // changing formula. It works!
    //   days = 367*(y-1980)-7*(y+(m+9)/12)/4-3*((y+(m-9)/7)/100+1)/4+275*m/9+d-100;
    // Changed :-) to make it relative to Jan. 1st, 1970 and to disambiguate negative
    // division. Every term is a byte's value or a small multiple of one: no `int` overflow.
    let days =
        367 * (y - 1960) - 7 * (y + (m + 9) / 12) / 4 - 3 * ((y + (m + 9) / 12 - 1) / 100 + 1) / 4
            + 275 * m / 9
            + d
            - 239;
    let mut crtime = days
        .wrapping_mul(24)
        .wrapping_add(hour)
        .wrapping_mul(60)
        .wrapping_add(minute)
        .wrapping_mul(60)
        .wrapping_add(second);

    // timezone offset is unreliable on some disks
    if (-48..=52).contains(&tz) {
        crtime = crtime.wrapping_sub(i32::from(tz) * 15 * 60);
    }
    pu.tv_sec = i64::from(crtime);
    pu.tv_nsec = 0;
    true
}

/// `cd9660_chars2ui`: the decimal number in the `len` digits at `begin`.
fn cd9660_chars2ui(begin: &[u8], len: usize) -> u32 {
    begin[..len].iter().fold(0u32, |rc, &c| {
        rc.wrapping_mul(10)
            .wrapping_add(u32::from(c).wrapping_sub(u32::from(b'0')))
    })
}

/// `cd9660_tstamp_conv17`: a 17-byte date ("YYYYMMDDHHMMSScc" digits and the offset from
/// GMT) to a time, through the 7-byte form.
pub fn cd9660_tstamp_conv17(pi: &[u8], pu: &mut Timespec) -> bool {
    let buf = [
        // year:"0001"-"9999" -> -1900
        cd9660_chars2ui(pi, 4).wrapping_sub(1900) as u8,
        // month: " 1"-"12" -> 1 - 12
        cd9660_chars2ui(&pi[4..], 2) as u8,
        // day: " 1"-"31" -> 1 - 31
        cd9660_chars2ui(&pi[6..], 2) as u8,
        // hour: " 0"-"23" -> 0 - 23
        cd9660_chars2ui(&pi[8..], 2) as u8,
        // minute:" 0"-"59" -> 0 - 59
        cd9660_chars2ui(&pi[10..], 2) as u8,
        // second:" 0"-"59" -> 0 - 59
        cd9660_chars2ui(&pi[12..], 2) as u8,
        // difference of GMT
        pi[16],
    ];

    cd9660_tstamp_conv7(&buf, pu)
}

/// `isodirino`: the inode number of a record, the byte offset of the data it describes (the
/// C's `int` sum shifted, stored into the `cdino_t`: the same bits as this `u32` wrapping
/// arithmetic).
pub fn isodirino<G: ProcGlue>(isodir: &IsoDirectoryRecord<'_>, imp: &IsoMnt<G>) -> Cdino {
    isonum_733(isodir.extent()).wrapping_add(u32::from(isonum_711(isodir.ext_attr_length())))
        << imp.im_bshift
}

/// `cd9660_test_reset`: forgets the node hash of a test thread's tables, without freeing it
/// (the memory behind it is gone with the last test's, testing.md).
#[cfg(any(test, feature = "doubles"))]
pub fn cd9660_test_reset<G: ProcGlue>() {
    core::mem::forget(vfs::<G>().cd9660.isohash.replace(None));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the date conversions, the inode numbers of records, the attributes of
    // plain ISO 9660 records, the node's attributes, the node hash and the reclaim.

    use super::*;
    use crate::fs::isofs::cd9660::cd9660_extern::ISO_FTYPE_DEFAULT;
    use crate::fs::isofs::cd9660::testcd9660::{attach_image, image, rec, test_mnt, test_node};
    use crate::fs::testfs::TestGlue;
    use hal::sys::stat::S_IFMT;

    /// `cd9660_tstamp_conv7` of a 7-byte date.
    fn conv7(d: [u8; 7]) -> (bool, Timespec) {
        let mut t = Timespec::new(-1, -1);
        let ok = cd9660_tstamp_conv7(&d, &mut t);
        (ok, t)
    }

    /// `cd9660_tstamp_conv17` of a 17-byte date.
    fn conv17(d: &[u8; 17]) -> (bool, Timespec) {
        let mut t = Timespec::new(-1, -1);
        let ok = cd9660_tstamp_conv17(d, &mut t);
        (ok, t)
    }

    #[test]
    fn conv7_counts_seconds_since_the_epoch() {
        assert_eq!(conv7([70, 1, 1, 0, 0, 0, 0]), (true, Timespec::new(0, 0)));
        assert_eq!(
            conv7([99, 12, 31, 23, 59, 59, 0]),
            (true, Timespec::new(946_684_799, 0))
        );
        assert_eq!(
            conv7([100, 2, 29, 12, 0, 0, 0]),
            (true, Timespec::new(951_825_600, 0))
        );
        // the offset from GMT is in 15-minute units: local 07:36:48 at -3h is 10:36:48 GMT
        assert_eq!(
            conv7([126, 10, 4, 7, 36, 48, (-12i8) as u8]),
            (true, Timespec::new(1_791_110_208, 0))
        );
        // an offset outside -48..=52 is unreliable and ignored
        assert_eq!(
            conv7([126, 10, 4, 7, 36, 48, 100]),
            (true, Timespec::new(1_791_099_408, 0))
        );
        // before 1970: the epoch, and false
        assert_eq!(
            conv7([69, 12, 31, 0, 0, 0, 0]),
            (false, Timespec::new(0, 0))
        );
        // the largest values a disc can hold do not overflow the C's int arithmetic
        assert!(conv7([255, 255, 255, 255, 255, 255, 52]).0);
    }

    #[test]
    fn conv17_reads_the_digits() {
        assert_eq!(
            conv17(b"2026100407364800\xf4"),
            (true, Timespec::new(1_791_110_208, 0))
        );
        assert_eq!(conv17(b"1970010100000000\x00"), (true, Timespec::new(0, 0)));
        assert_eq!(
            conv17(b"1969123123595900\x00"),
            (false, Timespec::new(0, 0))
        );
        // digits that are not digits wrap as the C's unsigned arithmetic: the year's
        // 229,977 - 1900 stored into a byte is 237, so 2137
        let (ok, t) =
            conv17(b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x00");
        assert!(ok);
        assert!(t.tv_sec > 0);
    }

    #[test]
    fn isodirino_is_the_byte_offset_of_the_data() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        assert_eq!(isodirino(&rec(&image::SUB_REC), &imp), 21 << 11);
        assert_eq!(isodirino(&rec(&image::ROOT_DOT), &imp), 20 << 11);
        // the extended attribute record comes first
        let mut r = image::FILE_REC;
        r[1] = 2;
        assert_eq!(isodirino(&rec(&r), &imp), (23 + 2) << 11);
        // the C's int sum wraps into the cdino_t
        r[2..6].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
        assert_eq!(isodirino(&rec(&r), &imp), 1 << 11);
    }

    #[test]
    fn plain_records_get_default_attributes_and_times() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        let (ip, _vp) = test_node(&imp, 21 << 11);

        let mut ino = IsoRripInode::default();
        cd9660_defattr(&rec(&image::SUB_REC), &ip, &mut ino, None);
        assert_eq!(u32::from(ino.iso_mode) & S_IFMT, S_IFDIR);
        assert_eq!(u32::from(ino.iso_mode) & 0o7777, 0o555);
        assert_eq!((ino.iso_links, ino.iso_uid, ino.iso_gid), (1, 0, 0));

        cd9660_defattr(&rec(&image::FILE_REC), &ip, &mut ino, None);
        cd9660_deftstamp(&rec(&image::FILE_REC), &ip, &mut ino, None);
        assert_eq!(u32::from(ino.iso_mode), S_IFREG | 0o555);
        assert_eq!(ino.iso_ctime, Timespec::new(1_791_110_208, 0));
        assert_eq!(ino.iso_atime, ino.iso_ctime);
        assert_eq!(ino.iso_mtime, ino.iso_ctime);
    }

    #[test]
    fn extended_attributes_give_the_permissions_owner_and_times() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        let (ip, _vp) = test_node(&imp, 23 << 11);
        // an extended attribute record in a block of its own, version 1
        let mut blk = std::vec![0u8; 2048];
        blk[0..4].copy_from_slice(&[0xe8, 0x03, 0x03, 0xe8]); // owner 1000
        blk[4..8].copy_from_slice(&[0x0a, 0x00, 0x00, 0x0a]); // group 10
        blk[8] = 0x40 | 0x04; // no execute for others, none for the group
        blk[9] = 0x10; // no read for the owner
        blk[10..27].copy_from_slice(b"1999123123595900\x00"); // ctime
        blk[27..44].copy_from_slice(b"2026100407364800\xf4"); // mtime
        blk[61..78].copy_from_slice(b"1960010100000000\x00"); // ftime: before 1970, none
        blk[180] = 1;
        let mut t = crate::fs::bio::geteblk(2048);
        t.data()[..2048].copy_from_slice(&blk);
        let mut ino = IsoRripInode::default();
        cd9660_defattr(&rec(&image::FILE_REC), &ip, &mut ino, Some(&t));
        // the owner may execute, the group and the others read
        assert_eq!(u32::from(ino.iso_mode), S_IFREG | 0o144);
        assert_eq!((ino.iso_uid, ino.iso_gid), (1000, 10));
        cd9660_deftstamp(&rec(&image::FILE_REC), &ip, &mut ino, Some(&t));
        assert_eq!(ino.iso_mtime, Timespec::new(1_791_110_208, 0));
        assert_eq!(ino.iso_ctime, Timespec::new(946_684_799, 0));
        // the ftime is before 1970: the access time is the ctime
        assert_eq!(ino.iso_atime, ino.iso_ctime);
        // another version is no record: the defaults
        t.data()[180] = 2;
        cd9660_defattr(&rec(&image::FILE_REC), &ip, &mut ino, Some(&t));
        assert_eq!(u32::from(ino.iso_mode), S_IFREG | 0o555);
        crate::fs::bio::brelse(t);
    }

    #[test]
    fn a_new_node_is_unhashed_and_zeroed_and_keeps_its_attributes_whole() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        let (ip, vp) = test_node(&imp, 40 << 11);
        assert!(!ip.i_hash.is_linked());
        assert_eq!(ip.inode(), IsoRripInode::default());
        assert!(core::ptr::eq(ip.itov(), vp));
        assert_eq!(ip.i_number, 40 << 11);
        let v = IsoRripInode {
            iso_atime: Timespec::new(-1, 999_999_999),
            iso_mtime: Timespec::new(i64::MAX, 0),
            iso_ctime: Timespec::new(i64::MIN, 1),
            iso_mode: 0o120_755,
            iso_uid: u32::MAX,
            iso_gid: 7,
            iso_links: -2,
            iso_rdev: -1,
        };
        ip.set_inode(&v);
        assert_eq!(ip.inode(), v);
        ip.update_inode(|i| i.iso_mode = 0o40555);
        assert_eq!(ip.inode().iso_mode, 0o40555);
        assert_eq!(ip.inode().iso_links, -2);
        assert!(core::ptr::eq(&*vtoi(vp), &*ip));
    }

    #[test]
    fn the_hash_finds_inserts_once_and_removes() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        let dev = imp.im_dev;
        assert!(cd9660_ihashget::<TestGlue>(dev, 21 << 11).is_none());
        let (ip, vp) = test_node(&imp, 21 << 11);
        cd9660_ihashins(ip.as_pin()).expect("hashed");
        assert!(ip.i_hash.is_linked());
        assert_eq!(
            ip.i_chain.get(),
            with_isohash::<TestGlue, _>(|h| h.cd9660_isohash(dev, 21 << 11))
        );
        // the insertion locked the vnode
        assert_ne!(ip.i_lock.status(), 0);
        let _ = vop_unlock(vp);
        // a second node of the same pair is refused, unlocked
        let (iq, vq) = test_node(&imp, 21 << 11);
        assert_eq!(cd9660_ihashins(iq.as_pin()), Err(Errno::EEXIST));
        assert!(!iq.i_hash.is_linked());
        assert_eq!(iq.i_lock.status(), 0);
        // another device or another number is another node
        assert!(cd9660_ihashget::<TestGlue>(dev + 1, 21 << 11).is_none());
        assert!(cd9660_ihashget::<TestGlue>(dev, 22 << 11).is_none());
        // the pair's vnode, referenced and locked
        let got = cd9660_ihashget::<TestGlue>(dev, 21 << 11).expect("found");
        assert!(core::ptr::eq(got, vp));
        assert_ne!(ip.i_lock.status(), 0);
        vput(got);
        cd9660_ihashrem(&ip);
        assert!(!ip.i_hash.is_linked());
        assert!(cd9660_ihashget::<TestGlue>(dev, 21 << 11).is_none());
        // a node never hashed, or removed already, is left alone
        cd9660_ihashrem(&ip);
        cd9660_ihashrem(&iq);
        crate::fs::subr::vrele(vq);
    }

    #[test]
    fn the_reclaim_unhashes_and_gives_back_the_node_and_the_device() {
        let (_g, _p) = attach_image(image::image_bytes());
        let imp = test_mnt(ISO_FTYPE_DEFAULT, |_| {});
        let devvp = imp.im_devvp;
        let (ip, vp) = test_node(&imp, 21 << 11);
        let uses = devvp.usecount();
        cd9660_ihashins(ip.as_pin()).expect("hashed");
        let _ = vop_unlock(vp);
        drop(ip);
        cd9660_reclaim(vp).expect("reclaimed");
        assert!(vp.node_arc::<IsoNode<TestGlue>>().is_none());
        assert_eq!(devvp.usecount(), uses - 1);
        assert!(cd9660_ihashget::<TestGlue>(imp.im_dev, 21 << 11).is_none());
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/isofs/cd9660/cd9660_node.h");
        assert_eq!(
            crate::reftest::int(&defs, "IN_ACCESS"),
            Some(i64::from(IN_ACCESS))
        );
        let defs = crate::reftest::defines("sys/sys/malloc.h");
        assert_eq!(
            crate::reftest::int(&defs, "M_ISOFSNODE"),
            Some(i64::from(M_ISOFSNODE))
        );
        assert_eq!(
            crate::reftest::int(&defs, "M_ISOFSMNT"),
            Some(i64::from(M_ISOFSMNT))
        );
    }
}
/* </TESTS> */
