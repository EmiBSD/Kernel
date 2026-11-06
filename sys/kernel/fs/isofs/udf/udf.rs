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
 * Copyright (c) 2001, 2002 Scott Long <scottl@freebsd.org>
 * All rights reserved.
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
 *
 * $FreeBSD: src/sys/fs/udf/udf.h,v 1.9 2004/10/29 10:40:58 phk Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `<isofs/udf/udf.h>`: the in-core UDF node (`struct unode`), the mounted volume
//! (`struct umount`), the directory stream the lookups and `readdir` walk
//! (`struct udf_dirstream`), and the block reading helpers.
//!
//! Upstream: sys/isofs/udf/udf.h @ 3ce1f3f79392
//! LZ: sys/isofs/udf/udf.rs@3c62ede68802
//!
//! Ported to OpenBSD by Pedro Martelletto in February 2005.
//!
//! ## Deviations
//! - `u_fentry` is the `malloc(M_UDFFENTRY)`ed copy of the (extended) file entry as bytes
//!   ([`Unode::u_fentry`]); its fixed part is parsed once into [`FentryHdr`] (`up->fe`), the
//!   members the C reads through `up->u_fentry->` and its `(struct file_entry *)` cast.
//! - The union `un_u` of two `long`s (`u_diroff`, `u_vatlen`) is one word with an accessor
//!   pair per name.
//! - `um_hashtbl`, `um_hashsz` and `um_hashkey` are one [`UdfHash`], set once by the mount
//!   (absent before it, the C's `NULL` table); `um_hashmtx` is a `hal::mutex::Mutexed<()>`
//!   whose guard is the chains' witness.
//! - `um_vat` is the `malloc(M_UDFMOUNT)` block `udf_vat_get` makes ([`VatBlock`]: a counted
//!   handle of the VAT's unode and the VAT's length, not a copy of the unode;
//!   `udf_subr.rs`'s deviations); `um_stbl` is the `malloc(M_UDFMOUNT)`ed copy of the sparing
//!   table as bytes, which each sparable partition map the mount reads replaces, the one
//!   replaced kept allocated as the C leaks it (D-N5.8-R3: [`Umount::set_stbl`]).
//! - `um_root_icb` is set once by the mount, after its metadata adjustment (the C's `bcopy`
//!   then its two member writes, udf_vfsops.c:384-388, are one value made first); zero
//!   before.
//! - `struct udf_dirstream`'s `bp` and `data` are one [`UdfData`] (where the current chunk
//!   is: in the file entry, or in a buffer whose token it holds), `None` before the first
//!   chunk; `buf` is the `malloc(M_UDFFID)`ed fragment buffer, `um_bsize` bytes; `error` is
//!   a `Result`, `fid_fragment` a `bool`.
//! - `RDSECTOR(devvp, sector, size, bp)`, which reads `ump` from the caller's scope, is
//!   [`rdsector`] with `ump` as an argument, returning `bread`'s pair; the shift by
//!   `um_bshift` wraps (the mount validates the block size first, D-N5.8-U3).
//! - `udf_readlblks` answers `EINVAL` before the read for a rounded size above `MAXBSIZE`
//!   (a descriptor longer than 64 KiB: the root's ICB length, a sparing table's size), where
//!   the C's `buf_map` maps pages past its slot (the user's D-N5.8-U3); it returns the
//!   buffer's token on success and releases the buffer on an error, as every C caller does.
//! - The prototypes (`udf_allocv`, `udf_hashlookup`, `udf_hashins`, `udf_hashrem`,
//!   `udf_checktag`) are the functions of `udf_vnops.rs` and `udf_vfsops.rs`.
//!
//! ## Redesign
//! - LZ's `Unode` (`Cell` members of a raw `unode_pool` item hung from `v_data`, reached by
//!   `VTOU`'s `unsafe` deref behind a tag check as a `&'static Unode`; `u_fentry` a raw
//!   `malloc` block read through an `unsafe` slice; a `queue_adapter!` hash link) ->
//!   [`Unode`], made whole where the C fills its `pool_get(PR_ZERO)` item (`udf_vget`, N5.8u2,
//!   holds the file entry's copy before the unode is hashed or hung: [`Unode::new`]) and
//!   published as an item of the glue's recorded `unode_pool` ([`Unode::alloc`], `"udfndpl"`,
//!   the UFS inode's shape, docs/IDIOMS.md's `v_data` row; D-N5.8-7), erased into the vnode's
//!   typed slot; [`VTOU`] is the slot's read and a count (`PoolArc::from_ref`), LZ's
//!   `VT_UDF` check kept. The file entry's copy is an `MArray` of `M_UDFFENTRY` owned by the
//!   unode and parsed once into [`FentryHdr`] (D-N5.8-5: no view rebuilt per `bmap`, where
//!   LZ rebuilt one per `udf_bmap_internal` call). What the C writes after publication is a
//!   word (`un_u`) or a `Link` (`u_devvp`, given back by the reclaim); the identity is plain.
//! - LZ's `Umount` (`Cell` members of a raw `malloc` block hung from `mnt_data`, reached by
//!   `VFSTOUDFFS`'s `unsafe` deref; a raw hash table slice, a raw sparing table, a raw VAT
//!   unode copy) -> [`Umount`], published where the C mallocs it (`udf_mountfs`,
//!   udf_vfsops.c:249, before the anchor is read: the VAT's `udf_vget` goes through the
//!   mount's slot during the mount, so it cannot be built whole; D-N5.8-6) in a counted
//!   `malloc(M_UDFMOUNT)` block ([`Umount::alloc`], `hal::malloc::MArc`); every member the
//!   C fills during the mount is a relaxed word under the kernel lock (`fs::word`,
//!   docs/IDIOMS.md's row of a structure used mid-fill); the hash table and the root ICB
//!   are `hal::once::OnceStatic`s set once by the mount (udf_vfsops.c:355, :384) and freed
//!   with the structure (a read is one load); the sparing table is a `hal::slot::ReadMostly`
//!   slot a later sparable map replaces (udf_vfsops.c:762, called per map of each logical
//!   volume descriptor: a compare-and-swap pair per `udf_bmap_internal` of a sparable
//!   volume, against the C's load), and so is the VAT block, which the unmount and the
//!   mount's error path take (its unode's handle names the mount back: the take ends the
//!   cycle); [`VFSTOUDFFS`] is the mount slot's typed read.
//! - LZ's lent buffers (`rdsector`/`udf_readlblks` returning `&'static Buf`, `udf_data`'s
//!   `unsafe` slice of `b_data`) -> `fs::bio`'s `BufRef` tokens: [`UdfData::Buf`] holds the
//!   chunk's token, [`UdfData::bytes`] lends its bytes. 0 `unsafe` (LZ: 4).

