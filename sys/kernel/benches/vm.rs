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
//! Host microbenchmarks of `kernel::vm`'s hot paths (decision 42, phase 1): the real code of
//! the fault handler, the map, the amaps and anons, the objects and the page system, over the
//! kernel's host doubles (`kernel::testutil`, feature `doubles`). Never part of a kernel
//! image; it measures and changes nothing.
//!
//! `cargo bench -p kernel --features doubles --bench vm` (release: the `bench` profile) or
//! with `--profile dev` (the kernel's opt-level 1). Arguments: `list`; the names of the
//! benches to run (all by default); `--profile NAME SECS` loops one bench for `SECS` seconds,
//! for a sampling profiler attached to the process (`sample <pid>`).
//!
//! Two doubles differ from testutil's on purpose: the pool and `malloc(9)` backends here
//! recycle the items given back (testutil's leak a fresh, zeroed, page-aligned block per
//! item, which would make a 4 KiB `calloc` the cost of every `pool_get` and run the host out
//! of memory in a long loop). They are registered before testutil's, which then find the
//! slots taken. Recycling needs no `unsafe`: an item given back is kept as its address
//! (`expose_provenance`) and handed out again from it (`with_exposed_provenance_mut`).
//!
//! What an op is, after `tools/bench/bench.c`:
//! - `pagefault`: the first write of one page of a fresh 1024-page anonymous mapping, the
//!   `mmap` and `munmap` of the mapping spread over its pages (the guest's op); the phases
//!   apart: `pagefault.mmap`, `pagefault.fault`, `pagefault.munmap`.
//! - `fault.anon-read`: a read fault of a fresh anonymous page (zero fill, case 2B).
//! - `fault.lower-read`: a read fault of a file page already resident in the vnode's object
//!   (case 2A, no I/O), map and unmap spread as in `pagefault`.
//! - `fault.lower-cow`: a write fault of a private file mapping (promotion of a copy, 2B).
//! - `fault.upper-cow`: a child's write of a page its parent wrote before the fork (the amap
//!   copied once per mapping, a new anon with a copy of the page: case 1B), the faults alone
//!   timed; `fork.big` is that fork of a 1024-page mapping alone.
//! - `forkexit`: `uvmspace_fork` of a vmspace laid out as a small static program's, the
//!   child's stack write, `uvmspace_free` of the child, the parent's writes after it (stack
//!   and data); `forkexit.uarea` the u-area's `uvm_uarea_alloc`/`uvm_uarea_free` beside it.
//! - `forkexec`: the same with the child's `uvmspace_exec`, the mappings `exec` makes and the
//!   faults of a trivial program; `exec` times that part alone.
//! - `map.anon`, `map.syscall`: `uvm_mapanon` + `uvm_unmap` of one page, and `sys_mmap` +
//!   `sys_munmap`, in a map holding the program's entries and 64 more.
//! - `mprotect.k`, `madvise.k`, `minherit.k`, `mlock-munlock.k` (k = 64, 1024): the call
//!   pair over a range of k one-page anonymous entries, alternately read-only and
//!   read-write (none join), faulted (to read-write and back to read-only; random and normal
//!   advice; shared and copy inheritance; wire and unwire), ns per call; `munmap.k` the
//!   `uvm_unmap` of such a range alone (laid out again after it); `fork.k` the
//!   `uvmspace_fork` of the vmspace holding it alone (N12.1: the map's walks).
//! - `cmp.*`: anonymous memory only, through `uvm_mapanon`/`uvm_unmap`/`uvm_fault`/
//!   `uvmspace_fork`/`uvmspace_free` (a 1024-page mapping faulted; a fork of 70 one-page
//!   entries, alone, its free, and with a copy-on-write fault on each side; a one-page map
//!   and unmap): the calls a scratch test can make over LZ's legacy uvm before N3, to compare
//!   the two on the host.
//! - Primitives: `prim.*`, the pieces the paths are made of, one by one.

#![forbid(unsafe_code)]

use std::boxed::Box;
use std::collections::BTreeMap;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::vec::Vec;

