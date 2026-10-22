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
//! Host microbenchmarks of N4.4's hot paths (decision 43): a knote's delivery on an object's
//! klist, a kqueue's scan, `kqueue_register`, `poll(2)` and `select(2)` over the thread's
//! poll kqueue, over the kernel's host doubles (`kernel::testutil`, feature `doubles`) and
//! a recycling pool backend (vm.rs's: the doubles' pool hands out a fresh zeroed page per
//! item and never takes one back, which would time the host's allocator). Never part of a
//! kernel image.
//!
//! `cargo bench -p kernel --features doubles --bench event` (release) or with
//! `--profile dev`. Arguments: the names of the benches to run (all by default).
//!
//! What an op is (the "before" of each `Bench:` line is the same loop over LZ's functions,
//! a scratch test of `bsd`'s kern_event.rs at 3109a3cc, whose pools were `bsd`'s): eight
//! files (descriptors 3..10) of a test object whose kernel-lock klist their `EVFILT_READ`
//! knotes hook on, the object always ready.
//! - `kn.deliver-1`, `kn.deliver-8`: `knote(&klist, 0)` with one, then eight, active and
//!   queued knotes on it (each filter runs; nothing is queued twice).
//! - `kq.scan-8`: one non-blocking `kqueue_scan` collecting the eight (level-triggered, so
//!   each is queued again).
//! - `prim.kn-fp`: a filter's `kn->kn_fp` (the slot's counted reference, then its drop).
//! - `kq.register-user`: `EV_ADD` then `EV_DELETE` of an `EVFILT_USER` knote.
//! - `kq.register-fd`: `EV_ADD` then `EV_DELETE` of `EVFILT_READ` on descriptor 3.
//! - `poll.1`, `poll.8`: `doppoll` without the copies: register the pollfds on the poll
//!   kqueue, one scan, collect, `kqpoll_done`.
//! - `select.8`: `dopselect` likewise over one word of each fd set.
//! - `prim.klist-own`: insert and remove on a klist of its own mutex (the locking forms).

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::klock;
use hal::pool::{PoolBackend, RawPool};
use hal::sys::errno::Errno;
use hal::sys::types::Off;
use kernel::descrip::{falloc, fdinit, fdinsert, fdplock, filedesc_init};
use kernel::event::{
    EV_ADD, EV_DELETE, EVFILT_READ, EVFILT_USER, FILTEROP_ISFD, FILTEROP_MPSAFE, Filterops, Kevent,
    Klist, Knote, KqRef, Kqueue, ev_set, fd_kqlist_insert, knote_modify, knote_process,
    kqpoll_done, kqpoll_init, kqueue_alloc, kqueue_init, kqueue_register, kqueue_scan,
    kqueue_scan_setup, thread_kq,
};
use kernel::file::{DTYPE_PIPE, File, FileOps};
use kernel::proc::{self, Pgrp, ProcGlue, ProcTables, Process, Session, Thread};
use kernel::resource::{self, PlimitRef};
use kernel::select::{
    FdMask, POLLIN, Pollfd, ppollcollect, ppollregister, pselcollect, pselregister,
};
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
// take pool items (knotes, kqueues) get them from here.

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
// The glue of this bench: no member of bsd's.

/// The bench's glue.
pub struct BenchGlue;

kernel::proc_glue_links!(BenchGlue);
kernel::test_file_glue!(BenchFiles, BenchGlue);

static TABLES: ProcTables<BenchGlue> = ProcTables::new();

impl ProcGlue for BenchGlue {
    type ProcessG = ();
    type ThreadG = ();
    type PgrpG = ();
    type SessionG = ();
    type RunqEntry = ();
    type FileGlue = BenchFiles;
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
    fn curproc() -> Option<&'static Thread<Self>> {
        None
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

/// The test object: its kernel-lock klist, and whether its event is ready.
static OBJ_KLIST: Klist<BenchGlue> = Klist::new();
static OBJ_READY: AtomicBool = AtomicBool::new(true);

fn filt_objdetach(kn: &Knote<BenchGlue>) {
    OBJ_KLIST.remove(kn);
}

fn filt_obj(kn: &Knote<BenchGlue>, _hint: i64) -> bool {
    let ready = OBJ_READY.load(Ordering::Relaxed);
    kn.kn_data().set(i64::from(ready));
    ready
}

fn filt_objmodify(kev: &mut Kevent, kn: &Knote<BenchGlue>) -> bool {
    knote_modify(kev, kn)
}

fn filt_objprocess(kn: &Knote<BenchGlue>, kev: Option<&mut Kevent>) -> bool {
    knote_process(kn, kev)
}

static OBJ_FILTOPS: Filterops<BenchGlue> = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_objdetach),
    f_event: Some(filt_obj),
    f_modify: Some(filt_objmodify),
    f_process: Some(filt_objprocess),
};

