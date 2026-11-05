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
 * Copyright (c) 1989, 1991, 1993, 1994
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
 *	@(#)ffs_vfsops.c	8.14 (Berkeley) 11/28/94
 */
/* </LICENSES> */

/* <CODE> */
//! The fast file system's file-system-type operations: mounting (`ffs_mountroot`,
//! `ffs_mount`, `ffs_mountfs` with the FFS1/FFS2 super-block search and compatibility code),
//! reloading, unmounting, `statfs`, `sync`, the inode cache's `ffs_vget`, file handles, the
//! super-block write-back (`ffs_sbupdate`), `ffs_init` and `ffs_sysctl`; and `ffs_vtbl`, the
//! inode operations the UFS code calls through `ip->i_vtbl`.
//!
//! Upstream: sys/ufs/ffs/ffs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_vfsops.rs@3c62ede68802
//!
//! ## Deviations
//! - The in-core-only slots of the super-block hold 0 where the C stores kernel addresses
//!   (`fs.rs`'s deviations): `ffs_sbupdate` writes 0 in `fs_csp`, `fs_contigdirs` and, with a
//!   cluster summary, `fs_maxcluster` (fsck ignores them); `ffs_reload` keeps the slots of the
//!   copy in core as the C restores its pointers.
//! - `ffs_mount` looks the device name up in a `Nameidata` of its own (`ndinit` around the
//!   kernel copy of the name) instead of reinitialising the caller's `ndp`, whose lifetime
//!   cannot hold a local name (LZ's).
//! - `disk_map` and `vfs_export` (with the export list, `um_export`) are `bsd`'s until N7 and
//!   N6: [`ffs_mount`] calls them through the glue (`FileGlue::disk_map`,
//!   `FileGlue::vfs_export`, docs/SCAFFOLDING.md), and [`ffs_mountfs`] empties the recycled
//!   mount's export list where the C's `malloc(M_ZERO)` of the `ufsmount` starts it empty
//!   (`FileGlue::ufs_export_init`).
//! - `rootdev` and `swapdev` are the caller's ([`ffs_mountroot`]'s arguments: `bsd`'s
//!   `ROOTDEV` and the machine's `conf.c` until N7); `rootvp` and `swapdev_vp` are `bsd`'s
//!   `init_main.c` globals, reached through the glue (`FileGlue::rootvp`, `set_rootvp`,
//!   `set_swapdev_vp`, docs/SCAFFOLDING.md); the error paths release the swap vnode they
//!   made, which the C reads back from its global.
//! - Two paths where the C dereferences NULL answer an error: [`ffs_mountroot`] returns
//!   `ENODEV` for a `NODEV` root (`bdevvp` succeeds with a NULL `rootvp`, which the C's
//!   `ffs_mountfs` then faults on), and [`ffs_mount`] returns `EINVAL` for a new mount without
//!   arguments (the C's `copyinstr(args->fspec, ..)` with `args == NULL`; only in-kernel
//!   callers pass none, since `sys_mount` always copies arguments in).
//! - `ffs_vars[]` holds only the `UFS_DIRHASH` variables (feature `ufs_dirhash`); without the
//!   feature it is empty and [`ffs_sysctl`] answers every name as `sysctl_bounded_arr` does
//!   for an unknown one (LZ's).
//! - `ffs_init`'s `static int done` and the three pools are the glue's tables
//!   (`UfsTables::ffs_init_done`, `ffs_ino`, `ffs_dino1`, `ffs_dino2`, D-N5.5b-10); the host
//!   tests clear the flag with the UFS layer's (`ufs_test_reset`).
//! - The `struct ffs_reload_args`/`struct ffs_sync_args` callbacks of
//!   `vfs_mount_foreach_vnode` are closures (`ffs_sync`'s over [`FfsSyncArgs`]).
//! - `ffs_checkrange` releases the buffer of a failed `bread` (the C returns without it), and
//!   reads `cg_initediblk` before the release (the C after) (LZ's).
//! - A failed read-write mount frees the summary space and `fs_contigdirs` with the rest of
//!   the structure (the drops of its last handle), where the C's `out:` frees only `um_fs`
//!   and the `ufsmount`.
//! - `ffs_sbupdate`'s compatibility rewrites of the buffer's super-block (`fs_nrpos`, the
//!   five words from `fs_qbmask`, `fs_maxfilesize`, `ffs1_compat_write`) write only the bytes
//!   inside the buffer: a member past `fs_sbsize` is a write the C makes past `b_bcount`,
//!   which never reaches the disk.
//! - A `ffs_reload` of a changed geometry (`fs_ncg`, `fs_cssize`, `fs_contigsumsize`) panics
//!   on the summary area's bounds where the C overruns its arrays.
//! - `ffs_mountfs`'s super-block search takes a buffer shorter than a super-block (a memory
//!   file system that ends inside the slot it reads, `mfs_doio`'s clamp) for no super-block
//!   and tries the next slot, where the C reads `struct fs` past `b_bcount`, out of whatever
//!   the buffer's memory held before (N5.5b3).
//!
//! ## Redesign
//! - N5.5b1: LZ's `FfsVtbl` unit struct and its `FFS_VTBL` static (`impl InodeOps` for
//!   `bsd`'s inode) -> [`Ffs<G>`], generic over the glue, and its constant [`Ffs::VTBL`]
//!   (`&Ffs::<G>::VTBL` is the `&'static dyn InodeOps<G>` an inode holds, the
//!   `SpecVops::<G>::TABLE` shape). Each method is the C's function of the same slot over the
//!   kernel's buffers: `iv_buf_alloc` and `iv_bufatoff` return the busy buffers' tokens
//!   (`IoBuf::Token`).
//! - N5.5b2: LZ's `FFS_VFSOPS` (a `bsd` `Vfsops` of function pointers over `bsd`'s mount)
//!   -> `impl Vfs<G> for Ffs<G>` and its constant [`Ffs::VFS`], which `bsd`'s `vfsconflist[]`
//!   names (`&Ffs::<BsdProcGlue>::VFS`); the functions are the C's, generic over the glue,
//!   in its order, and each `Vfs` method calls its function. The in-core super-block is the
//!   UFS mount's own `Superblock` (`fs.rs`): [`ffs_mountfs`] builds it from the buffer's
//!   `fs_sbsize` bytes and reads the summaries into its space, [`ffs_sbupdate`] copies its
//!   words and summaries out into the buffers (no `unsafe` copy, no `&'static Fs` over a
//!   `malloc`), [`ffs_reload`] copies the disk's bytes back in over its words, keeping the
//!   in-core slots. The UFS mount is the counted handle of the mount's typed slot (N5.5b0):
//!   each operation reached from the mount holds one (`vfstoufs`), each inode one
//!   (`InodeInit::ump`), and the unmount's `take_node` ends the mount's; the last frees the
//!   structure with its super-block. The super-block is read out of its buffer as a copy of
//!   the on-disk `struct fs` (`pod_from_bytes`) for `ffs_validate` and the mount's checks,
//!   where LZ cast the buffer (`fs_in_buf`, an `unsafe` reference).
//! - LZ's `FFS_INO_POOL`, `FFS_DINODE1_POOL`, `FFS_DINODE2_POOL` and `FFS_INIT_DONE` statics
//!   -> members of the glue's `UfsTables` (`ufs_vfsops.rs`), made once by [`ffs_init`].
//! - `ffs_vget` gives the new vnode the kernel's composed table (`UfsTables::ffs_vops`, the
//!   glue's tables' member) and its inode the UFS mount's handle and [`Ffs::VTBL`]; the dinode is
//!   copied out of the busy buffer's token (`dinode1_at`/`dinode2_at`), LZ's lent buffer and
//!   `unsafe` slice gone.
//! - `ffs1_compat_read` sign-extends `fs_ffs1_flags` into `fs_flags` as the C's `int8_t` to
//!   `u_int32_t` assignment does (LZ zero-extended it, so a super-block whose
//!   `FS_FLAGS_UPDATED` bit is set was written back with `0x80` where the C writes
//!   `0xffffff80`).
//! - Cost against the C: `VFSTOUFS` is a read of the mount's slot and one count pair per
//!   operation reached from the mount (`statfs`, `sync`, `vget`, `fhtovp`, `unmount`,
//!   `quotactl`), none on a read or a write; a member of the super-block is one relaxed load
//!   (`fs.rs`); the walks of `ffs_sync` and `ffs_reload` are the C's over
//!   `vfs_mount_foreach_vnode`, `vtoi` a count pair per vnode as LZ's. `ffs_sync_vnode` takes
//!   the C's one `splbio` pair per vnode.

use core::marker::PhantomData;
use core::mem::offset_of;
use core::ptr;
use core::sync::atomic::Ordering;

use hal::intr::{IPL_BIO, IPL_NONE, splraise};
use hal::malloc::MArc;
use hal::pool::{PR_WAITOK, PoolArc};
use hal::random::arc4random;
use hal::sleep::INFSLP;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::mount::{
    Fid, MNAMELEN, MNT_FORCE, MNT_LAZY, MNT_LOCAL, MNT_QUOTA, MNT_RDONLY, MNT_RELOAD, MNT_UPDATE,
    MNT_WAIT, MNT_WANTRDWR, Statfs, UfsArgs,
};
use hal::sys::param::PAGE_SIZE;
use hal::sys::sysctl::SysctlBoundedArgs;
use hal::sys::types::{Daddr, Dev, Ino, Mode, Off, Uid, major};
use hal::user::{UserPtr, pod_from_bytes};

use super::ffs_alloc::{ffs_inode_alloc, ffs_inode_free};
use super::ffs_balloc::ffs_balloc;
use super::ffs_inode::{ffs_truncate, ffs_update};
use super::ffs_subr::{ffs_bufatoff, ffs_vinit};
use super::fs::{
    AFPDIR, AVFILESIZ, CgView, FRAGTBL, FS_42INODEFMT, FS_42POSTBLFMT, FS_44INODEFMT,
    FS_FLAGS_UPDATED, FS_MAGIC, FS_UFS1_MAGIC, FS_UFS2_MAGIC, FS_UNCLEAN, Fs, MAXFRAG, SBLOCK_UFS2,
    SBLOCKSEARCH, SBSIZE, Superblock, cgtod, fs_kernmaxfilesize, fsbtodb, ino_to_cg, ino_to_fsba,
    ino_to_fsbo, nindir,
};
use crate::assert::kpanic;
use crate::file::{DM_OPENBLCK, FileGlue};
use crate::fs::bio::getblk_wait;
use crate::fs::bio::{DEV_BSHIFT, bawrite, bread, brelse, bwrite};
use crate::fs::buf::{B_INVAL, B_NOCACHE, BufRef};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::mount::{Mount, Vfsconf, mountlist_insert_tail, vfs_mount_free, vfs_unbusy};
use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
use crate::fs::spec::devsw;
use crate::fs::subr::{
    bdevvp, copy_statfs_info, getnewvnode, vcount, vflush, vfs, vfs_mount_foreach_vnode,
    vfs_mountedon, vfs_rootmountalloc, vget, vgonel, vinvalbuf, vput, vref, vrele,
};
use crate::fs::ufs::ufs::dinode::{NDADDR, NIADDR, ROOTINO, Ufsino};
use crate::fs::ufs::ufs::dir::MAXNAMLEN;
use crate::fs::ufs::ufs::inode::{
    Dinode, IN_ACCESS, IN_CHANGE, IN_LAZYMOD, IN_MODIFIED, IN_UPDATE, Inode, InodeInit, InodeOps,
    Ufid, Ufs1Din, dinode1_at, vtoi,
};
#[cfg(feature = "ffs2")]
use crate::fs::ufs::ufs::inode::{Ufs2Din, dinode2_at};
use crate::fs::ufs::ufs::quota::{MAXQUOTAS, qsync, quotaoff, ufs_quotactl};
use crate::fs::ufs::ufs::ufs_ihash::{ufs_ihashget, ufs_ihashins};
use crate::fs::ufs::ufs::ufs_vfsops::{ufs_fhtovp, ufs_init, ufs_root, ufs_start};
#[cfg(feature = "ffs2")]
use crate::fs::ufs::ufs::ufsmount::UM_UFS2;
use crate::fs::ufs::ufs::ufsmount::{UM_UFS1, Ufsmount, UfsmountRef, vfstoufs};
use crate::fs::vnode::{
    FORCECLOSE, IGNORECLEAN, LK_EXCLUSIVE, LK_NOWAIT, LK_RETRY, SKIPSYSTEM, V_SAVE, VBLK, VNON,
    VT_UFS, Vnode, WRITECLOSE, vn_lock,
};
use crate::fs::vops::{
    Cred, IoBuf, Nd, VFS_STATFS, VFS_SYNC, Vfs, vop_close, vop_fsync, vop_open, vop_unlock,
};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::sysctl::{SysctlReq, bounded_arr};
use crate::tc::gettime;
use crate::time::inittodr;

/// `DEV_BSIZE`.
const DEV_BSIZE: i64 = 1 << DEV_BSHIFT;

/// `sbtry[]`: possible locations for the super-block.
pub const SBTRY: [i32; 4] = SBLOCKSEARCH;

/// The fast file system of the glue `G`: `ffs_vfsops` ([`Ffs::VFS`]), `ffs_vtbl` (the inode
/// operations, [`Ffs::VTBL`]), and the file system's own slots of the vnode tables `ffs_vops`
/// and `ffs_specvops` (`ffs_vnops.rs`'s `UfsFs`; the tables are the glue's
/// `UfsTables::ffs_vops` and `ffs_specvops`).
pub struct Ffs<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Ffs<G> {
    /// `ffs_vfsops`: the table a mount's `mnt_op` names (`vfsconflist[]`'s `"ffs"`).
    pub const VFS: Ffs<G> = Ffs(PhantomData);
    /// `ffs_vtbl`: the table an FFS inode's `i_vtbl` names.
    pub const VTBL: Ffs<G> = Ffs(PhantomData);
}

/// `struct ffs_sync_args`.
struct FfsSyncArgs<'a, G: ProcGlue> {
    /// `allerror`.
    allerror: Result<(), Errno>,
    /// `p`.
    p: &'a Thread<G>,
    /// `waitfor`.
    waitfor: i32,
    /// `nlink0`.
    nlink0: i32,
    /// `inflight`.
    inflight: i32,
    /// `cred`.
    cred: Cred<'a>,
}

