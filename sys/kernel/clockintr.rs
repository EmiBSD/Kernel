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
 * Copyright (c) 2003 Dale Rahn <drahn@openbsd.org>
 * Copyright (c) 2020 Mark Kettenis <kettenis@openbsd.org>
 * Copyright (c) 2020-2024 Scott Cheloha <cheloha@openbsd.org>
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
 * Copyright (c) 2020-2024 Scott Cheloha <cheloha@openbsd.org>
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
//! Schedulable clock interrupts (`kern_clockintr.c`): every CPU keeps a queue of clock
//! interrupt callbacks ordered by expiration, its interrupt clock (`intrclock`) is rearmed for
//! the first, and `clockintr_dispatch` runs the expired ones from the clock interrupt.
//!
//! Upstream: sys/kern/kern_clockintr.c @ 3ce1f3f79392
//! Upstream: sys/sys/clockintr.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_clockintr.rs@3c62ede68802
//! LZ: sys/sys/clockintr.rs@3c62ede68802
//!
//! Locks: `[m]` the queue's mutex (`cq_mtx`, `IPL_CLOCK`): what [`Queue`] holds, inside the
//! `Mutexed`; `[o]` owned by the queue's CPU (the request, the cached uptime, the statistics
//! and their generation, the dispatch flag); `[I]` set once (the interrupt clock).
//!
//! ## Deviations
//! - Not a deviation, a reminder: the `cq_stat` time sums keep the C's modular `uint64_t`
//!   arithmetic (`wrapping_add`/`wrapping_sub`): `clockintr_dispatch` can read the clock
//!   behind its own `start` when a clock interrupt is held off longer than the timecounter's
//!   period (amd64's 15-bit i8254 behind the LAPIC timer wraps every 27.46 ms); OpenBSD on
//!   the same counter does the same, and checked arithmetic would panic where the C goes on.
//! - Under `bsd`'s feature `qemu` only (not in C; [`set_uptime_check`]): the dispatch checks
//!   every uptime it reads against the previous one of the same CPU and prints `uptime went
//!   backwards by <n> ns`, which the smoke tests reject, counting the checks and the readings
//!   behind per CPU ([`uptime_check_counts`], the MP self-test's proof that every CPU
//!   dispatches its own clock interrupts).
//! - The `ddb` printers (`db_show_clockintr*`) come with the real ddb.
//!
//! ## Redesign
//! - The callbacks a queue holds are the C's five binders (`sched_init_cpu`'s `spc_itimer`,
//!   `spc_profclock`, `spc_roundrobin`, `spc_statclock` and `clockintr_cpu_init`'s
//!   `cq_hardclock`; no driver binds one on either machine at the pin, decision 9): a
//!   [`Clockintr`] is a `Copy` handle naming a CPU and a [`Slot`], not a `struct clockintr`
//!   linked by raw `TAILQ_ENTRY`s into the queue. The `[m]` state (each slot's expiration,
//!   callback, pending flag; the pending order; the running slot; `cq_flags`) is one
//!   [`Queue`] inside the queue's `Mutexed`, so no field is reachable without the mutex. The
//!   pending queue is the slots in expiration order (`clockqueue_pend_insert`'s walk to the
//!   first strictly later one, so equal expirations keep their insertion order, as the C's
//!   `TAILQ_INSERT_BEFORE`); `cq_all` is the bound slots. A sixth binder (dt(4)'s, GPROF's
//!   `gmonclock`, neither in LZ) is one more slot.
//! - `cl_arg` is gone: every binder at the pin passes `NULL` (decision 9; dt(4) would bring
//!   the typed argument back). The callback is `fn(&Clockrequest, &ClockFrame)`, the frame
//!   `hal::clock`'s typed one, not a `void *`.
//! - `struct clockqueue` leaves `cpu_info`: [`CPUS`] is a `hal::percpu::PerCpu<ClockCpu>`
//!   (decision 26's `Clockqueue` boundary, typed here, after N4.5's `SchedCpu`); the C's
//!   `Cell<*const Clockqueue>` back pointers (`cl_queue`, `cr_queue`) are the handle's CPU
//!   number and the running CPU's own queue.
//! - `struct intrclock` (a cookie and two function pointers) is the [`Intrclock`] trait
//!   (decision 46; `ic_cookie` is `self`), installed once per CPU in a `OnceStatic`.
//! - `cq_stat` is seven atomics behind the C's generation (`cq_gen`), the same protocol the
//!   timehands use (`docs/IDIOMS.md`).
//! - `clockintr_unbind(CL_BARRIER)` sleeps until the running callback returns (`clkbar`,
//!   on the queue's own channel): LZ's `unported!` is closed.
//! - `clockrequest_advance_random`'s `random(&ci->ci_randseed)` reads the queue's own seed
//!   ([`set_randseed`], written where `bsd` writes `ci_randseed`).

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering, fence};

use hal::clock::ClockFrame;
use hal::intr::IPL_CLOCK;
use hal::link::Link;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::percpu::{PerCpu, PerCpuInit};
use hal::sleep::{INFSLP, PWAIT};

use crate::assert::{kassert, kpanic};
use crate::clock::{PROFCLOCK_PERIOD, STATCLOCK_AVG, STATCLOCK_IS_RANDOMIZED, hardclock_period};
use crate::sys::errno::Errno;
use crate::sysctl::{self, SysctlNode, SysctlReq};
use crate::tc::nsecuptime;

/// `CQ_INIT`: `clockintr_cpu_init()` done.
pub const CQ_INIT: u32 = 0x0000_0001;
/// `CQ_INTRCLOCK`: intrclock installed.
pub const CQ_INTRCLOCK: u32 = 0x0000_0002;
/// `CQ_IGNORE_REQUEST`: ignore callback requests.
pub const CQ_IGNORE_REQUEST: u32 = 0x0000_0004;
/// `CQ_NEED_WAKEUP`: caller at barrier.
pub const CQ_NEED_WAKEUP: u32 = 0x0000_0008;
/// `CQ_STATE_MASK`.
pub const CQ_STATE_MASK: u32 = 0x0000_000f;

/// `CL_BARRIER`: block if callback is running.
pub const CL_BARRIER: u32 = 0x0000_0001;
/// `CL_FLAG_MASK`.
pub const CL_FLAG_MASK: u32 = 0x0000_0001;

