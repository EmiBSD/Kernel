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
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)tty_pty.c	8.4 (Berkeley) 2/20/95
 */
/* </LICENSES> */

/* <CODE> */
//! The pseudo-teletype driver (actually two drivers, requiring two entries in `cdevsw`):
//! `kern/tty_pty.c`, the kernel half (the `/dev/ptm` cloning ioctl's file and vnode work is
//! `bsd`'s, `sys/kern/tty_pty.rs`, N5).
//!
//! Upstream: sys/kern/tty_pty.c @ 3ce1f3f79392
//! LZ: sys/kern/tty_pty.rs@3c62ede68802
//!
//! `pts` is `/dev/tty[p-zP-T][0-9a-zA-Z]`, the slave side a program uses as its terminal;
//! `ptc` is `/dev/pty[p-zP-T][0-9a-zA-Z]`, the master side (`sshd`, `xterm`, `script`) that
//! sees the slave's output and feeds its input.
//!
//! ## Deviations
//! - `ptsclose` ORs the errnos of the line discipline's close and `ttyclose` in C; here the
//!   first error wins (LZ's).
//! - `ptsread`'s remote mode tests `ureadc(..) < 0`, which no errno is, so the C ignores a
//!   copy error there; so does this port (LZ's).
//! - The entry points answer `ENXIO` for a minor without a softc, where the C would
//!   dereference NULL (LZ's).
//! - `NPTY` (`pty.h`, which `config(8)` writes from `pseudo-device pty 16`) is a constant
//!   (LZ's).
//!
//! ## Redesign
//! - LZ's `pt_softc[]` was a raw `malloc(M_DEVBUF)` array of `Option<&'static PtSoftc>`
//!   grown by `copy_nonoverlapping` under `pt_softc_lock`, read by an `unsafe` index, its
//!   softcs written into raw blocks and shared under an `unsafe impl Sync` (8 `unsafe`). Here
//!   the slots are a table of `OnceStatic`s in the glue's tables ([`PtyTables`], `NPTY_MAX` of
//!   them, set once each under `pt_softc_lock` and read without it, as the C reads a slot it
//!   made): `npty` still grows by powers of two (it is what `pty_getfree` scans and where a new
//!   pty's minor comes from), the array's reallocation is gone. A softc is an `MBox` of
//!   `M_DEVBUF` leaked for the kernel's life (the C never frees one), its tty a counted
//!   handle, its flags atomic words.
//! - `t_oproc = ptsstart` is the [`PtsDriver`] the master's open sets ([`TtyDriver`],
//!   decision 46); `t_oproc != NULL`, "the controller is still around", is
//!   `Tty::has_driver`.
//! - `ptcread_filtops`, `ptcwrite_filtops` and `ptcexcept_filtops` are [`PtcRead`],
//!   [`PtcWrite`] and [`PtcExcept`], `impl kernel::event::Filter`s; a master knote's hook is
//!   the minor, a word compared and never followed (the softc found again in the table).
//! - `ptyioctl` is told which side it was called for (`master`): `bsd`'s device switch, which
//!   compares `cdevsw[major(dev)].d_open` with `ptcopen`, answers it (N7).
//! - `ptmattach`'s scan of `cdevsw` for `ptsopen` stays `bsd`'s; it hands the major to
//!   [`set_pts_major`].

use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicI32, Ordering};

use hal::malloc::{M_WAITOK, MBox};
use hal::once::OnceStatic;
use hal::rwlock::RawRwlock;
use hal::sleep::PCATCH;
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FNONBLOCK, FREAD, FWRITE};
use hal::sys::filio::FIONREAD;
use hal::sys::signal::{NSIG, SIGINFO, SIGTTIN};
use hal::sys::types::{Dev, major, makedev, minor};
use hal::user::{pod_from_bytes, pod_to_bytes};

use crate::assert::kpanic;
use crate::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_EOF, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote, NOTE_OOB,
};
use crate::proc::{I32Word, PS_PPWAIT, ProcGlue, Thread, U8Word, suser};
use crate::select::{Selinfo, selwakeup};
use crate::sig::{pgsignal, sigismasked};
use crate::sigio::TIOCGPGRP;
use crate::sysctl::SysctlReq;
use crate::termios::{
    B115200, EXTPROC, ICANON, IXON, NOFLSH, NOKERNINFO, TIOCCBRK, TIOCEXT, TIOCPKT, TIOCPKT_DOSTOP,
    TIOCPKT_IOCTL, TIOCPKT_NOSTOP, TIOCPKT_START, TIOCPKT_STOP, TIOCREMOTE, TIOCSBRK, TIOCSETA,
    TIOCSETAF, TIOCSETAW, TIOCSETD, TIOCSIG, TIOCUCNTL, TIOCUCNTL_CBRK, TIOCUCNTL_SBRK,
    TTYDEF_CFLAG, TTYDEF_IFLAG, TTYDEF_LFLAG, TTYDEF_OFLAG, VSTART, VSTOP, cceq, ctrl, uioccmd,
};
use crate::tty::{
    IO_NDELAY, TS_CARR_ON, TS_ISOPEN, TS_TTSTOP, TS_WOPEN, TS_XCLUDE, TTIPRI, TTOPEN, TTOPRI,
    TTYBG, TTYIN, TTYOUT, Tty, TtyDriver, TtyRef, isbackground, linesw, ttioctl, ttsetwater,
    ttwakeup, ttwakeupwr, ttychars, ttyclose, ttyflush, ttyinfo, ttymalloc, ttysleep,
    ttysleep_nsec,
};
use crate::uio::UioMove;

/// Chunk size iomoved to/from user.
const BUFSIZ: usize = 100;

// XXX this needs to come from somewhere sane, and work with MAKEDEV

/// The letters of the pty names.
const TTY_LETTERS: &[u8] = b"pqrstuvwxyzPQRST";
/// The suffixes of the pty names.
const TTY_SUFFIX: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// `NPTY`: `pseudo-device pty 16` (MI `conf/GENERIC`).
pub const NPTY: i32 = 16;

/// Number of initial ptys.
pub const NPTY_MIN: i32 = 8;
/// Maximum number of ptys supported.
pub const NPTY_MAX: i32 = 992;

/// Packet mode.
pub const PF_PKT: i32 = 0x08;
/// User told stopped.
pub const PF_STOPPED: i32 = 0x10;
/// Remote and flow controlled input.
pub const PF_REMOTE: i32 = 0x20;
/// No ^S/^Q flow control.
pub const PF_NOSTOP: i32 = 0x40;
/// User control mode.
pub const PF_UCNTL: i32 = 0x80;

/// `M_DEVBUF`: device driver memory (`vmstat -m`'s `devbuf`).
const M_DEVBUF: i32 = 2;

/// A second in nanoseconds.
const SEC_NSEC: u64 = 1_000_000_000;

/// `struct pt_softc`: one pseudo-terminal.
pub struct PtSoftc<G: ProcGlue> {
    /// `pt_tty`.
    pub pt_tty: TtyRef<G>,
    /// `pt_flags` (`PF_*`).
    pub pt_flags: I32Word,
    /// `pt_selr`.
    pub pt_selr: Selinfo<G>,
    /// `pt_selw`.
    pub pt_selw: Selinfo<G>,
    /// `pt_send`: the `TIOCPKT_*` byte the master reads next in packet mode.
    pub pt_send: U8Word,
    /// `pt_ucntl`: the user control byte the master reads next.
    pub pt_ucntl: U8Word,
    /// `pty_pn`: the master's path, NUL-terminated.
    pub pty_pn: [u8; 11],
    /// `pty_sn`: the slave's path, NUL-terminated.
    pub pty_sn: [u8; 11],
}