use hal::intr::IPL_NONE;
use hal::link::Link;
use hal::malloc::{M_WAITOK, MArc, MArray, MBox};
use hal::mutex::Mutexed;
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{PR_WAITOK, PoolArc, PoolHome};
use hal::rwlock::RecursiveRwlock;
use hal::slot::ReadMostly;
use hal::sys::endian::letoh32;
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Dev};
use hal::user::zeroed;
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use crate::assert::kpanic;
use crate::fs::bio::{DEV_BSHIFT, bread, brelse};
use crate::fs::buf::BufRef;
use crate::fs::isofs::udf::ecma167_udf::{
    DescTag, ExtfileEntry, FileEntry, IcbTag, LongAd, TAGID_EXTFENTRY, Timestamp,
    UDF_EXTFENTRY_SIZE, UDF_FENTRY_SIZE,
};
use crate::fs::isofs::udf::udf_vfsops::udf_checktag;
use crate::fs::mount::Mount;
use crate::fs::subr::vfs;
use crate::fs::vnode::{VT_UDF, Vnode};
use crate::fs::word::{I32Word, I64Word, U32Word};
use crate::proc::ProcGlue;

/// `M_UDFMOUNT`: UDF mount structures (`<sys/malloc.h>`; `bsd`'s `sys/malloc.rs` until N7
/// moves the header).
pub const M_UDFMOUNT: i32 = 140;
/// `M_UDFFENTRY`: UDF file entries.
pub const M_UDFFENTRY: i32 = 141;
/// `M_UDFFID`: UDF file IDs.
pub const M_UDFFID: i32 = 142;

/// `MAXBSIZE` (`<sys/param.h>`).
pub(crate) const MAXBSIZE: i32 = 64 * 1024;
/// `DEV_BSIZE`.
pub(crate) const DEV_BSIZE: i32 = 1 << DEV_BSHIFT;

/// `UDF_HASHTBLSIZE`.
pub const UDF_HASHTBLSIZE: i32 = 100;

/// `udfino_t`.
pub type Udfino = u32;

/// `UDF_MNT_FIND_VAT`: indicates a VAT must be found.
pub const UDF_MNT_FIND_VAT: i32 = 0x01;
/// `UDF_MNT_USES_VAT`: indicates a VAT must be used.
pub const UDF_MNT_USES_VAT: i32 = 0x02;
/// `UDF_MNT_USES_META`: indicates we are using a Metadata partition.
pub const UDF_MNT_USES_META: i32 = 0x04;

