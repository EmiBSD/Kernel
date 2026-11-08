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
//! `cargo xtask manual` and `cargo xtask include`: the two files of the release tarball that
//! Userland reads (the user's decisions of 2026-11-07 and 2026-11-08).
//!
//! `manual --arch arm64 --boot-log LOG --out FILE` writes `MANUAL.toml`, the kernel's real
//! interface inventory, generated from the tree and never by hand:
//!
//! - `[drivers]`: `configured` is every device and pseudo-device of the arm64 kernel by its
//!   man4 name, read from `sys/arch/arm64/conf/ioconf.rs` (the driver of each `Cfdata::new`
//!   entry, named by its `*_CD` static, and `PDEVNAMES`); `attached_qemu_virt` is the subset
//!   whose attach line (`<name><unit> at <parent>`) is in the boot log of the release kernel.
//! - `[syscalls]`: every number below `SYS_MAXSYSCALL` in exactly one list. `native` is
//!   `sys/abi/sysent.rs`'s `Sysent::Native` entries; the rest are `Legacy`, which `bsd`'s
//!   generated table `sys/kern/init_sysent.rs` answers: a row whose handler is `sys_nosys`
//!   (unimplemented, obsolete, or "not ported") is `absent`, the others `legacy`. A call that
//!   is implemented but reaches an `unported!` path inside is not told apart here: no
//!   `partial` list is written, because no reliable mechanical mapping from a handler to
//!   the `unported!` sites under it exists.
//!   A slot with no name in `syscalls.master` has the empty name; an `obsolete` one keeps the
//!   master file's words (`4.2 sigreturn`), so a name is never another call's.
//! - `[[pages]]`: manual pages whose text is not true for this kernel, reviewed data in
//!   [`PAGES`], not generated.
//!
//! `include --arch arm64 --out DIR` copies what OpenBSD's `include/Makefile` target `copies`
//! installs from `sys/`: the `.h` files of `LDIRS` (but `dev/microcode` and `dev/pci/drm`),
//! keeping subdirectories, and `sys/arch/arm64/include/*.h` as `arm64/` with `machine`
//! a link to it. The text is the OpenBSD pin's, unchanged. arm64 only (decision 44).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;

/// The directories `include/Makefile` installs from `sys/` (`LDIRS`).
const LDIRS: &[&str] = &[
    "crypto", "ddb", "dev", "isofs", "miscfs", "msdosfs", "net", "netinet", "netinet6", "netmpls",
    "net80211", "nfs", "ntfs", "scsi", "sys", "ufs", "uvm",
];

/// Subtrees of `LDIRS` that the Makefile leaves out.
const EXCLUDED: &[&str] = &["dev/microcode/", "dev/pci/drm/"];

/// Manual pages whose text is not true for this kernel, with the reason. Reviewed data.
const PAGES: &[(&str, &str)] = &[
    (
        "config(8)",
        "EmiBSD's kernel is not configured with config(8): the autoconfiguration tables are \
         hand-written (sys/arch/arm64/conf/ioconf.rs) and there is no kernel configuration \
         file to build.",
    ),
    (
        "options(4)",
        "Kernel options are Cargo features of the bsd crate (sys/Cargo.toml), not config(8) \
         options; arm64 ships one MP kernel (decision 40), so no option selects a \
         uniprocessor kernel.",
    ),
    (
        "ddb(4)",
        "ddb is built in and its command loop is ported (sys/ddb), but several commands are \
         visible stubs that report 'unported': print, examine and search, breakpoints and \
         watchpoints, symbol lookup (addresses print as numbers), ps, and most show \
         printers; WITNESS is not configured (sys/ddb/db_command.rs, Deviations).",
    ),
    (
        "boot(8)",
        "unverified: EmiBSD's loaders (sys/stand/boot, sys/arch/arm64/stand/efiboot) are \
         ports of OpenBSD's, and the pin carries no arm64 boot.8 to compare with.",
    ),
];

/// One row of `init_sysent.rs`: a number, OpenBSD's name and whether the handler is `sys_nosys`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    number: usize,
    name: String,
    nosys: bool,
}

