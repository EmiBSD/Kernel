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
//! IP implementation variables, the kernel's half of `<netinet/ip_var.h>`: the `ip_output`
//! flags, `IPMTUDISCTIMEOUT`, the per-CPU IP statistics (`ipcounters`, defined by
//! `ip_input.c`) and their `ipstat_inc`/`ipstat_add`. The userland half (`struct ipstat`,
//! `struct ipovly`, `struct ipoption`, `MAX_IPOPTLEN`) is `sysdefs::netinet::ip_var`.
//!
//! Upstream: sys/netinet/ip_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/ip_var.rs@3c62ede68802
//!
//! ## Deviations
//! - LZ's `mtod_ip`/`mtod_ip_store` (`bsd`'s header copies) stay `bsd`'s, for `bsd`'s
//!   transports; the kernel reads and writes a header with `ip_output`'s `ip_load`/`ip_store`.
//!
//! ## Redesign
//! - LZ's `IPCOUNTERS`, a static array of atomics in `bsd`'s `ip_input.rs` (`<sys/percpu.h>`
//!   was not ported) -> [`IPCOUNTERS`], a `hal::sync::percpu::Counters` here beside the enum that
//!   indexes it (`rtcounters`' shape, `kernel::net::route`): per CPU, as the C's `cpumem`, read
//!   whole by `net.inet.ip.stats` (`Counters::read`). `ipstat_inc`, `ipstat_add` and
//!   `ipstat_dec` are `#[inline]`: called per packet from `bsd`'s protocols across the crate
//!   boundary (N6.3c1's R12). `bsd` reaches all of it under LZ's path through its own
//!   `sys/netinet/ip_var.rs`, which re-exports it.
//! - N6.4a: `ip_mtudisc` and `ipmultipath` (`ip_input.c`'s, `extern` here as in the C's
//!   header), atomics in LZ's `bsd` `ip_input.rs`, are [`ip_mtudisc`] and [`IPMULTIPATH`] here:
//!   the kernel's `ip_output` and routes read them (the `Inet` leg `ip_multipath` goes), `bsd`'s
//!   `ip_input.rs` re-exports them for its sysctl and its readers until N6.4c2.
//! - N6.4c1: `struct ip_moptions` ([`IpMoptions`]) is whole here, its memberships with it:
//!   LZ's raw `imo_membership` vector (a `*mut Option<&InMulti>` read and written through
//!   `unsafe` slot pointers, grown by `mallocarray` and `free`) -> an `MArray` of `M_IPMOPTS`
//!   (`imo_max_memberships` is its length), owned by the set, which a socket's control block
//!   owns as an `MBox` of `M_IPMOPTS` (`kernel::netinet::in_pcb::Inpcb::moptions`): the C's
//!   two allocations, of the C's type. A set made for one `ip_output` call (IGMP's and
//!   pfsync(4)'s, on the stack in the C, which leaves the vector unset) has none
//!   ([`IpMoptions::output`]). `ip_defttl` ([`IP_DEFTTL`]) moved here from LZ's `bsd`
//!   `ip_input.rs` as `ip_mtudisc` did: `ip_ctloutput` reads it (`IP_TTL` -1,
//!   `IP_IPDEFTTL`); `bsd`'s `ip_input.rs` re-exports it. The set's `M_IPMOPTS` block is 48
//!   bytes (a 64-byte bucket) where the C's and LZ's are 16 (the `MArray` handle carries its
//!   length, malloc type and allocator; the D-3 class, docs/SCAFFOLDING.md's pool-size row);
//!   the vector's is the C's, `imo_max_memberships` pointers.
//! - N6.5e's 5e2: each slot holds the membership's counted handle of its record
//!   (`hal::mem::malloc::MArc`, still 8 bytes with `None` its niche), which `in_addmulti` gives
//!   and `in_delmulti` takes back, where it held a `&'static` record; growing the vector moves
//!   the handles into the new one.
//! - N6.4c2, the reassembly records: LZ's `struct ipqent` and `struct ipq` were pool items
//!   written whole through `unsafe` and linked on `bsd`'s `LIST`s (`IpqList`, `Ipqehead`) by
//!   `&'static` references, the fragment's header reached through the raw `ipqe_ip`
//!   pointer and read and written with unaligned copies. Here [`Ipqent`] and [`Ipq`] are
//!   items of `ip_input.c`'s pools ("ipqe", "ipq") owned by `hal::ds::owned::OwnedSlist`s (the
//!   C's `LIST`s, walked forward only, D-15): a datagram's fragments in its
//!   [`Ipq::ipq_fragq`], the datagrams in `ip_input`'s queue. `ipqe_ip` is gone: the three
//!   header members `ip_reass` reads and changes (the offset, the data length, the type of
//!   service) are cells of the record ([`Ipqent::ipqe_off`], `ipqe_len`, `ipqe_tos`), set
//!   from the header when the fragment is queued, and the header is written back once,
//!   into the first fragment, when the datagram is complete (the only header the C's
//!   writes through `ipqe_ip` end up in); the mbuf is owned (`Option<MbufBox>`, `Some`
//!   while queued). `struct ipoffnxt` ([`Ipoffnxt`]) is the `PACKET_TAG_IP_OFFNXT` tag's
//!   image.

