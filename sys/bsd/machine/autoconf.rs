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
//! What machine-independent autoconfiguration needs from the machine: the tables
//! `config(8)` generates into `ioconf.c` and the hooks each `arch/<arch>/<arch>/autoconf.c`
//! defines.
//!
//! OpenBSD's `subr_autoconf.c` reads `cfdata[]` and `cfroots[]` (from the kernel
//! configuration, compiled per machine), names `mainbus_cd` (each machine's root bus) and
//! calls `device_register()` (each machine's `autoconf.c`). Here `config(8)` is not ported:
//! each architecture writes its `ioconf` by hand in `sys/arch/<arch>/conf/ioconf.rs`, listing
//! the GENERIC entries whose drivers exist (`docs/ARCHITECTURE.md`, "Deviations").
//!
//! `boot -c` (`kern/subr_userconf.c`, UKC, the kernel's `dev::userconf`) edits those tables
//! before autoconfiguration reads them: the machine keeps them in `StaticCell`s, hands them
//! to [`user_config`] once through [`Autoconf::ioconf_mut`] and only then to everybody else.
//! As `config(8)` does, `cfdata[]` ends in free slots ([`Cfdata::free`]) for UKC's `add`;
//! [`ioconf_cfdata`] cuts them off, as the C's loops stop at the first entry without an
//! attachment. The editor reads and writes them through the kernel's `UkcTables`, which
//! [`UkcIoconf`] (the tables and `change`'s working copy) implements here until N9.1's
//! registry owns the tables (`bridges.toml`'s `UkcTables`, `docs/SCAFFOLDING.md`).

use core::any::Any;
use core::ffi::c_void;

#[cfg(feature = "boot_config")]
use alloc::vec::Vec;

#[cfg(feature = "boot_config")]
use kernel::dev::userconf::UkcTables;

use crate::machine::Machine;
use crate::sys::device::{Cfdata, Cfdriver, Device, Nam2blk, Pdevinit};

/// The host's presenter's typed argument and the legacy one it lends, for the shim's tests.
#[cfg(test)]
pub use crate::arch::current::{HostLegacyArgs, HostTypedArgs};

/// The `ioconf.c` tables as UKC (`user_config`) edits them: the whole of `cfdata[]` with its
/// free slots, `cfroots[]`, `pdevinit[]`, and the names `config(8)` writes beside them.
pub struct IoconfTables<'a> {
    /// `cfdata[]`, free slots included.
    pub cfdata: &'a mut [Cfdata],
    /// `cfroots[]` (no terminating `-1`).
    pub cfroots: &'a mut [i16],
    /// `pdevinit[]` (no terminating entry).
    pub pdevinit: &'a mut [Pdevinit],
    /// `pdevnames[]`: the pseudo-devices' names, in `pdevinit[]`'s order.
    pub pdevnames: &'a [&'static [u8]],
    /// `locnames[]`: every locator name, once.
    pub locnames: &'a [&'static [u8]],
    /// `locnamp[]`: runs of indices into `locnames[]`, each ended by `-1`; an entry's
    /// `cf_locnames` is the start of its run.
    pub locnamp: &'a [i16],
}

/// The machine's autoconfiguration tables and hooks.
pub trait Autoconf {
    /// `cfdata[]`: every device the kernel configuration knows, in `ioconf.c`'s order (no
    /// terminating entry).
    fn cfdata() -> &'static [Cfdata];

    /// `cfroots[]`: the indices in `cfdata[]` of the root devices (no terminating `-1`).
    fn cfroots() -> &'static [i16];

    /// The tables for `user_config` to edit.
    ///
    /// # Safety
    ///
    /// Called once, by `user_config` on the boot CPU in `cpu_startup`, before anything has
    /// read `cfdata`, `cfroots` or `pdevinit`; the tables it returns are dropped before
    /// autoconfiguration starts.
    unsafe fn ioconf_mut() -> IoconfTables<'static>;

    /// `mainbus_cd`: the root bus's driver, which `device_mainbus()` reads.
    fn mainbus_cd() -> &'static Cfdriver;

    /// `device_register(dev, aux)`: the machine's look at every device before it attaches
    /// (to find the boot device).
    fn device_register(dev: &Device, aux: *mut c_void);

    /// Scaffolding (docs/SCAFFOLDING.md): the machine's presenter, a native machine bus's
    /// typed attach argument lent to a legacy driver as the machine's legacy one (arm64's
    /// `FdtArgs` as LZ's `FdtAttachArgs`): `f` is called once with its `void *`, valid for
    /// the call, when the machine knows the argument's type, and not at all otherwise (the
    /// default: a machine with no native bus). Only the autoconf shim's door calls it
    /// (`kern/subr_autoconf.rs`'s `present_aux`), which tries it first; the presenters of
    /// machine-independent buses are the shim's, not the machine's.
    fn present_aux(_aux: &(dyn Any + Send + Sync), _f: &mut dyn FnMut(*mut c_void)) {}