/// The syscalls of `MANUAL.toml`, each list sorted by number.
#[derive(Debug, Default, PartialEq, Eq)]
struct Syscalls {
    native: Vec<(String, usize)>,
    legacy: Vec<(String, usize)>,
    absent: Vec<(String, usize)>,
}

/// The text of `src` from `marker` on; an error when `marker` is missing.
fn after<'a>(src: &'a str, marker: &str) -> Result<&'a str> {
    src.find(marker)
        .map(|i| &src[i..])
        .ok_or_else(|| format!("`{marker}` not found").into())
}

/// The identifier that starts `s` after an `&`, skipping blanks, and the rest.
fn amp_ident(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start().strip_prefix('&')?;
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    Some((&s[..end], &s[end..]))
}

/// Every device and pseudo-device of `ioconf.rs`, by its man4 name, sorted and unique.
fn parse_ioconf(src: &str) -> Result<Vec<String>> {
    let mut names = BTreeSet::new();
    let cfdata = after(src, "pub static CFDATA")?;
    let end = cfdata.find("pub static CFROOTS").unwrap_or(cfdata.len());
    for part in cfdata[..end].split("Cfdata::new(").skip(1) {
        let (_ca, rest) = amp_ident(part).ok_or("Cfdata::new: no attachment argument")?;
        let rest = rest
            .trim_start()
            .strip_prefix(',')
            .ok_or("Cfdata::new: no comma")?;
        let (cd, _) = amp_ident(rest).ok_or("Cfdata::new: no driver argument")?;
        let name = cd
            .strip_suffix("_CD")
            .ok_or_else(|| format!("Cfdata::new: driver `{cd}` does not end in _CD"))?;
        names.insert(name.to_ascii_lowercase());
    }
    let pdev = after(src, "pub static PDEVNAMES")?;
    let pdev = &pdev[pdev.find('=').ok_or("PDEVNAMES: no `=`")?..];
    let pdev = &pdev[..pdev.find("];").ok_or("PDEVNAMES: no end")?];
    for line in pdev.lines() {
        if let Some(rest) = line.trim().strip_prefix("b\"") {
            let name = rest.split('"').next().unwrap_or("");
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
    }
    if names.is_empty() {
        return Err("ioconf.rs: no driver found".into());
    }
    Ok(names.into_iter().collect())
}

/// The driver names that have an attach line (`name<unit> at parent...`) in a boot log. Only
/// the part after the `----- serial transcript` banner counts when the log has one.
fn parse_attached(log: &str) -> BTreeSet<String> {
    let body = match log.find("----- serial transcript") {
        Some(i) => &log[i..],
        None => log,
    };
    let mut out = BTreeSet::new();
    for line in body.lines() {
        let mut words = line.split_whitespace();
        let (Some(dev), Some("at"), Some(_parent)) = (words.next(), words.next(), words.next())
        else {
            continue;
        };
        let name = dev.trim_end_matches(|c: char| c.is_ascii_digit());
        let unit = &dev[name.len()..];
        if !name.is_empty() && !unit.is_empty() && name.chars().all(|c| c.is_ascii_lowercase()) {
            out.insert(name.to_string());
        }
    }
    out
}

/// `pub const SYS_name: i32 = N;` of `sys/hal/sys/syscall.rs`, and `SYS_MAXSYSCALL`.
fn parse_syscall_consts(src: &str) -> Result<(BTreeMap<String, usize>, usize)> {
    let mut map = BTreeMap::new();
    let mut max = None;
    for line in src.lines() {
        let Some(rest) = line.trim().strip_prefix("pub const SYS_") else {
            continue;
        };
        let Some((name, val)) = rest.split_once(':') else {
            continue;
        };
        let Some((_, num)) = val.split_once('=') else {
            continue;
        };
        let Ok(n) = num.trim().trim_end_matches(';').trim().parse::<usize>() else {
            continue;
        };
        if name == "MAXSYSCALL" {
            max = Some(n);
        } else {
            map.insert(name.to_string(), n);
        }
    }
    let max = max.ok_or("syscall.rs: no SYS_MAXSYSCALL")?;
    Ok((map, max))
}

/// The `x` of every `table[SYS_x as usize] = Sysent::Native(...)` of `sysent.rs`.
fn parse_native(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("table[SYS_")
            && t.contains("= Sysent::Native(")
            && let Some((name, _)) = rest.split_once(" as usize]")
        {
            out.push(name.to_string());
        }
    }
    out
}

