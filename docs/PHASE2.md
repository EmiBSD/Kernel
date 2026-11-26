# The native process

EmiBSD is an operating system in Rust, derived from LandingZone (LZ, formerly EmiBSD.LZ), the
faithful file-by-file port of the OpenBSD kernel. LZ ports; this repository, Kernel, redesigns. This document is the process: where the code
comes from, what never changes, how a change is made, measured and reviewed, how LZ's later
work is absorbed, and the decisions that shaped all of it. The design itself is in
`docs/ARCHITECTURE.md`; the rules Claude follows are in `.claude/rules/`.

## Origin

EmiBSD started as the port itself. On 2026-10-09 the port's repository became
[LandingZone](https://github.com/EmiBSD/LandingZone) (then named EmiBSD.LZ), with its whole history (tag `lz-origin`,
commit `f5985f1d055a`, `docs: M16b met`), and this repository, which keeps that history up to that
commit, became the native system from there on (milestone N0). `lz/PINNED.md` records the LZ commit EmiBSD is synced to;
`lineage.toml` records what every module derives from; `docs/JOURNAL.md` keeps the port's
milestones M0..M14 as the first part of this system's history. LZ never refers to this
repository: the dependency points one way.

## What never changes

- The system-call ABI and the behaviour OpenBSD's userland sees. The ABI equals LZ's at the
  pin; additions arrive only through `lz-sync:` commits.
- The same userland runs on LZ and on EmiBSD, and `just diff-openbsd` is the oracle: EmiBSD
  against LZ against OpenBSD.
- Every smoke stays green (`just ci`, `just ci-full`).
- Licences: every file keeps the whole licence blocks of every LZ file it derives from (the RCS
  ident lines are not licence text and were dropped at N0), with the author's ISC block before
  them (the authorship rule of 2026-10-09); new code is ISC; code from outside OpenBSD's tree is the user's decision.
- The `machine` contract and the two architectures.

## Goals, in priority order

1. **Safety.** Zero `unsafe` in the kernel: `sys/kernel/` and `sys/abi/` are crates with
   `#![forbid(unsafe_code)]` at their roots; `sys/hal/` is the only crate with `unsafe`, every
   site argued and reviewed under a ceiling; the LZ tree (`bsd`) is replaced subsystem by
   subsystem and deleted at N9 (`docs/ZERO_UNSAFE.md` section 0, the one diagram;
   `.claude/rules/unsafe-budget.md`).
2. **Maintainability.** Idiomatic Rust where the LZ shape exists only to mirror C:
   `Cell`-everywhere structs, raw-pointer links, C-shaped out parameters (`docs/IDIOMS.md`).
3. **Performance.** Measured improvements only (`just bench`), never at the cost of 1 or 2.

These are the criteria of DARPA's TRACTOR program (functional correctness, safety guarantees,
competitive performance, long-term maintainability); LZ covers correctness, this system the rest.

## How a change is made

`.claude/rules/re-engineering.md`, in one line: pick one module of one subsystem, measure it,
read the LZ module and the C it ported, redesign with ownership first, prove it (soundness
arguments, tests, smokes, `diff-openbsd`), record it (`lineage.toml`, `//! LZ:`, `## Redesign`,
`docs/IDIOMS.md`), commit one step with the numbers (`.claude/rules/git-commits.md`).

Layout (decided 2026-10-07): the subsystem directories under `sys/` are fixed, as the unit for
metrics and lineage; inside a subsystem the module tree is free; a whole subsystem moves only
when it is redesigned, in a `lineage:` commit. A module is `inherited` (byte for byte LZ's, modulo
the RCS ident lines), `adapted` (only its call sites changed, because a module it uses was
redesigned) or `redesigned`; a redesign of an item used by more than 50 modules waits for the
LZ milestone that touches them to close (`.claude/rules/re-engineering.md`).

## Who redesigns: the agents