/// The object's files: `EVFILT_READ` hooks the knote on the object's klist.
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
        _: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }
    fn kqfilter(&self, _: &File<BenchFiles>, kn: &Knote<BenchGlue>) -> Result<(), Errno> {
        if kn.kn_filter().get() != EVFILT_READ {
            return Err(Errno::EINVAL);
        }
        kn.kn_fop.set(Some(&OBJ_FILTOPS));
        OBJ_KLIST.insert(kn);
        Ok(())
    }
    fn stat(
        &self,
        _: &File<BenchFiles>,
        _: &mut testutil::TestStat,
        _: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn close(&self, _: &File<BenchFiles>, _: Option<&Thread<BenchGlue>>) -> Result<(), Errno> {
        Ok(())
    }
    fn seek(
        &self,
        _: &File<BenchFiles>,
        _: &mut Off,
        _: i32,
        _: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ESPIPE)
    }
}

static BENCH_OPS: BenchOps = BenchOps;

/// One non-blocking scan of `kq` into `out`: the events collected.
fn scan(p: &Thread<BenchGlue>, kq: &KqRef<BenchGlue>, out: &mut [Kevent]) -> usize {
    let start = core::pin::pin!(Knote::marker(kq));
    let end = core::pin::pin!(Knote::marker(kq));
    let s = kqueue_scan_setup(kq, start.as_ref(), end.as_ref());
    let mut ts = kernel::clock::Timespec::ZERO;
    let n = out.len();
    kqueue_scan(&s, n, out, Some(&mut ts), p).0
}

/// `doppoll` without the copies.
fn poll_once(p: &Thread<BenchGlue>, pl: &mut [Pollfd]) -> usize {
    let nfds = pl.len() as u32;
    if kqpoll_init(p, nfds).is_err() {
        std::process::exit(1);
    }
    let (mut nevents, mut ncollected) = (0, 0);
    ppollregister(p, pl, &mut nevents, &mut ncollected);
    if nevents > 0 {
        let kq = thread_kq(p).clone();
        let mut kev = [Kevent::default(); 8];
        let count = nevents.min(kev.len());
        let ready = scan(p, &kq, &mut kev[..count]);
        for k in &kev[..ready] {
            ncollected += ppollcollect(p, k, pl);
        }
    }
    kqpoll_done(p, nfds);
    ncollected
}

/// `dopselect` without the copies, over one word of each set: the read bits back.
fn select_once(p: &Thread<BenchGlue>, nd: usize, r: FdMask) -> FdMask {
    if kqpoll_init(p, nd as u32).is_err() {
        std::process::exit(1);
    }
    let pibits = [r, 0, 0];
    let mut pobits = [0 as FdMask; 3];
    let mut ncollected = 0;
    if let Ok(nevents) = pselregister(p, &pibits, 1, nd) {
        let kq = thread_kq(p).clone();
        let mut kev = [Kevent::default(); 8];
        let count = nevents.min(kev.len());
        let ready = scan(p, &kq, &mut kev[..count]);
        for k in &kev[..ready] {
            let _ = pselcollect(p, k, &mut pobits, 1, &mut ncollected);
        }
    }
    kqpoll_done(p, nd as u32);
    pobits[0]
}

