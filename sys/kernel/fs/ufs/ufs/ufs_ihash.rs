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
//! The inode hash: the in-core inodes of every UFS mount, found by device and inode number
//! (`ufs_ihashget`), so that a file has one vnode however it is reached.
//!
//! Upstream: sys/ufs/ufs/ufs_ihash.c @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/ufs_ihash.rs@3c62ede68802
//!
//! The chains are picked by a SipHash of the pair under a random key, so that an attacker
//! cannot predict which inodes collide.
//!
//! ## Deviations
//! - `ihashtbl`, `ihash` (the table's size - 1) and `ihashkey` are one [`Ihash`] in the glue's
//!   VFS tables (`UfsTables`), made by `ufs_ihashinit` (from `ufs_init`, before any mount).
//! - `ufs_ihashrem`'s `i_hash.le_prev == NULL` test (an inode that was never hashed) is the
//!   `IN_HASHED` test that follows it: the flag is set exactly when the inode is linked. The
//!   `DIAGNOSTIC` clearing of the links is the list's own (an unlinked entry holds none).
//! - The `EXT2FS` branch of `ufs_ihashget` (`IS_EXT2_VNODE`: an ext2fs inode's link count is
//!   `i_e2fs_nlink`, an unsigned 16 bits, so `<= 0` is `== 0`) reads the count from the
//!   inode's glue member (`FileGlue::inode_e2fs_nlink`) while ext2fs is `bsd`'s (N5.6).
//! - `arc4random_buf(&ihashkey, ..)` is four `arc4random()` words of the same generator (the
//!   kernel reaches `rnd.c` a word at a time, `hal::random`).
//! - A chain is a `PinList`, 40 bytes (its head and the kernel lock's key) where the C's
//!   `LIST_HEAD` is 8, so the table `hashinit` makes in `M_UFSMNT` is about five times the C's
//!   and `vmstat -m`'s "UFS mount" MemUse grows by that much. No `diff-openbsd` scenario
//!   prints it; recorded here, not changed (a pool's or a malloc type's size follows its type,
//!   the standing rule).
//! - `hashinit` is written in `ufs_ihashinit`, as kernel::proc's and kernel::vm::aobj's are
//!   in theirs, where the C has one, kern_subr.c's; neither private one fits without a new
//!   shared function over the malloc type and the bucket (one kernel `hashinit` for the
//!   three is N12's list).
//!
//! ## Redesign
//! - LZ's chains of `sys/queue.rs` `ListHead`s in a `StaticCell`, linked and unlinked in
//!   `unsafe` blocks whose argument was the kernel lock (sys/ufs/ufs/ufs_ihash.rs:48-60,
//!   :205-226) -> `hal::pinlist::PinList`s of the pinned inodes (pool items, unlinked by
//!   `ufs_reclaim` before the last handle drops them), bound to the kernel lock: a walk takes
//!   `klock::reader()` (a per-CPU count, no interrupt level), an insertion or a removal
//!   `klock::witness()` (the C's `XXXLOCKING` made the kernel lock's witness). The witness
//!   raises the interrupt level for its section: one spl pair per `ufs_ihashins` and per
//!   `ufs_ihashrem` of a hashed inode (once in an inode's life each), where the C has none:
//!   docs/ROADMAP.md N12's row (D-N5.3-2's witness without the IPL raise removes it).
//! - The table in a `ReadMostly` cell (`UfsTables::ihash`), read by every operation (two
//!   atomics), so the host tests can start each test's file systems afresh (`ufs_test_reset`
//!   forgets it) where the C makes it once.
//! - `ufs_ihash(dev, inum)` (`INOHASH`) is [`Ihash::chain`], the table's own.
//! - `ufs_ihashget` finds the inode's vnode under the reader and drops it before `vget`
//!   (which may sleep), as the C's `XXXLOCKING unlock hash list?` places the unlock: nothing
//!   can change between the walk and `vget`'s first test without a sleep, which the kernel
//!   lock's holder has not made.

use core::pin::Pin;

