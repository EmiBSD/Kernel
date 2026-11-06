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
 * Copyright (c) 2005, 2006, 2007 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Julio M. Merino Vidal, developed as part of Google's Summer of Code
 * 2005 program.
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
//! `<tmpfs/tmpfs.h>`: the in-memory representation of tmpfs: the inode (`struct tmpfs_node`),
//! the directory entry (`struct tmpfs_dirent`), the mount (`struct tmpfs_mount`), the NFS file
//! handle (`struct tmpfs_fid`), their constants and the conversions from the VFS structures.
//!
//! Upstream: sys/tmpfs/tmpfs.h @ 3ce1f3f79392
//! LZ: sys/tmpfs/tmpfs.rs@3c62ede68802
//!
//! The prototypes of `tmpfs_subr.c` and `tmpfs_mem.c` are their functions in
//! `tmpfs_subr.rs` and `tmpfs_mem.rs`.
//!
//! **Who holds a node or an entry.** A node is an item of the glue's `tmpfs_node_pool`
//! ([`TmpfsTables`](super::tmpfs_vfsops::TmpfsTables)) that lives, as the C's, from
//! `tmpfs_alloc_node` to `tmpfs_free_node`: its place on the mount's `tm_nodes` holds a
//! reference (parked with `PoolArc::forget` when it is linked, taken back with
//! `PoolArc::adopt` when `tmpfs_free_node` unlinks it), so a node nothing else names stays
//! alive and reachable for the unmount's rounds. A directory entry is an item of
//! `tmpfs_dirent_pool` whose place on its directory's `tn_dir` holds a reference likewise
//! (`tmpfs_dir_attach` to `tmpfs_dir_detach`); detached, it is its holder's handle, which
//! `tmpfs_free_dirent` drops. The other pointers of the C are counted handles too ([`NodeRef`]
//! cells): the entry's `td_node`, a directory's `tn_parent` and `tn_readdir_lastp`, a node's
//! `tn_dirent_hint`, the mount's `tm_root`, the vnode's typed slot (`v_data`); every one the
//! C clears (`tmpfs_dir_detach` clears the entry's node, the node's hint and the
//! subdirectory's parent) is cleared at that point, so no cycle (a directory and its parent,
//! a node and its hint) outlives the detach that breaks it, and the root's own parent goes
//! with the root (`tmpfs_free_node`). A late holder (a vnode's reclaim, a lookup's handle)
//! keeps the item alive where the C would free it under it.
//!
//! **Locks.** What the C writes under the vnode lock (`tn_vlock`, which every tmpfs path
//! holds with the kernel lock: `vn_read`, `vn_write`, `namei`) are relaxed words (the inode's
//! shape); `tn_nlock` (a [`RawRwlock`], "tvlk") guards the vnode association, read without
//! it where the C reads it (a locked directory's vnode, the asserts); `tm_lock` ("tmplk")
//! is the witness of the mount's node list, `tm_acc_lock` ("tacclk") owns the accounting
//! ([`TmAcc`]). A directory's entries are a `PinList` whose witness is the kernel lock
//! (`klock::reader()` for the walks, `klock::witness()` for a change: msdosfs's hash shape),
//! the C's protocol (the vnode lock) kept by the callers.
//!
//! ## Deviations
//! - `union tn_spec` is the enum [`TnSpec`]: a node of each type carries its case alone
//!   (`VFIFO` and `VSOCK` none), and a case read on a node of another type is `panic(9)`
//!   ([`TmpfsNode::dir`], [`TmpfsNode::reg`]), where the C reads the union's bytes.
//! - `tn_flags` is a `u32`, the type of the `UF_*`/`SF_*` flags it holds (an `int` in C).
//! - `TMPFS_DIRSEQ_FULL`, `TMPFS_NODE_RECLAIMING`, `TMPFS_NODE_GEN` and
//!   `TMPFS_VALIDATE_DIR` are functions with lowercase names; `VFS_TO_TMPFS`,
//!   `VP_TO_TMPFS_DIR` and `VP_TO_TMPFS_NODE` are [`vfstotmpfs`], [`vptodir`] and
//!   [`vptonode`], counted handles.
//! - `struct tmpfs_fid` is read from and written into a `struct fid` member by member
//!   ([`TmpfsFid::from_fid`], [`TmpfsFid::to_fid`]), where the C `memcpy`s it over the
//!   `struct fid`.
//! - `tn_gen` is an atomic word: the C's `tn_gen |= TMPFS_RECLAIMING_BIT` (its atomic form is
//!   commented out, tmpfs_subr.c) is a `fetch_or`, the same bits under the kernel lock.
//!
//! ## Redesign
//! - LZ's members were `Cell`s of `&'static TmpfsNode`/`&'static TmpfsDirent` pointers into raw
//!   pool items (`crget`/`crfree`'s idiom of `docs/C_TO_RUST.md`, the lifetimes the C's
//!   conventions), with `unsafe impl Sync` on the three structures, the name and the link as
//!   raw pointers read in `unsafe` slices, and `VFS_TO_TMPFS`/`VP_TO_TMPFS_NODE` as `unsafe`
//!   casts of `mnt_data`/`v_data` (7 `unsafe`). Here the node and the entry are
//!   [`TmpfsNode`] and [`TmpfsDirent`] items of recorded pools, their C pointers counted
//!   handles in [`NodeRef`] cells and their lists `PinList`s whose places hold parked
//!   references (the module's notes); `struct tmpfs_mount` is [`TmpfsMount`] in a counted
//!   `malloc(M_MISCFSMNT)` block hung from the mount's typed slot; the vnode reaches its node
//!   through its typed slot ([`vptonode`], udf's `VTOU` shape). The name and the target are
//!   the `MArray`s of their `malloc(M_TEMP)` blocks (the C's sizes, `tmpfs_mem.rs`).
//! - `tn_spec.tn_reg`'s `tn_aobj_pgptr`/`tn_aobj_pgnum` pair is [`TnReg::cached`], the page
//!   number with the [`ObjectWindow`] that maps it (no address kept apart from its range); a
//!   copy moves the window out of the cell and back (`tmpfs_subr.rs`), so no reference to the
//!   cell is held across the copy's faults and sleeps, and no lock is taken for it.
//! - `TMPFS_VALIDATE_DIR`'s size check is the C's `sizeof(tmpfs_dirent_t)`
//!   ([`TMPFS_DIRENT_CHARGE`], 48: D-N5.9-6), the size a directory grows by per entry (a
//!   directory's `st_size`); LZ's `size_of::<TmpfsDirent>()` happened to be 48 too, a
//!   coincidence of its members that the kernel's counted entry (120 bytes) does not keep.

