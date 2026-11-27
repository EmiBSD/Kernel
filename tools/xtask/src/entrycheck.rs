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
//! `cargo xtask entry-check`: the rules of the crate `entry` (decisions 56 and 58,
//! docs/ENTRY.md section 5; the user's lock of 2026-11-27), as token scans of the tree,
//! arm64's; and `hal-check`'s H7 and H8, the other side of the same door. Each finding prints
//! `path:line: message`; with no `sys/entry/` the command fails.
//!
//! - E1 (R4): no `.S` file, no `global_asm!`, no `asm!` under `sys/entry/`.
//! - E2 (R3): no `pub` item under `sys/entry/`; no `.rs` outside it starts a path with
//!   `entry::`, and only the image's binary (`sys/bsd/main.rs`) writes `extern crate entry`;
//!   no `Cargo.toml` under `sys/` but the image's names `entry`, whose own depends on `abi`,
//!   `hal`, `kernel` and `sysdefs` only.
//! - E3 (R4): the root has `#![deny(unsafe_code)]`; exactly one `allow(unsafe_code)` under
//!   `sys/entry/`, on the item `#[unsafe(no_mangle)] extern "C" fn entry_user_trap`, pinned by
//!   `const _: UserTrapFn = entry_user_trap;`; no other `unsafe` token, no `extern` block, no
//!   `export_name`, `link_section`, `link_name`, `naked` or `used`.
//! - E4 (R5): no `static`, no `loop`, `while` or `for`, no cell, atomic or lock type.
//! - E5, L1 (R5, the lock's door): of `abi` only `abi::arch::current::svc`; of `kernel` only
//!   `kernel::trap`'s functions; of `hal` only `Arrival` (and, as method calls, its `irq`,
//!   `fiq` and `serror`), `UserVector`, `UserTrapFn`, `UserExit` and the exception classes;
//!   no other crate but `core` and `sysdefs`.
//! - L2 (the lock): the class match's catch-all arm is `kernel::trap::unknown` and nothing
//!   else, every other pattern is exception classes; the vector match has no catch-all.
//! - L3 (R2): no function pointer type, no `dyn`, no call that registers (`install*`,
//!   `register*`, `set_*`).
//! - H7 (`hal-check`, R1): `Arrival`, `SyncTrap` and `IdRead` are named outside `sys/hal/` only
//!   under `sys/entry/` and in `sys/kernel/trap.rs`; the registration's names (`set_handler`,
//!   `SyscallFn`, `HandlerInfo`, `abi_init`, `TrapHandler`) nowhere under `sys/`.
//! - H8 (`hal-check`, R1 and R3): each `handle_el0_*` of `hal`'s `exception.S` holds exactly one
//!   `bl entry_user_trap`; no `do_el0_sync`, `do_el0_error`, `arm_cpu_irq_el0`,
//!   `arm_cpu_fiq_el0` function and no `do_ast` macro under `sys/hal/`; the symbol appears under
//!   `sys/` only at its definition and as a `bl` operand in `hal`'s `.S` files; the external
//!   symbols `hal`'s assembly references are `entry_user_trap` and `bsd`'s `pmap_setttb`
//!   (until `bsd`'s arm64 glue moves, N7 to N9).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;
use crate::halcheck::Finding;
use crate::unsafereport::{Tok, lex_pos, matching};

/// The crate's directory.
const ENTRY_DIR: &str = "sys/entry/";
/// The image crate's manifest, the one that may depend on `entry`.
const IMAGE_MANIFEST: &str = "sys/bsd/Cargo.toml";
/// The image's binary, the one file outside `entry` that names it.
const IMAGE_MAIN: &str = "sys/bsd/main.rs";
/// The one symbol `hal`'s assembly calls.
const SYMBOL: &str = "entry_user_trap";
/// The symbols above `hal` its assembly may name.
const HAL_EXTERNALS: [&str; 2] = [SYMBOL, "pmap_setttb"];
/// The dependencies `entry` may have.
const ENTRY_DEPS: [&str; 4] = ["abi", "hal", "kernel", "sysdefs"];
/// The paths of `hal` `entry` may name (E5): types and the exception classes (`EXCP_`).
const HAL_PATHS: [&str; 5] = [
    "hal::arch::arm64::usertrap::Arrival",
    "hal::arch::arm64::usertrap::UserVector",
    "hal::arch::arm64::usertrap::UserTrapFn",
    "hal::cpu::usertrap::UserExit",
    "hal::arch::arm64::include::armreg::EXCP_",
];
/// The `Arrival`'s methods `entry` may call (R6's destinations in `hal`).
const HAL_METHODS: [&str; 3] = ["irq", "fiq", "serror"];
/// The one path of `abi` `entry` may name.
const ABI_PATH: &str = "abi::arch::current::svc";
/// The roots a path in `entry` may start with.
const ROOTS: [&str; 10] = [
    "abi", "kernel", "hal", "sysdefs", "core", "crate", "super", "self", "Self", "clippy",
];
/// The lock's table (L2): every exception class the class match names, with the one call its
/// arm makes (and an argument its body must hold, `true` or `false` for an abort's `exe`).
const CLASSES: [(&str, &str, &str); 14] = [
    ("EXCP_UNKNOWN", "kernel::trap::undefined", ""),
    ("EXCP_SVE", "kernel::trap::sve", ""),
    ("EXCP_FP_SIMD", "kernel::trap::fpu", ""),
    ("EXCP_TRAP_FP", "kernel::trap::fpu", ""),
    ("EXCP_BRANCH_TGT", "kernel::trap::bti", ""),
    ("EXCP_MSR", "kernel::trap::msr", ""),
    ("EXCP_FPAC", "kernel::trap::fpac", ""),
    ("EXCP_SVC", "abi::arch::current::svc", "svc"),
    ("EXCP_INSN_ABORT_L", "kernel::trap::udata_abort", "true"),
    ("EXCP_PC_ALIGN", "kernel::trap::pc_align", ""),
    ("EXCP_SP_ALIGN", "kernel::trap::sp_align", ""),
    ("EXCP_DATA_ABORT_L", "kernel::trap::udata_abort", "false"),
    ("EXCP_BRK", "kernel::trap::breakpoint", ""),
    ("EXCP_SOFTSTP_EL0", "kernel::trap::single_step", ""),
];
/// The vector match's arms: each `UserVector` and the call its arm makes.
const VECTORS: [(&str, &str); 4] = [
    ("Sync", "do_el0_sync"),
    ("Irq", "irq"),
    ("Fiq", "fiq"),
    ("SError", "serror"),
];
/// The macros `entry` may invoke: none.
const MACROS: [&str; 0] = [];
/// The paths with a type's name first `entry` may write (a), the vector's variants.
const TYPE_PATHS: [&str; 4] = [
    "UserVector::Sync",
    "UserVector::Irq",
    "UserVector::Fiq",
    "UserVector::SError",
];
/// The trap tokens only `entry` and the kernel's door may name outside `hal` (H7).
const TOKENS: [&str; 3] = ["Arrival", "SyncTrap", "IdRead"];
/// The registration's names, gone (H7, R2).
const GONE: [&str; 5] = [
    "set_handler",
    "SyscallFn",
    "HandlerInfo",
    "abi_init",
    "TrapHandler",
];
/// `hal`'s handlers of a trap from user mode, gone into `entry` and the kernel (H8).
const GONE_HAL: [&str; 5] = [
    "do_el0_sync",
    "do_el0_error",
    "arm_cpu_irq_el0",
    "arm_cpu_fiq_el0",
    "do_ast",
];

