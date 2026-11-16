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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993, 1994
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
/* </LICENSES> */

/* <CODE> */
//! TCP's reassembly queue and selective acknowledgements: `netinet/tcp_input.c`'s
//! `tcp_reass` and `tcp_flush_queue`, the receiver's SACK report (`tcp_update_sack_list`,
//! `tcp_clean_sackreport`) and the sender's scoreboard of SACK holes (`tcp_sack_option`,
//! `tcp_del_sackholes`, `tcp_sack_partialack`).
//!
//! Upstream: sys/netinet/tcp_input.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_input.rs@3c62ede68802
//!
//! Out-of-order segments wait in the reassembly queue, sorted by sequence number, trimmed of
//! what their neighbours already hold; when the next expected segment arrives, the queue is
//! presented to the socket as far as the sequence space is complete. The receiver reports the
//! blocks it holds beyond `rcv_nxt` (RFC 2018); the sender keeps the holes the peer's blocks
//! leave, sorted, which the retransmissions fill.
//!
//! ## Deviations
//! - `tcp_reass` returns `Ok(TH_FIN or 0)` or `Err(ENOBUFS)` for the C's -1 (the segment was
//!   dropped for lack of a queue entry), LZ's choice; `tcp_sack_option` takes the option as a
//!   slice, its kind and length included (the C's pointer and count).
//! - The hole list is walked by a cursor on the link that owns the current hole, where the C
//!   keeps `p` (the previous hole) and `cur`: unlinking `cur` is the same step whether or not
//!   it is the first hole. After the walk the C appends a new hole after `p`, "the last hole
//!   on the list"; when the walk stopped early (`sack.end <= cur->start`) `p` is not the last
//!   hole, but the append cannot happen then (every hole lies below `rcv_lastsack`, so
//!   `rcv_lastsack < sack.start` fails): here the new hole goes in at the cursor, which is the
//!   list's end in every case the C reaches.
//!
//! ## Redesign
//! - The queue and the holes are owned chains (`hal::owned::SlistLink`: each link owns the
//!   next entry's "tcpqe" or "sackhl" item, D-13), walked forward with a cursor on the link
//!   that owns the current entry: an insert or a removal where the walk stands is a link
//!   change, O(1), as the C's `TAILQ_INSERT_AFTER` and `TAILQ_REMOVE` (LZ: `unsafe` list
//!   calls over pool items held as `&'static`). `TAILQ_LAST` with `TAILQ_REMOVE`, taken only
//!   when the "tcpqe" pool is exhausted (its hard limit), is a walk to the last entry, O(n)
//!   where the C is O(1): a named cost on a path the hard limit bounds (docs/ROADMAP.md's N12
//!   row). A queue entry owns its segment while queued (`tcpqe_m`, `Some` on the queue).

use core::cmp::min;

use hal::owned::SlistLink;
use hal::pool::{PR_NOWAIT, PoolBox};
use hal::sys::errno::Errno;
use hal::sys::netinet_tcp::{
    MAX_SACK_BLKS, TCP_SACKHOLE_LIMIT, TCPOLEN_SACK, TH_ACK, TH_FIN, TcpSeq, Tcphdr,
};

use crate::assert::kpanic;
use crate::net::mbuf::{MbufBox, m_adj};
use crate::net::sockbuf::SS_CANTRCVMORE;
use crate::net::socket::sorwakeup;
use crate::netinet::tcp_fsm::{TCPS_LISTEN, TCPS_SYN_RECEIVED, tcps_haveestablished};
use crate::netinet::tcp_seq::{seq_geq, seq_gt, seq_leq, seq_lt};
use crate::netinet::tcp_subr::{SACKHL_POOL, TCPQE_POOL, tcp_freeq};
use crate::netinet::tcp_timer::{TCPT_REXMT, tcp_timer_disarm};
use crate::netinet::tcp_var::{
    Sackblk, Sackhole, TF_NEEDOUTPUT, TcpState, Tcpqent, TcpstatCounters, Tp, tcpstat_inc,
    tcpstat_pkt,
};
use crate::proc::ProcGlue;

/// `tcprexmtthresh`.
pub const TCPREXMTTHRESH: i32 = 3;

/// `SEQ_MIN(a, b)`.
#[inline]
const fn seq_min(a: TcpSeq, b: TcpSeq) -> TcpSeq {
    if seq_lt(a, b) { a } else { b }
}

/// `SEQ_MAX(a, b)`.
#[inline]
const fn seq_max(a: TcpSeq, b: TcpSeq) -> TcpSeq {
    if seq_gt(a, b) { a } else { b }
}

/// The last entry of the queue, taken off it (`TAILQ_LAST` and `TAILQ_REMOVE`): a walk to the
/// link that owns it (see the Redesign).
fn take_last(head: &mut SlistLink<Tcpqent>) -> Option<PoolBox<Tcpqent>> {
    head.get()?;
    let mut link = head;
    while link.get().is_some_and(|e| e.tcpqe_q.is_some()) {
        link = match link.get_mut() {
            Some(e) => &mut e.tcpqe_q,
            None => kpanic(format_args!("tcp_reass: the queue changed under its walk")),
        };
    }
    link.take()
}

