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
 * Copyright (c) 1982, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)inode.h	8.5 (Berkeley) 7/8/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ufs/inode.h>`: the in-core inode. The inode is used to describe each active (or
//! recently active) file in the UFS filesystem. It is composed of two types of information.
//! The first part is the information that is needed only while the file is active (such as
//! the identity of the file and linkage to speed its lookup). The second part is the
//! permanent meta-data associated with the file which is read in from the permanent dinode
//! from long term storage when the file becomes active, and is put back when the file is no
//! longer being used.
//!
//! Upstream: sys/ufs/ufs/inode.h @ 3ce1f3f79392
//! LZ: sys/ufs/ufs/inode.rs@3c62ede68802
//!
//! An inode is an item of its file system's inode pool (`ffs_ino_pool`, `ext2fs_inode_pool`),
//! made by the pool as the file system's `vget` fills it, hung from its vnode's typed node
//! slot ([`Vnode::set_node`]) and reached as an [`InodeRef`], a counted handle: [`vtoi`]
//! counts one out of the slot, the file system's reclaim takes the vnode's own
//! ([`Vnode::take_node`]), and the last handle puts the item (and its dinode) back.
//!
//! ## Deviations
//! - `inode_u` (`i_fs` of FFS, `i_e2fs` of EXT2FS) and EXT2FS's `inode_ext` (`e2fs`) are
//!   members of `bsd`'s types (ffs's super block, ext2fs's): they are the glue's member
//!   [`Inode::g`] (`FileGlue::InodeG`, docs/SCAFFOLDING.md) until their owners move (`i_fs` at
//!   N5.5b, the ext2fs members at N5.6). `i_dquot[]` and UFS's `inode_ext.dirhash` are the
//!   inode's own since N5.5a2 ([`Inode::i_dquot`], [`Inode::i_dirhash`]). EXT2FS's dinode
//!   (`i_e2din`) is the glue member's too: the kernel's [`Dinode`] holds the FFS dinodes only,
//!   and a `DIP` of an ext2fs inode is `panic(9)` (the C's `DIP` reads FFS members there;
//!   nothing calls it so).
//! - `i_ump` is [`Inode::ump`], the mount's `struct ufsmount` as the file system's `vget` lends
//!   it (`bsd`'s `vfstoufs`, a `&'static` the unmount's `vflush` ends: no inode, and so no
//!   [`InodeRef`], outlives it; docs/SCAFFOLDING.md), beside [`Inode::mount`] and the copies
//!   of the words the UFS layer reads on every block (`um_fstype`, `um_nindir`,
//!   `um_bptrtodb`, `um_seqinc`), written once by `vget`, which never change after the mount;
//!   `i_devvp` (the C's macro for `i_ump->um_devvp`) is [`Inode::i_devvp`].
//! - The `DIP` macros select the UFS1 or UFS2 member by the dinode the inode holds, as the C
//!   does by `um_fstype` under `FFS2`; LZ's `ffs2` feature is `bsd`'s (ffs reads a UFS2 dinode
//!   only with it), the kernel reads either. A getter returns the wider of the two members'
//!   types (`DIP`'s conditional expression does the same); a setter truncates to the member's
//!   type, as the assignment does. `SHORTLINK(ip)` is [`Inode::with_shortlink`].
//! - The `struct inode_vtbl` calls (`UFS_TRUNCATE`, `UFS_UPDATE`, ...) are [`InodeOps`]'s
//!   methods (decision 46: the C's table as a trait); `iv_inode_alloc` returns the new vnode,
//!   `iv_buf_alloc` and `iv_bufatoff` the buffer the C passes back through `bpp`
//!   ([`IoBuf`]: the busy buffer's token, or a buffer `bsd` lent).
//! - `VTOI`/`ITOV`/`DOINGASYNC` are [`vtoi`], [`Inode::itov`] and [`doingasync`].
//! - `struct ufid` is [`Ufid`], converted to and from the `struct fid` it overlays.
//! - The non-`_KERNEL` userland compatibility names (`i_atime`, ...) are left out.
//! - `struct inode` is 440 bytes where LZ's was 304 (the inode hash's pin entry, the atomic
//!   words, the vnode link, the glue member; N5.5a2's `i_dirhash` cell and `i_ump`, 400 at
//!   N5.5a1), so the `ffsino` and `ext2inopl` pools' item size,
//!   `vmstat -m`'s and `systat pool`'s Size column, grows with it (plus the 16 bytes of a
//!   `TypedPool` item, sys/hal/pool.rs's deviation). No `diff-openbsd` scenario prints it; a
//!   pool's size follows its type (the standing rule), recorded here, not changed.
//!
//! ## Redesign
//! - LZ's `pool_get` of a raw `ffs_ino_pool` item written with `ptr::write`, hung from
//!   `v_data` and cast back by `vtoi`'s `unsafe` deref behind a tag check
//!   (sys/ufs/ufs/inode.rs:865-873, sys/ufs/ffs/ffs_vfsops.rs:1332-1338) -> a
//!   `hal::pool::TypedPool<Inode<G>>` item made by `init_recorded` (its [`PoolHome`] lets a
//!   `&Inode` be counted again), erased into the vnode's node slot, and [`InodeRef`] =
//!   `PoolArc<Inode<G>>` (D-N5.5a-3): [`vtoi`] reads the slot (the type's check replaces the
//!   tag's) and counts a handle, dropped where the C's pointer dies. Cost against the C's one
//!   load: the slot's read (two atomics), a `TypeId` compare, the count's increment and, at the
//!   drop, its decrement.
//! - LZ's `Cell` members under one `unsafe impl Sync` -> the C's words as relaxed atomics
//!   with LZ's `get`/`set` ([`U32Word`], [`I32Word`], [`U64Word`], [`CiCell`]; docs/IDIOMS.md,
//!   the counters' row): the C changes them under the vnode's lock and the kernel lock and
//!   reads some unlocked (`ufs_ihashget`'s `i_number`), as the vnode's `[K]` words of N5.2.
//!   Every writer holds the kernel lock (OpenBSD runs every ufs path `KERNEL_LOCK`ed:
//!   vfs_vnops.c's `vn_read`/`vn_write`/`vn_ioctl`, uvm_vnode.c's pager; the native
//!   kernel::fs::vnops and kernel::vm::vnode the same), which orders the writes: a change is
//!   a load and a store, as the C's `ip->i_flag |= f`, and the setters check the lock under
//!   `diagnostic` (`hal::klock::kernel_assert_locked`).
//!   Not D-N5.5a-4's `Guarded::new_kernel` cell: `klock::witness()` raises the interrupt
//!   level (sys/hal/klock.rs:432-447), one spl pair per change where the C has none (on every
//!   `read`'s `IN_ACCESS`, every lookup's `i_diroff`); an atomic word needs no witness, and
//!   its loads and stores are the C's plain ones. The identity (`i_dev`, `i_number`) is plain fields,
//!   set when the pool makes the item; `i_vnode` a `Link` (`ffs_vinit` moves the inode to the
//!   alias of a device vnode).
//! - LZ's `dinode_u` (a raw pointer to a pool dinode, read and written through `unsafe`
//!   `&mut` borrows in `with_din1`/`with_din2`/`with_shortlink`) -> [`Dinode`], the FFS dinode
//!   as `DinWords<N>` (N atomic words, the C's 128 or 256 bytes, an item of the file system's
//!   `dino1pl`/`dino2pl` `BoxPool`) in a set-once cell the file system's `vget` fills: each
//!   `DIP` reads or writes its member's bytes at the on-disk structure's `offset_of!` (a
//!   change is a load and a store of its word, the neighbour members kept, by a writer that
//!   holds the kernel lock as every word's), and the whole dinode is copied in and out as
//!   bytes (`ffs_vget`, `ffs_update`).
//! - LZ's `dinode1_at`/`set_dinode1_at` and their UFS2 twins (`unsafe` unaligned reads and
//!   writes) -> `pod_from_bytes`/`pod_to_bytes` of the `hal::abi_pod!` dinodes.

