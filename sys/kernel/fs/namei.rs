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
//! `<sys/namei.h>`: the encapsulation of `namei` parameters (`struct nameidata` and the
//! `struct componentname` it hands to `VOP_LOOKUP`), the namei operations and flags, the
//! pathname buffers (`namei_pool`), the unveil flags and `kern.nchstats`' names.
//!
//! Upstream: sys/sys/namei.h @ 3ce1f3f79392
//! LZ: sys/sys/namei.rs@3c62ede68802
//!
//! ## Deviations
//! - `ni_dirp` is a [`NiDirp`]: a user address or a kernel byte string; `ni_segflg` is kept
//!   and always agrees with it (`ndinitat` sets both). `NDINITAT`/`NDINIT` are
//!   [`super::lookup::ndinitat`]/[`super::lookup::ndinit`], which return the structure
//!   instead of filling one (LZ's).
//! - The prototypes are their functions in `fs::lookup`, `fs::cache` and (the `unveil_*`
//!   ones) the glue's hooks (`kern_unveil.c` is `bsd`'s until N8).
//!
//! ## Redesign
//! - LZ's `cn_pnbuf`, `cn_rpbuf`, `cn_nameptr` and `ni_next` as raw pointers walked with
//!   pointer arithmetic, with the contract "whoever sets `cn_nameptr` makes it point at
//!   `cn_namelen` readable bytes for the lookup's duration" (sys/sys/namei.rs:49-53, three
//!   `unsafe` derefs) -> the pathname and realpath buffers **owned** by the componentname
//!   ([`PathBuf`], a `PoolBox` of the `"namei"` pool: its drop is the C's `pool_put`), the
//!   component an offset and a length into the pathname buffer or a `'static` literal (the
//!   C's `cn_nameptr = ".."`), `ni_next` an offset. A buffer is in one place at a time, which
//!   is how the C uses it: where the C copies a `cn_rpbuf` pointer into a second `nameidata`
//!   (`exec_script.c`), the buffer is moved there and back.
//! - `cn_proc` is `&'a Thread<G>` (the C's contract that the thread outlives its lookup, as a
//!   lifetime), `cn_cred` the opaque [`Cred`] the vnode operations take (`namei` sets it from
//!   the thread, as the C).
//! - `ni_unveil_match` is the glue's typed slot ([`FileGlue::UnveilMatch`]): the unveil table
//!   is `bsd`'s until N8; the kernel writes its `Default` where the C writes NULL.
//! - `struct namecache` and `struct nchstats` are `fs::cache`'s (the name cache's own types).
//! - A pathname buffer is initialised when the pool hands it out (1 KiB of zeroes: a Rust
//!   value has no uninitialised bytes), where the C's `pool_get` leaves it as it was; the
//!   copy-in that follows is the C's.

use hal::pool::{PR_WAITOK, PoolBox};
use hal::sys::errno::Errno;
use hal::sys::syslimits::PATH_MAX;

use super::vnode::Vnode;
use super::vops::Cred;
use crate::assert::kpanic;
use crate::file::FileGlue;
use crate::proc::{ProcGlue, Thread};
use crate::uio::UioSeg;

pub use hal::sys::fcntl::AT_FDCWD;

/// `MAXPATHLEN` (`<sys/param.h>`): the longest pathname, its NUL included (`PATH_MAX`).
pub const MAXPATHLEN: usize = PATH_MAX;

/// A pathname buffer (`MAXPATHLEN` bytes of `namei_pool`), owned: its drop is the C's
/// `pool_put(&namei_pool, buf)`.
pub type PathBuf = PoolBox<[u8; MAXPATHLEN]>;

/// `pool_get(&namei_pool, PR_WAITOK)`: a pathname buffer of the glue's pool (it sleeps for
/// one); `None` only before the pool is made.
pub fn pathbuf_get<G: ProcGlue>() -> Option<PathBuf> {
    <G::FileGlue as FileGlue>::vfs()
        .namei
        .get(PR_WAITOK, || [0; MAXPATHLEN])
}

