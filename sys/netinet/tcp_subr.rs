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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! TCP support routines: `netinet/tcp_subr.c`, what stays in `bsd` once TCP is the kernel's
//! (N6.6a5): the `XF_TCPSIGNATURE` transform, IPsec's, until IPsec moves (N8).
//!
//! Upstream: sys/netinet/tcp_subr.c @ 3ce1f3f79392
//!
//! With `TCP_SIGNATURE`, the `tcp_signature_tdb_*` functions are the `XF_TCPSIGNATURE`
//! transform of IPsec's `xformsw[]` (`netinet/ip_ipsp.rs`), which only stores the key;
//! [`tdb_amxkey_lend`] lends that key to the kernel's `tcp_signature` through the `Ipsec`
//! bridge's leg (`net/if_enc.rs`), never copying it out. [`tcp_softtso_chop`] is the kernel's
//! software TSO (`kernel::netinet::tcp_tso`) over LZ's parked mbufs, for `bsd`'s IPsec output
//! halves (`ip_output.rs`, `ip6_output.rs`), until they move (N8).
//!
//! The rest of the file is the kernel's (`kernel::netinet::tcp_subr`, N6.6a2-N6.6a5).
//!
//! ## Deviations
//! - `tcp_signature_tdb_init` keeps the key in an `M_XDATA` allocation of `ii_authkeylen`
//!   bytes, wiped and freed by `tcp_signature_tdb_zeroize`, as the C.

use core::ptr::{self, NonNull};

use crate::kern::kern_malloc::malloc;
use crate::kern::uipc_mbuf::m_freem;
use crate::net::if_var::{Ifnet, Netstack};
use crate::netinet::in_::IPPROTO_DONE;
use crate::netinet::ip_ipsp::{IpsecInit, Tdb, Xformsw};
use crate::sys::errno::Errno;
use crate::sys::malloc::{M_NOWAIT, M_XDATA};
use crate::sys::mbuf::{Mbuf, MbufList, m_freemp};

/// `tcp_signature_tdb_attach` (`TCP_SIGNATURE`).
pub fn tcp_signature_tdb_attach() -> i32 {
    0
}

/// `tcp_signature_tdb_init`: keeps a copy of the key.
pub fn tcp_signature_tdb_init(
    tdbp: &Tdb,
    _xsp: &'static Xformsw,
    ii: &mut IpsecInit<'_>,
) -> Result<(), Errno> {
    if ii.ii_authkeylen < 1 || ii.ii_authkeylen > 80 {
        return Err(Errno::EINVAL);
    }

    let len = usize::from(ii.ii_authkeylen);
    let key = &ii.ii_authkey[..len.min(ii.ii_authkey.len())];
    let Some(p) = malloc(len, M_XDATA, M_NOWAIT) else {
        return Err(Errno::ENOMEM);
    };
    // SAFETY: a fresh allocation of `len` bytes; the key holds at most `len`.
    unsafe { ptr::copy_nonoverlapping(key.as_ptr(), p.as_ptr(), key.len()) };
    tdbp.tdb_amxkey.set(p.as_ptr());
    tdbp.tdb_amxkeylen.set(ii.ii_authkeylen);

    Ok(())
}

/// `tcp_signature_tdb_zeroize`.
pub fn tcp_signature_tdb_zeroize(tdbp: &Tdb) -> Result<(), Errno> {
    if let Some(p) = NonNull::new(tdbp.tdb_amxkey.get()) {
        let len = usize::from(tdbp.tdb_amxkeylen.get());
        // SAFETY: the key of `len` bytes `tcp_signature_tdb_init` allocated.
        libkern::explicit_bzero(unsafe { core::slice::from_raw_parts_mut(p.as_ptr(), len) });
        crate::kern::kern_malloc::free(p, M_XDATA, len);
        tdbp.tdb_amxkey.set(ptr::null_mut());
    }

    Ok(())
}

