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
 * Copyright (c) 2016 Jonathan Gray <jsg@openbsd.org>
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
//! ARM Power State Coordination Interface on the device tree in the kernel: `dev/fdt/psci.c`.
//!
//! The firmware interface that powers the machine off (`SYSTEM_OFF`), resets it
//! (`SYSTEM_RESET`), starts and stops CPUs, and carries the SMCCC calls of the Spectre
//! workarounds. `psci* at fdt? early 1` attaches it from the `/psci` node (mainbus's early
//! pass, before the CPUs); it assigns [`psci_powerdown`] and [`psci_reset`] to the kernel's
//! `powerdownfn` and `cpuresetfn` slots (`kernel::base::reboot`), which arm64's `boot(9)` calls.
//!
//! Upstream: sys/dev/fdt/psci.c @ 3ce1f3f79392
//! LZ: sys/dev/fdt/psci.rs@e8249449d905
//!
//! ## Deviations
//! - `psci_flush_bp` takes the current CPU's `ci_flush_bp` cell as an argument instead of
//!   reading `curcpu()->ci_flush_bp`: `ci_flush_bp` is a member of `bsd`'s arm64 `struct
//!   cpu_info` (N7.2c's); arm64's `cpu.rs`'s `cpu_flush_bp_psci` passes it.
//! - The results are truncated to `i32`/`u32` as the C's return types do.
//! - `psci_cpu_suspend` and `psci_system_suspend` (whose arguments are a resume entry point
//!   and its context) are visible stubs: they report themselves once (`unported!`) and answer
//!   `PSCI_NOT_SUPPORTED`. Neither has a caller (`bsd`'s `cpu_psci_idle_cycle` and
//!   `cpu_suspend_primary` are reported unported, LZ's gaps); the entry point arrives with the
//!   secondary start's typed one (N7.2c).
//! - `psci_cpu_on` is [`psci_cpu_on_call`]: the conduit and the `CPU_ON` id, for the caller
//!   that owns the entry point and the context (`bsd`'s `bootarg_mp_start`, an `unsafe fn`, with
//!   `bsd`'s own `smc_call`/`hvc_call`), until N7.2c types the secondary start.
//! - A PSCI 0.1 node's ids reach the firmware only as `hal::cpu::smccc` classifies them: its
//!   `system_off`, `system_reset` and `cpu_off`, recorded at the first attach; every other
//!   call is a fast call of the Arm architecture or of PSCI (stricter than the C's `smccc()`,
//!   which passes any id; no caller at the pin passes another).
//! - A second node that attaches prints its line, but the first node's softc and ids stay the
//!   ones every call uses, where the C's `psci_sc` would name the last: the SMCCC call's record
//!   of a 0.1 firmware's ids is sealed by the first call, which the first attach makes, so the
//!   last node's could not be recorded. The machine's tree has one `/psci`, which mainbus
//!   attaches in its own pass and the root's passes skip.
//!
//! ## Redesign
//! - `psci_ca`/`psci_cd` -> [`PSCI`], a [`DriverOps`] (decision 46) whose probe reads the bus's
//!   [`FdtArgs`] back from [`Aux::Typed`]; `cfdata[]`'s row 29 names it (`Cfdata::native`)
//!   with a stub `Cfdriver` of the name (`bsd`'s `conf/ioconf.rs`).
//! - `struct psci_softc` (LZ: a `#[repr(C)]` softc of `Cell`s behind an `unsafe impl Softc`, and
//!   `psci_sc` an `AtomicPtr` read back with an `unsafe` block) -> [`Psci`], built whole from
//!   the node and set once in `PSCI_SC` (a `OnceStatic`) where the C assigns `psci_sc`; the
//!   two members attach writes after (`sc_smccc_version`, `sc_system_suspend`) are atomics.
//!   The device's softc is empty ([`PsciDev`]).
//! - `sc_callfn` (a pointer to `hvc_call` or `smc_call`, which LZ reached through
//!   `machine::fdt`'s `Fdt` trait) -> a `hal::cpu::smccc::Conduit` held by value; every call is
//!   `hal::cpu::smccc::call`, the SMCCC value class (P44).
//! - `cpuresetfn = psci_reset`, `powerdownfn = psci_powerdown` -> the kernel's slots
//!   (`kernel::base::reboot::set_cpuresetfn`, `set_powerdownfn`).
//! - Every operation is a method of [`Psci`] and its C-named free function the same over
//!   `PSCI_SC`, so the tests run the C's decisions on a softc of their own; the attach line is
//!   [`PsciLine`], printed whole.

