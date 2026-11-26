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
 *	California, Lawrence Berkeley Laboratories.
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
 * from: Header: subr_autoconf.c,v 1.12 93/02/01 19:31:48 torek Exp  (LBL)
 *
 *	@(#)subr_autoconf.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Autoconfiguration: `kern/subr_autoconf.c`, see `autoconf(9)`, `config_attach(9)`,
//! `config_defer(9)`, `config_found(9)`, `device_lookup(9)`.
//!
//! Upstream: sys/kern/subr_autoconf.c @ 3ce1f3f79392
//! LZ: sys/kern/subr_autoconf.rs@e8249449d905
//!
//! A bus driver's attach function describes each device it finds ([`Aux`]) and calls
//! [`config_found`]; [`config_search`] walks the machine's `cfdata[]` for the rows whose
//! parents include the bus, runs their probes and keeps the best; [`config_attach`] makes the
//! node ([`config_make_softc`]: unit, name, the driver's softc), files it in `alldevs` and the
//! driver's unit table, prints `"<name> at <parent>"` and calls the driver's attach. The
//! machine starts it all with `config_rootfound("mainbus", ...)`. The machine's table, its
//! `device_register` and what has not moved reach this module through [`ConfigGlue`]
//! (`bridges.toml`).
//!
//! ## Deviations
//! - `hotplug(4)` is in both GENERICs (`NHOTPLUG > 0`) but `dev/hotplug.c` is not ported:
//!   `hotplug_device_attach`/`hotplug_device_detach` are reported when not `cold`, as in LZ.
//! - `NMPATH` is 0 (`mpath(4)` is not configured, `scsi/mpath.c` is not ported):
//!   `config_suspend_all` has no `mpath` tree, as the C compiles it with a count of 0.
//! - `autoconf_verbose` starts at 0 (`AUTOCONF_VERBOSE`, a kernel option not configured).
//! - `config_attach` tells a preallocated softc from a row by the [`Match`] it is given
//!   instead of by the parent's `CD_INDIRECT`, which is what produced that variant (LZ's).
//! - `config_detach`'s last loop in C leaves `cf` at `cfdata[]`'s terminating entry and then
//!   writes `cf->cf_unit = 0` into it when the driver has no units left, a store nothing
//!   reads; without a terminating entry the store is not made (LZ's).
//! - A device deferred twice (`config_defer`, `config_mountroot`) is `panic(9)` in every
//!   kernel, not only under `DIAGNOSTIC`: the queue's record is the node's slot, which holds
//!   one function (a driver's bug in the C, which a non-`DIAGNOSTIC` kernel runs twice).
//!
//! ## Redesign
//! - LZ's `alldevs` and the two deferred queues were `queue.rs` TAILQs of raw `Device`
//!   pointers in statics made `Sync` by hand (`Devicelist`, `DeferredConfigQueue`), the
//!   deferred records `malloc`ed (`DeferredConfig`); every walk dereferenced pointers whose
//!   lifetime rested on `config_detach`'s ordering. Here the nodes are counted
//!   ([`DevRef`]): `alldevs` and the queues are `hal::ds::pinlist::PinList`s linking the pinned
//!   node, all keyed to one lock, [`TREE`], the C's kernel lock for them, held for list and
//!   table steps only, never across a driver's function or a sleep. A walk carries an owning
//!   handle for the element it is at (from the driver's unit table for `alldevs`, from the
//!   node's own slot for a queue), so a driver's function runs with no lock and no
//!   reference into the list. The deferred record is the node's slot ([`Deferred`]), which
//!   also holds the queue's handle: no allocation per deferral.
//! - `autoconf_attdet` and its mutex -> [`ATTDET`], a `Mutexed<i32>` at `IPL_HIGH` slept on
//!   with `sleep_nsec` (the C's `msleep_nsec(&autoconf_attdet, ..)`).
//! - `config_detach` and `device_unref` were `unsafe fn`s over raw pointers: here a detach
//!   takes the caller's handle, the unit table's goes with the detach, and the node (with the
//!   driver's softc) goes with the last handle; a handle held across a detach keeps the node
//!   alive and [`device_lookup`] refuses it (inactive, out of the table).
//! - `config_mountroot` asks the glue whether the root is mounted (the C's `rootvp == NULL`);
//!   LZ's stub said never. No caller at the pin runs after the root is mounted.
//! - Cost against the C: `cfdata[]` is lent once per walk ([`ConfigGlue::with_rows`]: the
//!   machine finds its end once, as LZ trims it once), each row then an index, so a search
//!   is O(rows x parents) as the C's, one indirect call per member read; a walk of `alldevs` from a driver's function (activate, detach
//!   children) takes [`TREE`] once a step to get the next node's handle (the C follows a
//!   pointer under the kernel lock); attach takes it three times, detach twice; these are
//!   attach-time paths (decision 43's `device_lookup` of `bsd`'s drivers stays the legacy
//!   mirror's, unchanged).

use core::fmt::{self, Write as _};
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::cpu::intr::{IPL_HIGH, IPL_NONE};
use hal::ds::pinlist::PinList;
use hal::mem::malloc::MArc;
use hal::sync::mutex::{Mutexed, MutexedGuard};
use hal::sync::once::OnceStatic;
use hal::sync::sleep::{INFSLP, PWAIT, wakeup};
use hal::sys::reboot::{RB_COCOVM, RB_UNHIBERNATE};

use crate::assert::{kassert, kpanic};
use crate::base::prf::unported;
use crate::base::prf::{Str, printf};
use crate::dev::device::{
    AllLink, Aux, CD_COCOVM, CD_INDIRECT, CD_SKIPHIBERNATE, ConfigRow, DETACH_FORCE, DETACH_QUIET,
    DV_IFNET, DV_TAPE, DVACT_DEACTIVATE, DVACT_POWERDOWN, DVACT_QUIESCE, DVACT_RESUME,
    DVACT_SUSPEND, DVACT_WAKEUP, DVF_ACTIVE, DeferLink, Deferred, DevRef, Device, FSTATE_DNOTFOUND,
    FSTATE_DSTAR, FSTATE_FOUND, FSTATE_NOTFOUND, FSTATE_STAR, Ident, Match, MountrootLink,
    UnitTable,
};
use crate::sys::errno::Errno;

/// `msgs[]`: what `config_found` prints after a `cfprint_t` returns `QUIET`, `UNCONF` or
/// `UNSUPP`.
const MSGS: [&str; 3] = ["", " not configured\n", " unsupported\n"];

/// `cfprint_t`: prints what the bus found; `pnp` is the parent's name when the device was not
/// configured, `None` when it was (the result is then ignored).
pub type Print<'a> = &'a dyn Fn(&Aux<'_>, Option<&[u8]>) -> i32;

/// `cfmatch_t`, a bus's submatch: a match level for a candidate, in place of its probe.
pub type Submatch<'a> = &'a dyn Fn(Option<&DevRef>, &Match, &Aux<'_>) -> i32;

/// `cfscan_t`: called for every potential child; it owns the match.
pub type Scan<'a> = &'a dyn Fn(&DevRef, Match);

/// `struct matchinfo`: the state of one search for the best match.
struct Matchinfo<'a> {
    /// `fn`: the submatch function, `None` for each candidate's probe.
    f: Option<Submatch<'a>>,
    /// `parent`; `None` (`ROOT`) for a root search.
    parent: Option<&'a DevRef>,
    /// `match`: the best match so far.
    best: Option<Match>,
    /// `aux`.
    aux: &'a Aux<'a>,
    /// `indirect`: the parent's driver is `CD_INDIRECT`.
    indirect: bool,
    /// `pri`: the best match level so far.
    pri: i32,
}

/// The tree's lock: `alldevs`, the deferred queues, every driver's [`UnitTable`] and the
/// nodes' deferred slots (the C keeps them under the kernel lock). Never held across a
/// driver's function or a sleep.
pub(crate) static TREE: Mutexed<()> = Mutexed::new(IPL_NONE, ());

/// `autoconf_attdet` under `autoconf_attdet_mtx`: if > 0, devices are being attached and any
/// thread which tries to detach will sleep; if < 0 devices are being detached and any thread
/// which tries to attach will sleep.
static ATTDET: Mutexed<i32> = Mutexed::new(IPL_HIGH, 0);

/// \[TREE\] `alldevs`: list of all devices.
static ALLDEVS: PinList<AllLink> = PinList::new();
/// \[TREE\] `deferred_config_queue`.
static DEFERRED_CONFIG_QUEUE: PinList<DeferLink> = PinList::new();
/// \[TREE\] `mountroot_config_queue`.
static MOUNTROOT_CONFIG_QUEUE: PinList<MountrootLink> = PinList::new();

/// `autoconf_verbose`: trace probe calls.
pub static AUTOCONF_VERBOSE: AtomicI32 = AtomicI32::new(0);

/// `config_pending`: semaphore for mountroot; woken (its address) at 0.
pub static CONFIG_PENDING: AtomicI32 = AtomicI32::new(0);

/// `autoconf_serial`: versioned state of the devices tree so that changes can be detected.
pub static AUTOCONF_SERIAL: AtomicI32 = AtomicI32::new(0);

/// The machine's table and what has not moved, registered by `config_init`.
static GLUE: OnceStatic<&'static dyn ConfigGlue> = OnceStatic::new();

/// The machine's `cfdata[]`, `cfroots[]` and `mainbus_cd`, its `device_register`, and the
/// kernel state autoconfiguration reads (`boothowto`, `rootvp`): what `config(8)` and the
/// machine give `subr_autoconf.c`. A bridge while `bsd` implements it (`bridges.toml`).
pub trait ConfigGlue: Sync {
    /// `cfdata[]`'s rows in use (the C's walks stop at the first free slot), lent to `f` for
    /// one walk: the machine finds the end once, and each row is an index.
    fn with_rows(&self, f: &mut dyn FnMut(&dyn RowTable));
    /// `cfroots[]`: the root rows, as indices into `cfdata[]`.
    fn roots(&self) -> &'static [i16];
    /// `mainbus_cd`'s units.
    fn mainbus(&self) -> &'static UnitTable;
    /// `device_register(dev, aux)`: the machine's look at every device before its attach.
    fn device_register(&self, dev: &DevRef, aux: &Aux<'_>);
    /// `boothowto`.
    fn boothowto(&self) -> i32;
    /// `rootvp != NULL`: the root file system is mounted.
    fn root_mounted(&self) -> bool;
    /// `nam2blk[]`: the disk drivers' names and block majors (`findblkmajor`,
    /// `findblkname`); none for a machine without disks.
    fn nam2blk(&self) -> &'static [crate::dev::disk::Nam2blk] {
        &[]
    }
}

/// `cfdata[]` as one walk sees it: its rows in use, each found in O(1).
pub trait RowTable {
    /// The rows in use.
    fn len(&self) -> usize;
    /// No row in use.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// `&cfdata[i]`, `i < len()`.
    fn row(&self, i: usize) -> &'static dyn ConfigRow;
}

/// The glue's table for one walk: `f`'s result.
fn with_rows<R>(f: impl FnOnce(&dyn RowTable) -> R) -> R {
    let mut f = Some(f);
    let mut out = None;
    glue().with_rows(&mut |t| {
        if let Some(f) = f.take() {
            out = Some(f(t));
        }
    });
    match out {
        Some(r) => r,
        None => kpanic(format_args!("autoconf: the glue lent no cfdata[]")),
    }
}

/// A C string argument up to its first NUL.
fn cstr(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
}

/// The machine's `nam2blk[]` (`ConfigGlue::nam2blk`); none before the glue is registered.
pub fn config_nam2blk() -> &'static [crate::dev::disk::Nam2blk] {
    GLUE.get().map_or(&[], |g| g.nam2blk())
}

/// The registered glue.
fn glue() -> &'static dyn ConfigGlue {
    match GLUE.get() {
        Some(g) => *g,
        None => kpanic(format_args!("autoconf: no ConfigGlue registered")),
    }
}

