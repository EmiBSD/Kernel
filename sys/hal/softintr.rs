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
 * Copyright (c) 2021, 2025 Visa Hankala
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
//! Machine-independent soft interrupts: `kern/kern_softintr.c` (`__USE_MI_SOFTINTR`):
//! `softintr_establish(9)`, `softintr_schedule(9)`, `softintr_disestablish(9)` and the
//! dispatch of a soft level's queue, which `crate::intr`'s `do_pending_intr` runs when the
//! level drops below it.
//!
//! Upstream: sys/kern/kern_softintr.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_softintr.rs@3c62ede68802
//!
//! A handler is a function and its argument (`fn(*mut c_void)` and a `void *`, the C's
//! `sih_fn`/`sih_arg`): a callback, not an operations table, so it stays a function pointer
//! (decision 46's written reason). A handler established without `IPL_MPSAFE` runs under the
//! kernel lock (`SIF_MPSAFE` unset), as `softclock`, `comsoft` and `pluart_softint` do.
//!
//! **Handles.** [`establish`] gives a [`SoftHandle`]: the handler's entry (a `malloc(9)` block
//! of `M_DEVBUF`, as the C's) and its generation. An entry is never freed: one disestablished
//! (`softintr_disestablish`, in two steps here, [`SoftHandle::disestablish`] then
//! [`SoftHandle::release`], with the caller's `sched_barrier` between) goes on a free list
//! and serves the next [`establish`], its generation bumped, so a handle kept past its
//! disestablishment names an entry that still exists and is refused by the generation
//! (`panic(9)`, where the C would use freed memory). The handle is `Copy`, as the C's
//! `void *`, so the drivers keep it in a softc cell as before.
//!
//! ## Deviations
//! - A disestablished handler's entry is kept for the next establishment instead of being
//!   freed (at most [`FREE_SLOTS`] kept; one more is left allocated, unused): `vmstat -m`'s
//!   `devbuf` row counts the peak of established handlers, the C's the current number. Only
//!   a detach disestablishes (`com`, `usb`).
//! - The runner (`sih_runner`, the CPU running the handler) is the CPU's number, compared,
//!   where the C keeps the `cpu_info` pointer.
//! - `softintr_disestablish`'s `sched_barrier(runner)` is the caller's, between the two
//!   steps: the scheduler is the kernel's (`kernel::sched`), above `hal`.
//!
//! ## Redesign
//! - LZ's queues were `TAILQ`s of `malloc`ed handlers under an `unsafe impl Sync` saying
//!   "`softintr_lock`", the handle a `NonNull` dereferenced by every function (10 `unsafe`):
//!   the queues are pinned lists bound to `softintr_lock` ([`SOFTINTR_LOCK`]), a handler's
//!   mutable members a cell of that lock, and a handle cannot outlive its entry.
//! - N4.1's `SoftIntr` bridge (the kernel's `fn()` handlers over `bsd`'s queues) is gone:
//!   the kernel and `bsd` establish here.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::assert::hal_panic;
use crate::intr::{IPL_HIGH, IPL_MPSAFE, IPL_SOFTCLOCK, IPL_SOFTNET, IPL_SOFTTTY, IPL_TTY};
use crate::malloc::{M_NOWAIT, MBox};
use crate::mutex::Mutexed;
use crate::once::OnceStatic;
use crate::pin::pin_static;
use crate::pinlist::{PinEntry, PinList};
use crate::rwlock::Guarded;
use crate::sys::softintr::{NSOFTINTR, SOFTINTR_CLOCK, SOFTINTR_NET, SOFTINTR_TTY};
use crate::vm::uvmexp::UVMEXP;

/// `M_DEVBUF` (`<sys/malloc.h>`): device driver memory, where the C allocates a handler.
const M_DEVBUF: i32 = 2;

/// A soft interrupt handler's function (`void (*)(void *)`).
pub type SoftintrFn = fn(*mut c_void);

