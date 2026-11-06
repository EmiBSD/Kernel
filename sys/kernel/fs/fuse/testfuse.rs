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
/* </LICENSES> */

/* <CODE> */
//! The host tests' fixture of the kernel's fuse (N5.9f1): the set-up (the test thread, the
//! `"fmsg"` pool and the inode hash, `fusefs_init`'s work until N5.9f2 brings it), the
//! simulated daemon `fb_queue` runs (LZ's `TEST_DAEMON`, moved) and the daemon's side of the
//! device (LZ's `fuse_device` tests' helpers, moved: `read_req`, `reply`, `dev_read`,
//! `dev_write`), a mount structure over a mount that is not a real one, and a small vnode
//! table for nodes on the inode hash (the node's lock, `fusefs_reclaim`'s hash removal) until
//! `fuse_vnops` lands. Host tests and benches only (`cfg(test)` or `doubles`).

use core::cell::Cell;
use core::sync::atomic::Ordering;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::intr::IPL_NONE;
use hal::malloc::{M_WAITOK, MArc};
use hal::pool::PR_WAITOK;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::types::{Dev, Ino, makedev};
use hal::user::{AbiPod, bytes_of, bytes_of_mut, pod_from_bytes};

use crate::fs::fuse::fuse_device::{fuse_device_set_fmp, fuseopen, fuseread, fusewrite};
use crate::fs::fuse::fuse_ihash::{fuse_ihashinit, fuse_ihashrem};
use crate::fs::fuse::fusebuf_h::{
    FUSE_FLUSH, FUSE_FORGET, FUSE_FSYNC, FUSE_GETATTR, FUSE_INIT, FUSE_LINK, FUSE_MKDIR,
    FUSE_MKNOD, FUSE_OPEN, FUSE_OPENDIR, FUSE_READ, FUSE_READDIR, FUSE_RELEASE, FUSE_RELEASEDIR,
    FUSE_RENAME, FUSE_SETATTR, FUSE_WRITE, FuseFlushIn, FuseForgetIn, FuseFsyncIn, FuseGetattrIn,
    FuseInHeader, FuseInitIn, FuseLinkIn, FuseMkdirIn, FuseMknodIn, FuseOpenIn, FuseOutHeader,
    FuseReadIn, FuseReleaseIn, FuseRenameIn, FuseSetattrIn, FuseWriteIn,
};
use crate::fs::fuse::fusefs::{FusefsMnt, FusefsMntRef, M_FUSEFS};
use crate::fs::fuse::fusefs_node::{FusefsNode, FusefsNodeRef, vtoi};
use crate::fs::mount::vfs_mount_alloc;
use crate::fs::subr::{getnewvnode, vfs};
use crate::fs::testfs::{TEST_CONF, TestGlue};
use crate::fs::vnode::{IO_NDELAY, LK_RWFLAGS, VT_FUSEFS, Vnode};
use crate::fs::vops::{Vnops, vop_unlock};
use crate::proc::Thread;
use crate::uio::{Iovec, Uio, UioRw, UioSeg};

/// fuse(4)'s character major (`cdevsw[92]`).
pub const FUSE_MAJOR: u32 = 92;

/// `PENDING` (`fuse_vfsops.c`): `FUSE_INIT` reply not yet received, the `sess_init` a mount
/// starts with (N5.9f2's `fuse_vfsops` defines it for the kernel).
pub const PENDING: i32 = 2;

std::thread_local! {
    /// The simulated daemon: called by `fb_queue` with the device once the fusebuf is queued
    /// and the requester is on its sleep queue (`fusebuf`'s deviations).
    static TEST_DAEMON: Cell<Option<fn(Dev)>> = const { Cell::new(None) };
}

/// The simulated daemon `fb_queue` runs, if one is set.
pub fn test_daemon() -> Option<fn(Dev)> {
    TEST_DAEMON.with(Cell::get)
}

/// Sets (or clears) the simulated daemon.
pub fn set_daemon(d: Option<fn(Dev)>) {
    TEST_DAEMON.with(|c| c.set(d));
}

/// The test thread over testfs's root with its tables made afresh (`vfs_test_reset`, which
/// forgets fuse's hash and counters), the `"fmsg"` pool and the inode hash (`fusefs_init`'s
/// work), the kernel uios' copies on, no simulated daemon: the memory guard and the thread.
pub fn setup() -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = crate::fs::testfs::test_thread();
    // The device's kernel uios (`dev_read`, `dev_write`) are copied by the uiomove double.
    crate::testutil::set_copy_kernel(true);
    match vfs::<TestGlue>()
        .fuse
        .fbuf_pool
        .init_recorded(IPL_NONE, PR_WAITOK, "fmsg")
    {
        Ok(()) | Err(Errno::EBUSY) => {}
        Err(e) => std::panic!("testfuse: the fmsg pool: {e:?}"),
    }
    crate::fs::fuse::fuse_ihash::fuse_ihash_test_reset::<TestGlue>();
    fuse_ihashinit::<TestGlue>();
    set_daemon(None);
    (g, p)
}