/// `config_init`: autoconfiguration's tables come from the machine through `glue` (the C's
/// `TAILQ_INIT`s are the lists' constant heads). Before console initialization; a second
/// registration (a test's) is ignored.
pub fn config_init(glue: &'static dyn ConfigGlue) {
    let _ = GLUE.set(glue);
}

/// `cf->cf_driver == cd`: two rows of one driver share its unit table.
fn same_driver(row: &'static dyn ConfigRow, units: &UnitTable) -> bool {
    ptr::eq(row.driver().units(), units)
}

/// The handle of a node on `alldevs`, from its driver's unit table (an attached node is in
/// it, the C's `cd_devs` reference).
fn handle(w: &MutexedGuard<'_, ()>, d: &Device) -> DevRef {
    match d.driver().units().get_in(w, d.unit()) {
        Some(h) => h.clone(),
        None => kpanic(format_args!(
            "autoconf: {} on alldevs but not in its cd_devs",
            d.xname()
        )),
    }
}

/// The handle a node on a deferred queue parks in its slot.
fn parked(w: &MutexedGuard<'_, ()>, d: &Device, mountroot: bool) -> DevRef {
    let slot = if mountroot {
        d.mountroot_slot.get(w)
    } else {
        d.defer_slot.get(w)
    };
    match slot {
        Some((_, h)) => h.clone(),
        None => kpanic(format_args!(
            "autoconf: a deferred device without its record"
        )),
    }
}

/// `TAILQ_NEXT(dev, dv_list)`, or `TAILQ_FIRST(&alldevs)` for `None`: the next node, with a
/// handle. `after` must still be on `alldevs`.
pub fn alldevs_next(after: Option<&DevRef>) -> Option<DevRef> {
    let w = TREE.lock();
    let next = match after {
        None => ALLDEVS.first(&w),
        Some(d) => ALLDEVS.next(&w, d),
    };
    next.map(|d| handle(&w, d))
}

/// `TAILQ_PREV(dev, devicelist, dv_list)`.
fn alldevs_prev(of: &DevRef) -> Option<DevRef> {
    let w = TREE.lock();
    ALLDEVS.prev(&w, of).map(|d| handle(&w, d))
}

/// `TAILQ_LAST(&alldevs, devicelist)`.
fn alldevs_last() -> Option<DevRef> {
    let w = TREE.lock();
    ALLDEVS.last(&w).map(|d| handle(&w, d))
}

/// `mapply`: apply the matching function and choose the best. This is used a few times and we
/// want to keep the code small.
fn mapply(m: &mut Matchinfo<'_>, cf: &'static dyn ConfigRow) {
    let drv = cf.driver();
    let candidate = if m.indirect {
        Match::Softc(config_make_softc(m.parent, cf))
    } else {
        Match::Row(cf)
    };

    let verbose = AUTOCONF_VERBOSE.load(Ordering::Relaxed) != 0;
    if verbose {
        printf(format_args!(">>> probing for {}", Str(drv.name())));
        if cf.fstate() == FSTATE_STAR {
            printf(format_args!("*\n"));
        } else {
            printf(format_args!("{}\n", cf.unit()));
        }
    }
    let pri = match m.f {
        Some(f) => f(m.parent, &candidate, m.aux),
        None => drv.probe(m.parent, &candidate, m.aux),
    };
    if verbose {
        printf(format_args!(
            ">>> {} probe returned {}\n",
            Str(drv.name()),
            pri
        ));
    }

    // A loser (or the match it beats) is dropped: an indirect bus's softc goes with it.
    if pri > m.pri {
        m.best = Some(candidate);
        m.pri = pri;
    }
}

/// How many of `cf`'s potential parents are `parent`'s row (the C applies the match once per
/// matching parent index; its list ends at the first negative index).
fn parent_hits(t: &dyn RowTable, parent: &DevRef, cf: &dyn ConfigRow) -> usize {
    let n = t.len();
    let prow: *const dyn ConfigRow = parent.row();
    cf.parents()
        .iter()
        .take_while(|&&p| p >= 0)
        .filter(|&&p| {
            let p = p as usize;
            p < n && ptr::addr_eq(t.row(p) as *const dyn ConfigRow, prow)
        })
        .count()
}

/// Whether a search may try `cf` at all: not found yet, not disabled, and allowed by the boot
/// flags.
fn eligible(cf: &'static dyn ConfigRow, boothowto: i32) -> bool {
    let fstate = cf.fstate();
    if fstate == FSTATE_FOUND || fstate == FSTATE_DNOTFOUND || fstate == FSTATE_DSTAR {
        return false;
    }
    let drv = cf.driver();
    if boothowto & RB_UNHIBERNATE != 0
        && (drv.mode() & CD_SKIPHIBERNATE != 0 || drv.class() == DV_IFNET || drv.class() == DV_TAPE)
    {
        return false;
    }
    !(boothowto & RB_COCOVM != 0 && drv.mode() & CD_COCOVM == 0)
}

/// `config_search`: iterate over all potential children of some device, calling the given
/// function (default being the child's probe) for each one. Nonzero returns are matches; the
/// highest value returned is considered the best match. Return the "found child" if we got a
/// match, or `None` otherwise. The `aux` argument is simply passed on through.
///
/// Note that this function is designed so that it can be used to apply an arbitrary function
/// to all potential children (its return value can be ignored).
pub fn config_search(f: Option<Submatch<'_>>, parent: &DevRef, aux: &Aux<'_>) -> Option<Match> {
    let g = glue();
    let mut m = Matchinfo {
        f,
        parent: Some(parent),
        best: None,
        aux,
        indirect: parent.driver().mode() & CD_INDIRECT != 0,
        pri: 0,
    };
    let boothowto = g.boothowto();

    with_rows(|t| {
        for i in 0..t.len() {
            // Skip cf if no longer eligible, otherwise scan through parents for one matching
            // `parent', and try match function.
            let cf = t.row(i);
            if !eligible(cf, boothowto) {
                continue;
            }
            for _ in 0..parent_hits(t, parent, cf) {
                mapply(&mut m, cf);
            }
        }
    });

    if AUTOCONF_VERBOSE.load(Ordering::Relaxed) != 0 {
        match &m.best {
            Some(found) => printf(format_args!(
                ">>> {} probe won\n",
                Str(found.row().driver().name())
            )),
            None => printf(format_args!(">>> no winning probe\n")),
        };
    }
    m.best
}

