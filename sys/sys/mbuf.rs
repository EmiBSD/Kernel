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
 *	@(#)mbuf.h	8.5 (Berkeley) 2/19/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/mbuf.h>`: memory buffers, the network stack's packets, for `bsd`'s legacy code over
//! the kernel's mbuf (`kernel::net::mbuf`, N6.1).
//!
//! Upstream: sys/sys/mbuf.h @ 3ce1f3f79392
//!
//! The mbuf is the kernel's: an owned pool item (`kernel::net::mbuf::MbufBox`) whose links
//! own the next mbuf and the next packet. `bsd`'s 114 files keep LZ's `struct mbuf *` shape,
//! a `&'static Mbuf`, until their own slices move them; this file is the boundary between the
//! two (the timing rule: `Mbuf`, `m_len()`, `mtod`, `m_freem`, `m_pkthdr()` are used by more
//! than 50 files each). The protocol it keeps:
//!
//! - **An mbuf is in a link, or parked, never both.** An mbuf `bsd` holds in its own memory
//!   (a variable, a queue head, `sb_mb`) is parked (`hal::pool::PoolBox::park`); an mbuf
//!   inside a chain or a packet list is owned by the link before it. [`park`] and [`unpark`]
//!   cross the line; [`m_set_next`] and [`m_set_nextpkt`] move an mbuf from parked into a
//!   link (the one the link held parked, and returned); a free (`m_free`, `m_freem`) takes a
//!   parked mbuf and parks what its links held.
//! - The kernel's in-place operations need `&mut Mbuf`, which a `&'static Mbuf` cannot give
//!   in safe code: [`mut_of`] makes it under the C's one-owner convention, which LZ's `Cell`
//!   writes through `&'static` already relied on.
//!
//! The header's accessors (`m_len()`, `m_type()`, `m_flags()`, `m_pkthdr()`, `m_next()`,
//! `m_nextpkt()`) are the kernel's own methods, under the C's names, so most callers compile
//! unchanged; what the kernel's offsets replace (`m_data()`, `m_ext()`) is [`mtod`],
//! [`m_data_add`], [`m_set_data`], [`m_ext_size`] and [`m_ext_buf`].
//!
//! ## Deviations
//! - `struct pf_state_key *` and `struct inpcb *` in `struct pkthdr_pf` are words pf reads
//!   and writes through the `Pf` bridge (`kernel::net::pf_hooks`, LZ: untyped pointers).
//! - `MGET`, `MGETHDR` and `M_PREPEND` only assign the result of `m_get`, `m_gethdr` and
//!   `m_prepend` to their argument; Rust callers write the assignment (LZ's).
//! - A free of an mbuf that is in a link (not parked) is `panic(9)`: the C would leave its
//!   predecessor pointing at freed memory; the caller takes it off its link first
//!   ([`m_take_next`]).
//! - `struct mbuf_queue` stays `bsd`'s here ([`MbufQueue`], LZ's shape over [`MbufList`]):
//!   `bsd`'s softcs are zero-allocated and `mq_init` sets the queue's mutex level at run
//!   time, which `hal::mutex::Mutexed` (the kernel's `MbufQueue`) fixes at construction.

use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::pool::{Parkable, PoolBox};
pub use kernel::net::mbuf::{
    Ext, M_AUTH, M_BCAST, M_BITS, M_COMP, M_CONF, M_COPYALL, M_COPYFLAGS, M_DONTWAIT, M_EOR, M_EXT,
    M_EXTWR, M_FLOWID, M_ICMP_CSUM_IN_BAD, M_ICMP_CSUM_IN_OK, M_ICMP_CSUM_OUT, M_IPV4_CSUM_IN_BAD,
    M_IPV4_CSUM_IN_OK, M_IPV4_CSUM_OUT, M_IPV6_DF_OUT, M_LINK0, M_LOOP, M_MAXLOOP, M_MCAST,
    M_PKTHDR, M_PROTO1, M_TCP_CSUM_IN_BAD, M_TCP_CSUM_IN_OK, M_TCP_CSUM_OUT, M_TCP_TSO,
    M_TIMESTAMP, M_TUNNEL, M_UDP_CSUM_IN_BAD, M_UDP_CSUM_IN_OK, M_UDP_CSUM_OUT, M_VLANTAG, M_WAIT,
    M_ZEROIZE, MAXMCLBYTES, MCLBYTES, MCLSHIFT, MCS_BITS, MFlags, MHLEN, MINCLSIZE, MLEN, MPF_BITS,
    MSIZE, MT_CONTROL, MT_DATA, MT_FREE, MT_FTABLE, MT_HEADER, MT_NTYPES, MT_OOBDATA, MT_SONAME,
    MT_SOOPTS, MTAG_BITS, MTag, Mbstat, MbstatCounters, Mbuf, MbufBox, PACKET_TAG_CARP_BAL_IP,
    PACKET_TAG_DLT, PACKET_TAG_GRE, PACKET_TAG_IP_OFFNXT, PACKET_TAG_IP6_OFFNXT,
    PACKET_TAG_IPSEC_FLOWINFO, PACKET_TAG_IPSEC_IN_DONE, PACKET_TAG_IPSEC_OUT_DONE,
    PACKET_TAG_MAXSIZE, PACKET_TAG_PF_DIVERT, PACKET_TAG_PF_REASSEMBLED, PACKET_TAG_SRCROUTE,
    PACKET_TAG_TUNNEL, PACKET_TAG_WIREGUARD, PF_TAG_DIVERTED, PF_TAG_DIVERTED_PACKET,
    PF_TAG_GENERATED, PF_TAG_PROCESSED, PF_TAG_REFRAGMENTED, PF_TAG_REROUTE,
    PF_TAG_SYNCOOKIE_RECREATED, PF_TAG_TRANSLATE_LOCALHOST, Pkthdr, PkthdrPf, mbstat_inc,
};

use crate::kern::uipc_mbuf::{m_clget, m_freem};
use crate::sys::mutex::Mutex;

/// The `&'static T` of an item `bsd` holds by address: a parked mbuf or tag
/// (`PoolBox::park`'s pointer), or one inside a chain, a packet list or a tag list whose
/// owner `bsd`'s caller keeps ([`lend`]: LZ's `&'static Mbuf`, valid until `m_free`).
fn of_parked<T: Parkable>(p: NonNull<T>) -> &'static T {
    // SAFETY: `p` names an initialised item of `T::POOL` in one of three places: parked
    // (`park` gave the box up and nothing put it back), or owned by a link of a chain or a
    // packet list (`m_next`, `m_nextpkt`, an `MbufList`, a header's tag list) whose head
    // `bsd`'s caller holds. Each stays allocated until a free takes it back: `unpark` for a
    // parked item (which refuses a linked one), the owner's drop for a linked one, and a
    // linked item is freed only with or after its owner, which `bsd` frees through the
    // adapters only. Lifetime: the C's one-owner convention, which LZ's `&'static Mbuf`
    // made the same promise on: `bsd`'s code uses a reference only while it owns the packet
    // (or the chain or list holding it), and a free is the owner's last use of every
    // reference into it, so no `&'static` is used after its item's free. Threads: `Mbuf` is
    // `Send` but not `Sync` (its cells), and a `&Mbuf` is shared across CPUs as the C hands
    // a packet over: one CPU uses a packet at a time, and the handover is a lock or a queue
    // whose release and acquire order every access of the giver before the taker's
    // (`mq_mtx`, `sb_mtx`, the interface queues' mutexes, the softnet task's queue). One
    // place does more, as the C does: wg's serial queue (`wg_queue_dequeue`, under
    // `q_mtx`) peeks at its head, and the next `wg_queue_in`/`wg_queue_out` links a packet
    // after its tail, while another CPU decrypts one of those packets (`wg_decap`, which
    // writes its flags, its `m_off` and `m_len`, its header's lengths and the WireGuard
    // tag's `t_done`/`t_mbuf`) or encrypts it (`wg_encap`, which writes the tag's). The peek
    // reads the flags word (`hal::overlay::Body`'s, an `AtomicU16` since review C1), the
    // header's `ph_tagsset` and the tag list (`ph_tags` and each tag's `m_tag_link`, atomic
    // words, and each tag's `m_tag_id`, written once before the packet was queued); the
    // enqueue writes the tail's `m_nextpkt` (`OwnedSlist::push_tail` through a shared
    // borrow: an atomic load and store under `&self`, `SlistLink::fill`, no `&mut` to the
    // tail). The other CPU writes the flags (atomically), `m_off`, `m_len`, other cells of
    // the header and the tag's data, never `ph_tagsset`, the tag list or `m_nextpkt`, so the
    // two touch no non-atomic memory in common, but for the tag's `t_done`/`t_mbuf` cells,
    // an inherited race the C has too (a plain int, if_wg.c), N8's (if_wg.rs's
    // Deviations). The reference is shared; `mut_of`'s `&mut` is the convention's other
    // half.
    unsafe { p.as_ref() }
}

