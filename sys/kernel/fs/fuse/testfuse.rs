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
//! The host tests' fixture of the kernel's fuse (N5.9f1, completed at N5.9f2): the set-up (the
//! test thread, `fusefs_init`), the simulated daemon `fb_queue` runs (LZ's `TEST_DAEMON`,
//! moved), the daemon's side of the device (LZ's `fuse_device` tests' helpers, moved:
//! `read_req`, `reply`, `dev_read`, `dev_write`), a mount structure over a mount that is not a
//! real one and a node on it (the device's and the hash's tests), and for the vnode
//! operations' tests (N5.9f2) a daemon scripted by opcode ([`script`], [`scripted`]: one
//! closure per request kind, every request seen recorded), fuse's configuration entry, the
//! mount arguments' bytes, the daemon's descriptor of the device and a real mount over it
//! ([`mounted`], LZ's `fuse_vfsops` tests' set-up, moved). Host tests and benches only
//! (`cfg(test)` or `doubles`).

use core::cell::{Cell, RefCell};
use core::mem::offset_of;
use core::sync::atomic::Ordering;
use std::rc::Rc;
use std::sync::MutexGuard;
use std::vec::Vec;

use hal::malloc::{M_WAITOK, MArc};
use hal::sys::dirent::Dirent;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FREAD, FWRITE};
use hal::sys::mount::{FusefsArgs, MOUNT_FUSEFS};
use hal::sys::types::{Dev, Ino, Off, makedev};
use hal::user::{AbiPod, bytes_of, bytes_of_mut, pod_from_bytes};

use crate::descrip::{falloc, fdinsert, fdplock, fdpunlock};
use crate::file::{DTYPE_VNODE, FileGlue};
use crate::fs::fuse::fuse_device::{fuse_device_set_fmp, fuseopen, fuseread, fusewrite};
use crate::fs::fuse::fuse_vfsops::{FusefsVfs, PENDING, fusefs_init, fusefs_mount};
use crate::fs::fuse::fusebuf_h::{
    FUSE_DESTROY, FUSE_FATTR_SIZE, FUSE_FLUSH, FUSE_FORGET, FUSE_FSYNC, FUSE_GETATTR, FUSE_INIT,
    FUSE_LINK, FUSE_LOOKUP, FUSE_MKDIR, FUSE_MKNOD, FUSE_OPEN, FUSE_OPENDIR, FUSE_READ,
    FUSE_READDIR, FUSE_RELEASE, FUSE_RELEASEDIR, FUSE_RENAME, FUSE_ROOT_ID, FUSE_SETATTR,
    FUSE_STATFS, FUSE_WRITE, FuseAttr, FuseAttrOut, FuseDirent, FuseEntryOut, FuseFlushIn,
    FuseForgetIn, FuseFsyncIn, FuseGetattrIn, FuseInHeader, FuseInitIn, FuseInitOut, FuseKstatfs,
    FuseLinkIn, FuseMkdirIn, FuseMknodIn, FuseOpenIn, FuseOpenOut, FuseOutHeader, FuseReadIn,
    FuseReleaseIn, FuseRenameIn, FuseSetattrIn, FuseStatfsOut, FuseWriteIn, FuseWriteOut,
    fuse_dirent_size,
};
use crate::fs::fuse::fusefs::{FusefsMnt, FusefsMntRef, M_FUSEFS};
use crate::fs::fuse::fusefs_node::{
    FUFH_INVALID, FufhKind, FusefsFilehandle, FusefsNode, FusefsNodeRef,
};
use crate::fs::lookup::ndinit;
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc, vfs_mount_free, vfs_unbusy};
use crate::fs::namei::{LOOKUP, NiDirp};
use crate::fs::subr::{cdevvp, getnewvnode, vfs, vref};
use crate::fs::testfs::{TEST_CONF, TestGlue};
use crate::fs::vnode::{IO_NDELAY, VT_FUSEFS, Vnode};
use crate::fs::vops::{Cred, VFS_ROOT, vop_read, vop_readdir, vop_write};
use crate::proc::{ProcGlue, Thread};
use crate::uio::{Iovec, Uio, UioRw, UioSeg};

