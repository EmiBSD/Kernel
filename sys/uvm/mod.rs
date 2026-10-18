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
//! Virtual memory: OpenBSD `sys/uvm/*.c`.
//!
//! Physical page management, kernel memory, maps, faults, the pager interface. Headers become
//! modules too (`uvm_page.h` → `uvm_page.rs`, the same file as `uvm_page.c`).

#[allow(clippy::module_inception)] // OpenBSD's name: uvm/uvm.h
pub mod uvm;
pub mod uvm_addr;
#[forbid(unsafe_code)]
pub mod uvm_amap;
#[forbid(unsafe_code)]
pub mod uvm_anon;
// The anonymous objects, the objects and the pager interface are the kernel's since N3.4e
// (`uvm_aobj.c`, `uvm_object.c`, `uvm_pager.c`); the legacy paths stay.
pub use kernel::vm::aobj as uvm_aobj;
pub mod uvm_extern;
#[forbid(unsafe_code)]
pub mod uvm_fault;
pub mod uvm_glue;
pub mod uvm_init;
pub mod uvm_io;
#[forbid(unsafe_code)]
pub mod uvm_km;
pub mod uvm_map;
pub mod uvm_meter;
pub mod uvm_mmap;
pub use kernel::vm::object as uvm_object;
#[forbid(unsafe_code)]
pub mod uvm_page;
pub use kernel::vm::pager as uvm_pager;
#[forbid(unsafe_code)]
pub mod uvm_param;
#[forbid(unsafe_code)]
pub mod uvm_pdaemon;

#[forbid(unsafe_code)]
pub mod uvm_pmap;
// The free page allocator and the per-CPU page caches are the kernel's since N3.3
// (`uvm_pmemrange.c`, `uvm_percpu.h`); the legacy path stays.
pub use kernel::vm::pmemrange as uvm_pmemrange;
#[forbid(unsafe_code)]
pub mod uvm_swap;
#[forbid(unsafe_code)]
pub mod uvm_unix;
#[forbid(unsafe_code)]
pub mod uvm_vnode;
// `<uvm/uvmexp.h>` and its static live in `hal` since N3.3 (the kernel's page system writes
// the counters); the legacy path stays.
pub use hal::vm::uvmexp;
/* </CODE> */
