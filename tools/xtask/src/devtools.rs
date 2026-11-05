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
//! `cargo xtask devtools-hello --arch A --llvm DIR --rust DIR`: the programs of the Kernel's
//! proof of DevTools' toolchains (DevTools' T1 and T2: "a C and a C++ hello compiled for the
//! target run under the EmiBSD kernel in QEMU", "a static Rust hello runs under the EmiBSD
//! kernel in QEMU"), and the ramdisk `just smoke-devtools` boots them from.
//!
//! `DIR`s are the unpacked releases `just toolchain` installs, used in place and unmodified:
//! the LLVM one (`bin/<triple>-clang`, `bin/<triple>-clang++` with the runtimes for the
//! triple) and the Rust one (`bin/rustc` with `std` for `<cpu>-emibsd-emibsd`). Each one's
//! `MANIFEST` must name the architecture (`arch:`) and the target (`targets:`); the clang
//! triple, release included (`aarch64-emibsd-emibsd8.0`), is read from it, not assumed.
//!
//! The sysroot is the Kernel's own userland build, `target/userland/<arch>/sysroot` (`just
//! userland`: OpenBSD's headers, crt and libc), the one DevTools built and checked its
//! runtimes against (its `EMIBSD_SYSROOT_ARM64`; DevTools' docs/TOOLCHAIN.md: "until [the
//! userland repository] exists, the base sets the kernel boots").
//!
//! Four programs, built the way DevTools' `llvm/tests/check.sh` and `rust/tests/check.sh`
//! build theirs, into `<run dir>/devtools/<arch>/` (`boot::run_dir`):
//!
//! - `hello-c`: C, `-static` (a static PIE over `rcrt0.o`), `strlcpy` of its first argument;
//! - `hello-cxx`: C++, `-static`, over libc++, libc++abi and libunwind: an exception thrown
//!   and caught, its message through iostreams;
//! - `hello-rust`: Rust, `+crt-static`, linked by the clang named after the triple: its
//!   arguments, a panic caught (the unwinder) and a locked `stdout`;
//! - `hello-c-dyn`: the C one linked dynamically, run by the ramdisk's `/usr/libexec/ld.so`
//!   and `libc.so`.
//!
//! Each command is printed before it runs; a failure is the command and its whole output,
//! which is what a report to DevTools needs (its request: "anything that breaks because of
//! the toolchain, reported ... with the command and the output rather than worked around").
//! Each ELF's header is read before the image is made: 64-bit little-endian, the OpenBSD
//! OS/ABI byte, the machine, a PIE (`ET_DYN`), and an interpreter only for the dynamic one.
//! Then `just userland`'s ramdisk is copied with the four in `/usr/bin`
//! (`userland::ramdisk_with`) into `ramdisk.ffs` beside them. Nothing is stripped: the
//! programs are what the drivers linked.
//!
//! amd64 waits for N10 (decision 44): with it off this prints `SKIPPED` and builds nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;
use crate::boot::{self, Arch};

/// The C hello: its first argument through `strlcpy`, as DevTools' `h.c`.
const HELLO_C: &str = r#"#include <stdio.h>
#include <string.h>

int
main(int argc, char **argv)
{
	char s[64];

	strlcpy(s, argc > 1 ? argv[1] : "none", sizeof s);
	printf("devtools: C hello, argv[1]=%s\n", s);
	return 0;
}
"#;

/// The C++ hello: an exception thrown and caught (libc++abi, libunwind), its message through
/// iostreams and `std::string` (libc++), as DevTools' `x.cpp`.
const HELLO_CXX: &str = r#"#include <iostream>
#include <stdexcept>
#include <string>

int
main()
{
	try {
		throw std::runtime_error("thrown and caught");
	} catch (const std::exception &e) {
		std::cout << "devtools: C++ hello, " << std::string(e.what()) << std::endl;
	}
	return 0;
}
"#;

