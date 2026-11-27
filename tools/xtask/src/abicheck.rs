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
//! `cargo xtask abi-check`: `sys/abi/` holds no logic (docs/ZERO_UNSAFE.md 0.2, decisions 21
//! and 25). Over every `.rs` under `sys/abi/` (test code apart), in the body of every `fn`:
//!
//! * **R1** no `loop`, `while` or `for`.
//! * **R2** an `if` or `match` only on the identifier `nr` (the syscall number) or on the
//!   conversion's own result. `match nr { .. }` is allowed; an `if` is allowed when its
//!   condition names `nr` and otherwise only constants (ALL_CAPS) and literals; a `match`
//!   whose every arm pattern is `Ok(..)`, `Err(..)`, `Some(..)` or `None` (alternatives with
//!   `|` allowed, no guard, no `_`) is allowed; `if let Ok|Err|Some|None(..) = ..` is allowed.
//!   Anything else is a finding.
//! * **R3** at most one call per function into `kernel::` (a path whose first segment is
//!   the crate, read as R6 reads it: `::kernel` and `r#kernel` too): one kernel function per
//!   system call. And no `use` of `kernel` outside test code, in any form (a path, a group,
//!   a rename): a call through an imported name would not be counted.
//! * **R4** at most 40 lines per function body (the line of `{` to the line of `}`).
//! * **R5** no `static` whose type names `Cell`, `RefCell`, `Atomic*`, `OnceStatic`,
//!   `Mutexed`, `Rwlocked` or `UnsafeCell`: no state in `abi`; plain tables are fine.
//! * **R6** (decision 52, the user, 2026-11-26) `kernel` named only through `kernel::api`
//!   (whole: its group modules and their functions, `kernel::api::syscall_legacy`), the
//!   door of one function per system call; `hal` named only through, of `hal::syscall`, its types (`HandlerInfo`, `Register`, `Rval`, `SysArgs`,
//!   `SysResult`, `SyscallFn`, `ENOSYS`) and `set_handler`, the registration of the ABI's
//!   handler; decision 53 (the user, 2026-11-26, N7.abi2) adds `UserRegs` (the view of the
//!   trapping thread's user registers, valid for the handler's call), `Gpr` (the machine's
//!   registers by name) and (decision 56's E1, U5) `ChildReturn` (a forked child's first
//!   return as data, the constant `abi` defines and never applies itself; `ChildFn` before),
//!   with which `abi` owns the per-arch system call convention (not `enter` or `handler_info`,
//!   which re-enter or read the system call path, and not the module itself, whose alias
//!   would reach them; `UserRegs`' methods are not paths and need no entry). In a path, a `use` or
//!   a `{..}` group; never a glob of either crate's root or of `hal::syscall`, nor the crate
//!   alone (`use kernel as k`, `extern crate hal`), which would carry it past the list. A
//!   crate's name starts a path unless a field's `.` (not a range's `..`) or another
//!   module's `x::` stands before it, `x` an identifier other than a keyword (`self`,
//!   `super`, `crate` and `Self` name modules); `r#hal` is `hal`, `r#kernel` is `kernel`.
//!   Linked into the kernel image, `abi` shares `hal`'s statics, and the rest of `hal`'s and
//!   `kernel`'s public API acts on the kernel's own state (`hal::cpu::set_panicking`, the
//!   kcopy tokens, the kernel's tables). The rule is textual: it trusts what the allowed
//!   modules export (a `pub` item added to `kernel::api`, or a `pub use` added to
//!   `hal::syscall`, widens it without a change under `sys/abi/`), and like R1-R5 it reads only
//!   the files under `sys/abi/` (no `#[path]` module or `include!`d file elsewhere exists).
//!   Decision 54 (the user, 2026-11-26): the ABI edge's constants and `#[repr(C)]` structures
//!   are the leaf crate `sysdefs` (data only, no `unsafe`, no state), which `abi` names freely
//!   and R6 does not read; `hal::sys` no longer exists, so a path through it is a finding.
//!   Before decision 52 the door was `hal::api`'s `KernelApi` table and R3 also counted the
//!   calls through a `&KernelApi` parameter.
//!
//! A nested `fn` is judged on its own and its tokens are not the outer function's. Each
//! finding prints as `path:line: message`; the command exits non-zero when there is one.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::Result;
use crate::halcheck::Finding;
use crate::unsafereport::{Tok, lex_pos, matching, scan};

