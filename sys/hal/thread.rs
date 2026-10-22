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
//! The part of a thread the machine reaches by offset: `struct proc`'s `p_stat`, `p_cpu`,
//! `p_addr` and `p_md`, which both machines' context switch, syscall entry and exception
//! return read and write from assembly (`genassym.cf`'s `P_STAT`, `P_CPU`, `P_ADDR`,
//! `P_MD_ASTPENDING`, and amd64's `P_MD_REGS`, `P_MD_FLAGS`); and since N4.5a the
//! machine-independent half of the context switch: the scheduler's lock and its hold, the
//! chosen thread's token, `cpu_switchto`, `cpu_fork`'s and the trampoline's halves, and
//! `curproc`.
//!
//! Upstream: sys/arch/arm64/arm64/cpuswitch.S @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/cpuswitch.rs@3c62ede68802
//!
//! The kernel's thread (`kernel::proc::Thread`) is `#[repr(C)]` with a [`MachineSlot`] as
//! its first field, so a thread's address is its slot's: `ci_curproc` keeps holding the
//! thread, as `CPUVAR(CURPROC)` and `CI_CURPROC` do, and the assembly's offsets are
//! `offset_of!(MachineSlot, field)`, the same whether the code naming them is `bsd`'s or
//! `hal`'s, which cannot name the kernel's type. The machine halves of the switch are
//! `arch/<machine>/switch.rs` (arm64's; the host refuses a switch until its double).
//!
//! ## Redesign
//! - The four members are atomics: the switch writes `p_stat` (`SONPROC`) and `p_cpu` while
//!   `mi_switch` holds `sched_lock` but cannot present a witness from assembly, `signotify`
//!   reads `p_cpu` from another CPU without the lock (`cpu_unidle(p->p_cpu)`), and `sysctl`
//!   and ddb read `p_stat` unlocked. `cpu` holds the `cpu_info` word the switch stores
//!   (`hal::cpu::CpuCore::word`); `addr` the u-area's address, 0 before `cpu_fork` (an
//!   `Option` of a word would put a discriminant where the assembly reads the address).
//! - N4.2 writes the struct and its constructor only; N4.5 moves the switch, the trampoline
//!   and `cpu_fork` here (`hal::thread`). amd64's `Mdproc` is N10's (decision 44): this file
//!   names arm64's and the host's.
//! - N4.5a: `sched_lock`, the scheduler's mutex the C keeps in `sched_bsd.c`, is
//!   [`SCHED_LOCK`], a [`SharedMutex`] at `IPL_SCHED` whose guard is a [`SchedHold`]: the
//!   witness of the scheduler's queues (`hal::rwlock::Witness`, `Exclusive`, `Divisible` by
//!   delegation to the guard). The C hands the lock from the thread that switches away to
//!   the one switched to: the mutex is the CPU's (its owner word names the CPU, not a
//!   thread), the old thread's frame keeps its hold dormant across the switch, and the new
//!   thread releases the lock with its own hold, the one it kept when it switched away, or a
//!   new thread's adopted one ([`SchedHold::adopt`], `proc_trampoline_mi`). The level to
//!   restore lives in each thread's hold (the guard keeps it), so nothing swaps it around the
//!   switch, where the C saves and puts back `MUTEX_OLDIPL(&sched_lock)`.
//! - N4.5a: [`Next`] names the thread a switch goes to, made under the hold from the thread
//!   pinned (`sched_chooseproc`'s result): two addresses and, under `DIAGNOSTIC`, the
//!   generation of the hold it was made under, which the switch compares (a token kept across
//!   an unlock is refused). [`ThreadLayout`] is what a thread type gives the switch: its
//!   slot. One thread type per kernel: the first one named is the only one accepted (a
//!   `TypeId` compared), so an address a switch stored is turned back into that type alone.
//!
//! ## Soundness of the switch's tokens
//!
//! The premises the switch rests on (the reviewer's E, brief section 12), each upheld by the
//! kernel's scheduler (`kernel::sched`) and checked where a check is cheap:
//! - E1: a hold dormant in a suspended frame is resumed only by a switch made under the lock,
//!   so the frame that continues finds the lock held by the CPU it continues on, which the
//!   hold then releases ([`SchedHold`] holds no CPU: the mutex's owner word is the CPU that
//!   holds the lock when the hold drops).
//! - E2: a thread's memory is not freed while it is some CPU's current thread: the kernel's
//!   pool of threads refuses to free one (`BoxPool::init_checked`'s release check), after
//!   the thread's last switch away updated the CPU's current thread.
//! - E3: a thread on a run queue is never freed (its queue entry's drop panics while it is
//!   linked), and the thread a [`Next`] names was taken off a queue under the hold the switch
//!   is made under (the generation, under `DIAGNOSTIC`), so the addresses it holds are alive
//!   when the switch uses them.

