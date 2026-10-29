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
//! The kernel crate's host test doubles of `hal`'s bridges (`cfg(test)`, or feature `doubles`
//! for the tests of a crate that uses the kernel's pieces, never a kernel image; declared so
//! in `lib.rs`), in safe code: this crate forbids `unsafe`, its tests included.
//!
//! `hal` keeps its own doubles crate-private (`sys/hal/testutil.rs`); this crate's tests, and
//! `bsd`'s (`sys/kern/pipe_glue.rs`), run in a process of their own, so they register these
//! instead (never the real bridges), once, the first time a test calls [`install`]. What they keep per test thread (the running "process" and "CPU"
//! words, the pending sleep) lets two test threads contend for a lock and sleep and wake each
//! other as two processes do.
//!
//! - `IntrHardware`: `hal::intr`'s default levels (each test thread's core), whose
//!   `splassert` fails the test.
//! - `Sleep`: each test thread a process; a wait channel has a generation that every wakeup
//!   bumps, `sleep_setup` records it and `sleep_finish` waits on one condition variable until
//!   it moves (a wakeup between the two is never lost); no signals, so `PCATCH` changes
//!   nothing.
//! - `PageMemory` (not a bridge: [`page_memory`] hands it out as a value) and `PoolBackend`:
//!   zeroed, page-aligned blocks of the host's heap, leaked
//!   (a block given back is counted and never reused: freeing it would need the pointer
//!   turned back into its `Box`, which is `unsafe`); the pages out are counted per
//!   thread, the items out per pool; [`fail_pages`] makes a thread's next page allocations
//!   fail, as a full kernel map does. A pool's `set_hiwat` and `cache_init` are recorded by
//!   its name ([`pool_knobs`]).
//! - `Malloc`: the same leaked blocks (page-aligned, so `MALLOC_ALIGN`-aligned), the bytes out
//!   counted per thread and malloc type ([`malloc_out`]); [`fail_pages`] fails them too.
//! - The user copy is `hal`'s own on the host (`hal::arch::host::copy`, N4.8b): one user
//!   address space of [`USER_LEN`] bytes at [`USER_BASE`], one run of "foreign" kernel
//!   memory of [`FOREIGN_LEN`] bytes at [`FOREIGN_BASE`] ([`user_bytes`], [`foreign_bytes`],
//!   [`set_foreign_bytes`] are its, re-exported).
//! - `Printf`: every line formatted into one buffer the tests read ([`printf_count`]).
//! - The kernel lock is `hal::klock`'s own since N4.5c, a per-thread depth on the host (no
//!   exclusion; the tests that take it hold [`MEMORY`]); [`kernel_lock_depth`] reads it.
//! - `NetLock`: a per-thread "held for writing" flag ([`net_lock_set`], [`net_lock_held`]).
//! - The soft interrupts are `hal::softintr`'s own (no bridge since N4.5c).
//! - The clocks are not a double since N5.0: `kernel::tc`'s readers on its dummy counter (a
//!   microsecond more per read at its nominal 1 MHz), whose timehands nothing winds up but a
//!   test that calls `tc::tc_ticktock` (the coarse clocks stay where they are).
//! - `thread`'s operations: a host thread holding the kernel lock double from its start; a
//!   deferred creation runs at once in this crate's own tests and is held everywhere else
//!   (`bsd`'s host tests, the benches), as LZ's host tests never ran `main`'s
//!   `kthread_run_deferred_queue`: those tests drive a queue's workers by hand, and a worker
//!   thread would race them; `kthread_exit` unwinds the thread, which its spawner catches.

//! - `vm::proc`'s `VmProc`: each test thread a process, whose answers and records are a
//!   per-thread [`VmProcState`] ([`vmproc_with`]); `sigexit` panics with its signal.
//! - `vm::vnode`'s `VnodeOps`, not a slot: [`MemVnode`], a vnode whose file is a `Vec` in
//!   memory ([`mem_vnode`] leaks one), which records what it saw during its last read.
//! - `vm::km`'s maps, not a double: [`kernel_maps_install`] sets up the kernel's own
//!   `kernel_map`, `kmem_map` and a one-page `exec_map` (the kernel's `VmMap`s, switched to
//!   best fit as `uvm_init_late` does) once per process, in a range no host address uses, and
//!   counts the calls `km` makes to them per thread ([`km_map_calls`]). `km`'s pages are the
//!   kernel's page system over a heap block ([`pages_install`]).
//! - The page system itself (`vm::page`, `vm::pmemrange`), not a double: [`pmap_boot`]
//!   bootstraps the host pmap once per process with the kernel's provider
//!   (`vm::page::PHYS_PAGES`), and [`pages_setup`]/[`pages_install`] load fresh segments
//!   under [`MEMORY`], which every test of the page system holds (one set of statics).

use std::boxed::Box;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::string::String;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, Once, OnceLock};
use std::time::{Duration, Instant};
use std::vec;
use std::vec::Vec;

use hal::bridges;
use hal::intr::IntrHardware;
use hal::kmbuf::{PageKind, PageMemory};
use hal::link::Link;
use hal::malloc::Malloc;
use hal::netlock::NetLock;
use hal::pmap::BootstrapParams;
use hal::pool::{PoolBackend, RawPool};
use hal::prf::Printf;
use hal::sleep::{INFSLP, Sleep};
use hal::sys::errno::Errno;
use hal::sys::types::{Paddr, Vaddr};
use hal::vm::uvmexp::UVMEXP;

use crate::vm::km::{KvMap, LegacyLend};
use crate::vm::page::{
    PHYS_PAGES, SegPolicy, SegStrat, uvm_page_init, uvm_page_physload, uvm_page_test_reset,
    uvm_setpagesize,
};
use crate::vm::types::{UvmConstraintRange, Voff};
use crate::vm::vnode::{Uvn, VnodeOps};

/// The page size of the doubles.
const TEST_PAGE: usize = 4096;

/// The host's user and foreign memory (`hal`'s machine copies on the host, N4.8b).
pub use hal::arch::host::copy::{
    FOREIGN_BASE, FOREIGN_LEN, USER_BASE, USER_LEN, foreign_bytes, set_foreign_bytes, user_bytes,
};

/// One page of the doubles' memory: page-aligned, so a run of them is too.
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
struct Page([u8; TEST_PAGE]);