use core::sync::atomic::{AtomicU64, Ordering};

use hal::link::Link;
use hal::malloc::{MArc, MArray};
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{PoolArc, PoolHome};
use hal::rwlock::{RawRwlock, RecursiveRwlock, Rwlocked};
use hal::slot::ReadMostly;
use hal::sys::mount::Fid;
use hal::sys::types::{Dev, Ino};

use crate::assert::{kassert, kpanic};
use crate::fs::lockf::LockfSlot;
use crate::fs::mount::Mount;
use crate::fs::vnode::{VDIR, VNOVAL, VT_TMPFS, Vnode, Vtype};
use crate::fs::vops::vop_islocked;
use crate::fs::word::{I64Word, U16Word, U32Word, U64Word};
use crate::proc::ProcGlue;
use crate::resource::TimespecWord;
use crate::vm::aobj::Aobj;
use crate::vm::km::ObjectWindow;
use crate::vm::types::Voff;

/// `sizeof(tmpfs_node_t)` of the C (LP64, no `WITNESS`): what `tmpfs_node_get` charges to the
/// mount and `tmpfs_statfs` divides the free memory by (D-N5.9-6: `statfs`'s `f_files` and
/// `f_ffree` are OpenBSD's, whatever [`TmpfsNode`] weighs). Pinned by a `_Static_assert`
/// compiled over the reference headers (N5.9t1).
pub const TMPFS_NODE_CHARGE: usize = 272;

/// `sizeof(tmpfs_dirent_t)` of the C (LP64): what `tmpfs_dirent_get` charges and a
/// directory's `tn_size` grows by per entry (D-N5.9-6).
pub const TMPFS_DIRENT_CHARGE: usize = 48;

/// A counted handle of a node ([`TmpfsNode`], an item of `tmpfs_node_pool`).
pub type TmpfsNodeRef<G> = PoolArc<TmpfsNode<G>>;

/// A counted handle of a directory entry ([`TmpfsDirent`], an item of `tmpfs_dirent_pool`).
pub type TmpfsDirentRef<G> = PoolArc<TmpfsDirent<G>>;

/// A C pointer the redesign counts (the module's notes): the handle or none (`NULL`), read
/// by a clone (`ReadMostly`'s count, never held across a sleep), replaced where the C
/// assigns.
pub struct NodeRef<T: Send + Sync + 'static>(ReadMostly<Option<PoolArc<T>>>);

