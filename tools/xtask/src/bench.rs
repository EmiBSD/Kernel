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
//! `cargo xtask bench` (N3): EmiBSD's performance against OpenBSD 8.0 on the same emulated
//! machine, as a ratio per measurement.
//!
//! ```text
//! cargo xtask bench [--arch A]... [--smp N] [--reps N] [--no-tcpbench] [--profile debug|release]
//! cargo xtask bench compare BEFORE.toml AFTER.toml
//! ```
//!
//! Per arch, both systems boot one after the other, never side by side, on the machine
//! `diff-openbsd` gives them (`--smp 4` by default, the same CPU model, virtio disks and NICs)
//! and with the same memory (`boot::VM_MEMORY`; `diff-openbsd` itself gives OpenBSD 1 GiB):
//! EmiBSD is the smokes' MP kernel with the ramdisk, OpenBSD the installed snapshot of
//! `diff-openbsd` (`target/openbsd/<arch>/disk.img` under `-snapshot`; never downloaded or
//! installed here: a missing `installed` marker stops the run). Each of `--reps` runs (10)
//! boots EmiBSD, then OpenBSD (the order alternates from run to run), and measures:
//!
//! - `boot`: seconds from QEMU's start (the firmware's) to the first `login:` on the serial
//!   console, firmware and boot loader included (OpenBSD's efiboot waits at its `boot>`
//!   prompt; EmiBSD's Limine does not);
//! - `tools/bench` (ISC, ours, `bench.c`), the same static binary on both: ns per op of
//!   `pagefault` (first touch of a page of a fresh anonymous mapping), `forkexit`, `forkexec`
//!   (fork and exec of the program itself as a trivial `true`), `pipe` (a one-byte round trip
//!   between two processes) and `fileread` (64 KiB read(2)s of a cold 256 MiB file on the
//!   scratch disk, a fresh 384 MiB virtio disk, FFS, mounted again after the file is written);
//!   EmiBSD runs it from the ramdisk, OpenBSD fetches it from `diff-openbsd`'s HTTP server.
//!   Each boot runs it twice: a warm-up pass, checked and discarded, then, with the scratch
//!   disk mounted again (the file cold once more), the pass that is recorded;
//! - unless `--no-tcpbench`: tcpbench(1), `smoke-tcpbench`'s two VMs on `smoke2`'s private
//!   link, for each system: B is the client of A's server, a 2 s run retried until A answers,
//!   one 10 s run as a warm-up, discarded, and then `--reps` runs of 10 s in the same boot
//!   (Mbps, tcpbench's average). pf is disabled on both first.
//!
//! Per measurement and side the worst sample ([`DISCARD_WORST`]: the slowest for times, the
//! lowest for Mbps) is left out; the table on stdout and `<run dir>/<arch>/report.toml` give,
//! over the samples kept, the median,
//! the minimum and the spread (`(max - min) / median`) of each system and the ratio of the
//! medians, EmiBSD / OpenBSD; the ratio is refused (and the report says why) when either
//! spread exceeds 15 %. The report also records the kernel's commit and SHA-256, the QEMU
//! command lines, the date, the method (`reps`, `discard_worst`) and every raw sample, the
//! discarded one marked (`emibsd_discarded`, `openbsd_discarded`: its index), so a later run
//! compares with it: `compare`, which refuses two reports of different methods,
//! prints, for EmiBSD's medians, a `Bench:` trailer line (`git-commits.md`) per measurement
//! that moved by more than the spread of either report. Under TCG absolute numbers are
//! indicative only; the ratio between two kernels on the same emulation is the measure.
//! Around every run (each side, tcpbench's too) the host's one-minute load average at its
//! start and end and its busiest processes but QEMU are printed and kept (`[[host]]` in the
//! report): a diagnostic for reading a sample against the host, which changes no sample, rule
//! or limit (a debugger traced a step in amd64's `pipe` and `fileread` to host contention).
//! `just bench` sets `EMIBSD_RUN_DIR` to `target/bench`; logs are `<run dir>/<arch>/*.log`.
//!
//! `--profile release` (decision 42, a diagnostic beside the baseline, never in its place)
//! boots the kernel of the release profile, `target/<triple>/release/bsd` (Cargo.toml's
//! `[profile.release]`: LTO, one codegen unit, symbols kept), instead of the debug one;
//! init, `abi_openbsd.ko` and the ramdisk are the same files as with `debug`, so only the
//! kernel changes profile. The report records it (`profile = "release"`; a report without
//! the key is a debug one) and the table's first line says it; `just bench-release` builds
//! that kernel and sets `EMIBSD_RUN_DIR` to `target/bench-release`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::boot::{self, Arch, Disks, VmLink};
use crate::diffopenbsd::serial::Vm;
use crate::diffopenbsd::{self, Boot, http};

/// Runs per system and arch.
const DEFAULT_REPS: usize = 10;
/// How many samples per measurement and side are left out of the statistics: the worst (the
/// slowest for times, the lowest for Mbps). `report.toml` records it, and `compare` refuses two
/// reports made with different values.
const DISCARD_WORST: u32 = 1;
/// The largest spread, in percent, at which a ratio is printed.
const SPREAD_LIMIT: f64 = 15.0;
/// The scratch disk: room for FFS and the 256 MiB file.
const SCRATCH_SIZE: u64 = 384 << 20;
/// The cold file `fileread` reads, and its size.
const BENCH_FILE: &str = "/mnt/bench.dat";
const FILE_MIB: u32 = 256;
/// Length of each tcpbench(1) run.
const TCPBENCH_SECS: u32 = 10;
/// Where the program is on each system.
const EMIBSD_BENCH: &str = "/usr/bin/bench";
const OPENBSD_BENCH: &str = "/usr/local/bin/bench";
/// What [`sh`] prints after a command: `@@RC <status>`.
const RC_MARK: &str = "@@RC ";
/// What `bench.c` prints per measurement: `@@BENCH <name> <ns per op> <ops>`.
const BENCH_MARK: &str = "@@BENCH ";
/// The `report.toml` format; a change of its fields bumps it (an optional field with a
/// default that keeps older reports readable, as `discard_worst`, `host` and `profile`, does
/// not).
const SCHEMA: u32 = 1;
/// The kernel's build profile when `--profile` is not given, and in reports without the key.
const DEFAULT_PROFILE: &str = "debug";
/// The build profiles `--profile` takes: the cargo profiles' directories under
/// `target/<triple>/`.
const PROFILES: &[&str] = &["debug", "release"];

/// One measurement: its name, unit and which way is better.
struct Kind {
    name: &'static str,
    unit: &'static str,
    better: &'static str,
}

/// Every measurement, in the order of the table.
const KINDS: &[Kind] = &[
    Kind {
        name: "boot",
        unit: "s",
        better: "lower",
    },
    Kind {
        name: "pagefault",
        unit: "ns/op",
        better: "lower",
    },
    Kind {
        name: "forkexit",
        unit: "ns/op",
        better: "lower",
    },
    Kind {
        name: "forkexec",
        unit: "ns/op",
        better: "lower",
    },
    Kind {
        name: "pipe",
        unit: "ns/op",
        better: "lower",
    },
    Kind {
        name: "fileread",
        unit: "ns/op",
        better: "lower",
    },
    Kind {
        name: "tcpbench",
        unit: "Mbps",
        better: "higher",
    },
];

