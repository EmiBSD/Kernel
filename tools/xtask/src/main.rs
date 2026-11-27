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
//! `xtask`: host-side developer tooling for EmiBSD.
//!
//! Invoked through the cargo alias in `.cargo/config.toml`:
//!
//! ```text
//! cargo xtask lz check                     validate lineage.toml against the tree and the LZ pin
//! cargo xtask lz status [--write]          inherited/redesigned modules per subsystem; --write
//!                                          regenerates the tables of README.md and docs/STATUS.md
//! cargo xtask lz drift [--fetch] [--security] [--functions] [--strict]
//!                                          LZ commits after the pin without an lz-sync.toml record
//! cargo xtask lz trace <lz path>:<item> | --rust <path>:<item> | --c <c path>:<item>
//!                                          where an item's code is now, or came from (lz.rs)
//! cargo xtask lz patch <commit>            an LZ commit's patch with its paths in this tree's
//!                                          layout (sys/bsd/, sys/boot/), for `git am -3`
//! cargo xtask qemu --arch A (--image tree|init | --kernel K --root ...) [--disk-fresh] [--disks N]
//!                                          boot an efiboot image (below), serial and
//!                                          monitor on stdio;
//!                                          (also smoke and smoke2) the persistent disk
//!                                          target/disk-A[-a|-b].img, 64 MiB, is kept
//!                                          across boots unless --disk-fresh recreates it;
//!                                          --disks N (1..=4, default 1) attaches N such
//!                                          disks, sd0 the file above and sd1..sd3
//!                                          target/disk-A[-a|-b]-sdK.img (each VM of smoke2
//!                                          gets N); a run with fewer disks than the last
//!                                          keeps the extra files, unattached;
//!                                          --disk-set NAME (qemu, smoke) uses the set
//!                                          target/disk-A-NAME[-sdK].img instead;
//!                                          --smp N (1..=8; qemu, smoke, smoke2) gives
//!                                          every VM N processors;
//!                                          --memory SIZE (256M..16G; qemu, smoke) the guest's
//!                                          memory instead of boot::VM_MEMORY (512M);
//!                                          --usb, --audio hda|ac97|usb and --expect-tone (qemu,
//!                                          smoke) add M12's devices: a qemu-xhci with a
//!                                          USB stick and keyboard, Intel HDA, AC97 or a
//!                                          usb-audio (M16b) into a WAV file that must hold
//!                                          a tone (devices.rs); --usb-hc ehci|uhci|ohci (M16b) puts
//!                                          the stick on a usb-ehci (alone) or a
//!                                          piix3-usb-uhci instead
//! cargo xtask smoke --arch A (--image tree|init | --kernel K --root ...) [--status N]
//!                   [--send-after L --send T]... [--until-seen] [--timeout SECS] --expect L...
//!                                          boot an efiboot image headless; pass if every L
//!                                          appears and QEMU exits with status N (default:
//!                                          the kernel's success status)
//!                   [--https-server DIR:PORT:trusted|untrusted|echo]...
//!                                          (smoke and smoke2) TLS test servers on this
//!                                          machine during the run, the guest's
//!                                          emibsd-host:PORT (https.rs)
//! cargo xtask smoke2 --arch A (--image tree|init | --kernel K --root ...) [--timeout SECS] [--show-transcripts]
//!                   [--both-|--a-|--b-send-after L --send T]... [--both-|--a-|--b-expect L]...
//!                                          boot TWO VMs of A at once, each with vio1 on a
//!                                          private link, each running its own script; pass
//!                                          when both saw all they expect (twovm.rs)
//! cargo xtask smoke|smoke2|qemu ... --image tree|init
//! cargo xtask smoke|smoke2|qemu ... --kernel K --root bare|init[=FILE]|tree[=DIR]
//!                   [--owners FILE] [--duid HEX16] [--selftest WORD]
//!                   [--boot-flags -FLAGS | --boot-manual] [--expect-root] [--boot-on nvme]
//!                                          boot through efiboot (efiboot.rs): a shared
//!                                          image of `just efiboot-images-A`, or a per-run
//!                                          one made first (efiboot-image's options), both
//!                                          attached with snapshot=on; smoke and smoke2
//!                                          type `boot [-FLAGS]` at every `boot> ` unless
//!                                          --boot-manual (the recipe types); --expect-root
//!                                          expects `(<duid>.a) swap on`; --boot-on puts the
//!                                          image on an NVMe namespace (EDK2 AArch64 boots
//!                                          no AHCI disk); every per-run image's /bsd is
//!                                          stripped of its debug sections; arm64 only;
//!                                          image, --cmdline, --init, --ramdisk and
//!                                          --expect-ramdisk went with Limine's image
//!                                          builder (2026-11-26) and are refused, naming
//!                                          N10's first step, which converts amd64's recipes
//! cargo xtask smoke-all [-j N] [--just PATH] RECIPE...
//!                                          run the justfile's smoke RECIPEs with
//!                                          `just --no-deps`, N (default 4) at a time, each
//!                                          in target/smoke/RECIPE (its images, disks and
//!                                          log); a line per recipe, the failed ones' logs
//!                                          at the end (smokeall.rs)
//! cargo xtask diff-openbsd [--arch A]... [--smp N] [--kernel-dir D] [fetch|install|run]
//!                                          the same scenarios on EmiBSD and on a real OpenBSD
//!                                          VM (the snapshot of openbsd-snapshot.toml,
//!                                          installed under target/openbsd), compared step by
//!                                          step (diffopenbsd.rs)
//! cargo xtask diff-openbsd --arch A [--ipmi] [--nic MODEL] [--usb] [--usb-hc H] [--ukc CMD]... [--sh CMD] probe
//!                                          that OpenBSD alone with the smokes' device
//!                                          options, booted with `-c` and the UKC commands
//!                                          given: its dmesg and CMD's output
//! cargo xtask bench [--arch A]... [--smp N] [--reps N] [--no-tcpbench] [--profile debug|release]
//!                                          N3: EmiBSD against that OpenBSD on the same
//!                                          emulated machine, one after the other, N runs
//!                                          (10): boot time, tools/bench's ns per op, tcpbench
//!                                          between two VMs; the worst sample per side left
//!                                          out, then median, min, spread and the
//!                                          ratio EmiBSD / OpenBSD, refused over a 15 %
//!                                          spread; <run dir>/A/report.toml (bench.rs);
//!                                          `--profile release` boots the release kernel
//!                                          (decision 42, a diagnostic: init stays
//!                                          debug)
//! cargo xtask bench compare BEFORE AFTER   the `Bench:` lines of what moved between two
//!                                          reports, beyond their spreads
//! cargo xtask unsafe-report [--write|--check] [--shapes]
//!                                          `unsafe` blocks, fns, impls and traits per kernel
//!                                          subsystem, test code apart, the shapes to replace,
//!                                          and the core, forbid and legacy groups of
//!                                          unsafe-core.toml; --write puts the totals on
//!                                          docs/STATUS.md's `Unsafe` line, lowers
//!                                          unsafe-budget.toml and moves its [ratchet] the
//!                                          good way only (its baseline never); --check fails
//!                                          when a subsystem exceeds its budget, the forbid
//!                                          modules fall below the floor or legacy rises
//!                                          above the ceiling; --shapes lists the shapes per
//!                                          module; a `crates` block (kernel, abi, hal,
//!                                          bsd with the part that moves into hal, boot: files and `unsafe`) and
//!                                          --check's rules for it (kernel and abi 0, hal
//!                                          under hal_ceiling, bsd outside the core under
//!                                          legacy_ceiling: the hardware is in unsafe-core.toml,
//!                                          so it is already outside) (unsafereport.rs)
//! cargo xtask unsafe-report --json         the same measure as one JSON document: every
//!                                          kernel file with its lineage status, group,
//!                                          `unsafe` and shapes, the totals, the ratchet and
//!                                          the pace since its baseline and, additively, a
//!                                          `crates` object (what /progress reads)
//! cargo xtask hal-check                    the rules of the hal crate (sys/hal/): no `pub unsafe
//!                                          fn`, a `// SAFETY:` comment above every `unsafe`
//!                                          block, no `static mut`, the crate's `unsafe` at or
//!                                          under hal_ceiling of unsafe-budget.toml
//!                                          (halcheck.rs)
//! cargo xtask abi-check                    the rules of the abi crate (sys/abi/), decision 25:
//!                                          no loop/while/for, if/match only on nr or on
//!                                          Ok/Err/Some/None, at most one kernel call and 40
//!                                          lines per fn, no static with interior
//!                                          mutability; R6 (decision 52): kernel only
//!                                          through kernel::api, hal only through
//!                                          hal::syscall's types and set_handler
//!                                          (sysdefs freely)
//!                                          (abicheck.rs)
//! cargo xtask symbolize --arch A [--kernel K]
//!                                          annotate the addresses of a stack trace on stdin
//!                                          with K's symbols (default: the debug kernel)
//! cargo xtask userland --arch A            cross-compile OpenBSD's libc, init, ksh, echo and
//!                                          ls from the reference sources (target/userland/A)
//! cargo xtask devtools-hello --arch A --llvm DIR --rust DIR
//!                                          the C, C++ and Rust hellos of smoke-devtools,
//!                                          built with DevTools' toolchains in DIR against
//!                                          target/userland/A/sysroot, and the ramdisk
//!                                          that carries them (devtools.rs)
//! cargo xtask devtools-native --arch A --llvm TAR --llvm-sha256 HEX --rust TAR --rust-sha256 HEX
//!                                          smoke-devtools-native's disk: comp's root with
//!                                          DevTools' native trees (T3b) over it, each
//!                                          tarball checked against its pin (userland/comp.rs)
//! cargo xtask comp --arch A [--jobs N]     M14: OpenBSD's compiler (clang, lld, libc++) from
//!                                          gnu/llvm by its build glue, into target/comp/A
//!                                          (userland/comp.rs); N jobs, default half the CPUs
//! cargo xtask efiboot-image --arch A --kernel K --root bare|init[=FILE]|tree[=DIR]
//!                     [--owners FILE] [--duid HEX16] [--efi FILE] [--selftest WORD]
//!                     (--shared | --out FILE)
//!                                          an efiboot disk (MBR, label, FFS a with /bsd,
//!                                          ESP): --shared writes target/efiboot/A/
//!                                          emibsd-{tree,init}.img, --out a per-run image
//!                                          in the run directory (efiboot.rs)
//! cargo xtask install-img --arch A         M16g: install80.img, the install media with the
//!                                          sets on one disk image (distrib.rs)
//! cargo xtask cd-iso --arch amd64          M16g: cd80.iso, bsd.rd on an El Torito CD
//!                                          (distrib.rs, iso9660.rs)
//! cargo xtask install80|cd80 --arch A      M16g: install from install80.img as a USB stick
//!                                          and boot the result; boot cd80.iso to the
//!                                          installer (distrib.rs)
//! cargo xtask ntfs-image OUT [--check]     write M10d's NTFS test volume to OUT (ntfsgen.rs);
//!                                          --check mounts it with macOS's NTFS driver
//! cargo xtask e2fsck --arch A [--disk-set NAME] [--cat PATH=TEXT]...
//!                                          check the ext2 file system on partition a of
//!                                          the persistent disk with e2fsprogs' e2fsck -fn;
//!                                          debugfs must cat PATH and print TEXT (e2fs.rs)
//! cargo xtask manual --arch arm64 --boot-log LOG --out FILE
//!                                          MANUAL.toml: the kernel's drivers (configured,
//!                                          attached in LOG), syscalls (native, legacy,
//!                                          absent) and the manual pages that are not true
//!                                          here, generated from the tree (manual.rs)
//! cargo xtask include --arch arm64 --out DIR
//!                                          the C headers of the ABI as OpenBSD's
//!                                          include/Makefile installs them from sys/ (the
//!                                          pin's text), for the release tarball (manual.rs)
//! cargo xtask fsck-msdos --arch A          check the USB smokes' stick's FAT32 file system
//!                                          with macOS's fsck_msdos -n (devices.rs)
//! ```
//!
//! Paths are resolved from the workspace root (derived from `CARGO_MANIFEST_DIR`), never from the
//! current directory. The files a boot writes (the image, the EDK2 variable store, the
//! persistent disks; `target/...` above) go to `$EMIBSD_RUN_DIR` instead of `target/` when it
//! is set (`smoke-all` does, per recipe); `$EMIBSD_TIMEOUT_SCALE` (1 to 10) multiplies the
//! time limits of `smoke` and `smoke2`. While a boot waits it prints a line a minute
//! (`boot::Heartbeat`), which `smoke-all`'s watchdog relies on (`smokeall.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod abicheck;
mod bench;
mod boot;
mod bridges;
mod bsdmake;
mod devices;
mod devtools;
mod diffopenbsd;
mod distrib;
mod e2fs;
mod efiboot;
mod halcheck;
mod https;
mod hwopts;
mod install;
mod iso9660;
mod layout;
mod lz;
mod manual;
mod ntfsgen;
mod rdsetroot;
mod smokeall;
mod storage;
mod symbolize;
mod syscalls;
mod twovm;
mod unsafereport;
mod userland;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Placeholder hash used before the reference tree has been cloned and pinned.
const REFERENCE_DIR: &str = "reference/openbsd-src";

