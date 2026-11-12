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
//! took ([`TdbHold`], whose drop is the C's `tdb_unref` at `ip_output`'s `done:`). A
//! `TdbRef` is neither `Copy` nor `Clone` and is given back by value, and only the holder of
//! the [`TdbKey`] (the legs' registrant, [`register_ipsec`]) makes one or sees the SA behind
//! it for longer than a borrow: kernel code can neither keep an SA past its `tdb_unref` nor
//! give one back twice (review 1 of N6.4a).

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

/// The capability of IPsec's legs over [`TdbRef`]s: [`register_ipsec`] gives its one value
/// to the registrant, once; nothing else in the kernel can name or make one (a private field,
/// a private static).
pub struct TdbKey(());

/// `struct tdb *`: an SA of `bsd`'s, with the reference its lookup took; opaque here. Neither
/// `Copy` nor `Clone`: one value per reference taken, made by the legs' lookup
/// ([`TdbRef::new`], the key's), lent by its [`TdbHold`] (`&TdbRef`, for
/// [`Ipsec::ip_output_send`]) and consumed by [`Ipsec::tdb_unref`] from the hold's drop.
pub struct TdbRef(&'static (dyn Any + Sync));

impl TdbRef {
    /// The SA `tdb` with the reference the lookup took, for the key's holder (the legs).
    #[inline]
    pub fn new(_key: &TdbKey, tdb: &'static (dyn Any + Sync)) -> Self {
        Self(tdb)
    }

    /// The SA, for the key's holder (the legs' downcast to their own type).
    #[inline]
    pub fn get(&self, _key: &TdbKey) -> &'static (dyn Any + Sync) {
        self.0
    }
}

impl core::fmt::Debug for TdbRef {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("TdbRef")
    }
}

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
    /// the SA's enc(4) interface, the path MTU checks and update, the TSO chop), lent by its
    /// [`TdbHold`]: the packet consumed, the reference kept.
    fn ip_output_send(
        &self,
        tdb: &TdbRef,
        m: MbufBox,
        ro: &mut Route,
        rtableid: u32,
        fwd: bool,
    ) -> Result<(), Errno>;
    /// `tdb_unref(tdb)`: the lookup's reference given back, consumed (from [`TdbHold`]'s
    /// drop).
    fn tdb_unref(&self, tdb: TdbRef);
    /// `ipsec_adjust_mtu(m, mtu)`: the path MTU of the SAs `m` went through lowered to
    /// `mtu`, after the packet was too large to send.
    fn adjust_mtu(&self, m: &Mbuf, mtu: u32);
}

/// IPsec's legs, once registered.
static IPSEC: OnceStatic<&'static dyn Ipsec> = OnceStatic::new();

/// The one [`TdbKey`], given to the legs' registrant.
static TDB_KEY: TdbKey = TdbKey(());

/// Registers IPsec's legs, once (`bsd`'s enc(4), at boot before the first routing request),
/// and gives the registrant the [`TdbKey`] its legs make and read [`TdbRef`]s with; a second
/// call keeps the first, names it and gives no key.
pub fn register_ipsec(imp: &'static dyn Ipsec) -> Result<&'static TdbKey, AlreadyRegistered> {
    IPSEC
        .set(imp)
        .map(|()| &TDB_KEY)
        .map_err(|_| AlreadyRegistered("Ipsec"))
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

    /// The SA, lent for a call (the borrow ends before the hold's drop gives it back).
    #[inline]
    pub(crate) fn tdb(&self) -> &TdbRef {
        &self.tdb
    }

    /// The legs that gave it.
    #[inline]
    pub(crate) fn legs(&self) -> &'static dyn Ipsec {
        self.legs
    }
}

impl Drop for TdbHold {
    fn drop(&mut self) {
        // The reference moved out for the legs, an inert `()` left in its place that no
        // leg ever sees (the hold is gone after this).
        let t = core::mem::replace(&mut self.tdb, TdbRef(&()));
        self.legs.tdb_unref(t);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the SA's reference across the bridge: a hold lends it and gives it back
    // once, by value, on its drop; the key reads what the legs' lookup made.

    use core::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// Legs that count the references given back and remember the last one's address.
    struct CountingLegs {
        unrefs: AtomicUsize,
        last: AtomicUsize,
    }

    impl Ipsec for CountingLegs {
        fn enc_getifa(&self, _rtableid: u32, _unit: u32) -> Option<&'static Ifaddr> {
            None
        }

        fn in_use(&self) -> bool {
            true
        }

        fn ip_output_lookup(
            &self,
            _m: &Mbuf,
            _hlen: i32,
            _seclevel: Option<&IpsecLevel>,
            _ipsecflowinfo: u32,
        ) -> Result<Option<TdbRef>, SpdError> {
            Ok(None)
        }

        fn ip_output_send(
            &self,
            _tdb: &TdbRef,
            m: MbufBox,
            _ro: &mut Route,
            _rtableid: u32,
            _fwd: bool,
        ) -> Result<(), Errno> {
            drop(m);
            Ok(())
        }

        fn tdb_unref(&self, tdb: TdbRef) {
            let sa = tdb.get(&TDB_KEY);
            let addr = core::ptr::from_ref(sa).cast::<u8>() as usize;
            self.last.store(addr, Ordering::Relaxed);
            self.unrefs.fetch_add(1, Ordering::Relaxed);
        }

        fn adjust_mtu(&self, _m: &Mbuf, _mtu: u32) {}
    }

    static LEGS: CountingLegs = CountingLegs {
        unrefs: AtomicUsize::new(0),
        last: AtomicUsize::new(0),
    };

    /// The SA of the test, a `u32` standing for `bsd`'s `Tdb`.
    static SA: u32 = 7;

    #[test]
    fn a_hold_lends_its_reference_and_gives_it_back_once_on_drop() {
        let before = LEGS.unrefs.load(Ordering::Relaxed);
        let hold = TdbHold::new(&LEGS, TdbRef::new(&TDB_KEY, &SA));
        // Lent, not given back: the legs' downcast sees the SA the lookup made.
        let sa: &'static dyn Any = hold.tdb().get(&TDB_KEY);
        assert_eq!(
            sa.downcast_ref::<u32>(),
            Some(&7),
            "the SA behind the lent reference"
        );
        assert!(core::ptr::addr_eq(
            core::ptr::from_ref(hold.legs()),
            core::ptr::from_ref(&LEGS)
        ));
        assert_eq!(LEGS.unrefs.load(Ordering::Relaxed), before);
        drop(hold);
        // Given back once, the SA itself (not the inert value left in the hold).
        assert_eq!(LEGS.unrefs.load(Ordering::Relaxed), before + 1);
        assert_eq!(
            LEGS.last.load(Ordering::Relaxed),
            core::ptr::from_ref(&SA).cast::<u8>() as usize
        );
    }
}
/* </TESTS> */