use hal::arch::current::vmparam::{MAXSSIZ, USRSTACK, VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS};
use hal::malloc::Malloc;
use hal::pool::{PoolBackend, RawPool};
use hal::rwlock::SharedRwlock;
use hal::sys::errno::Errno;
use hal::sys::mman::{
    MADV_NORMAL, MADV_RANDOM, MAP_ANON, MAP_INHERIT_COPY, MAP_INHERIT_SHARE, MAP_PRIVATE,
    PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE,
};
use kernel::proc::{ProcGlue, ProcTables, Process, Session, Thread};
use kernel::testutil;
use kernel::vm::amap::{amap_init, amap_lookup};
use kernel::vm::anon::{uvm_analloc, uvm_anon_init};
use kernel::vm::aobj::uao_init;
use kernel::vm::fault::{FaultMapGuard, VM_FAULT_INVALID, uvm_fault, uvmfault_init};
use kernel::vm::glue::{uvm_uarea_alloc, uvm_uarea_free};
use kernel::vm::map::{
    Vmspace, VmspaceRef, uvm_map, uvm_map_advice, uvm_map_inherit, uvm_map_init, uvm_map_pageable,
    uvm_map_protect, uvm_mapanon, uvm_unmap, uvmspace_alloc, uvmspace_exec, uvmspace_fork,
    uvmspace_free,
};
use kernel::vm::mmap::{MmapArgs, sys_mmap, sys_munmap};
use kernel::vm::object::ObjRef;
use kernel::vm::page::{Owner, uvm_pagealloc, uvm_pagefree};
use kernel::vm::types::{
    PROT_MASK, UVM_FLAG_COPYONW, UVM_FLAG_FIXED, UVM_FLAG_STACK, UVM_PGA_ZERO, UVM_UNKNOWN_OFFSET,
    Voff, uvm_mapflag,
};
use kernel::vm::vnode::{uvn_attach, uvn_init};

/// The page size of the doubles.
const PAGE: usize = 4096;
/// The guest's pages per mapping (`FAULT_PAGES`).
const FAULT_PAGES: usize = 1024;
/// Physical memory for the page system: 64 MiB.
const MEM_PAGES: usize = 16384;

// ---------------------------------------------------------------------------------------
// The recycling pool and malloc doubles.

/// Aborts the bench with a message (no `panic!` in code clippy reads as kernel code).
fn kernel_panic(what: &str) -> ! {
    eprintln!("vm bench: {what}");
    std::process::exit(2)
}

/// `r`'s value, or the bench stops naming `what`.
fn ok<T, E: core::fmt::Debug>(r: Result<T, E>, what: &str) -> T {
    match r {
        Ok(v) => v,
        Err(e) => kernel_panic(&format!("{what}: {e:?}")),
    }
}

/// `o`'s value, or the bench stops naming `what`.
fn some<T>(o: Option<T>, what: &str) -> T {
    match o {
        Some(v) => v,
        None => kernel_panic(&format!("{what}: none")),
    }
}

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

/// The recycling `malloc(9)` double: one free list per size class (16-byte steps, then
/// pages).
struct BenchMalloc {
    classes: Mutex<BTreeMap<usize, &'static FreeList>>,
}

impl BenchMalloc {
    fn class(&self, size: usize) -> &'static FreeList {
        let stride = if size <= PAGE {
            size.max(16).next_multiple_of(16)
        } else {
            size.next_multiple_of(PAGE)
        };
        let mut c = self
            .classes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        c.entry(stride)
            .or_insert_with(|| Box::leak(Box::new(FreeList::new(stride))))
    }
}

impl Malloc for BenchMalloc {
    fn malloc(&self, size: usize, _ty: i32, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        Some((self.class(size).get()?, size))
    }

    fn free(&self, ptr: NonNull<u8>, _ty: i32, size: usize) {
        self.class(size).put(ptr);
    }
}

static BENCH_POOLS: BenchPools = BenchPools;
static BENCH_MALLOC: BenchMalloc = BenchMalloc {
    classes: Mutex::new(BTreeMap::new()),
};

// ---------------------------------------------------------------------------------------
// The bench's process (N5.0): the vm entry points take the calling thread (its limits, its
// `p_ru`), as the C passes `struct proc *p`.

/// The bench's glue: no member of bsd's.
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

    fn session_hangup(_sess: &Session<Self>) {}
    fn session_placed(_sess: &Session<Self>) {}
    fn session_destroy(_sess: &Session<Self>) {}
    fn with_curproc<R>(f: impl FnOnce(Option<&Thread<Self>>) -> R) -> R {
        f(None)
    }
    fn exit1(_p: &Thread<Self>, _xexit: i32, xsig: i32, _flags: i32) -> ! {
        kernel_panic(&format!("exit1 (signal {xsig})"))
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
    fn pledge_protexec(_p: &Thread<Self>, _prot: kernel::vm::types::VmProt) -> Result<(), Errno> {
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
    fn process_ref(pr: &Process<Self>) -> kernel::proc::ProcessRef<Self> {
        match hal::pool::PoolArc::from_ref(pr) {
            Some(a) => kernel::proc::StaticOrPooled::Pooled(a),
            None => kernel_panic("process_ref of the static process"),
        }
    }
}

/// The bench's process, static.
static PR: Process<BenchGlue> = Process::new(42, ());

/// The bench's thread (made once by [`thread`]): the faults count on its `p_ru`, `mmap(2)`
/// reads its limits.
static THREAD: std::sync::OnceLock<&'static Thread<BenchGlue>> = std::sync::OnceLock::new();

/// The bench's thread, made the first time with `lim_startup`'s limits (infinite data and
/// stack, as the doubles' answer before N5.0).
fn thread() -> &'static Thread<BenchGlue> {
    THREAD.get_or_init(|| {
        let lim = kernel::resource::lim_startup(7030, 1 << 30, PAGE);
        drop(PR.set_limit(kernel::resource::PlimitRef::new(lim)));
        Box::leak(Box::new(Thread::new_static(&PR, ())))
    })
}

