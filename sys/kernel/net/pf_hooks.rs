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
//! The `Pf` bridge: what the mbuf asks of pf(4), which has not moved yet (N8), at the C's four
//! points (`m_clearhdr`, `m_free`, `m_dup_pkthdr`: uipc_mbuf.c's `NPF > 0` blocks).
//!
//! A packet header carries two references of pf's, the state key it matched and the pcb of an
//! outgoing packet (`pkthdr_pf.statekey`, `.inp`): words the mbuf keeps opaque, which only pf
//! links and unlinks. `bsd`'s pf registers its implementation at boot ([`register_pf`],
//! bridges.toml's `Pf` row, paid by N8 when pf moves); before that every leg is a no-op, the
//! C compiled with `NPF == 0`. N6.3c adds the stack's other legs to the same trait.
//!
//! `if.c`'s legs (N6.3a, the `NPF > 0` blocks of `if_enqueue`, `if_vinput`, `if_attachsetup`,
//! `if_detach` and the groups' functions) are default methods doing what the C without pf
//! does, so that `bsd`'s implementation overrides them one by one.
//!
//! N6.4a adds `pf_test` for `ip_output` (`kernel::netinet::ip_output`, ip_output.c's
//! `NPF > 0` block), a default that passes every packet, the C with `NPF == 0`.

use core::sync::atomic::{AtomicUsize, Ordering};

use hal::bridges::AlreadyRegistered;
use hal::once::OnceStatic;

use hal::sys::errno::Errno;
use hal::sys::types::SaFamily;

use crate::assert::kpanic;
use crate::net::if_var::{IfgGroup, Ifnet};
use crate::net::mbuf::{Mbuf, MbufBox, PkthdrPf};

/// `PF_IN`: the packet comes in (`pfvar.h`'s direction).
pub const PF_IN: u8 = 1;
/// `PF_OUT`: the packet goes out.
pub const PF_OUT: u8 = 2;
/// `PF_FWD`: the packet is forwarded.
pub const PF_FWD: u8 = 3;
/// `PF_PASS`: pf's verdict that lets the packet go on (`pfvar.h`'s action).
pub const PF_PASS: u8 = 0;

/// The legs of pf the mbuf calls (`net/pf.c`'s `pf_mbuf_*`), over the header's pf part.
pub trait Pf: Sync {
    /// `pf_mbuf_link_state_key(to, from->m_pkthdr.pf.statekey)`: `to` takes a reference on
    /// `from`'s state key, if it has one (`to`'s word is 0).
    fn mbuf_link_state_key(&self, to: &PkthdrPf, from: &PkthdrPf);
    /// `pf_mbuf_link_inpcb(to, from->m_pkthdr.pf.inp)`: `to` takes a reference on `from`'s
    /// pcb, if it has one (`to`'s word is 0).
    fn mbuf_link_inpcb(&self, to: &PkthdrPf, from: &PkthdrPf);
    /// `pf_mbuf_unlink_state_key(m)`: the header's state key reference dropped, its word 0.
    fn mbuf_unlink_state_key(&self, pf: &PkthdrPf);
    /// `pf_mbuf_unlink_inpcb(m)`: the header's pcb reference dropped, its word 0.
    fn mbuf_unlink_inpcb(&self, pf: &PkthdrPf);