const USAGE: &str = "usage: cargo xtask <lz check | lz status [--write] | lz drift [--fetch] [--security] [--functions] [--strict] | \
                     lz trace <lz path>:<item> | lz trace --rust <path>:<item> | lz trace --c <c path>:<item> | lz patch <commit> | \
                     qemu --arch A (--image tree|init | --kernel K --root bare|init|tree) [--disk-fresh] [--disks N] [--disk-set NAME] [--nvme FILE] [--ahci FILE] [--scsi-cd ISO] [--lsi FILE [--lsi-cd ISO]] [--pci-serial FILE] [--pci-bridges] [--machine pc] [--ide|--megasas|--megasas-gen2|--mptsas|--pvscsi|--am53c974|--dc390|--ufs|--sdhci|--floppy FILE]... [--ipmi] [--reboot] [--vio-mq] [--fb] | gen-syscalls [--check] | \
                     smoke --arch A (--image tree|init | --kernel K --root bare|init|tree) [--disk-fresh] [--disks N] [--disk-set NAME] [--nvme FILE] [--ahci FILE] [--scsi-cd ISO] [--lsi FILE [--lsi-cd ISO]] [--pci-serial FILE] [--pci-bridges] [--machine pc] [--ide|--megasas|--megasas-gen2|--mptsas|--pvscsi|--am53c974|--dc390|--ufs|--sdhci|--floppy FILE]... [--ipmi] [--reboot] [--vio-mq] [--expect-pci-serial T]... [--fb] [--screenshot-after L [--screen-text ROW:COL:TEXT]] [--sendkey-after L --sendkeys K] [--usb] [--usb-hc xhci|ehci|uhci|ohci] [--usb-mouse] [--usb-tablet] [--usb-wacom-tablet] [--usb-ccid] [--usb-net] [--usb-serial FILE [--usb-serial-send-after L --usb-serial-send T]... [--expect-usb-serial T]...] [--audio hda|ac97|usb] [--expect-tone] [--status N] [--send-after L --send T]... [--until-seen] [--timeout S] [--https-server DIR:PORT:MODE]... [--reject L]... --expect L... | \
                     smoke2 --arch A (--image tree|init | --kernel K --root bare|init|tree) [--timeout S] [--show-transcripts] [--disk-fresh] [--disks N] [--both-|--a-|--b-send-after L --send T]... [--both-|--a-|--b-expect L]... [--reject L]... [--https-server DIR:PORT:MODE]... | \
                     smoke-all [-j N] [--just PATH] RECIPE... | \
                     unsafe-report [--write] [--check] [--shapes] | unsafe-report --json | \
                     diff-openbsd [--arch A]... [--smp N] [--kernel-dir D] [fetch | install | run | powerbtn] | \
                     diff-openbsd --arch A [--ipmi] [--nic MODEL] [--usb] [--usb-hc xhci|ehci|uhci|ohci] [--ukc CMD]... [--sh CMD] probe | \
                     bench [--arch A]... [--smp N] [--reps N] [--no-tcpbench] [--profile debug|release] | bench compare BEFORE AFTER | \
                     symbolize --arch A [--kernel K] | userland --arch A | devtools-hello --arch A --llvm DIR --rust DIR | devtools-native --arch A --llvm TAR --llvm-sha256 HEX --rust TAR --rust-sha256 HEX | comp --arch A [--jobs N] | ntfs-image OUT [--check] | \
                     e2fsck --arch A [--disk-set NAME] [--cat PATH=TEXT]... | fsck-msdos --arch A | \
                     manual --arch arm64 --boot-log LOG --out FILE | include --arch arm64 --out DIR | \
                     nvme-root --arch A [--duid HEX] [--out FILE] [--root-dev DEV] | \
                     efiboot-image --arch A --kernel K --root bare|init[=FILE]|tree[=DIR] [--owners FILE] [--duid HEX16] [--efi FILE] [--selftest WORD] (--shared | --out FILE)>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    let root = workspace_root()?;
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();
    boot::set_smp(smp_flag(&argv)?);
    boot::set_memory(memory_flag(&argv)?);
    boot::set_accel_hvf(accel_flag(&argv)?);
    hwopts::set(&root, &argv)?;
    storage::set(&root, &argv)?;
    devices::set_from_args(&argv)?;
    match argv.as_slice() {
        ["lz", "check"] => lz::check(&root),
        ["lz", "status"] => lz::status(&root, false),
        ["lz", "status", "--write"] => lz::status(&root, true),
        ["lz", "drift", flags @ ..] => lz::drift(
            &root,
            flags.contains(&"--fetch"),
            flags.contains(&"--security"),
            flags.contains(&"--functions"),
            flags.contains(&"--strict"),
        ),
        ["lz", "trace", rest @ ..] => lz::trace(&root, rest),
        ["lz", "patch", commit] => lz::patch(&root, commit),
        // Limine's image builder, retired with the removal of Limine (2026-11-26).
        ["image", ..] => Err(
            "image: retired with Limine (2026-11-26); `efiboot-image --root \
             bare|init|tree (--shared | --out FILE)` makes the boot images, and N10's first \
             step converts amd64's recipes to them"
                .into(),
        ),
        ["qemu", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            let efiboot = efiboot::boot_args(&root, arch, rest)?;
            boot::qemu(
                &root,
                arch,
                &boot::Disks {
                    fresh: rest.contains(&"--disk-fresh"),
                    count: disks_flag(rest)?,
                    set: optional_flag(rest, "--disk-set"),
                },
                &efiboot,
            )
        }
        ["smoke", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            if boot::skip_disabled("smoke", arch)? {
                return Ok(());
            }
            let expects = flags(rest, "--expect");
            if expects.is_empty() {
                return Err(format!("missing `--expect <line>`\n{USAGE}").into());
            }
            let status = match optional_flag(rest, "--status") {
                Some(s) => s.parse::<i32>().map_err(|e| format!("--status {s}: {e}"))?,
                None => boot::QEMU_SUCCESS_STATUS,
            };
            // `--send-after A --send T`, repeatable: each text is sent once its trigger line
            // has been seen, in order.
            let afters = flags(rest, "--send-after");
            let texts = flags(rest, "--send");
            if afters.len() != texts.len() {
                return Err(format!("--send-after and --send go together\n{USAGE}").into());
            }
            let sends: Vec<(&str, String)> = afters
                .iter()
                .zip(&texts)
                .map(|(a, t)| (*a, t.replace("\\n", "\n")))
                .collect();
            // `--image` or `--kernel K --root ...`: an efiboot boot (efiboot.rs), whose
            // per-run image is made here, once, before any boot.
            let efiboot = efiboot::boot_args(&root, arch, rest)?;
            // Killed when dropped, after the run.
            let _servers = https::start(&root, &flags(rest, "--https-server"))?;
            boot::smoke(
                &root,
                arch,
                &boot::SmokeOptions {
                    expects: &expects,
                    rejects: &flags(rest, "--reject"),
                    status,
                    sends: &sends
                        .iter()
                        .map(|(a, t)| (*a, t.as_str()))
                        .collect::<Vec<_>>(),
                    until_seen: rest.contains(&"--until-seen"),
                    disks: boot::Disks {
                        fresh: rest.contains(&"--disk-fresh"),
                        count: disks_flag(rest)?,
                        set: optional_flag(rest, "--disk-set"),
                    },
                    timeout: match optional_flag(rest, "--timeout") {
                        Some(s) => Some(std::time::Duration::from_secs(
                            s.parse::<u64>()
                                .map_err(|e| format!("--timeout {s}: {e}"))?,
                        )),
                        None => None,
                    },
                    efiboot: &efiboot,
                },
            )
        }
        ["smoke2", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            if boot::skip_disabled("smoke2", arch)? {
                return Ok(());
            }
            let plan = twovm::parse_plan(rest)?;
            let efiboot = efiboot::boot_args(&root, arch, rest)?;
            let _servers = https::start(&root, &flags(rest, "--https-server"))?;
            twovm::smoke2(
                &twovm::Boot {
                    root: &root,
                    arch,
                    efiboot: &efiboot,
                },
                plan,
            )
        }
        ["smoke-all", rest @ ..] => {
            let a = smokeall::parse_args(rest)?;
            smokeall::smoke_all(&root, a.jobs, a.just, &a.recipes)
        }
        ["diff-openbsd", rest @ ..] => diffopenbsd::diff_openbsd(&root, rest),
        ["bench", rest @ ..] => bench::bench(&root, rest),
        ["manual", rest @ ..] => manual::manual(&root, rest),
        ["include", rest @ ..] => manual::include(&root, rest),
        ["hal-check"] => halcheck::hal_check(&root),
        ["abi-check"] => abicheck::abi_check(&root),
        ["unsafe-report", flags @ ..] => unsafereport::unsafe_report(
            &root,
            flags.contains(&"--write"),
            flags.contains(&"--check"),
            flags.contains(&"--shapes"),
            flags.contains(&"--json"),
        ),
        ["gen-syscalls"] => syscalls::gen_syscalls(&root, false),
        ["gen-syscalls", "--check"] => syscalls::gen_syscalls(&root, true),
        ["symbolize", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            let kernel = match optional_flag(rest, "--kernel") {
                Some(k) => PathBuf::from(k),
                None => root
                    .join("target")
                    .join(arch.target())
                    .join("debug")
                    .join("bsd"),
            };
            symbolize::symbolize(&kernel)
        }
        ["userland", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            userland::userland(&root, arch)
        }
        ["devtools-hello", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            devtools::hello(
                &root,
                arch,
                Path::new(flag(rest, "--llvm")?),
                Path::new(flag(rest, "--rust")?),
            )
        }
        ["devtools-native", rest @ ..] => userland::comp::devtools_native(
            &root,
            boot::Arch::parse(flag(rest, "--arch")?)?,
            userland::comp::Pinned {
                tarball: Path::new(flag(rest, "--llvm")?),
                sha256: flag(rest, "--llvm-sha256")?,
            },
            userland::comp::Pinned {
                tarball: Path::new(flag(rest, "--rust")?),
                sha256: flag(rest, "--rust-sha256")?,
            },
        ),
        ["comp", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            // Half the CPUs by default: other builds may share the machine.
            let jobs = match optional_flag(rest, "--jobs") {
                Some(n) => n
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| format!("--jobs {n}: not a positive number"))?,
                None => std::thread::available_parallelism()
                    .map_or(4, |n| n.get())
                    .div_ceil(2),
            };
            userland::comp::comp(&root, arch, jobs)
        }
        // M14c: OpenBSD's rdsetroot(8) for our kernel ELF (rdsetroot.rs).
        ["rdsetroot", rest @ ..] => rdsetroot::rdsetroot(rest),
        ["miniroot", rest @ ..] => install::miniroot(&root, rest),
        ["sets", rest @ ..] => install::sets(&root, rest),
        ["install-media", rest @ ..] => install::install_media(&root, rest),
        ["install", rest @ ..] => install::install(&root, rest),
        ["install-boot", rest @ ..] => install::install_boot(&root, rest),
        // M16g: the release images and their smokes (distrib.rs, iso9660.rs).
        ["install-img", rest @ ..] => distrib::install_img(&root, rest),
        ["cd-iso", rest @ ..] => distrib::cd_iso(&root, rest),
        ["install80", rest @ ..] => distrib::install80(&root, rest),
        ["cd80", rest @ ..] => distrib::cd80(&root, rest),
        ["ntfs-image", out] => ntfsgen::ntfs_image(&root.join(out), false),
        ["ntfs-image", out, "--check"] => ntfsgen::ntfs_image(&root.join(out), true),
        // M14: efiboot's PE image (efiboot.rs).
        ["efiboot", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            efiboot::efiboot(
                &root,
                arch,
                Path::new(flag(rest, "--elf")?),
                optional_flag(rest, "--out"),
            )
        }
        // Retired with the removal of Limine's S2 (2026-11-26); amd64's smoke-efiboot still
        // names it (decision 44: its text stays until N10's first step converts it).
        ["efiboot-disk", ..] => Err("efiboot-disk: retired; `efiboot-image --root bare|tree \
             (--shared | --out FILE)` makes the efiboot images, and N10's first step converts \
             amd64's smoke-efiboot to them"
            .into()),
        // The removal of Limine (2026-11-26): the one efiboot image builder (efiboot.rs).
        ["efiboot-image", rest @ ..] => efiboot::image_command(&root, rest),
        ["nvme-root", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            hwopts::nvme_root(
                &root,
                arch,
                optional_flag(rest, "--duid"),
                optional_flag(rest, "--out"),
                optional_flag(rest, "--root-dev"),
            )
        }
        ["e2fsck", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            e2fs::e2fsck(
                &root,
                arch,
                optional_flag(rest, "--disk-set"),
                &flags(rest, "--cat"),
            )
        }
        ["fsck-msdos", rest @ ..] => {
            let arch = boot::Arch::parse(flag(rest, "--arch")?)?;
            devices::fsck_msdos(&root, arch)
        }
        _ => Err(USAGE.into()),
    }
}

