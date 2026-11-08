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
//! Host microbenchmarks of the network stack's runtime in `hal` (decision 43 with U10, N6.0):
//! the net lock, the pool items, the byte views of the protocol headers and the owned lists the network
//! slices (N6.1-N6.7) build on, over the kernel's host doubles (`kernel::testutil`, feature
//! `doubles`) and a recycling pool backend (vm.rs's: the doubles' pool hands out a fresh page
//! per item). `sys/benches/net.rs` (N6.0's first commit to its last) timed the net lock's and
//! `smr(9)`'s rows on LZ's shape by the same names; their numbers are in N6.0's commits. Never part of a kernel image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench net` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is:
//! - `netlock.write`: the net lock's write witness taken and dropped
//!   (`hal::netlock::NETLOCK.write()`), a free lock, the one running thread.
//! - `netlock.read`: its read witness taken and dropped (`NET_LOCK_SHARED()` and
//!   `NET_UNLOCK_SHARED()`'s work).
//! - `netlock.paired`: `hal::netlock::lock()` and `unlock()`, the C's `NET_LOCK()` and
//!   `NET_UNLOCK()` (the witness forgotten, then resumed and dropped).
//! - `smr.read`: a read section opened and closed (`hal::smr::read()` and its drop: the
//!   depth's load and store each way).
//! - `smr.load`: `SmrPtr::load` of a published object inside one read section.
//! - `smr.defer`: `hal::smr::defer_link` of an idle `'static` link on a CPU that is not idle
//!   (`smr_call`): claimed and queued on the CPU's list at `splhigh`; between rounds,
//!   untimed, the links are dispatched, collected, run (a call that does nothing) and made
//!   idle again (`smr_init`).
//! - `pool.get_put`: a `BoxPool` item of an mbuf's size (256 bytes) taken and dropped (put
//!   back), on the recycling backend.
//! - `pool.get_put.sourced`: the same, from a pool made with a page source
//!   (`BoxPool::init_with_source`, the mbuf clusters' shape): the source is asked only when
//!   the pool grows, so an op costs what `pool.get_put` does.
//! - `pod.copy`: an IPv4 header (20 bytes, an `abi_pod!` image) copied out of a packet's
//!   bytes at offset 16, one field changed, written back: the C's `m_copydata` and
//!   `m_copyback` of a header.
//! - `pod.view`: the same two fields changed in place through `hal::pod::view_mut` (the C's
//!   `mtod(m, struct ip *)`): the bounds and alignment checks, no copy.
//! - `slist.push_tail`: one element pushed at the tail of an `OwnedSlist` (`ml_enqueue`,
//!   `sbappend`'s tail: the link of the last element through the tail word); the list
//!   drained between rounds, untimed.
//! - `slist.concat`: a list of two elements joined at the tail of another (`ml_enlist`,
//!   `m_cat`'s chain), 1,000 in a row; the lists made again between rounds, untimed.
//! - `list.push`: one element pushed at the tail of an `OwnedList` (`TAILQ_INSERT_TAIL`); the
//!   list drained between rounds, untimed.
//! - The socket layer's rows (N6.2a, decision 43 with U10), over the doubles' thread (its
//!   descriptor table, user memory and credentials), the mbufs and the UNIX domain, the
//!   data copied from and to kernel buffers (`UIO_SYSSPACE` through hal's kcopy,
//!   `testutil::set_copy_kernel`, as bsd's uiomove copies them in the "before" rows):
//!   - `sockbuf.append.drop`: a 100-byte mbuf got, appended to a stream buffer
//!     (`sbappendstream` under the buffer's mutex) and dropped from it (`sbdrop`, a second
//!     mutex section): the TCP input and ack paths' buffer work.
//!   - `soreceive.4k`: `soreceive` of 4 KiB from a UNIX stream socket whose peer sent them
//!     (the send untimed).
//!   - `soreceive.peek.4k`: `soreceive` with `MSG_PEEK` of the 4 KiB a UNIX stream socket
//!     holds (sent once, untimed; a peek takes nothing): the walk down the record.
//!   - `soreceive.peek.1`: the same peek of one byte: `soreceive`'s work around the copy
//!     (`sblock`, the mutex sections, the record taken out and put back, the cursor),
//!     the 4 KiB copy left out.
//!   - `sockbuf.record.out.in`: the receive buffer's first record (those 4 KiB) taken out
//!     and put back under one mutex section (`take_first_record`, `put_back_record`).
//!   - `unix.socketpair.send.recv.64`: `sosend` of 64 bytes on one end of a UNIX stream
//!     pair and `soreceive` of them on the other.
//!   - `solock.unlock.unix`: `solock` and its drop on a UNIX socket (`so_lock`'s write
//!     witness).
//!   - `socket.create.close.unix`: `socreate` of a UNIX stream socket and `soclose` (the
//!     socket and its control block taken from and given back to their pools).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::netlock::{self, NETLOCK};
use hal::owned::{OwnedEntry, OwnedList, OwnedSlist, SlistLink};
use hal::pool::{BoxPool, PR_WAITOK, PageSource, PoolBackend, PoolBox, RawPool};
use hal::rwlock::RawRwlock;
use hal::smr::{self, CpuWalk, SmrLink, SmrLinked, SmrPtr};
use kernel::net::domain::Domain;
use kernel::net::mbuf::{
    M_COPYALL, M_DONTWAIT, MT_DATA, MbufBox, PACKET_TAG_DLT, PACKET_TAG_GRE, m_adj, m_copydata,
    m_copym, m_get, m_gethdr, m_pullup, mbinit, mclget, mclgetl,
};
use kernel::net::mbuf2::{m_tag_find, m_tag_get, m_tag_prepend};
use kernel::net::unix::Unix;
use kernel::testutil;
use kernel::uio::{Iovec, Uio, UioRw, UioSeg};

