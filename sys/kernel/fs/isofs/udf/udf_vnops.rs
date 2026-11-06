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
//! UDF vnode operations: the unode hash, the time stamps, the names, and the read of a
//! node's data at an offset through its block map (N5.8u1); the vnode operations and their
//! table (lookup, read, readdir, getattr, the lock) follow with the mount (N5.8u2).
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
//!   the C may hand out bytes past the buffer when the offset is not block aligned.
//! - For a file recorded in its file entry, `udf_readatoffset` returns the data from
//!   `offset` on; the C returns it from its start whatever the offset, so a `read` that does
//!   not start at 0, a directory read resumed past its start and a VAT lookup past the first
//!   entry got the wrong bytes (LZ's).
//! - `udf_bmap_internal` returns [`UdfBmap`]: a block, or `UDF_INVALID_BMAP` (the file data
//!   is in the file entry) as its own variant; `udf_bmap` (N5.8u2) passes that case on as the
//!   C's -1, `ERESTART`'s value ([`UDF_INVALID_BMAP`]). An allocation descriptor read past
//!   the end of the file entry's copy reads zeros (the C reads past an extended file entry's
//!   allocation). The descriptor offset is computed in 64 bits, so a huge `l_ad` ends the
//!   walk with `EINVAL` where the C's `int` would overflow.
//! - `udf_transname` returns 0 (failure) when `destname` cannot hold the name and its NUL;
//!   `udf_cmpname` returns a `bool` (`true` where the C returns nonzero: no match).
//! - `udf_timetotimespec` ignores months past December (the C reads past `mon_lens`).
//! - The `DIAGNOSTIC` messages of `udf_transname` under feature `diagnostic`.
//!
//! ## Redesign
//! - LZ's hash (a raw `hashinit` slice of `sys/queue.rs` chains linked and unlinked in
//!   `unsafe` blocks under a legacy `Mutex`) -> the mount's [`UdfHash`] of pinned unodes
//!   (`PoolArc::as_pin`) under `um_hashmtx`, a `hal::mutex::Mutexed<()>` at `IPL_NONE`
//!   whose guard is the chains' witness (the C's mutex, never held across `vget` or a sleep:
//!   `udf_hashlookup` drops it before `vget` and walks again on `ENOENT` or a recycled vnode,
//!   udf_vnops.c:113-140); `udf_hashins` inserts without a duplicate scan, as the C (two
//!   `vget`s of one file may hash two unodes: the C's race, kept).
//! - LZ's `udf_data` (an `unsafe fn` lending a buffer's bytes) and `fentry_read` over the
//!   unode's raw file entry -> [`UdfData::bytes`] over the token or the unode's
//!   `MArray`; `udf_bmap_internal` reads the members of the parsed [`FentryHdr`] and the
//!   allocation descriptors out of the copy at the C's offsets (8 or 16 bytes a step, from
//!   the first on every call, as the C's walk; D-N5.8-5).
//! - LZ's name buffers (a raw `udf_trans_pool` item cast to a slice in an `unsafe` block)
//!   -> `PoolBox`es of the glue's `trans_pool` (`udf_cmpname`'s char buffer is its item's
//!   bytes, `hal::user::bytes_of_mut`). The item is zeroed when taken (the C's
//!   `PR_WAITOK` leaves it as it was: 510 bytes written per name, a cost named for ROADMAP
//!   N12). 0 `unsafe` in this part (LZ's file: 18, the rest N5.8u2's).
//!
//! [`UdfHash`]: crate::fs::isofs::udf::udf::UdfHash
//! [`FentryHdr`]: crate::fs::isofs::udf::udf::FentryHdr

use hal::pool::PR_WAITOK;
use hal::sys::dirent::MAXNAMLEN;
use hal::sys::endian::{letoh16, letoh32};
use hal::sys::errno::Errno;
use hal::sys::types::{Daddr, Off};
use hal::user::bytes_of_mut;

use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::fs::isofs::udf::ecma167_udf::{
    LongAd, ShortAd, SpareMapEntry, Timestamp, UdfSparingTable, geticblen,
};
use crate::fs::isofs::udf::udf::{
    UdfData, Udfino, Umount, Unicode, Unode, UnodeRef, udf_readlblks,
};
use crate::fs::isofs::udf::udf_subr::udf_rawnametounicode;
use crate::fs::subr::{vfs, vget, vput};
use crate::fs::vnode::{LK_EXCLUSIVE, LK_RETRY, Vnode, vn_lock};
use crate::prf::printf;
use crate::proc::ProcGlue;

/// `MAXBSIZE` (`<sys/param.h>`).
const MAXBSIZE: i32 = 64 * 1024;

/// `UDF_INVALID_BMAP`: the C's -1, `ERESTART`'s value (the module's deviations).
pub const UDF_INVALID_BMAP: Errno = Errno::ERESTART;

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
            h.chain(id)
                .iter(&g)
                .find(|up| up.u_ino == id)
                .map(|up| (up.u_vnode, up.u_vnode.id()))
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

    let _ = vn_lock(up.u_vnode, LK_EXCLUSIVE | LK_RETRY);
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
    let Some(mut transname) = vfs::<G>()
        .udf
        .trans_pool
        .get(PR_WAITOK, || [0 as Unicode; MAXNAMLEN])
    else {
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
    let Some(mut item) = vfs::<G>()
        .udf
        .trans_pool
        .get(PR_WAITOK, || [0 as Unicode; MAXNAMLEN])
    else {
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
            let len = up.u_fentry.len() as u64;
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
    let avail = t.bytes().len().saturating_sub(off);
    *size = (*size as u32).min(avail as u32) as i32;
    Ok(UdfData::Buf(t, off))
}

/// `N` bytes of the node's file entry copy from `off` on, zeros past its end.
fn fentry_read<G: ProcGlue, const N: usize>(up: &Unode<G>, off: i64) -> [u8; N] {
    let mut out = [0u8; N];
    if let Ok(off) = usize::try_from(off)
        && let Some(src) = up.u_fentry.get(off..)
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
    if let Some(stbl) = ump.stbl() {
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
    }

    Ok(UdfBmap::Block { sector, max_size })
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