/// fuse(4)'s character major (`cdevsw[92]`).
pub const FUSE_MAJOR: u32 = 92;

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
/// forgets fuse's hash and counters), `fusefs_init` (the `"fmsg"` pool, made once per test
/// process, and a new inode hash), the kernel uios' copies on, no simulated daemon and an
/// empty script: the memory guard and the thread.
pub fn setup() -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = crate::fs::testfs::test_thread();
    // The device's kernel uios (`dev_read`, `dev_write`) are copied by the uiomove double.
    crate::testutil::set_copy_kernel(true);
    crate::fs::fuse::fuse_ihash::fuse_ihash_test_reset::<TestGlue>();
    if let Err(e) = fusefs_init::<TestGlue>() {
        std::panic!("testfuse: fusefs_init: {e:?}");
    }
    set_daemon(None);
    script_reset();
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

/// A vnode of fuse's table and its node for inode `ino` of `fmp`, its handles invalid,
/// unhashed: `fusefs_vget`'s before `fuse_ihashins`.
pub fn node(fmp: &FusefsMntRef<TestGlue>, ino: Ino) -> FusefsNodeRef<TestGlue> {
    let vp = match getnewvnode(VT_FUSEFS, Some(fmp.mp), &vfs::<TestGlue>().fuse.vops) {
        Ok(vp) => vp,
        Err(e) => std::panic!("getnewvnode: {e:?}"),
    };
    let Some(ip) = MArc::new(M_FUSEFS, M_WAITOK, FusefsNode::new(vp, fmp.clone(), ino)) else {
        std::panic!("testfuse: no node");
    };
    for t in FufhKind::ALL {
        ip.set_fufh(
            t,
            FusefsFilehandle {
                fh_id: 0,
                fh_type: FUFH_INVALID,
            },
        );
    }
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
#[derive(Clone)]
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

/// A reply of the scripted daemon: the error (a negated errno, 0 for none), the output
/// structure's bytes and the data.
pub struct Answer {
    /// `hdr.error`.
    pub error: i32,
    /// The output structure's bytes.
    pub out: Vec<u8>,
    /// The data.
    pub data: Vec<u8>,
}

impl Answer {
    /// Success with the output structure `out` and no data.
    pub fn out<T: AbiPod>(out: &T) -> Self {
        Self {
            error: 0,
            out: bytes_of(core::slice::from_ref(out)).to_vec(),
            data: Vec::new(),
        }
    }

    /// Success with the data `data` and no output structure.
    pub fn data(data: &[u8]) -> Self {
        Self {
            error: 0,
            out: Vec::new(),
            data: data.to_vec(),
        }
    }

    /// Success with nothing.
    pub fn ok() -> Self {
        Self::data(&[])
    }

    /// The error `e`.
    pub fn err(e: Errno) -> Self {
        Self {
            error: -(e as i32),
            out: Vec::new(),
            data: Vec::new(),
        }
    }
}

/// An answer of the scripted daemon to a request kind.
type Script = Rc<dyn Fn(&Req) -> Answer>;

std::thread_local! {
    /// The scripted daemon's answers, by opcode.
    static SCRIPT: RefCell<Vec<(u32, Script)>> = const { RefCell::new(Vec::new()) };
    /// Every request the scripted daemon read, in order.
    static SEEN: RefCell<Vec<Req>> = const { RefCell::new(Vec::new()) };
}

/// The scripted daemon answers requests of `opcode` with `f` (replacing an earlier answer).
pub fn script(opcode: u32, f: impl Fn(&Req) -> Answer + 'static) {
    SCRIPT.with(|s| {
        let mut s = s.borrow_mut();
        s.retain(|(o, _)| *o != opcode);
        s.push((opcode, Rc::new(f)));
    });
}

/// No answer scripted, nothing seen.
pub fn script_reset() {
    SCRIPT.with(|s| s.borrow_mut().clear());
    SEEN.with(|s| s.borrow_mut().clear());
}

/// The requests the scripted daemon read since the last [`take_seen`] or reset.
pub fn take_seen() -> Vec<Req> {
    SEEN.with(|s| core::mem::take(&mut *s.borrow_mut()))
}

/// The opcodes of [`take_seen`].
pub fn take_ops() -> Vec<u32> {
    take_seen().iter().map(|r| r.hdr.opcode).collect()
}

/// The scripted daemon (`set_daemon(Some(scripted))`): answers every request waiting on the
/// device by its opcode's script, `FUSE_FORGET` with nothing (it has no reply); without a
/// script `FUSE_INIT` gets a 7.19 reply with a `max_write` of 64 KiB, `FUSE_RELEASE*` and
/// `FUSE_DESTROY` success, anything else `ENOSYS`.
pub fn scripted(dev: Dev) {
    while let Some(req) = read_req(dev) {
        SEEN.with(|s| s.borrow_mut().push(req.clone()));
        if req.hdr.opcode == FUSE_FORGET {
            continue;
        }
        let f = SCRIPT.with(|s| {
            s.borrow()
                .iter()
                .find(|(o, _)| *o == req.hdr.opcode)
                .map(|(_, f)| f.clone())
        });
        let a = match f {
            Some(f) => f(&req),
            None => match req.hdr.opcode {
                FUSE_INIT => Answer::out(&FuseInitOut {
                    major: 7,
                    minor: 19,
                    max_write: 65536,
                    ..FuseInitOut::default()
                }),
                FUSE_RELEASE | FUSE_RELEASEDIR | FUSE_DESTROY => Answer::ok(),
                _ => Answer::err(Errno::ENOSYS),
            },
        };
        if let Err(e) = reply(dev, req.hdr.unique, a.error, &a.out, &a.data) {
            std::panic!(
                "the scripted daemon's reply to opcode {}: {e:?}",
                req.hdr.opcode
            );
        }
    }
}

/// The fixture's configuration entry: fuse as `vfsconflist[]` has it (`"fuse"`, type 18, no
/// flags, `struct fusefs_args`).
pub static TEST_FUSE_CONF: Vfsconf<TestGlue> = Vfsconf::new(
    &FusefsVfs::<TestGlue>::VFS,
    MOUNT_FUSEFS,
    18,
    0,
    FusefsArgs::SIZE,
);

/// The bytes of a `struct fusefs_args` as `sys_mount` copies them in.
pub fn args(fd: i32, max_read: i32, allow_other: i32) -> [u8; FusefsArgs::SIZE] {
    let mut b = [0u8; FusefsArgs::SIZE];
    let mut put = |off: usize, v: &[u8]| b[off..off + v.len()].copy_from_slice(v);
    put(offset_of!(FusefsArgs, fd), &fd.to_ne_bytes());
    put(offset_of!(FusefsArgs, max_read), &max_read.to_ne_bytes());
    put(
        offset_of!(FusefsArgs, allow_other),
        &allow_other.to_ne_bytes(),
    );
    b
}

/// A descriptor of `p` for a character device vnode of `dev`, as `open(2)` of `/dev/fuse0`
/// leaves it in the daemon's table (the vnode from `cdevvp`; the file is never closed: the
/// test's memory goes with its guard).
pub fn install_dev_fd(p: &'static Thread<TestGlue>, dev: Dev) -> i32 {
    let vp = match cdevvp::<TestGlue>(dev) {
        Ok(Some(vp)) => vp,
        _ => std::panic!("testfuse: cdevvp"),
    };
    install_vnode_fd(p, vp)
}

/// A descriptor of `p` for a file open on `vp` for reading and writing.
pub fn install_vnode_fd(p: &'static Thread<TestGlue>, vp: &'static Vnode<TestGlue>) -> i32 {
    let mut g = fdplock(p.fd());
    let Ok((fp, fd)) = falloc(p, &mut g) else {
        std::panic!("testfuse: falloc");
    };
    fp.f_flag.store((FREAD | FWRITE) as u32, Ordering::SeqCst);
    vref(vp);
    let (ops, data) = <<TestGlue as ProcGlue>::FileGlue as FileGlue>::vnode_file(vp);
    fp.install(DTYPE_VNODE, ops, data);
    fdinsert(&mut g, fd, 0, &fp);
    fdpunlock(g);
    let _ = fp.frele(Some(p));
    fd
}

/// `mount -t fuse` on `path` with the argument bytes `data`: a mount of the fixture's
/// configuration, `fusefs_mount`, unbusied; `Err` freed it.
pub fn mount(
    p: &'static Thread<TestGlue>,
    path: &[u8],
    data: &mut [u8],
) -> Result<&'static Mount<TestGlue>, Errno> {
    let mp = vfs_mount_alloc(None, &TEST_FUSE_CONF);
    let mut nd = ndinit(LOOKUP, 0, NiDirp::Sys(path), p);
    let r = fusefs_mount(mp, path, data, &mut nd, p);
    vfs_unbusy(mp);
    match r {
        Ok(()) => Ok(mp),
        Err(e) => {
            vfs_mount_free(mp);
            Err(e)
        }
    }
}

