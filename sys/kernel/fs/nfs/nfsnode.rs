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
 *	@(#)nfsnode.h	8.9 (Berkeley) 5/14/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfsnode.h>` in the kernel: the client's node (`struct nfsnode`, the NFS equivalent of
//! ufs's inode), the silly rename record, the node's flags, and the conversions between nodes
//! and vnodes.
//!
//! Upstream: sys/nfs/nfsnode.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfsnode.rs@e8249449d905
//!
//! There is a unique nfsnode allocated for each active file, each current directory, each
//! mounted-on file, text file, and the root. An nfsnode is "named" by its file handle
//! (`nfs_nget`, `nfs_node`).
//!
//! ## Deviations
//! - `n_fhp` is gone, as in LZ: the C always points it at the node's own `n_fh`, which holds
//!   any handle; [`NfsFh`] is the handle and its size (`n_fh`, `n_fhsize`) in one cell.
//! - The unions `n_un1`/`n_un2` are their members side by side (`n_atim` and
//!   `n_cookieverf`, `n_mtim` and `n_direofoffset`), as in LZ; a node uses the pair of its
//!   type.
//! - `n_entry` links the node on a chain of the client's node hash (`nfs_node`'s
//!   `NfsNhash`), not on its mount's red-black tree (`nm_ntree`): the kernel's tree of
//!   counted, pinned elements does not exist, and every kernel file system keys its counted
//!   nodes in `PinList` chains bound to the kernel lock. A lookup is O(1) expected where the
//!   C's is O(log n); what changes is the order of a walk over every node (ddb's `show all
//!   nfsnodes`, N6.7a5).
//! - `nfs_bufq` (declared here, defined in `nfs_bio.c`) is `NfsTables::nfs_bufq`, a member of
//!   the glue's tables; its counts `nfs_bufqlen` and `nfs_bufqmax` are the statics
//!   [`NFS_BUFQLEN`] and [`NFS_BUFQMAX`] (their addresses are wait channels), and
//!   `nfs_numasync` (`nfs_vnops.c`'s) is [`NFS_NUMASYNC`] beside them, the nfsiods' count.
//!
//! ## Redesign
//! - LZ's `NfsNode` was an `nfs_node_pool` item handed around as `&'static NfsNode` with
//!   `Cell` members (an `unsafe impl Sync`), found by `VTONFS`'s cast of `v_data` (one
//!   `unsafe`, the tag checked first), its credentials raw `*const Ucred`, its silly rename
//!   record a `NonNull`. [`NfsNode`] is a counted item of "nfsnodepl" (`NfsTables`) in the
//!   vnode's typed slot, and [`vtonfs`] one more handle out of the slot, ufs's `vtoi`: taken
//!   once at a vnode operation's entry and lent down (`panic(9)` for a vnode whose node is no
//!   nfsnode of the glue, LZ's tag check made by the slot's type).
//! - The C changes the node under its vnode lock (`n_lock`), at `splbio` (the commit ranges)
//!   or under the kernel lock: the words are relaxed atomics; the attribute cache, the times
//!   and the handle `ReadMostly` cells copied out whole (cd9660's `inode`); the credentials and
//!   the silly rename record counted or owned values in one kernel-lock cell ([`NfsNodeK`]).
//! - N6.7a3: `n_vnode` is a `Link` (ufs's `i_vnode`), since `nfs_loadattrcache` moves a
//!   device's node to the alias `checkalias` found (`np->n_vnode = nvp`); [`nfstov`] reads
//!   it.

use core::sync::atomic::{AtomicI16, AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};

use hal::link::Link;
use hal::malloc::MBox;
use hal::pinlist::PinEntry;
use hal::pool::{PoolArc, PoolHome};
use hal::rwlock::{Guarded, RawRwlock, RecursiveRwlock};
use hal::slot::ReadMostly;

use super::nfsproto::{NFS_MAXFHSIZE, Nfsfh, Nfsuint64};
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::cred::CredRef;
use crate::fs::lockf::LockfSlot;
use crate::fs::vnode::{Vattr, Vnode};
use crate::proc::ProcGlue;

/// `NFS_COMMIT_PUSH_VALID`: push range valid; values for `n_commitflags`.
pub const NFS_COMMIT_PUSH_VALID: i32 = 0x0001;
/// `NFS_COMMIT_PUSHED_VALID`: pushed range valid.
pub const NFS_COMMIT_PUSHED_VALID: i32 = 0x0002;

/// `NFLUSHWANT`: want wakeup from a flush in prog; flags for `n_flag`.
pub const NFLUSHWANT: i16 = 0x0001;
/// `NFLUSHINPROG`: avoid multiple calls to `vinvalbuf()`.
pub const NFLUSHINPROG: i16 = 0x0002;
/// `NMODIFIED`: might have a modified buffer in bio.
pub const NMODIFIED: i16 = 0x0004;
/// `NWRITEERR`: flag write errors so close will know.
pub const NWRITEERR: i16 = 0x0008;
/// `NACC`: special file accessed.
pub const NACC: i16 = 0x0100;
/// `NUPD`: special file updated.
pub const NUPD: i16 = 0x0200;
/// `NCHG`: special file times changed.
pub const NCHG: i16 = 0x0400;

/// `struct sillyrename`: silly rename structure that hangs off the nfsnode until the name
/// can be removed by `nfs_inactive()`.
pub struct Sillyrename<G: ProcGlue> {
    /// `s_cred`: the credential the rename was done with (`crdup`'s, held).
    pub s_cred: CredRef,
    /// `s_dvp`: the directory (referenced, `vref`).
    pub s_dvp: &'static Vnode<G>,
    /// `s_namlen`: the length of `s_name`.
    pub s_namlen: i64,
    /// `s_name`: the silly name.
    pub s_name: [u8; 24],
}

/// A node's file handle: `n_fh`'s bytes and `n_fhsize`, read and written as one value.
#[derive(Clone, Copy)]
pub struct NfsFh {
    /// `n_fh`: the handle's buffer, `NFS_MAXFHSIZE` bytes.
    pub fh: Nfsfh,
    /// `n_fhsize`: size in bytes, of fh.
    pub size: i16,
}

impl NfsFh {
    /// The handle `fh` (at most `NFS_MAXFHSIZE` bytes; `None` past it).
    pub fn new(fh: &[u8]) -> Option<Self> {
        if fh.len() > NFS_MAXFHSIZE {
            return None;
        }
        let mut n = Nfsfh::new();
        n.fh_bytes[..fh.len()].copy_from_slice(fh);
        Some(Self {
            fh: n,
            size: fh.len() as i16,
        })
    }

    /// `n_fhp` with `n_fhsize`: the handle's bytes.
    pub fn bytes(&self) -> &[u8] {
        let n = usize::try_from(self.size).unwrap_or(0).min(NFS_MAXFHSIZE);
        &self.fh.fh_bytes[..n]
    }
}

/// The members of a node the C changes under the kernel lock and that own what they hold:
/// one cell, so the node carries one lock key for the three.
pub struct NfsNodeK<G: ProcGlue> {
    /// `n_sillyrename`: ptr to silly rename struct.
    pub n_sillyrename: Option<MBox<Sillyrename<G>>>,
    /// `n_rcred`: the credential of the last read, held.
    pub n_rcred: Option<CredRef>,
    /// `n_wcred`: the credential of the last write, held.
    pub n_wcred: Option<CredRef>,
}

/// `struct nfsnode`: the NFS equivalent of ufs's inode. Any similarity is purely
/// coincidental. A counted item of "nfsnodepl" (the module's Redesign).
pub struct NfsNode<G: ProcGlue> {
    /// `n_entry`: the link on the client's node hash (the kernel lock; the module's
    /// Deviations).
    pub n_entry: PinEntry<NfsNode<G>>,
    /// The item's home in "nfsnodepl" (`PoolArc::from_ref`).
    n_home: PoolHome<NfsNode<G>>,
    /// `n_size`: current size of file.
    pub n_size: AtomicU64,
    /// `n_vattr`: vnode attribute cache.
    pub n_vattr: ReadMostly<Vattr>,
    /// `n_attrstamp`: attr. cache timestamp.
    pub n_attrstamp: AtomicI64,
    /// `n_mtime`: prev modify time.
    pub n_mtime: ReadMostly<Timespec>,
    /// `n_ctime`: prev create time.
    pub n_ctime: AtomicI64,
    /// `n_fh` and `n_fhsize` (`n_fhp` names `n_fh`): the file handle.
    pub n_fh: ReadMostly<NfsFh>,
    /// `n_vnode`: associated vnode (moved to a device's alias by `nfs_loadattrcache`; read
    /// with [`nfstov`]).
    pub n_vnode: Link<Vnode<G>>,
    /// `n_lockf`: locking record of file.
    pub n_lockf: LockfSlot,
    /// `n_lock`: NFSnode lock.
    pub n_lock: RecursiveRwlock,
    /// `n_error`: save write error value (an errno's number, 0 for none).
    pub n_error: AtomicI32,
    /// `n_atim` (`n_un1.nf_atim`): special file times.
    pub n_atim: ReadMostly<Timespec>,
    /// `n_cookieverf` (`n_un1.nd_cookieverf`): cookie verifier (dir only).
    pub n_cookieverf: ReadMostly<Nfsuint64>,
    /// `n_mtim` (`n_un2.nf_mtim`).
    pub n_mtim: ReadMostly<Timespec>,
    /// `n_direofoffset` (`n_un2.nd_direof`): dir. EOF offset cache.
    pub n_direofoffset: AtomicI64,
    /// `n_sillyrename`, `n_rcred` and `n_wcred`: one cell of the kernel lock.
    pub n_k: Guarded<NfsNodeK<G>>,
    /// `n_flag`: flag for locking.. (`NFLUSHWANT`, ...).
    pub n_flag: AtomicI16,
    /// `n_accstamp`: access cache timestamp (`-1`: invalid).
    pub n_accstamp: AtomicI64,
    /// `n_accuid`: last access requester.
    pub n_accuid: AtomicU32,
    /// `n_accmode`: last mode requested.
    pub n_accmode: AtomicI32,
    /// `n_accerror`: last returned error.
    pub n_accerror: AtomicI32,
    /// `n_pushedlo`: 1st blk in committed range.
    pub n_pushedlo: AtomicI64,
    /// `n_pushedhi`: last block in range.
    pub n_pushedhi: AtomicI64,
    /// `n_pushlo`: 1st block in commit range.
    pub n_pushlo: AtomicI64,
    /// `n_pushhi`: last block in range.
    pub n_pushhi: AtomicI64,
    /// `n_commitlock`: serialize commits.
    pub n_commitlock: RawRwlock,
    /// `n_commitflags`: `NFS_COMMIT_*`.
    pub n_commitflags: AtomicI32,
}

hal::pool_home!(impl[G: ProcGlue] NfsNode<G>, n_home);

hal::pin_adapter!(
    /// `n_entry`: the link of a node on its chain of the client's node hash (the C's
    /// `RBT_ENTRY(nfsnode)` of `nm_ntree`).
    impl[G: ProcGlue] pub NfsNodeEntry<G>: NfsNode<G>, n_entry
);

/// A counted handle to a node (the vnode's slot's, a vnode operation's).
pub type NfsNodeRef<G> = PoolArc<NfsNode<G>>;

impl<G: ProcGlue> NfsNode<G> {
    /// A node of `vp` with the handle `fh`, every other member clear (`pool_get(..,
    /// PR_ZERO)`, `rrw_init`, `rw_init`): what `nfs_nget` fills.
    pub fn new(vp: &'static Vnode<G>, fh: NfsFh) -> Self {
        Self {
            n_entry: PinEntry::new(),
            n_home: PoolHome::new(),
            n_size: AtomicU64::new(0),
            n_vattr: ReadMostly::new(Vattr::new()),
            n_attrstamp: AtomicI64::new(0),
            n_mtime: ReadMostly::new(Timespec::ZERO),
            n_ctime: AtomicI64::new(0),
            n_fh: ReadMostly::new(fh),
            n_vnode: Link::to(vp),
            n_lockf: LockfSlot::new(),
            n_lock: RecursiveRwlock::new("nfsnode"),
            n_error: AtomicI32::new(0),
            n_atim: ReadMostly::new(Timespec::ZERO),
            n_cookieverf: ReadMostly::new(Nfsuint64 { nfsuquad: [0; 2] }),
            n_mtim: ReadMostly::new(Timespec::ZERO),
            n_direofoffset: AtomicI64::new(0),
            n_k: Guarded::new_kernel(NfsNodeK {
                n_sillyrename: None,
                n_rcred: None,
                n_wcred: None,
            }),
            n_flag: AtomicI16::new(0),
            n_accstamp: AtomicI64::new(0),
            n_accuid: AtomicU32::new(0),
            n_accmode: AtomicI32::new(0),
            n_accerror: AtomicI32::new(0),
            n_pushedlo: AtomicI64::new(0),
            n_pushedhi: AtomicI64::new(0),
            n_pushlo: AtomicI64::new(0),
            n_pushhi: AtomicI64::new(0),
            n_commitlock: RawRwlock::new("nfs_commitlk"),
            n_commitflags: AtomicI32::new(0),
        }
    }

    /// `np->n_fhp` with `np->n_fhsize`: a copy of the node's handle.
    pub fn fh(&self) -> NfsFh {
        *self.n_fh.read()
    }

    /// `np->n_flag & f`.
    #[inline]
    pub fn isset(&self, f: i16) -> bool {
        self.n_flag.load(Ordering::Relaxed) & f != 0
    }

    /// `np->n_flag |= f`.
    #[inline]
    pub fn set(&self, f: i16) {
        self.n_flag.fetch_or(f, Ordering::Relaxed);
    }

    /// `np->n_flag &= ~f`.
    #[inline]
    pub fn clr(&self, f: i16) {
        self.n_flag.fetch_and(!f, Ordering::Relaxed);
    }
}

/// `nfs_bufqlen`: the buffers on `nfs_bufq`; its address is the channel `nfs_asyncio`
/// sleeps on for room (`nfs_bio.c`'s).
pub static NFS_BUFQLEN: AtomicU32 = AtomicU32::new(0);

/// `nfs_bufqmax`: the most buffers `nfs_asyncio` queues before it waits (or, for a
/// read-ahead, gives up); each nfsiod adds its share when it starts (`nfs_bio.c`'s).
pub static NFS_BUFQMAX: AtomicU32 = AtomicU32::new(0);

/// `nfs_numasync`: the number of running nfsiods (`nfs_vnops.c`'s), kept by the nfsiods.
pub static NFS_NUMASYNC: AtomicI32 = AtomicI32::new(0);

/// `NFS_INVALIDATE_ATTRCACHE(np)`.
pub fn nfs_invalidate_attrcache<G: ProcGlue>(np: &NfsNode<G>) {
    np.n_attrstamp.store(0, Ordering::Relaxed);
}

/// `VTONFS(vp)`: the nfsnode of an NFS vnode, one more handle to it; `panic(9)` for a vnode
/// whose node is no nfsnode of the glue `G` (LZ's tag check, made by the slot's type).
pub fn vtonfs<G: ProcGlue>(vp: &Vnode<G>) -> NfsNodeRef<G> {
    match vp.node_with(|np: &NfsNode<G>| PoolArc::from_ref(np)) {
        Some(Some(np)) => np,
        _ => kpanic(format_args!("VTONFS: vnode {vp:p} has no nfsnode")),
    }
}

/// `NFSTOV(np)`: the vnode of an nfsnode (`panic(9)` for a node whose link was emptied,
/// which nothing does: the C's NULL `n_vnode`).
pub fn nfstov<G: ProcGlue>(np: &NfsNode<G>) -> &'static Vnode<G> {
    match np.n_vnode.get() {
        Some(vp) => vp,
        None => kpanic(format_args!("NFSTOV: node {np:p} without a vnode")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the handle's bounds and of the node's flag words.

    use super::*;

    #[test]
    fn a_handle_keeps_its_size_and_refuses_one_past_the_maximum() {
        let fh = NfsFh::new(&[1, 2, 3, 4]).expect("a handle");
        assert_eq!(fh.bytes(), &[1, 2, 3, 4]);
        assert_eq!(fh.size, 4);
        let big = [7u8; NFS_MAXFHSIZE];
        assert_eq!(
            NfsFh::new(&big).map(|h| h.bytes().len()),
            Some(NFS_MAXFHSIZE)
        );
        assert!(NfsFh::new(&[0u8; NFS_MAXFHSIZE + 1]).is_none());
        // A size word outside 0..=NFS_MAXFHSIZE reads no byte past the buffer.
        let odd = NfsFh {
            fh: Nfsfh::new(),
            size: -3,
        };
        assert!(odd.bytes().is_empty());
        let odd = NfsFh {
            fh: Nfsfh::new(),
            size: 200,
        };
        assert_eq!(odd.bytes().len(), NFS_MAXFHSIZE);
    }

    /// The "nfsnodepl" item's size (D-6: `vmstat -m` shows it; S11 stops above `bsd`'s item
    /// plus 64 bytes): 632 bytes, 648 with the counted item's header, against bsd's 584
    /// (LZ's shape, measured at N6.7a1's base with a compile-time probe of
    /// `size_of::<NfsNode>()`): S11's bound exactly. The `ReadMostly` cells' claim words, the
    /// kernel-lock cell's key and the hash link (`n_entry`, 32 bytes: a lock identity beside
    /// the two links and the element, where the C's `RBT_ENTRY` has four words) are the
    /// growth; the three owning members share one cell for it, and the link's chain is
    /// recomputed at removal rather than stored.
    #[test]
    fn the_nfsnodepl_item_size() {
        let n = core::mem::size_of::<NfsNode<crate::fs::testfs::TestGlue>>();
        assert_eq!(
            size_of::<PinEntry<NfsNode<crate::fs::testfs::TestGlue>>>(),
            32
        );
        assert_eq!(n, 632);
        assert!(n + 16 <= 584 + 64, "S11: the \"nfsnodepl\" item");
    }
}
/* </TESTS> */
