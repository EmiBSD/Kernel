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
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfsmount.h	8.3 (Berkeley) 3/30/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfsmount.h>` in the kernel: the mount structure, one allocated on every NFS mount
//! and holding the NFS specific information for it, and `VFSTONFS`.
//!
//! Upstream: sys/nfs/nfsmount.h @ 3ce1f3f79392
//! LZ: sys/nfs/nfsmount.rs@e8249449d905
//!
//! The header's prototypes (`nfs_mount`, `nfs_fsinfo`, `nfs_init`) are the functions of
//! `nfs_vfsops` and `nfs_subs`.
//!
//! ## Deviations
//! - `nm_ntree` (the node tree, `RBT_HEAD(nfs_nodetree, nfsnode)`) is not a member: every
//!   mount's nodes are on `nfs_node`'s one node hash (`NfsTables::nhash`, keyed by the mount
//!   and the handle; that module's Deviations).
//!
//! ## Redesign
//! - LZ's `NfsMount` was the `malloc(9)`ed object `nfs_mount` hung from `mnt_data`, handed
//!   around as `&'static NfsMount` with `Cell` members (an `unsafe impl Sync`), found by
//!   `VFSTONFS`'s cast of the raw word (one `unsafe`, the tag checked first), its request
//!   queue an intrusive `TailqHead`, its socket a `&'static Socket`, its server address a
//!   parked mbuf. [`NfsMount`] is a counted `malloc(M_NFSMNT)` block ([`NfsMountRef`], an
//!   `MArc`) in the mount's typed slot, and [`vfstonfs`] one more handle out of the slot
//!   (`panic(9)` for a mount that holds none, LZ's check): the handle may be kept across the
//!   RPC's sleeps. The timeout holds a second one (`nm_rtimeout`'s argument, `nfs_vfsops`).
//! - The C keeps the structure under the kernel lock: the scalars are relaxed atomics written
//!   there, `nm_reqsq` a `hal::pinlist` list bound to it (walked under `klock::reader`,
//!   changed under `klock::witness`), `nm_so` a counted socket handle in a kernel-lock cell
//!   (cloned out for a send or a receive, so no witness is held across either).
//! - `nm_rtimeout` is a `struct timeout` in a block of its own (an `MBox`, pinned there), so
//!   `nfs_timer`, which reaches the mount through its erased handle, adds it again without a
//!   count; the C embeds it.
//! - `nm_nam`, the server's address, is the `sockaddr`'s bytes ([`NfsNam`]): written once at
//!   mount, read without a lock; a call that needs it as a name mbuf (`soconnect`, a send on
//!   an `NFSMNT_NOCONN` socket) makes one ([`NfsNam::mbuf`]). An `Mbuf` holds `Cell`s, so a
//!   chain shared by every thread of the mount would need an exclusive witness per read.
//! - `nm_verf` is one atomic word of the verifier's eight bytes, read and written whole.

use core::sync::atomic::{AtomicI32, AtomicU16, AtomicU64, Ordering};

use hal::malloc::{MArc, MBox};
use hal::once::OnceStatic;
use hal::pinlist::PinList;
use hal::rwlock::Guarded;

use super::nfs::{NFS_MAX_TIMER, RChain};
use super::nfsproto::NFSX_V3WRITEVERF;
use crate::assert::kpanic;
use crate::fs::mount::Mount;
use crate::fs::vnode::Vnode;
use crate::net::mbuf::{MLEN, MT_SONAME, MbufBox, m_get};
use crate::net::socket::SocketRef;
use crate::proc::ProcGlue;
use crate::timeout::Timeout;

/// `M_NFSMNT`: the NFS mount structures (`<sys/malloc.h>`'s 23, "NFS mount").
pub const M_NFSMNT: i32 = 23;

/// The longest server address a mount keeps (`sockargs`' `UCHAR_MAX` for a name).
pub const NFS_NAMMAX: usize = 255;

/// `nm_nam`: the server's address, the bytes of its `sockaddr` (`sa_len` of them).
#[derive(Clone, Copy)]
pub struct NfsNam {
    /// The length.
    len: u8,
    /// The `sockaddr`.
    b: [u8; NFS_NAMMAX],
}

impl NfsNam {
    /// The address `sa` (at most [`NFS_NAMMAX`] bytes; `None` past it).
    pub fn new(sa: &[u8]) -> Option<Self> {
        let len = u8::try_from(sa.len()).ok()?;
        let mut b = [0u8; NFS_NAMMAX];
        b[..sa.len()].copy_from_slice(sa);
        Some(Self { len, b })
    }

