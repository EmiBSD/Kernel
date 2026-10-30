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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)tty_pty.c	8.4 (Berkeley) 2/20/95
 */
/* </LICENSES> */

/* <CODE> */
//! The pseudo-teletype driver's `bsd` side, `kern/tty_pty.c`: the device switch's entry
//! points of `pts` (5) and `ptc` (6), bound to the kernel's ptys (`kernel::pty`, N4.7b), and
//! the `/dev/ptm` cloning device (81), whose `PTMGET` opens a free pair through the vnode
//! layer and the descriptor table.
//!
//! Upstream: sys/kern/tty_pty.c @ 3ce1f3f79392
//! LZ: sys/kern/tty_pty.rs@3c62ede68802
//!
//! ## Deviations
//! - `PTMGET` opens `/dev/ptyXX` and `/dev/ttyXX` through `namei`, as the C does; without a
//!   root file system both lookups fail with `ENOENT` (LZ's).
//! - `ptyioctl` recognises the master by comparing `cdevsw[major(dev)].d_open` with
//!   `ptcopen` as the C does, with `ptr::fn_addr_eq` (LZ's).
//!
//! ## Redesign
//! - N4.7b: the softcs, the slave and master entry points, the filters and `ptyioctl`'s body
//!   are the kernel's (`kernel::pty`'s documentation has the shape); LZ's eight `unsafe` (the
//!   softc array and its raw slots, the softcs' `Sync`, `kn_pti`, `ptm_vn_open`'s detached
//!   `&Proc`) are gone: the array is the kernel's table, and `ptm_vn_open` takes the thread
//!   as a parameter of its own instead of re-borrowing it out of the `Nameidata`.
//! - What stays is `ptmioctl`'s `PTMGET` (the descriptors, the three-stage rocket on the
//!   slave's vnode, the root credentials of `ptm_vn_open`: VFS, N5), `ptmattach`'s scan of
//!   the device switch (N7), and the names the switch rows and `kern_pledge` know.

use core::ptr;
use core::sync::atomic::Ordering;

use crate::kern::kern_descrip::{BsdFileData, closef, falloc, fdinsert, fdremove};
use crate::kern::kern_prot::{crfree, crget};
use crate::kern::kern_tc::getnanotime;
use crate::kern::proc_glue::BsdProcGlue;
use crate::kern::subr_prf::panic;
use crate::kern::tty::TtyUio;
use crate::kern::vfs_lookup::{namei, ndinit};
use crate::kern::vfs_vnops::VNOPS;
use crate::kern::vfs_vops::{VOP_OPEN, VOP_REVOKE, VOP_SETATTR, VOP_UNLOCK};
use crate::machine::conf::{cdevsw, nchrdev};
use crate::sys::conf::DevTypeOpen;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::fcntl::{FREAD, FWRITE};
use crate::sys::file::{DTYPE_VNODE, frele};
use crate::sys::filedesc::{fdplock, fdpunlock};
use crate::sys::mount::MNT_RDONLY;
use crate::sys::namei::{KERNELPATH, LOCKLEAF, LOOKUP, NOFOLLOW, Nameidata, NiDirp};
use crate::sys::pledge::{PLEDGE_RPATH, PLEDGE_WPATH};
use crate::sys::proc::Proc;
use crate::sys::stat::{ALLPERMS, S_IRUSR, S_IWGRP, S_IWUSR};
use crate::sys::tty::{PTMGET, Ptmget, TTY_GID, Tty};
use crate::sys::types::{Dev, major, minor};
use crate::sys::ucred::CredRef;
use crate::sys::uio::Uio;
use crate::sys::vnode::{REVOKEALL, VA_UTIMES_NULL, VALIASED, VCHR, Vattr};
use crate::sys::vnode::{vattr_null, vput, vrele};

pub use kernel::pty::{
    NPTY, NPTY_MAX, NPTY_MIN, PF_NOSTOP, PF_PKT, PF_REMOTE, PF_STOPPED, PF_UCNTL,
};

/// `tty_gid`.
static TTY_GID_: u32 = TTY_GID;

/// `check_pty`: check if the minor is correct and ensure necessary structures are properly
/// allocated.
pub fn check_pty(dev: Dev) -> Result<(), Errno> {
    kernel::pty::check_pty::<BsdProcGlue>(dev)
}