use core::cell::Cell;
use core::sync::atomic::AtomicI32;

use hal::ds::owned::{OwnedSlist, SlistLink};
use hal::mem::malloc::{MArc, MArray};
use hal::mem::pool::{BoxPool, Pooled};
use hal::sync::percpu::Counters;
use sysdefs::net::socket::SO_BROADCAST;
use sysdefs::netinet::in_::InAddr;
use sysdefs::netinet::ip::IPDEFTTL;

use crate::net::mbuf::MbufBox;
use crate::netinet::in_var::InMulti;

// Flags passed to ip_output

/// Most of ip header exists.
pub const IP_FORWARDING: i32 = 0x0001;
/// Raw ip header exists.
pub const IP_RAWOUTPUT: i32 = 0x0002;
/// Redirected by pf or source route.
pub const IP_REDIRECT: i32 = 0x0004;
/// Only packets processed by IPsec.
pub const IP_FORWARDING_IPSEC: i32 = 0x0008;
/// Can send broadcast packets.
pub const IP_ALLOWBROADCAST: i32 = SO_BROADCAST;
/// Pmtu discovery, set DF.
pub const IP_MTUDISC: i32 = 0x0800;

/// As per RFC 1191: seconds before a learned path MTU expires.
pub const IPMTUDISCTIMEOUT: i32 = 10 * 60;

