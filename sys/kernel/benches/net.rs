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
//! `doubles`), a recycling pool backend (vm.rs's: the doubles' pool hands out a fresh page
//! per item) and, since N6.3b2, a recycling `malloc(9)` by size class (the doubles' takes
//! fresh zeroed pages per block: the routing rows' addresses would time the host's
//! allocator). `sys/benches/net.rs` (N6.0's first commit to its last) timed the net lock's and
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
//! - The interface layer's rows (N6.3a, decision 43 with U10), over an interface attached by
//!   `if_attach` (a `kernel::net::if_::test_ifnet_fresh`: listed, its index, its queues) whose
//!   driver hands every packet it is given back to the rows (a thread-local stash: no
//!   packet is allocated or freed inside an op), `IFXF_CLONED` set (no `enqueue_randomness`,
//!   whose host doubles differ between the shapes), `IFF_RUNNING` and `if_txmit` 1 (the start
//!   at once, not deferred to a softnet task no thread serves); every run prints where the
//!   interface and the packets lie (`# placement`):
//!   - `if_get.put`: `if_get` of the interface's index and `if_put` (the SMR read section,
//!     the map's two loads, the reference taken and released).
//!   - `if_get_smr`: an SMR read section opened, `if_get_smr`, closed.
//!   - `ifq.enq.deq`: one packet `ifq_enqueue`d on the send queue (priq) and `ifq_dequeue`d.
//!   - `ifiq.input.8`: `ifiq_input` of a list of 8 packets (the input queue's mutex, the
//!     counters, the list joined, the softnet task added), the list taken back under the
//!     mutex (`ifiq_test_take`) into the stash; an op is the 8 packets.
//!   - `if_input_process.8`: `if_input_process` of 8 packets: the shared net lock, the
//!     driver's input per packet, the netstack's lists walked empty.
//!   - `ifq.serialize`: `ifq_serialize` of a task that does nothing on an idle queue (the
//!     serializer's mutex twice, the task linked, unlinked and run).
//!   - `if_counters.pkt`: `counters_pkt` of a 60-byte packet on the interface's counters.
//!   - `ifa_ifwithaddr.8x2`: `ifa_ifwithaddr` of the last of 16 addresses (8 listed
//!     interfaces of 2 `AF_INET` addresses each), under the net lock's write witness.
//!   - `if_enqueue_ifq.mq`: `if_enqueue_ifq` on an interface of 4 send queues, the flow id
//!     picking the third, whose start the driver runs and empties at once.
//! - The routing rows (N6.3b2, decision 43 with U10), over a table of four IPv4 routes on one
//!   listed interface (`10.0.2.15/24`): the connected `10.0.2.0/24`, `10.1.0.0/24` and
//!   `10.2.0.0/24` through `10.0.2.2`, and the default route through `10.0.2.2` (the "before"
//!   rows build the same table with LZ's functions); every lookup under the read witness of the
//!   net lock where the C holds it:
//!   - `rtable.match.4`: `rtable_match` of `10.2.0.77` (no source), the route's handle dropped
//!     (`rtfree`).
//!   - `rtable.lookup`: `rtable_lookup` of `10.1.0.0/24` exactly (`RTP_ANY`), its handle dropped.
//!   - `rtalloc.cached`: `Route::mpath4` of `10.2.0.77` on a cache that holds its route
//!     (`route_mpath`'s hit: `rtisvalid`, the generation, the destination compared).
//!   - `rt_hash.mpath`: `rt_hash` of `10.2.0.0/24` (made `RTF_MPATH` for the row) with a source.
//!   - `rtrequest.add.delete`: `rtrequest(RTM_ADD)` of `10.9.0.0/24` through `10.0.2.2` and
//!     `rtrequest_delete` of it, under the write witness, beside a `10.9.1.0/24` that stays (so
//!     a delete frees the node and not the ART's tables, which LZ's side would leave to a gc
//!     task no thread runs there); the releases the deletes defer run every 64 ops, untimed
//!     (the grace periods of a CPU alone, as `smr.defer`'s rounds).
//!   - `route.ref.unref` and `route.isvalid`, the attribution rows of the five: a counted
//!     handle taken from a lent route and dropped (`PoolArc::try_from_ref`, the C's `rtref`
//!     and `rtfree`), and `rtisvalid` of the gateway route `10.2.0.0/24` (its next hop read
//!     through `rt_gwroute`'s cell).
//! - bpf(4)'s rows (N6.3c1, decision 73), the same ops as the "before" rows on LZ's shape (a
//!   scratch `#[ignore]` test of `bsd` at the base): two interfaces attached to bpf on
//!   Ethernet, one with no listener, one with a descriptor opened by `bpfopen` on the doubles'
//!   thread, a 2 MiB buffer and its interface set; a 64-byte packet in one mbuf:
//!   - `bpf.mtap.none`: a driver's tap of the interface no descriptor listens on (the word's
//!     test, nothing tapped).
//!   - `bpf.mtap.1`: the tap of the listened interface, the read filter a one-instruction
//!     program that keeps the whole packet: the record captured into the store buffer; the
//!     buffer flushed between rounds (`BIOCFLUSH`), untimed, so no packet is dropped.
//!   - `bpf.mtap.1.reject`: the same tap, the filter a program that rejects.
//!   - `bpf.filter.dns`: `_bpf_lfilter` of `tcpdump -d 'ip and udp and dst port 53'` over a
//!     42-byte Ethernet, IPv4 and UDP frame to port 53 in one mbuf (the accepting path,
//!     ten instructions).
//!   - `bpf.validate.512`: `bpf_validate` of a 512-instruction program (loads, ALU, forward
//!     jumps, stores, a return last).

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

/// The smallest and the largest size class of [`BenchMalloc`] (`malloc(9)`'s buckets: 16
/// bytes to 64 KiB, by powers of two).
const MALLOC_MIN: usize = 16;
/// See [`MALLOC_MIN`].
const MALLOC_CLASSES: usize = 13;

/// A `malloc(9)` double that recycles its blocks, by size class (`kmembuckets`): the
/// doubles' `malloc` takes fresh zeroed pages for every block and never reuses one, which an
/// op that allocates (the routing rows' addresses) would time instead of the code.
/// Registered before the doubles; blocks above the largest class are fresh host memory.
struct BenchMalloc {
    classes: [FreeList; MALLOC_CLASSES],
}

