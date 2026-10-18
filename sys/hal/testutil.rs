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
//! The host tests' doubles of the bridges (`cfg(test)` only, declared so in `lib.rs`).
//!
//! The tests share one process and the registry is set once, so [`install`] registers every
//! double the first time any test calls it and does nothing afterwards; every test that
//! reaches a bridge calls it first. The doubles keep their state per test thread where a test
//! asserts on it (the interrupt level, the spin count), so tests running in parallel do not
//! see each other's.
//!
//! - `IntrLevel`: a per-thread level; `splraise` never lowers it, `splx` restores; the
//!   `IPL_MPFLOOR` of the host double of `bsd` (9).
//! - `CpuId`: each test thread is a CPU of its own (its index from a counter, its word the
//!   address of a per-thread static), so two test threads contend for a lock and update
//!   per-CPU counters as two CPUs do; `ncpusfound` is `MAXCPUS`.
//! - `Sleep`: each test thread is a process (its word the address of another per-thread
//!   static); a wait channel has a generation number that every wakeup bumps, `sleep_setup`
//!   records it and `sleep_finish` waits on one condition variable until it moves (so a wakeup
//!   between the two is never lost, as with the C's sleep queue lock) or the timeout
//!   (`EWOULDBLOCK`); no signals.
//! - `PageMemory` (not a bridge: [`page_memory`] hands it to `KmBuf` as a value): zeroed,
//!   page-aligned allocations of the host's allocator, the bytes out counted per thread.
//! - `PoolBackend`: each pool a host allocation layout and an atomic `pr_nout`; its
//!   `set_hiwat` and `cache_init` recorded by the pool's name ([`pool_knobs`]).
//! - `Malloc`: host blocks aligned to `MALLOC_ALIGN`, the bytes out counted per thread and
//!   malloc type.
//! - `UserCopy`: one user address space of `USER_LEN` bytes at [`user_base`]; anything
//!   outside it is `EFAULT`. Its `kcopy_in`/`kcopy_out` reach one run of "foreign" kernel
//!   memory of [`FOREIGN_LEN`] bytes at [`foreign_base`] (the pages a `ForeignWindow` would
//!   name); anything outside it faults (`EFAULT`).
//! - `Console`: the output kept in one buffer, the input a queue the tests feed.

use std::alloc::{Layout, alloc, alloc_zeroed, dealloc};
use std::boxed::Box;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ptr::NonNull;
use std::string::String;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, Once, OnceLock};
use std::time::{Duration, Instant};

use crate::bridges;
use crate::cons::Console;
use crate::cpu::CpuId;
use crate::intr::IntrLevel;
use crate::kmbuf::{PageKind, PageMemory};
use crate::malloc::{MALLOC_ALIGN, Malloc};
use crate::pmap::{BootstrapParams, Bootstrapped, PageMd, PhysPages};
use crate::pool::{PoolBackend, RawPool};
use crate::prf::Printf;
use crate::sleep::{INFSLP, Sleep};
use crate::sys::errno::Errno;
use crate::sys::types::{Paddr, Vaddr};
use crate::user::UserCopy;

std::thread_local! {
    static SPL: Cell<i32> = const { Cell::new(0) };
    static SPINNING: Cell<isize> = const { Cell::new(0) };
    static MUTEX_LEVEL: Cell<i32> = const { Cell::new(0) };
    static WORD: u8 = const { 0 };
    static CPU: usize = NEXT_CPU.fetch_add(1, Ordering::Relaxed);
    static PROC: u8 = const { 0 };
    static PENDING: Cell<Option<(usize, u64)>> = const { Cell::new(None) };
    static COLD: Cell<bool> = const { Cell::new(false) };
    static COLD_YIELDS: Cell<u32> = const { Cell::new(0) };
    static PAGES_OUT: Cell<isize> = const { Cell::new(0) };
    static MALLOC_OUT: RefCell<BTreeMap<i32, isize>> = const { RefCell::new(BTreeMap::new()) };
}

/// The `IPL_HIGH` of the interrupt-level double (the host double of `bsd`'s).
pub(crate) const IPL_HIGH: i32 = 0xd;

/// The page size of the doubles.
const TEST_PAGE: usize = 4096;

/// Where the user address space of the double starts.
const USER_BASE: usize = 0x1000_0000;
/// Its size.
const USER_LEN: usize = 64 * 1024;