fn register(p: &Thread<BenchGlue>, kq: &KqRef<BenchGlue>, kev: Kevent) {
    let mut kev = kev;
    if kqueue_register(kq, &mut kev, 0, Some(p)).is_err() {
        eprintln!("bench: kqueue_register {} {} failed", kev.ident, kev.filter);
        std::process::exit(1);
    }
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
    let _ = filedesc_init::<BenchFiles>();
    if kqueue_init::<BenchGlue>().is_err() {
        eprintln!("bench: kqueue_init failed");
        std::process::exit(1);
    }
    testutil::set_test_maxfiles(4096);
    testutil::set_file_log(false);

    let pr = proc::process_alloc::<BenchGlue>(42, ());
    drop(pr.set_ucred(kernel::cred::CredRef::new(kernel::cred::Ucred::new())));
    let lim = resource::lim_startup(7030, 1 << 30, 4096);
    drop(pr.set_limit(PlimitRef::new(lim)));
    pr.set_fd(fdinit());
    let t = proc::thread_alloc::<BenchGlue>(&pr, (), ());
    drop(t.set_cred(pr.ucred()));
    let p: &Thread<BenchGlue> = &t;
    // Descriptors 0..10, the object's files from 3.
    for _ in 0..11 {
        let mut g = fdplock(p.fd());
        let Ok((fp, fd)) = falloc(p, &mut g) else {
            eprintln!("bench: falloc failed");
            std::process::exit(1);
        };
        fp.f_flag.store(3, Ordering::SeqCst); // FREAD | FWRITE
        fp.install(DTYPE_PIPE, &BENCH_OPS, testutil::TestFileData::default());
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        let _ = fp.frele(Some(p));
    }
    let Ok(kq) = kqueue_alloc::<BenchGlue>(pr.fd_ref()) else {
        eprintln!("bench: kqueue_alloc failed");
        std::process::exit(1);
    };
    fd_kqlist_insert(&kq);
    let mut out = [Kevent::default(); 8];
    let own: &'static Klist<BenchGlue> = Box::leak(Box::new(Klist::new()));
    own.init_own();
    let lone = kernel::event::test_knote(&kq, 999); // not the user rows' ident

    let mut benches: Vec<Bench<'_>> = Vec::new();
    // kn.deliver-1 and -8 are timed in main's order: one knote first, then eight.
    register(p, &kq, ev_set(3, EVFILT_READ, EV_ADD, 0, 0, 0));
    let one = measure(
        &mut Bench {
            name: "kn.deliver-1",
            ops: PRIM,
            f: Box::new(prim(|| OBJ_KLIST.knote(0))),
        },
        7,
        Duration::from_millis(300),
    );
    for fd in 4..11 {
        register(p, &kq, ev_set(fd, EVFILT_READ, EV_ADD, 0, 0, 0));
    }
    benches.push(Bench {
        name: "kn.deliver-8",
        ops: PRIM,
        f: Box::new(prim(|| OBJ_KLIST.knote(0))),
    });
    let kq2 = &kq;
    let out2 = std::cell::RefCell::new(&mut out);
    benches.push(Bench {
        name: "kq.scan-8",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(scan(p, kq2, &mut out2.borrow_mut()[..]));
        })),
    });
    let user_bench = Bench {
        name: "kq.register-user",
        ops: PRIM,
        f: Box::new(prim(|| {
            register(p, kq2, ev_set(1, EVFILT_USER, EV_ADD, 0, 0, 0));
            register(p, kq2, ev_set(1, EVFILT_USER, EV_DELETE, 0, 0, 0));
        })),
    };
    let fd_bench = Bench {
        name: "kq.register-fd",
        ops: PRIM,
        f: Box::new(prim(|| {
            register(p, kq2, ev_set(3, EVFILT_READ, EV_ADD, 0, 0, 0));
            register(p, kq2, ev_set(3, EVFILT_READ, EV_DELETE, 0, 0, 0));
        })),
    };
    let pl1 = std::cell::RefCell::new([Pollfd {
        fd: 3,
        events: POLLIN,
        revents: 0,
    }]);
    let pl8 = std::cell::RefCell::new(
        (3..11)
            .map(|fd| Pollfd {
                fd,
                events: POLLIN,
                revents: 0,
            })
            .collect::<Vec<_>>(),
    );
    let poll1 = Bench {
        name: "poll.1",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(poll_once(p, &mut pl1.borrow_mut()[..]));
        })),
    };
    let poll8 = Bench {
        name: "poll.8",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(poll_once(p, &mut pl8.borrow_mut()[..]));
        })),
    };
    let select8 = Bench {
        name: "select.8",
        ops: PRIM,
        f: Box::new(prim(|| {
            black_box(select_once(p, 11, 0x7f8));
        })),
    };
    let lone_kn = lone.kn();
    let klist_own = Bench {
        name: "prim.klist-own",
        ops: PRIM,
        f: Box::new(prim(|| {
            own.insert(lone_kn);
            own.remove(lone_kn);
        })),
    };

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    let report = |name: &str, (min, med, max, calls): (f64, f64, f64, usize)| {
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            name,
            min,
            med,
            max,
            (max - min) / med * 100.0,
            calls
        );
    };
    let wanted = |name: &str| args.is_empty() || args.iter().any(|x| x == name);
    if wanted("kn.deliver-1") {
        report("kn.deliver-1", one);
    }
    for bench in benches.iter_mut() {
        if wanted(bench.name) {
            report(bench.name, measure(bench, reps, target));
        }
    }
    drop(benches);
    // prim.kn-fp: the first knote, read under the kernel lock's reader held for the bench.
    if wanted("prim.kn-fp") {
        let r = klock::reader();
        let Some(kn) = OBJ_KLIST.first(&r) else {
            eprintln!("bench: no knote on the object");
            std::process::exit(1);
        };
        let mut b = Bench {
            name: "prim.kn-fp",
            ops: PRIM,
            f: Box::new(prim(|| drop(black_box(kn.fp())))),
        };
        report("prim.kn-fp", measure(&mut b, reps, target));
    }
    // The eight object knotes go before the register and poll rows, as the "before"'s.
    for fd in 3..11 {
        register(p, &kq, ev_set(fd, EVFILT_READ, EV_DELETE, 0, 0, 0));
    }
    let mut rest = [user_bench, fd_bench, poll1, poll8, select8, klist_own];
    for bench in rest.iter_mut() {
        if wanted(bench.name) {
            report(bench.name, measure(bench, reps, target));
        }
    }
    let _: &Kqueue<BenchGlue> = &kq;
    std::process::exit(0);
}
/* </CODE> */
