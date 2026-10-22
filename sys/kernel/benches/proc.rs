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
//! Host microbenchmarks of N4.2's hot paths (decision 43): the credentials' copy, hold and
//! refresh, a thread's uid and limit reads, a process's insertion into and removal from the
//! process list and its pid chain, over the kernel's host doubles (`kernel::testutil`, feature
//! `doubles`) and a recycling pool backend. Never part of a kernel image.
//!
//! `cargo bench -p kernel --features doubles --bench proc` (release) or with `--profile dev`.
//! Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's functions, a
//! scratch test of `bsd`'s kern_prot.rs at the parent commit):
//! - `prim.cred-copy`: `crdup(cr)` then its drop (LZ: `crdup` + `crfree`).
//! - `prim.cred-ref`: `crhold(p->p_ucred)` then its drop: `Thread::cred_ref` under `ps_mtx`
//!   (LZ: `crfree(crhold(cr))`, two atomics).
//! - `prim.cred-refresh`: the process's credentials swapped to the other of two, then the
//!   thread's `dorefreshcreds` (here: `set_ucred` + `refresh_cred`, the ids copied; LZ: the
//!   `ps_ucred` store + `dorefreshcreds`, a pointer swap under the kernel lock).
//! - `prim.getuid`: `p->p_ucred->cr_uid` (here: the thread's own copy).
//! - `prim.lim-read`: `lim_cur(RLIMIT_NOFILE)` of a thread whose copy is current.
//! - `proc.list-insert-remove`: a process put on `allprocess` and its pid chain and taken
//!   off both, under the kernel lock's writer (LZ: four `LIST_*` macros, no lock object).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::klock;
use hal::pool::{PoolBackend, RawPool};
use hal::sys::types::Pid;
use kernel::cred::{CredRef, Ucred, crdup};
use kernel::proc::{self, Pgrp, ProcGlue, ProcTables, Process, Session};
use kernel::resource::{self, PlimitRef, RLIMIT_NOFILE};
use kernel::testutil;

// ---------------------------------------------------------------------------------------
// The harness (vm.rs's).

/// One bench: its name, the ops one call of `f` makes, and `f`, which returns the time of
/// the part it measures (all of it, or a phase).
struct Bench<'a> {
    name: &'static str,
    ops: usize,
    f: Box<dyn FnMut() -> Duration + 'a>,
}

/// The calls of a primitive per timed call.
const PRIM: usize = 1000;

/// Times `PRIM` calls of `f`.
fn prim(mut f: impl FnMut()) -> impl FnMut() -> Duration {
    move || {
        let t = Instant::now();
        for _ in 0..PRIM {
            f();
        }
        t.elapsed()
    }
}

/// ns per op of `reps` repetitions of about `target` of wall time each (after a
/// calibration that is the warm-up): min, median, max, and the calls per repetition.
fn measure(b: &mut Bench<'_>, reps: usize, target: Duration) -> (f64, f64, f64, usize) {
    let mut calls = 1usize;
    loop {
        let t = Instant::now();
        for _ in 0..calls {
            black_box((b.f)());
        }
        let wall = t.elapsed();
        if wall >= target / 4 || calls >= 1 << 24 {
            let per = wall.as_secs_f64() / calls as f64;
            calls = ((target.as_secs_f64() / per.max(1e-9)) as usize).max(3);
            break;
        }
        calls *= 4;
    }
    let mut v = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut t = Duration::ZERO;
        for _ in 0..calls {
            t += (b.f)();
        }
        v.push(t.as_nanos() as f64 / (calls * b.ops) as f64);
    }
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() - 1], calls)
}

// ---------------------------------------------------------------------------------------
// A pool backend that recycles its items (vm.rs's), registered before the doubles: the
// doubles' pool hands out a fresh zeroed page per item and never takes one back, which
// would time the host's page allocator and page faults, not extent(9). The ops below that
// take pool items (extent.alloc-free's descriptors) get them from here.

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
// timeout(9).

// ---------------------------------------------------------------------------------------
// The processes' glue of this bench: no member of bsd's.

/// The bench's glue.
pub struct BenchGlue;

kernel::proc_glue_links!(BenchGlue);

static TABLES: ProcTables<BenchGlue> = ProcTables::new();

impl ProcGlue for BenchGlue {
    type ProcessG = ();
    type ThreadG = ();
    type PgrpG = ();
    type SessionG = ();
    type RunqEntry = ();
    kernel::proc_glue_link_types!();

    fn tables() -> &'static ProcTables<Self> {
        &TABLES
    }

    fn pgsignal(_pg: &Pgrp<Self>, _sig: i32, _checkctty: bool) {}
    fn prsignal(_pr: &Process<Self>, _sig: i32) {}
    fn pgrp_destroy(_pg: &Pgrp<Self>) {}
    fn session_hangup(_sess: &Session<Self>) {}
    fn session_placed(_sess: &Session<Self>) {}
    fn session_destroy(_sess: &Session<Self>) {}
}

fn ids(uid: u32) -> CredRef {
    let mut c = Ucred::new();
    c.cr_uid.set(uid);
    CredRef::new(c)
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let _ = hal::bridges::register_pool_backend(&BENCH_POOLS);
    testutil::install();
    klock::kernel_lock();
    proc::procinit::<BenchGlue>(1024, 1024);

    let pr = proc::process_alloc::<BenchGlue>(42, ());
    let a = ids(1000);
    let b = ids(1001);
    drop(pr.set_ucred(a.clone()));
    let lim = resource::lim_startup(7030, 1 << 30, 4096);
    drop(pr.set_limit(PlimitRef::new(lim)));
    let t = proc::thread_alloc::<BenchGlue>(&pr, (), ());
    drop(t.set_cred(pr.ucred()));
    let other = proc::process_alloc::<BenchGlue>(4242, ());
    let pid: Pid = 4242;

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "prim.cred-copy",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(crdup(&a))))),
    });
    benches.push(Bench {
        name: "prim.cred-ref",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(t.cred_ref())))),
    });
    let flip = std::cell::Cell::new(false);
    benches.push(Bench {
        name: "prim.cred-refresh",
        ops: PRIM,
        f: Box::new(prim(|| {
            flip.set(!flip.get());
            drop(pr.set_ucred(if flip.get() { b.clone() } else { a.clone() }));
            t.refresh_cred();
        })),
    });
    let t2 = &t;
    benches.push(Bench {
        name: "prim.getuid",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(t2.ucred().cr_uid.get());
        })),
    });
    benches.push(Bench {
        name: "prim.lim-read",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(t2.lim_cur(RLIMIT_NOFILE));
        })),
    });
    benches.push(Bench {
        name: "proc.list-insert-remove",
        ops: PRIM,
        f: Box::new(prim(|| {
            let tb = BenchGlue::tables();
            let mut w = klock::witness();
            tb.allprocess.insert_head(&mut w, other.as_pin());
            tb.pidhash(pid).insert_head(&mut w, other.as_pin());
            tb.pidhash(pid).remove(&mut w, &other);
            tb.allprocess.remove(&mut w, &other);
        })),
    });

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for bench in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|x| x == bench.name) {
            continue;
        }
        let (min, med, max, calls) = measure(bench, reps, target);
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            bench.name,
            min,
            med,
            max,
            (max - min) / med * 100.0,
            calls
        );
    }
    std::process::exit(0);
}
/* </CODE> */
