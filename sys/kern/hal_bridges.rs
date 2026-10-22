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
//! the code still here (the machine's `spl(9)` and `curcpu()`, the sleep queues,
//! `pool(9)`, `malloc(9)`, the user copy, the console, `printf(9)` (N3.3, the kernel crate's
//! lines), and for the pmap (N3.1) amd64's CPUs
//! of the TLB shootdown and `arc4random(9)`) arrives as one `&'static dyn Trait` per
//! bridge, registered once. This file is those implementations: each method is one call into
//! the legacy function of the same name and nothing else (a bridge carries no logic,
//! section 0.3.2), with the C's modes spelled out; `Errno` is one type on both sides
//! (`hal::sys::errno`, which `crate::sys::errno` re-exports).
//! It writes no `unsafe` (decision 30: the memory bridges are safe traits). One piece of
//! storage is its own: the `struct pool`s of the first pools `hal` makes (`pmap_init`'s, before
//! `kmeminit`, as the C's static pools) are a static array, handed out once each.
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
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use hal::cons::Console;
use hal::intr::IntrLevel;
use hal::klock::KernelLock;
use hal::malloc::Malloc;
use hal::netlock::NetLock;
use hal::pool::{PoolBackend, RawPool};
use hal::prf::Printf;
use hal::user::UserCopy;

use crate::dev::cons::{cngetc, cnpollc, cnputc};
use crate::kern::kern_kthread::{kthread_create, kthread_create_deferred, kthread_exit};
use crate::kern::kern_malloc::{free, malloc};
use crate::kern::kern_sched::sched_peg_curproc;
use crate::kern::kern_softintr::{SoftintrHand, softintr_establish, softintr_schedule};
use crate::kern::kern_subr::uiomove;
use crate::kern::subr_pool::{pool_cache_init, pool_get, pool_init, pool_put, pool_sethiwat};
use crate::kern::subr_prf::printf;
use crate::machine::Machine;
use crate::machine::copy::{copyin, copyinstr, copyout, copyoutstr};
use crate::machine::cpu::{Cpu, cpu_info_foreach};
use crate::machine::intr::{IPL_HIGH, IPL_MPFLOOR, splassert, splraise, splx};
use crate::sys::errno::Errno;
use crate::sys::pool::Pool;
use crate::sys::uio::{Iovec, Uio, UioRw, UioSeg};

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

    fn splassert(&self, wantipl: i32, func: &str) {
        splassert(wantipl, func);
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

    fn size(&self) -> usize {
        self.pr_size.get() as usize
    }

    fn set_hiwat(&self, n: u32) {
        pool_sethiwat(self, n);
    }

    fn cache_init(&self) {
        pool_cache_init(self);
    }
}

/// `pool_init(9)` of a new pool that lives for ever.
struct BsdPools;

/// How many `struct pool`s are kept in [`STATIC_POOLS`]: the pools `hal` and the kernel make
/// before `kmeminit` (`pmap_init`'s two on either machine: amd64's `pmappl` and `pvpl`, arm64's
/// `pmap` and `pted`; N3.4e: `rw_obj_init`'s `rwobjpl` and `uao_init`'s `aobjpl` and
/// `uaoeltpl`, which `uvm_km_init` runs for the kernel object; N3.5: `uvm_map_init`'s
/// `vmsppl`, `vmmpepl` and `vmmpekpl` and `uvm_addr_init`'s `uaddr`, `uaddrrnd`,
/// `uaddrbest` and `uaddrpivot`, the kernel's since the map moved: twelve), with room for
/// four more.
const NSTATIC_POOLS: usize = 16;

/// `struct pool`s that need no `malloc(9)`, for the first pools made: the C's `pmap_init`
/// runs before `kmeminit` (`uvm_init`) and calls `pool_init` on static `struct pool`s
/// (`pmap_pmap_pool`, `pmap_pv_pool`, `pmap_pted_pool`). Each is handed out once.
static STATIC_POOLS: [Pool; NSTATIC_POOLS] = [const { Pool::new() }; NSTATIC_POOLS];
/// How many of [`STATIC_POOLS`] were handed out.
static STATIC_POOLS_USED: AtomicUsize = AtomicUsize::new(0);
/// Whether `malloc(9)` serves the global allocator yet (`uvm_init` sets it after `kmeminit`):
/// a pool made before, past [`STATIC_POOLS`], is refused by name rather than by the
/// allocator's out-of-memory panic.
static HEAP_POOLS_READY: AtomicBool = AtomicBool::new(false);

