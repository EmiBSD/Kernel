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
 * Copyright (c) 1985, 1989, 1991, 1993
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
 *	@(#)namei.h	8.4 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/namei.h>` for `bsd`: the encapsulation of `namei` parameters (`struct nameidata` and
//! the `struct componentname` it hands to `VOP_LOOKUP`), the namei operations and flags, the
//! name cache's statistics and the unveil flags, all the kernel's (`kernel::fs::namei`,
//! `lookup` and `cache`, N5.3), re-exported at the path of their 54 users.
//!
//! Upstream: sys/sys/namei.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - `ni_dirp` is a [`NiDirp`]: a user address or a kernel byte string; `ni_segflg` is kept
//!   and always agrees with it (`ndinitat` sets both). `NDINITAT`/`NDINIT` are `ndinitat`/
//!   `ndinit`, which return the structure instead of filling one.
//! - The pathname buffer is the componentname's own ([`PathBuf`], `kernel::fs::namei`'s
//!   notes): the callers that free it (`pool_put(&namei_pool, cnp->cn_pnbuf)`) drop what
//!   `take_pnbuf` gives, those that lend one (`nfsd`, `__realpath`, `exec`) `set_pnbuf` one
//!   [`pathbuf_get`] made.
//! - `cnp->cn_cred->cr_uid` is [`cn_ucred`]`(cnp).cr_uid`: the kernel keeps the credential
//!   as the opaque `Cred` the vnode operations take.
//! - `nchinit` is called by the kernel's `vfsinit`, as the C's; `kern.nchstats` reads the
//!   kernel's counters ([`NCHSTATS`]).

use crate::kassert;
use crate::kern::kern_descrip::VFS_TABLES;
use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::ucred::Ucred;

pub use crate::sys::fcntl::AT_FDCWD;
pub use kernel::fs::cache::{
    Nchstats, cache_enter, cache_lookup, cache_purge, cache_purgevfs, cache_revlookup,
};
pub use kernel::fs::lookup::{
    component_pop, component_push, namei, ndinit, ndinitat, vfs_lookup, vfs_relookup,
};
pub use kernel::fs::namei::{
    BPU_LOCALTIME, BPU_ZONEINFO, BYPASSUNVEIL, CREATE, DELETE, EXECPATH, FOLLOW, HASBUF, ISDOTDOT,
    ISLASTCN, ISSYMLINK, KERN_NCHSTATS_2PASSES, KERN_NCHSTATS_BADHITS, KERN_NCHSTATS_DOTDOTHITS,
    KERN_NCHSTATS_DOTHITS, KERN_NCHSTATS_FALSEHITS, KERN_NCHSTATS_GOODHITS, KERN_NCHSTATS_LONG,
    KERN_NCHSTATS_MAXID, KERN_NCHSTATS_MISS, KERN_NCHSTATS_NEGHITS, KERN_NCHSTATS_PASS2,
    KERN_NCHSTATS_REVHITS, KERN_NCHSTATS_REVMISS, KERNELPATH, LOCKLEAF, LOCKPARENT, LOOKUP,
    MAKEENTRY, MODMASK, NAMECACHE_MAXLEN, NOCACHE, NOCROSSMOUNT, NOFOLLOW, NiDirp, OPMASK,
    PDIRUNLOCK, PathBuf, RDONLY, REALPATH, RENAME, REQUIREDIR, SAVENAME, SAVESTART, STRIPSLASHES,
    UNVEIL_CREATE, UNVEIL_EXEC, UNVEIL_MASK, UNVEIL_PLEDGEOPEN, UNVEIL_READ, UNVEIL_USERSET,
    UNVEIL_WRITE, WANTPARENT,
};

/// `struct componentname` of `bsd`'s threads.
pub type Componentname<'a> = kernel::fs::namei::Componentname<'a, BsdProcGlue>;

/// `struct nameidata` of `bsd`'s threads.
pub type Nameidata<'a> = kernel::fs::namei::Nameidata<'a, BsdProcGlue>;

/// `nchstats`: the name cache's statistics (the kernel's counters, `kern.nchstats`).
pub static NCHSTATS: &Nchstats = &VFS_TABLES.nchstats;

/// `pool_get(&namei_pool, PR_WAITOK)`: a pathname buffer, `None` when the pool fails.
pub fn pathbuf_get() -> Option<PathBuf> {
    kernel::fs::namei::pathbuf_get::<BsdProcGlue>()
}

/// `cnp->cn_cred` as the C dereferences it (`cnp->cn_cred->cr_uid`): the credential a
/// lookup always has (`namei` sets it to the thread's `p_ucred`, nfsd to the request's).
pub fn cn_ucred<'c>(cnp: &'c Componentname<'_>) -> &'c Ucred {
    let cred = cnp.cn_cred.as_raw();
    kassert!(!cred.is_null());
    // SAFETY: LZ's `Componentname::cred`, moved here with the credential's type: `namei` sets
    // `cn_cred` to the thread's `p_ucred` and nfsd to the request's credential, each held by
    // its holder for the lookup's duration, so the pointer is a live `Ucred` for the borrow
    // of `cnp`; NULL is refused above.
    unsafe { &*cred }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The constants against the C header; and the kernel's `namei` and `getcwd` over bsd's
    // glue and test file system (`vfs_subr.rs`), whose uiomove moves bytes: symbolic links
    // (relative, absolute, loops), the components and the root as a barrier, the locking and
    // reference protocol, `CREATE` of a missing name, `REALPATH`, the errors, and the
    // directory scan of `getcwd` (LZ's vfs_lookup.rs and vfs_getcwd.rs tests, kept here when
    // N5.3 moved their modules into the kernel, whose own tests run over a glue that copies
    // no kernel byte).

    use core::sync::atomic::Ordering;
    use std::{assert, assert_eq};

    use super::*;
    use crate::kern::vfs_init::rootvnode;
    use crate::kern::vfs_subr::tests::setup;
    use crate::kern::vfs_subr::tests::testfs::{
        self, A, ABS, B, C, ETCZ, L, LOCALTIME, LOOP, LT_DIR, LT_LINK, LT_UP, N, ROOT, UTC, ZLINK,
        any_locked, node_of, usecounts, vget_node, vnode_of,
    };
    use crate::kern::vfs_vops::{VOP_ABORTOP, VOP_ISLOCKED, VOP_UNLOCK};
    use crate::sys::errno::Errno;
    use crate::sys::param::MAXPATHLEN;
    use crate::sys::proc::Proc;
    use crate::sys::vnode::{
        GETCWD_CHECK_ACCESS, VREG, Vnode, vfs_getcwd_common, vn_isunder, vput, vrele,
    };

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/sys/namei.h");
        for (name, value) in [
            ("LOCKLEAF", LOCKLEAF),
            ("WANTPARENT", WANTPARENT),
            ("FOLLOW", FOLLOW),
            ("MODMASK", MODMASK),
            ("NOCROSSMOUNT", NOCROSSMOUNT),
            ("SAVESTART", SAVESTART),
            ("ISSYMLINK", ISSYMLINK),
            ("STRIPSLASHES", STRIPSLASHES),
            ("PDIRUNLOCK", PDIRUNLOCK),
            ("KERNELPATH", KERNELPATH),
            ("EXECPATH", EXECPATH),
            ("NAMECACHE_MAXLEN", NAMECACHE_MAXLEN as u64),
        ] {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(value as i64),
                "{name}"
            );
        }
        assert_eq!(
            crate::reftest::int(&defs, "AT_FDCWD"),
            Some(i64::from(AT_FDCWD))
        );
        assert_eq!(
            crate::reftest::int(&defs, "UNVEIL_PLEDGEOPEN"),
            Some(i64::from(UNVEIL_PLEDGEOPEN))
        );
    }

    /// `namei` of a kernel path, the result reduced to the node it found.
    fn lookup(
        p: &'static Proc,
        path: &[u8],
        op: u64,
        flags: u64,
    ) -> Result<Nameidata<'static>, Errno> {
        let path: &'static [u8] = std::boxed::Box::leak(path.to_vec().into_boxed_slice());
        let mut nd = ndinit(op, flags, NiDirp::Sys(path), p);
        namei(&mut nd).map(|()| nd)
    }

    /// Looks `path` up with `LOOKUP` and `flags`, checks it found node `want`, releases it and
    /// checks nothing leaked.
    fn finds(p: &'static Proc, path: &[u8], flags: u64, want: usize) {
        let before = usecounts();
        let nd = lookup(p, path, LOOKUP, flags).unwrap_or_else(|e| {
            std::panic!("{:?}: error {e:?}", core::str::from_utf8(path));
        });
        let vp = nd.ni_vp.unwrap();
        assert_eq!(node_of(vp), want, "{:?}", core::str::from_utf8(path));
        if flags & LOCKLEAF != 0 {
            assert_eq!(VOP_ISLOCKED(vp), 1);
            vput(vp);
        } else {
            assert!(
                !any_locked(),
                "{:?} left a lock",
                core::str::from_utf8(path)
            );
            vrele(vp);
        }
        assert!(!any_locked());
        // Nodes looked up for the first time now have a vnode on the free list (count 0).
        let after = usecounts();
        for i in 0..N {
            assert_eq!(after[i], before[i], "use count of node {i} after {path:?}");
        }
    }

    #[test]
    fn absolute_and_relative_paths() {
        let (_g, p, _mp) = testfs::setup_root();
        finds(p, b"/", FOLLOW, ROOT);
        finds(p, b"/a", FOLLOW, A);
        finds(p, b"/a/b", FOLLOW, B);
        finds(p, b"/a/b", FOLLOW | LOCKLEAF, B);
        finds(p, b"a/c", FOLLOW, C);
        finds(p, b"./a/./c/.", FOLLOW, C);
    }

    #[test]
    fn slashes_split_components() {
        let (_g, p, _mp) = testfs::setup_root();
        finds(p, b"//a///c", FOLLOW, C);
        finds(p, b"/a/c/", FOLLOW, C);
        finds(p, b"/a/c//", FOLLOW | LOCKLEAF, C);
        // A trailing slash requires a directory.
        assert_eq!(
            lookup(p, b"/a/b/", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOTDIR)
        );
        // A component under a file.
        assert_eq!(
            lookup(p, b"/a/b/x", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOTDIR)
        );
        assert!(!any_locked());
    }

    #[test]
    fn dot_dot_climbs_and_stops_at_the_root() {
        let (_g, p, _mp) = testfs::setup_root();
        finds(p, b"/a/c/..", FOLLOW, A);
        finds(p, b"/a/../a/b", FOLLOW, B);
        finds(p, b"/..", FOLLOW, ROOT);
        finds(p, b"../../a", FOLLOW, A);
        finds(p, b"/a/c/../../a/c", FOLLOW | LOCKLEAF, C);
    }

    #[test]
    fn errors() {
        let (_g, p, _mp) = testfs::setup_root();
        let before = usecounts();
        assert_eq!(lookup(p, b"", LOOKUP, FOLLOW).err(), Some(Errno::ENOENT));
        assert_eq!(
            lookup(p, b"/nope", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOENT)
        );
        assert_eq!(
            lookup(p, b"/a/nope/b", LOOKUP, FOLLOW).err(),
            Some(Errno::ENOENT)
        );
        let long = [b'y'; 300];
        assert_eq!(
            lookup(p, &long, LOOKUP, FOLLOW).err(),
            Some(Errno::ENAMETOOLONG)
        );
        let toolong = [b'z'; MAXPATHLEN + 10];
        assert_eq!(
            lookup(p, &toolong, LOOKUP, FOLLOW).err(),
            Some(Errno::ENAMETOOLONG)
        );
        assert!(!any_locked());
        assert_eq!(usecounts()[ROOT], before[ROOT]);
    }

    #[test]
    fn symbolic_links() {
        let (_g, p, _mp) = testfs::setup_root();
        // Relative to the link's directory, absolute from the root.
        finds(p, b"/l", FOLLOW, C);
        finds(p, b"/l/..", FOLLOW, A);
        finds(p, b"/a/abs", FOLLOW | LOCKLEAF, B);
        // The last component is not followed without FOLLOW; the others always are.
        finds(p, b"/l", NOFOLLOW, L);
        finds(p, b"/a/abs", NOFOLLOW, ABS);
        finds(p, b"/l/", NOFOLLOW, C);
        finds(p, b"l/..", NOFOLLOW, A);
        assert_eq!(
            lookup(p, b"/loop", LOOKUP, FOLLOW).err(),
            Some(Errno::ELOOP)
        );
        finds(p, b"/loop", NOFOLLOW, LOOP);
        assert!(!any_locked());
    }

    /// `__pledge_open`'s /etc/localtime (vfs_lookup.c:315-326, the symbolic link check and
    /// `terminal:`): with BPU_LOCALTIME a link must point into /usr/share/zoneinfo/ without a
    /// `..` (kern_pledge's checkzoneinfopath, EACCES otherwise); the walk then goes on under
    /// BPU_ZONEINFO, which refuses another symbolic link (ELOOP) and a last component that
    /// is not a regular file (EACCES). Without the flags the same links resolve.
    #[test]
    fn localtime_follows_the_zoneinfo_rules() {
        let (_g, p, _mp) = testfs::setup_root();
        let before = usecounts();
        finds(p, b"/etc/localtime", FOLLOW | BPU_LOCALTIME, UTC);
        // Not a link: the flag has nothing to check.
        finds(p, b"/usr/share/zoneinfo/UTC", FOLLOW | BPU_LOCALTIME, UTC);
        for (path, flags, want) in [
            // a link out of zoneinfo, or through a `..`
            (&b"/a/abs"[..], FOLLOW | BPU_LOCALTIME, Errno::EACCES),
            (b"/etc/lt-up", FOLLOW | BPU_LOCALTIME, Errno::EACCES),
            // zoneinfo allows no symbolic link
            (b"/etc/lt-link", FOLLOW | BPU_LOCALTIME, Errno::ELOOP),
            (
                b"/usr/share/zoneinfo/link",
                FOLLOW | BPU_ZONEINFO,
                Errno::ELOOP,
            ),
            // only a regular file at the end
            (b"/etc/lt-dir", FOLLOW | BPU_LOCALTIME, Errno::EACCES),
            (
                b"/usr/share/zoneinfo/Etc",
                FOLLOW | BPU_ZONEINFO,
                Errno::EACCES,
            ),
        ] {
            assert_eq!(
                lookup(p, path, LOOKUP, flags).err(),
                Some(want),
                "{:?}",
                core::str::from_utf8(path)
            );
            assert!(!any_locked(), "{:?}", core::str::from_utf8(path));
        }
        // Without the flags, the links resolve as any.
        finds(p, b"/etc/lt-link", FOLLOW, UTC);
        finds(p, b"/etc/lt-dir", FOLLOW, ETCZ);
        finds(p, b"/etc/lt-up", FOLLOW, UTC);
        finds(p, b"/usr/share/zoneinfo/link", NOFOLLOW, ZLINK);
        finds(p, b"/etc/localtime", NOFOLLOW, LOCALTIME);
        finds(p, b"/etc/lt-dir", NOFOLLOW, LT_DIR);
        finds(p, b"/etc/lt-link", NOFOLLOW, LT_LINK);
        finds(p, b"/etc/lt-up", NOFOLLOW, LT_UP);
        assert_eq!(usecounts()[ROOT], before[ROOT]);
    }

    #[test]
    fn lockparent_returns_both_vnodes_locked() {
        let (_g, p, _mp) = testfs::setup_root();
        let before = usecounts();
        let nd = lookup(p, b"/a/b", LOOKUP, LOCKPARENT | LOCKLEAF).unwrap();
        let (vp, dvp) = (nd.ni_vp.unwrap(), nd.ni_dvp.unwrap());
        assert_eq!((node_of(dvp), node_of(vp)), (A, B));
        assert_eq!((VOP_ISLOCKED(dvp), VOP_ISLOCKED(vp)), (1, 1));
        vput(vp);
        vput(dvp);
        assert!(!any_locked());
        assert_eq!(usecounts()[ROOT], before[ROOT]);

        // WANTPARENT: the parent comes back referenced but unlocked.
        let nd = lookup(p, b"/a/c", LOOKUP, WANTPARENT).unwrap();
        let (vp, dvp) = (nd.ni_vp.unwrap(), nd.ni_dvp.unwrap());
        assert_eq!((node_of(dvp), node_of(vp)), (A, C));
        assert!(!any_locked());
        vrele(vp);
        vrele(dvp);
    }

    #[test]
    fn create_of_a_missing_name_returns_the_locked_parent() {
        let (_g, p, _mp) = testfs::setup_root();
        let before = usecounts();
        let mut nd = lookup(p, b"/a/new", CREATE, LOCKPARENT | SAVENAME).unwrap();
        assert!(nd.ni_vp.is_none());
        let dvp = nd.ni_dvp.unwrap();
        assert_eq!(node_of(dvp), A);
        assert_eq!(VOP_ISLOCKED(dvp), 1);
        // SAVENAME keeps the pathname buffer and the last component for the caller.
        assert!(nd.ni_cnd.cn_flags & HASBUF != 0);
        assert_eq!(nd.ni_cnd.name(), b"new");
        let _ = VOP_ABORTOP(dvp, &mut nd.ni_cnd);
        vput(dvp);
        assert!(!any_locked());
        assert_eq!(usecounts()[A], before[A]);

        // A missing intermediate directory is an error, not a creation.
        assert_eq!(
            lookup(p, b"/nope/new", CREATE, LOCKPARENT).err(),
            Some(Errno::ENOENT)
        );
        // Neither is a trailing slash on a missing name.
        assert_eq!(
            lookup(p, b"/a/new/", CREATE, LOCKPARENT).err(),
            Some(Errno::ENOENT)
        );
        // The root has no parent to create in.
        assert_eq!(
            lookup(p, b"/", CREATE, LOCKPARENT).err(),
            Some(Errno::EISDIR)
        );
        assert!(!any_locked());
    }

    #[test]
    fn realpath_builds_the_canonical_name() {
        let (_g, p, _mp) = testfs::setup_root();
        let path: &'static [u8] = b"/a/./c/../abs";
        let mut nd = ndinit(LOOKUP, FOLLOW | SAVENAME | REALPATH, NiDirp::Sys(path), p);
        nd.ni_cnd.cn_rpbuf = pathbuf_get();
        nd.ni_cnd.cn_rpi = 0;
        namei(&mut nd).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert_eq!(node_of(vp), B);
        let rpbuf = nd.ni_cnd.rpbuf().unwrap();
        let len = rpbuf.iter().position(|&c| c == 0).unwrap();
        assert_eq!(&rpbuf[..len], b"/a/b");
        vrele(vp);
        drop(nd.ni_cnd.take_pnbuf());
    }

    #[test]
    fn stripslashes_drops_the_trailing_slashes_first() {
        let (_g, p, _mp) = testfs::setup_root();
        let nd = lookup(p, b"/a/b///", LOOKUP, FOLLOW | STRIPSLASHES).unwrap();
        let vp = nd.ni_vp.unwrap();
        assert_eq!(node_of(vp), B);
        assert_eq!(vp.vtype(), VREG);
        vrele(vp);
    }

    #[test]
    fn no_root_file_system_is_enoent() {
        let (_g, p) = setup();
        assert_eq!(lookup(p, b"/a", LOOKUP, FOLLOW).err(), Some(Errno::ENOENT));
        assert_eq!(lookup(p, b"a", LOOKUP, FOLLOW).err(), Some(Errno::ENOENT));
        assert!(vnode_of(ROOT).is_none());
    }

    fn cwd(p: &Proc, vp: &'static Vnode) -> std::vec::Vec<u8> {
        let mut buf = [0u8; 64];
        let mut bp = buf.len() - 1;
        let lim = 32;
        vfs_getcwd_common(
            vp,
            None,
            Some((&mut buf, &mut bp)),
            lim,
            GETCWD_CHECK_ACCESS,
            p,
        )
        .unwrap();
        buf[bp..buf.len() - 1].to_vec()
    }

    #[test]
    fn getcwd_by_directory_scan_then_by_cache() {
        let (_g, p, mp) = testfs::setup_root();
        let c = vget_node(mp, C).unwrap();
        let _ = VOP_UNLOCK(c);
        // Nothing cached: each step looks ".." up and scans it.
        assert_eq!(cwd(p, c), b"/a/c");
        assert!(!any_locked());

        // After a lookup the reverse map answers.
        let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/a/c"), p);
        namei(&mut nd).unwrap();
        vrele(nd.ni_vp.unwrap());
        let revhits = NCHSTATS.ncs_revhits.load(Ordering::SeqCst);
        assert_eq!(cwd(p, c), b"/a/c");
        assert_eq!(NCHSTATS.ncs_revhits.load(Ordering::SeqCst), revhits + 2);

        let root = rootvnode().unwrap();
        assert_eq!(cwd(p, root), b"/");
        assert!(vn_isunder(c, root, p));
        let a = vget_node(mp, A).unwrap();
        let _ = VOP_UNLOCK(a);
        assert!(vn_isunder(c, a, p));
        assert!(!vn_isunder(a, c, p));
        vrele(a);
        vrele(c);
        assert!(!any_locked());
    }
}
/* </TESTS> */
