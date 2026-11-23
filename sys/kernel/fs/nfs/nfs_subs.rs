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
//! cursors of `nfsm_subs` ([`XdrWriter`], [`XdrReader`]).
//!
//! Upstream: sys/nfs/nfs_subs.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_subs.rs@3c62ede68802
//!
//! ## Deviations
//! - The XDR words `nfs_init` computes at startup (`rpc_call`, `nfs_true`, `nfs_xdrneg1`,
//!   ...) are `const`s computed at compile time, with their C names (lowercase:
//!   `RPC_CALL` is already the `rpcv2.h` constant).
//! - `nfsstats` is [`NFSSTATS`], counters of atomics (`nfs`).
//! - `nfsm_uiotombuf` ignores `uiomove`'s error as the C does, but the bytes a failed move
//!   did not fill are zeroed: the C sends whatever the fresh mbuf held there.
//! - `nfsm_mbuftouio` checks the uio's residual count where the C checks for iovecs left
//!   (`EFBIG`): the same thing for a uio whose `uio_resid` is the sum of its iovecs, which every
//!   uio the client builds is; its ignored `copyout` error is kept (the uio moves on).
//!
//! ## Redesign
//! - LZ's `bsd` module is the starting point of the client's half (D-5, N6.7a0): the words,
//!   the tables, the statistics and `txdr_nfsv2time` are the kernel's, `bsd`'s `nfs_subs`
//!   re-exports them for the server; the RPC header, the xids, the attribute cache and the
//!   commit ranges follow with N6.7a1 and N6.7a2.
//! - LZ's build functions over a `&mut &'static Mbuf` cursor (`nfsm_build`, `nfsm_uiotombuf`
//!   with its two raw writes into the trailing space, `nfsm_buftombuf` and `nfsm_strtombuf`
//!   over `UIO_SYSSPACE` uios of raw iovecs) are methods of the writer that owns the chain:
//!   one body, `append`, takes the bytes from the source in pieces, so a buffer or a string is
//!   copied straight from its slice (no uio built over it) with the C's mbuf layout. LZ's
//!   `nfsm_mbuftouio` (a raw copy into a `UIO_SYSSPACE` iovec, `copyout` otherwise) is the
//!   reader's, each piece moved by the glue's `uiomove_out`; `nfsm_disct`, `nfs_adv` and
//!   `nfsm_nextbytes` are the reader's copies and walks (`nfsm_subs`): no mbuf is inserted
//!   into a reply, no raw position kept.

#![allow(non_upper_case_globals)] // the C's names of the XDR words

use super::nfs::NfsCounters;
use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_padlen};
use super::nfsproto::{
    NFBLK, NFCHR, NFDIR, NFFIFO, NFLNK, NFNON, NFREG, NFS_MAXFHSIZE, NFS_PROG, NFSOCK,
    NFSV3SATTRTIME_DONTCHANGE, NFSV3SATTRTIME_TOCLIENT, NFSV3SATTRTIME_TOSERVER, NFSX_V2FH, Nfsfh,
    Nfstype, Nfsv2Time,
};
use super::xdr_subs::{txdr_nfsv3time, txdr_unsigned};
use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::file::FileGlue;
use crate::fs::vnode::Vtype::{self, VBLK, VCHR, VDIR, VFIFO, VLNK, VNON, VREG, VSOCK};
use crate::fs::vnode::{VNOVAL, Vattr};
use crate::net::mbuf::{M_WAIT, MLEN, MT_DATA, m_get, mclget};
use crate::proc::ProcGlue;
use crate::sys::errno::Errno;
use crate::sys::nfs_rpcv2::{
    RPC_AUTHERR, RPC_CALL, RPC_MISMATCH, RPC_MSGACCEPTED, RPC_MSGDENIED, RPC_REPLY, RPC_VER2,
    RPCAUTH_UNIX,
};
use crate::sys::types::{Gid, Mode, Off, Uid};
use crate::tc::gettime;
use crate::uio::{Uio, UioRw};

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

