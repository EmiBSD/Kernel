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
 * Copyright (c) 2001, 2002 Scott Long <scottl@freebsd.org>
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: src/sys/fs/udf/udf_vnops.c,v 1.50 2005/01/28 14:42:16 phk Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! UDF vnode operations (`udf_vnops`): the table of a UDF vnode ([`UdfVnops`]), the unode
//! hash, attributes and access, `read`, `readdir` and `lookup` over the file identifier
//! descriptors of a directory, the block map and the strategy routine, the vnode lock (the
//! unode's recursive rwlock), the time stamps, the names, and the read of a node's data at an
//! offset through its block map.
//!
//! Upstream: sys/isofs/udf/udf_vnops.c @ 3ce1f3f79392
//! LZ: sys/isofs/udf/udf_vnops.rs@3c62ede68802
//!
//! Ported to OpenBSD by Pedro Martelletto in February 2005.
//!
//! ## Deviations
//! - `udf_readatoffset` returns where its data is ([`UdfData`], with the buffer's token, the
//!   C's `*bp`; on an error the buffer is released, as every C caller does). The size it
//!   returns is clamped to what the buffer (or the file entry) holds past the data's start:
//!   the C may hand out bytes past the buffer when the offset is not block aligned (LZ's).
//! - For a file recorded in its file entry, `udf_readatoffset` returns the data from
//!   `offset` on; the C returns it from its start whatever the offset, so a `read` that does
//!   not start at 0, a directory read resumed past its start and a VAT lookup past the first
//!   entry got the wrong bytes (LZ's).
//! - `udf_bmap_internal` returns [`UdfBmap`]: a block, or `UDF_INVALID_BMAP` (the file data
//!   is in the file entry) as its own variant; `udf_bmap` passes that case on as the C's -1,
//!   `ERESTART`'s value ([`UDF_INVALID_BMAP`]). An allocation descriptor read past the end of
//!   the file entry's copy reads zeros (the C reads past an extended file entry's
//!   allocation). The descriptor offset is computed in 64 bits, so a huge `l_ad` ends the
//!   walk with `EINVAL` where the C's `int` would overflow; a crafted `l_ad` near 2^31 still
//!   walks up to 2^28 zero-length descriptors before it, as the C's walk (not bounded).
//! - `udf_transname` returns 0 (failure) when `destname` cannot hold the name and its NUL;
//!   `udf_cmpname` returns a `bool` (`true` where the C returns nonzero: no match) (LZ's).
//! - `udf_uiodir` returns `Ok(false)` where the C returns -1 (no room for the entry); where
//!   the C's `readdir` breaks out with that -1 (the "." entry) and returns it, this returns
//!   `ERESTART`, its value (LZ's).
//! - The directory stream's fragment buffer is freed when the stream is (`udf_closedir`) on
//!   every path; the C leaks it when reading the next extent fails or the FID is too long. A
//!   FID that would start past the end of its chunk is an invalid fragment (`EINVAL`), as one
//!   longer than a block is (LZ's).
//! - `udf_read` stops on a zero-length chunk (an embedded file whose allocation descriptor
//!   area is empty), where the C would loop for ever (LZ's).
//! - `udf_timetotimespec` ignores months past December (the C reads past `mon_lens`) (LZ's).
//! - `udf_strategy` gives the buffer the device's number (`u_devvp`'s `v_rdev`); the C reads
//!   `v_rdev` of the file's own vnode, which has no `specinfo` (LZ's). A buffer handed to it
//!   is a token (`IoBuf::into_token`; `panic(9)` on a lent one: no `bsd` caller lends one to
//!   a UDF vnode).
//! - `udf_getattr`'s `vp->v_type & VDIR` keeps the C's bitwise test of the enumeration
//!   (`VBLK`, `VSOCK` and `VFIFO` share `VDIR`'s bit) (LZ's).
//! - `udf_print` prints under feature `debug` or `diagnostic` (`VFSLCKDEBUG` has none); the
//!   `DIAGNOSTIC` messages of `udf_transname` under feature `diagnostic` (LZ's).
//! - The C's NULL slots of `udf_vops` (`kqfilter` among them: udf has no filters) and its
//!   `eopnotsupp` `remove` are the `Vnops` trait's defaults, which answer `EOPNOTSUPP` as the
//!   C's dispatch of a NULL slot and `eopnotsupp` do.
//!
//! ## Redesign
//! - LZ's `UDF_VOPS` (a `bsd` `Vops` of function pointers over argument structures) ->
//!   [`UdfVnops`], one `impl Vnops<G>` of the C's 34 slots (decision 46), the member `vops`
//!   of the glue's `UdfTables` (one address per glue for `vops_id`; a byte so that it is not
//!   zero-sized, D-b2-10). Device files keep it as the C's do, unless `checkalias` hands
//!   `udf_vget` an alias (the spec table, `udf_vfsops.rs`).
//! - LZ's hash (a raw `hashinit` slice of `sys/queue.rs` chains linked and unlinked in
//!   `unsafe` blocks under a legacy `Mutex`) -> the mount's `UdfHash` of pinned unodes
//!   (`PoolArc::as_pin`) under `um_hashmtx`, a `hal::mutex::Mutexed<()>` at `IPL_NONE` whose
//!   guard is the chains' witness (the C's mutex, never held across `vget` or a sleep:
//!   `udf_hashlookup` drops it before `vget` and walks again on `ENOENT` or a recycled vnode,
//!   udf_vnops.c:113-140); `udf_hashins` inserts without a duplicate scan, as the C (two
//!   `vget`s of one file may hash two unodes: the C's race, kept, not tested: the host tests
//!   are single-threaded).
//! - LZ's lent buffers (`udf_data`, an `unsafe fn` over a buffer's bytes) and `fentry_read`
//!   over the unode's raw file entry -> [`UdfData::bytes`] over the token or the unode's
//!   `MArray`; `udf_read` moves buffer data out of the token's bytes (no bounce), and a file
//!   recorded in its entry through LZ's 256-byte bounce (the entry is shared, `uiomove` wants
//!   a writable source: a copy per 256 bytes, ROADMAP N12's row). `udf_bmap_internal` reads
//!   the members of the parsed `FentryHdr` and the allocation descriptors out of the copy at
//!   the C's offsets (8 or 16 bytes a step, from the first on every call, as the C's walk;
//!   D-N5.8-5).
//! - The directory stream ([`udf_opendir`]) is a `PoolBox` of the glue's `ds_pool` holding
//!   counted handles of the directory's unode and of the mount (a count pair each per
//!   `readdir` and `lookup`, ROADMAP N12's row), its chunk a `UdfData` (the token of the
//!   chunk's buffer) and its fragment buffer an `MArray` of `M_UDFFID`; dropping it is
//!   `udf_closedir` (the buffer released, the fragment freed, the item put back). LZ wrote it
//!   into a raw pool item and lent `&'static mut` views of it (three `unsafe`).
//! - `readdir`'s entry is one `struct dirent` reused across the directory, as the C's, copied
//!   out through `fs::subr::dirent_uiomove` (its first `d_reclen` bytes, which
//!   `uiomove(dirent, de_size, uio)` copies: what an earlier, longer name left past the NUL
//!   goes with it, as in the C, where LZ zeroed it).
//! - LZ's name buffers (a raw `udf_trans_pool` item cast to a slice in an `unsafe` block)
//!   -> `PoolBox`es of the glue's `trans_pool` (`udf_cmpname`'s char buffer is its item's
//!   bytes, `hal::user::bytes_of_mut`). The item is zeroed when taken (the C's `PR_WAITOK`
//!   leaves it as it was: 510 bytes written per name, a cost named for ROADMAP N12).
//! - `udf_reclaim` takes the unode out of the vnode's slot where the C frees it: the last
//!   handle frees it, which is the VAT block's for the VAT's unode until the unmount takes
//!   the block (`udf.rs`). `VTOU` is a counted handle per operation (ROADMAP N12's row).
//!   0 `unsafe` (LZ: 18).
//!
//! [`UdfData::bytes`]: crate::fs::isofs::udf::udf::UdfData::bytes

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use hal::malloc::{M_WAITOK, MArray};
use hal::pool::{PR_WAITOK, PoolBox};
use hal::sys::dirent::{DT_DIR, DT_UNKNOWN, Dirent, MAXNAMLEN, dirent_size};
use hal::sys::endian::{letoh16, letoh32, letoh64};
use hal::sys::errno::Errno;
use hal::sys::syslimits::NAME_MAX;
use hal::sys::types::{Daddr, Ino, Mode, Nlink, Off, Register};
use hal::sys::unistd::{
    _PC_CHOWN_RESTRICTED, _PC_LINK_MAX, _PC_NAME_MAX, _PC_NO_TRUNC, _PC_TIMESTAMP_RESOLUTION,
};
use hal::user::bytes_of_mut;

