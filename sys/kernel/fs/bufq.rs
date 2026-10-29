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
 * Copyright (c) 2010 Thordur I. Bjornsson <thib@openbsd.org>
 * Copyright (c) 2010 David Gwynne <dlg@openbsd.org>
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
//! Disk buffer queues (`bufq_init(9)`): the queue a disk driver keeps its pending buffers on,
//! with a discipline (`fifo`, or `nscan`, which sorts segments of up to `BUFQ_NSCAN_N`
//! buffers by block number), the high and low water marks that throttle writers
//! (`bufq_wait`, `bufq_done`), and the quiescing of every queue for suspend.
//!
//! Upstream: sys/kern/kern_bufq.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_bufq.rs@3c62ede68802
//!
//! A queued buffer's token is parked in the buffer while it waits ([`super::buf::Buf::park`])
//! and taken back by `bufq_dequeue`; the discipline's lists hold `hal::owned::Member`s, counted
//! for a cache buffer, uncounted for a buffer a driver embeds (`'static`).
//!
//! ## Deviations
//! - The discipline's head (`bufq_data`, `malloc(M_DEVBUF)` by `impl_create`) is embedded in
//!   the queue: `vmstat -m`'s `devbuf` row no longer counts 16 or 32 bytes per disk, and
//!   `bufq_init` cannot fail for want of memory (its "bufq init fail" panic has nothing left
//!   to report: the type made it impossible).
//! - `impl_peek` returns `bool`; `bufqs_stop` is an atomic.
//!
//! ## Redesign
//! - LZ's `struct bufq_impl` of `unsafe fn` pointers over a `NonNull<c_void>` head, the fifo
//!   and nscan heads `malloc`ed and cast, `queue_adapter!` `SIMPLEQ`s over `&'static Buf`
//!   -> the trait [`BufqDiscipline`] with two impls, [`Fifo`] and [`Nscan`], both heads in the
//!   queue's state under its mutex (the queue's `bufq_type` picks one: `create`/`destroy`
//!   are the state's reset, not a dispatch), their lists `hal::owned::SecondaryList`s.
//! - The queue is all zero when unused: a disk's softc, zeroed by autoconf, embeds it, and
//!   `bufq_init` fills it in (`mtx_init(&bq->bufq_mtx, IPL_BIO)`: the mutex at `IPL_NONE`
//!   raises to `IPL_MPFLOOR`, above `IPL_BIO`, as one at `IPL_BIO` does).
//! - `bufqs` is a `hal::list::List` of `&'static` queues (a softc's) under its own mutex.

use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::intr::{IPL_BIO, IPL_NONE};
use hal::list::{List, ListEntry};
use hal::mutex::Mutexed;
use hal::owned::{Member, SecondaryList};
use hal::sleep::{INFSLP, PRIBIO, wakeup};
use hal::sys::errno::Errno;

use super::bio::{BCSTATS, biodone, note_queued};
use super::buf::{
    B_ERROR, BUFQ_FIFO, BUFQ_HI, BUFQ_HOWMANY, BUFQ_LOW, BUFQ_NSCAN, BUFQ_NSCAN_N, Buf, BufRef,
    ByBufhead, ByBufq,
};
use crate::assert::{kassert, kpanic};

/// `struct bufq`: a disk's queue of buffers. All zero (`Bufq::new`) until [`bufq_init`].
pub struct Bufq {
    /// `bufq_entries`: the link in `bufqs`.
    entries: ListEntry<Bufq>,
    /// `bufq_mtx` and what it guards.
    st: Mutexed<BufqState>,
    /// `bufq_outstanding`: buffers queued and not yet done.
    outstanding: AtomicU32,
    /// `bufq_waiting`: the writers sleeping in `bufq_wait` (its address the channel).
    waiting: AtomicI32,
    /// `bufq_stop`: `bufq_quiesce` stopped the queue (its address the channel).
    stop: AtomicI32,
}

/// What `bufq_mtx` guards: the water marks, the discipline and its two heads.
struct BufqState {
    /// `bufq_hi`: `bufq_wait` sleeps at this many outstanding buffers.
    hi: u32,
    /// `bufq_low`: and wakes below this many.
    low: u32,
    /// `bufq_type`: `BUFQ_FIFO` or `BUFQ_NSCAN`.
    kind: i32,
    /// The fifo discipline's head.
    fifo: Fifo,
    /// The nscan discipline's head.
    nscan: Nscan,
}

