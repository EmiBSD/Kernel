# Journal

One section per milestone of `docs/ROADMAP.md`, N0 onwards: what went well, what failed, the
idioms that took more than one attempt, the rules that had to be corrected, and reproducible
numbers. Only what the history shows; `Effort` and `Time` are left for the user.

## Origin

This repository is EmiBSD.LZ's history up to `lz-origin` (`f5985f1d055a`, `docs: M16b met`,
2026-10-08), the faithful port of OpenBSD: milestones M0..M16b, 542 commits, 1860 `Upstream:`
trailers, 1042 ported and 140 wip entries of its `ports.toml`. Their journal is LZ's
`docs/JOURNAL.md` at that commit (`git show lz-origin:docs/JOURNAL.md`). From here on the port
continues in LZ and this journal records the native system.

## N0 Bootstrap

Boundary: the `docs: N0 met` commit. Range `lz-origin..` that commit (f5985f1d055a onwards).

- Went well: the governance, `lineage.toml` 1:1 from `ports.toml` and `cargo xtask lz` held
  up: every check that failed did so on its own tooling, never on the kernel. The first
  `just ci` in this tree passed all 67 smokes on its first run. The author's ISC block reached
  1200 files through LZ 44edb2c with the lz-sync machinery the milestone built, and 1017 of the
  1028 inherited modules matched LZ byte for byte with the native script as well.
- Failed: three tooling bugs from the RCS-ident drop (3a7e877), found by the first runs: `lz
  check` kept the blank line an ident left (15 false "differs from LZ"), `gen-syscalls` still
  wrote the ident line, and ten modules of licence-less C files carried an empty LICENSES zone
  without `license = "none"`. Stopping a `just ci` mid-build corrupted cargo's incremental cache
  (undefined `anon.*` symbols at link); `cargo clean -p bsd` per target fixed it. Running
  `just comp`, a subagent and the smokes at once took the load to 50 on 11 cores. The author's
  block was first planned "after" the notices, then "first"; and the push the user needed at
  once waited on a `just ci`: an explicit push order is now pushed at once.
- Idioms: none settled (N1 settles the first).
- Rules: the governance rewrite (`CLAUDE.md`, `.claude/rules/`), decisions 16 to 20
  (`docs/PHASE2.md`); the authorship rule of 2026-10-09 (the author's ISC block first in every
  `.rs`), first planned as a milestone N0b, then absorbed from LZ 44edb2c and pushed at once at
  the user's request; the milestone was dropped once it was in.
