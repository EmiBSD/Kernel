/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
/* </LICENSES> */

/* <CODE> */
//! `cargo xtask unsafe-report [--write|--check|--json] [--shapes]`: how much `unsafe` the
//! kernel holds, per subsystem.
//!
//! The Phase 2 baseline (docs/PHASE2.md, "Metrics"). Every `.rs` file of the kernel crates is
//! read: `sys/` (the `bsd` crate, `sys/lib/libkern`, `sys/lib/libz`) and `init/` (the
//! freestanding init stand-in). `tools/xtask` is host tooling and is not counted.
//!
//! A small lexer ([`lex`]) turns each file into identifiers and punctuation, dropping comments
//! (nested block comments too), string, byte-string, C-string and raw-string literals, char
//! and byte literals, lifetimes and numbers, so an `unsafe` in a comment or a string never
//! counts and `r#unsafe` is an identifier, not the keyword. Each `unsafe` keyword is then
//! classified by the tokens that follow it ([`Kind`]).
//!
//! Test code is counted apart: the tests live inline since M15 (`#[cfg(test)] mod tests { .. }` in
//! the TESTS zone of their file), so the rule that counts them is the `cfg` one below; files
//! named `tests.rs` (and anything under a `tests/` directory) still count as test files, as do
//! files whose `mod` declaration is test-only, a file that starts with
//! `#![cfg(test)]`, and the item that follows a test-only `#[cfg(...)]` attribute (a
//! `mod tests { .. }`, a test helper `fn`, an `impl`). A `cfg` predicate is test-only when it
//! can only hold under `cargo test`: `test`, `all(.., test, ..)`, or `any(..)` whose every
//! arm is test-only. `cfg(any(test, feature = "x"))` code also builds into a kernel, so it
//! counts as kernel code.
//!
//! The grouping is in [`subsystem`] and docs/PORTING.md ("Measuring unsafe").
//! `--write` replaces the one line of docs/STATUS.md that starts with `Unsafe` with the
//! totals line ([`totals_line`]).
//!
//! The crates of docs/ZERO_UNSAFE.md 0.4 (`kernel`, `abi`, `hal`, `boot`, `bsd`, and
//! `hal_pending`: the hardware and the lists and trees that move into `hal` at N2) are counted
//! from each file's path ([`crate_of_file`]); `--check` bounds them ([`crate_errors`]) and the
//! rows `kernel`, `abi` and `hal` carry no per-row budget ([`is_crate_row`]).
//!
//! `--json` prints the same measure ([`measure`]) as one JSON document instead ([`json`]):
//! every kernel file with its lineage status, group, `unsafe` and shapes, the totals, the
//! ratchet with its baseline, and the pace of the legacy `unsafe` removed since that
//! baseline ([`rate`]). `/progress` reads it; the field names are stable
//! (.claude/rules/xtask.md).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;

const STATUS_DOC: &str = "docs/STATUS.md";
const STATUS_PREFIX: &str = "Unsafe";
const BUDGET_FILE: &str = "unsafe-budget.toml";
const CORE_FILE: &str = "unsafe-core.toml";

/// One token of a Rust source file, as far as counting `unsafe` needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// An identifier or keyword (`r#name` keeps its `r#`).
    Ident(String),
    /// One punctuation character (`{`, `;`, `#`, ...).
    Punct(char),
    /// A plain `".."` string literal, its text as written between the quotes (escapes kept):
    /// what a `#[path = ".."]` attribute names.
    Str(String),
    /// Any other string, char, byte, number literal or a lifetime: its text does not matter.
    Lit,
}

/// What one `unsafe` keyword introduces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `unsafe { .. }`.
    Block,
    /// `unsafe fn name`, `unsafe extern "C" fn name`.
    Fn,
    /// `unsafe impl`.
    Impl,
    /// `unsafe trait`, `unsafe auto trait`.
    Trait,
    /// Anything else: `unsafe extern "C" { .. }` blocks, `#[unsafe(no_mangle)]` attributes,
    /// `unsafe fn(..)` pointer types.
    Other,
}

/// Counts of each [`Kind`], plus how many files they came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) files: usize,
    pub(crate) blocks: usize,
    pub(crate) fns: usize,
    pub(crate) impls: usize,
    pub(crate) traits: usize,
    pub(crate) other: usize,
}

impl Counts {
    fn add(&mut self, k: Kind) {
        match k {
            Kind::Block => self.blocks += 1,
            Kind::Fn => self.fns += 1,
            Kind::Impl => self.impls += 1,
            Kind::Trait => self.traits += 1,
            Kind::Other => self.other += 1,
        }
    }

    fn merge(&mut self, o: &Counts) {
        self.files += o.files;
        self.blocks += o.blocks;
        self.fns += o.fns;
        self.impls += o.impls;
        self.traits += o.traits;
        self.other += o.other;
    }

    fn total(&self) -> usize {
        self.blocks + self.fns + self.impls + self.traits + self.other
    }
}

/// One subsystem's kernel and test counts.
#[derive(Debug, Default)]
pub(crate) struct Row {
    kernel: Counts,
    test: Counts,
}

/// `unsafe-budget.toml` (`.claude/rules/unsafe-budget.md`): the kernel total each subsystem may
/// not exceed, and the zero-unsafe ratchet (docs/ZERO_UNSAFE.md, section 5).
#[derive(serde::Deserialize)]
struct Budget {
    #[serde(default, rename = "subsystem")]
    subsystems: Vec<BudgetRow>,
    #[serde(default)]
    ratchet: Option<Ratchet>,
}

#[derive(serde::Deserialize)]
struct BudgetRow {
    name: String,
    total: usize,
}

/// The `[ratchet]` table. Two numbers only move one way: the `forbid` modules may not fall
/// below `forbid_floor`, the legacy `unsafe` (outside the core) may not rise above
/// `legacy_ceiling`. Two never move: `legacy_baseline`, the legacy total on `baseline_date`,
/// the day the zero-unsafe plan was adopted, written once by hand and kept as they are by
/// `--write`; [`rate`] measures from them.
#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ratchet {
    pub(crate) forbid_floor: usize,
    pub(crate) legacy_ceiling: usize,
    #[serde(default)]
    pub(crate) legacy_baseline: Option<usize>,
    /// The most `unsafe` the `hal` crate (`sys/hal/`) may hold; `cargo xtask hal-check`
    /// compares against it and `--write` only lowers it. Written by hand the first time.
    #[serde(default)]
    pub(crate) hal_ceiling: Option<usize>,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub(crate) baseline_date: Option<String>,
}

/// The rows and the ratchet of `unsafe-budget.toml`; empty and `None` when the file is absent.
pub(crate) fn load_budget(root: &Path) -> Result<(BTreeMap<String, usize>, Option<Ratchet>)> {
    let path = root.join(BUDGET_FILE);
    if !path.is_file() {
        return Ok((BTreeMap::new(), None));
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let b: Budget = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(d) = b.ratchet.as_ref().and_then(|r| r.baseline_date.as_deref())
        && !is_iso_date(d)
    {
        return Err(format!(
            "{}: [ratchet] baseline_date = \"{d}\" is not a YYYY-MM-DD date",
            path.display()
        )
        .into());
    }
    let rows = b
        .subsystems
        .into_iter()
        .map(|r| (r.name, r.total))
        .collect();
    Ok((rows, b.ratchet))
}

/// What `--check` says of the ratchet: one line per number on the wrong side of its bound.
pub(crate) fn ratchet_errors(r: &Ratchet, forbid: usize, legacy: usize) -> Vec<String> {
    let mut out = Vec::new();
    if forbid < r.forbid_floor {
        out.push(format!(
            "forbid modules: {forbid}, below the floor {} ({BUDGET_FILE} [ratchet]); a module lost its #[forbid(unsafe_code)]",
            r.forbid_floor
        ));
    }
    if legacy > r.legacy_ceiling {
        out.push(format!(
            "legacy unsafe: {legacy}, above the ceiling {} ({BUDGET_FILE} [ratchet]); unsafe outside unsafe-core.toml may only fall",
            r.legacy_ceiling
        ));
    }
    out
}

/// The ratchet `--write` records: the floor only rises and the ceiling only falls, the
/// baseline stays as it is; with no ratchet yet, today's numbers and no baseline.
pub(crate) fn ratchet_next(
    old: Option<&Ratchet>,
    forbid: usize,
    legacy: usize,
    hal: usize,
) -> Ratchet {
    match old {
        Some(o) => Ratchet {
            forbid_floor: o.forbid_floor.max(forbid),
            legacy_ceiling: o.legacy_ceiling.min(legacy),
            legacy_baseline: o.legacy_baseline,
            hal_ceiling: o.hal_ceiling.map(|c| c.min(hal)),
            baseline_date: o.baseline_date.clone(),
        },
        None => Ratchet {
            forbid_floor: forbid,
            legacy_ceiling: legacy,
            legacy_baseline: None,
            hal_ceiling: None,
            baseline_date: None,
        },
    }
}

/// `s` is a `YYYY-MM-DD` date (digits where digits go; the calendar is not checked).
fn is_iso_date(s: &str) -> bool {
    s.len() == 10
        && s.bytes().enumerate().all(|(i, c)| match i {
            4 | 7 => c == b'-',
            _ => c.is_ascii_digit(),
        })
}

/// The comment `--write` puts at the top of `unsafe-budget.toml`.
const BUDGET_HEADER: &str = "\
# Per-subsystem unsafe budget: the kernel total (blocks + fn + impl + trait + other)
# each subsystem may not exceed. `cargo xtask unsafe-report --check` (in `just ci`) fails
# when one does; `--write` lowers a budget to the current count and never raises it.
# Raising a budget is a commit of its own, with the reason in the body
# (.claude/rules/unsafe-budget.md). Names are unsafe-report's row names.
#
# [ratchet] (docs/ZERO_UNSAFE.md, section 5): the `forbid` modules may not fall below
# forbid_floor, and the `unsafe` outside unsafe-core.toml may not rise above
# legacy_ceiling; `--write` only raises the floor and lowers the ceiling.
# legacy_baseline is the legacy total on baseline_date, the day the plan was adopted;
# /progress measures its pace from them. Written once by hand; `--write` keeps them.
";

/// The text `--write` gives `unsafe-budget.toml`: the header, the ratchet (its baseline as
/// it was) and one table per row, `rows` being each row's new budget.
pub(crate) fn budget_text(r: &Ratchet, rows: &BTreeMap<String, usize>) -> String {
    let mut text = String::from(BUDGET_HEADER);
    text.push_str(&format!(
        "\n[ratchet]\nforbid_floor = {}\nlegacy_ceiling = {}\n",
        r.forbid_floor, r.legacy_ceiling
    ));
    if let Some(b) = r.legacy_baseline {
        text.push_str(&format!("legacy_baseline = {b}\n"));
    }
    if let Some(c) = r.hal_ceiling {
        text.push_str(&format!("hal_ceiling = {c}\n"));
    }
    if let Some(d) = &r.baseline_date {
        text.push_str(&format!("baseline_date = \"{d}\"\n"));
    }
    for (name, total) in rows {
        text.push_str(&format!(
            "\n[[subsystem]]\nname = \"{name}\"\ntotal = {total}\n"
        ));
    }
    text
}

/// The three groups of docs/ZERO_UNSAFE.md, section 5, over the kernel counts (test code
/// apart): the core of `unsafe-core.toml`, the `forbid` modules, and legacy (the rest).
#[derive(Debug, Default)]
pub(crate) struct Groups {
    /// `unsafe` in the core's files.
    pub(crate) core: usize,
    /// Core files that hold any.
    pub(crate) core_files: usize,
    /// The core's `unsafe` per `unsafe-report` row.
    pub(crate) core_rows: BTreeMap<String, usize>,
    /// `unsafe` outside the core.
    pub(crate) legacy: usize,
    /// Files outside the core that hold any.
    pub(crate) legacy_files: usize,
    /// Modules (the `.rs` files under `sys/` that `lineage.toml` tracks) whose `mod`
    /// declaration is `#[forbid(unsafe_code)]`, directly or through an ancestor.
    pub(crate) forbid: usize,
    /// All those modules.
    pub(crate) modules: usize,
    /// The v2 crates (docs/ZERO_UNSAFE.md 0.4): files and `unsafe` per crate, keyed by
    /// [`CRATE_NAMES`] (test code apart).
    pub(crate) crates: BTreeMap<&'static str, CrateCount>,
    /// The `unsafe` of `bsd` files that `unsafe-core.toml` does not list: what
    /// `legacy_ceiling` bounds (the old runtime files are in `bsd` and in the core).
    pub(crate) bsd_noncore: usize,
}

/// Files (test-only files apart) and kernel-code `unsafe` of one crate.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CrateCount {
    pub(crate) files: usize,
    pub(crate) unsafe_n: usize,
}