impl BufqState {
    const fn new() -> Self {
        Self {
            hi: 0,
            low: 0,
            kind: 0,
            fifo: Fifo::new(),
            nscan: Nscan::new(),
        }
    }

    /// `bq->bufq_impl` with `bq->bufq_data`: the queue's discipline.
    fn discipline(&mut self) -> &mut dyn BufqDiscipline {
        match self.kind {
            BUFQ_NSCAN => &mut self.nscan,
            _ => &mut self.fifo,
        }
    }
}

impl Bufq {
    /// A queue with no discipline yet, all zero (a disk's softc holds one, zeroed, until
    /// `bufq_init`).
    pub const fn new() -> Self {
        Self {
            entries: ListEntry::new(),
            st: Mutexed::new(IPL_NONE, BufqState::new()),
            outstanding: AtomicU32::new(0),
            waiting: AtomicI32::new(0),
            stop: AtomicI32::new(0),
        }
    }

    /// `bq->bufq_outstanding`.
    pub fn outstanding(&self) -> u32 {
        self.outstanding.load(Ordering::Relaxed)
    }

    /// `(bq->bufq_hi, bq->bufq_low)`.
    pub fn water_marks(&self) -> (u32, u32) {
        let g = self.st.lock();
        (g.hi, g.low)
    }

    /// `bq->bufq_type`.
    pub fn kind(&self) -> i32 {
        self.st.lock().kind
    }
}

impl Default for Bufq {
    fn default() -> Self {
        Self::new()
    }
}

hal::list_adapter!(
    /// `SLIST_ENTRY(bufq) bufq_entries`: every initialised queue.
    BufqEntries: Bufq, entries
);

/// `bufqs`: every initialised queue, under `bufqs_mtx` (`IPL_NONE`).
static BUFQS: Mutexed<List<BufqEntries>> = Mutexed::new(IPL_NONE, List::new());
/// `bufqs_stop`: `bufq_quiesce` stopped every queue (its address the channel).
static BUFQS_STOP: AtomicI32 = AtomicI32::new(0);

/// `struct bufq_impl`: a queueing discipline. `impl_create` and `impl_destroy` are the
/// head's construction and reset; the token of a buffer queued is parked in the buffer by
/// [`bufq_queue`], so a discipline orders the buffers' links ([`Member`]s).
pub trait BufqDiscipline {
    /// `impl_queue`: adds a buffer.
    fn queue(&mut self, m: Member<ByBufhead>);
    /// `impl_dequeue`: takes the next buffer.
    fn dequeue(&mut self) -> Option<Member<ByBufhead>>;
    /// `impl_peek`: whether a buffer is queued.
    fn peek(&self) -> bool;
}

/// `struct bufq_fifo_head`: the fifo discipline.
pub struct Fifo {
    q: SecondaryList<ByBufq>,
}

impl Fifo {
    const fn new() -> Self {
        Self {
            q: SecondaryList::new(),
        }
    }
}

impl BufqDiscipline for Fifo {
    /// `bufq_fifo_queue`.
    fn queue(&mut self, m: Member<ByBufhead>) {
        self.q.push_tail(m);
    }

    /// `bufq_fifo_dequeue`.
    fn dequeue(&mut self) -> Option<Member<ByBufhead>> {
        self.q.pop_head()
    }

    /// `bufq_fifo_peek`.
    fn peek(&self) -> bool {
        !self.q.is_empty()
    }
}

/// `BUF_INORDER(ba, bb)`.
fn buf_inorder(ba: &Buf, bb: &Buf) -> bool {
    ba.blkno() < bb.blkno()
}

/// `struct bufq_nscan_data`: the nscan discipline.
pub struct Nscan {
    /// `sorted`: the segment being served, by block.
    sorted: SecondaryList<ByBufq>,
    /// `fifo`: the buffers waiting for the next segment.
    fifo: SecondaryList<ByBufq>,
    /// `leftoverroom`: remaining number of buffer inserts allowed.
    leftoverroom: i32,
}

impl Nscan {
    const fn new() -> Self {
        Self {
            sorted: SecondaryList::new(),
            fifo: SecondaryList::new(),
            leftoverroom: 0,
        }
    }