/// `kmeminit` has run: the `struct pool`s past [`STATIC_POOLS`] may come from the heap.
pub fn hal_bridges_heap_ready() {
    HEAP_POOLS_READY.store(true, Ordering::Release);
}

/// The next `struct pool` of [`STATIC_POOLS`] no one has, if one is left.
fn static_pool() -> Option<&'static Pool> {
    let i = STATIC_POOLS_USED
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < NSTATIC_POOLS).then_some(n + 1)
        })
        .ok()?;
    STATIC_POOLS.get(i)
}

impl PoolBackend for BsdPools {
    /// The `struct pool` is one of [`STATIC_POOLS`] while one is left (so the pools made
    /// before `kmeminit` need no heap), then comes from the Rust global allocator
    /// (`malloc(9)` with `M_TEMP`, `M_NOWAIT` behind it; a failure there is the allocator's
    /// panic) and is leaked. Either way it lives for ever: `pool_init(9)` assumes its
    /// `struct pool` outlives every item, and no pool is ever destroyed here. The pool is the
    /// C's either way (`pool_init` with the caller's size, name and `ipl`), so `vmstat -m`
    /// and `kern.pool` show the same row.
    fn create(
        &self,
        size: usize,
        align: usize,
        ipl: i32,
        flags: i32,
        wchan: &'static str,
    ) -> Option<&'static dyn RawPool> {
        let align = u32::try_from(align).ok()?;
        let pp = match static_pool() {
            Some(pp) => pp,
            None if HEAP_POOLS_READY.load(Ordering::Acquire) => heap_pool()?,
            None => crate::kern::subr_prf::panic(format_args!(
                "pool_init {wchan}: more than NSTATIC_POOLS ({NSTATIC_POOLS}) pools before kmeminit"
            )),
        };
        pool_init(pp, size, align, ipl, flags, wchan, None);
        Some(pp)
    }
}

/// A new `struct pool` from the Rust global allocator, leaked.
#[cfg(feature = "alloc")]
fn heap_pool() -> Option<&'static Pool> {
    Some(alloc::boxed::Box::leak(alloc::boxed::Box::new(Pool::new())))
}

/// Without the global allocator there is nowhere to keep a new pool once [`STATIC_POOLS`]
/// are gone: none is made.
#[cfg(not(feature = "alloc"))]
fn heap_pool() -> Option<&'static Pool> {
    None
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

/// The user copy of `sys/machine/copy.rs`, and `kcopy` onto a `hal::user::ForeignWindow`
/// through `uiomove` (`sys/kern/kern_subr.rs`), whose `UIO_SYSSPACE` copy is the machine's.
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

    /// The window's bytes into `kbuf`: `uiomove` from a `UIO_SYSSPACE` iovec over the window,
    /// whose copy is the machine's `kcopy` (a fault is `EFAULT`, `pcb_onfault`).
    fn kcopy_in(&self, kva: usize, kbuf: &mut [u8]) -> Result<(), Errno> {
        sysspace_move(kva, kbuf, UioRw::UIO_WRITE)
    }

    /// `kbuf` into the window: `uiomove` to a `UIO_SYSSPACE` iovec over it, through a small
    /// buffer of the stack (`uiomove`'s kernel side is writable).
    fn kcopy_out(&self, kbuf: &[u8], kva: usize) -> Result<(), Errno> {
        let mut chunk = [0u8; KCOPY_CHUNK];
        let mut at = kva;
        for piece in kbuf.chunks(KCOPY_CHUNK) {
            let c = &mut chunk[..piece.len()];
            c.copy_from_slice(piece);
            sysspace_move(at, c, UioRw::UIO_READ)?;
            at = at.checked_add(piece.len()).ok_or(Errno::EFAULT)?;
        }
        Ok(())
    }
}

/// The stack buffer of [`BsdUser::kcopy_out`]'s copies (kbind's parameters fit in one).
const KCOPY_CHUNK: usize = 256;

/// `uiomove(buf, uio)` over one `UIO_SYSSPACE` iovec at the kernel address `kva`: `rw` says
/// which way (`UIO_READ`: `buf` to `kva`). The iovec's builder vouches for it (`struct uio`'s
/// contract): `hal` reaches this only for a range of a `ForeignWindow`.
fn sysspace_move(kva: usize, buf: &mut [u8], rw: UioRw) -> Result<(), Errno> {
    let len = buf.len();
    let mut iov = [Iovec {
        iov_base: ptr::with_exposed_provenance_mut::<c_void>(kva),
        iov_len: len,
    }];
    let mut uio = Uio {
        uio_iov: &mut iov,
        uio_offset: 0,
        uio_resid: len,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: rw,
        uio_procp: None,
    };
    uiomove(buf, &mut uio)
}

