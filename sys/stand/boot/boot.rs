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
 * Copyright (c) 2003 Dale Rahn
 * Copyright (c) 1997,1998 Michael Shalayeff
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 */
/* </LICENSES> */

/* <CODE> */
//! boot(8)'s main loop: probe the machine, print the banner, run `/etc/boot.conf`, prompt
//! `boot>`, seed the random generator, load the kernel and start it.
//!
//! Upstream: sys/stand/boot/boot.c @ 3ce1f3f79392
//! LZ: sys/stand/boot/boot.rs@3c62ede68802
//!
//! Compiled as efiboot compiles it: `MDRANDOM`, `FWRANDOM`, `HIBERNATE`.
//!
//! ## Deviations
//! - What the C takes from the program at link time (`machdep()`, `devboot()`,
//!   `run_loadfile()`, `getsecs()`, the `machine` command table, `version[]`, `MACHINE`,
//!   `mdrandom()`, `fwrandom()`, `bootdev_has_hibernate()`, `check_skip_conf()`, `ttyname()`,
//!   `ttydev()`, `cnspeed()`) is one [`BootMd`], registered with [`boot_md_register`]; the
//!   options that select them (`MDRANDOM`, `FWRANDOM`, `HIBERNATE`, `CHECK_SKIP_CONF`,
//!   `BOOT_STTY`, `MACHINE_CMD`) are its `Option`s.
//! - `progname`, `kernelfile`, `boottimeout` and `bootprompt`, which the program may change
//!   before `boot()`, are atomics and cells here; `cmd` is a local of [`boot`] that the
//!   commands get as `&mut CmdState`; `randomctx` is libsa's `RANDOMCTX` (libsa's loader
//!   reads it).
//! - The banner names the operating system as the kernel does: `>> EmiBSD/amd64 BOOTX64
//!   3.71` (`docs/ARCHITECTURE.md`, "The system's identity"); the C prints `OpenBSD`.
//!
//! ## Redesign
//! - N2.4 (decision 22, the boot loaders read a second file): boot() opens
//!   `/abi_openbsd.ko` on the kernel's device (`abi_module_open`) and, after `loadfile`,
//!   reads it onto the page after the kernel's image when it fits the room the program gives
//!   (`BootMd::abi_module_room`, new), raising `marks[MARK_END]` over it and recording it in
//!   `CmdState::abi_module` for `run_loadfile`'s hand-over (amd64 `BOOTARG_ABIMODULE`, arm64
//!   `/chosen`'s `emibsd,abi-module`): the move and the cache clean of `run_loadfile` carry
//!   it, and the kernel's `esym` keeps it out of free memory. Without the file, or without
//!   the room, nothing changes but a line.

use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};

use libkern::staticcell::StaticCell;
use libsa::arc4::{RANDOMCTX, rc4_keysetup, rc4_skip};
use libsa::cread::{close, open, read};
use libsa::dev::errno;
use libsa::fchmod::fchmod;
use libsa::fstat::fstat;
use libsa::hdr::param::MAXPATHLEN;
use libsa::hdr::reboot::{RB_GOODRANDOM, RB_UNHIBERNATE};
use libsa::hdr::stat::{S_ISTXT, S_IXGRP, S_IXOTH, S_IXUSR, Stat};
use libsa::hdr::types::{Dev, Time};
use libsa::loadfile::{LOAD_ALL, MARK_END, MARK_MAX, MARK_START, loadfile};
use libsa::printf;
use libsa::printf::Str;
use libsa::saerrno::Errno;
use libsa::snprintf;
use libsa::stand::{BOOTRANDOM, BOOTRANDOM_MAX};
use libsa::strerror::strerror;

use crate::cmd::{BOOTDEVLEN, CmdState, CmdTable, cstr, getcmd, read_conf, strlcpy, upgrade};

/// `KERNEL`: the default kernel.
pub const KERNEL: &[u8] = b"/bsd";

/// The `BOOT_STTY` routines of the program.
pub struct Stty {
    /// `ttyname(0)`: the console's name (NUL-terminated).
    pub ttyname: fn() -> [u8; 8],
    /// `ttydev(name)`: the device of a console name, `NODEV` if none.
    pub ttydev: fn(&[u8]) -> Dev,
    /// `cnspeed(dev, sp)`: set (`sp > 0`) or get the speed; the old one.
    pub cnspeed: fn(Dev, i32) -> i32,
}