/// The crates `unsafe-report` prints, in order. `hal_pending` is the hardware and the lists
/// and trees that are still under `sys/arch/`, `sys/machine/` and `queue`/`tree` and move into
/// `hal` at N2: not `bsd` any more, not `hal` yet.
pub(crate) const CRATE_NAMES: [&str; 6] = ["kernel", "abi", "hal", "hal_pending", "boot", "bsd"];

/// The crate a file belongs to (`rel` relative to the root): `kernel`, `abi`, `hal`, `boot`
/// (`sys/stand/`, `sys/lib/libsa/`, `sys/arch/*/stand/`), `hal_pending`, or `bsd` for every
/// other file under `sys/` and `init/`; `None` for anything else.
pub(crate) fn crate_of_file(rel: &str) -> Option<&'static str> {
    if rel.starts_with("init/") {
        // The binary that links the crates: part of the legacy `bsd` (STATUS counts it there).
        return Some("bsd");
    }
    let r = rel.strip_prefix("sys/")?;
    let parts: Vec<&str> = r.split('/').collect();
    Some(match parts.as_slice() {
        ["stand", ..] | ["lib", "libsa", ..] | ["arch", _, "stand", ..] => "boot",
        ["kernel", ..] => "kernel",
        ["abi", ..] => "abi",
        ["hal", ..] => "hal",
        ["arch", ..]
        | ["machine", ..]
        | ["sys", "queue.rs" | "tree.rs"]
        | ["kern", "subr_tree.rs"] => "hal_pending",
        _ => "bsd",
    })
}

/// A row that is a v2 crate (`kernel`, `abi`, `hal`): no per-row budget, `crate_errors` and
/// `hal_ceiling` bound it instead.
pub(crate) fn is_crate_row(name: &str) -> bool {
    matches!(name, "kernel" | "abi" | "hal")
}

/// What `--check` says of the crates: `kernel` and `abi` must be 0 (the compiler already
/// refuses it), `hal` at or under `hal_ceiling` when the key is there, and the `bsd` files
/// outside the core at or under `legacy_ceiling`.
pub(crate) fn crate_errors(
    r: Option<&Ratchet>,
    crates: &BTreeMap<&'static str, CrateCount>,
    bsd_noncore: usize,
) -> Vec<String> {
    let n = |c: &str| crates.get(c).map_or(0, |x| x.unsafe_n);
    let mut out = Vec::new();
    for c in ["kernel", "abi"] {
        if n(c) > 0 {
            out.push(format!(
                "{c}: {} unsafe; the crate is #![forbid(unsafe_code)] and holds none",
                n(c)
            ));
        }
    }
    if let Some(r) = r {
        if let Some(h) = r.hal_ceiling
            && n("hal") > h
        {
            out.push(format!(
                "hal: {} unsafe, above hal_ceiling {h} ({BUDGET_FILE} [ratchet]); it may only fall",
                n("hal")
            ));
        }
        if bsd_noncore > r.legacy_ceiling {
            out.push(format!(
                "bsd: {bsd_noncore} unsafe outside the core, above legacy_ceiling {} ({BUDGET_FILE} [ratchet])",
                r.legacy_ceiling
            ));
        }
    }
    out
}

/// `crates: kernel 0, abi 0, hal 0, hal_pending 2000, boot 319, bsd 8906` (the unsafe of each).
pub(crate) fn crates_summary(crates: &BTreeMap<&'static str, CrateCount>) -> String {
    CRATE_NAMES
        .iter()
        .map(|c| format!("{c} {}", crates.get(c).map_or(0, |x| x.unsafe_n)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every kernel `.rs` file (`sys/` and `init/`), lexed, by absolute path.
pub(crate) fn lex_tree(root: &Path) -> Result<BTreeMap<PathBuf, Vec<Tok>>> {
    let mut files = Vec::new();
    for dir in ["sys", "init"] {
        crate::walk_rs(&root.join(dir), &mut files)?;
    }
    let mut lexed = BTreeMap::new();
    for f in files {
        let src = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let toks = lex(&src);
        lexed.insert(f, toks);
    }
    Ok(lexed)
}

/// Which files are test-only: by name first, then by the test-only `mod x;` declarations,
/// until nothing changes (a test file's own `mod` lines declare test files too).
fn test_files(lexed: &BTreeMap<PathBuf, Vec<Tok>>) -> BTreeSet<PathBuf> {
    let mut test: BTreeSet<PathBuf> = lexed.keys().filter(|f| is_test_path(f)).cloned().collect();
    loop {
        let mut added = false;
        for (f, toks) in lexed {
            let scan = scan(toks, test.contains(f));
            for name in scan.test_mods {
                for child in child_module_paths(f, &name) {
                    if lexed.contains_key(&child) && test.insert(child) {
                        added = true;
                    }
                }
            }
        }
        if !added {
            return test;
        }
    }
}

/// `f` relative to the workspace root, with `/` separators.
fn rel_path(root: &Path, f: &Path) -> String {
    let rel = f.strip_prefix(root).unwrap_or(f);
    rel.to_string_lossy().replace('\\', "/")
}

/// One kernel file as the report measures it.
#[derive(Debug, Clone)]
pub(crate) struct FileMeasure {
    /// Relative to the workspace root, with `/` separators.
    pub(crate) rel: String,
    /// Its row ([`subsystem`]).
    pub(crate) subsystem: String,
    /// A test-only file ([`test_files`]): all its code is test code.
    pub(crate) test: bool,
    /// Listed in `unsafe-core.toml`.
    pub(crate) core: bool,
    /// Its module is `#[forbid(unsafe_code)]` ([`forbid_files`]).
    pub(crate) forbid: bool,
    /// One of the modules the forbid ratchet counts: under `sys/` and not structure (a crate
    /// root, `mod.rs`, `build.rs`, or a file of `sys/machine/`, `sys/arch/host/`, `sys/stand/`).
    pub(crate) module: bool,
    /// Its `unsafe` in kernel code.
    pub(crate) kernel: Counts,
    /// The shapes to replace, in its kernel code.
    pub(crate) shapes: Shapes,
}

impl FileMeasure {
    /// Its group (docs/ZERO_UNSAFE.md, section 5): `core` when `unsafe-core.toml` lists it,
    /// else `forbid` when its module is, else `legacy`.
    pub(crate) fn group(&self) -> &'static str {
        if self.core {
            "core"
        } else if self.forbid {
            "forbid"
        } else {
            "legacy"
        }
    }
}

/// What the report measures: every kernel file, the rows and groups they add up to, and the
/// two files they are checked against.
pub(crate) struct Measure {
    /// Every kernel `.rs` file, ordered by path.
    pub(crate) files: Vec<FileMeasure>,
    /// Kernel and test counts per row.
    pub(crate) rows: BTreeMap<String, Row>,
    /// The shapes per row.
    pub(crate) shape_rows: BTreeMap<String, Shapes>,
    /// The three groups.
    pub(crate) groups: Groups,
    /// `unsafe-budget.toml`'s rows.
    pub(crate) budget: BTreeMap<String, usize>,
    /// `unsafe-budget.toml`'s `[ratchet]`.
    pub(crate) ratchet: Option<Ratchet>,
    /// `unsafe-core.toml`.
    pub(crate) core: CoreList,
}

impl Measure {
    /// The row budgets left to the core: over the rows that hold core files, each row's
    /// budget (its count when it has none) minus its legacy `unsafe`. The core's files may
    /// hold that much before a row fails `--check`, with legacy as it stands.
    pub(crate) fn core_budget(&self) -> usize {
        self.groups
            .core_rows
            .iter()
            .map(|(name, core)| {
                let total = self.rows.get(name).map_or(0, |r| r.kernel.total());
                let legacy = total.saturating_sub(*core);
                self.budget
                    .get(name)
                    .copied()
                    .unwrap_or(total)
                    .saturating_sub(legacy)
            })
            .sum()
    }
}

/// Measure the kernel tree under `root` (`sys/` and `init/`) against `unsafe-core.toml` and
/// `unsafe-budget.toml`.
pub(crate) fn measure(root: &Path) -> Result<Measure> {
    let lexed = lex_tree(root)?;
    let core = CoreList::load(root)?;
    let (budget, ratchet) = load_budget(root)?;
    Ok(measure_lexed(root, &lexed, core, budget, ratchet))
}

/// [`measure`] over files already lexed, keyed by their path under `root`.
fn measure_lexed(
    root: &Path,
    lexed: &BTreeMap<PathBuf, Vec<Tok>>,
    core: CoreList,
    budget: BTreeMap<String, usize>,
    ratchet: Option<Ratchet>,
) -> Measure {
    let test_files = test_files(lexed);
    let forbid = forbid_files(lexed);
    let mut m = Measure {
        files: Vec::new(),
        rows: BTreeMap::new(),
        shape_rows: BTreeMap::new(),
        groups: Groups::default(),
        budget,
        ratchet,
        core,
    };
    for (f, toks) in lexed {
        let rel = rel_path(root, f);
        let is_test = test_files.contains(f);
        let sub = subsystem(root, &rel);
        let scan = scan(toks, is_test);
        let mut k = Counts::default();
        let mut t = Counts::default();
        for (kind, in_test) in scan.found {
            if in_test {
                t.add(kind);
            } else {
                k.add(kind);
            }
        }
        if is_test {
            t.files = 1;
        } else {
            k.files = 1;
        }
        let row = m.rows.entry(sub.clone()).or_default();
        row.kernel.merge(&k);
        row.test.merge(&t);

        let n = k.total();
        let in_core = m.core.contains(&rel);
        if in_core {
            m.groups.core += n;
            *m.groups.core_rows.entry(sub.clone()).or_default() += n;
            m.groups.core_files += usize::from(n > 0);
        } else {
            m.groups.legacy += n;
            m.groups.legacy_files += usize::from(n > 0);
        }
        if let Some(c) = crate_of_file(&rel) {
            let cc = m.groups.crates.entry(c).or_default();
            cc.files += usize::from(!is_test);
            cc.unsafe_n += n;
            if c == "bsd" && !in_core {
                m.groups.bsd_noncore += n;
            }
        }
        let module = rel.starts_with("sys/") && !crate::is_structural(&rel);
        let is_forbid = forbid.contains(f);
        if module {
            m.groups.modules += 1;
            m.groups.forbid += usize::from(is_forbid);
        }
        m.shape_rows
            .entry(sub.clone())
            .or_default()
            .merge(&scan.shapes);
        m.files.push(FileMeasure {
            rel,
            subsystem: sub,
            test: is_test,
            core: in_core,
            forbid: is_forbid,
            module,
            kernel: k,
            shapes: scan.shapes,
        });
    }
    m
}

/// `cargo xtask unsafe-report [--write] [--check] [--shapes]`, or `--json` alone.
pub(crate) fn unsafe_report(
    root: &Path,
    write: bool,
    check: bool,
    per_module: bool,
    json_out: bool,
) -> Result<()> {
    if json_out && (write || check || per_module) {
        return Err("unsafe-report: --json goes alone".into());
    }
    let m = measure(root)?;
    if json_out {
        let statuses = crate::lz::lineage_statuses(root)?;
        let log = crate::git(root, &["log", "--format=%cd", "--date=short"])?;
        let dates: Vec<&str> = log.lines().collect();
        let pace = rate(m.ratchet.as_ref(), m.groups.legacy, &dates);
        print!("{}", json(&m, &statuses, &pace));
        return Ok(());
    }
    let groups = &m.groups;
    let rows = &m.rows;

    let mut kernel = Counts::default();
    let mut test = Counts::default();
    println!(
        "{:<16} {:>5} {:>6} {:>4} {:>4} {:>5} {:>5} {:>6} | {:>5} {:>6} {:>4} {:>4} {:>5}",
        "subsystem",
        "files",
        "blocks",
        "fn",
        "impl",
        "trait",
        "other",
        "total",
        "tests",
        "blocks",
        "fn",
        "impl",
        "other"
    );
    let print_row = |name: &str, k: &Counts, t: &Counts| {
        println!(
            "{:<16} {:>5} {:>6} {:>4} {:>4} {:>5} {:>5} {:>6} | {:>5} {:>6} {:>4} {:>4} {:>5}",
            name,
            k.files,
            k.blocks,
            k.fns,
            k.impls,
            k.traits,
            k.other,
            k.total(),
            t.files,
            t.blocks,
            t.fns,
            t.impls,
            t.traits + t.other
        );
    };
    for (name, row) in rows {
        print_row(name, &row.kernel, &row.test);
        kernel.merge(&row.kernel);
        test.merge(&row.test);
    }
    print_row("total", &kernel, &test);

    // The shapes to replace (docs/ZERO_UNSAFE.md, section 2): counted, never gated.
    println!(
        "\n{:<16} {:>10} {:>10} {:>7} {:>10}   (shapes, kernel code)",
        "subsystem", "StaticCell", "Send/Sync", "Cell<*", "UnsafeCell"
    );
    let print_shapes = |name: &str, s: &Shapes| {
        println!(
            "{:<16} {:>10} {:>10} {:>7} {:>10}",
            name, s.static_cell, s.send_sync, s.raw_cell, s.unsafe_cell
        );
    };
    let mut all_shapes = Shapes::default();
    for (name, s) in &m.shape_rows {
        if !s.is_empty() {
            print_shapes(name, s);
        }
        all_shapes.merge(s);
    }
    print_shapes("total", &all_shapes);
    if per_module {
        println!();
        for f in m.files.iter().filter(|f| !f.shapes.is_empty()) {
            print_shapes(&f.rel, &f.shapes);
        }
    }

    let ratchet = m.ratchet.as_ref();
    let shown =
        |f: fn(&Ratchet) -> usize| ratchet.map_or_else(|| "none".to_string(), |r| f(r).to_string());
    println!("\nzero-unsafe groups (docs/ZERO_UNSAFE.md; {CORE_FILE}):");
    let core_rows: Vec<String> = groups
        .core_rows
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(name, n)| format!("{name} {n}"))
        .collect();
    println!(
        "  core   {:>6} unsafe in {} files ({})",
        groups.core,
        groups.core_files,
        core_rows.join(", ")
    );
    println!(
        "  forbid {:>6} of {} modules (floor {})",
        groups.forbid,
        groups.modules,
        shown(|r| r.forbid_floor)
    );
    println!(
        "  legacy {:>6} unsafe in {} files (ceiling {})",
        groups.legacy,
        groups.legacy_files,
        shown(|r| r.legacy_ceiling)
    );
    println!("\ncrates (docs/ZERO_UNSAFE.md 0.4; files, unsafe in kernel code):");
    for c in CRATE_NAMES {
        let cc = groups.crates.get(c).copied().unwrap_or_default();
        println!("  {c:<12} {:>5} files {:>6} unsafe", cc.files, cc.unsafe_n);
    }
    let line = totals_line(&kernel, &test, groups);
    println!("\n{line}");

    if write {
        let path = root.join(STATUS_DOC);
        let doc = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let new = replace_status_line(&doc, &line)?;
        fs::write(&path, new).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("wrote {STATUS_DOC}");
        // The budget only ever goes down here, the floor only up, and the baseline stays;
        // raising a budget is a commit of its own.
        let next = ratchet_next(
            ratchet,
            groups.forbid,
            groups.legacy,
            groups.crates.get("hal").map_or(0, |c| c.unsafe_n),
        );
        let budgets: BTreeMap<String, usize> = rows
            .iter()
            .filter(|(name, _)| !is_crate_row(name))
            .map(|(name, row)| {
                let now = row.kernel.total();
                (
                    name.clone(),
                    m.budget.get(name).map_or(now, |b| (*b).min(now)),
                )
            })
            .collect();
        let path = root.join(BUDGET_FILE);
        fs::write(&path, budget_text(&next, &budgets))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("wrote {BUDGET_FILE}");
    }
    if check {
        if m.budget.is_empty() {
            return Err(format!(
                "{BUDGET_FILE} missing or empty; run `cargo xtask unsafe-report --write`"
            )
            .into());
        }
        let mut bad = 0usize;
        for (name, row) in rows {
            if is_crate_row(name) {
                continue;
            }
            let now = row.kernel.total();
            match m.budget.get(name) {
                Some(b) if now > *b => {
                    println!("error: {name}: {now} unsafe, budget {b} ({BUDGET_FILE})");
                    bad += 1;
                }
                Some(_) => {}
                None => {
                    println!(
                        "error: {name}: no budget in {BUDGET_FILE} (run `cargo xtask unsafe-report --write`)"
                    );
                    bad += 1;
                }
            }
        }
        for e in crate_errors(ratchet, &groups.crates, groups.bsd_noncore) {
            println!("error: {e}");
            bad += 1;
        }
        for p in m.core.missing(root) {
            println!("error: {CORE_FILE}: `{p}` names no file");
            bad += 1;
        }
        match ratchet {
            Some(r) => {
                for e in ratchet_errors(r, groups.forbid, groups.legacy) {
                    println!("error: {e}");
                    bad += 1;
                }
            }
            None => {
                println!("error: {BUDGET_FILE}: no [ratchet] table (forbid_floor, legacy_ceiling)");
                bad += 1;
            }
        }
        if bad > 0 {
            return Err(format!(
                "{bad} unsafe budget error(s): a subsystem over or without its budget, the core list or the ratchet"
            )
            .into());
        }
        println!(
            "unsafe-report --check: every subsystem within its budget; forbid {} >= {}, legacy {} <= {}",
            groups.forbid,
            shown(|r| r.forbid_floor),
            groups.legacy,
            shown(|r| r.legacy_ceiling)
        );
    }
    Ok(())
}

/// The pace of the zero-unsafe work (`--json`'s `rate`): the legacy `unsafe` removed since
/// the ratchet's baseline, per active day (a day with a commit, from `baseline_date` on).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Rate {
    /// Distinct commit dates on or after `baseline_date`; 0 without a baseline.
    pub(crate) active_days: usize,
    /// `legacy_baseline` minus the legacy total now; 0 without a baseline or when it rose.
    pub(crate) removed: usize,
    /// `removed / active_days`; `None` ("n/a") without a baseline, before two active days or
    /// while nothing has been removed.
    pub(crate) per_day: Option<f64>,
}