/// The ptys of one glue: `pt_softc[]`, `npty`, `pt_softc_lock`, `pts_major`.
pub struct PtyTables<G: ProcGlue> {
    /// `pt_softc_lock`: for pty array (the growth and the slots' filling).
    lock: RawRwlock,
    /// `npty`: size of pty array (grown by powers of two up to `maxptys`).
    npty: AtomicI32,
    /// `pt_softc[]`: each slot set once, for the kernel's life.
    slots: [OnceStatic<&'static PtSoftc<G>>; NPTY_MAX as usize],
    /// `pts_major`.
    pts_major: AtomicI32,
}

impl<G: ProcGlue> PtyTables<G> {
    /// No pty yet (`ptyattach` sets `npty`).
    pub const fn new() -> Self {
        Self {
            lock: RawRwlock::new("ptarrlk"),
            npty: AtomicI32::new(0),
            slots: [const { OnceStatic::new() }; NPTY_MAX as usize],
            pts_major: AtomicI32::new(0),
        }
    }
}

impl<G: ProcGlue> Default for PtyTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `t_oproc = ptsstart`: the pty's driver of its slave's tty.
pub struct PtsDriver<G>(PhantomData<fn() -> G>);

/// `ptcread_filtops`: `EVFILT_READ` on the master.
pub struct PtcRead;
/// `ptcwrite_filtops`: `EVFILT_WRITE` on the master.
pub struct PtcWrite;
/// `ptcexcept_filtops`: `EVFILT_EXCEPT` on the master.
pub struct PtcExcept;

/// The erased tables of the master's filters (`kn_fop`'s targets).
struct PtcFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> PtcFiltops<G> {
    /// `&ptcread_filtops`.
    const READ: Filterops<G> = Filterops::of::<PtcRead>();
    /// `&ptcwrite_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<PtcWrite>();
    /// `&ptcexcept_filtops`.
    const EXCEPT: Filterops<G> = Filterops::of::<PtcExcept>();
}

/// `maxptys`: maximum number of ptys.
const MAXPTYS: i32 = NPTY_MAX;

/// The glue's pty tables.
fn tables<G: ProcGlue>() -> &'static PtyTables<G> {
    &G::tables().pty
}

/// `pt_softc[minor]`, `None` for a slot not made yet.
pub fn pt_softc<G: ProcGlue>(minor: u32) -> Option<&'static PtSoftc<G>> {
    let t = tables::<G>();
    if minor as i32 >= t.npty.load(Ordering::Acquire) {
        return None;
    }
    t.slots.get(minor as usize)?.get().copied()
}

/// The softc and tty of `dev`, for the entry points (`ENXIO` for a slot never made).
fn pty_of<G: ProcGlue>(dev: Dev) -> Result<(&'static PtSoftc<G>, &'static Tty<G>), Errno> {
    let pti = pt_softc::<G>(minor(dev)).ok_or(Errno::ENXIO)?;
    Ok((pti, &pti.pt_tty))
}

/// `ptydevname`: the names of pty `minor` (master, slave), empty past the letters.
pub fn ptydevname(minor: i32) -> ([u8; 11], [u8; 11]) {
    let mut buf = *b"/dev/XtyXX\0";

    let i = minor as usize / TTY_SUFFIX.len();
    let j = minor as usize % TTY_SUFFIX.len();
    if i >= TTY_LETTERS.len() {
        return ([0; 11], [0; 11]);
    }
    buf[5] = b'p';
    buf[8] = TTY_LETTERS[i];
    buf[9] = TTY_SUFFIX[j];
    let pn = buf;
    buf[5] = b't';
    (pn, buf)
}

/// `check_pty`: check if the minor is correct and ensure necessary structures are properly
/// allocated.
pub fn check_pty<G: ProcGlue>(dev: Dev) -> Result<(), Errno> {
    let t = tables::<G>();
    let minor = minor(dev) as i32;

    let w = t.lock.write();
    let npty = t.npty.load(Ordering::Relaxed);
    if minor >= npty {
        // check if the requested pty can be granted
        if minor >= MAXPTYS {
            drop(w);
            crate::prf::printf(format_args!("pty: table is full\n"));
            return Err(Errno::ENXIO);
        }

        // grow pty array by powers of two, up to maxptys
        let mut newnpty = npty.max(1);
        while newnpty <= minor {
            newnpty *= 2;
        }

        if newnpty > MAXPTYS {
            newnpty = MAXPTYS;
        }
        t.npty.store(newnpty, Ordering::Release);
    }

    // If the entry is not yet allocated, allocate one.
    let slot = &t.slots[minor as usize];
    if slot.get().is_none() {
        let tp = ttymalloc::<G>(1_000_000);
        tp.t_dev.set(dev);
        let (pty_pn, pty_sn) = ptydevname(minor);
        let Some(pti) = MBox::new(
            M_DEVBUF,
            M_WAITOK,
            PtSoftc {
                pt_tty: tp,
                pt_flags: I32Word::new(0),
                pt_selr: Selinfo::new(),
                pt_selw: Selinfo::new(),
                pt_send: U8Word::new(0),
                pt_ucntl: U8Word::new(0),
                pty_pn,
                pty_sn,
            },
        ) else {
            kpanic(format_args!("check_pty: out of memory"));
        };
        let _ = slot.set(MBox::leak(pti));
    }
    drop(w);
    Ok(())
}

/// `ptyattach`: establish `n` (or default if `n` is 1) ptys in the system (`ptmattach`, the
/// device switch's part, is `bsd`'s).
pub fn ptyattach<G: ProcGlue>(n: i32) {
    // maybe should allow 0 => none?
    let n = if n <= 1 { NPTY_MIN } else { n };
    tables::<G>().npty.store(n.min(MAXPTYS), Ordering::Release);
}

/// `pts_major = maj` (`ptmattach`'s scan of the device switch).
pub fn set_pts_major<G: ProcGlue>(maj: i32) {
    tables::<G>().pts_major.store(maj, Ordering::Relaxed);
}

/// `ptsopen`: open the slave side.
pub fn ptsopen<G: ProcGlue>(
    dev: Dev,
    flag: i32,
    _devtype: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    check_pty::<G>(dev)?;

    let (_pti, tp) = pty_of::<G>(dev)?;
    if !tp.t_state_isset(TS_ISOPEN) {
        tp.t_state_set(TS_WOPEN);
        ttychars(tp); // Set up default chars
        tp.set_t_iflag(TTYDEF_IFLAG);
        tp.set_t_oflag(TTYDEF_OFLAG);
        tp.set_t_lflag(TTYDEF_LFLAG);
        tp.set_t_cflag(TTYDEF_CFLAG);
        tp.set_t_ispeed(B115200 as i32);
        tp.set_t_ospeed(B115200 as i32);
        ttsetwater(tp); // would be done in xxparam()
    } else if tp.t_state_isset(TS_XCLUDE) && suser(p).is_err() {
        return Err(Errno::EBUSY);
    }
    if tp.has_driver() {
        // Ctrlr still around.
        tp.t_state_set(TS_CARR_ON);
    }
    while !tp.t_state_isset(TS_CARR_ON) {
        tp.t_state_set(TS_WOPEN);
        if flag & FNONBLOCK != 0 {
            break;
        }
        ttysleep(
            tp,
            ptr::from_ref(&tp.t_rawq).cast(),
            TTIPRI | PCATCH,
            TTOPEN,
        )?;
    }
    let error = linesw(tp).open(dev, tp, p);
    ptcwakeup(tp, FREAD | FWRITE);
    error
}

