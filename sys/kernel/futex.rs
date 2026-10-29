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
 * Copyright (c) 2016-2017 Martin Pieuchot
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
//! `futex(2)`: a thread waits on a 32-bit word of user memory until another wakes it, or
//! moves it to another word (`FUTEX_WAIT`, `FUTEX_WAKE`, `FUTEX_REQUEUE` and their `_PRIVATE`
//! forms; libc's mutexes, condition variables and `__thrsleep` are built on them).
//!
//! Upstream: sys/kern/sys_futex.c @ 3ce1f3f79392
//! LZ: sys/kern/sys_futex.rs@3c62ede68802
//!
//! A waiter is a [`Futex`] on its own stack, pinned, linked on one of 64 buckets
//! (`futex_slpques`) chosen by the word's key, and asleep on its own address ("fsleep"). A
//! key names the word: the process and the address for a private futex, else the object (or
//! amap) and offset of a `MAP_INHERIT_SHARE` mapping, else the address alone. Each bucket has
//! its rwlock ("futexlk"); a requeue between two buckets takes both in the order of their
//! random ids, as the C.
//!
//! The rule the C states for `ft_proc` holds here for the waiter's link: only a holder of the
//! bucket lock takes a waiter off a bucket, and once it is off its waiter may return and its
//! frame go; the waiter itself reads its link without the lock. Errors are the C's: `EINVAL`
//! (a bad timeout), `ETIMEDOUT` (a zero timeout, or the timeout's end), `EAGAIN` (the word
//! changed), `EFAULT` (the word or the timeout unreadable, a misaligned word), `EINTR` or
//! `ECANCELED` (a signal: `ERESTART` becomes `ECANCELED`), `ENOSYS` (an unknown operation).
//!
//! ## Deviations
//! - `KTRACE` is not configured (`ktrreltimespec`).
//!
//! ## Redesign
//! - LZ's `struct futex` on the stack linked through raw `TailqEntry` links, inserted and
//!   removed in `unsafe` blocks, with `ft_proc` an `AtomicPtr<Proc>` read by the waiter and
//!   swapped by the waker, `ft_fsq` an `AtomicPtr` dereferenced in `futex_unwait`, the keys
//!   `Cell<*const ..>` and the buckets `unsafe impl Sync` (9 `unsafe`) -> a [`Futex`] pinned
//!   on the waiter's stack (`core::pin::pin!`) on a `hal::pinlist::PinList` bound to its
//!   bucket's `hal::rwlock::RawRwlock`, 0 `unsafe`: "waiting" (`ft_proc != NULL`) **is** being
//!   linked ([`Futex::waiting`], the entry's own word, cleared last by the unlink itself), so
//!   a waker never touches a futex after letting it go, and a frame dropped while linked stops
//!   the kernel (the entry's drop) instead of leaving a dangling link. `ft_fsq` is the
//!   bucket's index; the key is four words compared, never followed ([`FutexKey`]).
//! - The waker's `wakeup_proc(ft_proc)` is a wakeup of the waiter's own channel, the futex's
//!   address it sleeps on ([`crate::sched::wakeup_n_locked`]), under the one `SCHED_LOCK`
//!   hold the C takes for the batch: no reference to the waiting thread is kept in a list
//!   that outlives it. The same thread is woken, at the same point; the difference is inside
//!   the kernel only: a waiter asleep elsewhere (a page fault in its `copyin32`) is not cut
//!   short, which the C's `wakeup_proc` would do (and the fault would sleep again), and the
//!   wakeup walks the channel's sleep queue (`LOOKUP(ident)`) where the C has the thread.
//! - `FUTEX_REQUEUE` between two buckets moves the waiters with
//!   `PinList::move_where_across` under both locks: a waiter is never off a list while it
//!   moves, so it cannot take the move for its wakeup. Its second walk starts from the head
//!   rather than from the element after the n-th woken one: every element before that one
//!   was walked by the first loop and does not match (the matching ones were woken), so the
//!   same waiters move in the same order; a requeue onto its own bucket goes through a
//!   local list spliced back at the tail, the C's "only to the tail of the list as it is
//!   now".
//! - The private key's process and the shared key's object and amap are addresses
//!   (`usize`), as the C compares pointers it never follows.

use core::pin::{Pin, pin};
use core::ptr;
use core::sync::atomic::{AtomicI64, AtomicU32, AtomicUsize, Ordering};

use hal::pinlist::{PinEntry, PinList};
use hal::rwlock::{RawRwlock, WriteWitness};
use hal::sleep::{INFSLP, PCATCH, PWAIT};
use hal::sys::errno::Errno;
use hal::sys::futex::{
    FUTEX_FLAG_MASK, FUTEX_OP_MASK, FUTEX_PRIVATE_FLAG, FUTEX_REQUEUE, FUTEX_WAIT, FUTEX_WAKE,
};
use hal::sys::mman::MAP_INHERIT_SHARE;
use hal::thread::sched_lock;
use hal::user::UserPtr;
use hal::vm::param::ptoa;

use crate::assert::kpanic;
use crate::proc::{ProcGlue, Thread};
use crate::sched::{sleep_finish, sleep_setup, wakeup_n_locked};
use crate::sig::UserTimespec;
use crate::vm::map::{VmMap, uvm_et_isobj};
use crate::vm::types::Voff;

