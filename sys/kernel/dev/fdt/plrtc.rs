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
 * Copyright (c) 2015 Jonathan Gray <jsg@openbsd.org>
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
//! The ARM PrimeCell PL031 real-time clock on the device tree in the kernel: `dev/fdt/plrtc.c`.
//!
//! Upstream: sys/dev/fdt/plrtc.c @ 3ce1f3f79392
//! LZ: sys/dev/fdt/plrtc.rs@e8249449d905
//!
//! The counter is the time in seconds: [`Plrtc`]'s `gettime` reads `RTCDR`, `settime` loads
//! `RTCLR`; the attach starts the clock (`RTCCR_START`) after handing the chip to
//! `todr_attach` (quality 0, so it wins over efi(4)'s -1000). Under QEMU with EDK2 the
//! firmware disables the PL031's node and keeps it for its own `GetTime`, so the driver does
//! not attach there (no smoke reaches it: P52).
//!
//! ## Deviations
//! - The chip is leaked once per attach (`MBox::leak`), as the C's `malloc`ed
//!   `todr_chip_handle` is never freed (no detach).
//!
//! ## Redesign
//! - `plrtc_ca`/`plrtc_cd` -> [`PLRTC`], a [`DriverOps`] (decision 46) whose probe reads the
//!   bus's [`FdtArgs`]; `cfdata[]`'s row 7 names it (`Cfdata::native`) with a stub `Cfdriver`
//!   (`bsd`'s `conf/ioconf.rs`).
//! - `struct plrtc_softc` (LZ: `#[repr(C)]`, `Cell`s of the tag and the handle) and the
//!   `malloc`ed `todr_chip_handle` whose cookie named it (LZ: two `unsafe` to write and leak it,
//!   one to cast the cookie back) -> [`Plrtc`], the mapping itself ([`Mmio<Pl031>`]), leaked
//!   once and handed to `todr_attach` as the kernel's [`Todr`]; the device's softc is empty
//!   ([`PlrtcDev`]). `bus_space_map` is the bus's `FdtBus::map` of the node's first `reg`.

use hal::dev::mmio::{DevCache, Mmio, Reg, RegBlock};
use hal::mem::malloc::{M_NOWAIT, MArc, MBox};
use hal::mem::pool::AnyArc;
use sysdefs::base::errno::Errno;

use crate::assert::kpanic;
use crate::base::prf::printf;
use crate::dev::arm64::fdt::FdtArgs;
use crate::dev::device::{
    Aux, DV_DULL, DevRef, Devclass, DriverOps, Ident, M_DEVBUF, Match, UnitTable,
};
use crate::dev::ofw::fdt::OF_is_compatible;
use crate::time::clock::Timeval;
use crate::time::{Todr, todr_attach};

/// The PL031's registers, up to `RTCICR`.
pub struct Pl031;

impl RegBlock for Pl031 {
    const SIZE: usize = 0x20;
}

/// `RTCDR`: data register.
const RTCDR: Reg<Pl031, u32> = Reg::new(0x00);
/// `RTCMR`: match register.
#[allow(dead_code)] // the C defines it; the driver does not use it
const RTCMR: Reg<Pl031, u32> = Reg::new(0x04);
/// `RTCLR`: load register.
const RTCLR: Reg<Pl031, u32> = Reg::new(0x08);
/// `RTCCR`: control register.
const RTCCR: Reg<Pl031, u32> = Reg::new(0x0c);

/// `RTCCR_START`.
const RTCCR_START: u32 = 1 << 0;

/// The attached chip: `sc_iot` and `sc_ioh` as the mapping, the C's `todr_chip_handle`.
pub struct Plrtc {
    /// The registers.
    regs: Mmio<Pl031>,
}

/// The device's softc: the state is the leaked [`Plrtc`]'s.
pub struct PlrtcDev;

/// `plrtc_ca` with `plrtc_cd`'s constants.
pub struct PlrtcDriver;

/// The driver `cfdata[]`'s `plrtc* at fdt?` row names (`Cfdata::native`).
pub static PLRTC: PlrtcDriver = PlrtcDriver;

/// `plrtc_cd`'s `cd_devs`.
static PLRTC_UNITS: UnitTable = UnitTable::new();

impl Plrtc {
    /// `plrtc_gettime`: the counter is the time in seconds.
    pub fn plrtc_gettime(&self) -> Timeval {
        Timeval {
            tv_sec: i64::from(self.regs.read(RTCDR)),
            tv_usec: 0,
        }
    }

    /// `plrtc_settime`: load the counter (the C's `bus_space_write_4` of `tv_sec`, truncated
    /// to the register's 32 bits).
    pub fn plrtc_settime(&self, tv: &Timeval) {
        self.regs.write(RTCLR, tv.tv_sec as u32);
    }

    /// Enable the rtc.
    fn start(&self) {
        self.regs.write(RTCCR, RTCCR_START);
    }
}

impl Todr for Plrtc {
    fn quality(&self) -> i32 {
        0
    }

    fn gettime(&self) -> Result<Timeval, Errno> {
        Ok(self.plrtc_gettime())
    }

    fn settime(&self, tv: &Timeval) -> Result<(), Errno> {
        self.plrtc_settime(tv);
        Ok(())
    }
}

/// `plrtc_match`: a node compatible with `arm,pl031`.
pub fn plrtc_match(fa: &FdtArgs) -> i32 {
    i32::from(OF_is_compatible(fa.node, b"arm,pl031"))
}

