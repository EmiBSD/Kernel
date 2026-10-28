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
//! Host microbenchmarks of pipe(2)'s hot paths (N5.0, decision 43): a pair made and closed,
//! a 512-byte write and read through it, the same with an `EVFILT_READ` knote on the read
//! side, and `pipe2(2)` with the close of both descriptors, over the kernel's host doubles
//! (`kernel::testutil`, feature `doubles`), a recycling pool backend and a recycling page
//! provider for the buffers (the doubles hand out fresh memory and never take it back, which
//! would time the host's allocator). Never part of a kernel image.
//!
//! `cargo bench -p kernel --features doubles --bench pipe` (release) or with
//! `--profile dev`. Arguments: the names of the benches to run (all by default).
//!
//! The glue is this file's, as `bsd`'s is for the kernel: a process glue with no member of
//! `bsd`'s, a file glue whose object holds a pipe's end (`bsd`'s `BsdFileData::Pipe`), and
//! the pipe's own glue: the clock is `kernel::tc`'s `getnanotime` (what `bsd`'s glue reads)
//! and the filterops find the pair through the knote's file, as `bsd`'s do. `dopipe`'s
//! descriptor work is this file's copy of `bsd`'s (`sys/kern/pipe_glue.rs`) over the
//! kernel's descriptor calls. What an op is:
//! - `pipe.create-close`: `pipe_pair_create` (the pool item, two buffers, the times) and
//!   both ends dropped (`pipe_destroy` twice, the pair back to the pool).
//! - `pipe.write-read-512`: one 512-byte write into a pair and one 512-byte read out of it.
//! - `pipe.write-read-512-kq`: the same through a pair whose read side has an `EVFILT_READ`
//!   knote of a kqueue (the write runs the filter through `knote`).
//! - `pipe.pipe2-close`: `pipe2(2)` (the pair, two files, two descriptors, the copyout of
//!   both numbers) and `close(2)` of both descriptors.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::kmbuf::{PageKind, PageMemory};
use hal::pool::{PoolBackend, RawPool};
use hal::sys::errno::Errno;
use hal::sys::types::{Off, Register};
use hal::user::UserPtr;
use kernel::descrip::{
    closef, falloc, fd_getfile, fdinit, fdinsert, fdplock, fdrelease, fdremove, filedesc_init,
};
use kernel::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_ADD, EV_EOF, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE,
    FILTEROP_ISFD, FILTEROP_MPSAFE, Filterops, Kevent, Knote, KqRef, ev_set, knote_modify,
    knote_process, kqueue_alloc, kqueue_init, kqueue_register,
};
use kernel::file::{DTYPE_PIPE, File, FileGlue, FileOps, FileTables, LockOwner};
use kernel::pipe::{
    Dir, PipeEnd, PipeFiles, PipeFilter, PipeFlags, PipeGlue, Pipes, Timespec, UioMove,
};
use kernel::proc::{self, ProcGlue, ProcTables, Process, Session, Thread};
use kernel::resource::{self, PlimitRef};
use kernel::testutil;

// ---------------------------------------------------------------------------------------
// The harness (event.rs's).

/// One bench: its name, the ops one call of `f` makes, and `f`, which returns the time of
/// the part it measures.
struct Bench<'a> {
    name: &'static str,
    ops: usize,
    f: Box<dyn FnMut() -> Duration + 'a>,
}

/// The calls of an op per timed call.
const PRIM: usize = 100;

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
// Recycling memory: the pool backend (vm.rs's) and the buffers' pages.

/// A page of the host.
const PAGE: usize = 4096;

