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
 * Copyright (c) 2000 Poul-Henning Kamp <phk@FreeBSD.org>
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
 * If we meet some day, and you think this stuff is worth it, you
 * can buy me a beer in return. Poul-Henning Kamp
 */

/*
 * Copyright (c) 2000 Poul-Henning Kamp <phk@FreeBSD.org>
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
 * If we meet some day, and you think this stuff is worth it, you
 * can buy me a beer in return. Poul-Henning Kamp
 */
/* </LICENSES> */

/* <CODE> */
//! Timecounters: the kernel's notion of time (`timecounters(9)`). A ring of two timehands
//! holds the offset the active hardware counter is read against; `tc_windup` advances it
//! every `tc_tick` hardclocks, and the `[get]{bin,nano,micro}{up,boot,run}time()` readers read
//! it without a lock, retrying while its generation is 0 or changed under them.
//!
//! Upstream: sys/kern/kern_tc.c @ 3ce1f3f79392
//! Upstream: sys/sys/timetc.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_tc.rs@3c62ede68802
//! LZ: sys/sys/timetc.rs@3c62ede68802
//!
//! Locks: `[W]` `windup_mtx` ([`WINDUP`], `IPL_CLOCK`; its guard is what [`tc_windup`]
//! takes), `[T]` `tc_lock` ([`TC_LOCK`], an rwlock: the timecounter choice and the clock
//! settings; `kern_time.c` takes it around `tc_adjfreq`/`tc_adjtime`), `[I]` set before the
//! counter is shared. Order: `tc_lock`, then `windup_mtx`.
//!
//! ## Deviations
//! - `getuptime`/`gettime` take the `__LP64__` branch (both machines are LP64).
//! - `membar_consumer`/`membar_producer` are `fence(Acquire)`/`fence(Release)`.
//! - `sysctl_tc_choice`'s `malloc(M_TEMP)` buffer is the stack's: at most 32 bytes per
//!   counter, written into the caller's buffer as the C's `strlcat`s would.
//!
//! ## Redesign
//! - `struct timecounter` (one function pointer, its `tc_priv` cookie, five fields the driver
//!   fills and four the kernel owns, `Cell`s behind `unsafe impl Sync`) is split in two
//!   (decision 46): the hardware's half is the [`Timecounter`] trait (`get_timecount`,
//!   `counter_mask`, `frequency`, `name`, `quality`, `user`; `tc_priv` is `self`), the
//!   kernel's half the [`TcEntry`] the driver hands to [`tc_init`]: the `[I]` values copied
//!   from the hardware once, the quality, precision and frequency adjustment as atomics, the
//!   list link. `tc_delta` loads what the C loads (`th_counter`, the mask, `th_offset_count`)
//!   plus one vtable slot for the read.
//! - `struct timehands` was `Cell`s behind `unsafe impl Sync` and two `AtomicPtr`s
//!   (`timehands`, `timecounter`) dereferenced in `unsafe` blocks. Here every field is an
//!   atomic (a `bintime` two words, a `timeval`/`timespec` two), so a reader racing the
//!   writer reads stale words but never a data race, and the generation protocol is the C's
//!   (`th_generation = 0`, release fence, the fields, release fence, the new generation;
//!   readers: the generation, acquire fence, the fields, acquire fence, the generation
//!   again). `th_next` is the ring's other index ([`TH`], [`ACTIVE`]); `th_counter` and
//!   `timecounter` are `hal::link::Link`s to `&'static` entries. `memcpy(th, tho, ...)` is a
//!   field-by-field copy (`Timehands::copy_from`).
//! - `windup_mtx` is [`WINDUP`], a `Mutexed<Windup>` whose guard [`tc_windup`] takes: the C's
//!   `MUTEX_ASSERT_LOCKED` becomes the parameter, and the `[W]` state the C keeps in statics
//!   (`tc_update_timekeep`'s `last_tc`) lives in it. The `[T,W]` changes ([`tc_windup`]'s new
//!   boot time and adjustment, `tc_adjfreq`'s and `tc_adjtime`'s setters) take a witness of
//!   [`TC_LOCK`], checked as the C's `rw_assert_wrlock`.
//! - The timekeep page is written through `hal::sys::timetc::TimekeepAtomic` ([`TIMEKEEP`],
//!   attached once by whoever maps it), the C's write order kept; LZ wrote it through a raw
//!   `&mut` of `kern_exec`'s pointer.
//! - `tc_list` is a `hal::list::List` of `&'static TcEntry`, newest first as the C's SLIST.
//! - The `log(LOG_INFO, ...)` of a clock step and `enqueue_randomness` reach `bsd` through
//!   the `Printf` and `Random` bridges (`hal::prf::log`, `hal::random::enqueue_randomness`).

use core::sync::atomic::{
    AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering, fence,
};

use hal::intr::IPL_CLOCK;
use hal::link::Link;
use hal::list::{List, ListEntry};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::rwlock::{RW_WRITE, RawRwlock, RwWitness, Witness, WriteWitness};
pub use hal::sys::timetc::{TK_VERSION, Timekeep, TimekeepAtomic};

use crate::assert::{kassert, kpanic};
use crate::clock::{Bintime, Timespec, Timeval};
use crate::prf::printf;

/// `LOG_INFO` (`<sys/syslog.h>`): the priority of a clock step's log line.
const LOG_INFO: i32 = 6;

/// The `get_timecount` half of `struct timecounter`: what a counter's driver implements
/// (`tc_priv` is `self`). The values are read once, by [`tc_init`] (`[I]` in the C).
pub trait Timecounter: Sync + 'static {
    /// `tc_get_timecount(tc)`: reads the counter. Unimplemented bits need not be masked,
    /// as long as they are constant.
    fn get_timecount(&self) -> u32;
    /// `tc_counter_mask`: masks off the unimplemented bits.
    fn counter_mask(&self) -> u32;
    /// `tc_frequency`: the counter's frequency in Hz (an attach-time value on arm64).
    fn frequency(&self) -> u64;
    /// `tc_name`.
    fn name(&self) -> &'static str;
    /// `tc_quality`, the driver's initial value: higher is better, negative means "only at
    /// explicit request".
    fn quality(&self) -> i32;
    /// `tc_user`: how userland reads this counter itself (`TC_AGTIMER`, ...), 0 for not at
    /// all.
    fn user(&self) -> i32 {
        0
    }
}

/// The kernel's half of `struct timecounter`: one per counter, a `static` of the driver's,
/// registered by [`tc_init`].
///
/// - `[I]` (copied from the hardware by `tc_init`): `mask`, `frequency` (the driver may set
///   it again, [`TcEntry::set_frequency`]: agtimer's clock-rate change), `user`.
/// - `[T]`: `quality` (`tc_init`, `tc_reset_quality`), `precision` (`tc_init`).
/// - `[T,W]`: `freq_adj`.
pub struct TcEntry {
    hw: &'static dyn Timecounter,
    mask: AtomicU32,
    frequency: AtomicU64,
    user: AtomicI32,
    quality: AtomicI32,
    precision: AtomicU64,
    freq_adj: AtomicI64,
    link: ListEntry<TcEntry>,
}

