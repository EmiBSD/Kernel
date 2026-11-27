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
//! M14: OpenBSD's efiboot as the boot loader (`sys/boot/amd64`, `sys/boot/arm64`).
//!
//! - `cargo xtask efiboot --arch amd64 --elf FILE [--out FILE]`: the PE32+ image
//!   `BOOTX64.EFI` of the efiboot ELF that `just efiboot-amd64` linked (position-independent,
//!   at 0, with its PE header first: `ldscript.amd64`, `start_amd64.S`), made as arm64's
//!   efiboot makes its own: `llvm-objcopy -O binary` (OpenBSD's amd64 Makefile uses GNU
//!   objcopy's `efi-app-x86_64` target, which llvm-objcopy does not have). The default output
//!   is `target/efiboot/<arch>/BOOTX64.EFI`. llvm-objcopy is `$EMIBSD_LLVM_BIN`'s (DevTools'
//!   LLVM, which the justfile names), else the Rust toolchain's own when it has one.
//! - The efiboot image ([`image`], the one builder; the removal of Limine, 2026-11-26): a disk
//!   laid out as OpenBSD installs one that boots with efiboot: an MBR whose first partition is
//!   the OpenBSD one (`DOSPTYP_OPENBSD`, from sector 64; the disklabel in its sector 1, with
//!   the image's DUID, partition `a` an FFS, `c` the whole disk) and whose second is the EFI
//!   system partition (FAT, `EFI/BOOT/BOOT{X64,AA64}.EFI`). The FFS is made by OpenBSD's
//!   makefs, the host build `cargo xtask userland` leaves (`userland/ramdisk.rs`); the label
//!   and the MBR come from `hwopts.rs`, as `nvme-root`'s. Partition `a` always holds `/bsd`
//!   (stripped of its debug sections, as OpenBSD installs `bsd` and keeps `bsd.gdb`;
//!   `.symtab` stays, and ddb, which prints addresses alone here, shows the same; the
//!   unstripped build is `xtask symbolize`'s; `efiboot-disk` alone keeps the whole kernel),
//!   `/etc/boot.conf` and
//!   `/etc/random.seed`; [`Root`] says what else: nothing (install media), `/sbin/init` (the
//!   freestanding stand-in of `init/`, which the kernel's own `initpaths` loop execs), or a
//!   whole staged tree (`target/userland/<arch>/ramdisk-root` by default, with its ownership
//!   table `host/owners.txt`), whose fstab root line names the label's DUID
//!   (`454649424f4f5430.a / ffs rw 1 1`, as the installer writes it; diskmap(4) resolves it),
//!   or `/dev/<dev>` for the older `efiboot-disk --root-dev`. boot(8) passes the label's DUID
//!   (`BOOTARG_BOOTDUID`, arm64's `openbsd,bootduid`), by which the kernel finds its root.
//! - `cargo xtask efiboot-image --arch A --kernel K --root bare|init[=FILE]|tree[=DIR]
//!   [--owners FILE] [--duid HEX16] [--efi FILE] [--selftest WORD] (--shared | --out FILE)`:
//!   one such image. `--selftest WORD` writes `selftest=WORD` into the self-test preset of
//!   the image's copy of the kernel (`sys/bsd/kern/selftest.rs`, feature `qemu`), as
//!   rdsetroot(8) writes a ramdisk into `bsd.rd`: boot(8) passes the kernel only its flags,
//!   so the word Limine's command line carried has no other channel. `--shared` writes one of the two images every recipe may boot
//!   ([`Shared`], `target/efiboot/<arch>/emibsd-{tree,init}.img`, made by `just
//!   efiboot-images-arm64`); a VM attaches it with `snapshot=on`, so it is never written and
//!   is as fresh at each QEMU start as a ramdisk. `--out FILE` is a per-run image in the run
//!   directory (`EMIBSD_RUN_DIR`). The file is written whole under a temporary name and
//!   renamed, so a reader never sees half an image.
//! - `cargo xtask efiboot-disk --arch A --efi FILE --kernel FILE [--root-dev DEV]`: the
//!   image of `smoke-efiboot`, `smoke-acpi` and the installer's media (`install.rs`), where
//!   `smoke` without `--kernel` boots from (the run directory's `emibsd-<arch>.img`): a bare
//!   root, or with `--root-dev` the staged tree with its fstab root `/dev/DEV`. It is
//!   [`image`] with those arguments.

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::Result;
use crate::boot::{self, Arch};
use crate::{flag, optional_flag};

/// The bytes of a sector.
const SECTOR: u64 = 512;
/// The OpenBSD partition's first sector (and partition `a`'s), as `hwopts.rs` has it.
const OPENBSD_START: u64 = 64;
/// The disk is a whole number of MiB.
const ALIGN: u64 = 2048;
/// The EFI system partition's size: 8 MiB.
const ESP_SECTORS: u64 = 16384;
/// The DUID of the efiboot images' label (`EFIBOOT0` in ASCII, `454649424f4f5430`).
pub(crate) const DUID: [u8; 8] = *b"EFIBOOT0";

/// The magic of the kernel's self-test preset (`sys/bsd/kern/selftest.rs`, `PRESET`, feature
/// `qemu`): the same 16 bytes, which a host test checks against that file.
const PRESET_MAGIC: [u8; 16] = *b"EMIBSD-SELFTEST\0";
/// The preset's slot after the magic, which [`write_preset`] fills.
const PRESET_SLOT: usize = 48;

/// `boot.conf` on the disk: efiboot runs it before its prompt. It sets no `boot` command,
/// so the prompt comes, and turns the prompt's timeout off so that the smoke types at it.
const BOOT_CONF: &str = "set timeout 0\necho efiboot: boot.conf read\n";

/// The `.EFI` file name of `arch`'s efiboot.
fn efi_name(arch: Arch) -> &'static str {
    match arch {
        Arch::Amd64 => "BOOTX64.EFI",
        Arch::Arm64 => "BOOTAA64.EFI",
    }
}