/// The members of a node's file entry that the C reads through `up->u_fentry->` (as a
/// `struct extfile_entry *`) and its `(struct file_entry *)` cast, parsed once from the copy
/// when the node is made; each as it is on the medium (little-endian: a reader applies
/// `letoh*` where the C does).
#[derive(Clone, Copy)]
pub struct FentryHdr {
    /// `udf_checktag(&up->u_fentry->tag, TAGID_EXTFENTRY) == 0`: an extended file entry
    /// (the C's test on every read; the copy never changes, so its answer does not).
    pub extended: bool,
    /// `icbtag` (the same offset in both kinds).
    pub icbtag: IcbTag,
    /// `uid`.
    pub uid: u32,
    /// `gid`.
    pub gid: u32,
    /// `perm`.
    pub perm: u32,
    /// `link_cnt`.
    pub link_cnt: u16,
    /// `inf_len`.
    pub inf_len: u64,
    /// `logblks_rec`, of the entry's own kind.
    pub logblks_rec: u64,
    /// `atime`, of the entry's own kind.
    pub atime: Timestamp,
    /// `mtime`, of the entry's own kind.
    pub mtime: Timestamp,
    /// `l_ea`, of the entry's own kind.
    pub l_ea: u32,
    /// `l_ad`, of the entry's own kind.
    pub l_ad: u32,
}

impl FentryHdr {
    /// The members of the file entry at the start of `fentry` (zero when `fentry` is
    /// shorter than the entry's fixed part, which a copy made by `udf_vget`, `size +
    /// UDF_EXTFENTRY_SIZE` bytes, never is).
    pub fn parse(fentry: &[u8]) -> FentryHdr {
        let tag: DescTag = DescTag::at(fentry, 0).unwrap_or_else(zeroed);
        let fe: FileEntry = FileEntry::at(fentry, 0).unwrap_or_else(zeroed);
        let mut h = FentryHdr {
            extended: udf_checktag(&tag, TAGID_EXTFENTRY).is_ok(),
            icbtag: fe.icbtag,
            uid: fe.uid,
            gid: fe.gid,
            perm: fe.perm,
            link_cnt: fe.link_cnt,
            inf_len: fe.inf_len,
            logblks_rec: fe.logblks_rec,
            atime: fe.atime,
            mtime: fe.mtime,
            l_ea: fe.l_ea,
            l_ad: fe.l_ad,
        };
        if h.extended {
            let x: ExtfileEntry = ExtfileEntry::at(fentry, 0).unwrap_or_else(zeroed);
            h.logblks_rec = x.logblks_rec;
            h.atime = x.atime;
            h.mtime = x.mtime;
            h.l_ea = x.l_ea;
            h.l_ad = x.l_ad;
        }
        h
    }

    /// Where the entry's `data[]` starts: `UDF_EXTFENTRY_SIZE` or `UDF_FENTRY_SIZE`.
    pub fn data_off(&self) -> usize {
        if self.extended {
            UDF_EXTFENTRY_SIZE
        } else {
            UDF_FENTRY_SIZE
        }
    }
}

/// `struct unode`: the in-core node of a file of a mounted UDF volume (see the module's
/// redesign for which members are words).
pub struct Unode<G: ProcGlue> {
    /// `u_le`: the link of the mount's hash chain.
    pub u_le: PinEntry<Unode<G>>,
    /// The item's place in `unode_pool`, for [`VTOU`]'s count.
    home: PoolHome<Unode<G>>,
    /// `u_vnode` (the C never changes it, even when `checkalias` moves the unode to an
    /// alias: udf_vfsops.c:686).
    pub u_vnode: &'static Vnode<G>,
    /// `u_devvp`, referenced by `udf_vget`, given back by `udf_reclaim`.
    pub u_devvp: Link<Vnode<G>>,
    /// `u_ump`: the mount, a counted handle for the node's life.
    pub u_ump: UmountRef<G>,
    /// `u_lock`: the vnode lock.
    pub u_lock: RecursiveRwlock,
    /// `u_dev`.
    pub u_dev: Dev,
    /// `u_ino`.
    pub u_ino: Udfino,
    /// `un_u`: `u_diroff` or `u_vatlen` (the module's deviations).
    un_u: I64Word,
    /// The members of the file entry (the module's deviations).
    pub fe: FentryHdr,
    /// `u_fentry`: the file entry, its extended attributes and allocation descriptors, then
    /// zeros (the `malloc(M_UDFFENTRY | M_ZERO)` block `udf_vget` filled).
    pub u_fentry: MArray<u8>,
}

/// A counted handle of a published [`Unode`]: the vnode's slot holds one, and so does the
/// mount's VAT block for the VAT's node.
pub type UnodeRef<G> = PoolArc<Unode<G>>;