    /// `pf_delay_pkt(m, ifidx)`: a packet whose header asks for a delay (`pf.delay > 0`,
    /// which only pf's rules set) held back by pf before it is sent on. Without pf no packet
    /// asks for one: the default is `panic(9)`.
    fn delay_pkt(&self, m: MbufBox, ifidx: u32) -> Result<(), Errno> {
        drop(m);
        kpanic(format_args!("pf_delay_pkt({ifidx}): no pf"))
    }
    /// `pf_pkt_addr_changed(m)`: the packet's addresses changed (its state key forgotten).
    fn pkt_addr_changed(&self, _m: &Mbuf) {}
    /// `pfi_attach_ifnet(ifp)`.
    fn attach_ifnet(&self, _ifp: &'static Ifnet) {}
    /// `pfi_detach_ifnet(ifp)`.
    fn detach_ifnet(&self, _ifp: &'static Ifnet) {}
    /// `pfi_attach_ifgroup(ifg)`: pf's kif of a new group (`ifg_pf_kif`).
    fn attach_ifgroup(&self, _ifg: &IfgGroup) {}
    /// `pfi_detach_ifgroup(ifg)`: the group's kif let go, before the group goes.
    fn detach_ifgroup(&self, _ifg: &IfgGroup) {}
    /// `pfi_group_addmember(name)`: an interface joined the group `name`.
    fn group_addmember(&self, _name: &[u8]) {}
    /// `pfi_group_delmember(name)`: an interface left the group `name`.
    fn group_delmember(&self, _name: &[u8]) {}

    /// `pf_test(af, dir, ifp, &m)`: pf's verdict on the packet `m` of family `af` going in
    /// direction `dir` (`PF_IN`, `PF_OUT`, `PF_FWD`) on `ifp`, and the packet as pf left it:
    /// `None` when pf took it (dropped, diverted, queued, or a `PF_PASS` it consumed).
    /// Default: `PF_PASS` and the packet untouched, the C without pf.
    fn test(
        &self,
        _af: SaFamily,
        _dir: u8,
        _ifp: &'static Ifnet,
        m: MbufBox,
    ) -> (u8, Option<MbufBox>) {
        (PF_PASS, Some(m))
    }

    /// `pf_find_divert(m)`: the divert-to or divert-reply target pf recorded on a packet it
    /// tagged `PF_TAG_DIVERTED` (`in_pcblookup_listen`'s and `in6_pcblookup_listen`'s key).
    /// Default: none, the C without pf (no packet carries the tag).
    fn find_divert(&self, _m: &Mbuf) -> Option<PfDivert> {
        None
    }

    /// `pf_remove_divert_state(inp)`: the socket whose pf word is `slot` closes; a
    /// divert-to or divert-reply state pf linked to it goes (`in_pcbdetach`,
    /// `in_pcbdisconnect`). Default: nothing, the C without pf.
    fn inp_remove_divert_state(&self, _slot: &PfSlot) {}

    /// `pf_inp_unlink(inp)`: the state key linked to the socket whose pf word is `slot` is
    /// unlinked from it, and its reference to the pcb dropped. Default: nothing.
    fn inp_unlink(&self, _slot: &PfSlot) {}
}

/// `PF_DIVERT_TO`: `pfvar.h`'s divert type.
pub const PF_DIVERT_TO: u8 = 1;
/// `PF_DIVERT_REPLY`.
pub const PF_DIVERT_REPLY: u8 = 2;

/// `struct pf_divert` as the pcb lookups read it: the address (an IPv4 address in its first
/// four bytes), the port (network order), the routing domain and the type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PfDivert {
    /// `addr`.
    pub addr: [u8; 16],
    /// `port`.
    pub port: u16,
    /// `rdomain`.
    pub rdomain: u16,
    /// `type`.
    pub type_: u8,
}

/// The capability to write a [`PfSlot`]: [`register_pf`] gives its one value to the
/// registrant, once (`bsd`'s pf, which dereferences the word it writes); nothing else in the
/// kernel can name or make one (a private field, a private static). The `TdbKey` shape
/// (`kernel::net::ipsec_hooks`).
pub struct PfKey(());

/// The one [`PfKey`].
static PF_KEY: PfKey = PfKey(());

/// `inp_pf_sk`: pf's word in a protocol control block (\[L\] `pf_inp_mtx`, pf's): the
/// address of the state key the socket's packets match, which only pf writes (with its
/// [`PfKey`]) and reads, the `pkthdr_pf.statekey` shape. The kernel hands it to pf's legs and
/// never reads it.
pub struct PfSlot(AtomicUsize);

impl PfSlot {
    /// No state key.
    pub const fn new() -> Self {
        Self(AtomicUsize::new(0))
    }

    /// The word.
    #[inline]
    pub fn get(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }

