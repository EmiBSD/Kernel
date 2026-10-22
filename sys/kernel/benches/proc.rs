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
//! - `proc.thread-join-leave`: a thread box joins a process of eight threads and leaves it
//!   again under its `ps_mtx` (`thread_join`'s and `exit1`'s moves: the C's
//!   `TAILQ_INSERT_TAIL` and `TAILQ_REMOVE` of `ps_threads`).
//!
//! N4.3's descriptors (`kernel::descrip`), over a table of 19 open files (the "before" a
//! scratch test of `bsd`'s kern_descrip.rs at abfbed96, whose pools were `bsd`'s):
//! - `prim.fd-get`: `fd_getfile(fdp, 3)` then `FRELE`.
//! - `prim.fd-dup-close`: `dup(3)` as the C makes it (`fd_getfile`, `fdalloc`,
//!   `finishdup`) then `close` of the new descriptor (`fdrelease`).
//! - `fd.copy-20`: `fdcopy` of the table then its last handle dropped (LZ: `fdcopy` +
//!   `fdfree`).
//! - `fd.prepforexec-20`: `fdprepforexec` over the 19 descriptors, none close-on-exec.
//! - `prim.vn-read-offset`: `vn_read`'s use of the offset: `foffset(fp)` read, then the
//!   update `f_offset += n` under `f_mtx` (N4.3 review S3: the read takes no `f_mtx`, the
//!   update one section, as the C's).
//! - `prim.mutexed`: a `hal::Mutexed` taken and released (`fd_fplock`'s cost on these
//!   doubles; LZ's `mtx_enter`/`mtx_leave` over `bsd`'s was 3.0 ns in the same run as the
//!   "before").
//!
//! The test glue's hooks are the kernel's doubles (`kernel::test_file_glue!`), with their
//! record of `knote_fdclose` turned off.

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
use hal::sys::errno::Errno;
use hal::sys::types::Off;
use hal::sys::types::Pid;
use kernel::cred::{CredRef, Ucred, crdup};
use kernel::descrip::{
    falloc, fd_getfile, fdalloc, fdcopy, fdinit, fdinsert, fdplock, fdprepforexec, fdrelease,
    filedesc_init, finishdup,
};
use kernel::file::{DTYPE_PIPE, File, FileOps, foffset};
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
// timeout(9).

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

    fn pgsignal(_pg: &Pgrp<Self>, _sig: i32, _checkctty: bool) {}
    fn prsignal(_pr: &Process<Self>, _sig: i32) {}
    fn pgrp_destroy(_pg: &Pgrp<Self>) {}
    fn session_hangup(_sess: &Session<Self>) {}
    fn session_placed(_sess: &Session<Self>) {}
    fn session_destroy(_sess: &Session<Self>) {}
}

/// The bench's files' operations: nothing is read or written through them.
struct BenchOps;

impl FileOps<BenchFiles> for BenchOps {
    fn read(
        &self,
        _: &File<BenchFiles>,
        _: &mut kernel::uio::Uio<'_, BenchGlue>,
        _: i32,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn write(
        &self,
        _: &File<BenchFiles>,
        _: &mut kernel::uio::Uio<'_, BenchGlue>,
        _: i32,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn ioctl(
        &self,
        _: &File<BenchFiles>,
        _: u64,
        _: &mut [u8],
        _: &proc::Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }
    fn kqfilter(&self, _: &File<BenchFiles>, _: &()) -> Result<(), Errno> {
        Err(Errno::EINVAL)
    }
    fn stat(
        &self,
        _: &File<BenchFiles>,
        _: &mut testutil::TestStat,
        _: &proc::Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn close(
        &self,
        _: &File<BenchFiles>,
        _: Option<&proc::Thread<BenchGlue>>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn seek(
        &self,
        _: &File<BenchFiles>,
        _: &mut Off,
        _: i32,
        _: &proc::Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ESPIPE)
    }
}

static BENCH_OPS: BenchOps = BenchOps;

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
    let t = proc::thread_alloc::<BenchGlue>(&pr, ());
    drop(t.set_cred(pr.ucred()));
    // N4.3: a table of 19 open files (descriptors 0..18).
    let _ = filedesc_init::<BenchFiles>();
    testutil::set_test_maxfiles(4096);
    testutil::set_file_log(false);
    pr.set_fd(fdinit());
    for _ in 0..19 {
        let mut g = fdplock(t.fd());
        let Ok((fp, fd)) = falloc(&t, &mut g) else {
            eprintln!("bench: falloc failed");
            std::process::exit(1);
        };
        fp.install(DTYPE_PIPE, &BENCH_OPS, testutil::TestFileData::default());
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        let _ = fp.frele(Some(&t));
    }
    let other = proc::process_alloc::<BenchGlue>(4242, ());
    let pid: Pid = 4242;
    let joined = proc::process_alloc::<BenchGlue>(43, ());
    {
        let mut g = joined.ps_mtx.lock();
        for i in 0..8 {
            let b = proc::thread_alloc::<BenchGlue>(&joined, ());
            b.p_tid.set(100_000 + i);
            joined.insert_thread_locked(&mut g, b);
        }
    }
    let leaver = std::cell::Cell::new(Some(proc::thread_alloc::<BenchGlue>(&joined, ())));
    // The joining thread's order number, as `exit1` reads it from the thread (`p_seq`).
    let next_seq = std::cell::Cell::new(8u64);

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

    benches.push(Bench {
        name: "proc.thread-join-leave",
        ops: PRIM,
        f: Box::new(prim(|| {
            let Some(b) = leaver.take() else {
                return;
            };
            b.p_tid.set(100_100);
            let mut g = joined.ps_mtx.lock();
            joined.insert_thread_locked(&mut g, b);
            let seq = next_seq.get();
            next_seq.set(seq + 1);
            leaver.set(joined.remove_thread_locked(&mut g, seq));
        })),
    });

    let t3 = &t;
    benches.push(Bench {
        name: "prim.fd-get",
        ops: PRIM,
        f: Box::new(prim(|| {
            if let Some(fp) = fd_getfile(t3.fd(), black_box(3)) {
                let _ = fp.frele(Some(t3));
            }
        })),
    });
    benches.push(Bench {
        name: "prim.fd-dup-close",
        ops: PRIM,
        f: Box::new(prim(|| {
            let Some(fp) = fd_getfile(t3.fd(), 3) else {
                return;
            };
            let mut g = fdplock(t3.fd());
            let Ok(new) = fdalloc(t3, &mut g, 0) else {
                return;
            };
            let _ = finishdup(t3, g, fp, 3, new, 0);
            let _ = fdrelease(t3, fdplock(t3.fd()), new);
        })),
    });
    let mtx = hal::mutex::Mutexed::new(hal::intr::mpfloor(), 0u32);
    benches.push(Bench {
        name: "prim.mutexed",
        ops: PRIM,
        f: Box::new(prim(|| *black_box(&mtx).lock() += 1)),
    });
    let Some(fpo) = fd_getfile(t3.fd(), 3) else {
        eprintln!("bench: no descriptor 3");
        std::process::exit(1);
    };
    let fpo: &'static _ = Box::leak(Box::new(fpo));
    benches.push(Bench {
        name: "prim.vn-read-offset",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(foffset(fpo));
            fpo.advance_offset(black_box(1));
        })),
    });
    let pr3 = &pr;
    benches.push(Bench {
        name: "fd.copy-20",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(fdcopy(pr3))))),
    });
    benches.push(Bench {
        name: "fd.prepforexec-20",
        ops: PRIM,
        f: Box::new(prim(|| fdprepforexec(t3))),
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
