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
//! Host microbenchmarks of `futex(2)` (N4.6c, decision 43: libc's mutexes and condition
//! variables wait and wake through it), over the kernel's host doubles (`kernel::testutil`,
//! feature `doubles`). Never part of a kernel image.
//!
//! `cargo bench -p kernel --features doubles --bench futex` (release) or with `--profile
//! dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's
//! `futex_wait`/`futex_wake`, a scratch test of `bsd`'s sys_futex.rs at the parent commit).
//! The host has no context switch to time, so the round trip is the futex's own work on
//! both sides of a sleep:
//! - `futex.wait-eagain`: `FUTEX_WAIT_PRIVATE` on a word whose value changed: the waiter
//!   published on its bucket, the word read (`copyin32`), the waiter taken off again
//!   (`futex_unwait`), `EAGAIN`.
//! - `futex.wake-none`: `FUTEX_WAKE_PRIVATE` of one on a word nobody waits on: the key, the
//!   bucket's lock, the bucket found empty.
//! - `prim.futex-wait-wake`: the two above in a row (the brief's round trip).
//! - `futex.wake-shared-none`: `FUTEX_WAKE` (shared) of one on an unmapped word: the thread's
//!   address space, the map's read lock and lookup, then as `futex.wake-none`.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::arch::current::vmparam::{VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS};
use hal::sys::errno::Errno;
use hal::sys::futex::{FUTEX_WAIT_PRIVATE, FUTEX_WAKE, FUTEX_WAKE_PRIVATE};
use hal::user::UserPtr;
use kernel::proc::{self, Pgrp, ProcGlue, ProcTables, Process, Session, Thread};
use kernel::testutil;

// ---------------------------------------------------------------------------------------
// The harness (vm.rs's).

/// One bench: its name, the ops one call of `f` makes, and `f`, which returns the time of
/// the part it measures.
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
// The processes' glue of this bench: no member of bsd's.

/// The bench's glue.
pub struct BenchGlue;

kernel::proc_glue_links!(BenchGlue);
kernel::test_file_glue!(BenchFiles, BenchGlue);

static TABLES: ProcTables<BenchGlue> = ProcTables::new();
static SCHED: kernel::sched::SchedTables<BenchGlue> = kernel::sched::SchedTables::new();

impl ProcGlue for BenchGlue {
    type ProcessG = ();
    type ThreadG = ();
    type PgrpG = ();
    type SessionG = ();
    type FileGlue = BenchFiles;
    kernel::proc_glue_link_types!();

    fn tables() -> &'static ProcTables<Self> {
        &TABLES
    }

    fn sched() -> &'static kernel::sched::SchedTables<Self> {
        &SCHED
    }

    fn pgrp_destroy(_pg: &Pgrp<Self>) {}
    fn session_hangup(_sess: &Session<Self>) {}
    fn session_placed(_sess: &Session<Self>) {}
    fn session_destroy(_sess: &Session<Self>) {}
    fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R {
        f(None)
    }
    fn exit1(_p: &Thread<Self>, _xexit: i32, xsig: i32, _flags: i32) -> ! {
        std::panic!("exit1 in a bench (signal {xsig})")
    }
    fn pledge_fail(_p: &Thread<Self>, error: Errno, _code: u64) -> Errno {
        error
    }
    fn process_ref(pr: &Process<Self>) -> proc::ProcessRef<Self> {
        match hal::pool::PoolArc::from_ref(pr) {
            Some(a) => proc::StaticOrPooled::Pooled(a),
            None => std::process::exit(1),
        }
    }
}

/// The bench's process, static.
static PR: Process<BenchGlue> = Process::new(42, ());

/// The futex word of the private ops (the user-copy double's memory).
const WORD: usize = testutil::USER_BASE + 0x100;

/// An unmapped word of the bench's address space.
const UNMAPPED: usize = 0x5000_0000;

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    testutil::install();
    // The address space of the shared op: the map pools over the doubles' pages.
    let _pages = testutil::pages_install(256 * 4096);
    testutil::init_shared_once();
    hal::pmap::init();
    kernel::vm::map::uvm_map_init();
    kernel::futex::futex_init();

    let t: &'static Thread<BenchGlue> = Box::leak(Box::new(Thread::new_static(&PR, ())));
    let vm = kernel::vm::map::uvmspace_alloc(VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS, true, false);
    drop(t.set_vmspace(Some(vm)));
    if UserPtr::<u32>::new(WORD).copyout(&1).is_err() {
        eprintln!("bench: the futex word is not in the user-copy double");
        std::process::exit(1);
    }

    let wait = || {
        let r = kernel::futex::sys_futex::<BenchGlue>(t, WORD, FUTEX_WAIT_PRIVATE, 2, 0, 0);
        debug_assert_eq!(r, Err(Errno::EAGAIN));
        let _ = black_box(r);
    };
    let wake = || {
        let r = kernel::futex::sys_futex::<BenchGlue>(t, WORD, FUTEX_WAKE_PRIVATE, 1, 0, 0);
        debug_assert_eq!(r, Ok(0));
        let _ = black_box(r);
    };
    let wake_shared = || {
        let r = kernel::futex::sys_futex::<BenchGlue>(t, UNMAPPED, FUTEX_WAKE, 1, 0, 0);
        debug_assert_eq!(r, Ok(0));
        let _ = black_box(r);
    };

    let lock = hal::rwlock::RawRwlock::new("benchlk");
    let word = UserPtr::<u32>::new(WORD);

    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "futex.wait-eagain",
        ops: PRIM,
        f: Box::new(prim(|| {
            wait();
        })),
    });
    benches.push(Bench {
        name: "futex.wake-none",
        ops: PRIM,
        f: Box::new(prim(|| {
            wake();
        })),
    });
    benches.push(Bench {
        name: "prim.futex-wait-wake",
        ops: PRIM,
        f: Box::new(prim(|| {
            wait();
            wake();
        })),
    });
    benches.push(Bench {
        name: "futex.wake-shared-none",
        ops: PRIM,
        f: Box::new(prim(|| {
            wake_shared();
        })),
    });

    // What the ops above are made of, on these doubles (to tell the design from the doubles).
    benches.push(Bench {
        name: "prim.rwlock-write",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(lock.write())))),
    });
    benches.push(Bench {
        name: "prim.copyin32",
        ops: PRIM,
        f: Box::new(prim(|| {
            let _ = black_box(black_box(word).copyin32());
        })),
    });
    benches.push(Bench {
        name: "prim.vmspace-share",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(t.vmspace_share())))),
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
