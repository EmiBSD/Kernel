# The ABI crate (decision 47, the user's rule of 2026-11-06; decision 52, 2026-11-26)

`abi` is linked into the kernel image but stays a separate crate whose only job is
translation (decisions 21 and 25). Decision 52 (the user, 2026-11-26, option A): the
chain is userland -> `abi` -> `kernel` -> `hal`; `abi` calls the kernel directly
through `kernel::api`, and the `KernelApi` table in `hal::api` (needed while `abi` was a
loadable module, decision 22) goes. The CPU's trap still enters in `hal` (the vector is
assembly and registers); `hal` saves the registers and hands them, uninterpreted, to the
dispatcher `abi` registers: the only upward call, like the panic handler.

- `abi` depends on `kernel` and `hal` in Cargo; never on `bsd`. `kernel` and `hal`
  never name `abi`.
- Its only door to the kernel is `kernel::api`: one function per system call (R3).
  From `kernel` it names nothing else.
- From `hal` it names only passive items (abi-check R6):
  - `hal::sys::*` (the ABI edge's constants and `#[repr(C)]` structures);
  - from `hal::syscall`, only the types `HandlerInfo`, `Register`, `Rval`, `SysArgs`,
    `SysResult`, `SyscallFn` and `ENOSYS`, and the registration of its dispatcher.
  It never names a `hal` function with side effects (`dispatch`, `hal::cpu::*`,
  `hal::user::*`, the kcopy tokens, ...).
- No logic, no state, no loops (R1, R2, R4, R5), as before.
- Widening the door (another `kernel` module, or more of `hal`) is the user's decision,
  in its own commit, with the reason.
- A `pub` item added to `kernel::api` widens the door without touching `sys/abi/`: a
  review of such a change checks what it exposes to `abi`.
- Until the slice that implements decision 52 lands, the tree still has `hal::api` and
  abi-check enforces the 2026-11-06 text; that slice rewrites abi-check with this one.
