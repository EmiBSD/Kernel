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
 *	@(#)nfsm_subs.h	8.2 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfsm_subs.h>` in the kernel: the cursors an RPC is built and taken apart with. A
//! reply is read by an [`XdrReader`] (the C's `nmi_md`/`nmi_dpos`, `nfsm_dissect`, `nfsm_adv`,
//! `nfsm_strsiz`, `nfsm_mtouio`), a request built by an [`XdrWriter`] (the C's `struct mbuf
//! **mb`, `nfsm_build`, `nfsm_strtom`). The functions of `nfs_subs.c` the header's inlines
//! call (`nfsm_reqhead`, `nfsm_uiotombuf`, `nfsm_strtombuf`, `nfsm_fhtom`, `nfsm_mbuftouio`,
//! ...) are methods of the same cursors in `nfs_subs`.
//!
//! Upstream: sys/nfs/nfsm_subs.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfsm_subs.rs@3c62ede68802
//!
//! ## Deviations
//! - `int *nmi_errorp` is gone: every reader method returns `Result` and the RPC's owner
//!   frees the reply when it drops it (the C's `m_freem(nmi_mrep); nmi_mrep = NULL` before
//!   it stores the error); the caller's `goto nfsmout` is `?`.
//! - `nfsm_postop_attr` (it loads the attribute cache) comes with the cache (N6.7a2); the
//!   server's `nfsd_*` twins stay `bsd`'s until N6.7b.
//!
//! ## Redesign
//! - LZ's cursor was an mbuf and a raw position into its data (`nmi_dpos: *mut u8`, checked
//!   by `nfsm_avail` before each use), and LZ's views (`XdrIn`, `XdrOut`) pointers into the
//!   mbufs with five `unsafe`. An [`XdrReader`] borrows the reply's chain and holds an mbuf and
//!   an offset into its data; every read copies the bytes out (a word, a fixed array, a wire
//!   structure, a caller's buffer), so nothing it returns points into an mbuf and the chain is
//!   never changed. Where the C's `nfsm_disct` makes straddling bytes contiguous by inserting
//!   an mbuf (C:nfs/nfs_subs.c:797-849), the reader copies across the boundary; its
//!   `panic("nfs S too big")` has no counterpart (a fixed `N`, or a caller's buffer). The
//!   reader counts the bytes consumed ([`XdrReader::offset`]) and starts at any offset of a
//!   chain ([`XdrReader::at`]): an RPC's reply keeps that number, not a cursor (N6.7a1, and the
//!   server's request at N6.7b2).
//! - An [`XdrWriter`] owns the chain it builds: the finished mbufs behind
//!   (`MChainBuilder`, O(1) a step) and the one being filled (the C's `*mb`, the chain's
//!   tail). A request is a whole `MbufBox` when it is done ([`XdrWriter::into_chain`]); one
//!   dropped unfinished is freed. A writer may start over a first mbuf its caller made
//!   ([`XdrWriter::with_first`]: the RPC header, the server's `nfs_rephead`).
//! - Every length from the wire is an `i32` compared as the C compares it (`nfsm_strsiz`'s
//!   `len < 0 || len > maxlen`) before it becomes a size, and becomes one by `usize::try_from`,
//!   never `as`; a read past the chain's end is `EBADRPC`, never a panic.

use core::slice;

use hal::user::{AbiPod, bytes_of_mut, zeroed};

use crate::assert::kpanic;
use crate::net::mbuf::{
    M_WAIT, MChainBuilder, MLEN, MT_DATA, Mbuf, MbufBox, m_get, m_trailingspace,
};
use crate::sys::errno::Errno;

/// A reply's dissection cursor (the C's `nmi_md` and `nmi_dpos`): the mbuf it is in and the
/// offset of the next byte in that mbuf's data, over a chain it borrows and never changes.
#[derive(Clone)]
pub struct XdrReader<'a> {
    /// `nmi_md`: the mbuf the next byte is in (or past whose data the cursor stands).
    md: &'a Mbuf,
    /// `nmi_dpos` as an offset into `md.data()`, at most its length.
    off: usize,
    /// The bytes consumed since the chain's first: where [`XdrReader::at`] starts again.
    abs: usize,
}

