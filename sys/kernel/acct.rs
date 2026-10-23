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

/*-
 * Copyright (c) 1994 Christopher G. Demetriou
 * Copyright (c) 1982, 1986, 1989, 1993
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)kern_acct.c	8.1 (Berkeley) 6/14/93
 */

/*-
 * Copyright (c) 1990, 1993, 1994
 *	The Regents of the University of California.  All rights reserved.
 * (c) UNIX System Laboratories, Inc.
 * All or some portions of this file are derived from material licensed
 * to the University of California by American Telephone and Telegraph
 * Co. or Unix System Laboratories, Inc. and are reproduced herein with
 * the permission of UNIX System Laboratories, Inc.
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
 *	@(#)acct.h	8.3 (Berkeley) 7/10/94
 */
/* </LICENSES> */

/* <CODE> */
//! Process accounting (`option ACCOUNTING`, configured as in GENERIC): `kern/kern_acct.c`
//! and `<sys/acct.h>`: `acct(2)`, the record `exit1` writes (`acct_process`),
//! `encode_comp_t`, the free-space watcher thread (`acct_start`, `acct_thread`) and
//! `acct_shutdown` (`vfs_shutdown`).
//!
//! Upstream: sys/kern/kern_acct.c @ 3ce1f3f79392
//! Upstream: sys/sys/acct.h @ 3ce1f3f79392
//! LZ: sys/kern/kern_acct.rs@3c62ede68802
//! LZ: sys/sys/acct.rs@3c62ede68802
//!
//! The accounting file is a vnode of the file systems, which are `bsd`'s until N5: the
//! kernel holds it as an opaque [`AcctSink`] that `bsd`'s [`AcctFile`] opened (bridges.toml
//! `AcctFile`, paid by N5): `vn_open` with `UNVEIL_WRITE` and the `VREG` check, `vn_rdwr`
//! of a record, `VFS_STATFS`, `vn_close`, all inside the bridge.
//!
//! ## Deviations
//! - The sink is a `Box`, so `acct(2)` with a path allocates one small `temp` block
//!   (`vmstat -m`'s `temp` row) where the C keeps the vnode pointer.
//! - `ac_btime` is `nanotime() - nanouptime()` plus the start (the boot time as the uptime
//!   bridge gives it; the C's `nanoboottime` is the same less any suspended time, which QEMU
//!   never has).
//! - `acct_thread` asks for the free space through the sink, which hands `VFS_STATFS` the
//!   watcher thread where the C passes NULL (LZ's).
//!
//! ## Redesign
//! - LZ's `acctp`/`savacctp` in a `StaticCell` read and written through an `unsafe fn` whose
//!   contract was "hold `acct_lock`" -> a `Rwlocked<AcctState>` "acctlk": the lock owns the
//!   two sinks, `sys_acct`, `acct_thread` and `acct_shutdown` change them through its writer,
//!   `acct_process` reads them through its reader; the C's unlocked peek is an atomic flag
//!   kept with them ([`ACCT_ON`]).
//! - `acct_proc`, a thread pointer used only as a wait channel and a "running" flag ->
//!   [`ACCT_RUNNING`], whose address is the channel.
//! - `ac_tty` is the session's terminal (`s_ttyp->t_dev` under `PS_CONTROLT`, through the
//!   glue's `session_ttydev` until N4.7b), where LZ wrote -1 (the user's decision 15 of N4.6:
//!   `lastcomm` names the terminal again).

use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use hal::bridges::AlreadyRegistered;
use hal::klock;
use hal::once::OnceStatic;
use hal::rwlock::Rwlocked;
use hal::sys::errno::Errno;
use hal::sys::types::{Dev, Gid, Pid, Uid};
use hal::user::UserPtr;

use crate::assert::kpanic;
use crate::clock;
use crate::proc::{_MAXCOMLEN, PS_CONTROLT, ProcGlue, Process, Thread};
use crate::resource::{Tusage, calctsru, tuagg_get_process};