use hal::klock;
use hal::malloc::{M_WAITOK, MArray};
use hal::pinlist::PinList;
use hal::random::arc4random;
use hal::sys::errno::Errno;
use hal::sys::mount::MNT_RDONLY;
use hal::sys::types::Dev;
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use super::dinode::Ufsino;
use super::inode::{IN_HASHED, Inode};
use super::ufsmount::M_UFSMNT;
use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::fs::subr::{vfs, vget, vput};
use crate::fs::vnode::{LK_EXCLUSIVE, VT_EXT2FS, Vnode};
use crate::fs::vops::{vop_lock, vop_unlock};
use crate::proc::ProcGlue;

/// The inode hash's adapter of the glue `G` (`i_hash`, `ufs_glue_links!`).
type Chain<G> = PinList<<<G as ProcGlue>::FileGlue as FileGlue>::InodeHash>;

/// `ihashtbl`, `ihash` and `ihashkey`: the hash chains, the table's size - 1, the key.
pub struct Ihash<G: ProcGlue> {
    tbl: MArray<Chain<G>>,
    mask: u64,
    key: SiphashKey,
}

impl<G: ProcGlue> Ihash<G> {
    /// `ufs_ihash(dev, inum)` (`INOHASH`): the chain of the pair.
    fn chain(&self, dev: Dev, inum: Ufsino) -> &Chain<G> {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&dev.to_ne_bytes());
        ctx.update(&inum.to_ne_bytes());
        &self.tbl[(ctx.end() & self.mask) as usize]
    }
}

/// The table, when `ufs_ihashinit` has made it: `panic(9)` before.
fn with_ihash<G: ProcGlue, R>(f: impl FnOnce(&Ihash<G>) -> R) -> R {
    let t = vfs::<G>().ufs.ihash.read();
    match t.as_ref() {
        Some(ih) => f(ih),
        None => kpanic(format_args!("ufs_ihash: no table")),
    }
}

/// `ufs_ihashinit`: initialize inode hash table: `hashinit(initialvnodes, M_UFSMNT,
/// M_WAITOK, &ihash)` and a random key.
pub fn ufs_ihashinit<G: ProcGlue>(initialvnodes: i32) {
    if initialvnodes <= 0 {
        kpanic(format_args!("hashinit: bad cnt"));
    }
    let n = (initialvnodes as usize).next_power_of_two();
    let Some(tbl) = MArray::new(M_UFSMNT, M_WAITOK, n, |_| PinList::new()) else {
        kpanic(format_args!("ufs_ihashinit: no memory"));
    };
    // arc4random_buf(&ihashkey, sizeof(ihashkey)): the key's 16 bytes from the generator,
    // a word at a time.
    let word = || u64::from(arc4random()) << 32 | u64::from(arc4random());
    let ih = Ihash {
        tbl,
        mask: n as u64 - 1,
        key: SiphashKey {
            k0: word(),
            k1: word(),
        },
    };
    let old = vfs::<G>().ufs.ihash.replace(Some(ih));
    if old.is_some() {
        kpanic(format_args!("ufs_ihashinit: twice"));
    }
}

/// `ufs_ihashget(dev, inum)`: use the device/inum pair to find the incore inode, and return
/// its vnode, referenced and locked. If it is in core, but locked, wait for it.
pub fn ufs_ihashget<G: ProcGlue>(dev: Dev, inum: Ufsino) -> Option<&'static Vnode<G>> {
    'retry: loop {
        // XXXLOCKING lock hash list
        let found = with_ihash::<G, _>(|ih| {
            let r = klock::reader();
            ih.chain(dev, inum)
                .iter(&r)
                .find(|ip| ip.i_number == inum && ip.i_dev == dev)
                .map(Inode::itov)
        });
        // XXXLOCKING unlock hash list?
        let vp = found?;
        if vget(vp, LK_EXCLUSIVE).is_err() {
            continue 'retry;
        }
        let ip = super::inode::vtoi(vp);
        // Check if the inode is valid. The condition has been adapted from ufs_inactive().
        let rdonly = vp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY != 0);
        // XXX DIP does not cover ext2fs so hack around this for now since this is using
        // ufs_ihashget as well.
        let unlinked = if vp.tag() == VT_EXT2FS {
            match <G::FileGlue as FileGlue>::inode_e2fs_nlink(&ip.g) {
                Some(n) => n <= 0,
                None => kpanic(format_args!("ufs_ihashget: an ext2fs inode of no ext2fs")),
            }
        } else {
            ip.dip_nlink() <= 0
        };
        drop(ip);
        if unlinked && !rdonly {
            // This should recycle the inode immediately, unless there are other threads
            // that try to access it. Pause to give the threads a chance to finish with the
            // inode.
            vput(vp);
            crate::sched::r#yield::<G>();
            continue 'retry;
        }

        return Some(vp);
    }
}