impl<'a> XdrReader<'a> {
    /// `info.nmi_md = m; info.nmi_dpos = mtod(m, caddr_t)`: a cursor at the chain's first byte.
    pub fn new(m: &'a Mbuf) -> Self {
        Self {
            md: m,
            off: 0,
            abs: 0,
        }
    }

    /// A cursor `off` bytes into the chain at `m` (a reply whose header was read before, the
    /// server's request after its RPC header): `EBADRPC` past the chain's end.
    pub fn at(m: &'a Mbuf, off: usize) -> Result<Self, Errno> {
        let mut r = Self::new(m);
        r.adv(off)?;
        Ok(r)
    }

    /// The bytes consumed since the chain's first.
    pub fn offset(&self) -> usize {
        self.abs
    }

    /// The bytes left in the current mbuf (the C's `avail`).
    fn avail(&self) -> usize {
        self.md.data().len() - self.off
    }

    /// `nfsm_adv(infop, s)` and `nfs_adv`: skips `s` bytes, across mbufs; `EBADRPC` when the
    /// chain ends first. A skip that ends exactly at the chain's end is not an error, as in C.
    pub fn adv(&mut self, s: usize) -> Result<(), Errno> {
        let mut offs = s;
        let mut left = self.avail();
        let mut md = self.md;
        let mut base = self.off;
        while left < offs {
            offs -= left;
            md = md.m_next().get().ok_or(Errno::EBADRPC)?;
            left = md.data().len();
            base = 0;
        }
        self.md = md;
        self.off = base + offs;
        self.abs += s;
        Ok(())
    }

    /// `nfsm_dissect` into a caller's buffer: the next `dst.len()` bytes, copied across
    /// mbufs as they lie (the C's `nfsm_disct` gathers them); `EBADRPC` when the chain ends
    /// first. A handle, a name, a verifier: the caller has bounded `dst` by the C's check.
    pub fn bytes_into(&mut self, dst: &mut [u8]) -> Result<(), Errno> {
        let mut done = 0;
        while done < dst.len() {
            let avail = self.avail();
            if avail == 0 {
                self.md = self.md.m_next().get().ok_or(Errno::EBADRPC)?;
                self.off = 0;
                continue;
            }
            let n = avail.min(dst.len() - done);
            dst[done..done + n].copy_from_slice(&self.md.data()[self.off..self.off + n]);
            self.off += n;
            self.abs += n;
            done += n;
        }
        Ok(())
    }

    /// `nfsm_dissect(infop, N)`: the next `N` bytes, copied out.
    pub fn get<const N: usize>(&mut self) -> Result<[u8; N], Errno> {
        let mut b = [0u8; N];
        self.bytes_into(&mut b)?;
        Ok(b)
    }

    /// `*tl` of `tl = nfsm_dissect(infop, NFSX_UNSIGNED)`: the next raw XDR word, as it lies
    /// (compared with the `rpc_*` and `nfs_true` words, or kept as an xid).
    pub fn word(&mut self) -> Result<u32, Errno> {
        Ok(u32::from_ne_bytes(self.get::<4>()?))
    }

    /// `fxdr_unsigned(u_int32_t, *tl)`: the next word's host value.
    pub fn u32(&mut self) -> Result<u32, Errno> {
        Ok(u32::from_be_bytes(self.get::<4>()?))
    }

    /// `fxdr_unsigned(int32_t, *tl)`: the next word as the C's signed `int`.
    pub fn i32(&mut self) -> Result<i32, Errno> {
        Ok(i32::from_be_bytes(self.get::<4>()?))
    }