/// `config_scan`: iterate over all potential children of some device, calling the given
/// function for each one.
pub fn config_scan(f: Scan<'_>, parent: &DevRef) {
    let indirect = parent.driver().mode() & CD_INDIRECT != 0;

    with_rows(|t| {
        for i in 0..t.len() {
            // Skip cf if no longer eligible, otherwise scan through parents for one matching
            // `parent', and try match function.
            let cf = t.row(i);
            let fstate = cf.fstate();
            if fstate == FSTATE_FOUND || fstate == FSTATE_DNOTFOUND || fstate == FSTATE_DSTAR {
                continue;
            }
            for _ in 0..parent_hits(t, parent, cf) {
                let m = if indirect {
                    Match::Softc(config_make_softc(Some(parent), cf))
                } else {
                    Match::Row(cf)
                };
                f(parent, m);
            }
        }
    });
}

/// `config_rootsearch`: find the given root device. This is much like `config_search`, but
/// there is no parent.
pub fn config_rootsearch(f: Option<Submatch<'_>>, rootname: &[u8], aux: &Aux<'_>) -> Option<Match> {
    let g = glue();
    let mut m = Matchinfo {
        f,
        parent: None,
        best: None,
        aux,
        indirect: false,
        pri: 0,
    };
    // Look at root entries for matching name. We do not bother with found-state here since
    // only one instance of each possible root child should ever be searched.
    with_rows(|t| {
        for &p in g.roots().iter().take_while(|&&p| p >= 0) {
            let p = p as usize;
            if p >= t.len() {
                continue;
            }
            let cf = t.row(p);
            let fstate = cf.fstate();
            if fstate == FSTATE_DNOTFOUND || fstate == FSTATE_DSTAR {
                continue;
            }
            if cstr(cf.driver().name()) == cstr(rootname) {
                mapply(&mut m, cf);
            }
        }
    });
    m.best
}

/// `config_found_sm`: the given `aux` argument describes a device that has been found on the
/// given parent, but not necessarily configured. Locate the configuration data for that
/// device (using the submatch function provided, or using candidates' probes) and attach it,
/// and return it. If the device was not configured, call the given `print` function and
/// return `None`.
pub fn config_found_sm(
    parent: &DevRef,
    aux: &Aux<'_>,
    print: Option<Print<'_>>,
    submatch: Option<Submatch<'_>>,
) -> Option<DevRef> {
    if let Some(m) = config_search(submatch, parent, aux) {
        return Some(config_attach(Some(parent), m, aux, print));
    }
    if let Some(print) = print {
        let r = print(aux, Some(cstr(parent.xname_bytes())));
        printf(format_args!(
            "{}",
            usize::try_from(r)
                .ok()
                .and_then(|r| MSGS.get(r))
                .copied()
                .unwrap_or("")
        ));
    }
    None
}

/// `config_found(d, a, p)`: `config_found_sm` without a submatch function (`<sys/device.h>`).
pub fn config_found(parent: &DevRef, aux: &Aux<'_>, print: Option<Print<'_>>) -> Option<DevRef> {
    config_found_sm(parent, aux, print, None)
}

/// `config_rootfound`: as `config_found_sm`, but for root devices.
pub fn config_rootfound(rootname: &[u8], aux: &Aux<'_>) -> Option<DevRef> {
    if let Some(m) = config_rootsearch(None, rootname, aux) {
        return Some(config_attach(None, m, aux, None));
    }
    printf(format_args!(
        "root device {} not configured\n",
        Str(rootname)
    ));
    None
}

/// The entry half of `autoconf_attdet`: an attach waits for the detaches in flight, a detach
/// for the attaches.
fn attdet_enter(attach: bool) {
    let mut g = ATTDET.lock();
    while if attach { *g < 0 } else { *g > 0 } {
        let _ = g.sleep_nsec(ptr::from_ref(&ATTDET), PWAIT, "autoconf", INFSLP);
    }
    *g += if attach { 1 } else { -1 };
}

/// The exit half: the last one out wakes the other kind; the tree's version moves.
fn attdet_leave(attach: bool) {
    let mut g = ATTDET.lock();
    *g -= if attach { 1 } else { -1 };
    if *g == 0 {
        wakeup(ptr::from_ref(&ATTDET));
    }
    AUTOCONF_SERIAL.fetch_add(1, Ordering::Relaxed);
}

/// `config_attach`: attach a found device. Makes the node and its softc unless the match
/// brought one.
pub fn config_attach(
    parent: Option<&DevRef>,
    m: Match,
    aux: &Aux<'_>,
    print: Option<Print<'_>>,
) -> DevRef {
    attdet_enter(true);

    let dev = match m {
        Match::Softc(dev) => dev,
        Match::Row(cf) => config_make_softc(parent, cf),
    };
    let cf = dev.row();
    let drv = cf.driver();
    let units = drv.units();
    let unit = dev.unit();

    {
        let mut w = TREE.lock();
        kassert!(units.len_in(&w) > 0);
        kassert!(usize::try_from(unit).is_ok_and(|u| u < units.len_in(&w)));
        kassert!(units.get_in(&w, unit).is_none());
        let _none = units.set(&mut w, unit, Some(dev.clone()));
        drv.unit_set(unit, Some(&dev));

        // If this is a "STAR" device and we used the last unit, prepare for another one.
        if cf.fstate() == FSTATE_STAR {
            if unit == i32::from(cf.unit()) {
                cf.set_unit(cf.unit() + 1);
            }
        } else {
            cf.set_fstate(FSTATE_FOUND);
        }

        ALLDEVS.insert_tail(&mut w, dev.as_pin());
    }

    match parent {
        None => printf(format_args!("{} at root", dev.xname())),
        Some(p) => {
            printf(format_args!("{} at {}", dev.xname(), p.xname()));
            if let Some(print) = print {
                let _ = print(aux, None);
            }
        }
    }

    // Before attaching, clobber any unfound devices that are otherwise identical, or bump the
    // unit number on all starred cfdata for this device.
    with_rows(|t| {
        let _w = TREE.lock();
        for i in 0..t.len() {
            let c = t.row(i);
            if same_driver(c, units) && i32::from(c.unit()) == unit {
                if c.fstate() == FSTATE_NOTFOUND {
                    c.set_fstate(FSTATE_FOUND);
                }
                if c.fstate() == FSTATE_STAR {
                    c.set_unit(c.unit() + 1);
                }
            }
        }
    });
    glue().device_register(&dev, aux);
    drv.attach(parent, &dev, aux);
    config_process_deferred_children(&dev);
    // NHOTPLUG > 0
    if !hal::sync::sleep::cold() {
        let _ = unported!("hotplug_device_attach (dev/hotplug.c)");
    }

    attdet_leave(true);
    dev
}

/// `dv_xname`: `"%s%d"` of the driver's name and the unit, `panic(9)` when it does not fit.
fn xname_of(name: &[u8], unit: i32) -> [u8; 16] {
    /// `snprintf` into the name: the bytes that fit before the NUL, and the whole length.
    struct Name {
        buf: [u8; 16],
        len: usize,
    }
    impl fmt::Write for Name {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for b in s.bytes() {
                if self.len < self.buf.len() - 1 {
                    self.buf[self.len] = b;
                }
                self.len += 1;
            }
            Ok(())
        }
    }
    let mut n = Name {
        buf: [0; 16],
        len: 0,
    };
    let _ = write!(n, "{}{}", Str(name), unit);
    if n.len >= n.buf.len() {
        kpanic(format_args!("config_make_softc: device name too long"));
    }
    n.buf
}

/// `config_make_softc`: the node of a device `cf` describes below `parent`: its unit (the
/// lowest free one from `cf_starunit1` for a starred row), its name, room in the driver's
/// unit table, and the driver's softc. Not attached: dropping it gives everything back.
pub fn config_make_softc(parent: Option<&DevRef>, cf: &'static dyn ConfigRow) -> DevRef {
    let drv = cf.driver();
    let units = drv.units();

    let (unit, xname) = {
        let mut w = TREE.lock();
        // If this is a STAR device, search for a free unit number
        let unit = if cf.fstate() == FSTATE_STAR {
            let mut unit = i32::from(cf.starunit1());
            while unit < i32::from(cf.unit()) {
                if units.get_in(&w, unit).is_none() {
                    break;
                }
                unit += 1;
            }
            unit
        } else {
            i32::from(cf.unit())
        };

        // Build the device name into dv_xname.
        let xname = xname_of(drv.name(), unit);

        // put this device in the devices array
        if let Some(len) = units.grow(&mut w, unit) {
            drv.units_resized(len);
        }
        if units.get_in(&w, unit).is_some() {
            kpanic(format_args!("config_make_softc: duplicate {}", Str(&xname)));
        }
        (unit, xname)
    };

    let id = Ident {
        class: drv.class(),
        unit,
        xname: &xname,
        row: cf,
        parent,
    };
    let softc = drv.make_softc(&id);
    // always initially active
    Device::new(&id, DVF_ACTIVE, softc)
}