/// `tcp_reass`: insert segment `th` (host order, `m` its data) into the reassembly queue of
/// `tp`. Return `TH_FIN` if reassembly now includes a segment with FIN, or `ENOBUFS` (the C's
/// -1) if the input segment was discarded due to memory pressure. The common case (segment is
/// the next to be received on an established connection, and the queue is empty) is done
/// inline by `tcp_input`, avoiding linkage into and removal from the queue and repetition of
/// various conversions. Set DELACK for segments received in order, but ack immediately when
/// segments are out of order (so fast retransmit can work).
pub fn tcp_reass<G: ProcGlue>(
    tp: &mut Tp<'_, '_, G>,
    th: &mut Tcphdr,
    m: MbufBox,
    tlen: &mut i32,
) -> Result<u8, Errno> {
    let st = &mut *tp.st;

    // Allocate a new queue entry, before we throw away any data. If we can't, just drop the
    // packet. XXX
    let mut seg = Some(m);
    let fresh = TCPQE_POOL.get(PR_NOWAIT, || Tcpqent {
        tcpqe_q: SlistLink::new(),
        tcpqe_tcp: *th,
        tcpqe_m: seg.take(),
    });
    let mut tiqe = match fresh {
        Some(q) => q,
        None => {
            let Some(m) = seg.take() else {
                kpanic(format_args!("tcp_reass: the segment went with no entry"));
            };
            // Any SACK report for data we are about to discard is stale.
            if st.sack_enable != 0 && st.rcv_numsacks != 0 {
                tcp_clean_sackreport(st);
            }
            let last = if th.th_seq == st.rcv_nxt {
                // Reuse last entry since new segment fills a hole: its segment freed.
                take_last(&mut st.t_segq)
            } else {
                None
            };
            match last {
                Some(mut q) => {
                    q.tcpqe_m = Some(m);
                    q
                }
                None => {
                    // Flush segment queue for this connection
                    tcp_freeq(st);
                    tcpstat_inc(TcpstatCounters::TcpsRcvmemdrop);
                    drop(m);
                    return Err(Errno::ENOBUFS);
                }
            }
        }
    };

    // Find a segment which begins after this one does: `link` owns it, `phdr` is the header
    // of the one before it.
    let mut phdr: Option<Tcphdr> = None;
    let mut link = &mut st.t_segq;
    while link
        .get()
        .is_some_and(|q| !seq_gt(q.tcpqe_tcp.th_seq, th.th_seq))
    {
        link = match link.get_mut() {
            Some(q) => {
                phdr = Some(q.tcpqe_tcp);
                &mut q.tcpqe_q
            }
            None => kpanic(format_args!("tcp_reass: the queue changed under its walk")),
        };
    }

    // If there is a preceding segment, it may provide some of our data already. If so, drop
    // the data from the incoming segment. If it provides all of our data, drop us.
    if let Some(phdr) = phdr {
        // conversion to int (in i) handles seq wraparound
        let i = phdr
            .th_seq
            .wrapping_add(u32::from(phdr.th_reseqlen()))
            .wrapping_sub(th.th_seq) as i32;
        if i > 0 {
            if i >= *tlen {
                tcpstat_pkt(
                    TcpstatCounters::TcpsRcvduppack,
                    TcpstatCounters::TcpsRcvdupbyte,
                    *tlen as u64,
                );
                // m_freem(m) and pool_put(tiqe): the entry and its segment go together.
                drop(tiqe);
                return Ok(0);
            }
            if let Some(m) = &tiqe.tcpqe_m {
                m_adj(m, i);
            }
            *tlen -= i;
            th.th_seq = th.th_seq.wrapping_add(i as u32);
        }
    }
    tcpstat_pkt(
        TcpstatCounters::TcpsRcvoopack,
        TcpstatCounters::TcpsRcvoobyte,
        *tlen as u64,
    );
    st.t_rcvoopack = st.t_rcvoopack.wrapping_add(1);

    // While we overlap succeeding segments trim them or, if they are completely covered,
    // dequeue them.
    loop {
        let Some(q) = link.get_mut() else {
            break;
        };
        let qhdr = &mut q.tcpqe_tcp;
        let i = th
            .th_seq
            .wrapping_add(*tlen as u32)
            .wrapping_sub(qhdr.th_seq) as i32;

        if i <= 0 {
            break;
        }
        if i < i32::from(qhdr.th_reseqlen()) {
            qhdr.th_seq = qhdr.th_seq.wrapping_add(i as u32);
            qhdr.set_th_reseqlen(qhdr.th_reseqlen() - i as u16);
            if let Some(m) = &q.tcpqe_m {
                m_adj(m, i);
            }
            break;
        }
        // Covered: off the queue where it stands, its segment freed with it.
        if let Some(mut gone) = link.take()
            && let Some(next) = gone.tcpqe_q.take()
        {
            let _ = link.set(next);
        }
    }

    // Insert the new segment queue entry into place.
    th.set_th_reseqlen(*tlen as u16);
    tiqe.tcpqe_tcp = *th;
    if let Some(rest) = link.take() {
        let _ = tiqe.tcpqe_q.set(rest);
    }
    let _ = link.set(tiqe);

    if th.th_seq != st.rcv_nxt {
        return Ok(0);
    }

    Ok(tcp_flush_queue(tp))
}

