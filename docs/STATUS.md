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
- N2.2 slice 1 merged (de5af8ac): the 24 bsd-free machine headers and the 19 constant headers
  of sys/sys/ in `hal`, inherited, old paths re-exported; `hal` 1 unsafe. Decisions 26-28 (N2.2
  staged by owner, headers in hal, hal primitives only) and 30 (safe memory bridges) taken today.

Next (N2, in order; docs/ZERO_UNSAFE.md 0.8): 3. the runtime in `hal` (branch written, 105
unsafe, under review; decision 30 being applied); 4. `KernelApi`, the ELF loader,
`abi-openbsd.ko` (in flight); 5. crypto into `kernel`; 6. pipe(2) into `kernel`; 7. the close
per decision 24: `just build`, `just check`, one `just ci`, one `just diff-openbsd`, push;
`ci-full` at N3. The rest of the hardware moves by owner (N3 pmap, N4 cpu/clock/trap, N7 bus).

Unsafe today (`cargo xtask unsafe-report`, 2026-10-16): `kernel` 0, `abi` 0, `hal` 1 (ceiling
2,150), `bsd` 10,603 in 1,118 files (8,906 legacy + 1,697 hardware, host double, lists and trees
not moved yet), boot loaders 621; total 11,225. At N9: `hal` ~1,430, `bsd` deleted, boot 621 (0.7).
Blockers (inherited from LZ; not regressions): amd64 kernel stacks tight (~4.9 KB free under
softraid I/O); a `diagnostic` MP kernel panics at boot; flakes: amd64 serial cut at QEMU exit,
host test `no idleproc set on CPU0` (~1 in 80), EDK2 UhciDxe ASSERT on arm64 (~1 in 5).
Process: develop first, compile later (decision 23): no build per commit; `just check` and QEMU at the close.