std::thread_local! {

    static PROC: u8 = const { 0 };
    static PENDING: Cell<Option<(usize, u64)>> = const { Cell::new(None) };
    static COLD: Cell<bool> = const { Cell::new(false) };
    static COLD_YIELDS: Cell<u32> = const { Cell::new(0) };
    static SCHED_PAUSES: Cell<u32> = const { Cell::new(0) };
    static RANDOM: Cell<u64> = const { Cell::new(0x9e37_79b9_7f4a_7c15) };
    static PAGES_OUT: Cell<i64> = const { Cell::new(0) };
    static MALLOC_OUT: RefCell<BTreeMap<i32, i64>> = const { RefCell::new(BTreeMap::new()) };
    static FAIL_PAGES: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
    static FAIL_POOL: Cell<Option<(&'static str, u32)>> = const { Cell::new(None) };
    static NET_HELD: Cell<bool> = const { Cell::new(false) };
    static PLEDGE_REFUSE: Cell<Option<(i32, Errno, u64)>> = const { Cell::new(None) };
    static TTY_VNODE: Cell<bool> = const { Cell::new(false) };

    static KM_MAP_CALLS: Cell<u32> = const { Cell::new(0) };

}

/// The generation number of every wait channel a wakeup touched.
static QUEUES: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
/// Signalled by every wakeup.
static WAKE: Condvar = Condvar::new();

/// A double's state, a panicking test's poison ignored.
fn lock<T>(m: &'static Mutex<T>) -> MutexGuard<'static, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A zeroed, page-aligned, leaked block of at least `len` bytes (`None` for 0).
fn pages(len: usize) -> Option<NonNull<u8>> {
    if len == 0 {
        return None;
    }
    let n = len.div_ceil(TEST_PAGE);
    let block: &'static mut [Page] = Box::leak(vec![Page([0; TEST_PAGE]); n].into_boxed_slice());
    // Made from the whole slice, so the pointer may reach every byte of it.
    Some(NonNull::from(block).cast::<u8>())
}

/// The interrupt controller double: `hal::intr`'s default levels (each test thread's core),
/// and a `splassert` that fails the test.
struct TestIntr;

impl IntrHardware for TestIntr {
    fn splassert(&self, wantipl: i32, func: &str) {
        let at = hal::cpu::curcpu().cpl();
        assert!(at >= wantipl, "splassert: {func}: want {wantipl} have {at}");
    }
}

/// The sleep queue double.
struct TestSleep;

impl Sleep for TestSleep {
    fn curproc_word(&self) -> usize {
        PROC.with(|p| std::ptr::from_ref(p).addr() & !0xf) | 0x10
    }

    fn sleep_setup(&self, ident: *const (), _prio: i32, _wmesg: &'static str) {
        let q = lock(&QUEUES);
        let chan = ident.addr();
        let generation = q.get(&chan).copied().unwrap_or(0);
        PENDING.with(|p| p.set(Some((chan, generation))));
    }

    fn sleep_finish(&self, nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
        let Some((chan, generation)) = PENDING.with(Cell::take) else {
            return Ok(());
        };
        if !do_sleep {
            return Ok(());
        }
        let deadline = (nsecs != INFSLP).then(|| Instant::now() + Duration::from_nanos(nsecs));
        let mut q = lock(&QUEUES);
        loop {
            if q.get(&chan).copied().unwrap_or(0) != generation {
                return Ok(());
            }
            match deadline {
                None => {
                    q = WAKE
                        .wait(q)
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                }
                Some(d) => {
                    let now = Instant::now();
                    if now >= d {
                        return Err(Errno::EWOULDBLOCK);
                    }
                    q = WAKE
                        .wait_timeout(q, d - now)
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .0;
                }
            }
        }
    }

    fn tsleep_nsec(
        &self,
        ident: *const (),
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        self.sleep_setup(ident, prio, wmesg);
        self.sleep_finish(nsecs, true)
    }

    fn wakeup(&self, ident: *const ()) {
        let mut q = lock(&QUEUES);
        *q.entry(ident.addr()).or_insert(0) += 1;
        WAKE.notify_all();
    }

    fn wakeup_one(&self, ident: *const ()) {
        // Waking every sleeper is a correct wakeup_one for callers that retry.
        self.wakeup(ident);
    }

    fn kernel_lock_held(&self) -> bool {
        false
    }

    /// This thread's `cold || panicstr`, as [`set_cold`] put it.
    fn cold(&self) -> bool {
        COLD.with(Cell::get)
    }

    /// Counts the early return's yields of this thread ([`cold_yields`]).
    fn cold_yield(&self) {
        COLD_YIELDS.with(|c| c.set(c.get() + 1));
    }

    /// Counts this thread's pauses ([`sched_pauses`]); never asked to yield.
    fn sched_pause(&self) {
        SCHED_PAUSES.with(|c| c.set(c.get() + 1));
    }
}

/// The `arc4random(9)` double: a per-thread xorshift64* sequence ([`set_random_seed`]).
struct TestRandom;

impl hal::random::Random for TestRandom {
    fn random_u32(&self) -> u32 {
        RANDOM.with(|s| {
            let mut x = s.get();
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            s.set(x);
            (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
        })
    }

    fn enqueue_randomness(&self, _val: u32) {
        ENQUEUED.with(|n| n.set(n.get() + 1));
    }
}

static TEST_RANDOM: TestRandom = TestRandom;

std::thread_local! {
    /// The words this thread gave `enqueue_randomness`.
    static ENQUEUED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// How many words this thread gave `enqueue_randomness` (the terminal's input characters).
pub fn randomness_enqueued() -> u64 {
    ENQUEUED.with(core::cell::Cell::get)
}

/// The kernel map double.
struct TestPages;

// The registration contract: a zeroed, page-aligned block of `len` bytes that only the caller
// references (leaked, so it is never handed out again).
impl PageMemory for TestPages {
    fn alloc(&self, len: usize, _kind: PageKind, _wait: bool) -> Option<(NonNull<u8>, usize)> {
        if page_fails() {
            return None;
        }
        let p = pages(len)?;
        PAGES_OUT.with(|c| c.set(c.get() + len as i64));
        Some((p, len))
    }

    fn free(&self, _ptr: NonNull<u8>, len: usize, _kind: PageKind) {
        PAGES_OUT.with(|c| c.set(c.get() - len as i64));
    }

    fn page_size(&self) -> usize {
        TEST_PAGE
    }
}

/// One pool of the pool double.
struct TestPool {
    size: usize,
    nout: AtomicU64,
    /// The pool's name (`wchan`), the key of [`POOL_KNOBS`].
    wchan: &'static str,
}

/// Each pool's last high-water mark and whether its cache was asked for, by the pool's name.
static POOL_KNOBS: Mutex<BTreeMap<&'static str, (u32, bool)>> = Mutex::new(BTreeMap::new());

// The registration contract: a fresh block of the pool's size, page-aligned (which covers any
// item alignment up to a page; `create` refuses more), the caller's alone.
impl RawPool for TestPool {
    fn get(&self, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        if pool_fails(self.wchan) {
            return None;
        }
        let p = pages(self.size)?;
        self.nout.fetch_add(1, Ordering::Relaxed);
        Some((p, self.size))
    }

    fn put(&self, _item: NonNull<u8>) {
        self.nout.fetch_sub(1, Ordering::Relaxed);
    }

    fn nout(&self) -> u64 {
        self.nout.load(Ordering::Relaxed)
    }

    fn size(&self) -> usize {
        self.size
    }

    /// The double keeps no idle item, so it records the bound for [`pool_knobs`].
    fn set_hiwat(&self, n: u32) {
        lock(&POOL_KNOBS).entry(self.wchan).or_insert((0, false)).0 = n;
    }

    /// The double has no per-CPU cache to make: it records the request for [`pool_knobs`].
    fn cache_init(&self) {
        lock(&POOL_KNOBS).entry(self.wchan).or_insert((0, false)).1 = true;
    }
}

/// The `malloc(9)` double.
struct TestMalloc;

// The registration contract: a zeroed, page-aligned (so `MALLOC_ALIGN`-aligned) block of at
// least `size` bytes that only the caller references (leaked, never handed out again); the
// bytes out counted per thread and type.
impl Malloc for TestMalloc {
    fn malloc(&self, size: usize, ty: i32, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        if page_fails() {
            return None;
        }
        let p = pages(size.max(1))?;
        MALLOC_OUT.with(|m| *m.borrow_mut().entry(ty).or_insert(0) += size as i64);
        Some((p, size))
    }

    fn free(&self, _ptr: NonNull<u8>, ty: i32, size: usize) {
        MALLOC_OUT.with(|m| *m.borrow_mut().entry(ty).or_insert(0) -= size as i64);
    }
}

/// The pool allocator double.
struct TestPools;

impl PoolBackend for TestPools {
    fn create(
        &self,
        size: usize,
        align: usize,
        _ipl: i32,
        _flags: i32,
        wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        if align > TEST_PAGE {
            return None;
        }
        lock(&POOL_KNOBS).entry(wchan).or_insert((0, false));
        Some(Box::leak(Box::new(TestPool {
            size: size.max(1),
            nout: AtomicU64::new(0),
            wchan,
        })))
    }
}

/// How many times this thread holds the kernel lock (`hal::klock`'s host double, a
/// per-thread depth).
pub fn kernel_lock_depth() -> u32 {
    hal::klock::depth()
}

/// The network lock double.
struct TestNetLock;

impl NetLock for TestNetLock {
    fn write_held(&self) -> bool {
        NET_HELD.with(Cell::get)
    }

    fn unlock(&self) {
        NET_HELD.with(|h| h.set(false));
    }

    fn lock(&self) {
        NET_HELD.with(|h| h.set(true));
    }
}

/// Says whether this thread holds the network lock double for writing.
pub fn net_lock_set(held: bool) {
    NET_HELD.with(|h| h.set(held));
}

/// Whether this thread holds the network lock double for writing.
pub fn net_lock_held() -> bool {
    NET_HELD.with(Cell::get)
}

static TEST_NET_LOCK: TestNetLock = TestNetLock;

/// The kernel thread double: a thread is a host thread that starts holding the kernel lock
/// double (as a kernel thread starts under the kernel lock); a deferred creation runs at once
/// in this crate's tests (the install runs `kthread_run_deferred_queue`) and waits, never run,
/// in `kernel::thread`'s queue in `bsd`'s (a queue destroyed then is in `TQ_S_CREATED` and makes no
/// thread), and `kthread_exit` unwinds the host thread with [`KthreadExit`], which its
/// spawner catches.
struct TestKthread;

/// What a test kernel thread's `kthread_exit` unwinds with.
struct KthreadExit;

impl TestKthread {
    /// `kthread_create` of a ticket: a host thread running `kernel::thread::run`.
    fn create(name: &'static [u8], ticket: usize) -> Result<(), Errno> {
        std::thread::Builder::new()
            .name(String::from_utf8_lossy(name).into_owned())
            .spawn(move || {
                hal::klock::kernel_lock();
                let r = std::panic::catch_unwind(move || {
                    crate::thread::run(ticket);
                });
                if let Err(e) = r
                    && !e.is::<KthreadExit>()
                {
                    std::panic::resume_unwind(e);
                }
            })
            .map(drop)
            .map_err(|_| Errno::EAGAIN)
    }

    /// `kthread_exit`: the host thread unwinds.
    fn exit(_ecode: i32) -> ! {
        std::panic::resume_unwind(Box::new(KthreadExit))
    }

    /// The peg to the primary CPU: nothing on the host.
    fn peg() {}
}

/// A page of a test u-area, page aligned.
#[repr(C, align(4096))]
struct TestPage([u8; 4096]);

/// The u-area double (`hal::thread::UAreaMemory`): zeroed, page-aligned heap pages of the
/// machine's `uspace`, leaked (a test's u-area is never given back to the heap; `free` keeps
/// nothing).
struct TestUAreas;

impl hal::thread::UAreaMemory for TestUAreas {
    fn alloc(&self) -> Option<core::num::NonZeroUsize> {
        let pages = TEST_VM_MACHINE.uspace.div_ceil(4096);
        let v: Vec<TestPage> = (0..pages).map(|_| TestPage([0; 4096])).collect();
        let b: &'static mut [TestPage] = Box::leak(v.into_boxed_slice());
        core::num::NonZeroUsize::new(b.as_ptr().expose_provenance())
    }

    fn free(&self, _va: core::num::NonZeroUsize) {}
}

static TEST_UAREAS: TestUAreas = TestUAreas;

/// What a [`MemVnode`] saw during its last read.
#[derive(Clone, Copy, Debug)]
pub struct ReadSeen {
    /// The object's lock was held (by anyone).
    pub object_locked: bool,
    /// This thread held the network lock double.
    pub net_locked: bool,
    /// This thread held the kernel lock double.
    pub kernel_locked: bool,
    /// The vnode was locked.
    pub vnode_locked: bool,
}

/// A vnode double for the vnode pager: its file is a `Vec` in memory.
pub struct MemVnode {
    file: Mutex<Vec<u8>>,
    uvn: Link<Uvn>,
    usecount: std::sync::atomic::AtomicI32,
    vtext: std::sync::atomic::AtomicBool,
    nlocked: std::sync::atomic::AtomicU32,
    fail: std::sync::atomic::AtomicBool,
    last_read: Mutex<Option<ReadSeen>>,
    /// Run by the next read inside the "VOP_READ", as another process might meanwhile.
    during_read: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    /// `v_type` (`VREG` unless a test sets another).
    vtype: Mutex<crate::vm::vnode::VType>,
}

impl MemVnode {
    /// Sets `v_type` (`vgone` makes it `VBAD`).
    pub fn set_vtype(&self, t: crate::vm::vnode::VType) {
        *lock_any(&self.vtype) = t;
    }

    /// `v_usecount`.
    pub fn usecount(&self) -> i32 {
        self.usecount.load(Ordering::Relaxed)
    }

    /// The file's bytes.
    pub fn contents(&self) -> Vec<u8> {
        lock_any(&self.file).clone()
    }

    /// How many times the vnode is locked.
    pub fn nlocked(&self) -> u32 {
        self.nlocked.load(Ordering::Relaxed)
    }

    /// Makes the reads and writes fail with `EIO` (or work again).
    pub fn fail_io(&self, fail: bool) {
        self.fail.store(fail, Ordering::Relaxed);
    }

    /// Sets `VTEXT`.
    pub fn set_vtext(&self) {
        self.vtext.store(true, Ordering::Relaxed);
    }

    /// What the last read saw.
    pub fn last_read(&self) -> Option<ReadSeen> {
        *lock_any(&self.last_read)
    }

    /// Runs `f` inside the next read, the pager's page busy and its locks let go.
    pub fn during_read(&self, f: impl FnOnce() + Send + 'static) {
        *lock_any(&self.during_read) = Some(Box::new(f));
    }
}

/// A `std` mutex of a double, poisoned or not.
fn lock_any<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl VnodeOps for MemVnode {
    fn vtype(&self) -> crate::vm::vnode::VType {
        *lock_any(&self.vtype)
    }

    fn is_zero_device(&self) -> bool {
        false
    }

    fn attr_flags(&'static self) -> Result<u64, Errno> {
        Ok(0)
    }

    fn uvn(&self) -> Option<&'static Uvn> {
        self.uvn.get()
    }

    fn set_uvn(&self, uvn: &'static Uvn) {
        self.uvn.set(Some(uvn));
    }

    fn attr_size(&'static self) -> Result<u64, Errno> {
        Ok(lock_any(&self.file).len() as u64)
    }

    fn is_blk(&self) -> bool {
        false
    }

    fn vtext(&self) -> bool {
        self.vtext.load(Ordering::Relaxed)
    }

    fn clear_vtext(&self) {
        self.vtext.store(false, Ordering::Relaxed);
    }

    fn mount_id(&self) -> usize {
        0x1000
    }

    fn vref(&'static self) {
        self.usecount.fetch_add(1, Ordering::Relaxed);
    }

    fn vrele(&'static self) {
        self.usecount.fetch_sub(1, Ordering::Relaxed);
    }

    fn vget_nowait(&'static self) -> Result<(), Errno> {
        self.vref();
        Ok(())
    }

    fn lock(&'static self, _flags: i32) -> Result<(), Errno> {
        self.nlocked.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn unlock(&'static self) {
        self.nlocked.fetch_sub(1, Ordering::Relaxed);
    }

    fn read(&'static self, off: Voff, buf: &mut [u8], _ioflag: i32) -> Result<usize, Errno> {
        *lock_any(&self.last_read) = Some(ReadSeen {
            object_locked: self
                .uvn
                .get()
                .is_some_and(|u| u.object().lock().status() != 0),
            net_locked: hal::netlock::net_lock_write_held(),
            kernel_locked: kernel_lock_depth() > 0,
            vnode_locked: self.nlocked() > 0,
        });
        let during = lock_any(&self.during_read).take();
        if let Some(f) = during {
            f();
        }
        if self.fail.load(Ordering::Relaxed) {
            return Err(Errno::EIO);
        }
        let data = lock_any(&self.file);
        let off = usize::try_from(off).unwrap_or(usize::MAX).min(data.len());
        let n = buf.len().min(data.len() - off);
        buf[..n].copy_from_slice(&data[off..off + n]);
        Ok(n)
    }

    fn write(&'static self, off: Voff, buf: &[u8], _ioflag: i32) -> Result<usize, Errno> {
        if self.fail.load(Ordering::Relaxed) {
            return Err(Errno::EIO);
        }
        let mut data = lock_any(&self.file);
        let off = usize::try_from(off).unwrap_or(0);
        if data.len() < off + buf.len() {
            data.resize(off + buf.len(), 0);
        }
        data[off..off + buf.len()].copy_from_slice(buf);
        Ok(buf.len())
    }
}

/// A vnode double over `contents`, leaked (vnodes are never freed).
pub fn mem_vnode(contents: Vec<u8>) -> &'static MemVnode {
    Box::leak(Box::new(MemVnode {
        file: Mutex::new(contents),
        uvn: Link::new(),
        usecount: std::sync::atomic::AtomicI32::new(1),
        vtext: std::sync::atomic::AtomicBool::new(false),
        nlocked: std::sync::atomic::AtomicU32::new(0),
        fail: std::sync::atomic::AtomicBool::new(false),
        last_read: Mutex::new(None),
        during_read: Mutex::new(None),
        vtype: Mutex::new(crate::vm::vnode::VType::VREG),
    }))
}
static TEST_INTR: TestIntr = TestIntr;
static TEST_SLEEP: TestSleep = TestSleep;
static TEST_PAGES: TestPages = TestPages;
static TEST_POOLS: TestPools = TestPools;
static TEST_MALLOC: TestMalloc = TestMalloc;

/// The `printf(9)` double: every line formatted into one buffer.
struct TestPrintf;

impl Printf for TestPrintf {
    fn printf(&self, args: core::fmt::Arguments<'_>) {
        let _ = core::fmt::Write::write_fmt(&mut *lock(&PRINTF_OUT), args);
    }

    /// `log(9)`: the line after its `<level>`, as the message buffer holds it.
    fn log(&self, level: i32, args: core::fmt::Arguments<'_>) {
        let mut out = lock(&PRINTF_OUT);
        let _ = core::fmt::Write::write_fmt(&mut *out, format_args!("<{level}>"));
        let _ = core::fmt::Write::write_fmt(&mut *out, args);
    }
}

/// What the `printf(9)` double was given.
static PRINTF_OUT: Mutex<String> = Mutex::new(String::new());

static TEST_PRINTF: TestPrintf = TestPrintf;

/// Whether [`install`] registered the pool and `malloc(9)` doubles (`None` before it ran).
static MEMORY_DOUBLES: OnceLock<bool> = OnceLock::new();

/// Whether the kernel's pools and `malloc(9)` are this module's heap doubles: then what they
/// hand out never comes from the memory a dependent crate's tests load and replace (`bsd`'s
/// `setup_real_memory`), so a process-wide kernel pool (`ucredpl`, `plimitpl`, `zombiepl`)
/// needs no reset between tests. `None` before [`install`]; `Some(false)` when another
/// backend held the slots first.
pub fn memory_is_the_doubles() -> Option<bool> {
    MEMORY_DOUBLES.get().copied()
}

/// Registers every double, once per process.
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = bridges::register_intr_hardware(&TEST_INTR);
        let _ = bridges::register_sleep(&TEST_SLEEP);
        let pools = bridges::register_pool_backend(&TEST_POOLS).is_ok();
        let malloc = bridges::register_malloc(&TEST_MALLOC).is_ok();
        let _ = MEMORY_DOUBLES.set(pools && malloc);
        let _ = bridges::register_printf(&TEST_PRINTF);
        let _ = bridges::register_net_lock(&TEST_NET_LOCK);
        let _ = bridges::register_random(&TEST_RANDOM);

        let _ = crate::vm::proc::install(&TEST_VM_PROC);

        // The clocks are kernel::tc's readers (N5.0): the dummy counter, a microsecond per
        // read at its nominal 1 MHz, warmed up as initclocks does.
        crate::tc::inittimecounter();
        crate::thread::install_test_ops(crate::thread::Ops {
            create: TestKthread::create,
            exit: TestKthread::exit,
            peg: TestKthread::peg,
        });
        let _ = hal::thread::register_uarea_memory(&TEST_UAREAS);
        // `bsd`'s host tests (this crate built with `doubles`, not `test`) drive their task
        // queues' workers by hand, as under LZ, where no deferred creation ever ran there.
        if cfg!(test) {
            crate::thread::kthread_run_deferred_queue();
        }
        crate::vm::init::uvm_machine_init(&TEST_VM_MACHINE);
    });
}