/// `SIF_MPSAFE`: the handler runs without the kernel lock.
pub const SIF_MPSAFE: u16 = 0x0001;

/// `SIS_DYING`: being disestablished.
pub const SIS_DYING: u16 = 0x0001;
/// `SIS_PENDING`: queued.
pub const SIS_PENDING: u16 = 0x0002;
/// `SIS_RESTART`: scheduled again while running.
pub const SIS_RESTART: u16 = 0x0004;

/// How many disestablished entries the free list keeps for reuse.
pub const FREE_SLOTS: usize = 32;

/// A handler's members under `softintr_lock` (`[S]`).
#[derive(Clone, Copy)]
struct Hand {
    /// `sih_fn`.
    func: SoftintrFn,
    /// `sih_arg`, as an address with its provenance exposed (handed back to `func`).
    arg: usize,
    /// `sih_level`: `SOFTINTR_*`.
    level: usize,
    /// `sih_flags`: `SIF_*`.
    flags: u16,
    /// `sih_state`: `SIS_*`.
    state: u16,
    /// `sih_runner`: the number of the CPU running the handler, plus one; 0 for none.
    runner: usize,
}

/// `struct softintr_hand`: an established soft interrupt handler's entry.
pub struct SoftintrHand {
    /// `[S]` `sih_q`: its level's queue, while pending.
    sih_q: PinEntry<SoftintrHand>,
    /// The establishment the entry serves: bumped when it is released.
    generation: AtomicU32,
    /// `[S]` the handler.
    hand: Guarded<Hand>,
    /// The entry itself, for ever (it is never freed): what a walk of a queue, whose
    /// references end with the lock's guard, keeps across the handler's run.
    me: OnceStatic<&'static SoftintrHand>,
}

crate::pin_adapter!(
    /// `TAILQ_HEAD(softintr_queue, softintr_hand)`.
    SihQ: SoftintrHand, sih_q
);

/// `softintr_lock`, and the free entries it keeps for reuse (a stack).
pub static SOFTINTR_LOCK: Mutexed<FreeEntries> = Mutexed::new(IPL_HIGH, FreeEntries::new());

/// `softintr_queue[NSOFTINTR]`: one queue per level, bound to [`SOFTINTR_LOCK`].
static SOFTINTR_QUEUE: [PinList<SihQ>; NSOFTINTR] = [const { PinList::new() }; NSOFTINTR];

/// The disestablished entries waiting for the next establishment.
pub struct FreeEntries {
    slots: [Option<&'static SoftintrHand>; FREE_SLOTS],
    n: usize,
}

impl FreeEntries {
    /// None.
    const fn new() -> Self {
        Self {
            slots: [None; FREE_SLOTS],
            n: 0,
        }
    }
}

/// An established handler (`void *` of `softintr_establish`): its entry and the
/// establishment it names.
#[derive(Clone, Copy)]
pub struct SoftHandle {
    hand: &'static SoftintrHand,
    generation: u32,
}

impl core::fmt::Debug for SoftHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "SoftHandle({:#x}, {})",
            ptr::from_ref(self.hand).addr(),
            self.generation
        )
    }
}

/// The level that serves `ipl`, `None` for a level that has none (the C's `panic`).
fn level_of(ipl: i32) -> Option<usize> {
    if ipl == IPL_SOFTCLOCK {
        Some(SOFTINTR_CLOCK as usize)
    } else if ipl == IPL_SOFTNET {
        Some(SOFTINTR_NET as usize)
    } else if ipl == IPL_TTY || ipl == IPL_SOFTTTY {
        Some(SOFTINTR_TTY as usize)
    } else {
        None
    }
}

