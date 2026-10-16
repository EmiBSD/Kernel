# Zero unsafe in the kernel

The kernel is a crate the compiler refuses to build with one `unsafe` in it; the only `unsafe`
of the system lives in a small HAL below it, proved site by site; an ABI crate above it keeps
OpenBSD's userland interface byte-identical and decides nothing. Section 0 is the plan in force
(v2, 2026-10-16); sections 1-9 record v1 and its measurements.

## 0. v2 (2026-10-16): the kernel is a crate with no `unsafe`

Decisions 20 and 21 (section 10) replace the shape of sections 4, 6 and 7: the kernel is not
"the LZ tree with a listed core", it is a **new crate with `#![forbid(unsafe_code)]` at its
root**, and the LZ tree is what that crate replaces. Everything below here is the plan in force;
sections 1-9 stay as the record of how the first version was measured and why it was dropped.

### 0.1 The four crates

| Crate | Path | `unsafe` | Holds |
|---|---|---|---|
| `hal` | `sys/hal/` | the only `unsafe` of the system, every site with a soundness proof, Miri on its host tests | the hardware (today's `sys/arch/amd64`, `sys/arch/arm64`, `sys/machine`, moved in), the host test double (`sys/arch/host`), and the runtime written new: `Mutexed<T>`, `Rwlocked<T>` (locks that own their data), `TypedPool<T>`/`PoolArc<T>`, `MBox<T>`, `KmBuf` (owned memory), `OnceStatic<T>` (the init-once static that replaces `StaticCell`), `List`/`Tree` (today's `queue.rs`, `tree.rs`, `subr_tree.rs`), `PerCpu<T>`, `Smr`, `Refcnt`, `UserPtr<T>` with `abi_pod!` (typed copyin/copyout), `Mmio<Regs>`, `DmaBuf`, the interrupt-level guard, the console primitive, the bridge registry |
| `kernel` | `sys/kernel/` | **0**, `#![forbid(unsafe_code)]` at the root: the compiler refuses any | every decision the operating system makes: memory (vm), processes, scheduler, signals, descriptors, kqueue, pipes, VFS and the file systems, the network stack, the drivers' logic, crypto, pf, pledge, unveil, exec. Its own types, ownership first, no C shape |
| `abi` | `sys/abi/`, built as the loadable module `abi-openbsd.ko` (decision 22) | **0**, `#![forbid(unsafe_code)]` (the user allowed `unsafe` here; none is needed: `UserPtr` is safe) | OpenBSD's userland interface, byte-identical: the syscall table, the `#[repr(C)]` structs (`stat`, `kevent`, `sockaddr`, `kinfo_*`, ...), ioctl and sysctl encodings, errno. Translation only (decision 21): bytes to the kernel's types, one kernel entry point per syscall, the `Result` to an errno; no check on what a value means, no state |
| `bsd` | `sys/` (the LZ tree, today's crate) | legacy: 8,906 today (the 8,747 outside the old core plus the 159 of the old runtime files, which are legacy now), **0 files at N9** | what gets replaced. It keeps booting, running every smoke and `diff-openbsd` throughout; it links the three crates and calls down into them; the kernel crate never calls it except through a registered bridge (0.3) |

The boot loaders (`sys/stand`, `sys/lib/libsa`, 319 `unsafe`) are separate binaries that run
before the kernel; they are outside the four crates and unchanged. `libkern` and `libz` stay
crates of their own, `forbid` (`StaticCell` moves to `hal`, so `libkern` can be).

Dependencies, one direction: `bsd -> kernel -> hal`; `abi -> hal` (for `UserPtr`) and `abi -> KernelApi` (the vtable `hal` hands it at load, decision 22); nothing names `abi`, plus `libkern`/`libz` under
`kernel`. `hal` depends on nothing of ours. No cycle: a kernel-to-legacy call is a bridge.

### 0.2 What the compiler guarantees

- `cargo build` of `kernel` or `abi` with one `unsafe` anywhere, in any module, under any
  `cfg`, fails: `#![forbid(unsafe_code)]` cannot be overridden below the root. There is no
  allow list, no core list, no per-module attribute.
- `hal`'s API is safe: an `unsafe fn` in `hal` is `pub(crate)`. `cargo xtask hal-check` fails
  `just check` on a `pub unsafe fn`, on an `unsafe` block without a `// SAFETY:`, and on a
  `hal` count above `hal_ceiling` in `unsafe-budget.toml` (lowered by `--write`, never raised
  without the user).
- `abi` holds no logic: `cargo xtask abi-check` fails `just check` on a function of `abi` that
  branches on anything but the syscall number, its own conversion's `Ok`/`Err` and the byte
  layout, that calls more than one `kernel` entry point, or that exceeds the per-syscall line
  limit (decision 21).

### 0.3 The migration mechanism

1. **A subsystem moves, it is not patched.** The unit of work is still one owner module (or
   cluster) of the ROADMAP row: the surveyor inventories the LZ module and the C, the planner
   writes the brief, the redesigner writes the subsystem **in `kernel`** from the C's behaviour
   (LZ's shape is reference, never the starting text), deletes the LZ module from `bsd`, and
   leaves in `bsd` at most a call-site adapter with 0 `unsafe` (`adapted`, `adapted_by` the
   kernel module) until the callers move too.
2. **Bridges, for the kernel's calls into what has not moved yet.** `kernel` declares a trait
   (`pub trait Vfs`, `pub trait Sockets`, ...) and holds it in a `hal::OnceStatic<&'static dyn
   Trait>`; `bsd` implements it over its legacy code and registers it in `main()` before first
   use. Every bridge is a row of `bridges.toml` (`trait`, `for`, `paid_by` = the milestone whose
   subsystem removes it); `cargo xtask` counts them; N9 requires 0. A bridge carries no logic
   of its own (the same rule as `abi`).
3. **The syscall table lives in `abi` from N2 on.** Each entry is `Native(fn)` (translation into
   `kernel`) or `Legacy` (the fallback `bsd` registers at boot). A migrated syscall flips its
   entry; `diff-openbsd` and the smokes see no difference, which is the test.
4. **Lineage keeps every provenance rule.** A kernel module is `redesigned` with its `lz` list
   and `//! LZ:` lines; the LZ file it replaces becomes `[[dropped]]` with `absorbed by
   sys/kernel/...`; `lineage.toml` gains `crate`. An LZ change to a dropped file is
   `covered-by-redesign` or `reimplemented` in `kernel` (`lz-sync.md`).
5. **Tests.** `kernel`'s host tests run on `hal`'s host backend; `just check` (build on both
   archs, clippy, fmt, `lz check`, `hal-check`, `abi-check`, `unsafe-report --check`, host
   tests) gates every commit; the smokes, `just ci`, `ci-full` and `diff-openbsd` run at the
   milestone close (the user's rule of 2026-10-16).

### 0.4 The metric

`cargo xtask unsafe-report` prints four numbers and `/progress` shows them per milestone:
`kernel` (always 0, else the build failed), `abi` (always 0), `hal` (today's hardware code plus
the new runtime; the ceiling in `unsafe-budget.toml`, lowered only), and `bsd` (legacy: the
`unsafe` still in the LZ tree, per ROADMAP row). A row is **complete when its files in `bsd`
are 0** (deleted, or a 0-`unsafe` adapter listed as such) and its subsystem answers in
`kernel`. The old `forbid` count is gone: nothing in `kernel` needs marking.

### 0.5 Milestones, replanned

Measured on `main` at `47e1c4f` with `/progress`. "Files" and "`unsafe`" are what leaves `bsd`.
The estimate is continuous agent time (sessions open around the clock), from the one slice
measured end to end on 2026-10-16 (the typed slots: 4 h for two series of a mid-size slice);
it is the planning session's number, labelled as such, and `/progress` replaces it once five
active days have moved 100 `unsafe`.

| Milestone | Scope (what moves into `kernel`/`abi`/`hal`) | Files | `unsafe` | Exit (mechanical) | Est. |
|---|---|---:|---:|---|---:|
| **N2 Foundations** (under way, replanned) | the crates `hal`, `kernel`, `abi` exist and `bsd` links them; `hal` holds the hardware code (moved) and the new runtime (locks, pools, `MBox`, `KmBuf`, `OnceStatic`, lists and trees, `UserPtr`/`abi_pod!`, the bridge registry); `abi` holds the syscall table (all `Legacy`) and the 41 constant headers of `sys/sys/`; `kernel` holds crypto (28 files, already safe), and the first subsystem by the new path, **pipe(2)** (`sys_pipe.rs`, `pipe.rs`), with bridges to descriptors, kqueue and sigio; `xtask`: `hal-check`, `abi-check`, `unsafe-report` v2, `/progress` v2, `bridges.toml` | 2 + 28 + 41 + the hardware move | 21 (+ 1,891 moved into `hal`, not removed; + 30 new for the module loader) | `kernel` and `abi` build with `forbid` at the root; `hal` ≤ 2,150 with every site proved; `sys_pipe.rs` and `pipe.rs` deleted from `bsd`; `just ci`, `ci-full`, `diff-openbsd` equal | 9 d |
| **N3 Memory** | `sys/uvm/` → `kernel::vm` in owner slices (km, object, anon, amap, aobj, vnode pager, page, pmemrange, addr, map, mmap, glue; `uvm_fault` last, with `security-review.md`); the pmap behind a safe `hal::Pmap` API | 26 | 215 | `sys/uvm/` gone from `bsd`; `kernel::vm` answers every `uvm_*` call through its API; `hal::Pmap` has no `pub unsafe fn` | 6 d |
| **N4 Processes** | `sys/kern/` by owner into `kernel`: timeout, task, event/kqueue, descriptors (`file`, `kern_descrip`), signals, proc, the scheduler, sysctl's core, tty; `StaticCell` uses in `kern` → `OnceStatic` | 75 + 4 (kqueue and descriptors, N2's before) | 680 + 95 | `sys/kern/` in `bsd` holds only the VFS (N5), the sockets (N6) and the N8 files; the bridges `Proc`, `Fd`, `Kq` removed from `bridges.toml` | 12 d |
| **N5 VFS and file systems** | `sys/kern/vfs_*`, `sys/ufs/`, `sys/isofs/`, `sys/msdosfs/`, `sys/nfs/`, `sys/ntfs/`, `sys/tmpfs/`, `sys/miscfs/` → `kernel::fs` | 140 | 816 | those directories gone from `bsd`; the `Vfs` bridge removed | 14 d |
| **N6 Network** | `sys/net/`, `sys/netinet/`, `sys/netinet6/`, the sockets of `sys/kern/uipc_*` → `kernel::net`, except N8's files | 99 | 1,840 | those directories gone from `bsd` but N8's files; the `Sockets` bridge removed | 18 d |
| **N7 Devices** | `sys/dev/` and `sys/scsi/` → `kernel::dev`, one bus at a time, over `hal::Mmio<Regs>` and `hal::DmaBuf`; `sys/dev/rnd.rs` last | 395 | 4,198 | `sys/dev/` and `sys/scsi/` gone from `bsd` but `softraid_crypto.rs` (N8) | 40 d |
| **N8 Security subsystems** | `pf*`, `if_wg`/`wg_*`, IPsec, `softraid_crypto`, `pledge`, `unveil`, `exec_*`, the crypto framework → `kernel`, each with `Security-Review:` and the area's smokes before every commit | 36 | 849 | those files gone from `bsd`; `lz drift --security` empty | 8 d |
| **N9 Close** | what is left: `sys/ddb/`, `sys/conf/`, `init/`, the old runtime files (`subr_pool`, `kern_lock`, `kern_rwlock`, `kern_malloc`, `smr`, `percpu`: deleted with their last user), `sys/sys/` emptied into `abi`/`kernel`; the `bsd` crate deleted; the binary is `kernel` + `abi` + `hal`; `bridges.toml` empty; `hal`'s floor recorded with its proof list; Miri green on `hal` | 14 + the old runtime 19 | 28 + 159 | `sys/` holds `hal/`, `kernel/`, `abi/`, `stand/`, `lib/`; `unsafe-report`: `bsd` does not exist, `kernel` 0, `abi` 0, `hal` = its floor | 5 d |

Total: 8,906 `unsafe` out of `bsd` (8,747 legacy + 159 old runtime files), 580 + 19 files, about **112 days of continuous agent
time** at the measured pace, serial (the slices share files); this number is the planning
session's and promises nothing: `/progress` measures.

What does not change in any milestone: what userland sees (`abi` is byte-identical, the smokes
and `diff-openbsd` are the proof), the LZ sync (`lz-sync.md`, with `covered-by-redesign` for
dropped files), the two architectures on every commit, the per-commit `just check`.

### 0.6 The work in flight on 2026-10-16

The two branches of the typed-slots slice (`worktree-agent-ab058223a74c4dcd6` at `ade1ddb`,
series 0 and 1 reviewed; `worktree-agent-ad1fb0331db500561`, series 2 half done) redesign
inside `bsd`. Under v2 that is not the goal: `bsd` is deleted, not improved. They are **not
merged**; their shapes (`FileData` by value to the close, `CredRef`, the typed hook over `dyn
Any`, `KlistLock`) are reused when descriptors and kqueue move into `kernel` at N4. About
eight hours of agent work set aside by the architecture change; said here so it is not
recounted as progress.


### 0.7 The whole account (measured 2026-10-16 with `unsafe-report --json`)

| Today | `unsafe` | At N9 | `unsafe` |
|---|---:|---|---:|
| LZ code not redesigned (580 files) | 8,747 | rewritten in `kernel`, deleted | 0 |
| old LZ runtime files (19: `subr_pool` 58, `subr_percpu` 16, `kern_malloc` 14, `staticcell` 9, `kern_lock` 8, `smr` 8, `kern_tc` 7, `subr_prf` 7, `percpu` 7, `kern_smr` 6, `kern_subr` 6, `malloc` 3, `kern_rwlock` 2, `mutex` 2, `rwlock` 2, `explicit_bzero` 1, `pool` 1, `refcnt` 1, `systm` 1) | 159 | deleted with their last user; `hal` replaces them | 0 |
| hardware (`arch/amd64` 863, `arch/arm64` 943, `machine` 85) | 1,891 | moved into `hal` | 1,891 |
| host test double (`arch/host`) | 38 | moved into `hal` | 38 |
| ours (`queue`, `tree`, `subr_tree`) | 71 | moved into `hal` | 71 |
| new `hal` code, written new, each site proved: N2 runtime 120 + ELF loader 30, N3 `Pmap` API 10, N4 `cpu_switch`/clock glue 10, N7 `Mmio`/`DmaBuf`/interrupts 60 | 0 | added by the milestone that needs it, a commit each | 230 (estimate) |
| boot loaders (`stand`, `libsa`), separate binaries | 319 | unchanged | 319 |
| **total** | **11,225** | | **2,549** |

Removed from the repository: 8,676 (8,747 + 159 − 230). `kernel` 0 and `abi` 0 by the
compiler; `hal` 2,230, 85 % of it the hardware code that exists today; the boot loaders apart.

What remains after each close (bsd / hal / total with the boot loaders): today 8,906 / 2,000 /
11,225; N2 8,885 / 2,150 / 11,354 (the total rises: the runtime and the loader are written
before the legacy falls); N3 8,670 / 2,160 / 11,149; N4 7,895 / 2,170 / 10,384; N5 7,079 /
2,170 / 9,568; N6 5,239 / 2,170 / 7,728; N7 1,041 / 2,230 / 3,590; N8 192 / 2,230 / 2,741;
N9 0 / 2,230 / 2,549. `hal`'s ceiling in `unsafe-budget.toml` falls between milestones and
rises only in the commit that adds a listed piece, with the user's OK.

### 0.8 The slices, milestone by milestone

Each slice is one surveyor -> planner -> redesigner -> reviewer -> `just check` cycle; the
smokes, `just ci`, `ci-full` and `diff-openbsd` run at the close. Files and `unsafe` are the
tool's at `47e1c4f`; days are continuous agent time, the planning session's estimate.

**N2 Foundations, 9 d.** 1. The crates: `sys/hal/`, `sys/kernel/` and `sys/abi/` with
`#![forbid(unsafe_code)]` on the last two, `bsd` linking `kernel` and `hal`, `just check` green
(0.5 d). 2. The hardware into `hal`: `sys/arch/amd64`, `sys/arch/arm64`, `sys/machine`,
`sys/arch/host`, one `lineage:` move per directory, code untouched, `crate::machine` paths kept
by re-export (1 d). 3. The runtime, each piece with its first user in `kernel` or in the move:
`List`/`Tree` (moved), `Mutexed<T>`, `Rwlocked<T>`, `TypedPool<T>`/`PoolArc<T>`, `MBox<T>`,
`KmBuf`, `OnceStatic<T>`, `PerCpu<T>`, `Refcnt`, `UserPtr<T>` with `abi_pod!`, the bridge
registry, the interrupt-level guard, the console primitive; `hal-check` (3 d). 4. `KernelApi`
(the vtable), the ELF loader, `abi-openbsd.ko` with every entry `Legacy` and the 41 constant
headers, the boot loaders and the installer reading two files, `smoke-abi`, `abi-check`,
`unsafe-report` v2 (kernel/abi/hal/bsd), `/progress` v2, `bridges.toml` (2 d). 5. Crypto into
`kernel` (28 files, 0 `unsafe`, already safe) (0.5 d). 6. **pipe(2)** into `kernel`: its own
`Pipe`/`PipePair` over `Rwlocked`, `PoolArc`, `KmBuf`; bridges `Fd` (descriptor and `File`
glue), `Kq` (knote/klist), `Sigio`, `Uio` (the copy); `sys_pipe.rs` and `pipe.rs` deleted from
`bsd`; the two syscall entries flipped to `Native` (1.5 d). 7. Close: `just ci`, `ci-full`,
`diff-openbsd`, docs, push (0.5 d). Out of `bsd`: 21 `unsafe`, 71 files (2 + 28 + 41).

**N3 Memory, 6 d.** 1. `hal::Pmap`: a safe typed API over the two arch pmaps (`enter`,
`remove`, `protect`, `extract`, `activate`; `uvm_pmap.rs` is the shape), the page allocator's
hardware side (1 d). 2. `uvm_km`, `uvm_object`, `uvm_anon`, `uvm_extern`, `uvm` -> `kernel::vm`
(0.5 d). 3. `uvm_page`, `uvm_pmemrange`, `uvm_addr` (86 `unsafe`, the free lists over `List`/
`Tree`) (1.5 d). 4. `uvm_amap`, `uvm_aobj`, `uvm_vnode`, `uvm_pager` (61; bridge `Vfs` for the
vnode pager) (1 d). 5. `uvm_map`, `uvm_mmap`, `uvm_glue`, `uvm_io`, `uvm_meter`, `uvm_percpu`,
`uvmexp`, `uvm_init`, `uvm_pdaemon`, `uvm_swap`, `uvm_unix` (bridge `Proc` for the current
vmspace, `Swap` for the device) (1.5 d). 6. `uvm_fault`, `security-review.md`, its smokes before
every commit (0.5 d). Out of `bsd`: 215 `unsafe`, 26 files; `sys/uvm/` deleted.

**N4 Processes and scheduling, 12 d.** 1. `kern_timeout`, `kern_task`, `subr_extent` (hal: the
clock interrupt as a registered handler) (1 d). 2. `kern_proc`, `kern_prot` (credentials, the
`CredRef` shape), `kern_fork`, `kern_exit`, `kern_resource`, `kern_sysctl`'s core (2 d). 3.
Descriptors: `sys/sys/file.rs`, `kern_descrip.rs`, `sys_generic` (the `FileData` shape; the `Fd`
bridge of N2 removed) (1.5 d). 4. kqueue: `kern_event.rs`, `sys/sys/event.rs` (the typed hook
over `dyn Any`, `KlistLock`; the `Kq` bridge removed) (1.5 d). 5. `kern_sig`, `kern_synch`,
`kern_sched`, `sched_bsd`, `kern_kthread`, `kern_smr`'s logic (the sleep queues and run queues in
`kernel` over `hal::cpu_switch`; the `Sigio` bridge removed) (2.5 d). 6. `tty`, `tty_pty`,
`tty_tty`, `tty_conf`, `subr_log` (1.5 d). 7. `kern_bufq`, `subr_disk`, `subr_autoconf`,
`subr_hibernate`, `kern_time`, `kern_clock`, `kern_xxx`, `subr_*`, `selftest.rs` rewritten as
`kernel` tests (1.5 d). 8. Close (0.5 d). Out of `bsd`: 699 `unsafe`, 56 files + the 4 of the
old N2 (`kern_event`, `kern_descrip`, `event.rs`, `file.rs`) = 775 in 60 files.

**N5 VFS and file systems, 14 d.** 1. The VFS core: `vfs_subr`, `vfs_bio`, `vfs_lockf`,
`vfs_syscalls`, `vfs_lookup`, `vfs_vnops`, `vfs_cache`, `vfs_init`, `vfs_sync`, `vfs_getcwd`,
`vfs_vops`, `vfs_default`, `spec_vnops` (149; the `Vfs` bridge becomes the kernel's own; bridge
`Disk` to the block drivers until N7) (4 d). 2. `ufs` + `ffs` + `ext2fs` (43 files, 228) (3 d).
3. `nfs` (24, 143; over `kernel::net`'s sockets or the `Sockets` bridge) (2 d). 4. `isofs`
(cd9660, udf; 16, 89) (1.5 d). 5. `msdosfs` (12, 60) and `ntfs` (9, 54) (1.5 d). 6. `miscfs`
(fuse, fifofs, deadfs) (10, 37) and `tmpfs` (7, 24) (1.5 d). 7. Close (0.5 d). Out of `bsd`:
816 `unsafe`, 134 files; those directories deleted.

**N6 Network stack, 18 d.** 1. The core: `if_`, `route`, `rtable`, `art`, `radix`, `rtsock`,
`if_ether`, `if_loop`, `netisr`, `pfkeyv2`'s stub (858 in `sys/net/` minus N8's files; bridge
`Ifnet` to the NIC drivers until N7) (5 d). 2. `netinet`: `in_`, `in_pcb`, `ip_input`,
`ip_output`, `tcp_*`, `udp_usrreq`, `raw_ip`, `igmp`, `ip_icmp`, `if_ether` (38, 465) (5 d). 3.
`netinet6` (25, 355) (4 d). 4. The sockets: `uipc_socket`, `uipc_socket2`, `uipc_usrreq`,
`uipc_syscalls`, `uipc_mbuf`, `uipc_domain`, `sys_socket` (9, 155; `mbuf` over `TypedPool`;
the `Sockets` bridge removed) (3 d). 5. Close (0.5 d). Out of `bsd`: 1,840 `unsafe`, 95 files.

**N7 Devices, 40 d.** 0. `hal`: `Mmio<Regs>`, `DmaBuf`, typed interrupt handlers, `bus_space`
and `bus_dma` behind them, the autoconf attach protocol as safe traits (3 d). 1. `dev/pv` first,
virtio: every smoke boots on it (12, 249) (3 d). 2. `dev/pci` core and `ppb`, then `dev/ic`'s
shared chips with their `pci` front ends, bus by bus (`com`, `pckbc`, `ahci`, `nvme`, `em`,
`ix`, `re`, `vmx`, `mpi`, `azalia`, `auich`, `eap`, `vga`, ...) (135, 1,401; the `Ifnet` and
`Disk` bridges removed as each driver moves) (14 d). 3. `wscons` + `rasops` + `wsfont` (47,
611) (4 d). 4. `usb` (43, 528) (5 d). 5. `acpi` (18, 205), `fdt` + `ofw` (15, 115), `isa` (19,
106) (4 d). 6. `scsi` (10, 149), `ata` (9, 82), `sdmmc` (13, 110), `softraid` (6, 192 but
`softraid_crypto`) (4 d). 7. `audio`, `midi`, `mii`, `ipmi`, `gpio`, `i2c`, `hid`, `rd`, `vnd`,
`bio`, `cons`, `puc`, `efi`, `clock_subr`, `firmload`, `microcode` (2 d). 8. `dev/rnd.rs` last,
`security-review.md` (0.5 d). 9. Close (0.5 d). Out of `bsd`: 4,198 `unsafe`, 395 files. Honest
limit: a driver no QEMU device exercises is verified by `just check` and its host tests only;
the list of those drivers goes in the close commit.

**N8 Security subsystems, 8 d.** Every commit with `Security-Review:`, the area's smokes and
`diff-openbsd` before it. 1. pf: `pf`, `pfvar`, `pfvar_priv`, `pf_table`, `pf_ioctl`,
`pf_norm`, `pf_if`, `pf_osfp`, `pf_lb`, `pf_ruleset`, `pf_syncookies` (already safe), `if_pfsync`,
`if_pflog`, `if_pflow` (3 d). 2. WireGuard: `if_wg`, `wg_cookie`, `wg_noise` (1 d). 3. IPsec:
`ip_ipsp`, `ip_esp`, `ip_ah`, `ip_ipcomp`, `ipsec_input`, `ipsec_output`, `pfkeyv2`,
`pfkeyv2_convert`, `pfkeyv2_parsemessage` (2 d). 4. `kern_pledge`, `kern_unveil`, `exec_*` and
the ELF loader of userland (1 d). 5. The crypto framework: `crypto`, `cryptosoft`, `criov`,
`cryptodev`; `softraid_crypto` (0.5 d). 6. Close (0.5 d). Out of `bsd`: 849 `unsafe`, 32 files.

**N9 Close, 5 d.** 1. `ddb` (10, 15), `sys/conf/` (3, 2), `sys/reftest.rs`, `init/` (the
stand-in, 11) (1.5 d). 2. The old runtime files deleted with their last user: `subr_pool`,
`subr_percpu`, `kern_malloc`, `kern_lock`, `kern_smr`, `kern_rwlock`, `kern_tc`, `subr_prf`,
`kern_subr`, `staticcell`, `explicit_bzero` and their `sys/sys/` headers (19, 159);
`sys/sys/` emptied into `abi` and `kernel` (1.5 d). 3. The `bsd` crate deleted; the binary is
`kernel` + `hal`; `bridges.toml` empty; `hal`'s floor recorded with its proof list; Miri green on
`hal`'s host tests; `README`, `ARCHITECTURE`, `JOURNAL` (1.5 d). 4. `ci-full`, `diff-openbsd`,
push (0.5 d). Out of `bsd`: 192 `unsafe`, 33 files; `sys/` holds `hal/`, `kernel/`, `abi/`,
`stand/`, `lib/` and nothing else.

Sum: 8,906 `unsafe` and 599 files out of `bsd`; 112 days. Order of risk, highest first: the
sleep/wakeup and run queues over `hal::cpu_switch` (N4.5), the ELF loader (N2.4), the VFS
core's locking (N5.1), `if_` and the routing tables (N6.1), `uvm_map` (N3.5), `uvm_fault` (N3.6).

## 1. Status and goal

**Status: in force since 2026-10-10** (`docs/PHASE2.md`, "Decisions"; section 10 records the
answers). The plan is iterative: each step is measured and this file follows what the steps
teach. Every number below was measured on `main` at `72ea58e`, with
`cargo xtask unsafe-report`, `grep` over `sys/` and one compile experiment.

### The goal, as a tool can check it

1. **No `unsafe` outside the core.** Every module outside a listed core compiles under
   `#[forbid(unsafe_code)]`. The compiler then refuses `unsafe` blocks, `unsafe fn`,
   `unsafe trait` and `unsafe impl` there, macro expansions included.
2. **The core is listed and budgeted.** All `unsafe` lives in the paths of `unsafe-core.toml`
   (new): the hardware (`sys/arch/amd64/`, `sys/arch/arm64/`, `sys/machine/`), the boot code
   (`sys/stand/`, `sys/lib/libsa/`, a separate binary), the host test harness
   (`sys/arch/host/`) and a runtime of primitives (section 4). Every `unsafe` there has a
   `// SAFETY:` (clippy's `undocumented_unsafe_blocks` is already denied workspace-wide,
   `Cargo.toml:42`). Each core row has a budget, and no budget goes up without the user.
3. **Legacy only shrinks.** What lies outside the core and is not yet `forbid` is legacy
   (below). Its total never rises, an LZ sync aside (section 8), and it reaches 0 at N9.

Why: "fewer and smaller `unsafe` blocks" (goal 1 of `docs/PHASE2.md`) has no end point. A
boundary the compiler checks has one, and every step towards it is a number from a tool.

What userland sees does not change. `forbid` and safe traits act at compile time only, and each
slice keeps the ABI edge and passes the smokes and `just diff-openbsd`.

### Why not literal zero

A kernel on real hardware must do what no type system can check: write device registers
(MMIO), build page tables, switch contexts, enter and leave interrupts, copy across the user
boundary (`copyin`, `copyout`) and hand out raw memory inside its allocator. Some code has to
state those invariants by hand. The goal keeps that code small, listed, argued and reviewed,
and keeps everything else out of it.

### The legacy layer

Legacy is a state, not a directory: a module outside the core that does not compile under
`forbid` yet. Today that is most of the kernel (section 2): the LZ modules not yet redesigned
that still hold `unsafe`. They keep it for now. A module leaves legacy when the sweep marks it
because it holds no `unsafe` (section 3), or when it is redesigned, which then means `forbid`
(section 5). It never goes back: the count of `forbid` modules never falls.

### Prior art

Others draw the same line:

- Asterinas calls its design a framekernel: a small framework, OSTD, holds the `unsafe`, and
  the services built on it forbid it.
- Tock compiles its capsules, the drivers and services, with `unsafe` forbidden; the kernel
  core beneath them holds the rest.
- Rust for Linux writes abstractions with `unsafe` over the C kernel's interfaces, so that
  drivers can be written in safe Rust.

## 2. Where the unsafe is today

`cargo xtask unsafe-report` at `72ea58e`, kernel column (test code is counted apart):

| Layer | `unsafe-report` rows | Total | Fate |
|---|---|---:|---|
| Core: hardware | `arch/amd64` 863, `arch/arm64` 943, `machine` 85 | 1,891 | stays, budgeted |
| Core: boot | `stand` 235, `lib/libsa` 84 | 319 | stays; a separate binary |
| Core: test harness | `arch/host` 38 | 38 | stays; host only |
| Core: runtime candidates | the files of the next table | ~250 | stays, budgeted, shrinks |
| Kernel (everything else) | `net` 1,579, `kern` 1,146, `netinet` 553, `netinet6` 355, `sys` 330, `uvm` 215, the 22 `dev` rows 3,975 (`dev/pci` 737, `dev/ic` 664 and `dev/usb` 528 the largest), the seven file-system rows 635, `scsi` 149, `ddb` 15, `init` 11, `lib/libkern` 10, `crypto` 5, `conf` 2 | ~8,700 | 0 |
| Total | | 11,228 | |

The kernel rows hold 8,980 in all. The runtime candidates sit inside `kern`, `sys` and
`lib/libkern`; their row is a grep of `unsafe` tokens in the candidates' code zones, not an
`unsafe-report` count, and the kernel layer is what is left, about 8,700.

Runtime candidates (`unsafe` tokens in each file's code zone):

| Primitive | Files |
|---|---|
| Statics | `sys/lib/libkern/staticcell.rs` 11 |
| Locks and reference counts | `sys/sys/mutex.rs` 2, `sys/kern/kern_lock.rs` 8, `sys/sys/rwlock.rs` 2, `sys/kern/kern_rwlock.rs` 2, `sys/sys/mplock.rs` 0, `sys/sys/refcnt.rs` 1 |
| Per-CPU data and SMR | `sys/sys/percpu.rs` 7, `sys/kern/subr_percpu.rs` 17, `sys/sys/smr.rs` 8, `sys/kern/kern_smr.rs` 6 |
| Allocators | `sys/sys/pool.rs` 1, `sys/kern/subr_pool.rs` 58, `sys/sys/malloc.rs` 3, `sys/kern/kern_malloc.rs` 14 |
| Lists and trees | `sys/sys/queue.rs` 49 (6 blocks, 38 `unsafe fn`, the trait and its impl), `sys/sys/tree.rs` 23, `sys/kern/subr_tree.rs` 15 |
| Copy, print, time | `sys/kern/kern_subr.rs` 6 (`uiomove`), `sys/kern/subr_prf.rs` 7, `sys/sys/systm.rs` 1, `sys/kern/kern_tc.rs` 7 |

The atomics live in `sys/machine/atomic.rs`, already inside the hardware core.

**Shapes to replace**, kernel-wide:

| Shape | Count | Target (section 4) |
|---|---:|---|
| `StaticCell<T>` uses | 626 in 117 files | init-once cell, `Mutex<T>` statics, `PerCpu<T>` |
| `unsafe impl Send` and `unsafe impl Sync` | 325 | structs made of `Send` and `Sync` parts |
| `Cell<*const T>` and `Cell<*mut T>` | 674 (0 left in queue.rs and tree.rs) | state owned by its lock, owners' collections |
| `UnsafeCell` | 74 | `Mutex<T>`, `Rwlock<T>` |

**The primitives are C-shaped.** A lock holds no data (`mtx_enter(&Mutex)`,
`rw_enter(&Rwlock, flags)`), so what it guards sits elsewhere, in `Cell`s. `pool_get` and
`malloc` return `Option<NonNull<u8>>`: raw memory that the caller types and frees by hand.

**Already safe at the signature**, with the `unsafe` inside the core: `copyin`, `copyout` and
`copyinstr` over slices (`sys/machine/copy.rs:105`), `uiomove(&mut [u8], &mut Uio)`
(`sys/kern/kern_subr.rs:93`), `bus_space_read_4` and `bus_space_write_4`
(`sys/machine/bus.rs:195`, `sys/machine/bus.rs:210`).

**Day-one baseline.** Of the 1,191 `.rs` files under `sys/` (host, stand and `mod.rs`
excluded), 423 hold no `unsafe` token anywhere, tests included (433 with the tests left out).
One mechanical sweep can mark them `forbid`: about 35% of the files on day one, less the few
that section 3 says must wait. For scale, the hardware core is about 9% of the lines:
`arch/amd64` 40,372, `arch/arm64` 44,715 and `machine` 5,551 of the 1,002,358 under `sys/`,
tests included.

## 3. The mechanism

The attribute goes on the `mod` declaration in the parent, never inside the module's own file:

```rust
// sys/kern/mod.rs, line 89; today the line reads `pub mod sys_pipe;`
#[forbid(unsafe_code)]
pub mod sys_pipe;
```

**Rust note:** a lint level says what the compiler does when a lint fires. `warn` prints a
warning, `deny` makes it an error, and `forbid` makes it an error that no `#[allow]` further in
can lift.

A compile experiment (`cargo check`, rustc 1.98.1, a small crate with one `forbid` module)
verified the two facts the design rests on.

**Fact 1: `forbid` on the declaration reaches the out-of-line file.**
`#[forbid(unsafe_code)] pub mod svc;` in the crate root made a plain `unsafe {}` in `svc/mod.rs`
an error ("usage of an `unsafe` block"). So the ratchet lives in the `mod.rs` files and the
crate roots, which `lineage.toml` does not track (`.claude/rules/lineage.md`); the layout rule
already keeps every `mod` declaration there (`.claude/rules/rust-kernel.md`). An `inherited`
module can be `forbid` and stay byte-identical to LZ's. Why there: an inner
`#![forbid(unsafe_code)]` would change the module's own file and break that identity. The
attribute also covers the module's inline `mod tests`, so the 10 files whose only `unsafe` is
in their tests wait until those tests lose it.

**Fact 2: a macro's `unsafe impl` is refused.** A macro of the same crate that expands to
`unsafe impl Trait` is an error inside a `forbid` module ("implementation of an `unsafe` trait
... in this macro invocation"). `queue_adapter!` expands to `unsafe impl Adapter`
(`sys/sys/queue.rs:160`), and `tree_adapter!` expands to `queue_adapter!`
(`sys/sys/tree.rs:137`): 267 and 37 sites in 133 and 18 files. An `unsafe impl` is the
implementer's promise that a contract the compiler cannot check holds (here: `OFFSET` is the
offset of the field that `entry` returns), and `forbid` refuses that promise wherever it is
written. Every `unsafe impl Send` and `unsafe impl Sync` (325 in `sys/`) is refused the same
way.

What follows from fact 2:

- **`Adapter` becomes a safe trait.** `Adapter` (`sys/sys/queue.rs:1587`) drops its `unsafe`.
  The macro already computes `OFFSET` with `core::mem::offset_of!` (`sys/sys/queue.rs:163`) and
  writes `entry` as `&elem.field`, so the two agree by construction. What changes is who vouches
  for that: today the `unsafe impl` does; after the change the macro alone does, because
  `cargo xtask lz check` refuses an `Adapter` impl written by hand (none exists today). The
  core's `SAFETY:` comments that rest on the contract (`sys/sys/tree.rs:616`, "upheld by
  `queue_adapter!`") then cite that check. `TreeAdapter` (`sys/sys/tree.rs:980`) is a safe trait
  already; only the `queue_adapter!` it expands made it fail. The files that invoke the macros
  do not change. Why: with the trait left `unsafe`, every module that declares a list stays out
  of `forbid` until its own slice.
- **The proof moves from the compiler to a tool.** A hand-written `impl Adapter` would compile,
  and `just ci`, which runs `lz check`, fails on it. This is the one place where the proposal
  trades a compiler guarantee for a CI check; every review of a core change should know it.
- **`unsafe impl Send` and `Sync` are not swept.** Such a struct must become `Send` or `Sync`
  from its parts (the candidate row of `docs/IDIOMS.md`), usually because its mutable state
  moves into a core type: a `Mutex<T>`, an atomic. That happens owner by owner, in the slices
  (section 6).
- **The compiler judges the sweep.** The sweep adds the attribute to the 423 candidates and
  keeps the declarations that compile. A file with no `unsafe` token is still refused when it
  invokes a macro that expands one: the adapters, or `byte_view!` (`sys/msdosfs/bpb.rs:212`).
  Such a file waits for its macro's fix.

**Rust note:** `core::mem::offset_of!(Type, field)` is a constant the compiler computes, the
byte offset of a field inside its struct. It is stable and needs no `unsafe`; only
dereferencing an element pointer computed from it does, and that stays in the core
(`docs/IDIOMS.md`, the `container_of` row).

## 4. The core

### The initial list

`unsafe-core.toml` (new, at the root) holds the paths and globs that may contain `unsafe`.
`cargo xtask unsafe-report` and `cargo xtask lz check` read it.

| Layer | Paths | Why it is core |
|---|---|---|
| Hardware | `sys/arch/amd64/**`, `sys/arch/arm64/**`, `sys/machine/**` | MMIO, page tables, traps, context switch, `copyin`/`copyout`, atomics |
| Boot | `sys/stand/**`, `sys/lib/libsa/**` | the boot loaders: a separate binary that runs before the kernel |
| Test harness | `sys/arch/host/**` | the host test double; host only |
| Runtime | the files of the runtime table (section 2), and `sys/lib/libkern/explicit_bzero.rs` | the primitives every subsystem builds on; their `unsafe` pays for the safe API above them |

`explicit_bzero.rs` is not among the measured candidates, but it is `redesigned`
(`lineage.toml:4376`) and keeps the volatile store that N1 left (`docs/ROADMAP.md`, N1), so
"redesigned = `forbid`" (section 5) puts it here. The volatile `wipe` that `docs/STATUS.md`
lists as a crypto follow-up is the same kind of store and would join it. Two rows stay out, as
legacy like the rest: `conf` (2) and `init` (11, the freestanding init stand-in that
`unsafe-report` also reads).

Rules of the list:

- A path joins it only with the user's OK, in a commit of its own, like a budget raise. Why: the
  list is the boundary, and growing it is the one way around the goal.
- A core file may carry `forbid` too, and should once its `unsafe` is gone (`sys/sys/mplock.rs`
  has none today). Why: the list says where `unsafe` may live, not where it must.
- The runtime gets a budget row of its own, carved out of `kern`, `sys` and `lib/libkern`. Why:
  those rows then measure legacy alone, and the runtime's size is a number of its own.

### What it must expose

The runtime exists to give `forbid` modules a safe API. Its target shapes, each a row of
`docs/IDIOMS.md` in the commit that first uses it (the candidates there name most of them):

- **Locks that own their data.** `Mutex<T>` and `Rwlock<T>` with guards, replacing the C-shaped
  locks and the `Cell` fields beside them. A guard can sleep (`rwsleep`) and wake (`wakeup`).
- **Typed allocation.** `Pool<T>` handing out an owned `PoolBox<T>`; a typed `malloc` as
  `KBox<T>` and `KVec<T>`, fallible, behind the `alloc` feature.
- **Statics.** An init-once cell for tables written at boot, `Mutex<T>` statics, and `PerCpu<T>`
  behind the interrupt-level guard. They replace `StaticCell`, whose API stays frozen by the N1
  timing rule, so each use moves with the module that owns it.
- **Collections for owners.** Arena lists (a slab with a generational index: O(1), several lists
  per element, and a stale index is an error instead of undefined behaviour) and lists of
  `Arc`-owned elements. The typed-link intrusive lists stay inside the core, for its own use;
  queue.rs's 38 `unsafe fn` mutators are deleted once no module outside the core calls them
  (`queue_adapter!` alone appears in 133 files today).
- **The user boundary.** `copyin`, `copyout` and `uiomove` keep their slice signatures.
- **Devices (N7).** A typed `Mmio<Regs>` over `bus_space` and an owned `DmaBuf` over `bus_dma`.

**Rust note:** an `unsafe fn` declares a contract its caller must keep, so every call is written
inside an `unsafe { }` block, where the caller takes that duty on. A module that calls one of
queue.rs's mutators therefore holds `unsafe` blocks and cannot be `forbid`.

### Its rules

- Every `unsafe` block has a `// SAFETY:` soundness argument and every `unsafe fn` a `# Safety`
  section, both enforced (`undocumented_unsafe_blocks`, `Cargo.toml:42`; `missing_safety_doc`,
  `Cargo.toml:43`). No `static mut`.
- An agent that did not write the change reviews every core change (`reviewer`). Why: a mistake
  in the core is undefined behaviour for every module above it.
- Each core row has a budget in `unsafe-budget.toml`. Budgets ratchet down and rise only with the
  user, as today (`.claude/rules/unsafe-budget.md`).
- A primitive lands with its first user, never ahead of it. Why: `docs/IDIOMS.md` adds a row in
  the commit that first uses an idiom, and an API nobody calls is an untested design.
- Miri, an interpreter that runs tests and stops at undefined behaviour, could run the core's
  host tests. It needs a nightly toolchain to run, while the code stays on stable; the project
  forbids nightly features, not necessarily a nightly tool for tests. The user decides
  (decision 4).

## 5. The metric

`cargo xtask unsafe-report` splits the kernel into three groups:

- **core**: the files of `unsafe-core.toml`, per budget row;
- **forbid**: the modules whose declaration carries `#[forbid(unsafe_code)]`, read from the
  `mod.rs` files and the crate roots; they hold 0 by construction;
- **legacy**: everything else.

`--check`, in `just ci`, then enforces three things:

1. the number of `forbid` modules never falls;
2. the legacy total never rises (an LZ sync is the one exception, section 8);
3. each core row stays within its budget.

The floor of item 1 and the ceiling of item 2 live in `unsafe-budget.toml` beside the rows, and
`--write` moves them only in the good direction, as it already does the budgets. Beside them,
`legacy_baseline` and `baseline_date` record the legacy total on the day the plan was adopted
(8,748 on 2026-10-10): `/progress` measures its pace from them, and `--write` never moves them.
The
per-subsystem rows stay; outside the core their sum is the legacy total. `--write` also writes
the `Unsafe` lines of `docs/STATUS.md` and of `README.md` ("Lineage and safety"): core, legacy,
and `forbid` modules out of all modules.

Beside the gate, the report prints four shape counts per subsystem and per module: `StaticCell`
uses, `unsafe impl Send` and `Sync`, `Cell<*const T>` and `Cell<*mut T>`, `UnsafeCell`. They
gate nothing; the milestone exits name them where they matter (N2, N4). Why: a count of `Cell`s
says little alone, and a gate on it would reward moving them around.

**"Redesigned" means "compiles with `forbid`".** `cargo xtask lz check` refuses a `redesigned`
module outside the core whose declaration lacks the attribute, and an `Adapter` impl written by
hand (section 3). Why: the status then states something the compiler checked, not only that the
shape changed. An `inherited` module may be `forbid` as well (the sweep) and stays `inherited`.

**Where it shows.** `cargo xtask lz status --write` shows, per subsystem of the README table,
the modules outside the core and the `forbid` ones among them, and a line for the core (its
modules, its `unsafe`, the budget the rows leave it). `/progress` reads `cargo xtask
unsafe-report --json`, the one parser of the sources, and shows per milestone the `forbid`
modules out of the row's non-core modules, the legacy modules and legacy `unsafe` left (the
exit criterion's number: a row is complete when both are 0), its core modules, and an estimate
from the legacy `unsafe` removed per active day since the baseline. Why: the goal's progress
then sits where the redesign's progress is already read, and the core, which keeps its `unsafe`
by design, does not dilute it.

## 6. The order

**Runtime first, then vertical slices per owner.** A slice takes one owner, the module that owns
a data structure (`sys/kern/sys_pipe.rs` for a pipe), with the header types it owns, and
redesigns them to `forbid` in one go, once the core primitives it needs exist. Why: a type's
fields and their users change together; a horizontal pass ("every `Cell` out of `sys/sys/`")
would rewrite modules that later milestones own, and pay for it again in every LZ sync.

### Before pipe: the two type-erased slots (decision 9, 2026-10-16)

`File.f_data: Cell<*mut c_void>` (`sys/sys/file.rs:169`, 16 files) and `Knote.kn_hook:
Cell<*mut c_void>` (`sys/sys/event.rs:446`, 18 files) are the slots every file type and every
kqueue filter casts from. A slice that keeps them needs a seam per slot (a tag-checked cast in
`kern_descrip`, an index smuggled into `kn_hook`): `unsafe` moved, not removed. Both are under
the 50-module timing rule, so they are typed first, as two slices of their own: `f_data` becomes
an owned `enum FileData` dropped by `fdrop` (the direction of a pipe end rides in its variant);
`kn_hook` a typed hook. Then pipe(2) needs no seam into `file` or `kern_event`.

### The first slice: pipe(2), recommended

- Modules: `sys/kern/sys_pipe.rs` (1,659 lines, 21 `unsafe`) and `sys/sys/pipe.rs` (233 lines,
  14 `Cell`, 5 of them raw-pointer `Cell`s), declared at `sys/kern/mod.rs:89` and
  `sys/sys/mod.rs:76`.
- Sources: one C file with its header, `reference/openbsd-src/sys/kern/sys_pipe.c` and
  `reference/openbsd-src/sys/sys/pipe.h`; in LZ, `reference/emibsd-lz/sys/kern/sys_pipe.rs` and
  `reference/emibsd-lz/sys/sys/pipe.rs`.
- Callers: 4 files.
- Imports: `kern_descrip`, `kern_event`, `kern_rwlock`, `kern_sig`, `kern_subr::uiomove`,
  `kern_synch`, `kern_tc`, `subr_pool`, `machine::copy`, `uvm_km`, `file` and `filedesc`.
- Coverage: shell pipelines in the `--send` lines of 12 smokes (ahci, ehci, ext2fs, fuse, https,
  nvme, pf, siop, smmu, softraid, tpm, uaudio) and in every installer run. No `diff-openbsd`
  step names pipes, so the slice adds one.

Why pipe: it is small, comes from one C file, has four callers, every smoke with a pipeline
exercises it, and it uses much of the runtime at once (a sleeping lock, a pool, `uiomove`,
kqueue, `SIGIO`). It proves the core's API on real code.

What it needs first, six items:

1. `Rwlock<PipeState>`: the pipe's state owned by its lock;
2. `Pool<Pipe>`: pipes from a typed pool;
3. an owned buffer: `km_alloc` returning owned pages instead of an address;
4. a safe seam over the legacy `kern_event` (`klist`, `knote`);
5. a safe seam over the legacy `kern_sig` (`sigio`);
6. `rwsleep` and `wakeup` on the `Rwlock<T>` guard.

A seam is a safe function that a legacy module exports, with the `unsafe` inside it, so that a
`forbid` caller can use the module before that module's own slice. The ratchet still holds: the
slice must take out more legacy `unsafe` than its seams put in.

The slice's exit is part of N2's (section 7): both files `forbid`, no raw-pointer `Cell` left in
them, every smoke and `just diff-openbsd` equal with the new pipe step.

### The warm-up alternative: msgbuf

`sys/kern/subr_log.rs` (695 lines, 12 `unsafe`, 5 `StaticCell`) and `sys/sys/msgbuf.rs` (7
`Cell`): 9 callers, `dmesg` in `smoke-virtio` and `smoke-wscons`, and imports of the sockets
(`sosend`) and the kernel lock. It is smaller, and it exercises the statics (the init-once cell,
`Mutex<T>` statics) more than the sleeping lock. Its coverage is two smokes, and it needs a seam
into the socket layer. Pipe stays the recommendation for its broader coverage; msgbuf suits a
smaller first step (decision 3).

Other owners measured, for the order after the first slice:

| Owner | Lines | `unsafe` | Callers |
|---|---:|---:|---:|
| `kern_timeout` | 1,198 | 35 | 93 |
| `kern_event` | 3,199 | 76 | 42 |
| `kern_task` | 714 | 19 | 46 |
| `kern_sensors` | 467 | 16 | 11 |

`kern_timeout` has too many callers for a first slice; a change to its API would also fall
under the timing rule of `.claude/rules/re-engineering.md` (more than 50 modules).

## 7. Milestones

This replaces rows N2 to N8 of the table in `docs/ROADMAP.md` and adds N9. N0 and N1 stay as
they are (met), and so do the order (leaves first, the security subsystems last) and the rolling
LZ sync. Same columns; `<n>` is filled from the tools when a row starts (`docs/ROADMAP.md`,
"Adding or changing a milestone"). A row's scope is resolved as `/progress` resolves it, the most
specific path winning, so a row's numbers count only its own modules: `sys/kern/vfs_*` belongs
to N5, and the files N8 names belong to N8, not to N4, N6 or N7.

| Milestone | Scope | Exit criterion |
|---|---|---|
| **N2 Core runtime** (under way; was "Core structures") | First step, done: `sys/sys/queue.rs`, `sys/sys/tree.rs` and `sys/kern/subr_tree.rs` with typed links. Then: the metric in `just ci` (`unsafe-core.toml`, the core, forbid and legacy groups, the ratchet); `Adapter` a safe trait; the `forbid` sweep (~423 modules); the runtime's safe API, each piece with its first user: `Mutex<T>` (`sys/sys/mutex.rs`, `sys/kern/kern_lock.rs`), `Rwlock<T>` (`sys/sys/rwlock.rs`, `sys/kern/kern_rwlock.rs`), `Pool<T>` (`sys/sys/pool.rs`, `sys/kern/subr_pool.rs`) and the init-once static; the verdicts of the 46 ABI-constant headers; the first slice, pipe(2): `sys/kern/sys_pipe.rs`, `sys/sys/pipe.rs` | `cargo xtask unsafe-report` prints the core, forbid and legacy groups, and `--check` passes in `just ci`; `forbid` modules >= `<n>`; `sys/kern/sys_pipe.rs` and `sys/sys/pipe.rs` declared `forbid`, and `grep -c 'Cell<\*'` prints 0 for each; every smoke and `just diff-openbsd` equal, with the new pipe step |
| **N3 Memory** | `sys/uvm/` in owner slices (amap, aobj, map, the pagers; `uvm_fault` last, per `security-review.md`); the pmap interface in the core | the `uvm` row holds 0 `unsafe` outside the core |
| **N4 Processes and scheduling** | `sys/kern/` by owner (timeout, task, event and kqueue, signals, proc and the scheduler), each with its header types | the `kern` row holds 0 `unsafe` outside the core, apart from the modules of N5 and N8; `StaticCell` uses in `kern`: 0 |
| **N5 VFS and file systems** | `sys/kern/vfs_*`, `sys/ufs/` first, then `sys/isofs/`, `sys/msdosfs/`, `sys/nfs/`, `sys/ntfs/`, `sys/tmpfs/`, `sys/miscfs/` | the `ufs`, `isofs`, `msdosfs`, `nfs`, `ntfs`, `tmpfs` and `miscfs` rows hold 0 `unsafe` outside the core; every `sys/kern/vfs_*` module `forbid` |
| **N6 Network stack** | `sys/net/`, `sys/netinet/`, `sys/netinet6/`, except the files of N8 | the `net`, `netinet` and `netinet6` rows hold 0 `unsafe` outside the core, apart from the modules of N8 |
| **N7 Devices** | `sys/dev/` and the bus layers (`sys/scsi/`), one bus directory at a time; a typed `Mmio<Regs>` and an owned `DmaBuf` in the core; drivers `forbid`; `sys/dev/rnd.rs` last (`security-review.md`) | every `dev` row and the `scsi` row hold 0 `unsafe` outside the core, apart from `sys/dev/softraid_crypto.rs` (N8) |
| **N8 Security subsystems** | Under `security-review.md`: `sys/net/pf*` (pf and pfkeyv2), `sys/net/if_pf*`, `sys/net/if_wg.rs`, `sys/net/wg_*`, `sys/netinet/ip_ipsp.rs`, `sys/netinet/ip_esp.rs`, `sys/netinet/ip_ah.rs`, `sys/netinet/ip_ipcomp.rs`, `sys/netinet/ipsec_*`, `sys/dev/softraid_crypto.rs`, `sys/kern/kern_pledge.rs`, `sys/kern/kern_unveil.rs`, `sys/kern/exec_*`, and the crypto framework (N1 did the primitives) | every file of the scope `forbid`; `Security-Review:` on every commit; `cargo xtask lz drift --security` empty |
| **N9 Close** | What no row claims (`sys/ddb/`, `sys/conf/`, the init stand-in) and the legacy left; the boundary made final: the core in a crate of its own, with `#![forbid(unsafe_code)]` at the root of the crate that uses it, or kept as the module list with every declaration outside it `forbid` (decision 5); Miri on the core's host tests if adopted (decision 4) | `cargo xtask unsafe-report`: legacy 0; with a core crate, the logic crate's root carries `#![forbid(unsafe_code)]` and `just build` passes; the core's size recorded in `docs/STATUS.md`; Miri green if adopted |

Every exit also requires:

- each module of the scope is `redesigned` (and so `forbid`), listed in the core, or `reviewed`:
  a per-file verdict in `lineage.toml`, with its reason, for a module that needs no redesign and
  is `forbid` as it stands (the 46 ABI-constant headers of `sys/sys/` get `forbid` and
  `just test-ref` checks of their constants);
- `just ci`, `just ci-full` and `just diff-openbsd` equal, with the same expected list;
- `cargo xtask lz drift --security` empty, and the numbers in the closing commit.

## 8. LZ

- **A `forbid` module takes an LZ change as `reimplemented` when it must.** An `inherited`
  module that is `forbid` is still byte-identical to LZ's, so LZ's changes to it still arrive by
  cherry-pick. A change that adds `unsafe` does not compile there, and it is re-implemented in
  safe code. The compiler decides: a cherry-pick that compiles is still a cherry-pick. Why: the
  `forbid` count never falls, so a sync cannot buy LZ's shape back with an `unsafe` line.
- **New LZ files arrive `inherited`, as legacy.** Their `unsafe` raises the legacy total, the
  one way it may rise: the sync re-baselines the ceiling in a commit of its own, with the user's
  OK, as the M16 sync's budget rows were. Why: LZ's new code is not a regression of this tree,
  but its size must still be seen and approved.
- **Drivers become `forbid` in N7**, with the typed bus API. Until then LZ's driver commits keep
  landing by cherry-pick.
- **`reimplemented` growing is expected now**, no longer the signal to reorder the roadmap that
  `.claude/rules/lz-sync.md` makes it today. The new signal is a legacy total that stops falling
  from one sync to the next.
- **Security fixes keep their rule** (`.claude/rules/lz-sync.md`): applied, re-implemented in a
  `forbid` module if need be, or `covered-by-redesign` with the test or type named; never
  skipped silently.
- **The cost moves to the `mod.rs` files.** An LZ commit that adds a module next to a `forbid`
  declaration conflicts there, and the resolution is one line (`cherry-pick-conflicts`). The
  adapter change touches only `sys/sys/queue.rs` and `sys/sys/tree.rs`, both redesigned already,
  so it costs nothing in the files that invoke the macros.

## 9. Commits after the OK

One topic each, in this order. Every commit builds, every group ends with `just ci` green, and
the trailers follow `.claude/rules/git-commits.md`.

1. **`docs: adopt zero unsafe outside a listed core`.** This file in force (its status line);
   in `docs/PHASE2.md` the decision row below and goal 1 reworded; in `docs/ROADMAP.md` the
   table of section 7; the status lines of `docs/STATUS.md` and `README.md` (N2 "Core runtime").
2. **`rules: forbid unsafe code outside the core`.** `unsafe-budget.md` (the core list, the
   ratchet, `forbid`), `re-engineering.md` (redesigned = `forbid`), `lineage.md` (`forbid` in
   `mod.rs`; the `lz check` rules), `lz-sync.md` (a `forbid` module may need `reimplemented`;
   the legacy re-baseline), `rust-kernel.md` (no `unsafe` outside the core; the adapter rule),
   `subagents.md` (what a launch prompt says about the core), and the one line this adds to the
   non-negotiables of `CLAUDE.md`. Files under `.claude/` are edited with Edit/Write (the user's
   rule).
3. **`xtask: split unsafe-report into core, forbid and legacy`.** `unsafe-core.toml`; the split,
   the `forbid` count, the floor and the ceiling, the shape counts; `lz check` refuses a
   hand-written `Adapter` impl; the `forbid` columns of `lz status --write` and of
   `.claude/skills/progress/scripts/progress.py`.
4. **`sys:` the adapters, then the sweep.** First `Adapter` a safe trait (`sys/sys/queue.rs`,
   `sys/sys/tree.rs`; the files that invoke the macros unchanged). Then the sweep over the
   `mod.rs` files and crate roots, with `unsafe-report --write` recording the floor. Last, an
   `xtask:` commit turns on the `lz check` rule "redesigned = `forbid`", which passes only once
   the sweep has marked the redesigned modules of N1.
5. **The pipe slice.** A `redesigner` (opus) with the six items of section 6, then a `reviewer`,
   then the `integrator` and `just ci`: one commit per primitive with its `docs/IDIOMS.md` row,
   the seams, the pipe redesign (`kern:`) and the `diff-openbsd` pipe step (`xtask:`), each with
   its `LZ:` and `Unsafe:` trailers.

The decision row proposed for `docs/PHASE2.md`:

| Date | Decision |
|---|---|
| `<date of the OK>` | Zero `unsafe` outside a listed core (`docs/ZERO_UNSAFE.md`): every module outside `unsafe-core.toml` compiles under `#[forbid(unsafe_code)]`, set on its `mod` declaration; the core (hardware, boot, the host harness, a runtime of primitives) is budgeted and reviewed; the legacy total only falls, an LZ sync aside; "redesigned" means "compiles with `forbid`"; N2 becomes "Core runtime" and N9 "Close" is added. |

Goal 1 reworded: "**Safety.** Zero `unsafe` outside a listed core: every module outside
`unsafe-core.toml` compiles under `#[forbid(unsafe_code)]`, and every `unsafe` inside the core is
budgeted, argued and reviewed (`docs/ZERO_UNSAFE.md`, `.claude/rules/unsafe-budget.md`)."

## 10. Decisions

Taken by the user on 2026-10-10 ("ta bien, demosle, igual esto es iterativo. lz check porfa"):

1. **The goal: adopted** as stated in section 1.
2. **The initial core list: as listed** in section 4, `explicit_bzero.rs` included, `conf` and
   `init` left as legacy. A later addition is a commit of its own, with the user's OK.
3. **The first slice: pipe(2)** (section 6); since decision 9, after the two slot slices.
4. **Miri: deferred.** Decided at N9; the plan does not depend on it.
5. **The core at N9, crate or list: deferred** to N9. Nothing before it depends on it.
6. **LZ: `reimplemented` is the normal method for `forbid` modules** (section 8).
7. **The adapter: a safe trait checked by `lz check`.** `Adapter` (`sys/sys/queue.rs`) becomes a
   safe trait whose impls only `queue_adapter!` and `tree_adapter!` write; `cargo xtask lz check`
   refuses a hand-written impl. The alternative (back links that point at the element or the
   head, so no offset and no `container_of`) was set aside for now; it stays open if the
   tool-enforced boundary proves weak.

Taken by the user on 2026-10-16 ("vamos por la recta, lo barato me sale caro siempre"):

8. **Names.** The owning primitives are `Rwlocked<T>`, `TypedPool<T>` with `PoolArc<T>`, and
   later `Mutexed<T>`, beside the C-shaped `Rwlock`, `Pool` and `Mutex` (72, 73 and 129 files:
   the timing rule). The renames (`RawRwlock`, ... and the plain names for the owning types) are
   N9's.
9. **No hidden seams.** The two type-erased slots are typed first (section 6, "Before pipe"); pipe
   follows with no seam into `file` or `kern_event`. The one seam left, an owned pageable
   allocation (`KmBuf`) in `sys/uvm/uvm_km.rs` until N3 owns uvm, is recorded as such: a
   `[[seam]]` row of `lineage.toml` (`module`, `for`, `paid_by`) that `lz status` and
   `unsafe-report` print as their own group, never as `adapted`.
10. **No `&'static` from a pool** (docs/IDIOMS.md): `PoolArc<T>` is an owned handle; a raw form
    exists only inside the core.
11. **The pipe's `PIPE_LOCK`** stays a bit of `pipe_state` (userland sees the word whole through
    `kinfo_file`); the buffer is an owned `KmBuf` moved out of the lock while `uiomove` runs, so
    no I/O-lock primitive is needed.
12. **`pipe_state` and `peer_alive`** are atomics written only under the pair's write guard, so
    `kern_sysctl` keeps the C's unlocked read; a race the C has, kept on purpose and named here.
13. **sigio and klist need no primitive** for pipe: their APIs are safe. After its slice the
    pipe's klists lock through `KlistLock::Kernel` semantics replaced by the pair's guard: every
    call is the `_locked` form under it (today they use the rwlock klistops, as the C). Five
    items became three.
14. **`FileData`** (2026-10-16, from the `f_data` survey): `File.f_data` is an `OnceCell<enum
    FileData { None, Vnode, Socket, Pipe, Kqueue }>` whose payloads are the handles the owners
    hold today; `fo_close` receives it by value from `fdrop`, the only taker (one `unsafe` there,
    on the invariant `pool_put` already rests on). `f_cred` is an owned counted `CredRef`
    (`kern_prot.rs`, the second `[[seam]]`, paid by N4). No cast is moved: the five are deleted.
15. **The knote's hook** is `OnceCell<Hook>`, `Borrowed(&'static dyn Any)` or `Owned(Box<dyn
    Any>)` (the timer), read by `downcast_ref`: typed at run time, no `unsafe` per filter. The
    klist's lock is `enum KlistLock { Kernel, Mutex(&'static Mutex), Rwlock(&'static Rwlock) }`;
    `Klistops` and the three `unsafe fn klist_init*` go. A filter with no `&'static` object in
    hand stops the work; no cast is added for it.
