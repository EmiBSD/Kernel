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
//! Rules D1 and D2 keep drivers and devices apart (hal's groups, 2026-11-26): D1, no file under
//! `sys/hal/dev/` names `kernel::`, `bsd::` or a driver word (agintc, ampintc, gic but `gicsys`,
//! virtio, vio, pluart, psci, pci, nvme, ahci, usb); D2, a file under `sys/kernel/dev/` names
//! `hal::` only through `dev`, `sync`, `mem`, `ds`, `cpu`, `sys`, `user`, `prf`, `assert`,
//! `random`, `bridges` and `cons` (and the root's `*_adapter!` macros), never `vm`, `arch`,
//! `machine`, `api` or `syscall`.
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
//!    apart: the test-only files are the ones `unsafe-report` leaves out, `tests.rs`, `tests/`
//!    and the files a test-only `mod x;` declares, so both tools compare the same number) may
//!    not exceed `hal_ceiling` in the `[ratchet]` table of `unsafe-budget.toml`;
//!    `unsafe-report --write` lowers the key and never raises it. A missing key is an error.
//!
//! Two more rules keep the premise of `hal::ds::pinlist` (its I1: an element linked through
//! `Pin<&T>` is not `Unpin`, so safe code cannot move it while a list names it), over every
//! `.rs` under `sys/`, on the lexed tokens (a comment or a string never matches):
//!
//! 5. **`PinAdapter` is implemented only by `pin_adapter!`.** An `impl .. PinAdapter for`, or
//!    any naming of the trait's hidden seal `PinSealed` (it has no use but an impl), outside
//!    `sys/hal/ds/pinlist.rs` (the macro's body) is an error: every adapter then passes the
//!    macro's compile-time refusal of an `Unpin` element.
//! 6. **No `impl .. Unpin for` in `sys/kernel/` or `sys/abi/`.** A manual `Unpin` is the only
//!    safe way to defeat `Pin` (a pinned value of an `Unpin` type may be moved); `hal`'s own
//!    are allowed (it writes none for an element type). A rename (`Unpin as X`, which would
//!    let `impl X for` through) is refused there too.
//!
//! Both rules read an identifier as Rust does: a raw one (`r#PinSealed`, `r#Unpin`) is the
//! same name.
//!
//! Rules 1 to 3 apply to every line of every file, test code included (a test that builds
//! under Miri proves its sites too). Each finding prints as `path:line: message`; the
//! command exits non-zero when there is one.

use std::fs;
use std::path::Path;

use crate::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::unsafereport::{Kind, Lexed, Tok, lex_pos, scan};

const HAL_DIR: &str = "sys/hal";

/// Where rules 5 and 6 look: every `.rs` of the kernel's tree.
const SYS_DIR: &str = "sys";

/// The one file that may implement `PinAdapter` and name its seal: `pin_adapter!`'s.
const PINLIST: &str = "sys/hal/ds/pinlist.rs";

/// The crates where rule 6 refuses an `impl Unpin`.
const NO_UNPIN_DIRS: [&str; 2] = ["sys/kernel/", "sys/abi/"];

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
    // The test-only files as `unsafe-report` finds them (`tests.rs`, `tests/`, and the files a
    // test-only `mod x;` declares, such as `testutil.rs`), so both tools count the same sites
    // against `hal_ceiling`.
    let mut sources = BTreeMap::new();
    for f in &files {
        let src = fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        sources.insert(f.clone(), src);
    }
    let lexed: BTreeMap<PathBuf, Vec<Tok>> = sources
        .iter()
        .map(|(f, src)| (f.clone(), crate::unsafereport::lex(src)))
        .collect();
    let test_files = crate::unsafereport::test_files(&lexed);
    let mut findings = Vec::new();
    let mut sites = 0usize;
    for f in &files {
        let src = &sources[f];
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let c = check_source(&rel, src, test_files.contains(f));
        sites += c.unsafe_sites;
        findings.extend(c.findings);
    }
    // Rules 5 and 6, over the whole tree.
    let mut all = Vec::new();
    let sys = root.join(SYS_DIR);
    if sys.is_dir() {
        crate::walk_rs(&sys, &mut all)?;
    }
    all.sort();
    for f in &all {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let src = match sources.get(f) {
            Some(s) => s.clone(),
            None => fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?,
        };
        findings.extend(check_pin_premise(&rel, &src));
        findings.extend(check_dev_rules(&rel, &src));
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
        "hal-check: {} files, {sites} unsafe sites <= hal_ceiling {}; no pub unsafe fn, every block proved, no static mut; {} files of sys/: PinAdapter only through pin_adapter!, no impl Unpin in kernel or abi; D1 and D2 hold",
        files.len(),
        ceiling.unwrap_or(0),
        all.len()
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
        if *t == Tok::Ident("static".into())
            && lexed.toks.get(i + 1) == Some(&Tok::Ident("mut".into()))
        {
            add(lexed.lines[i], "static mut is forbidden");
        }
    }
    out
}