impl<G: ProcGlue> Unode<G> {
    /// The node of the file `ino` of the mount `ump`, whose vnode is `vp` and whose file
    /// entry `udf_vget` copied into `fentry`, as the C fills its zeroed item before hashing
    /// it: `u_devvp` unset (the caller sets and references it), unhashed, unlocked.
    pub fn new(vp: &'static Vnode<G>, ump: UmountRef<G>, ino: Udfino, fentry: MArray<u8>) -> Self {
        Self {
            u_le: PinEntry::new(),
            home: PoolHome::new(),
            u_vnode: vp,
            u_devvp: Link::new(),
            u_dev: ump.um_dev,
            u_ump: ump,
            u_lock: RecursiveRwlock::new("unode"),
            u_ino: ino,
            un_u: I64Word::new(0),
            fe: FentryHdr::parse(&fentry),
            u_fentry: fentry,
        }
    }

    /// Publishes the node as an item of the glue's `unode_pool` (`pool_get(PR_WAITOK)`,
    /// which cannot fail: `panic(9)` if it does, or before `udf_init` made the pool).
    pub fn alloc(self) -> UnodeRef<G> {
        match vfs::<G>().udf.unode_pool.get_homed(PR_WAITOK, || self) {
            Some(up) => up,
            None => kpanic(format_args!("udf: pool_get(unode_pool) failed")),
        }
    }

    /// `u_diroff`: where the last lookup in this directory found its name.
    pub fn u_diroff(&self) -> i64 {
        self.un_u.get()
    }

    /// `u_diroff = off`.
    pub fn set_u_diroff(&self, off: i64) {
        self.un_u.set(off);
    }

    /// `u_vatlen`: the number of entries of the VAT this node is.
    pub fn u_vatlen(&self) -> i64 {
        self.un_u.get()
    }

    /// `u_vatlen = len`.
    pub fn set_u_vatlen(&self, len: i64) {
        self.un_u.set(len);
    }

    /// `up->u_devvp`, which `udf_vget` sets.
    pub fn devvp(&self) -> &'static Vnode<G> {
        match self.u_devvp.get() {
            Some(vp) => vp,
            None => kpanic(format_args!("unode {self:p}: no u_devvp")),
        }
    }
}

hal::pool_home!(impl[G: ProcGlue] Unode<G>, home);

hal::pin_adapter!(
    /// The mount's unode hash chains (`LIST_ENTRY(unode) u_le`), through `u_le`.
    impl[G: ProcGlue] pub UnodeHashLink<G>: Unode<G>, u_le
);

/// `LIST_HEAD(udf_hash_lh, unode)`: a chain of the unode hash.
pub type UdfHashLh<G> = PinList<UnodeHashLink<G>>;

/// `um_hashtbl`, `um_hashsz` and `um_hashkey`: the mount's unode hash.
pub struct UdfHash<G: ProcGlue> {
    /// `um_hashtbl`.
    tbl: MArray<UdfHashLh<G>>,
    /// `um_hashsz`: the table's size - 1 (a mask).
    mask: u64,
    /// `um_hashkey`.
    key: SiphashKey,
}

impl<G: ProcGlue> UdfHash<G> {
    /// The table `hashinit` made (`tbl`, `mask`) under the key `key`.
    pub fn new(tbl: MArray<UdfHashLh<G>>, mask: u64, key: SiphashKey) -> Self {
        Self { tbl, mask, key }
    }

    /// `&um_hashtbl[SipHash24(&um_hashkey, &id, sizeof(id)) & um_hashsz]`.
    pub fn chain(&self, id: Udfino) -> &UdfHashLh<G> {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&id.to_ne_bytes());
        // `& um_hashsz`, the table's size - 1: an index of the table.
        &self.tbl[(ctx.end() & self.mask) as usize]
    }
}

/// `struct umount`: a mounted UDF volume (see the module's redesign for which members are
/// words).
pub struct Umount<G: ProcGlue> {
    /// `um_flags`: the `UDF_MNT_*`.
    pub um_flags: I32Word,
    /// `um_mountp`.
    pub um_mountp: &'static Mount<G>,
    /// `um_devvp` (which `udf_vget` replaces by a device alias, as the C does: udf_vfsops.c:692).
    pub um_devvp: Link<Vnode<G>>,
    /// `um_dev`.
    pub um_dev: Dev,
    /// `um_bsize`: the logical block size.
    pub um_bsize: I32Word,
    /// `um_bshift`.
    pub um_bshift: I32Word,
    /// `um_bmask`.
    pub um_bmask: I32Word,
    /// `um_start`: the partition's first sector.
    pub um_start: U32Word,
    /// `um_realstart`.
    pub um_realstart: U32Word,
    /// `um_len`.
    pub um_len: U32Word,
    /// `um_reallen`.
    pub um_reallen: U32Word,
    /// `um_meta_start`.
    pub um_meta_start: U32Word,
    /// `um_meta_len`.
    pub um_meta_len: U32Word,
    /// `um_vat`: the `malloc(M_UDFMOUNT)` block `udf_vat_get` makes (the module's
    /// deviations).
    pub um_vat: ReadMostly<Option<MBox<VatBlock<G>>>>,
    /// `um_root_icb`, once the mount read the file set descriptor.
    um_root_icb: OnceStatic<LongAd>,
    /// `um_hashmtx`: the hash chains' lock.
    pub um_hashmtx: Mutexed<()>,
    /// The hash, once the mount made it.
    um_hashtbl: OnceStatic<UdfHash<G>>,
    /// `um_psecs`: sectors per packet.
    pub um_psecs: I32Word,
    /// `um_stbl_len`.
    pub um_stbl_len: I32Word,
    /// `um_stbl`: the sparing table the mount read last.
    um_stbl: ReadMostly<Option<MArray<u8>>>,
}

