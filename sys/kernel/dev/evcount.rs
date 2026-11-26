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
 * Copyright (c) 2004 Artur Grabowski <art@openbsd.org>
 * Copyright (c) 2004 Aaron Campbell <aaron@openbsd.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES,
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY
 * AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
 * THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL  DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! Event counters, the interrupt counts `vmstat -i` shows: `kern/subr_evcount.c` and
//! `struct evcount` of `<sys/evcount.h>`, with the sysctl nodes `kern.intrcnt` and
//! `kern.evcount`.
//!
//! Upstream: sys/kern/subr_evcount.c @ 3ce1f3f79392
//! Upstream: sys/sys/evcount.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_evcount.rs@e8249449d905
//! LZ: sys/sys/evcount.rs@e8249449d905
//!
//! A driver embeds an [`Evcount`] in its interrupt record or softc, attaches it once with a
//! name and, for an interrupt, its vector, and bumps `ec_count` from its handler; `vmstat -i`
//! reads the counters in attach order through `kern.intrcnt`'s four leaves.
//!
//! ## Deviations
//! - `ec_count` is the first field and an `AtomicU64`: the amd64 interrupt stubs increment it
//!   in assembly through the handler's `ih_count`, and the arm64 controllers from Rust (LZ's).
//! - `kern.intrcnt` and `kern.evcount` with no name below them answer `ENOTDIR` (the C reads
//!   `name[0]` past the end) (LZ's).
//!
//! ## Redesign
//! - LZ's two `TailqHead`s behind an `unsafe impl Sync` whose comment says "attach time, on
//!   the boot CPU" -> two [`PinList`]s under one [`Mutexed`] at `IPL_HIGH`, held for each list
//!   step and for the read of a count (the C's `splhigh()` around it): the convention became
//!   the lock, and an interrupt handler bumping a count takes nothing, as in the C. A counter
//!   is linked through `Pin<&'static Evcount>`: its owner's detach unlinks it before the
//!   memory goes, as the C requires; a forgotten detach stops the kernel at the entry's drop.
//!   Cost: a `kern.intrcnt` read of a name, a vector or the number takes the lock (one spl
//!   pair) where the C walks the list with none, since a list a detach changes needs its
//!   lock to be walked; a count's read is the C's one pair.
//! - `struct evcount` is valid all zero, every member an atomic (`hal::sync::seqlock::StrSlot` for
//!   the name), so the zeroed interrupt records and softcs that embed one need no
//!   initialisation, as in the C (`docs/IDIOMS.md`, a statistics record drivers embed).
//! - `ec_data`, a `void *` the sysctl dereferenced as the counter's `int` vector, is the vector
//!   itself ([`Evcount::vector`]), read by the driver at attach: the only users give the
//!   address of an interrupt number fixed when the handler is established (`ih_irq`), so the
//!   value read once is the value the C reads at each sysctl; the stored pointer and its
//!   `unsafe` deref go. amd64's `intr_establish` sets `is_idtvec` after its attach and may
//!   change it on a re-establish: N10 attaches after the assignment and updates the source's
//!   counters (`docs/SCAFFOLDING.md`).
//! - The per-CPU counts (`ec_percpu`, `counters_alloc(1)`): one padded word per CPU
//!   (`M_COUNTERS`), indexed by `cpu_number()`, in a block of a static table of
//!   [`NPERCPU`] slots ([`PERCPU`]); the counter holds its slot's number (`ec_percpu`, 0 for
//!   none, so all zero is none), set under the list's `IPL_HIGH` mutex at attach (or by
//!   `evcount_init_percpu`) and cleared under it at detach. A slot's block is made once and
//!   never freed: a detached slot is recycled, zeroed, for the next per-CPU counter. So the
//!   interrupt path ([`evcount_inc`]) takes no lock and no count: one load of the number, the
//!   block reached through the table, one relaxed add on its own CPU's line, as the C's
//!   `counters_inc` writes only the CPU's own memory. A single counter needs no generation
//!   number (one word cannot tear), so it is not a `hal::sync::percpu::Counters<1>`, which holds
//!   `MAXCPUS` slots by value (16 KiB) and would be built on the stack of the record's
//!   constructor. Its only users are amd64's clock and IPI counters (N10).
//! - `evcount_detach` takes the counter off the list it is on (a counter that asked to be
//!   per-CPU before `evcount_init_percpu` waits on the second list; the C always names
//!   `evcount_list`).
//! - `evcount_sysctl` is two nodes, [`IntrcntNode`] (`kern.intrcnt`, the C's
//!   `sysctl_intrcnt`, which passes no new value) and [`EvcountNode`] (`kern.evcount`), over
//!   one function; the copy out happens after the list's lock is released.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