use core::mem::offset_of;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};

use hal::link::Link;
use hal::malloc::MBox;
use hal::once::OnceStatic;
use hal::pinlist::PinEntry;
use hal::pool::{PoolArc, PoolBox, PoolHome};
use hal::rwlock::RecursiveRwlock;
use hal::slot::ReadMostly;
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, MNT_ASYNC};
use hal::sys::types::{Daddr, Dev, Mode, Off};
use hal::user::{AbiPod, bytes_of, bytes_of_mut, pod_from_bytes, pod_to_bytes};

use super::dinode::{
    MAXSYMLINKLEN_UFS1, MAXSYMLINKLEN_UFS2, NDADDR, NIADDR, Ufs1Dinode, Ufs2Dinode, Ufsino,
};
use super::dirhash::Dirhash;
use super::quota::MAXQUOTAS;
use super::ufs_quota::Dquot;
use super::ufsmount::{UM_UFS1, Ufsmount};
use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::fs::buf::ClusterInfo;
use crate::fs::lockf::LockfSlot;
use crate::fs::mount::Mount;
use crate::fs::vnode::Vnode;
use crate::fs::vops::{Cred, IoBuf};
use crate::proc::ProcGlue;

/// Generates a word of the inode or of its mount: a relaxed atomic read and written whole by
/// `get()`/`set()`, as LZ's `Cell`.
macro_rules! word {
    ($(#[$doc:meta])* $name:ident, $t:ty, $atomic:ty) => {
        $(#[$doc])*
        #[derive(Debug, Default)]
        pub struct $name($atomic);

        impl $name {
            /// `v`.
            pub const fn new(v: $t) -> Self {
                Self(<$atomic>::new(v))
            }

            /// The value.
            #[inline]
            pub fn get(&self) -> $t {
                self.0.load(Ordering::Relaxed)
            }

            /// Stores `v`: by a holder of the kernel lock (checked under `diagnostic`).
            #[inline]
            pub fn set(&self, v: $t) {
                #[cfg(feature = "diagnostic")]
                hal::klock::kernel_assert_locked();
                self.0.store(v, Ordering::Relaxed);
            }
        }
    };
}

word!(
    /// A `u_int32_t` word (`i_flag`, `i_ino`, `i_reclen`, `um_maxsymlinklen`).
    U32Word, u32, AtomicU32
);
word!(
    /// An `int32_t` word (`i_effnlink`, the directory offsets, `um_dev`).
    I32Word, i32, AtomicI32
);
word!(
    /// A `u_int64_t` word (`i_modrev`, the `um_*` words of the mount).
    U64Word, u64, AtomicU64
);

/// `i_ci`: the inode's `struct cluster_info`, each member a relaxed atomic, read and written
/// whole by `get()`/`set()` as LZ's `Cell<ClusterInfo>` (the read-ahead's state, changed under
/// the vnode's lock).
#[derive(Debug, Default)]
pub struct CiCell {
    lastr: AtomicI64,
    lastw: AtomicI64,
    cstart: AtomicI64,
    lasta: AtomicI64,
    clen: AtomicI32,
    ralen: AtomicI32,
    maxra: AtomicI64,
}

impl CiCell {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            lastr: AtomicI64::new(0),
            lastw: AtomicI64::new(0),
            cstart: AtomicI64::new(0),
            lasta: AtomicI64::new(0),
            clen: AtomicI32::new(0),
            ralen: AtomicI32::new(0),
            maxra: AtomicI64::new(0),
        }
    }

    /// `ip->i_ci`.
    pub fn get(&self) -> ClusterInfo {
        let r = Ordering::Relaxed;
        ClusterInfo {
            ci_lastr: self.lastr.load(r),
            ci_lastw: self.lastw.load(r),
            ci_cstart: self.cstart.load(r),
            ci_lasta: self.lasta.load(r),
            ci_clen: self.clen.load(r),
            ci_ralen: self.ralen.load(r),
            ci_maxra: self.maxra.load(r),
        }
    }

    /// `ip->i_ci = ci`, by a holder of the kernel lock (checked under `diagnostic`).
    pub fn set(&self, ci: ClusterInfo) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        let r = Ordering::Relaxed;
        self.lastr.store(ci.ci_lastr, r);
        self.lastw.store(ci.ci_lastw, r);
        self.cstart.store(ci.ci_cstart, r);
        self.lasta.store(ci.ci_lasta, r);
        self.clen.store(ci.ci_clen, r);
        self.ralen.store(ci.ci_ralen, r);
        self.maxra.store(ci.ci_maxra, r);
    }
}

/// A member of a dinode as [`DinWords`] reads and writes it: its width and its bits in a
/// native-endian `u64` word.
pub trait DinField: Copy {
    /// The member's size in bytes (2, 4 or 8).
    const SIZE: usize;
    /// The member from its bits (the low `SIZE * 8` of `w`).
    fn from_bits(w: u64) -> Self;
    /// The member's bits, in the low `SIZE * 8` of a word.
    fn to_bits(self) -> u64;
}

macro_rules! din_field {
    ($($t:ty => $u:ty),+) => {
        $(
            impl DinField for $t {
                const SIZE: usize = size_of::<$t>();
                #[inline]
                fn from_bits(w: u64) -> Self {
                    w as $u as $t
                }
                #[inline]
                fn to_bits(self) -> u64 {
                    u64::from(self as $u)
                }
            }
        )+
    };
}

din_field!(u16 => u16, i16 => u16, u32 => u32, i32 => u32, u64 => u64, i64 => u64);

/// The in-core copy of an FFS dinode (`*ip->i_din1`, `*ip->i_din2`): the on-disk structure's
/// `N * 8` bytes as `N` relaxed atomic words, so the inode's readers share it with its writers
/// (holders of the kernel lock, which orders them: [`Inode`]'s locks) without a lock of its
/// own. A member is read and written at the on-disk structure's offset ([`DinWords::get`],
/// [`DinWords::set`]); the whole is copied as words.
pub struct DinWords<const N: usize> {
    w: [AtomicU64; N],
}

/// `struct ufs1_dinode` in core: 16 words, 128 bytes (`dino1pl`'s items).
pub type Ufs1Din = DinWords<16>;
/// `struct ufs2_dinode` in core: 32 words, 256 bytes (`dino2pl`'s items).
pub type Ufs2Din = DinWords<32>;

impl<const N: usize> DinWords<N> {
    /// All zero.
    pub const fn zeroed() -> Self {
        Self {
            w: [const { AtomicU64::new(0) }; N],
        }
    }

    /// The shift of a member of `size` bytes at byte `off` of its word, in bits.
    #[inline]
    fn shift(off: usize, size: usize) -> u32 {
        let b = off % 8;
        let s = if cfg!(target_endian = "little") {
            b * 8
        } else {
            (8 - b - size) * 8
        };
        s as u32
    }

    /// The word holding the member at `off` of `size` bytes: `panic(9)` for one outside the
    /// dinode or across two words (no member of the C's structures is either).
    #[inline]
    fn word(&self, off: usize, size: usize) -> &AtomicU64 {
        if !off.is_multiple_of(size) || off + size > N * 8 {
            kpanic(format_args!("dinode: member at {off} of {size} bytes"));
        }
        &self.w[off / 8]
    }

    /// The member at byte `off` (an `offset_of!` of the on-disk structure).
    #[inline]
    pub fn get<T: DinField>(&self, off: usize) -> T {
        let w = self.word(off, T::SIZE).load(Ordering::Relaxed);
        T::from_bits(w >> Self::shift(off, T::SIZE))
    }

