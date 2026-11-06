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
 * Copyright (c) 2010, 2011 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Mindaugas Rasiukevicius.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! tmpfs memory allocation routines. Implements memory usage accounting and limiting: the
//! per-mount byte counter against its limit (or against the global `tmpfs_bytes_limit` when
//! the mount has none), the node and directory entry pools and the name buffers.
//!
//! Upstream: sys/tmpfs/tmpfs_mem.c @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs_mem.rs@3c62ede68802
//!
//! ## Deviations
//! - `tmpfs_bytes_limit` and `tmpfs_bytes_used` are atomics of the glue's tables
//!   ([`TmpfsTables`]): the C updates them under whichever mount's `tm_acc_lock` is held.
//! - `tmpfs_mem_incr` returns `bool` (the C's 1 or 0).
//! - `tmpfs_strname_neqlen` compares the two names themselves after their rounded lengths;
//!   the C compares the rounded length's bytes of each path buffer, so it also calls equal
//!   names unequal when what follows them in the paths differs, and may read past the end
//!   of the buffer. Its one caller (`tmpfs_rename`) only uses the answer to decide whether
//!   to allocate a new name buffer (LZ's, kept).
//!
//! ## Redesign
//! - The accounting charges the C's `sizeof(tmpfs_node_t)` and `sizeof(tmpfs_dirent_t)`
//!   ([`TMPFS_NODE_CHARGE`] 272, [`TMPFS_DIRENT_CHARGE`] 48, LP64; D-N5.9-6), so the bytes a
//!   mount uses and `statfs`'s counts are OpenBSD's; LZ charged `size_of` its Rust structs.
//! - `tmpfs_mntmem_init` is [`TmpfsMount::new`]'s fill (the limit, nothing used, the lock),
//!   since the structure is built whole where the C mallocs it (`tmpfs_vfsops.rs`); the
//!   accounting is [`TmAcc`], owned by `tm_acc_lock` ([`TmpfsMount::tm_acc`]), so
//!   [`tmpfs_pages_avail`]'s "the caller holds `tm_acc_lock`" is its argument's type.
//! - `tmpfs_dirent_get`/`tmpfs_node_get` take the item with the value its caller makes
//!   (`get_homed`, the recorded pools of `tmpfs_vfsops.rs`) and hand back its handle; the
//!   `_put`s take that handle (the C's `pool_put`: the item goes with its last holder). LZ
//!   wrote a fresh value into a raw item in two `unsafe` blocks.
//! - The name buffers are `MArray<u8>`s of the C's size and type (`roundup2(len, 32)` bytes of
//!   `M_TEMP`), freed by their drop in [`tmpfs_strname_free`].

use core::sync::atomic::Ordering;

use hal::arch::current::param::PAGE_SHIFT;
use hal::malloc::{M_WAITOK, MArray};
use hal::pool::PR_WAITOK;

use crate::assert::{kassert, kpanic};
use crate::fs::namei::Componentname;
use crate::fs::subr::vfs;
use crate::fs::tmpfs::tmpfs::{
    TMPFS_DIRENT_CHARGE, TMPFS_NODE_CHARGE, TmAcc, TmpfsDirent, TmpfsDirentRef, TmpfsMount,
    TmpfsNode, TmpfsNodeRef,
};
use crate::proc::ProcGlue;

/// `M_TEMP`: the malloc type of the names and the symbolic links' targets.
pub(super) const M_TEMP: i32 = 127;

/// `TMPFS_NAME_QUANTUM`: quantum size to round-up the tmpfs names in order to reduce
/// re-allocations.
const TMPFS_NAME_QUANTUM: usize = 32;

/// `tmpfs_mntmem_destroy`: end the accounting of a mount; everything was given back.
pub fn tmpfs_mntmem_destroy<G: ProcGlue>(mp: &TmpfsMount<G>) {
    kassert!(mp.tm_acc.read().tm_bytes_used == 0);
    // mutex_destroy(&mp->tm_acc_lock): nothing to do for an rwlock.
}

/// `tmpfs_pages_total`: the size of the file system in pages, from the accounting `acc` of
/// its mount (read under `tm_acc_lock`, `tmpfs_statfs`'s).
pub fn tmpfs_pages_total<G: ProcGlue>(acc: &TmAcc) -> u64 {
    let t = &vfs::<G>().tmpfs;
    let total = if acc.tm_mem_limit != 0 {
        acc.tm_mem_limit
    } else {
        t.bytes_limit
            .load(Ordering::Relaxed)
            .wrapping_sub(t.bytes_used.load(Ordering::Relaxed))
            .wrapping_add(acc.tm_bytes_used)
    };

    total >> PAGE_SHIFT
}