/// Ops per repetition.
const OPS: usize = 100_000;

/// Elements per round of the list benches.
const ELEMS: usize = 1000;

/// Repetitions per bench.
const REPS: usize = 7;

/// ns per op of `REPS` repetitions of `f` (which returns the time of `ops` ops): min, median,
/// max.
fn measure(ops: usize, mut f: impl FnMut() -> Duration) -> (f64, f64, f64) {
    black_box(f());
    let mut v: Vec<f64> = (0..REPS)
        .map(|_| f().as_nanos() as f64 / ops as f64)
        .collect();
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1])
}

// ---------------------------------------------------------------------------------------
// A pool backend that recycles its items (vm.rs's), registered before the doubles.

/// A page of the host.
const PAGE: usize = 4096;

/// A free list of items of one size, as addresses (the provenance exposed when they were
/// carved or given back).
struct FreeList {
    stride: usize,
    free: Mutex<Vec<usize>>,
    /// Where the slabs come from: the host (`fresh`), or a pool's page source (a page of
    /// `PAGE` bytes at a time).
    source: Option<&'static dyn PageSource>,
}

impl FreeList {
    const fn new(stride: usize, source: Option<&'static dyn PageSource>) -> Self {
        Self {
            stride,
            free: Mutex::new(Vec::new()),
            source,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<usize>> {
        self.free
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// An item: one given back, or a fresh slab carved into items.
    fn get(&self) -> Option<NonNull<u8>> {
        let mut free = self.lock();
        if free.is_empty() {
            if let Some(src) = self.source {
                let page = src.alloc(PAGE, true, true)?.as_ptr();
                for i in 0..(PAGE / self.stride).max(1) {
                    free.push(page.wrapping_add(i * self.stride).expose_provenance());
                }
                return NonNull::new(std::ptr::with_exposed_provenance_mut::<u8>(free.pop()?));
            }
            let per = (64 * 1024 / self.stride).max(1);
            let mut rest: &'static mut [u8] = fresh(per * self.stride);
            while rest.len() >= self.stride {
                let (item, tail) = rest.split_at_mut(self.stride);
                free.push(
                    NonNull::from(item)
                        .cast::<u8>()
                        .as_ptr()
                        .expose_provenance(),
                );
                rest = tail;
            }
        }
        let addr = free.pop()?;
        NonNull::new(std::ptr::with_exposed_provenance_mut::<u8>(addr))
    }

    fn put(&self, item: NonNull<u8>) {
        self.lock().push(item.as_ptr().expose_provenance());
    }
}

/// `len` bytes of fresh, zeroed, page-aligned memory, leaked.
fn fresh(len: usize) -> &'static mut [u8] {
    // A byte block one page longer, cut at its first page boundary.
    let block: &'static mut [u8] = vec![0u8; len + PAGE].leak();
    let skip = block.as_ptr().addr().next_multiple_of(PAGE) - block.as_ptr().addr();
    let (_, tail) = block.split_at_mut(skip);
    let (run, _) = tail.split_at_mut(len);
    run
}

/// One pool of the recycling double.
struct BenchPool {
    items: FreeList,
    size: usize,
    nout: AtomicU64,
}

impl RawPool for BenchPool {
    fn get(&self, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        let p = self.items.get()?;
        self.nout.fetch_add(1, Ordering::Relaxed);
        Some((p, self.size))
    }

    fn put(&self, item: NonNull<u8>) {
        self.nout.fetch_sub(1, Ordering::Relaxed);
        self.items.put(item);
    }

    fn nout(&self) -> u64 {
        self.nout.load(Ordering::Relaxed)
    }

    fn size(&self) -> usize {
        self.size
    }

    fn set_hiwat(&self, _n: u32) {}

    fn cache_init(&self) {}
}

/// The recycling pool backend.
struct BenchPools;

impl PoolBackend for BenchPools {
    fn create(
        &self,
        size: usize,
        align: usize,
        _ipl: i32,
        _flags: i32,
        _wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        if align > PAGE {
            return None;
        }
        let size = size.max(1);
        let stride = size.next_multiple_of(align.max(16));
        Some(Box::leak(Box::new(BenchPool {
            items: FreeList::new(stride, None),
            size,
            nout: AtomicU64::new(0),
        })))
    }

    /// A pool carving its items from `source`'s pages (`pool_init` with an allocator).
    fn create_with(
        &self,
        size: usize,
        align: usize,
        _ipl: i32,
        _flags: i32,
        _wchan: &'static str,
        source: &'static dyn PageSource,
    ) -> Option<&'static dyn RawPool> {
        if align > PAGE {
            return None;
        }
        let size = size.max(1);
        let stride = size.next_multiple_of(align.max(16));
        Some(Box::leak(Box::new(BenchPool {
            items: FreeList::new(stride, Some(source)),
            size,
            nout: AtomicU64::new(0),
        })))
    }
}

/// The page source of `pool.get_put.sourced`: host pages, counted (the mbuf clusters'
/// `m_pool_allocator` charges each against its limit).
struct BenchSource {
    pages: AtomicU64,
}

impl PageSource for BenchSource {
    fn alloc(&self, len: usize, _aligned: bool, _wait: bool) -> Option<NonNull<u8>> {
        self.pages.fetch_add(1, Ordering::Relaxed);
        Some(NonNull::from(fresh(len)).cast::<u8>())
    }