/// OpenBSD's name out of a row's comment (`unimplemented ptrace`, `obsolete 4.2 sigreturn`,
/// `__thrsleep (sys___thrsleep not ported)`, `read`). A slot with no name in
/// `syscalls.master` (`unimplemented` alone) has the empty name.
fn row_name(comment: &str) -> String {
    let mut words = comment.split_whitespace();
    let first = words.next().unwrap_or("");
    if first == "unimplemented" || first == "obsolete" {
        words.collect::<Vec<_>>().join(" ")
    } else {
        first.to_string()
    }
}

/// The rows of `init_sysent.rs`'s `SYSENT`, in table order. A row under `#[cfg(not(...))]`
/// is the master file's `#else` branch and is left out; the one it replaces is kept.
fn parse_init_sysent(src: &str) -> Result<Vec<Row>> {
    let start = src
        .find("pub static SYSENT")
        .ok_or("init_sysent.rs: no SYSENT")?;
    let mut rows = Vec::new();
    let mut buf = String::new();
    let mut skip = false;
    for line in src[start..].lines().skip(1) {
        let t = line.trim();
        if t == "];" {
            break;
        }
        if t.starts_with("#[cfg(not(") {
            skip = true;
            continue;
        }
        if t.starts_with("#[cfg(") {
            continue;
        }
        if t.starts_with("Sysent::new(") || !buf.is_empty() {
            buf.push_str(t);
            buf.push(' ');
        } else {
            continue;
        }
        let Some((code, comment)) = buf.split_once(" // ") else {
            continue;
        };
        let Some((num, name)) = comment.split_once(" = ") else {
            return Err(format!("init_sysent.rs: row without `// N = name`: {buf}").into());
        };
        let number: usize = num
            .trim()
            .parse()
            .map_err(|_| format!("init_sysent.rs: bad number in `{buf}`"))?;
        if !skip {
            rows.push(Row {
                number,
                name: row_name(name),
                nosys: code.contains("sys_nosys"),
            });
        }
        skip = false;
        buf.clear();
    }
    if !buf.is_empty() {
        return Err(format!("init_sysent.rs: unterminated row `{buf}`").into());
    }
    Ok(rows)
}

/// Puts every number below `max` in exactly one list. Errors when a number is missing or
/// repeated, or when a native entry has no number or a different name.
fn classify(
    rows: &[Row],
    native: &[String],
    consts: &BTreeMap<String, usize>,
    max: usize,
) -> Result<Syscalls> {
    let mut by_number: BTreeMap<usize, &Row> = BTreeMap::new();
    for r in rows {
        if by_number.insert(r.number, r).is_some() {
            return Err(format!("init_sysent.rs: number {} appears twice", r.number).into());
        }
    }
    if by_number.len() != max || (0..max).any(|n| !by_number.contains_key(&n)) {
        return Err(format!(
            "init_sysent.rs has {} rows for numbers 0..{max}: every number needs exactly one",
            by_number.len()
        )
        .into());
    }
    let mut native_numbers = BTreeSet::new();
    for n in native {
        let num = *consts
            .get(n)
            .ok_or_else(|| format!("sysent.rs: SYS_{n} is not in syscall.rs"))?;
        let row = by_number
            .get(&num)
            .ok_or_else(|| format!("sysent.rs: SYS_{n} = {num} is out of range"))?;
        if row.name != *n {
            return Err(format!(
                "sysent.rs: SYS_{n} is number {num}, whose table row is named `{}`",
                row.name
            )
            .into());
        }
        if !native_numbers.insert(num) {
            return Err(format!("sysent.rs: SYS_{n} is native twice").into());
        }
    }
    let mut out = Syscalls::default();
    for (num, row) in by_number {
        let item = (row.name.clone(), num);
        if native_numbers.contains(&num) {
            out.native.push(item);
        } else if row.nosys {
            out.absent.push(item);
        } else {
            out.legacy.push(item);
        }
    }
    Ok(out)
}