/// `ptsclose`: close the slave side.
pub fn ptsclose<G: ProcGlue>(
    dev: Dev,
    flag: i32,
    _mode: i32,
    p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let (_pti, tp) = pty_of::<G>(dev)?;

    let error = linesw(tp).close(tp, flag, p);
    let error = error.and(ttyclose(tp));
    ptcwakeup(tp, FREAD | FWRITE);
    error
}

/// `ptsread`: read from the slave side.
pub fn ptsread<G: ProcGlue>(dev: Dev, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let (pti, tp) = pty_of::<G>(dev)?;
    let mut error = Ok(());

    loop {
        // again:
        if pti.pt_flags.get() & PF_REMOTE != 0 {
            loop {
                let bg = G::with_curproc(|p| {
                    let p = p?;
                    let pr = p.process();
                    if !isbackground(pr, tp) {
                        return None;
                    }
                    // A process in the background of a tty is in a process group.
                    let pg = {
                        let r = hal::klock::reader();
                        pr.pgrp(&r).cloned()
                    };
                    if sigismasked(p, SIGTTIN)
                        || pg.as_ref().is_none_or(|pg| pg.pg_jobc.get() == 0)
                        || pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT != 0
                    {
                        return Some(Err(Errno::EIO));
                    }
                    pgsignal(pg.as_deref(), SIGTTIN, true);
                    Some(Ok(()))
                });
                match bg {
                    None => break,
                    Some(Err(e)) => return Err(e),
                    Some(Ok(())) => ttysleep_nsec(
                        tp,
                        ptr::from_ref(&crate::sched::NOWAKE).cast(),
                        TTIPRI | PCATCH,
                        TTYBG,
                        SEC_NSEC,
                    )?,
                }
            }
            if tp.t_canq.c_cc.get() == 0 {
                if flag & IO_NDELAY != 0 {
                    return Err(Errno::EWOULDBLOCK);
                }
                ttysleep(tp, ptr::from_ref(&tp.t_canq).cast(), TTIPRI | PCATCH, TTYIN)?;
                continue;
            }
            while tp.t_canq.c_cc.get() > 1 && uio.resid() > 0 {
                // The C's `ureadc(..) < 0` is never true (see the module's deviations).
                let c = tp.t_canq.getc();
                let _ = uio.uiomove(&mut [c as u8]);
            }
            if tp.t_canq.c_cc.get() == 1 {
                let _ = tp.t_canq.getc();
            }
            if tp.t_canq.c_cc.get() != 0 {
                return error;
            }
        } else if tp.has_driver() {
            error = linesw(tp).read(tp, uio, flag);
        }
        break;
    }
    ptcwakeup(tp, FWRITE);
    error
}

/// `ptswrite`: write to pseudo-tty. Wakeups of controlling tty will happen indirectly, when
/// tty driver calls `ptsstart`.
pub fn ptswrite<G: ProcGlue>(dev: Dev, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let (_pti, tp) = pty_of::<G>(dev)?;

    if !tp.has_driver() {
        return Err(Errno::EIO);
    }
    linesw(tp).write(tp, uio, flag)
}

/// `ptsstart`: start output on pseudo-tty. Wake up process polling or sleeping for input
/// from controlling tty.
pub fn ptsstart<G: ProcGlue>(tp: &Tty<G>) {
    let Some(pti) = pt_softc::<G>(minor(tp.t_dev.get())) else {
        return;
    };

    if tp.t_state_isset(TS_TTSTOP) {
        return;
    }
    if pti.pt_flags.get() & PF_STOPPED != 0 {
        pti.pt_flags.set(pti.pt_flags.get() & !PF_STOPPED);
        pti.pt_send.set(TIOCPKT_START as u8);
    }
    ptcwakeup(tp, FREAD);
}

impl<G: ProcGlue> TtyDriver<G> for PtsDriver<G> {
    /// `ptsstart`.
    fn start(&self, tp: &Tty<G>) {
        ptsstart(tp);
    }
}

/// `ptsstop`: stop output on the slave side (the `d_stop` of `pts`).
pub fn ptsstop<G: ProcGlue>(tp: &Tty<G>, flush: i32) -> Result<(), Errno> {
    let Some(pti) = pt_softc::<G>(minor(tp.t_dev.get())) else {
        return Ok(());
    };
    let mut flush = flush;

    // note: FLUSHREAD and FLUSHWRITE already ok
    if flush == 0 {
        flush = TIOCPKT_STOP;
        pti.pt_flags.set(pti.pt_flags.get() | PF_STOPPED);
    } else {
        pti.pt_flags.set(pti.pt_flags.get() & !PF_STOPPED);
    }
    pti.pt_send.set(pti.pt_send.get() | flush as u8);
    // change of perspective
    let mut flag = 0;
    if flush & FREAD != 0 {
        flag |= FWRITE;
    }
    if flush & FWRITE != 0 {
        flag |= FREAD;
    }
    ptcwakeup(tp, flag);
    Ok(())
}

/// `ptcwakeup`: wake up the master's readers (`FREAD`) or writers (`FWRITE`).
pub fn ptcwakeup<G: ProcGlue>(tp: &Tty<G>, flag: i32) {
    let Some(pti) = pt_softc::<G>(minor(tp.t_dev.get())) else {
        return;
    };

    if flag & FREAD != 0 {
        selwakeup(&pti.pt_selr);
        hal::sleep::wakeup(tp.t_outq.cf_chan());
    }
    if flag & FWRITE != 0 {
        selwakeup(&pti.pt_selw);
        hal::sleep::wakeup(tp.t_rawq.cf_chan());
    }
}

/// `ptcopen`: open the master side.
pub fn ptcopen<G: ProcGlue>(
    dev: Dev,
    _flag: i32,
    _devtype: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    check_pty::<G>(dev)?;

    let (pti, tp) = pty_of::<G>(dev)?;
    if tp.has_driver() {
        return Err(Errno::EIO);
    }
    tp.set_driver(&PtsDriver::<G>(PhantomData));
    let _ = linesw(tp).modem(tp, true);
    tp.t_lflag_clr(EXTPROC);
    pti.pt_flags.set(0);
    pti.pt_send.set(0);
    pti.pt_ucntl.set(0);
    Ok(())
}

/// `ptcclose`: close the master side.
pub fn ptcclose<G: ProcGlue>(
    dev: Dev,
    _flag: i32,
    _devtype: i32,
    _p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let (_pti, tp) = pty_of::<G>(dev)?;

    let _ = linesw(tp).modem(tp, false);
    tp.t_state_clr(TS_CARR_ON);
    tp.clear_driver(); // mark closed
    Ok(())
}