/// The faulting thread's counters (`curproc->p_ru`).
fn ru() -> Option<&'static kernel::resource::Rusage> {
    Some(&thread().p_ru)
}

// ---------------------------------------------------------------------------------------
// Set-up.

/// The kernel's pieces as the map tests set them up, over the recycling doubles; the page
/// system's lock held for the whole run.
fn setup() -> MutexGuard<'static, ()> {
    let _ = hal::bridges::register_pool_backend(&BENCH_POOLS);
    let _ = hal::bridges::register_malloc(&BENCH_MALLOC);
    let guard = testutil::pages_install(MEM_PAGES * PAGE);
    testutil::init_shared_once();
    ok(uvm_anon_init(0), "uvm_anon_init");
    ok(amap_init(0), "amap_init");
    ok(uao_init(), "uao_init");
    hal::pmap::init();
    uvm_map_init();
    uvmfault_init();
    ok(uvn_init(), "uvn_init");
    testutil::kernel_maps_install();
    guard
}

/// A user vmspace laid out as `exec` leaves one (the map tests' `user_vmspace`).
fn user_vmspace() -> VmspaceRef {
    let vm = uvmspace_alloc(VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS, true, false);
    vm.vm_daddr.store(256 << 20, Ordering::Relaxed);
    vm.vm_minsaddr.store(USRSTACK, Ordering::Relaxed);
    vm.vm_maxsaddr.store(USRSTACK - MAXSSIZ, Ordering::Relaxed);
    vm
}

const RW: i32 = PROT_READ | PROT_WRITE;

/// A fault, which must succeed.
fn fault(vm: &Vmspace, va: usize, access: i32) {
    ok(
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, access, ru()),
        "uvm_fault",
    );
}

/// A small static program: its file (text then data) and where `exec` maps it.
struct Program {
    vp: &'static testutil::MemVnode,
}

const TEXT: usize = 0x0020_0000;
const TEXT_PAGES: usize = 128;
const DATA: usize = TEXT + TEXT_PAGES * PAGE;
const DATA_PAGES: usize = 4;
const BSS: usize = DATA + DATA_PAGES * PAGE;
const BSS_PAGES: usize = 8;
/// `ep_ssize`: `DFLSSIZ`, 8 MiB.
const SSIZ: usize = 8 << 20;
const HEAP_PAGES: usize = 16;

impl Program {
    fn new() -> Self {
        let len = (TEXT_PAGES + DATA_PAGES) * PAGE;
        let vp = testutil::mem_vnode((0..len).map(|i| (i % 251) as u8).collect());
        Self { vp }
    }

    /// The vnode's object for one mapping (`uvn_attach` under the kernel lock, as exec's
    /// `vmcmd_map_pagedvn`).
    fn object(&self, prot: i32) -> ObjRef {
        hal::klock::kernel_lock();
        let o = some(uvn_attach(self.vp, prot), "uvn_attach");
        hal::klock::kernel_unlock();
        o
    }

    /// The mappings `exec` makes (`vmcmd_map_pagedvn` for text and data, `vmcmd_map_zero`
    /// for bss and the stack's two parts) and the heap's first `mmap`; the stack's top.
    fn map(&self, vm: &Vmspace) -> (usize, usize) {
        let fixed = |prot: i32, extra: u32| {
            uvm_mapflag(
                prot,
                PROT_MASK,
                MAP_INHERIT_COPY,
                MADV_NORMAL,
                UVM_FLAG_FIXED | UVM_FLAG_COPYONW | extra,
            )
        };
        let m = &vm.vm_map;
        let mut a = TEXT;
        let text = self.object(PROT_READ | PROT_EXEC);
        let f = fixed(PROT_READ | PROT_EXEC, 0);
        ok(
            uvm_map(m, &mut a, TEXT_PAGES * PAGE, Some(text), 0, 0, f),
            "map text",
        );
        let mut a = DATA;
        let data = self.object(RW);
        let off = (TEXT_PAGES * PAGE) as Voff;
        let f = fixed(RW, 0);
        ok(
            uvm_map(m, &mut a, DATA_PAGES * PAGE, Some(data), off, 0, f),
            "map data",
        );
        let mut a = BSS;
        let f = fixed(RW, 0);
        let bss = uvm_map(m, &mut a, BSS_PAGES * PAGE, None, UVM_UNKNOWN_OFFSET, 0, f);
        ok(bss, "map bss");
        let mut a = USRSTACK - MAXSSIZ;
        let f = fixed(PROT_NONE, 0);
        let guard = uvm_map(m, &mut a, MAXSSIZ - SSIZ, None, UVM_UNKNOWN_OFFSET, 0, f);
        ok(guard, "map stack guard");
        let mut a = USRSTACK - SSIZ;
        let f = fixed(RW, UVM_FLAG_STACK);
        let stack = uvm_map(m, &mut a, SSIZ, None, UVM_UNKNOWN_OFFSET, 0, f);
        ok(stack, "map stack");
        let heap = ok(
            sys_mmap(thread(), vm, anon_args(HEAP_PAGES * PAGE)),
            "mmap the heap",
        );
        (heap, USRSTACK)
    }