/// `ffs_vars[]`: the `UFS_DIRHASH` variables `vfs.ffs.dirhash_dirsize`, `dirhash_maxmem`
/// and `dirhash_mem` (read-only).
#[cfg(feature = "ufs_dirhash")]
static FFS_VARS: [SysctlBoundedArgs; 3] = {
    use super::ffs_extern::{FFS_DIRHASH_DIRSIZE, FFS_DIRHASH_MAXMEM, FFS_DIRHASH_MEM};
    use crate::fs::ufs::ufs::ufs_dirhash::{UFS_DIRHASHMAXMEM, UFS_DIRHASHMEM, UFS_MINDIRHASHSIZE};
    [
        SysctlBoundedArgs::new(FFS_DIRHASH_DIRSIZE, &UFS_MINDIRHASHSIZE, 0, i32::MAX),
        SysctlBoundedArgs::new(FFS_DIRHASH_MAXMEM, &UFS_DIRHASHMAXMEM, 0, i32::MAX),
        SysctlBoundedArgs::readonly(FFS_DIRHASH_MEM, &UFS_DIRHASHMEM),
    ]
};
/// `ffs_vars[]`: empty without `UFS_DIRHASH`.
#[cfg(not(feature = "ufs_dirhash"))]
static FFS_VARS: [SysctlBoundedArgs; 0] = [];

/// `ffs_checkrange`: whether `ino` can name an inode of the mount (`ESTALE` if not), with
/// FFS2's lazy inode initialisation taken into account.
pub fn ffs_checkrange<G: ProcGlue>(mp: &'static Mount<G>, ino: u32) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    let fs = ump.um_fs();
    if ino < ROOTINO || ino >= fs.fs_ncg().wrapping_mul(fs.fs_ipg()) {
        return Err(Errno::ESTALE);
    }

    // Need to check if inode is initialized because ffsv2 does lazy initialization and we
    // can get here from nfs_fhtovp
    if fs.fs_magic() != FS_UFS2_MAGIC {
        return Ok(());
    }

    let cg = ino_to_cg(fs, ino);

    let (mut t, error) = bread(
        &ump.devvp().bufs,
        fsbtodb(fs, cgtod(fs, cg)),
        fs.fs_cgsize(),
    );
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }

    let cgp = CgView::new(t.data());
    if !cgp.cg_chkmagic() {
        brelse(t);
        return Err(Errno::ESTALE);
    }
    let initediblk = cgp.cg_initediblk();

    brelse(t);

    if cg.wrapping_mul(fs.fs_ipg()).wrapping_add(initediblk) < ino {
        return Err(Errno::ESTALE);
    }

    Ok(())
}

/// `ffs_mountroot`: called by `main()` when ufs is going to be mounted as root: the vnodes of
/// `swapdev` and `rootdev` (into `swapdev_vp` and `rootvp`), the root mount of `"ffs"`, the
/// mount list, and the clock set from the super-block's time.
pub fn ffs_mountroot<G: ProcGlue>(rootdev: Dev, swapdev: Dev) -> Result<(), Errno> {
    G::with_curproc(|p| {
        let Some(p) = p else {
            kpanic(format_args!("ffs_mountroot: no curproc"));
        };

        // Get vnodes for swapdev and rootdev.
        <G::FileGlue as FileGlue>::set_swapdev_vp(None);
        let mut svp = None;
        let made = bdevvp::<G>(swapdev).and_then(|vp| {
            svp = vp;
            <G::FileGlue as FileGlue>::set_swapdev_vp(vp);
            bdevvp::<G>(rootdev)
        });
        let rvp = match made {
            Ok(Some(rvp)) => rvp,
            r => {
                printf(format_args!("ffs_mountroot: can't setup bdevvp's\n"));
                if let Some(svp) = svp {
                    vrele(svp);
                }
                // A NODEV root (`bdevvp`'s NULL) has no vnode to mount from.
                return Err(r.err().unwrap_or(Errno::ENODEV));
            }
        };
        <G::FileGlue as FileGlue>::set_rootvp(Some(rvp));

        let release = || {
            if let Some(svp) = svp {
                vrele(svp);
            }
            vrele(rvp);
        };

        let mp = match vfs_rootmountalloc::<G>(b"ffs", b"root_device") {
            Ok(mp) => mp,
            Err(e) => {
                release();
                return Err(e);
            }
        };

        if let Err(e) = ffs_mountfs(rvp, mp, p) {
            vfs_unbusy(mp);
            vfs_mount_free(mp);
            release();
            return Err(e);
        }

        mountlist_insert_tail(mp);
        let ump = vfstoufs(mp);
        let fs = ump.um_fs();
        let (name, len) = mp.mntonname();
        fs.set_fsmnt(&name[..len]);
        let mut st = mp.stat();
        let _ = ffs_statfs(mp, &mut st, p);
        mp.with_stat(|sp| *sp = st);
        vfs_unbusy(mp);
        inittodr(fs.fs_time());

        Ok(())
    })
}

/// `memset(dst, 0, MNAMELEN); strlcpy(dst, src, MNAMELEN)`: the C's pair, over libkern's
/// `strlcpy(9)` (statfs(2) copies the whole buffer out, so the zeroed tail is visible); mfs's
/// `mfs_mount` makes the same pairs.
pub(crate) fn mname_copy(dst: &mut [u8; MNAMELEN], src: &[u8]) {
    *dst = [0; MNAMELEN];
    libkern::strlcpy(dst, src);
}

/// `ffs_mount` (`vfs_mount`): mount system call. `data` is the kernel copy of the user's
/// `struct ufs_args` (empty for the C's NULL).
pub fn ffs_mount<G: ProcGlue>(
    mp: &'static Mount<G>,
    path: &[u8],
    data: &mut [u8],
    ndp: &mut Nd<'_, G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let args = UfsArgs::from_bytes(data);
    let mut ump: Option<UfsmountRef<G>> = None;
    let mut ronly = 0;
    let mut fname = [0u8; MNAMELEN];
    let mut fspec = [0u8; MNAMELEN];

    // If updating, check whether changing from read-only to read/write; if there is no
    // device name, that's all we do.
    let error: Result<(), Errno> = 'error_1: {
        'success: {
            if mp.flag() & MNT_UPDATE != 0 {
                let u: &UfsmountRef<G> = ump.insert(vfstoufs(mp));
                let fs = u.um_fs();
                ronly = fs.fs_ronly();
                let mut error = Ok(());

                if ronly == 0 && mp.flag() & MNT_RDONLY != 0 {
                    // Flush any dirty data
                    let _ = VFS_SYNC(mp, MNT_WAIT, 0, Cred::real(p.ucred()), p);

                    // Get rid of files open for writing.
                    let mut flags = WRITECLOSE;
                    if args.is_none() {
                        flags |= IGNORECLEAN;
                    }
                    if mp.flag() & MNT_FORCE != 0 {
                        flags |= FORCECLOSE;
                    }
                    error = ffs_flushfiles(mp, flags, p);
                    mp.set_flag(mp.flag() | MNT_RDONLY);
                    ronly = 1;
                }

                if error.is_ok() && mp.flag() & MNT_RELOAD != 0 {
                    error = ffs_reload(mp, ndp.ni_cnd.cn_cred, p);
                }
                if let Err(e) = error {
                    break 'error_1 Err(e);
                }

                if ronly != 0 && mp.flag() & MNT_WANTRDWR != 0 {
                    if fs.fs_clean() == 0 {
                        if mp.flag() & MNT_FORCE != 0 {
                            printf(format_args!(
                                "WARNING: {} was not properly unmounted\n",
                                fs.fsmnt()
                            ));
                        } else {
                            printf(format_args!(
                                "WARNING: R/W mount of {} denied.  Filesystem is not clean - run fsck\n",
                                fs.fsmnt()
                            ));
                            break 'error_1 Err(Errno::EROFS);
                        }
                    }

                    fs.contigdirs_alloc();

                    ronly = 0;
                }
                let Some(a) = &args else {
                    break 'success;
                };
                if a.fspec == 0 {
                    // Process export requests.
                    if let Err(e) = <G::FileGlue as FileGlue>::vfs_export(mp, &a.export_info) {
                        break 'error_1 Err(e);
                    }
                    break 'success;
                }
            }

            // Not an update, or updating the name: look up the name and verify that it
            // refers to a sensible block device.
            let Some(a) = &args else {
                break 'error_1 Err(Errno::EINVAL);
            };
            if let Err(e) = UserPtr::<u8>::new(a.fspec).copyinstr(&mut fspec) {
                break 'error_1 Err(e);
            }

            if !<G::FileGlue as FileGlue>::disk_map(&fspec, &mut fname, DM_OPENBLCK) {
                fname = fspec;
            }

            let flen = fname.iter().position(|&c| c == 0).unwrap_or(MNAMELEN);
            let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(&fname[..flen]), p);
            if let Err(e) = namei(&mut nd) {
                break 'error_1 Err(e);
            }
            let Some(devvp) = nd.ni_vp else {
                break 'error_1 Err(Errno::ENOENT);
            };

            let error: Result<(), Errno> = 'error_2: {
                if devvp.vtype() != VBLK {
                    break 'error_2 Err(Errno::ENOTBLK);
                }

                if major(devvp.rdev()) >= devsw::<G>().nblkdev() {
                    break 'error_2 Err(Errno::ENXIO);
                }

                let mut error = Ok(());
                if mp.flag() & MNT_UPDATE != 0 {
                    // UPDATE
                    // If it's not the same vnode, or at least the same device then it's not
                    // correct.
                    let Some(u) = &ump else {
                        kpanic(format_args!("ffs_mount: update without ufsmount"));
                    };
                    if ptr::eq(devvp, u.devvp()) || devvp.rdev() == u.devvp().rdev() {
                        vrele(devvp);
                    } else {
                        error = Err(Errno::EINVAL); // needs translation
                    }
                    // Update device name only on success
                    if error.is_ok() {
                        // Save "mounted from" info for mount point (NULL pad)
                        mp.with_stat(|sp| {
                            mname_copy(&mut sp.f_mntfromname, &fname);
                            mname_copy(&mut sp.f_mntfromspec, &fspec);
                        });
                    }
                } else {
                    // Since this is a new mount, we want the names for the device and the
                    // mount point copied in. If an error occurs, the mountpoint is discarded
                    // by the upper level code.
                    mp.with_stat(|sp| {
                        mname_copy(&mut sp.f_mntonname, path);
                        mname_copy(&mut sp.f_mntfromname, &fname);
                        mname_copy(&mut sp.f_mntfromspec, &fspec);
                    });

                    error = ffs_mountfs(devvp, mp, p);
                }

                if error.is_err() {
                    break 'error_2 error;
                }

                // Initialize FS stat information in mount struct; uses both
                // mp->mnt_stat.f_mntonname and mp->mnt_stat.f_mntfromname
                //
                // This code is common to root and non-root mounts
                mp.with_stat(|sp| {
                    sp.mount_info.__align[..UfsArgs::SIZE].copy_from_slice(&data[..UfsArgs::SIZE]);
                });
                let mut st = mp.stat();
                let _ = VFS_STATFS(mp, &mut st, p);
                mp.with_stat(|sp| *sp = st);
                Ok(())
            };
            if let Err(e) = error {
                // error_2: error with devvp held
                vrele(devvp);
                break 'error_1 Err(e);
            }
        }

        // success:
        if !path.is_empty() && mp.flag() & MNT_UPDATE != 0 {
            // Update clean flag after changing read-onlyness.
            let Some(u) = &ump else {
                kpanic(format_args!("ffs_mount: update without ufsmount"));
            };
            let fs = u.um_fs();
            if ronly != fs.fs_ronly() {
                fs.set_fs_ronly(ronly);
                fs.set_fs_clean(i8::from(ronly != 0 && fs.fs_flags() & FS_UNCLEAN == 0));
                if ronly != 0 {
                    fs.contigdirs_free();
                }
            }
            let _ = ffs_sbupdate(u, MNT_WAIT);
        }
        return Ok(());
    };

    // error_1: no state to back out
    error
}

