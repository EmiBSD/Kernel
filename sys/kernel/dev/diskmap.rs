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
 * Copyright (c) 2009, 2010 Joel Sing <jsing@openbsd.org>
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
/* </LICENSES> */

/* <CODE> */
//! diskmap(4): the disk mapper, `/dev/diskmap`. opendev(3) opens it and hands it a disk's
//! name or its disklabel UID (`DIOCMAP`); the device swaps the caller's descriptor for one
//! open on the disk's real device node.
//!
//! Upstream: sys/dev/diskmap.c @ 3ce1f3f79392
//! LZ: sys/dev/diskmap.rs@e8249449d905
//!
//! `cdevsw[90]` of amd64 and arm64 (`cdev_disk_init(1,diskmap)`): the entry points are the
//! cdevsw signatures over the glue, as fuse(4)'s. The DUID lookup is `disk_map`
//! (`dev::disk`), which `mount(2)` uses too.
//!
//! ## Deviations
//! - `disk_map` writes the mapped name into a second `PATH_MAX` buffer (both `malloc`'d,
//!   `M_DEVBUF`), where the C maps in place.
//!
//! ## Redesign
//! - N7.1d: LZ's module was `bsd`'s, over its descriptor shims (`kern_descrip`'s `fnew`,
//!   `fdinsert`, `closef`, `vfs_syscalls`' `getvnode`, `namei`'s `ndinit`, `vn_open`) and a
//!   hand-read `struct dk_diskmap`; it is the kernel's, over the kernel's own (`file::
//!   descrip`, `fs::{syscalls, lookup, vnops}`), generic over the glue, the argument an
//!   `AbiPod` read (`sysdefs::dev::dkio::DkDiskmap`), the user's name through `UserPtr`'s
//!   `copyinstr`/`copyoutstr`. `ni_pledge = PLEDGE_RPATH` and `ni_unveil = UNVEIL_READ` are
//!   kept: the device node is opened as a path the caller may read.

use core::sync::atomic::Ordering;

use hal::mem::malloc::{M_WAITOK, MArray};
use hal::user::{UserPtr, pod_from_bytes};
use sysdefs::base::syslimits::PATH_MAX;
use sysdefs::base::types::Dev;
use sysdefs::dev::dkio::{DIOCMAP, DkDiskmap};
use sysdefs::proc::pledge::PLEDGE_RPATH;

use crate::assert::{kassert, kpanic};
use crate::dev::disk::disk_map;
use crate::file::descrip::{closef, fdinsert, fdplock, fdpunlock};
use crate::file::uio::Uio;
use crate::file::{DTYPE_VNODE, FileGlue, FileRef, fnew};
use crate::fs::lookup::ndinit;
use crate::fs::namei::{NiDirp, UNVEIL_READ};
use crate::fs::syscalls::getvnode;
use crate::fs::vnops::{vn_close, vn_open};
use crate::fs::vops::{Cred, vop_unlock};
use crate::proc::{ProcGlue, Thread};
use sysdefs::base::errno::Errno;

/// `M_DEVBUF` (`<sys/malloc.h>`): the device name buffers.
const M_DEVBUF: i32 = 2;

/// `*(struct dk_diskmap *)addr`, read from the ioctl's argument buffer (zeros past a short
/// one).
fn dk_diskmap_load(addr: &[u8]) -> DkDiskmap {
    let mut b = [0u8; size_of::<DkDiskmap>()];
    let n = addr.len().min(b.len());
    b[..n].copy_from_slice(&addr[..n]);
    pod_from_bytes(&b).unwrap_or(DkDiskmap {
        device: 0,
        fd: 0,
        flags: 0,
    })
}

/// A `PATH_MAX` buffer of `malloc(M_DEVBUF, M_WAITOK)`.
fn path_buf() -> MArray<u8> {
    match MArray::new(M_DEVBUF, M_WAITOK, PATH_MAX, |_| 0u8) {
        Some(b) => b,
        None => kpanic(format_args!("diskmapioctl: malloc(M_WAITOK) failed")),
    }
}

/// `diskmapopen`: nothing to do.
pub fn diskmapopen<G: ProcGlue>(
    _dev: Dev,
    _flag: i32,
    _fmt: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    Ok(())
}

/// `diskmapclose`: nothing to do.
pub fn diskmapclose<G: ProcGlue>(
    _dev: Dev,
    _flag: i32,
    _fmt: i32,
    _p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    Ok(())
}