/// `ptyattach`: establish `n` (or default if `n` is 1) ptys in the system; if we have pty,
/// we need ptm too.
pub fn ptyattach(n: i32) {
    kernel::pty::ptyattach::<BsdProcGlue>(n);
    ptmattach(1);
}

/// `ptsopen`: open the slave side.
pub fn ptsopen(dev: Dev, flag: i32, devtype: i32, p: &Proc) -> Result<(), Errno> {
    kernel::pty::ptsopen::<BsdProcGlue>(dev, flag, devtype, p)
}

/// `ptsclose`: close the slave side.
pub fn ptsclose(dev: Dev, flag: i32, mode: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::pty::ptsclose::<BsdProcGlue>(dev, flag, mode, p)
}

/// `ptsread`: read from the slave side.
pub fn ptsread(dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::pty::ptsread::<BsdProcGlue>(dev, &mut TtyUio { uio }, flag)
}

/// `ptswrite`: write to pseudo-tty.
pub fn ptswrite(dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::pty::ptswrite::<BsdProcGlue>(dev, &mut TtyUio { uio }, flag)
}

/// `ptsstop`: stop output on the slave side (`cdevsw[5].d_stop`).
pub fn ptsstop(tp: &Tty, flush: i32) -> Result<(), Errno> {
    kernel::pty::ptsstop(tp, flush)
}

/// `ptcopen`: open the master side (its address is what `kern_pledge` and `ptyioctl`
/// compare).
pub fn ptcopen(dev: Dev, flag: i32, devtype: i32, p: &Proc) -> Result<(), Errno> {
    kernel::pty::ptcopen::<BsdProcGlue>(dev, flag, devtype, p)
}

/// `ptcclose`: close the master side.
pub fn ptcclose(dev: Dev, flag: i32, devtype: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::pty::ptcclose::<BsdProcGlue>(dev, flag, devtype, p)
}

/// `ptcread`: read the slave's output from the master side.
pub fn ptcread(dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::pty::ptcread::<BsdProcGlue>(dev, &mut TtyUio { uio }, flag)
}

/// `ptcwrite`: feed the slave's input from the master side.
pub fn ptcwrite(dev: Dev, uio: &mut Uio<'_>, flag: i32) -> Result<(), Errno> {
    kernel::pty::ptcwrite::<BsdProcGlue>(dev, &mut TtyUio { uio }, flag)
}

/// `ptckqfilter`: attaches a kqueue filter to the master side.
pub fn ptckqfilter(dev: Dev, kn: &Knote) -> Result<(), Errno> {
    kernel::pty::ptckqfilter::<BsdProcGlue>(dev, kn)
}

/// `ptytty` (also `ptstty` and `ptctty`): the tty of a pty.
pub fn ptytty(dev: Dev) -> Option<&'static Tty> {
    kernel::pty::ptytty::<BsdProcGlue>(dev)
}

/// `ptyioctl` (also `ptsioctl` and `ptcioctl`): the master is the device whose switch row
/// opens with `ptcopen`.
pub fn ptyioctl(dev: Dev, cmd: u64, data: &mut [u8], flag: i32, p: &Proc) -> Result<(), Errno> {
    let master = ptr::fn_addr_eq(cdevsw(major(dev)).d_open, ptcopen as DevTypeOpen);
    kernel::pty::ptyioctl::<BsdProcGlue>(dev, master, cmd, data, flag, p)
}

/// `pty_isfree`.
fn pty_isfree(minor: u32) -> bool {
    kernel::pty::pty_isfree::<BsdProcGlue>(minor)
}

/// `pty_getfree`: the first free pty, or the one past the array.
pub fn pty_getfree() -> Dev {
    kernel::pty::pty_getfree::<BsdProcGlue>()
}