/// `ffs_reload_vnode`: steps 4 to 6 of [`ffs_reload`] for one vnode.
fn ffs_reload_vnode<G: ProcGlue>(
    vp: &'static Vnode<G>,
    fs: &Superblock,
    p: &Thread<G>,
    cred: Cred<'_>,
    devvp: &'static Vnode<G>,
) -> Result<(), Errno> {
    // Step 4: invalidate all inactive vnodes.
    if vp.usecount() == 0 {
        vgonel(vp, Some(p));
        return Ok(());
    }

    // Step 5: invalidate all cached file data.
    if vget(vp, LK_EXCLUSIVE).is_err() {
        return Ok(());
    }

    if vinvalbuf(vp, 0, cred, Some(p), 0, INFSLP).is_err() {
        kpanic(format_args!("ffs_reload: dirty2"));
    }

    // Step 6: re-read inode data for all active vnodes.
    let ip = vtoi(vp);

    let (mut t, error) = bread(
        &devvp.bufs,
        fsbtodb(fs, ino_to_fsba(fs, ip.i_number)),
        fs.fs_bsize(),
    );
    if let Err(e) = error {
        brelse(t);
        drop(ip);
        vput(vp);
        return Err(e);
    }

    let idx = ino_to_fsbo(fs, ip.i_number);
    if fs.fs_magic() == FS_UFS1_MAGIC {
        ip.ufs1_din().store(&dinode1_at(t.data(), idx));
    } else {
        #[cfg(feature = "ffs2")]
        ip.ufs2_din().store(&dinode2_at(t.data(), idx));
    }
    ip.i_effnlink.set(ip.dip_nlink());
    brelse(t);
    drop(ip);
    vput(vp);
    Ok(())
}

/// `ffs_reload`: reload all incore data for a filesystem (used after running fsck on the
/// root filesystem and finding things to fix). The filesystem must be mounted read-only.
///
/// Things to do to update the mount:
///  1) invalidate all cached meta-data.
///  2) re-read superblock from disk.
///  3) re-read summary information from disk.
///  4) invalidate all inactive vnodes.
///  5) invalidate all cached file data.
///  6) re-read inode data for all active vnodes.
pub fn ffs_reload<G: ProcGlue>(
    mountp: &'static Mount<G>,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if mountp.flag() & MNT_RDONLY == 0 {
        return Err(Errno::EINVAL);
    }
    // Step 1: invalidate all cached meta-data.
    let ump = vfstoufs(mountp);
    let devvp = ump.devvp();
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vinvalbuf(devvp, 0, cred, Some(p), 0, INFSLP);
    let _ = vop_unlock(devvp);
    if error.is_err() {
        kpanic(format_args!("ffs_reload: dirty1"));
    }

    // Step 2: re-read superblock from disk.
    let fs = ump.um_fs();

    let (mut t, error) = bread(&devvp.bufs, fs.fs_sblockloc() / DEV_BSIZE, SBSIZE as i32);
    if let Err(e) = error {
        brelse(t);
        return Err(e);
    }

    let Some(newfs) = pod_from_bytes::<Fs>(t.bytes()) else {
        kpanic(format_args!(
            "ffs_reload: a super-block buffer of {} bytes",
            t.bytes().len()
        ));
    };
    if !ffs_validate(&newfs) {
        brelse(t);
        return Err(Errno::EINVAL);
    }

    // Copy pointer fields back into superblock before copying in new superblock. These
    // should really be in the ufsmount. Note that important parameters (eg fs_ncg) are
    // unchanged.
    {
        let b = t.data();
        put_bytes(b, offset_of!(Fs, fs_csp), &fs.fs_csp_slot().to_ne_bytes());
        put_bytes(
            b,
            offset_of!(Fs, fs_maxcluster),
            &fs.fs_maxcluster_slot().to_ne_bytes(),
        );
        put_bytes(b, offset_of!(Fs, fs_ronly), &fs.fs_ronly().to_ne_bytes());
        let n = (fs.fs_sbsize().max(0) as usize).min(b.len());
        fs.write_bytes(0, &b[..n]);
    }
    if (fs.fs_sbsize() as usize) < SBSIZE {
        t.buf().set(B_INVAL);
    }
    brelse(t);
    ump.um_maxsymlinklen.set(fs.fs_maxsymlinklen() as u32);
    ffs1_compat_read(fs, &ump, fs.fs_sblockloc());
    ffs_oldfscompat(fs);
    let mut st = mountp.stat();
    let _ = ffs_statfs(mountp, &mut st, p);
    mountp.with_stat(|sp| *sp = st);
    // Step 3: re-read summary information from disk.
    read_summary(fs, devvp)?;
    // We no longer know anything about clusters per cylinder group.
    if fs.fs_contigsumsize() > 0 {
        for i in 0..fs.fs_ncg() {
            fs.set_maxcluster(i, fs.fs_contigsumsize());
        }
    }

    vfs_mount_foreach_vnode(mountp, &mut |vp| ffs_reload_vnode(vp, fs, p, cred, devvp))
}

/// The cylinder group summaries (`fs_cssize` bytes at `fs_csaddr`, in whole fragments) read
/// into the super-block's summary space (`ffs_mountfs`'s and `ffs_reload`'s loop).
fn read_summary<G: ProcGlue>(fs: &Superblock, devvp: &'static Vnode<G>) -> Result<(), Errno> {
    let blks = howmany(fs.fs_cssize(), fs.fs_fsize());
    let mut space = 0;
    let mut i = 0;
    while i < blks {
        let mut size = fs.fs_bsize();
        if i + fs.fs_frag() > blks {
            size = (blks - i) * fs.fs_fsize();
        }
        let (t, error) = bread(
            &devvp.bufs,
            fsbtodb(fs, fs.fs_csaddr() + Daddr::from(i)),
            size,
        );
        if let Err(e) = error {
            brelse(t);
            return Err(e);
        }
        let n = size as usize;
        fs.write_csum_bytes(space, &t.bytes()[..n]);
        space += n;
        brelse(t);
        i += fs.fs_frag();
    }
    Ok(())
}

/// `howmany(x, y)` of the super-block's `int`s (a negative `x` counts as none).
fn howmany(x: i32, y: i32) -> i32 {
    crate::select::howmany(x.max(0) as usize, y.max(1) as usize) as i32
}

/// `ffs_validate`: checks if a super block is sane enough to be mounted.
pub fn ffs_validate(fsp: &Fs) -> bool {
    #[cfg(feature = "ffs2")]
    if fsp.fs_magic != FS_UFS2_MAGIC && fsp.fs_magic != FS_UFS1_MAGIC {
        return false; // Invalid magic
    }
    #[cfg(not(feature = "ffs2"))]
    if fsp.fs_magic != FS_UFS1_MAGIC {
        return false; // Invalid magic
    }

    if fsp.fs_bsize as u32 > crate::vm::pager::MAXBSIZE as u32 {
        return false; // Invalid block size
    }

    if (fsp.fs_bsize as u32) < size_of::<Fs>() as u32 {
        return false; // Invalid block size
    }

    if fsp.fs_sbsize as u32 > SBSIZE as u32 {
        return false; // Invalid super block size
    }

    let frag = fsp.fs_frag as u32;
    if frag > MAXFRAG as u32 || FRAGTBL[frag as usize].is_none() {
        return false; // Invalid number of fragments
    }

    if fsp.fs_inodefmt == FS_42INODEFMT {
        return false; // Obsolete format, support broken in 2014
    }
    if fsp.fs_maxsymlinklen <= 0 {
        return false; // Invalid max size of short symlink
    }

    true // Super block is okay
}

/// `ffs_mountfs`: common code for mount and mountroot: read and check the super-block on
/// `devvp`, set up the in-core super-block, its summaries and the `ufsmount` in the mount's
/// slot, and mark the file system dirty when mounting read-write.
pub fn ffs_mountfs<G: ProcGlue>(
    devvp: &'static Vnode<G>,
    mp: &'static Mount<G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let dev = devvp.rdev();
    let cred = Cred::real(p.ucred());
    // Disallow multiple mounts of the same device. Disallow mounting of a device that is
    // currently in use (except for root, which might share swap device for miniroot). Flush
    // out any old buffers remaining from a previous use.
    vfs_mountedon(devvp)?;
    if vcount(devvp) > 1 && !<G::FileGlue as FileGlue>::rootvp().is_some_and(|r| ptr::eq(r, devvp))
    {
        return Err(Errno::EBUSY);
    }
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vinvalbuf(devvp, V_SAVE, cred, Some(p), 0, INFSLP);
    let _ = vop_unlock(devvp);
    error?;

    let ronly = mp.flag() & MNT_RDONLY != 0;
    let omode = if ronly { FREAD } else { FREAD | FWRITE };
    vop_open(devvp, omode, Cred::FSCRED, p)?;

    let mut bp: Option<BufRef> = None;
    let mut ump: Option<UfsmountRef<G>> = None;

    let error: Errno = 'out: {
        // Try reading the super-block in each of its possible locations.
        let mut i = 0;
        let mut sbloc = 0;
        let mut found = None;
        while SBTRY[i] != -1 {
            if let Some(b) = bp.take() {
                b.buf().set(B_NOCACHE);
                brelse(b);
            }

            let (b, error) = bread(&devvp.bufs, i64::from(SBTRY[i]) / DEV_BSIZE, SBSIZE as i32);
            let b = bp.insert(b);
            if let Err(e) = error {
                break 'out e;
            }

            // A buffer shorter than a super-block (a memory file system that ends inside
            // this slot: `mfs_doio` clamps the transfer at its end) holds none.
            let Some(fs) = pod_from_bytes::<Fs>(b.bytes()) else {
                i += 1;
                continue;
            };
            sbloc = SBTRY[i];

            // Do not look for an FFS1 file system at SBLOCK_UFS2. Doing so will find the
            // wrong super-block for file systems with 64k block size.
            if !(fs.fs_magic == FS_UFS1_MAGIC && sbloc == SBLOCK_UFS2) && ffs_validate(&fs) {
                found = Some(fs);
                break; // Super block validated
            }
            i += 1;
        }

        let (Some(mut bfs), Some(b)) = (found, bp.as_mut()) else {
            break 'out Errno::EINVAL;
        };

        // fs->fs_fmod = 0; fs->fs_flags &= ~FS_UNCLEAN: in the buffer, which the copy in
        // core is made of, and in the copy read out of it.
        bfs.fs_fmod = 0;
        bfs.fs_flags &= !FS_UNCLEAN;
        put_bytes(
            b.data(),
            offset_of!(Fs, fs_fmod),
            &bfs.fs_fmod.to_ne_bytes(),
        );
        put_bytes(
            b.data(),
            offset_of!(Fs, fs_flags),
            &bfs.fs_flags.to_ne_bytes(),
        );
        if bfs.fs_clean == 0 {
            if ronly || mp.flag() & MNT_FORCE != 0 {
                printf(format_args!(
                    "WARNING: {} was not properly unmounted\n",
                    bfs.fsmnt()
                ));
            } else {
                printf(format_args!(
                    "WARNING: R/W mount of {} denied.  Filesystem is not clean - run fsck\n",
                    bfs.fsmnt()
                ));
                break 'out Errno::EROFS;
            }
        }

        if bfs.fs_postblformat == FS_42POSTBLFMT && !ronly {
            printf(format_args!(
                "ffs_mountfs(): obsolete rotational table format, please use fsck_ffs(8) -c 1\n"
            ));
            break 'out Errno::EROFS;
        }

        let u: &UfsmountRef<G> = ump.insert(Ufsmount::alloc());
        // The zeroed structure's export list (the mount's glue member).
        <G::FileGlue as FileGlue>::ufs_export_init(mp);
        // ump->um_fs = malloc(fs_sbsize); memcpy(ump->um_fs, bp->b_data, fs_sbsize), and
        // its summary space (`fs.rs`).
        let sbsize = bfs.fs_sbsize as usize;
        let fs = Superblock::new(&b.bytes()[..sbsize]);

        if bfs.fs_magic == FS_UFS1_MAGIC {
            u.um_fstype.set(UM_UFS1);
        } else {
            #[cfg(feature = "ffs2")]
            u.um_fstype.set(UM_UFS2);
        }

        if sbsize < SBSIZE {
            b.buf().set(B_INVAL);
        }
        if let Some(b) = bp.take() {
            brelse(b);
        }

        ffs1_compat_read(&fs, u, i64::from(sbloc));

        if fs.fs_clean() == 0 {
            fs.set_fs_flags(fs.fs_flags() | FS_UNCLEAN);
        }
        fs.set_fs_ronly(i8::from(ronly));
        // The summaries; a failed read frees them with the copy (the C's free(fs_csp) and
        // `out:`'s free(um_fs)).
        if let Err(e) = read_summary(&fs, devvp) {
            break 'out e;
        }
        // fs_maxcluster's counters start at fs_contigsumsize (`Superblock::new`).
        u.set_um_fs(fs);
        let fs = u.um_fs();
        mp.set_node(MArc::erase(u.clone()));
        mp.with_stat(|sp| {
            sp.f_fsid.val[0] = dev;
            // Use on-disk fsid if it exists, else fake it
            let id = fs.fs_id();
            if id[0] != 0 && id[1] != 0 {
                sp.f_fsid.val[1] = id[1];
            } else {
                sp.f_fsid.val[1] = mp.vfc().vfc_typenum;
            }
            sp.f_namemax = MAXNAMLEN as u32;
        });
        mp.set_flag(mp.flag() | MNT_LOCAL);
        u.um_mountp.set(Some(mp));
        u.um_dev.set(dev);
        u.um_devvp.set(Some(devvp));
        u.um_nindir.set(fs.fs_nindir() as u64);
        u.um_bptrtodb.set(fs.fs_fsbtodb() as u64);
        u.um_seqinc.set(fs.fs_frag() as u64);
        u.um_maxsymlinklen.set(fs.fs_maxsymlinklen() as u32);
        for q in &u.um_quotas {
            q.set(None);
        }

        devvp.set_specmountpoint(Some(mp));
        ffs_oldfscompat(fs);

        if ronly {
            fs.contigdirs_free();
        } else {
            fs.contigdirs_alloc();
        }

        // Set FS local "last mounted on" information (NULL pad)
        let (name, len) = mp.mntonname();
        fs.set_fsmnt(&name[..len]);

        // XXX
        // Limit max file size. Even though ffs can handle files up to 16TB, we do limit the
        // max file to 2^31 pages to prevent overflow of a 32-bit unsigned int. The buffer
        // cache has its own checks but a little added paranoia never hurts.
        u.um_savedmaxfilesize.set(fs.fs_maxfilesize()); // XXX
        let maxfilesize = fs_kernmaxfilesize(PAGE_SIZE as u64, fs);
        if fs.fs_maxfilesize() > maxfilesize {
            fs.set_fs_maxfilesize(maxfilesize); // XXX
        }
        if !ronly {
            fs.set_fs_fmod(1);
            fs.set_fs_clean(0);
            if let Err(Errno::EROFS) = ffs_sbupdate(u, MNT_WAIT) {
                break 'out Errno::EROFS;
            }
        }
        return Ok(());
    };

    // out:
    if devvp.has_specinfo() {
        devvp.set_specmountpoint(None);
    }
    if let Some(b) = bp {
        brelse(b);
    }

    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vop_close(devvp, omode, cred, Some(p));
    let _ = vop_unlock(devvp);

    if ump.is_some() {
        // `mp->mnt_data = NULL`; the last handle, `ump`, frees the structure and its
        // super-block.
        drop(mp.take_node());
    }
    drop(ump);
    Err(error)
}

