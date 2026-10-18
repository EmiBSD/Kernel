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
 *
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
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed by Charles D. Cranor and
 *      Washington University.
 * 4. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
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
 * from: Id: uvm_vnode.h,v 1.1.2.4 1997/10/03 21:18:24 chuck Exp
 */
/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993
 *      The Regents of the University of California.
 * Copyright (c) 1990 University of Utah.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
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
 *      @(#)vnode_pager.c       8.8 (Berkeley) 2/13/94
 * from: Id: uvm_vnode.c,v 1.1.2.26 1998/02/02 20:38:07 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_vnode.c`: the vnode pager, the `uvm_object` behind a mapped file (`struct
//! uvm_vnode`, `<uvm/uvm_vnode.h>`): `bsd`'s adapter to the kernel's vnode pager,
//! `kernel::vm::vnode` (N3.4e).
//!
//! Upstream: sys/uvm/uvm_vnode.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_vnode.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_vnode.rs@3c62ede68802
//!
//! The pager is the kernel's (`sys/kernel/vm/vnode.rs`): the `uvm_vnode` pool, the write list
//! and the sync queue, `uvn_attach`, `uvn_io` and every function of `uvm_vnode.c`. It reaches
//! the vnode through the `VnodeOps` bridge, which `bsd`'s [`Vnode`] implements here (one call
//! each into the vnode layer), and the clock and the shutdown state through `SysInfo`
//! ([`uvn_install_sysinfo`], from `uvm_init`). The functions the file systems call
//! (`uvn_attach`, `uvm_vnp_terminate`, `uvm_vnp_uncache`, `uvm_vnp_setsize`, `uvm_vnp_sync`)
//! keep their `bsd` signatures and forward.
//!
//! ## Deviations
//! - `uvn_attach` of a block device needs the device switch (`bdevsw[].d_type`, `DIOCGPART`
//!   through `d_ioctl`, `conf.c`): reported by the kernel's pager, and the attach fails (NULL)
//!   as for a device that is not a disk (LZ's).
//! - `uvm_vnp_sync(NULL)` is `uvm_vnp_sync(None)`; `uvm_vnp_uncache` returns `bool` (LZ's).
//! - `UVM_PAGE_OWN` is not configured; the `DEBUG` printfs are compiled out as in C (LZ's).
//!
//! ## Redesign
//! - N3.4e: LZ's module is the whole pager (`UvmVnode` a `#[repr(C)]` pool item reached from
//!   its first member by a cast, the I/O window a raw address, the queues intrusive lists of
//!   raw links). It moved into the kernel crate (`sys/kernel/vm/vnode.rs`, `## Redesign`);
//!   this file holds no `unsafe` and is `forbid`.
//! - The vnode as the pager sees it: `impl VnodeOps for Vnode`. `VOP_READ`/`VOP_WRITE` take
//!   the window's bytes as a slice; the `UIO_SYSSPACE` uio over it, `curproc`'s credentials
//!   and the residual count are built here, as LZ's `uvn_io` built them.
//! - A vnode's mount is compared by identity (`VnodeOps::mount_id`, its address as a number):
//!   `uvm_vnp_sync(mp)` passes `mp`'s.

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::Ordering;

pub use kernel::vm::vnode::*;

use crate::kern::kern_tc::getmicrouptime;
use crate::kern::kern_xxx::REBOOTING;
use crate::kern::subr_prf::panic;
use crate::kern::vfs_subr::{vget, vref, vrele};
use crate::kern::vfs_vnops::vn_lock;
use crate::kern::vfs_vops::{VOP_GETATTR, VOP_READ, VOP_UNLOCK, VOP_WRITE};
use crate::machine::conf::iszerodev;
use crate::machine::cpu::curproc;
use crate::sys::errno::Errno;
use crate::sys::mount::Mount;
use crate::sys::uio::{Iovec, Uio, UioRw, UioSeg};
use crate::sys::vnode::{VBLK, VTEXT, Vattr, Vnode, Vtype};
use crate::uvm::uvm_extern::{VmProt, Voff};
use crate::uvm::uvm_object::ObjRef;

/// `VOP_READ`/`VOP_WRITE` of `buf` at `off` (`UIO_SYSSPACE`, `curproc`'s credentials): the
/// bytes moved (`len - uio_resid`).
fn vop_rw(
    vp: &'static Vnode,
    rw: UioRw,
    off: Voff,
    base: *mut c_void,
    len: usize,
    ioflag: i32,
) -> Result<usize, Errno> {
    let mut iov = [Iovec {
        iov_base: base,
        iov_len: len,
    }];
    let p = curproc();
    let mut uio = Uio {
        uio_iov: &mut iov,
        uio_offset: off,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: rw,
        uio_resid: len,
        uio_procp: p,
    };
    let cred = p.map_or(ptr::null(), |p| p.p_ucred.get());
    if rw == UioRw::UIO_READ {
        VOP_READ(vp, &mut uio, ioflag, cred)?;
    } else {
        VOP_WRITE(vp, &mut uio, ioflag, cred)?;
    }
    Ok(len - uio.uio_resid)
}

// The vnode as the vnode pager reaches it (bridges.toml `VnodeOps`): one call each into the
// vnode layer. Vnodes are never freed, so the pager keeps them as `&'static`.
impl VnodeOps for Vnode {
    fn uvn(&self) -> Option<&'static Uvn> {
        self.v_uvm.get()
    }

    fn set_uvn(&self, uvn: &'static Uvn) {
        self.v_uvm.set(Some(uvn));
    }

    fn attr_size(&'static self) -> Result<u64, Errno> {
        let Some(p) = curproc() else {
            panic(format_args!("uvn_attach: no curproc"));
        };
        let mut vattr = Vattr::new();
        VOP_GETATTR(self, &mut vattr, p.p_ucred.get(), p)?;
        Ok(vattr.va_size)
    }

    fn is_blk(&self) -> bool {
        self.v_type.get() == VBLK
    }

    fn vtext(&self) -> bool {
        self.v_flag.get() & VTEXT != 0
    }

    fn clear_vtext(&self) {
        self.v_flag.set(self.v_flag.get() & !VTEXT);
    }

    fn mount_id(&self) -> usize {
        mount_id(self.v_mount.get())
    }

    fn vref(&'static self) {
        vref(self);
    }

    fn vrele(&'static self) {
        let _ = vrele(self);
    }

    fn vget_nowait(&'static self) -> Result<(), Errno> {
        vget(self, crate::sys::lock::LK_NOWAIT)
    }

    fn lock(&'static self, flags: i32) -> Result<(), Errno> {
        vn_lock(self, flags)
    }

    fn unlock(&'static self) {
        let _ = VOP_UNLOCK(self);
    }

    fn read(&'static self, off: Voff, buf: &mut [u8], ioflag: i32) -> Result<usize, Errno> {
        vop_rw(
            self,
            UioRw::UIO_READ,
            off,
            buf.as_mut_ptr().cast(),
            buf.len(),
            ioflag,
        )
    }

    fn write(&'static self, off: Voff, buf: &[u8], ioflag: i32) -> Result<usize, Errno> {
        // UIO_WRITE only reads through the iovec (uiomove copies out of it).
        vop_rw(
            self,
            UioRw::UIO_WRITE,
            off,
            buf.as_ptr().cast_mut().cast(),
            buf.len(),
            ioflag,
        )
    }

    fn vtype(&self) -> VType {
        match self.v_type.get() {
            Vtype::VNON => VType::VNON,
            Vtype::VREG => VType::VREG,
            Vtype::VDIR => VType::VDIR,
            Vtype::VBLK => VType::VBLK,
            Vtype::VCHR => VType::VCHR,
            Vtype::VLNK => VType::VLNK,
            Vtype::VSOCK => VType::VSOCK,
            Vtype::VFIFO => VType::VFIFO,
            Vtype::VBAD => VType::VBAD,
        }
    }

    fn is_zero_device(&self) -> bool {
        iszerodev(self.v_rdev())
    }

    fn attr_flags(&'static self) -> Result<u64, Errno> {
        let Some(p) = curproc() else {
            panic(format_args!("mmap: no curproc"));
        };
        let mut vattr = Vattr::new();
        VOP_GETATTR(self, &mut vattr, p.p_ucred.get(), p)?;
        Ok(vattr.va_flags)
    }
}

