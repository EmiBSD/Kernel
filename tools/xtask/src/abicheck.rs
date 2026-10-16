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
//! * **R3** at most one call per function into `kernel::` (a path starting with `kernel::`) or
//!   through a parameter of type `&KernelApi`.
//! * **R4** at most 40 lines per function body (the line of `{` to the line of `}`).
//! * **R5** no `static` whose type names `Cell`, `RefCell`, `Atomic*`, `OnceStatic`,
//!   `Mutexed`, `Rwlocked` or `UnsafeCell`: no state in `abi`; plain tables are fine.
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
        "abi-check: {} files; no loop, branches only on nr or Ok/Err/Some/None, at most one kernel call and {MAX_BODY_LINES} lines per fn, no state",
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
    params: (usize, usize),
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
        let params = (j + 1, pe);
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
            params,
            open,
            close,
        });
    }
    out
}

/// The names of the parameters whose type is a reference to `KernelApi`.
fn api_params(toks: &[Tok], (from, to): (usize, usize)) -> Vec<String> {
    let mut names = Vec::new();
    let (mut start, mut depth) = (from, 0i32);
    let mut parts: Vec<(usize, usize)> = Vec::new();
    for k in from..to {
        match toks[k] {
            Tok::Punct('(' | '[' | '{' | '<') => depth += 1,
            Tok::Punct(')' | ']' | '}') => depth -= 1,
            Tok::Punct('>') if k > 0 && toks[k - 1] != Tok::Punct('-') => depth -= 1,
            Tok::Punct(',') if depth == 0 => {
                parts.push((start, k));
                start = k + 1;
            }
            _ => {}
        }
    }
    parts.push((start, to));
    for (a, b) in parts {
        let colon = (a..b).find(|&k| {
            toks[k] == Tok::Punct(':')
                && toks.get(k + 1) != Some(&Tok::Punct(':'))
                && (k == 0 || toks[k - 1] != Tok::Punct(':'))
        });
        let Some(c) = colon else { continue };
        let ty = &toks[c + 1..b];
        if ty.first() == Some(&Tok::Punct('&'))
            && ty.iter().any(|t| id(t, "KernelApi"))
            && let Some(Tok::Ident(n)) = toks.get(c.wrapping_sub(1))
        {
            names.push(n.clone());
        }
    }
    names
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
        let api = api_params(toks, f.params);
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
                // R3, a path call into the kernel.
                Tok::Ident(w)
                    if w == "kernel"
                        && toks.get(i + 1) == Some(&Tok::Punct(':'))
                        && toks.get(i + 2) == Some(&Tok::Punct(':'))
                        && (i == 0 || !matches!(toks[i - 1], Tok::Punct(':' | '.'))) =>
                {
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
                // R3, a method call on a `&KernelApi` parameter.
                Tok::Ident(w)
                    if api.contains(w)
                        && (i == 0 || toks[i - 1] != Tok::Punct('.'))
                        && toks.get(i + 1) == Some(&Tok::Punct('.')) =>
                {
                    // `name.a.b(..)`: walk the field chain to the call.
                    let mut k = i + 1;
                    while toks.get(k) == Some(&Tok::Punct('.'))
                        && matches!(toks.get(k + 1), Some(Tok::Ident(_)))
                    {
                        match toks.get(k + 2) {
                            Some(Tok::Punct('(')) => {
                                calls += 1;
                                break;
                            }
                            Some(Tok::Punct('.')) => k += 2,
                            _ => break,
                        }
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
    out.sort_by_key(|f| f.line);
    out
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
        clean("fn f(a: &KernelApi) { a.pipe(1); }\n");
        clean("fn f() { kernel::pipe::new(1); }\n");
        clean("fn f() { let x = Wrap(1); kernel::open(x); }\n");
        one("fn f() { kernel::a(1); kernel::b(2); }\n", "R3");
        one("fn f(api: &KernelApi) { api.a(1); api.b(2); }\n", "R3");
        one(
            "fn f(api: &KernelApi) { api.a(1); kernel::b(api.x); }\n",
            "R3",
        );
        one(
            "fn f(api: &KernelApi) { api.sub.a(1); api.sub.b(2); }\n",
            "R3",
        );
        // A by-value parameter, a field read and another crate's path are not calls into it.
        clean("fn f(api: KernelApi) { api.a(1); api.b(2); }\n");
        clean("fn f(a: &KernelApi) { let _ = a.n; other::kernel(); kernel::K; }\n");
        // Nested functions are judged on their own.
        clean("fn f() { kernel::a(1); fn g() { kernel::b(2); } }\n");
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