/// Rules 5 and 6 over one file's text (`rel` is its path from the workspace root).
pub(crate) fn check_pin_premise(rel: &str, src: &str) -> Vec<Finding> {
    let lexed = lex_pos(src);
    let toks = &lexed.toks;
    let mut out = Vec::new();
    let mut add = |i: usize, message: &str| {
        out.push(Finding {
            path: rel.to_string(),
            line: lexed.lines[i],
            message: message.to_string(),
        });
    };
    let pinlist = rel == PINLIST;
    let no_unpin = NO_UNPIN_DIRS.iter().any(|d| rel.starts_with(d));
    for (i, t) in toks.iter().enumerate() {
        let Tok::Ident(w) = t else { continue };
        let w = plain(w);
        if w == "Unpin"
            && no_unpin
            && toks
                .get(i + 1)
                .is_some_and(|n| matches!(n, Tok::Ident(a) if a == "as"))
        {
            add(
                i,
                "Unpin renamed in kernel or abi: an impl of the new name would defeat Pin (pinlist's I1)",
            );
        }
        if w == "PinSealed" && !pinlist {
            add(
                i,
                "PinSealed named outside sys/hal/ds/pinlist.rs: PinAdapter is implemented by pin_adapter! alone (pinlist's I1)",
            );
        } else if w == "impl" {
            match impl_trait(toks, i).as_deref() {
                Some("PinAdapter") if !pinlist => add(
                    i,
                    "impl PinAdapter by hand: write it with hal::pin_adapter!, which refuses an Unpin element (pinlist's I1)",
                ),
                Some("Unpin") if no_unpin => add(
                    i,
                    "impl Unpin in kernel or abi: a manual Unpin defeats Pin (pinlist's I1)",
                ),
                _ => {}
            }
        }
    }
    out
}

/// The directory of `hal`'s driver-facing group (rule D1).
const HAL_DEV_DIR: &str = "sys/hal/dev/";

/// The directory of the kernel's device services (rule D2).
const KERNEL_DEV_DIR: &str = "sys/kernel/dev/";

/// Rule D1: identifiers that name a driver or a bus. `sys/hal/dev/` holds the typed registers
/// and DMA memory every driver uses, so none of them may appear there: a driver's name in
/// `hal` is a driver in the wrong crate. Kept short on purpose (a long deny-list is brittle);
/// `gic` is refused as a prefix too, except `gicsys` (the GIC system registers N7 adds).
const D1_DRIVER_WORDS: [&str; 11] = [
    "agintc", "ampintc", "gic", "virtio", "vio", "pluart", "psci", "pci", "nvme", "ahci", "usb",
];

/// Rule D2: the `hal` groups a file under `sys/kernel/dev/` may name. Macros exported at the
/// root of `hal` (`hal::list_adapter!`, `hal::tree_adapter!`, `hal::pin_adapter!`) are the
/// `ds` group's and are allowed too. Not allowed: `vm`, `arch`, `machine`, `api`, `syscall` (and `sys`, which is the crate `sysdefs` since decision 54, a leaf of data that a file under `sys/kernel/dev/` names freely).
const D2_ALLOWED: [&str; 12] = [
    "dev", "sync", "mem", "ds", "cpu", "user", "sysbytes", "prf", "assert", "random", "bridges",
    "cons",
];