/// The user address space of the double.
static USER: Mutex<[u8; USER_LEN]> = Mutex::new([0; USER_LEN]);
/// Where the double's foreign kernel memory starts (`kcopy`'s window side).
const FOREIGN_BASE: usize = 0x3000_0000;
/// Its size.
pub(crate) const FOREIGN_LEN: usize = 64 * 1024;
/// The double's foreign kernel memory.
static FOREIGN: Mutex<[u8; FOREIGN_LEN]> = Mutex::new([0; FOREIGN_LEN]);
/// What the console double was given.
static CONS_OUT: Mutex<std::vec::Vec<u8>> = Mutex::new(std::vec::Vec::new());
/// What the console double will answer.
static CONS_IN: Mutex<std::collections::VecDeque<u8>> =
    Mutex::new(std::collections::VecDeque::new());

/// The generation number of every wait channel a wakeup touched.
static QUEUES: Mutex<BTreeMap<usize, u64>> = Mutex::new(BTreeMap::new());
/// Each pool's high-water mark and whether its cache was made, by the pool's name.
static POOL_KNOBS: Mutex<BTreeMap<&'static str, (u32, bool)>> = Mutex::new(BTreeMap::new());
/// The memory frames [`set_idle`] made not busy.
static IDLE_FRAMES: Mutex<std::collections::BTreeSet<usize>> =
    Mutex::new(std::collections::BTreeSet::new());
/// The blocks of host memory that [`memory_frames`] handed out, the managed pages of
/// [`FAKE_PAGES`]: the first frame's address and a `PageMd` per frame.
static MEMORY_FRAMES: Mutex<std::vec::Vec<(usize, &'static [PageMd])>> =
    Mutex::new(std::vec::Vec::new());
/// Signalled by every wakeup.
static WAKE: Condvar = Condvar::new();

/// The index the next test thread gets.
static NEXT_CPU: AtomicUsize = AtomicUsize::new(0);

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
        IPL_HIGH
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
        CPU.with(|c| *c) % crate::cpu::MAXCPUS
    }

    fn curcpu_word(&self) -> usize {
        // The address of a per-thread static: distinct per live thread, never 0, and with
        // the two low bits clear (the locks' flag bits) once aligned down.
        WORD.with(|w| core::ptr::from_ref(w).addr() & !0xf) | 0x10
    }

    fn spinning(&self, delta: isize) {
        SPINNING.with(|s| s.set(s.get() + delta));
    }

    fn past_caring(&self) -> bool {
        false
    }

    fn mutex_level(&self, delta: i32) {
        MUTEX_LEVEL.with(|m| m.set(m.get() + delta));
    }

    fn ncpus(&self) -> usize {
        crate::cpu::MAXCPUS
    }
}

/// The sleep queue double.
struct TestSleep;