impl XdrWriter {
    /// `nfsm_reqhead(hsiz)`: a request's first mbuf, empty, with a cluster when the rest of
    /// the header (`hsiz` bytes) would not fit a plain one.
    pub fn reqhead(hsiz: usize) -> Self {
        let Some(mut mb) = m_get(M_WAIT, MT_DATA) else {
            kpanic(format_args!("nfs: out of mbufs"));
        };
        if hsiz > MLEN {
            mclget(&mut mb, M_WAIT);
        }
        mb.m_len().set(0);
        Self::with_first(mb)
    }

    /// The body `nfsm_uiotombuf` runs for every source: `len` bytes appended across mbufs
    /// (a fresh one, with a cluster while more than `MLEN` bytes are left, each time the tail
    /// is full), `fill` writing each piece in order, then the XDR padding, zeroed (in a plain
    /// fresh mbuf when the tail has no room for it).
    fn append(&mut self, mut len: usize, mut fill: impl FnMut(&mut [u8])) {
        let pad = nfsm_padlen(len);
        while len > 0 {
            let xfer = len.min(self.trailing());
            fill(self.take(xfer));
            len -= xfer;
            if len > 0 {
                self.grow(len > MLEN);
            }
        }
        if pad > 0 {
            if pad > self.trailing() {
                self.grow(false);
            }
            self.take(pad).fill(0);
        }
    }

    /// `nfsm_uiotombuf(mp, uiop, len)`: `len` bytes of `uio` appended, then their padding. As
    /// the C, `uio_resid` is set to `len` and the uio made a write first, and `uiomove`'s
    /// error is not returned; the bytes a failed move did not fill are zero.
    pub fn from_uio<G: ProcGlue>(&mut self, uio: &mut Uio<'_, G>, len: usize) {
        // XXX -- the following should be done by the caller
        uio.uio_resid = len;
        uio.uio_rw = UioRw::UIO_WRITE;
        self.append(len, |dst| {
            let before = uio.uio_resid;
            if <G::FileGlue as FileGlue>::uiomove(dst, uio).is_err() {
                let moved = before - uio.uio_resid;
                dst[moved..].fill(0);
            }
        });
    }

    /// `nfsm_buftombuf(mp, buf, len)`: the bytes of `buf` appended, then their padding.
    pub fn buftombuf(&mut self, buf: &[u8]) {
        let mut at = 0;
        self.append(buf.len(), |dst| {
            dst.copy_from_slice(&buf[at..at + dst.len()]);
            at += dst.len();
        });
    }

    /// `nfsm_strtombuf(mp, str, len)`: the XDR string `str`: its length word, its bytes, the
    /// padding, appended as one stream (the length word may straddle two mbufs, as the C's
    /// two-iovec uio lets it).
    pub fn strtombuf(&mut self, str: &[u8]) {
        let Ok(n) = u32::try_from(str.len()) else {
            kpanic(format_args!("nfsm_strtombuf: {} bytes", str.len()));
        };
        let head = n.to_be_bytes();
        let mut at = 0;
        self.append(head.len() + str.len(), |dst| {
            for b in dst.iter_mut() {
                *b = if at < head.len() {
                    head[at]
                } else {
                    str[at - head.len()]
                };
                at += 1;
            }
        });
    }

    /// `nfsm_fhtom(mb, v, v3)`: a node's file handle (`n_fhp`, `n_fhsize` bytes of it): a
    /// counted string in version 3, the fixed `NFSX_V2FH` bytes in version 2.
    pub fn fhtom(&mut self, fh: &Nfsfh, fhsize: usize, v3: bool) {
        if v3 {
            self.strtombuf(&fh.fh_bytes[..fhsize.min(NFS_MAXFHSIZE)]);
        } else {
            self.buftombuf(&fh.fh_bytes[..NFSX_V2FH]);
        }
    }