    fn free(&self, _page: NonNull<u8>, _len: usize, _aligned: bool) {
        self.pages.fetch_sub(1, Ordering::Relaxed);
    }

    fn sizes(&self) -> usize {
        PAGE | 1
    }
}

static BENCH_SOURCE: BenchSource = BenchSource {
    pages: AtomicU64::new(0),
};

static BENCH_POOLS: BenchPools = BenchPools;

// ---------------------------------------------------------------------------------------
// The items.

/// An mbuf's size (`MSIZE`).
const MSIZE: usize = 256;

/// The pool of `pool.get_put`: items of an mbuf's size.
static MBUFS: BoxPool<[u8; MSIZE]> = BoxPool::new();

/// The pool of `pool.get_put.sourced`: the same items, their pages from a source.
static SOURCED: BoxPool<[u8; MSIZE]> = BoxPool::new();

/// `struct ip` (netinet/ip.h): 20 bytes, its two 4-bit fields as one byte.
#[derive(Clone, Copy)]
#[repr(C)]
struct Ip {
    ip_vhl: u8,
    ip_tos: u8,
    ip_len: u16,
    ip_id: u16,
    ip_off: u16,
    ip_ttl: u8,
    ip_p: u8,
    ip_sum: u16,
    ip_src: u32,
    ip_dst: u32,
}
hal::abi_pod!(Ip {
    ip_vhl: u8,
    ip_tos: u8,
    ip_len: u16,
    ip_id: u16,
    ip_off: u16,
    ip_ttl: u8,
    ip_p: u8,
    ip_sum: u16,
    ip_src: u32,
    ip_dst: u32
});

/// Where the header starts in the packet's bytes (an aligned offset: the C's `ETHER_ALIGN`
/// puts the IP header on a 4-byte boundary).
const IP_OFF: usize = 16;

/// An element of `list.push`.
struct ListElem {
    link: OwnedEntry<ListElem>,
    _payload: [u64; 4],
}

hal::owned_adapter!(
    /// `TAILQ_HEAD(, ListElem)`, in insertion order (every key equal: a push is a tail insert).
    ByList: ListElem, link, |_, _| core::cmp::Ordering::Equal
);

/// The pool of `list.push`'s elements.
static LIST_ELEMS: BoxPool<ListElem> = BoxPool::new();

/// An element of the `slist.*` benches (an mbuf's link and the rest of its header).
struct SlElem {
    next: SlistLink<SlElem>,
    _payload: [u64; 4],
}

hal::slist_adapter!(
    /// `m_next`'s chain.
    SlChain: SlElem, next
);

/// The pool of the `slist.*` benches' elements.
static SL_ELEMS: BoxPool<SlElem> = BoxPool::new();

impl hal::pool::Pooled for SlElem {
    const POOL: &'static BoxPool<Self> = &SL_ELEMS;
}

/// A fresh `slist.*` element.
fn slelem() -> PoolBox<SlElem> {
    need(
        SL_ELEMS.get(PR_WAITOK, || SlElem {
            next: SlistLink::new(),
            _payload: [0; 4],
        }),
        "no element",
    )
}

/// An object `smr.load` reads.
struct SmrObj {
    link: SmrLink,
    value: u64,
}

/// The pool of `smr.load`'s object.
static SMR_OBJS: BoxPool<SmrObj> = BoxPool::new();

impl SmrLinked for SmrObj {
    const POOL: &'static BoxPool<SmrObj> = &SMR_OBJS;
    fn smr_link(&self) -> &SmrLink {
        &self.link
    }
}

/// The writers' lock of `smr.load`'s pointer.
static BENCH_LOCK: RawRwlock = RawRwlock::new("benchsmr");

/// The pointer `smr.load` reads.
static PUBLISHED: SmrPtr<SmrObj> = SmrPtr::new_static(&BENCH_LOCK);

/// The deferred function of `smr.defer`'s links.
fn noop(_arg: *mut core::ffi::c_void) {}

/// The bench's grace periods: no CPU but the bench's own.
struct NoOtherCpu;

impl CpuWalk for NoOtherCpu {
    fn running(&self, _n: usize) -> bool {
        false
    }
    fn peg(&self, _n: usize) {}
    fn unpeg(&self) {}
}

fn need<T>(v: Option<T>, what: &str) -> T {
    v.unwrap_or_else(|| {
        eprintln!("net: {what}");
        std::process::exit(1)
    })
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let _ = hal::bridges::register_pool_backend(&BENCH_POOLS);
    testutil::install();

    let names = [
        "netlock.write",
        "netlock.read",
        "netlock.paired",
        "smr.read",
        "smr.load",
        "smr.defer",
        "pool.get_put",
        "pool.get_put.sourced",
        "pod.copy",
        "pod.view",
        "slist.push_tail",
        "slist.concat",
        "list.push",
    ];
    let names: Vec<&str> = names
        .iter()
        .chain(MBUF_ROWS.iter())
        .chain(SOCKET_ROWS.iter())
        .copied()
        .collect();
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    if MBUFS.init(0, PR_WAITOK, "mbufpl").is_err()
        || LIST_ELEMS.init(0, PR_WAITOK, "benchlst").is_err()
        || SL_ELEMS.init(0, PR_WAITOK, "benchsl").is_err()
        || SMR_OBJS.init(0, PR_WAITOK, "benchsmr").is_err()
        || SOURCED
            .init_with_source(0, PR_WAITOK, "mclpl", &BENCH_SOURCE)
            .is_err()
    {
        eprintln!("net: a pool could not be made");
        std::process::exit(1);
    }

    println!("# bench min median max ns/op, spread (max-min)/median, reps {REPS}");
    let report = |name: &str, (min, med, max): (f64, f64, f64)| {
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}%",
            name,
            min,
            med,
            max,
            (max - min) / med * 100.0
        );
    };

    if wanted(names[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(NETLOCK.write()));
            }
            t.elapsed()
        });
        report(names[0], r);
    }
    if wanted(names[1]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(NETLOCK.read()));
            }
            t.elapsed()
        });
        report(names[1], r);
    }
    if wanted(names[2]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                netlock::lock();
                netlock::unlock();
            }
            t.elapsed()
        });
        report(names[2], r);
    }
    if wanted(names[3]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(smr::read()));
            }
            t.elapsed()
        });
        report(names[3], r);
    }
    if wanted(names[4]) {
        let mut w = BENCH_LOCK.write();
        let old = PUBLISHED.store(
            &mut w,
            Some(need(
                SMR_OBJS.get(PR_WAITOK, || SmrObj {
                    link: SmrLink::new(),
                    value: 7,
                }),
                "no object",
            )),
        );
        drop(w);
        drop(old);
        let r = measure(OPS, || {
            let g = smr::read();
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(PUBLISHED.load(black_box(&g)).map(|o| o.value));
            }
            let d = t.elapsed();
            drop(g);
            d
        });
        report(names[4], r);
    }
    if wanted(names[5]) {
        let links: &'static [SmrLink] = Box::leak(
            (0..ELEMS)
                .map(|_| SmrLink::new())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let rounds = OPS / ELEMS;
        let r = measure(rounds * ELEMS, || {
            let mut d = Duration::ZERO;
            for _ in 0..rounds {
                let t = Instant::now();
                for l in links {
                    smr::defer_link(black_box(l), noop, std::ptr::null_mut());
                }
                d += t.elapsed();
                // Dispatched, collected, run and made idle again, untimed.
                smr::idle();
                let _ = smr::collect().run(&NoOtherCpu);
                for l in links {
                    l.init();
                }
            }
            d
        });
        report(names[5], r);
    }
    if wanted(names[6]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let b = need(MBUFS.get(PR_WAITOK, || [0; MSIZE]), "no mbuf");
                drop(black_box(b));
            }
            t.elapsed()
        });
        report(names[6], r);
    }
    if wanted(names[7]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let b = need(SOURCED.get(PR_WAITOK, || [0; MSIZE]), "no sourced item");
                drop(black_box(b));
            }
            t.elapsed()
        });
        report(names[7], r);
    }
    if wanted(names[8]) {
        let mut pkt = vec![0u8; 128];
        let r = measure(OPS, || {
            let t = Instant::now();
            for i in 0..OPS {
                let mut ip: Ip = need(hal::pod::at(black_box(&pkt), IP_OFF), "short packet");
                ip.ip_ttl = ip.ip_ttl.wrapping_sub(1);
                ip.ip_id = i as u16;
                if !hal::pod::put(black_box(&mut pkt), IP_OFF, &ip) {
                    eprintln!("net: short packet");
                    std::process::exit(1);
                }
            }
            t.elapsed()
        });
        report(names[8], r);
    }
    if wanted(names[9]) {
        let mut pkt = vec![0u8; 128];
        let r = measure(OPS, || {
            let t = Instant::now();
            for i in 0..OPS {
                let ip: &mut Ip = need(
                    hal::pod::view_mut(black_box(&mut pkt), IP_OFF),
                    "short or misaligned packet",
                );
                ip.ip_ttl = ip.ip_ttl.wrapping_sub(1);
                ip.ip_id = i as u16;
            }
            t.elapsed()
        });
        report(names[9], r);
    }
    if wanted(names[10]) {
        let mut boxes: Vec<PoolBox<SlElem>> = (0..ELEMS).map(|_| slelem()).collect();
        let mut list = OwnedSlist::<SlChain>::new();
        let rounds = OPS / ELEMS;
        let r = measure(rounds * ELEMS, || {
            let mut d = Duration::ZERO;
            for _ in 0..rounds {
                let t = Instant::now();
                for b in boxes.drain(..) {
                    list.push_tail(black_box(b));
                }
                d += t.elapsed();
                while let Some(b) = list.pop_head() {
                    boxes.push(b);
                }
            }
            d
        });
        report(names[10], r);
    }
    if wanted(names[11]) {
        let mut lists: Vec<OwnedSlist<SlChain>> = (0..ELEMS)
            .map(|_| {
                let mut l = OwnedSlist::new();
                l.push_tail(slelem());
                l.push_tail(slelem());
                l
            })
            .collect();
        let mut big = OwnedSlist::<SlChain>::new();
        let rounds = OPS / ELEMS;
        let r = measure(rounds * ELEMS, || {
            let mut d = Duration::ZERO;
            for _ in 0..rounds {
                let t = Instant::now();
                for l in &mut lists {
                    big.concat(black_box(l));
                }
                d += t.elapsed();
                for l in &mut lists {
                    for _ in 0..2 {
                        l.push_tail(need(big.pop_head(), "an element"));
                    }
                }
            }
            d
        });
        report(names[11], r);
    }
    if wanted(names[12]) {
        let mut boxes: Vec<PoolBox<ListElem>> = (0..ELEMS)
            .map(|_| {
                need(
                    LIST_ELEMS.get(PR_WAITOK, || ListElem {
                        link: OwnedEntry::new(),
                        _payload: [0; 4],
                    }),
                    "no element",
                )
            })
            .collect();
        let mut list = OwnedList::<ByList>::new();
        let rounds = OPS / ELEMS;
        let r = measure(rounds * ELEMS, || {
            let mut d = Duration::ZERO;
            for _ in 0..rounds {
                let t = Instant::now();
                for b in boxes.drain(..) {
                    list.push(black_box(b));
                }
                d += t.elapsed();
                while let Some(b) = list.pop() {
                    boxes.push(b);
                }
            }
            d
        });
        report(names[12], r);
    }
    mbuf_rows(&wanted, &report);
    socket_rows(&wanted, &report);
}