use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::file::FileGlue;
use crate::fs::bio::{DEV_BSHIFT, biodone};
use crate::fs::buf::{B_ERROR, BufRef};
use crate::fs::cache::{cache_enter, cache_lookup};
use crate::fs::isofs::udf::ecma167_udf::{
    FileidDesc, LongAd, ShortAd, SpareMapEntry, TAGID_FID, Timestamp, UDF_FENTRY_PERM_GRP_MASK,
    UDF_FENTRY_PERM_OWNER_MASK, UDF_FENTRY_PERM_USER_MASK, UDF_FID_SIZE, UDF_FILE_CHAR_DEL,
    UDF_FILE_CHAR_DIR, UDF_FILE_CHAR_PAR, UDF_ICB_TAG_FLAGS_SETGID, UDF_ICB_TAG_FLAGS_SETUID,
    UDF_ICB_TAG_FLAGS_STICKY, UdfSparingTable, geticblen,
};
use crate::fs::isofs::udf::udf::{
    M_UDFFID, MAXBSIZE, UDF_MNT_USES_META, UdfData, UdfDirstream, Udfino, Umount, Unode, UnodeRef,
    VTOU, lblk_bytes, udf_getid, udf_readlblks,
};
use crate::fs::isofs::udf::udf_subr::udf_rawnametounicode;
use crate::fs::isofs::udf::udf_vfsops::{udf_checktag, udf_vget};
use crate::fs::mount::Mount;
use crate::fs::namei::{
    CREATE, ISDOTDOT, ISLASTCN, LOCKPARENT, LOOKUP, MAKEENTRY, PDIRUNLOCK, RENAME,
};
use crate::fs::subr::{dirent_uiomove, getnewvnode, vaccess, vfs, vget, vput, vref, vrele};
use crate::fs::ufs::ufs::ufs_vnops::ucred;
use crate::fs::vnode::{
    LK_EXCLUSIVE, LK_RETRY, LK_RWFLAGS, VDIR, VEXEC, VLNK, VREG, VT_UDF, VWRITE, Vattr, Vnode,
    vn_lock,
};
use crate::fs::vops::{Cn, Cred, IoBuf, Vnops, vop_access, vop_bmap, vop_strategy, vop_unlock};
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::uio::Uio;

/// `UDF_INVALID_BMAP`: the C's -1, `ERESTART`'s value (the module's deviations).
pub const UDF_INVALID_BMAP: Errno = Errno::ERESTART;

/// `SELF_OFFSET`: the `d_off` of the "." entry (a dummy: "NFS doesn't like" 0).
const SELF_OFFSET: i64 = 1;
/// `PARENT_OFFSET`: the `d_off` of the ".." entry.
const PARENT_OFFSET: i64 = 2;

/// `udf_vops`: the operations of a vnode of a UDF volume (the glue's `UdfTables::vops`, one
/// address).
pub struct UdfVnops<G: ProcGlue> {
    /// A byte, so the table is no zero-sized member: the registry tells tables apart by
    /// their addresses.
    _id: u8,
    _g: PhantomData<fn() -> G>,
}

impl<G: ProcGlue> UdfVnops<G> {
    /// The table (the glue's `UdfTables::vops`).
    pub(crate) const fn new() -> Self {
        Self {
            _id: 0,
            _g: PhantomData,
        }
    }
}

/// What `udf_bmap_internal` found for an offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UdfBmap {
    /// The physical sector (`*sector`) and the length of its extent (`*max_size`).
    Block {
        /// `*sector`.
        sector: Daddr,
        /// `*max_size`.
        max_size: u32,
    },
    /// `UDF_INVALID_BMAP`: the file data is stored in the allocation descriptor field of the
    /// file entry, recorded at `*sector` (`*max_size` is 0).
    Invalid {
        /// `*sector`: the file entry's.
        sector: Daddr,
    },
}

/// `struct udf_uiodir`: `readdir`'s entry and its end-of-file flag.
struct UdfUiodir {
    /// `dirent`: the entry being built (the C's `dir`, which it points at).
    dirent: Dirent,
    /// `eofflag`.
    eofflag: i32,
}

/// Where `udf_readdir` starts: from the beginning, or at "." or "..".
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReaddirMode {
    /// `MODE_NORMAL`.
    Normal,
    /// `MODE_SELF`.
    SelfEntry,
    /// `MODE_PARENT`.
    Parent,
}

/// The month lengths of a common and of a leap year.
static MON_LENS: [[i32; 12]; 2] = [
    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
];

/// Look up a unode based on the `udfino_t` passed in and return its vnode, referenced and
/// locked with `flags`; `None` when it is not in the cache, `ENOENT` when the mount has no
/// table yet (the C's `NULL` chain).
pub fn udf_hashlookup<G: ProcGlue>(
    ump: &Umount<G>,
    id: Udfino,
    flags: i32,
) -> Result<Option<&'static Vnode<G>>, Errno> {
    loop {
        let found = {
            let g = ump.um_hashmtx.lock();
            let Some(h) = ump.hashtbl() else {
                return Err(Errno::ENOENT);
            };
            h.chain(id).iter(&g).find(|up| up.u_ino == id).map(|up| {
                let vp = up.vnode();
                (vp, vp.id())
            })
        };
        // The mutex is left before `vget`, which may sleep.
        let Some((vp, vpid)) = found else {
            return Ok(None);
        };
        match vget(vp, flags) {
            Err(Errno::ENOENT) => continue,
            Err(e) => return Err(e),
            Ok(()) => {}
        }
        if vpid != vp.id() {
            vput(vp);
            continue;
        }
        return Ok(Some(vp));
    }
}

/// `udf_hashins(up)`: lock the unode's vnode and put the unode on its hash chain.
pub fn udf_hashins<G: ProcGlue>(up: &UnodeRef<G>) -> Result<(), Errno> {
    let ump = &up.u_ump;

    let _ = vn_lock(up.vnode(), LK_EXCLUSIVE | LK_RETRY);
    let mut g = ump.um_hashmtx.lock();
    let Some(h) = ump.hashtbl() else {
        kpanic(format_args!("hash entry is NULL, up->u_ino = {}", up.u_ino));
    };
    h.chain(up.u_ino).insert_head(&mut g, up.as_pin());
    drop(g);

    Ok(())
}

/// `udf_hashrem(up)`: take the unode off its hash chain.
pub fn udf_hashrem<G: ProcGlue>(up: &Unode<G>) -> Result<(), Errno> {
    let ump = &up.u_ump;

    let mut g = ump.um_hashmtx.lock();
    let Some(h) = ump.hashtbl() else {
        kpanic(format_args!("hash entry is NULL, up->u_ino = {}", up.u_ino));
    };
    h.chain(up.u_ino).remove(&mut g, up);
    drop(g);

    Ok(())
}

/// `udf_allocv(mp, vpp, p)`: a new vnode of `mp` with udf's table.
pub fn udf_allocv<G: ProcGlue>(mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
    match getnewvnode(VT_UDF, Some(mp), &vfs::<G>().udf.vops) {
        Ok(vp) => Ok(vp),
        Err(e) => {
            printf(format_args!("udf_allocv: failed to allocate new vnode\n"));
            Err(e)
        }
    }
}

/// Convert file entry permission (5 bits per owner/group/user) to a `mode_t`.
fn udf_permtomode<G: ProcGlue>(up: &Unode<G>) -> Mode {
    let perm = letoh32(up.fe.perm);
    let flags = u32::from(letoh16(up.fe.icbtag.flags));

    let mut mode = perm & UDF_FENTRY_PERM_USER_MASK;
    mode |= (perm & UDF_FENTRY_PERM_GRP_MASK) >> 2;
    mode |= (perm & UDF_FENTRY_PERM_OWNER_MASK) >> 4;
    mode |= (flags & u32::from(UDF_ICB_TAG_FLAGS_STICKY)) << 4;
    mode |= (flags & u32::from(UDF_ICB_TAG_FLAGS_SETGID)) << 6;
    mode |= (flags & u32::from(UDF_ICB_TAG_FLAGS_SETUID)) << 8;

    mode
}

/// `udf_access` (`vop_access`).
pub fn udf_access<G: ProcGlue>(
    vp: &'static Vnode<G>,
    a_mode: i32,
    cred: Cred<'_>,
) -> Result<(), Errno> {
    let up = VTOU(vp);

    if a_mode & VWRITE != 0 {
        match vp.vtype() {
            VDIR | VLNK | VREG => return Err(Errno::EROFS),
            _ => {}
        }
    }

    let mode = udf_permtomode(&up);

    // The C passes the members as they are on the medium.
    vaccess(vp.vtype(), mode, up.fe.uid, up.fe.gid, a_mode, ucred(cred))
}

/// 1 for a leap year, 0 otherwise.
fn udf_isaleapyear(year: i32) -> i32 {
    let mut i = i32::from(year % 4 == 0);
    i &= i32::from(year % 100 != 0);
    i |= i32::from(year % 400 == 0);

    i
}