use core::cell::Cell;
use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::cpu::smccc::{self, Conduit, Psci01Ids};
use hal::mem::malloc::{M_NOWAIT, MArc};
use hal::mem::pool::AnyArc;
use hal::sync::once::OnceStatic;

use crate::assert::{kassert, kpanic};
use crate::base::prf::{printf, unported};
use crate::base::reboot::{set_cpuresetfn, set_powerdownfn};
use crate::dev::arm64::fdt::FdtArgs;
use crate::dev::device::{
    Aux, DV_DULL, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, UnitTable,
};
use crate::dev::fdt::pscivar::{
    CPU_OFF, CPU_ON, CPU_SUSPEND, PSCI_FEATURES, PSCI_METHOD_HVC, PSCI_METHOD_NONE,
    PSCI_METHOD_SMC, PSCI_NOT_SUPPORTED, PSCI_SUCCESS, PSCI_VERSION, SYSTEM_OFF, SYSTEM_RESET,
    SYSTEM_SUSPEND,
};
use crate::dev::ofw::fdt::{OF_getprop, OF_getpropint, OF_is_compatible};

/// `SMCCC_VERSION`.
const SMCCC_VERSION: u32 = 0x8000_0000;
/// `SMCCC_ARCH_FEATURES`.
const SMCCC_ARCH_FEATURES: u32 = 0x8000_0001;
/// `SMCCC_ARCH_WORKAROUND_1`.
const SMCCC_ARCH_WORKAROUND_1: u32 = 0x8000_8000;
/// `SMCCC_ARCH_WORKAROUND_2`.
const SMCCC_ARCH_WORKAROUND_2: u32 = 0x8000_7fff;
/// `SMCCC_ARCH_WORKAROUND_3`.
const SMCCC_ARCH_WORKAROUND_3: u32 = 0x8000_3fff;

/// `struct psci_softc` but `sc_dev`: the conduit and the function ids, fixed at attach.
#[derive(Debug)]
pub struct Psci {
    /// `sc_callfn`: `None` when `method` is absent or neither `hvc` nor `smc`.
    conduit: Option<Conduit>,
    /// `sc_method`: `PSCI_METHOD_HVC`, `PSCI_METHOD_SMC` or `PSCI_METHOD_NONE`.
    method: i32,
    /// `sc_psci_version`: `PSCI_VERSION`'s id, 0 for none.
    version_id: u32,
    /// `sc_system_off`.
    system_off: u32,
    /// `sc_system_reset`.
    system_reset: u32,
    /// `sc_cpu_on`.
    cpu_on: u32,
    /// `sc_cpu_off`.
    cpu_off: u32,
    /// `sc_cpu_suspend`.
    cpu_suspend: u32,
    /// The node is the old specification's (`arm,psci`), whose ids are its own properties.
    old: bool,
    /// `sc_smccc_version`: written by the attach once the version is known.
    smccc_version: AtomicU32,
    /// `sc_system_suspend`: written by the attach when the firmware has it.
    system_suspend: AtomicU32,
}

impl Psci {
    /// The softc `psci_attach` fills from `node`: the conduit from `method`, then the ids (the
    /// specification's for `arm,psci-0.2` and `arm,psci-1.0`, the node's properties for the
    /// old `arm,psci`).
    pub fn of_node(node: i32) -> Psci {
        let mut method = [0u8; 128];
        let mut conduit = None;
        let mut m = PSCI_METHOD_NONE;

        // C: `if (OF_getprop(...))`, true for -1 as well; the zeroed buffer then matches
        // nothing.
        if OF_getprop(node, b"method", &mut method) != 0 {
            let len = method.iter().position(|&b| b == 0).unwrap_or(method.len());
            match &method[..len] {
                b"hvc" => {
                    conduit = Some(Conduit::Hvc);
                    m = PSCI_METHOD_HVC;
                }
                b"smc" => {
                    conduit = Some(Conduit::Smc);
                    m = PSCI_METHOD_SMC;
                }
                _ => {}
            }
        }

        let mut sc = Psci {
            conduit,
            method: m,
            version_id: 0,
            system_off: 0,
            system_reset: 0,
            cpu_on: 0,
            cpu_off: 0,
            cpu_suspend: 0,
            old: false,
            smccc_version: AtomicU32::new(0),
            system_suspend: AtomicU32::new(0),
        };

        // The function IDs are only to be parsed for the old specification (as in version
        // 0.1). All newer implementations are supposed to use the specified values.
        if OF_is_compatible(node, b"arm,psci-0.2") || OF_is_compatible(node, b"arm,psci-1.0") {
            sc.version_id = PSCI_VERSION;
            sc.system_off = SYSTEM_OFF;
            sc.system_reset = SYSTEM_RESET;
            sc.cpu_on = CPU_ON;
            sc.cpu_off = CPU_OFF;
            sc.cpu_suspend = CPU_SUSPEND;
        } else if OF_is_compatible(node, b"arm,psci") {
            sc.system_off = OF_getpropint(node, b"system_off", 0);
            sc.system_reset = OF_getpropint(node, b"system_reset", 0);
            sc.cpu_on = OF_getpropint(node, b"cpu_on", 0);
            sc.cpu_off = OF_getpropint(node, b"cpu_off", 0);
            sc.cpu_suspend = OF_getpropint(node, b"cpu_suspend", 0);
            sc.old = true;
        }
        sc
    }

