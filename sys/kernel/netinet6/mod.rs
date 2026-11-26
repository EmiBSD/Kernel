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
//! The IPv6 protocols (N6.5): OpenBSD's `sys/netinet6/`, as its modules move from `bsd`
//! (`docs/ROADMAP.md`'s N6 row). N6.4c1 brings the protocol control block's IPv6 lookups
//! (D-17); the userland halves of the headers are `sysdefs::netinet6_*`.

pub mod dest6;
pub mod frag6;
pub mod icmp6;
pub mod in6;
pub mod in6_cksum;
pub mod in6_ifattach;
pub mod in6_pcb;
pub mod in6_proto;
pub mod in6_src;
pub mod in6_var;
pub mod ip6_divert;
pub mod ip6_forward;
pub mod ip6_id;
pub mod ip6_input;
pub mod ip6_output;
pub mod ip6_var;
pub mod mld6;
pub mod mld6_var;
pub mod nd6;
pub mod nd6_nbr;
pub mod nd6_rtr;
pub mod raw_ip6;
pub mod route6;
pub mod udp6_output;
/* </CODE> */
