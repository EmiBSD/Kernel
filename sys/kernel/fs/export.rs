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
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)vfs_subr.c	8.13 (Berkeley) 4/18/94
 */
/*
 * Copyright (c) 1989, 1991, 1993
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
 *	@(#)mount.h	8.15 (Berkeley) 7/14/94
 */
/*
 * Copyright (c) 1991, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)ufs_vfsops.c	8.4 (Berkeley) 4/16/94
 */
/* </LICENSES> */

/* <CODE> */
//! The NFS server's export lists (`NFSSERVER`): `struct netcred` and `struct netexport` of
//! `<sys/mount.h>`, `vfs_hang_addrlist`, `vfs_free_addrlist`, `vfs_export` and
//! `vfs_export_lookup` of `kern/vfs_subr.c`, and the body the file systems' `vfs_checkexp`
//! share (`ufs_check_export` of `ufs/ufs/ufs_vfsops.c`, which msdosfs's and cd9660's copy):
//! [`check`]. A mount's list is [`Mount::export`].
//!
//! Upstream: sys/kern/vfs_subr.c @ 3ce1f3f79392
//! Upstream: sys/sys/mount.h @ 3ce1f3f79392
//! Upstream: sys/ufs/ufs/ufs_vfsops.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_subr.rs@3c62ede68802
//! LZ: sys/sys/mount.rs@3c62ede68802
//! LZ: sys/ufs/ufs/ufs_vfsops.rs@3c62ede68802
//!
//! mountd(8) hands each exported mount's list to mount(2) (`MNT_UPDATE` with `struct
//! export_args`): `MNT_DELEXPORT` empties it, `MNT_EXPORTED` adds an entry, the default one
//! when no address is given. The NFS server asks [`check`] (through the file system's
//! `Vfs::checkexp`) what a client may do: the entry its address falls under, else the
//! default, as an [`ExportGrant`]. Both run under the kernel lock (`sys_mount` and the nfsd's
//! call are `STD`; `nfsrv_rcv` takes it): the list is a cell of it, changed under
//! `klock::witness()` and read under `klock::reader()`, neither held across a sleep.
//!
//! ## Deviations
//! - The answer is by value (D-6): [`vfs_export_lookup`] and [`check`] copy the entry's
//!   export flags and anonymous ids into an [`ExportGrant`] under the reader, where the C's
//!   `vfs_checkexp` hands out `&np->netc_anon`, which `nfsrv_fhtovp` reads after
//!   `VFS_FHTOVP`'s sleeps: a `MNT_DELEXPORT` from mountd in that window frees the entry
//!   under it. The C's checks, their order and their errnos are kept; only that race answers
//!   differently (the entry as it was when it matched, where the C reads freed memory).
//! - An address shorter than 2 bytes (no `sa_family`) is `EINVAL` (LZ's); the C reads the
//!   family byte past it. An address or mask shorter than its length byte, or a client's
//!   address mbuf shorter than its `sa_len`, reads zeros past its bytes, and a length byte
//!   above 16 reads as 16 (the radix tree's D-7); a client's address mbuf of fewer than 2
//!   bytes has no family, so only the default export answers it.
//! - A failed `malloc(M_WAITOK)` is `panic(9)` (LZ's; it cannot fail once `malloc` is
//!   registered). The C's `ENOBUFS` for a failed `rn_inithead` has no path left (the tree is
//!   in the list); a tree that cannot take the entry for want of memory (`M_NOWAIT` chunks)
//!   answers `EPERM`, the C's answer for `rn_addroute`'s `NULL`.
//! - The list is the mount's ([`Mount::export`]) rather than the file system's structure's
//!   (`um_export`, `pm_export`, `im_export`); the UFS-like mounts empty it where their
//!   `mountfs` `malloc(M_ZERO)`s that structure ([`Netexport::clear`]), so a recycled mount's
//!   old entries are freed there, where the C's `free(ump)` leaks them (D-12).
//! - Without `NFSSERVER` (feature `nfsserver`) [`vfs_export`] answers `ENOTSUP` and
//!   [`vfs_export_lookup`] nothing, as the C.
//!
//! ## Redesign
//! - LZ's `struct netcred` (`#[repr(C)]` with the radix node pair first, `malloc`ed with the
//!   address, the mask and 64 bytes of slack behind it, the leaf cast back to the entry,
//!   `netc_len`, an `unsafe impl Sync`) -> [`Netcred`], the value an `MBox` of `M_NETADDR` the
//!   kernel's radix tree (`kernel::net::radix`) carries: the tree owns the keys, no cast.
//! - LZ's `struct netexport` (`ne_rtable_inet` a `Cell<Option<&'static RadixNodeHead>>`, an
//!   `unsafe impl Sync`) in `bsd`'s glue member of the mount (N5.5b0) ->
//!   [`Netexport`], a kernel-lock cell holding the default entry and the tree, the mount's own
//!   member ([`Mount::export`]); ntfs keeps one in its structure as the C.
//! - `vfs_checkexp`'s anonymous credentials, a pointer into the entry, -> [`ExportGrant`], the
//!   ids by value (D-6; the check the C makes through the pointer is unchanged in `nfsrv_fhtovp`:
//!   uid 0 or `MNT_EXPORTANON` take the grant's ids, at most `NGROUPS_MAX` groups).
//! - `vfs_free_netcred`, the walker that deleted and freed one entry, is the tree's drop:
//!   `vfs_free_addrlist` puts an empty tree in the list's place.
//! - `ufs_check_export`, `msdosfs_check_export` and `cd9660_check_export` (one body over
//!   each file system's list) are [`check`] over the mount's list.
//! - Costs against the C: a check is one `klock::reader()` (a per-CPU count pair, no spl), the
//!   tree's `rn_match` (two loads a node), one box load and the grant's 80 bytes copied; an
//!   update one `klock::witness()` section (an spl pair the C lacks) after its sleeps.

