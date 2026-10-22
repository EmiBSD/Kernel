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
 * Copyright (c) 2016 David Gwynne <dlg@openbsd.org>
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
 * Copyright (c) 2016 David Gwynne <dlg@openbsd.org>
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
//! `PerCpu<T>` and `Counters<N>`: per-CPU memory and the per-CPU counters of `percpu(9)`.
//!
//! Upstream: sys/kern/subr_percpu.c @ 3ce1f3f79392
//! Upstream: sys/sys/percpu.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_percpu.rs@3c62ede68802
//! LZ: sys/sys/percpu.rs@3c62ede68802
//!
//! The behaviour of `cpumem_*(9)` and `counters_*(9)`: each CPU has its own copy, on its
//! own cache line, which it reaches through `cpumem_enter` (the slot of `cpu_number()`)
//! without a lock; a reader walks every CPU's copy (`CPUMEM_FOREACH`, the `ncpusfound`
//! CPUs). A counter set adds a generation number per CPU (`MULTIPROCESSOR`'s, the only
//! kernel built since decision 40): an update makes it odd, then even again, and
//! `counters_read` retries a CPU's copy while the number is odd or changed under it, so the
//! sum it returns is made of whole updates.
//!
//! The running CPU's index is its core's (`cpu.rs`): 0 before the boot CPU's core is
//! installed, and `ncpusfound` is 1 before any CPU is attached.
//!
//! ## Deviations
//! - `counters_read` waits for an odd generation with `CPU_BUSY_CYCLE` where the C calls
//!   `yield()`: the scheduler is not `hal`'s, and an update is a few stores long.
//! - The C's uniprocessor counters (one set, no generation, read and zeroed at `splhigh()`)
//!   are not here: the kernel is always `MULTIPROCESSOR` (decision 40).
//!
//! ## Redesign
//! - LZ's `cpumem` is a handle to `ncpusfound` pool or `malloc` allocations, reached by raw
//!   pointer (`cpumem_enter` -> `NonNull<u8>`, cast by every user). `PerCpu<T>` holds the
//!   copies by value, `MAXCPUS` cache-line aligned slots typed `T`, so a slot is `&T` and the
//!   type is `Sync` exactly when `T` is: the per-CPU discipline (each CPU writes its own
//!   copy) is `T`'s interior mutability, atomics for the counters. No allocation, no
//!   `unsafe`; the cost is `MAXCPUS` slots instead of `ncpusfound` (16 KiB per instance at
//!   256 CPUs and 64 bytes a slot), so an instance lives in a static or in the object that
//!   owns it, never on a kernel stack.
//! - `counters_alloc(n)` is [`Counters::new`], a `const fn` (a static needs no boot-time
//!   allocation); `counters_enter`/`counters_leave` are [`Counters::enter`] and the drop of
//!   the [`CountersRef`] it returns; `counters_read` returns the sums instead of filling an
//!   output array with a scratch buffer.

use core::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering, fence};

use crate::cpu::{self, MAXCPUS};

/// One CPU's copy, alone on its cache line (`CACHELINESIZE`, 64 on amd64 and arm64).
#[repr(align(64))]
struct Slot<T>(T);

/// A value with a constant initial state, so that a [`PerCpu`] of it can be built by a
/// `const fn` (a static): the atomics start at 0.
pub trait PerCpuInit: Sized {
    /// The initial value of every CPU's copy.
    const INIT: Self;
}

impl PerCpuInit for AtomicU64 {
    const INIT: Self = AtomicU64::new(0);
}

impl PerCpuInit for AtomicI64 {
    const INIT: Self = AtomicI64::new(0);
}

impl PerCpuInit for AtomicU32 {
    const INIT: Self = AtomicU32::new(0);
}

impl PerCpuInit for AtomicUsize {
    const INIT: Self = AtomicUsize::new(0);
}

impl<T: PerCpuInit, const N: usize> PerCpuInit for [T; N] {
    const INIT: Self = [const { T::INIT }; N];
}

/// One copy of `T` per CPU (`struct cpumem`).
pub struct PerCpu<T> {
    slots: [Slot<T>; MAXCPUS],
}

