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
//!   fail, as a full kernel map does.
//! - `UserCopy`: one user address space of [`USER_LEN`] bytes at [`USER_BASE`]; anything
//!   outside it is `EFAULT`.
//! - `Printf`: every line formatted into one buffer the tests read ([`printf_count`]).
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
use std::cell::Cell;
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
use hal::kmbuf::{PageKind, PageMemory};
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

/// The page size of the doubles.
const TEST_PAGE: usize = 4096;

/// Where the user address space of the double starts.
pub const USER_BASE: usize = 0x1000_0000;
/// Its size.
pub const USER_LEN: usize = 64 * 1024;

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
    static PAGES_OUT: Cell<i64> = const { Cell::new(0) };
    static FAIL_PAGES: Cell<(u32, u32)> = const { Cell::new((0, 0)) };

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
}

// The registration contract: a fresh block of the pool's size, page-aligned (which covers any
// item alignment up to a page; `create` refuses more), the caller's alone.
impl RawPool for TestPool {
    fn get(&self, _flags: i32) -> Option<(NonNull<u8>, usize)> {
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
        _wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        if align > TEST_PAGE {
            return None;
        }
        Some(Box::leak(Box::new(TestPool {
            size: size.max(1),
            nout: AtomicU64::new(0),
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
}

static TEST_INTR: TestIntr = TestIntr;
static TEST_CPU: TestCpu = TestCpu;
static TEST_SLEEP: TestSleep = TestSleep;
static TEST_PAGES: TestPages = TestPages;
static TEST_POOLS: TestPools = TestPools;
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
        let _ = bridges::register_user_copy(&TEST_USER);
        let _ = bridges::register_printf(&TEST_PRINTF);
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

/// `len` bytes of the double's user memory at `uaddr`, `None` outside it.
pub fn user_bytes(uaddr: usize, len: usize) -> Option<Vec<u8>> {
    let r = user_range(uaddr, len).ok()?;
    Some(lock(&USER)[r].to_vec())
}

/// Makes this thread's page allocations fail, as `km_alloc` does when the kernel map is full:
/// after `skip` more that succeed, the next `n` fail (`fail_pages(0, 0)` clears it).
pub fn fail_pages(skip: u32, n: u32) {
    FAIL_PAGES.with(|f| f.set((skip, n)));
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
