# The ABI crate (decision 47, the user's rule of 2026-11-06; decision 52, 2026-11-26; decision 56)

`abi` is linked into the kernel image but stays a separate crate whose only job is
translation (decisions 21 and 25). Decision 52 (the user, 2026-11-26, option A): the
chain is userland -> `abi` -> `kernel` -> `hal`; `abi` calls the kernel directly
through `kernel::api`, and the `KernelApi` table in `hal::api` (needed while `abi` was a
loadable module, decision 22) goes. A trap from userland enters `hal`'s vectors, which save
the registers and call `entry` (decisions 56 and 58: one symbol, resolved at link time);
`entry` calls `abi`'s per-arch handler directly with the trapping thread's registers as a
`hal::syscall::UserRegs`; nothing is registered, and no call goes upward for a system call.
The handler (`abi::arch::current::svc`) is `abi`'s one entry function: `entry`'s closed
`match` names it and nothing else of `abi` (the user's lock of 2026-11-27,
`cargo xtask entry-check`).

- `abi` depends on `kernel`, `hal` and `sysdefs` in Cargo; never on `bsd`. `kernel` and `hal`
  never name `abi`.
- Its only door to the kernel is `kernel::api`: one function per system call (R3).
  From `kernel` it names nothing else.
- Decision 54 (2026-11-26, the user): the ABI edge's constants and `#[repr(C)]` structures
  are the leaf crate `sysdefs` (`sys/lib/sysdefs/`, data only: no `unsafe`, no state, no
  logic), which `abi` names freely and R6 does not read; `hal::sys` no longer exists.
- From `hal` it names only passive items (abi-check R6):
  - from `hal::syscall`, only the types `Register`, `Rval`, `SysArgs`, `SysResult`,
    `UserRegs`, `Gpr`, `ChildReturn` and `ENOSYS`;
  - decision 53 (2026-11-26, the user, with N7.abi2): also `hal::syscall`'s `UserRegs`
    (a view of the user trap frame, valid for the handler's call only, with exactly three
    methods: read a set of registers, set the return register with the error flag, and
    `restart()`), `Gpr` (the machine's registers as a per-arch enum with their names: the
    convention reads as the domain, `Gpr::X8`, and a wrong register does not compile) and,
    `ChildReturn`, the forked child's first return as data (which register, which value,
    the error flag), a constant `abi` defines and never applies itself. abi owns OpenBSD's
    per-arch system call convention
    (`sys/abi/arch/<arch>.rs`: the number register, the argument registers, the return
    register, the carry bit, when to restart, the child's first return) and calls
    `kernel::api::syscall_return` (`mi_syscall_return`) after writing the result; `hal`
    keeps only machine mechanics (the vector, the frame, the counters, the interrupt
    enable, the barrier skip and the pc arithmetic inside `restart()`) and interprets
    nothing userland sends. abi never writes a pc. `abi` holds no list of the CPU
    identification registers a program may read: the user's lock (2026-11-27) amended
    decision 56's U7, and the list and the values are the kernel's (`kernel::trap`), the
    decode of the trapped instruction and the register's write `hal`'s.
  It never names a `hal` function with side effects (`dispatch`, `hal::cpu::*`,
  `hal::user::*`, the kcopy tokens, ...).
- No logic, no state, no loops (R1, R2, R4, R5), as before.
- Widening the door (another `kernel` module, or more of `hal`) is the user's decision,
  in its own commit, with the reason.
- A `pub` item added to `kernel::api` widens the door without touching `sys/abi/`: a
  review of such a change checks what it exposes to `abi`.
