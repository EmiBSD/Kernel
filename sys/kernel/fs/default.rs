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
 * Portions of this code are:
 *
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
 */
/* </LICENSES> */

/* <CODE> */
//! The generic vnode operations file systems share: `vop_generic_revoke` (revoke a vnode and
//! its aliases), `vop_generic_badop` (an operation that must never be called),
//! `vop_generic_bmap` (the identity block map), `vop_generic_bwrite`, `vop_generic_abortop`
//! (drop the pathname buffer) and `vop_generic_lookup` (a lookup that always fails).
//!
//! Upstream: sys/kern/vfs_default.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_default.rs@3c62ede68802
//!
//! ## Redesign
//! - The operations take the C's arguments one by one (the [`super::vops::Vnops`] methods
//!   call them), not their argument structures; `vop_generic_badop` never returns, so it
//!   fills a slot of any type.
//! - `vop_generic_abortop`'s `pool_put(&namei_pool, cnp->cn_pnbuf)` and `vop_generic_revoke`'s
//!   `dounmount(mp, MNT_FORCE | MNT_DOOMED, p)` are the glue's (`namei` and `dounmount` are
//!   `bsd`'s until N5.3 and N5.4).
//! - `vop_generic_bwrite` passes the buffer as it came ([`IoBuf`]): a token to the kernel's
//!   `bwrite`, a lent buffer's parked token likewise.

use core::ptr;

use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_DOOMED, MNT_FORCE, VB_WAIT, VB_WRITE};
use hal::sys::types::Daddr;

use super::bio;
use super::mount::vfs_busy;
use super::spec::spechash;
use super::subr::{vfs, vgonel};
use super::vnode::{PINOD, REVOKEALL, VALIASED, VBLK, VNODE_MTX, VXLOCK, VXWANT, Vnode};
use super::vops::{Cn, IoBuf};
use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::proc::ProcGlue;

/// Eliminate all activity associated with the requested vnode and with all vnodes aliased to
/// the requested vnode.
pub fn vop_generic_revoke<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    #[cfg(feature = "diagnostic")]
    if flags & REVOKEALL == 0 {
        kpanic(format_args!("vop_generic_revoke"));
    }
    #[cfg(not(feature = "diagnostic"))]
    let _ = (flags, REVOKEALL);

    G::with_curproc(|p| {
        while vp.vtype() == VBLK
            && let Some(mp) = vp.specmountpoint()
        {
            // If we have a mount point associated with the vnode, we must flush it out now, as
            // to not leave a dangling zombie mount point laying around in VFS.
            if vfs_busy(mp, VB_WRITE | VB_WAIT).is_ok() {
                if let Some(p) = p {
                    let _ = <G::FileGlue as FileGlue>::dounmount(mp, MNT_FORCE | MNT_DOOMED, p);
                }
                break;
            }
        }

        if vp.flag() & VALIASED != 0 {
            // If a vgone (or vclean) is already in progress, wait until it is done and return.
            let mut g = VNODE_MTX.lock();
            if vp.vlock.get(&g).lflag & VXLOCK != 0 {
                vp.vlock.get_mut(&mut g).lflag |= VXWANT;
                let _ = g.sleep(ptr::from_ref(vp), PINOD, "vop_generic_revokeall");
                return Ok(());
            }

            // Ensure that vp will not be vgone'd while we are eliminating its aliases.
            vp.vlock.get_mut(&mut g).lflag |= VXLOCK;
            drop(g);

            while vp.flag() & VALIASED != 0 {
                let Some(vq) = other_alias(vp) else {
                    break;
                };
                vgonel(vq, p);
            }

            // Remove the lock so that vgone below will really eliminate the vnode after which
            // time vgone will awaken any sleepers.
            let mut g = VNODE_MTX.lock();
            vp.vlock.get_mut(&mut g).lflag &= !VXLOCK;
        }

        vgonel(vp, p);

        Ok(())
    })
}

/// The first alias of `vp` on its chain other than `vp` itself (the same device and type),
/// found under the chains' lock.
fn other_alias<G: ProcGlue>(vp: &'static Vnode<G>) -> Option<&'static Vnode<G>> {
    let rdev = vp.rdev();
    let vt = vp.vtype();
    let chains = vfs::<G>().spechash.lock();
    chains[spechash(rdev)]
        .iter()
        .find(|vq| !ptr::eq(*vq, vp) && vq.vtype() == vt && vq.rdev() == rdev)
}

/// `vop_generic_badop`: an operation that must never be reached.
pub fn vop_generic_badop() -> ! {
    kpanic(format_args!("vop_generic_badop"))
}

/// `vop_generic_bmap`: the identity block map: the vnode itself, the same block, no run.
pub fn vop_generic_bmap<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode<G>>>,
    bnp: Option<&mut Daddr>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    if let Some(vpp) = vpp {
        *vpp = Some(vp);
    }
    if let Some(bnp) = bnp {
        *bnp = bn;
    }
    if let Some(runp) = runp {
        *runp = 0;
    }
    Ok(())
}

/// `vop_generic_bwrite`: `bwrite(ap->a_bp)`.
pub fn vop_generic_bwrite(bp: IoBuf) -> Result<(), Errno> {
    match bp.into_token() {
        Some(t) => bio::bwrite(t),
        None => kpanic(format_args!("vop_generic_bwrite: a buffer with no token")),
    }
}

/// `vop_generic_abortop`: frees the pathname buffer a lookup left behind, unless the caller
/// asked to keep it (`SAVESTART`).
pub fn vop_generic_abortop<G: ProcGlue>(cnp: &mut Cn<G>) -> Result<(), Errno> {
    <G::FileGlue as FileGlue>::abortop(cnp);
    Ok(())
}

/// Trivial lookup routine that always fails.
pub fn vop_generic_lookup<G: ProcGlue>(vpp: &mut Option<&'static Vnode<G>>) -> Result<(), Errno> {
    *vpp = None;
    Err(Errno::ENOTDIR)
}
/* </CODE> */