16. **`[[seam]]` is tooling first**: `lz check`, `lz status` and `unsafe-report` learn the table
    before the first seam lands.

Taken by the planning session on 2026-10-16 under the user's standing authority of that day
("autorizo dentro de reglas"), after the slots redesigner stopped before its series 2:

17. **`AbiPod` is a safe trait checked by `lz check`**, like `Adapter` (decision 7): the only
    impls are written by `abi_pod!(Type { field: Type, ... })` in the core (`sys/machine/copy.rs`),
    which checks every field's exact type at compile time against the primitives the core
    declares `AbiPod`; `lz check` refuses a hand-written `impl` (`unsafe` or not). First user:
    `Kevent` (`sys/sys/event.rs`), so that file can be `forbid`; the other ~200 impls convert
    as their owners' slices come. Rejected: moving the `unsafe impl` into `kern_event.rs` (an
    `unsafe` moved, not removed).
18. **`kn_kq` and `kqs_kq`** become `Cell<Option<&'static Kqueue>>`, like `kn_fp`; the callers
    already hold the `&'static` (`fp.kqueue()`, `p.kq()`). No new cast. A hook site that has no
    `&'static` object in hand takes it from the same owner lookup the other device filters use
    (`device_lookup`-style, the softc the autoconf tree owns); where the owner holds only a raw
    pointer, the site stops and reports: none is cast here.
