# Glossary

The words this repository uses as if everyone knew them, in one place. Part 1 is the
project's own vocabulary, alphabetical. Part 2 is the tree under `sys/`: every subsystem,
file system, protocol, bus and driver directory, what its name stands for and what it does,
in the order of the tree. The layers of the system are described in full in
`docs/ARCHITECTURE.md` ("Vocabulary", the layers table); here each has one line and a
pointer. Written on 2026-10-24 at the user's request; a new term that a document uses without
explaining, and a new directory under `sys/`, is a new row here, in the same commit.

## Part 1: the project's vocabulary

| Term | Meaning |
|---|---|
| `abi` | **Application Binary Interface.** The contract in bytes between the kernel and the binaries of userland: syscall numbers, `#[repr(C)]` structs (`stat`, `kevent`, `sockaddr`, `kinfo_*`), ioctl and sysctl encodings, errno. Here the crate `sys/abi/`, built as the loadable module `abi_openbsd.ko`: translation only, no logic, `#![forbid(unsafe_code)]`, byte-identical to OpenBSD's so its userland runs unchanged. Layer 1 of `docs/ARCHITECTURE.md`, "Vocabulary" |
| `abi_openbsd.ko` | The binary `abi` is built as: a kernel module `hal`'s ELF loader loads at boot and calls through `abi_init(KernelApi)`; it then registers the syscall handler (decision 22) |
| `abi_pod!` | The macro of `hal` that marks a `#[repr(C)]` type as plain old data, so `UserPtr<T>` may copy it from and to userland without `unsafe` at the call site |
| ACPI | **Advanced Configuration and Power Interface.** The tables the firmware hands the kernel to describe the hardware: on amd64 always, on arm64 when EDK2 boots the machine (`smoke-install-arm64-acpi`). The alternative on arm64 is a device tree (FDT) |
| adapted | A `lineage.toml` status: a module of `bsd` that changed only at its call sites because a type or API it uses was redesigned elsewhere; `adapted_by` names the redesigned module. See inherited, redesigned |
| arm64, amd64 | The two architectures: 64-bit ARM (AArch64) and 64-bit x86. Since decision 44 (2026-10-18) only arm64 is built, booted and measured until N10 homologates amd64 |
| author's block | The ISC licence block of the author that opens the `<LICENSES>` zone of every `.rs` under `sys/` and `tools/` (`.claude/rules/scope-and-stubs.md`, Authorship); the canonical text is `AUTHOR_BLOCK` in `tools/xtask/src/layout.rs` |
| `Bench:` | A commit trailer, `name before -> after unit`, carried by a commit that measured a benchmark; required on a hot-path slice (decision 43) |
| bridge | A trait `kernel` or `hal` declares and `bsd` implements and registers in `hal`'s registry at boot, so `kernel` can reach a subsystem that has not moved out of `bsd` yet without depending on it. One row of `bridges.toml` each (`trait`, `for`, `paid_by`: the milestone whose owner removes it); 0 rows at N9 |
| `bsd` | The Cargo package at `sys/` itself, named after OpenBSD's kernel image `/bsd`: the LandingZone tree, OpenBSD's kernel ported file by file. The legacy that boots and passes every smoke today, replaced subsystem by subsystem and deleted at N9. Calls down into `kernel` and `hal`, never the reverse |
| `bus_dma(9)` | OpenBSD's interface for DMA: how a driver gives a device memory it can read and write. Its hardware side is `hal`'s (`DmaBuf`) |
| ceiling | `hal_ceiling` and `legacy_ceiling` in `unsafe-budget.toml`: the `unsafe` count `hal` and `bsd` may not exceed. `--write` lowers them to the current count at each close; only the user raises one, in a commit of its own. See ratchet |
| `ci`, `ci-full` | `just ci`: build, clippy, fmt, lineage, budget, host tests, the smokes, drift and `diff-openbsd`; `just ci-full`: `ci` with every smoke on four CPUs and the installer end to end. Both run once, at a milestone close (decision 23) |
| close | The end of a milestone: `just check`, `ci`, `ci-full` and `diff-openbsd` green, the numbers in the closing `docs:` commit, the annotated tag `n<N>-<name>` on it |
| `crate::machine` | The one path through which generic code (`kern/`, `uvm/`, `dev/`) reaches architecture code; `crate::arch::amd64` and `crate::arch::arm64` are never named outside `sys/arch/` and `sys/machine/` |
| DDB, `ddb(4)` | OpenBSD's in-kernel debugger, entered on a panic or from the console |
| decision N | A numbered decision of the user, recorded in `docs/ZERO_UNSAFE.md` ("Decisions") or `docs/PHASE2.md`; cited as "decision 44". The number is the reference, never the date alone |
| Deviations | The `//! ## Deviations` list of a module: every inherited gap (an `unported!` path, a stub) it still has; a redesign closes an entry or keeps it visible, never widens it |
| `diff-openbsd` | The oracle: `just diff-openbsd` runs the same scenarios on EmiBSD and on a real OpenBSD snapshot in QEMU and compares them step by step; `expected.toml` lists the differences EmiBSD must have, each with its reason |
| DMA | **Direct Memory Access.** A device reading or writing memory itself, without the CPU; `bus_dma(9)`, `hal::DmaBuf` |
| drift | LZ commits after the pin without a record in `lz-sync.toml`; `cargo xtask lz drift --strict` lists them and fails `ci` while any exists. See lz-sync |
| dropped | A `[[dropped]]` row of `lineage.toml`: an LZ file nothing derives from any more, with its reason; the user's decision |
| DTB, FDT | **Device Tree Blob, Flattened Device Tree.** The firmware's description of an arm64 machine's hardware as a tree, when ACPI is not used |
| EDK2 | The open UEFI firmware QEMU runs for arm64 (and amd64 with OVMF); its UhciDxe assertion is a known flake the smokes retry once |
| EFI, UEFI | **(Unified) Extensible Firmware Interface.** The firmware that boots the machine; `efiboot` is OpenBSD's loader for it, ported at M14 |
| EL0, EL1 | **Exception Level 0 and 1** on arm64: userland and the kernel. The KPTI trampolines switch page tables on every crossing |
| ELF | **Executable and Linkable Format.** The format of the kernel image, of `abi_openbsd.ko` and of every userland binary |
| errno | The error number a failed syscall returns to userland (`<sys/errno.h>`, `hal::sys::errno`). Inside the kernel it is the `Err` of a `Result<T, Errno>`; `abi` turns it into the number |
| `[[extra]]` | A `lineage.toml` row for a native file with no LZ source, with its reason (a project helper) |
| FFS, MFS, NFS | **Fast File System, Memory File System, Network File System.** OpenBSD's root file system, its in-memory one (the installer's ramdisk) and the network one; N5 and N6 |
| forbid | `#![forbid(unsafe_code)]` at a crate root (`kernel`, `abi`): the compiler refuses one `unsafe` in the whole crate, and nothing can `#[allow]` it back. The v1 form, `#[forbid]` on a `mod` of `bsd`, is counted but superseded |
| GIC | **Generic Interrupt Controller.** arm64's interrupt controller (GICv2 and GICv3 in QEMU) |
| `hal` | **Hardware Abstraction Layer.** The crate `sys/hal/`: the hardware (page tables and the pmaps, traps, the syscall entry, the context switch, the interrupt level, MMIO, caches, early boot) and the runtime written new (`Mutexed`, `Rwlocked`, `TypedPool`, `KmBuf`, `OnceStatic`, `UserPtr`, `PerCpu`, the bridge registry, the module loader). The only crate where `unsafe` may appear, every site with a `// SAFETY:` proof, under `hal_ceiling`; its API towards `kernel` is safe (`cargo xtask hal-check` refuses a `pub unsafe fn`). Layer 3 |
| hot path | The page fault, fork, exec, mmap/munmap, the scheduler and what N4 added: a slice that touches one runs the host microbenchmarks before its commit and carries `Bench:` lines; more than 10 % slower without a reason stops the agent (decision 43) |
| inherited | A `lineage.toml` status: a module of `bsd` byte-identical to LZ's at the pin (modulo the RCS ident lines and the author's block). See adapted, redesigned |
| `/insight` | The skill that asks two external models (through OpenRouter, the user's key) one bounded question and verifies every claim against the code before anything becomes work; four triggers only |
| ISC | The licence of the author's block and of most of OpenBSD: permission to use, copy, modify and distribute, with the notice kept |
| `just` | The command runner of the repository (`justfile`): `just build`, `just smoke`, `just ci`, ... Nothing is run by hand (`qemu-system-*`, `cargo build --target`, `rustup`) |
| `kernel` | The crate `sys/kernel/`: every decision the operating system makes (memory, processes, the scheduler, signals, descriptors, kqueue, VFS, the network stack, the drivers' logic, crypto, pf, pledge, unveil, exec), its own types, ownership first, `#![forbid(unsafe_code)]` at the root. Reaches `bsd` only through a bridge. Layer 2 |
| `KernelApi` | The vtable `kernel` exports and `hal` hands `abi_openbsd.ko` at load: one entry per syscall, the only way the module calls the kernel |
| KPTI | **Kernel Page-Table Isolation.** Userland runs on page tables that do not map the kernel, so a speculative-execution leak (Meltdown) finds nothing; the trampolines of N4.8d, OpenBSD's `pmap_tramp` on arm64 |
| kqueue | OpenBSD's event queue (`kqueue(2)`, `kevent(2)`): a descriptor that reports readiness, signals, timers and file changes. Into `kernel` at N4 |
| Legacy, Native | The two states of a syscall table entry: `Native` dispatches to `kernel` through `KernelApi`; `Legacy` falls back to the `bsd` implementation `bsd`'s `main()` registered in `hal` at boot. Every entry is `Native` at N9 |
| `lib` | The leaf crates below `hal`: `sys/lib/libkern/` (what the C kernel takes from libc: string and memory routines, `crc32`, `StaticCell`) and `sys/lib/libz/` (what the kernel compiles of zlib). Layer 3' |
| Limine | The third-party boot loader that booted the kernel until OpenBSD's `boot(8)`/`efiboot` were ported at M14; kept as scaffolding |
| lineage, `lineage.toml` | The record of what every native module derives from in LZ: one `[[module]]` per `.rs` under `sys/` with its status, LZ files and `[[module.fn]]` rows for every item split, renamed, moved, merged or dropped; `cargo xtask lz check` validates it, `lz trace` answers where an item went |
| LZ, LandingZone | The faithful file-by-file port of the OpenBSD kernel to Rust this repository derives from (`EmiBSD/LandingZone`, formerly EmiBSD.LZ), read-only at `reference/emibsd-lz/`, synced to the commit of `lz/PINNED.md`. "The LZ tree" is `bsd` |
| `lz-origin` | LZ's tag of the commit this repository forked at; never moves |
| lz-sync | The monthly absorption of LZ's later commits: each gets one record in `lz-sync.toml` (`applied`, `not-applicable`, `covered-by-redesign`, `deferred-to-N10`) in an `lz-sync:` commit that does nothing else (`docs/SYNC.md`) |
| M0..M16g | The milestones of LZ (the port), before this repository's N milestones; cited where a piece arrived (M14: the boot loaders) |
| machine lock | `/tmp/emibsd/ci.lock`, shared with LandingZone: while it exists no second CI, smoke or full build runs on the Mac |
| milestone, N0..N12 | The rows of `docs/ROADMAP.md`, each with a mechanical exit criterion and a tag at its close: N0 bootstrap, N1 leaves, N2 foundations, N3 memory, N4 processes, N5 VFS, ..., N9 `bsd` deleted, N10 amd64 homologated, N11 verification, N12 performance |
| MMIO | **Memory-Mapped I/O.** Device registers reached by reading and writing memory addresses; `hal::Mmio<Regs>` types them |
| MP, UP | **Multiprocessor, uniprocessor.** OpenBSD's `option MULTIPROCESSOR`; since decision 40 the only kernel built is MP, and `smoke-up` boots it with one CPU |
| `OnceStatic<T>` | `hal`'s init-once static: written once at boot, read afterwards, replacing LZ's `StaticCell` and every `static mut` |
| `pf(4)` | OpenBSD's packet filter, the firewall; a security-sensitive area, redesigned at N8 |
| pin | The LZ commit the tree is synced to (`lz/PINNED.md`, `lineage.toml [meta].lz`); moves only in an `lz: bump pin` commit with the user's OK. `reference/PINNED.md` is the OpenBSD commit LZ itself is pinned to |
| pledge, unveil | `pledge(2)` and `unveil(2)`: the two restrictions an OpenBSD process puts on itself, the syscalls it will still make and the paths it will still see; a violation kills it. Security-sensitive areas |
| pmap | **Physical map.** The machine-dependent half of memory: the page tables of one address space (`pmap(9)`), one implementation per architecture, behind `hal::Pmap` since N3.1 |
| `/progress` | The skill that measures the redesign per milestone from `cargo xtask unsafe-report --json`: files and `unsafe` left in `bsd`, what moved, the bridges open, a time estimate |
| ratchet | The `[ratchet]` table of `unsafe-budget.toml` (`forbid_floor`, `legacy_ceiling`, `hal_ceiling`): numbers that `--write` only moves in the good direction and `--check` enforces |
| redesigned | A `lineage.toml` status: a module written anew from the C's behaviour, into `sys/kernel/` (or `hal`, `abi`), with `//! LZ:` lines, a `## Redesign` section and `[[module.fn]]` rows for every item that moved. See inherited, adapted |
| `reference/` | The read-only trees: `reference/openbsd-src/` (the C, the behavioural specification, at LZ's pin), `reference/emibsd-lz/` (LZ, full history), `reference/posix-2024/` (the standard). Never edited, never committed, never `cd`'d into |
| `// SAFETY:` | The comment every `unsafe` block carries: a soundness argument (the invariant, who establishes it, who upholds it, why the operation cannot break it). "The C does this" is not one |
| scaffolding | What exists only while `bsd` does: the glue generic, the associated types of `ProcGlue` and `FileGlue`, `bsd`'s aliases and adapter modules, the system-call hop through `bsd`'s `native()`. One row each in `docs/SCAFFOLDING.md` with the milestone that removes it; the bridges are `bridges.toml`'s |
| seam (v1) | A safe function a legacy module exported with the `unsafe` inside it, so a `forbid` module could call it before its own slice; superseded by bridges on 2026-10-16, no new seam is written |
| semihosting, `isa-debug-exit` | How a smoke makes QEMU exit with a code: arm64's `SYS_EXIT` semihosting call and amd64's `isa-debug-exit` device |
| slice | One unit of a milestone's plan (`docs/ZERO_UNSAFE.md` 0.8): N4.3, N4.8b. One worktree branch, one redesigner, one review, one commit series, merged at the close |
| smoke | A QEMU boot test: `just smoke-<name>` boots the kernel headless, asserts serial lines and the exit code; `just smoke` runs them all, `JOBS` at a time |
| softintr | **Software interrupt.** A handler deferred to run below the hardware interrupt level (`softintr(9)`) |
| spl | **System priority level.** OpenBSD's interrupt masking levels (`spl(9)`: `splnet`, `splhigh`, ...); into `hal` at N4 |
| `StaticCell` | LZ's cell for a mutable global, in `libkern`; replaced in native code by `OnceStatic`, `Mutexed` or atomics, kept in `bsd` until its file is replaced |
| subagent | A Claude Code agent with a role of `.claude/agents/` (`surveyor`, `planner`, `redesigner`, `reviewer`, `mechanical`, `integrator`, `debugger`, `milestone-coordinator`, `openbsd-probe`, `image-worker`), launched one by one by the main session under `.claude/rules/subagents.md` |
| sysctl | OpenBSD's interface to read and set kernel state by a numeric path (`sysctl(2)`, the MIB); the names and numbers userland sees never change |
| TLB | **Translation Lookaside Buffer.** A CPU's cache of page-table entries; a change to a mapping must flush it (`pmap`, `hal`) |
| trailer | The `Key: value` lines closing a commit message: `LZ:`, `Unsafe:`, `Security-Review:`, `Bench:`, `Co-Authored-By:` (`.claude/rules/git-commits.md`) |
| `unported!` | The macro that makes an inherited gap visible: prints it once and returns `ENOSYS`; never a `todo!()` or a silently dropped path |
| `unsafe-report` | `cargo xtask unsafe-report`: counts every `unsafe` block, fn, impl and trait per subsystem and per crate; `--check` enforces the budget, `--write` records the totals |
| `Unsafe:` | A commit trailer, `subsystem before -> after`, one line per subsystem whose `unsafe` count the commit moved |
| `UserPtr<T>` | `hal`'s typed pointer into userland memory: the one way a `copyin`/`copyout` happens, safe at the call site, `abi_pod!` on the type |
| UVM | OpenBSD's virtual memory system (`uvm(9)`), redesigned into `kernel::vm` at N3; `sys/uvm/` is deleted |
| VFS | **Virtual File System.** The layer independent of any file system (`vfs(9)`: vnodes, mounts, the name cache), under which FFS, MFS, NFS and the rest sit; N5 |
| worktree | A `git worktree`: a second checkout of the repository on its own branch, where every writing subagent works so the main checkout stays on `main` |
| xtask | `tools/xtask/`: the repository's host tooling as a Cargo package (`cargo xtask image`, `qemu`, `smoke`, `lz`, `unsafe-report`, `diff-openbsd`, ...), called through `just` |
| zones | The three markers every `.rs` under `sys/` carries: `/* <LICENSES> */` (the author's block, then the original notices whole), `/* <CODE> */`, `/* <TESTS> */` (the inline `mod tests`); `cargo xtask lz check` validates them |

## Part 2: the tree under `sys/`

What each directory is in OpenBSD and here. The man page in parentheses (`ffs(4)`, `pf(4)`,
`vfs(9)`) is OpenBSD's own description. A driver's name is the one `dmesg` prints (`em0`,
`vio0`, `sd0`). Where a directory has moved into `kernel` or `hal`, the row says so; the rest
is `bsd`, legacy, until its milestone (`docs/ROADMAP.md`).

### The core

| Directory | Name | What it is |
|---|---|---|
| `sys/kern/` | kernel | the machine-independent core of OpenBSD's kernel: processes, scheduling, signals, descriptors, syscalls, the tty layer, sockets, VFS's generic half. Its `kern_*`, `subr_*`, `sys_*`, `uipc_*`, `vfs_*`, `tty_*` and `exec_*` files are listed below. Being emptied into `kernel`: at N4 proc, fork, exit, credentials, descriptors, kqueue, signals, futex, the scheduler, sysctl, tty, the clock and time moved |
| `kern_*` | | one kernel service each: `kern_clock` (ticks, `hardclock`), `kern_clockintr` (per-CPU clock interrupts), `kern_descrip` (file descriptors), `kern_event` (kqueue), `kern_exec` (execve), `kern_intrmap` (interrupt-to-CPU maps for multi-queue NICs), `kern_kthread` (kernel threads), `kern_lock` (the kernel lock, `KERNEL_LOCK`), `kern_malloc` (`malloc(9)`), `kern_physio` (raw device I/O), `kern_pledge` and `kern_unveil` (the two self-restrictions), `kern_prot` (uid, gid, credentials), `kern_resource` (rlimits, priorities, rusage), `kern_rwlock` (`rwlock(9)`), `kern_sched` (the per-CPU run queues), `kern_sensors` (`sensorsd`'s hardware sensors framework), `kern_sig` (signals), `kern_smr` (Safe Memory Reclamation, a read-side lock-free scheme), `kern_softintr` (software interrupts), `kern_subr` (uio copy, hooks), `kern_synch` (`tsleep`/`wakeup`, the sleep queues), `kern_sysctl` (`sysctl(2)`), `kern_tc` (timecounters, `nanotime`), `kern_time` (`clock_gettime`, `setitimer`), `kern_watchdog` (hardware watchdog timers), `kern_bufq` (the disk I/O queue, N5.1), `kern_xxx` (reboot, the odd syscalls) |
| `sched_bsd` | | the 4.4BSD scheduler's policy (priorities, `schedclock`, `yield`); `kern_sched` holds its queues. Into `kernel::sched` at N4 |
| `subr_*` | subroutines | shared helpers: `subr_autoconf` (device attachment, `config_attach`, the device tree of `dmesg`), `subr_disk` (disklabels, `disk(9)`), `subr_evcount` (event counters shown by `vmstat -i`), `subr_log` (`/dev/log`, the kernel message buffer), `subr_percpu` (per-CPU data), `subr_pool` (`pool(9)`, the fixed-size allocator), `subr_prf` (`printf(9)`, `panic`), `subr_prof` (kernel profiling), `subr_tree` (red-black trees, `RBT_*`), `subr_userconf` (`boot -c`, editing the device config at boot), `subr_xxx` (the `enodev`/`nullop` stubs) |
| `sys_*` | system calls | syscalls that are not a subsystem of their own: `sys_futex` (`futex(2)`), `sys_generic` (`read`, `write`, `select`, `poll`, `ioctl`), `sys_socket` (socket descriptors' file ops) |
| `uipc_*` | Unix interprocess communication | sockets: `uipc_socket`/`uipc_socket2` (the socket layer, buffers), `uipc_syscalls` (`socket`, `bind`, `sendmsg`, ...), `uipc_usrreq` (`AF_UNIX` sockets), `uipc_domain`/`uipc_proto` (the protocol family tables), `uipc_mbuf`/`uipc_mbuf2` (mbufs, the network packet buffers, `mbuf(9)`) |
| `vfs_*` | Virtual File System | the file-system-independent half of files: `vfs_syscalls` (`open`, `mkdir`, `rename`, ...), `vfs_vnops` (read/write on a vnode), `vfs_lookup` and `vfs_cache` (`namei(9)`, path lookup and the name cache), `vfs_subr` (vnode life cycle), `vfs_bio`/`vfs_biomem` (the buffer cache, `buf(9)`), `vfs_sync` (the syncer), `vfs_lockf` (file locks), `vfs_getcwd` (`__getcwd`), `vfs_init`, `vfs_default`, `vfs_vops` (the vnode operations table). N5 |
| `tty*` | teletype | the terminal layer: `tty` (line discipline, `termios`), `tty_pty` (pseudo-terminals, `pty(4)`), `tty_tty` (`/dev/tty`), `tty_subr` (clists, the character queues), `tty_conf`. Into `kernel::tty` at N4 |
| `exec_*` | | program loading: `exec_elf` (the ELF loader), `exec_script` (`#!`), `exec_subr` (mapping the image) |
| `spec_vnops` | special files | the vnode operations of device nodes (`/dev/*`): I/O goes to the driver's `cdevsw`/`bdevsw` |
| `init_main`, `init_sysent` | | `main()`, the boot sequence after the machine is up; the syscall table (generated, `syscalls.master`) |
| `pipe_glue`, `proc_glue`, `vm_bsd`, `hal_bridges`, `abi_module`, `rust_alloc`, `unported`, `selftest` | | this repository's own files in `kern/` (`[[extra]]` rows): the `bsd` side of the bridges to `kernel`'s pipe, proc and vm, the registrations in `hal`, loading `abi_openbsd.ko`, the Rust global allocator over `malloc(9)`, the `unported!` macro, the boot self-tests the smokes assert |
| `sys/sys/` | system headers | OpenBSD's `<sys/*.h>` as Rust modules: the shared types and constants (`proc`, `vnode`, `mbuf`, `socket`, `stat`, `signal`, `termios`, `ioctl`, `syscall`, `errno` ...). The constant headers of the ABI edge moved to `sys/hal/sys/` (decision 27); the rest follow their subsystem |
| `sys/conf/` | configuration | `param` (the tunables `maxproc`, `nmbclust`, ... as `param.c`), `swapgeneric` (the root and swap device at boot), `vers` (the version string `dmesg` prints first) |
| `sys/ddb/` | DDB | the in-kernel debugger (`ddb(4)`): `db_command` (its commands), `db_expr`/`db_lex`/`db_input`/`db_output` (its parser and console), `db_run` (breakpoints, single step), `db_trap` (how a panic enters it), `db_usrreq` (`ddb.*` sysctls), `db_variables` |
| `sys/crypto/` | | the kernel's cryptography (`crypto(9)`): `crypto` and `cryptosoft` (the framework and its software driver), `cryptodev` (`/dev/crypto`), `xform` (the IPsec transforms table), `aes`, `rijndael` (AES), `blake2s`, `siphash`, `curve25519`, `gmac` (AES-GCM's MAC), `idgen` (unpredictable id sequences), `criov` (iovec helpers). Security-sensitive, N8; `kernel::crypto` already holds `md5`, `sha1`, `sha2`, `rmd160`, `hmac`, `chacha`, `poly1305`, `chachapoly`, `blf` (Blowfish), `cast` (CAST-128), `des`, `xform_ipcomp` |
| `sys/benches/` | | host microbenchmarks of `bsd` (`harness = false`): the legacy scheduler and timeout wheel, for the before/after of decision 43 |

### The file systems

| Directory | Name | What it is |
|---|---|---|
| `sys/ufs/` | Unix File System | the family of BSD's native file systems and what they share: `ufs/ufs/` (inodes, directories, `dirhash`, quotas, `ufs_lookup`, `ufs_vnops`: the on-disk layout common to FFS, MFS and ext2fs) |
| `sys/ufs/ffs/` | Fast File System | OpenBSD's root file system (`ffs(4)`, FFS1 and FFS2): cylinder groups, block allocation (`ffs_alloc`, `ffs_balloc`), the superblock (`fs`), `ffs_vfsops` (mount, sync, fsck's view), `ffs_vnops` |
| `sys/ufs/mfs/` | Memory File System | an FFS in memory (`mount_mfs(8)`): the installer's ramdisk and `/tmp` on some setups |
| `sys/ufs/ext2fs/` | second extended file system | Linux's ext2, and ext3/ext4 read-only (`mount_ext2fs(8)`): `ext2fs_extents` is ext4's extent tree |
| `sys/msdosfs/` | MS-DOS file system | FAT12/16/32 (`mount_msdos(8)`): the boot sector (`bootsect`), the BIOS parameter block (`bpb`), the file allocation table (`fat`), directory entries with long names (`direntry`, `msdosfs_conv`), `denode` (its inode) |
| `sys/isofs/` | ISO 9660 file system | `cd9660` (CD-ROM file systems, `mount_cd9660(8)`, with Rock Ridge and Joliet) and `udf` (Universal Disk Format, DVDs, `mount_udf(8)`) |
| `sys/ntfs/` | NT File System | Windows' NTFS, read-only (`mount_ntfs(8)`): the MFT (`ntfs_inode`), attributes, compressed files (`ntfs_compr`), name conversion (`ntfs_conv`) |
| `sys/nfs/` | Network File System | the NFS client and server (`nfs(4)`, `mount_nfs(8)`, `nfsd(8)`), versions 2 and 3 over Sun RPC: `krpc_subr` and `xdr_subs` (RPC and XDR encoding), `nfs_socket` (the transport), `nfs_serv`/`nfs_srvcache`/`nfs_srvsubs` (the server and its duplicate-request cache), `nfs_bio` (its buffer cache), `nfs_boot`/`nfsdiskless` (booting from NFS), `nfs_kq` (kqueue on NFS files) |
| `sys/tmpfs/` | temporary file system | a file system in memory and swap (`mount_tmpfs(8)`), unlike MFS not an FFS image: `tmpfs_mem` is its allocator |
| `sys/miscfs/` | miscellaneous file systems | `deadfs` (the vnode operations of a vnode whose file system went away: every op fails) and `fuse` (Filesystem in Userspace, `fuse(4)`: a file system served by a process through `/dev/fuse`) |

### The network

| Directory | Name | What it is |
|---|---|---|
| `sys/net/` | | the protocol-independent network layer: `if_` and `if_var` (the interface, `ifnet`), `ifq` (interface queues), `if_ethersubr` (Ethernet input and output), `if_arp`/`if_dl`/`if_types`/`ethertypes` (link-layer types), `if_media` (`ifconfig media`), `if_loop` (`lo0`), `route`/`rtable`/`rtsock`/`radix`/`art` (the routing table: `art` is OpenBSD's Allotment Routing Table, `radix` the older trie, `rtsock` the routing socket `route(8)` talks to), `netisr` (the network software interrupt), `toeplitz` (the hash multi-queue NICs spread flows with), `fq_codel` and `hfsc` (the queueing disciplines of `pf.conf`'s `queue`) |
| `bpf*` | Berkeley Packet Filter | `bpf(4)`: `tcpdump`'s tap on an interface and its filter machine (`bpf_filter`) |
| `pf*` | packet filter | OpenBSD's firewall (`pf(4)`, `pf.conf(5)`): `pf` (the rule engine and state table), `pf_ioctl` (`pfctl`'s interface), `pf_ruleset`, `pf_table` (address tables), `pf_norm` (packet normalisation, `scrub`), `pf_lb` (load balancing, NAT and rdr pools), `pf_osfp` (passive OS fingerprinting), `pf_syncookies` (SYN cookies), `pf_if` (interface groups), `pfvar`/`pfvar_priv` (its structures). Security-sensitive |
| `if_pflog`, `if_pflow`, `if_pfsync` | | pf's companions: `pflog0` (logged packets as a BPF interface), `pflow(4)` (NetFlow/IPFIX export), `pfsync(4)` (state table replication between two firewalls) |
| `if_wg`, `wg_noise`, `wg_cookie` | WireGuard | the WireGuard VPN interface (`wg(4)`): the Noise handshake, the cookie reply mechanism. Security-sensitive, N8 |
| `if_enc` | | `enc0`: the interface where IPsec traffic appears before encryption and after decryption, for pf and tcpdump |
| `pfkeyv2*` | PF_KEY version 2 | the key management socket IPsec daemons (`iked`, `isakmpd`) load security associations through (RFC 2367) |
| `sys/netinet/` | Internet | IPv4 and the transports: `ip_input`/`ip_output`/`ip` (IPv4), `ip_icmp` (ICMP), `igmp` (multicast group management), `in_pcb` (protocol control blocks: the 4-tuple table of sockets), `in_cksum`/`in4_cksum` (the Internet checksum), `ip_id` (unpredictable IP ids), `ip_ecn` (Explicit Congestion Notification), `ip_divert` (`divert(4)` sockets for pf), `ip_ipip` (IP-in-IP tunnels, `gif`), `raw_ip`, `inet_nat64` (pf's NAT64 translation), `if_ether` (ARP) |
| `tcp_*` | Transmission Control Protocol | `tcp_input`, `tcp_output`, `tcp_timer` (retransmission, keepalive), `tcp_subr`, `tcp_usrreq` (the socket side), `tcp_fsm` (the state machine), `tcp_seq`, `tcp_debug` |
| `udp*` | User Datagram Protocol | `udp_usrreq`, `udp6_output` |
| `ip_ipsp`, `ip_spd`, `ipsec_input`, `ipsec_output`, `ip_esp`, `ip_ah`, `ip_ipcomp` | IP Security | IPsec: the security policy database (`ip_spd`) and association database (`ip_ipsp`), input and output processing, and the three protocols: ESP (Encapsulating Security Payload: encryption), AH (Authentication Header: integrity only), IPcomp (IP Payload Compression). Security-sensitive, N8 |
| `sys/netinet6/` | Internet version 6 | IPv6: `ip6_input`/`ip6_output`/`ip6_forward`, `icmp6`, `nd6`/`nd6_nbr`/`nd6_rtr` (Neighbor Discovery: address resolution, routers, prefixes), `mld6` (Multicast Listener Discovery), `frag6` (reassembly), `dest6` (the destination options header), `in6_ifattach` (link-local addresses), `in6_src` (source address selection), `in6_pcb`, `route6`, `raw_ip6`, `ip6_divert`, `ip6_id` |

### The buses and the drivers (`sys/dev/`)

The tree mirrors the buses a device hangs from: a driver's logic is in `dev/ic/` (independent
of the bus chip) and its attachment in `dev/pci/`, `dev/isa/`, `dev/usb/`, ... (decision 45
keeps the layout in `kernel::dev`, N7). `*reg` is a chip's register layout, `*var` the
driver's own structures.

| Directory | Name | What it is |
|---|---|---|
| `dev/pci/` | Peripheral Component Interconnect | the PCI and PCIe bus (`pci(4)`): `pci`, `pci_map`, `pci_subr`, `pci_quirks`, `pcidevs` (the vendor and device id table), `ppb` (PCI-to-PCI bridges), and the attachments: `if_em` (Intel Gigabit Ethernet, `em(4)`), `if_vmx` (VMware vmxnet3, `vmx(4)`), `if_fxp_pci` (Intel EtherExpress Pro/100, `fxp(4)`), `if_re_pci` (Realtek 8139C+/8169, `re(4)`), `if_dc_pci` (DEC 21143 "tulip" clones, `dc(4)`), `if_ne_pci` (NE2000 clones), `if_pcn` (AMD PCnet), `ahci_pci`, `nvme_pci`, `mpi_pci`, `siop_pci`, `pciide` (IDE/SATA controllers in legacy mode, one `pciide_*_reg` per chipset family), `ehci_pci`/`ohci_pci`/`uhci_pci`/`xhci_pci` (USB host controllers), `auich` (Intel ICH audio), `azalia` (Intel HD Audio), `eap` (Ensoniq AudioPCI), `ichiic`/`piixpm` (SMBus controllers), `cy82c693` (a legacy chipset), `puc` (PCI "universal" serial/parallel cards), `sdhc_pci` (SD host controllers), `vga_pci`, `virtio_pci`, `vmwpvs` (VMware paravirtual SCSI), `gcu` (Intel EP80579 Global Configuration Unit) |
| `dev/isa/` | Industry Standard Architecture | the legacy PC bus (`isa(4)`) and what still hangs from it: `com_isa` (serial ports), `pckbc_isa` (the keyboard controller), `fd`/`fdc` (floppy), `lpt_isa` (parallel), `pcppi`/`spkr` (the PC speaker), `isadma` (the 8237 DMA controller), `vga_isa` |
| `dev/ic/` | integrated circuits | bus-independent chip drivers: `com` (the 16550 UART, `com(4)`), `pluart` (ARM PrimeCell PL011 UART, arm64's console), `ahci` (SATA AHCI controllers, `ahci(4)`), `nvme` (NVM Express SSDs, `nvme(4)`), `wdc` (IDE/ATA controllers, `wd(4)` disks), `siop` (Symbios/NCR 53c8xx SCSI), `mpi` (LSI Fusion-MPT SCSI/SAS), `re`/`rtl81x9` (Realtek), `dc` (tulip), `fxp` (Intel 8255x), `ne2000`/`dp8390`/`rtl80x9` (NE2000 Ethernet family), `lancereg`/`am79900reg` (AMD LANCE/PCnet), `ax88190reg` (ASIX), `vga`, `pcdisplay` (text-mode VGA/MDA), `pckbc` (the 8042 keyboard controller), `lpt` (parallel port), `ac97` (audio codecs), `i8042reg`/`i8237reg`/`i8253reg`/`i8259` (the PC's keyboard controller, DMA controller, timer and interrupt controller), `mc146818reg` (the real-time clock), `mc6845reg` (the CRT controller), `nec765reg` (floppy controller), `ns16550reg` (the UART) |
| `dev/usb/` | Universal Serial Bus | the USB stack (`usb(4)`): `usb`, `usbdi` (the device interface drivers use), `usb_subr`, `usb_mem`, `usbdevs` (the vendor/product table), `usb_quirks`, `uhub` (hubs), the host controllers `uhci` (USB 1.1, Intel), `ohci` (USB 1.1, others), `ehci` (USB 2.0), `xhci` (USB 3), and the device drivers: `umass` (mass storage: USB disks as SCSI), `ukbd`/`ums`/`uhid`/`uhidev` (keyboards, mice, HID), `uwacom` (tablets), `ucom`/`uftdi` (serial adapters), `if_cdce` (CDC Ethernet), `uaudio` (audio), `ugen` (generic access from userland), `usbpcap` (USB packet capture for `tcpdump`) |
| `dev/hid/` | Human Interface Device | the HID report parser (`hid`) and the keyboard and mouse logic (`hidkbd`, `hidms`) shared by USB and Bluetooth devices |
| `dev/pckbc/` | PC keyboard controller | the devices behind the 8042: `pckbd` (the PS/2 keyboard), `pms` (the PS/2 mouse, with Synaptics, ALPS and Elantech touchpads), `wskbdmap_mfii` (the key maps) |
| `dev/wscons/` | workstation console | the console framework (`wscons(4)`): `wsdisplay` (screens, virtual consoles), `wskbd` (keyboards), `wsmouse` (mice, `wstpad` touchpad gestures), `wsmux` (multiplexing several input devices into one), `wsemul_vt100` (the VT100 terminal emulation), `wsemul_dumb`, `wsksymdef` (key symbols), `wsconsio` (its ioctls), `wsdisplay_compat_usl` (the `/dev/ttyC*` virtual terminal switching X uses) |
| `dev/rasops/` | raster operations | drawing text on a framebuffer, one module per depth (`rasops1` monochrome to `rasops32`), with `rasops_bitops` and `rasops_masks` |
| `dev/wsfont/` | workstation fonts | the console fonts: Spleen in four sizes |
| `dev/ata/` | AT Attachment | the ATA/IDE and SATA disk protocol: `wd` (ATA disks, `wd(4)`), `ata_wdc`, `atascsi` (presenting SATA disks as SCSI, how `ahci` and `nvme` show `sd(4)`), `satareg`, `pmreg` (port multipliers) |
| `dev/sdmmc/` | SD / MultiMediaCard | SD cards and eMMC: `sdhc` (SD Host Controller standard), `sdmmc` (the bus), `sdmmc_mem` (memory cards as `sd(4)` through `sdmmc_scsi`), `sdmmc_io` (SDIO), `sdmmc_cis` (the card information structure) |
| `dev/mii/` | Media Independent Interface | Ethernet PHYs (the transceiver chips behind a NIC): `mii` (the bus), `mii_physubr`, `miidevs`, and the PHY drivers `ukphy` (generic), `rgephy`/`rlphy` (Realtek), `inphy` (Intel), `lxtphy` (Level One), `dcphy` (DEC 21143 internal) |
| `dev/pv/` | paravirtual | drivers for virtual machines: `virtio` (the virtio bus, `virtio(4)`) with `virtio_pci` and `virtio_mmio` attachments, and its devices `if_vio` (network, `vio(4)`), `vioblk` (block, `vioblk(4)`), `vioscsi`, `viornd` (entropy), `viomb` (memory balloon), `viocon` (console), `viogpu`. What QEMU gives EmiBSD |
| `dev/acpi/` | Advanced Configuration and Power Interface | the ACPI interpreter and tables (`acpi(4)`): `dsdt` (the AML interpreter, `amltypes`), `acpimadt` (the Multiple APIC Description Table: CPUs and interrupt controllers), `acpimcfg` (PCIe memory-mapped config space), `acpiprt` (PCI interrupt routing), `acpihpet`/`acpitimer` (timers), `acpicpu_x86` (C-states, P-states), `acpidmar`/`amd_iommu` (IOMMUs: Intel VT-d's DMAR table and AMD's), `ipmi_acpi`, `tpm` (Trusted Platform Module), `pluart_acpi` |
| `dev/fdt/` | Flattened Device Tree | drivers attached from a device tree on arm64: `pciecam` (PCIe ECAM host bridge), `psci` (Power State Coordination Interface: CPU on/off, reboot), `pluart_fdt`, `plgpio`/`plrtc` (PrimeCell GPIO and RTC), `simplefb` (the firmware's framebuffer), `virtio_mmio`, `gpiokeys`, `ipmi_fdt` |
| `dev/ofw/` | Open Firmware | the device tree API (`fdt`, `openfirm`: `OF_getprop`, ...) and helpers (`ofw_gpio`, `ofw_pinctrl`, `ofw_misc`: clocks, regulators, phys) |
| `dev/efi/` | Extensible Firmware Interface | `efi(4)`: the runtime services the kernel keeps after boot (the real-time clock, variables) |
| `dev/i2c/` | Inter-Integrated Circuit | the two-wire bus (`iic(4)`): `i2c`, `i2c_exec`, `i2c_io` (`/dev/iic*`), `i2c_scan` (probing for known sensor chips) |
| `dev/gpio/` | General-Purpose I/O | `gpio(4)`: pins exposed to userland (`gpioctl(8)`) |
| `dev/puc/` | PCI universal communications | `com_puc`: serial ports on PCI multi-port cards |
| `dev/microcode/` | | firmware the kernel loads into a chip: `fxp` (Intel 8255x), `siop` (the NCR SCRIPTS processor) |

### The architectures

| Directory | Name | What it is |
|---|---|---|
| `sys/arch/amd64/` | | 64-bit x86 (frozen until N10, decision 44). `amd64/`: `locore`/`locore0` (the assembly entry and low-level routines), `machdep` (machine-dependent setup, `cpu_startup`), `cpu`/`identcpu` (CPU attach and feature detection), `lapic`/`ioapic`/`apic`/`i8259` (the local and I/O APICs and the legacy PIC: interrupt controllers), `intr`/`vector`/`spl` (interrupt dispatch and masking), `trap` (exceptions and the syscall entry), `copy` (`copyin`/`copyout`), `fpu` (the FPU and SSE/AVX state), `gdt` (the Global Descriptor Table: segments), `mptramp`/`ipi`/`ipifuncs` (starting the other CPUs and inter-processor interrupts), `tsc` (the Time Stamp Counter as timecounter), `bios`/`efifb`/`consinit`/`mem`/`disksubr`/`bus_dma`/`bus_space`/`vm_machdep`/`acpi_machdep`/`mainbus`/`autoconf`/`qemu`. `isa/`: the ISA clock (`clock`, the 8254 and RTC), `nvram`. `pci/`: `pci_machdep`, `acpipci`, `pciide_machdep`. `include/`: the machine headers (`cpu`, `pcb`, `frame`, `segments`, `specialreg`, `pio`, `psl`, `vmparam`, `param`, ...). `stand/`: `efiboot` and its `libsa` |
| `sys/arch/arm64/` | | 64-bit ARM (AArch64), the architecture built and tested. `arm64/`: `locore`, `machdep`, `cpu`/`cpufunc` (CPU attach, cache and TLB operations, errata), `intr` (the interrupt framework), `cpuswitch` (the context switch, moved into `hal` at N4), `fpu`, `db_interface`/`db_trace`, `bus_dma`/`bus_space`, `mem`, `disksubr`, `vm_machdep`, `acpi_machdep`, `qemu`. `dev/`: `mainbus`, `simplebus` (the device tree's bus), `agintc` (the GICv3 interrupt controller), `ampintc` (GICv2), `agtimer` (the ARM generic timer), `smmu` (the System MMU, ARM's IOMMU, from ACPI's IORT or the device tree), `acpiiort`, `acpipci`, `pci_machdep`, `efi_machdep`. `include/`: `armreg` (system registers), `frame`, `tcb`, `hypervisor` (EL2 calls), `bootconfig`, `fdt`, `efivar`, .... `stand/`: `efiboot` |
| `sys/arch/host/` | | the host test double: what `cargo test` on macOS links in place of a machine |
| `sys/machine/` | | the `crate::machine` contract: one module per machine-dependent interface (`cpu`, `intr`, `pmap`, `copy`, `bus`, `cons`, `atomic`, `proc`, `signal`, `tcb`, `disklabel`, `autoconf`, `pci_machdep`, ...) dispatching to the arch selected; the only way generic code reaches `sys/arch/` |
| `sys/hal/arch/` | | the hardware that moved into `hal`: `arm64/` (`pmap`, `trap`, `exception.S`, `syscall`, `copy`, `copystr.S`, `cpuswitch.S`, `switch`, `cpu`, `cpufunc`, `fpu`, `sig`, `timer`, `trampoline.S` for KPTI), `amd64/` (`pmap` only, the rest at N10), `host/` (the test double of each) |
| `sys/hal/machine/`, `sys/hal/vm/`, `sys/hal/sys/` | | the machine parameters (`param`, `vmparam`, `exec`), the vm constants (`uvmexp`, `param`) and the constant headers of the ABI edge (`errno`, `syscall`, `signal`, `sysctl`, `fcntl`, `mman`, `types`, ...) that moved into `hal` (decision 27) |
| `sys/hal/module/` | | the loader of `abi_openbsd.ko`: `elf` (parsing), `load` (mapping and relocating), `rt` (the runtime it hands the module) |

### The boot loaders and the libraries

| Directory | Name | What it is |
|---|---|---|
| `sys/stand/` | standalone | the boot loaders' common code: `boot` (OpenBSD's `boot(8)`: the prompt, `boot.conf`, loading `/bsd`), `efi` (the UEFI glue), `bootarg` (what the loader passes the kernel), `fdtfb` (a framebuffer from the device tree), `limine` and `limine.conf` (the Limine scaffolding) |
| `sys/lib/libsa/` | standalone library | the loaders' libc (`libsa(9)`): a tiny `alloc`, `printf`, `open`/`read`/`stat` over `ufs`/`ufs2`/`cd9660`, `loadfile`/`loadfile_elf` (loading the kernel), the network boot (`arp`, `ether`, `netif`, `netudp`, `tftp`, `in_cksum`), `disklabel`, `cons` |
| `sys/lib/libkern/` | kernel library | what the kernel takes from libc (`libkern(9)`): `strlcpy`, `strlcat`, `strnlen`, `strncasecmp`, `scanc`, `skpc`, `getsn`, `crc32c`, `explicit_bzero`, `timingsafe_bcmp` (constant-time compare), `random` (the kernel's `arc4random`), `staticcell` |
| `sys/lib/libz/` | zlib | what the kernel compiles of zlib: crc32, adler32, deflate, inflate (compressed kernels and `ipcomp`) |
| `sys/kernel/` | | the `kernel` crate (Part 1): `proc`, `fork`, `exit`, `thread`, `cred`, `prot`, `resource`, `sched`, `sig`, `sigio`, `descrip`, `file`, `event` (kqueue), `select`, `pipe`, `futex`, `sysctl`, `syscall`, `tty`, `pty`, `termios`, `clist`, `clock`, `clockintr`, `tc`, `time`, `timeout`, `task`, `extent`, `uio`, `acct`, `log`, `prf`, `reboot`, `api` (`KernelApi`), `vm/` (uvm redesigned: `map`, `fault`, `page`, `pager`, `amap`, `anon`, `aobj`, `vnode`, `swap`, `pdaemon`, `pmemrange`, `km`, `mmap`, `unix`, `meter`, `glue`), `crypto/` (above) |