/// The mbuf's rows (N6.1, decision 43), the same ops as the before rows on LZ's shape.
const MBUF_ROWS: [&str; 13] = [
    "mbuf.get.free",
    "mbuf.gethdr.cluster.free",
    "mbuf.copym.share",
    "mbuf.copydata.1500",
    "mbuf.adj",
    "mbuf.mtod",
    "mbuf.header.view",
    "mbuf.header.copy",
    "mbuf.pullup.40",
    "mbuf.tag.find",
    "mbuf.data",
    "mbuf.copym.share.deep",
    "mbuf.adj.restore",
];

/// The bench's `nmbclust`.
static BENCH_NMBCLUST: core::sync::atomic::AtomicI64 =
    core::sync::atomic::AtomicI64::new(256 * 1024);

/// A 1500-byte packet in one cluster, its data `off` bytes in.
fn cluster_packet(off: usize) -> MbufBox {
    let mut m = need(m_gethdr(M_DONTWAIT, MT_DATA), "no mbuf");
    if !mclgetl(&mut m, M_DONTWAIT, 1500 + off as u32) {
        eprintln!("net: no cluster");
        std::process::exit(1);
    }
    m.set_data_off(off);
    m.m_len().set(1500);
    m.m_pkthdr().len.set(1500);
    if let Some(d) = m.data_mut() {
        for (i, b) in d.iter_mut().enumerate() {
            *b = (i * 7 + 3) as u8;
        }
    }
    m
}