/// `ni_dirp`: where the pathname is.
#[derive(Clone, Copy, Debug)]
pub enum NiDirp<'a> {
    /// A user address (`UIO_USERSPACE`).
    User(usize),
    /// A kernel string (`UIO_SYSSPACE`); it ends at its first NUL or at the slice's end.
    Sys(&'a [u8]),
}

/// `cn_nameptr`/`cn_namelen`: where the component being looked up is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CnName {
    /// None yet (`cn_nameptr == NULL`).
    None,
    /// `len` bytes at offset `off` of the pathname buffer (`off + len <= MAXPATHLEN`).
    InBuf { off: usize, len: usize },
    /// A string outside the buffer (the C's `cn_nameptr = ".."`).
    Lit(&'static [u8]),
}

/// `struct componentname`: the subset of the `nameidata` structure that is passed through
/// the VOP interface.
pub struct Componentname<'a, G: ProcGlue> {
    // Arguments to lookup.
    /// `cn_nameiop`: namei operation.
    pub cn_nameiop: u64,
    /// `cn_flags`: flags to namei.
    pub cn_flags: u64,
    /// `cn_proc`: process requesting lookup.
    pub cn_proc: &'a Thread<G>,
    /// `cn_cred`: credentials.
    pub cn_cred: Cred<'a>,
    // Shared between lookup and commit routines.
    /// `cn_pnbuf`: pathname buffer, `None` for NULL.
    pub cn_pnbuf: Option<PathBuf>,
    /// `cn_rpbuf`: realpath buffer, `None` for NULL.
    pub cn_rpbuf: Option<PathBuf>,
    /// `cn_rpi`: realpath index.
    pub cn_rpi: usize,
    /// `cn_nameptr` and `cn_namelen`: the component being looked up.
    name: CnName,
    /// `cn_consume`: chars to consume in lookup().
    pub cn_consume: i64,
}

impl<'a, G: ProcGlue> Componentname<'a, G> {
    /// A componentname of `p` for `op` with `flags` (`cn_nameiop`, `cn_flags`, `cn_proc`,
    /// `cn_cred`), no buffer and no component yet: what a caller that builds one by hand
    /// fills (the C's stack `struct componentname`).
    pub fn new(op: u64, flags: u64, p: &'a Thread<G>, cred: Cred<'a>) -> Self {
        Self {
            cn_nameiop: op,
            cn_flags: flags,
            cn_proc: p,
            cn_cred: cred,
            cn_pnbuf: None,
            cn_rpbuf: None,
            cn_rpi: 0,
            name: CnName::None,
            cn_consume: 0,
        }
    }

    /// The `".."` lookup `vfs_getcwd_scandir` (and unveil's cover search) makes: `LOOKUP`,
    /// `ISLASTCN | ISDOTDOT | RDONLY`, no pathname buffer, the name the literal `".."`.
    pub fn dotdot(p: &'a Thread<G>, cred: Cred<'a>) -> Self {
        let mut cn = Self::new(LOOKUP, ISLASTCN | ISDOTDOT | RDONLY, p, cred);
        cn.name = CnName::Lit(b"..");
        cn
    }