impl TcEntry {
    /// The entry of the counter `hw`, its values not yet read (`tc_init` reads them).
    pub const fn new(hw: &'static dyn Timecounter) -> Self {
        Self {
            hw,
            mask: AtomicU32::new(0),
            frequency: AtomicU64::new(0),
            user: AtomicI32::new(0),
            quality: AtomicI32::new(0),
            precision: AtomicU64::new(0),
            freq_adj: AtomicI64::new(0),
            link: ListEntry::new(),
        }
    }

    /// `tc_name`.
    pub fn name(&self) -> &'static str {
        self.hw.name()
    }

    /// `tc_quality` as the kernel keeps it.
    pub fn quality(&self) -> i32 {
        self.quality.load(Ordering::Relaxed)
    }

    /// `tc_frequency`.
    pub fn frequency(&self) -> u64 {
        self.frequency.load(Ordering::Relaxed)
    }

    /// `tc_frequency = f`: the driver's new counter rate (`agtimer_set_clockrate`); the next
    /// windup scales with it.
    pub fn set_frequency(&self, f: u64) {
        self.frequency.store(f, Ordering::Relaxed);
    }

    /// `tc_precision`.
    pub fn precision(&self) -> u64 {
        self.precision.load(Ordering::Relaxed)
    }

    /// `tc_counter_mask`.
    pub fn counter_mask(&self) -> u32 {
        self.mask.load(Ordering::Relaxed)
    }

    /// `tc_user`.
    pub fn user(&self) -> i32 {
        self.user.load(Ordering::Relaxed)
    }

    /// `tc->tc_get_timecount(tc)`.
    #[inline(always)]
    pub fn get_timecount(&self) -> u32 {
        self.hw.get_timecount()
    }

    /// Copies the hardware's `[I]` values (`tc_init`'s first act; the dummy's at build).
    fn load_from_hw(&self) {
        self.mask.store(self.hw.counter_mask(), Ordering::Relaxed);
        self.frequency.store(self.hw.frequency(), Ordering::Relaxed);
        self.user.store(self.hw.user(), Ordering::Relaxed);
        self.quality.store(self.hw.quality(), Ordering::Relaxed);
    }
}

hal::list_adapter!(
    /// `SLIST_HEAD(, timecounter) tc_list`, through the entry's link.
    TcList: TcEntry, link
);

/// `dummy_timecounter`: a counter that advances by one on every read, at a nominal 1 MHz:
/// what the console and other early code read until a real one is in the air.
pub struct Dummy;

impl Timecounter for Dummy {
    /// `dummy_get_timecount`.
    fn get_timecount(&self) -> u32 {
        DUMMY_NOW.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }
    fn counter_mask(&self) -> u32 {
        !0u32
    }
    fn frequency(&self) -> u64 {
        1_000_000
    }
    fn name(&self) -> &'static str {
        "dummy"
    }
    fn quality(&self) -> i32 {
        -1_000_000
    }
}

/// A `struct bintime` member of the timehands: two atomic words.
struct AtomicBintime {
    sec: AtomicI64,
    frac: AtomicU64,
}

impl AtomicBintime {
    const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            frac: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    fn get(&self) -> Bintime {
        Bintime::new(
            self.sec.load(Ordering::Relaxed),
            self.frac.load(Ordering::Relaxed),
        )
    }

    #[inline(always)]
    fn set(&self, bt: Bintime) {
        self.sec.store(bt.sec, Ordering::Relaxed);
        self.frac.store(bt.frac, Ordering::Relaxed);
    }
}

/// A two-word time member (`timeval` or `timespec`): seconds and the fraction's count.
struct AtomicPair {
    sec: AtomicI64,
    sub: AtomicI64,
}

impl AtomicPair {
    const fn new() -> Self {
        Self {
            sec: AtomicI64::new(0),
            sub: AtomicI64::new(0),
        }
    }

    #[inline(always)]
    fn get(&self) -> (i64, i64) {
        (
            self.sec.load(Ordering::Relaxed),
            self.sub.load(Ordering::Relaxed),
        )
    }

    #[inline(always)]
    fn set(&self, sec: i64, sub: i64) {
        self.sec.store(sec, Ordering::Relaxed);
        self.sub.store(sub, Ordering::Relaxed);
    }
}

/// `struct timehands`. Written only by [`tc_windup`] (under `windup_mtx`) and only into the
/// ring's other member, whose generation is 0 meanwhile; read without a lock by the readers,
/// which check the generation around their reads.
struct Timehands {
    /// \[W\] `th_counter` (the dummy while unset: `th1` before its first windup).
    counter: Link<TcEntry>,
    /// \[T,W\] `th_adjtimedelta`.
    adjtimedelta: AtomicI64,
    /// \[T,W\] `th_next_ntp_update`.
    next_ntp_update: AtomicBintime,
    /// \[W\] `th_adjustment`.
    adjustment: AtomicI64,
    /// \[W\] `th_scale`.
    scale: AtomicU64,
    /// \[W\] `th_offset_count`.
    offset_count: AtomicU32,
    /// \[T,W\] `th_boottime`.
    boottime: AtomicBintime,
    /// \[W\] `th_offset`.
    offset: AtomicBintime,
    /// \[W\] `th_naptime`.
    naptime: AtomicBintime,
    /// \[W\] `th_microtime`.
    microtime: AtomicPair,
    /// \[W\] `th_nanotime`.
    nanotime: AtomicPair,
    /// \[W\] `th_generation` (volatile): 0 while inconsistent.
    generation: AtomicU32,
    /// \[I\] `th_next`: the ring's other member.
    next: usize,
}

impl Timehands {
    const fn new(counter: Link<TcEntry>, scale: u64, generation: u32, next: usize) -> Self {
        Self {
            counter,
            adjtimedelta: AtomicI64::new(0),
            next_ntp_update: AtomicBintime::new(),
            adjustment: AtomicI64::new(0),
            scale: AtomicU64::new(scale),
            offset_count: AtomicU32::new(0),
            boottime: AtomicBintime::new(),
            offset: AtomicBintime::new(),
            naptime: AtomicBintime::new(),
            microtime: AtomicPair::new(),
            nanotime: AtomicPair::new(),
            generation: AtomicU32::new(generation),
            next,
        }
    }