fn mbuf_rows(wanted: &dyn Fn(&str) -> bool, report: &dyn Fn(&str, (f64, f64, f64))) {
    if !MBUF_ROWS.iter().any(|n| wanted(n)) {
        return;
    }
    // The pools' pages are the kernel's (MbufPages over km_alloc): the page system and the
    // kernel maps of the tests, held for the run.
    let _g = testutil::pages_install(64 << 20);
    testutil::kernel_maps_install();
    mbinit(&BENCH_NMBCLUST);

    if wanted(MBUF_ROWS[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let m = need(m_get(M_DONTWAIT, MT_DATA), "no mbuf");
                drop(black_box(m));
            }
            t.elapsed()
        });
        report(MBUF_ROWS[0], r);
    }
    if wanted(MBUF_ROWS[1]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let mut m = need(m_gethdr(M_DONTWAIT, MT_DATA), "no mbuf");
                mclget(&mut m, M_DONTWAIT);
                drop(black_box(m));
            }
            t.elapsed()
        });
        report(MBUF_ROWS[1], r);
    }
    let mut p = cluster_packet(0);
    if wanted(MBUF_ROWS[2]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let c = need(
                    m_copym(black_box(&mut p), 0, M_COPYALL, M_DONTWAIT),
                    "no copy",
                );
                drop(c);
            }
            t.elapsed()
        });
        report(MBUF_ROWS[2], r);
    }
    if wanted(MBUF_ROWS[3]) {
        let mut buf = vec![0u8; 1500];
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                m_copydata(black_box(&p), 0, &mut buf);
                black_box(&buf);
            }
            t.elapsed()
        });
        report(MBUF_ROWS[3], r);
    }
    if wanted(MBUF_ROWS[4]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                m_adj(black_box(&p), 14);
                p.set_data_off(p.data_off() - 14);
                p.m_len().set(p.m_len().get() + 14);
                p.m_pkthdr().len.set(p.m_pkthdr().len.get() + 14);
            }
            t.elapsed()
        });
        report(MBUF_ROWS[4], r);
    }
    if wanted(MBUF_ROWS[5]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(black_box(&p).data_raw());
            }
            t.elapsed()
        });
        report(MBUF_ROWS[5], r);
    }
    // ETHER_ALIGN: the IP header 16 bytes into the cluster, aligned for the view.
    let h = cluster_packet(2);
    if wanted(MBUF_ROWS[6]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let ip = need(hal::pod::view::<Ip>(black_box(&h).data(), 14), "misaligned");
                black_box(ip.ip_ttl);
            }
            t.elapsed()
        });
        report(MBUF_ROWS[6], r);
    }
    if wanted(MBUF_ROWS[7]) {
        let mut hdr = [0u8; 20];
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                m_copydata(black_box(&h), 14, &mut hdr);
                black_box(&hdr);
            }
            t.elapsed()
        });
        report(MBUF_ROWS[7], r);
    }
    if wanted(MBUF_ROWS[8]) {
        // A 14-byte header mbuf and a 1486-byte cluster mbuf: m_pullup(40) copies 26 bytes
        // into the first; the offsets put back by hand.
        let mut hd = need(m_gethdr(M_DONTWAIT, MT_DATA), "no mbuf");
        hd.m_len().set(14);
        let mut c = need(m_get(M_DONTWAIT, MT_DATA), "no mbuf");
        mclget(&mut c, M_DONTWAIT);
        c.m_len().set(1486);
        drop(hd.m_next.set(c));
        hd.m_pkthdr().len.set(1500);
        let mut hd = Some(hd);
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let m = need(m_pullup(need(hd.take(), "the packet"), 40), "pulled up");
                m.m_len().set(14);
                if let Some(c) = m.m_next.get() {
                    c.set_data_off(c.data_off() - 26);
                    c.m_len().set(1486);
                }
                hd = Some(black_box(m));
            }
            t.elapsed()
        });
        report(MBUF_ROWS[8], r);
    }
    if wanted(MBUF_ROWS[9]) {
        let mut tp = need(m_gethdr(M_DONTWAIT, MT_DATA), "no mbuf");
        m_tag_prepend(
            &mut tp,
            need(m_tag_get(PACKET_TAG_GRE, 4, M_DONTWAIT), "no tag"),
        );
        m_tag_prepend(
            &mut tp,
            need(m_tag_get(PACKET_TAG_DLT, 4, M_DONTWAIT), "no tag"),
        );
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(m_tag_find(black_box(&tp), PACKET_TAG_GRE, None));
            }
            t.elapsed()
        });
        report(MBUF_ROWS[9], r);
    }
    if wanted(MBUF_ROWS[10]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(black_box(&p).data());
            }
            t.elapsed()
        });
        report(MBUF_ROWS[10], r);
    }
    if wanted(MBUF_ROWS[11]) {
        // Three 500-byte cluster mbufs; 100 bytes copied from offset 1100, in the third: the
        // walk to the offset is what tcp_output's per-segment copy pays.
        let mut deep = cluster_chain(&[500, 500, 500]);
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let c = need(
                    m_copym(black_box(&mut deep), 1100, 100, M_DONTWAIT),
                    "no copy",
                );
                drop(c);
            }
            t.elapsed()
        });
        report(MBUF_ROWS[11], r);
    }
    if wanted(MBUF_ROWS[12]) {
        // `mbuf.adj`'s restore alone: what that row adds to `m_adj` itself.
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let q = black_box(&p);
                q.set_data_off(q.data_off());
                q.m_len().set(q.m_len().get());
                q.m_pkthdr().len.set(q.m_pkthdr().len.get());
            }
            t.elapsed()
        });
        report(MBUF_ROWS[12], r);
    }
}