/// The queues, a panicking test's poison ignored (the map is consistent at every unlock).
fn queues() -> MutexGuard<'static, BTreeMap<usize, u64>> {
    QUEUES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Sleep for TestSleep {
    fn curproc_word(&self) -> usize {
        PROC.with(|p| core::ptr::from_ref(p).addr() & !0xf) | 0x10
    }

    fn sleep_setup(&self, ident: *const (), _prio: i32, _wmesg: &'static str) {
        let q = queues();
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
        let mut q = queues();
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
        let mut q = queues();
        *q.entry(ident.addr()).or_insert(0) += 1;
        WAKE.notify_all();
    }

    fn wakeup_one(&self, ident: *const ()) {
        // Waking every sleeper is a correct wakeup_one for the callers: they all retry.
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

// The registration contract: the host allocator's zeroed, page-aligned block of `len` bytes,
// owned by the caller alone until `free` gives it back with the same layout.
impl PageMemory for TestPages {
    fn alloc(&self, len: usize, _kind: PageKind, _wait: bool) -> Option<(NonNull<u8>, usize)> {
        if len == 0 {
            return None;
        }
        let layout = Layout::from_size_align(len, TEST_PAGE).ok()?;
        // SAFETY: the layout's size is not 0.
        let p = NonNull::new(unsafe { alloc_zeroed(layout) })?;
        PAGES_OUT.with(|c| c.set(c.get() + len as isize));
        Some((p, len))
    }

    fn free(&self, ptr: NonNull<u8>, len: usize, _kind: PageKind) {
        if let Ok(layout) = Layout::from_size_align(len, TEST_PAGE) {
            // SAFETY: `ptr` came from `alloc` with this very layout (the trait's contract on
            // the caller, `KmBuf`), once.
            unsafe { dealloc(ptr.as_ptr(), layout) };
        }
        PAGES_OUT.with(|c| c.set(c.get() - len as isize));
    }

    fn page_size(&self) -> usize {
        TEST_PAGE
    }
}

/// One pool of the pool double.
struct TestPool {
    layout: Layout,
    nout: AtomicU64,
    /// The pool's name, the key of [`POOL_KNOBS`].
    wchan: &'static str,
}

// The registration contract: the host allocator's block of the pool's layout (the size and
// alignment it was created with), owned by the caller until `put` gives it back.
impl RawPool for TestPool {
    fn get(&self, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        // SAFETY: the layout's size is not 0 (`create` makes it at least 1).
        let p = NonNull::new(unsafe { alloc(self.layout) })?;
        self.nout.fetch_add(1, Ordering::Relaxed);
        Some((p, self.layout.size()))
    }

    fn put(&self, item: NonNull<u8>) {
        // SAFETY: `item` came from `get` with this pool's layout (the trait's contract on the
        // caller, `PoolArc`), once.
        unsafe { dealloc(item.as_ptr(), self.layout) };
        self.nout.fetch_sub(1, Ordering::Relaxed);
    }

    fn nout(&self) -> u64 {
        self.nout.load(Ordering::Relaxed)
    }

    /// The double keeps no idle item: it records the bound for [`pool_knobs`].
    fn set_hiwat(&self, n: u32) {
        lock(&POOL_KNOBS).entry(self.wchan).or_insert((0, false)).0 = n;
    }

    /// The double has no per-CPU cache to make: it records the request for [`pool_knobs`].
    fn cache_init(&self) {
        lock(&POOL_KNOBS).entry(self.wchan).or_insert((0, false)).1 = true;
    }
}

/// The pool allocator double.
struct TestPools;

// Every pool `create` returns is a `TestPool` of the size and alignment asked for, leaked so
// it lives for ever.
impl PoolBackend for TestPools {
    fn create(
        &self,
        size: usize,
        align: usize,
        _ipl: i32,
        _flags: i32,
        wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        let layout = Layout::from_size_align(size.max(1), align).ok()?;
        lock(&POOL_KNOBS).entry(wchan).or_insert((0, false));
        Some(Box::leak(Box::new(TestPool {
            layout,
            nout: AtomicU64::new(0),
            wchan,
        })))
    }
}

/// The `malloc(9)` double.
struct TestMalloc;

// The registration contract: the host allocator's block of `size` bytes aligned to
// `MALLOC_ALIGN` (one byte for `malloc(0)`, which the C serves from its smallest bucket),
// owned by the caller until `free` gives it back with the same size.
impl Malloc for TestMalloc {
    fn malloc(&self, size: usize, ty: i32, _flags: i32) -> Option<(NonNull<u8>, usize)> {
        let layout = Layout::from_size_align(size.max(1), MALLOC_ALIGN).ok()?;
        // SAFETY: the layout's size is not 0.
        let p = NonNull::new(unsafe { alloc(layout) })?;
        MALLOC_OUT.with(|m| *m.borrow_mut().entry(ty).or_insert(0) += size as isize);
        Some((p, size))
    }

    fn free(&self, ptr: NonNull<u8>, ty: i32, size: usize) {
        if let Ok(layout) = Layout::from_size_align(size.max(1), MALLOC_ALIGN) {
            // SAFETY: `ptr` came from `malloc` with this very layout (the trait's contract on
            // the caller, `MBox`), once.
            unsafe { dealloc(ptr.as_ptr(), layout) };
        }
        MALLOC_OUT.with(|m| *m.borrow_mut().entry(ty).or_insert(0) -= size as isize);
    }
}

/// The user copy double.
struct TestUser;

/// The range `[kva, kva + len)` of the double's foreign memory, `EFAULT` outside it.
fn foreign_range(kva: usize, len: usize) -> Result<core::ops::Range<usize>, Errno> {
    let start = kva.checked_sub(FOREIGN_BASE).ok_or(Errno::EFAULT)?;
    let end = start.checked_add(len).ok_or(Errno::EFAULT)?;
    if end > FOREIGN_LEN {
        return Err(Errno::EFAULT);
    }
    Ok(start..end)
}

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

    fn copyinstr(&self, uaddr: usize, kbuf: &mut [u8]) -> Result<usize, Errno> {
        let start = user_range(uaddr, 0)?.start;
        let user = lock(&USER);
        for (i, k) in kbuf.iter_mut().enumerate() {
            let c = *user.get(start + i).ok_or(Errno::EFAULT)?;
            *k = c;
            if c == 0 {
                return Ok(i + 1);
            }
        }
        Err(Errno::ENAMETOOLONG)
    }

    fn copyoutstr(&self, kbuf: &[u8], uaddr: usize) -> Result<usize, Errno> {
        let n = kbuf
            .iter()
            .position(|&c| c == 0)
            .map(|i| i + 1)
            .ok_or(Errno::ENAMETOOLONG)?;
        self.copyout(&kbuf[..n], uaddr)?;
        Ok(n)
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

/// The console double.
struct TestCons;

impl Console for TestCons {
    fn cnputc(&self, c: u8) {
        lock(&CONS_OUT).push(c);
    }

    fn cngetc(&self) -> i32 {
        lock(&CONS_IN).pop_front().map_or(0, i32::from)
    }

    fn cnpollc(&self, _on: bool) {}
}

/// The `printf(9)` double: every line formatted into one buffer.
struct TestPrintf;

impl Printf for TestPrintf {
    fn printf(&self, args: core::fmt::Arguments<'_>) {
        let _ = core::fmt::Write::write_fmt(&mut *lock(&PRINTF_OUT), args);
    }
}

/// What the `printf(9)` double was given.
static PRINTF_OUT: Mutex<String> = Mutex::new(String::new());

/// A double's state, a panicking test's poison ignored.
fn lock<T>(m: &'static Mutex<T>) -> MutexGuard<'static, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

static TEST_INTR: TestIntr = TestIntr;
static TEST_CPU: TestCpu = TestCpu;
static TEST_SLEEP: TestSleep = TestSleep;
static TEST_PAGES: TestPages = TestPages;
static TEST_POOLS: TestPools = TestPools;
static TEST_MALLOC: TestMalloc = TestMalloc;
static TEST_USER: TestUser = TestUser;
static TEST_CONS: TestCons = TestCons;
static TEST_PRINTF: TestPrintf = TestPrintf;

/// Registers every double, once per process.
pub(crate) fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = bridges::register_intr_level(&TEST_INTR);
        let _ = bridges::register_cpu_id(&TEST_CPU);
        let _ = bridges::register_sleep(&TEST_SLEEP);
        let _ = bridges::register_pool_backend(&TEST_POOLS);
        let _ = bridges::register_malloc(&TEST_MALLOC);
        let _ = bridges::register_user_copy(&TEST_USER);
        let _ = bridges::register_console(&TEST_CONS);
        let _ = bridges::register_printf(&TEST_PRINTF);
    });
}

/// The kernel map double, for `KmBuf`'s constructors (a provider passed as a value, not a
/// registered bridge).
pub(crate) fn page_memory() -> &'static dyn PageMemory {
    &TEST_PAGES
}

