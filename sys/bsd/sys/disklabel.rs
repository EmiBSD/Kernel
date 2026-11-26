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
 * Copyright (c) 1987, 1988, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	@(#)disklabel.h	8.2 (Berkeley) 7/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/disklabel.h>` as `bsd` names it: `sysdefs::dev::disklabel` (N7.1d) re-exported, and
//! `struct partinfo`, which stays here until N7.3c.
//!
//! Upstream: sys/sys/disklabel.h @ 3ce1f3f79392
//!
//! The header is `hal`'s since N7.1d (`sysdefs::dev::disklabel`, with its machine constants);
//! the functions it declares are the kernel's (`kernel::dev::disk`, `kernel::dev::disksubr`),
//! which `bsd`'s drivers reach through `sys/sys/disk.rs`.
//!
//! ## Deviations
//! - `struct partinfo` holds raw pointers, as in C: `DIOCGPART` hands the in-core label and
//!   partition to a file system (`ffs_mountfs`); [`Partinfo::store`] and [`Partinfo::load`]
//!   move it through an `ioctl` data buffer. N7.3c's block device replaces it.

pub use sysdefs::dev::disklabel::*;

/// `struct partinfo`: structure used internally to retrieve information about a partition
/// on a disk (`DIOCGPART`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Partinfo {
    /// `disklab`: the disk's in-core label.
    pub disklab: *mut Disklabel,
    /// `part`: the partition's entry in it.
    pub part: *mut Partition,
}

impl Partinfo {
    /// Writes the structure into an `ioctl` data buffer, as the driver's
    /// `((struct partinfo *)data)->disklab = ...` does. A buffer too short is left alone.
    pub fn store(&self, data: &mut [u8]) {
        if data.len() >= core::mem::size_of::<Self>() {
            // SAFETY: the buffer holds at least `size_of::<Partinfo>()` bytes; the write is
            // unaligned because an ioctl buffer has no alignment guarantee.
            unsafe { data.as_mut_ptr().cast::<Self>().write_unaligned(*self) };
        }
    }

    /// Reads the structure back out of an `ioctl` data buffer; `None` if it is too short.
    pub fn load(data: &[u8]) -> Option<Self> {
        if data.len() < core::mem::size_of::<Self>() {
            return None;
        }
        // SAFETY: as for `store`; any bit pattern is a pair of raw pointers.
        Some(unsafe { data.as_ptr().cast::<Self>().read_unaligned() })
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partinfo_travels_through_an_ioctl_buffer() {
        let mut d = Disklabel::zeroed();
        let pi = Partinfo {
            disklab: &mut d,
            part: core::ptr::null_mut(),
        };
        let mut buf = [0u8; 16];
        pi.store(&mut buf);
        let back = Partinfo::load(&buf).expect("long enough");
        assert_eq!(back.disklab, pi.disklab);
        assert!(back.part.is_null());
        assert!(Partinfo::load(&buf[..8]).is_none());
    }
}
/* </TESTS> */
