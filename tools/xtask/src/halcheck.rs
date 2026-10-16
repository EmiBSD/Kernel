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
//! `cargo xtask hal-check`: the rules of the `hal` crate (`sys/hal/`), the only crate of the
//! system where `unsafe` may live (docs/ZERO_UNSAFE.md, sections 0.1 and 0.2; decision 20).
//!
//! The crate's API is safe and every site is proved, so over every `.rs` under `sys/hal/`:
//!
//! 1. **No `pub unsafe fn`.** An `unsafe fn` is `pub(crate)` (or `pub(super)`, `pub(self)`,
//!    `pub(in ..)`) or private; a bare `pub` one is an error, because a caller outside the
//!    crate would carry an obligation the compiler cannot see. Methods of a trait impl have
//!    no visibility of their own and are not looked at.
//! 2. **Every `unsafe` block has a `// SAFETY:` comment directly above it**: a comment that
//!    contains `SAFETY:` and ends on the line before the one holding the `unsafe` keyword
//!    (a run of consecutive comment lines counts as one comment, so the `SAFETY:` may open a
//!    multi-line comment), or sits on the same line before it. Only blocks are checked;
//!    `unsafe impl` and `unsafe fn` have their own rule (`# Safety`, unsafe-budget.md).
//! 3. **No `static mut`.**
//! 4. **The count stays under the ceiling.** The crate's `unsafe` sites (every kind, test code
//!    apart, as `unsafe-report` counts them) may not exceed `hal_ceiling` in the `[ratchet]`
//!    table of `unsafe-budget.toml`; `unsafe-report --write` lowers the key and never raises
//!    it. A missing key is an error.
//!
//! Rules 1 to 3 apply to every line of every file, test code included (a test that builds
//! under Miri proves its sites too). Each finding prints as `path:line: message`; the
//! command exits non-zero when there is one.

use std::fs;
use std::path::Path;

use crate::Result;
use crate::unsafereport::{Kind, Lexed, Tok, lex_pos, scan};

const HAL_DIR: &str = "sys/hal";

/// One violation: where and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) message: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.path, self.line, self.message)
    }
}

/// What [`check_source`] finds in one file.
#[derive(Debug, Default)]
pub(crate) struct FileCheck {
    pub(crate) findings: Vec<Finding>,
    /// The file's `unsafe` sites in kernel code (every kind, test code apart).
    pub(crate) unsafe_sites: usize,
}

/// `cargo xtask hal-check`.
pub(crate) fn hal_check(root: &Path) -> Result<()> {
    let mut files = Vec::new();
    let dir = root.join(HAL_DIR);
    if dir.is_dir() {
        crate::walk_rs(&dir, &mut files)?;
    }
    files.sort();
    let mut findings = Vec::new();
    let mut sites = 0usize;
    for f in &files {
        let src = fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let c = check_source(&rel, &src, crate::unsafereport::is_test_path(f));
        sites += c.unsafe_sites;
        findings.extend(c.findings);
    }
    let ceiling = crate::unsafereport::load_budget(root)?
        .1
        .and_then(|r| r.hal_ceiling);
    match ceiling {
        Some(c) if sites > c => findings.push(Finding {
            path: "unsafe-budget.toml".to_string(),
            line: 1,
            message: format!(
                "hal holds {sites} unsafe sites, above hal_ceiling {c} ([ratchet]); it may only fall"
            ),
        }),
        Some(_) => {}
        None => findings.push(Finding {
            path: "unsafe-budget.toml".to_string(),
            line: 1,
            message: "no hal_ceiling in [ratchet]".to_string(),
        }),
    }
    for f in &findings {
        println!("error: {f}");
    }
    if !findings.is_empty() {
        return Err(format!("hal-check: {} finding(s)", findings.len()).into());
    }
    println!(
        "hal-check: {} files, {sites} unsafe sites <= hal_ceiling {}; no pub unsafe fn, every block proved, no static mut",
        files.len(),
        ceiling.unwrap_or(0)
    );
    Ok(())
}

/// Rules 1 to 3 over one file's text (`rel` is its path for the messages; `test_file` marks a
/// file that is all test code), and its `unsafe` count.
pub(crate) fn check_source(rel: &str, src: &str, test_file: bool) -> FileCheck {
    let lexed = lex_pos(src);
    let sc = scan(&lexed.toks, test_file);
    let mut out = FileCheck::default();
    let mut add = |line: usize, message: &str| {
        out.findings.push(Finding {
            path: rel.to_string(),
            line,
            message: message.to_string(),
        });
    };
    for &(i, kind, in_test) in &sc.sites {
        let line = lexed.lines[i];
        match kind {
            Kind::Fn if is_public(&lexed.toks, i) => add(
                line,
                "pub unsafe fn: an unsafe fn of hal is pub(crate) or private",
            ),
            Kind::Block if !has_safety_comment(&lexed, line) => add(
                line,
                "unsafe block without a `// SAFETY:` comment directly above it",
            ),
            _ => {}
        }
        if !in_test {
            out.unsafe_sites += 1;
        }
    }
    for (i, t) in lexed.toks.iter().enumerate() {
        if *t == Tok::Ident("static".into()) && lexed.toks.get(i + 1) == Some(&Tok::Ident("mut".into()))
        {
            add(lexed.lines[i], "static mut is forbidden");
        }
    }
    out
}

