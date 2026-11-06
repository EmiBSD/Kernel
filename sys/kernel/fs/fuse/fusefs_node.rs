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
 * Copyright (c) 2012-2013 Sylvestre Gallon <ccna.syl@gmail.com>
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
/* </LICENSES> */

/* <CODE> */
//! `fusefs_node.h`: the in-core node of a FUSE file (`struct fusefs_node`), its file handles
//! and the `ITOV`/`VTOI` conversions.
//!
//! Upstream: sys/miscfs/fuse/fusefs_node.h @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fusefs_node.rs@3c62ede68802
//!
//! The prototypes of the header are functions of the files that define them:
//! `fuse_ihashinit`, `fuse_ihashget`, `fuse_ihashins`, `fuse_ihashrem` (`fuse_ihash`) and
//! `fusefs_fd_get` (`fuse_file`).
//!
//! ## Deviations
//! - `VTOI(vp)` checks the vnode's tag and its typed slot and panics on a vnode that is not a
//!   FUSE one or has no node, where the C would follow the pointer (LZ's).
//! - `vmstat -m`: a node's `M_FUSEFS` block is 224 bytes (the 208-byte `FusefsNode` and the
//!   `MArc`'s count and type; C 160, LZ 200), in the 256-byte bucket as the C's.
//!
//! ## Redesign
//! - The node is a counted block of `malloc(M_FUSEFS)` ([`FusefsNodeRef`], `MArc`): the
//!   vnode's typed slot holds one from `fusefs_vget` (N5.9f2) to `fusefs_reclaim`, and
//!   [`vtoi`] answers a handle from it; LZ's `&'static` over a raw block hung on `v_data` and
//!   its `unsafe` deref are gone. `i_fmp` is a counted handle to the mount's block, held for
//!   the node's life. The members the C changes through shared pointers are words.
//! - `i_hash` is a `PinEntry` of the kernel lock's hash chains (`fuse_ihash`); LZ's
//!   `i_hashed` flag, which stood for the C's `i_hash.le_prev != NULL`, is the entry's own
//!   `is_linked` (msdosfs's denode shape), and `i_chain` records the chain the node is on, so
//!   `fuse_ihashrem` takes it off without hashing again.
//! - `fufh[]` is indexed by [`FufhKind`], the three valid handle kinds: LZ's `idx()` of
//!   `FUFH_INVALID` (`-1 as usize`) could name no slot and is no longer writable; the slots'
//!   `fh_type` keeps the C's [`FufhType`], `FUFH_INVALID` included.

use hal::malloc::MArc;
use hal::pinlist::PinEntry;
use hal::rwlock::RecursiveRwlock;
use hal::sys::types::{Ino, Off};

use crate::assert::kpanic;
use crate::fs::fuse::fusefs::FusefsMntRef;
use crate::fs::lockf::LockfSlot;
use crate::fs::vnode::{VT_FUSEFS, Vnode};
use crate::fs::word::{I32Word, I64Word, U32Word, U64Word};
use crate::proc::ProcGlue;

/// `enum fufh_type`: the kind of a FUSE file handle, which is also its index in
/// `fusefs_node.fufh[]` ([`FufhKind`]).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)] // OpenBSD names, verbatim
pub enum FufhType {
    /// No handle.
    FUFH_INVALID = -1,
    /// Opened for reading.
    FUFH_RDONLY = 0,
    /// Opened for writing.
    FUFH_WRONLY = 1,
    /// Opened for reading and writing.
    FUFH_RDWR = 2,
    /// The number of handle kinds.
    FUFH_MAXTYPE = 3,
}

pub use FufhType::{FUFH_INVALID, FUFH_MAXTYPE, FUFH_RDONLY, FUFH_RDWR, FUFH_WRONLY};

impl FufhType {
    /// The kind of a handle slot's `fh_type`, `FUFH_INVALID` for any other value.
    const fn from_raw(v: i32) -> FufhType {
        match v {
            0 => FUFH_RDONLY,
            1 => FUFH_WRONLY,
            2 => FUFH_RDWR,
            3 => FUFH_MAXTYPE,
            _ => FUFH_INVALID,
        }
    }
}

/// A valid handle kind, the index of a slot of `fusefs_node.fufh[]`: the C's `enum
/// fufh_type` values `0..FUFH_MAXTYPE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FufhKind {
    /// `FUFH_RDONLY`.
    RdOnly = 0,
    /// `FUFH_WRONLY`.
    WrOnly = 1,
    /// `FUFH_RDWR`.
    RdWr = 2,
}

impl FufhKind {
    /// The kinds in the C's `for (type = 0; type < FUFH_MAXTYPE; type++)` order.
    pub const ALL: [FufhKind; FUFH_MAXTYPE as usize] =
        [FufhKind::RdOnly, FufhKind::WrOnly, FufhKind::RdWr];

    /// The C's enum value of the kind.
    pub const fn fufh_type(self) -> FufhType {
        match self {
            FufhKind::RdOnly => FUFH_RDONLY,
            FufhKind::WrOnly => FUFH_WRONLY,
            FufhKind::RdWr => FUFH_RDWR,
        }
    }
}

/// `struct fusefs_filehandle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FusefsFilehandle {
    /// `fh_id`: the daemon's handle.
    pub fh_id: u64,
    /// `fh_type`: the kind, `FUFH_INVALID` when the slot is unused.
    pub fh_type: FufhType,
}