/// The first address of the double's user address space.
pub(crate) fn user_base() -> usize {
    USER_BASE
}

/// The first address of the double's foreign kernel memory.
pub(crate) fn foreign_base() -> usize {
    FOREIGN_BASE
}

/// `len` bytes of the double's foreign memory from `off`.
pub(crate) fn foreign_bytes(off: usize, len: usize) -> std::vec::Vec<u8> {
    lock(&FOREIGN)[off..off + len].to_vec()
}

/// Whether the `printf(9)` double was given `s`, in one piece.
pub(crate) fn printf_saw(s: &str) -> bool {
    lock(&PRINTF_OUT).contains(s)
}

/// Whether the console double was given `s`, in one piece.
pub(crate) fn console_saw(s: &[u8]) -> bool {
    lock(&CONS_OUT).windows(s.len()).any(|w| w == s)
}

/// Queues `c` for the console double's `cngetc`.
pub(crate) fn console_feed(c: u8) {
    lock(&CONS_IN).push_back(c);
}

/// The bytes of `malloc(9)` blocks of type `ty` this thread holds, as the double counts them.
pub(crate) fn malloc_out(ty: i32) -> isize {
    MALLOC_OUT.with(|m| m.borrow().get(&ty).copied().unwrap_or(0))
}

/// The bytes of kernel pages this thread holds, as the double counts them.
pub(crate) fn pages_out() -> isize {
    PAGES_OUT.with(Cell::get)
}

/// This thread's interrupt level, as the double keeps it.
pub(crate) fn spl_level() -> i32 {
    SPL.with(Cell::get)
}

/// The last high-water mark `set_hiwat` gave the pool named `wchan` (0: never) and whether its
/// `cache_init` was asked for; `None` for a pool the double never made.
pub(crate) fn pool_knobs(wchan: &str) -> Option<(u32, bool)> {
    lock(&POOL_KNOBS).get(wchan).copied()
}