    /// A PSCI 0.1 node's ids, for `hal::cpu::smccc`'s record; `None` for the new
    /// specification's, which the SMCCC call classifies itself.
    fn psci_0_1_ids(&self) -> Option<Psci01Ids> {
        self.old.then_some(Psci01Ids {
            system_off: self.system_off,
            system_reset: self.system_reset,
            cpu_off: self.cpu_off,
            cpu_on: self.cpu_on,
            cpu_suspend: self.cpu_suspend,
        })
    }

    /// `(*sc->sc_callfn)(fid, a1, a2, a3)`, `None` without a conduit.
    fn call(&self, fid: u32, a1: u64, a2: u64, a3: u64) -> Option<u64> {
        self.conduit.map(|c| smccc::call(c, fid, a1, a2, a3))
    }

    /// A call of the id `fid` when there is a conduit and the id is not 0, else
    /// `PSCI_NOT_SUPPORTED` (the C's `if (sc && sc->sc_callfn && sc->sc_X != 0)`).
    fn call_id(&self, fid: u32, a1: u64, a2: u64, a3: u64) -> i32 {
        if fid == 0 {
            return PSCI_NOT_SUPPORTED;
        }
        self.call(fid, a1, a2, a3)
            .map_or(PSCI_NOT_SUPPORTED, |r| r as i32)
    }

    /// The probe of `psci_attach` after `psci_sc = sc`: the version, then, from 1.0 on, the
    /// SMCCC version and `SYSTEM_SUSPEND` when `PSCI_FEATURES` has them; the line it prints.
    fn probe_versions(&self) -> PsciLine {
        let version = self.version();
        let mut line = PsciLine {
            version,
            smccc: None,
            system_suspend: false,
        };
        if version >= 0x10000 {
            if self.features(SMCCC_VERSION) == PSCI_SUCCESS {
                let v = self.smccc_version_call() as u32;
                self.smccc_version.store(v, Ordering::Relaxed);
                line.smccc = Some(v);
            }
            if self.features(SYSTEM_SUSPEND) == PSCI_SUCCESS {
                self.system_suspend.store(SYSTEM_SUSPEND, Ordering::Relaxed);
                line.system_suspend = true;
            }
        }
        line
    }

    /// `psci_reset`'s call: `SYSTEM_RESET`.
    fn reset(&self) {
        let _ = self.call(self.system_reset, 0, 0, 0);
    }

    /// `psci_powerdown`'s call: `SYSTEM_OFF`.
    fn powerdown(&self) {
        let _ = self.call(self.system_off, 0, 0, 0);
    }

    /// `psci_flush_bp_smccc_arch_workaround_1`'s call.
    fn workaround_1(&self) {
        let _ = self.call(SMCCC_ARCH_WORKAROUND_1, 0, 0, 0);
    }

    /// Whether the firmware implements, and the CPU needs, the workaround `id`: SMCCC 1.1
    /// allows us to detect it.
    fn needs(&self, id: u32) -> bool {
        self.smccc_version.load(Ordering::Relaxed) >= 0x10001 && self.arch_features(id) == 0
    }

    /// `psci_flush_bp`'s decision for one CPU: the workaround when implemented and needed
    /// (and the flush now), else nothing.
    fn flush_bp(&self, ci_flush_bp: &Cell<Option<fn()>>) {
        if self.needs(SMCCC_ARCH_WORKAROUND_1) {
            // Workaround implemented and needed.
            ci_flush_bp.set(Some(psci_flush_bp_smccc_arch_workaround_1));
            self.workaround_1();
        } else {
            // Workaround isn't implemented or isn't needed.
            ci_flush_bp.set(Some(psci_flush_bp_none));
        }
    }

