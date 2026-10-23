---
name: redesigner-host-tests
description: Host tests and doubles: kernel::testutil, setup_real_memory, flakes, parallel tests
metadata:
  type: feedback
---

- Dropping `Copy` breaks `Cell::get` and `StaticCell::read` at call sites: use `take`/`clone`/`set` on the Cell, `get().clone()` on the StaticCell. A stable "not Copy" check: `crate::crypto::testutil::assert_not_copy!` (E0283).
- MP host tests share CPU 0's per-CPU page magazines across setup_real_memory (uvm_page_test_reset resets them; a new per-CPU cache of kernel memory needs the same), and frees land there: count `uvmexp.free + percpucaches`; kernel tests over the real page system hold `testutil::MEMORY`.
- `amap_lookup` panics on an entry without an amap: unwire/wire paths test `aref.amap().is_some()` first (shared file and aobj mappings have none).
- `uvm_swapisfull()` is true with no swap (0 >= 0): out-of-RAM faults return ENOMEM, not ERESTART; a retry test raises `UVMEXP.swpages`. `uvmfault_anonget` counts rusage on every retry (the C's `goto retry`).
- Test helpers calling `init_shared()` (any `TypedPool::init`) from parallel tests race (EBUSY, then no pool, ~1 in 20): wrap the init in a `std::sync::Once`.
- `try_read` by the holder of the write witness panics "enter read deadlock" (the C's check): test "held elsewhere" from another thread (`thread::scope`).
- Shared kernel test maps go below the host's VM_MIN_KERNEL_ADDRESS: the map tests store UVM_MAXKADDR downwards, which desyncs a best-fit map above it ("uaddr_bestfit_remove: entry was not in tree").
- Baseline a filtered test run at another commit: stash the paths with a unique message, check out the base, run, come back, apply the stash by its SHA, then drop the `stash@{n}` whose SHA it is.
- An aborting test module hides its later failures: after fixing an abort, rerun the module at the fixing commit and bisect what fails behind it (wg's data-message tests were masked from 7311cd9d to 7227d7f8).
- A host test that drops an element linked on two pinned lists aborts the whole test binary (the second PinEntry panics during the unwind): link it on one list only in a should_panic test.
- bsd's MpLock depth is per `cpu_number()`, 0 for every host test thread: kern_lock.rs's release checks read a test-only per-thread depth that a test arms (`kernel_lock_checks_arm`), none elsewhere; never call `__mp_release_all` unarmed in a test (it zeroes CPU 0's depth every thread shares); host tests that reach `[K]` readers take `kernel_lock()` as the syscall would.
- A kernel test that writes the user-copy double's memory (64 KiB, shared by parallel tests) takes a range of its own; kernel::pipe's tests use USER_BASE + 64.
- A witness-taking form called from legacy code (VN_KNOTE's knote_locked) panics in the file systems' host tests, which run VOPs without the kernel lock: the legacy form takes the lock recursively itself.
- bsd tests that pass host pointers as user addresses (LZ's machine copyin double read host memory) fail with EFAULT once the path goes through `hal::user::UserPtr` (kernel::testutil's 64 KiB double): put the buffers at `USER_BASE + <own range>` with `copyout_slice`/`copyin_slice`; the double's copyinstr/copyoutstr copy strings since N4.6a.
- Kernel tests that need real sleeps reuse kernel::sched's harness (`pub(crate) mod tests`: SchedGlue, setup, thread, run_as, VCPUS, TABLES): one host thread sleeps on vCPU a, a spawned one acts on vCPU b after polling `stat()==SSLEEP && !P_INSCHED`; never write a second harness (one `register_start` per process).
- `pin!(..)` inside a call argument is not lifetime-extended (E0716): `let f = pin!(x); let f = use_it(f.into_ref());`.
- kernel::testutil's copyin32 double (a std Mutex over the user memory) is ~9 ns dev vs bsd host's 0.5 ns: bench it as a prim before blaming a futex-like design.
- kernel::testutil's 64 KiB user-memory double is shared: uio/select use 0x8000..0xC000, kernel::log 0xE000..0x10000 (kern.msgbuf's copy runs to the end and faults); pick an unused range and grep `USER_BASE + ` first.
- `klist_invalidate` waits for a knote's KN_PROCESSING to clear; kernel::event's `test_knote` stays processing, so a test of an object whose teardown invalidates its klists (ttyfree) must not attach test knotes or it hangs the run.
- kernel::tty's defaults leave VSTATUS disabled (CSTATUS 0o377, ttydefaults.h): a ^T test sets c_cc[VSTATUS] first; and `tputchar` emits "\r\r\n" for '\n' (the C's), which assertions must expect.
- Signals in a kernel test need the thread in `ps_threads`, `siginit` and `test_sigacts` (kernel::tty's `world()`), or pgsignal finds no one to deliver to.
- A kernel test whose path reaches `klock::reader()`/`witness()` (wait4, acct's session) panics "the kernel lock is not held": take `hal::klock::kernel_lock()` once per test thread in the setup (the double counts it), as a system call would.
- `kernel::test_proc_glue!(Name, Files, { fn hook(..) {..} })` overrides glue members (N4.6a); a second glue in one test module needs its own `mod` (the link types are module items), and a test process put in a pgrp/session is leaked with `mem::forget` (dropping it linked panics).
- `test_proc_glue!`'s tables are per test thread (a thread_local `ProcTables`): a list walk (`sysctl_doproc`) sees only the processes its own thread made, so an error that needs a process looked at (an unknown op's EINVAL) needs one put on the list first.
- hal's host user-copy double keeps its own addresses below 4 GiB (EFAULT outside the user and foreign runs) and copies raw above, so bsd's UIO_SYSSPACE host tests reach the kcopy token with real host pointers.
- A hal function the kernel calls with its test glues' thread types must not call `hal::thread::check_type` (one type per process): `slot_of` alone checks the layout (the TCB accessors).