/// The [`Rate`] of the legacy total `legacy` given the ratchet `r` and the commit dates
/// `dates` (`YYYY-MM-DD`, in any order, repeats allowed).
pub(crate) fn rate(r: Option<&Ratchet>, legacy: usize, dates: &[&str]) -> Rate {
    let (Some(baseline), Some(since)) = (
        r.and_then(|r| r.legacy_baseline),
        r.and_then(|r| r.baseline_date.as_deref()),
    ) else {
        return Rate {
            active_days: 0,
            removed: 0,
            per_day: None,
        };
    };
    let active_days = dates
        .iter()
        .filter(|d| **d >= since)
        .collect::<BTreeSet<_>>()
        .len();
    let removed = baseline.saturating_sub(legacy);
    let per_day = (active_days >= 2 && removed > 0).then(|| removed as f64 / active_days as f64);
    Rate {
        active_days,
        removed,
        per_day,
    }
}

/// `s` as a JSON string literal.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `n` as a JSON number, or `null`.
fn json_opt(n: Option<usize>) -> String {
    n.map_or_else(|| "null".to_string(), |n| n.to_string())
}

/// `--json`: the measure as one document, a line per row and per file. The field names are
/// stable (.claude/rules/xtask.md lists them); `statuses` maps a path to its `lineage.toml`
/// status (`extra` for an `[[extra]]`), and a file it does not name gets `null`.
pub(crate) fn json(m: &Measure, statuses: &BTreeMap<String, &'static str>, pace: &Rate) -> String {
    let g = &m.groups;
    let kernel: usize = m.rows.values().map(|r| r.kernel.total()).sum();
    let tests: usize = m.rows.values().map(|r| r.test.total()).sum();
    let mut out = String::from("{\n  \"schema\": 1,\n");
    out.push_str(&format!(
        "  \"totals\": {{\"kernel_unsafe\": {kernel}, \"test_unsafe\": {tests}, \
         \"core_unsafe\": {}, \"core_unsafe_files\": {}, \"legacy_unsafe\": {}, \
         \"legacy_unsafe_files\": {}, \"modules\": {}, \"forbid_modules\": {}, \
         \"core_budget\": {}}},\n",
        g.core,
        g.core_files,
        g.legacy,
        g.legacy_files,
        g.modules,
        g.forbid,
        m.core_budget()
    ));
    let crates: Vec<String> = CRATE_NAMES
        .iter()
        .map(|c| {
            let cc = g.crates.get(c).copied().unwrap_or_default();
            format!(
                "\"{c}\": {{\"files\": {}, \"unsafe\": {}}}",
                cc.files, cc.unsafe_n
            )
        })
        .collect();
    out.push_str(&format!("  \"crates\": {{{}}},\n", crates.join(", ")));
    let ratchet = m.ratchet.as_ref().map_or_else(
        || "null".to_string(),
        |r| {
            format!(
                "{{\"forbid_floor\": {}, \"legacy_ceiling\": {}, \"legacy_baseline\": {}, \
                 \"hal_ceiling\": {}, \"baseline_date\": {}}}",
                r.forbid_floor,
                r.legacy_ceiling,
                json_opt(r.legacy_baseline),
                json_opt(r.hal_ceiling),
                r.baseline_date
                    .as_deref()
                    .map_or_else(|| "null".to_string(), json_str)
            )
        },
    );
    out.push_str(&format!("  \"ratchet\": {ratchet},\n"));
    out.push_str(&format!(
        "  \"rate\": {{\"active_days\": {}, \"removed\": {}, \"per_day\": {}}},\n",
        pace.active_days,
        pace.removed,
        pace.per_day
            .map_or_else(|| "null".to_string(), |x| format!("{x:.2}"))
    ));
    let subsystems: Vec<String> = m
        .rows
        .iter()
        .map(|(name, row)| {
            let total = row.kernel.total();
            let core = g.core_rows.get(name).copied().unwrap_or(0);
            format!(
                "    {{\"name\": {}, \"budget\": {}, \"unsafe\": {total}, \"core\": {core}, \
                 \"legacy\": {}}}",
                json_str(name),
                json_opt(m.budget.get(name).copied()),
                total.saturating_sub(core)
            )
        })
        .collect();
    out.push_str("  \"subsystems\": [\n");
    out.push_str(&subsystems.join(",\n"));
    out.push_str("\n  ],\n  \"files\": [\n");
    let files: Vec<String> = m
        .files
        .iter()
        .map(|f| {
            let status = statuses
                .get(&f.rel)
                .map_or_else(|| "null".to_string(), |s| json_str(s));
            format!(
                "    {{\"path\": {}, \"status\": {status}, \"subsystem\": {}, \"group\": \"{}\", \
                 \"module\": {}, \"test\": {}, \"unsafe\": {}, \"static_cell\": {}, \
                 \"send_sync\": {}, \"raw_cell\": {}, \"unsafe_cell\": {}}}",
                json_str(&f.rel),
                json_str(&f.subsystem),
                f.group(),
                f.module,
                f.test,
                f.kernel.total(),
                f.shapes.static_cell,
                f.shapes.send_sync,
                f.shapes.raw_cell,
                f.shapes.unsafe_cell
            )
        })
        .collect();
    out.push_str(&files.join(",\n"));
    out.push_str("\n  ]\n}\n");
    out
}

/// The stable one-line summary recorded in docs/STATUS.md.
pub(crate) fn totals_line(k: &Counts, t: &Counts, g: &Groups) -> String {
    format!(
        "{STATUS_PREFIX} (`cargo xtask unsafe-report`): kernel {} blocks, {} fn, {} impl, {} trait, \
         {} other; tests {} more; core {}, legacy {}, forbid {} of {} modules; crates: {}.",
        k.blocks,
        k.fns,
        k.impls,
        k.traits,
        k.other,
        t.total(),
        g.core,
        g.legacy,
        g.forbid,
        g.modules,
        crates_summary(&g.crates)
    )
}

