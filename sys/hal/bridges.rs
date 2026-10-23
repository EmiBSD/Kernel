/* <LICENSES> */
/*
 * Copyright (c) 2026 Emilio Navarrete Lineros <enavarre@outlook.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
/* </LICENSES> */

/* <CODE> */
//! The bridge registry: what the runtime needs from code that has not moved into `hal` yet.
//!
//! Dependencies go one way, `bsd -> kernel -> hal` (docs/ZERO_UNSAFE.md section 0.1): `hal`
//! cannot call the machine code still in `bsd` (the interrupt controller, the CPU hooks), nor the
//! sleep queues, `uvm_km`, `malloc(9)` or `pool(9)`. Each of those is a trait declared beside
//! the primitive that uses it, and `bsd`'s `main()` registers one `&'static dyn Trait`
//! implementation here, once, before first use. A bridge carries no logic (section 0.3.2):
//! each method is one call into the legacy function of the same name.
//!
//! Every bridge is a row of `bridges.toml` (`trait`, `for`, `paid_by`, the milestone whose
//! subsystem moves the implementation in and removes the row); N9 requires the file empty.
//! The kernel crate's own bridges (`Fd`, `Kq`, ... for pipe(2), N2.6) live in `kernel`, in the
//! same shape, one [`crate::once::OnceStatic`] each.
//!
//! What each primitive does while its bridge is not registered (early boot, host tests that
//! register none) is written in its module and is never a panic in a hot path: no interrupts
//! yet (`intr.rs`). The running CPU is no bridge since N4.5a: its core (`cpu.rs`) is read
//! from the machine's per-CPU register.
//!
//! The kernel's page allocator is not a bridge: `kernel::vm::km` implements `hal`'s
//! [`crate::kmbuf::PageMemory`] and its callers pass it down as a value (N3.2), as the pmap's
//! page provider is (D16).
//!
//! **The boot contract.** Every bridge is a safe trait (decision 30). The two memory
//! bridges (`PoolBackend`, `Malloc`) are registered once by the kernel's boot
//! path (`bsd`'s `hal_bridges_register`, called by `stand::start_kernel` before `main()`, on
//! the boot CPU, before interrupts and before any user of the pieces) with the kernel's own
//! `pool(9)` and `malloc(9)`; `hal`'s soundness rests on that registration: the
//! blocks they return are kernel memory of the size granted, exclusively the caller's until
//! freed through the same bridge (`sys/hal/trust.rs` checks every answer for alignment and
//! size). A slot is set once, so nothing registered later can replace them.
//!
//! ## Redesign
//! - Not a port: the C calls these functions directly. The registry is the one place where
//!   an arrow goes up, so it is audited here: the slots are private, a slot is set once
//!   (`register_*` refuses a second implementation), and a reader gets an `Option`.

use crate::cons::Console;
use crate::intr::IntrHardware;
use crate::malloc::Malloc;
use crate::module::ModulePlatform;
use crate::netlock::NetLock;
use crate::once::OnceStatic;
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
use crate::pmap::{Random, TlbCpus};
use crate::pool::PoolBackend;
use crate::prf::Printf;
use crate::sleep::Sleep;

/// A second registration of a bridge, which keeps the first; names the trait.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AlreadyRegistered(pub &'static str);

