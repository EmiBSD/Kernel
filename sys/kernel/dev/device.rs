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
 * Copyright (c) 1992, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This software was developed by the Computer Systems Engineering group
 * at Lawrence Berkeley Laboratory under DARPA contract BG 91-66 and
 * contributed to Berkeley.
 *
 * All advertising materials mentioning features or use of this software
 * must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Lawrence Berkeley Laboratory.
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
 *	@(#)device.h	8.2 (Berkeley) 2/17/94
 */
/* </LICENSES> */

/* <CODE> */
//! The device tree's node and what a driver gives autoconfiguration: `<sys/device.h>`, the
//! types of `autoconf(9)` and `config_attach(9)`.
//!
//! Upstream: sys/sys/device.h @ 3ce1f3f79392
//! LZ: sys/sys/device.rs@e8249449d905
//!
//! A [`Device`] is `struct device`: its class, unit, name, the `cfdata[]` row that found it,
//! its parent, its flags and the links of the lists `kern/subr_autoconf.c` keeps
//! ([`crate::dev::autoconf`]). It is a counted block ([`DevRef`], `MArc<Device>`): the
//! driver's unit table holds one handle while it is attached (the C's `cd_devs` slot and
//! `dv_ref`), and whoever looked it up holds another. The driver's own state, which the C
//! allocates around the `struct device` (`ca_devsize`, the device first), is the node's
//! [`Device::softc`], a handle with its type erased that only the driver names
//! ([`AnyArc::downcast_ref`]).
//!
//! A driver is a [`DriverOps`] (the C's `struct cfattach` with its `struct cfdriver`'s name,
//! class, mode and units); a row of the machine's table is a [`ConfigRow`] (`struct
//! cfdata`). Every legacy driver of `bsd` reaches the tree through one implementation of
//! both, over its `Cfdata` (`sys/kern/subr_autoconf.rs`, `bridges.toml`'s `DriverOps`).
//!
//! ## Deviations
//! - The prototypes of the suspend/resume, root-device and `loadfirmware` functions, and the
//!   types and constants only they use, stay in `bsd`'s `sys/sys/device.rs` with the files
//!   that are not ported (`subr_suspend.c`, `dev/firmload.c`), as in LZ.
//!
//! ## Redesign
//! - LZ's `Device` was the C's header, `#[repr(C)]` first in every softc, every member a
//!   `Cell`, `dv_list` a `queue.rs` link and `dv_ref` a count the code paired by hand; the
//!   driver cast the header back to its softc (`Device::softc::<T>()`, an `unsafe fn`). Here
//!   the node is its own counted block, written once when it is made (class, unit, name, row,
//!   parent) with the flags an atomic, linked on `alldevs` and the deferred queues through
//!   `PinEntry`s ([`hal::ds::pinlist`]), and the softc is an erased handle the node owns: no
//!   cast, no layout rule, and a node outlives every handle to it.
//! - `cd_devs`/`cd_ndevs` (a `mallocarray`ed array of raw pointers in `Cell`s) -> a
//!   [`UnitTable`] per driver: an `MArray` of handles in a cell keyed to the tree's lock,
//!   grown as `config_make_softc` grows the C's array.
//! - `void *aux` -> [`Aux`]; the `void *match` of `cfmatch_t` (LZ's `CfMatch`) -> [`Match`],
//!   which owns the softc made for an indirect bus's probe, so a losing one is dropped, not
//!   freed by hand.
//! - `struct cfattach`/`struct cfdriver` -> the trait [`DriverOps`]; `struct cfdata` -> the
//!   trait [`ConfigRow`] (the machine's table stays `bsd`'s until N7.1c).

use core::any::Any;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::ds::pinlist::PinEntry;
use hal::mem::malloc::{M_NOWAIT, MArc, MArray};
use hal::mem::pool::AnyArc;
use hal::sync::mutex::MutexedGuard;
use hal::sync::rwlock::Guarded;

use crate::assert::kpanic;
use crate::dev::autoconf::TREE;
use sysdefs::base::errno::Errno;

/// `DVACT_DEACTIVATE`: deactivate the device.
pub const DVACT_DEACTIVATE: i32 = 1;
/// `DVACT_QUIESCE`: warn the device about suspend.
pub const DVACT_QUIESCE: i32 = 2;
/// `DVACT_SUSPEND`: suspend the device.
pub const DVACT_SUSPEND: i32 = 3;
/// `DVACT_RESUME`: resume the device.
pub const DVACT_RESUME: i32 = 4;
/// `DVACT_WAKEUP`: tell device to recover after resume.
pub const DVACT_WAKEUP: i32 = 5;
/// `DVACT_POWERDOWN`: power device down.
pub const DVACT_POWERDOWN: i32 = 6;

/// `DVF_ACTIVE`: device is activated (`dv_flags`).
pub const DVF_ACTIVE: i32 = 0x0001;

/// `FSTATE_NOTFOUND`: has not been found.
pub const FSTATE_NOTFOUND: i16 = 0;
/// `FSTATE_FOUND`: has been found.
pub const FSTATE_FOUND: i16 = 1;
/// `FSTATE_STAR`: duplicable.
pub const FSTATE_STAR: i16 = 2;
/// `FSTATE_DNOTFOUND`: has not been found, and is disabled.
pub const FSTATE_DNOTFOUND: i16 = 3;
/// `FSTATE_DSTAR`: duplicable, and is disabled.
pub const FSTATE_DSTAR: i16 = 4;

/// `DETACH_FORCE`: force detachment; hardware gone (`config_detach`, `ca_detach`).
pub const DETACH_FORCE: i32 = 0x01;
/// `DETACH_QUIET`: don't print a notice.
pub const DETACH_QUIET: i32 = 0x02;

/// `CD_INDIRECT` (`cd_mode`): the children match against a preallocated softc.
pub const CD_INDIRECT: i32 = 1;
/// `CD_SKIPHIBERNATE`: not attached when unhibernating.
pub const CD_SKIPHIBERNATE: i32 = 2;
/// `CD_COCOVM`: allow a device on a VM employing confidential computing methods, e.g. AMD
/// SEV.
pub const CD_COCOVM: i32 = 4;

/// `QUIET`: print nothing (a `cfprint_t` result).
pub const QUIET: i32 = 0;
/// `UNCONF`: print " not configured\n".
pub const UNCONF: i32 = 1;
/// `UNSUPP`: print " not supported\n".
pub const UNSUPP: i32 = 2;

/// `M_DEVBUF` (`<sys/malloc.h>`): device driver memory, where the C allocates a softc and
/// `cd_devs`.
pub(crate) const M_DEVBUF: i32 = 2;

/// `MINALLOCSIZE` (`<sys/malloc.h>`, `1 << MINBUCKET`): the smallest `malloc(9)` block, which
/// sizes a driver's first `cd_devs`.
const MINALLOCSIZE: usize = 16;

/// `enum devclass`: minimal device classes. Note that all "system" device types are listed
/// here.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)] // OpenBSD names, verbatim, for grep-ability
pub enum Devclass {
    /// Generic, no special info.
    DV_DULL = 0,
    /// CPU (carries resource utilization).
    DV_CPU = 1,
    /// Disk drive (label, etc).
    DV_DISK = 2,
    /// Network interface.
    DV_IFNET = 3,
    /// Tape device.
    DV_TAPE = 4,
    /// Serial line interface (???).
    DV_TTY = 5,
}

pub use Devclass::*;

/// A handle of a node: the C's `struct device *` with its `dv_ref` (`device_ref` is the
/// clone, `device_unref` the drop).
pub type DevRef = MArc<Device>;

/// `void *aux`: what a bus says about the device it found, handed to every candidate's probe
/// and to the winner's attach.
pub enum Aux<'a> {
    /// A bus's arguments, typed by the bus and read back by its children (`downcast_ref`).
    Typed(&'a (dyn Any + Send + Sync)),
    /// A legacy bus's `void *`, as an address whose provenance was exposed: handed back to
    /// the legacy driver that reads it and never read here (scaffolding, docs/SCAFFOLDING.md).
    Legacy(usize),
}

/// The `void *match` of `cfmatch_t`/`cfscan_t`: the row being tried, or, below an indirect
/// (`CD_INDIRECT`) bus, a softc already made for it. Owned: a losing softc is dropped.
pub enum Match {
    /// The usual case: the `cfdata[]` row.
    Row(&'static dyn ConfigRow),
    /// Below a `CD_INDIRECT` parent: the node `config_make_softc` made for the row.
    Softc(DevRef),
}

impl Match {
    /// The `cfdata[]` row: the match itself, or the softc's `dv_cfdata`.
    pub fn row(&self) -> &'static dyn ConfigRow {
        match self {
            Match::Row(row) => *row,
            Match::Softc(dev) => dev.row,
        }
    }
}

/// What `config_make_softc` writes into a new device before its driver sees it: the
/// driver's softc is made from it ([`DriverOps::make_softc`]).
pub struct Ident<'a> {
    /// `dv_class`.
    pub class: Devclass,
    /// `dv_unit`.
    pub unit: i32,
    /// `dv_xname`, NUL-terminated.
    pub xname: &'a [u8; 16],
    /// `dv_cfdata`.
    pub row: &'static dyn ConfigRow,
    /// `dv_parent`; `None` for a root.
    pub parent: Option<&'a DevRef>,
}