/// `tmpfs_pages_avail`: the pages the file system may still use; `acc` is its mount's
/// accounting, borrowed from `tm_acc_lock` (the C's `rw_assert_anylock`).
pub fn tmpfs_pages_avail<G: ProcGlue>(acc: &TmAcc) -> u64 {
    let t = &vfs::<G>().tmpfs;
    let free = if acc.tm_mem_limit != 0 {
        acc.tm_mem_limit.wrapping_sub(acc.tm_bytes_used)
    } else {
        t.bytes_limit
            .load(Ordering::Relaxed)
            .wrapping_sub(t.bytes_used.load(Ordering::Relaxed))
    };

    free >> PAGE_SHIFT
}

/// `tmpfs_mem_incr`: account `sz` more bytes to the mount; `false` when that would pass
/// its limit.
pub fn tmpfs_mem_incr<G: ProcGlue>(mp: &TmpfsMount<G>, sz: usize) -> bool {
    let sz = sz as u64;
    let t = &vfs::<G>().tmpfs;
    let mut acc = mp.tm_acc.write();

    if acc.tm_mem_limit != 0 {
        if acc.tm_mem_limit.wrapping_sub(acc.tm_bytes_used) < sz {
            return false;
        }
    } else {
        let limit = t.bytes_limit.load(Ordering::Relaxed);
        if limit.wrapping_sub(t.bytes_used.load(Ordering::Relaxed)) < sz {
            return false;
        }
        t.bytes_used.fetch_add(sz, Ordering::Relaxed);
    }

    acc.tm_bytes_used += sz;
    true
}

/// `tmpfs_mem_decr`: give `sz` bytes back.
pub fn tmpfs_mem_decr<G: ProcGlue>(mp: &TmpfsMount<G>, sz: usize) {
    let sz = sz as u64;
    let mut acc = mp.tm_acc.write();
    kassert!(acc.tm_bytes_used >= sz);
    if acc.tm_mem_limit == 0 {
        vfs::<G>().tmpfs.bytes_used.fetch_sub(sz, Ordering::Relaxed);
    }
    acc.tm_bytes_used -= sz;
}

/// `tmpfs_dirent_get`: a new directory entry (`pool_get(PR_ZERO | PR_WAITOK)`), accounted to
/// the mount; `None` when the mount is full.
pub fn tmpfs_dirent_get<G: ProcGlue>(mp: &TmpfsMount<G>) -> Option<TmpfsDirentRef<G>> {
    if !tmpfs_mem_incr(mp, TMPFS_DIRENT_CHARGE) {
        return None;
    }
    match vfs::<G>()
        .tmpfs
        .dirent_pool
        .get_homed(PR_WAITOK, TmpfsDirent::new)
    {
        Some(de) => Some(de),
        None => kpanic(format_args!("tmpfs_dirent_get: pool_get failed")),
    }
}

/// `tmpfs_dirent_put`: give a directory entry back (`pool_put`: the item goes with its last
/// handle).
pub fn tmpfs_dirent_put<G: ProcGlue>(mp: &TmpfsMount<G>, de: TmpfsDirentRef<G>) {
    tmpfs_mem_decr(mp, TMPFS_DIRENT_CHARGE);
    drop(de);
}

/// `tmpfs_node_get`: a new node holding `init()` (`pool_get(PR_WAITOK)`), counted and
/// accounted to the mount; `None` when the mount has its maximum number of nodes or is full
/// (`init` is not called then).
pub fn tmpfs_node_get<G: ProcGlue>(
    mp: &TmpfsMount<G>,
    init: impl FnOnce() -> TmpfsNode<G>,
) -> Option<TmpfsNodeRef<G>> {
    mp.tm_nodes_cnt.set(mp.tm_nodes_cnt.get() + 1);
    if mp.tm_nodes_cnt.get() > mp.tm_nodes_max {
        mp.tm_nodes_cnt.set(mp.tm_nodes_cnt.get() - 1);
        return None;
    }
    if !tmpfs_mem_incr(mp, TMPFS_NODE_CHARGE) {
        mp.tm_nodes_cnt.set(mp.tm_nodes_cnt.get() - 1);
        return None;
    }
    match vfs::<G>().tmpfs.node_pool.get_homed(PR_WAITOK, init) {
        Some(node) => Some(node),
        None => kpanic(format_args!("tmpfs_node_get: pool_get failed")),
    }
}

/// `tmpfs_node_put`: give a node back (`pool_put`: the item goes with its last handle).
pub fn tmpfs_node_put<G: ProcGlue>(mp: &TmpfsMount<G>, tn: TmpfsNodeRef<G>) {
    mp.tm_nodes_cnt.set(mp.tm_nodes_cnt.get() - 1);
    tmpfs_mem_decr(mp, TMPFS_NODE_CHARGE);
    drop(tn);
}