    /// `pdevinit[]`: the pseudo-devices `main` attaches, in `ioconf.c`'s order (no
    /// terminating entry).
    fn pdevinit() -> &'static [Pdevinit];
    /// `nam2blk[]`: the disk drivers' names and block majors, from the machine's
    /// `autoconf.c` (`findblkmajor`, `findblkname`; no terminating entry).
    fn nam2blk() -> &'static [Nam2blk];

    /// `diskconf()`: finds the boot device and configures the root, swap and dump devices
    /// (`setroot`); `main` calls it once autoconfiguration is done.
    fn diskconf();
}

/// `cfdata` on the selected machine.
pub fn cfdata() -> &'static [Cfdata] {
    Machine::cfdata()
}

/// `cfdata[]` up to its first free slot: the entries the kernel configuration has, as the C's
/// loops (`for (cf = cfdata; cf->cf_driver; cf++)`) see them.
pub fn ioconf_cfdata(table: &[Cfdata]) -> &[Cfdata] {
    let n = table
        .iter()
        .position(Cfdata::is_free)
        .unwrap_or(table.len());
    &table[..n]
}

/// `cfroots` on the selected machine.
pub fn cfroots() -> &'static [i16] {
    Machine::cfroots()
}

/// `mainbus_cd` on the selected machine.
pub fn mainbus_cd() -> &'static Cfdriver {
    Machine::mainbus_cd()
}

/// `pdevinit` on the selected machine.
pub fn pdevinit() -> &'static [Pdevinit] {
    Machine::pdevinit()
}

/// `device_register` on the selected machine.
pub fn device_register(dev: &Device, aux: *mut c_void) {
    Machine::device_register(dev, aux)
}

/// `present_aux` on the selected machine.
pub fn present_aux(aux: &(dyn Any + Send + Sync), f: &mut dyn FnMut(*mut c_void)) {
    Machine::present_aux(aux, f);
}

/// `diskconf` on the selected machine.
pub fn diskconf() {
    Machine::diskconf()
}

/// `nam2blk` on the selected machine.
pub fn nam2blk() -> &'static [Nam2blk] {
    Machine::nam2blk()
}

/// The tables as UKC edits them, with `change`'s working copy of an entry's locators (the C's
/// `lk`), which `IoconfTables` has no member for.
#[cfg(feature = "boot_config")]
pub(crate) struct UkcIoconf<'a> {
    /// The tables.
    t: IoconfTables<'a>,
    /// `change`'s working copy, between `loc_copy_begin` and `loc_copy_end`.
    lk: Vec<i64>,
}

#[cfg(feature = "boot_config")]
impl<'a> UkcIoconf<'a> {
    /// The tables `t`, with no working copy.
    pub(crate) fn new(t: IoconfTables<'a>) -> Self {
        Self { t, lk: Vec::new() }
    }
}

