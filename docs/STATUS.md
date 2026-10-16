# Status

Milestone: **N2 Foundations under way** (replanned 2026-10-16, docs/ZERO_UNSAFE.md section 0);
N1 met on 2026-10-09. Goal: zero `unsafe` in the kernel: `sys/kernel/` and `sys/abi/` are
`#![forbid(unsafe_code)]` crates, `sys/hal/` holds the only `unsafe`, `sys/` (`bsd`, the LZ
tree) is replaced subsystem by subsystem and deleted at N9 (decisions 20-24).
Updated: 2026-10-16.

Done:
- N2 step 1: the crates `hal`, `kernel` (forbid) and `abi` (forbid) exist and `bsd` links them
  (056c20a); `just build` rc=0 on both archs, 0 warnings (2026-10-16). Nothing has moved in yet.
- Decisions 20-24 recorded; ROADMAP N2..N9 replanned with files, unsafe and estimates (0.5, 0.8).
- 2026-10-16 review: the six agent worktrees removed; three branches kept (the typed slots
  ade1ddb, set aside per 0.6; `hal-check` WIP c88f5c8; `lz_home` WIP b0501d6c). N2.2 measured:
  arch/machine/host make 1,334 `crate::` references into `bsd` (117 of 198 files), so the
  move is not mechanical: a surveyor first, then brief and redesigner.

Next (N2, in order; docs/ZERO_UNSAFE.md 0.8): 2. the hardware into `hal` (survey, brief,
redesign); 3. the runtime in `hal` (`Mutexed`, `Rwlocked`, `TypedPool`, `MBox`, `KmBuf`,
`OnceStatic`, `UserPtr`/`abi_pod!`, bridges); 4. `KernelApi`, the ELF loader, `abi-openbsd.ko`,
`hal-check`, `abi-check`, `unsafe-report` v2; 5. crypto into `kernel`; 6. pipe(2) into
`kernel`; 7. the close per decision 24: `just build`, `just check`, one `just ci`, one
`just diff-openbsd`, push. `ci-full` pending: runs once at the N3 close for N2 and N3 together.

Unsafe today (`cargo xtask unsafe-report`, 2026-10-16): 11,225 in the tree = `bsd` legacy 8,906
(8,747 + the 159 of the old runtime files) + hardware 1,891 + host double 38 + our lists/trees
71 + boot loaders 319; `kernel` 0, `abi` 0, `hal` 0 (empty). At N9: `kernel` 0, `abi` 0, `hal`
2,230, `bsd` deleted, boot loaders 319 (section 0.7).
Blockers (inherited from LZ; not regressions): amd64 kernel stacks tight (~4.9 KB free under
softraid I/O); a `diagnostic` MP kernel panics at boot; flakes: amd64 serial cut at QEMU exit,
host test `no idleproc set on CPU0` (~1 in 80), EDK2 UhciDxe ASSERT on arm64 (~1 in 5).
Process: develop first, compile later (decision 23): no build per commit; `just check` and the
QEMU tiers at the milestone close.
