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
//! `bsd`'s implementations of the `hal` runtime's bridges, and their registration.
//!
//! The runtime in `hal` (docs/ZERO_UNSAFE.md 0.1) names nothing of `bsd`: what it needs from
//! the code still here (the machine's `spl(9)` and `curcpu()`, the sleep queues, `uvm_km`,
//! `pool(9)`, `malloc(9)`, the user copy, the console) arrives as one `&'static dyn Trait` per
//! bridge, registered once. This file is those implementations: each method is one call into
//! the legacy function of the same name and nothing else (a bridge carries no logic,
//! section 0.3.2), with the C's modes spelled out; `Errno` is one type on both sides
//! (`hal::sys::errno`, which `crate::sys::errno` re-exports).
//! It writes no `unsafe` (decision 30: the memory bridges are safe traits).
//!
//! [`hal_bridges_register`] is called by `stand::start_kernel` (sys/stand/mod.rs), on the boot
//! CPU, after the machine's earliest setup and the console and just before `main()`: before
//! interrupts, before any other CPU runs, and before any user of a `hal` piece (the kernel
//! crate's subsystems are initialised from `main()`, after `uvm_init`, `kmeminit` and the
//! pools). That is the boot contract `hal`'s soundness rests on (`sys/hal/bridges.rs`,
//! `sys/hal/trust.rs`): the memory bridges are the kernel's own allocators, registered once.
//! Registering does not call them; a bridge is used only by a piece whose subsystem is up.

use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

use hal::cons::Console;
use hal::cpu::CpuId;
use hal::intr::IntrLevel;
use hal::kmbuf::{PageKind, PageMemory};
use hal::malloc::Malloc;
use hal::pool::{PoolBackend, RawPool};
use hal::sleep::Sleep;
use hal::user::UserCopy;

use crate::dev::cons::{cngetc, cnpollc, cnputc};
use crate::kern::init_main::{DB_ACTIVE, NCPUSFOUND};
use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_synch::{sleep_finish, sleep_setup, tsleep_nsec, wakeup, wakeup_one};
use crate::kern::subr_pool::{pool_get, pool_init, pool_put};
use crate::kern::subr_prf::panicstr;
use crate::machine::Machine;
use crate::machine::copy::{copyin, copyinstr, copyout, copyoutstr};
use crate::machine::cpu::{Cpu, cpu_number, curcpu, curproc};
use crate::machine::intr::{IPL_HIGH, IPL_MPFLOOR, splraise, splx};
use crate::sys::errno::Errno;
use crate::sys::mutex::mtx_curcpu;
use crate::sys::param::PAGE_SIZE;
use crate::sys::pool::Pool;
use crate::uvm::uvm_km::{KD_NOWAIT, KD_WAITOK, KP_PAGEABLE, KP_ZERO, KV_ANY, km_alloc, km_free};

/// `spl(9)` of `sys/machine/intr.rs`.
struct BsdIntr;

impl IntrLevel for BsdIntr {
    fn splraise(&self, ipl: i32) -> i32 {
        splraise(ipl)
    }

    fn splx(&self, s: i32) {
        splx(s);
    }

    fn mpfloor(&self) -> i32 {
        IPL_MPFLOOR
    }

    fn ipl_high(&self) -> i32 {
        IPL_HIGH
    }
}

/// The running CPU of `sys/machine/cpu.rs`.
struct BsdCpu;

impl CpuId for BsdCpu {
    fn cpu_number(&self) -> usize {
        cpu_number() as usize
    }

    fn curcpu_word(&self) -> usize {
        mtx_curcpu()
    }

    fn spinning(&self, delta: isize) {
        let spc = Machine::ci_schedstate(curcpu());
        if delta >= 0 {
            spc.spc_spinning
                .fetch_add(delta.unsigned_abs() as u32, Ordering::Relaxed);
        } else {
            spc.spc_spinning
                .fetch_sub(delta.unsigned_abs() as u32, Ordering::Relaxed);
        }
    }

    fn past_caring(&self) -> bool {
        panicstr() || DB_ACTIVE.load(Ordering::Relaxed)
    }

    fn mutex_level(&self, delta: i32) {
        Machine::curcpu_mutex_level_add(delta);
    }

    fn ncpus(&self) -> usize {
        usize::try_from(NCPUSFOUND.load(Ordering::Relaxed)).unwrap_or(1)
    }
}

/// The sleep queues of `sys/kern/kern_synch.rs`.
struct BsdSleep;

impl Sleep for BsdSleep {
    fn curproc_word(&self) -> usize {
        curproc().map_or(0, |p| ptr::from_ref(p).addr())
    }

    fn sleep_setup(&self, ident: *const (), prio: i32, wmesg: &'static str) {
        sleep_setup(ident.cast::<c_void>(), prio, wmesg);
    }

    fn sleep_finish(&self, nsecs: u64, do_sleep: bool) -> Result<(), Errno> {
        sleep_finish(nsecs, do_sleep)
    }

    fn tsleep_nsec(
        &self,
        ident: *const (),
        prio: i32,
        wmesg: &'static str,
        nsecs: u64,
    ) -> Result<(), Errno> {
        tsleep_nsec(ident, prio, wmesg, nsecs)
    }

    fn wakeup(&self, ident: *const ()) {
        wakeup(ident);
    }

    fn wakeup_one(&self, ident: *const ()) {
        wakeup_one(ident);
    }