/// `comp_t`: a 3 bits base 8 exponent, 13 bit fraction "floating point" number; units are
/// 1/[`AHZ`] seconds.
pub type Comp = u16;

/// `struct acct`: one accounting record, written to the file as its 64 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Acct {
    /// `ac_comm`: command name, incl NUL.
    pub ac_comm: [u8; _MAXCOMLEN],
    /// `ac_utime`: user time.
    pub ac_utime: Comp,
    /// `ac_stime`: system time.
    pub ac_stime: Comp,
    /// `ac_etime`: elapsed time.
    pub ac_etime: Comp,
    /// `ac_io`: count of IO blocks.
    pub ac_io: Comp,
    /// `ac_btime`: starting time.
    pub ac_btime: i64,
    /// `ac_uid`: user id.
    pub ac_uid: Uid,
    /// `ac_gid`: group id.
    pub ac_gid: Gid,
    /// `ac_mem`: average memory usage.
    pub ac_mem: u32,
    /// `ac_tty`: controlling tty, or -1.
    pub ac_tty: Dev,
    /// `ac_pid`: process id.
    pub ac_pid: Pid,
    /// `ac_flag`: accounting flags.
    pub ac_flag: u32,
}

hal::abi_pod!(Acct {
    ac_comm: [u8; _MAXCOMLEN],
    ac_utime: Comp,
    ac_stime: Comp,
    ac_etime: Comp,
    ac_io: Comp,
    ac_btime: i64,
    ac_uid: Uid,
    ac_gid: Gid,
    ac_mem: u32,
    ac_tty: Dev,
    ac_pid: Pid,
    ac_flag: u32,
});

/// `AFORK`: fork'd but not exec'd.
pub const AFORK: u32 = 0x0000_0001;
/// `AMAP`: killed by syscall or stack mapping violation.
pub const AMAP: u32 = 0x0000_0004;
/// `ACORE`: dumped core.
pub const ACORE: u32 = 0x0000_0008;
/// `AXSIG`: killed by a signal.
pub const AXSIG: u32 = 0x0000_0010;
/// `APLEDGE`: killed due to pledge violation.
pub const APLEDGE: u32 = 0x0000_0020;
/// `ATRAP`: memory access violation.
pub const ATRAP: u32 = 0x0000_0040;
/// `AUNVEIL`: unveil access violation.
pub const AUNVEIL: u32 = 0x0000_0080;
/// `APINSYS`: killed by syscall pin violation.
pub const APINSYS: u32 = 0x0000_0200;
/// `ABTCFI`: BT CFI violation.
pub const ABTCFI: u32 = 0x0000_0400;

/// `AHZ`: 1/AHZ is the granularity of the data encoded in the `comp_t` fields. This is not
/// necessarily equal to hz.
pub const AHZ: u32 = 64;

/// `MANTSIZE`: 13 bit mantissa.
const MANTSIZE: u32 = 13;
/// `EXPSIZE`: base 8 (3 bit) exponent.
const EXPSIZE: u32 = 3;
/// `MAXFRACT`: maximum fractional value.
const MAXFRACT: u64 = (1 << MANTSIZE) - 1;
/// `PPAUSE`.
const PPAUSE: i32 = 40;
/// `LOG_NOTICE`.
const LOG_NOTICE: i32 = 5;

/// An open accounting file (`acctp`/`savacctp`, a `VREG` vnode opened `FWRITE | O_APPEND`),
/// as `bsd`'s file systems keep it.
pub trait AcctSink: Send + Sync {
    /// `vn_rdwr(UIO_WRITE, vp, rec, IO_APPEND|IO_UNIT|IO_NOLIMIT, ...)` with the running
    /// thread's credentials.
    fn write(&self, rec: &[u8]) -> Result<(), Errno>;
    /// `VFS_STATFS` of its file system: `(f_bavail, f_blocks)`, `None` when it has no mount.
    fn free_space(&self) -> Option<(i64, i64)>;
    /// `vp->v_type == VBAD`: the vnode was revoked (its file system forcibly unmounted).
    fn is_bad(&self) -> bool;
    /// `vn_close(vp, FWRITE, cred, p)`: with the running thread's credentials when
    /// `by_curproc` (`acct(2)`), `NOCRED` otherwise (the watcher, `acct_shutdown`).
    fn close(self: Box<Self>, by_curproc: bool);
}

