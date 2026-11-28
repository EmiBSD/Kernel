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
//! Autoconfiguration subroutines: `kern/subr_autoconf.c`, see `autoconf(9)`,
//! `config_attach(9)`, `config_defer(9)`, `config_found(9)`, `device_lookup(9)`.
//!
//! Upstream: sys/kern/subr_autoconf.c @ 3ce1f3f79392
//!
//! Since N7.1a the device tree is the kernel's (`kernel::dev::autoconf`): this file is the
//! shim `bsd`'s drivers reach it through, every name and signature LZ's, carrying no logic.
//! A legacy driver's `Cfdata` is both a kernel [`ConfigRow`] and a [`DriverOps`] (a row is
//! its own driver object): its probe, attach, detach and activate call the `Cfattach`'s
//! functions with the legacy header, and the softc it makes is the zeroed block of
//! `ca_devsize` bytes LZ made, its header written from the kernel's [`Ident`] and held by the
//! node through a [`LegacySoftc`] (one `dv_ref`). A legacy `&Device` names its node through
//! `cf_driver.cd_units` and `dv_unit`; `void *aux` crosses as [`Aux::Legacy`]; the machine's
//! table, `device_register`, `boothowto` and `rootvp` are the kernel's [`ConfigGlue`]
//! (`bridges.toml`).
//!
//! Since N7.2b1a the tree may mix both kinds of driver. A `cfdata[]` row may name a native
//! driver (`Cfdata::native`): the row's [`ConfigRow::driver`] is then the kernel's. A native
//! node that legacy code names (the parent of a legacy child, a native root) lends it a
//! shadow header: a legacy header written from the node, made the first time, held by the
//! node ([`Node::legacy_shadow`]) as an owning `LegacySoftc`, so a legacy child's
//! `dv_parent` and `node_of` work as for a legacy parent. A native bus's typed argument
//! ([`Aux::Typed`]) reaches a legacy driver's match, attach and the machine's
//! `device_register` through the machine's presenter (`machine::autoconf::present_aux`):
//! a type no presenter knows matches no legacy row. All of it is scaffolding
//! (docs/SCAFFOLDING.md), gone with the last legacy driver.
//!
//! ## Deviations
//! - `cfdata[]`, `cfroots[]` and `mainbus_cd` come from the machine
//!   (`machine::autoconf`, written by hand in `sys/arch/<arch>/conf/ioconf.rs` instead of by
//!   `config(8)`); `device_register` is the machine's too, as in C.
//! - `config_attach` tells a preallocated softc from a `cfdata` by the [`CfMatch`] it is given
//!   instead of by the parent's `CD_INDIRECT`, which is what produced that variant.
//! - Devices are `NonNull<Device>` where the C may free them (`config_attach`'s result,
//!   `config_detach`, `device_unref`, `device_lookup`), so no reference outlives the softc;
//!   `config_detach` and `device_unref` are `unsafe` for that reason.
//! - `dv_ref` counts the legacy holders and the node's one reference: after an attach it is 1
//!   where the C's is 2 (`config_make_softc`'s and `config_attach`'s are the node's handles,
//!   the unit table's among them); the block goes when both counts are gone.
//! - A device made by hand (a test's parent, never `config_make_softc`'s) has no node: each
//!   call that names it makes a shadow node for the call, not in the tree, which gives no
//!   reference back.
//! - `parsedisk` returns the device it names as `&'static Device` (the C's pointer kept as
//!   `bootdv`): it takes one `dv_ref` it never gives back ([`device_hold`]), so the block is
//!   never freed (docs/SCAFFOLDING.md).
//! - `NMPATH` is 0: `mpath(4)` is not in the machines' `ioconf.rs` (`scsi/mpath.c` is not
//!   ported), so `device_mpath` returns `None`, as the C compiles it with a count of 0.
//! - The `hotplug(4)` gap and `autoconf_verbose` are the kernel's (`kernel::dev::autoconf`).

use core::any::Any;
use core::ffi::c_void;
use core::ptr::{self, NonNull};
use core::sync::atomic::Ordering;

use hal::mem::malloc::MArc;
use hal::mem::pool::AnyArc;
use kernel::dev::autoconf::{self as kac, ConfigGlue, RowTable};
pub use kernel::dev::autoconf::{AUTOCONF_SERIAL, AUTOCONF_VERBOSE, CONFIG_PENDING};
use kernel::dev::device::{
    Aux, ConfigRow, Deferred, DevRef, Device as Node, DriverOps, Ident, Match, UnitTable,
};

use crate::kassert;
use crate::kern::init_main::{BOOTHOWTO, rootvp};
use crate::kern::kern_malloc::{free, malloc, mallocarray};
use crate::kern::subr_prf::{Str, panic};
use crate::machine::autoconf::{
    cfdata, cfroots, device_register, mainbus_cd, present_aux as machine_present_aux,
};
use crate::sys::device::{
    CfMatch, Cfdata, Cfdriver, CfmatchT, CfprintT, CfscanT, DVF_ACTIVE, Devclass, Device,
    SoftcMatch,
};
use crate::sys::errno::Errno;
use crate::sys::malloc::{M_DEVBUF, M_NOWAIT, M_ZERO};

/// The softc a legacy driver's node holds: the `struct device` heading the block LZ's
/// `config_make_softc` allocates, and one `dv_ref` on it (`owns`), given back by the drop.
/// A native node's shadow header (`Node::legacy_shadow`) is an owning one too, over a block
/// of the header alone. A shadow node (`owns == false`) names a device made by hand and
/// holds nothing.
struct LegacySoftc {
    /// The header.
    dev: NonNull<Device>,
    /// The node holds one `dv_ref` on the block.
    owns: bool,
}

// SAFETY: a `LegacySoftc` only names the block; the block's `Cell`s are read and written by
// the legacy drivers and this shim under the kernel lock, as LZ's `unsafe impl Sync for
// Cfdriver`/`Cfdata` rest on, and `dv_ref` is atomic. Moving the name to another CPU moves no
// access with it.
unsafe impl Send for LegacySoftc {}
// SAFETY: as for `Send`: a shared `LegacySoftc` hands out only the shared header
// (`legacy_device`), whose members are `Cell`s touched under the kernel lock and an atomic.
unsafe impl Sync for LegacySoftc {}

impl Drop for LegacySoftc {
    fn drop(&mut self) {
        if self.owns {
            // SAFETY: an owning `LegacySoftc` holds one `dv_ref` on a live block (taken when
            // the block was made, or handed over by a `SoftcMatch`), given back here once.
            unsafe { device_unref(self.dev) };
        }
    }
}

/// A legacy driver's `fn(&Device)` deferred by `config_defer`/`config_mountroot`, the
/// kernel's [`Deferred::arg`].
struct LegacyFn(fn(&Device));

/// The machine's table and the state autoconfiguration reads, for the kernel.
struct BsdConfigGlue;

/// `cfdata[]` up to its first free slot, for one walk of the kernel's.
struct CfTable(&'static [Cfdata]);

static BSD_GLUE: BsdConfigGlue = BsdConfigGlue;

/// The legacy header of a node: a legacy driver's own (its softc), or, for a native node
/// that legacy code names (the parent of a legacy driver below a native bus, a native root
/// `config_rootfound` returns), its shadow header, made on first use and held by the node.
pub fn legacy_device(n: &Node) -> &Device {
    if let Some(ls) = n.softc().downcast_ref::<LegacySoftc>() {
        return header(ls);
    }
    let dev = header(shadow(n));
    // The node's flags are the truth (`config_deactivate`); the shadow's are a copy.
    dev.dv_flags.set(n.flags());
    dev
}

/// The header a `LegacySoftc` names, borrowed for as long as the `LegacySoftc`.
fn header(ls: &LegacySoftc) -> &Device {
    // SAFETY: `ls.dev` heads a live block for as long as `ls` lives, and the reference
    // borrows `ls`. An owning `LegacySoftc` (a legacy driver's softc, or a native node's
    // shadow header, which lives in the node's `legacy_shadow` slot) holds a `dv_ref`, and
    // `device_unref`, the only place a softc block is freed, frees only at 0. A shadow node
    // (`owns == false`, a device made by hand, `node_of`) is held by two kinds of handle
    // only: the shim call that made it from the caller's `&Device`, which keeps the device
    // alive for the call, and the nodes of the children attached below it in such a call
    // (their `dv_parent`). Those children are `bsd`'s, whose nodes `bsd` holds only in the
    // unit table and for the length of a shim call, so a child's node goes with its detach;
    // and a parent is not freed while it has attached children, the premise LZ's
    // `Device::parent` already rests on (its owner detaches them first; a native child holds
    // its parent's node, and with it the parent's shadow header). Only shared references to
    // the header are made (its members are `Cell`s and an atomic).
    unsafe { ls.dev.as_ref() }
}

/// A native node's shadow header: the legacy header written from the node, in a block of
/// the header alone (`CFATTACH_NATIVE`'s `ca_devsize`), held by the node (`legacy_shadow`).
/// Its row is the node's (`cfdata_of`: a native row is a `Cfdata`), its parent the parent's
/// legacy header. Two makers racing would both find the slot empty: the loser's `set` fails
/// (its block freed with its `LegacySoftc`), and if its `get` falls in the winner's window
/// between taking the slot and publishing it (`OnceStatic`'s BUSY), it answers `None` and the
/// loser panics with "has no legacy header". The kernel lock rules the race out: every
/// caller of the shim (autoconfiguration, the legacy drivers) runs under it.
fn shadow(n: &Node) -> &LegacySoftc {
    let slot = n.legacy_shadow();
    if !slot.is_set() {
        let id = Ident {
            class: n.class(),
            unit: n.unit(),
            xname: n.xname_bytes(),
            row: n.row(),
            parent: n.parent(),
        };
        let ls = LegacySoftc {
            dev: new_header(&id, n.flags()),
            owns: true,
        };
        // A header made beside another's would be dropped here, its block freed with it.
        let _ = slot.set(new_softc(ls));
    }
    match slot.get().and_then(|s| s.downcast_ref::<LegacySoftc>()) {
        Some(ls) => ls,
        None => panic(format_args!("autoconf: {} has no legacy header", n.xname())),
    }
}

/// `parsedisk`'s device, kept for ever: one `dv_ref` taken and never given back, so the
/// block is never freed (the boot device the C keeps as a pointer).
pub fn device_hold(n: &Node) -> &'static Device {
    let dv = legacy_device(n);
    device_ref(dv);
    // SAFETY: the reference just taken is never given back, so `dv_ref` never reaches 0 and
    // `device_unref`, the only place the block is freed, never frees it: it lives for the
    // rest of the kernel's life. Only shared references to the header are made.
    unsafe { NonNull::from(dv).as_ref() }
}