    /// `nfsm_v3attrbuild(mp, a, full)`: a version 3 `sattr3` of `a`: the mode and the times
    /// always, the owner, group and size only when `full`; a member left `VNOVAL` is "do not
    /// set", a time equal to the server's second is "set to the server's time".
    pub fn v3attrbuild(&mut self, a: &Vattr, full: bool) {
        let set = |w: &mut Self, on: bool, v: u32| {
            if on {
                w.word(nfs_true);
                w.u32(v);
            } else {
                w.word(nfs_false);
            }
        };
        set(self, a.va_mode != VNOVAL as Mode, a.va_mode);
        set(self, full && a.va_uid != VNOVAL as Uid, a.va_uid);
        set(self, full && a.va_gid != VNOVAL as Gid, a.va_gid);
        if full && a.va_size != VNOVAL as u64 {
            self.word(nfs_true);
            self.hyper(a.va_size);
        } else {
            self.word(nfs_false);
        }
        for t in [&a.va_atime, &a.va_mtime] {
            if t.tv_nsec != i64::from(VNOVAL) {
                if t.tv_sec != gettime() {
                    self.u32(NFSV3SATTRTIME_TOCLIENT);
                    self.pod(&txdr_nfsv3time(t));
                } else {
                    self.u32(NFSV3SATTRTIME_TOSERVER);
                }
            } else {
                self.u32(NFSV3SATTRTIME_DONTCHANGE);
            }
        }
    }
}

impl XdrReader<'_> {
    /// `nfsm_mbuftouio(mrep, uiop, siz, dpos)`: `siz` bytes of the reply into `uio`, mbuf by
    /// mbuf, then their XDR padding skipped. `EFBIG` when the uio is full first, `EBADRPC`
    /// when the chain ends first. A failed copy is the C's ignored `copyout` error: the uio
    /// moves past the bytes all the same.
    pub(super) fn mbuftouio<G: ProcGlue>(
        &mut self,
        uio: &mut Uio<'_, G>,
        siz: usize,
    ) -> Result<(), Errno> {
        let mut left = siz;
        while left > 0 {
            if uio.uio_resid == 0 || uio.uio_iov.is_empty() {
                return Err(Errno::EFBIG);
            }
            let piece = self.next_piece(left.min(uio.uio_resid))?;
            let before = uio.uio_resid;
            if <G::FileGlue as FileGlue>::uiomove_out(piece, uio).is_err() {
                uio_skip(uio, piece.len() - (before - uio.uio_resid));
            }
            left -= piece.len();
        }
        self.adv(nfsm_padlen(siz))
    }
}