/// `dc_func`: a deferred configuration's function, given the device and its argument.
pub type DeferFn = fn(&DevRef, Option<&AnyArc>);

/// `struct deferred_config` without its device and link, which are the node's: the function
/// `config_defer`/`config_mountroot` queue and its argument.
pub struct Deferred {
    /// `dc_func`.
    pub func: DeferFn,
    /// What the function needs besides the device (a legacy driver's `fn(&Device)`).
    pub arg: Option<AnyArc>,
}

/// `struct device`: a node of the device tree.
pub struct Device {
    /// `dv_class`.
    class: Devclass,
    /// `dv_unit`.
    unit: i32,
    /// `dv_xname`: name + unit, NUL-terminated.
    xname: [u8; 16],
    /// `dv_cfdata`: the row that found us.
    row: &'static dyn ConfigRow,
    /// `dv_parent`; `None` for a root. A child holds its parent alive.
    parent: Option<DevRef>,
    /// `dv_flags` (`DVF_*`).
    flags: AtomicI32,
    /// The driver's softc, typed by the driver alone.
    softc: AnyArc,
    /// \[TREE\] `dv_list`: `alldevs`.
    pub(crate) all: PinEntry<Device>,
    /// \[TREE\] the `deferred_config_queue` link (`dc_queue`).
    pub(crate) deferred: PinEntry<Device>,
    /// \[TREE\] the `mountroot_config_queue` link.
    pub(crate) mountroot: PinEntry<Device>,
    /// \[TREE\] the deferred function while `deferred` is linked, with the node's own handle
    /// (the queue's reference: the C's `dc_dev`).
    pub(crate) defer_slot: Guarded<Option<(Deferred, DevRef)>>,
    /// \[TREE\] the same for `mountroot`.
    pub(crate) mountroot_slot: Guarded<Option<(Deferred, DevRef)>>,
}

