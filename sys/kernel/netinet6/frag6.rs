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
/* </LICENSES> */

/* <CODE> */
//! IPv6 reassembly: `netinet6/frag6.c`. [`frag6_input`] is the fragment header's `pr_input`.
//! An atomic fragment (offset 0, no more fragments, RFC 6946) is processed in isolation;
//! every other fragment is queued on the reassembly queue of its datagram (an [`Ip6q`],
//! found by identification, addresses and routing domain), sorted by offset. Overlapping
//! fragments discard the whole datagram (RFC 5722, Errata 3089). Once the fragments cover the
//! datagram they are concatenated, the fragment header is removed and the next header of the
//! reassembled packet is returned. [`frag6_slowtimo`] ages the queues, trims them to
//! `net.inet6.ip6.maxfragpackets`, and sends an ICMPv6 time exceeded error for the first
//! fragment of an expired datagram.
//!
//! Upstream: sys/netinet6/frag6.c @ 3ce1f3f79392
//! LZ: sys/netinet6/frag6.rs@3c62ede68802
//!
//! Locks: \[Q\] `frag6_mutex` ([`FRAG6`], `IPL_SOFTNET`) owns the queue, the two counters and
//! every [`Ip6q`] and [`Ip6asfrag`] on it.
//!
//! ## Deviations
//! - The C's `struct mbuf **` is `&mut Option<MbufBox>`: a queued fragment is the queue's,
//!   `*mp` is `None` then (the C returns `IPPROTO_DONE` with `*mp` still naming it).
//! - `frag6_nfragpackets` is counted before the datagram's queue is allocated and not given
//!   back when the allocation fails, as the C (a count that stays one above the queues until
//!   reboot after each such failure).
//! - The headers the C writes in place (the reassembled header's length, addresses and ECN
//!   bit, the restored next header, an expired or oversized fragment's addresses) are
//!   written into the fragment's first mbuf, which `ipv6_check` made writable when it was
//!   read-only; a write that finds no buffer drops the packet, counted as the C counts a
//!   buffer failure here (`ip6s_fragdropped`): never taken while the mbuf is writable.
//!
//! ## Redesign
//! - LZ's `FRAG6_QUEUE` (a `bsd` `TailqHead` made `Sync` by hand), `FRAG6_MUTEX` beside it,
//!   the two counters as atomics and the queue's entries written whole into pool items through
//!   `unsafe` (22 `unsafe`) -> [`FRAG6`], a `Mutexed` owning [`Frag6State`] (the C's \[Q\]
//!   members inside their lock): the datagrams on an `OwnedSlist` of "ip6q" items, each owning
//!   its fragments on an `OwnedSlist` of "ip6af" items (`kernel::netinet6::ip6_var`'s
//!   [`Ip6q`] and [`Ip6asfrag`]), the IPv4 shape of `ip_reass` (D-12). The C's
//!   `MUTEX_ASSERT_LOCKED(&frag6_mutex)` is the `&mut Frag6State` the guard lends.
//! - The datagram a fragment belongs to is found by the C's walk from the head, the cursor
//!   kept on its predecessor; it is taken off the queue to be changed (`remove_next`, or
//!   `pop_head` at the head) and put back in its place (`split_after`, `push_tail`, `concat`),
//!   all O(1); a new one goes in at the head (`TAILQ_INSERT_HEAD`). A fragment goes in after
//!   its predecessor the same way; the fragments of a first fragment's datagram that would
//!   exceed `IPV6_MAXPACKET` are taken out with the cursor (`LIST_FOREACH_SAFE`).
//! - `frag6_slowtimo`'s drain of the oldest datagrams (`TAILQ_LAST`, O(1) a step on the C's
//!   doubly-linked queue) takes the queue's last `k` at once: one walk to the cut and
//!   `split_after`, O(n) for the `k`, where the C's is O(k); the function was O(n) already
//!   (the aging walk before it), so its order is the C's. D-12 named `OwnedList::pop_tail` for
//!   this; the singly-linked queue keeps the input path's changes O(1) without the counted
//!   references an `OwnedList` element needs to be changed off its list (R1 of N6.5d).
//! - `frag6_deletefraghdr` moves the unfragmentable headers in place (`copy_within`) when the
//!   first mbuf is writable and holds them, as the C's `memmove`; otherwise it takes the C's
//!   other branch (`m_split`, `m_adj`, `m_cat`), which writes nothing.
//! - The restored next header is written in the mbuf of the chain that holds it
//!   (`ip6_input::ip6_set_byte`), not through `m_copyback`.
//!
//! **Lock and spl transitions** against the C: `frag6_input` takes `frag6_mutex` once per
//! fragment that is not atomic (one spl pair), as the C, and sends the ICMPv6 errors of a
//! first fragment's datagram under it, as the C; `frag6_slowtimo` takes it once, and the
//! shared net lock once per expired first fragment (`frag6_freef`), as the C. What the C
//! frees after `mtx_leave` is freed after the guard: a flushed datagram's queue, fragments,
//! packets and new entry (`flushfrags`), the entry of a fragment the ECN mix drops, the
//! queue of a datagram that left the queue (the section hands them out in `Out` and its
//! second value); what the C frees under the mutex (the entries the concatenation consumes,
//! a first fragment's datagram's too long ones) is freed under it.

use core::mem::{offset_of, size_of};
use core::sync::atomic::Ordering;

use hal::cpu::intr::IPL_SOFTNET;
use hal::ds::owned::OwnedSlist;
use hal::mem::pod;
use hal::mem::pool::{BoxPool, PR_NOWAIT, PoolBox};
use hal::sync::mutex::Mutexed;
use hal::sync::netlock::{NETLOCK, NetHeld};
use hal::sys::netinet_icmp6::{
    ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, ICMP6_TIME_EXCEED_REASSEMBLY, ICMP6_TIME_EXCEEDED,
};
use hal::sys::netinet_in::IPPROTO_DONE;
use hal::sys::netinet_ip::{IPTOS_ECN_CE, IPTOS_ECN_MASK, IPTOS_ECN_NOTECT};
use hal::sys::netinet_ip6::{
    IP6F_MORE_FRAG, IP6F_OFF_MASK, IPV6_FRAGTTL, IPV6_MAXPACKET, Ip6Frag, Ip6Hdr,
};
use hal::sys::netinet6_ip6_var::Ip6statCounters;

use crate::assert::kpanic;
use crate::net::if_var::Netstack;
use crate::net::mbuf::{M_DONTWAIT, Mbuf, MbufBox, m_adj, m_calchdrlen, m_cat, m_split};
use crate::net::rtable::rtable_l2;
use crate::netinet6::icmp6::icmp6_error;
use crate::netinet6::ip6_input::{ip6_exthdr_bytes, ip6_exthdr_get, ip6_get_prevhdr, ip6_set_byte};
use crate::netinet6::ip6_output::{ip6_load, ip6_store};
use crate::netinet6::ip6_var::{
    IP6_MAXFRAGPACKETS, IP6_MAXFRAGS, Ip6asfrag, Ip6asfragList, Ip6q, Ip6qList, ip6stat_add,
    ip6stat_inc,
};

/// `sizeof(struct ip6_hdr)` in offset arithmetic.
const IP6_HDR_LEN: i32 = size_of::<Ip6Hdr>() as i32;
/// `sizeof(struct ip6_frag)` in offset arithmetic.
const IP6_FRAG_LEN: i32 = size_of::<Ip6Frag>() as i32;

/// \[Q\] What `frag6_mutex` guards.
pub struct Frag6State {
    /// `frag6_queue`: ip6 reassemble queue, the newest datagram first.
    q: OwnedSlist<Ip6qList>,
    /// `frag6_nfragpackets`.
    nfragpackets: u32,
    /// `frag6_nfrags`.
    nfrags: u32,
}

#[cfg(test)]
impl Frag6State {
    /// `frag6_nfrags`, for the tests.
    pub(crate) fn nfrags(&self) -> u32 {
        self.nfrags
    }

