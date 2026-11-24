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
 * Copyright (c) 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfs_node.c	8.6 (Berkeley) 5/22/95
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS client's nodes in the kernel: the table of every mount's nfsnodes by file handle
//! (`nm_ntree`: [`nfs_nfind`], [`nfs_ninsert`], [`nfs_nremove`], [`nfs_nrehash`]) and the
//! vnode operation `nfs_reclaim` (give the node back to "nfsnodepl").
//!
//! Upstream: sys/nfs/nfs_node.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_node.rs@e8249449d905
//!
//! `nfs_nget` (find or make the node and vnode of a handle) and `nfs_inactive` (remove a
//! silly-renamed file on last close) call into the vnode operations (`nfs_vops`,
//! `nfs_removeit`): they come with them (N6.7a3), over this module's table.
//!
//! ## Deviations
//! - `nm_ntree` (`RBT_HEAD(nfs_nodetree, nfsnode)` in each mount, ordered by `nfsnode_cmp`)
//!   is one table of `PinList` chains for every NFS mount ([`NfsNhash`], `NfsTables::nhash`),
//!   chosen by a SipHash of the mount's address and the handle under a random key, its size
//!   `initialvnodes` (the kernel's other node hashes' shape, `fuse_ihash`, `msdosfs`'s
//!   `dehash`): the kernel has no tree of counted, pinned elements. A lookup is O(1)
//!   expected where the C's is O(log n); a walk over every node (ddb's `show all
//!   nfsnodes`, N6.7a5) meets them in another order. `nfs_ninit` (`RBT_INIT` of a mount's
//!   tree) has nothing left to do and is gone. The table is `malloc(M_NFSMNT)`: `vmstat -m`'s
//!   "NFS mount" row counts it from `nfs_vfs_init` on, where the C's tree costs nothing
//!   beyond the nodes' own links.
//! - The C writes a node's handle in place when a lookup finds the node under a new one
//!   (`nfs_lookitup`'s `*npp` case, for `nfs_sillyrename`), leaving its tree ordered by the
//!   old handle. A chain is chosen by the handle, so that write is [`nfs_nrehash`]: the node
//!   leaves its chain, takes the handle and goes on the new one's, in one section; it is then
//!   found by its new handle, which the C's tree may or may not do.
//! - `VFSLCKDEBUG` (`VLOCKSWORK`) is not configured; the `DIAGNOSTIC` `prtactive` report is
//!   behind feature `diagnostic`, and its panic on a vnode without `v_data` is `vtonfs`'s.
//!
//! ## Redesign
//! - LZ's tree was `nfsnode.rs`'s `NfsNodetree` adapter over `&'static NfsNode` pool items,
//!   inserted and removed through `unsafe` (the node "stays in place until `nfs_reclaim`
//!   removes it"), keyed by a node built on the stack around a copy of the handle. Here a
//!   node is a counted "nfsnodepl" item, pinned where the pool placed it, linked by its
//!   `n_entry` on a chain bound to the kernel lock (the C's lock for every client path): a
//!   lookup walks its chain under `klock::reader()` and hands out the vnode and its `v_id`
//!   (the caller's `vget` may sleep, `nfs_nget`'s recheck of `v_id` as the C's); an insertion,
//!   a removal and a rehash take `klock::witness()`, which raises the interrupt level for its
//!   section: one spl pair per node made or reclaimed where the C has none (docs/ROADMAP.md
//!   N12's row, as `ufs_ihash`'s and msdosfs's).
//! - The race check of `nfs_nget` after its sleeps (`RBT_FIND` again, then `RBT_INSERT` with
//!   `KASSERT(np2 == NULL)`) is [`nfs_ninsert`]'s `EEXIST`, the find and the insertion in one
//!   section (msdosfs's `msdosfs_hashins`).
//! - A node's chain is not stored (the "nfsnodepl" item is at S11's bound): it is computed
//!   again from the vnode's mount and the handle at removal, which [`nfs_nrehash`] keeps
//!   true.
//! - `nfs_reclaim` gives the vnode's typed slot back (`Vnode::take_node`, the C's
//!   `pool_put(&nfs_node_pool, vp->v_data)` and `v_data = NULL`): the node goes back to its
//!   pool with its last handle, and its credentials (`n_rcred`, `n_wcred`) are let go first,
//!   as the C's `crfree`s.