/// Rules D1 and D2 over one file's text (`rel` is its path from the workspace root): drivers
/// and devices stay apart. D1: no file under `sys/hal/dev/` names `kernel::` or `bsd::` or a
/// driver word. D2: a file under `sys/kernel/dev/` names `hal::` only through the groups of
/// [`D2_ALLOWED`].
pub(crate) fn check_dev_rules(rel: &str, src: &str) -> Vec<Finding> {
    let d1 = rel.starts_with(HAL_DEV_DIR);
    let d2 = rel.starts_with(KERNEL_DEV_DIR);
    let mut out = Vec::new();
    if !d1 && !d2 {
        return out;
    }
    let lexed = lex_pos(src);
    let toks = &lexed.toks;
    let colons = |i: usize| {
        toks.get(i) == Some(&Tok::Punct(':')) && toks.get(i + 1) == Some(&Tok::Punct(':'))
    };
    let mut add = |i: usize, message: String| {
        out.push(Finding {
            path: rel.to_string(),
            line: lexed.lines[i],
            message,
        });
    };
    for (i, t) in toks.iter().enumerate() {
        let Tok::Ident(w) = t else { continue };
        let w = plain(w);
        if d1 {
            let lower = w.to_ascii_lowercase();
            if (w == "kernel" || w == "bsd") && colons(i + 1) {
                add(
                    i,
                    format!(
                        "D1: sys/hal/dev/ names `{w}::`: the driver-facing group knows no driver or kernel module"
                    ),
                );
            } else if D1_DRIVER_WORDS.contains(&lower.as_str())
                || (lower.starts_with("gic") && !lower.starts_with("gicsys"))
            {
                add(
                    i,
                    format!(
                        "D1: sys/hal/dev/ names `{w}`, a driver or bus: only typed registers and DMA memory live here"
                    ),
                );
            }
        }
        if d2 && w == "hal" && colons(i + 1) {
            let allowed = |name: &str| D2_ALLOWED.contains(&name);
            match toks.get(i + 3) {
                Some(Tok::Ident(n)) => {
                    let n = plain(n);
                    let is_macro = toks.get(i + 4) == Some(&Tok::Punct('!'));
                    if !allowed(n) && !is_macro {
                        add(
                            i,
                            format!(
                                "D2: sys/kernel/dev/ names `hal::{n}`; a device service reaches hal through hal::dev, sync, mem, ds, cpu, user, sysbytes, prf, assert, random, bridges or cons"
                            ),
                        );
                    }
                }
                Some(Tok::Punct('{')) => {
                    let mut depth = 0usize;
                    let mut first = true;
                    for (k, tk) in toks.iter().enumerate().skip(i + 3) {
                        match tk {
                            Tok::Punct('{') => {
                                depth += 1;
                                first = true;
                            }
                            Tok::Punct('}') => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            Tok::Punct(',') => first = true,
                            Tok::Ident(n) if depth == 1 && first => {
                                first = false;
                                let n = plain(n);
                                if !allowed(n) {
                                    add(
                                        k,
                                        format!(
                                            "D2: sys/kernel/dev/ names `hal::{{{n}`; a device service reaches hal through hal::dev, sync, mem, ds, cpu, user, sysbytes, prf, assert, random, bridges or cons"
                                        ),
                                    );
                                }
                            }
                            _ => first = false,
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// An identifier as Rust reads it: `r#name` is `name`.
fn plain(w: &str) -> &str {
    w.strip_prefix("r#").unwrap_or(w)
}

/// The last identifier of the trait an `impl` at `i` implements (`impl<..> a::b::Trait<..>
/// for`), or `None` when the `impl` is no trait impl (`impl X {`, an `impl Trait` type).
fn impl_trait(toks: &[Tok], i: usize) -> Option<String> {
    let mut j = i + 1;
    // The impl's generics.
    if toks.get(j) == Some(&Tok::Punct('<')) {
        j = skip_angles(toks, j)?;
    }
    let mut last: Option<String> = None;
    let mut depth = 0usize;
    while let Some(t) = toks.get(j) {
        match t {
            Tok::Punct('-') if toks.get(j + 1) == Some(&Tok::Punct('>')) => j += 1,
            Tok::Punct('<') => depth += 1,
            Tok::Punct('>') => depth = depth.checked_sub(1)?,
            Tok::Ident(w) if depth == 0 && w == "for" => return last,
            Tok::Ident(w) if depth == 0 => last = Some(plain(w).to_string()),
            Tok::Punct('{' | ';' | ',' | ')' | '=') if depth == 0 => return None,
            _ => {}
        }
        j += 1;
    }
    None
}

/// The index past the `>` that closes the `<` at `j` (a `->` inside is no closer).
fn skip_angles(toks: &[Tok], mut j: usize) -> Option<usize> {
    let mut depth = 0usize;
    while let Some(t) = toks.get(j) {
        match t {
            Tok::Punct('-') if toks.get(j + 1) == Some(&Tok::Punct('>')) => j += 1,
            Tok::Punct('<') => depth += 1,
            Tok::Punct('>') => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(j + 1);
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
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

    fn dev_msgs(rel: &str, src: &str) -> Vec<String> {
        check_dev_rules(rel, src)
            .iter()
            .map(|f| format!("{}:{}", f.line, &f.message[..2]))
            .collect()
    }

    #[test]
    fn d1_keeps_drivers_out_of_hal_dev() {
        let hal = "sys/hal/dev/x.rs";
        assert!(dev_msgs(hal, "use crate::mem::pool::TypedPool;\nstruct Mmio;\n").is_empty());
        assert!(dev_msgs(hal, "struct GicSys; fn gicsys_read() {}\n").is_empty());
        assert!(dev_msgs(hal, "// agintc and virtio in a comment\nlet s = \"pci\";\n").is_empty());
        for src in [
            "use kernel::dev::x;\n",
            "fn f() { bsd::dev::y(); }\n",
            "struct Agintc;\n",
            "fn virtio() {}\n",
            "use x::PCI;\n",
            "mod gic;\n",
            "mod gicv3;\n",
            "fn nvme() {}\n",
        ] {
            assert_eq!(dev_msgs(hal, src), vec!["1:D1"], "{src}");
        }
        // Only sys/hal/dev/ is policed.
        assert!(dev_msgs("sys/hal/mem/x.rs", "struct Agintc;\n").is_empty());
    }

    #[test]
    fn d2_limits_the_hal_groups_of_kernel_dev() {
        let k = "sys/kernel/dev/x.rs";
        for src in [
            "use hal::sync::mutex::Mutexed;\n",
            "use hal::cons::putc;\n",
            "use hal::sysbytes::DisklabelBytes;\n",
            "use hal::{cpu::intr::IPL_BIO, ds::link::Link, sync::percpu::PerCpu};\n",
            "use hal::{bridges, user::X};\nuse sysdefs::errno::Errno;\n",
            "hal::pin_adapter!(X);\n",
            "let n = hal::user::copyin();\n",
            "use hal::dev::mmio::Mmio;\n",
        ] {
            assert!(dev_msgs(k, src).is_empty(), "{src}");
        }
        for src in [
            "use hal::vm::pmap::Pmap;\n",
            "use hal::arch::arm64::x;\n",
            "use hal::machine::x;\n",
            "use hal::api::KernelApi;\n",
            "use hal::syscall::SysArgs;\n",
            "use hal::sys::errno::Errno;\n",
            "use hal::{sync::mutex::Mutexed, vm::param::PAGE_SIZE};\n",
        ] {
            assert_eq!(dev_msgs(k, src), vec!["1:D2"], "{src}");
        }
        assert!(dev_msgs("sys/kernel/proc/x.rs", "use hal::vm::pmap::Pmap;\n").is_empty());
    }

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
    fn a_file_a_test_only_mod_declares_is_test_code() {
        use crate::unsafereport::{lex, test_files};
        let mut lexed = BTreeMap::new();
        lexed.insert(
            PathBuf::from("/w/sys/hal/lib.rs"),
            lex("pub mod user;\n#[cfg(test)]\npub(crate) mod testutil;\n"),
        );
        lexed.insert(PathBuf::from("/w/sys/hal/user.rs"), lex("pub fn f() {}"));
        lexed.insert(
            PathBuf::from("/w/sys/hal/testutil.rs"),
            lex("pub fn g() { unsafe { h() } }"),
        );
        let tests = test_files(&lexed);
        assert!(tests.contains(&PathBuf::from("/w/sys/hal/testutil.rs")));
        assert!(!tests.contains(&PathBuf::from("/w/sys/hal/user.rs")));
        let c = check_source("sys/hal/testutil.rs", "pub fn g() { unsafe { h() } }", true);
        assert_eq!(c.unsafe_sites, 0);
    }

    fn premise_lines(rel: &str, src: &str) -> Vec<usize> {
        check_pin_premise(rel, src).iter().map(|f| f.line).collect()
    }

    #[test]
    fn rule_5_pin_adapter_only_through_the_macro() {
        // Written by hand anywhere but pinlist.rs: refused, however the path is spelt.
        for src in [
            "impl PinAdapter for A {}\n",
            "impl hal::ds::pinlist::PinAdapter for A {\n    type Elem = N;\n}\n",
            "impl<T: Sync> ::hal::ds::pinlist::PinAdapter for A<T> {}\n",
            "impl<F: Fn() -> u8> PinAdapter for A<F> {}\n",
            "macro_rules! m { ($n:ident) => { impl $crate::pinlist::PinAdapter for $n {} }; }\n",
        ] {
            assert_eq!(premise_lines("sys/kernel/x.rs", src).len(), 1, "{src}");
            assert_eq!(premise_lines("sys/net/if_x.rs", src).len(), 1, "{src}");
        }
        // The seal, named at all (an impl, a renaming use) outside pinlist.rs: refused.
        assert_eq!(
            premise_lines(
                "sys/kernel/x.rs",
                "\nimpl hal::ds::pinlist::sealed::PinSealed for A {}\n"
            ),
            vec![2]
        );
        assert_eq!(
            premise_lines(
                "sys/dev/x.rs",
                "use hal::ds::pinlist::sealed::PinSealed as S;\n"
            )
            .len(),
            1
        );
        // The macro's own file, a bound, a call, a comment or a string: allowed.
        assert!(
            premise_lines(
                "sys/hal/ds/pinlist.rs",
                "impl sealed::PinSealed for A {}\nimpl PinAdapter for A {}\n"
            )
            .is_empty()
        );
        for src in [
            "fn f<A: PinAdapter>(l: &PinList<A>) {}\n",
            "impl<A: PinAdapter> Drop for PinList<A> {}\n",
            "hal::pin_adapter!(ToList: Timeout, entry);\n",
            "// impl PinAdapter for A {}\nlet s = \"impl PinSealed for A\";\n",
            "fn f(x: impl PinAdapter) {}\n",
        ] {
            assert!(premise_lines("sys/kernel/x.rs", src).is_empty(), "{src}");
        }
    }

    #[test]
    fn rules_5_and_6_see_through_raw_identifiers_and_renames() {
        // Rule 5: a raw seal, a raw trait name.
        assert_eq!(
            premise_lines(
                "sys/kernel/x.rs",
                "impl hal::ds::pinlist::sealed::r#PinSealed for A {}\n"
            ),
            vec![1]
        );
        assert_eq!(
            premise_lines(
                "sys/net/x.rs",
                "impl hal::ds::pinlist::r#PinAdapter for A {}\n"
            ),
            vec![1]
        );
        // Rule 6: a raw Unpin, a renamed one (and the impl of the new name it would allow).
        assert_eq!(
            premise_lines("sys/abi/x.rs", "impl r#Unpin for N {}\n"),
            vec![1]
        );
        assert_eq!(
            premise_lines(
                "sys/kernel/x.rs",
                "use core::marker::Unpin as Movable;\nimpl Movable for N {}\n"
            ),
            vec![1]
        );
        assert_eq!(
            premise_lines("sys/kernel/x.rs", "use core::marker::{r#Unpin as U};\n"),
            vec![1]
        );
        // Outside kernel and abi a rename is not this rule's business.
        assert!(premise_lines("sys/kern/x.rs", "use core::marker::Unpin as U;\n").is_empty());
    }

    #[test]
    fn rule_6_no_unpin_impl_in_kernel_or_abi() {
        for src in [
            "impl Unpin for Node {}\n",
            "impl core::marker::Unpin for Node {}\n",
            "impl<T> ::core::marker::Unpin for Node<T> {}\n",
        ] {
            assert_eq!(premise_lines("sys/kernel/x.rs", src), vec![1], "{src}");
            assert_eq!(premise_lines("sys/abi/x.rs", src), vec![1], "{src}");
            // hal's own, and the legacy tree, are outside the rule.
            assert!(premise_lines("sys/hal/x.rs", src).is_empty(), "{src}");
            assert!(premise_lines("sys/kern/x.rs", src).is_empty(), "{src}");
        }
        for src in [
            "fn f<T: Unpin>(t: T) {}\n",
            "impl<T: Unpin> Foo for Bar<T> {}\n",
            "fn f() -> impl Unpin {}\n",
            "impl Node {\n    fn unpin(&self) {}\n}\n",
        ] {
            assert!(premise_lines("sys/kernel/x.rs", src).is_empty(), "{src}");
        }
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