    fn kernel_lock_held(&self) -> bool {
        #[cfg(feature = "multiprocessor")]
        let held = crate::kern::kern_lock::_kernel_lock_held();
        #[cfg(not(feature = "multiprocessor"))]
        let held = false;
        held
    }
}

/// `km_alloc`/`km_free` of `sys/uvm/uvm_km.rs`, with `&kv_any`.
struct BsdPages;

impl PageMemory for BsdPages {
    fn alloc(&self, len: usize, kind: PageKind, wait: bool) -> Option<(NonNull<u8>, usize)> {
        let kp = match kind {
            PageKind::Pageable => &KP_PAGEABLE,
            PageKind::Wired => &KP_ZERO,
        };
        let kd = if wait { &KD_WAITOK } else { &KD_NOWAIT };
        km_alloc(len, &KV_ANY, kp, kd).map(|p| (p, len))
    }

    fn free(&self, ptr: NonNull<u8>, len: usize, kind: PageKind) {
        let kp = match kind {
            PageKind::Pageable => &KP_PAGEABLE,
            PageKind::Wired => &KP_ZERO,
        };
        km_free(ptr, len, &KV_ANY, kp);
    }

    fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

/// One pool of `pool(9)` (`sys/kern/subr_pool.rs`).
impl RawPool for Pool {
    fn get(&self, flags: i32) -> Option<(NonNull<u8>, usize)> {
        pool_get(self, flags).map(|p| (p, self.pr_size.get() as usize))
    }

    fn put(&self, item: NonNull<u8>) {
        pool_put(self, item);
    }

    fn nout(&self) -> u64 {
        u64::from(self.pr_nout.get())
    }
}

/// `pool_init(9)` of a new pool that lives for ever.
struct BsdPools;

impl PoolBackend for BsdPools {
    #[cfg(feature = "alloc")]
    fn create(
        &self,
        size: usize,
        align: usize,
        ipl: i32,
        flags: i32,
        wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        let align = u32::try_from(align).ok()?;
        let pp: &'static Pool = alloc::boxed::Box::leak(alloc::boxed::Box::new(Pool::new()));
        pool_init(pp, size, align, ipl, flags, wchan, None);
        Some(pp)
    }

    /// Without the global allocator there is nowhere to keep a new pool: none is made.
    #[cfg(not(feature = "alloc"))]
    fn create(
        &self,
        _size: usize,
        _align: usize,
        _ipl: i32,
        _flags: i32,
        _wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        None
    }
}

/// `malloc(9)`/`free(9)` of `sys/kern/kern_malloc.rs`.
struct BsdMalloc;

impl Malloc for BsdMalloc {
    fn malloc(&self, size: usize, ty: i32, flags: i32) -> Option<(NonNull<u8>, usize)> {
        malloc(size, ty, flags).map(|p| (p, size))
    }

    fn free(&self, ptr: NonNull<u8>, ty: i32, size: usize) {
        free(ptr, ty, size);
    }
}

/// The user copy of `sys/machine/copy.rs`.
struct BsdUser;

impl UserCopy for BsdUser {
    fn copyin(&self, uaddr: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
        copyin(uaddr, kbuf)
    }

    fn copyout(&self, kbuf: &[u8], uaddr: usize) -> Result<(), Errno> {
        copyout(kbuf, uaddr)
    }

    fn copyinstr(&self, uaddr: usize, kbuf: &mut [u8]) -> Result<usize, Errno> {
        copyinstr(uaddr, kbuf)
    }

    fn copyoutstr(&self, kbuf: &[u8], uaddr: usize) -> Result<usize, Errno> {
        copyoutstr(kbuf, uaddr)
    }
}

/// The console of `sys/dev/cons.rs`.
struct BsdCons;

impl Console for BsdCons {
    fn cnputc(&self, c: u8) {
        cnputc(i32::from(c));
    }

    fn cngetc(&self) -> i32 {
        cngetc()
    }

    fn cnpollc(&self, on: bool) {
        cnpollc(on);
    }
}

static BSD_INTR: BsdIntr = BsdIntr;
static BSD_CPU: BsdCpu = BsdCpu;
static BSD_SLEEP: BsdSleep = BsdSleep;
static BSD_PAGES: BsdPages = BsdPages;
static BSD_POOLS: BsdPools = BsdPools;
static BSD_MALLOC: BsdMalloc = BsdMalloc;
static BSD_USER: BsdUser = BsdUser;
static BSD_CONS: BsdCons = BsdCons;

/// Registers every bridge of the `hal` runtime, once (a second call changes nothing: each slot
/// keeps its first implementation). Called by `stand::start_kernel` before `main()`: the boot
/// contract of the module docs.
pub fn hal_bridges_register() {
    let _ = hal::bridges::register_intr_level(&BSD_INTR);
    let _ = hal::bridges::register_cpu_id(&BSD_CPU);
    let _ = hal::bridges::register_sleep(&BSD_SLEEP);
    let _ = hal::bridges::register_page_memory(&BSD_PAGES);
    let _ = hal::bridges::register_pool_backend(&BSD_POOLS);
    let _ = hal::bridges::register_malloc(&BSD_MALLOC);
    let _ = hal::bridges::register_user_copy(&BSD_USER);
    let _ = hal::bridges::register_console(&BSD_CONS);
}
/* </CODE> */
