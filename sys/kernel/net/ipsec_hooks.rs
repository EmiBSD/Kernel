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
//! (N8, the last of the security-sensitive areas): `enc_getifa`, which rtsock.c's
//! `rtm_getifa` calls under `#ifdef IPSEC` to give a route to a `PF_KEY` destination the
//! address of the encapsulation interface enc(4); and since N6.4a the `#ifdef IPSEC` points
//! of `ip_output` (`kernel::netinet::ip_output`): `ipsec_in_use`, the two halves of
//! ip_output.c's own IPsec code (`ip_output_ipsec_lookup`, and `ip_output_ipsec_send` with
//! its `ip_output_ipsec_pmtu_update`), `tdb_unref` and `ipsec_adjust_mtu`.
//!
//! `bsd`'s `if_enc.rs` registers its implementation at boot ([`register_ipsec`],
//! bridges.toml's `Ipsec` row, paid by N8); before that the legs answer as the C compiled
//! without `IPSEC`: no address, no SA to apply.
//!
//! An SA (`struct tdb`) crosses as a [`TdbRef`], `bsd`'s `&'static Tdb` seen as an opaque
//! `dyn Any`: the kernel never reads it, only holds the reference `ip_output_ipsec_lookup`
//! took ([`TdbHold`], whose drop is the C's `tdb_unref` at `ip_output`'s `done:`).

use core::any::Any;

use hal::bridges::AlreadyRegistered;
use hal::once::OnceStatic;
use hal::sys::errno::Errno;

use crate::net::if_var::Ifaddr;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::route::Route;

/// `struct ipsec_level`: the security levels of a socket (`inp_seclevel`, `ip_ipsp.h`), the
/// argument `ip_output` passes to the SPD lookup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpsecLevel {
    /// Authentication level.
    pub sl_auth: u8,
    /// ESP transport level.
    pub sl_esp_trans: u8,
    /// ESP network (encapsulation) level.
    pub sl_esp_network: u8,
    /// Compression level.
    pub sl_ipcomp: u8,
}

/// The SPD's refusal of a packet (`ipsp_spd_lookup`'s error): the C's `-EINVAL`, a silent
/// drop, or an error the caller returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpdError {
    /// The C's `-EINVAL`: silently drop the packet.
    Drop,
    /// Drop the packet and return this error.
    Errno(Errno),
}

/// `struct tdb *`: an SA of `bsd`'s, with the reference its lookup took; opaque here.
pub type TdbRef = &'static (dyn Any + Sync);

/// The legs of IPsec the kernel calls (`net/if_enc.c`'s, `netinet/ip_output.c`'s
/// `#ifdef IPSEC` code, `netinet/ipsec_output.c`'s).
pub trait Ipsec: Sync {
    /// `enc_getifa(rdomain, unit)`: the address of the enc(4) interface of `unit` in the
    /// routing domain of table `rtableid` (`enc0` for unit 0), `None` when there is none.
    fn enc_getifa(&self, rtableid: u32, unit: u32) -> Option<&'static Ifaddr>;
    /// `ipsec_in_use`: some SA or flow is configured.
    fn in_use(&self) -> bool;
    /// `ip_output_ipsec_lookup(m, hlen, seclevel, &tdb, ipsecflowinfo)`: the SA the policy
    /// wants applied to the IPv4 packet `m` (its header `hlen` bytes long), with a
    /// reference; `None` when no IPsec is needed (no SA, or the packet already went through
    /// this one).
    fn ip_output_lookup(
        &self,
        m: &Mbuf,
        hlen: i32,
        seclevel: Option<&IpsecLevel>,
        ipsecflowinfo: u32,
    ) -> Result<Option<TdbRef>, SpdError>;
    /// `ip_output_ipsec_send(tdb, m, ro, rtableid, fwd)`: `m` handed to the SA `tdb` (pf on
    /// the SA's enc(4) interface, the path MTU checks and update, the TSO chop): the packet
    /// consumed.
    fn ip_output_send(
        &self,
        tdb: TdbRef,
        m: MbufBox,
        ro: &mut Route,
        rtableid: u32,
        fwd: bool,
    ) -> Result<(), Errno>;
    /// `tdb_unref(tdb)`: the lookup's reference given back.
    fn tdb_unref(&self, tdb: TdbRef);
    /// `ipsec_adjust_mtu(m, mtu)`: the path MTU of the SAs `m` went through lowered to
    /// `mtu`, after the packet was too large to send.
    fn adjust_mtu(&self, m: &Mbuf, mtu: u32);
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

/// The reference of an SA a lookup returned, given back by its drop (`tdb_unref`).
pub(crate) struct TdbHold {
    /// The legs that gave it.
    legs: &'static dyn Ipsec,
    /// The SA.
    tdb: TdbRef,
}

impl TdbHold {
    /// `tdb`, whose reference `legs` took.
    pub(crate) fn new(legs: &'static dyn Ipsec, tdb: TdbRef) -> Self {
        Self { legs, tdb }
    }

    /// The SA, lent for a call.
    #[inline]
    pub(crate) fn tdb(&self) -> TdbRef {
        self.tdb
    }

    /// The legs that gave it.
    #[inline]
    pub(crate) fn legs(&self) -> &'static dyn Ipsec {
        self.legs
    }
}

impl Drop for TdbHold {
    fn drop(&mut self) {
        self.legs.tdb_unref(self.tdb);
    }
}
/* </CODE> */