const ABI_DIR: &str = "sys/abi";
// No module of `hal` is named whole since decision 54: the constant headers of the ABI edge are
// `sysdefs`, a leaf crate of data that `abi` names freely (R6 has nothing to say of it).
/// The items of `hal::syscall` that `abi` may name (R6): the system call's types, the
/// registration of the ABI's handler and (decision 53) the trap's user registers, `UserRegs`
/// and `Gpr`, with `ChildReturn` (decision 56's E1: a forked child's return as data); its
/// other functions re-enter or read the system call path.
const HAL_SYSCALL_ITEMS: [&str; 11] = [
    "HandlerInfo",
    "Register",
    "Rval",
    "SysArgs",
    "SysResult",
    "SyscallFn",
    "ChildReturn",
    "UserRegs",
    "Gpr",
    "ENOSYS",
    "set_handler",
];
/// The one module of `kernel` that `abi` may name (R6, decision 52): one function per system
/// call.
const KERNEL_MODULE: &str = "api";
/// Rust's keywords (strict, reserved and weak), which the lexer keeps as identifiers: one
/// before `::hal` makes that `hal` the first segment of a path (R6), where an identifier would
/// make it another module's.
const KEYWORDS: [&str; 56] = [
    "as",
    "async",
    "await",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "gen",
    "macro",
    "override",
    "priv",
    "try",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "macro_rules",
    "raw",
    "safe",
    "union",
];
/// The keywords that name a module before `::` (`crate::hal` is this crate's `hal`).
const PATH_KEYWORDS: [&str; 4] = ["self", "super", "crate", "Self"];
/// The most lines of a `fn` body (decision 25).
pub(crate) const MAX_BODY_LINES: usize = 40;

/// `cargo xtask abi-check`.
pub(crate) fn abi_check(root: &Path) -> Result<()> {
    let mut files = Vec::new();
    let dir = root.join(ABI_DIR);
    if dir.is_dir() {
        crate::walk_rs(&dir, &mut files)?;
    }
    files.sort();
    let mut findings = Vec::new();
    for f in &files {
        if crate::unsafereport::is_test_path(f) {
            continue;
        }
        let src = fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        findings.extend(check_source(&rel, &src));
    }
    for f in &findings {
        println!("error: {f}");
    }
    if !findings.is_empty() {
        return Err(format!("abi-check: {} finding(s)", findings.len()).into());
    }
    println!(
        "abi-check: {} files; no loop, branches only on nr or Ok/Err/Some/None, at most one kernel call and {MAX_BODY_LINES} lines per fn, no state, kernel only through kernel::api, hal only through hal::syscall's types and set_handler (sysdefs freely)",
        files.len()
    );
    Ok(())
}

fn id(t: &Tok, s: &str) -> bool {
    matches!(t, Tok::Ident(x) if x == s)
}

fn is_result_word(w: &str) -> bool {
    matches!(w, "Ok" | "Err" | "Some" | "None")
}