/// `tcp_flush_queue`: present data to user, advancing `rcv_nxt` through completed sequence
/// space; `TH_FIN` when the last segment presented carries a FIN.
fn tcp_flush_queue<G: ProcGlue>(tp: &mut Tp<'_, '_, G>) -> u8 {
    let so = tp.so;
    let st = &mut *tp.st;

    if !tcps_haveestablished(st.t_state) {
        return 0;
    }
    match st.t_segq.get() {
        Some(q) if q.tcpqe_tcp.th_seq == st.rcv_nxt => {
            if st.t_state == TCPS_SYN_RECEIVED && q.tcpqe_tcp.th_reseqlen() != 0 {
                return 0;
            }
        }
        _ => return 0,
    }
    let mut flags;
    loop {
        let Some(mut q) = st.t_segq.take() else {
            kpanic(format_args!(
                "tcp_flush_queue: the queue changed under its walk"
            ));
        };
        if let Some(next) = q.tcpqe_q.take() {
            let _ = st.t_segq.set(next);
        }
        let hdr = q.tcpqe_tcp;
        st.rcv_nxt = st.rcv_nxt.wrapping_add(u32::from(hdr.th_reseqlen()));
        flags = hdr.th_flags & TH_FIN;

        if let Some(m) = q.tcpqe_m.take() {
            if so.so_rcv.has_state(SS_CANTRCVMORE) {
                drop(m);
            } else {
                so.so_rcv.lock().sbappendstream(m);
            }
        }
        // pool_put(&tcpqe_pool, q)
        drop(q);
        if flags != 0
            || !st
                .t_segq
                .get()
                .is_some_and(|n| n.tcpqe_tcp.th_seq == st.rcv_nxt)
        {
            break;
        }
    }
    sorwakeup(so);
    flags
}

/// `tcp_seq_subtract`: `a - b` of two sequence numbers widened to `u_long`, wrapping (the C's
/// `(long)(a - b)` returned as `u_long`).
#[inline]
pub fn tcp_seq_subtract(a: u64, b: u64) -> u64 {
    a.wrapping_sub(b)
}

/// `tcp_update_sack_list`: called upon receipt of new valid data (while not in header
/// prediction mode), it updates the ordered list of sacks.
pub fn tcp_update_sack_list(st: &mut TcpState, rcv_laststart: TcpSeq, rcv_lastend: TcpSeq) {
    // First reported block MUST be the most recent one. Subsequent blocks SHOULD be in the
    // order in which they arrived at the receiver. These two conditions make the
    // implementation fully compliant with RFC 2018.
    let mut j = 0usize;
    let mut count = 0;
    let mut lastpos: i32 = -1;
    let mut temp = [Sackblk::default(); MAX_SACK_BLKS];
    let zero = Sackblk { start: 0, end: 0 };

    // First clean up current list of sacks
    for i in 0..st.rcv_numsacks as usize {
        let sack = st.sackblks[i];
        if sack.start == 0 && sack.end == 0 {
            count += 1; // count = number of blocks to be discarded
            continue;
        }
        if seq_leq(sack.end, st.rcv_nxt) {
            st.sackblks[i] = zero;
            count += 1;
        } else {
            temp[j] = sack;
            j += 1;
        }
    }
    st.rcv_numsacks -= count;
    if st.rcv_numsacks == 0 {
        // no sack blocks currently (fast path)
        tcp_clean_sackreport(st);
        if seq_lt(st.rcv_nxt, rcv_laststart) {
            // ==> need first sack block
            st.sackblks[0] = Sackblk {
                start: rcv_laststart,
                end: rcv_lastend,
            };
            st.rcv_numsacks = 1;
        }
        return;
    }
    // Otherwise, sack blocks are already present.
    let numsacks = st.rcv_numsacks as usize;
    st.sackblks[..numsacks].copy_from_slice(&temp[..numsacks]); // first copy back sack list
    if seq_geq(st.rcv_nxt, rcv_lastend) {
        return; // sack list remains unchanged
    }
    // From here, segment just received should be (part of) the 1st sack. Go through list,
    // possibly coalescing sack block entries.
    let mut firstsack = Sackblk {
        start: rcv_laststart,
        end: rcv_lastend,
    };
    for i in 0..numsacks {
        let sack = st.sackblks[i];
        if seq_lt(sack.end, firstsack.start) || seq_gt(sack.start, firstsack.end) {
            continue; // no overlap
        }
        if sack.start == firstsack.start && sack.end == firstsack.end {
            // identical block; delete it here since we will move it to the front of the
            // list.
            st.sackblks[i] = zero;
            lastpos = i as i32; // last posn with a zero entry
            continue;
        }
        if seq_leq(sack.start, firstsack.start) {
            firstsack.start = sack.start; // merge blocks
        }
        if seq_geq(sack.end, firstsack.end) {
            firstsack.end = sack.end; // merge blocks
        }
        st.sackblks[i] = zero;
        lastpos = i as i32; // last posn with a zero entry
    }
    if lastpos != -1 {
        // at least one merge
        let mut j = 1usize;
        for i in 0..numsacks {
            let sack = st.sackblks[i];
            if sack.start == 0 && sack.end == 0 {
                continue;
            }
            temp[j] = sack;
            j += 1;
        }
        st.rcv_numsacks = j as i32; // including first blk (added later)
        st.sackblks[1..j].copy_from_slice(&temp[1..j]); // now copy back
    } else {
        // no merges -- shift sacks by 1
        if (st.rcv_numsacks as usize) < MAX_SACK_BLKS {
            st.rcv_numsacks += 1;
        }
        for i in (1..st.rcv_numsacks as usize).rev() {
            st.sackblks[i] = st.sackblks[i - 1];
        }
    }
    st.sackblks[0] = firstsack;
}

/// What the hole walk of [`tcp_sack_option`] does with the current hole after looking at it.
enum HoleStep {
    /// No use going through more holes.
    Stop,
    /// The hole stays, changed or not: the walk goes on past it.
    Next,
    /// The block covers the hole: it leaves the list where it stands.
    Delete,
    /// The block falls in the middle of the hole: the second half follows it.
    Split(PoolBox<Sackhole>),
    /// No hole to be had for the split: the block is dropped (`goto dropped`).
    Dropped,
}