/// Accessors over the slices; the locator and parent vectors, constant here, are replaced by
/// copies kept for ever (the C's `mallocarray`, never freed).
#[cfg(feature = "boot_config")]
impl UkcTables for UkcIoconf<'_> {
    fn slots(&self) -> usize {
        self.t.cfdata.len()
    }

    fn is_free(&self, dev: usize) -> bool {
        self.t.cfdata[dev].is_free()
    }

    fn name(&self, dev: usize) -> &'static [u8] {
        self.t.cfdata[dev].cf_driver.cd_name
    }

    fn unit(&self, dev: usize) -> i16 {
        self.t.cfdata[dev].cf_unit.get()
    }

    fn set_unit(&mut self, dev: usize, unit: i16) {
        self.t.cfdata[dev].cf_unit.set(unit);
    }

    fn fstate(&self, dev: usize) -> i16 {
        self.t.cfdata[dev].cf_fstate.get()
    }

    fn set_fstate(&mut self, dev: usize, fstate: i16) {
        self.t.cfdata[dev].cf_fstate.set(fstate);
    }

    fn starunit1(&self, dev: usize) -> i16 {
        self.t.cfdata[dev].cf_starunit1
    }

    fn set_starunit1(&mut self, dev: usize, unit: i16) {
        self.t.cfdata[dev].cf_starunit1 = unit;
    }

    fn flags(&self, dev: usize) -> i32 {
        self.t.cfdata[dev].cf_flags
    }

    fn set_flags(&mut self, dev: usize, flags: i32) {
        self.t.cfdata[dev].cf_flags = flags;
    }

    fn loc(&self, dev: usize) -> &[i64] {
        self.t.cfdata[dev].cf_loc
    }

    fn loc_copy_begin(&mut self, dev: usize) -> bool {
        let loc = self.t.cfdata[dev].cf_loc;
        let mut lk: Vec<i64> = Vec::new();
        if lk.try_reserve_exact(loc.len()).is_err() {
            return false;
        }
        lk.extend_from_slice(loc);
        self.lk = lk;
        true
    }

    fn loc_copy_at(&self, k: usize) -> i64 {
        self.lk[k]
    }

    fn set_loc_copy_at(&mut self, k: usize, val: i64) {
        self.lk[k] = val;
    }

    /// The copy's exact capacity makes `leak` an allocation-free hand-over; an unchanged copy
    /// is dropped, the C's `free`.
    fn loc_copy_end(&mut self, dev: usize) {
        let lk = core::mem::take(&mut self.lk);
        if lk[..] != *self.t.cfdata[dev].cf_loc {
            self.t.cfdata[dev].cf_loc = lk.leak();
        }
    }

    fn parents(&self, dev: usize) -> &[i16] {
        self.t.cfdata[dev].cf_parents
    }

    fn locnames_at(&self, dev: usize) -> i32 {
        self.t.cfdata[dev].cf_locnames
    }

    fn slot_copy(&mut self, to: usize, from: usize) {
        self.t.cfdata[to] = self.t.cfdata[from].clone();
    }

    /// The C's one `pv[]` is a constant vector per entry here: a renumbered copy of each
    /// vector that has an index at or above `from`, one per vector however many entries
    /// share it, all made before any entry changes, so running out of memory changes nothing
    /// (the copies made so far are lost). The search for a shared vector is linear in the
    /// copies made (LZ's; the C has none to make): `add` is typed at the console.
    fn parents_shift(&mut self, last: usize, from: i16) -> bool {
        let mut fixed: Vec<(&'static [i16], &'static [i16])> = Vec::new();
        for cf in &self.t.cfdata[..=last] {
            let pv = cf.cf_parents;
            if !pv.iter().any(|&p| p >= from)
                || fixed.iter().any(|(old, _)| core::ptr::eq(*old, pv))
            {
                continue;
            }
            let mut copy: Vec<i16> = Vec::new();
            if fixed.try_reserve(1).is_err() || copy.try_reserve_exact(pv.len()).is_err() {
                return false;
            }
            copy.extend(pv.iter().map(|&p| if p >= from { p + 1 } else { p }));
            fixed.push((pv, copy.leak()));
        }
        for cf in &mut self.t.cfdata[..=last] {
            if let Some((_, copy)) = fixed
                .iter()
                .find(|(old, _)| core::ptr::eq(*old, cf.cf_parents))
            {
                cf.cf_parents = copy;
            }
        }
        true
    }

    fn roots(&self) -> &[i16] {
        self.t.cfroots
    }

    fn set_root(&mut self, i: usize, dev: i16) {
        self.t.cfroots[i] = dev;
    }

    fn pdevs(&self) -> usize {
        self.t.pdevinit.len()
    }

    fn pdev_name(&self, p: usize) -> &'static [u8] {
        self.t.pdevnames[p]
    }

    fn pdev_count(&self, p: usize) -> i32 {
        self.t.pdevinit[p].pdev_count
    }

    fn set_pdev_count(&mut self, p: usize, n: i32) {
        self.t.pdevinit[p].pdev_count = n;
    }

    fn locname(&self, i: usize) -> Option<&'static [u8]> {
        self.t.locnames.get(i).copied()
    }

    fn locnamp(&self) -> &[i16] {
        self.t.locnamp
    }
}

/// `user_config()`: `boot -c`'s `UKC>` prompt, from `cpu_startup`, over the selected
/// machine's tables; the editor is the kernel's (`kernel::dev::userconf`).
#[cfg(feature = "boot_config")]
pub fn user_config() {
    // SAFETY: `Autoconf::ioconf_mut` needs the tables unread and unshared while the returned
    // borrow lives. `cpu_startup` calls this once (its one `RB_CONFIG` test), on the boot CPU,
    // before `main` reaches autoconfiguration or the pseudo-devices, the only readers of
    // `cfdata`, `cfroots` and `pdevinit`; the borrow `t` is lent to the kernel's editor, which
    // keeps nothing of it (`user_config` takes `&mut dyn UkcTables` for the call only), and
    // is dropped when this function returns, before any of those readers runs.
    let t = unsafe { Machine::ioconf_mut() };
    let mut t = UkcIoconf::new(t);
    kernel::dev::userconf::user_config(&mut t, crate::machine::db_machdep::db_enter);
}
/* </CODE> */