use hal::klock;
#[cfg(feature = "nfsserver")]
use hal::malloc::M_WAITOK;
use hal::malloc::MBox;
use hal::rwlock::Guarded;
use hal::sys::errno::Errno;
use hal::sys::mount::ExportArgs;
#[cfg(feature = "nfsserver")]
use hal::sys::mount::{MNT_DEFEXPORTED, MNT_DELEXPORT, MNT_EXPORTED};
#[cfg(feature = "nfsserver")]
use hal::sys::socket::AF_INET;
use hal::sys::types::{Gid, Uid};
#[cfg(feature = "nfsserver")]
use hal::user::UserPtr;

use super::mount::Mount;
#[cfg(feature = "nfsserver")]
use crate::assert::kpanic;
#[cfg(feature = "nfsserver")]
use crate::cred::crfromxucred;
use crate::cred::{NGROUPS_MAX, Ucred};
#[cfg(feature = "nfsserver")]
use crate::net::mbuf::MLEN;
use crate::net::mbuf::Mbuf;
use crate::net::radix::RadixTree;
use crate::proc::ProcGlue;

/// `M_NETADDR`: the `malloc(9)` type of an export entry ("Export Host", `<sys/malloc.h>`).
pub const M_NETADDR: i32 = 49;
/// `M_RTABLE`: the `malloc(9)` type of the export tree's chunks (the C's radix head's).
const M_RTABLE: i32 = 5;
/// `rn_init(sizeof(struct sockaddr_in))` (`vntblinit`): the export tree's key length.
const SIN_LEN: usize = 16;
/// `offsetof(struct sockaddr_in, sin_addr)`: where the export tree's keys are compared from.
const SIN_ADDR_OFF: usize = 4;

/// `struct netcred`: the rights of the clients an export entry names (the C's node pair and
/// `netc_len` are the tree's).
pub struct Netcred {
    /// `netc_exflags`: the `export_args`'s `ex_flags` (`MNT_EXRDONLY`, `MNT_EXPORTANON`, ...).
    pub exflags: i32,
    /// `netc_anon`: the credentials anonymous clients map to.
    pub anon: Ucred,
}

impl Netcred {
    /// A zeroed entry, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            exflags: 0,
            anon: Ucred::new(),
        }
    }

    /// The entry's rights by value.
    #[cfg(feature = "nfsserver")]
    fn grant(&self) -> ExportGrant {
        let a = &self.anon;
        let mut groups = [0; NGROUPS_MAX];
        for (g, c) in groups.iter_mut().zip(&a.cr_groups) {
            *g = c.get();
        }
        ExportGrant {
            exflags: self.exflags,
            uid: a.cr_uid.get(),
            gid: a.cr_gid.get(),
            ngroups: a.cr_ngroups.get(),
            groups,
        }
    }
}

impl Default for Netcred {
    fn default() -> Self {
        Self::new()
    }
}