/// A kernel node's handle for the legacy `cfdata` row.
fn cfdata_of(row: &'static dyn ConfigRow) -> &'static Cfdata {
    let any: &'static dyn Any = row;
    match any.downcast_ref::<Cfdata>() {
        Some(cf) => cf,
        None => panic(format_args!("autoconf: a native row for a legacy driver")),
    }
}

/// LZ's `config_make_softc`'s block for the device `id` describes: its row's `ca_devsize`
/// zeroed bytes (a legacy softc, or a native node's shadow header: `CFATTACH_NATIVE`'s, the
/// header alone), the header written from the identity with `flags`, `dv_ref` 1 (the
/// caller's `LegacySoftc`'s), freed by `device_unref` with the same size.
fn new_header(id: &Ident<'_>, flags: i32) -> NonNull<Device> {
    let cf = cfdata_of(id.row);
    let ca = cf.cf_attach;
    if ca.ca_devsize < size_of::<Device>() {
        panic(format_args!("config_make_softc"));
    }

    // get memory for all device vars
    let Some(mem) = malloc(ca.ca_devsize, M_DEVBUF, M_NOWAIT | M_ZERO) else {
        panic(format_args!(
            "config_make_softc: allocation for device softc failed"
        ));
    };
    let devp = mem.cast::<Device>();
    kassert!(devp.as_ptr().is_aligned());
    // SAFETY: `ca_devsize` zeroed bytes, at least a `Device`, suitably aligned (malloc's
    // chunks are aligned to their power-of-two size); all-zero is a valid `Device`.
    let dev = unsafe { devp.as_ref() };

    dev.dv_class.set(id.class);
    dev.dv_cfdata.set(Some(cf));
    dev.dv_flags.set(flags);
    dev.dv_unit.set(id.unit);
    dev.dv_xname.set(*id.xname);
    dev.dv_parent
        .set(id.parent.map(|p| NonNull::from(legacy_device(p))));
    dev.dv_ref.store(1, Ordering::Relaxed);
    devp
}

/// `void *aux` for the kernel.
fn aux_of(aux: *mut c_void) -> Aux<'static> {
    Aux::Legacy(aux.expose_provenance())
}

/// The one door through which a native bus's typed argument reaches a legacy driver: `f`
/// gets the legacy argument (a `void *` valid for the call), once, and the answer is whether
/// it ran. The machine's presenter is tried first (`machine::autoconf::present_aux`: the
/// FDT's, arm64's `FdtArgs` as `FdtAttachArgs`, N7.2b1b); a machine-independent bus's
/// presenter (virtio N7.3a, scsibus N7.4a, pci N7.5) is added here, after it, in the slice
/// that makes that bus native: a direct call, no registration, no list before its first
/// entry. Scaffolding (docs/SCAFFOLDING.md).
fn present_aux(aux: &(dyn Any + Send + Sync), f: &mut dyn FnMut(*mut c_void)) -> bool {
    let mut ran = false;
    machine_present_aux(aux, &mut |a| {
        if !ran {
            ran = true;
            f(a);
        }
    });
    ran
}

/// The kernel's `aux` lent to a legacy driver's function as its `void *`: a legacy bus's
/// as it is, a native bus's typed argument through the shim's door ([`present_aux`]), valid
/// for the call. `None` when no presenter lent it (the legacy row cannot be that bus's
/// child).
fn with_aux<R>(aux: &Aux<'_>, f: impl FnOnce(*mut c_void) -> R) -> Option<R> {
    match aux {
        Aux::Legacy(a) => Some(f(ptr::with_exposed_provenance_mut(*a))),
        Aux::Typed(t) => {
            let mut f = Some(f);
            let mut out = None;
            let lent = present_aux(*t, &mut |a| {
                if let Some(f) = f.take() {
                    out = Some(f(a));
                }
            });
            if lent { out } else { None }
        }
    }
}

/// A native bus's argument that no presenter knows, at a legacy driver's attach: chosen by
/// the bus's own submatch (its probe answers 0 for such an argument), the driver cannot read
/// it.
fn unpresented(what: &str, name: &[u8]) -> ! {
    panic(format_args!(
        "autoconf: {what} of '{}': a native bus's arguments no presenter knows",
        Str(name)
    ))
}

/// The erased softc of a legacy node.
fn new_softc(ls: LegacySoftc) -> AnyArc {
    match MArc::new(M_DEVBUF, hal::mem::malloc::M_NOWAIT, ls) {
        Some(a) => MArc::erase(a),
        None => panic(format_args!(
            "config_make_softc: allocation for device softc failed"
        )),
    }
}

/// The node of a legacy device: the one in its driver's unit table, or, for a device made by
/// hand, a shadow for the call (the module's deviations).
pub(crate) fn node_of(dev: &Device) -> DevRef {
    let cf = dev.cfdata();
    if let Some(n) = ConfigRow::driver(cf).units().get(dev.dv_unit.get())
        && ptr::eq(legacy_device(&n), dev)
    {
        return n;
    }
    let xname = dev.dv_xname.get();
    let id = Ident {
        class: dev.dv_class.get(),
        unit: dev.dv_unit.get(),
        xname: &xname,
        row: cf,
        parent: None,
    };
    kac::device_adopt(
        &id,
        dev.dv_flags.get(),
        new_softc(LegacySoftc {
            dev: NonNull::from(dev),
            owns: false,
        }),
    )
}

/// A legacy softc's header: the block `make_softc` made for a legacy row, which a legacy
/// caller may cast to its softc. `None` for a native node: its softc is its driver's, and
/// its shadow header is a block of the header alone, no legacy softc.
fn legacy_softc(d: &Node) -> Option<&Device> {
    d.softc().downcast_ref::<LegacySoftc>().map(header)
}

/// [`legacy_softc`] for a preallocated softc handed to legacy code (an indirect bus's
/// match): a native node's is refused. Only a legacy indirect bus makes one (isa, amd64's).
fn claimed_softc<'a>(d: &'a Node, what: &str) -> &'a Device {
    match legacy_softc(d) {
        Some(dv) => dv,
        None => panic(format_args!(
            "autoconf: {}'s native softc for a legacy indirect bus's {what}",
            d.xname()
        )),
    }
}

/// The legacy view of a match, lent to a legacy match function for the call.
fn view(m: &Match) -> CfMatch {
    match m {
        Match::Row(row) => CfMatch::Cfdata(cfdata_of(*row)),
        Match::Softc(d) => {
            let p = NonNull::from(claimed_softc(d, "match"));
            // SAFETY: `SoftcMatch::new`'s contract, a live softc made by `config_make_softc`
            // (`make_softc`'s block for a legacy row: `claimed_softc` refuses a native node,
            // whose shadow header is no softc) that outlives the value: the value is only lent
            // (`&CfMatch`) to a legacy match function for the call, while `d` holds the node
            // and its `dv_ref`; a `SoftcMatch` has no drop, so this view frees nothing when
            // it goes.
            CfMatch::Softc(unsafe { SoftcMatch::new(p) })
        }
    }
}