/// The files the rules read: path relative to the root, and text.
pub(crate) type Tree = BTreeMap<String, String>;

pub(crate) fn entry_check(root: &Path) -> Result<()> {
    if !root.join(ENTRY_DIR).is_dir() {
        return Err(format!("entry-check: no {ENTRY_DIR} (decision 56)").into());
    }
    let tree = load(root)?;
    let findings = check_entry(&tree);
    for f in &findings {
        println!("error: {f}");
    }
    if !findings.is_empty() {
        return Err(format!("entry-check: {} finding(s)", findings.len()).into());
    }
    let n = tree.keys().filter(|k| k.starts_with(ENTRY_DIR)).count();
    println!(
        "entry-check: {n} files of sys/entry/; E1-E5 and the lock (L1-L3) hold: no assembly, nothing names entry but the image, one exported function and no other unsafe, no state, the matches closed, abi only through svc, kernel only through kernel::trap, hal only through Arrival, UserVector, UserTrapFn, UserExit and the classes"
    );
    Ok(())
}

/// `hal-check`'s H7 and H8 over the tree under `root`.
pub(crate) fn hal_upward(root: &Path) -> Result<Vec<Finding>> {
    Ok(check_hal(&load(root)?))
}

/// The `.rs`, `.S`, `.ld` and `Cargo.toml` files under `root/sys`.
fn load(root: &Path) -> Result<Tree> {
    let mut files = Vec::new();
    walk(&root.join("sys"), &mut files)?;
    let mut tree = Tree::new();
    for f in files {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(&f)
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        tree.insert(rel, text);
    }
    Ok(tree)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for e in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, out)?;
        } else if let Some(name) = p.file_name().and_then(|n| n.to_str())
            && (name.ends_with(".rs")
                || name.ends_with(".S")
                || name.ends_with(".ld")
                || name == "Cargo.toml")
        {
            out.push(p);
        }
    }
    Ok(())
}

fn finding(path: &str, line: usize, message: impl Into<String>) -> Finding {
    Finding {
        path: path.to_string(),
        line,
        message: message.into(),
    }
}

/// E1 to E5 and L1 to L3 over `tree`.
pub(crate) fn check_entry(tree: &Tree) -> Vec<Finding> {
    let mut out = Vec::new();
    let door = door_fns(tree, &mut out);
    let allows: usize = tree
        .iter()
        .filter(|(k, _)| k.starts_with(ENTRY_DIR) && k.ends_with(".rs"))
        .map(|(_, t)| {
            lex_pos(t)
                .toks
                .windows(3)
                .filter(|w| {
                    id(&w[0], "allow") && w[1] == Tok::Punct('(') && id(&w[2], "unsafe_code")
                })
                .count()
        })
        .sum();
    if allows != 1 {
        out.push(finding(
            "sys/entry/lib.rs",
            1,
            format!("E3: {allows} allow(unsafe_code) under sys/entry/; exactly one, on the exported item"),
        ));
    }
    let pinned = tree
        .iter()
        .filter(|(k, _)| k.starts_with(ENTRY_DIR) && k.ends_with(".rs"))
        .any(|(_, t)| {
            let toks = lex_pos(t).toks;
            toks.windows(6).any(|w| {
                id(&w[0], "const")
                    && id(&w[1], "_")
                    && w[2] == Tok::Punct(':')
                    && id(&w[3], "UserTrapFn")
                    && w[4] == Tok::Punct('=')
                    && id(&w[5], SYMBOL)
            })
        });
    if !pinned {
        out.push(finding(
            "sys/entry/lib.rs",
            1,
            format!(
                "E3: no `const _: UserTrapFn = {SYMBOL};` (the signature hal's assembly assumes)"
            ),
        ));
    }
    for (path, text) in tree {
        let in_entry = path.starts_with(ENTRY_DIR);
        if in_entry && path.ends_with(".S") {
            out.push(finding(
                path,
                1,
                "E1: an assembly file in entry (the vectors are hal's)",
            ));
        } else if path.ends_with("Cargo.toml") {
            check_manifest(path, text, &mut out);
        } else if path.ends_with(".rs") {
            if in_entry {
                check_entry_rs(path, text, &door, &mut out);
            } else {
                check_other_rs(path, text, &mut out);
            }
        }
    }
    out
}

fn id(t: &Tok, s: &str) -> bool {
    matches!(t, Tok::Ident(w) if w == s)
}

/// E2: who depends on `entry`, and what `entry` depends on.
fn check_manifest(path: &str, text: &str, out: &mut Vec<Finding>) {
    let mut section = String::new();
    for (i, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.starts_with('[') {
            section = t.trim_matches(|c| c == '[' || c == ']').to_string();
            if section.ends_with("dependencies.entry") && path != IMAGE_MANIFEST {
                out.push(finding(
                    path,
                    i + 1,
                    "E2: only the image crate depends on entry",
                ));
            }
            continue;
        }
        if !section.ends_with("dependencies") || t.starts_with('#') || t.is_empty() {
            continue;
        }
        let key = t.split(['=', '.', ' ']).next().unwrap_or("");
        if path == "sys/entry/Cargo.toml" {
            if !ENTRY_DEPS.contains(&key) {
                out.push(finding(
                    path,
                    i + 1,
                    format!("E2: entry depends on `{key}`; only abi, hal, kernel and sysdefs"),
                ));
            }
        } else if key == "entry" && path != IMAGE_MANIFEST {
            out.push(finding(
                path,
                i + 1,
                "E2: only the image crate depends on entry",
            ));
        }
    }
}

