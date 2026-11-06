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
 * Copyright (c) 2012-2013 Sylvestre Gallon <ccna.syl@gmail.com>
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
//! The FUSE request buffers: `fb_setup` makes a fusebuf for an operation, `fb_queue` hands
//! it to the daemon and waits for the reply, `fb_delete` frees it.
//!
//! Upstream: sys/miscfs/fuse/fusebuf.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fusebuf.rs@3c62ede68802
//!
//! ## Deviations
//! - The operation is a `u32`, the type of the header's `opcode` it is stored in (an `int`
//!   argument in C) (LZ's).
//! - `fb_queue` returns the daemon's error as a `Result`; an error number that is not an
//!   `Errno` (the daemon chooses it) becomes `EIO`, where the C returns the number as it is
//!   (LZ's).
//! - The host tests and benches (`cfg(test)`, `doubles`) have no daemon process: when the
//!   fixture's simulated daemon is set (`testfuse::TEST_DAEMON`), `fb_queue` runs it between
//!   `sleep_setup` and `sleep_finish` (`tsleep_nsec`'s two halves), where the daemon process
//!   would run while the requester sleeps; it reads the request from the device and writes
//!   its reply, whose `wakeup` then ends the sleep at once. The kernel build has no such
//!   call (LZ's hook, moved).
//!
//! ## Redesign
//! - `fb_setup` answers a counted handle to its `"fmsg"` pool item ([`FusebufRef`]) where
//!   LZ handed out a `&'static Fusebuf` over a raw item it wrote through `unsafe`: the value
//!   is built whole, the C's `PR_ZERO` and member stores, inside the pool's get (the C's
//!   order: `pool_get`, then the data's `malloc(M_WAITOK | M_ZERO)`). `fb_queue` lends the
//!   device a reference (a clone, parked by the queue) and keeps its own; `fb_delete` drops a
//!   handle, and the last one frees the data and puts the item back, as the C's `free` and
//!   `pool_put`. `fb_uuid`'s eight random bytes are two `hal::random::arc4random()` words
//!   (the kernel reaches `rnd.c` only through that bridge): two `rndlock` sections where the
//!   C's `arc4random_buf` takes one, an spl pair the C lacks per request (N5.9f1's D-f1-6,
//!   for ROADMAP N12: an `arc4random_buf` member of the bridge).
//! - `fb_queue`'s sleep is the C's: the fusebuf's address, `PWAIT`, no `PCATCH`, no timeout
//!   (kept as the C's design, D-N5.9-9: an unresponsive daemon hangs its callers until it is
//!   killed and its device closes).

use hal::pool::{PR_WAITOK, PoolArc};
use hal::sleep::{INFSLP, PWAIT, tsleep_nsec};
use hal::sys::errno::Errno;
use hal::sys::types::{Dev, Ino};

use crate::assert::{kassert, kpanic};
use crate::fs::fuse::fuse_device::fuse_device_queue_fbuf;
use crate::fs::fuse::fusebuf_h::{
    FUSE_FLUSH, FUSE_FORGET, FUSE_FSYNC, FUSE_GETATTR, FUSE_INIT, FUSE_LINK, FUSE_LOOKUP,
    FUSE_MKDIR, FUSE_MKNOD, FUSE_OPEN, FUSE_OPENDIR, FUSE_READ, FUSE_READDIR, FUSE_READLINK,
    FUSE_RELEASE, FUSE_RELEASEDIR, FUSE_RENAME, FUSE_SETATTR, FUSE_STATFS, FUSE_SYMLINK,
    FUSE_WRITE, FUSEBUFMAXSIZE, FuseAttrOut, FuseEntryOut, FuseFlushIn, FuseForgetIn, FuseFsyncIn,
    FuseGetattrIn, FuseInHeader, FuseInitIn, FuseInitOut, FuseLinkIn, FuseMkdirIn, FuseMknodIn,
    FuseOpenIn, FuseOpenOut, FuseReadIn, FuseReleaseIn, FuseRenameIn, FuseSetattrIn, FuseStatfsOut,
    FuseWriteIn, FuseWriteOut, Fusebuf,
};
use crate::fs::fuse::fusefs::M_FUSEFS;
use crate::fs::subr::vfs;
use crate::proc::{ProcGlue, THREAD_PID_OFFSET, Thread};
use hal::malloc::{M_WAITOK, MArray};

/// `struct fusebuf *` held: a counted handle to a request's `"fmsg"` pool item.
pub type FusebufRef = PoolArc<Fusebuf>;

