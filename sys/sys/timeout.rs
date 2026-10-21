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
 * Copyright (c) 2000-2001 Artur Grabowski <art@openbsd.org>
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

/*
 * Copyright (c) 2001 Thomas Nordin <nordin@openbsd.org>
 * Copyright (c) 2000-2001 Artur Grabowski <art@openbsd.org>
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
//! `<sys/timeout.h>`: `timeout(9)`, functions run once after a number of clock ticks or at an
//! absolute uptime. The wheel, its soft interrupt and its threads are the kernel crate's
//! (`kernel::timeout`, N4.1); this is the legacy API the callers in `bsd` keep, over it.
//!
//! Upstream: sys/sys/timeout.h @ 3ce1f3f79392
//! Upstream: sys/kern/kern_timeout.c @ 3ce1f3f79392
//! LZ: sys/sys/timeout.rs@3c62ede68802
//! LZ: sys/kern/kern_timeout.rs@3c62ede68802
//!
//! `crate::kern::kern_timeout` names this module, so the callers' paths are LZ's.
//!
//! ## Deviations
//! - Each legacy [`Timeout`] is served by a node of the pool `timeoutpl` (a
//!   `kernel::timeout::Timeout`, which the wheel links), found by the wrapper's address in a
//!   registry and never given back: LZ frees the structures that embed timeouts without
//!   dropping them (`pool_put` of a `struct proc`, a `tcpcb`, a SYN cache entry; a softc's
//!   `free`), so no wrapper can be trusted to say when its node is free. A wrapper made again
//!   at an address the registry knows (the same pool item, reused) takes that address's node
//!   back, deleted first, so the nodes are bounded by the number of distinct addresses that
//!   ever held a timeout that was set or added: in practice the peak of live timeouts, since
//!   `pool(9)` and `malloc(9)` reuse their items' addresses (a pool page given back and carved
//!   again for another pool can bring new ones). The C embeds the timeout and allocates
//!   nothing. A node that cannot be had (`PR_NOWAIT`) is `panic(9)`, where the C cannot fail;
//!   both go when the callers own their timeouts (`kernel::timeout::Timeout`, pinned in place),
//!   subsystem by subsystem.
//! - `timeout_set` writes a timeout's fields under the wheel's lock (the C writes them bare),
//!   the kernel's change (`kernel::timeout`).
//! - `to_process` (the `kcov` identifier) is not kept: `kcov` is not configured.
//!
//! ## Redesign
//! - LZ's `struct timeout` was the C's `#[repr(C)]` record of `Cell`s, linked by its `to_list`
//!   `struct circq` (raw pointers, `unsafe impl Sync`) into the wheel's queues, and its fields
//!   were read and written by `kern_timeout.rs` and by three callers. Now the wheel links a
//!   pinned `kernel::timeout::Timeout`, and [`Timeout`] is what a legacy structure embeds: the
//!   values of `TIMEOUT_INITIALIZER` (written only by the `const` constructors) and a link to
//!   its node (`hal::link::Link`). It stays valid all zero (`Timeout::zeroed`, a zeroed softc's
//!   timeouts: the `unsafe impl Softc` proofs cite it), `Sync` without `unsafe`, and its
//!   `const fn`s keep the statics compiling. The three direct field users read
//!   [`Timeout::to_time`] or call [`Timeout::set_none`].
//! - The node is found by the wrapper's address: the link when it names a node of this
//!   address (one load and one compare per call, the hot path), else the registry (a
//!   `hal::tree::Tree` of the nodes by address under its own mutex), else a new node. A node
//!   made for the first `timeout_add` of a `TIMEOUT_INITIALIZER` takes the wrapper's values.
//! - A call on a wrapper with no node is answered as the C answers an idle timeout without
//!   making one: `timeout_del` counts its deletion and `timeout_barrier` finds nothing
//!   running, through an idle node of this module's.
//! - `kern_timeout.rs`'s functions (`timeout_startup`, `timeout_proc_init`,
//!   `timeout_hardclock_update`, `softclock`, `timeout_adjust_ticks`, `timeout_maskwheel`) are
//!   the kernel's under their legacy names; `timeout_sysctl` copies the kernel's `stats()`
//!   (twelve `uint64_t`, the C's order) as `kern.timeout_stats`.

use core::ffi::c_void;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

pub use kernel::timeout::TimeoutFn;

use hal::intr::IPL_HIGH;
use hal::link::Link;
use hal::mutex::Mutexed;
use hal::pin::pin_static;
use hal::pool::{PR_NOWAIT, PR_WAITOK, PoolArc, TypedPool};
use hal::rwlock::Rwlocked;
use hal::tree::{Tree, TreeEntry};

use crate::conf::param::{HZ, TICK, TICK_NSEC};
use crate::kassert;
use crate::kern::kern_sysctl::sysctl_rdstruct;
use crate::kern::subr_prf::panic;
use crate::sys::errno::Errno;
use crate::sys::time::Timespec;

/*
 * flags in the to_flags field.
 */