/// `roundup2(x, y)`: if y is powers of two.
const fn roundup2(x: usize, y: usize) -> usize {
    (x + (y - 1)) & !(y - 1)
}

/// `tmpfs_strname_alloc`: a buffer for a name of `len` bytes (`malloc(sz, M_TEMP,
/// M_WAITOK)` of the rounded size), accounted to the mount in `TMPFS_NAME_QUANTUM` steps;
/// `None` when the mount is full.
pub fn tmpfs_strname_alloc<G: ProcGlue>(mp: &TmpfsMount<G>, len: usize) -> Option<MArray<u8>> {
    let sz = roundup2(len, TMPFS_NAME_QUANTUM);

    kassert!(sz > 0 && sz <= 1024);
    if !tmpfs_mem_incr(mp, sz) {
        return None;
    }
    match MArray::new(M_TEMP, M_WAITOK, sz, |_| 0u8) {
        // XXX
        Some(b) => Some(b),
        None => kpanic(format_args!("tmpfs_strname_alloc: malloc failed")),
    }
}

/// `tmpfs_strname_free`: give back the buffer `str` of a name of `len` bytes (`None`: a
/// symbolic link's target, whose block goes with its node's item, `tmpfs_free_node`'s last
/// handle; only its charge is given back here).
pub fn tmpfs_strname_free<G: ProcGlue>(mp: &TmpfsMount<G>, str: Option<MArray<u8>>, len: usize) {
    let sz = roundup2(len, TMPFS_NAME_QUANTUM);

    kassert!(sz > 0 && sz <= 1024);
    tmpfs_mem_decr(mp, sz);
    drop(str); // free(str, M_TEMP, sz)
}

/// `tmpfs_strname_neqlen`: whether the two names differ (in their rounded lengths or in
/// their bytes; see the module's deviations), that is, whether a rename needs a new name
/// buffer.
pub fn tmpfs_strname_neqlen<G: ProcGlue>(
    fcnp: &Componentname<'_, G>,
    tcnp: &Componentname<'_, G>,
) -> bool {
    let fln = roundup2(fcnp.name().len(), TMPFS_NAME_QUANTUM);
    let tln = roundup2(tcnp.name().len(), TMPFS_NAME_QUANTUM);

    fln != tln || fcnp.name() != tcnp.name()
}