/// E2 outside `entry`: no `entry::` path, no `extern crate entry` but the image's.
fn check_other_rs(path: &str, text: &str, out: &mut Vec<Finding>) {
    let lx = lex_pos(text);
    let toks = &lx.toks;
    let sep = |k: usize| {
        toks.get(k) == Some(&Tok::Punct(':')) && toks.get(k + 1) == Some(&Tok::Punct(':'))
    };
    for (k, t) in toks.iter().enumerate() {
        if !id(t, "entry") {
            continue;
        }
        let path_start = !(k >= 2 && sep(k - 2)) && sep(k + 1);
        let extern_crate = k >= 2 && id(&toks[k - 1], "crate") && id(&toks[k - 2], "extern");
        if path_start || extern_crate && path != IMAGE_MAIN {
            out.push(finding(
                path,
                lx.lines.get(k).copied().unwrap_or(0),
                "E2: entry is named outside sys/entry/ (only the image's binary may, `extern crate entry as _`)",
            ));
        }
    }
}

/// E1 to E5 and the lock on one of `entry`'s `.rs` files.
/// The `pub fn`s of `kernel::trap` (`sys/kernel/trap.rs`), the only kernel functions `entry`
/// may name (b); a `pub use` there, which would widen the door, is a finding.
fn door_fns(tree: &Tree, out: &mut Vec<Finding>) -> BTreeSet<String> {
    let path = "sys/kernel/trap.rs";
    let mut fns = BTreeSet::new();
    let Some(text) = tree.get(path) else {
        return fns;
    };
    let lx = lex_pos(text);
    for (k, w) in lx.toks.windows(3).enumerate() {
        if id(&w[0], "pub")
            && id(&w[1], "fn")
            && let Tok::Ident(name) = &w[2]
        {
            fns.insert(name.clone());
        }
        if id(&w[0], "pub") && id(&w[1], "use") {
            out.push(finding(
                path,
                lx.lines.get(k).copied().unwrap_or(0),
                "E5: a `pub use` in kernel::trap would widen entry's door",
            ));
        }
    }
    fns
}

fn check_entry_rs(path: &str, text: &str, door: &BTreeSet<String>, out: &mut Vec<Finding>) {
    let lx = lex_pos(text);
    let toks = &lx.toks;
    let line = |k: usize| lx.lines.get(k).copied().unwrap_or(0);
    let p = |k: usize, c: char| toks.get(k) == Some(&Tok::Punct(c));
    if path == "sys/entry/lib.rs"
        && !toks.windows(7).any(|w| {
            w[0] == Tok::Punct('#')
                && w[1] == Tok::Punct('!')
                && w[2] == Tok::Punct('[')
                && id(&w[3], "deny")
                && w[4] == Tok::Punct('(')
                && id(&w[5], "unsafe_code")
                && w[6] == Tok::Punct(')')
        })
    {
        out.push(finding(
            path,
            1,
            "E3: the root has no #![deny(unsafe_code)]",
        ));
    }
    for (k, t) in toks.iter().enumerate() {
        let Tok::Ident(w) = t else { continue };
        let msg = match w.as_str() {
            "pub" if !p(k + 1, '(') => {
                Some("E2: a `pub` item in entry (pub(crate) or pub(super) only)".to_string())
            }
            "global_asm" | "asm" if p(k + 1, '!') => {
                Some(format!("E1: {w}! in entry (no assembly)"))
            }
            "unsafe"
                if !(p(k + 1, '(') && id_at(toks, k + 2, "no_mangle") && exported_fn(toks, k)) =>
            {
                Some("E3: `unsafe` in entry but the export's `#[unsafe(no_mangle)]`".to_string())
            }
            "extern"
                if p(k + 1, '{')
                    || matches!(toks.get(k + 1), Some(Tok::Str(_))) && p(k + 2, '{') =>
            {
                Some("E3: an extern block in entry".to_string())
            }
            "export_name" | "link_section" | "link_name" | "naked" | "used"
                if k >= 1 && (p(k - 1, '[') || p(k - 1, '(')) =>
            {
                Some(format!("E3: `{w}` in entry"))
            }
            "allow" if p(k + 1, '(') && id_at(toks, k + 2, "unsafe_code") => {
                let mut j = k + 3;
                while j < toks.len() && !p(j, ']') {
                    j += 1;
                }
                (!exported_fn(toks, j + 1)).then(|| {
                    "E3: allow(unsafe_code) on something else than the exported function"
                        .to_string()
                })
            }
            "as" => Some("E5: `as` in entry (no alias of a token, no cast)".to_string()),
            "macro_use" | "macro_rules" => Some(format!("E5: `{w}` in entry (no macro)")),
            m if p(k + 1, '!')
                && matches!(toks.get(k + 2), Some(Tok::Punct('(' | '[' | '{')))
                && !MACROS.contains(&m)
                && m != "global_asm"
                && m != "asm" =>
            {
                Some(format!("E5: the macro `{w}!` in entry (no macro)"))
            }
            "static" => Some("E4/L3: a `static` in entry (no state)".to_string()),
            "loop" | "while" | "for" => Some(format!("E4: `{w}` in entry (no loop)")),
            "Cell" | "RefCell" | "UnsafeCell" | "OnceStatic" | "Mutexed" | "Rwlocked" => {
                Some(format!("E4: `{w}` in entry (no state)"))
            }
            a if a.starts_with("Atomic") => Some(format!("E4: `{w}` in entry (no state)")),
            "dyn" => Some("L3: `dyn` in entry (no table)".to_string()),
            "fn" if p(k + 1, '(') => Some("L3: a function pointer type in entry".to_string()),
            r if (r.starts_with("install")
                || r.starts_with("register")
                || r.starts_with("set_"))
                && p(k + 1, '(') =>
            {
                Some(format!("L3: `{w}(`: entry registers nothing"))
            }
            m if k >= 1
                && p(k - 1, '.')
                && (p(k + 1, '(') || p(k + 1, ':'))
                && !HAL_METHODS.contains(&m) =>
            {
                Some(format!(
                    "L1: `.{w}(`: entry calls no method but the Arrival's irq, fiq and serror"
                ))
            }
            _ => None,
        };
        if let Some(m) = msg {
            out.push(finding(path, line(k), m));
        }
    }
    for (k, full) in paths(toks) {
        check_path(path, line(k), &full, door, out);
    }
    if path.ends_with("/trap.rs") {
        check_matches(path, toks, &lx.lines, out);
    }
}