/// `TIMEOUT_PROC`: needs a process context.
pub const TIMEOUT_PROC: i32 = 0x01;
/// `TIMEOUT_ONQUEUE`: on any timeout queue.
pub const TIMEOUT_ONQUEUE: i32 = 0x02;
/// `TIMEOUT_INITIALIZED`: initialized.
pub const TIMEOUT_INITIALIZED: i32 = 0x04;
/// `TIMEOUT_TRIGGERED`: running or ran.
pub const TIMEOUT_TRIGGERED: i32 = 0x08;
/// `TIMEOUT_MPSAFE`: run without kernel lock.
pub const TIMEOUT_MPSAFE: i32 = 0x10;

/// `KCLOCK_NONE`: dummy clock for sanity checks.
pub const KCLOCK_NONE: i32 = -1;
/// `KCLOCK_UPTIME`: uptime clock; time since boot.
pub const KCLOCK_UPTIME: i32 = 0;
/// `KCLOCK_MAX`.
pub const KCLOCK_MAX: i32 = 1;

/// `struct timeout`, as a legacy structure embeds it: valid all zero (a `static struct
/// timeout`, a zeroed softc's), served by a node of `timeoutpl` (the module's deviations).
pub struct Timeout {
    /// `TIMEOUT_INITIALIZER`'s `to_func`, for the node its first add makes (written only by
    /// the `const` constructors).
    init_func: Option<TimeoutFn>,
    /// `TIMEOUT_INITIALIZER`'s `to_arg` (never written after construction).
    init_arg: AtomicPtr<c_void>,
    /// `TIMEOUT_INITIALIZER`'s `to_kclock`.
    init_kclock: i32,
    /// `TIMEOUT_INITIALIZER`'s `to_flags`, with `TIMEOUT_INITIALIZED` (0: never initialised).
    init_flags: i32,
    /// The node found or made for this wrapper (empty until then; it serves this wrapper only
    /// while the node's address is this wrapper's).
    node: Link<Node>,
}

impl Timeout {
    /// `TIMEOUT_INITIALIZER_FLAGS(_fn, _arg, _kclock, _flags)`.
    pub const fn new_flags(func: TimeoutFn, arg: *mut c_void, kclock: i32, flags: i32) -> Self {
        Self {
            init_func: Some(func),
            init_arg: AtomicPtr::new(arg),
            init_kclock: kclock,
            init_flags: flags | TIMEOUT_INITIALIZED,
            node: Link::new(),
        }
    }

    /// `TIMEOUT_INITIALIZER(_f, _a)`.
    pub const fn new(func: TimeoutFn, arg: *mut c_void) -> Self {
        Self::new_flags(func, arg, KCLOCK_NONE, 0)
    }

    /// A timeout that `timeout_set` has not seen yet (all zero, like a `static struct
    /// timeout`).
    pub const fn zeroed() -> Self {
        Self {
            init_func: None,
            init_arg: AtomicPtr::new(ptr::null_mut()),
            init_kclock: 0,
            init_flags: 0,
            node: Link::new(),
        }
    }

    /// `to->to_time`: the value of `ticks` it is due at (0 before it was ever added).
    pub fn to_time(&self) -> i32 {
        self.served().map_or(0, |n| n.to.time())
    }

    /// `timeout_set(to, NULL, NULL)` (`usbd_alloc_xfer`'s): initialised, with no function.
    pub fn set_none(&self) {
        self.node_or_make()
            .to
            .set(None, ptr::null_mut(), KCLOCK_NONE, 0);
    }