impl<T> PerCpu<T> {
    /// Every CPU's copy at `T::INIT` (`cpumem_get`/`cpumem_malloc` of zeroed memory).
    pub const fn new() -> Self
    where
        T: PerCpuInit,
    {
        Self {
            slots: [const { Slot(T::INIT) }; MAXCPUS],
        }
    }

    /// CPU `i`'s copy made by `f(i)`. Builds the whole value on the caller's stack: for a
    /// small `T` or outside a kernel stack only (see the module's notes).
    pub fn from_fn(mut f: impl FnMut(usize) -> T) -> Self {
        Self {
            slots: core::array::from_fn(|i| Slot(f(i))),
        }
    }

    /// `cpumem_enter`: the running CPU's copy.
    pub fn get(&self) -> &T {
        &self.slots[cpu::cpu_number()].0
    }

    /// CPU `i`'s copy, `None` past `MAXCPUS`.
    pub fn cpu(&self, i: usize) -> Option<&T> {
        self.slots.get(i).map(|s| &s.0)
    }

    /// `CPUMEM_FOREACH`: the copies of the `ncpusfound` CPUs, in CPU order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots[..ncpus()].iter().map(|s| &s.0)
    }
}

impl<T: PerCpuInit> Default for PerCpu<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// One CPU's counters: the generation number and the `N` counters.
pub struct CounterWords<const N: usize> {
    /// The generation number: odd during an update.
    generation: AtomicU64,
    counters: [AtomicU64; N],
}

impl<const N: usize> PerCpuInit for CounterWords<N> {
    const INIT: Self = CounterWords {
        generation: AtomicU64::new(0),
        counters: [const { AtomicU64::new(0) }; N],
    };
}

/// `N` per-CPU counters (`counters_alloc(N)`): updated by each CPU on its own copy without a
/// lock, summed by [`Counters::read`].
pub struct Counters<const N: usize> {
    cpus: PerCpu<CounterWords<N>>,
}

impl<const N: usize> Counters<N> {
    /// `counters_alloc(N)`: every counter of every CPU at 0.
    pub const fn new() -> Self {
        Self {
            cpus: PerCpu::new(),
        }
    }

    /// `counters_enter`: this CPU's counters, its generation number odd until the returned
    /// reference drops (`counters_leave`).
    pub fn enter(&self) -> CountersRef<'_, N> {
        let words = self.cpus.get();
        // An odd number tells a reader that this copy is being written.
        let g = words.generation.load(Ordering::Relaxed);
        words.generation.store(g.wrapping_add(1), Ordering::Relaxed);
        fence(Ordering::Release); // membar_producer()
        CountersRef { words }
    }

    /// `counters_inc(cm, c)`.
    pub fn inc(&self, c: usize) {
        self.enter().counters()[c].fetch_add(1, Ordering::Relaxed);
    }

    /// `counters_dec(cm, c)`.
    pub fn dec(&self, c: usize) {
        self.enter().counters()[c].fetch_sub(1, Ordering::Relaxed);
    }

    /// `counters_add(cm, c, v)`.
    pub fn add(&self, c: usize, v: u64) {
        self.enter().counters()[c].fetch_add(v, Ordering::Relaxed);
    }

    /// `counters_pkt(cm, c, b, v)`: one more packet in `c`, `v` more bytes in `b`.
    pub fn pkt(&self, c: usize, b: usize, v: u64) {
        let r = self.enter();
        r.counters()[c].fetch_add(1, Ordering::Relaxed);
        r.counters()[b].fetch_add(v, Ordering::Relaxed);
    }

    /// `counters_read`: the sum of every CPU's counters, each CPU's copy read whole (retried
    /// while its generation number is odd or moves under the read).
    pub fn read(&self) -> [u64; N] {
        let mut out = [0u64; N];
        for words in self.cpus.iter() {
            let temp = read_whole(words);
            for (o, t) in out.iter_mut().zip(temp) {
                *o = o.wrapping_add(t);
            }
        }
        out
    }

    /// `counters_zero`: every counter of every CPU back to 0, the generation numbers too.
    pub fn zero(&self) {
        fence(Ordering::Release); // membar_producer()
        for words in self.cpus.iter() {
            for c in &words.counters {
                c.store(0, Ordering::Relaxed);
            }
            // The number goes back to 0 with the counters it guards.
            fence(Ordering::Release); // membar_producer()
            words.generation.store(0, Ordering::Relaxed);
        }
    }
}