/// This is just a rough hack. Daylight savings isn't calculated and `tv_nsec` is ignored.
/// Timezone calculation compliments of Julian Elischer <julian@elischer.org>.
pub fn udf_timetotimespec(time: &Timestamp, t: &mut Timespec) {
    // DirectCD seems to like using bogus year values
    let year = i32::from(letoh16(time.year));
    if year < 1970 {
        t.tv_sec = 0;
        t.tv_nsec = 0;
        return;
    }

    // Calculate the time and day
    t.tv_nsec = 1000 * i64::from(time.usec)
        + 100_000 * i64::from(time.hund_usec)
        + 10_000_000 * i64::from(time.centisec);
    t.tv_sec = i64::from(time.second);
    t.tv_sec += i64::from(time.minute) * 60;
    t.tv_sec += i64::from(time.hour) * 3600;
    t.tv_sec += i64::from(time.day) * 3600 * 24;

    // Calculate the month (the C's index from 1, the days of the months after January)
    let lpyear = udf_isaleapyear(year) as usize;
    for i in 1..usize::from(time.month) {
        let days = MON_LENS[lpyear].get(i).copied().unwrap_or(0);
        t.tv_sec += i64::from(days) * 3600 * 24;
    }

    // Speed up the calculation
    if year > 1979 {
        t.tv_sec += 315_532_800;
    }
    if year > 1989 {
        t.tv_sec += 315_619_200;
    }
    if year > 1999 {
        t.tv_sec += 315_532_800;
    }
    for i in 2000..year {
        let daysinyear = udf_isaleapyear(i) + 365;
        t.tv_sec += i64::from(daysinyear) * 3600 * 24;
    }

    // Calculate the time zone. The timezone is 12 bit signed 2's compliment, so we gotta do
    // some extra magic to handle it right.
    let mut tz = letoh16(time.type_tz) & 0x0fff;
    if tz & 0x0800 != 0 {
        tz |= 0xf000; // extend the sign to 16 bits
    }
    let s_tz_offset = tz as i16;
    if { time.type_tz } & 0x1000 != 0 && s_tz_offset != -2047 {
        t.tv_sec -= i64::from(s_tz_offset) * 60;
    }
}

/// `udf_getattr` (`vop_getattr`).
pub fn udf_getattr<G: ProcGlue>(vp: &'static Vnode<G>, vap: &mut Vattr) -> Result<(), Errno> {
    let up = VTOU(vp);
    let fe = &up.fe;
    let bsize = up.u_ump.um_bsize.get();

    // The C tests `vp->v_type & VDIR`, a bitwise and of the enumeration (the module's
    // deviations).
    let dirlike = vp.vtype() as i32 & VDIR as i32 != 0;

    vap.va_fsid = i64::from(up.u_dev);
    vap.va_fileid = u64::from(up.u_ino);
    vap.va_mode = udf_permtomode(&up);
    vap.va_nlink = Nlink::from(letoh16(fe.link_cnt));
    // The spec says that -1 is valid for uid/gid and indicates an invalid uid/gid. How should
    // this be represented?
    let uid = letoh32(fe.uid);
    vap.va_uid = if uid == u32::MAX { 0 } else { uid };
    let gid = letoh32(fe.gid);
    vap.va_gid = if gid == u32::MAX { 0 } else { gid };
    vap.va_rdev = 0;
    if dirlike {
        vap.va_nlink += 1; // Count a reference to ourselves
        // Directories that are recorded within their ICB will show as having 0 blocks
        // recorded. Since tradition dictates that directories consume at least one logical
        // block, make it appear so.
        vap.va_size = i64::from(bsize) as u64;
    } else {
        vap.va_size = letoh64(fe.inf_len);
    }
    // The times and the blocks of the entry's own kind (extended or not).
    udf_timetotimespec(&fe.atime, &mut vap.va_atime);
    udf_timetotimespec(&fe.mtime, &mut vap.va_mtime);
    if dirlike && fe.logblks_rec != 0 {
        vap.va_size = letoh64(fe.logblks_rec).wrapping_mul(i64::from(bsize) as u64);
    }
    vap.va_ctime = vap.va_mtime; // Stored as an Extended Attribute
    vap.va_flags = 0;
    vap.va_gen = 1;
    vap.va_blocksize = i64::from(bsize);
    vap.va_bytes = letoh64(fe.inf_len);
    vap.va_type = vp.vtype();
    vap.va_filerev = 0;

    Ok(())
}

/// `udf_open` (`vop_open`): nothing to be done at this point.
pub fn udf_open() -> Result<(), Errno> {
    Ok(())
}

/// `udf_close` (`vop_close`): nothing to be done at this point.
pub fn udf_close() -> Result<(), Errno> {
    Ok(())
}

/// `udf_ioctl` (`vop_ioctl`): file specific ioctls.
pub fn udf_ioctl() -> Result<(), Errno> {
    Err(Errno::ENOTTY)
}

/// `udf_pathconf` (`vop_pathconf`). I'm not sure that this has much value in a read-only
/// filesystem, but cd9660 has it too.
pub fn udf_pathconf(name: i32, retval: &mut Register) -> Result<(), Errno> {
    *retval = match name {
        _PC_LINK_MAX => 65535,
        _PC_NAME_MAX => NAME_MAX as Register,
        _PC_CHOWN_RESTRICTED | _PC_NO_TRUNC => 1,
        _PC_TIMESTAMP_RESOLUTION => 1000, // 1 microsecond
        _ => return Err(Errno::EINVAL),
    };

    Ok(())
}

/// `udf_read` (`vop_read`).
pub fn udf_read<G: ProcGlue>(vp: &'static Vnode<G>, uio: &mut Uio<'_, G>) -> Result<(), Errno> {
    let up = VTOU(vp);

    if uio.uio_offset < 0 {
        return Err(Errno::EINVAL);
    }

    let fsize = letoh64(up.fe.inf_len) as Off;

    while uio.uio_offset < fsize && uio.uio_resid > 0 {
        let offset = uio.uio_offset;
        let mut size = uio.uio_resid.min(MAXBSIZE as usize) as i32;
        if Off::from(size) > fsize - offset {
            size = (fsize - offset) as i32;
        }
        let mut data = udf_readatoffset(&up, &mut size, offset)?;
        // A chunk of no bytes ends the read (the module's deviations).
        let Ok(n) = usize::try_from(size) else {
            break;
        };
        if n == 0 {
            break;
        }
        let error = match &mut data {
            // The buffer's bytes, moved out of its token.
            UdfData::Buf(t, off) => match t.data().get_mut(*off..*off + n) {
                Some(d) => <G::FileGlue as FileGlue>::uiomove(d, uio),
                None => Err(Errno::EIO),
            },
            // The file entry's, through a bounce buffer (the module's redesign).
            UdfData::Entry(off) => {
                let src = up.fentry().get(*off..*off + n).unwrap_or(&[]);
                let mut bounce = [0u8; 256];
                let mut r = Ok(());
                for chunk in src.chunks(bounce.len()) {
                    if uio.uio_resid == 0 {
                        break;
                    }
                    let b = &mut bounce[..chunk.len()];
                    b.copy_from_slice(chunk);
                    r = <G::FileGlue as FileGlue>::uiomove(b, uio);
                    if r.is_err() {
                        break;
                    }
                }
                r
            }
        };
        // brelse(bp)
        drop(data);
        error?;
    }

    Ok(())
}

/// Translate the name from a CS0 dstring to a 16-bit Unicode String. Hooks need to be placed
/// in here to translate from Unicode to the encoding that the kernel/user expects. Return the
/// length of the translated string (0 on failure).
pub fn udf_transname<G: ProcGlue>(
    cs0string: &[u8],
    destname: &mut [u8],
    len: i32,
    _ump: Option<&Umount<G>>,
) -> usize {
    if len > MAXNAMLEN as i32 {
        #[cfg(feature = "diagnostic")]
        printf(format_args!("udf_transname(): name too long\n"));
        return 0;
    }

    // allocate a buffer big enough to hold an 8->16 bit expansion
    let Some(mut transname) = vfs::<G>().udf.trans_pool.get(PR_WAITOK, || [0; MAXNAMLEN]) else {
        kpanic(format_args!(
            "udf_transname: pool_get(udf_trans_pool) failed"
        ));
    };

    let unilen = match udf_rawnametounicode(len as u32, cs0string, &mut transname[..]) {
        Ok(n) if n < destname.len() => n,
        _ => {
            #[cfg(feature = "diagnostic")]
            printf(format_args!(
                "udf_transname(): Unicode translation failed\n"
            ));
            return 0;
        }
    };

    // Pack it back to 8-bit Unicode.
    for (d, &t) in destname.iter_mut().zip(transname[..unilen].iter()) {
        *d = if t & 0xff00 != 0 {
            b'?' // Fudge the 16bit chars
        } else {
            (t & 0xff) as u8
        };
    }

    drop(transname);

    // Don't forget to terminate the string.
    destname[unilen] = 0;

    unilen
}

/// Compare a CS0 dstring with a name passed in from the VFS layer. Return `false` on a
/// successful match, `true` otherwise (the C's 0 and nonzero). Unicode work may need to be
/// done here also.
pub fn udf_cmpname<G: ProcGlue>(
    cs0string: &[u8],
    cmpname: &[u8],
    cs0len: i32,
    cmplen: usize,
    ump: &Umount<G>,
) -> bool {
    // This is overkill, but not worth creating a new pool
    let Some(mut item) = vfs::<G>().udf.trans_pool.get(PR_WAITOK, || [0; MAXNAMLEN]) else {
        kpanic(format_args!("udf_cmpname: pool_get(udf_trans_pool) failed"));
    };
    let transname = bytes_of_mut(&mut item[..]);

    let cs0len = udf_transname(cs0string, transname, cs0len, Some(ump));

    // Easy check. If they aren't the same length, they aren't equal
    if cs0len == 0 || cs0len != cmplen {
        true
    } else {
        transname.get(..cmplen) != cmpname.get(..cmplen)
    }
}