/// The `&mut Mbuf` the kernel's in-place operations need (`m_adj`'s callers' `m_copyback`,
/// `m_cat`, `m_split`, the tags, ...), from the `&'static Mbuf` `bsd` holds.
pub(crate) fn mut_of(m: &Mbuf) -> &'static mut Mbuf {
    // SAFETY: the C's one-owner convention (LZ's `Cell` writes through `&'static`): `bsd`'s
    // caller owns the packet `m` belongs to for the call, the kernel function it calls is the
    // only code that uses `m` while the `&mut` lives (one adapter call, never kept), and no
    // other reference to `m` is read or written by the kernel during it. `m` may be inside a
    // packet an `MbufList` holds (`tcp_softlro_concat` appends to the last mbuf of a packet
    // still on its list, as the C's `tcp_softlro_concat` does): the list's chain is the
    // elements' `m_nextpkt` links, plus its `head` and `tail`, and no write through
    // `mut_of` touches the `m_nextpkt` of a list's element: the in-place operations change
    // data, lengths, offsets, the header and `m_next` chains, and the `m_nextpkt` writers
    // that go through `mut_of` (`m_set_nextpkt` and `m_take_nextpkt`, which the adapters'
    // `m_cat` calls too) act on parked packets and on the socket buffers' and nfs's records,
    // which no `MbufList` holds. The adapters' `m_free` and `ml_enqueue` write the packet's
    // own link through its box, and `ml_enqueue` the tail element's through `change()` (the
    // list's own `push_tail`), not through `mut_of`. So the list's head, tail, length and
    // `m_nextpkt` chain change only through the list. The item is alive (as for
    // `of_parked`).
    unsafe { &mut *NonNull::from(m).as_ptr() }
}