/// `ffs_oldfscompat`: sanity checks for old file systems.
pub fn ffs_oldfscompat(fs: &Superblock) -> i32 {
    fs.set_fs_npsect(fs.fs_npsect().max(fs.fs_nsect())); // XXX
    fs.set_fs_interleave(fs.fs_interleave().max(1)); // XXX
    if fs.fs_postblformat() == FS_42POSTBLFMT {
        // XXX
        fs.set_fs_nrpos(8); // XXX
    }
    if fs.fs_inodefmt() < FS_44INODEFMT {
        // XXX
        let mut sizepb = fs.fs_bsize() as u64; // XXX
        let mut maxfilesize = (i64::from(fs.fs_bsize()) * NDADDR as i64 - 1) as u64; // XXX
        for _ in 0..NIADDR {
            // XXX
            sizepb = sizepb.wrapping_mul(nindir(fs) as u64); // XXX
            maxfilesize = maxfilesize.wrapping_add(sizepb); // XXX
        } // XXX
        fs.set_fs_maxfilesize(maxfilesize);
        fs.set_fs_qbmask(i64::from(!fs.fs_bmask())); // XXX
        fs.set_fs_qfmask(i64::from(!fs.fs_fmask())); // XXX
    } // XXX
    if fs.fs_avgfilesize() == 0 {
        // XXX (`<= 0` of a `u_int32_t`)
        fs.set_fs_avgfilesize(AVFILESIZ); // XXX
    }
    if fs.fs_avgfpdir() == 0 {
        // XXX
        fs.set_fs_avgfpdir(AFPDIR); // XXX
    }
    0
}

/// `ffs1_compat_read`: auxiliary function for reading FFS1 super blocks: the old members'
/// values into the FFS2 ones the kernel uses.
pub fn ffs1_compat_read<G: ProcGlue>(fs: &Superblock, _ump: &Ufsmount<G>, sbloc: Daddr) {
    if fs.fs_magic() == FS_UFS2_MAGIC {
        return; // UFS2
    }
    // fs_flags = fs_ffs1_flags: an int8_t into a u_int32_t, sign-extended.
    fs.set_fs_flags(i32::from(fs.fs_ffs1_flags()) as u32);
    fs.set_fs_sblockloc(sbloc);
    fs.set_fs_maxbsize(fs.fs_bsize());
    fs.set_fs_time(i64::from(fs.fs_ffs1_time()));
    fs.set_fs_size(i64::from(fs.fs_ffs1_size()));
    fs.set_fs_dsize(i64::from(fs.fs_ffs1_dsize()));
    fs.set_fs_csaddr(i64::from(fs.fs_ffs1_csaddr()));
    fs.set_fs_cstotal_ndir(i64::from(fs.fs_ffs1_cstotal_ndir()));
    fs.set_fs_cstotal_nbfree(i64::from(fs.fs_ffs1_cstotal_nbfree()));
    fs.set_fs_cstotal_nifree(i64::from(fs.fs_ffs1_cstotal_nifree()));
    fs.set_fs_cstotal_nffree(i64::from(fs.fs_ffs1_cstotal_nffree()));
    fs.set_fs_ffs1_flags((fs.fs_ffs1_flags() as u8 | FS_FLAGS_UPDATED as u8) as i8);
}

/// `ffs1_compat_write`: auxiliary function for writing FFS1 super blocks, over the
/// super-block's bytes in the buffer `ffs_sbupdate` writes (`dfs`).
pub fn ffs1_compat_write<G: ProcGlue>(dfs: &mut [u8], _ump: &Ufsmount<G>) {
    if i32::from_ne_bytes(get_bytes(dfs, offset_of!(Fs, fs_magic))) != FS_UFS1_MAGIC {
        return; // UFS2
    }

    let time = i64::from_ne_bytes(get_bytes(dfs, offset_of!(Fs, fs_time)));
    put_bytes(
        dfs,
        offset_of!(Fs, fs_ffs1_time),
        &(time as i32).to_ne_bytes(),
    );
    for (new, old) in [
        (
            offset_of!(Fs, fs_cstotal.cs_ndir),
            offset_of!(Fs, fs_ffs1_cstotal.cs_ndir),
        ),
        (
            offset_of!(Fs, fs_cstotal.cs_nbfree),
            offset_of!(Fs, fs_ffs1_cstotal.cs_nbfree),
        ),
        (
            offset_of!(Fs, fs_cstotal.cs_nifree),
            offset_of!(Fs, fs_ffs1_cstotal.cs_nifree),
        ),
        (
            offset_of!(Fs, fs_cstotal.cs_nffree),
            offset_of!(Fs, fs_ffs1_cstotal.cs_nffree),
        ),
    ] {
        let v = i64::from_ne_bytes(get_bytes(dfs, new));
        put_bytes(dfs, old, &(v as i32).to_ne_bytes());
    }
}

/// The `N` bytes of the buffer's super-block at `off` (zero past the buffer's end).
fn get_bytes<const N: usize>(b: &[u8], off: usize) -> [u8; N] {
    let mut v = [0u8; N];
    if off < b.len() {
        let n = N.min(b.len() - off);
        v[..n].copy_from_slice(&b[off..off + n]);
    }
    v
}

/// `v` written over the buffer's super-block at `off`: the bytes of it that fall inside the
/// buffer (a member past `fs_sbsize` is a write the C makes past `b_bcount`, which never
/// reaches the disk).
fn put_bytes(b: &mut [u8], off: usize, v: &[u8]) {
    if off < b.len() {
        let n = v.len().min(b.len() - off);
        b[off..off + n].copy_from_slice(&v[..n]);
    }
}

/// `ffs_unmount` (`vfs_unmount`): unmount system call.
pub fn ffs_unmount<G: ProcGlue>(
    mp: &'static Mount<G>,
    mntflags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut flags = 0;
    if mntflags & MNT_FORCE != 0 {
        flags |= FORCECLOSE;
    }

    let ump = vfstoufs(mp);
    let fs = ump.um_fs();
    ffs_flushfiles(mp, flags, p)?;

    if fs.fs_ronly() == 0 {
        fs.set_fs_clean(i8::from(fs.fs_flags() & FS_UNCLEAN == 0));
        let error = ffs_sbupdate(&ump, MNT_WAIT);
        // ignore write errors if mounted RW on read-only device
        if let Err(e) = error
            && e != Errno::EROFS
        {
            fs.set_fs_clean(0);
            return Err(e);
        }
        fs.contigdirs_free();
    }
    let devvp = ump.devvp();
    devvp.set_specmountpoint(None);

    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let _ = vinvalbuf(devvp, V_SAVE, Cred::NOCRED, Some(p), 0, INFSLP);
    let omode = if fs.fs_ronly() != 0 {
        FREAD
    } else {
        FREAD | FWRITE
    };
    let _ = vop_close(devvp, omode, Cred::NOCRED, Some(p));
    vput(devvp);
    // free(fs->fs_csp); free(fs); free(ump); mp->mnt_data = NULL: the mount's handle goes;
    // the inodes' went with `vflush`, so this call's, dropped on return, is the last and frees
    // the structure with its super-block and summaries.
    drop(mp.take_node());
    mp.set_flag(mp.flag() & !MNT_LOCAL);
    Ok(())
}

/// `ffs_flushfiles`: flush out all the files in a filesystem.
pub fn ffs_flushfiles<G: ProcGlue>(
    mp: &'static Mount<G>,
    flags: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    if mp.flag() & MNT_QUOTA != 0 {
        vflush(mp, None, SKIPSYSTEM | flags)?;
        for i in 0..MAXQUOTAS {
            if ump.um_quotas[i].get().is_none() {
                continue;
            }
            let _ = quotaoff(p, mp, i);
        }
        // Here we fall through to vflush again to ensure that we have gotten rid of all the
        // system vnodes.
    }

    // Flush all the files.
    vflush(mp, None, flags)?;
    // Flush filesystem metadata.
    let devvp = ump.devvp();
    let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
    let error = vop_fsync(devvp, Cred::real(p.ucred()), MNT_WAIT, p);
    let _ = vop_unlock(devvp);
    error
}

/// `ffs_statfs` (`vfs_statfs`): get file system statistics.
pub fn ffs_statfs<G: ProcGlue>(
    mp: &'static Mount<G>,
    sbp: &mut Statfs,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    let fs = ump.um_fs();

    #[cfg(feature = "ffs2")]
    if fs.fs_magic() != FS_MAGIC && fs.fs_magic() != FS_UFS2_MAGIC {
        kpanic(format_args!("ffs_statfs"));
    }
    #[cfg(not(feature = "ffs2"))]
    if fs.fs_magic() != FS_MAGIC {
        kpanic(format_args!("ffs_statfs"));
    }

    sbp.f_bsize = fs.fs_fsize() as u32;
    sbp.f_iosize = fs.fs_bsize() as u32;
    sbp.f_blocks = fs.fs_dsize() as u64;
    sbp.f_bfree =
        (fs.fs_cstotal_nbfree() * i64::from(fs.fs_frag()) + fs.fs_cstotal_nffree()) as u64;
    sbp.f_bavail = sbp.f_bfree as i64 - (fs.fs_dsize() * i64::from(fs.fs_minfree()) / 100);
    sbp.f_files = u64::from(fs.fs_ncg().wrapping_mul(fs.fs_ipg()).wrapping_sub(ROOTINO));
    sbp.f_ffree = fs.fs_cstotal_nifree() as u64;
    sbp.f_favail = sbp.f_ffree as i64;
    copy_statfs_info(sbp, mp);

    Ok(())
}

/// `ffs_sync_vnode`: write back one (modified) inode for [`ffs_sync`] (the C's callback
/// always answers 0).
fn ffs_sync_vnode<G: ProcGlue>(vp: &'static Vnode<G>, fsa: &mut FfsSyncArgs<'_, G>) {
    if vp.vtype() == VNON {
        return;
    }

    let ip = vtoi(vp);
    let mut nlink0 = 0;

    // If unmounting or converting rw to ro, then stop deferring timestamp writes.
    if fsa.waitfor == MNT_WAIT && ip.i_flag.get() & IN_LAZYMOD != 0 {
        ip.set_flag(IN_MODIFIED);
        let _ = ip.vtbl().update(&ip, 1);
    }

    if ip.i_effnlink.get() == 0 {
        nlink0 = 1;
    }

    let s = splraise(IPL_BIO);
    let skip = ip.i_flag.get() & (IN_ACCESS | IN_CHANGE | IN_MODIFIED | IN_UPDATE) == 0
        && vp.bufs.dirty_is_empty();
    drop(s);
    drop(ip);

    'end: {
        if skip {
            break 'end;
        }

        if vget(vp, LK_EXCLUSIVE | LK_NOWAIT).is_err() {
            fsa.inflight = (fsa.inflight + 1).min(65536);
            break 'end;
        }

        if let Err(e) = vop_fsync(vp, fsa.cred, fsa.waitfor, fsa.p) {
            fsa.allerror = Err(e);
        }
        let _ = vop_unlock(vp);
        vrele(vp);
    }

    // end:
    fsa.nlink0 = (fsa.nlink0 + nlink0).min(65536);
}

