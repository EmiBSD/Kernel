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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_io.c,v 1.1.2.2 1997/12/30 12:02:00 mrg Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_io.c`: uvm i/o ops, reading or writing another address space through `kernel_map`.
//!
//! Upstream: sys/uvm/uvm_io.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_io.rs@3c62ede68802
//!
//! [`uvm_io`] is what `kern.proc_args` (`sysctl_proc_args`) reads a victim's `ps_strings`
//! and argument strings with: each chunk of the victim's map is extracted into `kernel_map`
//! (`uvm_map_extract`), copied (faulting the pages in as the kernel touches them), and
//! unmapped again.
//!
//! ## Deviations
//! - `uvm_unmap_remove` cannot fail on `kernel_map` (no immutable entries are checked), so
//!   its result is not consulted, as the C ignores it (LZ's).
//! - A write (`UIO_WRITE`, `UVM_IO_FIXPROT`: ptrace's `PT_WRITE`/`PT_IO`, not ported yet) moves
//!   the chunk out of the uio into the bounce before the copy into the window, so a fault in
//!   the window leaves the uio advanced by the chunk where the C's `uiomove` straight into the
//!   window advances it by none of it. No caller sees it today: the one caller,
//!   `kern.proc_args` (`sysctl_proc_args`), reads, and throws the uio away on any error. To be
//!   fixed when `PT_IO` is ported (the window copied from the uio's own iovecs).
//!
//! ## Redesign
//! - The extracted chunk is a [`KernelWindow`](super::km::KernelWindow): minted only from
//!   the range `uvm_map_extract` returned ([`super::km::uvm_map_extract_window`]), its drop
//!   is the C's step 4 (`uvm_unmap_remove` of the range from `kernel_map`, then
//!   `uvm_unmap_detach(AMAP_REFALL)`). LZ's `unsafe` slice over the window is gone: the
//!   window's pages belong to another address space (another thread may write them), so no
//!   `&mut [u8]` is made of them. The bytes go through a bounce ([`KmBuf`]) as large as the
//!   first chunk (at most `MAXBSIZE`; chunks only shrink): `kcopy` between the window and the
//!   bounce (`hal::user::ForeignWindow`, the machine's copy with its fault recovery), then one
//!   `uiomove` of the chunk between the bounce and the uio, so a read advances the uio by the
//!   whole chunk or, on a fault, by none of it, as the C's one `uiomove` per chunk does (a
//!   write: see the deviations).
//! - The `struct uio *` is the kernel's `UioMove` (its direction is `is_write`), the
//!   address in the map (`uio_offset`) a parameter.

use hal::arch::current::param::{PAGE_MASK, PAGE_SIZE};
use hal::arch::current::vmparam::VM_MAXUSER_ADDRESS;
use hal::kmbuf::KmBuf;
use hal::sys::errno::Errno;
use hal::user::ForeignWindow;
use hal::vm::param::{round_page, trunc_page};

use super::km::{page_memory, uvm_map_extract_window};
use super::map::{UVM_EXTRACT_FIXPROT, VmMap};
use super::types::UVM_IO_FIXPROT;
use crate::uio::UioMove;

/// `MAXBSIZE` (`<sys/param.h>`): the largest chunk `uvm_io` extracts at once.
pub const MAXBSIZE: usize = 64 * 1024;

/// The start of a transfer (`io_range`): its first page, the bytes to move, the offset into
/// the first page.
struct IoRange {
    baseva: usize,
    togo: usize,
    pageoffset: usize,
}

/// `uvm_io`: perform I/O on a map. `offset` is the address in `map` (`uio_offset`), the uio's
/// residual count the length; the transfer stops at `VM_MAXUSER_ADDRESS` (an EOF
/// truncation). `flags` may hold `UVM_IO_FIXPROT` (extract with the maximum protection, for
/// a write).
///
/// The caller must hold a reference to `map` so that it does not go away meanwhile.
pub fn uvm_io(map: &VmMap, uio: &mut dyn UioMove, offset: usize, flags: i32) -> Result<(), Errno> {
    // step 0: sanity checks and set up for copy loop. start with a large chunk size. if we
    // have trouble finding vm space we will reduce it.
    let resid = uio.resid();
    if resid == 0 {
        return Ok(());
    }
    let Some(range) = io_range(offset, resid) else {
        return Err(Errno::EIO);
    };
    let IoRange {
        mut baseva,
        mut togo,
        mut pageoffset,
    } = range;
    let mut chunksz = round_page(togo + pageoffset).min(MAXBSIZE);

    let extractflags = if flags & UVM_IO_FIXPROT != 0 {
        UVM_EXTRACT_FIXPROT
    } else {
        0
    };

    let Some(mut bounce) = KmBuf::alloc_wired(page_memory(), chunksz, true) else {
        return Err(Errno::ENOMEM);
    };

    // step 1: main loop... while we've got data to move
    while togo > 0 {
        // step 2: extract mappings from the map into kernel_map
        let window = match uvm_map_extract_window(map, baseva, chunksz, extractflags) {
            Ok(w) => w,
            Err(Errno::ENOMEM) if chunksz > PAGE_SIZE => {
                // retry with a smaller chunk... (the C's `continue` also clears pageoffset)
                chunksz = smaller_chunk(chunksz);
                pageoffset = 0;
                continue;
            }
            Err(e) => return Err(e),
        };

        // step 3: move a chunk of data
        let sz = (chunksz - pageoffset).min(togo);
        let moved = io_move(window.window(), pageoffset, sz, uio, &mut bounce);
        togo -= sz;
        baseva += chunksz;

        // step 4: unmap the area of kernel memory
        drop(window);

        moved?;
        pageoffset = 0;
    }

    Ok(())
}

/// Where `uvm_io`'s transfer of `resid` bytes at `offset` starts: the page, the bytes it may
/// move (truncated at `VM_MAXUSER_ADDRESS`) and the offset into the first page; `None` (the
/// C's `EIO`) when the range wraps or starts at or above `VM_MAXUSER_ADDRESS`.
fn io_range(offset: usize, resid: usize) -> Option<IoRange> {
    let mut togo = resid;
    let baseva = offset;
    let endva = baseva.checked_add(togo - 1)?; // wrap around?
    if baseva >= VM_MAXUSER_ADDRESS {
        return None;
    }
    if endva >= VM_MAXUSER_ADDRESS {
        togo -= endva - VM_MAXUSER_ADDRESS + 1; // EOF truncate
    }
    Some(IoRange {
        baseva: trunc_page(baseva),
        togo,
        pageoffset: baseva & PAGE_MASK,
    })
}

/// The chunk `uvm_io` retries with when `kernel_map` has no room for `chunksz`: half of it,
/// truncated to a page, at least one page.
fn smaller_chunk(chunksz: usize) -> usize {
    trunc_page(chunksz / 2).max(PAGE_SIZE)
}

/// Step 3 of `uvm_io`: `sz` bytes between the window at `off` and the uio, through `bounce`
/// (`uvm_io`'s holds the whole chunk: one piece), in the uio's direction (a read: window to
/// bounce to uio; a write: uio to bounce to window).
fn io_move(
    window: &ForeignWindow<'_>,
    off: usize,
    sz: usize,
    uio: &mut dyn UioMove,
    bounce: &mut [u8],
) -> Result<(), Errno> {
    let mut done = 0;
    while done < sz {
        let n = (sz - done).min(bounce.len());
        let piece = &mut bounce[..n];
        if uio.is_write() {
            uio.uiomove(piece)?;
            window.write_at(off + done, piece)?;
        } else {
            window.read_at(off + done, piece)?;
            uio.uiomove(piece)?;
        }
        done += n;
    }
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::testutil;

    /// A uio over a buffer: a read's destination or a write's source.
    struct Buf {
        data: Vec<u8>,
        pos: usize,
        write: bool,
    }

    impl UioMove for Buf {
        fn resid(&self) -> usize {
            self.data.len() - self.pos
        }

        fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
            let n = buf.len().min(self.resid());
            let user = &mut self.data[self.pos..self.pos + n];
            if self.write {
                buf[..n].copy_from_slice(user);
            } else {
                user.copy_from_slice(&buf[..n]);
            }
            self.pos += n;
            Ok(())
        }

        fn nonblock(&self) -> bool {
            false
        }

        fn is_write(&self) -> bool {
            self.write
        }

        fn set_resid(&mut self, resid: usize) {
            self.pos = self.data.len().saturating_sub(resid);
        }
    }

    fn window() -> ForeignWindow<'static> {
        testutil::install();
        let _ = testutil::pmap_boot();
        super::super::km::test_window(
            hal::sys::types::Vaddr::new(testutil::FOREIGN_BASE),
            testutil::FOREIGN_LEN,
        )
    }

    #[test]
    fn the_range_is_the_cs() {
        assert!(io_range(VM_MAXUSER_ADDRESS, 1).is_none(), "EIO at the top");
        assert!(io_range(usize::MAX, 2).is_none(), "EIO on a wrap");
        let r = io_range(0x1234, 0x10).unwrap();
        assert_eq!((r.baseva, r.togo, r.pageoffset), (0x1000, 0x10, 0x234));
        // EOF truncate at VM_MAXUSER_ADDRESS.
        let r = io_range(VM_MAXUSER_ADDRESS - 3, 10).unwrap();
        assert_eq!(r.togo, 3);
        assert_eq!(r.pageoffset, (VM_MAXUSER_ADDRESS - 3) & PAGE_MASK);
        assert_eq!(round_page(0x10 + 0x234).min(MAXBSIZE), PAGE_SIZE);
        assert_eq!(round_page(1 << 20).min(MAXBSIZE), MAXBSIZE);
    }

    #[test]
    fn enomem_halves_the_chunk_down_to_a_page() {
        let mut c = MAXBSIZE;
        let mut seen = vec![c];
        while c > PAGE_SIZE {
            c = smaller_chunk(c);
            seen.push(c);
        }
        assert_eq!(seen, [65536, 32768, 16384, 8192, 4096]);
        assert_eq!(smaller_chunk(3 * PAGE_SIZE), PAGE_SIZE);
    }

    #[test]
    fn a_read_moves_the_window_into_the_uio_through_the_bounce() {
        let w = window();
        let off = 3 * PAGE_SIZE + 100;
        let src: Vec<u8> = (0..2 * PAGE_SIZE + 7).map(|i| (i * 7) as u8).collect();
        assert!(testutil::set_foreign_bytes(
            testutil::FOREIGN_BASE + off,
            &src
        ));
        let mut uio = Buf {
            data: vec![0; src.len()],
            pos: 0,
            write: false,
        };
        let mut bounce = vec![0u8; PAGE_SIZE];
        io_move(&w, off, src.len(), &mut uio, &mut bounce).unwrap();
        assert_eq!(uio.data, src);
        assert_eq!(uio.resid(), 0);
    }

    #[test]
    fn a_write_moves_the_uio_into_the_window() {
        let w = window();
        let off = 9 * PAGE_SIZE + 4000;
        let src: Vec<u8> = (0..PAGE_SIZE + 300).map(|i| (i * 13 + 1) as u8).collect();
        let mut uio = Buf {
            data: src.clone(),
            pos: 0,
            write: true,
        };
        let mut bounce = vec![0u8; PAGE_SIZE];
        io_move(&w, off, src.len(), &mut uio, &mut bounce).unwrap();
        assert_eq!(
            testutil::foreign_bytes(testutil::FOREIGN_BASE + off, src.len()).unwrap(),
            src
        );
    }

    #[test]
    fn a_fault_in_the_window_is_efault() {
        testutil::install();
        let _ = testutil::pmap_boot();
        // A window past the double's foreign memory: every copy faults.
        let w = super::super::km::test_window(
            hal::sys::types::Vaddr::new(testutil::FOREIGN_BASE + testutil::FOREIGN_LEN),
            PAGE_SIZE,
        );
        let mut uio = Buf {
            data: vec![0; 16],
            pos: 0,
            write: false,
        };
        let mut bounce = vec![0u8; PAGE_SIZE];
        assert_eq!(
            io_move(&w, 0, 16, &mut uio, &mut bounce),
            Err(Errno::EFAULT)
        );
        assert_eq!(uio.resid(), 16, "nothing reached the uio");
    }

    /// A read whose chunk faults past its first page leaves the uio where it was, as the C's
    /// one `uiomove` per chunk: the bounce holds the whole chunk.
    #[test]
    fn a_read_fault_mid_chunk_moves_nothing() {
        testutil::install();
        let _ = testutil::pmap_boot();
        let base = testutil::FOREIGN_BASE + testutil::FOREIGN_LEN - PAGE_SIZE;
        let w = super::super::km::test_window(hal::sys::types::Vaddr::new(base), 2 * PAGE_SIZE);
        let mut uio = Buf {
            data: vec![0; 2 * PAGE_SIZE],
            pos: 0,
            write: false,
        };
        let mut bounce = vec![0u8; 2 * PAGE_SIZE];
        assert_eq!(
            io_move(&w, 0, 2 * PAGE_SIZE, &mut uio, &mut bounce),
            Err(Errno::EFAULT)
        );
        assert_eq!(uio.resid(), 2 * PAGE_SIZE, "the uio did not move");
    }
}
/* </TESTS> */
