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
//! (an entry point) is as strong as a jump there: [`call`] refuses the specification's
//! entry-point functions, `CPU_SUSPEND`, `CPU_ON`, `CPU_DEFAULT_SUSPEND` and `SYSTEM_SUSPEND`
//! in both their SMC32 and SMC64 forms, answering `PSCI_NOT_SUPPORTED` (-1) without trapping.
//! What is left (`PSCI_VERSION`, `PSCI_FEATURES`, `SYSTEM_OFF`, `SYSTEM_RESET`, `CPU_OFF`,
//! `SMCCC_VERSION`, `SMCCC_ARCH_FEATURES`, the three Spectre workarounds) reads values and
//! acts on the machine, never on memory the kernel owns. A PSCI 0.1 firmware names its own
//! ids in the device tree (`arm,psci`), which no table here can classify: for such a node the
//! value class is the caller's word, the one boundary this module cannot draw (decision 30's
//! kind). `CPU_ON` itself stays with the code that owns the entry point and the context it
//! hands over (`bsd`'s secondary start, until N7.2c).
//!
//! The instruction is the machine's (`hal::arch::arm64::smccc`); the host double answers from
//! a per-thread fake firmware a test installs ([`set_firmware`]).

pub use crate::arch::current::smccc::Conduit;
#[cfg(not(target_os = "none"))]
pub use crate::arch::current::smccc::{FakeFirmware, set_firmware};

/// `PSCI_NOT_SUPPORTED` as the register the firmware answers with: what a refused call
/// returns.
pub const NOT_SUPPORTED: u64 = u64::MAX;

/// The SMC64 bit of a function id (bit 30): the same function with 64-bit arguments.
const SMC64: u32 = 1 << 30;

/// The PSCI functions whose arguments include an address the firmware runs at, by their
/// SMC32 ids (PSCI 1.1, section 5.1): `CPU_SUSPEND`, `CPU_ON`, `CPU_DEFAULT_SUSPEND` and
/// `SYSTEM_SUSPEND`.
const ENTRY_POINT_IDS: [u32; 4] = [0x8400_0001, 0x8400_0003, 0x8400_000c, 0x8400_000e];

/// Whether `fid` is one of the entry-point functions, in either width.
pub fn takes_entry_point(fid: u32) -> bool {
    ENTRY_POINT_IDS.contains(&(fid & !SMC64))
}

/// An SMCCC call through `conduit` of the function `fid` with the value arguments `a1`..`a3`:
/// the value `x0` comes back with. An entry-point function answers [`NOT_SUPPORTED`] without
/// reaching the firmware (see the module's documentation).
pub fn call(conduit: Conduit, fid: u32, a1: u64, a2: u64, a3: u64) -> u64 {
    if takes_entry_point(fid) {
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
    fn without_a_firmware_every_call_is_unsupported() {
        set_firmware(None);
        assert_eq!(call(Conduit::Smc, 0x8400_0000, 0, 0, 0), NOT_SUPPORTED);
    }
}
/* </TESTS> */