/// `tcp_sack_option`: process the TCP SACK option `cp` (kind and length included).
/// `tp->snd_holes` is an ordered list of holes (oldest to newest, in terms of the sequence
/// space).
pub fn tcp_sack_option(st: &mut TcpState, th: &Tcphdr, cp: &[u8]) {
    let optlen = cp.len();

    if st.sack_enable == 0 {
        return;
    }
    // SACK without ACK doesn't make sense.
    if th.th_flags & TH_ACK == 0 {
        return;
    }
    // Make sure the ACK on this segment is in [snd_una, snd_max].
    if seq_lt(th.th_ack, st.snd_una) || seq_gt(th.th_ack, st.snd_max) {
        return;
    }
    // Note: TCPOLEN_SACK must be 2*sizeof(tcp_seq)
    let sacklen = usize::from(TCPOLEN_SACK);
    if optlen <= 2 || !(optlen - 2).is_multiple_of(sacklen) {
        return;
    }
    // Note: TCPOLEN_SACK must be 2*sizeof(tcp_seq)
    tcpstat_inc(TcpstatCounters::TcpsSackRcvOpts);
    if st.snd_numholes < 0 {
        st.snd_numholes = 0;
    }
    if st.t_maxseg == 0 {
        kpanic(format_args!("tcp_sack_option")); // Should never happen
    }
    let maxseg = u32::from(st.t_maxseg);
    let thresh = TCPREXMTTHRESH as u32;
    // dups past the threshold: `((sack.end - hole.end) / t_maxseg) >= tcprexmtthresh`.
    let clamp = |h: &mut Sackhole, end: TcpSeq| {
        if end.wrapping_sub(h.end) / maxseg >= thresh {
            h.dups = TCPREXMTTHRESH;
        }
    };
    for blk in cp[2..].chunks_exact(sacklen) {
        let sack = Sackblk {
            start: u32::from_be_bytes([blk[0], blk[1], blk[2], blk[3]]),
            end: u32::from_be_bytes([blk[4], blk[5], blk[6], blk[7]]),
        };
        if seq_leq(sack.end, sack.start) {
            continue; // bad SACK fields
        }
        if seq_leq(sack.end, st.snd_una) {
            continue; // old block
        }
        if seq_gt(th.th_ack, st.snd_una) && seq_lt(sack.start, th.th_ack) {
            continue;
        }
        if seq_gt(sack.end, st.snd_max) {
            continue;
        }
        if seq_lt(sack.start, st.snd_una) {
            continue;
        }
        if !st.snd_holes.is_some() {
            // first hole
            // dups is at least one. If more data has been SACKed, it can be greater than
            // one.
            let dups = min(thresh, sack.end.wrapping_sub(sack.start) / maxseg) as i32;
            let Some(cur) = SACKHL_POOL.get(PR_NOWAIT, || {
                Sackhole::new(th.th_ack, sack.start, dups.max(1), th.th_ack)
            }) else {
                // ENOBUFS, so ignore SACKed block for now
                tcpstat_inc(TcpstatCounters::TcpsSackDropOpts);
                return;
            };
            let _ = st.snd_holes.set(cur);
            st.snd_numholes = 1;
            st.rcv_lastsack = sack.end;
            continue; // with next sack block
        }
        // Go thru list of holes: `link` owns the current one; `advanced` tells whether a hole
        // was passed (the C's `p` is then the one before `cur`, else `cur` itself).
        let numholes = &mut st.snd_numholes;
        let mut link = &mut st.snd_holes;
        let mut advanced = false;
        loop {
            let step = match link.get_mut() {
                None => break,
                Some(cur) => {
                    if seq_leq(sack.end, cur.start) {
                        // SACKs data before the current hole
                        HoleStep::Stop
                    } else if seq_geq(sack.start, cur.end) {
                        // SACKs data beyond the current hole
                        cur.dups += 1;
                        clamp(cur, sack.end);
                        HoleStep::Next
                    } else if seq_leq(sack.start, cur.start) {
                        // Data acks at least the beginning of hole
                        if seq_geq(sack.end, cur.end) {
                            // Acks entire hole, so delete hole
                            HoleStep::Delete
                        } else {
                            // otherwise, move start of hole forward
                            cur.start = sack.end;
                            cur.rxmit = seq_max(cur.rxmit, cur.start);
                            HoleStep::Next
                        }
                    } else if seq_geq(sack.end, cur.end) {
                        // move end of hole backward
                        cur.end = sack.start;
                        cur.rxmit = seq_min(cur.rxmit, cur.end);
                        cur.dups += 1;
                        clamp(cur, sack.end);
                        HoleStep::Next
                    } else {
                        // seq_lt(cur.start, sack.start) && seq_gt(cur.end, sack.end): ACKs
                        // some data in middle of a hole; need to split current hole
                        if *numholes >= TCP_SACKHOLE_LIMIT {
                            HoleStep::Dropped
                        } else {
                            let start = sack.end;
                            let rxmit = seq_max(cur.rxmit, start);
                            match SACKHL_POOL
                                .get(PR_NOWAIT, || Sackhole::new(start, cur.end, cur.dups, rxmit))
                            {
                                None => HoleStep::Dropped, // ENOBUFS
                                Some(temp) => {
                                    cur.end = sack.start;
                                    cur.rxmit = seq_min(cur.rxmit, cur.end);
                                    cur.dups += 1;
                                    clamp(cur, sack.end);
                                    HoleStep::Split(temp)
                                }
                            }
                        }
                    }
                }
            };
            match step {
                HoleStep::Stop => break,
                HoleStep::Dropped => {
                    tcpstat_inc(TcpstatCounters::TcpsSackDropOpts);
                    return;
                }
                HoleStep::Next => {
                    link = match link.get_mut() {
                        Some(cur) => &mut cur.next,
                        None => kpanic(format_args!("tcp_sack_option: a hole went")),
                    };
                    advanced = true;
                }
                HoleStep::Delete => {
                    if let Some(mut gone) = link.take()
                        && let Some(next) = gone.next.take()
                    {
                        let _ = link.set(next);
                    }
                    *numholes -= 1;
                }
                HoleStep::Split(mut temp) => {
                    link = match link.get_mut() {
                        Some(cur) => {
                            if let Some(rest) = cur.next.take() {
                                let _ = temp.next.set(rest);
                            }
                            let _ = cur.next.set(temp);
                            match cur.next.get_mut() {
                                Some(t) => &mut t.next,
                                None => kpanic(format_args!("tcp_sack_option: a hole went")),
                            }
                        }
                        None => kpanic(format_args!("tcp_sack_option: a hole went")),
                    };
                    *numholes += 1;
                    advanced = true;
                }
            }
        }
        // At this point, p points to the last hole on the list: the cursor is past it (see
        // the deviations); `p` is NULL when every hole was deleted.
        let p_exists = advanced || link.is_some();
        if p_exists && seq_lt(st.rcv_lastsack, sack.start) {
            // Need to append new hole at end. Last hole is p (and it's not NULL).
            if *numholes >= TCP_SACKHOLE_LIMIT {
                tcpstat_inc(TcpstatCounters::TcpsSackDropOpts);
                return;
            }
            let start = st.rcv_lastsack;
            let dups = min(thresh, sack.end.wrapping_sub(sack.start) / maxseg) as i32;
            let Some(mut temp) = SACKHL_POOL.get(PR_NOWAIT, || {
                Sackhole::new(start, sack.start, dups.max(1), start)
            }) else {
                tcpstat_inc(TcpstatCounters::TcpsSackDropOpts); // ENOBUFS
                return;
            };
            if let Some(rest) = link.take() {
                let _ = temp.next.set(rest);
            }
            let _ = link.set(temp);
            st.rcv_lastsack = sack.end;
            *numholes += 1;
        }
    }
}