    /// What a trivial program touches once mapped: text read, data and bss written, the
    /// stack's top pages and the heap's first pages.
    fn run(&self, vm: &Vmspace, heap: usize, stack: usize) {
        for i in 0..16 {
            fault(vm, TEXT + i * PAGE, PROT_READ);
        }
        fault(vm, DATA, PROT_WRITE);
        fault(vm, DATA + PAGE, PROT_WRITE);
        fault(vm, BSS, PROT_WRITE);
        fault(vm, BSS + PAGE, PROT_WRITE);
        fault(vm, stack - PAGE, PROT_WRITE);
        fault(vm, stack - 2 * PAGE, PROT_WRITE);
        for i in 0..4 {
            fault(vm, heap + i * PAGE, PROT_WRITE);
        }
    }
}

/// `mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_ANON | MAP_PRIVATE, -1, 0)`.
fn anon_args(len: usize) -> MmapArgs {
    MmapArgs {
        addr: 0,
        len,
        prot: RW,
        flags: MAP_ANON | MAP_PRIVATE,
        fd: -1,
        pos: 0,
    }
}

/// Where the range operations' entries start.
const RANGE_BASE: usize = 0x6000_0000;

/// The entry counts of the range operations, with their benches' names (`mprotect`,
/// `madvise`, `minherit`, `mlock` + `munlock`, `munmap`, `fork`).
const RANGE_KS: [(usize, [&str; 6]); 2] = [
    (
        64,
        [
            "mprotect.64",
            "madvise.64",
            "minherit.64",
            "mlock-munlock.64",
            "munmap.64",
            "fork.64",
        ],
    ),
    (
        1024,
        [
            "mprotect.1024",
            "madvise.1024",
            "minherit.1024",
            "mlock-munlock.1024",
            "munmap.1024",
            "fork.1024",
        ],
    ),
];