/// The measurements of `bench all`, in its order.
const MICRO: &[&str] = &["pagefault", "forkexit", "forkexec", "pipe", "fileread"];

/// Median, minimum, maximum and spread of one system's samples.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct Stats {
    median: f64,
    min: f64,
    max: f64,
    /// `(max - min) / median`, in percent.
    spread_percent: f64,
}

/// One measurement of a report: the raw samples of both systems and what follows from them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Measurement {
    name: String,
    unit: String,
    better: String,
    /// Every sample, in run order, the discarded one included.
    emibsd: Vec<f64>,
    openbsd: Vec<f64>,
    /// The index in `emibsd` of the sample left out of `emibsd_stats` ([`DISCARD_WORST`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emibsd_discarded: Option<usize>,
    /// The same for `openbsd`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    openbsd_discarded: Option<usize>,
    /// EmiBSD's median over OpenBSD's, when both spreads are at most [`SPREAD_LIMIT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ratio: Option<f64>,
    /// Why there is no ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emibsd_stats: Option<Stats>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    openbsd_stats: Option<Stats>,
}

/// The QEMU command lines of the run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct QemuLines {
    emibsd: String,
    openbsd: String,
    #[serde(default)]
    emibsd_link: Vec<String>,
    #[serde(default)]
    openbsd_link: Vec<String>,
}

/// `<run dir>/<arch>/report.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Report {
    schema: u32,
    arch: String,
    /// UTC, `YYYY-MM-DDTHH:MM:SSZ`.
    date: String,
    /// `git rev-parse HEAD` of the tree the kernel was built from.
    kernel_commit: String,
    /// Whether `git status --porcelain -- sys` was empty.
    kernel_tree_clean: bool,
    kernel_sha256: String,
    /// The kernel's build profile (`--profile`): `debug`, or `release` (decision 42); only
    /// the kernel's: init and `abi_openbsd.ko` are always the debug builds. Not written for
    /// `debug`, so a debug report is as it was before the option.
    #[serde(
        default = "default_profile",
        skip_serializing_if = "is_default_profile"
    )]
    profile: String,
    reps: usize,
    /// Samples per measurement and side left out of the statistics (0 in reports made before
    /// the rule).
    #[serde(default)]
    discard_worst: u32,
    smp: u32,
    memory: String,
    spread_limit_percent: f64,
    file_mib: u32,
    tcpbench_seconds: u32,
    qemu: QemuLines,
    measurement: Vec<Measurement>,
    /// The host's state around every run, for reading a sample against it: a diagnostic that
    /// changes no sample, rule or limit (empty in the reports made before it).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    host: Vec<HostRun>,
}

/// The host around one run of one system ([`host_before`], [`host_after`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct HostRun {
    /// `emibsd`, `openbsd`, or `emibsd-tcpbench`, `openbsd-tcpbench` for a tcpbench run.
    system: String,
    /// The run, from 1.
    run: usize,
    /// The host's one-minute load average at the run's start and end (`vm.loadavg`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    load_start: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    load_end: Option<f64>,
    /// The busiest processes at the run's end other than QEMU, by ps(1)'s %CPU.
    #[serde(default)]
    top: Vec<TopProc>,
}

/// One process of [`HostRun::top`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TopProc {
    name: String,
    cpu: f64,
}

/// How many processes [`HostRun::top`] keeps.
const TOP_PROCS: usize = 5;

fn default_profile() -> String {
    DEFAULT_PROFILE.to_string()
}

fn is_default_profile(p: &str) -> bool {
    p == DEFAULT_PROFILE
}

/// `--profile`'s value: one of [`PROFILES`].
fn parse_profile(v: &str) -> Result<&'static str> {
    PROFILES
        .iter()
        .copied()
        .find(|p| *p == v)
        .ok_or_else(|| format!("--profile {v}: expected debug or release").into())
}

/// The kernel `bench` boots: `target/<triple>/<profile>/bsd`.
fn kernel_path(root: &Path, arch: Arch, profile: &str) -> PathBuf {
    root.join("target")
        .join(arch.target())
        .join(profile)
        .join("bsd")
}

/// What one boot of one system measured.
struct One {
    boot: f64,
    micro: BTreeMap<String, f64>,
    qemu: String,
}

/// `cargo xtask bench ...`.
pub(crate) fn bench(root: &Path, args: &[&str]) -> Result<()> {
    if let ["compare", before, after] = args {
        let (b, a) = (
            read_report(Path::new(before))?,
            read_report(Path::new(after))?,
        );
        for line in compare(&b, &a)? {
            println!("{line}");
        }
        return Ok(());
    }
    let mut arches = Vec::new();
    let mut reps = DEFAULT_REPS;
    let mut tcpbench = true;
    let mut profile = DEFAULT_PROFILE;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match *a {
            "--arch" => arches.push(Arch::parse(it.next().ok_or("--arch needs a value")?)?),
            // `main` has recorded it (`boot::set_smp`).
            "--smp" => {
                it.next();
            }
            "--reps" => {
                let v = it.next().ok_or("--reps needs a value")?;
                reps = match v.parse::<usize>() {
                    Ok(n) if (1..=20).contains(&n) => n,
                    _ => return Err(format!("--reps {v}: expected 1 to 20").into()),
                };
            }
            "--no-tcpbench" => tcpbench = false,
            "--profile" => {
                profile = parse_profile(it.next().ok_or("--profile needs a value")?)?;
            }
            other => return Err(format!("bench: unknown argument {other:?}").into()),
        }
    }
    if arches.is_empty() {
        arches = vec![Arch::Amd64, Arch::Arm64];
    }
    // `diff-openbsd`'s configuration: four processors unless `--smp` says otherwise.
    if boot::smp().is_none() {
        boot::set_smp(Some(4));
    }
    for &arch in &arches {
        diffopenbsd::installed(root, arch).map_err(|e| {
            format!(
                "bench: the OpenBSD VM of {} is not installed ({e}); bench never installs or \
                 downloads it: copy target/openbsd/ from the main checkout, or ask the user",
                arch.name()
            )
        })?;
        let prog = bench_program(root, arch);
        if !prog.is_file() {
            return Err(format!("bench: no {}; run `just userland` first", prog.display()).into());
        }
        let kernel = kernel_path(root, arch, profile);
        if !kernel.is_file() {
            return Err(format!(
                "bench: no {}; build the {profile} kernel first (`just bench{}`)",
                kernel.display(),
                if profile == DEFAULT_PROFILE {
                    ""
                } else {
                    "-release"
                }
            )
            .into());
        }
    }
    for &arch in &arches {
        let started = Instant::now();
        let report = run_arch(root, arch, reps, tcpbench, profile)?;
        let work = work_dir(root, arch)?;
        let text = toml::to_string(&report).map_err(|e| format!("report.toml: {e}"))?;
        let rp = work.join("report.toml");
        fs::write(&rp, text).map_err(|e| format!("{}: {e}", rp.display()))?;
        let table = table(&report);
        fs::write(work.join("report.txt"), &table)
            .map_err(|e| format!("{}: {e}", work.display()))?;
        print!("{table}");
        println!(
            "xtask: bench {}: report in {} ({:.0}s)",
            arch.name(),
            rp.display(),
            started.elapsed().as_secs_f32()
        );
    }
    Ok(())
}