/// A preallocated softc handed to a legacy caller, who owns it from now on (it attaches it
/// or `free`s it, as the C's): one `dv_ref` taken for the claim before the node's goes.
fn release_softc(d: DevRef) -> SoftcMatch {
    let dv = claimed_softc(&d, "claim");
    device_ref(dv);
    let p = NonNull::from(dv);
    drop(d);
    // SAFETY: the reference `device_ref` took above is the claim: `dv_ref` stays at least 1
    // once the node's goes with `d`, so the block lives until the claim's holder attaches it
    // (`adopt` hands the reference to the new node) or frees it (`free`, as the C's loser):
    // `SoftcMatch::new`'s contract.
    unsafe { SoftcMatch::new(p) }
}

/// A kernel match handed to a legacy caller (`config_search`, `config_scan`).
fn release(m: Match) -> CfMatch {
    match m {
        Match::Row(row) => CfMatch::Cfdata(cfdata_of(row)),
        Match::Softc(d) => CfMatch::Softc(release_softc(d)),
    }
}

/// A legacy match given to the kernel: a preallocated softc becomes a node again, taking
/// over the claim's `dv_ref`.
fn adopt(m: CfMatch, parent: Option<&DevRef>) -> Match {
    match m {
        CfMatch::Cfdata(cf) => Match::Row(cf),
        CfMatch::Softc(sc) => {
            let dv = sc.device();
            let xname = dv.dv_xname.get();
            let id = Ident {
                class: dv.dv_class.get(),
                unit: dv.dv_unit.get(),
                xname: &xname,
                row: dv.cfdata(),
                parent,
            };
            let flags = dv.dv_flags.get();
            let softc = new_softc(LegacySoftc {
                dev: sc.into_raw(),
                owns: true,
            });
            Match::Softc(kac::device_adopt(&id, flags, softc))
        }
    }
}

/// A legacy submatch for the kernel.
fn submatch(f: CfmatchT, aux: *mut c_void) -> impl Fn(Option<&DevRef>, &Match, &Aux<'_>) -> i32 {
    move |parent, m, _| f(parent.map(|p| legacy_device(p)), &view(m), aux)
}

/// A legacy print function for the kernel.
fn printer(f: CfprintT, aux: *mut c_void) -> impl Fn(&Aux<'_>, Option<&[u8]>) -> i32 {
    move |_, pnp| f(aux, pnp)
}

/// `cd->cd_devs[unit] = dev`: the mirror of the kernel's unit table.
fn mirror_set(cd: &Cfdriver, unit: i32, dev: Option<NonNull<Device>>) {
    if unit < 0 || unit >= cd.cd_ndevs.get() {
        panic(format_args!(
            "autoconf: unit {unit} outside the cd_devs mirror"
        ));
    }
    // SAFETY: `cd_devs` has `cd_ndevs` slots (`mirror_resize` keeps the mirror the size of the
    // kernel's table) and `unit` is within them (checked above); the kernel calls this under
    // its tree lock, the writers' lock, and readers take the slot by value.
    unsafe { *cd.cd_devs.get().add(unit as usize) = dev };
}

/// The `cd_devs` array made `len` slots (0: freed), the slots in use copied.
fn mirror_resize(cd: &Cfdriver, len: usize) {
    let old = usize::try_from(cd.cd_ndevs.get()).unwrap_or(0);
    let slot = size_of::<Option<NonNull<Device>>>();
    let osp = NonNull::new(cd.cd_devs.get());
    if len == 0 {
        // nothing found; deallocate
        if let Some(osp) = osp {
            free(osp.cast(), M_DEVBUF, old * slot);
        }
        cd.cd_devs.set(ptr::null_mut());
        cd.cd_ndevs.set(0);
        return;
    }
    let Some(nsp) = mallocarray(len, slot, M_DEVBUF, M_NOWAIT | M_ZERO) else {
        panic(format_args!(
            "config_make_softc: {}ing dev array",
            if old != 0 { "expand" } else { "creat" }
        ));
    };
    let nsp = nsp.cast::<Option<NonNull<Device>>>();
    if let Some(osp) = osp {
        // SAFETY: the old array has `old` slots, the new one `len` zeroed ones, and
        // `old.min(len)` are copied; distinct allocations.
        unsafe { ptr::copy_nonoverlapping(osp.as_ptr(), nsp.as_ptr(), old.min(len)) };
        free(osp.cast(), M_DEVBUF, old * slot);
    }
    cd.cd_devs.set(nsp.as_ptr());
    cd.cd_ndevs.set(len as i32);
}

/// `config_init`: initialize autoconfiguration data structures. This occurs before console
/// initialization as that might require use of this subsystem. Furthermore this means that
/// malloc et al. isn't yet available.
pub fn config_init() {
    kac::config_init(&BSD_GLUE);
}

