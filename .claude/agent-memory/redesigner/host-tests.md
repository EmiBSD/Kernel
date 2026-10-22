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