    /// `th->th_counter`.
    #[inline(always)]
    fn counter(&self) -> &'static TcEntry {
        self.counter.get().unwrap_or(&DUMMY)
    }

    /// `memcpy(th, tho, offsetof(struct timehands, th_generation))`: every field before the
    /// generation.
    fn copy_from(&self, tho: &Timehands) {
        self.counter.set(Some(tho.counter()));
        self.adjtimedelta
            .store(tho.adjtimedelta.load(Ordering::Relaxed), Ordering::Relaxed);
        self.next_ntp_update.set(tho.next_ntp_update.get());
        self.adjustment
            .store(tho.adjustment.load(Ordering::Relaxed), Ordering::Relaxed);
        self.scale
            .store(tho.scale.load(Ordering::Relaxed), Ordering::Relaxed);
        self.offset_count
            .store(tho.offset_count.load(Ordering::Relaxed), Ordering::Relaxed);
        self.boottime.set(tho.boottime.get());
        self.offset.set(tho.offset.get());
        self.naptime.set(tho.naptime.get());
        let (s, u) = tho.microtime.get();
        self.microtime.set(s, u);
        let (s, n) = tho.nanotime.get();
        self.nanotime.set(s, n);
    }

    /// `tc_delta(th)`: the counts since the timehands' `th_offset_count` was taken.
    #[inline(always)]
    fn delta(&self) -> u32 {
        let tc = self.counter();
        tc.get_timecount()
            .wrapping_sub(self.offset_count.load(Ordering::Relaxed))
            & tc.mask.load(Ordering::Relaxed)
    }

    /// The uptime by this timehands' counter now (`binuptime`'s body).
    #[inline(always)]
    fn uptime(&self) -> Bintime {
        Bintime::from_timecount(self.delta(), self.scale.load(Ordering::Relaxed))
            .add(self.offset.get())
    }
}

/// `windup_mtx`'s `[W]` state outside the timehands: what [`tc_windup`] keeps between calls.
pub struct Windup {
    /// `tc_update_timekeep`'s `last_tc`: the counter whose mask and user flag the page holds.
    last_tc: Option<&'static TcEntry>,
}

/// `dummy_timecounter`'s entry: never in `tc_list`; slot of the boot timehands.
static DUMMY: TcEntry = TcEntry {
    hw: &Dummy,
    mask: AtomicU32::new(!0u32),
    frequency: AtomicU64::new(1_000_000),
    user: AtomicI32::new(0),
    quality: AtomicI32::new(-1_000_000),
    precision: AtomicU64::new(0),
    freq_adj: AtomicI64::new(0),
    link: ListEntry::new(),
};

/// `dummy_get_timecount`'s count.
static DUMMY_NOW: AtomicU32 = AtomicU32::new(0);

/// `th0`, `th1`: the ring. `th0` starts live on the dummy counter at its scale, `th1` empty.
static TH: [Timehands; 2] = [
    Timehands::new(Link::to(&DUMMY), u64::MAX / 1_000_000, 1, 1),
    Timehands::new(Link::new(), 0, 0, 0),
];

/// \[W\] `timehands`: the live member of [`TH`].
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// \[T\] `timecounter`: the active counter (the next windup switches the timehands to it).
static TIMECOUNTER: Link<TcEntry> = Link::to(&DUMMY);

/// `tc_lock`: the timecounter choice and the clock settings (`kern_time.c` takes it).
pub static TC_LOCK: RawRwlock = RawRwlock::new("tc_lock");

/// `windup_mtx`: `tc_windup()` must be called before leaving this mutex.
pub static WINDUP: Mutexed<Windup> = Mutexed::new(IPL_CLOCK, Windup { last_tc: None });

/// `tc_list`: the registered timecounters, newest first.
static TC_LIST: List<TcList> = List::new();

/// `naptime` (volatile), updated by `tc_windup`: the seconds spent suspended.
pub static NAPTIME: AtomicI64 = AtomicI64::new(0);
/// `time_second` (volatile): the time of day in seconds, as of the last windup.
pub static TIME_SECOND: AtomicI64 = AtomicI64::new(0);
/// `time_uptime` (volatile): the uptime in seconds, as of the last windup.
pub static TIME_UPTIME: AtomicI64 = AtomicI64::new(0);

/// `timestepwarnings` (`kern.timecounter.timestepwarnings`, 0 or 1).
pub static TIMESTEPWARNINGS: AtomicI32 = AtomicI32::new(0);

/// `tc_tick` (`kern.timecounter.tick`, read-only): windups happen every `tc_tick`
/// hardclocks, so the hardware counter never wraps between two and the `get*` readers stay
/// as precise as that.
pub static TC_TICK: AtomicI32 = AtomicI32::new(0);

/// `timekeep`: the page shared with userland, once something maps it ([`timekeep_attach`]);
/// written by `tc_update_timekeep` under `windup_mtx`.
static TIMEKEEP: OnceStatic<&'static TimekeepAtomic> = OnceStatic::new();

/// The live timehands.
#[inline(always)]
fn timehands() -> &'static Timehands {
    &TH[ACTIVE.load(Ordering::Relaxed) & 1]
}

/// `timecounter`: the active counter.
#[inline]
pub fn timecounter() -> &'static TcEntry {
    TIMECOUNTER.get().unwrap_or(&DUMMY)
}

/// Reads a consistent value off the timehands: loops until the timehands it read was not
/// wound up under its feet (the generation is not 0 and the same before and after).
#[inline(always)]
fn read_timehands<T>(f: impl Fn(&Timehands) -> T) -> T {
    loop {
        let th = timehands();
        let generation = th.generation.load(Ordering::Relaxed);
        fence(Ordering::Acquire); // membar_consumer()
        let v = f(th);
        fence(Ordering::Acquire); // membar_consumer()
        if generation != 0 && generation == th.generation.load(Ordering::Relaxed) {
            return v;
        }
    }
}

/// `binboottime`.
#[inline]
pub fn binboottime() -> Bintime {
    read_timehands(|th| th.boottime.get())
}

/// `microboottime`.
#[inline]
pub fn microboottime() -> Timeval {
    binboottime().to_timeval()
}

/// `nanoboottime`.
#[inline]
pub fn nanoboottime() -> Timespec {
    binboottime().to_timespec()
}

/// `binuptime`.
#[inline(always)]
pub fn binuptime() -> Bintime {
    read_timehands(Timehands::uptime)
}

/// `getbinuptime`.
#[inline]
pub fn getbinuptime() -> Bintime {
    read_timehands(|th| th.offset.get())
}

/// `nanouptime`.
#[inline]
pub fn nanouptime() -> Timespec {
    binuptime().to_timespec()
}

/// `microuptime`.
#[inline]
pub fn microuptime() -> Timeval {
    binuptime().to_timeval()
}

/// `getuptime`.
#[inline]
pub fn getuptime() -> i64 {
    TIME_UPTIME.load(Ordering::Relaxed) // atomic
}

/// `nsecuptime`.
#[inline]
pub fn nsecuptime() -> u64 {
    binuptime().to_nsec()
}

/// `getnsecuptime`.
#[inline]
pub fn getnsecuptime() -> u64 {
    getbinuptime().to_nsec()
}