use hal::cpu::intr::IPL_HIGH;
use hal::ds::pin::pin_static;
use hal::ds::pinlist::{PinEntry, PinList};
use hal::mem::malloc::{M_WAITOK, MArray};
use hal::sync::mutex::Mutexed;
use hal::sync::once::OnceStatic;
use hal::sync::seqlock::StrSlot;
use hal::sys::errno::Errno;
use hal::sys::sysctl::{
    KERN_INTRCNT_CNT, KERN_INTRCNT_NAME, KERN_INTRCNT_NUM, KERN_INTRCNT_VECTOR,
};

use crate::assert::kassert;
use crate::sysctl::{SysctlNode, SysctlReq, rdint, rdquad, rdstring};

/// `M_COUNTERS`: per-CPU counters via counters_alloc(9) (`<sys/malloc.h>`).
pub const M_COUNTERS: i32 = 12;

/// One CPU's count, padded to a cache line (the C's `cpumem` gives each CPU its own
/// allocation); `malloc(9)` aligns a block of 64 bytes or more to its bucket, at least 64,
/// though it promises only `MALLOC_ALIGN` (16) to the type.
struct PcpuCount {
    /// The count.
    count: AtomicU64,
    /// The rest of the line.
    _pad: [u64; 7],
}

impl PcpuCount {
    /// A zero count.
    const ZERO: Self = PcpuCount {
        count: AtomicU64::new(0),
        _pad: [0; 7],
    };
}

/// How many counters may be per-CPU at once (a slot each in [`PERCPU`]); one more counts
/// on its shared `ec_count` (the same values, without the per-CPU lines).
pub const NPERCPU: usize = 32;

/// `struct evcount`. Valid all zero (a detached counter at 0); its owner calls
/// [`evcount_detach`] before the memory that holds it goes.
#[repr(C)]
pub struct Evcount {
    /// `ec_count`: main counter, bumped by the interrupt handlers without a lock.
    pub ec_count: AtomicU64,
    /// `ec_id`: counter ID, from 1 in attach order.
    ec_id: AtomicI32,
    /// `ec_name`: counter name.
    ec_name: StrSlot,
    /// The `int` `ec_data` pointed at: the interrupt vector, when `has_vector`.
    ec_vector: AtomicI32,
    /// Whether `ec_data` was non-NULL.
    has_vector: AtomicBool,
    /// Whether the counter waits on `evcount_percpu_init_list` instead of `evcount_list`.
    on_init_list: AtomicBool,
    /// `ec_percpu`: the per-CPU counts' slot in [`PERCPU`] plus one (0: `NULL`). Set and
    /// cleared only under [`EVCOUNT_LOCK`]; read without it by [`evcount_inc`].
    ec_percpu: AtomicU32,
    /// `next`: the link on `evcount_list` (or `evcount_percpu_init_list`).
    next: PinEntry<Evcount>,
}

impl Evcount {
    /// A detached, zero counter (all zero bytes).
    pub const fn new() -> Self {
        Self {
            ec_count: AtomicU64::new(0),
            ec_id: AtomicI32::new(0),
            ec_name: StrSlot::new(),
            ec_vector: AtomicI32::new(0),
            has_vector: AtomicBool::new(false),
            on_init_list: AtomicBool::new(false),
            ec_percpu: AtomicU32::new(0),
            next: PinEntry::new(),
        }
    }

    /// `ec_id`.
    pub fn id(&self) -> i32 {
        self.ec_id.load(Ordering::Relaxed)
    }