/// Copy the entry out with `d_off = off`; `Ok(false)` (the C's -1) when it does not fit.
fn udf_uiodir<G: ProcGlue>(
    uiodir: &mut UdfUiodir,
    uio: &mut Uio<'_, G>,
    off: i64,
) -> Result<bool, Errno> {
    let de_size = dirent_size(&uiodir.dirent);

    if uio.uio_resid < de_size {
        uiodir.eofflag = 0;
        return Ok(false);
    }
    uiodir.dirent.d_off = off;
    uiodir.dirent.d_reclen = de_size as u16;

    let namlen = usize::from(uiodir.dirent.d_namlen);
    if uiodir.dirent.d_name[..namlen].contains(&b'/') {
        // illegal file name
        return Err(Errno::EINVAL);
    }

    dirent_uiomove(&uiodir.dirent, uio).map(|()| true)
}

/// `udf_opendir(up, offset, fsize, ump)`: a directory stream from `offset`
/// (`pool_get(&udf_ds_pool, PR_WAITOK | PR_ZERO)`); its directory and mount are the walk's
/// arguments (`udf.rs`'s deviations).
fn udf_opendir<G: ProcGlue>(offset: i32, fsize: i32) -> PoolBox<UdfDirstream> {
    let ds = vfs::<G>().udf.ds_pool.get(PR_WAITOK, || UdfDirstream {
        data: None,
        buf: None,
        fsize,
        off: 0,
        this_off: 0,
        offset,
        size: 0,
        error: Ok(()),
        fid_fragment: false,
    });
    match ds {
        Some(ds) => ds,
        None => kpanic(format_args!("udf_opendir: pool_get(udf_ds_pool) failed")),
    }
}

/// `udf_getfid(ds)`: the next file identifier descriptor of the stream over the directory
/// `node` (the C's `ds->node`), with its implementation use area and its name
/// (`UDF_FID_SIZE + l_iu + l_fi` bytes); `None` at the end or on an error (`ds->error`).
fn udf_getfid<'a, G: ProcGlue>(ds: &'a mut UdfDirstream, node: &'a Unode<G>) -> Option<&'a [u8]> {
    let bsize = node.u_ump.um_bsize.get();
    let mut frag_size = 0;

    // End of directory?
    if ds.offset.wrapping_add(ds.off) >= ds.fsize {
        ds.error = Ok(());
        return None;
    }

    // Grab the first extent of the directory
    if ds.off == 0 {
        ds.size = 0;
        match udf_readatoffset(node, &mut ds.size, Off::from(ds.offset)) {
            Ok(d) => ds.data = Some(d),
            Err(e) => {
                // The buffer, if any, was released.
                ds.error = Err(e);
                return None;
            }
        }
    }

    // Clean up from a previous fragmented FID. Is this the right place for this?
    if ds.fid_fragment && ds.buf.is_some() {
        ds.fid_fragment = false;
        ds.buf = None; // free(ds->buf, M_UDFFID)
    }

    let off = ds.off as usize;
    let size = ds.size;
    let chunk = ds.data.as_ref().map_or(&[][..], |d| d.bytes(node, size));

    // Check to see if the fid is fragmented. The first test ensures that we don't wander off
    // the end of the buffer looking for the l_iu and l_fi fields.
    let fragmented = off + UDF_FID_SIZE > size as usize
        || FileidDesc::at(chunk, off).is_none_or(|fid| {
            off + usize::from(letoh16(fid.l_iu)) + usize::from(fid.l_fi) + UDF_FID_SIZE
                > size as usize
        });

    let total_fid_size = if fragmented {
        // Copy what we have of the fid into a buffer
        frag_size = ds.size - ds.off;
        if frag_size >= bsize || frag_size < 0 {
            printf(format_args!("udf: invalid FID fragment\n"));
            ds.error = Err(Errno::EINVAL);
            return None;
        }
        let frag = frag_size as usize;

        // File ID descriptors can only be at most one logical sector in size.
        let Some(mut buf) = MArray::new(M_UDFFID, M_WAITOK, bsize as usize, |_| 0u8) else {
            kpanic(format_args!("udf_getfid: malloc(M_UDFFID) failed"));
        };
        let have = chunk.get(off..).unwrap_or(&[]);
        let n = frag.min(have.len());
        buf[..n].copy_from_slice(&have[..n]);

        // The stream owns the buffer from here on: it is freed with the stream (the module's
        // deviations).
        ds.buf = Some(buf);

        // brelse(ds->bp)
        ds.data = None;

        // Fetch the next allocation
        ds.offset = ds.offset.wrapping_add(ds.size);
        ds.size = 0;
        match udf_readatoffset(node, &mut ds.size, Off::from(ds.offset)) {
            Ok(d) => ds.data = Some(d),
            Err(e) => {
                ds.error = Err(e);
                return None;
            }
        }
        let chunk = ds.data.as_ref().map_or(&[][..], |d| d.bytes(node, ds.size));
        let Some(buf) = ds.buf.as_mut() else {
            kpanic(format_args!("udf_getfid: no fragment buffer"));
        };

        // If the fragment was so small that we didn't get the l_iu and l_fi fields, copy
        // those in.
        if frag < UDF_FID_SIZE {
            let n = (UDF_FID_SIZE - frag).min(chunk.len());
            buf[frag..frag + n].copy_from_slice(&chunk[..n]);
        }

        // Now that we have enough of the fid to work with, copy in the rest of the fid from
        // the new allocation.
        let total = FileidDesc::at(buf, 0).map_or(usize::MAX, |fid| {
            UDF_FID_SIZE + usize::from(letoh16(fid.l_iu)) + usize::from(fid.l_fi)
        });
        if total > bsize as usize {
            printf(format_args!("udf: invalid FID\n"));
            ds.error = Err(Errno::EIO);
            return None;
        }
        let n = (total - frag).min(chunk.len());
        buf[frag..frag + n].copy_from_slice(&chunk[..n]);

        ds.fid_fragment = true;
        total
    } else {
        FileidDesc::at(chunk, off).map_or(UDF_FID_SIZE, |fid| {
            usize::from(letoh16(fid.l_iu)) + usize::from(fid.l_fi) + UDF_FID_SIZE
        })
    };

    // Update the offset. Align on a 4 byte boundary because the UDF spec says so.
    if ds.fid_fragment {
        ds.off = ((total_fid_size - frag_size as usize + 3) & !0x03) as i32;
    } else {
        ds.off = ds.off.wrapping_add(((total_fid_size + 3) & !0x03) as i32);
    }
    ds.this_off = ds.offset.wrapping_add(ds.off);

    if ds.fid_fragment {
        ds.buf.as_deref().and_then(|b| b.get(..total_fid_size))
    } else {
        ds.data
            .as_ref()
            .map_or(&[][..], |d| d.bytes(node, ds.size))
            .get(off..off + total_fid_size)
    }
}

/// `udf_closedir(ds)`: the stream's buffer released, its fragment buffer freed and the item
/// put back (the stream's drop, in that order).
fn udf_closedir(ds: PoolBox<UdfDirstream>) {
    drop(ds);
}

/// `um_start += um_meta_start; um_len = um_meta_len` while a directory of a metadata
/// partition is read (the words of the mount every `bmap` reads: the C's race across the
/// walk's sleeps, kept, BRIEF 5).
fn meta_enter<G: ProcGlue>(ump: &Umount<G>) {
    if ump.um_flags.get() & UDF_MNT_USES_META != 0 {
        ump.um_start
            .set(ump.um_start.get().wrapping_add(ump.um_meta_start.get()));
        ump.um_len.set(ump.um_meta_len.get());
    }
}

/// Undoes [`meta_enter`].
fn meta_leave<G: ProcGlue>(ump: &Umount<G>) {
    if ump.um_flags.get() & UDF_MNT_USES_META != 0 {
        ump.um_start.set(ump.um_realstart.get());
        ump.um_len.set(ump.um_reallen.get());
    }
}

/// A `struct dirent` with every byte zero (the C's `memset(&dir, 0, sizeof(dir))`).
const fn dirent_zeroed() -> Dirent {
    Dirent {
        d_fileno: 0,
        d_off: 0,
        d_reclen: 0,
        d_type: DT_UNKNOWN,
        d_namlen: 0,
        __d_padding: [0; 4],
        d_name: [0; MAXNAMLEN + 1],
    }
}

