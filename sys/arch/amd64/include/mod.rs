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
//! Header ports: OpenBSD `sys/arch/amd64/include/*.h`.
//!
//! Constants, `#[repr(C)]` hardware structs and inline accessors only; never state.

pub use hal::arch::amd64::include::_types;
pub mod apicvar;
pub mod biosvar;
pub mod bus;
pub mod cpu;
pub mod cpu_full;
pub mod cpufunc;
pub mod cpuvar;
pub mod db_machdep;
pub use hal::arch::amd64::include::disklabel;
pub use hal::arch::amd64::include::efifbvar;
pub mod exec;
pub mod fpu;
pub use hal::arch::amd64::include::frame;
pub mod i82093reg;
pub mod i82093var;
pub mod i82489reg;
pub use hal::arch::amd64::include::i8259;
pub use hal::arch::amd64::include::i82489var;
pub mod intr;
pub mod intrdefs;
pub mod ioctl_fd;
pub mod isa_machdep;
pub mod mpbiosreg;
pub mod mpbiosvar;
pub mod mpconfig;
pub use hal::arch::amd64::include::mplock;
pub use hal::arch::amd64::include::mutex;
pub mod param;
pub mod pcb;
pub use hal::arch::amd64::include::pci_machdep;
pub mod pic;
pub mod pio;
pub mod pmap;
pub use hal::arch::amd64::include::proc;
pub mod psl;
pub use hal::arch::amd64::include::pte;
pub mod segments;
pub mod signal;
pub mod smbiosvar;
pub mod specialreg;
pub use hal::arch::amd64::include::tcb;
pub use hal::arch::amd64::include::timetc;
pub mod trap;
pub use hal::arch::amd64::include::tss;
pub mod vmparam;
/* </CODE> */