    /// Stores `w`, for the holder of pf's key only (`pf_inp_link`, `pf_inp_unlink`).
    #[inline]
    pub fn set(&self, _key: &PfKey, w: usize) {
        self.0.store(w, Ordering::Release);
    }
}

impl Default for PfSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// pf's legs, once registered.
static PF: OnceStatic<&'static dyn Pf> = OnceStatic::new();

/// Registers pf's legs, once (`bsd`'s pf, at boot before the first packet), and gives the
/// registrant the [`PfKey`] that writes the pcbs' pf words; a second call keeps the first and
/// names it.
pub fn register_pf(imp: &'static dyn Pf) -> Result<&'static PfKey, AlreadyRegistered> {
    PF.set(imp)
        .map(|()| &PF_KEY)
        .map_err(|_| AlreadyRegistered("Pf"))
}

/// pf's legs, `None` before registration (`NPF == 0`: nothing to link or unlink).
#[inline]
pub(crate) fn pf() -> Option<&'static dyn Pf> {
    PF.get().copied()
}

/// The host tests' pf (the mbuf's legs and the pcb's): it links and unlinks a header's words
/// as pf's `pf_mbuf_*` legs do and counts them, and empties a pcb's word as `pf_inp_unlink`
/// does, recording what it held. One per test binary (`PF` is set once): every test that
/// needs pf registers this one.
#[cfg(test)]
pub(crate) mod testpf {
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    use std::sync::OnceLock;

    use super::{Pf, PfKey, PfSlot};
    use crate::net::mbuf::PkthdrPf;

    /// The counting pf.
    pub(crate) struct CountingPf {
        /// References a header took (`pf_mbuf_link_*`).
        pub(crate) links: AtomicU32,
        /// References a header dropped (`pf_mbuf_unlink_*`).
        pub(crate) unlinks: AtomicU32,
        /// `pf_inp_unlink` calls that found a state key.
        pub(crate) inp_unlinks: AtomicU32,
        /// `pf_remove_divert_state` calls.
        pub(crate) divert_removals: AtomicU32,
        /// The last state key word `pf_inp_unlink` took out of a pcb.
        pub(crate) last_unlinked: AtomicUsize,
    }

    impl Pf for CountingPf {
        fn mbuf_link_state_key(&self, to: &PkthdrPf, from: &PkthdrPf) {
            if from.statekey.get() != 0 {
                to.statekey.set(from.statekey.get());
                self.links.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn mbuf_link_inpcb(&self, to: &PkthdrPf, from: &PkthdrPf) {
            if from.inp.get() != 0 {
                to.inp.set(from.inp.get());
                self.links.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn mbuf_unlink_state_key(&self, pf: &PkthdrPf) {
            if pf.statekey.get() != 0 {
                pf.statekey.set(0);
                self.unlinks.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn mbuf_unlink_inpcb(&self, pf: &PkthdrPf) {
            if pf.inp.get() != 0 {
                pf.inp.set(0);
                self.unlinks.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn inp_remove_divert_state(&self, _slot: &PfSlot) {
            self.divert_removals.fetch_add(1, Ordering::Relaxed);
        }
        fn inp_unlink(&self, slot: &PfSlot) {
            let w = slot.get();
            if w != 0 {
                slot.set(key(), 0);
                self.last_unlinked.store(w, Ordering::Relaxed);
                self.inp_unlinks.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// The binary's pf.
    pub(crate) static COUNTING_PF: CountingPf = CountingPf {
        links: AtomicU32::new(0),
        unlinks: AtomicU32::new(0),
        inp_unlinks: AtomicU32::new(0),
        divert_removals: AtomicU32::new(0),
        last_unlinked: AtomicUsize::new(0),
    };

    /// The key [`register`] got.
    static KEY: OnceLock<&'static PfKey> = OnceLock::new();

    /// [`COUNTING_PF`] registered (by the first test that asks), its key kept.
    pub(crate) fn register() -> &'static CountingPf {
        KEY.get_or_init(|| match super::register_pf(&COUNTING_PF) {
            Ok(k) => k,
            Err(e) => std::panic!("the test binary's pf was registered by another: {e:?}"),
        });
        &COUNTING_PF
    }

    /// The test pf's [`PfKey`] (as pf's `pf_inp_link` writes a pcb's word).
    pub(crate) fn key() -> &'static PfKey {
        register();
        KEY.get().copied().unwrap_or_else(|| std::unreachable!())
    }
}
/* </CODE> */
