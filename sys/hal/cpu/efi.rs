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
//! The UEFI runtime services as one object with typed methods: what efi(4) calls between
//! `efi_enter` and `efi_leave` (`arch/arm64/dev/efi_machdep.c`, `machine/efivar.h`).
//!
//! [`EfiRuntime::install`] takes, once per machine, the runtime services' pmap (the EFI pmap
//! `hal::vm::pmap::create_efi` made and efi(4) filled from the memory map) and the address of
//! `EFI_RUNTIME_SERVICES`. Each method is one transaction: this object's lock, then the
//! machine's `efi_enter` (interrupts masked, the EFI pmap in the lower half, the FPU usable),
//! the work, and `efi_leave` (the FPU trapped again, the CPU's own pmap back, the interrupt
//! state restored) on every path:
//! - [`EfiRuntime::read`]: bytes of the firmware's tables (the vendor string, the
//!   configuration table) through `kcopy`, a fault being `EFAULT`;
//! - [`EfiRuntime::get_time`], [`EfiRuntime::set_time`]: `GetTime(&time, NULL)` and
//!   `SetTime(&time)`, the service's address read from its slot and called in the same
//!   transaction, a fault in either being `EFAULT` (`efi_enter_check`'s `setjmp`).
//!
//! The firmware's addresses are trusted as the C trusts them, with one bound the C does not
//! write: what is read, and the service called, lie below the EFI pmap's limit
//! (`1 << EFI_SPACE_BITS`), so the lower half, which the EFI pmap translates; an address of
//! the kernel's half is `EFAULT`, never a read of kernel memory or a call into kernel code.
//!
//! The machine's part (`hal::arch::arm64::efi`: the trampoline, the switch) is reached through
//! [`EfiLocked`], which only this object's lock makes; the host double answers from a fake
//! firmware per test thread.

use core::mem::offset_of;

use sysdefs::base::errno::Errno;
use sysdefs::dev::efi::{EfiRuntimeServices, EfiStatus, EfiTime};

use crate::arch::current::efi as machine;
use crate::cpu::intr::IPL_HIGH;
use crate::sync::mutex::{Mutexed, MutexedGuard};
use crate::sync::once::OnceStatic;
use crate::vm::pmap::PmapRef;

#[cfg(not(target_os = "none"))]
pub use crate::arch::current::efi::fake;

/// The runtime services of this machine: their pmap, the table's address and the lock every
/// transaction holds.
pub struct EfiRuntime {
    /// `sc_pm`: the EFI pmap, kept for the kernel's lifetime (the C never destroys it).
    pm: PmapRef,
    /// `sc_rs`: the address of `EFI_RUNTIME_SERVICES`, mapped by `pm`.
    rs: u64,
    /// One transaction at a time: the checkpoint the machine's trampoline writes is one per
    /// machine, as the C's `efi_jmpbuf` (and `sc_psw`), and a second CPU's call would overwrite
    /// it.
    lock: Mutexed<()>,
}

/// The proof that the caller holds the runtime's lock: made only by [`EfiRuntime`]'s methods,
/// handed to the machine's transaction.
pub(crate) struct EfiLocked<'a> {
    _held: MutexedGuard<'a, ()>,
}

/// The installed runtime services.
static RUNTIME: OnceStatic<EfiRuntime> = OnceStatic::new();

/// Whether `[va, va + len)` lies below `limit`: the bound every read and call is held to.
pub(crate) fn below(va: u64, len: usize, limit: u64) -> bool {
    u64::try_from(len)
        .ok()
        .and_then(|l| va.checked_add(l))
        .is_some_and(|end| end <= limit)
}