/// A packet of cluster mbufs of `lens` bytes each, the first with the header (the before
/// rows' `packet` on LZ's shape).
fn cluster_chain(lens: &[u32]) -> MbufBox {
    let mut chain: Vec<MbufBox> = Vec::new();
    for (i, &len) in lens.iter().enumerate() {
        let m = if i == 0 {
            m_gethdr(M_DONTWAIT, MT_DATA)
        } else {
            m_get(M_DONTWAIT, MT_DATA)
        };
        let mut m = need(m, "no mbuf");
        if !mclgetl(&mut m, M_DONTWAIT, len) {
            eprintln!("net: no cluster");
            std::process::exit(1);
        }
        m.m_len().set(len);
        chain.push(m);
    }
    let mut next: Option<MbufBox> = None;
    while let Some(mut m) = chain.pop() {
        if let Some(n) = next.take() {
            drop(m.m_next.set(n));
        }
        next = Some(m);
    }
    let h = need(next, "an mbuf");
    h.m_pkthdr().len.set(lens.iter().sum::<u32>() as i32);
    h
}

/// The socket layer's rows (N6.2a).
const SOCKET_ROWS: [&str; 8] = [
    "sockbuf.append.drop",
    "soreceive.4k",
    "soreceive.peek.4k",
    "unix.socketpair.send.recv.64",
    "solock.unlock.unix",
    "socket.create.close.unix",
    "soreceive.peek.1",
    "sockbuf.record.out.in",
];