impl<T: Send + Sync + 'static> NodeRef<T> {
    /// `NULL`.
    pub const fn new() -> Self {
        Self(ReadMostly::new(None))
    }

    /// The pointer, counted again (`None` for `NULL`).
    pub fn get(&self) -> Option<PoolArc<T>> {
        self.0.read().clone()
    }

    /// Whether it is `NULL`.
    pub fn is_none(&self) -> bool {
        self.0.read().is_none()
    }

    /// Whether it names `x` (an address comparison, nothing counted).
    pub fn points_to(&self, x: &T) -> bool {
        self.0
            .read()
            .as_ref()
            .is_some_and(|h| core::ptr::eq(PoolArc::as_ptr(h), x))
    }

    /// `p = v`: the old handle, which the caller drops.
    pub fn set(&self, v: Option<PoolArc<T>>) -> Option<PoolArc<T>> {
        self.0.replace(v)
    }
}

impl<T: Send + Sync + 'static> Default for NodeRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct tmpfs_dirent` (`tmpfs_dirent_t`): internal representation of a tmpfs directory
/// entry. All fields are protected by vnode lock (the C's); the link by the kernel lock.
pub struct TmpfsDirent<G: ProcGlue> {
    /// `td_entries`: the link in the directory's `tn_dir` list.
    pub td_entries: PinEntry<TmpfsDirent<G>>,
    /// The item's place in `tmpfs_dirent_pool`, for the list's parked reference.
    home: PoolHome<TmpfsDirent<G>>,
    /// `td_node`: the inode this entry refers to (`NULL` while detached).
    pub td_node: NodeRef<TmpfsNode<G>>,
    /// `td_seq`: sequence number, see `tmpfs_dir_getseq()`.
    pub td_seq: U64Word,
    /// `td_name`: the name, its `tmpfs_strname_alloc` block (`roundup2(len, 32)` bytes of
    /// `M_TEMP`), `NULL` in a fresh item (`PR_ZERO`) until `tmpfs_alloc_dirent` sets it;
    /// `tmpfs_rename` writes a new name over it or replaces it.
    td_name: ReadMostly<Option<MArray<u8>>>,
    /// `td_namelen`: the name's length.
    pub td_namelen: U16Word,
}

impl<G: ProcGlue> TmpfsDirent<G> {
    /// A fresh item's value (`pool_get(PR_ZERO)`): no node, no name; `tmpfs_alloc_dirent`
    /// names it and leaves it unnumbered (`TMPFS_DIRSEQ_NONE`).
    pub const fn new() -> Self {
        Self {
            td_entries: PinEntry::new(),
            home: PoolHome::new(),
            td_node: NodeRef::new(),
            td_seq: U64Word::new(0),
            td_name: ReadMostly::new(None),
            td_namelen: U16Word::new(0),
        }
    }

    /// Whether the entry's name is `name` (`de->td_namelen == len && !memcmp(de->td_name,
    /// name, len)`: the length first, the bytes read only when it matches).
    pub fn name_is(&self, name: &[u8]) -> bool {
        usize::from(self.td_namelen.get()) == name.len()
            && self
                .td_name
                .read()
                .as_deref()
                .and_then(|b| b.get(..name.len()))
                == Some(name)
    }

    /// `f(name)`: the name lent to `f`, which must not sleep (the cell's read).
    pub fn with_name<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        let len = usize::from(self.td_namelen.get());
        let b = self.td_name.read();
        f(b.as_deref().and_then(|b| b.get(..len)).unwrap_or(&[]))
    }

    /// `de->td_name = name; de->td_namelen = len`: the old block (none for a fresh entry),
    /// which the caller frees (`tmpfs_strname_free`): `tmpfs_alloc_dirent`'s, `tmpfs_rename`'s
    /// new name.
    pub fn set_name(&self, name: MArray<u8>, len: u16) -> Option<MArray<u8>> {
        let old = self.td_name.replace(Some(name));
        self.td_namelen.set(len);
        old
    }

    /// `memcpy(de->td_name, name, len); de->td_namelen = len`: a new name in the same block
    /// (`tmpfs_rename` when the rounded lengths agree; the block holds them).
    pub fn rename_in_place(&self, name: &[u8]) {
        self.td_name.update(|b| {
            if let Some(b) = b {
                b[..name.len()].copy_from_slice(name);
            }
        });
        self.td_namelen.set(name.len() as u16);
    }

    /// `de->td_name = NULL`: the block, which `tmpfs_free_dirent` frees.
    pub fn take_name(&self) -> Option<MArray<u8>> {
        self.td_name.replace(None)
    }

    /// `de->td_node`, which the caller knows is set (the C dereferences it).
    pub fn node(&self) -> TmpfsNodeRef<G> {
        match self.td_node.get() {
            Some(node) => node,
            None => kpanic(format_args!(
                "tmpfs: directory entry {self:p} without a node"
            )),
        }
    }
}

