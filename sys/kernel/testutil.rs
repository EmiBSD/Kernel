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
//! - `IntrLevel`: a per-thread level, as hal's double.
//! - `CpuId`: each test thread a CPU (its index from a counter, its word a per-thread
//!   static's address).
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
//! - `UserCopy`: one user address space of [`USER_LEN`] bytes at [`USER_BASE`]; anything
//!   outside it is `EFAULT`. Its `kcopy_in`/`kcopy_out` (a `hal::user::ForeignWindow`'s
//!   copies) reach one run of "foreign" kernel memory of [`FOREIGN_LEN`] bytes at
//!   [`FOREIGN_BASE`] ([`foreign_bytes`], [`set_foreign_bytes`]); anything outside it faults.
//! - `Printf`: every line formatted into one buffer the tests read ([`printf_count`]).
//! - `KernelLock`: a per-thread depth, no exclusion (the tests that take it hold [`MEMORY`]);
//!   the assertion panics on a thread that does not hold it ([`kernel_lock_depth`]).
//! - `NetLock`: a per-thread "held for writing" flag ([`net_lock_set`], [`net_lock_held`]).
//! - `vm::vnode`'s `SysInfo`: the host's monotonic clock from one second at the first call,
//!   never rebooting.
//! - `vm::vnode`'s `VnodeOps`, not a slot: [`MemVnode`], a vnode whose file is a `Vec` in
//!   memory ([`mem_vnode`] leaks one), which records what it saw during its last read.
//! - `vm::km`'s maps bridge, installed only by [`km_install`] (the kernel's own tests: `bsd`'s
//!   installs its real one): `KmMaps`, a first-fit allocator of virtual space per map over a
//!   `BTreeMap`, in a range no host address uses, whose next maps a thread can make fail
//!   ([`km_fail_maps`]). `km`'s pages are the kernel's page system over a heap block
//!   ([`pages_install`]).
//! - The page system itself (`vm::page`, `vm::pmemrange`), not a double: [`pmap_boot`]
//!   bootstraps the host pmap once per process with the kernel's provider
//!   (`vm::page::PHYS_PAGES`), and [`pages_setup`]/[`pages_install`] load fresh segments
//!   under [`MEMORY`], which every test of the page system holds (one set of statics).

use std::boxed::Box;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::string::String;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, Once, OnceLock};
use std::time::{Duration, Instant};
use std::vec;
use std::vec::Vec;

use hal::bridges;
use hal::cpu::CpuId;
use hal::intr::IntrLevel;
use hal::klock::KernelLock;
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
use hal::user::UserCopy;
use hal::vm::uvmexp::UVMEXP;

use crate::vm::km::{KmMaps, KmObject, KvMap, LegacyLend};
use crate::vm::page::{
    PHYS_PAGES, SegPolicy, SegStrat, uvm_page_init, uvm_page_physload, uvm_page_test_reset,
    uvm_setpagesize,
};
use crate::vm::types::{UvmConstraintRange, Voff};
use crate::vm::vnode::{SysInfo, Uvn, VnodeOps};

/// The page size of the doubles.
const TEST_PAGE: usize = 4096;

/// Where the user address space of the double starts.
pub const USER_BASE: usize = 0x1000_0000;
/// Its size.
pub const USER_LEN: usize = 64 * 1024;
/// Where the double's foreign kernel memory starts (a `ForeignWindow`'s side of `kcopy`).
pub const FOREIGN_BASE: usize = 0x3000_0000;
/// Its size.
pub const FOREIGN_LEN: usize = 64 * 1024;

/// One page of the doubles' memory: page-aligned, so a run of them is too.
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
struct Page([u8; TEST_PAGE]);