/// How many times the `printf(9)` double was given `s` (whole, by any test of the process).
pub fn printf_count(s: &str) -> usize {
    lock(&PRINTF_OUT).matches(s).count()
}

/// The kernel map double, for `KmBuf` and `kernel::pipe::Pipes::new` (a provider passed as a
/// value, not a registered bridge; a `const fn` so a test's `static` can name it).
pub const fn page_memory() -> &'static dyn PageMemory {
    &TEST_PAGES
}

/// `hal::rwlock::init_shared()` once per test process (`rwobjpl`): called from tests in
/// parallel, the one that lost the race could get `EBUSY` while the winner's
/// `OnceStatic::set` was still under way, and find no pool.
pub fn init_shared_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = hal::rwlock::init_shared();
    });
}

/// Makes this thread's page allocations fail, as `km_alloc` does when the kernel map is full:
/// after `skip` more that succeed, the next `n` fail (`fail_pages(0, 0)` clears it).
pub fn fail_pages(skip: u32, n: u32) {
    FAIL_PAGES.with(|f| f.set((skip, n)));
}

/// Makes this thread's next `n` items of the pool named `wchan` fail, as a pool with no page
/// left does (`fail_pool(wchan, 0)` clears it): N3.4d's fault handler, `amap_add` without a
/// chunk.
pub fn fail_pool(wchan: &'static str, n: u32) {
    FAIL_POOL.with(|f| f.set((n != 0).then_some((wchan, n))));
}