/// `doc` with its single line starting with [`STATUS_PREFIX`] replaced by `line`.
fn replace_status_line(doc: &str, line: &str) -> Result<String> {
    let hits = doc.lines().filter(|l| l.starts_with(STATUS_PREFIX)).count();
    if hits != 1 {
        return Err(format!(
            "{STATUS_DOC}: expected exactly one line starting with `{STATUS_PREFIX}`, found {hits}"
        )
        .into());
    }
    let mut out = String::with_capacity(doc.len());
    for l in doc.lines() {
        out.push_str(if l.starts_with(STATUS_PREFIX) {
            line
        } else {
            l
        });
        out.push('\n');
    }
    Ok(out)
}

/// `tests.rs`, or a file under a `tests/` directory.
pub(crate) fn is_test_path(p: &Path) -> bool {
    p.file_name().and_then(|n| n.to_str()) == Some("tests.rs")
        || p.components().any(|c| c.as_os_str() == "tests")
}

/// The files `mod name;` in `parent` may load: `name.rs` or `name/mod.rs` next to a
/// `mod.rs`/`lib.rs`/`main.rs`, inside the directory named after the file otherwise.
fn child_module_paths(parent: &Path, name: &str) -> [PathBuf; 2] {
    let dir = parent.parent().unwrap_or(Path::new(""));
    let file = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let base = if matches!(file, "mod.rs" | "lib.rs" | "main.rs") {
        dir.to_path_buf()
    } else {
        dir.join(file.trim_end_matches(".rs"))
    };
    [
        base.join(format!("{name}.rs")),
        base.join(name).join("mod.rs"),
    ]
}

/// The subsystem a kernel file (path relative to the workspace root) is counted under:
///
/// - `sys/arch/<a>/..` → `arch/<a>`, `sys/lib/<l>/..` → `lib/<l>`;
/// - `sys/dev/<d>/..` → `dev/<d>` when `sys/dev/<d>` is a bus or chip directory (it has a
///   `mod.rs`: `pci`, `pv`, `ic`, `isa`, `fdt`, `ofw`, `efi`, later `usb`, ...); every other
///   file of `sys/dev` (softraid, vnd, rd, bio, cons, rnd)
///   → `dev`;
/// - any other `sys/<x>/..` → `x` (`kern`, `uvm`, `net`, `netinet`, `ufs`, `isofs`, ...);
/// - `sys/<file>.rs` → `(crate root)`, `init/..` → `init`.
pub(crate) fn subsystem(root: &Path, rel: &str) -> String {
    if rel.starts_with("init/") {
        return "init".to_string();
    }
    let parts: Vec<&str> = rel.strip_prefix("sys/").unwrap_or(rel).split('/').collect();
    match parts.as_slice() {
        [_] => "(crate root)".to_string(),
        ["arch" | "lib", second, _, ..] => format!("{}/{second}", parts[0]),
        ["dev", d, _, ..] if root.join("sys/dev").join(d).join("mod.rs").is_file() => {
            format!("dev/{d}")
        }
        [first, ..] => (*first).to_string(),
        [] => "(crate root)".to_string(),
    }
}

/// The paths of `unsafe-core.toml`: the only places `unsafe` may live (docs/ZERO_UNSAFE.md,
/// section 4). A pattern is an exact file path or a directory followed by `/**`.
#[derive(Debug)]
pub(crate) struct CoreList {
    patterns: Vec<String>,
}

#[derive(serde::Deserialize)]
struct CoreFile {
    core: CoreSection,
}

#[derive(serde::Deserialize)]
struct CoreSection {
    paths: Vec<String>,
}

impl CoreList {
    /// Read `unsafe-core.toml` at the workspace root.
    pub(crate) fn load(root: &Path) -> Result<Self> {
        let path = root.join(CORE_FILE);
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{CORE_FILE}: {e}").into())
    }

    /// Parse the file's text; every pattern must be `sys/<path>.rs` or `sys/<dir>/**`.
    pub(crate) fn parse(text: &str) -> std::result::Result<Self, String> {
        let f: CoreFile = toml::from_str(text).map_err(|e| e.to_string())?;
        for p in &f.core.paths {
            let body = p.strip_suffix("/**").unwrap_or(p);
            let exact_ok = p.ends_with(".rs") || p.ends_with("/**");
            if !p.starts_with("sys/") || !exact_ok || body.contains(['*', '?', '[']) {
                return Err(format!(
                    "`{p}`: a pattern is `sys/<path>.rs` or `sys/<dir>/**`, nothing else"
                ));
            }
        }
        Ok(CoreList {
            patterns: f.core.paths,
        })
    }

    /// Whether the workspace-relative `rel` lies in the core.
    pub(crate) fn contains(&self, rel: &str) -> bool {
        self.patterns.iter().any(|p| match p.strip_suffix("/**") {
            Some(dir) => rel
                .strip_prefix(dir)
                .is_some_and(|rest| rest.starts_with('/')),
            None => rel == p,
        })
    }

    /// The patterns that name nothing in the tree (a file renamed or gone).
    pub(crate) fn missing(&self, root: &Path) -> Vec<String> {
        self.patterns
            .iter()
            .filter(|p| match p.strip_suffix("/**") {
                Some(dir) => !root.join(dir).is_dir(),
                None => !root.join(p).is_file(),
            })
            .cloned()
            .collect()
    }
}

/// One out-of-line `mod name;` a file declares.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ChildMod {
    /// The files it may load, in the order rustc tries them (`name.rs`, then `name/mod.rs`;
    /// one file with `#[path]`).
    pub(crate) files: Vec<PathBuf>,
    /// Its own attribute, or that of an inline `mod x { .. }` around it, is
    /// `#[forbid(unsafe_code)]`.
    pub(crate) forbid: bool,
    /// Loaded through `#[path]`: the file then counts as a `mod.rs` for its own children.
    pub(crate) via_path: bool,
}

/// What [`mod_scan`] finds in one file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ModScan {
    /// The file opens with `#![forbid(unsafe_code)]`.
    pub(crate) inner_forbid: bool,
    /// Its out-of-line module declarations, inline modules looked through.
    pub(crate) children: Vec<ChildMod>,
}

/// The module declarations of `file` (lexed as `toks`). `mod_rs` says whether its children
/// live next to it (`mod.rs`, `lib.rs`, `main.rs`, a file loaded by `#[path]`) or in the
/// directory named after it.
pub(crate) fn mod_scan(file: &Path, toks: &[Tok], mod_rs: bool) -> ModScan {
    let file_dir = file.parent().unwrap_or(Path::new("")).to_path_buf();
    let base = if mod_rs {
        file_dir.clone()
    } else {
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        file_dir.join(stem)
    };
    let (inner_forbid, start) = inner_attrs(toks, 0, toks.len());
    let mut children = Vec::new();
    let ctx = ModCtx {
        dir: base,
        file_dir: &file_dir,
        top: true,
        forbid: false,
    };
    walk_mods(toks, start, toks.len(), &ctx, &mut children);
    ModScan {
        inner_forbid,
        children,
    }
}

/// Where [`walk_mods`] is: the directory children resolve in, the file's own directory (a
/// top-level `#[path]` is relative to it), and the inline modules' `forbid`.
struct ModCtx<'a> {
    dir: PathBuf,
    file_dir: &'a Path,
    top: bool,
    forbid: bool,
}

/// Collect the `mod` declarations between `from` and `to` (one item level: other items'
/// bodies are skipped), descending into inline `mod x { .. }` blocks.
fn walk_mods(toks: &[Tok], from: usize, to: usize, ctx: &ModCtx, out: &mut Vec<ChildMod>) {
    let mut i = from;
    while i < to {
        match &toks[i] {
            Tok::Punct('{') => i = matching(toks, i).map_or(to, |e| e + 1),
            Tok::Ident(kw) if kw == "mod" => {
                let Some(Tok::Ident(name)) = toks.get(i + 1) else {
                    i += 1;
                    continue;
                };
                let attrs = outer_attrs(toks, from, i);
                let forbid = ctx.forbid || attrs.iter().any(|a| is_forbid_attr(a));
                let path = attrs.iter().find_map(|a| path_attr(a));
                match toks.get(i + 2) {
                    Some(Tok::Punct(';')) => {
                        let files = match path {
                            Some(p) if ctx.top => vec![normalize(&ctx.file_dir.join(p))],
                            Some(p) => vec![normalize(&ctx.dir.join(p))],
                            None => vec![
                                ctx.dir.join(format!("{name}.rs")),
                                ctx.dir.join(name).join("mod.rs"),
                            ],
                        };
                        out.push(ChildMod {
                            files,
                            forbid,
                            via_path: path.is_some(),
                        });
                        i += 3;
                    }
                    Some(Tok::Punct('{')) => {
                        let end = matching(toks, i + 2).unwrap_or(to).min(to);
                        let (inner, start) = inner_attrs(toks, i + 3, end);
                        let sub = ModCtx {
                            dir: normalize(&ctx.dir.join(path.unwrap_or(name))),
                            file_dir: ctx.file_dir,
                            top: false,
                            forbid: forbid || inner,
                        };
                        walk_mods(toks, start, end, &sub, out);
                        i = end + 1;
                    }
                    _ => i += 1,
                }
            }
            _ => i += 1,
        }
    }
}

/// The inner attributes (`#![..]`) at `from`: whether one is `forbid(unsafe_code)`, and the
/// index past them.
fn inner_attrs(toks: &[Tok], from: usize, to: usize) -> (bool, usize) {
    let mut i = from;
    let mut forbid = false;
    while i + 2 < to
        && toks[i] == Tok::Punct('#')
        && toks[i + 1] == Tok::Punct('!')
        && toks[i + 2] == Tok::Punct('[')
    {
        let Some(end) = matching(toks, i + 2) else {
            break;
        };
        forbid |= is_forbid_attr(&toks[i + 3..end]);
        i = end + 1;
    }
    (forbid, i)
}

/// The outer attributes (`#[..]`, the inside of each) of the item whose keyword is at `kw`,
/// looking back past a visibility (`pub`, `pub(crate)`, ...) but not before `from`.
fn outer_attrs(toks: &[Tok], from: usize, kw: usize) -> Vec<&[Tok]> {
    let mut j = kw;
    if j > from && toks[j - 1] == Tok::Punct(')') {
        if let Some(k) = matching_back(toks, j - 1)
            && k > from
            && toks[k - 1] == Tok::Ident("pub".into())
        {
            j = k - 1;
        }
    } else if j > from && toks[j - 1] == Tok::Ident("pub".into()) {
        j -= 1;
    }
    let mut attrs = Vec::new();
    while j > from && toks[j - 1] == Tok::Punct(']') {
        let Some(k) = matching_back(toks, j - 1) else {
            break;
        };
        if k <= from || toks[k - 1] != Tok::Punct('#') || toks[k] != Tok::Punct('[') {
            break;
        }
        attrs.push(&toks[k + 1..j - 1]);
        j = k - 1;
    }
    attrs
}