/// The size class of a block of `size` bytes, and its size.
fn malloc_class(size: usize) -> Option<(usize, usize)> {
    let sz = size.max(MALLOC_MIN).next_power_of_two();
    let i = (sz.trailing_zeros() - MALLOC_MIN.trailing_zeros()) as usize;
    (i < MALLOC_CLASSES).then_some((i, sz))
}

impl hal::malloc::Malloc for BenchMalloc {
    fn malloc(&self, size: usize, _ty: i32, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        match malloc_class(size) {
            Some((i, sz)) => Some((self.classes[i].get()?, sz)),
            None => Some((NonNull::from(fresh(size)).cast::<u8>(), size)),
        }
    }

    fn free(&self, ptr: NonNull<u8>, _ty: i32, size: usize) {
        if let Some((i, _)) = malloc_class(size) {
            self.classes[i].put(ptr);
        }
    }
}

/// The one [`BenchMalloc`].
static BENCH_MALLOC: BenchMalloc = BenchMalloc {
    classes: [
        FreeList::new(16, None),
        FreeList::new(32, None),
        FreeList::new(64, None),
        FreeList::new(128, None),
        FreeList::new(256, None),
        FreeList::new(512, None),
        FreeList::new(1024, None),
        FreeList::new(2048, None),
        FreeList::new(4096, None),
        FreeList::new(8192, None),
        FreeList::new(16384, None),
        FreeList::new(32768, None),
        FreeList::new(65536, None),
    ],
};

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
    let _ = hal::bridges::register_malloc(&BENCH_MALLOC);
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
        .chain(IF_ROWS.iter())
        .chain(ROUTE_ROWS.iter())
        .chain(BPF_ROWS.iter())
        .chain(ETHER_ROWS.iter())
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
    if_rows(&wanted, &report);
    route_rows(&wanted, &report);
    bpf_rows(&wanted, &report);
    ether_rows(&wanted, &report);
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
/// The interface layer's rows (N6.3a, decision 43 with U10), the same ops as the "before"
/// rows on LZ's shape (a scratch `#[ignore]` test of `bsd` at the base, over `setup_net`'s
/// zeroed `test_ifnet`).
const IF_ROWS: [&str; 9] = [
    "if_get.put",
    "if_get_smr",
    "ifq.enq.deq",
    "ifiq.input.8",
    "if_input_process.8",
    "ifq.serialize",
    "if_counters.pkt",
    "ifa_ifwithaddr.8x2",
    "if_enqueue_ifq.mq",
];

