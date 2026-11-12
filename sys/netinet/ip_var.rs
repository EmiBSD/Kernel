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
 *	@(#)ip_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! IP implementation variables: statistics, the header overlay, `ip_output` flags, the
//! multicast options and the reassembly queues: `<netinet/ip_var.h>`.
//!
//! Upstream: sys/netinet/ip_var.h @ 3ce1f3f79392
//!
//! Since N6.4a the header's userland half is `hal::sys::netinet_ip_var` and its counters and
//! `ip_output` flags `kernel::netinet::ip_var` (with `ipcounters`, which `ip_input.c`
//! defines), re-exported here under LZ's names; what is left is `bsd`'s until `ip_output`
//! (N6.4a) and the reassembly (N6.4c2) move.
//!
//! ## Deviations
//! - `struct ip_moptions`'s `imo_membership` is a raw array of `Option<&InMulti>` (it grows
//!   with `imo_max_memberships`, as the C's does); `struct ipqent` and `struct ipq` hold
//!   `Cell`s (they change under `ipq_mutex`); `ipqe_ip` points at the fragment's header in its
//!   mbuf, read unaligned (`netinet/ip_input.rs`).
//! - The globals (`ip_defttl`, `ip_mtudisc`, `ipport_*`, `ip_forwarding`, `ipmultipath`,
//!   `ip_directedbcast`, `la_hold_total`, `ip_mtudisc_timeout_q`, `ipqent_pool`, ...) and the
//!   prototypes (`ip_output`, `ip_input_if`, ...) are defined by `netinet/ip_input.rs`,
//!   `ip_output.rs`, `ip_icmp.rs` and `if_ether.rs`; `rip_usrreqs` and the `rip_*`
//!   functions by `netinet/raw_ip.c`, which is not ported (`netinet/in_proto.rs` has the
//!   `rip_input` stand-in). `struct ipstat` as `sysctl(2)` returns it is here.

use core::cell::Cell;
use core::mem::size_of;

pub use hal::sys::netinet_ip_var::{Ipoption, Ipovly, Ipstat, MAX_IPOPTLEN};
pub use kernel::netinet::ip_var::{
    IP_ALLOWBROADCAST, IP_FORWARDING, IP_FORWARDING_IPSEC, IP_MTUDISC, IP_RAWOUTPUT, IP_REDIRECT,
    IPCOUNTERS, IPMTUDISCTIMEOUT, IPS_NCOUNTERS, IpstatCounters, ipstat_add, ipstat_dec,
    ipstat_inc,
};

use crate::netinet::in_::InAddr;
use crate::netinet::in_var::InMulti;
use crate::netinet::ip::Ip;
use crate::queue_adapter;
use crate::sys::mbuf::{Mbuf, mtod};
use crate::sys::queue::{ListEntry, ListHead};

/// `struct ip_moptions`: attached to `inpcb.ip_moptions` and passed to `ip_output` when IP
/// multicast options are in use.
pub struct IpMoptions {
    /// `imo_membership`: group memberships, `imo_max_memberships` slots.
    pub imo_membership: *mut Option<&'static InMulti>,
    /// `imo_ifidx`: ifp index for outgoing multicasts.
    pub imo_ifidx: u16,
    /// `imo_ttl`: TTL for outgoing multicasts.
    pub imo_ttl: u8,
    /// `imo_loop`: 1 => hear sends if a member.
    pub imo_loop: u8,
    /// `imo_num_memberships`: no. memberships this socket.
    pub imo_num_memberships: u16,
    /// `imo_max_memberships`: max memberships this socket.
    pub imo_max_memberships: u16,
}

/// `struct ipqent`: a fragment on a reassembly queue.
pub struct Ipqent {
    /// `ipqe_q`.
    pub ipqe_q: ListEntry<Ipqent>,
    /// `ipqe_ip`: the fragment's IP header, in `ipqe_m`.
    pub ipqe_ip: Cell<*mut Ip>,
    /// `ipqe_m`: mbuf contains packet.
    pub ipqe_m: Cell<Option<&'static Mbuf>>,
    /// `ipqe_mff`: for IP fragmentation.
    pub ipqe_mff: Cell<u16>,
}

queue_adapter!(
    /// `LIST_HEAD(ipqehead, ipqent)`: a datagram's fragments.
    pub Ipqehead: Ipqent, ipqe_q => ListEntry<Ipqent>
);

/// `struct ipq`: an IP reassembly queue. Each fragment being reassembled is attached to one
/// of these structures. They are timed out after `ipq_ttl` drops to 0, and may also be
/// reclaimed if memory becomes tight.
pub struct Ipq {
    /// `ipq_q`: to other reass headers.
    pub ipq_q: ListEntry<Ipq>,
    /// `ipq_rdomain`: routing domain for reassembly.
    pub ipq_rdomain: Cell<u32>,
    /// `ipq_id`: sequence id for reassembly.
    pub ipq_id: Cell<u16>,
    /// `ipq_ttl`: time for reass q to live.
    pub ipq_ttl: Cell<u8>,
    /// `ipq_p`: protocol of this fragment.
    pub ipq_p: Cell<u8>,
    /// `ipq_fragq`: to ip fragment queue.
    pub ipq_fragq: ListHead<Ipqehead>,
    /// `ipq_src`.
    pub ipq_src: Cell<InAddr>,
    /// `ipq_dst`.
    pub ipq_dst: Cell<InAddr>,
}

queue_adapter!(
    /// `LIST_HEAD(, ipq) ipq`: the reassembly queues.
    pub IpqList: Ipq, ipq_q => ListEntry<Ipq>
);

/// `struct ipoffnxt`: a fragment's offset and next header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ipoffnxt {
    /// Offset.
    pub ion_off: i32,
    /// Next header.
    pub ion_nxt: i32,
}

/// `*mtod(m, struct ip *)`: the IP header at the start of `m`'s data, as a value. The data
/// may sit at any alignment in the mbuf, so the header is copied out (`docs/C_TO_RUST.md`).
/// Panics if the first mbuf is shorter than a header.
pub fn mtod_ip(m: &Mbuf) -> Ip {
    match hal::pod::at::<Ip>(m.data(), 0) {
        Some(ip) => ip,
        None => {
            crate::kern::subr_prf::panic(format_args!("mtod_ip: mbuf shorter than an ip header"))
        }
    }
}

/// Writes `ip` back as the IP header at the start of `m`'s data (see [`mtod_ip`]).
pub fn mtod_ip_store(m: &Mbuf, ip: &Ip) {
    if (m.m_len().get() as usize) < size_of::<Ip>() {
        crate::kern::subr_prf::panic(format_args!(
            "mtod_ip_store: mbuf shorter than an ip header"
        ));
    }
    // SAFETY: as in `mtod_ip`.
    unsafe { core::ptr::write_unaligned(mtod::<Ip>(m), *ip) };
}

// LP64 size of the C structure.
const _: () = assert!(size_of::<Ipoffnxt>() == 8);
/* </CODE> */