    /// `frag6_nfragpackets`, for the tests.
    pub(crate) fn nfragpackets(&self) -> u32 {
        self.nfragpackets
    }

    /// The identifications of the datagrams on the queue, from its head.
    pub(crate) fn idents(&self) -> std::vec::Vec<u32> {
        self.q.iter().map(|q6| q6.ip6q_ident).collect()
    }
}

/// `frag6_mutex` and what it guards.
pub static FRAG6: Mutexed<Frag6State> = Mutexed::new(
    IPL_SOFTNET,
    Frag6State {
        q: OwnedSlist::new(),
        nfragpackets: 0,
        nfrags: 0,
    },
);

/// `ip6af_pool`: the fragments ("ip6af").
pub static IP6AF_POOL: BoxPool<Ip6asfrag> = BoxPool::new();
/// `ip6q_pool`: the datagrams being reassembled ("ip6q").
pub static IP6Q_POOL: BoxPool<Ip6q> = BoxPool::new();

/// `frag6_init`: initialise reassembly queue and pools (the queue is made empty with its
/// mutex; a second call, a test's, leaves the pools as they are).
pub fn frag6_init() {
    let _ = IP6AF_POOL.init(IPL_SOFTNET, 0, "ip6af");
    let _ = IP6Q_POOL.init(IPL_SOFTNET, 0, "ip6q");
    // TAILQ_INIT(&frag6_queue): FRAG6's initialiser.
}

/// What the section under `frag6_mutex` did with a fragment.
enum Out {
    /// It was queued (`IPPROTO_DONE`).
    Queued,
    /// It completed its datagram: the packet, the next header and its offset.
    Reassembled(MbufBox, u8, i32),
    /// `dropfrag`: the fragment (or the datagram it completed) dropped, counted.
    Dropfrag(Option<MbufBox>),
    /// `dropfrag` of a fragment whose entry was made (the ECN mix): the entry and its packet,
    /// both freed after the mutex (the C's `pool_put` after `mtx_leave`, then `m_freemp`).
    Dropentry(PoolBox<Ip6asfrag>),
    /// `flushfrags`: an overlap discarded the datagram; the new fragment's entry and its
    /// packet, and the count of what was dropped (the datagram's fragments and the new one),
    /// freed and counted after the mutex (the datagram's fragments go with its queue, which
    /// left the queue).
    Flush(PoolBox<Ip6asfrag>, u64),
    /// The reassembled packet would exceed `IPV6_MAXPACKET`: a parameter problem at `erroff`.
    Toobig(MbufBox, i32),
}

// In RFC2460, fragment and reassembly rule do not agree with each other, in terms of next
// header field handling in fragment header. While the sender will use the same value for all
// of the fragmented packets, receiver is suggested not to check the consistency.
//
// fragment rule (p20):
//	(2) A Fragment header containing:
//	The Next Header value that identifies the first header of
//	the Fragmentable Part of the original packet.
//		-> next header field is same for all fragments
//
// reassembly rule (p21):
//	The Next Header field of the last header of the Unfragmentable
//	Part is obtained from the Next Header field of the first
//	fragment's Fragment header.
//		-> should grab it from the first fragment only
//
// The following note also contradicts with fragment rule - noone is going to send different
// fragment with different next header field.
//
// additional note (p22):
//	The Next Header values in the Fragment headers of different
//	fragments of the same original packet may differ.  Only the value
//	from the Offset zero fragment packet is used for reassembly.
//		-> should grab it from the first fragment only
//
// There is no explicit reason given in the RFC. Historical reason maybe?

/// `frag6_input(mp, offp, proto, af, ns)`: fragment input, the fragment header's `pr_input`,
/// under the shared net lock (`net`): the next header of the reassembled (or atomic) packet
/// with `*offp` at it, or `IPPROTO_DONE` (`*mp` cleared) when the fragment was queued or
/// dropped.
pub fn frag6_input(
    mp: &mut Option<MbufBox>,
    offp: &mut i32,
    _proto: i32,
    _af: i32,
    _ns: Option<&mut Netstack>,
    net: &NetHeld<'_>,
) -> i32 {
    let Some(m) = mp.take() else {
        return IPPROTO_DONE;
    };
    let mut offset = *offp;

    // ip6 = mtod(*mp, struct ip6_hdr *): read once (m_pulldown below keeps the bytes before
    // the fragment header where they are).
    let ip6 = ip6_load(&m);
    let Some((m, at, o)) = ip6_exthdr_get(m, offset, IP6_FRAG_LEN) else {
        return IPPROTO_DONE;
    };
    let Some(ip6f) =
        ip6_exthdr_bytes(&m, at, o, IP6_FRAG_LEN as usize).and_then(|b| pod::at::<Ip6Frag>(b, 0))
    else {
        // ip6_exthdr_get made the header's bytes contiguous there.
        return IPPROTO_DONE;
    };
    let plen = i32::from(u16::from_be(ip6.ip6_plen));

    // jumbo payload can't contain a fragment header
    if ip6.ip6_plen == 0 {
        icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, offset, net);
        return IPPROTO_DONE;
    }

    // check whether fragment packet's fragment length is multiple of 8 octets.
    // sizeof(struct ip6_frag) == 8
    // sizeof(struct ip6_hdr) = 40
    if ip6f.ip6f_offlg & IP6F_MORE_FRAG != 0 && (plen - offset) & 0x7 != 0 {
        icmp6_error(
            m,
            ICMP6_PARAM_PROB,
            ICMP6_PARAMPROB_HEADER,
            offset_of!(Ip6Hdr, ip6_plen) as i32,
            net,
        );
        return IPPROTO_DONE;
    }

    ip6stat_inc(Ip6statCounters::Ip6sFragments);

    // offset now points to data portion
    offset += IP6_FRAG_LEN;

    // RFC6946: A host that receives an IPv6 packet which includes a Fragment Header with the
    // "Fragment Offset" equal to 0 and the "M" bit equal to 0 MUST process such packet in
    // isolation from any other packets/fragments.
    let fragoff = i32::from(u16::from_be(ip6f.ip6f_offlg & IP6F_OFF_MASK));
    if fragoff == 0 && ip6f.ip6f_offlg & IP6F_MORE_FRAG == 0 {
        ip6stat_inc(Ip6statCounters::Ip6sReassembled);
        *offp = offset;
        *mp = Some(m);
        return i32::from(ip6f.ip6f_nxt);
    }

    // Ignore empty non atomic fragment, do not classify as overlapping.
    if IP6_HDR_LEN + plen <= offset {
        return IPPROTO_DONE;
    }

    let rdomain = rtable_l2(m.m_pkthdr().ph_rtableid.get());
    let (out, q6) = {
        let mut g = FRAG6.lock();
        frag6_locked(&mut g, m, &ip6, ip6f, offset, fragoff, plen, rdomain, net)
    };
    // pool_put(&ip6q_pool, q6) after mtx_leave(&frag6_mutex), its fragments with it (a
    // flushed datagram's: m_freem and pool_put of each).
    drop(q6);

    match out {
        Out::Queued => IPPROTO_DONE,
        Out::Dropentry(ip6af) => {
            // pool_put(&ip6af_pool, ip6af), then dropfrag's m_freemp(mp).
            drop(ip6af);
            ip6stat_inc(Ip6statCounters::Ip6sFragdropped);
            IPPROTO_DONE
        }
        Out::Flush(ip6af, n) => {
            // pool_put(&ip6af_pool, ip6af), ip6stat_add, m_freemp(mp).
            drop(ip6af);
            ip6stat_add(Ip6statCounters::Ip6sFragdropped, n);
            IPPROTO_DONE
        }
        Out::Toobig(m, erroff) => {
            icmp6_error(m, ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, erroff, net);
            IPPROTO_DONE
        }
        Out::Dropfrag(m) => {
            ip6stat_inc(Ip6statCounters::Ip6sFragdropped);
            drop(m);
            IPPROTO_DONE
        }
        Out::Reassembled(m, nxt, offset) => {
            m_calchdrlen(&m);

            // Restore NXT to the original.
            let prvnxt = ip6_get_prevhdr(&m, offset);
            let mut m = m;
            if !ip6_set_byte(&mut m, prvnxt, nxt) {
                ip6stat_inc(Ip6statCounters::Ip6sFragdropped);
                return IPPROTO_DONE;
            }

            ip6stat_inc(Ip6statCounters::Ip6sReassembled);

            // Tell launch routine the next header
            *offp = offset;
            *mp = Some(m);
            i32::from(nxt)
        }
    }
}