/// `ptm_vn_open`: hacked up version of `vn_open`. We _only_ handle ptys and only open them
/// with `FREAD|FWRITE` and never deal with creat or stuff like that.
///
/// We need it because we have to fake up root credentials to open the pty.
fn ptm_vn_open(ndp: &mut Nameidata<'_>, p: &Proc) -> Result<(), Errno> {
    namei(ndp)?;
    let Some(vp) = ndp.ni_vp else {
        return Err(Errno::ENOENT);
    };
    if vp.vtype() != VCHR {
        vput(vp);
        return Err(Errno::EINVAL);
    }

    // Get us a fresh cred with root privileges.
    let cred = crget();
    let error = VOP_OPEN(vp, FREAD | FWRITE, CredRef::as_ptr(&cred), p);
    if error.is_ok() {
        // update atime/mtime
        let mut vattr = Vattr::new();
        vattr_null(&mut vattr);
        vattr.va_atime = getnanotime();
        vattr.va_mtime = vattr.va_atime;
        vattr.va_vaflags |= VA_UTIMES_NULL;
        let _ = VOP_SETATTR(vp, &mut vattr, p.ucred_ptr(), p);
    }
    crfree(cred);

    if let Err(e) = error {
        vput(vp);
        return Err(e);
    }

    vp.set_writecount(vp.writecount() + 1);

    Ok(())
}

/// `ptmattach`: find the major and minor of the pty devices.
pub fn ptmattach(_n: i32) {
    let n = nchrdev();
    let mut i = 0;
    while i < n {
        if ptr::fn_addr_eq(cdevsw(i).d_open, ptsopen as DevTypeOpen) {
            break;
        }
        i += 1;
    }

    if i == n {
        panic(format_args!("ptmattach: Can't find pty slave in cdevsw"));
    }

    kernel::pty::set_pts_major::<BsdProcGlue>(i as i32);
}

/// `ptmopen`.
pub fn ptmopen(_dev: Dev, _flag: i32, _mode: i32, _p: &Proc) -> Result<(), Errno> {
    Ok(())
}

/// `ptmclose`.
pub fn ptmclose(_dev: Dev, _flag: i32, _mode: i32, _p: Option<&Proc>) -> Result<(), Errno> {
    Ok(())
}