impl<const N: usize> Default for Counters<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct counters_ref`: this CPU's counters during an update; its drop is `counters_leave`.
pub struct CountersRef<'a, const N: usize> {
    words: &'a CounterWords<N>,
}

impl<const N: usize> CountersRef<'_, N> {
    /// The counters to update.
    pub fn counters(&self) -> &[AtomicU64; N] {
        &self.words.counters
    }
}

impl<const N: usize> Drop for CountersRef<'_, N> {
    fn drop(&mut self) {
        fence(Ordering::Release); // membar_producer()
        // Even again: the copy is whole.
        let g = self.words.generation.load(Ordering::Relaxed);
        self.words
            .generation
            .store(g.wrapping_add(1), Ordering::Release);
    }
}

/// `ncpusfound`, between 1 and `MAXCPUS` (`cpu::ncpus`).
fn ncpus() -> usize {
    cpu::ncpus()
}

/// One CPU's counters, read whole: a snapshot is kept only when the generation number was
/// even before it and unchanged after it, so no update was under way or happened in between.
fn read_whole<const N: usize>(words: &CounterWords<N>) -> [u64; N] {
    loop {
        let before = stable_generation(words);
        fence(Ordering::Acquire); // membar_consumer()
        let snapshot: [u64; N] =
            core::array::from_fn(|i| words.counters[i].load(Ordering::Relaxed));
        fence(Ordering::Acquire); // membar_consumer()
        if words.generation.load(Ordering::Relaxed) == before {
            return snapshot;
        }
    }
}

/// The copy's generation number once no update is under way (an even number).
fn stable_generation<const N: usize>(words: &CounterWords<N>) -> u64 {
    loop {
        let g = words.generation.load(Ordering::Acquire);
        if g & 1 == 0 {
            return g;
        }
        core::hint::spin_loop();
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    #[test]
    fn a_static_per_cpu_value_starts_at_zero_and_is_per_cpu() {
        testutil::install();
        static P: PerCpu<AtomicU64> = PerCpu::new();
        P.get().fetch_add(2, Ordering::Relaxed);
        assert!(P.get().load(Ordering::Relaxed) >= 2);
        assert_eq!(P.iter().count(), ncpus());
        assert!(P.cpu(MAXCPUS).is_none());
    }

    #[test]
    fn from_fn_numbers_the_copies() {
        let p = PerCpu::from_fn(|i| i);
        assert_eq!(p.cpu(0), Some(&0));
        assert_eq!(p.cpu(MAXCPUS - 1), Some(&(MAXCPUS - 1)));
    }

    #[test]
    fn counters_sum_every_update() {
        testutil::install();
        let c = Arc::new(Counters::<3>::new());
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let c = Arc::clone(&c);
                thread::spawn(move || {
                    for _ in 0..1000 {
                        c.inc(0);
                        c.add(1, 3);
                        c.pkt(2, 1, 10);
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(c.read(), [4000, 4000 * 13, 4000]);
        c.dec(0);
        assert_eq!(c.read()[0], 3999);
        c.zero();
        assert_eq!(c.read(), [0, 0, 0]);
    }

    #[test]
    fn the_generation_is_even_outside_an_update() {
        testutil::install();
        let c = Counters::<1>::new();
        {
            let r = c.enter();
            r.counters()[0].fetch_add(1, Ordering::Relaxed);
            assert_eq!(c.cpus.get().generation.load(Ordering::Relaxed) & 1, 1);
        }
        assert_eq!(c.cpus.get().generation.load(Ordering::Relaxed) & 1, 0);
        assert_eq!(c.read(), [1]);
    }
}
/* </TESTS> */