use core::any::TypeId;
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU8, AtomicUsize};
#[cfg(feature = "diagnostic")]
use core::sync::atomic::{AtomicU32, Ordering};

use crate::assert::hal_panic;
use crate::intr::IPL_SCHED;
use crate::mutex::{Mutexed, MutexedGuard, SharedMutex};
use crate::once::OnceStatic;

/// `struct mdproc`: the machine's per-thread state (arm64: `md_astpending`).
#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub use crate::arch::arm64::include::proc::Mdproc;

/// `struct mdproc` of the host tests: nothing of a machine's.
#[cfg(not(target_os = "none"))]
#[derive(Debug, Default)]
pub struct Mdproc;

#[cfg(not(target_os = "none"))]
impl Mdproc {
    /// All zero.
    pub const fn new() -> Self {
        Self
    }
}

/// The members of a thread the machine reaches by offset, first in the thread
/// (`#[repr(C)]`), so their offsets are the same from the thread and from the slot.
#[repr(C)]
#[derive(Default)]
pub struct MachineSlot {
    /// `p_stat`: the thread's `S*` state (`SONPROC` stored by the switch).
    pub stat: AtomicU8,
    /// `p_cpu`: the `cpu_info` word of the CPU the thread runs or last ran on, 0 before it
    /// first runs.
    pub cpu: AtomicUsize,
    /// `p_addr`: the address of the thread's u-area (`struct user`, whose `u_pcb` is first),
    /// 0 before `cpu_fork`.
    pub addr: AtomicUsize,
    /// `p_md`.
    pub md: Mdproc,
}

impl MachineSlot {
    /// A slot of a thread being made (`SIDL` is set by `fork1`): all zero, as a `static
    /// struct proc` holds.
    pub const fn new() -> Self {
        Self {
            stat: AtomicU8::new(0),
            cpu: AtomicUsize::new(0),
            addr: AtomicUsize::new(0),
            md: Mdproc::new(),
        }
    }
}

/// `SONPROC`: the `p_stat` the switch stores in the new thread's slot (`<sys/proc.h>`; the
/// kernel's `proc.rs` asserts it equals its own).
pub const SONPROC: u8 = 7;

/// What the context switch needs of a thread type: the slot the machine reaches, which the
/// kernel's thread keeps first (`#[repr(C)]`, asserted there at compile time, so the thread's
/// address is the slot's for the assembly that reads `ci_curproc` by offset).
pub trait ThreadLayout: Sync + 'static {
    /// The thread's [`MachineSlot`].
    fn machine(&self) -> &MachineSlot;
}

/// `SCHED_LOCK`: the scheduler's mutex (`sched_lock`), at `IPL_SCHED` from its making (the
/// C's `SCHED_LOCK_INIT` in `main`). It keeps the run and sleep queues and the threads'
/// `[S]` members; it is held across the context switch (the module's notes).
pub static SCHED_LOCK: SharedMutex = Mutexed::new(IPL_SCHED, ());

/// The generation of [`SCHED_LOCK`]'s holds (`DIAGNOSTIC`): bumped by every
/// [`sched_lock`], recorded by a [`Next`] and compared by the switch.
#[cfg(feature = "diagnostic")]
static GENERATION: AtomicU32 = AtomicU32::new(0);

/// The thread type of this kernel: the first one a token named (see [`ThreadLayout`]).
static THREAD_TYPE: OnceStatic<TypeId> = OnceStatic::new();