/// `binruntime`: the uptime less the time spent suspended.
#[inline]
pub fn binruntime() -> Bintime {
    read_timehands(|th| th.uptime().sub(th.naptime.get()))
}

/// `nanoruntime`.
#[inline]
pub fn nanoruntime() -> Timespec {
    binruntime().to_timespec()
}

/// `getbinruntime`.
#[inline]
pub fn getbinruntime() -> Bintime {
    read_timehands(|th| th.offset.get().sub(th.naptime.get()))
}

/// `getnsecruntime`.
#[inline]
pub fn getnsecruntime() -> u64 {
    getbinruntime().to_nsec()
}

/// `bintime`: the time of day.
#[inline]
pub fn bintime() -> Bintime {
    read_timehands(|th| th.uptime().add(th.boottime.get()))
}

/// `nanotime`.
#[inline]
pub fn nanotime() -> Timespec {
    bintime().to_timespec()
}

/// `microtime`.
#[inline]
pub fn microtime() -> Timeval {
    bintime().to_timeval()
}

/// `gettime`.
#[inline]
pub fn gettime() -> i64 {
    TIME_SECOND.load(Ordering::Relaxed) // atomic
}

/// `getnanouptime`.
#[inline]
pub fn getnanouptime() -> Timespec {
    read_timehands(|th| th.offset.get().to_timespec())
}

/// `getmicrouptime`.
#[inline]
pub fn getmicrouptime() -> Timeval {
    read_timehands(|th| th.offset.get().to_timeval())
}

/// `getnanotime`.
#[inline]
pub fn getnanotime() -> Timespec {
    read_timehands(|th| {
        let (s, n) = th.nanotime.get();
        Timespec::new(s, n)
    })
}

/// `getmicrotime`.
#[inline]
pub fn getmicrotime() -> Timeval {
    read_timehands(|th| {
        let (s, u) = th.microtime.get();
        Timeval::new(s, u)
    })
}

/// `tc_init`: registers a new timecounter and possibly uses it.
pub fn tc_init(tc: &'static TcEntry) {
    tc.load_from_hw();

    let mask = u64::from(tc.mask.load(Ordering::Relaxed));
    let freq = tc.frequency();
    let mut u = (freq / mask.max(1)) as u32;
    // XXX: We need some margin here, 10% is a guess
    u = u.wrapping_mul(11);
    u /= 10;
    if tc.quality() >= 0 && u > crate::timeout::hz() as u32 {
        tc.quality.store(-2000, Ordering::Relaxed);
        printf(format_args!(
            "Timecounter \"{}\" frequency {} Hz",
            tc.name(),
            freq
        ));
        printf(format_args!(" -- Insufficient hz, needs at least {u}\n"));
    }

    // Determine the counter's precision.
    let mut tmp: u64 = 1;
    while tmp & mask == 0 {
        tmp <<= 1;
    }
    tc.precision.store(tmp, Ordering::Relaxed);

    TC_LIST.insert_head(tc);

    // Never automatically use a timecounter with negative quality. Even though we run on the
    // dummy counter, switching here may be worse since this timecounter may not be monotonic.
    if tc.quality() < 0 {
        return;
    }
    let active = timecounter();
    if tc.quality() < active.quality() {
        return;
    }
    if tc.quality() == active.quality() && freq < active.frequency() {
        return;
    }
    let _ = tc.get_timecount();
    hal::random::enqueue_randomness(tc.get_timecount());

    TIMECOUNTER.set(Some(tc));
}

/// `tc_reset_quality`: changes the counter's quality; if it is the active counter and no
/// longer the best, the best one takes over.
pub fn tc_reset_quality(tc: &'static TcEntry, quality: i32) {
    if core::ptr::eq(tc, &DUMMY) {
        kpanic(format_args!(
            "tc_reset_quality: cannot change dummy counter quality"
        ));
    }

    tc.quality.store(quality, Ordering::Relaxed);
    if TIMECOUNTER.points_to(tc) {
        let mut best: &'static TcEntry = &DUMMY;
        for tmp in TC_LIST.iter() {
            if tmp.quality() < 0 {
                continue;
            }
            if tmp.quality() < best.quality() {
                continue;
            }
            if tmp.quality() == best.quality() && tmp.frequency() < best.frequency() {
                continue;
            }
            best = tmp;
        }
        if !core::ptr::eq(best, tc) {
            hal::random::enqueue_randomness(best.get_timecount());
            TIMECOUNTER.set(Some(best));
            printf(format_args!(
                "timecounter: active counter changed: {} -> {}\n",
                tc.name(),
                best.name()
            ));
        }
    }
}

/// `tc_getfrequency`: the frequency of the counter the timehands read.
pub fn tc_getfrequency() -> u64 {
    timehands().counter().frequency()
}

/// `tc_getprecision`: the precision of the counter the timehands read.
pub fn tc_getprecision() -> u64 {
    timehands().counter().precision()
}

/// `tc_setrealtimeclock`: steps UTC (the realtime clock) by moving the boot time. Any
/// adjtime(2) in progress is meaningless after a jump, so the adjustment is zeroed too.
pub fn tc_setrealtimeclock(ts: &Timespec) {
    let utc = ts.to_bintime();

    let w = TC_LOCK.write();
    let mut g = WINDUP.lock();

    let uptime = binuptime();
    let boottime = utc.sub(uptime);
    let old_utc = timehands().boottime.get().add(uptime);
    // XXX fiddle all the little crinkly bits around the fiords...
    tc_windup(&mut g, Some(&w), Some(&boottime), None, Some(0));

    drop(g);
    drop(w);

    hal::random::enqueue_randomness(ts.tv_sec as u32);

    if TIMESTEPWARNINGS.load(Ordering::Relaxed) != 0 {
        let tmp = old_utc.to_timespec();
        hal::prf::log(
            LOG_INFO,
            format_args!(
                "Time stepped from {}.{:09} to {}.{:09}\n",
                tmp.tv_sec, tmp.tv_nsec, ts.tv_sec, ts.tv_nsec
            ),
        );
    }
}

/// `tc_setclock`: steps the monotonic and realtime clocks, triggering the timeouts that
/// should have fired across the interval. The first call, when the root file system is
/// mounted at boot, sets the boot time instead.
pub fn tc_setclock(ts: &Timespec) {
    static FIRST: AtomicBool = AtomicBool::new(true);

    if FIRST.swap(false, Ordering::Relaxed) {
        tc_setrealtimeclock(ts);
        return;
    }

    hal::random::enqueue_randomness(ts.tv_sec as u32);

    let utc = ts.to_bintime();

    let mut g = WINDUP.lock();
    let uptime = utc.sub(timehands().boottime.get());
    let old_naptime = timehands().naptime.get();
    // XXX fiddle all the little crinkly bits around the fiords...
    tc_windup(&mut g, None, None, Some(&uptime), None);
    let new_naptime = timehands().naptime.get();
    drop(g);

    // !SMALL_KERNEL: the time skipped, in ticks, moves the timeout wheel forward.
    let elapsed = new_naptime.sub(old_naptime);
    let adj_ticks = elapsed.to_nsec() / crate::timeout::tick_nsec().max(1) as u64;
    if adj_ticks > 0 {
        crate::timeout::adjust_ticks(adj_ticks.min(i32::MAX as u64) as i32);
    }
}