std::thread_local! {
    /// The packets [`BenchDriver`] was given, handed out again by the rows: no packet is
    /// freed or allocated inside a timed op.
    static STASH: std::cell::RefCell<Vec<MbufBox>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// A packet back into the stash.
fn stash(m: MbufBox) {
    STASH.with(|s| s.borrow_mut().push(m));
}

/// A packet out of the stash.
fn unstash() -> MbufBox {
    need(STASH.with(|s| s.borrow_mut().pop()), "an empty stash")
}

/// The rows' driver: its start takes every packet off the queue it is given and its input
/// takes the packet, both into the stash (a driver's ring and a protocol, at no cost of
/// their own beyond a thread-local push).
struct BenchDriver;

impl kernel::net::if_var::NetDriver for BenchDriver {
    fn ioctl(
        &self,
        _ifp: &'static kernel::net::if_var::Ifnet,
        _cmd: u64,
        _data: &mut [u8],
    ) -> Result<(), hal::sys::errno::Errno> {
        Ok(())
    }

    fn qstart(&self, ifq: &'static kernel::net::ifq::Ifqueue) {
        while let Some(m) = kernel::net::ifq::ifq_dequeue(ifq) {
            stash(m);
        }
    }

    fn input(
        &self,
        _ifp: &'static kernel::net::if_var::Ifnet,
        m: MbufBox,
        _ns: Option<&mut kernel::net::if_var::Netstack>,
    ) {
        stash(m);
    }
}

static BENCH_DRIVER: BenchDriver = BenchDriver;

/// `ifq_serialize`'s work for `ifq.serialize`: nothing.
fn serial_noop(_t: &'static (dyn core::any::Any + Send + Sync)) {}

/// The target of `ifq.serialize`'s task.
static SERIAL_TARGET: () = ();

/// Where `what` lies: its address, and its offset in its page (the rows' placement, printed
/// with every run, decision items 23-24).
fn placement<T>(what: &str, p: &T) {
    let a = core::ptr::from_ref(p).addr();
    println!("# placement {what} {a:#x} page+{:#x}", a % PAGE);
}

fn if_rows(wanted: &dyn Fn(&str) -> bool, report: &dyn Fn(&str, (f64, f64, f64))) {
    use hal::netlock::NetHeld;
    use hal::sys::net_if::{IFF_RUNNING, IFXF_CLONED};
    use kernel::net::if_::{
        if_attach, if_attach_queues, if_counters_alloc, if_enqueue_ifq, if_get, if_get_smr,
        if_input_process, if_put, if_test_reset, ifa_add, ifa_del, ifa_ifwithaddr,
        test_ifnet_fresh, test_packet,
    };
    use kernel::net::if_var::{IfCounters, Ifaddr, counters_pkt};
    use kernel::net::ifq::{ifiq_input, ifiq_test_take, ifq_dequeue, ifq_enqueue, ifq_serialize};
    use kernel::net::mbuf::{M_FLOWID, MbufList, ml_dequeue, ml_enqueue};
    use kernel::task::Task;

    if !IF_ROWS.iter().any(|n| wanted(n)) {
        return;
    }
    // The pools' pages are the kernel's: the page system and the kernel maps of the tests,
    // held for the run; the mbufs, a fresh index map, no interface listed.
    let _g = testutil::pages_install(64 << 20);
    testutil::kernel_maps_install();
    mbinit(&BENCH_NMBCLUST);
    if_test_reset();

    // Listed, as the "before" rows' (`if_attach`): the address walk passes it.
    let ifp = test_ifnet_fresh(b"bnch0", &BENCH_DRIVER);
    if_attach(ifp);
    // No `enqueue_randomness` in `if_input_process` (the host doubles of the two shapes'
    // entropy pools differ; the "before" rows set it too).
    ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_CLONED);
    ifp.if_flags.set(ifp.if_flags.get() | IFF_RUNNING);
    // The start runs at once (no transmit mitigation to the softnet task, which no thread
    // serves here).
    ifp.if_txmit.set(1);
    let idx = ifp.if_index.get();
    placement("ifnet", ifp);
    for _ in 0..64 {
        stash(test_packet(3, 60));
    }

    if wanted(IF_ROWS[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                if_put(black_box(if_get(black_box(idx))));
            }
            t.elapsed()
        });
        report(IF_ROWS[0], r);
    }
    if wanted(IF_ROWS[1]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let g = smr::read();
                black_box(if_get_smr(black_box(idx), &g));
                drop(g);
            }
            t.elapsed()
        });
        report(IF_ROWS[1], r);
    }
    if wanted(IF_ROWS[2]) {
        let ifq = ifp.if_snd();
        let mut m = Some(unstash());
        if let Some(p) = &m {
            placement("ifq.enq.deq mbuf", &**p);
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let p = need(m.take(), "the packet");
                if ifq_enqueue(ifq, black_box(p)).is_err() {
                    eprintln!("net: ifq_enqueue refused the packet");
                    std::process::exit(1);
                }
                m = ifq_dequeue(ifq);
            }
            t.elapsed()
        });
        stash(need(m, "the packet"));
        report(IF_ROWS[2], r);
    }
    if wanted(IF_ROWS[3]) {
        let ifiq = ifp.ifiq(0);
        let n = OPS / 8;
        let r = measure(n, || {
            let t = Instant::now();
            for _ in 0..n {
                let mut ml = MbufList::new();
                for _ in 0..8 {
                    ml_enqueue(&mut ml, unstash());
                }
                black_box(ifiq_input(ifiq, &mut ml));
                let mut back = ifiq_test_take(ifiq);
                while let Some(m) = ml_dequeue(&mut back) {
                    stash(m);
                }
            }
            t.elapsed()
        });
        report(IF_ROWS[3], r);
    }
    if wanted(IF_ROWS[4]) {
        let n = OPS / 8;
        let r = measure(n, || {
            let t = Instant::now();
            for _ in 0..n {
                let mut ml = MbufList::new();
                for _ in 0..8 {
                    ml_enqueue(&mut ml, unstash());
                }
                if_input_process(ifp, &mut ml, 0);
            }
            t.elapsed()
        });
        report(IF_ROWS[4], r);
    }
    if wanted(IF_ROWS[5]) {
        let ifq = ifp.if_snd();
        let task: &'static Task = Box::leak(Box::new(Task::unset()));
        task.set_ref(serial_noop, &SERIAL_TARGET);
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                ifq_serialize(ifq, core::pin::Pin::static_ref(black_box(task)));
            }
            t.elapsed()
        });
        report(IF_ROWS[5], r);
    }
    if wanted(IF_ROWS[6]) {
        if_counters_alloc(ifp);
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                if let Some(c) = black_box(ifp).if_counters.get() {
                    counters_pkt(c, IfCounters::IfcIpackets, IfCounters::IfcIbytes, 60);
                }
            }
            t.elapsed()
        });
        report(IF_ROWS[6], r);
    }
    if wanted(IF_ROWS[7]) {
        // Eight listed interfaces of two addresses each, 10.i.0.1 and 172.16.i.1; the
        // address looked for is the last one walked (the C's two nested TAILQ walks whole).
        let token = need(hal::link::SaToken::take(), "the SaToken");
        let sin = |a: [u8; 4]| -> &'static core::cell::Cell<hal::sys::socket::Sockaddr> {
            let mut sa: hal::sys::socket::Sockaddr = hal::user::zeroed();
            sa.sa_len = 16;
            sa.sa_family = hal::sys::socket::AF_INET;
            sa.sa_data[2..6].copy_from_slice(&a);
            Box::leak(Box::new(core::cell::Cell::new(sa)))
        };
        let mut addrs: Vec<(&'static kernel::net::if_var::Ifnet, &'static Ifaddr)> = Vec::new();
        for i in 0..8u8 {
            let lifp = test_ifnet_fresh(b"bnch1", &BENCH_DRIVER);
            if_attach(lifp);
            for a in [[10, i, 0, 1], [172, 16, i, 1]] {
                let ifa: &'static Ifaddr = Box::leak(Box::new(Ifaddr::new()));
                ifa.ifa_addr.set(token, Some(sin(a)));
                ifa.ifa_ifp.set(Some(lifp));
                ifa_add(lifp, ifa);
                addrs.push((lifp, ifa));
            }
        }
        let want = sin([172, 16, 7, 1]).get();
        let want = hal::user::bytes_of(core::slice::from_ref(&want)).to_vec();
        let net = NETLOCK.write();
        let held = need(NetHeld::of(&net), "the net lock's witness");
        if let Some((_, last)) = addrs.last() {
            placement("ifa_ifwithaddr last address", *last);
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(ifa_ifwithaddr(&held, black_box(&want), 0));
            }
            t.elapsed()
        });
        if ifa_ifwithaddr(&held, &want, 0)
            .is_none_or(|f| addrs.last().is_none_or(|l| !core::ptr::eq(f, l.1)))
        {
            eprintln!("net: ifa_ifwithaddr found another address");
            std::process::exit(1);
        }
        for (lifp, ifa) in &addrs {
            ifa_del(lifp, ifa);
        }
        drop(net);
        report(IF_ROWS[7], r);
    }
    if wanted(IF_ROWS[8]) {
        // Four send queues; the packet's flow id picks the third (priq_idx), whose start the
        // driver runs at once and empties.
        let mq = test_ifnet_fresh(b"bnch2", &BENCH_DRIVER);
        if_attach(mq);
        mq.if_flags.set(mq.if_flags.get() | IFF_RUNNING);
        mq.if_txmit.set(1);
        if_attach_queues(mq, 4);
        placement("if_enqueue_ifq ifnet", mq);
        let m = unstash();
        m.m_pkthdr().ph_flowid.set(2);
        m.m_pkthdr()
            .csum_flags
            .set(m.m_pkthdr().csum_flags.get() | M_FLOWID);
        stash(m);
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                if if_enqueue_ifq(mq, unstash()).is_err() {
                    eprintln!("net: if_enqueue_ifq refused the packet");
                    std::process::exit(1);
                }
            }
            t.elapsed()
        });
        if !kernel::net::ifq::ifq_empty(mq.ifq(2)) {
            eprintln!("net: if_enqueue_ifq left the packet queued");
            std::process::exit(1);
        }
        report(IF_ROWS[8], r);
    }
    // The packets back to their pool while its pages are still installed.
    STASH.with(|s| s.borrow_mut().clear());
}