/// The value following `name` in `args`, if present.
fn optional_flag<'a>(args: &[&'a str], name: &str) -> Option<&'a str> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1])
}

/// `--disks N`: how many persistent disks a VM gets, 1 (the default) to `boot::MAX_DISKS`.
fn disks_flag(args: &[&str]) -> Result<usize> {
    let Some(s) = optional_flag(args, "--disks") else {
        return Ok(1);
    };
    match s.parse::<usize>() {
        Ok(n) if (1..=boot::MAX_DISKS).contains(&n) => Ok(n),
        _ => Err(format!(
            "--disks {s}: expected a number from 1 to {}",
            boot::MAX_DISKS
        )
        .into()),
    }
}

/// `--smp N` (qemu, smoke, smoke2): QEMU's `-smp N`, 1 to `boot::MAX_SMP`; absent, QEMU's
/// default of one processor.
fn smp_flag(args: &[&str]) -> Result<Option<u32>> {
    let Some(s) = optional_flag(args, "--smp") else {
        return Ok(None);
    };
    match s.parse::<u32>() {
        Ok(n) if (1..=boot::MAX_SMP).contains(&n) => Ok(Some(n)),
        _ => Err(format!("--smp {s}: expected a number from 1 to {}", boot::MAX_SMP).into()),
    }
}

