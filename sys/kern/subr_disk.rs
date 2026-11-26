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
 * Copyright (c) 1982, 1986, 1988, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)ufs_disksubr.c	8.5 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The root, swap and dump device choice of `subr_disk.c` (`setroot`, `getdisk`,
//! `parsedisk`, `dk_mountroot`), over the kernel's disk list (`kernel::dev::disk`) until it
//! moves into the kernel with `rootdev`, `dumpdev`, `swdevt` and `mountroot` (N7.1d's next
//! step).
//!
//! Upstream: sys/kern/subr_disk.c @ 3ce1f3f79392
//!
//! The rest of `subr_disk.c` (the label code, the disk list, `disk_map`, `disk_readlabel`)
//! is the kernel's (`kernel::dev::disk`, `kernel::dev::disksubr`, N7.1d); `bsd`'s drivers
//! reach it through `sys/sys/disk.rs`.
//!
//! ## Deviations
//! - `setroot`'s `RB_ASKNAME` dialogue (the "root device:" and "swap device:" prompts read
//!   with `getsn` under `cnpollc`) is reported and skipped: `getsn` is not ported. A kernel
//!   booted with `-a` goes on with its configured root. `NFSCLIENT` (feature `nfsclient`):
//!   the `nfs_mountroot` branches are ported (`nfs_mountroot` is a two-line call of the
//!   kernel's since N6.7a5, so that `setroot`'s address test has a `bsd` function);
//!   `swapdev = NODEV` (a constant of `conf.c` here) is not assigned, nothing reads it
//!   before `swdevt[0]` is.
//! - `dk_mountroot`: `EXT2FS` (feature `ext2fs`), `FFS` (feature `ffs`) and `CD9660` (feature
//!   `cd9660`) are the file systems the kernel configuration names with a mountroot.
//! - `parsedisk` returns the device and the `dev_t` as an `Option` pair instead of filling
//!   `*devp`; `getdisk` likewise.

use core::fmt;
use core::ptr;
use core::sync::atomic::Ordering;

use kernel::dev::disk::{
    DUID_SIZE, Disk, DiskRef, disk_next, disk_readlabel, duid_equal, duid_format, duid_iszero,
    findblkmajor, findblkname,
};
use libkern::StaticCell;

use crate::dev::softraid::sr_map_root;
use crate::kern::init_main::BOOTHOWTO;
use crate::kern::kern_synch::tsleep_nsec;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_autoconf::{device_hold, node_of};
use crate::kern::subr_prf::Str;
use crate::kern::subr_prf::{panic, printf};
use crate::net::if_::{if_addgroup, if_put, if_unit};
#[cfg(feature = "nfsclient")]
use kernel::fs::nfs::nfs_boot::set_nfsbootdevname;

use crate::sys::conf::SWDEVT;
use crate::sys::device::Device;
use crate::sys::device::{DV_DISK, DV_IFNET};
use crate::sys::disk::DKF_OPENED;
use crate::sys::disklabel::{
    Disklabel, FS_BSDFFS, FS_EXT2FS, MAXPARTITIONS, diskpart, diskunit, dl_getpsize,
    dl_partname2num, dl_partnum2name, makediskdev,
};
use crate::sys::errno::Errno;
use crate::sys::param::NODEV;
use crate::sys::reboot::RB_ASKNAME;
#[cfg(feature = "nfsclient")]
use crate::sys::systm::MountrootFn;
use crate::sys::systm::{DUMPDEV, MOUNTROOT, ROOTDEV};
use crate::sys::time::sec_to_nsec;
use crate::sys::types::{Dev, major};
use crate::unported;

/// `bootduid`: DUID of boot disk. Written by the machine's boot glue before autoconf.
pub static BOOTDUID: StaticCell<[u8; DUID_SIZE]> = StaticCell::new([0; DUID_SIZE]);
/// `rootduid`: DUID of root disk. Written by `setroot`.
pub static ROOTDUID: StaticCell<[u8; DUID_SIZE]> = StaticCell::new([0; DUID_SIZE]);

/// `rootdv`: the root device, chosen by `setroot`.
pub static ROOTDV: core::sync::atomic::AtomicPtr<Device> =
    core::sync::atomic::AtomicPtr::new(ptr::null_mut());

/// The first attached disk for which `f` holds (`TAILQ_FOREACH` ... `break`).
fn disk_find(f: impl Fn(&Disk) -> bool) -> Option<DiskRef> {
    let mut d = disk_next(None);
    while let Some(dk) = d {
        if f(&dk) {
            return Some(dk);
        }
        d = disk_next(Some(&dk));
    }
    None
}

/// `nfs_mountroot`: the `mountroot` of a root on NFS, the kernel's (N6.7a5) over `swdevt[0]`
/// and the machine's `swapdev`, as `dk_mountroot`'s file systems' (`setroot` compares
/// `mountroot` with this function's address).
#[cfg(feature = "nfsclient")]
pub fn nfs_mountroot() -> Result<(), Errno> {
    kernel::fs::nfs::nfs_vfsops::nfs_mountroot::<crate::kern::proc_glue::BsdProcGlue>(
        &crate::sys::conf::SWDEVT[0],
        crate::machine::conf::swapdev(),
    )
}

/// `dk_mountroot`: the `mountroot` of a disk root: reads `rootdev`'s label and mounts the
/// root partition with the mountroot of its file system type.
pub fn dk_mountroot() -> Result<(), Errno> {
    let rootdev = ROOTDEV.load(Ordering::Relaxed);
    let part = diskpart(rootdev) as usize;

    let mut dl = Disklabel::zeroed();
    if let Err(error) = disk_readlabel::<BsdProcGlue>(&mut dl, rootdev) {
        panic(format_args!("{error}"));
    }

    if dl_getpsize(&dl.d_partitions[part]) == 0 {
        panic(format_args!("root filesystem has size 0"));
    }
    let fstype = dl.d_partitions[part].p_fstype;
    #[cfg(feature = "ext2fs")]
    if fstype == FS_EXT2FS {
        // The kernel's (N5.6c3), over `rootdev` and the machine's `swapdev`, as the ffs
        // adapter's `ffs_mountroot` (no ext2fs adapter: this is its one caller).
        return kernel::fs::ufs::ext2fs::ext2fs_vfsops::ext2fs_mountroot::<
            crate::kern::proc_glue::BsdProcGlue,
        >(rootdev, crate::machine::conf::swapdev());
    }
    #[cfg(feature = "cd9660")]
    if fstype == crate::sys::disklabel::FS_ISO9660 {
        // The kernel's (N5.8c2), over `rootdev` and the machine's `swapdev`, as ext2fs's.
        return kernel::fs::isofs::cd9660::cd9660_vfsops::cd9660_mountroot::<
            crate::kern::proc_glue::BsdProcGlue,
        >(rootdev, crate::machine::conf::swapdev());
    }
    #[cfg(feature = "ffs")]
    {
        if fstype != FS_BSDFFS {
            let _ = printf(format_args!(
                "filesystem type {fstype} not known.. assuming ffs\n"
            ));
        }
        crate::ufs::ffs::ffs_vfsops::ffs_mountroot()
    }
    #[cfg(not(feature = "ffs"))]
    panic(format_args!(
        "disk {rootdev:#x} filesystem type {fstype} not known"
    ));
}

/// `getdisk`: `parsedisk`, listing the choices when `str` names no disk.
pub fn getdisk(str: &[u8], defpart: u32) -> Option<(&'static Device, Dev)> {
    let found = parsedisk(str, defpart);
    if found.is_none() {
        let _ = printf(format_args!("use one of: exit"));
        let mut d = kernel::dev::autoconf::alldevs_next(None);
        while let Some(n) = d {
            // The kernel's node: a native device has no legacy header.
            if n.class() == DV_DISK {
                let _ = printf(format_args!(" {}[a-p]", n.xname()));
            }
            #[cfg(feature = "nfsclient")]
            if n.class() == DV_IFNET {
                let _ = printf(format_args!(" {}", n.xname()));
            }
            d = kernel::dev::autoconf::alldevs_next(Some(&n));
        }
        let _ = printf(format_args!("\n"));
    }
    found
}

/// `parsedisk`: the disk device `str` names (`wd0`, `rd0a`, ...) and its block device for
/// the partition the name ends in (`defpart` without one).
pub fn parsedisk(str: &[u8], defpart: u32) -> Option<(&'static Device, Dev)> {
    let mut len = str.len();
    if len == 0 {
        return None;
    }
    let part = match dl_partname2num(str[len - 1]) {
        Some(n) if n < MAXPARTITIONS => {
            len -= 1;
            n as u32
        }
        _ => defpart,
    };

    let mut d = kernel::dev::autoconf::alldevs_next(None);
    while let Some(n) = d {
        // The kernel's node's class and name; the legacy header only for the match.
        if n.class() == DV_DISK && n.xname().as_bytes() == &str[..len] {
            let majdev = findblkmajor(&n);
            if majdev < 0 {
                return None;
            }
            return Some((
                device_hold(&n),
                makediskdev(majdev as u32, n.unit() as u32, part),
            ));
        }
        #[cfg(feature = "nfsclient")]
        if n.class() == DV_IFNET && n.xname().as_bytes() == &str[..len] {
            return Some((device_hold(&n), NODEV));
        }
        d = kernel::dev::autoconf::alldevs_next(Some(&n));
    }
    None
}

/// `setroot`: chooses the root, swap and dump devices (`rootdev`, `swdevt[0]`, `dumpdev`)
/// from the configuration, the boot device and the boot disk's DUID, and sets `mountroot`.
pub fn setroot(bootdv: Option<&'static Device>, part: u32, exitflags: i32) {
    let _ = exitflags; // reboot(exitflags) answers "exit" at the prompt (RB_ASKNAME, below).
    let mut bootdv = bootdv;
    let mut part = part;

    // Ensure that all disk attach callbacks have completed.
    let mut slept = 0;
    loop {
        let pending = disk_find(|dk| dk.devno() != NODEV && dk.flags() & DKF_OPENED == 0);
        let Some(dk) = pending else { break };
        let _ = tsleep_nsec(ptr::from_ref::<Disk>(&dk), 0, "dkopen", sec_to_nsec(1));
        slept += 1;
        if slept >= 5 {
            break;
        }
    }

    if slept == 5 {
        let _ = printf(format_args!("disklabels not read:"));
        let mut d = disk_next(None);
        while let Some(dk) = d {
            if dk.devno() != NODEV && dk.flags() & DKF_OPENED == 0 {
                let _ = printf(format_args!(" {}", Str(dk.name())));
            }
            d = disk_next(Some(&dk));
        }
        let _ = printf(format_args!("\n"));
    }

    // SAFETY: bootduid and rootduid are written only here, by the one thread running main.
    let bootduid = unsafe { BOOTDUID.get_mut() };
    if duid_iszero(bootduid) {
        // Locate DUID for boot disk since it was not provided.
        let node = bootdv.map(node_of);
        let dk = disk_find(|dk| match &node {
            Some(n) => dk.is_device(n),
            None => dk.device().is_none(),
        });
        if let Some(dk) = dk {
            *bootduid = dk.uid();
        }
    } else if bootdv.is_none() {
        // Locate boot disk based on the provided DUID.
        let dk = disk_find(|dk| duid_equal(&dk.uid(), bootduid));
        if let Some(dv) = dk.and_then(|dk| dk.device()) {
            bootdv = Some(device_hold(&dv));
        }
    }
    // SAFETY: as for bootduid.
    let rootduid = unsafe { ROOTDUID.get_mut() };
    *rootduid = *bootduid;

    // NSOFTRAID > 0
    sr_map_root();

    // If `swap generic' and we couldn't determine boot device, ask the user.
    let mut dk_found: Option<DiskRef> = None;
    // SAFETY: mountroot is written only by the boot path and here, by the thread running main.
    let generic = unsafe { MOUNTROOT.read() }.is_none();
    if generic && bootdv.is_none() {
        BOOTHOWTO.fetch_or(RB_ASKNAME, Ordering::Relaxed);
    }
    let rootdv: &'static Device;
    if BOOTHOWTO.load(Ordering::Relaxed) & RB_ASKNAME != 0 {
        // The "root device" and "swap device" prompts (getsn under cnpollc).
        let _ = unported!("setroot: RB_ASKNAME prompts (getsn)");
        if generic && bootdv.is_none() {
            let _ = printf(format_args!("root device: none configured\n"));
            return;
        }
    }
    let rootdev = ROOTDEV.load(Ordering::Relaxed);
    #[cfg(feature = "nfsclient")]
    let nfs_root =
        // SAFETY: as for MOUNTROOT above.
        unsafe { MOUNTROOT.read() }.is_some_and(|f| core::ptr::fn_addr_eq(f, nfs_mountroot as MountrootFn));
    #[cfg(not(feature = "nfsclient"))]
    let nfs_root = false;
    if nfs_root {
        // `mountroot == nfs_mountroot'
        let Some(dv) = bootdv else { return };
        rootdv = dv;
        ROOTDEV.store(NODEV, Ordering::Relaxed);
        DUMPDEV.store(NODEV, Ordering::Relaxed);
        // swapdev = NODEV: see the module's deviations.
    } else if generic && rootdev == NODEV {
        // `swap generic'
        let Some(mut dv) = bootdv else { return };

        if dv.dv_class.get() == DV_DISK && !duid_iszero(rootduid) {
            let dk = disk_find(|dk| duid_equal(&dk.uid(), rootduid));
            let Some(dk) = dk else {
                panic(format_args!(
                    "root device ({}) not found",
                    Str(&duid_format(rootduid))
                ));
            };
            if let Some(d) = dk.device() {
                dv = device_hold(&d);
            }
            dk_found = Some(dk);
        }
        rootdv = dv;

        let majdev = findblkmajor(&node_of(rootdv));
        let nswapdev = if majdev >= 0 {
            // Root and swap are on the disk. Assume swap is on partition b.
            let unit = rootdv.dv_unit.get() as u32;
            ROOTDEV.store(makediskdev(majdev as u32, unit, part), Ordering::Relaxed);
            makediskdev(majdev as u32, unit, 1)
        } else {
            // Root and swap are on a net.
            NODEV
        };
        DUMPDEV.store(nswapdev, Ordering::Relaxed);
        SWDEVT[0].store(nswapdev, Ordering::Relaxed);
    } else {
        // Completely pre-configured, but we want rootdv ..
        let majdev = major(rootdev) as i32;
        let Some(name) = findblkname(majdev) else {
            return;
        };
        let unit = diskunit(rootdev);
        part = diskpart(rootdev);
        let mut buf = [0u8; 128];
        let len = bfmt(
            &mut buf,
            format_args!(
                "{}{}{}",
                Str(name),
                unit,
                char::from(dl_partnum2name(part as usize).unwrap_or(b'?'))
            ),
        );
        let Some((dv, _)) = parsedisk(&buf[..len], 0) else {
            panic(format_args!("root device ({}) not found", Str(&buf[..len])));
        };
        rootdv = dv;
    }
    ROOTDV.store(ptr::from_ref(rootdv).cast_mut(), Ordering::Relaxed);

    if let Some(bdv) = bootdv
        && bdv.dv_class.get() == DV_IFNET
        && let Some(ifp) = if_unit(bdv.xname().as_bytes())
    {
        let _ = if_addgroup(ifp, b"netboot");
        if_put(ifp);
    }

    if rootdv.dv_class.get() == DV_DISK {
        // SAFETY: as for MOUNTROOT above.
        unsafe { MOUNTROOT.write(Some(dk_mountroot)) };
        part = diskpart(ROOTDEV.load(Ordering::Relaxed));
    } else {
        #[cfg(feature = "nfsclient")]
        if rootdv.dv_class.get() == DV_IFNET {
            // SAFETY: as for MOUNTROOT above.
            unsafe { MOUNTROOT.write(Some(nfs_mountroot)) };
            set_nfsbootdevname(rootdv.xname().as_bytes());
            return;
        }
        let _ = printf(format_args!(
            "can't figure root, hope your kernel is right\n"
        ));
        return;
    }

    let partname = char::from(dl_partnum2name(part as usize).unwrap_or(b'?'));
    let _ = printf(format_args!("root on {}{partname}", rootdv.xname()));

    if let Some(dk) = dk_found
        && dk.is_device(&node_of(rootdv))
    {
        let _ = printf(format_args!(
            " ({}.{partname})",
            Str(&duid_format(rootduid))
        ));
    }

    // Make the swap partition on the root drive the primary swap.
    let rootdev = ROOTDEV.load(Ordering::Relaxed);
    let mut temp = NODEV;
    let mut found = None;
    for (i, sw) in SWDEVT.iter().enumerate() {
        let d = sw.load(Ordering::Relaxed);
        if d == NODEV {
            break;
        }
        if major(rootdev) == major(d) && diskunit(rootdev) == diskunit(d) {
            temp = SWDEVT[0].load(Ordering::Relaxed);
            SWDEVT[0].store(d, Ordering::Relaxed);
            sw.store(temp, Ordering::Relaxed);
            found = Some(i);
            break;
        }
    }
    if found.is_some() {
        // If dumpdev was the same as the old primary swap device, move it to the new
        // primary swap device.
        if temp == DUMPDEV.load(Ordering::Relaxed) {
            DUMPDEV.store(SWDEVT[0].load(Ordering::Relaxed), Ordering::Relaxed);
        }
    }
    let print_dev = |what: &str, d: Dev| {
        let name = findblkname(major(d) as i32).unwrap_or(b"??");
        let p = char::from(dl_partnum2name(diskpart(d) as usize).unwrap_or(b'?'));
        let _ = printf(format_args!(" {what} on {}{}{p}", Str(name), diskunit(d)));
    };
    let sw0 = SWDEVT[0].load(Ordering::Relaxed);
    if sw0 != NODEV {
        print_dev("swap", sw0);
    }
    let dumpdev = DUMPDEV.load(Ordering::Relaxed);
    if dumpdev != NODEV {
        print_dev("dump", dumpdev);
    }
    let _ = printf(format_args!("\n"));
}

/// Formats into `buf` (the C's `snprintf`), truncating; returns the length written.
fn bfmt(buf: &mut [u8], args: fmt::Arguments<'_>) -> usize {
    struct W<'a>(&'a mut [u8], usize);
    impl fmt::Write for W<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let n = s.len().min(self.0.len() - self.1);
            self.0[self.1..self.1 + n].copy_from_slice(&s.as_bytes()[..n]);
            self.1 += n;
            Ok(())
        }
    }
    let mut w = W(buf, 0);
    let _ = fmt::write(&mut w, args);
    w.1
}
/* </CODE> */