    /// This wrapper's address, the registry's key.
    fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }

    /// The node that serves this wrapper, when it has one: its link names a node of its
    /// address.
    #[inline]
    fn served(&self) -> Option<&'static Node> {
        self.node.get().filter(|n| n.owner == self.addr())
    }

    /// The wrapper's node, found in the registry or made (`timeout_set`'s path): a node
    /// another wrapper left at this address is deleted first.
    fn node_or_make(&self) -> &'static Node {
        if let Some(n) = self.served() {
            return n;
        }
        let n = node_at(self.addr());
        n.to.del();
        self.node.set(Some(n));
        n
    }

    /// The wrapper's node for an add: when it has none, the one at its address set to the
    /// wrapper's `TIMEOUT_INITIALIZER` values (the C's `KASSERT` that it was initialised
    /// first).
    fn node_for_add(&self) -> &'static Node {
        if let Some(n) = self.served() {
            return n;
        }
        kassert!(self.init_flags & TIMEOUT_INITIALIZED != 0);
        let n = self.node_or_make();
        n.to.set(
            self.init_func,
            self.init_arg.load(Ordering::Relaxed),
            self.init_kclock,
            flags_of(self.init_flags),
        );
        n
    }

    /// The node, pinned: a pool item never given back (the module's deviations).
    #[inline]
    fn pinned_for_add(&self) -> Pin<&'static kernel::timeout::Timeout> {
        pin_static(&self.node_for_add().to)
    }
}

/// A node of `timeoutpl`: the kernel's timeout the wheel links, for the wrapper at `owner`.
struct Node {
    /// The address of the wrapper this node serves (the registry's key, fixed).
    owner: usize,
    /// The registry's link.
    entry: TreeEntry<Node>,
    /// What the wheel links.
    to: kernel::timeout::Timeout,
}

hal::tree_adapter!(
    /// The registry's order: by the wrapper's address.
    ByOwner: Node, entry, |a, b| a.owner.cmp(&b.owner)
);

/// `timeoutpl`: the legacy timeouts' nodes (the module's deviations).
static TIMEOUT_POOL: TypedPool<Node> = TypedPool::new();
/// Whether [`TIMEOUT_POOL`] is made.
static POOL_MADE: AtomicBool = AtomicBool::new(false);
/// Serialises the making of [`TIMEOUT_POOL`] (a sleeping lock: `pool_init` may sleep), once.
static POOL_INIT: Rwlocked<()> = Rwlocked::new("timeoutpl", ());
/// The nodes by wrapper address, under a mutex at the wheel's level (timeouts are added from
/// interrupt handlers).
static REGISTRY: Mutexed<Tree<ByOwner>> = Mutexed::new(IPL_HIGH, Tree::new());
/// The idle node answering for a wrapper with none: never added, so never queued, never
/// running.
static IDLE: kernel::timeout::Timeout = kernel::timeout::Timeout::unset();

/// The kernel's flags for the legacy `int` ones.
const fn flags_of(flags: i32) -> u32 {
    (flags & (TIMEOUT_PROC | TIMEOUT_MPSAFE)) as u32
}

/// The legacy uptime as the kernel's.
const fn kernel_ts(ts: &Timespec) -> kernel::clock::Timespec {
    kernel::clock::Timespec {
        tv_sec: ts.tv_sec,
        tv_nsec: ts.tv_nsec,
    }
}

/// [`TIMEOUT_POOL`], made at the first need (the first `timeout_set`, in `main` before any
/// other CPU runs; the lock keeps a racer from making a second pool).
fn timeout_pool() -> &'static TypedPool<Node> {
    if !POOL_MADE.load(Ordering::Acquire) {
        let _w = POOL_INIT.write();
        if !POOL_MADE.load(Ordering::Relaxed) {
            if TIMEOUT_POOL.init(IPL_HIGH, PR_WAITOK, "timeoutpl").is_err() {
                panic(format_args!("timeout_set: cannot make timeoutpl"));
            }
            POOL_MADE.store(true, Ordering::Release);
        }
    }
    &TIMEOUT_POOL
}

/// The host tests' hal bridges (the kernel's doubles), installed once per process: the
/// registry's and the wheel's locks and the nodes' pool need them, and a bsd test that sets a
/// timeout may run before any test that installs them (a filtered run, or the order of a
/// whole one). Nothing outside the host tests.
#[inline]
fn test_bridges() {
    #[cfg(test)]
    kernel::testutil::install();
}

