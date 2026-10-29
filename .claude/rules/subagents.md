# Subagents: the common contract

The project's subagents are defined in `.claude/agents/*.md` (the set-up LandingZone adopted on
2026-10-09, adapted here on 2026-10-10; it replaces the ad-hoc `general-purpose` prompts of
N1). Every definition there points here; this file is the part they share. The rules it
summarises stay authoritative in their own files (`re-engineering.md`, `large-changes.md`,
`lineage.md`, `testing.md`, `git-commits.md`, `security-review.md`, `scope-and-stubs.md`,
`unsafe-budget.md`, `lz-sync.md`); when they change, this file follows.

## Roles

| Agent | Model | Worktree | Memory | Writes | Use it for |
|---|---|---|---|---|---|
| `milestone-coordinator` | opus | yes | yes | branch | an `N` milestone: measure, split, launch, integrate, close CI and docs |
| `redesigner` | opus | yes | yes | branch | a delicate redesign: locking, `unsafe`, data structures, anything in `security-review.md`'s list, any module over 3,000 lines (one each) |
| `mechanical` | sonnet | yes | yes | branch | tests added to an existing module, `lineage.toml` bookkeeping, the same call-site change over many files, doc updates once decided |
| `integrator` | opus | yes | yes | branch | merge agent branches, resolve the shared files, finish a stopped agent's work, `just ci` |
| `debugger` | opus | yes | yes | branch | root cause of a failing or flaky smoke, test or `diff-openbsd` scenario, fixed without weakening an expectation |
| `reviewer` | opus | no | no | nothing | read-only review of a branch: behaviour kept, `unsafe`, rules, security; returns the `Security-Review:` text |
| `planner` | fable | no | no | scratchpad | a focused design burst: from a surveyor's inventory, the brief an Opus redesigner executes and the decisions the user must take; proposes, never decides, never writes code (the user's decision of 2026-10-16: Fable for planning only, as a subagent so the main session stays on Opus) |
| `surveyor` | opus | no | yes | scratchpad | read-only inventory of a module before its redesign is designed: every `unsafe` and raw field by pattern, the primitives it needs and what exists of them, callers against the 50-module rule, the C's constraints, coverage; facts with `file:line`, never a design (the user's decision of 2026-10-16) |
| `openbsd-probe` | sonnet | no | no | nothing | boot real OpenBSD 8.0 on a QEMU setup and report what it does |
| `image-worker` | sonnet | no | no | scratchpad | view, crop, resize images so the coordinator's context stays small |

