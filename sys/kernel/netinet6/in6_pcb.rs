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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */

/*
 * Copyright (c) 1982, 1986, 1990, 1993, 1995
 *	Regents of the University of California.  All rights reserved.
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
 */
/* </LICENSES> */

/* <CODE> */
//! IPv6 protocol control blocks: the lookups of `INP_IPV6` pcbs, `netinet6/in6_pcb.c`
//! (prototypes in `<netinet/in_pcb.h>`).
//!
//! Upstream: sys/netinet6/in6_pcb.c @ 3ce1f3f79392
//! LZ: sys/netinet6/in6_pcb.rs@3c62ede68802
//!
//! The licence block carries the NRL notice with its advertising clause, accepted as every
//! licence of the pinned OpenBSD tree is.
//!
//! ## Deviations
//! - `NPF`: the divert keys of [`in6_pcblookup_listen`] come through the `Pf` bridge
//!   (`find_divert`); before pf registers, no packet carries `PF_TAG_DIVERTED`.
//! - The `DIAGNOSTIC` `in_pcbnotifymiss` printfs are not made.
//!
//! ## Redesign
//! - D-17's first half (R3): the lookups and the hash, which read only the table and the
//!   pcbs' \[t\] members, are the kernel's here, over `crate::netinet::in_pcb`'s table (the
//!   pinned hash chains, the counted answers); the functions that need `in6_src.c`,
//!   `in6.c`'s address flags or the scope zones (`in6_pcbaddrisavail`, `in6_pcbconnect`,
//!   `in6_setsockaddr`, `in6_setpeeraddr`, `in6_pcbnotify`, `in6_pcbrtentry`,
//!   `in6_pcbset_addr`) stay in `bsd`'s `netinet6/in6_pcb.rs` until N6.5 moves those.
//! - The addresses are their sixteen bytes (`crate::netinet::in_pcb::Addr6`), so `bsd`'s
//!   `In6Addr` and `hal`'s both reach them.

use hal::sys::netinet6_in6::In6Addr;

use crate::assert::kassert;
use crate::assert::kpanic;
use crate::net::mbuf::{Mbuf, PF_TAG_DIVERTED, PF_TAG_TRANSLATE_LOCALHOST};
use crate::net::pf_hooks::{PF_DIVERT_REPLY, PF_DIVERT_TO, pf};
use crate::net::rtable::rtable_l2;
use crate::netinet::in_pcb::{Addr6, INP_IPV6, InpGuard, InpcbRef, Inpcbtable, hash_chain_find};
use crate::proc::ProcGlue;

/// `zeroin6_addr`: the unspecified address.
pub static ZEROIN6_ADDR: In6Addr = In6Addr { s6_addr: [0; 16] };

/// `in6_pcbhash`: the hash of the complete IPv6 address quadruple in routing domain
/// `rdomain`.
#[inline]
pub fn in6_pcbhash<G: ProcGlue>(
    table: &Inpcbtable<G>,
    rdomain: u32,
    faddr: &[u8; 16],
    fport: u16,
    laddr: &[u8; 16],
    lport: u16,
) -> u64 {
    let mut ctx = table.inpt_key.siphash24();
    ctx.update(&rdomain.to_be_bytes());
    ctx.update(faddr);
    ctx.update(&fport.to_ne_bytes());
    ctx.update(laddr);
    ctx.update(&lport.to_ne_bytes());
    ctx.end()
}

/// `in6_pcbhash_lookup`: the IPv6 control block with exactly these addresses, moved to the
/// head of its chain, counted; the table mutex held.
pub fn in6_pcbhash_lookup<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    hash: u64,
    rdomain: u32,
    faddr: &[u8; 16],
    fport: u16,
    laddr: &[u8; 16],
    lport: u16,
) -> Option<InpcbRef<G>> {
    hash_chain_find(g, hash, |inp| {
        kassert!(inp.has_flags(INP_IPV6));

        inp.inp_fport.get() == fport
            && inp.inp_lport.get() == lport
            && inp.inp_faddr6.bytes() == *faddr
            && inp.inp_laddr6.bytes() == *laddr
            && rtable_l2(inp.inp_rtableid.get()) == rdomain
    })
}

