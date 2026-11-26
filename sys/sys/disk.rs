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
 * Copyright (c) 1995 Jason R. Thorpe.  All rights reserved.
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
 * from: Header: disk.h,v 1.5 92/11/19 04:33:03 torek Exp  (LBL)
 *
 *	@(#)disk.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/disk.h>` and the disk layer as `bsd`'s disk drivers name them, until the last of
//! them moves (sd, cd: N7.4b; rd, vnd, softraid: N7.7; wd, fd: N10): the kernel's
//! `kernel::dev::disk` (`struct disk`, `disklist`, the label code) and
//! `kernel::dev::disksubr` (`readdisklabel`, `writedisklabel`), with the driver's side of
//! the handle.
//!
//! Upstream: sys/sys/disk.h @ 3ce1f3f79392
//!
//! A softc holds a [`DiskSlot`] where the C embeds `struct disk sc_dk`: valid all zero (an
//! `M_ZERO` softc), filled by [`DiskSlot::construct`] with the disk's name and flags before
//! [`disk_attach`], emptied by [`DiskSlot::destroy`] after [`disk_detach`]. The functions
//! here keep LZ's names and take the slot (`disk_busy(&sc.sc_dk)`); the member reads are the
//! slot's methods (`sc.sc_dk.openmask()` for `sc->sc_dk.dk_openmask`). The label routines
//! take the driver's strategy routine (`DevTypeStrategy`) and lend the kernel's buffer token
//! to it (`sys::buf::lend`, `biowait`, `token_of`).
//!
//! ## Deviations
//! - A pseudo-device's disk whose `struct device` its driver made by hand (vnd(4): no
//!   `cfdata`, no node in the kernel's tree) attaches without a `dk_device`
//!   (`disk_attach_unit`): its block device comes from its name and unit, as the C's.
//! - A slot read before `construct` (or after `destroy`) is `panic(9)` where the C reads the
//!   zeroed `struct disk`: no driver of the tree does (each constructs in its attach).
//!
//! The shim holds no `unsafe` (docs/SCAFFOLDING.md, "The disk's legacy edge").

use core::ptr::{self, NonNull};

use hal::sync::slot::ReadMostly;
pub use hal::sys::disk::*;
pub use kernel::dev::disk::{
    DUID_SIZE, Disk, DiskLock, DiskRef, LabelGeom, bounds_check_with_label, checkdisklabel,
    diskerr, dkcksum, duid_equal, duid_format, duid_iszero, initdisklabel, set_bootduid,
    setdisklabel,
};

use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_autoconf::{device_lookup, node_of};
use crate::kern::subr_prf::panic;
use crate::machine::conf::{bdevsw, cdevsw, nblkdev, nchrdev};
use crate::machine::intr::{splbio, splx};
use crate::sys::buf::{Buf, BufRef, biowait, lend, token_of};
use crate::sys::conf::{DevTypeOpen, DevTypeStrategy};
use crate::sys::device::{Cfdriver, Device};
use crate::sys::disklabel::{Disklabel, MAXPARTITIONS, diskminor};
use crate::sys::errno::Errno;
use crate::sys::types::Dev;
use crate::sys::vnode::{VBLK, VCHR, vdevgone};

/// `struct disk sc_dk` in a legacy softc: the kernel's disk handle, `None` (all zero) until
/// the driver constructs it.
pub struct DiskSlot(ReadMostly<Option<DiskRef>>);

impl DiskSlot {
    /// An empty slot (all zero, as in an `M_ZERO` softc).
    pub const fn new() -> Self {
        Self(ReadMostly::new(None))
    }

    /// `disk_construct` with the `dk_name` and `dk_flags` the driver sets before
    /// `disk_attach`: the kernel's block made and held here.
    pub fn construct(&self, name: &[u8], flags: i32) {
        drop(self.0.replace(Some(Disk::new(name, flags))));
    }

    /// Whether the disk was constructed (and not destroyed).
    pub fn is_constructed(&self) -> bool {
        self.0.read().is_some()
    }

    /// Whether the slot's disk is attached (on `disklist`); `false` before `construct` or
    /// after `destroy`.
    pub fn is_attached(&self) -> bool {
        self.0.read().as_ref().is_some_and(|dk| dk.is_attached())
    }

    /// The disk, a handle: `panic(9)` before `construct`.
    pub fn get(&self) -> DiskRef {
        match self.try_get() {
            Some(dk) => dk,
            None => panic(format_args!("disk: the softc's disk is not constructed")),
        }
    }

    /// The disk, a handle, if constructed.
    pub fn try_get(&self) -> Option<DiskRef> {
        self.0.read().clone()
    }

    /// `f` on the disk, without a handle of its own (the I/O path's reads): `panic(9)` before
    /// `construct`. `f` must not sleep (a reader of the slot).
    pub fn with<R>(&self, f: impl FnOnce(&Disk) -> R) -> R {
        match &*self.0.read() {
            Some(dk) => f(dk),
            None => panic(format_args!("disk: the softc's disk is not constructed")),
        }
    }

    /// The slot emptied after `disk_detach`, the handle dropped: at `splbio`, so no disk
    /// interrupt of this CPU reads the slot while the writer waits for its readers.
    pub fn destroy(&self) {
        let s = splbio();
        let old = self.0.replace(None);
        splx(s);
        drop(old);
    }

    /// `dk_openmask`.
    pub fn openmask(&self) -> u64 {
        self.with(Disk::openmask)
    }

    /// `dk_bopenmask`.
    pub fn bopenmask(&self) -> u64 {
        self.with(Disk::bopenmask)
    }

    /// `dk_copenmask`.
    pub fn copenmask(&self) -> u64 {
        self.with(Disk::copenmask)
    }

    /// `*dk_label`, copied out (an ioctl's or an open's: 1,172 bytes, never on the I/O
    /// path, which reads [`DiskSlot::geom`]).
    pub fn label(&self) -> Disklabel {
        let mut lp = Disklabel::zeroed();
        self.with(|dk| dk.label(&mut lp));
        lp
    }

    /// `*dk_label = *lp`, by a holder of the disk lock or the kernel lock.
    pub fn label_store(&self, lp: &Disklabel) {
        self.with(|dk| dk.label_store(lp));
    }

    /// The geometry of partition `part` of the in-core label (the I/O path's).
    pub fn geom(&self, part: u32) -> LabelGeom {
        self.with(|dk| dk.geom(part))
    }

    /// `dk_label`'s address, for `DIOCGPART` (`Disk::label_ptr`).
    pub fn label_ptr(&self) -> *mut Disklabel {
        self.with(Disk::label_ptr)
    }

    /// `&dk_label->d_partitions[part]`'s address, for `DIOCGPART`.
    pub fn partition_ptr(&self, part: u32) -> *mut crate::sys::disklabel::Partition {
        self.with(|dk| dk.partition_ptr(part))
    }
}

impl Default for DiskSlot {
    fn default() -> Self {
        Self::new()
    }
}

/// `disk_attach(dv, &sc->sc_dk)`: the slot's disk attached for the legacy device `dv` (its
/// kernel node, `subr_autoconf`'s `node_of`).
pub fn disk_attach(dv: Option<&Device>, sc_dk: &DiskSlot) {
    let dk = sc_dk.get();
    match dv {
        // A pseudo-device whose `struct device` its driver made by hand (vnd(4)): no node.
        Some(d) if d.dv_cfdata.get().is_none() => {
            let xname = d.dv_xname.get();
            let n = xname.iter().position(|&c| c == 0).unwrap_or(xname.len());
            kernel::dev::disk::disk_attach_unit::<BsdProcGlue>(&xname[..n], d.dv_unit.get(), &dk);
        }
        _ => {
            let node = dv.map(node_of);
            kernel::dev::disk::disk_attach::<BsdProcGlue>(node.as_ref(), &dk);
        }
    }
}

/// `disk_detach(&sc->sc_dk)`.
pub fn disk_detach(sc_dk: &DiskSlot) {
    kernel::dev::disk::disk_detach(&sc_dk.get());
}

/// `disk_openpart(&sc->sc_dk, part, fmt, haslabel)`.
pub fn disk_openpart(sc_dk: &DiskSlot, part: u32, fmt: i32, haslabel: bool) -> Result<(), Errno> {
    sc_dk.with(|dk| kernel::dev::disk::disk_openpart(dk, part, fmt, haslabel))
}

/// `disk_closepart(&sc->sc_dk, part, fmt)`.
pub fn disk_closepart(sc_dk: &DiskSlot, part: u32, fmt: i32) {
    sc_dk.with(|dk| kernel::dev::disk::disk_closepart(dk, part, fmt));
}

/// `disk_busy(&sc->sc_dk)`.
pub fn disk_busy(sc_dk: &DiskSlot) {
    sc_dk.with(kernel::dev::disk::disk_busy);
}

/// `disk_unbusy(&sc->sc_dk, bcount, blkno, read)`.
pub fn disk_unbusy(sc_dk: &DiskSlot, bcount: i64, blkno: crate::sys::types::Daddr, read: bool) {
    sc_dk.with(|dk| kernel::dev::disk::disk_unbusy(dk, bcount, blkno, read));
}

/// `disk_lookup(cd, unit)`: the unit's device, referenced (`device_lookup`; the caller gives
/// it back with `device_unref`), when it has completed attaching its disk. `sc_dk` is the
/// driver's way from its device to its softc's slot (`|dv| &sd_softc(dv).sc_dk`): the C's walk
/// of `disklist` for `dk_device == dv` is the slot's own disk being attached (a softc's disk
/// is attached for that softc's device alone), O(1) and lock-free, as the strategy path wants.
/// The mirror (`cd_dev`) holds an attached unit's softc until its detach clears the slot, and
/// nothing sleeps between the check and `device_lookup` (the callers hold the kernel lock).
pub fn disk_lookup(
    cd: &Cfdriver,
    unit: i32,
    sc_dk: impl FnOnce(NonNull<Device>) -> &'static DiskSlot,
) -> Option<NonNull<Device>> {
    let dv = cd.cd_dev(unit)?;
    if !sc_dk(dv).is_attached() {
        return None;
    }
    device_lookup(cd, unit)
}

/// The driver's strategy routine as the label code calls it (a `kernel::dev::disk::Strat`):
/// the token lent to the driver as the legacy `&'static Buf`, waited for, and taken back.
pub fn label_io(strat: DevTypeStrategy) -> impl FnMut(BufRef) -> (BufRef, Result<(), Errno>) {
    move |t| {
        let bp: &'static Buf = lend(t);
        strat(bp);
        let error = biowait(bp);
        (token_of(bp), error)
    }
}

/// `readdisklabel(dev, strat, lp, spoofonly)` (the kernel's `disksubr`).
pub fn readdisklabel(
    dev: Dev,
    strat: DevTypeStrategy,
    lp: &mut Disklabel,
    spoofonly: bool,
) -> Result<(), Errno> {
    let mut io = label_io(strat);
    kernel::dev::disksubr::readdisklabel::<BsdProcGlue>(dev, &mut io, lp, spoofonly)
}

/// `writedisklabel(dev, strat, lp)` (the kernel's `disksubr`).
pub fn writedisklabel(dev: Dev, strat: DevTypeStrategy, lp: &mut Disklabel) -> Result<(), Errno> {
    let mut io = label_io(strat);
    kernel::dev::disksubr::writedisklabel(dev, &mut io, lp)
}

/// `disk_gone`: revokes the vnodes of every partition of a disk that is going away (the
/// device switch's entries whose open is `open`).
pub fn disk_gone(open: DevTypeOpen, unit: u32) {
    // Locate the lowest minor number to be detached.
    let mn = diskminor(unit, 0);
    let mx = mn + MAXPARTITIONS as u32 - 1;

    for bmaj in 0..nblkdev() {
        if ptr::fn_addr_eq(bdevsw(bmaj).d_open, open) {
            vdevgone(bmaj, mn, mx, VBLK);
        }
    }
    for cmaj in 0..nchrdev() {
        if ptr::fn_addr_eq(cdevsw(cmaj).d_open, open) {
            vdevgone(cmaj, mn, mx, VCHR);
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zeroed_slot_is_empty_constructs_and_is_empty_after_destroy() {
        // An M_ZERO softc's slot.
        // SAFETY: test code; `DiskSlot` is a `ReadMostly` (a zero state word) of an
        // `Option<MArc>` (`#[repr(transparent)]` over a `NonNull`: `None` is all zero).
        let slot: DiskSlot = unsafe { core::mem::zeroed() };
        assert!(!slot.is_constructed());
        assert!(slot.try_get().is_none());

        let (_g, _p) = crate::kern::vfs_subr::tests::setup();
        slot.construct(b"sd9", 0);
        assert!(slot.is_constructed());
        assert_eq!(slot.get().name(), b"sd9");
        assert_eq!(slot.openmask(), 0);
        assert_eq!(slot.geom(0), LabelGeom::default());
        slot.destroy();
        assert!(!slot.is_constructed());
    }

    #[test]
    fn a_detached_disk_is_freed_by_destroy() {
        let (_g, _p) = crate::kern::vfs_subr::tests::setup();
        let slot = DiskSlot::new();
        let before = kernel::testutil::malloc_out(2);
        slot.construct(b"sd8", 0);
        assert!(kernel::testutil::malloc_out(2) > before);
        disk_attach(None, &slot);
        assert!(slot.is_attached());
        disk_detach(&slot);
        assert!(!slot.is_attached());
        slot.destroy();
        // The list's handle went with disk_detach, the slot's with destroy: the block is
        // freed (sddetach, cddetach, wddetach do this after disk_detach).
        assert_eq!(kernel::testutil::malloc_out(2), before);
    }
}
/* </TESTS> */