/// Parks `b` (`bsd` holds it by address from now on): the `struct mbuf *` LZ's callers keep.
pub fn park<T: Parkable>(b: PoolBox<T>) -> &'static T {
    of_parked(PoolBox::park(b))
}

/// The box of the parked `m` back (the mbuf leaves `bsd`'s hands for the kernel's).
/// `panic(9)` for one that is not parked: a second free, or a free of an mbuf still in a
/// link, which the C would corrupt the chain on.
pub fn unpark<T: Parkable>(m: &T) -> PoolBox<T> {
    match PoolBox::unpark(lend(m)) {
        Some(b) => b,
        None => crate::kern::subr_prf::panic(format_args!(
            "mbuf: an item freed or linked that bsd does not hold (not parked)"
        )),
    }
}

/// `m` as the `&'static` LZ's callers keep (a parked mbuf, or one of a chain they hold).
pub fn lend<T: Parkable>(m: &T) -> &'static T {
    of_parked(NonNull::from(m))
}

/// `mtod(m, t)`: convert mbuf pointer to data pointer of correct type.
#[inline]
pub fn mtod<T>(m: &Mbuf) -> *mut T {
    m.data_raw().cast::<T>()
}

/// `m->m_data += n` (`-=` for a negative `n`).
#[inline]
pub fn m_data_add(m: &Mbuf, n: isize) {
    m.data_add(n);
}