/// The Rust hello: its arguments, a panic caught (the unwinder; the hook silenced so the
/// console shows only the line) and a locked `stdout`, as DevTools' `e.rs`.
const HELLO_RS: &str = r#"use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(|| panic!("unwound")).is_err();
    let mut out = std::io::stdout().lock();
    let written = writeln!(
        out,
        "devtools: Rust hello on {} ({} args, panic caught: {caught})",
        std::env::consts::OS,
        args.len()
    );
    std::process::exit(if written.is_ok() && caught { 0 } else { 1 });
}
"#;

/// The lines the programs print in `smoke-devtools`'s session (the justfile's
/// `devtools_steps` expects each; a test keeps the two in step).
pub(crate) const EXPECTED: [&str; 4] = [
    "devtools: C hello, argv[1]=from-c",
    "devtools: C++ hello, thrown and caught",
    "devtools: Rust hello on emibsd (2 args, panic caught: true)",
    "devtools: C hello, argv[1]=dynamic",
];

/// How one program is built.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    C,
    Cxx,
    Rust,
}

/// One program: its name (in `/usr/bin` on the ramdisk), its source, its language and
/// whether it is linked statically.
struct Program {
    name: &'static str,
    source: &'static str,
    lang: Lang,
    static_link: bool,
}

const PROGRAMS: [Program; 4] = [
    Program {
        name: "hello-c",
        source: "hello.c",
        lang: Lang::C,
        static_link: true,
    },
    Program {
        name: "hello-cxx",
        source: "hello.cpp",
        lang: Lang::Cxx,
        static_link: true,
    },
    Program {
        name: "hello-rust",
        source: "hello.rs",
        lang: Lang::Rust,
        static_link: true,
    },
    Program {
        name: "hello-c-dyn",
        source: "hello.c",
        lang: Lang::C,
        static_link: false,
    },
];

/// The sources, by file name.
const SOURCES: [(&str, &str); 3] = [
    ("hello.c", HELLO_C),
    ("hello.cpp", HELLO_CXX),
    ("hello.rs", HELLO_RS),
];

/// LLVM's and Rust's name of the architecture's CPU, the first part of every triple.
fn cpu(arch: Arch) -> &'static str {
    match arch {
        Arch::Amd64 => "x86_64",
        Arch::Arm64 => "aarch64",
    }
}

/// The value of `key:` in a DevTools `MANIFEST` (one `key: value` line each).
fn manifest_field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|l| {
        l.strip_prefix(key)
            .and_then(|r| r.strip_prefix(':'))
            .map(str::trim)
    })
}

/// The two toolchains of one architecture, as their manifests describe them.
#[derive(Debug)]
struct Toolchain {
    llvm: PathBuf,
    rust: PathBuf,
    /// LLVM's triple, release included (`aarch64-emibsd-emibsd8.0`): `bin/<triple>-clang`.
    clang_triple: String,
    /// Rust's target (`aarch64-emibsd-emibsd`).
    rust_target: String,
    /// `tag:` and `commit:` of each manifest, for the log.
    llvm_tag: String,
    rust_tag: String,
}