/// A slot of `fufh[]`: the handle's two members as words.
#[derive(Debug, Default)]
pub struct FufhSlot {
    /// `fh_id`.
    fh_id: U64Word,
    /// `fh_type` (a [`FufhType`] value).
    fh_type: I32Word,
}

/// `struct fusefs_node`: the in-core node of a FUSE file.
pub struct FusefsNode<G: ProcGlue> {
    /// `i_hash`: hash chain.
    pub i_hash: PinEntry<FusefsNode<G>>,
    /// The chain `fuse_ihashins` put `i_hash` on.
    pub i_chain: U32Word,
    /// `i_vnode`: vnode associated with this inode.
    pub i_vnode: &'static Vnode<G>,
    /// `i_fmp`: the mount, a handle held for the node's life.
    pub i_fmp: FusefsMntRef<G>,
    /// `i_number`: the identity of the inode.
    pub i_number: Ino,
    /// `i_parent_cache`: parent inode (only dirs).
    pub i_parent_cache: U64Word,
    /// `i_lockf`: byte-level lock state.
    pub i_lockf: LockfSlot,
    /// `i_lock`: inode lock.
    pub i_lock: RecursiveRwlock,
    /// `fufh`: I/O: one handle per kind.
    pub fufh: [FufhSlot; FUFH_MAXTYPE as usize],
    /// `filesize`: meta.
    pub filesize: I64Word,
    /// `nlookup`: lookup count needed by `FUSE_FORGET`.
    pub nlookup: U64Word,
}

hal::pin_adapter!(
    /// `i_hash`: the link of a node on its hash chain (`struct fuse_ihashhead`).
    impl[G: ProcGlue] pub FusefsIHash<G>: FusefsNode<G>, i_hash
);

/// `struct fusefs_node *` held: a counted handle to a node's block of `malloc(M_FUSEFS)`,
/// which the last handle frees.
pub type FusefsNodeRef<G> = MArc<FusefsNode<G>>;

impl<G: ProcGlue> FusefsNode<G> {
    /// A node of `vp` on `fmp` for inode `ino`, every other member as `malloc(M_ZERO)`
    /// leaves it (`fusefs_vget` then marks the handles invalid).
    pub fn new(vp: &'static Vnode<G>, fmp: FusefsMntRef<G>, ino: Ino) -> Self {
        Self {
            i_hash: PinEntry::new(),
            i_chain: U32Word::new(0),
            i_vnode: vp,
            i_fmp: fmp,
            i_number: ino,
            i_parent_cache: U64Word::new(0),
            i_lockf: LockfSlot::new(),
            i_lock: RecursiveRwlock::new("fuseinode"),
            fufh: [const {
                FufhSlot {
                    fh_id: U64Word::new(0),
                    fh_type: I32Word::new(0),
                }
            }; FUFH_MAXTYPE as usize],
            filesize: I64Word::new(0),
            nlookup: U64Word::new(0),
        }
    }

    /// `ip->fufh[type]`.
    pub fn fufh(&self, t: FufhKind) -> FusefsFilehandle {
        let s = &self.fufh[t as usize];
        FusefsFilehandle {
            fh_id: s.fh_id.get(),
            fh_type: FufhType::from_raw(s.fh_type.get()),
        }
    }

    /// `ip->fufh[type] = fh`.
    pub fn set_fufh(&self, t: FufhKind, fh: FusefsFilehandle) {
        let s = &self.fufh[t as usize];
        s.fh_id.set(fh.fh_id);
        s.fh_type.set(fh.fh_type as i32);
    }

    /// `ip->filesize`.
    pub fn filesize(&self) -> Off {
        self.filesize.get()
    }
}

/// `ITOV(ip)`: the vnode of a node.
pub fn itov<G: ProcGlue>(ip: &FusefsNode<G>) -> &'static Vnode<G> {
    ip.i_vnode
}

/// `VTOI(vp)`: the node of a FUSE vnode, a handle from its typed slot (the module's
/// deviations).
pub fn vtoi<G: ProcGlue>(vp: &Vnode<G>) -> FusefsNodeRef<G> {
    match vp.node_arc::<FusefsNode<G>>() {
        Some(ip) if vp.tag() == VT_FUSEFS => ip,
        _ => kpanic(format_args!("VTOI: vnode {vp:p} has no fusefs node")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the node's handle slots: the three kinds index three slots, a slot
    // keeps the C's `fh_type`, `FUFH_INVALID` included.

    use std::assert_eq;

    use super::*;

    #[test]
    fn the_kinds_are_the_cs_enum_values() {
        for (i, k) in FufhKind::ALL.into_iter().enumerate() {
            assert_eq!(k as usize, i);
            assert_eq!(k.fufh_type() as i32, i as i32);
            assert_eq!(FufhType::from_raw(i as i32), k.fufh_type());
        }
        assert_eq!(FufhType::from_raw(-1), FUFH_INVALID);
        assert_eq!(FufhType::from_raw(7), FUFH_INVALID);
        assert_eq!(FUFH_MAXTYPE as usize, FufhKind::ALL.len());
    }

    #[test]
    fn a_slot_keeps_the_handle_and_its_type() {
        let s = FufhSlot::default();
        s.fh_id.set(u64::MAX);
        s.fh_type.set(FUFH_INVALID as i32);
        assert_eq!(
            (s.fh_id.get(), FufhType::from_raw(s.fh_type.get())),
            (u64::MAX, FUFH_INVALID)
        );
    }
}
/* </TESTS> */