/// `softintr_init()`: the queues' `TAILQ_INIT`. A [`PinList`] is an empty head from `new`,
/// so what is left is the C's premise, checked: no handler is queued before the machine
/// starts dispatching (`panic(9)` otherwise).
pub fn init() {
    let g = SOFTINTR_LOCK.lock();
    if SOFTINTR_QUEUE.iter().any(|q| !q.is_empty(&g)) {
        hal_panic(format_args!("softintr_init: a handler queued before init"));
    }
    drop(g);
}

/// `softintr_establish(ipl, func, arg)`: `func(arg)` runs at the soft level that serves
/// `ipl` (without the kernel lock when `ipl` carries `IPL_MPSAFE`) each time the handle is
/// scheduled; `None` when no memory can be had (`M_NOWAIT`, the C's `NULL`). A level that no
/// soft interrupt serves is `panic(9)`, as the C's.
pub fn establish(ipl: i32, func: SoftintrFn, arg: *mut c_void) -> Option<SoftHandle> {
    let mut flags = 0;
    if ipl & IPL_MPSAFE != 0 {
        flags |= SIF_MPSAFE;
    }
    let ipl = ipl & !IPL_MPSAFE;

    // IPL_SOFT: not defined on amd64 or arm64.
    let Some(level) = level_of(ipl) else {
        hal_panic(format_args!("softintr_establish: unhandled ipl {ipl}"));
    };
    let hand = Hand {
        func,
        arg: arg.expose_provenance(),
        level,
        flags,
        state: 0,
        runner: 0,
    };

    let mut g = SOFTINTR_LOCK.lock();
    if g.n > 0 {
        g.n -= 1;
        let i = g.n;
        if let Some(e) = g.slots[i].take() {
            *e.hand.get_mut(&mut g) = hand;
            let generation = e.generation.load(Ordering::Relaxed);
            return Some(SoftHandle {
                hand: e,
                generation,
            });
        }
    }
    drop(g);

    let entry = MBox::new(
        M_DEVBUF,
        M_NOWAIT,
        SoftintrHand {
            sih_q: PinEntry::new(),
            generation: AtomicU32::new(0),
            hand: Guarded::new_static_mutexed(&SOFTINTR_LOCK, hand),
            me: OnceStatic::new(),
        },
    )?;
    let e: &'static SoftintrHand = MBox::leak(entry);
    let _ = e.me.set(e);
    Some(SoftHandle {
        hand: e,
        generation: 0,
    })
}

impl SoftHandle {
    /// The handler, for the holder of `softintr_lock` (`g`): `panic(9)` for a handle whose
    /// establishment was released.
    fn check(&self, g: &crate::mutex::MutexedGuard<'_, FreeEntries>) -> Hand {
        if self.hand.generation.load(Ordering::Relaxed) != self.generation {
            hal_panic(format_args!(
                "softintr: a handle used after softintr_disestablish"
            ));
        }
        *self.hand.hand.get(g)
    }

    /// `softintr_schedule(sih)`: queues the handler (or marks it to run again if it is
    /// running) and raises the soft interrupt on this CPU.
    pub fn schedule(self) {
        let mut g = SOFTINTR_LOCK.lock();
        let h = self.check(&g);
        crate::assert::kassert!(h.state & SIS_DYING == 0);
        if h.runner == 0 {
            crate::assert::kassert!(h.state & SIS_RESTART == 0);
            if h.state & SIS_PENDING == 0 {
                SOFTINTR_QUEUE[h.level].insert_tail(&mut g, pin_static(self.hand));
                self.hand.hand.get_mut(&mut g).state |= SIS_PENDING;
                // Call softintr() while SPL is still at IPL_HIGH.
                crate::intr::softintr(h.level);
            }
        } else {
            crate::assert::kassert!(h.state & SIS_PENDING == 0);
            self.hand.hand.get_mut(&mut g).state |= SIS_RESTART;
        }
        drop(g);
    }