/// `FT_PRIVATE`: the futex is process-private (`futex_addrs`' flags sit in
/// `FUTEX_OP_MASK` space).
const FT_PRIVATE: i32 = FUTEX_PRIVATE_FLAG;

/// `FUTEX_SLPQUES_BITS`.
const FUTEX_SLPQUES_BITS: u32 = 6;
/// `FUTEX_SLPQUES_SIZE`.
const FUTEX_SLPQUES_SIZE: usize = 1 << FUTEX_SLPQUES_BITS;
/// `FUTEX_SLPQUES_MASK`.
const FUTEX_SLPQUES_MASK: u32 = FUTEX_SLPQUES_SIZE as u32 - 1;

/// `MAXTSLP`: the longest sleep that is not `INFSLP`.
const MAXTSLP: u64 = u64::MAX - 1;

/// The userland address a futex waits on (`ft_ps`, `ft_obj`, `ft_amap`, `ft_off`): the
/// process (private), or the object or amap and the offset (a shared mapping), or the address
/// alone. Addresses compared, never followed; 0 is the C's `NULL`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct FutexKey {
    /// `ft_ps`: the process, for a private futex.
    pub ps: usize,
    /// `ft_obj`: the UVM object of a shared mapping.
    pub obj: usize,
    /// `ft_amap`: the amap of a shared anonymous mapping.
    pub amap: usize,
    /// `ft_off`: the offset in the object or amap, else the address.
    pub off: Voff,
}

/// `struct futex`: a waiting thread's futex, pinned on its stack while it is on a bucket.
///
/// \[f\] fields are the bucket lock's: the key is written by a requeue holding both buckets'
/// locks and read under the lock of the bucket the futex is on, as atomics so the type is
/// shared without a lie (the C's `volatile`).
struct Futex {
    /// \[f\] `ft_entry`: the link on the bucket's list; linked is the C's `ft_proc != NULL`.
    entry: PinEntry<Futex>,
    /// \[f\] `ft_fsq`: the index of the bucket the futex is on.
    fsq: AtomicUsize,
    /// \[f\] `ft_ps`.
    ps: AtomicUsize,
    /// \[f\] `ft_obj`.
    obj: AtomicUsize,
    /// \[f\] `ft_amap`.
    amap: AtomicUsize,
    /// \[f\] `ft_off`.
    off: AtomicI64,
}

impl Futex {
    /// A futex of `key`, on no bucket yet, for bucket `fsq`.
    fn new(key: FutexKey, fsq: usize) -> Self {
        Self {
            entry: PinEntry::new(),
            fsq: AtomicUsize::new(fsq),
            ps: AtomicUsize::new(key.ps),
            obj: AtomicUsize::new(key.obj),
            amap: AtomicUsize::new(key.amap),
            off: AtomicI64::new(key.off),
        }
    }

    /// The key the futex waits on now (a requeue changes it).
    fn key(&self) -> FutexKey {
        FutexKey {
            ps: self.ps.load(Ordering::Relaxed),
            obj: self.obj.load(Ordering::Relaxed),
            amap: self.amap.load(Ordering::Relaxed),
            off: self.off.load(Ordering::Relaxed),
        }
    }

    /// The key and bucket a requeue gives it (both bucket locks held).
    fn set_key(&self, key: &FutexKey, fsq: usize) {
        self.fsq.store(fsq, Ordering::Relaxed);
        self.ps.store(key.ps, Ordering::Relaxed);
        self.obj.store(key.obj, Ordering::Relaxed);
        self.amap.store(key.amap, Ordering::Relaxed);
        self.off.store(key.off, Ordering::Relaxed);
    }

    /// `f->ft_proc != NULL`: whether the futex is still waiting (on a bucket). Read by its
    /// waiter without the lock, a snapshot: once it reads `false` no waker touches the futex
    /// again.
    fn waiting(&self) -> bool {
        self.entry.is_linked()
    }

    /// The channel the waiter sleeps on: the futex's own address (`sleep_setup(&f, ..)`).
    fn ident(&self) -> usize {
        ptr::from_ref(self).addr()
    }
}

hal::pin_adapter!(
    /// `TAILQ_HEAD(futex_list, futex)`: a bucket's waiters, in arrival order.
    FutexList: Futex, entry
);

/// `futex_is_eq`: whether two keys name the same userland address.
fn futex_is_eq(a: &FutexKey, b: &FutexKey) -> bool {
    a.off == b.off && a.ps == b.ps && a.obj == b.obj && a.amap == b.amap
}

/// `struct futex_slpque`: one bucket of waiting futexes.
#[repr(align(64))]
struct FutexSlpque {
    /// \[f\] `fsq_list`.
    fsq_list: PinList<FutexList>,
    /// `fsq_lock` ("futexlk").
    fsq_lock: RawRwlock,
    /// \[I\] `fsq_id`: for lock ordering, set once by [`futex_init`].
    fsq_id: AtomicU32,
}

impl FutexSlpque {
    /// An empty bucket, its id not drawn yet.
    const fn new() -> Self {
        Self {
            fsq_list: PinList::new(),
            fsq_lock: RawRwlock::new("futexlk"),
            fsq_id: AtomicU32::new(0),
        }
    }
}