19. **The timer's hook owns its object through `MBox<T>`**, an owned typed `malloc` in the core
    (`sys/sys/malloc.rs`, `sys/kern/kern_malloc.rs`, already core paths): `MBox::new(ty, flags,
    value) -> Option<MBox<T>>` with the C's `M_KEVENT, M_WAITOK`, so `vmstat -m` and the wait
    behaviour stay the C's; `Drop` frees. `Hook` is `Borrowed(&'static dyn Any) |
    Timer(MBox<FiltTimer>)`: the one owned hook is kqueue's own, named, not generic. Rejected:
    `Box` (the global allocator is `M_TEMP, M_NOWAIT`: a different `vmstat` row and a panic
    where the C sleeps).

Taken by the user on 2026-10-16, late ("OK te voy a permitir eso por ahora"), after rejecting
every design that kept `unsafe` inside the kernel:

20. **The kernel is a crate with `#![forbid(unsafe_code)]` at its root: zero `unsafe`, enforced
    by the compiler, with no exception inside it.** Below it, a HAL crate, new code, small, holds
    the only `unsafe` the system has (locks and the allocator: `UnsafeCell`, bytes to `T`; page
    tables, traps, context switch, MMIO, DMA), each with a soundness proof, Miri on its host
    tests: the kernel's `std`. Above it, an ABI compatibility crate keeps OpenBSD's userland
    interface byte-identical (the syscall table, the `#[repr(C)]` structs, ioctl and sysctl
    encodings) and frees the kernel from the C's shape. The LZ-derived tree is not a layer: it
    is what the kernel crate replaces, subsystem by subsystem, until it is empty; while it runs,
    it calls the new crates, never the reverse except through the HAL's registered bridges.
    This supersedes sections 4 (the core list) and 7 (the milestones): a v2 of this document
    rewrites them with the crate split, the migration mechanism and the new N2/N3. The
    Asterinas framekernel (OS services in safe Rust over an `unsafe` OSTD) is the precedent.
    The user's words: zero `unsafe` in the kernel, "por ahora" the HAL may have it.

