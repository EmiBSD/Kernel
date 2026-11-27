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
//! Boot glue: the kernel's entry from OpenBSD's boot(8) (efiboot, M14), ending in
//! [`start_kernel`].
//!
//! The machine's entry code (`locore0.S` on both archs) calls `bootarg.rs`'s `bootarg_main`,
//! which takes the [`BootInfo`] from `Machine::getbootinfo` (boot(8)'s `bootarg` list on
//! amd64, efiboot's `/chosen` on arm64) and calls [`start_kernel`]: it sets `boothowto` and
//! the boot DUID, lets the machine do its earliest setup (OpenBSD's `init_x86_64` /
//! `initarm`: the message buffer and the console), prints the boot banner and hands over to
//! `kern::init_main::main`. Limine's entry (`limine.rs`, its requests and `_start`, the
//! `init` and ramdisk boot modules, the frame buffer response and `fdtfb.rs`'s copy of the
//! device tree with a `/chosen/framebuffer` node) went with the removal of Limine
//! (2026-11-26): init is `/sbin/init` on the root, the root is the boot disk's, and efiboot
//! writes the frame buffer node itself.

mod bootarg;

use bsd::kern::init_main::{self, BOOTHOWTO};
use bsd::kern::subr_prf::Str;
use bsd::kprintf;
use bsd::machine::{BootInfo, Cpu, Exit, ExitStatus, Machine, MachineInfo};

/// Why the boot glue gave up before a console existed: the machine's earliest setup failed.
/// It ends in a failure exit; under QEMU the exit status (`ExitStatus::Failure`) is the only
/// trace, so the message `early_init` returns is dropped here until a console-less channel
/// (semihosting) can carry it.
#[derive(Debug)]
enum BootError {
    EarlyInit,
}

/// What the entry does with its [`BootInfo`]: `boothowto`, boot(8)'s DUID, the machine's
/// earliest setup (the console), the boot banner, and `main`.
///
/// # Safety
///
/// Once, on the boot CPU, from the entry, with `boot` describing the image just loaded.
unsafe fn start_kernel(boot: BootInfo) -> ! {
    // SAFETY: forwarded from the entry.
    if let Err(_unprintable) = unsafe { boot_init(&boot) } {
        // No console yet: the failure exit status is the only trace.
        Machine::exit(ExitStatus::Failure)
    }
    kprintf!(
        "bsd: booted on {} by {} {}\n",
        Machine::MACHINE,
        Str(boot.bootloader_name.to_bytes()),
        Str(boot.bootloader_version.to_bytes())
    );
    kprintf!(
        "bsd: boot(8) bootarg protocol, {} regions, {} MiB usable\n",
        boot.memmap.len(),
        boot.memmap.usable_bytes() >> 20
    );
    if !boot.cmdline.is_empty() {
        kprintf!("bootargs: {}\n", Str(boot.cmdline.to_bytes()));
    }
    if let Some(mp) = boot.mp {
        kprintf!(
            "bsd: {} processors, boot processor hwid {:#x}\n",
            mp.ncpus,
            mp.bsp_hwid
        );
    }
    // The hal runtime's bridges, before main() and so before any user of a hal piece, on
    // the boot CPU with interrupts off: the boot contract of sys/hal/bridges.rs.
    bsd::kern::hal_bridges::hal_bridges_register();
    // The system call path (docs/ZERO_UNSAFE.md 0.1, decision 47): the ABI, linked into the
    // image, registers its handler before main() and so before the first system call.
    bsd::kern::abi_module::abi_boot();
    init_main::main()
}

/// Sets `boothowto` and the boot DUID and brings up the machine's console.
///
/// # Safety
///
/// As for [`start_kernel`].
unsafe fn boot_init(boot: &BootInfo) -> Result<(), BootError> {
    BOOTHOWTO.store(boot.boothowto(), core::sync::atomic::Ordering::Relaxed);
    // boot(8)'s BOOTARG_BOOTDUID (efiboot's `openbsd,bootduid`): `setroot` finds the boot
    // disk by this label DUID.
    if let Some(duid) = boot.bootduid() {
        bsd::sys::disk::set_bootduid(duid);
    }
    // boot(8) passes only its flags: a self-test word is the preset the image builder wrote
    // into this kernel file (`xtask efiboot-image --selftest`).
    #[cfg(feature = "qemu")]
    bsd::kern::selftest::preset_apply();
    // SAFETY: forwarded from the entry; `boot` describes the image just loaded.
    unsafe { Machine::early_init(boot) }.map_err(|_unprintable| BootError::EarlyInit)
}
/* </CODE> */