/// What `vfs_checkexp` answers a client (the C's export flags and anonymous ids), copied out of
/// its export entry under the kernel lock's reader (D-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportGrant {
    /// `netc_exflags`.
    pub exflags: i32,
    /// `netc_anon.cr_uid`.
    pub uid: Uid,
    /// `netc_anon.cr_gid`.
    pub gid: Gid,
    /// `netc_anon.cr_ngroups` as the entry holds it.
    pub ngroups: i16,
    /// `netc_anon.cr_groups`.
    pub groups: [Gid; NGROUPS_MAX],
}

/// The members of `struct netexport`, under the kernel lock.
#[cfg_attr(
    not(feature = "nfsserver"),
    allow(dead_code, reason = "read by NFSSERVER's code alone")
)]
struct ExportList {
    /// `ne_defexported`: the default export.
    defexported: Netcred,
    /// `ne_rtable_inet`: the individual exports, keyed by `struct sockaddr_in`.
    rtable_inet: RadixTree<MBox<Netcred>, SIN_LEN>,
}

impl ExportList {
    /// An empty list.
    const fn new() -> Self {
        Self {
            defexported: Netcred::new(),
            rtable_inet: RadixTree::new(SIN_ADDR_OFF, M_RTABLE),
        }
    }
}

/// `struct netexport`: the network export information of one mount ([`Mount::export`]) or
/// of ntfs's structure (`ntm_export`).
pub struct Netexport {
    /// The list, a cell of the kernel lock.
    k: Guarded<ExportList>,
}

impl Netexport {
    /// An empty export list, as `malloc(M_ZERO)` returns it.
    pub const fn new() -> Self {
        Self {
            k: Guarded::new_kernel(ExportList::new()),
        }
    }

    /// The list emptied, its entries freed, as a UFS-like `mountfs` starts its structure's
    /// (`malloc(M_ZERO)` of the `ufsmount`, `msdosfsmount`, `iso_mnt`): a recycled mount
    /// keeps none of its last mount's entries. The caller holds the kernel lock.
    pub fn clear(&self) {
        let mut w = klock::witness();
        *self.k.get_mut(&mut w) = ExportList::new();
    }
}

impl Default for Netexport {
    fn default() -> Self {
        Self::new()
    }
}

/// `vfs_hang_addrlist(mp, nep, argp)`: build the export entry `argp` names and hang it on the
/// mount's list (no address: the default export). Every sleep (the entry's `malloc`, the
/// copies in) comes before the list is changed.
#[cfg(feature = "nfsserver")]
fn vfs_hang_addrlist<G: ProcGlue>(
    mp: &Mount<G>,
    nep: &Netexport,
    argp: &ExportArgs,
) -> Result<(), Errno> {
    let x = &argp.ex_anon;
    if argp.ex_addrlen == 0 {
        if mp.flag() & MNT_DEFEXPORTED != 0 {
            return Err(Errno::EPERM);
        }
        let mut w = klock::witness();
        let np = &mut nep.k.get_mut(&mut w).defexported;
        // fill in the kernel's ucred from userspace's xucred
        crfromxucred(&mut np.anon, x.cr_uid, x.cr_gid, x.cr_ngroups, &x.cr_groups)?;
        mp.flag_set(MNT_DEFEXPORTED);
        np.exflags = argp.ex_flags;
        return Ok(());
    }
    let (Ok(addrlen), Ok(masklen)) = (
        usize::try_from(argp.ex_addrlen),
        usize::try_from(argp.ex_masklen),
    ) else {
        return Err(Errno::EINVAL);
    };
    if addrlen > MLEN || masklen > MLEN {
        return Err(Errno::EINVAL);
    }
    if addrlen < 2 {
        // No room for the family byte `saddr->sa_family` reads.
        return Err(Errno::EINVAL);
    }
    let Some(mut np) = MBox::new(M_NETADDR, M_WAITOK, Netcred::new()) else {
        kpanic(format_args!("vfs_hang_addrlist: out of memory"));
    };
    let mut saddr = [0u8; MLEN];
    UserPtr::<u8>::new(argp.ex_addr).copyin_slice(&mut saddr[..addrlen])?;
    if usize::from(saddr[0]) > addrlen {
        saddr[0] = addrlen as u8;
    }
    let mut smask = [0u8; MLEN];
    if masklen != 0 {
        UserPtr::<u8>::new(argp.ex_mask).copyin_slice(&mut smask[..masklen])?;
        if usize::from(smask[0]) > masklen {
            smask[0] = masklen as u8;
        }
    }
    // fill in the kernel's ucred from userspace's xucred
    crfromxucred(&mut np.anon, x.cr_uid, x.cr_gid, x.cr_ngroups, &x.cr_groups)?;
    if saddr[1] != AF_INET {
        return Err(Errno::EINVAL);
    }
    np.exflags = argp.ex_flags;
    let mask = (masklen != 0).then_some(&smask[..masklen]);
    let mut w = klock::witness();
    match nep
        .k
        .get_mut(&mut w)
        .rtable_inet
        .add(&saddr[..addrlen], mask, np)
    {
        Ok(_) => Ok(()),
        // already exists (or no memory for the tree): the entry is freed with the error
        Err(_) => Err(Errno::EPERM),
    }
}