/// `CR_RESCHEDULE`: reschedule upon return.
pub const CR_RESCHEDULE: u32 = 0x0000_0001;

/// `KERN_CLOCKINTR_STATS`: struct: stats.
pub const KERN_CLOCKINTR_STATS: i32 = 1;

/// The binders' slots of a queue.
const NSLOTS: usize = 5;

/// The callback of a clock interrupt: `cl_func(request, frame, arg)` without the argument
/// no binder uses.
pub type ClockintrFn = fn(&Clockrequest, &ClockFrame);

/// The clock interrupts a CPU's queue holds: the C's five binders.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    /// `cq_hardclock` (`clockintr_cpu_init`, the primary CPU only).
    Hardclock = 0,
    /// `spc_statclock` (`sched_init_cpu`).
    Statclock = 1,
    /// `spc_itimer` (`sched_init_cpu`).
    Itimer = 2,
    /// `spc_profclock` (`sched_init_cpu`).
    Profclock = 3,
    /// `spc_roundrobin` (`sched_init_cpu`).
    Roundrobin = 4,
}

impl Slot {
    /// The index of the slot in its queue.
    const fn index(self) -> usize {
        self as usize
    }
}

/// `struct clockintr`: a handle on one clock interrupt, a CPU's queue and a slot of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Clockintr {
    cpu: u16,
    slot: Slot,
}

impl Clockintr {
    /// The clock interrupt `slot` of CPU `cpu`'s queue.
    pub const fn of(cpu: usize, slot: Slot) -> Self {
        Self {
            cpu: cpu as u16,
            slot,
        }
    }

    /// The CPU whose queue it is on (`cl_queue`).
    pub const fn cpu(&self) -> usize {
        self.cpu as usize
    }

    /// Its slot.
    pub const fn slot(&self) -> Slot {
        self.slot
    }

    /// Its queue.
    fn queue(&self) -> &'static ClockCpu {
        match CPUS.cpu(self.cpu as usize) {
            Some(q) => q,
            None => kpanic(format_args!("clockintr: no CPU {}", self.cpu)),
        }
    }
}

/// `struct clockintr_stat`: the dispatch statistics (`kern.clockintr.stats`, the C's
/// layout).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClockintrStat {
    /// `cs_dispatched`: total time in dispatch (ns).
    pub cs_dispatched: u64,
    /// `cs_early`: number of early dispatch calls.
    pub cs_early: u64,
    /// `cs_earliness`: total earliness (ns).
    pub cs_earliness: u64,
    /// `cs_lateness`: total lateness (ns).
    pub cs_lateness: u64,
    /// `cs_prompt`: number of prompt dispatch calls.
    pub cs_prompt: u64,
    /// `cs_run`: number of events dispatched.
    pub cs_run: u64,
    /// `cs_spurious`: number of spurious dispatch calls.
    pub cs_spurious: u64,
}

hal::abi_pod!(ClockintrStat {
    cs_dispatched: u64,
    cs_early: u64,
    cs_earliness: u64,
    cs_lateness: u64,
    cs_prompt: u64,
    cs_run: u64,
    cs_spurious: u64
});

/// `struct clockrequest`: what a callback asks of the dispatch (`[o]`, the queue's CPU's).
pub struct Clockrequest {
    /// `cr_expiration`: copy of the dispatch time.
    expiration: AtomicU64,
    /// `cr_flags`: `CR_*` flags.
    flags: AtomicU32,
    /// \[I\] `cr_queue`: the enclosing queue (set by `clockqueue_init`).
    queue: Link<ClockCpu>,
}

impl Clockrequest {
    const fn new() -> Self {
        Self {
            expiration: AtomicU64::new(0),
            flags: AtomicU32::new(0),
            queue: Link::new(),
        }
    }

    /// `cr_expiration`: the time the running callback was due (advanced by its request).
    pub fn expiration(&self) -> u64 {
        self.expiration.load(Ordering::Relaxed)
    }

    /// `cr_flags`.
    pub fn flags(&self) -> u32 {
        self.flags.load(Ordering::Relaxed)
    }
}

/// One slot's `[m]` members of `struct clockintr`.
#[derive(Clone, Copy)]
struct Cl {
    /// `cl_expiration`: dispatch time.
    expiration: u64,
    /// `cl_func`: the callback, `None` while unbound (`cl_queue == NULL`).
    func: Option<ClockintrFn>,
    /// `CLST_PENDING`: scheduled to run (on the pending order).
    pending: bool,
}

/// `struct clockqueue`'s `[m]` members: the slots, the pending order, the running slot and
/// `cq_flags`.
pub struct Queue {
    cl: [Cl; NSLOTS],
    /// `cq_pend`: the pending slots, by expiration.
    pend: [Slot; NSLOTS],
    npend: usize,
    /// `cq_running`: the slot whose callback runs.
    running: Option<Slot>,
    /// `cq_flags`: `CQ_*` flags.
    flags: u32,
}

impl Queue {
    const fn new() -> Self {
        Self {
            cl: [Cl {
                expiration: 0,
                func: None,
                pending: false,
            }; NSLOTS],
            pend: [Slot::Hardclock; NSLOTS],
            npend: 0,
            running: None,
            flags: 0,
        }
    }

    /// `TAILQ_FIRST(&cq->cq_pend)`.
    fn first(&self) -> Option<Slot> {
        if self.npend == 0 {
            None
        } else {
            Some(self.pend[0])
        }
    }

    /// `clockqueue_next`: the expiration of the first pending clock interrupt.
    fn next(&self) -> u64 {
        match self.first() {
            Some(s) => self.cl[s.index()].expiration,
            None => kpanic(format_args!("clockqueue_next: nothing pending")),
        }
    }

    /// `clockqueue_pend_delete`.
    fn pend_delete(&mut self, s: Slot) {
        kassert!(self.cl[s.index()].pending);
        let Some(i) = self.pend[..self.npend].iter().position(|&p| p == s) else {
            kpanic(format_args!("clockqueue_pend_delete: not pending"));
        };
        self.pend.copy_within(i + 1..self.npend, i);
        self.npend -= 1;
        self.cl[s.index()].pending = false;
    }