/// `tcp_del_sackholes`: delete stale (i.e, cumulatively ack'd) holes. Hole is deleted only if
/// it is completely acked; otherwise, `tcp_sack_option()`, called from `tcp_dooptions()`,
/// will fix up the hole.
pub fn tcp_del_sackholes(st: &mut TcpState, th: &Tcphdr) {
    if st.sack_enable != 0 && st.t_state != TCPS_LISTEN {
        // max because this could be an older ack just arrived
        let lastack = if seq_gt(th.th_ack, st.snd_una) {
            th.th_ack
        } else {
            st.snd_una
        };
        while let Some(cur) = st.snd_holes.get_mut() {
            if seq_leq(cur.end, lastack) {
                if let Some(mut prev) = st.snd_holes.take()
                    && let Some(next) = prev.next.take()
                {
                    let _ = st.snd_holes.set(next);
                }
                st.snd_numholes -= 1;
            } else {
                if seq_lt(cur.start, lastack) {
                    cur.start = lastack;
                    if seq_lt(cur.rxmit, cur.start) {
                        cur.rxmit = cur.start;
                    }
                }
                break;
            }
        }
    }
}

/// `tcp_clean_sackreport`: delete all receiver-side SACK information.
pub fn tcp_clean_sackreport(st: &mut TcpState) {
    st.rcv_numsacks = 0;
    st.sackblks = [Sackblk { start: 0, end: 0 }; MAX_SACK_BLKS];
}