/// `udf_readdir` (`vop_readdir`).
pub fn udf_readdir<G: ProcGlue>(
    vp: &'static Vnode<G>,
    uio: &mut Uio<'_, G>,
    eofflag: &mut i32,
) -> Result<(), Errno> {
    let up = VTOU(vp);
    let ump = &up.u_ump;
    let mut uiodir = UdfUiodir {
        dirent: dirent_zeroed(),
        eofflag: 1,
    };
    let mut error: Result<(), Errno> = Ok(());

    // if asked to start at SELF_OFFSET or PARENT_OFFSET, search for the parent ref
    let mut mode = if uio.uio_offset == SELF_OFFSET {
        uio.uio_offset = 0;
        ReaddirMode::SelfEntry
    } else if uio.uio_offset == PARENT_OFFSET {
        uio.uio_offset = 0;
        ReaddirMode::Parent
    } else {
        ReaddirMode::Normal
    };

    // Iterate through the file id descriptors. Give the parent dir entry special attention.
    meta_enter(ump);
    let mut ds = udf_opendir::<G>(uio.uio_offset as i32, letoh64(up.fe.inf_len) as i32);

    let mut last_off = ds.offset.wrapping_add(ds.off);
    while let Some(fid) = udf_getfid(&mut ds, &up) {
        let Some(hdr) = FileidDesc::at(fid, 0) else {
            break;
        };

        // Should we return an error on a bad fid?
        if udf_checktag(&hdr.tag, TAGID_FID).is_err() {
            printf(format_args!("Invalid FID tag ({})\n", { hdr.tag.id }));
            error = Err(Errno::EIO);
            break;
        }

        // Is this a deleted file?
        if hdr.file_char & UDF_FILE_CHAR_DEL != 0 {
            continue;
        }

        let r = if hdr.l_fi == 0 && hdr.file_char & UDF_FILE_CHAR_PAR != 0 {
            // Do up the '.' and '..' entries. Dummy values are used for the offset since the
            // offset here is usually zero, and NFS doesn't like that value
            if mode == ReaddirMode::Normal {
                let d = &mut uiodir.dirent;
                d.d_fileno = Ino::from(up.u_ino);
                d.d_type = DT_DIR;
                d.d_name[0] = b'.';
                d.d_name[1] = 0;
                d.d_namlen = 1;
                match udf_uiodir(&mut uiodir, uio, SELF_OFFSET) {
                    Ok(true) => {}
                    // The C breaks out with udf_uiodir's -1 and returns it (the module's
                    // deviations).
                    Ok(false) => {
                        error = Err(Errno::ERESTART);
                        break;
                    }
                    Err(e) => {
                        error = Err(e);
                        break;
                    }
                }
            }
            let r = if mode == ReaddirMode::Parent {
                Ok(true)
            } else {
                let d = &mut uiodir.dirent;
                d.d_fileno = Ino::from(udf_getid(&hdr.icb));
                d.d_type = DT_DIR;
                d.d_name[0] = b'.';
                d.d_name[1] = b'.';
                d.d_name[2] = 0;
                d.d_namlen = 2;
                udf_uiodir(&mut uiodir, uio, PARENT_OFFSET)
            };
            mode = ReaddirMode::Normal;
            r
        } else if mode != ReaddirMode::Normal {
            continue;
        } else {
            // `&fid->data[fid->l_iu]`: the name past the implementation use area.
            let name = fid
                .get(UDF_FID_SIZE + usize::from(letoh16(hdr.l_iu))..)
                .unwrap_or(&[]);
            let d = &mut uiodir.dirent;
            d.d_namlen = udf_transname(name, &mut d.d_name, i32::from(hdr.l_fi), Some(ump)) as u8;
            d.d_fileno = Ino::from(udf_getid(&hdr.icb));
            d.d_type = if hdr.file_char & UDF_FILE_CHAR_DIR != 0 {
                DT_DIR
            } else {
                DT_UNKNOWN
            };
            udf_uiodir(&mut uiodir, uio, i64::from(ds.this_off))
        };
        match r {
            Ok(true) => {}
            // udf_uiodir() indicates there isn't space for another entry by returning -1
            Ok(false) => break,
            Err(e) => {
                error = Err(e);
                break;
            }
        }
        last_off = ds.this_off;
    }

    // tell the calling layer whether we need to be called again
    *eofflag = uiodir.eofflag;
    uio.uio_offset = Off::from(last_off);

    if error.is_ok() {
        error = ds.error;
    }

    udf_closedir(ds);
    meta_leave(ump);

    error
}

/// `udf_readlink` (`vop_readlink`). Are there any implementations out there that do
/// soft-links?
pub fn udf_readlink() -> Result<(), Errno> {
    Err(Errno::EOPNOTSUPP)
}

/// The token of a buffer handed to `udf_strategy` (`panic(9)` on a lent buffer with none:
/// no `bsd` caller lends one to a UDF vnode, the module's deviations).
fn strategy_token(bp: IoBuf) -> BufRef {
    match bp.into_token() {
        Some(t) => t,
        None => kpanic(format_args!("udf_strategy: a lent buffer with no token")),
    }
}

/// `udf_strategy` (`vop_strategy`). A hole's buffer is finished by `biodone` (the C's
/// `splbio(); biodone(bp); splx(s)`: one section of the cache's lock).
#[allow(
    clippy::if_not_else,
    reason = "the C's test, whose comment says why it is not cd9660's"
)]
pub fn udf_strategy<G: ProcGlue>(vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
    let mut bp = strategy_token(bp);
    let up = VTOU(vp);

    // cd9660 has this test reversed, but it seems more logical this way
    if bp.buf().blkno() != bp.buf().lblkno() {
        // Files that are embedded in the fentry don't translate well to a block number.
        // Reject. (The C's `udf_bmap_internal` writes the sector into `b_lblkno`: kept.)
        let off = bp
            .buf()
            .lblkno()
            .wrapping_mul(Daddr::from(up.u_ump.um_bsize.get()));
        let invalid = match udf_bmap_internal(&up, off) {
            Ok(UdfBmap::Block { sector, .. }) => {
                bp.buf().set_lblkno(sector);
                false
            }
            Ok(UdfBmap::Invalid { sector }) => {
                bp.buf().set_lblkno(sector);
                true
            }
            Err(_) => true,
        };
        if invalid {
            bp.clear();
            bp.buf().set_blkno(-1);
        }
    } else {
        let b = bp.buf();
        let mut blkno: Daddr = b.blkno();
        let error = vop_bmap(vp, b.lblkno(), None, Some(&mut blkno), None);
        b.set_blkno(blkno);
        if let Err(e) = error {
            b.set_error(Some(e));
            b.set(B_ERROR);
            biodone(bp);
            return Err(e);
        }

        if blkno == -1 {
            bp.clear();
        }
    }

    if bp.buf().blkno() == -1 {
        biodone(bp);
    } else {
        let devvp = up.devvp();
        bp.buf().set_dev(devvp.rdev());
        drop(up);
        let _ = vop_strategy(devvp, IoBuf::Token(bp));
    }

    Ok(())
}

/// `udf_lock` (`vop_lock`).
pub fn udf_lock<G: ProcGlue>(vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
    VTOU(vp).u_lock.enter(flags & LK_RWFLAGS)
}

/// `udf_unlock` (`vop_unlock`).
pub fn udf_unlock<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    VTOU(vp).u_lock.exit();
    Ok(())
}

/// `udf_islocked` (`vop_islocked`).
pub fn udf_islocked<G: ProcGlue>(vp: &'static Vnode<G>) -> i32 {
    VTOU(vp).u_lock.status()
}

/// `udf_print` (`vop_print`): complete the information given by `vprint()`.
pub fn udf_print<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    #[cfg(any(feature = "debug", feature = "diagnostic"))]
    {
        let up = VTOU(vp);

        printf(format_args!("tag VT_UDF, hash id {}\n", up.u_ino));
        #[cfg(feature = "diagnostic")]
        printf(format_args!("\n"));
    }
    #[cfg(not(any(feature = "debug", feature = "diagnostic")))]
    let _ = vp;

    Ok(())
}

/// `udf_bmap` (`vop_bmap`).
pub fn udf_bmap<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bn: Daddr,
    vpp: Option<&mut Option<&'static Vnode<G>>>,
    bnp: Option<&mut Daddr>,
    runp: Option<&mut i32>,
) -> Result<(), Errno> {
    let up = VTOU(vp);
    let ump = &up.u_ump;

    if let Some(vpp) = vpp {
        *vpp = up.u_devvp.get();
    }
    let Some(bnp) = bnp else {
        return Ok(());
    };

    let lsector = match udf_bmap_internal(&up, bn.wrapping_mul(Daddr::from(ump.um_bsize.get())))? {
        UdfBmap::Block { sector, .. } => sector,
        UdfBmap::Invalid { .. } => return Err(UDF_INVALID_BMAP),
    };

    // Translate logical to physical sector number
    *bnp = lsector.wrapping_shl((ump.um_bshift.get() - DEV_BSHIFT as i32) as u32);

    // Punt on read-ahead for now
    if let Some(runp) = runp {
        *runp = 0;
    }

    Ok(())
}