/// The file systems' half of `acct(2)` (bridges.toml `AcctFile`, N5): opening the file.
pub trait AcctFile: Sync {
    /// `vn_open` of the user path `path` for `FWRITE | O_APPEND` with `UNVEIL_WRITE` by the
    /// running thread, unlocked; `EACCES` (closed again) when it is not a regular file.
    fn open(&self, path: UserPtr<u8>) -> Result<Box<dyn AcctSink>, Errno>;
}

/// The registered [`AcctFile`].
static ACCT_FILE: OnceStatic<&'static dyn AcctFile> = OnceStatic::new();

/// Registers the [`AcctFile`] bridge, once.
pub fn install_acct_file(f: &'static dyn AcctFile) -> Result<(), AlreadyRegistered> {
    ACCT_FILE.set(f).map_err(|_| AlreadyRegistered("AcctFile"))
}

/// `acctp` and `savacctp`: the accounting file, and the one saved while its file system is
/// short of space.
struct AcctState {
    /// `acctp`.
    acctp: Option<Box<dyn AcctSink>>,
    /// `savacctp`.
    savacctp: Option<Box<dyn AcctSink>>,
}

/// `acct_lock` "acctlk", owning `acctp` and `savacctp`.
static ACCT: Rwlocked<AcctState> = Rwlocked::new(
    "acctlk",
    AcctState {
        acctp: None,
        savacctp: None,
    },
);

/// Whether `acctp` is set (`acct_process`'s unlocked peek, as the C's), written with it under
/// `acct_lock`'s writer.
static ACCT_ON: AtomicBool = AtomicBool::new(false);

/// `acct_proc != NULL`: the free-space watcher runs; its address is the watcher's channel.
pub static ACCT_RUNNING: AtomicBool = AtomicBool::new(false);

/// `acctsuspend`: stop accounting when < 2% free space left.
pub static ACCTSUSPEND: AtomicI32 = AtomicI32::new(2);
/// `acctresume`: resume when free space risen to > 4%.
pub static ACCTRESUME: AtomicI32 = AtomicI32::new(4);
/// `acctrate`: delay (in seconds) between space checks.
pub static ACCTRATE: AtomicI32 = AtomicI32::new(15);

impl AcctState {
    /// Records whether `acctp` is set for the unlocked peek.
    fn publish(&self) {
        ACCT_ON.store(self.acctp.is_some(), Ordering::Relaxed);
    }
}

/// The watcher's channel (`&acct_proc`).
fn acct_chan() -> usize {
    core::ptr::from_ref(&ACCT_RUNNING).addr()
}

/// `acct(2)`: accounting to the file at `path` (root only), or off for NULL. Written based on
/// the specification and previous implementation done by Mark Tinguely.
pub fn sys_acct<G: ProcGlue>(p: &Thread<G>, path: UserPtr<u8>) -> Result<(), Errno> {
    // Make sure that the caller is root.
    crate::proc::suser(p)?;

    // If accounting is to be started to a file, open that file for writing and make sure
    // it's 'normal'.
    let newvp = if path.is_null() {
        None
    } else {
        let Some(f) = ACCT_FILE.get() else {
            kpanic(format_args!("acct: no AcctFile bridge registered"));
        };
        Some(f.open(path)?)
    };

    let mut st = ACCT.write();

    // If accounting was previously enabled, kill the old space-watcher, close the file, and
    // (if no new file was specified, leave).
    if st.acctp.is_some() || st.savacctp.is_some() {
        crate::sched::wakeup_n::<G>(acct_chan(), -1);
        if let Some(old) = st.acctp.take().or_else(|| st.savacctp.take()) {
            old.close(true);
        }
        // The C closes `acctp ? acctp : savacctp` and forgets both.
        drop(st.savacctp.take());
    }
    let mut error = Ok(());
    if let Some(vp) = newvp {
        // Save the new accounting file vnode, and schedule the new free space watcher.
        st.acctp = Some(vp);
        error = acct_start::<G>();
        if error.is_err()
            && let Some(vp) = st.acctp.take()
        {
            vp.close(true);
        }
    }
    st.publish();
    drop(st);
    error
}