/// `ffs_sync` (`vfs_sync`): go through the disk queues to initiate sandbagged IO; go
/// through the inodes to write those that have been modified; initiate the writing of the
/// super block if it has been modified.
///
/// Should always be called with the mount point locked.
pub fn ffs_sync<G: ProcGlue>(
    mp: &'static Mount<G>,
    waitfor: i32,
    stall: i32,
    cred: Cred<'_>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let ump = vfstoufs(mp);
    let fs = ump.um_fs();
    let mut allerror = Ok(());
    // Write back modified superblock. Consistency check that the superblock is still in the
    // buffer cache.
    if fs.fs_fmod() != 0 && fs.fs_ronly() != 0 {
        printf(format_args!("fs = {}\n", fs.fsmnt()));
        kpanic(format_args!("update: rofs mod"));
    }

    // Write back each (modified) inode.
    let mut fsa = FfsSyncArgs {
        allerror: Ok(()),
        p,
        cred,
        waitfor,
        nlink0: 0,
        inflight: 0,
    };

    // Don't traverse the vnode list if we want to skip all of them.
    if waitfor != MNT_LAZY {
        let _ = vfs_mount_foreach_vnode(mp, &mut |vp| {
            ffs_sync_vnode(vp, &mut fsa);
            Ok(())
        });
        allerror = fsa.allerror;
    }

    // Force stale file system control information to be flushed.
    if waitfor != MNT_LAZY {
        let devvp = ump.devvp();
        let _ = vn_lock(devvp, LK_EXCLUSIVE | LK_RETRY);
        if let Err(e) = vop_fsync(devvp, cred, waitfor, p) {
            allerror = Err(e);
        }
        let _ = vop_unlock(devvp);
    }
    let _ = qsync(mp);
    // Write back modified superblock.
    let clean = fs.fs_clean();
    let fmod = fs.fs_fmod();
    if stall != 0 && fs.fs_ronly() == 0 {
        fs.set_fs_fmod(1);
        if allerror.is_ok() && fsa.nlink0 == 0 && fsa.inflight == 0 {
            fs.set_fs_clean(i8::from(fs.fs_flags() & FS_UNCLEAN == 0));
        } else {
            fs.set_fs_clean(0);
        }
    }
    if fs.fs_fmod() != 0
        && let Err(e) = ffs_sbupdate(&ump, waitfor)
    {
        allerror = Err(e);
    }
    fs.set_fs_clean(clean);
    fs.set_fs_fmod(fmod);

    allerror
}

/// `ffs_vget` (`vfs_vget`): look up a FFS dinode number to find its incore vnode, otherwise
/// read it in from disk. If it is in core, wait for the lock bit to clear, then return the
/// inode locked. Detection and handling of mount points must be done by the calling routine.
pub fn ffs_vget<G: ProcGlue>(mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
    if ino > u64::from(Ufsino::MAX) {
        kpanic(format_args!("ffs_vget: alien ino_t {ino}"));
    }
    let ino = ino as Ufsino;

    let ump = vfstoufs(mp);
    let dev = ump.um_dev.get();
    let tables = &vfs::<G>().ufs;
    loop {
        // retry:
        if let Some(vp) = ufs_ihashget::<G>(dev, ino) {
            return Ok(vp);
        }

        // Allocate a new vnode/inode.
        let vp = getnewvnode(VT_UFS, Some(mp), &tables.ffs_vops)?;

        let fs = ump.um_fs();
        let Some(ip) = tables.ffs_ino.get_homed(PR_WAITOK, || {
            Inode::new(InodeInit {
                vnode: Some(vp),
                mount: Some(mp),
                ump: Some(ump.clone()),
                devvp: Some(ump.devvp()),
                dev,
                number: ino,
                fstype: ump.um_fstype.get(),
                nindir: ump.um_nindir.get(),
                bptrtodb: ump.um_bptrtodb.get(),
                seqinc: ump.um_seqinc.get(),
                vtbl: Some(&Ffs::<G>::VTBL),
                ..InodeInit::default()
            })
        }) else {
            kpanic(format_args!("ffs_vget: no inode"));
        };
        vref(ump.devvp());
        vp.set_node(PoolArc::erase(ip.clone()));

        // Put it onto its hash chain and lock it so that other requests for this inode will
        // block if they arrive while we are sleeping waiting for old data structures to be
        // purged or for the contents of the disk portion of this inode to be read.
        if let Err(error) = ufs_ihashins(ip.as_pin()) {
            drop(ip);
            // VOP_INACTIVE will treat this as a stale file and recycle it quickly
            vrele(vp);

            if error == Errno::EEXIST {
                continue;
            }

            return Err(error);
        }

        // Read in the disk contents for the inode, copy into the inode.
        let (mut t, error) = bread(
            &ump.devvp().bufs,
            fsbtodb(fs, ino_to_fsba(fs, ino)),
            fs.fs_bsize(),
        );
        if let Err(e) = error {
            // The inode does not contain anything useful, so it would be misleading to leave
            // it on its hash chain. With mode still zero, it will be unlinked and returned
            // to the free list by vput().
            drop(ip);
            vput(vp);
            brelse(t);
            return Err(e);
        }

        let idx = ino_to_fsbo(fs, ino);
        #[cfg(feature = "ffs2")]
        let ufs2 = ump.um_fstype.get() == UM_UFS2;
        #[cfg(not(feature = "ffs2"))]
        let ufs2 = false;
        if ufs2 {
            #[cfg(feature = "ffs2")]
            {
                let d = dinode2_at(t.data(), idx);
                let Some(d) = tables.ffs_dino2.get(PR_WAITOK, || Ufs2Din::new(&d)) else {
                    kpanic(format_args!("ffs_vget: no dinode"));
                };
                ip.set_din(Dinode::Ufs2(d));
            }
        } else {
            let d = dinode1_at(t.data(), idx);
            let Some(d) = tables.ffs_dino1.get(PR_WAITOK, || Ufs1Din::new(&d)) else {
                kpanic(format_args!("ffs_vget: no dinode"));
            };
            ip.set_din(Dinode::Ufs1(d));
        }

        brelse(t);

        ip.i_effnlink.set(ip.dip_nlink());

        // Initialize the vnode from the inode, check for aliases. Note that the underlying
        // vnode may have changed.
        let vp = match ffs_vinit(mp, vp) {
            Ok(vp) => vp,
            Err(e) => {
                drop(ip);
                vput(vp);
                return Err(e);
            }
        };

        // Set up a generation number for this inode if it does not already have one. This
        // should only happen on old filesystems.
        if ip.dip_gen() == 0 {
            while ip.dip_gen() == 0 {
                ip.dip_set_gen(arc4random());
            }
            if vp.mount().is_some_and(|m| m.flag() & MNT_RDONLY == 0) {
                ip.set_flag(IN_MODIFIED);
            }
        }

        // Ensure that uid and gid are correct. This is a temporary fix until fsck has been
        // changed to do the update.
        if fs.fs_magic() == FS_UFS1_MAGIC && fs.fs_inodefmt() < FS_44INODEFMT {
            let d = ip.din1();
            ip.dip_set_uid(u32::from(d.di_ouid()));
            ip.dip_set_gid(u32::from(d.di_ogid()));
        }

        return Ok(vp);
    }
}

/// `ffs_fhtovp` (`vfs_fhtovp`): file handle to vnode. Have to be really careful about stale
/// file handles.
pub fn ffs_fhtovp<G: ProcGlue>(
    mp: &'static Mount<G>,
    fhp: &Fid,
) -> Result<&'static Vnode<G>, Errno> {
    let ufhp = Ufid::from_fid(fhp);
    if usize::from(ufhp.ufid_len) != size_of::<Ufid>() {
        return Err(Errno::EINVAL);
    }

    ffs_checkrange(mp, ufhp.ufid_ino)?;

    ufs_fhtovp(mp, &ufhp)
}

/// `ffs_vptofh` (`vfs_vptofh`): vnode pointer to file handle.
pub fn ffs_vptofh<G: ProcGlue>(vp: &'static Vnode<G>, fhp: &mut Fid) -> Result<(), Errno> {
    let ip = vtoi(vp);
    Ufid {
        ufid_len: size_of::<Ufid>() as u16,
        ufid_pad: fhp.fid_reserved,
        ufid_ino: ip.i_number,
        ufid_gen: ip.dip_gen(),
    }
    .to_fid(fhp);

    Ok(())
}

/// `ffs_sbupdate`: write a superblock and associated information back to disk: the summary
/// space a block at a time, then the copy in core's `fs_sbsize` bytes with the
/// compatibility rewrites of old file systems.
pub fn ffs_sbupdate<G: ProcGlue>(mp: &Ufsmount<G>, waitfor: i32) -> Result<(), Errno> {
    let fs = mp.um_fs();
    let devvp = mp.devvp();
    let mut allerror = Ok(());

    // First write back the summary information.
    let blks = howmany(fs.fs_cssize(), fs.fs_fsize());
    let mut space = 0;
    let mut i = 0;
    while i < blks {
        let mut size = fs.fs_bsize();
        if i + fs.fs_frag() > blks {
            size = (blks - i) * fs.fs_fsize();
        }
        let mut t = getblk_wait(
            &devvp.bufs,
            fsbtodb(fs, fs.fs_csaddr() + Daddr::from(i)),
            size,
        );
        let n = size as usize;
        fs.read_csum_bytes(space, &mut t.data()[..n]);
        space += n;
        if waitfor != MNT_WAIT {
            bawrite(t);
        } else if let Err(e) = bwrite(t) {
            allerror = Err(e);
        }
        i += fs.fs_frag();
    }

    // Now write back the superblock itself. If any errors occurred up to this point, then
    // fail so that the superblock avoids being written out as clean.
    allerror?;

    let mut t = getblk_wait(
        &devvp.bufs,
        fs.fs_sblockloc() >> (fs.fs_fshift() - fs.fs_fsbtodb()),
        fs.fs_sbsize(),
    );
    fs.set_fs_fmod(0);
    fs.set_fs_time(gettime());
    {
        let dfs = t.data();
        let n = (fs.fs_sbsize().max(0) as usize).min(dfs.len());
        fs.read_bytes(0, &mut dfs[..n]);
        // Restore compatibility to old file systems. XXX
        if fs.fs_postblformat() == FS_42POSTBLFMT {
            // XXX
            put_bytes(dfs, offset_of!(Fs, fs_nrpos), &(-1i32).to_ne_bytes()); // XXX
        }
        if fs.fs_inodefmt() < FS_44INODEFMT {
            // XXX
            // The five 32-bit words from fs_qbmask on rotate by one. XXX
            let base = offset_of!(Fs, fs_qbmask);
            let lp: [[u8; 4]; 5] = core::array::from_fn(|k| get_bytes(dfs, base + 4 * k));
            for (k, w) in [lp[4], lp[0], lp[1], lp[2], lp[3]].iter().enumerate() {
                put_bytes(dfs, base + 4 * k, w);
            }
        } // XXX
        put_bytes(
            dfs,
            offset_of!(Fs, fs_maxfilesize),
            &mp.um_savedmaxfilesize.get().to_ne_bytes(),
        ); // XXX

        ffs1_compat_write(dfs, mp);
    }

    if waitfor == MNT_WAIT {
        bwrite(t)
    } else {
        bawrite(t);
        Ok(())
    }
}

/// `ffs_init` (`vfs_init`): the inode and dinode pools, once, then the UFS layer.
pub fn ffs_init<G: ProcGlue>(_vfsp: &'static Vfsconf<G>) -> Result<(), Errno> {
    let t = &vfs::<G>().ufs;
    if t.ffs_init_done.swap(true, Ordering::Relaxed) {
        return Ok(());
    }

    // A second call (a host test's, `ufs_test_reset` cleared the flag) keeps the pools made
    // first.
    let _ = t.ffs_ino.init_recorded(IPL_NONE, PR_WAITOK, "ffsino");
    let _ = t.ffs_dino1.init(IPL_NONE, PR_WAITOK, "dino1pl");
    #[cfg(feature = "ffs2")]
    let _ = t.ffs_dino2.init(IPL_NONE, PR_WAITOK, "dino2pl");

    ufs_init::<G>(vfs::<G>().initialvnodes.load(Ordering::Relaxed))
}

/// `ffs_sysctl` (`vfs_sysctl`): fast filesystem related variables (`ffs_vars[]`).
pub fn ffs_sysctl<G: ProcGlue>(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let mut req = SysctlReq {
        oldp,
        oldlenp,
        newp,
        newlen,
    };
    bounded_arr(&FFS_VARS, name, &mut req)
}

