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
//! The Internet protocols (N6.4): OpenBSD's `sys/netinet/`, as its modules move from `bsd`
//! (`docs/ROADMAP.md`'s N6 row). Headers become modules as in `bsd` (`ip_var.h` beside
//! `ip_input.c`'s counters in `ip_var.rs`); the userland halves of the headers are
//! `hal::sys::netinet_*`.

pub mod in4_cksum;
pub mod in_cksum;
pub mod in_systm;
pub mod inet_nat64;
pub mod ip_ecn;
pub mod ip_id;
pub mod ip_output;
pub mod ip_var;
/* </CODE> */