fn id_at(toks: &[Tok], k: usize, s: &str) -> bool {
    toks.get(k).is_some_and(|t| id(t, s))
}

/// Whether the next item from `k` on (its attributes skipped) is `extern "C" fn
/// entry_user_trap`.
fn exported_fn(toks: &[Tok], mut k: usize) -> bool {
    while k < toks.len() && !id(&toks[k], "extern") {
        if id(&toks[k], "fn") || matches!(toks[k], Tok::Punct(';' | '{')) {
            return false;
        }
        k += 1;
    }
    matches!(toks.get(k + 1), Some(Tok::Str(s)) if s == "C")
        && id_at(toks, k + 2, "fn")
        && id_at(toks, k + 3, SYMBOL)
}

/// Every path a file names, with the index of its first token: plain `a::b::c` and the leaves
/// of `use` trees (`use a::{b, c::d}` gives `a::b` and `a::c::d`).
fn paths(toks: &[Tok]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let sep = |k: usize| {
        toks.get(k) == Some(&Tok::Punct(':')) && toks.get(k + 1) == Some(&Tok::Punct(':'))
    };
    let mut k = 0;
    while k < toks.len() {
        if id(&toks[k], "use") {
            let mut end = k + 1;
            while end < toks.len() && toks[end] != Tok::Punct(';') {
                end += 1;
            }
            use_tree(toks, k + 1, end, "", &mut out);
            k = end + 1;
            continue;
        }
        if let Tok::Ident(w) = &toks[k]
            && sep(k + 1)
            && !(k >= 2 && sep(k - 2))
        {
            let mut full = w.clone();
            let mut j = k + 1;
            while sep(j) {
                if let Some(Tok::Ident(n)) = toks.get(j + 2) {
                    full.push_str("::");
                    full.push_str(n);
                    j += 3;
                } else {
                    j += 2;
                    break;
                }
            }
            out.push((k, full));
            k = j;
            continue;
        }
        k += 1;
    }
    out
}

/// The leaves of the `use` tree in `toks[from..end]` under `prefix`.
fn use_tree(toks: &[Tok], from: usize, end: usize, prefix: &str, out: &mut Vec<(usize, String)>) {
    let mut cur = prefix.to_string();
    let mut k = from;
    while k < end {
        match &toks[k] {
            Tok::Ident(w) if w == "as" => k += 1,
            Tok::Ident(w) => {
                if !cur.is_empty() {
                    cur.push_str("::");
                }
                cur.push_str(w);
            }
            Tok::Punct('{') => {
                let Some(close) = matching(toks, k) else {
                    return;
                };
                let mut depth = 0;
                let mut start = k + 1;
                for j in k + 1..close {
                    match toks[j] {
                        Tok::Punct('{') => depth += 1,
                        Tok::Punct('}') => depth -= 1,
                        Tok::Punct(',') if depth == 0 => {
                            use_tree(toks, start, j, &cur, out);
                            start = j + 1;
                        }
                        _ => {}
                    }
                }
                use_tree(toks, start, close, &cur, out);
                return;
            }
            _ => {}
        }
        k += 1;
    }
    if cur != prefix {
        out.push((from, cur));
    }
}

/// E5/L1: a path `entry` names.
fn check_path(
    path: &str,
    line: usize,
    full: &str,
    door: &BTreeSet<String>,
    out: &mut Vec<Finding>,
) {
    let root = full.split("::").next().unwrap_or("");
    if !ROOTS.contains(&root) {
        if root.chars().next().is_some_and(char::is_uppercase) && !TYPE_PATHS.contains(&full) {
            out.push(finding(
                path,
                line,
                format!("E5/L1: `{full}`: a call through a type's path; entry calls only kernel::trap's functions, abi's svc and the Arrival's irq, fiq and serror"),
            ));
        } else if root.chars().next().is_some_and(char::is_lowercase) {
            out.push(finding(
                path,
                line,
                format!("E5: `{full}`: entry names abi, kernel, hal, sysdefs and core only"),
            ));
        }
        return;
    }
    let ok = match root {
        "abi" => full == ABI_PATH,
        "kernel" => full
            .strip_prefix("kernel::trap::")
            .is_some_and(|f| door.contains(f)),
        "hal" => {
            HAL_PATHS.iter().any(|p| {
                if p.ends_with('_') {
                    full.starts_with(p) && full.len() > p.len()
                } else {
                    full == *p
                }
            }) || full.starts_with("hal::arch::arm64::usertrap::UserVector::")
                && full.matches("::").count() == 5
        }
        _ => true,
    };
    if !ok {
        out.push(finding(path, line, format!("E5/L1: `{full}`: of abi entry names only {ABI_PATH}, of kernel only kernel::trap's functions, of hal only Arrival, UserVector, UserTrapFn, UserExit and the exception classes")));
    }
}