    /// `ec_name` (`""` before the attach).
    pub fn name(&self) -> &'static str {
        self.ec_name.get().unwrap_or("")
    }

    /// The interrupt vector given at attach (`*(int *)ec_data`), `None` for none (`NULL`).
    pub fn vector(&self) -> Option<i32> {
        self.has_vector
            .load(Ordering::Relaxed)
            .then(|| self.ec_vector.load(Ordering::Relaxed))
    }

    /// The count: the per-CPU counts' sum (`counters_read`) or `ec_count`.
    pub fn count(&self) -> u64 {
        match percpu_words(self.ec_percpu.load(Ordering::Acquire)) {
            Some(pc) => pc
                .iter()
                .fold(0u64, |s, c| s.wrapping_add(c.count.load(Ordering::Relaxed))),
            None => self.ec_count.load(Ordering::Relaxed),
        }
    }

    /// Whether the counter is attached (on one of the lists).
    pub fn is_attached(&self) -> bool {
        self.next.is_linked()
    }
}

impl Default for Evcount {
    fn default() -> Self {
        Self::new()
    }
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(, evcount)`: the list of counters, through `next`.
    EvcountList: Evcount, next
);

/// `evcount_list`, in attach order.
static EVCOUNT_LIST: PinList<EvcountList> = PinList::new();
/// `evcount_percpu_init_list`: the counters that asked to be per-CPU before the CPUs were
/// counted.
static EVCOUNT_PERCPU_INIT_LIST: PinList<EvcountList> = PinList::new();
/// The lists' lock (the C changes the lists at attach time and reads a count at
/// `splhigh()`).
static EVCOUNT_LOCK: Mutexed<()> = Mutexed::new(IPL_HIGH, ());
/// `evcount_percpu_done`.
static EVCOUNT_PERCPU_DONE: AtomicBool = AtomicBool::new(false);
/// `evcount_attach`'s `nextid`.
static NEXTID: AtomicI32 = AtomicI32::new(0);
/// The per-CPU counts' blocks (`counters_alloc(1)`'s `cpumem`), one per slot, each made the
/// first time its slot is taken and never freed.
static PERCPU: [OnceStatic<MArray<PcpuCount>>; NPERCPU] = [const { OnceStatic::new() }; NPERCPU];
/// The slots of [`PERCPU`] a counter holds, a bit each; changed under [`EVCOUNT_LOCK`].
static PERCPU_USED: AtomicU32 = AtomicU32::new(0);

// Soundness of the per-CPU path, which takes no lock (no `unsafe` is involved: this is why no
// reference it makes can dangle and why a count is never lost to the table's changes).
// - A slot's block lives in a `static` `OnceStatic`, set once by the one claimant of a slot
//   that has none and never freed or replaced: any slot number, read at any time, names a
//   live block, so `evcount_inc` may hold a reference to it across a concurrent detach.
// - `ec_percpu` is stored (Release) under `EVCOUNT_LOCK` only after the slot's block is made
//   and its words zeroed, and read with Acquire: a reader that sees the number sees the
//   block and the zeros, so no add of its is ordered before the zeroing.
// - Detach clears the number under the lock and frees the slot for reuse, never the block.
//   The owner's order is the C's (`ampintc_intr_disestablish`, `intr_disestablish`): the
//   handler leaves its interrupt's queue, then the counter is detached, then the record is
//   freed; a handler still running on another CPU in that window adds either to `ec_count`
//   or to the slot it read, alive. Only if the slot is taken again meanwhile does that add
//   land in the new owner's count: a miscount of the C's own race, never a stray write.

/// The block of the slot whose number plus one is `n` (`None` for 0, or a slot not made).
fn percpu_words(n: u32) -> Option<&'static MArray<PcpuCount>> {
    let i = usize::try_from(n.checked_sub(1)?).ok()?;
    PERCPU.get(i)?.get()
}

/// `counters_alloc(1)`: takes a free slot of [`PERCPU`] and gives back its number plus one,
/// the block made (`M_WAITOK`, outside the lock) or a recycled one zeroed; `None` when all
/// [`NPERCPU`] are held.
fn percpu_claim() -> Option<u32> {
    let i = {
        let _w = EVCOUNT_LOCK.lock();
        let used = PERCPU_USED.load(Ordering::Relaxed);
        let i = (!used).trailing_zeros();
        if i as usize >= NPERCPU {
            return None;
        }
        PERCPU_USED.store(used | 1 << i, Ordering::Relaxed);
        i
    };
    let slot = &PERCPU[i as usize];
    match slot.get() {
        Some(words) => {
            for w in words.iter() {
                w.count.store(0, Ordering::Relaxed);
            }
        }
        None => {
            let Some(words) =
                MArray::new(M_COUNTERS, M_WAITOK, hal::cpu::ncpus(), |_| PcpuCount::ZERO)
            else {
                percpu_release(i + 1);
                return None;
            };
            // This caller claimed the slot, which had no block: its `set` is the only one.
            let _ = slot.set(words);
        }
    }
    Some(i + 1)
}

/// `counters_free(cm, 1)`: the slot numbered `n` (plus one) free again; its block stays.
fn percpu_release(n: u32) {
    if let Some(i) = n.checked_sub(1)
        && (i as usize) < NPERCPU
    {
        let _w = EVCOUNT_LOCK.lock();
        PERCPU_USED.fetch_and(!(1 << i), Ordering::Relaxed);
    }
}

/// `evcount_attach`: registers `ec` as `name`, with `vector` (the interrupt number the C
/// passes by address as `data`) for `kern.intrcnt.vector`.
pub fn evcount_attach(ec: &'static Evcount, name: &'static str, vector: Option<i32>) {
    // memset(ec, 0, sizeof(*ec))
    ec.ec_count.store(0, Ordering::Relaxed);
    ec.ec_name.set(Some(name));
    ec.ec_id.store(
        NEXTID.fetch_add(1, Ordering::Relaxed) + 1,
        Ordering::Relaxed,
    );
    ec.ec_vector.store(vector.unwrap_or(0), Ordering::Relaxed);
    ec.has_vector.store(vector.is_some(), Ordering::Relaxed);
    ec.on_init_list.store(false, Ordering::Relaxed);
    let old = {
        let mut w = EVCOUNT_LOCK.lock();
        EVCOUNT_LIST.insert_tail(&mut w, pin_static(ec));
        ec.ec_percpu.swap(0, Ordering::Release)
    };
    percpu_release(old);
}

/// `evcount_percpu`: asks for a per-CPU counter for `ec`, attached.
pub fn evcount_percpu(ec: &'static Evcount) {
    if !EVCOUNT_PERCPU_DONE.load(Ordering::Relaxed) {
        let mut w = EVCOUNT_LOCK.lock();
        EVCOUNT_LIST.remove(&mut w, ec);
        EVCOUNT_PERCPU_INIT_LIST.insert_tail(&mut w, pin_static(ec));
        ec.on_init_list.store(true, Ordering::Relaxed);
    } else if let Some(n) = percpu_claim() {
        let old = {
            let _w = EVCOUNT_LOCK.lock();
            ec.ec_percpu.swap(n, Ordering::Release)
        };
        percpu_release(old);
    }
}

/// `evcount_init_percpu`: once the CPUs are counted, gives the waiting counters their per-CPU
/// counts, each starting with what it counted so far, and appends them to `evcount_list`.
pub fn evcount_init_percpu() {
    kassert!(!EVCOUNT_PERCPU_DONE.load(Ordering::Relaxed));

    // The slots are taken before the walk (a new block's `M_WAITOK` may sleep): one per
    // waiting counter, which only the boot CPU adds, at attach time.
    let n = EVCOUNT_PERCPU_INIT_LIST.iter(&EVCOUNT_LOCK.lock()).count();
    let mut slots = [0u32; NPERCPU];
    for s in slots.iter_mut().take(n) {
        *s = percpu_claim().unwrap_or(0);
    }
    let mut unused = slots.into_iter().filter(|&s| s != 0);
    {
        let mut w = EVCOUNT_LOCK.lock();
        for ec in EVCOUNT_PERCPU_INIT_LIST.iter(&w) {
            if let Some(s) = unused.next() {
                ec.ec_percpu.store(s, Ordering::Release);
                // What it counted so far, taken after the switch so that no add is lost
                // (the C reads and zeroes `ec_count` as two steps).
                let early = ec.ec_count.swap(0, Ordering::Relaxed);
                if let Some(c) = percpu_words(s).and_then(|pc| pc.first()) {
                    c.count.fetch_add(early, Ordering::Relaxed);
                }
            }
            ec.on_init_list.store(false, Ordering::Relaxed);
        }
        EVCOUNT_LIST.concat(&mut w, &EVCOUNT_PERCPU_INIT_LIST);
    }
    // Slots no waiting counter took (the list cannot grow meanwhile on the boot CPU).
    unused.for_each(percpu_release);
    EVCOUNT_PERCPU_DONE.store(true, Ordering::Relaxed);
}

/// `evcount_detach`: unregisters `ec`, its per-CPU slot free again (the block is kept for
/// the next per-CPU counter: see the soundness note above [`percpu_words`]). The owner calls
/// it after its handler has left the interrupt's queue and before it frees the record.
pub fn evcount_detach(ec: &'static Evcount) {
    let old = {
        let mut w = EVCOUNT_LOCK.lock();
        if ec.on_init_list.swap(false, Ordering::Relaxed) {
            EVCOUNT_PERCPU_INIT_LIST.remove(&mut w, ec);
        } else {
            EVCOUNT_LIST.remove(&mut w, ec);
        }
        ec.ec_percpu.swap(0, Ordering::Release)
    };
    percpu_release(old);
}

/// `evcount_inc`: counts one event, on this CPU's line when `ec` is per-CPU: one load of the
/// slot number and one relaxed add, no lock and no count taken (from interrupt context).
pub fn evcount_inc(ec: &Evcount) {
    if let Some(c) = percpu_words(ec.ec_percpu.load(Ordering::Acquire))
        .and_then(|pc| pc.get(hal::cpu::cpu_number()))
    {
        c.count.fetch_add(1, Ordering::Relaxed);
        return;
    }
    ec.ec_count.fetch_add(1, Ordering::Relaxed);
}

/// What one `kern.intrcnt` leaf reads, taken under the list's lock.
enum Leaf {
    /// `KERN_INTRCNT_NUM`: the number of counters.
    Num(i32),
    /// `KERN_INTRCNT_CNT`.
    Cnt(u64),
    /// `KERN_INTRCNT_NAME`.
    Name(&'static str),
    /// `KERN_INTRCNT_VECTOR`.
    Vector(i32),
}

/// `evcount_sysctl`: `name` is `KERN_INTRCNT_NUM` (the number of counters) or
/// `<KERN_INTRCNT_CNT|NAME|VECTOR>.<index>` (the index-th counter's count, name or interrupt
/// vector). Read-only: `EPERM` for a new value (`kern.intrcnt` passes none).
fn evcount_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    if req.newp != 0 {
        return Err(Errno::EPERM);
    }
    let Some(&what) = name.first() else {
        return Err(Errno::ENOTDIR);
    };
    let i = if what == KERN_INTRCNT_NUM {
        None
    } else {
        if name.len() != 2 {
            return Err(Errno::ENOTDIR);
        }
        let Ok(i) = usize::try_from(name[1]) else {
            return Err(Errno::EINVAL);
        };
        Some(i)
    };

    // The C's walk, counting to `i` (or to the end), under the lock.
    let leaf = {
        let w = EVCOUNT_LOCK.lock();
        match i {
            None => Leaf::Num(EVCOUNT_LIST.iter(&w).count() as i32),
            Some(i) => {
                let ec = EVCOUNT_LIST.iter(&w).nth(i);
                match (what, ec) {
                    (KERN_INTRCNT_CNT | KERN_INTRCNT_NAME | KERN_INTRCNT_VECTOR, None) => {
                        return Err(Errno::ENOENT);
                    }
                    (KERN_INTRCNT_CNT, Some(ec)) => Leaf::Cnt(ec.count()),
                    (KERN_INTRCNT_NAME, Some(ec)) => Leaf::Name(ec.name()),
                    (KERN_INTRCNT_VECTOR, Some(ec)) => {
                        Leaf::Vector(ec.vector().ok_or(Errno::ENOENT)?)
                    }
                    _ => return Err(Errno::EOPNOTSUPP),
                }
            }
        }
    };

    match leaf {
        Leaf::Num(n) => rdint(req, n),
        Leaf::Cnt(c) => rdquad(req, c as i64),
        Leaf::Name(s) => rdstring(req, s.as_bytes()),
        Leaf::Vector(v) => rdint(req, v),
    }
}

/// `kern.intrcnt` (`KERN_INTRCNT`): `sysctl_intrcnt`, `evcount_sysctl` with no new value.
pub struct IntrcntNode;

impl SysctlNode for IntrcntNode {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        let mut q = SysctlReq {
            oldp: req.oldp,
            oldlenp: &mut *req.oldlenp,
            newp: 0,
            newlen: 0,
        };
        evcount_sysctl(name, &mut q)
    }
}

/// `kern.evcount` (`KERN_EVCOUNT`): `evcount_sysctl`.
pub struct EvcountNode;

impl SysctlNode for EvcountNode {
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        evcount_sysctl(name, req)
    }
}

// The amd64 interrupt stubs bump `ec_count` through the handler's `ih_count` address.
const _: () = assert!(core::mem::offset_of!(Evcount, ec_count) == 0);

/// Forgets every counter an earlier test attached (they are unlinked, not freed) and the
/// per-CPU step, for the host tests (`testing.md`).
#[cfg(any(test, feature = "doubles"))]
pub fn evcount_test_reset() {
    let mut w = EVCOUNT_LOCK.lock();
    while EVCOUNT_LIST.remove_first(&mut w) {}
    while EVCOUNT_PERCPU_INIT_LIST.remove_first(&mut w) {}
    EVCOUNT_PERCPU_DONE.store(false, Ordering::Relaxed);
    // The slots' blocks are statics' and stay; which slots are held is forgotten.
    PERCPU_USED.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The counters' list, the four `kern.intrcnt` leaves and their refusals as the C's, the
    // vector given at attach, the per-CPU step.

    use std::sync::{Mutex, MutexGuard};
    use std::vec::Vec;

    use hal::user::UserPtr;

    use super::*;
    use crate::testutil;

    /// The tests share the process's one list.
    fn setup() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        testutil::install();
        evcount_test_reset();
        g
    }

    /// The leaf `name` of `node`, copied to the user address `u` with `room` bytes.
    fn read(
        node: &dyn SysctlNode,
        name: &[i32],
        u: usize,
        room: usize,
        newp: usize,
    ) -> Result<Vec<u8>, Errno> {
        let mut len = room;
        let mut q = SysctlReq {
            oldp: u,
            oldlenp: &mut len,
            newp,
            newlen: if newp != 0 { 4 } else { 0 },
        };
        node.sysctl(name, &mut q)?;
        Ok(testutil::user_bytes(u, len).unwrap_or_default())
    }

    #[test]
    fn attach_count_detach() {
        let _g = setup();
        static A: Evcount = Evcount::new();
        static B: Evcount = Evcount::new();
        evcount_attach(&A, "com0", None);
        evcount_attach(&B, "pckbc0", None);
        assert!(A.id() >= 1);
        assert_eq!(B.id(), A.id() + 1);
        assert_eq!(A.name(), "com0");
        evcount_inc(&A);
        evcount_inc(&A);
        A.ec_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(A.count(), 3);
        assert!(A.is_attached());
        evcount_detach(&A);
        assert!(!A.is_attached());
        evcount_detach(&B);
        assert!(EVCOUNT_LIST.is_empty(&EVCOUNT_LOCK.lock()));
    }

    #[test]
    fn the_four_leaves_read_the_counters_in_attach_order() {
        let _g = setup();
        static IRQ: Evcount = Evcount::new();
        static SPUR: Evcount = Evcount::new();
        evcount_attach(&IRQ, "irq33/virtio0", Some(33));
        evcount_attach(&SPUR, "irq1023/spur", None);
        IRQ.ec_count.fetch_add(7, Ordering::Relaxed);
        let u = testutil::USER_BASE + 0xE000;

        let n = read(&IntrcntNode, &[KERN_INTRCNT_NUM], u, 4, 0);
        assert_eq!(n, Ok(2i32.to_ne_bytes().to_vec()));
        let c = read(&IntrcntNode, &[KERN_INTRCNT_CNT, 0], u, 8, 0);
        assert_eq!(c, Ok(7i64.to_ne_bytes().to_vec()));
        let s = read(&IntrcntNode, &[KERN_INTRCNT_NAME, 1], u, 64, 0);
        assert_eq!(s, Ok(b"irq1023/spur\0".to_vec()));
        let v = read(&EvcountNode, &[KERN_INTRCNT_VECTOR, 0], u, 4, 0);
        assert_eq!(v, Ok(33i32.to_ne_bytes().to_vec()));
        // A string leaf's length with no room: the length, and ENOMEM with too little.
        let mut len = 0;
        let mut q = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(IntrcntNode.sysctl(&[KERN_INTRCNT_NAME, 0], &mut q), Ok(()));
        assert_eq!(len, b"irq33/virtio0\0".len());
        assert_eq!(
            read(&IntrcntNode, &[KERN_INTRCNT_NAME, 0], u, 3, 0),
            Err(Errno::ENOMEM)
        );

        evcount_detach(&IRQ);
        evcount_detach(&SPUR);
    }

    #[test]
    fn the_refusals_are_the_cs() {
        let _g = setup();
        static ONE: Evcount = Evcount::new();
        evcount_attach(&ONE, "one", None);
        let u = testutil::USER_BASE + 0xE100;
        let newp = testutil::USER_BASE + 0xE200;
        UserPtr::<i32>::new(newp).copyout(&1).unwrap();
        // (node, name, a new value, the answer)
        let cases: &[(&dyn SysctlNode, &[i32], bool, Result<(), Errno>)] = &[
            (&EvcountNode, &[KERN_INTRCNT_NUM], true, Err(Errno::EPERM)),
            (&EvcountNode, &[KERN_INTRCNT_NUM], false, Ok(())),
            // kern.intrcnt passes no new value: the C's sysctl_intrcnt.
            (&IntrcntNode, &[KERN_INTRCNT_NUM], true, Ok(())),
            (&EvcountNode, &[KERN_INTRCNT_NUM, 5], false, Ok(())),
            (&EvcountNode, &[], false, Err(Errno::ENOTDIR)),
            (
                &EvcountNode,
                &[KERN_INTRCNT_CNT],
                false,
                Err(Errno::ENOTDIR),
            ),
            (
                &EvcountNode,
                &[KERN_INTRCNT_CNT, 0, 0],
                false,
                Err(Errno::ENOTDIR),
            ),
            (
                &EvcountNode,
                &[KERN_INTRCNT_NAME, -1],
                false,
                Err(Errno::EINVAL),
            ),
            (
                &EvcountNode,
                &[KERN_INTRCNT_CNT, 1],
                false,
                Err(Errno::ENOENT),
            ),
            (
                &EvcountNode,
                &[KERN_INTRCNT_NAME, i32::MAX],
                false,
                Err(Errno::ENOENT),
            ),
            // No vector: ENOENT, as a NULL ec_data.
            (
                &EvcountNode,
                &[KERN_INTRCNT_VECTOR, 0],
                false,
                Err(Errno::ENOENT),
            ),
            (&EvcountNode, &[99, 0], false, Err(Errno::EOPNOTSUPP)),
            (&EvcountNode, &[99, 7], false, Err(Errno::EOPNOTSUPP)),
            (&EvcountNode, &[99], false, Err(Errno::ENOTDIR)),
        ];
        for (i, (node, name, new, want)) in cases.iter().enumerate() {
            let got = read(*node, name, u, 64, if *new { newp } else { 0 }).map(|_| ());
            assert_eq!(got, *want, "case {i}: {name:?}");
        }
        evcount_detach(&ONE);
    }

    #[test]
    fn per_cpu_counts_fold_the_early_count_in() {
        let _g = setup();
        static CLK: Evcount = Evcount::new();
        static IPI: Evcount = Evcount::new();
        static LATE: Evcount = Evcount::new();
        evcount_attach(&CLK, "clock", Some(0));
        evcount_attach(&IPI, "ipi", None);
        evcount_percpu(&CLK);
        // Waiting for the CPUs: off evcount_list, as in the C.
        assert_eq!(EVCOUNT_LIST.iter(&EVCOUNT_LOCK.lock()).count(), 1);
        evcount_inc(&CLK);
        CLK.ec_count.fetch_add(4, Ordering::Relaxed);
        evcount_init_percpu();
        assert_eq!(CLK.count(), 5);
        assert_eq!(CLK.ec_count.load(Ordering::Relaxed), 0);
        evcount_inc(&CLK);
        evcount_inc(&CLK);
        assert_eq!(CLK.count(), 7);
        assert_eq!(CLK.ec_count.load(Ordering::Relaxed), 0);
        // Appended after the others, as TAILQ_CONCAT.
        let names: Vec<&str> = EVCOUNT_LIST
            .iter(&EVCOUNT_LOCK.lock())
            .map(Evcount::name)
            .collect();
        assert_eq!(names, ["ipi", "clock"]);
        // After the step, a counter gets its per-CPU counts at once.
        evcount_attach(&LATE, "late", None);
        evcount_percpu(&LATE);
        evcount_inc(&LATE);
        assert_eq!(LATE.count(), 1);
        assert_eq!(LATE.ec_count.load(Ordering::Relaxed), 0);
        evcount_detach(&CLK);
        evcount_detach(&IPI);
        evcount_detach(&LATE);
        assert_eq!(CLK.count(), 0);
    }

    #[test]
    fn a_detached_slot_is_recycled_zeroed_and_a_full_table_counts_on_the_word() {
        let _g = setup();
        // After the boot step: a per-CPU counter gets its slot at once.
        EVCOUNT_PERCPU_DONE.store(true, Ordering::Relaxed);
        static C: [Evcount; NPERCPU + 1] = [const { Evcount::new() }; NPERCPU + 1];
        for c in &C {
            evcount_attach(c, "pc", None);
            evcount_percpu(c);
        }
        // The table is full: the last one counts on its shared word, the same values.
        assert_eq!(C[NPERCPU].ec_percpu.load(Ordering::Relaxed), 0);
        evcount_inc(&C[NPERCPU]);
        assert_eq!(C[NPERCPU].count(), 1);
        assert_eq!(C[NPERCPU].ec_count.load(Ordering::Relaxed), 1);
        let s0 = C[0].ec_percpu.load(Ordering::Relaxed);
        assert_ne!(s0, 0);
        evcount_inc(&C[0]);
        evcount_inc(&C[0]);
        assert_eq!(C[0].count(), 2);
        assert_eq!(C[0].ec_count.load(Ordering::Relaxed), 0);
        // Detached, its slot goes to the next per-CPU counter, zeroed.
        evcount_detach(&C[0]);
        assert_eq!(C[0].ec_percpu.load(Ordering::Relaxed), 0);
        static D: Evcount = Evcount::new();
        evcount_attach(&D, "next", None);
        evcount_percpu(&D);
        assert_eq!(D.ec_percpu.load(Ordering::Relaxed), s0);
        assert_eq!(D.count(), 0);
        evcount_inc(&D);
        assert_eq!(D.count(), 1);
        evcount_detach(&D);
        for c in &C[1..] {
            evcount_detach(c);
        }
        assert_eq!(PERCPU_USED.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_counter_waiting_for_the_cpus_detaches_from_its_list() {
        let _g = setup();
        static W: Evcount = Evcount::new();
        evcount_attach(&W, "waiting", None);
        evcount_percpu(&W);
        evcount_detach(&W);
        assert!(!W.is_attached());
        assert!(EVCOUNT_PERCPU_INIT_LIST.is_empty(&EVCOUNT_LOCK.lock()));
    }
}
/* </TESTS> */