/// A TOML basic string.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn put_list(out: &mut String, key: &str, items: &[String]) {
    let _ = writeln!(out, "{key} = [");
    for i in items {
        let _ = writeln!(out, "    {},", quote(i));
    }
    out.push_str("]\n");
}

fn put_calls(out: &mut String, key: &str, items: &[(String, usize)]) {
    let _ = writeln!(out, "{key} = [");
    for (name, number) in items {
        let _ = writeln!(out, "    {{ name = {}, number = {number} }},", quote(name));
    }
    out.push_str("]\n");
}

/// The `Commit:` line of a `PINNED.md`.
fn pinned_commit(text: &str, what: &str) -> Result<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("Commit:"))
        .map(|c| c.trim().to_string())
        .ok_or_else(|| format!("{what}: no `Commit:` line").into())
}

/// Everything `MANUAL.toml` states.
struct Manual<'a> {
    kernel: &'a str,
    lz: &'a str,
    openbsd: &'a str,
    configured: &'a [String],
    attached: &'a [String],
    syscalls: &'a Syscalls,
}

/// The text of `MANUAL.toml`.
fn render(m: &Manual) -> String {
    let mut out = String::new();
    out.push_str(
        "# MANUAL.toml: the kernel's interface inventory, generated by `cargo xtask manual`.\n",
    );
    out.push_str("# Names are OpenBSD's; the lists are sorted.\n\n");
    out.push_str("[meta]\nschema = 1\narch = \"arm64\"\n");
    let _ = writeln!(out, "kernel = {}", quote(m.kernel));
    let _ = writeln!(out, "landingzone = {}", quote(m.lz));
    let _ = writeln!(out, "openbsd = {}", quote(m.openbsd));
    out.push_str("\n[drivers]\n");
    put_list(&mut out, "configured", m.configured);
    put_list(&mut out, "attached_qemu_virt", m.attached);
    out.push_str("\n[syscalls]\n");
    put_calls(&mut out, "native", &m.syscalls.native);
    put_calls(&mut out, "legacy", &m.syscalls.legacy);
    put_calls(&mut out, "absent", &m.syscalls.absent);
    for (page, reason) in PAGES {
        let _ = writeln!(
            out,
            "\n[[pages]]\npage = {}\nreason = {}",
            quote(page),
            quote(reason)
        );
    }
    out
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()).into())
}

fn arch_arm64(args: &[&str]) -> Result<()> {
    match crate::flag(args, "--arch")? {
        "arm64" => Ok(()),
        other => Err(format!("--arch {other}: only arm64 until N10 (decision 44)").into()),
    }
}

/// `cargo xtask manual --arch arm64 --boot-log LOG --out FILE`.
pub fn manual(root: &Path, args: &[&str]) -> Result<()> {
    arch_arm64(args)?;
    let log = read(Path::new(crate::flag(args, "--boot-log")?))?;
    let out = PathBuf::from(crate::flag(args, "--out")?);
    let configured = parse_ioconf(&read(&root.join("sys/arch/arm64/conf/ioconf.rs"))?)?;
    let seen = parse_attached(&log);
    let attached: Vec<String> = configured
        .iter()
        .filter(|d| seen.contains(*d))
        .cloned()
        .collect();
    if attached.is_empty() {
        return Err("the boot log shows no attach line of a configured driver".into());
    }
    let (consts, max) = parse_syscall_consts(&read(&root.join("sys/hal/sys/syscall.rs"))?)?;
    let native = parse_native(&read(&root.join("sys/abi/sysent.rs"))?);
    let rows = parse_init_sysent(&read(&root.join("sys/kern/init_sysent.rs"))?)?;
    let syscalls = classify(&rows, &native, &consts, max)?;
    let kernel = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git rev-parse HEAD: {e}"))?;
    if !kernel.status.success() {
        return Err("git rev-parse HEAD failed".into());
    }
    let kernel = String::from_utf8_lossy(&kernel.stdout).trim().to_string();
    let lz = pinned_commit(&read(&root.join("lz/PINNED.md"))?, "lz/PINNED.md")?;
    let openbsd = pinned_commit(
        &read(&root.join("reference/PINNED.md"))?,
        "reference/PINNED.md",
    )?;
    let text = render(&Manual {
        kernel: &kernel,
        lz: &lz,
        openbsd: &openbsd,
        configured: &configured,
        attached: &attached,
        syscalls: &syscalls,
    });
    fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "manual: {}: {} drivers configured, {} attached; syscalls {} native, {} legacy, {} absent; {} pages",
        out.display(),
        configured.len(),
        attached.len(),
        syscalls.native.len(),
        syscalls.legacy.len(),
        syscalls.absent.len(),
        PAGES.len()
    );
    Ok(())
}

