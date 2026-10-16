# Status

Milestone: **N2 Foundations under way** (replanned 2026-10-16, docs/ZERO_UNSAFE.md section 0);
N1 met on 2026-10-09. Goal: zero `unsafe` in the kernel: `sys/kernel/` and `sys/abi/` are
`#![forbid(unsafe_code)]` crates, `sys/hal/` holds the only `unsafe`, `sys/` (`bsd`, the LZ
tree) is replaced subsystem by subsystem and deleted at N9 (decisions 20-24).
Updated: 2026-10-16.

Done:
- N2.1: the crates `hal`, `kernel` (forbid), `abi` (forbid), `bsd` linking them (056c20a);
  `just build` rc=0 on both archs, 0 warnings (2026-10-16). Decisions 20-25 recorded.
- N2.4b: the v2 tooling merged (c1b2bea3): `hal-check`, `abi-check`, `unsafe-report`'s crates
  block, `lz check` with `crate` and `bridges.toml`, `hal_ceiling` 2150; xtask tests 247.
- N2.2 slice 1 (de5af8ac, f619ca94): 24 machine headers and 18 constant headers in `hal`,
  inherited, old paths re-exported (`types.rs` back to bsd: uvm_param's inherent impls, N3).
  N2.5 (736103ad): 18 of 28 crypto files in `kernel`, 0 unsafe; `just build` rc=0 after each.

Next (N2, in order; docs/ZERO_UNSAFE.md 0.8): 3. the runtime in `hal` (branch fixed after review,
32 unsafe, second review); 4. `KernelApi`, the ELF loader, `abi-openbsd.ko` (branch reviewed,
fixes under way); 6. pipe(2) into `kernel`; 7. the close per decision 24: integrate, `just
build`, `just check`, one `just ci`, one `just diff-openbsd`, push; `ci-full` at N3. Decisions
26-28 and 30 taken today; the rest of the hardware moves by owner (N3 pmap, N4 cpu, N7 bus).

Unsafe (`cargo xtask unsafe-report`): kernel 8913 blocks, 1126 fn, 829 impl, 30 trait, 340 other; tests 903 more; core 2512, legacy 8726, forbid 320 of 1195 modules; crates: kernel 0, abi 0, hal 35, bsd 10582 (1697 to move into hal), boot 621.

Blockers (inherited from LZ; not regressions): amd64 kernel stacks tight (~4.9 KB free under
softraid I/O); a `diagnostic` MP kernel panics at boot; flakes: amd64 serial cut at QEMU exit,
host test `no idleproc set on CPU0` (~1 in 80), EDK2 UhciDxe ASSERT on arm64 (~1 in 5).
Process: develop first, compile later (decision 23): no build per commit; `just check` and QEMU at the close.