    /// `cn_proc`.
    #[inline]
    pub fn proc(&self) -> &'a Thread<G> {
        self.cn_proc
    }

    /// The component being looked up: the `cn_namelen` bytes at `cn_nameptr` (empty for
    /// none, or for an offset into a buffer that is gone).
    #[inline]
    pub fn name(&self) -> &[u8] {
        match self.name {
            CnName::None => &[],
            CnName::InBuf { off, len } => self
                .cn_pnbuf
                .as_deref()
                .map_or(&[][..], |b| &b[off..off + len]),
            CnName::Lit(s) => s,
        }
    }

    /// `cn_namelen`.
    #[inline]
    pub fn namelen(&self) -> i64 {
        match self.name {
            CnName::None => 0,
            CnName::InBuf { len, .. } => len as i64,
            CnName::Lit(s) => s.len() as i64,
        }
    }

    /// The bytes from `cn_nameptr` to the pathname's NUL (`strlen(cn_nameptr)` and what it
    /// measures: the rest of the path from this component on, unveil's and exec's reads); a
    /// literal's bytes for a name outside the buffer.
    pub fn name_rest(&self) -> &[u8] {
        match self.name {
            CnName::None => &[],
            CnName::InBuf { off, .. } => self.cn_pnbuf.as_deref().map_or(&[][..], |b| {
                let rest = &b[off..];
                &rest[..rest.iter().position(|&c| c == 0).unwrap_or(rest.len())]
            }),
            CnName::Lit(s) => s,
        }
    }

    /// The offset of `cn_nameptr` in the pathname buffer, `None` for a name outside it.
    #[inline]
    pub fn name_off(&self) -> Option<usize> {
        match self.name {
            CnName::InBuf { off, .. } => Some(off),
            _ => None,
        }
    }

    /// `cn_nameptr = cn_pnbuf + off; cn_namelen = len`: `panic(9)` past the buffer.
    #[inline]
    pub fn set_name_at(&mut self, off: usize, len: usize) {
        if off.checked_add(len).is_none_or(|end| end > MAXPATHLEN) {
            name_past_the_buffer(off, len);
        }
        self.name = CnName::InBuf { off, len };
    }

    /// `cn_namelen = len`, `cn_nameptr` where it is (in the buffer); `panic(9)` past the
    /// buffer or for a name that is not in it.
    pub fn set_namelen(&mut self, len: usize) {
        match self.name {
            CnName::InBuf { off, .. } => self.set_name_at(off, len),
            _ => kpanic(format_args!(
                "componentname: cn_namelen of a name outside cn_pnbuf"
            )),
        }
    }

    /// `cn_nameptr = s; cn_namelen = strlen(s)`: a name outside the pathname buffer.
    pub fn set_name_lit(&mut self, s: &'static [u8]) {
        self.name = CnName::Lit(s);
    }

    /// The pathname buffer.
    #[inline]
    pub fn pnbuf(&self) -> Option<&[u8; MAXPATHLEN]> {
        self.cn_pnbuf.as_deref()
    }

    /// The pathname buffer, to write.
    #[inline]
    pub fn pnbuf_mut(&mut self) -> Option<&mut [u8; MAXPATHLEN]> {
        self.cn_pnbuf.as_deref_mut()
    }

    /// The pathname buffer taken out (`cn_pnbuf = NULL` with the caller keeping it: its
    /// `pool_put` is the box's drop).
    pub fn take_pnbuf(&mut self) -> Option<PathBuf> {
        self.cn_pnbuf.take()
    }

    /// `cn_pnbuf = b` (a buffer this componentname did not have: the old one, if any, goes
    /// back to its pool).
    pub fn set_pnbuf(&mut self, b: PathBuf) {
        self.cn_pnbuf = Some(b);
    }

    /// The realpath buffer.
    #[inline]
    pub fn rpbuf(&self) -> Option<&[u8; MAXPATHLEN]> {
        self.cn_rpbuf.as_deref()
    }
}

/// [`Componentname::set_name_at`]'s refusal, out of the line.
#[cold]
#[inline(never)]
fn name_past_the_buffer(off: usize, len: usize) -> ! {
    kpanic(format_args!(
        "componentname: name at {off} of {len} bytes past the pathname buffer"
    ))
}