/// The node at `owner`'s address: the registry's, or a new one inserted (never given back).
fn node_at(owner: usize) -> &'static Node {
    test_bridges();
    let find = |t: &Tree<ByOwner>| t.find_by(|n| owner.cmp(&n.owner));
    if let Some(n) = find(&REGISTRY.lock()) {
        return n;
    }
    // Made with the registry's lock released (the pool's own lock is not nested in it).
    let Some(new) = timeout_pool().get_shared(PR_NOWAIT, || Node {
        owner,
        entry: TreeEntry::new(),
        to: kernel::timeout::Timeout::unset(),
    }) else {
        panic(format_args!("timeout_set: no node for a legacy timeout"));
    };
    let reg = REGISTRY.lock();
    if let Some(n) = find(&reg) {
        // Another CPU made this address's node meanwhile: ours goes back to the pool.
        drop(reg);
        drop(new);
        return n;
    }
    let n = PoolArc::leak(new);
    reg.insert(n);
    n
}

/*
 * special macros
 */

/// `timeout_pending(to)`: is this timeout already scheduled to run?
pub fn timeout_pending(to: &Timeout) -> bool {
    to.served().is_some_and(|n| n.to.pending())
}

/// `timeout_initialized(to)`: is this timeout initialized?
pub fn timeout_initialized(to: &Timeout) -> bool {
    match to.served() {
        Some(n) => n.to.initialized(),
        None => to.init_flags & TIMEOUT_INITIALIZED != 0,
    }
}

/// `timeout_triggered(to)`: is this timeout running, or did it run?
pub fn timeout_triggered(to: &Timeout) -> bool {
    to.served().is_some_and(|n| n.to.triggered())
}

/// `timeout_startup()`: the wheel's `hz`, `tick` and `tick_nsec` (`param.c`'s). In the host
/// tests it also installs the kernel's doubles of hal's bridges (the wheel's lock and its
/// nodes' pool need them; a test that runs alone has had no other test install them) and
/// empties the wheel (`timeout_test_reset`), as LZ's re-initialised every queue: the tests
/// that call it rely on a wheel with nothing left by an earlier test.
pub fn timeout_startup() {
    #[cfg(test)]
    {
        test_bridges();
        kernel::timeout::timeout_test_reset();
    }
    kernel::timeout::startup(
        HZ.load(Ordering::Relaxed),
        TICK.load(Ordering::Relaxed),
        TICK_NSEC.load(Ordering::Relaxed),
    );
}

/// `timeout_proc_init()`: the `softclock` soft interrupt and threads.
pub fn timeout_proc_init() {
    kernel::timeout::proc_init();
}

/// `timeout_set(new, fn, arg)`.
pub fn timeout_set(new: &Timeout, func: TimeoutFn, arg: *mut c_void) {
    timeout_set_flags(new, func, arg, KCLOCK_NONE, 0);
}

/// `timeout_set_flags(to, fn, arg, kclock, flags)`.
pub fn timeout_set_flags(to: &Timeout, func: TimeoutFn, arg: *mut c_void, kclock: i32, flags: i32) {
    kassert!(flags & !(TIMEOUT_PROC | TIMEOUT_MPSAFE) == 0);
    to.node_or_make()
        .to
        .set(Some(func), arg, kclock, flags_of(flags));
}

/// `timeout_set_proc(new, fn, arg)`.
pub fn timeout_set_proc(new: &Timeout, func: TimeoutFn, arg: *mut c_void) {
    timeout_set_flags(new, func, arg, KCLOCK_NONE, TIMEOUT_PROC);
}

/// `timeout_add(new, to_ticks)`: `true` when it was not already scheduled.
pub fn timeout_add(new: &Timeout, to_ticks: i32) -> bool {
    new.pinned_for_add().add(to_ticks)
}

/// `timeout_add_sec(to, secs)`.
pub fn timeout_add_sec(to: &Timeout, secs: i32) -> bool {
    to.pinned_for_add().add_sec(secs)
}

/// `timeout_add_msec(to, msecs)`.
pub fn timeout_add_msec(to: &Timeout, msecs: u64) -> bool {
    to.pinned_for_add().add_msec(msecs)
}

/// `timeout_add_usec(to, usecs)`.
pub fn timeout_add_usec(to: &Timeout, usecs: u64) -> bool {
    to.pinned_for_add().add_usec(usecs)
}