/// `udf_lookup` (`vop_lookup`): the all powerful VOP_LOOKUP().
#[allow(
    clippy::too_many_lines,
    reason = "the C's one function, its passes and exits kept in place"
)]
pub fn udf_lookup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vpp: &mut Option<&'static Vnode<G>>,
    cnp: &mut Cn<'_, G>,
) -> Result<(), Errno> {
    let up = VTOU(dvp);
    let ump = &up.u_ump;
    let nameiop = cnp.cn_nameiop;
    let flags = cnp.cn_flags;
    let namelen = usize::try_from(cnp.namelen()).unwrap_or(0);
    let fsize = letoh64(up.fe.inf_len) as i32;
    let p = cnp.proc();
    *vpp = None;

    // Make sure the process can scan the requested directory.
    vop_access(dvp, VEXEC, cnp.cn_cred, p)?;

    // Check if the (directory, name) tuple has been already cached.
    if let Some(vp) = cache_lookup(dvp, cnp)? {
        *vpp = Some(vp);
        return Ok(());
    }

    // If dvp is what's being looked up, then return it.
    if namelen == 1 && cnp.name().first() == Some(&b'.') {
        vref(dvp);
        *vpp = Some(dvp);
        return Ok(());
    }

    // If this is a LOOKUP and we've already partially searched through the directory, pick
    // up where we left off and flag that the directory may need to be searched twice. For a
    // full description, see /sys/isofs/cd9660/cd9660_lookup.c:cd9660_lookup()
    let diroff = up.u_diroff();
    let (mut offset, mut numdirpasses) =
        if nameiop != LOOKUP || diroff == 0 || diroff > i64::from(fsize) {
            (0, 1)
        } else {
            vfs::<G>()
                .nchstats
                .ncs_2passes
                .fetch_add(1, Ordering::Relaxed);
            (diroff as i32, 2)
        };

    meta_enter(ump);
    let mut id: Udfino = 0;
    let ds = loop {
        // lookloop:
        let mut ds = udf_opendir::<G>(offset, fsize);
        let mut error: Result<(), Errno> = Ok(());

        let nameptr = cnp.name();
        while let Some(fid) = udf_getfid(&mut ds, &up) {
            let Some(hdr) = FileidDesc::at(fid, 0) else {
                break;
            };

            // Check for a valid FID tag.
            if udf_checktag(&hdr.tag, TAGID_FID).is_err() {
                printf(format_args!("udf_lookup: Invalid tag\n"));
                error = Err(Errno::EIO);
                break;
            }

            // Is this a deleted file?
            if hdr.file_char & UDF_FILE_CHAR_DEL != 0 {
                continue;
            }

            if hdr.l_fi == 0 && hdr.file_char & UDF_FILE_CHAR_PAR != 0 {
                if flags & ISDOTDOT != 0 {
                    id = udf_getid(&hdr.icb);
                    break;
                }
            } else {
                let name = fid
                    .get(UDF_FID_SIZE + usize::from(letoh16(hdr.l_iu))..)
                    .unwrap_or(&[]);
                if !udf_cmpname(name, nameptr, i32::from(hdr.l_fi), namelen, ump) {
                    id = udf_getid(&hdr.icb);
                    break;
                }
            }
        }

        if error.is_ok() {
            error = ds.error;
        }

        if let Err(e) = error {
            udf_closedir(ds);
            meta_leave(ump);
            return Err(e);
        }

        // Did we have a match? If not, do another pass?
        if id == 0 && numdirpasses == 2 {
            numdirpasses -= 1;
            offset = 0;
            udf_closedir(ds);
            continue;
        }
        break ds;
    };

    let mut error: Result<(), Errno> = Ok(());
    if id != 0 {
        match udf_vget(ump.um_mountp, Ino::from(id)) {
            Ok(tdp) => {
                // Remember where this entry was if it's the final component.
                if flags & ISLASTCN != 0 && nameiop == LOOKUP {
                    up.set_u_diroff(i64::from(ds.offset.wrapping_add(ds.off)));
                }
                if numdirpasses == 2 {
                    vfs::<G>()
                        .nchstats
                        .ncs_pass2
                        .fetch_add(1, Ordering::Relaxed);
                }
                if flags & LOCKPARENT == 0 || flags & ISLASTCN == 0 {
                    cnp.cn_flags |= PDIRUNLOCK;
                    let _ = vop_unlock(dvp);
                }

                *vpp = Some(tdp);
            }
            Err(e) => error = Err(e),
        }
    } else if flags & ISLASTCN != 0 && (nameiop == CREATE || nameiop == RENAME) {
        error = Err(Errno::EROFS);
    } else {
        error = Err(Errno::ENOENT);
    }

    // Cache the result of this lookup.
    if flags & MAKEENTRY != 0 {
        cache_enter(dvp, *vpp, cnp.name());
    }

    udf_closedir(ds);
    meta_leave(ump);

    error
}

/// `udf_inactive` (`vop_inactive`): no need to sync anything, so just unlock the vnode and
/// return.
pub fn udf_inactive<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    let _ = vop_unlock(vp);

    Ok(())
}

/// `udf_reclaim` (`vop_reclaim`): the unode off its hash chain, its device reference given
/// back and the unode out of the vnode (freed with its file entry by its last handle, the
/// module's redesign).
pub fn udf_reclaim<G: ProcGlue>(vp: &'static Vnode<G>) -> Result<(), Errno> {
    if vp.has_node() {
        let up = VTOU(vp);
        let _ = udf_hashrem(&up);
        if let Some(devvp) = up.u_devvp.get() {
            vrele(devvp);
            up.u_devvp.set(None);
        }

        // free(up->u_fentry); pool_put(&unode_pool, up); vp->v_data = NULL
        drop(vp.take_node());
        drop(up);
    }

    Ok(())
}

/// Read the block and then set the data pointer to correspond with the offset passed in.
/// Only read in at most `size` bytes, and then set `size` to the number of bytes pointed to.
/// If `size` is zero, try to read in a whole extent.
pub fn udf_readatoffset<G: ProcGlue>(
    up: &Unode<G>,
    size: &mut i32,
    offset: Off,
) -> Result<UdfData, Errno> {
    let ump = &up.u_ump;

    let (sector, max_size) = match udf_bmap_internal(up, offset)? {
        UdfBmap::Invalid { .. } => {
            // This error means that the file *data* is stored in the allocation descriptor
            // field of the file entry: from `offset` on (the module's deviations).
            let off = up.fe.data_off() as u64 + u64::from(letoh32(up.fe.l_ea));
            let l_ad = letoh32(up.fe.l_ad);
            let skip = (offset.max(0) as u64).min(u64::from(l_ad));
            let len = up.fentry().len() as u64;
            let off = (off + skip).min(len);
            *size = (u64::from(l_ad) - skip).min(len - off) as i32;
            return Ok(UdfData::Entry(off as usize));
        }
        UdfBmap::Block { sector, max_size } => (sector, max_size),
    };

    // Adjust the size so that it is within range
    if *size == 0 || *size as u32 > max_size {
        *size = max_size as i32;
    }
    *size = (*size as u32).min(MAXBSIZE as u32) as i32;

    let t = match udf_readlblks(ump, sector as i32, *size) {
        Ok(t) => t,
        Err(e) => {
            printf(format_args!(
                "warning: udf_readlblks returned error {}\n",
                e as i32
            ));
            return Err(e);
        }
    };

    let off = offset
        .checked_rem(Off::from(ump.um_bsize.get()))
        .unwrap_or(0) as usize;
    let avail = lblk_bytes(&t).len().saturating_sub(off);
    *size = (*size as u32).min(avail as u32) as i32;
    Ok(UdfData::Buf(t, off))
}

/// `N` bytes of the node's file entry copy from `off` on, zeros past its end.
fn fentry_read<G: ProcGlue, const N: usize>(up: &Unode<G>, off: i64) -> [u8; N] {
    let mut out = [0u8; N];
    if let Ok(off) = usize::try_from(off)
        && let Some(src) = up.fentry().get(off..)
    {
        let n = src.len().min(N);
        out[..n].copy_from_slice(&src[..n]);
    }
    out
}

/// Translate a file offset into a logical block and then into a physical block.
pub fn udf_bmap_internal<G: ProcGlue>(up: &Unode<G>, offset: Off) -> Result<UdfBmap, Errno> {
    let ump = &up.u_ump;
    let tag = up.fe.icbtag;
    let l_ea = letoh32(up.fe.l_ea) as i32;
    let l_ad = letoh32(up.fe.l_ad) as i32;
    let hdr = up.fe.data_off() as i64;

    match letoh16(tag.strat_type) {
        4 => {}
        4096 => {
            printf(format_args!("Cannot deal with strategy4096 yet!\n"));
            return Err(Errno::ENODEV);
        }
        _ => {
            printf(format_args!("Unknown strategy type {}\n", {
                tag.strat_type
            }));
            return Err(Errno::ENODEV);
        }
    }

    let bshift = ump.um_bshift.get() as u32;
    let mut offset = offset;
    let mut icblen: u32 = 0;
    let mut ad_num: i64 = 0;

    let lsector: Daddr = match letoh16(tag.flags) & 0x7 {
        0 => {
            // The allocation descriptor field is filled with short_ad's. If the offset is
            // beyond the current extent, look for the next extent.
            let mut icb: ShortAd;
            loop {
                offset -= Off::from(icblen);
                let ad_offset = ShortAd::SIZE as i64 * ad_num;
                if ad_offset > i64::from(l_ad) {
                    printf(format_args!(
                        "SFile offset out of bounds ({ad_offset} > {l_ad})\n"
                    ));
                    return Err(Errno::EINVAL);
                }

                let raw: [u8; 8] = fentry_read(up, hdr + i64::from(l_ea) + ad_offset);
                icb = ShortAd::at(&raw, 0).unwrap_or(ShortAd { len: 0, lb_num: 0 });
                icblen = geticblen(&icb);
                ad_num += 1;
                if offset < Off::from(icblen) {
                    break;
                }
            }

            offset.wrapping_shr(bshift) + Daddr::from(letoh32(icb.lb_num))
        }
        1 => {
            // The allocation descriptor field is filled with long_ad's If the offset is
            // beyond the current extent, look for the next extent.
            let mut icb: LongAd;
            loop {
                offset -= Off::from(icblen);
                let ad_offset = LongAd::SIZE as i64 * ad_num;
                if ad_offset > i64::from(l_ad) {
                    printf(format_args!(
                        "LFile offset out of bounds ({ad_offset} > {l_ad})\n"
                    ));
                    return Err(Errno::EINVAL);
                }
                let raw: [u8; 16] = fentry_read(up, hdr + i64::from(l_ea) + ad_offset);
                icb = LongAd::at(&raw, 0).unwrap_or_else(hal::user::zeroed);
                icblen = geticblen(&icb);
                ad_num += 1;
                if offset < Off::from(icblen) {
                    break;
                }
            }

            offset.wrapping_shr(bshift) + Daddr::from(letoh32(icb.loc.lb_num))
        }
        3 => {
            // This type means that the file *data* is stored in the allocation descriptor
            // field of the file entry.
            return Ok(UdfBmap::Invalid {
                sector: Daddr::from(up.u_ino.wrapping_add(ump.um_start.get())),
            });
        }
        // 2: DirectCD does not use extended_ad's
        _ => {
            printf(format_args!(
                "Unsupported allocation descriptor {}\n",
                { tag.flags } & 0x7
            ));
            return Err(Errno::ENODEV);
        }
    };
    let max_size = icblen;

    let mut sector = lsector + Daddr::from(ump.um_start.get());

    // Check the sparing table. Each entry represents the beginning of a packet.
    ump.with_stbl(|stbl| {
        let Some(stbl) = stbl else {
            return;
        };
        let entries = stbl.get(UdfSparingTable::SIZE..).unwrap_or(&[]);
        for i in 0..usize::try_from(ump.um_stbl_len.get()).unwrap_or(0) {
            // An entry past the table's copy ends the scan (the C reads past it).
            let Some(e) = SpareMapEntry::at(entries, i * SpareMapEntry::SIZE) else {
                break;
            };
            // The C's `int p_offset`.
            let p_offset = (lsector - Daddr::from(letoh32(e.org))) as i32;
            if p_offset < ump.um_psecs.get() && p_offset >= 0 {
                sector = Daddr::from(letoh32(e.map).wrapping_add(p_offset as u32));
                break;
            }
        }
    });

    Ok(UdfBmap::Block { sector, max_size })
}