use core::pin::Pin;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::klock;
use hal::malloc::{M_WAITOK, MArray};
use hal::pinlist::PinList;
use hal::sys::errno::Errno;
use libkern::siphash::{SipHash24Ctx, SiphashKey};

use super::nfsmount::M_NFSMNT;
use super::nfsnode::{NfsFh, NfsNode, NfsNodeEntry, vtonfs};
use crate::assert::kpanic;
use crate::fs::cache::cache_purge;
use crate::fs::mount::Mount;
use crate::fs::subr::{hashinit, siphash_key, vfs};
use crate::fs::vnode::{VLARVAL, Vnode};
use crate::proc::ProcGlue;

/// A chain of the node hash.
type Chain<G> = PinList<NfsNodeEntry<G>>;

/// `nm_ntree` of every NFS mount (the module's Deviations): the chains, the table's size - 1,
/// the key.
pub struct NfsNhash<G: ProcGlue> {
    tbl: MArray<Chain<G>>,
    mask: u64,
    key: SiphashKey,
}

impl<G: ProcGlue> NfsNhash<G> {
    /// The chain of the handle `fh` of the mount `mp`: a SipHash of the mount's address
    /// (unique among mounted file systems) and the handle's size and bytes.
    fn chain_of(&self, mp: &Mount<G>, fh: &[u8]) -> &Chain<G> {
        let mut ctx = SipHash24Ctx::new(&self.key);
        ctx.update(&mp.addr().to_ne_bytes());
        ctx.update(&fh.len().to_ne_bytes());
        ctx.update(fh);
        // `& mask`, the table's size - 1, a power of two below 2^31 (`hashinit` of an
        // `int`): the index fits the table.
        let i = (ctx.end() & self.mask) as usize;
        &self.tbl[i]
    }
}

/// The table, when `nfs_vfs_init` has made it: `panic(9)` before.
fn with_nhash<G: ProcGlue, R>(f: impl FnOnce(&NfsNhash<G>) -> R) -> R {
    let t = vfs::<G>().nfs.nhash.read();
    match t.as_ref() {
        Some(h) => f(h),
        None => kpanic(format_args!("nfs_nhash: no table")),
    }
}

/// The mount a hashed node is on: its vnode's (set before the node is hashed, and kept until
/// after `nfs_reclaim`).
fn node_mount<G: ProcGlue>(np: &NfsNode<G>) -> &'static Mount<G> {
    match np.n_vnode.mount() {
        Some(mp) => mp,
        None => kpanic(format_args!(
            "nfs_node: node {np:p}: its vnode has no mount"
        )),
    }
}

/// `nfsnode_cmp(a, b) == 0` for a node of the mount `mp` and the handle `fh`: the same mount
/// (the C's tree is the mount's), the same size, the same bytes.
fn is<G: ProcGlue>(np: &NfsNode<G>, mp: &Mount<G>, fh: &[u8]) -> bool {
    np.n_vnode.mount().is_some_and(|m| ptr::eq(m, mp)) && np.fh().bytes() == fh
}

/// `nm_ntree`'s making for every mount (`RBT_INIT`): the table, `hashinit(initialvnodes,
/// M_NFSMNT, M_WAITOK)` with a random key; once, from `nfs_vfs_init`.
pub fn nfs_nhashinit<G: ProcGlue>() {
    let initialvnodes = vfs::<G>().initialvnodes.load(Ordering::Relaxed);
    let Some((tbl, mask)) = hashinit::<NfsNodeEntry<G>>(initialvnodes, M_NFSMNT, M_WAITOK) else {
        kpanic(format_args!("nfs_vfs_init: no memory for the node hash"));
    };
    let h = NfsNhash {
        tbl,
        mask,
        key: siphash_key(),
    };
    if vfs::<G>().nfs.nhash.replace(Some(h)).is_some() {
        kpanic(format_args!("nfs_vfs_init: twice"));
    }
}

