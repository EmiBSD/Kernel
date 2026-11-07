# The ABI crate (decision 47, the user's rule of 2026-11-06)

`abi` is linked into the kernel image but stays a separate crate whose only job is
translation (decisions 21 and 25). The isolation the loadable `abi_openbsd.ko` gave
for free is kept by rule:

- `abi` depends on `hal` alone in Cargo; never on `kernel` or `bsd`.
- Its only door to the kernel is `KernelApi`: one call per system call (R3).
- From `hal` it names only passive items (abi-check R6):
  - `hal::api` (`KernelApi`);
  - `hal::sys::*` (the ABI edge's constants);
  - from `hal::syscall`, only the types `HandlerInfo`, `Register`, `Rval`, `SysArgs`,
    `SysResult`, `SyscallFn` and `ENOSYS`.
  It never names a `hal` function with side effects (`dispatch`, `set_handler`,
  `hal::cpu::*`, `hal::user::*`, the kcopy tokens, ...).
- No logic, no state, no loops (R1, R2, R4, R5), as before.
- Widening R6's list (for example to `hal::user`, if `abi` ever copies a structure
  itself) is the user's decision, in its own commit, with the reason.
- A `pub use` added to `hal::api`, `hal::syscall` or `hal::sys` widens R6 without
  touching `sys/abi/`: a review of such a change checks what it exposes to `abi`.