/// The proof of holding [`SCHED_LOCK`] (`SCHED_LOCK()` ... `SCHED_UNLOCK()`), the scheduler's
/// witness; its drop releases the lock and restores the level its taking raised. Not `Send`:
/// it stays with the thread that holds it, across the switch (the module's notes, E1).
#[must_use = "dropping the hold at once releases the scheduler lock"]
pub struct SchedHold(pub(crate) MutexedGuard<'static, ()>);

impl SchedHold {
    /// The hold of a thread that did not take the lock: a new thread's first instructions,
    /// with the lock the switch to it kept (`proc_trampoline_mi`'s `mtx_leave`). Restores no
    /// level. Panics unless this CPU holds the lock.
    #[cfg_attr(
        feature = "module-rt",
        allow(dead_code, reason = "the switch is not the ABI module's")
    )]
    pub(crate) fn adopt() -> SchedHold {
        SchedHold(SCHED_LOCK.adopt())
    }

    /// The generation this hold was taken in (`DIAGNOSTIC`).
    #[cfg(feature = "diagnostic")]
    fn generation(&self) -> u32 {
        GENERATION.load(Ordering::Relaxed)
    }
}

/// `SCHED_LOCK()`: takes the scheduler's lock (spinning while another CPU holds it), raising
/// the level to `IPL_SCHED`.
pub fn sched_lock() -> SchedHold {
    let g = SCHED_LOCK.lock();
    #[cfg(feature = "diagnostic")]
    GENERATION.fetch_add(1, Ordering::Relaxed);
    SchedHold(g)
}

/// `SCHED_LOCK_ASSERT_LOCKED()`'s test: this CPU holds the scheduler's lock (or the kernel is
/// past caring, as `mtx_owned`).
pub fn sched_locked() -> bool {
    SCHED_LOCK.owned()
}

/// Accepts `T` as this kernel's thread type: the first type named is recorded, any other is
/// `panic(9)` (one thread type per kernel: an address stored by a switch of one type is never
/// read back as another).
#[inline]
pub(crate) fn check_type<T: 'static>() {
    let want = TypeId::of::<T>();
    let have = match THREAD_TYPE.get() {
        Some(t) => *t,
        None => {
            // A race of two first users records one of them; the other is compared below.
            let _ = THREAD_TYPE.set(want);
            match THREAD_TYPE.get() {
                Some(t) => *t,
                None => want,
            }
        }
    };
    if have != want {
        hal_panic(format_args!(
            "hal::thread: a second thread type ({})",
            core::any::type_name::<T>()
        ));
    }
}

/// The thread a switch goes to (`sched_chooseproc`'s result): made under the scheduler's hold
/// from the thread, pinned, before it leaves its queue; consumed by the switch made under the
/// same hold (E3). Not `Send`, not `Clone`.
#[must_use = "a chosen thread is switched to"]
pub struct Next {
    /// The thread, with its provenance.
    thread: NonNull<()>,
    /// Its slot.
    #[cfg_attr(
        feature = "module-rt",
        allow(dead_code, reason = "the switch is not the ABI module's")
    )]
    slot: NonNull<MachineSlot>,
    /// The hold's generation (`DIAGNOSTIC`).
    #[cfg(feature = "diagnostic")]
    generation: u32,
}

impl Next {
    /// `t`, chosen under `h`.
    pub fn of<T: ThreadLayout>(h: &SchedHold, t: Pin<&T>) -> Next {
        check_type::<T>();
        let t = t.get_ref();
        #[cfg(not(feature = "diagnostic"))]
        let _ = h;
        Next {
            thread: NonNull::from(t).cast(),
            slot: NonNull::from(t.machine()),
            #[cfg(feature = "diagnostic")]
            generation: h.generation(),
        }
    }

    /// The thread's address (what `ci_curproc` will hold), compared, never followed.
    pub fn thread_addr(&self) -> usize {
        self.thread.addr().get()
    }

    /// The thread's slot address.
    #[cfg(test)]
    pub(crate) fn slot_addr(&self) -> usize {
        self.slot.addr().get()
    }