/// `timeout_add_nsec(to, nsecs)`.
pub fn timeout_add_nsec(to: &Timeout, nsecs: u64) -> bool {
    to.pinned_for_add().add_nsec(nsecs)
}

/// `timeout_abs_ts(to, abstime)`: due at the uptime `abstime` (`KCLOCK_UPTIME`).
pub fn timeout_abs_ts(to: &Timeout, abstime: &Timespec) -> bool {
    to.pinned_for_add().abs_ts(&kernel_ts(abstime))
}

/// `timeout_del(to)`: `true` when it was pending.
pub fn timeout_del(to: &Timeout) -> bool {
    test_bridges();
    to.served().map_or(&IDLE, |n| &n.to).del()
}

/// `timeout_del_barrier(to)`: deleted, and not running when it returns.
pub fn timeout_del_barrier(to: &Timeout) -> bool {
    test_bridges();
    to.served().map_or(&IDLE, |n| &n.to).del_barrier()
}

/// `timeout_barrier(to)`: waits until `to` is not running.
pub fn timeout_barrier(to: &Timeout) {
    test_bridges();
    to.served().map_or(&IDLE, |n| &n.to).barrier();
}

/// `timeout_maskwheel(level, abstime)`: the bucket of `abstime` on a clock wheel's level.
pub fn timeout_maskwheel(level: u32, abstime: &Timespec) -> u32 {
    kernel::timeout::timeout_maskwheel(level, &kernel_ts(abstime))
}

/// `timeout_hardclock_update()`: called by `hardclock` on the primary CPU every tick.
pub fn timeout_hardclock_update() {
    kernel::timeout::hardclock_update();
}

/// `softclock(arg)`: the soft interrupt's work (the kernel establishes the interrupt; the
/// legacy tests call it by hand).
pub fn softclock(_arg: *mut c_void) {
    kernel::timeout::softclock();
}

/// `timeout_adjust_ticks(adj)` (`!SMALL_KERNEL`): `tc_setclock` stepped `ticks` forward.
pub fn timeout_adjust_ticks(adj: i32) {
    kernel::timeout::adjust_ticks(adj);
}

/// `timeout_sysctl` (`!SMALL_KERNEL`): `kern.timeout_stats`, `struct timeoutstat` (twelve
/// `uint64_t` in the C's order) copied under the wheel's lock.
pub fn timeout_sysctl(
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let _ = newlen;
    let mut status = [[0u8; 8]; 12];
    for (b, v) in status.iter_mut().zip(kernel::timeout::stats()) {
        *b = v.to_ne_bytes();
    }
    sysctl_rdstruct(oldp, oldlenp, newp, status.as_flattened())
}