    /// `fxdr_hyper(tl)`: the next two words as a 64-bit value, most significant first.
    pub fn hyper(&mut self) -> Result<u64, Errno> {
        Ok(u64::from_be_bytes(self.get::<8>()?))
    }

    /// `(struct T *)nfsm_dissect(infop, sizeof(struct T))`: a wire structure, copied out.
    pub fn pod<T: AbiPod>(&mut self) -> Result<T, Errno> {
        self.pod_len(size_of::<T>())
    }

    /// `(struct T *)nfsm_dissect(infop, len)` where the version decides `len` (a version 2
    /// `nfs_fattr` is 68 of the structure's 84 bytes): `len` bytes consumed, the first
    /// `min(len, size_of::<T>())` of them in the structure, the rest of it zero.
    pub fn pod_len<T: AbiPod>(&mut self, len: usize) -> Result<T, Errno> {
        let mut v = zeroed::<T>();
        let b = bytes_of_mut(slice::from_mut(&mut v));
        let n = len.min(b.len());
        self.bytes_into(&mut b[..n])?;
        self.adv(len - n)?;
        Ok(v)
    }

    /// `nfsm_strsiz(infop, lenp, maxlen)`: a string's length word, `EBADRPC` when it is
    /// negative or above `maxlen`, compared as the C's `int`s before it is used as a size.
    pub fn strsiz(&mut self, maxlen: i32) -> Result<i32, Errno> {
        let len = self.i32()?;
        if len < 0 || len > maxlen {
            return Err(Errno::EBADRPC);
        }
        Ok(len)
    }

    /// `nfsm_mtouio(infop, uiop, len)`: `len` bytes into `uio`, then their XDR padding
    /// skipped; nothing when `len <= 0`. `EFBIG` when `uio` holds less, `EBADRPC` when the
    /// chain ends first (`nfsm_mbuftouio`, `nfs_subs`).
    pub fn to_uio<G: crate::proc::ProcGlue>(
        &mut self,
        uio: &mut crate::uio::Uio<'_, G>,
        len: i32,
    ) -> Result<(), Errno> {
        let Ok(siz) = usize::try_from(len) else {
            return Ok(());
        };
        if siz == 0 {
            return Ok(());
        }
        self.mbuftouio(uio, siz)
    }

    /// The bytes of the current mbuf's data at the cursor, at most `max`, after the empty
    /// mbufs are skipped; the cursor moves past them (`EBADRPC` at the chain's end when bytes
    /// are wanted). The walk `nfsm_mbuftouio` makes, piece by piece.
    pub(super) fn next_piece(&mut self, max: usize) -> Result<&'a [u8], Errno> {
        while self.avail() == 0 && max > 0 {
            self.md = self.md.m_next().get().ok_or(Errno::EBADRPC)?;
            self.off = 0;
        }
        let md = self.md;
        let n = self.avail().min(max);
        let p = &md.data()[self.off..self.off + n];
        self.off += n;
        self.abs += n;
        Ok(p)
    }
}

/// A request's build cursor (the C's `struct mbuf **mb`, the chain's last mbuf): the chain
/// built so far, which it owns, and the mbuf being filled.
pub struct XdrWriter {
    /// The full mbufs, first to last.
    done: MChainBuilder,
    /// `*mb`: the mbuf being filled, the chain's tail.
    cur: MbufBox,
}

impl XdrWriter {
    /// A writer that appends after the data of `first`, an mbuf its caller made (the RPC
    /// header's packet header mbuf, the server's reply head).
    pub fn with_first(first: MbufBox) -> Self {
        Self {
            done: MChainBuilder::new(),
            cur: first,
        }
    }

    /// The bytes the current mbuf can still take (`m_trailingspace(*mb)`).
    pub(super) fn trailing(&self) -> usize {
        usize::try_from(m_trailingspace(&self.cur)).unwrap_or(0)
    }

