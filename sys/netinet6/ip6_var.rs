/*	$KAME: ip6_var.h,v 1.33 2000/06/11 14:59:20 jinmei Exp $	*/
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
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
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
 *	@(#)ip_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 implementation variables: statistics, the reassembly queues, the multicast and
//! packet options, the `ip6_output` flags: `<netinet6/ip6_var.h>`.
//!
//! Upstream: sys/netinet6/ip6_var.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - `struct ip6stat`, `enum ip6stat_counters` (`hal::sys::netinet6_ip6_var`), the flags, the
//!   counters and their increments (`kernel::netinet6::ip6_var`) are re-exported (N6.5a).
//! - `struct ip6q` and `struct ip6asfrag` hold `Cell`s (they change under `frag6_mutex`);
//!   `struct ip6_moptions` and `struct ip6_pktopts` are the kernel's since N6.5c
//!   (`kernel::netinet6::ip6_var`), re-exported.
//! - `mtod_ip6` and `mtod_ip6_store` read and write the IPv6 header at the start of an
//!   mbuf as a copy, as `mtod_ip` does for `struct ip` (`netinet/ip_var.rs`), through the
//!   kernel mbuf's byte views (N6.5c review N3: no `unsafe`); a header in a shared cluster
//!   is written copy-on-write, where the C writes the shared cluster.
//! - The globals (`ip6_defhlim`, `ip6_forwarding`, `ip6_mtudisc_timeout`, ...) are the
//!   kernel's (`kernel::netinet6::ip6_var`), `icmp6_mtudisc_timeout_q` is
//!   `netinet6/icmp6.rs`'s, `rip6_usrreqs` `netinet6/raw_ip6.rs`'s; the prototypes by their `.c` files (`ip6_output` in
//!   `netinet6/ip6_output.rs`, `ip6_input_if` in `netinet6/ip6_input.rs`, ...). `ip6_mrouter[]`, `ip6_mrouter_active` and `ip6_mforward`
//!   (`netinet6/ip6_mroute.c`) are not compiled: `MROUTING` is not configured.

use core::cell::Cell;
use core::mem::size_of;

use crate::kern::subr_prf::panic;
use crate::netinet::ip6::Ip6Hdr;
use crate::netinet6::in6::In6Addr;
use crate::queue_adapter;
use crate::sys::mbuf::{M_DONTWAIT, Mbuf, mut_of};
use crate::sys::queue::{ListEntry, ListHead, TailqEntry};
use kernel::netinet::in_pcb::Hdr6;

pub use hal::sys::netinet6_ip6_var::{IP6S_NCOUNTERS, Ip6stat, Ip6statCounters};
pub use kernel::netinet6::ip6_var::{
    IP6PO_DONTFRAG, IP6PO_MINMTU_ALL, IP6PO_MINMTU_DISABLE, IP6PO_MINMTU_MCASTONLY,
    IPV6_FORWARDING, IPV6_FORWARDING_IPSEC, IPV6_MINMTU, IPV6_REDIRECT, IPV6_UNSPECSRC,
    Ip6Moptions, Ip6OutOpts, Ip6Pktopts, ip6stat_add, ip6stat_dec, ip6stat_inc, ip6stat_inc_idx,
};

/// `struct ip6q`: IP6 reassembly queue structure. Each fragment being reassembled is
/// attached to one of these structures.
pub struct Ip6q {
    /// `ip6q_queue`.
    pub ip6q_queue: TailqEntry<Ip6q>,
    /// `ip6q_asfrag`: the fragments.
    pub ip6q_asfrag: ListHead<Ip6asfragList>,
    /// `ip6q_src`.
    pub ip6q_src: Cell<In6Addr>,
    /// `ip6q_dst`.
    pub ip6q_dst: Cell<In6Addr>,
    /// Len of unfragmentable part.
    pub ip6q_unfrglen: Cell<i32>,
    /// # of fragments.
    pub ip6q_nfrag: Cell<i32>,
    /// Routing domain for reassembly.
    pub ip6q_rdomain: Cell<u32>,
    /// Fragment identification.
    pub ip6q_ident: Cell<u32>,
    /// `ip6f_nxt` in first fragment.
    pub ip6q_nxt: Cell<u8>,
    /// `ip6q_ecn`.
    pub ip6q_ecn: Cell<u8>,
    /// Time to live in slowtimo units.
    pub ip6q_ttl: Cell<u8>,
}

queue_adapter!(
    /// `TAILQ_HEAD(ip6q_head, ip6q)` through `ip6q_queue`: `frag6_queue`.
    pub Ip6qList: Ip6q, ip6q_queue => TailqEntry<Ip6q>
);

/// `struct ip6asfrag`: a fragment on a reassembly queue.
pub struct Ip6asfrag {
    /// `ip6af_list`.
    pub ip6af_list: ListEntry<Ip6asfrag>,
    /// `ip6af_m`.
    pub ip6af_m: Cell<Option<&'static Mbuf>>,
    /// Offset in `ip6af_m` to next header.
    pub ip6af_offset: Cell<i32>,
    /// Fragmentable part length.
    pub ip6af_frglen: Cell<i32>,
    /// Fragment offset.
    pub ip6af_off: Cell<i32>,
    /// More fragment bit in frag off.
    pub ip6af_mff: Cell<u16>,
}

queue_adapter!(
    /// `LIST_HEAD(ip6asfrag_list, ip6asfrag)` through `ip6af_list`.
    pub Ip6asfragList: Ip6asfrag, ip6af_list => ListEntry<Ip6asfrag>
);

/// `*mtod(m, struct ip6_hdr *)`: the IPv6 header at the start of `m`'s data, as a value.
/// The data may sit at any alignment in the mbuf, so the header is copied out of the first
/// mbuf's bytes ([`Mbuf::data`]). Panics if the first mbuf is shorter than a header.
pub fn mtod_ip6(m: &Mbuf) -> Ip6Hdr {
    let mut b = [0u8; size_of::<Ip6Hdr>()];
    match m.data().get(..b.len()) {
        Some(d) => b.copy_from_slice(d),
        None => panic(format_args!("mtod_ip6: mbuf shorter than an ip6 header")),
    }
    Ip6Hdr::from_bytes(b)
}

/// Writes `ip6` back as the IPv6 header at the start of `m`'s data (see [`mtod_ip6`]), in
/// place when the first mbuf's storage is writable ([`Mbuf::data_mut`] through `mut_of`, the
/// adapters' one-owner convention: the caller owns the packet for the call). A header in
/// read-only storage (a shared cluster) is written through the kernel's copy-on-write
/// `m_copyback`, as `ip6_store` does, where the C writes the shared cluster in place; no
/// caller hands one over (pf's packets have their headers in the driver's writable cluster
/// or the stack's own mbufs, `ip6_mloopback` pulls its copy's header up), and the C cannot
/// fail here, so a copy that cannot be had is `panic(9)`. Panics if the first mbuf is
/// shorter than a header.
pub fn mtod_ip6_store(m: &Mbuf, ip6: &Ip6Hdr) {
    if (m.m_len().get() as usize) < size_of::<Ip6Hdr>() {
        panic(format_args!(
            "mtod_ip6_store: mbuf shorter than an ip6 header"
        ));
    }
    let b = ip6.to_bytes();
    let m = mut_of(m);
    if let Some(d) = m.data_mut() {
        d[..b.len()].copy_from_slice(&b);
        return;
    }
    if kernel::net::mbuf::m_copyback(m, 0, &b, M_DONTWAIT).is_err() {
        panic(format_args!(
            "mtod_ip6_store: no cluster for the copy of a read-only header"
        ));
    }
}

/* </CODE> */
