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

use hal::bridges::AlreadyRegistered;
use hal::once::OnceStatic;

use hal::sys::errno::Errno;

use crate::assert::kpanic;
use crate::net::if_var::{IfgGroup, Ifnet};
use crate::net::mbuf::{Mbuf, MbufBox, PkthdrPf};

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
}

/// pf's legs, once registered.
static PF: OnceStatic<&'static dyn Pf> = OnceStatic::new();

/// Registers pf's legs, once (`bsd`'s pf, at boot before the first packet); a second call
/// keeps the first and names it.
pub fn register_pf(imp: &'static dyn Pf) -> Result<(), AlreadyRegistered> {
    PF.set(imp).map_err(|_| AlreadyRegistered("Pf"))
}

/// pf's legs, `None` before registration (`NPF == 0`: nothing to link or unlink).
#[inline]
pub(crate) fn pf() -> Option<&'static dyn Pf> {
    PF.get().copied()
}
/* </CODE> */
