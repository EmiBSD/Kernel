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
 * Copyright (c) 1982, 1986, 1989, 1991, 1993
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
 *	@(#)ufs_ihash.c	8.4 (Berkeley) 12/30/93
 */
/* </LICENSES> */

/* <CODE> */
//! The FUSE inode hash: the in-core nodes of every FUSE mount, found by mount and inode
//! number (`fuse_ihashget`), so that a file has one vnode however it is reached.
//!
//! Upstream: sys/miscfs/fuse/fuse_ihash.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fuse_ihash.rs@3c62ede68802
//!
//! The chains are picked by a SipHash of the pair under a random key. The mount is hashed
//! by its address: mounts are unique for all mounted file systems.
//!
//! ## Deviations
//! - `fuse_ihashget` returns the vnode as an `Option` (the C's NULL is `None`) (LZ's).
//!
//! ## Redesign
//! - LZ's `fuse_ihashtbl`, `fuse_ihashsz` and `fuse_ihashkey` were three `StaticCell`s written
//!   by `fuse_ihashinit` and read through `unsafe` (4 blocks, a `Sync`/`Send` claim on the
//!   table): the three are one [`Fuseihash`] in `FuseTables::ihash`, a `ReadMostly` cell
//!   `fuse_ihashinit` fills once, its chains a `hashinit` `MArray` of `PinList`s (msdosfs's
//!   denode hash shape, D-N5.7-4: the kernel's shared `fs::subr::hashinit` and
//!   `siphash_key`, whose four `arc4random()` words are the C's `arc4random_buf` of the key).
//! - The C's "XXXLOCKING" is the kernel lock, which every caller holds: a walk takes
//!   `klock::reader()` (a per-CPU count, no interrupt level) and drops it before `vget`, which
//!   may sleep, rechecking `v_id` as the C does; an insertion or a removal takes
//!   `klock::witness()`, which raises the interrupt level for its section: one spl pair per
//!   `fuse_ihashins` and per `fuse_ihashrem` of a hashed node (once in a node's life each),
//!   where the C has none: docs/ROADMAP.md N12's row, as `ufs_ihash.rs`'s and msdosfs's.
//! - A node is compared with the mount by the handle it holds (the same block, `MArc`'s
//!   address), and the hash reads that block's address, the C's `fmp` pointer.

use core::pin::Pin;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::malloc::{M_WAITOK, MArray};
use hal::pinlist::PinList;
use hal::sys::errno::Errno;
use hal::sys::types::Ino;
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use crate::assert::kpanic;
use crate::fs::fuse::fusefs::{FusefsMnt, M_FUSEFS};
use crate::fs::fuse::fusefs_node::{FusefsIHash, FusefsNode, itov};
use crate::fs::subr::{hashinit, siphash_key, vfs, vget, vput};
use crate::fs::vnode::{LK_EXCLUSIVE, Vnode};
use crate::fs::vops::{vop_lock, vop_unlock};
use crate::proc::ProcGlue;

/// A chain of the inode hash.
type Chain<G> = PinList<FusefsIHash<G>>;

/// `fuse_ihashtbl`, `fuse_ihashsz` and `fuse_ihashkey`: the hash chains, the table's size -
/// 1, the key.
pub struct Fuseihash<G: ProcGlue> {
    tbl: MArray<Chain<G>>,
    mask: u64,
    key: SiphashKey,
}

impl<G: ProcGlue> Fuseihash<G> {
    /// `fuse_ihash(fmp, inum)`: create a hash for the inode using the mounted file system
    /// id, the index of its chain. These are guaranteed to be unique for all mounted file
    /// systems by `vfs_getnewfsid`.
    pub fn fuse_ihash(&self, fmp: &FusefsMnt<G>, inum: Ino) -> u32 {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&ptr::from_ref(fmp).addr().to_ne_bytes());
        ctx.update(&inum.to_ne_bytes());
        // `& fuse_ihashsz`, the table's size - 1, a power of two below 2^31 (`hashinit` of an
        // `int`): the index fits a `u32`.
        (ctx.end() & self.mask) as u32
    }

    /// `&fuse_ihashtbl[i]`.
    fn chain(&self, i: u32) -> &Chain<G> {
        &self.tbl[i as usize]
    }
}

/// The table, when `fuse_ihashinit` has made it: `panic(9)` before.
fn with_ihash<G: ProcGlue, R>(f: impl FnOnce(&Fuseihash<G>) -> R) -> R {
    let t = vfs::<G>().fuse.ihash.read();
    match t.as_ref() {
        Some(h) => f(h),
        None => kpanic(format_args!("fuse_ihash: no table")),
    }
}

/// Whether `ip` is the node of inode `inum` of `fmp`.
fn is<G: ProcGlue>(ip: &FusefsNode<G>, fmp: &FusefsMnt<G>, inum: Ino) -> bool {
    inum == ip.i_number && ptr::eq(ptr::from_ref(&*ip.i_fmp), ptr::from_ref(fmp))
}

/// `fuse_ihashinit`: initialize inode hash table, `hashinit(initialvnodes, M_FUSEFS,
/// M_WAITOK, &fuse_ihashsz)`, and a random key.
pub fn fuse_ihashinit<G: ProcGlue>() {
    let initialvnodes = vfs::<G>().initialvnodes.load(Ordering::Relaxed);
    let Some((tbl, mask)) = hashinit::<FusefsIHash<G>>(initialvnodes, M_FUSEFS, M_WAITOK) else {
        kpanic(format_args!("fuse_ihashinit: no memory"));
    };
    let h = Fuseihash {
        tbl,
        mask,
        key: siphash_key(),
    };
    if vfs::<G>().fuse.ihash.replace(Some(h)).is_some() {
        kpanic(format_args!("fuse_ihashinit: twice"));
    }
}