hal::pin_adapter!(
    /// `alldevs`, through `Device::all`.
    pub(crate) AllLink: Device, all
);

hal::pin_adapter!(
    /// `deferred_config_queue`, through `Device::deferred`.
    pub(crate) DeferLink: Device, deferred
);

hal::pin_adapter!(
    /// `mountroot_config_queue`, through `Device::mountroot`.
    pub(crate) MountrootLink: Device, mountroot
);

impl Device {
    /// A node for `id` holding `softc`, one handle. The C's `malloc(ca_devsize)` of the
    /// device, which fails by `panic(9)` as `config_make_softc`'s.
    pub(crate) fn new(id: &Ident<'_>, flags: i32, softc: AnyArc) -> DevRef {
        let node = Device {
            class: id.class,
            unit: id.unit,
            xname: *id.xname,
            row: id.row,
            parent: id.parent.cloned(),
            flags: AtomicI32::new(flags),
            softc,
            all: PinEntry::new(),
            deferred: PinEntry::new(),
            mountroot: PinEntry::new(),
            defer_slot: Guarded::new_static_mutexed(&TREE, None),
            mountroot_slot: Guarded::new_static_mutexed(&TREE, None),
        };
        match MArc::new(M_DEVBUF, M_NOWAIT, node) {
            Some(dev) => dev,
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }

    /// `dv_class`.
    pub fn class(&self) -> Devclass {
        self.class
    }

    /// `dv_unit`.
    pub fn unit(&self) -> i32 {
        self.unit
    }

    /// `dv_xname`, NUL-terminated.
    pub fn xname_bytes(&self) -> &[u8; 16] {
        &self.xname
    }

    /// `dv_xname` as a string (`"com0"`), as the C prints it with `%s`.
    pub fn xname(&self) -> &str {
        let len = self.xname.iter().position(|&b| b == 0).unwrap_or(16);
        core::str::from_utf8(&self.xname[..len]).unwrap_or("?")
    }

    /// `dv_cfdata`.
    pub fn row(&self) -> &'static dyn ConfigRow {
        self.row
    }