/// [`tmpfs_strname_neqlen`]'s rounding, for the tests.
#[cfg(test)]
pub(super) const fn name_quantum(len: usize) -> usize {
    roundup2(len, TMPFS_NAME_QUANTUM)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the tmpfs memory accounting: the per-mount limit, the global limit of the
    // mounts without one, the name buffers' quantum, the node/entry pools and the C's charges
    // (LZ's, moved; the charges D-N5.9-6's).

    use std::sync::MutexGuard;

    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::fs::tmpfs::testtmpfs;
    use crate::fs::tmpfs::tmpfs::TnSpec;
    use crate::fs::vnode::VREG;
    use crate::fs::vops::Cred;
    use hal::arch::current::param::PAGE_SIZE;

    /// The fixture, and the global limit set to `limit` bytes with nothing used.
    fn setup(limit: u64) -> MutexGuard<'static, ()> {
        let (g, _p) = testtmpfs::setup();
        let t = &vfs::<TestGlue>().tmpfs;
        t.bytes_limit.store(limit, Ordering::Relaxed);
        t.bytes_used.store(0, Ordering::Relaxed);
        g
    }

    /// A mount structure with a memory limit of `memlimit` bytes (0: the global limit) and
    /// room for `nodes` nodes (`tmpfs_mntmem_init`'s accounting, not hung on a mount).
    fn mount(memlimit: u64, nodes: u32) -> TmpfsMount<TestGlue> {
        TmpfsMount::new(nodes, memlimit)
    }

    fn used() -> u64 {
        vfs::<TestGlue>().tmpfs.bytes_used.load(Ordering::Relaxed)
    }

    #[test]
    fn a_limited_mount_counts_against_its_own_limit() {
        let _g = setup(64 * PAGE_SIZE as u64);
        let tmp = mount(2 * PAGE_SIZE as u64, 10);

        assert!(tmpfs_mem_incr(&tmp, PAGE_SIZE));
        assert!(
            !tmpfs_mem_incr(&tmp, PAGE_SIZE + 1),
            "past the mount's limit"
        );
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, PAGE_SIZE as u64);
        assert_eq!(
            used(),
            0,
            "a limited mount reserved its limit at mount time"
        );
        assert_eq!(tmpfs_pages_total::<TestGlue>(&tmp.tm_acc.read()), 2);
        assert_eq!(tmpfs_pages_avail::<TestGlue>(&tmp.tm_acc.read()), 1);

        tmpfs_mem_decr(&tmp, PAGE_SIZE);
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, 0);
        tmpfs_mntmem_destroy(&tmp);
    }

    #[test]
    fn unlimited_mounts_share_the_global_limit() {
        let _g = setup(4 * PAGE_SIZE as u64);
        let a = mount(0, 10);
        let b = mount(0, 10);

        assert!(tmpfs_mem_incr(&a, 3 * PAGE_SIZE));
        assert_eq!(used(), 3 * PAGE_SIZE as u64);
        assert!(!tmpfs_mem_incr(&b, 2 * PAGE_SIZE), "a used most of it");
        assert!(tmpfs_mem_incr(&b, PAGE_SIZE));
        assert_eq!(
            tmpfs_pages_total::<TestGlue>(&a.tm_acc.read()),
            3,
            "the free memory plus a's own"
        );
        assert_eq!(tmpfs_pages_avail::<TestGlue>(&b.tm_acc.read()), 0);

        tmpfs_mem_decr(&a, 3 * PAGE_SIZE);
        tmpfs_mem_decr(&b, PAGE_SIZE);
        assert_eq!(used(), 0);
    }

    #[test]
    fn names_are_accounted_and_allocated_in_quanta() {
        let _g = setup(64 * PAGE_SIZE as u64);
        let tmp = mount(PAGE_SIZE as u64, 10);

        let (reqs, _) = crate::testutil::malloc_requests(M_TEMP);
        let a = tmpfs_strname_alloc(&tmp, 1).expect("a name");
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, 32);
        let b = tmpfs_strname_alloc(&tmp, 33).expect("a name");
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, 32 + 64);
        assert_eq!((a.len(), b.len()), (32, 64), "the C's rounded malloc sizes");
        assert_eq!(
            crate::testutil::malloc_requests(M_TEMP),
            (reqs + 2, 64),
            "two M_TEMP requests, the last of 64 bytes"
        );
        tmpfs_strname_free(&tmp, Some(a), 1);
        tmpfs_strname_free(&tmp, Some(b), 33);
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, 0);
    }

    #[test]
    fn nodes_are_limited_in_number_and_memory_at_the_cs_charges() {
        let _g = setup(64 * PAGE_SIZE as u64);
        let tmp = mount(PAGE_SIZE as u64, 2);
        let reg = || TmpfsNode::new(VREG, TnSpec::None);

        let a = tmpfs_node_get(&tmp, reg).expect("a node");
        let b = tmpfs_node_get(&tmp, reg).expect("a node");
        assert!(tmpfs_node_get(&tmp, reg).is_none(), "tm_nodes_max reached");
        assert_eq!(tmp.tm_nodes_cnt.get(), 2);
        assert_eq!(
            tmp.tm_acc.read().tm_bytes_used,
            2 * TMPFS_NODE_CHARGE as u64
        );
        tmpfs_node_put(&tmp, a);
        tmpfs_node_put(&tmp, b);
        assert_eq!(tmp.tm_nodes_cnt.get(), 0);

        let de = tmpfs_dirent_get(&tmp).expect("an entry");
        assert!(de.td_node.is_none() && !de.name_is(b"x"));
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, TMPFS_DIRENT_CHARGE as u64);
        tmpfs_dirent_put(&tmp, de);
        assert_eq!(tmp.tm_acc.read().tm_bytes_used, 0);

        // A mount whose memory cannot hold one more node.
        let small = mount(PAGE_SIZE as u64, 100);
        assert!(tmpfs_mem_incr(&small, PAGE_SIZE - TMPFS_NODE_CHARGE + 1));
        assert!(tmpfs_node_get(&small, reg).is_none());
        assert_eq!(small.tm_nodes_cnt.get(), 0, "the count is given back");
        tmpfs_mem_decr(&small, PAGE_SIZE - TMPFS_NODE_CHARGE + 1);
    }

    #[test]
    fn rename_needs_a_new_name_only_when_the_names_differ() {
        let (_g, p) = testtmpfs::setup();
        let mut f = Componentname::new(0, 0, p, Cred::NULL);
        let mut t = Componentname::new(0, 0, p, Cred::NULL);
        let (a, b, c) = (b"name", b"name", b"other");
        f.set_name_lit(a);
        t.set_name_lit(b);
        assert!(!tmpfs_strname_neqlen(&f, &t));
        t.set_name_lit(c);
        assert!(tmpfs_strname_neqlen(&f, &t));
        assert_eq!(name_quantum(33), 64);
    }
}
/* </TESTS> */
