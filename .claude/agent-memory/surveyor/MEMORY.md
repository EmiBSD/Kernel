# surveyor: lessons across runs

One line per lesson, appended at the end; task state goes in the report
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.
- The host test double (`sys/arch/host`) refuses pageable `km_alloc`; host tests of a module with a kernel-memory buffer build it by hand with heap buffers, so count them as tied to the module's internals (section F), not as coverage of the allocation path.
- `docs/ZERO_UNSAFE.md` section 6 quoted `mod` line numbers, caller counts and a smoke list that had drifted by the next week; always re-measure what a planning document states and report the difference in section G.
- A smoke `--send` line with `||` is not a pipeline; grep for `|` not followed by `|` when counting the smokes that exercise pipes.