/// `futex_slpques[]`.
static FUTEX_SLPQUES: [FutexSlpque; FUTEX_SLPQUES_SIZE] =
    [const { FutexSlpque::new() }; FUTEX_SLPQUES_SIZE];

/// `futex_init`: each bucket's lock-ordering id, random with the bucket's index in its low
/// bits (the lists and locks are constants).
pub fn futex_init() {
    for (i, fsq) in FUTEX_SLPQUES.iter().enumerate() {
        let id = (hal::random::arc4random() & !FUTEX_SLPQUES_MASK) | i as u32;
        fsq.fsq_id.store(id, Ordering::Relaxed);
    }
}

/// `futex(2)`: `op` on the word at `uaddr`; for `FUTEX_WAIT` `val` is the expected value and
/// `timeout` the address of a `struct timespec` (or `NULL`), for `FUTEX_WAKE` `val` the
/// number to wake, for `FUTEX_REQUEUE` `val` the number to wake, `timeout` (as an integer)
/// the number to move and `g` the word to move them to. The return value: the number woken
/// (0 for a wait).
pub fn sys_futex<G: ProcGlue>(
    p: &Thread<G>,
    uaddr: usize,
    op: i32,
    val: i32,
    timeout: usize,
    g: usize,
) -> Result<u32, Errno> {
    let flags = op & FUTEX_FLAG_MASK;
    let val = val as u32;

    match op & FUTEX_OP_MASK {
        FUTEX_WAIT => futex_wait(p, uaddr, val, UserPtr::new(timeout), flags).map(|()| 0),
        FUTEX_WAKE => futex_wake(p, uaddr, val, flags),
        // The C passes the pointer argument as `(u_long)timeout` to a `uint32_t`.
        FUTEX_REQUEUE => futex_requeue(p, uaddr, val, g, timeout as u32, flags),
        _ => Err(Errno::ENOSYS),
    }
}

/// `futex_addrs`: the key of the word at `uaddr` for thread `p`: its process for a private
/// futex, else [`futex_shared_key`] in its address space.
fn futex_addrs<G: ProcGlue>(p: &Thread<G>, uaddr: usize, flags: i32) -> FutexKey {
    if flags & FT_PRIVATE != 0 {
        return FutexKey {
            ps: ptr::from_ref(p.process()).addr(),
            obj: 0,
            amap: 0,
            off: uaddr as Voff,
        };
    }
    match p.vmspace_share() {
        Some(vm) => futex_shared_key(&vm.vm_map, uaddr),
        // The C follows `p->p_vmspace` unchecked; a thread in a system call has one.
        None => kpanic(format_args!("futex: a thread without an address space")),
    }
}

/// `futex_addrs`' shared half: under the map's read lock, the object and offset of the entry
/// holding `uaddr` when it is inherited shared (`MAP_INHERIT_SHARE`), or its amap and the
/// offset in it; else the address alone.
pub fn futex_shared_key(map: &VmMap, uaddr: usize) -> FutexKey {
    let mut key = FutexKey {
        off: uaddr as Voff,
        ..FutexKey::default()
    };
    let g = map.lock_read();
    if let Some(entry) = g.lookup(uaddr)
        && entry.inheritance() == MAP_INHERIT_SHARE
    {
        let h = entry.handles(g.witness());
        let into = (uaddr - entry.start()) as Voff;
        if uvm_et_isobj(entry) {
            if let Some(o) = h.uvm_obj() {
                key.obj = o.as_ptr().addr();
            }
            key.off = entry.offset().wrapping_add(into);
        } else if let Some(a) = &h.amap {
            key.amap = hal::pool::PoolArc::as_ptr(a).addr();
            key.off = (ptoa(h.pageoff as usize) as Voff).wrapping_add(into);
        }
    }
    key
}

/// `futex_get_slpque`: the index in `futex_slpques` of the bucket of `key`.
fn futex_get_slpque(key: &FutexKey) -> usize {
    let mut k = (key.off >> 3) as u32; // watevs
    k ^= k >> FUTEX_SLPQUES_BITS;
    (k & FUTEX_SLPQUES_MASK) as usize
}

/// `futex_unwait`: takes `f` off its bucket unless a waker already did, following it from
/// bucket `ofsq` where a requeue moved it; whether it was still waiting.
fn futex_unwait(ofsq: usize, f: &Futex) -> bool {
    let mut at = ofsq;
    loop {
        let fsq = &FUTEX_SLPQUES[at];
        let mut w = fsq.fsq_lock.write();
        let now = f.fsq.load(Ordering::Relaxed);
        if now == at {
            let rv = f.waiting();
            if rv {
                fsq.fsq_list.remove(&mut w, f);
            }
            return rv;
        }
        drop(w);
        at = now;
    }
}