/// `config_search`: iterate over all potential children of some device, calling the given
/// function (default being the child's match function) for each one. Nonzero returns are
/// matches; the highest value returned is considered the best match. Return the "found child"
/// if we got a match, or `None` otherwise. The `aux` pointer is simply passed on through.
///
/// Note that this function is designed so that it can be used to apply an arbitrary function
/// to all potential children (its return value can be ignored).
pub fn config_search(fn_: Option<CfmatchT>, parent: &Device, aux: *mut c_void) -> Option<CfMatch> {
    let p = node_of(parent);
    let sub = fn_.map(|f| submatch(f, aux));
    kac::config_search(
        sub.as_ref().map(|s| s as kac::Submatch<'_>),
        &p,
        &aux_of(aux),
    )
    .map(release)
}

/// `config_scan`: iterate over all potential children of some device, calling the given
/// function for each one.
pub fn config_scan(fn_: CfscanT, parent: &Device) {
    let p = node_of(parent);
    kac::config_scan(
        &|par: &DevRef, m: Match| fn_(legacy_device(par), release(m)),
        &p,
    );
}

/// `config_rootsearch`: find the given root device. This is much like `config_search`, but
/// there is no parent.
pub fn config_rootsearch(
    fn_: Option<CfmatchT>,
    rootname: &[u8],
    aux: *mut c_void,
) -> Option<CfMatch> {
    let sub = fn_.map(|f| submatch(f, aux));
    kac::config_rootsearch(
        sub.as_ref().map(|s| s as kac::Submatch<'_>),
        rootname,
        &aux_of(aux),
    )
    .map(release)
}

/// `config_found_sm`: the given `aux` argument describes a device that has been found on the
/// given parent, but not necessarily configured. Locate the configuration data for that
/// device (using the submatch function provided, or using candidates' `ca_match`
/// configuration driver functions) and attach it, and return it. If the device was not
/// configured, call the given `print` function and return `None`.
pub fn config_found_sm(
    parent: &Device,
    aux: *mut c_void,
    print: Option<CfprintT>,
    submatch_: Option<CfmatchT>,
) -> Option<NonNull<Device>> {
    let p = node_of(parent);
    let pr = print.map(|f| printer(f, aux));
    let sub = submatch_.map(|f| submatch(f, aux));
    let d = kac::config_found_sm(
        &p,
        &aux_of(aux),
        pr.as_ref().map(|f| f as kac::Print<'_>),
        sub.as_ref().map(|s| s as kac::Submatch<'_>),
    )?;
    Some(NonNull::from(legacy_device(&d)))
}

/// `config_found(d, a, p)`: `config_found_sm` without a submatch function (`<sys/device.h>`).
pub fn config_found(
    parent: &Device,
    aux: *mut c_void,
    print: Option<CfprintT>,
) -> Option<NonNull<Device>> {
    config_found_sm(parent, aux, print, None)
}

/// `config_rootfound`: as `config_found_sm`, but for root devices.
pub fn config_rootfound(rootname: &[u8], aux: *mut c_void) -> Option<NonNull<Device>> {
    let d = kac::config_rootfound(rootname, &aux_of(aux))?;
    Some(NonNull::from(legacy_device(&d)))
}

/// `config_attach`: attach a found device. Allocates memory for device variables.
pub fn config_attach(
    parent: Option<&Device>,
    match_: CfMatch,
    aux: *mut c_void,
    print: Option<CfprintT>,
) -> NonNull<Device> {
    let p = parent.map(node_of);
    let m = adopt(match_, p.as_ref());
    let pr = print.map(|f| printer(f, aux));
    let d = kac::config_attach(
        p.as_ref(),
        m,
        &aux_of(aux),
        pr.as_ref().map(|f| f as kac::Print<'_>),
    );
    NonNull::from(legacy_device(&d))
}

/// `config_make_softc`: allocates the softc of a device `cf` describes below `parent`, picks
/// its unit, names it and makes room for it in the driver's `cd_devs`; the caller owns it
/// (it attaches it or frees it).
pub fn config_make_softc(parent: Option<&Device>, cf: &'static Cfdata) -> NonNull<Device> {
    let p = parent.map(node_of);
    release_softc(kac::config_make_softc(p.as_ref(), cf)).into_raw()
}

/// `config_detach`: detach a device. Optionally forced (e.g. because of hardware removal) and
/// quiet. Returns `Ok` if successful, the error otherwise.
///
/// Note that this code wants to be run from a process context, so that the detach can sleep to
/// allow processes which have a device open to run and unwind their stacks.
///
/// # Safety
///
/// `dev` is an attached device. On success it has been freed (unless someone else holds a
/// reference from `device_lookup`/`device_ref`), so the caller must not use it again.
pub unsafe fn config_detach(dev: NonNull<Device>, flags: i32) -> Result<(), Errno> {
    // SAFETY: the caller's guarantee: an attached device, alive for the call (its node, which
    // `node_of` finds, holds a reference until the detach gives it up).
    let n = node_of(unsafe { dev.as_ref() });
    kac::config_detach(n, flags)
}

/// `config_deactivate`: clears `DVF_ACTIVE` and tells the device (`DVACT_DEACTIVATE`); the
/// flag comes back if the device refuses.
pub fn config_deactivate(dev: &Device) -> Result<(), Errno> {
    kac::config_deactivate(&node_of(dev))
}

/// The legacy function of a deferral, for the kernel.
fn run_deferred(dev: &DevRef, arg: Option<&AnyArc>) {
    match arg.and_then(|a| a.downcast_ref::<LegacyFn>()) {
        Some(f) => (f.0)(legacy_device(dev)),
        None => panic(format_args!(
            "autoconf: a legacy deferral without its function"
        )),
    }
}

/// The kernel's record of a legacy deferral (the C's `malloc`ed `struct deferred_config`).
fn deferred(func: fn(&Device), what: &str) -> Deferred {
    match MArc::new(M_DEVBUF, hal::mem::malloc::M_NOWAIT, LegacyFn(func)) {
        Some(f) => Deferred {
            func: run_deferred,
            arg: Some(MArc::erase(f)),
        },
        None => panic(format_args!("{what}: can't allocate defer structure")),
    }
}

/// `config_defer`: defer the configuration of the specified device until all of its parent's
/// devices have been attached.
pub fn config_defer(dev: &Device, func: fn(&Device)) {
    kac::config_defer(&node_of(dev), deferred(func, "config_defer"));
}

/// `config_mountroot`: defer the configuration of the specified device until after root file
/// system is mounted.
pub fn config_mountroot(dev: &Device, func: fn(&Device)) {
    kac::config_mountroot(&node_of(dev), deferred(func, "config_mountroot"));
}

/// `config_process_deferred_children`: process the deferred configuration queue for a
/// device.
pub fn config_process_deferred_children(parent: &Device) {
    kac::config_process_deferred_children(&node_of(parent));
}

/// `config_process_deferred_mountroot`: process the deferred configuration queue after the
/// root file system is mounted.
pub fn config_process_deferred_mountroot() {
    kac::config_process_deferred_mountroot();
}

/// `config_pending_incr`: manipulate the `config_pending` semaphore.
pub fn config_pending_incr() {
    kac::config_pending_incr();
}

/// `config_pending_decr`.
pub fn config_pending_decr() {
    kac::config_pending_decr();
}

/// `config_detach_children`: detaches every child of `parent`, last attached first.
pub fn config_detach_children(parent: &Device, flags: i32) -> Result<(), Errno> {
    kac::config_detach_children(&node_of(parent), flags)
}

/// `config_suspend`: hands `act` to the device's `ca_activate`, or to its children when it
/// has none.
pub fn config_suspend(dev: &Device, act: i32) -> Result<(), Errno> {
    kac::config_suspend(&node_of(dev), act)
}

/// `config_suspend_all`: `config_suspend` on the `mpath` and `mainbus` trees, in the order
/// that suits `act`.
pub fn config_suspend_all(act: i32) -> Result<(), Errno> {
    kac::config_suspend_all(act)
}

/// `config_activate_children`: call the `ca_activate` for each of our children, letting each
/// decide whether they wish to do the same for their children and more.
pub fn config_activate_children(parent: &Device, act: i32) -> Result<(), Errno> {
    kac::config_activate_children(&node_of(parent), act)
}

/// `device_lookup`: lookup a device in the cfdriver device array. Does not return a device if
/// it is not active.
///
/// Increments ref count on the device by one, reflecting the new reference created on the
/// stack; the caller gives it back with `device_unref`.
///
/// Context: process only
pub fn device_lookup(cd: &Cfdriver, unit: i32) -> Option<NonNull<Device>> {
    let dv = cd.cd_dev(unit)?;
    // SAFETY: a unit in the `cd_devs` mirror is attached: its node, in the kernel's unit
    // table, holds a `dv_ref` on the block until the detach has cleared the slot.
    let d = unsafe { dv.as_ref() };

    if d.dv_flags.get() & DVF_ACTIVE == 0 {
        return None;
    }

    device_ref(d);
    Some(dv)
}

/// `device_mainbus`: the root bus, once it is attached.
pub fn device_mainbus() -> Option<NonNull<Device>> {
    let cd = mainbus_cd();
    if cd.cd_ndevs.get() < 1 {
        return None;
    }
    cd.cd_dev(0)
}

/// `device_mpath`: the `mpath(4)` root; `NMPATH` is 0 (see the module's deviations).
pub fn device_mpath() -> Option<NonNull<Device>> {
    None
}

/// `device_ref`: increments the ref count on the device structure. The device structure is
/// freed when the ref count hits 0.
///
/// Context: process or interrupt
pub fn device_ref(dv: &Device) {
    dv.dv_ref.fetch_add(1, Ordering::Relaxed);
}

/// `device_unref`: decrement the ref count on the device structure; frees the structure when
/// the ref count hits zero.
///
/// Context: process or interrupt
///
/// # Safety
///
/// `dv` is a live device on which the caller holds a reference (from `config_make_softc`,
/// `device_ref` or `device_lookup`), which this gives up: if it was the last, the device is
/// freed and must not be used again.
pub unsafe fn device_unref(dv: NonNull<Device>) {
    // SAFETY: the caller's reference keeps the device allocated until the decrement.
    let d = unsafe { dv.as_ref() };
    if d.dv_ref.fetch_sub(1, Ordering::AcqRel) == 1 {
        let ca = d.cfdata().cf_attach;
        free(dv.cast(), M_DEVBUF, ca.ca_devsize);
    }
}

/// Host tests: the kernel's tree forgotten (its nodes are an earlier test's memory) and no
/// attach in progress. A test that panicked inside `config_attach` leaves `autoconf_attdet`
/// raised, and every later `config_detach` would sleep on it for ever. The `cd_devs` mirror
/// of every attached legacy device's driver is forgotten first, not freed, as the kernel
/// forgets its unit table: a `static` driver's array would otherwise be freed into the next
/// test's memory when it grows. A native node has no mirror (its shadow header, if any, is
/// forgotten with the node).
#[cfg(test)]
pub(crate) fn autoconf_test_reset() {
    let mut d = kac::alldevs_next(None);
    while let Some(n) = d {
        if n.softc().downcast_ref::<LegacySoftc>().is_some() {
            let cd = legacy_device(&n).cfdata().cf_driver;
            cd.cd_devs.set(ptr::null_mut());
            cd.cd_ndevs.set(0);
        }
        d = kac::alldevs_next(Some(&n));
    }
    kac::autoconf_test_reset();
}

/// A row of `cfdata[]`: a legacy row is its own driver, a native row's driver is the
/// kernel's it names (`Cfdata::native`).
impl ConfigRow for Cfdata {
    fn driver(&'static self) -> &'static dyn DriverOps {
        match self.cf_native {
            Some(drv) => drv,
            None => self,
        }
    }
    fn parents(&self) -> &'static [i16] {
        self.cf_parents
    }
    fn starunit1(&self) -> i16 {
        self.cf_starunit1
    }
    fn unit(&self) -> i16 {
        self.cf_unit.get()
    }
    fn set_unit(&self, unit: i16) {
        self.cf_unit.set(unit);
    }
    fn fstate(&self) -> i16 {
        self.cf_fstate.get()
    }
    fn set_fstate(&self, fstate: i16) {
        self.cf_fstate.set(fstate);
    }
    fn loc(&self, i: usize) -> Option<i64> {
        self.cf_loc.get(i).copied()
    }
    fn flags(&self) -> i32 {
        self.cf_flags
    }
}

/// A legacy row is its own driver object: its `Cfattach`'s functions with its `Cfdriver`'s
/// constants (`bridges.toml`'s `DriverOps`).
impl DriverOps for Cfdata {
    fn name(&self) -> &'static [u8] {
        self.cf_driver.cd_name
    }
    fn class(&self) -> Devclass {
        self.cf_driver.cd_class
    }
    fn mode(&self) -> i32 {
        self.cf_driver.cd_mode
    }
    fn units(&self) -> &'static UnitTable {
        &self.cf_driver.cd_units
    }

    /// LZ's `config_make_softc`'s allocation: `ca_devsize` zeroed bytes, the header written
    /// from the kernel's identity, `dv_ref` 1 (the node's).
    fn make_softc(&self, id: &Ident<'_>) -> AnyArc {
        new_softc(LegacySoftc {
            dev: new_header(id, DVF_ACTIVE), // always initially active
            owns: true,
        })
    }

    fn probe(&self, parent: Option<&DevRef>, m: &Match, aux: &Aux<'_>) -> i32 {
        let Some(ca_match) = self.cf_attach.ca_match else {
            panic(format_args!(
                "mapply: no match function for '{}' device",
                Str(self.cf_driver.cd_name)
            ));
        };
        // A typed argument no presenter knows is another bus's: no match, as the C's
        // ca_match of a driver that does not attach there.
        with_aux(aux, |a| {
            ca_match(parent.map(|p| legacy_device(p)), &view(m), a)
        })
        .unwrap_or(0)
    }

    fn attach(&self, parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let attached = with_aux(aux, |a| {
            (self.cf_attach.ca_attach)(parent.map(|p| legacy_device(p)), legacy_device(dev), a);
        });
        if attached.is_none() {
            unpresented("the attach", self.cf_driver.cd_name);
        }
    }

    fn detach(&self, dev: &DevRef, flags: i32) -> Result<(), Errno> {
        match self.cf_attach.ca_detach {
            Some(detach) => detach(legacy_device(dev), flags),
            None => Err(Errno::EOPNOTSUPP),
        }
    }

    fn activate(&self, dev: &DevRef, act: i32) -> Option<Result<(), Errno>> {
        self.cf_attach
            .ca_activate
            .map(|activate| activate(legacy_device(dev), act))
    }

    fn flags_set(&self, dev: &Node, flags: i32) {
        legacy_device(dev).dv_flags.set(flags);
    }

    fn unit_set(&self, unit: i32, dev: Option<&Node>) {
        mirror_set(
            self.cf_driver,
            unit,
            dev.map(|d| NonNull::from(legacy_device(d))),
        );
    }

    fn units_resized(&self, len: usize) {
        mirror_resize(self.cf_driver, len);
    }
}

impl RowTable for CfTable {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn row(&self, i: usize) -> &'static dyn ConfigRow {
        match self.0.get(i) {
            Some(cf) => cf,
            None => panic(format_args!("autoconf: cfdata[{i}] out of the table")),
        }
    }
}

impl ConfigGlue for BsdConfigGlue {
    fn with_rows(&self, f: &mut dyn FnMut(&dyn RowTable)) {
        // cfdata() finds the first free slot once, for the whole walk (LZ's walks trim once).
        f(&CfTable(cfdata()));
    }
    fn roots(&self) -> &'static [i16] {
        cfroots()
    }
    /// The units of `mainbus_cd`'s row: a legacy row's `cd_units`, a native row's driver's
    /// table (`Cfdata::native`). A walk of the rows, for `config_suspend_all` only (suspend,
    /// powerdown), where the C reads `mainbus_cd.cd_devs[0]`.
    fn mainbus(&self) -> &'static UnitTable {
        let cd = mainbus_cd();
        match cfdata().iter().find(|cf| ptr::eq(cf.cf_driver, cd)) {
            Some(cf) => ConfigRow::driver(cf).units(),
            None => &cd.cd_units,
        }
    }
    /// The machine's `device_register` for a legacy device; a native device's is not called
    /// (arm64's does nothing, as its C; amd64's is N10's, which gives it a typed form).
    fn device_register(&self, dev: &DevRef, aux: &Aux<'_>) {
        if dev.softc().downcast_ref::<LegacySoftc>().is_none() {
            return;
        }
        let d = legacy_device(dev);
        if with_aux(aux, |a| device_register(d, a)).is_none() {
            unpresented("device_register", d.cfdata().cf_driver.cd_name);
        }
    }
    fn boothowto(&self) -> i32 {
        BOOTHOWTO.load(Ordering::Relaxed)
    }
    fn root_mounted(&self) -> bool {
        rootvp().is_some()
    }
    fn nam2blk(&self) -> &'static [kernel::dev::disk::Nam2blk] {
        crate::machine::autoconf::nam2blk()
    }
    fn swapdev(&self) -> i32 {
        crate::machine::conf::swapdev()
    }
    fn boothowto_or(&self, flags: i32) {
        BOOTHOWTO.fetch_or(flags, Ordering::Relaxed);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the shim over the kernel's autoconfiguration, with a test `ioconf`: a
    // root bus `troot0` and a starred child driver `tchild*` below it, attached, looked up,
    // deferred and detached through the legacy names, the legacy header mirroring its node.

    use core::sync::atomic::{AtomicI32, AtomicUsize};
    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::vec::Vec;
    use std::{assert, assert_eq};

    use super::*;
    use crate::kern::init_main::set_rootvp;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::machine::Machine;
    // What the child's attach arguments say (whether to match, and at which level): the
    // host presenter's legacy argument, so a native bus's typed one reaches the child as the
    // same struct.
    use crate::machine::autoconf::{HostLegacyArgs as ChildArgs, HostTypedArgs};
    use crate::sys::device::{
        CD_INDIRECT, CFATTACH_NATIVE, Cfattach, DETACH_QUIET, DV_DULL, DVACT_RESUME, DVACT_SUSPEND,
        FSTATE_FOUND, FSTATE_NOTFOUND, FSTATE_STAR, QUIET, UNCONF,
    };

    /// A C string up to its first NUL.
    fn cstr(s: &[u8]) -> &[u8] {
        &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())]
    }

    static ROOT_ATTACHED: AtomicUsize = AtomicUsize::new(0);
    static CHILD_ATTACHED: AtomicUsize = AtomicUsize::new(0);
    static PRINTED: AtomicUsize = AtomicUsize::new(0);
    static DEFERRED_RAN: AtomicUsize = AtomicUsize::new(0);
    static ACTIVATE_FAIL_UNIT: AtomicI32 = AtomicI32::new(-1);
    static RESUMED: AtomicUsize = AtomicUsize::new(0);
    /// The `pri` the last child's attach read from its arguments.
    static ATTACHED_PRI: AtomicI32 = AtomicI32::new(0);

    fn root_match(parent: Option<&Device>, _m: &CfMatch, _aux: *mut c_void) -> i32 {
        assert!(parent.is_none());
        1
    }

    fn root_attach(parent: Option<&Device>, _self: &Device, _aux: *mut c_void) {
        assert!(parent.is_none());
        ROOT_ATTACHED.fetch_add(1, Ordering::Relaxed);
    }

    fn child_match(parent: Option<&Device>, m: &CfMatch, aux: *mut c_void) -> i32 {
        assert!(parent.is_some());
        assert_eq!(m.cfdata().cf_driver.cd_name, b"tchild");
        // SAFETY: the tests always pass a `ChildArgs` as the child's aux.
        unsafe { (*aux.cast::<ChildArgs>()).pri }
    }

    fn child_attach(parent: Option<&Device>, self_: &Device, aux: *mut c_void) {
        assert!(parent.is_some());
        assert!(ptr::eq(
            self_.parent().expect("a parent"),
            parent.expect("a parent")
        ));
        // SAFETY: the tests always pass a `ChildArgs` as the child's aux.
        ATTACHED_PRI.store(unsafe { (*aux.cast::<ChildArgs>()).pri }, Ordering::Relaxed);
        CHILD_ATTACHED.fetch_add(1, Ordering::Relaxed);
    }

    fn child_detach(_dev: &Device, _flags: i32) -> Result<(), Errno> {
        Ok(())
    }

    fn child_activate(dev: &Device, act: i32) -> Result<(), Errno> {
        if dev.dv_unit.get() == ACTIVATE_FAIL_UNIT.load(Ordering::Relaxed) && act == DVACT_SUSPEND {
            return Err(Errno::EBUSY);
        }
        if act == DVACT_RESUME {
            RESUMED.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    fn child_print(_aux: *mut c_void, pnp: Option<&[u8]>) -> i32 {
        PRINTED.fetch_add(1, Ordering::Relaxed);
        if pnp.is_some() { UNCONF } else { QUIET }
    }

    fn deferred(dev: &Device) {
        assert_eq!(dev.cfdata().cf_driver.cd_name, b"tchild");
        DEFERRED_RAN.fetch_add(1, Ordering::Relaxed);
    }

    /// A fresh test `ioconf` (state lives in the tables, so every test gets its own) with
    /// `troot0 at root` and `tchild* at troot0`; `root_mode` is the root driver's `cd_mode`.
    fn ioconf(root_mode: i32) -> (&'static [Cfdata], &'static Cfdriver, &'static Cfdriver) {
        let root_ca: &'static Cfattach = Box::leak(Box::new(Cfattach {
            ca_devsize: size_of::<Device>(),
            ca_match: Some(root_match),
            ca_attach: root_attach,
            ca_detach: None,
            ca_activate: None,
        }));
        let root_cd: &'static Cfdriver =
            Box::leak(Box::new(Cfdriver::new(b"troot", DV_DULL, root_mode)));
        let child_ca: &'static Cfattach = Box::leak(Box::new(Cfattach {
            ca_devsize: size_of::<Device>() + 64,
            ca_match: Some(child_match),
            ca_attach: child_attach,
            ca_detach: Some(child_detach),
            ca_activate: Some(child_activate),
        }));
        let child_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"tchild", DV_DULL, 0)));
        let table: &'static [Cfdata] = Box::leak(Box::new([
            Cfdata::new(root_ca, root_cd, 0, FSTATE_NOTFOUND, &[], 0, &[], 0, 0),
            Cfdata::new(child_ca, child_cd, 0, FSTATE_STAR, &[], 0, &[0], 0, 0),
        ]));
        (table, root_cd, child_cd)
    }

    /// Real memory for malloc(9), a fresh `ioconf` installed, the counters reset.
    fn setup(
        root_mode: i32,
    ) -> (
        MutexGuard<'static, ()>,
        &'static [Cfdata],
        &'static Cfdriver,
        &'static Cfdriver,
    ) {
        let (table, root_cd, child_cd) = ioconf(root_mode);
        (setup_with(table), table, root_cd, child_cd)
    }

    /// Real memory for malloc(9), `table` installed as `cfdata[]` (row 0 the root), the
    /// counters reset.
    fn setup_with(table: &'static [Cfdata]) -> MutexGuard<'static, ()> {
        let guard = setup_real_memory();
        // SAFETY: `setup_real_memory`'s lock serialises the tests.
        unsafe { Machine::set_ioconf(table, &[0]) };
        config_init();
        for c in [
            &ROOT_ATTACHED,
            &CHILD_ATTACHED,
            &PRINTED,
            &DEFERRED_RAN,
            &RESUMED,
            &NATIVE_ATTACHED,
        ] {
            c.store(0, Ordering::Relaxed);
        }
        ACTIVATE_FAIL_UNIT.store(-1, Ordering::Relaxed);
        ATTACHED_PRI.store(0, Ordering::Relaxed);
        guard
    }

    /// A test `ioconf` with a native root (`Cfdata::native` over a fresh `TestNative` named
    /// as `root_cd`, its stub `Cfdriver`) and the legacy `tchild* at` it.
    fn native_ioconf(
        root_cd: &'static Cfdriver,
    ) -> (&'static [Cfdata], &'static TestNative, &'static Cfdriver) {
        let (legacy, _, child_cd) = ioconf(0);
        let drv = native_driver(root_cd.cd_name);
        let table: &'static [Cfdata] = Box::leak(Box::new([
            Cfdata::new(
                &CFATTACH_NATIVE,
                root_cd,
                0,
                FSTATE_NOTFOUND,
                &[],
                0,
                &[],
                0,
                0,
            )
            .native(drv),
            legacy[1].clone(),
        ]));
        (table, drv, child_cd)
    }

    fn name(dev: NonNull<Device>) -> Vec<u8> {
        // SAFETY: the tests only name attached devices.
        let n = unsafe { dev.as_ref() }.dv_xname.get();
        cstr(&n).to_vec()
    }

    fn aux(args: &mut ChildArgs) -> *mut c_void {
        ptr::from_mut(args).cast()
    }

    /// `alldevs`, by the legacy headers' names.
    fn alldevs() -> Vec<Vec<u8>> {
        let mut v = Vec::new();
        let mut d = kac::alldevs_next(None);
        while let Some(n) = d {
            v.push(cstr(&legacy_device(&n).dv_xname.get()).to_vec());
            d = kac::alldevs_next(Some(&n));
        }
        v
    }

    /// A native driver (a kernel `DriverOps`, as `Cfdata::native` names one) with its own
    /// unit table: a root, which matches only without a parent, or (`child`) a child, which
    /// matches only below one.
    struct TestNative {
        name: &'static [u8],
        units: &'static UnitTable,
        child: bool,
    }

    /// The native driver's softc: nothing the tests read.
    struct NativeSoftc;

    static NATIVE_ATTACHED: AtomicUsize = AtomicUsize::new(0);

    impl DriverOps for TestNative {
        fn name(&self) -> &'static [u8] {
            self.name
        }
        fn class(&self) -> Devclass {
            DV_DULL
        }
        fn mode(&self) -> i32 {
            0
        }
        fn units(&self) -> &'static UnitTable {
            self.units
        }
        fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
            let sc = MArc::new(M_DEVBUF, hal::mem::malloc::M_NOWAIT, NativeSoftc);
            MArc::erase(sc.expect("a native softc"))
        }
        fn probe(&self, parent: Option<&DevRef>, _m: &Match, _aux: &Aux<'_>) -> i32 {
            i32::from(parent.is_some() == self.child)
        }
        fn attach(&self, parent: Option<&DevRef>, _dev: &DevRef, _aux: &Aux<'_>) {
            assert_eq!(parent.is_some(), self.child);
            NATIVE_ATTACHED.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A fresh native root driver named `name`.
    fn native_driver(name: &'static [u8]) -> &'static TestNative {
        native_kind(name, false)
    }

    /// A fresh native driver named `name`, a root or a child.
    fn native_kind(name: &'static [u8], child: bool) -> &'static TestNative {
        Box::leak(Box::new(TestNative {
            name,
            units: Box::leak(Box::new(UnitTable::new())),
            child,
        }))
    }

    #[test]
    fn a_row_gives_its_locators_and_a_native_row_its_driver() {
        let (table, _, child_cd) = ioconf(0);
        let drv = native_driver(b"tnat");
        let nat_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"tnat", DV_DULL, 0)));
        let rows: &'static [Cfdata] = Box::leak(Box::new([
            Cfdata::new(
                &CFATTACH_NATIVE,
                nat_cd,
                0,
                FSTATE_STAR,
                &[1, -1],
                0,
                &[],
                0,
                0,
            )
            .native(drv),
            Cfdata::new(
                table[1].cf_attach,
                child_cd,
                0,
                FSTATE_STAR,
                &[7],
                0,
                &[0],
                0,
                0,
            ),
        ]));

        // cf_flags, a native row's as a legacy row's (viornd's interval, N7.3a).
        let flagged = Cfdata::new(
            &CFATTACH_NATIVE,
            nat_cd,
            0,
            FSTATE_STAR,
            &[],
            0x1000,
            &[],
            0,
            0,
        )
        .native(drv);
        assert_eq!(ConfigRow::flags(&flagged), 0x1000);
        assert_eq!(ConfigRow::flags(&rows[1]), 0);

        // cf_loc[i], and nothing past the row's locators.
        assert_eq!(rows[0].loc(0), Some(1));
        assert_eq!(rows[0].loc(1), Some(-1));
        assert_eq!(rows[0].loc(2), None);
        assert_eq!(rows[1].loc(0), Some(7));
        assert_eq!(table[1].loc(0), None, "a row without locators");

        // A native row's driver is the kernel's it names; a legacy row is its own.
        let native: &dyn DriverOps = drv;
        assert!(ptr::addr_eq(ConfigRow::driver(&rows[0]), native));
        assert_eq!(ConfigRow::driver(&rows[0]).name(), b"tnat");
        let legacy: &dyn DriverOps = &rows[1];
        assert!(ptr::addr_eq(ConfigRow::driver(&rows[1]), legacy));

        // Still a row of cfdata[]: not a free slot, and UKC's copy keeps its driver.
        assert!(!rows[0].is_free());
        assert!(ptr::eq(rows[0].cf_attach, &CFATTACH_NATIVE));
        assert!(rows[1].cf_native.is_none());
        let copy = rows[0].clone();
        assert!(copy.cf_native.is_some_and(|d| ptr::addr_eq(d, native)));
        assert_eq!(copy.cf_loc, rows[0].cf_loc);

        // A legacy bus's submatch that calls ca_match by hand gets no match from it.
        let ca_match = CFATTACH_NATIVE.ca_match.expect("a match function");
        assert_eq!(
            ca_match(None, &CfMatch::Cfdata(&rows[0]), ptr::null_mut()),
            0
        );
    }

    #[test]
    fn rootfound_attaches_the_named_root() {
        let (_g, table, root_cd, _) = setup(0);
        assert!(config_rootfound(b"nosuchroot", ptr::null_mut()).is_none());
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        assert_eq!(name(root), b"troot0");
        assert_eq!(ROOT_ATTACHED.load(Ordering::Relaxed), 1);
        assert_eq!(table[0].cf_fstate.get(), FSTATE_FOUND);
        assert_eq!(root_cd.cd_dev(0), Some(root));
        assert_eq!(alldevs(), [b"troot0".to_vec()]);
        // SAFETY: just attached.
        let r = unsafe { root.as_ref() };
        assert_eq!(
            r.dv_ref.load(Ordering::Relaxed),
            1,
            "the node's (config_make_softc's and config_attach's are its handles)"
        );
        assert!(r.dv_flags.get() & DVF_ACTIVE != 0);
        assert!(r.parent().is_none());
        // The header mirrors its node, which the driver's unit table holds.
        let n = root_cd.cd_units.get(0).expect("troot0's node");
        assert!(ptr::eq(legacy_device(&n), r));
        assert_eq!(n.xname(), "troot0");
        assert_eq!(root_cd.cd_ndevs.get() as usize, root_cd.cd_units.len());
    }

    /// The block goes only when the node's last handle and the last legacy `device_ref` are
    /// both gone.
    #[test]
    fn a_legacy_block_outlives_both_counts() {
        let (_g, _, _, child_cd) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 1 };
        let a = config_found(r, aux(&mut yes), None).expect("tchild0");
        let looked = device_lookup(child_cd, 0).expect("active");
        let node = child_cd.cd_units.get(0).expect("tchild0's node");
        // SAFETY: attached; on success the lookup's reference and `node` keep it.
        assert_eq!(unsafe { config_detach(a, DETACH_QUIET) }, Ok(()));
        assert!(child_cd.cd_dev(0).is_none(), "the mirror's slot is cleared");
        // SAFETY: the lookup's reference keeps the block.
        let d = unsafe { looked.as_ref() };
        assert_eq!(
            d.dv_ref.load(Ordering::Relaxed),
            2,
            "the node's and the lookup's"
        );
        drop(node);
        assert_eq!(d.dv_ref.load(Ordering::Relaxed), 1, "the lookup's");
        // SAFETY: the last reference; the block is freed and not used again.
        unsafe { device_unref(looked) };
    }

    #[test]
    fn starred_children_take_increasing_units() {
        let (_g, table, _, child_cd) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 1 };
        let a = config_found(r, aux(&mut yes), Some(child_print)).expect("tchild0");
        let b = config_found(r, aux(&mut yes), Some(child_print)).expect("tchild1");
        assert_eq!(name(a), b"tchild0");
        assert_eq!(name(b), b"tchild1");
        assert_eq!(table[1].cf_unit.get(), 2);
        assert_eq!(table[1].cf_fstate.get(), FSTATE_STAR);
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 2);
        assert_eq!(
            PRINTED.load(Ordering::Relaxed),
            2,
            "once each, with pnp == NULL"
        );
        assert_eq!(child_cd.cd_dev(1), Some(b));
        assert_eq!(alldevs().len(), 3);
    }

    #[test]
    fn an_unmatched_device_is_printed_not_configured() {
        let (_g, _, _, _) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut no = ChildArgs { pri: 0 };
        assert!(config_found(r, aux(&mut no), Some(child_print)).is_none());
        assert_eq!(PRINTED.load(Ordering::Relaxed), 1);
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn indirect_parents_match_against_softcs() {
        let (_g, _, _, child_cd) = setup(CD_INDIRECT);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 3 };
        let m = config_search(None, r, aux(&mut yes)).expect("a match");
        assert!(matches!(m, CfMatch::Softc(_)));
        let child = config_attach(Some(r), m, aux(&mut yes), None);
        assert_eq!(name(child), b"tchild0");
        assert_eq!(child_cd.cd_dev(0), Some(child));
        // A losing softc is freed: nothing is left behind in cd_devs.
        let mut no = ChildArgs { pri: 0 };
        assert!(config_search(None, r, aux(&mut no)).is_none());
        assert!(child_cd.cd_dev(1).is_none());
    }

    #[test]
    fn detach_frees_the_unit_for_reuse() {
        let (_g, table, _, child_cd) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 1 };
        let a = config_found(r, aux(&mut yes), None).expect("tchild0");
        let b = config_found(r, aux(&mut yes), None).expect("tchild1");
        let serial = AUTOCONF_SERIAL.load(Ordering::Relaxed);

        // SAFETY: attached, not used afterwards.
        assert_eq!(unsafe { config_detach(b, DETACH_QUIET) }, Ok(()));
        assert_eq!(table[1].cf_unit.get(), 1, "the last starred unit is reused");
        assert!(child_cd.cd_dev(1).is_none());
        assert_eq!(AUTOCONF_SERIAL.load(Ordering::Relaxed), serial + 1);
        let c = config_found(r, aux(&mut yes), None).expect("tchild1 again");
        assert_eq!(name(c), b"tchild1");

        // The root has no ca_detach: it fails after the deactivation, which the C keeps (and
        // which reached the children through config_activate_children).
        // SAFETY: attached; on failure it stays attached.
        let rv = unsafe { config_detach(root, DETACH_QUIET) };
        assert_eq!(rv, Err(Errno::EOPNOTSUPP));
        assert!(r.dv_flags.get() & DVF_ACTIVE == 0);
        // SAFETY: still attached.
        assert!(unsafe { a.as_ref() }.dv_flags.get() & DVF_ACTIVE == 0);

        assert_eq!(config_detach_children(r, DETACH_QUIET), Ok(()));
        assert_eq!(alldevs(), [b"troot0".to_vec()]);
        assert_eq!(
            child_cd.cd_ndevs.get(),
            0,
            "the empty cd_devs array is freed"
        );
    }

    #[test]
    fn device_lookup_takes_a_reference_on_active_devices() {
        let (_g, _, _, child_cd) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 1 };
        let a = config_found(r, aux(&mut yes), None).expect("tchild0");
        // SAFETY: attached above.
        let d = unsafe { a.as_ref() };

        assert!(device_lookup(child_cd, 1).is_none());
        assert!(device_lookup(child_cd, -1).is_none());
        assert_eq!(device_lookup(child_cd, 0), Some(a));
        assert_eq!(
            d.dv_ref.load(Ordering::Relaxed),
            2,
            "the node's and the lookup's"
        );
        // SAFETY: gives back the lookup's reference.
        unsafe { device_unref(a) };
        assert_eq!(d.dv_ref.load(Ordering::Relaxed), 1);

        assert_eq!(config_deactivate(d), Ok(()));
        assert!(device_lookup(child_cd, 0).is_none(), "inactive");
    }

    #[test]
    fn deferred_configuration_runs_after_the_parent_attach() {
        let (_g, _, _, _) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 1 };
        let a = config_found(r, aux(&mut yes), None).expect("tchild0");
        // SAFETY: attached above.
        let d = unsafe { a.as_ref() };
        // No root file system for config_mountroot below (an earlier test may have set one).
        let saved_root = rootvp();
        set_rootvp(None);

        config_defer(d, deferred);
        assert_eq!(CONFIG_PENDING.load(Ordering::Relaxed), 1);
        config_process_deferred_children(d);
        assert_eq!(
            DEFERRED_RAN.load(Ordering::Relaxed),
            0,
            "not a child of itself"
        );
        config_process_deferred_children(r);
        assert_eq!(DEFERRED_RAN.load(Ordering::Relaxed), 1);
        assert_eq!(CONFIG_PENDING.load(Ordering::Relaxed), 0);

        config_mountroot(d, deferred);
        assert_eq!(
            DEFERRED_RAN.load(Ordering::Relaxed),
            1,
            "no root file system yet"
        );
        config_process_deferred_mountroot();
        assert_eq!(DEFERRED_RAN.load(Ordering::Relaxed), 2);
        set_rootvp(saved_root);
    }

    #[test]
    fn a_refused_suspend_resumes_the_earlier_siblings() {
        let (_g, _, _, _) = setup(0);
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        // SAFETY: attached above.
        let r = unsafe { root.as_ref() };
        let mut yes = ChildArgs { pri: 1 };
        for _ in 0..3 {
            config_found(r, aux(&mut yes), None).expect("a child");
        }
        ACTIVATE_FAIL_UNIT.store(2, Ordering::Relaxed);
        assert_eq!(config_suspend(r, DVACT_SUSPEND), Err(Errno::EBUSY));
        assert_eq!(
            RESUMED.load(Ordering::Relaxed),
            2,
            "tchild1 and tchild0 resumed"
        );
        ACTIVATE_FAIL_UNIT.store(-1, Ordering::Relaxed);
        assert_eq!(config_suspend(r, DVACT_SUSPEND), Ok(()));
    }

    #[test]
    fn a_native_root_lends_its_legacy_children_a_shadow_header() {
        let root_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"troot", DV_DULL, 0)));
        let (table, drv, child_cd) = native_ioconf(root_cd);
        let _g = setup_with(table);

        // config_rootfound attaches the native root and returns its shadow header.
        let root = config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        assert_eq!(NATIVE_ATTACHED.load(Ordering::Relaxed), 1);
        assert_eq!(table[0].cf_fstate.get(), FSTATE_FOUND);
        let node = drv.units.get(0).expect("troot0's node");
        assert!(
            node.softc().downcast_ref::<LegacySoftc>().is_none(),
            "the node's softc is the native driver's"
        );
        // SAFETY: the node holds its shadow header.
        let r = unsafe { root.as_ref() };
        assert!(ptr::eq(legacy_device(&node), r));
        assert!(
            ptr::eq(legacy_device(&node), legacy_device(&node)),
            "one shadow header per node"
        );
        // The header is the node's identity.
        assert_eq!(cstr(&r.dv_xname.get()), b"troot0");
        assert_eq!(r.dv_unit.get(), node.unit());
        assert_eq!(r.dv_class.get(), node.class());
        assert!(ptr::eq(r.cfdata(), &table[0]));
        assert!(r.parent().is_none());
        assert_eq!(r.dv_ref.load(Ordering::Relaxed), 1, "the node's");
        assert!(r.dv_flags.get() & DVF_ACTIVE != 0);
        // ... named back to its node, through the native driver's unit table.
        assert!(MArc::ptr_eq(&node_of(r), &node));
        assert!(
            root_cd.cd_dev(0).is_none(),
            "a native driver keeps no mirror"
        );

        // A legacy child below it, through the legacy names.
        let mut yes = ChildArgs { pri: 1 };
        let child = config_found(r, aux(&mut yes), None).expect("tchild0");
        assert_eq!(name(child), b"tchild0");
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 1);
        // SAFETY: attached above.
        let c = unsafe { child.as_ref() };
        assert!(ptr::eq(c.parent().expect("a parent"), r));
        let cnode = child_cd.cd_units.get(0).expect("tchild0's node");
        assert!(cnode.is_child_of(&node));
        assert_eq!(alldevs(), [b"troot0".to_vec(), b"tchild0".to_vec()]);

        // The header's flags follow the node's.
        assert_eq!(kac::config_deactivate(&node), Ok(()));
        assert!(legacy_device(&node).dv_flags.get() & DVF_ACTIVE == 0);
    }

    #[test]
    fn a_native_bus_s_typed_argument_is_presented_to_a_legacy_driver() {
        let root_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"troot", DV_DULL, 0)));
        let (table, drv, child_cd) = native_ioconf(root_cd);
        let _g = setup_with(table);
        config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        let node = drv.units.get(0).expect("troot0's node");

        // The host's presenter lends HostTypedArgs as the legacy struct the child reads in
        // its ca_match (the level) and its ca_attach.
        let typed = HostTypedArgs { pri: 2 };
        let found = kac::config_found(&node, &Aux::Typed(&typed), None).expect("tchild0");
        assert_eq!(found.xname(), "tchild0");
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 1);
        assert_eq!(ATTACHED_PRI.load(Ordering::Relaxed), 2);
        assert!(found.is_child_of(&node));
        assert!(child_cd.cd_dev(0).is_some(), "the legacy child's mirror");
        assert!(ptr::eq(
            legacy_device(&found).parent().expect("a parent"),
            legacy_device(&node)
        ));
        // A typed argument at level 0 matches nothing, as the legacy one would.
        let nomatch = HostTypedArgs { pri: 0 };
        assert!(kac::config_found(&node, &Aux::Typed(&nomatch), None).is_none());
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_typed_argument_nobody_presents_matches_no_legacy_row() {
        /// A native bus's argument no presenter knows.
        struct Unknown;

        let root_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"troot", DV_DULL, 0)));
        let (table, drv, _) = native_ioconf(root_cd);
        let _g = setup_with(table);
        config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        let node = drv.units.get(0).expect("troot0's node");

        assert!(kac::config_found(&node, &Aux::Typed(&Unknown), None).is_none());
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 0);
        assert!(with_aux(&Aux::Typed(&Unknown), |_| ()).is_none());
        let known = HostTypedArgs { pri: 5 };
        // SAFETY: the host's presenter lends a `HostLegacyArgs` for a `HostTypedArgs`.
        let pri = with_aux(&Aux::Typed(&known), |a| unsafe {
            (*a.cast::<ChildArgs>()).pri
        });
        assert_eq!(pri, Some(5));
    }

    #[test]
    fn the_glue_s_mainbus_follows_the_row_of_mainbus_cd() {
        let cd = mainbus_cd();
        let (table, drv, _) = native_ioconf(cd);
        let _g = setup_with(table);
        config_rootfound(b"mainbus", ptr::null_mut()).expect("mainbus0");
        let units = BSD_GLUE.mainbus();
        assert!(ptr::eq(units, drv.units), "a native root's units");
        assert_eq!(units.get(0).expect("mainbus0").xname(), "mainbus0");

        // A legacy mainbus row's are its cd_units, as without any row.
        let (legacy, _, _) = ioconf(0);
        let rows: &'static [Cfdata] = Box::leak(Box::new([Cfdata::new(
            legacy[0].cf_attach,
            cd,
            0,
            FSTATE_NOTFOUND,
            &[],
            0,
            &[],
            0,
            0,
        )]));
        // SAFETY: `setup_real_memory`'s lock (held in `_g`) serialises the tests.
        unsafe { Machine::set_ioconf(rows, &[0]) };
        assert!(ptr::eq(BSD_GLUE.mainbus(), &cd.cd_units));
        // SAFETY: as above.
        unsafe { Machine::set_ioconf(&[], &[]) };
        assert!(ptr::eq(BSD_GLUE.mainbus(), &cd.cd_units));
    }

    #[test]
    fn a_shadow_header_goes_with_its_node() {
        let root_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"troot", DV_DULL, 0)));
        let (table, _, _) = native_ioconf(root_cd);
        let _g = setup_with(table);

        // A native node made and not attached: dropping it gives everything back.
        let d = kac::config_make_softc(None, &table[0]);
        let h = legacy_device(&d);
        assert!(
            legacy_softc(&d).is_none(),
            "a shadow header is no softc for a legacy caller (view, release_softc)"
        );
        assert_eq!(
            h.cfdata().cf_attach.ca_devsize,
            size_of::<Device>(),
            "the block is CFATTACH_NATIVE's, the size device_unref frees"
        );
        let p = NonNull::from(h);
        device_ref(h);
        drop(d);
        // SAFETY: the reference taken above keeps the block.
        let held = unsafe { p.as_ref() };
        assert_eq!(
            held.dv_ref.load(Ordering::Relaxed),
            1,
            "the node's went with it"
        );
        // SAFETY: the last reference; the block is freed and not used again.
        unsafe { device_unref(p) };

        // A legacy node's softc is its own block, which a legacy caller may hold.
        let (legacy, _, _) = ioconf(0);
        let l = kac::config_make_softc(None, &legacy[0]);
        assert!(legacy_softc(&l).is_some_and(|dv| ptr::eq(dv, legacy_device(&l))));
    }

    #[test]
    fn a_native_child_of_a_native_parent_chains_the_shadow_headers() {
        /// A native bus's argument no presenter knows: the legacy row answers 0 for it.
        struct Unknown;

        let root_cd: &'static Cfdriver = Box::leak(Box::new(Cfdriver::new(b"troot", DV_DULL, 0)));
        let nchild_cd: &'static Cfdriver =
            Box::leak(Box::new(Cfdriver::new(b"tnchild", DV_DULL, 0)));
        let (legacy, _, _) = ioconf(0);
        let root = native_driver(b"troot");
        let nchild = native_kind(b"tnchild", true);
        let table: &'static [Cfdata] = Box::leak(Box::new([
            Cfdata::new(
                &CFATTACH_NATIVE,
                root_cd,
                0,
                FSTATE_NOTFOUND,
                &[],
                0,
                &[],
                0,
                0,
            )
            .native(root),
            Cfdata::new(
                &CFATTACH_NATIVE,
                nchild_cd,
                0,
                FSTATE_STAR,
                &[],
                0,
                &[0],
                0,
                0,
            )
            .native(nchild),
            legacy[1].clone(),
        ]));
        let _g = setup_with(table);
        config_rootfound(b"troot", ptr::null_mut()).expect("troot0");
        let node = root.units.get(0).expect("troot0's node");

        let c = kac::config_found(&node, &Aux::Typed(&Unknown), None).expect("tnchild0");
        assert_eq!(c.xname(), "tnchild0");
        assert_eq!(NATIVE_ATTACHED.load(Ordering::Relaxed), 2);
        assert_eq!(CHILD_ATTACHED.load(Ordering::Relaxed), 0);
        assert!(c.is_child_of(&node));

        // The child's header names the parent's header, whose own parent is none.
        let ch = legacy_device(&c);
        let rh = legacy_device(&node);
        assert!(ptr::eq(ch.parent().expect("a parent"), rh));
        assert!(rh.parent().is_none());
        assert_eq!(cstr(&ch.dv_xname.get()), b"tnchild0");
        assert!(MArc::ptr_eq(&node_of(ch), &c));
        assert!(MArc::ptr_eq(&node_of(rh), &node));
    }
}
/* </TESTS> */