/// `ptcread`: read the slave's output from the master side.
pub fn ptcread<G: ProcGlue>(dev: Dev, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let (pti, tp) = pty_of::<G>(dev)?;
    let mut buf = [0u8; BUFSIZ];
    let mut bufcc = 0;

    // We want to block until the slave is open, and there's something to read; but if we
    // lost the slave or we're NBIO, then return the appropriate error instead.
    loop {
        if tp.t_state_isset(TS_ISOPEN) {
            if pti.pt_flags.get() & PF_PKT != 0 && pti.pt_send.get() != 0 {
                uio.uiomove(&mut [pti.pt_send.get()])?;
                if i32::from(pti.pt_send.get()) & TIOCPKT_IOCTL != 0 {
                    let cc = uio.resid().min(size_of::<crate::termios::Termios>());
                    let mut t = [0u8; size_of::<crate::termios::Termios>()];
                    let _ = pod_to_bytes(&tp.termios(), &mut t);
                    uio.uiomove(&mut t[..cc])?;
                }
                pti.pt_send.set(0);
                return Ok(());
            }
            if pti.pt_flags.get() & PF_UCNTL != 0 && pti.pt_ucntl.get() != 0 {
                uio.uiomove(&mut [pti.pt_ucntl.get()])?;
                pti.pt_ucntl.set(0);
                return Ok(());
            }
            if tp.t_outq.c_cc.get() != 0 && !tp.t_state_isset(TS_TTSTOP) {
                break;
            }
        }
        if !tp.t_state_isset(TS_CARR_ON) {
            return Ok(()); // EOF
        }
        if flag & IO_NDELAY != 0 {
            return Err(Errno::EWOULDBLOCK);
        }
        hal::sleep::tsleep_nsec(
            tp.t_outq.cf_chan(),
            TTIPRI | PCATCH,
            TTYIN,
            hal::sleep::INFSLP,
        )?;
    }
    let mut error = Ok(());
    if pti.pt_flags.get() & (PF_PKT | PF_UCNTL) != 0 {
        error = uio.uiomove(&mut [0]);
    }
    while uio.resid() > 0 && error.is_ok() {
        let cc = uio.resid().min(BUFSIZ);
        let cc = tp.t_outq.q_to_b(&mut buf[..cc]);
        if cc > bufcc {
            bufcc = cc;
        }
        if cc == 0 {
            break;
        }
        error = uio.uiomove(&mut buf[..cc]);
    }
    ttwakeupwr(tp);
    if bufcc != 0 {
        libkern::explicit_bzero(&mut buf[..bufcc]);
    }
    error
}

/// How `ptcwrite` ends.
enum PtcExit {
    /// `goto block`.
    Block,
    /// `goto interrupt`.
    Interrupt,
    /// `goto done` with this result.
    Done(Result<(), Errno>),
}

/// `ptcwrite`: feed the slave's input from the master side.
pub fn ptcwrite<G: ProcGlue>(dev: Dev, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let (pti, tp) = pty_of::<G>(dev)?;
    let mut buf = [0u8; BUFSIZ];
    let mut cp = 0usize;
    let mut cc = 0usize;
    let mut bufcc = 0usize;
    let mut cnt = 0usize;

    let error = 'done: loop {
        // again:
        let exit = 'body: {
            if !tp.t_state_isset(TS_ISOPEN) {
                break 'body PtcExit::Block;
            }
            if pti.pt_flags.get() & PF_REMOTE != 0 {
                if tp.t_canq.c_cc.get() != 0 {
                    break 'body PtcExit::Block;
                }
                while uio.resid() > 0 && tp.t_canq.c_cc.get() < tp.ttyhog() - 1 {
                    if cc == 0 {
                        cc = uio.resid().min(BUFSIZ);
                        cc = cc.min((tp.ttyhog() - 1 - tp.t_canq.c_cc.get()) as usize);
                        if cc > bufcc {
                            bufcc = cc;
                        }
                        cp = 0;
                        if let Err(e) = uio.uiomove(&mut buf[..cc]) {
                            break 'body PtcExit::Done(Err(e));
                        }
                        // check again for safety
                        if !tp.t_state_isset(TS_ISOPEN) {
                            break 'body PtcExit::Done(Err(Errno::EIO));
                        }
                    }
                    if cc != 0 {
                        let _ = tp.t_canq.b_to_q(&buf[cp..cp + cc]);
                    }
                    cc = 0;
                }
                let _ = tp.t_canq.putc(0);
                ttwakeup(tp);
                hal::sleep::wakeup(ptr::from_ref(&tp.t_canq));
                break 'body PtcExit::Done(Ok(()));
            }
            loop {
                if cc == 0 {
                    cc = uio.resid().min(BUFSIZ);
                    if cc > bufcc {
                        bufcc = cc;
                    }
                    cp = 0;
                    if let Err(e) = uio.uiomove(&mut buf[..cc]) {
                        break 'body PtcExit::Done(Err(e));
                    }
                    // check again for safety
                    if !tp.t_state_isset(TS_ISOPEN) {
                        break 'body PtcExit::Done(Err(Errno::EIO));
                    }
                }
                bufcc = cc;
                while cc > 0 {
                    if tp.t_rawq.c_cc.get() + tp.t_canq.c_cc.get() >= tp.ttyhog() - 2
                        && (tp.t_canq.c_cc.get() > 0 || tp.t_lflag() & ICANON == 0)
                    {
                        hal::sleep::wakeup(ptr::from_ref(&tp.t_rawq));
                        break 'body PtcExit::Block;
                    }
                    let c = i32::from(buf[cp]);
                    cp += 1;
                    if linesw(tp).rint(c, tp) == 1
                        && hal::sleep::tsleep_nsec(
                            ptr::from_ref(tp),
                            TTIPRI | PCATCH,
                            "ttyretype",
                            // tsleep(.., 1): one tick
                            crate::timeout::tick() as u64 * 1000,
                        ) == Err(Errno::EINTR)
                    {
                        break 'body PtcExit::Interrupt;
                    }
                    cnt += 1;
                    cc -= 1;
                }
                cc = 0;
                if uio.resid() == 0 {
                    break;
                }
            }
            PtcExit::Done(Ok(()))
        };

        match exit {
            PtcExit::Done(e) => break 'done e,
            PtcExit::Interrupt => {
                // adjust for data copied in but not written
                uio.set_resid(uio.resid() + cc);
                break 'done Ok(());
            }
            PtcExit::Block => {
                // Come here to wait for slave to open, for space in outq, or space in rawq.
                if !tp.t_state_isset(TS_CARR_ON) {
                    break 'done Err(Errno::EIO);
                }
                if flag & IO_NDELAY != 0 {
                    // adjust for data copied in but not written
                    uio.set_resid(uio.resid() + cc);
                    break 'done if cnt == 0 {
                        Err(Errno::EWOULDBLOCK)
                    } else {
                        Ok(())
                    };
                }
                if let Err(e) = hal::sleep::tsleep_nsec(
                    tp.t_rawq.cf_chan(),
                    TTOPRI | PCATCH,
                    TTYOUT,
                    hal::sleep::INFSLP,
                ) {
                    // interrupt: adjust for data copied in but not written
                    uio.set_resid(uio.resid() + cc);
                    break 'done Err(e);
                }
            }
        }
    };
    // done:
    if bufcc != 0 {
        libkern::explicit_bzero(&mut buf[..bufcc]);
    }
    error
}