/// `enum ipstat_counters`: the per-CPU IP counters, one per field of [`Ipstat`] in the same
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum IpstatCounters {
    /// Total packets received.
    IpsTotal,
    /// Checksum bad.
    IpsBadsum,
    /// Packet too short.
    IpsTooshort,
    /// Not enough data.
    IpsToosmall,
    /// IP header length < data size.
    IpsBadhlen,
    /// IP length < ip header length.
    IpsBadlen,
    /// Fragments received.
    IpsFragments,
    /// Frags dropped (dups, out of space).
    IpsFragdropped,
    /// Fragments timed out.
    IpsFragtimeout,
    /// Packets forwarded.
    IpsForward,
    /// Packets rcvd for unreachable dest.
    IpsCantforward,
    /// Packets forwarded on same net.
    IpsRedirectsent,
    /// Unknown or unsupported protocol.
    IpsNoproto,
    /// Datagrams delivered to upper level.
    IpsDelivered,
    /// Total ip packets generated here.
    IpsLocalout,
    /// Lost output packets due to nobufs, etc.
    IpsOdropped,
    /// Total packets reassembled ok.
    IpsReassembled,
    /// Datagrams successfully fragmented.
    IpsFragmented,
    /// Output fragments created.
    IpsOfragments,
    /// Don't fragment flag was set, etc.
    IpsCantfrag,
    /// Error in option processing.
    IpsBadoptions,
    /// Packets discarded due to no route.
    IpsNoroute,
    /// IP version != 4.
    IpsBadvers,
    /// Total raw ip packets generated.
    IpsRawout,
    /// Malformed fragments (bad length).
    IpsBadfrags,
    /// Frags dropped for lack of memory.
    IpsRcvmemdrop,
    /// IP length > max ip packet size.
    IpsToolong,
    /// No match gif found.
    IpsNogif,
    /// Invalid address on header.
    IpsBadaddr,
    /// Software checksummed on input.
    IpsInswcsum,
    /// Software checksummed on output.
    IpsOutswcsum,
    /// Multicasts for unregistered groups.
    IpsNotmember,
    /// Valid route to destination found in cache.
    IpsRtcachehit,
    /// Route cache filled with new destination.
    IpsRtcachemiss,
    /// Packet received on wrong interface.
    IpsWrongif,
    /// Lost input packets due to nobufs, etc.
    IpsIdropped,
    /// The number of counters.
    IpsNcounters,
}

/// `ips_ncounters`: the number of IP counters.
pub const IPS_NCOUNTERS: usize = IpstatCounters::IpsNcounters as usize;

/// `struct ip_moptions`: attached to `inp_moptions` and passed to `ip_output` when IP
/// multicast options are in use.
#[derive(Default)]
pub struct IpMoptions {
    /// `imo_membership`: group memberships, `imo_max_memberships` slots (`M_IPMOPTS`), the
    /// first `imo_num_memberships` in use, each the membership's handle of its record
    /// (`in_addmulti`'s, which `in_delmulti` takes back); `None` in a set made for one
    /// `ip_output` call.
    pub imo_membership: Option<MArray<Option<MArc<InMulti>>>>,
    /// `imo_ifidx`: ifp index for outgoing multicasts.
    pub imo_ifidx: u16,
    /// `imo_ttl`: TTL for outgoing multicasts.
    pub imo_ttl: u8,
    /// `imo_loop`: 1 => hear sends if a member.
    pub imo_loop: u8,
    /// `imo_num_memberships`: no. memberships this socket.
    pub imo_num_memberships: u16,
}

impl IpMoptions {
    /// The options `ip_output` reads, for one call: the interface for outgoing multicasts,
    /// their TTL and loopback switch, and no membership (the C's `struct ip_moptions` on the
    /// stack of `igmp_sendpkt` and `pfsync_sendout`; a socket's, copied by
    /// `Inpcb::moptions_out`).
    #[inline]
    pub const fn output(ifidx: u16, ttl: u8, loop_: u8) -> Self {
        Self {
            imo_membership: None,
            imo_ifidx: ifidx,
            imo_ttl: ttl,
            imo_loop: loop_,
            imo_num_memberships: 0,
        }
    }

    /// `imo_max_memberships`: max memberships this socket (the vector's slots).
    #[inline]
    pub fn imo_max_memberships(&self) -> u16 {
        self.imo_membership.as_ref().map_or(0, |v| v.len() as u16)
    }

    /// `imo_membership[i]` of a slot in use (`i < imo_num_memberships`); `None` beyond.
    #[inline]
    pub fn membership(&self, i: usize) -> Option<&InMulti> {
        if i >= usize::from(self.imo_num_memberships) {
            return None;
        }
        self.imo_membership.as_ref()?.get(i)?.as_deref()
    }
}