/// `m->m_data = p`, `p` inside the mbuf's data storage (`panic(9)` otherwise).
pub fn m_set_data(m: &Mbuf, p: *mut u8) {
    let base = m.storage_raw().addr();
    match p.addr().checked_sub(base) {
        Some(off) => m.set_data_off(off),
        None => crate::kern::subr_prf::panic(format_args!("m_data: an address before the storage")),
    }
}

/// `m->m_data = M_DATABUF(m) + off`: the data at `off` bytes into the storage (`ext_buf`,
/// `m_pktdat` or `m_dat`).
#[inline]
pub fn m_set_data_off(m: &Mbuf, off: usize) {
    m.set_data_off(off);
}

/// `m->m_ext.ext_size`: the cluster's size, 0 without `M_EXT`.
#[inline]
pub fn m_ext_size(m: &Mbuf) -> u32 {
    m.ext().map_or(0, Ext::ext_size)
}

/// `m->m_ext.ext_buf`: the cluster's address, null without `M_EXT`.
#[inline]
pub fn m_ext_buf(m: &Mbuf) -> *mut u8 {
    m.ext().map_or(core::ptr::null_mut(), Ext::ext_buf)
}

/// `link = n` for the setters below: `n` (parked, or none) into the link, the mbuf the link
/// held before parked and returned. `n` already in the link (the C's `m->m_next = m->m_next`,
/// which the socket buffer's walks write when the next mbuf is its head) leaves it there and
/// returns none.
fn set_link(
    link: &mut hal::owned::SlistLink<Mbuf>,
    n: Option<&'static Mbuf>,
) -> Option<&'static Mbuf> {
    if let Some(n) = n
        && link.get().is_some_and(|held| core::ptr::eq(held, n))
    {
        return None;
    }
    let old = link.take();
    if let Some(n) = n {
        drop(link.set(unpark(n)));
    }
    old.map(park)
}

/// `m->m_next = n`: `n` (parked, or none) linked after `m`; the mbuf the link held before is
/// parked and returned (`bsd`'s callers that read `m->m_next` before overwriting it keep a
/// valid reference to it).
pub fn m_set_next(m: &Mbuf, n: Option<&'static Mbuf>) -> Option<&'static Mbuf> {
    set_link(&mut mut_of(m).m_next, n)
}

/// `n = m->m_next; m->m_next = NULL`: the next mbuf taken off its link, parked.
pub fn m_take_next(m: &Mbuf) -> Option<&'static Mbuf> {
    m_set_next(m, None)
}

/// `m->m_nextpkt = n`: as [`m_set_next`], for the packet link.
pub fn m_set_nextpkt(m: &Mbuf, n: Option<&'static Mbuf>) -> Option<&'static Mbuf> {
    set_link(&mut mut_of(m).m_nextpkt, n)
}

/// `n = m->m_nextpkt; m->m_nextpkt = NULL`: the next packet taken off its link, parked.
pub fn m_take_nextpkt(m: &Mbuf) -> Option<&'static Mbuf> {
    m_set_nextpkt(m, None)
}

/// `struct mbuf_list`: packets linked through `m_nextpkt`, the kernel's list
/// (`OwnedSlist<MPkt>`) in a cell `bsd`'s callers change through `&MbufList` under the
/// list's lock, as the C changes its `struct mbuf_list` through a pointer; its length
/// mirrored in an atomic for the readers that take no lock (`ifiq_len`'s and `mq_len`'s
/// `READ_ONCE`, LZ's `ml_len: AtomicU32`).
pub struct MbufList {
    /// The list. Reached only through [`MbufList::change`], under the list's lock.
    list: UnsafeCell<kernel::net::mbuf::MbufList>,
    /// `ml_len`: the list's length, stored by every change, read without the lock.
    len: AtomicU32,
}