/// `frag6_input`'s section under `frag6_mutex` (`st`, its guard's): the fragment `m` (header
/// `ip6`, fragment header `ip6f`, its fragmentable part at `offset` and fragment offset
/// `fragoff`, the payload length `plen`) of the datagram of `rdomain` queued, dropped or
/// completing its datagram; with it, a datagram's queue that left the queue, to be freed after
/// the mutex (the C's `pool_put` after `mtx_leave`).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's locals the section reads, and the net lock's proof for its ICMPv6 errors"
)]
fn frag6_locked(
    st: &mut Frag6State,
    m: MbufBox,
    ip6: &Ip6Hdr,
    ip6f: Ip6Frag,
    offset: i32,
    fragoff: i32,
    plen: i32,
    rdomain: u32,
    net: &NetHeld<'_>,
) -> (Out, Option<PoolBox<Ip6q>>) {
    let Frag6State {
        q,
        nfragpackets,
        nfrags,
    } = st;

    // Enforce upper bound on number of fragments. If maxfrag is 0, never accept fragments.
    if *nfrags >= IP6_MAXFRAGS.load(Ordering::Relaxed) as u32 {
        return (Out::Dropfrag(Some(m)), None);
    }

    // TAILQ_FOREACH: the cursor kept on the predecessor of the one found, so the datagram
    // comes off the queue and goes back in its place in O(1).
    let matches = |q6: &Ip6q| {
        ip6f.ip6f_ident == q6.ip6q_ident
            && ip6.ip6_src == q6.ip6q_src
            && ip6.ip6_dst == q6.ip6q_dst
            && rdomain == q6.ip6q_rdomain
    };
    let (mut q6, mut rest) = if q.first().is_some_and(matches) {
        (q.pop_head(), None)
    } else {
        let mut c = q.cursor_mut();
        let found = loop {
            let Some(cur) = c.current() else {
                break false;
            };
            match OwnedSlist::<Ip6qList>::next(cur) {
                None => break false,
                Some(n) if matches(n) => break true,
                Some(_) => {
                    c.move_next();
                }
            }
        };
        if found {
            let q6 = c.remove_next();
            (q6, Some(c.split_after()))
        } else {
            (None, None)
        }
    };
    let first_frag = q6.is_none();
    if q6.is_none() {
        // the first fragment to arrive, create a reassembly queue.

        // Enforce upper bound on number of fragmented packets for which we attempt
        // reassembly; If maxfragpackets is 0, never accept fragments.
        if *nfragpackets >= IP6_MAXFRAGPACKETS.load(Ordering::Relaxed) as u32 {
            return (Out::Dropfrag(Some(m)), None);
        }
        *nfragpackets += 1;
        // ip6q_nxt will be filled afterwards, from 1st fragment
        q6 = IP6Q_POOL.get(PR_NOWAIT, || {
            let ecn = ((u32::from_be(ip6.ip6_flow) >> 20) as u8) & IPTOS_ECN_MASK;
            Ip6q::new(
                rdomain,
                ip6f.ip6f_ident,
                ip6.ip6_src,
                ip6.ip6_dst,
                ecn,
                IPV6_FRAGTTL,
            )
        });
        if q6.is_none() {
            // The count stays (the module's deviations).
            return (Out::Dropfrag(Some(m)), None);
        }
    }
    let Some(mut q6) = q6 else {
        kpanic(format_args!("frag6_input: no reassembly queue"));
    };

    let (out, keep) = frag6_queue(
        nfrags, &mut q6, m, ip6, ip6f, offset, fragoff, plen, first_frag, net,
    );

    // The datagram's place again (LIST_INSERT_AFTER of the C's untouched place, or the head:
    // its own or a new queue's TAILQ_INSERT_HEAD), unless it left the queue.
    if keep {
        match rest.as_mut() {
            Some(r) => {
                q.push_tail(q6);
                q.concat(r);
            }
            None => q.push_head(q6),
        }
        return (out, None);
    }
    if let Some(r) = rest.as_mut() {
        q.concat(r);
    }
    // TAILQ_REMOVE(&frag6_queue, q6, ip6q_queue): the counts of what left.
    *nfrags -= q6.ip6q_nfrag as u32;
    *nfragpackets -= 1;
    (out, Some(q6))
}