/// Whether this item of the pool named `wchan` is one [`fail_pool`] asked to fail (and count
/// it).
fn pool_fails(wchan: &str) -> bool {
    FAIL_POOL.with(|f| match f.get() {
        Some((name, n)) if name == wchan => {
            f.set((n > 1).then_some((name, n - 1)));
            true
        }
        _ => false,
    })
}

/// Whether this page allocation is one [`fail_pages`] asked to fail (and count it).
fn page_fails() -> bool {
    FAIL_PAGES.with(|f| match f.get() {
        (0, 0) => false,
        (0, n) => {
            f.set((0, n - 1));
            true
        }
        (skip, n) => {
            f.set((skip - 1, n));
            false
        }
    })
}

/// The bytes of `malloc(9)` blocks of type `ty` this thread holds, as the double counts them.
pub fn malloc_out(ty: i32) -> i64 {
    MALLOC_OUT.with(|m| m.borrow().get(&ty).copied().unwrap_or(0))
}

/// The last high-water mark `set_hiwat` gave the pool named `wchan` (0: never) and whether
/// its `cache_init` was asked for; `None` for a pool the double never made.
pub fn pool_knobs(wchan: &str) -> Option<(u32, bool)> {
    lock(&POOL_KNOBS).get(wchan).copied()
}

/// Makes this thread see `cold || panicstr` as `cold` says (the sleeps' early return).
pub fn set_cold(cold: bool) {
    COLD.with(|c| c.set(cold));
}

/// How many times this thread's sleeps took the `cold` early return's yield.
pub fn cold_yields() -> u32 {
    COLD_YIELDS.with(Cell::get)
}