/// `ffs_vfsops`: each slot the C's function of the same name.
impl<G: ProcGlue> Vfs<G> for Ffs<G> {
    /// `vfs_mount`: [`ffs_mount`].
    fn mount(
        &self,
        mp: &'static Mount<G>,
        path: &[u8],
        data: &mut [u8],
        ndp: &mut Nd<'_, G>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ffs_mount(mp, path, data, ndp, p)
    }

    /// `vfs_start`: `ufs_start`.
    fn start(&self, mp: &'static Mount<G>, flags: i32, p: &Thread<G>) -> Result<(), Errno> {
        ufs_start(mp, flags, p)
    }

    /// `vfs_unmount`: [`ffs_unmount`].
    fn unmount(&self, mp: &'static Mount<G>, mntflags: i32, p: &Thread<G>) -> Result<(), Errno> {
        ffs_unmount(mp, mntflags, p)
    }

    /// `vfs_root`: `ufs_root`.
    fn root(&self, mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        ufs_root(mp)
    }

    /// `vfs_quotactl`: `ufs_quotactl`.
    fn quotactl(
        &self,
        mp: &'static Mount<G>,
        cmds: i32,
        uid: Uid,
        arg: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ufs_quotactl(mp, cmds, uid, arg, p)
    }

    /// `vfs_statfs`: [`ffs_statfs`].
    fn statfs(&self, mp: &'static Mount<G>, sbp: &mut Statfs, p: &Thread<G>) -> Result<(), Errno> {
        ffs_statfs(mp, sbp, p)
    }

    /// `vfs_sync`: [`ffs_sync`].
    fn sync(
        &self,
        mp: &'static Mount<G>,
        waitfor: i32,
        stall: i32,
        cred: Cred<'_>,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ffs_sync(mp, waitfor, stall, cred, p)
    }

    /// `vfs_vget`: [`ffs_vget`].
    fn vget(&self, mp: &'static Mount<G>, ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        ffs_vget(mp, ino)
    }

    /// `vfs_fhtovp`: [`ffs_fhtovp`].
    fn fhtovp(&self, mp: &'static Mount<G>, fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        ffs_fhtovp(mp, fid)
    }

    /// `vfs_vptofh`: [`ffs_vptofh`].
    fn vptofh(&self, vp: &'static Vnode<G>, fid: &mut Fid) -> Result<(), Errno> {
        ffs_vptofh(vp, fid)
    }

    /// `vfs_init`: [`ffs_init`].
    fn init(&self, vfc: &'static Vfsconf<G>) -> Result<(), Errno> {
        ffs_init(vfc)
    }

    /// `vfs_sysctl`: [`ffs_sysctl`].
    fn sysctl(
        &self,
        name: &[i32],
        oldp: usize,
        oldlenp: &mut usize,
        newp: usize,
        newlen: usize,
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        ffs_sysctl(name, oldp, oldlenp, newp, newlen, p)
    }
}

/// `ffs_vtbl`: each slot the C's function of the same name.
impl<G: ProcGlue> InodeOps<G> for Ffs<G> {
    /// `iv_truncate`: `ffs_truncate`.
    fn truncate(&self, ip: &Inode<G>, off: Off, flags: i32, cred: Cred<'_>) -> Result<(), Errno> {
        ffs_truncate(ip, off, flags, cred)
    }

    /// `iv_update`: `ffs_update`.
    fn update(&self, ip: &Inode<G>, waitfor: i32) -> Result<(), Errno> {
        ffs_update(ip, waitfor)
    }