/// Write out process accounting information, on process exit (`exit1`): the record of
/// Leffler et al., enumerated below (and noted in `<sys/acct.h>`).
pub fn acct_process<G: ProcGlue>(p: &Thread<G>) -> Result<(), Errno> {
    let pr = p.process();

    // If accounting isn't enabled, don't bother
    if !ACCT_ON.load(Ordering::Relaxed) {
        return Ok(());
    }

    let st = ACCT.read();

    // Check the vnode again in case accounting got disabled while waiting for the lock.
    let Some(vp) = st.acctp.as_ref() else {
        return Ok(());
    };

    // (1) The name of the command that ran
    let mut ac_comm = [0u8; _MAXCOMLEN];
    let comm = pr.comm();
    let n = comm.len().min(ac_comm.len());
    ac_comm[..n].copy_from_slice(&comm[..n]);

    // (2) The amount of user and system time that was used
    let tu = Tusage::new();
    tuagg_get_process(&tu, pr);
    let (ut, st_, _) = calctsru(&tu, G::stathz());

    // (3) The elapsed time the command ran (and its starting time)
    let uptime = clock::nanouptime();
    let booted = clock::nanotime().sub(uptime);
    let start = pr.ps_start.get();
    let realstart = booted.add(start);
    let elapsed = uptime.sub(start);

    // (4) The average amount of memory used
    let r = &p.p_ru;
    let tmp = ut.add(st_);
    let hz = i64::from(crate::timeout::hz());
    let tick = i64::from(crate::timeout::tick()).max(1);
    let t = tmp.tv_sec * hz + tmp.tv_nsec / (1000 * tick);
    let ac_mem = if t != 0 {
        ((r.ru_ixrss.get() + r.ru_idrss.get() + r.ru_isrss.get()) / t) as u32
    } else {
        0
    };

    // (7) The terminal from which the process was started
    let ac_tty = acct_tty(pr);

    let cred = pr.ucred();
    let acct = Acct {
        ac_comm,
        ac_utime: encode_comp_t(ut.tv_sec as u64, ut.tv_nsec as u64),
        ac_stime: encode_comp_t(st_.tv_sec as u64, st_.tv_nsec as u64),
        ac_etime: encode_comp_t(elapsed.tv_sec as u64, elapsed.tv_nsec as u64),
        // (5) The number of disk I/O operations done
        ac_io: encode_comp_t((r.ru_inblock.get() + r.ru_oublock.get()) as u64, 0),
        ac_btime: realstart.tv_sec,
        // (6) The UID and GID of the process
        ac_uid: cred.cr_ruid.get(),
        ac_gid: cred.cr_rgid.get(),
        ac_mem,
        ac_tty,
        // Extensions
        ac_pid: pr.ps_pid,
        // (8) The flags that tell how process terminated or misbehaved.
        ac_flag: pr.ps_acflag.get(),
    };

    // Now, just write the accounting information to the file.
    let error = vp.write(hal::user::bytes_of(core::slice::from_ref(&acct)));
    drop(st);
    error
}