/// `/dev/fuse<unit>`.
pub fn fusedev(unit: u32) -> Dev {
    makedev(FUSE_MAJOR, unit)
}

/// `fuseopen` of `/dev/fuse<unit>` for reading and writing.
pub fn open_dev(p: &Thread<TestGlue>, unit: u32) -> Dev {
    let dev = fusedev(unit);
    if let Err(e) = fuseopen(dev, FREAD | FWRITE, 0, p) {
        std::panic!("fuseopen: {e:?}");
    }
    dev
}

/// A mount structure for the device as `fusefs_mount` makes it (`sess_init` `PENDING`), on a
/// mount of testfs's configuration that is not a real one (the device reads `max_read`,
/// `max_write` and `sess_init`, the hash the structure's address).
pub fn fake_fmp(dev: Dev, max_read: i32) -> FusefsMntRef<TestGlue> {
    let mp = vfs_mount_alloc(None, &TEST_CONF[0]);
    let fmp = FusefsMnt {
        mp,
        undef_op: crate::fs::word::U32Word::new(0),
        max_read,
        max_write: crate::fs::word::I32Word::new(4096),
        sess_init: crate::fs::word::I32Word::new(PENDING),
        allow_other: 0,
        dev,
    };
    match MArc::new(M_FUSEFS, M_WAITOK, fmp) {
        Some(fmp) => fmp,
        None => std::panic!("testfuse: no fusefs_mnt"),
    }
}

/// [`fake_fmp`] attached to its device (`fuse_device_set_fmp(fmp, 1)`).
pub fn mount_fmp(dev: Dev, max_read: i32) -> FusefsMntRef<TestGlue> {
    let fmp = fake_fmp(dev, max_read);
    fuse_device_set_fmp(&fmp, true);
    fmp
}

/// The fixture's vnode table: the node's lock (`i_lock`), an inactive that unlocks, a reclaim
/// that takes the node off the hash and out of the vnode's slot (`fusefs_reclaim`'s part the
/// hash needs).
pub struct TestFuseVops;

impl Vnops<TestGlue> for TestFuseVops {
    fn lock(&self, vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        vtoi(vp).i_lock.enter(flags & LK_RWFLAGS)
    }
    fn unlock(&self, vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        vtoi(vp).i_lock.exit();
        Ok(())
    }
    fn islocked(&self, vp: &'static Vnode<TestGlue>) -> i32 {
        vtoi(vp).i_lock.status()
    }
    fn inactive(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        vop_unlock(vp)
    }
    fn reclaim(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        fuse_ihashrem(&vtoi(vp));
        drop(vp.take_node());
        Ok(())
    }
}

/// The fixture's one vnode table.
pub static TEST_FUSE_VOPS: TestFuseVops = TestFuseVops;

/// A vnode of the fixture's table and its node for inode `ino` of `fmp`, unhashed, as
/// `fusefs_vget` makes them before `fuse_ihashins`.
pub fn node(fmp: &FusefsMntRef<TestGlue>, ino: Ino) -> FusefsNodeRef<TestGlue> {
    let vp = match getnewvnode(VT_FUSEFS, Some(fmp.mp), &TEST_FUSE_VOPS) {
        Ok(vp) => vp,
        Err(e) => std::panic!("getnewvnode: {e:?}"),
    };
    let Some(ip) = MArc::new(M_FUSEFS, M_WAITOK, FusefsNode::new(vp, fmp.clone(), ino)) else {
        std::panic!("testfuse: no node");
    };
    vp.set_node(MArc::erase(ip.clone()));
    ip
}

/// `read(fd, buf, len)` on the device: the bytes read.
pub fn dev_read(dev: Dev, buf: &mut [u8], ioflag: i32) -> Result<usize, Errno> {
    let len = buf.len();
    let mut iov = [Iovec::default()];
    let mut uio = Uio::<TestGlue>::kernel_read(&mut iov, buf, None);
    fuseread(dev, &mut uio, ioflag)?;
    Ok(len - uio.uio_resid)
}