/// The routing rows (N6.3b2, decision 43 with U10), the same ops as the "before" rows on
/// LZ's shape (a scratch `#[ignore]` test of `bsd` at the base, over the same table).
const ROUTE_ROWS: [&str; 7] = [
    "rtable.match.4",
    "rtable.lookup",
    "rtalloc.cached",
    "rt_hash.mpath",
    "rtrequest.add.delete",
    "route.ref.unref",
    "route.isvalid",
];

/// The internet protocols of the routing rows: IPv6 multipath on and its route cache's hits
/// and misses counted (the C's `ip6stat_inc(ip6s_rtcachehit)`); IPv4's switch and counters
/// are the kernel's own since N6.4a (`kernel::netinet::ip_var`'s `IPMULTIPATH` and
/// `IPCOUNTERS`).
struct BenchInet {
    /// `ip6s_rtcachehit` and `ip6s_rtcachemiss`.
    rtcache: [AtomicU64; 2],
}

impl kernel::net::if_::Inet for BenchInet {
    /// `arpresolve`'s multicast arm (`ether.output.inet`): a group's mapped address, no
    /// route, the packet back (bsd's leg adds its park and unpark around bsd's arpresolve).
    fn arpresolve(
        &self,
        _ifp: &'static kernel::net::if_var::Ifnet,
        _rt: Option<&kernel::net::route::Rtentry>,
        m: MbufBox,
        dst: kernel::net::proto::SockaddrView<'_>,
        desten: &mut [u8; 6],
    ) -> Result<MbufBox, hal::sys::errno::Errno> {
        let Some(sin) = hal::pod::at::<hal::sys::netinet_in::SockaddrIn>(dst.bytes(), 0) else {
            return Err(hal::sys::errno::Errno::EINVAL);
        };
        *desten = kernel::net::if_ethersubr::ether_map_ip_multicast(&sin.sin_addr);
        Ok(m)
    }
    fn arpintr(&self) {}
    fn ipintr(&self) {}
    fn ip6intr(&self) {}
    fn tcp_input_mlist(
        &self,
        _ml: &mut kernel::net::mbuf::MbufList,
        _af: hal::sys::types::SaFamily,
    ) {
    }
    fn ipv4_input(&self) -> kernel::net::if_::ProtoFn {
        kernel::net::if_::test_proto_count
    }
    fn ipv6_input(&self) -> kernel::net::if_::ProtoFn {
        kernel::net::if_::test_proto_count
    }
    fn tcpstat_lro(&self, _hw: bool) {}
    fn nd6_ifattach(&self, _ifp: &'static kernel::net::if_var::Ifnet) {}
    fn nd6_ifdetach(&self, _ifp: &'static kernel::net::if_var::Ifnet) {}
    fn rti_delete(&self, _ifp: &'static kernel::net::if_var::Ifnet) {}
    fn revarp_ifdetach(&self, _ifp: &'static kernel::net::if_var::Ifnet) {}
    fn in_ifdetach(&self, _ifp: &'static kernel::net::if_var::Ifnet) {}
    fn in6_ifdetach(&self, _ifp: &'static kernel::net::if_var::Ifnet) {}
    fn in6_ifattach(
        &self,
        _ifp: &'static kernel::net::if_var::Ifnet,
    ) -> Result<(), hal::sys::errno::Errno> {
        Ok(())
    }
    fn in_ioctl(
        &self,
        _cmd: u64,
        _data: &mut [u8],
        _ifp: &'static kernel::net::if_var::Ifnet,
        _privileged: bool,
    ) -> Result<(), hal::sys::errno::Errno> {
        Err(hal::sys::errno::Errno::EOPNOTSUPP)
    }
    fn ip6_forwarding(&self) -> bool {
        false
    }
    fn in6_purge_linklocal(&self, _ifp: &'static kernel::net::if_var::Ifnet) -> bool {
        false
    }
    fn ip6_multipath(&self) -> bool {
        true
    }
    fn ip6stat_rtcache(&self, hit: bool) {
        self.rtcache[usize::from(!hit)].fetch_add(1, Ordering::Relaxed);
    }
}

/// The one [`BenchInet`].
static BENCH_INET: BenchInet = BenchInet {
    rtcache: [AtomicU64::new(0), AtomicU64::new(0)],
};

/// A `sockaddr_in` of `a`.
fn sin(a: [u8; 4]) -> [u8; 16] {
    let mut s = [0u8; 16];
    s[0] = 16;
    s[1] = hal::sys::socket::AF_INET;
    s[4..8].copy_from_slice(&a);
    s
}

/// The view of a socket address the rows built.
fn sav(b: &[u8]) -> kernel::net::proto::SockaddrView<'_> {
    need(kernel::net::proto::SockaddrView::new(b), "a socket address")
}

/// The routing rows' requests: `dst`/`mask` through `gw` on `ifa` (`route add`'s).
fn route_info<'a>(
    dst: &'a [u8],
    mask: &'a [u8],
    gw: &'a [u8],
    ifa: &'static kernel::net::if_var::Ifaddr,
) -> kernel::net::route::RtAddrinfo<'a> {
    use hal::sys::net_route::{
        RTAX_DST, RTAX_GATEWAY, RTAX_NETMASK, RTF_GATEWAY, RTF_STATIC, RTF_UP,
    };
    let mut info = kernel::net::route::RtAddrinfo::new();
    info.rti_info[RTAX_DST] = Some(sav(dst));
    info.rti_info[RTAX_NETMASK] = Some(sav(mask));
    info.rti_info[RTAX_GATEWAY] = Some(sav(gw));
    info.rti_ifa = Some(ifa);
    info.rti_flags = RTF_GATEWAY | RTF_STATIC | RTF_UP;
    info
}

fn route_rows(wanted: &dyn Fn(&str) -> bool, report: &dyn Fn(&str, (f64, f64, f64))) {
    use hal::netlock::NetHeld;
    use hal::sys::net_route::{RTF_CONNECTED, RTF_MPATH, RTM_ADD, RTP_ANY, RTP_STATIC};
    use kernel::net::if_::{if_alloc_sadl, if_attach, if_test_reset, ifa_add, test_ifnet_fresh};
    use kernel::net::if_var::Ifaddr;
    use kernel::net::route::{
        Route, route_init, rt_hash, rt_ifa_add, rtlabel_test_reset, rtrequest, rtrequest_delete,
    };
    use kernel::net::rtable::{
        RtDom, rtable_init_doms, rtable_lookup, rtable_match, rtable_test_reset,
    };

    if !ROUTE_ROWS.iter().any(|n| wanted(n)) {
        return;
    }
    // The C's `net.inet.ip.multipath` set, which `rt_hash` reads.
    kernel::netinet::ip_var::IPMULTIPATH.store(1, Ordering::Relaxed);
    if kernel::net::if_::register_inet(&BENCH_INET).is_err() {
        eprintln!("net: the internet protocols were registered already");
        std::process::exit(1);
    }
    let _g = testutil::pages_install(64 << 20);
    testutil::kernel_maps_install();
    if_test_reset();
    rtable_test_reset();
    rtlabel_test_reset();
    route_init();
    rtable_init_doms(&[
        RtDom {
            family: hal::sys::socket::AF_INET,
            rtoffset: 4,
            maxplen: 32,
            sasize: 16,
        },
        RtDom {
            family: hal::sys::socket::AF_INET6,
            rtoffset: 8,
            maxplen: 128,
            sasize: 28,
        },
    ]);

    // One listed interface with 10.0.2.15/24, its link-level address (the connected route's
    // gateway).
    let ifp = test_ifnet_fresh(b"rtb0", &BENCH_DRIVER);
    ifp.if_flags
        .set(ifp.if_flags.get() | hal::sys::net_if::IFF_UP | hal::sys::net_if::IFF_BROADCAST);
    if_alloc_sadl(ifp);
    if_attach(ifp);
    let token = need(hal::link::SaToken::take(), "the SaToken");
    let cell = |a: [u8; 4]| -> &'static core::cell::Cell<hal::sys::socket::Sockaddr> {
        let mut sa: hal::sys::socket::Sockaddr = hal::user::zeroed();
        sa.sa_len = 16;
        sa.sa_family = hal::sys::socket::AF_INET;
        sa.sa_data[2..6].copy_from_slice(&a);
        Box::leak(Box::new(core::cell::Cell::new(sa)))
    };
    let ifa: &'static Ifaddr = Box::leak(Box::new(Ifaddr::new()));
    ifa.ifa_refcnt.init();
    ifa.ifa_addr.set(token, Some(cell([10, 0, 2, 15])));
    ifa.ifa_netmask.set(token, Some(cell([255, 255, 255, 0])));
    ifa.ifa_ifp.set(Some(ifp));
    ifa_add(ifp, ifa);

    let mask24 = sin([255, 255, 255, 0]);
    let mask0 = sin([0; 4]);
    let gw = sin([10, 0, 2, 2]);
    let net1 = sin([10, 1, 0, 0]);
    let net2 = sin([10, 2, 0, 0]);
    let net9 = sin([10, 9, 0, 0]);
    let any = sin([0; 4]);
    let host = sin([10, 2, 0, 77]);
    let host_addr = [10, 2, 0, 77];
    {
        let w = NETLOCK.write();
        let net = need(NetHeld::of(&w), "the net lock's witness");
        if rt_ifa_add(ifa, RTF_CONNECTED, sav(&sin([10, 0, 2, 0])), 0, &net).is_err() {
            eprintln!("net: the connected route");
            std::process::exit(1);
        }
        for dst in [&net1, &net2] {
            let mut info = route_info(dst, &mask24, &gw, ifa);
            if rtrequest(RTM_ADD, &mut info, 0, None, 0, &net).is_err() {
                eprintln!("net: a route through 10.0.2.2");
                std::process::exit(1);
            }
        }
        let mut info = route_info(&any, &mask0, &gw, ifa);
        if rtrequest(RTM_ADD, &mut info, 0, None, 0, &net).is_err() {
            eprintln!("net: the default route");
            std::process::exit(1);
        }
    }

    let r_lock = NETLOCK.read();
    let held = need(NetHeld::of(&r_lock), "the net lock's read witness");
    let found = need(rtable_match(0, sav(&host), None), "the route of 10.2.0.77");
    if found.rt_plen() != 24 || !found.dest_view().bytes().starts_with(&net2[..8]) {
        eprintln!("net: 10.2.0.77 matched another route");
        std::process::exit(1);
    }
    placement("rtentry 10.2.0.0/24", &*found);
    if wanted(ROUTE_ROWS[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(rtable_match(0, black_box(sav(&host)), None)));
            }
            t.elapsed()
        });
        report(ROUTE_ROWS[0], r);
    }
    if wanted(ROUTE_ROWS[1]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(rtable_lookup(
                    0,
                    black_box(sav(&net1)),
                    Some(sav(&mask24)),
                    None,
                    RTP_ANY,
                )));
            }
            t.elapsed()
        });
        report(ROUTE_ROWS[1], r);
    }
    if wanted(ROUTE_ROWS[2]) {
        let mut ro = Route::new();
        if ro.mpath4(host_addr, None, 0, &held).is_none() {
            eprintln!("net: no route to 10.2.0.77");
            std::process::exit(1);
        }
        let hit = kernel::netinet::ip_var::IpstatCounters::IpsRtcachehit as usize;
        let hits = kernel::netinet::ip_var::IPCOUNTERS.read()[hit];
        if ro.mpath4(host_addr, None, 0, &held).is_none()
            || kernel::netinet::ip_var::IPCOUNTERS.read()[hit] != hits + 1
        {
            eprintln!("net: the cache missed 10.2.0.77");
            std::process::exit(1);
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(ro.mpath4(black_box(host_addr), None, 0, &held));
            }
            t.elapsed()
        });
        report(ROUTE_ROWS[2], r);
    }
    if wanted(ROUTE_ROWS[3]) {
        found.rt_flags.set(found.rt_flags.get() | RTF_MPATH);
        let src = [u32::from_ne_bytes([10, 0, 2, 15])];
        if rt_hash(&found, sav(&host), Some(&src)).is_none() {
            eprintln!("net: rt_hash of a multipath route answered none");
            std::process::exit(1);
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(rt_hash(
                    &found,
                    black_box(sav(&host)),
                    Some(black_box(&src)),
                ));
            }
            t.elapsed()
        });
        found.rt_flags.set(found.rt_flags.get() & !RTF_MPATH);
        report(ROUTE_ROWS[3], r);
    }
    if wanted(ROUTE_ROWS[5]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                drop(black_box(hal::pool::PoolArc::try_from_ref(black_box(
                    &*found,
                ))));
            }
            t.elapsed()
        });
        report(ROUTE_ROWS[5], r);
    }
    if wanted(ROUTE_ROWS[6]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(kernel::net::route::rtisvalid(Some(black_box(&*found))));
            }
            t.elapsed()
        });
        report(ROUTE_ROWS[6], r);
    }
    drop(held);
    drop(r_lock);
    if wanted(ROUTE_ROWS[4]) {
        // A sibling that stays: a delete frees the node, never the ART's tables.
        {
            let net91 = sin([10, 9, 1, 0]);
            let w = NETLOCK.write();
            let net = need(NetHeld::of(&w), "the net lock's witness");
            let mut info = route_info(&net91, &mask24, &gw, ifa);
            if rtrequest(RTM_ADD, &mut info, 0, None, 0, &net).is_err() {
                eprintln!("net: the sibling 10.9.1.0/24");
                std::process::exit(1);
            }
        }
        // Fewer ops: each one allocates and frees a route, its addresses and its node.
        const ADD_OPS: usize = 64 * 160;
        let r = measure(ADD_OPS, || {
            let mut spent = Duration::ZERO;
            let mut done = 0;
            while done < ADD_OPS {
                let w = NETLOCK.write();
                let net = need(NetHeld::of(&w), "the net lock's witness");
                let t = Instant::now();
                for _ in 0..64 {
                    let mut info = route_info(&net9, &mask24, &gw, ifa);
                    if rtrequest(RTM_ADD, &mut info, 0, None, 0, &net).is_err() {
                        eprintln!("net: RTM_ADD of 10.9.0.0/24");
                        std::process::exit(1);
                    }
                    let mut del = route_info(&net9, &mask24, &gw, ifa);
                    del.rti_info[hal::sys::net_route::RTAX_GATEWAY] = None;
                    if rtrequest_delete(&mut del, RTP_STATIC, ifp, None, 0, &net).is_err() {
                        eprintln!("net: the delete of 10.9.0.0/24");
                        std::process::exit(1);
                    }
                }
                spent += t.elapsed();
                drop(net);
                drop(w);
                done += 64;
                // The deferred releases, untimed.
                loop {
                    smr::idle();
                    if smr::collect().run(&NoOtherCpu) == 0 {
                        break;
                    }
                }
            }
            spent
        });
        report(ROUTE_ROWS[4], r);
    }
    drop(found);
}