/// `struct nameidata`: encapsulation of namei parameters.
pub struct Nameidata<'a, G: ProcGlue> {
    // Arguments to namei/lookup.
    /// `ni_dirp`: pathname pointer.
    pub ni_dirp: NiDirp<'a>,
    /// `ni_dirfd`: dirfd from *at() functions.
    pub ni_dirfd: i32,
    /// `ni_segflg`: location of pathname.
    pub ni_segflg: UioSeg,
    // Arguments to lookup.
    /// `ni_startdir`: starting directory.
    pub ni_startdir: Option<&'static Vnode<G>>,
    /// `ni_rootdir`: logical root directory.
    pub ni_rootdir: Option<&'static Vnode<G>>,
    /// `ni_pledge`: expected pledge for namei.
    pub ni_pledge: u64,
    /// `ni_unveil`: required unveil flags for namei.
    pub ni_unveil: u8,
    // Results: returned from/manipulated by lookup.
    /// `ni_vp`: vnode of result.
    pub ni_vp: Option<&'static Vnode<G>>,
    /// `ni_dvp`: vnode of intermediate directory.
    pub ni_dvp: Option<&'static Vnode<G>>,
    // Shared between namei and lookup/commit routines.
    /// `ni_pathlen`: remaining chars in path.
    pub ni_pathlen: usize,
    /// `ni_next`: next location in pathname, an offset in `cn_pnbuf`.
    pub(super) ni_next: usize,
    /// `ni_loopcnt`: count of symlinks encountered.
    pub ni_loopcnt: u64,
    /// `ni_unveil_match`: last matching unveil component (the glue's; `Default` is NULL).
    pub ni_unveil_match: <G::FileGlue as FileGlue>::UnveilMatch,
    /// `ni_cnd`: lookup parameters.
    pub ni_cnd: Componentname<'a, G>,
}

impl<G: ProcGlue> Nameidata<'_, G> {
    /// `ni_next - cn_pnbuf`: the offset of the rest of the path after the component.
    pub fn ni_next(&self) -> usize {
        self.ni_next
    }

    /// The rest of the path after the component (`ni_next`, to the NUL).
    pub fn next_bytes(&self) -> &[u8] {
        self.ni_cnd.pnbuf().map_or(&[][..], |b| {
            let rest = &b[self.ni_next.min(MAXPATHLEN)..];
            &rest[..rest.iter().position(|&c| c == 0).unwrap_or(rest.len())]
        })
    }
}

/// `LOOKUP`: perform name lookup only.
pub const LOOKUP: u64 = 0;
/// `CREATE`: setup for file creation.
pub const CREATE: u64 = 1;
/// `DELETE`: setup for file deletion.
pub const DELETE: u64 = 2;
/// `RENAME`: setup for file renaming.
pub const RENAME: u64 = 3;
/// `OPMASK`: mask for operation.
pub const OPMASK: u64 = 3;

/// `LOCKLEAF`: lock inode on return.
pub const LOCKLEAF: u64 = 0x0004;
/// `LOCKPARENT`: want parent vnode returned locked.
pub const LOCKPARENT: u64 = 0x0008;
/// `WANTPARENT`: want parent vnode returned unlocked.
pub const WANTPARENT: u64 = 0x0010;
/// `NOCACHE`: name must not be left in cache.
pub const NOCACHE: u64 = 0x0020;
/// `FOLLOW`: follow symbolic links.
pub const FOLLOW: u64 = 0x0040;
/// `NOFOLLOW`: do not follow symbolic links (pseudo).
pub const NOFOLLOW: u64 = 0x0000;
/// `MODMASK`: mask of operational modifiers.
pub const MODMASK: u64 = 0x00fc;

