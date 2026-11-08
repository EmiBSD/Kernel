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
 * Copyright (c) 1982, 1986, 1988, 1991, 1993
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
 *	@(#)uipc_mbuf.c	8.2 (Berkeley) 1/4/94
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
/* </LICENSES> */

/* <CODE> */
//! The mbuf allocator and the mbuf chain operations (`kern/uipc_mbuf.c`), for `bsd`'s legacy
//! code over the kernel's (`kernel::net::mbuf`, N6.1).
//!
//! Upstream: sys/kern/uipc_mbuf.c @ 3ce1f3f79392
//!
//! Every function keeps LZ's name and signature over `&'static Mbuf`, so its callers compile
//! unchanged, and forwards to the kernel's: an mbuf the kernel consumes is taken back from
//! `bsd` (`unpark`), one it returns is handed to `bsd` (`park`), one it changes in place is
//! lent mutably (`mut_of`); the boundary's protocol is `sys/sys/mbuf.rs`'s. `mbuf_list` and
//! `mbuf_queue` keep LZ's shape over the kernel's list (`sys/sys/mbuf.rs`'s `MbufList` and
//! `MbufQueue`), an mbuf parked when it leaves a list and unparked when it enters one.
//!
//! ## Deviations
//! - Each crossing costs a compare-and-swap of the mbuf's parked mark (`unpark` where the
//!   kernel consumes the head, `park` where it hands one back): legacy paths only; the
//!   kernel's own paths own their boxes.
//! - `m_makespace` and `m_pulldown` find the mbuf the kernel names by its index in the
//!   chain: a second walk the C does not make (N6.2 and N6.3 move their callers).
//! - `mbuf_dma_64bit_enable` stays here (amd64's `autoconf` calls it; it walks `ifnetlist`).

use core::fmt;
use core::sync::atomic::Ordering;

use kernel::net::mbuf as k;

use crate::kern::kern_lock::{mtx_enter, mtx_init, mtx_leave};
use crate::kern::subr_prf::printf;
use crate::machine::db_machdep::PrFn;
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, MbufList, MbufQueue, lend, ml_len, mq_len, mut_of, park, unpark};
use crate::sys::time::Timeval;

pub use kernel::net::mbuf::{
    M_TYPES, MAX_HDR, MAX_LINKHDR, MAX_PROTOHDR, MBS_NCOUNTERS, MBUF_MEM_ALLOC, MBUF_MEM_LIMIT,
    MBUFPL, MCLNAMES, MCLSIZES, MTAGPL, m_align, m_apply, m_calchdrlen, m_copydata, m_getptr,
    m_leadingspace, m_pool_noconstraints, m_pool_used, m_trailingspace, mbstat_read, mcl_nout,
    nmbclust_update,
};

/// `mbinit`: initialize the mbuf allocator, its cluster limit `nmbclust` (`conf/param.c`).
pub fn mbinit() {
    k::mbinit(&crate::conf::param::NMBCLUST);
    // pf's legs of the mbuf (the C's `NPF > 0` blocks), bridges.toml's `Pf` row: `bsd`'s pf
    // registers once; a second `mbinit` (the host tests') keeps the first.
    let _ = kernel::net::pf_hooks::register_pf(&crate::net::pf::BSD_PF);
}

/// `mbcpuinit`: the pool caches per CPU.
pub fn mbcpuinit() {
    k::mbcpuinit();
}

/// `m_get`: space allocation routines.
pub fn m_get(nowait: i32, type_: i32) -> Option<&'static Mbuf> {
    k::m_get(nowait, type_).map(park)
}

/// `m_gethdr`.
pub fn m_gethdr(nowait: i32, type_: i32) -> Option<&'static Mbuf> {
    k::m_gethdr(nowait, type_).map(park)
}

/// `m_inithdr`: makes `m` the first mbuf of an empty packet.
pub fn m_inithdr(m: &'static Mbuf) -> &'static Mbuf {
    k::m_inithdr(mut_of(m));
    m
}

/// `m_removehdr`: makes the first mbuf of a packet a plain one.
pub fn m_removehdr(m: &Mbuf) {
    k::m_removehdr(mut_of(m));
}

/// `m_resethdr`: like `m_inithdr()`, but keep any associated data and mbufs.
pub fn m_resethdr(m: &Mbuf) {
    k::m_resethdr(mut_of(m));
}

/// `m_getclr`: an mbuf with its data area zeroed.
pub fn m_getclr(nowait: i32, type_: i32) -> Option<&'static Mbuf> {
    k::m_getclr(nowait, type_).map(park)
}

