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
 *	@(#)nfs_subs.c	8.8 (Berkeley) 5/22/95
 */
/* </LICENSES> */

/* <CODE> */
//! `nfs/nfs_subs.c` in the kernel: the XDR words `nfs_init` computes, the type tables of the
//! conversion macros, the statistics, `txdr_nfsv2time`, and the functions that support the
//! `nfsm_subs.h` cursors: building a request (`nfsm_reqhead`, `nfsm_build`,
//! `nfsm_uiotombuf`, `nfsm_buftombuf`, `nfsm_strtombuf`, `nfsm_fhtom`, `nfsm_v3attrbuild`)
//! and taking a reply apart (`nfsm_disct`, `nfs_adv`, `nfsm_mbuftouio`), as methods of the
//! cursors of `nfsm_subs` (`XdrWriter`, `XdrReader`).
//!
//! Upstream: sys/nfs/nfs_subs.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_subs.rs@3c62ede68802
//!
//! ## Deviations
//! - The XDR words `nfs_init` computes at startup (`rpc_call`, `nfs_true`, `nfs_xdrneg1`,
//!   ...) are `const`s computed at compile time, with their C names (lowercase:
//!   `RPC_CALL` is already the `rpcv2.h` constant).
//! - `nfsstats` is [`NFSSTATS`], counters of atomics (`nfs`).
//!
//! ## Redesign
//! - LZ's `bsd` module is the starting point of the client's half (D-5, N6.7a0): the words,
//!   the tables, the statistics and `txdr_nfsv2time` are the kernel's, `bsd`'s `nfs_subs`
//!   re-exports them for the server; the RPC header, the xids, the attribute cache and the
//!   commit ranges follow with N6.7a1 and N6.7a2.

#![allow(non_upper_case_globals)] // the C's names of the XDR words

use super::nfs::NfsCounters;
use super::nfsproto::{
    NFBLK, NFCHR, NFDIR, NFFIFO, NFLNK, NFNON, NFREG, NFS_PROG, NFSOCK, Nfstype, Nfsv2Time,
};
use super::xdr_subs::txdr_unsigned;
use crate::clock::Timespec;
use crate::fs::vnode::VNOVAL;
use crate::fs::vnode::Vtype::{self, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREG, VSOCK};
use crate::sys::nfs_rpcv2::{
    RPC_AUTHERR, RPC_CALL, RPC_MISMATCH, RPC_MSGACCEPTED, RPC_MSGDENIED, RPC_REPLY, RPC_VER2,
    RPCAUTH_UNIX,
};

/// `nfs_xdrneg1`: the XDR word of -1. Data items converted to xdr at startup, since they are
/// constant. This is kinda hokey, but may save a little time doing byte swaps.
pub const nfs_xdrneg1: u32 = txdr_unsigned(-1i32 as u32);
/// `rpc_call`.
pub const rpc_call: u32 = txdr_unsigned(RPC_CALL);
/// `rpc_vers`.
pub const rpc_vers: u32 = txdr_unsigned(RPC_VER2);
/// `rpc_reply`.
pub const rpc_reply: u32 = txdr_unsigned(RPC_REPLY);
/// `rpc_msgdenied`.
pub const rpc_msgdenied: u32 = txdr_unsigned(RPC_MSGDENIED);
/// `rpc_autherr`.
pub const rpc_autherr: u32 = txdr_unsigned(RPC_AUTHERR);
/// `rpc_mismatch`.
pub const rpc_mismatch: u32 = txdr_unsigned(RPC_MISMATCH);
/// `rpc_auth_unix`.
pub const rpc_auth_unix: u32 = txdr_unsigned(RPCAUTH_UNIX);
/// `rpc_msgaccepted`.
pub const rpc_msgaccepted: u32 = txdr_unsigned(RPC_MSGACCEPTED);
/// `nfs_prog`.
pub const nfs_prog: u32 = txdr_unsigned(NFS_PROG);
/// `nfs_true`.
pub const nfs_true: u32 = txdr_unsigned(1);
/// `nfs_false`.
pub const nfs_false: u32 = txdr_unsigned(0);

/// `nfsv2_type`: the version 2 file type of each vnode type.
pub const NFSV2_TYPE: [Nfstype; 9] = [
    NFNON, NFREG, NFDIR, NFBLK, NFCHR, NFLNK, NFNON, NFCHR, NFNON,
];
/// `nfsv3_type`: the version 3 file type of each vnode type.
pub const NFSV3_TYPE: [Nfstype; 9] = [
    NFNON, NFREG, NFDIR, NFBLK, NFCHR, NFLNK, NFSOCK, NFFIFO, NFNON,
];
/// `nv2tov_type`: the vnode type of each version 2 file type.
pub const NV2TOV_TYPE: [Vtype; 8] = [VNON, VREG, VDIR, VBLK, VCHR, VLNK, VNON, VNON];
/// `nv3tov_type`: the vnode type of each version 3 file type.
pub const NV3TOV_TYPE: [Vtype; 8] = [VNON, VREG, VDIR, VBLK, VCHR, VLNK, VSOCK, VFIFO];

/// `nfsstats`: the client's and the server's counters (`vfs.nfs.nfsstats`).
pub static NFSSTATS: NfsCounters = NfsCounters::new();

/// `txdr_nfsv2time(from, to)`: the version 2 time of `from`; `VNOVAL` nanoseconds are sent
/// as -1, a time of -1 seconds as -1.000001.
pub fn txdr_nfsv2time(from: &Timespec) -> Nfsv2Time {
    if from.tv_nsec == i64::from(VNOVAL) {
        Nfsv2Time {
            nfsv2_sec: nfs_xdrneg1,
            nfsv2_usec: nfs_xdrneg1,
        }
    } else if from.tv_sec == -1 {
        // can't request a time of -1; send -1.000001 == {-2,999999} instead
        Nfsv2Time {
            nfsv2_sec: txdr_unsigned(-2i32 as u32),
            nfsv2_usec: txdr_unsigned(999_999),
        }
    } else {
        Nfsv2Time {
            nfsv2_sec: txdr_unsigned(from.tv_sec as u32),
            nfsv2_usec: txdr_unsigned((from.tv_nsec / 1000) as u32),
        }
    }
}

const _: () = {
    assert!(super::xdr_subs::fxdr_unsigned(nfs_xdrneg1) == u32::MAX);
    assert!(NFSV3_TYPE.len() == 9 && NV3TOV_TYPE.len() == 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of nfs_subs.c's words, tables and times.

    use super::*;
    use crate::fs::nfs::xdr_subs::fxdr_unsigned;

    #[test]
    fn version2_times() {
        let t = txdr_nfsv2time(&Timespec::new(5, 7_000));
        assert_eq!(
            (fxdr_unsigned(t.nfsv2_sec), fxdr_unsigned(t.nfsv2_usec)),
            (5, 7)
        );
        let none = txdr_nfsv2time(&Timespec::new(5, i64::from(VNOVAL)));
        assert_eq!(
            (none.nfsv2_sec, none.nfsv2_usec),
            (nfs_xdrneg1, nfs_xdrneg1)
        );
        let m1 = txdr_nfsv2time(&Timespec::new(-1, 0));
        assert_eq!(fxdr_unsigned(m1.nfsv2_sec) as i32, -2);
        assert_eq!(fxdr_unsigned(m1.nfsv2_usec), 999_999);
    }

    #[test]
    fn tables() {
        assert_eq!(NFSV2_TYPE[VFIFO as usize], NFCHR);
        assert_eq!(NFSV3_TYPE[VSOCK as usize], NFSOCK);
        assert_eq!(NFSV3_TYPE[Vtype::VBAD as usize], NFNON);
        assert_eq!(NV3TOV_TYPE[NFFIFO as usize], VFIFO);
        assert_eq!(NV2TOV_TYPE[NFSOCK as usize], VNON);
        assert_eq!(fxdr_unsigned(rpc_call), 0);
        assert_eq!(fxdr_unsigned(nfs_prog), 100_003);
        assert_eq!(fxdr_unsigned(nfs_true), 1);
    }
}
/* </TESTS> */