/// `--memory SIZE` (qemu, smoke): QEMU's `-m`, a number of mebibytes (`768M`) or gibibytes
/// (`2G`), from 256M to 16G; absent, `boot::VM_MEMORY`. For the boots that need more than the
/// default, as `smoke-devtools-native`'s rustc.
fn memory_flag(args: &[&str]) -> Result<Option<String>> {
    let Some(s) = optional_flag(args, "--memory") else {
        return Ok(None);
    };
    let mib = match s.split_at(s.len().saturating_sub(1)) {
        (n, "M") => n.parse::<u32>().ok(),
        (n, "G") => n.parse::<u32>().ok().and_then(|g| g.checked_mul(1024)),
        _ => None,
    };
    match mib {
        Some(m) if (256..=16384).contains(&m) => Ok(Some(s.to_string())),
        _ => Err(format!("--memory {s}: expected 256M to 16G (a number then M or G)").into()),
    }
}

/// `--accel hvf` (qemu, smoke): arm64 guests under Hypervisor.framework; the only accelerator
/// accepted, and an amd64 guest cannot use it on an arm64 Mac.
fn accel_flag(args: &[&str]) -> Result<bool> {
    match optional_flag(args, "--accel") {
        None => Ok(false),
        Some("hvf") if optional_flag(args, "--arch") == Some("amd64") => {
            Err("--accel hvf: arm64 guests only (the Mac is arm64)".into())
        }
        Some("hvf") => Ok(true),
        Some(other) => Err(format!("--accel {other}: only `hvf` is known").into()),
    }
}