/// `m_clget`: adds a cluster of at least `pktlen` bytes to `m`, or to a new packet header mbuf
/// when `m` is `None`; `None` when no cluster could be had (`m` keeps none, and is not freed).
pub fn m_clget(m: Option<&'static Mbuf>, how: i32, pktlen: u32) -> Option<&'static Mbuf> {
    match m {
        Some(m) => k::m_clget(mut_of(m), how, pktlen).then_some(m),
        None => k::m_clget_hdr(how, pktlen).map(park),
    }
}

/// `m_free`: frees one mbuf and returns the next in its chain. What the mbuf's links held is
/// parked first (`bsd`'s callers read `m->m_nextpkt` before the free and keep it).
pub fn m_free<'a>(m: impl Into<Option<&'a Mbuf>>) -> Option<&'static Mbuf> {
    let m = m.into()?;
    let mut b = unpark(m);
    if let Some(p) = b.m_nextpkt.take() {
        let _ = park(p);
    }
    k::m_free(b).map(park)
}

/// `m_freem`: frees a chain and returns the next packet.
pub fn m_freem<'a>(m: impl Into<Option<&'a Mbuf>>) -> Option<&'static Mbuf> {
    let m = m.into()?;
    k::m_freem(unpark(m)).map(park)
}

/// `m_purge`: frees a list of packets.
pub fn m_purge<'a>(m: impl Into<Option<&'a Mbuf>>) {
    let mut m = m.into().map(lend);
    while let Some(mm) = m {
        m = m_freem(mm);
    }
}

/// `m_defrag`: the chain defragmented into one buffer, the head mbuf kept.
pub fn m_defrag(m: &Mbuf, how: i32) -> Result<(), Errno> {
    k::m_defrag(mut_of(m), how)
}

/// `m_prepend`: ensure `len` bytes of contiguous space at the beginning of the mbuf chain.
pub fn m_prepend(m: &'static Mbuf, len: i32, how: i32) -> Option<&'static Mbuf> {
    k::m_prepend(unpark(m), len, how).map(park)
}

/// `m_copym`: a copy of `len` bytes of the chain from `off` (`M_COPYALL`: to the end); the
/// source's clusters are shared with the copy.
pub fn m_copym(m0: &Mbuf, off: i32, len: i32, wait: i32) -> Option<&'static Mbuf> {
    k::m_copym(mut_of(m0), off, len, wait).map(park)
}

/// `m_copyback`: `data` copied into the chain at `off`, extending it if necessary.
pub fn m_copyback<'a>(
    m0: impl Into<Option<&'a Mbuf>>,
    off: i32,
    data: &[u8],
    wait: i32,
) -> Result<(), Errno> {
    let Some(m0) = m0.into() else {
        return Ok(());
    };
    k::m_copyback(mut_of(m0), off, data, wait)
}

/// `m_adj`: trims `req_len` bytes from the head of the chain, or `-req_len` bytes from the
/// tail when negative; nothing for no chain.
pub fn m_adj<'a>(mp: impl Into<Option<&'a Mbuf>>, req_len: i32) {
    if let Some(mp) = mp.into() {
        k::m_adj(mp, req_len);
    }
}

/// `m_cat`: concatenate mbuf chain `n` to `m`.
pub fn m_cat(m: &Mbuf, n: Option<&'static Mbuf>) {
    k::m_cat(mut_of(m), n.map(unpark));
}

/// `m_pullup`: `len` bytes made contiguous at the head; the chain freed on failure.
pub fn m_pullup(m0: &'static Mbuf, len: i32) -> Option<&'static Mbuf> {
    k::m_pullup(unpark(m0), len).map(park)
}

/// `m_split`: the chain cut after `len0` bytes, the tail returned.
pub fn m_split(m0: &'static Mbuf, len0: i32, wait: i32) -> Option<&'static Mbuf> {
    k::m_split(mut_of(m0), len0, wait).map(park)
}

/// `m_makespace`: space for a header of `hlen` bytes at `skip`; the mbuf where it goes and
/// the offset there.
pub fn m_makespace(m0: &'static Mbuf, skip: i32, hlen: i32) -> Option<(&'static Mbuf, i32)> {
    let (n, off) = k::m_makespace(mut_of(m0), skip, hlen)?;
    Some((lend(n), off))
}

/// `m_devget`: a packet copied from device memory.
pub fn m_devget(buf: &[u8], off: i32) -> Option<&'static Mbuf> {
    k::m_devget(buf, off).map(park)
}

/// `m_zero`: zeroes the mbuf's storage, or marks a shared cluster to be zeroed.
pub fn m_zero(m: &Mbuf) {
    k::m_zero(mut_of(m));
}

/// `m_dup_pkthdr`: duplicate mbuf pkthdr from `from` to `to`.
pub fn m_dup_pkthdr(to: &Mbuf, from: &Mbuf, wait: i32) -> Result<(), Errno> {
    k::m_dup_pkthdr(mut_of(to), from, wait)
}

/// `m_dup_pkt`: a copy of the packet in one mbuf (or cluster), `adj` bytes in.
pub fn m_dup_pkt(m0: &Mbuf, adj: u32, wait: i32) -> Option<&'static Mbuf> {
    k::m_dup_pkt(m0, adj, wait).map(park)
}

