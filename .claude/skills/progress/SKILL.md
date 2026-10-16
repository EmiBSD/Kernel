---
name: progress
description: Redesign progress of EmiBSD per ROADMAP milestone (N0..), measured live from `cargo xtask unsafe-report --json` by crate (docs/ZERO_UNSAFE.md section 0) - the files and unsafe still in bsd per milestone (the exit criterion), what moved into hal, kernel or abi, the bridges open, the crates' totals (kernel 0, abi 0, hal under its ceiling, bsd), and a time estimate from the unsafe removed per active day since the baseline. Use when the user asks how far the redesign is, how much is left, how long a milestone will take, or wants status numbers.
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
  where it concentrates (Files in bsd and Unsafe in bsd are the exit criterion: a row is
  complete when both are 0 and it pays no open bridge), and the caveats that matter for the
  question asked: Moved counts the row's files already in `hal`, `kernel` or `abi` (a move into
  `hal` keeps its `unsafe`: it is not a reduction, the Crates line shows where it went); an
  `adapted` module only followed a redesign and is shown apart; a met row may keep files in
  bsd that a later row's prose claims (N1's crypto framework is N8's); the estimate divides a
  row's unsafe in bsd by the unsafe removed per active day since the baseline of
  `unsafe-budget.toml`, reads n/a until 5 active days have removed 100 or more (a smaller
  sample is noise), and promises nothing. The Crates line is the whole picture: kernel and abi
  are 0 by the compiler, hal has the only unsafe under its ceiling, bsd is what must reach 0
  files, the boot loaders are outside the plan.
- A `Warning:` line under the table names files in bsd no ROADMAP row claims: say so; they
  need an owner row.
- With a milestone argument (`/progress N2`) the file list follows the table, bsd files first
  and the most unsafe first: say what is left in it and which files hold the most.
- If the output starts with `Note:`, unsafe-report could not run (the note says why) and the
  table is the old lineage one (modules and LZ lines, no unsafe metric): say so.
- If the table is missing or starts with `progress: error`, run the script with Bash and
  report the error. Never estimate by hand.
- These numbers are for the conversation. The docs (README, STATUS, JOURNAL) take theirs from
  `cargo xtask lz status`, `cargo xtask unsafe-report` and the git commands `docs/JOURNAL.md`
  names.