/// A free list of blocks of one size, as addresses (the provenance exposed when they were
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

    /// A block: one given back, or a fresh slab carved into blocks.
    fn get(&self) -> Option<NonNull<u8>> {
        let mut free = self.lock();
        if free.is_empty() {
            let per = (256 * 1024 / self.stride).max(1);
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

/// The pipe buffers' pages: two sizes (`PIPE_SIZE`, `BIG_PIPE_SIZE`), each recycled. The
/// blocks given back are not zeroed again: no op here reads a buffer before writing it.
struct BenchPages {
    small: FreeList,
    big: FreeList,
}

impl PageMemory for BenchPages {
    fn alloc(&self, len: usize, _kind: PageKind, _wait: bool) -> Option<(NonNull<u8>, usize)> {
        let list = if len <= self.small.stride {
            &self.small
        } else if len <= self.big.stride {
            &self.big
        } else {
            return None;
        };
        Some((list.get()?, len))
    }

    fn free(&self, ptr: NonNull<u8>, len: usize, _kind: PageKind) {
        if len <= self.small.stride {
            self.small.put(ptr);
        } else {
            self.big.put(ptr);
        }
    }

    fn page_size(&self) -> usize {
        PAGE
    }
}

static BENCH_PAGES: BenchPages = BenchPages {
    small: FreeList::new(kernel::pipe::PIPE_SIZE),
    big: FreeList::new(kernel::pipe::BIG_PIPE_SIZE),
};

// ---------------------------------------------------------------------------------------
// The glue of this bench: no member of bsd's.

/// The bench's process glue.
pub struct BenchGlue;

kernel::proc_glue_links!(BenchGlue);

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

    fn session_hangup(_sess: &Session<Self>) {}
    fn session_placed(_sess: &Session<Self>) {}
    fn session_destroy(_sess: &Session<Self>) {}
    fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R {
        f(None)
    }
    fn exit1(_p: &Thread<Self>, _xexit: i32, xsig: i32, _flags: i32) -> ! {
        eprintln!("bench: exit1 (signal {xsig})");
        std::process::exit(1)
    }
    fn pledge_fail(_p: &Thread<Self>, error: Errno, _code: u64) -> Errno {
        error
    }
    fn pledge_syscall(_p: &Thread<Self>, _code: i32) -> Result<(), (Errno, u64)> {
        Ok(())
    }
    fn pledge_sysctl(_p: &Thread<Self>, _mib: &[i32], _new: usize) -> Result<(), Errno> {
        Ok(())
    }
    fn pledge_adjtime(_p: &Thread<Self>, _delta: usize) -> Result<(), Errno> {
        Ok(())
    }
    /// No interval timers: nothing is set, the old value stays zero.
    fn setitimer(
        _which: i32,
        _itv: Option<&kernel::time::Itimerval>,
        _olditv: Option<&mut kernel::time::Itimerval>,
    ) {
    }
    /// No per-CPU slice start: the uptime itself (no running part).
    fn spc_runtime() -> kernel::clock::Timespec {
        kernel::tc::nanouptime()
    }
    fn process_ref(pr: &Process<Self>) -> proc::ProcessRef<Self> {
        match hal::pool::PoolArc::from_ref(pr) {
            Some(a) => proc::StaticOrPooled::Pooled(a),
            None => std::process::exit(1),
        }
    }
}

/// The object of one of the bench's files (`f_data`): a pipe's end or a kqueue.
pub enum BenchData {
    /// A pipe file's pipe (`bsd`'s `BsdFileData::Pipe`).
    Pipe(PipeEnd<BenchPipe>),
    /// A kqueue file's kqueue.
    Kqueue(KqRef<BenchGlue>),
}

/// The bench's file glue.
pub struct BenchFiles;

kernel::file_glue_links!(BenchFiles);

/// `numfiles`.
static NUMFILES: AtomicI32 = AtomicI32::new(0);
static FILE_TABLES: FileTables<BenchFiles> = FileTables::new(0, &NUMFILES);

/// `kqueueops` of the bench's kqueue files.
pub struct BenchKqueueOps;

impl FileOps<BenchFiles> for BenchKqueueOps {
    fn read(
        &self,
        _fp: &File<BenchFiles>,
        _uio: &mut kernel::uio::Uio<'_, BenchGlue>,
        _fflags: i32,
    ) -> Result<(), Errno> {
        kernel::event::kqueue_rw()
    }
    fn write(
        &self,
        _fp: &File<BenchFiles>,
        _uio: &mut kernel::uio::Uio<'_, BenchGlue>,
        _fflags: i32,
    ) -> Result<(), Errno> {
        kernel::event::kqueue_rw()
    }
    fn ioctl(
        &self,
        _fp: &File<BenchFiles>,
        _com: u64,
        _data: &mut [u8],
        _p: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        kernel::event::kqueue_ioctl()
    }
    fn kqfilter(&self, fp: &File<BenchFiles>, kn: &Knote<BenchGlue>) -> Result<(), Errno> {
        kernel::event::kqueue_kqfilter::<BenchGlue>(fp, kn)
    }
    fn stat(
        &self,
        fp: &File<BenchFiles>,
        st: &mut testutil::TestStat,
        _p: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        let (size, _blksize) = kernel::event::kqueue_stat::<BenchGlue>(fp);
        *st = testutil::TestStat::default();
        st.st_size = size as u64;
        Ok(())
    }
    fn close(&self, fp: &File<BenchFiles>, p: Option<&Thread<BenchGlue>>) -> Result<(), Errno> {
        kernel::event::kqueue_close::<BenchGlue>(fp, p)
    }
}

static BENCH_KQUEUE_OPS: BenchKqueueOps = BenchKqueueOps;

impl FileGlue for BenchFiles {
    type Proc = BenchGlue;
    type Ops = dyn FileOps<BenchFiles>;
    type Data = BenchData;
    type Stat = testutil::TestStat;
    type Vnode = testutil::TestVnode;
    type FileList = FileListLink;

    fn tables() -> &'static FileTables<Self> {
        &FILE_TABLES
    }
    fn maxfiles() -> i32 {
        4096
    }
    fn tablefull(_tab: &str) {}
    fn stall_wait() {}
    fn vref(v: &'static Self::Vnode) {
        v.refs.fetch_add(1, Ordering::Relaxed);
    }
    fn vrele(v: &'static Self::Vnode) {
        v.refs.fetch_sub(1, Ordering::Relaxed);
    }
    fn advlock(
        _d: &Self::Data,
        _owner: LockOwner,
        _op: i32,
        _fl: &mut hal::sys::fcntl::Flock,
        _flags: i32,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn pathconf(_d: &Self::Data, _name: i32) -> Result<Register, Errno> {
        Err(Errno::EINVAL)
    }
    fn isatty(_d: &Self::Data) -> bool {
        false
    }
    fn position_check(_d: &Self::Data, _off: Off) -> Result<(), Errno> {
        Err(Errno::ESPIPE)
    }
    fn dns_socket(_d: &Self::Data) -> bool {
        false
    }
    fn kqueue_file(kq: KqRef<BenchGlue>) -> (&'static Self::Ops, Self::Data) {
        (&BENCH_KQUEUE_OPS, BenchData::Kqueue(kq))
    }
    fn kqueue_of(d: &Self::Data) -> Option<&KqRef<BenchGlue>> {
        match d {
            BenchData::Kqueue(kq) => Some(kq),
            BenchData::Pipe(_) => None,
        }
    }
    fn vnode_revoked(_d: &Self::Data) -> bool {
        false
    }
    fn sigpipe(_p: &Thread<BenchGlue>) {}
    fn pledge_fcntl(_p: &Thread<BenchGlue>, _cmd: i32) -> Result<(), Errno> {
        Ok(())
    }
    fn pledge_flock(_p: &Thread<BenchGlue>) -> Result<(), Errno> {
        Ok(())
    }
    fn pledge_ioctl(_p: &Thread<BenchGlue>, _com: u64, _fp: &File<Self>) -> Result<(), Errno> {
        Ok(())
    }
    fn stat_hide_gen(st: &mut Self::Stat) {
        st.st_gen = 0;
    }
    fn stat_copyout(st: &Self::Stat, sb: usize) -> Result<(), Errno> {
        UserPtr::<testutil::TestStat>::new(sb).copyout(st)
    }
}

/// The pipe's glue of this bench: the clock and the filterops, as `bsd`'s.
pub struct BenchPipe;

impl PipeGlue for BenchPipe {
    type Proc = BenchGlue;

    fn nanotime() -> Timespec {
        kernel::tc::getnanotime()
    }

    fn filterops(filter: PipeFilter) -> &'static Filterops<BenchGlue> {
        match filter {
            PipeFilter::Read => &PIPE_RFILTOPS,
            PipeFilter::Write => &PIPE_WFILTOPS,
            PipeFilter::Except => &PIPE_EFILTOPS,
        }
    }
}