std::thread_local! {
    static SPL: Cell<i32> = const { Cell::new(0) };
    static WORD: u8 = const { 0 };
    static PROC: u8 = const { 0 };
    static CPU: usize = NEXT_CPU.fetch_add(1, Ordering::Relaxed);
    static PENDING: Cell<Option<(usize, u64)>> = const { Cell::new(None) };
    static COLD: Cell<bool> = const { Cell::new(false) };
    static COLD_YIELDS: Cell<u32> = const { Cell::new(0) };
    static PAGES_OUT: Cell<i64> = const { Cell::new(0) };
    static MALLOC_OUT: RefCell<BTreeMap<i32, i64>> = const { RefCell::new(BTreeMap::new()) };
    static FAIL_PAGES: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
    static FAIL_POOL: Cell<Option<(&'static str, u32)>> = const { Cell::new(None) };
    static KLOCK_DEPTH: Cell<u32> = const { Cell::new(0) };
    static NET_HELD: Cell<bool> = const { Cell::new(false) };

    static KM_FAIL_MAPS: Cell<u32> = const { Cell::new(0) };
    static KM_MAP_CALLS: Cell<u32> = const { Cell::new(0) };
    static KM_LAST_UNMAP: Cell<Option<(KvMap, usize, usize)>> = const { Cell::new(None) };

}

/// The index the next test thread gets.
static NEXT_CPU: AtomicUsize = AtomicUsize::new(0);

/// The generation number of every wait channel a wakeup touched.
static QUEUES: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
/// Signalled by every wakeup.
static WAKE: Condvar = Condvar::new();

/// The user address space of the double.
static USER: Mutex<[u8; USER_LEN]> = Mutex::new([0; USER_LEN]);
/// The double's foreign kernel memory.
static FOREIGN: Mutex<[u8; FOREIGN_LEN]> = Mutex::new([0; FOREIGN_LEN]);

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

/// The interrupt-level double.
struct TestIntr;

impl IntrLevel for TestIntr {
    fn splraise(&self, ipl: i32) -> i32 {
        SPL.with(|l| {
            let old = l.get();
            l.set(old.max(ipl));
            old
        })
    }

    fn splx(&self, s: i32) {
        SPL.with(|l| l.set(s));
    }

    fn mpfloor(&self) -> i32 {
        9
    }

    fn ipl_high(&self) -> i32 {
        0xd
    }

    fn splassert(&self, wantipl: i32, func: &str) {
        let at = SPL.with(Cell::get);
        assert!(at >= wantipl, "splassert: {func}: want {wantipl} have {at}");
    }
}

/// The CPU double.
struct TestCpu;

impl CpuId for TestCpu {
    fn cpu_number(&self) -> usize {
        CPU.with(|c| *c) % hal::cpu::MAXCPUS
    }

    fn curcpu_word(&self) -> usize {
        // Distinct per live thread, never 0, the locks' low flag bits clear.
        WORD.with(|w| std::ptr::from_ref(w).addr() & !0xf) | 0x10
    }

    fn spinning(&self, _delta: isize) {}

    fn past_caring(&self) -> bool {
        false
    }

    fn mutex_level(&self, _delta: i32) {}

    fn ncpus(&self) -> usize {
        hal::cpu::MAXCPUS
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

/// The user copy double.
struct TestUser;

/// The range `[uaddr, uaddr + len)` of the double's address space, `EFAULT` outside it.
fn user_range(uaddr: usize, len: usize) -> Result<core::ops::Range<usize>, Errno> {
    let start = uaddr.checked_sub(USER_BASE).ok_or(Errno::EFAULT)?;
    let end = start.checked_add(len).ok_or(Errno::EFAULT)?;
    if end > USER_LEN {
        return Err(Errno::EFAULT);
    }
    Ok(start..end)
}

impl UserCopy for TestUser {
    fn copyin(&self, uaddr: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
        let r = user_range(uaddr, kbuf.len())?;
        kbuf.copy_from_slice(&lock(&USER)[r]);
        Ok(())
    }

    fn copyout(&self, kbuf: &[u8], uaddr: usize) -> Result<(), Errno> {
        let r = user_range(uaddr, kbuf.len())?;
        lock(&USER)[r].copy_from_slice(kbuf);
        Ok(())
    }

    fn copyinstr(&self, _uaddr: usize, _kbuf: &mut [u8]) -> Result<usize, Errno> {
        Err(Errno::EFAULT)
    }

    fn copyoutstr(&self, _kbuf: &[u8], _uaddr: usize) -> Result<usize, Errno> {
        Err(Errno::EFAULT)
    }

    fn kcopy_in(&self, kva: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
        let r = foreign_range(kva, kbuf.len())?;
        kbuf.copy_from_slice(&lock(&FOREIGN)[r]);
        Ok(())
    }

    fn kcopy_out(&self, kbuf: &[u8], kva: usize) -> Result<(), Errno> {
        let r = foreign_range(kva, kbuf.len())?;
        lock(&FOREIGN)[r].copy_from_slice(kbuf);
        Ok(())
    }
}

/// The range `[kva, kva + len)` of the double's foreign memory, `EFAULT` outside it.
fn foreign_range(kva: usize, len: usize) -> Result<core::ops::Range<usize>, Errno> {
    let start = kva.checked_sub(FOREIGN_BASE).ok_or(Errno::EFAULT)?;
    let end = start.checked_add(len).ok_or(Errno::EFAULT)?;
    if end > FOREIGN_LEN {
        return Err(Errno::EFAULT);
    }
    Ok(start..end)
}

/// The kernel lock double.
struct TestKernelLock;

impl KernelLock for TestKernelLock {
    fn lock(&self) {
        KLOCK_DEPTH.with(|d| d.set(d.get() + 1));
    }

    fn unlock(&self) {
        KLOCK_DEPTH.with(|d| match d.get().checked_sub(1) {
            Some(n) => d.set(n),
            None => crate::assert::kpanic(format_args!("kernel_unlock: not locked")),
        });
    }

    fn assert_locked(&self) {
        if KLOCK_DEPTH.with(Cell::get) == 0 {
            crate::assert::kpanic(format_args!("kernel lock not held"));
        }
    }
}

/// How many times this thread holds the kernel lock double.
pub fn kernel_lock_depth() -> u32 {
    KLOCK_DEPTH.with(Cell::get)
}

static TEST_KERNEL_LOCK: TestKernelLock = TestKernelLock;

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

/// The clock and shutdown double of `vm::vnode`.
struct TestSysInfo;

/// When the clock double started.
static CLOCK_START: OnceLock<Instant> = OnceLock::new();

/// `rebooting`'s wait channel in the double.
static REBOOTING_CHAN: u8 = 0;

impl SysInfo for TestSysInfo {
    fn uptime(&self) -> (i64, i64) {
        let t = CLOCK_START.get_or_init(Instant::now).elapsed() + Duration::from_secs(1);
        (t.as_secs() as i64, i64::from(t.subsec_micros()))
    }

    fn rebooting(&self) -> bool {
        false
    }

    fn rebooting_chan(&self) -> *const () {
        std::ptr::from_ref(&REBOOTING_CHAN).cast()
    }
}

static TEST_SYS_INFO: TestSysInfo = TestSysInfo;

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
}

impl MemVnode {
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
    }))
}
static TEST_INTR: TestIntr = TestIntr;
static TEST_CPU: TestCpu = TestCpu;
static TEST_SLEEP: TestSleep = TestSleep;
static TEST_PAGES: TestPages = TestPages;
static TEST_POOLS: TestPools = TestPools;
static TEST_MALLOC: TestMalloc = TestMalloc;
static TEST_USER: TestUser = TestUser;