    /// The member at byte `off` set to `v`, the word's other members kept: a load and a
    /// store of the word, by a holder of the kernel lock (checked under `diagnostic`), whose
    /// writes it orders.
    pub fn set<T: DinField>(&self, off: usize, v: T) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        let s = Self::shift(off, T::SIZE);
        let mask = if T::SIZE == 8 {
            u64::MAX
        } else {
            ((1u64 << (T::SIZE * 8)) - 1) << s
        };
        let bits = (v.to_bits() << s) & mask;
        let w = self.word(off, T::SIZE);
        w.store(
            (w.load(Ordering::Relaxed) & !mask) | bits,
            Ordering::Relaxed,
        );
    }

    /// The dinode's words (one relaxed load each).
    #[inline]
    fn words(&self) -> [u64; N] {
        core::array::from_fn(|i| self.w[i].load(Ordering::Relaxed))
    }

    /// The dinode's words set to `words` (one relaxed store each), by a holder of the
    /// kernel lock (checked under `diagnostic`).
    #[inline]
    fn set_words(&self, words: [u64; N]) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        self.put_words(words);
    }

    /// The words of a dinode no one else can see yet (`new`'s): stored without the lock's
    /// check.
    #[inline]
    fn put_words(&self, words: [u64; N]) {
        for (w, v) in self.w.iter().zip(words) {
            w.store(v, Ordering::Relaxed);
        }
    }

    /// The structure `T` the words hold, read whole (one relaxed load per word): a `T` of
    /// `N * 8` bytes (a dinode, quota.h's `dqblk`).
    pub fn load_pod<T: AbiPod + Default>(&self) -> T {
        Self::check_size::<T>();
        pod_from_bytes(bytes_of(&self.words())).unwrap_or_default()
    }

    /// The words set to `d`'s bytes (one relaxed store per word), by a holder of the kernel
    /// lock (checked under `diagnostic`).
    pub fn store_pod<T: AbiPod>(&self, d: &T) {
        Self::check_size::<T>();
        let mut words = [0u64; N];
        let _ = pod_to_bytes(d, bytes_of_mut(&mut words));
        self.set_words(words);
    }

    /// `panic(9)` unless `T` is the words' `N * 8` bytes.
    fn check_size<T>() {
        if size_of::<T>() != N * 8 {
            kpanic(format_args!(
                "DinWords<{N}>: a structure of {} bytes",
                size_of::<T>()
            ));
        }
    }

    /// The bytes `off..off + out.len()` of the dinode into `out`.
    pub fn read_bytes(&self, off: usize, out: &mut [u8]) {
        if off + out.len() > N * 8 {
            kpanic(format_args!(
                "dinode: bytes {off}+{} past the end",
                out.len()
            ));
        }
        if off.is_multiple_of(8) && out.len().is_multiple_of(8) {
            // Whole words (the dinode copied whole, a block pointer array): one fixed-size
            // store per word.
            for (dst, w) in out
                .as_chunks_mut::<8>()
                .0
                .iter_mut()
                .zip(&self.w[off / 8..])
            {
                *dst = w.load(Ordering::Relaxed).to_ne_bytes();
            }
            return;
        }
        let mut i = 0;
        while i < out.len() {
            let at = off + i;
            let first = at % 8;
            let n = (8 - first).min(out.len() - i);
            let w = self.w[at / 8].load(Ordering::Relaxed).to_ne_bytes();
            out[i..i + n].copy_from_slice(&w[first..first + n]);
            i += n;
        }
    }

    /// The bytes `off..off + src.len()` of the dinode set to `src`; the bytes of the first and
    /// last words outside the range are kept (a load and a store of each, by a holder of the
    /// kernel lock, checked under `diagnostic`).
    pub fn write_bytes(&self, off: usize, src: &[u8]) {
        #[cfg(feature = "diagnostic")]
        hal::klock::kernel_assert_locked();
        if off + src.len() > N * 8 {
            kpanic(format_args!(
                "dinode: bytes {off}+{} past the end",
                src.len()
            ));
        }
        if off.is_multiple_of(8) && src.len().is_multiple_of(8) {
            // Whole words: no neighbour to keep, one store per word.
            for (w, part) in self.w[off / 8..].iter().zip(src.as_chunks::<8>().0) {
                w.store(u64::from_ne_bytes(*part), Ordering::Relaxed);
            }
            return;
        }
        let mut i = 0;
        while i < src.len() {
            let at = off + i;
            let wi = at / 8;
            let first = at % 8;
            let n = (8 - first).min(src.len() - i);
            let part = &src[i..i + n];
            if n == 8 {
                // A whole word: no neighbour to keep.
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(part);
                self.w[wi].store(u64::from_ne_bytes(bytes), Ordering::Relaxed);
                i += n;
                continue;
            }
            let mut bytes = self.w[wi].load(Ordering::Relaxed).to_ne_bytes();
            bytes[first..first + n].copy_from_slice(part);
            self.w[wi].store(u64::from_ne_bytes(bytes), Ordering::Relaxed);
            i += n;
        }
    }
}

impl DinWords<16> {
    /// The in-core copy of `d`: a value no one else sees yet, so its words are stored
    /// without the kernel lock's check.
    pub fn new(d: &Ufs1Dinode) -> Self {
        let w = Self::zeroed();
        w.put_words(Self::words_of(d));
        w
    }

    /// `*ip->i_din1`: a copy of the whole dinode.
    pub fn load(&self) -> Ufs1Dinode {
        pod_from_bytes(bytes_of(&self.words())).unwrap_or_default()
    }

    /// `*ip->i_din1 = *d`.
    pub fn store(&self, d: &Ufs1Dinode) {
        self.set_words(Self::words_of(d));
    }

    /// The words of `d`'s bytes.
    fn words_of(d: &Ufs1Dinode) -> [u64; 16] {
        let mut words = [0u64; 16];
        let _ = pod_to_bytes(d, bytes_of_mut(&mut words));
        words
    }
}

impl DinWords<32> {
    /// The in-core copy of `d`: a value no one else sees yet, so its words are stored
    /// without the kernel lock's check.
    pub fn new(d: &Ufs2Dinode) -> Self {
        let w = Self::zeroed();
        w.put_words(Self::words_of(d));
        w
    }

    /// `*ip->i_din2`: a copy of the whole dinode.
    pub fn load(&self) -> Ufs2Dinode {
        pod_from_bytes(bytes_of(&self.words())).unwrap_or_default()
    }

    /// `*ip->i_din2 = *d`.
    pub fn store(&self, d: &Ufs2Dinode) {
        self.set_words(Self::words_of(d));
    }

    /// The words of `d`'s bytes.
    fn words_of(d: &Ufs2Dinode) -> [u64; 32] {
        let mut words = [0u64; 32];
        let _ = pod_to_bytes(d, bytes_of_mut(&mut words));
        words
    }
}

/// `dinode_u`: the FFS dinode an inode holds, an item of the file system's dinode pool,
/// given back with the inode.
pub enum Dinode {
    /// `i_din1`: an FFS1 dinode.
    Ufs1(PoolBox<Ufs1Din>),
    /// `i_din2`: an FFS2 dinode.
    Ufs2(PoolBox<Ufs2Din>),
}

/// What [`Inode::new`] takes: the inode's identity and the mount's words, which never change
/// once the file system's `vget` has filled them in.
pub struct InodeInit<G: ProcGlue> {
    /// `i_vnode`.
    pub vnode: Option<&'static Vnode<G>>,
    /// `i_ump->um_mountp`.
    pub mount: Option<&'static Mount<G>>,
    /// `i_ump`.
    pub ump: Option<&'static Ufsmount<G>>,
    /// `i_devvp` (`i_ump->um_devvp`).
    pub devvp: Option<&'static Vnode<G>>,
    /// `i_dev`.
    pub dev: Dev,
    /// `i_number`.
    pub number: Ufsino,
    /// `i_ump->um_fstype`.
    pub fstype: u64,
    /// `i_ump->um_nindir`.
    pub nindir: u64,
    /// `i_ump->um_bptrtodb`.
    pub bptrtodb: u64,
    /// `i_ump->um_seqinc`.
    pub seqinc: u64,
    /// `i_vtbl`.
    pub vtbl: Option<&'static dyn InodeOps<G>>,
    /// The members of `bsd`'s types (the glue's).
    pub g: <G::FileGlue as FileGlue>::InodeG,
}

