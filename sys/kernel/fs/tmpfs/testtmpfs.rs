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
//! The host tests' fixture of the kernel's tmpfs (N5.9t1): tmpfs's configuration entry, a
//! small vnode table of the operations its modules' tests need until `tmpfs_vnops` lands
//! (t2: the lock, `tmpfs_inactive`'s and `tmpfs_reclaim`'s work, testcd9660's `TestIsoVops`
//! shape), the set-up (pages enough for the kernel map's faults, the kernel maps, the test
//! thread, `tmpfs_init` and the tables), the mount arguments' bytes and a mount over no device. Host tests and benches only (`cfg(test)` or `doubles`).

use core::mem::offset_of;
use std::sync::MutexGuard;

use hal::sys::errno::Errno;
use hal::sys::mount::{MNT_LOCAL, MOUNT_TMPFS, TMPFS_ARGS_VERSION, TmpfsArgs};
use hal::sys::types::{Mode, Uid};

use crate::fs::cache::cache_purge;
use crate::fs::lookup::ndinit;
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc, vfs_mount_free, vfs_unbusy};
use crate::fs::namei::{LOOKUP, NiDirp};
use crate::fs::subr::{vfs, vrecycle};
use crate::fs::testfs::TestGlue;
use crate::fs::tmpfs::tmpfs::{tmpfs_node_reclaiming, vfstotmpfs, vptonode};
use crate::fs::tmpfs::tmpfs_subr::{tmpfs_free_node, tmpfs_uio_cached, tmpfs_uio_uncache};
use crate::fs::tmpfs::tmpfs_vfsops::{TmpfsVfs, tmpfs_init, tmpfs_mount};
use crate::fs::vnode::{LK_RWFLAGS, VREG, Vnode};
use crate::fs::vops::{Vnops, vop_unlock};
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

/// The fixture's vnode table: the node's lock (`tn_vlock`), `tmpfs_inactive`'s work (the kept
/// page dropped, the vnode unlocked, recycled when the node has no link) and
/// `tmpfs_reclaim`'s (the association broken under `tn_nlock`, the node freed when it has no
/// link and no `tmpfs_vnode_get` races); every other slot `EOPNOTSUPP`.
pub struct TestTmpfsVops;

impl Vnops<TestGlue> for TestTmpfsVops {
    fn lock(&self, vp: &'static Vnode<TestGlue>, flags: i32) -> Result<(), Errno> {
        vptonode(vp).tn_vlock.enter(flags & LK_RWFLAGS)
    }
    fn unlock(&self, vp: &'static Vnode<TestGlue>) -> Result<(), Errno> {
        vptonode(vp).tn_vlock.exit();
        Ok(())
    }
    fn islocked(&self, vp: &'static Vnode<TestGlue>) -> i32 {
        vptonode(vp).tn_vlock.status()
    }
    fn inactive(
        &self,
        vp: &'static Vnode<TestGlue>,
        p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        let node = vptonode(vp);
        if vp.vtype() == VREG && tmpfs_uio_cached(&node) {
            tmpfs_uio_uncache(&node);
        }
        let _ = vop_unlock(vp);
        if node.tn_links.get() == 0 {
            let _ = vrecycle(vp, p);
        }
        Ok(())
    }
    fn reclaim(
        &self,
        vp: &'static Vnode<TestGlue>,
        _p: Option<&Thread<TestGlue>>,
    ) -> Result<(), Errno> {
        let Some(mp) = vp.mount() else {
            std::panic!("testtmpfs: reclaim of a vnode without a mount");
        };
        let tmp = vfstotmpfs(mp);
        let node = vptonode(vp);
        let racing = {
            let _w = node.tn_nlock.write();
            node.tn_vnode.set(None);
            drop(vp.take_node());
            tmpfs_node_reclaiming(&node)
        };
        cache_purge(vp);
        if node.tn_links.get() == 0 && !racing {
            tmpfs_free_node(&tmp, node);
        }
        Ok(())
    }
}

/// The table, a `static` (one address for the registry).
pub static TEST_TMPFS_VOPS: TestTmpfsVops = TestTmpfsVops;

/// 16 MiB of pages (a window's faults wait for memory when the pages run out), the kernel
/// maps, the test thread over testfs's root, `tmpfs_init` and the fixture's tables, nothing
/// used: the memory guard and the thread.
pub fn setup() -> (MutexGuard<'static, ()>, &'static Thread<TestGlue>) {
    let (g, p) = crate::fs::testfs::test_thread_cached(16 << 20, 64);
    crate::testutil::kernel_maps_install();
    // readdir's dirents are copied out of kernel buffers into the tests' kernel uios.
    crate::testutil::set_copy_kernel(true);
    if let Err(e) = tmpfs_init::<TestGlue>() {
        std::panic!("testtmpfs: tmpfs_init: {e:?}");
    }
    let t = &vfs::<TestGlue>().tmpfs;
    let _ = t.vops.set(&TEST_TMPFS_VOPS);
    let _ = t.specvops.set(&TEST_TMPFS_VOPS);
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
/* </CODE> */