impl<G: ProcGlue> Default for TmpfsDirent<G> {
    fn default() -> Self {
        Self::new()
    }
}

hal::pool_home!(impl[G: ProcGlue] TmpfsDirent<G>, home);

hal::pin_adapter!(
    /// `TAILQ_HEAD(tmpfs_dir, tmpfs_dirent)`: the entries of a directory (`td_entries`).
    impl[G: ProcGlue] pub TmpfsDir<G>: TmpfsDirent<G>, td_entries
);

/// `tn_spec.tn_dir`: type case VDIR.
pub struct TnDir<G: ProcGlue> {
    /// `tn_parent`: parent directory (root inode points to itself).
    pub tn_parent: NodeRef<TmpfsNode<G>>,
    /// `tn_dir`: list of directory entries.
    pub tn_dir: PinList<TmpfsDir<G>>,
    /// `tn_next_seq`: last given sequence number.
    pub tn_next_seq: U64Word,
    /// `tn_readdir_lastp`: pointer of the last directory entry returned by the readdir(3)
    /// operation.
    pub tn_readdir_lastp: NodeRef<TmpfsDirent<G>>,
}

impl<G: ProcGlue> TnDir<G> {
    /// `TAILQ_INIT(&tn_dir)`, no parent, the first sequence number `TMPFS_DIRSEQ_START`, no
    /// readdir cache (`tmpfs_alloc_node`'s VDIR case).
    pub const fn new() -> Self {
        Self {
            tn_parent: NodeRef::new(),
            tn_dir: PinList::new(),
            tn_next_seq: U64Word::new(TMPFS_DIRSEQ_START),
            tn_readdir_lastp: NodeRef::new(),
        }
    }
}