/// The C's `uiop->uio_iov++`/`iov_base += n` past `n` bytes a copy did not move (its
/// ignored `copyout` error): the iovecs, `uio_resid` and `uio_offset` advanced, nothing copied.
fn uio_skip<G: ProcGlue>(uio: &mut Uio<'_, G>, mut n: usize) {
    while n > 0 {
        let Some(iov) = uio.uio_iov.first_mut() else {
            return;
        };
        if iov.iov_len == 0 {
            let iovs = core::mem::take(&mut uio.uio_iov);
            uio.uio_iov = &mut iovs[1..];
            continue;
        }
        let c = iov.iov_len.min(n);
        iov.iov_base = iov.iov_base.wrapping_byte_add(c);
        iov.iov_len -= c;
        uio.uio_resid -= c;
        uio.uio_offset = uio
            .uio_offset
            .saturating_add(Off::try_from(c).unwrap_or(Off::MAX));
        n -= c;
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
    // Host tests of nfs_subs.c's words, tables and times, and of the cursors' methods it
    // supplies: the uio copies both ways, the strings, the handles, the sattr3.

    use super::*;
    use crate::fs::nfs::xdr_subs::fxdr_unsigned;

    use std::vec;
    use std::vec::Vec;

    use crate::fs::nfs::nfsm_subs::{TestRng, nfsm_rndup, test_bytes, test_chain, test_lens};
    use crate::fs::vnode::Vattr;
    use crate::net::mbuf::tests::setup;
    use crate::uio::{Iovec, Uio};

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// `len` bytes of the reply at `r` read into a buffer of `room` bytes through a
    /// `UIO_SYSSPACE` uio (`nfsm_mtouio`): the result, the buffer and the residual count.
    fn read_into(
        r: &mut XdrReader<'_>,
        len: i32,
        room: usize,
    ) -> (Result<(), Errno>, Vec<u8>, usize) {
        let mut buf = vec![0xeeu8; room];
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<Procs>::kernel_read(&mut iov, &mut buf, None);
        let res = r.to_uio(&mut uio, len);
        let resid = uio.uio_resid;
        drop(uio);
        (res, buf, resid)
    }

    #[test]
    fn mtouio_copies_skips_the_padding_and_checks_both_ends() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        // A 5-byte opaque, its 3 padding bytes, then a word.
        let m = test_chain(&[&[1, 2], &[3, 4, 5, 0], &[0, 0, 0xaa, 0xbb, 0xcc, 0xdd]]);
        let mut r = XdrReader::new(&m);
        let (res, buf, resid) = read_into(&mut r, 5, 8);
        assert_eq!(res, Ok(()));
        assert_eq!(buf[..5], [1, 2, 3, 4, 5]);
        assert_eq!(resid, 3);
        assert_eq!(r.u32(), Ok(0xaabb_ccdd));
        // len <= 0 moves nothing.
        let mut r = XdrReader::new(&m);
        assert_eq!(read_into(&mut r, 0, 8).0, Ok(()));
        assert_eq!(read_into(&mut r, -7, 8).0, Ok(()));
        assert_eq!(r.offset(), 0);
        // A uio smaller than the data: EFBIG; a chain shorter than it: EBADRPC.
        let mut r = XdrReader::new(&m);
        assert_eq!(read_into(&mut r, 5, 4).0, Err(Errno::EFBIG));
        let mut r = XdrReader::new(&m);
        assert_eq!(read_into(&mut r, 13, 64).0, Err(Errno::EBADRPC));
        crate::testutil::set_copy_kernel(false);
    }

    #[test]
    fn property_mtouio_around_the_uio_and_the_chain() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        let mut rng = TestRng(0x6e66_735f_7569_6f31);
        for _ in 0..600 {
            let n = rng.below(120);
            let data = rng.bytes(n);
            let m = test_chain(&rng.split(&data));
            let len = rng.below(n + 8) as i32 - 4;
            let room = rng.below(n + 8);
            let mut r = XdrReader::new(&m);
            let (res, buf, resid) = read_into(&mut r, len, room);
            let siz = usize::try_from(len).unwrap_or(0);
            if siz == 0 {
                assert_eq!((res, resid, r.offset()), (Ok(()), room, 0));
            } else if siz > room {
                // The uio fills first unless the chain ends before it.
                let want = if room > n {
                    Errno::EBADRPC
                } else {
                    Errno::EFBIG
                };
                assert_eq!(res, Err(want), "len {len} room {room} n {n}");
            } else if siz > n {
                assert_eq!(res, Err(Errno::EBADRPC));
            } else {
                assert_eq!(buf[..siz], data[..siz]);
                assert_eq!(resid, room - siz);
                // The padding skipped, or EBADRPC when the chain has not all of it.
                let padded = nfsm_rndup(siz);
                if padded <= n {
                    assert_eq!((res, r.offset()), (Ok(()), padded));
                } else {
                    assert_eq!(res, Err(Errno::EBADRPC));
                }
            }
            assert!(
                buf[siz.min(room)..].iter().all(|&b| b == 0xee),
                "never past the data"
            );
        }
        crate::testutil::set_copy_kernel(false);
    }

    #[test]
    fn uiotombuf_appends_the_uio_and_its_padding_across_mbufs() {
        let _g = setup();
        crate::testutil::set_copy_kernel(true);
        let data: Vec<u8> = (0..(MLEN * 3 + 5)).map(|i| (i * 7 + 3) as u8).collect();
        let mut w = XdrWriter::reqhead(0);
        w.u32(7);
        let mut iov = [Iovec::new()];
        let mut uio = Uio::<Procs>::kernel_write(&mut iov, &data, None);
        uio.uio_resid = 0; // nfsm_uiotombuf sets it, as the C
        w.from_uio(&mut uio, data.len());
        assert_eq!(uio.uio_resid, 0);
        drop(uio);
        let m = w.into_chain();
        let mut want = 7u32.to_be_bytes().to_vec();
        want.extend_from_slice(&data);
        want.extend_from_slice(&[0, 0, 0]);
        assert_eq!(test_bytes(&m), want);
        assert!(test_lens(&m).len() >= 2);
        crate::testutil::set_copy_kernel(false);
    }

    #[test]
    fn strings_and_buffers_keep_the_c_layout() {
        let _g = setup();
        // The tail with two bytes of room: the length word straddles, as through the C's uio.
        let mut w = XdrWriter::with_first(test_chain(&[&[]]));
        let room = w.trailing();
        let pre = vec![0x55u8; room - 2];
        w.buftombuf(&pre);
        assert_eq!(w.trailing(), room - nfsm_rndup(room - 2));
        let name = b"hello";
        let mut w2 = XdrWriter::with_first(test_chain(&[&[]]));
        let fill = vec![1u8; w2.trailing() - 2];
        w2.build_slice(fill.len()).copy_from_slice(&fill);
        w2.strtombuf(name);
        let m = w2.into_chain();
        assert_eq!(
            test_lens(&m)[0],
            fill.len() + 2,
            "the word's first half in the first mbuf"
        );
        let mut want = fill.clone();
        want.extend_from_slice(&[0, 0, 0, 5]);
        want.extend_from_slice(name);
        want.extend_from_slice(&[0, 0, 0]);
        assert_eq!(test_bytes(&m), want);
        drop(w);
    }

    #[test]
    fn file_handles_by_version() {
        let _g = setup();
        let mut fh = Nfsfh::new();
        for (i, b) in fh.fh_bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        let mut w = XdrWriter::reqhead(0);
        w.fhtom(&fh, 28, true);
        w.fhtom(&fh, 28, false);
        let m = w.into_chain();
        let b = test_bytes(&m);
        assert_eq!(b[..4], 28u32.to_be_bytes());
        assert_eq!(b[4..32], fh.fh_bytes[..28]);
        assert_eq!(b[32..64], fh.fh_bytes[..NFSX_V2FH]);
        assert_eq!(b.len(), 64);
    }

    #[test]
    fn v3_sattr() {
        let _g = setup();
        let mut va = Vattr::new();
        crate::fs::subr::vattr_null(&mut va);
        va.va_mode = 0o644;
        va.va_size = 0x1_0000_0002;
        va.va_mtime = Timespec::new(12_345, 0);
        let words = |full: bool| {
            let mut w = XdrWriter::reqhead(0);
            w.v3attrbuild(&va, full);
            let b = test_bytes(&w.into_chain());
            b.chunks(4)
                .map(|c| u32::from_be_bytes(c.try_into().unwrap()))
                .collect::<Vec<_>>()
        };
        // mode set; uid, gid unset; size only when full; atime unchanged; mtime to the client's.
        assert_eq!(
            words(true),
            [
                1,
                0o644,
                0,
                0,
                1,
                1,
                2,
                NFSV3SATTRTIME_DONTCHANGE,
                NFSV3SATTRTIME_TOCLIENT,
                12_345,
                0
            ]
        );
        assert_eq!(
            words(false),
            [
                1,
                0o644,
                0,
                0,
                0,
                NFSV3SATTRTIME_DONTCHANGE,
                NFSV3SATTRTIME_TOCLIENT,
                12_345,
                0
            ]
        );
    }

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
