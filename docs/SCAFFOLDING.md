# Scaffolding: what exists only while `bsd` does

The register of the temporary structure (the user's rule of 2026-10-30): every generic, glue
associated type, adapter or hop that exists only because the `kernel` crate lives beside
`bsd` (the LZ tree, deleted at N9). One row each: what it is, where it lives, why it exists,
and the milestone that removes it. The goal is to decouple and delete `bsd`; a row that
outlives its milestone is a defect.

The bridges (`kernel` or `hal` traits that `bsd` implements) are not repeated here: they are
the rows of `bridges.toml`, each with its `paid_by`. This file lists what the bridges drag
along: the associated types inside `ProcGlue` and `FileGlue`, the generic parameter that
carries them, the `bsd` adapters and the system-call hop.

Every slice that adds or removes scaffolding updates this table in the same commit (the
planner names it in the brief, the redesigner writes the row, the reviewer checks it). A
reviewer who finds an abstraction with no second real user and no equivalent in the C says
whether it is scaffolding (a row here) or over-architecture (fixed in the slice).

Locations are on `main` at the commit that last edited this file.

## The glue generic

| What | Where | Why | Removed by |
|---|---|---|---|
| The `G: ProcGlue` parameter of the kernel's process, file, VFS and tty types (`Process<G>`, `Thread<G>`, `Vnode<G>`, `Mount<G>`, `Componentname<'a, G>`, ...) | `sys/kernel/proc.rs:762` (the trait) | The kernel's types carry members and list links that belong to `bsd`'s subsystems; the parameter lets `bsd` supply them as associated types without the kernel depending on `bsd` | N9, with `bsd`. Whether the host tests' glue then keeps a parameter is N9's decision |

## `ProcGlue`'s associated types (the trait is the `ProcGlue` row of `bridges.toml`)

| What | Where | Why | Removed by |
|---|---|---|---|
| `ProcessG` | `sys/kernel/proc.rs:765` | The members of `struct process` still owned by `bsd`'s subsystems (the unveil table, the timeouts, the interval timers) | Each member with its subsystem: the unveil table at N8; the type at N9 at the latest |
| `ThreadG` | `sys/kernel/proc.rs:767` | The members of `struct proc` of `bsd`'s subsystems | N9 at the latest, member by member |
| `PgrpG` | `sys/kernel/proc.rs:770` | `struct pgrp`'s members of `bsd`'s: none since N4.5b | Empty now: removed by the next slice that touches `proc.rs`'s trait |
| `SessionG` | `sys/kernel/proc.rs:773` | `struct session`'s members of `bsd`'s: only `s_verauthto` today (`BsdSessionG`, `sys/kern/proc_glue.rs:332`); the trait's doc at `:771` still names `s_ttyvp`, the kernel's since N5.3, and `s_ttyp`: stale | N9 at the latest |
| The list adapters: `ThreadRunq`, `ProcessList`, `ProcessHash`, `ProcessPglist`, `ProcessSibling`, `ProcessOrphan`, `ThreadList`, `ThreadHash`, `ThreadLink`, `ThreadOwned`, `PgrpHash`, `KnLink`, `KnTqe`, `KnSelnext`, `KqNext`, `TtyLink` | `sys/kernel/proc.rs:775-807` | The elements are generic over `G`, so each adapter type is instantiated by `kernel::proc_glue_links!` in the crate that names the glue (`sys/kern/proc_glue.rs:355`) | With the glue generic, N9 |
| `FileGlue` (the pairing of the two glues) | `sys/kernel/proc.rs:797` | Ties a process glue to its file glue | With the glue generic, N9 |

## `FileGlue`'s associated types (the trait is the `FileGlue` row of `bridges.toml`)

| What | Where | Why | Removed by |
|---|---|---|---|
| `Proc` | `sys/kernel/file.rs:594` | The other half of the pairing above | With the glue generic, N9 |
| `Ops` | `sys/kernel/file.rs:596` | `struct fileops` of the objects still in `bsd` (sockets, and the kinds whose fileops have not moved) | When the last file kind's ops are the kernel's: sockets at N6, the rest by N8 |
| `Data` | `sys/kernel/file.rs:598` | `f_data` is `bsd`'s enum over the object kinds while sockets and pipes' wrappers are `bsd`'s (D-N5.3-4, D-N5.4-11) | N8 |
| `FileList` | `sys/kernel/file.rs:600` | `f_list`'s adapter, instantiated by `file_glue_links!` | With the glue generic, N9 |
| `VnodeG` | `sys/kernel/file.rs:603` | The members of `struct vnode` of `bsd`'s types: `v_socket` | N6 (the sockets) |
| `UnveilMatch` | `sys/kernel/file.rs:606` | `ni_unveil_match`, a pointer into `bsd`'s unveil table | N8 (unveil) |
| `vnode_ops`'s default method | `sys/kernel/file.rs:628` | The production path is the default (the kernel's vnode); only the kernel's test glue overrides it, for the vm tests' `MemVnode` pager double (N5.3) | When the vm tests use a VFS vnode, N9 at the latest |
| `InodeG` | `sys/kernel/file.rs` (`FileGlue`) | The members of `bsd`'s types a UFS inode carries (`inode_u`: ffs's `i_fs`, ext2fs's `i_e2fs`; ext2fs's `inode_ext` members; ext2fs's dinode pointer), `bsd`'s `BsdInodeG` from N5.5a1's switch (`i_dquot` and the dirhash left it at N5.5a2: the kernel inode's) | Member by member: `i_fs` at N5.5b, the ext2fs members at N5.6; the type with them |
| `UfsmountG` | `sys/kernel/file.rs` (`FileGlue`) | The members of `bsd`'s types a UFS mount carries (`ufsmount_u`: `um_fs`, `um_e2fs`; `um_export`, the NFS server's; the quota members left it at N5.5a2: the kernel structure's) | `um_fs` at N5.5b, `um_e2fs` at N5.6, `um_export` at N6 |
| `InodeHash`, `DqHash`, `DqFree` and the macro `ufs_glue_links!` | `sys/kernel/file.rs` | The inode hash's adapter (`i_hash`) for `Inode<G>` and the dquot hash's and free list's (`dq_hash`, `dq_freelist`) for `Dquot<G>` (N5.5a2), declared in the crate that names the glue (as `FileList`) | With the glue generic, N9 |
| `inode_e2fs_nlink`'s default method | `sys/kernel/file.rs` | `ufs_ihashget`'s `IS_EXT2_VNODE` reads an ext2fs inode's link count, which `bsd`'s ext2fs keeps in `InodeG` | N5.6 (ext2fs's dinode in the kernel) |
| `ext2fs_itimes` and `ext2fs_size` (default methods that panic) | `sys/kernel/file.rs` | `ufs_itimes`'s and `filt_ufsread`'s `IS_EXT2_VNODE` branches (`EXT2FS_ITIMES`, `ext2fs_size`) read and write ext2fs's dinode, which `bsd`'s ext2fs keeps in `InodeG` (N5.5a3) | N5.6 (ext2fs's dinode in the kernel) |
| The parameter `F: UfsFs<G>` of `UfsVnops` and `UfsSpecVnops` (the trait is `bridges.toml`'s `UfsFs` row) | `sys/kernel/fs/ufs/ufs/ufs_vnops.rs` | The three slots of `ffs_vops`/`ffs_specvops` that are ffs's own (`ffs_read`, `ffs_write`, `ffs_fsync`) are `bsd`'s; the parameter carries them into the kernel's composed tables (N5.5a3), and the kernel's test double fills it | N5.5b (the kernel's ffs: its tables name its operations; the parameter stays only if the double still needs it) |
| `VfsTables::ufs` (`UfsTables<G>`) | `sys/kernel/fs/subr.rs` | The UFS layer's globals (`ufs_init`'s `done`, the inode hash, and since N5.5a2 the dquot tables: `dqhashtbl`, `dqfreelist`, `numdquot`, `desireddquot`) per glue, since `Inode<G>` and `Dquot<G>` are generic | With the glue generic, N9 |

## The kernel's members kept for `bsd`'s users

| What | Where | Why | Removed by |
|---|---|---|---|
| `Vnode::data` (`v_data` as the C's raw word, `data()`/`set_data()`) beside `Vnode::node` (the kernel file systems' counted, typed handle), and the double tests over both: `getnewvnode`'s cleaned-vnode check and reset, `vprint` | `sys/kernel/fs/vnode.rs:680` (the member), `:684` (`node`); `sys/kernel/fs/subr.rs:587`, `:606-609`, `:1334` | The file systems still in `bsd` keep their node behind the C's `void *` (msdosfs, cd9660, udf, tmpfs, fuse, ntfs, nfs, and mfs's device node); the UFS inode of ffs, mfs and ext2fs is in the typed slot since N5.5a | The last `bsd` file system's move into the kernel (nfs at N6, ntfs at N10): `data` and the double tests go, `node` stays |

## `bsd`'s adapters over the kernel's types

| What | Where | Why | Removed by |
|---|---|---|---|
| Type aliases keeping LZ's names: `Vnode`, `Mount`, `Vfsconf`, `Specinfo`, `Componentname`, `Nameidata` | `sys/sys/vnode.rs:122`, `sys/sys/mount.rs:137`, `:140`, `sys/sys/specdev.rs:83`, `sys/sys/namei.rs:96`, `:99` | `bsd`'s file systems and drivers still call these types by LZ's names | Each alias with its last `bsd` user; all at N9 |
| Adapter modules of no logic: `sys/sys/namei.rs` (500 lines), `sys/sys/lockf.rs` (89), `sys/kern/vfs_syscalls.rs` (1,735) | those files | LZ's signatures for `bsd`'s importers and `init_sysent.rs`'s rows, over `kernel::fs` (N5.3, N5.4) | When their importers move (the file systems N5.5-N5.9, nfs N6) and the system-call table is the kernel's (N9) |
| The UFS layer's aliases and adapters: `Inode`, `InodeRef`, `InodeInit` (`sys/ufs/ufs/inode.rs`), `Ufsmount` (`sys/ufs/ufs/ufsmount.rs`), `Dquot` (`sys/ufs/ufs/ufs_quota.rs`); the modules `dinode.rs`, `dir.rs`, `ufs_ihash.rs`, `ufs_bmap.rs` (re-exports and LZ's `vop_bmap` slot), `ufs_vfsops.rs` (`ufs_init`'s `Vfsops` slot over the kernel's, since N5.5a2, and `ufs_check_export` beside the re-exports), and N5.5a2's no-logic `quota.rs` (LZ's signatures over `vfstoufs`), `ufs_quota.rs`, `dirhash.rs`, `ufs_dirhash.rs` (re-exports) and `ufs_inode.rs` (`ufs_inactive`'s `Vops` slot), and N5.5a3's `ufs_lookup.rs` (re-exports) | `sys/ufs/ufs/` | `bsd`'s ffs, ext2fs and mfs call LZ's names over the kernel's `fs::ufs::ufs` (N5.5a1 to N5.5a3) | With ffs (N5.5b) and ext2fs (N5.6) |
| The `IoBuf::Lent` arms of `strategy_done` and `strategy_clrbuf` (a lent buffer's parked token, or its foreign handle when none is parked) | `sys/kernel/fs/ufs/ufs/ufs_vnops.rs` | `ufs_strategy` receives `bsd`'s lent buffers from ext2fs (its `ufs_strategy` slot, and bsd's `VOP_STRATEGY` callers); ffs's buffers come as tokens since N5.5a3 | N5.6 (ext2fs's buffers are the kernel's tokens) |
| ext2fs's slots in `bsd`'s `Vops` shape: `ufs_close`, `ufs_ioctl`, `ufs_kqfilter`, `ufs_lock`, `ufs_unlock`, `ufs_strategy`, `ufs_print`, `ufs_islocked`, `ufsspec_close`, `ufsspec_read`, `ufsspec_write` and `ufs_pathconf` (each unpacks its `struct vop_*_args` and calls the kernel's) | `sys/ufs/ufs/ufs_vnops.rs` | ext2fs's tables (`EXT2FS_VOPS`, `EXT2FS_SPECVOPS`) are `bsd`'s `Vops` statics and name the UFS layer's operations (N5.5a3; ffs's tables are the kernel's `UfsVnops`) | N5.6 (ext2fs's tables in the kernel) |
| `BsdInodeG` and the `InodeExt` trait (`fs`, `e2fs`, `ext2fs_itimes`), with the two `unsafe` of `sys/ufs/ufs/` it keeps: `unsafe impl Sync for BsdInodeG` (`sys/ufs/ufs/inode.rs:175`, LZ's `struct inode` argument moved with its `Cell` members) and the deref of ext2fs's dinode pointer in `BsdInodeG::with_e2din` (`:206`) | `sys/ufs/ufs/inode.rs` | `bsd`'s `InodeG`: the members of ffs's and ext2fs's types an inode carries, and the readers of them (an inherent method cannot be added to the kernel's type in `bsd`; `ump` is the kernel's since N5.5a2) | `i_fs` at N5.5b, the ext2fs members at N5.6 |
| `BsdUfsmountG` and the `UfsmountExt` trait (`fs`, `e2fs`, `with_e2fsb`), with `unsafe impl Sync for BsdUfsmountG` (`sys/ufs/ufs/ufsmount.rs:116`, its `Cell` members); `vfstoufs` and its `unsafe` (`:170`) | `sys/ufs/ufs/ufsmount.rs` | `bsd`'s `UfsmountG`; the structure is still allocated into `mnt_data` by `bsd`'s mounts (its `um_export`, a `Netexport` of radix nodes, is not `Send`, which the mount's typed slot needs: N5.5a2 found no way into the slot without a new `unsafe impl Send` in `bsd`, the user's question) | `um_fs` at N5.5b, `um_e2fs` at N5.6, `um_export` at N6, which lets the mount's typed slot hold the structure (the user chose on 2026-10-31 to add no `unsafe impl Send`: the slot and `AnyBox` wait for N6, which decides; N5.5a2's reviewer's option (c), unverified: move `um_export` out of `UfsmountG` into `bsd`'s mount, so the member is `Send` with no `unsafe`) |
| The `Ufsmount` argument of the kernel's `qsync`, `quotaon`, `quotaoff` and `ufs_quotactl`, and `InodeInit::ump` filled from `vfstoufs` | `sys/kernel/fs/ufs/ufs/quota.rs`, `ufs_quota.rs`, `inode.rs`; `sys/ufs/ufs/quota.rs` (the adapters that pass `vfstoufs(mp)`) | `VFSTOUFS(mp)` is `bsd`'s `vfstoufs` until the mount's typed slot holds the structure (the row above). `i_ump` (the C's member, which stays) stores that `&'static` for the inode's life: sound because no inode, so no `InodeRef`, outlives the `vflush` of `ffs_unmount`/`ext2fs_unmount`, which frees the structure after it (vfstoufs's SAFETY) | With the mount's typed slot (N6) |

## The system-call hop

| What | Where | Why | Removed by |
|---|---|---|---|
| A `Native` entry's path: `abi` -> a `hal::api::KernelApi` group -> a `bsd` `native_*` wrapper -> `kern_descrip::native()` -> `mi_syscall` on `bsd`'s `SYSENT[nr]` row -> the row's `sys_*` adapter -> the kernel's function | `sys/kern/kern_descrip.rs:463` (`native()`); the wrappers in `sys/kern/vfs_syscalls.rs` (65), `proc_glue.rs` (46), `kern_descrip.rs` (20), `kern_time.rs` (10), `vm_bsd.rs` (10) and six more files | The prologue (pledge, the pinned system calls, the kernel lock) reads `curproc` and the row's flags (`SY_NOLOCK`, the argument count), which are `bsd`'s `init_sysent.rs` | N9: the kernel owns the table of flags and dispatches its entries itself (DECISIONS-N5.4 D-N5.4-1 calls it N9.1) |

## Arriving

Nothing announced.