/// `n` frames of host memory of this test's own, physically contiguous and page-aligned (a
/// fresh leaked, zeroed block), that [`FAKE_PAGES`] knows as managed pages from now on: the
/// frames a byte view of mapped pages may be made over (`mapped.rs`).
pub(crate) fn memory_frames(n: usize) -> std::vec::Vec<Paddr> {
    let block: &'static mut [u8] = std::vec![0u8; (n + 1) * TEST_PAGE].leak();
    let base = block
        .as_mut_ptr()
        .expose_provenance()
        .next_multiple_of(TEST_PAGE);
    let mds: &'static [PageMd] = (0..n)
        .map(|_| PageMd::new())
        .collect::<std::vec::Vec<_>>()
        .leak();
    lock(&MEMORY_FRAMES).push((base, mds));
    (0..n).map(|i| Paddr::new(base + i * TEST_PAGE)).collect()
}

/// Makes this thread see `cold || panicstr` as `cold` says (the sleeps' early return).
pub(crate) fn set_cold(cold: bool) {
    COLD.with(|c| c.set(cold));
}

/// How many times this thread's sleeps took the `cold` early return's yield.
pub(crate) fn cold_yields() -> u32 {
    COLD_YIELDS.with(Cell::get)
}

/// Makes the memory frame `pa` a page that is not busy (`PG_BUSY` clear), as a page of a
/// pool or of `km_alloc` is.
pub(crate) fn set_idle(pa: Paddr) {
    lock(&IDLE_FRAMES).insert(pa.as_usize());
}

/// This thread's spin count, as the double keeps it.
pub(crate) fn spinning() -> isize {
    SPINNING.with(Cell::get)
}

/// The page provider of the host pmap in `hal`'s tests: frame numbers from a counter, the
/// page system never up (so `steal_memory` serves `bootmem`'s tests). Its managed pages are
/// the frames of host memory [`memory_frames`] handed out (the host pmap's direct map is the
/// identity, so they are memory there), each with a `PageMd` of its own; every other frame is
/// no managed page.
/// `page_init_done` is false: N3.1's provider of the same name in `pmap.rs`'s tests answered
/// true, which `steal_memory` refuses; the host backend never reads it (only the machines'
/// page-table allocation does), so the pmap tests see no difference.
pub(crate) struct FakePages {
    next: AtomicUsize,
}

impl PhysPages for FakePages {
    fn alloc_ptp(&self) -> Option<Paddr> {
        Some(Paddr::new(
            self.next.fetch_add(TEST_PAGE, Ordering::Relaxed),
        ))
    }
    fn free_ptp(&self, _pa: Paddr) {}
    fn md_of(&self, pa: Paddr) -> Option<&'static PageMd> {
        let pa = pa.as_usize();
        lock(&MEMORY_FRAMES).iter().find_map(|&(start, mds)| {
            pa.checked_sub(start)
                .map(|off| off / TEST_PAGE)
                .and_then(|i| mds.get(i))
        })
    }
    /// The memory frames are their test's busy pages, but those [`set_idle`] named.
    fn is_busy(&self, pa: Paddr) -> bool {
        self.md_of(pa).is_some() && !lock(&IDLE_FRAMES).contains(&pa.as_usize())
    }
    fn steal(&self, npg: usize) -> Option<Paddr> {
        Some(Paddr::new(
            self.next.fetch_add(npg * TEST_PAGE, Ordering::Relaxed),
        ))
    }
    fn page_init_done(&self) -> bool {
        false
    }
}

/// The one provider of the test process's host pmap.
pub(crate) static FAKE_PAGES: FakePages = FakePages {
    next: AtomicUsize::new(0x10_0000),
};

/// The bridges' doubles, the host pmap's bootstrap over [`FAKE_PAGES`] and `pmap_init`, once
/// per test process (a second `bootstrap` panics, so every test module shares this one).
pub(crate) fn pmap_boot() -> &'static Bootstrapped {
    static BOOTED: OnceLock<Bootstrapped> = OnceLock::new();
    BOOTED.get_or_init(|| {
        install();
        let params = BootstrapParams {
            direct_base: Vaddr::new(0),
            direct_end: Vaddr::new(0),
            nx: true,
            first_avail: Paddr::new(0x20_0000),
            image: None,
        };
        let b = crate::pmap::bootstrap(&params, &FAKE_PAGES);
        assert_eq!(b.first_avail, Paddr::new(0x20_0000));
        crate::pmap::init();
        b
    })
}
/* </CODE> */
