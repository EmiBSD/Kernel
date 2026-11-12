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
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1982, 1989, 1993
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
 *	@(#)if_ethersubr.c	8.1 (Berkeley) 6/10/93
 */

/*
%%% portions-copyright-nrl-95
Portions of this software are Copyright 1995-1998 by Randall Atkinson,
Ronald Lee, Daniel McDonald, Bao Phan, and Chris Winters. All Rights
Reserved. All rights under this copyright have been assigned to the US
Naval Research Laboratory (NRL). The NRL Copyright Notice and License
Agreement Version 1.1 (January 17, 1995) applies to these portions of the
software.
You should have received a copy of the license with this software. If you
didn't get a copy, you may request one from <license@ipv6.nrl.navy.mil>.
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
 *	@(#)if_ether.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)if_ether.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Ethernet, the link layer every Ethernet driver shares: `net/if_ethersubr.c` with
//! `<netinet/if_ether.h>`'s kernel part (`struct arpcom`, `struct ether_multi`, `struct
//! ether_extracted`, `struct ether_port`, the multicast macros): attaching an interface
//! (`ether_ifattach`), encapsulating and sending (`ether_output`, `ether_resolve`,
//! `ether_encap`), receiving and handing frames to the protocols (`ether_input`), the
//! multicast filters (`ether_addmulti`, `ether_delmulti`), the CRCs of the drivers' hashes and
//! a quick look inside a frame (`ether_extract_headers`). `<netinet/if_ether.h>`'s userland
//! part is `hal::sys::netinet_if_ether`'s, `<net/ethertypes.h>` `hal::sys::net_ethertypes`'s.
//!
//! Upstream: sys/net/if_ethersubr.c @ 3ce1f3f79392
//! Upstream: sys/netinet/if_ether.h @ 3ce1f3f79392
//! Upstream: sys/netinet/if_ether.c @ 3ce1f3f79392
//! LZ: sys/net/if_ethersubr.rs@3c62ede68802
//! LZ: sys/netinet/if_ether.rs@3c62ede68802
//!
//! A driver's softc embeds an [`Arpcom`], sets its `ac_if` (name, flags, handlers) and the
//! hardware address in `ac_enaddr`, then calls `if_attach(&ac.ac_if)` and
//! [`ether_ifattach`]`(ac)`, which makes [`ETHER`] the interface's link layer: `if_output`
//! (unless the driver has its own), `if_input` and `if_rtrequest`. `ether_input` filters a
//! frame in phases: an aggregation port may take it, a service-delimited (tagged) frame is
//! dropped with no vlan(4), a bridge port may take it, a frame not for this address must be
//! multicast or broadcast (and not our own, unless the interface is simplex); then its type
//! picks the protocol, which `if_input_proto` calls in the softnet thread.
//!
//! **Locks.** The multicast list ([`EtherMultis`]) is inside a `Mutexed` at `IPL_NET` (the
//! C's `splnet()` sections, D-N6.3c-16), taken before `malloc(9)`'s own lock (a new record's
//! `MBox::new` and a record's free run under it, as the C's `malloc`/`free` under `splnet`:
//! the order is the list's mutex, then malloc's), its two counts relaxed atomics written under
//! it and read by the drivers without it, as the C's; the ports are set under the kernel lock
//! (`ether_brport_*`) and read per frame with one load; the address `ac_enaddr` is one word
//! (D-N6.3c-15), read per frame without a lock where the C copies six bytes.
//!
//! ## Deviations
//! - Options and pseudo-devices that are not configured stay comments at their sites (LZ's):
//!   vlan(4) (`NVLAN` 0: a tagged frame is service delimited and dropped unless a bridge takes
//!   it), carp(4), pppoe(4)/`PIPEX`, bpe(4), `MPLS`, and `af_frame` (`NAF_FRAME` 0: the whole
//!   `#if NAF_FRAME > 0` part, `ether_frm_*` and `struct ether_pcb`, is not compiled).
//! - `ether_sprintf` returns the string by value (18 bytes, NUL-terminated) instead of a
//!   static buffer; `ether_multiaddr` returns the range; `ether_encap` answers `Ok(None)` for a
//!   packet address resolution kept (the C's NULL with error 0) (LZ's).
//! - `ether_addmulti`/`ether_delmulti` answer `Err(ENETRESET)` when the list changed, the
//!   C's "error" that tells the driver to reprogram its filter (LZ's).
//! - `ETHERDEBUG` is not defined, so `DPRINTF` prints nothing, as in C.
//!
//! ## Redesign
//! - LZ's `struct arpcom` (in `bsd`'s `netinet/if_ether.rs`, `Cell`s behind an `unsafe impl
//!   Sync`, reached from an interface by a checked cast, `arpcom_of`'s `unsafe`) -> the
//!   kernel's [`Arpcom`], still `ac_if` first and embedded in the drivers' zeroed softcs; the
//!   interface names it by [`Ifnet::if_arpcom`] (a `hal::link::Link` set by
//!   `ether_ifattach`), so `arpcom_of` is one load and no cast. Every member is valid all zero
//!   (the softcs are `M_ZERO`): `ac_enaddr` an atomic word, the counts atomics, the ports
//!   `Link`s, the multicast list a `OnceStatic` that `ether_ifattach` fills (the C's
//!   `LIST_INIT`), since a chain of boxes has no guaranteed all-zero `None`.
//! - `ac_enaddr`'s six bytes -> [`EtherAddrCell`], the address as `ether_addr_to_e64`'s
//!   48-bit word in an `AtomicU64` (D-N6.3c-15): `get()`/`set()` keep the drivers' call
//!   sites, `ether_input` compares the word with one load (the C converts six bytes per
//!   frame), `ether_resolve` converts it back (six shifts where the C copies six bytes).
//! - `ac_multiaddrs` (a `LIST` of `malloc(M_IFMADDR)` records changed under `splnet()`) ->
//!   [`EtherMultis`], a chain of `MBox`es of `M_IFMADDR` newest first (the C's
//!   `LIST_INSERT_HEAD`, one block per record as the C's) inside a `Mutexed` at
//!   `IPL_NET`, taken before malloc's lock (D-N6.3c-16); `enm_refcnt` a count under it.
//!   The drivers' `ETHER_FIRST_MULTI`
//!   / `ETHER_NEXT_MULTI` walk is `for enm in ac.multiaddrs().iter()`, the guard held for the
//!   walk (O(1) a step, as the C's): a step object that held the guard and a position into the
//!   chain it guards cannot be written in safe Rust, so `struct ether_multistep` and the two
//!   macros are gone and the eleven drivers' loops follow (`[[module.fn]]` rows).
//! - `struct ether_port`'s `ep_input`/`ep_port_take`/`ep_port_rele` over `void *ep_port`, in
//!   `SMR_PTR`s -> the [`EtherPort`] trait behind `Link<EtherPortSlot>` (D-N6.3c-17): no port
//!   exists at the pin (bridge, veb, tpmr, aggr, trunk are not ported), so the slots are read
//!   with one load each and set only by `ether_brport_set` under the kernel lock.
//! - `struct ether_extracted`'s pointers into the mbuf -> offsets into the packet
//!   (`Option<u32>`; D-N6.3c-21): the headers are read as copies of `hal::sys`'s images
//!   (`Ip`, `Ip6Hdr`, `Tcphdr`, `Udphdr`) at the C's points and with the C's bounds; `bsd`'s
//!   `sys/net/if_ethersubr.rs` turns the offsets back into the pointers its legacy users
//!   dereference (tcp_input's software LRO, vio, vmx).
//! - `if_input`/`if_output`/`if_rtrequest` set by `ether_ifattach` -> [`ETHER`], the
//!   interface's [`LinkLayer`], behind `NetDriver`'s defaults (D-N6.3c-5); a driver with its
//!   own `if_output` keeps it.
//! - The address resolution and the checksums `ether_resolve`, `ether_input` and
//!   `ether_offload_ifcap` call (`arpresolve`, `nd6_resolve`, `arpinput`, `revarpinput`,
//!   `arp_rtrequest`, `nd6_rtrequest`, `in6_proto_cksum_out`) are legs of the `Inet` bridge
//!   until N6.4/N6.5 (D-N6.3c-14); the IPv4 checksums (`in_hdr_cksum_out`,
//!   `in_proto_cksum_out`) are `kernel::netinet::ip_output`'s since N6.4a.
//! - The Ethernet header is read and written as a copy (`hal::pod::at`/`put`, 14 bytes,
//!   D-N6.3c-20): where a frame starts depends on the driver (two bytes past a word with
//!   `ETHER_ALIGN`), and a copy of 14 bytes per frame is the price of not trusting it (N12).
//!
//! **Lock and spl transitions** against the C: `ether_input`, `ether_output` and
//! `ether_resolve` take none (the C's port sections are SMR read sections, here plain loads);
//! `ether_addmulti`/`ether_delmulti` one `Mutexed` section where the C raises to `splnet` (one
//! spl pair each, as the C's); a driver's multicast walk one section where the C holds
//! nothing of its own (one spl pair more, a control path); `ether_ifattach` the kernel lock's
//! witness in `bpfattach` (bpf.rs's row).

use core::sync::atomic::{AtomicI32, AtomicU64, Ordering};

use hal::intr::IPL_NET;
use hal::link::Link;
use hal::malloc::{M_NOWAIT, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::sys::errno::Errno;
use hal::sys::net_bpf::DLT_EN10MB;
use hal::sys::net_ethertypes::{
    ETHERTYPE_ARP, ETHERTYPE_IP, ETHERTYPE_IPV6, ETHERTYPE_QINQ, ETHERTYPE_REVARP, ETHERTYPE_VLAN,
};
use hal::sys::net_if::{
    IFCAP_CSUM_IPv4, IFCAP_CSUM_TCPv4, IFCAP_CSUM_TCPv6, IFCAP_CSUM_UDPv4, IFCAP_CSUM_UDPv6,
    IFF_MULTICAST, IFF_NOARP, IFF_RUNNING, IFF_SIMPLEX, IFNAMSIZ, IFT_ETHER, Ifreq,
};
use hal::sys::netinet_if_ether::{
    ETHER_ADDR_LEN, ETHER_ALIGN, ETHER_HDR_LEN, ETHERMIN, ETHERMTU, EtherHeader, EtherVlanHeader,
    eth64_is_broadcast, eth64_is_multicast, ether_is_multicast,
};
use hal::sys::netinet_in::{INADDR_ANY, IPPROTO_TCP, IPPROTO_UDP, InAddr, SockaddrIn};
use hal::sys::netinet_ip::{IP_MF, IP_OFFMASK, Ip};
use hal::sys::netinet_ip6::Ip6Hdr;
use hal::sys::netinet_tcp::Tcphdr;
use hal::sys::netinet_udp::Udphdr;
use hal::sys::netinet6_in6::{In6Addr, SockaddrIn6, in6_is_addr_unspecified};
use hal::sys::socket::{AF_INET, AF_INET6, AF_UNSPEC, pseudo_AF_HDRCMPLT};
use hal::sys::sockio::{SIOCADDMULTI, SIOCDELMULTI, SIOCSIFADDR, SIOCSIFMTU};

use crate::assert::{kassert, kpanic};
use crate::net::bpf::bpfattach;
use crate::net::if_::{
    ProtoFn, if_alloc_sadl, if_deactivate, if_enqueue, if_input_local, if_input_proto,
    if_sadl_set_lladdr, inet,
};
use crate::net::if_var::{AtomicCell, Ifnet, LinkLayer, Netstack};
use crate::net::mbuf::{
    M_BCAST, M_COPYALL, M_DONTWAIT, M_IPV4_CSUM_OUT, M_MCAST, M_PKTHDR, M_TCP_CSUM_OUT,
    M_UDP_CSUM_OUT, M_VLANTAG, Mbuf, MbufBox, m_adj, m_copym, m_getptr, m_prepend, m_pullup,
};
use crate::net::proto::SockaddrView;
use crate::net::route::Rtentry;
use crate::prf::{Str, printf};

/// `M_IFMADDR`: the `malloc(9)` type of the multicast records (`<sys/malloc.h>`, `bsd`'s; a
/// copy).
pub(crate) const M_IFMADDR: i32 = 55;

/// `ETHER_CRC_POLY_LE` as the table generator uses it.
const CRC_POLY_LE: u32 = 0xedb8_8320;
/// The big-endian polynomial with its top bit, as the C's table holds it (`0x04c11db7`).
const CRC_POLY_BE: u32 = 0x04c1_1db7;

/// `ether_crc32_le_update`'s `crctab`: the CRC of each nibble, little-endian.
const CRCTAB_LE: [u32; 16] = crctab_le();
/// `ether_crc32_be_update`'s `crctab`.
const CRCTAB_BE: [u32; 16] = crctab_be();
/// `ether_crc32_be_update`'s `rev`: each nibble bit-reversed.
const REV: [u8; 16] = nibble_rev();

/// The bytes of a request [`ether_ioctl`] reads: `SIOCSIFMTU`'s `struct ifreq`, or the
/// multicast requests' address at `ifr_addr`, which `in6_addmulti` makes a `sockaddr_in6`
/// (the C reads a `struct in6_ifreq` through `struct ifreq *`): `IFNAMSIZ` and 28 bytes.
pub const ETHER_IOCTL_MULTI_LEN: usize = IFNAMSIZ + size_of::<SockaddrIn6>();

/// `ac_enaddr`: an Ethernet address as `ether_addr_to_e64`'s 48-bit word, one atomic (the
/// module's redesign). All zero is `00:00:00:00:00:00`.
pub struct EtherAddrCell(AtomicU64);

impl EtherAddrCell {
    /// `00:00:00:00:00:00`.
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// The six bytes.
    #[inline]
    pub fn get(&self) -> [u8; ETHER_ADDR_LEN] {
        let mut ea = [0u8; ETHER_ADDR_LEN];
        ether_e64_to_addr(&mut ea, self.e64());
        ea
    }

    /// `memcpy(ac->ac_enaddr, ea, ETHER_ADDR_LEN)`.
    #[inline]
    pub fn set(&self, ea: [u8; ETHER_ADDR_LEN]) {
        self.0.store(ether_addr_to_e64(&ea), Ordering::Relaxed);
    }

    /// `ether_addr_to_e64(ac->ac_enaddr)`: one load.
    #[inline]
    pub fn e64(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for EtherAddrCell {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct ether_multi`: one multicast address or range the interface listens to. A record
/// of [`EtherMultis`]' chain.
pub struct EtherMulti {
    /// `enm_addrlo`: low or only address of range.
    pub enm_addrlo: [u8; ETHER_ADDR_LEN],
    /// `enm_addrhi`: high or only address of range.
    pub enm_addrhi: [u8; ETHER_ADDR_LEN],
    /// `enm_refcnt`: the claims to this address or range (under the list's mutex).
    pub enm_refcnt: u32,
    /// `enm_list`: the next (older) record.
    enm_next: Option<MBox<EtherMulti>>,
}

/// `ac_multiaddrs`: an interface's multicast records, newest first (the C's
/// `LIST_INSERT_HEAD`), each a `malloc(M_IFMADDR)` block.
pub struct EtherMultis {
    /// `LIST_FIRST(&ac->ac_multiaddrs)`.
    head: Option<MBox<EtherMulti>>,
}

impl EtherMultis {
    /// `LIST_INIT`: no record.
    pub const fn new() -> Self {
        Self { head: None }
    }

    /// The records, newest first (`ETHER_FIRST_MULTI` and `ETHER_NEXT_MULTI`'s order).
    pub fn iter(&self) -> EtherMultiIter<'_> {
        EtherMultiIter {
            next: self.head.as_deref(),
        }
    }

    /// `ETHER_LOOKUP_MULTI(addrlo, addrhi, ac, enm)`: the record of the range, if any.
    fn lookup_mut(
        &mut self,
        addrlo: [u8; ETHER_ADDR_LEN],
        addrhi: [u8; ETHER_ADDR_LEN],
    ) -> Option<&mut EtherMulti> {
        let mut p = self.head.as_deref_mut();
        while let Some(enm) = p {
            if enm.enm_addrlo == addrlo && enm.enm_addrhi == addrhi {
                return Some(enm);
            }
            p = enm.enm_next.as_deref_mut();
        }
        None
    }

    /// Every record unlinked and freed (`ether_ifdetach`'s loop), one at a time.
    fn purge(&mut self) {
        while let Some(mut enm) = self.head.take() {
            self.head = enm.enm_next.take();
        }
    }
}

impl Default for EtherMultis {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a EtherMultis {
    type Item = &'a EtherMulti;
    type IntoIter = EtherMultiIter<'a>;

    fn into_iter(self) -> EtherMultiIter<'a> {
        self.iter()
    }
}

/// The walk of an interface's multicast records ([`EtherMultis::iter`]).
pub struct EtherMultiIter<'a> {
    /// The record the next step gives.
    next: Option<&'a EtherMulti>,
}

impl<'a> Iterator for EtherMultiIter<'a> {
    type Item = &'a EtherMulti;

    fn next(&mut self) -> Option<&'a EtherMulti> {
        let enm = self.next?;
        self.next = enm.enm_next.as_deref();
        Some(enm)
    }
}

/// What an `ac_trport`/`ac_brport` slot names: a port.
pub struct EtherPortSlot {
    /// The port.
    pub port: &'static dyn EtherPort,
}

/// `struct arpcom`: what an Ethernet driver's softc embeds, the interface first. Valid all
/// zero (the module's redesign).
#[repr(C)]
pub struct Arpcom {
    /// `ac_if`: the network-visible interface.
    pub ac_if: Ifnet,
    /// `ac_enaddr`: the hardware address.
    pub ac_enaddr: EtherAddrCell,
    /// `ac_multiaddrs`: the multicast records, made by `ether_ifattach` (the C's `LIST_INIT`).
    ac_multi: OnceStatic<Mutexed<EtherMultis>>,
    /// `ac_multicnt`: the records' count (written under the list's mutex).
    pub ac_multicnt: AtomicCell<i32>,
    /// `ac_multirangecnt`: the ranges' count (the same).
    pub ac_multirangecnt: AtomicCell<i32>,
    /// `ac_trport`: the aggregation port, none at the pin.
    pub ac_trport: Link<EtherPortSlot>,
    /// `ac_brport`: the bridge port, none at the pin.
    pub ac_brport: Link<EtherPortSlot>,
}

impl Arpcom {
    /// What a zeroed softc holds: no address, no list yet, no port.
    pub const fn new() -> Self {
        Self {
            ac_if: Ifnet::new(),
            ac_enaddr: EtherAddrCell::new(),
            ac_multi: OnceStatic::new(),
            ac_multicnt: AtomicCell::new(),
            ac_multirangecnt: AtomicCell::new(),
            ac_trport: Link::new(),
            ac_brport: Link::new(),
        }
    }

    /// The multicast records, locked (a driver's `ETHER_FIRST_MULTI` .. `ETHER_NEXT_MULTI`
    /// walk is `for enm in ac.multiaddrs().iter()`). `panic(9)` before `ether_ifattach`.
    pub fn multiaddrs(&self) -> MutexedGuard<'_, EtherMultis> {
        match self.ac_multi.get() {
            Some(m) => m.lock(),
            None => self
                .ac_if
                .if_xname
                .with_str(|n| kpanic(format_args!("{}: no ether_ifattach", Str(n)))),
        }
    }
}

impl Default for Arpcom {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct ether_extracted`: a quick view of the TCP/IP headers of an Ethernet frame
/// ([`ether_extract_headers`]); the members are the headers' offsets into the packet, `None`
/// for one not found (the C's NULL).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EtherExtracted {
    /// `eh`: the Ethernet header (at 0).
    pub eh: Option<u32>,
    /// `evh`: the 802.1Q header (never: vlan(4) is not configured).
    pub evh: Option<u32>,
    /// `ip4`.
    pub ip4: Option<u32>,
    /// `ip6`.
    pub ip6: Option<u32>,
    /// `tcp`.
    pub tcp: Option<u32>,
    /// `udp`.
    pub udp: Option<u32>,
    /// `iplen`.
    pub iplen: u32,
    /// `iphlen`.
    pub iphlen: u32,
    /// `tcphlen`.
    pub tcphlen: u32,
    /// `paylen`.
    pub paylen: u32,
}

impl EtherExtracted {
    /// `memset(ext, 0, sizeof(*ext))`.
    pub const fn new() -> Self {
        Self {
            eh: None,
            evh: None,
            ip4: None,
            ip6: None,
            tcp: None,
            udp: None,
            iplen: 0,
            iphlen: 0,
            tcphlen: 0,
            paylen: 0,
        }
    }
}

/// Ethernet as an interface's link layer ([`ETHER`]).
pub struct Ether;

/// The Ethernet link layer, every Ethernet interface's `if_link` (`ether_ifattach`).
pub static ETHER: Ether = Ether;

/// `etherbroadcastaddr`.
pub static ETHERBROADCASTADDR: [u8; ETHER_ADDR_LEN] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
/// `etheranyaddr`.
pub static ETHERANYADDR: [u8; ETHER_ADDR_LEN] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// `ether_ipmulticast_min`: the first Ethernet address of the IP multicast range.
pub static ETHER_IPMULTICAST_MIN: [u8; ETHER_ADDR_LEN] = [0x01, 0x00, 0x5e, 0x00, 0x00, 0x00];
/// `ether_ipmulticast_max`: the last one.
pub static ETHER_IPMULTICAST_MAX: [u8; ETHER_ADDR_LEN] = [0x01, 0x00, 0x5e, 0x7f, 0xff, 0xff];
/// `ether_ip6multicast_min`: the first Ethernet address of the IPv6 multicast range.
pub static ETHER_IP6MULTICAST_MIN: [u8; ETHER_ADDR_LEN] = [0x33, 0x33, 0x00, 0x00, 0x00, 0x00];
/// `ether_ip6multicast_max`: the last one.
pub static ETHER_IP6MULTICAST_MAX: [u8; ETHER_ADDR_LEN] = [0x33, 0x33, 0xff, 0xff, 0xff, 0xff];

/// `ether_fakeaddr`'s `unit`.
static FAKEADDR_UNIT: AtomicI32 = AtomicI32::new(0);

/// `struct ether_port`: an aggregation (`ac_trport`: aggr(4), trunk(4)) or bridge
/// (`ac_brport`: bridge(4), veb(4), tpmr(4)) port on an Ethernet interface (decision 46; no
/// implementer at the pin).
pub trait EtherPort: Sync + 'static {
    /// `ep_input(ifp, m, dst, ep_port, ns)`: the port's look at a received frame addressed to
    /// `dst` (its e64 form); the frame back when the port does not take it.
    fn input(
        &self,
        ifp: &'static Ifnet,
        m: MbufBox,
        dst: u64,
        ns: Option<&mut Netstack>,
    ) -> Option<MbufBox>;

    /// `ep_port_take(ep_port)`: a reference to the port for the frame's way through it.
    fn take(&self);

    /// `ep_port_rele(ref, ep_port)`: the reference [`EtherPort::take`] took, released.
    fn rele(&self);
}

impl LinkLayer for Ether {
    fn output(
        &self,
        ifp: &'static Ifnet,
        m: MbufBox,
        dst: SockaddrView<'_>,
        rt: Option<&Rtentry>,
    ) -> Result<(), Errno> {
        ether_output(ifp, m, dst, rt)
    }

    fn input(&self, ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
        ether_input(ifp, m, ns);
    }

    fn rtrequest(
        &self,
        ifp: &'static Ifnet,
        req: u8,
        rt: Option<&Rtentry>,
        _net: &hal::netlock::NetHeld<'_>,
    ) {
        ether_rtrequest(ifp, req, rt);
    }
}

/// `(struct arpcom *)ifp`: the `struct arpcom` an Ethernet interface is the first member of.
/// `panic(9)` for an interface `ether_ifattach` did not attach (the C's cast would read
/// another structure).
#[inline]
pub fn arpcom_of(ifp: &Ifnet) -> &'static Arpcom {
    match ifp.if_arpcom.get() {
        Some(ac) => ac,
        None => ifp
            .if_xname
            .with_str(|n| kpanic(format_args!("{}: not an arpcom", Str(n)))),
    }
}

/// `ETHER_MAP_IP_MULTICAST(ipaddr, enaddr)`: an IP multicast address's Ethernet address,
/// the high 25 bits fixed and the low 23 the IP address's.
pub const fn ether_map_ip_multicast(ipaddr: &InAddr) -> [u8; ETHER_ADDR_LEN] {
    let ip = ipaddr.s_addr.to_ne_bytes();
    [0x01, 0x00, 0x5e, ip[1] & 0x7f, ip[2], ip[3]]
}

/// `ETHER_MAP_IPV6_MULTICAST(ip6addr, enaddr)`: an IPv6 multicast address's Ethernet
/// address, the high 16 bits fixed and the low 32 the IPv6 address's.
pub const fn ether_map_ipv6_multicast(ip6addr: &In6Addr) -> [u8; ETHER_ADDR_LEN] {
    let a = &ip6addr.s6_addr;
    [0x33, 0x33, a[12], a[13], a[14], a[15]]
}

/// `ether_ioctl(ifp, arp, cmd, data)`: the Ethernet part of a driver's `if_ioctl`:
/// `SIOCSIFMTU` within `[ETHERMIN, if_hardmtu]`, the multicast list (`ENETRESET` when it
/// changed); `ENOTTY` for what the driver handles itself. `data` holds the bytes the command
/// carries ([`ether_ioctl_len`]).
pub fn ether_ioctl(ifp: &Ifnet, arp: &Arpcom, cmd: u64, data: &[u8]) -> Result<(), Errno> {
    match cmd {
        SIOCSIFADDR => Ok(()),

        SIOCSIFMTU => {
            let ifr = hal::pod::at::<Ifreq>(data, 0).ok_or(Errno::EINVAL)?;
            let mtu = ifr.ifr_mtu();
            if mtu < ETHERMIN as i32 || mtu as u32 > ifp.if_hardmtu.get() {
                Err(Errno::EINVAL)
            } else {
                ifp.if_mtu.set(mtu as u32);
                Ok(())
            }
        }

        SIOCADDMULTI | SIOCDELMULTI => {
            if ifp.if_flags.get() & IFF_MULTICAST == 0 {
                return Err(Errno::ENOTTY);
            }
            if cmd == SIOCADDMULTI {
                ether_addmulti(data, arp)
            } else {
                ether_delmulti(data, arp)
            }
        }

        _ => Err(Errno::ENOTTY),
    }
}

/// How many bytes of `cmd`'s argument [`ether_ioctl`] reads (0 for a command whose argument
/// it ignores: `SIOCSIFADDR`'s is the protocol's `struct ifaddr`).
pub const fn ether_ioctl_len(cmd: u64) -> usize {
    match cmd {
        SIOCSIFMTU => size_of::<Ifreq>(),
        SIOCADDMULTI | SIOCDELMULTI => ETHER_IOCTL_MULTI_LEN,
        _ => 0,
    }
}

/// `ether_rtrequest(ifp, req, rt)`: an Ethernet interface's route change, to ARP or ND by
/// the route's family.
pub fn ether_rtrequest(ifp: &'static Ifnet, req: u8, rt: Option<&Rtentry>) {
    let Some(rt) = rt else {
        return;
    };
    let Some(i) = inet() else {
        return;
    };

    match rt.dest().get(1).copied() {
        Some(AF_INET) => i.arp_rtrequest(ifp, req, rt),
        Some(AF_INET6) => i.nd6_rtrequest(ifp, req, rt),
        _ => {}
    }
}

/// `ether_resolve(ifp, m, dst, rt, eh)`: the Ethernet header `eh` of a packet for `dst`: the
/// destination from address resolution (or the socket address's data), the type, our source.
/// The packet back when it may be sent; `Err(EAGAIN)` when address resolution kept it, any
/// other error once it is freed.
pub fn ether_resolve(
    ifp: &'static Ifnet,
    mut m: MbufBox,
    dst: SockaddrView<'_>,
    rt: Option<&Rtentry>,
    eh: &mut EtherHeader,
) -> Result<MbufBox, Errno> {
    let ac = arpcom_of(ifp);
    let af = dst.family();

    let error = 'bad: {
        if ifp.if_flags.get() & IFF_RUNNING == 0 {
            break 'bad Errno::ENETDOWN;
        }

        kassert!(
            rt.is_some()
                || m.flags() & (M_MCAST | M_BCAST) != 0
                || af == AF_UNSPEC
                || af == pseudo_AF_HDRCMPLT
        );

        // DIAGNOSTIC: the "trying to send packet on wrong domain" print of an interface whose
        // rdomain is not the packet's table's (not compiled: `diagnostic` is off in the
        // kernel crate's builds, as LZ's).

        match af {
            AF_INET => {
                let Some(i) = inet() else {
                    break 'bad Errno::EAFNOSUPPORT;
                };
                // arpresolve owns the packet when it fails (or holds it: EAGAIN).
                m = i.arpresolve(ifp, rt, m, dst, &mut eh.ether_dhost)?;
                eh.ether_type = ETHERTYPE_IP.to_be();

                // If broadcasting on a simplex interface, loopback a copy. The checksum must
                // be calculated in software. Keep the condition in sync with
                // in_ifcap_cksum().
                if m.flags() & M_BCAST != 0
                    && ifp.if_flags.get() & IFF_SIMPLEX != 0
                    && m.m_pkthdr().pf.routed.get() == 0
                {
                    // XXX Should we input an unencrypted IPsec packet?
                    if let Some(mcopy) = m_copym(&mut m, 0, M_COPYALL, M_NOWAIT) {
                        let _ = if_input_local(ifp, mcopy, af, None);
                    }
                }
            }
            AF_INET6 => {
                let Some(i) = inet() else {
                    break 'bad Errno::EAFNOSUPPORT;
                };
                // nd6_resolve owns the packet when it fails (or holds it: EAGAIN).
                m = i.nd6_resolve(ifp, rt, m, dst, &mut eh.ether_dhost)?;
                eh.ether_type = ETHERTYPE_IPV6.to_be();
            }
            // MPLS: the gateway's link address or address resolution, ETHERTYPE_MPLS(_MCAST);
            // not configured.
            af if af == pseudo_AF_HDRCMPLT => {
                // take the whole header from the sa
                let Some(h) = sa_data_header(dst) else {
                    break 'bad Errno::EINVAL;
                };
                *eh = h;
                return Ok(m);
            }

            AF_UNSPEC => {
                // take the dst and type from the sa, but get src below
                let Some(h) = sa_data_header(dst) else {
                    break 'bad Errno::EINVAL;
                };
                *eh = h;
            }

            _ => {
                ifp.if_xname.with_str(|n| {
                    printf(format_args!("{}: can't handle af{}\n", Str(n), af));
                });
                break 'bad Errno::EAFNOSUPPORT;
            }
        }

        eh.ether_shost = ac.ac_enaddr.get();

        return Ok(m);
    };

    // bad:
    drop(m);
    Err(error)
}

/// `memcpy(eh, dst->sa_data, sizeof(*eh))`: the header in a socket address's 14 data bytes;
/// `None` for an address shorter than a `struct sockaddr` (no sender makes one).
fn sa_data_header(dst: SockaddrView<'_>) -> Option<EtherHeader> {
    hal::pod::at::<EtherHeader>(dst.bytes(), 2)
}

/// `ether_encap(ifp, m, dst, rt, &error)`: the destination resolved and the Ethernet header
/// prepended (`ETHER_ALIGN` bytes before it trimmed). `Ok(None)` when address resolution kept
/// the packet for later (the C's `EAGAIN`, answered as success); `ENOBUFS` without room.
pub fn ether_encap(
    ifp: &'static Ifnet,
    m: MbufBox,
    dst: SockaddrView<'_>,
    rt: Option<&Rtentry>,
) -> Result<Option<MbufBox>, Errno> {
    let mut eh = EtherHeader::default();

    let m = match ether_resolve(ifp, m, dst, rt, &mut eh) {
        Ok(m) => m,
        Err(Errno::EAGAIN) => return Ok(None),
        Err(e) => return Err(e),
    };

    let Some(mut m) = m_prepend(m, (ETHER_ALIGN + ETHER_HDR_LEN) as i32, M_DONTWAIT) else {
        return Err(Errno::ENOBUFS);
    };

    m_adj(&m, ETHER_ALIGN as i32);
    // m_prepend left the header's bytes at m_data, in a writable mbuf of its own when there
    // was no leading space.
    let written = m.data_mut().is_some_and(|d| hal::pod::put(d, 0, &eh));
    if written {
        Ok(Some(m))
    } else {
        Err(Errno::ENOBUFS)
    }
}

/// `ether_output(ifp, m, dst, rt)`: an Ethernet interface's `if_output`: encapsulate and
/// enqueue.
pub fn ether_output(
    ifp: &'static Ifnet,
    m: MbufBox,
    dst: SockaddrView<'_>,
    rt: Option<&Rtentry>,
) -> Result<(), Errno> {
    let Some(m) = ether_encap(ifp, m, dst, rt)? else {
        return Ok(());
    };

    if_enqueue(ifp, m)
}

/// `ether_port_input(ifp, m, dst, epp, ns)`: the frame offered to the port in `epp`, if any,
/// which gives it back when it does not take it.
fn ether_port_input(
    ifp: &'static Ifnet,
    m: MbufBox,
    dst: u64,
    epp: &Link<EtherPortSlot>,
    ns: Option<&mut Netstack>,
) -> Option<MbufBox> {
    let Some(ep) = epp.get() else {
        return Some(m);
    };
    ep.port.take();
    let m = ep.port.input(ifp, m, dst, ns);
    ep.port.rele();
    m
}

/// `ether_input(ifp, m, ns)`: a received Ethernet frame through the phases of filtering (the
/// module's docs) to its protocol, or dropped.
pub fn ether_input(ifp: &'static Ifnet, m: MbufBox, mut ns: Option<&mut Netstack>) {
    let ac = arpcom_of(ifp);
    let mut sdelim = false;

    // Drop short frames
    let Some(eh) = hal::pod::at::<EtherHeader>(m.data(), 0) else {
        return;
    };
    let dst = ether_addr_to_e64(&eh.ether_dhost);

    // First phase:
    //
    // The first phase supports drivers that aggregate multiple Ethernet ports into a single
    // logical interface, ie, aggr(4) and trunk(4).
    let Some(m) = ether_port_input(ifp, m, dst, &ac.ac_trport, ns.as_deref_mut()) else {
        return;
    };

    // Second phase: service delimited packet filtering.
    //
    // Let vlan(4) and svlan(4) look at "service delimited" packets. If a virtual interface
    // does not exist to take those packets, they're returned to ether_input() so a bridge
    // can have a go at forwarding them.

    let etype = u16::from_be(eh.ether_type);

    if m.flags() & M_VLANTAG != 0 || etype == ETHERTYPE_VLAN || etype == ETHERTYPE_QINQ {
        // NVLAN > 0: m = vlan_input(ifp, m, &sdelim, ns); vlan(4) is not configured.
        sdelim = true;
    }

    // Third phase: bridge processing.
    //
    // Give the packet to a bridge interface, ie, bridge(4), veb(4), or tpmr(4), if it is
    // configured. A bridge may take the packet and forward it to another port, or it may
    // return it here to ether_input() to support local delivery to this port.
    let Some(m) = ether_port_input(ifp, m, dst, &ac.ac_brport, ns.as_deref_mut()) else {
        return;
    };

    // Fourth phase: drop service delimited packets.
    //
    // If the packet has a tag, and a bridge didn't want it, it's not for this port.
    if sdelim {
        return;
    }

    // Fifth phase: destination address check.
    //
    // Is the packet specifically addressed to this port? A port may have handed back another
    // mbuf: the header is read again from the frame as it is now (the C's second mtod), and
    // the type with it; the destination stays the first read's, as the C's `dst`.
    let Some(eh) = hal::pod::at::<EtherHeader>(m.data(), 0) else {
        return;
    };
    let self_ = ac.ac_enaddr.e64();
    if dst != self_ {
        // NCARP > 0: if it's not for this port, it could be for carp(4) (carp_input on an
        // IFT_ETHER with carp interfaces); not configured.

        // If not, it must be multicast or broadcast to go further.
        if !eth64_is_multicast(dst) {
            return;
        }

        // If this is not a simplex interface, drop the packet if it came from us.
        if ifp.if_flags.get() & IFF_SIMPLEX == 0 && self_ == ether_addr_to_e64(&eh.ether_shost) {
            return;
        }

        let bit = if eth64_is_broadcast(dst) {
            M_BCAST
        } else {
            M_MCAST
        };
        m.m_flags().set(m.flags() | bit);
        ifp.if_imcasts().set(ifp.if_imcasts().get() + 1);
    }

    // Sixth phase: protocol demux.
    //
    // At this point it is known that the packet is destined for layer 3 protocol handling on
    // the local port.
    let etype = u16::from_be(eh.ether_type);
    let Some(i) = inet() else {
        return;
    };
    let input: ProtoFn = match etype {
        ETHERTYPE_IP => i.ipv4_input(),

        ETHERTYPE_ARP => {
            if ifp.if_flags.get() & IFF_NOARP != 0 {
                return;
            }
            i.arpinput()
        }

        ETHERTYPE_REVARP => {
            if ifp.if_flags.get() & IFF_NOARP != 0 {
                return;
            }
            i.revarpinput()
        }

        // Schedule IPv6 software interrupt for incoming IPv6 packet.
        ETHERTYPE_IPV6 => i.ipv6_input(),
        // NPPPOE > 0 || PIPEX: ETHERTYPE_PPPOEDISC/ETHERTYPE_PPPOE go to the pppoe queues or
        // a pipex session; MPLS: ETHERTYPE_MPLS(_MCAST) to mpls_input; NBPE > 0:
        // ETHERTYPE_PBB to bpe_input; NAF_FRAME > 0: ether_frm_input. None is configured.
        _ => return,
    };

    m_adj(&m, ETHER_HDR_LEN as i32);
    if_input_proto(ifp, m, input, ns);
}

/// `ether_brport_isset(ifp)`: `EBUSY` when a bridge port is installed. Under the kernel lock.
pub fn ether_brport_isset(ifp: &Ifnet) -> Result<(), Errno> {
    let ac = arpcom_of(ifp);

    hal::klock::kernel_assert_locked();
    if !ac.ac_brport.is_none() {
        return Err(Errno::EBUSY);
    }

    Ok(())
}

/// `ether_brport_set(ifp, ep)`: installs a bridge port; `panic(9)` when one is. Under the
/// kernel lock.
pub fn ether_brport_set(ifp: &Ifnet, ep: &'static EtherPortSlot) {
    let ac = arpcom_of(ifp);

    hal::klock::kernel_assert_locked();
    if !ac.ac_brport.is_none() {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!("{} setting an already set brport", Str(n)));
        });
    }

    ac.ac_brport.set(Some(ep));
}

/// `ether_brport_clr(ifp)`: removes the bridge port; `panic(9)` when there is none. Under the
/// kernel lock.
pub fn ether_brport_clr(ifp: &Ifnet) {
    let ac = arpcom_of(ifp);

    hal::klock::kernel_assert_locked();
    if ac.ac_brport.is_none() {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!("{} clearing an already clear brport", Str(n)));
        });
    }

    ac.ac_brport.set(None);
}

/// `ether_brport_get(ifp)`: the bridge port (the C's caller is inside an SMR read section; a
/// port here lives for ever, so the load needs none).
pub fn ether_brport_get(ifp: &Ifnet) -> Option<&'static EtherPortSlot> {
    arpcom_of(ifp).ac_brport.get()
}

/// `ether_brport_get_locked(ifp)`: the bridge port, under the kernel lock.
pub fn ether_brport_get_locked(ifp: &Ifnet) -> Option<&'static EtherPortSlot> {
    let ac = arpcom_of(ifp);
    hal::klock::kernel_assert_locked();
    ac.ac_brport.get()
}

/// `ether_sprintf(ap)`: an Ethernet address as `xx:xx:xx:xx:xx:xx` and a NUL.
pub fn ether_sprintf(ap: &[u8; ETHER_ADDR_LEN]) -> [u8; ETHER_ADDR_LEN * 3] {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut etherbuf = [0u8; ETHER_ADDR_LEN * 3];

    for (i, &b) in ap.iter().enumerate() {
        etherbuf[i * 3] = DIGITS[usize::from(b >> 4)];
        etherbuf[i * 3 + 1] = DIGITS[usize::from(b & 0xf)];
        etherbuf[i * 3 + 2] = b':';
    }
    etherbuf[ETHER_ADDR_LEN * 3 - 1] = 0;
    etherbuf
}

/// `ether_fakeaddr(ifp)`: a random, locally administered, non-multicast address.
pub fn ether_fakeaddr(ifp: &Ifnet) {
    let rng = hal::random::arc4random();
    let unit = FAKEADDR_UNIT.fetch_add(1, Ordering::Relaxed);

    // Non-multicast; locally administered address
    arpcom_of(ifp).ac_enaddr.set([
        0xfe,
        0xe1,
        0xba,
        0xd0 | (unit & 0xf) as u8,
        rng as u8,
        (rng >> 8) as u8,
    ]);
}

/// `ether_ifattach(ifp)`: an Ethernet interface's common set-up, after `if_attach`: the type,
/// the lengths and the MTU, [`ETHER`] as its link layer, its link-level name holding the
/// address (an obviously invalid one replaced), the multicast list, its bpf(4) tap.
pub fn ether_ifattach(ac: &'static Arpcom) {
    let ifp = &ac.ac_if;
    ifp.if_arpcom.set(Some(ac));

    // Any interface which provides a MAC address which is obviously invalid gets whacked, so
    // that users will notice.
    if ether_is_multicast(&ac.ac_enaddr.get()) {
        ether_fakeaddr(ifp);
    }

    ifp.if_type.set(IFT_ETHER);
    ifp.if_addrlen.set(ETHER_ADDR_LEN as u8);
    ifp.if_hdrlen.set(ETHER_HDR_LEN as u8);
    ifp.if_mtu.set(ETHERMTU as u32);
    // if_input = ether_input, if_output = ether_output unless the driver has one,
    // if_rtrequest = ether_rtrequest: the link layer behind the driver's defaults.
    if ifp.if_link.set(&ETHER).is_err() {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!("{}: ether_ifattach twice", Str(n)));
        });
    }

    if ifp.if_hardmtu.get() == 0 {
        ifp.if_hardmtu.set(ETHERMTU as u32);
    }

    if_alloc_sadl(ifp);
    // memcpy(LLADDR(ifp->if_sadl), ac->ac_enaddr, ifp->if_addrlen)
    if_sadl_set_lladdr(ifp, &ac.ac_enaddr.get());
    let _ = ac.ac_multi.set(Mutexed::new(IPL_NET, EtherMultis::new()));
    bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
}

/// `ether_ifdetach(ifp)`: the pseudo-drivers' changes undone and the multicast records freed.
pub fn ether_ifdetach(ifp: &Ifnet) {
    let ac = arpcom_of(ifp);

    // Undo pseudo-driver changes.
    if_deactivate(ifp);

    if let Some(m) = ac.ac_multi.get() {
        m.lock().purge();
    }
}

/// `ether_crc32_le_update(crc, buf, len)`: the little-endian Ethernet CRC of `buf` from
/// `crc`, a nibble at a time.
pub fn ether_crc32_le_update(mut crc: u32, buf: &[u8]) -> u32 {
    for &b in buf {
        crc ^= u32::from(b);
        crc = (crc >> 4) ^ CRCTAB_LE[(crc & 0xf) as usize];
        crc = (crc >> 4) ^ CRCTAB_LE[(crc & 0xf) as usize];
    }

    crc
}

/// `ether_crc32_be_update(crc, buf, len)`: the big-endian Ethernet CRC of `buf` from `crc`.
pub fn ether_crc32_be_update(mut crc: u32, buf: &[u8]) -> u32 {
    for &data in buf {
        crc = (crc << 4)
            ^ CRCTAB_BE[((crc >> 28) ^ u32::from(REV[usize::from(data & 0xf)])) as usize];
        crc =
            (crc << 4) ^ CRCTAB_BE[((crc >> 28) ^ u32::from(REV[usize::from(data >> 4)])) as usize];
    }

    crc
}

/// `ether_crc32_le(buf, len)`.
pub fn ether_crc32_le(buf: &[u8]) -> u32 {
    ether_crc32_le_update(0xffff_ffff, buf)
}

/// `ether_crc32_be(buf, len)`.
pub fn ether_crc32_be(buf: &[u8]) -> u32 {
    ether_crc32_be_update(0xffff_ffff, buf)
}

/// The little-endian nibble table: entry `i` is the register after nibble `i` is shifted out
/// with the reflected polynomial (the C's constant table, computed at compile time and
/// checked against it by the tests).
const fn crctab_le() -> [u32; 16] {
    let mut tab = [0u32; 16];
    let mut i = 0;
    while i < 16 {
        let mut c = i as u32;
        let mut j = 0;
        while j < 4 {
            c = if c & 1 != 0 {
                (c >> 1) ^ CRC_POLY_LE
            } else {
                c >> 1
            };
            j += 1;
        }
        tab[i] = c;
        i += 1;
    }
    tab
}

/// The big-endian nibble table: entry `i` is nibble `i` at the top of the register, shifted
/// out with the polynomial.
const fn crctab_be() -> [u32; 16] {
    let mut tab = [0u32; 16];
    let mut i = 0;
    while i < 16 {
        let mut c = (i as u32) << 28;
        let mut j = 0;
        while j < 4 {
            c = if c & 0x8000_0000 != 0 {
                (c << 1) ^ CRC_POLY_BE
            } else {
                c << 1
            };
            j += 1;
        }
        tab[i] = c;
        i += 1;
    }
    tab
}

/// Each nibble with its four bits reversed.
const fn nibble_rev() -> [u8; 16] {
    let mut tab = [0u8; 16];
    let mut i = 0;
    while i < 16 {
        let n = i as u8;
        tab[i] = ((n & 1) << 3) | ((n & 2) << 1) | ((n & 4) >> 1) | ((n & 8) >> 3);
        i += 1;
    }
    tab
}

/// `ether_multiaddr(sa, addrlo, addrhi)`: the Ethernet address or range of a multicast socket
/// address (`sa`, the request's `ifr_addr` and what follows it): `AF_UNSPEC` its data,
/// `AF_INET`/`AF_INET6` the group's address, or every IP/IPv6 group for the wildcard;
/// `EAFNOSUPPORT` otherwise.
pub fn ether_multiaddr(sa: &[u8]) -> Result<([u8; ETHER_ADDR_LEN], [u8; ETHER_ADDR_LEN]), Errno> {
    match sa.get(1).copied() {
        Some(AF_UNSPEC) => {
            let data = sa.get(2..2 + ETHER_ADDR_LEN).ok_or(Errno::EINVAL)?;
            let mut addrlo = [0u8; ETHER_ADDR_LEN];
            addrlo.copy_from_slice(data);
            Ok((addrlo, addrlo))
        }

        Some(AF_INET) => {
            let sin = hal::pod::at::<SockaddrIn>(sa, 0).ok_or(Errno::EINVAL)?;
            if sin.sin_addr.s_addr == INADDR_ANY {
                // An IP address of INADDR_ANY means listen to or stop listening to all of the
                // Ethernet multicast addresses used for IP. (This is for the sake of IP
                // multicast routers.)
                Ok((ETHER_IPMULTICAST_MIN, ETHER_IPMULTICAST_MAX))
            } else {
                let addrlo = ether_map_ip_multicast(&sin.sin_addr);
                Ok((addrlo, addrlo))
            }
        }

        Some(AF_INET6) => {
            let sin6 = hal::pod::at::<SockaddrIn6>(sa, 0).ok_or(Errno::EINVAL)?;
            if in6_is_addr_unspecified(&sin6.sin6_addr) {
                // An IP6 address of 0 means listen to or stop listening to all of the Ethernet
                // multicast address used for IP6.
                //
                // (This might not be healthy, given IPv6's reliance on multicast for things
                // like neighbor discovery. Perhaps initializing all-nodes, solicited nodes, and
                // possibly all-routers for this interface afterwards is not a bad idea.)
                Ok((ETHER_IP6MULTICAST_MIN, ETHER_IP6MULTICAST_MAX))
            } else {
                let addrlo = ether_map_ipv6_multicast(&sin6.sin6_addr);
                Ok((addrlo, addrlo))
            }
        }

        _ => Err(Errno::EAFNOSUPPORT),
    }
}

/// `ifr->ifr_addr` and what follows it in a request's bytes.
fn ifr_addr(ifr: &[u8]) -> &[u8] {
    ifr.get(IFNAMSIZ..).unwrap_or(&[])
}

/// `ether_addmulti(ifr, ac)`: a multicast address or range added to the interface's list (a
/// claim more on a record already there); `Err(ENETRESET)` tells the driver the list changed
/// and its reception filter must be reprogrammed.
pub fn ether_addmulti(ifr: &[u8], ac: &Arpcom) -> Result<(), Errno> {
    let (addrlo, addrhi) = ether_multiaddr(ifr_addr(ifr))?;

    // Verify that we have valid Ethernet multicast addresses.
    if addrlo[0] & 0x01 != 1 || addrhi[0] & 0x01 != 1 {
        return Err(Errno::EINVAL);
    }

    let mut g = ac.multiaddrs();
    // See if the address range is already in the list.
    if let Some(enm) = g.lookup_mut(addrlo, addrhi) {
        // Found it; just increment the reference count.
        enm.enm_refcnt += 1;
        return Ok(());
    }
    // New address or range; malloc a new multicast record and link it into the interface's
    // multicast list.
    let rec = EtherMulti {
        enm_addrlo: addrlo,
        enm_addrhi: addrhi,
        enm_refcnt: 1,
        enm_next: None,
    };
    let Some(mut enm) = MBox::new(M_IFMADDR, M_NOWAIT, rec) else {
        return Err(Errno::ENOBUFS);
    };
    enm.enm_next = g.head.take();
    g.head = Some(enm);
    ac.ac_multicnt.set(ac.ac_multicnt.get() + 1);
    if addrlo != addrhi {
        ac.ac_multirangecnt.set(ac.ac_multirangecnt.get() + 1);
    }
    drop(g);
    // Return ENETRESET to inform the driver that the list has changed and its reception filter
    // should be adjusted accordingly.
    Err(Errno::ENETRESET)
}

/// `ether_delmulti(ifr, ac)`: a claim on a multicast record released, the record freed with
/// its last (`Err(ENETRESET)`, as [`ether_addmulti`]); `ENXIO` for a range not in the list.
pub fn ether_delmulti(ifr: &[u8], ac: &Arpcom) -> Result<(), Errno> {
    let (addrlo, addrhi) = ether_multiaddr(ifr_addr(ifr))?;

    let mut g = ac.multiaddrs();
    // Look up the address in our list.
    let mut p = &mut g.head;
    loop {
        if p.as_ref()
            .is_none_or(|e| e.enm_addrlo == addrlo && e.enm_addrhi == addrhi)
        {
            break;
        }
        match p {
            Some(e) => p = &mut e.enm_next,
            None => return Err(Errno::ENXIO),
        }
    }
    let Some(enm) = p.as_mut() else {
        return Err(Errno::ENXIO);
    };
    enm.enm_refcnt -= 1;
    if enm.enm_refcnt != 0 {
        // Still some claims to this record.
        return Ok(());
    }
    // No remaining claims to this record; unlink and free it.
    if let Some(mut gone) = p.take() {
        *p = gone.enm_next.take();
    }
    ac.ac_multicnt.set(ac.ac_multicnt.get() - 1);
    if addrlo != addrhi {
        ac.ac_multirangecnt.set(ac.ac_multirangecnt.get() - 1);
    }
    drop(g);
    // Return ENETRESET to inform the driver that the list has changed and its reception filter
    // should be adjusted accordingly.
    Err(Errno::ENETRESET)
}

/// `ether_addr_to_e64(ea)`: the address as a 48-bit integer, the first octet highest.
#[inline]
pub fn ether_addr_to_e64(ea: &[u8; ETHER_ADDR_LEN]) -> u64 {
    ea.iter().fold(0, |e64, &b| (e64 << 8) | u64::from(b))
}

/// `ether_e64_to_addr(ea, e64)`: the inverse of [`ether_addr_to_e64`].
#[inline]
pub fn ether_e64_to_addr(ea: &mut [u8; ETHER_ADDR_LEN], mut e64: u64) {
    for b in ea.iter_mut().rev() {
        *b = e64 as u8;
        e64 >>= 8;
    }
}

/// `ether_extract_headers(m0, ext)`: where the IPv4 or IPv6 header and the TCP or UDP header
/// of a frame are, and their lengths, read at the C's points with the C's bounds (each header
/// inside one mbuf); a member stays `None` for a header not recognised.
pub fn ether_extract_headers(m0: &Mbuf, ext: &mut EtherExtracted) {
    *ext = EtherExtracted::new();

    kassert!(m0.flags() & M_PKTHDR != 0);
    ext.paylen = m0.m_pkthdr().len.get() as u32;

    let Some(eh) = hal::pod::at::<EtherHeader>(m0.data(), 0) else {
        return;
    };
    ext.eh = Some(0);
    let mut hlen = ETHER_HDR_LEN;
    if (ext.paylen as usize) < hlen {
        ext.eh = None;
        return;
    }
    ext.paylen -= hlen as u32;
    let ether_type = u16::from_be(eh.ether_type);

    // NVLAN > 0: an ETHERTYPE_VLAN frame is looked at through its ether_vlan_header (evh);
    // vlan(4) is not configured.

    // The IP header's mbuf, its offset in it and in the packet, and the protocol.
    let (m, hoff, off, ipproto) = match ether_type {
        ETHERTYPE_IP => {
            let Some((m, hoff)) = m_getptr(m0, hlen as i32) else {
                return;
            };
            let seg = m.data().get(hoff as usize..).unwrap_or(&[]);
            let Some(ip) = hal::pod::at::<Ip>(seg, 0) else {
                return;
            };
            ext.ip4 = Some(hlen as u32);

            let off = hlen;
            hlen = usize::from(ip.ip_hl()) << 2;
            let iplen = usize::from(u16::from_be(ip.ip_len));
            if seg.len() < hlen
                || (ext.paylen as usize) < hlen
                || (ext.paylen as usize) < iplen
                || iplen < hlen
            {
                ext.ip4 = None;
                return;
            }
            ext.iplen = iplen as u32;
            ext.iphlen = hlen as u32;
            ext.paylen -= hlen as u32;

            if u16::from_be(ip.ip_off) & (IP_MF | IP_OFFMASK) != 0 {
                return;
            }
            (m, hoff as usize, off, ip.ip_p)
        }

        ETHERTYPE_IPV6 => {
            let Some((m, hoff)) = m_getptr(m0, hlen as i32) else {
                return;
            };
            let seg = m.data().get(hoff as usize..).unwrap_or(&[]);
            let Some(ip6) = hal::pod::at::<Ip6Hdr>(seg, 0) else {
                return;
            };
            ext.ip6 = Some(hlen as u32);

            let off = hlen;
            hlen = size_of::<Ip6Hdr>();
            let iplen = hlen + usize::from(u16::from_be(ip6.ip6_plen));
            if (ext.paylen as usize) < hlen || (ext.paylen as usize) < iplen {
                ext.ip6 = None;
                return;
            }
            ext.iplen = iplen as u32;
            ext.iphlen = hlen as u32;
            ext.paylen -= hlen as u32;
            (m, hoff as usize, off, ip6.ip6_nxt)
        }

        _ => return,
    };

    let toff = (off + hlen) as u32;
    match i32::from(ipproto) {
        IPPROTO_TCP => {
            let Some((m, hoff)) = m_getptr(m, (hoff + hlen) as i32) else {
                return;
            };
            let seg = m.data().get(hoff as usize..).unwrap_or(&[]);
            let Some(th) = hal::pod::at::<Tcphdr>(seg, 0) else {
                return;
            };
            ext.tcp = Some(toff);

            let thlen = usize::from(th.th_off()) << 2;
            if seg.len() < thlen || ((ext.iplen - ext.iphlen) as usize) < thlen {
                ext.tcp = None;
                return;
            }
            ext.tcphlen = thlen as u32;
            ext.paylen -= thlen as u32;
        }

        IPPROTO_UDP => {
            let Some((m, hoff)) = m_getptr(m, (hoff + hlen) as i32) else {
                return;
            };
            if m.data().len().saturating_sub(hoff as usize) < size_of::<Udphdr>() {
                return;
            }
            ext.udp = Some(toff);

            if ((ext.iplen - ext.iphlen) as usize) < size_of::<Udphdr>() {
                ext.udp = None;
            }
        }

        _ => {}
    }
}

/// `ether_offload_ifcap(ifp, m)`: the checksums (and VLAN tag) the interface cannot offload
/// done in software; `None` when the packet was lost.
pub fn ether_offload_ifcap(ifp: &'static Ifnet, m: MbufBox) -> Option<MbufBox> {
    let mut m = m;
    let mut ext = EtherExtracted::new();
    let caps = ifp.if_capabilities.get();
    let csum_flags = m.m_pkthdr().csum_flags.get();
    let mut csum = false;

    // NVLAN > 0: an M_VLANTAG packet on an interface without IFCAP_VLAN_HWTAGGING gets its tag
    // injected (vlan_inject); vlan(4) is not configured.

    if csum_flags & M_IPV4_CSUM_OUT != 0 && caps & IFCAP_CSUM_IPv4 == 0 {
        csum = true;
    }

    if csum_flags & M_TCP_CSUM_OUT != 0
        && (caps & IFCAP_CSUM_TCPv4 == 0 || caps & IFCAP_CSUM_TCPv6 == 0)
    {
        csum = true;
    }

    if csum_flags & M_UDP_CSUM_OUT != 0
        && (caps & IFCAP_CSUM_UDPv4 == 0 || caps & IFCAP_CSUM_UDPv6 == 0)
    {
        csum = true;
    }

    if csum {
        ether_extract_headers(&m, &mut ext);

        let ethlen = if ext.evh.is_some() {
            size_of::<EtherVlanHeader>()
        } else {
            ETHER_HDR_LEN
        };

        let hlen = (m.m_pkthdr().len.get() as u32).wrapping_sub(ext.paylen);

        if m.m_len().get() < hlen {
            m = m_pullup(m, hlen as i32)?;
        }

        // hide ethernet header
        shift_data(&m, ethlen, true);

        if ext.ip4.is_some() {
            crate::netinet::ip_output::in_hdr_cksum_out(&mut m, Some(ifp));
            crate::netinet::ip_output::in_proto_cksum_out(&mut m, Some(ifp));
        } else if ext.ip6.is_some()
            && let Some(i) = inet()
        {
            i.in6_proto_cksum_out(&m, ifp);
        }

        // show ethernet header again
        shift_data(&m, ethlen, false);
    }

    Some(m)
}

/// `m->m_data += n; m->m_len -= n; m->m_pkthdr.len -= n` (`hide`), or the reverse.
fn shift_data(m: &Mbuf, n: usize, hide: bool) {
    let ph = m.m_pkthdr();
    if hide {
        m.set_data_off(m.data_off() + n);
        m.m_len().set(m.m_len().get() - n as u32);
        ph.len.set(ph.len.get() - n as i32);
    } else {
        m.set_data_off(m.data_off() - n);
        m.m_len().set(m.m_len().get() + n as u32);
        ph.len.set(ph.len.get() + n as i32);
    }
}

// NAF_FRAME > 0: the frame sockets (struct ether_pcb, ether_pcb_group, ether_frm_usrreqs,
// ether_frm_attach/detach/bind/connect/disconnect/shutdown/send/sockaddr/peeraddr,
// ether_frm_group, the FRAME_* socket options, ether_frm_ctloutput, ether_frm_recv and
// ether_frm_input) are not configured.

const _: () = {
    assert!(CRCTAB_LE[1] == 0x1db7_1064);
    assert!(CRCTAB_BE[1] == 0x04c1_1db7);
    assert!(ETHER_IOCTL_MULTI_LEN == 44);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use hal::sys::net_if::{IFF_BROADCAST, IFNAMSIZ, IFXF_MPSAFE};

    use super::*;
    use crate::net::if_::{TestDriver, if_attach};
    use crate::net::if_var::Netstack;
    use crate::net::mbuf::{m_devget, ml_len};
    use crate::net::route::tests::{sin, sin6, testroutes_setup};

    const OURS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    const PEER: [u8; 6] = [0x52, 0x54, 0x00, 0xaa, 0xbb, 0xcc];

    /// The routing tests' fixture: real memory, table 0, the `Inet` double (ARP and IPv4
    /// count in `TEST_PROTO_INPUTS`, 10.0.2.2 resolves).
    fn setup() -> MutexGuard<'static, ()> {
        testroutes_setup()
    }

    /// A zeroed `struct arpcom` with a test driver, attached and `ether_ifattach`ed under the
    /// kernel lock (bpfattach asserts it), up and running.
    fn test_arpcom(name: &[u8]) -> &'static Arpcom {
        let ac: &'static Arpcom = Box::leak(Box::new(Arpcom::new()));
        let d: &'static TestDriver = Box::leak(Box::new(TestDriver::new()));
        let ifp = &ac.ac_if;
        let mut n = [0u8; IFNAMSIZ];
        n[..name.len()].copy_from_slice(name);
        ifp.if_xname.set(n);
        assert!(ifp.if_driver.set(d).is_ok());
        ifp.if_xflags.set(IFXF_MPSAFE);
        ifp.if_flags
            .set(IFF_BROADCAST | IFF_SIMPLEX | IFF_MULTICAST | IFF_RUNNING);
        ac.ac_enaddr.set(OURS);
        hal::klock::kernel_lock();
        if_attach(ifp);
        ether_ifattach(ac);
        hal::klock::kernel_unlock();
        ac
    }

    /// A frame `dst` <- `src` of type `etype` with `payload`, in an mbuf.
    fn frame(dst: [u8; 6], src: [u8; 6], etype: u16, payload: &[u8]) -> MbufBox {
        let mut f = Vec::new();
        f.extend_from_slice(&dst);
        f.extend_from_slice(&src);
        f.extend_from_slice(&etype.to_be_bytes());
        f.extend_from_slice(payload);
        m_devget(&f, 0).expect("an mbuf")
    }

    #[test]
    fn sprintf_and_the_address_word() {
        let s = ether_sprintf(&[0x00, 0x1b, 0x21, 0xab, 0xcd, 0xef]);
        assert_eq!(&s[..17], b"00:1b:21:ab:cd:ef");
        assert_eq!(s[17], 0);

        let e64 = ether_addr_to_e64(&OURS);
        assert_eq!(e64, 0x5254_0012_3456, "the first octet highest");
        let mut back = [0u8; 6];
        ether_e64_to_addr(&mut back, e64);
        assert_eq!(back, OURS);
        assert!(eth64_is_broadcast(ether_addr_to_e64(&ETHERBROADCASTADDR)));
        assert_eq!(ether_addr_to_e64(&ETHERANYADDR), 0);

        let cell = EtherAddrCell::new();
        assert_eq!(cell.get(), [0; 6]);
        cell.set(PEER);
        assert_eq!((cell.get(), cell.e64()), (PEER, ether_addr_to_e64(&PEER)));
    }

    /// The C's `#if 0` reference versions, bit by bit.
    fn crc32_le_bitwise(mut crc: u32, buf: &[u8]) -> u32 {
        for &b in buf {
            let mut c = u32::from(b);
            for _ in 0..8 {
                let carry = (crc & 1) ^ (c & 1);
                crc >>= 1;
                c >>= 1;
                if carry != 0 {
                    crc ^= CRC_POLY_LE;
                }
            }
        }
        crc
    }

    fn crc32_be_bitwise(mut crc: u32, buf: &[u8]) -> u32 {
        for &b in buf {
            let mut c = u32::from(b);
            for _ in 0..8 {
                let carry = u32::from(crc & 0x8000_0000 != 0) ^ (c & 1);
                crc <<= 1;
                c >>= 1;
                if carry != 0 {
                    crc = (crc ^ CRC_POLY_BE) | carry;
                }
            }
        }
        crc
    }

    #[test]
    fn the_crc_tables_and_values_are_the_c_s() {
        assert_eq!(
            CRCTAB_LE,
            [
                0x00000000, 0x1db71064, 0x3b6e20c8, 0x26d930ac, 0x76dc4190, 0x6b6b51f4, 0x4db26158,
                0x5005713c, 0xedb88320, 0xf00f9344, 0xd6d6a3e8, 0xcb61b38c, 0x9b64c2b0, 0x86d3d2d4,
                0xa00ae278, 0xbdbdf21c,
            ]
        );
        assert_eq!(
            CRCTAB_BE,
            [
                0x00000000, 0x04c11db7, 0x09823b6e, 0x0d4326d9, 0x130476dc, 0x17c56b6b, 0x1a864db2,
                0x1e475005, 0x2608edb8, 0x22c9f00f, 0x2f8ad6d6, 0x2b4bcb61, 0x350c9b64, 0x31cd86d3,
                0x3c8ea00a, 0x384fbdbd,
            ]
        );
        assert_eq!(
            REV,
            [
                0x0, 0x8, 0x4, 0xc, 0x2, 0xa, 0x6, 0xe, 0x1, 0x9, 0x5, 0xd, 0x3, 0xb, 0x7, 0xf
            ]
        );
        // CRC-32 of "123456789" is 0xcbf43926 after the final inversion, which
        // ether_crc32_le leaves to its callers.
        assert_eq!(ether_crc32_le(b"123456789"), !0xcbf4_3926);
        for buf in [&b""[..], b"a", b"123456789", &OURS, &[0xffu8; 64]] {
            assert_eq!(ether_crc32_le(buf), crc32_le_bitwise(0xffff_ffff, buf));
            assert_eq!(ether_crc32_be(buf), crc32_be_bitwise(0xffff_ffff, buf));
        }
    }

    #[test]
    fn ether_ifattach_makes_an_ethernet_interface() {
        let _g = setup();
        let ac = test_arpcom(b"tet0");
        let ifp = &ac.ac_if;
        assert_eq!(ifp.if_type.get(), IFT_ETHER);
        assert_eq!(usize::from(ifp.if_addrlen.get()), ETHER_ADDR_LEN);
        assert_eq!(usize::from(ifp.if_hdrlen.get()), ETHER_HDR_LEN);
        assert_eq!(ifp.if_mtu.get() as usize, ETHERMTU);
        assert_eq!(ifp.if_hardmtu.get() as usize, ETHERMTU);
        assert!(core::ptr::eq(arpcom_of(ifp), ac));
        assert!(ifp.if_link.get().is_some());
        {
            let sdl = ifp.sadl();
            let sdl = sdl.as_ref().expect("a link name");
            let start = usize::from(sdl.sdl_nlen);
            assert_eq!(
                sdl.sdl_data[start..start + 6],
                OURS,
                "LLADDR holds ac_enaddr"
            );
        }
        assert_eq!(ac.multiaddrs().iter().count(), 0);
        // A multicast hardware address is replaced by a locally administered one.
        let bad: &'static Arpcom = Box::leak(Box::new(Arpcom::new()));
        bad.ac_enaddr.set([0x01, 0, 0x5e, 0, 0, 1]);
        bad.ac_if.if_arpcom.set(Some(bad));
        ether_fakeaddr(&bad.ac_if);
        let a = bad.ac_enaddr.get();
        assert_eq!((a[0], a[1], a[2], a[3] & 0xf0), (0xfe, 0xe1, 0xba, 0xd0));
        assert!(!ether_is_multicast(&a));
    }

    #[test]
    fn ether_input_hands_each_type_to_its_protocol_and_drops_the_rest() {
        let _g = setup();
        let ac = test_arpcom(b"tet1");
        let ifp = &ac.ac_if;
        let mut ns = Netstack::new(0);
        // A frame for a protocol is queued on the softnet's stack (`if_input_proto`), with
        // the protocol in its cookie.
        let count = |f: MbufBox, ns: &mut Netstack| {
            ether_input(ifp, f, Some(ns));
            let n = ml_len(&ns.ns_proto);
            while let Some(m) = crate::net::mbuf::ml_dequeue(&mut ns.ns_proto) {
                assert_eq!(
                    m.m_pkthdr().ph_cookie.get(),
                    crate::net::if_::proto_fn_id(crate::net::if_::test_proto_count)
                );
                assert_eq!(m.m_pkthdr().len.get(), 46, "the Ethernet header trimmed");
            }
            n
        };
        let body = [0u8; 46];
        // IPv4 and ARP to the double's protocols; IPv6 likewise; RARP too.
        assert_eq!(count(frame(OURS, PEER, ETHERTYPE_IP, &body), &mut ns), 1);
        assert_eq!(count(frame(OURS, PEER, ETHERTYPE_ARP, &body), &mut ns), 1);
        assert_eq!(
            count(frame(OURS, PEER, ETHERTYPE_REVARP, &body), &mut ns),
            1
        );
        assert_eq!(count(frame(OURS, PEER, ETHERTYPE_IPV6, &body), &mut ns), 1);
        // An unknown type, a tagged frame, another host's unicast: dropped.
        assert_eq!(count(frame(OURS, PEER, 0x88b5, &body), &mut ns), 0);
        assert_eq!(count(frame(OURS, PEER, ETHERTYPE_VLAN, &body), &mut ns), 0);
        assert_eq!(count(frame(PEER, OURS, ETHERTYPE_IP, &body), &mut ns), 0);
        // A short frame.
        let short = m_devget(&OURS, 0).expect("an mbuf");
        assert_eq!(count(short, &mut ns), 0);
        // IFF_NOARP drops ARP and RARP, not IP.
        ifp.if_flags.set(ifp.if_flags.get() | IFF_NOARP);
        assert_eq!(count(frame(OURS, PEER, ETHERTYPE_ARP, &body), &mut ns), 0);
        assert_eq!(
            count(frame(OURS, PEER, ETHERTYPE_REVARP, &body), &mut ns),
            0
        );
        assert_eq!(count(frame(OURS, PEER, ETHERTYPE_IP, &body), &mut ns), 1);
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_NOARP);
        // Broadcast and multicast are counted and go up; our own echo is dropped on a
        // non-simplex interface only.
        let imcasts = ifp.if_imcasts().get();
        assert_eq!(
            count(
                frame(ETHERBROADCASTADDR, PEER, ETHERTYPE_ARP, &body),
                &mut ns
            ),
            1
        );
        assert_eq!(
            count(
                frame([0x01, 0, 0x5e, 0, 0, 1], OURS, ETHERTYPE_IP, &body),
                &mut ns
            ),
            1,
            "simplex: our own multicast goes up"
        );
        assert_eq!(ifp.if_imcasts().get(), imcasts + 2);
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_SIMPLEX);
        assert_eq!(
            count(
                frame([0x01, 0, 0x5e, 0, 0, 1], OURS, ETHERTYPE_IP, &body),
                &mut ns
            ),
            0
        );
    }

    /// A multicast request's bytes: the name, then the socket address at `ifr_addr`.
    fn mreq(sa: &[u8]) -> Vec<u8> {
        let mut r = std::vec![0u8; ETHER_IOCTL_MULTI_LEN];
        r[IFNAMSIZ..IFNAMSIZ + sa.len()].copy_from_slice(sa);
        r
    }

    #[test]
    fn ether_multiaddr_maps_each_family() {
        let mut unspec = [0u8; 16];
        unspec[0] = 16;
        unspec[2..8].copy_from_slice(&[0x01, 0x80, 0xc2, 0, 0, 0x0e]);
        let cases: [(&[u8], Result<([u8; 6], [u8; 6]), Errno>); 6] = [
            (
                &unspec,
                Ok((
                    [0x01, 0x80, 0xc2, 0, 0, 0x0e],
                    [0x01, 0x80, 0xc2, 0, 0, 0x0e],
                )),
            ),
            (
                &sin([224, 0, 0, 251]),
                Ok(([0x01, 0, 0x5e, 0, 0, 251], [0x01, 0, 0x5e, 0, 0, 251])),
            ),
            (
                &sin([239, 255, 1, 2]),
                Ok(([0x01, 0, 0x5e, 0x7f, 1, 2], [0x01, 0, 0x5e, 0x7f, 1, 2])),
            ),
            (
                &sin([0, 0, 0, 0]),
                Ok((ETHER_IPMULTICAST_MIN, ETHER_IPMULTICAST_MAX)),
            ),
            (
                &sin6([
                    0xff, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0xff, 0x12, 0x34, 0x56,
                ]),
                Ok((
                    [0x33, 0x33, 0xff, 0x12, 0x34, 0x56],
                    [0x33, 0x33, 0xff, 0x12, 0x34, 0x56],
                )),
            ),
            (
                &sin6([0; 16]),
                Ok((ETHER_IP6MULTICAST_MIN, ETHER_IP6MULTICAST_MAX)),
            ),
        ];
        for (sa, want) in cases {
            assert_eq!(ether_multiaddr(sa), want, "{sa:?}");
        }
        let mut link = [0u8; 20];
        link[0] = 20;
        link[1] = hal::sys::socket::AF_LINK;
        assert_eq!(ether_multiaddr(&link), Err(Errno::EAFNOSUPPORT));
    }

    #[test]
    fn the_multicast_list_counts_claims_and_tells_the_driver_of_changes() {
        let _g = setup();
        let ac = test_arpcom(b"tet2");
        let ifp = &ac.ac_if;
        let g1 = mreq(&sin([224, 0, 0, 1]));
        let g2 = mreq(&sin([224, 0, 0, 2]));
        let all = mreq(&sin([0, 0, 0, 0]));
        assert_eq!(
            ether_ioctl(ifp, ac, SIOCADDMULTI, &g1),
            Err(Errno::ENETRESET)
        );
        assert_eq!(
            ether_ioctl(ifp, ac, SIOCADDMULTI, &g1),
            Ok(()),
            "a second claim"
        );
        assert_eq!(
            ether_ioctl(ifp, ac, SIOCADDMULTI, &g2),
            Err(Errno::ENETRESET)
        );
        assert_eq!(
            ether_ioctl(ifp, ac, SIOCADDMULTI, &all),
            Err(Errno::ENETRESET)
        );
        assert_eq!((ac.ac_multicnt.get(), ac.ac_multirangecnt.get()), (3, 1));
        {
            let g = ac.multiaddrs();
            let order: Vec<u8> = g.iter().map(|e| e.enm_addrlo[5]).collect();
            assert_eq!(order, [0, 2, 1], "newest first, as LIST_INSERT_HEAD");
            let claims: Vec<u32> = g.iter().map(|e| e.enm_refcnt).collect();
            assert_eq!(claims, [1, 1, 2]);
        }
        // A unicast address is refused, a range not listed is not found.
        let mut uni = [0u8; 16];
        uni[0] = 16;
        uni[2..8].copy_from_slice(&PEER);
        assert_eq!(ether_addmulti(&mreq(&uni), ac), Err(Errno::EINVAL));
        assert_eq!(
            ether_delmulti(&mreq(&sin([224, 0, 0, 9])), ac),
            Err(Errno::ENXIO)
        );
        // The last claim goes with the record.
        assert_eq!(ether_ioctl(ifp, ac, SIOCDELMULTI, &g1), Ok(()));
        assert_eq!(
            ether_ioctl(ifp, ac, SIOCDELMULTI, &g1),
            Err(Errno::ENETRESET)
        );
        assert_eq!(
            ether_ioctl(ifp, ac, SIOCDELMULTI, &all),
            Err(Errno::ENETRESET)
        );
        assert_eq!((ac.ac_multicnt.get(), ac.ac_multirangecnt.get()), (1, 0));
        assert_eq!(ether_ioctl(ifp, ac, SIOCDELMULTI, &g1), Err(Errno::ENXIO));
        // Without IFF_MULTICAST the requests are the driver's.
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_MULTICAST);
        assert_eq!(ether_ioctl(ifp, ac, SIOCADDMULTI, &g1), Err(Errno::ENOTTY));
        ifp.if_flags.set(ifp.if_flags.get() | IFF_MULTICAST);
        // SIOCSIFMTU within [ETHERMIN, if_hardmtu].
        let mut ifr = Ifreq::default();
        for (mtu, want) in [
            (1400, Ok(())),
            (ETHERMIN as i32 - 1, Err(Errno::EINVAL)),
            (1501, Err(Errno::EINVAL)),
        ] {
            ifr.set_ifr_mtu(mtu);
            let bytes = hal::user::bytes_of(core::slice::from_ref(&ifr));
            assert_eq!(ether_ioctl(ifp, ac, SIOCSIFMTU, bytes), want);
        }
        assert_eq!(ifp.if_mtu.get(), 1400);
        assert_eq!(
            ether_ioctl(ifp, ac, hal::sys::sockio::SIOCGIFMEDIA, &[]),
            Err(Errno::ENOTTY)
        );
        ether_ifdetach_records(ac);
        assert_eq!(ac.multiaddrs().iter().count(), 0);
    }

    /// `ether_ifdetach`'s loop over the records (its `if_deactivate` needs the interface's
    /// hooks, which these tests do not run).
    fn ether_ifdetach_records(ac: &Arpcom) {
        ac.multiaddrs().purge();
    }

    #[test]
    fn ether_resolve_takes_the_header_from_a_link_sockaddr_or_from_arp() {
        let _g = setup();
        let ac = test_arpcom(b"tet3");
        let ifp: &'static Ifnet = &ac.ac_if;
        let pkt = || m_devget(&[0u8; 28], 0).expect("an mbuf");

        // AF_UNSPEC: destination and type from sa_data, our source.
        let mut sa = [0u8; 16];
        sa[0] = 16;
        sa[1] = AF_UNSPEC;
        sa[2..8].copy_from_slice(&PEER);
        sa[14..16].copy_from_slice(&ETHERTYPE_ARP.to_be_bytes());
        let mut eh = EtherHeader::default();
        let dst = SockaddrView::new(&sa).expect("an address");
        assert!(ether_resolve(ifp, pkt(), dst, None, &mut eh).is_ok());
        assert_eq!(
            (eh.ether_dhost, eh.ether_shost, u16::from_be(eh.ether_type)),
            (PEER, OURS, ETHERTYPE_ARP)
        );

        // pseudo_AF_HDRCMPLT: the whole header from sa_data.
        sa[1] = pseudo_AF_HDRCMPLT;
        sa[8..14].copy_from_slice(&PEER);
        let dst = SockaddrView::new(&sa).expect("an address");
        let mut eh = EtherHeader::default();
        assert!(ether_resolve(ifp, pkt(), dst, None, &mut eh).is_ok());
        assert_eq!((eh.ether_dhost, eh.ether_shost), (PEER, PEER));

        // AF_INET through ARP: resolved, held, unreachable.
        let rt = crate::net::route::tests::entry(&sin([10, 0, 2, 0]), 24, None, 0, 8);
        let a = sin([10, 0, 2, 2]);
        let dst = SockaddrView::new(&a).expect("an address");
        let mut eh = EtherHeader::default();
        assert!(ether_resolve(ifp, pkt(), dst, Some(&rt), &mut eh).is_ok());
        assert_eq!(
            (eh.ether_dhost, eh.ether_shost, u16::from_be(eh.ether_type)),
            ([2, 0, 0, 0, 0, 2], OURS, ETHERTYPE_IP)
        );
        let held = sin([10, 0, 2, 99]);
        let dst = SockaddrView::new(&held).expect("an address");
        assert!(matches!(ether_encap(ifp, pkt(), dst, Some(&rt)), Ok(None)));
        let far = sin([10, 0, 2, 7]);
        let dst = SockaddrView::new(&far).expect("an address");
        assert!(matches!(
            ether_encap(ifp, pkt(), dst, Some(&rt)),
            Err(Errno::EHOSTUNREACH)
        ));

        // Another family, and an interface that is not running.
        let mut link = [0u8; 20];
        link[0] = 20;
        link[1] = hal::sys::socket::AF_LINK;
        let dst = SockaddrView::new(&link).expect("an address");
        assert!(matches!(
            ether_resolve(ifp, pkt(), dst, Some(&rt), &mut EtherHeader::default()),
            Err(Errno::EAFNOSUPPORT)
        ));
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_RUNNING);
        let dst = SockaddrView::new(&a).expect("an address");
        assert!(matches!(
            ether_resolve(ifp, pkt(), dst, Some(&rt), &mut EtherHeader::default()),
            Err(Errno::ENETDOWN)
        ));
    }

    #[test]
    fn ether_output_prepends_the_header_and_queues_the_frame() {
        let _g = setup();
        let ac = test_arpcom(b"tet4");
        let ifp: &'static Ifnet = &ac.ac_if;
        let mut sa = [0u8; 16];
        sa[0] = 16;
        sa[1] = AF_UNSPEC;
        sa[2..8].copy_from_slice(&PEER);
        sa[14..16].copy_from_slice(&ETHERTYPE_IP.to_be_bytes());
        let dst = SockaddrView::new(&sa).expect("an address");
        let before = crate::net::ifq::ifq_len(ifp.if_snd());
        // Through the interface's driver: its default output is the link layer's.
        assert_eq!(
            ifp.driver()
                .output(ifp, m_devget(&[7u8; 20], 0).expect("an mbuf"), dst, None),
            Ok(())
        );
        assert_eq!(crate::net::ifq::ifq_len(ifp.if_snd()), before + 1);
        let m = crate::net::ifq::ifq_dequeue(ifp.if_snd()).expect("the frame");
        let eh = hal::pod::at::<EtherHeader>(m.data(), 0).expect("a header");
        assert_eq!((eh.ether_dhost, eh.ether_shost), (PEER, OURS));
        assert_eq!(m.m_pkthdr().len.get(), 34);
        let mut first = [0u8; 1];
        crate::net::mbuf::m_copydata(&m, ETHER_HDR_LEN as i32, &mut first);
        assert_eq!(first, [7], "the payload after the header");
    }

    #[test]
    fn ether_extract_headers_finds_an_ipv4_udp_frame_s_headers() {
        let _g = setup();
        // IPv4, 20-byte header, total length 28, UDP; then the UDP header.
        let mut p = std::vec![
            0x45, 0, 0, 28, 0, 0, 0, 0, 64, 17, 0, 0, 10, 0, 2, 2, 10, 0, 2, 15
        ];
        p.extend_from_slice(&[0x12, 0x34, 0x00, 0x35, 0x00, 0x08, 0x00, 0x00]);
        let m = frame(OURS, PEER, ETHERTYPE_IP, &p);
        let mut ext = EtherExtracted::new();
        ether_extract_headers(&m, &mut ext);
        assert_eq!(
            (ext.eh, ext.ip4, ext.udp, ext.tcp, ext.ip6),
            (Some(0), Some(14), Some(34), None, None)
        );
        assert_eq!((ext.iplen, ext.iphlen, ext.paylen), (28, 20, 8));

        // A fragment: the transport header is not looked at.
        p[6] = 0x20;
        let m = frame(OURS, PEER, ETHERTYPE_IP, &p);
        ether_extract_headers(&m, &mut ext);
        assert_eq!((ext.ip4, ext.udp), (Some(14), None));
        // A header longer than the packet: not an IPv4 header.
        p[6] = 0;
        p[0] = 0x4f;
        let m = frame(OURS, PEER, ETHERTYPE_IP, &p);
        ether_extract_headers(&m, &mut ext);
        assert_eq!((ext.eh, ext.ip4), (Some(0), None));
    }

    #[test]
    fn ether_extract_headers_finds_an_ipv6_tcp_frame_s_headers() {
        let _g = setup();
        // IPv6, payload length 24, TCP, fe80::1 -> fe80::2; TCP with data offset 5 and four
        // bytes of payload.
        let mut p = std::vec![0x60, 0, 0, 0, 0, 24, 6, 64];
        p.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        p.extend_from_slice(&[0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        p.extend_from_slice(&[
            0, 22, 0, 80, 0, 0, 0, 1, 0, 0, 0, 0, 0x50, 0x18, 0x40, 0, 0, 0, 0, 0,
        ]);
        p.extend_from_slice(&[1, 2, 3, 4]);
        let m = frame(OURS, PEER, ETHERTYPE_IPV6, &p);
        let mut ext = EtherExtracted::new();
        ether_extract_headers(&m, &mut ext);
        assert_eq!(
            (ext.ip6, ext.ip4, ext.tcp, ext.udp),
            (Some(14), None, Some(54), None)
        );
        assert_eq!(
            (ext.iplen, ext.iphlen, ext.tcphlen, ext.paylen),
            (64, 40, 20, 4)
        );
        // A data offset past the segment: no TCP header.
        p[40 + 12] = 0xf0;
        let m = frame(OURS, PEER, ETHERTYPE_IPV6, &p);
        ether_extract_headers(&m, &mut ext);
        assert_eq!((ext.ip6, ext.tcp), (Some(14), None));
    }

    /// A bridge port that takes every frame and hands back another of an unknown type (a
    /// port that rewrites the frame).
    struct SwapPort;

    impl EtherPort for SwapPort {
        fn input(
            &self,
            _ifp: &'static Ifnet,
            m: MbufBox,
            _dst: u64,
            _ns: Option<&mut Netstack>,
        ) -> Option<MbufBox> {
            drop(m);
            Some(frame(OURS, PEER, 0x88b5, &[0u8; 46]))
        }

        fn take(&self) {}

        fn rele(&self) {}
    }

    static SWAP_PORT: SwapPort = SwapPort;
    static SWAP_SLOT: EtherPortSlot = EtherPortSlot { port: &SWAP_PORT };

    #[test]
    fn ether_input_reads_the_header_again_after_a_port_replaced_the_frame() {
        let _g = setup();
        let ac = test_arpcom(b"tet5");
        let ifp = &ac.ac_if;
        let mut ns = Netstack::new(0);
        hal::klock::kernel_lock();
        assert_eq!(ether_brport_isset(ifp), Ok(()));
        ether_brport_set(ifp, &SWAP_SLOT);
        assert_eq!(ether_brport_isset(ifp), Err(Errno::EBUSY));
        assert!(ether_brport_get_locked(ifp).is_some());
        hal::klock::kernel_unlock();
        // An IPv4 frame the port replaces by one of an unknown type: dropped, as the C's
        // second read of the header finds (the first read's type would have queued it).
        ether_input(
            ifp,
            frame(OURS, PEER, ETHERTYPE_IP, &[0u8; 46]),
            Some(&mut ns),
        );
        assert_eq!(ml_len(&ns.ns_proto), 0);
        hal::klock::kernel_lock();
        ether_brport_clr(ifp);
        hal::klock::kernel_unlock();
        ether_input(
            ifp,
            frame(OURS, PEER, ETHERTYPE_IP, &[0u8; 46]),
            Some(&mut ns),
        );
        assert_eq!(ml_len(&ns.ns_proto), 1);
        while let Some(m) = crate::net::mbuf::ml_dequeue(&mut ns.ns_proto) {
            drop(m);
        }
    }
}
/* </TESTS> */