/// L2: the matches of `trap.rs`: the vector match without a catch-all, the class match's
/// catch-all `kernel::trap::unknown` and its other patterns exception classes.
fn check_matches(path: &str, toks: &[Tok], lines: &[usize], out: &mut Vec<Finding>) {
    let line = |k: usize| lines.get(k).copied().unwrap_or(0);
    let mut seen = 0;
    let mut classes_seen: Vec<&str> = Vec::new();
    let mut vectors_seen: Vec<&str> = Vec::new();
    for m in (0..toks.len()).filter(|&k| id(&toks[k], "match")) {
        seen += 1;
        let Some(open) = (m..toks.len()).find(|&k| toks[k] == Tok::Punct('{')) else {
            continue;
        };
        let Some(close) = matching(toks, open) else {
            continue;
        };
        let class = toks[m..open].iter().any(|t| id(t, "class"));
        let mut k = open + 1;
        while k < close {
            let start = k;
            while k + 1 < close && !(toks[k] == Tok::Punct('=') && toks[k + 1] == Tok::Punct('>')) {
                k += 1;
            }
            let pat = &toks[start..k];
            k += 2;
            let body_start = k;
            let body_end;
            if toks.get(k) == Some(&Tok::Punct('{'))
                && let Some(end) = matching(toks, k)
            {
                body_end = end + 1;
                k = end + 1;
                if toks.get(k) == Some(&Tok::Punct(',')) {
                    k += 1;
                }
            } else {
                let mut depth = 0i32;
                while k < close {
                    match toks[k] {
                        Tok::Punct('(' | '[' | '{') => depth += 1,
                        Tok::Punct(')' | ']' | '}') => depth -= 1,
                        Tok::Punct(',') if depth == 0 => break,
                        _ => {}
                    }
                    k += 1;
                }
                body_end = k.min(close);
                k += 1;
            }
            let body = &toks[body_start..body_end];
            if pat.is_empty() {
                continue;
            }
            let catch_all = catch_all_pat(pat);
            if class {
                if catch_all {
                    let unknown = body.len() >= 7
                        && id(&body[0], "kernel")
                        && id(&body[3], "trap")
                        && id(&body[6], "unknown")
                        && body.iter().filter(|t| matches!(t, Tok::Ident(_))).count() <= 5;
                    if !unknown || !id(&pat[0], "_") {
                        out.push(finding(path, line(start), "L2: the class match's catch-all goes elsewhere than kernel::trap::unknown (trap.c's default)"));
                    }
                } else if pat.iter().all(|t| {
                    matches!(t, Tok::Ident(w) if w.starts_with("EXCP_")) || *t == Tok::Punct('|')
                }) {
                    let callee = first_path(body);
                    for t in pat {
                        let Tok::Ident(c) = t else { continue };
                        match CLASSES.iter().find(|(name, _, _)| name == c) {
                            Some((name, dest, arg)) => {
                                classes_seen.push(name);
                                let arg_ok = arg.is_empty() || body.iter().any(|t| id(t, arg));
                                if callee != *dest || !arg_ok {
                                    out.push(finding(path, line(start), format!("L2: `{c}` goes to `{callee}`; the lock sends it to `{dest}`{}", if arg.is_empty() { String::new() } else { format!(" with `{arg}`") })));
                                }
                            }
                            None => out.push(finding(
                                path,
                                line(start),
                                format!("L2: `{c}` is not a class of the lock's table"),
                            )),
                        }
                    }
                } else {
                    out.push(finding(
                        path,
                        line(start),
                        "L2: a class match arm whose pattern is not exception classes",
                    ));
                }
            } else if catch_all {
                out.push(finding(
                    path,
                    line(start),
                    "L2: the vector match has a catch-all arm (four vectors, four arms)",
                ));
            } else if let [Tok::Ident(ty), _, _, Tok::Ident(v)] = pat
                && ty == "UserVector"
            {
                match VECTORS.iter().find(|(name, _)| name == v) {
                    Some((name, call)) => {
                        vectors_seen.push(name);
                        if !body.iter().any(|t| id(t, call)) {
                            out.push(finding(
                                path,
                                line(start),
                                format!("L2: `UserVector::{v}` does not call `{call}`"),
                            ));
                        }
                    }
                    None => out.push(finding(
                        path,
                        line(start),
                        format!("L2: `UserVector::{v}` is not a vector"),
                    )),
                }
            }
        }
    }
    if seen == 0 {
        out.push(finding(path, 1, "L2: no class match in trap.rs"));
    }
    for (name, _, _) in CLASSES {
        let n = classes_seen.iter().filter(|c| **c == name).count();
        if n != 1 {
            out.push(finding(
                path,
                1,
                format!(
                    "L2: `{name}` has {n} arms in the class match; exactly one (the lock's table)"
                ),
            ));
        }
    }
    for (name, _) in VECTORS {
        let n = vectors_seen.iter().filter(|c| **c == name).count();
        if n != 1 {
            out.push(finding(
                path,
                1,
                format!("L2: `UserVector::{name}` has {n} arms in the vector match; exactly one"),
            ));
        }
    }
}

/// The first path of `body` (`kernel::trap::msr` of `kernel::trap::msr(t, s)`), or its first
/// word.
fn first_path(body: &[Tok]) -> String {
    let mut out = String::new();
    let mut k = 0;
    while k < body.len() {
        match &body[k] {
            Tok::Ident(w) => {
                out.push_str(w);
                if body.get(k + 1) == Some(&Tok::Punct(':'))
                    && body.get(k + 2) == Some(&Tok::Punct(':'))
                {
                    out.push_str("::");
                    k += 3;
                    continue;
                }
                return out;
            }
            _ if out.is_empty() => k += 1,
            _ => return out,
        }
    }
    out
}

/// A catch-all pattern: `_` or a binding.
fn catch_all_pat(pat: &[Tok]) -> bool {
    pat.len() == 1
        && (id(&pat[0], "_")
            || matches!(&pat[0], Tok::Ident(w) if w.chars().next().is_some_and(char::is_lowercase)))
}

