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
//! The Arm SMC Calling Convention's value class: a call to the firmware (`hvc #0` to the
//! hypervisor or `smc #0` to the secure monitor, the conduit psci(4) reads from `/psci`'s
//! `method`) whose arguments are numbers only, `smc_call`/`hvc_call` of `support.S` for the
//! callers that hand the firmware no address.
//!
//! The firmware runs with the kernel's rights, so a call that gives it an address to run at
//! (an entry point) or to follow is as strong as a jump or a store there. [`call`] therefore
//! admits only what this module can classify, and answers every other id
//! `PSCI_NOT_SUPPORTED` (-1) without trapping ([`admits`]):
//! - fast calls of owning entity 0 (the Arm architecture: `SMCCC_VERSION`,
//!   `SMCCC_ARCH_FEATURES`, the three Spectre workarounds), and of owning entity 4 (the
//!   standard secure services) only PSCI's function numbers, 0x00 to 0x15 (PSCI 1.3's): the
//!   rest of entity
//!   4 holds SDEI (0x20-0x3F: `SDEI_EVENT_REGISTER` enters EL1 at an address it is given) and
//!   FF-A (0x60-0xEF: `FFA_RXTX_MAP` maps buffers at addresses it is given), among others; bits
//!   23:16 zero as the convention requires; minus the PSCI functions that take an entry point
//!   (`CPU_SUSPEND`, `CPU_ON`, `CPU_DEFAULT_SUSPEND`, `SYSTEM_SUSPEND`, in both their SMC32 and
//!   SMC64 forms), which are refused before anything else;
//! - a PSCI 0.1 firmware's own ids, which its device tree names (`arm,psci`) and psci(4)'s
//!   attach records here ([`record_psci_0_1`]): its `system_off`, `system_reset` and `cpu_off`
//!   are admitted, its `cpu_on` and `cpu_suspend` refused like their 1.x forms. The record is
//!   sealed by the first record, the first [`call`] or [`seal_psci_0_1`], whichever comes
//!   first: psci(4)'s attach records (`None` for a 1.x node) before its first call, and
//!   mainbus seals it after its early passes whether or not a psci node attached, so once
//!   autoconfiguration passes them no code can name an id of its choosing as one of these.
//!   Before that seal the trust is the boot path's (decision 30's kind): nothing but psci(4)'s
//!   attach records, which no check here can enforce.
//!
//! Every other id (silicon provider, OEM, trusted OS, vendor hypervisor, a yielding call) is
//! refused: the C's `smccc()` hands any id to the firmware, but its only other user, scmi(4),
//! is not ported; when it is, it widens this door with its own decision. What stays trusted
//! is the firmware itself, as the C trusts it, and the 0.1 ids the device tree names. `CPU_ON`
//! stays with the code that owns the entry point and the context it hands over (`bsd`'s
//! secondary start, until N7.2c).
//!
//! The instruction is the machine's (`hal::arch::arm64::smccc`); the host double answers from
//! a per-thread fake firmware a test installs ([`set_firmware`]).
//!
//! ## Deviations
//! - Stricter than the C: an id outside the classes above answers `PSCI_NOT_SUPPORTED`
//!   without reaching the firmware (psci.c's `smccc()` passes any). No caller at the pin
//!   passes one.

use crate::sync::once::OnceStatic;

pub use crate::arch::current::smccc::Conduit;
#[cfg(not(target_os = "none"))]
pub use crate::arch::current::smccc::{FakeFirmware, set_firmware};

/// `PSCI_NOT_SUPPORTED` as the register the firmware answers with: what a refused call
/// returns.
pub const NOT_SUPPORTED: u64 = u64::MAX;

/// The fast-call bit of a function id (bit 31); a yielding call has it clear.
const FAST: u32 = 1 << 31;

/// The SMC64 bit of a function id (bit 30): the same function with 64-bit arguments.
const SMC64: u32 = 1 << 30;