/// bpf(4)'s rows (N6.3c1, decision 73).
const BPF_ROWS: [&str; 5] = [
    "bpf.mtap.none",
    "bpf.mtap.1",
    "bpf.mtap.1.reject",
    "bpf.filter.dns",
    "bpf.validate.512",
];

/// A batch of `bpf.mtap.1`'s captures that a 2 MiB buffer holds (96-byte records).
const BPF_CAPS: usize = 16_384;

/// `tcpdump -d 'ip and udp and dst port 53'` on Ethernet.
const BPF_DNS: [hal::sys::net_bpf::BpfInsn; 11] = {
    use hal::sys::net_bpf::{
        BPF_ABS, BPF_B, BPF_H, BPF_IND, BPF_JEQ, BPF_JMP, BPF_JSET, BPF_K, BPF_LD, BPF_LDX,
        BPF_MSH, BPF_RET, bpf_jump, bpf_stmt,
    };
    [
        bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 12),
        bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 0x800, 0, 8),
        bpf_stmt(BPF_LD | BPF_B | BPF_ABS, 23),
        bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 17, 0, 6),
        bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 20),
        bpf_jump(BPF_JMP | BPF_JSET | BPF_K, 0x1fff, 4, 0),
        bpf_stmt(BPF_LDX | BPF_B | BPF_MSH, 14),
        bpf_stmt(BPF_LD | BPF_H | BPF_IND, 16),
        bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 53, 0, 1),
        bpf_stmt(BPF_RET | BPF_K, 262_144),
        bpf_stmt(BPF_RET | BPF_K, 0),
    ]
};