/// How many times this thread called `hal::sleep::sched_pause`.
pub fn sched_pauses() -> u32 {
    SCHED_PAUSES.with(Cell::get)
}

/// Restarts this thread's `arc4random` double at `seed` (not 0).
pub fn set_random_seed(seed: u64) {
    RANDOM.with(|s| s.set(seed.max(1)));
}

/// The bytes of pages this thread holds, as the page double counts them.
pub fn pages_out() -> i64 {
    PAGES_OUT.with(Cell::get)
}

/// Where the test kernel maps lie: kernel-like addresses no host allocation has, below the
/// host's `VM_MIN_KERNEL_ADDRESS`, so `uvm_maxkaddr` (which the map tests set back to that
/// address for their own kernel-style maps, where the C only ever raises it) is always above
/// them: their free space is never split at it, and they never grow.
pub const KM_VA_BASE: usize = 0xffff_7000_0000_0000;
/// Their end.
pub const KM_VA_END: usize = KM_VA_BASE + (1 << 40);
/// Where `kmem_map` starts (`kernel_map` is below, `exec_map` above).
pub const KM_KMEM_BASE: usize = KM_VA_BASE + (1 << 39);
/// The one page of the test `exec_map`, below [`KM_VA_END`]'s last megabyte (a full map
/// after one page-sized allocation).
pub const KM_EXEC_BASE: usize = KM_VA_END - (1 << 20);

/// The kernel maps of the kernel's own tests, once per process: `kernel_map` over
/// `[KM_VA_BASE, KM_KMEM_BASE)` (pageable) and `kmem_map` over `[KM_KMEM_BASE,
/// KM_EXEC_BASE)` (intrsafe), both switched to best fit as `uvm_init_late` does, a one-page
/// `exec_map` at [`KM_EXEC_BASE`], and the kernel object (`uvm_km_init`'s, if no test made
/// it yet). Call it with [`MEMORY`] held (a test of the page system's set-up), as the kernel
/// object's other maker does.
pub fn kernel_maps_install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        use crate::vm::addr::uaddr_bestfit_create;
        use crate::vm::map::{
            UaddrSlot, VM_MAP_INTRSAFE, VM_MAP_PAGEABLE, uvm_map_create, uvm_map_init,
            uvm_map_set_uaddr,
        };
        init_shared_once();
        hal::pmap::init();
        uvm_map_init();
        let _ = crate::vm::aobj::uao_init();
        if crate::vm::aobj::kernel_object().is_none() {
            let _ = crate::vm::aobj::uao_create(
                hal::sys::types::Vsize::new(16 << 20),
                crate::vm::aobj::UAO_FLAG_KERNOBJ,
            );
        }
        let make = |min: usize, max: usize, flags: i32, bestfit: bool| {
            let map = uvm_map_create(hal::pmap::PmapRef::Kernel, min, max, flags);
            if bestfit {
                uvm_map_set_uaddr(map, UaddrSlot::Any(3), Some(uaddr_bestfit_create(min, max)));
            }
            map
        };
        let install = crate::vm::km::install_submap;
        install(
            KvMap::Kernel,
            make(KM_VA_BASE, KM_KMEM_BASE, VM_MAP_PAGEABLE, true),
        );
        let spl = hal::intr::splraise(hal::intr::IPL_VM);
        install(
            KvMap::Kmem,
            make(KM_KMEM_BASE, KM_EXEC_BASE, VM_MAP_INTRSAFE, true),
        );
        drop(spl);
        install(
            KvMap::Exec,
            make(
                KM_EXEC_BASE,
                KM_EXEC_BASE + TEST_PAGE,
                VM_MAP_PAGEABLE,
                false,
            ),
        );
    });
}

/// Counts one map call of `vm::km` (`uvm_map` of a kernel map) for this thread.
pub fn km_map_called() {
    KM_MAP_CALLS.with(|c| c.set(c.get() + 1));
}

/// How many maps this thread has asked the kernel maps for (`vm::km`'s map path).
pub fn km_map_calls() -> u32 {
    KM_MAP_CALLS.with(Cell::get)
}

/// Serialises the tests that use the page system (`vm::page`, `vm::pmemrange`: one set of
/// statics); [`pages_reset`], [`pages_setup`] and [`pages_install`] take it.
pub static MEMORY: Mutex<()> = Mutex::new(());

/// The host's segment policy (`VM_PHYSSEG_MAX`, `VM_PHYSSEG_STRAT`: amd64's).
pub const HOST_SEGS: SegPolicy = SegPolicy {
    max: 16,
    strat: SegStrat::Bigfirst,
};

/// The host's ISA range (16 MiB).
static ISA_CONSTRAINT: UvmConstraintRange = UvmConstraintRange {
    ucr_low: Paddr::new(0),
    ucr_high: Paddr::new(0x00ff_ffff),
};
/// The host's DMA range (4 GiB).
static DMA_CONSTRAINT: UvmConstraintRange = UvmConstraintRange {
    ucr_low: Paddr::new(0),
    ucr_high: Paddr::new(0xffff_ffff),
};

/// The host's `uvm_md_constraints[]` (the ISA and DMA ranges, as `bsd`'s host double has them).
pub static HOST_CONSTRAINTS: [&UvmConstraintRange; 2] = [&ISA_CONSTRAINT, &DMA_CONSTRAINT];

/// The lend of the kernel map's token the shared bootstrap gave.
static LEND: OnceLock<LegacyLend> = OnceLock::new();

/// The host pmap, bootstrapped once per process with the kernel's page provider
/// (`vm::page::PHYS_PAGES`, which reads the page system afresh on every call), its kernel map
/// token adopted by `vm::km` (the lend kept here): every test module of the crate shares it, a
/// second bootstrap panics. Registers the doubles first.
pub fn pmap_boot() -> &'static LegacyLend {
    LEND.get_or_init(|| {
        install();
        let b = hal::pmap::bootstrap(
            &BootstrapParams {
                direct_base: Vaddr::new(0),
                direct_end: Vaddr::new(usize::MAX),
                nx: false,
                first_avail: Paddr::new(0),
                image: None,
            },
            &PHYS_PAGES,
        );
        match crate::vm::km::adopt_mapper(b.mapper) {
            Ok(lend) => lend,
            Err(_) => {
                crate::assert::kpanic(format_args!("the kernel map's token was adopted already"))
            }
        }
    })
}

/// The page system forgotten (`uvm_page_test_reset`) and the page size set, [`MEMORY`] held:
/// segments may be loaded next.
pub fn pages_reset() -> MutexGuard<'static, ()> {
    let guard = lock(&MEMORY);
    let _ = pmap_boot();
    uvm_page_test_reset();
    UVMEXP.pagesize.store(TEST_PAGE as i32, Ordering::Relaxed);
    uvm_setpagesize();
    guard
}

/// The page system over `segs` (page frame ranges, every frame available) and `constraints`
/// (the machine's DMA ranges), [`MEMORY`] held. The frames need not be memory: the host's
/// `steal_memory` hands out heap blocks and its `pmap_zero_page` models no contents.
pub fn pages_setup(
    segs: &[(usize, usize)],
    constraints: &[&UvmConstraintRange],
) -> MutexGuard<'static, ()> {
    let guard = pages_reset();
    for &(start, end) in segs {
        if uvm_page_physload(start, end, start, end, 0, HOST_SEGS).is_err() {
            crate::assert::kpanic(format_args!("pages_setup: a segment was refused"));
        }
    }
    uvm_page_init(constraints);
    guard
}

/// The page system over a fresh, leaked block of `bytes` of the host's heap, page-aligned, as
/// one segment, no constraint: its frames are memory through the host pmap's identity direct
/// map, for the tests that write the pages they get (`vm::km`). [`MEMORY`] held.
pub fn pages_install(bytes: usize) -> MutexGuard<'static, ()> {
    let block: &'static mut [u8] = vec![0u8; bytes + TEST_PAGE].leak();
    let base = block
        .as_ptr()
        .expose_provenance()
        .next_multiple_of(TEST_PAGE);
    let (start, end) = (base / TEST_PAGE, (base + bytes) / TEST_PAGE);
    pages_setup(&[(start, end)], &[])
}