/// The nodes the registry holds (tests).
#[cfg(test)]
fn registry_len() -> usize {
    REGISTRY.lock().iter().count()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // The wheel is the kernel's and global: the bsd tests that drive it (here, kern_event,
    // kern_tc, the drivers' and pf's) serialise on `LOCK` and leave nothing pending.

    use core::sync::atomic::AtomicU32;
    use std::sync::{Mutex as StdMutex, MutexGuard};

    use super::*;

    /// The wheel's lock for the bsd tests (LZ's `kern_timeout::tests::LOCK`).
    pub(crate) static LOCK: StdMutex<()> = StdMutex::new(());

    static FIRED: AtomicU32 = AtomicU32::new(0);

    fn fire(_arg: *mut c_void) {
        FIRED.fetch_add(1, Ordering::Relaxed);
    }

    fn setup() -> MutexGuard<'static, ()> {
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        kernel::testutil::install();
        timeout_startup();
        FIRED.store(0, Ordering::Relaxed);
        g
    }

    #[test]
    fn a_zeroed_timeout_has_no_node_until_set() {
        let _g = setup();
        static TO: Timeout = Timeout::zeroed();
        assert!(!timeout_initialized(&TO));
        assert!(!timeout_pending(&TO));
        assert!(!timeout_del(&TO), "an idle timeout is not pending");
        assert!(TO.node.get().is_none(), "del makes no node");
        timeout_set(&TO, fire, ptr::null_mut());
        assert!(timeout_initialized(&TO));
        let n = TO.served().map(ptr::from_ref);
        assert!(n.is_some());
        timeout_set(&TO, fire, ptr::null_mut());
        assert_eq!(
            TO.served().map(ptr::from_ref),
            n,
            "set again keeps the node"
        );
    }

    #[test]
    fn the_legacy_api_runs_a_timeout_through_the_kernel_wheel() {
        let _g = setup();
        static TO: Timeout = Timeout::zeroed();
        timeout_set(&TO, fire, ptr::null_mut());
        assert!(timeout_add(&TO, 2));
        assert!(timeout_pending(&TO));
        assert!(!timeout_add(&TO, 2), "already scheduled");
        kernel::timeout::tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 0);
        kernel::timeout::tick_once();
        assert_eq!(FIRED.load(Ordering::Relaxed), 1);
        assert!(timeout_triggered(&TO));
        assert!(!timeout_pending(&TO));
        assert!(!timeout_del(&TO));
        assert!(!timeout_triggered(&TO));
    }

    #[test]
    fn startup_in_the_tests_empties_the_wheel() {
        let _g = setup();
        static TO: Timeout = Timeout::zeroed();
        timeout_set(&TO, fire, ptr::null_mut());
        assert!(timeout_add(&TO, 5));
        let pending = kernel::timeout::stats()[4];
        // An earlier test's timeout, left pending: the next setup takes it off unrun.
        timeout_startup();
        assert!(!timeout_pending(&TO));
        assert!(kernel::timeout::wheel_is_empty());
        assert_eq!(kernel::timeout::stats()[4], pending - 1);
        for _ in 0..10 {
            kernel::timeout::tick_once();
        }
        assert_eq!(FIRED.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn an_initializer_makes_its_node_at_the_first_add() {
        let _g = setup();
        static TO: Timeout = Timeout::new(fire, ptr::null_mut());
        assert!(timeout_initialized(&TO));
        assert!(timeout_add(&TO, 1));
        assert!(TO.served().is_some());
        let now = kernel::clock::ticks();
        assert_eq!(TO.to_time().wrapping_sub(now), 1);
        assert!(timeout_del(&TO));
        assert!(!timeout_pending(&TO));
    }

    #[test]
    fn an_address_keeps_one_node() {
        let _g = setup();
        let before = registry_len();
        let a = node_at(0x10);
        assert!(ptr::eq(a, node_at(0x10)), "found, not made again");
        assert_eq!(registry_len(), before + 1);
        assert!(!ptr::eq(a, node_at(0x20)));
        assert_eq!(registry_len(), before + 2);
    }

    #[test]
    fn a_wrapper_made_again_at_an_address_takes_its_node_back_deleted() {
        let _g = setup();
        static TO: Timeout = Timeout::zeroed();
        timeout_set(&TO, fire, ptr::null_mut());
        assert!(timeout_add(&TO, 50));
        let n = TO.served().map(ptr::from_ref);
        // The structure is freed without timeout_del and its memory made again, zeroed: the
        // new wrapper has no link.
        TO.node.set(None);
        assert!(!timeout_pending(&TO), "a fresh wrapper is idle");
        timeout_set(&TO, fire, ptr::null_mut());
        assert_eq!(
            TO.served().map(ptr::from_ref),
            n,
            "the address's node, taken back"
        );
        assert!(!timeout_pending(&TO), "deleted first");
        assert_eq!(FIRED.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_wrapper_linked_to_another_address_gets_its_own_node() {
        let _g = setup();
        static A: Timeout = Timeout::zeroed();
        static B: Timeout = Timeout::zeroed();
        timeout_set(&A, fire, ptr::null_mut());
        // B's bytes copied from A (a structure moved after its timeout was set).
        B.node.set(A.node.get());
        assert!(B.served().is_none());
        timeout_set(&B, fire, ptr::null_mut());
        assert!(B.served().is_some());
        assert_ne!(B.served().map(ptr::from_ref), A.served().map(ptr::from_ref));
    }

    #[test]
    fn set_none_initialises_without_a_function() {
        let _g = setup();
        static TO: Timeout = Timeout::zeroed();
        TO.set_none();
        assert!(timeout_initialized(&TO));
        assert!(!timeout_pending(&TO));
    }

    #[test]
    fn the_sysctl_copies_twelve_counters() {
        let _g = setup();
        let mut len = 0usize;
        assert!(timeout_sysctl(0, &mut len, 0, 0).is_ok());
        assert_eq!(len, 12 * 8);
    }
}
/* </TESTS> */
