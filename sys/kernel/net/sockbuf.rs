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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)uipc_socket2.c	8.1 (Berkeley) 6/10/93
 */

/*-
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)socketvar.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The socket buffers: `struct sockbuf` (`<sys/socketvar.h>`) and its routines
//! (`kern/uipc_socket2.c`): reserving space, appending records, dropping data, and the locks
//! and wait channel of a buffer.
//!
//! Upstream: sys/kern/uipc_socket2.c @ 3ce1f3f79392
//! Upstream: sys/sys/socketvar.h @ 3ce1f3f79392
//! LZ: sys/kern/uipc_socket2.rs@3c62ede68802
//! LZ: sys/sys/socketvar.rs@3c62ede68802
//!
//! Each socket contains two socket buffers: one for sending data and one for receiving data.
//! Each buffer contains a queue of mbufs, information about the number of mbufs and amount
//! of data in the queue, and other fields allowing select() statements and notification on
//! data availability to be implemented.
//!
//! Data stored in a socket buffer is maintained as a list of records. Each record is a list
//! of mbufs chained together with the `m_next` field; records are chained together with the
//! `m_nextpkt` field. The upper level routine `soreceive` expects the following conventions
//! to be observed when placing information in the receive buffer:
//!
//! 1. If the protocol requires each message be preceded by the sender's name, then a record
//!    containing that name must be present before any associated data (`MT_SONAME`).
//! 2. If the protocol supports the exchange of "access rights" (really just additional data
//!    associated with the message), and there are "rights" to be received, then a record
//!    containing this data should be present (`MT_CONTROL`).
//! 3. If a name or rights record exists, then it must be followed by a data record, perhaps
//!    of zero length.
//!
//! Before using a new socket structure it is first necessary to reserve buffer space to the
//! socket, by calling `sbreserve`. This should commit some of the available buffer space in
//! the system buffer pool for the socket (currently, it does nothing but enforce limits).
//! The space should be released by calling `sbrelease` when the socket is destroyed.
//!
//! The routines `sbappend` or `sbappendrecord` are normally called to append new mbufs to a
//! socket buffer, after checking that adequate space is available, comparing the function
//! `sbspace` with the amount of data to be added. `sbappendrecord` differs from `sbappend`
//! in that data supplied is treated as the beginning of a new record. To place a sender's
//! address, optional access rights, and data in a socket receive buffer, `sbappendaddr`
//! should be used; to place access rights and data, `sbappendcontrol`. In either case, the
//! new data begins a new record, and these routines check for the caller that there will be
//! enough space to store the data. Reliable protocols may use the socket send buffer to hold
//! data awaiting acknowledgement: data is copied from it with `m_copym` for output to a
//! peer, and removed with `sbdrop` or `sbdroprecord` when the peer acknowledges it.
//!
//! Locks: `sb_mtx` (\[m\]) guards the records and the counters; `sblock` (\[b\], the
//! buffer's `sb_lock`) serialises its readers (or its writers) across their sleeps.
//!
//! ## Deviations
//! - `SOCKBUF_DEBUG` is not configured, as in GENERIC: `sbcheck` is kept (the host tests
//!   call it), `sblastrecordchk` and `sblastmbufchk` are gone with the fields they checked
//!   (`sb_lastrecord` and `sb_mbtail` are positions of the record list now, see Redesign).
//! - `sbappendaddr`, `sbappendcontrol` and `sbchecklowmem` return `bool` (the C's 1 or 0);
//!   `sbreserve` and `sbcheckreserve` return `Result` with `ENOBUFS` for the C's non-zero.
//! - `sbappendaddr` takes the address as its bytes (`asa.sa_len` of them, the first byte
//!   being `sa_len`), as the C copies them with `memcpy`.
//! - While a reader holds the first record out of the buffer for a copy with the mutex
//!   released (`soreceive`, `somove`: [`SbGuard::take_first_record`]), the senders cannot
//!   compress into that record's last mbuf, which the C may (`sbcompress` into a tail the
//!   reader is copying from): the new data starts a fresh mbuf. Counts and record
//!   boundaries are the C's; only how the bytes are split between mbufs differs.
//!
//! ## Redesign
//! - LZ's `struct sockbuf` was `Cell`s beside a legacy `struct mutex`, the chain `&'static
//!   Mbuf` links with `sb_mb`, `sb_mbtail` and `sb_lastrecord` as raw positions every
//!   routine kept in step by hand. Here [`Sockbuf`] holds `sb_lock` as a `hal`
//!   [`RawRwlock`] (`sblock` gives its witness, [`SbLocked`]), `sb_mtx` as a
//!   [`Mutexed`] that owns the records ([`SbData`]), and the counters, limits, flags and
//!   state as atomics written only through the mutex's guard ([`SbGuard`], docs/IDIOMS.md
//!   row 39): the kqueue filters (called under the mutex with no witness), `kern.file` and
//!   `FIONREAD` read them as the C reads them, with no lock.
//! - The records are owned: `records` (every record but the last, by `m_nextpkt`, a
//!   [`MbufList`]), `last` (the last record's mbufs but its final one, by `m_next`) and
//!   `tail` (that final mbuf, the C's `sb_mbtail`), so `sb_mb`, `sb_lastrecord` and
//!   `sb_mbtail` are O(1) positions and `sbcompress` writes into `tail` through the guard's
//!   `&mut` (D-N6.2-1). The brief's `hal::owned::OwnedSlist::last_mut` is not used: a `&mut`
//!   to a listed mbuf would let its holder relink the mbuf's own `m_next` (a `pub` field)
//!   behind the list, leaving the list's tail word naming a freed mbuf; holding the final
//!   mbuf outside the list needs no `unsafe` at all. Invariants: `tail` empty means `last`
//!   is empty; `records` non-empty means `tail` is not.
//! - A reader copies to user space with the mutex released (the C's
//!   `mtx_leave`/`uiomove`/`mtx_enter`), where a sender may append meanwhile. The C reads
//!   the first record through pointers that stay valid because only the reader removes
//!   records; here the reader takes the first record out of the buffer
//!   ([`SbGuard::take_first_record`], an owned [`SbRecord`]) and puts what is left back
//!   ([`SbGuard::put_back_record`]); a sender that closes the last record meanwhile keeps the
//!   record's continuation aside (`Out::Last { pending }`) so the record is whole again when
//!   it comes back. Counts stay in the buffer throughout, as the C's do until the reader's
//!   `sbfree`.
//! - Cost against the C: `sbappendstream` and `sbcompress` are O(k) over the appended mbufs
//!   with an O(1) tail, as the C; `sbdrop` pops one mbuf per step (O(1)), plus one branch
//!   per step for which part of the list the front is in; `sbappend`'s walk of the last
//!   record for `M_EOR` is the C's (its "XXX"). Every operation takes the buffer's mutex as
//!   often as the C takes `sb_mtx`.
//! - `tcp_output` copies its segment out of the send buffer under the socket's lock alone
//!   in the C (`m_copym(so->so_snd.sb_mb, off, len)`: every writer of the buffer holds that
//!   lock too); here the records are the mutex's, so the copy is a guard's method
//!   ([`SbGuard::copym_first`], [`SbGuard::copydata_first`]): the first record's mbufs up to
//!   `off + len` taken off as one chain for `m_copym` (which shares their clusters through
//!   `&mut`) and put back, O(k) for the C's walk of k mbufs. `tcp_output` reads
//!   `SS_ISSENDING` as the atomic it is instead of the C's `sb_mtx` section for it, so it
//!   takes the mutex once per data segment, as the C takes it once per call (N6.2a).

use core::mem;
use core::sync::atomic::{AtomicBool, AtomicI64, AtomicU16, AtomicU32, AtomicU64, Ordering};

use hal::intr::IPL_MPFLOOR;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::owned::{OwnedSlist, SlistIter};
use hal::rwlock::{RW_INTR, RW_NOSLEEP, RawRwlock, WriteWitness};
use hal::sleep::PCATCH;
use hal::sys::errno::Errno;
use hal::sys::socket::{Cmsghdr, cmsg_align, cmsg_len, cmsg_space};
use hal::user::bytes_of;

use crate::assert::{kassert, kpanic};
use crate::event::Klist;
use crate::net::mbuf::{
    Ext, M_DONTWAIT, M_EOR, M_EXT, MAXMCLBYTES, MCLBYTES, MLEN, MNext, MSIZE, MT_CONTROL,
    MT_SONAME, Mbuf, MbufBox, MbufList, m_get, m_pool_used, m_trailingspace, mclget,
};
use crate::proc::ProcGlue;

/// `SB_MAX`: default for max chars in sockbuf.
pub const SB_MAX: u64 = 2 * 1024 * 1024;
/// `SB_WAIT`: someone is waiting for data/space.
pub const SB_WAIT: u16 = 0x0001;
/// `SB_ASYNC`: ASYNC I/O, need signals.
pub const SB_ASYNC: u16 = 0x0002;
/// `SB_SPLICE`: buffer is splice source or drain.
pub const SB_SPLICE: u16 = 0x0004;
/// `SB_NOINTR`: operations not interruptible.
pub const SB_NOINTR: u16 = 0x0008;

/// `SBL_WAIT`: wait if lock not immediately available.
pub const SBL_WAIT: i32 = 0x01;
/// `SBL_NOINTR`: enforce non-interruptible sleep.
pub const SBL_NOINTR: i32 = 0x02;

/// `PSOCK`: the priority of a sleep on a socket.
pub const PSOCK: i32 = 24;

// The socket state bits (`so_state`, and `sb_state` for the four that belong to one buffer:
// SS_CANTSENDMORE and SS_ISSENDING with so_snd, SS_CANTRCVMORE and SS_RCVATMARK with
// so_rcv).

/// `SS_NOFDREF`: no file table ref any more.
pub const SS_NOFDREF: u32 = 0x001;
/// `SS_ISCONNECTED`: socket connected to a peer.
pub const SS_ISCONNECTED: u32 = 0x002;
/// `SS_ISCONNECTING`: in process of connecting to peer.
pub const SS_ISCONNECTING: u32 = 0x004;
/// `SS_ISDISCONNECTING`: in process of disconnecting.
pub const SS_ISDISCONNECTING: u32 = 0x008;
/// `SS_CANTSENDMORE`: can't send more data to peer.
pub const SS_CANTSENDMORE: u32 = 0x010;
/// `SS_CANTRCVMORE`: can't receive more data from peer.
pub const SS_CANTRCVMORE: u32 = 0x020;
/// `SS_RCVATMARK`: at mark on input.
pub const SS_RCVATMARK: u32 = 0x040;
/// `SS_ISDISCONNECTED`: socket disconnected from peer.
pub const SS_ISDISCONNECTED: u32 = 0x800;
/// `SS_PRIV`: privileged for broadcast, raw...
pub const SS_PRIV: u32 = 0x080;
/// `SS_CONNECTOUT`: connect, not accept, at this end.
pub const SS_CONNECTOUT: u32 = 0x1000;
/// `SS_ISSENDING`: hint for lower layer.
pub const SS_ISSENDING: u32 = 0x2000;
/// `SS_DNS`: created using `SOCK_DNS` socket(2).
pub const SS_DNS: u32 = 0x4000;
/// `SS_YP`: created using ypconnect(2).
pub const SS_YP: u32 = 0x8000;

/// \[I\] `sb_max`: patchable.
pub static SB_MAX_VAR: AtomicU64 = AtomicU64::new(SB_MAX);

/// `sbchecklowmem`'s `static int sblowmem`.
static SBLOWMEM: AtomicBool = AtomicBool::new(false);

/// Whether `m` counts in `sb_datacc`: it is neither an address nor control.
fn is_data(m: &Mbuf) -> bool {
    let t = i32::from(m.m_type().get());
    t != MT_CONTROL && t != MT_SONAME
}

/// The storage `m` takes: `MSIZE`, plus its cluster.
fn mbcnt(m: &Mbuf) -> u64 {
    let mut n = MSIZE as u64;
    if m.flags() & M_EXT != 0 {
        n += u64::from(m.ext().map_or(0, Ext::ext_size));
    }
    n
}

/// The C's `sb_lastrecord` chain walked by `m_next` from `m`.
fn chain(m: Option<&Mbuf>) -> impl Iterator<Item = &Mbuf> {
    core::iter::successors(m, |m| m.m_next.get())
}

/// Where the reader's record is while it is out of the buffer.
enum Out {
    /// No record is out.
    None,
    /// A record that was not the last is out: the senders, which touch only the last one,
    /// are not concerned.
    Rec,
    /// The last record is out (it was the buffer's only one). The senders see an empty
    /// buffer: what they append continues that record (`last`, `tail`), until one of them
    /// closes it (`sbappendrecord`, `sbappendaddr`, `sbappendcontrol`): then the
    /// continuation waits in `pending` and the new record is the last.
    Last {
        /// The record's last mbuf carried `M_EOR` when it went out (`sbappend`'s walk).
        eor: bool,
        /// The continuation of the record, closed while it was out.
        pending: Option<(OwnedSlist<MNext>, Option<MbufBox>)>,
    },
}

/// What `sb_mtx` guards alone (\[m\]): the records, and the reader's record when it is out.
pub struct SbData {
    /// Every record but the last, by `m_nextpkt` (`sb_mb` .. the one before
    /// `sb_lastrecord`).
    records: MbufList,
    /// The last record's mbufs but its final one, by `m_next` (`sb_lastrecord` ..).
    last: OwnedSlist<MNext>,
    /// The last record's final mbuf (`sb_mbtail`).
    tail: Option<MbufBox>,
    /// The reader's record out of the buffer.
    out: Out,
}

impl SbData {
    /// [`SbGuard::take_all`]'s work on the records.
    fn take_all(&mut self) -> Option<MbufBox> {
        // No reader can hold a record out here: the callers own the buffer (the last
        // reference's `sbrelease`) or take `sblock` (`sorflush`), or empty a socket no
        // process has open (`unp_gc`'s dead ones).
        kassert!(matches!(self.out, Out::None));
        self.close_last();
        mem::take(&mut self.records).into_head()
    }

    /// [`SbGuard::close_last`]'s work on the records.
    fn close_last(&mut self) {
        if let Out::Last { pending, .. } = &mut self.out
            && pending.is_none()
        {
            *pending = Some((mem::take(&mut self.last), self.tail.take()));
            return;
        }
        if let Some(t) = self.tail.take() {
            self.last.push_tail(t);
            if let Some(h) = mem::take(&mut self.last).into_head() {
                self.records.push_tail(h);
            }
        }
    }

    /// An empty buffer.
    const fn new() -> Self {
        Self {
            records: MbufList::new(),
            last: OwnedSlist::new(),
            tail: None,
            out: Out::None,
        }
    }

    /// `sb->sb_mb`: the first mbuf of the first record.
    pub fn first(&self) -> Option<&Mbuf> {
        self.records
            .first()
            .or_else(|| self.last.first())
            .or(self.tail.as_deref())
    }

    /// `sb->sb_lastrecord`: the first mbuf of the last record.
    pub fn lastrecord(&self) -> Option<&Mbuf> {
        self.last.first().or(self.tail.as_deref())
    }

    /// `sb->sb_mbtail`: the last mbuf of the last record.
    pub fn mbtail(&self) -> Option<&Mbuf> {
        self.tail.as_deref()
    }

    /// Whether the buffer holds no record (`sb->sb_mb == NULL`).
    pub fn is_empty(&self) -> bool {
        self.tail.is_none()
    }

    /// `sb->sb_mb->m_nextpkt != NULL`: there is a record after the first.
    pub fn has_next_record(&self) -> bool {
        !self.records.is_empty()
    }

    /// The records in order, each as its mbufs (`for (m = sb_mb; m; m = m->m_nextpkt)`
    /// with an inner `m_next` walk): O(1) a step.
    pub fn records(&self) -> impl Iterator<Item = RecordIter<'_>> {
        self.records
            .iter()
            .map(|r| RecordIter::Chain(Some(r)))
            .chain(
                self.tail
                    .is_some()
                    .then(|| RecordIter::Last(self.last.iter(), self.tail.as_deref())),
            )
    }

    /// Every mbuf of the buffer in order (`m_next` within a record, then the next record).
    pub fn mbufs(&self) -> impl Iterator<Item = &Mbuf> {
        self.records().flatten()
    }

    /// The first record's mbufs (`for (m = sb_mb; m; m = m->m_next)`).
    pub fn first_record(&self) -> impl Iterator<Item = &Mbuf> {
        self.records().next().into_iter().flatten()
    }

    /// Whether an mbuf of the last record carries `M_EOR` (`sbappend`'s walk from
    /// `sb_lastrecord`, `while (n->m_next && (n = n->m_next))`).
    fn ends_with_eor(&self) -> bool {
        self.last
            .iter()
            .chain(self.tail.as_deref())
            .any(|m| m.flags() & M_EOR != 0)
    }
}

/// The mbufs of one record, as [`SbData::records`] walks them.
pub enum RecordIter<'a> {
    /// A record before the last: its chain by `m_next`.
    Chain(Option<&'a Mbuf>),
    /// The last record: `last`, then `tail`.
    Last(SlistIter<'a, MNext>, Option<&'a Mbuf>),
}

impl<'a> Iterator for RecordIter<'a> {
    type Item = &'a Mbuf;

    fn next(&mut self) -> Option<&'a Mbuf> {
        match self {
            Self::Chain(m) => {
                let x = (*m)?;
                *m = x.m_next.get();
                Some(x)
            }
            Self::Last(it, tail) => it.next().or_else(|| tail.take()),
        }
    }
}

/// The first record of a buffer, out of it for a reader's copy ([`SbGuard::take_first_record`]):
/// owned, so the reader walks it, copies from it and changes it with the buffer's mutex
/// released, then gives back what is left ([`SbGuard::put_back_record`]).
pub struct SbRecord {
    /// The record's mbufs but its last, by `m_next`.
    list: OwnedSlist<MNext>,
    /// The record's last mbuf.
    tail: Option<MbufBox>,
}

impl SbRecord {
    /// An empty record.
    const fn new() -> Self {
        Self {
            list: OwnedSlist::new(),
            tail: None,
        }
    }

    /// The record from its first mbuf, which owns the others by `m_next`: O(k), each mbuf
    /// taken off its predecessor (the C's walk of the record is O(k) too).
    fn from_chain(mut m: Option<MbufBox>) -> Self {
        let mut r = Self::new();
        while let Some(mut b) = m {
            m = b.m_next.take();
            r.push(b);
        }
        r
    }

    /// `b` (alone) after the record's last mbuf.
    fn push(&mut self, b: MbufBox) {
        if let Some(t) = self.tail.replace(b) {
            self.list.push_tail(t);
        }
    }

    /// The record's first mbuf (`m = so->so_rcv.sb_mb`).
    pub fn first(&self) -> Option<&Mbuf> {
        self.list.first().or(self.tail.as_deref())
    }

    /// The record's mbufs in order.
    pub fn iter(&self) -> impl Iterator<Item = &Mbuf> {
        self.list.iter().chain(self.tail.as_deref())
    }

    /// Whether nothing is left of the record.
    pub fn is_empty(&self) -> bool {
        self.tail.is_none()
    }

    /// The first mbuf taken off the record when it is not the last one, which stays (a
    /// `MSG_PEEK` reader's step past an mbuf, `m = m->m_next`, O(1)).
    pub fn pop_first_not_last(&mut self) -> Option<MbufBox> {
        self.list.pop_head()
    }

    /// The first mbuf taken off the record (alone: its `m_next` empty).
    pub fn pop_first(&mut self) -> Option<MbufBox> {
        self.list.pop_head().or_else(|| self.tail.take())
    }

    /// `b` (alone) back at the record's front.
    pub fn push_first(&mut self, b: MbufBox) {
        if self.tail.is_none() {
            self.tail = Some(b);
        } else {
            self.list.push_head(b);
        }
    }

    /// The mbufs of `l` (each alone) before the record's first, in their order. The record
    /// must not be empty (its last mbuf stays its last).
    pub fn prepend(&mut self, mut l: OwnedSlist<MNext>) {
        if l.is_empty() {
            return;
        }
        kassert!(self.tail.is_some());
        l.concat(&mut self.list);
        self.list = l;
    }

    /// The record as one chain from its first mbuf (owning the others by `m_next`).
    fn into_chain(mut self) -> Option<MbufBox> {
        if let Some(t) = self.tail.take() {
            self.list.push_tail(t);
        }
        mem::take(&mut self.list).into_head()
    }
}

/// `struct sockbuf`: variables for socket buffering.
pub struct Sockbuf<G: ProcGlue> {
    /// `sb_lock`: serialises the buffer's readers (or writers) across their sleeps
    /// ([`Sockbuf::sblock`]).
    sb_lock: RawRwlock,
    /// `sb_mtx` and what it guards alone.
    mtx: Mutexed<SbData>,
    /// \[m\] `sb_cc`: actual chars in buffer.
    cc: AtomicU64,
    /// \[m\] `sb_datacc`: data only chars in buffer.
    datacc: AtomicU64,
    /// \[m\] `sb_hiwat`: max actual char count.
    hiwat: AtomicU64,
    /// \[m\] `sb_wat`: default watermark.
    wat: AtomicU64,
    /// \[m\] `sb_mbcnt`: chars of mbufs used.
    mbcnt: AtomicU64,
    /// \[m\] `sb_mbmax`: max chars of mbufs to use.
    mbmax: AtomicU64,
    /// \[m\] `sb_lowat`: low water mark.
    lowat: AtomicI64,
    /// \[m\] `sb_flags`: flags, see `SB_*`.
    flags: AtomicU16,
    /// \[m\] `sb_state`: socket state on sockbuf (`SS_*`).
    state: AtomicU32,
    /// \[m\] `sb_timeo_nsecs`: timeout for read/write.
    timeo_nsecs: AtomicU64,
    /// Whether a splice peer is set (`ssp_socket` or `ssp_soback`), for `isspliced` and
    /// `issplicedback` read where the C reads the pointer: the filters, `soreadable`.
    spliced: AtomicBool,
    /// \[m\] `sb_klist`: list of knotes, under the mutex (`klist_init_mutex`).
    pub(crate) klist: Klist<G>,
}

impl<G: ProcGlue> Sockbuf<G> {
    /// A zeroed `struct sockbuf` as `soalloc` leaves it: its locks made (`sbuf`, `sb_mtx` at
    /// `IPL_MPFLOOR`), its klist of the mutex.
    pub const fn new() -> Self {
        Self {
            sb_lock: RawRwlock::new("sbuf"),
            mtx: Mutexed::new(IPL_MPFLOOR, SbData::new()),
            cc: AtomicU64::new(0),
            datacc: AtomicU64::new(0),
            hiwat: AtomicU64::new(0),
            wat: AtomicU64::new(0),
            mbcnt: AtomicU64::new(0),
            mbmax: AtomicU64::new(0),
            lowat: AtomicI64::new(0),
            flags: AtomicU16::new(0),
            state: AtomicU32::new(0),
            timeo_nsecs: AtomicU64::new(0),
            spliced: AtomicBool::new(false),
            klist: Klist::owner(),
        }
    }

    /// `sb_mtx`, for the cells keyed to it (the splice peers, `Guarded::new_mutexed`).
    #[inline]
    pub(crate) fn mutex(&self) -> &Mutexed<SbData> {
        &self.mtx
    }

    /// The watermarks and the timeout of a buffer whose socket nobody else reaches yet
    /// (`socreate`'s `sb_timeo_nsecs = INFSLP`, `sonewconn`'s copy of its listener's), stored
    /// without the mutex as the C stores them: no reader or writer can see the buffer before
    /// the socket is handed out, which publishes these stores with it.
    pub(crate) fn init_fresh(&self, wat: u64, lowat: i64, timeo_nsecs: u64) {
        self.wat.store(wat, Ordering::Relaxed);
        self.lowat.store(lowat, Ordering::Relaxed);
        self.timeo_nsecs.store(timeo_nsecs, Ordering::Relaxed);
    }

    /// The records of a buffer nobody else can reach (its socket's last reference going:
    /// `sofree`'s unlocked `m_purge(so->so_rcv.sb_mb)`), taken whole without the mutex, the
    /// `&mut` the proof.
    pub(crate) fn take_all_unshared(&mut self) -> Option<MbufBox> {
        self.mtx.get_mut().take_all()
    }

    /// `mtx_enter(&sb->sb_mtx)`.
    #[inline]
    pub fn lock(&self) -> SbGuard<'_, G> {
        SbGuard {
            sb: self,
            g: self.mtx.lock(),
        }
    }

    /// `sb->sb_cc`, read as the C reads it without the mutex.
    #[inline]
    pub fn cc(&self) -> u64 {
        self.cc.load(Ordering::Relaxed)
    }

    /// `sb->sb_datacc`.
    #[inline]
    pub fn datacc(&self) -> u64 {
        self.datacc.load(Ordering::Relaxed)
    }

    /// `sb->sb_hiwat`.
    #[inline]
    pub fn hiwat(&self) -> u64 {
        self.hiwat.load(Ordering::Relaxed)
    }

    /// `sb->sb_wat`.
    #[inline]
    pub fn wat(&self) -> u64 {
        self.wat.load(Ordering::Relaxed)
    }

    /// `sb->sb_mbcnt`.
    #[inline]
    pub fn mbcnt(&self) -> u64 {
        self.mbcnt.load(Ordering::Relaxed)
    }

    /// `sb->sb_mbmax`.
    #[inline]
    pub fn mbmax(&self) -> u64 {
        self.mbmax.load(Ordering::Relaxed)
    }

    /// `sb->sb_lowat`.
    #[inline]
    pub fn lowat(&self) -> i64 {
        self.lowat.load(Ordering::Relaxed)
    }

    /// `sb->sb_flags`.
    #[inline]
    pub fn flags(&self) -> u16 {
        self.flags.load(Ordering::Relaxed)
    }

    /// `sb->sb_flags & bits`.
    #[inline]
    pub fn has_flags(&self, bits: u16) -> bool {
        self.flags() & bits != 0
    }

    /// `sb->sb_state`.
    #[inline]
    pub fn state(&self) -> u32 {
        self.state.load(Ordering::Relaxed)
    }

    /// `sb->sb_state & bits`.
    #[inline]
    pub fn has_state(&self, bits: u32) -> bool {
        self.state() & bits != 0
    }

    /// `sb->sb_timeo_nsecs`.
    #[inline]
    pub fn timeo_nsecs(&self) -> u64 {
        self.timeo_nsecs.load(Ordering::Relaxed)
    }

    /// Whether the buffer has a splice peer (`isspliced`'s `ssp_socket` in a receive buffer,
    /// `issplicedback`'s `ssp_soback` in a send buffer).
    #[inline]
    pub fn is_spliced(&self) -> bool {
        self.spliced.load(Ordering::Relaxed)
    }

    /// The wait channel of `sb->sb_cc` (`sbwait` and `sowakeup`).
    #[inline]
    pub fn cc_chan(&self) -> *const AtomicU64 {
        core::ptr::from_ref(&self.cc)
    }

    /// `sbspace(sb)`, read without the mutex (the filters', which run under it with no
    /// witness): how much space is there in a socket buffer? This is problematical if the
    /// fields are unsigned, as the space might still be negative (`cc > hiwat` or `mbcnt >
    /// mbmax`): the differences wrap and read as negative `long`s, as in C.
    #[inline]
    pub fn space_unlocked(&self) -> i64 {
        let cc = self.hiwat().wrapping_sub(self.cc()) as i64;
        let mb = self.mbmax().wrapping_sub(self.mbcnt()) as i64;
        cc.min(mb)
    }

    /// `sbspace(sb)`: `sbspace_locked` under the buffer's mutex.
    pub fn sbspace(&self) -> i64 {
        self.lock().space()
    }

    /// `sb->sb_cc = cc; sb->sb_mbcnt = mbcnt` under the socket's lock alone, without
    /// `sb_mtx`: `unp_disconnect`'s plain stores into the send buffers of a UNIX domain
    /// stream pair, which hold no data (their counts mirror the peer's receive buffer).
    pub(crate) fn set_cc_mbcnt_unlocked(&self, cc: u64, mbcnt: u64) {
        self.cc.store(cc, Ordering::Relaxed);
        self.mbcnt.store(mbcnt, Ordering::Relaxed);
    }

    /// `sb_notify(sb)`: do we need to notify the other side when I/O is possible?
    pub fn sb_notify(&self) -> bool {
        let _g = self.lock();
        self.has_flags(SB_WAIT | SB_ASYNC | SB_SPLICE) || !self.klist.is_empty()
    }

    /// `knote(&sb->sb_klist, hint)`: the buffer's knotes activated, under its mutex (the
    /// klist's lock).
    pub fn knote(&self, hint: i64) {
        let g = self.lock();
        self.klist.knote_locked(g.witness_ref(), hint);
    }

    /// `sblock(sb, flags)`: serialises the readers (or writers) of the buffer;
    /// `EWOULDBLOCK` without `SBL_WAIT` when another holds it, the sleep's error when a
    /// signal ends the wait (`SBL_NOINTR` or `SB_NOINTR` make it uninterruptible).
    pub fn sblock(&self, flags: i32) -> Result<SbLocked<'_>, Errno> {
        let mut rwflags = 0;
        if !(flags & SBL_NOINTR != 0 || self.has_flags(SB_NOINTR)) {
            rwflags |= RW_INTR;
        }
        if flags & SBL_WAIT == 0 {
            rwflags |= RW_NOSLEEP;
        }
        match self.sb_lock.write_flags(rwflags) {
            Ok(w) => Ok(SbLocked { _w: w }),
            Err(Errno::EBUSY) => Err(Errno::EWOULDBLOCK),
            Err(e) => Err(e),
        }
    }

    /// `rw_status(&sb->sb_lock)`: whether `sblock` is held (`sbassertlocked`'s question).
    pub fn sb_lock_status(&self) -> i32 {
        self.sb_lock.status()
    }
}

impl<G: ProcGlue> Default for Sockbuf<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `sblock(sb)` held: the buffer's readers (or writers) are this holder; `sbunlock` is its
/// drop.
#[must_use = "dropping it releases sblock"]
pub struct SbLocked<'a> {
    _w: WriteWitness<'a>,
}

/// `sb_mtx` held: the buffer's records and the writes of its counters
/// (`mtx_enter(&sb->sb_mtx)` .. `mtx_leave`, the drop).
pub struct SbGuard<'a, G: ProcGlue> {
    sb: &'a Sockbuf<G>,
    g: MutexedGuard<'a, SbData>,
}

impl<'a, G: ProcGlue> SbGuard<'a, G> {
    /// The buffer.
    #[inline]
    pub fn sockbuf(&self) -> &'a Sockbuf<G> {
        self.sb
    }

    /// The records, to read them.
    #[inline]
    pub fn data(&self) -> &SbData {
        &self.g
    }

    /// The mutex's witness (`klist_insert_locked`, `knote_locked` on `sb_klist`).
    #[inline]
    pub fn witness(&mut self) -> &mut MutexedGuard<'a, SbData> {
        &mut self.g
    }

    /// The mutex's witness, shared (a cell keyed to the mutex read).
    #[inline]
    pub fn witness_ref(&self) -> &MutexedGuard<'a, SbData> {
        &self.g
    }

    /// `sb->sb_cc = v`.
    #[inline]
    fn set_cc(&mut self, v: u64) {
        self.sb.cc.store(v, Ordering::Relaxed);
    }

    /// `sb->sb_cc += n` (wrapping, as the C's unsigned arithmetic).
    #[inline]
    fn add_cc(&mut self, n: u64) {
        self.set_cc(self.sb.cc().wrapping_add(n));
    }

    /// `sb->sb_datacc += n`.
    #[inline]
    fn add_datacc(&mut self, n: u64) {
        self.sb
            .datacc
            .store(self.sb.datacc().wrapping_add(n), Ordering::Relaxed);
    }

    /// `sb->sb_mbcnt += n`.
    #[inline]
    fn add_mbcnt(&mut self, n: u64) {
        self.sb
            .mbcnt
            .store(self.sb.mbcnt().wrapping_add(n), Ordering::Relaxed);
    }

    /// `sb->sb_cc -= len; sb->sb_datacc -= len` of data trimmed from an mbuf (`sbdrop`'s
    /// partial mbuf, `soreceive`'s partial copy).
    pub fn sub_data(&mut self, m: &Mbuf, len: u64) {
        self.set_cc(self.sb.cc().wrapping_sub(len));
        if is_data(m) {
            self.add_datacc(len.wrapping_neg());
        }
    }

    /// `sb->sb_cc = cc; sb->sb_mbcnt = mbcnt`: the counts a UNIX domain stream sender
    /// mirrors from its peer's receive buffer for back pressure (`uipc_rcvd`, `uipc_send`,
    /// `unp_disconnect`).
    pub fn set_cc_mbcnt(&mut self, cc: u64, mbcnt: u64) {
        self.set_cc(cc);
        self.sb.mbcnt.store(mbcnt, Ordering::Relaxed);
    }

    /// `sb->sb_hiwat = v`.
    pub fn set_hiwat(&mut self, v: u64) {
        self.sb.hiwat.store(v, Ordering::Relaxed);
    }

    /// `sb->sb_wat = v`.
    pub fn set_wat(&mut self, v: u64) {
        self.sb.wat.store(v, Ordering::Relaxed);
    }

    /// `sb->sb_mbmax = v`.
    pub fn set_mbmax(&mut self, v: u64) {
        self.sb.mbmax.store(v, Ordering::Relaxed);
    }

    /// `sb->sb_lowat = v`.
    pub fn set_lowat(&mut self, v: i64) {
        self.sb.lowat.store(v, Ordering::Relaxed);
    }

    /// `sb->sb_timeo_nsecs = v`.
    pub fn set_timeo_nsecs(&mut self, v: u64) {
        self.sb.timeo_nsecs.store(v, Ordering::Relaxed);
    }

    /// `sb->sb_flags |= bits`.
    pub fn set_flags(&mut self, bits: u16) {
        self.sb
            .flags
            .store(self.sb.flags() | bits, Ordering::Relaxed);
    }

    /// `sb->sb_flags &= ~bits`.
    pub fn clear_flags(&mut self, bits: u16) {
        self.sb
            .flags
            .store(self.sb.flags() & !bits, Ordering::Relaxed);
    }

    /// `sb->sb_state |= bits`.
    pub fn set_state(&mut self, bits: u32) {
        self.sb
            .state
            .store(self.sb.state() | bits, Ordering::Relaxed);
    }

    /// `sb->sb_state &= ~bits`.
    pub fn clear_state(&mut self, bits: u32) {
        self.sb
            .state
            .store(self.sb.state() & !bits, Ordering::Relaxed);
    }

    /// `sb->sb_state = v` (`somove`'s `rcvstate`, read then written back whole).
    pub fn set_state_word(&mut self, v: u32) {
        self.sb.state.store(v, Ordering::Relaxed);
    }

    /// Whether the buffer has a splice peer now (`ssp_socket` of a receive buffer set or
    /// cleared by `sosplice`/`sounsplice`, `ssp_soback` of a send buffer), stored under the
    /// mutex as the C stores the pointer.
    pub fn set_spliced(&mut self, on: bool) {
        self.sb.spliced.store(on, Ordering::Relaxed);
    }

    /// `sbspace_locked(sb)`: [`Sockbuf::space_unlocked`] under the mutex.
    #[inline]
    pub fn space(&self) -> i64 {
        self.sb.space_unlocked()
    }

    /// `sballoc(sb, m)`: adjust counters in `sb` reflecting allocation of `m`.
    #[inline]
    pub fn sballoc(&mut self, m: &Mbuf) {
        let len = u64::from(m.m_len().get());
        self.add_cc(len);
        if is_data(m) {
            self.add_datacc(len);
        }
        self.add_mbcnt(mbcnt(m));
    }

    /// `sbfree(sb, m)`: adjust counters in `sb` reflecting freeing of `m`.
    #[inline]
    pub fn sbfree(&mut self, m: &Mbuf) {
        let len = u64::from(m.m_len().get());
        self.add_cc(len.wrapping_neg());
        if is_data(m) {
            self.add_datacc(len.wrapping_neg());
        }
        self.add_mbcnt(mbcnt(m).wrapping_neg());
    }

    /// `sbwait(sb)`: wait for data to arrive at/drain from a socket buffer, the mutex held
    /// again on return. `SB_WAIT` is set for the waker (`sowakeup`).
    pub fn sbwait(&mut self) -> Result<(), Errno> {
        let prio = if self.sb.has_flags(SB_NOINTR) {
            PSOCK
        } else {
            PSOCK | PCATCH
        };
        self.set_flags(SB_WAIT);
        let nsecs = self.sb.timeo_nsecs();
        let chan = self.sb.cc_chan();
        self.g.sleep_nsec(chan, prio, "sbwait", nsecs)
    }

    /// `sowakeup`'s part under the mutex: `SB_WAIT` cleared (and whether a waiter must be
    /// woken), whether `SIGIO` is asked (`SB_ASYNC`), the knotes activated
    /// (`knote_locked(&sb->sb_klist, 0)`).
    pub fn wakeup_locked(&mut self) -> (bool, bool) {
        let dowakeup = self.sb.has_flags(SB_WAIT);
        if dowakeup {
            self.clear_flags(SB_WAIT);
        }
        let dopgsigio = self.sb.has_flags(SB_ASYNC);
        self.sb.klist.knote_locked(&self.g, 0);
        (dowakeup, dopgsigio)
    }

    /// `sbreserve(sb, cc)`: allot mbufs to a sockbuf. Attempt to scale `mbmax` so that
    /// `mbcnt` doesn't become limiting if buffering efficiency is near the normal case.
    pub fn sbreserve(&mut self, cc: u64) -> Result<(), Errno> {
        if cc == 0 || cc > SB_MAX_VAR.load(Ordering::Relaxed) {
            return Err(Errno::ENOBUFS);
        }
        self.set_hiwat(cc);
        self.set_mbmax((3 * MAXMCLBYTES as u64).max(cc * 8));
        if self.sb.lowat() > self.sb.hiwat() as i64 {
            self.set_lowat(self.sb.hiwat() as i64);
        }
        Ok(())
    }

    /// `sbrelease(sb)`: free mbufs held by a socket, and reserved mbuf space.
    pub fn sbrelease(&mut self) {
        self.sbflush();
        self.set_hiwat(0);
        self.set_mbmax(0);
    }

    /// The buffer's records taken whole and its counts and limits cleared, from `sb_cc` up
    /// to (not including) `sb_flags` (`sorflush`'s and `unp_gc`'s `memset(&sb->sb_startzero,
    /// 0, ..)` after `m = sb->sb_mb`): the records, as one chain of records by `m_nextpkt`.
    pub fn take_all_zeroed(&mut self) -> Option<MbufBox> {
        let m = self.take_all();
        for a in [
            &self.sb.cc,
            &self.sb.datacc,
            &self.sb.hiwat,
            &self.sb.wat,
            &self.sb.mbcnt,
            &self.sb.mbmax,
        ] {
            a.store(0, Ordering::Relaxed);
        }
        self.sb.lowat.store(0, Ordering::Relaxed);
        m
    }

    /// Every record taken off as one chain of records (the first record's first mbuf, the
    /// records linked by `m_nextpkt`, each record's mbufs by `m_next`), the counts left.
    fn take_all(&mut self) -> Option<MbufBox> {
        self.g.take_all()
    }

    /// The last record closed into `records` (`SBLINKRECORD`'s first half: the next record
    /// starts after it). With the reader's last record out, the first close keeps the
    /// continuation aside instead.
    fn close_last(&mut self) {
        self.g.close_last();
    }

    /// `b` (alone) as the last record's new last mbuf (`n->m_next = m` or `sb->sb_mb = m`,
    /// then `sb->sb_mbtail = m`).
    fn link_tail(&mut self, b: MbufBox) {
        let d = &mut *self.g;
        if let Some(t) = d.tail.replace(b) {
            d.last.push_tail(t);
        }
    }

    /// `sbappend(sb, m)`: append mbuf chain `m` to the last record in the socket buffer `sb`.
    /// The additional space associated the mbuf chain is recorded in `sb`. Empty mbufs are
    /// discarded and mbufs are compacted where possible.
    pub fn sbappend(&mut self, m: Option<MbufBox>) {
        let Some(m) = m else {
            return;
        };
        // XXX Would like to simply use sb_mbtail here, but XXX I need to verify that I won't
        // miss an EOR that XXX way.
        // The last record, while the reader holds it out, is that record and what continues
        // it here.
        let out_eor = matches!(
            &self.g.out,
            Out::Last {
                eor: true,
                pending: None
            }
        );
        if out_eor || self.g.ends_with_eor() {
            self.sbappendrecord(Some(m)); // XXXXXX!!!!
            return;
        }
        self.sbcompress(Some(m));
    }

    /// `sbappendstream(sb, m)`: this version of `sbappend()` should only be used when the
    /// caller absolutely knows that there will never be more than one record in the socket
    /// buffer, that is, a stream protocol (such as TCP).
    pub fn sbappendstream(&mut self, m: MbufBox) {
        crate::assert::kdassert!(!m.m_nextpkt.is_some());
        kassert!(self.g.records.is_empty());
        self.sbcompress(Some(m));
    }

    /// `sbappendrecord(sb, m0)`: as above, except the mbuf chain begins a new record.
    pub fn sbappendrecord(&mut self, m0: Option<MbufBox>) {
        let Some(mut m0) = m0 else {
            return;
        };
        // Put the first mbuf on the queue. Note this permits zero length records.
        self.sballoc(&m0);
        self.close_last();
        let m = m0.m_next.take();
        if let Some(mm) = m {
            if m0.flags() & M_EOR != 0 {
                m0.m_flags().set(m0.flags() & !M_EOR);
                mm.m_flags().set(mm.flags() | M_EOR);
            }
            self.g.tail = Some(m0);
            self.sbcompress_chain(Some(mm), &mut 0);
            return;
        }
        self.g.tail = Some(m0);
    }

    /// `sbappendaddr(sb, asa, m0, control)`: append address and data, and optionally,
    /// control (ancillary) data to the receive queue of a socket. If present, `m0` must
    /// include a packet header with total length. Returns `false` if no space in sockbuf or
    /// insufficient mbufs (the data and the control are given back then).
    pub fn sbappendaddr(
        &mut self,
        asa: &[u8],
        m0: Option<MbufBox>,
        control: Option<MbufBox>,
    ) -> Result<(), (Option<MbufBox>, Option<MbufBox>)> {
        let sa_len = usize::from(asa.first().copied().unwrap_or(0));
        let mut space = sa_len as i64;

        if let Some(m0) = &m0 {
            let Some(h) = m0.pkthdr() else {
                kpanic(format_args!("sbappendaddr"));
            };
            space += i64::from(h.len.get());
        }
        space += chain(control.as_deref())
            .map(|n| i64::from(n.m_len().get()))
            .sum::<i64>();
        if space > self.space() {
            return Err((m0, control));
        }
        if sa_len > MLEN {
            return Err((m0, control));
        }
        let Some(addr) = asa.get(..sa_len) else {
            return Err((m0, control));
        };
        let Some(mut m) = m_get(M_DONTWAIT, MT_SONAME) else {
            return Err((m0, control));
        };
        m.m_len().set(sa_len as u32);
        match m.data_mut() {
            Some(d) => d.copy_from_slice(addr),
            None => kpanic(format_args!("sbappendaddr: a fresh mbuf read-only")),
        }
        // The record: the name, the control, the data (`n->m_next = m0`, `m->m_next =
        // control`), each mbuf counted.
        self.close_last();
        self.sballoc(&m);
        self.link_tail(m);
        for c in [control, m0] {
            let mut c = c;
            while let Some(mut b) = c {
                c = b.m_next.take();
                self.sballoc(&b);
                self.link_tail(b);
            }
        }
        Ok(())
    }

    /// `sbappendcontrol(sb, m0, control)`: append control and data to the receive queue of a
    /// socket as a new record; the data and the control given back if there is not enough
    /// space.
    pub fn sbappendcontrol(
        &mut self,
        m0: Option<MbufBox>,
        control: MbufBox,
    ) -> Result<(), (Option<MbufBox>, MbufBox)> {
        let mut space: i64 = chain(Some(&control))
            .map(|m| i64::from(m.m_len().get()))
            .sum();
        let mut eor: u16 = 0;
        for x in chain(m0.as_deref()) {
            space += i64::from(x.m_len().get());
            eor |= x.flags() & M_EOR;
            if eor != 0 {
                if x.m_next.is_some() {
                    x.m_flags().set(x.flags() & !M_EOR);
                } else {
                    x.m_flags().set(x.flags() | M_EOR);
                }
            }
        }
        if space > self.space() {
            return Err((m0, control));
        }
        self.close_last();
        for c in [Some(control), m0] {
            let mut c = c;
            while let Some(mut b) = c {
                c = b.m_next.take();
                self.sballoc(&b);
                self.link_tail(b);
            }
        }
        Ok(())
    }

    /// `sbcompress(sb, m, sb->sb_mbtail)`: compress mbuf chain `m` into the socket buffer
    /// following its last mbuf. If there is none, the buffer is presumed empty.
    pub fn sbcompress(&mut self, m: Option<MbufBox>) {
        self.sbcompress_chain(m, &mut 0);
    }

    /// `sbcompress`'s loop, `eor` the `M_EOR` seen so far.
    fn sbcompress_chain(&mut self, m: Option<MbufBox>, eor: &mut u16) {
        let mut m = m;
        while let Some(mut mm) = m {
            let next = mm.m_next.take();
            *eor |= mm.flags() & M_EOR;
            let n = self.g.tail.as_deref();
            if mm.m_len().get() == 0
                && (*eor == 0
                    || next
                        .as_deref()
                        .or(n)
                        .is_some_and(|o| o.m_type().get() == mm.m_type().get()))
            {
                drop(mm);
                m = next;
                continue;
            }
            if let Some(nn) = self.g.tail.as_deref_mut()
                && nn.flags() & M_EOR == 0
                // m_trailingspace() checks buffer writeability
                && mm.m_len().get()
                    <= (if nn.flags() & M_EXT != 0 {
                        nn.ext().map_or(0, Ext::ext_size)
                    } else {
                        MCLBYTES as u32
                    }) / 4 // XXX Don't copy too much
                && mm.m_len().get() as i32 <= m_trailingspace(nn)
                && nn.m_type().get() == mm.m_type().get()
            {
                let len = mm.m_len().get() as usize;
                let at = nn.data_off() + nn.m_len().get() as usize;
                match nn.storage_mut().and_then(|s| s.get_mut(at..at + len)) {
                    Some(dst) => dst.copy_from_slice(mm.data()),
                    None => kpanic(format_args!("sbcompress: no room after the tail")),
                }
                nn.m_len().set(nn.m_len().get() + len as u32);
                self.add_cc(len as u64);
                if is_data(&mm) {
                    self.add_datacc(len as u64);
                }
                drop(mm);
                m = next;
                continue;
            }
            self.sballoc(&mm);
            mm.m_flags().set(mm.flags() & !M_EOR);
            self.link_tail(mm);
            m = next;
        }
        if *eor != 0 {
            match self.g.tail.as_deref() {
                Some(nn) => nn.m_flags().set(nn.flags() | *eor),
                None => crate::prf::printf(format_args!("semi-panic: sbcompress")),
            }
        }
    }

    /// `sbflush(sb)`: free all mbufs in a sockbuf. Check that all resources are reclaimed.
    pub fn sbflush(&mut self) {
        kassert!(self.sb.sb_lock.status() == 0);
        while self.sb.mbcnt() != 0 {
            self.sbdrop(self.sb.cc() as i32);
        }
        kassert!(self.sb.cc() == 0);
        kassert!(self.sb.datacc() == 0);
        kassert!(self.g.is_empty());
    }

    /// The front mbuf of the first record taken off the buffer (alone), the rest of that
    /// record staying the first record, and whether that was the record's last mbuf;
    /// `None` when the buffer is empty.
    fn pop_front(&mut self) -> Option<(MbufBox, bool)> {
        let d = &mut *self.g;
        if let Some(mut r) = d.records.pop_head() {
            return Some(match r.m_next.take() {
                Some(rest) => {
                    d.records.push_head(rest);
                    (r, false)
                }
                None => (r, true),
            });
        }
        if let Some(b) = d.last.pop_head() {
            return Some((b, false));
        }
        d.tail.take().map(|b| (b, true))
    }

    /// `sbdrop(sb, len)`: drop data from (the front of) a sockbuf.
    pub fn sbdrop(&mut self, len: i32) {
        kassert!(matches!(self.g.out, Out::None));
        let mut len = len;
        // The C's `m == NULL`: the record the drop is in has no mbuf left (the next drop
        // goes on in the next record, `m = next`).
        let mut exhausted = false;
        while len > 0 {
            exhausted = false;
            let Some(m) = self.g.first() else {
                kpanic(format_args!("sbdrop"));
            };
            let mlen = m.m_len().get();
            if mlen as i32 > len {
                m.m_len().set(mlen - len as u32);
                m.data_add(len as isize);
                let datum = is_data(m);
                self.set_cc(self.sb.cc().wrapping_sub(len as u64));
                if datum {
                    self.add_datacc((len as u64).wrapping_neg());
                }
                break;
            }
            len -= mlen as i32;
            exhausted = self.free_front();
        }
        // The empty mbufs left at the front of that record go too.
        while !exhausted
            && let Some(m) = self.g.first()
            && m.m_len().get() == 0
        {
            exhausted = self.free_front();
        }
    }

    /// The buffer's first mbuf freed (`sbfree(sb, m); m = m_free(m)`); whether its record is
    /// used up.
    fn free_front(&mut self) -> bool {
        let Some((b, exhausted)) = self.pop_front() else {
            kpanic(format_args!("sbdrop: no mbuf"));
        };
        self.sbfree(&b);
        drop(b);
        exhausted
    }

    /// `sbdroprecord(sb)`: drop a record off the front of a sockbuf and move the next record
    /// to the front.
    pub fn sbdroprecord(&mut self) {
        kassert!(matches!(self.g.out, Out::None));
        let d = &mut *self.g;
        let rec = if let Some(r) = d.records.pop_head() {
            Some(r)
        } else {
            SbRecord {
                list: mem::take(&mut d.last),
                tail: d.tail.take(),
            }
            .into_chain()
        };
        let mut m = rec;
        while let Some(mut b) = m {
            m = b.m_next.take();
            self.sbfree(&b);
            drop(b);
        }
    }

    /// `m_copym(sb->sb_mb, off, len, wait)`: a copy of `len` bytes (or `M_COPYALL`) of the
    /// buffer's first record from `off`, its clusters shared as the C's (`tcp_output`'s
    /// segment out of a stream socket's send buffer); `None` when an mbuf cannot be had.
    /// The first record's mbufs up to `off + len` are taken off as one chain for the copy
    /// and put back in order: O(k) for the k mbufs the C's walk reaches. No reader's record
    /// may be out (a send buffer's readers are its protocol's, under the socket's lock).
    pub fn copym_first(&mut self, off: i32, len: i32, wait: i32) -> Option<MbufBox> {
        kassert!(matches!(self.g.out, Out::None));
        let d = &mut *self.g;
        if let Some(mut h) = d.records.pop_head() {
            // A closed record: its mbufs are one chain by `m_next`.
            let c = crate::net::mbuf::m_copym(&mut h, off, len, wait);
            d.records.push_head(h);
            return c;
        }
        // The last record: the mbufs of `last` the copy reaches, and its final one when the
        // copy goes that far, linked into one chain.
        let want = if len == crate::net::mbuf::M_COPYALL {
            u64::MAX
        } else {
            u64::from(off.unsigned_abs()) + u64::from(len.unsigned_abs())
        };
        let mut taken = OwnedSlist::<MNext>::new();
        let mut have = 0u64;
        while have < want {
            let Some(b) = d.last.pop_head() else {
                break;
            };
            have += u64::from(b.m_len().get());
            taken.push_tail(b);
        }
        let took_tail = have < want;
        if took_tail && let Some(t) = d.tail.take() {
            taken.push_tail(t);
        }
        let mut head = taken.into_head()?;
        let c = crate::net::mbuf::m_copym(&mut head, off, len, wait);
        // Back in order: the chain split mbuf by mbuf again (O(k)), before what is left of
        // `last`, or as the whole record when its final mbuf was taken too.
        let mut rec = SbRecord::from_chain(Some(head));
        if took_tail {
            kassert!(d.last.is_empty());
            d.last = mem::take(&mut rec.list);
            d.tail = rec.tail.take();
        } else {
            if let Some(t) = rec.tail.take() {
                rec.list.push_tail(t);
            }
            rec.list.concat(&mut d.last);
            d.last = mem::take(&mut rec.list);
        }
        c
    }

    /// `m_copydata(sb->sb_mb, off, len, p)`: `p.len()` bytes of the buffer's first record
    /// from `off` copied into `p` (`tcp_output`'s segment that fits its header mbuf), O(k)
    /// for the k mbufs the C's walk reaches. `panic(9)` for a record shorter than that, as
    /// the C's.
    pub fn copydata_first(&self, off: i32, p: &mut [u8]) {
        if let Some(h) = self.g.records.first() {
            crate::net::mbuf::m_copydata(h, off, p);
            return;
        }
        if off < 0 {
            kpanic(format_args!("m_copydata: off {off} < 0"));
        }
        let mut off = off.unsigned_abs() as usize;
        let mut at = 0;
        for m in self.g.last.iter().chain(self.g.tail.as_deref()) {
            if at == p.len() {
                break;
            }
            let data = m.data();
            if off >= data.len() {
                off -= data.len();
                continue;
            }
            let n = (data.len() - off).min(p.len() - at);
            p[at..at + n].copy_from_slice(&data[off..off + n]);
            at += n;
            off = 0;
        }
        if at < p.len() {
            kpanic(format_args!("m_copydata: short mbuf chain"));
        }
    }

    /// The buffer's first record taken out for a reader's copy with the mutex released
    /// (`soreceive`, `somove` under `sblock`); `None` when the buffer is empty. The counts
    /// stay; [`SbGuard::put_back_record`] gives back what is left.
    pub fn take_first_record(&mut self) -> Option<SbRecord> {
        kassert!(matches!(self.g.out, Out::None));
        let d = &mut *self.g;
        if let Some(r) = d.records.pop_head() {
            d.out = Out::Rec;
            return Some(SbRecord::from_chain(Some(r)));
        }
        let tail = d.tail.take()?;
        let eor = d
            .last
            .iter()
            .chain(Some(&*tail))
            .any(|m| m.flags() & M_EOR != 0);
        d.out = Out::Last { eor, pending: None };
        Some(SbRecord {
            list: mem::take(&mut d.last),
            tail: Some(tail),
        })
    }

    /// What the senders appended to the reader's record while it was out (its continuation),
    /// moved to the record's end, so the reader goes on through it as the C's reader goes on
    /// down `m_next` into mbufs appended meanwhile; a record closed meanwhile is no longer
    /// the last one from then on. Nothing for a record that was not the last.
    pub fn refill_record(&mut self, rec: &mut SbRecord) {
        let d = &mut *self.g;
        let (list, tail, closed) = match &mut d.out {
            Out::Last { pending: None, eor } => {
                if d.tail.is_none() {
                    return;
                }
                let tail = d.tail.take();
                *eor |= d
                    .last
                    .iter()
                    .chain(tail.as_deref())
                    .any(|m| m.flags() & M_EOR != 0);
                (mem::take(&mut d.last), tail, false)
            }
            Out::Last {
                pending: Some((l, t)),
                ..
            } => (mem::take(l), t.take(), true),
            Out::Rec | Out::None => return,
        };
        if closed {
            d.out = Out::Rec;
        }
        let mut more = SbRecord { list, tail };
        if more.is_empty() {
            return;
        }
        if let Some(t) = rec.tail.take() {
            rec.list.push_tail(t);
        }
        rec.list.concat(&mut more.list);
        rec.tail = more.tail.take();
    }

    /// Whether a record follows the one a reader holds out, or (none out) the buffer's
    /// first (`nextrecord != NULL`).
    pub fn next_record_exists(&self) -> bool {
        match &self.g.out {
            Out::None => self.g.has_next_record(),
            Out::Rec => !self.g.is_empty(),
            Out::Last { pending, .. } => pending.is_some(),
        }
    }

    /// What is left of the record [`SbGuard::take_first_record`] took out, back at the
    /// buffer's front, whole again with what the senders appended to it meanwhile.
    pub fn put_back_record(&mut self, mut rec: SbRecord) {
        let d = &mut *self.g;
        match mem::replace(&mut d.out, Out::None) {
            Out::None => kpanic(format_args!("put_back_record: no record out")),
            Out::Rec => {
                if let Some(h) = rec.into_chain() {
                    d.records.push_head(h);
                }
            }
            Out::Last { pending: None, .. } => {
                // The record is still the last one: what was appended continues it.
                if rec.is_empty() {
                    return;
                }
                if d.tail.is_none() {
                    d.last = mem::take(&mut rec.list);
                    d.tail = rec.tail.take();
                } else {
                    if let Some(t) = rec.tail.take() {
                        rec.list.push_tail(t);
                    }
                    rec.list.concat(&mut d.last);
                    d.last = mem::take(&mut rec.list);
                }
            }
            Out::Last {
                pending: Some((l, t)),
                ..
            } => {
                // Closed meanwhile: the record and its continuation are the first record.
                let mut cont = SbRecord { list: l, tail: t };
                if let Some(t) = rec.tail.take() {
                    rec.list.push_tail(t);
                }
                if let Some(t) = cont.tail.take() {
                    cont.list.push_tail(t);
                }
                rec.list.concat(&mut cont.list);
                if let Some(h) = mem::take(&mut rec.list).into_head() {
                    d.records.push_head(h);
                }
            }
        }
    }
}

/// `sbcheckreserve(cnt, defcnt)`: in low memory situation, do not accept any greater than
/// normal request.
pub fn sbcheckreserve(cnt: u64, defcnt: u64) -> Result<(), Errno> {
    if cnt > defcnt && sbchecklowmem() {
        return Err(Errno::ENOBUFS);
    }
    Ok(())
}

/// `sbchecklowmem()`: whether the mbuf pools are short of memory (with hysteresis between
/// 60 and 80 percent used).
pub fn sbchecklowmem() -> bool {
    // m_pool_used() is thread safe. Global variable sblowmem is updated by multiple CPUs,
    // but most times with the same value. And even if the value is not correct for a short
    // time, it does not matter.
    let used = m_pool_used();
    if used < 60 {
        SBLOWMEM.store(false, Ordering::Relaxed);
    } else if used > 80 {
        SBLOWMEM.store(true, Ordering::Relaxed);
    }
    SBLOWMEM.load(Ordering::Relaxed)
}

/// `sbcreatecontrol(p, size, type, level)`: create a "control" mbuf containing the
/// specified data with the specified type for presentation on a socket buffer.
pub fn sbcreatecontrol(p: &[u8], type_: i32, level: i32) -> Option<MbufBox> {
    let size = p.len();

    if cmsg_space(size) > MCLBYTES {
        crate::prf::printf(format_args!("sbcreatecontrol: message too large {size}\n"));
        return None;
    }

    let mut m = m_get(M_DONTWAIT, MT_CONTROL)?;
    if cmsg_space(size) > MLEN {
        mclget(&mut m, M_DONTWAIT);
        if m.flags() & M_EXT == 0 {
            return None;
        }
    }
    let space = cmsg_space(size);
    m.m_len().set(space as u32);
    let cm = Cmsghdr {
        cmsg_len: cmsg_len(size) as u32,
        cmsg_level: level,
        cmsg_type: type_,
    };
    let hdr = cmsg_align(size_of::<Cmsghdr>());
    let Some(d) = m.data_mut() else {
        kpanic(format_args!("sbcreatecontrol: a fresh mbuf read-only"));
    };
    d.fill(0);
    d[..size_of::<Cmsghdr>()].copy_from_slice(bytes_of(core::slice::from_ref(&cm)));
    d[hdr..hdr + size].copy_from_slice(p);
    Some(m)
}

/// `sbcheck(so, sb)` (`SOCKBUF_DEBUG`): the counters match the chain.
pub fn sbcheck<G: ProcGlue>(g: &SbGuard<'_, G>) {
    let mut len: u64 = 0;
    let mut mbcnt_: u64 = 0;
    for rec in g.data().records() {
        for (i, n) in rec.enumerate() {
            len += u64::from(n.m_len().get());
            mbcnt_ += mbcnt(n);
            if i > 0 && n.m_nextpkt.is_some() {
                kpanic(format_args!("sbcheck nextpkt"));
            }
        }
    }
    let sb = g.sockbuf();
    if len != sb.cc() || mbcnt_ != sb.mbcnt() {
        crate::prf::printf(format_args!(
            "cc {len} != {} || mbcnt {mbcnt_} != {}\n",
            sb.cc(),
            sb.mbcnt()
        ));
        kpanic(format_args!("sbcheck"));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::net::mbuf::tests::setup;
    use crate::net::mbuf::{M_COPYALL, M_WAIT, MT_DATA, m_gethdr};

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files);

    /// An mbuf holding `bytes` (a packet header one when `pkthdr`, its length set).
    fn data(bytes: &[u8], pkthdr: bool) -> MbufBox {
        let mut m = if pkthdr {
            m_gethdr(M_WAIT, MT_DATA)
        } else {
            m_get(M_WAIT, MT_DATA)
        }
        .expect("an mbuf");
        m.m_len().set(bytes.len() as u32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        if let Some(h) = m.pkthdr() {
            h.len.set(bytes.len() as i32);
        }
        m
    }

    /// `m` with `n` linked after it.
    fn cat(mut m: MbufBox, n: MbufBox) -> MbufBox {
        let mut at = &mut *m;
        while at.m_next.is_some() {
            at = at.next_mut().expect("the next mbuf");
        }
        drop(at.m_next.set(n));
        m
    }

    /// A buffer of `hiwat` bytes.
    fn sockbuf(hiwat: u64) -> Sockbuf<Procs> {
        let sb = Sockbuf::new();
        sb.lock().sbreserve(hiwat).expect("reserve");
        sb
    }

    /// The bytes of each record, its mbufs' types beside them.
    fn records(sb: &Sockbuf<Procs>) -> Vec<(Vec<i16>, Vec<u8>)> {
        let g = sb.lock();
        g.data()
            .records()
            .map(|r| {
                let mut types = Vec::new();
                let mut bytes = Vec::new();
                for m in r {
                    types.push(m.m_type().get());
                    bytes.extend_from_slice(m.data());
                }
                (types, bytes)
            })
            .collect()
    }

    /// The mbufs of the buffer.
    fn nmbufs(sb: &Sockbuf<Procs>) -> usize {
        sb.lock().data().mbufs().count()
    }

    #[test]
    fn sbappend_counts_and_compresses() {
        let _g = setup();
        let sb = sockbuf(8192);
        {
            let mut g = sb.lock();
            g.sbappend(Some(data(b"hello ", true)));
            // A small mbuf goes into the trailing space of the last one.
            g.sbappend(Some(data(b"world", false)));
            sbcheck(&g);
        }
        assert_eq!(sb.cc(), 11);
        assert_eq!(sb.datacc(), 11);
        assert_eq!(nmbufs(&sb), 1);
        assert_eq!(sb.mbcnt(), MSIZE as u64);
        assert_eq!(
            records(&sb),
            [(std::vec![MT_DATA as i16], b"hello world".to_vec())]
        );

        // Empty mbufs are dropped; the stream form appends to the one record.
        {
            let mut g = sb.lock();
            g.sbappendstream(cat(data(b"", false), data(b"!", false)));
            sbcheck(&g);
        }
        assert_eq!(records(&sb)[0].1, b"hello world!");
        assert!(!sb.lock().data().has_next_record());

        sb.lock().sbdrop(6);
        assert_eq!(records(&sb)[0].1, b"world!");
        assert_eq!(sb.cc(), 6);
        sb.lock().sbflush();
        assert_eq!((sb.cc(), sb.mbcnt()), (0, 0));
        assert!(sb.lock().data().is_empty());
    }

    #[test]
    fn records_with_address_and_control() {
        let _g = setup();
        let sb = sockbuf(8192);
        let addr = [4u8, 1, b'a', b'b'];
        {
            let mut g = sb.lock();
            assert!(
                g.sbappendaddr(&addr, Some(data(b"one", true)), None)
                    .is_ok()
            );
            let control = sbcreatecontrol(&[1, 2, 3, 4], 1, 0xffff).expect("control");
            assert!(g.sbappendcontrol(Some(data(b"two", true)), control).is_ok());
            sbcheck(&g);
            assert!(g.data().has_next_record());
        }
        let r = records(&sb);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, [MT_SONAME as i16, MT_DATA as i16]);
        assert_eq!(&r[0].1[..4], &addr);
        assert_eq!(&r[0].1[4..], b"one");
        assert_eq!(r[1].0, [MT_CONTROL as i16, MT_DATA as i16]);
        assert_eq!(sb.datacc(), 6);
        assert_eq!(sb.cc(), 4 + 3 + cmsg_space(4) as u64 + 3);

        sb.lock().sbdroprecord();
        let r = records(&sb);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, [MT_CONTROL as i16, MT_DATA as i16]);
        assert_eq!(sb.datacc(), 3);
        sb.lock().sbdroprecord();
        assert!(sb.lock().data().is_empty());
        assert_eq!((sb.cc(), sb.mbcnt()), (0, 0));
    }

    #[test]
    fn appends_that_do_not_fit_give_their_mbufs_back() {
        let _g = setup();
        let sb = sockbuf(4);
        let mut g = sb.lock();
        let Err((m0, c)) = g.sbappendaddr(&[2u8, 1], Some(data(b"too long", true)), None) else {
            panic!("appended past sbspace");
        };
        assert_eq!(m0.expect("the data back").data(), b"too long");
        assert!(c.is_none());
        let control = sbcreatecontrol(&[0; 8], 1, 0xffff).expect("control");
        let Err((_, c)) = g.sbappendcontrol(None, control) else {
            panic!("appended past sbspace");
        };
        assert_eq!(i32::from(c.m_type().get()), MT_CONTROL);
        assert!(g.data().is_empty());
        assert_eq!(sb.cc(), 0);
    }

    #[test]
    fn sbreserve_limits_and_lowat() {
        let _g = setup();
        let sb: Sockbuf<Procs> = Sockbuf::new();
        let mut g = sb.lock();
        assert_eq!(g.sbreserve(0), Err(Errno::ENOBUFS));
        assert_eq!(g.sbreserve(SB_MAX + 1), Err(Errno::ENOBUFS));
        g.set_lowat(1 << 20);
        assert_eq!(g.sbreserve(4096), Ok(()));
        assert_eq!(sb.hiwat(), 4096);
        assert_eq!(sb.mbmax(), 3 * MAXMCLBYTES as u64);
        assert_eq!(sb.lowat(), 4096);
        assert_eq!(g.sbreserve(SB_MAX), Ok(()));
        assert_eq!(sb.mbmax(), SB_MAX * 8);
        assert_eq!(g.space(), SB_MAX as i64);
        g.sbrelease();
        assert_eq!((sb.hiwat(), sb.mbmax()), (0, 0));
    }

    #[test]
    fn sbcreatecontrol_builds_one_message() {
        let _g = setup();
        let m = sbcreatecontrol(&[9, 8, 7], 4, 0xffff).expect("control");
        let d = m.data();
        assert_eq!(d.len(), cmsg_space(3));
        let cm: Cmsghdr = hal::user::pod_from_bytes(&d[..12]).expect("a header");
        assert_eq!(cm.cmsg_len as usize, cmsg_len(3));
        assert_eq!((cm.cmsg_level, cm.cmsg_type), (0xffff, 4));
        assert_eq!(&d[16..19], &[9, 8, 7]);
        assert!(d[19..].iter().all(|&b| b == 0));
        // Past MLEN a cluster holds it; past MCLBYTES there is none.
        let big = sbcreatecontrol(&[1; 300], 1, 0xffff).expect("in a cluster");
        assert!(big.flags() & M_EXT != 0);
        assert!(sbcreatecontrol(&[0; MCLBYTES], 1, 0xffff).is_none());
    }

    #[test]
    fn sbdrop_crosses_records_and_keeps_the_last_record() {
        let _g = setup();
        let sb = sockbuf(8192);
        {
            let mut g = sb.lock();
            g.sbappendrecord(Some(cat(data(b"ab", true), data(b"cd", false))));
            g.sbappendrecord(Some(cat(data(b"ef", true), data(b"", false))));
            g.sbappendrecord(Some(data(b"gh", true)));
            sbcheck(&g);
        }
        assert_eq!(records(&sb).len(), 3);
        // Through the first record and into the second.
        sb.lock().sbdrop(5);
        let r = records(&sb);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].1, b"f");
        assert_eq!(r[1].1, b"gh");
        // The rest of the second record, exactly: the next record is not touched.
        sb.lock().sbdrop(1);
        let r = records(&sb);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, b"gh");
        assert_eq!(sb.cc(), 2);
        {
            let g = sb.lock();
            assert!(!g.data().has_next_record());
            assert_eq!(g.data().lastrecord().map(Mbuf::data), Some(&b"gh"[..]));
            assert_eq!(g.data().mbtail().map(Mbuf::data), Some(&b"gh"[..]));
            sbcheck(&g);
        }
        sb.lock().sbdrop(2);
        assert!(sb.lock().data().is_empty());
        assert_eq!(sb.mbcnt(), 0);
    }

    #[test]
    fn an_eor_on_the_last_record_starts_a_new_one() {
        let _g = setup();
        let sb = sockbuf(8192);
        {
            let mut g = sb.lock();
            let m = data(b"rec", true);
            m.m_flags().set(m.flags() | M_EOR);
            g.sbappend(Some(m));
            g.sbappend(Some(data(b"next", false)));
            sbcheck(&g);
        }
        let r = records(&sb);
        assert_eq!(r.len(), 2);
        assert_eq!(
            (r[0].1.as_slice(), r[1].1.as_slice()),
            (&b"rec"[..], &b"next"[..])
        );
    }

    #[test]
    fn a_record_taken_out_comes_back_whole() {
        let _g = setup();
        let sb = sockbuf(8192);
        sb.lock().sbappendstream(data(b"abc", true));
        // The reader takes the only record out; a sender appends meanwhile: the new bytes
        // continue the record (not compressed into the mbuf the reader holds).
        let mut rec = sb.lock().take_first_record().expect("a record");
        assert!(sb.lock().data().is_empty());
        sb.lock().sbappendstream(data(b"def", false));
        assert_eq!(nmbufs(&sb), 1);
        assert_eq!(rec.first().map(Mbuf::data), Some(&b"abc"[..]));
        // The reader consumed one byte of its mbuf.
        let first = rec.first().expect("the first mbuf");
        first.m_len().set(2);
        first.data_add(1);
        {
            let mut g = sb.lock();
            g.sub_data(rec.first().expect("the first mbuf"), 1);
            g.put_back_record(rec);
            sbcheck(&g);
        }
        assert_eq!(
            records(&sb),
            [(std::vec![MT_DATA as i16; 2], b"bcdef".to_vec())]
        );

        // Taken out again, consumed whole: what was appended meanwhile is the record.
        rec = sb.lock().take_first_record().expect("a record");
        sb.lock().sbappendstream(data(b"g", false));
        {
            let mut g = sb.lock();
            while let Some(b) = rec.pop_first() {
                g.sbfree(&b);
            }
            g.put_back_record(rec);
            sbcheck(&g);
        }
        assert_eq!(records(&sb)[0].1, b"g");
    }

    #[test]
    fn a_record_closed_while_out_keeps_its_place() {
        let _g = setup();
        let sb = sockbuf(8192);
        sb.lock().sbappend(Some(data(b"one", true)));
        let rec = sb.lock().take_first_record().expect("a record");
        {
            let mut g = sb.lock();
            // Continues the record out, then a new record closes it.
            g.sbappend(Some(data(b"+", false)));
            g.sbappendrecord(Some(data(b"two", true)));
            g.sbappendrecord(Some(data(b"three", true)));
            g.put_back_record(rec);
            sbcheck(&g);
        }
        let r = records(&sb);
        let bytes: Vec<&[u8]> = r.iter().map(|(_, b)| b.as_slice()).collect();
        assert_eq!(bytes, [&b"one+"[..], b"two", b"three"]);

        // A record before the last, out and back: the others do not move.
        let rec = sb.lock().take_first_record().expect("a record");
        sb.lock().sbappendrecord(Some(data(b"four", true)));
        sb.lock().put_back_record(rec);
        let bytes: Vec<Vec<u8>> = records(&sb).into_iter().map(|(_, b)| b).collect();
        assert_eq!(
            bytes,
            [
                b"one+".to_vec(),
                b"two".to_vec(),
                b"three".to_vec(),
                b"four".to_vec()
            ]
        );
    }

    /// The bytes of a chain.
    fn bytes_of(c: &Mbuf) -> Vec<u8> {
        chain(Some(c))
            .flat_map(|m| m.data().iter().copied())
            .collect()
    }

    #[test]
    fn copies_out_of_the_first_record_leave_it_whole() {
        let _g = setup();
        let sb = sockbuf(65536);
        // Four mbufs of 200 bytes, none fitting in the one before (tcp_output's send buffer).
        {
            let mut g = sb.lock();
            for i in 0..4u8 {
                g.sbappendstream(data(&[i; 200], false));
            }
            sbcheck(&g);
        }
        assert_eq!(nmbufs(&sb), 4);
        let whole = records(&sb);
        let want = |off: usize, len: usize| -> Vec<u8> {
            (off..off + len).map(|i| (i / 200) as u8).collect()
        };

        // m_copydata across three mbufs of the record's list.
        let mut buf = [0u8; 300];
        sb.lock().copydata_first(150, &mut buf);
        assert_eq!(buf.to_vec(), want(150, 300));

        // m_copym from the list into the final mbuf, then of the final mbuf alone, then all.
        for (off, len) in [(150, 300), (500, 300), (700, 100), (0, 200)] {
            let c = sb.lock().copym_first(off, len, M_WAIT).expect("a copy");
            assert_eq!(
                bytes_of(&c),
                want(off as usize, len as usize),
                "{off} {len}"
            );
            let g = sb.lock();
            sbcheck(&g);
        }
        let all = sb.lock().copym_first(0, M_COPYALL, M_WAIT).expect("a copy");
        assert_eq!(bytes_of(&all), want(0, 800));
        drop(all);
        // The record as it was, its final mbuf still the one sbcompress fills.
        assert_eq!(records(&sb), whole);
        assert_eq!(nmbufs(&sb), 4);
        sb.lock().sbappendstream(data(b"0123456789", false));
        assert_eq!(nmbufs(&sb), 4);
        assert_eq!(sb.cc(), 810);
        sb.lock().sbflush();

        // A closed record first: the copy is of it alone.
        {
            let mut g = sb.lock();
            g.sbappendrecord(Some(data(b"abc", true)));
            g.sbappendrecord(Some(data(b"def", true)));
            sbcheck(&g);
        }
        let c = sb.lock().copym_first(1, 2, M_WAIT).expect("a copy");
        assert_eq!(bytes_of(&c), b"bc");
        let mut buf = [0u8; 3];
        sb.lock().copydata_first(0, &mut buf);
        assert_eq!(&buf, b"abc");
        assert_eq!(records(&sb).len(), 2);
        sb.lock().sbflush();
    }
}
/* </TESTS> */