/// A node around a softc its driver made outside `config_make_softc` (a legacy indirect
/// bus's preallocated softc, given back to be attached; a device a legacy test made by
/// hand): scaffolding (docs/SCAFFOLDING.md), not in the tree until attached.
pub fn device_adopt(id: &Ident<'_>, flags: i32, softc: hal::mem::pool::AnyArc) -> DevRef {
    Device::new(id, flags, softc)
}

/// `rdattach`'s attach by hand: a node for `row`'s unit `unit`, in its driver's unit table
/// (an array of exactly `unit + 1` slots when it had fewer) and on `alldevs`, then the
/// driver's attach with no parent. No message, no row state, no attach serialisation, as the
/// C's `rdattach`.
pub fn config_attach_by_hand(row: &'static dyn ConfigRow, unit: i32, aux: &Aux<'_>) -> DevRef {
    let drv = row.driver();
    let units = drv.units();
    let xname = xname_of(drv.name(), unit);
    {
        let mut w = TREE.lock();
        let len = usize::try_from(unit).unwrap_or(0) + 1;
        if let Some(len) = units.grow_exact(&mut w, len) {
            drv.units_resized(len);
        }
    }
    let id = Ident {
        class: drv.class(),
        unit,
        xname: &xname,
        row,
        parent: None,
    };
    let dev = Device::new(&id, DVF_ACTIVE, drv.make_softc(&id));
    {
        let mut w = TREE.lock();
        let _none = units.set(&mut w, unit, Some(dev.clone()));
        drv.unit_set(unit, Some(&dev));
        ALLDEVS.insert_tail(&mut w, dev.as_pin());
    }
    drv.attach(None, &dev, aux);
    dev
}

/// `config_detach`: detach a device. Optionally forced (e.g. because of hardware removal) and
/// quiet. Returns `Ok` if successful, the error otherwise.
///
/// Note that this code wants to be run from a process context, so that the detach can sleep to
/// allow processes which have a device open to run and unwind their stacks.
pub fn config_detach(dev: DevRef, flags: i32) -> Result<(), Errno> {
    attdet_enter(false);

    let cf = dev.row();
    #[cfg(feature = "diagnostic")]
    if cf.fstate() != FSTATE_FOUND && cf.fstate() != FSTATE_STAR {
        kpanic(format_args!("config_detach: bad device fstate"));
    }
    let drv = cf.driver();
    let units = drv.units();

    let rv = 'done: {
        // Ensure the device is deactivated. If the device has an activation entry point and
        // DVF_ACTIVE is still set, the device is busy, and the detach fails.
        let mut rv = config_deactivate(&dev);

        // Try to detach the device. If that's not possible, then we either panic() (for the
        // forced but failed case), or return an error.
        if rv.is_ok() {
            rv = drv.detach(&dev, flags);
        }
        if let Err(e) = rv {
            if flags & DETACH_FORCE == 0 {
                break 'done rv;
            }
            kpanic(format_args!(
                "config_detach: forced detach of {} failed ({})",
                dev.xname(),
                e as i32
            ));
        }

        // The device has now been successfully detached.

        #[cfg(feature = "diagnostic")]
        {
            // Sanity: If you're successfully detached, you should have no children. (Note
            // that because children must be attached after parents, we only need to search
            // the latter part of the list.)
            let w = TREE.lock();
            let mut found = false;
            let mut c = ALLDEVS.next(&w, &dev);
            while let Some(child) = c {
                if child.is_child_of(&dev) {
                    printf(format_args!(
                        "config_detach: {} attached at {}\n",
                        child.xname(),
                        dev.xname()
                    ));
                    found = true;
                }
                c = ALLDEVS.next(&w, child);
            }
            if found {
                kpanic(format_args!(
                    "config_detach: detached device ({}) has children",
                    dev.xname()
                ));
            }
        }

        let unit = dev.unit();
        let old = with_rows(|t| {
            let mut w = TREE.lock();
            // Mark cfdata to show that the unit can be reused, if possible. Note that we can
            // only re-use a starred unit number if the unit being detached had the last
            // assigned unit number.
            for i in 0..t.len() {
                let c = t.row(i);
                if same_driver(c, units) {
                    if c.fstate() == FSTATE_FOUND && i32::from(c.unit()) == unit {
                        c.set_fstate(FSTATE_NOTFOUND);
                    }
                    if c.fstate() == FSTATE_STAR && i32::from(c.unit()) == unit + 1 {
                        c.set_unit(c.unit() - 1);
                    }
                }
            }

            // Unlink from device list, and remove from cfdriver's array.
            ALLDEVS.remove(&mut w, &dev);
            let old = units.set(&mut w, unit, None);
            drv.unit_set(unit, None);
            old
        });
        // The array's reference (the C's `device_unref` of `config_attach`'s).
        drop(old);

        // Tell the world.
        if flags & DETACH_QUIET == 0 {
            printf(format_args!("{} detached\n", dev.xname()));
        }

        // If the device now has no units in use, deallocate its softc array.
        let array = {
            let mut w = TREE.lock();
            let array = units.release_if_vacant(&mut w);
            if array.is_some() {
                drv.units_resized(0);
            }
            array
        };
        drop(array);

        // NHOTPLUG > 0
        if !hal::sync::sleep::cold() {
            let _ = unported!("hotplug_device_detach (dev/hotplug.c)");
        }

        // Return success.
        Ok(())
    };

    attdet_leave(false);
    // The caller's reference goes last: the node, and the driver's softc, with it unless a
    // `device_lookup`'s or a child's reference is still held.
    drop(dev);
    rv
}

/// `config_deactivate`: clears `DVF_ACTIVE` and tells the device (`DVACT_DEACTIVATE`); the
/// flag comes back if the device refuses.
pub fn config_deactivate(dev: &DevRef) -> Result<(), Errno> {
    let oflags = dev.flags();
    let mut rv = Ok(());

    if oflags & DVF_ACTIVE != 0 {
        dev.set_flags(oflags & !DVF_ACTIVE);
        rv = config_suspend(dev, DVACT_DEACTIVATE);
        if rv.is_err() {
            dev.set_flags(oflags);
        }
    }
    rv
}

/// `config_defer`: defer the configuration of the specified device until all of its parent's
/// devices have been attached.
pub fn config_defer(dev: &DevRef, d: Deferred) {
    if dev.parent().is_none() {
        kpanic(format_args!(
            "config_defer: can't defer config of a root device"
        ));
    }

    {
        let mut w = TREE.lock();
        if dev.deferred.is_linked() {
            kpanic(format_args!("config_defer: deferred twice"));
        }
        *dev.defer_slot.get_mut(&mut w) = Some((d, dev.clone()));
        DEFERRED_CONFIG_QUEUE.insert_tail(&mut w, dev.as_pin());
    }
    config_pending_incr();
}

/// `config_mountroot`: defer the configuration of the specified device until after root file
/// system is mounted.
pub fn config_mountroot(dev: &DevRef, d: Deferred) {
    // No need to defer if root file system is already mounted.
    if glue().root_mounted() {
        (d.func)(dev, d.arg.as_ref());
        return;
    }

    let mut w = TREE.lock();
    if dev.mountroot.is_linked() {
        kpanic(format_args!("config_mountroot: deferred twice"));
    }
    *dev.mountroot_slot.get_mut(&mut w) = Some((d, dev.clone()));
    MOUNTROOT_CONFIG_QUEUE.insert_tail(&mut w, dev.as_pin());
}

/// One step of `config_process_deferred_children`'s walk, from `from` (the C's `ndc`, taken
/// before the last call; the head when `None` or when that call took it off the queue): the
/// next device deferred below `parent`, off the queue with its record, and the one after it.
fn next_deferred_child(
    parent: &DevRef,
    from: Option<&DevRef>,
) -> Option<(DevRef, Deferred, Option<DevRef>)> {
    let mut w = TREE.lock();
    let (me, next) = {
        let mut e = match from {
            Some(f) if f.deferred.is_linked() => Some(&**f),
            _ => DEFERRED_CONFIG_QUEUE.first(&w),
        };
        while let Some(d) = e {
            if d.is_child_of(parent) {
                break;
            }
            e = DEFERRED_CONFIG_QUEUE.next(&w, d);
        }
        let d = e?;
        let next = DEFERRED_CONFIG_QUEUE
            .next(&w, d)
            .map(|n| parked(&w, n, false));
        (parked(&w, d, false), next)
    };
    DEFERRED_CONFIG_QUEUE.remove(&mut w, &me);
    let (rec, _queue_ref) = me.defer_slot.get_mut(&mut w).take()?;
    Some((me, rec, next))
}