// SAFETY: the only access to `list` is `change`, which builds the one `&mut` to it for one
// closure and drops it before returning. Its callers are this file's `mbuf_list_first` and
// `kern/uipc_mbuf.rs`'s `ml_*` and `mq_*` (no other caller exists today),
// and those are called as the C calls them: under the lock the C names for the list
// (`mq_mtx` inside the `mq_*`, `sb_mtx`, `ifq_mtx` and `ifiq_mtx` for the interface queues
// and their disciplines, wg's `q_mtx`, a pf queue's `ifq_mtx`), or on a list the caller
// owns alone (a local `MbufList` on its stack: `if_input_process`'s, `tcp_softlro`'s,
// `mq_purge`'s). So no two CPUs, and no two calls, hold a `&mut` to one list at once; the
// readers that take no lock (`ml_len`, `ml_empty`, `mq_len`, `ifiq_len`) read `len`, an
// atomic, never `list`. The elements are `Send` (the kernel's `Mbuf`), so a list handed
// between CPUs under its lock is sound.
unsafe impl Sync for MbufList {}

impl MbufList {
    /// `MBUF_LIST_INITIALIZER()`.
    pub const fn new() -> Self {
        Self {
            list: UnsafeCell::new(kernel::net::mbuf::MbufList::new()),
            len: AtomicU32::new(0),
        }
    }

    /// One change (or locked read) of the kernel's list, `f` given the list for the call
    /// alone; the length mirrored after it. Callers hold the list's lock or own the list.
    pub(crate) fn change<R>(&self, f: impl FnOnce(&mut kernel::net::mbuf::MbufList) -> R) -> R {
        // SAFETY: `Sync`'s argument above: the caller holds the list's lock or owns the list
        // alone, and this `&mut` is the only reference to `list` while it lives: it is made
        // here, lent to `f` for the call, used once more for the length and dropped before
        // the return (`f` cannot keep it: its lifetime is the call's). `f` runs no `ml_*` of
        // this same list (the adapters' closures are the kernel's list operations; `ml_enlist`
        // refuses one list as both arguments before nesting two calls).
        let list = unsafe { &mut *self.list.get() };
        let r = f(list);
        self.len.store(list.len() as u32, Ordering::Relaxed);
        r
    }

    /// `ml_len`, as `READ_ONCE` reads it: no lock.
    #[inline]
    pub fn len(&self) -> u32 {
        self.len.load(Ordering::Relaxed)
    }

    /// `ml_empty`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `MBUF_LIST_FOREACH(ml, m)`.
    pub fn iter(&self) -> MbufListIter {
        MbufListIter {
            m: mbuf_list_first(self),
        }
    }
}

impl Default for MbufList {
    fn default() -> Self {
        Self::new()
    }
}

/// The iterator behind `MBUF_LIST_FOREACH`.
pub struct MbufListIter {
    m: Option<&'static Mbuf>,
}

impl Iterator for MbufListIter {
    type Item = &'static Mbuf;

    fn next(&mut self) -> Option<&'static Mbuf> {
        let m = self.m?;
        self.m = mbuf_list_next(m);
        Some(m)
    }
}

/// `struct mbuf_queue`: a locked, bounded `mbuf_list` (LZ's shape: see the module's
/// deviations).
pub struct MbufQueue {
    /// `mq_mtx`.
    pub mq_mtx: Mutex,
    /// `mq_list`. Protected by: `mq_mtx`.
    pub mq_list: MbufList,
    /// `mq_maxlen`. Written under `mq_mtx`, read with `READ_ONCE`.
    pub mq_maxlen: AtomicU32,
    /// `mq_drops`. Written under `mq_mtx`, read with `READ_ONCE`.
    pub mq_drops: AtomicU32,
}

impl MbufQueue {
    /// `MBUF_QUEUE_INITIALIZER(maxlen, ipl)`.
    pub const fn new(maxlen: u32, ipl: i32) -> Self {
        Self {
            mq_mtx: Mutex::new(ipl),
            mq_list: MbufList::new(),
            mq_maxlen: AtomicU32::new(maxlen),
            mq_drops: AtomicU32::new(0),
        }
    }
}

