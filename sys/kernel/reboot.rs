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

/*
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)kern_xxx.c	8.2 (Berkeley) 11/14/93
 */
/* </LICENSES> */

/* <CODE> */
//! `reboot()` and `rebooting`: the tail of `kern_xxx.c` every path that goes down takes
//! (`reboot(2)`, `panic(9)`, ddb's `boot` commands).
//!
//! Upstream: sys/kern/kern_xxx.c @ 3ce1f3f79392
//! LZ: sys/kern/kern_xxx.rs@3c62ede68802
//!
//! `sys_reboot` (its `suser` and the secondary CPUs' stop), `powerbutton_event` and
//! `do_powerdown` (a signal to init) and the `SYSCALL_DEBUG` printers stay `bsd`'s
//! `kern_xxx.rs` until the process and the system call entry move (N4.6, N4.8b).
//!
//! ## Redesign
//! - `reboot` ends in the machine's `boot(9)` through the [`MachineBoot`] bridge
//!   (bridges.toml; `bsd`'s machdep, which syncs the file systems and suspends the devices
//!   first, until N7), registered at boot; LZ called `crate::machine::cpu::boot` directly.
//! - `rebooting` is read by the kernel crate here ([`rebooting`]), not through `SysInfo`.
//! - The C's `KASSERT((howto & RB_NOSYNC) || curproc != NULL)` is kept over
//!   `hal::sleep::curproc_word` (LZ's `reboot` had dropped it; the N4.8a review).

use core::sync::atomic::{AtomicBool, Ordering};

use hal::bridges::AlreadyRegistered;
use hal::once::OnceStatic;
use hal::sys::reboot::RB_NOSYNC;

use crate::assert::{kassert, kpanic};
use crate::time::stop_periodic_resettodr;

/// `rebooting`: set once the system started to go down, for the code that must not sleep
/// any more.
pub static REBOOTING: AtomicBool = AtomicBool::new(false);

/// The machine's `boot(9)`, registered once.
static MACHINE_BOOT: OnceStatic<&'static dyn MachineBoot> = OnceStatic::new();

/// The machine's `boot(9)`: `bsd`'s machdep (`vfs_shutdown`, `config_suspend_all`, the
/// halt, power-down or reset `howto` asks for). Paid by N7.
pub trait MachineBoot: Sync {
    /// `boot(howto)`: never returns.
    fn boot(&self, howto: i32) -> !;
}

/// Registers the machine's `boot(9)`, once; a second call keeps the first and names it.
pub fn install_machine_boot(imp: &'static dyn MachineBoot) -> Result<(), AlreadyRegistered> {
    MACHINE_BOOT
        .set(imp)
        .map_err(|_| AlreadyRegistered("MachineBoot"))
}

/// `rebooting`.
#[inline]
pub fn rebooting() -> bool {
    REBOOTING.load(Ordering::Relaxed)
}

/// `reboot`: stops the clock bookkeeping, marks the system going down and hands over to the
/// machine's `boot(9)` with the `RB_*` flags `howto` (`<sys/reboot.h>`).
pub fn reboot(howto: i32) -> ! {
    // Syncing the file systems needs a thread to do it on.
    kassert!(howto & RB_NOSYNC != 0 || hal::sleep::curproc_word() != 0);

    stop_periodic_resettodr();

    REBOOTING.store(true, Ordering::Relaxed);

    match MACHINE_BOOT.get() {
        Some(m) => m.boot(howto),
        None => kpanic(format_args!("reboot: no machine boot registered")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebooting_starts_false() {
        // REBOOTING is a process-wide flag; a fresh one is false until reboot() sets it.
        let fresh = AtomicBool::new(false);
        assert!(!fresh.load(Ordering::Relaxed));
        assert!(!rebooting() || REBOOTING.load(Ordering::Relaxed));
    }

    #[test]
    #[should_panic(expected = "reboot: no machine boot registered")]
    fn reboot_without_a_machine_panics() {
        if MACHINE_BOOT.get().is_some() {
            kpanic(format_args!("reboot: no machine boot registered"));
        }
        reboot(0);
    }
}
/* </TESTS> */
