# mechanical: lessons across runs

One line per lesson, appended at the end; task state goes in HANDOFF.md
(`.claude/rules/subagents.md`, "Memory"). Committed with the tree.

- A worktree agent's compound shell command mixing cd, git and long heredocs can be refused as "too complex": keep git calls plain and separate, and write long python patches to a scratchpad file first.
- xtask's lexer (unsafereport::lex_pos, scan, matching) is enough for token-level rule checks; Scan.test_mask marks cfg(test) tokens. BSD sed has no in-place edit without a suffix argument: edit with python.