    /// `softintr_disestablish(sih)`, its first step: the handler is dying, taken off its
    /// queue if pending and never restarted; the number of the CPU running it now, if one
    /// is, which the caller waits for (`sched_barrier`) before [`SoftHandle::release`].
    pub fn disestablish(self) -> Option<usize> {
        let mut g = SOFTINTR_LOCK.lock();
        let h = self.check(&g);
        if h.state & SIS_PENDING != 0 {
            SOFTINTR_QUEUE[h.level].remove(&mut g, self.hand);
        }
        let hand = self.hand.hand.get_mut(&mut g);
        hand.state |= SIS_DYING;
        hand.state &= !(SIS_RESTART | SIS_PENDING);
        let runner = hand.runner;
        drop(g);
        runner.checked_sub(1)
    }

    /// `softintr_disestablish(sih)`, its second step (after the caller's `sched_barrier`):
    /// the entry released (`free`): kept for the next [`establish`], this handle's
    /// establishment gone (any later use of it is `panic(9)`).
    pub fn release(self) {
        let mut g = SOFTINTR_LOCK.lock();
        let h = self.check(&g);
        crate::assert::kassert!(h.state & (SIS_PENDING | SIS_RESTART) == 0);
        crate::assert::kassert!(h.runner == 0);
        self.hand.generation.fetch_add(1, Ordering::Relaxed);
        if g.n < FREE_SLOTS {
            let i = g.n;
            g.slots[i] = Some(self.hand);
            g.n += 1;
        }
        drop(g);
    }
}