/// The CPUs as amd64's TLB shootdown and cache flush need them (`hal::pmap::TlbCpus`):
/// `CPU_INFO_FOREACH`, `cpu_is_running`, `ci_proc_pmap`, `x86_fast_ipi`,
/// `wbinvd_on_all_cpus` and `ci_cflushsz` of `sys/arch/amd64/amd64/{cpu,ipi}.rs`.
#[cfg(all(machine_x86, feature = "multiprocessor"))]
struct BsdTlbCpus;

#[cfg(all(machine_x86, feature = "multiprocessor"))]
impl hal::pmap::TlbCpus for BsdTlbCpus {
    fn for_each_other_running(&self, f: &mut dyn FnMut(usize, *const hal::pmap::Pmap)) {
        let me = crate::machine::cpu::curcpu();
        crate::machine::cpu::cpu_info_foreach(&mut |ci| {
            if !ptr::eq(ci, me) && crate::machine::cpu::cpu_is_running(ci) {
                f(
                    ci.ci_cpuid.get() as usize,
                    ci.ci_proc_pmap.load(Ordering::Relaxed).cast_const(),
                );
            }
        });
    }

    fn send(&self, cpuid: usize, ipi: hal::pmap::TlbIpi) -> bool {
        use crate::machine::x86::{LAPIC_IPI_INVLPG, LAPIC_IPI_INVLRANGE, LAPIC_IPI_INVLTLB};
        let vector = match ipi {
            hal::pmap::TlbIpi::Page => LAPIC_IPI_INVLPG,
            hal::pmap::TlbIpi::Range => LAPIC_IPI_INVLRANGE,
            hal::pmap::TlbIpi::Tlb => LAPIC_IPI_INVLTLB,
        };
        let mut sent = false;
        crate::machine::cpu::cpu_info_foreach(&mut |ci| {
            if ci.ci_cpuid.get() as usize == cpuid {
                sent = crate::machine::x86::x86_fast_ipi(ci, vector).is_ok();
            }
        });
        sent
    }

    fn wbinvd_all(&self) {
        let _ = crate::machine::x86::wbinvd_on_all_cpus();
    }

    fn cflushsz(&self) -> usize {
        crate::machine::cpu::curcpu().ci_cflushsz.get() as usize
    }
}

/// `arc4random(9)` of `sys/dev/rnd.rs` (`hal::pmap::Random`).
struct BsdRandom;

impl hal::pmap::Random for BsdRandom {
    fn random_u32(&self) -> u32 {
        crate::dev::rnd::arc4random()
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

/// `printf(9)` of `sys/kern/subr_prf.rs`.
struct BsdPrintf;

impl Printf for BsdPrintf {
    fn printf(&self, args: core::fmt::Arguments<'_>) {
        let _ = printf(args);
    }
}

/// `KERNEL_LOCK`, `KERNEL_UNLOCK`, `KERNEL_ASSERT_LOCKED` of `sys/sys/systm.rs`
/// (`kern_lock.rs`), for the kernel's objects and vnode pager (N3.4e).
struct BsdKernelLock;

impl KernelLock for BsdKernelLock {
    fn lock(&self) {
        crate::sys::systm::kernel_lock();
    }

    fn unlock(&self) {
        crate::sys::systm::kernel_unlock();
    }

    fn assert_locked(&self) {
        crate::sys::systm::kernel_assert_locked();
    }

    fn held(&self) -> bool {
        crate::kern::kern_lock::_kernel_lock_held()
    }

    fn release_all(&self) -> u32 {
        crate::kern::kern_lock::__mp_release_all(&crate::kern::kern_lock::KERNEL_LOCK)
    }

    fn acquire_count(&self, count: u32) {
        crate::kern::kern_lock::__mp_acquire_count(&crate::kern::kern_lock::KERNEL_LOCK, count);
    }
}

/// The soft interrupts (`sys/kern/kern_softintr.rs`). `softintr_establish` takes a
/// `fn(*mut c_void)` and an argument; the kernel's handlers are plain `fn()`s, so each takes
/// a slot of [`SOFT_RUNS`] and the argument is the slot's number, an address without
/// provenance that [`soft_run`] turns back into the number (never dereferenced); the handle
/// the bridge returns is the slot's number plus one, and the `SoftintrHand` it names is kept,
/// exposed, in [`SOFT_HANDS`].
struct BsdSoftIntr;

/// How many kernel soft interrupt handlers bsd serves (`softclock` alone today).
const SOFT_SLOTS: usize = 4;

/// The kernel's handlers, by slot.
static SOFT_RUNS: [hal::once::OnceStatic<fn()>; SOFT_SLOTS] =
    [const { hal::once::OnceStatic::new() }; SOFT_SLOTS];

/// The `SoftintrHand` of each slot, its address with its provenance exposed (0: none yet).
static SOFT_HANDS: [AtomicUsize; SOFT_SLOTS] = [const { AtomicUsize::new(0) }; SOFT_SLOTS];

/// The next slot to hand out.
static SOFT_NEXT: AtomicUsize = AtomicUsize::new(0);

/// The soft interrupt handler of every slot: runs the kernel's handler of slot `arg`.
fn soft_run(arg: *mut c_void) {
    if let Some(run) = SOFT_RUNS.get(arg.addr()).and_then(|s| s.get()) {
        run();
    }
}

impl hal::softintr::SoftIntr for BsdSoftIntr {
    fn establish(&self, ipl: i32, run: fn()) -> Option<usize> {
        let slot = SOFT_NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < SOFT_SLOTS).then_some(n + 1)
            })
            .ok()?;
        SOFT_RUNS[slot].set(run).ok()?;
        let sih = softintr_establish(ipl, soft_run, ptr::without_provenance_mut(slot))?;
        SOFT_HANDS[slot].store(sih.as_ptr().expose_provenance(), Ordering::Release);
        Some(slot + 1)
    }

    fn schedule(&self, handle: usize) {
        let Some(hand) = handle.checked_sub(1).and_then(|s| SOFT_HANDS.get(s)) else {
            return;
        };
        let p = ptr::with_exposed_provenance_mut::<SoftintrHand>(hand.load(Ordering::Acquire));
        if let Some(sih) = NonNull::new(p) {
            softintr_schedule(sih);
        }
    }
}