/// What boot(8) takes from the program that links it.
pub struct BootMd {
    /// `MACHINE`: the architecture's name in the banner.
    pub machine: &'static str,
    /// `version[]` (the program's `conf.c`).
    pub version: &'static str,
    /// `machdep()`: probe the machine (console, memory, disks).
    pub machdep: fn(),
    /// `devboot(bootdev, buf)`: the name of the boot device into `buf`.
    pub devboot: fn(Dev, &mut [u8; BOOTDEVLEN]),
    /// `run_loadfile(marks, howto)`: start the loaded kernel; does not return when it works.
    pub run_loadfile: fn(&mut CmdState, &mut [u64; MARK_MAX], i32),
    /// `getsecs()`: seconds of the real-time clock.
    pub getsecs: fn() -> Time,
    /// `MACHINE_CMD`: the `machine` commands.
    pub cmd_machine: Option<&'static [CmdTable]>,
    /// `CHECK_SKIP_CONF`: true to skip `boot.conf` (a key held down).
    pub check_skip_conf: Option<fn() -> bool>,
    /// `MDRANDOM`: mix machine entropy into the seed; 0 when it did.
    pub mdrandom: Option<fn(&mut [u8]) -> i32>,
    /// `FWRANDOM`: mix firmware entropy into the seed; 0 when it did.
    pub fwrandom: Option<fn(&mut [u8]) -> i32>,
    /// `HIBERNATE`: `bootdev_has_hibernate()`.
    pub bootdev_has_hibernate: Option<fn() -> bool>,
    /// `BOOT_STTY`.
    pub stty: Option<Stty>,
    /// Not in the C (N2.4, decision 22): the end of the room the program reserved for the
    /// kernel (the load space `LOADADDR` maps into), where boot() puts `/abi_openbsd.ko`
    /// after the kernel's image when it fits; `None` loads no module.
    pub abi_module_room: Option<fn() -> u64>,
}

/// The program's routines (null until [`boot_md_register`]).
static BOOT_MD: AtomicPtr<BootMd> = AtomicPtr::new(core::ptr::null_mut());

/// `progname`: the program's name in the banner ("BOOT"; efiboot sets "BOOTX64").
pub static PROGNAME: StaticCell<&'static str> = StaticCell::new("BOOT");
/// `kernelfile`: the kernel to load (can be changed by MD code).
pub static KERNELFILE: StaticCell<&'static [u8]> = StaticCell::new(KERNEL);
/// `boottimeout`: seconds before the prompt boots (can be changed by MD code).
pub static BOOTTIMEOUT: AtomicI32 = AtomicI32::new(5);
/// `bootprompt`: MD code can clear it to avoid the prompt the first time round.
pub static BOOTPROMPT: AtomicBool = AtomicBool::new(true);
/// `prog_ident[40]`: the banner.
static PROG_IDENT: StaticCell<[u8; 40]> = StaticCell::new([0; 40]);
/// `rnddata[BOOTRANDOM_MAX]`: the random seed handed to the kernel.
pub static RNDDATA: StaticCell<[u8; BOOTRANDOM_MAX]> = StaticCell::new([0; BOOTRANDOM_MAX]);

/// Registers the program's routines; it calls this before [`boot`].
pub fn boot_md_register(md: &'static BootMd) {
    BOOT_MD.store(core::ptr::from_ref(md).cast_mut(), Ordering::Release);
}

/// The program's routines.
pub fn boot_md() -> &'static BootMd {
    let p = BOOT_MD.load(Ordering::Acquire);
    if p.is_null() {
        libsa::exit::panic(format_args!("boot: no machine-dependent routines"));
    }
    // SAFETY: a non-null value was stored by `boot_md_register` from a `&'static BootMd`.
    unsafe { &*p }
}

/// `prog_ident`, the banner (NUL-terminated).
pub fn prog_ident() -> [u8; 40] {
    // SAFETY: written once by `boot` before anything reads it; boot(8) is single-threaded.
    unsafe { PROG_IDENT.read() }
}