/// `diskmapioctl`: `DIOCMAP` maps the disk name or disklabel UID at `dm->device` to its
/// device node (writing the mapped name back), opens that node with the flags of the file
/// at `dm->fd` and puts the new file in that descriptor's place.
pub fn diskmapioctl<P: ProcGlue<FileGlue = G>, G: FileGlue<Proc = P>>(
    _dev: Dev,
    cmd: u64,
    addr: &mut [u8],
    _flag: i32,
    p: &Thread<P>,
) -> Result<(), Errno> {
    if cmd != DIOCMAP {
        return Err(Errno::ENOTTY);
    }

    // Map a request for a disk to the correct device. We should be supplied with either a
    // diskname or a disklabel UID.
    let dm = dk_diskmap_load(addr);
    let fd = dm.fd;
    let mut devname = path_buf();
    UserPtr::<u8>::new(dm.device).copyinstr(&mut devname)?;
    let mut mapped = path_buf();
    if disk_map(&devname, &mut mapped, dm.flags) {
        devname.copy_from_slice(&mapped);
        let len = devname.iter().position(|&c| c == 0).unwrap_or(PATH_MAX - 1);
        UserPtr::<u8>::new(dm.device).copyoutstr(&devname[..=len])?;
    }

    // Attempt to open actual device.
    let fp0 = getvnode(p, fd)?;
    let fflag = fp0.f_flag.load(Ordering::SeqCst);

    let mut nd = ndinit(0, 0, NiDirp::Sys(&devname), p);
    nd.ni_pledge = PLEDGE_RPATH;
    nd.ni_unveil = UNVEIL_READ;
    if let Err(error) = vn_open(&mut nd, fflag as i32, 0) {
        let _ = fp0.frele(Some(p));
        return Err(error);
    }
    let Some(vp) = nd.ni_vp else {
        let _ = fp0.frele(Some(p));
        return Err(Errno::ENOENT);
    };
    let _ = vop_unlock(vp);

    let fdp = p.fd();
    let mut g = fdplock(fdp);

    // Stop here if the 'struct file *' has been replaced, for example by another thread
    // calling dup2(2), while this thread was sleeping in vn_open(). Note that this would
    // not happen for correct usages of "/dev/diskmap".
    let replaced = !g.ofile(fd).is_some_and(|f| FileRef::ptr_eq(&f, &fp0));
    let error = if replaced {
        Some(Errno::EAGAIN)
    } else {
        match fnew(p) {
            None => Some(Errno::ENFILE),
            Some(fp) => {
                // Zap old file: the slot's reference comes out, the descriptor stays
                // allocated.
                let (old, flags) = g.take_ofile(fd);
                kassert!(old.as_ref().is_some_and(|f| FileRef::ptr_eq(f, &fp0)));

                // Insert new file.
                fp.f_flag.store(fflag, Ordering::SeqCst);
                let (ops, data) = G::vnode_file(vp);
                fp.install(DTYPE_VNODE, ops, data);
                fdinsert(&mut g, fd, flags, &fp);
                fdpunlock(g);
                drop(fp);

                // The slot's reference and getvnode's (the C's closef of a count of 2).
                drop(old);
                let _ = closef(fp0, Some(p));
                return Ok(());
            }
        }
    };

    // bad:
    fdpunlock(g);
    let _ = vn_close(vp, fflag as i32, Cred::real(p.ucred()), Some(p));
    let _ = fp0.frele(Some(p));
    Err(error.unwrap_or(Errno::EAGAIN))
}

/// `diskmapread`: the mapper has no data.
pub fn diskmapread<G: ProcGlue>(_dev: Dev, _uio: &mut Uio<'_, G>, _flag: i32) -> Result<(), Errno> {
    Err(Errno::ENXIO)
}

/// `diskmapwrite`: the mapper takes no data.
pub fn diskmapwrite<G: ProcGlue>(
    _dev: Dev,
    _uio: &mut Uio<'_, G>,
    _flag: i32,
) -> Result<(), Errno> {
    Err(Errno::ENXIO)
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn the_argument_is_read_at_the_c_offsets() {
        let mut b = [0u8; size_of::<DkDiskmap>()];
        b[..8].copy_from_slice(&0x1234_5678usize.to_ne_bytes()[..8]);
        b[8..12].copy_from_slice(&7i32.to_ne_bytes());
        b[12..16].copy_from_slice(&3i32.to_ne_bytes());
        let dm = dk_diskmap_load(&b);
        assert_eq!(dm.device, 0x1234_5678);
        assert_eq!((dm.fd, dm.flags), (7, 3));
        // A short argument reads zeros past its end.
        let dm = dk_diskmap_load(&b[..8]);
        assert_eq!((dm.device, dm.fd, dm.flags), (0x1234_5678, 0, 0));
    }

    #[test]
    fn the_argument_has_the_c_size() {
        assert_eq!(size_of::<DkDiskmap>(), 16);
    }

    #[test]
    fn only_diocmap_is_answered() {
        let (_g, p) = crate::fs::testfs::test_thread();
        let mut b = [0u8; 16];
        assert_eq!(
            diskmapioctl(0, DIOCMAP + 1, &mut b, 0, p),
            Err(Errno::ENOTTY)
        );
        assert_eq!(diskmapopen(0, 0, 0, p), Ok(()));
        assert_eq!(diskmapclose(0, 0, 0, Some(p)), Ok(()));
    }
}
/* </TESTS> */