/// `(op_in_len, op_out_len, op_out_buf)` of operation `op`: the sizes of its input and output
/// structures and whether a data reply is expected (`fb_setup`'s `switch`).
const fn op_lens(op: u32) -> (usize, usize, bool) {
    match op {
        FUSE_GETATTR => (size_of::<FuseGetattrIn>(), size_of::<FuseAttrOut>(), false),
        FUSE_SETATTR => (size_of::<FuseSetattrIn>(), size_of::<FuseAttrOut>(), false),
        FUSE_READLINK => (0, 0, true),
        FUSE_FLUSH => (size_of::<FuseFlushIn>(), 0, false),
        FUSE_INIT => (size_of::<FuseInitIn>(), size_of::<FuseInitOut>(), false),
        FUSE_OPEN | FUSE_OPENDIR => (size_of::<FuseOpenIn>(), size_of::<FuseOpenOut>(), false),
        FUSE_READ | FUSE_READDIR => (size_of::<FuseReadIn>(), 0, true),
        FUSE_MKDIR => (size_of::<FuseMkdirIn>(), size_of::<FuseEntryOut>(), false),
        FUSE_MKNOD => (size_of::<FuseMknodIn>(), size_of::<FuseEntryOut>(), false),
        FUSE_FSYNC => (size_of::<FuseFsyncIn>(), 0, false),
        FUSE_RENAME => (size_of::<FuseRenameIn>(), 0, false),
        FUSE_LINK => (size_of::<FuseLinkIn>(), size_of::<FuseEntryOut>(), false),
        FUSE_SYMLINK | FUSE_LOOKUP => (0, size_of::<FuseEntryOut>(), false),
        FUSE_RELEASE | FUSE_RELEASEDIR => (size_of::<FuseReleaseIn>(), 0, false),
        FUSE_WRITE => (size_of::<FuseWriteIn>(), size_of::<FuseWriteOut>(), false),
        FUSE_FORGET => (size_of::<FuseForgetIn>(), 0, false),
        FUSE_STATFS => (0, size_of::<FuseStatfsOut>(), false),
        _ => (0, 0, false),
    }
}

/// `fb_setup`: a fusebuf for operation `op` on inode `ino`, issued by `p`, with a zeroed data
/// buffer of `len` bytes (none when `len` is 0), its union zeroed. The sizes of the
/// operation's input and output structures, and whether a data reply is expected, follow from
/// `op`.
pub fn fb_setup<G: ProcGlue>(len: usize, ino: Ino, op: u32, p: &Thread<G>) -> FusebufRef {
    kassert!(len <= FUSEBUFMAXSIZE);

    let (op_in_len, op_out_len, op_out_buf) = op_lens(op);
    let cred = p.ucred();
    let fbuf = vfs::<G>().fuse.fbuf_pool.get_homed(PR_WAITOK, || {
        let unique =
            u64::from(hal::random::arc4random()) << 32 | u64::from(hal::random::arc4random());
        let hdr = FuseInHeader {
            len: (size_of::<FuseInHeader>() + op_in_len + len) as u32,
            opcode: op,
            unique,
            nodeid: ino,
            // When exposed to userspace, thread IDs have THREAD_PID_OFFSET added to keep
            // them from overlapping the PID range.
            pid: (p.p_tid.get() + THREAD_PID_OFFSET) as u32,
            uid: cred.cr_uid.get(),
            gid: cred.cr_gid.get(),
            padding: 0,
        };
        let dat = (len != 0).then(|| match MArray::new(M_FUSEFS, M_WAITOK, len, |_| 0u8) {
            Some(d) => d,
            None => kpanic(format_args!("fb_setup: malloc(M_WAITOK) failed")),
        });
        Fusebuf::new(hdr, op_in_len, op_out_len, op_out_buf, dat)
    });
    match fbuf {
        Some(fbuf) => fbuf,
        None => kpanic(format_args!("fb_setup: pool_get(PR_WAITOK) failed")),
    }
}

/// `fb_queue`: puts the fbuf on the queue and waits for the file system to process it. The
/// current process will block indefinitely and cannot be interrupted or killed. This is
/// consistent with how VFS system calls should behave. nfs supports the `-ointr` or `-i`
/// mount option and FUSE can too but this is non-trivial. The file system daemon must be
/// multi-threaded and also support being interrupted. Note that libfuse currently only
/// supports single-threaded daemons.
///
/// Why not timeout similar to `mount_nfs -osoft`? It introduces another point of failure and
/// a possible mount option (to specify the timeout) that users need to understand and tune
/// to avoid premature timeouts for slow file systems. More complexity, less reliability.
///
/// In the case where the daemon has become unresponsive the daemon will have to be killed
/// in order for the current process to wakeup. The FUSE device is automatically closed when
/// the daemon terminates and any waiting fbuf is woken up.
pub fn fb_queue<G: ProcGlue>(dev: Dev, fbuf: &FusebufRef) -> Result<(), Errno> {
    fuse_device_queue_fbuf::<G>(dev, fbuf.clone());
    let chan = PoolArc::as_ptr(fbuf);
    #[cfg(any(test, feature = "doubles"))]
    if let Some(daemon) = crate::fs::fuse::testfuse::test_daemon() {
        hal::sleep::sleep_setup(chan, PWAIT, "fuse");
        daemon(dev);
        let _ = hal::sleep::sleep_finish(INFSLP, true);
        return fb_result(fbuf);
    }
    let _ = tsleep_nsec(chan, PWAIT, "fuse", INFSLP);

    fb_result(fbuf)
}

/// `return (fbuf->fb_err)`: the daemon's error (see the module's deviations).
fn fb_result(fbuf: &Fusebuf) -> Result<(), Errno> {
    match fbuf.fb_err() {
        0 => Ok(()),
        e => Err(Errno::from_raw(e).unwrap_or(Errno::EIO)),
    }
}

/// `fb_delete`: drops the requester's handle; the last one frees the fusebuf and its data
/// buffer. `None` (the C's NULL) is ignored.
pub fn fb_delete(fbuf: impl Into<Option<FusebufRef>>) {
    drop(fbuf.into());
}
/* </CODE> */
