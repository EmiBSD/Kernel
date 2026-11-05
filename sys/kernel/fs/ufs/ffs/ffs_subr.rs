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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)ffs_subr.c	8.2 (Berkeley) 9/21/93
 */
/* </LICENSES> */

/* <CODE> */
//! Fast file system helpers: the directory block of an offset (`ffs_bufatoff`), and the
//! fragment and block map operations the allocator and `fsck` share (`ffs_fragacct`,
//! `ffs_isblock`, `ffs_clrblock`, `ffs_setblock`, `ffs_isfreeblock`).
//!
//! Upstream: sys/ufs/ffs/ffs_subr.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_subr.rs@3c62ede68802
//!
//! ## Deviations
//! - The userland prototypes (`!_KERNEL`) are left out.
//! - The maps are byte slices (`u_char *cp` in C); `ffs_fragacct`'s `fraglist` is the
//!   cylinder group's `cg_frsum`, reached through its [`CgView`].
//! - `ffs_bufatoff` returns the buffer and the offset of `*res` in its data.
//!
//! ## Redesign
//! - N5.5b1: LZ's functions over `&Fs` (a `Cell` structure) and `Inode` -> over the in-core
//!   [`Superblock`]'s accessors and the kernel's `Inode<G>`; `ffs_bufatoff` returns the busy
//!   buffer's token (`bread` of the kernel's buffer cache), as LZ's did since N5.5a2.
//!   `ffs_fragacct` adds to `cg_frsum` through the cylinder group's view, the C's `u_int32_t`
//!   arithmetic (LZ's `&[Cell<i32>]`).
//! - N5.5b2: `ffs_vinit` over the kernel's vnode: `v_op = &ffs_specvops` is the vnode's id of
//!   the glue's `UfsTables::ffs_specvops` (`vops_id`), and the alias takes the inode's node from the
//!   discarded vnode's slot (`take_node`/`set_node`: the inode's handle moves, its lock with
//!   it, as the C moves `v_data`).

use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Dev, Off};

use super::fs::{AROUND, CgView, FRAGTBL, INSIDE, Superblock, blkoff, blksize, lblkno};
use crate::assert::kpanic;
use crate::fs::bio::{bread, brelse, buf_adjcnt};
use crate::fs::buf::BufRef;
use crate::fs::mount::Mount;
use crate::fs::subr::{checkalias, vfs, vgone, vops_id, vrele};
use crate::fs::ufs::ufs::dinode::ROOTINO;
use crate::fs::ufs::ufs::inode::{Inode, vtoi};
use crate::fs::vnode::{
    VBAD, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREG, VROOT, VSOCK, Vnode, VopsId, iftovt,
};
use crate::proc::ProcGlue;
use crate::tc::getmicrouptime;

/// `ffs_bufatoff` (`iv_bufatoff`): the busy buffer with the contents of block `offset` from
/// the beginning of directory `ip`, and the offset in it of the remaining space in the
/// directory (`*res`).
pub fn ffs_bufatoff<G: ProcGlue>(ip: &Inode<G>, offset: Off) -> Result<(BufRef, usize), Errno> {
    let vp = ip.itov();
    let fs = ip.i_fs();
    let lbn = lblkno(fs, offset);
    let bsize = blksize(fs, ip, lbn);

    let (t, error) = bread(&vp.bufs, lbn, fs.fs_bsize());
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }
    buf_adjcnt(&t, bsize as i64);
    Ok((t, blkoff(fs, offset) as usize))
}

/// `ffs_fragacct`: update the frsum fields of the cylinder group `cgp` to reflect addition
/// or deletion of some frags.
pub fn ffs_fragacct(fs: &Superblock, fragmap: i32, cgp: &mut CgView<'_>, cnt: i32) {
    let frag = fs.fs_frag();
    let Some(tbl) = usize::try_from(frag)
        .ok()
        .and_then(|f| FRAGTBL.get(f).copied().flatten())
    else {
        kpanic(format_args!("ffs_fragacct: fs_frag {frag}"));
    };
    let inblk = i32::from(tbl[fragmap as u8 as usize]) << 1;
    let fragmap = fragmap << 1;
    for siz in 1..frag {
        if inblk & (1 << (siz + (frag % 8))) == 0 {
            continue;
        }
        let mut field = AROUND[siz as usize];
        let mut subfield = INSIDE[siz as usize];
        let mut pos = siz;
        while pos <= frag {
            if fragmap & field == subfield {
                cgp.add_cg_frsum(siz as usize, cnt);
                pos += siz;
                field <<= siz;
                subfield <<= siz;
            }
            field <<= 1;
            subfield <<= 1;
            pos += 1;
        }
    }
}

