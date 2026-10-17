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
//! cannot call the machine code still in `bsd` (`spl(9)`, `curcpu()`, the user copy), nor the
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
//! yet (`intr.rs`), one boot CPU (`cpu.rs`).
//!
//! **The boot contract.** Every bridge is a safe trait (decision 30). The three memory
//! bridges (`PageMemory`, `PoolBackend`, `Malloc`) are registered once by the kernel's boot
//! path (`bsd`'s `hal_bridges_register`, called by `stand::start_kernel` before `main()`, on
//! the boot CPU, before interrupts and before any user of the pieces) with the kernel's own
//! `uvm_km`, `pool(9)` and `malloc(9)`; `hal`'s soundness rests on that registration: the
//! blocks they return are kernel memory of the size granted, exclusively the caller's until
//! freed through the same bridge (`sys/hal/trust.rs` checks every answer for alignment and
//! size). A slot is set once, so nothing registered later can replace them.
//!
//! ## Redesign
//! - Not a port: the C calls these functions directly. The registry is the one place where
//!   an arrow goes up, so it is audited here: the slots are private, a slot is set once
//!   (`register_*` refuses a second implementation), and a reader gets an `Option`.

use crate::cons::Console;
use crate::cpu::CpuId;
use crate::intr::IntrLevel;
use crate::kmbuf::PageMemory;
use crate::malloc::Malloc;
use crate::module::ModulePlatform;
use crate::once::OnceStatic;
#[cfg(any(not(target_os = "none"), target_arch = "x86_64"))]
use crate::pmap::{Random, TlbCpus};
use crate::pool::PoolBackend;
use crate::sleep::Sleep;
use crate::user::UserCopy;

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
    /// Registers the machine's `spl(9)` (`sys/machine/intr.rs`). Paid by N4 (the hardware enters hal with its kernel owner, N4.1/N4.5).
    INTR_LEVEL: IntrLevel,
    register_intr_level,
    intr_level
);

bridge!(
    /// Registers the running CPU's identity (`curcpu()`, `cpu_number()`). Paid by N4 (the hardware enters hal with its kernel owner, N4.1/N4.5).
    CPU_ID: CpuId,
    register_cpu_id,
    cpu_id
);

bridge!(
    /// Registers the sleep queues (`sys/kern/kern_synch.rs`). Paid by N4.
    SLEEP: Sleep,
    register_sleep,
    sleep
);

bridge!(
    /// Registers the kernel map's pages (`km_alloc`/`km_free` of `sys/uvm/uvm_km.rs`). The boot
    /// contract (module docs): called once by the kernel's boot path, before interrupts and
    /// before any user, with the kernel's own page allocator; `hal`'s soundness rests on it.
    /// Paid by N3.
    PAGE_MEMORY: PageMemory,
    register_page_memory,
    page_memory
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
    /// Registers the machine's user copy (`copyin`, `copyout`, `copyinstr`, `copyoutstr` of
    /// `sys/machine/copy.rs`). Paid by N4 (with the trap code, N4.5).
    USER_COPY: UserCopy,
    register_user_copy,
    user_copy
);

bridge!(
    /// Registers the console (`cnputc`, `cngetc`, `cnpollc` of `sys/dev/cons.rs`), once
    /// `consinit()` has run. Paid by N7.
    CONSOLE: Console,
    register_console,
    console
);

bridge!(
    /// Registers the module loader's platform (`sys/hal/module/load.rs`): the boot loader's
    /// hand-over of `abi_openbsd.ko` and the kernel pmap it is mapped with
    /// (`sys/kern/abi_module.rs`), once, by the kernel's boot path before `main` loads the
    /// module; the loader's three `unsafe` rest on it (the platform premise). Paid by N3.
    MODULE_PLATFORM: ModulePlatform,
    register_module_platform,
    module_platform
);

// The pmap's two bridges exist where `hal::pmap` does (`lib.rs`: the host and amd64 now, arm64
// with its backend, N3.1 commit 5); their readers are the backends'.

#[cfg(any(not(target_os = "none"), target_arch = "x86_64"))]
bridge!(
    /// Registers the CPUs as amd64's TLB shootdown needs them (`CPU_INFO_FOREACH`,
    /// `cpu_is_running`, `ci_proc_pmap`, `x86_fast_ipi`, `wbinvd_on_all_cpus`, `ci_cflushsz` of
    /// `sys/arch/amd64/amd64/{cpu,ipi}.rs`). Paid by N4 (the CPUs enter hal with their kernel
    /// owner). arm64 registers none.
    TLB_CPUS: TlbCpus,
    register_tlb_cpus,
    tlb_cpus #[cfg_attr(
        not(all(target_os = "none", target_arch = "x86_64", feature = "multiprocessor")),
        allow(dead_code, reason = "read by amd64's MULTIPROCESSOR shootdown alone")
    )]
);

#[cfg(any(not(target_os = "none"), target_arch = "x86_64"))]
bridge!(
    /// Registers `arc4random(9)` (`sys/dev/rnd.rs`) for arm64's ASID allocator. Paid by N7.
    RANDOM: Random,
    register_random,
    random #[allow(dead_code, reason = "read by arm64's ASID allocator, N3.1 commit 5")]
);

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    struct Other;

    impl IntrLevel for Other {
        fn splraise(&self, _ipl: i32) -> i32 {
            0
        }
        fn splx(&self, _s: i32) {}
        fn mpfloor(&self) -> i32 {
            0
        }
        fn ipl_high(&self) -> i32 {
            0
        }
    }

    static OTHER: Other = Other;

    #[test]
    fn a_bridge_is_registered_once() {
        testutil::install();
        assert!(intr_level().is_some());
        assert_eq!(
            register_intr_level(&OTHER),
            Err(AlreadyRegistered("IntrLevel"))
        );
    }
}
/* </TESTS> */