/// The pipes of the bench, over the recycling pages.
static PIPES: Pipes<BenchPipe> = Pipes::new(&BENCH_PAGES);

/// The pair and the pipe of a pipe file (the pair counted again, as `bsd`'s `pipe_of`).
fn pipe_of(fp: &File<BenchFiles>) -> (hal::pool::PoolArc<kernel::pipe::PipePair<BenchPipe>>, Dir) {
    match fp.data() {
        Some(BenchData::Pipe(end)) => (end.pair().clone(), end.dir()),
        _ => std::process::exit(1),
    }
}

/// `pipeops`: what the rows reach of a pipe file (the I/O rows call the pair directly).
struct BenchPipeOps;

impl FileOps<BenchFiles> for BenchPipeOps {
    fn read(
        &self,
        _fp: &File<BenchFiles>,
        _uio: &mut kernel::uio::Uio<'_, BenchGlue>,
        _fflags: i32,
    ) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    fn write(
        &self,
        _fp: &File<BenchFiles>,
        _uio: &mut kernel::uio::Uio<'_, BenchGlue>,
        _fflags: i32,
    ) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    fn ioctl(
        &self,
        _fp: &File<BenchFiles>,
        _com: u64,
        _data: &mut [u8],
        _p: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Err(Errno::ENOTTY)
    }
    /// `pipe_kqfilter`: the knote's filter as the pipe's.
    fn kqfilter(&self, fp: &File<BenchFiles>, kn: &Knote<BenchGlue>) -> Result<(), Errno> {
        let (pair, dir) = pipe_of(fp);
        let filter = match kn.kn_filter().get() {
            EVFILT_READ => Some(PipeFilter::Read),
            EVFILT_WRITE => Some(PipeFilter::Write),
            EVFILT_EXCEPT => Some(PipeFilter::Except),
            _ => None,
        };
        pair.kqfilter(
            dir,
            filter,
            kn.has_flags(__EV_SELECT),
            kn.has_flags(__EV_POLL),
            kn,
        )
    }
    fn stat(
        &self,
        _fp: &File<BenchFiles>,
        _st: &mut testutil::TestStat,
        _p: &Thread<BenchGlue>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    /// The end is the file's object, dropped with the file's item.
    fn close(&self, _fp: &File<BenchFiles>, _p: Option<&Thread<BenchGlue>>) -> Result<(), Errno> {
        Ok(())
    }
}

static BENCH_PIPEOPS: BenchPipeOps = BenchPipeOps;

/// `filt_pipedetach`.
fn filt_pipedetach(kn: &Knote<BenchGlue>) {
    let (pair, _) = pipe_of(&kn.fp());
    let Some(hook) = Dir::from_index(kn.kn_hook.get().addr()) else {
        std::process::exit(1);
    };
    pair.detach(hook, kn);
}

/// One filter's answer written into the knote (`bsd`'s `filt_pipe`).
fn filt_pipe(kn: &Knote<BenchGlue>, which: PipeFilter) -> bool {
    let (pair, dir) = pipe_of(&kn.fp());
    let answer = pair.filter(dir, which, kn.has_flags(__EV_POLL));
    if let Some(data) = answer.data {
        kn.kn_data().set(data);
    }
    if answer.eof {
        kn.set_flags(EV_EOF);
    }
    if answer.hup {
        kn.set_flags(__EV_HUP);
    }
    answer.active
}

fn filt_piperead(kn: &Knote<BenchGlue>, _hint: i64) -> bool {
    filt_pipe(kn, PipeFilter::Read)
}

fn filt_pipewrite(kn: &Knote<BenchGlue>, _hint: i64) -> bool {
    filt_pipe(kn, PipeFilter::Write)
}

fn filt_pipeexcept(kn: &Knote<BenchGlue>, _hint: i64) -> bool {
    filt_pipe(kn, PipeFilter::Except)
}

fn filt_pipemodify(kev: &mut Kevent, kn: &Knote<BenchGlue>) -> bool {
    let (pair, _) = pipe_of(&kn.fp());
    pair.with_lock(|| knote_modify(kev, kn))
}

fn filt_pipeprocess(kn: &Knote<BenchGlue>, kev: Option<&mut Kevent>) -> bool {
    let (pair, _) = pipe_of(&kn.fp());
    pair.with_lock(|| knote_process(kn, kev))
}

static PIPE_RFILTOPS: Filterops<BenchGlue> = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_pipedetach),
    f_event: Some(filt_piperead),
    f_modify: Some(filt_pipemodify),
    f_process: Some(filt_pipeprocess),
};