    /// The driver: `dv_cfdata->cf_attach` with its `cf_driver`.
    pub fn driver(&self) -> &'static dyn DriverOps {
        self.row.driver()
    }

    /// `dv_parent`.
    pub fn parent(&self) -> Option<&DevRef> {
        self.parent.as_ref()
    }

    /// `dv_flags`.
    pub fn flags(&self) -> i32 {
        self.flags.load(Ordering::Relaxed)
    }

    /// `dv_flags = flags`, told to the driver (a legacy header's mirror).
    pub(crate) fn set_flags(&self, flags: i32) {
        self.flags.store(flags, Ordering::Relaxed);
        self.driver().flags_set(self, flags);
    }

    /// The driver's softc, which the driver names by its type (`(struct foo_softc *)self`).
    pub fn softc(&self) -> &AnyArc {
        &self.softc
    }

    /// `dv_parent == parent`.
    pub fn is_child_of(&self, parent: &DevRef) -> bool {
        self.parent
            .as_ref()
            .is_some_and(|p| MArc::ptr_eq(p, parent))
    }
}

/// `cd_devs` and `cd_ndevs` of one driver: its attached devices, by unit.
pub struct UnitTable {
    /// \[TREE\] the slots, `None` before the first unit and after the last detach.
    slots: Guarded<Option<MArray<Option<DevRef>>>>,
}

impl UnitTable {
    /// `{ NULL, .., 0 }`: no units yet.
    pub const fn new() -> Self {
        Self {
            slots: Guarded::new_static_mutexed(&TREE, None),
        }
    }

    /// `cd->cd_devs[unit]` with a reference, whatever its state; `None` outside the array or
    /// for an empty slot.
    pub fn get(&self, unit: i32) -> Option<DevRef> {
        let w = TREE.lock();
        self.get_in(&w, unit).cloned()
    }

    /// `cd_ndevs`.
    pub fn len(&self) -> usize {
        let w = TREE.lock();
        self.len_in(&w)
    }