/// H7 and H8 over `tree`.
pub(crate) fn check_hal(tree: &Tree) -> Vec<Finding> {
    let mut out = Vec::new();
    let hal_labels: BTreeSet<String> = tree
        .iter()
        .filter(|(k, _)| k.starts_with("sys/hal/") && k.ends_with(".S"))
        .flat_map(|(_, t)| labels(t))
        .collect();
    for (path, text) in tree {
        let in_hal = path.starts_with("sys/hal/");
        if path.ends_with(".rs") {
            let lx = lex_pos(text);
            let toks = &lx.toks;
            for (k, t) in toks.iter().enumerate() {
                let line = lx.lines.get(k).copied().unwrap_or(0);
                match t {
                    Tok::Ident(w) if GONE.contains(&w.as_str()) => {
                        out.push(finding(path, line, format!("H7: `{w}`: the registration of a trap's handler is gone (decision 56)")));
                    }
                    Tok::Ident(w)
                        if !in_hal
                            && TOKENS.contains(&w.as_str())
                            && !path.starts_with(ENTRY_DIR)
                            && path != "sys/kernel/trap.rs" =>
                    {
                        out.push(finding(
                            path,
                            line,
                            format!("H7: `{w}` named outside hal, entry and kernel::trap"),
                        ));
                    }
                    Tok::Ident(w)
                        if in_hal
                            && GONE_HAL.contains(&w.as_str())
                            && k >= 1
                            && id(&toks[k - 1], "fn") =>
                    {
                        out.push(finding(
                            path,
                            line,
                            format!("H8: `fn {w}` in hal (the classification is entry's)"),
                        ));
                    }
                    Tok::Ident(w)
                        if w == SYMBOL
                            && !(path.starts_with(ENTRY_DIR)
                                && k >= 1
                                && id(&toks[k - 1], "fn"))
                            && !(path.starts_with(ENTRY_DIR)
                                && k >= 1
                                && toks[k - 1] == Tok::Punct('=')) =>
                    {
                        out.push(finding(
                            path,
                            line,
                            format!("H8: `{SYMBOL}` named in Rust outside its definition"),
                        ));
                    }
                    Tok::Str(s) if s.contains(SYMBOL) => {
                        out.push(finding(
                            path,
                            line,
                            format!("H8: a string names `{SYMBOL}`"),
                        ));
                    }
                    _ => {}
                }
            }
        } else if path.ends_with(".ld") && text.contains(SYMBOL) {
            out.push(finding(
                path,
                1,
                format!("H8: the linker script names `{SYMBOL}`"),
            ));
        } else if path.ends_with(".S") && in_hal {
            check_hal_s(path, text, &hal_labels, &mut out);
        }
    }
    out
}

/// The labels a `.S` defines (`name:` at a line's start; numeric local labels apart; a macro's
/// label keeps its `\` part).
fn labels(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            let (name, _) = l.trim_start().split_once(':')?;
            let first = name.chars().next()?;
            (!first.is_ascii_digit()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '\\'))
            .then(|| name.to_string())
        })
        .collect()
}

/// The code of a `.S` line, comments apart (`in_comment` carries a `/* .. */` over lines).
fn code_of(line: &str, in_comment: &mut bool) -> String {
    let mut code = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("*/") {
                Some(k) => {
                    *in_comment = false;
                    rest = &rest[k + 2..];
                }
                None => break,
            }
        } else if let Some(k) = rest.find("/*") {
            code.push_str(&rest[..k]);
            *in_comment = true;
            rest = &rest[k + 2..];
        } else {
            code.push_str(rest.split("//").next().unwrap_or(""));
            break;
        }
    }
    code
}