/// `struct ipqent`: a fragment queued for reassembly, an item of the "ipqe" pool.
pub struct Ipqent {
    /// `ipqe_q`: the next fragment of its datagram.
    ipqe_q: SlistLink<Ipqent>,
    /// `ipqe_m`: the fragment, its data past its header; `Some` while it is queued.
    pub ipqe_m: Option<MbufBox>,
    /// `ipqe_mff`: more fragments follow (the header's `IP_MF`).
    pub ipqe_mff: bool,
    /// `ipqe_ip->ip_hl << 2`: the fragment's header length, in bytes.
    pub ipqe_hlen: u8,
    /// `ntohs(ipqe_ip->ip_off)`: the fragment's offset, in bytes (`ip_fragcheck` shifted it).
    pub ipqe_off: Cell<u16>,
    /// `ntohs(ipqe_ip->ip_len)`: the fragment's data length (`ip_fragcheck` took the header
    /// out).
    pub ipqe_len: Cell<u16>,
    /// `ipqe_ip->ip_tos`: the type of service, ECN's bits among them.
    pub ipqe_tos: Cell<u8>,
}

impl Ipqent {
    /// A fragment of `hlen` header bytes at offset `off` with `len` data bytes.
    pub const fn new(m: MbufBox, mff: bool, hlen: u8, off: u16, len: u16, tos: u8) -> Self {
        Self {
            ipqe_q: SlistLink::new(),
            ipqe_m: Some(m),
            ipqe_mff: mff,
            ipqe_hlen: hlen,
            ipqe_off: Cell::new(off),
            ipqe_len: Cell::new(len),
            ipqe_tos: Cell::new(tos),
        }
    }

    /// `ntohs(ipqe_ip->ip_off) + ntohs(ipqe_ip->ip_len)`: where the fragment's data ends.
    #[inline]
    pub fn end(&self) -> i32 {
        i32::from(self.ipqe_off.get()) + i32::from(self.ipqe_len.get())
    }
}

impl Pooled for Ipqent {
    const POOL: &'static BoxPool<Self> = &crate::netinet::ip_input::IPQENT_POOL;
}

hal::slist_adapter!(
    /// `ipqe_q`: the fragments of a datagram.
    pub IpqentList: Ipqent, ipqe_q
);

/// `struct ipq`: a datagram being reassembled, an item of the "ipq" pool.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct ipq's ipq_* names so the C can be grepped"
)]
pub struct Ipq {
    /// `ipq_q`: the next datagram of the reassembly queue.
    ipq_q: SlistLink<Ipq>,
    /// `ipq_rdomain`.
    pub ipq_rdomain: u32,
    /// `ipq_ttl`: time for reass q to live (`ip_slowtimo` counts it down).
    pub ipq_ttl: Cell<u8>,
    /// `ipq_p`: protocol of this fragment.
    pub ipq_p: u8,
    /// `ipq_id`: sequence id for reassembly.
    pub ipq_id: u16,
    /// `ipq_fragq`: the fragments, by offset.
    pub ipq_fragq: OwnedSlist<IpqentList>,
    /// `ipq_src`.
    pub ipq_src: InAddr,
    /// `ipq_dst`.
    pub ipq_dst: InAddr,
}

impl Ipq {
    /// The queue of datagram `id` of protocol `p` from `src` to `dst` in `rdomain`, empty, with
    /// `ttl` to live.
    pub const fn new(rdomain: u32, id: u16, p: u8, src: InAddr, dst: InAddr, ttl: u8) -> Self {
        Self {
            ipq_q: SlistLink::new(),
            ipq_rdomain: rdomain,
            ipq_ttl: Cell::new(ttl),
            ipq_p: p,
            ipq_id: id,
            ipq_fragq: OwnedSlist::new(),
            ipq_src: src,
            ipq_dst: dst,
        }
    }
}

impl Pooled for Ipq {
    const POOL: &'static BoxPool<Self> = &crate::netinet::ip_input::IPQ_POOL;
}

hal::slist_adapter!(
    /// `ipq_q`: the reassembly queue.
    pub IpqList: Ipq, ipq_q
);