/// The `printf(9)` double: every line formatted into one buffer.
struct TestPrintf;

impl Printf for TestPrintf {
    fn printf(&self, args: core::fmt::Arguments<'_>) {
        let _ = core::fmt::Write::write_fmt(&mut *lock(&PRINTF_OUT), args);
    }
}

/// What the `printf(9)` double was given.
static PRINTF_OUT: Mutex<String> = Mutex::new(String::new());

static TEST_PRINTF: TestPrintf = TestPrintf;

/// Registers every double, once per process.
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = bridges::register_intr_level(&TEST_INTR);
        let _ = bridges::register_cpu_id(&TEST_CPU);
        let _ = bridges::register_sleep(&TEST_SLEEP);
        let _ = bridges::register_pool_backend(&TEST_POOLS);
        let _ = bridges::register_malloc(&TEST_MALLOC);
        let _ = bridges::register_user_copy(&TEST_USER);
        let _ = bridges::register_printf(&TEST_PRINTF);
        let _ = bridges::register_kernel_lock(&TEST_KERNEL_LOCK);
        let _ = bridges::register_net_lock(&TEST_NET_LOCK);
        let _ = crate::vm::vnode::install_sysinfo(&TEST_SYS_INFO);
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

/// `len` bytes of the double's user memory at `uaddr`, `None` outside it.
pub fn user_bytes(uaddr: usize, len: usize) -> Option<Vec<u8>> {
    let r = user_range(uaddr, len).ok()?;
    Some(lock(&USER)[r].to_vec())
}

