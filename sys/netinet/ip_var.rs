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
//! Since N6.4a the header's userland half is `sysdefs::netinet::ip_var` and its counters and
//! `ip_output` flags `kernel::netinet::ip_var` (with `ipcounters`, which `ip_input.c`
//! defines), re-exported here under LZ's names, with the reassembly records (`struct ipqent`,
//! `struct ipq`, `struct ipoffnxt`, N6.4c2); what is left is LZ's header copies `mtod_ip` and
//! `mtod_ip_store` for `bsd`'s transports.
//!
//! ## Deviations
//! - `struct ip_moptions` is the kernel's since N6.4c1 (`kernel::netinet::ip_var`),
//!   re-exported here.
//! - The globals (`ip_defttl`, `ip_mtudisc`, `ipport_*`, `ip_forwarding`, `ipmultipath`,
//!   `ip_directedbcast`, `la_hold_total`, `ip_mtudisc_timeout_q`, `ipqent_pool`, ...) and the
//!   prototypes (`ip_output`, `ip_input_if`, ...) are defined by `netinet/ip_input.rs`,
//!   `ip_output.rs`, `ip_icmp.rs` and `if_ether.rs`; `rip_usrreqs` and the `rip_*`
//!   functions by `netinet/raw_ip.c`, the kernel's since N6.6b1 (`kernel::netinet::raw_ip`).
//!   `struct ipstat` as `sysctl(2)` returns it is here.

use core::mem::size_of;

pub use kernel::netinet::ip_var::{
    IP_ALLOWBROADCAST, IP_FORWARDING, IP_FORWARDING_IPSEC, IP_MTUDISC, IP_RAWOUTPUT, IP_REDIRECT,
    IPCOUNTERS, IPMTUDISCTIMEOUT, IPS_NCOUNTERS, IpMoptions, Ipoffnxt, Ipq, Ipqent, IpstatCounters,
    ipstat_add, ipstat_dec, ipstat_inc,
};
pub use sysdefs::netinet::ip_var::{Ipoption, Ipovly, Ipstat, MAX_IPOPTLEN};

use crate::netinet::ip::Ip;
use crate::sys::mbuf::{Mbuf, mtod};

/// `*mtod(m, struct ip *)`: the IP header at the start of `m`'s data, as a value. The data
/// may sit at any alignment in the mbuf, so the header is copied out (`docs/C_TO_RUST.md`).
/// Panics if the first mbuf is shorter than a header.
pub fn mtod_ip(m: &Mbuf) -> Ip {
    match hal::mem::pod::at::<Ip>(m.data(), 0) {
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