/// `ac_tty`: the controlling terminal of `pr`'s session when `pr` has one (`PS_CONTROLT`
/// and the session's `s_ttyp`, kern_acct.c:231-235), -1 otherwise (the user's decision 15
/// of N4.6; LZ always wrote -1).
fn acct_tty<G: ProcGlue>(pr: &Process<G>) -> Dev {
    if pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT == 0 {
        return -1;
    }
    let rd = klock::reader();
    pr.session(&rd).and_then(G::session_ttydev).unwrap_or(-1)
}

/// `encode_comp_t` converts from ticks in seconds and nanoseconds to ticks in 1/AHZ
/// seconds. The encoding is described in Leffler, et al., on page 63.
pub fn encode_comp_t(s: u64, ns: u64) -> Comp {
    let mut exp: u64 = 0;
    let mut rnd: u64 = 0;
    let mut s = s.wrapping_mul(u64::from(AHZ));
    s = s.wrapping_add(ns / (1_000_000_000 / u64::from(AHZ))); // Maximize precision.

    while s > MAXFRACT {
        rnd = s & (1 << (EXPSIZE - 1)); // Round up?
        s >>= EXPSIZE; // Base 8 exponent == 3 bit shift.
        exp += 1;
    }

    // If we need to round up, do it (and handle overflow correctly).
    if rnd != 0 {
        s += 1;
        if s > MAXFRACT {
            s >>= EXPSIZE;
            exp += 1;
        }
    }

    // Clean it up and polish it off.
    exp <<= MANTSIZE; // Shift the exponent into place
    exp += s; // and add on the mantissa.
    exp as Comp
}

/// `acct_start`: start the free-space watcher, unless it is already running.
pub fn acct_start<G: ProcGlue>() -> Result<(), Errno> {
    // Already running.
    if ACCT_RUNNING.load(Ordering::Relaxed) {
        return Ok(());
    }
    crate::thread::kthread_create::<G>(b"acct", crate::fork::Start::Acct)?;
    ACCT_RUNNING.store(true, Ordering::Relaxed);
    Ok(())
}

/// What one look at the free space decided ([`watch`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Watch {
    /// Accounting goes on (or stays suspended); look again later.
    Again,
    /// Accounting resumed (`savacctp` back to `acctp`).
    Resumed,
    /// Accounting suspended (`acctp` saved in `savacctp`).
    Suspended,
    /// The file is gone (revoked) or there is none: the watcher ends.
    Stop,
}

/// One pass of the watcher over the state: thresholds of [`ACCTSUSPEND`]/[`ACCTRESUME`]
/// percent of the file system's blocks.
fn watch(st: &mut AcctState) -> Watch {
    if let Some(sav) = st.savacctp.as_ref() {
        if sav.is_bad() {
            if let Some(sav) = st.savacctp.take() {
                sav.close(false);
            }
            return Watch::Stop;
        }
        let (bavail, blocks) = sav.free_space().unwrap_or((0, 0));
        if bavail > i64::from(ACCTRESUME.load(Ordering::Relaxed)) * blocks / 100 {
            st.acctp = st.savacctp.take();
            return Watch::Resumed;
        }
        Watch::Again
    } else if let Some(cur) = st.acctp.as_ref() {
        if cur.is_bad() {
            if let Some(cur) = st.acctp.take() {
                cur.close(false);
            }
            return Watch::Stop;
        }
        let (bavail, blocks) = cur.free_space().unwrap_or((0, 0));
        if bavail <= i64::from(ACCTSUSPEND.load(Ordering::Relaxed)) * blocks / 100 {
            st.savacctp = st.acctp.take();
            return Watch::Suspended;
        }
        Watch::Again
    } else {
        Watch::Stop
    }
}