/// `boot(bootdev)`.
pub fn boot(bootdev: Dev) -> ! {
    let md = boot_md();
    let mut isupgrade = false;
    let mut tries = 0;
    let mut marks = [0u64; MARK_MAX];
    let mut cmd = CmdState::new();

    (md.machdep)();

    // SAFETY: single-threaded; no other reference to PROGNAME or PROG_IDENT is live.
    unsafe {
        let progname = PROGNAME.read();
        snprintf!(
            PROG_IDENT.get_mut(),
            ">> EmiBSD/{} {} {}",
            md.machine,
            progname,
            md.version
        );
    }
    printf!("{}\n", Str(&prog_ident()));

    (md.devboot)(bootdev, &mut cmd.bootdev);
    // SAFETY: single-threaded; a copy of the value.
    strlcpy(&mut cmd.image, unsafe { KERNELFILE.read() });
    cmd.boothowto = 0;
    cmd.conf = b"/etc/boot.conf";
    cmd.timeout = BOOTTIMEOUT.load(Ordering::Relaxed);

    if upgrade(&mut cmd) {
        strlcpy(&mut cmd.image, b"/bsd.upgrade");
        printf!("upgrade detected: switching to {}\n", Str(&cmd.image));
        isupgrade = true;
    }

    let mut st = read_conf(&mut cmd);

    if let Some(has_hib) = md.bootdev_has_hibernate
        && has_hib()
    {
        strlcpy(&mut cmd.image, b"/bsd.booted");
        printf!("unhibernate detected: switching to {}\n", Str(&cmd.image));
        cmd.boothowto |= RB_UNHIBERNATE;
    }

    if !BOOTPROMPT.load(Ordering::Relaxed) {
        let (dev, image) = (cmd.bootdev, cmd.image);
        snprintf!(&mut cmd.path, "{}:{}", Str(&dev), Str(&image));
    }

    loop {
        // no boot.conf, or no boot cmd in there
        if BOOTPROMPT.load(Ordering::Relaxed) && st <= 0 {
            loop {
                printf!("boot> ");
                if getcmd(&mut cmd) != 0 {
                    break;
                }
            }
        }

        // SAFETY: single-threaded; the seed buffer and the generator are used only here and
        // by loadfile, which runs after this block.
        unsafe {
            let rnddata = RNDDATA.get_mut();
            if loadrandom(&cmd, BOOTRANDOM, rnddata) == 0 {
                cmd.boothowto |= RB_GOODRANDOM;
            }
            if let Some(mdrandom) = md.mdrandom
                && mdrandom(rnddata) == 0
            {
                cmd.boothowto |= RB_GOODRANDOM;
            }
            if let Some(fwrandom) = md.fwrandom
                && fwrandom(rnddata) == 0
            {
                cmd.boothowto |= RB_GOODRANDOM;
            }
            rc4_keysetup(RANDOMCTX.get_mut(), rnddata);
            rc4_skip(RANDOMCTX.get_mut(), 1536);
        }

        st = 0;
        BOOTPROMPT.store(true, Ordering::Relaxed); // allow reselect should we fail

        printf!("booting {}: ", Str(&cmd.path));
        marks[MARK_START] = 0;
        let path = cmd.path;
        let module = md
            .abi_module_room
            .and_then(|room| abi_module_open(&cmd).map(|(fd, len)| (fd, len, room())));
        cmd.abi_module = None;
        // SAFETY: the program's LOADADDR maps the kernel into the memory it reserved for it
        // (efiboot: the 64 MB `efi_memprobe` allocates), the contract of `BootMd`. The
        // module is read straight into the page after the kernel's image (`marks[MARK_END]`,
        // a load-space address as every mark is) only when it ends before `room`, the end of
        // that same reserved memory (`BootMd::abi_module_room`), so it overwrites nothing
        // the program or the kernel's image uses, and no heap is spent on it.
        let loaded = unsafe {
            let loaded = loadfile(cstr(&path), &mut marks, LOAD_ALL);
            if let (Ok(_), Some((fd, len, room))) = (&loaded, module) {
                let at = marks[MARK_END].next_multiple_of(ABI_MODULE_ALIGN);
                match at.checked_add(len).filter(|&end| end <= room) {
                    Some(end) => {
                        let dst =
                            core::slice::from_raw_parts_mut(at as usize as *mut u8, len as usize);
                        if read(fd, dst) == Ok(len as usize) {
                            marks[MARK_END] = end;
                            cmd.abi_module = Some((at, len));
                        } else {
                            printf!("{}: cannot read it whole\n", Str(ABI_MODULE));
                        }
                    }
                    None => printf!("{}: no room after the kernel\n", Str(ABI_MODULE)),
                }
            }
            loaded
        };
        if let Some((fd, _, _)) = module {
            let _ = close(fd);
        }
        if let Ok(fd) = loaded {
            // Prevent re-upgrade: chmod a-x bsd.upgrade
            if isupgrade {
                let mut sb = Stat::default();
                if fstat(fd, &mut sb).is_ok() {
                    sb.st_mode &= !(S_IXUSR | S_IXGRP | S_IXOTH);
                    if fchmod(fd, sb.st_mode).is_err() {
                        printf!("fchmod a-x {}: failed\n", Str(&cmd.path));
                    }
                }
            }
            let _ = close(fd);
            break;
        }

        // SAFETY: single-threaded; MD code only writes KERNELFILE before boot().
        unsafe { KERNELFILE.write(KERNEL) };
        tries += 1;
        strlcpy(&mut cmd.image, KERNEL);
        printf!(" failed({}). will try {}\n", errno().0, Str(KERNEL));

        if tries < 2 {
            if cmd.timeout > 0 {
                cmd.timeout += 1;
            }
        } else {
            if cmd.timeout != 0 {
                printf!("Turning timeout off.\n");
            }
            cmd.timeout = 0;
        }
    }

    // exec
    let howto = cmd.boothowto;
    (md.run_loadfile)(&mut cmd, &mut marks, howto);
    libsa::exit::panic(format_args!("run_loadfile returned"))
}