/// `NOCROSSMOUNT`: do not cross mount points.
pub const NOCROSSMOUNT: u64 = 0x000100;
/// `RDONLY`: lookup with read-only semantics.
pub const RDONLY: u64 = 0x000200;
/// `HASBUF`: has allocated pathname buffer.
pub const HASBUF: u64 = 0x000400;
/// `SAVENAME`: save pathname buffer.
pub const SAVENAME: u64 = 0x000800;
/// `SAVESTART`: save starting directory.
pub const SAVESTART: u64 = 0x001000;
/// `ISDOTDOT`: current component name is `..`.
pub const ISDOTDOT: u64 = 0x002000;
/// `MAKEENTRY`: entry is to be added to name cache.
pub const MAKEENTRY: u64 = 0x004000;
/// `ISLASTCN`: this is last component of pathname.
pub const ISLASTCN: u64 = 0x008000;
/// `ISSYMLINK`: symlink needs interpretation.
pub const ISSYMLINK: u64 = 0x010000;
/// `REALPATH`: save pathname buffer for realpath.
pub const REALPATH: u64 = 0x020000;
/// `BPU_LOCALTIME`: /etc/localtime may cross 1 symlink.
pub const BPU_LOCALTIME: u64 = 0x040000;
/// `REQUIREDIR`: must be a directory.
pub const REQUIREDIR: u64 = 0x080000;
/// `STRIPSLASHES`: strip trailing slashes.
pub const STRIPSLASHES: u64 = 0x100000;
/// `PDIRUNLOCK`: vfs_lookup() unlocked parent dir.
pub const PDIRUNLOCK: u64 = 0x200000;
/// `BYPASSUNVEIL`: bypass pledgepath check.
pub const BYPASSUNVEIL: u64 = 0x400000;
/// `KERNELPATH`: access file as kernel, not process.
pub const KERNELPATH: u64 = 0x800000;
/// `BPU_ZONEINFO`: /usr/share/zoneinfo prohibits symlinks.
pub const BPU_ZONEINFO: u64 = 0x1000000;
/// `EXECPATH`: like `REALPATH`, but can give up.
pub const EXECPATH: u64 = 0x2000000;

/// `NAMECACHE_MAXLEN`: maximum name segment length we bother with.
pub const NAMECACHE_MAXLEN: usize = 31;

/// `KERN_NCHSTATS_GOODHITS`.
pub const KERN_NCHSTATS_GOODHITS: i32 = 1;
/// `KERN_NCHSTATS_NEGHITS`.
pub const KERN_NCHSTATS_NEGHITS: i32 = 2;
/// `KERN_NCHSTATS_BADHITS`.
pub const KERN_NCHSTATS_BADHITS: i32 = 3;
/// `KERN_NCHSTATS_FALSEHITS`.
pub const KERN_NCHSTATS_FALSEHITS: i32 = 4;
/// `KERN_NCHSTATS_MISS`.
pub const KERN_NCHSTATS_MISS: i32 = 5;
/// `KERN_NCHSTATS_LONG`.
pub const KERN_NCHSTATS_LONG: i32 = 6;
/// `KERN_NCHSTATS_PASS2`.
pub const KERN_NCHSTATS_PASS2: i32 = 7;
/// `KERN_NCHSTATS_2PASSES`.
pub const KERN_NCHSTATS_2PASSES: i32 = 8;
/// `KERN_NCHSTATS_REVHITS`.
pub const KERN_NCHSTATS_REVHITS: i32 = 9;
/// `KERN_NCHSTATS_REVMISS`.
pub const KERN_NCHSTATS_REVMISS: i32 = 10;
/// `KERN_NCHSTATS_DOTHITS`.
pub const KERN_NCHSTATS_DOTHITS: i32 = 11;
/// `KERN_NCHSTATS_DOTDOTHITS`.
pub const KERN_NCHSTATS_DOTDOTHITS: i32 = 12;
/// `KERN_NCHSTATS_MAXID`.
pub const KERN_NCHSTATS_MAXID: i32 = 13;

/// `UNVEIL_READ`.
pub const UNVEIL_READ: u8 = 0x01;
/// `UNVEIL_WRITE`.
pub const UNVEIL_WRITE: u8 = 0x02;
/// `UNVEIL_CREATE`.
pub const UNVEIL_CREATE: u8 = 0x04;
/// `UNVEIL_EXEC`.
pub const UNVEIL_EXEC: u8 = 0x08;
/// `UNVEIL_USERSET`.
pub const UNVEIL_USERSET: u8 = 0x10;
/// `UNVEIL_PLEDGEOPEN`.
pub const UNVEIL_PLEDGEOPEN: u8 = 0x20;
/// `UNVEIL_MASK`.
pub const UNVEIL_MASK: u8 = 0x0f;

