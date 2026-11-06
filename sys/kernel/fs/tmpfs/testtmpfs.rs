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
//! The host tests' fixture of the kernel's tmpfs (N5.9t1, completed at N5.9t2): tmpfs's
//! configuration entry, the set-up (pages enough for the kernel map's faults, the kernel
//! maps, the test thread, `tmpfs_init`), the mount arguments' bytes, a mount over no device
//! and the same made the root (testffs's `make_root`, the thread's current directory). Host
//! tests and benches only (`cfg(test)` or `doubles`).

use core::mem::offset_of;
use std::sync::MutexGuard;

use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_LOCAL, MOUNT_TMPFS, TMPFS_ARGS_VERSION, TmpfsArgs};
use hal::sys::types::{Mode, Uid};

use crate::fs::lookup::ndinit;
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc, vfs_mount_free, vfs_unbusy};
use crate::fs::namei::{LOOKUP, NiDirp};
use crate::fs::subr::vfs;
use crate::fs::testfs::TestGlue;
use crate::fs::tmpfs::tmpfs_vfsops::{TmpfsVfs, tmpfs_init, tmpfs_mount};
use crate::proc::Thread;

/// The fixture's configuration entry: the kernel's tmpfs as `vfsconflist[]` has it
/// (`"tmpfs"`, type 19, `MNT_LOCAL`, `struct tmpfs_args`).
pub static TEST_TMPFS_CONF: Vfsconf<TestGlue> = Vfsconf::new(
    &TmpfsVfs::<TestGlue>::VFS,
    MOUNT_TMPFS,
    19,
    MNT_LOCAL,
    TmpfsArgs::SIZE,
);

/// 16 MiB of pages (a window's faults wait for memory when the pages run out), the kernel
/// maps, the test thread over testfs's root, `tmpfs_init`, nothing used: the memory guard
/// and the thread.
pub fn setup() -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = crate::fs::testfs::test_thread_cached(16 << 20, 64);
    crate::testutil::kernel_maps_install();
    // readdir's dirents are copied out of kernel buffers into the tests' kernel uios.
    crate::testutil::set_copy_kernel(true);
    if let Err(e) = tmpfs_init::<TestGlue>() {
        std::panic!("testtmpfs: tmpfs_init: {e:?}");
    }
    let t = &vfs::<TestGlue>().tmpfs;
    t.bytes_used.store(0, core::sync::atomic::Ordering::Relaxed);
    (g, p)
}

/// The bytes of a `struct tmpfs_args` as `sys_mount` copies them in.
pub fn args(size_max: i64, nodes_max: u64, uid: Uid, mode: Mode) -> [u8; TmpfsArgs::SIZE] {
    let mut b = [0u8; TmpfsArgs::SIZE];
    let mut put = |off: usize, v: &[u8]| b[off..off + v.len()].copy_from_slice(v);
    put(
        offset_of!(TmpfsArgs, ta_version),
        &TMPFS_ARGS_VERSION.to_ne_bytes(),
    );
    put(
        offset_of!(TmpfsArgs, ta_nodes_max),
        &nodes_max.to_ne_bytes(),
    );
    put(offset_of!(TmpfsArgs, ta_size_max), &size_max.to_ne_bytes());
    put(offset_of!(TmpfsArgs, ta_root_uid), &uid.to_ne_bytes());
    put(offset_of!(TmpfsArgs, ta_root_gid), &0u32.to_ne_bytes());
    put(offset_of!(TmpfsArgs, ta_root_mode), &mode.to_ne_bytes());
    b
}

/// `mount -t tmpfs` on "/tmp" with the argument bytes `data`: a mount of the fixture's
/// configuration, `tmpfs_mount`, unbusied; `Err` freed it.
pub fn mount(
    p: &'static Thread<TestGlue>,
    data: &mut [u8],
) -> Result<&'static Mount<TestGlue>, Errno> {
    let mp = vfs_mount_alloc(None, &TEST_TMPFS_CONF);
    let mut nd = ndinit(LOOKUP, 0, NiDirp::Sys(b"/tmp"), p);
    let r = tmpfs_mount(mp, b"/tmp", data, &mut nd, p);
    vfs_unbusy(mp);
    match r {
        Ok(()) => Ok(mp),
        Err(e) => {
            vfs_mount_free(mp);
            Err(e)
        }
    }
}

/// [`setup`] and a tmpfs of 1 MiB (root's, mode 0755) made the root and the thread's current
/// directory (testffs's `make_root`), for the vnode operations' tests through the system
/// calls' paths; `testffs::unmount_root` undoes it.
pub fn setup_root() -> (
    MutexGuard<'static, ()>,
    &'static Thread<TestGlue>,
    &'static Mount<TestGlue>,
) {
    let (g, p) = setup();
    let mut data = args(1 << 20, 0, 0, 0o755);
    let mp = match mount(p, &mut data) {
        Ok(mp) => mp,
        Err(e) => std::panic!("testtmpfs: tmpfs_mount: {e:?}"),
    };
    crate::fs::ufs::ffs::testffs::make_root(p, mp);
    (g, p, mp)
}
/* </CODE> */