/// `frag6_input`'s work on the fragment's datagram `q6` (off the queue, `nfrags` the guard's
/// `frag6_nfrags`): the fragment `m` checked and inserted, the datagram reassembled when
/// complete. With the outcome, whether `q6` goes back on the queue (`false`: it left it, its
/// fragments freed or made into the packet).
#[allow(
    clippy::too_many_arguments,
    reason = "the C's locals the section reads, and the net lock's proof for its ICMPv6 errors"
)]
fn frag6_queue(
    nfrags: &mut u32,
    q6: &mut PoolBox<Ip6q>,
    m: MbufBox,
    ip6: &Ip6Hdr,
    ip6f: Ip6Frag,
    offset: i32,
    fragoff: i32,
    plen: i32,
    first_frag: bool,
    net: &NetHeld<'_>,
) -> (Out, bool) {
    // If it's the 1st fragment, record the length of the unfragmentable part and the next
    // header of the fragment header.
    if fragoff == 0 {
        q6.ip6q_unfrglen = offset - IP6_HDR_LEN - IP6_FRAG_LEN;
        q6.ip6q_nxt = ip6f.ip6f_nxt;
    }

    // Check that the reassembled packet would not exceed 65535 bytes in size. If it would
    // exceed, discard the fragment and return an ICMP error.
    let frgpartlen = IP6_HDR_LEN + plen - offset;
    let erroff = offset - IP6_FRAG_LEN + offset_of!(Ip6Frag, ip6f_offlg) as i32;
    let toobig = if q6.ip6q_unfrglen >= 0 {
        // The 1st fragment has already arrived.
        q6.ip6q_unfrglen + fragoff + frgpartlen > IPV6_MAXPACKET as i32
    } else {
        fragoff + frgpartlen > IPV6_MAXPACKET as i32
    };
    if toobig {
        return (Out::Toobig(m, erroff), true);
    }

    // If it's the first fragment, do the above check for each fragment already stored in the
    // reassembly queue (LIST_FOREACH_SAFE, the cursor on the predecessor of the one looked at).
    if fragoff == 0 {
        let unfrglen = q6.ip6q_unfrglen;
        let (src, dst) = (q6.ip6q_src, q6.ip6q_dst);
        let toolong =
            |af6: &Ip6asfrag| unfrglen + af6.ip6af_off + af6.ip6af_frglen > IPV6_MAXPACKET as i32;
        let mut sent = |af6: PoolBox<Ip6asfrag>, nfrag: &mut i32| {
            let mut af6 = af6;
            let erroff = af6.ip6af_offset - IP6_FRAG_LEN + offset_of!(Ip6Frag, ip6f_offlg) as i32;
            let merr = af6.ip6af_m.take();
            // dequeue the fragment: pool_put(&ip6af_pool, af6).
            drop(af6);
            *nfrags -= 1;
            *nfrag -= 1;
            if let Some(merr) = merr {
                frag6_error(
                    merr,
                    src,
                    dst,
                    ICMP6_PARAM_PROB,
                    ICMP6_PARAMPROB_HEADER,
                    erroff,
                    net,
                );
            }
        };
        let mut nfrag = q6.ip6q_nfrag;
        let fragq = &mut q6.ip6q_asfrag;
        while fragq.first().is_some_and(toolong) {
            if let Some(af6) = fragq.pop_head() {
                sent(af6, &mut nfrag);
            }
        }
        let mut c = fragq.cursor_mut();
        while let Some(n) = c.current().and_then(OwnedSlist::<Ip6asfragList>::next) {
            if toolong(n) {
                if let Some(af6) = c.remove_next() {
                    sent(af6, &mut nfrag);
                }
            } else {
                c.move_next();
            }
        }
        q6.ip6q_nfrag = nfrag;
    }

    let mff = ip6f.ip6f_offlg & IP6F_MORE_FRAG;
    let mut m = Some(m);
    let Some(ip6af) = IP6AF_POOL.get(PR_NOWAIT, || {
        let Some(m) = m.take() else {
            kpanic(format_args!("frag6_input: no fragment"));
        };
        Ip6asfrag::new(m, offset, frgpartlen, fragoff, mff)
    }) else {
        return (Out::Dropfrag(m), true);
    };

    if !first_frag {
        // Handle ECN by comparing this segment with the first one; if CE is set, do not lose
        // CE. drop if CE and not-ECT are mixed for the same packet.
        let ecn = ((u32::from_be(ip6.ip6_flow) >> 20) as u8) & IPTOS_ECN_MASK;
        let ecn0 = q6.ip6q_ecn;
        if ecn == IPTOS_ECN_CE {
            if ecn0 == IPTOS_ECN_NOTECT {
                return (Out::Dropentry(ip6af), true);
            }
            if ecn0 != IPTOS_ECN_CE {
                q6.ip6q_ecn = IPTOS_ECN_CE;
            }
        }
        if ecn == IPTOS_ECN_NOTECT && ecn0 != IPTOS_ECN_NOTECT {
            return (Out::Dropentry(ip6af), true);
        }
    }

    // Find a segment which begins after this one does: `paf6` the last one that does not (the
    // cursor's element), `af6` the one after it. A first fragment of a new datagram goes in
    // at the head (goto insert).
    let fragq = &mut q6.ip6q_asfrag;
    let head = first_frag
        || fragq
            .first()
            .is_none_or(|af6| af6.ip6af_off > ip6af.ip6af_off);
    let fragq = &mut q6.ip6q_asfrag;
    if head {
        // RFC 5722, Errata 3089: When reassembling an IPv6 datagram, if one or more its
        // constituent fragments is determined to be an overlapping fragment, the entire
        // datagram (and any constituent fragments) MUST be silently discarded. (No
        // predecessor; the head is the one after.)
        let overlap = !first_frag
            && fragq
                .first()
                .is_some_and(|af6| ip6af.ip6af_off + ip6af.ip6af_frglen - af6.ip6af_off > 0);
        if overlap {
            return flushfrags(q6, ip6af);
        }
        // insert: LIST_INSERT_HEAD.
        fragq.push_head(ip6af);
    } else {
        let mut c = fragq.cursor_mut();
        while let Some(n) = c.current().and_then(OwnedSlist::<Ip6asfragList>::next) {
            if n.ip6af_off > ip6af.ip6af_off {
                break;
            }
            c.move_next();
        }
        let overlap = c
            .current()
            .is_some_and(|paf6| paf6.ip6af_off + paf6.ip6af_frglen - ip6af.ip6af_off > 0)
            || c.current()
                .and_then(OwnedSlist::<Ip6asfragList>::next)
                .is_some_and(|af6| ip6af.ip6af_off + ip6af.ip6af_frglen - af6.ip6af_off > 0);
        if overlap {
            return flushfrags(q6, ip6af);
        }
        // insert: LIST_INSERT_AFTER(paf6, ip6af, ip6af_list).
        let mut rest = c.split_after();
        fragq.push_tail(ip6af);
        fragq.concat(&mut rest);
    }

    // Stick new segment in its place; check for complete reassembly. Move to front of packet
    // queue, as we are the most recently active fragmented packet. (The C does not move it.)
    *nfrags += 1;
    q6.ip6q_nfrag += 1;
    let mut next = 0;
    let mut last: Option<&Ip6asfrag> = None;
    for af6 in &q6.ip6q_asfrag {
        if af6.ip6af_off != next {
            return (Out::Queued, true);
        }
        next += af6.ip6af_frglen;
        last = Some(af6);
    }
    if last.is_none_or(|l| l.ip6af_mff != 0) {
        return (Out::Queued, true);
    }

    // Reassembly is complete; concatenate fragments.
    let fragq = &mut q6.ip6q_asfrag;
    let Some(mut first) = fragq.pop_head() else {
        kpanic(format_args!(
            "frag6_input: a complete datagram without fragments"
        ));
    };
    let Some(mut m) = first.ip6af_m.take() else {
        kpanic(format_args!("frag6_input: a fragment without its packet"));
    };
    {
        // `t`: the chain's last mbuf, kept across the fragments (the C's walk from it).
        let mut t: &mut Mbuf = &mut m;
        while let Some(mut af6) = fragq.pop_head() {
            t = last_mbuf(t);
            let Some(n) = af6.ip6af_m.take() else {
                kpanic(format_args!("frag6_input: a fragment without its packet"));
            };
            let skip = af6.ip6af_offset;
            // pool_put(&ip6af_pool, af6)
            drop(af6);
            drop(t.m_next.set(n));
            if let Some(n) = t.m_next.get_mut() {
                m_adj(n, skip);
                crate::net::mbuf::m_removehdr(n);
            }
        }
    }

    // adjust offset to point where the original next header starts
    let offset = first.ip6af_offset - IP6_FRAG_LEN;
    // pool_put(&ip6af_pool, ip6af)
    drop(first);
    next += offset - IP6_HDR_LEN;
    if next as u32 > IPV6_MAXPACKET as u32 {
        return (Out::Dropfrag(Some(m)), false);
    }
    let mut h = ip6_load(&m);
    h.ip6_plen = (next as u16).to_be();
    h.ip6_src = q6.ip6q_src;
    h.ip6_dst = q6.ip6q_dst;
    if q6.ip6q_ecn == IPTOS_ECN_CE {
        h.ip6_flow |= (u32::from(IPTOS_ECN_CE) << 20).to_be();
    }
    let nxt = q6.ip6q_nxt;
    if !ip6_store(&mut m, &h) {
        return (Out::Dropfrag(Some(m)), false);
    }

    // Delete frag6 header
    if frag6_deletefraghdr(&mut m, offset).is_err() {
        return (Out::Dropfrag(Some(m)), false);
    }

    (Out::Reassembled(m, nxt, offset), false)
}

/// The last mbuf of the chain from `m` (`while (t->m_next) t = t->m_next`).
fn last_mbuf(m: &mut Mbuf) -> &mut Mbuf {
    let mut t = m;
    while t.m_next.is_some() {
        t = match t.m_next.get_mut() {
            Some(n) => n,
            None => kpanic(format_args!("frag6_input: a link that is and is not")),
        };
    }
    t
}

/// `flushfrags`: the datagram `q6` discarded with the new fragment `ip6af`: the queue leaves
/// the queue (the C's `TAILQ_REMOVE` and its counts are the caller's) with its fragments, and
/// the new fragment's entry goes out with the count of `ip6s_fragdropped`, all freed after
/// the mutex as the C's (`frag6_input` drops them once the guard is gone).
fn flushfrags(q6: &PoolBox<Ip6q>, ip6af: PoolBox<Ip6asfrag>) -> (Out, bool) {
    let n = u64::try_from(q6.ip6q_nfrag + 1).unwrap_or(0);
    (Out::Flush(ip6af, n), false)
}

/// `frag6_deletefraghdr(m, offset)`: delete fragment header after the unfragmentable header
/// portions (`offset` bytes). `ENOBUFS` when the split finds no mbuf.
pub fn frag6_deletefraghdr(m: &mut Mbuf, offset: i32) -> Result<(), hal::sys::errno::Errno> {
    let len = m.m_len().get() as usize;
    let off = offset as usize;
    let frag = IP6_FRAG_LEN as usize;
    if len >= off + frag
        && let Some(d) = m.data_mut()
    {
        // memmove(mtod(m) + sizeof(struct ip6_frag), mtod(m), offset)
        d.copy_within(..off, frag);
        m.data_add(frag as isize);
        m.m_len().set((len - frag) as u32);
        return Ok(());
    }
    // this comes with no copy if the boundary is on cluster
    let Some(t) = m_split(m, offset, M_DONTWAIT) else {
        return Err(hal::sys::errno::Errno::ENOBUFS);
    };
    m_adj(&t, IP6_FRAG_LEN);
    m_cat(m, Some(t));
    Ok(())
}