/// `$EMIBSD_LLVM_BIN`'s llvm-objcopy (DevTools' LLVM), else the Rust toolchain's own
/// (`llvm-tools`, which DevTools' Rust does not carry).
fn llvm_objcopy() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("EMIBSD_LLVM_BIN") {
        let p = PathBuf::from(dir).join("llvm-objcopy");
        if p.is_file() {
            return Ok(p);
        }
    }
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let out = |args: &[&str]| -> Option<String> {
        let o = Command::new(&rustc).args(args).output().ok()?;
        o.status
            .success()
            .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    if let (Some(sysroot), Some(vv)) = (out(&["--print", "sysroot"]), out(&["-vV"]))
        && let Some(host) = vv.lines().find_map(|l| l.strip_prefix("host: "))
    {
        let p = Path::new(&sysroot)
            .join("lib/rustlib")
            .join(host)
            .join("bin/llvm-objcopy");
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(
        "llvm-objcopy not found: set EMIBSD_LLVM_BIN to the bin/ of DevTools' LLVM (the \
         justfile does; `just toolchain` installs it, docs/SETUP.md)"
            .into(),
    )
}

/// `cargo xtask efiboot`: the PE image of the efiboot ELF (module docs).
pub(crate) fn efiboot(root: &Path, arch: Arch, elf: &Path, out: Option<&str>) -> Result<()> {
    let out = match out {
        Some(o) => root.join(o),
        None => root.join(format!("target/efiboot/{}/{}", arch.name(), efi_name(arch))),
    };
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let objcopy = llvm_objcopy()?;
    let st = Command::new(&objcopy)
        .args(["-O", "binary"])
        .arg(elf)
        .arg(&out)
        .status()
        .map_err(|e| format!("{}: {e}", objcopy.display()))?;
    if !st.success() {
        return Err(format!("{} -O binary {} failed", objcopy.display(), elf.display()).into());
    }
    let image = fs::read(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    check_pe(&image).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("efiboot: {} ({} KiB)", out.display(), image.len() / 1024);
    Ok(())
}

/// The PE32+ image's sanity: the DOS and PE signatures, a PE32+ optional header, and a
/// `SizeOfImage` that the file holds whole.
fn check_pe(image: &[u8]) -> std::result::Result<(), String> {
    let le32 = |o: usize| {
        image
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if image.get(..2) != Some(b"MZ") {
        return Err("no MZ signature (is .peheader first?)".into());
    }
    let pe = le32(0x3c).ok_or("truncated")? as usize;
    if image.get(pe..pe + 4) != Some(b"PE\0\0") {
        return Err("no PE signature".into());
    }
    let opt = pe + 24;
    if image.get(opt..opt + 2) != Some(&[0x0b, 0x02]) {
        return Err("not PE32+".into());
    }
    let size_of_image = le32(opt + 56).ok_or("truncated")? as usize;
    if size_of_image != image.len() {
        return Err(format!(
            "SizeOfImage {size_of_image} != file size {}",
            image.len()
        ));
    }
    Ok(())
}

/// What partition `a` holds besides `/bsd`, `/etc/boot.conf` and `/etc/random.seed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Root {
    /// Nothing else: the installer's media (`/bsd` is then bsd.rd).
    Bare,
    /// `/sbin/init`: this file (the freestanding stand-in of `init/`).
    Init(PathBuf),
    /// A whole root: the staged tree `dir`, with its ownership table `owners` (the makefs
    /// shim's `mode uid gid path` lines).
    Tree { dir: PathBuf, owners: PathBuf },
}

/// One efiboot image ([`image`]).
#[derive(Clone, Debug)]
pub(crate) struct ImageSpec {
    pub arch: Arch,
    /// efiboot's PE image (`target/efiboot/<arch>/BOOT*.EFI`).
    pub efi: PathBuf,
    /// The kernel, copied to `/bsd`.
    pub kernel: PathBuf,
    pub root: Root,
    /// The label's DUID, which efiboot passes and the kernel finds its root by.
    pub duid: [u8; 8],
    /// With [`Root::Tree`]: `/dev/<dev>` as the fstab root (`efiboot-disk --root-dev`)
    /// instead of the DUID.
    pub fstab_dev: Option<String>,
    /// `/bsd` without its `.debug_*` sections (`llvm-objcopy --strip-debug`): every image
    /// but `efiboot-disk`'s, which keeps the disk its recipes always booted.
    pub strip_debug: bool,
    /// `--selftest WORD`: `selftest=WORD` written into the kernel's preset ([`write_preset`])
    /// in the image's copy of the kernel.
    pub selftest: Option<String>,
}

/// The two images every recipe may boot ([`shared_path`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shared {
    /// [`Root::Tree`] of the default staged tree.
    Tree,
    /// [`Root::Init`] of the default stand-in.
    Init,
}

impl Shared {
    /// The name `--image` and `--root` take.
    pub(crate) fn parse(name: &str) -> Result<Shared> {
        match name {
            "tree" => Ok(Shared::Tree),
            "init" => Ok(Shared::Init),
            other => Err(format!("`{other}`: the shared images are `tree` and `init`").into()),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Shared::Tree => "tree",
            Shared::Init => "init",
        }
    }
}

/// `target/efiboot/<arch>/emibsd-{tree,init}.img`: a shared image, made by `just
/// efiboot-images-<arch>` and booted read-only (`snapshot=on`) by any number of VMs.
pub(crate) fn shared_path(root: &Path, arch: Arch, which: Shared) -> PathBuf {
    root.join(format!(
        "target/efiboot/{}/emibsd-{}.img",
        arch.name(),
        which.name()
    ))
}

/// efiboot's PE image as `just efiboot-<arch>` leaves it.
pub(crate) fn default_efi(root: &Path, arch: Arch) -> PathBuf {
    root.join(format!("target/efiboot/{}/{}", arch.name(), efi_name(arch)))
}

/// The tree `just userland` stages, with its ownership table.
fn default_tree(root: &Path, arch: Arch) -> Root {
    Root::Tree {
        dir: root.join(format!("target/userland/{}/ramdisk-root", arch.name())),
        owners: root.join(format!("target/userland/{}/host/owners.txt", arch.name())),
    }
}

/// The stand-in `just build-init-<arch>` leaves.
fn default_init(root: &Path, arch: Arch) -> Result<Root> {
    boot::default_init(root, arch)
        .map(Root::Init)
        .ok_or_else(|| {
            format!(
                "no init stand-in in target/{}/debug/init: run `just build-init-{}` first",
                arch.target(),
                arch.name()
            )
            .into()
        })
}

/// `--root bare|init[=FILE]|tree[=DIR]` (with `--owners FILE` for a tree of its own).
pub(crate) fn parse_root(
    root: &Path,
    arch: Arch,
    spec: &str,
    owners: Option<&str>,
) -> Result<Root> {
    let (name, path) = match spec.split_once('=') {
        Some((n, p)) if !p.is_empty() => (n, Some(PathBuf::from(p))),
        Some(_) => return Err(format!("--root {spec}: a path after `=`").into()),
        None => (spec, None),
    };
    if owners.is_some() && !(name == "tree" && path.is_some()) {
        return Err("--owners FILE goes with --root tree=DIR".into());
    }
    match (name, path) {
        ("bare", None) => Ok(Root::Bare),
        ("init", None) => default_init(root, arch),
        ("init", Some(file)) => Ok(Root::Init(file)),
        ("tree", None) => Ok(default_tree(root, arch)),
        ("tree", Some(dir)) => match owners {
            Some(o) => Ok(Root::Tree {
                dir,
                owners: PathBuf::from(o),
            }),
            None => Err(
                format!("--root {spec}: needs --owners FILE, the tree's ownership table").into(),
            ),
        },
        _ => Err(format!("--root {spec}: expected bare, init[=FILE] or tree[=DIR]").into()),
    }
}

/// The DUID as `disklabel(8)` and fstab(5) write it: 16 lowercase hexadecimal digits.
pub(crate) fn duid_hex(duid: [u8; 8]) -> String {
    duid.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bytes of the regular files under `dir` (symbolic links not followed).
fn tree_bytes(dir: &Path) -> Result<u64> {
    let mut total = 0;
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.is_dir() {
            total += tree_bytes(&entry.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

/// What the root's fstab line names.
#[derive(Clone, Copy, Debug)]
enum FstabRoot<'a> {
    /// `<duid>.a`, as the installer writes it.
    Duid([u8; 8]),
    /// `/dev/<dev>` (`efiboot-disk --root-dev`).
    Dev(&'a str),
}

/// The staged tree's fstab with its root line (`/dev/rd0a /`) naming `to` instead.
fn root_fstab(fstab: &str, to: FstabRoot<'_>) -> Result<String> {
    let spec = match to {
        FstabRoot::Duid(duid) => format!("{}.a", duid_hex(duid)),
        FstabRoot::Dev(dev) => {
            if dev.is_empty() || !dev.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(
                    format!("--root-dev {dev}: expected a device name such as sd1a").into(),
                );
            }
            format!("/dev/{dev}")
        }
    };
    let mut seen = false;
    let lines: Vec<String> = fstab
        .lines()
        .map(|l| match l.strip_prefix("/dev/rd0a / ") {
            Some(rest) => {
                seen = true;
                format!("{spec} / {rest}")
            }
            None => l.to_string(),
        })
        .collect();
    if !seen {
        return Err("the staged tree's fstab has no `/dev/rd0a /` root line".into());
    }
    Ok(lines.join("\n") + "\n")
}

/// Copies the kernel to `to`, without its debug sections when `strip`.
fn stage_kernel(kernel: &Path, to: &Path, strip: bool) -> Result<()> {
    if !strip {
        fs::copy(kernel, to).map_err(|e| format!("{}: {e}", kernel.display()))?;
        return Ok(());
    }
    let objcopy = llvm_objcopy()?;
    let st = Command::new(&objcopy)
        .arg("--strip-debug")
        .arg(kernel)
        .arg(to)
        .status()
        .map_err(|e| format!("{}: {e}", objcopy.display()))?;
    if !st.success() {
        return Err(format!(
            "{} --strip-debug {} failed",
            objcopy.display(),
            kernel.display()
        )
        .into());
    }
    Ok(())
}

/// Writes `selftest=WORD` into the self-test preset of `kernel` (the bytes of the kernel
/// file), as rdsetroot(8) writes a ramdisk into `bsd.rd`: the file must hold exactly one
/// [`PRESET_MAGIC`] followed by an empty (zero) slot, and the word is lowercase letters and
/// digits, `selftest=WORD` at most [`PRESET_SLOT`] bytes (NUL padded). A kernel built without
/// feature `qemu` has no preset.
fn write_preset(kernel: &mut [u8], word: &str) -> std::result::Result<(), String> {
    if word.is_empty()
        || !word
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return Err(format!(
            "--selftest {word}: a self-test name such as trap or fb"
        ));
    }
    let text = format!("selftest={word}");
    if text.len() > PRESET_SLOT {
        return Err(format!(
            "--selftest {word}: longer than the preset's {PRESET_SLOT} bytes"
        ));
    }
    let at: Vec<usize> = kernel
        .windows(PRESET_MAGIC.len())
        .enumerate()
        .filter(|(_, w)| *w == PRESET_MAGIC)
        .map(|(i, _)| i)
        .collect();
    let [i] = at[..] else {
        return Err(format!(
            "{} copies of the self-test preset's magic, not one (a kernel built without \
             feature qemu has none)",
            at.len()
        ));
    };
    let start = i + PRESET_MAGIC.len();
    let slot = kernel
        .get_mut(start..start + PRESET_SLOT)
        .ok_or("the self-test preset's slot runs past the file's end")?;
    if slot.iter().any(|&b| b != 0) {
        return Err("the self-test preset's slot is not empty".into());
    }
    slot[..text.len()].copy_from_slice(text.as_bytes());
    Ok(())
}

/// Writes the FFS of partition `a` in `work` and returns it: `/bsd`, `/etc/boot.conf`,
/// `/etc/random.seed` and what [`ImageSpec::root`] adds.
fn make_ffs(root: &Path, spec: &ImageSpec, work: &Path) -> Result<Vec<u8>> {
    let arch = spec.arch;
    let makefs = root.join(format!("target/userland/{}/host/bin/makefs", arch.name()));
    if !makefs.is_file() {
        return Err(format!(
            "{}: no makefs; run `just userland` first (it builds OpenBSD's makefs for the host)",
            makefs.display()
        )
        .into());
    }
    let staging = work.join("root");
    let _ = fs::remove_dir_all(&staging);
    let mut owners_text = String::new();
    match &spec.root {
        Root::Bare => {}
        Root::Init(file) => {
            fs::create_dir_all(staging.join("sbin"))
                .map_err(|e| format!("{}: {e}", staging.display()))?;
            fs::copy(file, staging.join("sbin/init"))
                .map_err(|e| format!("{}: {e}", file.display()))?;
            owners_text.push_str("0755 0 0 /sbin\n0555 0 0 /sbin/init\n");
        }
        Root::Tree { dir, owners } => {
            if !dir.join("etc/fstab").is_file() {
                return Err(format!(
                    "{}: no staged root; run `just userland` first",
                    dir.display()
                )
                .into());
            }
            // cp -pR copies symbolic links as links and keeps the modes.
            let status = Command::new("cp")
                .arg("-pR")
                .arg(dir)
                .arg(&staging)
                .status()
                .map_err(|e| format!("cp: {e}"))?;
            if !status.success() {
                return Err(format!("cp -pR {} failed", dir.display()).into());
            }
            let fstab = staging.join("etc/fstab");
            let text =
                fs::read_to_string(&fstab).map_err(|e| format!("{}: {e}", fstab.display()))?;
            let to = match spec.fstab_dev.as_deref() {
                Some(dev) => FstabRoot::Dev(dev),
                None => FstabRoot::Duid(spec.duid),
            };
            fs::write(&fstab, root_fstab(&text, to)?)?;
            owners_text =
                fs::read_to_string(owners).map_err(|e| format!("{}: {e}", owners.display()))?;
        }
    }
    fs::create_dir_all(staging.join("etc")).map_err(|e| format!("{}: {e}", staging.display()))?;
    let bsd = staging.join("bsd");
    stage_kernel(&spec.kernel, &bsd, spec.strip_debug)?;
    if let Some(word) = &spec.selftest {
        let mut k = fs::read(&bsd).map_err(|e| format!("{}: {e}", bsd.display()))?;
        write_preset(&mut k, word).map_err(|e| format!("{}: {e}", spec.kernel.display()))?;
        fs::write(&bsd, k)?;
    }
    fs::write(staging.join("etc/boot.conf"), BOOT_CONF)?;
    // A seed boot(8) can read (and marks used, sticky bit, as on a real install).
    let seed: Vec<u8> = (0..256u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    fs::write(staging.join("etc/random.seed"), seed)?;
    let owners = work.join("owners.txt");
    owners_text.push_str(
        "0755 0 0 /etc\n0555 0 0 /bsd\n0644 0 0 /etc/boot.conf\n0600 0 0 /etc/random.seed\n",
    );
    fs::write(&owners, owners_text)?;
    // makefs's own estimate leaves no room for the indirect blocks of a big kernel: a tenth
    // more and 4 MiB, in whole MiB; a root gets room to write besides (twice its size).
    const MIB: u64 = 1 << 20;
    let kernel_len = fs::metadata(&bsd).map(|m| m.len()).unwrap_or(0);
    let tree_len = match spec.root {
        Root::Bare => 0,
        Root::Init(_) | Root::Tree { .. } => tree_bytes(&staging)? - kernel_len,
    };
    let size = (kernel_len + kernel_len / 10 + 2 * tree_len + 4 * MIB).div_ceil(MIB) * MIB;
    let image = work.join("a.ffs");
    let _ = fs::remove_file(&image);
    let out = Command::new(&makefs)
        .args([
            "-t",
            "ffs",
            "-o",
            "version=1,minfree=0,density=4096",
            "-T",
            "1790985600",
        ])
        .args(["-s", &size.to_string()])
        .env("EMIBSD_OWNERS", &owners)
        .env("EMIBSD_STAGING", &staging)
        .arg(&image)
        .arg(&staging)
        .output()
        .map_err(|e| format!("{}: {e}", makefs.display()))?;
    if !out.status.success() {
        return Err(format!(
            "makefs failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
        .into());
    }
    fs::read(&image).map_err(|e| format!("{}: {e}", image.display()).into())
}

/// The FAT system partition holding `EFI/BOOT/<efi_name>`.
fn make_esp(arch: Arch, efi: &[u8]) -> Result<Vec<u8>> {
    let mut part = Cursor::new(vec![0u8; (ESP_SECTORS * SECTOR) as usize]);
    fatfs::format_volume(
        &mut part,
        fatfs::FormatVolumeOptions::new().volume_label(*b"EFIBOOT    "),
    )?;
    {
        let fs = fatfs::FileSystem::new(&mut part, fatfs::FsOptions::new())?;
        {
            let dir = fs.root_dir().create_dir("EFI")?.create_dir("BOOT")?;
            dir.create_file(efi_name(arch))?.write_all(efi)?;
        }
        fs.unmount()?;
    }
    Ok(part.into_inner())
}

/// The disk of `ffs` (partition `a`) and `esp`: the MBR, the label with `duid`, the two
/// partitions.
fn assemble(ffs: &[u8], esp: &[u8], duid: [u8; 8]) -> Vec<u8> {
    let fs_sectors = (ffs.len() as u64).div_ceil(SECTOR);
    let esp_start = (OPENBSD_START + fs_sectors).div_ceil(ALIGN) * ALIGN;
    let total = esp_start + ESP_SECTORS;
    let mut disk = vec![0u8; (total * SECTOR) as usize];

    // The MBR: hwopts's one OpenBSD partition, and the EFI system partition beside it.
    let mut mbr = crate::hwopts::mbr(OPENBSD_START, esp_start - OPENBSD_START);
    mbr[446] = 0; // only the ESP is "active" for the firmware's sake
    let e = &mut mbr[462..478];
    e[1..4].copy_from_slice(&[0xfe, 0xff, 0xff]);
    e[4] = 0xef; // DOSPTYP_EFISYS
    e[5..8].copy_from_slice(&[0xfe, 0xff, 0xff]);
    e[8..12].copy_from_slice(&(esp_start as u32).to_le_bytes());
    e[12..16].copy_from_slice(&(ESP_SECTORS as u32).to_le_bytes());
    disk[..512].copy_from_slice(&mbr);

    let fs_at = (OPENBSD_START * SECTOR) as usize;
    disk[fs_at..fs_at + ffs.len()].copy_from_slice(ffs);
    // The label goes in the file system's boot area, which ffs leaves free (`LABELSECTOR`).
    let label_at = ((OPENBSD_START + 1) * SECTOR) as usize;
    disk[label_at..label_at + 512]
        .copy_from_slice(&crate::hwopts::disklabel(total, fs_sectors, duid));

    let esp_at = (esp_start * SECTOR) as usize;
    disk[esp_at..esp_at + esp.len()].copy_from_slice(esp);
    disk
}

/// Writes the efiboot image `spec` describes to `out` (module docs): staged in
/// `<out>.work/`, written whole to `<out>.tmp`, then renamed.
pub(crate) fn image(root: &Path, spec: &ImageSpec, out: &Path) -> Result<()> {
    let started = Instant::now();
    let efi_bytes = fs::read(&spec.efi).map_err(|e| {
        format!(
            "{}: {e} (run `just efiboot-{}` first)",
            spec.efi.display(),
            spec.arch.name()
        )
    })?;
    let dir = out
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("target"));
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{}: not a file path", out.display()))?;
    // A pid suffix: two runs that write the same image never share a staging file.
    let pid = std::process::id();
    let work = dir.join(format!("{name}.work.{pid}"));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;

    let ffs = make_ffs(root, spec, &work)?;
    let kernel_kib = fs::metadata(work.join("root/bsd")).map_or(0, |m| m.len() / 1024);
    let _ = fs::remove_dir_all(work.join("root"));
    let esp = make_esp(spec.arch, &efi_bytes)?;
    let disk = assemble(&ffs, &esp, spec.duid);
    let tmp = dir.join(format!("{name}.tmp.{pid}"));
    fs::write(&tmp, &disk).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, out).map_err(|e| format!("{} -> {}: {e}", tmp.display(), out.display()))?;
    let _ = fs::remove_dir_all(&work);
    let what = match &spec.root {
        Root::Bare => String::new(),
        Root::Init(file) => format!(", /sbin/init from {}", file.display()),
        Root::Tree { dir, .. } => format!(
            ", the tree {} with fstab root {}",
            dir.display(),
            match &spec.fstab_dev {
                Some(d) => format!("/dev/{d}"),
                None => format!("{}.a", duid_hex(spec.duid)),
            }
        ),
    };
    println!(
        "efiboot-image: {} ({} sectors, DUID {}; a = ffs of {} sectors with /bsd ({kernel_kib} \
         KiB{}{}){what}, /etc/boot.conf, /etc/random.seed; ESP with EFI/BOOT/{} ({} KiB); \
         made in {:.1}s)",
        out.display(),
        disk.len() as u64 / SECTOR,
        duid_hex(spec.duid),
        (ffs.len() as u64).div_ceil(SECTOR),
        if spec.strip_debug {
            ", debug sections stripped"
        } else {
            ""
        },
        spec.selftest
            .as_ref()
            .map(|w| format!(", preset selftest={w}"))
            .unwrap_or_default(),
        efi_name(spec.arch),
        efi_bytes.len() / 1024,
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

/// The image `args` describe, as `efiboot-image` and a per-run boot (`--kernel K --root ...`)
/// spell it: `--kernel`, `--root` (with `--owners`), `--duid` (the default [`DUID`]),
/// `--selftest`; the default efiboot of `arch`; the kernel always stripped of its debug
/// sections (ddb prints addresses alone, with or without them).
fn spec_from_args(root: &Path, arch: Arch, args: &[&str]) -> Result<ImageSpec> {
    let duid = optional_flag(args, "--duid")
        .map(crate::hwopts::parse_duid)
        .transpose()?
        .unwrap_or(DUID);
    Ok(ImageSpec {
        arch,
        efi: default_efi(root, arch),
        kernel: PathBuf::from(flag(args, "--kernel")?),
        root: parse_root(
            root,
            arch,
            flag(args, "--root")?,
            optional_flag(args, "--owners"),
        )?,
        duid,
        fstab_dev: None,
        strip_debug: true,
        selftest: optional_flag(args, "--selftest").map(str::to_string),
    })
}

/// `cargo xtask efiboot-image` (module docs).
pub(crate) fn image_command(root: &Path, args: &[&str]) -> Result<()> {
    let arch = Arch::parse(flag(args, "--arch")?)?;
    if boot::skip_disabled("efiboot-image", arch)? {
        return Ok(());
    }
    let mut spec = spec_from_args(root, arch, args)?;
    if let Some(efi) = optional_flag(args, "--efi") {
        spec.efi = PathBuf::from(efi);
    }
    let out = match (args.contains(&"--shared"), optional_flag(args, "--out")) {
        (true, None) => {
            // The shared images are the standard ones: the default tree or stand-in, the
            // default DUID (what `--image` and `--expect-root` assume), no preset.
            let which = Shared::parse(flag(args, "--root")?)?;
            if optional_flag(args, "--duid").is_some() || spec.selftest.is_some() {
                return Err(
                    "--shared images carry the default DUID and no preset: no --duid, no \
                     --selftest"
                        .into(),
                );
            }
            shared_path(root, arch, which)
        }
        (false, Some(o)) => boot::run_dir(root).join(o),
        _ => return Err("efiboot-image: exactly one of --shared and --out FILE".into()),
    };
    image(root, &spec, &out)
}

/// `cargo xtask efiboot-disk`: the boot image of `smoke-efiboot` (module docs).
pub(crate) fn efiboot_disk(
    root: &Path,
    arch: Arch,
    efi: &Path,
    kernel: &Path,
    root_dev: Option<&str>,
) -> Result<()> {
    let spec_root = match root_dev {
        Some(_) => default_tree(root, arch),
        None => Root::Bare,
    };
    image(
        root,
        &ImageSpec {
            arch,
            efi: efi.to_path_buf(),
            kernel: kernel.to_path_buf(),
            root: spec_root,
            duid: DUID,
            fstab_dev: root_dev.map(str::to_string),
            strip_debug: false,
            selftest: None,
        },
        &boot::image_path(root, arch, None),
    )
}

/// The image `bench` and `diff-openbsd` boot EmiBSD from: the default staged tree with
/// `kernel` at `/bsd` (stripped of its debug sections), the default DUID, no preset, at
/// `out`; attached with `snapshot=on` (set here for the VMs this run starts) by every VM of
/// the run, so each boot sees it as made. `kernel` is their own (`--profile release`,
/// `diff-openbsd --kernel-dir`), so the shared image of `just efiboot-images-<arch>` is not
/// theirs.
pub(crate) fn tree_image(root: &Path, arch: Arch, kernel: &Path, out: &Path) -> Result<()> {
    if arch != Arch::Arm64 {
        return Err("an efiboot boot is arm64's until N10's first step enables amd64's".into());
    }
    image(
        root,
        &ImageSpec {
            arch,
            efi: default_efi(root, arch),
            kernel: kernel.to_path_buf(),
            root: default_tree(root, arch),
            duid: DUID,
            fstab_dev: None,
            strip_debug: true,
            selftest: None,
        },
        out,
    )?;
    boot::set_boot_medium(boot::BootMedium::Disk);
    boot::set_boot_snapshot(true);
    Ok(())
}

/// The prompt boot(8) prints when `boot.conf` set no `boot` command (`BOOT_CONF`).
pub(crate) const BOOT_PROMPT: &str = "boot> ";

/// What a boot without `--boot-flags` types at [`BOOT_PROMPT`].
pub(crate) const BOOT_LINE: &str = "boot\n";

/// The options of `smoke`, `smoke2` and `qemu` that are only an efiboot boot's, and the
/// Limine ones an efiboot boot refuses (no command line, no modules).
const EFIBOOT_ONLY: &[&str] = &["--boot-flags", "--boot-on", "--selftest", "--owners"];
const EFIBOOT_ONLY_SWITCHES: &[&str] = &["--boot-manual", "--expect-root"];
const LIMINE_ONLY: &[&str] = &["--cmdline", "--init", "--ramdisk"];
const LIMINE_ONLY_SWITCHES: &[&str] = &["--expect-ramdisk"];

/// How a boot goes through efiboot: `--image tree|init` (a [`Shared`] image) or `--kernel K
/// --root ...` (a per-run image, [`image`], in the run directory).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EfiBoot {
    /// The image the VM attaches with `snapshot=on` (never written).
    pub image: PathBuf,
    /// What xtask types at every `boot> ` (`boot`, with `--boot-flags`' flags); `None` with
    /// `--boot-manual`, where the recipe's own sends type there.
    pub boot_line: Option<String>,
    /// `--expect-root`: the root line's tail, `(<duid>.a) swap on`: the kernel found its root
    /// by the DUID efiboot passed.
    pub expect_root: Option<String>,
}

/// The line typed at `boot> ` for `--boot-flags FLAGS` (boot(8)'s own `-a`, `-c`, `-d`,
/// `-s`: `boot -s`); plain `boot` without them.
fn boot_line(flags: Option<&str>) -> Result<String> {
    match flags {
        None => Ok(BOOT_LINE.to_string()),
        Some(f) => {
            let ok = f
                .strip_prefix('-')
                .is_some_and(|rest| !rest.is_empty() && rest.bytes().all(|b| b"acds".contains(&b)));
            if !ok {
                return Err(format!(
                    "--boot-flags {f}: boot(8)'s flags, `-` and some of a, c, d, s"
                )
                .into());
            }
            Ok(format!("boot {f}\n"))
        }
    }
}

/// `--boot-on nvme`: the disk the image is on ([`boot::BootMedium`]); the virtio disk
/// without it. EDK2 AArch64 has no AHCI driver, so no image boots from AHCI: smoke-ahci
/// boots the virtio image (`--image tree`) and its AHCI disk is a data disk, since the
/// kernel's `boot -a` has no `root device:` prompt to name an AHCI root at (setroot's
/// RB_ASKNAME dialogue is unported, LZ's gap, sys/kernel/dev/disk.rs).
fn boot_medium(args: &[&str]) -> Result<boot::BootMedium> {
    match optional_flag(args, "--boot-on") {
        None => Ok(boot::BootMedium::Disk),
        Some("nvme") if optional_flag(args, "--nvme").is_none() => Ok(boot::BootMedium::Nvme),
        Some("nvme") => {
            Err("--boot-on nvme and --nvme FILE: the boot disk is the NVMe namespace".into())
        }
        Some(other) => Err(format!("--boot-on {other}: expected nvme").into()),
    }
}

/// The efiboot boot `args` ask for (`--image` or `--root`), or `None` for a Limine boot or
/// an existing image; with `--root` the per-run image is made here, at the run directory's
/// `emibsd-<arch>.img`, once every other option has been checked. Sets the boot medium
/// (`--boot-on nvme`) and `snapshot=on` for the VMs this run starts.
pub(crate) fn boot_args(root: &Path, arch: Arch, args: &[&str]) -> Result<Option<EfiBoot>> {
    let shared = optional_flag(args, "--image");
    let root_spec = optional_flag(args, "--root");
    if shared.is_none() && root_spec.is_none() {
        let stray: Vec<&str> = EFIBOOT_ONLY
            .iter()
            .filter(|o| optional_flag(args, o).is_some())
            .chain(EFIBOOT_ONLY_SWITCHES.iter().filter(|o| args.contains(o)))
            .copied()
            .collect();
        if !stray.is_empty() {
            return Err(format!(
                "{}: an efiboot boot's, with --image tree|init or --kernel K --root ...",
                stray.join(", ")
            )
            .into());
        }
        return Ok(None);
    }
    let limine: Vec<&str> = LIMINE_ONLY
        .iter()
        .filter(|o| optional_flag(args, o).is_some())
        .chain(LIMINE_ONLY_SWITCHES.iter().filter(|o| args.contains(o)))
        .copied()
        .collect();
    if !limine.is_empty() {
        return Err(format!(
            "{}: a Limine boot's; boot(8) passes only its flags (--boot-flags), init is \
             /sbin/init on the root, the root is the image's partition a",
            limine.join(", ")
        )
        .into());
    }
    if arch != Arch::Arm64 {
        return Err("an efiboot boot is arm64's until N10's first step enables amd64's".into());
    }
    let boot_line = if args.contains(&"--boot-manual") {
        if optional_flag(args, "--boot-flags").is_some() {
            return Err(
                "--boot-manual and --boot-flags: the recipe types its own boot line".into(),
            );
        }
        None
    } else {
        Some(boot_line(optional_flag(args, "--boot-flags"))?)
    };
    let medium = boot_medium(args)?;
    let (image_path, duid) = match (shared, root_spec) {
        (Some(_), Some(_)) => return Err("--image and --root: one or the other".into()),
        (Some(name), None) => {
            let fixed: Vec<&str> = ["--kernel", "--duid", "--selftest", "--owners"]
                .into_iter()
                .filter(|o| optional_flag(args, o).is_some())
                .collect();
            if !fixed.is_empty() {
                return Err(format!(
                    "--image {name}: a shared image is made by `just efiboot-images-{}`; {} \
                     go with --kernel K --root ...",
                    arch.name(),
                    fixed.join(", ")
                )
                .into());
            }
            let p = shared_path(root, arch, Shared::parse(name)?);
            if !p.is_file() {
                return Err(format!(
                    "{} does not exist; run `just efiboot-images-{}`",
                    p.display(),
                    arch.name()
                )
                .into());
            }
            (p, DUID)
        }
        (None, Some(_)) => {
            let spec = spec_from_args(root, arch, args)?;
            let out = boot::image_path(root, arch, None);
            image(root, &spec, &out)?;
            (out, spec.duid)
        }
        // Returned above.
        (None, None) => return Ok(None),
    };
    boot::set_boot_medium(medium);
    boot::set_boot_snapshot(true);
    Ok(Some(EfiBoot {
        image: image_path,
        boot_line,
        expect_root: args
            .contains(&"--expect-root")
            .then(|| format!("({}.a) swap on", duid_hex(duid))),
    }))
}

/// Types the boot line once at each `boot> ` that appears on a serial transcript (the
/// prompt of every boot, a `--reboot`'s second one included). It keeps the offset where the
/// next prompt may start, so each poll scans only the bytes that arrived since the last one.
#[derive(Clone, Debug)]
pub(crate) struct BootTypist {
    line: String,
    /// Where in the transcript the next prompt is looked for: past the last one answered,
    /// or where a prompt cut by the end of what had arrived may begin.
    from: usize,
}

impl BootTypist {
    pub(crate) fn new(line: &str) -> BootTypist {
        BootTypist {
            line: line.to_string(),
            from: 0,
        }
    }

    /// The line [`poll`](Self::poll) asks to type.
    pub(crate) fn line(&self) -> &str {
        &self.line
    }

    /// Whether `transcript` (the whole serial output so far) shows a prompt not answered
    /// yet: then the line is to be typed now. One prompt per call.
    pub(crate) fn poll(&mut self, transcript: &[u8]) -> bool {
        let prompt = BOOT_PROMPT.as_bytes();
        let from = self.from.min(transcript.len());
        match transcript[from..]
            .windows(prompt.len())
            .position(|w| w == prompt)
        {
            Some(i) => {
                self.from = from + i + prompt.len();
                true
            }
            None => {
                self.from = transcript.len().saturating_sub(prompt.len() - 1).max(from);
                false
            }
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_fstab_renames_the_root_line_only() {
        let fstab = "/dev/rd0a / ffs rw 1 1\n/dev/sd0a /mnt ffs rw,userquota,noauto 1 2\n";
        assert_eq!(
            root_fstab(fstab, FstabRoot::Dev("sd1a")).unwrap(),
            "/dev/sd1a / ffs rw 1 1\n/dev/sd0a /mnt ffs rw,userquota,noauto 1 2\n"
        );
        assert!(root_fstab("/dev/sd0a /mnt ffs rw 1 2\n", FstabRoot::Dev("sd1a")).is_err());
        assert!(root_fstab(fstab, FstabRoot::Dev("sd1a /x")).is_err());
    }

    #[test]
    fn root_fstab_names_the_duid_as_the_installer_writes_it() {
        let fstab = "/dev/rd0a / ffs rw 1 1\n/dev/sd0a /mnt ffs rw,userquota,noauto 1 2\n";
        assert_eq!(
            root_fstab(fstab, FstabRoot::Duid(DUID)).unwrap(),
            "454649424f4f5430.a / ffs rw 1 1\n/dev/sd0a /mnt ffs rw,userquota,noauto 1 2\n"
        );
        assert_eq!(duid_hex(*b"NVMEROOT"), "4e564d45524f4f54");
        assert!(root_fstab("/dev/sd0a / ffs rw 1 1\n", FstabRoot::Duid(DUID)).is_err());
    }

    #[test]
    fn root_spec_names_bare_init_and_tree() {
        let r = Path::new("/w");
        let a = Arch::Arm64;
        assert_eq!(parse_root(r, a, "bare", None).unwrap(), Root::Bare);
        assert_eq!(
            parse_root(r, a, "init=/x/init", None).unwrap(),
            Root::Init(PathBuf::from("/x/init"))
        );
        assert_eq!(
            parse_root(r, a, "tree", None).unwrap(),
            Root::Tree {
                dir: PathBuf::from("/w/target/userland/arm64/ramdisk-root"),
                owners: PathBuf::from("/w/target/userland/arm64/host/owners.txt"),
            }
        );
        assert_eq!(
            parse_root(r, a, "tree=/t", Some("/t.owners")).unwrap(),
            Root::Tree {
                dir: PathBuf::from("/t"),
                owners: PathBuf::from("/t.owners"),
            }
        );
        assert!(parse_root(r, a, "tree=/t", None).is_err());
        assert!(parse_root(r, a, "tree", Some("/o")).is_err());
        assert!(parse_root(r, a, "bare=/x", None).is_err());
        assert!(parse_root(r, a, "init=", None).is_err());
        assert!(parse_root(r, a, "ramdisk", None).is_err());
        assert_eq!(Shared::parse("tree").unwrap(), Shared::Tree);
        assert!(Shared::parse("bare").is_err());
        assert_eq!(
            shared_path(r, a, Shared::Init),
            PathBuf::from("/w/target/efiboot/arm64/emibsd-init.img")
        );
    }

    #[test]
    fn the_disk_holds_the_label_with_its_duid_and_both_partitions() {
        let ffs = vec![0xa5u8; 3 * SECTOR as usize];
        let esp = vec![0x5au8; (ESP_SECTORS * SECTOR) as usize];
        let disk = assemble(&ffs, &esp, DUID);
        assert_eq!(disk.len() as u64, (ALIGN + ESP_SECTORS) * SECTOR);
        assert_eq!(&disk[510..512], &[0x55, 0xaa]);
        assert_eq!(disk[462 + 4], 0xef);
        let fs_at = (OPENBSD_START * SECTOR) as usize;
        assert_eq!(disk[fs_at], 0xa5);
        let label = (OPENBSD_START + 1) as usize * SECTOR as usize;
        assert_eq!(&disk[label + 64..label + 72], &DUID);
        assert_eq!(disk[(ALIGN * SECTOR) as usize], 0x5a);
    }

    #[test]
    fn the_boot_line_is_boot_with_boot8s_own_flags() {
        assert_eq!(boot_line(None).unwrap(), "boot\n");
        assert_eq!(boot_line(Some("-s")).unwrap(), "boot -s\n");
        assert_eq!(boot_line(Some("-ds")).unwrap(), "boot -ds\n");
        assert_eq!(boot_line(Some("-c")).unwrap(), "boot -c\n");
        for bad in ["s", "-", "-x", "-s selftest=fb", "/bsd"] {
            assert!(boot_line(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_typist_answers_each_prompt_once() {
        let mut t = BootTypist::new("boot -s\n");
        assert_eq!(t.line(), "boot -s\n");
        let mut serial: Vec<u8> =
            b">> EmiBSD/arm64 BOOTAA64 1.26\nefiboot: boot.conf read\nbo".to_vec();
        assert!(!t.poll(&serial));
        // The prompt completes in the next bytes.
        serial.extend_from_slice(b"ot> ");
        assert!(t.poll(&serial));
        assert!(!t.poll(&serial));
        serial.extend_from_slice(b"boot -s\nbooting sd0a:/bsd\n");
        assert!(!t.poll(&serial));
        // A `--reboot`: the firmware, efiboot and its prompt again; and two prompts at once
        // are answered one per poll.
        serial.extend_from_slice(b">> EmiBSD/arm64 BOOTAA64 1.26\nboot> boot\nboot> ");
        assert!(t.poll(&serial));
        assert!(t.poll(&serial));
        assert!(!t.poll(&serial));
    }

    #[test]
    fn boot_args_keep_efiboot_and_limine_options_apart() {
        let r = Path::new("/nonexistent-root");
        let a = Arch::Arm64;
        let err = |args: &[&str]| boot_args(r, a, args).unwrap_err().to_string();
        assert_eq!(boot_args(r, a, &["--arch", "arm64"]).unwrap(), None);
        assert_eq!(
            boot_args(r, a, &["--kernel", "k", "--cmdline", "-s"]).unwrap(),
            None
        );
        assert!(err(&["--boot-flags", "-s"]).contains("--boot-flags"));
        assert!(err(&["--expect-root"]).contains("--expect-root"));
        assert!(err(&["--image", "tree", "--cmdline", "-s"]).contains("--cmdline"));
        assert!(err(&["--image", "tree", "--expect-ramdisk"]).contains("--expect-ramdisk"));
        assert!(err(&["--root", "tree", "--init", "none"]).contains("--init"));
        assert!(err(&["--image", "tree", "--root", "tree"]).contains("one or the other"));
        assert!(err(&["--image", "tree", "--kernel", "k"]).contains("--kernel"));
        assert!(err(&["--image", "bare"]).contains("shared images"));
        assert!(err(&["--image", "tree"]).contains("efiboot-images-arm64"));
        // Checked before a per-run image is made (no --kernel needed to see them).
        assert!(err(&["--root", "tree", "--boot-flags", "s"]).contains("--boot-flags"));
        assert!(err(&["--root", "tree", "--boot-on", "ahci"]).contains("expected nvme"));
        assert!(err(&["--root", "tree", "--boot-on", "nvme", "--nvme", "x.img"]).contains("NVMe"));
        assert!(
            err(&["--root", "tree", "--boot-manual", "--boot-flags", "-s"])
                .contains("--boot-manual")
        );
        assert!(err(&["--root", "tree"]).contains("--kernel"));
        assert!(boot_args(r, Arch::Amd64, &["--image", "tree"]).is_err());
    }

    #[test]
    fn the_preset_is_written_once_into_an_empty_slot() {
        let mut k = vec![0u8; 256];
        k[100..116].copy_from_slice(&PRESET_MAGIC);
        write_preset(&mut k, "mpstress").unwrap();
        assert_eq!(&k[116..133], b"selftest=mpstress");
        assert!(k[133..164].iter().all(|&b| b == 0));
        // Its slot is no longer empty.
        assert!(
            write_preset(&mut k, "trap")
                .unwrap_err()
                .contains("not empty")
        );
        // No magic: a kernel without feature qemu.
        assert!(
            write_preset(&mut [0u8; 256], "trap")
                .unwrap_err()
                .contains("0 copies")
        );
        // Two magics.
        let mut two = vec![0u8; 256];
        two[0..16].copy_from_slice(&PRESET_MAGIC);
        two[128..144].copy_from_slice(&PRESET_MAGIC);
        assert!(
            write_preset(&mut two, "trap")
                .unwrap_err()
                .contains("2 copies")
        );
        // A slot cut by the file's end.
        let mut short = vec![0u8; 40];
        short[8..24].copy_from_slice(&PRESET_MAGIC);
        assert!(write_preset(&mut short, "trap").is_err());
        // Words.
        let mut k = vec![0u8; 256];
        k[0..16].copy_from_slice(&PRESET_MAGIC);
        for bad in ["", "Trap", "trap fb", "-s", &"x".repeat(40)] {
            assert!(write_preset(&mut k, bad).is_err(), "{bad}");
        }
        write_preset(&mut k, &"x".repeat(39)).unwrap();
    }

    #[test]
    fn the_preset_magic_is_the_kernels() {
        let selftest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sys/bsd/kern/selftest.rs");
        let text = fs::read_to_string(&selftest).unwrap();
        assert!(text.contains("const PRESET_MAGIC: [u8; 16] = *b\"EMIBSD-SELFTEST\\0\";"));
        assert!(text.contains(&format!("const PRESET_SLOT: usize = {PRESET_SLOT};")));
        assert_eq!(PRESET_MAGIC, *b"EMIBSD-SELFTEST\0");
    }

    #[test]
    fn pe_check_wants_signatures_and_a_whole_image() {
        let mut img = vec![0u8; 0x200];
        img[..2].copy_from_slice(b"MZ");
        img[0x3c] = 0x40;
        img[0x40..0x44].copy_from_slice(b"PE\0\0");
        img[0x58..0x5a].copy_from_slice(&[0x0b, 0x02]);
        img[0x58 + 56..0x58 + 60].copy_from_slice(&0x200u32.to_le_bytes());
        assert_eq!(check_pe(&img), Ok(()));
        img[0x58 + 56..0x58 + 60].copy_from_slice(&0x100u32.to_le_bytes());
        assert!(check_pe(&img).is_err());
        img[0] = b'X';
        assert!(check_pe(&img).is_err());
    }
}
/* </TESTS> */