/// `kn->kn_hook` of a master-side knote: its pty (the minor the hook holds).
fn kn_pti<G: ProcGlue>(kn: &Knote<G>) -> &'static PtSoftc<G> {
    match pt_softc::<G>(kn.kn_hook.get().addr() as u32) {
        Some(pti) => pti,
        None => kpanic(format_args!("knote {:p}: no pty", kn)),
    }
}

/// `filt_ptcrdetach`: unhooks a read or except knote.
fn filt_ptcrdetach<G: ProcGlue>(kn: &Knote<G>) {
    let pti = kn_pti(kn);
    let _s = hal::intr::splraise(hal::intr::IPL_TTY);
    pti.pt_selr.si_note.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for PtcRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ptcrdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ptcrdetach(kn);
    }

    /// `filt_ptcread`: the master is readable when the slave has output (or a packet-mode or
    /// user-control byte) for it; EOF (and, for poll, a hang-up) once the carrier is gone.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let pti = kn_pti(kn);
        let tp = &pti.pt_tty;
        kn.kn_data().set(0);

        if tp.t_state_isset(TS_ISOPEN) {
            if !tp.t_state_isset(TS_TTSTOP) {
                kn.kn_data().set(i64::from(tp.t_outq.c_cc.get()));
            }
            if (pti.pt_flags.get() & PF_PKT != 0 && pti.pt_send.get() != 0)
                || (pti.pt_flags.get() & PF_UCNTL != 0 && pti.pt_ucntl.get() != 0)
            {
                kn.kn_data().set(kn.kn_data().get() + 1);
            }
        }
        let mut active = kn.kn_data().get() > 0;

        if !tp.t_state_isset(TS_CARR_ON) {
            kn.set_flags(EV_EOF);
            if kn.has_flags(__EV_POLL) {
                kn.set_flags(__EV_HUP);
            }
            active = true;
        } else {
            kn.clear_flags(EV_EOF | __EV_HUP);
        }

        active
    }
}