/// `MCLGET(m, how)`: allocates and adds an mbuf cluster to a normal mbuf; the flag `M_EXT` is
/// set upon success.
pub fn mclget(m: &'static Mbuf, how: i32) {
    let _ = m_clget(Some(m), how, MCLBYTES as u32);
}

/// `MCLGETL(m, how, l)`: as `MCLGET`, for a cluster of at least `l` bytes.
pub fn mclgetl(m: &'static Mbuf, how: i32, l: u32) -> Option<&'static Mbuf> {
    m_clget(Some(m), how, l)
}

/// `m_extreferenced`: whether the cluster is shared with another mbuf; `false` for a plain
/// mbuf.
#[inline]
pub fn m_extreferenced(m: &Mbuf) -> bool {
    m.ext().is_some_and(Ext::is_shared)
}

/// `MCLISREFERENCED(m)`.
#[inline]
pub fn mclisreferenced(m: &Mbuf) -> bool {
    m_extreferenced(m)
}

/// `M_MOVE_HDR(to, from)`: move just `m_pkthdr` from `from` to `to`, remove `M_PKTHDR` and
/// clean flags/tags for `from`.
pub fn m_move_hdr(to: &Mbuf, from: &Mbuf) {
    kernel::net::mbuf::m_move_hdr(mut_of(to), mut_of(from));
}

/// `M_MOVE_PKTHDR(to, from)`: move mbuf pkthdr from `from` to `to`. `from` must have
/// `M_PKTHDR` set, and `to` must be empty.
pub fn m_move_pkthdr(to: &Mbuf, from: &Mbuf) {
    kernel::net::mbuf::m_move_pkthdr(mut_of(to), mut_of(from));
}

/// `M_READONLY(m)`: determine if an mbuf's data area is read-only. This is true for
/// non-cluster external storage and for clusters that are being referenced by more than one
/// mbuf.
#[inline]
pub fn m_readonly(m: &Mbuf) -> bool {
    m.is_readonly()
}

/// `m_freemp`: frees the packet `*mp` and clears the caller's pointer.
pub fn m_freemp(mp: &mut Option<&'static Mbuf>) -> Option<&'static Mbuf> {
    let m = mp.take();
    m_freem(m)
}

/// `ml_len(ml)`: the mirrored length, without the list's lock (`READ_ONCE`).
#[inline]
pub fn ml_len(ml: &MbufList) -> u32 {
    ml.len()
}

/// `ml_empty(ml)`.
#[inline]
pub fn ml_empty(ml: &MbufList) -> bool {
    ml.is_empty()
}

/// `MBUF_LIST_FIRST(ml)`, under the list's lock (or on a list the caller owns).
#[inline]
pub fn mbuf_list_first(ml: &MbufList) -> Option<&'static Mbuf> {
    ml.change(|l| l.first().map(lend))
}

/// `MBUF_LIST_NEXT(m)`.
#[inline]
pub fn mbuf_list_next(m: &Mbuf) -> Option<&'static Mbuf> {
    m.m_nextpkt().get().map(lend)
}

/// `mq_len(mq)`: the list's mirrored length, without `mq_mtx` (`READ_ONCE`).
#[inline]
pub fn mq_len(mq: &MbufQueue) -> u32 {
    mq.mq_list.len()
}

/// `mq_empty(mq)`.
#[inline]
pub fn mq_empty(mq: &MbufQueue) -> bool {
    mq_len(mq) == 0
}

/// `mq_full(mq)`.
#[inline]
pub fn mq_full(mq: &MbufQueue) -> bool {
    mq_len(mq) >= mq.mq_maxlen.load(Ordering::Relaxed)
}