21. **The ABI compatibility crate holds no business logic** (the user, 2026-10-16: "solo debe
    orquestar con la basura unsafe llamadas al verdadero núcleo safe"). It only translates: the
    syscall number and its register arguments to a typed call into the kernel crate, the
    userland bytes of a `#[repr(C)]` struct to the kernel's own type and back, an errno from the
    kernel's `Result`. No permission check, no semantic validation, no state of its own, no
    branch on a value's meaning: every decision OpenBSD's behaviour requires lives in the
    kernel crate, where it is safe code. Mechanical criterion, checked by `cargo xtask`: a
    function of the ABI crate contains no `if`/`match` on anything but the syscall number, the
    conversion's own result (`Ok`/`Err`) and the layout of the bytes; it calls exactly one
    kernel entry point; its size stays under a fixed line count per syscall. A violation fails
    `just check`.

22. **The ABI is a loadable module, from N2 on** (the user, 2026-10-16: the compatibility layer
    out of the kernel, "una biblioteca dinámica"). `abi-openbsd.ko` is a separate ELF file on the
    boot medium: the boot loader reads it, `hal`'s loader relocates it before `init(8)` and the
    module registers its syscall handler in `hal`'s registry and receives `KernelApi`, the vtable
    of safe functions the kernel exports: the only face of the kernel the module sees. The
    kernel does not name the ABI; the ABI does not name `bsd`. The syscall trap in `hal` calls
    the registered handler. Costs, stated: an ELF loader in `hal` (about 30 `unsafe`, the
    parsing in safe code), `KernelApi` as a vtable, the boot loaders and the installer reading
    two files, one smoke for the load. The layers are, top down: userland (0), the module (1,
    0 `unsafe`, no logic), `kernel` (2, 0 `unsafe`), `hal` (3, the only `unsafe`, ceiling
    2,150), hardware (4); `bsd` beside them until N9, calling down and registering up. There is
    no "static first, dynamic later": the module exists from N2's close.

23. **Develop first, compile later** (the user, 2026-10-17: "desarrolla primero compila
    después"; "necesito que termine en 2 semanas"). No per-commit build, clippy or test gate:
    the agents write a milestone's slices without compiling; `just check`, the smokes, `ci-full`
    and `diff-openbsd` run once, at the milestone close, where the errors are fixed in a
    compile-and-fix pass before the close. The pre-commit hook keeps only what takes seconds
    (`cargo fmt --check`, `cargo xtask lz check`). The planning session said the risk: errors
    found at the close compound across the milestone's commits; the user took it. The two-week
    target is N2 and N3 closed, with parallel redesigners from N2.3 on (section 0.8).

24. **The N2 close** (the user, 2026-10-17): integrate the agents' branches into `main`, then
    compile without QEMU until green (`just build` on both archs, then `just check`), fixing
    the integration errors between `hal`, `kernel`, `abi` and `bsd`; no `just ci-full`; one
    `just ci` and one `just diff-openbsd`, both rc=0, mark N2 closed and push. `ci-full` is
    recorded as pending in `docs/STATUS.md` and runs once at the N3 close for both milestones.
