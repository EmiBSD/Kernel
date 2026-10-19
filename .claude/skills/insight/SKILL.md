---
name: insight
description: A second opinion from external models (OpenRouter, the user's key; Gemini 4 Argon and GPT Sol by default, two families, none of them Claude) on one bounded EmiBSD question, verified claim by claim against the code and the C before anything becomes work. Use only on one of three triggers - a hot-path measurement that cannot be explained (decision 43 tripped, or a profile that names a module but not a cause), a design fork between two viable shapes, or a second reviewer in a security-review.md area. Never per slice or per commit; it applies nothing.
argument-hint: <brief.md> [--models a,b] [--out dir] [--dry-run]
allowed-tools: Bash(python3 -I ${CLAUDE_SKILL_DIR}/scripts/insight.py*)
---

`/insight` is the user's decision of 2026-10-19: the main session (and the planner) may ask
external models for an opinion when blocked, and then judge, point by point, what applies.
The opinion is input, never a verdict: today's first use (the `map.rs` lookups, 2026-10-19)
found one false claim, one right conclusion with the wrong reason, and one "regression" the
C also has, next to two real items. The cycle below is what makes the opinion worth its cost.

## The cycle

1. **Trigger**: one of the three in the description. Not "I am unsure"; a measurement, a
   fork, or a review. Write which one in the brief.
2. **Brief**: `<scratchpad>/insight/<task>/BRIEF.md`, English, this template:

   ```markdown
   # Insight brief: <title>
   ## Trigger
   <decision 43 tripped on <op> | profile names <module> | design fork <A> vs <B> | review of <area>>
   ## Question
   <one question; the narrower, the cheaper and the more checkable>
   ## Measurements
   <what was measured, how (host bench, `just bench`, profile), the numbers; "none" if none>
   ## Constraints
   <the rules that bind the answer: no `unsafe` in `kernel`/`abi`; `hal` at its ceiling
   (`unsafe-budget.toml`), raised only by the user; arm64 only (decision 44); what userland
   sees never changes; no new dependency; OpenBSD's C is the specification>
   ## Excerpts
   <line-numbered ranges of the Rust (`awk 'NR>=a&&NR<=b{print NR": "$0}'`), the hal API it
   uses, and the C at the same place; never a whole file; under ~40k tokens total; when a
   `pub fn` is a guard-splitting wrapper, include the inner function it delegates to (the
   smoke test sent `uvm_unmap_remove` without `unmap_remove`, and the loop was missing)>
   ## Answer format
   <numbered claims, each with file:line evidence, the mechanism, what the C does there,
   confidence; "no claim" rather than a guess>
   ```

3. **Call**: `python3 -I ${CLAUDE_SKILL_DIR}/scripts/insight.py BRIEF.md --out <scratchpad>/insight/<task>`,
   `--dry-run` first when the brief is large (it prints the estimated input and the worst-case
   cost per model). Defaults: Gemini 4 Argon and GPT Sol, reasoning off (`--effort none`),
   16k output tokens per model, a refusal above 40k input tokens (estimated at 3 bytes per
   token: code tokenizes densely; a 48.7 KB brief cost 16.5k-21k input tokens). The key comes from `OPENROUTER_API_KEY`
   in the user's environment and nowhere else; the script refuses without it and refuses when
   the key's remaining limit is under `--min-remaining` (1 USD). Any HTTP error, a 402 above
   all, is reported to the user at once: the balance is theirs.
4. **Verdict per claim**: read each answer; for every claim, check the code and the C it
   cites and give one of: `correct`, `false` (with the line that shows it), `not a regression`
   (the C does the same), `not applicable` (a rule forbids it: `unsafe`, a `hal` API, a budget,
   amd64, the ABI edge). Agreement between models is not evidence; the code is. Only `correct`
   and allowed becomes work; `not applicable` items that would need the user's decision go to
   the user as a question, never applied.
5. **Record**: the verdict table (claim, verdict, evidence) goes in the commit body or in the
   task's `HANDOFF.md`; the raw answers stay under `--out`; the script appends one line per
   model to `ledger.tsv` next to this file (date, brief, model, tokens, cost, seconds); the
   session adds one line to `docs/JOURNAL.md` per call: date, trigger, models, claims made /
   claims that held, what became work. The ledger rides with the docs commit at the close
   (decision 31), never a commit of its own.

## Measured (the smoke test of 2026-10-19, the `map.rs` lookups brief, 48.7 KB)

| model | effort | claims | correct | new to the session | cost | s |
|---|---|---|---|---|---|---|
| google/gemini-4-argon | none | 8 | 8 | 2 (`insert_get` has a user; `splitentry` clones and drops `Handles`) | $0.08 | 71 |
| google/gemini-4-argon | low | 6 | 6 | 0 (0 reasoning tokens: low is none) | $0.08 | 44 |
| google/gemini-4-argon | high | 0 | - | reasoning ate the 16k cap, empty answer | $0.20 | 194 |
| ~deepseek/deepseek-pro-latest | none | 10 | 10 | 0 | $0.02 | 12 |
| ~deepseek/deepseek-pro-latest | high | 10 | 10 | 0 (the same claims) | $0.04 | 33 |
| qwen/qwen3.8-max-prime | none | - | - | HTTP 400: reasoning is mandatory | - | - |
| qwen/qwen3.8-max-prime | low | 10 | 10 | 0 | $0.15 | 61 |
| qwen/qwen3.8-max-prime | high | 0 | - | reasoning ate the cap, empty answer | $0.20 | 198 |
| ~openai/gpt-sol-latest (served gpt-6.1-sol) | none | 6 | 5 + one honest "no claim" | 0; the only one to say the excerpt of `uvm_unmap_remove` was a wrapper (the loop is `unmap_remove`, 3880) and to refuse to infer from it | $0.06 | 40 |

Reasoning found nothing the plain answers missed and at `high` it produced nothing at all;
the structured brief (line-numbered Rust, the C beside it, the answer format) is what made
the claims checkable. Gemini finds more. The user then dropped Qwen (mandatory reasoning,
twice the price, a doubt about its training lineage) and DeepSeek (named in Anthropic's
distillation report of February 2026; its claims were the session's own, nothing new: a model
distilled from Claude shares Claude's blind spots, which defeats the point of a second
opinion), and chose GPT Sol (`~openai/gpt-sol-latest`, the alias that follows the family's
newest model) as the second family: fewer claims, none false, and it names what the excerpts
do not show instead of inferring it. That first test cost $0.97.

Three more cases the same day (the user: "uno solo no ayuda mucho"), one per trigger, five
models, reasoning `medium` (Gemini spends 0 reasoning tokens below `high`; the others 300 to
9600), $1.74 for the fifteen answers:

| case | Gemini 4 Argon | GPT Sol | GPT Astra | Qwen3.8 Max Prime | DeepSeek Pro |
|---|---|---|---|---|---|
| A design fork: `kernel::timeout`, owner-embedded vs wheel-owned (N4.1) | 8/8, 0 false, 3 design inputs verified (stack barrier cannot be a node; `timeout_set` writes unsynchronized in the C; a cond on `tctx_running` beats a fake node) $0.06 | 10/10, 0 false, notes LZ omits `timeout_sync_order`/KCOV $0.06 | 10/10, 0 false, same content as Sol $0.29 | 8/10 (one half-wrong on `void *arg`, one self-contradicting on the self-barrier), 1 new verified (`CIRCQ_CONCAT` has no `OwnedList` equivalent: the per-element owner id makes a move O(n)) $0.12 | 9/10, 0 false, 0 new $0.06 |
| B security review of 9dab32dd (clean diff) | 5/5, signs $0.04 | 7/7, signs with its scope $0.04 | 7/7, **blocks** on the unexcerpted `VmMapEntry::handles` $0.21 | 10/10, signs $0.09 | 8/8, signs $0.03 |
| C the fault's handle clones (trap: they cross sleeps) | 6/6, 0 false, 2 new verified (the view is built twice around `copy_amap`, so the first clones are wasted on the COW path; the resident path never unlocks, so a lazy clone before `uvmfault_unlockall` is the design) $0.08 | 8/8, 0 false, 1 new verified (the C re-reads `uobj = pg->uobject` after I/O where the Rust panics on an owner-tag mismatch, a documented deviation) $0.06 | 8/8, same new as Sol $0.32 | 7/10, **1 false** (claims every post-unlock amap use is gated on `locked`; `fault.rs:879` runs on `owned.is_some()` too) and two claims built on it $0.23 | 10/10, 0 false, 0 new, says "needed" everywhere $0.05 |

Totals over the four cases: Gemini 27 claims, 0 false, 7 new; Sol 31, 0 false, 2 new; Astra
same findings as Sol at five times the price and one blocked review; Qwen 1 false and 2
derived errors in 30; DeepSeek 0 false and 0 new in 38 (it answers what the session would).
All five recommended the wheel-owned shape for `kernel::timeout`; agreement is not evidence,
the verified constraints are. The defaults stay Gemini 4 Argon + GPT Sol. The four tests
cost $2.71 (`ledger.tsv`).

## Guards

- Applies nothing. The verdict table is the only thing that leaves the scratchpad.
- One call per block; the two default models, from families other than Claude; ranges, not
  files; `--dry-run` first; reasoning off unless the user asks (a model that refuses to run
  without it is retried once at `low`, and the answer says so).
- Escalation (the user, 2026-10-19): the other models (`~openai/gpt-astra-latest`,
  `qwen/qwen3.8-max-prime`, `~deepseek/deepseek-pro-latest`, by `--models`) are asked only
  when Gemini and GPT Sol did not unblock the question, in a second call with the same brief
  and what the first two said in it; never in the first call.
- The expectation is honest: a model reading `map.rs` sees what the host profile sees; it does
  not see QEMU, the pmaps or the TLB. For "where does the time go" the answer is a profile or
  a counter, and the call comes after it, to choose how to fix what the profile named.
- Who calls: the main session and the `planner`; a `redesigner` or `debugger` only when its
  launch prompt says so. Never from a loop or a script; never by `just ci`.
- Decision 43's stop is unchanged: a hot-path op over 10 % slower without a reason still
  stops and asks the user; this skill makes the stop arrive with verified hypotheses.
- Network: this is the third network use of the tree (`.claude/rules/lz-sync.md` names the
  other two). Nothing of `reference/` leaves it that is not already public OpenBSD C.

When invoked as `/insight <brief.md>`: run step 3 with `$ARGUMENTS`, then do steps 4 and 5 and
reply with the verdict table in the user's language (the table's headers in English).