/* <TESTS> */
#[cfg(all(test, feature = "boot_config"))]
mod tests {
    // The tables' side of UKC: what the kernel's editor writes is what autoconfiguration
    // reads, shared parent vectors are renumbered alike, a slot copy is the whole entry, and
    // `change`'s copy is kept only when it differs.

    use std::vec;

    use kernel::dev::device::ConfigRow;

    use super::*;
    use crate::sys::device::{Cfattach, DV_DULL, FSTATE_DSTAR, FSTATE_NOTFOUND, FSTATE_STAR};

    static T_CA: Cfattach = Cfattach {
        ca_devsize: 0,
        ca_match: None,
        ca_attach: |_, _, _| {},
        ca_detach: None,
        ca_activate: None,
    };
    static MAINBUS_CD: Cfdriver = Cfdriver::new(b"mainbus", DV_DULL, 0);
    static VIO_CD: Cfdriver = Cfdriver::new(b"vio", DV_DULL, 0);
    static SHARED: [i16; 2] = [0, 1];
    static LOCNAMES: [&[u8]; 1] = [b"dev"];
    static LOCNAMP: [i16; 2] = [0, -1];
    static PDEVNAMES: [&[u8]; 0] = [];

    /// mainbus0, vio* and vio0 (the last two sharing one parent vector), two free slots.
    fn tables() -> (Vec<Cfdata>, Vec<i16>) {
        let cf = vec![
            Cfdata::new(&T_CA, &MAINBUS_CD, 0, FSTATE_NOTFOUND, &[], 0, &[], 1, 0),
            Cfdata::new(&T_CA, &VIO_CD, 0, FSTATE_STAR, &[-1], 0, &SHARED, 0, 0),
            Cfdata::new(&T_CA, &VIO_CD, 0, FSTATE_NOTFOUND, &[3], 7, &SHARED, 0, 1),
            Cfdata::free(),
            Cfdata::free(),
        ];
        (cf, vec![0])
    }

    fn with_tables(f: impl FnOnce(&mut UkcIoconf<'_>)) -> Vec<Cfdata> {
        let (mut cf, mut roots) = tables();
        let mut t = UkcIoconf::new(IoconfTables {
            cfdata: &mut cf,
            cfroots: &mut roots,
            pdevinit: &mut [],
            pdevnames: &PDEVNAMES,
            locnames: &LOCNAMES,
            locnamp: &LOCNAMP,
        });
        f(&mut t);
        cf
    }

    #[test]
    fn a_disabled_row_is_what_config_search_reads() {
        let cf = with_tables(|t| t.set_fstate(1, FSTATE_DSTAR));
        assert_eq!(ConfigRow::fstate(&cf[1]), FSTATE_DSTAR);
        assert_eq!(ConfigRow::fstate(&cf[2]), FSTATE_NOTFOUND);
    }

    #[test]
    fn rows_sharing_a_parent_vector_are_renumbered_alike() {
        let cf = with_tables(|t| assert!(t.parents_shift(2, 1)));
        assert_eq!(cf[1].cf_parents, [0, 2]);
        assert!(core::ptr::eq(cf[1].cf_parents, cf[2].cf_parents));
        assert_eq!(SHARED, [0, 1], "the constant vector is left alone");
        assert!(cf[0].cf_parents.is_empty());
    }

    #[test]
    fn a_slot_copy_is_the_whole_entry_and_a_changed_copy_the_new_locators() {
        let cf = with_tables(|t| {
            t.slot_copy(3, 2);
            assert!(t.loc_copy_begin(3));
            assert_eq!(t.loc_copy_at(0), 3);
            t.set_loc_copy_at(0, 5);
            t.loc_copy_end(3);
            t.set_flags(3, 9);
        });
        assert!(!cf[3].is_free() && cf[4].is_free());
        assert_eq!(cf[3].cf_driver.cd_name, b"vio");
        assert_eq!(
            (cf[3].cf_unit.get(), cf[3].cf_fstate.get()),
            (0, FSTATE_NOTFOUND)
        );
        assert_eq!((cf[3].cf_loc, cf[3].cf_flags), (&[5][..], 9));
        assert_eq!(cf[2].cf_loc, [3], "the source keeps its locators");
        assert!(core::ptr::eq(cf[3].cf_parents, &SHARED[..]));
        assert_eq!((cf[3].cf_locnames, cf[3].cf_starunit1), (0, 1));
    }

    #[test]
    fn an_unchanged_copy_is_not_kept() {
        let before = tables().0[2].cf_loc;
        let cf = with_tables(|t| {
            assert!(t.loc_copy_begin(2));
            t.set_loc_copy_at(0, 3);
            t.loc_copy_end(2);
        });
        assert!(core::ptr::eq(cf[2].cf_loc, before));
    }
}
/* </TESTS> */
