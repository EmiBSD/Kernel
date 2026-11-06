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
//! FUSE file handles: `FUSE_OPEN`/`FUSE_OPENDIR` and `FUSE_RELEASE`/`FUSE_RELEASEDIR` for a
//! node's handle of one kind, and the handle a read or write uses.
//!
//! Upstream: sys/miscfs/fuse/fuse_file.c @ 3ce1f3f79392
//! LZ: sys/miscfs/fuse/fuse_file.rs@3c62ede68802
//!
//! ## Deviations
//! - `flags` is the `int` the C passes, sent as the protocol's `uint32_t`; `isdir` is a
//!   `bool` (LZ's).
//!
//! ## Redesign
//! - The handle kind is a [`FufhKind`], one of the three slots of `fufh[]` (the C's `enum
//!   fufh_type` used as an index, `FUFH_INVALID` excluded); the requests are counted handles
//!   (`fb_setup`'s), dropped by `fb_delete` as the C's. Otherwise LZ's functions over the
//!   kernel's node and mount.

use hal::sys::errno::Errno;

use crate::fs::fuse::fusebuf::{fb_delete, fb_queue, fb_setup};
use crate::fs::fuse::fusebuf_h::{
    FUSE_OPEN, FUSE_OPENDIR, FUSE_RELEASE, FUSE_RELEASEDIR, FuseOpenIn, FuseOpenOut, FuseReleaseIn,
};
use crate::fs::fuse::fusefs::FusefsMnt;
use crate::fs::fuse::fusefs_node::{FUFH_INVALID, FufhKind, FusefsFilehandle, FusefsNode};
use crate::proc::{ProcGlue, Thread};

/// `fusefs_file_open`: asks the daemon to open the node (a directory when `isdir`) with
/// `flags` and records the handle it returns as the node's handle of kind `fufh_type`.
/// Nothing is sent before the session is up.
pub fn fusefs_file_open<G: ProcGlue>(
    fmp: &FusefsMnt<G>,
    ip: &FusefsNode<G>,
    fufh_type: FufhKind,
    flags: i32,
    isdir: bool,
    p: &Thread<G>,
) -> Result<(), Errno> {
    if fmp.sess_init.get() == 0 {
        return Ok(());
    }

    let fbuf = fb_setup(
        0,
        ip.i_number,
        if isdir { FUSE_OPENDIR } else { FUSE_OPEN },
        p,
    );
    fbuf.op_set(&FuseOpenIn {
        flags: flags as u32,
        ..FuseOpenIn::default()
    });

    if let Err(error) = fb_queue::<G>(fmp.dev, &fbuf) {
        fb_delete(fbuf);
        return Err(error);
    }

    ip.set_fufh(
        fufh_type,
        FusefsFilehandle {
            fh_id: fbuf.op_get::<FuseOpenOut>().fh,
            fh_type: fufh_type.fufh_type(),
        },
    );

    fb_delete(fbuf);
    Ok(())
}

/// `fusefs_file_close`: asks the daemon to release the node's handle of kind `fufh_type`
/// (the daemon's error is returned, but the handle is forgotten in any case).
pub fn fusefs_file_close<G: ProcGlue>(
    fmp: &FusefsMnt<G>,
    ip: &FusefsNode<G>,
    fufh_type: FufhKind,
    flags: i32,
    isdir: bool,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let mut error = Ok(());

    if fmp.sess_init.get() != 0 {
        let fbuf = fb_setup(
            0,
            ip.i_number,
            if isdir { FUSE_RELEASEDIR } else { FUSE_RELEASE },
            p,
        );
        fbuf.op_set(&FuseReleaseIn {
            fh: ip.fufh(fufh_type).fh_id,
            flags: flags as u32,
            ..FuseReleaseIn::default()
        });

        error = fb_queue::<G>(fmp.dev, &fbuf);
        // if (error && (error != ENOSYS)) DPRINTF("file close error: %d\n", error);

        fb_delete(fbuf);
    }

    ip.set_fufh(
        fufh_type,
        FusefsFilehandle {
            fh_id: u64::MAX,
            fh_type: FUFH_INVALID,
        },
    );

    error
}