/// `m_microtime`: the packet's timestamp, or the current time.
pub fn m_microtime(m: &Mbuf) -> Timeval {
    k::m_microtime(m)
}

/// `m_clpool`: the index in `mclpools` of the smallest cluster pool for `pktlen` bytes.
pub fn m_clpool(pktlen: u32) -> Option<usize> {
    k::m_clpool(pktlen)
}

/// `mbuf_dma_64bit_enable`: lifts the DMA constraint when every interface can reach all of
/// memory.
pub fn mbuf_dma_64bit_enable() {
    for ifp in crate::net::if_::IFNETLIST.0.iter() {
        if ifp.if_xflags.get() & crate::net::if_::IFXF_MBUF_64BIT == 0 {
            printf(format_args!(
                "{}: restrict all mbufs to low memory\n",
                crate::kern::subr_prf::Str(&ifp.if_xname.get())
            ));
            return;
        }
    }

    printf(format_args!("enable mbufs in high memory\n"));
    m_pool_noconstraints();
}

/// A `fmt::Write` over ddb's `pr` (the printers' output).
struct Pr(PrFn);

impl fmt::Write for Pr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        (self.0)(format_args!("{s}"));
        Ok(())
    }
}

/// `m_print` (`DDB`): prints one mbuf.
pub fn m_print(m: &Mbuf, pr: PrFn) {
    let _ = k::m_print(m, &mut Pr(pr));
}

/// `m_print_chain` (`DDB`): one line per mbuf of a chain and a total.
pub fn m_print_chain(v: Option<&Mbuf>, deep: bool, pr: PrFn) {
    let _ = k::m_print_chain(v, deep, &mut Pr(pr));
}

/// `m_print_packet` (`DDB`): one line per packet of a list (or every chain, when `deep`).
pub fn m_print_packet(v: Option<&Mbuf>, deep: bool, pr: PrFn) {
    let _ = k::m_print_packet(v, deep, &mut Pr(pr));
}

// mbuf lists

/// `ml_init`: an empty list; the packets it held stay where `bsd` holds them (parked), as the
/// C forgets them.
pub fn ml_init(ml: &MbufList) {
    ml.change(|l| {
        while let Some(m) = l.pop_head() {
            let _ = park(m);
        }
    });
}

/// `ml_enqueue`: appends the packet `m`. A packet `m->m_nextpkt` held is parked (the C's
/// overwrite forgets it).
pub fn ml_enqueue(ml: &MbufList, m: &'static Mbuf) {
    let mut b = unpark(m);
    if let Some(p) = b.m_nextpkt.take() {
        let _ = park(p);
    }
    ml.change(|l| l.push_tail(b));
}

/// `ml_enlist`: appends all of `mlb` to `mla` and empties `mlb`. One list as both is
/// `panic(9)` (the C would loop the list on itself; here two `&mut` to one list).
pub fn ml_enlist(mla: &MbufList, mlb: &MbufList) {
    if core::ptr::eq(mla, mlb) {
        crate::kern::subr_prf::panic(format_args!("ml_enlist: a list onto itself"));
    }
    mla.change(|a| mlb.change(|b| a.concat(b)));
}

/// `ml_dequeue`: takes the first packet.
pub fn ml_dequeue(ml: &MbufList) -> Option<&'static Mbuf> {
    ml.change(|l| l.pop_head()).map(park)
}

/// `ml_dechain`: takes all packets, still linked through `m_nextpkt` (the first one parked).
pub fn ml_dechain(ml: &MbufList) -> Option<&'static Mbuf> {
    ml.change(core::mem::take).into_head().map(park)
}

/// `ml_purge`: frees every packet; returns how many there were.
pub fn ml_purge(ml: &MbufList) -> u32 {
    ml.change(k::ml_purge)
}