/// One slot: a private `OnceStatic` of the trait object, its `register_*` function and its
/// crate-private reader (with the attribute given after the reader's name, when one is).
macro_rules! bridge {
    ($(#[$doc:meta])* $slot:ident: $tr:ident, $register:ident, $get:ident $(#[$gattr:meta])?) => {
        static $slot: OnceStatic<&'static dyn $tr> = OnceStatic::new();

        $(#[$doc])*
        pub fn $register(imp: &'static dyn $tr) -> Result<(), AlreadyRegistered> {
            $slot
                .set(imp)
                .map_err(|_| AlreadyRegistered(::core::stringify!($tr)))
        }

        /// The registered implementation, `None` before registration.
        $(#[$gattr])?
        pub(crate) fn $get() -> Option<&'static dyn $tr> {
            $slot.get().copied()
        }
    };
}

bridge!(
    /// Registers the machine's interrupt controller (`struct arm_intr_func`: what the
    /// controller's attach gave `arm_set_intr_handler`, `sys/arch/arm64/arm64/intr.rs`).
    /// Paid by N7 (the GIC drivers).
    INTR_HARDWARE: IntrHardware,
    register_intr_hardware,
    intr_hardware
);

bridge!(
    /// Registers the sleep queues (`sys/kern/kern_synch.rs`). Paid by N4.
    SLEEP: Sleep,
    register_sleep,
    sleep
);

bridge!(
    /// Registers `pool(9)` (`sys/kern/subr_pool.rs`). The boot contract (module docs): called
    /// once by the kernel's boot path, before interrupts and before any user, with the
    /// kernel's own pool allocator; `hal`'s soundness rests on it. Paid by N9.
    POOL_BACKEND: PoolBackend,
    register_pool_backend,
    pool_backend
);

bridge!(
    /// Registers `malloc(9)` and `free(9)` (`sys/kern/kern_malloc.rs`). The boot contract
    /// (module docs): called once by the kernel's boot path, before interrupts and before any
    /// user, with the kernel's own `malloc(9)`; `hal`'s soundness rests on it. Paid by N9.

    MALLOC: Malloc,
    register_malloc,
    malloc
);

bridge!(
    /// Registers the console (`cnputc`, `cngetc`, `cnpollc` of `sys/dev/cons.rs`), once
    /// `consinit()` has run. Paid by N7.
    CONSOLE: Console,
    register_console,
    console
);

bridge!(
    /// Registers `printf(9)` (`sys/kern/subr_prf.rs`: the console and the message buffer),
    /// for the kernel crate's lines (`hal::prf`). Paid by N9 (`subr_prf` goes with the `bsd`
    /// crate).
    PRINTF: Printf,
    register_printf,
    printf
);

bridge!(
    /// Registers the module loader's platform (`sys/hal/module/load.rs`): the boot loader's
    /// hand-over of `abi_openbsd.ko` (`sys/kern/abi_module.rs`), once, by the kernel's boot
    /// path before `main` loads the module; the loader's three `unsafe` rest on it with the
    /// kernel's `ModuleMemory` (the platform premise). Paid by N4 (the boot entry, N4.1).
    MODULE_PLATFORM: ModulePlatform,
    register_module_platform,
    module_platform
);

bridge!(
    /// Registers the network lock (`NET_LOCK`, `NET_UNLOCK` and `rw_status(&netlock)` of
    /// `sys/net/if.rs`), for the kernel crate's vnode pager (`hal::netlock`, N3.4c). Paid by
    /// N6 (the network stack moves).
    NET_LOCK: NetLock,
    register_net_lock,
    net_lock
);

// The pmap's two bridges exist where `hal::pmap` does (`lib.rs`: the host, amd64 and arm64);
// their readers are the backends' (amd64's shootdown, arm64's ASID allocator).
#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
bridge!(
    /// Registers the CPUs as amd64's TLB shootdown needs them (`CPU_INFO_FOREACH`,
    /// `cpu_is_running`, `ci_proc_pmap`, `x86_fast_ipi`, `wbinvd_on_all_cpus`, `ci_cflushsz` of
    /// `sys/arch/amd64/amd64/{cpu,ipi}.rs`). Paid by N4 (the CPUs enter hal with their kernel
    /// owner). arm64 registers none.
    TLB_CPUS: TlbCpus,
    register_tlb_cpus,
    tlb_cpus #[cfg_attr(
        not(all(target_os = "none", target_arch = "x86_64")),
        allow(dead_code, reason = "read by amd64's shootdown alone")
    )]
);

#[cfg(any(
    not(target_os = "none"),
    target_arch = "x86_64",
    target_arch = "aarch64"
))]
bridge!(
    /// Registers `arc4random(9)` (`sys/dev/rnd.rs`) for arm64's ASID allocator and, through
    /// `hal::random`, the kernel's map (N3.5). Paid by N7.
    RANDOM: Random,
    register_random,
    random
);

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    struct Other;

    impl IntrHardware for Other {}

    static OTHER: Other = Other;

    #[test]
    fn a_bridge_is_registered_once() {
        testutil::install();
        assert!(intr_hardware().is_some());
        assert_eq!(
            register_intr_hardware(&OTHER),
            Err(AlreadyRegistered("IntrHardware"))
        );
    }
}
/* </TESTS> */
