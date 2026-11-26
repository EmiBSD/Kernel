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
 * Copyright (c) 1996 Theo de Raadt
 * Copyright (c) 1982, 1986, 1988 Regents of the University of California.
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
 */
/* </LICENSES> */

/* <CODE> */
//! The disk label I/O of arm64's `disksubr.c` (`readdisklabel`, `writedisklabel`): the
//! label read from (or written to) a disk through its driver's strategy routine, under an
//! MBR, a GPT or nothing, or spoofed from an ISO 9660 or UDF volume.
//!
//! Upstream: sys/arch/arm64/arm64/disksubr.c @ 3ce1f3f79392
//! LZ: sys/arch/arm64/arm64/disksubr.rs@e8249449d905
//!
//! amd64's `disksubr.c` is this file plus the BIOS geometry lookup (`bios_getdiskinfo`):
//! N10's step decides where that lives (decision 44).
//!
//! ## Deviations
//! - `spoofonly` is a `bool`; the functions return `Result` (`docs/C_TO_RUST.md`).
//! - `readdisklabel` releases its `geteblk` buffer before the ISO 9660 and UDF spoofs get
//!   their own, where the C keeps it until its `done:` label: the strategy closure is lent
//!   to one buffer at a time. Nothing reads the first buffer meanwhile.
//!
//! ## Redesign
//! - N7.1d: the machine's label I/O was `bsd`'s (`arch/arm64/arm64/disksubr.rs`, reached
//!   through `machine::disklabel`'s `MachineDisklabel` trait), over a `&'static Buf` with an
//!   `unsafe` view of `b_data`; it is the kernel's, over the disk layer's `LabelBuf` (the
//!   `geteblk` token and the driver's [`Strat`] closure), its `brelse` the buffer's drop. No
//!   machine code is involved: the label's place (`LABELSECTOR`) is `hal::sys::disklabel`'s.

use core::sync::atomic::Ordering;

use hal::sys::disklabel::{DISKLABEL_SIZE, DOS_LABELSECTOR, Disklabel, dl_blkoffset, dl_blktosec};
use hal::sys::types::{Daddr, Dev};

use crate::dev::disk::{DISK_CHANGE, LabelBuf, Strat, initdisklabel, readdisksector, readdoslabel};
use crate::fs::buf::{B_BUSY, B_DONE, B_RAW, B_READ, B_WRITE};
use crate::proc::ProcGlue;
use crate::sys::errno::Errno;

/// Attempt to read a disk label from a device using the indicated strategy routine. The
/// label must be partly set up before this: secpercyl, secsize and anything required for a
/// block i/o read operation in the driver's strategy/start routines must be filled in before
/// calling us.
///
/// If dos partition table requested, attempt to load it and find disklabel inside a DOS
/// partition.
///
/// We would like to check if each MBR has a valid DOSMBR_SIGNATURE, but we cannot because
/// it doesn't always exist. So.. we assume the MBR is valid.
pub fn readdisklabel<G: ProcGlue>(
    dev: Dev,
    io: Strat<'_>,
    lp: &mut Disklabel,
    spoofonly: bool,
) -> Result<(), Errno> {
    let result = (|| {
        initdisklabel(lp)?;

        // get a buffer and initialize it
        let error = {
            let mut bp = LabelBuf::new(dev, lp.d_secsize as usize, &mut *io);
            readdoslabel(&mut bp, lp, None, spoofonly)
        };
        // Each spoof is tried in turn while nothing has matched (the C's `goto done`).
        #[cfg(feature = "cd9660")]
        let error = match error {
            Err(_) => {
                crate::fs::isofs::cd9660::cd9660_vfsops::iso_disklabelspoof(dev, &mut *io, lp)
            }
            ok => ok,
        };
        #[cfg(feature = "udf")]
        let error = match error {
            Err(_) => crate::fs::isofs::udf::udf_subr::udf_disklabelspoof::<G>(dev, &mut *io, lp),
            ok => ok,
        };
        error
    })();

    DISK_CHANGE.store(1, Ordering::Relaxed);
    result
}