static PIPE_WFILTOPS: Filterops<BenchGlue> = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_pipedetach),
    f_event: Some(filt_pipewrite),
    f_modify: Some(filt_pipemodify),
    f_process: Some(filt_pipeprocess),
};

static PIPE_EFILTOPS: Filterops<BenchGlue> = Filterops {
    f_flags: FILTEROP_ISFD | FILTEROP_MPSAFE,
    f_attach: None,
    f_detach: Some(filt_pipedetach),
    f_event: Some(filt_pipeexcept),
    f_modify: Some(filt_pipemodify),
    f_process: Some(filt_pipeprocess),
};

/// `dopipe`'s descriptor work for the thread `p` (`bsd`'s `BsdFiles`).
struct BenchDopipe<'a> {
    p: &'a Thread<BenchGlue>,
}

/// Sets up a new pipe file: both ends are `FREAD | FWRITE`, as in the C.
fn pipe_file(fp: &File<BenchFiles>, end: PipeEnd<BenchPipe>, flags: PipeFlags) {
    let fflag = 3 | if flags.nonblock { 0x0004 } else { 0 }; // FREAD | FWRITE [| FNONBLOCK]
    fp.f_flag.store(fflag, Ordering::SeqCst);
    fp.install(DTYPE_PIPE, &BENCH_PIPEOPS, BenchData::Pipe(end));
}

