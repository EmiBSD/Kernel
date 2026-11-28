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
//! The host double of the SMCCC conduit: the firmware is a function the running test thread
//! installs ([`set_firmware`]); without one every call answers `PSCI_NOT_SUPPORTED`.

use std::cell::Cell;

/// psci(4)'s `sc_callfn`: which instruction reaches the firmware.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conduit {
    /// `hvc #0`: the hypervisor (`method = "hvc"`).
    Hvc,
    /// `smc #0`: the secure monitor (`method = "smc"`).
    Smc,
}

/// A test's firmware: the conduit, the function id and the three arguments in, `x0` out.
pub type FakeFirmware = fn(Conduit, u32, u64, u64, u64) -> u64;

std::thread_local! {
    /// The running test thread's firmware.
    static FIRMWARE: Cell<Option<FakeFirmware>> = const { Cell::new(None) };
}

/// Installs (or, with `None`, removes) the running thread's firmware.
pub fn set_firmware(f: Option<FakeFirmware>) {
    FIRMWARE.with(|c| c.set(f));
}

/// The trap: the thread's firmware, `PSCI_NOT_SUPPORTED` (-1) without one.
pub(crate) fn trap(conduit: Conduit, fid: u32, a1: u64, a2: u64, a3: u64) -> u64 {
    FIRMWARE
        .with(Cell::get)
        .map_or(u64::MAX, |f| f(conduit, fid, a1, a2, a3))
}
/* </CODE> */