/// `config_process_deferred_children`: process the deferred configuration queue for a
/// device.
pub fn config_process_deferred_children(parent: &DevRef) {
    let mut from: Option<DevRef> = None;
    let mut started = false;
    loop {
        if started && from.is_none() {
            return;
        }
        let Some((dev, d, next)) = next_deferred_child(parent, from.as_ref()) else {
            return;
        };
        started = true;
        (d.func)(&dev, d.arg.as_ref());
        config_pending_decr();
        from = next;
    }
}

/// `config_process_deferred_mountroot`: process the deferred configuration queue after the
/// root file system is mounted.
pub fn config_process_deferred_mountroot() {
    loop {
        let (dev, d) = {
            let mut w = TREE.lock();
            let Some(first) = MOUNTROOT_CONFIG_QUEUE
                .first(&w)
                .map(|d| parked(&w, d, true))
            else {
                return;
            };
            MOUNTROOT_CONFIG_QUEUE.remove(&mut w, &first);
            let Some((d, _queue_ref)) = first.mountroot_slot.get_mut(&mut w).take() else {
                return;
            };
            (first, d)
        };
        (d.func)(&dev, d.arg.as_ref());
    }
}

/// `config_pending_incr`: manipulate the `config_pending` semaphore.
pub fn config_pending_incr() {
    CONFIG_PENDING.fetch_add(1, Ordering::Relaxed);
}

/// `config_pending_decr`.
pub fn config_pending_decr() {
    #[cfg(feature = "diagnostic")]
    if CONFIG_PENDING.load(Ordering::Relaxed) == 0 {
        kpanic(format_args!("config_pending_decr: config_pending == 0"));
    }
    if CONFIG_PENDING.fetch_sub(1, Ordering::Relaxed) == 1 {
        wakeup(ptr::from_ref(&CONFIG_PENDING));
    }
}

/// `config_detach_children`: detaches every child of `parent`, last attached first.
///
/// The `config_detach` routine may sleep, meaning devices may be added to the queue. However,
/// all devices will be added to the tail of the queue, the queue won't be re-organized, and
/// the subtree of parent here should be locked for purposes of adding/removing children.
///
/// Note that we can not afford trying to walk the device list once - our "next" device might
/// be a child of the device we are about to detach, so it would disappear. Just play it safe
/// and restart from the parent.
pub fn config_detach_children(parent: &DevRef, flags: i32) -> Result<(), Errno> {
    let mut dev = alldevs_last();
    while let Some(d) = dev {
        if d.is_child_of(parent) {
            config_detach(d, flags)?;
            dev = alldevs_last();
        } else {
            dev = alldevs_prev(&d);
        }
    }
    Ok(())
}

/// `config_suspend`: hands `act` to the device's `ca_activate`, or to its children when it
/// has none.
pub fn config_suspend(dev: &DevRef, act: i32) -> Result<(), Errno> {
    // The caller's handle is the C's device_ref/device_unref pair around the call.
    match dev.driver().activate(dev, act) {
        Some(r) => r,
        None => config_activate_children(dev, act),
    }
}

/// `config_suspend_all`: `config_suspend` on the `mpath` (none: `NMPATH` is 0) and `mainbus`
/// trees, in the order that suits `act`.
pub fn config_suspend_all(act: i32) -> Result<(), Errno> {
    let mut rv = Ok(());
    let Some(mainbus) = device_mainbus() else {
        return rv;
    };
    match act {
        DVACT_QUIESCE | DVACT_SUSPEND | DVACT_POWERDOWN | DVACT_RESUME | DVACT_WAKEUP => {
            rv = config_suspend(&mainbus, act);
        }
        _ => {}
    }
    rv
}

/// `config_activate_children`: call the `ca_activate` for each of our children, letting each
/// decide whether they wish to do the same for their children and more.
pub fn config_activate_children(parent: &DevRef, act: i32) -> Result<(), Errno> {
    let mut rv = Ok(());

    let mut d = alldevs_next(Some(parent));
    while let Some(dv) = d {
        if !dv.is_child_of(parent) {
            d = alldevs_next(Some(&dv));
            continue;
        }
        match act {
            DVACT_QUIESCE | DVACT_SUSPEND | DVACT_RESUME | DVACT_WAKEUP | DVACT_POWERDOWN => {
                rv = config_suspend(&dv, act);
            }
            DVACT_DEACTIVATE => rv = config_deactivate(&dv),
            _ => {}
        }
        if rv.is_ok() {
            d = alldevs_next(Some(&dv));
            continue;
        }

        // Found a device that refuses the action. If we were being asked to suspend, we can
        // try to resume all previous devices.
        #[cfg(feature = "diagnostic")]
        printf(format_args!(
            "config_activate_children: device {} failed {}\n",
            dv.xname(),
            act
        ));
        if act == DVACT_RESUME {
            printf(format_args!("failing resume cannot be handled\n"));
        }
        if act == DVACT_POWERDOWN {
            return rv;
        }
        if act != DVACT_SUSPEND {
            return rv;
        }

        let mut p = alldevs_prev(&dv);
        while let Some(pd) = p {
            if MArc::ptr_eq(&pd, parent) {
                break;
            }
            if pd.is_child_of(parent) {
                printf(format_args!("resume {}\n", pd.xname()));
                let _ = config_suspend(&pd, DVACT_RESUME);
            }
            p = alldevs_prev(&pd);
        }
        return rv;
    }
    rv
}

/// `device_lookup`: lookup a device in the cfdriver device array. Does not return a device if
/// it is not active.
///
/// The handle is the C's reference on the stack (`device_ref`); its drop is `device_unref`.
///
/// Context: process only
pub fn device_lookup(units: &UnitTable, unit: i32) -> Option<DevRef> {
    let w = TREE.lock();
    let dv = units.get_in(&w, unit)?;
    if dv.flags() & DVF_ACTIVE == 0 {
        return None;
    }
    Some(dv.clone())
}

/// `device_mainbus`: the root bus, once it is attached.
pub fn device_mainbus() -> Option<DevRef> {
    glue().mainbus().get(0)
}