/// `acct_thread`: periodically check the file system to see if accounting should be turned
/// on or off. Beware the case where the vnode has been vgone()'d out from underneath us,
/// e.g. when the file system containing the accounting file has been forcibly unmounted.
pub fn acct_thread<G: ProcGlue>() -> ! {
    let mut st = ACCT.write();
    loop {
        let w = watch(&mut st);
        st.publish();
        match w {
            Watch::Stop => break,
            Watch::Resumed => G::log(LOG_NOTICE, format_args!("Accounting resumed\n")),
            Watch::Suspended => G::log(LOG_NOTICE, format_args!("Accounting suspended\n")),
            Watch::Again => {}
        }
        let secs = u64::try_from(ACCTRATE.load(Ordering::Relaxed)).unwrap_or(0);
        let _ = st.sleep_nsec(
            core::ptr::from_ref(&ACCT_RUNNING),
            PPAUSE,
            "acct",
            secs.saturating_mul(1_000_000_000),
        );
    }
    ACCT_RUNNING.store(false, Ordering::Relaxed);
    drop(st);
    crate::thread::kthread_exit::<G>(0)
}

/// `acct_shutdown`: close the accounting file (`vfs_shutdown`).
pub fn acct_shutdown() {
    let mut st = ACCT.write();
    if let Some(vp) = st.acctp.take().or_else(|| st.savacctp.take()) {
        vp.close(false);
    }
    drop(st.savacctp.take());
    st.publish();
}

/// The host tests' reset: no accounting file, no watcher.
#[cfg(any(test, feature = "doubles"))]
pub fn acct_test_reset() {
    let mut st = ACCT.write();
    st.acctp = None;
    st.savacctp = None;
    st.publish();
    ACCT_RUNNING.store(false, Ordering::Relaxed);
}