impl<G: ProcGlue> Default for InodeInit<G> {
    fn default() -> Self {
        Self {
            vnode: None,
            mount: None,
            ump: None,
            devvp: None,
            dev: 0,
            number: 0,
            fstype: 0,
            nindir: 0,
            bptrtodb: 0,
            seqinc: 0,
            vtbl: None,
            g: Default::default(),
        }
    }
}

/// `struct inode`.
///
/// Locks: the identity and the mount's words are set when the item is made and never change;
/// the words the C changes under the inode's lock (`i_lock`, held through its vnode) and the
/// kernel lock are relaxed atomics, the dinode's ([`DinWords`]) among them; `i_hash` is the
/// inode hash's (the kernel lock). Every writer of a word holds the kernel lock, as every
/// ufs path in OpenBSD runs `KERNEL_LOCK`ed; that lock orders the writes, so a change is a
/// load and a store ([`Inode::set_flag`], [`DinWords::set`]) and the setters check it under
/// `diagnostic`. Readers may not hold it: a relaxed load returns a whole word, one a writer
/// stored.
pub struct Inode<G: ProcGlue> {
    /// `i_hash`: hash chain.
    pub i_hash: PinEntry<Inode<G>>,
    /// `i_vnode`: vnode associated with this inode.
    pub i_vnode: Link<Vnode<G>>,
    /// `i_ump->um_mountp`.
    mount: Option<&'static Mount<G>>,
    /// `i_ump`: the mount's UFS structure (lent by the file system's `vget` for the inode's
    /// life: every inode of a mount is reclaimed before its unmount frees the structure).
    ump: Option<&'static Ufsmount<G>>,
    /// `i_devvp`: the mount's device vnode.
    devvp: Option<&'static Vnode<G>>,
    /// `i_flag`: flags, see below.
    pub i_flag: U32Word,
    /// `i_dev`: device associated with the inode.
    pub i_dev: Dev,
    /// `i_number`: the identity of the inode.
    pub i_number: Ufsino,
    /// `i_effnlink`: `i_nlink` when I/O completes.
    pub i_effnlink: I32Word,
    /// `i_ump->um_fstype`.
    fstype: u64,
    /// `i_ump->um_nindir`: indirect pointers per block.
    nindir: u64,
    /// `i_ump->um_bptrtodb`: indirect pointer to disk block shift.
    bptrtodb: u64,
    /// `i_ump->um_seqinc`: increment between sequential blocks.
    seqinc: u64,
    /// `i_ci`.
    pub i_ci: CiCell,
    /// `i_modrev`: revision level for NFS lease.
    pub i_modrev: U64Word,
    /// `i_lockf`: byte-level lock state.
    pub i_lockf: LockfSlot,
    /// `i_lock`: inode lock.
    pub i_lock: RecursiveRwlock,
    /// `i_dquot`: dquot structures (`ufs_quota.rs`; none without `QUOTA`).
    pub i_dquot: [Link<Dquot<G>>; MAXQUOTAS],
    /// `i_dirhash` (`inode_ext.dirhash`): hashing for large directories (`ufs_dirhash.rs`),
    /// written by `ufsdirhash_build` and `ufsdirhash_free` only, under the inode's lock.
    pub i_dirhash: ReadMostly<Option<MBox<Dirhash>>>,
    // Side effects; used during directory lookup.
    /// `i_count`: size of free slot in directory.
    pub i_count: I32Word,
    /// `i_endoff`: end of useful stuff in directory.
    pub i_endoff: I32Word,
    /// `i_diroff`: offset in dir, where we found last entry.
    pub i_diroff: I32Word,
    /// `i_offset`: offset of free space in directory.
    pub i_offset: I32Word,
    /// `i_ino`: inode number of found directory.
    pub i_ino: U32Word,
    /// `i_reclen`: size of found directory entry.
    pub i_reclen: U32Word,
    /// `dinode_u`: the FFS dinode, set once by the file system's `vget`.
    din: OnceStatic<Dinode>,
    /// `i_vtbl`.
    vtbl: Option<&'static dyn InodeOps<G>>,
    /// Where the pool placed the inode ([`vtoi`]'s `PoolArc::from_ref`).
    home: PoolHome<Inode<G>>,
    /// The members of `bsd`'s types (`inode_u`, `inode_ext`, `i_dquot`; the module's
    /// deviations).
    pub g: <G::FileGlue as FileGlue>::InodeG,
}

hal::pool_home!(impl[G: ProcGlue] Inode<G>, home);

/// `struct inode *` held: a counted handle to an inode, which stays in its pool item until
/// the last handle drops.
pub type InodeRef<G> = PoolArc<Inode<G>>;