Mechanical vs delicate (the user's split, 2026-10-03): when unsure, it is delicate (opus).
Concurrency: at most 4 agents run at the same time, and a milestone coordinator runs at most 2
of its own. That is a ceiling on parallel runs, not on how they start: each agent is still
launched individually with the Agent tool ("one by one", below), never by a script. There is no
`external-bugs` role here: a slip found in OpenBSD's C, in QEMU, in EDK2 or in LZ goes to the
user in the final report, with the evidence; no agent reports anything upstream or to LZ (LZ
is never told this repository exists, `reference-readonly.md`).

Effort (the user's decision, 2026-10-19): every definition fixes `effort` in its frontmatter,
so a run never inherits the main session's level: `low` for `image-worker` and
`openbsd-probe`, `medium` for `mechanical` and `surveyor` (their value is completeness, which
the instruction gives, not reasoning per step), `high` for `milestone-coordinator`,
`integrator`, `redesigner`, `debugger`, `reviewer` and `planner`. The launching session may
raise one run to `xhigh` through the Agent tool's `effort` parameter when the task justifies
it (a module over 3,000 lines, a `security-review.md` area, a close that failed once) and says
so in the prompt; it never lowers one. `CLAUDE_CODE_EFFORT_LEVEL`, if ever set, overrides all
of this; it is not set.

Turns (the user's decision, 2026-10-19): the read-only roles carry `maxTurns` in their
frontmatter as a spending stop, set at about 1.5 times the longest run the project's own
transcripts show (134 runs up to that date; the longest reviewer 380 turns, surveyor 545,
planner 152): 400 for `reviewer`, 600 for `surveyor`, 200 for `planner`, 80 for
`openbsd-probe`, 30 for `image-worker`. At the limit Claude Code returns the output marked
partial; the launching session reads it and, if the work is worth finishing, resumes the run
once with `SendMessage` (its context intact); a second partial return is a stop: the user
decides. The Agent tool has no per-invocation `maxTurns`, so a launcher cannot raise one for
a big task: a cap that cuts legitimate runs is raised here, in a `rules:` commit, with the
turn counts that showed it. Whether a resumed run gets a fresh budget is not documented;
until one shows, assume it does not. The worktree roles (`milestone-coordinator`,
`integrator`, `redesigner`, `debugger`, `mechanical`) carry none: a stop in the middle of
their work leaves commits unmade, which `large-changes.md` forbids.

Guard hook (the user's decision, 2026-10-19): the writing roles (`redesigner`, `mechanical`,
`integrator`, `debugger`, `milestone-coordinator`) carry a `PreToolUse` hook in their
frontmatter, `.claude/hooks/agent-guard.sh`, that refuses before it runs: `git push`,
`git commit --amend`, a `cd` into `reference/`, any `git` other than a reading subcommand or
`fetch` inside a reference tree, and any write (a write command, a redirection, an in-place
edit, `git add`/`checkout`/`restore`, Edit or Write) under `reference/` or `lz/`. The one
write it allows is the worktree setup's own: `ln -s` of `reference/openbsd-src` and
`reference/emibsd-lz` and their unlinking (the link itself, never a path inside). It makes
the words of this file and of `reference-readonly.md` mechanical; the main session has no
such hook, since pushing and the pin bump are done there, on the user's word. A refusal is a
"STOP" like any other (below): the agent reports it, never routes around it. The hook is a
belt against a slip, not a wall: its tests are the table the session that wrote it ran, and
a case it misses is added to the script, not argued with.

Reviews and fixes (the user's decision, 2026-10-22, to spend fewer tokens; a full slice
cycle measured about 2.9 M subagent tokens at N4.3, a review about 0.25-0.45 M): every slice
gets one `reviewer` run before its merge. Its findings go back to the **same** redesigner
with `SendMessage` (its context intact, no re-reading), never to a fresh agent. A second,
closing review runs only when the first found something blocking, or when the fixes change a
`hal` `unsafe` site or its `// SAFETY:` argument, or the slice is a `security-review.md` area
(N4.8b's syscall entry and copy, N8): those keep both reviews. Otherwise the main session
checks each fix against the review's finding itself (the diff, the cited lines, the tests
the redesigner ran) and writes that check in the merge message. The reviewer's
`Security-Review:` text is required as before.

## Memory (`memory: project`)

An agent with memory keeps `.claude/agent-memory/<agent>/MEMORY.md`: lessons that outlive one
run (an idiom that took two attempts, a known flake, a conflict pattern), one line each, in
English, appended at the end. It is committed with the tree, so a worktree agent's lines travel
on its branch and merge by git's `union` driver (`.gitattributes`: both sides kept, no
conflict); whoever merges drops exact duplicates afterwards. Task
state (branches, hashes, what is left) goes in `HANDOFF.md`, never in memory. The `reviewer`
has none on purpose: it reads every change with fresh eyes. `agent-memory-local/` is not used.

## The machine lock

`/tmp/emibsd/ci.lock` is shared with LandingZone on purpose: both repositories run QEMU on the
same Mac, and the Mac is the bottleneck (the rule of 2026-10-07: no competing CI). Two
repositories sharing one lock never run two `ci`s at once. A launching session may name a lock
of its own in its scratchpad instead. There is no batch workflow: the main session (or a
`milestone-coordinator` it launches) drives the redesign, launches the roles one by one with
the Agent tool (each launch its own call with its own prompt; several may then run in
parallel up to the ceiling above) and sees every plan and every commit (the user's decision of
2026-10-10).

## What the launching prompt must give

The definitions are generic; the prompt of each launch supplies the specifics:

1. the task and its scope (`lineage.toml` modules, the ROADMAP row, the LZ files);
2. the branch to merge first (the coordinator's), or the base commit;
3. the scratchpad directory for `HANDOFF.md` and logs;
4. the machine lock path (`/tmp/emibsd/ci.lock`, or `ci.lock` in the launching session's
   scratchpad), or "no lock";
5. what else runs on the machine at the same time (in this repository and in LZ);
6. any authorisation the user gave for this launch, quoted;
7. the block "Rules that apply to this work" (decision 31): the 10-15 lines that bind the task
   (zones and the author's block, the lineage of a move or of a redesign, decision 23, the
   hook, what is forbidden) and the two or three files of `.claude/rules/` the agent reads
   whole; the rest only if the work touches them. The reviewer is launched without it and
   reads everything.

A redesign prompt also says which `bsd` modules the work deletes, which `hal` pieces it may
add or must wait for, and which bridges it creates or removes (`docs/ZERO_UNSAFE.md` section
0): no agent writes `unsafe` outside `sys/hal/`, raises `hal_ceiling`, or adds a bridge
without its `bridges.toml` row.

A prompt that misses one of these gets a question back, not a guess.

## Setup in a worktree

- The main checkout is the first line of `git worktree list --porcelain`:
  `MAIN=$(git worktree list --porcelain | sed -n '1s/^worktree //p')`. The gitignored reference
  trees live only there: `ln -s "$MAIN/reference/openbsd-src" reference/openbsd-src` and
  `ln -s "$MAIN/reference/emibsd-lz" reference/emibsd-lz` if missing (`lz/PINNED.md` is tracked
  and needs nothing). Both are read-only (`reference-readonly.md`). Never commit them, never
  `git add -A` / `git add .`; add paths by name. Unlink both before the worktree is removed.
- `PATH=/opt/homebrew/opt/rustup/bin:$PATH` before `cargo`/`just`. Install nothing.
- `diff-openbsd` needs the snapshot: `cp -Rc "$MAIN/target/openbsd" target/openbsd`. Never
  re-download it, never delete the main checkout's copy.
- The smokes need the userland (`target/userland/<arch>/ramdisk.ffs`): `cp -Rc
  "$MAIN/target/userland" target/userland` (an APFS clone, seconds), then `just userland`,
  which rebuilds only what its stamps say changed. Once, before any smoke.
- Read the "Rules that apply to this work" block of your prompt and the rule files it names
  before writing anything (decision 31); this file is the common contract. The reviewer reads
  CLAUDE.md and every file of `.claude/rules/`.

## While working

- Author block: every `.rs` under `sys/` and `tools/` opens `/* <LICENSES> */` with the
  author's ISC block (`AUTHOR_BLOCK` in `tools/xtask/src/layout.rs`), then one blank line and
  the original notice(s) of every LZ source, whole (`scope-and-stubs.md`, Authorship).
- Commit early: each piece that builds and passes clippy/fmt is committed at once. Trailers
  (`git-commits.md`): `LZ: <lz path>@<12-hex>` per LZ source file, `Unsafe: <subsystem>
  <before> -> <after>` per subsystem whose count moved, `Security-Review:` in the areas of
  `security-review.md`, `Bench:` when measured, then the session's `Co-Authored-By:` line.
  Never push, never amend a published commit, never force anything, never touch `main`. Never
  mix a redesign with an `lz-sync:` or a `lineage:` commit.
- Machine lock: while the lock directory exists and is not yours, run no smokes, no
  `just test`, no full `just build`/`just clippy`, no `ci`; reading, editing and a single-target
  check are fine. To take it: `mkdir -p` its parent once, then `mkdir <lock>` (it fails while someone
  holds it), your name in `<lock>/owner`, `rmdir`-remove it when done. Only coordinators and integrators run `just ci` / `just smoke` (all), and only at a milestone
  close (`testing.md`); redesigners build nothing and run nothing per commit (the user's decision of
  2026-10-16, `testing.md`): they write, commit, and leave compiling to the milestone close; a
  smoke only when their prompt names one (a `security-review.md` area).
- Long runs (`large-changes.md`): every background run of minutes has a watcher that warns when
  its log has not changed for ten minutes (`stat -f %m <log>`); a progress report gives the
  log's last change time; before the final report every background task started is waited for
  or stopped, and `ps` shows none left (the report names any that was killed).
- Read cheaply: `grep`, `sed -n 'a,bp'` with literal numbers, `grep -n -A<N> '<pattern>'` to read
  from a pattern (a computed `sed` range on `reference/` is refused: `reference-readonly.md`),
  long output redirected to scratchpad files. An LZ
  module being redesigned, and the C behind it, are still read completely, in ranges.
- Behaviour: OpenBSD's behaviour is the specification and what userland sees never changes
  (the syscall ABI, sysctl MIBs, ioctls, device majors, every `#[repr(C)]` shared with
  userland, every line a smoke or `diff-openbsd` compares). No behavioural deviation to make
  something pass; an inherited `unported!` path is closed or kept visible, never widened.

## Stopping

- A "STOP ... wait for the user" refusal or a permission denial: stop at once, commit nothing
  more, report it. Never retry it or route around it.
- Anything that is the user's decision: a dependency, a download, an install, anything outside
  the repository, a raise of a line in `unsafe-budget.toml`, a bump of `lz/PINNED.md` or of the
  toolchain, dropping an LZ file, moving a whole subsystem, a scope change: stop and hand back
  with the question.
- This repository's own stop rules: a change userland could see (the ABI edge); an LZ security
  fix skipped without a `covered-by-redesign` reason that names the test or the type; a new
  algorithm or dependency in `sys/crypto/`; a check the C makes relaxed instead of made a type;
  more `unsafe` in a module than LZ had, without a reason: stop and ask.
- Low on context: stop in a committed state, update `HANDOFF.md` (done with hashes, left,
  pending diffs, the `lineage.toml` rows and the numbers for the trailers), then hand back
  naming its path.

## Final report

Branch and tip, commits in order, the evidence (`cargo xtask unsafe-report` before and after
per subsystem, test counts before and after, serial lines, `rc=`, wall times, the
`diff-openbsd` result), the `lineage.toml` rows changed, deviations closed or kept, what is
left, the `HANDOFF.md` path, and the background tasks stopped. Then `rm -rf target/smoke` in
the worktree. The host checks run once per branch (decision 31): the agent quotes the lines of
`cargo xtask lz check`, `unsafe-report`, `hal-check` and `abi-check` it saw at its tip, and the
integrator runs them once more on `main` after the merge, not in the worktree.