    /// `clockqueue_pend_insert`: `s` due at `expiration`, before the first pending slot due
    /// strictly later.
    fn pend_insert(&mut self, s: Slot, expiration: u64) {
        kassert!(!self.cl[s.index()].pending);
        self.cl[s.index()].expiration = expiration;
        let at = self.pend[..self.npend]
            .iter()
            .position(|&p| expiration < self.cl[p.index()].expiration)
            .unwrap_or(self.npend);
        self.pend.copy_within(at..self.npend, at + 1);
        self.pend[at] = s;
        self.npend += 1;
        self.cl[s.index()].pending = true;
    }
}

/// `cq_stat` as seven atomics, read and written under `cq_gen`'s protocol.
struct StatWords([AtomicU64; 7]);

impl StatWords {
    const fn new() -> Self {
        Self([const { AtomicU64::new(0) }; 7])
    }

    fn get(&self) -> ClockintrStat {
        let w = |i: usize| self.0[i].load(Ordering::Relaxed);
        ClockintrStat {
            cs_dispatched: w(0),
            cs_early: w(1),
            cs_earliness: w(2),
            cs_lateness: w(3),
            cs_prompt: w(4),
            cs_run: w(5),
            cs_spurious: w(6),
        }
    }

    fn set(&self, s: &ClockintrStat) {
        let w = |i: usize, v: u64| self.0[i].store(v, Ordering::Relaxed);
        w(0, s.cs_dispatched);
        w(1, s.cs_early);
        w(2, s.cs_earliness);
        w(3, s.cs_lateness);
        w(4, s.cs_prompt);
        w(5, s.cs_run);
        w(6, s.cs_spurious);
    }
}

/// `struct clockqueue`: one CPU's clock interrupt state.
pub struct ClockCpu {
    /// \[a\] `cq_mtx` and the \[m\] members it guards.
    q: Mutexed<Queue>,
    /// \[o\] `cq_request`: the callback request object.
    request: Clockrequest,
    /// \[o\] `cq_uptime`: cached uptime.
    uptime: AtomicU64,
    /// \[o\] `cq_stat`: dispatch statistics.
    stat: StatWords,
    /// \[o\] `cq_gen` (volatile): `cq_stat` update generation.
    generation: AtomicU32,
    /// \[o\] `cq_dispatch` (volatile): dispatch is running.
    dispatching: AtomicU32,
    /// \[I\] `cq_intrclock`: local interrupt clock.
    intrclock: OnceStatic<&'static dyn Intrclock>,
    /// `CQ_INIT`, readable without the mutex (`sysctl_clockintr` reads it so).
    init: AtomicBool,
    /// `CPU_IS_PRIMARY(ci)`: the CPU that runs the hardclock.
    primary: AtomicBool,
    /// \[o\] `ci_randseed`: `random()`'s seed for the randomized statclock.
    randseed: AtomicU32,
    /// `&cq->cq_running` as a wait channel (`clockintr_unbind`'s barrier).
    barrier: AtomicU8,
    /// Feature `qemu` of `bsd`: the last uptime the dispatch read, the checks, the behinds.
    last_uptime: AtomicU64,
    uptime_checks: AtomicU64,
    uptime_behind: AtomicU64,
}

impl PerCpuInit for ClockCpu {
    /// A queue before `clockqueue_init`: all zero, as the C's `cpu_info` static.
    const INIT: Self = Self {
        q: Mutexed::new(IPL_CLOCK, Queue::new()),
        request: Clockrequest::new(),
        uptime: AtomicU64::new(0),
        stat: StatWords::new(),
        generation: AtomicU32::new(0),
        dispatching: AtomicU32::new(0),
        intrclock: OnceStatic::new(),
        init: AtomicBool::new(false),
        primary: AtomicBool::new(false),
        randseed: AtomicU32::new(1),
        barrier: AtomicU8::new(0),
        last_uptime: AtomicU64::new(0),
        uptime_checks: AtomicU64::new(0),
        uptime_behind: AtomicU64::new(0),
    };
}

impl ClockCpu {
    /// `clockqueue_intrclock_install(cq, ic)`: the interrupt clock installed, once.
    fn intrclock_install(&self, ic: &'static dyn Intrclock) {
        let mut g = self.q.lock();
        if g.flags & CQ_INTRCLOCK == 0 {
            let _ = self.intrclock.set(ic);
            g.flags |= CQ_INTRCLOCK;
        }
    }

    /// Whether this is the running CPU's queue (`cq == &curcpu()->ci_queue`).
    fn here(&self) -> bool {
        core::ptr::eq(self, this())
    }

    /// The interrupt clock, once installed.
    fn ic(&self) -> Option<&'static dyn Intrclock> {
        self.intrclock.get().copied()
    }

    /// `clockqueue_intrclock_reprogram`: arms the interrupt clock for the first pending
    /// clock interrupt, or triggers it when that is already due.
    fn reprogram(&self, g: &MutexedGuard<'_, Queue>) {
        kassert!(g.flags & CQ_INTRCLOCK != 0);

        let exp = g.next();
        let now = nsecuptime();
        let Some(ic) = self.ic() else {
            return;
        };
        if now < exp {
            ic.rearm(exp - now);
        } else {
            ic.trigger();
        }
    }

    /// `clockintr_cancel_locked`.
    fn cancel_locked(&self, g: &mut MutexedGuard<'_, Queue>, s: Slot) {
        if g.cl[s.index()].pending {
            let was_next = g.first() == Some(s);
            g.pend_delete(s);
            if g.flags & CQ_INTRCLOCK != 0 && was_next && g.npend != 0 && self.here() {
                self.reprogram(g);
            }
        }
        if g.running == Some(s) {
            g.flags |= CQ_IGNORE_REQUEST;
        }
    }

    /// `clockintr_schedule_locked`.
    fn schedule_locked(&self, g: &mut MutexedGuard<'_, Queue>, s: Slot, expiration: u64) {
        if g.cl[s.index()].pending {
            g.pend_delete(s);
        }
        g.pend_insert(s, expiration);
        if g.flags & CQ_INTRCLOCK != 0 && g.first() == Some(s) && self.here() {
            self.reprogram(g);
        }
        if g.running == Some(s) {
            g.flags |= CQ_IGNORE_REQUEST;
        }
    }

    /// Feature `qemu`: prints `uptime went backwards` when `now` is behind this CPU's
    /// previous reading.
    fn uptime_check(&self, now: u64) {
        if !UPTIME_CHECK.load(Ordering::Relaxed) {
            return;
        }
        let last = self.last_uptime.swap(now, Ordering::Relaxed);
        self.uptime_checks.fetch_add(1, Ordering::Relaxed);
        if now < last {
            self.uptime_behind.fetch_add(1, Ordering::Relaxed);
            hal::prf::printf(format_args!(
                "uptime went backwards by {} ns ({} -> {})\n",
                last - now,
                last,
                now
            ));
        }
    }
}