/// The owning entity number of a fast call, bits 29:24.
const fn oen(fid: u32) -> u32 {
    (fid >> 24) & 0x3f
}

/// The Arm architecture's owning entity.
const OEN_ARCH: u32 = 0;

/// The standard secure services' owning entity (PSCI, SDEI, TRNG, FF-A, ...).
const OEN_STD_SECURE: u32 = 4;

/// PSCI's last function number in the standard secure services: 0x00 to 0x15, the
/// functions PSCI 1.3 defines (the numbers above it up to 0x1F are reserved for PSCI and
/// could one day take an address).
const PSCI_LAST: u32 = 0x15;

/// The PSCI functions whose arguments include an address the firmware runs at, by their
/// SMC32 ids (PSCI 1.1, section 5.1): `CPU_SUSPEND`, `CPU_ON`, `CPU_DEFAULT_SUSPEND` and
/// `SYSTEM_SUSPEND`.
const ENTRY_POINT_IDS: [u32; 4] = [0x8400_0001, 0x8400_0003, 0x8400_000c, 0x8400_000e];

/// A PSCI 0.1 firmware's function ids, as its device tree names them (0 for none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Psci01Ids {
    /// `system_off`: admitted.
    pub system_off: u32,
    /// `system_reset`: admitted.
    pub system_reset: u32,
    /// `cpu_off`: admitted.
    pub cpu_off: u32,
    /// `cpu_on`: refused (an entry point).
    pub cpu_on: u32,
    /// `cpu_suspend`: refused (an entry point).
    pub cpu_suspend: u32,
}

/// The recorded PSCI 0.1 ids (`None`: a 1.x firmware, or no record before the first call),
/// sealed by the first record or the first call.
static PSCI_0_1: OnceStatic<Option<Psci01Ids>> = OnceStatic::new();

/// Records a PSCI 0.1 firmware's ids into `cell`, unless it is sealed already.
fn record_into(cell: &OnceStatic<Option<Psci01Ids>>, ids: Option<Psci01Ids>) -> bool {
    cell.set(ids).is_ok()
}

/// The ids `cell` records, sealing it (with none) if nothing was recorded yet.
fn recorded_from(cell: &OnceStatic<Option<Psci01Ids>>) -> Option<&Psci01Ids> {
    if cell.get().is_none() {
        let _ = cell.set(None);
    }
    cell.get().and_then(Option::as_ref)
}

/// Records the PSCI firmware's 0.1 ids (`None` for a 1.x node), psci(4)'s attach before its
/// first call; `false` (nothing recorded) once the record is sealed: by an earlier record, by
/// a [`call`] made before it or by [`seal_psci_0_1`].
pub fn record_psci_0_1(ids: Option<Psci01Ids>) -> bool {
    record_into(&PSCI_0_1, ids)
}

/// Seals the 0.1 record (with none if nothing was recorded): mainbus's call once its early
/// passes are done, so a machine without a `/psci` node, or before it attaches, leaves no
/// record open to later code.
pub fn seal_psci_0_1() {
    let _ = record_into(&PSCI_0_1, None);
}

/// Whether `fid` is one of the standard entry-point functions, in either width.
pub fn takes_entry_point(fid: u32) -> bool {
    ENTRY_POINT_IDS.contains(&(fid & !SMC64))
}

/// Whether [`call`] hands `fid` to the firmware, given the PSCI 0.1 ids `old` recorded (see
/// the module's documentation).
fn admits_with(fid: u32, old: Option<&Psci01Ids>) -> bool {
    if takes_entry_point(fid) {
        return false;
    }
    if let Some(ids) = old
        && fid != 0
    {
        if fid == ids.cpu_on || fid == ids.cpu_suspend {
            return false;
        }
        if fid == ids.system_off || fid == ids.system_reset || fid == ids.cpu_off {
            return true;
        }
    }
    fid & FAST != 0
        && fid & 0x00ff_0000 == 0
        && match oen(fid) {
            OEN_ARCH => true,
            OEN_STD_SECURE => fid & 0xffff <= PSCI_LAST,
            _ => false,
        }
}