/// `ptmioctl`: `PTMGET` hands out a free master/slave pair, opened, at two new descriptors.
pub fn ptmioctl(_dev: Dev, cmd: u64, data: &mut [u8], _flag: i32, p: &Proc) -> Result<(), Errno> {
    let fdp = p.fd();

    if cmd != PTMGET {
        return Err(Errno::EINVAL);
    }

    let mut g = fdplock(fdp);
    // Grab two filedescriptors.
    let (cfp, cindx) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(e) => {
            fdpunlock(g);
            return Err(e);
        }
    };
    let (sfp, sindx) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(e) => {
            let _ = fdremove(&mut g, cindx);
            fdpunlock(g);
            let _ = closef(cfp, p);
            return Err(e);
        }
    };
    fdpunlock(g);

    let result = 'bad: loop {
        // retry:
        // Find and open a free master pty.
        let newdev = pty_getfree();
        if let Err(e) = check_pty(newdev) {
            break 'bad Err(e);
        }
        let Some(pti) = kernel::pty::pt_softc::<BsdProcGlue>(minor(newdev)) else {
            break 'bad Err(Errno::ENXIO);
        };
        let pn = pti.pty_pn;
        let mut cnd = ndinit(
            LOOKUP,
            NOFOLLOW | LOCKLEAF | KERNELPATH,
            NiDirp::Sys(&pn),
            p,
        );
        cnd.ni_pledge = PLEDGE_RPATH | PLEDGE_WPATH;
        if let Err(e) = ptm_vn_open(&mut cnd, p) {
            // Check if the master open failed because we lost the race to grab it.
            if e == Errno::EIO && !pty_isfree(minor(newdev)) {
                continue 'bad;
            }
            break 'bad Err(e);
        }
        let Some(cvp) = cnd.ni_vp else {
            break 'bad Err(Errno::ENOENT);
        };
        cfp.f_flag.store((FREAD | FWRITE) as u32, Ordering::SeqCst);
        cfp.install(DTYPE_VNODE, &VNOPS, BsdFileData::vnode(cvp));
        let _ = VOP_UNLOCK(cvp);

        // Open the slave.
        // namei -> setattr -> unlock -> revoke -> vrele -> namei -> open -> unlock
        // Three stage rocket:
        // 1. Change the owner and permissions on the slave.
        // 2. Revoke all the users of the slave.
        // 3. open the slave.
        let sn = pti.pty_sn;
        let mut snd = ndinit(
            LOOKUP,
            NOFOLLOW | LOCKLEAF | KERNELPATH,
            NiDirp::Sys(&sn),
            p,
        );
        snd.ni_pledge = PLEDGE_RPATH | PLEDGE_WPATH;
        if let Err(e) = namei(&mut snd) {
            break 'bad Err(e);
        }
        let Some(svp) = snd.ni_vp else {
            break 'bad Err(Errno::ENOENT);
        };
        if svp.mount().is_some_and(|mp| mp.flag() & MNT_RDONLY == 0) {
            let gid = TTY_GID_;
            // get real uid
            let uid = p.ucred().cr_ruid.get();

            let mut vattr = Vattr::new();
            vattr_null(&mut vattr);
            vattr.va_uid = uid;
            vattr.va_gid = gid;
            vattr.va_mode = (S_IRUSR | S_IWUSR | S_IWGRP) & ALLPERMS;
            // Get a fake cred to pretend we're root.
            let cred = crget();
            let error = VOP_SETATTR(svp, &mut vattr, CredRef::as_ptr(&cred), p);
            crfree(cred);
            if let Err(e) = error {
                vput(svp);
                break 'bad Err(e);
            }
        }
        let _ = VOP_UNLOCK(svp);
        if svp.usecount() > 1 || svp.flag() & VALIASED != 0 {
            let _ = VOP_REVOKE(svp, REVOKEALL);
        }

        // The vnode is useless after the revoke, we need to namei again.
        vrele(svp);

        let mut snd = ndinit(
            LOOKUP,
            NOFOLLOW | LOCKLEAF | KERNELPATH,
            NiDirp::Sys(&sn),
            p,
        );
        snd.ni_pledge = PLEDGE_RPATH | PLEDGE_WPATH;
        // now open it
        if let Err(e) = ptm_vn_open(&mut snd, p) {
            break 'bad Err(e);
        }
        let Some(svp) = snd.ni_vp else {
            break 'bad Err(Errno::ENOENT);
        };
        sfp.f_flag.store((FREAD | FWRITE) as u32, Ordering::SeqCst);
        sfp.install(DTYPE_VNODE, &VNOPS, BsdFileData::vnode(svp));
        let _ = VOP_UNLOCK(svp);

        // now, put the indexen and names into struct ptmget
        let mut ptm = Ptmget {
            cfd: cindx,
            sfd: sindx,
            ..Ptmget::default()
        };
        ptm.cn[..pn.len()].copy_from_slice(&pn);
        ptm.sn[..sn.len()].copy_from_slice(&sn);
        if !hal::user::pod_to_bytes(&ptm, data) {
            break 'bad Err(Errno::EINVAL);
        }

        // insert files now that we've passed all errors
        let mut g = fdplock(fdp);
        fdinsert(&mut g, cindx, 0, &cfp);
        fdinsert(&mut g, sindx, 0, &sfp);
        fdpunlock(g);
        break 'bad Ok(());
    };

    if result.is_err() {
        // bad:
        let mut g = fdplock(fdp);
        let _ = fdremove(&mut g, cindx);
        let _ = fdremove(&mut g, sindx);
        fdpunlock(g);
        let _ = closef(cfp, p);
        let _ = closef(sfp, p);
    } else {
        let _ = frele(cfp, p);
        let _ = frele(sfp, p);
    }
    result
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The glue's device switch work: ptmattach finds the slave's major, ptyioctl tells the
    // master from the slave by the switch row's open (the pty itself is kernel::pty's, tested
    // there).

    use super::*;

    #[test]
    fn ptmattach_finds_the_slave_s_major_and_ptcopen_is_the_master_s_open() {
        // The host's switch has amd64's slots: pts at 5, ptc at 6, ptm at 81.
        ptyattach(1);
        assert_eq!(kernel::pty::pts_major::<BsdProcGlue>(), 5);
        assert!(ptr::fn_addr_eq(cdevsw(6).d_open, ptcopen as DevTypeOpen));
        assert!(!ptr::fn_addr_eq(cdevsw(5).d_open, ptcopen as DevTypeOpen));
        assert!(ptr::fn_addr_eq(cdevsw(81).d_open, ptmopen as DevTypeOpen));
    }
}
/* </TESTS> */