const _: () = {
    // The record is written as its bytes: no padding (abi_pod! checks it), 64 of them.
    assert!(size_of::<Acct>() == 64);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicI64;

    use super::*;

    #[test]
    fn comp_t_encoding() {
        // below the mantissa limit: plain 1/64 s ticks
        assert_eq!(encode_comp_t(1, 0), 64);
        assert_eq!(encode_comp_t(0, 500_000_000), 32);
        // 200 s = 12800 ticks > 8191: one base-8 exponent step, rounded
        let c = encode_comp_t(200, 0);
        assert_eq!(c >> MANTSIZE, 1);
        assert_eq!(u64::from(c & MAXFRACT as u16), 12800 >> 3);
    }

    #[test]
    fn the_record_is_64_bytes_with_the_tty() {
        assert_eq!(size_of::<Acct>(), 64);
        assert_eq!(core::mem::offset_of!(Acct, ac_btime), 32);
        assert_eq!(core::mem::offset_of!(Acct, ac_tty), 52);
        assert_eq!(core::mem::offset_of!(Acct, ac_flag), 60);
        let a = Acct {
            ac_tty: 0x0c00,
            ..Acct::default()
        };
        let b = hal::user::bytes_of(core::slice::from_ref(&a));
        assert_eq!(&b[52..56], &0x0c00i32.to_ne_bytes());
    }

    /// A sink double: its free space and whether it was revoked are the test's to set.
    struct Sink {
        bavail: Arc<AtomicI64>,
        bad: Arc<AtomicBool>,
        closed: Arc<AtomicBool>,
    }

    impl AcctSink for Sink {
        fn write(&self, _rec: &[u8]) -> Result<(), Errno> {
            Ok(())
        }
        fn free_space(&self) -> Option<(i64, i64)> {
            Some((self.bavail.load(Ordering::Relaxed), 100))
        }
        fn is_bad(&self) -> bool {
            self.bad.load(Ordering::Relaxed)
        }
        fn close(self: Box<Self>, _by_curproc: bool) {
            self.closed.store(true, Ordering::Relaxed);
        }
    }

    #[test]
    fn accounting_suspends_and_resumes_at_the_thresholds() {
        let bavail = Arc::new(AtomicI64::new(50));
        let bad = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(AtomicBool::new(false));
        let mut st = AcctState {
            acctp: Some(Box::new(Sink {
                bavail: Arc::clone(&bavail),
                bad: Arc::clone(&bad),
                closed: Arc::clone(&closed),
            })),
            savacctp: None,
        };
        assert_eq!(watch(&mut st), Watch::Again);
        // at most 2% free: suspended
        bavail.store(2, Ordering::Relaxed);
        assert_eq!(watch(&mut st), Watch::Suspended);
        assert!(st.acctp.is_none() && st.savacctp.is_some());
        // 4% is not above 4%: still suspended
        bavail.store(4, Ordering::Relaxed);
        assert_eq!(watch(&mut st), Watch::Again);
        bavail.store(5, Ordering::Relaxed);
        assert_eq!(watch(&mut st), Watch::Resumed);
        assert!(st.acctp.is_some() && st.savacctp.is_none());
        // a revoked vnode stops accounting and is closed
        bad.store(true, Ordering::Relaxed);
        assert_eq!(watch(&mut st), Watch::Stop);
        assert!(st.acctp.is_none() && st.savacctp.is_none());
        assert!(closed.load(Ordering::Relaxed));
        assert_eq!(watch(&mut st), Watch::Stop);
    }

    crate::test_file_glue!(Files, Procs);
    crate::test_proc_glue!(Procs, Files, {
        fn session_ttydev(_sess: &crate::proc::Session<Self>) -> Option<Dev> {
            Some(0x0c01)
        }
    });
    /// A second glue (its own module: the glues' link types are module items), whose
    /// sessions have no terminal (`session_ttydev`'s default).
    mod notty {
        crate::test_file_glue!(NoTtyFiles, NoTty);
        crate::test_proc_glue!(NoTty, NoTtyFiles);
    }
    use notty::NoTty;

    /// Both glues' tables, once per test thread (they are the thread's).
    fn setup() {
        std::thread_local! {
            static MADE: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        crate::testutil::install();
        if !MADE.with(|m| m.replace(true)) {
            // As exit1 runs acct_process: the kernel lock held (the double counts it).
            hal::klock::kernel_lock();
            crate::proc::procinit::<Procs>(64, 64);
            crate::proc::procinit::<NoTty>(64, 64);
        }
    }

    /// A process `pid` of glue `G`, in a session of its own when `session`, with
    /// `PS_CONTROLT` when `controlt`; leaked by the caller (a member of its group, which the
    /// tests do not take apart).
    fn process_of<G: ProcGlue<ProcessG = (), PgrpG = (), SessionG = ()>>(
        pid: Pid,
        session: bool,
        controlt: bool,
    ) -> crate::proc::ProcessRef<G> {
        setup();
        let pr = crate::proc::process_alloc::<G>(pid, ());
        if session {
            crate::proc::enternewpgrp(
                &pr,
                crate::proc::pgrp_alloc::<G>(()),
                Some(crate::proc::session_alloc::<G>(())),
            );
        }
        if controlt {
            pr.ps_flags.fetch_or(PS_CONTROLT, Ordering::Relaxed);
        }
        pr
    }

    /// The user's decision 15 of N4.6 (kern_acct.c:231-235): `ac_tty` is the session's
    /// terminal when the process has a controlling one, -1 otherwise.
    #[test]
    fn ac_tty_is_the_session_s_terminal_under_controlt() {
        // (session, PS_CONTROLT, ac_tty with a terminal, ac_tty without one)
        let rows = [
            (false, false, -1, -1),
            (false, true, -1, -1),
            (true, false, -1, -1),
            (true, true, 0x0c01, -1),
        ];
        for (i, &(session, controlt, tty, notty)) in rows.iter().enumerate() {
            let pid = 900 + i as Pid;
            let pr = process_of::<Procs>(pid, session, controlt);
            assert_eq!(acct_tty(&pr), tty, "row {i}");
            core::mem::forget(pr);
            let pr = process_of::<NoTty>(pid, session, controlt);
            assert_eq!(acct_tty(&pr), notty, "row {i}");
            core::mem::forget(pr);
        }
    }
}
/* </TESTS> */