/// Every value following an occurrence of `name` in `args`.
fn flags<'a>(args: &[&'a str], name: &str) -> Vec<&'a str> {
    args.windows(2)
        .filter(|w| w[0] == name)
        .map(|w| w[1])
        .collect()
}

/// The value following `name` in `args`; an error naming the flag otherwise.
fn flag<'a>(args: &[&'a str], name: &str) -> Result<&'a str> {
    optional_flag(args, name).ok_or_else(|| format!("missing `{name} <value>`\n{USAGE}").into())
}

/// `tools/xtask` → workspace root. Compile-time manifest dir, so the cwd never matters.
fn workspace_root() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .ok_or("cannot locate the workspace root from CARGO_MANIFEST_DIR")?;
    Ok(root.to_path_buf())
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// The boot loaders' directories of `sys/boot/` and the LZ path of each (the user's layout of
/// 2026-11-26); the per-arch ones (`sys/boot/<arch>/`, LZ's `sys/arch/<arch>/stand/efiboot/`, and
/// amd64's three libsa files, LZ's `sys/arch/amd64/stand/libsa/`) follow the rules in
/// [`lz_flat`].
const BOOT_HOMES: [(&str, &str); 3] = [
    ("sys/boot/libsa/", "sys/lib/libsa/"),
    ("sys/boot/efi/", "sys/stand/efi/"),
    ("sys/boot/loader/", "sys/stand/boot/"),
];