/// `loadrandom(name, buf, buflen)`: read the random seed file on the kernel's device into
/// `buf`, and mark it used (sticky bit); 0 when a fresh seed was read.
pub fn loadrandom(cmd: &CmdState, name: &[u8], buf: &mut [u8]) -> i32 {
    let mut path = [0u8; MAXPATHLEN];

    // Extract the device name from the kernel we are loading.
    let kpath = cstr(&cmd.path);
    match kpath.iter().position(|&c| c == b':') {
        Some(i) => {
            snprintf!(&mut path, "{}:{}", Str(&kpath[..i]), Str(name));
        }
        None => {
            snprintf!(&mut path, "{}:{}", Str(&cmd.bootdev), Str(name));
        }
    }

    let fd = match open(cstr(&path), 0) {
        Ok(fd) => fd,
        Err(e) => {
            if e != Errno::EPERM {
                printf!("cannot open {}: {}\n", Str(&path), strerror(e));
            }
            return -1;
        }
    };
    let mut error = 0;
    let mut sb = Stat::default();
    if fstat(fd, &mut sb).is_err() || read(fd, buf) != Ok(buf.len()) {
        error = -1;
    } else if (sb.st_mode & S_ISTXT) != 0 {
        printf!("NOTE: random seed is being reused.\n");
        error = -1;
    } else {
        let _ = fchmod(fd, sb.st_mode | S_ISTXT);
    }
    let _ = close(fd);
    error
}

/// Not in the C (N2.4, decision 22): the ABI module the kernel loads before init(8), read
/// from the device of the kernel being booted.
pub const ABI_MODULE: &[u8] = b"/abi_openbsd.ko";

/// Where the module goes after the kernel's image: the next page.
const ABI_MODULE_ALIGN: u64 = 4096;

/// Not in the C (N2.4): `/abi_openbsd.ko` on the device of the kernel being loaded, opened:
/// its descriptor and length, which boot() reads straight into the room after the kernel;
/// `None` without one (the kernel then takes every system call through its legacy path and
/// says so). No buffer: an oversized or corrupt length is refused by boot()'s room check,
/// never allocated (efiboot's heap is 1 MiB and panics when full).
pub fn abi_module_open(cmd: &CmdState) -> Option<(usize, u64)> {
    let mut path = [0u8; MAXPATHLEN];
    let kpath = cstr(&cmd.path);
    match kpath.iter().position(|&c| c == b':') {
        Some(i) => {
            snprintf!(&mut path, "{}:{}", Str(&kpath[..i]), Str(ABI_MODULE));
        }
        None => {
            snprintf!(&mut path, "{}:{}", Str(&cmd.bootdev), Str(ABI_MODULE));
        }
    }
    let fd = open(cstr(&path), 0).ok()?;
    let mut sb = Stat::default();
    let len = if fstat(fd, &mut sb).is_ok() {
        u64::try_from(sb.st_size).ok()
    } else {
        None
    };
    if len.is_none() {
        printf!("cannot stat {}\n", Str(&path));
        let _ = close(fd);
    }
    len.map(|len| (fd, len))
}
/* </CODE> */