/// What `udf_vat_get`'s `malloc(M_UDFMOUNT)` block holds: the VAT's node and the length the C
/// computes into its copy (`u_vatlen`, frozen there: the node's own `un_u` word is also its
/// `u_diroff`).
pub struct VatBlock<G: ProcGlue> {
    /// The VAT's node, counted (its file entry is the VAT's map).
    pub up: UnodeRef<G>,
    /// `um_vat->u_vatlen`: the number of entries of the VAT.
    pub vatlen: i64,
}

/// A counted handle of a published [`Umount`]: the mount's slot holds one, each node one.
pub type UmountRef<G> = MArc<Umount<G>>;

impl<G: ProcGlue> Umount<G> {
    /// The structure of the mount `mp` of the device `devvp`, as `udf_mountfs` fills its
    /// `malloc(M_ZERO)` block before reading the volume: the rest zero.
    pub fn new(mp: &'static Mount<G>, devvp: &'static Vnode<G>) -> Self {
        Self {
            um_flags: I32Word::new(0),
            um_mountp: mp,
            um_devvp: Link::to(devvp),
            um_dev: devvp.rdev(),
            um_bsize: I32Word::new(0),
            um_bshift: I32Word::new(0),
            um_bmask: I32Word::new(0),
            um_start: U32Word::new(0),
            um_realstart: U32Word::new(0),
            um_len: U32Word::new(0),
            um_reallen: U32Word::new(0),
            um_meta_start: U32Word::new(0),
            um_meta_len: U32Word::new(0),
            um_vat: ReadMostly::new(None),
            um_root_icb: OnceStatic::new(),
            um_hashmtx: Mutexed::new(IPL_NONE, ()),
            um_hashtbl: OnceStatic::new(),
            um_psecs: I32Word::new(0),
            um_stbl_len: I32Word::new(0),
            um_stbl: ReadMostly::new(None),
        }
    }

    /// Publishes the structure in a counted `malloc(M_UDFMOUNT, M_WAITOK)` block (which
    /// cannot fail: `panic(9)` if it does).
    pub fn alloc(self) -> UmountRef<G> {
        match MArc::new(M_UDFMOUNT, M_WAITOK, self) {
            Some(ump) => ump,
            None => kpanic(format_args!("udf: malloc(M_UDFMOUNT) failed")),
        }
    }

