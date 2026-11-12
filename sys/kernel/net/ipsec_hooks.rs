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
//! The `Ipsec` bridge: what the kernel's network code asks of IPsec, which has not moved yet
//! (N8, the last of the security-sensitive areas): today one leg, `enc_getifa`, which
//! rtsock.c's `rtm_getifa` calls under `#ifdef IPSEC` to give a route to a `PF_KEY`
//! destination the address of the encapsulation interface enc(4).
//!
//! `bsd`'s `if_enc.rs` registers its implementation at boot ([`register_ipsec`],
//! bridges.toml's `Ipsec` row, paid by N8); before that the leg answers `None`, the C
//! compiled without `IPSEC`. N6.4 may move this module beside the internet protocols
//! (`kernel::netinet`) and grow it with their IPsec hooks.

use hal::bridges::AlreadyRegistered;
use hal::once::OnceStatic;

use crate::net::if_var::Ifaddr;

/// The legs of IPsec the kernel calls (`net/if_enc.c`'s, `netinet/ipsec_*.c`'s).
pub trait Ipsec: Sync {
    /// `enc_getifa(rdomain, unit)`: the address of the enc(4) interface of `unit` in the
    /// routing domain of table `rtableid` (`enc0` for unit 0), `None` when there is none.
    fn enc_getifa(&self, rtableid: u32, unit: u32) -> Option<&'static Ifaddr>;
}

/// IPsec's legs, once registered.
static IPSEC: OnceStatic<&'static dyn Ipsec> = OnceStatic::new();

/// Registers IPsec's legs, once (`bsd`'s enc(4), at boot before the first routing request);
/// a second call keeps the first and names it.
pub fn register_ipsec(imp: &'static dyn Ipsec) -> Result<(), AlreadyRegistered> {
    IPSEC.set(imp).map_err(|_| AlreadyRegistered("Ipsec"))
}

/// IPsec's legs, `None` before registration (the C without `IPSEC`).
#[inline]
pub(crate) fn ipsec() -> Option<&'static dyn Ipsec> {
    IPSEC.get().copied()
}
/* </CODE> */
