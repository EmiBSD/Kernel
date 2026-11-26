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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)ip_icmp.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
//! ICMP (`netinet/ip_icmp.c`), the kernel's since N6.4c2 (`kernel::netinet::ip_icmp`; the
//! header, `<netinet/ip_icmp.h>`, is `sysdefs::netinet::ip_icmp`): the kernel's items under
//! LZ's path, LZ's signatures over parked mbufs for `bsd`'s callers (pf, the transports, the
//! protocol table's ICMP entry), and [`IcmpPkt`], `bsd`'s view of a message inside a packet
//! for its readers (TCP's path MTU, the tests) until they move.
//!
//! Upstream: sys/netinet/ip_icmp.h @ 3ce1f3f79392
//! Upstream: sys/netinet/ip_icmp.c @ 3ce1f3f79392
//!
//! ## Deviations
//! - The message inside a packet is reached through [`IcmpPkt`], bounds-checked reads and
//!   writes at the C's member offsets: the packet may be shorter than `struct icmp` (8 bytes
//!   for an echo) and its data has no alignment guarantee, so no `&Icmp` is made over mbuf
//!   data. The C's `ICMP_ADVLEN`-style length checks come first, as in C, and the accessor
//!   panics where the C would read past them.

use core::mem::offset_of;
use core::mem::size_of;
use core::ptr;

use crate::kern::subr_prf::panic;
use crate::net::if_::net_held;
use crate::net::route::Rtentry;
use crate::netinet::in_::InAddr;
use crate::netinet::ip::Ip;
use crate::sys::mbuf::{Mbuf, mtod, unpark};

pub use kernel::netinet::icmp_var::ICMPCOUNTERS;
pub use kernel::netinet::ip_icmp::{
    ICMP_REDIRACCEPT, ICMP_REDIRECT_TIMEOUT_Q, ICMP_REDIRTIMEOUT, ICMPBMCASTECHO, ICMPERRPPSLIM,
    ICMPMASKREPL, ICMPTSTAMPREPL, IP_MTUDISC_TIMEOUT_Q, iptime,
};
pub use sysdefs::netinet::ip_icmp::*;

/// The offset of `icmp_hun` (the second word).
const ICMP_HUN: usize = ICMP_HUN_OFFSET;
/// The offset of `icmp_dun` (the data).
const ICMP_DUN: usize = ICMP_IP_OFFSET;

/// An ICMP message inside a packet: `len` contiguous bytes from its type byte on, read and
/// written at the members' offsets (see the module's deviations).
#[derive(Clone, Copy)]
pub struct IcmpPkt {
    p: *mut u8,
    len: usize,
}

impl IcmpPkt {
    /// The message at `p`, of which `len` bytes are contiguous.
    ///
    /// # Safety
    ///
    /// `p` points at `len` readable and writable bytes (an mbuf's data), valid as long as the
    /// view is used.
    pub unsafe fn new(p: *mut u8, len: usize) -> Self {
        Self { p, len }
    }

    /// The message at offset `off` of `m`'s first mbuf, up to that mbuf's end.
    pub fn of(m: &Mbuf, off: usize) -> Self {
        let mlen = m.m_len().get() as usize;
        if off > mlen {
            panic(format_args!("icmp: header at {off} of {mlen}"));
        }
        // SAFETY: the first mbuf holds `m_len` bytes at its data pointer.
        unsafe { Self::new(mtod::<u8>(m).add(off), mlen - off) }
    }

    /// The address of member bytes `off .. off + size_of::<T>()`.
    fn at<T>(&self, off: usize) -> *mut T {
        if off + size_of::<T>() > self.len {
            panic(format_args!("icmp: access at {off} of {}", self.len));
        }
        self.p.wrapping_add(off).cast()
    }

    fn get<T: Copy>(&self, off: usize) -> T {
        // SAFETY: `at` checked the bounds; the members are integers, valid for any bytes.
        unsafe { ptr::read_unaligned(self.at::<T>(off)) }
    }

    fn set<T>(&self, off: usize, v: T) {
        // SAFETY: `at` checked the bounds of the view's writable bytes.
        unsafe { ptr::write_unaligned(self.at::<T>(off), v) };
    }

    /// The message's first byte.
    pub fn as_ptr(&self) -> *mut u8 {
        self.p
    }

    /// `icmp_type`.
    pub fn icmp_type(&self) -> u8 {
        self.get(offset_of!(Icmp, icmp_type))
    }

    /// Sets `icmp_type`.
    pub fn set_icmp_type(&self, v: u8) {
        self.set(offset_of!(Icmp, icmp_type), v);
    }

    /// `icmp_code`.
    pub fn icmp_code(&self) -> u8 {
        self.get(offset_of!(Icmp, icmp_code))
    }

    /// Sets `icmp_code`.
    pub fn set_icmp_code(&self, v: u8) {
        self.set(offset_of!(Icmp, icmp_code), v);
    }

    /// Sets `icmp_cksum`.
    pub fn set_icmp_cksum(&self, v: u16) {
        self.set(ICMP_CKSUM_OFFSET, v);
    }