/// Generates a `DIP(ip, field)` getter and a `DIP_ASSIGN(ip, field, v)` setter over the
/// members of both FFS dinodes, of types `$t1` and `$t2`.
macro_rules! dip_field {
    ($(#[$doc:meta])* $get:ident, $set:ident, $t:ty, $field:ident: $t1:ty, $t2:ty) => {
        $(#[$doc])*
        #[inline]
        pub fn $get(&self) -> $t {
            match self.din() {
                Dinode::Ufs1(d) => d.get::<$t1>(offset_of!(Ufs1Dinode, $field)) as $t,
                Dinode::Ufs2(d) => d.get::<$t2>(offset_of!(Ufs2Dinode, $field)) as $t,
            }
        }

        $(#[$doc])*
        #[inline]
        pub fn $set(&self, v: $t) {
            match self.din() {
                Dinode::Ufs1(d) => d.set::<$t1>(offset_of!(Ufs1Dinode, $field), v as $t1),
                Dinode::Ufs2(d) => d.set::<$t2>(offset_of!(Ufs2Dinode, $field), v as $t2),
            }
        }
    };
}

impl<G: ProcGlue> Inode<G> {
    /// A new inode, as `pool_get(&ffs_ino_pool, PR_ZERO)` and the file system's `vget` fill it:
    /// no dinode yet ([`Inode::set_din`]), unhashed, unlocked.
    pub fn new(init: InodeInit<G>) -> Self {
        Self {
            i_hash: PinEntry::new(),
            i_vnode: {
                let l = Link::new();
                l.set(init.vnode);
                l
            },
            mount: init.mount,
            ump: init.ump,
            devvp: init.devvp,
            i_flag: U32Word::new(0),
            i_dev: init.dev,
            i_number: init.number,
            i_effnlink: I32Word::new(0),
            fstype: init.fstype,
            nindir: init.nindir,
            bptrtodb: init.bptrtodb,
            seqinc: init.seqinc,
            i_ci: CiCell::new(),
            i_modrev: U64Word::new(0),
            i_lockf: LockfSlot::new(),
            i_lock: RecursiveRwlock::new("inode"),
            i_dquot: [const { Link::new() }; MAXQUOTAS],
            i_dirhash: ReadMostly::new(None),
            i_count: I32Word::new(0),
            i_endoff: I32Word::new(0),
            i_diroff: I32Word::new(0),
            i_offset: I32Word::new(0),
            i_ino: U32Word::new(0),
            i_reclen: U32Word::new(0),
            din: OnceStatic::new(),
            vtbl: init.vtbl,
            home: PoolHome::new(),
            g: init.g,
        }
    }

    /// `ITOV(ip)`: the inode's vnode.
    pub fn itov(&self) -> &'static Vnode<G> {
        match self.i_vnode.get() {
            Some(vp) => vp,
            None => kpanic(format_args!("inode {self:p}: no vnode")),
        }
    }

    /// `ip->i_ump`: the mount's UFS structure, which every inode of a mounted UFS has.
    pub fn ump(&self) -> &'static Ufsmount<G> {
        match self.ump {
            Some(u) => u,
            None => kpanic(format_args!("inode {self:p}: no ufsmount")),
        }
    }

    /// `ip->i_ump->um_mountp`: the mount the inode belongs to.
    pub fn mount(&self) -> &'static Mount<G> {
        match self.mount {
            Some(mp) => mp,
            None => kpanic(format_args!("inode {self:p}: no mount")),
        }
    }

    /// `ip->i_devvp` (`i_ump->um_devvp`).
    pub fn i_devvp(&self) -> &'static Vnode<G> {
        match self.devvp {
            Some(vp) => vp,
            None => kpanic(format_args!("inode {self:p}: no devvp")),
        }
    }

    /// `ip->i_ump->um_fstype`.
    #[inline]
    pub fn fstype(&self) -> u64 {
        self.fstype
    }

    /// `MNINDIR(ip->i_ump)`: indirect pointers per block.
    #[inline]
    pub fn nindir(&self) -> u64 {
        self.nindir
    }

    /// `blkptrtodb(ip->i_ump, b)`: a block pointer to a disk block.
    #[inline]
    pub fn blkptrtodb(&self, b: Daddr) -> Daddr {
        b << self.bptrtodb
    }

    /// `is_sequential(ip->i_ump, a, b)`: whether `b` follows `a` on the disk.
    #[inline]
    pub fn is_sequential(&self, a: Daddr, b: Daddr) -> bool {
        b == a + self.seqinc as Daddr
    }

    /// `ip->i_vtbl`.
    pub fn vtbl(&self) -> &'static dyn InodeOps<G> {
        match self.vtbl {
            Some(v) => v,
            None => kpanic(format_args!("inode {self:p}: no vtbl")),
        }
    }

    /// `dinode_u = d`: the file system's `vget` gives the inode its dinode, once.
    pub fn set_din(&self, d: Dinode) {
        if self.din.set(d).is_err() {
            kpanic(format_args!("inode {self:p}: a second dinode"));
        }
    }

    /// `ip->i_din1 == NULL`: the inode has no dinode yet (or never will: an ext2fs inode's is
    /// the glue's member).
    pub fn din_is_null(&self) -> bool {
        !self.din.is_set()
    }

    /// The dinode: `panic(9)` without one, where the C would follow its NULL `i_din1`.
    #[inline]
    pub fn din(&self) -> &Dinode {
        match self.din.get() {
            Some(d) => d,
            None => kpanic(format_args!("inode {self:p}: no dinode")),
        }
    }

    /// The UFS1 dinode in core (`ip->i_din1`): `panic(9)` for another.
    pub fn ufs1_din(&self) -> &Ufs1Din {
        match self.din() {
            Dinode::Ufs1(d) => d,
            Dinode::Ufs2(_) => kpanic(format_args!("inode {self:p}: not UFS1")),
        }
    }

    /// The UFS2 dinode in core (`ip->i_din2`): `panic(9)` for another.
    pub fn ufs2_din(&self) -> &Ufs2Din {
        match self.din() {
            Dinode::Ufs2(d) => d,
            Dinode::Ufs1(_) => kpanic(format_args!("inode {self:p}: not UFS2")),
        }
    }

    /// `*ip->i_din1`: a copy of the UFS1 dinode.
    pub fn din1(&self) -> Ufs1Dinode {
        self.ufs1_din().load()
    }

    /// `*ip->i_din2`: a copy of the UFS2 dinode.
    pub fn din2(&self) -> Ufs2Dinode {
        self.ufs2_din().load()
    }

    /// Whether the inode's dinode is UFS2 (`i_ump->um_fstype == UM_UFS2`).
    pub fn is_ufs2(&self) -> bool {
        matches!(self.din(), Dinode::Ufs2(_))
    }

    dip_field!(
        /// `DIP(ip, mode)`.
        dip_mode, dip_set_mode, Mode, di_mode: u16, u16
    );
    dip_field!(
        /// `DIP(ip, nlink)`.
        dip_nlink, dip_set_nlink, i32, di_nlink: i16, i16
    );
    dip_field!(
        /// `DIP(ip, uid)`.
        dip_uid, dip_set_uid, u32, di_uid: u32, u32
    );
    dip_field!(
        /// `DIP(ip, gid)`.
        dip_gid, dip_set_gid, u32, di_gid: u32, u32
    );
    dip_field!(
        /// `DIP(ip, size)`.
        dip_size, dip_set_size, u64, di_size: u64, u64
    );
    dip_field!(
        /// `DIP(ip, blocks)`.
        dip_blocks, dip_set_blocks, i64, di_blocks: i32, u64
    );
    dip_field!(
        /// `DIP(ip, atime)`.
        dip_atime, dip_set_atime, i64, di_atime: i32, i64
    );
    dip_field!(
        /// `DIP(ip, atimensec)`.
        dip_atimensec, dip_set_atimensec, i32, di_atimensec: i32, i32
    );
    dip_field!(
        /// `DIP(ip, mtime)`.
        dip_mtime, dip_set_mtime, i64, di_mtime: i32, i64
    );
    dip_field!(
        /// `DIP(ip, mtimensec)`.
        dip_mtimensec, dip_set_mtimensec, i32, di_mtimensec: i32, i32
    );
    dip_field!(
        /// `DIP(ip, ctime)`.
        dip_ctime, dip_set_ctime, i64, di_ctime: i32, i64
    );
    dip_field!(
        /// `DIP(ip, ctimensec)`.
        dip_ctimensec, dip_set_ctimensec, i32, di_ctimensec: i32, i32
    );
    dip_field!(
        /// `DIP(ip, flags)`.
        dip_flags, dip_set_flags, u32, di_flags: u32, u32
    );
    dip_field!(
        /// `DIP(ip, gen)`.
        dip_gen, dip_set_gen, u32, di_gen: u32, i32
    );

    /// The byte offset of `di_db[i]` (`array`: `di_db` or `di_ib`'s offset, `n` its length) in
    /// a dinode whose block pointers are `size` bytes: `panic(9)` past the array, as LZ's index.
    fn ptr_off(array: usize, n: usize, size: usize, i: usize) -> usize {
        if i >= n {
            kpanic(format_args!("inode: block pointer {i} of {n}"));
        }
        array + i * size
    }

    /// `DIP(ip, db[i])`.
    pub fn dip_db(&self, i: usize) -> Daddr {
        match self.din() {
            Dinode::Ufs1(d) => Daddr::from(d.get::<i32>(Self::ptr_off(
                offset_of!(Ufs1Dinode, di_db),
                NDADDR,
                4,
                i,
            ))),
            Dinode::Ufs2(d) => {
                d.get::<i64>(Self::ptr_off(offset_of!(Ufs2Dinode, di_db), NDADDR, 8, i))
            }
        }
    }

    /// `DIP_ASSIGN(ip, db[i], v)`.
    pub fn dip_set_db(&self, i: usize, v: Daddr) {
        match self.din() {
            Dinode::Ufs1(d) => {
                d.set::<i32>(
                    Self::ptr_off(offset_of!(Ufs1Dinode, di_db), NDADDR, 4, i),
                    v as i32,
                );
            }
            Dinode::Ufs2(d) => d.set::<i64>(
                Self::ptr_off(offset_of!(Ufs2Dinode, di_db), NDADDR, 8, i),
                v,
            ),
        }
    }

    /// `DIP(ip, ib[i])`.
    pub fn dip_ib(&self, i: usize) -> Daddr {
        match self.din() {
            Dinode::Ufs1(d) => Daddr::from(d.get::<i32>(Self::ptr_off(
                offset_of!(Ufs1Dinode, di_ib),
                NIADDR,
                4,
                i,
            ))),
            Dinode::Ufs2(d) => {
                d.get::<i64>(Self::ptr_off(offset_of!(Ufs2Dinode, di_ib), NIADDR, 8, i))
            }
        }
    }

    /// `DIP_ASSIGN(ip, ib[i], v)`.
    pub fn dip_set_ib(&self, i: usize, v: Daddr) {
        match self.din() {
            Dinode::Ufs1(d) => {
                d.set::<i32>(
                    Self::ptr_off(offset_of!(Ufs1Dinode, di_ib), NIADDR, 4, i),
                    v as i32,
                );
            }
            Dinode::Ufs2(d) => d.set::<i64>(
                Self::ptr_off(offset_of!(Ufs2Dinode, di_ib), NIADDR, 8, i),
                v,
            ),
        }
    }

    /// `DIP(ip, rdev)` (`di_db[0]`).
    pub fn dip_rdev(&self) -> i64 {
        self.dip_db(0)
    }

    /// `DIP_ASSIGN(ip, rdev, v)`.
    pub fn dip_set_rdev(&self, v: i64) {
        self.dip_set_db(0, v);
    }

    /// `MAXSYMLINKLEN(ip)`: the longest symbolic link kept in the dinode itself.
    pub fn maxsymlinklen(&self) -> usize {
        if self.fstype == UM_UFS1 {
            MAXSYMLINKLEN_UFS1
        } else {
            MAXSYMLINKLEN_UFS2
        }
    }

    /// `f(SHORTLINK(ip))`: the bytes of `di_db` and `di_ib`, where a short symbolic link
    /// keeps its target, for the duration of `f`: copied out of the dinode, and back after
    /// `f` only when `f` changed them (`ufs_readlink` only reads them; a writer holds the
    /// kernel lock, as every word's).
    pub fn with_shortlink<R>(&self, f: impl FnOnce(&mut [u8]) -> R) -> R {
        let mut buf = [0u8; MAXSYMLINKLEN_UFS2];
        let mut was = [0u8; MAXSYMLINKLEN_UFS2];
        let (off, n) = match self.din() {
            Dinode::Ufs1(_) => (offset_of!(Ufs1Dinode, di_db), MAXSYMLINKLEN_UFS1),
            Dinode::Ufs2(_) => (offset_of!(Ufs2Dinode, di_db), MAXSYMLINKLEN_UFS2),
        };
        let s = &mut buf[..n];
        match self.din() {
            Dinode::Ufs1(d) => d.read_bytes(off, s),
            Dinode::Ufs2(d) => d.read_bytes(off, s),
        }
        was[..n].copy_from_slice(s);
        let r = f(s);
        if *s != was[..n] {
            match self.din() {
                Dinode::Ufs1(d) => d.write_bytes(off, s),
                Dinode::Ufs2(d) => d.write_bytes(off, s),
            }
        }
        r
    }

    /// `ip->i_flag |= f`.
    #[inline]
    pub fn set_flag(&self, f: u32) {
        self.i_flag.set(self.i_flag.get() | f);
    }

    /// `ip->i_flag &= ~f`.
    #[inline]
    pub fn clr_flag(&self, f: u32) {
        self.i_flag.set(self.i_flag.get() & !f);
    }
}