/// The `.h` files under `dir` (a path relative to `sys`), with the subdirectories kept;
/// symbolic links are followed as `find -follow` does.
fn walk_headers(sys: &Path, dir: &str, out: &mut Vec<String>) -> Result<()> {
    let here = sys.join(dir);
    let mut entries: Vec<_> = fs::read_dir(&here)
        .map_err(|e| format!("{}: {e}", here.display()))?
        .collect::<std::io::Result<_>>()
        .map_err(|e| format!("{}: {e}", here.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let rel = format!("{dir}/{name}");
        let meta = fs::metadata(e.path()).map_err(|e| format!("{rel}: {e}"))?;
        if meta.is_dir() {
            if !EXCLUDED.iter().any(|x| format!("{rel}/").starts_with(x)) {
                walk_headers(sys, &rel, out)?;
            }
        } else if meta.is_file()
            && Path::new(&name).extension().is_some_and(|x| x == "h")
            && !EXCLUDED.iter().any(|x| rel.starts_with(x))
        {
            out.push(rel);
        }
    }
    Ok(())
}

/// `sys/arch/arm64/include/*.h`, not recursive, as `arm64/<file>`.
fn walk_arch(sys: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    let dir = sys.join("arch/arm64/include");
    for e in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = e.file_name().to_string_lossy().into_owned();
        let is_file = fs::metadata(e.path()).is_ok_and(|m| m.is_file());
        if is_file && Path::new(&name).extension().is_some_and(|x| x == "h") {
            out.push((format!("arm64/{name}"), e.path()));
        }
    }
    Ok(())
}

/// The files `include/` holds: `(path under include/, source path)` for arm64, sorted.
fn include_files(sys: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut rels = Vec::new();
    for d in LDIRS {
        walk_headers(sys, d, &mut rels)?;
    }
    let mut files: Vec<(String, PathBuf)> = rels
        .into_iter()
        .map(|r| {
            let src = sys.join(&r);
            (r, src)
        })
        .collect();
    walk_arch(sys, &mut files)?;
    files.sort();
    Ok(files)
}

