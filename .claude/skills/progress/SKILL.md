---
name: progress
description: Redesign progress of EmiBSD per ROADMAP milestone (N0..), measured live by `cargo xtask unsafe-report --json` - the non-core modules made forbid, the legacy modules and legacy unsafe left, the core, the modules redesigned, and a time estimate from the legacy unsafe removed per active day since the zero-unsafe baseline. Use when the user asks how far the redesign is, how much is left, how long a milestone will take, or wants status numbers.
argument-hint: [milestone]
allowed-tools: Bash(python3 -I ${CLAUDE_SKILL_DIR}/scripts/progress.py*)
---

The table below was computed just now from `cargo xtask unsafe-report --json`,
`docs/ROADMAP.md`, `lineage.toml`, the LZ clone at the pin and git by
`${CLAUDE_SKILL_DIR}/scripts/progress.py` (its docstring defines every column).

```!
python3 -I ${CLAUDE_SKILL_DIR}/scripts/progress.py $ARGUMENTS
```

How to answer:

- Show the table as it is; never retype or round its numbers. Reply in the user's language,
  keeping the table's English headers (artifacts stay in English).
- Under it, three to six lines: the last milestone met and the one under way, what is left and
  where it concentrates (Legacy unsafe is the exit criterion's number; a row is complete when
  Legacy modules and Legacy unsafe are both 0), and the caveats that matter for the question
  asked: Forbid counts a row's non-core modules only, the core sits apart on the Core line;
  an `adapted` module only followed a redesign and is shown apart; a met row may keep legacy
  modules that a later row's prose claims (N1's crypto framework is N8's); the estimate
  divides a row's legacy unsafe by the legacy unsafe removed per active day since the
  baseline of `unsafe-budget.toml`, reads n/a until 5 active days have removed 100 or more (a smaller sample is noise), and
  promises nothing.
- A `Warning:` line under the table names non-core modules no ROADMAP row claims: say so;
  they need an owner row.
- With a milestone argument (`/progress N2`) the file list follows the table, legacy files
  first and the most unsafe first: say what is left in it and which files hold the most.
- If the output starts with `Note:`, unsafe-report could not run (the note says why) and the
  table is the old lineage one (modules and LZ lines, no forbid metric): say so.
- If the table is missing or starts with `progress: error`, run the script with Bash and
  report the error. Never estimate by hand.
- These numbers are for the conversation. The docs (README, STATUS, JOURNAL) take theirs from
  `cargo xtask lz status`, `cargo xtask unsafe-report` and the git commands `docs/JOURNAL.md`
  names.