/// `vfs_free_addrlist(nep)`: the list's entries freed (the tree dropped, every entry with it,
/// which `vfs_free_netcred` did one by one); the default entry stays as it is.
#[cfg(feature = "nfsserver")]
fn vfs_free_addrlist(nep: &Netexport) {
    let mut w = klock::witness();
    nep.k.get_mut(&mut w).rtable_inet = RadixTree::new(SIN_ADDR_OFF, M_RTABLE);
}

/// `vfs_export(mp, nep, argp)`: process a mount's export update: `MNT_DELEXPORT` empties the
/// list and clears `MNT_EXPORTED` and `MNT_DEFEXPORTED`, then `MNT_EXPORTED` hangs `argp`'s
/// entry and sets `MNT_EXPORTED`. `ENOTSUP` without `NFSSERVER`.
pub fn vfs_export<G: ProcGlue>(
    mp: &Mount<G>,
    nep: &Netexport,
    argp: &ExportArgs,
) -> Result<(), Errno> {
    #[cfg(feature = "nfsserver")]
    {
        if argp.ex_flags & MNT_DELEXPORT != 0 {
            vfs_free_addrlist(nep);
            mp.flag_clr(MNT_EXPORTED | MNT_DEFEXPORTED);
        }
        if argp.ex_flags & MNT_EXPORTED != 0 {
            vfs_hang_addrlist(mp, nep, argp)?;
            mp.flag_set(MNT_EXPORTED);
        }
        Ok(())
    }
    #[cfg(not(feature = "nfsserver"))]
    {
        let _ = (mp, nep, argp);
        Err(Errno::ENOTSUP)
    }
}

/// `vfs_export_lookup(mp, nep, nam)`: the rights of the client at `nam` (the `struct sockaddr`
/// mbuf; none: only the default export can answer) on an exported mount: its entry, else the
/// default when there is one; nothing on a mount that is not exported, or without
/// `NFSSERVER`. The kernel lock is held.
pub fn vfs_export_lookup<G: ProcGlue>(
    mp: &Mount<G>,
    nep: &Netexport,
    nam: Option<&Mbuf>,
) -> Option<ExportGrant> {
    #[cfg(feature = "nfsserver")]
    {
        if mp.flag() & MNT_EXPORTED == 0 {
            return None;
        }
        let r = klock::reader();
        let l = nep.k.get(&r);
        // Lookup in the export list first.
        let mut np = None;
        if let Some(nam) = nam {
            let saddr = nam.data();
            if saddr.get(1) == Some(&AF_INET) {
                np = l.rtable_inet.matches(saddr).map(|e| &**e);
            }
        }
        // If no address match, use the default if it exists.
        if np.is_none() && mp.flag() & MNT_DEFEXPORTED != 0 {
            np = Some(&l.defexported);
        }
        np.map(Netcred::grant)
    }
    #[cfg(not(feature = "nfsserver"))]
    {
        let _ = (mp, nep, nam);
        None
    }
}

/// `ufs_check_export` (and msdosfs's and cd9660's, the same body): a remote client's export
/// rights on `mp`, from the mount's list: `EACCES` without an entry for it.
pub fn check<G: ProcGlue>(mp: &Mount<G>, nam: &Mbuf) -> Result<ExportGrant, Errno> {
    vfs_export_lookup(mp, mp.export(), Some(nam)).ok_or(Errno::EACCES)
}
/* </CODE> */