/// `struct inode_vtbl`: the file system's inode operations the UFS layer calls (decision 46:
/// the C's table, `ffs_vtbl` and `ext2fs_vtbl`, as a trait). The `UFS_*` macros are its
/// methods through [`Inode::vtbl`].
pub trait InodeOps<G: ProcGlue>: Sync {
    /// `iv_truncate` (`UFS_TRUNCATE(ip, off, flags, cred)`).
    fn truncate(&self, ip: &Inode<G>, off: Off, flags: i32, cred: Cred<'_>) -> Result<(), Errno>;
    /// `iv_update` (`UFS_UPDATE(ip, sync)`).
    fn update(&self, ip: &Inode<G>, waitfor: i32) -> Result<(), Errno>;
    /// `iv_inode_alloc` (`UFS_INODE_ALLOC(pip, mode, cred, vpp)`): the new inode's vnode,
    /// locked and referenced.
    fn inode_alloc(
        &self,
        pip: &Inode<G>,
        mode: Mode,
        cred: Cred<'_>,
    ) -> Result<&'static Vnode<G>, Errno>;
    /// `iv_inode_free` (`UFS_INODE_FREE(pip, ino, mode)`).
    fn inode_free(&self, pip: &Inode<G>, ino: Ufsino, mode: Mode) -> Result<(), Errno>;
    /// `iv_buf_alloc` (`UFS_BUF_ALLOC(ip, startoffset, size, cred, flags, bpp)`): the buffer
    /// the C returns in `*bpp`.
    fn buf_alloc(
        &self,
        ip: &Inode<G>,
        startoffset: Off,
        size: i32,
        cred: Cred<'_>,
        flags: i32,
    ) -> Result<IoBuf, Errno>;
    /// `iv_bufatoff` (`UFS_BUFATOFF(ip, offset, res, bpp)`): the buffer and the offset of
    /// `res` in its data.
    fn bufatoff(&self, ip: &Inode<G>, offset: Off) -> Result<(IoBuf, usize), Errno>;
}

/// `IN_ACCESS`: access time update request.
pub const IN_ACCESS: u32 = 0x0001;
/// `IN_CHANGE`: inode change time update request.
pub const IN_CHANGE: u32 = 0x0002;
/// `IN_UPDATE`: modification time update request.
pub const IN_UPDATE: u32 = 0x0004;
/// `IN_MODIFIED`: inode has been modified.
pub const IN_MODIFIED: u32 = 0x0008;
/// `IN_RENAME`: inode is being renamed.
pub const IN_RENAME: u32 = 0x0010;
/// `IN_SHLOCK`: file has shared lock.
pub const IN_SHLOCK: u32 = 0x0020;
/// `IN_EXLOCK`: file has exclusive lock.
pub const IN_EXLOCK: u32 = 0x0040;
/// `IN_LAZYMOD`: modified, but don't write yet.
pub const IN_LAZYMOD: u32 = 0x0080;
/// `IN_HASHED`: inode is on the hash chain.
pub const IN_HASHED: u32 = 0x0100;

/// `struct indir`: logical block paths generated by `ufs_getlbns` and used by truncate and
/// bmap code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Indir {
    /// `in_lbn`: logical block number.
    pub in_lbn: Daddr,
    /// `in_off`: offset in buffer.
    pub in_off: i32,
    /// `in_exists`: flag if the block exists.
    pub in_exists: i32,
}

/// `VTOI(vp)`: the inode of a UFS vnode, one more handle to it; `panic(9)` for a vnode whose
/// node is not an inode of the glue `G` (LZ's tag check, made by the slot's type).
pub fn vtoi<G: ProcGlue>(vp: &Vnode<G>) -> InodeRef<G> {
    match vp.node_with(|ip: &Inode<G>| PoolArc::from_ref(ip)) {
        Some(Some(ip)) => ip,
        _ => kpanic(format_args!("VTOI: vnode {vp:p} has no inode")),
    }
}

/// `DOINGASYNC(vp)`: the file system is mounted `MNT_ASYNC`.
pub fn doingasync<G: ProcGlue>(vp: &Vnode<G>) -> bool {
    vp.mount().is_some_and(|mp| mp.flag() & MNT_ASYNC != 0)
}