/// `RLIM_INFINITY`: what the `VmProc` double answers for a limit a test did not set.
pub const RLIM_INFINITY: u64 = (1 << 63) - 1;

/// What the `VmProc` double answers and records for the calling test thread (each test
/// thread a process): set it up with [`vmproc_with`].
#[derive(Default)]
pub struct VmProcState {
    /// `pledge_protexec` fails with `EPERM` for a `PROT_EXEC` request.
    pub pledge_denies_exec: bool,
    /// `uvm_wxcheck`'s exemption.
    pub wx_allowed: bool,
    /// `ps_wxcounter`.
    pub wxcounter: u32,
    /// `ps_pid`.
    pub pid: i32,
    /// `p_tid`.
    pub tid: i32,
    /// `ps_comm`.
    pub comm: Vec<u8>,
    /// `PROC_PC(p)`.
    pub pc: usize,
    /// `ps_acflag & AMAP`.
    pub acflag_amap: bool,
    /// `lim_cur(which)` by `which`; a missing one is [`RLIM_INFINITY`].
    pub limits: BTreeMap<i32, u64>,
    /// `P_HASSIBLING(p)`.
    pub sibling: bool,
    /// `ps_strings`.
    pub ps_strings: usize,
    /// `ps_sigcode`.
    pub ps_sigcode: usize,
    /// `ps_timekeep`.
    pub ps_timekeep: usize,
    /// The descriptors: number, vnode (`None`: not `DTYPE_VNODE`), `f_flag`.
    pub files: Vec<(i32, Option<&'static dyn VnodeOps>, u32)>,
    /// `ps_kbind_addr`/`ps_kbind_cookie`.
    pub kbind: crate::vm::proc::KbindSlot,
    /// `ps_libcpin` as `pin_table_set` left it: start, end, the table.
    pub libcpin: Option<(usize, usize, Vec<u32>)>,
    /// `ru_minflt`.
    pub minflt: u64,
    /// `ru_majflt`.
    pub majflt: u64,
    /// `ru_maxrss`.
    pub maxrss: usize,
    /// The pcs `trapsignal_segv` was given.
    pub segv: Vec<usize>,
    /// The `log` lines, with their level.
    pub logs: Vec<(i32, String)>,
    /// The `uprintf` lines.
    pub uprintfs: Vec<String>,
}

std::thread_local! {
    static VM_PROC_STATE: RefCell<VmProcState> = RefCell::new(VmProcState::default());
}

/// `f` on the calling thread's [`VmProcState`] (registers the double first).
pub fn vmproc_with<R>(f: impl FnOnce(&mut VmProcState) -> R) -> R {
    install();
    VM_PROC_STATE.with(|s| f(&mut s.borrow_mut()))
}

/// The `VmProc` double: answers from the calling thread's [`VmProcState`]; `sigexit` panics
/// with the signal's number (`sigexit(N)`), as nothing returns from it.
struct TestVmProc;

impl crate::vm::proc::VmProc for TestVmProc {
    fn pledge_protexec(&self, prot: crate::vm::types::VmProt) -> Result<(), Errno> {
        let denied = VM_PROC_STATE.with(|s| s.borrow().pledge_denies_exec);
        if denied && prot & hal::sys::mman::PROT_EXEC != 0 {
            return Err(Errno::EPERM);
        }
        Ok(())
    }

    fn wx_allowed(&self) -> bool {
        VM_PROC_STATE.with(|s| s.borrow().wx_allowed)
    }

    fn wxcounter_inc(&self) -> u32 {
        VM_PROC_STATE.with(|s| {
            let mut s = s.borrow_mut();
            let n = s.wxcounter;
            s.wxcounter = n + 1;
            n
        })
    }

    fn sigexit(&self, sig: i32) -> ! {
        crate::assert::kpanic(format_args!("sigexit({sig})"))
    }

    fn trapsignal_segv(&self, pc: usize) {
        VM_PROC_STATE.with(|s| s.borrow_mut().segv.push(pc));
    }

    fn pid(&self) -> i32 {
        VM_PROC_STATE.with(|s| s.borrow().pid)
    }

    fn tid(&self) -> i32 {
        VM_PROC_STATE.with(|s| s.borrow().tid)
    }

    fn comm(&self, out: &mut [u8]) -> usize {
        VM_PROC_STATE.with(|s| {
            let s = s.borrow();
            let n = s.comm.len().min(out.len());
            out[..n].copy_from_slice(&s.comm[..n]);
            n
        })
    }

    fn pc(&self) -> usize {
        VM_PROC_STATE.with(|s| s.borrow().pc)
    }

    fn set_acflag_amap(&self) {
        VM_PROC_STATE.with(|s| s.borrow_mut().acflag_amap = true);
    }

    fn lim_cur(&self, which: i32) -> u64 {
        VM_PROC_STATE
            .with(|s| s.borrow().limits.get(&which).copied())
            .unwrap_or(RLIM_INFINITY)
    }

    fn has_sibling(&self) -> bool {
        VM_PROC_STATE.with(|s| s.borrow().sibling)
    }

    fn ps_strings(&self) -> usize {
        VM_PROC_STATE.with(|s| s.borrow().ps_strings)
    }

    fn ps_sigcode(&self) -> usize {
        VM_PROC_STATE.with(|s| s.borrow().ps_sigcode)
    }

    fn ps_timekeep(&self) -> usize {
        VM_PROC_STATE.with(|s| s.borrow().ps_timekeep)
    }

    fn with_fd_vnode(
        &self,
        fd: i32,
        f: &mut dyn FnMut(Option<&'static dyn VnodeOps>, u32) -> Result<(), Errno>,
    ) -> Result<(), Errno> {
        let file = VM_PROC_STATE.with(|s| {
            s.borrow()
                .files
                .iter()
                .find(|(n, _, _)| *n == fd)
                .map(|&(_, vp, flag)| (vp, flag))
        });
        let (vp, flag) = file.ok_or(Errno::EBADF)?;
        f(vp, flag)
    }

    fn with_kbind(&self, f: &mut dyn FnMut(&mut crate::vm::proc::KbindSlot)) {
        let mut slot = VM_PROC_STATE.with(|s| s.borrow().kbind);
        f(&mut slot);
        VM_PROC_STATE.with(|s| s.borrow_mut().kbind = slot);
    }

    fn pin_table_set(&self, base: usize, end: usize, pins: hal::malloc::MArray<u32>) {
        let table = pins.to_vec();
        VM_PROC_STATE.with(|s| s.borrow_mut().libcpin = Some((base, end, table)));
    }

    fn ru_minflt_inc(&self) {
        VM_PROC_STATE.with(|s| s.borrow_mut().minflt += 1);
    }

    fn ru_majflt_inc(&self) {
        VM_PROC_STATE.with(|s| s.borrow_mut().majflt += 1);
    }

    fn ru_maxrss(&self, kb: usize) {
        VM_PROC_STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.maxrss = s.maxrss.max(kb);
        });
    }

    fn log(&self, level: i32, args: core::fmt::Arguments<'_>) {
        let line = std::format!("{args}");
        VM_PROC_STATE.with(|s| s.borrow_mut().logs.push((level, line)));
    }

    fn uprintf(&self, args: core::fmt::Arguments<'_>) {
        let line = std::format!("{args}");
        VM_PROC_STATE.with(|s| s.borrow_mut().uprintfs.push(line));
    }
}

static TEST_VM_PROC: TestVmProc = TestVmProc;