/// Reads the two manifests' texts: both must be for `arch`, the LLVM one must target a
/// `<cpu>-emibsd-emibsd<release>` triple, the Rust one `<cpu>-emibsd-emibsd`.
fn toolchain_from(
    arch: Arch,
    llvm: &Path,
    llvm_manifest: &str,
    rust: &Path,
    rust_manifest: &str,
) -> Result<Toolchain> {
    let check_arch = |what: &str, text: &str| -> Result<()> {
        match manifest_field(text, "arch") {
            Some(a) if a == arch.name() => Ok(()),
            other => Err(format!(
                "{what}/MANIFEST: arch {}, not {}",
                other.unwrap_or("missing"),
                arch.name()
            )
            .into()),
        }
    };
    check_arch(&llvm.display().to_string(), llvm_manifest)?;
    check_arch(&rust.display().to_string(), rust_manifest)?;
    let system = format!("{}-emibsd-emibsd", cpu(arch));
    let targets = |text| manifest_field(text, "targets").unwrap_or("");
    let clang_triple = targets(llvm_manifest)
        .split_whitespace()
        .find(|t| {
            t.strip_prefix(system.as_str()).is_some_and(|rel| {
                !rel.is_empty() && rel.chars().all(|c| c.is_ascii_digit() || c == '.')
            })
        })
        .ok_or_else(|| {
            format!(
                "{}/MANIFEST: no {system}<release> in targets",
                llvm.display()
            )
        })?;
    if !targets(rust_manifest)
        .split_whitespace()
        .any(|t| t == system)
    {
        return Err(format!("{}/MANIFEST: no {system} in targets", rust.display()).into());
    }
    let tag = |text| {
        format!(
            "{} ({})",
            manifest_field(text, "tag").unwrap_or("no tag"),
            manifest_field(text, "commit").unwrap_or("no commit")
        )
    };
    Ok(Toolchain {
        llvm: llvm.to_path_buf(),
        rust: rust.to_path_buf(),
        clang_triple: clang_triple.to_string(),
        rust_target: system,
        llvm_tag: tag(llvm_manifest),
        rust_tag: tag(rust_manifest),
    })
}

/// The program and arguments that build `p` from `src` into `out`, against `sysroot`.
fn build_command(
    tc: &Toolchain,
    p: &Program,
    sysroot: &Path,
    src: &Path,
    out: &Path,
) -> (PathBuf, Vec<String>) {
    let clang = tc
        .llvm
        .join("bin")
        .join(format!("{}-clang", tc.clang_triple));
    let sysroot_arg = format!("--sysroot={}", sysroot.display());
    let src = src.display().to_string();
    let out = out.display().to_string();
    match p.lang {
        Lang::C | Lang::Cxx => {
            let driver = if p.lang == Lang::Cxx {
                tc.llvm
                    .join("bin")
                    .join(format!("{}-clang++", tc.clang_triple))
            } else {
                clang
            };
            let mut args = vec![sysroot_arg];
            if p.static_link {
                args.push("-static".to_string());
            }
            args.extend(["-O2".to_string(), "-o".to_string(), out, src]);
            (driver, args)
        }
        Lang::Rust => {
            let mut args = vec![
                "--target".to_string(),
                tc.rust_target.clone(),
                "-C".to_string(),
                format!("linker={}", clang.display()),
                "-C".to_string(),
                format!("link-arg={sysroot_arg}"),
            ];
            if p.static_link {
                args.extend(["-C".to_string(), "target-feature=+crt-static".to_string()]);
            }
            args.extend(["-O".to_string(), "-o".to_string(), out, src]);
            (tc.rust.join("bin/rustc"), args)
        }
    }
}

/// `PT_INTERP`.
const PT_INTERP: u32 = 3;
/// `ELFOSABI_OPENBSD`: what the kernel and ld.so recognise.
const ELFOSABI_OPENBSD: u8 = 12;
/// `ET_DYN`: a position-independent executable.
const ET_DYN: u16 = 3;

/// `EM_X86_64` or `EM_AARCH64`.
fn elf_machine(arch: Arch) -> u16 {
    match arch {
        Arch::Amd64 => 62,
        Arch::Arm64 => 183,
    }
}