/// `struct ufid`: this overlays the fid structure (see mount.h).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ufid {
    /// `ufid_len`: length of structure.
    pub ufid_len: u16,
    /// `ufid_pad`: force 32-bit alignment.
    pub ufid_pad: u16,
    /// `ufid_ino`: file number (ino).
    pub ufid_ino: Ufsino,
    /// `ufid_gen`: generation number.
    pub ufid_gen: u32,
}

impl Ufid {
    /// `(struct ufid *)fhp`: the file identifier read as a UFS one.
    pub fn from_fid(fid: &Fid) -> Self {
        let d = &fid.fid_data;
        Self {
            ufid_len: fid.fid_len,
            ufid_pad: fid.fid_reserved,
            ufid_ino: u32::from_ne_bytes([d[0], d[1], d[2], d[3]]),
            ufid_gen: u32::from_ne_bytes([d[4], d[5], d[6], d[7]]),
        }
    }

    /// Writes the UFS identifier over `fid` (the C's stores through the cast pointer).
    pub fn to_fid(&self, fid: &mut Fid) {
        fid.fid_len = self.ufid_len;
        fid.fid_reserved = self.ufid_pad;
        fid.fid_data[0..4].copy_from_slice(&self.ufid_ino.to_ne_bytes());
        fid.fid_data[4..8].copy_from_slice(&self.ufid_gen.to_ne_bytes());
    }
}

/// `((int32_t *)b_data)[i]`: an FFS1 block pointer in an indirect block.
pub fn daddr32_at(b: &[u8], i: usize) -> Daddr {
    let o = i * 4;
    Daddr::from(i32::from_ne_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]))
}

/// `((int32_t *)b_data)[i] = v`.
pub fn set_daddr32_at(b: &mut [u8], i: usize, v: Daddr) {
    b[i * 4..i * 4 + 4].copy_from_slice(&(v as i32).to_ne_bytes());
}

/// `((int64_t *)b_data)[i]`: an FFS2 block pointer in an indirect block.
pub fn daddr64_at(b: &[u8], i: usize) -> Daddr {
    let o = i * 8;
    let mut w = [0u8; 8];
    w.copy_from_slice(&b[o..o + 8]);
    i64::from_ne_bytes(w)
}

/// `((int64_t *)b_data)[i] = v`.
pub fn set_daddr64_at(b: &mut [u8], i: usize, v: Daddr) {
    b[i * 8..i * 8 + 8].copy_from_slice(&v.to_ne_bytes());
}

/// `((struct ufs1_dinode *)b_data)[i]`: a copy of an FFS1 dinode in an inode block;
/// `panic(9)` past the block's bytes, where LZ's slice index did.
pub fn dinode1_at(b: &[u8], i: usize) -> Ufs1Dinode {
    match b
        .get(i * size_of::<Ufs1Dinode>()..)
        .and_then(pod_from_bytes)
    {
        Some(d) => d,
        None => kpanic(format_args!("dinode1_at: dinode {i} past the block")),
    }
}

/// `((struct ufs1_dinode *)b_data)[i] = *d`.
pub fn set_dinode1_at(b: &mut [u8], i: usize, d: &Ufs1Dinode) {
    let written = b
        .get_mut(i * size_of::<Ufs1Dinode>()..)
        .is_some_and(|s| pod_to_bytes(d, s));
    if !written {
        kpanic(format_args!("set_dinode1_at: dinode {i} past the block"));
    }
}

/// `((struct ufs2_dinode *)b_data)[i]`: a copy of an FFS2 dinode in an inode block.
pub fn dinode2_at(b: &[u8], i: usize) -> Ufs2Dinode {
    match b
        .get(i * size_of::<Ufs2Dinode>()..)
        .and_then(pod_from_bytes)
    {
        Some(d) => d,
        None => kpanic(format_args!("dinode2_at: dinode {i} past the block")),
    }
}

/// `((struct ufs2_dinode *)b_data)[i] = *d`.
pub fn set_dinode2_at(b: &mut [u8], i: usize, d: &Ufs2Dinode) {
    let written = b
        .get_mut(i * size_of::<Ufs2Dinode>()..)
        .is_some_and(|s| pod_to_bytes(d, s));
    if !written {
        kpanic(format_args!("set_dinode2_at: dinode {i} past the block"));
    }
}