/// `ml_hdatalen`: the length of the first packet.
pub fn ml_hdatalen(ml: &MbufList) -> u32 {
    ml.change(|l| k::ml_hdatalen(l))
}

// mbuf queues

/// `mq_init`.
pub fn mq_init(mq: &MbufQueue, maxlen: u32, ipl: i32) {
    mtx_init(&mq.mq_mtx, ipl);
    ml_init(&mq.mq_list);
    mq.mq_maxlen.store(maxlen, Ordering::Relaxed);
}

/// `mq_push`: enqueues `m`, dropping the oldest packet when full; `true` when one was dropped.
pub fn mq_push(mq: &MbufQueue, m: &'static Mbuf) -> bool {
    let mut dropped = None;

    mtx_enter(&mq.mq_mtx);
    if mq_len(mq) >= mq.mq_maxlen.load(Ordering::Relaxed) {
        mq.mq_drops.fetch_add(1, Ordering::Relaxed);
        dropped = mq.mq_list.change(|l| l.pop_head());
    }
    ml_enqueue(&mq.mq_list, m);
    mtx_leave(&mq.mq_mtx);

    let any = dropped.is_some();
    drop(dropped);
    any
}

/// `mq_enqueue`: enqueues `m`, or drops it when full; `true` when it was dropped.
pub fn mq_enqueue(mq: &MbufQueue, m: &'static Mbuf) -> bool {
    let mut dropped = false;

    mtx_enter(&mq.mq_mtx);
    if mq_len(mq) < mq.mq_maxlen.load(Ordering::Relaxed) {
        ml_enqueue(&mq.mq_list, m);
    } else {
        mq.mq_drops.fetch_add(1, Ordering::Relaxed);
        dropped = true;
    }
    mtx_leave(&mq.mq_mtx);

    if dropped {
        m_freem(m);
    }

    dropped
}

/// `mq_dequeue`.
pub fn mq_dequeue(mq: &MbufQueue) -> Option<&'static Mbuf> {
    mtx_enter(&mq.mq_mtx);
    let m = mq.mq_list.change(|l| l.pop_head());
    mtx_leave(&mq.mq_mtx);

    m.map(park)
}

/// `mq_enlist`: appends `ml`, or drops all of it when the queue is full; returns how many
/// packets were dropped.
pub fn mq_enlist(mq: &MbufQueue, ml: &MbufList) -> u32 {
    let mut dropped = 0;

    mtx_enter(&mq.mq_mtx);
    if mq_len(mq) < mq.mq_maxlen.load(Ordering::Relaxed) {
        ml_enlist(&mq.mq_list, ml);
    } else {
        dropped = ml_len(ml);
        mq.mq_drops.fetch_add(dropped, Ordering::Relaxed);
    }
    mtx_leave(&mq.mq_mtx);

    if dropped != 0 {
        ml_purge(ml);
    }

    dropped
}

/// `mq_delist`: moves the whole queue to `ml`.
pub fn mq_delist(mq: &MbufQueue, ml: &MbufList) {
    mtx_enter(&mq.mq_mtx);
    ml_enlist(ml, &mq.mq_list);
    mtx_leave(&mq.mq_mtx);
}

/// `mq_purge`: frees every queued packet; returns how many there were.
pub fn mq_purge(mq: &MbufQueue) -> u32 {
    let ml = MbufList::new();

    mq_delist(mq, &ml);

    ml_purge(&ml)
}

/// `mq_hdatalen`: the length of the first queued packet.
pub fn mq_hdatalen(mq: &MbufQueue) -> u32 {
    mtx_enter(&mq.mq_mtx);
    let hdatalen = ml_hdatalen(&mq.mq_list);
    mtx_leave(&mq.mq_mtx);

    hdatalen
}

/// `mq_set_maxlen`.
pub fn mq_set_maxlen(mq: &MbufQueue, maxlen: u32) {
    mtx_enter(&mq.mq_mtx);
    mq.mq_maxlen.store(maxlen, Ordering::Relaxed);
    mtx_leave(&mq.mq_mtx);
}

/// `IFQCTL_LEN` (`<net/if.h>`, not moved yet).
const IFQCTL_LEN: i32 = 1;
/// `IFQCTL_MAXLEN` (`<net/if.h>`, not moved yet).
const IFQCTL_MAXLEN: i32 = 2;
/// `IFQCTL_DROPS` (`<net/if.h>`, not moved yet).
const IFQCTL_DROPS: i32 = 3;