/* <TESTS> */
#[cfg(all(test, feature = "nfsserver"))]
mod tests {
    // Host tests of the export list over a test mount: LZ's five (`bsd`'s vfs_subr.rs
    // `exports`) re-expressed, then the C's bounds around their edges, the length byte
    // clamped, a bad group count, the anonymous ids and groups by value (D-6), `check`'s
    // EACCES and the list emptied for a new mount. Addresses and masks are written into the
    // host's user memory double, as mountd's `export_args` point at its own.

    use std::boxed::Box;

    use hal::sys::mount::{MNT_EXPORTANON, MNT_EXRDONLY, Xucred};
    use hal::sys::syslimits::NGROUPS_MAX as XNGROUPS;

    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::net::mbuf::m_devget;
    use crate::testutil::USER_BASE;

    /// A `sockaddr_in` of `family` for `a`.
    fn sin(family: u8, a: [u8; 4]) -> [u8; 16] {
        let mut k = [0u8; 16];
        k[0] = 16;
        k[1] = family;
        k[4..8].copy_from_slice(&a);
        k
    }

    /// `b` copied out to the user memory double at `off`; its user address.
    fn user(off: usize, b: &[u8]) -> usize {
        let a = USER_BASE + off;
        UserPtr::<u8>::new(a).copyout_slice(b).expect("user memory");
        a
    }

    /// `struct export_args` for an address and a mask (user address and length each).
    fn args(flags: i32, anon: u32, addr: (usize, i32), mask: (usize, i32)) -> ExportArgs {
        ExportArgs {
            ex_flags: flags,
            ex_root: 0,
            ex_anon: Xucred {
                cr_uid: anon,
                cr_gid: anon,
                cr_ngroups: 0,
                _pad0: 0,
                cr_groups: [0; XNGROUPS],
            },
            _pad0: 0,
            ex_addr: addr.0,
            ex_addrlen: addr.1,
            _pad1: 0,
            ex_mask: mask.0,
            ex_masklen: mask.1,
            _pad2: 0,
        }
    }

    /// The arguments of an entry for the network or host `a` (`mask` none: a host).
    fn entry(flags: i32, anon: u32, a: [u8; 4], mask: Option<[u8; 4]>) -> ExportArgs {
        let addr = user(0, &sin(2, a));
        let m = mask.map_or((0, 0), |m| (user(64, &sin(2, m)), 16));
        args(flags, anon, (addr, 16), m)
    }

    /// The default entry's arguments.
    fn default(flags: i32, anon: u32) -> ExportArgs {
        args(flags, anon, (0, 0), (0, 0))
    }

    /// A fresh mount, on no list; the kernel lock held (the system call's).
    fn fresh() -> (std::sync::MutexGuard<'static, ()>, &'static Mount<TestGlue>) {
        let g = crate::net::mbuf::tests::setup();
        hal::klock::kernel_lock();
        (g, Box::leak(Box::new(Mount::new_unpooled(None))))
    }

    /// The grant a client at `a` gets, as `(exflags, anon uid)`.
    fn lookup(mp: &Mount<TestGlue>, a: [u8; 4]) -> Option<(i32, u32)> {
        let nam = m_devget(&sin(2, a), 0).expect("an address mbuf");
        vfs_export_lookup(mp, mp.export(), Some(&nam)).map(|g| (g.exflags, g.uid))
    }

    #[test]
    fn an_exported_network_matches_its_hosts_only() {
        let (_g, mp) = fresh();
        let ro = MNT_EXPORTED | MNT_EXRDONLY;
        let a = entry(ro, 32767, [10, 1, 0, 0], Some([255, 255, 0, 0]));
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        assert!(mp.flag() & MNT_EXPORTED != 0);
        assert_eq!(lookup(mp, [10, 1, 2, 3]), Some((ro, 32767)));
        assert_eq!(lookup(mp, [10, 1, 255, 255]), Some((ro, 32767)));
        assert_eq!(lookup(mp, [10, 2, 0, 1]), None);
        assert_eq!(lookup(mp, [192, 168, 0, 1]), None);
    }

    #[test]
    fn the_most_specific_entry_wins() {
        let (_g, mp) = fresh();
        let ro = MNT_EXPORTED | MNT_EXRDONLY;
        let net = entry(ro, 100, [10, 0, 0, 0], Some([255, 0, 0, 0]));
        assert_eq!(vfs_export(mp, mp.export(), &net), Ok(()));
        let host = entry(MNT_EXPORTED, 200, [10, 0, 0, 7], None);
        assert_eq!(vfs_export(mp, mp.export(), &host), Ok(()));
        assert_eq!(lookup(mp, [10, 0, 0, 7]), Some((MNT_EXPORTED, 200)));
        assert_eq!(lookup(mp, [10, 0, 0, 8]), Some((ro, 100)));
    }