/// The files of `sys/boot/amd64/` that LZ keeps in `sys/arch/amd64/stand/libsa/` (the rest of
/// `sys/boot/<arch>/` is LZ's `stand/efiboot/`).
const AMD64_LIBSA: [&str; 3] = ["disk.rs", "libsa.rs", "mdrandom.rs"];

/// A native path as LZ's tree lays it out (LZ's `sys/` is one flat tree): `sys/bsd/<x>`, the
/// legacy package `bsd`, is LZ's `sys/<x>`; `sys/boot/libsa/`, `efi/` and `loader/` are LZ's
/// `sys/lib/libsa/`, `sys/stand/efi/` and `sys/stand/boot/`; `sys/boot/<arch>/<x>` is LZ's
/// `sys/arch/<arch>/stand/efiboot/<x>`, except amd64's `disk.rs`, `libsa.rs` and `mdrandom.rs`,
/// which are LZ's `sys/arch/amd64/stand/libsa/`. Every other path is returned as it is. Each rule replaces a
/// prefix and keeps the rest, so the file's path below its directory is the same in both trees.
/// The tools that name a file by LZ's layout (the lineage's homes, the unsafe report's rows and
/// crates, the structure that is not ported) read a native path through it.
pub(crate) fn lz_flat(rel: &str) -> String {
    if let Some(rest) = rel.strip_prefix("sys/bsd/") {
        return format!("sys/{rest}");
    }
    for (native, lz) in BOOT_HOMES {
        if let Some(rest) = rel.strip_prefix(native) {
            return format!("{lz}{rest}");
        }
    }
    if let Some(rest) = rel.strip_prefix("sys/boot/")
        && let Some((arch, tail)) = rest.split_once('/')
        && !tail.contains('/')
    {
        let dir = if arch == "amd64" && AMD64_LIBSA.contains(&tail) {
            "libsa"
        } else {
            "efiboot"
        };
        return format!("sys/arch/{arch}/stand/{dir}/{tail}");
    }
    rel.to_string()
}