/// `tc_update_timekeep`: publishes the timehands to the page userland reads, in the
/// generation protocol libc follows.
fn tc_update_timekeep(g: &mut MutexedGuard<'_, Windup>) {
    let Some(tk) = TIMEKEEP.get().copied() else {
        return;
    };

    let th = timehands();
    tk.tk_generation.store(0, Ordering::Relaxed);
    fence(Ordering::Release); // membar_producer()
    tk.tk_scale
        .store(th.scale.load(Ordering::Relaxed), Ordering::Relaxed);
    tk.tk_offset_count
        .store(th.offset_count.load(Ordering::Relaxed), Ordering::Relaxed);
    let bt = th.offset.get();
    tk.tk_offset.set(bt.sec, bt.frac);
    let bt = th.naptime.get();
    tk.tk_naptime.set(bt.sec, bt.frac);
    let bt = th.boottime.get();
    tk.tk_boottime.set(bt.sec, bt.frac);
    let counter = th.counter();
    if !g.last_tc.is_some_and(|l| core::ptr::eq(l, counter)) {
        tk.tk_counter_mask
            .store(counter.counter_mask(), Ordering::Relaxed);
        tk.tk_user.store(counter.user(), Ordering::Relaxed);
        g.last_tc = Some(counter);
    }
    fence(Ordering::Release); // membar_producer()
    tk.tk_generation
        .store(th.generation.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Attaches the timekeep page (`exec_timekeep_map`'s first call): from the next windup on,
/// the timehands are published there. Once: a second page is refused.
pub fn timekeep_attach(tk: &'static TimekeepAtomic) -> Result<(), &'static TimekeepAtomic> {
    tk.tk_version.store(TK_VERSION, Ordering::Relaxed);
    TIMEKEEP.set(tk)?;
    let mut g = WINDUP.lock();
    g.last_tc = None;
    tc_update_timekeep(&mut g);
    Ok(())
}

/// Checks a witness of `tc_lock` held for writing (`rw_assert_wrlock(&tc_lock)`, under
/// `DIAGNOSTIC` as the C's).
#[inline(always)]
fn assert_tc_wrlock(w: Option<&WriteWitness<'_>>) {
    kassert!(w.is_some_and(|w| w.lock_id() == TC_LOCK.id()));
    kassert!(TC_LOCK.status() == RW_WRITE);
    let _ = w;
}

/// `tc_windup`: makes the ring's other timehands a copy of the live one advanced to now,
/// switching to a new counter and doing the NTP second processing on the way, and makes it
/// live. Slightly magic. `tc` is the `[T]` witness, required when the boot time or the
/// adjustment change.
pub fn tc_windup(
    g: &mut MutexedGuard<'_, Windup>,
    tc: Option<&WriteWitness<'_>>,
    new_boottime: Option<&Bintime>,
    new_offset: Option<&Bintime>,
    new_adjtimedelta: Option<i64>,
) {
    if new_boottime.is_some() || new_adjtimedelta.is_some() {
        assert_tc_wrlock(tc);
    }

    let active_tc = timecounter();

    // Make the next timehands a copy of the current one, but do not overwrite the generation
    // or next pointer. While we update the contents, the generation must be zero.
    let tho = timehands();
    let mut ogen = tho.generation.load(Ordering::Relaxed);
    let th = &TH[tho.next];
    th.generation.store(0, Ordering::Relaxed);
    fence(Ordering::Release); // membar_producer()
    th.copy_from(tho);

    // Capture a timecounter delta on the current timecounter and if changing timecounters,
    // a counter value from the new timecounter. Update the offset fields accordingly.
    let delta = th.delta();
    let ncount = if !core::ptr::eq(th.counter(), active_tc) {
        active_tc.get_timecount()
    } else {
        0
    };
    th.offset_count.store(
        th.offset_count.load(Ordering::Relaxed).wrapping_add(delta)
            & th.counter().mask.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    let bt = Bintime::from_timecount(delta, th.scale.load(Ordering::Relaxed));
    th.offset.set(th.offset.get().add(bt));

    // Ignore new offsets that predate the current offset. If changing the offset, first
    // increase the naptime accordingly.
    if let Some(new_offset) = new_offset
        && th.offset.get() < *new_offset
    {
        let bt = new_offset.sub(th.offset.get());
        th.naptime.set(th.naptime.get().add(bt));
        NAPTIME.store(th.naptime.get().sec, Ordering::Relaxed);
        th.offset.set(*new_offset);
    }

    // If changing the boot time or clock adjustment, do so before NTP processing.
    if let Some(new_boottime) = new_boottime {
        th.boottime.set(*new_boottime);
    }
    if let Some(new_adjtimedelta) = new_adjtimedelta {
        th.adjtimedelta.store(new_adjtimedelta, Ordering::Relaxed);
        // Reset the NTP update period.
        th.next_ntp_update
            .set(th.offset.get().sub(th.naptime.get()));
    }

    // Deal with NTP second processing. The loop normally iterates at most once, but in
    // extreme situations it might keep NTP sane if tc_windup() is not run for several
    // seconds.
    let bt = th.offset.get().sub(th.naptime.get());
    while th.next_ntp_update.get() <= bt {
        ntp_update_second(g, th);
        let mut next = th.next_ntp_update.get();
        next.sec += 1;
        th.next_ntp_update.set(next);
    }

    // Update the UTC timestamps used by the get*() functions.
    let bt = th.boottime.get().add(th.offset.get());
    let tv = bt.to_timeval();
    th.microtime.set(tv.tv_sec, tv.tv_usec);
    let ts = bt.to_timespec();
    th.nanotime.set(ts.tv_sec, ts.tv_nsec);

    // Now is a good time to change timecounters.
    if !core::ptr::eq(th.counter(), active_tc) {
        th.counter.set(Some(active_tc));
        th.offset_count.store(ncount, Ordering::Relaxed);
    }

    // Recalculate the scaling factor: the number of 1/2^64 fractions of a second per period
    // of the hardware counter, with the th_adjustment the NTP PLL/adjtime(2) provides
    // (nanoseconds per second with a 32-bit binary fraction, +/- 5000 PPM). Multiplying by
    // 2^32 / 10^9 = 4.294967296 overflows past ~850, so it is divided first, by 2199/512:
    // a systematic undercompensation of 10 PPM of th_adjustment (0.05 PPM of a 5000 PPM
    // adjustment), and the lowest of the 64 bits is sacrificed to clarity.
    let mut scale: u64 = 1 << 63;
    let adj = th.adjustment.load(Ordering::Relaxed) + th.counter().freq_adj.load(Ordering::Relaxed);
    scale = scale.wrapping_add(((adj / 1024) * 2199) as u64);
    scale /= th.counter().frequency().max(1);
    th.scale.store(scale.wrapping_mul(2), Ordering::Relaxed);

    // Now that the struct timehands is again consistent, set the new generation number,
    // making sure to not make it zero.
    ogen = ogen.wrapping_add(1);
    if ogen == 0 {
        ogen = 1;
    }
    fence(Ordering::Release); // membar_producer()
    th.generation.store(ogen, Ordering::Relaxed);

    // Go live with the new struct timehands.
    TIME_SECOND.store(th.microtime.get().0, Ordering::Relaxed);
    TIME_UPTIME.store(th.offset.get().sec, Ordering::Relaxed);
    fence(Ordering::Release); // membar_producer()
    ACTIVE.store(tho.next, Ordering::Relaxed);

    tc_update_timekeep(g);
}

/// `tc_ticktock`: the hardclock's share, a windup every `tc_tick` ticks (skipped while
/// another CPU winds up).
pub fn tc_ticktock() {
    static COUNT: AtomicI32 = AtomicI32::new(0);

    if COUNT.fetch_add(1, Ordering::Relaxed) + 1 < TC_TICK.load(Ordering::Relaxed) {
        return;
    }
    let Some(mut g) = WINDUP.try_lock() else {
        return;
    };
    COUNT.store(0, Ordering::Relaxed);
    tc_windup(&mut g, None, None, None, None);
}

/// `sysctl_tc_hardware`'s change: makes the registered counter named `name` the active one
/// (warmed up first); `EINVAL` when none has that name. The name of the active counter is
/// [`timecounter`]`().name()`.
pub fn tc_select(name: &[u8]) -> Result<(), crate::sys::errno::Errno> {
    if name == timecounter().name().as_bytes() {
        return Ok(());
    }
    for newtc in TC_LIST.iter() {
        if name != newtc.name().as_bytes() {
            continue;
        }

        // Warm up new timecounter.
        let _ = newtc.get_timecount();
        let _ = newtc.get_timecount();

        let _w = TC_LOCK.write();
        TIMECOUNTER.set(Some(newtc));
        return Ok(());
    }
    Err(crate::sys::errno::Errno::EINVAL)
}

/// `sysctl_tc_choice`'s string: every registered counter as `name(quality)`, space
/// separated, newest first, written into `out` as the C's `strlcat`s (truncated, always
/// NUL-terminated); returns the length without the NUL.
pub fn tc_choice(out: &mut [u8]) -> usize {
    let mut len = 0;
    let mut push = |b: &[u8]| {
        for &c in b {
            if len + 1 < out.len() {
                out[len] = c;
                len += 1;
            }
        }
    };
    for (i, tc) in TC_LIST.iter().enumerate() {
        if i > 0 {
            push(b" ");
        }
        push(tc.name().as_bytes());
        push(b"(");
        let mut num = [0u8; 12];
        push(fmt_i32(tc.quality(), &mut num));
        push(b")");
    }
    if let Some(nul) = out.get_mut(len) {
        *nul = 0;
    }
    len
}

/// `%d` of `v` into `buf`, returned as the written slice.
fn fmt_i32(v: i32, buf: &mut [u8; 12]) -> &[u8] {
    let mut n = i64::from(v).unsigned_abs();
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    if v < 0 {
        i -= 1;
        buf[i] = b'-';
    }
    &buf[i..]
}

/// The number of registered counters (`sysctl_tc_choice`'s `maxlen` is 32 bytes each).
pub fn tc_count() -> usize {
    TC_LIST.iter().count()
}

/// `inittimecounter`: the windup period (`tc_tick`, one tick per millisecond of `hz`, at
/// least one) and the active counter warmed up.
pub fn inittimecounter() {
    // Set the initial timeout to max(1, <approx. number of hardclock ticks in a
    // millisecond>). People should probably not use the sysctl to set the timeout to smaller
    // than its initial value, since that value is the smallest reasonable one. If they want
    // better timestamps they should use the non-"get"* functions.
    let hz = crate::timeout::hz();
    TC_TICK.store(
        if hz > 1000 { (hz + 500) / 1000 } else { 1 },
        Ordering::Relaxed,
    );
    #[cfg(feature = "debug")]
    {
        let p = (TC_TICK.load(Ordering::Relaxed) * 1_000_000) / hz;
        printf(format_args!(
            "Timecounters tick every {}.{:03} msec\n",
            p / 1000,
            p % 1000
        ));
    }

    // warm up new timecounter (again) and get rolling.
    let _ = timecounter().get_timecount();
    let _ = timecounter().get_timecount();
}

/// `ntp_update_second`: skews the timehands by a second's share of the adjtime(2)
/// adjustment (at most 5000 ns a second).
fn ntp_update_second(_g: &mut MutexedGuard<'_, Windup>, th: &Timehands) {
    let delta = th.adjtimedelta.load(Ordering::Relaxed);
    let adj = if delta > 0 {
        delta.min(5000)
    } else {
        delta.max(-5000)
    };
    th.adjtimedelta.store(delta - adj, Ordering::Relaxed);
    th.adjustment.store((adj * 1000) << 32, Ordering::Relaxed);
}

/// `tc_adjfreq`: reads (any hold of `tc_lock`) and/or sets (`tc_lock` written) the active
/// counter's frequency adjustment.
pub fn tc_adjfreq(w: &RwWitness<'_>, old: Option<&mut i64>, new: Option<i64>) {
    kassert!(w.lock_id() == TC_LOCK.id());
    if let Some(old) = old {
        *old = timecounter().freq_adj.load(Ordering::Relaxed);
    }
    if let Some(new) = new {
        let wr = match w {
            RwWitness::Write(wr) => Some(wr),
            RwWitness::Read(_) => None,
        };
        assert_tc_wrlock(wr);
        let mut g = WINDUP.lock();
        timecounter().freq_adj.store(new, Ordering::Relaxed);
        tc_windup(&mut g, wr, None, None, None);
    }
}

/// `tc_adjtime`: reads and/or sets (`tc_lock` written) the pending adjtime(2) delta.
pub fn tc_adjtime(w: &WriteWitness<'_>, old: Option<&mut i64>, new: Option<i64>) {
    if let Some(old) = old {
        *old = read_timehands(|th| th.adjtimedelta.load(Ordering::Relaxed));
    }
    if let Some(new) = new {
        assert_tc_wrlock(Some(w));
        let mut g = WINDUP.lock();
        tc_windup(&mut g, Some(w), None, None, Some(new));
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Over the dummy counter (one count per read at 1 MHz) and test counters of their own.
    // The timehands are global, so the tests that wind them up or change the active counter
    // serialise on `LOCK`.

    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::AtomicBool as StdAtomicBool;

    use super::*;
    use crate::testutil;

    /// Serialises the tests that wind the timehands up or change the active counter.
    pub(crate) static LOCK: StdMutex<()> = StdMutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        testutil::install();
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A counter whose value the test sets.
    struct Settable {
        count: AtomicU32,
        mask: u32,
        freq: u64,
        name: &'static str,
        quality: i32,
    }

    impl Timecounter for Settable {
        fn get_timecount(&self) -> u32 {
            self.count.load(Ordering::Relaxed)
        }
        fn counter_mask(&self) -> u32 {
            self.mask
        }
        fn frequency(&self) -> u64 {
            self.freq
        }
        fn name(&self) -> &'static str {
            self.name
        }
        fn quality(&self) -> i32 {
            self.quality
        }
    }

    /// A timehands of its own over `tc` (outside the ring), live, at `offset` seconds, its last
    /// windup having read `count`, scaled as `tc_windup` scales with no adjustment.
    fn own_timehands(tc: &'static TcEntry, offset: i64, count: u32) -> Timehands {
        let th = Timehands::new(Link::to(tc), 0, 1, 0);
        th.scale.store(
            ((1u64 << 63) / tc.frequency()).wrapping_mul(2),
            Ordering::Relaxed,
        );
        th.offset.set(Bintime::new(offset, 0));
        th.offset_count.store(count, Ordering::Relaxed);
        th
    }

    #[test]
    fn a_counter_read_a_period_after_the_windup_steps_uptime_back() {
        // amd64's i8254 behind the LAPIC timer: 15 bits at TIMER_FREQ, a period of 27.46 ms.
        static HW: Settable = Settable {
            count: AtomicU32::new(0),
            mask: 0x7fff,
            freq: 1_193_182,
            name: "wrap15",
            quality: 0,
        };
        static E: TcEntry = TcEntry::new(&HW);
        E.load_from_hw();
        let th = own_timehands(&E, 5, 0x7000);
        let at = |count: u32| {
            HW.count.store(count, Ordering::Relaxed);
            th.uptime().to_nsec()
        };

        // One tick (11932 counts) after the windup, across the counter's own wrap: tc_delta is
        // modular, so the reading is exact.
        let tick = at((0x7000 + 11932) & 0x7fff);
        let ns_11932 = (11932u64 * 1_000_000_000) / 1_193_182;
        assert!((tick - 5_000_000_000).abs_diff(ns_11932) <= 1, "{tick}");

        // Held off for more than a period, the reading lands 0x10 counts after the windup,
        // behind one taken just before the wrap: the C's behaviour on this counter.
        let before = at((0x7000 + 0x7ff0) & 0x7fff);
        let after = at((0x7000 + 0x8010) & 0x7fff);
        assert!(after < before, "{before} then {after}");
        let period_minus_0x20 = (0x7fe0u64 * 1_000_000_000) / 1_193_182;
        assert!((before - after).abs_diff(period_minus_0x20) <= 2);
    }

    #[test]
    fn the_tsc_counter_read_long_after_the_windup_does_not_step_back() {
        // amd64's TSC timecounter: the low 32 bits at QEMU's 1000.68 MHz, a period of 4.29 s.
        const FREQ: u64 = 1_000_680_000;
        static HW: Settable = Settable {
            count: AtomicU32::new(0),
            mask: !0,
            freq: FREQ,
            name: "tsc32",
            quality: 2000,
        };
        static E: TcEntry = TcEntry::new(&HW);
        E.load_from_hw();
        let windup = 0xffff_0000u32;
        let th = own_timehands(&E, 5, windup);
        let at = |count: u32| {
            HW.count.store(count, Ordering::Relaxed);
            th.uptime().to_nsec()
        };
        let counts = |ns: u64| (ns * FREQ / 1_000_000_000) as u32;
        let i8254_period_ns = (0x8000u64 * 1_000_000_000) / 1_193_182;
        let mut last = at(windup);
        for ns in [
            i8254_period_ns - 1000,
            i8254_period_ns + 1000,
            2 * i8254_period_ns,
            100_000_000,
            1_000_000_000,
            4_000_000_000,
        ] {
            let now = at(windup.wrapping_add(counts(ns)));
            assert!(now > last, "{ns} ns after the windup: {last} then {now}");
            assert!((now - 5_000_000_000).abs_diff(ns) <= 2, "{ns}: {now}");
            last = now;
        }
    }

    #[test]
    fn readers_are_monotonic_on_the_dummy_counter() {
        let _g = lock();
        let a = nsecuptime();
        let b = nsecuptime();
        assert!(b > a, "the active counter advances between reads: {a} {b}");
        let bt = binuptime();
        let ts = nanouptime();
        assert!(ts.tv_sec >= bt.sec);
        assert!(microuptime().tv_sec >= ts.tv_sec);
    }

    #[test]
    fn windup_advances_the_offset_and_the_cached_times() {
        let _g = lock();
        inittimecounter();
        let before = getnsecuptime();
        // Burn counts: at 1 MHz, 2_000_000 reads are two seconds of dummy time (only while
        // the dummy is the active counter: a test counter installed by another test winds
        // with the dummy demoted, and then time moves by its own reads).
        let dummy = core::ptr::eq(timecounter(), &DUMMY);
        for _ in 0..2_000_000 {
            let _ = timecounter().get_timecount();
        }
        tc_windup(&mut WINDUP.lock(), None, None, None, None);
        let after = getnsecuptime();
        if dummy {
            assert!(after - before >= 1_900_000_000, "{before} -> {after}");
            assert!(getuptime() >= 1);
        }
        assert_eq!(getnanouptime().tv_sec, getuptime());
        assert_eq!(getmicrouptime().tv_sec, getuptime());
        assert!(getnanotime().tv_sec >= getuptime());
        assert_eq!(gettime(), getmicrotime().tv_sec);
    }

    #[test]
    fn ticktock_winds_every_tc_tick() {
        let _g = lock();
        inittimecounter();
        assert_eq!(TC_TICK.load(Ordering::Relaxed), 1, "hz 100: every tick");
        let before = getbinuptime();
        for _ in 0..10_000 {
            let _ = timecounter().get_timecount();
        }
        tc_ticktock();
        let after = getbinuptime();
        assert!(after > before, "{before:?} -> {after:?}");
        assert!(tc_getfrequency() >= 1_000_000);
    }

    #[test]
    fn setrealtimeclock_sets_the_boot_time_and_setclock_steps_uptime() {
        let _g = lock();
        // tc_setclock moves the timeout wheel (timeout_adjust_ticks): hold its tests' lock.
        let _w = crate::timeout::tests::LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        inittimecounter();
        let utc = Timespec::new(1_700_000_000, 0);
        tc_setrealtimeclock(&utc);
        let boot = binboottime();
        let up = getbinuptime();
        // boottime = utc - uptime, so the seconds carry through the fractions.
        assert_eq!(boot.add(up).sec, 1_700_000_000);
        assert!(bintime().sec >= 1_700_000_000);
        assert!(nanotime().tv_sec >= 1_700_000_000);
        assert!(microtime().tv_sec >= 1_700_000_000);
        assert_eq!(nanoboottime().tv_sec, boot.sec);
        assert_eq!(microboottime().tv_sec, boot.sec);
        // tc_setclock's first call sets the boot time too (if no test made it already); a
        // later one steps the uptime forward and counts the step as naptime.
        tc_setclock(&utc);
        let run_before = getnsecruntime();
        let up_before = getuptime();
        let ticks_before = crate::clock::ticks();
        tc_setclock(&Timespec::new(nanotime().tv_sec + 100, 0));
        assert!(getuptime() >= up_before + 99, "{}", getuptime());
        assert!(NAPTIME.load(Ordering::Relaxed) >= 99);
        assert!(getnsecruntime() < getnsecuptime());
        assert!(nanoruntime().tv_sec <= nanouptime().tv_sec);
        assert!(getnsecruntime() >= run_before);
        assert!(binruntime() < binuptime());
        assert!(
            crate::clock::ticks().wrapping_sub(ticks_before) >= 99 * crate::timeout::hz(),
            "the wheel moved by the step"
        );
    }

    #[test]
    fn a_better_timecounter_takes_over_and_quality_resets() {
        let _g = lock();
        static COUNT: AtomicU32 = AtomicU32::new(0);
        struct Counting(&'static str, i32);
        impl Timecounter for Counting {
            fn get_timecount(&self) -> u32 {
                COUNT.fetch_add(100, Ordering::Relaxed)
            }
            fn counter_mask(&self) -> u32 {
                0xffff_ffff
            }
            fn frequency(&self) -> u64 {
                100_000_000
            }
            fn name(&self) -> &'static str {
                self.0
            }
            fn quality(&self) -> i32 {
                self.1
            }
        }
        static GOOD_HW: Counting = Counting("testtc", 1000);
        static BAD_HW: Counting = Counting("badtc", -1);
        static GOOD: TcEntry = TcEntry::new(&GOOD_HW);
        static BAD: TcEntry = TcEntry::new(&BAD_HW);
        tc_init(&BAD);
        assert_ne!(
            timecounter().name(),
            "badtc",
            "negative quality is never chosen"
        );
        tc_init(&GOOD);
        assert_eq!(timecounter().name(), "testtc");
        assert_eq!(GOOD.precision(), 1);
        tc_windup(&mut WINDUP.lock(), None, None, None, None);
        assert_eq!(tc_getfrequency(), 100_000_000);
        // The choice lists both, newest first, with their qualities.
        let mut buf = [0u8; 64];
        let n = tc_choice(&mut buf);
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        assert!(s.starts_with("testtc(1000) badtc(-1)"), "{s}");
        // kern.timecounter.hardware: an unknown name is refused, a registered one selected.
        assert_eq!(tc_select(b"nosuch"), Err(crate::sys::errno::Errno::EINVAL));
        assert_eq!(tc_select(b"badtc"), Ok(()));
        assert_eq!(timecounter().name(), "badtc");
        assert_eq!(tc_select(b"testtc"), Ok(()));
        tc_reset_quality(&GOOD, -5);
        assert_ne!(
            timecounter().name(),
            "testtc",
            "demoted: the best other wins"
        );
        tc_windup(&mut WINDUP.lock(), None, None, None, None);
    }

    #[test]
    fn a_counter_too_slow_for_hz_gets_quality_minus_2000() {
        let _g = lock();
        // 8 bits at 1 MHz wrap 3921 times a second, more than hz 100 windups can follow.
        static HW: Settable = Settable {
            count: AtomicU32::new(0),
            mask: 0xff,
            freq: 1_000_000,
            name: "narrow",
            quality: 10,
        };
        static E: TcEntry = TcEntry::new(&HW);
        let printed = testutil::printf_count("Insufficient hz, needs at least 4313");
        tc_init(&E);
        assert_eq!(E.quality(), -2000);
        assert_eq!(E.precision(), 1);
        assert_ne!(timecounter().name(), "narrow");
        assert_eq!(
            testutil::printf_count("Insufficient hz, needs at least 4313"),
            printed + 1
        );
    }

    #[test]
    fn adjtime_and_adjfreq_round_trip() {
        let _g = lock();
        let mut old = 0i64;
        {
            let w = TC_LOCK.write();
            tc_adjtime(&w, Some(&mut old), Some(4_000_000));
            tc_adjtime(&w, Some(&mut old), None);
            // The windup that installs the delta also runs one NTP second (5000 ns of it).
            assert_eq!(old, 4_000_000 - 5000);
            tc_adjtime(&w, None, Some(0));
        }
        let mut f = 0i64;
        let w = RwWitness::Write(TC_LOCK.write());
        tc_adjfreq(&w, Some(&mut f), Some(12));
        tc_adjfreq(&w, Some(&mut f), None);
        assert_eq!(f, 12);
        tc_adjfreq(&w, None, Some(0));
    }

    #[test]
    fn a_reader_never_sees_a_torn_timehands() {
        let _g = lock();
        let stop = StdAtomicBool::new(false);
        std::thread::scope(|s| {
            s.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    tc_windup(&mut WINDUP.lock(), None, None, None, None);
                }
            });
            for _ in 0..200_000 {
                // th_microtime is boottime + offset at the windup that wrote them: a read
                // mixing two windups would not match.
                let (offset, boot, micro) =
                    read_timehands(|th| (th.offset.get(), th.boottime.get(), th.microtime.get()));
                let tv = boot.add(offset).to_timeval();
                assert_eq!((tv.tv_sec, tv.tv_usec), micro);
            }
            stop.store(true, Ordering::Relaxed);
        });
    }

    #[test]
    fn the_timekeep_page_follows_the_timehands() {
        let _g = lock();
        static PAGE: TimekeepAtomic = TimekeepAtomic::new();
        if TIMEKEEP.get().is_none() {
            assert!(timekeep_attach(&PAGE).is_ok());
        }
        tc_windup(&mut WINDUP.lock(), None, None, None, None);
        let Some(tk) = TIMEKEEP.get() else {
            panic!("attached")
        };
        let snap = tk.snapshot();
        let th = timehands();
        assert_eq!(snap.tk_version, TK_VERSION);
        assert_eq!(snap.tk_generation, th.generation.load(Ordering::Relaxed));
        assert_ne!(snap.tk_generation, 0);
        assert_eq!(snap.tk_scale, th.scale.load(Ordering::Relaxed));
        assert_eq!(snap.tk_offset.sec, th.offset.get().sec);
        assert_eq!(snap.tk_offset.frac, th.offset.get().frac);
        assert_eq!(snap.tk_counter_mask, th.counter().counter_mask());
        assert!(timekeep_attach(&PAGE).is_err(), "one page");
    }
}
/* </TESTS> */
