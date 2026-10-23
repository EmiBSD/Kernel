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
//! names `bsd`: the `sysctlfn` prototype of the nodes still in `bsd`, and the byte view of
//! the structures they copy out.
//!
//! Upstream: sys/sys/sysctl.h @ 3ce1f3f79392
//! LZ: sys/sys/sysctl.rs@3c62ede68802
//!
//! ## Deviations
//! - The `sysctlfn` prototype is [`Sysctlfn`]: a name slice instead of `int *` plus `u_int`,
//!   user addresses as `usize` for `oldp`/`newp`, and `Result<(), Errno>`. The other
//!   prototypes are `kernel::sysctl`'s (`SysctlNode`) and their subsystems'.
//! - [`SysctlPlain`] is not OpenBSD's: it gives the structures `sysctl_rdstruct` and
//!   `sysctl_struct` copy their bytes, which C does with a `void *` and a size.
//!
//! ## Redesign
//! - N4.6b: the header (ids, names, `kinfo_proc`, `kinfo_file`, `kinfo_vmentry`,
//!   `sysctl_bounded_args`) is `hal::sys::sysctl`, shared with `kernel::sysctl`.
//! - [`SysctlPlain`] is a safe trait implemented for every `hal::user::AbiPod` type (a
//!   blanket impl), its byte views `hal::user::bytes_of` and `bytes_of_mut`: the `unsafe
//!   trait`, its two `unsafe` byte views and its 12 `unsafe impl`s (LZ) are gone, and the
//!   structures this side copies out (`struct timeval`, `struct clockinfo`, `struct
//!   diskstats`, `struct cpustats`, `struct mbstat`) are `AbiPod` through `hal::abi_pod!`
//!   here, which checks at compile time that their members tile them (the C's padding
//!   holes are members, `ds_pad0`).

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

/// A structure that `sysctl_rdstruct` and `sysctl_struct` copy as bytes (not OpenBSD's: C
/// passes a `void *` and a size): every `hal::user::AbiPod` type, whose bytes are all
/// initialised and any of whose bit patterns is a value.
pub trait SysctlPlain: hal::user::AbiPod {
    /// The structure's bytes, as `copyout` reads them.
    fn as_bytes(&self) -> &[u8] {
        hal::user::bytes_of(core::slice::from_ref(self))
    }

    /// The structure's bytes, as `copyin` writes them.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        hal::user::bytes_of_mut(core::slice::from_mut(self))
    }
}

impl<T: hal::user::AbiPod> SysctlPlain for T {}

// `struct timeval` and `struct clockinfo` are the kernel's (`kernel::clock::Timeval`,
// `kernel::time::Clockinfo`, N4.8a), `AbiPod` where they are defined.

// `struct diskstats`: the C's one padding hole a member (`ds_pad0`).
hal::abi_pod!(Diskstats {
    ds_name: [u8; crate::sys::disk::DS_DISKNAMELEN],
    ds_busy: i32,
    ds_pad0: u32,
    ds_rxfer: u64,
    ds_wxfer: u64,
    ds_seek: u64,
    ds_rbytes: u64,
    ds_wbytes: u64,
    ds_attachtime: Timeval,
    ds_timestamp: Timeval,
    ds_time: Timeval,
});

// `struct cpustats`: seven `uint64_t`.
hal::abi_pod!(Cpustats {
    cs_time: [u64; crate::sys::sched::CPUSTATES],
    cs_flags: u64,
});

// `struct mbstat`: `u_long`s only.
hal::abi_pod!(Mbstat {
    m_drops: u64,
    m_wait: u64,
    m_drain: u64,
    m_mtypes: [u64; crate::sys::mbuf::MT_NTYPES],
    m_defrag_alloc: u64,
    m_prepend_alloc: u64,
    m_pullup_alloc: u64,
    m_pullup_copy: u64,
    m_pulldown_alloc: u64,
    m_pulldown_copy: u64,
});

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
        assert_eq!(Diskstats::default().as_bytes().len(), 112);
    }

    #[test]
    fn as_bytes_mut_writes_the_structure() {
        let mut c = Clockinfo::default();
        c.as_bytes_mut()[..4].copy_from_slice(&100i32.to_ne_bytes());
        c.as_bytes_mut()[12..].copy_from_slice(&1024i32.to_ne_bytes());
        assert_eq!((c.hz, c.profhz), (100, 1024));
        let mut v = 0u64;
        v.as_bytes_mut().copy_from_slice(&7u64.to_ne_bytes());
        assert_eq!(v, 7);
    }
}
/* </TESTS> */