/// The `unsafe fn` whose `unsafe` keyword is at `i` is declared with a bare `pub`: walk back
/// over `const`, `async` and an `extern "ABI"`, then look for `pub` not followed by `( .. )`.
fn is_public(toks: &[Tok], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        match &toks[j - 1] {
            Tok::Ident(w) if matches!(w.as_str(), "const" | "async" | "extern") => j -= 1,
            Tok::Lit | Tok::Str(_) if j >= 2 && toks[j - 2] == Tok::Ident("extern".into()) => {
                j -= 1;
            }
            _ => break,
        }
    }
    if j == 0 {
        return false;
    }
    match &toks[j - 1] {
        // `pub unsafe fn`.
        Tok::Ident(w) if w == "pub" => true,
        // `pub(crate) unsafe fn`: restricted, whatever the path says.
        _ => false,
    }
}

/// A `SAFETY:` comment ends on the line before `line`, or sits on `line` itself.
fn has_safety_comment(lexed: &Lexed, line: usize) -> bool {
    // The lines above that hold a comment, walked up from `line - 1` while contiguous.
    let mut want = line.saturating_sub(1);
    let mut run: Vec<&crate::unsafereport::Comment> = Vec::new();
    while want > 0 {
        match lexed.comments.iter().find(|c| c.last == want) {
            Some(c) => {
                run.push(c);
                want = c.first.saturating_sub(1);
            }
            None => break,
        }
    }
    run.iter().any(|c| c.text.contains("SAFETY:"))
        || lexed
            .comments
            .iter()
            .any(|c| c.last == line && c.text.contains("SAFETY:"))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    fn lines_of(src: &str) -> Vec<usize> {
        check_source("sys/hal/x.rs", src, false)
            .findings
            .iter()
            .map(|f| f.line)
            .collect()
    }

    #[test]
    fn a_public_unsafe_fn_is_refused() {
        for src in [
            "pub unsafe fn f() {}\n",
            "pub const unsafe fn f() {}\n",
            "pub unsafe extern \"C\" fn f() {}\n",
            "impl X {\n    pub unsafe fn f(&self) {}\n}\n",
        ] {
            assert_eq!(
                lines_of(src).len(),
                1,
                "{src}: {:?}",
                check_source("x", src, false).findings
            );
        }
        assert_eq!(lines_of("\n\npub unsafe fn f() {}\n"), vec![3]);
    }

    #[test]
    fn restricted_and_private_unsafe_fns_pass() {
        for src in [
            "pub(crate) unsafe fn f() {}\n",
            "pub(super) unsafe fn f() {}\n",
            "pub(in crate::x) unsafe fn f() {}\n",
            "unsafe fn f() {}\n",
            "fn f(p: unsafe fn(u8)) {}\n",
            "impl T for X {\n    unsafe fn f() {}\n}\n",
        ] {
            assert!(lines_of(src).is_empty(), "{src}");
        }
    }

    #[test]
    fn an_unsafe_block_needs_a_safety_comment_directly_above() {
        let ok = [
            "// SAFETY: p is valid.\nunsafe { f() };\n",
            "// SAFETY: p is valid,\n// and aligned.\nunsafe { f() };\n",
            "    // SAFETY: first line\n    // second\n    // third\n    let x = unsafe { f() };\n",
            "/* SAFETY: block comment. */\nunsafe { f() };\n",
            "/*\n * SAFETY: long\n */\nunsafe { f() };\n",
            "let x = /* SAFETY: inline */ unsafe { f() };\n",
        ];
        for src in ok {
            assert!(lines_of(src).is_empty(), "{src}: {:?}", lines_of(src));
        }
        let bad = [
            ("unsafe { f() };\n", 1),
            ("// not it\nunsafe { f() };\n", 2),
            ("// SAFETY: too far\n\nunsafe { f() };\n", 3),
            ("// SAFETY: code between\nlet a = 1;\nunsafe { f() };\n", 3),
            ("let s = \"// SAFETY: in a string\";\nunsafe { f() };\n", 2),
        ];
        for (src, line) in bad {
            assert_eq!(lines_of(src), vec![line], "{src}");
        }
    }

    #[test]
    fn static_mut_is_refused_but_not_a_static_mut_reference() {
        assert_eq!(lines_of("\nstatic mut X: u8 = 0;\n"), vec![2]);
        assert_eq!(lines_of("pub static mut X: u8 = 0;\n"), vec![1]);
        assert!(lines_of("fn f(x: &'static mut u8) {}\nstatic Y: u8 = 0;\n").is_empty());
    }

    #[test]
    fn the_count_excludes_test_code_and_counts_every_kind() {
        let src = "\
// SAFETY: a.
unsafe { a() };
unsafe fn g() {}
unsafe impl Send for X {}
#[cfg(test)]
mod tests {
    fn t() {
        // SAFETY: b.
        unsafe { b() };
    }
}
";
        let c = check_source("sys/hal/x.rs", src, false);
        assert!(c.findings.is_empty(), "{:?}", c.findings);
        assert_eq!(c.unsafe_sites, 3);
        // A test file counts nothing, but its blocks still need their proofs.
        let t = check_source("sys/hal/tests.rs", "unsafe { b() };\n", true);
        assert_eq!(t.unsafe_sites, 0);
        assert_eq!(t.findings.len(), 1);
    }

    #[test]
    fn a_finding_prints_as_path_line_message() {
        let c = check_source("sys/hal/lib.rs", "\nstatic mut X: u8 = 0;\n", false);
        assert_eq!(
            c.findings[0].to_string(),
            "sys/hal/lib.rs:2: static mut is forbidden"
        );
    }
}
/* </TESTS> */
