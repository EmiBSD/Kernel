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
//! ACPI: OpenBSD `sys/dev/acpi/`. M13 brings the headers, the AML interpreter (`dsdt`), the
//! core (`acpi`, acpi0), the table checksum (`acpiutil`), the timers (`acpitimer`,
//! `acpihpet`), the MADT (`acpimadt`) and the PCI interrupt routing (`acpiprt`). M14 adds
//! the MCFG (`acpimcfg`) and arm64's console UART (`pluart_acpi`). M16e adds the DMA remapping
//! units (`acpidmar`, with `amd_iommu`) the TPM (`tpm`) and the x86 processor power management (`acpicpu_x86`).
//!
//! The timers are compiled for amd64 and the host only (N7, U14): OpenBSD configures
//! `acpitimer* at acpi?` and `acpihpet* at acpi?` in amd64's GENERIC (and i386's), not in
//! arm64's, whose autoconf table has no row for them; acpi0 still offers `acpitimer` by name
//! (`acpi.c`), and on arm64 nothing matches it, as in the C.

#[allow(clippy::module_inception)] // OpenBSD's layout: sys/dev/acpi/acpi.c
pub mod acpi;
pub mod acpicpu_x86;
#[forbid(unsafe_code)]
pub mod acpidev;
pub mod acpidmar;
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub mod acpihpet;
pub mod acpimadt;
pub mod acpimcfg;
pub mod acpiprt;
#[forbid(unsafe_code)]
pub mod acpireg;
#[cfg(not(all(target_os = "none", target_arch = "aarch64")))]
pub mod acpitimer;
#[forbid(unsafe_code)]
pub mod acpiutil;
pub mod acpivar;
#[forbid(unsafe_code)]
pub mod amd_iommu;
#[forbid(unsafe_code)]
pub mod amltypes;
pub mod dsdt;
pub mod ipmi_acpi;
pub mod pluart_acpi;
pub mod tpm;
/* </CODE> */