/// Host tests: forgets the tree an earlier test left (its nodes are that test's memory,
/// leaked with it, never dropped into the next) and any attach a failed test left counted,
/// which would make every later `config_detach` sleep on it for ever. The unit table of every
/// attached node's driver is forgotten too, not freed: a `static` driver's table would
/// otherwise free that memory into the next test's when it grows (testing.md). A table grown
/// by a `config_make_softc` whose device never attached is not reachable from here.
#[cfg(any(test, feature = "doubles"))]
pub fn autoconf_test_reset() {
    *ATTDET.lock() = 0;
    let mut w = TREE.lock();
    loop {
        let Some(units) = ALLDEVS.first(&w).map(|d| d.driver().units()) else {
            break;
        };
        units.forget(&mut w);
        ALLDEVS.remove_first(&mut w);
    }
    while DEFERRED_CONFIG_QUEUE.remove_first(&mut w) {}
    while MOUNTROOT_CONFIG_QUEUE.remove_first(&mut w) {}
    CONFIG_PENDING.store(0, Ordering::Relaxed);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for autoconfiguration over a fake machine: a table of fake rows (a root bus
    // `troot0` and children below it), fake drivers that count what they are asked, and a
    // fake glue. Every test gets its own drivers and rows (the state lives in them), under the
    // page system's lock (the tree is a global).

    use std::boxed::Box;
    use std::string::String;
    use std::sync::atomic::{AtomicBool, AtomicI16, AtomicUsize};
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::vec::Vec;
    use std::{assert, assert_eq, format};

    use hal::mem::pool::AnyArc;

    use super::*;
    use crate::dev::device::{DV_DULL, DevRef, Devclass, DriverOps, Ident, QUIET, UNCONF};
    use crate::testutil;

    /// What every test driver records, in order.
    static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
    /// The softcs alive (made and not yet dropped).
    static SOFTCS: AtomicUsize = AtomicUsize::new(0);
    /// The tables the glue lent (one per walk of `cfdata[]`).
    static WALKS: AtomicUsize = AtomicUsize::new(0);

    fn log(s: String) {
        LOG.lock().unwrap_or_else(PoisonError::into_inner).push(s);
    }

    fn take_log() -> Vec<String> {
        core::mem::take(&mut *LOG.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// A test driver's softc: counts itself alive.
    struct Softc;

    impl Drop for Softc {
        fn drop(&mut self) {
            SOFTCS.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// The child's attach arguments: the match level each driver answers.
    struct Args {
        pri: i32,
    }

    struct Driver {
        name: &'static [u8],
        mode: i32,
        /// `cd_devs`, leaked with the driver.
        units: &'static UnitTable,
        /// What this driver adds to the level a bus's `Args` asks for (a root always answers 1).
        bonus: i32,
        /// `ca_detach` succeeds.
        detach_ok: bool,
        /// A suspend of this unit is refused.
        refuse_unit: AtomicI32,
        /// The attach defers the rest of its configuration (`config_defer`).
        defer: AtomicBool,
    }

    fn driver_with(name: &'static [u8], mode: i32, bonus: i32, detach_ok: bool) -> &'static Driver {
        Box::leak(Box::new(Driver {
            name,
            mode,
            units: Box::leak(Box::new(UnitTable::new())),
            bonus,
            detach_ok,
            refuse_unit: AtomicI32::new(-1),
            defer: AtomicBool::new(false),
        }))
    }

    fn new_driver(name: &'static [u8], mode: i32, bonus: i32) -> &'static Driver {
        driver_with(name, mode, bonus, true)
    }

    fn deferred(dev: &DevRef, _arg: Option<&AnyArc>) {
        log(format!("deferred {}", dev.xname()));
    }

    impl DriverOps for Driver {
        fn name(&self) -> &'static [u8] {
            self.name
        }
        fn class(&self) -> Devclass {
            DV_DULL
        }
        fn mode(&self) -> i32 {
            self.mode
        }
        fn units(&self) -> &'static UnitTable {
            self.units
        }
        fn make_softc(&self, id: &Ident<'_>) -> AnyArc {
            assert_eq!(
                cstr(id.xname).len(),
                self.name.len() + format!("{}", id.unit).len()
            );
            SOFTCS.fetch_add(1, Ordering::Relaxed);
            MArc::erase(MArc::new(2, hal::mem::malloc::M_NOWAIT, Softc).expect("softc"))
        }
        fn probe(&self, parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
            if parent.is_none() {
                return 1;
            }
            match aux {
                Aux::Typed(a) => a
                    .downcast_ref::<Args>()
                    .map_or(0, |a| if a.pri == 0 { 0 } else { a.pri + self.bonus }),
                Aux::Legacy(_) => 0,
            }
        }
        fn attach(&self, parent: Option<&DevRef>, dev: &DevRef, _aux: &Aux<'_>) {
            assert!(dev.softc().downcast_ref::<Softc>().is_some());
            log(format!(
                "attach {} at {}",
                dev.xname(),
                parent.map_or("root", |p| p.xname())
            ));
            if self.defer.load(Ordering::Relaxed) {
                config_defer(
                    dev,
                    Deferred {
                        func: deferred,
                        arg: None,
                    },
                );
            }
        }
        fn detach(&self, dev: &DevRef, _flags: i32) -> Result<(), Errno> {
            if !self.detach_ok {
                return Err(Errno::EOPNOTSUPP);
            }
            log(format!("detach {}", dev.xname()));
            Ok(())
        }
        fn activate(&self, dev: &DevRef, act: i32) -> Option<Result<(), Errno>> {
            if dev.unit() == self.refuse_unit.load(Ordering::Relaxed) && act == DVACT_SUSPEND {
                return Some(Err(Errno::EBUSY));
            }
            log(format!("activate {} {act}", dev.xname()));
            Some(Ok(()))
        }
    }

    struct Row {
        drv: &'static Driver,
        parents: &'static [i16],
        starunit1: i16,
        unit: AtomicI16,
        fstate: AtomicI16,
    }

    impl ConfigRow for Row {
        fn driver(&'static self) -> &'static dyn DriverOps {
            self.drv
        }
        fn parents(&self) -> &'static [i16] {
            self.parents
        }
        fn starunit1(&self) -> i16 {
            self.starunit1
        }
        fn unit(&self) -> i16 {
            self.unit.load(Ordering::Relaxed)
        }
        fn set_unit(&self, unit: i16) {
            self.unit.store(unit, Ordering::Relaxed);
        }
        fn fstate(&self) -> i16 {
            self.fstate.load(Ordering::Relaxed)
        }
        fn set_fstate(&self, fstate: i16) {
            self.fstate.store(fstate, Ordering::Relaxed);
        }
    }

    fn row(
        drv: &'static Driver,
        parents: &'static [i16],
        unit: i16,
        fstate: i16,
        starunit1: i16,
    ) -> &'static Row {
        Box::leak(Box::new(Row {
            drv,
            parents,
            starunit1,
            unit: AtomicI16::new(unit),
            fstate: AtomicI16::new(fstate),
        }))
    }

    /// The fake machine: the table of the running test.
    struct Glue {
        rows: Mutex<Vec<&'static Row>>,
        mainbus: Mutex<Option<&'static UnitTable>>,
        boothowto: AtomicI32,
        root_mounted: AtomicBool,
    }

    impl Glue {
        /// The test's row `i`.
        fn row(&self, i: usize) -> &'static Row {
            self.rows.lock().unwrap_or_else(PoisonError::into_inner)[i]
        }
    }

    /// One walk's copy of the test's rows (a walk may nest: a probe or attach that searches).
    struct Rows(Vec<&'static Row>);

    impl RowTable for Rows {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn row(&self, i: usize) -> &'static dyn ConfigRow {
            self.0[i]
        }
    }

    impl ConfigGlue for Glue {
        fn with_rows(&self, f: &mut dyn FnMut(&dyn RowTable)) {
            let rows = Rows(
                self.rows
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone(),
            );
            WALKS.fetch_add(1, Ordering::Relaxed);
            f(&rows);
        }
        fn roots(&self) -> &'static [i16] {
            &[0]
        }
        fn mainbus(&self) -> &'static UnitTable {
            self.mainbus
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .expect("a root driver")
        }
        fn device_register(&self, dev: &DevRef, _aux: &Aux<'_>) {
            log(format!("register {}", dev.xname()));
        }
        fn boothowto(&self) -> i32 {
            self.boothowto.load(Ordering::Relaxed)
        }
        fn root_mounted(&self) -> bool {
            self.root_mounted.load(Ordering::Relaxed)
        }
    }

    static GLUE_DOUBLE: Glue = Glue {
        rows: Mutex::new(Vec::new()),
        mainbus: Mutex::new(None),
        boothowto: AtomicI32::new(0),
        root_mounted: AtomicBool::new(false),
    };

    /// A test's machine: `troot0 at root` (row 0, its driver's `cd_mode` `root_mode`) and the
    /// rows `more` makes below it.
    struct Setup {
        _mem: MutexGuard<'static, ()>,
        root: &'static Driver,
    }

    fn setup(root_name: &'static [u8], root_mode: i32) -> Setup {
        let mem = testutil::MEMORY
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        testutil::install();
        autoconf_test_reset();
        config_init(&GLUE_DOUBLE);
        let root = new_driver(root_name, root_mode, 0);
        *GLUE_DOUBLE
            .rows
            .lock()
            .unwrap_or_else(PoisonError::into_inner) =
            vec_of(&[row(root, &[], 0, FSTATE_NOTFOUND, 0)]);
        *GLUE_DOUBLE
            .mainbus
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(&root.units);
        GLUE_DOUBLE.root_mounted.store(false, Ordering::Relaxed);
        GLUE_DOUBLE.boothowto.store(0, Ordering::Relaxed);
        let _ = take_log();
        Setup { _mem: mem, root }
    }

    fn vec_of(rows: &[&'static Row]) -> Vec<&'static Row> {
        rows.to_vec()
    }

    fn add_row(r: &'static Row) {
        GLUE_DOUBLE
            .rows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(r);
    }

    fn names() -> Vec<String> {
        let mut v = Vec::new();
        let mut d = alldevs_next(None);
        while let Some(x) = d {
            v.push(String::from(x.xname()));
            d = alldevs_next(Some(&x));
        }
        v
    }

    fn args(pri: i32) -> Args {
        Args { pri }
    }

    #[test]
    fn rootfound_attaches_the_named_root_and_prints_it() {
        let s = setup(b"tra", 0);
        let before = testutil::printf_count("root device nosuchroot not configured\n");
        assert!(config_rootfound(b"nosuchroot", &Aux::Legacy(0)).is_none());
        assert_eq!(
            testutil::printf_count("root device nosuchroot not configured\n"),
            before + 1
        );
        let at = testutil::printf_count("tra0 at root");
        let root = config_rootfound(b"tra", &Aux::Legacy(0)).expect("tra0");
        assert_eq!(testutil::printf_count("tra0 at root"), at + 1);
        assert_eq!(root.xname(), "tra0");
        assert!(root.parent().is_none());
        assert!(root.flags() & DVF_ACTIVE != 0);
        assert_eq!(GLUE_DOUBLE.row(0).fstate(), FSTATE_FOUND);
        assert!(s.root.units.get(0).is_some_and(|d| MArc::ptr_eq(&d, &root)));
        assert_eq!(names(), ["tra0"]);
        assert_eq!(take_log(), ["register tra0", "attach tra0 at root"]);
        assert!(device_mainbus().is_some_and(|d| MArc::ptr_eq(&d, &root)));
    }

    #[test]
    fn starred_children_take_the_lowest_free_unit_and_bump_only_at_the_top() {
        let _s = setup(b"trb", 0);
        let child = new_driver(b"tcb", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trb", &Aux::Legacy(0)).expect("trb0");
        let a = args(1);
        let aux = Aux::Typed(&a);
        let c0 = config_found(&root, &aux, None).expect("tcb0");
        let c1 = config_found(&root, &aux, None).expect("tcb1");
        let c2 = config_found(&root, &aux, None).expect("tcb2");
        assert_eq!(
            [c0.xname(), c1.xname(), c2.xname()],
            ["tcb0", "tcb1", "tcb2"]
        );
        assert_eq!(GLUE_DOUBLE.row(1).unit(), 3);
        assert_eq!(GLUE_DOUBLE.row(1).fstate(), FSTATE_STAR);

        // tcb1 goes: its unit is the lowest free one, and cf_unit stays at the top.
        assert_eq!(config_detach(c1, DETACH_QUIET), Ok(()));
        assert_eq!(GLUE_DOUBLE.row(1).unit(), 3);
        let again = config_found(&root, &aux, None).expect("tcb1 again");
        assert_eq!(again.xname(), "tcb1");
        assert_eq!(GLUE_DOUBLE.row(1).unit(), 3, "not the top: no bump");
        assert_eq!(names(), ["trb0", "tcb0", "tcb2", "tcb1"]);
        drop((c0, c2));
    }

    #[test]
    fn an_unmatched_device_is_printed_not_configured() {
        let _s = setup(b"trc", 0);
        let child = new_driver(b"tcc", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trc", &Aux::Legacy(0)).expect("trc0");
        let printed = AtomicUsize::new(0);
        let print = |_: &Aux<'_>, pnp: Option<&[u8]>| {
            printed.fetch_add(1, Ordering::Relaxed);
            assert_eq!(pnp, Some(&b"trc0"[..]));
            UNCONF
        };
        let a = args(0);
        let before = testutil::printf_count(" not configured\n");
        assert!(config_found(&root, &Aux::Typed(&a), Some(&print)).is_none());
        assert_eq!(printed.load(Ordering::Relaxed), 1);
        assert_eq!(testutil::printf_count(" not configured\n"), before + 1);
        // A configured device's print is told so (pnp NULL), its result ignored.
        let a = args(1);
        let quiet = |_: &Aux<'_>, pnp: Option<&[u8]>| {
            assert!(pnp.is_none());
            QUIET
        };
        assert!(config_found(&root, &Aux::Typed(&a), Some(&quiet)).is_some());
    }

    #[test]
    fn the_best_probe_wins_and_a_found_row_is_not_tried_again() {
        let _s = setup(b"trd", 0);
        let weak = new_driver(b"tdw", 0, 0);
        let strong = new_driver(b"tds", 0, 5);
        add_row(row(weak, &[0], 0, FSTATE_NOTFOUND, 0));
        add_row(row(strong, &[0], 0, FSTATE_NOTFOUND, 0));
        let root = config_rootfound(b"trd", &Aux::Legacy(0)).expect("trd0");
        let a = args(1);
        let aux = Aux::Typed(&a);
        assert_eq!(
            config_found(&root, &aux, None).expect("tds0").xname(),
            "tds0"
        );
        assert_eq!(GLUE_DOUBLE.row(2).fstate(), FSTATE_FOUND);
        // The FOUND row is skipped: the weaker one is all that is left.
        assert_eq!(
            config_found(&root, &aux, None).expect("tdw0").xname(),
            "tdw0"
        );
        assert!(config_found(&root, &aux, None).is_none());
        // A submatch decides instead of the probes.
        let sub = |_: Option<&DevRef>, _: &Match, _: &Aux<'_>| 0;
        assert!(config_search(Some(&sub), &root, &aux).is_none());
    }

    #[test]
    fn an_indirect_bus_matches_against_softcs_and_drops_the_losers() {
        let _s = setup(b"tre", CD_INDIRECT);
        let child = new_driver(b"tce", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"tre", &Aux::Legacy(0)).expect("tre0");
        let alive = SOFTCS.load(Ordering::Relaxed);
        let a = args(3);
        let m = config_search(None, &root, &Aux::Typed(&a)).expect("a match");
        assert!(matches!(m, Match::Softc(_)));
        assert_eq!(SOFTCS.load(Ordering::Relaxed), alive + 1);
        let c = config_attach(Some(&root), m, &Aux::Typed(&a), None);
        assert_eq!(c.xname(), "tce0");
        assert!(child.units.get(0).is_some());
        // A probe that does not match: its softc is made and dropped.
        let no = args(0);
        assert!(config_search(None, &root, &Aux::Typed(&no)).is_none());
        assert_eq!(SOFTCS.load(Ordering::Relaxed), alive + 1);
        assert!(child.units.get(1).is_none());
        // config_scan hands every candidate over, owned.
        let seen = AtomicUsize::new(0);
        config_scan(
            &|p: &DevRef, m: Match| {
                assert!(MArc::ptr_eq(p, &root));
                assert!(matches!(m, Match::Softc(_)));
                seen.fetch_add(1, Ordering::Relaxed);
            },
            &root,
        );
        assert_eq!(seen.load(Ordering::Relaxed), 1);
        assert_eq!(SOFTCS.load(Ordering::Relaxed), alive + 1);
    }

    #[test]
    fn attach_runs_in_the_c_order() {
        let _s = setup(b"trf", 0);
        let child = new_driver(b"tcf", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trf", &Aux::Legacy(0)).expect("trf0");
        let at = testutil::printf_count("tcf0 at trf0");
        let a = args(1);
        let print = |_: &Aux<'_>, _: Option<&[u8]>| {
            log(String::from("print"));
            QUIET
        };
        let _c = config_found(&root, &Aux::Typed(&a), Some(&print)).expect("tcf0");
        assert_eq!(testutil::printf_count("tcf0 at trf0"), at + 1);
        assert_eq!(
            take_log(),
            [
                "register trf0",
                "attach trf0 at root",
                "print",
                "register tcf0",
                "attach tcf0 at trf0"
            ]
        );
    }

    #[test]
    fn detach_gives_the_unit_back_and_frees_the_table() {
        let s = setup(b"trg", 0);
        let child = new_driver(b"tcg", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trg", &Aux::Legacy(0)).expect("trg0");
        let a = args(1);
        let c0 = config_found(&root, &Aux::Typed(&a), None).expect("tcg0");
        let c1 = config_found(&root, &Aux::Typed(&a), None).expect("tcg1");
        let serial = AUTOCONF_SERIAL.load(Ordering::Relaxed);
        let detached = testutil::printf_count("tcg1 detached\n");
        assert_eq!(config_detach(c1, 0), Ok(()));
        assert_eq!(testutil::printf_count("tcg1 detached\n"), detached + 1);
        assert_eq!(
            GLUE_DOUBLE.row(1).unit(),
            1,
            "the last starred unit is reused"
        );
        assert_eq!(AUTOCONF_SERIAL.load(Ordering::Relaxed), serial + 1);
        assert!(child.units.get(1).is_none());

        // The children go last attached first; the last one frees the table.
        drop(c0);
        let alive = SOFTCS.load(Ordering::Relaxed);
        assert_eq!(config_detach_children(&root, DETACH_QUIET), Ok(()));
        assert_eq!(names(), ["trg0"]);
        assert!(child.units.is_empty(), "the empty cd_devs array is freed");
        assert_eq!(
            SOFTCS.load(Ordering::Relaxed),
            alive - 1,
            "tcg0's softc went"
        );
        assert_eq!(
            take_log()
                .iter()
                .filter(|l| l.starts_with("detach"))
                .count(),
            2
        );
        assert!(s.root.units.get(0).is_some());
    }

    #[test]
    fn a_refused_detach_leaves_the_device_deactivated() {
        let _s = setup(b"trh", 0);
        let child = driver_with(b"tch", 0, 0, false);
        add_row(row(child, &[0], 0, FSTATE_NOTFOUND, 0));
        let root = config_rootfound(b"trh", &Aux::Legacy(0)).expect("trh0");
        let a = args(1);
        let c = config_found(&root, &Aux::Typed(&a), None).expect("tch0");
        assert_eq!(
            config_detach(c.clone(), DETACH_QUIET),
            Err(Errno::EOPNOTSUPP)
        );
        assert!(c.flags() & DVF_ACTIVE == 0);
        assert_eq!(names(), ["trh0", "tch0"]);
        assert_eq!(GLUE_DOUBLE.row(1).fstate(), FSTATE_FOUND);
    }

    #[test]
    fn a_handle_held_across_a_detach_keeps_the_node_and_lookup_refuses_it() {
        let _s = setup(b"tri", 0);
        let child = new_driver(b"tci", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"tri", &Aux::Legacy(0)).expect("tri0");
        let a = args(1);
        let c = config_found(&root, &Aux::Typed(&a), None).expect("tci0");
        let held = device_lookup(&child.units, 0).expect("active");
        assert!(MArc::ptr_eq(&held, &c));
        assert!(device_lookup(&child.units, 1).is_none());
        assert!(device_lookup(&child.units, -1).is_none());
        let alive = SOFTCS.load(Ordering::Relaxed);
        assert_eq!(config_detach(c, DETACH_QUIET), Ok(()));
        assert_eq!(
            SOFTCS.load(Ordering::Relaxed),
            alive,
            "held: the softc stays"
        );
        assert_eq!(held.xname(), "tci0");
        assert!(device_lookup(&child.units, 0).is_none());
        drop(held);
        assert_eq!(SOFTCS.load(Ordering::Relaxed), alive - 1);

        // An inactive device is not looked up.
        let c = config_found(&root, &Aux::Typed(&a), None).expect("tci0 again");
        assert_eq!(config_deactivate(&c), Ok(()));
        assert!(device_lookup(&child.units, 0).is_none());
    }

    #[test]
    fn deferred_children_run_after_their_parent_in_order_once() {
        let _s = setup(b"trj", 0);
        let bus = new_driver(b"tbj", 0, 0);
        let leaf = new_driver(b"tlj", 0, 0);
        leaf.defer.store(true, Ordering::Relaxed);
        add_row(row(bus, &[0], 0, FSTATE_NOTFOUND, 0));
        add_row(row(leaf, &[1], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trj", &Aux::Legacy(0)).expect("trj0");
        let a = args(1);
        let aux = Aux::Typed(&a);

        // The bus's attach finds two leaves, which defer; they run once the bus is attached.
        let busdev = config_found(&root, &aux, None).expect("tbj0");
        let l0 = config_found(&busdev, &aux, None).expect("tlj0");
        let l1 = config_found(&busdev, &aux, None).expect("tlj1");
        assert_eq!(CONFIG_PENDING.load(Ordering::Relaxed), 2);
        let _ = take_log();
        config_process_deferred_children(&l0);
        assert!(take_log().is_empty(), "not a child of itself");
        config_process_deferred_children(&busdev);
        assert_eq!(take_log(), ["deferred tlj0", "deferred tlj1"]);
        assert_eq!(CONFIG_PENDING.load(Ordering::Relaxed), 0);
        config_process_deferred_children(&busdev);
        assert!(take_log().is_empty(), "once");
        drop(l1);
    }

    #[test]
    fn mountroot_waits_for_the_root_file_system() {
        let _s = setup(b"trk", 0);
        let child = new_driver(b"tck", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trk", &Aux::Legacy(0)).expect("trk0");
        let a = args(1);
        let c = config_found(&root, &Aux::Typed(&a), None).expect("tck0");
        let _ = take_log();
        let d = || Deferred {
            func: deferred,
            arg: None,
        };
        config_mountroot(&c, d());
        assert!(take_log().is_empty(), "no root file system yet");
        config_process_deferred_mountroot();
        assert_eq!(take_log(), ["deferred tck0"]);
        GLUE_DOUBLE.root_mounted.store(true, Ordering::Relaxed);
        config_mountroot(&c, d());
        assert_eq!(take_log(), ["deferred tck0"], "run at once");
    }

    #[test]
    fn a_refused_suspend_resumes_the_earlier_siblings_last_first() {
        let _s = setup(b"trl", 0);
        let child = new_driver(b"tcl", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trl", &Aux::Legacy(0)).expect("trl0");
        let a = args(1);
        let kids: Vec<DevRef> = (0..3)
            .map(|_| config_found(&root, &Aux::Typed(&a), None).expect("a child"))
            .collect();
        let _ = take_log();
        child.refuse_unit.store(2, Ordering::Relaxed);
        // The root has its own activate in these fakes: walk the children directly.
        assert_eq!(
            config_activate_children(&root, DVACT_SUSPEND),
            Err(Errno::EBUSY)
        );
        assert_eq!(
            take_log(),
            [
                format!("activate tcl0 {DVACT_SUSPEND}"),
                format!("activate tcl1 {DVACT_SUSPEND}"),
                format!("activate tcl1 {DVACT_RESUME}"),
                format!("activate tcl0 {DVACT_RESUME}"),
            ]
        );
        child.refuse_unit.store(-1, Ordering::Relaxed);
        assert_eq!(config_activate_children(&root, DVACT_SUSPEND), Ok(()));
        assert_eq!(config_suspend_all(DVACT_SUSPEND), Ok(()));
        drop(kids);
    }

    #[test]
    fn attach_by_hand_sizes_the_table_exactly_and_prints_nothing() {
        let _s = setup(b"trm", 0);
        let rd = new_driver(b"trd", 0, 0);
        let r = row(rd, &[], 0, FSTATE_NOTFOUND, 0);
        let at = testutil::printf_count("trd0 at");
        let dev = config_attach_by_hand(r, 0, &Aux::Legacy(0));
        assert_eq!(testutil::printf_count("trd0 at"), at);
        assert_eq!(dev.xname(), "trd0");
        assert_eq!(rd.units.len(), 1);
        assert_eq!(r.fstate(), FSTATE_NOTFOUND);
        assert_eq!(names(), ["trd0"]);
        assert_eq!(take_log(), ["attach trd0 at root"]);
    }

    #[test]
    fn a_search_borrows_the_table_once() {
        let _s = setup(b"tro", 0);
        let child = new_driver(b"tco", 0, 0);
        for _ in 0..8 {
            add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        }
        let root = config_rootfound(b"tro", &Aux::Legacy(0)).expect("tro0");
        let a = args(0);
        let before = WALKS.load(Ordering::Relaxed);
        assert!(config_search(None, &root, &Aux::Typed(&a)).is_none());
        assert_eq!(
            WALKS.load(Ordering::Relaxed),
            before + 1,
            "one walk, nine rows"
        );
    }

    /// Runs `f` on a thread of its own (not cold, so it really sleeps), and says whether it
    /// was still running after `ms` milliseconds.
    fn blocked_for<T: Send + 'static>(
        ms: u64,
        f: impl FnOnce() -> T + Send + 'static,
    ) -> (bool, std::thread::JoinHandle<T>) {
        let done = std::sync::Arc::new(AtomicBool::new(false));
        let d = done.clone();
        let h = std::thread::spawn(move || {
            testutil::set_cold(false);
            let r = f();
            d.store(true, Ordering::SeqCst);
            r
        });
        std::thread::sleep(std::time::Duration::from_millis(ms));
        (!done.load(Ordering::SeqCst), h)
    }

    #[test]
    fn an_attach_waits_for_a_detach_in_flight() {
        let _s = setup(b"trp", 0);
        // A detach in flight (`config_detach` between its two halves).
        attdet_enter(false);
        let (waited, h) = blocked_for(50, || {
            config_rootfound(b"trp", &Aux::Legacy(0)).map(|d| String::from(d.xname()))
        });
        assert!(waited, "the attach waits");
        assert_eq!(names(), Vec::<String>::new());
        attdet_leave(false);
        assert_eq!(h.join().expect("the attach"), Some(String::from("trp0")));
        assert_eq!(*ATTDET.lock(), 0);
    }

    #[test]
    fn a_detach_waits_for_an_attach_in_flight() {
        let _s = setup(b"trq", 0);
        let child = new_driver(b"tcq", 0, 0);
        add_row(row(child, &[0], 0, FSTATE_STAR, 0));
        let root = config_rootfound(b"trq", &Aux::Legacy(0)).expect("trq0");
        let a = args(1);
        let c = config_found(&root, &Aux::Typed(&a), None).expect("tcq0");
        // An attach in flight.
        attdet_enter(true);
        let (waited, h) = blocked_for(50, move || config_detach(c, DETACH_QUIET));
        assert!(waited, "the detach waits");
        assert!(child.units.get(0).is_some(), "still attached");
        attdet_leave(true);
        assert_eq!(h.join().expect("the detach"), Ok(()));
        assert!(child.units.get(0).is_none());
        assert_eq!(*ATTDET.lock(), 0);
    }

    #[test]
    fn config_pending_wakes_its_sleeper_at_zero() {
        let _s = setup(b"trr", 0);
        config_pending_incr();
        config_pending_incr();
        let asleep = std::sync::Arc::new(AtomicBool::new(false));
        let s = asleep.clone();
        let (waited, h) = blocked_for(0, move || {
            // init_main's wait for the deferred configurations, with a bound for the test.
            let mut woken = 0;
            while CONFIG_PENDING.load(Ordering::SeqCst) != 0 {
                s.store(true, Ordering::SeqCst);
                if hal::sync::sleep::tsleep_nsec(
                    ptr::from_ref(&CONFIG_PENDING),
                    PWAIT,
                    "cfpend",
                    5_000_000_000,
                )
                .is_ok()
                {
                    woken += 1;
                }
            }
            woken
        });
        let _ = waited;
        while !asleep.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        config_pending_decr();
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(!h.is_finished(), "1 left: no wakeup");
        config_pending_decr();
        assert_eq!(h.join().expect("the sleeper"), 1, "woken once, at zero");
    }

    #[test]
    fn the_reset_forgets_the_unit_tables() {
        let s = setup(b"trs", 0);
        let _root = config_rootfound(b"trs", &Aux::Legacy(0)).expect("trs0");
        assert_eq!(s.root.units.len(), 2);
        autoconf_test_reset();
        assert!(s.root.units.is_empty(), "forgotten, not freed");
        assert!(alldevs_next(None).is_none());
    }
}
/* </TESTS> */