/// `tcp_signature_tdb_input`: the transform carries no packets.
pub fn tcp_signature_tdb_input(
    mp: &mut Option<&'static Mbuf>,
    _tdbp: &'static Tdb,
    _skip: i32,
    _protoff: i32,
    _ns: Option<&mut Netstack>,
) -> i32 {
    m_freemp(mp);
    IPPROTO_DONE
}

/// `tcp_signature_tdb_output`: the transform carries no packets.
pub fn tcp_signature_tdb_output(
    m: &'static Mbuf,
    _tdbp: &'static Tdb,
    _skip: i32,
    _protoff: i32,
) -> Result<(), Errno> {
    m_freem(m);
    Err(Errno::EINVAL)
}

/// The 20 bytes of a TCP header as the C structure holds them.

/// The SA's authentication key (`tdb->tdb_amxkey`, `tdb_amxkeylen` bytes) lent to `f`
/// (`tcp_signature`'s `MD5Update`, here and in the kernel's TCP through the `Ipsec` leg),
/// never copied out by this function. `tdb` is a counted reference of the SA (a
/// `kernel::net::ipsec_hooks::TdbRef`), borrowed for the call.
pub(crate) fn tdb_amxkey_lend(tdb: &kernel::net::ipsec_hooks::TdbRef, f: &mut dyn FnMut(&[u8])) {
    let sa = crate::net::if_enc::tdb_of(tdb);
    // SAFETY: `tdb_amxkey`'s contract is that the key is not freed while the slice lives;
    // the key is freed only by the transform's `xf_zeroize`, which only `tdb_free` calls
    // (sys/netinet/ip_ipsp.rs:2785-2789), which only the last `tdb_unref` calls (:2714-2721,
    // the C's ip_ipsp.c:1045-1052 and :1112-1118). `tdb` is a `TdbRef`: one value per
    // reference a lookup took (it is neither `Copy` nor `Clone`, and only the holder of the
    // registration's `TdbKey`, `bsd`'s legs and this file's callers wrapping their own
    // lookup's reference, can make one: sys/kernel/net/ipsec_hooks.rs, `TdbRef::new`), given
    // back only by value (`Ipsec::tdb_unref`, or the caller's own `tdb_unref` after the
    // borrow ends). This function borrows it for its whole body, so that reference is still
    // counted when `f` returns: the last `tdb_unref`, and with it the key's free, cannot have
    // run, and the slice stays valid for `f`.
    f(unsafe { sa.tdb_amxkey() });
}