impl<G: ProcGlue> Default for TnDir<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct tn_reg`: type case VREG.
pub struct TnReg {
    /// `tn_aobj`: underlying UVM object to store contents, set by `tmpfs_alloc_node` once it
    /// has it (the node's handle; its reference, `uao_create`'s, is detached by
    /// `tmpfs_free_node`).
    pub tn_aobj: OnceStatic<PoolArc<Aobj>>,
    /// `tn_aobj_pages`: the pages accounted to the object.
    pub tn_aobj_pages: U64Word,
    /// `tn_aobj_pgnum` and `tn_aobj_pgptr`: the object offset of the one page kept mapped for
    /// the next small transfer and its window (`(voff_t)-1` and `NULL`: `None`).
    pub cached: ReadMostly<Option<(Voff, ObjectWindow)>>,
}

impl TnReg {
    /// No object yet, no page, no page cached (`tmpfs_alloc_node`'s VREG case).
    pub const fn new() -> Self {
        Self {
            tn_aobj: OnceStatic::new(),
            tn_aobj_pages: U64Word::new(0),
            cached: ReadMostly::new(None),
        }
    }

    /// `tn_uobj`: the object, which `tmpfs_alloc_node` set.
    pub fn aobj(&self) -> &PoolArc<Aobj> {
        match self.tn_aobj.get() {
            Some(a) => a,
            None => kpanic(format_args!("tmpfs: a regular file without its object")),
        }
    }
}

impl Default for TnReg {
    fn default() -> Self {
        Self::new()
    }
}

/// `union tn_spec`: the data that is only applicable to a particular type (see the module's
/// deviations).
pub enum TnSpec<G: ProcGlue> {
    /// `tn_dev`: type case VBLK or VCHR (`tn_rdev`).
    Dev(Dev),
    /// `tn_dir`: type case VDIR.
    Dir(TnDir<G>),
    /// `tn_lnk`: type case VLNK, the target's `tmpfs_strname_alloc` block, set once by
    /// `tmpfs_alloc_node` after the item is got (empty for an empty target: the C's `NULL`).
    Lnk(OnceStatic<MArray<u8>>),
    /// `tn_reg`: type case VREG.
    Reg(TnReg),
    /// VFIFO, VSOCK: no case.
    None,
}

/// `struct tmpfs_node` (`tmpfs_node_t`): internal representation of a tmpfs file system
/// node -- inode. Its identity (`tn_type`, `tn_spec`'s case) is set when the item is made;
/// the rest are words written under the vnode lock (`tn_vnode` under `tn_nlock`).
pub struct TmpfsNode<G: ProcGlue> {
    /// `tn_entries`: the link in the mount's `tm_nodes`.
    pub tn_entries: PinEntry<TmpfsNode<G>>,
    /// The item's place in `tmpfs_node_pool`, for the vnode's slot and the list.
    home: PoolHome<TmpfsNode<G>>,
    /// `tn_nlock`: node lock (`"tvlk"`), held across `getnewvnode`/`vget` while the vnode
    /// association changes.
    pub tn_nlock: RawRwlock,
    /// `tn_vlock`: vnode lock (`rrw_init_flags(.., "tnode", RWL_DUPOK | RWL_IS_VNODE)`),
    /// `VOP_LOCK`'s.
    pub tn_vlock: RecursiveRwlock,
    /// `tn_vnode`: each inode has a corresponding vnode. It is a bi-directional association.
    /// Whenever vnode is allocated, its `v_data` field is set to the inode it reference, and
    /// `tn_vnode` is set to point to the said vnode. It may be `None` when the node is unused
    /// (that is, no vnode has been allocated or it has been reclaimed). Written under
    /// `tn_nlock`.
    pub tn_vnode: Link<Vnode<G>>,
    /// `tn_dirent_hint`: directory entry. Only a hint, since hard link can have multiple.
    pub tn_dirent_hint: NodeRef<TmpfsDirent<G>>,
    /// `tn_type`: the inode type: VBLK, VCHR, VDIR, VFIFO, VLNK, VREG or VSOCK.
    pub tn_type: Vtype,
    /// `tn_id`: inode identifier (set by `tmpfs_alloc_node` after the item is got, as the C).
    pub tn_id: U64Word,
    /// `tn_gen`: generation number (and [`TMPFS_RECLAIMING_BIT`]).
    pub tn_gen: AtomicU64,
    /// `tn_size`: the inode size.
    pub tn_size: I64Word,
    /// `tn_uid`.
    pub tn_uid: U32Word,
    /// `tn_gid`.
    pub tn_gid: U32Word,
    /// `tn_mode`.
    pub tn_mode: U32Word,
    /// `tn_flags`.
    pub tn_flags: U32Word,
    /// `tn_links`.
    pub tn_links: U32Word,
    /// `tn_atime`.
    pub tn_atime: TimespecWord,
    /// `tn_mtime`.
    pub tn_mtime: TimespecWord,
    /// `tn_ctime`.
    pub tn_ctime: TimespecWord,
    /// `tn_birthtime`.
    pub tn_birthtime: TimespecWord,
    /// `tn_lockf`: head of byte-level lock list (used by `tmpfs_advlock`).
    pub tn_lockf: LockfSlot,
    /// `tn_spec`.
    pub tn_spec: TnSpec<G>,
}

impl<G: ProcGlue> TmpfsNode<G> {
    /// A node of type `tn_type` with the case `spec`, every member cleared (no link, no vnode,
    /// no hint, unlocked), before `tmpfs_alloc_node` fills it in.
    pub fn new(tn_type: Vtype, spec: TnSpec<G>) -> Self {
        Self {
            tn_entries: PinEntry::new(),
            home: PoolHome::new(),
            tn_nlock: RawRwlock::new("tvlk"),
            tn_vlock: RecursiveRwlock::new("tnode"),
            tn_vnode: Link::new(),
            tn_dirent_hint: NodeRef::new(),
            tn_type,
            tn_id: U64Word::new(0),
            tn_gen: AtomicU64::new(0),
            tn_size: I64Word::new(0),
            tn_uid: U32Word::new(0),
            tn_gid: U32Word::new(0),
            tn_mode: U32Word::new(0),
            tn_flags: U32Word::new(0),
            tn_links: U32Word::new(0),
            tn_atime: TimespecWord::new(),
            tn_mtime: TimespecWord::new(),
            tn_ctime: TimespecWord::new(),
            tn_birthtime: TimespecWord::new(),
            tn_lockf: LockfSlot::new(),
            tn_spec: spec,
        }
    }

    /// `node->tn_spec.tn_dir`: `panic(9)` on a node that is not a directory.
    pub fn dir(&self) -> &TnDir<G> {
        match &self.tn_spec {
            TnSpec::Dir(d) => d,
            _ => kpanic(format_args!("tmpfs node {self:p}: not a directory")),
        }
    }

    /// `node->tn_spec.tn_reg`: `panic(9)` on a node that is not a regular file.
    pub fn reg(&self) -> &TnReg {
        match &self.tn_spec {
            TnSpec::Reg(r) => r,
            _ => kpanic(format_args!("tmpfs node {self:p}: not a regular file")),
        }
    }

    /// `node->tn_spec.tn_dev.tn_rdev` (`VNOVAL` for another type, `tmpfs_getattr`'s answer).
    pub fn rdev(&self) -> Dev {
        match self.tn_spec {
            TnSpec::Dev(d) => d,
            _ => VNOVAL as Dev,
        }
    }

    /// The symbolic link's target: the `tn_size` bytes of `tn_spec.tn_lnk.tn_link`.
    pub fn link(&self) -> &[u8] {
        match &self.tn_spec {
            TnSpec::Lnk(b) => {
                let n = usize::try_from(self.tn_size.get()).unwrap_or(0);
                b.get().and_then(|b| b.get(..n)).unwrap_or(&[])
            }
            _ => &[],
        }
    }

    /// `node->tn_vnode`.
    pub fn vnode(&self) -> Option<&'static Vnode<G>> {
        self.tn_vnode.get()
    }
}

hal::pool_home!(impl[G: ProcGlue] TmpfsNode<G>, home);

hal::pin_adapter!(
    /// `LIST_HEAD(tmpfs_node_list, tmpfs_node)`: the inodes of a mount (`tn_entries`).
    impl[G: ProcGlue] pub TmpfsNodeList<G>: TmpfsNode<G>, tn_entries
);

/// `TMPFS_MAXNAMLEN`. Validate maximum `td_namelen` length: `TMPFS_MAXNAMLEN < UINT16_MAX`
/// (checked at the end of the file).
pub const TMPFS_MAXNAMLEN: usize = 255;

// Reserved values for the virtual entries (the first must be 0) and EOF. The start/end of
// the incremental range, see tmpfs_dir_getseq().

/// `TMPFS_DIRSEQ_DOT`.
pub const TMPFS_DIRSEQ_DOT: u64 = 0;
/// `TMPFS_DIRSEQ_DOTDOT`.
pub const TMPFS_DIRSEQ_DOTDOT: u64 = 1;
/// `TMPFS_DIRSEQ_EOF`.
pub const TMPFS_DIRSEQ_EOF: u64 = 2;

/// `TMPFS_DIRSEQ_START`: inclusive.
pub const TMPFS_DIRSEQ_START: u64 = 3;
/// `TMPFS_DIRSEQ_END`: exclusive.
pub const TMPFS_DIRSEQ_END: u64 = u64::MAX;

/// `TMPFS_DIRSEQ_NONE`: mark to indicate that the number is not set.
pub const TMPFS_DIRSEQ_NONE: u64 = u64::MAX;

/// `TMPFS_DIRSEQ_FULL(dnode)`: can we still append entries to a directory?
pub fn tmpfs_dirseq_full<G: ProcGlue>(dnode: &TmpfsNode<G>) -> bool {
    dnode.dir().tn_next_seq.get() == TMPFS_DIRSEQ_END
}

// Status flags.

/// `TMPFS_NODE_ACCESSED`.
pub const TMPFS_NODE_ACCESSED: i32 = 0x01;
/// `TMPFS_NODE_MODIFIED`.
pub const TMPFS_NODE_MODIFIED: i32 = 0x02;
/// `TMPFS_NODE_CHANGED`.
pub const TMPFS_NODE_CHANGED: i32 = 0x04;

/// `TMPFS_NODE_STATUSALL`.
pub const TMPFS_NODE_STATUSALL: i32 =
    TMPFS_NODE_ACCESSED | TMPFS_NODE_MODIFIED | TMPFS_NODE_CHANGED;

/// `TMPFS_NODE_GEN_MASK`: bit indicating vnode reclamation. We abuse `tn_gen` for that.
pub const TMPFS_NODE_GEN_MASK: u64 = !0u64 >> 1;
/// `TMPFS_RECLAIMING_BIT`.
pub const TMPFS_RECLAIMING_BIT: u64 = !TMPFS_NODE_GEN_MASK;

/// `TMPFS_NODE_RECLAIMING(node)`.
pub fn tmpfs_node_reclaiming<G: ProcGlue>(node: &TmpfsNode<G>) -> bool {
    node.tn_gen.load(Ordering::Relaxed) & TMPFS_RECLAIMING_BIT != 0
}

/// `TMPFS_NODE_GEN(node)`.
pub fn tmpfs_node_gen<G: ProcGlue>(node: &TmpfsNode<G>) -> u64 {
    node.tn_gen.load(Ordering::Relaxed) & TMPFS_NODE_GEN_MASK
}

/// What `tm_acc_lock` protects: the accounting and `tm_highest_inode`.
#[derive(Debug, Default)]
pub struct TmAcc {
    /// `tm_mem_limit`: limit of bytes in use by the file system (0: the global limit).
    pub tm_mem_limit: u64,
    /// `tm_bytes_used`: number of bytes in use by the file system.
    pub tm_bytes_used: u64,
    /// `tm_highest_inode`: highest allocated inode number.
    pub tm_highest_inode: u64,
}

/// `struct tmpfs_mount` (`tmpfs_mount_t`): internal representation of a tmpfs mount point,
/// a counted `malloc(M_MISCFSMNT)` block hung from the mount's slot where the C mallocs it
/// (`tmpfs_mount`, before the root node: the root's allocation charges it).
pub struct TmpfsMount<G: ProcGlue> {
    /// `tm_acc_lock` (`"tacclk"`) and what it protects.
    pub tm_acc: Rwlocked<TmAcc>,
    /// `tm_root`: pointer to the root inode, set once right after the root is made.
    pub tm_root: OnceStatic<TmpfsNodeRef<G>>,
    /// `tm_nodes_max`: maximum number of possible nodes for this file system.
    pub tm_nodes_max: u32,
    /// `tm_nodes_cnt`: number of nodes currently allocated.
    pub tm_nodes_cnt: U32Word,
    /// `tm_lock` (`"tmplk"`): the lock protecting the list of inodes, its witness.
    pub tm_lock: RawRwlock,
    /// `tm_nodes`: list of inodes, each place a parked reference (the module's notes).
    pub tm_nodes: PinList<TmpfsNodeList<G>>,
}

impl<G: ProcGlue> TmpfsMount<G> {
    /// A mount of at most `nodes_max` nodes and `memlimit` bytes (0: the global limit), no
    /// node yet, `tm_highest_inode` 1 (`tmpfs_mount`'s fill and `tmpfs_mntmem_init`).
    pub fn new(nodes_max: u32, memlimit: u64) -> Self {
        Self {
            tm_acc: Rwlocked::new(
                "tacclk",
                TmAcc {
                    tm_mem_limit: memlimit,
                    tm_bytes_used: 0,
                    tm_highest_inode: 1,
                },
            ),
            tm_root: OnceStatic::new(),
            tm_nodes_max: nodes_max,
            tm_nodes_cnt: U32Word::new(0),
            tm_lock: RawRwlock::new("tmplk"),
            tm_nodes: PinList::new(),
        }
    }

    /// `tm_root`, which `tmpfs_mount` sets before the mount is used.
    pub fn root(&self) -> &TmpfsNodeRef<G> {
        match self.tm_root.get() {
            Some(root) => root,
            None => kpanic(format_args!("tmpfs mount {self:p}: no root")),
        }
    }
}

/// A counted handle of a mount's [`TmpfsMount`].
pub type TmpfsMountRef<G> = MArc<TmpfsMount<G>>;

/// `struct tmpfs_fid` (`tmpfs_fid_t`): maps a file identifier to a tmpfs node. Used by the
/// NFS code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TmpfsFid {
    /// `tf_len`: `sizeof(tmpfs_fid_t)`, over `fid_len`.
    pub tf_len: u16,
    /// `tf_pad`.
    pub tf_pad: u16,
    /// `tf_gen`: the node's `TMPFS_NODE_GEN`.
    pub tf_gen: u32,
    /// `tf_id`: the node's `tn_id`.
    pub tf_id: Ino,
}

impl TmpfsFid {
    /// `sizeof(tmpfs_fid_t)`.
    pub const SIZE: usize = 16;

    /// `memcpy(&tfh, fhp, sizeof(tmpfs_fid_t))`: the handle stored in a `struct fid`.
    pub fn from_fid(fid: &Fid) -> Self {
        let d = &fid.fid_data;
        Self {
            tf_len: fid.fid_len,
            tf_pad: fid.fid_reserved,
            tf_gen: u32::from_ne_bytes([d[0], d[1], d[2], d[3]]),
            tf_id: u64::from_ne_bytes([d[4], d[5], d[6], d[7], d[8], d[9], d[10], d[11]]),
        }
    }

    /// `memcpy(fhp, &tfh, sizeof(tfh))`: stores the handle over a `struct fid` (the bytes
    /// past the handle are left as they are, as in C).
    pub fn to_fid(&self, fid: &mut Fid) {
        fid.fid_len = self.tf_len;
        fid.fid_reserved = self.tf_pad;
        fid.fid_data[0..4].copy_from_slice(&self.tf_gen.to_ne_bytes());
        fid.fid_data[4..12].copy_from_slice(&self.tf_id.to_ne_bytes());
    }
}

/// `TMPFS_VALIDATE_DIR(node)`: ensures that the node pointed by `node` is a directory and
/// that its contents are consistent with respect to directories.
pub fn tmpfs_validate_dir<G: ProcGlue>(node: &TmpfsNode<G>) {
    kassert!(node.vnode().is_none_or(|vp| vop_islocked(vp) != 0));
    kassert!(node.tn_type == VDIR);
    kassert!(node.tn_size.get() % TMPFS_DIRENT_CHARGE as i64 == 0);
}

/// `VFS_TO_TMPFS(mp)`: the tmpfs mount of a mount, a counted handle (`panic(9)` on a mount
/// that has none: not a mounted tmpfs).
pub fn vfstotmpfs<G: ProcGlue>(mp: &Mount<G>) -> TmpfsMountRef<G> {
    match mp.node_arc::<TmpfsMount<G>>() {
        Some(tmp) => tmp,
        None => kpanic(format_args!(
            "VFS_TO_TMPFS: mount {mp:p} has no tmpfs_mount"
        )),
    }
}

/// `VP_TO_TMPFS_NODE(vp)`: the node of a tmpfs vnode, a counted handle (`panic(9)` on a
/// vnode that has none, LZ's `VT_TMPFS` check kept).
pub fn vptonode<G: ProcGlue>(vp: &Vnode<G>) -> TmpfsNodeRef<G> {
    match vp.node_with(|n: &TmpfsNode<G>| PoolArc::from_ref(n)) {
        Some(Some(node)) if vp.tag() == VT_TMPFS => node,
        _ => kpanic(format_args!(
            "VP_TO_TMPFS_NODE: vnode {vp:p} has no tmpfs node"
        )),
    }
}

/// `VP_TO_TMPFS_DIR(vp)`: the directory node of a tmpfs vnode.
pub fn vptodir<G: ProcGlue>(vp: &Vnode<G>) -> TmpfsNodeRef<G> {
    let node = vptonode(vp);
    tmpfs_validate_dir(&node);
    node
}

const _: () = {
    assert!(TMPFS_MAXNAMLEN < u16::MAX as usize);
    assert!(TmpfsFid::SIZE <= size_of::<Fid>());
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::fs::vnode::VREG;

    #[test]
    fn reclaiming_bit_is_outside_the_generation() {
        let node = TmpfsNode::<TestGlue>::new(VREG, TnSpec::None);
        node.tn_gen
            .store(TMPFS_NODE_GEN_MASK & 0x1234_5678, Ordering::Relaxed);
        assert!(!tmpfs_node_reclaiming(&node));
        node.tn_gen
            .fetch_or(TMPFS_RECLAIMING_BIT, Ordering::Relaxed);
        assert!(tmpfs_node_reclaiming(&node));
        assert_eq!(tmpfs_node_gen(&node), 0x1234_5678);
    }

    #[test]
    fn fid_round_trips_through_a_struct_fid() {
        let tfh = TmpfsFid {
            tf_len: TmpfsFid::SIZE as u16,
            tf_pad: 0,
            tf_gen: 0xdead_beef,
            tf_id: 42,
        };
        let mut fid = Fid::default();
        tfh.to_fid(&mut fid);
        assert_eq!(fid.fid_len, 16);
        assert_eq!(TmpfsFid::from_fid(&fid), tfh);
    }

    #[test]
    fn the_charges_are_the_cs_lp64_sizes() {
        // D-N5.9-6: sizeof(tmpfs_node_t) and sizeof(tmpfs_dirent_t) of arm64's (and amd64's)
        // C, pinned by _Static_asserts over the reference headers (N5.9t1), whatever the Rust
        // structures weigh.
        assert_eq!((TMPFS_NODE_CHARGE, TMPFS_DIRENT_CHARGE), (272, 48));
    }

    #[test]
    fn an_entry_compares_its_name_length_first_and_renames_in_place() {
        let (_g, _p) = super::super::testtmpfs::setup();
        let de = TmpfsDirent::<TestGlue>::new();
        assert!(!de.name_is(b""), "a fresh entry has no name");
        let mut b = MArray::new(127, hal::malloc::M_WAITOK, 32, |_| 0u8).expect("a block");
        b[..4].copy_from_slice(b"file");
        assert!(de.set_name(b, 4).is_none());
        assert!(de.name_is(b"file"));
        assert!(!de.name_is(b"fil"), "a shorter name with the same bytes");
        assert!(!de.name_is(b"filE"));
        de.rename_in_place(b"other");
        assert!(de.name_is(b"other"));
        de.with_name(|n| assert_eq!(n, b"other"));
        assert_eq!(de.take_name().map(|b| b.len()), Some(32));
        assert!(!de.name_is(b"other"));
    }
}
/* </TESTS> */