    /// Checks that `h` is the hold this token was made under (`DIAGNOSTIC`): a token kept
    /// across an unlock and a new lock is `panic(9)`.
    #[inline]
    #[cfg_attr(
        feature = "module-rt",
        allow(dead_code, reason = "the switch is not the ABI module's")
    )]
    pub(crate) fn check(&self, h: &SchedHold) {
        #[cfg(feature = "diagnostic")]
        if self.generation != h.generation() {
            hal_panic(format_args!(
                "switch: a Next of generation {} under hold {}",
                self.generation,
                h.generation()
            ));
        }
        #[cfg(not(feature = "diagnostic"))]
        let _ = h;
    }
}

/// The start a new thread's trampoline calls with the scheduler's hold it adopted and the
/// two words its maker gave [`thread_setup`] (the C's `proc_trampoline_mi` and `func(arg)`):
/// registered once (bridges.toml `ThreadStart` while `bsd`'s `fork1` makes the threads).
static START: OnceStatic<fn(SchedHold, usize, usize)> = OnceStatic::new();

/// Registers the start of new threads, once; a second registration keeps the first.
pub fn register_start(
    f: fn(SchedHold, usize, usize),
) -> Result<(), crate::bridges::AlreadyRegistered> {
    START
        .set(f)
        .map_err(|_| crate::bridges::AlreadyRegistered("ThreadStart"))
}

/// `cpu_switchto(old, next)` under the scheduler's hold `h`: the running thread `old` leaves
/// this CPU (its FP state saved, its registers and stack in its pcb) and `next` runs. Returns
/// on `old`'s stack when a switch back to it is made, on whichever CPU, with `h` still
/// borrowed: the hold lies dormant in this suspended frame (E1) and its drop, after this
/// returns, releases the lock that CPU holds then. `next` must have been made under `h` (the
/// generation, under `DIAGNOSTIC`).
#[cfg(not(feature = "module-rt"))]
pub fn switch_to<T: ThreadLayout>(h: &mut SchedHold, old: Pin<&T>, next: Next) {
    check_type::<T>();
    next.check(h);
    let old = old.get_ref();
    #[cfg(feature = "diagnostic")]
    crate::assert::kassert!(
        crate::cpu::curcpu().curproc() == core::ptr::from_ref(old).cast::<()>()
    );
    crate::arch::current::switch::switch(Some(old.machine()), next.slot, next.thread);
}

/// `sched_toidle`'s switch: the running context is not saved (the thread is dead, or none
/// ran yet on this CPU) and `next` runs; the hold goes with it (the lock stays this CPU's,
/// released by `next`'s own hold), so this never returns.
#[cfg(not(feature = "module-rt"))]
pub fn exit_to_idle(h: SchedHold, next: Next) -> ! {
    next.check(&h);
    // The lock passes to `next`, whose dormant hold (or adopted one) releases it.
    core::mem::forget(h);
    crate::arch::current::switch::switch(None, next.slot, next.thread);
    hal_panic(format_args!("cpu_switchto returned"))
}

/// The running thread (`curproc`), pinned, for `f`; `panic(9)` when none runs (before the
/// first thread, on a CPU that has not started one).
pub fn with_current<T: ThreadLayout, R>(f: impl FnOnce(Pin<&T>) -> R) -> R {
    try_with_current(|p| match p {
        Some(p) => f(p),
        None => hal_panic(format_args!("curproc: no thread runs on this CPU")),
    })
}

/// The running thread (`curproc`), pinned, for `f`; `None` before the first thread.
pub fn try_with_current<T: ThreadLayout, R>(f: impl FnOnce(Option<Pin<&T>>) -> R) -> R {
    check_type::<T>();
    let p = crate::cpu::curcpu().curproc().cast::<T>();
    // SAFETY: `ci_curproc` is this CPU's word, written by this CPU alone: by the switch, with
    // a `Next`'s thread, and by `set_current_static`, with a `Pin<&'static T>`, both of the
    // kernel's one thread type (`check_type`, which `T` passed just above), or null. The
    // thread it names is the one running here, and a running thread is not freed (E2): its
    // pool refuses to free it while any CPU's `ci_curproc` names it, and the reaper frees a
    // dead thread only after its last switch away replaced it there. So the pointer is null
    // or names a live `T`. It was pinned when it was stored (a `Next` is made from `Pin<&T>`,
    // the static one is pinned), and a pinned value stays pinned for its whole life. Threads
    // are only ever shared (`&T`, their state atomics and cells under locks), so the shared
    // reference aliases no `&mut`; it lives for `f` alone, while this thread runs.
    let t = unsafe { p.as_ref().map(|t| Pin::new_unchecked(t)) };
    f(t)
}