    /// `bufq_simple_nscan(head, bp)`: inserts the buffer in block order: after the last
    /// buffer it is not before (the C's walk to the first slot where it would fit, then the
    /// insertion after the element it passed). The buffer's block (`BUF_INORDER`'s key) is
    /// read once, before the walk.
    fn simple_nscan(head: &mut SecondaryList<ByBufq>, m: Member<ByBufhead>) {
        let blkno = m.get().blkno();
        head.insert_before_first(m, |cur| blkno < cur.blkno());
    }

    /// `bufq_nscan_resort`: takes `BUFQ_NSCAN_N` buffers from the fifo queue and sorts them.
    fn resort(&mut self) {
        let segmentsize = BUFQ_NSCAN_N;
        let mut count = 0;
        while count < segmentsize {
            let Some(m) = self.fifo.pop_head() else {
                break;
            };
            Self::simple_nscan(&mut self.sorted, m);
            count += 1;
        }
        self.leftoverroom = segmentsize - count;
    }
}

impl BufqDiscipline for Nscan {
    /// `bufq_nscan_queue`.
    fn queue(&mut self, m: Member<ByBufhead>) {
        // If the previous sorted segment was small, we will continue packing in bufs as long
        // as they're in order.
        if self.leftoverroom != 0
            && let Some(next) = self.sorted.first()
            && buf_inorder(next, m.get())
        {
            Self::simple_nscan(&mut self.sorted, m);
            self.leftoverroom -= 1;
            return;
        }

        self.fifo.push_tail(m);
    }

    /// `bufq_nscan_dequeue`.
    fn dequeue(&mut self) -> Option<Member<ByBufhead>> {
        if self.sorted.is_empty() {
            self.resort();
        }
        self.sorted.pop_head()
    }

    /// `bufq_nscan_peek`.
    fn peek(&self) -> bool {
        !self.sorted.is_empty() || !self.fifo.is_empty()
    }
}

/// `bufq_init(bq, type)`: sets up a queue with discipline `type_` and links it on `bufqs`.
/// Panics on an unknown discipline, as the C.
pub fn bufq_init(bq: &'static Bufq, type_: i32) -> Result<(), Errno> {
    let mut hi = BUFQ_HI;
    let mut low = BUFQ_LOW;

    if type_ < 0 || type_ as usize >= BUFQ_HOWMANY {
        kpanic(format_args!("bufq_init: type {type_} unknown"));
    }

    // Ensure that writes can't consume the entire amount of kva available the buffer cache if
    // we only have a limited amount of kva available to us.
    let kvaslots = BCSTATS.kvaslots.load(Ordering::Relaxed);
    if i64::from(hi) >= kvaslots / 16 {
        hi = (kvaslots / 16).max(2) as u32;
        low = hi / 2;
    }

    {
        let mut st = bq.st.lock();
        st.hi = hi;
        st.low = low;
        st.kind = type_;
        // impl_create: the discipline's heads are empty.
        st.fifo = Fifo::new();
        st.nscan = Nscan::new();
    }

    let mut g = BUFQS.lock();
    while BUFQS_STOP.load(Ordering::Relaxed) != 0 {
        let _ = g.sleep_nsec(&raw const BUFQS_STOP, PRIBIO, "bqinit", INFSLP);
    }
    g.insert_head(bq);
    Ok(())
}

/// `bufq_destroy(bq)`: drains the queue (failing what is queued) and unlinks it.
pub fn bufq_destroy(bq: &'static Bufq) {
    bufq_drain(bq);

    let mut g = BUFQS.lock();
    while BUFQS_STOP.load(Ordering::Relaxed) != 0 {
        let _ = g.sleep_nsec(&raw const BUFQS_STOP, PRIBIO, "bqdest", INFSLP);
    }
    g.remove(bq);
}