/// The machine's constants of `vm::init` for the host tests: amd64's numbers, no DMA
/// constraint.
static TEST_VM_MACHINE: crate::vm::init::VmMachine = crate::vm::init::VmMachine {
    ldpgsz: TEST_PAGE,
    uspace: 5 * TEST_PAGE,
    uspace_align: 0,
    have_uspace_guard: true,
    dma_constraint: &crate::vm::km::NO_CONSTRAINT,
};

/// The object of a test glue's file ([`test_file_glue!`](crate::test_file_glue)): an id
/// (`kinfo_file.f_data`'s address stand-in), what the vnode and socket hooks answer, and a
/// kqueue file's kqueue.
pub struct TestFileData<P: crate::proc::ProcGlue> {
    /// The object's number, its "address".
    pub id: usize,
    /// `VISTTY` of the vnode.
    pub tty: bool,
    /// `SS_DNS` of the socket.
    pub dns: bool,
    /// The vnode was revoked (`vnode_revoked`).
    pub revoked: core::sync::atomic::AtomicBool,
    /// A kqueue file's kqueue.
    pub kq: Option<crate::event::KqRef<P>>,
}

impl<P: crate::proc::ProcGlue> Default for TestFileData<P> {
    fn default() -> Self {
        Self {
            id: 0,
            tty: false,
            dns: false,
            revoked: core::sync::atomic::AtomicBool::new(false),
            kq: None,
        }
    }
}

/// The `struct stat` of a test glue.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TestStat {
    /// `st_size`.
    pub st_size: u64,
    /// `st_gen`.
    pub st_gen: u32,
    /// `st_mode`.
    pub st_mode: u32,
}

hal::abi_pod!(TestStat {
    st_size: u64,
    st_gen: u32,
    st_mode: u32
});

/// The vnode of a test glue's `fd_cdir`/`fd_rdir`: its use count.
#[derive(Debug, Default)]
pub struct TestVnode {
    /// `v_usecount`.
    pub refs: core::sync::atomic::AtomicI32,
}

std::thread_local! {
    static FILE_LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static FILE_LOG_ON: Cell<bool> = const { Cell::new(true) };
    static MAXFILES: Cell<i32> = const { Cell::new(1000) };
}

/// Whether this thread's test glues record their hot hooks (`knote_fdclose`): on for the
/// tests, off for a bench that measures the descriptors and not the record.
pub fn file_log_on() -> bool {
    FILE_LOG_ON.with(Cell::get)
}

/// Turns the record of [`file_log_on`] on or off for this thread.
pub fn set_file_log(on: bool) {
    FILE_LOG_ON.with(|c| c.set(on));
}

/// What this thread's test glues' `pledge_syscall` answers for `code`: the refusal
/// [`set_pledge_refusal`] armed for it, `Ok` for every other call.
pub fn pledge_refusal(code: i32) -> Result<(), (Errno, u64)> {
    match PLEDGE_REFUSE.with(Cell::get) {
        Some((c, e, tval)) if c == code => Err((e, tval)),
        _ => Ok(()),
    }
}

/// Makes this thread's test glues refuse system call `code` with `error` and the promises
/// `tval` (`None`: refuse nothing).
pub fn set_pledge_refusal(r: Option<(i32, Errno, u64)>) {
    PLEDGE_REFUSE.with(|c| c.set(r));
}

/// What this thread's test glues' `session_has_tty_vnode` answers (`s_ttyvp != NULL`):
/// `false` until [`set_tty_vnode`] says otherwise.
pub fn tty_vnode() -> bool {
    TTY_VNODE.with(Cell::get)
}

/// Makes this thread's test glues answer `on` for every session's terminal vnode.
pub fn set_tty_vnode(on: bool) {
    TTY_VNODE.with(|c| c.set(on));
}

/// Records a test glue's hook call.
pub fn file_log_push(s: String) {
    FILE_LOG.with(|l| l.borrow_mut().push(s));
}

/// The hook calls this thread's test glues recorded since the last call, in order.
pub fn file_log() -> Vec<String> {
    FILE_LOG.with(|l| core::mem::take(&mut *l.borrow_mut()))
}

/// `maxfiles` of this thread's test glues (1000 until set).
pub fn test_maxfiles() -> i32 {
    MAXFILES.with(Cell::get)
}

/// Sets `maxfiles` of this thread's test glues.
pub fn set_test_maxfiles(n: i32) {
    MAXFILES.with(|m| m.set(n));
}

/// Declares `$name`, a `kernel::file::FileGlue` of the tests over the process glue `$proc`:
/// per-thread tables (a leaked `FileTables` per test thread, so tests share nothing), the
/// operations a `dyn FileOps`, the object a [`TestFileData`], and every hook recorded in
/// [`file_log`]. `$crate::test_file_glue!(Files, Procs);` then `type FileGlue = Files;` in
/// the process glue.
#[macro_export]
macro_rules! test_file_glue {
    ($name:ident, $proc:ty) => {
        /// A file glue of the tests (`kernel::test_file_glue!`).
        pub struct $name;

        $crate::file_glue_links!($name);

        std::thread_local! {
            static FILE_TABLES: &'static $crate::file::FileTables<$name> =
                std::boxed::Box::leak(std::boxed::Box::new($crate::file::FileTables::new(
                    0,
                    std::boxed::Box::leak(std::boxed::Box::new(
                        core::sync::atomic::AtomicI32::new(0),
                    )),
                )));
        }

        /// The operations of a test glue's kqueue file (`kqueueops`).
        pub struct TestKqueueOps;

        impl $crate::file::FileOps<$name> for TestKqueueOps {
            fn read(
                &self,
                _fp: &$crate::file::File<$name>,
                _uio: &mut $crate::uio::Uio<'_, $proc>,
                _fflags: i32,
            ) -> Result<(), hal::sys::errno::Errno> {
                $crate::event::kqueue_rw()
            }
            fn write(
                &self,
                _fp: &$crate::file::File<$name>,
                _uio: &mut $crate::uio::Uio<'_, $proc>,
                _fflags: i32,
            ) -> Result<(), hal::sys::errno::Errno> {
                $crate::event::kqueue_rw()
            }
            fn ioctl(
                &self,
                _fp: &$crate::file::File<$name>,
                _com: u64,
                _data: &mut [u8],
                _p: &$crate::proc::Thread<$proc>,
            ) -> Result<(), hal::sys::errno::Errno> {
                $crate::event::kqueue_ioctl()
            }
            fn kqfilter(
                &self,
                fp: &$crate::file::File<$name>,
                kn: &$crate::event::Knote<$proc>,
            ) -> Result<(), hal::sys::errno::Errno> {
                $crate::event::kqueue_kqfilter::<$proc>(fp, kn)
            }
            fn stat(
                &self,
                fp: &$crate::file::File<$name>,
                st: &mut $crate::testutil::TestStat,
                _p: &$crate::proc::Thread<$proc>,
            ) -> Result<(), hal::sys::errno::Errno> {
                let (size, _blksize) = $crate::event::kqueue_stat::<$proc>(fp);
                *st = $crate::testutil::TestStat::default();
                st.st_size = size as u64;
                Ok(())
            }
            fn close(
                &self,
                fp: &$crate::file::File<$name>,
                p: Option<&$crate::proc::Thread<$proc>>,
            ) -> Result<(), hal::sys::errno::Errno> {
                $crate::event::kqueue_close::<$proc>(fp, p)
            }
        }

        static TEST_KQUEUE_OPS: TestKqueueOps = TestKqueueOps;

        impl $crate::file::FileGlue for $name {
            type Proc = $proc;
            type Ops = dyn $crate::file::FileOps<$name>;
            type Data = $crate::testutil::TestFileData<$proc>;
            type Stat = $crate::testutil::TestStat;
            type Vnode = $crate::testutil::TestVnode;
            type FileList = FileListLink;

            fn tables() -> &'static $crate::file::FileTables<Self> {
                FILE_TABLES.with(|t| *t)
            }
            fn maxfiles() -> i32 {
                $crate::testutil::test_maxfiles()
            }
            fn tablefull(tab: &str) {
                $crate::testutil::file_log_push(std::format!("tablefull {tab}"));
            }
            fn stall_wait() {
                $crate::testutil::file_log_push(std::string::String::from("stall"));
            }
            fn vref(v: &'static Self::Vnode) {
                v.refs.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            }
            fn vrele(v: &'static Self::Vnode) {
                v.refs.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
            }
            fn advlock(
                d: &Self::Data,
                owner: $crate::file::LockOwner,
                op: i32,
                fl: &mut hal::sys::fcntl::Flock,
                flags: i32,
            ) -> Result<(), hal::sys::errno::Errno> {
                $crate::testutil::file_log_push(std::format!(
                    "advlock {} {:?} {} {} {} {:#x}",
                    d.id,
                    owner,
                    op,
                    fl.l_type,
                    fl.l_start,
                    flags
                ));
                Ok(())
            }
            fn pathconf(
                _d: &Self::Data,
                name: i32,
            ) -> Result<hal::sys::types::Register, hal::sys::errno::Errno> {
                Ok(1000 + name as hal::sys::types::Register)
            }
            fn isatty(d: &Self::Data) -> bool {
                d.tty
            }
            fn position_check(
                _d: &Self::Data,
                off: hal::sys::types::Off,
            ) -> Result<(), hal::sys::errno::Errno> {
                if off < 0 {
                    Err(hal::sys::errno::Errno::EINVAL)
                } else {
                    Ok(())
                }
            }
            fn dns_socket(d: &Self::Data) -> bool {
                d.dns
            }
            fn kqueue_file(kq: $crate::event::KqRef<$proc>) -> (&'static Self::Ops, Self::Data) {
                (
                    &TEST_KQUEUE_OPS,
                    $crate::testutil::TestFileData {
                        id: kq.addr(),
                        kq: Some(kq),
                        ..$crate::testutil::TestFileData::default()
                    },
                )
            }
            fn kqueue_of(d: &Self::Data) -> Option<&$crate::event::KqRef<$proc>> {
                d.kq.as_ref()
            }
            fn vnode_revoked(d: &Self::Data) -> bool {
                d.revoked.load(core::sync::atomic::Ordering::Relaxed)
            }
            fn sigpipe(_p: &$crate::proc::Thread<$proc>) {
                $crate::testutil::file_log_push(std::string::String::from("SIGPIPE"));
            }
            fn pledge_fcntl(
                _p: &$crate::proc::Thread<$proc>,
                _cmd: i32,
            ) -> Result<(), hal::sys::errno::Errno> {
                Ok(())
            }
            fn pledge_flock(
                _p: &$crate::proc::Thread<$proc>,
            ) -> Result<(), hal::sys::errno::Errno> {
                Ok(())
            }
            fn pledge_ioctl(
                _p: &$crate::proc::Thread<$proc>,
                _com: u64,
                _fp: &$crate::file::File<Self>,
            ) -> Result<(), hal::sys::errno::Errno> {
                Ok(())
            }
            fn stat_hide_gen(st: &mut Self::Stat) {
                st.st_gen = 0;
            }
            fn stat_copyout(st: &Self::Stat, sb: usize) -> Result<(), hal::sys::errno::Errno> {
                hal::user::UserPtr::<$crate::testutil::TestStat>::new(sb).copyout(st)
            }
        }
    };
}

