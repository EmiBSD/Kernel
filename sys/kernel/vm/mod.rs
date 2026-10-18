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
//! The kernel's virtual memory (`uvm`), moving out of `bsd` piece by piece (N3,
//! docs/ZERO_UNSAFE.md 0.8): the shared types of `<uvm/uvm_extern.h>` and the kernel map's
//! allocator, `km_alloc(9)` (N3.2); the resident pages and the free page allocator,
//! `uvm_page` and `uvm_pmemrange` with the per-CPU page caches (N3.3); the anons and the
//! amaps, `uvm_anon` and `uvm_amap` (N3.4b); the memory objects and their pagers,
//! `uvm_object`, `uvm_aobj`, `uvm_pager` and `uvm_vnode` (N3.4c); the fault handler,
//! `uvm_fault` (N3.4d).

pub mod addr;
pub mod amap;
pub mod anon;
pub mod aobj;
pub mod fault;
pub mod glue;
pub mod init;
pub mod io;
pub mod km;
pub mod map;
pub mod meter;
pub mod mmap;
pub mod object;
pub mod page;
pub mod pager;
pub mod pdaemon;
pub mod pmemrange;
pub mod proc;
pub mod swap;
pub mod types;
pub mod unix;
pub mod vnode;
/* </CODE> */
