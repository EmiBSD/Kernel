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
//! Group `netinet` of `sysdefs`: the Internet protocols (`<netinet/*.h>`).

#[forbid(unsafe_code)]
pub mod icmp6;
#[forbid(unsafe_code)]
pub mod icmp_var;
#[forbid(unsafe_code)]
pub mod if_ether;
#[forbid(unsafe_code)]
pub mod igmp;
#[forbid(unsafe_code)]
pub mod igmp_var;
#[forbid(unsafe_code)]
pub mod in_;
#[forbid(unsafe_code)]
pub mod in_var;
#[forbid(unsafe_code)]
pub mod ip;
#[forbid(unsafe_code)]
pub mod ip6;
#[forbid(unsafe_code)]
pub mod ip_divert;
#[forbid(unsafe_code)]
pub mod ip_icmp;
#[forbid(unsafe_code)]
pub mod ip_mroute;
#[forbid(unsafe_code)]
pub mod ip_var;
#[forbid(unsafe_code)]
pub mod tcp;
#[forbid(unsafe_code)]
pub mod tcp_fsm;
#[forbid(unsafe_code)]
pub mod tcp_var;
#[forbid(unsafe_code)]
pub mod udp;
#[forbid(unsafe_code)]
pub mod udp_var;
/* </CODE> */