/// Whether [`call`] hands `fid` to the firmware (sealing the 0.1 record).
pub fn admits(fid: u32) -> bool {
    admits_with(fid, recorded_from(&PSCI_0_1))
}

/// An SMCCC call through `conduit` of the function `fid` with the value arguments `a1`..`a3`:
/// the value `x0` comes back with. An id this module does not admit answers
/// [`NOT_SUPPORTED`] without reaching the firmware (see the module's documentation).
pub fn call(conduit: Conduit, fid: u32, a1: u64, a2: u64, a3: u64) -> u64 {
    if !admits(fid) {
        return NOT_SUPPORTED;
    }
    crate::arch::current::smccc::trap(conduit, fid, a1, a2, a3)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::vec::Vec;

    std::thread_local! {
        static SEEN: RefCell<Vec<(Conduit, u32, u64, u64, u64)>> = const { RefCell::new(Vec::new()) };
    }

    fn fake(c: Conduit, fid: u32, a1: u64, a2: u64, a3: u64) -> u64 {
        SEEN.with(|s| s.borrow_mut().push((c, fid, a1, a2, a3)));
        0x1_0000
    }

    #[test]
    fn entry_point_ids_are_refused_in_both_widths() {
        set_firmware(Some(fake));
        for id in [0x8400_0001u32, 0x8400_0003, 0x8400_000c, 0x8400_000e] {
            for fid in [id, id | SMC64] {
                assert!(takes_entry_point(fid), "{fid:#x}");
                assert_eq!(
                    call(Conduit::Smc, fid, 0x4000_0000, 0x4000_1000, 0),
                    NOT_SUPPORTED
                );
                assert_eq!(call(Conduit::Hvc, fid, 1, 0x4000_0000, 2), NOT_SUPPORTED);
            }
        }
        // A silicon provider's and a trusted OS's id are refused the same way.
        assert_eq!(
            call(Conduit::Smc, 0xc200_0010, 0x4000_0000, 0, 0),
            NOT_SUPPORTED
        );
        assert_eq!(call(Conduit::Smc, 0xb200_0001, 0, 0, 0), NOT_SUPPORTED);
        assert!(
            SEEN.with(|s| s.borrow().is_empty()),
            "a refused call trapped"
        );
        set_firmware(None);
    }

    #[test]
    fn value_calls_reach_the_firmware_with_their_arguments() {
        set_firmware(Some(fake));
        // PSCI_VERSION, SYSTEM_OFF, SYSTEM_RESET, PSCI_FEATURES, CPU_OFF, SMCCC_VERSION,
        // SMCCC_ARCH_FEATURES and WORKAROUND_2.
        let ids = [
            0x8400_0000u32,
            0x8400_0008,
            0x8400_0009,
            0x8400_000a,
            0x8400_0002,
            0x8000_0000,
            0x8000_0001,
            0x8000_7fff,
        ];
        for fid in ids {
            assert!(!takes_entry_point(fid), "{fid:#x}");
            assert_eq!(call(Conduit::Hvc, fid, 1, 2, 3), 0x1_0000);
        }
        let seen = SEEN.with(|s| s.borrow_mut().split_off(0));
        assert_eq!(seen.len(), ids.len());
        assert_eq!(seen[3], (Conduit::Hvc, 0x8400_000a, 1, 2, 3));
        set_firmware(None);
    }

    #[test]
    fn only_the_arch_and_standard_secure_fast_calls_are_admitted() {
        // The classes the module admits.
        for fid in [
            0x8000_0000u32,
            0x8000_8000,
            0x8000_3fff,
            0x8400_0000,
            0xc400_0008,
        ] {
            assert!(admits_with(fid, None), "{fid:#x}");
        }
        // Silicon provider (2), OEM (3), standard hypervisor (5), vendor hypervisor (6),
        // trusted application (48) and trusted OS (50) fast calls; a yielding call; a fast
        // call with bits 23:16 set.
        for fid in [
            0x8200_0001u32,
            0xc200_0010,
            0x8300_0000,
            0x8500_0000,
            0x8600_0001,
            0xb000_0000,
            0xb200_0001,
            0x0400_0000,
            0x8401_0000,
            // SDEI_EVENT_REGISTER, SDEI_EVENT_COMPLETE_AND_RESUME and FFA_RXTX_MAP, of the
            // standard secure services past PSCI's numbers; TRNG_VERSION beside them; 0x16,
            // PSCI's first number past 1.3's.
            0x8400_0016,
            0xc400_0021,
            0xc400_0026,
            0xc400_0066,
            0x8400_0050,
        ] {
            assert!(!admits_with(fid, None), "{fid:#x}");
        }
    }

    #[test]
    fn a_psci_0_1_firmware_s_ids_are_admitted_or_refused_by_kind() {
        let ids = Psci01Ids {
            system_off: 0x95c1_ba5e,
            system_reset: 0x95c1_ba5f,
            cpu_off: 0x95c1_ba5c,
            cpu_on: 0x8400_0005,
            cpu_suspend: 0x95c1_ba5b,
        };
        for fid in [ids.system_off, ids.system_reset, ids.cpu_off] {
            assert!(admits_with(fid, Some(&ids)), "{fid:#x}");
            assert!(!admits_with(fid, None), "{fid:#x} without the record");
        }
        // cpu_on and cpu_suspend are refused even where the class would admit them.
        assert!(!admits_with(ids.cpu_on, Some(&ids)));
        assert!(admits_with(ids.cpu_on, None));
        assert!(!admits_with(ids.cpu_suspend, Some(&ids)));
        // An id of 0 in the record names nothing.
        let none = Psci01Ids::default();
        assert!(!admits_with(0, Some(&none)));
        assert!(admits_with(0x8400_0000, Some(&none)));
        // A recorded system_off that is a standard entry point is still refused.
        let lying = Psci01Ids {
            system_off: 0xc400_0003,
            ..none
        };
        assert!(!admits_with(0xc400_0003, Some(&lying)));
    }

    #[test]
    fn the_record_is_sealed_by_the_first_record_or_the_first_call() {
        let ids = Psci01Ids {
            system_off: 0x95c1_ba5e,
            ..Psci01Ids::default()
        };
        // A second record fails, and the first stays.
        let cell = OnceStatic::new();
        assert!(record_into(&cell, Some(ids)));
        assert!(!record_into(&cell, None));
        assert_eq!(recorded_from(&cell), Some(&ids));
        // A call (which reads the record) before any record seals it with none.
        let cell = OnceStatic::new();
        assert_eq!(recorded_from(&cell), None);
        assert!(!record_into(&cell, Some(ids)));
        assert_eq!(recorded_from(&cell), None);
        // The process's own record: once a call ran, a record fails.
        let _ = call(Conduit::Hvc, 0x8400_0000, 0, 0, 0);
        assert!(!record_psci_0_1(Some(ids)));
        assert!(!admits(0x95c1_ba5e));
    }

    #[test]
    fn a_record_after_the_seal_fails() {
        let ids = Psci01Ids {
            system_off: 0x95c1_ba5e,
            ..Psci01Ids::default()
        };
        let cell = OnceStatic::new();
        let _ = record_into(&cell, None);
        assert!(!record_into(&cell, Some(ids)));
        assert_eq!(recorded_from(&cell), None);
        // The process's own: sealed, a record fails and the SiP-looking id stays refused.
        seal_psci_0_1();
        assert!(!record_psci_0_1(Some(ids)));
        assert!(!admits(0x95c1_ba5e));
    }

    #[test]
    fn without_a_firmware_every_call_is_unsupported() {
        set_firmware(None);
        assert_eq!(call(Conduit::Smc, 0x8400_0000, 0, 0, 0), NOT_SUPPORTED);
    }
}
/* </TESTS> */