/// [`setup`], `/dev/fuse0` opened by the daemon with its descriptor, the scripted daemon,
/// a mount on `"/mnt"` (`max_read` 4096) whose `FUSE_INIT` the daemon answered (the session
/// up, `max_write` 64 KiB), and its root vnode, locked and referenced; nothing seen yet.
pub fn mounted() -> (
    MutexGuard<'static, ()>,
    &'static Thread<TestGlue>,
    Dev,
    &'static Mount<TestGlue>,
    &'static Vnode<TestGlue>,
) {
    let (g, p) = setup();
    let dev = open_dev(p, 0);
    let fd = install_dev_fd(p, dev);
    set_daemon(Some(scripted));
    let mp = match mount(p, b"/mnt", &mut args(fd, 4096, 0)) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testfuse: fusefs_mount: {e:?}"),
    };
    scripted(dev); // FUSE_INIT
    let root = match VFS_ROOT(mp) {
        Ok(root) => root,
        Err(e) => std::panic!("testfuse: fusefs_root: {e:?}"),
    };
    let _ = take_seen();
    (g, p, dev, mp, root)
}
std::thread_local! {
    /// The contents of [`hello_fs`]'s file `hello`.
    static CONTENT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// The contents of [`hello_fs`]'s `hello`.
pub fn content() -> Vec<u8> {
    CONTENT.with(|c| c.borrow().clone())
}

/// The attributes of [`hello_fs`]'s inode 1 (the root) or 2 (`hello`).
pub fn hello_attr(ino: u64) -> FuseAttr {
    let mut a = FuseAttr {
        ino,
        ..FuseAttr::default()
    };
    if ino == FUSE_ROOT_ID {
        a.mode = 0o040755;
        a.nlink = 2;
        a.size = 512;
    } else {
        a.mode = 0o100644;
        a.nlink = 1;
        a.size = CONTENT.with(|c| c.borrow().len()) as u64;
        a.mtime = 1_700_000_000;
        a.mtimensec = 5;
    }
    a
}

/// FUSE dirents as `FUSE_READDIR` returns them: `(ino, name, type)` from index `offset` (each
/// entry's `off` is the next one's index), at most `size` bytes.
pub fn dirents(all: &[(u64, &[u8], u32)], offset: u64, size: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, (ino, name, ty)) in all.iter().enumerate().skip(offset as usize) {
        let d = FuseDirent {
            ino: *ino,
            off: i as u64 + 1,
            namelen: name.len() as u32,
            r#type: *ty,
        };
        let len = fuse_dirent_size(&d);
        if out.len() + len > size {
            break;
        }
        let start = out.len();
        out.extend_from_slice(bytes_of(core::slice::from_ref(&d)));
        out.extend_from_slice(name);
        out.resize(start + len, 0);
    }
    out
}