    /// `MGET(mb2, M_WAIT, MT_DATA); [MCLGET(mb2, M_WAIT);] mb2->m_len = 0; mb->m_next = mb2;
    /// mb = mb2`: a fresh mbuf (with a cluster when `cluster`) becomes the tail. `M_WAIT`
    /// cannot fail in the C; an exhausted pool here is `panic(9)`.
    pub(super) fn grow(&mut self, cluster: bool) {
        let Some(mut m) = m_get(M_WAIT, MT_DATA) else {
            kpanic(format_args!("nfs: out of mbufs"));
        };
        if cluster {
            crate::net::mbuf::mclget(&mut m, M_WAIT);
        }
        m.m_len().set(0);
        let old = core::mem::replace(&mut self.cur, m);
        self.done.push(old);
    }

    /// The `len` bytes after the tail's data, added to its length (`mb_offset(mb)` and
    /// `mb->m_len += len`): the caller has made sure they fit (`len <= trailing()`).
    pub(super) fn take(&mut self, len: usize) -> &mut [u8] {
        let start = self.cur.data_off() + self.cur.m_len().get() as usize;
        let Ok(l32) = u32::try_from(len) else {
            kpanic(format_args!("nfsm_build: {len} bytes"));
        };
        self.cur.m_len().set(self.cur.m_len().get() + l32);
        match self
            .cur
            .storage_mut()
            .and_then(|s| s.get_mut(start..start + len))
        {
            Some(d) => d,
            None => kpanic(format_args!(
                "nfsm_build: {len} bytes past the mbuf's storage"
            )),
        }
    }

    /// `nfsm_build(mb, N)`: `N` contiguous bytes at the chain's end, a fresh mbuf taken when
    /// the tail cannot hold them (`panic("build > MLEN")` past one mbuf, as the C).
    pub fn build<const N: usize>(&mut self) -> &mut [u8; N] {
        match self.build_slice(N).first_chunk_mut::<N>() {
            Some(b) => b,
            None => kpanic(format_args!("nfsm_build: short")),
        }
    }

    /// `nfsm_build(mb, len)` of a length known only at run time.
    pub fn build_slice(&mut self, len: usize) -> &mut [u8] {
        if len > self.trailing() {
            if len > MLEN {
                kpanic(format_args!("build > MLEN"));
            }
            self.grow(false);
        }
        self.take(len)
    }

    /// `*tl = w`: one raw XDR word (an `rpc_*` or `nfs_true` word, an xid).
    pub fn word(&mut self, w: u32) {
        *self.build::<4>() = w.to_ne_bytes();
    }

    /// `*tl = txdr_unsigned(v)`.
    pub fn u32(&mut self, v: u32) {
        *self.build::<4>() = v.to_be_bytes();
    }

    /// `txdr_hyper(v, tl)`.
    pub fn hyper(&mut self, v: u64) {
        *self.build::<8>() = v.to_be_bytes();
    }

    /// `bcopy(&v, nfsm_build(mb, sizeof(v)), sizeof(v))`: a wire structure.
    pub fn pod<T: AbiPod>(&mut self, v: &T) {
        let b = super::xdr_subs::xdr_bytes(v);
        self.build_slice(b.len()).copy_from_slice(b);
    }

    /// `nfsm_strtom(infop, mb, str, len, maxlen)`: the XDR string `s` (its length, its bytes,
    /// its padding); `ENAMETOOLONG` when it is longer than `maxlen`, nothing written.
    pub fn strtom(&mut self, s: &[u8], maxlen: usize) -> Result<(), Errno> {
        if s.len() > maxlen {
            return Err(Errno::ENAMETOOLONG);
        }
        self.strtombuf(s);
        Ok(())
    }

    /// The finished chain, its first mbuf owning the rest.
    pub fn into_chain(self) -> MbufBox {
        let Self { mut done, cur } = self;
        done.push(cur);
        match done.into_head() {
            Some(m) => m,
            None => kpanic(format_args!("nfs: an empty request chain")),
        }
    }
}