/// `ci_queue` of every CPU.
pub static CPUS: PerCpu<ClockCpu> = PerCpu::new();

/// Whether the dispatch checks that the uptime never steps back (`bsd`'s feature `qemu`).
static UPTIME_CHECK: AtomicBool = AtomicBool::new(false);

/// `struct intrclock`: the local interrupt clock a machine installs per CPU (`ic_cookie` is
/// `self`).
pub trait Intrclock: Sync {
    /// `ic_rearm(cookie, nsecs)`: program the next interrupt `nsecs` from now.
    fn rearm(&self, nsecs: u64);
    /// `ic_trigger(cookie)`: raise an interrupt as soon as possible.
    fn trigger(&self);
}

#[cfg(test)]
std::thread_local! {
    /// The CPU a test thread acts as (its queue is `this()`), `None` for `cpu_number()`.
    static TEST_CPU: core::cell::Cell<Option<usize>> = const { core::cell::Cell::new(None) };
}

/// The running CPU's queue.
fn this() -> &'static ClockCpu {
    #[cfg(test)]
    if let Some(n) = TEST_CPU.with(|c| c.get())
        && let Some(q) = CPUS.cpu(n)
    {
        return q;
    }
    CPUS.get()
}

/// The running CPU's number (`CPU_INFO_UNIT(curcpu())`).
fn this_cpu() -> usize {
    #[cfg(test)]
    if let Some(n) = TEST_CPU.with(|c| c.get()) {
        return n;
    }
    hal::cpu::cpu_number()
}

/// `clockintr_cpu_init`: readies the running CPU for `clockintr_dispatch`. The first time,
/// installs the interrupt clock, if any, and binds the hardclock on the primary CPU; then
/// advances the schedule as needed.
pub fn clockintr_cpu_init(ic: Option<&'static dyn Intrclock>) {
    let cq = this();
    let cpu = this_cpu();
    let primary = cq.primary.load(Ordering::Relaxed);
    let mut multiplier: u64 = 0;
    let mut reset_cq_intrclock = false;

    if let Some(ic) = ic {
        cq.intrclock_install(ic);
    }

    // TODO: Remove this from struct clockqueue.
    let hardclock = Clockintr::of(cpu, Slot::Hardclock);
    if primary && expiration(hardclock) == 0 {
        clockintr_bind(hardclock, clockintr_hardclock);
    }

    // Mask CQ_INTRCLOCK while we're advancing the internal clock interrupts. We don't want
    // the intrclock to fire until this thread reaches clockintr_trigger().
    {
        let mut g = cq.q.lock();
        if g.flags & CQ_INTRCLOCK != 0 {
            g.flags &= !CQ_INTRCLOCK;
            reset_cq_intrclock = true;
        }
    }

    // Until we understand scheduler lock contention better, stagger the hardclock and
    // statclock so they don't all happen at once. If we have no intrclock it doesn't matter,
    // we have no control anyway. The primary CPU's starting offset is always zero, so leave
    // the multiplier zero.
    if !primary && reset_cq_intrclock {
        multiplier = cpu as u64;
    }
    let ncpus = hal::cpu::MAXCPUS as u32;

    // The first time we do this, the primary CPU cannot skip any hardclocks. We can skip
    // hardclocks on subsequent calls because the global tick value is advanced during
    // inittodr(9) on our behalf.
    if primary {
        if expiration(hardclock) == 0 {
            clockintr_schedule(hardclock, 0);
        } else {
            clockintr_advance(hardclock, hardclock_period());
        }
    }

    // We can always advance the statclock. There is no reason to stagger a randomized
    // statclock.
    let statclock = Clockintr::of(cpu, Slot::Statclock);
    let statclock_avg = STATCLOCK_AVG.load(Ordering::Relaxed);
    if !STATCLOCK_IS_RANDOMIZED.load(Ordering::Relaxed) && expiration(statclock) == 0 {
        clockintr_stagger(statclock, statclock_avg, multiplier as u32, ncpus);
    }
    clockintr_advance(statclock, statclock_avg);

    // XXX Need to find a better place to do this. We can't do it in sched_init_cpu() because
    // initclocks() runs after it.
    let itimer = Clockintr::of(cpu, Slot::Itimer);
    if expiration(itimer) == 0 {
        clockintr_stagger(itimer, hardclock_period(), multiplier as u32, ncpus);
    }
    let profclock = Clockintr::of(cpu, Slot::Profclock);
    if expiration(profclock) == 0 {
        clockintr_stagger(
            profclock,
            PROFCLOCK_PERIOD.load(Ordering::Relaxed),
            multiplier as u32,
            ncpus,
        );
    }
    let roundrobin = Clockintr::of(cpu, Slot::Roundrobin);
    if expiration(roundrobin) == 0 {
        clockintr_stagger(roundrobin, hardclock_period(), multiplier as u32, ncpus);
    }
    clockintr_advance(
        roundrobin,
        crate::sched::ROUNDROBIN_PERIOD.load(Ordering::Relaxed),
    );

    if reset_cq_intrclock {
        cq.q.lock().flags |= CQ_INTRCLOCK;
    }
}

/// `clockintr_trigger`: if we have an intrclock, triggers it to start the dispatch cycle.
pub fn clockintr_trigger() {
    let cq = this();

    kassert!(cq.init.load(Ordering::Relaxed));

    let installed = cq.q.lock().flags & CQ_INTRCLOCK != 0;
    if installed && let Some(ic) = cq.ic() {
        ic.trigger();
    }
}