impl PipeFiles<BenchPipe> for BenchDopipe<'_> {
    /// `bsd`'s `dopipe` (`sys/kern/pipe_glue.rs`), over the kernel's descriptor calls.
    fn dopipe(
        &self,
        rpipe: PipeEnd<BenchPipe>,
        wpipe: PipeEnd<BenchPipe>,
        flags: PipeFlags,
        copyout: &dyn Fn([i32; 2]) -> Result<(), Errno>,
    ) -> Result<(), Errno> {
        let p = self.p;
        let fdp = p.fd();
        let fdflags = if flags.cloexec { 0x01 } else { 0 } | if flags.clofork { 0x04 } else { 0 };
        let mut g = fdplock(fdp);
        let (rf, fd0) = match falloc(p, &mut g) {
            Ok(r) => r,
            Err(error) => {
                drop(g);
                drop(wpipe);
                drop(rpipe);
                return Err(error);
            }
        };
        pipe_file(&rf, rpipe, flags);
        let (wf, fd1) = match falloc(p, &mut g) {
            Ok(r) => r,
            Err(error) => {
                let _ = fdremove(&mut g, fd0);
                let _ = closef(rf, Some(p));
                drop(g);
                drop(wpipe);
                return Err(error);
            }
        };
        pipe_file(&wf, wpipe, flags);
        fdinsert(&mut g, fd0, fdflags, &rf);
        fdinsert(&mut g, fd1, fdflags, &wf);
        let error = copyout([fd0, fd1]);
        if error.is_ok() {
            drop(g);
        } else {
            let _ = fdrelease(p, g, fd0);
            let g = fdplock(fdp);
            let _ = fdrelease(p, g, fd1);
        }
        let _ = rf.frele(Some(p));
        let _ = wf.frele(Some(p));
        error
    }
}

/// `pipe2(2)` of the bench's thread into its user buffer: the two descriptors.
fn pipe2(p: &Thread<BenchGlue>) -> [i32; 2] {
    if PIPES
        .sys_pipe2(UserPtr::new(UFDS), 0, &BenchDopipe { p })
        .is_err()
    {
        eprintln!("bench: pipe2 failed");
        std::process::exit(1);
    }
    let mut fds = [0i32; 2];
    if UserPtr::<i32>::new(UFDS).copyin_slice(&mut fds).is_err() {
        std::process::exit(1);
    }
    fds
}

// ---------------------------------------------------------------------------------------
// The rows.

/// Where `pipe2(2)` copies its two descriptors (the user-memory double).
const UFDS: usize = testutil::USER_BASE + 0x100;

/// A buffer the pipe copies into or out of (a `uio` of one kernel iovec).
struct Buf<'a> {
    data: &'a mut [u8],
    pos: usize,
    write: bool,
}

impl UioMove for Buf<'_> {
    fn resid(&self) -> usize {
        self.data.len() - self.pos
    }

    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
        let n = buf.len().min(self.resid());
        let user = &mut self.data[self.pos..self.pos + n];
        if self.write {
            buf[..n].copy_from_slice(user);
        } else {
            user.copy_from_slice(&buf[..n]);
        }
        self.pos += n;
        Ok(())
    }

    fn nonblock(&self) -> bool {
        true
    }

    fn is_write(&self) -> bool {
        self.write
    }

    fn set_resid(&mut self, resid: usize) {
        self.pos = self.data.len().saturating_sub(resid);
    }
}

