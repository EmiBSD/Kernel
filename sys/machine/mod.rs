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
//! The machine-dependent interface: OpenBSD `<machine/*.h>` and `cpufunc.h` as traits.
//!
//! Generic code reaches architecture code ONLY through this module. One module per OpenBSD header
//! ([`param`], [`vmparam`], [`cpu`], [`cons`], [`bus`], [`pmap`], [`intr`], [`db_machdep`],
//! [`disklabel`], [`fdt`], [`proc`], [`signal`], [`tcb`], [`pci_machdep`], [`isa_machdep`], [`atomic`];
//! [`acpi_machdep`] is the machine half of `<dev/acpi/acpivar.h>` (each arch's `acpi_machdep.c`)
//! and [`pciide_machdep`] that of `<dev/pci/pciidevar.h>` (each arch's `pciide_machdep.c`);
//! [`autoconf`] is what `ioconf.c` and the machine's `autoconf.c` give `subr_autoconf.c`;
//! [`conf`] is the device switch the machine's `conf.c` fills (`bdevsw[]`, `cdevsw[]`); [`bootinfo`]
//! is the record the boot glue hands over), all re-exported here. `pci_chipset` (cfg
//! `machine_pci_chipset`, arm64 only) is not a contract but the machine items the device-tree
//! PCI host bridges use directly, as their C does; it is reached by its path, as is `x86`
//! (cfg `machine_x86`, amd64 only), the x86 items `dev/acpi/acpidmar.c` and
//! `dev/acpi/acpicpu_x86.c` use directly. The selected
//! architecture is re-exported as [`Machine`]; the block at the
//! bottom proves at compile time that it implements every trait. Adding a trait method therefore
//! means implementing it for amd64, arm64 and the host test double in the same commit.

pub mod acpi_machdep;
pub mod atomic;
pub mod autoconf;
pub mod bootinfo;
pub mod bus;
pub mod conf;
pub mod cons;
pub mod copy;
pub mod cpu;
pub mod db_machdep;
pub mod disklabel;
pub use hal::machine::{exec, param, vmparam};
pub mod fdt;
pub mod intr;
pub mod isa_machdep;
pub mod mpconfig;
#[cfg(machine_pci_chipset)]
pub mod pci_chipset;
pub mod pci_machdep;
pub mod pciide_machdep;
pub mod pmap;
pub mod proc;
pub mod signal;
pub mod tcb;
#[cfg(machine_x86)]
pub mod x86;

pub use acpi_machdep::*;
pub use atomic::*;
pub use autoconf::*;
pub use bootinfo::*;
pub use bus::*;
pub use conf::*;
pub use cons::*;
pub use copy::*;
pub use cpu::*;
pub use db_machdep::*;
pub use disklabel::*;
pub use fdt::*;
pub use hal::machine::exec::*;
pub use hal::machine::param::*;
pub use hal::machine::vmparam::*;
pub use intr::*;
pub use isa_machdep::*;
pub use mpconfig::*;
pub use pci_machdep::*;
pub use pciide_machdep::*;
pub use pmap::*;
pub use proc::*;
pub use signal::*;
pub use tcb::*;

/// The selected architecture's implementation of the machine interface.
pub use crate::arch::current::Machine;

// Compile-time proof that the selected architecture implements the whole contract.
const _: () = {
    const fn assert_impl<
        M: MachineInfo
            + AcpiMachdep
            + Atomic
            + Autoconf
            + MachineParam
            + VmParam
            + Cpu
            + Console
            + Exit
            + Conf
            + BusSpace
            + BusDma
            + PciMachdep
            + PciideMachdep
            + DbMachdep
            + MachineDisklabel
            + Pmap
            + Intr
            + Fdt
            + IsaMachdep
            + MpConfig
            + MachineProc
            + UserCopy
            + MachineExec
            + MachineSignal
            + Tcb,
    >() {
    }
    assert_impl::<Machine>();
};

// `hal`'s copy of the machine's page geometry and address-space bounds
// (`hal::arch::current::{param, vmparam}`, N3.1: the pmap and the page arithmetic read them
// there) is the one this contract exports.
const _: () = {
    use hal::arch::current::{param as hp, vmparam as hv};
    assert!(<Machine as MachineParam>::PAGE_SHIFT == hp::PAGE_SHIFT);
    assert!(<Machine as MachineParam>::PAGE_SIZE == hp::PAGE_SIZE);
    assert!(<Machine as MachineParam>::PAGE_MASK == hp::PAGE_MASK);
    assert!(<Machine as MachineParam>::KERNBASE == hp::KERNBASE);
    // N4.7a: the message buffer's size, which kernel::log's static takes from hal.
    assert!(<Machine as MachineParam>::MSGBUFSIZE == hp::MSGBUFSIZE);
    assert!(<Machine as VmParam>::VM_MIN_ADDRESS == hv::VM_MIN_ADDRESS);
    assert!(<Machine as VmParam>::VM_MAXUSER_ADDRESS == hv::VM_MAXUSER_ADDRESS);
    assert!(<Machine as VmParam>::VM_MAX_ADDRESS == hv::VM_MAX_ADDRESS);
    assert!(<Machine as VmParam>::VM_MIN_KERNEL_ADDRESS == hv::VM_MIN_KERNEL_ADDRESS);
    assert!(<Machine as VmParam>::VM_MAX_KERNEL_ADDRESS == hv::VM_MAX_KERNEL_ADDRESS);
    // N3.5 S1: the user layout and the default limits, which the kernel's map reads there.
    assert!(<Machine as VmParam>::USRSTACK == hv::USRSTACK);
    assert!(<Machine as VmParam>::MAXTSIZ == hv::MAXTSIZ);
    assert!(<Machine as VmParam>::DFLDSIZ == hv::DFLDSIZ);
    assert!(<Machine as VmParam>::MAXDSIZ == hv::MAXDSIZ);
    assert!(<Machine as VmParam>::BRKSIZ == hv::BRKSIZ);
    assert!(<Machine as VmParam>::DFLSSIZ == hv::DFLSSIZ);
    assert!(<Machine as VmParam>::MAXSSIZ == hv::MAXSSIZ);
    assert!(<Machine as VmParam>::STACKGAP_RANDOM == hv::STACKGAP_RANDOM);
    assert!(<Machine as VmParam>::VM_MIN_STACK_ADDRESS == hv::VM_MIN_STACK_ADDRESS);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_machine_is_the_host_double_under_test() {
        assert_eq!(Machine::MACHINE, "host");
        assert_eq!(Machine::MACHINE_ARCH, "host");
    }

    #[test]
    fn host_page_geometry_is_consistent() {
        assert_eq!(Machine::PAGE_SIZE, 1 << Machine::PAGE_SHIFT);
        assert_eq!(Machine::PAGE_MASK, Machine::PAGE_SIZE - 1);
        assert_eq!(Machine::USPACE, Machine::UPAGES * Machine::PAGE_SIZE);
        assert!(Machine::MAX_PAGE_SHIFT >= Machine::PAGE_SHIFT);
    }

    #[test]
    fn qemu_exit_statuses_are_distinct_and_odd() {
        assert_ne!(
            ExitStatus::Success.qemu_status(),
            ExitStatus::Failure.qemu_status()
        );
        assert_eq!(ExitStatus::Success.qemu_status() % 2, 1);
        assert_eq!(ExitStatus::Failure.qemu_status() % 2, 1);
    }
}
/* </TESTS> */