/// The vmspaces of one entry count of the range operations.
struct Range {
    k: usize,
    names: [&'static str; 6],
    /// `mprotect`'s, `madvise`'s, `minherit`'s, `mlock`'s and `fork`'s.
    ops: VmspaceRef,
    /// `munmap`'s.
    unmap: VmspaceRef,
}

/// `k` one-page anonymous entries from `RANGE_BASE`, alternately read-only and read-write
/// (so no two join), each faulted.
fn lay_out(vm: &Vmspace, k: usize) {
    for i in 0..k {
        let (prot, access) = if i % 2 == 0 {
            (PROT_READ, PROT_READ)
        } else {
            (RW, PROT_WRITE)
        };
        let flags = uvm_mapflag(
            prot,
            PROT_MASK,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
        );
        let mut a = RANGE_BASE + i * PAGE;
        ok(uvm_mapanon(&vm.vm_map, &mut a, PAGE, 0, flags), "lay out");
        fault(vm, a, access);
    }
}

// ---------------------------------------------------------------------------------------
// The harness.

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

/// Times `f` whole.
fn whole(mut f: impl FnMut()) -> impl FnMut() -> Duration {
    move || {
        let t = Instant::now();
        f();
        t.elapsed()
    }
}

/// ns per op of `reps` repetitions of about `target` of wall time each (after a
/// calibration that is the warm-up): min, median, max, and the calls per repetition.
fn measure(b: &mut Bench<'_>, reps: usize, target: Duration) -> (f64, f64, f64, usize) {
    // Calibrate by wall time: a phase bench measures a part of each call only.
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

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let _g = setup();
    let prog = Program::new();

    // The parent of the fork benches: the program mapped and run, plus 64 small mappings
    // (a map of a few dozen entries, as a libc process has).
    let parent = user_vmspace();
    let (heap, stack) = prog.map(&parent);
    prog.run(&parent, heap, stack);
    let mut small = Vec::new();
    for _ in 0..64 {
        small.push(ok(
            sys_mmap(thread(), &parent, anon_args(PAGE)),
            "mmap a small mapping",
        ));
    }
    for &s in &small {
        fault(&parent, s, PROT_WRITE);
    }

    // A 4 MiB file for the lower-layer faults, its pages made resident once.
    let bigfile = testutil::mem_vnode((0..FAULT_PAGES * PAGE).map(|i| (i % 253) as u8).collect());
    let lower = user_vmspace();
    let map_file = |vm: &Vmspace, prot: i32| -> usize {
        hal::klock::kernel_lock();
        let o = some(uvn_attach(bigfile, prot), "uvn_attach");
        hal::klock::kernel_unlock();
        let mut a = 0x4000_0000;
        let f = uvm_mapflag(
            prot,
            PROT_MASK,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
        );
        ok(
            uvm_map(&vm.vm_map, &mut a, FAULT_PAGES * PAGE, Some(o), 0, 0, f),
            "map the file",
        );
        a
    };
    {
        let a = map_file(&lower, PROT_READ);
        for i in 0..FAULT_PAGES {
            fault(&lower, a + i * PAGE, PROT_READ);
        }
        uvm_unmap(&lower.vm_map, a, a + FAULT_PAGES * PAGE);
    }

    // The parent of `fault.upper-cow`: 1024 written pages.
    let cow_parent = user_vmspace();
    let cow_va = ok(
        sys_mmap(thread(), &cow_parent, anon_args(FAULT_PAGES * PAGE)),
        "mmap cow",
    );
    for i in 0..FAULT_PAGES {
        fault(&cow_parent, cow_va + i * PAGE, PROT_WRITE);
    }

    let pf = user_vmspace();
    let lock = some(SharedRwlock::new("benchlk"), "SharedRwlock::new");
    let anon = some(uvm_analloc(&lock), "uvm_analloc");
    let len = FAULT_PAGES * PAGE;
    let cmp_flags = uvm_mapflag(
        RW,
        PROT_MASK,
        MAP_INHERIT_COPY,
        MADV_NORMAL,
        UVM_FLAG_COPYONW,
    );
    let cmp_pf_vm = user_vmspace();
    let cmp_parent_vm = user_vmspace();
    let mut cmp_small = Vec::new();
    for _ in 0..70 {
        let mut a = 0;
        ok(
            uvm_mapanon(&cmp_parent_vm.vm_map, &mut a, PAGE, 0, cmp_flags),
            "mapanon",
        );
        fault(&cmp_parent_vm, a, PROT_WRITE);
        cmp_small.push(a);
    }
    // The range operations' vmspaces: k one-page entries each, for the operations and for
    // munmap's (which lays its range out again after each timed call).
    let ranges: Vec<Range> = RANGE_KS
        .iter()
        .map(|&(k, names)| {
            let ops = user_vmspace();
            lay_out(&ops, k);
            let unmap = user_vmspace();
            lay_out(&unmap, k);
            Range {
                k,
                names,
                ops,
                unmap,
            }
        })
        .collect();
    let mut benches: Vec<Bench<'_>> = Vec::new();

    benches.push(Bench {
        name: "pagefault",
        ops: FAULT_PAGES,
        f: Box::new(whole(|| {
            let p = ok(sys_mmap(thread(), &pf, anon_args(len)), "mmap");
            for i in 0..FAULT_PAGES {
                fault(&pf, p + i * PAGE, PROT_WRITE);
            }
            ok(sys_munmap(&pf, p, len), "munmap");
        })),
    });
    for (name, phase) in [
        ("pagefault.mmap", 0),
        ("pagefault.fault", 1),
        ("pagefault.munmap", 2),
    ] {
        let pf = &pf;
        benches.push(Bench {
            name,
            ops: FAULT_PAGES,
            f: Box::new(move || {
                let t0 = Instant::now();
                let p = ok(sys_mmap(thread(), pf, anon_args(len)), "mmap");
                let t1 = Instant::now();
                for i in 0..FAULT_PAGES {
                    fault(pf, p + i * PAGE, PROT_WRITE);
                }
                let t2 = Instant::now();
                ok(sys_munmap(pf, p, len), "munmap");
                let t3 = Instant::now();
                [t1 - t0, t2 - t1, t3 - t2][phase]
            }),
        });
    }
    benches.push(Bench {
        name: "fault.anon-read",
        ops: FAULT_PAGES,
        f: Box::new(|| {
            let p = ok(sys_mmap(thread(), &pf, anon_args(len)), "mmap");
            let t = Instant::now();
            for i in 0..FAULT_PAGES {
                fault(&pf, p + i * PAGE, PROT_READ);
            }
            let d = t.elapsed();
            ok(sys_munmap(&pf, p, len), "munmap");
            d
        }),
    });
    for (name, prot, access) in [
        ("fault.lower-read", PROT_READ, PROT_READ),
        ("fault.lower-cow", RW, PROT_WRITE),
    ] {
        let lower = &lower;
        let map_file = &map_file;
        benches.push(Bench {
            name,
            ops: FAULT_PAGES,
            f: Box::new(move || {
                let a = map_file(lower, prot);
                let t = Instant::now();
                for i in 0..FAULT_PAGES {
                    fault(lower, a + i * PAGE, access);
                }
                let d = t.elapsed();
                uvm_unmap(&lower.vm_map, a, a + len);
                d
            }),
        });
    }
    benches.push(Bench {
        name: "fault.upper-cow",
        ops: FAULT_PAGES,
        f: Box::new(|| {
            let child = uvmspace_fork(&cow_parent);
            let t = Instant::now();
            for i in 0..FAULT_PAGES {
                fault(&child, cow_va + i * PAGE, PROT_WRITE);
            }
            let d = t.elapsed();
            uvmspace_free(child);
            // The parent's pages are write-protected by the fork: its next writes are
            // faults again (not timed), as the guest's would be.
            for i in 0..FAULT_PAGES {
                fault(&cow_parent, cow_va + i * PAGE, PROT_WRITE);
            }
            d
        }),
    });
    benches.push(Bench {
        name: "fork.big",
        ops: 1,
        f: Box::new(|| {
            let t = Instant::now();
            let child = uvmspace_fork(&cow_parent);
            let d = t.elapsed();
            uvmspace_free(child);
            d
        }),
    });
    for r in &ranges {
        let m = &r.ops.vm_map;
        let end = RANGE_BASE + r.k * PAGE;
        let [mprotect, madvise, minherit, mlock, munmap, fork] = r.names;
        benches.push(Bench {
            name: mprotect,
            ops: 2,
            f: Box::new(whole(move || {
                let p = |prot| uvm_map_protect(thread(), m, RANGE_BASE, end, prot, 0, false, true);
                ok(p(RW), "mprotect RW");
                ok(p(PROT_READ), "mprotect READ");
            })),
        });
        benches.push(Bench {
            name: madvise,
            ops: 2,
            f: Box::new(whole(move || {
                ok(uvm_map_advice(m, RANGE_BASE, end, MADV_RANDOM), "madvise");
                ok(uvm_map_advice(m, RANGE_BASE, end, MADV_NORMAL), "madvise");
            })),
        });
        benches.push(Bench {
            name: minherit,
            ops: 2,
            f: Box::new(whole(move || {
                let share = uvm_map_inherit(m, RANGE_BASE, end, MAP_INHERIT_SHARE);
                ok(share, "minherit");
                ok(
                    uvm_map_inherit(m, RANGE_BASE, end, MAP_INHERIT_COPY),
                    "minherit",
                );
            })),
        });
        benches.push(Bench {
            name: mlock,
            ops: 2,
            f: Box::new(whole(move || {
                ok(uvm_map_pageable(m, RANGE_BASE, end, false, ru()), "mlock");
                ok(uvm_map_pageable(m, RANGE_BASE, end, true, ru()), "munlock");
            })),
        });
        let unmap_vm = &r.unmap;
        let k = r.k;
        benches.push(Bench {
            name: munmap,
            ops: 1,
            f: Box::new(move || {
                let t = Instant::now();
                uvm_unmap(&unmap_vm.vm_map, RANGE_BASE, end);
                let d = t.elapsed();
                lay_out(unmap_vm, k);
                d
            }),
        });
        let parent = &r.ops;
        benches.push(Bench {
            name: fork,
            ops: 1,
            f: Box::new(move || {
                let t = Instant::now();
                let child = uvmspace_fork(parent);
                let d = t.elapsed();
                uvmspace_free(child);
                d
            }),
        });
    }

    let parent_ref = &parent;
    let forkexit = move || {
        let child = uvmspace_fork(parent_ref);
        fault(&child, stack - PAGE, PROT_WRITE);
        uvmspace_free(child);
        fault(parent_ref, stack - PAGE, PROT_WRITE);
        fault(parent_ref, DATA, PROT_WRITE);
    };
    benches.push(Bench {
        name: "forkexit",
        ops: 1,
        f: Box::new(whole(forkexit)),
    });
    benches.push(Bench {
        name: "forkexit.fork",
        ops: 1,
        f: Box::new(move || {
            let t = Instant::now();
            let child = uvmspace_fork(parent_ref);
            let d = t.elapsed();
            fault(&child, stack - PAGE, PROT_WRITE);
            uvmspace_free(child);
            fault(parent_ref, stack - PAGE, PROT_WRITE);
            fault(parent_ref, DATA, PROT_WRITE);
            d
        }),
    });
    benches.push(Bench {
        name: "forkexit.free",
        ops: 1,
        f: Box::new(move || {
            let child = uvmspace_fork(parent_ref);
            fault(&child, stack - PAGE, PROT_WRITE);
            let t = Instant::now();
            uvmspace_free(child);
            let d = t.elapsed();
            fault(parent_ref, stack - PAGE, PROT_WRITE);
            fault(parent_ref, DATA, PROT_WRITE);
            d
        }),
    });
    benches.push(Bench {
        name: "forkexit.uarea",
        ops: 1,
        f: Box::new(whole(|| {
            let va = some(uvm_uarea_alloc(), "uvm_uarea_alloc");
            uvm_uarea_free(black_box(va));
        })),
    });
    let prog_ref = &prog;
    let exec_part = move |child: &Vmspace| {
        let n = uvmspace_exec(child, VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS);
        if n.is_some() {
            kernel_panic("uvmspace_exec made a new vmspace");
        }
        let (heap, top) = prog_ref.map(child);
        prog_ref.run(child, heap, top);
    };
    benches.push(Bench {
        name: "forkexec",
        ops: 1,
        f: Box::new(whole(move || {
            let child = uvmspace_fork(parent_ref);
            fault(&child, stack - PAGE, PROT_WRITE);
            exec_part(&child);
            uvmspace_free(child);
            fault(parent_ref, stack - PAGE, PROT_WRITE);
            fault(parent_ref, DATA, PROT_WRITE);
        })),
    });
    benches.push(Bench {
        name: "exec",
        ops: 1,
        f: Box::new(move || {
            let child = uvmspace_fork(parent_ref);
            fault(&child, stack - PAGE, PROT_WRITE);
            let t = Instant::now();
            exec_part(&child);
            let d = t.elapsed();
            uvmspace_free(child);
            fault(parent_ref, stack - PAGE, PROT_WRITE);
            fault(parent_ref, DATA, PROT_WRITE);
            d
        }),
    });
    benches.push(Bench {
        name: "exec.free",
        ops: 1,
        f: Box::new(move || {
            let child = uvmspace_fork(parent_ref);
            exec_part(&child);
            let t = Instant::now();
            uvmspace_free(child);
            let d = t.elapsed();
            fault(parent_ref, stack - PAGE, PROT_WRITE);
            fault(parent_ref, DATA, PROT_WRITE);
            d
        }),
    });
    benches.push(Bench {
        name: "map.anon",
        ops: 1,
        f: Box::new(whole(move || {
            let mut a = 0;
            let f = uvm_mapflag(
                RW,
                PROT_MASK,
                MAP_INHERIT_COPY,
                MADV_NORMAL,
                UVM_FLAG_COPYONW,
            );
            ok(
                uvm_mapanon(&parent_ref.vm_map, &mut a, PAGE, 0, f),
                "uvm_mapanon",
            );
            uvm_unmap(&parent_ref.vm_map, a, a + PAGE);
        })),
    });
    benches.push(Bench {
        name: "map.syscall",
        ops: 1,
        f: Box::new(whole(move || {
            let p = ok(sys_mmap(thread(), parent_ref, anon_args(PAGE)), "mmap");
            ok(sys_munmap(parent_ref, p, PAGE), "munmap");
        })),
    });

    // The cmp.* ops: anonymous memory only, through `uvm_mapanon` and `uvm_unmap`, the same
    // calls a scratch bench makes over LZ's legacy uvm before N3 (no file, no syscall layer).
    let cmp_pf = &cmp_pf_vm;
    benches.push(Bench {
        name: "cmp.pagefault",
        ops: FAULT_PAGES,
        f: Box::new(whole(move || {
            let mut a = 0;
            ok(
                uvm_mapanon(&cmp_pf.vm_map, &mut a, len, 0, cmp_flags),
                "mapanon",
            );
            for i in 0..FAULT_PAGES {
                fault(cmp_pf, a + i * PAGE, PROT_WRITE);
            }
            uvm_unmap(&cmp_pf.vm_map, a, a + len);
        })),
    });
    benches.push(Bench {
        name: "cmp.pagefault.fault",
        ops: FAULT_PAGES,
        f: Box::new(move || {
            let mut a = 0;
            ok(
                uvm_mapanon(&cmp_pf.vm_map, &mut a, len, 0, cmp_flags),
                "mapanon",
            );
            let t = Instant::now();
            for i in 0..FAULT_PAGES {
                fault(cmp_pf, a + i * PAGE, PROT_WRITE);
            }
            let d = t.elapsed();
            uvm_unmap(&cmp_pf.vm_map, a, a + len);
            d
        }),
    });
    let cmp_parent = &cmp_parent_vm;
    let first_small = cmp_small[0];
    benches.push(Bench {
        name: "cmp.fork70",
        ops: 1,
        f: Box::new(move || {
            let t = Instant::now();
            let child = uvmspace_fork(cmp_parent);
            let d = t.elapsed();
            uvmspace_free(child);
            d
        }),
    });
    benches.push(Bench {
        name: "cmp.fork70.free",
        ops: 1,
        f: Box::new(move || {
            let child = uvmspace_fork(cmp_parent);
            let t = Instant::now();
            uvmspace_free(child);
            t.elapsed()
        }),
    });
    benches.push(Bench {
        name: "cmp.forkcow70",
        ops: 1,
        f: Box::new(whole(move || {
            let child = uvmspace_fork(cmp_parent);
            fault(&child, first_small, PROT_WRITE);
            uvmspace_free(child);
            fault(cmp_parent, first_small, PROT_WRITE);
        })),
    });
    benches.push(Bench {
        name: "cmp.map",
        ops: 1,
        f: Box::new(whole(move || {
            let mut a = 0;
            ok(
                uvm_mapanon(&cmp_parent.vm_map, &mut a, PAGE, 0, cmp_flags),
                "mapanon",
            );
            uvm_unmap(&cmp_parent.vm_map, a, a + PAGE);
        })),
    });

    // Primitives: `PRIM` per timed call, so the clock's own cost (about 20 ns) is spread.
    benches.push(Bench {
        name: "prim.empty",
        ops: PRIM,
        f: Box::new(prim(|| black_box(()))),
    });
    benches.push(Bench {
        name: "prim.poolarc-clone-drop",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(anon.clone())))),
    });
    benches.push(Bench {
        name: "prim.rwlock-write",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(lock.write())))),
    });
    benches.push(Bench {
        name: "prim.rwlock-read",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(lock.read())))),
    });
    benches.push(Bench {
        name: "prim.sharedrwlock-clone-drop",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(lock.clone())))),
    });
    benches.push(Bench {
        name: "prim.anon-alloc-free",
        ops: PRIM,
        f: Box::new(prim(|| drop(black_box(uvm_analloc(&lock))))),
    });
    benches.push(Bench {
        name: "prim.map-lock-read",
        ops: PRIM,
        f: Box::new(prim(move || drop(black_box(parent_ref.vm_map.lock_read())))),
    });
    benches.push(Bench {
        name: "prim.map-lock-write",
        ops: PRIM,
        f: Box::new(prim(move || drop(black_box(parent_ref.vm_map.lock())))),
    });
    benches.push(Bench {
        name: "prim.amap-lookup",
        ops: PRIM,
        f: Box::new(prim(move || {
            let g = parent_ref.vm_map.lock_read();
            let e = some(g.lookup(BSS), "the bss entry");
            let (amap, off) = some(FaultMapGuard::aref(&g, e), "the bss amap");
            let w = amap.lock().read();
            black_box(amap_lookup(&amap, &w, off, BSS - e.start()).is_some());
        })),
    });
    benches.push(Bench {
        name: "prim.map-lookup",
        ops: PRIM,
        f: Box::new(prim(move || {
            let g = parent_ref.vm_map.lock_read();
            black_box(g.lookup(BSS).is_some());
        })),
    });
    for (name, flags) in [
        ("prim.page-alloc-free", 0),
        ("prim.page-alloc-zero-free", UVM_PGA_ZERO),
    ] {
        benches.push(Bench {
            name,
            ops: PRIM,
            f: Box::new(prim(move || {
                let pg = some(uvm_pagealloc(Owner::None, 0, flags), "uvm_pagealloc");
                uvm_pagefree(black_box(pg), Owner::None);
            })),
        });
    }
    let held = some(uvm_pagealloc(Owner::None, 0, 0), "uvm_pagealloc");
    benches.push(Bench {
        name: "prim.zero-page",
        ops: PRIM,
        f: Box::new(prim(move || {
            hal::pmap::zero_page(black_box(held.phys_addr()))
        })),
    });

    if args.first().map(String::as_str) == Some("list") {
        for b in &benches {
            println!("{}", b.name);
        }
        // The vmspaces are left as they are: their teardown is not measured.
        std::process::exit(0);
    }
    if args.first().map(String::as_str) == Some("--profile") {
        let name = some(args.get(1), "--profile NAME SECS");
        let secs: u64 = ok(some(args.get(2), "--profile NAME SECS").parse(), "SECS");
        let b = some(
            benches.iter_mut().find(|b| b.name == name.as_str()),
            "no such bench",
        );
        eprintln!(
            "pid {} profiling {} for {secs} s",
            std::process::id(),
            b.name
        );
        let end = Instant::now() + Duration::from_secs(secs);
        let mut calls = 0u64;
        let mut t = Duration::ZERO;
        while Instant::now() < end {
            t += (b.f)();
            calls += 1;
        }
        println!(
            "{} {:.1} ns/op over {calls} calls",
            b.name,
            t.as_nanos() as f64 / (calls as f64 * b.ops as f64)
        );
        // The vmspaces are left as they are: their teardown is not measured.
        std::process::exit(0);
    }
    let reps = 7;
    let target = Duration::from_millis(300);
    println!("# bench min median max ns/op, spread (max-min)/median, calls/rep, reps {reps}");
    for b in benches.iter_mut() {
        if !args.is_empty() && !args.iter().any(|a| a == b.name) {
            continue;
        }
        let (min, med, max, calls) = measure(b, reps, target);
        println!(
            "{:<30} {:>12.1} {:>12.1} {:>12.1} {:>6.1}% {:>8}",
            b.name,
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