/// Declares `$name`, a `kernel::proc::ProcGlue` of the tests whose descriptors are the file
/// glue `$files` ([`test_file_glue!`](crate::test_file_glue)): no member of `bsd`'s, per-thread
/// tables, every hook a no-op. For the tests of the modules over the processes
/// (`kernel::descrip`), which need threads with tables and credentials. A last argument
/// `{ fn hook(..) { .. } ... }` adds or overrides members of the glue (`kernel::acct`'s
/// `session_ttydev`).
#[macro_export]
macro_rules! test_proc_glue {
    ($name:ident, $files:ty $(, { $($extra:tt)* })?) => {
        /// A process glue of the tests (`kernel::test_proc_glue!`).
        pub struct $name;

        $crate::proc_glue_links!($name);

        std::thread_local! {
            static PROC_TABLES: &'static $crate::proc::ProcTables<$name> =
                std::boxed::Box::leak(std::boxed::Box::new($crate::proc::ProcTables::new()));
            static CURPROC: core::cell::Cell<Option<&'static $crate::proc::Thread<$name>>> =
                const { core::cell::Cell::new(None) };
        }

        #[allow(dead_code, reason = "the tests that need a curproc call it")]
        impl $name {
            /// This test thread's `curproc`.
            pub fn set_curproc(p: Option<&'static $crate::proc::Thread<$name>>) {
                CURPROC.with(|c| c.set(p));
            }
        }

        impl $crate::proc::ProcGlue for $name {
            type ProcessG = ();
            type ThreadG = ();
            type PgrpG = ();
            type SessionG = ();
            type FileGlue = $files;
            $crate::proc_glue_link_types!();

            fn tables() -> &'static $crate::proc::ProcTables<Self> {
                PROC_TABLES.with(|t| *t)
            }
            fn sched() -> &'static $crate::sched::SchedTables<Self> {
                static SCHED: $crate::sched::SchedTables<$name> = $crate::sched::SchedTables::new();
                &SCHED
            }
            fn session_hangup(_sess: &$crate::proc::Session<Self>) {}
            fn session_placed(_sess: &$crate::proc::Session<Self>) {}
            fn session_destroy(_sess: &$crate::proc::Session<Self>) {}
            fn with_curproc<R>(f: impl FnOnce(Option<&$crate::proc::Thread<Self>>) -> R) -> R {
                f(CURPROC.with(core::cell::Cell::get))
            }
            fn exit1(_p: &$crate::proc::Thread<Self>, _xexit: i32, xsig: i32, _flags: i32) -> ! {
                std::panic!("exit1 of a test thread (signal {xsig})")
            }
            fn pledge_fail(
                _p: &$crate::proc::Thread<Self>,
                error: hal::sys::errno::Errno,
                code: u64,
            ) -> hal::sys::errno::Errno {
                $crate::testutil::file_log_push(std::format!("pledge_fail {code:#x}"));
                error
            }
            fn pledge_syscall(
                _p: &$crate::proc::Thread<Self>,
                code: i32,
            ) -> Result<(), (hal::sys::errno::Errno, u64)> {
                $crate::testutil::pledge_refusal(code)
            }
            fn pledge_sysctl(
                _p: &$crate::proc::Thread<Self>,
                _mib: &[i32],
                _new: usize,
            ) -> Result<(), hal::sys::errno::Errno> {
                Ok(())
            }
            fn session_has_tty_vnode(_sess: &$crate::proc::Session<Self>) -> bool {
                $crate::testutil::tty_vnode()
            }
            fn pledge_adjtime(
                _p: &$crate::proc::Thread<Self>,
                _delta: usize,
            ) -> Result<(), hal::sys::errno::Errno> {
                Ok(())
            }
            /// The test glue keeps no interval timers: nothing is set, the old value stays zero.
            fn setitimer(
                _which: i32,
                _itv: Option<&$crate::time::Itimerval>,
                _olditv: Option<&mut $crate::time::Itimerval>,
            ) {
            }
            /// No per-CPU slice start: the uptime itself (no running part).
            fn spc_runtime() -> $crate::clock::Timespec {
                $crate::tc::nanouptime()
            }
            fn process_ref(pr: &$crate::proc::Process<Self>) -> $crate::proc::ProcessRef<Self> {
                match hal::pool::PoolArc::from_ref(pr) {
                    Some(a) => $crate::proc::StaticOrPooled::Pooled(a),
                    None => std::panic!("process_ref: not a pooled process"),
                }
            }
            $($($extra)*)?
        }
    };
}
/* </CODE> */