/// `tcp_softtso_chop`: cuts the TSO packet `m0` into segments of at most `mss` bytes of
/// payload on `ml`; consumes `m0`, purges `ml` on error. The kernel's
/// (`kernel::netinet::tcp_tso`), over LZ's parked mbufs for `bsd`'s IPsec output halves
/// (`ip_output.rs`, `ip6_output.rs`, N8): the packet unparked for the call and the segments
/// handed to `ml` (moved here from `bsd`'s `tcp_output.rs` with `bsd`'s TCP, N6.6a5).
pub fn tcp_softtso_chop(
    ml: &MbufList,
    m0: &'static Mbuf,
    ifp: &'static Ifnet,
    mss: u32,
) -> Result<(), Errno> {
    let mut segs = kernel::net::mbuf::MbufList::new();
    let error = kernel::netinet::tcp_tso::tcp_softtso_chop(
        &mut segs,
        crate::sys::mbuf::unpark(m0),
        ifp,
        mss,
    );
    ml.change(|l| {
        kernel::net::mbuf::ml_init(l);
        kernel::net::mbuf::ml_enlist(l, &mut segs);
    });
    error
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the software TSO adapter over `bsd`'s parked mbufs (`tcp_softtso_chop`
    // over the kernel's: segment sizes, sequence numbers, flags, both checksums, the original
    // first; malformed packets; the kernel's own cases are `kernel::netinet::tcp_tso`'s
    // tests), moved from `bsd`'s `tcp_output.rs` with the adapter (N6.6a5).

    use core::cmp::min;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use super::*;
    use crate::kern::uipc_mbuf::{m_copydata, m_gethdr, ml_dequeue};
    use crate::net::if_::tests::test_ifnet;
    use crate::netinet::in_::IPPROTO_TCP;
    use crate::netinet::ip::IP_MF;
    use crate::netinet::tcp::{TH_ACK, TH_FIN, TH_PUSH};
    use crate::sys::mbuf::{
        M_DONTWAIT, M_EXT, M_TCP_CSUM_OUT, M_TCP_TSO, MCLBYTES, MT_DATA, mclget, ml_len, mtod,
    };
    const SRC: [u8; 4] = [10, 0, 2, 15];
    const DST: [u8; 4] = [10, 0, 2, 2];
    const SEQ: u32 = 0xffff_fe00; // wraps inside the packet

    /// A TSO packet: IPv4 (with header length `hl` words and `ip_off`), TCP with flags
    /// PUSH|ACK|FIN and sequence [`SEQ`], `payload` bytes; `ph_mss` is `mss`.
    fn tso_packet(payload: usize, mss: u16, hl: u8, ip_off: u16) -> &'static Mbuf {
        let iphlen = usize::from(hl) * 4;
        let len = iphlen + 20 + payload;
        let mut p = vec![0x40 | hl, 0];
        p.extend_from_slice(&(len as u16).to_be_bytes());
        p.extend_from_slice(&[0, 0]);
        p.extend_from_slice(&ip_off.to_be_bytes());
        p.extend_from_slice(&[64, IPPROTO_TCP as u8, 0, 0]);
        p.extend_from_slice(&SRC);
        p.extend_from_slice(&DST);
        p.resize(iphlen, 1); // option bytes (NOPs)
        p.extend_from_slice(&1234u16.to_be_bytes());
        p.extend_from_slice(&80u16.to_be_bytes());
        p.extend_from_slice(&SEQ.to_be_bytes());
        p.extend_from_slice(&7u32.to_be_bytes());
        p.extend_from_slice(&[5 << 4, TH_PUSH | TH_ACK | TH_FIN]);
        p.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0]);
        p.extend((0..payload).map(|i| i as u8));
        assert!(p.len() <= MCLBYTES);

        let m = m_gethdr(M_DONTWAIT, MT_DATA).expect("mbuf");
        mclget(m, M_DONTWAIT);
        assert!(m.m_flags().get() & M_EXT != 0);
        // SAFETY: a cluster of MCLBYTES bytes at m_data, which the packet fits.
        unsafe { ptr::copy_nonoverlapping(p.as_ptr(), mtod::<u8>(m), p.len()) };
        m.m_len().set(p.len() as u32);
        m.m_pkthdr().len.set(p.len() as i32);
        m.m_pkthdr().csum_flags.set(M_TCP_TSO | M_TCP_CSUM_OUT);
        m.m_pkthdr().ph_mss.set(mss);
        m
    }

    /// The bytes of packet `m`.
    fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut v = vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut v);
        v
    }

    /// The ones' complement sum of `words` (16-bit, big-endian; an odd byte padded), folded.
    fn sum16(words: &[u8]) -> u16 {
        let mut sum: u32 = words
            .chunks(2)
            .map(|c| u32::from(u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])))
            .sum();
        while sum > 0xffff {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        sum as u16
    }

    /// Checks that `segs` are `payload` bytes cut at `mss`, with consecutive sequence numbers,
    /// PUSH and FIN only on the last, the IP lengths and both checksums right.
    fn check_segments(segs: &[Vec<u8>], payload: usize, mss: usize) {
        assert_eq!(segs.len(), payload.div_ceil(mss));
        for (k, b) in segs.iter().enumerate() {
            let dlen = min(mss, payload - k * mss);
            assert_eq!(b.len(), 40 + dlen, "segment {k}");
            assert_eq!(u16::from_be_bytes([b[2], b[3]]) as usize, 40 + dlen);
            assert_eq!(&b[12..20], &[SRC, DST].concat()[..]);
            let seq = u32::from_be_bytes([b[24], b[25], b[26], b[27]]);
            assert_eq!(seq, SEQ.wrapping_add((k * mss) as u32), "segment {k}");
            let last = k + 1 == segs.len();
            let want = if last {
                TH_PUSH | TH_ACK | TH_FIN
            } else {
                TH_ACK
            };
            assert_eq!(b[33], want, "flags of segment {k}");
            for (i, &x) in b[40..].iter().enumerate() {
                assert_eq!(x, (k * mss + i) as u8);
            }
            assert_eq!(sum16(&b[..20]), 0xffff, "IP checksum of segment {k}");
            let mut pseudo = [SRC, DST].concat();
            pseudo.extend_from_slice(&[0, IPPROTO_TCP as u8]);
            pseudo.extend_from_slice(&((20 + dlen) as u16).to_be_bytes());
            pseudo.extend_from_slice(&b[20..]);
            assert_eq!(sum16(&pseudo), 0xffff, "TCP checksum of segment {k}");
        }
    }

    #[test]
    fn softtso_chop_cuts_at_mss() {
        let (_g, _t) = crate::netinet::ip_input::tests::setup();
        let ifp = test_ifnet(b"ttso0");

        let m0 = tso_packet(1300, 500, 5, 0);
        let ml = MbufList::new();
        assert_eq!(tcp_softtso_chop(&ml, m0, ifp, 500), Ok(()));
        assert_eq!(ml_len(&ml), 3);
        let mut segs = Vec::new();
        while let Some(m) = ml_dequeue(&ml) {
            segs.push(m);
        }
        assert!(
            ptr::eq(segs[0], m0),
            "the first segment is the original packet"
        );
        for m in &segs {
            assert_eq!(m.m_pkthdr().csum_flags.get() & M_TCP_TSO, 0);
        }
        check_segments(
            &segs.iter().map(|m| bytes(m)).collect::<Vec<_>>(),
            1300,
            500,
        );
        for m in segs {
            m_freem(m);
        }

        // A packet that fits in one segment keeps its flags.
        let m0 = tso_packet(300, 500, 5, 0);
        assert_eq!(tcp_softtso_chop(&ml, m0, ifp, 500), Ok(()));
        let m = ml_dequeue(&ml).expect("segment");
        assert!(ml_dequeue(&ml).is_none());
        check_segments(&[bytes(m)], 300, 500);
        m_freem(m);
    }

    #[test]
    fn softtso_chop_rejects_what_it_cannot_cut() {
        let (_g, _t) = crate::netinet::ip_input::tests::setup();
        let ifp = test_ifnet(b"ttso1");
        let ml = MbufList::new();

        let cases = [
            (tso_packet(1000, 500, 5, 0), 0, Errno::EINVAL),
            (tso_packet(1000, 500, 5, IP_MF), 500, Errno::EPROTOTYPE),
            (tso_packet(1000, 500, 5, 1), 500, Errno::EPROTOTYPE),
            (tso_packet(1000, 500, 6, 0), 500, Errno::EPROTOTYPE),
        ];
        for (m0, mss, error) in cases {
            assert_eq!(tcp_softtso_chop(&ml, m0, ifp, mss), Err(error));
            assert_eq!(ml_len(&ml), 0, "the list is purged");
        }

        // Shorter than the headers it claims.
        let m0 = tso_packet(0, 500, 5, 0);
        m0.m_pkthdr().len.set(30);
        assert_eq!(tcp_softtso_chop(&ml, m0, ifp, 500), Err(Errno::ENOPROTOOPT));
        assert_eq!(ml_len(&ml), 0);
    }
}
/* </TESTS> */