/// `clockintr_dispatch`: runs every expired clock interrupt of the running CPU; `true` when
/// something ran (the interrupt handler's "handled").
pub fn clockintr_dispatch(frame: &ClockFrame) -> bool {
    let cq = this();
    let request = &cq.request;
    let mut lateness: u64 = 0;
    let mut run: u64 = 0;

    if cq.dispatching.load(Ordering::Relaxed) != 0 {
        kpanic(format_args!("clockintr_dispatch: recursive dispatch"));
    }
    cq.dispatching.store(1, Ordering::Relaxed);

    hal::intr::splassert(IPL_CLOCK, "clockintr_dispatch");
    kassert!(cq.init.load(Ordering::Relaxed));

    let mut g = cq.q.lock();

    // If nothing is scheduled or we arrived too early, we have nothing to do.
    let start = nsecuptime();
    cq.uptime_check(start);
    cq.uptime.store(start, Ordering::Relaxed);
    let mut rearm = true;
    if g.npend == 0 {
        rearm = false; // goto stats
    } else if start < g.next() {
        // goto rearm
    } else {
        lateness = start.wrapping_sub(g.next());

        // Dispatch expired events.
        while let Some(s) = g.first() {
            let exp = g.cl[s.index()].expiration;
            if cq.uptime.load(Ordering::Relaxed) < exp {
                // Double-check the time before giving up.
                let now = nsecuptime();
                cq.uptime_check(now);
                cq.uptime.store(now, Ordering::Relaxed);
                if now < exp {
                    break;
                }
            }

            // This clockintr has expired. Execute it.
            g.pend_delete(s);
            request.expiration.store(exp, Ordering::Relaxed);
            let func = g.cl[s.index()].func;
            g.running = Some(s);
            drop(g);

            if let Some(func) = func {
                func(request, frame);
            }

            g = cq.q.lock();
            g.running = None;
            if g.flags & CQ_IGNORE_REQUEST != 0 {
                g.flags &= !CQ_IGNORE_REQUEST;
                request.flags.fetch_and(!CR_RESCHEDULE, Ordering::Relaxed);
            }
            if request.flags.load(Ordering::Relaxed) & CR_RESCHEDULE != 0 {
                request.flags.fetch_and(!CR_RESCHEDULE, Ordering::Relaxed);
                let exp = request.expiration.load(Ordering::Relaxed);
                g.pend_insert(s, exp);
            }
            if g.flags & CQ_NEED_WAKEUP != 0 {
                g.flags &= !CQ_NEED_WAKEUP;
                drop(g);
                hal::sleep::wakeup(core::ptr::from_ref(&cq.barrier));
                g = cq.q.lock();
            }
            run += 1;
        }
    }

    // Dispatch complete.
    let uptime = cq.uptime.load(Ordering::Relaxed);
    if rearm {
        // Rearm the interrupt clock if we have one.
        if g.flags & CQ_INTRCLOCK != 0
            && g.npend != 0
            && let Some(ic) = cq.ic()
        {
            ic.rearm(g.next().wrapping_sub(uptime));
        }
    }

    // Update our stats.
    let ogen = cq.generation.load(Ordering::Relaxed);
    cq.generation.store(0, Ordering::Relaxed);
    fence(Ordering::Release); // membar_producer()
    let mut stat = cq.stat.get();
    // The C's uint64_t sums are modular, and this one relies on it (see the deviations).
    stat.cs_dispatched = stat.cs_dispatched.wrapping_add(uptime.wrapping_sub(start));
    if run > 0 {
        stat.cs_lateness = stat.cs_lateness.wrapping_add(lateness);
        stat.cs_prompt += 1;
        stat.cs_run += run;
    } else if g.npend != 0 {
        stat.cs_early += 1;
        stat.cs_earliness = stat
            .cs_earliness
            .wrapping_add(g.next().wrapping_sub(uptime));
    } else {
        stat.cs_spurious += 1;
    }
    cq.stat.set(&stat);
    fence(Ordering::Release); // membar_producer()
    cq.generation
        .store(ogen.wrapping_add(1).max(1), Ordering::Relaxed);

    drop(g);

    if cq.dispatching.load(Ordering::Relaxed) != 1 {
        kpanic(format_args!(
            "clockintr_dispatch: unexpected value: {}",
            cq.dispatching.load(Ordering::Relaxed)
        ));
    }
    cq.dispatching.store(0, Ordering::Relaxed);

    run > 0
}

/// `clockintr_advance`: schedules `cl` for the next multiple of `period` after its current
/// expiration that is after now; returns how many periods it was advanced.
pub fn clockintr_advance(cl: Clockintr, period: u64) -> u64 {
    let cq = cl.queue();
    let mut g = cq.q.lock();
    let mut expiration = g.cl[cl.slot.index()].expiration;
    let count = nsec_advance(&mut expiration, period, nsecuptime());
    cq.schedule_locked(&mut g, cl.slot, expiration);
    count
}

/// The running CPU's queue, the one whose request `cr` must be.
fn request_queue(cr: &Clockrequest) -> &'static ClockCpu {
    match cr.queue.get() {
        Some(cq) => cq,
        None => kpanic(format_args!("clockrequest: queue not initialized")),
    }
}

/// `clockrequest_advance`: from a callback, asks to run again a multiple of `period` after
/// the dispatch time; returns how many periods elapsed.
pub fn clockrequest_advance(cr: &Clockrequest, period: u64) -> u64 {
    let cq = request_queue(cr);
    cr.flags.fetch_or(CR_RESCHEDULE, Ordering::Relaxed);
    let mut expiration = cr.expiration.load(Ordering::Relaxed);
    let count = nsec_advance(&mut expiration, period, cq.uptime.load(Ordering::Relaxed));
    cr.expiration.store(expiration, Ordering::Relaxed);
    count
}

/// `clockrequest_advance_random`: as [`clockrequest_advance`], each period `min` plus a
/// random nonzero offset within `mask`.
pub fn clockrequest_advance_random(cr: &Clockrequest, min: u64, mask: u32) -> u64 {
    let cq = request_queue(cr);
    let mut count: u64 = 0;
    let uptime = cq.uptime.load(Ordering::Relaxed);
    let seed = core::cell::Cell::new(cq.randseed.load(Ordering::Relaxed));
    let mut expiration = cr.expiration.load(Ordering::Relaxed);
    while expiration <= uptime {
        let off = loop {
            let off = libkern::random(&seed) & mask;
            if off != 0 {
                break off;
            }
        };
        expiration = expiration.wrapping_add(min + u64::from(off));
        count += 1;
    }
    cq.randseed.store(seed.get(), Ordering::Relaxed);
    cr.expiration.store(expiration, Ordering::Relaxed);
    cr.flags.fetch_or(CR_RESCHEDULE, Ordering::Relaxed);
    count
}

