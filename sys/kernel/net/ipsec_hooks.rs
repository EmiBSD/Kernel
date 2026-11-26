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
//! its `ip_output_ipsec_pmtu_update`), `tdb_unref` and `ipsec_adjust_mtu`; and since N6.5c
//! the same two halves of ip6_output.c's (`ip6_output_ipsec_lookup`,
//! `ip6_output_ipsec_send`, D-18 of N6.5), for `kernel::netinet6::ip6_output`; and since
//! N6.6b0 udp_usrreq.c's `#ifdef IPSEC` points (the inbound check with the SA's flow id, the
//! UDP-encapsulated ESP input and its path MTU control input, and the two switches
//! `udpencap_enable` and `esp_enable`, atomics beside `udpencap_port`).
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
use hal::sync::once::OnceStatic;
use sysdefs::base::errno::Errno;

use crate::assert::kpanic;
use crate::net::if_var::{IfaddrRef, Netstack};
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::route::Route;

/// `sizeof(union sockaddr_union)`: its largest member, a `struct sockaddr_in6` (the SA
/// lookups' address arguments, [`Ipsec::tcp_signature_tdb`]'s).
pub const SOCKADDR_UNION_LEN: usize = core::mem::size_of::<sysdefs::netinet6::in6::SockaddrIn6>();

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
    /// routing domain of table `rtableid` (`enc0` for unit 0), `None` when there is none. A
    /// counted reference (`IfaddrRef::Linked`, T4): the C's pointer is uncounted, kept alive
    /// by the softc, which the caller cannot see; the handle is held for the request only.
    fn enc_getifa(&self, rtableid: u32, unit: u32) -> Option<IfaddrRef>;
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
    /// `ip6_output_ipsec_lookup(m, seclevel, &tdb)`: the SA the policy wants applied to the
    /// IPv6 packet `m`, with a reference; `None` when no IPsec is needed (no SA, or the packet
    /// already went through this one). N6.5c, `ip6_output`'s (and N6.5d's `ip6_forward`).
    fn ip6_output_lookup(
        &self,
        m: &Mbuf,
        seclevel: Option<&IpsecLevel>,
    ) -> Result<Option<TdbRef>, SpdError>;
    /// `ip6_output_ipsec_send(tdb, m, ro, rtableid, fwd)`: the IPv6 packet `m` handed to the SA
    /// `tdb` (pf on the SA's enc(4) interface, the path MTU checks and update, the TSO chop),
    /// lent by its [`TdbHold`]: the packet consumed, the reference kept.
    fn ip6_output_send(
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
    /// `ipsec_forward_check(m, hlen, af) != 0`: whether the policy refuses to forward `m`
    /// (`ip_input_if`, N6.4c2). Default: no refusal (the C without `IPSEC`).
    fn forward_check(&self, _m: &Mbuf, _hlen: i32, _af: i32) -> bool {
        false
    }
    /// `ipsec_local_check(m, hlen, proto, af) != 0`: whether the policy refuses to deliver
    /// `m` locally (`ip_deliver`, N6.4c2). Default: no refusal.
    fn local_check(&self, _m: &Mbuf, _hlen: i32, _proto: i32, _af: i32) -> bool {
        false
    }
    /// `tcp_input`'s `#ifdef IPSEC` block (N6.6a0 for the kernel's TCP; `udp_input`'s at
    /// N6.6b): while IPsec is in use, the inbound policy check of the segment `m` of family
    /// `af`, its IP header `hlen` bytes long, against the SA its innermost
    /// `PACKET_TAG_IPSEC_IN_DONE` tag names (`gettdb`, that lookup's reference given back by
    /// `tdb_unref`) and the socket's levels (`seclevel`, `None` without a pcb):
    /// `ipsp_spd_lookup(.., IPSP_DIRECTION_IN, ..)`, IPsec's own sequence on its side. `Err`
    /// refuses the segment (the caller counts `tcps_rcvnosec` and drops it). Default: `Ok`,
    /// the C without `IPSEC`.
    fn spd_check_in(
        &self,
        _m: &Mbuf,
        _af: i32,
        _hlen: i32,
        _seclevel: Option<&IpsecLevel>,
    ) -> Result<(), SpdError> {
        Ok(())
    }

    /// `udp_input`'s `#ifdef IPSEC` block (N6.6b0): [`Ipsec::spd_check_in`]'s check, and on
    /// success the flow id of the SA the tag names (`tdb->tdb_ids->id_flow`, read before that
    /// lookup's `tdb_unref`; 0 without an SA or without ids), which the socket receives as an
    /// `IP_IPSECFLOWINFO` control message. `Err` refuses the datagram (the caller counts
    /// `udps_nosec` and drops it). A second leg, not a changed one, so TCP's path reads no
    /// ids. Default: `Ok(0)`, the C without `IPSEC`.
    fn spd_check_in_flow(
        &self,
        _m: &Mbuf,
        _af: i32,
        _hlen: i32,
        _seclevel: Option<&IpsecLevel>,
    ) -> Result<u32, SpdError> {
        Ok(0)
    }

    /// The end of `udp_input`'s UDP encapsulation of ESP (N6.6b0): `espstat_inc(esps_udpencin)`
    /// then `ipsec_common_input(mp, skip, protoff, af, IPPROTO_ESP, 1, ns)` of the datagram
    /// `*mp`, whose UDP header the caller removed already (the IP header moved over it):
    /// `skip` the IP header's length, `protoff` the offset of its protocol (or next header)
    /// byte. The answer is `ipsec_common_input`'s: `IPPROTO_DONE` with the packet consumed,
    /// or the inner protocol with the decapsulated packet in `*mp`, for `ip_deliver`'s loop.
    /// Default (no legs: the caller never reaches it without them): the packet freed,
    /// `IPPROTO_DONE`.
    fn udpencap_input(
        &self,
        mp: &mut Option<MbufBox>,
        _skip: i32,
        _protoff: i32,
        _af: i32,
        _ns: Option<&mut Netstack>,
    ) -> i32 {
        drop(mp.take());
        sysdefs::netinet::in_::IPPROTO_DONE
    }

    /// `udpencap_ctlinput(cmd, sa, rdomain, v)` (`udp_ctlinput`'s path MTU discovery for
    /// UDP-encapsulated ESP, N6.6b0): the ICMP message `icmp` (from its type byte:
    /// `CtlArg::Ip`'s bytes) lowers the path MTU of every UDP-encapsulating ESP SA between
    /// the addresses of the IP header it returns, in routing domain `rdomain`. The leg reads
    /// `icmp_nextmtu` and that header from the bytes, checking each length it reads; a short
    /// message changes nothing. Default: nothing, the C without `IPSEC`.
    fn udpencap_ctlinput(&self, _rdomain: u32, _icmp: &[u8]) {}

    /// `gettdbbysrcdst(rdomain, 0, src, dst, IPPROTO_TCP)` (`TCP_SIGNATURE`'s SA, N6.6a0 for
    /// the kernel's TCP): the SA between `src` and `dst`, each a `union sockaddr_union` read
    /// whole ([`SOCKADDR_UNION_LEN`] bytes, never cut at `sa_len`), with the lookup's
    /// reference in the returned [`TdbRef`], which the caller gives back by
    /// [`Ipsec::tdb_unref`] (directly, or through a `TdbHold` made of it, whose drop calls
    /// it). Default: `None`, the C without `IPSEC`.
    fn tcp_signature_tdb(
        &self,
        _rdomain: u32,
        _src: &[u8; SOCKADDR_UNION_LEN],
        _dst: &[u8; SOCKADDR_UNION_LEN],
    ) -> Option<TdbRef> {
        None
    }

    /// The SA's authentication key (`tdb->tdb_amxkey`, `tdb_amxkeylen` bytes) lent to `f`
    /// for `tcp_signature`'s `MD5Update`: never copied into kernel memory by the leg, alive
    /// while the reference `tdb` names is held (the key goes at the SA's `xf_zeroize`, after
    /// its last `tdb_unref`). Without IPsec no [`TdbRef`] exists: the default is `panic(9)`.
    fn tdb_amxkey(&self, _tdb: &TdbRef, _f: &mut dyn FnMut(&[u8])) {
        kpanic(format_args!("tdb_amxkey: an SA without IPsec"))
    }

    /// `ipsec_init()` (`ip_init`, N6.4c2). Default: nothing.
    fn init(&self) {}
    /// `ipsec_sysctl(name, namelen, oldp, oldlenp, newp, newlen)`: the IPsec names of
    /// `net.inet.ip` (`ip_sysctl`, N6.4c2). Default: `EOPNOTSUPP` (the C without `IPSEC`,
    /// whose `ipctl_vars` has none of them).
    fn sysctl(
        &self,
        _name: &[i32],
        _req: &mut crate::base::sysctl::SysctlReq<'_>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}

/// IPsec's legs, once registered.
static IPSEC: OnceStatic<&'static dyn Ipsec> = OnceStatic::new();

/// \[a\] `udpencap_port`: triggers decapsulation (`ipsec_output.c`'s global, the
/// `net.inet.esp.udpencap_port` sysctl's; read by `in_baddynamic`, N6.4c1: here since the
/// kernel's pcb reads it, `bsd`'s IPsec re-exports it until N8).
pub static UDPENCAP_PORT: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(4500);

/// \[a\] `udpencap_enable`: enables the UDP encapsulation of ESP (`ipsec_output.c`'s global,
/// `net.inet.esp.udpencap`'s; N6.6b0: here beside [`UDPENCAP_PORT`], since the kernel's
/// `udp_input` and `udp_ctlinput` load it per datagram, the C's `atomic_load_int`; `bsd`'s
/// IPsec re-exports it until N8).
pub static UDPENCAP_ENABLE: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(1);

/// \[a\] `esp_enable`: ESP accepted on input (`ipsec_input.c`'s global, `net.inet.esp.enable`'s;
/// N6.6b0, as [`UDPENCAP_ENABLE`]).
pub static ESP_ENABLE: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(1);

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
        fn enc_getifa(&self, _rtableid: u32, _unit: u32) -> Option<IfaddrRef> {
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

        fn ip6_output_lookup(
            &self,
            _m: &Mbuf,
            _seclevel: Option<&IpsecLevel>,
        ) -> Result<Option<TdbRef>, SpdError> {
            Ok(None)
        }

        fn ip6_output_send(
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