/// A packet holding `bytes` in one mbuf.
fn bpf_packet(bytes: &[u8]) -> MbufBox {
    let mut m = need(m_gethdr(M_DONTWAIT, MT_DATA), "no mbuf");
    m.m_len().set(bytes.len() as u32);
    m.m_pkthdr().len.set(bytes.len() as i32);
    need(m.data_mut(), "a writable mbuf").copy_from_slice(bytes);
    m
}

fn bpf_rows(wanted: &dyn Fn(&str) -> bool, report: &dyn Fn(&str, (f64, f64, f64))) {
    use hal::sys::net_bpf::{
        BIOCFLUSH, BIOCGSTATS, BIOCSBLEN, BIOCSETF, BIOCSETIF, BPF_A, BPF_ABS, BPF_ADD, BPF_ALU,
        BPF_DIRECTION_IN, BPF_H, BPF_JEQ, BPF_JMP, BPF_K, BPF_LD, BPF_MAXBUFSIZE, BPF_RET, BPF_ST,
        BpfInsn, BpfProgram, BpfStat, DLT_EN10MB, bpf_jump, bpf_stmt,
    };
    use hal::sys::netinet_if_ether::ETHER_HDR_LEN;
    use hal::sys::types::makedev;
    use hal::user::UserPtr;
    use kernel::fs::spec::CLONE_SHIFT;
    use kernel::net::bpf::{BpfPkt, bpf_mtap, bpfattach, bpfclose, bpfioctl, bpfopen};
    use kernel::net::bpf_filter::{_bpf_lfilter, bpf_validate};
    use kernel::net::if_::{if_attach, if_test_reset, test_ifnet_fresh};

    if !BPF_ROWS.iter().any(|n| wanted(n)) {
        return;
    }
    // The doubles' thread (bpfopen's), the mbufs, a fresh index map; the kernel lock, which
    // the cdev entry points and the attach run under, held for the run.
    let (_g, p) = kernel::fs::testfs::test_thread_cached(64 << 20, 64);
    testutil::kernel_maps_install();
    mbinit(&BENCH_NMBCLUST);
    if_test_reset();
    hal::klock::kernel_lock();

    let quiet = test_ifnet_fresh(b"bnchq0", &BENCH_DRIVER);
    if_attach(quiet);
    bpfattach(&quiet.if_bpf, quiet, DLT_EN10MB, ETHER_HDR_LEN as u32);
    let frame: Vec<u8> = (0u8..64).collect();
    let m = bpf_packet(&frame);
    placement("ifnet", quiet);
    placement("mbuf", &*m);
    // A driver's tap site: the word tested, the tap called when it is set.
    let tap = |ifp: &'static kernel::net::if_var::Ifnet| {
        let w = &black_box(ifp).if_bpf;
        if w.is_set() {
            black_box(bpf_mtap(w, &m, BPF_DIRECTION_IN));
        }
    };

    if wanted(BPF_ROWS[0]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                tap(quiet);
            }
            t.elapsed()
        });
        report(BPF_ROWS[0], r);
    }

    // One listener: a 2 MiB buffer, the interface set, a program at a user address.
    let ifp = test_ifnet_fresh(b"bnch1", &BENCH_DRIVER);
    if_attach(ifp);
    bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
    let dev = makedev(23, 5 << CLONE_SHIFT);
    need(bpfopen::<G>(dev, 0, 0, p).ok(), "bpfopen");
    let ioctl = |cmd: u64, data: &mut [u8]| need(bpfioctl::<G>(dev, cmd, data, 0, p).ok(), "ioctl");
    let mut size = (BPF_MAXBUFSIZE as u32).to_ne_bytes();
    ioctl(BIOCSBLEN, &mut size);
    let mut ifr = [0u8; 32];
    ifr[..5].copy_from_slice(b"bnch1");
    ioctl(BIOCSETIF, &mut ifr);
    let uprog = testutil::USER_BASE + 0x2_8000;
    let setf = |insn: BpfInsn| {
        need(
            UserPtr::<BpfInsn>::new(uprog).copyout_slice(&[insn]).ok(),
            "the program in user memory",
        );
        let prog = BpfProgram {
            bf_len: 1,
            _pad0: 0,
            bf_insns: uprog as u64,
        };
        let mut b = hal::user::bytes_of(core::slice::from_ref(&prog)).to_vec();
        ioctl(BIOCSETF, &mut b);
    };
    setf(bpf_stmt(BPF_RET | BPF_K, u32::MAX));

    if wanted(BPF_ROWS[1]) {
        let r = measure(BPF_CAPS, || {
            ioctl(BIOCFLUSH, &mut []);
            let t = Instant::now();
            for _ in 0..BPF_CAPS {
                tap(ifp);
            }
            t.elapsed()
        });
        report(BPF_ROWS[1], r);
        let mut st = [0u8; core::mem::size_of::<BpfStat>()];
        ioctl(BIOCGSTATS, &mut st);
        let st: BpfStat = need(hal::pod::at(&st, 0), "the statistics");
        if st.bs_drop != 0 {
            eprintln!("net: bpf.mtap.1 dropped {} packets", st.bs_drop);
            std::process::exit(1);
        }
    }
    setf(bpf_stmt(BPF_RET | BPF_K, 0));
    if wanted(BPF_ROWS[2]) {
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                tap(ifp);
            }
            t.elapsed()
        });
        report(BPF_ROWS[2], r);
    }
    need(bpfclose::<G>(dev, 0, 0, None).ok(), "bpfclose");

    if wanted(BPF_ROWS[3]) {
        let mut dns = vec![0u8; 42];
        dns[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        dns[14] = 0x45;
        dns[23] = 17;
        dns[36..38].copy_from_slice(&53u16.to_be_bytes());
        let dm = bpf_packet(&dns);
        let len = dns.len() as u32;
        if _bpf_lfilter(Some(&BPF_DNS), &BpfPkt::mbuf(&dm), len) != 262_144 {
            eprintln!("net: the DNS program rejects its frame");
            std::process::exit(1);
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                black_box(_bpf_lfilter(
                    Some(black_box(&BPF_DNS[..])),
                    &BpfPkt::mbuf(&dm),
                    len,
                ));
            }
            t.elapsed()
        });
        report(BPF_ROWS[3], r);
    }

    if wanted(BPF_ROWS[4]) {
        let mut p512: Vec<BpfInsn> = (0..511)
            .map(|i| match i % 4 {
                0 => bpf_stmt(BPF_LD | BPF_H | BPF_ABS, 12),
                1 => bpf_stmt(BPF_ALU | BPF_ADD | BPF_K, 1),
                2 => bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 7, 0, 0),
                _ => bpf_stmt(BPF_ST, 3),
            })
            .collect();
        p512.push(bpf_stmt(BPF_RET | BPF_A, 0));
        let maxbuf = BPF_MAXBUFSIZE as u32;
        if !bpf_validate(&p512, maxbuf) {
            eprintln!("net: the 512-instruction program is refused");
            std::process::exit(1);
        }
        let vops = OPS / 10;
        let r = measure(vops, || {
            let t = Instant::now();
            for _ in 0..vops {
                black_box(bpf_validate(black_box(&p512), maxbuf));
            }
            t.elapsed()
        });
        report(BPF_ROWS[4], r);
    }
}
/// The link layers' rows (N6.3c2, decision 43): Ethernet's input and output and the
/// loopback's round trip, each through the interface's driver as the stack reaches it.
const ETHER_ROWS: [&str; 5] = [
    "ether.input",
    "ether.output.hdrcmplt",
    "ether.output.unspec",
    "lo.output.input",
    "ether.output.inet",
];