/// A mount's identity for the pager (its address as a number, 0 for none).
fn mount_id(mp: Option<&'static Mount>) -> usize {
    mp.map_or(0, |m| ptr::from_ref(m).addr())
}

/// The system's clock and shutdown state for the vnode pager (bridges.toml `SysInfo`).
struct BsdSysInfo;

impl SysInfo for BsdSysInfo {
    fn uptime(&self) -> (i64, i64) {
        let tv = getmicrouptime();
        (tv.tv_sec, tv.tv_usec as i64)
    }

    fn rebooting(&self) -> bool {
        REBOOTING.load(Ordering::Relaxed)
    }

    fn rebooting_chan(&self) -> *const () {
        ptr::from_ref(&REBOOTING).cast()
    }
}

static BSD_SYS_INFO: BsdSysInfo = BsdSysInfo;

/// Installs the `SysInfo` bridge for the kernel's vnode pager (`uvm_init`; a second call keeps
/// the first).
pub fn uvn_install_sysinfo() {
    let _ = install_sysinfo(&BSD_SYS_INFO);
}

/// uvn_attach
///
/// attach a vnode structure to a VM object. if the vnode is already attached, then just bump
/// the reference count by one and return the VM object. if not already attached, attach and
/// return the new VM obj. the "accessprot" tells the max access the attaching thread wants to
/// our pages.
///
/// => in fact, nothing should be locked so that we can sleep here.
pub fn uvn_attach(vp: &'static Vnode, accessprot: VmProt) -> Option<ObjRef> {
    kernel::vm::vnode::uvn_attach(vp, accessprot)
}