/// The running thread's address (`curproc` as a word, 0 when none runs): compared, never
/// followed (a lock's owner, a map's `busy`).
pub fn current_addr() -> usize {
    crate::cpu::curcpu().curproc().addr()
}

/// `curproc = p` on this CPU for a thread that lives for ever (`main`'s `curproc = &proc0`),
/// or none (`None`: a secondary CPU before its idle thread runs).
pub fn set_current_static<T: ThreadLayout>(t: Option<Pin<&'static T>>) {
    check_type::<T>();
    let p = t.map_or(core::ptr::null(), |t| {
        core::ptr::from_ref(t.get_ref()).cast::<()>()
    });
    crate::cpu::curcpu().set_curproc(p);
}

/// Whether `t` is the running thread of any attached CPU (the release check of the threads'
/// pool, E2): an `ncpus` walk, for a thread being freed (the reaper's cold path).
pub fn running_anywhere<T: ThreadLayout>(t: &T) -> bool {
    let me = core::ptr::from_ref(t).cast::<()>();
    crate::cpu::curcpu().curproc() == me || crate::cpu::cores().any(|c| c.curproc() == me)
}

/// `cpu_fork(parent, child, stack, tcb, func, arg)`: the child's pcb and frames, so that its
/// first switch runs the trampoline with `start` and `arg` (the registered start's words);
/// `stack` and `tcb`, when not 0, its user stack pointer and TCB. The parent's FP state is
/// saved first when it is the running thread (the CPU holds it).
#[cfg(not(feature = "module-rt"))]
pub fn thread_setup<T: ThreadLayout>(
    parent: Pin<&T>,
    child: Pin<&T>,
    stack: usize,
    tcb: usize,
    start: usize,
    arg: usize,
) {
    check_type::<T>();
    let runs = crate::cpu::curcpu().curproc() == core::ptr::from_ref(parent.get_ref()).cast();
    crate::arch::current::switch::thread_setup(
        parent.get_ref().machine(),
        runs,
        child.get_ref().machine(),
        stack,
        tcb,
        start,
        arg,
    );
}

/// The frames of a thread that runs nowhere and starts afresh with `start` and `arg` (a
/// secondary CPU's idle thread, `cpu_init_secondary`).
#[cfg(not(feature = "module-rt"))]
pub fn thread_restart<T: ThreadLayout>(t: Pin<&T>, start: usize, arg: usize) {
    check_type::<T>();
    crate::arch::current::switch::thread_restart(t.get_ref().machine(), start, arg);
}

/// The trampoline's machine-independent part (`proc_trampoline_mi`'s first step): the hold of
/// the lock the switch to this new thread kept, adopted, and the registered start called
/// with it and the two words.
#[cfg(not(feature = "module-rt"))]
pub(crate) fn trampoline(start: usize, arg: usize) {
    let hold = SchedHold::adopt();
    match START.get() {
        Some(f) => f(hold, start, arg),
        None => hal_panic(format_args!("proc_trampoline: no start registered")),
    }
}