    /// `smccc_enable_arch_workaround_2`'s decision and call.
    fn enable_workaround_2(&self) {
        if self.needs(SMCCC_ARCH_WORKAROUND_2) {
            // Workaround implemented and needed.
            let _ = self.call(SMCCC_ARCH_WORKAROUND_2, 1, 0, 0);
        }
    }

    /// `smccc_version`'s call (`KASSERT(sc && sc->sc_callfn)`): NOT_SUPPORTED reads as 1.0.
    fn smccc_version_call(&self) -> i32 {
        kassert!(self.conduit.is_some());
        let version = self
            .call(SMCCC_VERSION, 0, 0, 0)
            .map_or(PSCI_NOT_SUPPORTED, |r| r as i32);
        if version != PSCI_NOT_SUPPORTED {
            return version;
        }

        // Treat NOT_SUPPORTED as 1.0
        0x10000
    }

    /// `smccc_arch_features`'s call (`KASSERT(sc && sc->sc_callfn)`).
    fn arch_features(&self, arch_func_id: u32) -> i32 {
        kassert!(self.conduit.is_some());
        self.call(SMCCC_ARCH_FEATURES, u64::from(arch_func_id), 0, 0)
            .map_or(PSCI_NOT_SUPPORTED, |r| r as i32)
    }

    /// `psci_version`'s call: 0.0 without version support.
    fn version(&self) -> u32 {
        if self.version_id == 0 {
            // No version support; return 0.0.
            return 0;
        }
        self.call(self.version_id, 0, 0, 0).map_or(0, |r| r as u32)
    }

    /// `psci_features`'s call.
    fn features(&self, psci_func_id: u32) -> i32 {
        self.call(PSCI_FEATURES, u64::from(psci_func_id), 0, 0)
            .map_or(PSCI_NOT_SUPPORTED, |r| r as i32)
    }

    /// The conduit and `sc_cpu_on`, `None` without either.
    fn cpu_on_call(&self) -> Option<(Conduit, u32)> {
        match (self.conduit, self.cpu_on) {
            (Some(c), id) if id != 0 => Some((c, id)),
            _ => None,
        }
    }
}

/// `psci_attach`'s line: `: PSCI x.y[, SMCCC x.y][, SYSTEM_SUSPEND]` and its newline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PsciLine {
    /// The firmware's PSCI version, major in the high half.
    version: u32,
    /// The SMCCC version, when `PSCI_FEATURES` said it is there.
    smccc: Option<u32>,
    /// `SYSTEM_SUSPEND` is implemented.
    system_suspend: bool,
}

impl fmt::Display for PsciLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ": PSCI {}.{}", self.version >> 16, self.version & 0xffff)?;
        if let Some(v) = self.smccc {
            write!(f, ", SMCCC {}.{}", v >> 16, v & 0xffff)?;
        }
        if self.system_suspend {
            f.write_str(", SYSTEM_SUSPEND")?;
        }
        f.write_str("\n")
    }
}

/// The device's softc: the state is `PSCI_SC`'s.
pub struct PsciDev;

/// `psci_ca` with `psci_cd`'s constants.
pub struct PsciDriver;

/// The driver `cfdata[]`'s `psci* at fdt? early 1` row names (`Cfdata::native`).
pub static PSCI: PsciDriver = PsciDriver;

/// `psci_cd`'s `cd_devs`.
static PSCI_UNITS: UnitTable = UnitTable::new();

/// `psci_sc`: the softc of the first attach (see the module's deviations), set once.
static PSCI_SC: OnceStatic<Psci> = OnceStatic::new();

/// `psci_sc`, if `psci_attach` has run.
fn psci_sc() -> Option<&'static Psci> {
    PSCI_SC.get()
}

/// `psci_match`: `arm,psci`, `arm,psci-0.2` or `arm,psci-1.0`.
pub fn psci_match(fa: &FdtArgs) -> i32 {
    i32::from(
        OF_is_compatible(fa.node, b"arm,psci")
            || OF_is_compatible(fa.node, b"arm,psci-0.2")
            || OF_is_compatible(fa.node, b"arm,psci-1.0"),
    )
}

impl DriverOps for PsciDriver {
    fn name(&self) -> &'static [u8] {
        b"psci"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &PSCI_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        match MArc::new(M_DEVBUF, M_NOWAIT, PsciDev) {
            Some(sc) => MArc::erase(sc),
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, psci_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        psci_attach(fa);
    }
}