/// `ALL_CAPS` (a constant): at least one letter, no lower case.
fn is_const_name(w: &str) -> bool {
    w.chars().any(|c| c.is_ascii_uppercase())
        && w.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// One `fn` with a body: the `fn` keyword, its name, the parameter list and the body braces.
struct Func {
    kw: usize,
    name: String,
    open: usize,
    close: usize,
}

/// Every `fn` with a body outside test code.
fn functions(toks: &[Tok], mask: &[bool]) -> Vec<Func> {
    let mut out = Vec::new();
    for i in 0..toks.len() {
        if !id(&toks[i], "fn") || mask[i] {
            continue;
        }
        let Some(Tok::Ident(name)) = toks.get(i + 1) else {
            continue; // `fn(..)`, a pointer type
        };
        let mut j = i + 2;
        if toks.get(j) == Some(&Tok::Punct('<')) {
            let mut depth = 0i32;
            while j < toks.len() {
                match toks[j] {
                    Tok::Punct('<') => depth += 1,
                    Tok::Punct('>')
                        if !matches!(toks.get(j.wrapping_sub(1)), Some(Tok::Punct('-'))) =>
                    {
                        depth -= 1;
                        if depth == 0 {
                            j += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        if toks.get(j) != Some(&Tok::Punct('(')) {
            continue;
        }
        let Some(pe) = matching(toks, j) else {
            continue;
        };
        let mut k = pe + 1;
        let mut open = None;
        while k < toks.len() {
            match &toks[k] {
                Tok::Punct('(' | '[') => match matching(toks, k) {
                    Some(e) => k = e,
                    None => break,
                },
                Tok::Punct('{') => {
                    open = Some(k);
                    break;
                }
                Tok::Punct(';') => break,
                _ => {}
            }
            k += 1;
        }
        let Some(open) = open else { continue };
        let Some(close) = matching(toks, open) else {
            continue;
        };
        out.push(Func {
            kw: i,
            name: name.clone(),
            open,
            close,
        });
    }
    out
}

/// The token index just past the `{ .. }` block or the expression of a match arm starting at
/// `k`, up to its separating `,` (consumed) or the closing brace at `end`.
fn skip_arm_body(toks: &[Tok], mut k: usize, end: usize) -> usize {
    if toks.get(k) == Some(&Tok::Punct('{')) {
        k = matching(toks, k).map_or(end, |e| e + 1);
        if toks.get(k) == Some(&Tok::Punct(',')) {
            k += 1;
        }
        return k;
    }
    while k < end {
        match &toks[k] {
            Tok::Punct('(' | '[' | '{') => k = matching(toks, k).map_or(end, |e| e + 1),
            Tok::Punct(',') => return k + 1,
            _ => k += 1,
        }
    }
    end
}

/// Why the arms of the `match` whose body braces are `open..close` are not all
/// `Ok|Err|Some|None` patterns, as `(token index, message)` findings; `guards` receives the
/// index of every guard's `if`, so the `if` rule does not read it as a statement.
fn arm_findings(
    toks: &[Tok],
    open: usize,
    close: usize,
    guards: &mut BTreeSet<usize>,
) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut k = open + 1;
    let mut arms = 0usize;
    while k < close {
        // The pattern runs to the `=>` at depth 0.
        let start = k;
        let mut arrow = None;
        while k < close {
            match &toks[k] {
                Tok::Punct('(' | '[' | '{') => k = matching(toks, k).map_or(close, |e| e + 1),
                Tok::Punct('=') if toks.get(k + 1) == Some(&Tok::Punct('>')) => {
                    arrow = Some(k);
                    break;
                }
                _ => k += 1,
            }
        }
        let Some(arrow) = arrow else { break };
        arms += 1;
        // Each alternative of the pattern must start with a result word.
        let mut alt_start = start;
        let mut depth = 0i32;
        for p in start..=arrow {
            let at_end = p == arrow;
            match &toks[p.min(arrow)] {
                Tok::Punct('(' | '[' | '{') if !at_end => depth += 1,
                Tok::Punct(')' | ']' | '}') if !at_end => depth -= 1,
                _ => {}
            }
            let is_bar = !at_end && depth == 0 && toks[p] == Tok::Punct('|');
            if at_end || is_bar {
                let mut s = alt_start;
                if toks.get(s) == Some(&Tok::Punct('|')) {
                    s += 1;
                }
                let ok = matches!(&toks.get(s), Some(Tok::Ident(w)) if is_result_word(w)) && s < p;
                if !ok && s < p.max(s + 1) {
                    out.push((
                        s.min(arrow),
                        "R2: match arm pattern is not Ok(..), Err(..), Some(..) or None"
                            .to_string(),
                    ));
                }
                alt_start = p + 1;
            }
        }
        if let Some(g) = (start..arrow).find(|&p| id(&toks[p], "if")) {
            guards.insert(g);
            out.push((g, "R2: match arm guard branches on a value".to_string()));
        }
        k = skip_arm_body(toks, arrow + 2, close);
    }
    if arms == 0 {
        out.push((
            open,
            "R2: match with no arm patterns the check understands".to_string(),
        ));
    }
    out
}

/// Token index of the `{` that opens the block after the condition or scrutinee starting at
/// `from`, skipping brackets.
fn block_start(toks: &[Tok], from: usize, limit: usize) -> Option<usize> {
    let mut k = from;
    while k < limit {
        match &toks[k] {
            Tok::Punct('(' | '[') => k = matching(toks, k)? + 1,
            Tok::Punct('{') => return Some(k),
            _ => k += 1,
        }
    }
    None
}

/// All rules over one file's text; `rel` is its path for the messages.
pub(crate) fn check_source(rel: &str, src: &str) -> Vec<Finding> {
    let lexed = lex_pos(src);
    let toks = &lexed.toks;
    let mask = scan(toks, false).test_mask;
    let mut out: Vec<Finding> = Vec::new();
    let mut add = |tok: usize, message: String| {
        out.push(Finding {
            path: rel.to_string(),
            line: lexed.lines[tok.min(lexed.lines.len().saturating_sub(1))],
            message,
        });
    };
    let funcs = functions(toks, &mask);
    for f in &funcs {
        // R4.
        let lines = lexed.lines[f.close] - lexed.lines[f.open] + 1;
        if lines > MAX_BODY_LINES {
            add(
                f.kw,
                format!(
                    "R4: fn {} has a body of {lines} lines, at most {MAX_BODY_LINES}",
                    f.name
                ),
            );
        }
        // The body's own tokens: the nested fns' are theirs.
        let nested: Vec<(usize, usize)> = funcs
            .iter()
            .filter(|g| g.kw > f.open && g.close < f.close)
            .map(|g| (g.kw, g.close))
            .collect();
        let own: Vec<usize> = (f.open + 1..f.close)
            .filter(|&i| !nested.iter().any(|&(a, b)| i >= a && i <= b))
            .collect();
        let (mut calls, mut guards) = (0usize, BTreeSet::new());
        for &i in &own {
            match &toks[i] {
                // R1.
                Tok::Ident(w) if matches!(w.as_str(), "loop" | "while") => {
                    add(i, format!("R1: `{w}` in fn {}", f.name));
                }
                Tok::Ident(w) if w == "for" && toks.get(i + 1) != Some(&Tok::Punct('<')) => {
                    add(i, format!("R1: `for` in fn {}", f.name));
                }
                // R2, match.
                Tok::Ident(w) if w == "match" => {
                    let Some(open) = block_start(toks, i + 1, f.close) else {
                        continue;
                    };
                    let scrutinee = &toks[i + 1..open];
                    if scrutinee == [Tok::Ident("nr".into())] {
                        continue;
                    }
                    if let Some(close) = matching(toks, open) {
                        for (at, m) in arm_findings(toks, open, close, &mut guards) {
                            add(at, format!("{m} (fn {})", f.name));
                        }
                    }
                }
                // R3, a path call into the kernel: the path as R6 reads it (`::kernel` and
                // `r#kernel` are `kernel`, `x::kernel` is not).
                t if is_kernel(t) && is_sep(toks, i + 1) && starts_path(toks, i) => {
                    let mut k = i;
                    while matches!(toks.get(k), Some(Tok::Ident(_)))
                        && toks.get(k + 1) == Some(&Tok::Punct(':'))
                        && toks.get(k + 2) == Some(&Tok::Punct(':'))
                    {
                        k += 3;
                    }
                    if matches!(toks.get(k), Some(Tok::Ident(_)))
                        && toks.get(k + 1) == Some(&Tok::Punct('('))
                    {
                        calls += 1;
                    }
                }
                _ => {}
            }
        }
        // R2, if (after the match pass has found the guards).
        for &i in &own {
            if !id(&toks[i], "if") || guards.contains(&i) {
                continue;
            }
            // `else if` is an `if` like any other.
            if toks.get(i + 1).is_some_and(|t| id(t, "let")) {
                if !matches!(toks.get(i + 2), Some(Tok::Ident(w)) if is_result_word(w)) {
                    add(
                        i,
                        format!(
                            "R2: `if let` on a pattern other than Ok/Err/Some/None in fn {}",
                            f.name
                        ),
                    );
                }
                continue;
            }
            let Some(open) = block_start(toks, i + 1, f.close) else {
                continue;
            };
            let cond = &toks[i + 1..open];
            let mut names_nr = false;
            let mut only_consts = true;
            for t in cond {
                match t {
                    Tok::Ident(w) if w == "nr" => names_nr = true,
                    Tok::Ident(w) if is_const_name(w) => {}
                    Tok::Ident(_) => only_consts = false,
                    _ => {}
                }
            }
            if !(names_nr && only_consts) {
                add(
                    i,
                    format!(
                        "R2: `if` on something other than nr or Ok/Err/Some/None in fn {}",
                        f.name
                    ),
                );
            }
        }
        // R3.
        if calls > 1 {
            add(
                f.kw,
                format!(
                    "R3: fn {} makes {calls} calls into kernel, at most one",
                    f.name
                ),
            );
        }
    }
    // R5.
    for i in 0..toks.len() {
        if !id(&toks[i], "static") || mask[i] {
            continue;
        }
        let Some(colon) = (i + 1..toks.len().min(i + 4)).find(|&k| toks[k] == Tok::Punct(':'))
        else {
            continue;
        };
        let mut k = colon + 1;
        while k < toks.len() {
            match &toks[k] {
                Tok::Punct('=' | ';') => break,
                Tok::Ident(w) if is_state_type(w) => {
                    add(
                        i,
                        format!("R5: static with interior mutability ({w}): abi holds no state"),
                    );
                    break;
                }
                _ => k += 1,
            }
        }
    }
    // R6, kernel (and R3's `use`: an imported name would call the kernel uncounted).
    for i in 0..toks.len() {
        if !is_kernel(&toks[i]) || mask[i] || !starts_path(toks, i) {
            continue;
        }
        if in_use(toks, i) {
            add(
                i,
                "R3: a `use` of kernel (each call names its whole kernel::api path, so R3 counts it)"
                    .to_string(),
            );
            continue;
        }
        let mut found = Vec::new();
        if is_sep(toks, i + 1) {
            kernel_tree(toks, i + 3, &mut found);
        } else {
            found.push((
                i,
                "R6: `kernel` named alone (a rename or a re-export carries it past kernel::api)"
                    .to_string(),
            ));
        }
        for (at, m) in found {
            add(at, m);
        }
    }
    // R6, hal.
    for i in 0..toks.len() {
        if !is_hal(&toks[i]) || mask[i] || !starts_path(toks, i) {
            continue;
        }
        let mut found = Vec::new();
        if is_sep(toks, i + 1) {
            hal_tree(toks, i + 3, &mut found);
        } else {
            found.push((
                i,
                "R6: `hal` named alone (a rename or a re-export carries it past the hal paths abi may name)"
                    .to_string(),
            ));
        }
        for (at, m) in found {
            add(at, m);
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

/// `hal`, or `r#hal`, which is the same name.
fn is_hal(t: &Tok) -> bool {
    matches!(t, Tok::Ident(w) if w == "hal" || w == "r#hal")
}

/// `kernel`, or `r#kernel`, which is the same name.
fn is_kernel(t: &Tok) -> bool {
    matches!(t, Tok::Ident(w) if w == "kernel" || w == "r#kernel")
}

/// R6 over the path or use-tree after `kernel::`, starting at `k`: `api` and what is under it.
fn kernel_tree(toks: &[Tok], k: usize, out: &mut Vec<(usize, String)>) {
    match toks.get(k) {
        Some(Tok::Punct('{')) => {
            for j in group_items(toks, k) {
                kernel_tree(toks, j, out);
            }
        }
        Some(Tok::Ident(m)) if m == KERNEL_MODULE => {}
        Some(Tok::Ident(m)) => out.push((
            k,
            format!("R6: `kernel::{m}` is outside kernel::api, the one kernel path abi may name"),
        )),
        _ => out.push((
            k,
            "R6: a glob of kernel's root (abi names only kernel::api)".to_string(),
        )),
    }
}

/// Whether the path that starts at `i` is (part of) a `use` declaration: walking back over
/// the path, a `use` group's braces and its separators reaches `use` before anything that
/// ends or opens another statement or expression.
fn in_use(toks: &[Tok], i: usize) -> bool {
    let mut k = i;
    while k > 0 {
        k -= 1;
        match &toks[k] {
            Tok::Ident(w) if w == "use" => return true,
            Tok::Punct('{') => {
                let group = k >= 1 && (id(&toks[k - 1], "use") || (k >= 2 && is_sep(toks, k - 2)));
                if !group {
                    return false;
                }
            }
            Tok::Punct(';' | '}' | '(' | ')' | '[' | ']' | '=') => return false,
            _ => {}
        }
    }
    false
}

/// Whether `::` stands at `k`.
fn is_sep(toks: &[Tok], k: usize) -> bool {
    toks.get(k) == Some(&Tok::Punct(':')) && toks.get(k + 1) == Some(&Tok::Punct(':'))
}

/// Whether the `hal` at `i` starts a path, so names the crate: not a field (`s.hal`, though
/// `0..hal` is a range) and not another module's item (`x::hal`, `crate::hal`), where a
/// keyword before `::` (`use ::hal`, `return ::hal`) still leaves it a path's first segment.
fn starts_path(toks: &[Tok], i: usize) -> bool {
    if i >= 1 && toks[i - 1] == Tok::Punct('.') {
        return i >= 2 && toks[i - 2] == Tok::Punct('.');
    }
    if i >= 3 && is_sep(toks, i - 2) {
        return match &toks[i - 3] {
            Tok::Ident(w) => KEYWORDS.contains(&w.as_str()) && !PATH_KEYWORDS.contains(&w.as_str()),
            _ => true,
        };
    }
    true
}

/// R6 over the path or use-tree after `hal::`, starting at `k`.
fn hal_tree(toks: &[Tok], k: usize, out: &mut Vec<(usize, String)>) {
    match toks.get(k) {
        Some(Tok::Punct('{')) => {
            for j in group_items(toks, k) {
                hal_tree(toks, j, out);
            }
        }
        Some(Tok::Ident(m)) if m == "syscall" => {
            if !is_sep(toks, k + 1) {
                out.push((
                    k,
                    "R6: `hal::syscall` named as a module (abi may name only its types, HandlerInfo, Register, Rval, SysArgs, SysResult, SyscallFn, ChildReturn, UserRegs, Gpr, ENOSYS, and set_handler)"
                        .to_string(),
                ));
            } else if toks.get(k + 3) == Some(&Tok::Punct('{')) {
                for j in group_items(toks, k + 3) {
                    syscall_item(toks.get(j), j, out);
                }
            } else {
                syscall_item(toks.get(k + 3), k + 3, out);
            }
        }
        Some(Tok::Ident(m)) => out.push((
            k,
            format!(
                "R6: `hal::{m}` is outside the hal paths abi may name (hal::syscall's types and set_handler; the headers are sysdefs')"
            ),
        )),
        _ => out.push((
            k,
            "R6: a glob of hal's root (abi names only hal::syscall's types and set_handler)"
                .to_string(),
        )),
    }
}

/// R6 for an item named in `hal::syscall`: one of [`HAL_SYSCALL_ITEMS`], or a finding.
fn syscall_item(t: Option<&Tok>, at: usize, out: &mut Vec<(usize, String)>) {
    match t {
        Some(Tok::Ident(m)) if HAL_SYSCALL_ITEMS.contains(&m.as_str()) => {}
        Some(Tok::Ident(m)) => out.push((
            at,
            format!(
                "R6: `hal::syscall::{m}` is not one of the items abi may name (HandlerInfo, Register, Rval, SysArgs, SysResult, SyscallFn, ChildReturn, UserRegs, Gpr, ENOSYS, set_handler)"
            ),
        )),
        _ => out.push((
            at,
            "R6: a glob or a nested group of hal::syscall (abi names only its types and set_handler)".to_string(),
        )),
    }
}

/// The first token of each item of the `{..}` group opened at `open`.
fn group_items(toks: &[Tok], open: usize) -> Vec<usize> {
    let mut items = Vec::new();
    let Some(close) = matching(toks, open) else {
        return items;
    };
    let (mut j, mut start) = (open + 1, true);
    while j < close {
        match &toks[j] {
            Tok::Punct('{' | '(' | '[') => {
                if start {
                    items.push(j);
                    start = false;
                }
                j = matching(toks, j).map_or(close, |e| e + 1);
                continue;
            }
            Tok::Punct(',') => start = true,
            _ if start => {
                items.push(j);
                start = false;
            }
            _ => {}
        }
        j += 1;
    }
    items
}

fn is_state_type(w: &str) -> bool {
    matches!(
        w,
        "Cell" | "RefCell" | "OnceStatic" | "Mutexed" | "Rwlocked" | "UnsafeCell"
    ) || (w.starts_with("Atomic") && w.len() > 6)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn msgs(src: &str) -> Vec<String> {
        check_source("sys/abi/x.rs", src)
            .iter()
            .map(|f| format!("{}:{}", f.line, f.message.split(':').next().unwrap_or("")))
            .collect()
    }

    fn clean(src: &str) {
        let f = check_source("sys/abi/x.rs", src);
        assert!(f.is_empty(), "{src}\n{f:?}");
    }

    fn one(src: &str, rule: &str) {
        let f = check_source("sys/abi/x.rs", src);
        assert!(!f.is_empty(), "{src}: no finding");
        assert!(f.iter().all(|x| x.message.starts_with(rule)), "{f:?}");
    }

    #[test]
    fn r1_loops_are_refused() {
        one("fn f() { loop { } }\n", "R1");
        one("fn f() { while x { } }\n", "R1");
        one("fn f(v: &[u8]) { for b in v { } }\n", "R1");
        // A higher-ranked bound is not a loop.
        clean("fn f<F>(g: F) where F: for<'a> Fn(&'a u8) {}\n");
    }

    #[test]
    fn r2_branching_only_on_nr_or_results() {
        clean("fn f(nr: u32) { match nr { 1 => a(), _ => b() } }\n");
        clean("fn f(r: Result<u8, E>) { match r { Ok(v) => v, Err(e) => e } }\n");
        clean("fn f(r: Option<u8>) { match r { Some(_) => 1, None => 0 } }\n");
        clean("fn f(r: R) { match r { Ok(_) | Err(_) => 1, } }\n");
        clean("fn f(r: R) { match r { Ok(v) => { g(v) } Err(e) => { h(e) } } }\n");
        clean("fn f(r: R) { if let Ok(v) = r { g(v) } else if let Some(x) = h() { g(x) } }\n");
        clean("fn f(nr: u32) { if nr == SYS_EXIT { g() } }\n");
        clean("fn f(nr: u32) { if nr < 5 || nr > MAX_SYSCALL { g() } }\n");
        one("fn f(x: u32) { match x { 1 => a(), _ => b() } }\n", "R2");
        one("fn f(x: u32) { if x == 3 { g() } }\n", "R2");
        one("fn f(x: bool) { if x { g() } }\n", "R2");
        one(
            "fn f(nr: u32, y: u32) { if nr == 1 && y == 2 { g() } }\n",
            "R2",
        );
        one("fn f(r: R) { if let Foo(v) = r { g(v) } }\n", "R2");
        one("fn f(r: R) { match r { Ok(_) => 1, _ => 0 } }\n", "R2");
        one(
            "fn f(r: R) { match r { Ok(v) if v > 3 => 1, Err(_) => 0 } }\n",
            "R2",
        );
        one("fn f(r: R) { match r { Ok(_) => 1, Bad => 0 } }\n", "R2");
        // `else if` on a value is a finding too.
        one(
            "fn f(r: R, x: u8) { if let Ok(_) = r { 1 } else if x > 3 { 2 } else { 3 } }\n",
            "R2",
        );
    }

    #[test]
    fn r3_at_most_one_kernel_call() {
        clean("fn f() { kernel::api::file::pipe2(1); }\n");
        clean("fn f() { let x = Wrap(1); kernel::api::open(x); }\n");
        one("fn f() { kernel::api::a(1); kernel::api::b(2); }\n", "R3");
        // The registration in hal, a field read and another crate's path are not calls into
        // the kernel.
        clean("fn f() { let _ = hal::syscall::set_handler(g, I); kernel::api::a(1); }\n");
        clean("fn f(a: &A) { let _ = a.n; other::kernel(); kernel::api::K; }\n");
        // Nested functions are judged on their own.
        clean("fn f() { kernel::api::a(1); fn g() { kernel::api::b(2); } }\n");
        // `::kernel` and `r#kernel` are the crate, as R6 reads them; `x::kernel` is not.
        one(
            "fn f() { ::kernel::api::a(1); ::kernel::api::b(2); }\n",
            "R3",
        );
        one("fn f() { r#kernel::api::a(1); kernel::api::b(2); }\n", "R3");
        one(
            "fn f() -> R { g(); ::kernel::api::a(1); return ::kernel::api::b(2); }\n",
            "R3",
        );
        clean("fn f() { x::kernel::api::a(1); kernel::api::b(2); }\n");
        // A `use` of the kernel would let bare names call it uncounted: refused in every form.
        one(
            "use kernel::api::file::{read, write};\nfn f() { read(1); write(2); }\n",
            "R3",
        );
        one("use kernel::api::file::read;\n", "R3");
        one("use kernel::api::file;\n", "R3");
        one("use kernel::api;\n", "R3");
        one("pub use ::kernel::api::fs::open;\n", "R3");
        one("use r#kernel::api::net as n;\n", "R3");
        one("use {kernel::api::vm::mmap};\n", "R3");
        // Test code may import what it likes.
        clean("#[cfg(test)]\nmod tests {\n    use kernel::api::file::read;\n}\n");
    }

    #[test]
    fn r4_body_length() {
        let body = |n: usize| format!("fn f() {{\n{}}}\n", "    g();\n".repeat(n));
        // The braces' own lines count: n statements make n + 2 lines.
        clean(&body(MAX_BODY_LINES - 2));
        one(&body(MAX_BODY_LINES - 1), "R4");
        assert_eq!(msgs(&body(60))[0], "1:R4");
    }

    #[test]
    fn r5_no_interior_mutability_in_statics() {
        one("static X: Cell<u8> = Cell::new(0);\n", "R5");
        one(
            "static X: core::sync::atomic::AtomicU32 = AtomicU32::new(0);\n",
            "R5",
        );
        one("static X: OnceStatic<u8> = OnceStatic::new();\n", "R5");
        one("static X: Mutexed<[u8; 4]> = Mutexed::new([0; 4]);\n", "R5");
        one("pub static X: Rwlocked<u8> = Rwlocked::new(0);\n", "R5");
        one("static X: RefCell<u8> = RefCell::new(0);\n", "R5");
        one("static X: UnsafeCell<u8> = UnsafeCell::new(0);\n", "R5");
        clean("static TABLE: [u8; 3] = [1, 2, 3];\n");
        clean("static NAMES: &[&str] = &[\"a\"];\nconst C: Cell = x;\n");
        clean("fn f(x: &'static str) {}\n");
    }

    #[test]
    fn r6_hal_only_through_syscall_types() {
        // What the crate names today, and the forms of the same paths.
        clean("use hal::syscall::{HandlerInfo, Register};\n");
        // Decision 53: the trap's user registers, their names and a forked child's return.
        clean("use hal::syscall::{ChildReturn, Gpr, UserRegs};\n");
        clean("fn f(regs: &hal::syscall::UserRegs) { regs.restart(); }\n");
        clean("const X8: hal::syscall::Gpr = hal::syscall::Gpr::X8;\n");
        clean("fn f() { let _ = hal::syscall::set_handler(a, I); }\n");
        // What hal's trap calls, and what reads the registration, stay refused.
        one("use hal::syscall::enter;\n", "R6");
        one("fn f(r: &R) { hal::syscall::child(r); }\n", "R6");
        one("use hal::syscall::{Gpr, handler_info};\n", "R6");
        one("fn f() { hal::syscall::dispatch(1, &A, &mut R); }\n", "R6");
        clean("use sysdefs::proc::syscall::{SYS_exit, SYS_read};\nuse sysdefs::base::errno::*;\n");
        clean("use hal::syscall::SysArgs;\nuse sysdefs::proc::syscall::{SYS_exit, SYS_read};\n");
        clean("fn f() -> i32 { sysdefs::base::errno::Errno::ENOSYS.as_i32() }\n");
        clean("use sysdefs as headers;\n");
        // `hal::sys` is gone (decision 54): the headers are not hal's to name.
        one("use hal::sys::errno::*;\n", "R6");
        one(
            "fn f() -> i32 { hal::sys::errno::Errno::ENOSYS.as_i32() }\n",
            "R6",
        );
        // `::hal` is the crate: clean here because the path is allowed.
        clean("pub use ::sysdefs::proc::syscall::SYS_exit;\n");
        // Another crate's or this crate's module, a field and a comment are not the crate.
        clean("use other::hal::cpu;\nuse crate::hal::x;\nuse self::hal::y;\nuse super::hal::z;\n");
        clean("fn f(s: S) { s.hal.cpu(); }\n");
        clean("//! See `hal::cpu`.\n/// `hal::user::UserPtr`.\nfn f() {}\n");
        // A path outside the list: a `use`, a qualified call, a leading `::`, in a group.
        one("use hal::cpu::set_panicking;\n", "R6");
        one("fn f() { hal::cpu::set_panicking(); }\n", "R6");
        one(
            "fn f() { let t = ::hal::user::KcopyToken::take(); }\n",
            "R6",
        );
        one("use hal::{syscall::SysArgs, user::KcopyToken};\n", "R6");
        one("use hal::{syscall::{SysArgs}, bridges};\n", "R6");
        // Decision 52: the KernelApi table is gone, and hal::api with it.
        one("use hal::api::KernelApi;\n", "R6");
        one("fn f(a: &hal::api::KernelApi) {}\n", "R6");
        // A glob of the root, `self` in a group, `hal` alone.
        one("use hal::*;\n", "R6");
        one("use hal::{syscall::SysArgs, *};\n", "R6");
        one("use hal::{self, syscall::SysArgs};\n", "R6");
        one("use hal as h;\n", "R6");
        one("extern crate hal;\n", "R6");
        assert_eq!(msgs("use hal::{cpu, user::X};\n"), ["1:R6", "1:R6"]);
        // Test code is not checked.
        clean("#[cfg(test)]\nmod tests {\n    use hal::cpu::x;\n}\n");
    }

    #[test]
    fn r6_kernel_only_through_api() {
        // Decision 52: the door, its groups and functions, by their whole path.
        clean("fn f() -> R { kernel::api::file::read(1, 2) }\n");
        clean("fn f() { let g = kernel::api::fs::open; }\n");
        clean("fn f() { ::kernel::api::syscall_legacy(1, &A, &mut R); }\n");
        clean("fn f() { r#kernel::api::vm::mmap(1); }\n");
        // Another crate's module, a field, a comment and test code are not the crate.
        clean("use other::kernel::proc;\nuse crate::kernel::x;\nfn f(s: S) { s.kernel.proc(); }\n");
        clean("//! `kernel::proc::fork`.\nfn f() {}\n");
        clean("#[cfg(test)]\nmod tests {\n    use kernel::syscall::install_legacy_syscalls;\n}\n");
        // Any other module of the kernel, a glob, the crate alone (in a `use`, R3 refuses it
        // first, whatever it names).
        one(
            "fn f() { kernel::syscall::install_legacy_syscalls(&R); }\n",
            "R6",
        );
        one("fn f() { ::kernel::file::descrip::fdalloc(); }\n", "R6");
        one("fn f() { let x = kernel::proc::P; }\n", "R6");
        one("fn f() { kernel; }\n", "R6");
        one("extern crate kernel;\n", "R6");
        one("fn f() -> u32 { return ::kernel::proc::x(); }\n", "R6");
        one("fn f() { let r = 0..kernel::vm::N; }\n", "R6");
        one("use kernel::proc::fork;\n", "R3");
        one("use kernel::*;\n", "R3");
        one("use kernel as k;\n", "R3");
    }

    #[test]
    fn r6_hal_after_a_keyword_a_range_or_as_a_raw_identifier() {
        // Review 2's B1: a keyword before `::hal` leaves it the crate's path.
        one("use ::hal::cpu::set_panicking;\n", "R6");
        one("use ::hal::cpu::x;\n", "R6");
        one("pub use ::hal::user::KcopyToken;\n", "R6");
        one("fn f() -> u32 { return ::hal::cpu::x(); }\n", "R6");
        one("fn f(a: &mut ::hal::cpu::T) {}\n", "R6");
        one("impl ::hal::bridges::X for Y {}\n", "R6");
        // A range's `..` is no field access, and `r#hal` is `hal`.
        one("fn f() { let r = 0..hal::cpu::N; }\n", "R6");
        one("fn f() { let s = S { ..hal::cpu::DEFAULT }; }\n", "R6");
        one("fn f() { r#hal::cpu::x(); }\n", "R6");
        one("use r#hal::user::KcopyToken;\n", "R6");
    }

    #[test]
    fn r6_hal_syscall_only_its_types_and_the_registration() {
        // The user's answer of 2026-11-06 and decision 52: each type of hal::syscall and
        // set_handler, alone, in a group, in a root group.
        for t in HAL_SYSCALL_ITEMS {
            clean(&format!("use hal::syscall::{t};\n"));
        }
        clean(
            "use hal::syscall::{HandlerInfo, Register, Rval, SysArgs, SysResult, SyscallFn, ENOSYS};\n",
        );
        clean("fn f(r: hal::syscall::Rval) -> i32 { hal::syscall::ENOSYS }\n");
        clean("use hal::{syscall::{SysArgs, SysResult}};\n");
        clean("fn f() { let _ = hal::syscall::set_handler(g, I); }\n");
        clean("use ::hal::syscall::set_handler;\n");
        // Its other functions, a glob, the module itself or an alias of it.
        one("fn f() { hal::syscall::dispatch(1, &A, &mut R); }\n", "R6");
        one("fn f() { let i = hal::syscall::handler_info(); }\n", "R6");
        one("use hal::syscall::{SysArgs, dispatch};\n", "R6");
        one("use hal::syscall::*;\n", "R6");
        one("use hal::syscall;\n", "R6");
        one("use hal::syscall as s;\n", "R6");
        one("use hal::syscall::{self, SysArgs};\n", "R6");
        one("use hal::{syscall, syscall::SysArgs};\n", "R6");
        one(
            "use hal::{syscall::{SysArgs}, syscall::{handler_info}};\n",
            "R6",
        );
    }

    #[test]
    fn test_code_is_not_checked() {
        clean(
            "#[cfg(test)]\nmod tests {\n    fn f() { loop {} }\n    static X: Cell<u8> = Cell::new(0);\n}\n",
        );
    }

    #[test]
    fn a_finding_prints_as_path_line_message() {
        let f = check_source("sys/abi/lib.rs", "\nfn f() { loop {} }\n");
        assert_eq!(f[0].to_string(), "sys/abi/lib.rs:2: R1: `loop` in fn f");
    }

    #[test]
    fn declarations_without_a_body_and_the_real_crate_pass() {
        clean("trait T { fn f(&self, nr: u32) -> u32; }\nfn g(p: fn(u8) -> u8) {}\n");
        clean("#![no_std]\n#![forbid(unsafe_code)]\n");
    }
}
/* </TESTS> */