/// The glue of the socket rows: the doubles' process and file glues.
type G = kernel::fs::testfs::TestGlue;

/// The socket rows' domains: the UNIX domain alone.
static DOMS: [&dyn Domain<G>; 1] = [&Unix::<G>::NEW];

/// A kernel-space uio over `buf` (`UIO_SYSSPACE`).
fn kuio(iov: &mut [Iovec; 1], rw: UioRw) -> Uio<'_, G> {
    Uio {
        uio_resid: iov[0].iov_len,
        uio_iov: iov,
        uio_offset: 0,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: rw,
        uio_procp: None,
    }
}

/// `sosend` of `buf` on `so`.
fn ksend(so: &kernel::net::socket::Socket<G>, buf: &[u8]) {
    let mut iov = [Iovec {
        iov_base: buf.as_ptr().cast_mut().cast(),
        iov_len: buf.len(),
    }];
    let mut u = kuio(&mut iov, UioRw::UIO_WRITE);
    if kernel::net::socket::sosend(so, None, Some(&mut u), None, None, 0).is_err()
        || u.uio_resid != 0
    {
        eprintln!("net: sosend");
        std::process::exit(1);
    }
}

/// `soreceive` of `buf.len()` bytes from `so`.
fn krecv(so: &kernel::net::socket::Socket<G>, buf: &mut [u8]) {
    krecv_flags(so, buf, 0);
}

/// `soreceive` of `buf.len()` bytes from `so` with `flags`.
fn krecv_flags(so: &kernel::net::socket::Socket<G>, buf: &mut [u8], flags: i32) {
    let mut iov = [Iovec {
        iov_base: buf.as_mut_ptr().cast(),
        iov_len: buf.len(),
    }];
    let mut u = kuio(&mut iov, UioRw::UIO_READ);
    let mut flags = flags;
    if kernel::net::socket::soreceive(so, None, &mut u, None, None, Some(&mut flags), 0).is_err()
        || u.uio_resid != 0
    {
        eprintln!("net: soreceive");
        std::process::exit(1);
    }
}