    /// `cd_ndevs == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `cd->cd_devs[unit]`, for the tree's lock holder.
    pub(crate) fn get_in<'a>(
        &'a self,
        w: &'a MutexedGuard<'_, ()>,
        unit: i32,
    ) -> Option<&'a DevRef> {
        let slots = self.slots.get(w).as_ref()?;
        slots.get(usize::try_from(unit).ok()?)?.as_ref()
    }

    /// `cd_ndevs`, for the tree's lock holder.
    pub(crate) fn len_in(&self, w: &MutexedGuard<'_, ()>) -> usize {
        self.slots.get(w).as_ref().map_or(0, |s| s.len())
    }

    /// `config_make_softc`'s "expand the array": room for `unit`, doubling from
    /// `MINALLOCSIZE / sizeof(void *)` as the C does. The new length when it grew.
    pub(crate) fn grow(&self, w: &mut MutexedGuard<'_, ()>, unit: i32) -> Option<usize> {
        let unit = usize::try_from(unit).unwrap_or(0);
        let old = self.len_in(w);
        if unit < old {
            return None;
        }
        let mut new = if old == 0 {
            MINALLOCSIZE / size_of::<usize>()
        } else {
            old * 2
        };
        while new <= unit {
            new *= 2;
        }
        self.resize(w, new, old);
        Some(new)
    }

    /// `cd_ndevs = len` when it is smaller (`rdattach`'s array of exactly its units). The new
    /// length when it grew.
    pub(crate) fn grow_exact(&self, w: &mut MutexedGuard<'_, ()>, len: usize) -> Option<usize> {
        let old = self.len_in(w);
        if len <= old {
            return None;
        }
        self.resize(w, len, old);
        Some(len)
    }

    /// A new array of `new` slots holding the `old` ones (`mallocarray`, the copy, `free`).
    fn resize(&self, w: &mut MutexedGuard<'_, ()>, new: usize, old: usize) {
        let slot = self.slots.get_mut(w);
        let mut prev = slot.take();
        let Some(grown) = MArray::new(M_DEVBUF, M_NOWAIT, new, |i| {
            prev.as_mut()
                .and_then(|p| p.get_mut(i))
                .and_then(Option::take)
        }) else {
            kpanic(format_args!(
                "config_make_softc: {}ing dev array",
                if old != 0 { "expand" } else { "creat" }
            ));
        };
        *slot = Some(grown);
    }

    /// `cd->cd_devs[unit] = dev`: the slot's old handle, for the caller to drop.
    pub(crate) fn set(
        &self,
        w: &mut MutexedGuard<'_, ()>,
        unit: i32,
        dev: Option<DevRef>,
    ) -> Option<DevRef> {
        let i = usize::try_from(unit).ok();
        let slot = self
            .slots
            .get_mut(w)
            .as_mut()
            .and_then(|s| i.and_then(|i| s.get_mut(i)));
        match slot {
            Some(s) => core::mem::replace(s, dev),
            None => kpanic(format_args!("autoconf: unit {unit} outside cd_devs")),
        }
    }

    /// No unit in use: the array goes (`free(cd->cd_devs)`, `cd_ndevs = 0`), handed to the
    /// caller to drop. `None` when a unit is still in use or there is no array.
    pub(crate) fn release_if_vacant(
        &self,
        w: &mut MutexedGuard<'_, ()>,
    ) -> Option<MArray<Option<DevRef>>> {
        let slot = self.slots.get_mut(w);
        if slot.as_ref().is_some_and(|s| s.iter().all(Option::is_none)) {
            slot.take()
        } else {
            None
        }
    }
}

impl UnitTable {
    /// Host tests: the array forgotten, neither freed nor its handles dropped (they are an
    /// earlier test's memory, leaked with it).
    #[cfg(any(test, feature = "doubles"))]
    pub(crate) fn forget(&self, w: &mut MutexedGuard<'_, ()>) {
        if let Some(slots) = self.slots.get_mut(w).take() {
            core::mem::forget(slots);
        }
    }
}