/// `bufq_queue(bq, bp)`: queues the buffer of the token `t` for the disk (the token parked in
/// the buffer until `bufq_dequeue`), sleeping while the queue is stopped.
pub fn bufq_queue(bq: &'static Bufq, t: BufRef) {
    let mut st = bq.st.lock();
    while bq.stop.load(Ordering::Relaxed) != 0 {
        let _ = st.sleep_nsec(&raw const bq.stop, PRIBIO, "bqqueue", INFSLP);
    }

    t.buf().set_bq(Some(bq));
    bq.outstanding.fetch_add(1, Ordering::Relaxed);
    let m = match (t.counted(), t.foreign_buf()) {
        (Some(r), _) => Member::from_ref(r),
        (None, Some(b)) => Some(Member::of_static(b)),
        (None, None) => None,
    };
    let Some(m) = m else {
        kpanic(format_args!("bufq_queue: a consumed token"));
    };
    Buf::park(t);
    st.discipline().queue(m);
    drop(st);
    note_queued(bq);
}

/// `bufq_dequeue(bq)`: the next buffer for the disk, its token taken back.
pub fn bufq_dequeue(bq: &Bufq) -> Option<BufRef> {
    let m = bq.st.lock().discipline().dequeue()?;
    let t = m.get().take_parked();
    drop(m);
    match t {
        Some(t) => Some(t),
        None => kpanic(format_args!(
            "bufq_dequeue: a queued buffer without its token"
        )),
    }
}

/// `bufq_peek(bq)`: whether a buffer is queued.
pub fn bufq_peek(bq: &Bufq) -> bool {
    bq.st.lock().discipline().peek()
}

/// `bufq_drain(bq)`: fails every queued buffer with `ENXIO`.
pub fn bufq_drain(bq: &Bufq) {
    while let Some(t) = bufq_dequeue(bq) {
        t.buf().set_error(Some(Errno::ENXIO));
        t.buf().set(B_ERROR);
        biodone(t);
    }
}

