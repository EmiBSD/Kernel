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
 * Copyright (c) 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Mike Karels at Berkeley Software Design, Inc.
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
 *	@(#)sysctl.h	8.2 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! Definitions for the `sysctl(2)` call: `<sys/sysctl.h>`. The ids, the name tables and the
//! structures are `hal`'s header (`hal::sys::sysctl`), re-exported here; what stays is what
//! names `bsd`: the `sysctlfn` prototype and the byte view of the structures `bsd`'s nodes
//! copy out.
//!
//! Upstream: sys/sys/sysctl.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - The `sysctlfn` prototype is [`Sysctlfn`]: a name slice instead of `int *` plus `u_int`,
//!   user addresses as `usize` for `oldp`/`newp`, and `Result<(), Errno>`. The other
//!   prototypes are the functions in `kern/kern_sysctl.rs` and their subsystems.
//! - [`SysctlPlain`] is not OpenBSD's: it marks the structures `sysctl_rdstruct` and
//!   `sysctl_struct` may copy as bytes, which C does with a `void *` and a size.

use crate::sys::disk::Diskstats;
use crate::sys::errno::Errno;
use crate::sys::mbuf::Mbstat;
use crate::sys::proc::Proc;
use crate::sys::sched::Cpustats;
use crate::sys::time::{Clockinfo, Timeval};

pub use hal::sys::sysctl::*;

/// `sysctlfn`: the internal sysctl function calling convention,
/// `(*sysctlfn)(name, oldp, oldlenp, newp, newlen, p)`. `name` is the rest of the name from
/// the component to be interpreted on (the C's `name` and `namelen`); `oldp` and `newp` are
/// user addresses, 0 for the C's NULL.
pub type Sysctlfn = fn(&[i32], usize, &mut usize, usize, usize, &Proc) -> Result<(), Errno>;

/// A structure that `sysctl_rdstruct` and `sysctl_struct` may copy as raw bytes (not
/// OpenBSD's: C passes a `void *` and a size).
///
/// # Safety
///
/// The implementor is `#[repr(C)]` (or a primitive, or an array of such), has no padding
/// bytes, and every bit pattern is a valid value of it.
pub unsafe trait SysctlPlain: Sized {
    /// The structure's bytes, as `copyout` reads them.
    fn as_bytes(&self) -> &[u8] {
        // SAFETY: the trait's contract: no padding, so every byte is initialised, and the
        // slice borrows `self` for its lifetime.
        unsafe {
            core::slice::from_raw_parts(
                (self as *const Self).cast::<u8>(),
                core::mem::size_of::<Self>(),
            )
        }
    }

    /// The structure's bytes, as `copyin` writes them.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: the trait's contract: every bit pattern is a valid value, and the slice
        // borrows `self` exclusively for its lifetime.
        unsafe {
            core::slice::from_raw_parts_mut(
                (self as *mut Self).cast::<u8>(),
                core::mem::size_of::<Self>(),
            )
        }
    }
}

// SAFETY: primitives have no padding and every bit pattern is valid.
unsafe impl SysctlPlain for i32 {}
// SAFETY: as for `i32`.
unsafe impl SysctlPlain for u32 {}
// SAFETY: as for `i32`.
unsafe impl SysctlPlain for i64 {}
// SAFETY: as for `i32`.
unsafe impl SysctlPlain for u64 {}
// SAFETY: an array of padding-free, any-bit-pattern elements is laid out without gaps.
unsafe impl<T: SysctlPlain, const N: usize> SysctlPlain for [T; N] {}
// SAFETY: `#[repr(C)]`, two 64-bit words (`time_t`, `suseconds_t` is a `long`), no padding.
unsafe impl SysctlPlain for Timeval {}
// SAFETY: `#[repr(C)]`, four `i32`s, no padding.
unsafe impl SysctlPlain for Clockinfo {}
// SAFETY: `#[repr(C)]` integers and byte arrays, the one C padding hole made a member
// (`p_pad0`); the compile-time checks below pin the size and the offsets.
unsafe impl SysctlPlain for KinfoProc {}
// SAFETY: `#[repr(C)]` integers and byte arrays, the two C padding holes made members
// (`kf_pad0`, `kf_pad1`); the compile-time checks below pin the size and the offsets.
unsafe impl SysctlPlain for KinfoFile {}
// SAFETY: `#[repr(C)]` integers, a byte array and `Timeval`s, the one C padding hole made a
// member (`ds_pad0`); the compile-time checks below pin the size and the offsets.
unsafe impl SysctlPlain for Diskstats {}
// SAFETY: `#[repr(C)]`, seven `u64`s, no padding.
unsafe impl SysctlPlain for Cpustats {}
// SAFETY: `#[repr(C)]`, `u64`s only, no padding.
unsafe impl SysctlPlain for Mbstat {}

const _: () = {
    use core::mem::{offset_of, size_of};
    // The C layout (amd64 and arm64 are both LP64) of the structures this side copies out.
    assert!(size_of::<Diskstats>() == 112);
    assert!(offset_of!(Diskstats, ds_rxfer) == 24);
    assert!(offset_of!(Diskstats, ds_attachtime) == 64);
    assert!(size_of::<Timeval>() == 16);
    assert!(size_of::<Clockinfo>() == 16);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_byte_views_are_the_structures_sizes() {
        let kp = KinfoProc::zeroed();
        assert_eq!(kp.as_bytes().len(), 648);
        assert!(kp.as_bytes().iter().all(|&b| b == 0));
        let kf = KinfoFile::zeroed();
        assert_eq!(kf.as_bytes().len(), 632);
        assert_eq!(Cpustats::default().as_bytes().len(), 56);
    }
}
/* </TESTS> */