    /// The address's bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.b[..usize::from(self.len)]
    }

    /// `saddr->sa_family` (0 for an address too short to hold one).
    pub fn family(&self) -> u8 {
        if self.len < 2 { 0 } else { self.b[1] }
    }

    /// The address in a name mbuf (`MT_SONAME`), for a call that takes one; `None` when no
    /// mbuf is to be had (`M_DONTWAIT`) or the address would not fit an mbuf's data.
    pub fn mbuf(&self, how: i32) -> Option<MbufBox> {
        let n = usize::from(self.len);
        if n > MLEN {
            return None;
        }
        let mut m = m_get(how, MT_SONAME)?;
        m.m_len().set(u32::from(self.len));
        m.data_mut()?.copy_from_slice(self.bytes());
        Some(m)
    }
}

/// `struct nfsmount`: one allocated on every NFS mount, holding the NFS specific
/// information for it. Every member but the request queue's links is a word the C changes
/// under the kernel lock (the module's Redesign).
pub struct NfsMount<G: ProcGlue> {
    /// `nm_reqsq`: request queue for this mount (bound to the kernel lock).
    pub nm_reqsq: PinList<RChain<G>>,
    /// `nm_rtimeout`: timeout (scans/resends `nm_reqsq`), in a block of its own.
    pub nm_rtimeout: MBox<Timeout>,
    /// `nm_mountp`: VFS structure for this filesystem.
    pub nm_mountp: &'static Mount<G>,
    /// `nm_vnode`: vnode of root dir, set once by `mountnfs`.
    pub nm_vnode: OnceStatic<&'static Vnode<G>>,
    /// `nm_flag`: flags for soft/hard... (`NFSMNT_*`), the send and receive locks' bits
    /// among them; its address is their wait channel.
    pub nm_flag: AtomicI32,
    /// `nm_numgrps`: max. size of groupslist.
    pub nm_numgrps: AtomicI32,
    /// `nm_so`: RPC socket (the kernel lock).
    pub nm_so: Guarded<Option<SocketRef<G>>>,
    /// `nm_sotype`: type of socket.
    pub nm_sotype: AtomicI32,
    /// `nm_soproto`: and protocol.
    pub nm_soproto: AtomicI32,
    /// `nm_soflags`: `pr_flags` for socket protocol.
    pub nm_soflags: AtomicI32,
    /// `nm_nam`: addr of server.
    pub nm_nam: NfsNam,
    /// `nm_timeo`: init timer for `NFSMNT_DUMBTIMR`.
    pub nm_timeo: AtomicI32,
    /// `nm_retry`: max retries.
    pub nm_retry: AtomicI32,
    /// `nm_srtt`: RTT timers for RPCs.
    pub nm_srtt: [AtomicI32; NFS_MAX_TIMER],
    /// `nm_sdrtt`.
    pub nm_sdrtt: [AtomicI32; NFS_MAX_TIMER],
    /// `nm_sent`: request send count.
    pub nm_sent: AtomicI32,
    /// `nm_cwnd`: request send window.
    pub nm_cwnd: AtomicI32,
    /// `nm_timeouts`: request timeouts.
    pub nm_timeouts: AtomicI32,
    /// `nm_rsize`: max size of read rpc.
    pub nm_rsize: AtomicI32,
    /// `nm_wsize`: max size of write rpc.
    pub nm_wsize: AtomicI32,
    /// `nm_readdirsize`: size of a readdir rpc.
    pub nm_readdirsize: AtomicI32,
    /// `nm_readahead`: num. of blocks to readahead.
    pub nm_readahead: AtomicI32,
    /// `nm_verf`: V3 write verifier, its eight bytes in one word.
    pub nm_verf: AtomicU64,
    /// `nm_acregmin`: attr cache file recently modified.
    pub nm_acregmin: AtomicU16,
    /// `nm_acregmax`: ac file not recently modified.
    pub nm_acregmax: AtomicU16,
    /// `nm_acdirmin`: ac for dir recently modified.
    pub nm_acdirmin: AtomicU16,
    /// `nm_acdirmax`: ac for dir not recently modified.
    pub nm_acdirmax: AtomicU16,
}

/// A counted handle to a mount's structure (the slot's, the timeout's, a caller's).
pub type NfsMountRef<G> = MArc<NfsMount<G>>;