/// LZ's `fuse_vfsops` tests' file system, scripted: a root directory (inode 1) holding one
/// file, `hello` (inode 2, `contents` its bytes): `FUSE_GETATTR`, `FUSE_LOOKUP`, `FUSE_OPEN`
/// (handle 10 + the flags), `FUSE_OPENDIR` (handle 20), `FUSE_READ`, `FUSE_WRITE`,
/// `FUSE_SETATTR` (a truncation), `FUSE_READDIR` (`.`, `..`, `hello`) and `FUSE_STATFS`.
pub fn hello_fs(contents: &[u8]) {
    CONTENT.with(|c| *c.borrow_mut() = contents.to_vec());
    script(FUSE_GETATTR, |r| {
        Answer::out(&FuseAttrOut {
            attr: hello_attr(r.hdr.nodeid),
            ..FuseAttrOut::default()
        })
    });
    script(FUSE_LOOKUP, |r| {
        if r.hdr.nodeid == FUSE_ROOT_ID && r.data == b"hello\0" {
            Answer::out(&FuseEntryOut {
                nodeid: 2,
                attr: hello_attr(2),
                ..FuseEntryOut::default()
            })
        } else {
            Answer::err(Errno::ENOENT)
        }
    });
    script(FUSE_OPEN, |r| {
        Answer::out(&FuseOpenOut {
            fh: 10 + u64::from(r.op::<FuseOpenIn>().flags),
            ..FuseOpenOut::default()
        })
    });
    script(FUSE_OPENDIR, |_| {
        Answer::out(&FuseOpenOut {
            fh: 20,
            ..FuseOpenOut::default()
        })
    });
    script(FUSE_READ, |r| {
        let rd = r.op::<FuseReadIn>();
        CONTENT.with(|c| {
            let c = c.borrow();
            let start = (rd.offset as usize).min(c.len());
            let end = (start + rd.size as usize).min(c.len());
            Answer::data(&c[start..end])
        })
    });
    script(FUSE_WRITE, |r| {
        let w = r.op::<FuseWriteIn>();
        std::assert_eq!(w.size as usize, r.data.len());
        CONTENT.with(|c| {
            let mut c = c.borrow_mut();
            let end = w.offset as usize + r.data.len();
            if c.len() < end {
                c.resize(end, 0);
            }
            c[w.offset as usize..end].copy_from_slice(&r.data);
        });
        Answer::out(&FuseWriteOut {
            size: w.size,
            ..FuseWriteOut::default()
        })
    });
    script(FUSE_SETATTR, |r| {
        let s = r.op::<FuseSetattrIn>();
        if s.valid & FUSE_FATTR_SIZE != 0 {
            CONTENT.with(|c| c.borrow_mut().resize(s.size as usize, 0));
        }
        Answer::out(&FuseAttrOut {
            attr: hello_attr(r.hdr.nodeid),
            ..FuseAttrOut::default()
        })
    });
    script(FUSE_READDIR, |r| {
        let rd = r.op::<FuseReadIn>();
        std::assert_eq!(rd.fh, 20, "the handle FUSE_OPENDIR gave");
        let all: [(u64, &[u8], u32); 3] = [(1, b".", 4), (1, b"..", 4), (2, b"hello", 8)];
        Answer::data(&dirents(&all, rd.offset, rd.size as usize))
    });
    script(FUSE_STATFS, |_| {
        Answer::out(&FuseStatfsOut {
            st: FuseKstatfs {
                blocks: 100,
                bfree: 50,
                bavail: 40,
                files: 10,
                ffree: 5,
                bsize: 65536,
                namelen: 255,
                frsize: 512,
                ..FuseKstatfs::default()
            },
        })
    });
}
/// `VOP_READ` or `VOP_WRITE` of the kernel bytes `buf` at `offset`: the bytes moved.
pub fn rdwr(
    vp: &'static Vnode<TestGlue>,
    p: &'static Thread<TestGlue>,
    buf: &mut [u8],
    offset: Off,
    write: bool,
) -> Result<usize, Errno> {
    let len = buf.len();
    let mut iov = [Iovec {
        iov_base: buf.as_mut_ptr().cast(),
        iov_len: len,
    }];
    let mut uio = Uio {
        uio_iov: &mut iov,
        uio_offset: offset,
        uio_resid: len,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: if write {
            UioRw::UIO_WRITE
        } else {
            UioRw::UIO_READ
        },
        uio_procp: Some(p),
    };
    let cred = Cred::real(p.ucred());
    if write {
        vop_write(vp, &mut uio, 0, cred)?;
    } else {
        vop_read(vp, &mut uio, 0, cred)?;
    }
    Ok(len - uio.uio_resid)
}