impl DriverOps for PlrtcDriver {
    fn name(&self) -> &'static [u8] {
        b"plrtc"
    }
    fn class(&self) -> Devclass {
        DV_DULL
    }
    fn mode(&self) -> i32 {
        0
    }
    fn units(&self) -> &'static UnitTable {
        &PLRTC_UNITS
    }
    fn make_softc(&self, _id: &Ident<'_>) -> AnyArc {
        match MArc::new(M_DEVBUF, M_NOWAIT, PlrtcDev) {
            Some(sc) => MArc::erase(sc),
            None => kpanic(format_args!(
                "config_make_softc: allocation for device softc failed"
            )),
        }
    }
    fn probe(&self, _parent: Option<&DevRef>, _m: &Match, aux: &Aux<'_>) -> i32 {
        FdtArgs::of_aux(aux).map_or(0, plrtc_match)
    }
    fn attach(&self, _parent: Option<&DevRef>, dev: &DevRef, aux: &Aux<'_>) {
        let Some(fa) = FdtArgs::of_aux(aux) else {
            kpanic(format_args!("{}: not a device-tree node", dev.xname()));
        };
        plrtc_attach(fa);
    }
}

/// `plrtc_attach`'s mapping: the node's first `reg` through its bus, or the line the C
/// prints when there is none or it cannot be mapped.
fn plrtc_map(fa: &FdtArgs) -> Result<Mmio<Pl031>, &'static str> {
    let Some(reg) = fa.reg().first() else {
        return Err(": no register data\n");
    };
    fa.bus
        .map::<Pl031>(reg.addr, reg.size, DevCache::Device)
        .map_err(|_| ": failed to map mem space\n")
}

/// `plrtc_attach`: map the registers, hand the chip to `todr_attach` and start it.
pub fn plrtc_attach(fa: &FdtArgs) {
    let regs = match plrtc_map(fa) {
        Ok(regs) => regs,
        Err(line) => {
            printf(format_args!("{line}"));
            return;
        }
    };

    let Some(sc) = MBox::new(M_DEVBUF, M_NOWAIT, Plrtc { regs }) else {
        kpanic(format_args!("couldn't allocate todr_handle"));
    };
    let sc: &'static Plrtc = MBox::leak(sc);
    todr_attach(sc);

    // enable the rtc
    sc.start();

    printf(format_args!("\n"));
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The PL031 is a page of host memory mapped with `km::map_device`: a register reads back
    // what was last written; a test writes the "hardware's" counter itself.

    use std::boxed::Box;
    use std::sync::MutexGuard;

    use sysdefs::base::types::Paddr;

    use super::*;
    use crate::dev::arm64::fdt::FdtBus;
    use crate::dev::ofw::fdt::OF_finddevice;
    use crate::dev::ofw::fdt::testfdt::{FdtBuilder, install};
    use crate::testutil;

    const PAGE: usize = 4096;

    fn device_frame() -> Paddr {
        #[repr(C, align(4096))]
        struct DevPage([u8; PAGE]);
        let p = Box::into_raw(Box::new(DevPage([0; PAGE])));
        Paddr::new(p.expose_provenance())
    }

    fn setup() -> MutexGuard<'static, ()> {
        let g = testutil::pages_install(4 << 20);
        testutil::kernel_maps_install();
        g
    }

    fn chip() -> Plrtc {
        let regs: Mmio<Pl031> =
            crate::vm::km::map_device(device_frame(), PAGE, DevCache::Device).unwrap();
        Plrtc { regs }
    }

    #[test]
    fn the_counter_is_the_time_in_seconds() {
        let _g = setup();
        let sc = chip();
        sc.regs.write(RTCDR, 1_795_838_400);
        assert_eq!(
            sc.plrtc_gettime(),
            Timeval {
                tv_sec: 1_795_838_400,
                tv_usec: 0
            }
        );
        sc.plrtc_settime(&Timeval {
            tv_sec: 0x1_2345_6789,
            tv_usec: 7,
        });
        assert_eq!(
            sc.regs.read(RTCLR),
            0x2345_6789,
            "the low 32 bits, as the C"
        );
        sc.start();
        assert_eq!(sc.regs.read(RTCCR), RTCCR_START);
        assert_eq!(sc.quality(), 0);
        assert_eq!(sc.gettime().map(|t| t.tv_sec), Ok(1_795_838_400));
    }

    #[test]
    fn match_and_the_attach_s_two_failure_lines() {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"#address-cells", &[2]);
        b.prop_cells(b"#size-cells", &[2]);
        b.begin(b"pl031@9010000");
        b.prop(b"compatible", b"arm,pl031\0arm,primecell\0");
        b.end();
        b.begin(b"other@0");
        b.prop(b"compatible", b"arm,pl011\0");
        b.end();
        b.end();
        let _t = install(b.finish());
        let _g = setup();
        let bus = FdtBus::root();
        let fa = FdtArgs::of_node(OF_finddevice(b"/pl031@9010000"), &bus, 2, 2);
        assert_eq!(plrtc_match(&fa), 1);
        assert_eq!(plrtc_map(&fa).err(), Some(": no register data\n"));
        let other = FdtArgs::of_node(OF_finddevice(b"/other@0"), &bus, 2, 2);
        assert_eq!(plrtc_match(&other), 0);
    }
}
/* </TESTS> */
