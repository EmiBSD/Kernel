# Git and commits

- Commit only when the user asks. Never force-push, never amend a published commit.
- Subject: `<scope>: <imperative summary>`, at most 72 characters, lowercase scope, no trailing
  period. Scopes: the subsystem (`kern`, `uvm`, `sys`, `libkern`, `net`, `netinet`, `dev/<dir>`,
  `arch/amd64`, `arch/arm64`, `machine`, `ddb`, `crypto`, `ufs`, ...), plus `lz-sync`,
  `lineage`, `lz` (pin bumps), `xtask`, `docs`, `rules`, `build`, `reference`.
  Examples: `kern: own the run queues inside the scheduler lock`,
  `lz-sync: apply LZ 0123456789ab (kern: port kern_foo.c)`,
  `lineage: move sys/kern/sched into its own directory`, `lz: bump pin to 0123456789ab`.
- Body: what LZ's shape was, what the native shape is and why, and the numbers: the `unsafe`
  totals before and after (from `cargo xtask unsafe-report`), a benchmark when measured, the
  tests added. Mention every inherited gap the change closes or keeps.
- Trailers, in this order:
  - `LZ: sys/kern/kern_sched.rs@0123456789ab`, one line per LZ file the change derives from,
    12-hex LZ commit, no spaces around `@`. In an `lz-sync:` commit the commit-only form
    `LZ: 0123456789ab`; the record in `lz-sync.toml` lists the files.
  - `Unsafe: kern 1234 -> 1201`, one line per subsystem whose count changed, the names and
    totals of `unsafe-report` (`unsafe-budget.md`).
  - `Security-Review: ...`, required for the areas of `security-review.md`.
  - `Bench: boot-amd64 2.31 -> 2.05 s`, optional, name, before, after, unit.
  - `Co-Authored-By: ...` lines as required by the session.
  `Upstream:` is not a native trailer: OpenBSD provenance is reached through the `//! Upstream:`
  lines inside the files and through LZ's own trailers.
- One re-engineering step per commit. `lineage.toml`, `lz-sync.toml`, `unsafe-budget.toml` and
  docs changes ride with the commit that made them necessary. An `lz-sync:` commit does nothing
  but the sync; a `lineage:` commit nothing but the move.
- A file move is its own `lineage:` commit made with `git mv`, so `git log --follow` and
  `git blame -C -C` keep the history; the `[[module.fn]]` rows ride with the refactor that made
  them necessary, never with an `lz-sync:` commit.
- Toolchain bumps (`rust-toolchain.toml`), LZ pin bumps and budget raises are their own commits.
- Never commit `reference/openbsd-src/`, `reference/emibsd-lz/`, `target/`, `*.img` or editor
  files. `.gitignore` covers them; check `git status` before committing anyway.
- Default branch is `main`. If the user wants branches: `n<N>/<topic>` or `sync/<lz-12-hex>`.

## Tags (the user's decision of 2026-10-22)

- Every `N` milestone is tagged when it closes: the `docs:` commit that declares it met in
  `docs/ROADMAP.md`, `docs/STATUS.md` and `docs/JOURNAL.md` receives an annotated tag
  `n<N>-<name>`, lowercase, the name being the ROADMAP row's first word (`n0-bootstrap`,
  `n1-leaves`, `n2-foundations`, `n3-memory`, `n4-processes`, `n5-vfs`, ...). The main
  session creates it at that close, on `main`, with
  `git tag -a n<N>-<name> <commit> -F <message>`, and pushes it with `git push origin <tag>`
  beside the close's push, with the user's OK for each close, as for the push. Subagents
  never tag or push (the guard hook refuses both).
- The tag message is the close's summary: the LZ pin, the totals of `cargo xtask
  unsafe-report` per crate (`kernel`, `abi`, `hal`, `bsd`), the result of `just ci` and
  `just ci-full` (rc and wall time) and of `just diff-openbsd`, and the modules the milestone
  redesigned.
- A tag never moves. If a close has to be corrected, the milestone is reopened and closed
  again with another commit and another tag, `n<N>-<name>-2`. `lz-origin` is LZ's tag and
  is never touched.
- No semantic versions: the kernel stays `EmiBSD 8.0` to userland; the milestone's name is
  the version. GitHub releases (`gh release`) are not made for now: tags only, to be decided
  at the N4 close, when an arm64 image is worth attaching.
- N0 to N3 were tagged retroactively on 2026-10-22 on the `docs:` commits that declared them
  met, with the numbers those commits and `docs/JOURNAL.md` record.