const _: () = {
    assert!(core::mem::offset_of!(MachineSlot, stat) == 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;
    use core::sync::atomic::Ordering;

    use super::*;

    #[repr(C)]
    struct Outer {
        machine: MachineSlot,
        tail: u64,
    }

    #[test]
    fn the_slot_is_at_the_start_of_what_embeds_it_first() {
        assert_eq!(offset_of!(Outer, machine), 0);
        assert_eq!(
            offset_of!(Outer, machine) + offset_of!(MachineSlot, cpu),
            offset_of!(MachineSlot, cpu)
        );
        let o = Outer {
            machine: MachineSlot::new(),
            tail: 1,
        };
        assert_eq!(o.machine.stat.load(Ordering::Relaxed), 0);
        assert_eq!(o.machine.addr.load(Ordering::Relaxed), 0);
        assert_eq!(o.tail, 1);
    }

    /// The hal tests' thread type (one per process: `check_type`).
    #[repr(C)]
    pub(crate) struct TestThread {
        pub(crate) machine: MachineSlot,
        pub(crate) n: u32,
    }

    impl ThreadLayout for TestThread {
        fn machine(&self) -> &MachineSlot {
            &self.machine
        }
    }

    struct OtherThread(MachineSlot);

    impl ThreadLayout for OtherThread {
        fn machine(&self) -> &MachineSlot {
            &self.0
        }
    }

    /// The hold is the scheduler mutex's guard: the lock is this CPU's while it lives, and a
    /// `Guarded` cell keyed to it opens through it.
    #[test]
    fn the_hold_is_the_scheduler_lock_s_witness() {
        crate::testutil::install();
        static CELL: crate::rwlock::Guarded<u32> =
            crate::rwlock::Guarded::new_static_mutexed(&SCHED_LOCK, 0);
        let mut h = sched_lock();
        assert!(sched_locked());
        *CELL.get_mut(&mut h) += 1;
        assert!(*CELL.get(&h) >= 1);
        drop(h);
        assert!(!sched_locked());
    }

    /// A frame that did not take the lock adopts the hold of the CPU that holds it (a new
    /// thread's trampoline) and releases the lock with it.
    #[test]
    fn a_hold_is_adopted_where_the_lock_is_held() {
        crate::testutil::install();
        let h = sched_lock();
        // The switch: the taking frame's hold stays dormant, another frame releases.
        core::mem::forget(h);
        let adopted = SchedHold::adopt();
        drop(adopted);
        assert!(!sched_locked());
    }

    #[test]
    #[should_panic(expected = "does not hold it")]
    fn a_hold_is_not_adopted_where_the_lock_is_free() {
        crate::testutil::install();
        // This thread (a CPU of its own) does not hold the lock, whoever else may.
        let _h = SchedHold::adopt();
    }

    /// A chosen thread's token names it and its slot, and passes its hold's check.
    #[test]
    fn next_names_the_thread_and_its_slot() {
        crate::testutil::install();
        let t = core::pin::pin!(TestThread {
            machine: MachineSlot::new(),
            n: 7,
        });
        let h = sched_lock();
        let next = Next::of(&h, t.as_ref());
        assert_eq!(next.thread_addr(), core::ptr::from_ref(&*t).addr());
        assert_eq!(next.slot_addr(), core::ptr::from_ref(&t.machine).addr());
        next.check(&h);
        assert_eq!(t.n, 7);
    }

    #[test]
    #[should_panic(expected = "a second thread type")]
    fn one_thread_type_per_kernel() {
        check_type::<TestThread>();
        check_type::<OtherThread>();
    }

    /// Under `DIAGNOSTIC` a token kept across an unlock is refused.
    #[cfg(feature = "diagnostic")]
    #[test]
    #[should_panic(expected = "a Next of generation")]
    fn a_token_from_an_earlier_hold_is_refused() {
        crate::testutil::install();
        let t = core::pin::pin!(TestThread {
            machine: MachineSlot::new(),
            n: 0,
        });
        let next = Next::of(&sched_lock(), t.as_ref());
        let h = sched_lock();
        next.check(&h);
    }

    /// The running thread is read back pinned, as the type that set it; no thread, `None`.
    #[test]
    fn the_current_thread_is_read_back_pinned() {
        let t: &'static TestThread = std::boxed::Box::leak(std::boxed::Box::new(TestThread {
            machine: MachineSlot::new(),
            n: 41,
        }));
        assert!(try_with_current::<TestThread, _>(|p| p.is_none()));
        set_current_static(Some(crate::pin::pin_static(t)));
        assert_eq!(current_addr(), core::ptr::from_ref(t).addr());
        assert_eq!(with_current::<TestThread, _>(|p| p.n), 41);
        assert!(running_anywhere(t));
        set_current_static::<TestThread>(None);
        assert_eq!(current_addr(), 0);
        assert!(!running_anywhere(t));
    }

    #[test]
    #[should_panic(expected = "no thread runs on this CPU")]
    fn no_current_thread_is_a_panic_for_with_current() {
        with_current::<TestThread, _>(|_| ());
    }
}
/* </TESTS> */