/// The fragment `m` of a datagram from `src` to `dst` (its addresses restored in its header)
/// answered with the ICMPv6 error `type_`/`code` at `param`; freed when its header cannot be
/// written.
#[allow(
    clippy::too_many_arguments,
    reason = "icmp6_error's arguments and the addresses to restore"
)]
fn frag6_error(
    m: MbufBox,
    src: hal::sys::netinet6_in6::In6Addr,
    dst: hal::sys::netinet6_in6::In6Addr,
    type_: u8,
    code: u8,
    param: i32,
    net: &NetHeld<'_>,
) {
    let mut m = m;
    // Restore source and destination addresses in the erroneous IPv6 header.
    let mut h = ip6_load(&m);
    h.ip6_src = src;
    h.ip6_dst = dst;
    if ip6_store(&mut m, &h) {
        icmp6_error(m, type_, code, param, net);
    }
}

/// `frag6_freef(q6)`: free a fragment reassembly header and all associated datagrams. The
/// header must not be in any queue. Without the net lock (`frag6_slowtimo`'s caller holds
/// none): the ICMPv6 time exceeded error of the first fragment is sent under the shared net
/// lock, taken for it, as the C.
fn frag6_freef(q6: PoolBox<Ip6q>) {
    let mut q6 = q6;
    let (src, dst) = (q6.ip6q_src, q6.ip6q_dst);
    while let Some(mut af6) = q6.ip6q_asfrag.pop_head() {
        let m = af6.ip6af_m.take();
        // Return ICMP time exceeded error for the 1st fragment. Just free other fragments.
        if af6.ip6af_off == 0
            && let Some(m) = m
        {
            let shared = NETLOCK.read();
            match NetHeld::of(&shared) {
                Some(net) => frag6_error(
                    m,
                    src,
                    dst,
                    ICMP6_TIME_EXCEEDED,
                    ICMP6_TIME_EXCEED_REASSEMBLY,
                    0,
                    &net,
                ),
                None => kpanic(format_args!("frag6_freef: the net lock")),
            }
            drop(shared);
        }
        // pool_put(&ip6af_pool, af6)
        drop(af6);
    }
    // pool_put(&ip6q_pool, q6)
    drop(q6);
}

/// `frag6_slowtimo()`: IPv6 reassembling timer processing; if a timer expires on a reassembly
/// queue, discard it.
pub fn frag6_slowtimo() {
    let ip6_maxfragpackets_local = IP6_MAXFRAGPACKETS.load(Ordering::Relaxed) as u32;
    // rmq6: the queues unlinked, freed after the mutex (TAILQ_INSERT_HEAD each).
    let mut rmq6: OwnedSlist<Ip6qList> = OwnedSlist::new();

    {
        let mut g = FRAG6.lock();
        let Frag6State {
            q,
            nfragpackets,
            nfrags,
        } = &mut *g;
        let mut unlink = |q6: PoolBox<Ip6q>, rmq6: &mut OwnedSlist<Ip6qList>| {
            // frag6_unlink(q6, &rmq6)
            *nfrags -= q6.ip6q_nfrag as u32;
            *nfragpackets -= 1;
            rmq6.push_head(q6);
        };

        // TAILQ_FOREACH_SAFE: the head first, then the cursor on each one's predecessor.
        let expired = |q6: &Ip6q| {
            let ttl = q6.ip6q_ttl.get().wrapping_sub(1);
            q6.ip6q_ttl.set(ttl);
            ttl == 0
        };
        while q.first().is_some_and(expired) {
            ip6stat_inc(Ip6statCounters::Ip6sFragtimeout);
            if let Some(q6) = q.pop_head() {
                unlink(q6, &mut rmq6);
            }
        }
        let mut c = q.cursor_mut();
        while let Some(n) = c.current().and_then(OwnedSlist::<Ip6qList>::next) {
            if expired(n) {
                ip6stat_inc(Ip6statCounters::Ip6sFragtimeout);
                if let Some(q6) = c.remove_next() {
                    unlink(q6, &mut rmq6);
                }
            } else {
                c.move_next();
            }
        }

        // If we are over the maximum number of fragments (due to the limit being lowered),
        // drain off enough to get down to the new limit: the queue's last ones, which the C
        // takes one at a time from the tail (TAILQ_LAST), the newest of them into rmq6 last.
        let k = (*nfragpackets)
            .saturating_sub(ip6_maxfragpackets_local)
            .min(q.len() as u32) as usize;
        if k > 0 {
            let keep = q.len() - k;
            let mut drained = if keep == 0 {
                core::mem::take(q)
            } else {
                let mut c = q.cursor_mut();
                for _ in 1..keep {
                    c.move_next();
                }
                c.split_after()
            };
            // Unlinked from the tail, each to rmq6's head: rmq6 ends with them in the queue's
            // order before the expired ones.
            let mut lifo: OwnedSlist<Ip6qList> = OwnedSlist::new();
            while let Some(q6) = drained.pop_head() {
                ip6stat_inc(Ip6statCounters::Ip6sFragoverflow);
                *nfrags -= q6.ip6q_nfrag as u32;
                *nfragpackets -= 1;
                lifo.push_tail(q6);
            }
            lifo.concat(&mut rmq6);
            rmq6 = lifo;
        }
    }

    while let Some(q6) = rmq6.pop_head() {
        frag6_freef(q6);
    }
}

/// The reassembly queue forgotten for the host tests (its packets live in an earlier test's
/// memory: freeing them would free into it).
#[cfg(any(test, feature = "doubles"))]
pub fn frag6_test_reset() {
    let mut g = FRAG6.lock();
    core::mem::forget(core::mem::take(&mut g.q));
    g.nfrags = 0;
    g.nfragpackets = 0;
}

/// The host tests' probe of where a fragment's entry is freed: how many entries this thread
/// dropped, their packets with them, while it held `frag6_mutex` (the C frees some there, the
/// entries its concatenation consumes and a first fragment's datagram's too long ones, and
/// the others only after `mtx_leave`: `flushfrags`, the ECN drops).
#[cfg(test)]
pub(crate) mod freed_locked {
    std::thread_local! {
        static N: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
    }

    /// The count so far.
    pub(crate) fn get() -> u32 {
        N.with(core::cell::Cell::get)
    }

    /// One more.
    pub(super) fn bump() {
        N.with(|n| n.set(n.get() + 1));
    }
}