impl<G: ProcGlue> Filter<G> for PtcWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ptcwdetach`.
    fn detach(kn: &Knote<G>) {
        let pti = kn_pti(kn);
        let _s = hal::intr::splraise(hal::intr::IPL_TTY);
        pti.pt_selw.si_note.remove_legacy(kn);
    }

    /// `filt_ptcwrite`: the master is writable when the slave's input has room.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let pti = kn_pti(kn);
        let tp = &pti.pt_tty;
        kn.kn_data().set(0);

        if tp.t_state_isset(TS_ISOPEN) {
            let (rawcc, cancc) = (tp.t_rawq.c_cc.get(), tp.t_canq.c_cc.get());
            if pti.pt_flags.get() & PF_REMOTE != 0 {
                if cancc == 0 {
                    kn.kn_data().set(i64::from(tp.t_canq.c_cn.get()));
                }
            } else if rawcc + cancc < tp.ttyhog() - 2 || (cancc == 0 && tp.t_lflag() & ICANON != 0)
            {
                kn.kn_data()
                    .set(i64::from(tp.t_canq.c_cn.get() - (rawcc + cancc)));
            }
        }
        let mut active = kn.kn_data().get() > 0;

        // Write-side HUP condition is only for poll(2) and select(2).
        if kn.has_flags(__EV_POLL | __EV_SELECT) {
            if !tp.t_state_isset(TS_CARR_ON) {
                kn.set_flags(__EV_HUP);
                active = true;
            } else {
                kn.clear_flags(__EV_HUP);
            }
        }

        active
    }
}

impl<G: ProcGlue> Filter<G> for PtcExcept {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ptcrdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ptcrdetach(kn);
    }

    /// `filt_ptcexcept`: out-of-band data (packet or user-control mode) and, for poll, the
    /// hang-up.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let pti = kn_pti(kn);
        let tp = &pti.pt_tty;
        let mut active = false;

        if kn.kn_sfflags.get() & NOTE_OOB != 0 {
            // If in packet or user control mode, check for data.
            if (pti.pt_flags.get() & PF_PKT != 0 && pti.pt_send.get() != 0)
                || (pti.pt_flags.get() & PF_UCNTL != 0 && pti.pt_ucntl.get() != 0)
            {
                kn.kn_fflags().set(kn.kn_fflags().get() | NOTE_OOB);
                kn.kn_data().set(1);
                active = true;
            }
        }

        if kn.has_flags(__EV_POLL) {
            if !tp.t_state_isset(TS_CARR_ON) {
                kn.set_flags(__EV_HUP);
                active = true;
            } else {
                kn.clear_flags(__EV_HUP);
            }
        }

        active
    }
}

/// `ptckqfilter`: attaches a kqueue filter to the master side.
pub fn ptckqfilter<G: ProcGlue>(dev: Dev, kn: &Knote<G>) -> Result<(), Errno> {
    let Some(pti) = pt_softc::<G>(minor(dev)) else {
        return Err(Errno::ENXIO);
    };

    let klist = match kn.kn_filter().get() {
        EVFILT_READ => {
            kn.kn_fop.set(Some(&PtcFiltops::<G>::READ));
            &pti.pt_selr.si_note
        }
        EVFILT_WRITE => {
            kn.kn_fop.set(Some(&PtcFiltops::<G>::WRITE));
            &pti.pt_selw.si_note
        }
        EVFILT_EXCEPT => {
            kn.kn_fop.set(Some(&PtcFiltops::<G>::EXCEPT));
            &pti.pt_selr.si_note
        }
        _ => return Err(Errno::EINVAL),
    };

    // kn->kn_hook = pti: its minor, compared and never followed.
    kn.kn_hook
        .set(ptr::without_provenance_mut(minor(dev) as usize));

    let _s = hal::intr::splraise(hal::intr::IPL_TTY);
    klist.insert_legacy(kn);
    Ok(())
}

/// `ptytty` (also `ptstty` and `ptctty`): the tty of a pty.
pub fn ptytty<G: ProcGlue>(dev: Dev) -> Option<&'static Tty<G>> {
    pt_softc::<G>(minor(dev)).map(|pti| &*pti.pt_tty)
}

/// The `int` an ioctl's argument holds.
fn int_arg(data: &[u8]) -> Result<i32, Errno> {
    pod_from_bytes::<i32>(data).ok_or(Errno::EINVAL)
}

/// `ptyioctl` (also `ptsioctl` and `ptcioctl`): ioctls on either side of a pty; `master` is
/// the device switch's `cdevsw[major(dev)].d_open == ptcopen`.
pub fn ptyioctl<G: ProcGlue>(
    dev: Dev,
    master: bool,
    cmd: u64,
    data: &mut [u8],
    flag: i32,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let (pti, tp) = pty_of::<G>(dev)?;
    let mut cmd = cmd;

    // IF CONTROLLER STTY THEN MUST FLUSH TO PREVENT A HANG. ttywflush(tp) will hang if
    // there are characters in the outq.
    if cmd == TIOCEXT {
        // When the EXTPROC bit is being toggled, we need to send an TIOCPKT_IOCTL if the
        // packet driver is turned on.
        if int_arg(data)? != 0 {
            if pti.pt_flags.get() & PF_PKT != 0 {
                pti.pt_send.set(pti.pt_send.get() | TIOCPKT_IOCTL as u8);
                ptcwakeup(tp, FREAD);
            }
            tp.t_lflag_set(EXTPROC);
        } else {
            if tp.t_lflag() & EXTPROC != 0 && pti.pt_flags.get() & PF_PKT != 0 {
                pti.pt_send.set(pti.pt_send.get() | TIOCPKT_IOCTL as u8);
                ptcwakeup(tp, FREAD);
            }
            tp.t_lflag_clr(EXTPROC);
        }
        return Ok(());
    } else if master {
        match cmd {
            TIOCGPGRP => {
                // We avoid calling ttioctl on the controller since, in that case, tp must
                // be the controlling terminal.
                let _ = pod_to_bytes(&tp.pgrp_id(), data);
                return Ok(());
            }
            TIOCPKT => {
                if int_arg(data)? != 0 {
                    if pti.pt_flags.get() & PF_UCNTL != 0 {
                        return Err(Errno::EINVAL);
                    }
                    pti.pt_flags.set(pti.pt_flags.get() | PF_PKT);
                } else {
                    pti.pt_flags.set(pti.pt_flags.get() & !PF_PKT);
                }
                return Ok(());
            }
            TIOCUCNTL => {
                if int_arg(data)? != 0 {
                    if pti.pt_flags.get() & PF_PKT != 0 {
                        return Err(Errno::EINVAL);
                    }
                    pti.pt_flags.set(pti.pt_flags.get() | PF_UCNTL);
                } else {
                    pti.pt_flags.set(pti.pt_flags.get() & !PF_UCNTL);
                }
                return Ok(());
            }
            TIOCREMOTE => {
                if int_arg(data)? != 0 {
                    pti.pt_flags.set(pti.pt_flags.get() | PF_REMOTE);
                } else {
                    pti.pt_flags.set(pti.pt_flags.get() & !PF_REMOTE);
                }
                ttyflush(tp, FREAD | FWRITE);
                return Ok(());
            }
            TIOCSETD | TIOCSETA | TIOCSETAW | TIOCSETAF => {
                tp.t_outq.ndflush(tp.t_outq.c_cc.get());
            }
            TIOCSIG => {
                let sig = int_arg(data)? as u32;
                if sig >= NSIG as u32 || sig == 0 {
                    return Err(Errno::EINVAL);
                }
                if tp.t_lflag() & NOFLSH == 0 {
                    ttyflush(tp, FREAD | FWRITE);
                }
                let pg = tp.pgrp();
                pgsignal(pg.as_deref(), sig as i32, true);
                drop(pg);
                if sig as i32 == SIGINFO && tp.t_lflag() & NOKERNINFO == 0 {
                    ttyinfo(tp);
                }
                return Ok(());
            }
            FIONREAD => {
                // FIONREAD on the master side must return the amount in the output queue
                // rather than the input.
                let _ = pod_to_bytes(&tp.t_outq.c_cc.get(), data);
                return Ok(());
            }
            _ => {}
        }
    }
    let mut error = linesw(tp).ioctl(tp, cmd, data, flag, p);
    if error == Ok(false) {
        error = ttioctl(tp, cmd, data, flag, p);
    }
    let error = match error {
        Ok(true) => Ok(()),
        Err(e) => Err(e),
        Ok(false) => 'notty: {
            // Translate TIOCSBRK/TIOCCBRK to user mode ioctls to let the master interpret
            // BREAK conditions.
            match cmd {
                TIOCSBRK => cmd = uioccmd(TIOCUCNTL_SBRK as u8),
                TIOCCBRK => cmd = uioccmd(TIOCUCNTL_CBRK as u8),
                _ => {}
            }
            if pti.pt_flags.get() & PF_UCNTL != 0 && (cmd & !0xff) == uioccmd(0) {
                if cmd & 0xff != 0 {
                    pti.pt_ucntl.set(cmd as u8);
                    ptcwakeup(tp, FREAD);
                }
                return Ok(());
            }
            break 'notty Err(Errno::ENOTTY);
        }
    };
    // If external processing and packet mode send ioctl packet.
    if tp.t_lflag() & EXTPROC != 0
        && pti.pt_flags.get() & PF_PKT != 0
        && matches!(cmd, TIOCSETA | TIOCSETAW | TIOCSETAF)
    {
        pti.pt_send.set(pti.pt_send.get() | TIOCPKT_IOCTL as u8);
        ptcwakeup(tp, FREAD);
    }
    let stop = tp.t_iflag() & IXON != 0
        && cceq(tp.t_cc(VSTOP), ctrl(b's'))
        && cceq(tp.t_cc(VSTART), ctrl(b'q'));
    if pti.pt_flags.get() & PF_NOSTOP != 0 {
        if stop {
            pti.pt_send
                .set((pti.pt_send.get() & !(TIOCPKT_NOSTOP as u8)) | TIOCPKT_DOSTOP as u8);
            pti.pt_flags.set(pti.pt_flags.get() & !PF_NOSTOP);
            ptcwakeup(tp, FREAD);
        }
    } else if !stop {
        pti.pt_send
            .set((pti.pt_send.get() & !(TIOCPKT_DOSTOP as u8)) | TIOCPKT_NOSTOP as u8);
        pti.pt_flags.set(pti.pt_flags.get() | PF_NOSTOP);
        ptcwakeup(tp, FREAD);
    }
    error
}

/// `sysctl_pty`: return pty-related information (`kern.tty` names `sysctl_tty` does not
/// know): none since `KERN_TTY_MAXPTYS` and `KERN_TTY_NPTYS` went.
pub fn sysctl_pty(name: &[i32], _req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    if name.len() != 1 {
        return Err(Errno::ENOTDIR);
    }

    Err(Errno::EOPNOTSUPP)
}

/// `pty_isfree_locked`: check if a pty is free to use.
fn pty_isfree_locked<G: ProcGlue>(minor: u32) -> bool {
    match pt_softc::<G>(minor) {
        None => true,
        Some(pt) => !pt.pt_tty.has_driver(),
    }
}

/// `pty_isfree`.
pub fn pty_isfree<G: ProcGlue>(minor: u32) -> bool {
    let _r = tables::<G>().lock.read();
    pty_isfree_locked::<G>(minor)
}

/// `pty_getfree`: the first free pty, or the one past the array.
pub fn pty_getfree<G: ProcGlue>() -> Dev {
    let t = tables::<G>();
    let r = t.lock.read();
    let npty = t.npty.load(Ordering::Relaxed).max(0) as u32;
    let mut i = 0;
    while i < npty {
        if pty_isfree_locked::<G>(i) {
            break;
        }
        i += 1;
    }
    drop(r);
    makedev(t.pts_major.load(Ordering::Relaxed) as u32, i)
}

/// `pts_major`'s value (`ptmattach` set it).
pub fn pts_major<G: ProcGlue>() -> i32 {
    tables::<G>().pts_major.load(Ordering::Relaxed)
}

/// `major(dev) == pts_major`: whether `dev` names a slave (`ptmioctl`'s checks).
pub fn is_pts<G: ProcGlue>(dev: Dev) -> bool {
    major(dev) as i32 == pts_major::<G>()
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The ptys over the kernel's test glue: a master and its slave passing lines each way
    // through the line discipline, packet and user-control modes, remote mode, the master's
    // filters, the names, the table's growth and limit, the carrier at the master's close.

    use std::boxed::Box;
    use std::vec::Vec;

    use super::*;
    use crate::event::tests::{Kg, T, new_kqueue, setup};
    use crate::event::{test_knote, test_knote_drop};
    use crate::proc::Session;
    use crate::termios::{TIOCGETA, TIOCSTART, TIOCSTOP};
    use crate::tty::{TtyDevsw, register_ttydevsw};

    /// `cdevsw[5].d_stop` of the tests: the pty's own `ptsstop`.
    struct Devsw;

    impl TtyDevsw<Kg> for Devsw {
        fn console_tty(&self) -> bool {
            false
        }
        fn console_write(&self, uio: &mut crate::uio::Uio<'_, Kg>) -> Result<(), Errno> {
            uio.uio_resid = 0;
            Ok(())
        }
        fn stop(&self, tp: &Tty<Kg>, rw: i32) {
            let _ = ptsstop(tp, rw);
        }
        fn console_access(&self, _p: &Thread<Kg>) -> Result<(), Errno> {
            Ok(())
        }
        fn session_has_ttyvp(&self, _sess: &Session<Kg>) -> bool {
            false
        }
    }

    static DEVSW: Devsw = Devsw;

    fn world(pid: i32) -> T {
        let p = setup(pid);
        crate::sig::test_sigacts(p.process());
        Kg::set_curproc(Some(p));
        register_ttydevsw::<Kg>(&DEVSW);
        ptyattach::<Kg>(1);
        set_pts_major::<Kg>(5);
        p
    }

    /// A uio over a buffer.
    struct Buf {
        data: Vec<u8>,
        pos: usize,
        write: bool,
    }

    impl UioMove for Buf {
        fn resid(&self) -> usize {
            self.data.len() - self.pos
        }
        fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
            let n = buf.len().min(self.resid());
            let user = &mut self.data[self.pos..self.pos + n];
            if self.write {
                buf[..n].copy_from_slice(user);
            } else {
                user.copy_from_slice(&buf[..n]);
            }
            self.pos += n;
            Ok(())
        }
        fn nonblock(&self) -> bool {
            false
        }
        fn is_write(&self) -> bool {
            self.write
        }
        fn set_resid(&mut self, resid: usize) {
            self.pos = self.data.len().saturating_sub(resid);
        }
    }

    type Io = fn(Dev, &mut dyn UioMove, i32) -> Result<(), Errno>;

    fn write(f: Io, dev: Dev, s: &[u8]) -> (Result<(), Errno>, usize) {
        let mut b = Buf {
            data: s.to_vec(),
            pos: 0,
            write: true,
        };
        let r = f(dev, &mut b, IO_NDELAY);
        (r, b.pos)
    }

    fn read(f: Io, dev: Dev) -> (Result<(), Errno>, Vec<u8>) {
        let mut b = Buf {
            data: std::vec![0; 64],
            pos: 0,
            write: false,
        };
        let r = f(dev, &mut b, IO_NDELAY);
        let n = b.pos;
        b.data.truncate(n);
        (r, b.data)
    }

    fn ioctl_int(dev: Dev, master: bool, cmd: u64, v: i32, p: T) -> (Result<(), Errno>, i32) {
        let mut d = v.to_ne_bytes();
        let r = ptyioctl::<Kg>(dev, master, cmd, &mut d, 0, p);
        (r, i32::from_ne_bytes(d))
    }

    #[test]
    fn a_line_each_way_between_master_and_slave() {
        let p = world(9101);
        assert_eq!(pty_getfree::<Kg>(), makedev(5, 0));

        let master = makedev(6, 3);
        let slave = makedev(5, 3);
        // The slave waits for its master: without one, a non-blocking open stays WOPEN.
        assert_eq!(ptcopen::<Kg>(master, FREAD | FWRITE, 0, p), Ok(()));
        assert_eq!(
            ptcopen::<Kg>(master, FREAD | FWRITE, 0, p),
            Err(Errno::EIO),
            "busy"
        );
        assert!(!pty_isfree::<Kg>(3));
        assert_eq!(ptsopen::<Kg>(slave, FREAD | FWRITE, 0, p), Ok(()));
        let pti = pt_softc::<Kg>(3).expect("check_pty made the slot");
        assert_eq!(&pti.pty_pn, b"/dev/ptyp3\0");
        assert_eq!(&pti.pty_sn, b"/dev/ttyp3\0");
        assert_eq!(pti.pt_tty.t_dev.get(), slave);

        // Typed on the master: the slave reads the line, the master reads the echo.
        assert_eq!(write(ptcwrite::<Kg>, master, b"hello\n"), (Ok(()), 6));
        assert_eq!(read(ptsread::<Kg>, slave), (Ok(()), b"hello\n".to_vec()));
        assert_eq!(read(ptcread::<Kg>, master), (Ok(()), b"hello\r\n".to_vec()));

        // Written by the slave: output processing, then the master reads it.
        assert_eq!(write(ptswrite::<Kg>, slave, b"out\n"), (Ok(()), 4));
        assert_eq!(read(ptcread::<Kg>, master), (Ok(()), b"out\r\n".to_vec()));
        assert_eq!(read(ptcread::<Kg>, master).0, Err(Errno::EWOULDBLOCK));

        // The slave is a tty: TIOCGETA answers; the master's FIONREAD is the output queue.
        let mut t = [0u8; size_of::<crate::termios::Termios>()];
        assert_eq!(ptyioctl::<Kg>(slave, false, TIOCGETA, &mut t, 0, p), Ok(()));
        assert_eq!(write(ptswrite::<Kg>, slave, b"ab").1, 2);
        assert_eq!(ioctl_int(master, true, FIONREAD, 0, p), (Ok(()), 2));
        // An ioctl neither knows: ENOTTY.
        assert_eq!(
            ioctl_int(slave, false, 0x2000_7400, 0, p).0,
            Err(Errno::ENOTTY)
        );

        // The master's close: the carrier goes, the slave reads EOF, the pty is free.
        assert_eq!(ptsclose::<Kg>(slave, FREAD | FWRITE, 0, Some(p)), Ok(()));
        assert_eq!(ptcclose::<Kg>(master, FREAD | FWRITE, 0, Some(p)), Ok(()));
        assert!(pty_isfree::<Kg>(3));
        assert!(!pti.pt_tty.t_state_isset(TS_CARR_ON));
        assert_eq!(write(ptswrite::<Kg>, slave, b"x").0, Err(Errno::EIO));
    }

    #[test]
    fn packet_mode_reports_stop_start_and_flushes() {
        let p = world(9102);
        let master = makedev(6, 4);
        let slave = makedev(5, 4);
        assert_eq!(ptcopen::<Kg>(master, FREAD | FWRITE, 0, p), Ok(()));
        assert_eq!(ptsopen::<Kg>(slave, FREAD | FWRITE, 0, p), Ok(()));
        assert_eq!(ioctl_int(master, true, TIOCPKT, 1, p).0, Ok(()));
        // TIOCUCNTL refuses while in packet mode.
        assert_eq!(
            ioctl_int(master, true, TIOCUCNTL, 1, p).0,
            Err(Errno::EINVAL)
        );
        // The slave's ^S (TIOCSTOP): the master reads TIOCPKT_STOP alone.
        assert_eq!(ioctl_int(slave, false, TIOCSTOP, 0, p).0, Ok(()));
        let (r, got) = read(ptcread::<Kg>, master);
        assert_eq!(r, Ok(()));
        assert_eq!(got, [TIOCPKT_STOP as u8]);
        // ^Q: TIOCPKT_START.
        assert_eq!(ioctl_int(slave, false, TIOCSTART, 0, p).0, Ok(()));
        assert_eq!(read(ptcread::<Kg>, master).1, [TIOCPKT_START as u8]);
        // Data in packet mode comes after a 0 byte.
        assert_eq!(write(ptswrite::<Kg>, slave, b"d").1, 1);
        assert_eq!(read(ptcread::<Kg>, master).1, [0, b'd']);
        // A flush of the slave's input and output is FLUSHREAD|FLUSHWRITE.
        assert_eq!(
            ioctl_int(slave, false, crate::termios::TIOCFLUSH, 0, p).0,
            Ok(())
        );
        assert_eq!(read(ptcread::<Kg>, master).1, [(FREAD | FWRITE) as u8]);
        // IXON off: NOSTOP reported once.
        let tp = &pt_softc::<Kg>(4).unwrap().pt_tty;
        tp.set_t_iflag(tp.t_iflag() & !IXON);
        assert_eq!(
            ioctl_int(slave, false, TIOCGETA, 0, p).0,
            Err(Errno::EINVAL)
        );
        let got = read(ptcread::<Kg>, master).1;
        assert_eq!(got[0] as i32 & TIOCPKT_NOSTOP, TIOCPKT_NOSTOP);
    }

    #[test]
    fn user_control_and_remote_modes() {
        let p = world(9103);
        let master = makedev(6, 5);
        let slave = makedev(5, 5);
        assert_eq!(ptcopen::<Kg>(master, FREAD | FWRITE, 0, p), Ok(()));
        assert_eq!(ptsopen::<Kg>(slave, FREAD | FWRITE, 0, p), Ok(()));
        assert_eq!(ioctl_int(master, true, TIOCUCNTL, 1, p).0, Ok(()));
        assert_eq!(ioctl_int(master, true, TIOCPKT, 1, p).0, Err(Errno::EINVAL));
        // TIOCSBRK on the slave becomes a user-control byte for the master.
        assert_eq!(ioctl_int(slave, false, TIOCSBRK, 0, p).0, Ok(()));
        assert_eq!(read(ptcread::<Kg>, master).1, [TIOCUCNTL_SBRK as u8]);
        assert_eq!(ioctl_int(master, true, TIOCUCNTL, 0, p).0, Ok(()));
        // Remote mode: the master's write is one record in the canonical queue, read whole,
        // its NUL terminator dropped.
        assert_eq!(ioctl_int(master, true, TIOCREMOTE, 1, p).0, Ok(()));
        assert_eq!(write(ptcwrite::<Kg>, master, b"rec"), (Ok(()), 3));
        assert_eq!(read(ptsread::<Kg>, slave).1, b"rec");
        assert_eq!(read(ptsread::<Kg>, slave).0, Err(Errno::EWOULDBLOCK));
        // TIOCSIG: a signal number in range, else EINVAL.
        assert_eq!(ioctl_int(master, true, TIOCSIG, 0, p).0, Err(Errno::EINVAL));
        assert_eq!(
            ioctl_int(master, true, TIOCSIG, NSIG, p).0,
            Err(Errno::EINVAL)
        );
        assert_eq!(ioctl_int(master, true, TIOCSIG, 30, p).0, Ok(()));
        // TIOCGPGRP on the master: the slave's group (none: 0).
        assert_eq!(ioctl_int(master, true, TIOCGPGRP, 7, p), (Ok(()), 0));
    }

    #[test]
    fn the_master_filters_follow_the_slave() {
        let p = world(9104);
        let (_kfd, kq) = new_kqueue(p);
        let master = makedev(6, 6);
        let slave = makedev(5, 6);
        assert_eq!(ptcopen::<Kg>(master, FREAD | FWRITE, 0, p), Ok(()));
        assert_eq!(ptsopen::<Kg>(slave, FREAD | FWRITE, 0, p), Ok(()));

        let h = test_knote(&kq, 1);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_READ);
        assert_eq!(ptckqfilter::<Kg>(master, kn), Ok(()));
        assert_eq!(kn.kn_hook.get().addr(), 6, "the hook is the minor");
        let event = kn.fop().f_event.unwrap();
        assert!(!event(kn, 0));
        assert_eq!(write(ptswrite::<Kg>, slave, b"xy").1, 2);
        assert!(event(kn, 0));
        assert_eq!(kn.kn_data().get(), 2);
        (kn.fop().f_detach.unwrap())(kn);
        test_knote_drop(&kq, h);

        let h = test_knote(&kq, 2);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_WRITE);
        assert_eq!(ptckqfilter::<Kg>(master, kn), Ok(()));
        assert!((kn.fop().f_event.unwrap())(kn, 0));
        assert!(kn.kn_data().get() > 0);
        (kn.fop().f_detach.unwrap())(kn);
        test_knote_drop(&kq, h);

        let h = test_knote(&kq, 3);
        let kn = h.kn();
        kn.kn_kevent.filter.set(-13);
        assert_eq!(ptckqfilter::<Kg>(master, kn), Err(Errno::EINVAL));
        test_knote_drop(&kq, h);
        // No softc: ENXIO.
        let h = test_knote(&kq, 4);
        assert_eq!(
            ptckqfilter::<Kg>(makedev(6, 900), h.kn()),
            Err(Errno::ENXIO)
        );
        test_knote_drop(&kq, h);
    }

    #[test]
    fn names_the_table_and_its_limit() {
        let _p = world(9105);
        assert_eq!(ptydevname(0), (*b"/dev/ptyp0\0", *b"/dev/ttyp0\0"));
        assert_eq!(ptydevname(62), (*b"/dev/ptyq0\0", *b"/dev/ttyq0\0"));
        assert_eq!(ptydevname(16 * 62), ([0; 11], [0; 11]), "past the letters");
        // The table grows by powers of two up to maxptys.
        assert_eq!(check_pty::<Kg>(makedev(5, 20)), Ok(()));
        assert_eq!(tables::<Kg>().npty.load(Ordering::Relaxed), 32);
        assert_eq!(
            check_pty::<Kg>(makedev(5, NPTY_MAX as u32)),
            Err(Errno::ENXIO)
        );
        assert!(pt_softc::<Kg>(19).is_none());
        assert!(pt_softc::<Kg>(20).is_some());
        // sysctl_pty knows no name.
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(sysctl_pty(&[6], &mut req), Err(Errno::EOPNOTSUPP));
        assert_eq!(sysctl_pty(&[6, 1], &mut req), Err(Errno::ENOTDIR));
        // kern.tty's unknown names reach it.
        assert_eq!(
            crate::tty::sysctl_tty::<Kg>(&[7], &mut req),
            Err(Errno::EOPNOTSUPP)
        );
    }

    #[test]
    fn the_softc_is_shared() {
        const fn shared<T: Send + Sync>() {}
        shared::<PtSoftc<Kg>>();
        shared::<PtyTables<Kg>>();
        let _ = Box::new(PtyTables::<Kg>::new());
    }
}
/* </TESTS> */