/// `nfsm_rndup(a)`: `a` rounded up to a whole XDR word.
pub const fn nfsm_rndup(a: usize) -> usize {
    (a + 3) & !0x3
}

/// `nfsm_padlen(s)` (`nfs_var.h`): the XDR padding after `s` bytes.
pub const fn nfsm_padlen(s: usize) -> usize {
    nfsm_rndup(s) - s
}
/// A chain of the given segments, one mbuf each (an empty segment an mbuf with no data), for
/// the tests of the cursors here and in `nfs_subs`.
#[cfg(test)]
pub(crate) fn test_chain(segs: &[&[u8]]) -> MbufBox {
    let mut b = MChainBuilder::new();
    for s in segs {
        let Some(mut m) = m_get(M_WAIT, MT_DATA) else {
            kpanic(format_args!("test_chain: no mbuf"));
        };
        if s.len() > MLEN {
            crate::net::mbuf::mclget(&mut m, M_WAIT);
        }
        m.m_len().set(s.len() as u32);
        if let Some(d) = m.data_mut() {
            d.copy_from_slice(s);
        }
        b.push(m);
    }
    match b.into_head() {
        Some(m) => m,
        None => kpanic(format_args!("test_chain: no segment")),
    }
}

/// The chain's bytes, first to last.
#[cfg(test)]
pub(crate) fn test_bytes(m: &Mbuf) -> std::vec::Vec<u8> {
    let mut v = std::vec::Vec::new();
    let mut n = Some(m);
    while let Some(x) = n {
        v.extend_from_slice(x.data());
        n = x.m_next().get();
    }
    v
}

/// The lengths of the chain's mbufs, first to last.
#[cfg(test)]
pub(crate) fn test_lens(m: &Mbuf) -> std::vec::Vec<usize> {
    let mut v = std::vec::Vec::new();
    let mut n = Some(m);
    while let Some(x) = n {
        v.push(x.data().len());
        n = x.m_next().get();
    }
    v
}

/// A seeded xorshift64* sequence: the property tests' cases, the same on every run.
#[cfg(test)]
pub(crate) struct TestRng(pub(crate) u64);

#[cfg(test)]
impl TestRng {
    /// The next value.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A value in `0..n` (`n > 0`).
    pub(crate) fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// `n` bytes.
    pub(crate) fn bytes(&mut self, n: usize) -> std::vec::Vec<u8> {
        (0..n).map(|_| self.next_u64() as u8).collect()
    }