#[cfg(test)]
impl Drop for Ip6asfrag {
    fn drop(&mut self) {
        if FRAG6.owned() {
            freed_locked::bump();
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of IPv6 reassembly over kernel packets: LZ's (fragments in order and out of
    // order, an overlap, the atomic fragment of RFC 6946, the timer and the overflow of
    // `frag6_slowtimo`, `frag6_deletefraghdr`) and one crafted fragment per check of
    // `frag6_input`, each passing every earlier check, with the C's answer (the counter, the
    // ICMPv6 error and its pointer).

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::sync::netlock::NETLOCK;
    use hal::sys::netinet_icmp6::{
        ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER, ICMP6_TIME_EXCEED_REASSEMBLY, ICMP6_TIME_EXCEEDED,
    };
    use hal::sys::netinet_in::{IPPROTO_DSTOPTS, IPPROTO_FRAGMENT, IPPROTO_UDP};
    use hal::sys::netinet_ip6::IPV6_VERSION;
    use hal::sys::netinet6_in6::In6Addr;
    use hal::sys::socket::AF_INET6;

    use super::*;
    use crate::net::mbuf::{m_copydata, m_devget};
    use crate::net::route::tests::{take_icmp6_errors, take_icmp6_params};
    use crate::netinet6::ip6_var::IP6COUNTERS;

    fn stat(c: Ip6statCounters) -> u64 {
        IP6COUNTERS.read()[c as usize]
    }

    /// The routing tests' fixture, the pools made and the queue empty.
    fn setup6() -> MutexGuard<'static, ()> {
        let g = crate::net::route::tests::testroutes_setup();
        frag6_init();
        frag6_test_reset();
        let _ = take_icmp6_errors();
        let _ = take_icmp6_params();
        g
    }

    /// fd00::`last`.
    fn ula(last: u8) -> In6Addr {
        let mut a = [0u8; 16];
        a[0] = 0xfd;
        a[15] = last;
        In6Addr { s6_addr: a }
    }

    /// An IPv6 header from fd00::1 to fd00::2 (next header `nxt`, payload length `plen`)
    /// followed by `rest`, the traffic class `tclass`.
    fn ip6_bytes(nxt: i32, plen: u16, tclass: u8, rest: &[u8]) -> Vec<u8> {
        let mut ip6 = Ip6Hdr {
            ip6_flow: (u32::from(tclass) << 20).to_be(),
            ip6_plen: plen.to_be(),
            ip6_nxt: nxt as u8,
            ip6_hlim: 64,
            ip6_src: ula(1),
            ip6_dst: ula(2),
        };
        ip6.set_ip6_vfc(IPV6_VERSION);
        let mut b = vec![0u8; 40];
        assert!(pod::put(&mut b, 0, &ip6));
        b.extend_from_slice(rest);
        b
    }

    /// A fragment header (next UDP) of datagram `ident`, at byte offset `off`, more
    /// fragments after it if `more`.
    fn fh(ident: u32, off: u16, more: bool) -> Vec<u8> {
        let mut b = vec![IPPROTO_UDP as u8, 0];
        b.extend_from_slice(&(off | u16::from(more)).to_be_bytes());
        b.extend_from_slice(&ident.to_be_bytes());
        b
    }

    /// A fragment of datagram `ident` from fd00::1 to fd00::2: `data` at byte offset `off`,
    /// with the more-fragments bit `more` (the traffic class `tclass`).
    fn frag_tc(ident: u32, off: u16, more: bool, data: &[u8], tclass: u8) -> MbufBox {
        let mut rest = fh(ident, off, more);
        rest.extend_from_slice(data);
        let b = ip6_bytes(IPPROTO_FRAGMENT, rest.len() as u16, tclass, &rest);
        m_devget(&b, 0).expect("a packet")
    }

    fn frag(ident: u32, off: u16, more: bool, data: &[u8]) -> MbufBox {
        frag_tc(ident, off, more, data, 0)
    }

    /// `frag6_input` on `m` at offset `off` (its fragment header), under the shared net lock.
    fn input_at(m: MbufBox, off: i32) -> (i32, i32, Option<MbufBox>) {
        let r = NETLOCK.read();
        let net = NetHeld::of(&r).expect("the net lock");
        let mut mp = Some(m);
        let mut off = off;
        let nxt = frag6_input(
            &mut mp,
            &mut off,
            IPPROTO_FRAGMENT,
            i32::from(AF_INET6),
            None,
            &net,
        );
        (nxt, off, mp)
    }

    fn input(m: MbufBox) -> (i32, i32, Option<MbufBox>) {
        input_at(m, 40)
    }

    fn data(n: u8, len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| n.wrapping_mul(16).wrapping_add(i as u8))
            .collect()
    }

    fn bytes(m: &Mbuf) -> Vec<u8> {
        let mut b = vec![0u8; m.m_pkthdr().len.get() as usize];
        m_copydata(m, 0, &mut b);
        b
    }

    /// The counts of the queue: fragments and datagrams.
    fn counts() -> (u32, u32) {
        let g = FRAG6.lock();
        (g.nfrags(), g.nfragpackets())
    }

    /// Checks a reassembled datagram: UDP after the header, `payload` after it.
    fn check_reassembled(r: (i32, i32, Option<MbufBox>), payload: &[u8]) {
        let (nxt, off, mp) = r;
        assert_eq!(nxt, IPPROTO_UDP);
        assert_eq!(off, 40);
        let m = mp.expect("the reassembled datagram");
        let b = bytes(&m);
        assert_eq!(b.len(), 40 + payload.len());
        assert_eq!(i32::from(b[6]), IPPROTO_UDP, "the next header was restored");
        assert_eq!(usize::from(u16::from_be_bytes([b[4], b[5]])), payload.len());
        assert_eq!(&b[40..], payload);
    }

    #[test]
    fn fragments_in_order_are_reassembled() {
        let _g = setup6();
        let (a, b, c) = (data(1, 16), data(2, 16), data(3, 8));
        let reassembled = stat(Ip6statCounters::Ip6sReassembled);

        let r = input(frag(7, 0, true, &a));
        assert_eq!((r.0, r.2.is_none()), (IPPROTO_DONE, true), "queued");
        let r = input(frag(7, 16, true, &b));
        assert_eq!((r.0, r.2.is_none()), (IPPROTO_DONE, true), "queued");
        assert_eq!(counts(), (2, 1));
        let r = input(frag(7, 32, false, &c));
        check_reassembled(r, &[a, b, c].concat());
        assert_eq!(stat(Ip6statCounters::Ip6sReassembled), reassembled + 1);
        assert_eq!(counts(), (0, 0));
    }

    #[test]
    fn fragments_out_of_order_are_reassembled() {
        let _g = setup6();
        let (a, b, c) = (data(4, 24), data(5, 8), data(6, 3));

        assert_eq!(input(frag(9, 32, false, &c)).0, IPPROTO_DONE);
        assert_eq!(input(frag(9, 24, true, &b)).0, IPPROTO_DONE);
        // Other datagrams around it do not mix in, and keep their places.
        assert_eq!(input(frag(10, 8, true, &b)).0, IPPROTO_DONE);
        assert_eq!(input(frag(8, 8, true, &b)).0, IPPROTO_DONE);
        assert_eq!(counts(), (4, 3));
        assert_eq!(
            FRAG6.lock().idents(),
            [8u32.to_be(), 10u32.to_be(), 9u32.to_be()]
        );
        // A fragment of the datagram in the middle goes back in its place.
        assert_eq!(input(frag(10, 24, true, &b)).0, IPPROTO_DONE);
        assert_eq!(
            FRAG6.lock().idents(),
            [8u32.to_be(), 10u32.to_be(), 9u32.to_be()]
        );
        let r = input(frag(9, 0, true, &a));
        check_reassembled(r, &[a, b.clone(), c].concat());
        assert_eq!(counts(), (3, 2));
        assert_eq!(FRAG6.lock().idents(), [8u32.to_be(), 10u32.to_be()]);
    }

    #[test]
    fn an_overlap_discards_the_datagram() {
        let _g = setup6();
        // With the one before it, with the one after it, and with the head.
        for (first, second) in [((0, 16), (8, 16)), ((16, 16), (8, 16)), ((8, 8), (0, 16))] {
            frag6_test_reset();
            let dropped = stat(Ip6statCounters::Ip6sFragdropped);
            assert_eq!(
                input(frag(11, first.0, true, &data(1, first.1))).0,
                IPPROTO_DONE
            );
            if first.0 == 16 {
                assert_eq!(input(frag(11, 0, true, &data(1, 8))).0, IPPROTO_DONE);
            }
            let queued = counts().0;
            let (nxt, _, mp) = input(frag(11, second.0, true, &data(2, second.1)));
            assert_eq!(nxt, IPPROTO_DONE);
            assert!(mp.is_none());
            assert_eq!(
                stat(Ip6statCounters::Ip6sFragdropped),
                dropped + u64::from(queued) + 1,
                "the datagram and the new fragment: {first:?} {second:?}"
            );
            assert_eq!(counts(), (0, 0));
        }
        // Adjacent fragments do not overlap.
        frag6_test_reset();
        assert_eq!(input(frag(12, 0, true, &data(1, 16))).0, IPPROTO_DONE);
        assert_eq!(input(frag(12, 16, true, &data(1, 16))).0, IPPROTO_DONE);
        assert_eq!(counts(), (2, 1));
    }

