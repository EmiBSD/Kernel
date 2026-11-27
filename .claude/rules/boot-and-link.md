---
paths:
  - "sys/bsd/stand/**"
  - "sys/boot/loader/**"
  - "sys/boot/efi/**"
  - "sys/bsd/arch/*/conf/**"
  - "sys/bsd/build.rs"
  - "sys/bsd/main.rs"
  - ".cargo/**"
  - "rust-toolchain.toml"
---

# Boot and link

- The kernel is booted only by OpenBSD's own boot loader, boot(8)/efiboot (`sys/boot/libsa`,
  `sys/boot/loader`, `sys/boot/efi`, `sys/boot/<arch>`; docs/ARCHITECTURE.md, "Boot loaders"),
  which loads `/bsd` from the FFS `a` partition of the boot disk (decision 57, the user,
  2026-11-26: Limine is gone on both archs; amd64 is unbootable until N10's first step converts
  its recipes). No Multiboot, no Linux image header, no direct `-kernel` loading, no boot
  modules: init is `/sbin/init` on the root, and a test word reaches the kernel as the preset
  `cargo xtask efiboot-image --selftest` writes into its copy of the kernel.
- efiboot is built for the `none` targets (no UEFI target), position-independent:
  `RUSTFLAGS="-C relocation-model=pie"` in `just efiboot-<arch>`, its own `--target-dir
  target/efiboot`, its `ldscript.<arch>` with the hand-written PE header first, and
  `cargo xtask efiboot` (`llvm-objcopy -O binary`). Code that runs before `self_reloc` may not
  use statics, the GOT, formatting or panics.
- `sys/bsd/stand/` holds `mod.rs` and `bootarg.rs` only (N9 owns them): both archs' `locore0.S`
  call `bootarg_main`, which takes the arch-neutral `BootInfo` (memory map, DTB/RSDP, the
  boot DUID, the MP description) from `machine::Cpu::getbootinfo` (boot(8)'s `bootarg` list
  on amd64, efiboot's `/chosen` on arm64) and calls `stand::start_kernel`: `boothowto`, the
  DUID, `machine::Machine::early_init`, the banner, then `kern::init_main::main`. The symbol
  `_start` is arm64's `locore0.S` entry.
- `sys/bsd/arch/{amd64,arm64}/conf/kernel.ld`: `PHDRS` text/rodata/data (M16d: and the C's
  `openbsd_randomize` `PT_OPENBSD_RANDOMIZE` header over `.openbsd.randomdata` inside rodata,
  which boot(8) fills with the seed; `etext` on both archs), no `.requests` section,
  `.eh_frame*`/`.note*` discarded. arm64 (M14, so efiboot can load it): base
  `0xffffffff80000000`, physical addresses from 0 by `AT()` (efiboot's `LOADADDR` keeps 39
  bits and adds its 64 MB block), `.text.locore0` first, `__bss_start`/`_end`/`end` for
  `locore0.S`, `ENTRY(__start_phys)` (`_start`'s offset: `e_entry` goes through `LOADADDR`
  too). amd64 (M14, so efiboot can
  load it): OpenBSD's `ld.script` layout, `KERNTEXTOFF` `0xffffffff81000000`, physical addresses
  from `0x1000000` by `AT()`, page-aligned sections with the symbols `locore0.S` reads, `.got`
  inside `.data` (nothing after `end`), `ENTRY(start)` (the 32-bit boot(8) entry). The two
  scripts no longer have to be identical.
  `sys/bsd/build.rs` passes the script with `rustc-link-arg-bins` only when `target_os = "none"`.
- `.cargo/config.toml` never sets `[build] target`: a plain `cargo test`/`cargo check` must build
  for the host. Kernel builds always name the target through `just`.
- Rustflags are per target: `relocation-model=static` (non-PIE higher-half kernel) and
  `force-frame-pointers=yes` (panic backtraces). Adding a flag needs a line in `docs/ARCHITECTURE.md`.
- `rust-toolchain.toml` pins a stable version. Bumping it is its own commit titled
  `build: bump toolchain to <version>` and needs the user's OK. Never add nightly or `-Z` flags.
- The kernel is `/bsd` on the FFS root with `/etc/boot.conf` and `/etc/random.seed`, as OpenBSD
  installs it; the ESP holds only `EFI/BOOT/BOOT{AA64,X64}.EFI` (`tools/xtask/src/efiboot.rs`).