/// `udf_vops`: the C's 18 functions; its `eopnotsupp` `remove` and its NULL slots
/// (`kqfilter` among them) the trait's defaults, `EOPNOTSUPP` (the module's deviations).
impl<G: ProcGlue> Vnops<G> for UdfVnops<G> {
    /// `vop_access`: [`udf_access`].
    fn access(
        &self,
        vp: &'static Vnode<G>,
        mode: i32,
        cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_access(vp, mode, cred)
    }

    /// `vop_bmap`: [`udf_bmap`].
    fn bmap(
        &self,
        vp: &'static Vnode<G>,
        bn: Daddr,
        vpp: Option<&mut Option<&'static Vnode<G>>>,
        bnp: Option<&mut Daddr>,
        runp: Option<&mut i32>,
    ) -> Result<(), Errno> {
        udf_bmap(vp, bn, vpp, bnp, runp)
    }

    /// `vop_lookup`: [`udf_lookup`].
    fn lookup(
        &self,
        dvp: &'static Vnode<G>,
        vpp: &mut Option<&'static Vnode<G>>,
        cnp: &mut Cn<'_, G>,
    ) -> Result<(), Errno> {
        udf_lookup(dvp, vpp, cnp)
    }

    /// `vop_getattr`: [`udf_getattr`].
    fn getattr(
        &self,
        vp: &'static Vnode<G>,
        vap: &mut Vattr,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_getattr(vp, vap)
    }

    /// `vop_open`: [`udf_open`].
    fn open(
        &self,
        _vp: &'static Vnode<G>,
        _mode: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_open()
    }

    /// `vop_close`: [`udf_close`].
    fn close(
        &self,
        _vp: &'static Vnode<G>,
        _fflag: i32,
        _cred: Cred<'_>,
        _p: Option<&Thread<G>>,
    ) -> Result<(), Errno> {
        udf_close()
    }

    /// `vop_ioctl`: [`udf_ioctl`].
    fn ioctl(
        &self,
        _vp: &'static Vnode<G>,
        _command: u64,
        _data: &mut [u8],
        _fflag: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        udf_ioctl()
    }

    /// `vop_read`: [`udf_read`].
    fn read(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _ioflag: i32,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        udf_read(vp, uio)
    }

    /// `vop_readdir`: [`udf_readdir`].
    fn readdir(
        &self,
        vp: &'static Vnode<G>,
        uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
        eofflag: &mut i32,
    ) -> Result<(), Errno> {
        udf_readdir(vp, uio, eofflag)
    }

    /// `vop_readlink`: [`udf_readlink`].
    fn readlink(
        &self,
        _vp: &'static Vnode<G>,
        _uio: &mut Uio<'_, G>,
        _cred: Cred<'_>,
    ) -> Result<(), Errno> {
        udf_readlink()
    }

    /// `vop_inactive`: [`udf_inactive`].
    fn inactive(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        udf_inactive(vp)
    }

    /// `vop_reclaim`: [`udf_reclaim`].
    fn reclaim(&self, vp: &'static Vnode<G>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        udf_reclaim(vp)
    }

    /// `vop_strategy`: [`udf_strategy`].
    fn strategy(&self, vp: &'static Vnode<G>, bp: IoBuf) -> Result<(), Errno> {
        udf_strategy(vp, bp)
    }

    /// `vop_lock`: [`udf_lock`].
    fn lock(&self, vp: &'static Vnode<G>, flags: i32) -> Result<(), Errno> {
        udf_lock(vp, flags)
    }

    /// `vop_unlock`: [`udf_unlock`].
    fn unlock(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        udf_unlock(vp)
    }

    /// `vop_pathconf`: [`udf_pathconf`].
    fn pathconf(
        &self,
        _vp: &'static Vnode<G>,
        name: i32,
        retval: &mut Register,
    ) -> Result<(), Errno> {
        udf_pathconf(name, retval)
    }

    /// `vop_islocked`: [`udf_islocked`].
    fn islocked(&self, vp: &'static Vnode<G>) -> i32 {
        udf_islocked(vp)
    }

    /// `vop_print`: [`udf_print`].
    fn print(&self, vp: &'static Vnode<G>) -> Result<(), Errno> {
        udf_print(vp)
    }
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::mem::offset_of;
    use core::sync::atomic::Ordering;

    use std::vec;
    use std::vec::Vec;

    use hal::malloc::{M_WAITOK, MArray};

    use super::*;
    use crate::fs::bio::BCSTATS;
    use crate::fs::isofs::udf::ecma167_udf::FileEntry;
    use crate::fs::isofs::udf::testudf::mkudf::{
        B, file_entry, long_ad, short_ad, tag_bytes, timestamp,
    };
    use crate::fs::isofs::udf::testudf::{attach, test_node, test_ump};
    use crate::fs::isofs::udf::udf::{M_UDFMOUNT, Umount};
    use crate::fs::mount::vfs_mount_alloc;
    use crate::fs::testfs::{TEST_CONF, TestGlue};
    use crate::fs::ufs::ffs::testffs;
    use crate::fs::vops::vop_unlock;

    #[test]
    fn leap_years() {
        assert_eq!(udf_isaleapyear(2000), 1);
        assert_eq!(udf_isaleapyear(1900), 0);
        assert_eq!(udf_isaleapyear(2024), 1);
        assert_eq!(udf_isaleapyear(2023), 0);
    }

    #[test]
    fn timestamps_follow_the_c_arithmetic() {
        let ts = |tz| Timestamp::at(&timestamp(tz), 0).unwrap();
        let mut t = Timespec::new(7, 7);
        udf_timetotimespec(&ts(None), &mut t);
        assert_eq!((t.tv_sec, t.tv_nsec), (1_791_203_696, 0));
        udf_timetotimespec(&ts(Some(120)), &mut t);
        assert_eq!(t.tv_sec, 1_791_203_696 - 7200);
        // -2047 means "no time zone".
        udf_timetotimespec(&ts(Some(-2047)), &mut t);
        assert_eq!(t.tv_sec, 1_791_203_696);
        // Bogus years are the epoch; a month past December adds nothing more.
        let mut raw = timestamp(None);
        raw[2..4].copy_from_slice(&1969u16.to_le_bytes());
        udf_timetotimespec(&Timestamp::at(&raw, 0).unwrap(), &mut t);
        assert_eq!((t.tv_sec, t.tv_nsec), (0, 0));
        let mut raw = timestamp(None);
        raw[4] = 13;
        raw[9] = 1; // centisec
        let mut u = Timespec::new(0, 0);
        udf_timetotimespec(&Timestamp::at(&raw, 0).unwrap(), &mut u);
        raw[4] = 200;
        udf_timetotimespec(&Timestamp::at(&raw, 0).unwrap(), &mut t);
        assert_eq!(t, u);
        assert_eq!(t.tv_nsec, 10_000_000);
    }

