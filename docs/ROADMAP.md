# Roadmap

Native milestones, numbered `N0..` so they never collide with the port's `M` numbers (LZ's
`docs/ROADMAP.md`, M0..M17; M0..M14 are also this repository's history). Every milestone has a
mechanical exit criterion: `just ci` green (which includes `cargo xtask lz check`, `lz drift
--strict` and `unsafe-report --check`), `just ci-full` green, `just diff-openbsd` equal with the
same expected list, `cargo xtask lz drift --security` empty, and the subsystem's unsafe totals
against the baseline of N0. The numbers in the criteria are filled at N0 from the tools (real
data only, `docs.md`); until then they read `<n>`.

Order (recommended, the user's call at the go): leaves first, where the tests are strongest and
the risk lowest; the security subsystems last, with their own review rule. LZ sync is a rolling
gate for every milestone, never a milestone of its own.

| Milestone | Scope | Exit criterion |
|---|---|---|
| **N0 Bootstrap** | In `EmiBSD/`, in place, from `lz-origin`: the governance rewrite applied (`CLAUDE.md`, `.claude/rules/`, `docs/PHASE2.md`, this file, `docs/IDIOMS.md`, `docs/SYNC.md`, the README); `lz/PINNED.md`; `reference/emibsd-lz/` cloned; `lineage.toml` generated 1:1 from `ports.toml` (every LZ `.rs` maps to itself, `inherited`, licences and `wip` notes carried over; the `adapted` status and the `[[module.fn]]` rows in the schema from the first commit); `cargo xtask lz {check,status,drift,trace}`, `unsafe-report --check`, `unsafe-budget.toml` at the current counts, `just lz-sync`; `lz-sync.toml` with `[meta]` only; `ports.toml`, the `ports` subcommands and `docs/PORTING.md` retired; the RCS ident lines removed (decision 20); the baseline in `docs/STATUS.md`, `unsafe-budget.toml` and the N0 section of `docs/JOURNAL.md` (unsafe totals, smoke count, `diff-openbsd` result, the inherited blockers; the benchmarks when `just bench` exists); the memory directory curated | `cargo xtask lz check` passes; `cargo xtask lz drift` prints nothing; `cargo xtask unsafe-report --check` passes; `just ci` and `just ci-full` green; `just diff-openbsd` equal to LZ's last run; the baseline numbers recorded in `docs/JOURNAL.md` |
| **N1 Leaves** | `sys/lib/libkern`, `sys/lib/libz`, the primitives of `sys/crypto/` (ciphers, hashes, MACs; not the framework): slice and `&mut [u8]` APIs, no raw pointers, `unsafe` to zero or a justified list, the C's test vectors as table-driven tests, property tests for the parsers. It keeps 15 modules `inherited`: the crypto framework (`crypto`, `cryptodev`, `criov`; N8's), the constant tables (DES `podd`, `sk`, `spr`; libz `inffixed`, `zconf`), libkern's crate root and six helpers `forbid` as they stand | unsafe total of `lib/libkern`, `lib/libz` and `crypto` at 15 (from 10, 2 and 5 at `lz-origin`: 10, 0 and 5; the 10 of libkern are `StaticCell`, API frozen by the timing rule, and the `explicit_bzero` volatile store; the 5 of crypto are in the framework, N8), each remaining `unsafe` with a soundness argument; host test count (`just ci`) 2605 -> 2695; every crypto smoke passes (`smoke-wg`, `smoke-esp`, `smoke-ipcomp`, `smoke-softraid`, `smoke-https`) |
| **N2 Foundations** (met 2026-10-16; replanned the same day, docs/ZERO_UNSAFE.md sections 0 and 0.9) | The crates `sys/hal/` (the 24 `bsd`-free machine headers and 18 constant headers of `sys/sys/`, moved in `inherited` (`types.rs` follows uvm_param's page arithmetic at N3); the rest of the hardware enters `hal` with the kernel owner of each boundary, decisions 26-28; the new runtime: `Mutexed<T>`, `Rwlocked<T>`, `TypedPool<T>`, `PoolArc<T>`, `MBox<T>`, `KmBuf`, `OnceStatic<T>`, `List`/`Tree` written new with their first `kernel` user (the lists and trees of `sys/sys/queue.rs`, `sys/sys/tree.rs`, `sys/kern/subr_tree.rs` stay in `bsd`), `UserPtr<T>` with `abi_pod!`, the bridge registry), `sys/kernel/` (`#![forbid(unsafe_code)]`; crypto from `sys/crypto/`; pipe(2) from `sys/kern/sys_pipe.rs`, `sys/sys/pipe.rs`, the first subsystem by the new path) and `sys/abi/`, built as the loadable module `abi-openbsd.ko` (`#![forbid(unsafe_code)]`; the syscall table, all entries `Legacy` but pipe; the constants through `hal`; loaded by `hal`'s ELF loader at boot, registered as the syscall handler, given `KernelApi`, decision 22); `bsd` links `kernel` and `hal`; `cargo xtask hal-check`, `abi-check`, `unsafe-report` v2, `/progress` v2, `bridges.toml` | `kernel` and `abi` build with `forbid` at the root on both archs; `cargo xtask hal-check` and `abi-check` pass in `just check`; `hal` ≤ 2,150 `unsafe`, every site with `// SAFETY:`; `abi-openbsd.ko` loaded at boot on both archs (`smoke-abi`); `sys/kern/sys_pipe.rs` and `sys/sys/pipe.rs` absent from `bsd`; the 24 + 18 headers under `sys/hal/`; `just ci` and `just diff-openbsd` equal (`ci-full` at N3's close, decision 24). **Met 2026-10-16:** both forbid crates build on both archs (`just build`, `just check` rc=0); `hal-check` (74 files, 35 sites <= `hal_ceiling` 35, every block proved) and `abi-check` in `just check`; `hal` 35 <= 2,150; `abi-openbsd.ko: 331 syscalls, 2 native` on both archs (`smoke-abi`, `smoke-efiboot`; cd80.iso without it, decision 29); `sys/kern/sys_pipe.rs` and `sys/sys/pipe.rs` absent from `bsd`; 24 + 18 headers under `sys/hal/`; `just jobs=3 ci` rc=0 (85 of 85 smokes, 1734 s) and `just diff-openbsd` rc=0 (107 steps per arch, 104 equal, 3 expected, 0 unexpected); `ci-full` at N3 (decision 24) |
| **N3 Memory** | `sys/uvm/` -> `sys/kernel/src/vm/` in owner slices (km, object, anon, amap, aobj, the vnode pager, page, pmemrange, addr, map, mmap, glue; `uvm_fault` last, `security-review.md`); the pmap behind a safe `hal::Pmap` API, with the pmap cluster of `sys/arch` (14 files, `pmap_module.rs` included: 213 `unsafe` at the N2 close, 147 without the two `cpufunc.rs`) moved into `hal` behind it (decision 26) | `sys/uvm/` absent from `bsd`; the pmap files absent from `sys/arch/`; `cargo xtask unsafe-report` shows the `uvm` row of `bsd` gone; `hal` has no `pub unsafe fn` (`hal-check`) |
| **N4 Processes and scheduling** | `sys/kern/` by owner -> `sys/kernel/src/`: timeout, task, kqueue (`kern_event.rs`, `sys/sys/event.rs`), descriptors (`sys/sys/file.rs`, `kern_descrip.rs`), signals, proc, the scheduler, sysctl's core, tty; `StaticCell` uses in `kern` -> `OnceStatic`; the cpu, clock, interrupt, trap, syscall-entry and copy files of `sys/arch` (97) into `hal` (decision 26) | `sys/kern/` in `bsd` holds only `vfs_*` (N5), `uipc_*`/`sys_socket.rs` (N6) and the N8 files; the `Proc`, `Fd` and `Kq` bridges removed from `bridges.toml` |
| **N5 VFS and file systems** | `sys/kern/vfs_*`, `sys/ufs/`, `sys/isofs/`, `sys/msdosfs/`, `sys/nfs/`, `sys/ntfs/`, `sys/tmpfs/`, `sys/miscfs/` -> `sys/kernel/src/fs/` | those directories absent from `bsd`; the `Vfs` bridge removed |
| **N6 Network stack** | `sys/net/`, `sys/netinet/`, `sys/netinet6/`, `sys/kern/uipc_*`, `sys/kern/sys_socket.rs` -> `sys/kernel/src/net/`, except the files of N8 | those directories absent from `bsd` but N8's files; the `Sockets` bridge removed |
| **N7 Devices** | `sys/dev/` and `sys/scsi/` -> `sys/kernel/src/dev/`, one bus directory at a time, over `hal::Mmio<Regs>` and `hal::DmaBuf`; `sys/dev/rnd.rs` last (`security-review.md`); the bus, autoconf and ACPI/FDT glue of `sys/arch` (74 files) into `hal`, its 19 drivers into `sys/kernel/src/dev/` (decisions 26, 28) | `sys/dev/` and `sys/scsi/` absent from `bsd` but `sys/dev/softraid_crypto.rs` (N8) |
| **N8 Security subsystems** | Under `security-review.md`, each with `Security-Review:` and the area's smokes before every commit: `sys/net/pf*`, `sys/net/if_pf*`, `sys/net/if_wg.rs`, `sys/net/wg_*`, `sys/netinet/ip_ipsp.rs`, `sys/netinet/ip_esp.rs`, `sys/netinet/ip_ah.rs`, `sys/netinet/ip_ipcomp.rs`, `sys/netinet/ipsec_*`, `sys/dev/softraid_crypto.rs`, `sys/kern/kern_pledge.rs`, `sys/kern/kern_unveil.rs`, `sys/kern/exec_*`, the crypto framework -> `sys/kernel/src/` | those files absent from `bsd`; `cargo xtask lz drift --security` empty |
| **N9 Close** | What is left: `sys/ddb/` with the ddb glue and the configuration tables of `sys/arch/`, `sys/conf/`, `sys/reftest.rs`, `init/`, the old runtime files (`sys/kern/subr_pool.rs`, `sys/kern/kern_lock.rs`, `sys/kern/kern_rwlock.rs`, `sys/kern/kern_malloc.rs`, `sys/kern/kern_smr.rs`, `sys/kern/subr_percpu.rs`, their `sys/sys/` headers) deleted with their last user; `sys/sys/` emptied into `abi` and `kernel`; the `bsd` crate deleted; the kernel binary is `kernel` + `abi` + `hal`; Miri on `hal`'s host tests | `sys/` holds `hal/`, `kernel/`, `abi/`, `stand/`, `lib/` and nothing else; `cargo xtask unsafe-report`: no `bsd`, `kernel` 0, `abi` 0, `hal` at its recorded floor; `bridges.toml` empty; `just ci-full` and `just diff-openbsd` equal |

Every exit also requires:

- each file of the scope has left `bsd`: redesigned into `sys/kernel/` (or moved into `sys/abi/`
  or `sys/hal/` where the row says so) and `[[dropped]]` in `lineage.toml` with `absorbed by
  ...`, or a 0-`unsafe` call-site adapter listed as such (docs/ZERO_UNSAFE.md 0.3, 0.4);
- `just check`, one `just ci` and `just diff-openbsd` equal, with the same expected list;
  `just ci-full` at every close but N2's (decision 24: N3's close runs it for both);
- `cargo xtask lz drift --security` empty, and the numbers in the closing commit.

Since 2026-10-16 the goal is zero `unsafe` in the kernel: the crates `sys/kernel/` and `sys/abi/`
forbid it at their roots, `sys/hal/` holds the only `unsafe`, and `bsd` is emptied
(`docs/ZERO_UNSAFE.md` section 0); a row's scope resolves as `/progress` resolves it, the most specific path winning. A header of
`sys/sys/` belongs to the row whose C fills its fields (`proc.h` to N4, `vnode.h` to N5),
so it is redesigned with its owner into `kernel`; the constant headers nobody fills move into `hal`, `inherited`, and `kernel` and `abi` name them through `hal` (decision 27: 18 at N2, `types.rs` at N3 with uvm_param's inherent impls on its newtypes).

## Rolling: LZ sync

After every LZ milestone close and every LZ pin bump, and before any `N` is marked met:
`cargo xtask lz drift --security`, one `lz-sync.toml` record per LZ commit, `lz-sync:` commits,
then `lz: bump pin to <12-hex>` with the user's OK (`.claude/rules/lz-sync.md`). The ABI grows
only this way (M16's and M17's devices, syscalls and sysctls arrive as `applied` records).

## Adding or changing a milestone

State the subsystem it claims, the LZ modules it derives from (they turn `redesigned` in
`lineage.toml`), and one mechanical exit criterion with numbers from the tools. Update
`docs/STATUS.md` when a milestone starts and when it closes; `docs/JOURNAL.md` gains its section
at the close.