/// Put the current thread on the sleep queue of the futex at address `uaddr`. Let it sleep
/// for the specified `timeout` time, or indefinitely if the argument is `NULL`.
fn futex_wait<G: ProcGlue>(
    p: &Thread<G>,
    uaddr: usize,
    val: u32,
    timeout: UserPtr<UserTimespec>,
    flags: i32,
) -> Result<(), Errno> {
    let mut nsecs = INFSLP;

    if !timeout.is_null() {
        let ts = timeout.copyin()?;
        // KTRACE: not configured.
        if ts.tv_sec < 0 || !ts.is_valid() {
            return Err(Errno::EINVAL);
        }
        nsecs = ts.to_nsec().min(MAXTSLP);
        if nsecs == 0 {
            return Err(Errno::ETIMEDOUT);
        }
    }

    let key = futex_addrs(p, uaddr, flags);
    let i = futex_get_slpque(&key);
    let fsq = &FUTEX_SLPQUES[i];

    // Mark futex as waiting, and make it visible to wake and requeue.
    let f: Pin<&mut Futex> = pin!(Futex::new(key, i));
    let f: Pin<&Futex> = f.into_ref();
    {
        let mut w = fsq.fsq_lock.write();
        fsq.fsq_list.insert_tail(&mut w, f);
    }

    // Do not return before f has been removed from the slpque! (Every path below ends with
    // the futex off its bucket: a waker took it, or `futex_unwait` does.)

    // Read user space futex value; if it changed, stop here.
    let error = match UserPtr::<u32>::new(uaddr).copyin32() {
        Err(e) => Err(e),
        Ok(cval) if cval != val => Err(Errno::EAGAIN),
        Ok(_) => {
            sleep_setup::<G>(f.ident(), PWAIT | PCATCH, "fsleep");
            let mut error = sleep_finish::<G>(nsecs, f.waiting());
            // Remove ourself if we haven't been awaken.
            if error.is_err() || f.waiting() {
                if !futex_unwait(i, &f) {
                    error = Ok(());
                }
                error = match error {
                    Err(Errno::ERESTART) => Err(Errno::ECANCELED),
                    Err(Errno::EWOULDBLOCK) => Err(Errno::ETIMEDOUT),
                    other => other,
                };
            }
            return error;
        }
    };

    // exit:
    if f.waiting() {
        futex_unwait(i, &f);
    }
    error
}

/// `futex_list_wakeup`: wakes every futex of `fl`, a local list of the bucket lock `w` holds.
/// Taking a futex off `fl` releases it (`ft_proc = NULL`): its waiter may return from then
/// on, so only its channel, read before, is used after. `SCHED_LOCK` is held for the batch,
/// as the C's, so a returned waiter cannot sleep again on that channel before its wakeup.
fn futex_list_wakeup<G: ProcGlue>(w: &mut WriteWitness<'_>, fl: &PinList<FutexList>) {
    let mut h = sched_lock();
    while let Some(ident) = fl.first(&*w).map(Futex::ident) {
        fl.remove_first(&mut *w);
        // wakeup_proc(p): the one thread asleep on the futex's address.
        wakeup_n_locked::<G>(&mut h, ident, 1);
    }
    drop(h);
}

/// The first loop of wake and requeue: at most `max` futexes of `key` taken off `fsq`, in
/// order, and woken (bucket lock held); how many.
fn futex_wake_matching<G: ProcGlue>(
    fsq: &FutexSlpque,
    w: &mut WriteWitness<'_>,
    key: &FutexKey,
    max: usize,
) -> usize {
    // An empty bucket: nothing to walk, and no local list to bind.
    if fsq.fsq_list.is_empty(&*w) {
        return 0;
    }
    let fl: PinList<FutexList> = PinList::new();
    let count = fsq
        .fsq_list
        .move_where(&mut *w, max, |f| futex_is_eq(&f.key(), key), &fl);
    if count != 0 {
        futex_list_wakeup::<G>(&mut *w, &fl);
    }
    count
}

/// Wakeup at most `n` sibling threads sleeping on a futex at address `uaddr` and requeue at
/// most `m` sibling threads on a futex at address `uaddr2`.
fn futex_requeue<G: ProcGlue>(
    p: &Thread<G>,
    uaddr: usize,
    n: u32,
    uaddr2: usize,
    m: u32,
    flags: i32,
) -> Result<u32, Errno> {
    if m == 0 {
        return futex_wake(p, uaddr, n, flags);
    }

    let okey = futex_addrs(p, uaddr, flags);
    let oi = futex_get_slpque(&okey);
    let nkey = futex_addrs(p, uaddr2, flags);
    let ni = futex_get_slpque(&nkey);
    let ofsq = &FUTEX_SLPQUES[oi];
    let nfsq = &FUTEX_SLPQUES[ni];

    // The C counts with `++count == n`: 0 never matches, so `n == 0` wakes every waiter of
    // `uaddr` and moves none.
    let max = if n == 0 { usize::MAX } else { n as usize };
    let matches = |f: &Futex| futex_is_eq(&f.key(), &okey);

    let count = if oi == ni {
        let mut w = ofsq.fsq_lock.write();
        let count = futex_wake_matching::<G>(ofsq, &mut w, &okey, max);
        if count == max {
            // update matching futexes: through a local list spliced back at the tail, so the
            // walk ends at the tail of the list as it is now.
            let moved: PinList<FutexList> = PinList::new();
            ofsq.fsq_list.move_where(
                &mut w,
                m as usize,
                |f| matches(f) && retag(f, &nkey, ni),
                &moved,
            );
            ofsq.fsq_list.concat(&mut w, &moved);
        }
        drop(w);
        count
    } else {
        let (mut wo, mut wn) = lock_two(ofsq, nfsq);
        let count = futex_wake_matching::<G>(ofsq, &mut wo, &okey, max);
        if count == max {
            // update matching futexes
            ofsq.fsq_list.move_where_across(
                &mut wo,
                m as usize,
                |f| matches(f) && retag(f, &nkey, ni),
                &nfsq.fsq_list,
                &mut wn,
            );
        }
        drop(wn);
        drop(wo);
        count
    };

    Ok(count as u32)
}