/// The `Errno` of a lookup that cannot get a pathname buffer (the pool not made: before
/// `vfsinit`, never once the system runs).
pub(super) const NO_PATHBUF: Errno = Errno::ENOMEM;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::fs::testfs::{TestGlue, test_thread};

    #[test]
    fn a_component_is_an_offset_into_the_owned_buffer_or_a_literal() {
        let (_g, p) = test_thread();
        let mut cn = Componentname::<TestGlue>::new(LOOKUP, FOLLOW, p, Cred::real(p.ucred()));
        assert_eq!((cn.name(), cn.namelen()), (&b""[..], 0));
        let mut b = pathbuf_get::<TestGlue>().unwrap();
        b[..12].copy_from_slice(b"/usr/share\0x");
        cn.set_pnbuf(b);
        cn.set_name_at(5, 5);
        assert_eq!(cn.name(), b"share");
        assert_eq!(cn.namelen(), 5);
        assert_eq!(cn.name_off(), Some(5));
        assert_eq!(cn.name_rest(), b"share");
        cn.set_name_at(1, 3);
        assert_eq!(cn.name_rest(), b"usr/share");
        cn.set_namelen(9);
        assert_eq!(cn.name(), b"usr/share");
        // The name follows the buffer: taken out, the component reads empty.
        let b = cn.take_pnbuf().unwrap();
        assert_eq!(cn.name(), b"");
        cn.set_pnbuf(b);
        assert_eq!(cn.name(), b"usr/share");
        cn.set_name_lit(b"..");
        assert_eq!(
            (cn.name(), cn.namelen(), cn.name_off()),
            (&b".."[..], 2, None)
        );
        let dd = Componentname::<TestGlue>::dotdot(p, Cred::real(p.ucred()));
        assert_eq!(dd.name(), b"..");
        assert_eq!(dd.cn_flags, ISLASTCN | ISDOTDOT | RDONLY);
        assert_eq!(dd.cn_nameiop, LOOKUP);
        assert!(dd.pnbuf().is_none() && dd.rpbuf().is_none());
        assert!(core::ptr::eq(dd.proc(), p));
    }

    #[test]
    fn a_name_past_the_buffer_is_refused() {
        let (_g, p) = test_thread();
        let mut cn = Componentname::<TestGlue>::new(LOOKUP, 0, p, Cred::NOCRED);
        cn.set_name_at(MAXPATHLEN - 1, 1);
        let r = catch_unwind(AssertUnwindSafe(|| cn.set_name_at(MAXPATHLEN - 1, 2)));
        assert!(r.is_err());
        let r = catch_unwind(AssertUnwindSafe(|| cn.set_name_at(usize::MAX, 2)));
        assert!(r.is_err());
        cn.set_name_lit(b"x");
        let r = catch_unwind(AssertUnwindSafe(|| cn.set_namelen(1)));
        assert!(r.is_err());
    }

    #[test]
    fn values_match_the_lz_header() {
        // namei.h's values, as LZ's sys/sys/namei.rs had them (its reference test reads the
        // C header under `just test-ref`).
        assert_eq!(
            [
                LOCKLEAF,
                WANTPARENT,
                FOLLOW,
                MODMASK,
                NOCROSSMOUNT,
                SAVESTART,
                ISSYMLINK
            ],
            [0x4, 0x10, 0x40, 0xfc, 0x100, 0x1000, 0x10000]
        );
        assert_eq!(
            [STRIPSLASHES, PDIRUNLOCK, KERNELPATH, EXECPATH],
            [0x10_0000, 0x20_0000, 0x80_0000, 0x200_0000]
        );
        assert_eq!(
            (NAMECACHE_MAXLEN, AT_FDCWD, UNVEIL_PLEDGEOPEN),
            (31, -100, 0x20)
        );
        assert_eq!(MAXPATHLEN, 1024);
    }
}
/* </TESTS> */