/// `fuse_ihashget(fmp, inum)`: use the fsid/inum pair to find the incore inode, and return
/// its vnode, referenced and locked. If it is in core, but locked, wait for it.
pub fn fuse_ihashget<G: ProcGlue>(fmp: &FusefsMnt<G>, inum: Ino) -> Option<&'static Vnode<G>> {
    loop {
        // XXXLOCKING lock hash list
        let found = with_ihash::<G, _>(|h| {
            let r = klock::reader();
            h.chain(h.fuse_ihash(fmp, inum))
                .iter(&r)
                .find(|ip| is(ip, fmp, inum))
                .map(|ip| {
                    let vp = itov(ip);
                    (vp, vp.id())
                })
        });
        // XXXLOCKING unlock hash list?
        let (vp, vpid) = found?;
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

/// `fuse_ihashins(ip)`: insert the inode into the hash table, and return it locked.
/// `EEXIST` (with the inode unlocked) when the pair is hashed already.
pub fn fuse_ihashins<G: ProcGlue>(ip: Pin<&FusefsNode<G>>) -> Result<(), Errno> {
    let fmp = &*ip.i_fmp;
    let inum = ip.i_number;

    // lock the inode, then put it on the appropriate hash list
    let _ = vop_lock(itov(&ip), LK_EXCLUSIVE);

    // XXXLOCKING lock hash list
    let inserted = with_ihash::<G, _>(|h| {
        let mut w = klock::witness();
        let i = h.fuse_ihash(fmp, inum);
        let chain = h.chain(i);
        if chain.iter(&w).any(|curip| is(curip, fmp, inum)) {
            return false;
        }
        ip.i_chain.set(i);
        chain.insert_head(&mut w, ip);
        true
    });
    // XXXLOCKING unlock hash list?

    if !inserted {
        let _ = vop_unlock(itov(&ip));
        return Err(Errno::EEXIST);
    }
    Ok(())
}

/// `fuse_ihashrem(ip)`: remove the inode from the hash table; nothing for a node that was
/// never hashed (the C's `i_hash.le_prev == NULL`).
pub fn fuse_ihashrem<G: ProcGlue>(ip: &FusefsNode<G>) {
    // XXXLOCKING lock hash list
    with_ihash::<G, _>(|h| {
        let mut w = klock::witness();
        if ip.i_hash.is_linked() {
            h.chain(ip.i_chain.get()).remove(&mut w, ip);
        }
    });
    // XXXLOCKING unlock hash list?
}

/// `fuse_test_reset`'s part: forgets the inode hash of a test thread's tables without
/// freeing it (the memory behind it is gone with the last test's, testing.md).
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn fuse_ihash_test_reset<G: ProcGlue>() {
    core::mem::forget(vfs::<G>().fuse.ihash.replace(None));
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the inode hash (LZ's moved): nodes found by mount and inode number, a
    // second insertion of the pair refused, removal, a node never hashed; the hash of a
    // fixed key is stable and picks a chain inside the table.

    use std::assert;

    use super::*;
    use crate::fs::fuse::testfuse::{fake_fmp, node, setup};
    use crate::fs::testfs::TestGlue;

    #[test]
    fn nodes_are_found_by_mount_and_inode() {
        let (_g, _p) = setup();
        let (fmp_a, fmp_b) = (fake_fmp(1, 4096), fake_fmp(2, 4096));
        let a = node(&fmp_a, 7);
        fuse_ihashins(a.as_pin()).expect("insert");
        let _ = vop_unlock(itov(&a));
        let dup = node(&fmp_a, 7);
        assert!(fuse_ihashins(dup.as_pin()) == Err(Errno::EEXIST));
        assert!(!dup.i_hash.is_linked(), "the refused node is not hashed");
        assert!(dup.i_lock.status() == 0, "and is unlocked");
        let b = node(&fmp_b, 7);
        fuse_ihashins(b.as_pin()).expect("the same inode of another mount");
        let _ = vop_unlock(itov(&b));

        let vp = fuse_ihashget(&fmp_a, 7).expect("hashed");
        assert!(ptr::eq(vp, itov(&a)));
        vput(vp);
        assert!(fuse_ihashget(&fmp_a, 8).is_none());

        fuse_ihashrem(&a);
        assert!(!a.i_hash.is_linked());
        assert!(fuse_ihashget(&fmp_a, 7).is_none());
        fuse_ihashrem(&a); // not hashed: nothing to do
        assert!(fuse_ihashget(&fmp_b, 7).is_some_and(|vp| {
            vput(vp);
            ptr::eq(vp, itov(&b))
        }));
        fuse_ihashrem(&b);
    }

    #[test]
    fn the_hash_picks_a_chain_of_the_table() {
        let (_g, _p) = setup();
        let fmp = fake_fmp(1, 4096);
        with_ihash::<TestGlue, _>(|h| {
            for inum in 0..64 {
                let i = h.fuse_ihash(&fmp, inum);
                assert!(u64::from(i) <= h.mask);
                assert!(
                    i == h.fuse_ihash(&fmp, inum),
                    "the same pair, the same chain"
                );
            }
        });
    }
}
/* </TESTS> */