/// Write disk label back to device after modification.
pub fn writedisklabel(dev: Dev, io: Strat<'_>, lp: &mut Disklabel) -> Result<(), Errno> {
    let result = {
        // get a buffer and initialize it
        let mut bp = LabelBuf::new(dev, lp.d_secsize as usize, io);
        (|| {
            let mut partoff: Daddr = -1;
            if readdoslabel(&mut bp, lp, Some(&mut partoff), true).is_err() {
                return Err(Errno::EIO);
            }

            // Read it in, slap the new label in, and write it back out
            let blk = (partoff + DOS_LABELSECTOR) as u64;
            readdisksector(&mut bp, lp, dl_blktosec(lp, blk))?;
            let offset = dl_blkoffset(lp, blk) as usize;

            let Some(dst) = bp.data().get_mut(offset..) else {
                return Err(Errno::EIO);
            };
            let n = dst.len().min(DISKLABEL_SIZE);
            dst[..n].copy_from_slice(&lp.as_bytes()[..n]);
            let b = bp.buf();
            b.clr(B_READ | B_WRITE | B_DONE);
            b.set(B_BUSY | B_WRITE | B_RAW);
            bp.strategy()
        })()
        // done: bp->b_flags |= B_INVAL; brelse(bp): the buffer's drop.
    };

    DISK_CHANGE.store(1, Ordering::Relaxed);
    result
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests: a label written to a disk under an MBR and under a GPT, then read back
    // (the in-memory disk of the disk layer's tests, through a strategy closure); a disk
    // whose strategy fails refuses the write.

    use std::sync::Mutex;
    use std::vec::Vec;
    use std::{assert_eq, vec};

    use super::*;
    use crate::dev::disk::dkcksum;
    use crate::fs::buf::{B_ERROR, BufRef};
    use hal::sys::disklabel::{
        DISKMAGIC, DOSMBR_SIGNATURE, DOSPARTOFF, DOSPTYP_OPENBSD, FS_BSDFFS, RAW_PART,
        dl_getpoffset, dl_setdsize, dl_setpsize,
    };

    /// The disk the strategy reads and writes.
    static IMG: Mutex<Vec<u8>> = Mutex::new(Vec::new());

    fn img_io(mut t: BufRef) -> (BufRef, Result<(), Errno>) {
        let off = t.buf().blkno() as usize * 512;
        let len = t.buf().bcount() as usize;
        let mut img = IMG.lock().unwrap_or_else(|e| e.into_inner());
        if off + len > img.len() {
            t.buf().set_error(Some(Errno::EIO));
            t.buf().set(B_ERROR);
            return (t, Err(Errno::EIO));
        }
        if t.buf().isset(B_READ) {
            t.data()[..len].copy_from_slice(&img[off..off + len]);
        } else {
            img[off..off + len].copy_from_slice(&t.bytes()[..len]);
        }
        (t, Ok(()))
    }

    /// The geometry a driver fills in before `readdisklabel`: `sectors` of 512 bytes.
    fn geometry(sectors: u32) -> Disklabel {
        let mut d = Disklabel::zeroed();
        d.d_secsize = 512;
        d.d_nsectors = sectors;
        d.d_ntracks = 1;
        d.d_ncylinders = 1;
        d.d_secpercyl = sectors;
        dl_setdsize(&mut d, u64::from(sectors));
        d.d_version = 1;
        d
    }

    /// A label to write: `a` FFS from sector `off`.
    fn label(sectors: u32, off: u64) -> Disklabel {
        let mut d = geometry(sectors);
        d.d_magic = DISKMAGIC;
        d.d_magic2 = DISKMAGIC;
        d.d_npartitions = 3;
        hal::sys::disklabel::dl_setpoffset(&mut d.d_partitions[0], off);
        dl_setpsize(&mut d.d_partitions[0], u64::from(sectors) - off);
        d.d_partitions[0].p_fstype = FS_BSDFFS;
        dl_setpsize(&mut d.d_partitions[RAW_PART as usize], u64::from(sectors));
        d.d_checksum = dkcksum(&d);
        d
    }

    fn write_then_read(sectors: u32, off: u64) {
        let mut io = img_io;
        let mut lp = label(sectors, off);
        assert_eq!(writedisklabel(0, &mut io, &mut lp), Ok(()));
        let mut back = geometry(sectors);
        assert_eq!(
            readdisklabel::<crate::fs::testfs::TestGlue>(0, &mut io, &mut back, false),
            Ok(())
        );
        assert_eq!(back.d_partitions[0].p_fstype, FS_BSDFFS);
        assert_eq!(dl_getpoffset(&back.d_partitions[0]), off);
        assert_eq!(dkcksum(&back), 0);
        assert_eq!(DISK_CHANGE.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn a_label_written_on_a_bare_disk_reads_back() {
        let _g = crate::testutil::bufcache_install(1 << 20, 64);
        *IMG.lock().unwrap_or_else(|e| e.into_inner()) = vec![0u8; 64 * 512];
        write_then_read(64, 0);
        // At sector 1 (LABELSECTOR), offset 0.
        let img = IMG.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(&img[512..516], &DISKMAGIC.to_ne_bytes());
    }

    #[test]
    fn a_label_written_under_an_mbr_lands_in_the_openbsd_partition() {
        let _g = crate::testutil::bufcache_install(1 << 20, 64);
        let mut img = vec![0u8; 256 * 512];
        let e = DOSPARTOFF + 16;
        img[e + 4] = DOSPTYP_OPENBSD;
        img[e + 8..e + 12].copy_from_slice(&64u32.to_le_bytes());
        img[e + 12..e + 16].copy_from_slice(&192u32.to_le_bytes());
        img[510..512].copy_from_slice(&DOSMBR_SIGNATURE.to_le_bytes());
        *IMG.lock().unwrap_or_else(|e| e.into_inner()) = img;
        write_then_read(256, 64);
        let img = IMG.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(&img[65 * 512..65 * 512 + 4], &DISKMAGIC.to_ne_bytes());
    }

    #[test]
    fn a_disk_that_cannot_be_read_refuses_the_write() {
        let _g = crate::testutil::bufcache_install(1 << 20, 64);
        IMG.lock().unwrap_or_else(|e| e.into_inner()).clear();
        let mut io = img_io;
        let mut lp = label(64, 0);
        assert_eq!(writedisklabel(0, &mut io, &mut lp), Err(Errno::EIO));
    }
}
/* </TESTS> */