/// `clockintr_cancel`.
pub fn clockintr_cancel(cl: Clockintr) {
    let cq = cl.queue();
    let mut g = cq.q.lock();
    cq.cancel_locked(&mut g, cl.slot);
}

/// `clockintr_bind`: attaches `cl` to its CPU's queue with its callback.
pub fn clockintr_bind(cl: Clockintr, func: ClockintrFn) {
    let cq = cl.queue();

    hal::intr::splassert(hal::intr::IPL_NONE, "clockintr_bind");

    let mut g = cq.q.lock();
    kassert!(g.cl[cl.slot.index()].func.is_none());
    g.cl[cl.slot.index()].func = Some(func);
}

/// `clockintr_unbind`: detaches `cl` from its queue; with `CL_BARRIER`, waits for its
/// running callback to return.
pub fn clockintr_unbind(cl: Clockintr, flags: u32) {
    let cq = cl.queue();

    kassert!(flags & !CL_FLAG_MASK == 0);

    let mut g = cq.q.lock();
    cq.cancel_locked(&mut g, cl.slot);
    g.cl[cl.slot.index()].func = None;

    if flags & CL_BARRIER != 0 && g.running == Some(cl.slot) {
        g.flags |= CQ_NEED_WAKEUP;
        // msleep_nsec(PNORELOCK): the mutex is dropped with the guard once awake.
        let _ = g.sleep_nsec(core::ptr::from_ref(&cq.barrier), PWAIT, "clkbar", INFSLP);
    }
}

/// `clockintr_schedule`: schedules `cl` at the absolute uptime `expiration`.
pub fn clockintr_schedule(cl: Clockintr, expiration: u64) {
    let cq = cl.queue();
    let mut g = cq.q.lock();
    cq.schedule_locked(&mut g, cl.slot, expiration);
}

/// `clockintr_stagger`: sets an unscheduled `cl`'s expiration to `period / denom * numer`.
pub fn clockintr_stagger(cl: Clockintr, period: u64, numer: u32, denom: u32) {
    let cq = cl.queue();

    kassert!(numer < denom);

    let mut g = cq.q.lock();
    if g.cl[cl.slot.index()].pending {
        kpanic(format_args!("clockintr_stagger: clock interrupt pending"));
    }
    g.cl[cl.slot.index()].expiration = period / u64::from(denom) * u64::from(numer);
}

/// `cl->cl_expiration` (under the queue's mutex).
pub fn expiration(cl: Clockintr) -> u64 {
    cl.queue().q.lock().cl[cl.slot.index()].expiration
}

/// `ISSET(cl->cl_flags, CLST_PENDING)` (under the queue's mutex).
pub fn is_pending(cl: Clockintr) -> bool {
    cl.queue().q.lock().cl[cl.slot.index()].pending
}

/// `clockintr_hardclock`: the hardclock's callback, `hardclock()` once per elapsed period.
pub fn clockintr_hardclock(cr: &Clockrequest, frame: &ClockFrame) {
    let count = clockrequest_advance(cr, hardclock_period());
    for _ in 0..count {
        crate::clock::hardclock(frame);
    }
}

/// `clockqueue_init`: readies CPU `cpu`'s queue, once (`primary`: `CPU_IS_PRIMARY(ci)`).
pub fn clockqueue_init(cpu: usize, primary: bool) {
    let Some(cq) = CPUS.cpu(cpu) else {
        kpanic(format_args!("clockqueue_init: no CPU {cpu}"));
    };
    let mut g = cq.q.lock();
    if g.flags & CQ_INIT != 0 {
        return;
    }
    cq.request.queue.set(Some(cq));
    cq.primary.store(primary, Ordering::Relaxed);
    cq.generation.store(1, Ordering::Relaxed);
    g.flags |= CQ_INIT;
    cq.init.store(true, Ordering::Release);
}

/// `clockqueue_intrclock_install`: installs the machine's interrupt clock on CPU `cpu`'s
/// queue, once.
pub fn clockqueue_intrclock_install(cpu: usize, ic: &'static dyn Intrclock) {
    match CPUS.cpu(cpu) {
        Some(cq) => cq.intrclock_install(ic),
        None => kpanic(format_args!("clockqueue_intrclock_install: no CPU {cpu}")),
    }
}

/// `nsec_advance`: advances `*next` in increments of `period` until it exceeds `now`;
/// returns the increments. The common cases first, to avoid the division; no overflow
/// checking.
pub fn nsec_advance(next: &mut u64, period: u64, now: u64) -> u64 {
    if now < *next {
        return 0;
    }

    if now < next.wrapping_add(period) {
        *next = next.wrapping_add(period);
        return 1;
    }

    let elapsed = (now - *next) / period + 1;
    *next = next.wrapping_add(period.wrapping_mul(elapsed));
    elapsed
}

/// `sysctl_clockintr`'s `KERN_CLOCKINTR_STATS`: every initialised queue's statistics,
/// summed, each read whole under its generation.
pub fn stats_sum() -> ClockintrStat {
    let mut sum = ClockintrStat::default();
    for cq in CPUS.iter() {
        if !cq.init.load(Ordering::Acquire) {
            continue;
        }
        let tmp = loop {
            let generation = cq.generation.load(Ordering::Relaxed);
            fence(Ordering::Acquire); // membar_consumer()
            let tmp = cq.stat.get();
            fence(Ordering::Acquire); // membar_consumer()
            if generation != 0 && generation == cq.generation.load(Ordering::Relaxed) {
                break tmp;
            }
        };
        sum.cs_dispatched = sum.cs_dispatched.wrapping_add(tmp.cs_dispatched);
        sum.cs_early = sum.cs_early.wrapping_add(tmp.cs_early);
        sum.cs_earliness = sum.cs_earliness.wrapping_add(tmp.cs_earliness);
        sum.cs_lateness = sum.cs_lateness.wrapping_add(tmp.cs_lateness);
        sum.cs_prompt = sum.cs_prompt.wrapping_add(tmp.cs_prompt);
        sum.cs_run = sum.cs_run.wrapping_add(tmp.cs_run);
        sum.cs_spurious = sum.cs_spurious.wrapping_add(tmp.cs_spurious);
    }
    sum
}

