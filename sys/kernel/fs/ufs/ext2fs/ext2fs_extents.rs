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
 * Copyright (c) 2012, 2010 Zheng Liu <lz@freebsd.org>
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
 * $FreeBSD: head/sys/fs/ext2fs/ext2_extents.h 262623 2014-02-28 21:25:32Z pfg $
 */
/*-
 * Copyright (c) 2010 Zheng Liu <lz@freebsd.org>
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
 * $FreeBSD: head/sys/fs/ext2fs/ext2_extents.c 254260 2013-08-12 21:34:48Z pfg $
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ext2fs/ext2fs_extents.h>`: the ext4 extent tree on disk (`struct ext4_extent`,
//! `struct ext4_extent_index`, `struct ext4_extent_header`) and in core (the extent cache and
//! the path to an extent).
//!
//! This module is also the home of `ext2fs_extents.c`: finding a block in the extent cache
//! (`ext4_ext_in_cache`), filling the cache (`ext4_ext_put_cache`) and walking the tree down
//! to the extent of a logical block (`ext4_ext_find_extent`, with the binary searches of an
//! index node and of a leaf).
//!
//! Upstream: sys/ufs/ext2fs/ext2fs_extents.h @ 3ce1f3f79392
//! Upstream: sys/ufs/ext2fs/ext2fs_extents.c @ 3ce1f3f79392
//! LZ: sys/ufs/ext2fs/ext2fs_extents.rs@3c62ede68802
//!
//! Neither file has an `$OpenBSD$` line; both keep their FreeBSD notices.
//!
//! ## Deviations
//! - The three on-disk structures are `hal::abi_pod!` images (12 bytes each, pinned below),
//!   little-endian on disk, read in native order as the C reads them in place.
//! - The prototypes of the header are the functions of the `.c`'s part of this module.
//! - `ext4_ext_find_extent` returns `Option<&mut Ext4ExtentPath>` (the C's path or NULL).
//!   Before searching a node it checks that the node's `eh_ecount` entries fit in it (the
//!   60 bytes of `e2di_blocks` for the root, the buffer for the others): a node that claims
//!   more is a corrupt tree, whose search the C would run past the node; here the search
//!   stops and the path comes back with no extent (the callers' "no extent", `EIO`). The
//!   callers release `ep_bp` in every case.
//! - The binary searches keep the C's result for a block before the node's first entry: the
//!   entry "before the first", which is the node's header read as an entry (12 bytes as
//!   well, so inside the node). They do not take the inode, which the C's pass and never
//!   read.
//! - The root level is searched in a copy of the inode's 60 bytes of `e2di_blocks`, where
//!   the C reads them in place: one copy of 60 bytes per lookup (extents are read-only,
//!   `EXT4F_RO_INCOMPAT_SUPP`, so the copy cannot go stale under the search).
//!
//! ## Redesign
//! - LZ's path held raw pointers into the tree node (`ep_ext`, `ep_index`, `ep_header`: into
//!   the dinode's `e2di_blocks` or a lent buffer `ep_bp`, read with `read_unaligned`: six
//!   `unsafe` sites, and one more for the dinode's address) -> [`Ext4ExtentPath`] holds the
//!   node's bytes (the buffer's token `ep_bp`, a `BufRef` of the kernel's cache, or the copy
//!   of the root) and the entries' slots in it (`ep_ext`, `ep_index`: the entry at byte
//!   `12 * slot`, slot 0 being the header, the C's "entry before the first"), each read by
//!   `pod_from_bytes` of its 12 bytes. The searches are the C's loops over the same entries;
//!   `ep_header` is the node's first bytes, so it is gone as a member ([`Ext4ExtentPath::header`]).

use core::mem::size_of;

use hal::sys::types::Daddr;
use hal::user::pod_from_bytes;

use super::ext2fs::{Ext2Superblock, fsbtodb};
use super::ext2fs_dinode::{NDADDR, NIADDR};
use crate::fs::bio::{bread, brelse};
use crate::fs::buf::BufRef;
use crate::fs::ufs::ufs::inode::Inode;
use crate::proc::ProcGlue;

/// `EXT4_EXT_MAGIC`: the magic number of an extent tree header.
pub const EXT4_EXT_MAGIC: u16 = 0xf30a;

/// `EXT4_EXT_CACHE_NO`: nothing cached.
pub const EXT4_EXT_CACHE_NO: u32 = 0;
/// `EXT4_EXT_CACHE_GAP`: the cache holds a gap between extents.
pub const EXT4_EXT_CACHE_GAP: u32 = 1;
/// `EXT4_EXT_CACHE_IN`: the cache holds an extent.
pub const EXT4_EXT_CACHE_IN: u32 = 2;

/// `struct ext4_extent`: an ext4 file system extent on disk.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ext4Extent {
    /// `e_blk`: first logical block.
    pub e_blk: u32,
    /// `e_len`: number of blocks.
    pub e_len: u16,
    /// `e_start_hi`: high 16 bits of physical block.
    pub e_start_hi: u16,
    /// `e_start_lo`: low 32 bits of physical block.
    pub e_start_lo: u32,
}

/// `struct ext4_extent_index`: an extent index on disk.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ext4ExtentIndex {
    /// `ei_blk`: indexes logical blocks.
    pub ei_blk: u32,
    /// `ei_leaf_lo`: points to physical block of the next level.
    pub ei_leaf_lo: u32,
    /// `ei_leaf_hi`: high 16 bits of physical block.
    pub ei_leaf_hi: u16,
    /// `ei_unused`.
    pub ei_unused: u16,
}

/// `struct ext4_extent_header`: the extent tree header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ext4ExtentHeader {
    /// `eh_magic`: magic number: `EXT4_EXT_MAGIC`.
    pub eh_magic: u16,
    /// `eh_ecount`: number of valid entries.
    pub eh_ecount: u16,
    /// `eh_max`: capacity of store in entries.
    pub eh_max: u16,
    /// `eh_depth`: the depth of the extent tree.
    pub eh_depth: u16,
    /// `eh_gen`: generation of the extent tree.
    pub eh_gen: u32,
}

/// `struct ext4_extent_cache`: the saved cached extent.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ext4ExtentCache {
    /// `ec_start`: extent start.
    pub ec_start: Daddr,
    /// `ec_blk`: logical block.
    pub ec_blk: u32,
    /// `ec_len`.
    pub ec_len: u32,
    /// `ec_type`: `EXT4_EXT_CACHE_*`.
    pub ec_type: u32,
}

hal::abi_pod!(Ext4Extent {
    e_blk: u32,
    e_len: u16,
    e_start_hi: u16,
    e_start_lo: u32,
});
hal::abi_pod!(Ext4ExtentIndex {
    ei_blk: u32,
    ei_leaf_lo: u32,
    ei_leaf_hi: u16,
    ei_unused: u16,
});
hal::abi_pod!(Ext4ExtentHeader {
    eh_magic: u16,
    eh_ecount: u16,
    eh_max: u16,
    eh_depth: u16,
    eh_gen: u32,
});

/// The bytes of a tree node's header and of each of its entries.
const EXT4_ENTRY_SIZE: usize = 12;

/// The bytes of the root node, the inode's `e2di_blocks`.
const EXT4_ROOT_SIZE: usize = (NDADDR + NIADDR) * size_of::<u32>();

/// `struct ext4_extent_path`: the saved path to some extent: the node the search stopped in
/// (the buffer `ep_bp` holds, or the root copied out of the inode) and the slots of the
/// entries it found there (slot 0 is the node's header, entry `i` is slot `i + 1`).
pub struct Ext4ExtentPath {
    /// `ep_depth`.
    pub ep_depth: u16,
    /// `ep_bp`: the buffer holding this level's node (none for the root, in the inode).
    pub ep_bp: Option<BufRef>,
    /// The root node: a copy of the inode's `e2di_blocks`.
    ep_root: [u8; EXT4_ROOT_SIZE],
    /// `ep_ext`: the slot of the extent found in a leaf (`None` for the C's NULL).
    pub ep_ext: Option<usize>,
    /// `ep_index`: the slot of the index entry followed in an index node.
    pub ep_index: Option<usize>,
}

impl Ext4ExtentPath {
    /// A path level with nothing found yet.
    pub fn new() -> Self {
        Self {
            ep_depth: 0,
            ep_bp: None,
            ep_root: [0; EXT4_ROOT_SIZE],
            ep_ext: None,
            ep_index: None,
        }
    }

    /// The node's bytes: the buffer's, or the root's below no buffer.
    fn node(&self) -> &[u8] {
        match &self.ep_bp {
            Some(t) => t.bytes(),
            None => &self.ep_root,
        }
    }

    /// The 12 bytes of slot `slot` of the node as a `T` (zero past the node).
    fn entry<T: hal::user::AbiPod + Default>(&self, slot: usize) -> T {
        let at = slot * EXT4_ENTRY_SIZE;
        self.node()
            .get(at..at + EXT4_ENTRY_SIZE)
            .and_then(pod_from_bytes)
            .unwrap_or_default()
    }

    /// `*path->ep_header`: the node's header.
    pub fn header(&self) -> Ext4ExtentHeader {
        self.entry(0)
    }

    /// `*path->ep_ext`: a copy of the extent the path found, `None` for the C's NULL.
    pub fn ext(&self) -> Option<Ext4Extent> {
        self.ep_ext.map(|s| self.entry(s))
    }

    /// `*path->ep_index`: a copy of the index entry the path followed.
    pub fn index(&self) -> Option<Ext4ExtentIndex> {
        self.ep_index.map(|s| self.entry(s))
    }
}

impl Default for Ext4ExtentPath {
    fn default() -> Self {
        Self::new()
    }
}

/// The number of entries of the path's node, or `None` when its header claims more entries
/// than the node holds (see the module's deviations).
fn ext4_ext_node_entries(path: &Ext4ExtentPath) -> Option<usize> {
    let room = path.node().len();
    let n = usize::from(path.header().eh_ecount);
    (n <= room.saturating_sub(EXT4_ENTRY_SIZE) / EXT4_ENTRY_SIZE).then_some(n)
}

/// `ext4_ext_binsearch_index`: the index entry of the path's node (with `n` entries) whose
/// subtree holds `lbn`, into `path.ep_index`.
fn ext4_ext_binsearch_index(path: &mut Ext4ExtentPath, lbn: Daddr, n: usize) {
    let (mut l, mut r) = (1usize, n);
    while l <= r {
        let m = l + (r - l) / 2;
        let ei: Ext4ExtentIndex = path.entry(m);
        if lbn < Daddr::from(ei.ei_blk) {
            r = m - 1;
        } else {
            l = m + 1;
        }
    }

    path.ep_index = Some(l - 1);
}

/// `ext4_ext_binsearch`: the extent of the path's leaf (with `n` entries) that may hold
/// `lbn`, into `path.ep_ext` (left as it is for an empty leaf).
fn ext4_ext_binsearch(path: &mut Ext4ExtentPath, lbn: Daddr, n: usize) {
    if n == 0 {
        return;
    }

    let (mut l, mut r) = (1usize, n);
    while l <= r {
        let m = l + (r - l) / 2;
        let e: Ext4Extent = path.entry(m);
        if lbn < Daddr::from(e.e_blk) {
            r = m - 1;
        } else {
            l = m + 1;
        }
    }

    path.ep_ext = Some(l - 1);
}

/// `ext4_ext_in_cache`: find a block in the ext4 extent cache. Fills `ep` and returns the
/// cache's `EXT4_EXT_CACHE_*` type when `lbn` is in the cached range, `EXT4_EXT_CACHE_NO`
/// otherwise.
pub fn ext4_ext_in_cache<G: ProcGlue>(ip: &Inode<G>, lbn: Daddr, ep: &mut Ext4Extent) -> u32 {
    let ecp = ip.i_e2din().i_e2fs_ext_cache();
    let mut ret = EXT4_EXT_CACHE_NO;

    // cache is invalid
    if ecp.ec_type == EXT4_EXT_CACHE_NO {
        return ret;
    }

    if lbn >= Daddr::from(ecp.ec_blk) && lbn < Daddr::from(ecp.ec_blk.wrapping_add(ecp.ec_len)) {
        ep.e_blk = ecp.ec_blk;
        ep.e_start_lo = (ecp.ec_start & 0xffff_ffff) as u32;
        ep.e_start_hi = ((ecp.ec_start >> 32) & 0xffff) as u16;
        ep.e_len = ecp.ec_len as u16;
        ret = ecp.ec_type;
    }
    ret
}

/// `ext4_ext_put_cache`: put an extent in the ext4 cache, as `type_` (`EXT4_EXT_CACHE_*`).
pub fn ext4_ext_put_cache<G: ProcGlue>(ip: &Inode<G>, ep: &Ext4Extent, type_: u32) {
    ip.i_e2din().set_i_e2fs_ext_cache(&Ext4ExtentCache {
        ec_type: type_,
        ec_blk: ep.e_blk,
        ec_len: u32::from(ep.e_len),
        ec_start: (Daddr::from(ep.e_start_hi) << 32) | Daddr::from(ep.e_start_lo),
    });
}

/// `ext4_ext_find_extent`: find the extent of logical block `lbn` of `ip`. `None` when the
/// inode holds no extent tree or an index block cannot be read; otherwise the path, whose
/// `ep_ext` is the extent (none if there is none) and whose `ep_bp` holds the leaf's buffer
/// below the root, which the caller releases.
pub fn ext4_ext_find_extent<'a, G: ProcGlue>(
    fs: &Ext2Superblock,
    ip: &Inode<G>,
    lbn: Daddr,
    path: &'a mut Ext4ExtentPath,
) -> Option<&'a mut Ext4ExtentPath> {
    path.ep_root = ip.i_e2din().e2di_shortlink();
    let eh = path.header();
    if eh.eh_magic != EXT4_EXT_MAGIC {
        return None;
    }

    let mut i = eh.eh_depth;
    while i != 0 {
        let Some(n) = ext4_ext_node_entries(path) else {
            path.ep_ext = None;
            return Some(path);
        };
        ext4_ext_binsearch_index(path, lbn, n);
        path.ep_depth = 0;
        path.ep_ext = None;

        let ei = path.index().unwrap_or_default();
        let nblk = (Daddr::from(ei.ei_leaf_hi) << 32) | Daddr::from(ei.ei_leaf_lo);
        if let Some(t) = path.ep_bp.take() {
            brelse(t);
        }
        let (t, error) = bread(&ip.i_devvp().bufs, fsbtodb(fs, nblk), fs.e2fs_fsize.get());
        if error.is_err() {
            brelse(t);
            return None;
        }
        path.ep_bp = Some(t);
        i -= 1;
    }

    path.ep_depth = i;
    path.ep_ext = None;
    path.ep_index = None;

    if let Some(n) = ext4_ext_node_entries(path) {
        ext4_ext_binsearch(path, lbn, n);
    }
    Some(path)
}

const _: () = {
    assert!(size_of::<Ext4Extent>() == 12);
    assert!(size_of::<Ext4ExtentIndex>() == 12);
    assert!(size_of::<Ext4ExtentHeader>() == 12);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the extent tree: the constants against the C header,
    // `ext4_ext_find_extent` over a root node held in an inode's dinode (depth 0) and over the
    // test image's two-level tree read through the buffer cache (depth 1), and the extent
    // cache.

    use std::{assert, assert_eq};

    use super::super::ext2fs::Ext2Superblock;
    use super::super::testext2fs::newfs::{self, EXTLEAF};
    use super::super::testext2fs::{attach, detached, superblock, ufsmount};
    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::testffs;
    use crate::testutil;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ext2fs/ext2fs_extents.h");
        for (name, value) in [
            ("EXT4_EXT_MAGIC", i64::from(EXT4_EXT_MAGIC)),
            ("EXT4_EXT_CACHE_NO", i64::from(EXT4_EXT_CACHE_NO)),
            ("EXT4_EXT_CACHE_GAP", i64::from(EXT4_EXT_CACHE_GAP)),
            ("EXT4_EXT_CACHE_IN", i64::from(EXT4_EXT_CACHE_IN)),
        ] {
            assert_eq!(crate::reftest::int(&defs, name), Some(value), "{name}");
        }
    }

    /// An inode whose block array is an extent tree root: `ecount` entries of `extents`
    /// (`(first logical block, length, first disk block)`), with `magic`.
    fn inode(magic: u16, ecount: u16, extents: &[(u32, u16, u32)]) -> &'static Inode<TestGlue> {
        testutil::install();
        let ip = detached(&ufsmount(Ext2Superblock::new()), 20);
        let d = ip.i_e2din();
        d.set_i_e2fs_block(0, u32::from(magic) | u32::from(ecount) << 16);
        d.set_i_e2fs_block(1, 4); // eh_max, eh_depth 0
        for (i, &(blk, len, start)) in extents.iter().enumerate() {
            d.set_i_e2fs_block(3 + 3 * i, blk);
            d.set_i_e2fs_block(4 + 3 * i, u32::from(len));
            d.set_i_e2fs_block(5 + 3 * i, start);
        }
        ip
    }

    /// `ext4_ext_find_extent` at depth 0: the extent found, if any.
    fn find(ip: &Inode<TestGlue>, lbn: Daddr) -> Option<Option<Ext4Extent>> {
        let fs = ip.i_e2fs();
        let mut path = Ext4ExtentPath::new();
        ext4_ext_find_extent(fs, ip, lbn, &mut path).map(|p| {
            assert!(p.ep_bp.is_none());
            assert_eq!(p.ep_depth, 0);
            p.ext()
        })
    }

    fn e(blk: u32, len: u16, start: u32) -> Ext4Extent {
        Ext4Extent {
            e_blk: blk,
            e_len: len,
            e_start_hi: 0,
            e_start_lo: start,
        }
    }

    #[test]
    fn the_root_leaf_is_searched_for_the_extent_at_or_before_a_block() {
        let (_g, _p) = attach(newfs::image(true));
        let ip = inode(
            EXT4_EXT_MAGIC,
            3,
            &[(5, 4, 100), (10, 2, 200), (20, 1, 300)],
        );
        assert_eq!(find(ip, 5), Some(Some(e(5, 4, 100))));
        assert_eq!(find(ip, 8), Some(Some(e(5, 4, 100))));
        assert_eq!(find(ip, 10), Some(Some(e(10, 2, 200))));
        // Past an extent's end the C still answers with it; the callers do the arithmetic.
        assert_eq!(find(ip, 15), Some(Some(e(10, 2, 200))));
        assert_eq!(find(ip, 1000), Some(Some(e(20, 1, 300))));

        // Before the first extent: the entry before it, the header read as an extent.
        let mut path = Ext4ExtentPath::new();
        let _ = ext4_ext_find_extent(ip.i_e2fs(), ip, 2, &mut path);
        assert_eq!(path.ep_ext, Some(0));
        assert_eq!(path.header().eh_ecount, 3);

        // An empty leaf finds nothing; no tree at all is no path.
        assert_eq!(find(inode(EXT4_EXT_MAGIC, 0, &[]), 3), Some(None));
        assert_eq!(find(inode(0xef53, 1, &[(0, 1, 9)]), 0), None);
        // A header claiming more entries than the root holds (4) is a corrupt tree.
        assert_eq!(find(inode(EXT4_EXT_MAGIC, 5, &[(0, 1, 9)]), 0), Some(None));
        assert_eq!(
            find(inode(EXT4_EXT_MAGIC, 4, &[(0, 1, 9)]), 0).map(|e| e.is_some()),
            Some(true)
        );
    }

    /// The image's inode 14 (`ext`, a root index over one leaf) loaded into a detached inode.
    fn ext_inode() -> &'static Inode<TestGlue> {
        let ip = detached(&ufsmount(superblock()), 14);
        let disk = testffs::disk();
        let (blk, off) = newfs::Image::ioff(14);
        super::super::ext2fs_dinode::e2fs_iload(
            ip.i_e2fs(),
            &disk[blk * newfs::B + off..],
            ip.i_e2din(),
        );
        ip
    }

    #[test]
    fn a_two_level_tree_is_followed_through_its_index_block() {
        let (_g, _p) = attach(newfs::image(true));
        let ip = ext_inode();
        let fs = ip.i_e2fs();
        for (lbn, want) in [
            (0, e(0, 2, 27)),
            (1, e(0, 2, 27)),
            (2, e(2, 1, 30)),
            (9, e(2, 1, 30)),
        ] {
            let mut path = Ext4ExtentPath::new();
            let p = ext4_ext_find_extent(fs, ip, lbn, &mut path).map(|p| (p.ep_depth, p.ext()));
            assert_eq!(p, Some((0, Some(want))), "lbn {lbn}");
            let t = path.ep_bp.take();
            assert!(
                t.as_ref()
                    .is_some_and(|t| t.buf().blkno() == fsbtodb(fs, EXTLEAF as Daddr))
            );
            assert_eq!(path.index(), None, "the leaf level clears the index");
            if let Some(t) = t {
                brelse(t);
            }
        }
    }

    #[test]
    fn a_leaf_claiming_more_entries_than_its_block_holds_finds_nothing() {
        let mut img = newfs::image(true);
        // 85 entries: one more than a 1 KiB leaf holds after its header.
        img[EXTLEAF * newfs::B + 2..EXTLEAF * newfs::B + 4].copy_from_slice(&85u16.to_le_bytes());
        let (_g, _p) = attach(img);
        let ip = ext_inode();
        let mut path = Ext4ExtentPath::new();
        let p = ext4_ext_find_extent(ip.i_e2fs(), ip, 0, &mut path).map(|p| p.ext());
        assert_eq!(p, Some(None));
        if let Some(t) = path.ep_bp.take() {
            brelse(t);
        }
    }

    #[test]
    fn the_extent_cache_answers_inside_its_range_only() {
        let (_g, _p) = attach(newfs::image(true));
        let ip = inode(EXT4_EXT_MAGIC, 0, &[]);
        let mut ep = Ext4Extent::default();
        assert_eq!(ext4_ext_in_cache(ip, 0, &mut ep), EXT4_EXT_CACHE_NO);

        let cached = Ext4Extent {
            e_blk: 10,
            e_len: 2,
            e_start_hi: 1,
            e_start_lo: 5,
        };
        ext4_ext_put_cache(ip, &cached, EXT4_EXT_CACHE_IN);
        assert_eq!(ip.i_e2din().i_e2fs_ext_cache().ec_start, (1 << 32) | 5);
        assert_eq!(ext4_ext_in_cache(ip, 11, &mut ep), EXT4_EXT_CACHE_IN);
        assert_eq!(ep, cached);
        let mut other = Ext4Extent::default();
        assert_eq!(ext4_ext_in_cache(ip, 12, &mut other), EXT4_EXT_CACHE_NO);
        assert_eq!(ext4_ext_in_cache(ip, 9, &mut other), EXT4_EXT_CACHE_NO);
        assert_eq!(other, Ext4Extent::default());

        ext4_ext_put_cache(ip, &cached, EXT4_EXT_CACHE_GAP);
        assert_eq!(ext4_ext_in_cache(ip, 10, &mut ep), EXT4_EXT_CACHE_GAP);
    }
}
/* </TESTS> */
