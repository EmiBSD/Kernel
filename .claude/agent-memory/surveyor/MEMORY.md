# surveyor: lessons across runs

One line per lesson, appended at the end; task state goes in the report
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.
- The host test double (`sys/arch/host`) refuses pageable `km_alloc`; host tests of a module with a kernel-memory buffer build it by hand with heap buffers, so count them as tied to the module's internals (section F), not as coverage of the allocation path.
- `docs/ZERO_UNSAFE.md` section 6 quoted `mod` line numbers, caller counts and a smoke list that had drifted by the next week; always re-measure what a planning document states and report the difference in section G.
- A smoke `--send` line with `||` is not a pipeline; grep for `|` not followed by `|` when counting the smokes that exercise pipes.
- `grep -lw <field>` counts names, not users: same-named fields of other structs (`KinfoFile.f_data`) and comments inflate it; count `\.<field>\.(get|set)\(` plus the typed accessors' callers, and report both numbers.
- A `diff-openbsd` step line need not name what it tests: `difftest syscalls ipc` (tools/difftest/difftest.c `t_ipc`) checks pipes, sockets and kqueue; read the probe source before saying a scenario is missing.
- The planning text a prompt quotes (docs/ZERO_UNSAFE.md sections, decisions) may exist only uncommitted in the working tree; check `git show <commit>:docs/...` and say which state each quoted number comes from.
- Whether a smoke exercises a kqueue filter depends on the userland program polling that object: grep `reference/openbsd-src/{lib,usr.bin,sbin,libexec}` for `poll(`/`kevent(`/libevent before claiming coverage (libfuse, libpcap and ksh do not poll).
- `unsafe-report` attributes by path prefix: its `arch/amd64` and `arch/arm64` rows include the boot loaders under `sys/arch/*/stand/` (302 `unsafe` at 9dbcbafa); split per file with `--json` before quoting a hardware total. In zsh loops never name a variable `path` (it is tied to PATH).
