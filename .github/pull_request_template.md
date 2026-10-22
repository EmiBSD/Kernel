<!--
External pull requests are not accepted, for now (CONTRIBUTING.md). If you have a fix,
open an issue describing it instead; a diff in the issue is fine, and it will be applied
with credit.
-->

## What and why

<!-- LZ's shape, the native shape, and why. -->

## Checklist

- [ ] Nothing userland can see changed (syscalls, sysctl MIBs, ioctls, device majors,
      `#[repr(C)]` shared with userland, lines a smoke or `diff-openbsd` compares)
- [ ] `lineage.toml` updated in the same commit (status, `lz` list, `[[module.fn]]` rows)
- [ ] Trailers: `LZ:` per source file, `Unsafe:` when a count moved, `Security-Review:` in a
      `security-review.md` area, `Bench:` on a hot path
- [ ] `cargo fmt --check` and `cargo xtask lz check` green
- [ ] No `unsafe` outside `sys/hal/`; every `unsafe` block has a `// SAFETY:` argument
- [ ] Nothing under `reference/` or `lz/` touched; no amd64 code before N10
