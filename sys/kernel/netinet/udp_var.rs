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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)udp_var.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! UDP kernel structures and variables: the kernel part of `<netinet/udp_var.h>`, the
//! statistics' counters.
//!
//! Upstream: sys/netinet/udp_var.h @ 3ce1f3f79392
//! LZ: sys/netinet/udp_var.rs@3c62ede68802
//!
//! ## Deviations
//! - `udpcounters` (`struct cpumem *`) is [`UDPCOUNTERS`], a `hal::sync::percpu::Counters` here
//!   beside the enum that indexes it (TCP's form, `tcp_var.rs`); `counters_alloc` at
//!   `udp_init` is its static initialiser.
//!
//! ## Redesign
//! - N6.6b0: LZ's `sys/netinet/udp_var.rs` is split by audience: the userland ABI
//!   (`UDPCTL_*`, `struct udpstat`, `struct udpiphdr`) is `hal::sys::netinet_udp_var`; the
//!   kernel part, `enum udpstat_counters` ([`UdpstatCounters`]), `udps_ncounters` and
//!   `udpstat_inc`, is here. LZ's static array of atomics in `udp_usrreq.rs` is per-CPU
//!   counters, summed by `udp_sysctl`. `udbtable`, the requests and the prototypes are
//!   `udp_usrreq`'s; `udp6_output` is `netinet6::udp6_output`'s.

use hal::sync::percpu::Counters;

/// `enum udpstat_counters`: the index of each `struct udpstat` member.
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UdpstatCounters {
    // input statistics:
    /// Total input packets.
    UdpsIpackets,
    /// Packet shorter than header.
    UdpsHdrops,
    /// Checksum error.
    UdpsBadsum,
    /// No checksum.
    UdpsNosum,
    /// Data length larger than packet.
    UdpsBadlen,
    /// No socket on port.
    UdpsNoport,
    /// Of above, arrived as broadcast.
    UdpsNoportbcast,
    /// Dropped for lack of ipsec.
    UdpsNosec,
    /// Not delivered, input socket full.
    UdpsFullsock,
    /// Input packets missing pcb hash.
    UdpsPcbhashmiss,
    /// Input software-csummed packets.
    UdpsInswcsum,
    // output statistics:
    /// Total output packets.
    UdpsOpackets,
    /// Output software-csummed packets.
    UdpsOutswcsum,
}

/// `udps_ncounters`: the number of counters.
pub const UDPS_NCOUNTERS: usize = UdpstatCounters::UdpsOutswcsum as usize + 1;

/// `udpcounters`: UDP's statistics, per CPU.
pub static UDPCOUNTERS: Counters<UDPS_NCOUNTERS> = Counters::new();

/// `udpstat_inc(c)`.
#[inline]
pub fn udpstat_inc(c: UdpstatCounters) {
    UDPCOUNTERS.inc(c as usize);
}

// The counters are the structure's words, in its order.
const _: () = assert!(
    core::mem::size_of::<hal::sys::netinet_udp_var::Udpstat>()
        == UDPS_NCOUNTERS * core::mem::size_of::<u64>()
);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counters_index_the_structure_in_the_c_order() {
        assert_eq!(UDPS_NCOUNTERS, 13);
        assert_eq!(UdpstatCounters::UdpsNosec as usize, 7);
        assert_eq!(UdpstatCounters::UdpsOpackets as usize, 11);
        let before = UDPCOUNTERS.read()[UdpstatCounters::UdpsOutswcsum as usize];
        udpstat_inc(UdpstatCounters::UdpsOutswcsum);
        assert!(UDPCOUNTERS.read()[UdpstatCounters::UdpsOutswcsum as usize] > before);
    }
}
/* </TESTS> */