/// The index of the bracket that opens the one that closes at `close`.
fn matching_back(toks: &[Tok], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    for i in (0..=close).rev() {
        match toks[i] {
            Tok::Punct(')' | ']' | '}') => depth += 1,
            Tok::Punct('(' | '[' | '{') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// The inside of an attribute is `forbid(.., unsafe_code, ..)`.
fn is_forbid_attr(attr: &[Tok]) -> bool {
    matches!(attr, [Tok::Ident(f), Tok::Punct('('), .., Tok::Punct(')')] if f == "forbid")
        && attr.contains(&Tok::Ident("unsafe_code".into()))
}

/// The inside of an attribute is `path = ".."`: the path.
fn path_attr(attr: &[Tok]) -> Option<&str> {
    match attr {
        [Tok::Ident(p), Tok::Punct('='), Tok::Str(s)] if p == "path" => Some(s),
        _ => None,
    }
}

/// `p` with its `.` and `..` components folded, as the module paths rustc builds.
fn normalize(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The files of the module tree that are `#[forbid(unsafe_code)]`: from every crate root
/// (`lib.rs`, `main.rs`) down the out-of-line `mod` declarations, a module being `forbid`
/// when its declaration, an inline module around it, its own `#![forbid]` or any ancestor's
/// says so. A file no declaration reaches is not counted.
pub(crate) fn forbid_files(lexed: &BTreeMap<PathBuf, Vec<Tok>>) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut stack: Vec<(PathBuf, bool, bool)> = lexed
        .keys()
        .filter(|f| {
            matches!(
                f.file_name().and_then(|n| n.to_str()),
                Some("lib.rs" | "main.rs")
            )
        })
        .map(|f| (f.clone(), false, true))
        .collect();
    while let Some((file, inherited, mod_rs)) = stack.pop() {
        if !seen.insert(file.clone()) {
            continue;
        }
        let Some(toks) = lexed.get(&file) else {
            continue;
        };
        let scan = mod_scan(&file, toks, mod_rs);
        let forbid = inherited || scan.inner_forbid;
        if forbid {
            out.insert(file.clone());
        }
        for child in scan.children {
            let Some(found) = child.files.iter().find(|c| lexed.contains_key(*c)) else {
                continue;
            };
            let child_mod_rs =
                child.via_path || found.file_name().and_then(|n| n.to_str()) == Some("mod.rs");
            stack.push((found.clone(), forbid || child.forbid, child_mod_rs));
        }
    }
    out
}

/// Counts of the shapes the zero-unsafe plan replaces (docs/ZERO_UNSAFE.md, section 2), in
/// kernel code: measured, never gated.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Shapes {
    /// Uses of `StaticCell`.
    pub(crate) static_cell: usize,
    /// `unsafe impl Send` and `unsafe impl Sync`.
    pub(crate) send_sync: usize,
    /// `Cell<*const T>` and `Cell<*mut T>`.
    pub(crate) raw_cell: usize,
    /// Uses of `UnsafeCell`.
    pub(crate) unsafe_cell: usize,
}

impl Shapes {
    fn merge(&mut self, o: &Shapes) {
        self.static_cell += o.static_cell;
        self.send_sync += o.send_sync;
        self.raw_cell += o.raw_cell;
        self.unsafe_cell += o.unsafe_cell;
    }

    fn is_empty(&self) -> bool {
        *self == Shapes::default()
    }
}

/// The [`Shapes`] of the tokens not marked as test code.
fn shapes(toks: &[Tok], in_test: &[bool]) -> Shapes {
    let ident = |i: usize, s: &str| matches!(toks.get(i), Some(Tok::Ident(x)) if x == s);
    let mut out = Shapes::default();
    for i in 0..toks.len() {
        if in_test[i] {
            continue;
        }
        if ident(i, "StaticCell") {
            out.static_cell += 1;
        } else if ident(i, "UnsafeCell") {
            out.unsafe_cell += 1;
        } else if ident(i, "Cell")
            && toks.get(i + 1) == Some(&Tok::Punct('<'))
            && toks.get(i + 2) == Some(&Tok::Punct('*'))
            && (ident(i + 3, "const") || ident(i + 3, "mut"))
        {
            out.raw_cell += 1;
        } else if ident(i, "unsafe") && ident(i + 1, "impl") && impl_of_send_sync(&toks[i + 2..]) {
            out.send_sync += 1;
        }
    }
    out
}

/// After `impl`: `<..>`? `path::Send for` or `path::Sync for`.
fn impl_of_send_sync(after: &[Tok]) -> bool {
    let mut i = 0;
    if after.first() == Some(&Tok::Punct('<')) {
        let mut depth = 0usize;
        while i < after.len() {
            match after[i] {
                Tok::Punct('<') => depth += 1,
                Tok::Punct('>') => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    let mut last: Option<&str> = None;
    for t in &after[i..] {
        match t {
            Tok::Ident(x) if x == "for" => {
                return matches!(last, Some("Send" | "Sync"));
            }
            Tok::Ident(x) => last = Some(x),
            Tok::Punct(':') => {}
            _ => return false,
        }
    }
    false
}

/// What [`scan`] finds in one file.
pub(crate) struct Scan {
    /// Every `unsafe` keyword: its kind and whether it sits in test code.
    pub(crate) found: Vec<(Kind, bool)>,
    /// The same, with the index of the `unsafe` token: `(token, kind, in test code)`.
    pub(crate) sites: Vec<(usize, Kind, bool)>,
    /// The names of the `mod name;` declarations that are test-only.
    pub(crate) test_mods: Vec<String>,
    /// The shapes to replace, in its kernel code.
    pub(crate) shapes: Shapes,
    /// Per token: it sits in test code (`#[cfg(test)]` items, test files).
    pub(crate) test_mask: Vec<bool>,
}

/// Classify every `unsafe` in `toks` and find the test-only regions; `file_is_test` makes the
/// whole file test code.
pub(crate) fn scan(toks: &[Tok], file_is_test: bool) -> Scan {
    let mut in_test = vec![file_is_test; toks.len()];
    let mut test_mods = Vec::new();
    // `#![cfg(test)]` at the top of a file.
    if let [Tok::Punct('#'), Tok::Punct('!'), Tok::Punct('['), ..] = toks
        && let Some(end) = matching(toks, 2)
        && attr_is_test_cfg(&toks[3..end])
    {
        in_test.iter_mut().for_each(|t| *t = true);
    }
    let mut i = 0;
    while i < toks.len() {
        if toks[i] == Tok::Punct('#')
            && toks.get(i + 1) == Some(&Tok::Punct('['))
            && let Some(end) = matching(toks, i + 1)
        {
            if attr_is_test_cfg(&toks[i + 2..end]) {
                mark_item(toks, end + 1, &mut in_test);
            }
            i = end + 1;
            continue;
        }
        // A `mod name;` inside test code (or in a test file) declares a test file.
        if toks[i] == Tok::Ident("mod".into())
            && in_test[i]
            && let (Some(Tok::Ident(name)), Some(Tok::Punct(';'))) =
                (toks.get(i + 1), toks.get(i + 2))
        {
            test_mods.push(name.clone());
        }
        i += 1;
    }
    let sites: Vec<(usize, Kind, bool)> = toks
        .iter()
        .enumerate()
        .filter(|(_, t)| **t == Tok::Ident("unsafe".into()))
        .map(|(i, _)| (i, classify(&toks[i + 1..]), in_test[i]))
        .collect();
    let found = sites.iter().map(|&(_, k, t)| (k, t)).collect();
    let shapes = shapes(toks, &in_test);
    Scan {
        found,
        sites,
        test_mods,
        shapes,
        test_mask: in_test,
    }
}

/// What the tokens after an `unsafe` keyword make of it.
fn classify(after: &[Tok]) -> Kind {
    let ident = |t: Option<&Tok>, s: &str| matches!(t, Some(Tok::Ident(x)) if x == s);
    match after.first() {
        Some(Tok::Punct('{')) => Kind::Block,
        Some(Tok::Ident(x)) if x == "impl" => Kind::Impl,
        Some(Tok::Ident(x)) if x == "trait" => Kind::Trait,
        Some(Tok::Ident(x)) if x == "auto" && ident(after.get(1), "trait") => Kind::Trait,
        Some(Tok::Ident(x)) if x == "fn" => fn_kind(&after[1..]),
        Some(Tok::Ident(x)) if x == "extern" => {
            // `unsafe extern "C" fn name` / `unsafe extern "C" { .. }`.
            let rest = match after.get(1) {
                Some(Tok::Lit | Tok::Str(_)) => &after[2..],
                _ => &after[1..],
            };
            if ident(rest.first(), "fn") {
                fn_kind(&rest[1..])
            } else {
                Kind::Other
            }
        }
        _ => Kind::Other,
    }
}

/// `fn name` declares a function; `fn(` is a function pointer type.
fn fn_kind(after_fn: &[Tok]) -> Kind {
    match after_fn.first() {
        Some(Tok::Ident(_)) => Kind::Fn,
        _ => Kind::Other,
    }
}

/// The index of the bracket that closes the one at `open` (`(`, `[` or `{`).
pub(crate) fn matching(toks: &[Tok], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, t) in toks.iter().enumerate().skip(open) {
        match t {
            Tok::Punct('(' | '[' | '{') => depth += 1,
            Tok::Punct(')' | ']' | '}') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Mark the item that starts at `start` (after a test-only attribute) as test code: up to
/// its first `;` or through its `{ .. }` body, skipping brackets in its signature. (A
/// `mod name;` item so marked is then found by [`scan`]'s own pass.)
fn mark_item(toks: &[Tok], start: usize, in_test: &mut [bool]) {
    let mut i = start;
    while i < toks.len() {
        match &toks[i] {
            Tok::Punct('(' | '[') => match matching(toks, i) {
                Some(e) => i = e + 1,
                None => return,
            },
            Tok::Punct(';') => {
                in_test[start..=i].iter_mut().for_each(|t| *t = true);
                return;
            }
            Tok::Punct('{') => {
                let end = matching(toks, i).unwrap_or(toks.len() - 1);
                in_test[start..=end].iter_mut().for_each(|t| *t = true);
                return;
            }
            _ => i += 1,
        }
    }
}

/// The inside of `#[ .. ]` is `cfg(P)` with a test-only predicate `P`.
fn attr_is_test_cfg(attr: &[Tok]) -> bool {
    match attr {
        [Tok::Ident(c), Tok::Punct('('), pred @ .., Tok::Punct(')')] if c == "cfg" => {
            pred_is_test(pred)
        }
        _ => false,
    }
}

/// A `cfg` predicate that can only be true in a `cargo test` build.
fn pred_is_test(pred: &[Tok]) -> bool {
    match pred {
        [Tok::Ident(t)] => t == "test",
        [Tok::Ident(op), Tok::Punct('('), args @ .., Tok::Punct(')')] => {
            let args = split_args(args);
            match op.as_str() {
                "all" => args.iter().any(|a| pred_is_test(a)),
                "any" => !args.is_empty() && args.iter().all(|a| pred_is_test(a)),
                _ => false,
            }
        }
        _ => false,
    }
}

/// Split a predicate list at its top-level commas.
fn split_args(toks: &[Tok]) -> Vec<&[Tok]> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut from = 0;
    for (i, t) in toks.iter().enumerate() {
        match t {
            Tok::Punct('(') => depth += 1,
            Tok::Punct(')') => depth = depth.saturating_sub(1),
            Tok::Punct(',') if depth == 0 => {
                out.push(&toks[from..i]);
                from = i + 1;
            }
            _ => {}
        }
    }
    if from < toks.len() {
        out.push(&toks[from..]);
    }
    out
}

/// Tokenize Rust source, dropping comments, literals, lifetimes and whitespace.
pub(crate) fn lex(src: &str) -> Vec<Tok> {
    lex_pos(src).toks
}

/// A comment of a source file: the 1-based lines it starts and ends on, and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Comment {
    pub(crate) first: usize,
    pub(crate) last: usize,
    pub(crate) text: String,
}

/// [`lex`] with positions: the 1-based line of every token (`lines[i]` is `toks[i]`'s) and
/// the comments the token stream drops. The host tools that check comments (`hal-check`'s
/// `// SAFETY:` rule) read these.
#[derive(Debug, Default)]
pub(crate) struct Lexed {
    pub(crate) toks: Vec<Tok>,
    pub(crate) lines: Vec<usize>,
    pub(crate) comments: Vec<Comment>,
}

/// [`lex`], keeping each token's line and the comments.
pub(crate) fn lex_pos(src: &str) -> Lexed {
    let s: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut lines: Vec<usize> = Vec::new();
    let mut comments: Vec<Comment> = Vec::new();
    // `line_of[k]` is the 1-based line of char `k`.
    let mut line_of = Vec::with_capacity(s.len() + 1);
    let mut line = 1usize;
    for &ch in &s {
        line_of.push(line);
        if ch == '\n' {
            line += 1;
        }
    }
    line_of.push(line);
    let mut i = 0;
    let at = |i: usize| s.get(i).copied().unwrap_or('\0');
    while i < s.len() {
        let c = s[i];
        let start = i;
        let before = toks.len();
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && at(i + 1) == '/' {
            while i < s.len() && s[i] != '\n' {
                i += 1;
            }
            comments.push(Comment {
                first: line_of[start],
                last: line_of[start],
                text: s[start..i].iter().collect(),
            });
        } else if c == '/' && at(i + 1) == '*' {
            let mut depth = 0usize;
            while i < s.len() {
                if s[i] == '/' && at(i + 1) == '*' {
                    depth += 1;
                    i += 2;
                } else if s[i] == '*' && at(i + 1) == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let end = i.min(s.len());
            comments.push(Comment {
                first: line_of[start],
                last: line_of[end.saturating_sub(1).max(start)],
                text: s[start..end].iter().collect(),
            });
        } else if c == '"' {
            let from = i + 1;
            i = skip_string(&s, i);
            let to = i.saturating_sub(1).max(from).min(s.len());
            toks.push(Tok::Str(s[from..to].iter().collect()));
        } else if c == '\'' {
            i = skip_quote(&s, i);
            toks.push(Tok::Lit);
        } else if c.is_ascii_digit() {
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                i += 1;
                // A fraction (`1.5`), but not a range (`1..2`) or a method (`1.max(2)`).
                if at(i) == '.' && at(i + 1).is_ascii_digit() {
                    i += 1;
                }
            }
            toks.push(Tok::Lit);
        } else if c.is_alphabetic() || c == '_' {
            let from = i;
            while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                i += 1;
            }
            let word: String = s[from..i].iter().collect();
            let next = at(i);
            match (word.as_str(), next) {
                // Raw strings: r"..", r#".."#, br"..", cr"..".
                ("r" | "br" | "cr", '"' | '#') if raw_string_start(&s, i) => {
                    i = skip_raw_string(&s, i);
                    toks.push(Tok::Lit);
                }
                // Raw identifier r#name.
                ("r", '#') if at(i + 1).is_alphabetic() || at(i + 1) == '_' => {
                    let from = i + 1;
                    i += 1;
                    while i < s.len() && (s[i].is_alphanumeric() || s[i] == '_') {
                        i += 1;
                    }
                    let name: String = s[from..i].iter().collect();
                    toks.push(Tok::Ident(format!("r#{name}")));
                }
                ("b" | "c", '"') => {
                    i = skip_string(&s, i);
                    toks.push(Tok::Lit);
                }
                ("b", '\'') => {
                    i = skip_quote(&s, i);
                    toks.push(Tok::Lit);
                }
                _ => toks.push(Tok::Ident(word)),
            }
        } else {
            toks.push(Tok::Punct(c));
            i += 1;
        }
        if toks.len() > before {
            lines.push(line_of[start]);
        }
    }
    Lexed {
        toks,
        lines,
        comments,
    }
}

/// Past the `".."` string that starts at `i`.
fn skip_string(s: &[char], mut i: usize) -> usize {
    i += 1;
    while i < s.len() {
        match s[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    i
}

/// Past the char literal or lifetime whose `'` is at `i`.
fn skip_quote(s: &[char], i: usize) -> usize {
    let at = |j: usize| s.get(j).copied().unwrap_or('\0');
    if at(i + 1) == '\\' {
        // An escape: '\n', '\'', '\u{1F600}'.
        let mut j = i + 3;
        while j < s.len() && s[j] != '\'' {
            j += 1;
        }
        return j + 1;
    }
    if at(i + 2) == '\'' {
        return i + 3; // 'a'
    }
    // A lifetime or a label: 'a, 'static.
    let mut j = i + 1;
    while j < s.len() && (s[j].is_alphanumeric() || s[j] == '_') {
        j += 1;
    }
    j
}

/// At `i` (just past `r`, `br` or `cr`) starts `#*"`.
fn raw_string_start(s: &[char], mut i: usize) -> bool {
    while s.get(i) == Some(&'#') {
        i += 1;
    }
    s.get(i) == Some(&'"')
}

/// Past the raw string whose hashes (or quote) start at `i`.
fn skip_raw_string(s: &[char], mut i: usize) -> usize {
    let mut hashes = 0;
    while s.get(i) == Some(&'#') {
        hashes += 1;
        i += 1;
    }
    i += 1; // the opening quote
    while i < s.len() {
        if s[i] == '"' && (1..=hashes).all(|k| s.get(i + k) == Some(&'#')) {
            return i + 1 + hashes;
        }
        i += 1;
    }
    i
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(Kind, bool)> {
        scan(&lex(src), false).found
    }

    fn idents(src: &str) -> Vec<String> {
        lex(src)
            .into_iter()
            .filter_map(|t| match t {
                Tok::Ident(s) => Some(s),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn comments_and_strings_hide_unsafe() {
        let src = r####"
        // unsafe { line comment }
        /* unsafe /* nested unsafe */ still unsafe */
        /// doc: unsafe fn
        let a = "unsafe { in a string \" unsafe }";
        let b = r#"raw "unsafe" { }"#;
        let c = br##"raw bytes "# unsafe"##;
        let d = b"unsafe";
        let e = c"unsafe";
        let f = '"';
        let g = b'"';
        let h = '\'';
        let i = '\u{1F600}';
    "####;
        assert!(kinds(src).is_empty());
        assert!(!idents(src).contains(&"unsafe".to_string()));
    }

    #[test]
    fn lifetimes_are_not_char_literals() {
        // Were 'a read as the start of a char literal, `unsafe` would be swallowed.
        let src = "fn f<'a>(x: &'a u8) -> &'a u8 { unsafe { g(x) } } 'outer: loop {}";
        assert_eq!(kinds(src), vec![(Kind::Block, false)]);
    }

    #[test]
    fn raw_identifier_is_not_the_keyword() {
        assert!(kinds("let r#unsafe = 1;").is_empty());
        assert_eq!(idents("r#unsafe"), vec!["r#unsafe".to_string()]);
    }

    #[test]
    fn numbers_and_ranges() {
        let src = "let x = 1.5e3 + 0x1F_u32 as f64; for i in 0..2 { unsafe { f(i) } }";
        assert_eq!(kinds(src), vec![(Kind::Block, false)]);
        assert!(idents(src).contains(&"i".to_string()));
    }

    #[test]
    fn each_kind_is_classified() {
        let src = r#"
        unsafe fn a() {}
        pub(crate) const unsafe fn b() {}
        unsafe extern "C" fn c() {}
        unsafe extern "C" { fn d(); }
        unsafe impl Send for X {}
        unsafe trait T {}
        unsafe auto trait U {}
        #[unsafe(no_mangle)]
        fn e() { unsafe { f() } }
        type P = unsafe fn(u8);
        type Q = unsafe extern "C" fn();
    "#;
        let k: Vec<Kind> = kinds(src).into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            k,
            vec![
                Kind::Fn,
                Kind::Fn,
                Kind::Fn,
                Kind::Other,
                Kind::Impl,
                Kind::Trait,
                Kind::Trait,
                Kind::Other,
                Kind::Block,
                Kind::Other,
                Kind::Other,
            ]
        );
    }

    #[test]
    fn cfg_test_items_count_as_tests() {
        let src = r#"
        unsafe fn kernel() {}
        #[cfg(test)]
        mod tests {
            #[test]
            fn t() { unsafe { x() } }
        }
        #[cfg(test)]
        fn helper(a: [u8; 4]) { unsafe { y() } }
        #[cfg(any(test, feature = "debug"))]
        fn both() { unsafe { z() } }
        #[cfg(all(test, feature = "alloc"))]
        #[allow(dead_code)]
        unsafe impl Sync for W {}
        #[cfg(not(test))]
        fn not_test() { unsafe { w() } }
        #[cfg(test)]
        use std::vec::Vec;
        unsafe impl Send for V {}
    "#;
        assert_eq!(
            kinds(src),
            vec![
                (Kind::Fn, false),
                (Kind::Block, true),
                (Kind::Block, true),
                (Kind::Block, false),
                (Kind::Impl, true),
                (Kind::Block, false),
                (Kind::Impl, false),
            ]
        );
    }

    #[test]
    fn test_mod_declarations_are_found() {
        let src = "#[cfg(test)]\nmod tests;\n#[cfg(test)]\npub(crate) mod reftest;\nmod real;\n";
        let s = scan(&lex(src), false);
        assert_eq!(
            s.test_mods,
            vec!["tests".to_string(), "reftest".to_string()]
        );
        // Inside a test file every `mod x;` is a test file.
        let s = scan(&lex("mod helpers;\nunsafe fn f() {}"), true);
        assert_eq!(s.test_mods, vec!["helpers".to_string()]);
        assert_eq!(s.found, vec![(Kind::Fn, true)]);
    }

    #[test]
    fn inner_cfg_test_marks_the_file() {
        let s = scan(&lex("#![cfg(test)]\nunsafe fn f() {}"), false);
        assert_eq!(s.found, vec![(Kind::Fn, true)]);
    }

    #[test]
    fn child_paths() {
        let [a, b] = child_module_paths(Path::new("sys/kern/mod.rs"), "tty");
        assert_eq!(a, Path::new("sys/kern/tty.rs"));
        assert_eq!(b, Path::new("sys/kern/tty/mod.rs"));
        let [a, _] = child_module_paths(Path::new("sys/kern/tty.rs"), "tests");
        assert_eq!(a, Path::new("sys/kern/tty/tests.rs"));
    }

    #[test]
    fn subsystems() {
        let root = Path::new("/nonexistent");
        assert_eq!(subsystem(root, "sys/kern/tty.rs"), "kern");
        assert_eq!(
            subsystem(root, "sys/arch/amd64/amd64/pmap.rs"),
            "arch/amd64"
        );
        assert_eq!(subsystem(root, "sys/lib/libkern/strlcpy.rs"), "lib/libkern");
        assert_eq!(subsystem(root, "sys/ufs/ffs/ffs_alloc.rs"), "ufs");
        assert_eq!(subsystem(root, "sys/dev/softraid.rs"), "dev");
        assert_eq!(subsystem(root, "sys/lib.rs"), "(crate root)");
        assert_eq!(subsystem(root, "init/src/main.rs"), "init");
    }

    #[test]
    fn status_line_is_replaced() {
        let doc = "# Status\n\nUnsafe old totals.\nNext:\n";
        let new = replace_status_line(doc, "Unsafe new.").unwrap();
        assert_eq!(new, "# Status\n\nUnsafe new.\nNext:\n");
        assert!(replace_status_line("# Status\n", "Unsafe x").is_err());
    }
    #[test]
    fn plain_strings_keep_their_text() {
        assert_eq!(
            lex(r#"#[path = "../amd64"] mod a;"#)[4],
            Tok::Str("../amd64".to_string())
        );
        // `unsafe extern "C" fn` is still a function.
        assert_eq!(
            kinds(r#"unsafe extern "C" fn f() {}"#),
            vec![(Kind::Fn, false)]
        );
    }

    /// The children of a file, as (first candidate, forbid, via_path).
    fn children(file: &str, src: &str, mod_rs: bool) -> (bool, Vec<(String, bool, bool)>) {
        let s = mod_scan(Path::new(file), &lex(src), mod_rs);
        let c = s
            .children
            .into_iter()
            .map(|c| {
                (
                    c.files[0].to_string_lossy().into_owned(),
                    c.forbid,
                    c.via_path,
                )
            })
            .collect();
        (s.inner_forbid, c)
    }

    #[test]
    fn forbid_attributes_on_mod_declarations() {
        let src = r#"
        //! docs
        pub mod plain;
        #[forbid(unsafe_code)]
        pub mod direct;
        #[cfg(feature = "msdosfs")]
        #[forbid(unsafe_code)]
        pub(crate) mod after_cfg;
        #[forbid(unsafe_code)]
        #[cfg(any(feature = "nfsclient", feature = "nfsserver"))]
        pub mod before_cfg;
        #[deny(unsafe_code)]
        mod denied;
        #[forbid(missing_docs)]
        mod other_lint;
        #[forbid(missing_docs, unsafe_code)]
        mod two_lints;
        fn body() { let x = { 1 }; }
        impl X { fn y() {} }
        mod last;
    "#;
        let (inner, c) = children("sys/kern/mod.rs", src, true);
        assert!(!inner);
        let want = [
            ("sys/kern/plain.rs", false),
            ("sys/kern/direct.rs", true),
            ("sys/kern/after_cfg.rs", true),
            ("sys/kern/before_cfg.rs", true),
            ("sys/kern/denied.rs", false),
            ("sys/kern/other_lint.rs", false),
            ("sys/kern/two_lints.rs", true),
            ("sys/kern/last.rs", false),
        ];
        let got: Vec<(&str, bool)> = c.iter().map(|(f, b, _)| (f.as_str(), *b)).collect();
        assert_eq!(got, want);
        // `name/mod.rs` is the second candidate.
        let s = mod_scan(Path::new("sys/kern/mod.rs"), &lex("mod sub;"), true);
        assert_eq!(s.children[0].files[1], Path::new("sys/kern/sub/mod.rs"));
    }

    #[test]
    fn inline_modules_pass_forbid_and_their_directory_down() {
        let src = r#"
        #[forbid(unsafe_code)]
        pub mod outer {
            pub mod deep;
            mod inner { mod deeper; }
        }
        mod open {
            #![forbid(unsafe_code)]
            mod a;
        }
        mod plain { mod b; }
        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn t() { mod not_a_child; }
        }
    "#;
        let (_, c) = children("sys/dev/usb/uhub.rs", src, false);
        let got: Vec<(&str, bool)> = c.iter().map(|(f, b, _)| (f.as_str(), *b)).collect();
        assert_eq!(
            got,
            [
                ("sys/dev/usb/uhub/outer/deep.rs", true),
                ("sys/dev/usb/uhub/outer/inner/deeper.rs", true),
                ("sys/dev/usb/uhub/open/a.rs", true),
                ("sys/dev/usb/uhub/plain/b.rs", false),
            ]
        );
    }

    #[test]
    fn inner_forbid_and_path_attributes() {
        let (inner, _) = children(
            "sys/x.rs",
            "#![forbid(unsafe_code)]\n#![no_std]\nfn f() {}",
            false,
        );
        assert!(inner);
        let (inner, _) = children("sys/x.rs", "#![no_std]\n#![forbid(unsafe_code)]", false);
        assert!(inner);
        let src = r#"
        #[path = "alloc.rs"]
        mod sa_alloc;
        #[cfg(test)]
        #[path = "../amd64"]
        mod amd64_bios {
            pub mod include { pub mod biosvar; }
        }
    "#;
        let (_, c) = children("sys/arch/host/mod.rs", src, true);
        assert_eq!(
            c,
            [
                ("sys/arch/host/alloc.rs".to_string(), false, true),
                (
                    "sys/arch/amd64/include/biosvar.rs".to_string(),
                    false,
                    false
                ),
            ]
        );
    }

    #[test]
    fn forbid_is_inherited_down_the_tree() {
        let files = [
            (
                "/w/sys/lib.rs",
                "pub mod kern;\n#[forbid(unsafe_code)]\npub mod crypto;",
            ),
            (
                "/w/sys/kern/mod.rs",
                "pub mod a;\n#[forbid(unsafe_code)] pub mod b;",
            ),
            ("/w/sys/kern/a.rs", "mod nested;"),
            ("/w/sys/kern/a/nested.rs", ""),
            ("/w/sys/kern/b.rs", "mod child;"),
            ("/w/sys/kern/b/child.rs", ""),
            ("/w/sys/crypto/mod.rs", "pub mod sha2;"),
            ("/w/sys/crypto/sha2.rs", ""),
            ("/w/sys/orphan.rs", ""),
            (
                "/w/sys/lib/libz/lib.rs",
                "#![forbid(unsafe_code)]\nmod zutil;",
            ),
            ("/w/sys/lib/libz/zutil.rs", ""),
        ];
        let lexed: BTreeMap<PathBuf, Vec<Tok>> = files
            .iter()
            .map(|(p, s)| (PathBuf::from(p), lex(s)))
            .collect();
        let got: Vec<String> = forbid_files(&lexed)
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            got,
            [
                "/w/sys/crypto/mod.rs",
                "/w/sys/crypto/sha2.rs",
                // PathBuf orders by components: `b/child.rs` before `b.rs`.
                "/w/sys/kern/b/child.rs",
                "/w/sys/kern/b.rs",
                "/w/sys/lib/libz/lib.rs",
                "/w/sys/lib/libz/zutil.rs",
            ]
        );
    }

    #[test]
    fn core_patterns() {
        let core =
            CoreList::parse("[core]\npaths = [\"sys/arch/amd64/**\", \"sys/sys/queue.rs\"]\n")
                .unwrap();
        assert!(core.contains("sys/arch/amd64/amd64/pmap.rs"));
        assert!(core.contains("sys/sys/queue.rs"));
        assert!(!core.contains("sys/arch/amd64x/a.rs"));
        assert!(!core.contains("sys/arch/amd64"));
        assert!(!core.contains("sys/sys/queue.rs.bak"));
        assert!(!core.contains("sys/sys/tree.rs"));
        for bad in [
            "sys/kern/*.rs",
            "sys/kern",
            "kern/a.rs",
            "sys/**/x.rs",
            "sys/a?.rs",
        ] {
            let text = format!("[core]\npaths = [\"{bad}\"]\n");
            assert!(CoreList::parse(&text).is_err(), "{bad}");
        }
        assert!(CoreList::parse("[core]\n").is_err());
    }

    #[test]
    fn ratchet_moves_one_way() {
        let r = Ratchet {
            forbid_floor: 400,
            legacy_ceiling: 8000,
            legacy_baseline: Some(8748),
            hal_ceiling: None,
            baseline_date: Some("2026-10-10".to_string()),
        };
        assert!(ratchet_errors(&r, 400, 8000).is_empty());
        assert!(ratchet_errors(&r, 401, 7999).is_empty());
        assert_eq!(ratchet_errors(&r, 399, 8000).len(), 1);
        assert_eq!(ratchet_errors(&r, 400, 8001).len(), 1);
        assert_eq!(ratchet_errors(&r, 0, 9000).len(), 2);
        // --write raises the floor and lowers the ceiling, never the other way, and keeps the
        // baseline as it is.
        assert_eq!(
            ratchet_next(Some(&r), 420, 7900, 0),
            Ratchet {
                forbid_floor: 420,
                legacy_ceiling: 7900,
                ..r.clone()
            }
        );
        assert_eq!(ratchet_next(Some(&r), 10, 9999, 0), r);
        assert_eq!(
            ratchet_next(None, 7, 70, 0),
            Ratchet {
                forbid_floor: 7,
                legacy_ceiling: 70,
                legacy_baseline: None,
                hal_ceiling: None,
                baseline_date: None,
            }
        );
    }

    #[test]
    fn ratchet_is_read_from_the_budget() {
        let b: Budget = toml::from_str(
            "[ratchet]\nforbid_floor = 3\nlegacy_ceiling = 9\n\n[[subsystem]]\nname = \"kern\"\ntotal = 5\n",
        )
        .unwrap();
        assert_eq!(
            b.ratchet,
            Some(Ratchet {
                forbid_floor: 3,
                legacy_ceiling: 9,
                legacy_baseline: None,
                hal_ceiling: None,
                baseline_date: None,
            })
        );
        assert_eq!(b.subsystems.len(), 1);
        let b: Budget = toml::from_str("[[subsystem]]\nname = \"kern\"\ntotal = 5\n").unwrap();
        assert_eq!(b.ratchet, None);
        let b: Budget = toml::from_str(
            "[ratchet]\nforbid_floor = 3\nlegacy_ceiling = 9\nlegacy_baseline = 12\nbaseline_date = \"2026-10-10\"\n",
        )
        .unwrap();
        let r = b.ratchet.unwrap();
        assert_eq!(r.legacy_baseline, Some(12));
        assert_eq!(r.baseline_date.as_deref(), Some("2026-10-10"));
        assert!(is_iso_date("2026-10-10"));
        for bad in [
            "2026-1-10",
            "2026/10/10",
            "20261010",
            "2026-10-1x",
            "2026-10-100",
            "",
        ] {
            assert!(!is_iso_date(bad), "{bad}");
        }
    }

    #[test]
    fn write_keeps_the_baseline() {
        let old = Ratchet {
            forbid_floor: 305,
            legacy_ceiling: 8748,
            legacy_baseline: Some(8748),
            hal_ceiling: None,
            baseline_date: Some("2026-10-10".to_string()),
        };
        // A later --write: more forbid modules, less legacy; the baseline does not move.
        let next = ratchet_next(Some(&old), 320, 8700, 0);
        let rows: BTreeMap<String, usize> =
            [("kern".to_string(), 1100), ("uvm".to_string(), 215)].into();
        let text = budget_text(&next, &rows);
        assert!(text.starts_with("# Per-subsystem unsafe budget"));
        assert!(text.contains(
            "\n[ratchet]\nforbid_floor = 320\nlegacy_ceiling = 8700\nlegacy_baseline = 8748\nbaseline_date = \"2026-10-10\"\n"
        ));
        let b: Budget = toml::from_str(&text).unwrap();
        assert_eq!(
            b.ratchet,
            Some(Ratchet {
                forbid_floor: 320,
                legacy_ceiling: 8700,
                ..old
            })
        );
        assert_eq!(b.subsystems.len(), 2);
        // With no baseline recorded, --write invents none.
        let bare = budget_text(&ratchet_next(None, 1, 2, 0), &rows);
        assert!(!bare.contains("legacy_baseline =") && !bare.contains("baseline_date ="));
        assert!(toml::from_str::<Budget>(&bare).is_ok());
    }

    #[test]
    fn shapes_are_counted_outside_tests() {
        let src = r#"
        static A: StaticCell<u32> = StaticCell::new(0);
        struct S { a: Cell<*const u8>, b: Cell<*mut u8>, c: Cell<Option<NonNull<u8>>>, d: UnsafeCell<u8> }
        unsafe impl Send for S {}
        unsafe impl<T: Sync> core::marker::Sync for W<T> {}
        unsafe impl Adapter for X {}
        #[cfg(test)]
        mod tests { static B: StaticCell<u8> = StaticCell::new(0); unsafe impl Send for T {} }
    "#;
        let s = scan(&lex(src), false).shapes;
        assert_eq!(
            s,
            Shapes {
                static_cell: 2,
                send_sync: 2,
                raw_cell: 2,
                unsafe_cell: 1
            }
        );
    }

    #[test]
    fn totals_line_names_the_groups() {
        let k = Counts {
            files: 1,
            blocks: 5,
            fns: 4,
            impls: 3,
            traits: 2,
            other: 1,
        };
        let g = Groups {
            core: 10,
            legacy: 5,
            forbid: 400,
            modules: 1165,
            ..Groups::default()
        };
        assert_eq!(
            totals_line(&k, &Counts::default(), &g),
            "Unsafe (`cargo xtask unsafe-report`): kernel 5 blocks, 4 fn, 3 impl, 2 trait, 1 other; tests 0 more; core 10, legacy 5, forbid 400 of 1165 modules; crates: kernel 0, abi 0, hal 0, hal_pending 0, boot 0, bsd 0."
        );
    }

    /// A small tree: a core file, `forbid` modules (declared and inherited), legacy files, a
    /// `mod.rs` with code, a test-only file and the init crate.
    fn small_tree() -> Measure {
        let files = [
            (
                "/w/sys/lib.rs",
                "pub mod kern;\n#[forbid(unsafe_code)]\npub mod crypto;\n#[cfg(test)]\nmod testutil;",
            ),
            (
                "/w/sys/kern/mod.rs",
                "pub mod lock;\npub mod a;\n#[forbid(unsafe_code)]\npub mod b;\nfn f() { unsafe { g() } }",
            ),
            (
                "/w/sys/kern/lock.rs",
                "unsafe impl Send for L {}\nfn f() { unsafe { g() } }",
            ),
            (
                "/w/sys/kern/a.rs",
                "static S: StaticCell<u8> = StaticCell::new(0);\nunsafe fn h() {}",
            ),
            ("/w/sys/kern/b.rs", "fn safe() {}"),
            ("/w/sys/crypto/mod.rs", "pub mod sha2;"),
            ("/w/sys/crypto/sha2.rs", ""),
            ("/w/sys/testutil.rs", "fn t() { unsafe { x() } }"),
            ("/w/init/src/main.rs", "fn main() { unsafe { y() } }"),
        ];
        let lexed: BTreeMap<PathBuf, Vec<Tok>> = files
            .iter()
            .map(|(p, s)| (PathBuf::from(p), lex(s)))
            .collect();
        let core = CoreList::parse("[core]\npaths = [\"sys/kern/lock.rs\"]\n").unwrap();
        let budget: BTreeMap<String, usize> = [("kern".to_string(), 10)].into();
        let ratchet = Ratchet {
            forbid_floor: 2,
            legacy_ceiling: 3,
            legacy_baseline: Some(9),
            hal_ceiling: None,
            baseline_date: Some("2026-10-10".to_string()),
        };
        measure_lexed(Path::new("/w"), &lexed, core, budget, Some(ratchet))
    }

    #[test]
    fn files_are_grouped_core_forbid_legacy() {
        let m = small_tree();
        let got: Vec<(&str, &str, bool, bool, usize)> = m
            .files
            .iter()
            .map(|f| {
                (
                    f.rel.as_str(),
                    f.group(),
                    f.module,
                    f.test,
                    f.kernel.total(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                // A crate root and a mod.rs are not modules the ratchet counts; their unsafe
                // is legacy all the same.
                ("init/src/main.rs", "legacy", false, false, 1),
                ("sys/crypto/mod.rs", "forbid", false, false, 0),
                ("sys/crypto/sha2.rs", "forbid", true, false, 0),
                ("sys/kern/a.rs", "legacy", true, false, 1),
                ("sys/kern/b.rs", "forbid", true, false, 0),
                // The core wins over forbid and holds unsafe without counting as legacy.
                ("sys/kern/lock.rs", "core", true, false, 2),
                ("sys/kern/mod.rs", "legacy", false, false, 1),
                ("sys/lib.rs", "legacy", false, false, 0),
                // A test-only file: its unsafe is test code.
                ("sys/testutil.rs", "legacy", true, true, 0),
            ]
        );
        let g = &m.groups;
        assert_eq!(
            (g.core, g.core_files, g.legacy, g.legacy_files),
            (2, 1, 3, 3)
        );
        assert_eq!((g.forbid, g.modules), (2, 5));
        // kern: budget 10, 4 unsafe of which 2 legacy: 8 left to its core file.
        assert_eq!(m.core_budget(), 8);
        let a = m.files.iter().find(|f| f.rel == "sys/kern/a.rs").unwrap();
        assert_eq!(a.shapes.static_cell, 2);
    }

    #[test]
    fn json_shape() {
        let m = small_tree();
        let statuses: BTreeMap<String, &'static str> = [
            ("sys/kern/a.rs".to_string(), "inherited"),
            ("sys/kern/b.rs".to_string(), "redesigned"),
            ("sys/kern/lock.rs".to_string(), "adapted"),
            ("sys/testutil.rs".to_string(), "extra"),
        ]
        .into();
        let pace = rate(
            m.ratchet.as_ref(),
            m.groups.legacy,
            &["2026-10-11", "2026-10-10", "2026-10-09", "2026-10-11"],
        );
        let file = |path: &str,
                    status: &str,
                    sub: &str,
                    group: &str,
                    module: bool,
                    test: bool,
                    n: usize,
                    sc: usize,
                    ss: usize| {
            format!(
                "    {{\"path\": \"{path}\", \"status\": {status}, \"subsystem\": \"{sub}\", \"group\": \"{group}\", \
                 \"module\": {module}, \"test\": {test}, \"unsafe\": {n}, \"static_cell\": {sc}, \
                 \"send_sync\": {ss}, \"raw_cell\": 0, \"unsafe_cell\": 0}}"
            )
        };
        let files = [
            file(
                "init/src/main.rs",
                "null",
                "init",
                "legacy",
                false,
                false,
                1,
                0,
                0,
            ),
            file(
                "sys/crypto/mod.rs",
                "null",
                "crypto",
                "forbid",
                false,
                false,
                0,
                0,
                0,
            ),
            file(
                "sys/crypto/sha2.rs",
                "null",
                "crypto",
                "forbid",
                true,
                false,
                0,
                0,
                0,
            ),
            file(
                "sys/kern/a.rs",
                "\"inherited\"",
                "kern",
                "legacy",
                true,
                false,
                1,
                2,
                0,
            ),
            file(
                "sys/kern/b.rs",
                "\"redesigned\"",
                "kern",
                "forbid",
                true,
                false,
                0,
                0,
                0,
            ),
            file(
                "sys/kern/lock.rs",
                "\"adapted\"",
                "kern",
                "core",
                true,
                false,
                2,
                0,
                1,
            ),
            file(
                "sys/kern/mod.rs",
                "null",
                "kern",
                "legacy",
                false,
                false,
                1,
                0,
                0,
            ),
            file(
                "sys/lib.rs",
                "null",
                "(crate root)",
                "legacy",
                false,
                false,
                0,
                0,
                0,
            ),
            file(
                "sys/testutil.rs",
                "\"extra\"",
                "(crate root)",
                "legacy",
                true,
                true,
                0,
                0,
                0,
            ),
        ];
        let want = format!(
            "{{\n  \"schema\": 1,\n  \
             \"totals\": {{\"kernel_unsafe\": 5, \"test_unsafe\": 1, \"core_unsafe\": 2, \
             \"core_unsafe_files\": 1, \"legacy_unsafe\": 3, \"legacy_unsafe_files\": 3, \
             \"modules\": 5, \"forbid_modules\": 2, \"core_budget\": 8}},\n  \
             \"crates\": {{\"kernel\": {{\"files\": 0, \"unsafe\": 0}}, \"abi\": {{\"files\": 0, \"unsafe\": 0}}, \
             \"hal\": {{\"files\": 0, \"unsafe\": 0}}, \"hal_pending\": {{\"files\": 0, \"unsafe\": 0}}, \
             \"boot\": {{\"files\": 0, \"unsafe\": 0}}, \"bsd\": {{\"files\": 8, \"unsafe\": 5}}}},\n  \
             \"ratchet\": {{\"forbid_floor\": 2, \"legacy_ceiling\": 3, \"legacy_baseline\": 9, \
             \"hal_ceiling\": null, \"baseline_date\": \"2026-10-10\"}},\n  \
             \"rate\": {{\"active_days\": 2, \"removed\": 6, \"per_day\": 3.00}},\n  \
             \"subsystems\": [\n    \
             {{\"name\": \"(crate root)\", \"budget\": null, \"unsafe\": 0, \"core\": 0, \"legacy\": 0}},\n    \
             {{\"name\": \"crypto\", \"budget\": null, \"unsafe\": 0, \"core\": 0, \"legacy\": 0}},\n    \
             {{\"name\": \"init\", \"budget\": null, \"unsafe\": 1, \"core\": 0, \"legacy\": 1}},\n    \
             {{\"name\": \"kern\", \"budget\": 10, \"unsafe\": 4, \"core\": 2, \"legacy\": 2}}\n  \
             ],\n  \"files\": [\n{}\n  ]\n}}\n",
            files.join(",\n")
        );
        assert_eq!(json(&m, &statuses, &pace), want);
        // Without a ratchet: null, and a rate of nothing.
        let mut bare = small_tree();
        bare.ratchet = None;
        let pace = rate(None, bare.groups.legacy, &["2026-10-10"]);
        let doc = json(&bare, &statuses, &pace);
        assert!(doc.contains("  \"ratchet\": null,\n"));
        assert!(
            doc.contains("  \"rate\": {\"active_days\": 0, \"removed\": 0, \"per_day\": null},\n")
        );
        assert_eq!(json_str("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    }

    #[test]
    fn files_belong_to_one_crate() {
        for (rel, want) in [
            ("sys/kernel/pipe.rs", Some("kernel")),
            ("sys/abi/lib.rs", Some("abi")),
            ("sys/hal/lock.rs", Some("hal")),
            ("sys/stand/boot/main.rs", Some("boot")),
            ("sys/lib/libsa/open.rs", Some("boot")),
            ("sys/arch/amd64/stand/efiboot/x.rs", Some("boot")),
            ("sys/arch/amd64/amd64/pmap.rs", Some("hal_pending")),
            ("sys/machine/mod.rs", Some("hal_pending")),
            ("sys/sys/queue.rs", Some("hal_pending")),
            ("sys/kern/subr_tree.rs", Some("hal_pending")),
            ("sys/sys/errno.rs", Some("bsd")),
            ("sys/kern/kern_sig.rs", Some("bsd")),
            ("sys/lib/libkern/lib.rs", Some("bsd")),
            ("init/src/main.rs", Some("bsd")),
            ("tools/xtask/src/main.rs", None),
        ] {
            assert_eq!(crate_of_file(rel), want, "{rel}");
        }
    }

    #[test]
    fn crates_are_counted_and_checked() {
        let m = small_tree();
        let bsd = m.groups.crates["bsd"];
        assert_eq!((bsd.files, bsd.unsafe_n), (8, 5));
        // lock.rs is core: 2 of the 5 are not bounded by legacy_ceiling.
        assert_eq!(m.groups.bsd_noncore, 3);
        assert!(crates_summary(&m.groups.crates).ends_with("hal 0, hal_pending 0, boot 0, bsd 5"));
        let c = |k, h, b| -> BTreeMap<&'static str, CrateCount> {
            [
                (
                    "kernel",
                    CrateCount {
                        files: 1,
                        unsafe_n: k,
                    },
                ),
                (
                    "hal",
                    CrateCount {
                        files: 1,
                        unsafe_n: h,
                    },
                ),
                (
                    "bsd",
                    CrateCount {
                        files: 1,
                        unsafe_n: b,
                    },
                ),
            ]
            .into()
        };
        let r = Ratchet {
            forbid_floor: 0,
            legacy_ceiling: 10,
            legacy_baseline: None,
            hal_ceiling: Some(5),
            baseline_date: None,
        };
        assert!(crate_errors(Some(&r), &c(0, 5, 10), 10).is_empty());
        assert_eq!(crate_errors(Some(&r), &c(1, 5, 10), 10).len(), 1);
        assert_eq!(crate_errors(Some(&r), &c(0, 6, 10), 10).len(), 1);
        assert_eq!(crate_errors(Some(&r), &c(0, 5, 10), 11).len(), 1);
        let mut abi = c(0, 0, 0);
        abi.insert(
            "abi",
            CrateCount {
                files: 1,
                unsafe_n: 2,
            },
        );
        assert_eq!(crate_errors(Some(&r), &abi, 0).len(), 1);
        // No ratchet or no hal_ceiling: only the compiler's two rules apply here.
        assert!(crate_errors(None, &c(0, 99, 99), 99).is_empty());
    }

    #[test]
    fn crate_rows_have_no_row_budget() {
        for n in ["kernel", "abi", "hal"] {
            assert!(is_crate_row(n));
        }
        for n in ["kern", "arch/amd64", "halo", "(crate root)"] {
            assert!(!is_crate_row(n));
        }
    }

    #[test]
    fn write_lowers_hal_ceiling_and_never_raises_it() {
        let r = Ratchet {
            forbid_floor: 1,
            legacy_ceiling: 9,
            legacy_baseline: None,
            hal_ceiling: Some(2150),
            baseline_date: None,
        };
        assert_eq!(ratchet_next(Some(&r), 1, 9, 100).hal_ceiling, Some(100));
        assert_eq!(ratchet_next(Some(&r), 1, 9, 3000).hal_ceiling, Some(2150));
    }

    #[test]
    fn rate_is_na_until_two_active_days() {
        let r = Ratchet {
            forbid_floor: 0,
            legacy_ceiling: 100,
            legacy_baseline: Some(100),
            hal_ceiling: None,
            baseline_date: Some("2026-10-10".to_string()),
        };
        // The day of the baseline alone (dates before it do not count): n/a.
        assert_eq!(
            rate(Some(&r), 90, &["2026-10-10", "2026-10-10", "2026-10-09"]),
            Rate {
                active_days: 1,
                removed: 10,
                per_day: None
            }
        );
        // Two active days: 10 removed, 5 a day.
        assert_eq!(
            rate(
                Some(&r),
                90,
                &["2026-10-12", "2026-10-10", "2026-10-09", "2026-10-12"]
            ),
            Rate {
                active_days: 2,
                removed: 10,
                per_day: Some(5.0)
            }
        );
        // Nothing removed, or legacy above the baseline (an LZ sync): n/a.
        assert_eq!(
            rate(Some(&r), 100, &["2026-10-10", "2026-10-11"]).per_day,
            None
        );
        assert_eq!(
            rate(Some(&r), 120, &["2026-10-10", "2026-10-11"]),
            Rate {
                active_days: 2,
                removed: 0,
                per_day: None
            }
        );
        // No baseline recorded: nothing to measure from.
        let bare = Ratchet {
            legacy_baseline: None,
            hal_ceiling: None,
            baseline_date: None,
            ..r
        };
        assert_eq!(
            rate(Some(&bare), 90, &["2026-10-10", "2026-10-11"]),
            Rate {
                active_days: 0,
                removed: 0,
                per_day: None
            }
        );
        assert_eq!(rate(None, 90, &[]).per_day, None);
    }
}
/* </TESTS> */