/// `struct ipoffnxt`: the offset and the next protocol of a packet `ip_ours_enqueue` queues
/// with options, `PACKET_TAG_IP_OFFNXT`'s payload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ipoffnxt {
    /// `ion_off`.
    pub ion_off: i32,
    /// `ion_nxt`.
    pub ion_nxt: i32,
}

sysdefs::abi_pod!(Ipoffnxt {
    ion_off: i32,
    ion_nxt: i32,
});

/// `ipcounters`: the IP statistics, per CPU (`counters_alloc(ips_ncounters)` in `ip_init`).
pub static IPCOUNTERS: Counters<IPS_NCOUNTERS> = Counters::new();

/// \[a\] `ip_mtudisc`: mtu discovery (`net.inet.ip.mtudisc`, `ip_input.c`'s).
#[allow(non_upper_case_globals)] // the C name; `IP_MTUDISC` is the output flag above
pub static ip_mtudisc: AtomicI32 = AtomicI32::new(1);

/// \[a\] `ipmultipath`: enable multipath routing (`net.inet.ip.multipath`, `ip_input.c`'s).
pub static IPMULTIPATH: AtomicI32 = AtomicI32::new(0);

/// \[a\] `ip_defttl`: default IP ttl (`net.inet.ip.ttl`, `ip_input.c`'s).
pub static IP_DEFTTL: AtomicI32 = AtomicI32::new(IPDEFTTL as i32);

/// `ipstat_inc(c)`.
#[inline]
pub fn ipstat_inc(c: IpstatCounters) {
    IPCOUNTERS.inc(c as usize);
}

/// `ipstat_add(c, v)`.
#[inline]
pub fn ipstat_add(c: IpstatCounters, v: u64) {
    IPCOUNTERS.add(c as usize, v);
}

/// `counters_dec(ipcounters, c)` (`ip_deliver` takes back a delivery it queues).
#[inline]
pub fn ipstat_dec(c: IpstatCounters) {
    IPCOUNTERS.dec(c as usize);
}

// The counters are `struct ipstat`'s words (`CTASSERT` in `ip_sysctl_ipstat`).
const _: () =
    assert!(size_of::<sysdefs::netinet::ip_var::Ipstat>() == IPS_NCOUNTERS * size_of::<u64>());
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counters_count_per_index() {
        let before = IPCOUNTERS.read();
        ipstat_inc(IpstatCounters::IpsNogif);
        ipstat_add(IpstatCounters::IpsNogif, 4);
        ipstat_dec(IpstatCounters::IpsNogif);
        let after = IPCOUNTERS.read();
        let i = IpstatCounters::IpsNogif as usize;
        assert_eq!(after[i] - before[i], 4);
        assert_eq!(IPS_NCOUNTERS, 36);
    }

    #[test]
    fn a_set_of_multicast_options_is_48_bytes_and_its_vector_the_cs() {
        // The C's and LZ's struct ip_moptions is 16 bytes (a pointer, then 8 bytes of
        // scalars); here the vector's handle is 40 (the module's Redesign).
        assert_eq!(core::mem::size_of::<IpMoptions>(), 48);
        assert_eq!(core::mem::size_of::<Option<MArc<InMulti>>>(), 8);
        let set = IpMoptions::output(3, 9, 0);
        assert_eq!(set.imo_max_memberships(), 0);
        assert!(set.membership(0).is_none());
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/ip_var.h");
        let ip = crate::reftest::assert_defines!(defs;
            IP_FORWARDING, IP_RAWOUTPUT, IP_REDIRECT, IP_FORWARDING_IPSEC, IP_MTUDISC,
            IPMTUDISCTIMEOUT);
        assert_eq!(defs["IP_ALLOWBROADCAST"], "SO_BROADCAST");
        crate::reftest::assert_complete(&defs, "IP_", &[&ip[..], &["IP_ALLOWBROADCAST"]].concat());
    }
}
/* </TESTS> */
