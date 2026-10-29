# Journal

One section per milestone of `docs/ROADMAP.md`, N0 onwards: what went well, what failed, the
idioms that took more than one attempt, the rules that had to be corrected, and reproducible
numbers. Only what the history shows; `Effort` and `Time` are left for the user.

## Origin

This repository is LandingZone's (then EmiBSD.LZ's) history up to `lz-origin` (`f5985f1d055a`, `docs: M16b met`,
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

Met 2026-10-18 (under way since 2026-10-16). Boundary: from `docs: close N2` (7775163b) to
the `docs: close N3` commit.

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
  Each run also records the host's one-minute load average at its start and end and its five
  busiest processes but QEMU (cd47fb56, `[[host]]` in the report), a diagnostic that changes
  no sample.
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
  - d55bd02a's amd64 run (2026-10-18 00:03-00:18 UTC, ten runs): five of seven, EmiBSD's
    `pipe` (16.0 %) and `fileread` (26.0 %) refused, slow in runs 1 to 3 and fast after. A
    debugger traced that step to host CPU contention, not the kernel, the files or the run
    index: VS Code's cpptools in bursts of 350 to 430 % and macOS's idle-time daemons
    (photoanalysisd, mediaanalysisd and others, about a core each); three `yes` hogs on the
    host reproduce `pipe` at about 70k ns, its slow level. Its clean diagnostic run C10
    (01:17-01:28 UTC, same tool and method) was made without tcpbench, so it is not a
    baseline. The user had amd64 run again in full with the host diagnostic on (dd0f5a75).
- The baseline in force, arm64: the fc622fed run (2026-10-18 00:18-00:44 UTC; the whole run
  of both archs 2,446 s), whose `sys/` is identical to `main` 20035fd0 (`git diff 20035fd0
  fc622fed -- sys` is empty; the kernel's SHA-256 changes from one commit to the next only
  because `sys/build.rs` stamps the last commit's time), under the machine lock with nothing
  else running and a quiet minute (no cargo, rustc or qemu in `ps`) before the first boot.

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

- The baseline in force, amd64: the dd0f5a75 run (2026-10-18 01:45-02:01 UTC, 935 s of
  benchmark, 1,016 s with the build and the quiet minute), `sys/` identical to 20035fd0,
  under the machine lock, a quiet minute before the first boot, the host diagnostic on.

  amd64 (kernel SHA-256 264087ac...):

  | measurement | unit | EmiBSD median (min, spread) | OpenBSD median (min, spread) | ratio |
  |---|---|---|---|---|
  | boot | s | 17.35 (17.13, 2.4 %) | 21.64 (21.49, 1.5 %) | 0.802 |
  | pagefault | ns/op | 7956 (7793, 13.8 %) | 10646 (10267, 8.8 %) | 0.747 |
  | forkexit | ns/op | 459233 (432332, 7.4 %) | 786786 (768463, 3.7 %) | 0.584 |
  | forkexec | ns/op | 1214616 (1186903, 6.5 %) | 1842307 (1770422, 8.6 %) | 0.659 |
  | pipe | ns/op | 68504 (59989, 14.4 %) | 73197 (67555, 11.3 %) | 0.936 |
  | fileread | ns/op | 280928 (224734, 24.8 %) | 329772 (276228, 19.7 %) | spread |
  | tcpbench | Mbps | 767.3 (761.3, 1.2 %) | 590.4 (531.7, 14.0 %) | 1.300 |

  Discarded (run, value): boot EmiBSD 1 (17.72), OpenBSD 1 (22.59); pagefault 9 (9087), 5
  (11664); forkexit 9 (478218), 4 (817915); forkexec 1 (1387212), 7 (1971466); pipe 4
  (70121), 7 (85702); fileread 1 (317045), 7 (342389); tcpbench 4 (728.2), 6 (517.0).

  Thirteen of fourteen ratios print. The refused one is amd64's `fileread`, over the limit
  on both sides. EmiBSD's samples sit at 277 to 317 us but for runs 2 and 5 (224.7 and
  234.1); OpenBSD's at 276 to 317 in runs 1 to 4 and 330 to 342 in runs 5 to 10. The host
  rows do not line up with either: cpptools ran at 64 to 100 % from OpenBSD's run 1 to
  EmiBSD's run 5 (load 2.97 to 4.14), the quiet runs after it show 5 to 30 % for the busiest
  process (load 2.16 to 3.09), and EmiBSD's two fast runs both fall inside the busy part,
  OpenBSD's fast runs too. As recorded:

  | run | EmiBSD fileread | host at its end | OpenBSD fileread | host at its end |
  |---|---|---|---|---|
  | 1 | 317045 (discarded) | load 2.53 -> 3.50; WindowServer 14 %, Code Helper 14 % | 276228 | load 3.50 -> 3.34; cpptools 64 % |
  | 2 | 224734 | load 2.97 -> 3.46; cpptools 91 % | 285099 | load 3.34 -> 2.97; cpptools 82 % |
  | 3 | 280702 | load 3.46 -> 4.14; cpptools 95 % | 289801 | load 4.14 -> 3.76; cpptools 84 % |
  | 4 | 277067 | load 3.36 -> 3.54; cpptools 100 % | 317010 | load 3.76 -> 3.36; cpptools 86 % |
  | 5 | 234109 | load 3.54 -> 3.64; cpptools 85 % | 333894 | load 3.64 -> 2.91; Creative Cloud 5 % |
  | 6 | 283726 | load 2.61 -> 2.73; Creative Cloud 6 % | 329944 | load 2.91 -> 2.61; Creative Cloud 6 % |
  | 7 | 294477 | load 2.73 -> 2.68; WindowServer 28 % | 342389 (discarded) | load 2.68 -> 3.09; WindowServer 17 %, firefox 17 % |
  | 8 | 288055 | load 2.61 -> 2.72; socketfilterfw 30 % | 341235 | load 3.09 -> 2.61; Creative Cloud 6 % |
  | 9 | 283475 | load 2.72 -> 2.54; WindowServer 9 % | 329772 | load 2.54 -> 2.16; Creative Cloud 6 % |
  | 10 | 280928 | load 2.43 -> 2.74; cfprefsd 10 % | 338651 | load 2.16 -> 2.43; socketfilterfw 8 % |

  The limit was not tuned and the method not changed (the user's decision); the cause of
  `fileread`'s spread on amd64 is open. The N3 close runs the same command and carries
  `Bench:` lines from `cargo xtask bench compare` against these reports, for what prints.
- The close (2026-10-18): N3's branch (925cce59, 152 commits from main 6da2ef08, 141 without
  merges; `git rev-list --count [--no-merges] 6da2ef08..925cce59`) merged into the candidate
  at d612f9c5 with no conflict, n3/bench (60700aff) at 29d66500 with one (`OWN_PROGRAMS` in
  tools/xtask/src/userland.rs: mmapstress and bench, both kept). Wall time of the milestone:
  2026-10-16 23:01 (621a179d) to 2026-10-18 09:49 (925cce59), about 35 h, up to four agents
  at once. Six slices: N3.0 the module's name; N3.1 `hal::Pmap` and the two pmaps; N3.2
  `kernel::vm::km`; N3.3 the page system; N3.4 the owners and the fault; N3.5 the map and the
  rest, the flip that deleted `sys/uvm/`; decision 40 between N3.3 and N3.4.
- Went well: every slice reviewed by an agent that did not write it before it was merged into
  the next (25 Security-Review paragraphs, carried by d612f9c5's body), and the reviews found
  what no build would have: the arm64 backend's PTE cleared before its pv lock, a pted 8 bytes
  short of the C's (visible in `kern.pool`), the static-entry arena refilled through
  `kernel_map` (a deadlock while cold), an unprivileged panic on unwiring an entry without an
  amap (K1, reachable through `munlock(2)`), a use-after-free window in an owned tree's
  removal, `with_vmspace_handle` reachable from safe code, and four slips in OpenBSD's own C
  (`uvm_fault.c:1468-1485`, confirmed; `uvm_aobj.c:428-432`; `uvm_pager.c:292` with
  `uvm_page.c:1238`; `uvm_map.c:2214-2221`, confirmed; for the user, never upstream). The
  close's `just check` was green at its first run: the compile-and-fix pass found nothing,
  since every flip had booted its own smokes (a `security-review.md` area). `just ci-full` ran for the first time under v2 (decision 24, for N2 and N3) and was green at its first run, the installer end to end on both archs included.
- Failed: the first integration checkpoint (54f48bd4, N3.1 and N3.2 only) booted 0 of 85
  smokes: amd64 under Limine underflowed `uvm_map_setup` (4b's clamp of the kernel window, the
  HHDM reaching slot 258) and both machines panicked allocating with Box before `kmeminit`;
  fixed in 74eabc4d, 6e996212 and 799ca7ad, then 85 of 85. N3.4b stopped before its flip: the
  owners could not move before the fault held their witnesses, so the planner re-planned
  (N3.4c-e, N3.6 folded in); N3.5 S1-R1 stopped on `hal` pieces the kernel could not reach
  (`arc4random`, `sched_pause`), added in 7f2712c6. Two worktrees were created inside the LZ
  clone once (the session's cwd), written nothing; the rule is never to `cd` into
  `reference/`. The close's first `just ci` stopped in `test` on the inherited
  `dev::ic::pckbc` flake (below).
- Idioms that took several attempts: `MappedBytes`, from a bare `Paddr` (the review: safe code
  could alias `&mut`) to a claimed, busy, token-only view (three reviews); the witness-carrying
  fault (`RwWitness`, a witness consumed by a sleep, a `&'v mut Option<T>` slot to return a
  witness borrowing a value); `Guarded::get_mut` needing witness parts (`Divisible::split`) for
  two cells of one lock; owning intrusive containers that find and remove by key and re-read
  the secondary count after adapter calls; the entry's handle cells needing their own
  `AtomicBool` window under a read lock (smoke-mp found it); `NSTATIC_POOLS` 4 -> 8 -> 16 as
  more pools moved before `kmeminit`.
- Rules corrected: decision 40 (one MP kernel); decision 41 recorded as studied; decision 30
  extended to `MappedBytes`; the redesigner's memory trimmed under its read limit (a043de71);
  the bench recipe spells its features as the smokes do since decision 40 (818ed2f5).
- Numbers:
  - `cargo xtask unsafe-report` (crates), main 20035fd0 -> the close: `kernel` 0 (23 -> 48
    files), `abi` 0 (4), `hal` 35 -> 100 (72 -> 108 files; `hal_ceiling` 110, the user's
    raise a2ec4145), `bsd` 10,583 -> 10,196 (1,103 -> 1,069 files; legacy 8,726 -> 8,501,
    core 2,513 -> 2,416, `forbid` 320 -> 348 of 1,218 modules; rows arch/amd64 863 -> 773,
    arch/arm64 943 -> 881, arch/host 38 -> 35, machine 85 -> 79, kern 1,126 -> 1,117, dev/pv
    249 -> 246, sys 327 -> 328 with the user's raise 6da2ef08, uvm 215 -> gone), boot 621.
  - `cargo xtask lz check`: 1,156 -> 1,170 modules (1,058 -> 937 inherited, 40 -> 123 adapted,
    58 -> 110 redesigned), 157 -> 594 fn rows, 60 -> 68 extras; bridges 12 -> 21 open (N4 13,
    N5 2, N6 1, N7 2, N9 3).
  - Host tests (`just check`): 3,229 -> 3,775 passed, 0 failed (bsd 2,396, kernel 308, hal
    292, abi 9, xtask 260, libz 123, libkern 26, the boot loaders' 57).
  - `just jobs=3 ci`: the first run (10:17) stopped in `test` on the inherited pckbc flake
    (below); the second (10:20-10:43, 1,396 s) passed 86 of 86 smokes in 18 min 32 s, no retry,
    and stopped at `check-drift`: the applied record of LZ 602141098d1d named
    `sys/uvm/uvm_init.rs`, deleted by N3.5's flip (614d65af points it at `sys/kern/vm_bsd.rs`,
    as 27432aa6 did for the pmap at the checkpoint); the steps after it rc=0.
  - `just jobs=3 ci-full` at 614d65af (decision 24, N2 and N3): rc=0 in 2,248 s, its `ci` on
    four CPUs (86 of 86 smokes in 21 min 18 s, no retry) and the eight installer recipes
    (`smoke-install-amd64` 106 s, `-arm64` 164 s, `-arm64-acpi` 165 s, the four
    `smoke-install-boot-*`, `smoke-install80-amd64` 75 s, `-arm64` 123 s). `just comp` first
    (425 s, seeded from the main checkout).
  - `just diff-openbsd`: rc=0 in 77 s; amd64 and arm64, 107 steps, 104 equal, 3 expected, 0
    unexpected (`expected.toml` unchanged).
  - `just bench` at 614d65af (11:37-12:20, 2,573 s with the build and a quiet minute; the same
    command and method as the baselines, under the machine lock, nothing else running): fourteen
    of fourteen ratios print. The reports are kept as `docs/bench/n3-after-<arch>.toml`.
    `cargo xtask bench compare docs/bench/n3-before-<arch>.toml target/bench/<arch>/report.toml`
    gives the `Bench:` lines of the closing commit (EmiBSD medians; a row prints when it moved
    by more than the larger spread):

    | measurement | unit | amd64 before -> after | ratio to OpenBSD | arm64 before -> after | ratio to OpenBSD |
    |---|---|---|---|---|---|
    | boot | s | 17.35 -> 17.22 (within spread) | 0.802 -> 0.804 | 8.33 -> 8.65 | 0.263 -> 0.274 |
    | pagefault | ns/op | 7956 -> 12269 | 0.747 -> 1.184 | 75784 -> 81715 | 0.617 -> 0.661 |
    | forkexit | ns/op | 459233 -> 680755 | 0.584 -> 0.884 | 1686982 -> 2672794 | 0.762 -> 1.200 |
    | forkexec | ns/op | 1214616 -> 1820719 | 0.659 -> 1.034 | 4900752 -> 6868461 | 0.740 -> 1.075 |
    | pipe | ns/op | 68504 -> 69186 (within spread) | 0.936 -> 0.942 | 168655 -> 171029 (within spread) | 0.953 -> 0.922 |
    | fileread | ns/op | 280928 -> 322744 (baseline spread over 15 %) | spread -> 0.973 | 702139 -> 879892 | 0.790 -> 1.001 |
    | tcpbench | Mbps | 767.3 -> 787.6 | 1.300 -> 1.313 | 108.3 -> 102.1 (within spread) | 1.235 -> 1.143 |

    N3 made the page fault and fork slower on both machines: amd64 `pagefault` +54 %,
    `forkexit` +48 %, `forkexec` +50 %; arm64 `forkexit` +58 %, `forkexec` +40 %, `pagefault`
    +8 %, `fileread` +25 % (OpenBSD's own runs moved by under 6 %). On the debug kernel
    (`opt-level = 1`) the kernel's witnesses, counted handles and checked slices run unoptimised;
    where the time goes is not measured yet, and is the first question for N4's bench.
  - `cargo xtask lz drift --security`: empty; no LZ commit after the pin.
  - The flake of the close: `dev::ic::pckbc::tests::an_async_command_is_acked_byte_by_byte_then_freed`
    panicked at `sys/kern/kern_timeout.rs:291` (a null `prev` in `circq_insert_tail`) in the
    first `just ci`, as once at N3.4e. By reading: pckbc's test `controller()` takes
    `kern_timeout::tests::LOCK` but never calls `timeout_startup()`, so it depends on an earlier
    test (hidkbd's, kern_timeout's) having initialised the wheel; both files are inherited,
    byte-identical to LZ, so the fix is the user's decision. The second run passed.

Effort: _(user)_

Time: _(user)_

### Decision 42: N3's performance

Investigated 2026-10-18 after the close, before N4 (docs/ZERO_UNSAFE.md decisions 42 and 43).
Boundary: main e3cdf1d3 to the `docs:` commit of n3/perf-close. Three branches, 12 commits
without merges (`git rev-list --count --no-merges e3cdf1d3..n3/perf-release` 2,
`..n3/perf-opt` 10), merged at 8accf040 and 11eb98d3 with no conflict (disjoint files), and
the review's fix c51cb2c2; `git diff --shortstat e3cdf1d3 n3/perf-opt -- sys`: 7 files, +1,939
-418 (sys/kernel/benches/vm.rs 1,021 of them).

- The release diagnostic (eeb155ed, `just bench-release`; the method of the baselines: ten
  runs a side, the worst left out, the 15 % limit). The MP kernel in the release profile at
  20035fd0 (before N3, built at ef0888b7 with the flag picked onto it) and at eeb155ed (after),
  2026-10-18 14:06-16:13 under the machine lock. Cell: EmiBSD median (spread), ratio to
  OpenBSD; `spread` = refused.

  | measurement | arch | debug before | debug after | release before | release after | release/debug before | after |
  |---|---|---|---|---|---|---|---|
  | boot (s) | amd64 | 17.35 (2.4 %) 0.802 | 17.22 (3.1 %) 0.804 | 16.81 (9.1 %) 0.772 | 16.79 (1.7 %) 0.780 | 0.969 | 0.975 |
  | pagefault (ns) | amd64 | 7956 (13.8 %) 0.747 | 12269 (4.0 %) 1.184 | 6313 (5.0 %) 0.595 | 8550 (4.6 %) 0.821 | 0.793 | 0.697 |
  | forkexit (ns) | amd64 | 459233 (7.4 %) 0.584 | 680755 (3.4 %) 0.884 | 394051 (6.4 %) 0.490 | 554141 (2.4 %) 0.700 | 0.858 | 0.814 |
  | forkexec (ns) | amd64 | 1214616 (6.5 %) 0.659 | 1820719 (4.5 %) 1.034 | 999901 (7.7 %) 0.538 | 1431855 (4.3 %) 0.793 | 0.823 | 0.786 |
  | pipe (ns) | amd64 | 68504 (14.4 %) 0.936 | 69186 (3.6 %) 0.942 | 53004 (3.0 %) 0.707 | 52262 (9.8 %) 0.709 | 0.774 | 0.755 |
  | fileread (ns) | amd64 | 280928 (24.8 %) spread | 322744 (2.8 %) 0.973 | 275107 (3.4 %) 0.802 | 273054 (21.6 %) spread | 0.979 | 0.846 |
  | tcpbench (Mbps) | amd64 | 767.3 (1.2 %) 1.300 | 787.6 (1.6 %) 1.313 | 1085.7 (12.5 %) 1.946 | 1191.4 (1.2 %) 2.004 | 1.415 | 1.513 |
  | boot (s) | arm64 | 8.33 (2.9 %) 0.263 | 8.65 (1.5 %) 0.274 | 8.31 (2.1 %) 0.263 | 8.47 (3.3 %) 0.269 | 0.998 | 0.980 |
  | pagefault (ns) | arm64 | 75784 (2.7 %) 0.617 | 81715 (2.3 %) 0.661 | 72791 (1.7 %) 0.589 | 78044 (7.4 %) 0.633 | 0.961 | 0.955 |
  | forkexit (ns) | arm64 | 1686982 (4.1 %) 0.762 | 2672794 (2.5 %) 1.200 | 1572308 (1.7 %) 0.711 | 2225552 (3.5 %) 0.998 | 0.932 | 0.833 |
  | forkexec (ns) | arm64 | 4900752 (4.1 %) 0.740 | 6868461 (2.8 %) 1.075 | 4436730 (3.8 %) 0.695 | 5918638 (3.9 %) 0.925 | 0.905 | 0.862 |
  | pipe (ns) | arm64 | 168655 (9.3 %) 0.953 | 171029 (2.2 %) 0.922 | 161038 (4.6 %) 0.866 | 152366 (3.2 %) 0.821 | 0.955 | 0.891 |
  | fileread (ns) | arm64 | 702139 (13.0 %) 0.790 | 879892 (2.1 %) 1.001 | 595441 (1.8 %) 0.678 | 813719 (2.3 %) 0.925 | 0.848 | 0.925 |
  | tcpbench (Mbps) | arm64 | 108.3 (6.9 %) 1.235 | 102.1 (0.5 %) 1.143 | 105.5 (0.5 %) 1.189 | 107.2 (0.6 %) 1.226 | 0.974 | 1.050 |

  The release kernel is slower after N3 in the same places as the debug one (pagefault
  amd64 +35 %, forkexit +41 % / +42 %, forkexec +43 % / +33 %, fileread arm64 +37 %), so the
  cost was the design, not `opt-level = 1`. Kept as `docs/bench/n3-release-{before,after}-
  <arch>.toml`. Two runs were refused and run again alone: release-after amd64's first
  (`forkexec` OpenBSD spread 19.0 %, host daemons' bursts in runs 2-4; the re-run's
  `fileread` EmiBSD 21.6 %, one fast sample, reported as refused and not run a third time)
  and release-before arm64's first (`fileread` EmiBSD 17.6 %; the re-run 1.8 %).
- Where the time went (737b7b5d, `sys/kernel/benches/vm.rs`, docs/ARCHITECTURE.md "bench"):
  the real `kernel::vm` code on the host, debug and release, profiled with sample(1) and
  xctrace with the inlined frames expanded. Against LZ's legacy uvm at 20035fd0 (the same
  ops through a scratch copy, never a worktree) the fault was +29 % (dev) / +23 % (release)
  per page, a 70-entry fork +134 % / +168 %, its teardown +191 % / +163 %, a one-page
  map/unmap +121 % / +123 %. Share of each op without the host pmap and the doubles (dev /
  release), the pieces N3 added:
  - fork, exit, exec, map/unmap: entry looked up again by key (`ent`, `next_key`; 12-15
    tree searches per entry forked where the C follows pointers) 32-47 %; the owned tree's
    per-node identity and bounds checks inside each search 10-24 %; `MapState::owners()`
    rebuilt by every free-list call 8-17 %; `Guarded` checks 1.5-4.4 %; handle clones
    2.5-3 %.
  - the anonymous fault: the page system (queue mutexes, lists, the per-CPU cache)
    37-40 %, the same share as LZ's; the fault's own logic 24-30 %; `PoolArc` clones 6-7 %;
    `Guarded` 2-4 %, `PG_HAL_VIEW` 3 %, the owned-tree checks 4-6 %.
  - Not seen by the host: the pmaps (the host pmap is a `BTreeMap`, 45-58 % of a host fault),
    TLB shootdowns, the trap and syscall entry, TCG's costs. Busy-window relocks: 0 samples.
- Phase 2 (n3/perf-opt, the redesigner; the user: no approval of the table needed), one
  commit per piece, each with its host `Bench:` lines (two runs per profile, the lowest
  median, against the previous commit; fork.big dev / release):
  1. ac8b9ebf hold the map entry in hand instead of finding it again by key: 11834 -> 8804 /
     10165 -> 6971 ns (forkexit 24374 -> 18064 / 20822 -> 14276, map.syscall 721 -> 556 /
     600 -> 480).
  2. dd9dc059 compute a map operation's owners once and lend them: 8804 -> 7214 / 6971 ->
     6095 (keeping them in `MapState` was measured and refused: `Vmspace` 616 bytes, a
     `vmsppl` item size `vmstat -m` shows).
  3. 9986eaac view each node of an owned tree's walk once: 7214 -> 7374 / 6095 -> 5688.
  4. 6eb820e2 lend map entries' handles where the write lock keeps them: 7374 -> 6956 /
     5688 -> 5574.
  5. 9dab32dd inline `Guarded`'s check and give the fault a typed witness: 6956 -> 6586 /
     5574 -> 5458.
  6. ee5bbc7b lend the element an owned tree's insert linked (hal API, no user yet; hal tests
     292 -> 293).
  7. f5caa6de go on from the entry an insert lends: 6586 -> 6225 / 5458 -> 5230.
  Base 02f20ecb -> tip 98611cd0, dev / release: fork.big -47 % / -49 %, forkexit -43 % /
  -44 %, forkexec -36 % / -38 %, exec -26 % / -29 %, map.syscall -30 % / -28 %, cmp.fork70
  -49 % / -50 %, the faults -1 to -5 %; kernel tests 308 -> 310, `hal` 100. Refused by the
  user: `hal::owned`'s identity check once per search instead of per node (fork.big -8 % /
  -6 %); it stays per node.
- The review (an agent that wrote none of it) approved everything but one defect: ac8b9ebf
  dropped `UVM_MAP_CLIP_END`'s KASSERT from `uvm_unmap_remove` when the entry lies inside the
  range and `markfree` is false (reference/openbsd-src/sys/uvm/uvm_map.h:111, uvm_map.c:1987-
  1988), a tautology today but a check the C makes, and its body said "no check removed".
  c51cb2c2 restores it under the C's condition (`diagnostic` only, as KASSERT; the host
  bench unchanged, -7 % to +0.2 %).
- The close of decision 42 (n3/perf-close): `just check` rc=0 (344 s; host tests 3,775 ->
  3,780), `hal-check` 100 <= 100, `abi-check` clean, `cargo test -p xtask` 261,
  `cargo check -p bsd --tests` clean; `just jobs=3 ci` rc=0 in 1,330 s (18:34-18:56), 86 of
  86 smokes in 18 min 15 s, no retry, `lz drift --strict` 0 open; `just diff-openbsd` rc=0 in
  76 s, 107 steps on each arch, 104 equal, 3 expected, 0 unexpected (`expected.toml`
  unchanged). No `ci-full`: not an N close (it runs at N4's).
- The final debug `just bench` (the smokes' kernel, the same method), against the N3 close's
  reports (`cargo xtask bench compare docs/bench/n3-after-<arch>.toml <report>`) and the
  baseline before N3 (`n3-before-<arch>.toml`); kept as `docs/bench/n3-perf-after-<arch>.toml`:

  2026-10-18 18:59-19:43 (2,643 s with the build and a quiet minute), kernel c51cb2c2, under
  the machine lock, nothing of ours else running; the host diagnostic shows macOS's suggestd at
  100 % of a core through the whole run (not ours, not touched). EmiBSD medians; ratio to
  OpenBSD in brackets; the change against the N3 close and against the baseline before N3.

  | measurement | arch | before N3 | N3 close | now | now vs close | now vs before |
  |---|---|---|---|---|---|---|
  | boot (s) | amd64 | 17.35 [0.802] | 17.22 [0.804] | 17.31 [0.810] | within spread | within spread |
  | pagefault (ns) | amd64 | 7956 [0.747] | 12269 [1.184] | 11855 [1.157] | -3.4 %, within spread | +49 % |
  | forkexit (ns) | amd64 | 459233 [0.584] | 680755 [0.884] | 597826 [0.770] | -12.2 % | +30 % |
  | forkexec (ns) | amd64 | 1214616 [0.659] | 1820719 [1.034] | 1646020 [0.934] | -9.6 % | +36 % |
  | pipe (ns) | amd64 | 68504 [0.936] | 69186 [0.942] | 69415 [0.948] | within spread | within spread |
  | fileread (ns) | amd64 | 280928 [spread] | 322744 [0.973] | 318409 [0.959] | within spread | baseline spread over 15 % |
  | tcpbench (Mbps) | amd64 | 767.3 [1.300] | 787.6 [1.313] | 781.6 [1.295] | within spread | within spread |
  | boot (s) | arm64 | 8.33 [0.263] | 8.65 [0.274] | 8.61 [0.273] | within spread | +3.4 % |
  | pagefault (ns) | arm64 | 75784 [0.617] | 81715 [0.661] | 81425 [0.657] | within spread | +7.4 % |
  | forkexit (ns) | arm64 | 1686982 [0.762] | 2672794 [1.200] | 2706914 [1.225] | +1.3 %, within spread | +60 % |
  | forkexec (ns) | arm64 | 4900752 [0.740] | 6868461 [1.075] | 6904563 [1.077] | +0.5 %, within spread | +41 % |
  | pipe (ns) | arm64 | 168655 [0.953] | 171029 [0.922] | 171062 [0.918] | within spread | within spread |
  | fileread (ns) | arm64 | 702139 [0.790] | 879892 [1.001] | 867319 [0.983] | within spread | +24 % |
  | tcpbench (Mbps) | arm64 | 108.3 [1.235] | 102.1 [1.143] | 103.8 [spread] | refused (15.5 %) | refused |

  Thirteen of fourteen ratios print. arm64's `tcpbench` is refused: EmiBSD's spread 15.5 %
  (OpenBSD's 14.7 %; EmiBSD's discarded run 3 at 58.2 Mbps, the nine kept 98.5 to 114.5,
  run 1 the fast one). The arm64 half was started again alone at 19:45 for it and stopped by
  the main session at 19:54, in its fourth run, to free the machine for the QEMU profiling
  the user asked for; the first run's report is the one kept. `Bench:` lines (`cargo xtask bench compare`), against the N3
  close: `forkexit-amd64 680755 -> 597826 ns`, `forkexec-amd64 1820719 -> 1646020 ns`; every
  other row within the larger spread or not comparable. So on amd64 the phase recovered 37 %
  of `forkexit`'s N3 regression and 29 % of `forkexec`'s, none of the page fault's; on arm64
  nothing moved. The host's 36-49 % on fork and exec is the `kernel::vm` share only: what N3
  costs under QEMU is mostly elsewhere, in what the host doubles cannot see (the pmaps moved
  into `hal` at N3.1, arm64's above all; the page system's locks under TCG; the trap and the
  TLB shootdowns), and is the next question.
- Left: the lookups beyond the C the review listed in sys/kernel/vm/map.rs (unmap: one search
  per step and one after a clip; `uvm_mapent_mkfree` up to three and `uvm_map_fix_space`'s by
  key; `uvm_map_mkentry`'s first, `fix_space`'s from it and the final refind, ~8 % of a
  release fork; the clips' and `uvm_map_splitentry`'s refinds; the owners recomputed in
  `splitentry`, `clip_start` and the merge; the `next_key` walks of `pageable_pgon`,
  `pageable_wire`, `clip_first`, `inherit`, `immutable`, `advice`, `extract`, `refill`), a
  cursor over the state or a reshaped `mkentry` being the next step; the fault's aref/object
  clones stay (they cross the sleeps with the map unlocked); the pmap side of the fault,
  which only a QEMU profile or a kernel-side counter can see (the user's); `zero_page`/
  `copy_page` taking the `PageMd` waits for typed pages.
- `sys/kernel/benches/vm.rs` uses `std` (a host bench target, `harness = false`, never linked
  into a kernel image), which CLAUDE.md's `std` exceptions did not name; the review asked for
  the user's ruling, and the main session relayed on 2026-10-18 that the user allows
  `sys/*/benches/`. The exception's wording in CLAUDE.md and `.claude/rules/rust-kernel.md`
  is the main session's commit.

### /insight: external opinions, the first smoke test (2026-10-19)

- The user's decision of 2026-10-19: the session itself may ask external models for an opinion
  when blocked, as in the performance case, and judge point by point what applies. Built as the
  skill `.claude/skills/insight/` (`scripts/insight.py`, stdlib only, OpenRouter with the
  user's key from the environment; `ledger.tsv` for the spend); pointers in CLAUDE.md, the
  third network use in `.claude/rules/lz-sync.md`, decision 43's stop in `re-engineering.md`
  ("the stop reaches the user with hypotheses already verified"), the planner's design fork in
  `.claude/agents/planner.md`. Three triggers only; it applies nothing.
- The models, chosen by the user by OpenRouter link: `google/gemini-4-argon`,
  `qwen/qwen3.8-max-prime`, `~deepseek/deepseek-pro-latest`. After the test Qwen was dropped
  (reasoning mandatory: HTTP 400 when disabled; twice Gemini's price; two empty answers; the
  user's doubt about its training lineage); the defaults are Gemini 4 Argon and DeepSeek Pro
  with reasoning off; a third family (OpenAI) waits for the user's OK.
- The smoke test: the brief of today's chat feedback on `map.rs`, rewritten to the template
  (question, the decision 42 numbers, the constraints, 12 line-numbered excerpts of `map.rs`,
  the `OwnedTree` API and `uvm_map.c`, the answer format; 48.7 KB, 16.5k-21k input tokens:
  the bytes/4 estimator under-counted, now bytes/3). Three runs: reasoning off, `low`, `high`.
  Off: Gemini 8 claims, 8 correct, 2 the session did not have ($0.08, 71 s); DeepSeek 10/10,
  nothing new ($0.02, 12 s). `low`: Gemini 6/6 with 0 reasoning tokens (lost the `Handles`
  find, gained two implementation hints: `pe`'s free-tree removal hoisted inside the `fp`
  borrow in `uvm_mapent_mkfree`; `uvm_map_fix_space` returning the lent `&VmMapEntry` when
  `min == max`); Qwen 10/10 ($0.15, 61 s). `high`: DeepSeek the same 10 claims ($0.04);
  Gemini and Qwen spent the whole 16k cap on reasoning and returned nothing ($0.40 for
  nothing). Reasoning added no finding; the structured brief is what made the claims
  checkable. The user then replaced DeepSeek and Qwen by GPT Sol (`~openai/gpt-sol-latest`,
  served as `gpt-6.1-sol`): reasoning off, 6 claims, 5 correct and one honest "no claim" (the
  excerpt of `uvm_unmap_remove` was the guard-splitting wrapper, 3853-3875, not the loop in
  `unmap_remove`, 3880: GPT Sol was the only model to say so instead of inferring from the C;
  the template now asks for the inner function), nothing new, $0.06, 40 s. The whole test:
  $0.97.
- Verified against the code and the C, all `correct`: the per-entry refinds (`next_key` =
  find + next, `map.rs:2216`; the `clip_*_at` wrappers and their inner functions, 6407/6422
  and 6444/6470; `splitentry`'s `ent(orig)` at 4576 and the `ent(nkey)` after the insert at
  4645; `clip_start`'s chain, 6470-6493; `mkfree`'s five searches per freed entry, 3750-3770
  plus `FixFrom::Key` at 6761; `mkentry`'s loop and its final `ent(&st.addr, addr)` at 3468
  after `fix_space` had the reference lent). Two the session did not have: `insert_get` has a
  user, `uvm_mapent_addr_insert_parts` (2348), so the ee5bbc7b "no user yet" note is stale;
  `uvm_map_splitentry`'s free-space half clones `Handles` in `uvm_mapent_copy` (2243-2246)
  and replaces them with `Handles::default()` at 4594: a refcount increment and decrement per
  handle on the mmap/fork carve where the C does `memcpy` and `NULL`. No claim was false and
  none a "regression" the C also has (the chat feedback of the same morning had one of each
  and one "not a regression": the template, not the model, made the difference). What would
  need a `hal` API, the user's decision: removal by reference or a cursor (`take_by` searches
  again, `owned.rs:705`), and keeping a reference across a mutation.
- Nothing applied. The items are N12's (or N4.1's where that slice rewrites the code), under
  decision 43's `Bench:` lines, arm64 only; the QEMU-side arm64 regression (pmaps, trap) is
  untouched by all of them, as the brief said.
- Three more cases the same day, one per trigger, the five models (the user: "uno solo no
  ayuda mucho", "considera también a los descartados", reasoning on, `medium`), $1.74: a
  design fork (A: `kernel::timeout` owner-embedded vs wheel-owned, the N4.1 question, with
  the C, LZ's port and `hal`'s owned API), a security review (B: commit 9dab32dd, the fault's
  typed witness, a clean diff) and a trap (C: the fault's handle clones, which cross sleeps).
  Verified against the code: Gemini 4 Argon 0 false in 27 claims over the four cases and 7
  findings the session did not have; GPT Sol 0 false in 31 and 2; GPT Astra the same findings
  as Sol at five times the price and a blocked sign-off on B; Qwen one false claim in C
  (`fault.rs:879` relocks the amap on `owned.is_some()` without a map relock: "every
  post-unlock use is gated on `locked`" is wrong) and two built on it; DeepSeek 0 false and 0
  new in 38. The defaults stay Gemini + Sol. The system prompt named `uvm_map.c` for every
  brief (Sol and Astra answered "no claim about uvm_map.c" ten times each): fixed to "the C
  excerpt". Spend for the four tests: $2.71.
- What the three cases leave for the work, verified: (A, for the N4.1 planner) `OwnedList`
  owns `PoolBox` nodes (`owned.rs:856-866`), so an owner-embedded `Timeout` needs a new
  non-owning intrusive list in `hal` (new `unsafe`); `timeout_barrier`'s stack `struct
  timeout` (`kern_timeout.c:510`) cannot be a `'static` node in either shape (a pool node, or
  a cond on `tctx_running` without a fake node); `timeout_set_flags` writes without the mutex
  and overwrites `to_flags` (`kern_timeout.c:300-303`), so the native contract for "set while
  pending" must be chosen, not copied; `CIRCQ_CONCAT` (`kern_timeout.c:781`) has no
  `OwnedList` equivalent and the per-element owner id (`owned.rs:888`) makes a move O(n)
  unless `hal` grows a splice; `timeout_del_barrier` from the callback itself waits on its own
  context (`kern_timeout.c:533-549`); all five models recommend the wheel-owned shape. (C, for
  N12) `uvm_fault_check` builds the `EntryView` at 1162 and again at 1226 after `copy_amap`,
  so the COW path clones the handles twice (`fault.upper-cow` is that path); the resident path
  never unlocks the map, so the handles could be lent and cloned only before
  `uvmfault_unlockall` (the slow paths need them: `anonget`'s owned-page relock at 879,
  `lower_io`'s `get_sync` at 2060 and its cleanup at 2104-2110 even when the relock fails); the
  C re-reads `uobj = pg->uobject` after the I/O (`uvm_fault.c:1641`) where the Rust panics on
  an owner-tag mismatch (2105-2108), a deviation the module notes already keep. Nothing
  applied.

## N4 Processes and scheduling

Met 2026-10-24 (under way since 2026-10-19). Boundary: from `docs: close N3` (549a1b93) to the
`docs: close N4` commit. `git rev-list --count --no-merges 549a1b93..8e6db9d3`: 338 commits;
`git log --merges --first-parent 549a1b93..8e6db9d3`: 21 merges into `main`, one per slice or fix.

- What moved. `kernel`: timeout, task, extent (N4.1); proc, credentials, limits (N4.2); the
  descriptors (N4.3); kqueue, select, poll (N4.4), and `struct filterops` as the `Filter<G>`
  trait (N4.4b, decision 46); signals and sigio (N4.5b); futex (N4.6c); fork, exit, resources,
  acct (N4.6a); sysctl's core (N4.6b); the message buffer, `/dev/klog`, sendsyslog (N4.7a); tty,
  the clists, the line disciplines, pty (N4.7b); timecounters, clockintr, the clock, time,
  reboot (N4.8a); the 11 time and reboot calls Native (a6). `hal`: the context switch and the
  scheduler's machine half (N4.5a); spl, softintr, the kernel lock, the mutex (N4.5c); arm64's
  generic timer and the timekeep page (N4.8a); exception vectors, trap, the syscall entry,
  copy, cpufunc, sendsig and sigreturn, the Spectre-BHB trampolines (N4.8b); `pmap_tramp`, the
  `TTBR1_EL1` switch and every kernel mapping non-global (N4.8d, added by the user on
  2026-10-23). Native calls 17 -> 105; `hal` 100 -> 184 (`hal_ceiling` 600, the user's raise
  before N4.5); `bsd` 10196 -> 9490 (`cargo xtask unsafe-report`).
- Went well. Every slice ran surveyor -> planner -> redesigner -> reviewer and was booted on
  arm64 before its merge and after it (decision 23 kept the build out of the commits; the
  boot per merge caught what the host tests could not). The review rule of 2026-10-22 (one
  review, a closing one after a blocking finding or in a security area, fixes to the same
  redesigner) held: N4.8b's two blocking findings (`KcopyToken::take` reachable by a forbid
  crate; `UserFrame::with`'s uniqueness against the EL0 IRQ/FIQ entries) and N4.8d's (the
  direct map's global TLB entries surviving the switch to `pmap_tramp`) were found by the
  first review and closed before the merge. The decisions the user delegated overnight on
  2026-10-22/23 were taken with the recommended option and logged for the user.
- Failed, and what fixed it. N4.5a's first boot panicked (CPU 0's idle thread started with
  argument 0; ac635b78). A missing `uvmspace_addref` from N4.2 made `ps -ax` panic, found only
  when `smoke-diag` joined the post-merge smokes (2536a3e6, and a third site in N4.6b's
  `VsLock`). N4.8a's timekeep claim panicked the first user fault on the page (`PG_HAL_KEEP`,
  e6b89051). At the close: a host size bound N4.8b moved (8e6db9d3: the host's `Mdproc` holds
  the TCB); the kqueue-of-kqueue `diff-openbsd` step, added in N4.4 and first run here, raced
  on both systems (the outer knote is activated by `systqmp`, as the C does; QEMU's TCG runs
  that path cold on EmiBSD), so the step waits up to a second (d444f4e8). Another session's
  builds into `Kernel/target/` (a stale `xtask` with its copy's root, a corrupted `bsd` rlib)
  broke three post-merge and close runs; it now uses its own `CARGO_TARGET_DIR`.
- The exit criterion was restated at the close (the user, 2026-10-24): `sys/kern/` still holds
  N4's 28 adapters over `kernel::*` and files that belong to N5, N7, N8 and N9, each now named
  in its row; seven bridges said `paid_by = "N4"` and were re-dated (3b7c45ae: five to N5,
  whose implementations are the kernel's already; `ModulePlatform` to N9; `TlbCpus` to N10,
  for which `bridges.rs` now accepts `N10`).
- Idioms that took several attempts: the user frame borrowed per access (`UserFrame::with`,
  three shapes before the IRQ/FIQ entries joined it); the kcopy token (lazy in bsd, then taken
  first at boot); the timekeep page's claim (on `PG_BUSY`, then its own bit).
- Open: `just comp` fails inside `xtask comp` (`ln: File exists` on generated LLVM sources);
  `ci-full`'s installer recipes used the compiler of 2026-10-09. `just comp` and
  `smoke-efiboot` still run amd64 steps under decision 44. `unsafe-report --write` would lower
  `hal_ceiling` to 184; it stays at the user's 600.
- Numbers at the close: `just check` rc=0 (217 s) and `just ci` rc=0 (500 s; 67 of 86
  recipes, 19 amd64-only skipped) at 8e6db9d3; `just ci-full` rc=0 (1248 s); `just
  diff-openbsd` rc=0 twice at d444f4e8 (111 steps, 108 equal, 3 expected, 0 unexpected);
  `cargo xtask lz drift --security` empty; `lz check` 1209 modules (209 redesigned), 0 errors.
- `just bench` (arm64, 10 runs, `docs/bench/n4-after-arm64.toml`, 1759 s) against N3's
  `n3-perf-after-arm64.toml`, EmiBSD / OpenBSD medians: pagefault 81425 -> 115036 ns (+41 %;
  OpenBSD +2 %), pipe 171062 -> 230368 (+35 %; +1 %), forkexec 6904563 -> 8571085 (+24 %; +6 %),
  forkexit 2706914 -> 3210724 (+19 %; +5 %), fileread 867319 -> 960900 (+11 %; +4 %), boot
  8.61 -> 9.05 s; ratios to OpenBSD now pagefault 0.915, pipe 1.227, forkexec 1.259, forkexit
  1.383, fileread 1.049. Over decision 43's 10 % without a measured reason: the user's decision
  of 2026-10-24 closes N4 with them recorded and gives them to N12's row (suspects: N4.8d's
  TTBR1 switch and ASID-tagged kernel TLB entries, the owned locks, N4.8b's frame borrows).
- Effort / Time: left for the user.

## N5 VFS and file systems

(The close fills this section; the entries below are dated events inside the milestone.)

### clippy::pedantic in the three crates (2026-10-29)

- The user asked whether `clippy::pedantic` was on (it was not: the workspace lints were a
  hand-picked list over clippy's default set with `-D warnings`) and, after a measurement,
  decided to turn it on in `kernel`, `hal` and `abi`, never in `bsd`.
- Measured first, on the host, read-only: with pedantic as a whole, `kernel` 7,880 warnings
  (the run compiles `hal` and `libz` too), `hal` 785, `abi` 283 of its own; `bsd` 35,182.
  By lint: 4,480 literals without separators (4,135 of them the hex tables of
  `kernel::crypto::{cast,blf,sk,spr}`), 857 backticks in docs, 639 `must_use`, 483 `# Errors`;
  and the ones that matter, 1,348 casts that lose sign, truncate or wrap (kernel 1,061, abi 235,
  hal 52). `abi`'s 235 are all in `sysent.rs`, the syscall arguments: the place where an
  `as u32` on a negative userland value is an ABI bug.
- The shape: `#![warn(clippy::pedantic)]` as an inner attribute of the three crate roots (not
  `[workspace.lints]`, which `bsd` inherits), with the cosmetic lints allowed permanently with a
  `reason` each, and the three cast lints allowed with a `reason` that dates them: `abi`'s 235 are a
  slice of their own at the N5 close or the N6 start (the user, through the N5 session, the
  same day: the userland boundary does not wait), `kernel`'s 1,061 and `hal`'s 52 at N11,
  whose row names them. What remained, 232 small findings in 40
  redesigned or extra files (`uninlined_format_args` 46, `redundant_closure_for_method_calls`
  23, `if_not_else` 22, `single_match_else` 20, `return_self_not_must_use` 16, `borrow_as_ptr`
  16 inside `hal`'s `unsafe`, ...), is fixed by a `mechanical` run; the twelve findings that fell
  in inherited headers of `hal/sys/` (`_IOR` names, `_pad` fields) are two more permanent allows,
  since an inherited file cannot change.
- Userland was not the place: it is in planning, with a designer session working in its tree;
  two `cargo clippy --fix` runs from this session deadlocked against it on one `target/`
  (the lesson is in the session memory: tooling experiments go to the Kernel).