/// `len` bytes of the double's foreign kernel memory at `kva`, `None` outside it.
pub fn foreign_bytes(kva: usize, len: usize) -> Option<Vec<u8>> {
    let r = foreign_range(kva, len).ok()?;
    Some(lock(&FOREIGN)[r].to_vec())
}

/// Writes `bytes` into the double's foreign kernel memory at `kva` (what the other process
/// left in its pages); `false` outside it.
pub fn set_foreign_bytes(kva: usize, bytes: &[u8]) -> bool {
    match foreign_range(kva, bytes.len()) {
        Ok(r) => {
            lock(&FOREIGN)[r].copy_from_slice(bytes);
            true
        }
        Err(_) => false,
    }
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

/// The bytes of pages this thread holds, as the page double counts them.
pub fn pages_out() -> i64 {
    PAGES_OUT.with(Cell::get)
}

/// Where the kernel maps double hands out virtual space: kernel-like addresses no host
/// allocation has.
pub const KM_VA_BASE: usize = 0xffff_9000_0000_0000;
/// Its end.
pub const KM_VA_END: usize = KM_VA_BASE + (1 << 40);

/// One map entry of the kernel maps double: its end, map and object.
type KmEntry = (usize, KvMap, KmObject);

/// The entries of the kernel maps double, by start.
static KM_ENTRIES: Mutex<BTreeMap<usize, KmEntry>> = Mutex::new(BTreeMap::new());

/// One wait channel per map.
static KM_CHANS: [u8; 5] = [0; 5];

/// The kernel maps double.
struct TestKmMaps;

impl KmMaps for TestKmMaps {
    fn map(
        &self,
        which: KvMap,
        sz: usize,
        object: KmObject,
        _prefer: Voff,
        align: usize,
        _mapflags: u32,
    ) -> Result<Vaddr, Errno> {
        KM_MAP_CALLS.with(|c| c.set(c.get() + 1));
        if KM_FAIL_MAPS.with(|f| {
            let n = f.get();
            f.set(n.saturating_sub(1));
            n != 0
        }) {
            return Err(Errno::ENOMEM);
        }
        let align = align.max(TEST_PAGE);
        let mut m = lock(&KM_ENTRIES);
        let mut va = KM_VA_BASE;
        for (&start, &(end, _, _)) in m.iter() {
            if va + sz <= start {
                break;
            }
            va = va.max(end.next_multiple_of(align));
        }
        if va + sz > KM_VA_END {
            return Err(Errno::ENOMEM);
        }
        m.insert(va, (va + sz, which, object));
        Ok(Vaddr::new(va))
    }

    fn unmap(&self, which: KvMap, sva: Vaddr, eva: Vaddr) {
        KM_LAST_UNMAP.with(|u| u.set(Some((which, sva.as_usize(), eva.as_usize()))));
        lock(&KM_ENTRIES).remove(&sva.as_usize());
    }

    fn chan(&self, which: KvMap) -> *const () {
        let i = match which {
            KvMap::Kernel => 0,
            KvMap::Kmem => 1,
            KvMap::Exec => 2,
            KvMap::Phys => 3,
            KvMap::None => 4,
        };
        std::ptr::from_ref(&KM_CHANS[i]).cast::<()>()
    }
}

static TEST_KM_MAPS: TestKmMaps = TestKmMaps;

/// `vm::km`'s maps double, as `install` takes it.
pub fn km_maps() -> &'static dyn KmMaps {
    &TEST_KM_MAPS
}

/// Installs `vm::km`'s maps double, once per process (the kernel's own tests only).
pub fn km_install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = crate::vm::km::install(km_maps());
    });
}

/// Makes this thread's next `n` maps fail (`ENOMEM`), as a full map does.
pub fn km_fail_maps(n: u32) {
    KM_FAIL_MAPS.with(|f| f.set(n));
}

/// How many maps this thread has asked the double for.
pub fn km_map_calls() -> u32 {
    KM_MAP_CALLS.with(Cell::get)
}

/// The map and object of the double's entry at `va`, if one starts there.
pub fn km_entry(va: Vaddr) -> Option<(KvMap, KmObject)> {
    lock(&KM_ENTRIES)
        .get(&va.as_usize())
        .map(|&(_, which, object)| (which, object))
}

/// The last range this thread gave back to the double (`which`, start, end).
pub fn km_last_unmap() -> Option<(KvMap, usize, usize)> {
    KM_LAST_UNMAP.with(Cell::get)
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
/* </CODE> */