impl Default for UnitTable {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct cfattach` with its `struct cfdriver`'s constants: one object per driver
/// attachment, which autoconfiguration calls as the C calls through `cf_attach`.
pub trait DriverOps: Sync + 'static {
    /// `cd_name`.
    fn name(&self) -> &'static [u8];
    /// `cd_class`.
    fn class(&self) -> Devclass;
    /// `cd_mode` (`CD_*`).
    fn mode(&self) -> i32;
    /// `cd_devs`, `cd_ndevs`: one table per `cfdriver`.
    fn units(&self) -> &'static UnitTable;
    /// `malloc(ca_devsize, M_DEVBUF, M_NOWAIT | M_ZERO)` and the header: the driver's softc
    /// for the device `id` describes.
    fn make_softc(&self, id: &Ident<'_>) -> AnyArc;
    /// `ca_match`: a match level, 0 for no match; `parent` is `None` for a root.
    fn probe(&self, parent: Option<&DevRef>, m: &Match, aux: &Aux<'_>) -> i32;
    /// `ca_attach(parent, self, aux)`.
    fn attach(&self, parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>);
    /// `ca_detach(self, flags)`; a NULL `ca_detach` is `EOPNOTSUPP`.
    fn detach(&self, _dev: &DevRef, _flags: i32) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// `ca_activate(self, act)`; `None` for a NULL one (`config_suspend` then hands `act` to
    /// the children).
    fn activate(&self, _dev: &DevRef, _act: i32) -> Option<Result<(), Errno>> {
        None
    }
    /// Scaffolding (docs/SCAFFOLDING.md): `dv_flags` changed, for a legacy header's mirror.
    fn flags_set(&self, _dev: &Device, _flags: i32) {}
    /// Scaffolding: `cd_devs[unit]` changed, for a legacy `cd_devs` mirror.
    fn unit_set(&self, _unit: i32, _dev: Option<&Device>) {}
    /// Scaffolding: `cd_ndevs` changed (0: the array went), for a legacy mirror.
    fn units_resized(&self, _len: usize) {}
}

/// One row of `cfdata[]` (`struct cfdata`): its constants, and the two members attach and
/// detach change. Rows are `'static` (the machine's table); `Any`, so a legacy row is named
/// back by its type.
pub trait ConfigRow: Any + Sync {
    /// `cf_attach` with its `cf_driver`.
    fn driver(&'static self) -> &'static dyn DriverOps;
    /// `cf_parents`: the potential parents, as indices into `cfdata[]` (a negative one ends
    /// the list, as the C's `-1`).
    fn parents(&self) -> &'static [i16];
    /// `cf_starunit1`: 1st usable unit number by STAR.
    fn starunit1(&self) -> i16;
    /// `cf_unit`: the unit number (the next one, for a starred row).
    fn unit(&self) -> i16;
    /// `cf_unit = unit`.
    fn set_unit(&self, unit: i16);
    /// `cf_fstate` (`FSTATE_*`).
    fn fstate(&self) -> i16;
    /// `cf_fstate = fstate`.
    fn set_fstate(&self, fstate: i16);
    /// `cf_loc[i]`: the row's `i`-th locator (machine dependent: an FDT bus's `early` pass is
    /// locator 0), `None` past the row's locators; a row without locators has none.
    fn loc(&self, _i: usize) -> Option<i64> {
        None
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `<sys/device.h>`'s kernel half: the constants, the classes and the unit
    // table (the node itself is exercised by `crate::dev::autoconf`'s tests).

    use std::assert_eq;

    use super::*;

    #[test]
    fn devclass_values_follow_the_c_enum() {
        assert_eq!(DV_DULL as i32, 0);
        assert_eq!(DV_DISK as i32, 2);
        assert_eq!(DV_TTY as i32, 5);
    }

    #[test]
    fn a_unit_table_starts_without_units() {
        static UT: UnitTable = UnitTable::new();
        let _mem = crate::testutil::MEMORY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::testutil::install();
        assert!(UT.is_empty());
        assert!(UT.get(0).is_none());
        assert!(UT.get(-1).is_none());
    }

    #[test]
    fn a_unit_table_grows_as_the_c_array() {
        let _mem = crate::testutil::MEMORY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::testutil::install();
        let ut = UnitTable::new();
        let mut w = TREE.lock();
        // MINALLOCSIZE / sizeof(void *), then doubling past the unit.
        assert_eq!(ut.grow(&mut w, 0), Some(2));
        assert_eq!(ut.grow(&mut w, 1), None);
        assert_eq!(ut.grow(&mut w, 2), Some(4));
        assert_eq!(ut.grow(&mut w, 9), Some(16));
        assert_eq!(ut.grow_exact(&mut w, 8), None);
        assert_eq!(ut.len_in(&w), 16);
        assert!(ut.release_if_vacant(&mut w).is_some(), "no unit in use");
        assert_eq!(ut.len_in(&w), 0);
        assert_eq!(ut.grow_exact(&mut w, 1), Some(1), "rdattach's exact array");
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/device.h");
        let ours: &[(&str, i64)] = &[
            ("DVACT_DEACTIVATE", DVACT_DEACTIVATE.into()),
            ("DVACT_SUSPEND", DVACT_SUSPEND.into()),
            ("DVACT_POWERDOWN", DVACT_POWERDOWN.into()),
            ("DVF_ACTIVE", DVF_ACTIVE.into()),
            ("FSTATE_STAR", FSTATE_STAR.into()),
            ("FSTATE_DSTAR", FSTATE_DSTAR.into()),
            ("DETACH_FORCE", DETACH_FORCE.into()),
            ("DETACH_QUIET", DETACH_QUIET.into()),
            ("CD_INDIRECT", CD_INDIRECT.into()),
            ("CD_COCOVM", CD_COCOVM.into()),
            ("UNSUPP", UNSUPP.into()),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