/// H8 on one of `hal`'s `.S` files.
fn check_hal_s(path: &str, text: &str, hal_labels: &BTreeSet<String>, out: &mut Vec<Finding>) {
    let mut in_comment = false;
    let mut handler: Option<(String, usize, usize)> = None;
    let done = |h: Option<(String, usize, usize)>, out: &mut Vec<Finding>| {
        if let Some((name, at, calls)) = h
            && calls != 1
        {
            out.push(finding(
                path,
                at,
                format!("H8: `{name}` holds {calls} `bl {SYMBOL}`; exactly one"),
            ));
        }
    };
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let code = code_of(line, &mut in_comment);
        let t = code.trim();
        if let Some(label) = t.strip_suffix(':')
            && !label.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            done(handler.take(), out);
            if label.starts_with("handle_el0_") {
                handler = Some((label.to_string(), n, 0));
            }
            continue;
        }
        let ws: Vec<&str> = t
            .split(|c: char| {
                !(c.is_ascii_alphanumeric()
                    || c == '_'
                    || c == '.'
                    || c == '\\'
                    || c == '{'
                    || c == '}')
            })
            .filter(|w| !w.is_empty())
            .collect();
        if ws.first() == Some(&".macro") && ws.get(1) == Some(&"do_ast") {
            out.push(finding(
                path,
                n,
                "H8: the macro do_ast in hal (the AST loop is the kernel's)",
            ));
        }
        let op = ws.first().copied().unwrap_or("");
        if ws.contains(&SYMBOL) && op != "bl" {
            out.push(finding(
                path,
                n,
                format!("H8: `{SYMBOL}` is not a `bl` operand here (never adr, adrp, ldr or data)"),
            ));
        }
        if handler.is_some() && (op == "blr" || op == "bl" && ws.get(1) != Some(&SYMBOL)) {
            out.push(finding(
                path,
                n,
                format!("H8: a `{t}` in an EL0 handler: it calls {SYMBOL} and nothing else"),
            ));
        }
        if op == "bl"
            && ws.get(1) == Some(&SYMBOL)
            && let Some(h) = handler.as_mut()
        {
            h.2 += 1;
        }
        let target = match op {
            "b" | "bl" | "b.ne" | "b.eq" | "cbz" | "cbnz" | "tbz" | "tbnz" | "adrp" | "adr" => {
                ws.last().copied()
            }
            "add" if code.contains(":lo12:") => ws.last().copied(),
            _ => None,
        };
        if let Some(s) = target {
            let local = s
                .trim_end_matches(['b', 'f'])
                .chars()
                .all(|c| c.is_ascii_digit());
            let operand = s.starts_with('{') || s.contains('\\') || s.starts_with('#');
            let macro_label = hal_labels.iter().any(|l| {
                l.split_once('\\')
                    .is_some_and(|(p, _)| s.starts_with(p) && s.len() > p.len())
            });
            if !local
                && !operand
                && !macro_label
                && !hal_labels.contains(s)
                && !HAL_EXTERNALS.contains(&s)
            {
                out.push(finding(path, n, format!("H8: hal's assembly names `{s}`, above hal; only {SYMBOL} and pmap_setttb (N7-N9)")));
            }
        }
    }
    done(handler.take(), out);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// A tree that holds every rule: entry's root, its exported function and match, the image,
    /// hal's handler.
    fn good() -> Tree {
        let mut t = Tree::new();
        t.insert(
            "sys/entry/lib.rs".into(),
            "#![no_std]\n#![deny(unsafe_code)]\nmod arch;\n".into(),
        );
        t.insert(
            "sys/entry/Cargo.toml".into(),
            "[package]\nname = \"entry\"\n[dependencies]\nabi = { path = \"../abi\" }\nhal = { path = \"../hal\" }\nkernel = { path = \"../kernel\" }\nsysdefs.workspace = true\n".into(),
        );
        t.insert(
            "sys/entry/arch/arm64/trap.rs".into(),
            "use hal::arch::arm64::include::armreg::{EXCP_BRANCH_TGT, EXCP_BRK, EXCP_DATA_ABORT_L, EXCP_FP_SIMD, EXCP_FPAC, EXCP_INSN_ABORT_L, EXCP_MSR, EXCP_PC_ALIGN, EXCP_SOFTSTP_EL0, EXCP_SP_ALIGN, EXCP_SVC, EXCP_SVE, EXCP_TRAP_FP, EXCP_UNKNOWN};\n\
             use hal::arch::arm64::usertrap::{Arrival, UserTrapFn, UserVector};\n\
             use hal::cpu::usertrap::UserExit;\n\
             #[allow(unsafe_code)]\n#[unsafe(no_mangle)]\n\
             extern \"C\" fn entry_user_trap(a: Arrival, v: UserVector) -> UserExit {\n\
             match v { UserVector::Sync => do_el0_sync(a), UserVector::Irq => { a.irq(); kernel::trap::exit() } UserVector::Fiq => { a.fiq(); kernel::trap::exit() } UserVector::SError => a.serror(), }\n}\n\
             const _: UserTrapFn = entry_user_trap;\n\
             fn do_el0_sync(a: Arrival) -> UserExit {\n let (t, s) = kernel::trap::begin(a);\n\
             match s.class { EXCP_UNKNOWN => kernel::trap::undefined(t, s), EXCP_SVE => kernel::trap::sve(t), EXCP_FP_SIMD | EXCP_TRAP_FP => kernel::trap::fpu(t), EXCP_BRANCH_TGT => kernel::trap::bti(t, s), EXCP_MSR => kernel::trap::msr(t, s), EXCP_FPAC => kernel::trap::fpac(t, s), EXCP_SVC => abi::arch::current::svc(&kernel::trap::svc(t)), EXCP_INSN_ABORT_L => kernel::trap::udata_abort(t, s, true), EXCP_PC_ALIGN => kernel::trap::pc_align(t, s), EXCP_SP_ALIGN => kernel::trap::sp_align(t, s), EXCP_DATA_ABORT_L => kernel::trap::udata_abort(t, s, false), EXCP_BRK => kernel::trap::breakpoint(t, s), EXCP_SOFTSTP_EL0 => kernel::trap::single_step(t, s), _ => kernel::trap::unknown(t, s), }\n\
             kernel::trap::leave()\n}\n"
                .into(),
        );
        t.insert(
            "sys/bsd/Cargo.toml".into(),
            "[dependencies]\nentry = { path = \"../entry\" }\n".into(),
        );
        t.insert(
            "sys/kernel/trap.rs".into(),
            "pub fn begin() {}\npub fn undefined() {}\npub fn sve() {}\npub fn fpu() {}\npub fn bti() {}\npub fn msr() {}\npub fn fpac() {}\npub fn svc() {}\npub fn udata_abort() {}\npub fn pc_align() {}\npub fn sp_align() {}\npub fn breakpoint() {}\npub fn single_step() {}\npub fn unknown() {}\npub fn leave() {}\npub fn exit() {}\nfn hidden() {}\n".into(),
        );
        t.insert(
            "sys/bsd/main.rs".into(),
            "extern crate entry as _;\n".into(),
        );
        t.insert(
            "sys/hal/arch/arm64/exception.S".into(),
            "handle_el0_sync:\n\tmov\tx0, sp\n\tbl\tentry_user_trap\n\tb\thal_user_return\nhal_user_return:\n\tbl\tpmap_setttb\n\tb\t1f\n1:\n".into(),
        );
        t
    }

    fn rules(t: &Tree) -> Vec<String> {
        check_entry(t)
            .into_iter()
            .chain(check_hal(t))
            .map(|f| f.message.split(':').next().unwrap_or("").to_string())
            .collect()
    }

    /// `good()` passes; `edit` makes exactly the finding `rule`.
    fn fails(rule: &str, edit: impl FnOnce(&mut Tree)) {
        let mut t = good();
        edit(&mut t);
        let r = rules(&t);
        assert!(r.iter().any(|x| x == rule), "{rule} not found in {r:?}");
    }

    fn edit_trap(t: &mut Tree, from: &str, to: &str) {
        let s = t.get_mut("sys/entry/arch/arm64/trap.rs").unwrap();
        assert!(s.contains(from), "{from}");
        *s = s.replacen(from, to, 1);
    }

    #[test]
    fn the_good_tree_passes() {
        assert_eq!(rules(&good()), Vec::<String>::new());
    }

    #[test]
    fn e1_no_assembly_in_entry() {
        fails("E1", |t| {
            t.insert("sys/entry/arch/arm64/vectors.S".into(), "nop\n".into());
        });
        fails("E1", |t| {
            edit_trap(
                t,
                "fn do_el0_sync",
                "core::arch::global_asm!(\"nop\");\nfn do_el0_sync",
            )
        });
    }

    #[test]
    fn e2_nothing_names_entry() {
        fails("E2", |t| {
            edit_trap(t, "fn do_el0_sync(a", "pub fn do_el0_sync(a")
        });
        fails("E2", |t| {
            t.insert("sys/kernel/x.rs".into(), "fn f() { entry::x(); }\n".into());
        });
        fails("E2", |t| {
            t.insert(
                "sys/hal/Cargo.toml".into(),
                "[dependencies]\nentry = { path = \"../entry\" }\n".into(),
            );
        });
        fails("E2", |t| {
            let s = t.get_mut("sys/entry/Cargo.toml").unwrap();
            s.push_str("libkern.workspace = true\n");
        });
    }

    #[test]
    fn e3_one_unsafe_the_export() {
        fails("E3", |t| {
            let s = t.get_mut("sys/entry/lib.rs").unwrap();
            *s = s.replace("#![deny(unsafe_code)]", "#![forbid(missing_docs)]");
        });
        fails("E3", |t| {
            edit_trap(t, "const _: UserTrapFn = entry_user_trap;\n", "")
        });
        fails("E3", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a",
                "#[allow(unsafe_code)]\nfn do_el0_sync(a",
            )
        });
        fails("E3", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a: Arrival) -> UserExit {\n",
                "fn do_el0_sync(a: Arrival) -> UserExit {\n unsafe { x() };\n",
            )
        });
        fails("E3", |t| {
            edit_trap(t, "fn do_el0_sync(a", "#[used]\nfn do_el0_sync(a")
        });
    }

    #[test]
    fn e4_no_state_no_loop() {
        fails("E4", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a: Arrival) -> UserExit {\n",
                "fn do_el0_sync(a: Arrival) -> UserExit {\n loop {}\n",
            )
        });
        fails("E4", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a",
                "fn g(c: AtomicU8) {}\nfn do_el0_sync(a",
            )
        });
    }

    #[test]
    fn e5_the_door() {
        fails("E5/L1", |t| {
            edit_trap(t, "kernel::trap::leave()", "kernel::sig::userret()")
        });
        fails("E5/L1", |t| {
            edit_trap(t, "abi::arch::current::svc", "abi::sysent::dispatch")
        });
        fails("E5/L1", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a",
                "fn h() { hal::cpu::intr::intr_disable(); }\nfn do_el0_sync(a",
            )
        });
        fails("E5", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a",
                "fn h() { libkern::x(); }\nfn do_el0_sync(a",
            )
        });
    }

    #[test]
    fn the_lock() {
        fails("L1", |t| edit_trap(t, "a.irq();", "a.begin();"));
        fails("L2", |t| {
            edit_trap(
                t,
                "_ => kernel::trap::unknown(t, s),",
                "_ => kernel::trap::undefined(t, s),",
            )
        });
        fails("L2", |t| {
            edit_trap(t, "UserVector::SError => a.serror(),", "_ => a.serror(),")
        });
        fails("L2", |t| edit_trap(t, "EXCP_BRK => kernel", "x => kernel"));
        fails("L3", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a",
                "fn h(f: fn(u8)) {}\nfn do_el0_sync(a",
            )
        });
        fails("L3", |t| {
            edit_trap(
                t,
                "fn do_el0_sync(a",
                "fn h(f: &dyn Fn()) {}\nfn do_el0_sync(a",
            )
        });
        fails("L3", |t| {
            edit_trap(
                t,
                "kernel::trap::leave()",
                "{ kernel::trap::install(x); kernel::trap::leave() }",
            )
        });
    }

    /// The review's holes: a call through a type's path or an alias, a function of
    /// kernel::trap that is not its own, a macro, the lock's destinations and completeness, an
    /// extra call in an EL0 handler.
    #[test]
    fn the_reviews_holes() {
        // (a)
        fails("E5/L1", |t| {
            edit_trap(t, "kernel::trap::begin(a)", "Arrival::begin(a)")
        });
        fails("E5", |t| {
            edit_trap(
                t,
                "use hal::cpu::usertrap::UserExit;",
                "use hal::cpu::usertrap::UserExit;\nuse hal::arch::arm64::usertrap::Arrival as A;",
            )
        });
        // (b)
        fails("E5/L1", |t| {
            edit_trap(t, "kernel::trap::leave()", "kernel::trap::hidden()")
        });
        fails("E5", |t| {
            let s = t.get_mut("sys/kernel/trap.rs").unwrap();
            s.push_str("pub use crate::sig::sigexit;\n");
        });
        // (c)
        fails("E5", |t| {
            edit_trap(
                t,
                "kernel::trap::leave()",
                "{ boom!(); kernel::trap::leave() }",
            )
        });
        fails("E5", |t| {
            edit_trap(
                t,
                "const _:",
                "#[macro_use]\nextern crate kernel;\nconst _:",
            )
        });
        // (d)
        fails("L2", |t| {
            edit_trap(t, "EXCP_BRK => kernel::trap::breakpoint(t, s), ", "")
        });
        fails("L2", |t| {
            edit_trap(
                t,
                "EXCP_SVC => abi::arch::current::svc(&kernel::trap::svc(t))",
                "EXCP_SVC => kernel::trap::undefined(t, s)",
            )
        });
        fails("L2", |t| {
            edit_trap(
                t,
                "kernel::trap::udata_abort(t, s, true)",
                "kernel::trap::udata_abort(t, s, false)",
            )
        });
        fails("L2", |t| {
            edit_trap(
                t,
                "UserVector::Fiq => { a.fiq(); kernel::trap::exit() } ",
                "",
            )
        });
        // (e)
        fails("H8", |t| {
            let s = t.get_mut("sys/hal/arch/arm64/exception.S").unwrap();
            *s = s.replacen(
                "\tbl\tentry_user_trap\n",
                "\tbl\tentry_user_trap\n\tbl\t{AST}\n",
                1,
            );
        });
    }

    #[test]
    fn h7_the_tokens_and_the_registration() {
        fails("H7", |t| {
            t.insert("sys/kernel/sig/x.rs".into(), "fn f(a: Arrival) {}\n".into());
        });
        fails("H7", |t| {
            t.insert(
                "sys/hal/syscall.rs".into(),
                "pub fn set_handler() {}\n".into(),
            );
        });
    }

    #[test]
    fn h8_one_symbol_upward() {
        let edit_s = |t: &mut Tree, from: &str, to: &str| {
            let s = t.get_mut("sys/hal/arch/arm64/exception.S").unwrap();
            *s = s.replacen(from, to, 1);
        };
        fails("H8", |t| edit_s(t, "\tbl\tentry_user_trap\n", ""));
        fails("H8", |t| {
            edit_s(
                t,
                "\tbl\tentry_user_trap\n",
                "\tadrp\tx1, entry_user_trap\n\tbl\tentry_user_trap\n",
            )
        });
        fails("H8", |t| {
            edit_s(t, "\tbl\tpmap_setttb\n", "\tbl\tkern_foo\n")
        });
        fails("H8", |t| {
            edit_s(
                t,
                "hal_user_return:\n",
                ".macro do_ast\n.endm\nhal_user_return:\n",
            )
        });
        fails("H8", |t| {
            t.insert(
                "sys/hal/arch/arm64/trap.rs".into(),
                "fn do_el0_sync() {}\n".into(),
            );
        });
        fails("H8", |t| {
            t.insert(
                "sys/kernel/x.rs".into(),
                "fn f() { entry_user_trap(); }\n".into(),
            );
        });
        fails("H8", |t| {
            t.insert(
                "sys/bsd/arch/arm64/conf/kernel.ld".into(),
                "EXTERN(entry_user_trap)\n".into(),
            );
        });
    }
}
/* </TESTS> */