// The in-core dinodes are the C's sizes: `dino1pl`'s and `dino2pl`'s items.
const _: () = {
    assert!(size_of::<Ufs1Din>() == size_of::<Ufs1Dinode>());
    assert!(size_of::<Ufs2Din>() == size_of::<Ufs2Dinode>());
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn flags_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ufs/inode.h");
        for (name, value) in [
            ("IN_ACCESS", IN_ACCESS),
            ("IN_CHANGE", IN_CHANGE),
            ("IN_UPDATE", IN_UPDATE),
            ("IN_MODIFIED", IN_MODIFIED),
            ("IN_RENAME", IN_RENAME),
            ("IN_SHLOCK", IN_SHLOCK),
            ("IN_EXLOCK", IN_EXLOCK),
            ("IN_LAZYMOD", IN_LAZYMOD),
            ("IN_HASHED", IN_HASHED),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value)),
                "{name}"
            );
        }
    }

    #[test]
    fn block_pointers_and_fids_round_trip() {
        let mut b = [0u8; 64];
        set_daddr32_at(&mut b, 3, -7);
        assert_eq!(daddr32_at(&b, 3), -7);
        set_daddr64_at(&mut b, 5, 1 << 40);
        assert_eq!(daddr64_at(&b, 5), 1 << 40);
        let u = Ufid {
            ufid_len: 12,
            ufid_pad: 0,
            ufid_ino: 99,
            ufid_gen: 0xdead_beef,
        };
        let mut fid = Fid::default();
        u.to_fid(&mut fid);
        assert_eq!(Ufid::from_fid(&fid), u);
        assert_eq!(size_of::<Ufid>(), 12);
        // The fid's bytes: len and pad, then ino and gen at the head of fid_data.
        assert_eq!((fid.fid_len, fid.fid_reserved), (12, 0));
        assert_eq!(&fid.fid_data[0..4], &99u32.to_ne_bytes());
        assert_eq!(&fid.fid_data[4..8], &0xdead_beef_u32.to_ne_bytes());
    }

    /// A UFS1 dinode whose every member differs from its neighbours.
    fn din1() -> Ufs1Dinode {
        Ufs1Dinode {
            di_mode: 0o100644,
            di_nlink: 3,
            di_u: [11, 12],
            di_size: 0x1_2345_6789,
            di_atime: -5,
            di_atimensec: 6,
            di_mtime: 7,
            di_mtimensec: 8,
            di_ctime: 9,
            di_ctimensec: 10,
            di_db: [21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32],
            di_ib: [41, -42, 43],
            di_flags: 0x51,
            di_blocks: 52,
            di_gen: 53,
            di_uid: 54,
            di_gid: 55,
            di_spare: [56, 57],
        }
    }

    /// A UFS2 dinode whose every member differs from its neighbours.
    fn din2() -> Ufs2Dinode {
        Ufs2Dinode {
            di_mode: 0o040755,
            di_nlink: -2,
            di_uid: 1001,
            di_gid: 1002,
            di_blksize: 16384,
            di_size: 1 << 40,
            di_blocks: 77,
            di_atime: -1,
            di_mtime: 1 << 33,
            di_ctime: 3,
            di_birthtime: 4,
            di_mtimensec: 5,
            di_atimensec: 6,
            di_ctimensec: 7,
            di_birthnsec: 8,
            di_gen: -9,
            di_kernflags: 10,
            di_flags: 11,
            di_extsize: 12,
            di_extb: [13, 14],
            di_db: [101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, -112],
            di_ib: [121, 122, 1 << 50],
            di_spare: [131, 132, 133],
        }
    }

    #[test]
    fn dinodes_copy_in_and_out_of_a_block_at_their_offsets() {
        // ffs_vget's copy-in and ffs_update's copy-out: the dinode `ino_to_fsbo` names in an
        // inode block, byte for byte the on-disk structure.
        let mut blk = vec![0u8; 4096];
        set_dinode1_at(&mut blk, 5, &din1());
        assert_eq!(dinode1_at(&blk, 5), din1());
        assert_eq!(blk[5 * 128..5 * 128 + 2], 0o100644u16.to_ne_bytes());
        assert_eq!(
            blk[5 * 128 + 8..5 * 128 + 16],
            0x1_2345_6789u64.to_ne_bytes()
        );
        assert!(
            blk[..5 * 128].iter().all(|&b| b == 0),
            "the neighbours untouched"
        );
        set_dinode2_at(&mut blk, 15, &din2());
        assert_eq!(dinode2_at(&blk, 15), din2());
        assert_eq!(
            blk[15 * 256 + 16..15 * 256 + 24],
            (1u64 << 40).to_ne_bytes()
        );
        assert_eq!(dinode1_at(&blk, 5), din1(), "the earlier one kept");
    }

    #[test]
    #[should_panic(expected = "past the block")]
    fn a_dinode_past_the_block_is_refused() {
        let blk = vec![0u8; 1024];
        let _ = dinode2_at(&blk, 4);
    }

    #[test]
    fn the_in_core_dinode_reads_and_writes_each_member_alone() {
        let d = Ufs1Din::new(&din1());
        assert_eq!(d.load(), din1());
        assert_eq!(d.get::<u16>(offset_of!(Ufs1Dinode, di_mode)), 0o100644);
        assert_eq!(d.get::<i16>(offset_of!(Ufs1Dinode, di_nlink)), 3);
        assert_eq!(d.get::<u64>(offset_of!(Ufs1Dinode, di_size)), 0x1_2345_6789);
        assert_eq!(d.get::<i32>(offset_of!(Ufs1Dinode, di_atime)), -5);
        assert_eq!(d.get::<i32>(offset_of!(Ufs1Dinode, di_ib) + 4), -42);
        // A member changed alone: its neighbours in the same word stay.
        d.set::<i16>(offset_of!(Ufs1Dinode, di_nlink), -1);
        let mut want = din1();
        want.di_nlink = -1;
        assert_eq!(d.load(), want);
        d.set::<u16>(offset_of!(Ufs1Dinode, di_u) + 2, 0xffff);
        want.di_u[1] = 0xffff;
        assert_eq!(d.load(), want);

        let d2 = Ufs2Din::new(&din2());
        assert_eq!(d2.load(), din2());
        assert_eq!(d2.get::<i32>(offset_of!(Ufs2Dinode, di_gen)), -9);
        d2.set::<u32>(offset_of!(Ufs2Dinode, di_flags), 0xffff_ffff);
        let mut want2 = din2();
        want2.di_flags = 0xffff_ffff;
        assert_eq!(d2.load(), want2);
        d2.store(&din2());
        assert_eq!(d2.load(), din2());
    }

    #[test]
    fn byte_ranges_keep_the_words_outside_them() {
        let d = Ufs1Din::new(&din1());
        // SHORTLINK: the 60 bytes of di_db and di_ib, from offset 40 to 100 (half a word).
        let mut s = [0u8; MAXSYMLINKLEN_UFS1];
        d.read_bytes(offset_of!(Ufs1Dinode, di_db), &mut s);
        assert_eq!(s[..4], 21i32.to_ne_bytes());
        s.fill(b'x');
        d.write_bytes(offset_of!(Ufs1Dinode, di_db), &s);
        let got = d.load();
        assert_eq!(got.di_db, [i32::from_ne_bytes(*b"xxxx"); NDADDR]);
        assert_eq!(got.di_ib, [i32::from_ne_bytes(*b"xxxx"); NIADDR]);
        assert_eq!(got.di_flags, 0x51, "the member after the range kept");
        assert_eq!(got.di_ctimensec, 10, "the member before the range kept");
    }

    #[test]
    fn whole_word_ranges_keep_the_words_outside_them() {
        let d = Ufs2Din::new(&din2());
        // UFS2's SHORTLINK: the 120 bytes of di_db and di_ib, whole words from offset 112.
        let mut s = [0u8; MAXSYMLINKLEN_UFS2];
        d.read_bytes(offset_of!(Ufs2Dinode, di_db), &mut s);
        assert_eq!(s[..8], 101i64.to_ne_bytes());
        assert_eq!(s[112..], (1i64 << 50).to_ne_bytes());
        s.fill(b'y');
        d.write_bytes(offset_of!(Ufs2Dinode, di_db), &s);
        let got = d.load();
        assert_eq!(got.di_db, [i64::from_ne_bytes(*b"yyyyyyyy"); NDADDR]);
        assert_eq!(got.di_ib, [i64::from_ne_bytes(*b"yyyyyyyy"); NIADDR]);
        assert_eq!(got.di_extb, [13, 14], "the words before the range kept");
        assert_eq!(
            got.di_spare,
            [131, 132, 133],
            "the words after the range kept"
        );
    }

    #[test]
    #[should_panic(expected = "dinode: member")]
    fn a_member_across_two_words_is_refused() {
        let d = Ufs1Din::zeroed();
        let _ = d.get::<u64>(4);
    }

    #[test]
    fn the_cluster_info_is_read_and_written_whole() {
        let c = CiCell::new();
        let ci = ClusterInfo {
            ci_lastr: 1,
            ci_lastw: 2,
            ci_cstart: 3,
            ci_lasta: 4,
            ci_clen: 5,
            ci_ralen: 6,
            ci_maxra: 7,
        };
        c.set(ci);
        let got = c.get();
        assert_eq!(
            (got.ci_lastr, got.ci_lastw, got.ci_cstart, got.ci_lasta),
            (1, 2, 3, 4)
        );
        assert_eq!((got.ci_clen, got.ci_ralen, got.ci_maxra), (5, 6, 7));
    }

    #[test]
    fn the_short_link_is_read_in_place_and_written_back_when_changed() {
        use hal::intr::IPL_NONE;
        use hal::pool::{BoxPool, PR_WAITOK};

        use crate::fs::testfs::{TestGlue, setup};
        use crate::fs::ufs::ufs::ufsmount::UM_UFS2;

        static DINO2: BoxPool<Ufs2Din> = BoxPool::new();
        let _g = setup();
        let _ = DINO2.init(IPL_NONE, PR_WAITOK, "dino2pl");
        let ip: Inode<TestGlue> = Inode::new(InodeInit {
            fstype: UM_UFS2,
            ..InodeInit::default()
        });
        ip.set_din(Dinode::Ufs2(
            DINO2.get(PR_WAITOK, || Ufs2Din::new(&din2())).unwrap(),
        ));
        // ufs_readlink's use: the bytes of di_db and di_ib, the dinode left as it was.
        let mut got = [0u8; 16];
        let n = ip.with_shortlink(|s| {
            got.copy_from_slice(&s[..16]);
            s.len()
        });
        assert_eq!(n, MAXSYMLINKLEN_UFS2);
        assert_eq!(got[..8], 101i64.to_ne_bytes());
        assert_eq!(got[8..], 102i64.to_ne_bytes());
        assert_eq!(ip.din2(), din2());
        // ufs_symlink's: the target written over them, the members around them kept.
        ip.with_shortlink(|s| {
            s.fill(0);
            s[..5].copy_from_slice(b"/tmp/");
        });
        let d = ip.din2();
        assert_eq!(d.di_db[0].to_ne_bytes()[..5], *b"/tmp/");
        assert_eq!(d.di_db[1..], [0; NDADDR - 1]);
        assert_eq!(d.di_ib, [0; NIADDR]);
        assert_eq!(d.di_extb, [13, 14]);
        assert_eq!(d.di_spare, [131, 132, 133]);
    }
}
/* </TESTS> */