- LZ sync: `f5985f1d055a..44edb2c8323e`, 2 commits (the logo, the author's block): cherry-pick
  1, cherry-pick-conflicts 1, reimplemented 0.
- Numbers (at `lz-origin` unless said):
  - `cargo xtask unsafe-report`: kernel 8198 blocks, 1041 fn, 768 impl, 27 trait, 323 other,
    total 10357; tests 816 more. After libz's redesign (N1, already in): 10355.
  - `cargo xtask lz status`: 1028 modules, all inherited, 35 extras, 0 dropped (`git ls-files`
    of `sys/` and `tools/`: 1200 `.rs`); at this commit 1011 inherited, 4 adapted, 13 redesigned.
  - `just ci`: 67 smokes on both archs, host tests 2605 passed, 0 failed, 229 ignored, 1150 s.
  - `just ci-full`: green, 1655 s: the 67 smokes on `-smp 4`, the installer end to end on amd64,
    arm64 and arm64 ACPI (install 141, 153 and 228 s, each then booted).
  - `just diff-openbsd`: amd64 and arm64, 102 steps each, 99 equal, 3 expected (fifofs, the
    `kern.ostype` branding, core dumps), 0 unexpected.
  - `just comp`: both archs, the second (resumed) run 465 s.
  - LZ sync: 2 commits absorbed (see Rules); `lz drift --strict` 0 open.
  - Commits: `git log --oneline f5985f1d055a..` this commit.

Effort: _(user)_

Time: _(user)_

## N1 Leaves

Boundary: the `docs: N1 met` commit. Range `8e090e1..` that commit (N1's libkern and libz
commits, 33785a0..d6cfca6, landed just before N0's close and belong here).

- Went well: one subagent per area (libkern on Sonnet; libz, crypto-hash and crypto-cipher on
  Opus), each in its own worktree with a brief and a handoff note, and an independent Opus
  reviewer per security-sensitive branch. Every output stayed bit-identical: zlib's byte-for-byte
  vectors, the standards' known-answer vectors (FIPS 180-4, RFC 1321, 2104/4231, 7693, FIPS 197,
  SP 800-38D, RFC 8439, 7748, FIPS 81, RFC 2144) and `diff-openbsd` equal on both archs after
  every branch. libz lost its two `unsafe` (fallible allocation through `Vec::try_reserve_exact`
  and `Box<[T; 1]>`). The reviews found an inherited gap and closed it: LZ's cryptosoft "wiped"
  key schedules and HMAC states by storing an enum tag, leaving keys in freed session memory;
  every schedule and keyed context now zeroes itself on drop, as the C's `explicit_bzero` does.
- Failed: the first reviewer's trailer was pasted with its heading ("Security-Review: (trailer
  text) Security-Review: ...") into the seven published crypto-hash commits (8abd30a..417452c);
  left as published. A subagent's rebase left intermediate commits unbuilt; a per-commit
  `cargo check` on host, amd64 and arm64 proved all twelve. The cipher reviewer approved with
  four minor findings (a zero-round AES panic, three contexts without a wiping `Drop`, a stack
  copy of softraid's mask-key schedule, `PartialEq` on a key schedule); fixed in five commits and
  re-reviewed. The worktrees started at `lz-origin`, not `main`: every agent reset to `main`.
- Idioms: three rows in docs/IDIOMS.md: a C status code -> `Result` (libz), a hash/MAC context
  with free functions -> a type with `new`/`update`/`finalize(&mut self)` that wipes in place
  (`finalize(self)` would wipe a copy), and a key schedule wiped by its own `Drop`, no longer
  `Copy`.
- Rules: none changed; the brief for crypto (security-review.md applied in full) is the pattern
  for N8.
- Numbers:
  - `cargo xtask unsafe-report`: lib/libkern 10 -> 10, lib/libz 2 -> 0, crypto 5 -> 5; kernel
    total 10357 -> 10355.
  - `cargo xtask lz status`: 37 redesigned, 26 adapted (call sites only), 965 inherited; 111
    `[[module.fn]]` rows. libkern 5 redesigned, libz 10 (adler32 and crc32 for their tests),
    crypto 22 (spr, sk and podd stay inherited: constant tables).
  - Host tests (`just ci`): 2605 -> 2695 passed, 0 failed, 229 ignored; libz alone 85 -> 123
    (the user found 90 too few: dictionaries, the zlib and raw wrappers, every truncation and
    single-bit flip of eight streams, byte-at-a-time streaming with every flush mode,
    inflateBack, 60480 init parameter combinations, deflateBound, resets, checksums; no bug).
  - `just ci`: 67 of 67 smokes after each branch (1131-1404 s); at the close smoke-uhci failed
    once on arm64 with the inherited EDK2 UhciDxe ASSERT before the kernel, then passed on both
    archs, as did the checks after it; `just ci-full`: green in two parts: 66 of 67 smokes on `-smp 4`; `smoke-softraid` hung 2 h 30 min in
    macOS `_dyld_start` (xtask never reached `main`, QEMU never started, so its own time limit never
    ran), was killed and passed alone on 4 CPUs; then the checks and the installer end to end on
    amd64, arm64 and arm64 ACPI (install 98, 155 and 162 s, each booted; 554 s).
  - `just diff-openbsd`: amd64 and arm64, 102 steps, 99 equal, 3 expected, 0 unexpected.
  - LZ sync: no LZ commit after 44edb2c; `lz drift --security` empty.
  - Commits: 30 (`git log --oneline 8e090e1..` this commit, plus 33785a0..d6cfca6).

Effort: _(user)_

Time: _(user)_

## N2 Foundations

Met 2026-10-16 (under way since 2026-10-10). Boundary: from `lz: bump pin to 3c62ede68802`
(7eee150) to the `docs: close N2` commit.

- LZ sync before N2 (the timing rule: N2 redesigns `queue`/`tree`, used by 222 files, so LZ's
  M16 came first): `f5985f1d055a..3c62ede68802`, 99 LZ commits, every one recorded in
  `lz-sync.toml` (cherry-pick 39, cherry-pick-conflicts 30, reimplemented 1, the rest records of
  merges or not applicable), 122 new inherited modules, `rnd.rs` adapted to the N1 crypto API
  (reviewed). Wall time 01:48-05:28 on 2026-10-10 (3 h 40 min, `git log --format=%ci
  acf8449..956c81b`), four chunks by subagents and one integrator. Closed by the pin bump
  (7eee150), the unsafe re-baseline of the 14 rows the inherited drivers moved (c443f75, the
  user's rule in `unsafe-budget.md`), `just jobs=3 ci` rc=0 in 1643 s (84 of 84 smokes in
  17 min 43 s; host tests 2893 passed, 0 failed, 294 ignored) and `just diff-openbsd` rc=0
  (amd64 and arm64, 102 steps, 99 equal, 3 expected, 0 unexpected) at 154ad1d.
- v1 (2026-10-10 to 2026-10-16): the N2 of "core structures" (the safe `Adapter`, the typed
  slots of `File.f_data` and the knote hook, the `forbid` ratchet inside `bsd`). Replanned on
  2026-10-16 as N2 Foundations (docs/ZERO_UNSAFE.md section 0, decisions 20-24): the kernel is a
  crate with no `unsafe`, so the typed-slot branches were not merged (0.6).
- Went well: four crates where there was one, built on both archs at the first compile but one
  `;` (`just build` rc=0 after one fix; nothing of the runtime, the loader, the module or pipe
  had been compiled before, decision 23). The runtime in `hal` (N2.3), `KernelApi` with the
  ELF loader and `abi-openbsd.ko` (N2.4a), crypto into `kernel` (N2.5) and pipe(2) as the first
  subsystem by the new path (N2.6) arrived as three stacked branches, each reviewed before the
  close, merged into `main` once (decision 31). The reviews found real defects before any build:
  two soundness holes in `abi_pod!` (unions, fields left out), boot(8)'s record not
  range-checked, pipe's `O_NONBLOCK` read once instead of at every check, and the module
  window's one-shot link not enforced. `diff-openbsd`'s new `pipe` set was equal on both
  archs at its first run.
- Failed: the first `just ci` attempt stopped in `smoke-build`: `cd-iso-amd64` refused
  `cd80.iso` because ISO 9660 stores `abi-openbsd.ko` as `ABI_OPENBSD.KO` and libsa's cd9660
  (no Rock Ridge) looks for `ABI-OPENBSD.KO`; xtask's model of that lookup caught it, no smoke
  had booted. 1ae6774d leaves the module off the CD (decision 29's fallback); the module's
  name was decided the same night, `abi_openbsd.ko` for N3's first mechanical slice (decision 38; the user's standing authority of the night) (`abi_openbsd.ko`, a `-` -> `_` mapping in boot(8)'s
  cd9660, or relaxed ISO names). The second attempt booted: 7 of 85 smokes, every amd64 kernel
  panicked on the jump into the module (`panic: trap type 6, code=11`): amd64's `kernel_map`
  directories carry NX as OpenBSD's `pmap_alloc_level` sets them, arm64 has no PXN on its
  tables. A debugger found the cause (its first hypothesis, the direct map, was wrong); the
  user chose a window of the kernel's L4 slot (decision 37), redesigned in 8b2ed4e1, reviewed,
  fixed in 2ff6d854 (the link at most once). The third run was green. `smoke-fb`'s
  "monitor.sock: No such file" in the second run was the same panic, hidden by xtask
  returning before it printed the transcript (`tools/xtask/src/boot.rs` `poll_monitor`, a
  follow-up).
- Idioms that took several attempts: the lists and trees moved into `hal` and back (their
  first `kernel` user brings `hal`'s own); `hal::errno::Errno` interim, then folded into
  `hal::sys::errno` (decision 36); the memory bridges `unsafe trait` then safe with a checked
  trust boundary (decision 30); `KernelApi` typed in `kernel`, then in `hal` so the module
  links no kernel code; decision 13 as `Klist::owned()`.
- Rules corrected: decision 31 (the pipeline's repetitions cut: rules blocks in prompts, host
  checks once by the integrator, one merge into `main` per milestone with `just build` and
  `cargo check -p bsd --tests` after it); the commit hook now sees `PATH=...` prefixes and the
  `cd` target (44e48b6f); `hal-check` counts the sites `unsafe-report` counts (7deb1419: it
  had counted `testutil.rs`, 41 against 35), so `hal_ceiling` fell to 35 (f46a7e48);
  `gen-syscalls` writes `sys/hal/sys/syscall.rs` (db35b0db).
- Numbers:
  - `cargo xtask unsafe-report` (crates): before N2 `kernel` 0, `abi` 0, `hal` 0, `bsd`
    10,604, boot 621; at the close `kernel` 0 (23 files), `abi` 0 (4), `hal` 35 (72, ceiling
    35), `bsd` 10,583 (1,103 files; legacy 8,747 -> 8,726, `forbid` 306 -> 320; kern 1,146 ->
    1,126, sys 328 -> 327, arch/amd64 863 -> 862 -> 863 with the user's raise for decision
    37), boot 621.
  - `cargo xtask lz check`: 1,156 modules (1,058 inherited, 40 adapted, 58 redesigned), 157
    fn rows, 60 extras; 12 bridges open (N3 2, N4 7, N7 1, N9 2).
  - Host tests (`just test`): 3,010 passed at main f5101f5b -> 3,229 passed, 0 failed, 301
    ignored at the close.
  - `just jobs=3 ci` at 5716e767: rc=0 in 1734 s; 85 of 85 smokes in 22 min 36 s, one EDK2
    UhciDxe retry (smoke-uhci, the known firmware flake). After the review fix 2ff6d854 (merged
    at 17fb1cb1): `just build` rc=0 and `smoke-boot`, `smoke-abi`, `smoke-efiboot`, `smoke-fb`
    rc=0 on both archs (`abi-openbsd.ko: 331 syscalls, 2 native`, `abi-ok-42`, `abi-pipe-43`);
    no second full `ci` (the full run covered the tree one commit before).
  - `just diff-openbsd`: rc=0 in 79 s; amd64 and arm64, 107 steps, 104 equal, 3 expected, 0
    unexpected (the `pipe` set new).
  - `cargo xtask lz drift --security`: empty; no LZ commit after the pin.
  - Commits: 196 after the pin bump (176 without merges; `git rev-list --count
    [--no-merges] 7eee150..` this commit's parent), 140 of them on 2026-10-16.

Effort: _(user)_

Time: _(user)_

## N3 Memory

Under way (from the `docs: close N2` commit; this section is completed at the close).

- Performance baseline before any uvm slice is merged (`just bench`, docs/ARCHITECTURE.md
  "bench"; the brief of 2026-10-17). EmiBSD is the smokes' MP debug kernel (`opt-level = 1`,
  kept so by the user's decision of 2026-10-17, the caveat documented) with the ramdisk and
  `abi-openbsd.ko`; OpenBSD the `diff-openbsd` snapshot; both `-smp 4`, `-m 512M`, TCG, one
  after the other, the order alternating. Ten runs per side, each boot running a discarded
  warm-up pass before the recorded one; per measurement and side the worst sample is left
  out, and the median, minimum and spread `(max - min) / median` are those of the nine kept;
  the ratio is EmiBSD / OpenBSD and is refused (`spread`) when either spread exceeds 15 %.
  The reports, all ten samples and the discarded one's index included, are kept as
  `docs/bench/n3-before-<arch>.toml` for `bench compare`, which refuses a report of another
  method.
- Superseded baselines, each run on a kernel whose `sys/` equals `main` 20035fd0:
  - a49170f2 (22:01-22:16 UTC, five runs, one pass per boot, `forkexit` 500 ops, `forkexec`
    200, a 64 MiB file): six of fourteen ratios refused, most of the spread from the first
    sample after boot (amd64's first tcpbench run, 483.8 Mbps against 629.6 to 716.9;
    arm64's first `forkexit`, 2.47 ms against 1.70 to 2.03). The user chose a warm-up pass
    per boot, a 10 s tcpbench warm-up and more work per sample (`forkexit` 2,000 ops,
    `forkexec` 800, a 256 MiB file): b7e474e3.
  - 6c971a45 (22:27-22:50 UTC, five runs): arm64 seven of seven, amd64 one of seven, EmiBSD's
    first run (booted the moment `just userland` ended) 1.5 to 3.5 times slow in every
    measurement. 580cd038 (23:10-23:19 UTC, amd64 again alone, a quiet minute first): one
    of seven again, this time OpenBSD's second run the outlier (`forkexec` 6.07 ms against
    1.85 to 2.07). With five runs, one disturbed boot sets the max-min spread on either
    side; the user chose ten runs a side with the worst sample of each left out (fc622fed).
- The baseline in force: 2026-10-18 00:03-00:44 UTC (2,446 s of benchmark, 2,545 s with the
  build and the quiet minute), on `n3/bench` fc622fed, whose `sys/` is identical to `main`
  20035fd0 (`git diff 20035fd0 fc622fed -- sys` is empty; the kernel's SHA-256 changes from
  one commit to the next only because `sys/build.rs` stamps the last commit's time), under
  the machine lock with nothing else running and a quiet minute (no cargo, rustc or qemu in
  `ps`) before the first boot.

  amd64 (kernel SHA-256 e390f636...):

  | measurement | unit | EmiBSD median (min, spread) | OpenBSD median (min, spread) | ratio |
  |---|---|---|---|---|
  | boot | s | 17.85 (17.24, 7.1 %) | 22.12 (21.60, 3.7 %) | 0.807 |
  | pagefault | ns/op | 8306 (7982, 12.3 %) | 11014 (10175, 14.0 %) | 0.754 |
  | forkexit | ns/op | 455086 (443882, 6.9 %) | 781726 (751827, 6.1 %) | 0.582 |
  | forkexec | ns/op | 1249042 (1207337, 5.2 %) | 1903763 (1808216, 7.3 %) | 0.656 |
  | pipe | ns/op | 60844 (60342, 16.0 %) | 68119 (67300, 3.2 %) | spread |
  | fileread | ns/op | 231284 (226322, 26.0 %) | 290898 (283994, 5.9 %) | spread |
  | tcpbench | Mbps | 793.9 (773.3, 3.6 %) | 581.8 (550.7, 8.2 %) | 1.365 |

  Discarded (run, value): boot EmiBSD 1 (20.44), OpenBSD 4 (22.50); pagefault 9 (9072),
  10 (11869); forkexit 2 (477091), 10 (848794); forkexec 6 (1280964), 10 (2327631); pipe 1
  (70517), 8 (71376); fileread 2 (288854), 8 (340651); tcpbench 7 (771.4), 8 (548.9).

  arm64 (kernel SHA-256 48c08004...):

  | measurement | unit | EmiBSD median (min, spread) | OpenBSD median (min, spread) | ratio |
  |---|---|---|---|---|
  | boot | s | 8.33 (8.28, 2.9 %) | 31.67 (30.99, 3.5 %) | 0.263 |
  | pagefault | ns/op | 75784 (74836, 2.7 %) | 122733 (121450, 2.5 %) | 0.617 |
  | forkexit | ns/op | 1686982 (1640915, 4.1 %) | 2214634 (2102787, 11.3 %) | 0.762 |
  | forkexec | ns/op | 4900752 (4801397, 4.1 %) | 6621370 (6432361, 6.6 %) | 0.740 |
  | pipe | ns/op | 168655 (163968, 9.3 %) | 177063 (175689, 7.0 %) | 0.953 |
  | fileread | ns/op | 702139 (616961, 13.0 %) | 888372 (871559, 4.1 %) | 0.790 |
  | tcpbench | Mbps | 108.3 (102.7, 6.9 %) | 87.7 (87.2, 1.2 %) | 1.235 |

  Discarded (run, value): boot EmiBSD 2 (8.62), OpenBSD 9 (32.16); pagefault 8 (92261), 4
  (124531); forkexit 6 (2077130), 2 (2533415); forkexec 3 (5075677), 2 (6891198); pipe 3
  (182330), 3 (188904); fileread 6 (710295), 1 (1078541); tcpbench 3 (102.1), 10 (86.9).

  Twelve of fourteen ratios print. The two refused are EmiBSD's on amd64, and neither is one
  outlier: `pipe` sits at 69.9 to 70.5 us in runs 1 to 3 and at 60.3 to 61.2 in runs 4 to
  10, `fileread` at 263 to 289 us in runs 1 to 3 and at 226 to 238 in runs 4 to 10, two
  levels in time order (the amd64 run of 580cd038 showed the same split, three slow runs
  then fast ones), while OpenBSD's boots, alternating with them, show none. The limit was not
  tuned and the method not changed (the user's decision); the cause is open. The N3 close
  runs the same command and carries `Bench:` lines from `cargo xtask bench compare` against
  these reports, for what prints.