/// `psci_attach`: pick the conduit and the function ids, report the versions, and hook
/// `boot(9)`'s power-off and reset.
pub fn psci_attach(fa: &FdtArgs) {
    let new = Psci::of_node(fa.node);
    // The firmware's 0.1 ids, or none for the new specification: recorded before the first
    // call, which seals the record (`hal::cpu::smccc`).
    let _ = smccc::record_psci_0_1(new.psci_0_1_ids());
    let sc = match PSCI_SC.set(new) {
        Ok(()) => match psci_sc() {
            Some(sc) => sc,
            None => kpanic(format_args!("psci_attach: the softc went")),
        },
        Err(second) => {
            // A second node: its line, as the C prints it; the first node stays the one the
            // calls use (the module's deviations).
            printf(format_args!("{}", second.probe_versions()));
            return;
        }
    };

    printf(format_args!("{}", sc.probe_versions()));

    if sc.system_off != 0 {
        set_powerdownfn(psci_powerdown);
    }
    if sc.system_reset != 0 {
        set_cpuresetfn(psci_reset);
    }
}

/// `psci_reset`: `SYSTEM_RESET`; returns only if the firmware did not reset the machine.
pub fn psci_reset() {
    if let Some(sc) = psci_sc() {
        sc.reset();
    }
}

/// `psci_powerdown`: `SYSTEM_OFF`; returns only if the firmware did not power off.
pub fn psci_powerdown() {
    if let Some(sc) = psci_sc() {
        sc.powerdown();
    }
}

/*
 * Firmware-based workaround for CVE-2017-5715.  We determine whether
 * the workaround is actually implemented and needed the first time we
 * are invoked such that we only make the firmware call when appropriate.
 */

/// `psci_flush_bp_none`.
pub fn psci_flush_bp_none() {}

/// `psci_flush_bp_smccc_arch_workaround_1`.
pub fn psci_flush_bp_smccc_arch_workaround_1() {
    if let Some(sc) = psci_sc() {
        sc.workaround_1();
    }
}

/// `psci_flush_bp`: decide, once, what the current CPU's `ci_flush_bp` is, and flush now
/// (`ci_flush_bp` is the cell of `curcpu()`; see the module's deviations).
pub fn psci_flush_bp(ci_flush_bp: &Cell<Option<fn()>>) {
    match psci_sc() {
        Some(sc) => sc.flush_bp(ci_flush_bp),
        // Workaround isn't implemented or isn't needed.
        None => ci_flush_bp.set(Some(psci_flush_bp_none)),
    }
}

/// `smccc_enable_arch_workaround_2`: Spectre-V4's firmware workaround, when implemented and
/// needed.
pub fn smccc_enable_arch_workaround_2() {
    if let Some(sc) = psci_sc() {
        sc.enable_workaround_2();
    }
}

/// `smccc_needs_arch_workaround_3`: whether Spectre-BHB's firmware workaround is
/// implemented and needed.
pub fn smccc_needs_arch_workaround_3() -> i32 {
    i32::from(psci_sc().is_some_and(|sc| sc.needs(SMCCC_ARCH_WORKAROUND_3)))
}

/// `smccc_version` (`KASSERT(sc && sc->sc_callfn)`).
pub fn smccc_version() -> i32 {
    let sc = psci_sc();
    kassert!(sc.is_some());
    sc.map_or(0x10000, Psci::smccc_version_call)
}

/// `smccc`: an SMCCC call of the value class, or `PSCI_NOT_SUPPORTED` without a conduit.
pub fn smccc(func_id: u32, arg0: u64, arg1: u64, arg2: u64) -> i32 {
    psci_sc()
        .and_then(|sc| sc.call(func_id, arg0, arg1, arg2))
        .map_or(PSCI_NOT_SUPPORTED, |r| r as i32)
}

/// `smccc_arch_features` (`KASSERT(sc && sc->sc_callfn)`).
pub fn smccc_arch_features(arch_func_id: u32) -> i32 {
    let sc = psci_sc();
    kassert!(sc.is_some());
    sc.map_or(PSCI_NOT_SUPPORTED, |sc| sc.arch_features(arch_func_id))
}

/// `psci_version`: the firmware's version, 0.0 without support.
pub fn psci_version() -> u32 {
    psci_sc().map_or(0, Psci::version)
}

/// `psci_system_suspend`: a visible stub (the module's deviations).
pub fn psci_system_suspend() -> i32 {
    let _ = unported!("psci_system_suspend (a resume entry point: N7.2c)");
    PSCI_NOT_SUPPORTED
}

/// `psci_cpu_off`.
pub fn psci_cpu_off() -> i32 {
    psci_sc().map_or(PSCI_NOT_SUPPORTED, |sc| sc.call_id(sc.cpu_off, 0, 0, 0))
}

