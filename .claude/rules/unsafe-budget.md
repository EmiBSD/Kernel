# The unsafe budget

**v2 (2026-10-16, docs/ZERO_UNSAFE.md section 0, decisions 20-24) supersedes the core list
and the per-module `forbid` below:** the boundary is the crate. `sys/kernel/` and `sys/abi/`
carry `#![forbid(unsafe_code)]` at their roots (no budget: the build refuses one `unsafe`);
`sys/hal/` is the only crate with `unsafe`, under `hal_ceiling` in `unsafe-budget.toml`
(lowered by `--write`, raised only by the user, in a commit of its own, when a milestone adds a
listed piece: 2,150 at N2, 2,230 by N7); `sys/` (`bsd`, the LZ tree) is legacy under
`legacy_ceiling`, which only falls, until the crate is deleted at N9. `cargo xtask
unsafe-report` prints, beside its rows, a `crates` block: `kernel`, `abi`, `hal`, `hal_pending`
(the hardware and the lists and trees until they move into `hal`), `boot` and `bsd`, files and
`unsafe` each, and the headline ends with them. `--check` fails when `kernel` or `abi` is above 0,
`hal` above `hal_ceiling` (`[ratchet]`, lowered by `--write`, raised only by the user), or the
`bsd` files outside `unsafe-core.toml` above `legacy_ceiling`; the rows `kernel`, `abi` and `hal`
have no per-row budget. `hal-check` and `abi-check` (`xtask.md`) run in `just check`. The v1
groups below still print and gate until N9 (`legacy` is what `legacy_ceiling` ratchets and the
pace baseline reads); `unsafe-core.toml` is still read for them.

Safety is the first goal of the native system (`docs/PHASE2.md`): zero `unsafe` in the kernel
(`docs/ZERO_UNSAFE.md`). The budget makes that measurable and keeps it from regressing.

- v1 (2026-10-10 to 2026-10-16, superseded; kept so the v1 tool's output still reads): the core
  was the list of paths in `unsafe-core.toml`, and every module outside it was `forbid` (the
  attribute on its `mod` declaration) or legacy; the `forbid` count never fell and the legacy
  total never rose, except in the re-baseline commit of an LZ sync (below). In v2 the crate is
  the boundary (the header above); the modules `forbid` under v1 keep the attribute until they
  move into `sys/kernel/`.
### The v1 groups, kept until N9 (beside the crates of docs/ZERO_UNSAFE.md 0.4)

- `cargo xtask unsafe-report` prints, beside the per-subsystem rows, the three groups over the
  kernel counts (test code apart): core (the `unsafe` in the files of `unsafe-core.toml`, per
  row), `forbid` (the modules so declared, read from the module tree down from every `lib.rs`
  and `main.rs`, out of all the modules under `sys/` that `lineage.toml` tracks) and legacy
  (the `unsafe` outside the core). It also prints the shape counts (`StaticCell` uses,
  `unsafe impl Send`/`Sync`, `Cell<*const|*mut T>`, `UnsafeCell`) per subsystem, per module
  with `--shapes`; they gate nothing.
- `unsafe-budget.toml` holds the `[ratchet]` table (`forbid_floor`, `legacy_ceiling`) and, per
  `cargo xtask unsafe-report` row (`kern`, `uvm`, `net`, `dev/pci`, `arch/amd64`,
  `lib/libkern`, ...), the kernel total (blocks + fn + impl + trait + other) the subsystem may
  not exceed; the core's rows among them. Test code is not budgeted.
- `cargo xtask unsafe-report --check` runs in `just ci` and fails when any subsystem exceeds its
  budget, when the `forbid` modules fall below `forbid_floor`, when legacy rises above
  `legacy_ceiling`, or when a pattern of `unsafe-core.toml` names nothing.
  `cargo xtask unsafe-report --write` records the totals and the groups in `docs/STATUS.md`,
  lowers the budgets to the current counts, raises the floor and lowers the ceiling; it never
  moves any of them the other way.
- Raising a budget is a commit of its own (`build: raise the unsafe budget of <subsystem> to
  <n>`) with the reason in the body, after the user's OK. A redesign that needs more `unsafe`
  than LZ had is suspect by default.
- An LZ sync re-baselines the budget (the user's decision of 2026-10-10, after the M16 sync
  brought 122 inherited modules and put 14 rows over or without a budget): once its `lz-sync:`
  commits and the pin bump are in, one `build: re-baseline the unsafe budget after the LZ sync
  to <12-hex>` commit raises each row to the count the synced tree has, and `legacy_ceiling`
  with them (`docs/ZERO_UNSAFE.md`, section 8), with the before/after table and the LZ range in
  the body; `forbid_floor` never falls. It covers only what the inherited files and the
  cherry-picks into inherited files brought: a row that grew because of an adapted or
  redesigned module is raised only by the user, as above.
- Every commit whose change moves a subsystem's count carries the trailer
  `Unsafe: <subsystem> <before> -> <after>`, numbers from the tool, one line per subsystem.
- In a redesigned module, every `unsafe` that stays has a `// SAFETY:` that is a soundness
  argument: the invariant, who establishes it, who upholds it, and why the operation cannot
  violate it. "The C does this" is not an argument. Every `unsafe fn` has a `# Safety` section;
  `static mut` stays forbidden; the workspace lints stay denied and are never `#[allow]`ed.
- A safe abstraction that replaces `unsafe` across modules (a typed MMIO accessor, an owned
  DMA buffer, a lock that owns its data) is a row of `docs/IDIOMS.md` and, if it wants a crate,
  goes through the dependency allowlist (`rust-kernel.md`).
- `docs/STATUS.md` keeps the baseline table (the totals at `lz-origin`) next to the current
  ones; a milestone's exit criterion names the subsystems it lowered and by how much.
