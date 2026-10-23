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

/*
 * Copyright (c) 2016 Dale Rahn <drahn@dalerahn.com>
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
//! The running CPU: the members of `struct cpu_info` the runtime, the context switch and the
//! assembly reach ([`CpuCore`]), the running CPU's core (`curcpu()`), its index (for
//! [`crate::percpu::PerCpu`]), a word that names it (the owner of a
//! [`crate::mutex::Mutexed`]), its scheduler's spin counter, and whether the kernel is past
//! caring about locks.
//!
//! Upstream: sys/arch/arm64/include/cpu.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/cpu.rs@3c62ede68802
//!
//! A machine's `struct cpu_info` (`bsd`'s until its hardware moves, N4.8 and N7) holds a
//! [`CpuCore`] as its first member (`#[repr(C)]`, checked at compile time there), so the
//! CPU's address is its core's: `TPIDR_EL1` (arm64) keeps naming the `cpu_info`, as
//! `curcpu()` reads it in the C, and the assembly reaches `ci_curproc` and `ci_curpcb` at
//! `offset_of!(CpuCore, ..)`, the same whether the code naming them is `bsd`'s or `hal`'s.
//!
//! **Before [`set_current`]** (the boot CPU before its `cpu_info` is installed, the ABI
//! module's own copy of `hal`, which never installs one): one CPU runs, the core of
//! [`BOOT_CORE`], index 0. No lock's mutual exclusion rests on the answer: the locks
//! compare-and-swap a word that is never 0 whatever core names it (`mutex.rs`, `rwlock.rs`),
//! so a wrong answer costs a diagnostic, never an aliased `&mut`.
//!
//! ## Redesign
//! - LZ's `cpu_info` keeps `ci_curproc` (an `AtomicPtr` wrapper), `ci_curpcb`,
//!   `ci_want_resched` and `ci_mutex_level` as its own fields, and the runtime reached the
//!   running CPU through the `CpuId` bridge (`curcpu()`, `cpu_number()`,
//!   `spc_spinning`, `panicstr || db_active`, `ncpusfound`). Those members are the
//!   [`CpuCore`] at the head of the machine's `cpu_info`, the running core is read from the
//!   machine's per-CPU register here (arm64 `TPIDR_EL1`, the host's thread), and the bridge
//!   is gone (N4.5a). `cpu_number()` is the core's copy of `ci_cpuid`, set by [`attach`];
//!   `ncpusfound` is one past the highest attached number (the per-CPU readers sum the
//!   copies of the CPUs that ever ran: a CPU in the device tree that never attached has an
//!   untouched copy); `panicstr || db_active` is a flag pair `bsd`'s `panic` and ddb set
//!   ([`set_panicking`], [`set_db_active`]).
//! - The registry of attached cores ([`attach`], [`cores`], [`by_number`]) replaces
//!   `CPU_INFO_FOREACH` and `cpuset_infos` for the runtime and the kernel crate, which cannot
//!   name the machine's `cpu_info`.
//! - N4.5c: `ci_cpl`, `ci_ipending` and `ci_idepth` are the core's too (`spl(9)`'s generic
//!   half is [`crate::intr`]'s), and the machine's interrupt mask is reached through
//!   [`intr_disable`], a guard whose drop is `intr_restore` (LZ: `bsd`'s `intr_disable()`
//!   and the `unsafe fn intr_restore(s)` it pairs with).

use core::marker::PhantomData;
use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering,
};

use crate::arch::current::cpu as machine;
use crate::once::OnceStatic;

/// `MAXCPUS`: the most CPUs a kernel supports, at least the machine's (255 on amd64, 256 on
/// arm64). The kernel is always `MULTIPROCESSOR` (decision 40), so there is no one-CPU value.
pub const MAXCPUS: usize = 256;

/// The members of a CPU's `struct cpu_info` the runtime, the context switch and the
/// assembly reach, first in the machine's `cpu_info` (`#[repr(C)]`).
///
/// Who touches what: `curproc` and `curpcb` are written by the CPU itself (the context
/// switch, `main`'s `curproc = &proc0`, a secondary CPU's start) and read by other CPUs
/// (`need_resched`, `cpu_unidle`, ddb), hence atomics; `want_resched` is set from any CPU
/// (`need_resched`) and cleared by the CPU; `mutex_level` and `spinning` are the CPU's own
/// counters (`DIAGNOSTIC`'s `ci_mutex_level`, `spc_spinning`), read by other CPUs only for
/// statistics; `number` is written by [`attach`] before the CPU runs anything.
#[repr(C)]
pub struct CpuCore {
    /// `ci_curproc`: the thread running on the CPU (its address), null before the first.
    curproc: AtomicPtr<()>,
    /// `ci_curpcb`: that thread's pcb, null before the first thread.
    curpcb: AtomicPtr<()>,
    /// `ci_want_resched`: the scheduler asks this CPU to reschedule.
    want_resched: AtomicI32,
    /// `ci_mutex_level` (`DIAGNOSTIC`): the mutexes this CPU holds.
    mutex_level: AtomicI32,
    /// `spc_spinning` of the runtime's locks: this CPU spins for a lock (the legacy locks
    /// count in `bsd`'s `spc_spinning` until they move, N4.5c).
    spinning: AtomicU32,
    /// `ci_cpuid` as [`attach`] recorded it: [`cpu_number`] on this CPU.
    number: AtomicU32,
    /// `ci_cpl`: the current interrupt priority level (the CPU's own; `crate::intr`).
    cpl: AtomicI32,
    /// `ci_ipending`: the soft interrupts pending on this CPU, one bit per `SOFTINTR_*`
    /// (64 bits: amd64's width, its hardware sources share the word).
    ipending: AtomicU64,
    /// `ci_idepth`: the interrupt nesting depth.
    idepth: AtomicU32,
}

impl CpuCore {
    /// A core before its CPU is attached: no thread, index 0.
    pub const fn new() -> Self {
        Self {
            curproc: AtomicPtr::new(core::ptr::null_mut()),
            curpcb: AtomicPtr::new(core::ptr::null_mut()),
            want_resched: AtomicI32::new(0),
            mutex_level: AtomicI32::new(0),
            spinning: AtomicU32::new(0),
            number: AtomicU32::new(0),
            cpl: AtomicI32::new(0),
            ipending: AtomicU64::new(0),
            idepth: AtomicU32::new(0),
        }
    }

    /// `ci_cpl`: this CPU's interrupt priority level.
    #[inline]
    pub fn cpl(&self) -> i32 {
        self.cpl.load(Ordering::Relaxed)
    }

    /// `ci->ci_cpl = ipl`: the level's record, which the machine's `setipl` writes beside the
    /// interrupt controller's mask (`agintc_setipl`, `ampintc_setipl`, `crate::intr`'s
    /// default). The CPU's own word: only code running on this CPU writes it.
    #[inline]
    pub fn set_cpl(&self, ipl: i32) {
        self.cpl.store(ipl, Ordering::Relaxed);
    }

    /// `ci_ipending`: the soft interrupts pending on this CPU.
    #[inline]
    pub fn ipending(&self) -> u64 {
        self.ipending.load(Ordering::Relaxed)
    }

    /// `ci->ci_ipending |= bits`.
    #[inline]
    pub(crate) fn mark_pending(&self, bits: u64) {
        self.ipending.fetch_or(bits, Ordering::Relaxed);
    }

    /// `ci->ci_ipending &= ~bits`.
    #[inline]
    pub(crate) fn clear_pending(&self, bits: u64) {
        self.ipending.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `ci_idepth`: the interrupt nesting depth.
    #[inline]
    pub fn idepth(&self) -> u32 {
        self.idepth.load(Ordering::Relaxed)
    }

    /// `ci->ci_idepth++` (an interrupt handler's entry) or `--` (its exit), as `delta` says.
    #[inline]
    pub fn idepth_add(&self, delta: i32) {
        if delta >= 0 {
            self.idepth
                .fetch_add(delta.unsigned_abs(), Ordering::Relaxed);
        } else {
            self.idepth
                .fetch_sub(delta.unsigned_abs(), Ordering::Relaxed);
        }
    }

    /// A CPU's start (`cpu_init_secondary`): `ci_cpl = IPL_NONE`, `ci_ipending = 0`,
    /// `ci_idepth = 0`.
    pub fn reset_intr_state(&self) {
        self.cpl.store(0, Ordering::Relaxed);
        self.ipending.store(0, Ordering::Relaxed);
        self.idepth.store(0, Ordering::Relaxed);
    }

    /// `ci_curproc`: the address of the thread running on this CPU, null before the first.
    /// Compared, or turned back into a thread only by the code that put it there.
    #[inline]
    pub fn curproc(&self) -> *const () {
        self.curproc.load(Ordering::Relaxed)
    }

    /// `ci->ci_curproc = p`: what `main`'s `curproc = &proc0` and a secondary CPU's start do
    /// (the context switch writes it from assembly).
    #[inline]
    pub(crate) fn set_curproc(&self, p: *const ()) {
        self.curproc.store(p.cast_mut(), Ordering::Relaxed);
    }

    /// `ci_curpcb`: the running thread's pcb, null before the first thread.
    #[inline]
    pub fn curpcb(&self) -> *const () {
        self.curpcb.load(Ordering::Relaxed)
    }

    /// `ci->ci_curpcb = pcb` (`main`'s for `proc0`, a secondary CPU's start).
    #[inline]
    #[cfg_attr(
        any(not(target_os = "none"), feature = "module-rt"),
        allow(dead_code, reason = "the machine's switch alone writes it")
    )]
    pub(crate) fn set_curpcb(&self, pcb: *const ()) {
        self.curpcb.store(pcb.cast_mut(), Ordering::Relaxed);
    }

    /// `ci_want_resched`.
    #[inline]
    pub fn want_resched(&self) -> i32 {
        self.want_resched.load(Ordering::Relaxed)
    }

    /// `ci->ci_want_resched = v` (`need_resched` from any CPU, `clear_resched`).
    #[inline]
    pub fn set_want_resched(&self, v: i32) {
        self.want_resched.store(v, Ordering::Relaxed);
    }

    /// `ci_mutex_level`.
    #[inline]
    pub fn mutex_level(&self) -> i32 {
        self.mutex_level.load(Ordering::Relaxed)
    }

    /// `ci->ci_mutex_level += delta`.
    #[inline]
    pub fn add_mutex_level(&self, delta: i32) {
        self.mutex_level.fetch_add(delta, Ordering::Relaxed);
    }

    /// `ci->ci_mutex_level = 0` (a secondary CPU's start).
    pub fn reset_mutex_level(&self) {
        self.mutex_level.store(0, Ordering::Relaxed);
    }

    /// How many of the runtime's locks this CPU spins for now (`spc_spinning`'s share).
    #[inline]
    pub fn spinning(&self) -> u32 {
        self.spinning.load(Ordering::Relaxed)
    }

    /// The CPU's index, as [`attach`] recorded it.
    #[inline]
    pub fn number(&self) -> usize {
        self.number.load(Ordering::Relaxed) as usize
    }

    /// Records the CPU's index ([`attach`]; the host's own cores).
    pub(crate) fn set_number(&self, n: usize) {
        self.number.store(n as u32, Ordering::Relaxed);
    }

    /// The word that names this CPU: its core's address (the C's `struct cpu_info *`).
    #[inline]
    pub fn word(&self) -> usize {
        core::ptr::from_ref(self).addr()
    }
}

impl Default for CpuCore {
    fn default() -> Self {
        Self::new()
    }
}

/// `CI_CURPROC` within the core: where the assembly finds `ci_curproc`
/// (`offset_of!(cpu_info, core)` plus this, the core being first).
pub const CORE_CURPROC: usize = core::mem::offset_of!(CpuCore, curproc);

/// `CI_CURPCB` within the core.
pub const CORE_CURPCB: usize = core::mem::offset_of!(CpuCore, curpcb);

/// The core that answers before [`set_current`] installed the boot CPU's.
pub static BOOT_CORE: CpuCore = CpuCore::new();

/// The attached cores, by number.
static CORES: [OnceStatic<&'static CpuCore>; MAXCPUS] = [const { OnceStatic::new() }; MAXCPUS];

/// One past the highest number [`attach`] recorded.
static NCORES: AtomicUsize = AtomicUsize::new(0);

/// `panicstr` is set: the kernel panicked.
static PANICKING: AtomicBool = AtomicBool::new(false);

/// `db_active`: a CPU is in ddb.
static DB_ACTIVE: AtomicBool = AtomicBool::new(false);

/// `curcpu()`: the running CPU's core ([`BOOT_CORE`] before [`set_current`]).
#[inline]
pub fn curcpu() -> &'static CpuCore {
    machine::current().unwrap_or(&BOOT_CORE)
}

/// Installs `core` as the running CPU's (arm64: writes `TPIDR_EL1`; the host: the calling
/// thread's). The boot CPU's `initarm` calls it first thing; a secondary CPU's entry code
/// writes the register itself before any Rust code runs there (the machine's premise,
/// `arch/<machine>/cpu.rs`).
pub fn set_current(core: &'static CpuCore) {
    machine::set_current(core);
}

/// Records `core` as CPU `number`'s (`cpu_attach`'s `ci->ci_cpuid = dev->dv_unit`): its
/// [`cpu_number`] from now on, and the registry [`cores`] and [`by_number`] read. Panics for
/// a number at or past [`MAXCPUS`] or attached twice.
pub fn attach(core: &'static CpuCore, number: usize) {
    if number >= MAXCPUS {
        crate::assert::hal_panic(format_args!(
            "cpu attach: number {number} is not below MAXCPUS {MAXCPUS}"
        ));
    }
    core.set_number(number);
    if CORES[number].set(core).is_err() {
        crate::assert::hal_panic(format_args!("cpu attach: cpu{number} attached twice"));
    }
    NCORES.fetch_max(number + 1, Ordering::Release);
}

/// `CPU_INFO_FOREACH`: the attached cores, in number order.
pub fn cores() -> impl Iterator<Item = &'static CpuCore> {
    CORES[..NCORES.load(Ordering::Acquire).min(MAXCPUS)]
        .iter()
        .filter_map(|c| c.get().copied())
}

/// The core attached as CPU `n`, if any.
pub fn by_number(n: usize) -> Option<&'static CpuCore> {
    CORES.get(n).and_then(|c| c.get().copied())
}

/// The number of the attached CPU whose core's word is `w` (a thread's `p_cpu`, which the
/// switch writes as the running CPU's `TPIDR_EL1`), `None` for 0 or a word no attached core
/// has: a walk of the attached cores (`ncpus` compares).
pub fn number_of_word(w: usize) -> Option<usize> {
    if w == 0 {
        return None;
    }
    if curcpu().word() == w {
        return Some(curcpu().number());
    }
    cores().find(|c| c.word() == w).map(CpuCore::number)
}

/// `ncpusfound` as the per-CPU readers walk it: one past the highest attached number (the
/// host: [`MAXCPUS`], every test thread being a CPU of its own), at least 1.
pub fn ncpus() -> usize {
    machine::ncpus(NCORES.load(Ordering::Acquire)).clamp(1, MAXCPUS)
}

/// `cpu_number()`: the running CPU's index (0 before [`set_current`]).
#[inline]
pub fn cpu_number() -> usize {
    curcpu().number()
}

/// `curcpu()` as a word: distinct for every CPU, never 0 (a core's address).
#[inline]
pub(crate) fn curcpu_word() -> usize {
    curcpu().word()
}

/// `spc_spinning += delta`: the running CPU spins for one of the runtime's locks (1), or
/// stopped (-1).
#[inline]
pub(crate) fn spinning(delta: isize) {
    let s = &curcpu().spinning;
    if delta >= 0 {
        s.fetch_add(delta.unsigned_abs() as u32, Ordering::Relaxed);
    } else {
        s.fetch_sub(delta.unsigned_abs() as u32, Ordering::Relaxed);
    }
}

/// `panicstr` is set: `bsd`'s `panic` (and the trap handlers' claim of `panicstr`) call it
/// once the kernel has panicked.
///
/// A trust boundary (`sys/hal/trust.rs`): past caring ([`set_panicking`], [`set_db_active`])
/// the kernel lock's release stops checking that no witness of it is out, and mutexes answer
/// `owned` to every CPU, as the C's checks give up after a panic or in ddb. The two are
/// called only by `bsd`'s `panic` and ddb's entry and exit (no other caller exists, a
/// reviewer's grep); a caller that set them otherwise would let a witness holder read `[K]`
/// cells another CPU changes. They move with `panic(9)` and ddb (N9), when `hal` owns both.
pub fn set_panicking() {
    PANICKING.store(true, Ordering::Release);
}

/// `db_active = on`: ddb is entered (`true`) or left (`false`) on some CPU. Under the
/// premise of [`set_panicking`]: ddb's entry and exit alone call it.
pub fn set_db_active(on: bool) {
    DB_ACTIVE.store(on, Ordering::Release);
}

/// `panicstr || db_active`: after a panic or inside ddb the locks stop excluding, so a CPU
/// that stopped holding one cannot deadlock the debugger or the dump.
#[inline]
pub fn past_caring() -> bool {
    PANICKING.load(Ordering::Acquire) || DB_ACTIVE.load(Ordering::Acquire)
}

/// The machine's interrupts masked on this CPU (`intr_disable()`) until the guard drops
/// (`intr_restore(s)` of the state it saved). Not `Send`: the state is this CPU's.
#[must_use = "dropping the guard at once restores the interrupts"]
pub struct IntrGuard {
    /// The interrupt state `intr_disable` saved (arm64: `DAIF`).
    saved: u64,
    _not_send: PhantomData<*const ()>,
}

/// `intr_disable()`: masks the machine's interrupts on this CPU (arm64: IRQ and FIQ) until
/// the guard drops, which restores the state saved here.
#[inline]
pub(crate) fn intr_disable() -> IntrGuard {
    IntrGuard {
        saved: machine::intr_disable(),
        _not_send: PhantomData,
    }
}

impl Drop for IntrGuard {
    #[inline]
    fn drop(&mut self) {
        machine::intr_restore(self.saved);
    }
}

/// `s = intr_disable()` for a holder that keeps the state beyond one scope: the debugger's
/// mutex (`crate::mutex::DbMutex`), entered and left by separate calls on its owner, which
/// alone stores and reads the state.
#[inline]
pub(crate) fn intr_disable_raw() -> u64 {
    machine::intr_disable()
}

/// `intr_restore(s)` of a state [`intr_disable_raw`] returned on this CPU (`DbMutex::leave`,
/// with what its owner's `enter` stored).
#[inline]
pub(crate) fn intr_restore_raw(s: u64) {
    machine::intr_restore(s);
}

/// `ci_mutex_level += delta` under `DIAGNOSTIC`.
#[cfg(feature = "diagnostic")]
pub(crate) fn mutex_level(delta: i32) {
    curcpu().add_mutex_level(delta);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::thread;

    use super::*;

    #[test]
    fn each_test_thread_is_a_cpu_of_its_own() {
        let me = curcpu();
        assert!(me.number() < MAXCPUS);
        assert_ne!(curcpu_word(), 0);
        assert!(core::ptr::eq(me, curcpu()));
        let other = thread::spawn(curcpu_word).join().unwrap();
        assert_ne!(other, curcpu_word());
        assert!(!past_caring());
        assert_eq!(ncpus(), MAXCPUS);
    }

    #[test]
    fn a_core_installed_on_a_thread_answers_there() {
        let core: &'static CpuCore = Box::leak(Box::new(CpuCore::new()));
        let seen = thread::spawn(move || {
            set_current(core);
            curcpu_word()
        })
        .join()
        .unwrap();
        assert_eq!(seen, core.word());
        assert_ne!(curcpu_word(), core.word());
    }

    #[test]
    fn attached_cores_are_found_by_number() {
        let core: &'static CpuCore = Box::leak(Box::new(CpuCore::new()));
        attach(core, MAXCPUS - 1);
        assert_eq!(core.number(), MAXCPUS - 1);
        assert!(by_number(MAXCPUS - 1).is_some_and(|c| core::ptr::eq(c, core)));
        assert!(cores().any(|c| core::ptr::eq(c, core)));
        assert!(by_number(MAXCPUS).is_none());
    }

    #[test]
    #[should_panic(expected = "attached twice")]
    fn a_number_is_attached_once() {
        let a: &'static CpuCore = Box::leak(Box::new(CpuCore::new()));
        let b: &'static CpuCore = Box::leak(Box::new(CpuCore::new()));
        attach(a, MAXCPUS - 2);
        attach(b, MAXCPUS - 2);
    }

    #[test]
    fn the_spin_count_and_the_mutex_level_are_the_running_cpu_s() {
        let me = curcpu();
        spinning(1);
        assert_eq!(me.spinning(), 1);
        spinning(-1);
        assert_eq!(me.spinning(), 0);
        me.add_mutex_level(2);
        assert_eq!(me.mutex_level(), 2);
        me.reset_mutex_level();
        assert_eq!(me.mutex_level(), 0);
        me.set_want_resched(1);
        assert_eq!(me.want_resched(), 1);
        me.set_want_resched(0);
    }

    /// The host: a core runs on one host thread at a time; another thread's install is
    /// refused while the first holds it, and taken once that thread is gone.
    #[test]
    fn a_core_runs_on_one_host_thread() {
        let core: &'static CpuCore = Box::leak(Box::new(CpuCore::new()));
        let (hold_tx, hold_rx) = std::sync::mpsc::channel::<()>();
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let holder = thread::spawn(move || {
            set_current(core);
            hold_tx.send(()).unwrap();
            go_rx.recv().unwrap();
        });
        hold_rx.recv().unwrap();
        let refused =
            thread::spawn(move || std::panic::catch_unwind(|| set_current(core)).is_err())
                .join()
                .unwrap();
        assert!(refused);
        go_tx.send(()).unwrap();
        holder.join().unwrap();
        let seen = thread::spawn(move || {
            set_current(core);
            curcpu_word()
        })
        .join()
        .unwrap();
        assert_eq!(seen, core.word());
    }
}
/* </TESTS> */