/// `ffs_isblock`: block operations: check if a block is available.
pub fn ffs_isblock(fs: &Superblock, cp: &[u8], h: Daddr) -> bool {
    let h = h as usize;
    match fs.fs_frag() {
        4 => {
            let mask = 0x0f << ((h & 0x1) << 2);
            cp[h >> 1] & mask == mask
        }
        2 => {
            let mask = 0x03 << ((h & 0x3) << 1);
            cp[h >> 2] & mask == mask
        }
        1 => {
            let mask = 0x01 << (h & 0x7);
            cp[h >> 3] & mask == mask
        }
        _ => cp[h] == 0xff,
    }
}

/// `ffs_clrblock`: take a block out of the map.
pub fn ffs_clrblock(fs: &Superblock, cp: &mut [u8], h: Daddr) {
    let h = h as usize;
    match fs.fs_frag() {
        4 => cp[h >> 1] &= !(0x0f << ((h & 0x1) << 2)),
        2 => cp[h >> 2] &= !(0x03 << ((h & 0x3) << 1)),
        1 => cp[h >> 3] &= !(0x01 << (h & 0x7)),
        _ => cp[h] = 0,
    }
}

/// `ffs_setblock`: put a block into the map.
pub fn ffs_setblock(fs: &Superblock, cp: &mut [u8], h: Daddr) {
    let h = h as usize;
    match fs.fs_frag() {
        4 => cp[h >> 1] |= 0x0f << ((h & 0x1) << 2),
        2 => cp[h >> 2] |= 0x03 << ((h & 0x3) << 1),
        1 => cp[h >> 3] |= 0x01 << (h & 0x7),
        _ => cp[h] = 0xff,
    }
}

/// `ffs_isfreeblock`: check if a block is free.
pub fn ffs_isfreeblock(fs: &Superblock, cp: &[u8], h: Daddr) -> bool {
    let h = h as usize;
    match fs.fs_frag() {
        4 => cp[h >> 1] & (0x0f << ((h & 0x1) << 2)) == 0,
        2 => cp[h >> 2] & (0x03 << ((h & 0x3) << 1)) == 0,
        1 => cp[h >> 3] & (0x01 << (h & 0x7)) == 0,
        _ => cp[h] == 0,
    }
}