Since 2026-10-10 the redesign is done by Claude Code subagents with fixed roles, defined in
`.claude/agents/` and sharing one contract, `.claude/rules/subagents.md` (setup in a worktree,
the machine lock, the long-run watcher, when to stop, what to report). The set-up is LandingZone's
(its commit `b0901dd3`, where 196 hand-written launch prompts had carried the same rules),
adapted: the porter became the redesigner, `ports.toml` became `lineage.toml`, the trailers are
this repository's, and there is no bug-log role, since LZ is never told this repository exists.

| Agent | Model | Does |
|---|---|---|
| `milestone-coordinator` | opus | an `N` milestone: measure the baseline, split, launch, integrate, close CI and docs |
| `redesigner` | opus | a delicate redesign: locking, `unsafe`, data structures, the areas of `security-review.md`, any module over 3,000 lines |
| `mechanical` | sonnet | tests added to a module, `lineage.toml` bookkeeping, the same call-site change over many files, docs once decided |
| `integrator` | opus | merge agent branches, resolve the shared files (`lineage.toml`, the budget, the justfile), `just ci` |
| `debugger` | opus | the root cause of a failing or flaky smoke, test or `diff-openbsd` scenario, fixed without weakening an expectation |
| `reviewer` | opus | a read-only review of a branch: behaviour kept, `unsafe`, rules, security; it writes the `Security-Review:` text |
| `planner` | fable | a design burst from an inventory: the brief a redesigner executes and the decisions for the user; proposes, never decides |
| `surveyor` | opus | a read-only inventory of a module before its redesign is designed: `unsafe` by pattern, the primitives it needs, callers, the C's constraints, coverage; facts, not a design |
| `openbsd-probe` | sonnet | boot the real OpenBSD 8.0 on a QEMU setup and report what it does |
| `image-worker` | sonnet | view, crop, resize images, so the coordinator's context stays small |

Six of them keep a memory, `.claude/agent-memory/<agent>/MEMORY.md`: one line per lesson that
outlives a run (an idiom that took two attempts, a known flake, a conflict pattern), committed
with the tree so the whole team of agents learns. An agent in a worktree writes it on its branch;
the `integrator` keeps both sides on merge. Task state lives in a `HANDOFF.md`, never in memory.
The `reviewer` has no memory on purpose: every change gets fresh eyes.

