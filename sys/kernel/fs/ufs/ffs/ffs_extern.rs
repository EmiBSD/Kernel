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
 * Copyright (c) 1991, 1993, 1994
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
 *	@(#)ffs_extern.h	8.3 (Berkeley) 4/16/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<ufs/ffs/ffs_extern.h>`: the `CTL_VFS` names of the fast file system (`FFS_*`,
//! `FFS_NAMES`); the prototypes, operation tables and pools it declares are the items of
//! the modules that define them.
//!
//! Upstream: sys/ufs/ffs/ffs_extern.h @ 3ce1f3f79392
//! LZ: sys/ufs/ffs/ffs_extern.rs@3c62ede68802
//!
//! ## Deviations
//! - `ffs_fifovops` and `ffsfifo_reclaim` wait for `miscfs/fifofs` (`option FIFO`).
//! - `FFS_NAMES` is an array of `Ctlname`s.
//!
//! ## Redesign
//! - N5.5b2: LZ's `pub use` of the prototypes, the tables and the pools from `ffs_*.rs`
//!   goes: their callers name the defining modules (`ffs_vfsops`, `ffs_vnops`, `ffs_alloc`,
//!   ...), and the pools are the glue's tables' (`UfsTables`); the constants are LZ's.

use hal::sys::sysctl::{CTLTYPE_INT, Ctlname};

/// `FFS_CLUSTERREAD`: cluster reading enabled.
pub const FFS_CLUSTERREAD: i32 = 1;
/// `FFS_CLUSTERWRITE`: cluster writing enabled.
pub const FFS_CLUSTERWRITE: i32 = 2;
/// `FFS_REALLOCBLKS`: block reallocation enabled.
pub const FFS_REALLOCBLKS: i32 = 3;
/// `FFS_ASYNCFREE`: asynchronous block freeing enabled.
pub const FFS_ASYNCFREE: i32 = 4;
/// `FFS_MAX_SOFTDEPS`: maximum structs before slowdown.
pub const FFS_MAX_SOFTDEPS: i32 = 5;
/// `FFS_SD_TICKDELAY`: ticks to pause during slowdown.
pub const FFS_SD_TICKDELAY: i32 = 6;
/// `FFS_SD_WORKLIST_PUSH`: # of worklist cleanups.
pub const FFS_SD_WORKLIST_PUSH: i32 = 7;
/// `FFS_SD_BLK_LIMIT_PUSH`: # of times block limit neared.
pub const FFS_SD_BLK_LIMIT_PUSH: i32 = 8;
/// `FFS_SD_INO_LIMIT_PUSH`: # of times inode limit neared.
pub const FFS_SD_INO_LIMIT_PUSH: i32 = 9;
/// `FFS_SD_BLK_LIMIT_HIT`: # of times block slowdown imposed.
pub const FFS_SD_BLK_LIMIT_HIT: i32 = 10;
/// `FFS_SD_INO_LIMIT_HIT`: # of times inode slowdown imposed.
pub const FFS_SD_INO_LIMIT_HIT: i32 = 11;
/// `FFS_SD_SYNC_LIMIT_HIT`: # of synchronous slowdowns imposed.
pub const FFS_SD_SYNC_LIMIT_HIT: i32 = 12;
/// `FFS_SD_INDIR_BLK_PTRS`: bufs redirtied as indir ptrs not written.
pub const FFS_SD_INDIR_BLK_PTRS: i32 = 13;
/// `FFS_SD_INODE_BITMAP`: bufs redirtied as inode bitmap not written.
pub const FFS_SD_INODE_BITMAP: i32 = 14;
/// `FFS_SD_DIRECT_BLK_PTRS`: bufs redirtied as direct ptrs not written.
pub const FFS_SD_DIRECT_BLK_PTRS: i32 = 15;
/// `FFS_SD_DIR_ENTRY`: bufs redirtied as dir entry cannot write.
pub const FFS_SD_DIR_ENTRY: i32 = 16;
/// `FFS_DIRHASH_DIRSIZE`: min directory size, in bytes.
pub const FFS_DIRHASH_DIRSIZE: i32 = 17;
/// `FFS_DIRHASH_MAXMEM`: max kvm to use, in bytes.
pub const FFS_DIRHASH_MAXMEM: i32 = 18;
/// `FFS_DIRHASH_MEM`: current mem usage, in bytes.
pub const FFS_DIRHASH_MEM: i32 = 19;
/// `FFS_MAXID`: number of valid ffs ids.
pub const FFS_MAXID: i32 = 20;

/// `FFS_NAMES`.
pub const FFS_NAMES: [Ctlname; FFS_MAXID as usize] = {
    let mut n = [Ctlname::NONE; FFS_MAXID as usize];
    n[17] = Ctlname::new(b"dirhash_dirsize", CTLTYPE_INT);
    n[18] = Ctlname::new(b"dirhash_maxmem", CTLTYPE_INT);
    n[19] = Ctlname::new(b"dirhash_mem", CTLTYPE_INT);
    n
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn names_match_the_c_header() {
        let defs = crate::reftest::defines("sys/ufs/ffs/ffs_extern.h");
        for (name, value) in [
            ("FFS_CLUSTERREAD", FFS_CLUSTERREAD),
            ("FFS_SD_DIR_ENTRY", FFS_SD_DIR_ENTRY),
            ("FFS_DIRHASH_DIRSIZE", FFS_DIRHASH_DIRSIZE),
            ("FFS_DIRHASH_MAXMEM", FFS_DIRHASH_MAXMEM),
            ("FFS_DIRHASH_MEM", FFS_DIRHASH_MEM),
            ("FFS_MAXID", FFS_MAXID),
        ] {
            std::assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(value)),
                "{name}"
            );
        }
    }

    #[test]
    fn the_names_are_the_dirhash_ones_only() {
        let named: std::vec::Vec<usize> = (0..FFS_MAXID as usize)
            .filter(|&i| FFS_NAMES[i] != Ctlname::NONE)
            .collect();
        std::assert_eq!(
            named,
            std::vec![
                FFS_DIRHASH_DIRSIZE as usize,
                FFS_DIRHASH_MAXMEM as usize,
                FFS_DIRHASH_MEM as usize
            ]
        );
    }
}
/* </TESTS> */