/// `cargo xtask include --arch arm64 --out DIR`.
pub fn include(root: &Path, args: &[&str]) -> Result<()> {
    arch_arm64(args)?;
    let out = PathBuf::from(crate::flag(args, "--out")?);
    let files = include_files(&root.join(crate::REFERENCE_DIR).join("sys"))?;
    if out.exists() {
        fs::remove_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    for (rel, src) in &files {
        let dst = out.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::copy(src, &dst).map_err(|e| format!("{} -> {}: {e}", src.display(), dst.display()))?;
    }
    std::os::unix::fs::symlink("arm64", out.join("machine"))
        .map_err(|e| format!("{}/machine: {e}", out.display()))?;
    println!(
        "include: {}: {} files, machine -> arm64",
        out.display(),
        files.len()
    );
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    const IOCONF: &str = r#"
use crate::x::{FOO_CA, FOO_CD};
pub static CFDATA: StaticCell<[Cfdata; 3]> = StaticCell::new([
    // 0: mainbus0 at root
    Cfdata::new(&MAINBUS_CA, &MAINBUS_CD, 0, FSTATE_NOTFOUND, &[], 0, &[], 0, 0),
    // 1: uhub* at uhub?
    Cfdata::new(
        &UHUB_UHUB_CA,
        &UHUB_CD,
        0,
    ),
    #[cfg(feature = "multiprocessor")]
    Cfdata::new(&CPU_CA, &CPU_CD, 1),
    Cfdata::new(&CPU_CA, &CPU_CD, 1),
]);
pub static CFROOTS: StaticCell<[i16; 2]> = StaticCell::new([0, 11]);
pub static PDEVNAMES: [&[u8]; 3] = [
    b"pf",
    #[cfg(feature = "fuse")]
    b"fuse",
    b"pty",
];
"#;

    #[test]
    fn ioconf_names_drivers_and_pseudo_devices() {
        let got = parse_ioconf(IOCONF).unwrap();
        assert_eq!(got, ["cpu", "fuse", "mainbus", "pf", "pty", "uhub"]);
    }

    #[test]
    fn ioconf_refuses_a_driver_not_named_cd() {
        let src = "pub static CFDATA: X = [ Cfdata::new(&A_CA, &A_XX, 0) ]; pub static CFROOTS: Y;";
        assert!(parse_ioconf(src).is_err());
    }

    #[test]
    fn attach_lines_give_the_driver_without_the_unit() {
        let log = "boot noise\n----- serial transcript (arm64) -----\n  mainbus0 at root: x\n  \
                   sd0 at scsibus0 targ 0 lun 0: <a>\n  \"fw-cfg\" at mainbus0 not configured\n  \
                   selftest: bus_dma ok (4 maps, 4 pages at 0x5f)\n  virtio12 at mainbus0: Dev\n";
        let got: Vec<_> = parse_attached(log).into_iter().collect();
        assert_eq!(got, ["mainbus", "sd", "virtio"]);
        // Before the banner nothing counts.
        assert!(parse_attached("a0 at b0\n----- serial transcript\n").is_empty());
    }

    const MASTER: &str = r#"
pub static SYSENT: [Sysent; SYS_MAXSYSCALL] = [
    Sysent::new(0, 0, 0, sys_nosys), // 0 = unimplemented syscall
    Sysent::new(1, size_of::<SysExitArgs>(), 0, sys_exit), // 1 = exit
    Sysent::new(0, 0, 0, sys_nosys), // 2 = obsolete msyscall
    Sysent::new(5, 8, SY_NOLOCK, sys_nosys), // 3 = __thrsleep (sys___thrsleep not ported)
    Sysent::new(2, 8, 0, sys_read), // 4 = read
    #[cfg(any(feature = "nfsclient"))]
    Sysent::new(
        2,
        8,
        0,
        sys_nfssvc,
    ), // 5 = nfssvc
    #[cfg(not(any(feature = "nfsclient")))]
    Sysent::new(0, 0, 0, sys_nosys), // 5 = nfssvc (sys_nfssvc needs ...)
];
"#;

    #[test]
    fn init_sysent_rows_keep_one_per_number() {
        let rows = parse_init_sysent(MASTER).unwrap();
        let got: Vec<_> = rows
            .iter()
            .map(|r| (r.number, r.name.as_str(), r.nosys))
            .collect();
        assert_eq!(
            got,
            [
                (0, "syscall", true),
                (1, "exit", false),
                (2, "msyscall", true),
                (3, "__thrsleep", true),
                (4, "read", false),
                (5, "nfssvc", false),
            ]
        );
    }

    #[test]
    fn every_number_is_in_exactly_one_list() {
        let rows = parse_init_sysent(MASTER).unwrap();
        let src = "table[SYS_read as usize] = Sysent::Native(sys_read);\n\
                   table[SYS_exit as usize] = Sysent::Native(sys_exit);\n";
        let native = parse_native(src);
        assert_eq!(native, ["read", "exit"]);
        let consts: BTreeMap<String, usize> = [("read".into(), 4), ("exit".into(), 1)]
            .into_iter()
            .collect();
        let c = classify(&rows, &native, &consts, 6).unwrap();
        let numbers = |v: &[(String, usize)]| v.iter().map(|x| x.1).collect::<Vec<_>>();
        assert_eq!(numbers(&c.native), [1, 4]);
        assert_eq!(numbers(&c.legacy), [5]);
        assert_eq!(numbers(&c.absent), [0, 2, 3]);
        assert_eq!(c.native.len() + c.legacy.len() + c.absent.len(), 6);
    }

    #[test]
    fn classify_refuses_a_gap_a_wrong_name_and_an_unknown_native() {
        let rows = parse_init_sysent(MASTER).unwrap();
        let consts: BTreeMap<String, usize> = [("read".into(), 1)].into_iter().collect();
        assert!(classify(&rows, &[], &consts, 7).is_err());
        assert!(classify(&rows, &["read".into()], &consts, 6).is_err());
        assert!(classify(&rows, &["nope".into()], &consts, 6).is_err());
    }

    #[test]
    fn syscall_consts_and_max() {
        let src = "pub const SYS_exit: i32 = 1;\n/// x\npub const SYS_MAXSYSCALL: usize = 331;\n";
        let (m, max) = parse_syscall_consts(src).unwrap();
        assert_eq!((m["exit"], max), (1, 331));
    }

    #[test]
    fn render_is_valid_toml_with_every_table() {
        let sc = Syscalls {
            native: vec![("exit".into(), 1)],
            legacy: vec![("fork".into(), 2)],
            absent: vec![("syscall".into(), 0)],
        };
        let text = render(&Manual {
            kernel: "abc",
            lz: "def",
            openbsd: "123",
            configured: &["sd".into(), "vio".into()],
            attached: &["sd".into()],
            syscalls: &sc,
        });
        let v: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(v["meta"]["schema"].as_integer(), Some(1));
        assert_eq!(v["drivers"]["configured"].as_array().map(Vec::len), Some(2));
        assert_eq!(v["syscalls"]["absent"][0]["number"].as_integer(), Some(0));
        assert_eq!(v["pages"].as_array().map(Vec::len), Some(PAGES.len()));
    }

    #[test]
    fn row_names_follow_the_master_file() {
        assert_eq!(row_name("unimplemented ptrace"), "ptrace");
        assert_eq!(row_name("obsolete 4.2 sigreturn"), "4.2 sigreturn");
        assert_eq!(row_name("unimplemented"), "");
        assert_eq!(
            row_name("__thrsleep (sys___thrsleep not ported)"),
            "__thrsleep"
        );
    }

    #[test]
    fn pinned_commit_reads_the_line() {
        assert_eq!(pinned_commit("a\nCommit: abc \n", "x").unwrap(), "abc");
        assert!(pinned_commit("nothing", "x").is_err());
    }

    #[test]
    fn include_set_follows_the_makefile() {
        let dir = std::env::temp_dir().join(format!("xtask-include-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let files = [
            "sys/param.h",
            "sys/notes.txt",
            "net/if.h",
            "netinet/in.h",
            "dev/ic/x.h",
            "dev/microcode/fw/f.h",
            "dev/pci/drm/d.h",
            "dev/pci/pcireg.h",
            "stand/other.h",
            "arch/arm64/include/param.h",
            "arch/arm64/include/sub/skip.h",
            "arch/arm64/include/notes.txt",
        ];
        for f in files {
            let p = dir.join(f);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, f).unwrap();
        }
        // The Makefile's LDIRS lists each directory; the fixture lacks most of them.
        for d in LDIRS {
            fs::create_dir_all(dir.join(d)).unwrap();
        }
        let got: Vec<String> = include_files(&dir)
            .unwrap()
            .into_iter()
            .map(|f| f.0)
            .collect();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            got,
            [
                "arm64/param.h",
                "dev/ic/x.h",
                "dev/pci/pcireg.h",
                "net/if.h",
                "netinet/in.h",
                "sys/param.h",
            ]
        );
    }
}
/* </TESTS> */