/// `kern.clockintr` (`sysctl_clockintr`): a [`SysctlNode`].
pub struct ClockintrNode;

impl SysctlNode for ClockintrNode {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        sysctl_clockintr(name, req)
    }
}

/// `sysctl_clockintr`: `kern.clockintr.stats`, the CPUs' dispatch statistics summed (a
/// `struct clockintr_stat`), else `EINVAL`.
pub fn sysctl_clockintr(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let [what] = name else {
        return Err(Errno::ENOTDIR);
    };

    match *what {
        KERN_CLOCKINTR_STATS => {
            let sum = stats_sum();
            let mut bytes = [0u8; core::mem::size_of::<ClockintrStat>()];
            let _ = hal::user::pod_to_bytes(&sum, &mut bytes);
            sysctl::rdstruct(req, &bytes)
        }
        _ => Err(Errno::EINVAL),
    }
}

/// `ci_randseed = seed` for CPU `cpu`'s randomized statclock (`bsd`'s `init_main` and the
/// secondary CPUs' start, where they write `ci_randseed`).
pub fn set_randseed(cpu: usize, seed: u32) {
    if let Some(cq) = CPUS.cpu(cpu) {
        cq.randseed.store(seed, Ordering::Relaxed);
    }
}

/// Turns the dispatch's backward-uptime check on (`bsd`'s feature `qemu`, at boot).
pub fn set_uptime_check(on: bool) {
    UPTIME_CHECK.store(on, Ordering::Relaxed);
}

/// Feature `qemu`: how many uptimes the dispatch checked on CPU `cpu`, and how many were
/// behind the previous one.
pub fn uptime_check_counts(cpu: usize) -> (u64, u64) {
    CPUS.cpu(cpu).map_or((0, 0), |cq| {
        (
            cq.uptime_checks.load(Ordering::Relaxed),
            cq.uptime_behind.load(Ordering::Relaxed),
        )
    })
}

