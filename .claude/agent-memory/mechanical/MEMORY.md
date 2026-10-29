# mechanical: lessons across runs

One line per lesson, appended at the end; task state goes in HANDOFF.md
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.

- A worktree agent's compound shell command mixing cd, git and long heredocs can be refused as "too complex": keep git calls plain and separate, and write long python patches to a scratchpad file first.
- xtask's lexer (unsafereport::lex_pos, scan, matching) is enough for token-level rule checks; Scan.test_mask marks cfg(test) tokens. BSD sed has no in-place edit without a suffix argument: edit with python.
- Worktree isolation refuses compound shell (pipes into bash, heredoc with git, cd then git): put steps in a scratchpad .sh, run `bash script`, one simple command per call; commit with `git commit -F file`.
- rustfmt resolves cfg-gated `mod` files: a new `pub mod arm64;` under a cfg needs its file in the same commit or `cargo fmt` (the hook) fails.
- Before moving an inherited header into hal, grep it for `crate::reftest` (test-only, pub(crate) in bsd): such files cannot move byte-identical.
- A sweep that renames an LZ item (method) of an adapted module forces [[module.fn]] rows: keep the old method and let the new name delegate, or lz check fails; a path-prefix rewrite must skip `::tests::` imports and the frozen amd64 tree.