/// The native path of an LZ path, [`lz_flat`]'s inverse: LZ's `sys/lib/libsa/`,
/// `sys/stand/efi/`, `sys/stand/boot/`, `sys/arch/<arch>/stand/efiboot/` and amd64's
/// `sys/arch/amd64/stand/libsa/` are the loaders' `sys/boot/...`; LZ's `sys/lib/libkern/` and `sys/lib/libz/` stay (the leaf crates, grouped
/// by purpose since, are found by hand as before); every other `sys/<x>` is the legacy tree's
/// `sys/bsd/<x>`. A path outside `sys/` (LZ's justfile, `tools/`) is the same in both trees.
pub(crate) fn native_of_lz(lz: &str) -> String {
    for (native, home) in BOOT_HOMES {
        if let Some(rest) = lz.strip_prefix(home) {
            return format!("{native}{rest}");
        }
    }
    let Some(rest) = lz.strip_prefix("sys/") else {
        return lz.to_string();
    };
    if let Some(after) = rest.strip_prefix("arch/")
        && let Some((arch, tail)) = after.split_once('/')
        && let Some(dir) = tail.strip_prefix("stand/")
        && let Some((sub, file)) = dir.split_once('/')
        && matches!(sub, "efiboot" | "libsa")
    {
        return format!("sys/boot/{arch}/{file}");
    }
    if rest.starts_with("lib/libkern/") || rest.starts_with("lib/libz/") {
        return lz.to_string();
    }
    format!("sys/bsd/{rest}")
}

/// Files that are structure, not ports: never need a `ports.toml` entry. `rel` is a native or
/// an LZ path: a native one is read through [`lz_flat`].
fn is_structural(rel: &str) -> bool {
    let rel = lz_flat(rel);
    let rel = rel.as_str();
    let name = rel.rsplit('/').next().unwrap_or(rel);
    matches!(name, "mod.rs" | "lib.rs" | "main.rs" | "build.rs")
        || rel.starts_with("sys/machine/")
        || rel.starts_with("sys/arch/host/")
        || rel.starts_with("sys/hal/machine/")
        || rel.starts_with("sys/hal/arch/host/")
        || rel.starts_with("sys/stand/")
}