impl EfiRuntime {
    /// Installs the runtime services, once per machine: `pm` is the EFI pmap (a pmap
    /// `create_efi` did not make panics at the first transaction, as `Pmap::load_efi` does)
    /// and `rs` the address of `EFI_RUNTIME_SERVICES` under it. `None` when they are installed
    /// already.
    pub fn install(pm: PmapRef, rs: u64) -> Option<&'static EfiRuntime> {
        let rt = EfiRuntime {
            pm,
            rs,
            lock: Mutexed::new(IPL_HIGH, ()),
        };
        RUNTIME.set(rt).ok()?;
        RUNTIME.get()
    }

    /// The installed runtime services, if any.
    pub fn get() -> Option<&'static EfiRuntime> {
        RUNTIME.get()
    }

    /// This runtime's lock, as the proof the machine's transaction takes.
    fn locked(&self) -> EfiLocked<'_> {
        EfiLocked {
            _held: self.lock.lock(),
        }
    }

    /// `buf.len()` bytes at the firmware's address `va`, under the EFI pmap: `EFAULT` when
    /// they are not all mapped there or not all below its limit.
    pub fn read(&self, va: u64, buf: &mut [u8]) -> Result<(), Errno> {
        let held = self.locked();
        machine::transaction(&held, &self.pm, |tx| tx.read(va, buf))
    }

    /// The service of the slot at byte `slot` of `EFI_RUNTIME_SERVICES` called with `time`, in
    /// one transaction: the slot read, then the call; `EFAULT` when either faults or lies at
    /// or above the limit.
    fn time_service(&self, slot: usize, time: &mut EfiTime) -> Result<EfiStatus, Errno> {
        let va = self.rs.checked_add(slot as u64).ok_or(Errno::EFAULT)?;
        let held = self.locked();
        machine::transaction(&held, &self.pm, |tx| {
            let mut w = [0u8; 8];
            tx.read(va, &mut w)?;
            tx.time_call(u64::from_ne_bytes(w), time)
        })
    }

    /// `GetTime(&time, NULL)`: the status and the time the firmware wrote; `EFAULT` on a
    /// fault (`efi_enter_check`).
    pub fn get_time(&self) -> Result<(EfiStatus, EfiTime), Errno> {
        let mut time = EfiTime::default();
        let status = self.time_service(offset_of!(EfiRuntimeServices, GetTime), &mut time)?;
        Ok((status, time))
    }

    /// `SetTime(&time)`: the status; `EFAULT` on a fault.
    pub fn set_time(&self, time: &EfiTime) -> Result<EfiStatus, Errno> {
        let mut time = *time;
        self.time_service(offset_of!(EfiRuntimeServices, SetTime), &mut time)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    const RS: u64 = 0x4000_0000;
    const LIMIT: u64 = 1 << 48;

    #[test]
    fn the_bound_holds_every_byte_below_the_limit() {
        assert!(below(LIMIT - 8, 8, LIMIT));
        assert!(!below(LIMIT - 7, 8, LIMIT));
        assert!(below(LIMIT, 0, LIMIT));
        assert!(!below(u64::MAX - 3, 8, LIMIT), "an overflow is out");
        assert!(below(0, 0, LIMIT));
    }

    /// The process's runtime (installed once), over the host's EFI pmap.
    fn runtime() -> &'static EfiRuntime {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let _ = testutil::pmap_boot();
            let pm = crate::vm::pmap::create_efi().expect("the host's EFI pmap");
            assert!(EfiRuntime::install(pm, RS).is_some());
        });
        EfiRuntime::get().expect("installed")
    }

    fn time(year: u16, month: u8, day: u8) -> EfiTime {
        EfiTime {
            Year: year,
            Month: month,
            Day: day,
            Hour: 3,
            Minute: 4,
            Second: 5,
            ..EfiTime::default()
        }
    }

    #[test]
    fn the_runtime_is_installed_once() {
        let rt = runtime();
        let other = crate::vm::pmap::create_efi().expect("the host's EFI pmap");
        assert!(EfiRuntime::install(other, RS).is_none());
        assert!(core::ptr::eq(rt, runtime()));
    }

    #[test]
    fn a_read_is_refused_across_the_limit_and_where_nothing_is_mapped() {
        let rt = runtime();
        fake::reset();
        fake::map(0x5000, &[1, 2, 3, 4]);
        let mut b = [0u8; 4];
        assert_eq!(rt.read(0x5000, &mut b), Ok(()));
        assert_eq!(b, [1, 2, 3, 4]);
        assert_eq!(
            rt.read(0x5002, &mut b),
            Err(Errno::EFAULT),
            "past the mapping"
        );
        fake::map(LIMIT - 2, &[9, 9]);
        assert_eq!(
            rt.read(LIMIT - 2, &mut b),
            Err(Errno::EFAULT),
            "across the limit"
        );
        assert_eq!(rt.read(0xffff_0000_0000_0000, &mut b), Err(Errno::EFAULT));
    }

    #[test]
    fn get_time_reads_the_slot_and_calls_the_service() {
        let rt = runtime();
        fake::reset();
        fake::map(RS + 24, &0x6000u64.to_ne_bytes());
        let t = time(2026, 11, 28);
        fake::service(0x6000, 0, Some(t));
        assert_eq!(rt.get_time(), Ok((0, t)));
        // A status other than success is the firmware's, handed back.
        fake::service(0x6000, 7, Some(t));
        assert_eq!(rt.get_time().map(|(s, _)| s), Ok(7));
        assert_eq!(fake::calls(), 2);
    }

    #[test]
    fn a_fault_or_a_service_of_the_kernel_half_is_efault() {
        let rt = runtime();
        fake::reset();
        // No slot mapped: the read of the slot faults.
        assert_eq!(rt.get_time(), Err(Errno::EFAULT));
        // A slot naming the kernel's half is refused before any call.
        fake::map(RS + 24, &0xffff_8000_0000_1000u64.to_ne_bytes());
        assert_eq!(rt.get_time(), Err(Errno::EFAULT));
        assert_eq!(fake::calls(), 0);
        // The service itself faults.
        fake::map(RS + 24, &0x6000u64.to_ne_bytes());
        fake::fault(0x6000);
        assert_eq!(rt.get_time(), Err(Errno::EFAULT));
        assert_eq!(fake::calls(), 1);
    }

    #[test]
    fn set_time_hands_the_firmware_the_time() {
        let rt = runtime();
        fake::reset();
        fake::map(RS + 32, &0x7000u64.to_ne_bytes());
        fake::service(0x7000, 0, None);
        let t = time(2027, 1, 2);
        assert_eq!(rt.set_time(&t), Ok(0));
        assert_eq!(fake::last_time(), Some(t));
    }
}
/* </TESTS> */