    /// `ump->um_devvp`.
    pub fn devvp(&self) -> &'static Vnode<G> {
        match self.um_devvp.get() {
            Some(vp) => vp,
            None => kpanic(format_args!("umount {self:p}: no um_devvp")),
        }
    }

    /// `ump->um_root_icb`: zero before the mount set it (`M_ZERO`).
    pub fn root_icb(&self) -> LongAd {
        self.um_root_icb.get().copied().unwrap_or_else(zeroed)
    }

    /// `ump->um_root_icb = icb`, the file set descriptor's root ICB with the mount's
    /// metadata adjustment made: once (`panic(9)` twice).
    pub fn set_root_icb(&self, icb: LongAd) {
        if self.um_root_icb.set(icb).is_err() {
            kpanic(format_args!("umount {self:p}: um_root_icb twice"));
        }
    }

    /// The hash, `None` before the mount made it (the C's `NULL` table).
    pub fn hashtbl(&self) -> Option<&UdfHash<G>> {
        self.um_hashtbl.get()
    }

    /// `um_hashtbl = hashinit(..)` and its key: once (`panic(9)` twice).
    pub fn set_hashtbl(&self, h: UdfHash<G>) {
        if self.um_hashtbl.set(h).is_err() {
            kpanic(format_args!("umount {self:p}: um_hashtbl twice"));
        }
    }

    /// `f` of the sparing table's bytes, `None` when the volume has none (`um_stbl ==
    /// NULL`); `f` runs inside the slot's read and must not sleep.
    pub fn with_stbl<R>(&self, f: impl FnOnce(Option<&[u8]>) -> R) -> R {
        f(self.um_stbl.read().as_ref().map(|t| &t[..]))
    }

    /// `ump->um_stbl = malloc(..)` and its bytes, by `udf_get_spartmap` for each sparable
    /// partition map: the table read last is the one the block map uses.
    pub fn set_stbl(&self, t: MArray<u8>) {
        if let Some(old) = self.um_stbl.replace(Some(t)) {
            // The C overwrites `um_stbl` without freeing it (udf_vfsops.c:762): the replaced
            // table stays allocated and counted under M_UDFMOUNT for ever, so vmstat -m's
            // `UDF mount` row stays the C's (D-N5.8-R3, the user's U5 rule). Pending the
            // user's question (with udf_mountfs's leaks on its error path, N5.8u2): freeing
            // it instead is `drop(old)` here.
            core::mem::forget(old);
        }
    }

    /// `ump->um_vat`: the VAT's node and its length, `None` when there is none. The slot's
    /// read ends here, so the caller may sleep with the handle.
    pub fn vat(&self) -> Option<(UnodeRef<G>, i64)> {
        self.um_vat
            .read()
            .as_ref()
            .map(|v| (PoolArc::clone(&v.up), v.vatlen))
    }
}

/// `VTOU(vp)`: the unode of a UDF vnode, a counted handle (`panic(9)` on a vnode that has
/// none, LZ's `VT_UDF` check kept).
#[allow(non_snake_case, reason = "the C macro's name")]
pub fn VTOU<G: ProcGlue>(vp: &Vnode<G>) -> UnodeRef<G> {
    match vp.node_with(|up: &Unode<G>| PoolArc::from_ref(up)) {
        Some(Some(up)) if vp.tag() == VT_UDF => up,
        _ => kpanic(format_args!("VTOU: vnode {vp:p} has no unode")),
    }
}

/// Where the data `udf_readatoffset` found is (its `*bp` and `*data`).
pub enum UdfData {
    /// At this offset of the node's `u_fentry`: the file data recorded in the allocation
    /// descriptor area (no buffer was read).
    Entry(usize),
    /// At this offset of the buffer read, whose token this holds (`*bp`).
    Buf(BufRef, usize),
}

impl UdfData {
    /// `*data`: at most `size` bytes from where the data starts, in `up`'s file entry or in
    /// the buffer (fewer when the copy or the buffer ends first).
    pub fn bytes<'a, G: ProcGlue>(&'a self, up: &'a Unode<G>, size: i32) -> &'a [u8] {
        let (bytes, off): (&[u8], usize) = match self {
            UdfData::Entry(off) => (&up.u_fentry, *off),
            UdfData::Buf(t, off) => (t.bytes(), *off),
        };
        let size = usize::try_from(size).unwrap_or(0);
        let end = off.saturating_add(size).min(bytes.len());
        bytes.get(off..end).unwrap_or(&[])
    }
}

/// `struct udf_dirstream`: a walk over the file identifier descriptors of a directory
/// (`udf_opendir`, N5.8u2, makes one in `udf_ds_pool`).
pub struct UdfDirstream<G: ProcGlue> {
    /// `node`: the directory.
    pub node: UnodeRef<G>,
    /// `ump`.
    pub ump: UmountRef<G>,
    /// `bp` and `data`: where the current chunk is (the module's deviations).
    pub data: Option<UdfData>,
    /// `buf`: the fragment buffer, `um_bsize` bytes of `M_UDFFID`.
    pub buf: Option<MArray<u8>>,
    /// `fsize`: the size of the directory.
    pub fsize: i32,
    /// `off`: the offset of the next descriptor in the current chunk.
    pub off: i32,
    /// `this_off`: the directory offset after the descriptor returned last.
    pub this_off: i32,
    /// `offset`: the directory offset of the current chunk.
    pub offset: i32,
    /// `size`: the size of the current chunk.
    pub size: i32,
    /// `error`.
    pub error: Result<(), Errno>,
    /// `fid_fragment`: the descriptor returned last is in `buf`.
    pub fid_fragment: bool,
}

/// `VFSTOUDFFS(mp)`: the umount of a UDF mount, a counted handle (`panic(9)` on a mount that
/// has none: not a mounted UDF volume).
#[allow(non_snake_case, reason = "the C macro's name")]
pub fn VFSTOUDFFS<G: ProcGlue>(mp: &Mount<G>) -> UmountRef<G> {
    match mp.node_arc::<Umount<G>>() {
        Some(ump) => ump,
        None => kpanic(format_args!("VFSTOUDFFS: mount {mp:p} has no umount")),
    }
}