/// `target/userland/<arch>/root/usr/bin/bench`.
fn bench_program(root: &Path, arch: Arch) -> PathBuf {
    root.join("target/userland")
        .join(arch.name())
        .join("root/usr/bin/bench")
}

/// This run's files: `$EMIBSD_RUN_DIR/<arch>` (`just bench` sets it to `target/bench`), or
/// `target/bench/<arch>` without it.
fn work_dir(root: &Path, arch: Arch) -> Result<PathBuf> {
    let base = match std::env::var_os(boot::RUN_DIR_ENV) {
        Some(d) if !d.is_empty() => boot::run_dir(root),
        _ => root.join("target").join("bench"),
    };
    let d = base.join(arch.name());
    fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    Ok(d)
}

/// Every run of `arch`, then the tcpbench pairs; the report.
fn run_arch(root: &Path, arch: Arch, reps: usize, tcpbench: bool, profile: &str) -> Result<Report> {
    let work = work_dir(root, arch)?;
    let www = work.join("www");
    fs::create_dir_all(&www).map_err(|e| format!("{}: {e}", www.display()))?;
    let prog = bench_program(root, arch);
    fs::copy(&prog, www.join("bench")).map_err(|e| format!("{}: {e}", prog.display()))?;
    let server = http::Server::start(&www)?;
    let kernel = kernel_path(root, arch, profile);
    let init = boot::default_init(root, arch);
    let ramdisk = boot::default_ramdisk(root, arch);
    let image = boot::image_tagged(
        root,
        arch,
        Some("bench"),
        &kernel,
        None,
        init.as_deref(),
        ramdisk.as_deref(),
    )?;

    let mut emi: Vec<One> = Vec::new();
    let mut obsd: Vec<One> = Vec::new();
    let mut host = Vec::new();
    for rep in 1..=reps {
        // The order alternates, so a drift of the host's speed falls on both alike.
        for emibsd_turn in [rep % 2 == 1, rep % 2 == 0] {
            let started = Instant::now();
            let load_start = host_before();
            let one = if emibsd_turn {
                emibsd_run(root, arch, &image, &work, rep)?
            } else {
                openbsd_run(root, arch, &work, &server, rep)?
            };
            let system = if emibsd_turn { "emibsd" } else { "openbsd" };
            let h = host_after(system, rep, load_start);
            println!(
                "xtask: bench {} run {rep}/{reps} {system} host: {}",
                arch.name(),
                host_line(&h)
            );
            host.push(h);
            println!(
                "xtask: bench {} run {rep}/{reps} {}: boot {:.2}s, {} ({:.0}s)",
                arch.name(),
                if emibsd_turn { "EmiBSD" } else { "OpenBSD" },
                one.boot,
                one.micro
                    .iter()
                    .map(|(k, v)| format!("{k} {v:.0}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                started.elapsed().as_secs_f32()
            );
            if emibsd_turn {
                emi.push(one);
            } else {
                obsd.push(one);
            }
        }
    }
    drop(server);

    let mut qemu = QemuLines {
        emibsd: emi.first().map(|o| o.qemu.clone()).unwrap_or_default(),
        openbsd: obsd.first().map(|o| o.qemu.clone()).unwrap_or_default(),
        ..QemuLines::default()
    };
    let (mut tcp_emi, mut tcp_obsd) = (Vec::new(), Vec::new());
    if tcpbench {
        let e = pair_tcpbench(
            &format!("emibsd-{}-tcp", arch.name()),
            "emibsd-tcpbench",
            &work,
            reps,
            &mut host,
            &|l: &VmLink| {
                let img = boot::image_tagged(
                    root,
                    arch,
                    Some(l.tag),
                    &kernel,
                    None,
                    init.as_deref(),
                    ramdisk.as_deref(),
                )?;
                boot::qemu_command(
                    root,
                    arch,
                    &img,
                    "stdio",
                    Some(l),
                    &Disks {
                        fresh: true,
                        count: 1,
                        set: None,
                    },
                )
            },
        )?;
        let o = pair_tcpbench(
            &format!("openbsd-{}-tcp", arch.name()),
            "openbsd-tcpbench",
            &work,
            reps,
            &mut host,
            &|l: &VmLink| diffopenbsd::openbsd_qemu(root, arch, &Boot::Link(l)),
        )?;
        (tcp_emi, qemu.emibsd_link) = e;
        (tcp_obsd, qemu.openbsd_link) = o;
    }

    let mut measurement = Vec::new();
    for k in KINDS {
        let pick = |v: &[One]| -> Vec<f64> {
            v.iter()
                .filter_map(|o| match k.name {
                    "boot" => Some(o.boot),
                    n => o.micro.get(n).copied(),
                })
                .collect()
        };
        let (e, o) = match k.name {
            "tcpbench" => (tcp_emi.clone(), tcp_obsd.clone()),
            _ => (pick(&emi), pick(&obsd)),
        };
        if e.is_empty() && o.is_empty() {
            continue;
        }
        measurement.push(measure(k, e, o));
    }
    let clean = git(root, &["status", "--porcelain", "--", "sys"])?
        .trim()
        .is_empty();
    Ok(Report {
        schema: SCHEMA,
        arch: arch.name().to_string(),
        date: utc_now(),
        kernel_commit: git(root, &["rev-parse", "HEAD"])?.trim().to_string(),
        kernel_tree_clean: clean,
        kernel_sha256: diffopenbsd::sha256(&kernel)?,
        profile: profile.to_string(),
        reps,
        discard_worst: DISCARD_WORST,
        smp: boot::smp().unwrap_or(1),
        memory: boot::VM_MEMORY.to_string(),
        spread_limit_percent: SPREAD_LIMIT,
        file_mib: FILE_MIB,
        tcpbench_seconds: TCPBENCH_SECS,
        qemu,
        measurement,
        host,
    })
}

/// The host's one-minute load average now, taken at a run's start.
fn host_before() -> Option<f64> {
    let out = Command::new("sysctl")
        .args(["-n", "vm.loadavg"])
        .output()
        .ok()?;
    parse_loadavg(&String::from_utf8_lossy(&out.stdout))
}

/// The host at a run's end: the load average again and the busiest processes but QEMU.
fn host_after(system: &str, run: usize, load_start: Option<f64>) -> HostRun {
    let top = Command::new("ps")
        .args(["-axo", "pcpu=,comm="])
        .output()
        .map(|o| parse_top(&String::from_utf8_lossy(&o.stdout), TOP_PROCS))
        .unwrap_or_default();
    HostRun {
        system: system.to_string(),
        run,
        load_start,
        load_end: host_before(),
        top,
    }
}

/// `sysctl -n vm.loadavg`'s `{ 1.23 0.98 0.75 }`: the first, the last minute's.
fn parse_loadavg(text: &str) -> Option<f64> {
    text.trim()
        .trim_start_matches('{')
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// `ps -axo pcpu=,comm=`'s lines: the `n` busiest processes by %CPU, by the last component
/// of their path, QEMU left out (it is the run itself).
fn parse_top(text: &str, n: usize) -> Vec<TopProc> {
    let mut v: Vec<TopProc> = text
        .lines()
        .filter_map(|l| {
            let (cpu, comm) = l.trim().split_once(char::is_whitespace)?;
            let cpu: f64 = cpu.parse().ok()?;
            let comm = comm.trim();
            let name = comm.rsplit('/').next().unwrap_or(comm).to_string();
            (!name.starts_with("qemu-system")).then_some(TopProc { name, cpu })
        })
        .collect();
    v.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    v.truncate(n);
    v
}

/// One line of the host's state around a run, as printed after it.
fn host_line(h: &HostRun) -> String {
    let load = |l: Option<f64>| l.map_or("?".to_string(), |x| format!("{x:.2}"));
    format!(
        "load {} -> {}; top {}",
        load(h.load_start),
        load(h.load_end),
        h.top
            .iter()
            .map(|p| format!("{} {:.0}%", p.name, p.cpu))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// One boot of EmiBSD: the smokes' MP kernel and ramdisk, a fresh scratch disk as `sd0`.
fn emibsd_run(root: &Path, arch: Arch, image: &Path, work: &Path, rep: usize) -> Result<One> {
    let disk = boot::disk_path_n(root, arch, Some("bench"), 0);
    let make = || -> Result<Command> {
        scratch(&disk)?;
        boot::qemu_command(
            root,
            arch,
            image,
            "stdio",
            None,
            &Disks {
                fresh: false,
                count: 1,
                set: Some("bench"),
            },
        )
    };
    let (mut vm, boot_s, qemu) = boot_login(
        &format!("emibsd-{}", arch.name()),
        &work.join(format!("emibsd-{rep}.log")),
        &make,
        boot::time_limit(Duration::from_secs(600)),
    )?;
    let micro = run_program(&mut vm, "sd0", EMIBSD_BENCH)?;
    Ok(One {
        boot: boot_s,
        micro,
        qemu,
    })
}

/// One boot of OpenBSD: the installed disk under `-snapshot`, a fresh scratch disk as `sd1`,
/// the program fetched from `server`.
fn openbsd_run(
    root: &Path,
    arch: Arch,
    work: &Path,
    server: &http::Server,
    rep: usize,
) -> Result<One> {
    let disk = work.join("openbsd-scratch.img");
    let make = || -> Result<Command> {
        scratch(&disk)?;
        diffopenbsd::openbsd_qemu(root, arch, &Boot::Bench(&disk))
    };
    let (mut vm, boot_s, qemu) = boot_login(
        &format!("openbsd-{}", arch.name()),
        &work.join(format!("openbsd-{rep}.log")),
        &make,
        boot::time_limit(Duration::from_secs(900)),
    )?;
    sh(
        &mut vm,
        &format!(
            "until ftp -V -o {OPENBSD_BENCH} {} >/dev/null 2>&1; do sleep 1; done; \
             chmod 755 {OPENBSD_BENCH}",
            server.url("bench")
        ),
        boot::time_limit(Duration::from_secs(600)),
    )?;
    let micro = run_program(&mut vm, "sd1", OPENBSD_BENCH)?;
    Ok(One {
        boot: boot_s,
        micro,
        qemu,
    })
}

/// A blank scratch disk of [`SCRATCH_SIZE`] at `path` (sparse), made afresh.
fn scratch(path: &Path) -> Result<()> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(err(e).into()),
    }
    let f = fs::File::create(path).map_err(err)?;
    f.set_len(SCRATCH_SIZE).map_err(err)?;
    Ok(())
}

/// The scratch disk `d` labelled, formatted and mounted on /mnt, the 256 MiB file written,
/// then the file system mounted again, so the file is read cold.
fn setup_cmd(d: &str) -> String {
    format!(
        "umount /mnt >/dev/null 2>&1; fdisk -iy -f /dev/r{d}c {d} >/dev/null && \
         disklabel -w -A {d} && newfs {d}a >/dev/null && mount /dev/{d}a /mnt && \
         dd if=/dev/zero of={BENCH_FILE} bs=1m count={FILE_MIB} 2>/dev/null && umount /mnt && \
         mount /dev/{d}a /mnt && ls -l {BENCH_FILE}"
    )
}

/// The scratch disk `d` mounted again, so the file is cold once more.
fn remount_cmd(d: &str) -> String {
    format!("umount /mnt && mount /dev/{d}a /mnt")
}

/// The scratch disk prepared, `bench all` run once as a warm-up (checked, then discarded),
/// the disk mounted again, then `bench all` run for the record: its measurements.
fn run_program(vm: &mut Vm, disk: &str, prog: &str) -> Result<BTreeMap<String, f64>> {
    sh(
        vm,
        &setup_cmd(disk),
        boot::time_limit(Duration::from_secs(900)),
    )?;
    let warm = sh(
        vm,
        &format!("{prog} all {BENCH_FILE}"),
        boot::time_limit(Duration::from_secs(1800)),
    )?;
    parse_bench(&warm).map_err(|e| format!("{}: warm-up: {e}", vm.name))?;
    sh(
        vm,
        &remount_cmd(disk),
        boot::time_limit(Duration::from_secs(300)),
    )?;
    let out = sh(
        vm,
        &format!("{prog} all {BENCH_FILE}"),
        boot::time_limit(Duration::from_secs(1800)),
    )?;
    parse_bench(&out).map_err(|e| format!("{}: {e}", vm.name).into())
}

/// Boots the VM `make` describes and logs in as root; the VM, the seconds from QEMU's start
/// to `login:`, and the QEMU command line. A boot that ends in one of
/// `boot::FIRMWARE_FLAKES` before the kernel ran is booted once more, as a smoke's is (its
/// log is `<log>.retry`).
fn boot_login(
    name: &str,
    log: &Path,
    make: &dyn Fn() -> Result<Command>,
    limit: Duration,
) -> Result<(Vm, f64, String)> {
    let mut log = log.to_path_buf();
    let mut retried = false;
    loop {
        let cmd = make()?;
        let line = boot::command_line(&cmd);
        let started = Instant::now();
        let mut vm = Vm::spawn(name, cmd, log.clone())?;
        match vm.wait_for("login:", limit) {
            Ok(()) => {
                let secs = started.elapsed().as_secs_f64();
                vm.send("root\n")?;
                vm.wait_for("Password:", Duration::from_secs(120))?;
                vm.send(&format!("{}\n", diffopenbsd::ROOT_PASSWORD))?;
                vm.wait_for("# ", Duration::from_secs(300))?;
                return Ok((vm, round3(secs), line));
            }
            Err(e) => {
                if !retried && let Some(id) = boot::firmware_flake(&vm.text()) {
                    println!("xtask: {name}: {} ({id})", boot::FIRMWARE_RETRY_MARKER);
                    retried = true;
                    log.set_extension("log.retry");
                    continue;
                }
                return Err(e);
            }
        }
    }
}

/// Types `cmd` at the shell and waits for its status; its output (the echoed command
/// included) when the status is 0.
fn sh(vm: &mut Vm, cmd: &str, limit: Duration) -> Result<String> {
    let mark = vm.mark();
    // The quotes keep the echoed command from showing the marker itself.
    vm.send(&format!("{cmd}; echo \"@@R\"\"C $?\"\n"))?;
    vm.wait_for(RC_MARK, limit)?;
    vm.wait_for("\n", Duration::from_secs(60))?;
    let out = vm.since(mark);
    match parse_rc(&out) {
        Some(0) => Ok(out),
        Some(n) => Err(format!("{}: `{cmd}` exited with {n}", vm.name).into()),
        None => Err(format!("{}: no status after `{cmd}`", vm.name).into()),
    }
}

/// Two VMs on the private link: A serves, B runs the warm-up and `reps` runs of tcpbench(1)
/// against it; B's Mbps per run and the two QEMU command lines.
fn pair_tcpbench(
    name: &str,
    system: &str,
    work: &Path,
    reps: usize,
    host: &mut Vec<HostRun>,
    make: &dyn Fn(&VmLink) -> Result<Command>,
) -> Result<(Vec<f64>, Vec<String>)> {
    let ports = crate::twovm::free_udp_ports()?;
    let links = [
        crate::twovm::vm_link(0, ports),
        crate::twovm::vm_link(1, ports),
    ];
    let mut lines = Vec::new();
    let mut vms = Vec::new();
    for l in &links {
        let cmd = make(l)?;
        lines.push(boot::command_line(&cmd));
        vms.push(Vm::spawn(
            &format!("{name}-{}", l.tag),
            cmd,
            work.join(format!("{name}-{}.log", l.tag)),
        )?);
    }
    for vm in &mut vms {
        vm.wait_for("login:", boot::time_limit(Duration::from_secs(900)))?;
        vm.send("root\n")?;
        vm.wait_for("Password:", Duration::from_secs(120))?;
        vm.send(&format!("{}\n", diffopenbsd::ROOT_PASSWORD))?;
        vm.wait_for("# ", Duration::from_secs(300))?;
    }
    let limit = boot::time_limit(Duration::from_secs(300));
    let [a, b] = vms.as_mut_slice() else {
        return Err("bench: two VMs expected".into());
    };
    sh(
        a,
        "pfctl -d >/dev/null 2>&1; ifconfig vio1 inet 192.168.77.1/24 up",
        limit,
    )?;
    sh(a, "tcpbench -s >/dev/null 2>&1 & sleep 1", limit)?;
    sh(
        b,
        "pfctl -d >/dev/null 2>&1; ifconfig vio1 inet 192.168.77.2/24 up",
        limit,
    )?;
    // A first short run, retried until A's server answers; then a whole run as a warm-up,
    // checked and discarded.
    sh(
        b,
        "until tcpbench -t 2 192.168.77.1 >/dev/null 2>&1; do sleep 1; done",
        limit,
    )?;
    let warm = sh(
        b,
        &format!("tcpbench -t {TCPBENCH_SECS} 192.168.77.1"),
        limit,
    )?;
    let v = parse_tcpbench(&warm)
        .ok_or_else(|| format!("{}: no tcpbench summary in the warm-up", b.name))?;
    println!("xtask: bench {name} warm-up (discarded): {v:.1} Mbps");
    let mut mbps = Vec::new();
    for rep in 1..=reps {
        let load_start = host_before();
        let out = sh(
            b,
            &format!("tcpbench -t {TCPBENCH_SECS} 192.168.77.1"),
            limit,
        )?;
        let v = parse_tcpbench(&out)
            .ok_or_else(|| format!("{}: no tcpbench summary in run {rep}", b.name))?;
        let h = host_after(system, rep, load_start);
        println!(
            "xtask: bench {name} run {rep}/{reps}: {v:.1} Mbps; host: {}",
            host_line(&h)
        );
        host.push(h);
        mbps.push(v);
    }
    Ok((mbps, lines))
}

/// The status a [`sh`] marker gives (the last one in `text`).
fn parse_rc(text: &str) -> Option<i32> {
    let at = text.rfind(RC_MARK)?;
    let digits: String = text[at + RC_MARK.len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// `bench all`'s lines: ns per op of every [`MICRO`] measurement.
fn parse_bench(text: &str) -> Result<BTreeMap<String, f64>> {
    let mut m = BTreeMap::new();
    for line in text.lines() {
        let Some(at) = line.find(BENCH_MARK) else {
            continue;
        };
        let f: Vec<&str> = line[at + BENCH_MARK.len()..].split_whitespace().collect();
        let (name, ns, ops) = match f.as_slice() {
            [name, ns, ops] => (*name, *ns, *ops),
            _ => return Err(format!("bench: malformed line {line:?}").into()),
        };
        let ns: f64 = ns
            .parse()
            .map_err(|_| format!("bench: malformed line {line:?}"))?;
        let ops: u64 = ops
            .parse()
            .map_err(|_| format!("bench: malformed line {line:?}"))?;
        if ops == 0 {
            return Err(format!("bench: no operations in {line:?}").into());
        }
        m.insert(name.to_string(), ns);
    }
    for name in MICRO {
        if !m.contains_key(*name) {
            return Err(format!("bench: no {name} line").into());
        }
    }
    Ok(m)
}

/// tcpbench(1)'s average Mbps, from the last `bandwidth min/avg/max/std-dev = ` line.
fn parse_tcpbench(text: &str) -> Option<f64> {
    const P: &str = "bandwidth min/avg/max/std-dev = ";
    let rest = text
        .lines()
        .rev()
        .find_map(|l| l.find(P).map(|i| &l[i + P.len()..]))?;
    let values = rest.trim().trim_end_matches("Mbps").trim();
    values.split('/').nth(1)?.parse().ok()
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The statistics of `v`, `None` when it is empty.
fn stats(v: &[f64]) -> Option<Stats> {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    let (min, max) = (*s.first()?, *s.last()?);
    let median = if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    };
    let spread = if max == min {
        0.0
    } else if median > 0.0 {
        (max - min) / median * 100.0
    } else {
        f64::INFINITY
    };
    Some(Stats {
        median: round3(median),
        min: round3(min),
        max: round3(max),
        spread_percent: round3(spread),
    })
}

/// The index of the worst of `v` for a measurement where `better` (`lower` or `higher`) is
/// better: the largest, or the smallest; the first of equals. `None` with fewer than two
/// samples, which leave nothing to compare.
fn worst(v: &[f64], better: &str) -> Option<usize> {
    if v.len() < 2 {
        return None;
    }
    let mut at = 0;
    for (i, x) in v.iter().enumerate() {
        let is_worse = match better {
            "higher" => *x < v[at],
            _ => *x > v[at],
        };
        if is_worse {
            at = i;
        }
    }
    Some(at)
}

/// `v` without the sample at `skip`.
fn kept(v: &[f64], skip: Option<usize>) -> Vec<f64> {
    v.iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != skip)
        .map(|(_, x)| *x)
        .collect()
}

/// The measurement `k` of the samples of both systems: the worst sample of each side
/// discarded ([`DISCARD_WORST`]), the statistics of the others, and the ratio or the reason
/// for none.
fn measure(k: &Kind, emibsd: Vec<f64>, openbsd: Vec<f64>) -> Measurement {
    let (ed, od) = if DISCARD_WORST > 0 {
        (worst(&emibsd, k.better), worst(&openbsd, k.better))
    } else {
        (None, None)
    };
    let (es, os) = (stats(&kept(&emibsd, ed)), stats(&kept(&openbsd, od)));
    let (ratio, note) = match (es, os) {
        (Some(e), Some(o))
            if e.spread_percent <= SPREAD_LIMIT && o.spread_percent <= SPREAD_LIMIT =>
        {
            (Some(round3(e.median / o.median)), None)
        }
        (Some(e), Some(o)) => (
            None,
            Some(format!(
                "no ratio: spread {:.1} % (EmiBSD), {:.1} % (OpenBSD), over {SPREAD_LIMIT} %",
                e.spread_percent, o.spread_percent
            )),
        ),
        (Some(_), None) => (None, Some("EmiBSD only".to_string())),
        (None, Some(_)) => (None, Some("OpenBSD only".to_string())),
        (None, None) => (None, Some("not measured".to_string())),
    };
    Measurement {
        name: k.name.to_string(),
        unit: k.unit.to_string(),
        better: k.better.to_string(),
        emibsd,
        openbsd,
        emibsd_discarded: ed,
        openbsd_discarded: od,
        ratio,
        note,
        emibsd_stats: es,
        openbsd_stats: os,
    }
}

/// `v` as the table shows values of `unit`.
fn fmt_value(v: f64, unit: &str) -> String {
    match unit {
        "s" => format!("{v:.2}"),
        "Mbps" => format!("{v:.1}"),
        _ => format!("{v:.0}"),
    }
}

/// The report as the table printed on stdout and kept in `report.txt`.
fn table(r: &Report) -> String {
    let mut s = format!(
        "bench {}: kernel {}{}{} ({} runs, the worst {} of each side discarded, -smp {}, -m {}, {})\n\
         median, min and spread = (max - min) / median of the samples kept; \
         ratio = EmiBSD median / OpenBSD median, printed when both spreads are at most {} %\n",
        r.arch,
        r.kernel_commit.get(..12).unwrap_or(&r.kernel_commit),
        if r.kernel_tree_clean {
            ""
        } else {
            " (sys/ modified)"
        },
        if is_default_profile(&r.profile) {
            String::new()
        } else {
            format!(", {} profile", r.profile)
        },
        r.reps,
        r.discard_worst,
        r.smp,
        r.memory,
        r.date,
        r.spread_limit_percent
    );
    s.push_str(&format!(
        "{:<10} {:>6} | {:>10} {:>10} {:>7} | {:>10} {:>10} {:>7} | {:>6}\n",
        "measure", "unit", "EmiBSD", "min", "spread", "OpenBSD", "min", "spread", "ratio"
    ));
    let side = |st: Option<Stats>, unit: &str| match st {
        Some(st) => format!(
            "{:>10} {:>10} {:>6.1}%",
            fmt_value(st.median, unit),
            fmt_value(st.min, unit),
            st.spread_percent
        ),
        None => format!("{:>10} {:>10} {:>7}", "-", "-", "-"),
    };
    for m in &r.measurement {
        let ratio = match (m.ratio, &m.note) {
            (Some(x), _) => format!("{x:.3}"),
            (None, Some(n)) if n.starts_with("no ratio") => "spread".to_string(),
            _ => "n/a".to_string(),
        };
        s.push_str(&format!(
            "{:<10} {:>6} | {} | {} | {:>6}\n",
            m.name,
            m.unit,
            side(m.emibsd_stats, &m.unit),
            side(m.openbsd_stats, &m.unit),
            ratio
        ));
    }
    for m in &r.measurement {
        let gone = |v: &[f64], d: Option<usize>, who: &str| {
            d.and_then(|i| v.get(i).map(|x| (i, *x)))
                .map(|(i, x)| format!("{who} run {} ({})", i + 1, fmt_value(x, &m.unit)))
        };
        let both: Vec<String> = [
            gone(&m.emibsd, m.emibsd_discarded, "EmiBSD"),
            gone(&m.openbsd, m.openbsd_discarded, "OpenBSD"),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !both.is_empty() {
            s.push_str(&format!("{}: discarded {}\n", m.name, both.join(", ")));
        }
    }
    for m in &r.measurement {
        if let Some(n) = &m.note {
            s.push_str(&format!("{}: {n}\n", m.name));
        }
    }
    s
}

fn read_report(p: &Path) -> Result<Report> {
    let text = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let r: Report = toml::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))?;
    if r.schema != SCHEMA {
        return Err(format!("{}: schema {}, expected {SCHEMA}", p.display(), r.schema).into());
    }
    Ok(r)
}

/// `compare`: per measurement of EmiBSD, a `Bench: <name>-<arch> <before> -> <after> <unit>`
/// line when its median moved by more than the larger spread of the two reports; otherwise
/// a line that says why not.
fn compare(before: &Report, after: &Report) -> Result<Vec<String>> {
    if before.arch != after.arch {
        return Err(format!(
            "bench compare: {} against {}: not the same arch",
            before.arch, after.arch
        )
        .into());
    }
    let method = |r: &Report| (r.reps, r.discard_worst, r.file_mib, r.tcpbench_seconds);
    if method(before) != method(after) {
        return Err(format!(
            "bench compare: not the same method (runs, discarded, file MiB, tcpbench s): \
             {:?} against {:?}",
            method(before),
            method(after)
        )
        .into());
    }
    let mut out = Vec::new();
    if before.profile != after.profile {
        out.push(format!(
            "profile: {} -> {}: the kernels differ in build profile, not only in code",
            before.profile, after.profile
        ));
    }
    for a in &after.measurement {
        let unit = a.unit.trim_end_matches("/op");
        let b = before.measurement.iter().find(|b| b.name == a.name);
        let (Some(sb), Some(sa)) = (b.and_then(|b| b.emibsd_stats), a.emibsd_stats) else {
            out.push(format!("{}: not in both reports", a.name));
            continue;
        };
        let (vb, va) = (fmt_value(sb.median, unit), fmt_value(sa.median, unit));
        if sb.spread_percent > SPREAD_LIMIT || sa.spread_percent > SPREAD_LIMIT {
            out.push(format!(
                "{}: {vb} -> {va} {unit}: not comparable, spread over {SPREAD_LIMIT} %",
                a.name
            ));
            continue;
        }
        let change = (sa.median - sb.median) / sb.median * 100.0;
        let noise = sb.spread_percent.max(sa.spread_percent);
        if change.abs() > noise {
            out.push(format!(
                "Bench: {}-{} {vb} -> {va} {unit}",
                a.name, after.arch
            ));
        } else {
            out.push(format!(
                "{}: {vb} -> {va} {unit} ({change:+.1} %), within the spread ({noise:.1} %)",
                a.name
            ));
        }
    }
    Ok(out)
}

/// `git -C <root> <args>`'s output.
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!("git {}: {}", args.join(" "), out.status).into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The time now, UTC, `YYYY-MM-DDTHH:MM:SSZ`.
fn utc_now() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = crate::iso9660::ymdhms(i64::try_from(t).unwrap_or(0));
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    const BENCH_OUT: &str = "# /usr/bin/bench all /mnt/bench.dat; echo \"@@R\"\"C $?\"\r\n\
        @@BENCH pagefault 5120 32768\r\n@@BENCH forkexit 812345 500\r\n\
        @@BENCH forkexec 2345678 200\r\n@@BENCH pipe 45678 5000\r\n\
        @@BENCH fileread 345678 1024\r\n@@RC 0\r\n";

    #[test]
    fn rc_marker_is_read_and_the_echo_is_not() {
        assert_eq!(parse_rc(BENCH_OUT), Some(0));
        assert_eq!(
            parse_rc("x; echo \"@@R\"\"C $?\"\r\n@@RC 127\r\n"),
            Some(127)
        );
        assert_eq!(parse_rc("x; echo \"@@R\"\"C $?\"\r\n"), None);
    }

    #[test]
    fn bench_lines_parse() {
        let m = parse_bench(BENCH_OUT).unwrap();
        assert_eq!(m.len(), 5);
        assert_eq!(m["pagefault"], 5120.0);
        assert_eq!(m["fileread"], 345678.0);
        // A missing measurement, a malformed line and no operations are errors.
        assert!(parse_bench("@@BENCH pagefault 1 1\n").is_err());
        assert!(parse_bench(&BENCH_OUT.replace("pipe 45678 5000", "pipe x 5000")).is_err());
        assert!(parse_bench(&BENCH_OUT.replace("pipe 45678 5000", "pipe 45678")).is_err());
        assert!(parse_bench(&BENCH_OUT.replace("pipe 45678 5000", "pipe 45678 0")).is_err());
    }

    #[test]
    fn tcpbench_summary_gives_the_average() {
        let t = "Conn:   1 Mbps:      400.000 Peak Mbps:      450.000 Avg Mbps:      400.000\r\n\
                 --- 192.168.77.1 tcpbench statistics ---\r\n\
                 500000000 bytes sent over 10.001 seconds\r\n\
                 bandwidth min/avg/max/std-dev = 350.125/398.500/450.000/30.123 Mbps\r\n";
        assert_eq!(parse_tcpbench(t), Some(398.5));
        assert_eq!(parse_tcpbench("no summary\n"), None);
    }

    #[test]
    fn stats_median_min_and_spread() {
        let s = stats(&[10.0, 12.0, 11.0, 9.0, 10.0]).unwrap();
        assert_eq!((s.median, s.min, s.max), (10.0, 9.0, 12.0));
        assert_eq!(s.spread_percent, 30.0);
        let s = stats(&[4.0, 2.0]).unwrap();
        assert_eq!((s.median, s.spread_percent), (3.0, 66.667));
        assert_eq!(stats(&[7.0]).unwrap().spread_percent, 0.0);
        assert_eq!(stats(&[]), None);
    }

    #[test]
    fn worst_is_the_slowest_or_the_lowest() {
        assert_eq!(worst(&[3.0, 1.0, 3.0], "lower"), Some(0));
        assert_eq!(worst(&[1.0, 5.0, 2.0], "lower"), Some(1));
        assert_eq!(worst(&[5.0, 2.0, 9.0], "higher"), Some(1));
        assert_eq!(worst(&[7.0], "lower"), None);
        assert_eq!(worst(&[], "higher"), None);
        assert_eq!(kept(&[1.0, 2.0, 3.0], Some(1)), vec![1.0, 3.0]);
        assert_eq!(kept(&[1.0, 2.0], None), vec![1.0, 2.0]);
    }

    #[test]
    fn ratio_only_within_the_spread_limit() {
        let k = &KINDS[1];
        // The worst of each side (500, 101) is left out of the statistics.
        let m = measure(
            k,
            vec![200.0, 202.0, 198.0, 500.0],
            vec![100.0, 101.0, 99.0, 100.0],
        );
        assert_eq!(
            (m.emibsd_discarded, m.openbsd_discarded),
            (Some(3), Some(1))
        );
        assert_eq!(m.emibsd.len(), 4);
        assert_eq!(m.emibsd_stats.unwrap().max, 202.0);
        assert_eq!(m.ratio, Some(2.0));
        assert_eq!(m.note, None);
        // One bad sample is forgiven, two are not.
        let m = measure(
            k,
            vec![200.0, 260.0, 198.0, 300.0],
            vec![100.0, 101.0, 99.0, 100.0],
        );
        assert_eq!(m.emibsd_discarded, Some(3));
        assert_eq!(m.ratio, None);
        assert!(
            m.note
                .as_deref()
                .unwrap()
                .starts_with("no ratio: spread 31.0 % (EmiBSD)")
        );
        // Mbps: the lowest is the worst.
        let m = measure(&KINDS[6], vec![500.0, 400.0, 510.0], vec![]);
        assert_eq!(m.emibsd_discarded, Some(1));
        assert_eq!(m.emibsd_stats.unwrap().median, 505.0);
        assert_eq!(m.note.as_deref(), Some("EmiBSD only"));
        let m = measure(&KINDS[6], vec![500.0], vec![]);
        assert_eq!(m.emibsd_discarded, None);
    }

    fn report(medians: &[(&str, &[f64], &[f64])]) -> Report {
        Report {
            schema: SCHEMA,
            arch: "amd64".into(),
            date: "2026-10-17T18:00:00Z".into(),
            kernel_commit: "20035fd0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            kernel_tree_clean: true,
            kernel_sha256: "ab".repeat(32),
            profile: DEFAULT_PROFILE.into(),
            reps: 3,
            discard_worst: DISCARD_WORST,
            smp: 4,
            memory: "512M".into(),
            spread_limit_percent: SPREAD_LIMIT,
            file_mib: FILE_MIB,
            tcpbench_seconds: TCPBENCH_SECS,
            qemu: QemuLines {
                emibsd: "qemu-system-x86_64 -m 512M".into(),
                openbsd: "qemu-system-x86_64 -m 512M -snapshot".into(),
                emibsd_link: vec!["a".into(), "b".into()],
                openbsd_link: Vec::new(),
            },
            measurement: medians
                .iter()
                .map(|(n, e, o)| {
                    let k = KINDS.iter().find(|k| k.name == *n).unwrap();
                    measure(k, e.to_vec(), o.to_vec())
                })
                .collect(),
            host: vec![HostRun {
                system: "emibsd".into(),
                run: 1,
                load_start: Some(2.5),
                load_end: None,
                top: vec![TopProc {
                    name: "cpptools".into(),
                    cpu: 61.5,
                }],
            }],
        }
    }

    #[test]
    fn host_load_and_top_parse() {
        assert_eq!(parse_loadavg("{ 3.58 4.84 5.64 }\n"), Some(3.58));
        assert_eq!(parse_loadavg(""), None);
        let ps = " 43.0 /Users/x/.vscode/extensions/cpptools-1.34/bin/cpptools\n\
                  180.2 /opt/homebrew/bin/qemu-system-x86_64\n\
                   6.0 /Applications/Creative Cloud.app/Contents/MacOS/Creative Cloud UI Helper\n\
                  13.6 /System/Library/WindowServer\n\
                   0.0 launchd\n\
                  junk\n";
        let top = parse_top(ps, 3);
        let names: Vec<&str> = top.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            ["cpptools", "WindowServer", "Creative Cloud UI Helper"]
        );
        assert_eq!(top[0].cpu, 43.0);
        let h = HostRun {
            system: "openbsd".into(),
            run: 4,
            load_start: Some(3.5),
            load_end: None,
            top,
        };
        assert_eq!(
            host_line(&h),
            "load 3.50 -> ?; top cpptools 43%, WindowServer 14%, Creative Cloud UI Helper 6%"
        );
    }

    #[test]
    fn report_round_trips_through_toml() {
        let r = report(&[
            ("boot", &[30.5, 31.0, 30.0], &[60.0, 61.0, 59.5]),
            (
                "pagefault",
                &[5000.0, 5100.0, 4900.0],
                &[1000.0, 1300.0, 900.0, 1250.0],
            ),
            ("tcpbench", &[400.0, 410.0, 405.0], &[]),
        ]);
        let text = toml::to_string(&r).unwrap();
        assert!(text.contains("[[measurement]]"));
        assert!(text.contains("kernel_commit = \"20035fd0"));
        assert!(text.contains("discard_worst = 1"));
        assert!(text.contains("emibsd_discarded = 1"));
        let back: Report = toml::from_str(&text).unwrap();
        assert_eq!(back, r);
        assert!(text.contains("[[host]]") && text.contains("name = \"cpptools\""));
        // A report from before the diagnostic reads with no host rows.
        let old = text.split("[[host]]").next().unwrap();
        assert!(toml::from_str::<Report>(old).unwrap().host.is_empty());
        let t = table(&r);
        assert!(t.contains(
            "bench amd64: kernel 20035fd0aaaa (3 runs, the worst 1 of each side discarded, \
             -smp 4, -m 512M"
        ));
        assert!(
            t.lines()
                .any(|l| l.starts_with("boot") && l.ends_with(" 0.506"))
        );
        assert!(
            t.lines()
                .any(|l| l.starts_with("pagefault") && l.ends_with("spread"))
        );
        assert!(
            t.lines()
                .any(|l| l.starts_with("tcpbench") && l.ends_with("n/a"))
        );
        assert!(t.contains("boot: discarded EmiBSD run 2 (31.00), OpenBSD run 2 (61.00)\n"));
        assert!(t.contains("tcpbench: discarded EmiBSD run 1 (400.0)\n"));
        assert!(t.contains("tcpbench: EmiBSD only"));
    }

    #[test]
    fn compare_prints_a_bench_line_only_for_what_moved() {
        let before = report(&[
            ("boot", &[30.0, 30.3, 29.9, 35.0], &[60.0]),
            ("pagefault", &[5000.0, 5050.0, 4990.0, 9000.0], &[1000.0]),
            ("forkexit", &[800.0, 1000.0, 700.0, 1100.0], &[1000.0]),
        ]);
        let after = report(&[
            ("boot", &[30.1, 30.2, 30.0, 40.0], &[60.0]),
            ("pagefault", &[4000.0, 4020.0, 3990.0, 8000.0], &[1000.0]),
            ("forkexit", &[800.0, 810.0, 790.0, 900.0], &[1000.0]),
            ("pipe", &[1.0], &[1.0]),
        ]);
        let lines = compare(&before, &after).unwrap();
        assert!(lines[0].starts_with("boot: 30.00 -> 30.10 s (+0.3 %), within the spread"));
        assert_eq!(lines[1], "Bench: pagefault-amd64 5000 -> 4000 ns");
        assert!(lines[2].contains("not comparable"));
        assert_eq!(lines[3], "pipe: not in both reports");
        let mut arm = after.clone();
        arm.arch = "arm64".into();
        assert!(compare(&before, &arm).is_err());
        // Reports of another method are refused: runs, the discard rule, the file.
        let changes: [fn(&mut Report); 3] = [
            |r: &mut Report| r.reps = 5,
            |r: &mut Report| r.discard_worst = 0,
            |r: &mut Report| r.file_mib = 64,
        ];
        for change in changes {
            let mut other = after.clone();
            change(&mut other);
            assert!(compare(&before, &other).is_err());
        }
    }

    #[test]
    fn profile_picks_the_kernel_and_is_recorded_only_when_not_debug() {
        assert_eq!(parse_profile("debug").unwrap(), "debug");
        assert_eq!(parse_profile("release").unwrap(), "release");
        assert!(parse_profile("dev").is_err());
        assert!(parse_profile("").is_err());
        let root = Path::new("/r");
        assert_eq!(
            kernel_path(root, Arch::Amd64, "release"),
            Path::new("/r/target")
                .join(Arch::Amd64.target())
                .join("release/bsd")
        );
        assert_eq!(
            kernel_path(root, Arch::Arm64, DEFAULT_PROFILE),
            Path::new("/r/target")
                .join(Arch::Arm64.target())
                .join("debug/bsd")
        );
        // A debug report is written as before the option; one without the key reads as debug.
        let debug = report(&[("boot", &[30.0, 30.0, 31.0], &[60.0, 60.0, 61.0])]);
        let text = toml::to_string(&debug).unwrap();
        assert!(!text.contains("profile"));
        assert_eq!(toml::from_str::<Report>(&text).unwrap().profile, "debug");
        assert!(table(&debug).starts_with("bench amd64: kernel 20035fd0aaaa (3 runs"));
        let mut release = debug.clone();
        release.profile = "release".into();
        let text = toml::to_string(&release).unwrap();
        assert!(text.contains("profile = \"release\""));
        assert_eq!(toml::from_str::<Report>(&text).unwrap(), release);
        assert!(
            table(&release)
                .starts_with("bench amd64: kernel 20035fd0aaaa, release profile (3 runs")
        );
        // Two release reports compare as two debug ones do; across profiles it says so first.
        assert!(
            compare(&release, &release)
                .unwrap()
                .iter()
                .all(|l| !l.starts_with("profile"))
        );
        let lines = compare(&debug, &release).unwrap();
        assert_eq!(
            lines[0],
            "profile: debug -> release: the kernels differ in build profile, not only in code"
        );
        assert!(lines[1].starts_with("boot: 30.00 -> 30.00 s"));
    }

    #[test]
    fn setup_formats_the_named_disk() {
        let c = setup_cmd("sd1");
        assert!(c.contains("fdisk -iy -f /dev/rsd1c sd1"));
        assert!(c.contains("newfs sd1a"));
        assert!(c.contains("of=/mnt/bench.dat bs=1m count=256"));
        assert_eq!(remount_cmd("sd0"), "umount /mnt && mount /dev/sd0a /mnt");
        assert!(c.ends_with("mount /dev/sd1a /mnt && ls -l /mnt/bench.dat"));
    }

    #[test]
    fn utc_now_has_the_report_format() {
        let d = utc_now();
        assert_eq!(d.len(), 20);
        assert!(d.ends_with('Z') && d.as_bytes()[10] == b'T');
    }
}
/* </TESTS> */
