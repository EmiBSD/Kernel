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
//! `struct ipovly`, `struct ipoption`, `MAX_IPOPTLEN`) is `hal::sys::netinet_ip_var`.
//!
//! Upstream: sys/netinet/ip_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/ip_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `struct ip_moptions` is here without its group memberships (`imo_membership`,
//!   `imo_num_memberships`, `imo_max_memberships`): [`IpMoptions`] is what `ip_output` reads
//!   (the interface, the TTL, the loopback switch); `bsd`'s `sys/netinet/ip_var.rs` keeps the
//!   whole C struct for its socket options and lends `ip_output` this part until the pcb and
//!   `ip_setmoptions` move (N6.4c1), which adds the memberships here.
//! - `struct ipqent`, `struct ipq` and `struct ipoffnxt` are still `bsd`'s
//!   `sys/netinet/ip_var.rs` (LZ's), until the reassembly moves (N6.4c2); so are LZ's
//!   `mtod_ip`/`mtod_ip_store` (`bsd`'s header copies).
//!
//! ## Redesign
//! - LZ's `IPCOUNTERS`, a static array of atomics in `bsd`'s `ip_input.rs` (`<sys/percpu.h>`
//!   was not ported) -> [`IPCOUNTERS`], a `hal::percpu::Counters` here beside the enum that
//!   indexes it (`rtcounters`' shape, `kernel::net::route`): per CPU, as the C's `cpumem`, read
//!   whole by `net.inet.ip.stats` (`Counters::read`). `ipstat_inc`, `ipstat_add` and
//!   `ipstat_dec` are `#[inline]`: called per packet from `bsd`'s protocols across the crate
//!   boundary (N6.3c1's R12). `bsd` reaches all of it under LZ's path through its own
//!   `sys/netinet/ip_var.rs`, which re-exports it.
//! - N6.4a: `ip_mtudisc` and `ipmultipath` (`ip_input.c`'s, `extern` here as in the C's
//!   header), atomics in LZ's `bsd` `ip_input.rs`, are [`ip_mtudisc`] and [`IPMULTIPATH`] here:
//!   the kernel's `ip_output` and routes read them (the `Inet` leg `ip_multipath` goes), `bsd`'s
//!   `ip_input.rs` re-exports them for its sysctl and its readers until N6.4c2.

use core::sync::atomic::AtomicI32;

use hal::percpu::Counters;
use hal::sys::socket::SO_BROADCAST;

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

/// `struct ip_moptions`: the multicast options of a socket (`inp_moptions`), as `ip_output`
/// reads them (the module's deviations: the memberships are `bsd`'s until N6.4c1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpMoptions {
    /// `imo_ifidx`: ifp index for outgoing multicasts.
    pub imo_ifidx: u16,
    /// `imo_ttl`: TTL for outgoing multicasts.
    pub imo_ttl: u8,
    /// `imo_loop`: 1 => hear sends if a member.
    pub imo_loop: u8,
}

/// `ipcounters`: the IP statistics, per CPU (`counters_alloc(ips_ncounters)` in `ip_init`).
pub static IPCOUNTERS: Counters<IPS_NCOUNTERS> = Counters::new();

/// \[a\] `ip_mtudisc`: mtu discovery (`net.inet.ip.mtudisc`, `ip_input.c`'s).
#[allow(non_upper_case_globals)] // the C name; `IP_MTUDISC` is the output flag above
pub static ip_mtudisc: AtomicI32 = AtomicI32::new(1);

/// \[a\] `ipmultipath`: enable multipath routing (`net.inet.ip.multipath`, `ip_input.c`'s).
pub static IPMULTIPATH: AtomicI32 = AtomicI32::new(0);

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
    assert!(size_of::<hal::sys::netinet_ip_var::Ipstat>() == IPS_NCOUNTERS * size_of::<u64>());
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