static BSD_SOFTINTR: BsdSoftIntr = BsdSoftIntr;

/// The kernel crate's uptime clock (`kernel::clock::Uptime`) over `nanouptime` of
/// `sys/kern/kern_tc.rs`, the userland-shaped `Timespec` turned into the kernel's.
struct BsdUptime;

impl kernel::clock::Uptime for BsdUptime {
    fn nanouptime(&self) -> kernel::clock::Timespec {
        let ts = crate::kern::kern_tc::nanouptime();
        kernel::clock::Timespec::new(ts.tv_sec, ts.tv_nsec)
    }
}

static BSD_UPTIME: BsdUptime = BsdUptime;

/// The kernel crate's threads (`kernel::thread::Kthread`) over `kthread_create`,
/// `kthread_create_deferred` and `kthread_exit` of `sys/kern/kern_kthread.rs`: the argument
/// the C passes is the kernel's ticket, an address without provenance that the thread function
/// turns back into the number (never dereferenced) and hands to `kernel::thread`.
struct BsdKthread;

/// A kernel thread's function: runs the start its ticket names.
fn kthread_start(arg: *mut c_void) {
    kernel::thread::run(arg.addr());
}

/// A deferred creation's function: calls the start its ticket names.
fn kthread_deferred(arg: *mut c_void) {
    kernel::thread::run_deferred(arg.addr());
}

impl kernel::thread::Kthread for BsdKthread {
    fn create(&self, name: &'static [u8], ticket: usize) -> Result<(), Errno> {
        kthread_create(kthread_start, ptr::without_provenance_mut(ticket), name).map(drop)
    }

    fn create_deferred(&self, ticket: usize) {
        kthread_create_deferred(kthread_deferred, ptr::without_provenance_mut(ticket));
    }

    fn exit(&self, ecode: i32) -> ! {
        kthread_exit(ecode)
    }

    /// `softclock_thread`'s "be conservative for the moment": the primary CPU, then
    /// `sched_peg_curproc`.
    fn peg_to_primary_cpu(&self) {
        let mut primary = None;
        cpu_info_foreach(&mut |ci| {
            if primary.is_none() && Machine::cpu_is_primary(ci) {
                primary = Some(ci);
            }
        });
        let Some(ci) = primary else {
            crate::kern::subr_prf::panic(format_args!("softclock_thread: no primary CPU"));
        };
        sched_peg_curproc(ci);
    }
}

static BSD_KTHREAD: BsdKthread = BsdKthread;

/// `NET_LOCK`, `NET_UNLOCK` and `rw_status(&netlock) == RW_WRITE` (`sys/net/if.rs`), for the
/// vnode pager's `uvn_io` (N3.4e).
struct BsdNetLock;

impl NetLock for BsdNetLock {
    fn write_held(&self) -> bool {
        crate::kern::kern_rwlock::rw_status(&crate::net::if_::NETLOCK)
            == crate::sys::rwlock::RW_WRITE
    }