/// The header checks of one linked program (module docs): `Err` names the first one that
/// fails.
fn check_elf(bytes: &[u8], arch: Arch, static_link: bool) -> std::result::Result<(), String> {
    let u16_at = |o: usize| {
        bytes
            .get(o..o + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |o: usize| {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let u64_at = |o: usize| {
        bytes.get(o..o + 8).map(|b| {
            let mut a = [0; 8];
            a.copy_from_slice(b);
            u64::from_le_bytes(a)
        })
    };
    if bytes.get(..4) != Some(b"\x7fELF".as_slice()) {
        return Err("not an ELF file".into());
    }
    if bytes.get(4..6) != Some([2, 1].as_slice()) {
        return Err("not 64-bit little-endian".into());
    }
    if bytes.get(7) != Some(&ELFOSABI_OPENBSD) {
        return Err(format!(
            "OS/ABI byte {:?}, not {ELFOSABI_OPENBSD}",
            bytes.get(7)
        ));
    }
    if u16_at(16) != Some(ET_DYN) {
        return Err(format!("e_type {:?}, not ET_DYN (a PIE)", u16_at(16)));
    }
    if u16_at(18) != Some(elf_machine(arch)) {
        return Err(format!(
            "e_machine {:?}, not {}",
            u16_at(18),
            elf_machine(arch)
        ));
    }
    let (Some(phoff), Some(phentsize), Some(phnum)) = (u64_at(32), u16_at(54), u16_at(56)) else {
        return Err("truncated header".into());
    };
    let phoff = usize::try_from(phoff).map_err(|_| "e_phoff out of range")?;
    let mut interp = false;
    for i in 0..usize::from(phnum) {
        let p_type = u32_at(phoff + i * usize::from(phentsize))
            .ok_or_else(|| format!("program header {i} past the end of the file"))?;
        interp |= p_type == PT_INTERP;
    }
    match (static_link, interp) {
        (true, true) => Err("a static program with an interpreter (PT_INTERP)".into()),
        (false, false) => Err("a dynamic program without an interpreter (PT_INTERP)".into()),
        _ => Ok(()),
    }
}

/// The first line of `program args...`'s standard output, or why it has none.
fn first_line(program: &Path, args: &[&str]) -> String {
    match Command::new(program).args(args).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .to_string(),
        Ok(o) => format!("exit {}", o.status),
        Err(e) => e.to_string(),
    }
}

/// `cargo xtask devtools-hello` (module docs).
pub(crate) fn hello(root: &Path, arch: Arch, llvm: &Path, rust: &Path) -> Result<()> {
    if boot::skip_disabled("devtools-hello", arch)? {
        return Ok(());
    }
    let read = |p: PathBuf| fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()));
    let tc = toolchain_from(
        arch,
        llvm,
        &read(llvm.join("MANIFEST"))?,
        rust,
        &read(rust.join("MANIFEST"))?,
    )?;
    let userland = root.join("target/userland").join(arch.name());
    let sysroot = userland.join("sysroot");
    for p in ["usr/include/stdio.h", "usr/lib/rcrt0.o", "usr/lib/libc.a"] {
        if !sysroot.join(p).is_file() {
            return Err(format!(
                "{}: missing; run just userland first",
                sysroot.join(p).display()
            )
            .into());
        }
    }
    println!(
        "devtools-hello: {}: {} ({}), {}",
        arch.name(),
        tc.llvm_tag,
        first_line(&tc.llvm.join("bin/clang"), &["--version"]),
        tc.rust_tag,
    );
    println!(
        "devtools-hello: {}: {}, sysroot {}",
        arch.name(),
        first_line(&tc.rust.join("bin/rustc"), &["--version"]),
        sysroot.display()
    );

    let out = boot::run_dir(root).join("devtools").join(arch.name());
    fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    for (name, text) in SOURCES {
        let p = out.join(name);
        fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let mut built = Vec::new();
    for p in &PROGRAMS {
        let exe = out.join(p.name);
        let _ = fs::remove_file(&exe);
        let (program, args) = build_command(&tc, p, &sysroot, &out.join(p.source), &exe);
        println!("devtools-hello: $ {} {}", program.display(), args.join(" "));
        let o = Command::new(&program)
            .args(&args)
            .current_dir(&out)
            .output()
            .map_err(|e| format!("{}: {e}", program.display()))?;
        if !o.status.success() {
            return Err(format!(
                "devtools-hello: {} did not build ({}):\n$ {} {}\n{}{}",
                p.name,
                o.status,
                program.display(),
                args.join(" "),
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )
            .into());
        }
        let bytes = fs::read(&exe).map_err(|e| format!("{}: {e}", exe.display()))?;
        check_elf(&bytes, arch, p.static_link)
            .map_err(|e| format!("devtools-hello: {}: {e}", exe.display()))?;
        println!("devtools-hello: {} ok ({} bytes)", p.name, bytes.len());
        built.push((format!("/usr/bin/{}", p.name), exe));
    }
    let extra: Vec<_> = built
        .iter()
        .map(|(path, from)| crate::userland::ExtraProgram {
            path,
            from: from.as_path(),
        })
        .collect();
    let image = crate::userland::ramdisk_with(&userland, &out, &extra)?;
    let size = fs::metadata(&image).map(|m| m.len()).unwrap_or(0);
    println!(
        "devtools-hello: ramdisk {} ({size} bytes, just userland's with /usr/bin/{})",
        image.display(),
        PROGRAMS.map(|p| p.name).join(" /usr/bin/")
    );
    for line in EXPECTED {
        println!("devtools-hello: the session expects `{line}`");
    }
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    const LLVM_MANIFEST: &str = "name: llvm\nversion: 23.1.4+emibsd.1\nhost: aarch64-apple-darwin\n\
        arch: arm64\ntargets: aarch64-emibsd-emibsd8.0\ncommit: 83366eb9\ntag: llvm-23.1.4+emibsd.1\n";
    const RUST_MANIFEST: &str = "name: rust\nversion: 1.99.1+emibsd.1\nhost: aarch64-apple-darwin\n\
        arch: arm64\ntargets: aarch64-emibsd-emibsd aarch64-emibsd-none-softfloat\n\
        commit: 83366eb9\ntag: rust-1.99.1+emibsd.1\n";

    fn arm64_toolchain() -> Toolchain {
        toolchain_from(
            Arch::Arm64,
            Path::new("/t/llvm"),
            LLVM_MANIFEST,
            Path::new("/t/rust"),
            RUST_MANIFEST,
        )
        .expect("toolchain")
    }

    #[test]
    fn manifests_name_the_triples() {
        let tc = arm64_toolchain();
        assert_eq!(tc.clang_triple, "aarch64-emibsd-emibsd8.0");
        assert_eq!(tc.rust_target, "aarch64-emibsd-emibsd");
        assert_eq!(tc.llvm_tag, "llvm-23.1.4+emibsd.1 (83366eb9)");
        assert_eq!(
            manifest_field(LLVM_MANIFEST, "version"),
            Some("23.1.4+emibsd.1")
        );
        assert_eq!(manifest_field(LLVM_MANIFEST, "nothing"), None);
    }

    #[test]
    fn manifests_of_another_arch_or_target_are_refused() {
        let llvm = Path::new("/t/llvm");
        let rust = Path::new("/t/rust");
        let amd = LLVM_MANIFEST.replace("arch: arm64", "arch: amd64");
        let err = toolchain_from(Arch::Arm64, llvm, &amd, rust, RUST_MANIFEST).expect_err("arch");
        assert!(err.to_string().contains("arch amd64, not arm64"), "{err}");
        let bare = LLVM_MANIFEST.replace("aarch64-emibsd-emibsd8.0", "aarch64-emibsd-emibsd");
        assert!(toolchain_from(Arch::Arm64, llvm, &bare, rust, RUST_MANIFEST).is_err());
        let kernel_only = RUST_MANIFEST.replace("aarch64-emibsd-emibsd aarch64", "aarch64");
        assert!(toolchain_from(Arch::Arm64, llvm, LLVM_MANIFEST, rust, &kernel_only).is_err());
        assert!(toolchain_from(Arch::Amd64, llvm, LLVM_MANIFEST, rust, RUST_MANIFEST).is_err());
    }

    #[test]
    fn commands_are_check_sh_s() {
        let tc = arm64_toolchain();
        let sysroot = Path::new("/s");
        let cmd = |i: usize| {
            let p = &PROGRAMS[i];
            let (prog, args) =
                build_command(&tc, p, sysroot, Path::new("/o/src"), Path::new("/o/x"));
            (prog.display().to_string(), args.join(" "))
        };
        assert_eq!(
            cmd(0),
            (
                "/t/llvm/bin/aarch64-emibsd-emibsd8.0-clang".into(),
                "--sysroot=/s -static -O2 -o /o/x /o/src".into()
            )
        );
        assert_eq!(cmd(1).0, "/t/llvm/bin/aarch64-emibsd-emibsd8.0-clang++");
        assert_eq!(
            cmd(2),
            (
                "/t/rust/bin/rustc".into(),
                "--target aarch64-emibsd-emibsd -C linker=/t/llvm/bin/aarch64-emibsd-emibsd8.0-clang \
                 -C link-arg=--sysroot=/s -C target-feature=+crt-static -O -o /o/x /o/src"
                    .into()
            )
        );
        assert_eq!(cmd(3).1, "--sysroot=/s -O2 -o /o/x /o/src");
    }

    /// A minimal ELF64 header with `phnum` program headers of the types given.
    fn elf(osabi: u8, e_type: u16, machine: u16, types: &[u32]) -> Vec<u8> {
        let mut b = vec![0u8; 64 + 56 * types.len()];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        b[7] = osabi;
        b[16..18].copy_from_slice(&e_type.to_le_bytes());
        b[18..20].copy_from_slice(&machine.to_le_bytes());
        b[32..40].copy_from_slice(&64u64.to_le_bytes());
        b[54..56].copy_from_slice(&56u16.to_le_bytes());
        b[56..58].copy_from_slice(&u16::try_from(types.len()).expect("few").to_le_bytes());
        for (i, t) in types.iter().enumerate() {
            b[64 + 56 * i..68 + 56 * i].copy_from_slice(&t.to_le_bytes());
        }
        b
    }

    #[test]
    fn elf_headers_are_checked() {
        let load = 1;
        assert_eq!(
            check_elf(&elf(12, 3, 183, &[load]), Arch::Arm64, true),
            Ok(())
        );
        assert_eq!(
            check_elf(&elf(12, 3, 183, &[PT_INTERP, load]), Arch::Arm64, false),
            Ok(())
        );
        let bad = [
            (elf(0, 3, 183, &[load]), true, "OS/ABI"),
            (elf(12, 2, 183, &[load]), true, "ET_DYN"),
            (elf(12, 3, 62, &[load]), true, "e_machine"),
            (elf(12, 3, 183, &[PT_INTERP]), true, "static program with"),
            (elf(12, 3, 183, &[load]), false, "without an interpreter"),
            (b"#!/bin/sh\n".to_vec(), true, "not an ELF"),
        ];
        for (bytes, static_link, what) in bad {
            let err = check_elf(&bytes, Arch::Arm64, static_link).expect_err(what);
            assert!(err.contains(what), "{err}");
        }
        let mut short = elf(12, 3, 183, &[load]);
        short.truncate(66);
        let err = check_elf(&short, Arch::Arm64, true).expect_err("truncated");
        assert!(err.contains("past the end"), "{err}");
        short.truncate(40);
        assert!(check_elf(&short, Arch::Arm64, true).is_err());
    }

    #[test]
    fn sources_print_what_the_smoke_expects() {
        assert!(HELLO_C.contains("devtools: C hello, argv[1]=%s"));
        assert!(
            HELLO_CXX.contains("\"devtools: C++ hello, \"")
                && HELLO_CXX.contains("thrown and caught")
        );
        assert!(HELLO_RS.contains("devtools: Rust hello on {} ({} args, panic caught: {caught})"));
        let justfile =
            fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../justfile"))
                .expect("justfile");
        for line in EXPECTED {
            assert!(justfile.contains(&format!("--expect '{line}'")), "{line}");
        }
        assert_eq!(PROGRAMS.map(|p| p.name).len(), 4);
        for p in &PROGRAMS {
            assert!(SOURCES.iter().any(|(n, _)| *n == p.source), "{}", p.source);
        }
    }
}
/* </TESTS> */
