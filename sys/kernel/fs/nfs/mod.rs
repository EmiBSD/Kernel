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
/* </LICENSES> */

/* <CODE> */
//! The Network File System's kernel half (`docs/ROADMAP.md`'s N6.7, `sys/nfs/` in C): the
//! wire format and the ABI (N6.7a0); the client follows sub-slice by sub-slice, built beside
//! `bsd`'s and switched on by its last (D-1).
//!
//! Headers: `nfs` (`nfs.h`), `nfsproto` (`nfsproto.h`), `xdr_subs` (`xdr_subs.h`),
//! `nfsm_subs` (`nfsm_subs.h`), `nfsmount` (`nfsmount.h`), `nfsnode` (`nfsnode.h`),
//! `nfsdiskless` (`nfsdiskless.h`); files: `nfs_socket` (the client's half), `nfs_subs`, the
//! diskless boot's `krpc_subr` and `nfs_boot` (N6.7a4, `option NFSCLIENT` only), and the
//! call-closed part of the file system's half (N6.7a2): `nfs_node`, `nfs_bio`, `nfs_kq`,
//! `nfs_syscalls` (the nfsiods). Userland's headers are `hal::sys`'s images (`nfs_nfs`,
//! `nfs_nfsproto`, `nfs_rpcv2`), re-exported by the modules of the same name.

#[cfg(feature = "nfsclient")]
pub mod krpc_subr;
#[allow(clippy::module_inception)] // OpenBSD's layout: sys/nfs/nfs.h
pub mod nfs;
pub mod nfs_bio;
#[cfg(feature = "nfsclient")]
pub mod nfs_boot;
pub mod nfs_kq;
pub mod nfs_node;
pub mod nfs_socket;
pub mod nfs_subs;
pub mod nfs_syscalls;
pub mod nfsdiskless;
pub mod nfsm_subs;
pub mod nfsmount;
pub mod nfsnode;
pub mod nfsproto;
#[cfg(test)]
pub(crate) mod testnfs;
pub mod xdr_subs;

pub use crate::sys::nfs_rpcv2 as rpcv2;
/* </CODE> */
