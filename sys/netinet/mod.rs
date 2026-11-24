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
/* </LICENSES> */

/* <CODE> */
//! The Internet protocols: OpenBSD `sys/netinet/`.
//!
//! Headers become modules as in `sys/sys` (`ip_icmp.h` → `ip_icmp.rs`). `in.h` is `in_.rs`
//! because `in` is a Rust keyword (`docs/C_TO_RUST.md`).
//!
//! The modules the kernel's `kernel::netinet` and `hal::sys::netinet_*` absorbed (N6.4a) keep
//! their LZ paths here as re-exports, for `bsd`'s importers (docs/SCAFFOLDING.md); TCP is the
//! kernel's since N6.6a5 (`tcp_seq` and `tcp_var` re-exported for pf, the drivers' TSO and
//! LRO counters and IPsec's output; `tcp_subr` keeps the TCP_SIGNATURE transform, IPsec's,
//! until N8); UDP since N6.6b0 (`udp` and `udp_var` re-export `hal::sys`'s images for pf,
//! IPsec, divert, pflow and the tests).

pub use hal::sys::netinet_ip as ip;
pub use hal::sys::netinet_tcp as tcp;
pub use hal::sys::netinet_udp as udp;
pub use hal::sys::netinet_udp_var as udp_var;
pub use kernel::netinet::{
    in_cksum, in_systm, in4_cksum, inet_nat64, ip_ecn, ip_id, tcp_fsm, tcp_seq, tcp_var,
};

pub mod icmp6;
#[forbid(unsafe_code)]
pub mod icmp_var;
pub mod if_ether;
pub mod igmp;
#[forbid(unsafe_code)]
pub mod igmp_var;
pub mod in_;
pub mod in_pcb;
#[forbid(unsafe_code)]
pub mod in_proto;
pub mod in_var;
#[forbid(unsafe_code)]
pub mod ip6;
pub mod ip_ah;
pub mod ip_divert;
pub mod ip_esp;
pub mod ip_icmp;
pub mod ip_input;
pub mod ip_ipcomp;
pub mod ip_ipip;
pub mod ip_ipsp;
pub mod ip_output;
pub mod ip_spd;
pub mod ip_var;
pub mod ipsec_input;
pub mod ipsec_output;
pub mod raw_ip;
pub mod tcp_subr;
/* </CODE> */