/// uvm_vnp_terminate: external hook to clear out a vnode's VM
///
/// called in two cases: \[1\] when a persisting vnode vm object (i.e. one with a zero
/// reference count) needs to be freed so that a vnode can be reused, \[2\] at "umount -f" and
/// during a "reboot/halt" operation.
///
/// => the caller must XLOCK and VOP_LOCK the vnode before calling us
pub fn uvm_vnp_terminate(vp: &'static Vnode) {
    kernel::vm::vnode::uvm_vnp_terminate(vp);
}

/// uvm_vnp_uncache: disable "persisting" in a vnode... when last reference is gone we will
/// kill the object (flushing dirty pages back to the vnode if needed).
///
/// => returns TRUE if there was no uvm_object attached or if there was one and we killed it
///    [i.e. if there is no active uvn]
/// => called with the vnode VOP_LOCK'd [we will unlock it for I/O, if needed]
pub fn uvm_vnp_uncache(vp: &'static Vnode) -> bool {
    kernel::vm::vnode::uvm_vnp_uncache(vp)
}

/// uvm_vnp_setsize: grow or shrink a vnode uvn
///
/// grow   => just update size value
/// shrink => toss un-needed pages
///
/// => we assume that the caller has a reference of some sort to the vnode in question so that
///    it will not be yanked out from under us.
pub fn uvm_vnp_setsize(vp: &'static Vnode, newsize: Voff) {
    kernel::vm::vnode::uvm_vnp_setsize(vp, newsize);
}

/// uvm_vnp_sync: flush all dirty VM pages back to their backing vnodes.
///
/// => called from sys_sync with no VM structures locked
/// => only one process can do a sync at a time (the sync queue's lock).
pub fn uvm_vnp_sync(mp: Option<&'static Mount>) {
    kernel::vm::vnode::uvm_vnp_sync(mp.map(|m| mount_id(Some(m))));
}
/* </CODE> */