    fn unlock(&self) {
        crate::sys::systm::net_unlock();
    }

    fn lock(&self) {
        crate::sys::systm::net_lock();
    }
}

static BSD_INTR: BsdIntr = BsdIntr;
/// The kernel's scheduler as `hal::sleep`'s sleeps (`kernel::sched::KernelSleep`).
static KERNEL_SLEEP: kernel::sched::KernelSleep<crate::kern::proc_glue::BsdProcGlue> =
    kernel::sched::KernelSleep::new();
static BSD_POOLS: BsdPools = BsdPools;
static BSD_MALLOC: BsdMalloc = BsdMalloc;
static BSD_USER: BsdUser = BsdUser;
static BSD_CONS: BsdCons = BsdCons;
static BSD_PRINTF: BsdPrintf = BsdPrintf;
#[cfg(all(machine_x86, feature = "multiprocessor"))]
static BSD_TLB_CPUS: BsdTlbCpus = BsdTlbCpus;
static BSD_RANDOM: BsdRandom = BsdRandom;
static BSD_KERNEL_LOCK: BsdKernelLock = BsdKernelLock;
static BSD_NET_LOCK: BsdNetLock = BsdNetLock;

/// Whether `hal`'s `TlbCpus` slot holds `bsd`'s CPUs (amd64's processors are started only
/// then: `Machine::cpu_boot_secondary_processors`).
static TLB_CPUS_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Registers every bridge of the `hal` runtime, once (a second call changes nothing: each slot
/// keeps its first implementation). Called by `stand::start_kernel` before `main()`: the boot
/// contract of the module docs.
pub fn hal_bridges_register() {
    let _ = hal::bridges::register_intr_level(&BSD_INTR);
    let _ = hal::bridges::register_sleep(&KERNEL_SLEEP);
    let _ = hal::bridges::register_pool_backend(&BSD_POOLS);
    let _ = hal::bridges::register_malloc(&BSD_MALLOC);
    let _ = hal::bridges::register_user_copy(&BSD_USER);
    let _ = hal::bridges::register_console(&BSD_CONS);
    let _ = hal::bridges::register_printf(&BSD_PRINTF);
    let _ = hal::bridges::register_random(&BSD_RANDOM);
    let _ = hal::bridges::register_softintr(&BSD_SOFTINTR);
    let _ = hal::thread::register_start(crate::kern::kern_fork::thread_start);
    // The kernel crate's own bridges (N4.1): slots only, used from `main()` on.
    let _ = kernel::clock::install_uptime(&BSD_UPTIME);
    let _ = kernel::thread::install_kthread(&BSD_KTHREAD);
    // amd64's shootdowns; a machine that broadcasts its TLB invalidations (arm64) has none.
    #[cfg(all(machine_x86, feature = "multiprocessor"))]
    {
        let _ = hal::bridges::register_tlb_cpus(&BSD_TLB_CPUS);
        TLB_CPUS_REGISTERED.store(true, Ordering::Release);
    }
}

/// Registers the bridges of the kernel's VM objects and vnode pager (`KernelLock`, `NetLock`),
/// once: called by `uvm_init` before the first object is made (bridges.toml).
pub fn hal_bridges_register_vm() {
    let _ = hal::bridges::register_kernel_lock(&BSD_KERNEL_LOCK);
    let _ = hal::bridges::register_net_lock(&BSD_NET_LOCK);
}

/// Whether the TLB shootdown's CPUs are registered (always, once [`hal_bridges_register`]
/// ran, on a machine that has them), for the check before the application processors start.
pub fn tlb_cpus_registered() -> bool {
    TLB_CPUS_REGISTERED.load(Ordering::Acquire)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;

    #[test]
    fn the_static_pools_are_handed_out_once_each() {
        // Other tests may take some meanwhile: what this one gets is distinct, at most
        // NSTATIC_POOLS in all, and once they are gone none comes back.
        let mut got: Vec<&'static Pool> = Vec::new();
        while let Some(pp) = static_pool() {
            assert!(
                !got.iter().any(|q| ptr::eq(*q, pp)),
                "a pool handed out twice"
            );
            got.push(pp);
        }
        assert!(got.len() <= NSTATIC_POOLS);
        assert!(
            got.iter()
                .all(|pp| STATIC_POOLS.iter().any(|q| ptr::eq(q, *pp)))
        );
        assert!(static_pool().is_none());
        assert_eq!(STATIC_POOLS_USED.load(Ordering::Relaxed), NSTATIC_POOLS);
    }
}
/* </TESTS> */