The main session drives the redesign: it measures the baseline, splits the milestone into
clusters, launches the roles one by one with the Agent tool (at most four at once, two for a
coordinator), has every branch reviewed, integrates and runs `just ci`; the user sees every plan
and approves every commit. LZ's batch workflow (`/port`) was brought over as `/redesign` and
retired the same day, after its first run (the user's decision of 2026-10-10): it hid the plan
inside a script, could not be watched from VS Code, and had never been exercised. The machine
lock, `/tmp/emibsd/ci.lock`, is shared with LZ: one Mac, one `ci` at a time.

## How it is measured

- `cargo xtask unsafe-report`: per subsystem, blocks, `unsafe fn`, `unsafe impl`, `unsafe
  trait`, other, total; the baseline at `lz-origin` and the current totals in `docs/STATUS.md`;
  the budget in `unsafe-budget.toml`, checked in `just ci`.
- `cargo xtask lz status`: inherited and redesigned modules per subsystem.
- `just bench`: boot time per arch, `tcpbench` between two VMs, vioblk throughput; before and
  after in the commit (`Bench:` trailer).
- `just diff-openbsd`: steps compared, equal, expected; the expected list never grows.

Baseline at `lz-origin` (`f5985f1d055a`), from the tools: the `Unsafe` line and the lineage table of
`docs/STATUS.md`, `unsafe-budget.toml` (the per-subsystem totals, written by
`cargo xtask unsafe-report --write`), the smoke count and the `diff-openbsd` result of the
first `just ci` and `just diff-openbsd` in this tree (recorded in the N0 section of
`docs/JOURNAL.md`), and the benchmarks of `just bench` once it exists (boot to `login:` per
arch, `smoke-tcpbench`, a vioblk number; amd64 under TCG is indicative only until M17 brings
real hardware). The blockers LZ had at `lz-origin` are inherited and listed in `docs/STATUS.md`:
an N-milestone report is not a regression for them.

## Measuring progress

`/progress [milestone]` (`.claude/skills/progress/`) prints one row per ROADMAP milestone,
computed when asked from `lineage.toml`, `docs/ROADMAP.md`, the LZ clone at the pin and git:
modules redesigned, adapted and inherited, LZ lines done and left (the lines of each module's LZ
sources at the pin), Rust lines, and the time the rest would take at the project's own average
of LZ lines redesigned per active day. Its conventions:

- a module belongs to the `N` row whose scope cell names its path in backticks (a directory, a
  file or a glob), the most specific pattern winning; a row whose scope is prose resolves
  nothing; modules no row names are listed apart;
- `adapted` modules are shown apart from `redesigned` ones: their call sites followed a
  redesign, nothing of their own changed; extras count nowhere;
- the time column is a linear extrapolation of the past; it promises nothing.

The numbers in README, STATUS and JOURNAL do not come from it: they come from
`cargo xtask lz status`, `cargo xtask unsafe-report` and the git commands `docs/JOURNAL.md` names.

## How LZ's later work is absorbed

`.claude/rules/lz-sync.md` and `docs/SYNC.md`: `cargo xtask lz drift` lists the LZ commits
after the pin, the native modules they touch and (`--functions`) the items; every one gets a
record in `lz-sync.toml` (`applied` with the EmiBSD commit and its `method`: `cherry-pick`,
`cherry-pick-conflicts` or `reimplemented`; `not-applicable` with a reason;
`covered-by-redesign` with the proof); security fixes are never skipped silently; the pin
moves in a `lz: bump pin` commit. `just ci` fails on an untriaged LZ commit. The three method
totals in each sync commit say whether the roadmap's order is costing re-implementations.

## How security changes are reviewed

`.claude/rules/security-review.md`: the listed areas are redesigned last in their subsystem,
carry a `Security-Review:` trailer, and pass `just diff-openbsd` and their smokes before the
commit; crypto, IPsec, WireGuard and softraid CRYPTO change last of all, with extra tests.

## Decisions

| Date | Decision |
|---|---|
| 2026-10-07 | Three stages: OpenBSD -> EmiBSD.LZ (the port, continuous) -> EmiBSD (native). Governance files live inside each repository. |
| 2026-10-07 | The migration happens when M14 is complete, on the user's signal, never with agents running. |
| 2026-10-07 | `enavarre-cl/EmiBSD` stays the public home of the native system; `enavarre-cl/EmiBSD.LZ` is public and locked down (Issues only). |
| 2026-10-07 | LZ never names EmiBSD; EmiBSD declares LZ in `lz/PINNED.md`, `lineage.toml`, the `LZ:` trailer, this section and the README. |
| 2026-10-07 | Layout: subsystem directories fixed, free inside. |
| 2026-10-08 | Function-level traceability lives in `lineage.toml` (`[[module.fn]]`, exceptions only), not in the files; the RCS ident lines are dropped (decision 20). |
| 2026-10-09 | The port's repository became EmiBSD.LZ (public, locked down); this one is native from `lz-origin` = `f5985f1d055a`; `reference/openbsd-src` kept at LZ's pin; the milestone order N1..N8 recommended, the user's call at each start. |
| 2026-10-09 | Authorship, the user's decision: every `.rs` under `sys/` and `tools/` carries the author's ISC block ("Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>") in its `<LICENSES>` zone, first (the latest change), before the original blocks, which never change, or alone where there are none; `license = "none"` now describes the C source, not the Rust file; `lz check` requires the block and compares inherited modules without it. |
| 2026-10-09 | From the external review: `adapted` as a third module status; the `method` of every applied sync; a timing rule for widely used items; `docs/SYNC.md`; the blockers inherited at `lz-origin` recorded in the baseline. |
| 2026-10-10 | EmiBSD.LZ's subagent set-up (`b0901dd3`) adopted: roles in `.claude/agents/` (`redesigner` and `mechanical` for LZ's porters), one contract in `.claude/rules/subagents.md`, `.claude/agent-memory/` committed, the machine lock `/tmp/emibsd/ci.lock` shared with LZ, `/redesign` and `/progress` (the ROADMAP scope resolved against `lineage.toml`; no milestone key in its rows); no `external-bugs` role: slips go to the user. |
| 2026-10-10 | `/redesign` retired after its first run, stopped in its Plan phase: the session drives the redesign and launches the roles one by one (a `milestone-coordinator` when a whole milestone runs in the background); a workflow hid the plan, could not be watched from VS Code and was never tested. The roles, the contract, the memories and `/progress` stay. |
| 2026-10-10 | Zero `unsafe` outside a listed core (`docs/ZERO_UNSAFE.md`): every module outside `unsafe-core.toml` compiles under `#[forbid(unsafe_code)]`, set on its `mod` declaration; the core (hardware, boot, the host harness, a runtime of primitives, `explicit_bzero.rs`) is budgeted and reviewed; the legacy total only falls, an LZ sync aside; "redesigned" means "compiles with `forbid`"; N2 becomes "Core runtime", N9 "Close" is added; first slice pipe(2); `Adapter` a safe trait checked by `lz check`; `reimplemented` normal for `forbid` modules; Miri and the core crate decided at N9. Iterative: the plan follows what each step teaches. |
| 2026-10-16 | **v2 of the zero-unsafe plan** (`docs/ZERO_UNSAFE.md` section 0, decisions 20-24): the kernel is the crate `sys/kernel/` with `#![forbid(unsafe_code)]` at its root; `sys/abi/`, also `forbid`, is OpenBSD's userland interface as the loadable module `abi_openbsd.ko` with no logic (linked into the kernel image since decision 47, 2026-11-06); `sys/hal/` holds the only `unsafe` (hardware and runtime, ceiling 2,150 at N2); `bsd` (the LZ tree) is replaced subsystem by subsystem through bridges registered in `hal` and deleted at N9; develop first, compile later (no build per commit; `just check` and QEMU at the milestone close). Supersedes the row above: no core list, no per-module `forbid`. |
| 2026-10-18 | Decisions 42 and 43 (`docs/ZERO_UNSAFE.md` section 10): N3's performance regression measured before N4 (a release diagnostic, host microbenchmarks over the real `kernel::vm`, seven optimizations: host fork and exec -36 to -49 %; under QEMU amd64 `forkexit` -12 %, `forkexec` -10 %, arm64 unchanged, the rest in the pmaps and the trap), and the hot-path rule: `Bench:` lines before every hot-path commit, an op more than 10 % slower without a reason stops the agent. |
| 2026-10-18 | **Decision 44: arm64 first.** Only arm64 is built, booted and measured until N10 homologates amd64 (`EMIBSD_ARCHS`, landed by N4.0); no amd64 code is written or fixed before then; the planner records the amd64 constraints of every `hal` boundary type; N9 closes on arm64 with the amd64 tree frozen; N10 homologates amd64, N11 verifies (the TRACTOR gaps of 2026-10-17), N12 closes performance on both archs. The user's reason: the tokens spent advancing two architectures in parallel. |
| 2026-10-22 | EmiBSD.LZ's repository moved to the EmiBSD organisation as `EmiBSD/LandingZone` (public; GitHub redirects the old URL). `lz/PINNED.md` and every link follow; the name of the stage stays EmiBSD.LZ. |
| 2026-10-22 | The user's decision: the repositories are named for what they hold. EmiBSD.LZ is **LandingZone** (`EmiBSD/LandingZone`), the native system's repository is **Kernel** (`EmiBSD/Kernel`), and the website is `EmiBSD/Portal` (www.emibsd.org); the operating system keeps the name EmiBSD. `LZ` stays LandingZone's short name in the tooling (`lz/`, `cargo xtask lz`, the `LZ:` trailer and `//! LZ:` lines). Supersedes the row above's last clause; older rows keep the names of their day. The old landing page in `docs/` (emibsd.github.io/Kernel) is retired, GitHub Pages turned off for this repository, and LandingZone's Issues are turned on (they had been off since the transfer). |
| 2026-11-06 | **Decision 47: the ABI is linked into the kernel image** (`docs/ZERO_UNSAFE.md` section 10): `abi_openbsd.ko` no longer exists; `abi` stays a crate of its own, `#![forbid(unsafe_code)]`, no logic (`abi-check` R1-R5 unchanged), reaching the kernel only through `KernelApi`, which `bsd`'s boot glue hands `abi::abi_init` before `main()` (Cargo keeps `abi` away from `kernel` and `bsd`; `hal`'s public API acts on the kernel's own state, so `abi-check` R6, the user's answer to the slice's review, confines `abi` to `hal::api`, `hal::sys::*` and `hal::syscall`'s types); the syscall ABI is identical, the boot line reads `abi: 331 syscalls, 170 native`. The loader, its 3 `unsafe`, the module windows, the `ModulePlatform` bridge, hal's `module-rt` and the second file on every medium go; decision 22's module and decision 29 are superseded. |
| 2026-11-07 | **Decision 48: the ELF note name "EmiBSD"** (`docs/ZERO_UNSAFE.md` section 10): `exec_elf` accepts the ident note named "EmiBSD" beside "OpenBSD", with the same id, for Userland's renamed binaries; no numeric value changes and OpenBSD binaries exec as before. A deliberate, bounded exception to the unchanged-userland and lz-sync-only rules of CLAUDE.md; any further rename is a decision of its own. |
| 2026-11-11 | **Decision 49: the kernel is built with EmiBSD's own toolchain** (`docs/ZERO_UNSAFE.md` section 10): the justfile runs DevTools' Rust straight from `~/devel/emibsd/toolchains` (no rustup; `just cargo` for a bare cargo); arm64's target is `aarch64-emibsd-none-softfloat`; the linker and `llvm-objcopy` are DevTools' LLVM's. Never rustup or an official toolchain. amd64 follows at N10; the userland's compiler is a separate switch. |
| 2026-11-26 | **Decision 51: N7's scope** (the user, on the N7 plan's U2-U10): the drivers arm64's `ioconf.rs` does not configure move with amd64 to N10 (`adapted` at N7, redesigned with their smokes at N10); the LZ pin stays at e8249449 through N7 ("LandingZone esta con un trabajo muy incompleto": no fetch, LZ's TD commits on moved files are `reimplemented` later); `Devsw` is replaced at N7.3c and its legacy rows (pf's device) are paid at N8; one OpenBSD driver LZ did not port is added as the driver model's test, an exception to the rule that LZ's gaps come from LZ, with LandingZone told by a note left in its checkout; N7's hot paths (virtqueue enqueue and dequeue, interrupt dispatch, `device_lookup`, `bus_dmamap_load_mbuf`, `sd`'s strategy) join decision 43's list with `sys/kernel/benches/dev.rs`; wave D runs two wide, minding the weekly token limit; U2, option A: the arch glue's logic (the DMA segment loop, interrupt routing, the FDT and ACPI glue) is rewritten as safe code in `kernel`, and only the hardware access (registers, cache maintenance, ID registers, CPU start) lands in `hal` as small primitives with their proofs, `hal` about 300 at N7's end under the unchanged ceiling of 600 (option B, the glue files moved whole into `hal`, about +675 and a ceiling above 900, was declined). |
| 2026-11-26 | **Decision 52: `abi` calls the kernel directly** (the user, option A of three): the chain is userland -> `abi` -> `kernel` -> `hal`; `abi` depends on `kernel` and names only `kernel::api` (one function per system call, enforced by `abi-check`), `hal::api` and its `KernelApi` table go (they existed for the loadable module of decision 22, gone since decision 47), so a system call is a direct call instead of an indirect one; `hal` keeps only the trap's entry and the dispatcher `abi` registers, and the passive `hal::sys` and `hal::syscall` types. Rejected: B, a composition root where the image crate fills an `abi`-defined table (one indirect call per system call, about 980 lines moved, the same isolation `abi-check` already gives); D, a trait in `abi` that `kernel` implements (the kernel would name OpenBSD's ABI). `.claude/rules/abi.md` rewritten. |
| 2026-11-26 | **Decision 53: abi owns the system call convention; hal is only the cable** (the user, on N7.abi2's brief): the trap still enters `hal`'s vector (the hardware), but `hal` interprets nothing userland sends: it hands `abi` a `&UserRegs` view of the frame (three methods, no unsafe added) and `abi::arch::<arch>` reads the number and arguments, writes the return register and the carry, decides a restart and a forked child's first return, then calls `kernel::api::syscall_return`. Registers are a per-arch `Gpr` enum with names (the domain, not numbers). `hal::syscall::dispatch` goes; frozen amd64's call site waits for N10. One more frame borrow per system call is accepted until the N7 close measures it (fallback written in the brief: `hal` copies the registers `abi` declares at registration in its own entry borrow). `sendsig`/`setregs` (the signal frame, exec's registers) stay in `hal` for a later N7.abi3 with the user's go. R6 of `.claude/rules/abi.md` widened accordingly. Why: a second ABI (an EmiBSD-native userland after N9) becomes one more safe crate, with `hal` untouched. |
| 2026-11-26 | **Decision 54: `sysdefs`, the basic types in one leaf crate under every layer** (the user, "me parece perfecto"): OpenBSD's basic types, constants and `#[repr(C)]` ABI structures, which lived in `hal::sys` (72 modules), are the crate `sysdefs` (`sys/lib/sysdefs/`, beside `libkern` and `libz`), `#![forbid(unsafe_code)]`, no dependencies, no state, no logic; `hal`, `kernel`, `abi` and `bsd` depend on it (no re-export: every importer names `sysdefs::`). **sysdefs = data shapes, hal = byte reinterpretation**: `AbiPod` and `abi_pod!` (a safe marker and its compile-time layout checks) moved in, but every function that views bytes as a value or a value as bytes is `unsafe` underneath and stays in `hal` (`hal::user::{bytes_of, pod_from_bytes, zeroed, AbiBytes}`, `hal::mem::pod`, the extension traits of `hal::sysbytes`), with the tests that need them. Why: one definition of each shape instead of each layer inventing its own (`Register` and `Timespec` were each defined twice, and are now one), `abi` no longer touches `hal` for data (R6 shrinks to `hal::syscall`'s types and `set_handler`), and `hal` holds only mechanism. `libsa` (the boot loaders' library) keeps its own copies of `Dev`, `Off`, `Mode`, `Time`, `Uid` and `Gid`: they are not all identical to the kernel's and it is tested against `bsd`'s headers, so it is not switched; the page arithmetic of `Paddr`/`Vaddr`/`Vsize` (const shifts and masks over `sysdefs::param`, asserted equal to the machine's by `hal`) moved with the types so no call site changed |
| 2026-11-26 | **Decision 55: the kernel's `sys/lib/` crates hold no `unsafe`** (the user, "sip"): `libkern`, `libz` and `sysdefs` are `#![forbid(unsafe_code)]` at their roots; `libsa`, the boot loaders' library, stays the declared exception, since the loaders are separate binaries that run before the kernel. `explicit_bzero` (one volatile store) moved into `hal::mem` as a safe function with a spelled-out `SAFETY` argument and a compiler fence; `StaticCell` (the `static` with interior mutability, 9 `unsafe`) split in two copies: `sys/sys/staticcell.rs` for `bsd` (about 52 files, dying with its last user at N9) and `sys/lib/libsa/staticcell.rs` for the loaders (`libsa`, `sys/stand/boot` and the efiboot crates, which do not depend on `bsd`). Why: the boundary of `unsafe` is the crate, and a leaf library that the kernel layers share should not be the one place it hides; the second copy costs one small file in a crate that is excepted anyway, and the legacy `unsafe` count does not rise (the copy keeps its `unsafe-core.toml` classification) |

## Later

Graphics (drm, nouveau, Mesa, GSP firmware) is a proposal for LZ after M17; it would reach this
system through `lz-sync` like any other LZ work.