/// A requeued futex's new key and bucket (both locks held); `true`, so the move takes it.
fn retag(f: &Futex, key: &FutexKey, fsq: usize) -> bool {
    f.set_key(key, fsq);
    true
}

/// The write locks of two different buckets, taken in the order of their ids (then indices,
/// before [`futex_init`] drew the ids): `(ofsq's, nfsq's)`.
fn lock_two<'a>(
    ofsq: &'a FutexSlpque,
    nfsq: &'a FutexSlpque,
) -> (WriteWitness<'a>, WriteWitness<'a>) {
    let order = |q: &FutexSlpque| (q.fsq_id.load(Ordering::Relaxed), ptr::from_ref(q).addr());
    if order(ofsq) < order(nfsq) {
        let wo = ofsq.fsq_lock.write();
        let wn = nfsq.fsq_lock.write();
        (wo, wn)
    } else {
        let wn = nfsq.fsq_lock.write();
        let wo = ofsq.fsq_lock.write();
        (wo, wn)
    }
}

/// Wakeup at most `n` sibling threads sleeping on a futex at address `uaddr`.
fn futex_wake<G: ProcGlue>(p: &Thread<G>, uaddr: usize, n: u32, flags: i32) -> Result<u32, Errno> {
    if n == 0 {
        return Ok(0);
    }

    let key = futex_addrs(p, uaddr, flags);
    let fsq = &FUTEX_SLPQUES[futex_get_slpque(&key)];

    let mut w = fsq.fsq_lock.write();
    let count = futex_wake_matching::<G>(fsq, &mut w, &key, n as usize);
    drop(w);

    Ok(count as u32)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::boxed::Box;
    use std::sync::{MutexGuard, Once};
    use std::time::Duration;
    use std::vec::Vec;

    use hal::arch::current::param::PAGE_SIZE;
    use hal::arch::current::vmparam::{VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS};
    use hal::sys::futex::{FUTEX_REQUEUE_PRIVATE, FUTEX_WAIT_PRIVATE, FUTEX_WAKE_PRIVATE};
    use hal::sys::mman::{MADV_NORMAL, MAP_INHERIT_COPY, PROT_READ, PROT_WRITE};
    use hal::sys::types::Vsize;

    use super::*;
    use crate::proc::{P_INSCHED, Process, SSLEEP};
    use crate::sched::tests::{SchedGlue, T, TABLES, VCPUS, run_as, setup, thread};
    use crate::testutil;
    use crate::vm::amap::amap_init;
    use crate::vm::anon::uvm_anon_init;
    use crate::vm::aobj::{uao_create, uao_init};
    use crate::vm::fault::{VM_FAULT_INVALID, uvm_fault, uvmfault_init};
    use crate::vm::map::{
        uvm_map, uvm_map_inherit, uvm_map_init, uvm_mapanon, uvm_unmap, uvmspace_alloc,
        uvmspace_free,
    };
    use crate::vm::types::{UVM_FLAG_COPYONW, UVM_FLAG_FIXED, uvm_mapflag};

    type G = SchedGlue;

    /// This module's words of the user-copy double (each test module has its own range).
    const WORDS: usize = testutil::USER_BASE + 0xd000;

    /// The doubles, and the buckets' ids drawn once for the run.
    fn init() {
        testutil::install();
        static ONCE: Once = Once::new();
        ONCE.call_once(futex_init);
    }

    /// The word at `uaddr` set to `v`.
    fn put(uaddr: usize, v: u32) {
        UserPtr::<u32>::new(uaddr).copyout(&v).unwrap();
    }

    /// `n` words from `WORDS + 4 * from` whose private keys of `p` fall in distinct buckets
    /// (`distinct`) or all in the bucket of the first (`!distinct`).
    fn words(p: &T, from: usize, n: usize, distinct: bool) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let mut buckets: Vec<usize> = Vec::new();
        let mut i = from;
        while out.len() < n {
            let a = WORDS + 4 * i;
            let b = futex_get_slpque(&futex_addrs(p, a, FT_PRIVATE));
            let fits = match buckets.first() {
                None => true,
                Some(_) if distinct => !buckets.contains(&b),
                Some(&b0) => b == b0,
            };
            if fits {
                out.push(a);
                buckets.push(b);
            }
            i += 1;
            assert!(i < from + 4096, "no such words");
        }
        out
    }

    /// A waiter of `p` on the private word `uaddr` made by hand (never asleep), published on
    /// its bucket as `futex_wait` publishes one.
    fn publish<'f>(p: &T, uaddr: usize, f: Pin<&'f Futex>) -> Pin<&'f Futex> {
        let key = futex_addrs(p, uaddr, FT_PRIVATE);
        let i = futex_get_slpque(&key);
        f.set_key(&key, i);
        let fsq = &FUTEX_SLPQUES[i];
        let mut w = fsq.fsq_lock.write();
        fsq.fsq_list.insert_tail(&mut w, f);
        drop(w);
        f
    }

    /// A futex on no bucket yet.
    fn blank() -> Futex {
        Futex::new(FutexKey::default(), 0)
    }

    /// `futex(2)` by `p`.
    fn futex(
        p: &T,
        uaddr: usize,
        op: i32,
        val: i32,
        timeout: usize,
        g: usize,
    ) -> Result<u32, Errno> {
        sys_futex::<G>(p, uaddr, op, val, timeout, g)
    }

    /// The idents of the futexes of `key` on its bucket, in order.
    fn queued(key: &FutexKey) -> Vec<usize> {
        let q = &FUTEX_SLPQUES[futex_get_slpque(key)];
        let w = q.fsq_lock.write();
        q.fsq_list
            .iter(&w)
            .filter(|f| f.key() == *key)
            .map(Futex::ident)
            .collect()
    }

    #[test]
    fn the_bucket_of_a_key_is_the_c_s_hash() {
        // futex_get_slpque: key = off >> 3 (on the signed 64-bit offset, truncated to 32
        // bits); key ^= key >> 6; key & 63.
        for (off, want) in [
            (0i64, 0usize),
            (8, 1),
            (0x1f8, 63),
            (0x200, 1),
            (0x1000_d000, 40),
            (0x1000_d040, 32),
            (-8, 0),
        ] {
            let key = FutexKey {
                off,
                ..FutexKey::default()
            };
            assert_eq!(futex_get_slpque(&key), want, "offset {off:#x}");
        }
        init();
        for (i, q) in FUTEX_SLPQUES.iter().enumerate() {
            assert_eq!(
                q.fsq_id.load(Ordering::Relaxed) & FUTEX_SLPQUES_MASK,
                i as u32
            );
            assert_eq!(q.fsq_lock.name(), "futexlk");
        }
        assert_eq!(core::mem::align_of::<FutexSlpque>(), 64);
    }

    #[test]
    fn a_wait_s_errors_before_any_sleep() {
        let _g = setup();
        init();
        let p = thread(60, 50);
        let [w] = words(&p, 0, 1, true)[..] else {
            panic!("a word")
        };
        put(w, 7);
        let ts = UserPtr::<UserTimespec>::new(WORDS + 0x800);
        let wait = |val: i32, timeout: usize| futex(&p, w, FUTEX_WAIT_PRIVATE, val, timeout, 0);
        for (sec, nsec, want) in [
            (-1i64, 0i64, Errno::EINVAL),
            (0, 1_000_000_000, Errno::EINVAL),
            (0, -1, Errno::EINVAL),
            (0, 0, Errno::ETIMEDOUT),
        ] {
            ts.copyout(&UserTimespec {
                tv_sec: sec,
                tv_nsec: nsec,
            })
            .unwrap();
            assert_eq!(wait(7, ts.addr()), Err(want), "{sec}.{nsec}");
        }
        // An unreadable timeout; the word changed; a misaligned or unmapped word.
        assert_eq!(wait(7, 8), Err(Errno::EFAULT));
        assert_eq!(wait(8, 0), Err(Errno::EAGAIN));
        assert_eq!(
            futex(&p, w + 2, FUTEX_WAIT_PRIVATE, 7, 0, 0),
            Err(Errno::EFAULT)
        );
        assert_eq!(
            futex(&p, 16, FUTEX_WAIT_PRIVATE, 0, 0, 0),
            Err(Errno::EFAULT)
        );
        // None of them left a waiter; an unknown operation; a wake of none.
        assert_eq!(futex(&p, w, FUTEX_WAKE_PRIVATE, i32::MAX, 0, 0), Ok(0));
        assert_eq!(futex(&p, w, 0x7f | FT_PRIVATE, 0, 0, 0), Err(Errno::ENOSYS));
        assert_eq!(futex(&p, w, FUTEX_WAKE_PRIVATE, 0, 0, 0), Ok(0));
    }

    #[test]
    fn wake_takes_the_first_n_waiters_of_the_key_in_order() {
        let _g = setup();
        init();
        let p = thread(61, 50);
        // Two words of one bucket: their waiters share a list.
        let [a, b] = words(&p, 16, 2, false)[..] else {
            panic!("two words")
        };
        let f1 = pin!(blank());
        let f1 = publish(&p, a, f1.into_ref());
        let g1 = pin!(blank());
        let g1 = publish(&p, b, g1.into_ref());
        let f2 = pin!(blank());
        let f2 = publish(&p, a, f2.into_ref());
        let f3 = pin!(blank());
        let f3 = publish(&p, a, f3.into_ref());
        assert_eq!(futex(&p, a, FUTEX_WAKE_PRIVATE, 2, 0, 0), Ok(2));
        assert!(!f1.waiting() && !f2.waiting());
        assert!(f3.waiting() && g1.waiting());
        // Another process's word at the same address is another key.
        let other: &'static Process<G> = Box::leak(Box::new(Process::new(2, ())));
        let q: &'static T = Box::leak(Box::new(Thread::new_static(other, ())));
        assert_eq!(futex(q, a, FUTEX_WAKE_PRIVATE, 9, 0, 0), Ok(0));
        assert_eq!(futex(&p, a, FUTEX_WAKE_PRIVATE, 9, 0, 0), Ok(1));
        assert_eq!(futex(&p, b, FUTEX_WAKE_PRIVATE, 1, 0, 0), Ok(1));
        assert!(!f3.waiting() && !g1.waiting());
    }

    #[test]
    fn requeue_wakes_n_then_moves_m_across_buckets_and_within_one() {
        let _g = setup();
        init();
        let p = thread(62, 50);
        let [a, far] = words(&p, 64, 2, true)[..] else {
            panic!("two buckets")
        };
        let [_, near] = words(&p, 64, 2, false)[..] else {
            panic!("one bucket")
        };
        let key = |w: usize| futex_addrs(&p, w, FT_PRIVATE);

        // Across buckets: one woken, two of three moved, in order, with the new key.
        let f1 = pin!(blank());
        let f1 = publish(&p, a, f1.into_ref());
        let f2 = pin!(blank());
        let f2 = publish(&p, a, f2.into_ref());
        let f3 = pin!(blank());
        let f3 = publish(&p, a, f3.into_ref());
        let f4 = pin!(blank());
        let f4 = publish(&p, a, f4.into_ref());
        assert_eq!(futex(&p, a, FUTEX_REQUEUE_PRIVATE, 1, 2, far), Ok(1));
        assert!(!f1.waiting());
        for f in [&f2, &f3] {
            assert!(f.waiting());
            assert_eq!(f.key(), key(far));
            assert_eq!(f.fsq.load(Ordering::Relaxed), futex_get_slpque(&key(far)));
        }
        assert_eq!(queued(&key(far)), [f2.ident(), f3.ident()]);
        assert_eq!(queued(&key(a)), [f4.ident()]);
        // `n == 0` wakes every waiter of the word and moves none (the C's `++count == n`).
        assert_eq!(futex(&p, a, FUTEX_REQUEUE_PRIVATE, 0, 1, near), Ok(1));
        assert!(!f4.waiting());
        // Within one bucket: the moved waiter goes behind the word's own.
        let f5 = pin!(blank());
        let f5 = publish(&p, a, f5.into_ref());
        let f6 = pin!(blank());
        let f6 = publish(&p, a, f6.into_ref());
        let g1 = pin!(blank());
        let g1 = publish(&p, near, g1.into_ref());
        assert_eq!(futex(&p, a, FUTEX_REQUEUE_PRIVATE, 1, 1, near), Ok(1));
        assert!(!f5.waiting() && f6.waiting());
        assert_eq!(f6.key(), key(near));
        assert_eq!(queued(&key(near)), [g1.ident(), f6.ident()]);
        // `m == 0` is a wake; nothing is left.
        assert_eq!(futex(&p, near, FUTEX_REQUEUE_PRIVATE, 5, 0, a), Ok(2));
        assert_eq!(futex(&p, far, FUTEX_WAKE_PRIVATE, 5, 0, 0), Ok(2));
        assert_eq!(futex(&p, a, FUTEX_WAKE_PRIVATE, 5, 0, 0), Ok(0));
        for f in [&f2, &f3, &f6, &g1] {
            assert!(!f.waiting());
        }
    }

    /// Whether `t` is asleep on a futex: queued on its channel and out of `sleep_finish`'s
    /// window.
    fn asleep_on_a_futex(t: Pin<&'static T>) -> bool {
        t.stat() == SSLEEP
            && t.p_wmesg.get() == Some("fsleep")
            && t.p_flag.load(Ordering::Relaxed) & P_INSCHED == 0
    }

    /// Waits until `t` sleeps on a futex.
    fn until_asleep(t: Pin<&'static T>) {
        while !asleep_on_a_futex(t) {
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn wait_wake_requeue_on_a_private_key() {
        let _g = setup();
        init();
        let [a, b] = VCPUS;
        let me = thread(63, 50);
        run_as(a, me);
        let [w1, w2] = words(&me, 256, 2, true)[..] else {
            panic!("two buckets")
        };
        put(w1, 5);
        // A waiter ahead of this thread, made by hand (never asleep): the requeue wakes it
        // and moves this thread's.
        let first: &'static Futex = Box::leak(Box::new(blank()));
        let first = publish(&me, w1, hal::pin::pin_static(first));
        let waker = std::thread::spawn(move || {
            let w = thread(64, 50);
            run_as(b, w);
            until_asleep(me);
            assert_eq!(futex(&w, w1, FUTEX_REQUEUE_PRIVATE, 1, 1, w2), Ok(1));
            assert!(!first.waiting());
            // Moved, not woken: still asleep on its futex.
            assert!(asleep_on_a_futex(me));
            assert_eq!(futex(&w, w1, FUTEX_WAKE_PRIVATE, 1, 0, 0), Ok(0));
            assert_eq!(futex(&w, w2, FUTEX_WAKE_PRIVATE, 1, 0, 0), Ok(1));
            hal::thread::set_current_static::<T>(None);
        });
        assert_eq!(futex(&me, w1, FUTEX_WAIT_PRIVATE, 5, 0, 0), Ok(0));
        waker.join().unwrap();
        assert_eq!(me.p_wchan.get(), 0);
        hal::thread::set_current_static::<T>(None);
    }

    #[test]
    fn a_timed_wait_returns_etimedout() {
        let _g = setup();
        init();
        let [a, b] = VCPUS;
        let me = thread(65, 50);
        crate::sched::sleep_timeout_init(me.get_ref());
        run_as(a, me);
        {
            hal::klock::kernel_lock();
            let mut w = hal::klock::witness();
            TABLES.tidhash(65).insert_head(&mut w, me);
            drop(w);
            hal::klock::kernel_unlock();
        }
        let [w1] = words(&me, 512, 1, true)[..] else {
            panic!("a word")
        };
        put(w1, 1);
        let ts = UserPtr::<UserTimespec>::new(WORDS + 0x840);
        ts.copyout(&UserTimespec {
            tv_sec: 3600,
            tv_nsec: 0,
        })
        .unwrap();
        let timer = std::thread::spawn(move || {
            let w = thread(66, 50);
            run_as(b, w);
            until_asleep(me);
            // The softclock thread took the timeout off the wheel and runs its handler.
            assert!(hal::pin::project(me, |t| &t.p_sleep_to).del());
            hal::klock::kernel_lock();
            crate::sched::endtsleep::<G>(ptr::without_provenance_mut(65));
            hal::klock::kernel_unlock();
            hal::thread::set_current_static::<T>(None);
        });
        assert_eq!(
            futex(&me, w1, FUTEX_WAIT_PRIVATE, 1, ts.addr(), 0),
            Err(Errno::ETIMEDOUT)
        );
        timer.join().unwrap();
        // It took itself off its bucket.
        assert_eq!(futex(&me, w1, FUTEX_WAKE_PRIVATE, 1, 0, 0), Ok(0));
        hal::klock::kernel_lock();
        let mut w = hal::klock::witness();
        TABLES.tidhash(65).remove(&mut w, me.get_ref());
        drop(w);
        hal::klock::kernel_unlock();
        hal::thread::set_current_static::<T>(None);
    }

    /// Real page memory and the vm's pools (the map tests' set-up).
    fn vm_setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(1024 * 4096);
        testutil::init_shared_once();
        let _ = uvm_anon_init(0);
        let _ = amap_init(0);
        let _ = uao_init();
        hal::pmap::init();
        uvm_map_init();
        uvmfault_init();
        guard
    }

    #[test]
    fn a_shared_key_is_the_mapping_s_object_and_offset() {
        let _g = vm_setup();
        let vm = uvmspace_alloc(VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS, true, false);
        let map = &vm.vm_map;
        let rw = PROT_READ | PROT_WRITE;

        // An object mapped shared from its second page: the object and the offset in it.
        let uobj = uao_create(Vsize::new(4 * PAGE_SIZE), 0).expect("an aobj");
        let mut oa = 0x2000_0000;
        let share = uvm_mapflag(rw, rw, MAP_INHERIT_SHARE, MADV_NORMAL, UVM_FLAG_FIXED);
        uvm_map(
            map,
            &mut oa,
            2 * PAGE_SIZE,
            Some(uobj.clone()),
            PAGE_SIZE as Voff,
            0,
            share,
        )
        .expect("map the object");
        assert_eq!(
            futex_shared_key(map, oa + PAGE_SIZE + 12),
            FutexKey {
                ps: 0,
                obj: uobj.as_ptr().addr(),
                amap: 0,
                off: (2 * PAGE_SIZE + 12) as Voff,
            }
        );

        // Anonymous memory inherited shared: the address alone until it has an amap, then
        // the amap and the offset in it.
        let anon = uvm_mapflag(
            rw,
            rw,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
        );
        let mut sa = 0x3000_0000;
        uvm_mapanon(map, &mut sa, 2 * PAGE_SIZE, 0, anon).expect("map anon");
        uvm_map_inherit(map, sa, sa + 2 * PAGE_SIZE, MAP_INHERIT_SHARE).expect("inherit share");
        let at = sa + PAGE_SIZE + 4;
        let alone = |va: usize| FutexKey {
            off: va as Voff,
            ..FutexKey::default()
        };
        assert_eq!(futex_shared_key(map, at), alone(at));
        uvm_fault(map, sa, VM_FAULT_INVALID, PROT_WRITE, None).expect("a write fault");
        let (amap, pageoff) = {
            let g = map.lock_read();
            let e = g.lookup(sa).expect("the entry");
            let h = e.handles(g.witness());
            let amap = h
                .amap
                .as_ref()
                .map(|a| hal::pool::PoolArc::as_ptr(a).addr());
            (amap, h.pageoff)
        };
        let amap = amap.expect("an amap after the fault");
        assert_eq!(
            futex_shared_key(map, at),
            FutexKey {
                ps: 0,
                obj: 0,
                amap,
                off: (pageoff as usize * PAGE_SIZE + PAGE_SIZE + 4) as Voff,
            }
        );

        // Copied on fork, or not mapped: the address alone.
        let mut ca = 0x4000_0000;
        uvm_mapanon(map, &mut ca, PAGE_SIZE, 0, anon).expect("map anon");
        assert_eq!(futex_shared_key(map, ca + 8), alone(ca + 8));
        assert_eq!(futex_shared_key(map, 0x5000_0000), alone(0x5000_0000));

        uvm_unmap(map, oa, oa + 2 * PAGE_SIZE);
        uvm_unmap(map, sa, sa + 2 * PAGE_SIZE);
        uvm_unmap(map, ca, ca + PAGE_SIZE);
        drop(uobj);
        uvmspace_free(vm);
    }
}
/* </TESTS> */