impl<G: ProcGlue> NfsMount<G> {
    /// The structure of `mp` with the server address `nam` and the timeout block `to`, every
    /// other member clear (`malloc(.., M_ZERO)`): what `mountnfs` fills.
    pub fn new(mp: &'static Mount<G>, nam: NfsNam, to: MBox<Timeout>) -> Self {
        Self {
            nm_reqsq: PinList::new(),
            nm_rtimeout: to,
            nm_mountp: mp,
            nm_vnode: OnceStatic::new(),
            nm_flag: AtomicI32::new(0),
            nm_numgrps: AtomicI32::new(0),
            nm_so: Guarded::new_kernel(None),
            nm_sotype: AtomicI32::new(0),
            nm_soproto: AtomicI32::new(0),
            nm_soflags: AtomicI32::new(0),
            nm_nam: nam,
            nm_timeo: AtomicI32::new(0),
            nm_retry: AtomicI32::new(0),
            nm_srtt: [const { AtomicI32::new(0) }; NFS_MAX_TIMER],
            nm_sdrtt: [const { AtomicI32::new(0) }; NFS_MAX_TIMER],
            nm_sent: AtomicI32::new(0),
            nm_cwnd: AtomicI32::new(0),
            nm_timeouts: AtomicI32::new(0),
            nm_rsize: AtomicI32::new(0),
            nm_wsize: AtomicI32::new(0),
            nm_readdirsize: AtomicI32::new(0),
            nm_readahead: AtomicI32::new(0),
            nm_verf: AtomicU64::new(0),
            nm_acregmin: AtomicU16::new(0),
            nm_acregmax: AtomicU16::new(0),
            nm_acdirmin: AtomicU16::new(0),
            nm_acdirmax: AtomicU16::new(0),
        }
    }

    /// `nmp->nm_flag`.
    #[inline]
    pub fn flag(&self) -> i32 {
        self.nm_flag.load(Ordering::Relaxed)
    }

    /// `nmp->nm_verf`.
    pub fn verf(&self) -> [u8; NFSX_V3WRITEVERF] {
        self.nm_verf.load(Ordering::Relaxed).to_ne_bytes()
    }

    /// `bcopy(v, nmp->nm_verf, NFSX_V3WRITEVERF)`.
    pub fn set_verf(&self, v: [u8; NFSX_V3WRITEVERF]) {
        self.nm_verf.store(u64::from_ne_bytes(v), Ordering::Relaxed);
    }

    /// `nmp->nm_mountp->mnt_stat.f_mntfromname`, for the messages.
    pub fn mntfromname(&self) -> [u8; crate::sys::mount::MNAMELEN] {
        self.nm_mountp.stat().f_mntfromname
    }
}

/// `VFSTONFS(mp)`: convert mount ptr to nfsmount ptr, held: a counted handle out of the
/// mount's typed slot; `panic(9)` for a mount that holds no NFS structure (LZ's check of an
/// unmounted or a non-NFS mount, made by the slot's type).
pub fn vfstonfs<G: ProcGlue>(mp: &Mount<G>) -> NfsMountRef<G> {
    match mp.node_arc::<NfsMount<G>>() {
        Some(nmp) => nmp,
        None => kpanic(format_args!("VFSTONFS: mount {mp:p} is not a mounted NFS")),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the server address's copies and of the malloc type.

    use super::*;

    #[test]
    fn the_server_address_round_trips_through_a_name_mbuf() {
        let _g = crate::net::mbuf::tests::setup();
        let sa = [16u8, 2, 0x08, 0x01, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
        let nam = NfsNam::new(&sa).expect("an address");
        assert_eq!(nam.family(), 2);
        assert_eq!(nam.bytes(), &sa);
        let m = nam.mbuf(crate::net::mbuf::M_WAIT).expect("an mbuf");
        assert_eq!(m.data(), &sa);
        assert_eq!(i32::from(m.m_type().get()), MT_SONAME);
        assert!(NfsNam::new(&[0u8; 256]).is_none(), "past UCHAR_MAX");
        assert_eq!(NfsNam::new(&[1]).map(|n| n.family()), Some(0));
    }

    /// `M_NFSMNT` is the C's 23 ("NFS mount" in `vmstat -m`).
    #[test]
    #[ignore = "reads reference/openbsd-src (just test-ref)"]
    fn m_nfsmnt_is_the_c_s_type() {
        let defs = crate::reftest::defines("sys/sys/malloc.h");
        let _ = crate::reftest::assert_defines!(defs; M_NFSMNT);
    }
}
/* </TESTS> */
