# redesigner: lessons across runs

One line per lesson, appended at the end of the topic file it fits; task state goes in
HANDOFF.md (`.claude/rules/subagents.md`, "Memory"). Committed with the tree. This index
stays short (its readers cut it at 24 KB): new lessons go in the topic files.

- [tooling](tooling.md) — Worktree sandbox, git, shell, clippy, fmt, lz check, lineage.toml, bridges.toml, xtask, benches, builds (97 lessons)
- [hal-machine](hal-machine.md) — hal, unsafe and SAFETY, arm64/amd64 machine code, pmaps, boot, asm, the runtime primitives (44 lessons)
- [host-tests](host-tests.md) — Host tests and doubles: kernel::testutil, setup_real_memory, flakes, parallel tests (13 lessons)
- [design](design.md) — Rust and ownership idioms of the redesign: witnesses, guards, intrusive lists, cells, pools (39 lessons)