/// `RDSECTOR(devvp, sector, size, bp)`: read `size` bytes at logical block `sector` of the
/// device. The block layer refers to things in terms of 512 byte blocks by default.
pub fn rdsector<G: ProcGlue>(
    ump: &Umount<G>,
    devvp: &'static Vnode<G>,
    sector: Daddr,
    size: i32,
) -> (BufRef, Result<(), Errno>) {
    // `um_bshift` is the validated block size's (D-N5.8-U3); the shift wraps all the same.
    let blkno = sector.wrapping_shl(ump.um_bshift.get() as u32) / Daddr::from(DEV_BSIZE);
    bread(&devvp.bufs, blkno, size)
}

/// `udf_readlblks(ump, sector, size, bp)`: read the logical blocks holding `size` bytes from
/// `sector` on: the buffer's token; the read's error once the buffer is released, and
/// `EINVAL` before any read for a rounded size above `MAXBSIZE` (the module's deviations).
pub fn udf_readlblks<G: ProcGlue>(
    ump: &Umount<G>,
    sector: i32,
    size: i32,
) -> Result<BufRef, Errno> {
    let bmask = ump.um_bmask.get();
    let size = size.wrapping_add(bmask) & !bmask;
    // The C's callers hold the size as a `uint32_t` (the root ICB's length, a sparing
    // table's): a negative `int` is a size above `MAXBSIZE` too (D-N5.8-U3).
    if size as u32 > MAXBSIZE as u32 {
        return Err(Errno::EINVAL);
    }
    let (t, error) = rdsector(ump, ump.devvp(), Daddr::from(sector), size);
    match error {
        Ok(()) => Ok(t),
        Err(e) => {
            brelse(t);
            Err(e)
        }
    }
}

/// `udf_getid(icb)`: produce a suitable file number from an ICB. The passed in ICB is
/// expected to be in little endian. If the fileno resolves to 0, we might be in big trouble.
/// Assumes the ICB is a `long_ad`. This struct is compatible with `short_ad`, but not
/// `ext_ad`.
pub fn udf_getid(icb: &LongAd) -> Udfino {
    letoh32(icb.loc.lb_num)
}