/// `ufs_ihashins(ip)`: insert the inode into the hash table, and return it locked. `EEXIST`
/// (with the inode unlocked) when the pair is hashed already.
pub fn ufs_ihashins<G: ProcGlue>(ip: Pin<&Inode<G>>) -> Result<(), Errno> {
    let dev = ip.i_dev;
    let inum = ip.i_number;

    // lock the inode, then put it on the appropriate hash list
    let _ = vop_lock(ip.itov(), LK_EXCLUSIVE);

    // XXXLOCKING lock hash list
    let hashed = with_ihash::<G, _>(|ih| {
        let mut w = klock::witness();
        let chain = ih.chain(dev, inum);
        if chain
            .iter(&w)
            .any(|curip| inum == curip.i_number && dev == curip.i_dev)
        {
            // XXXLOCKING unlock hash list?
            return false;
        }
        ip.set_flag(IN_HASHED);
        chain.insert_head(&mut w, ip);
        // XXXLOCKING unlock hash list?
        true
    });
    if !hashed {
        let _ = vop_unlock(ip.itov());
        return Err(Errno::EEXIST);
    }
    Ok(())
}

/// `ufs_ihashrem(ip)`: remove the inode from the hash table.
pub fn ufs_ihashrem<G: ProcGlue>(ip: &Inode<G>) {
    // XXXLOCKING lock hash list
    if ip.i_flag.get() & IN_HASHED != 0 {
        with_ihash::<G, _>(|ih| {
            let mut w = klock::witness();
            ih.chain(ip.i_dev, ip.i_number).remove(&mut w, ip);
        });
        ip.clr_flag(IN_HASHED);
    }
    // XXXLOCKING unlock hash list?
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use hal::intr::IPL_NONE;
    use hal::pool::{BoxPool, PR_WAITOK, PoolArc, TypedPool};

    use super::*;
    use crate::fs::subr::vput;
    use crate::fs::testfs::{TestGlue, setup};
    use crate::fs::ufs::ufs::dinode::Ufs1Dinode;
    use crate::fs::ufs::ufs::inode::{Dinode, InodeInit, InodeRef, Ufs1Din, vtoi};
    use crate::fs::ufs::ufs::ufsmount::UM_UFS1;
    use crate::fs::vnode::VREG;
    use crate::testutil::test_vnode;

    /// The tests' `ffs_ino_pool` and `ffs_dinode1_pool`.
    static INODES: TypedPool<Inode<TestGlue>> = TypedPool::new();
    static DINO1: BoxPool<Ufs1Din> = BoxPool::new();

    /// The test thread's VFS tables afresh, the pools (made once per process), the hash.
    fn hash_setup() -> std::sync::MutexGuard<'static, ()> {
        let g = setup();
        let _ = INODES.init_recorded(IPL_NONE, PR_WAITOK, "ffsino");
        let _ = DINO1.init(IPL_NONE, PR_WAITOK, "dino1pl");
        ufs_ihashinit::<TestGlue>(64);
        g
    }

    /// A vnode holding a new inode `(dev, ino)` with a UFS1 dinode of `nlink` links, as
    /// `ffs_vget` makes them (the vnode's node is one handle, the one returned another).
    fn inode(dev: Dev, ino: Ufsino, nlink: i16) -> (&'static Vnode<TestGlue>, InodeRef<TestGlue>) {
        let vp = test_vnode::<TestGlue>(VREG, 0);
        let ip = INODES
            .get_homed(PR_WAITOK, || {
                Inode::new(InodeInit {
                    vnode: Some(vp),
                    dev,
                    number: ino,
                    fstype: UM_UFS1,
                    ..InodeInit::default()
                })
            })
            .unwrap();
        let d = Ufs1Dinode {
            di_mode: 0o100644,
            di_nlink: nlink,
            ..Ufs1Dinode::default()
        };
        ip.set_din(Dinode::Ufs1(
            DINO1.get(PR_WAITOK, || Ufs1Din::new(&d)).unwrap(),
        ));
        vp.set_node(PoolArc::erase(ip.clone()));
        (vp, ip)
    }

    #[test]
    fn the_hash_finds_each_inode_by_device_and_number() {
        let _g = hash_setup();
        // 200 inodes on 64 chains: chains of several, walked to the right one.
        let made: Vec<_> = (0..200u32)
            .map(|i| inode(5 + (i % 3) as Dev, 100 + i, 1))
            .collect();
        for (_, ip) in &made {
            ufs_ihashins(ip.as_pin()).unwrap();
            assert_ne!(ip.i_flag.get() & IN_HASHED, 0);
        }
        for (i, (vp, _)) in made.iter().enumerate() {
            let dev = 5 + (i % 3) as Dev;
            let ino = 100 + i as Ufsino;
            let got = ufs_ihashget::<TestGlue>(dev, ino);
            assert!(got.is_some_and(|v| core::ptr::eq(v, *vp)), "inode {i}");
            assert_eq!(vp.usecount(), 2, "vget took a reference");
            vput(*vp);
            // The same number on another device is another file.
            assert!(ufs_ihashget::<TestGlue>(dev + 3, ino).is_none());
        }
        assert!(ufs_ihashget::<TestGlue>(5, 99).is_none());
        for (_, ip) in &made {
            ufs_ihashrem(ip);
        }
    }

    #[test]
    fn a_second_inode_of_a_pair_is_refused_and_a_removed_one_is_gone() {
        let _g = hash_setup();
        let (vp, ip) = inode(7, 2, 1);
        ufs_ihashins(ip.as_pin()).unwrap();
        // ffs_vget's race: another inode of the same pair is refused, and stays unhashed.
        let (_vp2, ip2) = inode(7, 2, 1);
        assert_eq!(ufs_ihashins(ip2.as_pin()), Err(Errno::EEXIST));
        assert_eq!(ip2.i_flag.get() & IN_HASHED, 0);
        // Never hashed: nothing to do (the C's `le_prev == NULL`).
        ufs_ihashrem(&ip2);
        assert!(ufs_ihashget::<TestGlue>(7, 2).is_some_and(|v| core::ptr::eq(v, vp)));
        vput(vp);
        // ufs_reclaim's removal: the pair is no longer found, the flag is clear, and the
        // inode can go once its handles do.
        ufs_ihashrem(&ip);
        assert_eq!(ip.i_flag.get() & IN_HASHED, 0);
        assert!(ufs_ihashget::<TestGlue>(7, 2).is_none());
        assert!(!ip.i_hash.is_linked());
        drop(vp.take_node());
        assert_eq!(PoolArc::count(&ip), 1, "the test's handle alone");
    }

    #[test]
    fn vtoi_counts_a_handle_out_of_the_node() {
        let _g = hash_setup();
        let (vp, ip) = inode(9, 3, 1);
        assert_eq!(PoolArc::count(&ip), 2);
        let ip2 = vtoi(vp);
        assert!(PoolArc::ptr_eq(&ip, &ip2));
        assert_eq!(PoolArc::count(&ip), 3);
        drop(ip2);
        assert_eq!(PoolArc::count(&ip), 2);
        assert!(core::ptr::eq(ip.itov(), vp));
        drop(vp.take_node());
        assert_eq!(PoolArc::count(&ip), 1);
    }

    #[test]
    #[should_panic(expected = "has no inode")]
    fn vtoi_of_a_vnode_without_an_inode_panics() {
        let _g = hash_setup();
        let vp = test_vnode::<TestGlue>(VREG, 0);
        let _ = vtoi(vp);
    }

    #[test]
    #[should_panic(expected = "has no inode")]
    fn vtoi_of_another_file_systems_node_panics() {
        static OTHER: TypedPool<u64> = TypedPool::new();
        let _g = hash_setup();
        let _ = OTHER.init(IPL_NONE, PR_WAITOK, "othernode");
        let vp = test_vnode::<TestGlue>(VREG, 0);
        vp.set_node(PoolArc::erase(OTHER.get_shared(PR_WAITOK, || 7).unwrap()));
        let _ = vtoi(vp);
    }
}
/* </TESTS> */