/// What `psci_cpu_on` would call: the conduit and `CPU_ON`'s id, `None` without either (the
/// C's `PSCI_NOT_SUPPORTED`). For `bsd`'s secondary start, which owns the entry point and the
/// context (the module's deviations).
pub fn psci_cpu_on_call() -> Option<(Conduit, u32)> {
    psci_sc().and_then(Psci::cpu_on_call)
}

/// `psci_cpu_suspend`: a visible stub (the module's deviations).
pub fn psci_cpu_suspend(_power_state: u64) -> i32 {
    let _ = unported!("psci_cpu_suspend (a resume entry point: N7.2c)");
    PSCI_NOT_SUPPORTED
}

/// `psci_features`: `PSCI_FEATURES` for `psci_func_id`.
pub fn psci_features(psci_func_id: u32) -> i32 {
    psci_sc().map_or(PSCI_NOT_SUPPORTED, |sc| sc.features(psci_func_id))
}

/// `psci_can_suspend`: whether `SYSTEM_SUSPEND` is implemented.
pub fn psci_can_suspend() -> i32 {
    i32::from(psci_sc().is_some_and(|sc| sc.system_suspend.load(Ordering::Relaxed) != 0))
}

/// `psci_method`: `PSCI_METHOD_HVC`, `PSCI_METHOD_SMC` or `PSCI_METHOD_NONE`.
pub fn psci_method() -> i32 {
    psci_sc().map_or(PSCI_METHOD_NONE, |sc| sc.method)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;
    use std::format;
    use std::vec::Vec;

    use crate::dev::ofw::fdt::OF_finddevice;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, TreeGuard, install};

    /// A call the fake firmware saw.
    type Seen = (Conduit, u32, u64, u64, u64);

    std::thread_local! {
        /// What the running test's fake firmware saw (the fake is per thread).
        static SEEN: RefCell<Vec<Seen>> = const { RefCell::new(Vec::new()) };
        /// What it answers for `PSCI_FEATURES`, `SMCCC_ARCH_FEATURES` and `SMCCC_VERSION`.
        static ANSWERS: Cell<(u64, u64, u64)> = const { Cell::new((0, 0, 0)) };
    }

    /// A PSCI 1.1 / SMCCC 1.1 firmware: `PSCI_VERSION` 1.1, `PSCI_FEATURES` and
    /// `SMCCC_ARCH_FEATURES` from [`ANSWERS`] (features, arch features, SMCCC version).
    fn firmware(c: Conduit, fid: u32, a1: u64, a2: u64, a3: u64) -> u64 {
        SEEN.with(|v| v.borrow_mut().push((c, fid, a1, a2, a3)));
        let (features, arch, smccc) = ANSWERS.with(|a| a.get());
        match fid {
            PSCI_VERSION => 0x1_0001,
            PSCI_FEATURES => features,
            SMCCC_VERSION => smccc,
            SMCCC_ARCH_FEATURES => arch,
            _ => 0,
        }
    }

    fn seen() -> Vec<Seen> {
        SEEN.with(|v| core::mem::take(&mut *v.borrow_mut()))
    }

    fn tree(compatible: &[u8], method: Option<&[u8]>, ids: bool) -> TreeGuard {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.begin(b"psci");
        b.prop(b"compatible", compatible);
        if let Some(m) = method {
            b.prop(b"method", m);
        }
        if ids {
            b.prop_cells(b"system_off", &[0x8400_0108]);
            b.prop_cells(b"system_reset", &[0x8400_0109]);
            b.prop_cells(b"cpu_on", &[0xc400_0103]);
            b.prop_cells(b"cpu_off", &[0x8400_0102]);
            b.prop_cells(b"cpu_suspend", &[0xc400_0101]);
        }
        b.end();
        b.end();
        install(b.finish())
    }

    fn softc(compatible: &[u8], method: Option<&[u8]>, ids: bool) -> Psci {
        let _t = tree(compatible, method, ids);
        Psci::of_node(OF_finddevice(b"/psci"))
    }

    #[test]
    fn the_method_names_the_conduit() {
        let hvc = softc(b"arm,psci-1.0\0", Some(b"hvc\0"), false);
        assert_eq!(
            (hvc.conduit, hvc.method),
            (Some(Conduit::Hvc), PSCI_METHOD_HVC)
        );
        let smc = softc(b"arm,psci-1.0\0", Some(b"smc\0"), false);
        assert_eq!(
            (smc.conduit, smc.method),
            (Some(Conduit::Smc), PSCI_METHOD_SMC)
        );
        let other = softc(b"arm,psci-1.0\0", Some(b"svc\0"), false);
        assert_eq!((other.conduit, other.method), (None, PSCI_METHOD_NONE));
        let absent = softc(b"arm,psci-1.0\0", None, false);
        assert_eq!((absent.conduit, absent.method), (None, PSCI_METHOD_NONE));
    }

    #[test]
    fn the_ids_are_the_specification_s_from_0_2_and_the_node_s_before() {
        for compat in [&b"arm,psci-0.2\0"[..], b"arm,psci-1.0\0"] {
            // The node's properties are ignored for the new specification.
            let sc = softc(compat, Some(b"smc\0"), true);
            assert_eq!(
                (sc.version_id, sc.system_off, sc.system_reset),
                (PSCI_VERSION, SYSTEM_OFF, SYSTEM_RESET)
            );
            assert_eq!(
                (sc.cpu_on, sc.cpu_off, sc.cpu_suspend),
                (CPU_ON, CPU_OFF, CPU_SUSPEND)
            );
        }
        let old = softc(b"arm,psci\0", Some(b"smc\0"), true);
        assert_eq!(old.version_id, 0, "PSCI 0.1 has no PSCI_VERSION");
        assert_eq!(
            (
                old.system_off,
                old.system_reset,
                old.cpu_on,
                old.cpu_off,
                old.cpu_suspend
            ),
            (
                0x8400_0108,
                0x8400_0109,
                0xc400_0103,
                0x8400_0102,
                0xc400_0101
            )
        );
        let none = softc(b"arm,psci\0", Some(b"smc\0"), false);
        assert_eq!((none.system_off, none.cpu_on), (0, 0));
    }

    #[test]
    fn the_attach_line_has_three_forms() {
        smccc::set_firmware(Some(firmware));
        let sc = softc(b"arm,psci-1.0\0", Some(b"hvc\0"), false);

        ANSWERS.with(|a| a.set((u64::MAX, 0, 0)));
        assert_eq!(format!("{}", sc.probe_versions()), ": PSCI 1.1\n");

        ANSWERS.with(|a| a.set((0, 0, 0x1_0001)));
        let sc = softc(b"arm,psci-1.0\0", Some(b"hvc\0"), false);
        assert_eq!(
            format!("{}", sc.probe_versions()),
            ": PSCI 1.1, SMCCC 1.1, SYSTEM_SUSPEND\n"
        );
        assert_eq!(sc.system_suspend.load(Ordering::Relaxed), SYSTEM_SUSPEND);
        assert_eq!(sc.smccc_version.load(Ordering::Relaxed), 0x1_0001);

        // SMCCC_VERSION answering NOT_SUPPORTED reads as 1.0.
        ANSWERS.with(|a| a.set((0, 0, u64::MAX)));
        let sc = softc(b"arm,psci-1.0\0", Some(b"hvc\0"), false);
        assert_eq!(
            format!("{}", sc.probe_versions()),
            ": PSCI 1.1, SMCCC 1.0, SYSTEM_SUSPEND\n"
        );

        // PSCI 0.1: no version call at all, so nothing more is asked.
        let _ = seen();
        let old = softc(b"arm,psci\0", Some(b"hvc\0"), true);
        assert_eq!(format!("{}", old.probe_versions()), ": PSCI 0.0\n");
        assert!(seen().is_empty());
        smccc::set_firmware(None);
    }

    #[test]
    fn the_flush_follows_the_smccc_version_and_the_arch_features() {
        smccc::set_firmware(Some(firmware));
        let sc = softc(b"arm,psci-1.0\0", Some(b"smc\0"), false);
        let cell: Cell<Option<fn()>> = Cell::new(None);
        let is = |f: fn()| cell.get().is_some_and(|g| core::ptr::fn_addr_eq(g, f));

        // SMCCC below 1.1: no question asked, nothing to do.
        sc.smccc_version.store(0x1_0000, Ordering::Relaxed);
        let _ = seen();
        sc.flush_bp(&cell);
        assert!(is(psci_flush_bp_none));
        assert!(seen().is_empty());

        // 1.1, the workaround not needed (features nonzero): asked once, nothing called.
        sc.smccc_version.store(0x1_0001, Ordering::Relaxed);
        ANSWERS.with(|a| a.set((0, 1, 0)));
        sc.flush_bp(&cell);
        assert!(is(psci_flush_bp_none));
        assert_eq!(
            seen(),
            [(
                Conduit::Smc,
                SMCCC_ARCH_FEATURES,
                u64::from(SMCCC_ARCH_WORKAROUND_1),
                0,
                0
            )]
        );

        // 1.1 and needed (features 0): asked, then the flush itself.
        ANSWERS.with(|a| a.set((0, 0, 0)));
        sc.flush_bp(&cell);
        assert!(is(psci_flush_bp_smccc_arch_workaround_1));
        assert_eq!(
            seen(),
            [
                (
                    Conduit::Smc,
                    SMCCC_ARCH_FEATURES,
                    u64::from(SMCCC_ARCH_WORKAROUND_1),
                    0,
                    0
                ),
                (Conduit::Smc, SMCCC_ARCH_WORKAROUND_1, 0, 0, 0),
            ]
        );

        // WORKAROUND_2 is enabled with 1; WORKAROUND_3 is only asked about.
        sc.enable_workaround_2();
        assert_eq!(
            seen(),
            [
                (
                    Conduit::Smc,
                    SMCCC_ARCH_FEATURES,
                    u64::from(SMCCC_ARCH_WORKAROUND_2),
                    0,
                    0
                ),
                (Conduit::Smc, SMCCC_ARCH_WORKAROUND_2, 1, 0, 0),
            ]
        );
        assert!(sc.needs(SMCCC_ARCH_WORKAROUND_3));
        ANSWERS.with(|a| a.set((0, u64::MAX, 0)));
        assert!(!sc.needs(SMCCC_ARCH_WORKAROUND_3));
        let _ = seen();
        smccc::set_firmware(None);
    }

    #[test]
    fn cpu_on_is_handed_over_only_with_a_conduit_and_an_id() {
        let sc = softc(b"arm,psci-1.0\0", Some(b"smc\0"), false);
        assert_eq!(sc.cpu_on_call(), Some((Conduit::Smc, CPU_ON)));
        let sc = softc(b"arm,psci-1.0\0", None, false);
        assert_eq!(sc.cpu_on_call(), None);
        let sc = softc(b"arm,psci\0", Some(b"hvc\0"), false);
        assert_eq!(sc.cpu_on_call(), None, "a PSCI 0.1 node without cpu_on");
        let sc = softc(b"arm,psci\0", Some(b"hvc\0"), true);
        assert_eq!(sc.cpu_on_call(), Some((Conduit::Hvc, 0xc400_0103)));
    }

    #[test]
    fn cpu_off_and_the_hooks_call_their_ids() {
        smccc::set_firmware(Some(firmware));
        let sc = softc(b"arm,psci-1.0\0", Some(b"hvc\0"), false);
        let _ = seen();
        assert_eq!(sc.call_id(sc.cpu_off, 0, 0, 0), 0);
        sc.powerdown();
        sc.reset();
        assert_eq!(
            seen(),
            [
                (Conduit::Hvc, CPU_OFF, 0, 0, 0),
                (Conduit::Hvc, SYSTEM_OFF, 0, 0, 0),
                (Conduit::Hvc, SYSTEM_RESET, 0, 0, 0),
            ]
        );
        // An id of 0 is not called.
        assert_eq!(sc.call_id(0, 0, 0, 0), PSCI_NOT_SUPPORTED);
        assert!(seen().is_empty());
        smccc::set_firmware(None);
    }

    #[test]
    fn without_a_psci_node_every_call_is_unsupported() {
        // psci_attach never ran on the host: no softc, no conduit.
        assert!(psci_sc().is_none());
        assert_eq!(psci_version(), 0);
        assert_eq!(psci_features(SYSTEM_OFF), PSCI_NOT_SUPPORTED);
        assert_eq!(psci_cpu_on_call(), None);
        assert_eq!(psci_cpu_off(), PSCI_NOT_SUPPORTED);
        assert_eq!(psci_system_suspend(), PSCI_NOT_SUPPORTED);
        assert_eq!(psci_cpu_suspend(0), PSCI_NOT_SUPPORTED);
        assert_eq!(smccc(SMCCC_VERSION, 0, 0, 0), PSCI_NOT_SUPPORTED);
        assert_eq!(psci_can_suspend(), 0);
        assert_eq!(psci_method(), PSCI_METHOD_NONE);
        assert_eq!(smccc_needs_arch_workaround_3(), 0);
        let cell: Cell<Option<fn()>> = Cell::new(None);
        psci_flush_bp(&cell);
        assert!(
            cell.get()
                .is_some_and(|f| core::ptr::fn_addr_eq(f, psci_flush_bp_none as fn()))
        );
    }
}
/* </TESTS> */