/// Directory entries as `readdir` returns them: name, `d_type`, `d_fileno`, `d_off`.
pub type Entries = Vec<(Vec<u8>, u8, u64, Off)>;

/// `VOP_READDIR` of `vp` from `offset` into `size` kernel bytes: the entries (each name
/// checked NUL-terminated), the end offset and the EOF flag.
pub fn readdir(
    vp: &'static Vnode<TestGlue>,
    p: &'static Thread<TestGlue>,
    offset: Off,
    size: usize,
) -> Result<(Entries, Off, i32), Errno> {
    let mut buf = std::vec![0u8; size];
    let mut iov = [Iovec {
        iov_base: buf.as_mut_ptr().cast(),
        iov_len: size,
    }];
    let mut uio = Uio {
        uio_iov: &mut iov,
        uio_offset: offset,
        uio_resid: size,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: UioRw::UIO_READ,
        uio_procp: Some(p),
    };
    let mut eof = 0;
    vop_readdir(vp, &mut uio, Cred::real(p.ucred()), &mut eof)?;
    let used = size - uio.uio_resid;
    let mut out = Vec::new();
    let mut off = 0;
    while off < used {
        let Some(d) = Dirent::from_bytes(&buf[off..]) else {
            std::panic!("testfuse: a short dirent");
        };
        let name = buf[off + Dirent::NAME_OFFSET..][..usize::from(d.d_namlen)].to_vec();
        std::assert_eq!(buf[off + Dirent::NAME_OFFSET + usize::from(d.d_namlen)], 0);
        out.push((name, d.d_type, d.d_fileno, d.d_off));
        off += usize::from(d.d_reclen);
    }
    Ok((out, uio.uio_offset, eof))
}
/* </CODE> */
