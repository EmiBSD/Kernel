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
 * Copyright (c) 1991, 1993
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
 *	@(#)nfsdiskless.h	8.2 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfsdiskless.h>` in the kernel: the structure that must be initialized for a diskless
//! NFS client. It is used by `nfs_mountroot()` to set up the root and swap vnodes plus do a
//! partial `ifconfig(8)` and `route(8)` so that the critical net interface can communicate
//! with the server. Whether or not the swap area is NFS mounted is determined by the value in
//! `swdevt[0]` (equal to `NODEV` means swap over NFS). Currently only works for `AF_INET`
//! protocols.
//!
//! Upstream: sys/nfs/nfsdiskless.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfsdiskless.rs@e8249449d905
//!
//! NB: all fields are stored in net byte order to avoid hassles with client/server byte
//! ordering differences.
//!
//! The prototypes of `nfs_boot_init` and `nfs_boot_getfh` are the functions of `nfs_boot`.
//!
//! ## Deviations
//! - `ndm_args` is `hal::sys::mount::NfsArgs`, whose pointer members (`addr`, `fh`,
//!   `hostname`) are user addresses; the kernel's `nfs_boot_getfh` leaves them `NULL` and the
//!   members of the structure are passed themselves (`nfs_boot`'s Redesign).
//!
//! ## Redesign
//! - LZ's `NfsDlmount::new` spelled every member of `NfsArgs` (holes included) and
//!   `NfsDiskless` held a `&'static Vnode` of `bsd`'s. The arguments are the image's zeroed
//!   value (`hal::user::zeroed`), and the structure is generic over the glue whose vnode
//!   `sw_vp` is (`Option<&'static Vnode<G>>`); nothing in it points into itself, so it may
//!   move (LZ's had to stay put once `nfs_boot_getfh` had pointed the arguments at its
//!   members).

use hal::sys::mount::{MNAMELEN, NfsArgs};
use hal::sys::netinet_in::SockaddrIn;
use hal::user::zeroed;

use super::nfsproto::NFSX_V3FHMAX;
use crate::fs::vnode::Vnode;
use crate::proc::ProcGlue;

/// `struct nfs_dlmount`: the mount information of one diskless file system (root or swap).
#[derive(Clone, Copy)]
pub struct NfsDlmount {
    /// `ndm_args`: the mount arguments.
    pub ndm_args: NfsArgs,
    /// `ndm_saddr`: address of file server.
    pub ndm_saddr: SockaddrIn,
    /// `ndm_host`: host name for mount point (NUL-terminated).
    pub ndm_host: [u8; MNAMELEN],
    /// `ndm_fh`: the file's file handle.
    pub ndm_fh: [u8; NFSX_V3FHMAX],
}

impl NfsDlmount {
    /// An all-zero structure (the C's `static` or `memset`).
    pub fn new() -> Self {
        Self {
            ndm_args: zeroed::<NfsArgs>(),
            ndm_saddr: SockaddrIn::default(),
            ndm_host: [0; MNAMELEN],
            ndm_fh: [0; NFSX_V3FHMAX],
        }
    }
}

impl Default for NfsDlmount {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct nfs_diskless`: what a diskless client needs to find its root and swap.
pub struct NfsDiskless<G: ProcGlue> {
    /// `nd_boot`: address of boot server.
    pub nd_boot: SockaddrIn,
    /// `nd_root`: mount info for root.
    pub nd_root: NfsDlmount,
    /// `nd_swap`: mount info for swap.
    pub nd_swap: NfsDlmount,
    /// `sw_vp`: the swap vnode.
    pub sw_vp: Option<&'static Vnode<G>>,
}

impl<G: ProcGlue> NfsDiskless<G> {
    /// An empty structure to be filled in (`nfs_boot_init`).
    pub fn new() -> Self {
        Self {
            nd_boot: SockaddrIn::default(),
            nd_root: NfsDlmount::new(),
            nd_swap: NfsDlmount::new(),
            sw_vp: None,
        }
    }
}

impl<G: ProcGlue> Default for NfsDiskless<G> {
    fn default() -> Self {
        Self::new()
    }
}
/* </CODE> */