    /// `data` cut at random places into segments, some empty.
    pub(crate) fn split<'b>(&mut self, data: &'b [u8]) -> std::vec::Vec<&'b [u8]> {
        let mut segs = std::vec::Vec::new();
        let mut at = 0;
        loop {
            let n = if self.below(5) == 0 {
                0
            } else {
                self.below(40)
            };
            let n = n.min(data.len() - at);
            segs.push(&data[at..at + n]);
            at += n;
            if at == data.len() && self.below(3) != 0 {
                return segs;
            }
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the cursors: reads across mbufs against a flat decoder, the bounds of
    // every length, the writer's chain against the bytes it was given.

    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::net::mbuf::tests::setup;
    use crate::sys::nfs_nfsproto::{NFSX_V2FATTR, NFSX_V3FATTR, NfsFattr, Nfsv3Time};

    #[test]
    fn reads_cross_mbufs_and_skip_empty_ones() {
        let _g = setup();
        let data: Vec<u8> = (0u8..40).collect();
        let m = test_chain(&[&data[..3], &[], &data[3..5], &[], &[], &data[5..40]]);
        let mut r = XdrReader::new(&m);
        assert_eq!(r.get::<4>().unwrap(), [0, 1, 2, 3]);
        assert_eq!(r.word().unwrap(), u32::from_ne_bytes([4, 5, 6, 7]));
        assert_eq!(r.u32().unwrap(), 0x0809_0a0b);
        assert_eq!(r.i32().unwrap(), 0x0c0d_0e0f);
        assert_eq!(r.hyper().unwrap(), 0x1011_1213_1415_1617);
        assert_eq!(r.offset(), 24);
        r.adv(15).unwrap();
        assert_eq!(r.get::<1>().unwrap(), [39]);
        // At the end: nothing more to read, a zero skip is fine, one more byte is not.
        r.adv(0).unwrap();
        assert_eq!(r.get::<1>(), Err(Errno::EBADRPC));
    }

    #[test]
    fn adv_to_the_end_is_fine_and_past_it_is_ebadrpc() {
        let _g = setup();
        let data = [7u8; 10];
        let m = test_chain(&[&data[..4], &data[4..]]);
        let mut r = XdrReader::new(&m);
        r.adv(10).unwrap();
        assert_eq!(r.offset(), 10);
        let mut r = XdrReader::new(&m);
        assert_eq!(r.adv(11), Err(Errno::EBADRPC));
        assert!(XdrReader::at(&m, 10).is_ok());
        assert_eq!(XdrReader::at(&m, 11).err(), Some(Errno::EBADRPC));
        let mut r = XdrReader::at(&m, 6).unwrap();
        assert_eq!((r.offset(), r.get::<4>().unwrap()), (6, [7; 4]));
    }

    #[test]
    fn a_wire_structure_straddling_mbufs_is_copied_whole() {
        let _g = setup();
        let mut fa = NfsFattr::default();
        fa.fa_type = 1u32.to_be();
        fa.set_fa3_mtime(Nfsv3Time::from_words([5u32.to_be(), 6u32.to_be()]));
        let b = crate::fs::nfs::xdr_subs::xdr_bytes(&fa).to_vec();
        let m = test_chain(&[&b[..1], &b[1..50], &b[50..]]);
        let mut r = XdrReader::new(&m);
        assert_eq!(r.pod::<NfsFattr>().unwrap(), fa);
        // A version 2 reply: 68 bytes read, the rest of the structure zero.
        let m = test_chain(&[&b[..30], &b[30..]]);
        let mut r = XdrReader::new(&m);
        let v2: NfsFattr = r.pod_len(NFSX_V2FATTR).unwrap();
        assert_eq!(r.offset(), NFSX_V2FATTR);
        assert_eq!(v2.fa_un[13..], [0, 0, 0]);
        assert_eq!(v2.fa_type, fa.fa_type);
        let mut r = XdrReader::new(&m);
        assert_eq!(r.pod_len::<NfsFattr>(NFSX_V3FATTR + 4), Err(Errno::EBADRPC));
    }

    #[test]
    fn strsiz_takes_the_c_bounds() {
        let _g = setup();
        // (wire length, maxlen, result): `len < 0 || len > maxlen` is EBADRPC.
        let cases: &[(i32, i32, Result<i32, Errno>)] = &[
            (0, 0, Ok(0)),
            (255, 255, Ok(255)),
            (256, 255, Err(Errno::EBADRPC)),
            (-1, 255, Err(Errno::EBADRPC)),
            (i32::MIN, i32::MAX, Err(Errno::EBADRPC)),
            (i32::MAX, i32::MAX, Ok(i32::MAX)),
            (1024, 1024, Ok(1024)),
        ];
        for &(len, max, want) in cases {
            let w = len.to_be_bytes();
            let m = test_chain(&[&w[..1], &w[1..]]);
            assert_eq!(XdrReader::new(&m).strsiz(max), want, "{len} {max}");
        }
        let m = test_chain(&[&[0, 0, 0]]);
        assert_eq!(XdrReader::new(&m).strsiz(10), Err(Errno::EBADRPC));
    }

    #[test]
    fn property_reader_equals_a_flat_decoder_over_any_split() {
        let _g = setup();
        let mut rng = TestRng(0x6e66_735f_7864_7231);
        for _ in 0..1500 {
            let n = rng.below(200);
            let data = rng.bytes(n);
            let m = test_chain(&rng.split(&data));
            assert_eq!(test_bytes(&m), data);
            let mut r = XdrReader::new(&m);
            let mut pos = 0usize;
            for _ in 0..rng.below(30) {
                let k = rng.below(24);
                let (ok, used) = match rng.below(4) {
                    0 => {
                        let mut b = vec![0u8; k];
                        let ok = r.bytes_into(&mut b).is_ok();
                        assert_eq!(ok, pos + k <= data.len());
                        if ok {
                            assert_eq!(b, data[pos..pos + k]);
                        }
                        (ok, k)
                    }
                    1 => {
                        let ok = r.adv(k).is_ok();
                        assert_eq!(ok, pos + k <= data.len());
                        (ok, k)
                    }
                    2 => {
                        let got = r.u32();
                        assert_eq!(got.is_ok(), pos + 4 <= data.len());
                        if let Ok(v) = got {
                            assert_eq!(v.to_be_bytes(), data[pos..pos + 4]);
                        }
                        (got.is_ok(), 4)
                    }
                    _ => {
                        let got = r.hyper();
                        assert_eq!(got.is_ok(), pos + 8 <= data.len());
                        if let Ok(v) = got {
                            assert_eq!(v.to_be_bytes(), data[pos..pos + 8]);
                        }
                        (got.is_ok(), 8)
                    }
                };
                // A failed read ends the RPC (the C frees the reply): the sequence stops.
                if !ok {
                    break;
                }
                pos += used;
                assert_eq!(r.offset(), pos);
                // The same place, found again from the chain's start (an RPC's reply).
                let mut again = XdrReader::at(&m, pos).unwrap();
                if pos < data.len() {
                    assert_eq!(again.get::<1>().unwrap(), [data[pos]]);
                }
            }
        }
    }

    #[test]
    fn the_writer_builds_words_and_grows_the_chain() {
        let _g = setup();
        let mut w = XdrWriter::with_first(test_chain(&[&[9, 9]]));
        w.word(0x0102_0304u32.to_be());
        w.u32(5);
        w.hyper(0x0607_0809_0a0b_0c0d);
        w.pod(&Nfsv3Time::from_words([1, 2]));
        let mut want = vec![9, 9, 1, 2, 3, 4, 0, 0, 0, 5];
        want.extend_from_slice(&0x0607_0809_0a0b_0c0du64.to_be_bytes());
        want.extend_from_slice(crate::fs::nfs::xdr_subs::xdr_bytes(&Nfsv3Time::from_words(
            [1, 2],
        )));
        // Fill past the first mbuf: a build never straddles, the next one takes it whole.
        for i in 0..(MLEN / 4 + 3) as u32 {
            w.u32(i);
            want.extend_from_slice(&i.to_be_bytes());
        }
        let m = w.into_chain();
        assert_eq!(test_bytes(&m), want);
        assert!(test_lens(&m).len() >= 2);
        assert!(test_lens(&m).iter().all(|l| l % 2 == 0 && *l <= MLEN));
    }

    #[test]
    fn strtom_refuses_a_long_string_and_pads_a_short_one() {
        let _g = setup();
        let mut w = XdrWriter::with_first(test_chain(&[&[]]));
        assert_eq!(w.strtom(b"abcdef", 5), Err(Errno::ENAMETOOLONG));
        w.strtom(b"abcde", 5).unwrap();
        let m = w.into_chain();
        assert_eq!(
            test_bytes(&m),
            [0, 0, 0, 5, b'a', b'b', b'c', b'd', b'e', 0, 0, 0]
        );
    }

    #[test]
    fn rndup_and_padlen() {
        for (a, up, pad) in [(0, 0, 0), (1, 4, 3), (3, 4, 1), (4, 4, 0), (5, 8, 3)] {
            assert_eq!((nfsm_rndup(a), nfsm_padlen(a)), (up, pad));
        }
    }
}
/* </TESTS> */