    /// `iv_inode_alloc`: `ffs_inode_alloc`.
    fn inode_alloc(
        &self,
        pip: &Inode<G>,
        mode: Mode,
        cred: Cred<'_>,
    ) -> Result<&'static Vnode<G>, Errno> {
        ffs_inode_alloc(pip, mode, cred)
    }

    /// `iv_inode_free`: `ffs_inode_free`.
    fn inode_free(&self, pip: &Inode<G>, ino: Ufsino, mode: Mode) -> Result<(), Errno> {
        ffs_inode_free(pip, ino, mode)
    }

    /// `iv_buf_alloc`: `ffs_balloc`.
    fn buf_alloc(
        &self,
        ip: &Inode<G>,
        startoffset: Off,
        size: i32,
        cred: Cred<'_>,
        flags: i32,
    ) -> Result<IoBuf, Errno> {
        ffs_balloc(ip, startoffset, size, cred, flags).map(IoBuf::Token)
    }

    /// `iv_bufatoff`: `ffs_bufatoff`.
    fn bufatoff(&self, ip: &Inode<G>, offset: Off) -> Result<(IoBuf, usize), Errno> {
        ffs_bufatoff(ip, offset).map(|(t, off)| (IoBuf::Token(t), off))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests over testffs's images (a block device of testfs whose strategy serves the
    // image): the super-block checks and the mount's, the copy in core and its write-back byte
    // for byte, the mount updates, the reload, statfs, sync, file handles, vget and the
    // sysctl names; then an image mounted at `/` and used through the system calls
    // (sys_open, read, write, mkdir, rename, rmdir, unlink, symlink, readlink, getdents,
    // sync), unmounted (`dounmount`), checked as `fsck` would and mounted again. User memory:
    // USER_BASE + 0xb000..0xd000 (four path slots, then a buffer), which no other test of the
    // binary uses (the tests hold MEMORY: one at a time).

    use std::vec;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne};

    use hal::sys::fcntl::{O_CREAT, O_RDONLY, O_RDWR};

    use super::*;
    use crate::descrip::{sys_close, sys_read, sys_write};
    use crate::fs::mount::vfs_mount_alloc;
    use crate::fs::syscalls::{
        sys_fsync, sys_getdents, sys_mkdir, sys_open, sys_readlink, sys_rename, sys_rmdir,
        sys_symlink, sys_sync, sys_unlink,
    };
    use crate::fs::testfs::TestGlue;
    use crate::fs::ufs::ffs::fs::{SBLOCK_UFS1, cgtod};
    use crate::fs::ufs::ffs::testffs::newfs::{
        Din, FFS1_4M, FFS2_4M, Image, Params, check, super_block,
    };
    use crate::fs::ufs::ffs::testffs::{
        attach, devvp, disk, mount, mount_root, new_file, pattern, root, set_disk, setup,
        unmount_root,
    };
    use crate::fs::ufs::ufs::inode::vtoi;
    use crate::testutil::USER_BASE;

    /// The path slots: four of 256 bytes.
    const UPATH: usize = USER_BASE + 0xb000;
    /// The buffer, to USER_BASE + 0xd000.
    const UBUF: usize = USER_BASE + 0xb400;
    /// The buffer's length.
    const UBUF_LEN: usize = 0xd000 - 0xb400;

    /// The formats the tests build: FFS2's only with `option FFS2`.
    const FORMATS: &[Params] = if cfg!(feature = "ffs2") {
        &[FFS2_4M, FFS1_4M]
    } else {
        &[FFS1_4M]
    };

    /// The super-block's byte offset in an image of the format.
    fn sbloc(ufs2: bool) -> usize {
        (if ufs2 { SBLOCK_UFS2 } else { SBLOCK_UFS1 }) as usize
    }

    /// `v` written at the super-block's member at `off` in `img`.
    fn poke(img: &mut [u8], ufs2: bool, off: usize, v: &[u8]) {
        let at = sbloc(ufs2) + off;
        img[at..at + v.len()].copy_from_slice(v);
    }

    /// The NUL-terminated `s` in user memory, in slot `i`.
    fn up(i: usize, s: &[u8]) -> UserPtr<u8> {
        assert!(s.len() < 256 && i < 4);
        let at = UPATH + i * 256;
        let mut b = s.to_vec();
        b.push(0);
        UserPtr::<u8>::new(at).copyout_slice(&b).unwrap();
        UserPtr::new(at)
    }

    /// `s` as the first path argument.
    fn path(s: &[u8]) -> UserPtr<u8> {
        up(0, s)
    }

    /// The whole contents of the file at `name`.
    fn read_file(p: &Thread<TestGlue>, name: &[u8]) -> Result<Vec<u8>, Errno> {
        let fd = sys_open(p, path(name), O_RDONLY, 0)?;
        let mut out = Vec::new();
        loop {
            let n = sys_read(p, fd, UBUF, 3000)?;
            if n == 0 {
                break;
            }
            let mut b = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
            out.extend_from_slice(&b);
        }
        sys_close(p, fd)?;
        Ok(out)
    }

    /// Creates `name` (new) with `data`, in uneven pieces that cross block and fragment
    /// boundaries, and `fsync`s it.
    fn write_file(p: &Thread<TestGlue>, name: &[u8], data: &[u8]) -> Result<(), Errno> {
        let fd = sys_open(p, path(name), O_RDWR | O_CREAT, 0o644)?;
        for chunk in data.chunks(7001.min(UBUF_LEN)) {
            UserPtr::<u8>::new(UBUF).copyout_slice(chunk).unwrap();
            let n = sys_write(p, fd, UBUF, chunk.len())?;
            assert_eq!(n, chunk.len());
        }
        sys_fsync(p, fd)?;
        sys_close(p, fd)?;
        Ok(())
    }

    /// The names in the directory `name` (without `.` and `..`), sorted.
    fn list_dir(p: &Thread<TestGlue>, name: &[u8]) -> Vec<Vec<u8>> {
        let fd = sys_open(p, path(name), O_RDONLY, 0).unwrap();
        let mut names = Vec::new();
        loop {
            let n = sys_getdents(p, fd, UBUF, 4096).unwrap();
            if n == 0 {
                break;
            }
            let mut buf = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut buf).unwrap();
            let mut off = 0;
            while off < n {
                let reclen = u16::from_ne_bytes([buf[off + 16], buf[off + 17]]) as usize;
                let namlen = buf[off + 19] as usize;
                let nm = &buf[off + 24..off + 24 + namlen];
                if nm != b"." && nm != b".." {
                    names.push(nm.to_vec());
                }
                off += reclen;
            }
        }
        sys_close(p, fd).unwrap();
        names.sort();
        names
    }

    /// A `Nameidata` for `ffs_mount`'s `ndp` (an update reads its credential).
    fn nd(p: &Thread<TestGlue>) -> Nd<'_, TestGlue> {
        ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/ffs"), p)
    }

    #[test]
    fn ffs_validate_takes_the_c_checks() {
        for &params in FORMATS {
            let fs = super_block(&Image::new(params).finish(), params.ufs2);
            assert!(ffs_validate(&fs));
            let cases: [(&str, fn(&mut Fs)); 9] = [
                ("magic", |f| f.fs_magic = 0x1234),
                ("bsize past MAXBSIZE", |f| f.fs_bsize = 1 << 17),
                ("bsize under struct fs", |f| f.fs_bsize = 1024),
                ("a negative bsize, unsigned", |f| f.fs_bsize = -8192),
                ("sbsize past SBSIZE", |f| f.fs_sbsize = SBSIZE as i32 + 1),
                ("frag past MAXFRAG", |f| f.fs_frag = 16),
                ("frag without a fragtbl", |f| f.fs_frag = 3),
                ("4.2BSD inodes", |f| f.fs_inodefmt = FS_42INODEFMT),
                ("no short symlinks", |f| f.fs_maxsymlinklen = 0),
            ];
            for (what, f) in cases {
                let mut bad = fs;
                f(&mut bad);
                assert!(!ffs_validate(&bad), "{what}");
            }
        }
    }

    /// `ffs_sbupdate`'s super-block as the C writes it: the copy in core's `fs_sbsize` bytes,
    /// then `fs_maxfilesize` from `um_savedmaxfilesize` and, for FFS1, the old members from
    /// the new ones.
    fn written_super_block(ump: &Ufsmount<TestGlue>) -> Vec<u8> {
        let fs = ump.um_fs();
        let mut b = vec![0u8; fs.fs_sbsize() as usize];
        fs.read_bytes(0, &mut b);
        put_bytes(
            &mut b,
            offset_of!(Fs, fs_maxfilesize),
            &ump.um_savedmaxfilesize.get().to_ne_bytes(),
        );
        ffs1_compat_write(&mut b, ump);
        b
    }

    #[test]
    fn a_read_write_mount_writes_the_super_block_dirty_and_the_unmount_clean() {
        for &params in FORMATS {
            let mut img = Image::new(params);
            img.add_file(b"motd", b"hello, world\n");
            let (_g, p, mp) = setup(img.finish());
            let ump = vfstoufs(mp);
            let fs = ump.um_fs();
            assert_eq!(fs.fs_ronly(), 0);
            assert_eq!(
                fs.fs_clean(),
                0,
                "a read-write mount marks the file system dirty"
            );
            assert_eq!(fs.fs_fmod(), 0, "and writes it");
            assert_eq!(fs.fs_csp_slot(), 0, "the in-core slots hold 0");
            assert_eq!(fs.fs_contigdirs_slot(), 0);
            assert_eq!(fs.fsmnt().as_bytes(), b"/ffs");
            assert_eq!(
                ump.um_fstype.get(),
                if params.ufs2 { UM_UFS2_OR_1 } else { UM_UFS1 }
            );

            // The disk's super-block and summaries are the copy in core's, byte for byte.
            let d = disk();
            let at = sbloc(params.ufs2);
            let sb = written_super_block(&ump);
            assert_eq!(&d[at..at + sb.len()], &sb[..]);
            assert_eq!(super_block(&d, params.ufs2).fs_clean, 0);
            let cs = (fsbtodb(fs, fs.fs_csaddr()) * DEV_BSIZE) as usize;
            let mut sums = vec![0u8; fs.csum_len()];
            fs.read_csum_bytes(0, &mut sums);
            assert_eq!(&d[cs..cs + sums.len()], &sums[..]);

            ffs_unmount(mp, 0, p).unwrap();
            let d = disk();
            assert_eq!(super_block(&d, params.ufs2).fs_clean, 1, "clean again");
            check(&d, params.ufs2);
            assert!(!mp.has_node(), "the mount's handle went");
            assert!(devvp().specmountpoint().is_none());
        }
    }

    /// `UM_UFS2` with `option FFS2` (the FFS2 images are built only then).
    #[cfg(feature = "ffs2")]
    const UM_UFS2_OR_1: u64 = UM_UFS2;
    /// Without `option FFS2` no FFS2 image is mounted.
    #[cfg(not(feature = "ffs2"))]
    const UM_UFS2_OR_1: u64 = UM_UFS1;

    #[test]
    fn an_unclean_file_system_mounts_read_only_or_forced_and_stays_unclean() {
        for &params in FORMATS {
            let mut img = Image::new(params).finish();
            poke(&mut img, params.ufs2, offset_of!(Fs, fs_clean), &[0]);
            let (_g, p) = attach(img);
            assert_eq!(mount(p, 0).err(), Some(Errno::EROFS), "R/W mount denied");
            assert!(devvp().specmountpoint().is_none());

            let mp = mount(p, MNT_RDONLY).unwrap();
            let ump = vfstoufs(mp);
            assert_eq!(ump.um_fs().fs_ronly(), 1);
            assert_ne!(
                ump.um_fs().fs_flags() & FS_UNCLEAN,
                0,
                "remembered as unclean"
            );
            drop(ump);
            ffs_unmount(mp, 0, p).unwrap();
            assert_eq!(
                super_block(&disk(), params.ufs2).fs_clean,
                0,
                "nothing written"
            );

            let mp = mount(p, MNT_FORCE).unwrap();
            assert_eq!(vfstoufs(mp).um_fs().fs_ronly(), 0);
            ffs_unmount(mp, 0, p).unwrap();
            assert_eq!(
                super_block(&disk(), params.ufs2).fs_clean,
                0,
                "a forced mount of an unclean file system leaves it unclean"
            );
        }
    }

    #[test]
    fn no_super_block_is_einval_and_a_mounted_device_ebusy() {
        let (_g, p) = attach(vec![0u8; FFS1_4M.size]);
        assert_eq!(mount(p, 0).err(), Some(Errno::EINVAL));

        let params = FORMATS[0];
        set_disk(Image::new(params).finish());
        let mp = mount(p, 0).unwrap();
        assert_eq!(mount(p, 0).err(), Some(Errno::EBUSY), "vfs_mountedon");
        ffs_unmount(mp, 0, p).unwrap();
    }

    #[test]
    fn an_update_to_read_only_and_back_writes_the_clean_flag() {
        for &params in FORMATS {
            let (_g, p, mp) = setup(Image::new(params).finish());
            let fs_of = || vfstoufs(mp);

            mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RDONLY);
            ffs_mount(mp, b"/ffs", &mut [], &mut nd(p), p).unwrap();
            assert_eq!(fs_of().um_fs().fs_ronly(), 1);
            assert_eq!(super_block(&disk(), params.ufs2).fs_clean, 1);

            mp.set_flag(mp.flag() | MNT_WANTRDWR);
            ffs_mount(mp, b"/ffs", &mut [], &mut nd(p), p).unwrap();
            let fs = fs_of();
            assert_eq!(fs.um_fs().fs_ronly(), 0);
            assert_eq!(fs.um_fs().fs_clean(), 0);
            assert_eq!(super_block(&disk(), params.ufs2).fs_clean, 0);
            fs.um_fs()
                .with_contigdirs(|cd| assert_eq!(cd.len(), fs.um_fs().fs_ncg() as usize));
            drop(fs);

            // A reload needs a read-only mount.
            mp.set_flag((mp.flag() & !(MNT_RDONLY | MNT_WANTRDWR)) | MNT_UPDATE | MNT_RELOAD);
            assert_eq!(
                ffs_mount(mp, b"/ffs", &mut [], &mut nd(p), p),
                Err(Errno::EINVAL)
            );
            mp.set_flag(mp.flag() & !(MNT_UPDATE | MNT_RELOAD));
            ffs_unmount(mp, 0, p).unwrap();
        }
    }

    #[test]
    fn a_new_mount_needs_arguments_and_a_block_device() {
        let (_g, p) = attach(Image::new(FORMATS[0]).finish());
        let mp = vfs_mount_alloc(None, &crate::fs::ufs::ffs::testffs::TEST_FFS_CONF);
        assert_eq!(
            ffs_mount(mp, b"/ffs", &mut [], &mut nd(p), p),
            Err(Errno::EINVAL),
            "no ufs_args"
        );
        let mut data = vec![0u8; UfsArgs::SIZE];
        let mut args = UfsArgs::from_bytes(&data).unwrap();
        for (fspec, want) in [
            (&b"/f"[..], Errno::ENOTBLK),
            (b"/dev/sd0c", Errno::ENOTBLK),
            (b"/nothere", Errno::ENOENT),
        ] {
            args.fspec = path(fspec).addr();
            assert!(hal::user::pod_to_bytes(&args, &mut data));
            assert_eq!(ffs_mount(mp, b"/ffs", &mut data, &mut nd(p), p), Err(want));
        }
        // A name without its NUL within MNAMELEN.
        UserPtr::<u8>::new(UBUF)
            .copyout_slice(&[b'x'; MNAMELEN + 1])
            .unwrap();
        args.fspec = UBUF;
        assert!(hal::user::pod_to_bytes(&args, &mut data));
        assert_eq!(
            ffs_mount(mp, b"/ffs", &mut data, &mut nd(p), p),
            Err(Errno::ENAMETOOLONG)
        );
        vfs_unbusy(mp);
        vfs_mount_free(mp);
    }

    #[test]
    fn a_reload_reads_the_super_block_and_the_summaries_again() {
        for &params in FORMATS {
            let (_g, p) = attach(Image::new(params).finish());
            let mp = mount(p, MNT_RDONLY).unwrap();
            let ump = vfstoufs(mp);
            let fs = ump.um_fs();
            let nffree = fs.fs_cstotal_nffree();
            let cs0 = fs.fs_cs(0).cs_nffree();

            // fsck changed the disk under the read-only mount: a total and a summary.
            let mut img = disk();
            let off = offset_of!(Fs, fs_cstotal.cs_nffree);
            let v = nffree + 1;
            poke(&mut img, params.ufs2, off, &v.to_ne_bytes());
            if !params.ufs2 {
                let off = offset_of!(Fs, fs_ffs1_cstotal.cs_nffree);
                poke(&mut img, false, off, &(v as i32).to_ne_bytes());
            }
            let cs = (fsbtodb(fs, fs.fs_csaddr()) * DEV_BSIZE) as usize + 12;
            img[cs..cs + 4].copy_from_slice(&(cs0 + 1).to_ne_bytes());
            set_disk(img);

            mp.set_flag(mp.flag() | MNT_UPDATE | MNT_RELOAD);
            ffs_mount(mp, b"/ffs", &mut [], &mut nd(p), p).unwrap();
            mp.set_flag(mp.flag() & !(MNT_UPDATE | MNT_RELOAD));
            assert_eq!(fs.fs_cstotal_nffree(), v);
            assert_eq!(fs.fs_cs(0).cs_nffree(), cs0 + 1);
            assert_eq!(fs.fs_ronly(), 1, "kept");
            assert_eq!(fs.fs_csp_slot(), 0, "kept");
            let st = mp.stat();
            assert_eq!(
                st.f_bfree,
                (fs.fs_cstotal_nbfree() * i64::from(fs.fs_frag()) + v) as u64,
                "statfs again"
            );
            drop(ump);
            ffs_unmount(mp, 0, p).unwrap();
        }
    }

    #[test]
    fn statfs_counts_the_space_as_the_c() {
        for &params in FORMATS {
            let mut img = Image::new(params);
            img.add_file(b"a", &pattern(20000, 3));
            let (_g, p, mp) = setup(img.finish());
            let ump = vfstoufs(mp);
            let fs = ump.um_fs();
            let mut st = Statfs::default();
            ffs_statfs(mp, &mut st, p).unwrap();
            assert_eq!(st.f_bsize, fs.fs_fsize() as u32);
            assert_eq!(st.f_iosize, fs.fs_bsize() as u32);
            assert_eq!(st.f_blocks, fs.fs_dsize() as u64);
            let bfree = fs.fs_cstotal_nbfree() * i64::from(fs.fs_frag()) + fs.fs_cstotal_nffree();
            assert_eq!(st.f_bfree, bfree as u64);
            assert_eq!(
                st.f_bavail,
                bfree - fs.fs_dsize() * i64::from(fs.fs_minfree()) / 100
            );
            assert_eq!(st.f_files, u64::from(fs.fs_ncg() * fs.fs_ipg() - ROOTINO));
            assert_eq!(st.f_ffree, fs.fs_cstotal_nifree() as u64);
            assert_eq!(st.f_favail, st.f_ffree as i64);
            let ms = mp.stat();
            assert_eq!(ms.f_fsid.val, [TEST_DEV_OF_FIXTURE, fs.fs_id()[1]]);
            assert_eq!(ms.f_namemax, MAXNAMLEN as u32);
            drop(ump);
            ffs_unmount(mp, 0, p).unwrap();
        }
    }

    /// testffs's block device.
    const TEST_DEV_OF_FIXTURE: Dev = crate::fs::ufs::ufs::testufs::TEST_DEV;

    #[test]
    fn a_stalled_sync_writes_clean_only_without_dangling_inodes() {
        for &params in FORMATS {
            let (_g, p, mp) = setup(Image::new(params).finish());
            let ump = vfstoufs(mp);
            let fs = ump.um_fs();
            let cred = Cred::real(p.ucred());
            ffs_sync(mp, MNT_WAIT, 1, cred, p).unwrap();
            assert_eq!(super_block(&disk(), params.ufs2).fs_clean, 1);
            assert_eq!((fs.fs_clean(), fs.fs_fmod()), (0, 0), "restored in core");

            // A file removed while open: its inode dangles (nlink0), so the disk stays dirty.
            let vp = new_file(mp, p);
            let ip = vtoi(vp);
            ip.dip_set_nlink(0);
            ip.i_effnlink.set(0);
            ip.set_flag(IN_CHANGE);
            drop(ip);
            let _ = vop_unlock(vp);
            ffs_sync(mp, MNT_WAIT, 1, cred, p).unwrap();
            assert_eq!(super_block(&disk(), params.ufs2).fs_clean, 0);
            vrele(vp);
            // A lazy sync walks no vnode and writes nothing unmodified.
            ffs_sync(mp, MNT_LAZY, 0, cred, p).unwrap();
            drop(ump);
            ffs_unmount(mp, 0, p).unwrap();
            check(&disk(), params.ufs2);
        }
    }

    #[test]
    fn file_handles_name_inodes_and_refuse_stale_ones() {
        for &params in FORMATS {
            let mut img = Image::new(params);
            let ino = img.add_file(b"a", b"x");
            let (_g, p, mp) = setup(img.finish());
            let vp = ffs_vget(mp, u64::from(ino)).unwrap();
            let mut fid = Fid::default();
            ffs_vptofh(vp, &mut fid).unwrap();
            vput(vp);
            let ufid = Ufid::from_fid(&fid);
            assert_eq!(
                (ufid.ufid_len as usize, ufid.ufid_ino),
                (size_of::<Ufid>(), ino)
            );
            let back = ffs_fhtovp(mp, &fid).unwrap();
            assert!(ptr::eq(back, vp), "the hashed vnode");
            vput(back);

            let with = |f: &dyn Fn(&mut Ufid)| {
                let mut u = ufid;
                f(&mut u);
                let mut h = Fid::default();
                u.to_fid(&mut h);
                ffs_fhtovp(mp, &h).map(vput)
            };
            assert_eq!(with(&|u| u.ufid_len -= 1), Err(Errno::EINVAL));
            assert_eq!(
                with(&|u| u.ufid_ino = 1),
                Err(Errno::ESTALE),
                "below ROOTINO"
            );
            let fs = vfstoufs(mp);
            let n = fs.um_fs().fs_ncg() * fs.um_fs().fs_ipg();
            assert_eq!(
                with(&|u| u.ufid_ino = n),
                Err(Errno::ESTALE),
                "past the inodes"
            );
            assert_eq!(
                with(&|u| u.ufid_gen = u.ufid_gen.wrapping_add(1)),
                Err(Errno::ESTALE),
                "another generation"
            );
            if params.ufs2 {
                // FFS2's lazily initialised inode blocks: past cg 0's cg_initediblk.
                let fsb = fs.um_fs();
                let at = (fsbtodb(fsb, cgtod(fsb, 0)) * DEV_BSIZE) as usize;
                let mut d = disk();
                let initediblk =
                    CgView::new(&mut d[at..at + fsb.fs_cgsize() as usize]).cg_initediblk();
                if initediblk + 1 < fsb.fs_ipg() {
                    assert_eq!(with(&|u| u.ufid_ino = initediblk + 1), Err(Errno::ESTALE));
                }
            }
            drop(fs);
            ffs_unmount(mp, 0, p).unwrap();
        }
    }

    #[test]
    fn vget_finds_the_hashed_vnode_and_gives_an_old_inode_a_generation() {
        for &params in FORMATS {
            let mut img = Image::new(params);
            let ino = img.add_file(b"old", b"x");
            img.with_dinode(ino, |d| match d {
                Din::Ufs1(d) => d.di_gen = 0,
                Din::Ufs2(d) => d.di_gen = 0,
            });
            let (_g, p, mp) = setup(img.finish());
            let vp = ffs_vget(mp, u64::from(ino)).unwrap();
            let ip = vtoi(vp);
            assert_ne!(ip.dip_gen(), 0, "a generation for an old inode");
            assert_ne!(
                ip.i_flag.get() & IN_MODIFIED,
                0,
                "to be written on a R/W mount"
            );
            assert!(ptr::eq(ip.itov(), vp));
            assert_eq!(ip.i_effnlink.get(), ip.dip_nlink());
            drop(ip);
            vput(vp);
            let again = ffs_vget(mp, u64::from(ino)).unwrap();
            assert!(ptr::eq(again, vp), "from the inode hash");
            vput(again);
            let r = root(mp);
            assert_ne!(r.flag() & crate::fs::vnode::VROOT, 0);
            vput(r);
            ffs_unmount(mp, 0, p).unwrap();
        }
    }

    /// The two tables `ffs_vinit` chooses from have one registry id each: a device node's
    /// vnode gets `ffs_specvops`, a file's `ffs_vops` (the regression of promoted constants,
    /// whose identical bytes were merged into one address and so one id).
    #[test]
    fn a_device_node_gets_the_special_files_table() {
        use crate::fs::ufs::ufs::dinode::{IFCHR, IFREG};
        use crate::fs::vnode::{VCHR, VREG};
        for &params in FORMATS {
            let mut img = Image::new(params);
            let dev = img.add_file(b"console", b"");
            img.with_dinode(dev, |d| match d {
                Din::Ufs1(d) => {
                    d.di_mode = (IFCHR | 0o600) as u16;
                    d.di_db[0] = TEST_DEV_OF_FIXTURE;
                }
                Din::Ufs2(d) => {
                    d.di_mode = (IFCHR | 0o600) as u16;
                    d.di_db[0] = i64::from(TEST_DEV_OF_FIXTURE);
                }
            });
            let file = img.add_file(b"f", b"x");
            let (_g, p, mp) = setup(img.finish());
            let t = &vfs::<TestGlue>().ufs;

            let vp = ffs_vget(mp, u64::from(dev)).unwrap();
            assert_eq!(vp.vtype(), VCHR);
            assert_eq!(vp.rdev(), TEST_DEV_OF_FIXTURE);
            assert!(ptr::addr_eq(vp.ops(), &t.ffs_specvops), "ffs_specvops");
            assert!(!ptr::addr_eq(vp.ops(), &t.ffs_vops));
            assert_eq!(vtoi(vp).dip_mode() & IFCHR, IFCHR);
            vput(vp);

            let vp = ffs_vget(mp, u64::from(file)).unwrap();
            assert_eq!(vp.vtype(), VREG);
            assert_eq!(vtoi(vp).dip_mode() & IFREG, IFREG);
            assert!(ptr::addr_eq(vp.ops(), &t.ffs_vops), "ffs_vops");
            vput(vp);
            ffs_unmount(mp, 0, p).unwrap();
        }
    }

    #[cfg(feature = "ufs_dirhash")]
    #[test]
    fn sysctl_answers_the_dirhash_names_and_eopnotsupp_for_the_rest() {
        use super::super::ffs_extern::{FFS_CLUSTERREAD, FFS_DIRHASH_DIRSIZE, FFS_DIRHASH_MEM};
        use crate::fs::ufs::ufs::ufs_dirhash::UFS_MINDIRHASHSIZE;
        let (_g, p) = attach(Image::new(FORMATS[0]).finish());
        let read = |mib: i32| {
            let mut len = 4;
            ffs_sysctl(&[mib], UBUF, &mut len, 0, 0, p).map(|()| {
                let mut b = [0u8; 4];
                UserPtr::<u8>::new(UBUF).copyin_slice(&mut b).unwrap();
                (i32::from_ne_bytes(b), len)
            })
        };
        let v = UFS_MINDIRHASHSIZE.load(Ordering::Relaxed);
        assert_eq!(read(FFS_DIRHASH_DIRSIZE), Ok((v, 4)));
        // Writing the value it has: the old one back, the new one in.
        UserPtr::<u8>::new(UBUF + 64)
            .copyout_slice(&v.to_ne_bytes())
            .unwrap();
        let mut len = 4;
        assert_eq!(
            ffs_sysctl(&[FFS_DIRHASH_DIRSIZE], UBUF, &mut len, UBUF + 64, 4, p),
            Ok(())
        );
        let mut len = 4;
        assert_eq!(
            ffs_sysctl(&[FFS_DIRHASH_MEM], 0, &mut len, UBUF + 64, 4, p),
            Err(Errno::EPERM),
            "read-only"
        );
        assert_eq!(
            ffs_sysctl(&[FFS_DIRHASH_DIRSIZE], 0, &mut len, UBUF + 64, 2, p),
            Err(Errno::EINVAL)
        );
        assert_eq!(read(FFS_CLUSTERREAD), Err(Errno::EOPNOTSUPP));
        assert_eq!(
            ffs_sysctl(&[FFS_DIRHASH_DIRSIZE, 0], 0, &mut len, 0, 0, p),
            Err(Errno::ENOTDIR)
        );
        assert_eq!(
            Ffs::<TestGlue>::VFS.sysctl(&[FFS_DIRHASH_MEM], UBUF, &mut len, 0, 0, p),
            Ok(())
        );
    }

    #[test]
    fn an_ffs1_super_block_takes_its_old_flags_sign_extended() {
        let mut img = Image::new(FFS1_4M).finish();
        let flags = (FS_FLAGS_UPDATED as u8) as i8;
        poke(
            &mut img,
            false,
            offset_of!(Fs, fs_ffs1_flags),
            &flags.to_ne_bytes(),
        );
        let (_g, p, mp) = setup(img);
        let ump = vfstoufs(mp);
        assert_eq!(ump.um_fs().fs_flags(), 0xffff_ff80, "int8_t into u_int32_t");
        assert_eq!(
            super_block(&disk(), false).fs_flags,
            0xffff_ff80,
            "as written back"
        );
        let d = super_block(&disk(), false);
        assert_eq!(d.fs_ffs1_time, d.fs_time as i32, "ffs1_compat_write");
        assert_eq!(d.fs_sblockloc, i64::from(SBLOCK_UFS1));
        drop(ump);
        ffs_unmount(mp, 0, p).unwrap();
    }

    #[test]
    fn an_image_mounted_at_the_root_works_through_the_system_calls() {
        for &params in FORMATS {
            let mut img = Image::new(params);
            img.add_file(b"motd", b"hello, world\n");
            let big = pattern(30000, 1);
            img.add_file(b"big", &big);
            let (_g, p) = attach(img.finish());
            let mp = mount_root(p, 0);

            assert_eq!(read_file(p, b"/motd").unwrap(), b"hello, world\n");
            assert_eq!(read_file(p, b"/big").unwrap(), &big[..]);
            assert_eq!(read_file(p, b"/nothere"), Err(Errno::ENOENT));

            // A file past the direct blocks (an indirect block), a directory with a file in
            // it, renames, removals and a symbolic link.
            let large = pattern(NDADDR * params.bsize as usize + 20000, 7);
            write_file(p, b"/large", &large).unwrap();
            sys_mkdir(p, path(b"/dir"), 0o755).unwrap();
            write_file(p, b"/dir/f", b"in a directory\n").unwrap();
            sys_rename(p, path(b"/dir/f"), up(1, b"/dir/g")).unwrap();
            sys_mkdir(p, path(b"/dir/sub"), 0o755).unwrap();
            assert_eq!(sys_rmdir(p, path(b"/dir")), Err(Errno::ENOTEMPTY));
            sys_rmdir(p, path(b"/dir/sub")).unwrap();
            sys_unlink(p, path(b"/motd")).unwrap();
            sys_symlink(p, path(b"/dir/g"), up(1, b"/lnk")).unwrap();
            let n = sys_readlink(p, path(b"/lnk"), UBUF, 64).unwrap();
            let mut l = vec![0u8; n];
            UserPtr::<u8>::new(UBUF).copyin_slice(&mut l).unwrap();
            assert_eq!(l, b"/dir/g");
            assert_eq!(read_file(p, b"/lnk").unwrap(), b"in a directory\n");
            assert_eq!(read_file(p, b"/large").unwrap(), &large[..]);
            assert_eq!(
                list_dir(p, b"/"),
                vec![
                    b"big".to_vec(),
                    b"dir".to_vec(),
                    b"large".to_vec(),
                    b"lnk".to_vec()
                ]
            );
            assert_eq!(list_dir(p, b"/dir"), vec![b"g".to_vec()]);

            // sync(2), unmount: the image is clean and its counters agree with its maps.
            sys_sync(p).unwrap();
            unmount_root(p, mp);
            let d = disk();
            assert_eq!(super_block(&d, params.ufs2).fs_clean, 1);
            check(&d, params.ufs2);

            // Mounted again read-only: everything is where it was left.
            let mp = mount_root(p, MNT_RDONLY);
            assert_eq!(read_file(p, b"/large").unwrap(), &large[..]);
            assert_eq!(read_file(p, b"/big").unwrap(), &big[..]);
            assert_eq!(read_file(p, b"/dir/g").unwrap(), b"in a directory\n");
            assert_eq!(read_file(p, b"/motd"), Err(Errno::ENOENT));
            assert_eq!(write_file(p, b"/new", b"x"), Err(Errno::EROFS));
            unmount_root(p, mp);

            // Read-write again: removing everything gives the space back.
            let mp = mount_root(p, 0);
            let fs = vfstoufs(mp);
            for f in [&b"/large"[..], b"/lnk", b"/dir/g", b"/big"] {
                sys_unlink(p, path(f)).unwrap();
            }
            sys_rmdir(p, path(b"/dir")).unwrap();
            assert_eq!(list_dir(p, b"/"), Vec::<Vec<u8>>::new());
            sys_sync(p).unwrap();
            let ndir = fs.um_fs().fs_cstotal_ndir();
            drop(fs);
            unmount_root(p, mp);
            check(&disk(), params.ufs2);
            assert_eq!(ndir, 1, "only the root directory is left");
        }
    }

    #[test]
    fn ufs_getlbns_finds_the_indirect_path() {
        use crate::fs::ufs::ufs::inode::Indir;
        use crate::fs::ufs::ufs::ufs_bmap::ufs_getlbns;
        let (_g, p, mp) = setup(Image::new(FORMATS[0]).finish());
        let vp = root(mp);
        let nindir = vfstoufs(mp).um_nindir.get() as i64;
        let mut a = [Indir::default(); NIADDR + 2];
        let mut num = -1;
        ufs_getlbns(vp, 5, &mut a, Some(&mut num)).unwrap();
        assert_eq!(num, 0, "a direct block");
        ufs_getlbns(vp, NDADDR as i64, &mut a, Some(&mut num)).unwrap();
        assert_eq!(num, 2);
        assert_eq!((a[0].in_lbn, a[0].in_off), (-(NDADDR as i64), 0));
        assert_eq!((a[1].in_lbn, a[1].in_off), (-(NDADDR as i64), 0));
        let lbn = NDADDR as i64 + nindir + 3;
        ufs_getlbns(vp, lbn, &mut a, Some(&mut num)).unwrap();
        assert_eq!(num, 3, "a double indirect path");
        assert_eq!(a[0].in_off, 1);
        assert_eq!(a[2].in_off, 3);
        vput(vp);
        ffs_unmount(mp, 0, p).unwrap();
    }

    #[test]
    fn a_mount_name_is_copied_nul_padded_and_truncated() {
        // A name that was longer leaves no byte past the new one's NUL (statfs(2) copies
        // the whole buffer out).
        let mut dst = [0xffu8; MNAMELEN];
        mname_copy(&mut dst, b"/dev/sd0a\0junk");
        assert_eq!(&dst[..10], b"/dev/sd0a\0");
        assert!(dst[10..].iter().all(|&c| c == 0));
        // Without a NUL the source ends at its end; past MNAMELEN - 1 bytes it is cut.
        mname_copy(&mut dst, &[b'x'; MNAMELEN + 5]);
        assert!(dst[..MNAMELEN - 1].iter().all(|&c| c == b'x'));
        assert_eq!(dst[MNAMELEN - 1], 0);
        mname_copy(&mut dst, b"");
        assert!(dst.iter().all(|&c| c == 0));
    }
}
/* </TESTS> */