/// `RBT_FIND(nfs_nodetree, &nmp->nm_ntree, &find)` of `nfs_nget`: the vnode of the node of
/// the handle `fh` on the mount `mp`, and its `v_id` read in the same section, for the
/// caller's `vget` and recheck (`nfs_nget`'s `loop`).
pub fn nfs_nfind<G: ProcGlue>(mp: &Mount<G>, fh: &[u8]) -> Option<(&'static Vnode<G>, u32)> {
    with_nhash::<G, _>(|h| {
        let r = klock::reader();
        h.chain_of(mp, fh)
            .iter(&r)
            .find(|np| is(np, mp, fh))
            .map(|np| (np.n_vnode, np.n_vnode.id()))
    })
}

/// `RBT_INSERT(nfs_nodetree, &nmp->nm_ntree, np)` after `nfs_nget`'s second `RBT_FIND`: the
/// node goes on the chain of its mount and handle, unless a node of the same pair is there
/// already (`EEXIST`, nothing done: the caller lost the race its sleeps opened and starts
/// again, as the C's `goto loop`).
pub fn nfs_ninsert<G: ProcGlue>(np: Pin<&NfsNode<G>>) -> Result<(), Errno> {
    let mp = node_mount(&np);
    let fh = np.fh();
    with_nhash::<G, _>(|h| {
        let mut w = klock::witness();
        let chain = h.chain_of(mp, fh.bytes());
        if chain.iter(&w).any(|other| is(other, mp, fh.bytes())) {
            return Err(Errno::EEXIST);
        }
        chain.insert_head(&mut w, np);
        Ok(())
    })
}

/// `RBT_REMOVE(nfs_nodetree, &nmp->nm_ntree, np)`: the node off its chain; nothing for a node
/// that is on none (a vnode `nfs_nget` never finished, `VLARVAL`).
pub fn nfs_nremove<G: ProcGlue>(np: &NfsNode<G>) {
    if !np.n_entry.is_linked() {
        return;
    }
    let mp = node_mount(np);
    let fh = np.fh();
    with_nhash::<G, _>(|h| {
        let mut w = klock::witness();
        h.chain_of(mp, fh.bytes()).remove(&mut w, np);
    });
}

/// `np->n_fhp = &np->n_fh; bcopy(nfhp, np->n_fhp, fhlen); np->n_fhsize = fhlen;` of
/// `nfs_lookitup` (the module's Deviations): the node takes the handle `fh`, and a hashed one
/// moves to the chain of its new handle in the same section. The only writer of a node's
/// handle after `nfs_nget`.
pub fn nfs_nrehash<G: ProcGlue>(np: Pin<&NfsNode<G>>, fh: NfsFh) {
    if !np.n_entry.is_linked() {
        let _ = np.n_fh.replace(fh);
        return;
    }
    let mp = node_mount(&np);
    with_nhash::<G, _>(|h| {
        let mut w = klock::witness();
        h.chain_of(mp, np.fh().bytes()).remove(&mut w, &np);
        let _ = np.n_fh.replace(fh);
        h.chain_of(mp, fh.bytes()).insert_head(&mut w, np);
    });
}

/// `nfs_reclaim` (`vop_reclaim`): reclaim an nfsnode so that it can be used for other
/// purposes.
pub fn nfs_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    crate::fs::ufs::ufs::ufs_inode::pushing_active(vp, "nfs_reclaim: pushing active");

    if vp.flag() & VLARVAL != 0 {
        // vnode was incompletely set up, just return as we are throwing it away.
        return Ok(());
    }
    let np = vtonfs(vp);
    nfs_nremove(&np);

    // `if (np->n_rcred) crfree(np->n_rcred)`, and the same of `n_wcred`.
    let creds = {
        let mut w = klock::witness();
        let k = np.n_k.get_mut(&mut w);
        (k.n_rcred.take(), k.n_wcred.take())
    };
    drop(creds);

    cache_purge(vp);
    // `pool_put(&nfs_node_pool, vp->v_data); vp->v_data = NULL;`: the slot's handle, and with
    // ours the last one.
    drop(np);
    if vp.take_node().is_none() {
        kpanic(format_args!("nfs_reclaim: vnode {vp:p} without nfsnode"));
    }
    Ok(())
}

/// `nfs_test_reset`'s part: forgets the node hash of a test glue's tables without freeing it
/// (the memory behind it is gone with the last test's, testing.md).
#[cfg(any(test, feature = "doubles"))]
pub(crate) fn nfs_nhash_test_reset<G: ProcGlue>() {
    core::mem::forget(vfs::<G>().nfs.nhash.replace(None));
}
/* </CODE> */