    /// Sets `icmp_pptr`.
    pub fn set_icmp_pptr(&self, v: u8) {
        self.set(ICMP_HUN, v);
    }

    /// `icmp_length` (RFC 4884).
    pub fn icmp_length(&self) -> u8 {
        self.get(ICMP_HUN + offset_of!(IhExthdr, iex_length))
    }

    /// Sets `icmp_length`.
    pub fn set_icmp_length(&self, v: u8) {
        self.set(ICMP_HUN + offset_of!(IhExthdr, iex_length), v);
    }

    /// `icmp_gwaddr`.
    pub fn icmp_gwaddr(&self) -> InAddr {
        self.get(ICMP_HUN)
    }

    /// Sets `icmp_gwaddr`.
    pub fn set_icmp_gwaddr(&self, v: InAddr) {
        self.set(ICMP_HUN, v);
    }

    /// `icmp_id`, network order.
    pub fn icmp_id(&self) -> u16 {
        self.get(ICMP_HUN + offset_of!(IhIdseq, icd_id))
    }

    /// Sets `icmp_id`.
    pub fn set_icmp_id(&self, v: u16) {
        self.set(ICMP_HUN + offset_of!(IhIdseq, icd_id), v);
    }

    /// `icmp_seq`, network order.
    pub fn icmp_seq(&self) -> u16 {
        self.get(ICMP_HUN + offset_of!(IhIdseq, icd_seq))
    }

    /// Sets `icmp_seq`.
    pub fn set_icmp_seq(&self, v: u16) {
        self.set(ICMP_HUN + offset_of!(IhIdseq, icd_seq), v);
    }

    /// Sets `icmp_void`.
    pub fn set_icmp_void(&self, v: i32) {
        self.set(ICMP_HUN, v);
    }

    /// `icmp_nextmtu`.
    pub fn icmp_nextmtu(&self) -> u16 {
        self.get(ICMP_HUN + offset_of!(IhPmtu, ipm_nextmtu))
    }

    /// Sets `icmp_nextmtu`.
    pub fn set_icmp_nextmtu(&self, v: u16) {
        self.set(ICMP_HUN + offset_of!(IhPmtu, ipm_nextmtu), v);
    }

    /// `icmp_ip`: the returned IP header of an error message.
    pub fn icmp_ip(&self) -> Ip {
        self.get(ICMP_DUN)
    }

    /// `&icmp_ip`.
    pub fn icmp_ip_ptr(&self) -> *mut Ip {
        self.at::<Ip>(ICMP_DUN)
    }

    /// `icmp_rtime`.
    pub fn icmp_rtime(&self) -> u32 {
        self.get(ICMP_DUN + offset_of!(IdTs, its_rtime))
    }

    /// Sets `icmp_rtime`.
    pub fn set_icmp_rtime(&self, v: u32) {
        self.set(ICMP_DUN + offset_of!(IdTs, its_rtime), v);
    }

    /// Sets `icmp_ttime`.
    pub fn set_icmp_ttime(&self, v: u32) {
        self.set(ICMP_DUN + offset_of!(IdTs, its_ttime), v);
    }

    /// Sets `icmp_mask`.
    pub fn set_icmp_mask(&self, v: u32) {
        self.set(ICMP_DUN, v);
    }
}

/// `icmp_init` for `bsd`'s protocol tests (the boot's runs from `domaininit`, through the
/// kernel's ICMP entry).
#[cfg(test)]
pub(crate) fn icmp_init() {
    kernel::netinet::ip_icmp::icmp_init::<crate::kern::proc_glue::BsdProcGlue>();
}

/// `icmp_error(n, type, code, dest, destmtu)` of a parked packet (pf, UDP, raw IP).
pub fn icmp_error(n: &'static Mbuf, type_: u8, code: u8, dest: u32, destmtu: i32) {
    kernel::netinet::ip_icmp::icmp_error(
        unpark(n),
        type_,
        code,
        dest,
        destmtu,
        net_held("icmp_error").net(),
    );
}

/// `icmp_mtudisc(icp, rtableid)`: the kernel's, of the message's next-hop MTU
/// (`icmp_nextmtu`, network order) and returned header `oip` as `bsd`'s TCP read them.
pub fn icmp_mtudisc(nextmtu: u16, oip: &Ip, rtableid: u32) {
    kernel::netinet::ip_icmp::icmp_mtudisc(nextmtu, oip, rtableid, net_held("icmp_mtudisc").net());
}

/// `icmp_mtudisc_clone(dst, rtableid, ipsec)`: the kernel's, the route parked.
pub fn icmp_mtudisc_clone(dst: InAddr, rtableid: u32, ipsec: bool) -> Option<&'static Rtentry> {
    kernel::netinet::ip_icmp::icmp_mtudisc_clone(
        dst,
        rtableid,
        ipsec,
        net_held("icmp_mtudisc_clone").net(),
    )
    .map(crate::net::route::park)
}
/* </CODE> */