/// `softintr_dispatch(level)`: runs the handlers pending at `level` (`SOFTINTR_*`), from
/// `crate::intr`'s `do_pending_intr`, each with `softintr_lock` left, under the kernel lock
/// unless it is `MPSAFE`; one scheduled again while it ran is queued again.
pub fn dispatch(level: usize) {
    let Some(queue) = SOFTINTR_QUEUE.get(level) else {
        return;
    };
    let me = crate::cpu::cpu_number() + 1;

    let mut g = SOFTINTR_LOCK.lock();
    // The first entry, as its own `&'static` (the list's reference ends with the guard).
    while let Some(sih) = queue.first(&g).and_then(|e| e.me.get().copied()) {
        let h = *sih.hand.get(&g);
        crate::assert::kassert!(h.state & (SIS_PENDING | SIS_RESTART) == SIS_PENDING);
        crate::assert::kassert!(h.runner == 0);

        queue.remove(&mut g, sih);
        {
            let hand = sih.hand.get_mut(&mut g);
            hand.state &= !SIS_PENDING;
            hand.runner = me;
        }
        drop(g);

        let arg = ptr::with_exposed_provenance_mut::<c_void>(h.arg);
        if h.flags & SIF_MPSAFE != 0 {
            (h.func)(arg);
        } else {
            crate::klock::kernel_lock(); // KERNEL_LOCK()
            (h.func)(arg);
            crate::klock::kernel_unlock(); // KERNEL_UNLOCK()
        }

        g = SOFTINTR_LOCK.lock();
        let hand = sih.hand.get_mut(&mut g);
        crate::assert::kassert!(hand.state & SIS_PENDING == 0);
        hand.runner = 0;
        if hand.state & SIS_RESTART != 0 {
            hand.state |= SIS_PENDING;
            hand.state &= !SIS_RESTART;
            queue.insert_tail(&mut g, pin_static(sih));
        }

        UVMEXP.softs.fetch_add(1, Ordering::Relaxed);
    }
    drop(g);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicUsize;

    use super::*;
    use crate::testutil;

    /// A handler: adds its argument's address (a small number) to `COUNTS`.
    fn bump(arg: *mut c_void) {
        COUNTS.fetch_add(arg.addr() & 0xff, Ordering::Relaxed);
    }

    static COUNTS: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn a_scheduled_handler_runs_once_at_its_level() {
        testutil::install();
        let si = establish(
            IPL_SOFTCLOCK | IPL_MPSAFE,
            bump,
            ptr::without_provenance_mut(1),
        )
        .unwrap();
        {
            // Above the soft levels nothing runs when `softintr_lock` is left.
            let _s = crate::intr::splhigh();
            let before = COUNTS.load(Ordering::Relaxed);
            si.schedule();
            // Twice before it runs: queued once.
            si.schedule();
            {
                let g = SOFTINTR_LOCK.lock();
                assert!(si.check(&g).state & SIS_PENDING != 0);
            }
            assert_eq!(COUNTS.load(Ordering::Relaxed), before);
            dispatch(SOFTINTR_CLOCK as usize);
            assert_eq!(COUNTS.load(Ordering::Relaxed), before + 1);
            // Nothing left on the queue.
            let g = SOFTINTR_LOCK.lock();
            assert!(si.check(&g).state & SIS_PENDING == 0);
        }
        assert_eq!(si.disestablish(), None);
        si.release();
    }

    #[test]
    fn a_handler_scheduled_at_ipl_none_runs_when_the_lock_is_left() {
        testutil::install();
        let si = establish(
            IPL_SOFTNET | IPL_MPSAFE,
            bump,
            ptr::without_provenance_mut(4),
        )
        .unwrap();
        let _ = crate::intr::spllower(crate::intr::IPL_NONE);
        crate::intr::init_smask();
        let before = COUNTS.load(Ordering::Relaxed);
        // `mtx_leave(&softintr_lock)` lowers the level below IPL_SOFTNET: the C's splx runs
        // the pending soft interrupt there and then.
        si.schedule();
        assert_eq!(COUNTS.load(Ordering::Relaxed), before + 4);
        assert_eq!(si.disestablish(), None);
        si.release();
    }

    #[test]
    fn a_disestablished_pending_handler_leaves_its_queue_and_its_entry_is_reused() {
        testutil::install();
        let si = establish(IPL_TTY, bump, ptr::null_mut()).unwrap();
        let s = crate::intr::splhigh();
        si.schedule();
        {
            let g = SOFTINTR_LOCK.lock();
            assert!(si.check(&g).state & SIS_PENDING != 0);
        }
        assert_eq!(si.disestablish(), None);
        drop(s);
        {
            let g = SOFTINTR_LOCK.lock();
            let h = si.check(&g);
            assert!(h.state & SIS_DYING != 0);
            assert!(h.state & SIS_PENDING == 0);
        }
        si.release();
        // The next establishment may take the entry back, under a new generation.
        let again = establish(IPL_SOFTNET | IPL_MPSAFE, bump, ptr::null_mut()).unwrap();
        if ptr::eq(again.hand, si.hand) {
            assert_eq!(again.generation, si.generation + 1);
        }
        assert_eq!(again.disestablish(), None);
        again.release();
    }

    #[test]
    #[should_panic(expected = "used after softintr_disestablish")]
    fn a_handle_used_after_its_release_is_refused() {
        testutil::install();
        let si = establish(IPL_SOFTNET | IPL_MPSAFE, bump, ptr::null_mut()).unwrap();
        assert_eq!(si.disestablish(), None);
        si.release();
        si.schedule();
    }

    #[test]
    #[should_panic(expected = "unhandled ipl")]
    fn a_level_without_a_soft_interrupt_is_refused() {
        testutil::install();
        let _ = establish(IPL_HIGH, bump, ptr::null_mut());
    }

    #[test]
    fn the_levels_map_to_the_soft_queues() {
        assert_eq!(level_of(IPL_SOFTCLOCK), Some(SOFTINTR_CLOCK as usize));
        assert_eq!(level_of(IPL_SOFTNET), Some(SOFTINTR_NET as usize));
        assert_eq!(level_of(IPL_SOFTTTY), Some(SOFTINTR_TTY as usize));
        assert_eq!(level_of(IPL_TTY), Some(SOFTINTR_TTY as usize));
        assert_eq!(level_of(IPL_HIGH), None);
    }
}
/* </TESTS> */