/// `sysctl_mq` (`!SMALL_KERNEL`): the `IFQCTL_*` nodes of a queue. `oldp` and `newp` are user
/// addresses, 0 for NULL (`kern/kern_sysctl.rs`).
pub fn sysctl_mq(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    mq: &MbufQueue,
) -> Result<(), Errno> {
    use crate::kern::kern_sysctl::{sysctl_int, sysctl_rdint};
    use core::sync::atomic::AtomicI32;

    // All sysctl names at this level are terminal.
    let [mib] = name else {
        return Err(Errno::ENOTDIR);
    };

    match *mib {
        IFQCTL_LEN => sysctl_rdint(oldp, oldlenp, newp, mq_len(mq) as i32),
        IFQCTL_MAXLEN => {
            let oldval = mq.mq_maxlen.load(Ordering::Relaxed);
            let newval = AtomicI32::new(oldval as i32);
            let error = sysctl_int(oldp, oldlenp, newp, newlen, &newval);
            let newval = newval.into_inner() as u32;
            if error.is_ok() && oldval != newval {
                mq_set_maxlen(mq, newval);
            }
            error
        }
        IFQCTL_DROPS => sysctl_rdint(oldp, oldlenp, newp, crate::sys::mbuf::mq_drops(mq) as i32),
        _ => Err(Errno::EOPNOTSUPP),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // The mbuf's own tests are the kernel's (`kernel::net::mbuf`); these are the boundary's:
    // the set-up other files' tests share, and the crossings.

    use std::sync::MutexGuard;
    use std::{assert, assert_eq, vec};

    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::mbuf::{M_DONTWAIT, MT_DATA, m_set_nextpkt, mbuf_list_first, ml_empty};

    /// Real memory, the console (so that a kernel panic in a test says why) and `mbinit`.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        crate::machine::cons::consinit();
        mbinit_again();
        guard
    }

    /// `mbinit` over memory a test has just reset (`setup_real_memory`), for the tests of other
    /// files that need mbufs: the kernel's pools are made once (their pages come from the
    /// page system of the test that asks).
    pub(crate) fn mbinit_again() {
        mbinit();
    }

    #[test]
    fn a_packet_crosses_both_ways() {
        let _g = setup();
        let out = MBUFPL.nout();

        let m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_len().set(10);
        let m = m_pullup(m, 4).expect("already contiguous");
        let c = m_copym(m, 0, 10, M_DONTWAIT).expect("a copy");
        let mut got = vec![1u8; 10];
        m_copydata(c, 0, &mut got);
        assert!(m_freem(c).is_none());
        // A packet list through m_nextpkt: the free of the first gives the second back.
        let n = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        assert!(m_set_nextpkt(m, Some(n)).is_none());
        let next = m_freem(m).expect("the next packet");
        assert!(core::ptr::eq(next, n));
        assert!(m_freem(next).is_none());
        assert_eq!(MBUFPL.nout(), out);
    }

    #[test]
    fn lists_and_queues_park_what_leaves_them() {
        let _g = setup();
        let out = MBUFPL.nout();

        let ml = MbufList::new();
        for _ in 0..3 {
            ml_enqueue(&ml, m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf"));
        }
        assert_eq!(ml_len(&ml), 3);
        // The unlocked length (the mirror) follows every change.
        let other = MbufList::new();
        ml_enqueue(&other, m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf"));
        ml_enlist(&ml, &other);
        assert_eq!((ml_len(&ml), ml_len(&other)), (4, 0));
        assert!(ml_empty(&other) && !ml_empty(&ml));
        assert_eq!(ml_hdatalen(&ml), 0);
        assert!(mbuf_list_first(&ml).is_some());
        let first = ml_dequeue(&ml).expect("a packet");
        assert_eq!(ml_len(&ml), 3);
        let chain = ml_dechain(&ml).expect("the rest");
        assert_eq!(ml_len(&ml), 0);
        assert!(ml_empty(&ml));
        assert!(chain.m_nextpkt().get().is_some(), "still linked");
        m_purge(chain);
        m_freem(first);

        let mq = MbufQueue::new(0, 0);
        mq_init(&mq, 1, crate::machine::intr::IPL_NET);
        assert!(!mq_enqueue(
            &mq,
            m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf")
        ));
        assert!(mq_enqueue(
            &mq,
            m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf")
        ));
        assert_eq!(crate::sys::mbuf::mq_len(&mq), 1);
        assert!(mq_push(
            &mq,
            m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf")
        ));
        let m = mq_dequeue(&mq).expect("the newest");
        m_freem(m);
        assert_eq!(mq_purge(&mq), 0);
        assert_eq!(MBUFPL.nout(), out);
    }
}
/* </TESTS> */