/// `mq_drops(mq)`.
#[inline]
pub fn mq_drops(mq: &MbufQueue) -> u32 {
    mq.mq_drops.load(Ordering::Relaxed)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_lp64_openbsd() {
        assert_eq!(size_of::<Mbuf>(), MSIZE);
        assert_eq!(MLEN, 224);
        assert_eq!(size_of::<Pkthdr>(), 80);
        assert_eq!(MHLEN, 144);
        assert_eq!(MINCLSIZE, 369);
        assert_eq!(size_of::<MTag>(), PACKET_TAG_MAXSIZE + 16);
    }

    #[test]
    fn a_parked_mbuf_moves_into_a_link_and_back() {
        let _g = crate::kern::uipc_mbuf::tests::setup();
        let out = kernel::net::mbuf::MBUFPL.nout();

        let h = crate::kern::uipc_mbuf::m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        let n = crate::kern::uipc_mbuf::m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        assert!(m_set_next(h, Some(n)).is_none(), "the link was empty");
        assert!(core::ptr::eq(h.m_next().get().expect("linked"), n));
        // Setting the link to what it holds keeps it there (`m->m_next = m->m_next`).
        assert!(m_set_next(h, Some(n)).is_none());
        assert!(core::ptr::eq(h.m_next().get().expect("still linked"), n));
        // In the link, n is not parked: `unpark` (the free's first step) refuses it. The
        // refusal itself is `panic(9)`, which a host test must not unwind through: it would
        // leave the panic path's mutexes to the tests after it.
        assert!(
            PoolBox::unpark(lend(n)).is_none(),
            "an mbuf in a link is not parked"
        );
        // Taken off the link, it is parked again and frees.
        let back = m_take_next(h).expect("the next mbuf");
        assert!(core::ptr::eq(back, n) && h.m_next().get().is_none());
        assert!(crate::kern::uipc_mbuf::m_free(back).is_none());
        assert!(crate::kern::uipc_mbuf::m_free(h).is_none());
        assert_eq!(kernel::net::mbuf::MBUFPL.nout(), out);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/mbuf.h");
        let ours: &[(&str, i64)] = &[
            ("MSIZE", MSIZE as i64),
            ("MAXMCLBYTES", MAXMCLBYTES as i64),
            ("MCLSHIFT", MCLSHIFT as i64),
            ("M_EXT", i64::from(M_EXT)),
            ("M_PKTHDR", i64::from(M_PKTHDR)),
            ("M_EOR", i64::from(M_EOR)),
            ("M_EXTWR", i64::from(M_EXTWR)),
            ("M_PROTO1", i64::from(M_PROTO1)),
            ("M_VLANTAG", i64::from(M_VLANTAG)),
            ("M_LOOP", i64::from(M_LOOP)),
            ("M_BCAST", i64::from(M_BCAST)),
            ("M_MCAST", i64::from(M_MCAST)),
            ("M_CONF", i64::from(M_CONF)),
            ("M_AUTH", i64::from(M_AUTH)),
            ("M_TUNNEL", i64::from(M_TUNNEL)),
            ("M_ZEROIZE", i64::from(M_ZEROIZE)),
            ("M_COMP", i64::from(M_COMP)),
            ("M_LINK0", i64::from(M_LINK0)),
            ("M_IPV4_CSUM_OUT", i64::from(M_IPV4_CSUM_OUT)),
            ("M_TCP_CSUM_OUT", i64::from(M_TCP_CSUM_OUT)),
            ("M_UDP_CSUM_OUT", i64::from(M_UDP_CSUM_OUT)),
            ("M_IPV4_CSUM_IN_OK", i64::from(M_IPV4_CSUM_IN_OK)),
            ("M_IPV4_CSUM_IN_BAD", i64::from(M_IPV4_CSUM_IN_BAD)),
            ("M_TCP_CSUM_IN_OK", i64::from(M_TCP_CSUM_IN_OK)),
            ("M_TCP_CSUM_IN_BAD", i64::from(M_TCP_CSUM_IN_BAD)),
            ("M_UDP_CSUM_IN_OK", i64::from(M_UDP_CSUM_IN_OK)),
            ("M_UDP_CSUM_IN_BAD", i64::from(M_UDP_CSUM_IN_BAD)),
            ("M_ICMP_CSUM_OUT", i64::from(M_ICMP_CSUM_OUT)),
            ("M_ICMP_CSUM_IN_OK", i64::from(M_ICMP_CSUM_IN_OK)),
            ("M_ICMP_CSUM_IN_BAD", i64::from(M_ICMP_CSUM_IN_BAD)),
            ("M_IPV6_DF_OUT", i64::from(M_IPV6_DF_OUT)),
            ("M_TIMESTAMP", i64::from(M_TIMESTAMP)),
            ("M_FLOWID", i64::from(M_FLOWID)),
            ("M_TCP_TSO", i64::from(M_TCP_TSO)),
            ("PF_TAG_GENERATED", i64::from(PF_TAG_GENERATED)),
            (
                "PF_TAG_SYNCOOKIE_RECREATED",
                i64::from(PF_TAG_SYNCOOKIE_RECREATED),
            ),
            (
                "PF_TAG_TRANSLATE_LOCALHOST",
                i64::from(PF_TAG_TRANSLATE_LOCALHOST),
            ),
            ("PF_TAG_DIVERTED", i64::from(PF_TAG_DIVERTED)),
            ("PF_TAG_DIVERTED_PACKET", i64::from(PF_TAG_DIVERTED_PACKET)),
            ("PF_TAG_REROUTE", i64::from(PF_TAG_REROUTE)),
            ("PF_TAG_REFRAGMENTED", i64::from(PF_TAG_REFRAGMENTED)),
            ("PF_TAG_PROCESSED", i64::from(PF_TAG_PROCESSED)),
            ("MT_FREE", i64::from(MT_FREE)),
            ("MT_DATA", i64::from(MT_DATA)),
            ("MT_HEADER", i64::from(MT_HEADER)),
            ("MT_SONAME", i64::from(MT_SONAME)),
            ("MT_SOOPTS", i64::from(MT_SOOPTS)),
            ("MT_FTABLE", i64::from(MT_FTABLE)),
            ("MT_CONTROL", i64::from(MT_CONTROL)),
            ("MT_OOBDATA", i64::from(MT_OOBDATA)),
            ("MT_NTYPES", MT_NTYPES as i64),
            ("M_COPYALL", i64::from(M_COPYALL)),
            (
                "PACKET_TAG_IPSEC_IN_DONE",
                i64::from(PACKET_TAG_IPSEC_IN_DONE),
            ),
            (
                "PACKET_TAG_IPSEC_OUT_DONE",
                i64::from(PACKET_TAG_IPSEC_OUT_DONE),
            ),
            (
                "PACKET_TAG_IPSEC_FLOWINFO",
                i64::from(PACKET_TAG_IPSEC_FLOWINFO),
            ),
            ("PACKET_TAG_IP_OFFNXT", i64::from(PACKET_TAG_IP_OFFNXT)),
            ("PACKET_TAG_IP6_OFFNXT", i64::from(PACKET_TAG_IP6_OFFNXT)),
            ("PACKET_TAG_WIREGUARD", i64::from(PACKET_TAG_WIREGUARD)),
            ("PACKET_TAG_GRE", i64::from(PACKET_TAG_GRE)),
            ("PACKET_TAG_DLT", i64::from(PACKET_TAG_DLT)),
            ("PACKET_TAG_PF_DIVERT", i64::from(PACKET_TAG_PF_DIVERT)),
            (
                "PACKET_TAG_PF_REASSEMBLED",
                i64::from(PACKET_TAG_PF_REASSEMBLED),
            ),
            ("PACKET_TAG_SRCROUTE", i64::from(PACKET_TAG_SRCROUTE)),
            ("PACKET_TAG_TUNNEL", i64::from(PACKET_TAG_TUNNEL)),
            ("PACKET_TAG_CARP_BAL_IP", i64::from(PACKET_TAG_CARP_BAL_IP)),
            ("PACKET_TAG_MAXSIZE", PACKET_TAG_MAXSIZE as i64),
            ("M_MAXLOOP", i64::from(M_MAXLOOP)),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
