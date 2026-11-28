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
//! Devices (N7, docs/ROADMAP.md): the device tree, autoconfiguration and the driver
//! interface (`subr_autoconf.c`, `<sys/device.h>`, N7.1a); the interrupt handler and arm64's
//! interrupt core (N7.2a1); the drivers follow from N7.2 (the device tree's `dev/fdt`: psci(4) and
//! plrtc(4), N7.2b2a).

#[cfg(target_arch = "aarch64")]
pub mod arm64;
pub mod autoconf;
pub mod clock_subr;
pub mod device;
pub mod disk;
pub mod diskmap;
pub mod disksubr;
pub mod evcount;
#[cfg(target_arch = "aarch64")]
pub mod fdt;
pub mod intr;
pub mod ofw;
pub mod sensors;
#[cfg(feature = "boot_config")]
pub mod userconf;
pub mod watchdog;
/* </CODE> */