const _: () = assert!(core::mem::size_of::<ClockintrStat>() == 56);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Each test acts as a CPU of its own (`TEST_CPU`), whose queue nothing else uses, over the
    // dummy timecounter (which advances on every read) or a test counter.

    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::AtomicU32 as StdU32;

    use super::*;
    use crate::testutil;

    /// The queues of CPUs 200.. are the tests'; a test takes one number.
    fn as_cpu(n: usize) {
        testutil::install();
        TEST_CPU.with(|c| c.set(Some(n)));
        clockqueue_init(n, false);
    }

    struct Fake {
        rearmed: AtomicU64,
        triggered: StdU32,
    }

    impl Intrclock for Fake {
        fn rearm(&self, nsecs: u64) {
            self.rearmed.store(nsecs, Ordering::Relaxed);
        }
        fn trigger(&self) {
            self.triggered.fetch_add(1, Ordering::Relaxed);
        }
    }

    static FAKE: Fake = Fake {
        rearmed: AtomicU64::new(0),
        triggered: StdU32::new(0),
    };

    static RUNS: StdU32 = StdU32::new(0);

    fn once(_cr: &Clockrequest, _f: &ClockFrame) {
        RUNS.fetch_add(1, Ordering::Relaxed);
    }

    fn again(cr: &Clockrequest, _f: &ClockFrame) {
        RUNS.fetch_add(1, Ordering::Relaxed);
        clockrequest_advance(cr, 1_000_000);
    }

    fn frame() -> ClockFrame {
        ClockFrame::default()
    }

    #[test]
    fn nsec_advance_counts_periods() {
        let mut next = 100;
        assert_eq!(nsec_advance(&mut next, 10, 50), 0);
        assert_eq!(next, 100);
        assert_eq!(nsec_advance(&mut next, 10, 105), 1);
        assert_eq!(next, 110);
        assert_eq!(nsec_advance(&mut next, 10, 155), 5);
        assert_eq!(next, 160);
        assert_eq!(nsec_advance(&mut next, 10, 160), 1, "now == next is due");
        assert_eq!(next, 170);
    }

    #[test]
    fn bind_schedule_dispatch_cancel() {
        as_cpu(200);
        clockqueue_intrclock_install(200, &FAKE);
        let a = Clockintr::of(200, Slot::Itimer);
        let b = Clockintr::of(200, Slot::Profclock);
        clockintr_bind(a, once);
        clockintr_bind(b, again);
        let base = RUNS.load(Ordering::Relaxed);

        // Both due now: `0` is in the past.
        let triggered = FAKE.triggered.load(Ordering::Relaxed);
        clockintr_schedule(a, 0);
        clockintr_schedule(b, 0);
        assert!(
            FAKE.triggered.load(Ordering::Relaxed) > triggered,
            "due now: triggered"
        );
        assert_eq!(this().q.lock().npend, 2);

        let _s = hal::intr::splraise(IPL_CLOCK);
        assert!(clockintr_dispatch(&frame()));
        assert_eq!(RUNS.load(Ordering::Relaxed), base + 2);
        // `a` ran and is idle; `b` rescheduled itself 1 ms after its dispatch time.
        assert_eq!(this().q.lock().first(), Some(Slot::Profclock));
        assert_eq!(this().q.lock().npend, 1);
        assert!(FAKE.rearmed.load(Ordering::Relaxed) > 0, "rearmed for b");
        let stat = this().stat.get();
        assert_eq!(stat.cs_run, 2);
        assert_eq!(stat.cs_prompt, 1);

        // Too early: nothing runs, the clock is rearmed, cs_early counts.
        assert!(!clockintr_dispatch(&frame()));
        assert_eq!(this().stat.get().cs_early, 1);

        // Advanced by a period from now: at least one period, pending in the future.
        assert!(clockintr_advance(a, 5_000_000) >= 1);
        assert_eq!(this().q.lock().npend, 2);
        clockintr_cancel(a);
        assert!(!is_pending(a));
        clockintr_cancel(b);
        assert_eq!(this().q.lock().npend, 0);
        assert!(!clockintr_dispatch(&frame()));
        assert_eq!(this().stat.get().cs_spurious, 1);

        // The statistics summed over the CPUs include this queue's.
        let sum = stats_sum();
        assert!(sum.cs_run >= 2 && sum.cs_spurious >= 1 && sum.cs_early >= 1);

        clockintr_unbind(a, 0);
        clockintr_unbind(b, 0);
        assert!(this().q.lock().cl[Slot::Itimer.index()].func.is_none());
    }

    #[test]
    fn equal_expirations_keep_their_insertion_order() {
        as_cpu(201);
        let mut q = Queue::new();
        q.pend_insert(Slot::Statclock, 50);
        q.pend_insert(Slot::Itimer, 10);
        q.pend_insert(Slot::Profclock, 50);
        q.pend_insert(Slot::Roundrobin, 30);
        assert_eq!(
            &q.pend[..q.npend],
            &[
                Slot::Itimer,
                Slot::Roundrobin,
                Slot::Statclock,
                Slot::Profclock
            ]
        );
        q.pend_delete(Slot::Roundrobin);
        assert_eq!(
            &q.pend[..q.npend],
            &[Slot::Itimer, Slot::Statclock, Slot::Profclock]
        );
        assert_eq!(q.next(), 10);
        assert!(!q.cl[Slot::Roundrobin.index()].pending);
    }

    #[test]
    fn stagger_sets_an_offset_without_scheduling() {
        as_cpu(202);
        let cl = Clockintr::of(202, Slot::Roundrobin);
        clockintr_bind(cl, once);
        clockintr_stagger(cl, 10_000_000, 3, 4);
        assert_eq!(expiration(cl), 7_500_000);
        assert!(!is_pending(cl));
        clockintr_unbind(cl, 0);
    }

    #[test]
    fn advance_random_stays_within_the_mask() {
        as_cpu(203);
        let cq = this();
        let cr = &cq.request;
        cq.uptime.store(1_000_000, Ordering::Relaxed);
        cr.expiration.store(0, Ordering::Relaxed);
        cr.flags.store(0, Ordering::Relaxed);
        set_randseed(203, 12345);
        let count = clockrequest_advance_random(cr, 100, 0xff);
        assert!(count >= 1);
        assert!(cr.expiration() > 1_000_000);
        assert!(cr.expiration() <= 1_000_000 + 100 + 0xff);
        assert!(cr.flags() & CR_RESCHEDULE != 0);
        assert_ne!(cq.randseed.load(Ordering::Relaxed), 12345, "the seed moved");
        // Every period is min plus 1..=mask (a zero offset is drawn again): 35600 ns take
        // between 35600 / 355 and 35600 / 101 + 1 of them.
        cr.expiration.store(1_000_000, Ordering::Relaxed);
        cq.uptime.store(1_000_000 + 35_600, Ordering::Relaxed);
        let n = clockrequest_advance_random(cr, 100, 0xff);
        assert!((100..=353).contains(&n), "{n} periods");
        assert!(cr.expiration() > 1_035_600);
        cr.flags.store(0, Ordering::Relaxed);
    }

    static BARRIER_IN: AtomicBool = AtomicBool::new(false);
    static BARRIER_OUT: AtomicBool = AtomicBool::new(false);
    static GATE: StdMutex<()> = StdMutex::new(());

    fn slow(_cr: &Clockrequest, _f: &ClockFrame) {
        BARRIER_IN.store(true, Ordering::SeqCst);
        // Held until the unbinding thread has asked for the barrier.
        drop(GATE.lock().unwrap_or_else(|e| e.into_inner()));
        BARRIER_OUT.store(true, Ordering::SeqCst);
    }

    #[test]
    fn unbind_with_the_barrier_waits_for_the_running_callback() {
        as_cpu(204);
        let cl = Clockintr::of(204, Slot::Statclock);
        clockintr_bind(cl, slow);
        clockintr_schedule(cl, 0);
        let gate = GATE.lock().unwrap_or_else(|e| e.into_inner());
        std::thread::scope(|s| {
            s.spawn(|| {
                TEST_CPU.with(|c| c.set(Some(204)));
                let _s = hal::intr::splraise(IPL_CLOCK);
                assert!(clockintr_dispatch(&frame()));
            });
            while !BARRIER_IN.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
            let unbinder = s.spawn(|| {
                clockintr_unbind(cl, CL_BARRIER);
                // The barrier returned: the callback has.
                assert!(BARRIER_OUT.load(Ordering::SeqCst));
            });
            // The unbinder marks the queue and waits; let the callback finish.
            while this().q.lock().flags & CQ_NEED_WAKEUP == 0 {
                std::thread::yield_now();
            }
            drop(gate);
            assert!(unbinder.join().is_ok());
        });
        assert!(this().q.lock().cl[Slot::Statclock.index()].func.is_none());
    }

    #[test]
    fn kern_clockintr_stats_is_56_bytes_of_sums() {
        as_cpu(206);
        let u = testutil::USER_BASE + 0xCA00;
        let mut len = 0;
        let mut q = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(
            ClockintrNode.sysctl(&[KERN_CLOCKINTR_STATS], &mut q),
            Ok(())
        );
        assert_eq!(*q.oldlenp, 56);
        assert_eq!(sysctl_clockintr(&[2], &mut q), Err(Errno::EINVAL));
        assert_eq!(sysctl_clockintr(&[1, 1], &mut q), Err(Errno::ENOTDIR));
        let mut len = 56;
        let mut r = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(sysctl_clockintr(&[KERN_CLOCKINTR_STATS], &mut r), Ok(()));
        let got: Option<ClockintrStat> =
            testutil::user_bytes(u, 56).and_then(|b| hal::user::pod_from_bytes(&b));
        assert!(got.is_some());
    }

    #[test]
    fn the_uptime_check_counts_and_reports_a_step_back() {
        as_cpu(205);
        let cq = this();
        set_uptime_check(true);
        cq.uptime_check(1000);
        cq.uptime_check(2000);
        cq.uptime_check(1500);
        set_uptime_check(false);
        assert_eq!(uptime_check_counts(205), (3, 1));
        assert!(testutil::printf_count("uptime went backwards by 500 ns (2000 -> 1500)") >= 1);
    }
}
/* </TESTS> */