/// `unicode_t`.
pub type Unicode = u16;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use std::vec;

    use hal::sys::endian::{letoh16, letoh64};

    use super::*;
    use crate::fs::bio::BCSTATS;
    use crate::fs::isofs::udf::ecma167_udf::{Timestamp, UDF_ICB_FILETYPE_RANDOMACCESS};
    use crate::fs::isofs::udf::testudf::mkudf::{B, file_entry, long_ad, short_ad, timestamp};
    use crate::fs::isofs::udf::testudf::{attach, fentry_copy, test_node, test_ump};
    use crate::testutil::{malloc_out, malloc_requests};

    /// The bytes of a timestamp.
    fn ts(t: &Timestamp) -> [u8; 12] {
        let mut b = [0u8; 12];
        assert!(Timestamp::put(&mut b, 0, t));
        b
    }

    #[test]
    fn the_file_entry_is_parsed_once_of_either_kind() {
        let (_g, _p) = attach(vec![0; B]);
        let ads = short_ad(12, 5);
        for efe in [false, true] {
            let fe = file_entry(
                4,
                efe,
                UDF_ICB_FILETYPE_RANDOMACCESS,
                0,
                0x1884,
                501,
                12,
                &ads,
            );
            let h = FentryHdr::parse(&fentry_copy(&fe));
            assert_eq!(h.extended, efe);
            assert_eq!(h.icbtag.file_type, UDF_ICB_FILETYPE_RANDOMACCESS);
            assert_eq!(letoh16(h.icbtag.strat_type), 4);
            assert_eq!(
                (
                    letoh32(h.uid),
                    letoh32(h.gid),
                    letoh32(h.perm),
                    letoh16(h.link_cnt)
                ),
                (501, 20, 0x1884, 1)
            );
            assert_eq!(letoh64(h.inf_len), 12);
            // The members whose offsets differ between the two kinds.
            assert_eq!(letoh64(h.logblks_rec), if efe { 3 } else { 2 });
            assert_eq!(ts(&h.atime), timestamp(None));
            assert_eq!(
                ts(&h.mtime),
                if efe {
                    timestamp(Some(-60))
                } else {
                    timestamp(None)
                }
            );
            assert_eq!((letoh32(h.l_ea), letoh32(h.l_ad)), (0, 8));
            assert_eq!(h.data_off(), if efe { 216 } else { 176 });
        }
        // Not a file entry, or a damaged tag: the plain entry's view.
        let mut fe = file_entry(4, true, 5, 0, 0x1884, 501, 12, &ads);
        fe[4] ^= 1;
        let h = FentryHdr::parse(&fentry_copy(&fe));
        assert!(!h.extended);
        assert_eq!(letoh32(h.l_ad), 0);
        // Shorter than a fixed part: zeros.
        let h = FentryHdr::parse(&[0xffu8; 100]);
        assert_eq!((h.extended, h.uid, h.l_ad), (false, 0, 0));
    }

    #[test]
    fn the_mount_structure_keeps_the_root_icb_and_its_tables() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|_| {});
        let mut l = long_ad(0xc000_0800, 0x1234_5678);
        l[8..10].copy_from_slice(&3u16.to_le_bytes());
        l[12..16].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        let icb = LongAd::at(&l, 0).unwrap();
        // Zero before the mount sets it (M_ZERO).
        assert_eq!(udf_getid(&ump.root_icb()), 0);
        ump.set_root_icb(icb);
        let back = ump.root_icb();
        let mut b = [0u8; 16];
        assert!(LongAd::put(&mut b, 0, &back));
        assert_eq!(b, l);
        assert_eq!(udf_getid(&back), 0x1234_5678);
        assert!(ump.with_stbl(|t| t.is_none()));
        assert!(ump.vat().is_none());
        assert!(ump.hashtbl().is_some());
        // The mount's slot answers with the structure.
        assert!(MArc::ptr_eq(&VFSTOUDFFS(ump.um_mountp), &ump));
    }

    /// Two sparable partition maps (or two logical volume descriptors) each read a sparing
    /// table (udf_vfsops.c:762): the second replaces the first, which stays allocated as
    /// the C leaks it (D-N5.8-R3), so `M_UDFMOUNT`'s requests and bytes out are the C's.
    #[test]
    fn a_second_sparing_table_replaces_the_first() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|_| {});
        let table = |fill: u8, n: usize| {
            let Some(t) = MArray::new(M_UDFMOUNT, M_WAITOK, n, |_| fill) else {
                std::panic!("malloc");
            };
            t
        };
        let (n0, _) = malloc_requests(M_UDFMOUNT);
        let out0 = malloc_out(M_UDFMOUNT);
        ump.set_stbl(table(1, 64));
        let out1 = malloc_out(M_UDFMOUNT);
        ump.set_stbl(table(2, 80));
        assert!(ump.with_stbl(|t| t == Some(&[2u8; 80][..])));
        assert_eq!(malloc_requests(M_UDFMOUNT).0, n0 + 2);
        // Both blocks are out: the first is never freed, as the C's (a free of the first
        // would leave out1 - 64 + 80).
        assert_eq!(out1, out0 + 64);
        assert_eq!(malloc_out(M_UDFMOUNT), out1 + 80);
    }

    #[test]
    fn readlblks_refuses_a_size_above_maxbsize_before_reading() {
        let (_g, _p) = attach(vec![0; 40 * B]);
        let ump = test_ump(|_| {});
        let busy = BCSTATS.busymapped.load(Ordering::Relaxed);
        // 64 KiB, rounded from below: read.
        let t = udf_readlblks(&ump, 1, 63 * 1024 + 1).unwrap();
        assert_eq!(t.bytes().len(), 64 * 1024);
        drop(t);
        // The root ICB of 128 KiB, one byte past 64 KiB, a negative `int`: refused, no read.
        for size in [128 * 1024, 64 * 1024 + 1, -2048] {
            assert_eq!(
                udf_readlblks(&ump, 1, size).err(),
                Some(Errno::EINVAL),
                "size {size}"
            );
        }
        assert_eq!(BCSTATS.busymapped.load(Ordering::Relaxed), busy);
    }

    #[test]
    fn the_data_of_a_node_is_lent_within_its_bounds() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|_| {});
        let fe = file_entry(3, true, 5, 3, 0x1884, 0, 5, b"hello");
        let (up, vp) = test_node(&ump, 3, &fe);
        assert!(PoolArc::ptr_eq(&VTOU(vp), &up));
        assert_eq!(up.u_dev, ump.um_dev);
        let d = UdfData::Entry(216);
        assert_eq!(d.bytes(&up, 5), b"hello");
        assert_eq!(d.bytes(&up, 3), b"hel");
        assert_eq!(d.bytes(&up, -1), b"");
        // Past the copy's end (5 + 216 bytes): what is left.
        assert_eq!(UdfData::Entry(219).bytes(&up, 100), b"lo");
        assert_eq!(UdfData::Entry(9999).bytes(&up, 4), b"");
        up.set_u_diroff(40);
        assert_eq!((up.u_diroff(), up.u_vatlen()), (40, 40));
    }
}
/* </TESTS> */
