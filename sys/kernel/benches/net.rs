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
//! the pool items, the byte views of the protocol headers and the owned lists the network
//! slices (N6.1-N6.7) build on, over the kernel's host doubles (`kernel::testutil`, feature
//! `doubles`) and a recycling pool backend (vm.rs's: the doubles' pool hands out a fresh page
//! per item). `sys/benches/net.rs` timed the net lock's and `smr(9)`'s rows on LZ's shape by
//! the same names. Never part of a kernel image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench net` (release: the `bench` profile) or
//! with `--profile dev`. Arguments: `list`; the names of the benches to run (all by default).
//!
//! What an op is:
//! - `pool.get_put`: a `BoxPool` item of an mbuf's size (256 bytes) taken and dropped (put
//!   back), on the recycling backend.
//! - `pod.copy`: an IPv4 header (20 bytes, an `abi_pod!` image) copied out of a packet's
//!   bytes at offset 16, one field changed, written back: the C's `m_copydata` and
//!   `m_copyback` of a header.
//! - `list.push`: one element pushed at the tail of an `OwnedList` (`TAILQ_INSERT_TAIL`); the
//!   list drained between rounds, untimed.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::owned::{OwnedEntry, OwnedList};
use hal::pool::{BoxPool, PR_WAITOK, PoolBackend, PoolBox, RawPool};
use hal::user::{pod_from_bytes, pod_to_bytes};
use kernel::testutil;

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
}

impl FreeList {
    const fn new(stride: usize) -> Self {
        Self {
            stride,
            free: Mutex::new(Vec::new()),
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
            items: FreeList::new(stride),
            size,
            nout: AtomicU64::new(0),
        })))
    }
}

static BENCH_POOLS: BenchPools = BenchPools;

// ---------------------------------------------------------------------------------------
// The items.

/// An mbuf's size (`MSIZE`).
const MSIZE: usize = 256;

/// The pool of `pool.get_put`: items of an mbuf's size.
static MBUFS: BoxPool<[u8; MSIZE]> = BoxPool::new();

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

/// The `T` at byte `off` of `buf`, copied out (`kernel::fs::pod`'s `pod_at`).
fn pod_at<T: hal::user::AbiPod>(buf: &[u8], off: usize) -> Option<T> {
    pod_from_bytes(buf.get(off..)?)
}

/// `v` written at byte `off` of `buf` (`kernel::fs::pod`'s `pod_put`).
fn pod_put<T: hal::user::AbiPod>(buf: &mut [u8], off: usize, v: &T) -> bool {
    buf.get_mut(off..).is_some_and(|b| pod_to_bytes(v, b))
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

    let names = ["pool.get_put", "pod.copy", "list.push"];
    if args.first().map(String::as_str) == Some("list") {
        for n in names {
            println!("{n}");
        }
        return;
    }
    let wanted = |n: &str| args.is_empty() || args.iter().any(|a| a == n);

    if MBUFS.init(0, PR_WAITOK, "mbufpl").is_err()
        || LIST_ELEMS.init(0, PR_WAITOK, "benchlst").is_err()
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
                let b = need(MBUFS.get(PR_WAITOK, || [0; MSIZE]), "no mbuf");
                drop(black_box(b));
            }
            t.elapsed()
        });
        report(names[0], r);
    }
    if wanted(names[1]) {
        let mut pkt = vec![0u8; 128];
        let r = measure(OPS, || {
            let t = Instant::now();
            for i in 0..OPS {
                let mut ip: Ip = need(pod_at(black_box(&pkt), IP_OFF), "short packet");
                ip.ip_ttl = ip.ip_ttl.wrapping_sub(1);
                ip.ip_id = i as u16;
                if !pod_put(black_box(&mut pkt), IP_OFF, &ip) {
                    eprintln!("net: short packet");
                    std::process::exit(1);
                }
            }
            t.elapsed()
        });
        report(names[1], r);
    }
    if wanted(names[2]) {
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
        report(names[2], r);
    }
}
/* </CODE> */