/// The Ethernet rows' driver: its output, input and rtrequest are the defaults (the link
/// layer's); its start leaves the packets queued, for the rows to take.
struct EtherBenchDriver;

impl kernel::net::if_var::NetDriver for EtherBenchDriver {
    fn ioctl(
        &self,
        _ifp: &'static kernel::net::if_var::Ifnet,
        _cmd: u64,
        _data: &mut [u8],
    ) -> Result<(), hal::sys::errno::Errno> {
        Ok(())
    }

    fn qstart(&self, _ifq: &'static kernel::net::ifq::Ifqueue) {}
}

static ETHER_BENCH_DRIVER: EtherBenchDriver = EtherBenchDriver;

/// A packet of `len` bytes starting `lead` bytes into its mbuf (room for a link header).
fn lead_packet(lead: usize, len: usize) -> MbufBox {
    let m = need(m_gethdr(M_DONTWAIT, MT_DATA), "no mbuf");
    m.set_data_off(lead);
    m.m_len().set(len as u32);
    m.m_pkthdr().len.set(len as i32);
    m
}

/// `m_data -= n` with the lengths, the inverse of `m_adj(m, n)` (a received frame's header
/// shown again for the next op).
fn unadj(m: &kernel::net::mbuf::Mbuf, n: usize) {
    m.set_data_off(m.data_off() - n);
    m.m_len().set(m.m_len().get() + n as u32);
    m.m_pkthdr().len.set(m.m_pkthdr().len.get() + n as i32);
}

fn ether_rows(wanted: &dyn Fn(&str) -> bool, report: &dyn Fn(&str, (f64, f64, f64))) {
    use hal::sys::net_ethertypes::ETHERTYPE_IP;
    use hal::sys::net_if::{
        IFF_BROADCAST, IFF_MULTICAST, IFF_RUNNING, IFF_SIMPLEX, IFNAMSIZ, IFXF_CLONED, IFXF_MPSAFE,
    };
    use hal::sys::socket::{AF_UNSPEC, pseudo_AF_HDRCMPLT};
    use kernel::net::if_::{if_attach, if_test_reset};
    use kernel::net::if_ethersubr::{Arpcom, ether_ifattach};
    use kernel::net::if_loop::{LOOP_CLONER, loop_clone_create};
    use kernel::net::if_var::Netstack;
    use kernel::net::ifq::{ifiq_test_take, ifq_dequeue};
    use kernel::net::mbuf::ml_dequeue;

    const OURS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    const PEER: [u8; 6] = [0x52, 0x54, 0x00, 0xaa, 0xbb, 0xcc];

    if !ETHER_ROWS.iter().any(|n| wanted(n)) {
        return;
    }
    // The route rows may have registered it first; the same double either way.
    let _ = kernel::net::if_::register_inet(&BENCH_INET);
    let _g = testutil::pages_install(64 << 20);
    testutil::kernel_maps_install();
    mbinit(&BENCH_NMBCLUST);
    if_test_reset();

    let ac: &'static Arpcom = Box::leak(Box::new(Arpcom::new()));
    let ifp: &'static kernel::net::if_var::Ifnet = &ac.ac_if;
    let mut name = [0u8; IFNAMSIZ];
    name[..6].copy_from_slice(b"bnche0");
    ifp.if_xname.set(name);
    if ifp.if_driver.set(&ETHER_BENCH_DRIVER).is_err() {
        eprintln!("net: a fresh interface with a driver");
        std::process::exit(1);
    }
    ifp.if_xflags.set(IFXF_MPSAFE | IFXF_CLONED);
    ifp.if_flags
        .set(IFF_BROADCAST | IFF_SIMPLEX | IFF_MULTICAST | IFF_RUNNING);
    ac.ac_enaddr.set(OURS);
    hal::klock::kernel_lock();
    if_attach(ifp);
    ether_ifattach(ac);
    hal::klock::kernel_unlock();
    let d = need(ifp.if_driver.get().copied(), "the driver");
    placement("arpcom", ac);
    let mut ns = Netstack::new(0);

    if wanted(ETHER_ROWS[0]) {
        // An IPv4 frame to us, 60 bytes, to the protocol's queue of the softnet's stack.
        let mut m = Some(lead_packet(2, 60));
        if let Some(p) = m.as_mut() {
            let data = need(p.data_mut(), "a writable mbuf");
            data[..6].copy_from_slice(&OURS);
            data[6..12].copy_from_slice(&PEER);
            data[12..14].copy_from_slice(&ETHERTYPE_IP.to_be_bytes());
            data[14] = 0x45;
        }
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let p = need(m.take(), "the frame");
                d.input(ifp, black_box(p), Some(&mut ns));
                let q = need(ml_dequeue(&mut ns.ns_proto), "the queued frame");
                unadj(&q, 14);
                m = Some(q);
            }
            t.elapsed()
        });
        report(ETHER_ROWS[0], r);
    }

    for (row, family) in [
        (1, pseudo_AF_HDRCMPLT),
        (2, AF_UNSPEC),
        (4, hal::sys::socket::AF_INET),
    ] {
        if !wanted(ETHER_ROWS[row]) {
            continue;
        }
        // The destination's header in sa_data, or (AF_INET) a multicast group that
        // arpresolve maps with no route; a 46-byte packet with room for the header.
        let mut sa = [0u8; 16];
        sa[0] = 16;
        sa[1] = family;
        let mut m = Some(lead_packet(32, 46));
        if family == hal::sys::socket::AF_INET {
            sa[4..8].copy_from_slice(&[224, 0, 0, 1]);
            if let Some(p) = m.as_ref() {
                p.m_flags().set(p.flags() | kernel::net::mbuf::M_MCAST);
            }
        } else {
            sa[2..8].copy_from_slice(&PEER);
            sa[8..14].copy_from_slice(&OURS);
            sa[14..16].copy_from_slice(&ETHERTYPE_IP.to_be_bytes());
        }
        let dst = sav(&sa);
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let p = need(m.take(), "the packet");
                if d.output(ifp, black_box(p), dst, None).is_err() {
                    eprintln!("net: ether_output refused the packet");
                    std::process::exit(1);
                }
                let q = need(ifq_dequeue(ifp.if_snd()), "the queued frame");
                m_adj(&q, 14);
                m = Some(q);
            }
            t.elapsed()
        });
        report(ETHER_ROWS[row], r);
    }

    if wanted(ETHER_ROWS[3]) {
        hal::klock::kernel_lock();
        let made = loop_clone_create(&LOOP_CLONER, 0);
        hal::klock::kernel_unlock();
        if made.is_err() {
            eprintln!("net: no lo0");
            std::process::exit(1);
        }
        let lo = need(
            kernel::net::if_::IFNETLIST
                .iter()
                .find(|i| i.if_xname.get()[..4] == *b"lo0\0"),
            "lo0 listed",
        );
        let ld = need(lo.if_driver.get().copied(), "lo0's driver");
        let a = sin([127, 0, 0, 1]);
        let dst = sav(&a);
        let mut m = Some(lead_packet(32, 60));
        let r = measure(OPS, || {
            let t = Instant::now();
            for _ in 0..OPS {
                let p = need(m.take(), "the packet");
                if ld.output(lo, black_box(p), dst, None).is_err() {
                    eprintln!("net: looutput refused the packet");
                    std::process::exit(1);
                }
                let mut ml = ifiq_test_take(lo.if_rcv());
                let q = need(ml_dequeue(&mut ml), "the looped packet");
                ld.input(lo, q, Some(&mut ns));
                m = Some(need(ml_dequeue(&mut ns.ns_proto), "the protocol's packet"));
            }
            t.elapsed()
        });
        report(ETHER_ROWS[3], r);
    }
}
/* </CODE> */