    #[test]
    fn an_atomic_fragment_is_processed_alone() {
        let _g = setup6();
        let reassembled = stat(Ip6statCounters::Ip6sReassembled);

        let (nxt, off, mp) = input(frag(12, 0, false, &data(1, 8)));
        assert_eq!(nxt, IPPROTO_UDP);
        assert_eq!(off, 48, "past the fragment header");
        assert_eq!(stat(Ip6statCounters::Ip6sReassembled), reassembled + 1);
        assert_eq!(counts(), (0, 0));
        assert!(mp.is_some());
    }

    #[test]
    fn the_timer_expires_and_trims_the_queues() {
        let _g = setup6();
        let timeouts = stat(Ip6statCounters::Ip6sFragtimeout);

        // A later fragment alone (no ICMPv6 error for it when it expires).
        assert_eq!(input(frag(13, 16, true, &data(1, 8))).0, IPPROTO_DONE);
        for _ in 1..IPV6_FRAGTTL {
            frag6_slowtimo();
        }
        assert_eq!(counts(), (1, 1));
        frag6_slowtimo();
        assert_eq!(counts(), (0, 0));
        assert_eq!(stat(Ip6statCounters::Ip6sFragtimeout), timeouts + 1);
        assert!(
            take_icmp6_errors().is_empty(),
            "no first fragment, no error"
        );

        // A first fragment expired: time exceeded in reassembly.
        assert_eq!(input(frag(13, 0, true, &data(1, 8))).0, IPPROTO_DONE);
        for _ in 0..IPV6_FRAGTTL {
            frag6_slowtimo();
        }
        assert_eq!(
            take_icmp6_errors(),
            [(ICMP6_TIME_EXCEEDED, ICMP6_TIME_EXCEED_REASSEMBLY)]
        );

        // Three datagrams with the limit lowered to one: the two oldest go, from the tail.
        let overflow = stat(Ip6statCounters::Ip6sFragoverflow);
        for id in [14, 15, 16] {
            assert_eq!(input(frag(id, 16, true, &data(1, 8))).0, IPPROTO_DONE);
        }
        IP6_MAXFRAGPACKETS.store(1, Ordering::Relaxed);
        frag6_slowtimo();
        IP6_MAXFRAGPACKETS.store(200, Ordering::Relaxed);
        assert_eq!(stat(Ip6statCounters::Ip6sFragoverflow), overflow + 2);
        assert_eq!(counts(), (1, 1));
        assert_eq!(FRAG6.lock().idents(), [16u32.to_be()], "the newest stays");
        // None over the limit: nothing drained.
        frag6_slowtimo();
        assert_eq!(counts(), (1, 1));
    }

    #[test]
    fn the_fragment_header_is_deleted() {
        let _g = setup6();
        let payload = data(3, 8);

        // In the first mbuf: moved in place.
        let mut m = frag(16, 0, false, &payload);
        frag6_deletefraghdr(&mut m, 40).expect("in the first mbuf");
        assert_eq!(m.m_len().get(), 48);
        m_calchdrlen(&m);
        let b = bytes(&m);
        assert_eq!(&b[40..48], &payload[..]);
        assert_eq!(i32::from(b[6]), IPPROTO_FRAGMENT, "the header moved whole");

        // Past the first mbuf: split, the tail trimmed and put back.
        let mut m = frag(17, 0, false, &payload);
        let tail = m_split(&mut m, 44, M_DONTWAIT).expect("a split");
        drop(m.m_next.set(tail));
        m_calchdrlen(&m);
        assert_eq!(m.m_len().get(), 44);
        frag6_deletefraghdr(&mut m, 40).expect("split");
        m_calchdrlen(&m);
        let b = bytes(&m);
        assert_eq!(b.len(), 48);
        assert_eq!(&b[40..48], &payload[..]);
    }

    #[test]
    fn the_next_header_is_restored_after_the_unfragmentable_part() {
        let _g = setup6();
        // A destination options header (8 bytes, next the fragment header) before it.
        let dst = [IPPROTO_FRAGMENT as u8, 0, 1, 4, 0, 0, 0, 0];
        let mk = |off: u16, more: bool, d: &[u8]| {
            let mut rest = dst.to_vec();
            rest.extend_from_slice(&fh(21, off, more));
            rest.extend_from_slice(d);
            let b = ip6_bytes(IPPROTO_DSTOPTS, rest.len() as u16, 0, &rest);
            m_devget(&b, 0).expect("a packet")
        };
        let (a, b) = (data(1, 16), data(2, 5));
        assert_eq!(input_at(mk(0, true, &a), 48).0, IPPROTO_DONE);
        let (nxt, off, mp) = input_at(mk(16, false, &b), 48);
        assert_eq!((nxt, off), (IPPROTO_UDP, 48));
        let p = bytes(&mp.expect("reassembled"));
        assert_eq!(
            i32::from(p[6]),
            IPPROTO_DSTOPTS,
            "the IPv6 header's untouched"
        );
        assert_eq!(
            i32::from(p[40]),
            IPPROTO_UDP,
            "the destination options' restored"
        );
        assert_eq!(u16::from_be_bytes([p[4], p[5]]), 8 + 21);
        assert_eq!(&p[48..], &[a, b].concat()[..]);
    }