pub(crate) fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                continue;
            }
            walk_rs(&path, out)?;
        } else if path.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        let cmd = args.join(" ");
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("git {cmd}: {stderr}").into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz_flat_reads_the_bsd_and_boot_directories_as_lz_lays_them_out() {
        for (native, lz) in [
            ("sys/bsd/kern/kern_exec.rs", "sys/kern/kern_exec.rs"),
            ("sys/bsd/lib.rs", "sys/lib.rs"),
            ("sys/bsd/stand/bootarg.rs", "sys/stand/bootarg.rs"),
            (
                "sys/bsd/arch/arm64/arm64/machdep.rs",
                "sys/arch/arm64/arm64/machdep.rs",
            ),
            ("sys/boot/libsa/open.rs", "sys/lib/libsa/open.rs"),
            ("sys/boot/efi/lib.rs", "sys/stand/efi/lib.rs"),
            ("sys/boot/loader/cmd.rs", "sys/stand/boot/cmd.rs"),
            (
                "sys/boot/arm64/efiboot.rs",
                "sys/arch/arm64/stand/efiboot/efiboot.rs",
            ),
            (
                "sys/boot/amd64/disk.rs",
                "sys/arch/amd64/stand/libsa/disk.rs",
            ),
            (
                "sys/boot/amd64/efiboot.rs",
                "sys/arch/amd64/stand/efiboot/efiboot.rs",
            ),
            // Unchanged: the new crates, LZ's own paths, init.
            ("sys/hal/arch/arm64/trap.rs", "sys/hal/arch/arm64/trap.rs"),
            ("sys/kernel/proc/fork.rs", "sys/kernel/proc/fork.rs"),
            ("sys/lib/libkern/lib.rs", "sys/lib/libkern/lib.rs"),
            ("sys/kern/kern_exec.rs", "sys/kern/kern_exec.rs"),
            ("init/src/main.rs", "init/src/main.rs"),
        ] {
            assert_eq!(lz_flat(native), lz, "{native}");
        }
    }

    #[test]
    fn native_of_lz_is_lz_flat_inverse() {
        for (lz, native) in [
            ("sys/kern/kern_exec.rs", "sys/bsd/kern/kern_exec.rs"),
            ("sys/Cargo.toml", "sys/bsd/Cargo.toml"),
            ("sys/stand/bootarg.rs", "sys/bsd/stand/bootarg.rs"),
            (
                "sys/arch/arm64/arm64/machdep.rs",
                "sys/bsd/arch/arm64/arm64/machdep.rs",
            ),
            (
                "sys/arch/arm64/conf/kernel.ld",
                "sys/bsd/arch/arm64/conf/kernel.ld",
            ),
            ("sys/lib/libsa/open.rs", "sys/boot/libsa/open.rs"),
            ("sys/stand/efi/lib.rs", "sys/boot/efi/lib.rs"),
            ("sys/stand/boot/cmd.rs", "sys/boot/loader/cmd.rs"),
            (
                "sys/arch/arm64/stand/efiboot/start.S",
                "sys/boot/arm64/start.S",
            ),
            (
                "sys/arch/amd64/stand/libsa/disk.rs",
                "sys/boot/amd64/disk.rs",
            ),
            (
                "sys/arch/amd64/stand/efiboot/main.rs",
                "sys/boot/amd64/main.rs",
            ),
            ("justfile", "justfile"),
            ("tools/xtask/src/main.rs", "tools/xtask/src/main.rs"),
        ] {
            assert_eq!(native_of_lz(lz), native, "{lz}");
            assert_eq!(lz_flat(native), lz, "{native}");
        }
        // The leaf crates stay where LZ has them (their groups are found by hand).
        assert_eq!(
            native_of_lz("sys/lib/libkern/strlcpy.rs"),
            "sys/lib/libkern/strlcpy.rs"
        );
    }

    #[test]
    fn structure_is_read_in_lz_layout() {
        for rel in [
            "sys/bsd/machine/mod.rs",
            "sys/bsd/machine/cpu.rs",
            "sys/bsd/arch/host/alloc.rs",
            "sys/bsd/stand/bootarg.rs",
            "sys/boot/loader/cmd.rs",
            "sys/boot/efi/include/efiapi.rs",
            "sys/machine/cpu.rs",
            // LZ's own Limine entry, at the pin: no native module lists it (2026-11-26).
            "sys/stand/limine.rs",
        ] {
            assert!(is_structural(rel), "{rel}");
        }
        for rel in [
            "sys/bsd/kern/kern_exec.rs",
            "sys/boot/libsa/open.rs",
            "sys/boot/arm64/efiboot.rs",
        ] {
            assert!(!is_structural(rel), "{rel}");
        }
    }

    #[test]
    fn smp_is_optional_and_bounded() {
        assert_eq!(smp_flag(&[]).unwrap(), None);
        assert_eq!(
            smp_flag(&["--arch", "arm64", "--smp", "4"]).unwrap(),
            Some(4)
        );
        for bad in ["0", "9", "x"] {
            assert!(smp_flag(&["--smp", bad]).is_err(), "{bad}");
        }
    }

    #[test]
    fn memory_flag_takes_mebibytes_or_gibibytes_in_range() {
        assert_eq!(memory_flag(&[]).unwrap(), None);
        assert_eq!(
            memory_flag(&["--memory", "2G"]).unwrap().as_deref(),
            Some("2G")
        );
        assert_eq!(
            memory_flag(&["--memory", "768M"]).unwrap().as_deref(),
            Some("768M")
        );
        for bad in ["128M", "17G", "2", "G", "2T", "-1G"] {
            assert!(memory_flag(&["--memory", bad]).is_err(), "{bad}");
        }
    }

    #[test]
    fn accel_flag_knows_hvf_for_arm64_guests_only() {
        assert!(!accel_flag(&[]).unwrap());
        assert!(accel_flag(&["--arch", "arm64", "--accel", "hvf"]).unwrap());
        assert!(accel_flag(&["--arch", "amd64", "--accel", "hvf"]).is_err());
        assert!(accel_flag(&["--accel", "kvm"]).is_err());
    }

    #[test]
    fn disks_defaults_to_one_and_stops_at_four() {
        assert_eq!(disks_flag(&[]).unwrap(), 1);
        assert_eq!(disks_flag(&["--disks", "4"]).unwrap(), 4);
        assert_eq!(disks_flag(&["--arch", "amd64", "--disks", "2"]).unwrap(), 2);
        for bad in ["0", "5", "x", "-1"] {
            assert!(disks_flag(&["--disks", bad]).is_err(), "{bad}");
        }
    }

    #[test]
    fn smoke_reads_every_reject_line() {
        let args = [
            "--arch",
            "amd64",
            "--reject",
            "uptime went backwards",
            "--expect",
            "init: uptime monotonic ok",
            "--reject",
            "panic:",
        ];
        assert_eq!(
            flags(&args, "--reject"),
            vec!["uptime went backwards", "panic:"]
        );
        assert_eq!(flags(&args, "--expect"), vec!["init: uptime monotonic ok"]);
        assert!(flags(&["--arch", "amd64"], "--reject").is_empty());
    }
}
/* </TESTS> */