fn socket_rows(wanted: &dyn Fn(&str) -> bool, report: &dyn Fn(&str, (f64, f64, f64))) {
    use hal::sys::socket::{AF_UNIX, SOCK_STREAM};
    use kernel::net::socket::{soclose, soconnect2, socreate, solock};
    if !SOCKET_ROWS.iter().any(|n| wanted(n)) {
        return;
    }
    // The doubles' thread over the page system and the kernel maps of the tests, the
    // mbufs, the socket pools and the UNIX domain, held for the run.
    let (_g, p) = kernel::fs::testfs::test_thread_cached(64 << 20, 64);
    testutil::kernel_maps_install();
    mbinit(&BENCH_NMBCLUST);
    kernel::net::domain::soinit::<G>();
    kernel::net::domain::domaininit::<G>(&DOMS);
    kernel::net::unix::unp_init::<G>();
    // The data really copied to and from the rows' kernel buffers (hal's kcopy, as bsd's
    // uiomove copies a UIO_SYSSPACE iovec in the "before" rows), not only accounted.
    testutil::set_copy_kernel(true);
    let sock = || {
        need(
            socreate(p, i32::from(AF_UNIX), SOCK_STREAM, 0).ok(),
            "socreate",
        )
    };

    if wanted(SOCKET_ROWS[0]) {
        let sb = kernel::net::sockbuf::Sockbuf::<G>::new();
        if sb.lock().sbreserve(65536).is_err() {
            eprintln!("net: sbreserve");
            std::process::exit(1);
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let m = need(m_get(M_DONTWAIT, MT_DATA), "no mbuf");
                m.m_len().set(100);
                sb.lock().sbappendstream(black_box(m));
                sb.lock().sbdrop(100);
            }
            t.elapsed()
        });
        report(SOCKET_ROWS[0], r);
    }
    let (a, b) = (sock(), sock());
    if soconnect2(&a, &b).is_err() {
        eprintln!("net: soconnect2");
        std::process::exit(1);
    }
    if wanted(SOCKET_ROWS[1]) {
        let out = vec![7u8; 4096];
        let mut inb = vec![0u8; 4096];
        let n = OPS / 16;
        let r = measure(n, || {
            let mut d = Duration::ZERO;
            for _ in 0..n {
                ksend(&a, &out);
                let t = Instant::now();
                krecv(&b, black_box(&mut inb));
                d += t.elapsed();
            }
            d
        });
        report(SOCKET_ROWS[1], r);
    }
    if wanted(SOCKET_ROWS[2]) {
        let out = vec![7u8; 4096];
        // The destination 8-aligned (u64 words), as the "before" row's.
        let mut words = vec![0u64; 512];
        let inb = hal::user::bytes_of_mut(&mut words[..]);
        ksend(&a, &out);
        {
            let g = b.so_rcv.lock();
            let first = g.data().first().map(|m| m.data().as_ptr().addr() % 64);
            println!(
                "# the 4 KiB record: {} mbufs, data at {:?} mod 64, destination at {} mod 64",
                g.data().mbufs().count(),
                first,
                inb.as_ptr().addr() % 64
            );
        }
        let r = measure(OPS / 16, || {
            let t = Instant::now();
            for _ in 0..OPS / 16 {
                krecv_flags(&b, black_box(&mut *inb), hal::sys::socket::MSG_PEEK);
            }
            t.elapsed()
        });
        krecv(&b, &mut *inb);
        report(SOCKET_ROWS[2], r);
    }
    if wanted(SOCKET_ROWS[6]) || wanted(SOCKET_ROWS[7]) {
        let out = vec![7u8; 4096];
        ksend(&a, &out);
        println!(
            "# the 4 KiB record: {} mbufs",
            b.so_rcv.lock().data().mbufs().count()
        );
        if wanted(SOCKET_ROWS[6]) {
            let mut one = [0u8; 1];
            let r = measure(OPS / 16, || {
                let t = Instant::now();
                for _ in 0..OPS / 16 {
                    krecv_flags(&b, black_box(&mut one), hal::sys::socket::MSG_PEEK);
                }
                t.elapsed()
            });
            report(SOCKET_ROWS[6], r);
        }
        if wanted(SOCKET_ROWS[7]) {
            let r = measure(OPS, || {
                let t = Instant::now();
                for _ in 0..OPS {
                    let mut g = b.so_rcv.lock();
                    let rec = need(g.take_first_record(), "a record");
                    g.put_back_record(black_box(rec));
                }
                t.elapsed()
            });
            report(SOCKET_ROWS[7], r);
        }
        let mut inb = vec![0u8; 4096];
        krecv(&b, &mut inb);
    }
    if wanted(SOCKET_ROWS[3]) {
        let out = [5u8; 64];
        let mut inb = [0u8; 64];
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                ksend(&a, black_box(&out));
                krecv(&b, black_box(&mut inb));
            }
            t.elapsed()
        });
        report(SOCKET_ROWS[3], r);
    }
    if wanted(SOCKET_ROWS[4]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(solock(&a)));
            }
            t.elapsed()
        });
        report(SOCKET_ROWS[4], r);
    }
    let _ = soclose(&a, 0);
    let _ = soclose(&b, 0);
    if wanted(SOCKET_ROWS[5]) {
        let n = OPS / 16;
        let r = measure(n, || {
            let t = Instant::now();
            for _ in 0..n {
                let so = sock();
                let _ = soclose(black_box(&so), 0);
            }
            t.elapsed()
        });
        report(SOCKET_ROWS[5], r);
    }
}
/* </CODE> */