/// `bufq_wait(bq)`: a writer sleeps while the queue has `bufq_hi` buffers outstanding.
pub fn bufq_wait(bq: &Bufq) {
    let mut st = bq.st.lock();
    if st.hi != 0 {
        while bq.outstanding.load(Ordering::Relaxed) >= st.hi {
            bq.waiting.fetch_add(1, Ordering::Relaxed);
            let _ = st.sleep_nsec(&raw const bq.waiting, PRIBIO, "bqwait", INFSLP);
            bq.waiting.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

/// `bufq_done(bq, bp)`: a queued buffer's I/O finished (`biodone`).
pub fn bufq_done(bq: &Bufq, bp: &Buf) {
    let st = bq.st.lock();
    kassert!(bq.outstanding.load(Ordering::Relaxed) > 0);
    let n = bq.outstanding.fetch_sub(1, Ordering::Relaxed) - 1;
    if bq.stop.load(Ordering::Relaxed) != 0 && n == 0 {
        wakeup(&raw const bq.outstanding);
    }
    if bq.waiting.load(Ordering::Relaxed) != 0 && n < st.low {
        wakeup(&raw const bq.waiting);
    }
    drop(st);
    bp.set_bq(None);
}

/// `bufq_quiesce()`: stops every queue and waits for its outstanding I/O.
pub fn bufq_quiesce() {
    let g = BUFQS.lock();
    BUFQS_STOP.store(1, Ordering::Relaxed);
    // We can safely walk the list since it can't be modified as long as bufqs_stop is
    // non-zero.
    let first = g.first();
    drop(g);
    let mut cur = first;
    while let Some(bq) = cur {
        let mut st = bq.st.lock();
        bq.stop.store(1, Ordering::Relaxed);
        while bq.outstanding.load(Ordering::Relaxed) != 0 {
            let _ = st.sleep_nsec(&raw const bq.outstanding, PRIBIO, "bqquies", INFSLP);
        }
        drop(st);
        cur = List::<BufqEntries>::next(bq);
    }
}

/// `bufq_restart()`: lets every queue run again.
pub fn bufq_restart() {
    let g = BUFQS.lock();
    for bq in g.iter() {
        let st = bq.st.lock();
        bq.stop.store(0, Ordering::Relaxed);
        wakeup(&raw const bq.stop);
        drop(st);
    }
    BUFQS_STOP.store(0, Ordering::Relaxed);
    wakeup(&raw const BUFQS_STOP);
}

/// `bufqs` emptied and `bufqs_stop` cleared, without touching the queues (a host test's fresh
/// memory: the queues an earlier test linked are its leaked softcs).
#[cfg(any(test, feature = "doubles"))]
pub fn bufq_test_reset() {
    let g = BUFQS.lock();
    while g.pop_head().is_some() {}
    BUFQS_STOP.store(0, Ordering::Relaxed);
}

const _: () = assert!(BUFQ_FIFO == 0 && IPL_BIO > IPL_NONE);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the disk queues: the fifo order, the nscan sorting of segments and the
    // packing of in-order buffers into a short segment (a table), the water marks and
    // `bufq_drain`, a cache buffer and a driver's buffer through a queue, and the quiescing
    // of the queues with I/O outstanding.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;

    use super::*;
    use crate::fs::bio::{brelse, getblk};
    use crate::fs::buf::{B_DONE, BUFQ_FIFO, BUFQ_NSCAN};
    use crate::testutil::{BufTestVnode, bufcache_install};
    use hal::sleep::INFSLP;
    use hal::sys::types::Daddr;

    /// The queues' tests share the cache's counters (`kvaslots`): one at a time.
    fn setup() -> MutexGuard<'static, ()> {
        bufcache_install(8 << 20, 512)
    }

    /// A driver's buffer for block `blkno`, leaked for the test.
    fn buf(blkno: Daddr) -> &'static Buf {
        let bp: &'static Buf = Box::leak(Box::new(Buf::new()));
        bp.set_blkno(blkno);
        bp
    }

    /// A queue of discipline `type_`, initialised and leaked for the test.
    fn queue(type_: i32) -> &'static Bufq {
        let bq: &'static Bufq = Box::leak(Box::new(Bufq::new()));
        bufq_init(bq, type_).expect("bufq_init");
        bq
    }

    /// Queues the driver's buffers of `blks`.
    fn queue_blocks(bq: &'static Bufq, blks: &[Daddr]) {
        for &b in blks {
            bufq_queue(bq, BufRef::foreign(buf(b)));
        }
    }

    /// Everything left on the queue, in dequeue order, each done.
    fn drain_blocks(bq: &Bufq) -> Vec<Daddr> {
        let mut v = Vec::new();
        while let Some(t) = bufq_dequeue(bq) {
            v.push(t.buf().blkno());
            bufq_done(bq, t.buf());
        }
        v
    }

    #[test]
    fn fifo_serves_in_arrival_order() {
        let _g = setup();
        let bq = queue(BUFQ_FIFO);
        assert!(!bufq_peek(bq));
        queue_blocks(bq, &[30, 10, 20]);
        assert!(bufq_peek(bq));
        assert_eq!(bq.outstanding(), 3);
        assert_eq!(drain_blocks(bq), [30, 10, 20]);
        assert_eq!(bq.outstanding(), 0);
        bufq_destroy(bq);
    }

    #[test]
    fn nscan_sorts_each_segment_by_block() {
        let _g = setup();
        let bq = queue(BUFQ_NSCAN);
        queue_blocks(bq, &[50, 10, 40, 20, 30]);
        // The first dequeue sorts the fifo into a segment.
        let first = bufq_dequeue(bq).expect("a buffer");
        assert_eq!(first.buf().blkno(), 10);
        bufq_done(bq, first.buf());
        drop(first);
        // The segment was short (5 of BUFQ_NSCAN_N), so an in-order arrival joins it...
        queue_blocks(bq, &[35]);
        // ...and one before the segment's head waits for the next segment.
        queue_blocks(bq, &[5]);
        assert_eq!(drain_blocks(bq), [20, 30, 35, 40, 50, 5]);
        bufq_destroy(bq);
    }

    #[test]
    fn nscan_orders_as_the_c_does() {
        let _g = setup();
        // (queued before the first dequeue, queued after it, the whole dequeue order).
        let table: &[(&[Daddr], &[Daddr], &[Daddr])] = &[
            (&[3, 1, 2], &[], &[1, 2, 3]),
            (&[1, 1, 1], &[1], &[1, 1, 1, 1]),
            (&[5, 9], &[7, 2], &[5, 9, 2, 7]),
            (&[2, 4], &[5, 3], &[2, 4, 5, 3]),
            (&[4], &[6, 5], &[4, 5, 6]),
            (&[], &[8, 3], &[3, 8]),
        ];
        for (before, after, want) in table {
            let bq = queue(BUFQ_NSCAN);
            queue_blocks(bq, before);
            let mut got = Vec::new();
            if let Some(t) = bufq_dequeue(bq) {
                got.push(t.buf().blkno());
                bufq_done(bq, t.buf());
            }
            queue_blocks(bq, after);
            got.extend(drain_blocks(bq));
            assert_eq!(&got[..], *want, "{before:?} then {after:?}");
            bufq_destroy(bq);
        }
    }

    #[test]
    fn nscan_sorts_at_most_bufq_nscan_n_at_a_time() {
        let _g = setup();
        let bq = queue(BUFQ_NSCAN);
        let n = BUFQ_NSCAN_N as Daddr;
        // Two segments' worth, in descending order.
        let blks: Vec<Daddr> = (0..n + 3).rev().collect();
        queue_blocks(bq, &blks);
        let got = drain_blocks(bq);
        // The first segment is the first BUFQ_NSCAN_N arrivals, sorted; then the rest, sorted.
        let mut first: Vec<Daddr> = (3..n + 3).collect();
        first.extend([0, 1, 2]);
        assert_eq!(got, first);
        bufq_destroy(bq);
    }

    #[test]
    fn water_marks_follow_the_kva_slots_and_drain_fails_the_rest() {
        let _g = setup();
        // With few kva slots the high mark shrinks to a sixteenth of them (at least 2).
        BCSTATS.kvaslots.store(64, Ordering::Relaxed);
        let bq = queue(BUFQ_FIFO);
        assert_eq!(bq.water_marks(), (4, 2));
        BCSTATS.kvaslots.store(16, Ordering::Relaxed);
        let small = queue(BUFQ_FIFO);
        assert_eq!(small.water_marks(), (2, 1));
        BCSTATS.kvaslots.store(1 << 20, Ordering::Relaxed);
        let big = queue(BUFQ_FIFO);
        assert_eq!(big.water_marks(), (BUFQ_HI, BUFQ_LOW));

        // biodone through bufq_done gives the slot back; drain fails what is still queued.
        let a = buf(1);
        let b = buf(2);
        bufq_queue(bq, BufRef::foreign(a));
        bufq_queue(bq, BufRef::foreign(b));
        let first = bufq_dequeue(bq).expect("a");
        assert!(core::ptr::eq(first.buf(), a));
        biodone(first);
        assert!(a.bq().is_none() && a.isset(B_DONE));
        assert_eq!(bq.outstanding(), 1);
        bufq_drain(bq);
        assert!(b.isset(B_ERROR) && b.isset(B_DONE));
        assert_eq!(b.error(), Some(Errno::ENXIO));
        assert_eq!(bq.outstanding(), 0);

        for q in [bq, small, big] {
            bufq_destroy(q);
        }
    }

    #[test]
    fn a_cache_buffer_waits_on_a_queue_with_its_token_parked() {
        let _g = setup();
        let tv = BufTestVnode::new(64);
        let bq = queue(BUFQ_NSCAN);
        let t = getblk(&tv.bufs, 8, 4096, 0, INFSLP).expect("a buffer");
        let bp: *const Buf = t.buf();
        bufq_queue(bq, t);
        // Parked, counted: the cache cannot free it while it waits.
        let t = bufq_dequeue(bq).expect("the buffer");
        assert!(core::ptr::eq(t.buf(), bp) && !t.buf().is_parked());
        bufq_done(bq, t.buf());
        brelse(t);
        bufq_destroy(bq);
    }

    #[test]
    fn quiesce_waits_for_the_outstanding_io() {
        let _g = setup();
        let bq = queue(BUFQ_FIFO);
        queue_blocks(bq, &[7]);
        let t = bufq_dequeue(bq).expect("the buffer");
        std::thread::scope(|s| {
            let q = s.spawn(bufq_quiesce);
            // The quiescing thread sleeps on the outstanding I/O until it is done.
            while bq.stop.load(Ordering::Relaxed) == 0 {
                std::thread::yield_now();
            }
            bufq_done(bq, t.buf());
            q.join().expect("quiesce");
        });
        assert_eq!(BUFQS_STOP.load(Ordering::Relaxed), 1);
        bufq_restart();
        assert_eq!(BUFQS_STOP.load(Ordering::Relaxed), 0);
        assert_eq!(bq.stop.load(Ordering::Relaxed), 0);
        drop(t);
        bufq_destroy(bq);
    }
}
/* </TESTS> */