/// `tcp_sack_partialack`: partial ack handling within a sack recovery episode. When a partial
/// ack arrives, turn off retransmission timer, deflate the window, do not clear
/// `tp->t_dupacks`.
pub fn tcp_sack_partialack<G: ProcGlue>(tp: &mut Tp<'_, '_, G>, th: &Tcphdr) {
    // Turn off retx. timer (will start again next segment)
    tcp_timer_disarm(tp, TCPT_REXMT);
    let st = &mut *tp.st;
    st.t_rtttime = 0;
    // Partial window deflation. This statement relies on the fact that tp->snd_una has not
    // been updated yet.
    let acked = u64::from(th.th_ack.wrapping_sub(st.snd_una));
    let maxseg = u64::from(st.t_maxseg);
    if st.snd_cwnd > acked {
        st.snd_cwnd -= acked;
        st.snd_cwnd += maxseg;
    } else {
        st.snd_cwnd = maxseg;
    }
    st.snd_cwnd += maxseg;
    st.set_flags(TF_NEEDOUTPUT);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the reassembly queue and the SACK scoreboard: LZ's tables over the owned
    // chains, the C's orders and overlaps, the pools back at their baselines.

    use std::vec::Vec;

    use hal::sys::netinet_tcp::{TCPOPT_SACK, TH_PUSH};

    use super::*;
    use crate::net::mbuf::{M_DONTWAIT, m_gethdr};
    use crate::net::sockbuf::SS_CANTRCVMORE;
    use crate::netinet::tcp_fsm::TCPS_ESTABLISHED;
    use crate::netinet::tcp_subr::testtcp;

    /// A header with `flags`, `seq` and `ack`, host order.
    fn th(flags: u8, seq: TcpSeq, ack: TcpSeq) -> Tcphdr {
        Tcphdr {
            th_flags: flags,
            th_seq: seq,
            th_ack: ack,
            ..Tcphdr::default()
        }
    }

    /// The receiver's SACK blocks in use.
    fn blocks(st: &TcpState) -> Vec<(TcpSeq, TcpSeq)> {
        st.sackblks[..st.rcv_numsacks as usize]
            .iter()
            .map(|b| (b.start, b.end))
            .collect()
    }

    #[test]
    fn update_sack_list_keeps_the_newest_block_first() {
        let mut st = TcpState::new();
        st.rcv_nxt = 1000;

        // In-order data needs no block.
        tcp_update_sack_list(&mut st, 1000, 1100);
        assert_eq!(blocks(&st), []);

        tcp_update_sack_list(&mut st, 2000, 2100);
        assert_eq!(blocks(&st), [(2000, 2100)]);
        tcp_update_sack_list(&mut st, 3000, 3100);
        assert_eq!(blocks(&st), [(3000, 3100), (2000, 2100)]);
        // Adjacent to the older block: merged, and moved to the front.
        tcp_update_sack_list(&mut st, 2100, 2200);
        assert_eq!(blocks(&st), [(2000, 2200), (3000, 3100)]);
        // Once rcv_nxt passes a block it is dropped.
        st.rcv_nxt = 2200;
        tcp_update_sack_list(&mut st, 4000, 4100);
        assert_eq!(blocks(&st), [(4000, 4100), (3000, 3100)]);
        // A repeated block moves to the front without growing the list.
        tcp_update_sack_list(&mut st, 3000, 3100);
        assert_eq!(blocks(&st), [(3000, 3100), (4000, 4100)]);

        // At most MAX_SACK_BLKS are kept: the oldest falls off.
        for i in 0..10u32 {
            tcp_update_sack_list(&mut st, 10000 + 200 * i, 10100 + 200 * i);
        }
        assert_eq!(blocks(&st).len(), MAX_SACK_BLKS);
        assert_eq!(blocks(&st)[0], (11800, 11900));

        tcp_clean_sackreport(&mut st);
        assert_eq!(blocks(&st), []);
        assert!(st.sackblks.iter().all(|b| *b == Sackblk::default()));
    }

    /// The sender's holes, in list order.
    fn holes(st: &TcpState) -> Vec<(TcpSeq, TcpSeq, i32, TcpSeq)> {
        let mut v = Vec::new();
        let mut cur = st.snd_holes.get();
        while let Some(h) = cur {
            v.push((h.start, h.end, h.dups, h.rxmit));
            cur = h.next.get();
        }
        v
    }

    /// A SACK option holding `blocks`.
    fn sack_option(blocks: &[(TcpSeq, TcpSeq)]) -> Vec<u8> {
        let mut o = std::vec![TCPOPT_SACK, (2 + 8 * blocks.len()) as u8];
        for &(s, e) in blocks {
            o.extend_from_slice(&s.to_be_bytes());
            o.extend_from_slice(&e.to_be_bytes());
        }
        o
    }

    #[test]
    fn sack_option_tracks_the_holes() {
        let (_g, _p) = testtcp::setup();
        let out = SACKHL_POOL.nout();
        let mut st = TcpState::new();
        st.sack_enable = 1;
        st.t_state = TCPS_ESTABLISHED;
        st.snd_una = 1000;
        st.snd_max = 10000;
        st.t_maxseg = 100;
        let ack = th(TH_ACK, 1, 1000);

        // A SACK without ACK, or with a malformed length, is ignored.
        tcp_sack_option(&mut st, &th(0, 1, 1000), &sack_option(&[(2000, 3000)]));
        tcp_sack_option(&mut st, &ack, &sack_option(&[(2000, 3000)])[..9]);
        // So is a block outside [snd_una, snd_max] or inverted.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(3000, 2000), (900, 950)]));
        tcp_sack_option(&mut st, &ack, &sack_option(&[(9000, 10001)]));
        assert_eq!(holes(&st), []);

        // The first block opens a hole from the ACK up to it.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(2000, 3000)]));
        assert_eq!(holes(&st), [(1000, 2000, 3, 1000)]);
        assert_eq!(st.rcv_lastsack, 3000);
        // A block past the last one appends a hole; the hole before it counts one more
        // dup, capped at tcprexmtthresh.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(4000, 4100)]));
        assert_eq!(holes(&st), [(1000, 2000, 3, 1000), (3000, 4000, 1, 3000)]);
        assert_eq!(st.snd_numholes, 2);
        // The beginning of the first hole arrives, and the middle of the second splits it.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(1000, 1500), (3400, 3600)]));
        assert_eq!(
            holes(&st),
            [
                (1500, 2000, 3, 1500),
                (3000, 3400, 2, 3000),
                (3600, 4000, 1, 3600)
            ]
        );
        assert_eq!(st.snd_numholes, 3);
        // The end of a hole: it shrinks from above.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(3900, 4000)]));
        assert_eq!(
            holes(&st),
            [
                (1500, 2000, 3, 1500),
                (3000, 3400, 3, 3000),
                (3600, 3900, 2, 3600)
            ]
        );
        // A block covering a whole hole deletes it (the walk stops at the next hole, which
        // lies above the block: no dup for it); one covering the first, the head.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(3000, 3400)]));
        assert_eq!(holes(&st), [(1500, 2000, 3, 1500), (3600, 3900, 2, 3600)]);
        tcp_sack_option(&mut st, &ack, &sack_option(&[(1500, 2000)]));
        assert_eq!(holes(&st), [(3600, 3900, 2, 3600)]);
        assert_eq!(st.snd_numholes, 1);

        // A cumulative ACK deletes the holes below it and trims the one it falls into.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(5000, 5100)]));
        tcp_del_sackholes(&mut st, &th(TH_ACK, 1, 3700));
        assert_eq!(holes(&st), [(3700, 3900, 3, 3700), (4100, 5000, 1, 4100)]);
        tcp_del_sackholes(&mut st, &th(TH_ACK, 1, 5000));
        assert_eq!(holes(&st), []);
        assert_eq!(st.snd_numholes, 0);
        assert_eq!(SACKHL_POOL.nout(), out, "every hole back in its pool");
    }

    #[test]
    fn a_split_past_the_hole_limit_is_dropped() {
        let (_g, _p) = testtcp::setup();
        let mut st = TcpState::new();
        st.sack_enable = 1;
        st.snd_una = 1000;
        st.snd_max = 100_000;
        st.t_maxseg = 100;
        let ack = th(TH_ACK, 1, 1000);
        tcp_sack_option(&mut st, &ack, &sack_option(&[(50_000, 50_100)]));
        st.snd_numholes = TCP_SACKHOLE_LIMIT;
        let drops =
            crate::netinet::tcp_var::TCPCOUNTERS.read()[TcpstatCounters::TcpsSackDropOpts as usize];
        // A block in the middle of the hole would split it: refused at the limit.
        tcp_sack_option(&mut st, &ack, &sack_option(&[(2000, 3000)]));
        assert_eq!(holes(&st), [(1000, 50_000, 1, 1000)]);
        let after =
            crate::netinet::tcp_var::TCPCOUNTERS.read()[TcpstatCounters::TcpsSackDropOpts as usize];
        assert!(after > drops, "tcps_sack_drop_opts");
        crate::netinet::tcp_timer::tcp_timer_freesack(&mut st);
    }

    /// A segment of `len` bytes, each its sequence number's low byte.
    fn data(seq: TcpSeq, len: usize) -> MbufBox {
        let mut m = m_gethdr(M_DONTWAIT, crate::net::mbuf::MT_DATA).expect("an mbuf");
        m.m_len().set(len as u32);
        for (i, b) in m.data_mut().expect("writable").iter_mut().enumerate() {
            *b = seq.wrapping_add(i as u32) as u8;
        }
        m.m_pkthdr().len.set(len as i32);
        m
    }

    /// The bytes the socket's receive buffer holds.
    fn received(inp: &crate::netinet::in_pcb::Inpcb<testtcp::G>) -> Vec<u8> {
        let so = inp.socket();
        let n = so.so_rcv.cc() as usize;
        let g = so.so_rcv.lock();
        let mut v = std::vec![0u8; n];
        g.copydata_first(0, &mut v);
        v
    }

    #[test]
    fn reassembly_of_overlapping_segments_yields_the_stream() {
        let (_g, _p) = testtcp::setup();
        let qe = TCPQE_POOL.nout();
        let (so, inp) = testtcp::conn();
        // rcv_nxt 1000; segments [1100, 1200), [1150, 1250) (overlaps the first's end),
        // [1050, 1120) (overlaps its start), [1000, 1060) (presents [1000, 1250)), then
        // [1250, 1340) with a FIN (each fits a header mbuf; tcp_input trims what lies below
        // rcv_nxt before tcp_reass).
        let segs: [(u32, usize, u8); 5] = [
            (1100, 100, 0),
            (1150, 100, 0),
            (1050, 70, TH_PUSH),
            (1000, 60, 0),
            (1250, 90, TH_FIN),
        ];
        let got = testtcp::with_tp(&inp, |mut tp| {
            tp.st.t_state = TCPS_ESTABLISHED;
            tp.st.rcv_nxt = 1000;
            let mut fins = Vec::new();
            for (seq, len, flags) in segs {
                let mut h = th(flags, seq, 0);
                let mut tlen = len as i32;
                let r = tcp_reass(&mut tp, &mut h, data(seq, len), &mut tlen);
                fins.push(r);
            }
            ((fins, tp.st.rcv_nxt, tp.st.t_segq.is_some()), None)
        });
        let (fins, rcv_nxt, queued) = got;
        assert_eq!(fins, [Ok(0), Ok(0), Ok(0), Ok(0), Ok(TH_FIN)]);
        assert_eq!(rcv_nxt, 1340);
        assert!(!queued, "the queue is empty once the stream is whole");
        let want: Vec<u8> = (1000u32..1340).map(|s| s as u8).collect();
        assert_eq!(received(&inp), want);
        assert_eq!(TCPQE_POOL.nout(), qe);

        // A duplicate is dropped whole, counted, and no entry stays out.
        let dup = testtcp::with_tp(&inp, |mut tp| {
            let mut h = th(0, 1500, 0);
            let mut tlen = 50;
            let _ = tcp_reass(&mut tp, &mut h, data(1500, 50), &mut tlen);
            let mut h = th(0, 1500, 0);
            let mut tlen = 50;
            let r = tcp_reass(&mut tp, &mut h, data(1500, 50), &mut tlen);
            (r, None)
        });
        assert_eq!(dup, Ok(0));
        assert_eq!(TCPQE_POOL.nout(), qe + 1);
        // tcp_close frees the queue.
        testtcp::close(so, inp);
        assert_eq!(TCPQE_POOL.nout(), qe);
    }

    #[test]
    fn reassembly_waits_and_drops_as_the_state_says() {
        let (_g, _p) = testtcp::setup();
        let (so, inp) = testtcp::conn();
        let held = testtcp::with_tp(&inp, |mut tp| {
            // In SYN_RECEIVED, data waits in the queue even at rcv_nxt.
            tp.st.t_state = TCPS_SYN_RECEIVED;
            tp.st.rcv_nxt = 1;
            let mut h = th(0, 1, 0);
            let mut tlen = 10;
            let r = tcp_reass(&mut tp, &mut h, data(1, 10), &mut tlen);
            ((r, tp.st.rcv_nxt, tp.st.t_segq.is_some()), None)
        });
        assert_eq!(held, (Ok(0), 1, true));
        // A socket that cannot receive more drops what the queue presents (tcp_input
        // flushes the queue once the connection is established).
        inp.socket().so_rcv.lock().set_state(SS_CANTRCVMORE);
        let r = testtcp::with_tp(&inp, |mut tp| {
            tp.st.t_state = TCPS_ESTABLISHED;
            let mut h = th(0, 11, 0);
            let mut tlen = 5;
            let r = tcp_reass(&mut tp, &mut h, data(11, 5), &mut tlen);
            // A duplicate of the queued head is dropped whole, nothing presented.
            let mut h = th(0, 1, 0);
            let mut tlen = 10;
            let d = tcp_reass(&mut tp, &mut h, data(1, 10), &mut tlen);
            let at = tp.st.rcv_nxt;
            let f = tcp_flush_queue(&mut tp);
            ((r, d, at, f, tp.st.rcv_nxt, tp.st.t_segq.is_some()), None)
        });
        assert_eq!(r, (Ok(0), Ok(0), 1, 0, 16, false));
        assert!(received(&inp).is_empty());
        testtcp::close(so, inp);
    }

    #[test]
    fn reassembly_without_an_entry_reuses_the_last_or_drops_the_queue() {
        let (_g, _p) = testtcp::setup();
        let qe = TCPQE_POOL.nout();
        let (so, inp) = testtcp::conn();
        let memdrop = || {
            crate::netinet::tcp_var::TCPCOUNTERS.read()[TcpstatCounters::TcpsRcvmemdrop as usize]
        };
        let r = testtcp::with_tp(&inp, |mut tp| {
            tp.st.t_state = TCPS_ESTABLISHED;
            tp.st.rcv_nxt = 1000;
            // Two segments queued past a hole, then the pool at its hard limit.
            for seq in [1100, 1200] {
                let mut h = th(0, seq, 0);
                let mut tlen = 50;
                assert_eq!(tcp_reass(&mut tp, &mut h, data(seq, 50), &mut tlen), Ok(0));
            }
            let full = TCPQE_POOL.set_hardlimit(TCPQE_POOL.nout() as u32);
            // The segment that fills the hole takes the last entry, [1200, 1250), whose data
            // is lost (the C's reuse).
            let mut h = th(0, 1000, 0);
            let mut tlen = 100;
            let filled = tcp_reass(&mut tp, &mut h, data(1000, 100), &mut tlen);
            let after_fill = (tp.st.rcv_nxt, TCPQE_POOL.nout());
            // One that does not fill a hole: the queue flushed, the segment dropped and
            // counted.
            let before = memdrop();
            let mut h = th(0, 1300, 0);
            let mut tlen = 50;
            let q = tcp_reass(&mut tp, &mut h, data(1300, 50), &mut tlen);
            let _ = TCPQE_POOL.set_hardlimit(TCPQE_POOL.nout() as u32);
            let mut h = th(0, 1500, 0);
            let mut tlen = 50;
            let dropped = tcp_reass(&mut tp, &mut h, data(1500, 50), &mut tlen);
            let counted = memdrop() - before;
            let _ = TCPQE_POOL.set_hardlimit(TCP_REASS_LIMIT_TEST);
            (
                (
                    full,
                    filled,
                    after_fill,
                    q,
                    dropped,
                    counted,
                    tp.st.t_segq.is_some(),
                ),
                None,
            )
        });
        let (full, filled, after_fill, q, dropped, counted, queued) = r;
        assert_eq!(full, Ok(()));
        assert_eq!(filled, Ok(0));
        // [1000, 1100) and [1100, 1150) were presented, the queue emptied.
        assert_eq!(after_fill, (1150, qe));
        assert_eq!(
            (q, dropped, counted, queued),
            (Ok(0), Err(Errno::ENOBUFS), 1, false)
        );
        assert_eq!(
            &received(&inp)[..],
            &(1000u32..1150).map(|s| s as u8).collect::<Vec<_>>()[..]
        );
        testtcp::close(so, inp);
        assert_eq!(TCPQE_POOL.nout(), qe);
    }

    /// The tests' restored "tcpqe" limit (`tcp_init`'s, from the doubles' `nmbclust`).
    const TCP_REASS_LIMIT_TEST: u32 = 1 << 20;

    #[test]
    fn partialack_deflates_the_window() {
        let (_g, _p) = testtcp::setup();
        let (so, inp) = testtcp::conn();
        let cwnd = testtcp::with_tp(&inp, |mut tp| {
            tp.st.t_maxseg = 100;
            tp.st.snd_una = 1000;
            tp.st.snd_cwnd = 5000;
            tp.st.t_rtttime = 9;
            tcp_sack_partialack(&mut tp, &th(TH_ACK, 0, 1300));
            let a = (
                tp.st.snd_cwnd,
                tp.st.t_rtttime,
                tp.st.has_flags(TF_NEEDOUTPUT),
            );
            // Fewer bytes in the window than acknowledged: one segment, plus one.
            tp.st.snd_cwnd = 200;
            tcp_sack_partialack(&mut tp, &th(TH_ACK, 0, 1300));
            ((a, tp.st.snd_cwnd), None)
        });
        assert_eq!(cwnd, ((5000 - 300 + 200, 0, true), 200));
        testtcp::close(so, inp);
    }
}
/* </TESTS> */