    #[test]
    fn names_translate_and_compare() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|_| {});
        let mut d = [0xffu8; 16];
        assert_eq!(udf_transname(b"\x08abc", &mut d, 4, Some(&*ump)), 3);
        assert_eq!(&d[..4], b"abc\0");
        // 16-bit names: a character past 8 bits becomes '?'.
        assert_eq!(
            udf_transname::<TestGlue>(b"\x10\x00a\x26\x3a\x00b", &mut d, 7, None),
            3
        );
        assert_eq!(&d[..4], b"a?b\0");
        // Too long, bad compression id, no room for the NUL.
        assert_eq!(udf_transname::<TestGlue>(b"\x08abc", &mut d, 300, None), 0);
        assert_eq!(udf_transname::<TestGlue>(b"\x09abc", &mut d, 4, None), 0);
        assert_eq!(
            udf_transname::<TestGlue>(b"\x08abc", &mut d[..3], 4, None),
            0
        );
        // The longest name: MAXNAMLEN characters.
        let mut long = vec![8u8];
        long.extend(core::iter::repeat_n(b'n', MAXNAMLEN - 1));
        let mut big = [0u8; MAXNAMLEN + 1];
        assert_eq!(
            udf_transname::<TestGlue>(&long, &mut big, MAXNAMLEN as i32, None),
            MAXNAMLEN - 1
        );

        assert!(!udf_cmpname(b"\x08abc", b"abc", 4, 3, &*ump));
        assert!(udf_cmpname(b"\x08abc", b"abd", 4, 3, &*ump));
        assert!(udf_cmpname(b"\x08abc", b"ab", 4, 2, &*ump));
        assert!(udf_cmpname(b"\x08", b"", 1, 0, &*ump));
        // Every trans buffer went back to its pool.
        assert_eq!(vfs::<TestGlue>().udf.trans_pool.nout(), 0);
    }

    #[test]
    fn the_hash_finds_inserts_and_forgets_unodes() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|_| {});
        let fe = file_entry(4, false, 5, 0, 0x1884, 0, 12, &short_ad(12, 5));
        let (a, va) = test_node(&ump, 4, &fe);
        let (b, vb) = test_node(&ump, 7, &fe);
        udf_hashins(&a).unwrap();
        assert_ne!(a.u_lock.status(), 0);
        vop_unlock(va).unwrap();
        udf_hashins(&b).unwrap();
        vop_unlock(vb).unwrap();

        // Found, referenced and locked, as the C's.
        let uses = va.usecount();
        let vp = udf_hashlookup(&ump, 4, LK_EXCLUSIVE).unwrap();
        assert!(vp.is_some_and(|vp| core::ptr::eq(vp, va)));
        assert_ne!(a.u_lock.status(), 0);
        assert_eq!(va.usecount(), uses + 1);
        vput(va);
        assert!(
            udf_hashlookup(&ump, 7, LK_EXCLUSIVE)
                .unwrap()
                .is_some_and(|vp| core::ptr::eq(vp, vb))
        );
        vput(vb);
        assert!(udf_hashlookup(&ump, 9, LK_EXCLUSIVE).unwrap().is_none());

        // Off the hash, it is no longer found.
        udf_hashrem(&a).unwrap();
        assert!(udf_hashlookup(&ump, 4, LK_EXCLUSIVE).unwrap().is_none());
        udf_hashrem(&b).unwrap();

        // A mount whose table is not made yet (the C's NULL chain).
        let bare = Umount::new(vfs_mount_alloc(None, &TEST_CONF[0]), testffs::devvp()).alloc();
        assert_eq!(
            udf_hashlookup(&bare, 4, LK_EXCLUSIVE).err(),
            Some(Errno::ENOENT)
        );
    }

    /// Changes the ICB tag of a file entry block: the strategy type and the flags.
    fn set_icb(fe: &mut [u8], strat: u16, flags: u16) {
        let icb = offset_of!(FileEntry, icbtag);
        fe[icb + 4..icb + 6].copy_from_slice(&strat.to_le_bytes());
        fe[icb + 18..icb + 20].copy_from_slice(&flags.to_le_bytes());
    }

    #[test]
    fn bmap_walks_the_allocation_descriptors() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|u| u.um_start.set(20));
        let block = |sector, max_size| Ok(UdfBmap::Block { sector, max_size });

        // Two short ADs: 2048 bytes at lb 5, 4096 at lb 9.
        let ads = [short_ad(2048, 5), short_ad(4096, 9)].concat();
        let fe = file_entry(3, false, 5, 0, 0x1884, 0, 6144, &ads);
        let (up, _) = test_node(&ump, 3, &fe);
        assert_eq!(udf_bmap_internal(&up, 0), block(25, 2048));
        assert_eq!(udf_bmap_internal(&up, 2047), block(25, 2048));
        assert_eq!(udf_bmap_internal(&up, 2048), block(29, 4096));
        assert_eq!(udf_bmap_internal(&up, 2048 + 2048), block(30, 4096));
        // Past the last extent: zero-length descriptors to `l_ad`, then out of bounds.
        assert_eq!(udf_bmap_internal(&up, 6144), Err(Errno::EINVAL));

        // The same in an extended file entry (its data[] at 216).
        let xfe = file_entry(3, true, 5, 0, 0x1884, 0, 6144, &ads);
        let (xup, _) = test_node(&ump, 3, &xfe);
        assert_eq!(udf_bmap_internal(&xup, 2048), block(29, 4096));

        // A long AD.
        let fe = file_entry(3, false, 5, 1, 0x1884, 0, 2048, &long_ad(2048, 7));
        let (lup, _) = test_node(&ump, 3, &fe);
        assert_eq!(udf_bmap_internal(&lup, 100), block(27, 2048));

        // The data in the entry: the entry's sector.
        let fe = file_entry(3, true, 5, 3, 0x1884, 0, 5, b"hello");
        let (eup, _) = test_node(&ump, 3, &fe);
        assert_eq!(
            udf_bmap_internal(&eup, 0),
            Ok(UdfBmap::Invalid { sector: 23 })
        );

        // Strategy 4096, an unknown strategy, extended ADs: ENODEV.
        for (strat, flags) in [(4096, 0), (5, 0), (4, 2)] {
            let mut fe = file_entry(3, false, 5, 0, 0x1884, 0, 6144, &ads);
            set_icb(&mut fe, strat, flags);
            let (bad, _) = test_node(&ump, 3, &fe);
            assert_eq!(udf_bmap_internal(&bad, 0), Err(Errno::ENODEV));
        }
    }

    #[test]
    fn bmap_remaps_through_the_sparing_table() {
        let (_g, _p) = attach(vec![0; B]);
        let ump = test_ump(|u| {
            u.um_start.set(20);
            u.um_psecs.set(4);
            u.um_stbl_len.set(2);
            // The table's 56-byte header, then (org 24, map 100), (org 40, map 200).
            let mut t = vec![0u8; UdfSparingTable::SIZE];
            t[..16].copy_from_slice(&tag_bytes(0, 0));
            for (org, map) in [(24u32, 100u32), (40, 200)] {
                t.extend(org.to_le_bytes());
                t.extend(map.to_le_bytes());
            }
            let Some(mut a) = MArray::new(M_UDFMOUNT, M_WAITOK, t.len(), |_| 0u8) else {
                std::panic!("malloc");
            };
            a.copy_from_slice(&t);
            u.set_stbl(a);
        });
        let fe = file_entry(
            3,
            false,
            5,
            0,
            0x1884,
            0,
            16 * 2048,
            &short_ad(16 * 2048, 24),
        );
        let (up, _) = test_node(&ump, 3, &fe);
        let sector = |off| match udf_bmap_internal(&up, off) {
            Ok(UdfBmap::Block { sector, .. }) => sector,
            r => std::panic!("{r:?}"),
        };
        // lsector 24 + n: the packet at 24 is spared to 100 for its 4 sectors.
        assert_eq!(sector(0), 100);
        assert_eq!(sector(3 * 2048), 103);
        assert_eq!(sector(4 * 2048), 28 + 20);
        assert_eq!(sector(15 * 2048), 39 + 20);
    }

    #[test]
    fn readatoffset_reads_the_entry_or_a_block() {
        let mut img = vec![0u8; 32 * B];
        let data: Vec<u8> = (0..B).map(|i| (i % 251) as u8).collect();
        img[25 * B..26 * B].copy_from_slice(&data);
        let (_g, _p) = attach(img);
        let ump = test_ump(|u| u.um_start.set(20));
        let busy = BCSTATS.busymapped.load(Ordering::Relaxed);

        let fe = file_entry(3, false, 5, 0, 0x1884, 0, 2048, &short_ad(2048, 5));
        let (up, _) = test_node(&ump, 3, &fe);
        let mut size = 0;
        let d = udf_readatoffset(&up, &mut size, 0).unwrap();
        assert!(matches!(d, UdfData::Buf(_, 0)));
        assert_eq!(size, 2048);
        assert_eq!(d.bytes(&up, size), &data[..]);
        drop(d);
        let mut size = 10;
        let d = udf_readatoffset(&up, &mut size, 100).unwrap();
        assert_eq!((size, d.bytes(&up, size)), (10, &data[100..110]));
        drop(d);
        // A size past the extent is the extent's; past the block's end, what it holds.
        let mut size = 9000;
        let d = udf_readatoffset(&up, &mut size, 2000).unwrap();
        assert_eq!((size, d.bytes(&up, size)), (48, &data[2000..]));
        drop(d);
        assert_eq!(BCSTATS.busymapped.load(Ordering::Relaxed), busy);

        // In the entry: from the offset on.
        let fe = file_entry(3, true, 5, 3, 0x1884, 0, 5, b"hello");
        let (eup, _) = test_node(&ump, 3, &fe);
        for (off, want) in [(0, &b"hello"[..]), (2, b"llo"), (9, b"")] {
            let mut size = 0;
            let d = udf_readatoffset(&eup, &mut size, off).unwrap();
            assert!(matches!(d, UdfData::Entry(_)));
            assert_eq!((size as usize, d.bytes(&eup, size)), (want.len(), want));
        }
    }
}
/* </TESTS> */