/// `ffs_vinit`: initialize the vnode associated with a new inode, handle aliased vnodes.
/// Returns the vnode to use, which replaces `vp` when a device vnode had an alias.
pub fn ffs_vinit<G: ProcGlue>(
    mntp: &'static Mount<G>,
    vp: &'static Vnode<G>,
) -> Result<&'static Vnode<G>, Errno> {
    let mut vp = vp;
    let ip = vtoi(vp);
    vp.set_vtype(iftovt(ip.dip_mode()));
    match vp.vtype() {
        VCHR | VBLK => {
            vp.set_ops_id(vops_id::<G>(&vfs::<G>().ufs.ffs_specvops));
            if let Some(nvp) = checkalias(vp, ip.dip_rdev() as Dev, Some(mntp)) {
                // Discard unneeded vnode, but save its inode. Note that the lock is carried
                // over in the inode to the replacement vnode.
                if let Some(node) = vp.take_node() {
                    nvp.set_node(node);
                }
                vp.set_ops_id(VopsId::SPEC);
                vrele(vp);
                vgone(vp);
                // Reinitialize aliased inode.
                vp = nvp;
                ip.i_vnode.set(Some(vp));
            }
        }
        // FIFO: vp->v_op = &ffs_fifovops (miscfs/fifofs, not ported); without FIFO:
        VFIFO => return Err(Errno::EOPNOTSUPP),
        VNON | VBAD | VSOCK | VLNK | VDIR | VREG => {}
    }
    if ip.i_number == ROOTINO {
        vp.set_flag(vp.flag() | VROOT);
    }
    // Initialize modrev times.
    let mtv = getmicrouptime();
    let modrev = ((mtv.tv_sec as u64) << 32) | (mtv.tv_usec as u64).wrapping_mul(4294);
    ip.i_modrev.set(modrev);
    Ok(vp)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec;
    use std::{assert, assert_eq};

    use hal::user::pod_to_bytes;

    use super::super::fs::{CG_MAGIC, Cg, Fs, MAXFRAG};
    use super::*;
    use crate::testutil;

    /// An in-core super-block with only `fs_frag` (and what `Superblock::new` reads) set:
    /// the map operations read nothing else.
    fn fs_of(frag: i32) -> Superblock {
        let mut f = Fs::zeroed();
        f.fs_frag = frag;
        f.fs_fsize = 1024;
        let mut b = vec![0u8; size_of::<Fs>()];
        assert!(pod_to_bytes(&f, &mut b));
        Superblock::new(&b)
    }

    #[test]
    fn the_block_maps_agree_with_each_other() {
        testutil::install();
        for frag in [1, 2, 4, 8] {
            let fs = fs_of(frag);
            let mut map = [0u8; 4];
            for h in 0..(32 / frag) as i64 {
                assert!(ffs_isfreeblock(&fs, &map, h));
                ffs_setblock(&fs, &mut map, h);
                assert!(ffs_isblock(&fs, &map, h));
                assert!(!ffs_isfreeblock(&fs, &map, h));
                ffs_clrblock(&fs, &mut map, h);
                assert!(ffs_isfreeblock(&fs, &map, h));
            }
        }
        // A block of four fragments half free is neither whole nor free.
        let fs = fs_of(4);
        let map = [0b0000_0011u8];
        assert!(!ffs_isblock(&fs, &map, 0) && !ffs_isfreeblock(&fs, &map, 0));
        assert!(ffs_isfreeblock(&fs, &map, 1));
    }

    /// `cg_frsum` after `ffs_fragacct(fs, map, cgp, 1)` from zero.
    fn frsum(fs: &Superblock, map: i32) -> [u32; MAXFRAG] {
        let mut b = vec![0u8; size_of::<Cg>()];
        assert!(pod_to_bytes(
            &Cg {
                cg_magic: CG_MAGIC,
                ..Cg::default()
            },
            &mut b
        ));
        let mut cgp = CgView::new(&mut b);
        ffs_fragacct(fs, map, &mut cgp, 1);
        core::array::from_fn(|i| cgp.cg_frsum(i))
    }

    #[test]
    fn fragacct_counts_each_maximal_run_of_free_fragments() {
        testutil::install();
        let fs = fs_of(8);
        // The C's vectors: runs of 1 and 3 (0b0111_0001); one run of 7; none in a whole
        // free block (a block, not fragments) nor in a full one.
        assert_eq!(frsum(&fs, 0b0111_0001), [0, 1, 0, 1, 0, 0, 0, 0]);
        assert_eq!(frsum(&fs, 0b0111_1111), [0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(frsum(&fs, 0b1010_1010), [0, 4, 0, 0, 0, 0, 0, 0]);
        assert_eq!(frsum(&fs, 0xff), [0; MAXFRAG]);
        assert_eq!(frsum(&fs, 0), [0; MAXFRAG]);
        // Four-fragment blocks: the low block's run of 2 (the map's low nibble).
        let fs4 = fs_of(4);
        assert_eq!(frsum(&fs4, 0b0110), [0, 0, 1, 0, 0, 0, 0, 0]);
        // A decrement wraps as the C's u_int32_t does.
        let mut b = vec![0u8; size_of::<Cg>()];
        assert!(pod_to_bytes(
            &Cg {
                cg_magic: CG_MAGIC,
                ..Cg::default()
            },
            &mut b
        ));
        let mut cgp = CgView::new(&mut b);
        ffs_fragacct(&fs, 0b0000_0001, &mut cgp, -1);
        assert_eq!(cgp.cg_frsum(1), u32::MAX);
    }
}
/* </TESTS> */
