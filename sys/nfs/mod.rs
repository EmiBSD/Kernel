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
//! The Network File System's server half: OpenBSD `sys/nfs/` (`option NFSSERVER`, feature
//! `nfsserver`), until N6.7b moves it. The client (`option NFSCLIENT`) is the kernel's,
//! `kernel::fs::nfs`, since N6.7a5: the files left here are the server's four, `nfs_var`,
//! and the server half of the files the C shares between both (`nfs`, `nfs_socket`,
//! `nfs_subs`, `nfs_syscalls`, `nfsm_subs`), cut at the C's `#ifdef`s (D-5).
//!
//! Headers (types): `nfs` (`nfs.h`), `nfsproto` (`nfsproto.h`), `rpcv2` (`rpcv2.h`),
//! `xdr_subs` (`xdr_subs.h`), `nfsm_subs` (`nfsm_subs.h`), `nfsrvcache` (`nfsrvcache.h`),
//! `nfs_var` (`nfs_var.h`). Files (functions): `nfs_socket`, `nfs_subs`, `nfs_srvsubs`,
//! `nfs_srvcache`, `nfs_syscalls`, `nfs_serv`.

#[allow(clippy::module_inception)] // OpenBSD's layout: sys/nfs/nfs.h
pub mod nfs;
#[cfg(feature = "nfsserver")]
pub mod nfs_serv;
pub mod nfs_socket;
#[cfg(feature = "nfsserver")]
pub mod nfs_srvcache;
#[cfg(feature = "nfsserver")]
pub mod nfs_srvsubs;
pub mod nfs_subs;
pub mod nfs_syscalls;
#[forbid(unsafe_code)]
pub mod nfs_var;
pub mod nfsm_subs;
pub use kernel::fs::nfs::nfsproto;
pub mod nfsrvcache;
pub use hal::sys::nfs_rpcv2 as rpcv2;
pub use kernel::fs::nfs::xdr_subs;
/* </CODE> */
