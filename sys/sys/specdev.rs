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
 * Copyright (c) 1990, 1993
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
 *	@(#)specdev.h	8.3 (Berkeley) 8/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/specdev.h>` as `bsd` names it. Since N5.2 `struct specinfo`, the hash of device vnodes,
//! the cloning devices and the special files' operations are the kernel crate's
//! (`kernel::fs::spec`); this module is their face for the `bsd` file systems: the `spec_*`
//! operations with the `struct vop_*_args` their tables hold (`ffs_specvops`, ...), over the
//! kernel's, and `bsd`'s device switch (`machine::conf`'s `cdevsw[]` and `bdevsw[]`) as the
//! kernel's `Devsw` (a bridge until the device switch moves, N7.2).
//!
//! Upstream: sys/sys/specdev.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - `vp->v_op = &spec_vops` is `vp.set_ops_id(VopsId::SPEC)` (the kernel's table has a fixed
//!   id); the `spec_*` functions here serve the file systems' own special-file tables.

use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::conf::{bdevsw, cdevsw, iskmemdev, iszerodev, nblkdev, nchrdev};
use crate::sys::buf::lend;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::proc::Proc;
use crate::sys::types::{Dev, major};
use crate::sys::uio::Uio;
use crate::sys::vnode::{
    VopAccessArgs, VopAdvlockArgs, VopCloseArgs, VopFsyncArgs, VopGetattrArgs, VopInactiveArgs,
    VopIoctlArgs, VopKqfilterArgs, VopOpenArgs, VopPathconfArgs, VopPrintArgs, VopReadArgs,
    VopSetattrArgs, VopStrategyArgs, VopWriteArgs,
};
use kernel::fs::spec::{self, Devsw};
use kernel::fs::vops::{Cred, IoBuf};

pub use kernel::fs::spec::{CLONE_MAPSZ, CLONE_SHIFT, SPECHSZ, spechash};

/// `struct specinfo`: the kernel's.
pub type Specinfo = kernel::fs::spec::Specinfo<BsdProcGlue>;

/// Open a special file.
pub fn spec_open(ap: &mut VopOpenArgs<'_>) -> Result<(), Errno> {
    spec::spec_open(ap.a_vp, ap.a_mode, Cred::from_raw(ap.a_cred), ap.a_p)
}

/// Vnode op for read.
pub fn spec_read(ap: &mut VopReadArgs<'_, '_>) -> Result<(), Errno> {
    spec::spec_read(ap.a_vp, ap.a_uio, ap.a_ioflag)
}

/// `spec_inactive`: unlocks the vnode.
pub fn spec_inactive(ap: &mut VopInactiveArgs<'_>) -> Result<(), Errno> {
    spec::spec_inactive(ap.a_vp)
}

/// Vnode op for write.
pub fn spec_write(ap: &mut VopWriteArgs<'_, '_>) -> Result<(), Errno> {
    spec::spec_write(ap.a_vp, ap.a_uio, ap.a_ioflag)
}

/// Device ioctl operation.
pub fn spec_ioctl(ap: &mut VopIoctlArgs<'_>) -> Result<(), Errno> {
    spec::spec_ioctl(ap.a_vp, ap.a_command, ap.a_data, ap.a_fflag, ap.a_p)
}

/// `spec_kqfilter`: the driver's `d_kqfilter`, or `seltrue_kqfilter` for poll/select.
pub fn spec_kqfilter(ap: &mut VopKqfilterArgs<'_>) -> Result<(), Errno> {
    spec::spec_kqfilter(ap.a_vp, ap.a_kn)
}

/// Synch buffers associated with a block device.
pub fn spec_fsync(ap: &mut VopFsyncArgs<'_>) -> Result<(), Errno> {
    spec::spec_fsync(ap.a_vp, ap.a_waitfor)
}

/// `spec_strategy`: `bdevsw[major(bp->b_dev)].d_strategy(bp)`, the lent buffer passed on as
/// it came.
pub fn spec_strategy(ap: &mut VopStrategyArgs) -> Result<(), Errno> {
    spec::spec_strategy::<BsdProcGlue>(IoBuf::Lent(ap.a_bp))
}

/// Device close routine.
pub fn spec_close(ap: &mut VopCloseArgs<'_>) -> Result<(), Errno> {
    spec::spec_close(ap.a_vp, ap.a_fflag, Cred::from_raw(ap.a_cred), ap.a_p)
}

/// `spec_getattr`: a clone reads its parent's attributes.
pub fn spec_getattr(ap: &mut VopGetattrArgs<'_>) -> Result<(), Errno> {
    spec::spec_getattr(ap.a_vp, ap.a_vap, Cred::from_raw(ap.a_cred), ap.a_p)
}

/// `spec_setattr`: a clone sets its parent's attributes.
pub fn spec_setattr(ap: &mut VopSetattrArgs<'_>) -> Result<(), Errno> {
    spec::spec_setattr(ap.a_vp, ap.a_vap, Cred::from_raw(ap.a_cred), ap.a_p)
}

/// `spec_access`: a clone answers with its parent's permissions.
pub fn spec_access(ap: &mut VopAccessArgs<'_>) -> Result<(), Errno> {
    spec::spec_access(ap.a_vp, ap.a_mode, Cred::from_raw(ap.a_cred), ap.a_p)
}

/// Print out the contents of a special device vnode.
pub fn spec_print(ap: &mut VopPrintArgs) -> Result<(), Errno> {
    spec::spec_print(ap.a_vp)
}

/// Return POSIX pathconf information applicable to special devices.
pub fn spec_pathconf(ap: &mut VopPathconfArgs<'_>) -> Result<(), Errno> {
    spec::spec_pathconf(ap.a_name, ap.a_retval)
}

/// Special device advisory byte-level locks.
pub fn spec_advlock(ap: &mut VopAdvlockArgs<'_>) -> Result<(), Errno> {
    spec::spec_advlock(ap.a_vp, ap.a_id, ap.a_op, ap.a_fl, ap.a_flags)
}

/// `bsd`'s device switch, `machine::conf`'s tables, as the kernel's special files call it
/// (bridges.toml `Devsw`; registered by `hal_bridges_register`).
pub struct BsdDevsw;

/// The one [`BsdDevsw`].
pub static BSD_DEVSW: BsdDevsw = BsdDevsw;

impl Devsw<BsdProcGlue> for BsdDevsw {
    fn nchrdev(&self) -> u32 {
        nchrdev()
    }

    fn nblkdev(&self) -> u32 {
        nblkdev()
    }

    fn cdev_type(&self, maj: u32) -> u32 {
        cdevsw(maj).d_type
    }

    fn cdev_flags(&self, maj: u32) -> u32 {
        cdevsw(maj).d_flags
    }

    fn bdev_type(&self, maj: u32) -> u32 {
        bdevsw(maj).d_type
    }

    fn cdev_open(&self, dev: Dev, oflags: i32, devtype: i32, p: &Proc) -> Result<(), Errno> {
        (cdevsw(major(dev)).d_open)(dev, oflags, devtype, p)
    }

    fn cdev_close(
        &self,
        dev: Dev,
        fflag: i32,
        devtype: i32,
        p: Option<&Proc>,
    ) -> Result<(), Errno> {
        (cdevsw(major(dev)).d_close)(dev, fflag, devtype, p)
    }

    fn cdev_read(&self, dev: Dev, uio: &mut Uio<'_>, ioflag: i32) -> Result<(), Errno> {
        (cdevsw(major(dev)).d_read)(dev, uio, ioflag)
    }

    fn cdev_write(&self, dev: Dev, uio: &mut Uio<'_>, ioflag: i32) -> Result<(), Errno> {
        (cdevsw(major(dev)).d_write)(dev, uio, ioflag)
    }

    fn cdev_ioctl(
        &self,
        dev: Dev,
        cmd: u64,
        data: &mut [u8],
        fflag: i32,
        p: &Proc,
    ) -> Result<(), Errno> {
        (cdevsw(major(dev)).d_ioctl)(dev, cmd, data, fflag, p)
    }

    fn cdev_kqfilter(&self, dev: Dev, kn: &Knote) -> Option<Result<(), Errno>> {
        cdevsw(major(dev)).d_kqfilter.map(|f| f(dev, kn))
    }

    fn bdev_open(&self, dev: Dev, oflags: i32, devtype: i32, p: &Proc) -> Result<(), Errno> {
        (bdevsw(major(dev)).d_open)(dev, oflags, devtype, p)
    }

    fn bdev_close(
        &self,
        dev: Dev,
        fflag: i32,
        devtype: i32,
        p: Option<&Proc>,
    ) -> Result<(), Errno> {
        (bdevsw(major(dev)).d_close)(dev, fflag, devtype, p)
    }

    fn bdev_ioctl(
        &self,
        dev: Dev,
        cmd: u64,
        data: &mut [u8],
        fflag: i32,
        p: &Proc,
    ) -> Result<(), Errno> {
        (bdevsw(major(dev)).d_ioctl)(dev, cmd, data, fflag, p)
    }

    fn bdev_strategy(&self, bp: IoBuf) {
        // A driver's strategy takes the C's `struct buf *`: a lent buffer as it came, a token
        // lent (parked in its buffer) for the driver's `biodone` to take back.
        let bp = match bp {
            IoBuf::Lent(bp) => bp,
            IoBuf::Token(t) => lend(t),
        };
        (bdevsw(major(bp.dev())).d_strategy)(bp);
    }

    fn chrtoblk(&self, dev: Dev) -> Dev {
        crate::kern::subr_xxx::chrtoblk(dev)
    }

    fn iskmemdev(&self, dev: Dev) -> bool {
        iskmemdev(dev)
    }

    fn iszerodev(&self, dev: Dev) -> bool {
        iszerodev(dev)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::{assert, assert_eq};

    use super::*;
    use crate::machine::conf::{chrtoblktbl, mem_no};
    use crate::sys::types::makedev;
    use crate::sys::unistd::{_PC_CHOWN_RESTRICTED, _PC_LINK_MAX, _PC_NAME_MAX};
    use crate::sys::vnode::{VCHR, VopsId, cdevvp, vrele};

    #[test]
    fn the_device_switch_is_machine_conf_s() {
        let sw: &dyn Devsw<BsdProcGlue> = &BSD_DEVSW;
        assert_eq!(sw.nchrdev(), nchrdev());
        assert_eq!(sw.nblkdev(), nblkdev());
        for maj in 0..nchrdev() {
            assert_eq!(sw.cdev_type(maj), cdevsw(maj).d_type);
            assert_eq!(sw.cdev_flags(maj), cdevsw(maj).d_flags);
        }
        for maj in 0..nblkdev() {
            assert_eq!(sw.bdev_type(maj), bdevsw(maj).d_type);
        }
        let mem = mem_no();
        assert!(sw.iskmemdev(makedev(mem, 0)) && sw.iskmemdev(makedev(mem, 1)));
        assert!(!sw.iskmemdev(makedev(mem, 2)));
        assert!(sw.iszerodev(makedev(mem, 12)) && !sw.iszerodev(makedev(mem, 2)));
        for (maj, _) in chrtoblktbl().iter().enumerate() {
            let dev = makedev(maj as u32, 3);
            assert_eq!(sw.chrtoblk(dev), crate::kern::subr_xxx::chrtoblk(dev));
        }
    }

    #[test]
    fn the_spec_shims_answer_as_the_kernel_s_special_files() {
        let (_g, _p) = crate::kern::vfs_subr::tests::setup();
        // A device vnode, the kernel's spec_vops.
        let vp = cdevvp(makedev(2, 2)).unwrap().unwrap();
        assert_eq!(vp.ops_id(), VopsId::SPEC);
        assert_eq!(vp.vtype(), VCHR);
        for (name, want) in [(_PC_LINK_MAX, Ok(())), (_PC_CHOWN_RESTRICTED, Ok(()))] {
            let mut rv = 0;
            assert_eq!(
                spec_pathconf(&mut VopPathconfArgs {
                    a_vp: vp,
                    a_name: name,
                    a_retval: &mut rv,
                }),
                want
            );
            let mut krv = 0;
            assert_eq!(spec::spec_pathconf(name, &mut krv), want);
            assert_eq!(rv, krv);
        }
        let mut rv = 0;
        assert_eq!(
            spec_pathconf(&mut VopPathconfArgs {
                a_vp: vp,
                a_name: _PC_NAME_MAX,
                a_retval: &mut rv,
            }),
            Err(Errno::EINVAL)
        );
        assert_eq!(spec_print(&mut VopPrintArgs { a_vp: vp }), Ok(()));
        // A character device's fsync has nothing to flush.
        let p = crate::kern::proc_glue::test_thread(crate::kern::proc_glue::test_process(1));
        assert_eq!(
            spec_fsync(&mut VopFsyncArgs {
                a_vp: vp,
                a_cred: crate::sys::ucred::NOCRED,
                a_waitfor: 0,
                a_p: p,
            }),
            Ok(())
        );
        let _ = vrele(vp);
    }
}
/* </TESTS> */