/// `fusefs_fd_get`: the node's handle of kind `type`, or its read-write one when it has
/// none of that kind.
pub fn fusefs_fd_get<G: ProcGlue>(ip: &FusefsNode<G>, r#type: FufhKind) -> u64 {
    let t = if ip.fufh(r#type).fh_type == FUFH_INVALID {
        FufhKind::RdWr
    } else {
        r#type
    };

    ip.fufh(t).fh_id
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the file handles against the fixture's simulated daemon: an open
    // records the daemon's handle under its kind, a close releases it and forgets it, an
    // error is returned with the handle forgotten all the same, nothing is sent before the
    // session is up, and `fusefs_fd_get` falls back to the read-write handle.

    use std::{assert_eq, vec::Vec};

    use super::*;
    use crate::fs::fuse::fuse_device::fuseclose;
    use crate::fs::fuse::fusebuf_h::{FUSE_RELEASE, FuseReleaseIn};
    use crate::fs::fuse::fusefs_node::FUFH_RDONLY;
    use crate::fs::fuse::testfuse::{
        fake_fmp, mount_fmp, node, open_dev, read_req, reply, set_daemon, setup,
    };
    use crate::fs::testfs::TestGlue;
    use hal::user::bytes_of;

    std::thread_local! {
        /// The requests the daemon saw: (opcode, nodeid, release.fh).
        static SEEN: core::cell::RefCell<Vec<(u32, u64, u64)>> =
            const { core::cell::RefCell::new(Vec::new()) };
    }

    /// Answers `FUSE_OPEN` with handle 77, `FUSE_OPENDIR` with 78, a release of handle 78
    /// with `EIO`, any other release with success.
    fn daemon(dev: hal::sys::types::Dev) {
        while let Some(req) = read_req(dev) {
            let fh = if req.hdr.opcode == FUSE_RELEASE || req.hdr.opcode == FUSE_RELEASEDIR {
                req.op::<FuseReleaseIn>().fh
            } else {
                0
            };
            SEEN.with(|s| s.borrow_mut().push((req.hdr.opcode, req.hdr.nodeid, fh)));
            let r = match req.hdr.opcode {
                FUSE_OPEN | FUSE_OPENDIR => {
                    let out = FuseOpenOut {
                        fh: if req.hdr.opcode == FUSE_OPEN { 77 } else { 78 },
                        ..FuseOpenOut::default()
                    };
                    reply(
                        dev,
                        req.hdr.unique,
                        0,
                        bytes_of(core::slice::from_ref(&out)),
                        &[],
                    )
                }
                _ if fh == 78 => reply(dev, req.hdr.unique, -(Errno::EIO as i32), &[], &[]),
                _ => reply(dev, req.hdr.unique, 0, &[], &[]),
            };
            r.expect("reply");
        }
    }

    #[test]
    fn handles_are_opened_released_and_looked_up() {
        let (_g, p) = setup();
        let dev = open_dev(p, 0);
        let fmp = mount_fmp(dev, 4096);
        let ip = node(&fmp, 5);
        set_daemon(Some(daemon));

        fusefs_file_open(&fmp, &ip, FufhKind::RdOnly, 0, false, p).expect("open");
        assert_eq!(ip.fufh(FufhKind::RdOnly).fh_id, 77);
        assert_eq!(ip.fufh(FufhKind::RdOnly).fh_type, FUFH_RDONLY);
        fusefs_file_open(&fmp, &ip, FufhKind::RdWr, 2, true, p).expect("opendir");
        assert_eq!(ip.fufh(FufhKind::RdWr).fh_id, 78);

        ip.set_fufh(
            FufhKind::WrOnly,
            FusefsFilehandle {
                fh_id: 0,
                fh_type: FUFH_INVALID,
            },
        );
        assert_eq!(
            fusefs_fd_get(&ip, FufhKind::WrOnly),
            78,
            "the read-write one"
        );
        assert_eq!(fusefs_fd_get(&ip, FufhKind::RdOnly), 77);

        fusefs_file_close(&fmp, &ip, FufhKind::RdOnly, 0, false, p).expect("release");
        assert_eq!(ip.fufh(FufhKind::RdOnly).fh_type, FUFH_INVALID);
        assert_eq!(ip.fufh(FufhKind::RdOnly).fh_id, u64::MAX);
        assert_eq!(
            fusefs_file_close(&fmp, &ip, FufhKind::RdWr, 0, true, p),
            Err(Errno::EIO)
        );
        assert_eq!(
            ip.fufh(FufhKind::RdWr).fh_type,
            FUFH_INVALID,
            "forgotten all the same"
        );
        SEEN.with(|s| {
            assert_eq!(
                *s.borrow(),
                [
                    (FUSE_OPEN, 5, 0),
                    (FUSE_OPENDIR, 5, 0),
                    (FUSE_RELEASE, 5, 77),
                    (FUSE_RELEASEDIR, 5, 78)
                ]
            );
        });

        // Before the session is up nothing is sent and the handle is left as it was.
        let quiet = fake_fmp(dev, 4096);
        quiet.sess_init.set(0);
        let iq = node(&quiet, 6);
        fusefs_file_open(&quiet, &iq, FufhKind::RdOnly, 0, false, p).expect("no session");
        assert_eq!(iq.fufh(FufhKind::RdOnly).fh_id, 0);
        assert_eq!(SEEN.with(|s| s.borrow().len()), 4);
        assert_eq!(quiet.sess_init.get(), 0);

        set_daemon(None);
        fuseclose::<TestGlue>(dev, 0, 0, Some(p)).expect("close");
    }
}
/* </TESTS> */