/// `in6_pcblookup_lock(.., IN_PCBLOCK_HOLD)`: the connected IPv6 control block for
/// `faddr.fport <-> laddr.lport`, counted; no wildcard matching is done. The table mutex
/// held.
pub fn in6_pcblookup_held<G: ProcGlue, A: Addr6>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    faddr: &A,
    fport: u16,
    laddr: &A,
    lport: u16,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let (faddr, laddr) = (faddr.to_bytes(), laddr.to_bytes());
    let rdomain = rtable_l2(rtable);
    let hash = in6_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    in6_pcbhash_lookup(g, hash, rdomain, &faddr, fport, &laddr, lport)
}

/// `in6_pcblookup`: [`in6_pcblookup_held`] taking the table mutex (`IN_PCBLOCK_GRAB`).
pub fn in6_pcblookup<G: ProcGlue, A: Addr6>(
    table: &Inpcbtable<G>,
    faddr: &A,
    fport: u16,
    laddr: &A,
    lport: u16,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let (faddr, laddr) = (faddr.to_bytes(), laddr.to_bytes());
    let rdomain = rtable_l2(rtable);
    let hash = in6_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    let mut g = table.lock();
    in6_pcbhash_lookup(&mut g, hash, rdomain, &faddr, fport, &laddr, lport)
}

/// `in6_pcblookup_listen`: the listening IPv6 control block for `laddr.lport_arg`:
/// unspecified foreign address and port, bound to `laddr` or to the wildcard address (pf's
/// divert-to key, or the wildcard first for a connection redirected to localhost, from `m`),
/// counted.
pub fn in6_pcblookup_listen<G: ProcGlue, A: Addr6>(
    table: &Inpcbtable<G>,
    laddr: &A,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let keys = in6_pcblookup_listen_keys(laddr, lport_arg, m)?;
    let mut g = table.lock();
    in6_pcblookup_listen_held(&mut g, table, keys, rtable)
}

/// The keys `in6_pcblookup_listen` looks up before it takes the table mutex (as
/// `kernel::netinet::in_pcb::in_pcblookup_listen_keys`): the first local address, the
/// second and the port; `None` for a divert-reply packet.
pub fn in6_pcblookup_listen_keys<A: Addr6>(
    laddr: &A,
    lport_arg: u16,
    m: Option<&Mbuf>,
) -> Option<([u8; 16], [u8; 16], u16)> {
    let laddr = laddr.to_bytes();
    let zero = ZEROIN6_ADDR.s6_addr;
    let mut key1 = laddr;
    let mut key2 = zero;
    let mut lport = lport_arg;

    if let Some(m) = m
        && m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0
    {
        let divert = pf().and_then(|pf| pf.find_divert(m));
        kassert!(divert.is_some());
        let divert = divert?;
        match divert.type_ {
            PF_DIVERT_TO => {
                key1 = divert.addr;
                key2 = key1;
                lport = divert.port;
            }
            PF_DIVERT_REPLY => return None,
            t => kpanic(format_args!(
                "in6_pcblookup_listen: unknown divert type {t}, mbuf {m:p}"
            )),
        }
    } else if let Some(m) = m
        && m.m_pkthdr().pf.flags.get() & PF_TAG_TRANSLATE_LOCALHOST != 0
    {
        // Redirected connections should not be treated the same as connections directed to
        // ::1 since localhost can only be accessed from the host itself.
        key1 = zero;
        key2 = laddr;
    }
    Some((key1, key2, lport))
}

/// `in6_pcblookup_listen`'s lookups of `keys`, the table mutex held: the pcb bound to the
/// first key, else to the second, counted.
pub fn in6_pcblookup_listen_held<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    (key1, key2, lport): ([u8; 16], [u8; 16], u16),
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let zero = ZEROIN6_ADDR.s6_addr;
    let rdomain = rtable_l2(rtable);
    let hash = in6_pcbhash(table, rdomain, &zero, 0, &key1, lport);
    let mut inp = in6_pcbhash_lookup(g, hash, rdomain, &zero, 0, &key1, lport);
    if inp.is_none() && key1 != key2 {
        let hash = in6_pcbhash(table, rdomain, &zero, 0, &key2, lport);
        inp = in6_pcbhash_lookup(g, hash, rdomain, &zero, 0, &key2, lport);
    }
    inp
}
/* </CODE> */