/// One 512-byte write into `w`'s peer and one 512-byte read out of `r`'s pipe.
fn write_read(pair: &kernel::pipe::PipePair<BenchPipe>, src: &mut [u8; 512], dst: &mut [u8; 512]) {
    let mut wuio = Buf {
        data: src,
        pos: 0,
        write: true,
    };
    if pair.write(Dir::Wr, &mut wuio).is_err() {
        std::process::exit(1);
    }
    let mut ruio = Buf {
        data: dst,
        pos: 0,
        write: false,
    };
    if pair.read(Dir::Rd, &mut ruio).is_err() || ruio.pos != 512 {
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
    kernel::tc::inittimecounter();
    hal::klock::kernel_lock();
    proc::procinit::<BenchGlue>(1024, 1024);
    let _ = filedesc_init::<BenchFiles>();
    if kqueue_init::<BenchGlue>().is_err() || PIPES.init(9).is_err() {
        eprintln!("bench: kqueue_init or pipe_init failed");
        std::process::exit(1);
    }
    testutil::set_file_log(false);

    let pr = proc::process_alloc::<BenchGlue>(42, ());
    drop(pr.set_ucred(kernel::cred::CredRef::new(kernel::cred::Ucred::new())));
    let lim = resource::lim_startup(7030, 1 << 30, 4096);
    drop(pr.set_limit(PlimitRef::new(lim)));
    pr.set_fd(fdinit());
    let t = proc::thread_alloc::<BenchGlue>(&pr, ());
    drop(t.set_cred(pr.ucred()));
    let p: &Thread<BenchGlue> = &t;

    // Descriptors 0..2 taken, so pipe2 makes 3 and 4 as a program's would.
    for _ in 0..3 {
        let mut g = fdplock(p.fd());
        let Ok((fp, fd)) = falloc(p, &mut g) else {
            std::process::exit(1);
        };
        fp.f_flag.store(3, Ordering::SeqCst);
        let Ok(kq) = kqueue_alloc::<BenchGlue>(pr.fd_ref()) else {
            std::process::exit(1);
        };
        fp.install(
            kernel::file::DTYPE_KQUEUE,
            &BENCH_KQUEUE_OPS,
            BenchData::Kqueue(kq),
        );
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        let _ = fp.frele(Some(p));
    }

    // The pair of the plain row, and the pair of files with a knote on the read side.
    let Ok((r, w)) = PIPES.create() else {
        std::process::exit(1);
    };
    let plain = r.pair().clone();
    let fds = pipe2(p);
    let Some(rf) = fd_getfile(p.fd(), fds[0]) else {
        std::process::exit(1);
    };
    let (kqpair, _) = pipe_of(&rf);
    let _ = rf.frele(Some(p));
    let Ok(kq) = kqueue_alloc::<BenchGlue>(pr.fd_ref()) else {
        std::process::exit(1);
    };
    kernel::event::fd_kqlist_insert(&kq);
    let mut kev = ev_set(fds[0] as usize, EVFILT_READ, EV_ADD, 0, 0, 0);
    if kqueue_register(&kq, &mut kev, 0, Some(p)).is_err() || kqpair.klist_empty(Dir::Rd) {
        eprintln!("bench: the knote did not attach");
        std::process::exit(1);
    }

    let src = std::cell::RefCell::new([0x5au8; 512]);
    let dst = std::cell::RefCell::new([0u8; 512]);
    let mut benches: Vec<Bench<'_>> = Vec::new();
    benches.push(Bench {
        name: "pipe.create-close",
        ops: PRIM,
        f: Box::new(prim(|| {
            let Ok((r, w)) = PIPES.create() else {
                std::process::exit(1);
            };
            drop(black_box(w));
            drop(black_box(r));
        })),
    });
    benches.push(Bench {
        name: "pipe.write-read-512",
        ops: PRIM,
        f: Box::new(prim(|| {
            write_read(&plain, &mut src.borrow_mut(), &mut dst.borrow_mut());
        })),
    });
    benches.push(Bench {
        name: "pipe.write-read-512-kq",
        ops: PRIM,
        f: Box::new(prim(|| {
            write_read(&kqpair, &mut src.borrow_mut(), &mut dst.borrow_mut());
        })),
    });
    benches.push(Bench {
        name: "pipe.pipe2-close",
        ops: PRIM,
        f: Box::new(prim(|| {
            let fds = pipe2(p);
            for fd in fds {
                if fdrelease(p, fdplock(p.fd()), fd).is_err() {
                    std::process::exit(1);
                }
            }
        })),
    });

    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    let wanted = |name: &str| args.is_empty() || args.iter().any(|x| x == name);
    for bench in benches.iter_mut() {
        if wanted(bench.name) {
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
    }
    drop(benches);
    drop((r, w));
    std::process::exit(0);
}
/* </CODE> */