    #[test]
    fn ecn_bits_are_merged_or_the_fragment_dropped() {
        let _g = setup6();
        // CE on a later fragment of an ECT datagram: CE kept on the result.
        let (a, b) = (data(1, 8), data(2, 8));
        assert_eq!(input(frag_tc(30, 0, true, &a, 0x02)).0, IPPROTO_DONE);
        let (_, _, mp) = input(frag_tc(30, 8, false, &b, 0x03));
        let p = bytes(&mp.expect("reassembled"));
        let tc = (u32::from_be_bytes([p[0], p[1], p[2], p[3]]) >> 20) as u8;
        assert_eq!(tc & IPTOS_ECN_MASK, IPTOS_ECN_CE);

        // CE mixed with not-ECT, either way: the fragment dropped, the datagram kept.
        for (tc0, tc1) in [(0x00, 0x03), (0x02, 0x00)] {
            frag6_test_reset();
            let dropped = stat(Ip6statCounters::Ip6sFragdropped);
            assert_eq!(input(frag_tc(31, 0, true, &a, tc0)).0, IPPROTO_DONE);
            assert_eq!(input(frag_tc(31, 8, false, &b, tc1)).0, IPPROTO_DONE);
            assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped + 1);
            assert_eq!(counts(), (1, 1));
        }
    }

    #[test]
    fn flushed_and_ecn_dropped_fragments_are_freed_after_the_mutex() {
        let _g = setup6();
        let before = freed_locked::get();
        // An overlap: the datagram's two fragments, the new one and their packets freed once
        // the mutex is left (the C's flushfrags after mtx_leave), the three counted.
        assert_eq!(input(frag(40, 0, true, &data(1, 16))).0, IPPROTO_DONE);
        assert_eq!(input(frag(40, 16, true, &data(1, 16))).0, IPPROTO_DONE);
        assert_eq!(counts(), (2, 1));
        let dropped = stat(Ip6statCounters::Ip6sFragdropped);
        let (nxt, _, mp) = input(frag(40, 8, true, &data(2, 16)));
        assert_eq!(nxt, IPPROTO_DONE);
        assert!(mp.is_none());
        assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped + 3);
        assert_eq!(counts(), (0, 0));
        // CE on a not-ECT datagram: the new fragment's entry and packet freed after the mutex
        // (the C's pool_put after mtx_leave, then dropfrag), the datagram kept.
        let (a, b) = (data(1, 8), data(2, 8));
        assert_eq!(input(frag_tc(41, 0, true, &a, 0x00)).0, IPPROTO_DONE);
        assert_eq!(input(frag_tc(41, 8, false, &b, 0x03)).0, IPPROTO_DONE);
        assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped + 4);
        assert_eq!(counts(), (1, 1));
        // Not-ECT on an ECT datagram: the same.
        assert_eq!(input(frag_tc(43, 0, true, &a, 0x02)).0, IPPROTO_DONE);
        assert_eq!(input(frag_tc(43, 8, false, &b, 0x00)).0, IPPROTO_DONE);
        assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped + 5);
        assert_eq!(counts(), (2, 2));
        assert_eq!(
            freed_locked::get(),
            before,
            "nothing freed under frag6_mutex"
        );

        // The probe sees what the C itself frees under the mutex: the entries a reassembly
        // consumes (its concatenation's pool_put).
        frag6_test_reset();
        assert_eq!(input(frag(42, 0, true, &data(1, 8))).0, IPPROTO_DONE);
        let (nxt, _, mp) = input(frag(42, 8, false, &data(2, 8)));
        assert_eq!(nxt, IPPROTO_UDP);
        drop(mp);
        assert_eq!(
            freed_locked::get(),
            before + 2,
            "the two entries concatenated"
        );
    }

    /// One fragment per check of `frag6_input`, each passing the earlier ones: what the C
    /// answers (the ICMPv6 error and its pointer, or the counter that moves).
    #[test]
    fn each_check_of_the_input_answers_as_the_c() {
        let _g = setup6();
        let errors = || (take_icmp6_errors(), take_icmp6_params());
        let pp = (ICMP6_PARAM_PROB, ICMP6_PARAMPROB_HEADER);

        // A jumbo payload (plen 0) cannot carry a fragment header: the fragment header's
        // offset.
        let mut b = ip6_bytes(IPPROTO_FRAGMENT, 0, 0, &fh(40, 0, true));
        b.extend_from_slice(&data(1, 8));
        let r = input(m_devget(&b, 0).expect("a packet"));
        assert_eq!((r.0, r.2.is_none()), (IPPROTO_DONE, true));
        assert_eq!(errors(), (vec![pp], vec![40]));

        // More fragments and a length that is not a multiple of 8: the payload length.
        let r = input(frag(41, 0, true, &data(1, 7)));
        assert_eq!(r.0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![pp], vec![4]));
        // The last fragment may have any length.
        let r = input(frag(41, 8, false, &data(1, 7)));
        assert_eq!(r.0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![], vec![]));
        assert_eq!(counts(), (1, 1));
        frag6_test_reset();

        // An empty fragment that is not atomic: dropped, uncounted.
        let fragments = stat(Ip6statCounters::Ip6sFragments);
        let dropped = stat(Ip6statCounters::Ip6sFragdropped);
        let r = input(frag(42, 8, false, &[]));
        assert_eq!((r.0, r.2.is_none()), (IPPROTO_DONE, true));
        assert_eq!(stat(Ip6statCounters::Ip6sFragments), fragments + 1);
        assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped);
        assert_eq!(counts(), (0, 0));

        // net.inet6.ip6.maxfrags reached (0: never accept).
        IP6_MAXFRAGS.store(0, Ordering::Relaxed);
        let r = input(frag(43, 0, true, &data(1, 8)));
        IP6_MAXFRAGS.store(200, Ordering::Relaxed);
        assert_eq!(r.0, IPPROTO_DONE);
        assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped + 1);
        assert_eq!(counts(), (0, 0));

        // net.inet6.ip6.maxfragpackets reached: a new datagram dropped, a known one queued.
        assert_eq!(input(frag(44, 0, true, &data(1, 8))).0, IPPROTO_DONE);
        IP6_MAXFRAGPACKETS.store(1, Ordering::Relaxed);
        let r = input(frag(45, 0, true, &data(1, 8)));
        assert_eq!(r.0, IPPROTO_DONE);
        assert_eq!(stat(Ip6statCounters::Ip6sFragdropped), dropped + 2);
        assert_eq!(input(frag(44, 8, true, &data(1, 8))).0, IPPROTO_DONE);
        IP6_MAXFRAGPACKETS.store(200, Ordering::Relaxed);
        assert_eq!(counts(), (2, 1));
        frag6_test_reset();

        // Past IPV6_MAXPACKET without the first fragment (offset 65528 + 16 bytes): the
        // fragment header's offset and length field; its datagram's queue stays, empty.
        let r = input(frag(46, 65528, true, &data(1, 16)));
        assert_eq!(r.0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![pp], vec![42]));
        assert_eq!(counts(), (0, 1));
        // Exactly IPV6_MAXPACKET is accepted.
        let r = input(frag(47, 65528, false, &data(1, 7)));
        assert_eq!(r.0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![], vec![]));
        assert_eq!(counts(), (1, 2));
        frag6_test_reset();

        // With the first fragment known, its unfragmentable part counts (8 bytes of
        // destination options): 65520 + 8 + 8 + 8 > 65535 where 65520 + 8 + 7 fits.
        let dst = [IPPROTO_FRAGMENT as u8, 0, 1, 4, 0, 0, 0, 0];
        let mk = |off: u16, more: bool, d: &[u8]| {
            let mut rest = dst.to_vec();
            rest.extend_from_slice(&fh(48, off, more));
            rest.extend_from_slice(d);
            let b = ip6_bytes(IPPROTO_DSTOPTS, rest.len() as u16, 0, &rest);
            m_devget(&b, 0).expect("a packet")
        };
        assert_eq!(input_at(mk(0, true, &data(1, 8)), 48).0, IPPROTO_DONE);
        assert_eq!(input_at(mk(65520, true, &data(1, 8)), 48).0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![pp], vec![50]));
        assert_eq!(input_at(mk(65520, false, &data(1, 7)), 48).0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![], vec![]));
        frag6_test_reset();

        // A first fragment arriving after a later one it makes too long: the later one
        // answered (its own offset), the first queued.
        assert_eq!(input_at(mk(65520, false, &data(1, 8)), 48).0, IPPROTO_DONE);
        assert_eq!(input_at(mk(8, true, &data(1, 8)), 48).0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![], vec![]));
        assert_eq!(counts(), (2, 1));
        assert_eq!(input_at(mk(0, true, &data(1, 8)), 48).0, IPPROTO_DONE);
        assert_eq!(errors(), (vec![pp], vec![50]));
        assert_eq!(counts(), (2, 1), "the late one out, the first in");
    }

    /// `vmstat -m`'s `ip6q` and `ip6af` items against the C's (D-4: recorded, not matched).
    #[test]
    fn the_records_sizes_against_the_c() {
        // struct ip6q on LP64: TAILQ_ENTRY 16, LIST_HEAD 8, two in6_addr 32, four ints 16,
        // three u_int8_t, padded to 8.
        #[repr(C)]
        struct CIp6q {
            queue: [usize; 2],
            asfrag: usize,
            src: [u8; 16],
            dst: [u8; 16],
            unfrglen: i32,
            nfrag: i32,
            rdomain: u32,
            ident: u32,
            nxt: u8,
            ecn: u8,
            ttl: u8,
        }
        // struct ip6asfrag: LIST_ENTRY 16, a pointer, three ints, a u_int16_t.
        #[repr(C)]
        struct CIp6asfrag {
            list: [usize; 2],
            m: usize,
            offset: i32,
            frglen: i32,
            off: i32,
            mff: u16,
        }
        std::println!(
            "ip6q: C {}, kernel {}; ip6af: C {}, kernel {}",
            size_of::<CIp6q>(),
            size_of::<Ip6q>(),
            size_of::<CIp6asfrag>(),
            size_of::<Ip6asfrag>()
        );
        // C, LZ, kernel: ip6q 80, 80, 88; ip6af 40, 40, 40. LZ's (its Ip6q and Ip6asfrag over
        // bsd's TailqEntry 16, ListHead 8, ListEntry 16 and Cell members) measured once at
        // N6.5d's review by a replica in a bsd test, then removed (LZ's types are gone).
        assert_eq!(size_of::<CIp6q>(), 80);
        assert_eq!(size_of::<CIp6asfrag>(), 40);
        assert_eq!(size_of::<Ip6q>(), 88);
        assert_eq!(size_of::<Ip6asfrag>(), 40);
    }
}
/* </TESTS> */
