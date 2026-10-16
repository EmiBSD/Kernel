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
//! Header ports: OpenBSD `sys/arch/arm64/include/*.h`.
//!
//! Constants, `#[repr(C)]` hardware structs and inline accessors only; never state.

pub use hal::arch::arm64::include::_types;
pub mod armreg;
pub mod bootconfig;
pub mod bus;
pub mod cpu;
pub mod db_machdep;
pub use hal::arch::arm64::include::disklabel;
pub mod efivar;
pub use hal::arch::arm64::include::elf;
pub mod exec;
pub mod fdt;
pub mod frame;
pub mod hypervisor;
pub mod intr;
pub use hal::arch::arm64::include::mplock;
pub use hal::arch::arm64::include::mutex;
pub mod param;
pub mod pcb;
pub mod pci_machdep;
pub mod pmap;
pub use hal::arch::arm64::include::proc;
pub mod pte;
pub use hal::arch::arm64::include::reg;
pub mod signal;
pub mod simplebusvar;
pub mod tcb;
pub use hal::arch::arm64::include::timetc;
pub mod vmparam;
/* </CODE> */