    #[test]
    fn a_duplicate_entry_and_a_bad_address_are_refused() {
        let (_g, mp) = fresh();
        let host = entry(MNT_EXPORTED, 1, [10, 0, 0, 7], None);
        assert_eq!(vfs_export(mp, mp.export(), &host), Ok(()));
        let again = entry(MNT_EXPORTED, 2, [10, 0, 0, 7], None);
        assert_eq!(vfs_export(mp, mp.export(), &again), Err(Errno::EPERM));
        // The first one is still the entry.
        assert_eq!(lookup(mp, [10, 0, 0, 7]), Some((MNT_EXPORTED, 1)));

        // AF_UNSPEC is not an address family the export list knows.
        let other = args(
            MNT_EXPORTED,
            1,
            (user(0, &sin(0, [10, 0, 0, 9])), 16),
            (0, 0),
        );
        assert_eq!(vfs_export(mp, mp.export(), &other), Err(Errno::EINVAL));
        // Lengths past an mbuf, or negative.
        let mut a = entry(MNT_EXPORTED, 1, [10, 0, 0, 7], None);
        a.ex_addrlen = MLEN as i32 + 1;
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        a.ex_addrlen = -1;
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        a.ex_addrlen = 16;
        a.ex_masklen = -1;
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        a.ex_masklen = MLEN as i32 + 1;
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        // A bad address or mask pointer is the copyin's EFAULT.
        let a = args(MNT_EXPORTED, 1, (8, 16), (0, 0));
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EFAULT));
        let a = args(
            MNT_EXPORTED,
            1,
            (user(0, &sin(2, [10, 0, 0, 3])), 16),
            (8, 16),
        );
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EFAULT));
        assert_eq!(lookup(mp, [10, 0, 0, 3]), None, "nothing hung");
    }

    #[test]
    fn the_default_export_serves_everyone_else() {
        let (_g, mp) = fresh();
        // Not exported at all: nothing, whatever the lists hold.
        assert_eq!(lookup(mp, [10, 0, 0, 1]), None);
        let host = entry(MNT_EXPORTED, 1, [10, 0, 0, 7], None);
        assert_eq!(vfs_export(mp, mp.export(), &host), Ok(()));
        assert_eq!(lookup(mp, [172, 16, 0, 1]), None);

        let ro = MNT_EXPORTED | MNT_EXRDONLY;
        assert_eq!(vfs_export(mp, mp.export(), &default(ro, 77)), Ok(()));
        assert!(mp.flag() & MNT_DEFEXPORTED != 0);
        assert_eq!(lookup(mp, [172, 16, 0, 1]), Some((ro, 77)));
        assert_eq!(lookup(mp, [10, 0, 0, 7]), Some((MNT_EXPORTED, 1)));
        // No address at all (a NULL `nam`): only the default.
        assert_eq!(
            vfs_export_lookup(mp, mp.export(), None).map(|g| g.exflags),
            Some(ro)
        );
        // Only one default.
        assert_eq!(
            vfs_export(mp, mp.export(), &default(ro, 78)),
            Err(Errno::EPERM)
        );
        assert_eq!(lookup(mp, [172, 16, 0, 1]), Some((ro, 77)));
        // The default is used only on an exported mount (the C's test inside MNT_EXPORTED).
        mp.flag_clr(MNT_EXPORTED);
        assert_eq!(lookup(mp, [172, 16, 0, 1]), None);
    }

    #[test]
    fn mnt_delexport_empties_the_list() {
        let (_g, mp) = fresh();
        let net = entry(MNT_EXPORTED, 5, [10, 1, 0, 0], Some([255, 255, 0, 0]));
        assert_eq!(vfs_export(mp, mp.export(), &net), Ok(()));
        assert_eq!(
            vfs_export(mp, mp.export(), &default(MNT_EXPORTED, 6)),
            Ok(())
        );
        assert!(lookup(mp, [10, 1, 0, 9]).is_some());

        assert_eq!(
            vfs_export(mp, mp.export(), &default(MNT_DELEXPORT, 0)),
            Ok(())
        );
        assert_eq!(mp.flag() & (MNT_EXPORTED | MNT_DEFEXPORTED), 0);
        assert_eq!(lookup(mp, [10, 1, 0, 9]), None);
        assert!(vfs_export_lookup(mp, mp.export(), None).is_none());

        // And the list can be built again.
        let net = entry(MNT_EXPORTED, 8, [10, 1, 0, 0], Some([255, 255, 0, 0]));
        assert_eq!(vfs_export(mp, mp.export(), &net), Ok(()));
        assert_eq!(lookup(mp, [10, 1, 0, 9]), Some((MNT_EXPORTED, 8)));
        // Delete and add in one call, as mountd reloading its exports does.
        let both = entry(
            MNT_DELEXPORT | MNT_EXPORTED,
            9,
            [10, 1, 0, 0],
            Some([255, 255, 0, 0]),
        );
        assert_eq!(vfs_export(mp, mp.export(), &both), Ok(()));
        assert_eq!(
            lookup(mp, [10, 1, 0, 9]),
            Some((MNT_DELEXPORT | MNT_EXPORTED, 9))
        );
    }

    #[test]
    fn the_lengths_at_their_bounds() {
        let (_g, mp) = fresh();
        // ex_addrlen 1: no family byte (LZ's EINVAL).
        let a = args(
            MNT_EXPORTED,
            1,
            (user(0, &sin(2, [10, 0, 0, 1])), 1),
            (0, 0),
        );
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        // ex_addrlen 2: a family and nothing to compare from the offset, a host route on the
        // left root's key: radix.c refuses it (EPERM).
        let a = args(
            MNT_EXPORTED,
            1,
            (user(0, &sin(2, [10, 0, 0, 1])), 2),
            (0, 0),
        );
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EPERM));
        // ex_addrlen MLEN: the sockaddr_in and zeros behind it.
        let mut long = [0u8; MLEN];
        long[..16].copy_from_slice(&sin(2, [10, 0, 0, 2]));
        let a = args(MNT_EXPORTED, 2, (user(0, &long), MLEN as i32), (0, 0));
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        assert_eq!(lookup(mp, [10, 0, 0, 2]), Some((MNT_EXPORTED, 2)));
        // Masks of 1, 2 and MLEN bytes: shorter than the offset, all the zero mask.
        for (i, masklen) in [1usize, 2].into_iter().enumerate() {
            let addr = user(0, &sin(2, [10, 3 + i as u8, 0, 0]));
            let mask = user(64, &sin(2, [255, 0, 0, 0]));
            let a = args(MNT_EXPORTED, 3, (addr, 16), (mask, masklen as i32));
            assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()), "masklen {masklen}");
        }
        let mut lmask = [0u8; MLEN];
        lmask[..16].copy_from_slice(&sin(2, [255, 255, 255, 0]));
        let a = args(
            MNT_EXPORTED,
            4,
            (user(0, &sin(2, [10, 9, 9, 0])), 16),
            (user(512, &lmask), MLEN as i32),
        );
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        assert_eq!(lookup(mp, [10, 9, 9, 200]), Some((MNT_EXPORTED, 4)));
    }

    #[test]
    fn a_length_byte_above_the_bytes_copied_is_clamped() {
        let (_g, mp) = fresh();
        let mut s = sin(2, [10, 0, 0, 7]);
        s[0] = 200;
        let a = args(MNT_EXPORTED, 7, (user(0, &s), 16), (0, 0));
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        assert_eq!(lookup(mp, [10, 0, 0, 7]), Some((MNT_EXPORTED, 7)));
        // A mask's too: 255.255.0.0 under a length byte of 99, 16 bytes copied.
        let mut m = sin(2, [255, 255, 0, 0]);
        m[0] = 99;
        let a = args(
            MNT_EXPORTED,
            8,
            (user(0, &sin(2, [172, 16, 0, 0])), 16),
            (user(64, &m), 16),
        );
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        assert_eq!(lookup(mp, [172, 16, 3, 3]), Some((MNT_EXPORTED, 8)));
        // The clamp decides what is compared: a host whose `sin_zero` is not zero, then the
        // same address in 8 bytes under a length byte of 16, read as 8: the same key from the
        // offset to its length, so radix.c's duplicate (EPERM).
        let mut z = sin(2, [10, 0, 0, 20]);
        z[10] = 0x55;
        let a = args(MNT_EXPORTED, 9, (user(0, &z), 16), (0, 0));
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        let a = args(
            MNT_EXPORTED,
            10,
            (user(0, &sin(2, [10, 0, 0, 20])), 8),
            (0, 0),
        );
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EPERM));
    }

    #[test]
    fn a_bad_group_count_hangs_nothing() {
        let (_g, mp) = fresh();
        let mut a = entry(MNT_EXPORTED, 9, [10, 0, 0, 9], None);
        a.ex_anon.cr_ngroups = XNGROUPS as i16 + 1;
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        a.ex_anon.cr_ngroups = -1;
        assert_eq!(vfs_export(mp, mp.export(), &a), Err(Errno::EINVAL));
        assert_eq!(lookup(mp, [10, 0, 0, 9]), None);
        let mut d = default(MNT_EXPORTED, 9);
        d.ex_anon.cr_ngroups = XNGROUPS as i16 + 1;
        assert_eq!(vfs_export(mp, mp.export(), &d), Err(Errno::EINVAL));
        assert_eq!(mp.flag() & MNT_DEFEXPORTED, 0, "no default hung");
    }

    #[test]
    fn the_grant_is_the_entry_by_value() {
        let (_g, mp) = fresh();
        let mut a = entry(MNT_EXPORTED | MNT_EXPORTANON, 32767, [10, 0, 0, 5], None);
        a.ex_anon.cr_gid = 31;
        a.ex_anon.cr_ngroups = 3;
        a.ex_anon.cr_groups[..3].copy_from_slice(&[31, 5, 20]);
        assert_eq!(vfs_export(mp, mp.export(), &a), Ok(()));
        let nam = m_devget(&sin(2, [10, 0, 0, 5]), 0).expect("an address mbuf");
        let g = check(mp, &nam).expect("a grant");
        assert_eq!(g.exflags, MNT_EXPORTED | MNT_EXPORTANON);
        assert_eq!((g.uid, g.gid, g.ngroups), (32767, 31, 3));
        assert_eq!(&g.groups[..3], &[31, 5, 20]);
        // D-6: the entry deleted (mountd's MNT_DELEXPORT while the server slept in
        // VFS_FHTOVP), the grant taken before is the entry as it matched.
        assert_eq!(
            vfs_export(mp, mp.export(), &default(MNT_DELEXPORT, 0)),
            Ok(())
        );
        assert_eq!((g.uid, g.gid, g.ngroups), (32767, 31, 3));
        assert_eq!(check(mp, &nam), Err(Errno::EACCES));
    }

    #[test]
    fn check_refuses_a_client_without_an_entry() {
        let (_g, mp) = fresh();
        let nam = m_devget(&sin(2, [10, 0, 0, 5]), 0).expect("an address mbuf");
        assert_eq!(check(mp, &nam), Err(Errno::EACCES), "not exported");
        let host = entry(MNT_EXPORTED, 1, [10, 0, 0, 6], None);
        assert_eq!(vfs_export(mp, mp.export(), &host), Ok(()));
        assert_eq!(check(mp, &nam), Err(Errno::EACCES), "no entry");
        // An address mbuf of another family, or too short for one, finds only the default.
        let six = m_devget(&[28, 24, 0, 0], 0).expect("an address mbuf");
        assert_eq!(check(mp, &six), Err(Errno::EACCES));
        let short = m_devget(&[16], 0).expect("an address mbuf");
        assert_eq!(check(mp, &short), Err(Errno::EACCES));
        assert_eq!(
            vfs_export(mp, mp.export(), &default(MNT_EXPORTED, 3)),
            Ok(())
        );
        assert_eq!(check(mp, &six).map(|g| g.uid), Ok(3));
        assert_eq!(check(mp, &short).map(|g| g.uid), Ok(3));
        assert_eq!(check(mp, &nam).map(|g| g.uid), Ok(3));
    }

    #[test]
    fn a_new_mount_starts_with_an_empty_list() {
        let (_g, mp) = fresh();
        let host = entry(MNT_EXPORTED, 1, [10, 0, 0, 6], None);
        assert_eq!(vfs_export(mp, mp.export(), &host), Ok(()));
        assert_eq!(
            vfs_export(mp, mp.export(), &default(MNT_EXPORTED, 2)),
            Ok(())
        );
        mp.export().clear();
        // The flags are the mount's: still set, but the list holds nothing, the default's ids
        // are zeroed.
        assert_eq!(lookup(mp, [10, 0, 0, 6]), Some((0, 0)));
        mp.flag_clr(MNT_DEFEXPORTED);
        assert_eq!(lookup(mp, [10, 0, 0, 6]), None);
    }
}
/* </TESTS> */