/// `writev(fd, ...)` of `parts` on the device, one iovec each, as libfuse writes a reply.
pub fn dev_write(dev: Dev, parts: &[&[u8]]) -> Result<(), Errno> {
    let mut bufs: Vec<Vec<u8>> = parts.iter().map(|p| p.to_vec()).collect();
    let mut iov: Vec<Iovec> = bufs
        .iter_mut()
        .map(|b| Iovec {
            iov_base: b.as_mut_ptr().cast(),
            iov_len: b.len(),
        })
        .collect();
    let resid = parts.iter().map(|p| p.len()).sum();
    let mut uio = Uio::<TestGlue> {
        uio_iov: &mut iov,
        uio_offset: 0,
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: UioRw::UIO_WRITE,
        uio_procp: None,
    };
    fusewrite(dev, &mut uio, 0)
}

/// The size of an operation's input structure, as libfuse knows it (`<sys/fusebuf.h>`'s
/// userland table): independent of `fb_setup`'s.
pub fn op_in_len(opcode: u32) -> usize {
    match opcode {
        FUSE_GETATTR => size_of::<FuseGetattrIn>(),
        FUSE_SETATTR => size_of::<FuseSetattrIn>(),
        FUSE_MKNOD => size_of::<FuseMknodIn>(),
        FUSE_MKDIR => size_of::<FuseMkdirIn>(),
        FUSE_RENAME => size_of::<FuseRenameIn>(),
        FUSE_LINK => size_of::<FuseLinkIn>(),
        FUSE_OPEN | FUSE_OPENDIR => size_of::<FuseOpenIn>(),
        FUSE_READ | FUSE_READDIR => size_of::<FuseReadIn>(),
        FUSE_WRITE => size_of::<FuseWriteIn>(),
        FUSE_RELEASE | FUSE_RELEASEDIR => size_of::<FuseReleaseIn>(),
        FUSE_FSYNC => size_of::<FuseFsyncIn>(),
        FUSE_FLUSH => size_of::<FuseFlushIn>(),
        FUSE_INIT => size_of::<FuseInitIn>(),
        FUSE_FORGET => size_of::<FuseForgetIn>(),
        _ => 0, // LOOKUP, READLINK, SYMLINK, UNLINK, RMDIR, STATFS, DESTROY
    }
}

/// A request as the daemon reads it: the header, the input structure's bytes, the data.
pub struct Req {
    /// The header.
    pub hdr: FuseInHeader,
    /// The input structure's bytes.
    pub op: Vec<u8>,
    /// The data.
    pub data: Vec<u8>,
}

impl Req {
    /// The input structure `T`.
    pub fn op<T: AbiPod + Default>(&self) -> T {
        let mut v = T::default();
        bytes_of_mut(core::slice::from_mut(&mut v)).copy_from_slice(&self.op[..size_of::<T>()]);
        v
    }
}

/// Reads the next request from the device (`None` when there is none, `IO_NDELAY`).
pub fn read_req(dev: Dev) -> Option<Req> {
    let mut buf = std::vec![0u8; 64 * 1024];
    let n = match dev_read(dev, &mut buf, IO_NDELAY) {
        Ok(n) => n,
        Err(Errno::EAGAIN) => return None,
        Err(e) => std::panic!("fuseread: {e:?}"),
    };
    let Some(hdr) = pod_from_bytes::<FuseInHeader>(&buf) else {
        std::panic!("fuseread: no header");
    };
    std::assert_eq!(hdr.len as usize, n, "hdr.len is the request's length");
    let op_end = size_of::<FuseInHeader>() + op_in_len(hdr.opcode);
    Some(Req {
        hdr,
        op: buf[size_of::<FuseInHeader>()..op_end].to_vec(),
        data: buf[op_end..n].to_vec(),
    })
}

/// Writes a reply to request `unique`: `error` (negated errno), output structure, data.
pub fn reply(dev: Dev, unique: u64, error: i32, out: &[u8], data: &[u8]) -> Result<(), Errno> {
    let hdr = FuseOutHeader {
        len: (size_of::<FuseOutHeader>() + out.len() + data.len()) as u32,
        error,
        unique,
    };
    dev_write(dev, &[bytes_of(core::slice::from_ref(&hdr)), out, data])
}

/// The counters of `vfs.fuse`: open devices, requests to read, requests waiting for a reply.
pub fn stats() -> (i32, i32, i32) {
    let t = &vfs::<TestGlue>().fuse;
    (
        t.stat_opened_fusedev.load(Ordering::Relaxed),
        t.stat_fbufs_in.load(Ordering::Relaxed),
        t.stat_fbufs_wait.load(Ordering::Relaxed),
    )
}
/* </CODE> */
