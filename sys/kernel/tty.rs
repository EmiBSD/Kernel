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
 * Copyright (c) 1982, 1986, 1990, 1991, 1993
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
 *	@(#)tty.c	8.8 (Berkeley) 1/21/94
 */

/*-
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)tty.h	8.6 (Berkeley) 1/21/94
 */

/*-
 * Copyright (c) 1982, 1986, 1991, 1993
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
 *	@(#)tty_conf.c	8.4 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The terminal: `kern/tty.c` (the termios line discipline), `<sys/tty.h>` (`struct tty`,
//! the `TS_*` state, the character classes, `struct itty`, `struct ptmget`) and
//! `kern/tty_conf.c` (the line discipline switch).
//!
//! Upstream: sys/kern/tty.c @ 3ce1f3f79392
//! Upstream: sys/sys/tty.h @ 3ce1f3f79392
//! Upstream: sys/kern/tty_conf.c @ 3ce1f3f79392
//! LZ: sys/kern/tty.rs@3c62ede68802
//! LZ: sys/sys/tty.rs@3c62ede68802
//! LZ: sys/kern/tty_conf.rs@3c62ede68802
//!
//! A tty driver (`com(4)`, `pluart(4)`, the ptys) hands every received character to
//! [`ttyinput`] through its line discipline's `rint` and starts output through its
//! [`TtyDriver`]; readers and writers come in through the discipline's `read`/`write`
//! ([`ttread`], [`ttwrite`]) and the terminal ioctls through [`ttioctl`]. This is the termios
//! discipline: canonical (line) and raw input, echo and erase, the signal characters sent to
//! the foreground process group, flow control, output processing, job control of background
//! readers and writers, `^T` status ([`ttyinfo`]) and the list of ttys behind `kern.tty`.
//!
//! Locking. The C keeps a tty under `spltty()` and the kernel lock (the vnode layer takes it
//! for read, write and ioctl; the tty interrupts are not MPSAFE). Here the multi-word state
//! (`c_cc`, the window size, the output column, the foreground group, the session, the
//! driver) is a [`TtyState`] owned by the tty's `Mutexed` at `IPL_TTY`; the words the drivers
//! and the filters read and change without `spltty()` in the C (`t_state`, the four termios
//! flag words, the speeds, `t_dev`, `t_line`, the water marks, `t_gen`, the counters) are
//! relaxed atomic words beside it, and a compound check-then-set of them rests, as in the C,
//! on the kernel lock and the raised level. Each queue has its own lock (`kernel::clist`),
//! taken inside the tty's. Every section of the C runs as a `Sect`: the level raised to
//! `IPL_TTY` for its whole life (the C's `spltty()`), the state's lock held, and let go
//! around every call that leaves this module (the driver's `start`, `param`, `hwiflow` and
//! `d_stop`, `selwakeup`, the signals, the sleeps, `uiomove`, `ttyinfo`), then taken again:
//! nothing outside the module runs with the state locked, so a driver, a filter or a
//! re-entered tty function finds it free, and the C's order of effects is kept.
//!
//! ## Deviations
//! - `ttyinfo` walks the chosen process's threads (and sums them) under the process's
//!   `ps_mtx`, where the C walks `ps_threads` under the kernel lock alone; the group's
//!   members are walked under the kernel lock's reader, in the C's order (LZ's, N4.2).
//! - Functions returning an errno return `Result`; [`ttioctl`] returns `Ok(false)` for the
//!   C's `-1` ("not a tty ioctl"). The C lets an `ERESTART` from a background sleep in
//!   `ttioctl` look like `-1` too; here it propagates as `ERESTART` (LZ's).
//! - `cc = tp->t_cc` (a pointer to the live array) is a copy taken where the C takes the
//!   pointer, read again where a sleep or a call out could have changed it (LZ's).
//! - `CCEQ` and `TTBREAKC` compare the `int` character, `TTY_QUOTE` included, as the C does
//!   (`cceq` on an `int`); `kernel::termios::cceq` takes bytes and is for the drivers (LZ's).
//! - `ttread`'s `VMIN`/`VTIME` timer is a `struct timeout` on `ttread`'s stack instead of a
//!   `malloc(M_TEMP)` one (its argument is the channel it wakes, never followed); it is
//!   deleted on every way out, as the C frees it (LZ's).
//! - `TIOCCONS` checks that `/dev/console` can be opened (`namei` and `VOP_ACCESS`), as the C
//!   does, through the device switch's bridge ([`TtyDevsw::console_access`]): without a root
//!   file system that lookup fails with `ENOENT` (LZ's).
//! - `ttymalloc` panics if the tty pool is empty (`M_WAITOK` cannot fail in C) (LZ's).
//! - The line disciplines `ppp`, `nmea`, `msts` and `endrun` are not configured (their code
//!   is not ported), so slots 5, 7, 8 and 9 hold the defunct entries the C uses when `NPPP`,
//!   `NNMEA`, `NMSTS` or `NENDRUN` is 0 (LZ's, visible: `TIOCSETD` to them answers `ENODEV`).
//! - `ttyfree` retires the tty (off the list, its timeout stopped, its knotes told EOF, its
//!   rings freed) but its pool item stays: the drivers that free one (com, ucom, wsdisplay)
//!   still hold it as `&'static` (decision 5 of N4.7, until N7 moves them).
//! - `printf(9)` redirected by `TIOCCONS` (`kputchar`'s `constty` leg) goes through
//!   [`tputchar_try`]: a character that would have to wait for the tty (its state or output
//!   queue held on this CPU, its state held on another, or the scheduler's lock held here)
//!   goes to the real console instead. The C raises `spltty()` and writes the tty whatever
//!   it was doing; with the state behind a lock that re-entry would be "locking against
//!   myself", or a deadlock with a `printf` under `SCHED_LOCK` across CPUs.
//!
//! ## Redesign
//! - LZ's `struct tty` (sys/sys/tty.rs) was a `Cell` per member under an `unsafe impl Sync`
//!   ("changed at spltty() on the one CPU"), its foreground group and session raw pointers
//!   turned back into `&'static` by two `unsafe` derefs, its driver hooks `Cell<Option<fn>>`,
//!   allocated by `malloc(M_TTYS)` and an `unsafe` write, freed by an `unsafe fn ttyfree`,
//!   linked on a `queue.h` list under an `unsafe` insert, its timeout's and knotes' argument a
//!   raw pointer cast back (`kn_tty`, `ttrstrt`, `ttvtimeout`, 11 `unsafe` in tty.rs, 6 in
//!   sys/tty.rs). [`Tty`] is an item of a recorded pool (`ttypl`, `vmstat -m`'s new row; its
//!   rings stay `ttys`), handed out as a counted [`TtyRef`]; the state is a [`TtyState`]
//!   inside the tty's lock (decision 3 of N4.7); the foreground group and the session are
//!   counted handles (`SESSHOLD`/`SESSRELE` are the session handle's clone and drop; the
//!   group's is cleared by `pgdelete` through [`pgrp_gone`]); the list is a `PinList` under
//!   `ttylist_lock` in the glue's tables ([`TtyTables`], with `tty_count`, `constty`, the
//!   console's device and the device switch's bridge); `t_rstrt_to` holds the tty as an
//!   erased handle (`Timeout::set_shared`), and a knote holds it through its counted hook
//!   (`Knote::set_hook_shared`, [`ttkqfilter`]).
//! - `struct linesw` is the [`LineDiscipline`] trait (decision 46): the termios discipline
//!   ([`TermiosDisc`]) and the defunct one ([`Defunct`]) are its two implementations, and
//!   `linesw[]` is [`Discipline::TABLE`], indexed by `t_line`. `bsd` keeps a forwarding
//!   `struct linesw` table for the drivers that call `linesw[tp->t_line].l_rint` (N7).
//! - `t_oproc`, `t_param` and `t_hwiflow` are the [`TtyDriver`] trait (decision 46): the pty
//!   implements it; the inherited drivers set the three functions as before
//!   (`set_oproc`, ...), which the tty keeps as a table of function pointers until N7.
//!   `t_oproc == NULL` (the pty's "controller gone") is a tty without a driver
//!   ([`Tty::has_driver`]).
//! - `ttyread_filtops`, `ttywrite_filtops` and `ttyexcept_filtops` are [`TtyRead`],
//!   [`TtyWrite`] and [`TtyExcept`], `impl kernel::event::Filter`s (decision 46).
//! - `kern.tty` is [`TtyNode`], an `impl kernel::sysctl::SysctlNode`; `sysctl_tty` stays the
//!   function `bsd`'s `kern_sysctl` calls; `struct itty` is an `abi_pod!` (LZ: an `unsafe impl
//!   SysctlPlain`), the array built into an `MArray<u8>` of `M_SYSCTL` (LZ: a raw block and an
//!   `unsafe` slice over it).
//! - `ttypend` requeues the raw queue in place: as many times as it held characters, one is
//!   taken from its head and given to `ttyinput`, which may append it at its tail; each step
//!   frees a slot before one is filled, so no unread input is overwritten. The C (and LZ)
//!   read through a copy of the queue's header over the live ring (LZ's `copy_header`).
//! - A section lets the state's lock go around `wakeup` too ([`Sect::out`]), so no CPU waits
//!   for `SCHED_LOCK` while holding a tty's state.
//! - `d_stop` (`cdevsw[major(t_dev)].d_stop`), `/dev/console`'s write and access check and
//!   `s_ttyvp` are reached through the [`TtyDevsw`] bridge (bridges.toml, paid at N7 with the
//!   device switch); `enqueue_randomness` through hal's `Random` bridge.
//! - The `t_sc` member (`net/if_sl.c`'s softc, no user) is gone, with SLIP.

use core::fmt;
use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::{AtomicI32, AtomicI64, AtomicU32, Ordering};

use hal::intr::{IPL_TTY, SplGuard};
use hal::klock;
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{AnyArc, PR_WAITOK, PoolArc, PoolHome, TypedPool};
use hal::rwlock::RawRwlock;
use hal::sleep::{INFSLP, PCATCH};
use hal::sys::errno::Errno;
use hal::sys::fcntl::{FNONBLOCK, FREAD, FWRITE};
use hal::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use hal::sys::signal::{
    SIGHUP, SIGINFO, SIGINT, SIGIO, SIGQUIT, SIGTSTP, SIGTTIN, SIGTTOU, SIGWINCH,
};
use hal::sys::types::{Dev, Pid};
use hal::user::{pod_from_bytes, pod_to_bytes};

use crate::assert::kpanic;
use crate::clist::Clist;
pub use crate::clist::TTY_QUOTE;
use crate::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_EOF, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE,
    FILTEROP_ISFD, Filter, Filterops, Knote,
};
use crate::proc::{
    I16Word, I32Word, I64Word, NO_PID, P_WEXIT, PS_CONTROLT, PS_EMBRYO, PS_EXITING, PS_PPWAIT,
    PgrpRef, ProcGlue, Process, SONPROC, SRUN, SessionRef, Thread, U8Word, pgfind, prfind,
};
use crate::resource::{Timeval, Tusage, calctsru, tuagg_sumup};
use crate::sched::{FSCALE, FSHIFT, NOWAKE, PZERO};
use crate::select::{Selinfo, selwakeup};
use crate::sig::{pgsignal, prsignal, sigismasked, sigpending};
use crate::sigio::{TIOCGPGRP, TIOCSPGRP};
use crate::sysctl::{SysctlNode, SysctlReq, rdquad, rdstruct};
use crate::termios::{
    _POSIX_VDISABLE, ALTWERASE, BRKINT, CEOT, CHWFLOW, CIGNORE, CLOCAL, CREAD, Cc, ECHO, ECHOCTL,
    ECHOE, ECHOK, ECHOKE, ECHONL, ECHOPRT, EXTPROC, FLUSHO, ICANON, ICRNL, IEXTEN, IGNBRK, IGNCR,
    IGNPAR, IMAXBEL, INLCR, INPCK, ISIG, ISTRIP, IUCLC, IXANY, IXOFF, IXON, MDMBUF, NCCS, NOFLSH,
    NOKERNINFO, OCRNL, OLCUC, ONLCR, ONLRET, ONOCR, ONOEOT, OPOST, OXTABS, PARMRK, PENDIN,
    TIOCCBRK, TIOCCONS, TIOCDRAIN, TIOCEXCL, TIOCFLUSH, TIOCGETA, TIOCGETD, TIOCGSID, TIOCGTSTAMP,
    TIOCGWINSZ, TIOCM_CAR, TIOCM_CTS, TIOCNXCL, TIOCOUTQ, TIOCSBRK, TIOCSCTTY, TIOCSETA, TIOCSETAF,
    TIOCSETAW, TIOCSETD, TIOCSTART, TIOCSTAT, TIOCSTOP, TIOCSTSTAMP, TIOCSWINSZ, TOSTOP,
    TTYDEFCHARS, Tcflag, Termios, Tstamps, VDISCARD, VDSUSP, VEOF, VEOL, VEOL2, VERASE, VINTR,
    VKILL, VLNEXT, VMIN, VQUIT, VREPRINT, VSTART, VSTATUS, VSTOP, VSUSP, VTIME, VWERASE, Winsize,
    XCASE, ctrl,
};
use crate::timeout::{KCLOCK_NONE, Timeout};
use crate::uio::UioMove;

/// `IO_NDELAY` (`<sys/vnode.h>`): an I/O that must not block (`FNONBLOCK` on the file).
pub const IO_NDELAY: i32 = 0x10;

/// Quad: input chars.
pub const KERN_TTY_TKNIN: i32 = 1;
/// Quad: output chars.
pub const KERN_TTY_TKNOUT: i32 = 2;
/// Quad: input chars, raw mode.
pub const KERN_TTY_TKRAWCC: i32 = 3;
/// Quad: input char, cooked mode.
pub const KERN_TTY_TKCANCC: i32 = 4;
/// Struct: tty stats.
pub const KERN_TTY_INFO: i32 = 5;
// was KERN_TTY_MAXPTYS 6, was KERN_TTY_NPTYS 7
/// Number of `kern.tty` names.
pub const KERN_TTY_MAXID: usize = 8;

/// Get ptys (`PTMGET`, `/dev/ptm`'s ioctl).
pub const PTMGET: u64 = hal::sys::ioccom::_ior::<Ptmget>(b't', 1);
/// The cloning pty device.
pub const PATH_PTMDEV: &[u8] = b"/dev/ptm";
/// XXX evil hardcoding of tty gid.
pub const TTY_GID: u32 = 4;

/// Sleep priority for tty reads.
pub const TTIPRI: i32 = 25;
/// Sleep priority for tty writes.
pub const TTOPRI: i32 = 26;

/// `OBUFSIZ`.
pub const OBUFSIZ: usize = 512;

/// `TTMAXLOWAT`.
pub const TTMAXLOWAT: i32 = 256;
/// `TTMINLOWAT`.
pub const TTMINLOWAT: i32 = 32;
/// `TTMINHIWAT`.
pub const TTMINHIWAT: i32 = 100;
/// Min space above hiwat.
pub const TTHIWATMINSPACE: i32 = 200;

// These flags are kept in t_state.

/// Process waiting for tty.
pub const TS_ASLEEP: i32 = 0x00001;
/// Tty in async I/O mode.
pub const TS_ASYNC: i32 = 0x00002;
/// Draining output.
pub const TS_BUSY: i32 = 0x00004;
/// Carrier is present.
pub const TS_CARR_ON: i32 = 0x00008;
/// Outq has been flushed during DMA.
pub const TS_FLUSH: i32 = 0x00010;
/// Open has completed.
pub const TS_ISOPEN: i32 = 0x00020;
/// Further input blocked.
pub const TS_TBLOCK: i32 = 0x00040;
/// Wait for output char processing.
pub const TS_TIMEOUT: i32 = 0x00080;
/// Output paused.
pub const TS_TTSTOP: i32 = 0x00100;
/// Open in progress.
pub const TS_WOPEN: i32 = 0x00200;
/// Tty requires exclusivity.
pub const TS_XCLUDE: i32 = 0x00400;

// State for intra-line fancy editing work.

/// State for lowercase \ work.
pub const TS_BKSL: i32 = 0x00800;
/// Counting tab width, ignore FLUSHO.
pub const TS_CNTTB: i32 = 0x01000;
/// Within a \.../ for PRTRUB.
pub const TS_ERASE: i32 = 0x02000;
/// Next character is literal.
pub const TS_LNCH: i32 = 0x04000;
/// Retyping suspended input (PENDIN).
pub const TS_TYPEN: i32 = 0x08000;
/// The editing state.
pub const TS_LOCAL: i32 = TS_BKSL | TS_CNTTB | TS_ERASE | TS_LNCH | TS_TYPEN;

/// Update timestamp on DCD set.
pub const TS_TSTAMPDCDSET: i32 = 0x10000;
/// Update timestamp on DCD clr.
pub const TS_TSTAMPDCDCLR: i32 = 0x20000;
/// Update timestamp on CTS set.
pub const TS_TSTAMPCTSSET: i32 = 0x40000;
/// Update timestamp on CTS clr.
pub const TS_TSTAMPCTSCLR: i32 = 0x80000;

// Character type information.

/// Ordinary character.
pub const ORDINARY: u8 = 0;
/// Control character.
pub const CONTROL: u8 = 1;
/// Backspace.
pub const BACKSPACE: u8 = 2;
/// Newline.
pub const NEWLINE: u8 = 3;
/// Horizontal tab.
pub const TAB: u8 = 4;
/// Vertical tab.
pub const VTAB: u8 = 5;
/// Carriage return.
pub const RETURN: u8 = 6;

// Modem control commands (driver).

/// Set the modem lines.
pub const DMSET: i32 = 0;
/// Set bits.
pub const DMBIS: i32 = 1;
/// Clear bits.
pub const DMBIC: i32 = 2;
/// Get the modem lines.
pub const DMGET: i32 = 3;

// Flags on a character passed to ttyinput.

/// Character mask.
pub const TTY_CHARMASK: i32 = 0x0000_00ff;
/// Error mask.
pub const TTY_ERRORMASK: i32 = 0xff00_0000_u32 as i32;
/// Framing error or BREAK condition.
pub const TTY_FE: i32 = 0x0100_0000;
/// Parity error.
pub const TTY_PE: i32 = 0x0200_0000;

// Symbolic sleep message strings.

/// `ttclos`.
pub const TTCLOS: &str = "ttycls";
/// `ttopen`.
pub const TTOPEN: &str = "ttyopn";
/// `ttybg`.
pub const TTYBG: &str = "ttybg";
/// `ttyin`.
pub const TTYIN: &str = "ttyin";
/// `ttyout`.
pub const TTYOUT: &str = "ttyout";

// Table with character classes and parity. The 8th bit indicates parity, the 7th bit
// indicates the character is an alphameric or underscore (for ALTWERASE), and the low 6 bits
// indicate delay type. If the low 6 bits are 0 then the character needs no special
// processing on output; classes other than 0 might be translated or (not currently) require
// delays.

/// Even parity.
const E: u8 = 0x00;
/// Odd parity.
const O: u8 = 0x80;
/// Alpha or underscore.
const ALPHA: u8 = 0x40;
/// The class bits of a `char_type` entry.
pub const CCLASSMASK: u8 = 0x3f;

const BS: u8 = BACKSPACE;
const CC: u8 = CONTROL;
const CR: u8 = RETURN;
const NA: u8 = ORDINARY | ALPHA;
const NL: u8 = NEWLINE;
const NO: u8 = ORDINARY;
const TB: u8 = TAB;
const VT: u8 = VTAB;

/// `char_type[]`: the class and parity of every character; the meta characters (128-255)
/// are ordinary alphamerics (should be settable per character set).
pub const CHAR_TYPE: [u8; 256] = {
    const LOW: [u8; 128] = [
        E | CC,
        O | CC,
        O | CC,
        E | CC,
        O | CC,
        E | CC,
        E | CC,
        O | CC, // nul - bel
        O | BS,
        E | TB,
        E | NL,
        O | CC,
        E | VT,
        O | CR,
        O | CC,
        E | CC, // bs - si
        O | CC,
        E | CC,
        E | CC,
        O | CC,
        E | CC,
        O | CC,
        O | CC,
        E | CC, // dle - etb
        E | CC,
        O | CC,
        O | CC,
        E | CC,
        O | CC,
        E | CC,
        E | CC,
        O | CC, // can - us
        O | NO,
        E | NO,
        E | NO,
        O | NO,
        E | NO,
        O | NO,
        O | NO,
        E | NO, // sp - '
        E | NO,
        O | NO,
        O | NO,
        E | NO,
        O | NO,
        E | NO,
        E | NO,
        O | NO, // ( - /
        E | NA,
        O | NA,
        O | NA,
        E | NA,
        O | NA,
        E | NA,
        E | NA,
        O | NA, // 0 - 7
        O | NA,
        E | NA,
        E | NO,
        O | NO,
        E | NO,
        O | NO,
        O | NO,
        E | NO, // 8 - ?
        O | NO,
        E | NA,
        E | NA,
        O | NA,
        E | NA,
        O | NA,
        O | NA,
        E | NA, // @ - G
        E | NA,
        O | NA,
        O | NA,
        E | NA,
        O | NA,
        E | NA,
        E | NA,
        O | NA, // H - O
        E | NA,
        O | NA,
        O | NA,
        E | NA,
        O | NA,
        E | NA,
        E | NA,
        O | NA, // P - W
        O | NA,
        E | NA,
        E | NA,
        O | NO,
        E | NO,
        O | NO,
        O | NO,
        O | NA, // X - _
        E | NO,
        O | NA,
        O | NA,
        E | NA,
        O | NA,
        E | NA,
        E | NA,
        O | NA, // ` - g
        O | NA,
        E | NA,
        E | NA,
        O | NA,
        E | NA,
        O | NA,
        O | NA,
        E | NA, // h - o
        O | NA,
        E | NA,
        E | NA,
        O | NA,
        E | NA,
        O | NA,
        O | NA,
        E | NA, // p - w
        E | NA,
        O | NA,
        O | NA,
        E | NO,
        O | NO,
        E | NO,
        E | NO,
        O | CC, // x - del
    ];
    let mut t = [NA; 256];
    let mut i = 0;
    while i < 128 {
        t[i] = LOW[i];
        i += 1;
    }
    t
};

/// `nlinesw`: the line discipline slots.
pub const NLINESW: usize = 10;

/// `M_SYSCTL`: sysctl persistent buffers (`kern.tty.ttyinfo`'s array).
const M_SYSCTL: i32 = 11;

/// `struct ptmget`, for `/dev/ptm`'s pty getting ioctl `PTMGET`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ptmget {
    /// The controlling (master) side's descriptor.
    pub cfd: i32,
    /// The slave side's descriptor.
    pub sfd: i32,
    /// The master's path, NUL-terminated.
    pub cn: [u8; 16],
    /// The slave's path, NUL-terminated.
    pub sn: [u8; 16],
}

hal::abi_pod!(Ptmget {
    cfd: i32,
    sfd: i32,
    cn: [u8; 16],
    sn: [u8; 16],
});

/// `struct itty`: small version of `struct tty` exported via sysctl `KERN_TTY_INFO`. The C
/// compiler's padding is named, so the copy-out sends zeros there.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Itty {
    /// `t_dev`.
    pub t_dev: Dev,
    /// `t_rawq_c_cc`.
    pub t_rawq_c_cc: i32,
    /// `t_canq_c_cc`.
    pub t_canq_c_cc: i32,
    /// `t_outq_c_cc`.
    pub t_outq_c_cc: i32,
    /// `t_hiwat`.
    pub t_hiwat: i16,
    /// `t_lowat`.
    pub t_lowat: i16,
    /// `t_column`.
    pub t_column: i16,
    /// The padding before `t_state`.
    pub _pad0: i16,
    /// `t_state`.
    pub t_state: i32,
    /// `t_session_id`.
    pub t_session_id: Pid,
    /// `t_pgrp_pg_id`.
    pub t_pgrp_pg_id: Pid,
    /// `t_line`.
    pub t_line: u8,
    /// The tail padding.
    pub _pad1: [u8; 3],
}

hal::abi_pod!(Itty {
    t_dev: Dev,
    t_rawq_c_cc: i32,
    t_canq_c_cc: i32,
    t_outq_c_cc: i32,
    t_hiwat: i16,
    t_lowat: i16,
    t_column: i16,
    _pad0: i16,
    t_state: i32,
    t_session_id: Pid,
    t_pgrp_pg_id: Pid,
    t_line: u8,
    _pad1: [u8; 3],
});

/// `struct speedtab`: a speed and the driver's code for it; a table ends with `sp_speed`
/// -1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Speedtab {
    /// Speed.
    pub sp_speed: i32,
    /// Code.
    pub sp_code: i32,
}

/// A word of flag bits the C changes with `SET`/`CLR` without a lock (`t_state`, the termios
/// flags): each change one atomic read-modify-write.
#[derive(Debug, Default)]
pub struct FlagWord(AtomicU32);

impl FlagWord {
    /// `v`.
    pub const fn new(v: u32) -> Self {
        Self(AtomicU32::new(v))
    }

    /// The bits.
    #[inline]
    pub fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }

    /// All the bits.
    #[inline]
    pub fn set(&self, v: u32) {
        self.0.store(v, Ordering::Relaxed);
    }

    /// `SET(w, bits)`.
    #[inline]
    pub fn or(&self, bits: u32) {
        self.0.fetch_or(bits, Ordering::Relaxed);
    }

    /// `CLR(w, bits)`.
    #[inline]
    pub fn clr(&self, bits: u32) {
        self.0.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `ISSET(w, bits)`.
    #[inline]
    pub fn isset(&self, bits: u32) -> bool {
        self.get() & bits != 0
    }
}

/// `t_state`: the `TS_*` bits (an `int`, changed by the drivers too).
#[derive(Debug, Default)]
pub struct StateWord(AtomicI32);

impl StateWord {
    /// `v`.
    pub const fn new(v: i32) -> Self {
        Self(AtomicI32::new(v))
    }

    /// The bits.
    #[inline]
    pub fn get(&self) -> i32 {
        self.0.load(Ordering::Relaxed)
    }

    /// All the bits (`tp->t_state = v`).
    #[inline]
    pub fn set(&self, v: i32) {
        self.0.store(v, Ordering::Relaxed);
    }
}

/// `t_oproc`: start output.
pub type TOproc<G> = fn(&Tty<G>);
/// `t_param`: set hardware state.
pub type TParam<G> = fn(&Tty<G>, &Termios) -> Result<(), Errno>;
/// `t_hwiflow`: set hardware flow control; nonzero when it took effect.
pub type THwiflow<G> = fn(&Tty<G>, i32) -> i32;

/// The driver half of a tty: `t_oproc`, `t_param`, `t_hwiflow`.
enum Driver<G: ProcGlue> {
    /// `t_oproc == NULL`: no driver (a pty whose master is closed, a tty not opened yet).
    None,
    /// A driver of this crate ([`TtyDriver`]: the pty).
    Dyn(&'static dyn TtyDriver<G>),
    /// The three functions an inherited driver set ([`Tty::set_oproc`], ...).
    Table(FnTable<G>),
}

impl<G: ProcGlue> Clone for Driver<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: ProcGlue> Copy for Driver<G> {}

/// The function pointers of an inherited driver (`None`: the C's `NULL`).
struct FnTable<G: ProcGlue> {
    /// `t_oproc`.
    oproc: Option<TOproc<G>>,
    /// `t_param`.
    param: Option<TParam<G>>,
    /// `t_hwiflow`.
    hwiflow: Option<THwiflow<G>>,
}

impl<G: ProcGlue> Clone for FnTable<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: ProcGlue> Copy for FnTable<G> {}

impl<G: ProcGlue> Driver<G> {
    /// `t_oproc != NULL`.
    fn present(&self) -> bool {
        match self {
            Driver::None => false,
            Driver::Dyn(_) => true,
            Driver::Table(t) => t.oproc.is_some(),
        }
    }

    /// `(*tp->t_oproc)(tp)`, if there is one.
    fn start(&self, tp: &Tty<G>) {
        match self {
            Driver::None => {}
            Driver::Dyn(d) => d.start(tp),
            Driver::Table(t) => {
                if let Some(oproc) = t.oproc {
                    oproc(tp);
                }
            }
        }
    }

    /// `tp->t_param ? (*tp->t_param)(tp, t) : 0`.
    fn param(&self, tp: &Tty<G>, t: &Termios) -> Result<(), Errno> {
        match self {
            Driver::None => Ok(()),
            Driver::Dyn(d) => d.param(tp, t),
            Driver::Table(f) => f.param.map_or(Ok(()), |param| param(tp, t)),
        }
    }

    /// `tp->t_hwiflow && (*tp->t_hwiflow)(tp, block)`.
    fn hwiflow(&self, tp: &Tty<G>, block: bool) -> bool {
        match self {
            Driver::None => false,
            Driver::Dyn(d) => d.hwiflow(tp, block),
            Driver::Table(f) => f.hwiflow.is_some_and(|h| h(tp, i32::from(block)) != 0),
        }
    }

    /// The table an inherited driver's setter changes (a `Dyn` or no driver becomes one).
    fn table(&mut self) -> &mut FnTable<G> {
        if !matches!(self, Driver::Table(_)) {
            *self = Driver::Table(FnTable {
                oproc: None,
                param: None,
                hwiflow: None,
            });
        }
        match self {
            Driver::Table(t) => t,
            _ => kpanic(format_args!("tty: driver table")),
        }
    }
}

/// The members of `struct tty` that change together, under the tty's lock (`IPL_TTY`).
pub struct TtyState<G: ProcGlue> {
    /// `t_cc` (`t_termios.c_cc`): the control characters.
    cc: [Cc; NCCS],
    /// `t_winsize`: window size.
    winsize: Winsize,
    /// `t_pgrp`: foreground process group, counted; cleared by `pgdelete` ([`pgrp_gone`]).
    pgrp: Option<PgrpRef<G>>,
    /// `t_session`: enclosing session, counted (`SESSHOLD`).
    session: Option<SessionRef<G>>,
    /// `t_oproc`, `t_param`, `t_hwiflow`.
    driver: Driver<G>,
    /// `t_column`: tty output column.
    column: i16,
    /// `t_rocount`: characters echoed since the last output from elsewhere.
    rocount: i16,
    /// `t_rocol`: the column where they started.
    rocol: i16,
    /// `t_tv`: timestamp.
    tv: Timeval,
}

/// `struct tty`: per-tty structure, an item of `ttypl` (the glue's [`TtyTables`]).
///
/// Should be split in two, into device and tty drivers. Glue could be masks of what to echo
/// and circular buffer (low, high, timeout).
pub struct Tty<G: ProcGlue> {
    /// The item's record (`PoolArc::from_ref` of a `&Tty` a driver or a list lends).
    home: PoolHome<Tty<G>>,
    /// `tty_link`: link in global tty list, under `ttylist_lock`.
    pub tty_link: PinEntry<Tty<G>>,
    /// `t_rawq`: device raw input queue.
    pub t_rawq: Clist,
    /// `t_rawcc`: raw input queue statistics.
    pub t_rawcc: I64Word,
    /// `t_canq`: device canonical queue.
    pub t_canq: Clist,
    /// `t_cancc`: canonical queue statistics.
    pub t_cancc: I64Word,
    /// `t_outq`: device output queue.
    pub t_outq: Clist,
    /// `t_outcc`: output queue statistics.
    pub t_outcc: I64Word,
    /// `t_qlen`: length of above queues.
    pub t_qlen: I32Word,
    /// `t_line`: interface to device drivers (the line discipline's index).
    pub t_line: U8Word,
    /// `t_dev`: device.
    pub t_dev: I32Word,
    /// `t_state`: device and driver (`TS_*`) state.
    pub t_state: StateWord,
    /// `t_flags`: tty flags (the `TS_TSTAMP*` bits).
    pub t_flags: I32Word,
    /// `t_iflag` (`t_termios.c_iflag`).
    iflag: FlagWord,
    /// `t_oflag`.
    oflag: FlagWord,
    /// `t_cflag`.
    cflag: FlagWord,
    /// `t_lflag`.
    lflag: FlagWord,
    /// `t_ispeed`.
    ispeed: I32Word,
    /// `t_ospeed`.
    ospeed: I32Word,
    /// `t_rsel`: tty read/oob select.
    pub t_rsel: Selinfo<G>,
    /// `t_wsel`: tty write select.
    pub t_wsel: Selinfo<G>,
    /// `t_hiwat`: high water mark.
    pub t_hiwat: I16Word,
    /// `t_lowat`: low water mark.
    pub t_lowat: I16Word,
    /// `t_gen`: generation number.
    pub t_gen: I16Word,
    /// The state the C changes at `spltty()` in several words at once.
    st: Mutexed<TtyState<G>>,
    /// `t_rstrt_to`: restart timeout, holding the tty while it is set ([`ttymalloc`]).
    t_rstrt_to: Timeout,
}

hal::pool_home!(impl[G: ProcGlue] Tty<G>, home);

/// A counted reference to a tty.
pub type TtyRef<G> = PoolArc<Tty<G>>;

impl<G: ProcGlue> Tty<G> {
    /// A zeroed tty, as `malloc(M_ZERO)` returns it in `ttymalloc`.
    pub const fn new() -> Self {
        Self {
            home: PoolHome::new(),
            tty_link: PinEntry::new(),
            t_rawq: Clist::new(),
            t_rawcc: I64Word::new(0),
            t_canq: Clist::new(),
            t_cancc: I64Word::new(0),
            t_outq: Clist::new(),
            t_outcc: I64Word::new(0),
            t_qlen: I32Word::new(0),
            t_line: U8Word::new(0),
            t_dev: I32Word::new(0),
            t_state: StateWord::new(0),
            t_flags: I32Word::new(0),
            iflag: FlagWord::new(0),
            oflag: FlagWord::new(0),
            cflag: FlagWord::new(0),
            lflag: FlagWord::new(0),
            ispeed: I32Word::new(0),
            ospeed: I32Word::new(0),
            t_rsel: Selinfo::new(),
            t_wsel: Selinfo::new(),
            t_hiwat: I16Word::new(0),
            t_lowat: I16Word::new(0),
            t_gen: I16Word::new(0),
            st: Mutexed::new(
                IPL_TTY,
                TtyState {
                    cc: [0; NCCS],
                    winsize: Winsize {
                        ws_row: 0,
                        ws_col: 0,
                        ws_xpixel: 0,
                        ws_ypixel: 0,
                    },
                    pgrp: None,
                    session: None,
                    driver: Driver::None,
                    column: 0,
                    rocount: 0,
                    rocol: 0,
                    tv: Timeval::new(0, 0),
                },
            ),
            t_rstrt_to: Timeout::unset(),
        }
    }

    /// `ISSET(tp->t_state, bits)`.
    #[inline]
    pub fn t_state_isset(&self, bits: i32) -> bool {
        self.t_state.get() & bits != 0
    }

    /// `SET(tp->t_state, bits)`.
    #[inline]
    pub fn t_state_set(&self, bits: i32) {
        self.t_state.0.fetch_or(bits, Ordering::Relaxed);
    }

    /// `CLR(tp->t_state, bits)`.
    #[inline]
    pub fn t_state_clr(&self, bits: i32) {
        self.t_state.0.fetch_and(!bits, Ordering::Relaxed);
    }

    /// `t_cflag`.
    #[inline]
    pub fn t_cflag(&self) -> Tcflag {
        self.cflag.get()
    }

    /// `t_cflag = v`.
    pub fn set_t_cflag(&self, v: Tcflag) {
        self.cflag.set(v);
    }

    /// `t_iflag`.
    #[inline]
    pub fn t_iflag(&self) -> Tcflag {
        self.iflag.get()
    }

    /// `t_iflag = v`.
    pub fn set_t_iflag(&self, v: Tcflag) {
        self.iflag.set(v);
    }

    /// `t_lflag`.
    #[inline]
    pub fn t_lflag(&self) -> Tcflag {
        self.lflag.get()
    }

    /// `t_lflag = v`.
    pub fn set_t_lflag(&self, v: Tcflag) {
        self.lflag.set(v);
    }

    /// `SET(tp->t_lflag, bits)`.
    pub fn t_lflag_set(&self, bits: Tcflag) {
        self.lflag.or(bits);
    }

    /// `CLR(tp->t_lflag, bits)`.
    pub fn t_lflag_clr(&self, bits: Tcflag) {
        self.lflag.clr(bits);
    }

    /// `t_oflag`.
    #[inline]
    pub fn t_oflag(&self) -> Tcflag {
        self.oflag.get()
    }

    /// `t_oflag = v`.
    pub fn set_t_oflag(&self, v: Tcflag) {
        self.oflag.set(v);
    }

    /// `t_ispeed`.
    pub fn t_ispeed(&self) -> i32 {
        self.ispeed.get()
    }

    /// `t_ispeed = v`.
    pub fn set_t_ispeed(&self, v: i32) {
        self.ispeed.set(v);
    }

    /// `t_ospeed`.
    pub fn t_ospeed(&self) -> i32 {
        self.ospeed.get()
    }

    /// `t_ospeed = v`.
    pub fn set_t_ospeed(&self, v: i32) {
        self.ospeed.set(v);
    }

    /// `t_cc[i]`.
    pub fn t_cc(&self, i: u32) -> Cc {
        self.st.lock().cc[i as usize]
    }

    /// `t_cc`: the whole control character array.
    pub fn t_cc_all(&self) -> [Cc; NCCS] {
        self.st.lock().cc
    }

    /// `t_cc[..] = cc`.
    pub fn set_t_cc_all(&self, cc: [Cc; NCCS]) {
        self.st.lock().cc = cc;
    }

    /// `tp->t_termios`, whole (a driver's `t_param` of its own settings, `TIOCGETA`).
    pub fn termios(&self) -> Termios {
        let cc = self.t_cc_all();
        Termios {
            c_iflag: self.t_iflag(),
            c_oflag: self.t_oflag(),
            c_cflag: self.t_cflag(),
            c_lflag: self.t_lflag(),
            c_cc: cc,
            c_ispeed: self.t_ispeed(),
            c_ospeed: self.t_ospeed(),
        }
    }

    /// `tp->t_winsize`.
    pub fn winsize(&self) -> Winsize {
        self.st.lock().winsize
    }

    /// `tp->t_winsize = ws` (a driver that learns its size: viocon, wsdisplay).
    pub fn set_winsize(&self, ws: Winsize) {
        self.st.lock().winsize = ws;
    }

    /// `t_column`.
    pub fn column(&self) -> i16 {
        self.st.lock().column
    }

    /// `tp->t_pgrp`: the foreground group, counted.
    pub fn pgrp(&self) -> Option<PgrpRef<G>> {
        self.st.lock().pgrp.clone()
    }

    /// `tp->t_pgrp->pg_id`, 0 for none.
    pub fn pgrp_id(&self) -> Pid {
        self.st.lock().pgrp.as_ref().map_or(0, |pg| pg.pg_id.get())
    }

    /// `tp->t_session`: the session, counted.
    pub fn session(&self) -> Option<SessionRef<G>> {
        self.st.lock().session.clone()
    }

    /// `tp->t_session` as an address (compared, `kinfo_proc`'s `p_tsess` source), 0 for none.
    pub fn session_addr(&self) -> usize {
        self.st
            .lock()
            .session
            .as_ref()
            .map_or(0, SessionRef::<G>::addr)
    }

    /// `tp->t_oproc = f` (an inherited driver's open).
    pub fn set_oproc(&self, f: Option<TOproc<G>>) {
        self.st.lock().driver.table().oproc = f;
    }

    /// `tp->t_param = f`.
    pub fn set_param(&self, f: Option<TParam<G>>) {
        self.st.lock().driver.table().param = f;
    }

    /// `tp->t_hwiflow = f`.
    pub fn set_hwiflow(&self, f: Option<THwiflow<G>>) {
        self.st.lock().driver.table().hwiflow = f;
    }

    /// The tty's driver is `d` (the pty's master opening).
    pub fn set_driver(&self, d: &'static dyn TtyDriver<G>) {
        self.st.lock().driver = Driver::Dyn(d);
    }

    /// `tp->t_oproc = NULL`: no driver (the pty's master closing).
    pub fn clear_driver(&self) {
        self.st.lock().driver = Driver::None;
    }

    /// `tp->t_oproc != NULL`.
    pub fn has_driver(&self) -> bool {
        self.st.lock().driver.present()
    }

    /// `timeout_add(&tp->t_rstrt_to, ticks)`: `ttrstrt` in `ticks` ticks.
    pub fn restart_in(&self, ticks: i32) {
        let Some(h) = PoolArc::from_ref(self) else {
            kpanic(format_args!("tty: restart_in of a tty not from ttymalloc"));
        };
        let _ = hal::pin::project(h.as_pin(), |t| &t.t_rstrt_to).add(ticks);
    }

    /// `timeout_del(&tp->t_rstrt_to)`.
    pub fn restart_cancel(&self) {
        let _ = self.t_rstrt_to.del();
    }

    /// `ttyhog(tp)`: the input queue limit.
    #[inline]
    pub fn ttyhog(&self) -> i32 {
        self.t_qlen.get()
    }
}

impl<G: ProcGlue> Default for Tty<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The ttys of one glue: `ttypl`, `ttylist` under `ttylist_lock`, `tty_count`, `constty`,
/// the console's device and the device switch's bridge (the C's globals of `tty.c`,
/// `cons.c`'s `constty`).
pub struct TtyTables<G: ProcGlue> {
    /// `ttypl`: the ttys (recorded: a `&Tty` a driver lends can be counted again).
    pub ttypl: TypedPool<Tty<G>>,
    /// `ttylist_lock`.
    ttylist_lock: RawRwlock,
    /// `ttylist`: every allocated tty.
    ttylist: PinList<G::TtyLink>,
    /// `tty_count`: number of ttys in global ttylist.
    tty_count: AtomicI32,
    /// `constty`: the tty `TIOCCONS` made the console's (`cons.c`).
    constty: Mutexed<Option<TtyRef<G>>>,
    /// `constty != NULL`, written under `constty`'s lock, read without it.
    constty_set: core::sync::atomic::AtomicBool,
    /// The device switch's bridge ([`register_ttydevsw`]).
    devsw: OnceStatic<&'static dyn TtyDevsw<G>>,
}

impl<G: ProcGlue> TtyTables<G> {
    /// No tty yet; the pool is made at the first `ttymalloc`.
    pub const fn new() -> Self {
        Self {
            ttypl: TypedPool::new(),
            ttylist_lock: RawRwlock::new("ttylist"),
            ttylist: PinList::new(),
            tty_count: AtomicI32::new(0),
            constty: Mutexed::new(hal::intr::IPL_HIGH, None),
            constty_set: core::sync::atomic::AtomicBool::new(false),
            devsw: OnceStatic::new(),
        }
    }
}

impl<G: ProcGlue> Default for TtyTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The termios line discipline, slot 0 (`TTYDISC`).
pub struct TermiosDisc<G>(PhantomData<fn() -> G>);

/// A defunct (or not configured) line discipline: `enodev` for every entry but `ioctl`
/// (`nullioctl`) and `modem` (`nullmodem`).
pub struct Defunct<G>(PhantomData<fn() -> G>);

/// `linesw[]`: the line disciplines, indexed by `t_line`.
pub struct Discipline<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> Discipline<G> {
    /// The table: 0 termios; 1 defunct; 2 old NTTYDISC (defunct); 3 TABLDISC (defunct); 4
    /// SLIPDISC (defunct); 5 PPPDISC (`NPPP == 0`); 6 STRIPDISC (defunct); 7 NMEADISC
    /// (`NNMEA == 0`); 8 MSTSDISC (`NMSTS == 0`); 9 ENDRUNDISC (`NENDRUN == 0`).
    pub const TABLE: [&'static dyn LineDiscipline<G>; NLINESW] = [
        &TermiosDisc::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
        &Defunct::<G>(PhantomData),
    ];

    /// What `linesw(tp)` answers for an index past the table.
    const DEFUNCT: &'static dyn LineDiscipline<G> = &Defunct::<G>(PhantomData);
}

/// `ttyread_filtops`: `EVFILT_READ` on a tty.
pub struct TtyRead;
/// `ttywrite_filtops`: `EVFILT_WRITE` on a tty.
pub struct TtyWrite;
/// `ttyexcept_filtops`: `EVFILT_EXCEPT` (poll's hang-up) on a tty.
pub struct TtyExcept;

/// `kern.tty`: the `sysctl_tty` node.
pub struct TtyNode<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> TtyNode<G> {
    /// The node.
    pub const NODE: TtyNode<G> = TtyNode(PhantomData);
}

/// One section of the C at `spltty()`: the level raised for its life, the tty's state locked
/// and let go around every call that leaves the module ([`Sect::out`]).
struct Sect<'a, G: ProcGlue> {
    /// The tty.
    tp: &'a Tty<G>,
    /// Its state, locked (`None` only inside `out`).
    g: Option<MutexedGuard<'a, TtyState<G>>>,
    /// `spltty()` for the section (dropped after the guard).
    _spl: SplGuard,
}

impl<'a, G: ProcGlue> Sect<'a, G> {
    /// `s = spltty()`, and the state locked.
    fn new(tp: &'a Tty<G>) -> Self {
        let spl = hal::intr::splraise(IPL_TTY);
        Self {
            tp,
            g: Some(tp.st.lock()),
            _spl: spl,
        }
    }

    /// The state.
    fn st(&mut self) -> &mut TtyState<G> {
        match self.g.as_deref_mut() {
            Some(st) => st,
            None => kpanic(format_args!("tty: state used outside its lock")),
        }
    }

    /// `f` with the state's lock let go (the level stays raised), then taken again: a call
    /// that leaves the module.
    fn out<R>(&mut self, f: impl FnOnce() -> R) -> R {
        self.g = None;
        let r = f();
        self.g = Some(self.tp.st.lock());
        r
    }

    /// The driver, copied out of the state.
    fn driver(&mut self) -> Driver<G> {
        self.st().driver
    }

    /// `pgsignal(tp->t_pgrp, sig, 1)`: the group's handle is dropped outside the lock.
    fn pgsignal(&mut self, sig: i32) {
        let pg = self.st().pgrp.clone();
        self.out(move || {
            pgsignal(pg.as_deref(), sig, true);
            drop(pg);
        });
    }

    /// `d_stop(tp, rw)`: the device switch's stop of the tty's device.
    fn d_stop(&mut self, rw: i32) {
        let tp = self.tp;
        self.out(|| {
            if let Some(d) = devsw::<G>() {
                d.stop(tp, rw);
            }
        });
    }
}

/// `tk_cancc`: input characters in canonical mode.
pub static TK_CANCC: AtomicI64 = AtomicI64::new(0);
/// `tk_nin`: input characters.
pub static TK_NIN: AtomicI64 = AtomicI64::new(0);
/// `tk_nout`: output characters.
pub static TK_NOUT: AtomicI64 = AtomicI64::new(0);
/// `tk_rawcc`: input characters in raw mode.
pub static TK_RAWCC: AtomicI64 = AtomicI64::new(0);

/// `struct linesw` (`<sys/conf.h>`): a line discipline, the entries the drivers and the
/// device switch call through `linesw[tp->t_line]`.
pub trait LineDiscipline<G: ProcGlue>: Sync + 'static {
    /// `l_open`.
    fn open(&self, dev: Dev, tp: &Tty<G>, p: &Thread<G>) -> Result<(), Errno>;
    /// `l_close`.
    fn close(&self, tp: &Tty<G>, flag: i32, p: Option<&Thread<G>>) -> Result<(), Errno>;
    /// `l_read`.
    fn read(&self, tp: &Tty<G>, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno>;
    /// `l_write`.
    fn write(&self, tp: &Tty<G>, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno>;
    /// `l_ioctl`: `Ok(false)` for the C's `-1` (not this discipline's ioctl); both
    /// configured rows are `nullioctl`.
    fn ioctl(
        &self,
        tp: &Tty<G>,
        cmd: u64,
        data: &mut [u8],
        flag: i32,
        p: &Thread<G>,
    ) -> Result<bool, Errno> {
        nullioctl(tp, cmd, data, flag, p)
    }
    /// `l_rint`: one received character; the C's `int` (`ttyinput`'s 1 when a costly
    /// operation was reached, which `ptcwrite` waits a tick for; the defunct `ENODEV`).
    fn rint(&self, c: i32, tp: &Tty<G>) -> i32;
    /// `l_start`.
    fn start(&self, tp: &Tty<G>) -> Result<(), Errno>;
    /// `l_modem`: a carrier transition; `false` when the line should be turned off.
    fn modem(&self, tp: &Tty<G>, carrier: bool) -> bool;
}

/// The device driver's half of a tty: `t_oproc`, `t_param`, `t_hwiflow` of `struct tty`.
pub trait TtyDriver<G: ProcGlue>: Sync + 'static {
    /// `t_oproc`: start output.
    fn start(&self, tp: &Tty<G>);
    /// `t_param`: set hardware state (`NULL`: nothing to set).
    fn param(&self, _tp: &Tty<G>, _t: &Termios) -> Result<(), Errno> {
        Ok(())
    }
    /// `t_hwiflow`: block or unblock the remote end by hardware flow control; whether it took
    /// effect (`NULL`: no hardware flow control).
    fn hwiflow(&self, _tp: &Tty<G>, _block: bool) -> bool {
        false
    }
}

/// What the terminal needs of the device switch and the console, which stay `bsd`'s until N7
/// (`cdevsw`, `dev/cons.c`) and of the controlling terminal's vnode (N5): a bridge of
/// `bridges.toml`, implemented by `bsd` and registered once ([`register_ttydevsw`]).
pub trait TtyDevsw<G: ProcGlue>: Sync {
    /// `constty || cn_devvp`: the console is a terminal `cnwrite` reaches (a `TIOCCONS`
    /// redirection, or `/dev/console` opened).
    fn console_tty(&self) -> bool;
    /// `cnwrite(0, uio, 0)`.
    fn console_write(&self, uio: &mut crate::uio::Uio<'_, G>) -> Result<(), Errno>;
    /// `(*cdevsw[major(tp->t_dev)].d_stop)(tp, rw)`.
    fn stop(&self, tp: &Tty<G>, rw: i32);
    /// `TIOCCONS`'s check that `p` could open the real console: `namei("/dev/console")`
    /// with `PLEDGE_RPATH|PLEDGE_WPATH` and `UNVEIL_READ|UNVEIL_WRITE`, then
    /// `VOP_ACCESS(vp, VREAD)`.
    fn console_access(&self, p: &Thread<G>) -> Result<(), Errno>;
    /// `cn_tab->cn_dev`, `None` without a console (`ttwrite`'s console buffer copy).
    fn console_dev(&self) -> Option<Dev>;
}

/// The glue's tty tables.
fn tables<G: ProcGlue>() -> &'static TtyTables<G> {
    &G::tables().tty
}

/// The registered device switch's bridge, if any yet.
fn devsw<G: ProcGlue>() -> Option<&'static dyn TtyDevsw<G>> {
    tables::<G>().devsw.get().copied()
}

/// Registers the device switch's and the console's bridge ([`TtyDevsw`]), once (a second
/// registration is ignored).
pub fn register_ttydevsw<G: ProcGlue>(d: &'static dyn TtyDevsw<G>) {
    let _ = tables::<G>().devsw.set(d);
}

/// `constty || cn_devvp`, `cnwrite`: the console's half, for `kernel::log`.
pub fn console_bridge<G: ProcGlue>() -> Option<&'static dyn TtyDevsw<G>> {
    devsw::<G>()
}

/// `constty`: the tty `TIOCCONS` made the console's, counted.
///
/// `printf(9)` asks for every console character: the common answer (no redirection) is read
/// from a flag without the lock, and after a panic or inside ddb a held lock answers `None`
/// (the console's own output), as `kernel::log`'s `log_mtx` does.
pub fn constty<G: ProcGlue>() -> Option<TtyRef<G>> {
    let t = tables::<G>();
    if !t.constty_set.load(Ordering::Acquire) {
        return None;
    }
    if hal::cpu::past_caring() && t.constty.is_locked() {
        return None;
    }
    t.constty.lock().clone()
}

/// `constty = tp` (`TIOCCONS`; `None`: `constty = NULL`). After a panic or inside ddb with
/// the lock held elsewhere, `NULL` is the flag alone (the handle stays where it is).
pub fn set_constty<G: ProcGlue>(tp: Option<TtyRef<G>>) {
    let t = tables::<G>();
    if hal::cpu::past_caring() && t.constty.is_locked() {
        if tp.is_none() {
            t.constty_set.store(false, Ordering::Release);
        }
        return;
    }
    let old = {
        let mut c = t.constty.lock();
        t.constty_set.store(tp.is_some(), Ordering::Release);
        core::mem::replace(&mut *c, tp)
    };
    drop(old);
}

/// `if (constty == tp) constty = NULL`.
fn constty_forget<G: ProcGlue>(tp: &Tty<G>) {
    let old = {
        let mut c = tables::<G>().constty.lock();
        if c.as_deref().is_some_and(|c| ptr::eq(c, tp)) {
            tables::<G>().constty_set.store(false, Ordering::Release);
            c.take()
        } else {
            None
        }
    };
    drop(old);
}

/// `cn_tab->cn_dev` (`ttwrite` copies what is written to that device into the console
/// buffer), `None` without a console.
pub fn console_dev<G: ProcGlue>() -> Option<Dev> {
    devsw::<G>().and_then(TtyDevsw::console_dev)
}

/// `isctty(pr, tp)`: is `tp` the controlling terminal of `pr`? Under the kernel lock (the
/// process's session is `[K]`).
pub fn isctty<G: ProcGlue>(pr: &Process<G>, tp: &Tty<G>) -> bool {
    let sess = {
        let r = klock::reader();
        pr.session(&r).map_or(0, |s| ptr::from_ref(s).addr())
    };
    pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT != 0 && sess != 0 && sess == tp.session_addr()
}

/// `isbackground(pr, tp)`: is `pr` in the background of `tp`?
pub fn isbackground<G: ProcGlue>(pr: &Process<G>, tp: &Tty<G>) -> bool {
    if !isctty(pr, tp) {
        return false;
    }
    let pg = {
        let r = klock::reader();
        pr.pgrp(&r).map_or(0, PgrpRef::<G>::addr)
    };
    let tpg = tp.st.lock().pgrp.as_ref().map_or(0, PgrpRef::<G>::addr);
    pg != tpg
}

/// `CCEQ(val, c)` on an `int` character: `c` equals the enabled control character `val`.
fn cceq_c(val: Cc, c: i32) -> bool {
    c == i32::from(val) && val != _POSIX_VDISABLE
}

/// `ISALPHA(c)`.
fn isalpha(c: i32) -> u8 {
    CHAR_TYPE[(c & TTY_CHARMASK) as usize] & ALPHA
}

/// `CCLASS(c)`.
fn cclass(c: i32) -> u8 {
    CHAR_TYPE[(c & 0xff) as usize] & CCLASSMASK
}

/// `islower(c)`.
fn islower(c: i32) -> bool {
    (i32::from(b'a')..=i32::from(b'z')).contains(&c)
}

/// `isupper(c)`.
fn isupper(c: i32) -> bool {
    (i32::from(b'A')..=i32::from(b'Z')).contains(&c)
}

/// `tolower(c)`.
fn tolower(c: i32) -> i32 {
    c - i32::from(b'A') + i32::from(b'a')
}

/// `toupper(c)`.
fn toupper(c: i32) -> i32 {
    c - i32::from(b'a') + i32::from(b'A')
}

/// `ttyopen`: initial open of tty, or (re)entry to standard tty line discipline.
pub fn ttyopen<G: ProcGlue>(device: Dev, tp: &Tty<G>, _p: &Thread<G>) -> Result<(), Errno> {
    let mut s = Sect::new(tp);
    tp.t_dev.set(device);
    if !tp.t_state_isset(TS_ISOPEN) {
        tp.t_state_set(TS_ISOPEN);
        let st = s.st();
        st.winsize = Winsize::default();
        st.column = 0;
    }
    tp.t_state_clr(TS_WOPEN);
    Ok(())
}

/// `ttyclose`: handle close() on a tty line: flush and set to initial state, bumping
/// generation number so that pending read/write calls can detect recycling of the tty.
pub fn ttyclose<G: ProcGlue>(tp: &Tty<G>) -> Result<(), Errno> {
    constty_forget(tp);

    let (pgrp, sess) = {
        let mut s = Sect::new(tp);
        ttyflush_s(&mut s, FREAD | FWRITE);
        tp.t_gen.set(tp.t_gen.get().wrapping_add(1));
        let st = s.st();
        let pgrp = st.pgrp.take();
        // SESSRELE(tp->t_session): the tty's reference goes (dropped below, unlocked).
        let sess = st.session.take();
        tp.t_state.set(0);
        (pgrp, sess)
    };
    drop(pgrp);
    drop(sess);
    Ok(())
}

/// `FLUSHQ(q)`.
fn flushq(q: &Clist) {
    let n = q.c_cc.get();
    if n != 0 {
        q.ndflush(n);
    }
}

/// `TTBREAKC(c, lflag)`: is `c` a line delimiter ("break" character)?
fn ttbreakc(c: i32, cc: &[Cc; NCCS], lflag: u32) -> bool {
    c == i32::from(b'\n')
        || ((c == i32::from(cc[VEOF as usize])
            || c == i32::from(cc[VEOL as usize])
            || (c == i32::from(cc[VEOL2 as usize]) && lflag & IEXTEN != 0))
            && c != i32::from(_POSIX_VDISABLE))
}

/// Where `ttyinput`'s special-character switch ends up.
enum Input {
    /// `goto endcase`.
    Endcase,
    /// `goto restartoutput`.
    Restartoutput,
    /// `goto startoutput`.
    Startoutput,
    /// `return (r)`.
    Return(i32),
}

/// `parmrk:` in `ttyinput`: the `\377 \0 c` marking of a bad character.
fn parmrk<G: ProcGlue>(tp: &Tty<G>, c: i32, iflag: u32) {
    let _ = tp.t_rawq.putc(0o377 | TTY_QUOTE);
    if iflag & ISTRIP != 0 || c != 0o377 {
        let _ = tp.t_rawq.putc(TTY_QUOTE);
    }
    let _ = tp.t_rawq.putc(c | TTY_QUOTE);
}

/// `ttyinput`: process input of a single character received on a tty. Returns 0 normally,
/// 1 if a costly operation was reached.
pub fn ttyinput<G: ProcGlue>(c: i32, tp: &Tty<G>) -> i32 {
    hal::random::enqueue_randomness(((tp.t_dev.get() << 8) | c) as u32);
    // If receiver is not enabled, drop it.
    if tp.t_cflag() & CREAD == 0 {
        return 0;
    }
    let mut s = Sect::new(tp);
    ttyinput_s(c, &mut s)
}

/// [`ttyinput`] inside a section, past the randomness and the `CREAD` test.
fn ttyinput_s<G: ProcGlue>(c: i32, s: &mut Sect<'_, G>) -> i32 {
    let tp = s.tp;
    let mut c = c;
    let mut ret = 0;

    // If input is pending take it first.
    let lflag = tp.t_lflag();
    if lflag & PENDIN != 0 {
        ttypend_s(s);
    }
    // Gather stats.
    if lflag & ICANON != 0 {
        TK_CANCC.fetch_add(1, Ordering::Relaxed);
        tp.t_cancc.set(tp.t_cancc.get() + 1);
    } else {
        TK_RAWCC.fetch_add(1, Ordering::Relaxed);
        tp.t_rawcc.set(tp.t_rawcc.get() + 1);
    }
    TK_NIN.fetch_add(1, Ordering::Relaxed);

    let cc = s.st().cc;
    let iflag = tp.t_iflag();
    let exit = 'switch: {
        // Handle exceptional conditions (break, parity, framing).
        let error = c & TTY_ERRORMASK;
        if error != 0 {
            c &= !TTY_ERRORMASK;
            if error & TTY_FE != 0 && c == 0 {
                // Break.
                if iflag & IGNBRK != 0 {
                    break 'switch Input::Return(0);
                }
                ttyflush_s(s, FREAD | FWRITE);
                if iflag & BRKINT != 0 {
                    s.pgsignal(SIGINT);
                    break 'switch Input::Endcase;
                } else if iflag & PARMRK != 0 {
                    parmrk(tp, c, iflag);
                    break 'switch Input::Endcase;
                }
            } else if (error & TTY_PE != 0 && iflag & INPCK != 0) || error & TTY_FE != 0 {
                if iflag & IGNPAR != 0 {
                    break 'switch Input::Endcase;
                } else if iflag & PARMRK != 0 {
                    parmrk(tp, c, iflag);
                    break 'switch Input::Endcase;
                }
                c = 0;
            }
        }
        if c == 0o377 && iflag & ISTRIP == 0 && iflag & PARMRK != 0 {
            parmrk(tp, c, iflag);
            break 'switch Input::Endcase;
        }

        // In tandem mode, check high water mark.
        if iflag & IXOFF != 0 || tp.t_cflag() & CHWFLOW != 0 {
            ttyblock_s(s);
        }
        if !tp.t_state_isset(TS_TYPEN) && iflag & ISTRIP != 0 {
            c &= !0x80;
        }
        if lflag & EXTPROC == 0 {
            // Check for literal nexting very first.
            if tp.t_state_isset(TS_LNCH) {
                c |= TTY_QUOTE;
                tp.t_state_clr(TS_LNCH);
            }
            // Scan for special characters. This code is really just a big case statement
            // with non-constant cases. The bottom of the case statement is labeled
            // ``endcase'', so goto it after a case match, or similar.

            // Control chars which aren't controlled by ICANON, ISIG, or IXON.
            if lflag & IEXTEN != 0 {
                if cceq_c(cc[VLNEXT as usize], c) {
                    if lflag & ECHO != 0 {
                        if lflag & ECHOE != 0 {
                            let _ = ttyoutput_s(i32::from(b'^'), s);
                            let _ = ttyoutput_s(i32::from(b'\x08'), s);
                        } else {
                            ttyecho_s(c, s);
                        }
                    }
                    tp.t_state_set(TS_LNCH);
                    break 'switch Input::Endcase;
                }
                if cceq_c(cc[VDISCARD as usize], c) {
                    if lflag & FLUSHO != 0 {
                        tp.t_lflag_clr(FLUSHO);
                    } else {
                        ttyflush_s(s, FWRITE);
                        ttyecho_s(c, s);
                        if tp.t_rawq.c_cc.get() + tp.t_canq.c_cc.get() != 0 {
                            ret = ttyretype_s(s);
                        }
                        tp.t_lflag_set(FLUSHO);
                    }
                    break 'switch Input::Startoutput;
                }
            }
            // Signals.
            if lflag & ISIG != 0 {
                if cceq_c(cc[VINTR as usize], c) || cceq_c(cc[VQUIT as usize], c) {
                    if lflag & NOFLSH == 0 {
                        ttyflush_s(s, FREAD | FWRITE);
                    }
                    ttyecho_s(c, s);
                    s.pgsignal(if cceq_c(cc[VINTR as usize], c) {
                        SIGINT
                    } else {
                        SIGQUIT
                    });
                    break 'switch Input::Endcase;
                }
                if cceq_c(cc[VSUSP as usize], c) {
                    if lflag & NOFLSH == 0 {
                        ttyflush_s(s, FREAD);
                    }
                    ttyecho_s(c, s);
                    s.pgsignal(SIGTSTP);
                    break 'switch Input::Endcase;
                }
            }
            // Handle start/stop characters.
            if iflag & IXON != 0 {
                if cceq_c(cc[VSTOP as usize], c) {
                    if !tp.t_state_isset(TS_TTSTOP) {
                        tp.t_state_set(TS_TTSTOP);
                        s.d_stop(0);
                        break 'switch Input::Return(0);
                    }
                    if !cceq_c(cc[VSTART as usize], c) {
                        break 'switch Input::Return(0);
                    }
                    // if VSTART == VSTOP then toggle
                    break 'switch Input::Endcase;
                }
                if cceq_c(cc[VSTART as usize], c) {
                    break 'switch Input::Restartoutput;
                }
            }
            // IGNCR, ICRNL, & INLCR
            if c == i32::from(b'\r') {
                if iflag & IGNCR != 0 {
                    break 'switch Input::Endcase;
                } else if iflag & ICRNL != 0 {
                    c = i32::from(b'\n');
                }
            } else if c == i32::from(b'\n') && iflag & INLCR != 0 {
                c = i32::from(b'\r');
            }
        }
        if tp.t_lflag() & EXTPROC == 0 && lflag & ICANON != 0 {
            // From here on down canonical mode character processing takes place.

            // upper case or specials with IUCLC and XCASE
            if lflag & XCASE != 0 && iflag & IUCLC != 0 {
                if tp.t_state_isset(TS_BKSL) {
                    tp.t_state_clr(TS_BKSL);
                    c = match u8::try_from(c) {
                        Ok(b'\'') => i32::from(b'`'),
                        Ok(b'!') => i32::from(b'|'),
                        Ok(b'^') => i32::from(b'~'),
                        Ok(b'(') => i32::from(b'{'),
                        Ok(b')') => i32::from(b'}'),
                        _ => c,
                    };
                } else if c == i32::from(b'\\') {
                    tp.t_state_set(TS_BKSL);
                    break 'switch Input::Endcase;
                } else if isupper(c) {
                    c = tolower(c);
                }
            } else if iflag & IUCLC != 0 && isupper(c) {
                c = tolower(c);
            }
            // erase (^H / ^?)
            if cceq_c(cc[VERASE as usize], c) {
                if tp.t_rawq.c_cc.get() != 0 {
                    let r = tp.t_rawq.unputc();
                    ret = ttyrub_s(r, s);
                }
                break 'switch Input::Endcase;
            }
            // kill (^U)
            if cceq_c(cc[VKILL as usize], c) {
                if lflag & ECHOKE != 0
                    && tp.t_rawq.c_cc.get() == i32::from(s.st().rocount)
                    && lflag & ECHOPRT == 0
                {
                    while tp.t_rawq.c_cc.get() != 0 {
                        let r = tp.t_rawq.unputc();
                        if ttyrub_s(r, s) != 0 {
                            ret = 1;
                        }
                    }
                } else {
                    ttyecho_s(c, s);
                    if lflag & ECHOK != 0 || lflag & ECHOKE != 0 {
                        ttyecho_s(i32::from(b'\n'), s);
                    }
                    flushq(&tp.t_rawq);
                    s.st().rocount = 0;
                }
                tp.t_state_clr(TS_LOCAL);
                break 'switch Input::Endcase;
            }
            // word erase (^W)
            if cceq_c(cc[VWERASE as usize], c) && lflag & IEXTEN != 0 {
                let alt = lflag & ALTWERASE != 0;

                // erase whitespace
                loop {
                    c = tp.t_rawq.unputc();
                    if c != i32::from(b' ') && c != i32::from(b'\t') {
                        break;
                    }
                    if ttyrub_s(c, s) != 0 {
                        ret = 1;
                    }
                }
                if c == -1 {
                    break 'switch Input::Endcase;
                }
                // erase last char of word and remember the next chars type (for ALTWERASE)
                if ttyrub_s(c, s) != 0 {
                    ret = 1;
                }
                c = tp.t_rawq.unputc();
                if c == -1 {
                    break 'switch Input::Endcase;
                }
                if c == i32::from(b' ') || c == i32::from(b'\t') {
                    let _ = tp.t_rawq.putc(c);
                    break 'switch Input::Endcase;
                }
                let ctype = isalpha(c);
                // erase rest of word
                loop {
                    if ttyrub_s(c, s) != 0 {
                        ret = 1;
                    }
                    c = tp.t_rawq.unputc();
                    if c == -1 {
                        break 'switch Input::Endcase;
                    }
                    if c == i32::from(b' ') || c == i32::from(b'\t') || (alt && isalpha(c) != ctype)
                    {
                        break;
                    }
                }
                let _ = tp.t_rawq.putc(c);
                break 'switch Input::Endcase;
            }
            // reprint line (^R)
            if cceq_c(cc[VREPRINT as usize], c) && lflag & IEXTEN != 0 {
                ret = ttyretype_s(s);
                break 'switch Input::Endcase;
            }
            // ^T - kernel info and generate SIGINFO
            if cceq_c(cc[VSTATUS as usize], c) && lflag & IEXTEN != 0 {
                if lflag & ISIG != 0 {
                    s.pgsignal(SIGINFO);
                }
                if lflag & NOKERNINFO == 0 {
                    s.out(|| ttyinfo(tp));
                }
                break 'switch Input::Endcase;
            }
        }
        // Check for input buffer overflow
        if tp.t_rawq.c_cc.get() + tp.t_canq.c_cc.get() >= tp.ttyhog() {
            if iflag & IMAXBEL != 0 {
                if tp.t_outq.c_cc.get() < i32::from(tp.t_hiwat.get()) {
                    let _ = ttyoutput_s(i32::from(ctrl(b'g')), s);
                }
            } else {
                ttyflush_s(s, FREAD | FWRITE);
            }
            break 'switch Input::Endcase;
        }
        // Put data char in q for user and wakeup on seeing a line delimiter.
        if tp.t_rawq.putc(c) >= 0 {
            if lflag & ICANON == 0 {
                ttwakeup_s(s);
                ttyecho_s(c, s);
                break 'switch Input::Endcase;
            }
            if ttbreakc(c, &cc, lflag) {
                s.st().rocount = 0;
                tp.t_rawq.catq(&tp.t_canq);
                ttwakeup_s(s);
            } else {
                let st = s.st();
                let rocount = st.rocount;
                st.rocount = rocount.wrapping_add(1);
                if rocount == 0 {
                    st.rocol = st.column;
                }
            }
            if tp.t_state_isset(TS_ERASE) {
                // end of prterase \.../
                tp.t_state_clr(TS_ERASE);
                let _ = ttyoutput_s(i32::from(b'/'), s);
            }
            let mut i = i32::from(s.st().column);
            ttyecho_s(c, s);
            if cceq_c(cc[VEOF as usize], c) && lflag & ECHO != 0 {
                // Place the cursor over the '^' of the ^D.
                i = 2.min(i32::from(s.st().column) - i);
                while i > 0 {
                    let _ = ttyoutput_s(i32::from(b'\x08'), s);
                    i -= 1;
                }
            }
        }
        Input::Endcase
    };

    match exit {
        Input::Return(r) => return r,
        Input::Endcase => {
            // IXANY means allow any character to restart output.
            if tp.t_state_isset(TS_TTSTOP)
                && iflag & IXANY == 0
                && cc[VSTART as usize] != cc[VSTOP as usize]
            {
                return ret;
            }
            tp.t_lflag_clr(FLUSHO);
            tp.t_state_clr(TS_TTSTOP);
        }
        Input::Restartoutput => {
            tp.t_lflag_clr(FLUSHO);
            tp.t_state_clr(TS_TTSTOP);
        }
        Input::Startoutput => {}
    }
    ttstart_s(s);
    ret
}

/// `ttyoutput`: output a single character on a tty, doing output processing as needed
/// (expanding tabs, newline processing, etc.). Returns < 0 if succeeds, otherwise returns
/// char to resend. Must be recursive.
pub fn ttyoutput<G: ProcGlue>(c: i32, tp: &Tty<G>) -> i32 {
    let mut s = Sect::new(tp);
    ttyoutput_s(c, &mut s)
}

/// [`ttyoutput`] inside a section.
#[allow(
    clippy::match_same_arms,
    reason = "the C keeps the cases apart; OpenBSD's shape is kept so it can be grepped"
)]
fn ttyoutput_s<G: ProcGlue>(c: i32, s: &mut Sect<'_, G>) -> i32 {
    let tp = s.tp;
    let mut c = c;
    let oflag = tp.t_oflag();
    if oflag & OPOST == 0 {
        TK_NOUT.fetch_add(1, Ordering::Relaxed);
        tp.t_outcc.set(tp.t_outcc.get() + 1);
        if tp.t_lflag() & FLUSHO == 0 && tp.t_outq.putc(c) != 0 {
            return c;
        }
        return -1;
    }
    // Do tab expansion if OXTABS is set. Special case if we external processing, we don't
    // do the tab expansion because we'll probably get it wrong. If tab expansion needs to
    // be done, let it happen externally.
    c &= TTY_CHARMASK;
    if c == i32::from(b'\t') && oflag & OXTABS != 0 && tp.t_lflag() & EXTPROC == 0 {
        let mut c = 8 - (i32::from(s.st().column) & 7);
        let notout = if tp.t_lflag() & FLUSHO != 0 {
            0
        } else {
            let notout = tp.t_outq.b_to_q(&b"        "[..c as usize]) as i32;
            c -= notout;
            TK_NOUT.fetch_add(i64::from(c), Ordering::Relaxed);
            tp.t_outcc.set(tp.t_outcc.get() + i64::from(c));
            notout
        };
        let st = s.st();
        st.column = st.column.wrapping_add(c as i16);
        return if notout != 0 { i32::from(b'\t') } else { -1 };
    }
    if c == i32::from(CEOT) && oflag & ONOEOT != 0 {
        return -1;
    }

    // Newline translation: if ONLCR is set, translate newline into "\r\n". If OCRNL is set,
    // translate '\r' into '\n'.
    if c == i32::from(b'\n') && oflag & ONLCR != 0 && tp.t_lflag() & FLUSHO == 0 {
        // The common case in one hold of the queue's lock: nothing the C does between its
        // two putc's applies to a newline (OCRNL, OLCUC, XCASE, ONOCR test other
        // characters), so the two characters are queued together, counted as the C counts
        // them.
        TK_NOUT.fetch_add(2, Ordering::Relaxed);
        tp.t_outcc.set(tp.t_outcc.get() + 2);
        let (r, n) = tp.t_outq.putc2(i32::from(b'\r'), c);
        if r != 0 {
            // The C counted the newline only once it got past the '\r'.
            TK_NOUT.fetch_sub(1, Ordering::Relaxed);
            tp.t_outcc.set(tp.t_outcc.get() - 1);
            return c;
        }
        let st = s.st();
        st.column = 0;
        if n != 0 {
            return c;
        }
        // NEWLINE's class: the column is 0 already (ONLRET, OCRNL or not).
        return -1;
    }
    if c == i32::from(b'\n') && tp.t_oflag() & ONLCR != 0 {
        TK_NOUT.fetch_add(1, Ordering::Relaxed);
        tp.t_outcc.set(tp.t_outcc.get() + 1);
        if tp.t_lflag() & FLUSHO == 0 && tp.t_outq.putc(i32::from(b'\r')) != 0 {
            return c;
        }
        s.st().column = 0;
    } else if c == i32::from(b'\r') && tp.t_oflag() & OCRNL != 0 {
        c = i32::from(b'\n');
    }

    if tp.t_oflag() & OLCUC != 0 && islower(c) {
        c = toupper(c);
    } else if tp.t_oflag() & OLCUC != 0 && tp.t_lflag() & XCASE != 0 {
        let c2 = match u8::try_from(c) {
            Ok(b'`') => i32::from(b'\''),
            Ok(b'|') => i32::from(b'!'),
            Ok(b'~') => i32::from(b'^'),
            Ok(b'{') => i32::from(b'('),
            Ok(b'}') => i32::from(b')'),
            _ => c,
        };
        if c == i32::from(b'\\') || isupper(c) || c != c2 {
            TK_NOUT.fetch_add(1, Ordering::Relaxed);
            tp.t_outcc.set(tp.t_outcc.get() + 1);
            if tp.t_outq.putc(i32::from(b'\\')) != 0 {
                return c;
            }
            c = c2;
        }
    }
    if tp.t_oflag() & ONOCR != 0 && c == i32::from(b'\r') && s.st().column == 0 {
        return -1;
    }

    TK_NOUT.fetch_add(1, Ordering::Relaxed);
    tp.t_outcc.set(tp.t_outcc.get() + 1);
    if tp.t_lflag() & FLUSHO == 0 && tp.t_outq.putc(c) != 0 {
        return c;
    }

    let st = s.st();
    let mut col = st.column;
    match cclass(c) {
        BACKSPACE => {
            if col > 0 {
                col -= 1;
            }
        }
        CONTROL => {}
        NEWLINE => {
            if tp.t_oflag() & ONLRET != 0 || tp.t_oflag() & OCRNL != 0 {
                col = 0;
            }
        }
        RETURN => col = 0,
        ORDINARY => col = col.wrapping_add(1),
        TAB => col = col.wrapping_add(8) & !7,
        _ => {}
    }
    st.column = col;
    -1
}

/// The `int` an ioctl's argument holds.
fn int_arg(data: &[u8]) -> Result<i32, Errno> {
    pod_from_bytes::<i32>(data).ok_or(Errno::EINVAL)
}

/// Returns an `int` through an ioctl's argument.
fn int_ret(data: &mut [u8], v: i32) -> Result<(), Errno> {
    if pod_to_bytes(&v, data) {
        Ok(())
    } else {
        Err(Errno::EINVAL)
    }
}

/// Returns a structure through an ioctl's argument.
fn pod_ret<T: hal::user::AbiPod>(data: &mut [u8], v: &T) -> Result<(), Errno> {
    if pod_to_bytes(v, data) {
        Ok(())
    } else {
        Err(Errno::EINVAL)
    }
}

/// `pr->ps_pgrp->pg_jobc == 0` (a process with no group counts as orphaned).
fn pgrp_orphaned<G: ProcGlue>(pr: &Process<G>) -> bool {
    let r = klock::reader();
    pr.pgrp(&r).is_none_or(|pg| pg.pg_jobc.get() == 0)
}

/// `pgsignal(pr->ps_pgrp, sig, 1)` of the caller's own group.
fn pgsignal_own<G: ProcGlue>(pr: &Process<G>, sig: i32) {
    let pg = {
        let r = klock::reader();
        pr.pgrp(&r).cloned()
    };
    pgsignal(pg.as_deref(), sig, true);
}

/// `ttioctl`: ioctls for all tty devices. Called after line-discipline specific ioctl has
/// been called to do discipline-specific functions and/or reject any of these ioctl
/// commands. `Ok(false)` where the C returns `-1`: not a tty ioctl.
pub fn ttioctl<G: ProcGlue>(
    tp: &Tty<G>,
    cmd: u64,
    data: &mut [u8],
    flag: i32,
    p: &Thread<G>,
) -> Result<bool, Errno> {
    let pr = p.process();

    // If the ioctl involves modification, hang if in the background.
    if matches!(
        cmd,
        FIOSETOWN
            | TIOCFLUSH
            | TIOCDRAIN
            | TIOCSBRK
            | TIOCCBRK
            | TIOCSETA
            | TIOCSETD
            | TIOCSETAF
            | TIOCSETAW
            | TIOCSPGRP
            | TIOCSTAT
            | TIOCSWINSZ
    ) {
        while isbackground(pr, tp)
            && pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT == 0
            && !sigismasked(p, SIGTTOU)
        {
            // A process in the background of a tty is in a process group.
            if pgrp_orphaned(pr) {
                return Err(Errno::EIO);
            }
            pgsignal_own(pr, SIGTTOU);
            ttysleep_nsec(
                tp,
                ptr::from_ref(&NOWAKE).cast(),
                TTOPRI | PCATCH,
                TTYBG,
                SEC_NSEC,
            )?;
        }
    }

    match cmd {
        // Process the ioctl.
        FIOASYNC => {
            // set/clear async i/o
            if int_arg(data)? != 0 {
                tp.t_state_set(TS_ASYNC);
            } else {
                tp.t_state_clr(TS_ASYNC);
            }
        }
        FIONREAD => {
            // get # bytes to read
            let n = {
                let mut s = Sect::new(tp);
                ttnread_s(&mut s)
            };
            int_ret(data, n)?;
        }
        TIOCEXCL => {
            // set exclusive use of tty
            tp.t_state_set(TS_XCLUDE);
        }
        TIOCFLUSH => {
            // flush buffers
            let mut flags = int_arg(data)?;
            if flags == 0 {
                flags = FREAD | FWRITE;
            } else {
                flags &= FREAD | FWRITE;
            }
            ttyflush(tp, flags);
        }
        TIOCCONS => {
            // become virtual console
            if int_arg(data)? != 0 {
                if let Some(cons) = constty::<G>()
                    && !ptr::eq::<Tty<G>>(&raw const *cons, tp)
                    && cons.t_state.get() & (TS_CARR_ON | TS_ISOPEN) == (TS_CARR_ON | TS_ISOPEN)
                {
                    return Err(Errno::EBUSY);
                }

                // ensure user can open the real console
                match devsw::<G>() {
                    Some(d) => d.console_access(p)?,
                    None => return Err(Errno::ENXIO),
                }

                set_constty(Some(tty_ref(tp)));
            } else {
                constty_forget(tp);
            }
        }
        TIOCDRAIN => {
            // wait till output drained
            ttywait(tp)?;
        }
        TIOCGETA => {
            // get termios struct
            pod_ret(data, &tp.termios())?;
        }
        TIOCGETD => {
            // get line discipline
            int_ret(data, i32::from(tp.t_line.get()))?;
        }
        TIOCGWINSZ => {
            // get window size
            pod_ret(data, &tp.winsize())?;
        }
        TIOCGTSTAMP => {
            let tv = tp.st.lock().tv;
            pod_ret(data, &TstampTimeval::from(tv))?;
        }
        FIOGETOWN => {
            // get pgrp of tty
            if !isctty(pr, tp) && crate::proc::suser(p).is_err() {
                return Err(Errno::ENOTTY);
            }
            let pgid = tp.st.lock().pgrp.as_ref().map_or(0, |pg| -pg.pg_id.get());
            int_ret(data, pgid)?;
        }
        TIOCGPGRP => {
            // get pgrp of tty
            if !isctty(pr, tp) && crate::proc::suser(p).is_err() {
                return Err(Errno::ENOTTY);
            }
            let pgid = tp
                .st
                .lock()
                .pgrp
                .as_ref()
                .map_or(NO_PID, |pg| pg.pg_id.get());
            int_ret(data, pgid)?;
        }
        TIOCGSID => {
            // get sid of tty
            if !isctty(pr, tp) {
                return Err(Errno::ENOTTY);
            }
            let leader = tp.st.lock().session.as_ref().and_then(|s| s.leader_pid());
            let Some(leader) = leader else {
                // XXX session stored wrong
                return Err(Errno::ENOTTY);
            };
            int_ret(data, leader)?;
        }
        TIOCNXCL => {
            // reset exclusive use of tty
            tp.t_state_clr(TS_XCLUDE);
        }
        TIOCOUTQ => {
            // output queue size
            int_ret(data, tp.t_outq.c_cc.get())?;
        }
        TIOCSETA | TIOCSETAW | TIOCSETAF => {
            // set termios struct / drain output, set / drn out, fls in, set
            let mut t: Termios = pod_from_bytes(data).ok_or(Errno::EINVAL)?;
            ttsetattr(tp, cmd, &mut t)?;
        }
        TIOCSETD => {
            // set line discipline
            let t = int_arg(data)?;
            let device = tp.t_dev.get();

            let Some(new) = usize::try_from(t)
                .ok()
                .and_then(|i| Discipline::<G>::TABLE.get(i))
            else {
                return Err(Errno::ENXIO);
            };
            if t != i32::from(tp.t_line.get()) {
                let _s = hal::intr::splraise(IPL_TTY);
                let _ = linesw(tp).close(tp, flag, Some(p));
                if let Err(e) = new.open(device, tp, p) {
                    let _ = linesw(tp).open(device, tp, p);
                    return Err(e);
                }
                tp.t_line.set(t as u8);
            }
        }
        TIOCSTART => {
            // start output, like ^Q
            let mut s = Sect::new(tp);
            if tp.t_state_isset(TS_TTSTOP) || tp.t_lflag() & FLUSHO != 0 {
                tp.t_lflag_clr(FLUSHO);
                tp.t_state_clr(TS_TTSTOP);
                ttstart_s(&mut s);
            }
        }
        TIOCSTOP => {
            // stop output, like ^S
            let mut s = Sect::new(tp);
            if !tp.t_state_isset(TS_TTSTOP) {
                tp.t_state_set(TS_TTSTOP);
                s.d_stop(0);
            }
        }
        TIOCSCTTY => {
            // become controlling tty
            // Session ctty vnode pointer set in vnode layer.
            let sess = {
                let r = klock::reader();
                match pr.pgrp(&r).and_then(|pg| pg.pg_session.get(&r).clone()) {
                    Some(s) if s.led_by(pr) => s,
                    _ => return Err(Errno::EPERM),
                }
            };
            let ttyvp = !sess.ttyvp.is_none();
            let pgrp = {
                let r = klock::reader();
                pr.pgrp(&r).cloned()
            };
            let old = {
                let mut st = tp.st.lock();
                let cur = st.session.as_ref().map_or(0, SessionRef::<G>::addr);
                if (ttyvp || cur != 0) && cur != sess.addr() {
                    return Err(Errno::EPERM);
                }
                // SESSRELE(old), SESSHOLD(new): the tty's handle moves to the new session.
                let old_sess = st.session.replace(sess.clone());
                let old_pgrp = core::mem::replace(&mut st.pgrp, pgrp);
                (old_sess, old_pgrp)
            };
            drop(old);
            set_s_ttyp(&sess, tty_ref(tp));
            pr.ps_flags.fetch_or(PS_CONTROLT, Ordering::SeqCst);
        }
        FIOSETOWN => {
            // set pgrp of tty
            let pgid = int_arg(data)?;

            if !isctty(pr, tp) {
                return Err(Errno::ENOTTY);
            }
            let pgrp = {
                let r = klock::reader();
                let pgrp = if pgid < 0 {
                    pgrp_ref_of::<G, _>(&r, pgfind::<G, _>(&r, -pgid))
                } else {
                    let Some(pr1) = prfind::<G, _>(&r, pgid) else {
                        return Err(Errno::ESRCH);
                    };
                    pr1.pgrp(&r).cloned()
                };
                let mysess = pr.session(&r).map_or(0, |s| ptr::from_ref(s).addr());
                match pgrp {
                    None => return Err(Errno::EINVAL),
                    Some(pg) if pg.session(&r).map_or(0, |s| ptr::from_ref(s).addr()) != mysess => {
                        return Err(Errno::EPERM);
                    }
                    Some(pg) => pg,
                }
            };
            set_t_pgrp(tp, pgrp);
        }
        TIOCSPGRP => {
            // set pgrp of tty
            if !isctty(pr, tp) {
                return Err(Errno::ENOTTY);
            }
            let pgid = int_arg(data)?;
            let pgrp = {
                let r = klock::reader();
                let pgrp = pgrp_ref_of::<G, _>(&r, pgfind::<G, _>(&r, pgid));
                let mysess = pr.session(&r).map_or(0, |s| ptr::from_ref(s).addr());
                match pgrp {
                    None => return Err(Errno::EINVAL),
                    Some(pg) if pg.session(&r).map_or(0, |s| ptr::from_ref(s).addr()) != mysess => {
                        return Err(Errno::EPERM);
                    }
                    Some(pg) => pg,
                }
            };
            set_t_pgrp(tp, pgrp);
        }
        TIOCSTAT => {
            // get load avg stats
            ttyinfo(tp);
        }
        TIOCSWINSZ => {
            // set window size
            let ws: Winsize = pod_from_bytes(data).ok_or(Errno::EINVAL)?;
            let mut s = Sect::new(tp);
            if s.st().winsize != ws {
                s.st().winsize = ws;
                s.pgsignal(SIGWINCH);
            }
        }
        TIOCSTSTAMP => {
            let ts: Tstamps = pod_from_bytes(data).ok_or(Errno::EINVAL)?;
            let mut flags = tp.t_flags.get();
            flags &= !(TS_TSTAMPDCDSET | TS_TSTAMPCTSSET | TS_TSTAMPDCDCLR | TS_TSTAMPCTSCLR);
            if ts.ts_set & TIOCM_CAR != 0 {
                flags |= TS_TSTAMPDCDSET;
            }
            if ts.ts_set & TIOCM_CTS != 0 {
                flags |= TS_TSTAMPCTSSET;
            }
            if ts.ts_clr & TIOCM_CAR != 0 {
                flags |= TS_TSTAMPDCDCLR;
            }
            if ts.ts_clr & TIOCM_CTS != 0 {
                flags |= TS_TSTAMPCTSCLR;
            }
            tp.t_flags.set(flags);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// `TIOCSETA`, `TIOCSETAW`, `TIOCSETAF`: the new termios `t` (its `EXTPROC` bit replaced by
/// the tty's), at `spltty()` across, as the C.
fn ttsetattr<G: ProcGlue>(tp: &Tty<G>, cmd: u64, t: &mut Termios) -> Result<(), Errno> {
    let _spl = hal::intr::splraise(IPL_TTY);
    if cmd == TIOCSETAW || cmd == TIOCSETAF {
        ttywait(tp)?;
        if cmd == TIOCSETAF {
            ttyflush(tp, FREAD);
        }
    }
    let mut s = Sect::new(tp);
    if t.c_cflag & CIGNORE == 0 {
        // Some minor validation is necessary.
        if t.c_ispeed < 0 || t.c_ospeed < 0 {
            return Err(Errno::EINVAL);
        }
        // Set device hardware.
        let driver = s.driver();
        let tc = *t;
        s.out(|| driver.param(tp, &tc))?;
        if !tp.t_state_isset(TS_CARR_ON) && tp.t_cflag() & CLOCAL != 0 && t.c_cflag & CLOCAL == 0 {
            tp.t_state_clr(TS_ISOPEN);
            tp.t_state_set(TS_WOPEN);
            ttwakeup_s(&mut s);
        }
        tp.set_t_cflag(t.c_cflag);
        tp.set_t_ispeed(t.c_ispeed);
        tp.set_t_ospeed(t.c_ospeed);
        if t.c_ospeed == 0 {
            hangup_leader_s(&mut s);
        }
        ttsetwater(tp);
    }
    if cmd != TIOCSETAF && t.c_lflag & ICANON != tp.t_lflag() & ICANON {
        if t.c_lflag & ICANON != 0 {
            tp.t_lflag_set(PENDIN);
            ttwakeup_s(&mut s);
        } else {
            tp.t_rawq.catq(&tp.t_canq);
            tp.t_rawq.swap(&tp.t_canq);
            tp.t_lflag_clr(PENDIN);
        }
    }
    tp.set_t_iflag(t.c_iflag);
    tp.set_t_oflag(t.c_oflag);
    // Make the EXTPROC bit read only.
    if tp.t_lflag() & EXTPROC != 0 {
        t.c_lflag |= EXTPROC;
    } else {
        t.c_lflag &= !EXTPROC;
    }
    tp.set_t_lflag(t.c_lflag | (tp.t_lflag() & PENDIN));
    s.st().cc = t.c_cc;
    Ok(())
}

/// The counted handle of a tty from [`ttymalloc`] (every tty is one).
fn tty_ref<G: ProcGlue>(tp: &Tty<G>) -> TtyRef<G> {
    match PoolArc::from_ref(tp) {
        Some(h) => h,
        None => kpanic(format_args!("tty: a tty not from ttymalloc")),
    }
}

/// A counted handle of the group `pg` found under the kernel lock (`pgfind`'s): one of its
/// members' (a group in the hash has at least one, and `pgrp0`'s is process 0's `static`).
fn pgrp_ref_of<G: ProcGlue, W: hal::rwlock::Witness + ?Sized>(
    w: &W,
    pg: Option<&crate::proc::Pgrp<G>>,
) -> Option<PgrpRef<G>> {
    pg?.pg_members.first(w)?.pgrp(w).cloned()
}

/// `tp->t_pgrp = pgrp`, the old group's handle dropped unlocked.
fn set_t_pgrp<G: ProcGlue>(tp: &Tty<G>, pgrp: PgrpRef<G>) {
    let old = tp.st.lock().pgrp.replace(pgrp);
    drop(old);
}

/// `sess->s_ttyp = tp`: the session's controlling terminal (counted; not cleared when the
/// session loses it, to remember that it once had one).
fn set_s_ttyp<G: ProcGlue>(sess: &crate::proc::Session<G>, tp: TtyRef<G>) {
    let old = {
        let mut w = klock::witness();
        sess.s_ttyp.get_mut(&mut w).replace(tp)
    };
    drop(old);
}

/// `struct timeval` of `TIOCGTSTAMP`, the kernel's two `int64`s (`TIOCGTSTAMP`'s size).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct TstampTimeval {
    tv_sec: i64,
    tv_usec: i64,
}

hal::abi_pod!(TstampTimeval {
    tv_sec: i64,
    tv_usec: i64,
});

impl From<Timeval> for TstampTimeval {
    fn from(tv: Timeval) -> Self {
        Self {
            tv_sec: tv.tv_sec,
            tv_usec: tv.tv_usec,
        }
    }
}

/// `ttkqfilter`: attaches a kqueue filter to a tty: read and (poll's) except knotes on
/// `t_rsel`, write knotes on `t_wsel`; the knote holds the tty (its counted hook).
pub fn ttkqfilter<G: ProcGlue>(tp: &Tty<G>, kn: &Knote<G>) -> Result<(), Errno> {
    let klist = match kn.kn_filter().get() {
        EVFILT_READ => {
            kn.kn_fop.set(Some(&TtyFiltops::<G>::READ));
            &tp.t_rsel.si_note
        }
        EVFILT_WRITE => {
            kn.kn_fop.set(Some(&TtyFiltops::<G>::WRITE));
            &tp.t_wsel.si_note
        }
        EVFILT_EXCEPT => {
            if kn.has_flags(__EV_SELECT) {
                // Prevent triggering exceptfds.
                return Err(Errno::EPERM);
            }
            if !kn.has_flags(__EV_POLL) {
                // Disallow usage through kevent(2).
                return Err(Errno::EINVAL);
            }
            kn.kn_fop.set(Some(&TtyFiltops::<G>::EXCEPT));
            &tp.t_rsel.si_note
        }
        _ => return Err(Errno::EINVAL),
    };

    // kn->kn_hook = tp
    if kn.set_hook_shared(PoolArc::erase(tty_ref(tp))).is_err() {
        kpanic(format_args!("ttkqfilter: knote {kn:p} has a hook"));
    }

    let _s = hal::intr::splraise(IPL_TTY);
    klist.insert_legacy(kn);
    Ok(())
}

/// The erased tables of the three filters (`kn_fop`'s targets).
struct TtyFiltops<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> TtyFiltops<G> {
    /// `&ttyread_filtops`.
    const READ: Filterops<G> = Filterops::of::<TtyRead>();
    /// `&ttywrite_filtops`.
    const WRITE: Filterops<G> = Filterops::of::<TtyWrite>();
    /// `&ttyexcept_filtops`.
    const EXCEPT: Filterops<G> = Filterops::of::<TtyExcept>();
}

/// `kn->kn_hook` of a tty knote: its tty.
fn kn_tty<G: ProcGlue>(kn: &Knote<G>) -> &Tty<G> {
    match kn.hook_shared::<Tty<G>>() {
        Some(tp) => tp,
        None => kpanic(format_args!("knote {kn:p}: no tty")),
    }
}

/// `filt_ttyrdetach`: unhooks a read or except knote.
fn filt_ttyrdetach<G: ProcGlue>(kn: &Knote<G>) {
    let tp = kn_tty(kn);
    let _s = hal::intr::splraise(IPL_TTY);
    tp.t_rsel.si_note.remove_legacy(kn);
}

impl<G: ProcGlue> Filter<G> for TtyRead {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ttyrdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ttyrdetach(kn);
    }

    /// `filt_ttyread`: readable when a read would return characters; EOF (and, for poll, a
    /// hang-up) when the carrier is gone on a line that watches it.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let tp = kn_tty(kn);
        let n = {
            let mut s = Sect::new(tp);
            ttnread_s(&mut s)
        };
        kn.kn_data().set(i64::from(n));
        let mut active = n > 0;
        if tp.t_cflag() & CLOCAL == 0 && !tp.t_state_isset(TS_CARR_ON) {
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

impl<G: ProcGlue> Filter<G> for TtyWrite {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ttywdetach`.
    fn detach(kn: &Knote<G>) {
        let tp = kn_tty(kn);
        let _s = hal::intr::splraise(IPL_TTY);
        tp.t_wsel.si_note.remove_legacy(kn);
    }

    /// `filt_ttywrite`: writable when the output queue is below its low-water mark.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let tp = kn_tty(kn);
        let _s = hal::intr::splraise(IPL_TTY);
        kn.kn_data()
            .set(i64::from(tp.t_outq.c_cn.get() - tp.t_outq.c_cc.get()));
        let mut active = tp.t_outq.c_cc.get() <= i32::from(tp.t_lowat.get());

        // Write-side HUP condition is only for poll(2) and select(2).
        if kn.has_flags(__EV_POLL | __EV_SELECT) {
            if tp.t_cflag() & CLOCAL == 0 && !tp.t_state_isset(TS_CARR_ON) {
                kn.set_flags(__EV_HUP);
                active = true;
            } else {
                kn.clear_flags(__EV_HUP);
            }
        }
        active
    }
}

impl<G: ProcGlue> Filter<G> for TtyExcept {
    const FLAGS: i32 = FILTEROP_ISFD;

    /// `filt_ttyrdetach`.
    fn detach(kn: &Knote<G>) {
        filt_ttyrdetach(kn);
    }

    /// `filt_ttyexcept`: for poll only, the hang-up of a line whose carrier is gone.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let tp = kn_tty(kn);
        let mut active = false;
        let _s = hal::intr::splraise(IPL_TTY);
        if kn.has_flags(__EV_POLL) {
            if tp.t_cflag() & CLOCAL == 0 && !tp.t_state_isset(TS_CARR_ON) {
                kn.set_flags(__EV_HUP);
                active = true;
            } else {
                kn.clear_flags(__EV_HUP);
            }
        }
        active
    }
}

/// `ttnread`: the characters a read would return now (pending input taken first).
fn ttnread_s<G: ProcGlue>(s: &mut Sect<'_, G>) -> i32 {
    let tp = s.tp;
    if tp.t_lflag() & PENDIN != 0 {
        ttypend_s(s);
    }
    let mut nread = tp.t_canq.c_cc.get();
    if tp.t_lflag() & ICANON == 0 {
        nread += tp.t_rawq.c_cc.get();
        let cc = s.st().cc;
        if nread < i32::from(cc[VMIN as usize]) && cc[VTIME as usize] == 0 {
            nread = 0;
        }
    }
    nread
}

/// `ttnread(tp)`.
pub fn ttnread<G: ProcGlue>(tp: &Tty<G>) -> i32 {
    let mut s = Sect::new(tp);
    ttnread_s(&mut s)
}

/// A second in nanoseconds (`SEC_TO_NSEC(1)`).
const SEC_NSEC: u64 = 1_000_000_000;

/// `ttywait_nsec`: wait for output to drain, or if this times out, flush it.
pub fn ttywait_nsec<G: ProcGlue>(tp: &Tty<G>, nsecs: u64) -> Result<(), Errno> {
    let mut error = Ok(());
    let mut s = Sect::new(tp);
    let draining = |s: &mut Sect<'_, G>| {
        (tp.t_outq.c_cc.get() != 0 || tp.t_state_isset(TS_BUSY))
            && (tp.t_state_isset(TS_CARR_ON) || tp.t_cflag() & CLOCAL != 0)
            && s.st().driver.present()
    };
    while draining(&mut s) {
        ttstart_s(&mut s);
        if draining(&mut s) {
            tp.t_state_set(TS_ASLEEP);
            error = s.out(|| {
                ttysleep_nsec(
                    tp,
                    ptr::from_ref(&tp.t_outq).cast(),
                    TTOPRI | PCATCH,
                    TTYOUT,
                    nsecs,
                )
            });
            if error == Err(Errno::EWOULDBLOCK) {
                ttyflush_s(&mut s, FWRITE);
            }
            if error.is_err() {
                break;
            }
        } else {
            break;
        }
    }
    error
}

/// `ttywait`: wait for output to drain.
pub fn ttywait<G: ProcGlue>(tp: &Tty<G>) -> Result<(), Errno> {
    ttywait_nsec(tp, INFSLP)
}

/// `ttywflush`: flush if successfully wait.
pub fn ttywflush<G: ProcGlue>(tp: &Tty<G>) -> Result<(), Errno> {
    let error = ttywait_nsec(tp, 5 * SEC_NSEC);
    if error.is_ok() || error == Err(Errno::EWOULDBLOCK) {
        ttyflush(tp, FREAD);
    }
    error
}

/// `ttyflush`: flush tty read and/or write queues, notifying anyone waiting.
pub fn ttyflush<G: ProcGlue>(tp: &Tty<G>, rw: i32) {
    let mut s = Sect::new(tp);
    ttyflush_s(&mut s, rw);
}

/// [`ttyflush`] inside a section.
fn ttyflush_s<G: ProcGlue>(s: &mut Sect<'_, G>, rw: i32) {
    let tp = s.tp;
    if rw & FREAD != 0 {
        flushq(&tp.t_canq);
        flushq(&tp.t_rawq);
        let st = s.st();
        st.rocount = 0;
        st.rocol = 0;
        tp.t_state_clr(TS_LOCAL);
        ttyunblock_s(s);
        ttwakeup_s(s);
    }
    if rw & FWRITE != 0 {
        tp.t_state_clr(TS_TTSTOP);
        s.d_stop(rw);
        flushq(&tp.t_outq);
        s.out(|| {
            hal::sleep::wakeup(ptr::from_ref(&tp.t_outq));
            selwakeup(&tp.t_wsel);
        });
    }
}

/// `ttychars`: copy in the default termios characters.
pub fn ttychars<G: ProcGlue>(tp: &Tty<G>) {
    tp.set_t_cc_all(TTYDEFCHARS);
}

/// `ttyblock`: send stop character on input overflow.
fn ttyblock_s<G: ProcGlue>(s: &mut Sect<'_, G>) {
    let tp = s.tp;
    let total = tp.t_rawq.c_cc.get() + tp.t_canq.c_cc.get();
    if tp.t_rawq.c_cc.get() > tp.ttyhog() {
        ttyflush_s(s, FREAD | FWRITE);
        tp.t_state_clr(TS_TBLOCK);
    }
    // Block further input iff: current input > threshold AND input is available to user
    // program.
    if (total >= tp.ttyhog() / 2 && !tp.t_state_isset(TS_TBLOCK) && tp.t_lflag() & ICANON == 0)
        || tp.t_canq.c_cc.get() > 0
    {
        let vstop = s.st().cc[VSTOP as usize];
        if tp.t_iflag() & IXOFF != 0
            && vstop != _POSIX_VDISABLE
            && tp.t_outq.putc(i32::from(vstop)) == 0
        {
            tp.t_state_set(TS_TBLOCK);
            ttstart_s(s);
        }
        // Try to block remote output via hardware flow control.
        if tp.t_cflag() & CHWFLOW != 0 {
            let driver = s.driver();
            if s.out(|| driver.hwiflow(tp, true)) {
                tp.t_state_set(TS_TBLOCK);
            }
        }
    }
}

/// `ttrstrt`: the `t_rstrt_to` timeout: restart output after a delay. Its argument is the
/// tty's erased handle ([`ttymalloc`]).
fn ttrstrt<G: ProcGlue>(arg: &AnyArc) {
    let Some(tp) = arg.downcast_ref::<Tty<G>>() else {
        kpanic(format_args!("ttrstrt"));
    };
    let mut s = Sect::new(tp);
    tp.t_state_clr(TS_TIMEOUT);
    ttstart_s(&mut s);
}

/// `ttstart`: start output through the driver's `t_oproc`.
pub fn ttstart<G: ProcGlue>(tp: &Tty<G>) -> i32 {
    let mut s = Sect::new(tp);
    ttstart_s(&mut s);
    0
}

/// [`ttstart`] inside a section: the driver's start runs with the state unlocked.
fn ttstart_s<G: ProcGlue>(s: &mut Sect<'_, G>) {
    let driver = s.driver();
    if driver.present() {
        // XXX: Kludge for pty.
        let tp = s.tp;
        s.out(|| driver.start(tp));
    }
}

/// `ttylclose`: "close" a line discipline.
pub fn ttylclose<G: ProcGlue>(tp: &Tty<G>, flag: i32, _p: Option<&Thread<G>>) -> Result<(), Errno> {
    if flag & FNONBLOCK != 0 {
        ttyflush(tp, FREAD | FWRITE);
    } else {
        let _ = ttywflush(tp);
    }
    Ok(())
}

/// `ttymodem`: handle modem control transition on a tty. `flag` is the new state of the
/// carrier. Returns 0 if the line should be turned off, otherwise 1.
pub fn ttymodem<G: ProcGlue>(tp: &Tty<G>, flag: i32) -> i32 {
    let mut s = Sect::new(tp);
    if !tp.t_state_isset(TS_WOPEN) && tp.t_cflag() & MDMBUF != 0 {
        // MDMBUF: do flow control according to carrier flag
        if flag != 0 {
            tp.t_state_clr(TS_TTSTOP);
            ttstart_s(&mut s);
        } else if !tp.t_state_isset(TS_TTSTOP) {
            tp.t_state_set(TS_TTSTOP);
            s.d_stop(0);
        }
    } else if flag == 0 {
        // Lost carrier.
        tp.t_state_clr(TS_CARR_ON);
        if tp.t_state_isset(TS_ISOPEN) && tp.t_cflag() & CLOCAL == 0 {
            hangup_leader_s(&mut s);
            ttyflush_s(&mut s, FREAD | FWRITE);
            return 0;
        }
    } else {
        // Carrier now on.
        tp.t_state_set(TS_CARR_ON);
        ttwakeup_s(&mut s);
    }
    1
}

/// `if (tp->t_session && tp->t_session->s_leader) prsignal(tp->t_session->s_leader,
/// SIGHUP)`: the leader found by its pid under the kernel lock (the session keeps the
/// leader's address and pid, never a reference).
fn hangup_leader_s<G: ProcGlue>(s: &mut Sect<'_, G>) {
    let sess = s.st().session.clone();
    s.out(move || {
        if let Some(pid) = sess.as_ref().and_then(|s| s.leader_pid()) {
            let r = klock::reader();
            if let Some(leader) = prfind::<G, _>(&r, pid)
                && sess.as_ref().is_some_and(|s| s.led_by(leader))
            {
                prsignal(leader, SIGHUP);
            }
        }
        drop(sess);
    });
}

/// `nullmodem`: default modem control routine (for other line disciplines). Return argument
/// flag, to turn off device on carrier drop.
pub fn nullmodem<G: ProcGlue>(tp: &Tty<G>, flag: i32) -> i32 {
    let mut s = Sect::new(tp);
    if flag != 0 {
        tp.t_state_set(TS_CARR_ON);
    } else {
        tp.t_state_clr(TS_CARR_ON);
        if tp.t_state_isset(TS_ISOPEN) && tp.t_cflag() & CLOCAL == 0 {
            hangup_leader_s(&mut s);
            ttyflush_s(&mut s, FREAD | FWRITE);
            return 0;
        }
    }
    1
}

/// `ttypend`: reinput pending characters after state switch; call at `spltty()`.
pub fn ttypend<G: ProcGlue>(tp: &Tty<G>) {
    let mut s = Sect::new(tp);
    ttypend_s(&mut s);
}

/// [`ttypend`] inside a section: the raw queue requeued in place (the module's redesign),
/// each character through `ttyinput` again, `TS_TYPEN` set across.
fn ttypend_s<G: ProcGlue>(s: &mut Sect<'_, G>) {
    let tp = s.tp;
    tp.t_lflag_clr(PENDIN);
    tp.t_state_set(TS_TYPEN);
    let n = tp.t_rawq.c_cc.get();
    for _ in 0..n {
        let c = tp.t_rawq.getc();
        if c < 0 {
            break;
        }
        s.out(|| hal::random::enqueue_randomness(((tp.t_dev.get() << 8) | c) as u32));
        if tp.t_cflag() & CREAD != 0 {
            let _ = ttyinput_s(c, s);
        }
    }
    tp.t_state_clr(TS_TYPEN);
}

/// `ttvtimeout`: the `VTIME` timer of a non-canonical read: its argument is the channel it
/// wakes (`&tp->t_rawq`), compared and never followed.
fn ttvtimeout(arg: *mut core::ffi::c_void) {
    hal::sleep::wakeup(arg.cast_const());
}

/// What `ttread`'s queue checks decided.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Wait {
    /// `goto read` from the raw queue.
    ReadRaw,
    /// `goto read` from the canonical queue.
    ReadCan,
    /// `goto sleep`.
    Sleep,
}

/// `ttread`: process a read call on a tty device.
pub fn ttread<G: ProcGlue>(tp: &Tty<G>, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let stime = core::pin::pin!(Timeout::unset());
    let stime: Pin<&Timeout> = stime.into_ref();
    let mut armed = false;
    let mut last_cc = 0;

    let error = 'out: loop {
        // loop:
        let lflag = tp.t_lflag();
        // take pending input first
        if lflag & PENDIN != 0 {
            ttypend(tp);
        }

        // Hang process if it's in the background.
        let bg = G::with_curproc(|p| {
            let p = p?;
            let pr = p.process();
            if !isbackground(pr, tp) {
                return None;
            }
            // A process in the background of a tty is in a process group.
            if sigismasked(p, SIGTTIN)
                || pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT != 0
                || pgrp_orphaned(pr)
            {
                return Some(Err(Errno::EIO));
            }
            pgsignal_own(pr, SIGTTIN);
            Some(Ok(()))
        });
        match bg {
            Some(Err(e)) => break 'out Err(e),
            Some(Ok(())) => {
                if let Err(e) = ttysleep_nsec(
                    tp,
                    ptr::from_ref(&NOWAKE).cast(),
                    TTIPRI | PCATCH,
                    TTYBG,
                    SEC_NSEC,
                ) {
                    break 'out Err(e);
                }
                continue 'out;
            }
            None => {}
        }

        let mut s = Sect::new(tp);
        let cc = s.st().cc;
        let wait = if lflag & ICANON == 0 {
            let min = i32::from(cc[VMIN as usize]);
            let time = u64::from(cc[VTIME as usize]) * 100; // tenths of a second (ms)

            let qp = &tp.t_rawq;
            // Check each of the four combinations. (min > 0 && time == 0) is the normal
            // read case. It should be fairly efficient, so we check that and its companion
            // case (min == 0 && time == 0) first.
            'nc: {
                if time == 0 {
                    if qp.c_cc.get() < min {
                        break 'nc Wait::Sleep;
                    }
                    break 'nc Wait::ReadRaw;
                }
                if min > 0 {
                    if qp.c_cc.get() <= 0 {
                        break 'nc Wait::Sleep;
                    }
                    if qp.c_cc.get() >= min {
                        break 'nc Wait::ReadRaw;
                    }
                    if !armed {
                        // alloc_timer:
                        stime.set(
                            Some(ttvtimeout),
                            ptr::from_ref(&tp.t_rawq).cast_mut().cast(),
                            KCLOCK_NONE,
                            0,
                        );
                        let _ = stime.add_msec(time);
                        armed = true;
                    } else if qp.c_cc.get() > last_cc {
                        // got a character, restart timer
                        let _ = stime.add_msec(time);
                    }
                } else {
                    // min == 0
                    if qp.c_cc.get() > 0 {
                        break 'nc Wait::ReadRaw;
                    }
                    if !armed {
                        stime.set(
                            Some(ttvtimeout),
                            ptr::from_ref(&tp.t_rawq).cast_mut().cast(),
                            KCLOCK_NONE,
                            0,
                        );
                        let _ = stime.add_msec(time);
                        armed = true;
                    }
                }
                last_cc = qp.c_cc.get();
                if armed && !stime.triggered() {
                    break 'nc Wait::Sleep;
                }
                Wait::ReadRaw
            }
        } else if tp.t_canq.c_cc.get() <= 0 {
            Wait::Sleep
        } else {
            Wait::ReadCan
        };

        if wait == Wait::Sleep {
            // If there is no input, sleep on rawq awaiting hardware receipt and
            // notification. If we have data, we don't need to check for carrier.
            let carrier = tp.t_state_isset(TS_CARR_ON) || tp.t_cflag() & CLOCAL != 0;
            if !carrier && tp.t_state_isset(TS_ISOPEN) {
                break 'out Ok(());
            }
            if flag & IO_NDELAY != 0 {
                break 'out Err(Errno::EWOULDBLOCK);
            }
            let mut error = s.out(|| {
                ttysleep(
                    tp,
                    ptr::from_ref(&tp.t_rawq).cast(),
                    TTIPRI | PCATCH,
                    if carrier { TTYIN } else { TTOPEN },
                )
            });
            drop(s);
            if armed && stime.triggered() {
                error = Err(Errno::EWOULDBLOCK);
            }
            if tp.t_cc(VMIN) == 0 && error == Err(Errno::EWOULDBLOCK) {
                break 'out Ok(());
            }
            if let Err(e) = error
                && e != Errno::EWOULDBLOCK
            {
                break 'out Err(e);
            }
            continue 'out;
        }
        // read:
        drop(s);
        let qp = if wait == Wait::ReadRaw {
            &tp.t_rawq
        } else {
            &tp.t_canq
        };

        // Input present, check for input mapping and processing.
        let mut first = true;
        let mut error = Ok(());
        let mut again = false;
        loop {
            let c = qp.getc();
            if c < 0 {
                break;
            }
            // delayed suspend (^Y)
            if cceq_c(cc[VDSUSP as usize], c) && lflag & (IEXTEN | ISIG) == (IEXTEN | ISIG) {
                let pg = tp.pgrp();
                pgsignal(pg.as_deref(), SIGTSTP, true);
                drop(pg);
                if first {
                    error = ttysleep_nsec(
                        tp,
                        ptr::from_ref(&NOWAKE).cast(),
                        TTIPRI | PCATCH,
                        TTYBG,
                        SEC_NSEC,
                    );
                    if error.is_err() {
                        break;
                    }
                    again = true;
                }
                break;
            }
            // Interpret EOF only in canonical mode.
            if cceq_c(cc[VEOF as usize], c) && lflag & ICANON != 0 {
                break;
            }
            // Give user character.
            error = uio.uiomove(&mut [c as u8]);
            if error.is_err() {
                break;
            }
            if uio.resid() == 0 {
                break;
            }
            // In canonical mode check for a "break character" marking the end of a "line
            // of input".
            if lflag & ICANON != 0 && ttbreakc(c, &cc, lflag) {
                break;
            }
            first = false;
        }
        if again {
            continue 'out;
        }
        // Look to unblock output now that (presumably) the input queue has gone down.
        let mut s = Sect::new(tp);
        if tp.t_rawq.c_cc.get() < tp.ttyhog() / 5 {
            ttyunblock_s(&mut s);
        }
        break 'out error;
    };

    // out:
    if armed {
        let _ = stime.del();
    }
    error
}

/// `ttyunblock`: call at `spltty`.
pub fn ttyunblock<G: ProcGlue>(tp: &Tty<G>) {
    let mut s = Sect::new(tp);
    ttyunblock_s(&mut s);
}

/// [`ttyunblock`] inside a section.
fn ttyunblock_s<G: ProcGlue>(s: &mut Sect<'_, G>) {
    let tp = s.tp;
    if tp.t_state_isset(TS_TBLOCK) {
        let vstart = s.st().cc[VSTART as usize];
        if tp.t_iflag() & IXOFF != 0
            && vstart != _POSIX_VDISABLE
            && tp.t_outq.putc(i32::from(vstart)) == 0
        {
            tp.t_state_clr(TS_TBLOCK);
            ttstart_s(s);
        }
        // Try to unblock remote output via hardware flow control.
        if tp.t_cflag() & CHWFLOW != 0 {
            let driver = s.driver();
            if s.out(|| driver.hwiflow(tp, false)) {
                tp.t_state_clr(TS_TBLOCK);
            }
        }
    }
}

/// `ttycheckoutq`: check the output queue on `tp` for space for a kernel message (from
/// `uprintf` or `tprintf`). Allow some space over the normal hiwater mark so we don't lose
/// messages due to normal flow control, but don't let the tty run amok. Sleeps here are not
/// interruptible, but we return prematurely if new signals arrive.
pub fn ttycheckoutq<G: ProcGlue>(tp: &Tty<G>, wait: bool) -> bool {
    let hiwat = i32::from(tp.t_hiwat.get());
    let mut s = Sect::new(tp);
    let oldsig = if wait {
        G::with_curproc(|p| p.map(sigpending))
    } else {
        None
    };
    if tp.t_outq.c_cc.get() > hiwat + TTHIWATMINSPACE {
        while tp.t_outq.c_cc.get() > hiwat {
            ttstart_s(&mut s);
            if !wait || G::with_curproc(|p| p.map(sigpending)) != oldsig {
                return false;
            }
            tp.t_state_set(TS_ASLEEP);
            s.out(|| {
                let _ = hal::sleep::tsleep_nsec(
                    ptr::from_ref(&tp.t_outq),
                    PZERO - 1,
                    "ttckoutq",
                    SEC_NSEC,
                );
            });
        }
    }
    true
}

/// How `ttwrite`'s main loop ended.
enum WriteExit {
    /// `goto out`: give back what was copied in but not queued.
    Out(Result<(), Errno>),
    /// `goto done`.
    Done(Result<(), Errno>),
}

/// `ttwrite`: process a write call on a tty device.
pub fn ttwrite<G: ProcGlue>(tp: &Tty<G>, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let mut obuf = [0u8; OBUFSIZ];
    let mut cp = 0usize;
    let mut cc = 0usize;
    let mut obufcc = 0usize;

    let hiwat = i32::from(tp.t_hiwat.get());
    let cnt = uio.resid();

    let exit = 'top: loop {
        // loop:
        if !tp.t_state_isset(TS_CARR_ON) && tp.t_cflag() & CLOCAL == 0 {
            if tp.t_state_isset(TS_ISOPEN) {
                break 'top WriteExit::Done(Err(Errno::EIO));
            } else if flag & IO_NDELAY != 0 {
                break 'top WriteExit::Out(Err(Errno::EWOULDBLOCK));
            }
            // Sleep awaiting carrier.
            let error = {
                let _s = hal::intr::splraise(IPL_TTY);
                ttysleep(
                    tp,
                    ptr::from_ref(&tp.t_rawq).cast(),
                    TTIPRI | PCATCH,
                    TTOPEN,
                )
            };
            if let Err(e) = error {
                break 'top WriteExit::Out(Err(e));
            }
            continue 'top;
        }
        // Hang the process if it's in the background.
        let bg = G::with_curproc(|p| {
            let p = p?;
            let pr = p.process();
            if isbackground(pr, tp)
                && tp.t_lflag() & TOSTOP != 0
                && pr.ps_flags.load(Ordering::Relaxed) & PS_PPWAIT == 0
                && !sigismasked(p, SIGTTOU)
            {
                // A process in the background of a tty is in a process group.
                if pgrp_orphaned(pr) {
                    return Some(Err(Errno::EIO));
                }
                pgsignal_own(pr, SIGTTOU);
                return Some(Ok(()));
            }
            None
        });
        match bg {
            Some(Err(e)) => break 'top WriteExit::Out(Err(e)),
            Some(Ok(())) => {
                if let Err(e) = ttysleep_nsec(
                    tp,
                    ptr::from_ref(&NOWAKE).cast(),
                    TTIPRI | PCATCH,
                    TTYBG,
                    SEC_NSEC,
                ) {
                    break 'top WriteExit::Out(Err(e));
                }
                continue 'top;
            }
            None => {}
        }
        // Process the user's data in at most OBUFSIZ chunks. Perform any output
        // translation. Keep track of high water mark, sleep on overflow awaiting device aid
        // in acquiring new space.
        let mut error = Ok(());
        let mut ovhiwat = false;
        'chunks: while uio.resid() > 0 || cc > 0 {
            if tp.t_lflag() & FLUSHO != 0 {
                uio.set_resid(0);
                break 'top WriteExit::Done(Ok(()));
            }
            if tp.t_outq.c_cc.get() > hiwat {
                ovhiwat = true;
                break 'chunks;
            }
            // Grab a hunk of data from the user, unless we have some leftover from last
            // time.
            if cc == 0 {
                cc = uio.resid().min(OBUFSIZ);
                cp = 0;
                if let Err(e) = uio.uiomove(&mut obuf[..cc]) {
                    error = Err(e);
                    cc = 0;
                    break 'chunks;
                }
                if cc > obufcc {
                    obufcc = cc;
                }

                // duplicate /dev/console output into console buffer
                if console_dev::<G>() == Some(tp.t_dev.get()) && tp.t_gen.get() == 0 {
                    for &c in &obuf[..cc] {
                        if c != b'\0' && c != b'\r' && c != 0o177 {
                            crate::log::msgbuf_putchar(crate::log::Buf::Consbuf, c);
                        }
                    }
                }
            }
            // If nothing fancy need be done, grab those characters we can handle without
            // any of ttyoutput's processing and just transfer them to the output q. For
            // those chars which require special processing (as indicated by the bits in
            // char_type), call ttyoutput. After processing a hunk of data, look for FLUSHO
            // so ^O's will take effect immediately.
            let mut s = Sect::new(tp);
            while cc > 0 {
                let mut ce = if tp.t_oflag() & OPOST == 0 {
                    cc
                } else {
                    let ce = cc - libkern::scanc(&obuf[cp..cp + cc], &CHAR_TYPE, CCLASSMASK);
                    // If ce is zero, then we're processing a special character through
                    // ttyoutput.
                    if ce == 0 {
                        s.st().rocount = 0;
                        if ttyoutput_s(i32::from(obuf[cp]), &mut s) >= 0 {
                            // out of space
                            ovhiwat = true;
                            break 'chunks;
                        }
                        cp += 1;
                        cc -= 1;
                        if tp.t_lflag() & FLUSHO != 0 || tp.t_outq.c_cc.get() > hiwat {
                            ovhiwat = true;
                            break 'chunks;
                        }
                        continue;
                    }
                    ce
                };
                // A bunch of normal characters have been found. Transfer them en masse to
                // the output queue and continue processing at the top of the loop. If there
                // are any further characters in this <= OBUFSIZ chunk, the first should be a
                // character requiring special handling by ttyoutput.
                s.st().rocount = 0;
                let i = tp.t_outq.b_to_q(&obuf[cp..cp + ce]);
                ce -= i;
                let st = s.st();
                st.column = st.column.wrapping_add(ce as i16);
                cp += ce;
                cc -= ce;
                TK_NOUT.fetch_add(ce as i64, Ordering::Relaxed);
                tp.t_outcc.set(tp.t_outcc.get() + ce as i64);
                if i > 0 {
                    // out of space
                    ovhiwat = true;
                    break 'chunks;
                }
                if tp.t_lflag() & FLUSHO != 0 || tp.t_outq.c_cc.get() > hiwat {
                    break;
                }
            }
            ttstart_s(&mut s);
        }
        if !ovhiwat {
            break 'top WriteExit::Out(error);
        }

        // ovhiwat:
        let mut s = Sect::new(tp);
        ttstart_s(&mut s);
        // This can only occur if FLUSHO is set in t_lflag, or if ttstart/oproc is
        // synchronous (or very fast).
        if tp.t_outq.c_cc.get() <= hiwat {
            continue 'top;
        }
        if flag & IO_NDELAY != 0 {
            drop(s);
            uio.set_resid(uio.resid() + cc);
            if obufcc != 0 {
                libkern::explicit_bzero(&mut obuf[..obufcc]);
            }
            return if uio.resid() == cnt {
                Err(Errno::EWOULDBLOCK)
            } else {
                Ok(())
            };
        }
        tp.t_state_set(TS_ASLEEP);
        let error = s.out(|| {
            ttysleep(
                tp,
                ptr::from_ref(&tp.t_outq).cast(),
                TTOPRI | PCATCH,
                TTYOUT,
            )
        });
        if let Err(e) = error {
            break 'top WriteExit::Out(Err(e));
        }
    };

    let error = match exit {
        WriteExit::Out(error) => {
            // If cc is nonzero, we leave the uio structure inconsistent, as the offset and
            // iov pointers have moved forward, but it doesn't matter (the call will either
            // return short or restart with a new uio).
            uio.set_resid(uio.resid() + cc);
            error
        }
        WriteExit::Done(error) => error,
    };
    // done:
    if obufcc != 0 {
        libkern::explicit_bzero(&mut obuf[..obufcc]);
    }
    error
}

/// `ttyrub`: rubout one character from the rawq of tp as cleanly as possible.
pub fn ttyrub<G: ProcGlue>(c: i32, tp: &Tty<G>) -> i32 {
    let mut s = Sect::new(tp);
    ttyrub_s(c, &mut s)
}

/// [`ttyrub`] inside a section.
fn ttyrub_s<G: ProcGlue>(c: i32, s: &mut Sect<'_, G>) -> i32 {
    let tp = s.tp;
    let mut c = c;
    if tp.t_lflag() & ECHO == 0 || tp.t_lflag() & EXTPROC != 0 {
        return 0;
    }
    tp.t_lflag_clr(FLUSHO);
    if tp.t_lflag() & ECHOE != 0 {
        if s.st().rocount == 0 {
            // Screwed by ttwrite; retype
            return ttyretype_s(s);
        }
        if c == (i32::from(b'\t') | TTY_QUOTE) || c == (i32::from(b'\n') | TTY_QUOTE) {
            ttyrubo_s(s, 2);
        } else {
            c &= TTY_CHARMASK;
            match cclass(c) {
                ORDINARY => ttyrubo_s(s, 1),
                BACKSPACE | CONTROL | NEWLINE | RETURN | VTAB => {
                    if tp.t_lflag() & ECHOCTL != 0 {
                        ttyrubo_s(s, 2);
                    }
                }
                TAB => {
                    if i32::from(s.st().rocount) < tp.t_rawq.c_cc.get() {
                        return ttyretype_s(s);
                    }
                    let mut savecol = s.st().column;
                    tp.t_state_set(TS_CNTTB);
                    tp.t_lflag_set(FLUSHO);
                    let st = s.st();
                    st.column = st.rocol;
                    let (mut tabc, mut cc) = (0, 0);
                    let mut cp = tp.t_rawq.firstc(&mut tabc, &mut cc);
                    while cp.is_some() {
                        ttyecho_s(tabc, s);
                        cp = tp.t_rawq.nextc(cp, &mut tabc, &mut cc);
                    }
                    tp.t_lflag_clr(FLUSHO);
                    tp.t_state_clr(TS_CNTTB);

                    // savecol will now be length of the tab.
                    let st = s.st();
                    savecol = savecol.wrapping_sub(st.column);
                    st.column = st.column.wrapping_add(savecol);
                    if savecol > 8 {
                        savecol = 8; // overflow screw
                    }
                    loop {
                        savecol -= 1;
                        if savecol < 0 {
                            break;
                        }
                        let _ = ttyoutput_s(i32::from(b'\x08'), s);
                    }
                }
                _ => {
                    // XXX
                    let class = cclass(c);
                    s.out(|| {
                        crate::prf::printf(format_args!(
                            "ttyrub: would panic c = {c}, val = {class}\n"
                        ));
                    });
                }
            }
        }
    } else if tp.t_lflag() & ECHOPRT != 0 {
        if !tp.t_state_isset(TS_ERASE) {
            tp.t_state_set(TS_ERASE);
            let _ = ttyoutput_s(i32::from(b'\\'), s);
        }
        ttyecho_s(c, s);
    } else {
        let erase = s.st().cc[VERASE as usize];
        ttyecho_s(i32::from(erase), s);
    }
    let st = s.st();
    st.rocount = st.rocount.wrapping_sub(1);
    0
}

/// `ttyrubo`: back over `cnt` characters, erasing them.
fn ttyrubo_s<G: ProcGlue>(s: &mut Sect<'_, G>, cnt: i32) {
    for _ in 0..cnt {
        let _ = ttyoutput_s(i32::from(b'\x08'), s);
        let _ = ttyoutput_s(i32::from(b' '), s);
        let _ = ttyoutput_s(i32::from(b'\x08'), s);
    }
}

/// `ttyretype`: reprint the rawq line. Note, it is assumed that `c_cc` has already been
/// checked.
pub fn ttyretype<G: ProcGlue>(tp: &Tty<G>) -> i32 {
    let mut s = Sect::new(tp);
    ttyretype_s(&mut s)
}

/// [`ttyretype`] inside a section.
fn ttyretype_s<G: ProcGlue>(s: &mut Sect<'_, G>) -> i32 {
    let tp = s.tp;
    // Echo the reprint character.
    let reprint = s.st().cc[VREPRINT as usize];
    if reprint != _POSIX_VDISABLE {
        ttyecho_s(i32::from(reprint), s);
    }

    let _ = ttyoutput_s(i32::from(b'\n'), s);

    for q in [&tp.t_canq, &tp.t_rawq] {
        let (mut c, mut cc) = (0, 0);
        let mut cp = q.firstc(&mut c, &mut cc);
        while cp.is_some() {
            ttyecho_s(c, s);
            cp = q.nextc(cp, &mut c, &mut cc);
        }
    }
    tp.t_state_clr(TS_ERASE);

    let st = s.st();
    st.rocount = tp.t_rawq.c_cc.get() as i16;
    st.rocol = 0;
    1
}

/// `ttyecho`: echo a typed character to the terminal.
fn ttyecho_s<G: ProcGlue>(c: i32, s: &mut Sect<'_, G>) {
    let tp = s.tp;
    let mut c = c;
    if !tp.t_state_isset(TS_CNTTB) {
        tp.t_lflag_clr(FLUSHO);
    }
    let lflag = tp.t_lflag();
    if (lflag & ECHO == 0 && (lflag & ECHONL == 0 || c != i32::from(b'\n'))) || lflag & EXTPROC != 0
    {
        return;
    }
    if (lflag & ECHOCTL != 0
        && (c & TTY_CHARMASK) <= 0o37
        && c != i32::from(b'\t')
        && c != i32::from(b'\n'))
        || (c & TTY_CHARMASK) == 0o177
    {
        let _ = ttyoutput_s(i32::from(b'^'), s);
        c &= TTY_CHARMASK;
        if c == 0o177 {
            c = i32::from(b'?');
        } else {
            c += i32::from(b'A') - 1;
        }
    }
    let _ = ttyoutput_s(c, s);
}

/// `ttwakeupwr`: wakeup any writers if necessary.
pub fn ttwakeupwr<G: ProcGlue>(tp: &Tty<G>) {
    if tp.t_outq.c_cc.get() <= i32::from(tp.t_lowat.get()) {
        if tp.t_state_isset(TS_ASLEEP) {
            tp.t_state_clr(TS_ASLEEP);
            hal::sleep::wakeup(ptr::from_ref(&tp.t_outq));
        }
        selwakeup(&tp.t_wsel);
    }
}

/// `ttwakeup`: wake up any readers on a tty.
pub fn ttwakeup<G: ProcGlue>(tp: &Tty<G>) {
    let mut s = Sect::new(tp);
    ttwakeup_s(&mut s);
}

/// [`ttwakeup`] inside a section: the knotes and the signal with the state unlocked.
fn ttwakeup_s<G: ProcGlue>(s: &mut Sect<'_, G>) {
    let tp = s.tp;
    if !tp.t_rsel.si_note.is_empty() {
        s.out(|| selwakeup(&tp.t_rsel));
    }
    if tp.t_state_isset(TS_ASYNC) {
        s.pgsignal(SIGIO);
    }
    s.out(|| hal::sleep::wakeup(ptr::from_ref(&tp.t_rawq)));
}

/// `ttspeedtab`: look up a code for a specified speed in a conversion table; used by
/// drivers to map software speed values to hardware parameters. -1 when it is not there.
pub fn ttspeedtab(speed: i32, table: &[Speedtab]) -> i32 {
    for entry in table {
        if entry.sp_speed == -1 {
            break;
        }
        if entry.sp_speed == speed {
            return entry.sp_code;
        }
    }
    -1
}

/// `ttsetwater`: set tty hi and low water marks.
///
/// Try to arrange the dynamics so there's about one second from hi to low water.
pub fn ttsetwater<G: ProcGlue>(tp: &Tty<G>) {
    /// `CLAMP(x, h, l)`.
    fn clamp(x: i32, h: i32, l: i32) -> i32 {
        if x > h {
            h
        } else if x < l {
            l
        } else {
            x
        }
    }

    let cps = tp.t_ospeed() / 10;
    let mut x = clamp(cps / 2, TTMAXLOWAT, TTMINLOWAT);
    tp.t_lowat.set(x as i16);
    x += cps;
    tp.t_hiwat
        .set(clamp(x, tp.t_outq.c_cn.get() - TTHIWATMINSPACE, TTMINHIWAT) as i16);
}

/// `process_sum`: get the total estcpu for a process, summing across threads. Returns
/// true if at least one thread is runnable/running.
fn process_sum<G: ProcGlue>(pr: &Process<G>) -> (bool, u32) {
    let mut ret = false;
    let mut pctcpu: u32 = 0;
    // The thread list is ps_mtx's (the C walks it under the kernel lock alone).
    let g = pr.ps_mtx.lock();
    for p in pr.ps_threads.iter(&g) {
        if p.stat() == SRUN || p.stat() == SONPROC {
            ret = true;
        }
        pctcpu = pctcpu.wrapping_add(p.p_pctcpu.load(Ordering::Relaxed));
    }
    (ret, pctcpu)
}

/// `tuagg_get_process`: the usage of a process: its own plus all living threads.
fn process_usage<G: ProcGlue>(tu: &Tusage, pr: &Process<G>) {
    let g = pr.ps_mtx.lock();
    tuagg_sumup(tu, &pr.ps_tu);
    for p in pr.ps_threads.iter(&g) {
        tuagg_sumup(tu, &p.p_tu);
    }
}

/// What `ttyinfo` prints after the load average.
enum Info {
    /// "not a controlling terminal".
    NotCtty,
    /// "no foreground process group".
    NoPgrp,
    /// "empty foreground process group".
    Empty,
    /// The chosen process and thread: name, pid, state, user and system time, %cpu, rss.
    Cmd {
        comm: crate::proc::Name,
        pid: Pid,
        state: &'static str,
        utime: crate::clock::Timespec,
        stime: crate::clock::Timespec,
        pctcpu: i64,
        rss: i64,
    },
}

/// `ttyinfo`: report on state of foreground process group.
pub fn ttyinfo<G: ProcGlue>(tp: &Tty<G>) {
    if !ttycheckoutq(tp, false) {
        return;
    }

    // Print load average.
    let tmp = (i64::from(crate::sched::averunnable()[0]) * 100 + i64::from(FSCALE) / 2) >> FSHIFT;

    let (sess, pgrp) = {
        let st = tp.st.lock();
        (st.session.is_some(), st.pgrp.clone())
    };
    let info = if !sess {
        Info::NotCtty
    } else if let Some(pg) = pgrp.as_deref() {
        let r = klock::reader();
        pick(&r, pg).unwrap_or(Info::Empty)
    } else {
        Info::NoPgrp
    };
    drop(pgrp);

    let mut s = Sect::new(tp);
    let mut w = TtyWriter { s: &mut s };
    let _ = fmt::write(
        &mut w,
        format_args!("load: {}.{:02} ", tmp / 100, tmp % 100),
    );
    let _ = match info {
        Info::NotCtty => fmt::write(&mut w, format_args!("not a controlling terminal\n")),
        Info::NoPgrp => fmt::write(&mut w, format_args!("no foreground process group\n")),
        Info::Empty => fmt::write(&mut w, format_args!("empty foreground process group\n")),
        Info::Cmd {
            comm,
            pid,
            state,
            utime,
            stime,
            pctcpu,
            rss,
        } => fmt::write(
            &mut w,
            format_args!(
                " cmd: {} {} [{}] {}.{:02}u {}.{:02}s {}% {}k\n",
                ByteStr(&comm),
                pid,
                state,
                utime.tv_sec,
                utime.tv_nsec / 10_000_000,
                stime.tv_sec,
                stime.tv_nsec / 10_000_000,
                pctcpu / 100,
                rss
            ),
        ),
    };
    s.st().rocount = 0; // so pending input will be retyped if BS
}

/// `ttyinfo`'s choice in the group `pg`: the most active process, then its most active
/// thread; `None` when the group or the process has no member.
fn pick<G: ProcGlue, W: hal::rwlock::Witness + ?Sized>(
    r: &W,
    pg: &crate::proc::Pgrp<G>,
) -> Option<Info> {
    let first = pg.pg_members.first(r)?;
    // Pick the most active process:
    //  - prefer at least one running/runnable thread
    //  - prefer higher total pctcpu
    //  - prefer non-zombie
    // Otherwise take the most recently added to this process group
    let mut pickpr = first;
    let (mut run, mut pctcpu) = process_sum(pickpr);
    for pr in pg.pg_members.iter(r).skip(1) {
        let (run2, pctcpu2) = process_sum(pr);
        if run {
            // pick is running; is p running w/same or more cpu?
            if !(run2 && pctcpu2 >= pctcpu) {
                continue;
            }
        } else if !(run2 || pctcpu2 > pctcpu) {
            // pick isn't running; is p running *or* w/more cpu? if p has less cpu or is
            // exiting, then it's worse
            if pctcpu2 < pctcpu || pr.ps_flags.load(Ordering::Relaxed) & PS_EXITING != 0 {
                continue;
            }
        }
        // update_pickpr:
        pickpr = pr;
        run = run2;
        pctcpu = pctcpu2;
    }

    // Calculate percentage cpu, resident set size.
    let calc_pctcpu = ((u64::from(pctcpu) * 10000 + u64::from(FSCALE) / 2) >> FSHIFT) as i64;
    let mut rss: i64 = 0;
    if pickpr.ps_flags.load(Ordering::Relaxed) & (PS_EMBRYO | PS_EXITING) == 0
        // a live process holds its vmspace.
        && let Some(vm) = pickpr.vmspace_share()
    {
        rss = vm.vm_map.pmap().resident_count();
    }

    let tu = Tusage::new();
    process_usage(&tu, pickpr);
    let (mut utime, mut stime, _) = calctsru(&tu, G::stathz());

    // Round up and print user time.
    utime.tv_nsec += 5_000_000;
    if utime.tv_nsec >= 1_000_000_000 {
        utime.tv_sec += 1;
        utime.tv_nsec -= 1_000_000_000;
    }

    // Round up and print system time.
    stime.tv_nsec += 5_000_000;
    if stime.tv_nsec >= 1_000_000_000 {
        stime.tv_sec += 1;
        stime.tv_nsec -= 1_000_000_000;
    }

    // Find the most active thread:
    //  - prefer runnable
    //  - prefer higher pctcpu
    //  - prefer living
    // Otherwise take the newest thread
    let g = pickpr.ps_mtx.lock();
    let p = pickpr.ps_threads.first(&g)?;
    let mut pick = p;
    let mut run = p.stat() == SRUN || p.stat() == SONPROC;
    let mut pctcpu = p.p_pctcpu.load(Ordering::Relaxed);
    for p in pickpr.ps_threads.iter(&g).skip(1) {
        let run2 = p.stat() == SRUN || p.stat() == SONPROC;
        let pctcpu2 = p.p_pctcpu.load(Ordering::Relaxed);
        if run {
            // pick is running; is p running w/same or more cpu?
            if !(run2 && pctcpu2 >= pctcpu) {
                continue;
            }
        } else if !(run2 || pctcpu2 > pctcpu)
            && (pctcpu2 < pctcpu || p.p_flag.load(Ordering::Relaxed) & P_WEXIT != 0)
        {
            // pick isn't running; is p running *or* w/more cpu? if p has less cpu or is
            // exiting, then it's worse
            continue;
        }
        // update_pick:
        pick = p;
        run = run2;
        pctcpu = p.p_pctcpu.load(Ordering::Relaxed);
    }
    let state = if pick.stat() == SONPROC {
        "running"
    } else if pick.stat() == SRUN {
        "runnable"
    } else {
        pick.p_wmesg.get().unwrap_or("iowait")
    };
    Some(Info::Cmd {
        comm: pickpr.comm(),
        pid: pickpr.ps_pid,
        state,
        utime,
        stime,
        pctcpu: calc_pctcpu,
        rss,
    })
}

/// The bytes of a process name as `%s` prints them.
struct ByteStr<'a>(&'a [u8]);

impl fmt::Display for ByteStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &b in self.0 {
            fmt::Write::write_char(f, char::from(b))?;
        }
        Ok(())
    }
}

/// `ttyprintf(tp, ...)`'s output: every byte through `tputchar` inside one section.
struct TtyWriter<'s, 'a, G: ProcGlue> {
    s: &'s mut Sect<'a, G>,
}

impl<G: ProcGlue> fmt::Write for TtyWriter<'_, '_, G> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for b in text.bytes() {
            let _ = tputchar_s(i32::from(b), self.s);
        }
        Ok(())
    }
}

/// `ttyprintf(tp, fmt, ...)`: kernel output to a terminal (`uprintf`, `tprintf`), every byte
/// through `tputchar`.
pub fn ttyprintf<G: ProcGlue>(tp: &Tty<G>, args: fmt::Arguments<'_>) {
    let mut s = Sect::new(tp);
    let _ = fmt::write(&mut TtyWriter { s: &mut s }, args);
}

/// `uprintf(9)` (`subr_prf.c`): kernel output to the controlling terminal of `pr` (the C's
/// `curproc->p_p`), through [`ttyprintf`]; nothing when the process has none (`PS_CONTROLT`
/// clear, no session, or a session whose terminal has no vnode: `s_ttyvp`). The caller holds
/// the kernel lock, under which the session is read.
pub fn uprintf<G: ProcGlue>(pr: &Process<G>, args: fmt::Arguments<'_>) {
    if pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT == 0 {
        return;
    }
    let tp = {
        let r = klock::reader();
        pr.session(&r)
            .filter(|sess| !sess.ttyvp.is_none())
            .and_then(|sess| session_tty(sess))
    };
    if let Some(tp) = tp {
        ttyprintf(&tp, args);
    }
}

/// `tputchar`: output char to tty; console putchar style.
pub fn tputchar<G: ProcGlue>(c: i32, tp: &Tty<G>) -> i32 {
    let mut s = Sect::new(tp);
    tputchar_s(c, &mut s)
}

/// [`tputchar`] for `kputchar`'s `constty` leg (`printf(9)` redirected by `TIOCCONS`), which
/// may run at any level, from an interrupt above `IPL_TTY` or under `SCHED_LOCK`: `None`,
/// without touching the tty, when writing it would wait on this CPU (its state or output
/// queue held here, or the scheduler's lock, which a driver's start takes through
/// `wakeup`), or on another CPU that holds the state, so the caller puts the character on
/// the console instead. The C only raises `spltty()` there and re-enters the tty; the locks
/// that make its state typed would turn the re-entry into "locking against myself", and a
/// wait for a state held across `wakeup` into a deadlock with a `printf` under
/// `SCHED_LOCK`.
pub fn tputchar_try<G: ProcGlue>(c: i32, tp: &Tty<G>) -> Option<i32> {
    if hal::thread::sched_locked() || tp.st.owned() || tp.t_outq.locked_here() {
        return None;
    }
    let spl = hal::intr::splraise(IPL_TTY);
    let g = tp.st.try_lock()?;
    let mut s = Sect {
        tp,
        g: Some(g),
        _spl: spl,
    };
    Some(tputchar_s(c, &mut s))
}

/// [`tputchar`] inside a section.
fn tputchar_s<G: ProcGlue>(c: i32, s: &mut Sect<'_, G>) -> i32 {
    let tp = s.tp;
    if !tp.t_state_isset(TS_ISOPEN) || !(tp.t_state_isset(TS_CARR_ON) || tp.t_cflag() & CLOCAL != 0)
    {
        return -1;
    }
    if c == i32::from(b'\n') {
        let _ = ttyoutput_s(i32::from(b'\r'), s);
    }
    let _ = ttyoutput_s(c, s);
    ttstart_s(s);
    0
}

/// `ttysleep`: sleep on `chan`, returning `ERESTART` if tty changed while we napped and
/// returning any errors (e.g. `EINTR`/`ETIMEDOUT`) reported by `tsleep`. If the tty is
/// revoked, restarting a pending call will redo validation done at the start of the call.
pub fn ttysleep<G: ProcGlue>(
    tp: &Tty<G>,
    chan: *const (),
    pri: i32,
    wmesg: &'static str,
) -> Result<(), Errno> {
    ttysleep_nsec(tp, chan, pri, wmesg, INFSLP)
}

/// `ttysleep_nsec`: [`ttysleep`] with a timeout.
pub fn ttysleep_nsec<G: ProcGlue>(
    tp: &Tty<G>,
    chan: *const (),
    pri: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    let gen_ = tp.t_gen.get();
    hal::sleep::tsleep_nsec(chan, pri, wmesg, nsecs)?;
    if tp.t_gen.get() == gen_ {
        Ok(())
    } else {
        Err(Errno::ERESTART)
    }
}

/// `tty_init`: initialise the global tty list (and make `ttypl`).
pub fn tty_init<G: ProcGlue>() {
    let t = tables::<G>();
    t.tty_count.store(0, Ordering::Relaxed);
    ttypl_init::<G>();
}

/// `ttypl`, made once (at `tty_init`, or at the first `ttymalloc` of a host test).
fn ttypl_init<G: ProcGlue>() {
    let t = tables::<G>();
    if t.ttypl.item_size().is_none() {
        let _ = t.ttypl.init_recorded(IPL_TTY, 0, "ttypl");
    }
}

/// `ttymalloc`: allocate a tty structure and its associated buffers, and attach it to the
/// tty list.
pub fn ttymalloc<G: ProcGlue>(baud: i32) -> TtyRef<G> {
    ttypl_init::<G>();
    let t = tables::<G>();
    let Some(tp) = t.ttypl.get_homed(PR_WAITOK, Tty::new) else {
        kpanic(format_args!("ttymalloc: out of memory"));
    };

    let baud = if baud == 0 { 115200 } else { baud };

    tp.t_qlen.set(if baud <= 9600 {
        1024
    } else if baud <= 115200 {
        4096
    } else {
        8192
    });
    tp.t_rawq.clalloc(tp.t_qlen.get(), true);
    tp.t_canq.clalloc(tp.t_qlen.get(), true);
    // output queue doesn't need quoting
    tp.t_outq.clalloc(tp.t_qlen.get(), false);

    {
        let mut w = t.ttylist_lock.write();
        t.ttylist.insert_tail(&mut w, tp.as_pin());
        t.tty_count.fetch_add(1, Ordering::Relaxed);
    }

    tp.t_rstrt_to
        .set_shared(ttrstrt::<G>, PoolArc::erase(tp.clone()), KCLOCK_NONE, 0);

    tp
}

/// `ttyfree`: retire a tty: off the tty list, its restart timeout stopped and its handle
/// given back, its knotes detached (EOF), its rings freed. Its pool item stays for its
/// driver's `&'static` (the module's deviations); the last handle frees it.
pub fn ttyfree<G: ProcGlue>(tp: &Tty<G>) {
    let t = tables::<G>();
    {
        let mut w = t.ttylist_lock.write();
        if tp.tty_link.is_linked() {
            t.ttylist.remove(&mut w, tp);
            let count = t.tty_count.fetch_sub(1, Ordering::Relaxed) - 1;
            crate::assert::kassert!(count >= 0);
        }
    }

    let _ = tp.t_rstrt_to.del_barrier();
    tp.t_rstrt_to.clear();

    {
        let _s = hal::intr::splraise(IPL_TTY);
        tp.t_rsel.si_note.invalidate();
        tp.t_wsel.si_note.invalidate();
    }

    tp.t_rawq.clfree();
    tp.t_canq.clfree();
    tp.t_outq.clfree();
}

/// `tty_count`.
pub fn tty_count<G: ProcGlue>() -> i32 {
    tables::<G>().tty_count.load(Ordering::Relaxed)
}

/// `ttystats_init`: the `struct itty` of every tty, in list order, as the bytes
/// `kern.tty.ttyinfo` copies out (an array of `M_SYSCTL` sized from `tty_count`), and how
/// many ttys it holds (`*ttycp`: those visited, at most the count, fewer when some were
/// freed before the list's lock was taken).
fn ttystats_init<G: ProcGlue>() -> Option<(hal::malloc::MArray<u8>, usize)> {
    let t = tables::<G>();
    let ttyc = usize::try_from(t.tty_count.load(Ordering::Relaxed)).unwrap_or(0);
    let ttystatssiz = ttyc * size_of::<Itty>();
    if ttyc == 0 {
        return None;
    }
    let Some(mut out) =
        hal::malloc::MArray::new(M_SYSCTL, hal::malloc::M_WAITOK, ttystatssiz, |_| 0u8)
    else {
        kpanic(format_args!("ttystats_init: out of memory"));
    };

    let mut ntty = 0;
    let w = t.ttylist_lock.write();
    for (tp, slot) in t
        .ttylist
        .iter(&w)
        .zip(out.as_chunks_mut::<{ size_of::<Itty>() }>().0)
    {
        ntty += 1;
        let (column, session_id, pgrp_id) = {
            let st = tp.st.lock();
            (
                if tp.t_oflag() & OPOST != 0 {
                    st.column
                } else {
                    0
                },
                st.session
                    .as_ref()
                    .and_then(|s| s.leader_pid())
                    .unwrap_or(0),
                st.pgrp.as_ref().map_or(0, |pg| pg.pg_id.get()),
            )
        };
        let itp = Itty {
            t_dev: tp.t_dev.get(),
            t_rawq_c_cc: tp.t_rawq.c_cc.get(),
            t_canq_c_cc: tp.t_canq.c_cc.get(),
            t_outq_c_cc: tp.t_outq.c_cc.get(),
            t_hiwat: tp.t_hiwat.get(),
            t_lowat: tp.t_lowat.get(),
            t_column: column,
            _pad0: 0,
            t_state: tp.t_state.get(),
            t_session_id: session_id,
            t_pgrp_pg_id: pgrp_id,
            t_line: tp.t_line.get(),
            _pad1: [0; 3],
        };
        let _ = pod_to_bytes(&itp, &mut slot[..]);
    }
    drop(w);
    Some((out, ntty))
}

/// `sysctl_tty`: return tty-related information (`kern.tty`).
pub fn sysctl_tty<G: ProcGlue>(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    if name.len() != 1 {
        return Err(Errno::ENOTDIR);
    }

    match name[0] {
        KERN_TTY_TKNIN => rdquad(req, TK_NIN.load(Ordering::Relaxed)),
        KERN_TTY_TKNOUT => rdquad(req, TK_NOUT.load(Ordering::Relaxed)),
        KERN_TTY_TKRAWCC => rdquad(req, TK_RAWCC.load(Ordering::Relaxed)),
        KERN_TTY_TKCANCC => rdquad(req, TK_CANCC.load(Ordering::Relaxed)),
        KERN_TTY_INFO => {
            let stats = ttystats_init::<G>();
            let rows = stats
                .as_ref()
                .map_or(&[][..], |(s, ntty)| &s[..ntty * size_of::<Itty>()]);
            rdstruct(req, rows)
        }
        // NPTY > 0
        _ => crate::pty::sysctl_pty(name, req),
    }
}

impl<G: ProcGlue> SysctlNode for TtyNode<G> {
    /// `sysctl_tty`.
    fn sysctl(&self, name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
        sysctl_tty::<G>(name, req)
    }
}

/// `ttytstamp`: record the time of a modem line transition the user asked for
/// (`TIOCSTSTAMP`).
pub fn ttytstamp<G: ProcGlue>(tp: &Tty<G>, octs: i32, ncts: i32, odcd: i32, ndcd: i32) {
    let mut doit = 0;
    let flags = tp.t_flags.get();

    if ncts ^ octs != 0 {
        doit |= if ncts != 0 {
            flags & TS_TSTAMPCTSSET
        } else {
            flags & TS_TSTAMPCTSCLR
        };
    }
    if ndcd ^ odcd != 0 {
        doit |= if ndcd != 0 {
            flags & TS_TSTAMPDCDSET
        } else {
            flags & TS_TSTAMPDCDCLR
        };
    }

    if doit != 0 {
        // microtime(&tp->t_tv): nanotime truncated to microseconds.
        let tv = Timeval::from_timespec(crate::clock::nanotime());
        tp.st.lock().tv = tv;
    }
}

/// `linesw[tp->t_line]`: the tty's line discipline (an index past the table is a defunct
/// one).
pub fn linesw<G: ProcGlue>(tp: &Tty<G>) -> &'static dyn LineDiscipline<G> {
    match Discipline::<G>::TABLE.get(usize::from(tp.t_line.get())) {
        Some(d) => *d,
        None => Discipline::<G>::DEFUNCT,
    }
}

/// `nullioctl`: do nothing specific version of line discipline specific ioctl command.
pub fn nullioctl<G: ProcGlue>(
    _tp: &Tty<G>,
    _cmd: u64,
    _data: &mut [u8],
    _flags: i32,
    _p: &Thread<G>,
) -> Result<bool, Errno> {
    Ok(false)
}

impl<G: ProcGlue> LineDiscipline<G> for TermiosDisc<G> {
    /// `ttyopen`.
    fn open(&self, dev: Dev, tp: &Tty<G>, p: &Thread<G>) -> Result<(), Errno> {
        ttyopen(dev, tp, p)
    }
    /// `ttylclose`.
    fn close(&self, tp: &Tty<G>, flag: i32, p: Option<&Thread<G>>) -> Result<(), Errno> {
        ttylclose(tp, flag, p)
    }
    /// `ttread`.
    fn read(&self, tp: &Tty<G>, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
        ttread(tp, uio, flag)
    }
    /// `ttwrite`.
    fn write(&self, tp: &Tty<G>, uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
        ttwrite(tp, uio, flag)
    }
    /// `ttyinput`.
    fn rint(&self, c: i32, tp: &Tty<G>) -> i32 {
        ttyinput(c, tp)
    }
    /// `ttstart`.
    fn start(&self, tp: &Tty<G>) -> Result<(), Errno> {
        let _ = ttstart(tp);
        Ok(())
    }
    /// `ttymodem`.
    fn modem(&self, tp: &Tty<G>, carrier: bool) -> bool {
        ttymodem(tp, i32::from(carrier)) != 0
    }
}

impl<G: ProcGlue> LineDiscipline<G> for Defunct<G> {
    /// `ttynodisc`: `enodev`.
    fn open(&self, _dev: Dev, _tp: &Tty<G>, _p: &Thread<G>) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    /// `ttyerrclose`: `enodev`.
    fn close(&self, _tp: &Tty<G>, _flag: i32, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    /// `ttyerrio`: `enodev`.
    fn read(&self, _tp: &Tty<G>, _uio: &mut dyn UioMove, _flag: i32) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    /// `ttyerrio`: `enodev`.
    fn write(&self, _tp: &Tty<G>, _uio: &mut dyn UioMove, _flag: i32) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    /// `ttyerrinput`: `enodev`, as an `int`.
    fn rint(&self, _c: i32, _tp: &Tty<G>) -> i32 {
        Errno::ENODEV as i32
    }
    /// `ttyerrstart`: `enodev`.
    fn start(&self, _tp: &Tty<G>) -> Result<(), Errno> {
        Err(Errno::ENODEV)
    }
    /// `nullmodem`.
    fn modem(&self, tp: &Tty<G>, carrier: bool) -> bool {
        nullmodem(tp, i32::from(carrier)) != 0
    }
}

/// `pgdelete`'s terminal half: the group `pg` going away is forgotten by its session's
/// terminal if it was that terminal's foreground group (`tp->t_pgrp = NULL`). Called without
/// a witness of the kernel lock out (it reads the session under a reader).
pub fn pgrp_gone<G: ProcGlue>(pg: &crate::proc::Pgrp<G>) {
    let tp = {
        let r = klock::reader();
        pg.session(&r).and_then(|s| s.s_ttyp.get(&r).clone())
    };
    let Some(tp) = tp else {
        return;
    };
    let old = {
        let mut st = tp.st.lock();
        if st
            .pgrp
            .as_deref()
            .is_some_and(|p| ptr::eq::<crate::proc::Pgrp<G>>(p, pg))
        {
            st.pgrp.take()
        } else {
            None
        }
    };
    drop(old);
    drop(tp);
}

/// `killjobc`'s terminal half for a session leader whose session has a terminal vnode: if
/// the session's terminal is still the session's, its foreground group gets `SIGHUP` and the
/// terminal is drained (`ttywait`); the caller then revokes the vnode. Whether the terminal
/// was the session's.
pub fn session_hangup_tty<G: ProcGlue>(sess: &crate::proc::Session<G>) -> bool {
    let tp = {
        let r = klock::reader();
        sess.s_ttyp.get(&r).clone()
    };
    let Some(tp) = tp else {
        return false;
    };
    if tp.session_addr() != ptr::from_ref(sess).addr() {
        return false;
    }
    let pg = tp.pgrp();
    if pg.is_some() {
        pgsignal(pg.as_deref(), SIGHUP, true);
    }
    drop(pg);
    let _ = ttywait(&tp);
    true
}

/// `s_ttyp`'s terminal of `sess` (the session's controlling terminal, once it had one).
pub fn session_tty<G: ProcGlue>(sess: &crate::proc::Session<G>) -> Option<TtyRef<G>> {
    let r = klock::reader();
    sess.s_ttyp.get(&r).clone()
}

const _: () = assert!(size_of::<Itty>() == 40);
const _: () = assert!(size_of::<Ptmget>() == 40);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The line discipline over the kernel's test glue: canonical input (erase, kill, word
    // erase, end of file), signals, raw input, output processing, the termios and job
    // control ioctls with real sessions and groups, the driver's hooks, pending input, the
    // kqueue filters through ttkqfilter's counted hook, the disciplines' table, kern.tty,
    // ttyinfo's lines and the tty's lifetime.

    use std::boxed::Box;
    use std::sync::Mutex;
    use std::vec::Vec;

    use hal::sys::signal::sigmask;

    use super::*;
    use crate::event::tests::{Kg, T, new_kqueue, setup, thread_of};
    use crate::event::{test_knote, test_knote_drop};
    use crate::proc::{Pgrp, enternewpgrp, enterthispgrp, pgrp_alloc, session_alloc};
    use crate::termios::{TTYDEF_CFLAG, TTYDEF_IFLAG, TTYDEF_LFLAG, TTYDEF_OFLAG};
    use crate::testutil;

    /// What the device switch saw: the `d_stop` calls; whether `/dev/console`'s access
    /// check (`TIOCCONS`'s) fails.
    struct Devsw {
        stops: Mutex<Vec<i32>>,
        deny: core::sync::atomic::AtomicBool,
    }

    impl TtyDevsw<Kg> for Devsw {
        fn console_tty(&self) -> bool {
            false
        }
        fn console_write(&self, uio: &mut crate::uio::Uio<'_, Kg>) -> Result<(), Errno> {
            uio.uio_resid = 0;
            Ok(())
        }
        fn stop(&self, _tp: &Tty<Kg>, rw: i32) {
            self.stops.lock().unwrap().push(rw);
        }
        fn console_access(&self, _p: &Thread<Kg>) -> Result<(), Errno> {
            if self.deny.load(Ordering::Relaxed) {
                Err(Errno::EACCES)
            } else {
                Ok(())
            }
        }
        fn console_dev(&self) -> Option<Dev> {
            None
        }
    }

    /// The test thread's world: a process `pid` (its thread the `curproc`), the device
    /// switch double registered.
    fn world(pid: i32) -> (T, &'static Devsw) {
        let p = setup(pid);
        crate::sig::test_sigacts(p.process());
        crate::sig::siginit(p.process());
        // The thread on its process's list, where a process signal finds it.
        {
            let pr = p.process();
            let mut g = pr.ps_mtx.lock();
            pr.ps_threads.insert_tail(&mut g, hal::pin::pin_static(p));
        }
        p.process().ps_threadcnt.set(1);
        Kg::set_curproc(Some(p));
        let d: &'static Devsw = Box::leak(Box::new(Devsw {
            stops: Mutex::new(Vec::new()),
            deny: core::sync::atomic::AtomicBool::new(false),
        }));
        register_ttydevsw::<Kg>(d);
        (p, d)
    }

    /// A tty set up as a driver's open would: default termios, open, carrier on.
    fn test_tty() -> TtyRef<Kg> {
        let tp = ttymalloc::<Kg>(9600);
        tp.t_dev.set(hal::sys::types::makedev(12, 0));
        ttychars(&tp);
        tp.set_t_iflag(TTYDEF_IFLAG);
        tp.set_t_oflag(TTYDEF_OFLAG);
        tp.set_t_lflag(TTYDEF_LFLAG);
        tp.set_t_cflag(TTYDEF_CFLAG);
        tp.set_t_ispeed(9600);
        tp.set_t_ospeed(9600);
        ttsetwater(&tp);
        tp.t_state_set(TS_ISOPEN | TS_CARR_ON);
        tp
    }

    fn type_in(tp: &Tty<Kg>, s: &[u8]) {
        for &c in s {
            ttyinput(i32::from(c), tp);
        }
    }

    fn drain(q: &Clist) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let c = q.getc();
            if c < 0 {
                return out;
            }
            out.push(c as u8);
        }
    }

    /// A uio over a buffer: a read's destination or a write's source.
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

    /// Reads up to `n` bytes through `ttread`, without waiting.
    fn read(tp: &Tty<Kg>, n: usize) -> (Result<(), Errno>, Vec<u8>) {
        let mut b = Buf {
            data: std::vec![0; n],
            pos: 0,
            write: false,
        };
        let r = ttread(tp, &mut b, IO_NDELAY);
        let got = b.pos;
        b.data.truncate(got);
        (r, b.data)
    }

    /// Writes `s` through `ttwrite`; the result and the bytes left.
    fn write(tp: &Tty<Kg>, s: &[u8], flag: i32) -> (Result<(), Errno>, usize) {
        let mut b = Buf {
            data: s.to_vec(),
            pos: 0,
            write: true,
        };
        let r = ttwrite(tp, &mut b, flag);
        (r, b.resid())
    }

    fn ioctl_int(tp: &Tty<Kg>, cmd: u64, v: i32, p: T) -> (Result<bool, Errno>, i32) {
        let mut d = v.to_ne_bytes();
        let r = ttioctl(tp, cmd, &mut d, 0, p);
        (r, i32::from_ne_bytes(d))
    }

    /// `setsid()` for `p`'s process: a new session and group it leads.
    fn setsid(p: T) {
        enternewpgrp(
            p.process_ref(),
            pgrp_alloc::<Kg>(()),
            Some(session_alloc::<Kg>(())),
        );
    }

    /// The group `pr` is in.
    fn pgrp_of(p: T) -> crate::proc::PgrpRef<Kg> {
        let r = klock::reader();
        p.process().pgrp(&r).cloned().unwrap()
    }

    #[test]
    fn canonical_input_erases_and_breaks_lines() {
        let _w = world(9001);
        let tp = test_tty();
        type_in(&tp, b"abc\x7fd\r");
        // ICRNL made the carriage return a newline, which moved the line to the canonical
        // queue.
        assert_eq!(tp.t_rawq.c_cc.get(), 0);
        assert_eq!(drain(&tp.t_canq), b"abd\n");
        // Echo: the characters, the ECHOE rubout of 'c', and ONLCR's "\r\n".
        assert_eq!(drain(&tp.t_outq), b"abc\x08 \x08d\r\n");
        assert_eq!(tp.column(), 0);
        assert!(
            testutil::randomness_enqueued() >= 6,
            "every character is entropy"
        );
    }

    #[test]
    fn kill_and_word_erase_rub_out() {
        let _w = world(9002);
        let tp = test_tty();
        type_in(&tp, b"xyz");
        ttyinput(i32::from(tp.t_cc(VKILL)), &tp);
        assert_eq!(tp.t_rawq.c_cc.get(), 0);
        assert_eq!(tp.t_canq.c_cc.get(), 0);
        assert_eq!(drain(&tp.t_outq), b"xyz\x08 \x08\x08 \x08\x08 \x08");
        // ^W erases the last word and the blanks after it.
        type_in(&tp, b"one two  ");
        ttyinput(i32::from(tp.t_cc(VWERASE)), &tp);
        type_in(&tp, b"\n");
        assert_eq!(drain(&tp.t_canq), b"one \n");
    }

    #[test]
    fn end_of_file_ends_a_read_without_being_read() {
        let _w = world(9003);
        let tp = test_tty();
        type_in(&tp, b"ab");
        ttyinput(i32::from(tp.t_cc(VEOF)), &tp);
        // ^D echoes as "^D" and backs over it.
        assert_eq!(drain(&tp.t_outq), b"ab^D\x08\x08");
        let (r, got) = read(&tp, 16);
        assert_eq!(r, Ok(()));
        assert_eq!(got, b"ab");
        // A ^D alone is a read of 0 bytes: end of file.
        ttyinput(i32::from(tp.t_cc(VEOF)), &tp);
        let (r, got) = read(&tp, 16);
        assert_eq!(r, Ok(()));
        assert!(got.is_empty());
        // Nothing left: a non-blocking read would block.
        let (r, _) = read(&tp, 16);
        assert_eq!(r, Err(Errno::EWOULDBLOCK));
    }

    #[test]
    fn a_read_stops_at_the_end_of_the_line() {
        let _w = world(9004);
        let tp = test_tty();
        type_in(&tp, b"one\ntwo\n");
        assert_eq!(read(&tp, 64).1, b"one\n");
        assert_eq!(read(&tp, 2).1, b"tw");
        assert_eq!(read(&tp, 64).1, b"o\n");
    }

    #[test]
    fn interrupt_flushes_echoes_and_signals_the_foreground_group() {
        let (p, _d) = world(9005);
        setsid(p);
        let tp = test_tty();
        tp.st.lock().pgrp = Some(pgrp_of(p));
        type_in(&tp, b"partial");
        let _ = drain(&tp.t_outq);
        ttyinput(i32::from(tp.t_cc(VINTR)), &tp);
        assert_eq!(tp.t_rawq.c_cc.get(), 0);
        assert_eq!(drain(&tp.t_outq), b"^C");
        // pgsignal(.., 1) signals only the members with a controlling terminal.
        assert_eq!(p.process().ps_siglist.load(Ordering::Relaxed), 0);
        p.process().ps_flags.fetch_or(PS_CONTROLT, Ordering::SeqCst);
        ttyinput(i32::from(tp.t_cc(VQUIT)), &tp);
        let pending =
            p.process().ps_siglist.load(Ordering::Relaxed) | p.p_siglist.load(Ordering::Relaxed);
        assert_ne!(pending & sigmask(SIGQUIT), 0);
    }

    #[test]
    fn raw_mode_hands_out_characters_at_once() {
        let (p, _d) = world(9006);
        let tp = test_tty();
        tp.t_lflag_clr(ICANON | ECHO);
        type_in(&tp, b"q");
        assert_eq!(tp.t_canq.c_cc.get(), 0);
        assert_eq!(tp.t_rawq.c_cc.get(), 1);
        assert_eq!(tp.t_outq.c_cc.get(), 0, "no echo");
        assert_eq!(ioctl_int(&tp, FIONREAD, 0, p), (Ok(true), 1));
        assert_eq!(read(&tp, 8).1, b"q");
        // VMIN 0, VTIME 0: a read never waits.
        let mut cc = tp.t_cc_all();
        cc[VMIN as usize] = 0;
        tp.set_t_cc_all(cc);
        let mut b = Buf {
            data: std::vec![0; 4],
            pos: 0,
            write: false,
        };
        assert_eq!(ttread(&tp, &mut b, 0), Ok(()));
        assert_eq!(b.pos, 0);
    }

    #[test]
    fn literal_next_quotes_the_erase_character() {
        let _w = world(9007);
        let tp = test_tty();
        type_in(&tp, b"a\x16\x7f\n");
        assert_eq!(drain(&tp.t_canq), b"a\x7f\n");
    }

    #[test]
    fn output_processing_and_tabs() {
        let _w = world(9008);
        let tp = test_tty();
        assert_eq!(write(&tp, b"hi\tx\n", 0), (Ok(()), 0));
        assert_eq!(drain(&tp.t_outq), b"hi\tx\r\n");
        assert_eq!(tp.column(), 0);
        // With OXTABS the tab becomes spaces to the next stop.
        tp.set_t_oflag(tp.t_oflag() | OXTABS);
        assert_eq!(ttyoutput(i32::from(b'\t'), &tp), -1);
        assert_eq!(drain(&tp.t_outq), b"        ");
        // Without OPOST nothing is translated.
        tp.set_t_oflag(tp.t_oflag() & !OPOST);
        assert_eq!(ttyoutput(i32::from(b'\n'), &tp), -1);
        assert_eq!(drain(&tp.t_outq), b"\n");
        // FLUSHO: the write is consumed and nothing queued.
        tp.t_lflag_set(FLUSHO);
        assert_eq!(write(&tp, b"lost", 0), (Ok(()), 0));
        assert_eq!(tp.t_outq.c_cc.get(), 0);
    }

    #[test]
    fn a_full_output_queue_gives_back_what_was_not_queued() {
        let _w = world(9009);
        let tp = test_tty();
        // No driver drains the queue: past the high water mark a non-blocking write stops
        // and gives the rest back.
        let big = std::vec![b'z'; 2000];
        let (r, left) = write(&tp, &big, IO_NDELAY);
        assert_eq!(r, Ok(()));
        assert!(left > 0 && left < 2000);
        assert_eq!(tp.t_outq.c_cc.get() as usize + left, 2000);
        // Nothing fits now: EWOULDBLOCK, all of it given back.
        assert_eq!(write(&tp, b"more", IO_NDELAY), (Err(Errno::EWOULDBLOCK), 4));
    }

    #[test]
    fn termios_and_window_size_ioctls() {
        let (p, _d) = world(9010);
        let tp = test_tty();
        let mut t = [0u8; size_of::<Termios>()];
        assert_eq!(ttioctl(&tp, TIOCGETA, &mut t, 0, p), Ok(true));
        let mut termios: Termios = pod_from_bytes(&t).unwrap();
        assert_eq!(termios.c_lflag, TTYDEF_LFLAG);
        termios.c_lflag &= !ECHO;
        termios.c_cc[VERASE as usize] = 0x08;
        assert!(pod_to_bytes(&termios, &mut t));
        assert_eq!(ttioctl(&tp, TIOCSETA, &mut t, 0, p), Ok(true));
        assert_eq!(tp.t_lflag() & ECHO, 0);
        assert_eq!(tp.t_cc(VERASE), 0x08);
        type_in(&tp, b"ab\x08c\n");
        assert_eq!(drain(&tp.t_canq), b"ac\n");
        assert_eq!(tp.t_outq.c_cc.get(), 0, "ECHO is off");
        // A negative speed is refused.
        termios.c_ospeed = -1;
        assert!(pod_to_bytes(&termios, &mut t));
        assert_eq!(ttioctl(&tp, TIOCSETA, &mut t, 0, p), Err(Errno::EINVAL));

        let ws = Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let mut buf = [0u8; size_of::<Winsize>()];
        assert!(pod_to_bytes(&ws, &mut buf));
        assert_eq!(ttioctl(&tp, TIOCSWINSZ, &mut buf, 0, p), Ok(true));
        let mut out = [0u8; size_of::<Winsize>()];
        assert_eq!(ttioctl(&tp, TIOCGWINSZ, &mut out, 0, p), Ok(true));
        assert_eq!(pod_from_bytes::<Winsize>(&out), Some(ws));

        // Not a tty ioctl.
        assert_eq!(ioctl_int(&tp, 0x2000_7400, 0, p).0, Ok(false));
        // TIOCOUTQ, TIOCEXCL/NXCL, TIOCGETD.
        assert_eq!(ioctl_int(&tp, TIOCOUTQ, 0, p), (Ok(true), 0));
        assert_eq!(ioctl_int(&tp, TIOCEXCL, 0, p).0, Ok(true));
        assert!(tp.t_state_isset(TS_XCLUDE));
        assert_eq!(ioctl_int(&tp, TIOCNXCL, 0, p).0, Ok(true));
        assert!(!tp.t_state_isset(TS_XCLUDE));
        assert_eq!(ioctl_int(&tp, TIOCGETD, 7, p), (Ok(true), 0));
    }

    #[test]
    fn turning_icanon_on_requeues_the_raw_input_in_order_with_its_quotes() {
        let (p, _d) = world(9011);
        let tp = test_tty();
        tp.t_lflag_clr(ICANON | ECHO);
        // Raw input with a quoted erase (LNEXT works in raw mode too).
        type_in(&tp, b"ab\x16\x7fc");
        assert_eq!(tp.t_rawq.c_cc.get(), 4);
        // ICANON on: PENDIN, and the next read takes the pending input first.
        let mut t = [0u8; size_of::<Termios>()];
        assert_eq!(ttioctl(&tp, TIOCGETA, &mut t, 0, p), Ok(true));
        let mut termios: Termios = pod_from_bytes(&t).unwrap();
        termios.c_lflag |= ICANON;
        assert!(pod_to_bytes(&termios, &mut t));
        assert_eq!(ttioctl(&tp, TIOCSETA, &mut t, 0, p), Ok(true));
        assert_ne!(tp.t_lflag() & PENDIN, 0);
        type_in(&tp, b"\n");
        assert_eq!(tp.t_lflag() & PENDIN, 0);
        // The quoted DEL was not an erase: it stays, in order.
        assert_eq!(drain(&tp.t_canq), b"ab\x7fc\n");
        // ICANON off: the canonical queue's line moves back to the raw queue.
        type_in(&tp, b"xy\n");
        termios.c_lflag &= !ICANON;
        assert!(pod_to_bytes(&termios, &mut t));
        assert_eq!(ttioctl(&tp, TIOCSETA, &mut t, 0, p), Ok(true));
        assert_eq!(tp.t_canq.c_cc.get(), 0);
        assert_eq!(drain(&tp.t_rawq), b"xy\n");
    }

    /// The calls a driver of the tests saw.
    static DRIVER_LOG: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

    struct RecDriver;

    impl TtyDriver<Kg> for RecDriver {
        fn start(&self, tp: &Tty<Kg>) {
            DRIVER_LOG.lock().unwrap().push("start");
            // A driver's start runs with the state free: it may read it.
            let _ = tp.termios();
        }
        fn param(&self, _tp: &Tty<Kg>, t: &Termios) -> Result<(), Errno> {
            DRIVER_LOG.lock().unwrap().push("param");
            if t.c_ispeed == 1234 {
                Err(Errno::EINVAL)
            } else {
                Ok(())
            }
        }
        fn hwiflow(&self, _tp: &Tty<Kg>, block: bool) -> bool {
            DRIVER_LOG
                .lock()
                .unwrap()
                .push(if block { "block" } else { "unblock" });
            true
        }
    }

    static REC: RecDriver = RecDriver;

    fn oproc_table(_tp: &Tty<Kg>) {
        DRIVER_LOG.lock().unwrap().push("oproc");
    }

    #[test]
    fn the_driver_is_called_with_the_state_free() {
        let (p, d) = world(9012);
        let tp = test_tty();
        DRIVER_LOG.lock().unwrap().clear();
        assert!(!tp.has_driver());
        // No driver: ttstart does nothing.
        ttstart(&tp);
        tp.set_driver(&REC);
        assert!(tp.has_driver());
        type_in(&tp, b"x");
        // TIOCSETA asks the driver first; its error stops the change.
        let mut t = [0u8; size_of::<Termios>()];
        let mut termios = tp.termios();
        termios.c_ispeed = 1234;
        assert!(pod_to_bytes(&termios, &mut t));
        assert_eq!(ttioctl(&tp, TIOCSETA, &mut t, 0, p), Err(Errno::EINVAL));
        assert_eq!(tp.t_ispeed(), 9600);
        // Hardware flow control under CHWFLOW: the input queue past half its size blocks.
        tp.set_t_cflag(tp.t_cflag() | CHWFLOW);
        tp.t_lflag_clr(ICANON);
        let half = std::vec![b'h'; 600];
        type_in(&tp, &half);
        assert!(tp.t_state_isset(TS_TBLOCK));
        let _ = read(&tp, 2000);
        assert!(
            !tp.t_state_isset(TS_TBLOCK),
            "a read below a fifth unblocks"
        );
        let log = DRIVER_LOG.lock().unwrap().clone();
        assert_eq!(log[0], "start");
        assert!(log.contains(&"param"));
        assert!(log.contains(&"block") && log.contains(&"unblock"));
        // ^S stops output through the device switch's d_stop(tp, 0).
        tp.set_t_iflag(tp.t_iflag() | IXON);
        ttyinput(i32::from(tp.t_cc(VSTOP)), &tp);
        assert!(tp.t_state_isset(TS_TTSTOP));
        assert!(d.stops.lock().unwrap().contains(&0));
        // An inherited driver's function table.
        tp.clear_driver();
        tp.set_oproc(Some(oproc_table));
        assert!(tp.has_driver());
        ttyinput(i32::from(tp.t_cc(VSTART)), &tp);
        assert_eq!(DRIVER_LOG.lock().unwrap().last(), Some(&"oproc"));
    }

    #[test]
    fn ixoff_sends_the_stop_and_start_characters() {
        let _w = world(9013);
        let tp = test_tty();
        tp.set_t_iflag(tp.t_iflag() | IXOFF);
        tp.t_lflag_clr(ICANON | ECHO);
        let lots = std::vec![b'l'; 520];
        type_in(&tp, &lots);
        assert!(tp.t_state_isset(TS_TBLOCK));
        assert!(drain(&tp.t_outq).contains(&tp.t_cc(VSTOP)));
        let _ = read(&tp, 1000);
        assert!(!tp.t_state_isset(TS_TBLOCK));
        assert_eq!(drain(&tp.t_outq), [tp.t_cc(VSTART)]);
    }

    #[test]
    fn controlling_terminal_and_foreground_group_ioctls() {
        let (p, _d) = world(9014);
        let tp = test_tty();
        // Not in a session it leads: EPERM.
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Err(Errno::EPERM));
        // Not the controlling terminal: ENOTTY, but for the superuser's TIOCGPGRP.
        assert_eq!(ioctl_int(&tp, TIOCGPGRP, 0, p), (Ok(true), NO_PID));
        assert_eq!(ioctl_int(&tp, TIOCSPGRP, 9014, p).0, Err(Errno::ENOTTY));
        assert_eq!(ioctl_int(&tp, TIOCGSID, 0, p).0, Err(Errno::ENOTTY));

        setsid(p);
        let sess = {
            let r = klock::reader();
            p.process().session(&r).map(|s| ptr::from_ref(s).addr())
        };
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Ok(true));
        assert_eq!(tp.session_addr(), sess.unwrap());
        assert!(isctty(p.process(), &tp));
        assert_eq!(ioctl_int(&tp, TIOCGPGRP, 0, p), (Ok(true), 9014));
        assert_eq!(ioctl_int(&tp, FIOGETOWN, 0, p), (Ok(true), -9014));
        assert_eq!(ioctl_int(&tp, TIOCGSID, 0, p), (Ok(true), 9014));
        // The session remembers its terminal (counted).
        let r = klock::reader();
        let s_ttyp = p
            .process()
            .session(&r)
            .and_then(|s| s.s_ttyp.get(&r).clone())
            .unwrap();
        drop(r);
        assert!(PoolArc::ptr_eq(&s_ttyp, &tp));
        drop(s_ttyp);

        // TIOCSPGRP: no such group EINVAL; a group of another session EPERM.
        assert_eq!(ioctl_int(&tp, TIOCSPGRP, 4242, p).0, Err(Errno::EINVAL));
        let other = thread_of(9015);
        setsid(other);
        assert_eq!(ioctl_int(&tp, TIOCSPGRP, 9015, p).0, Err(Errno::EPERM));
        // FIOSETOWN of a pid that does not exist: ESRCH.
        assert_eq!(ioctl_int(&tp, FIOSETOWN, 4243, p).0, Err(Errno::ESRCH));
        assert_eq!(ioctl_int(&tp, TIOCSPGRP, 9014, p).0, Ok(true));

        // Another session's leader cannot take the terminal.
        let mut z = [0u8; 4];
        assert_eq!(ttioctl(&tp, TIOCSCTTY, &mut z, 0, other), Err(Errno::EPERM));

        // ttyclose forgets the group and the session and bumps the generation.
        let gen_ = tp.t_gen.get();
        assert_eq!(ttyclose(&tp), Ok(()));
        assert_eq!(tp.t_gen.get(), gen_.wrapping_add(1));
        assert_eq!(tp.session_addr(), 0);
        assert!(tp.pgrp().is_none());
    }

    /// `uprintf` writes to the controlling terminal of a process that has one: `PS_CONTROLT`
    /// and a session whose terminal has a vnode (`s_ttyvp`); to nothing otherwise, as the
    /// children of diff-openbsd's `syscalls/entry` (`setsid`, no terminal).
    #[test]
    fn uprintf_reaches_only_a_controlling_terminal() {
        let (p, _d) = world(9030);
        let pr = p.process();
        // The session's terminal vnode (`s_ttyvp`), set and cleared by hand (TIOCSCTTY
        // through the vnode layer sets it on the machine).
        let tty_vnode =
            |on: bool| {
                let r = klock::reader();
                if let Some(s) = pr.session(&r) {
                    s.ttyvp.set(on.then(|| {
                        &*Box::leak(Box::new(crate::fs::vnode::Vnode::<Kg>::new_unpooled()))
                    }));
                }
            };
        // No session terminal yet: nothing.
        uprintf(pr, format_args!("lost\n"));
        setsid(p);
        let tp = test_tty();
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Ok(true));
        tty_vnode(true);
        assert_ne!(pr.ps_flags.load(Ordering::Relaxed) & PS_CONTROLT, 0);
        assert_eq!(tp.t_outq.c_cc.get(), 0);

        // The terminal: the line, its newline as `tputchar` puts it (its own CR, then
        // `ttyoutput`'s ONLCR pair), as the C's.
        uprintf(pr, format_args!("[{}]{}: pin\n", "sh", 9030));
        assert_eq!(drain(&tp.t_outq), b"[sh]9030: pin\r\r\n");

        // A session whose terminal has no vnode (revoked): nothing.
        tty_vnode(false);
        uprintf(pr, format_args!("lost\n"));
        assert_eq!(tp.t_outq.c_cc.get(), 0);

        // PS_CONTROLT clear (setsid's child, a daemon): nothing.
        tty_vnode(true);
        pr.ps_flags.fetch_and(!PS_CONTROLT, Ordering::SeqCst);
        uprintf(pr, format_args!("lost\n"));
        assert_eq!(tp.t_outq.c_cc.get(), 0);
        tty_vnode(false);
    }

    #[test]
    fn pgrp_gone_forgets_only_the_terminal_s_own_group() {
        let (p, _d) = world(9016);
        setsid(p);
        let tp = test_tty();
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Ok(true));
        let pg = pgrp_of(p);
        // Another group of no session's terminal changes nothing.
        let lone: crate::proc::PgrpRef<Kg> = pgrp_alloc::<Kg>(());
        pgrp_gone::<Kg>(&lone);
        assert!(tp.pgrp().is_some());
        pgrp_gone::<Kg>(&pg);
        assert!(tp.pgrp().is_none());
    }

    #[test]
    fn a_background_reader_of_an_orphaned_group_gets_eio() {
        let (p, _d) = world(9017);
        setsid(p);
        let tp = test_tty();
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Ok(true));
        // Another group of the session (orphaned: no parent outside it) in the foreground.
        let q = thread_of(9018);
        enterthispgrp(q.process_ref(), pgrp_of(p));
        enternewpgrp(q.process_ref(), pgrp_alloc::<Kg>(()), None);
        let qpg = pgrp_of(q);
        assert_eq!(ioctl_int(&tp, TIOCSPGRP, 9018, p).0, Ok(true));
        assert!(isbackground(p.process(), &tp));
        assert_eq!(qpg.pg_jobc.get(), 0);
        // The reader's own group is orphaned: EIO, no signal.
        assert_eq!(read(&tp, 4).0, Err(Errno::EIO));
        // A modifying ioctl from the background: EIO too.
        assert_eq!(ioctl_int(&tp, TIOCFLUSH, 0, p).0, Err(Errno::EIO));
    }

    #[test]
    fn line_disciplines_and_tiocsetd() {
        let (p, _d) = world(9019);
        let tp = test_tty();
        assert_eq!(Discipline::<Kg>::TABLE.len(), NLINESW);
        let defunct = Discipline::<Kg>::TABLE[1];
        assert_eq!(defunct.open(0, &tp, p), Err(Errno::ENODEV));
        assert_eq!(defunct.close(&tp, 0, Some(p)), Err(Errno::ENODEV));
        assert_eq!(defunct.rint(i32::from(b'a'), &tp), Errno::ENODEV as i32);
        assert_eq!(defunct.start(&tp), Err(Errno::ENODEV));
        let mut d = [0u8; 4];
        assert_eq!(defunct.ioctl(&tp, TIOCGETA, &mut d, 0, p), Ok(false));
        // TIOCSETD past the table: ENXIO; to a defunct one: its open's ENODEV, the termios
        // discipline reopened.
        assert_eq!(ioctl_int(&tp, TIOCSETD, 10, p).0, Err(Errno::ENXIO));
        assert_eq!(ioctl_int(&tp, TIOCSETD, -1, p).0, Err(Errno::ENXIO));
        assert_eq!(ioctl_int(&tp, TIOCSETD, 5, p).0, Err(Errno::ENODEV));
        assert_eq!(tp.t_line.get(), 0);
        assert!(tp.t_state_isset(TS_ISOPEN));
        // An index past the table is a defunct discipline.
        tp.t_line.set(200);
        assert_eq!(linesw(&tp).rint(0, &tp), Errno::ENODEV as i32);
        tp.t_line.set(0);
        assert_eq!(linesw(&tp).rint(i32::from(b'k'), &tp), 0);
        assert_eq!(tp.t_rawq.c_cc.get(), 1, "slot 0 is the termios discipline");
    }

    #[test]
    fn carrier_loss_hangs_up_the_leader() {
        let (p, _d) = world(9020);
        setsid(p);
        let tp = test_tty();
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Ok(true));
        tp.set_t_cflag(tp.t_cflag() & !CLOCAL);
        type_in(&tp, b"gone");
        assert_eq!(ttymodem(&tp, 0), 0, "turn the line off");
        assert!(!tp.t_state_isset(TS_CARR_ON));
        assert_eq!(tp.t_rawq.c_cc.get(), 0, "flushed");
        let pending =
            p.process().ps_siglist.load(Ordering::Relaxed) | p.p_siglist.load(Ordering::Relaxed);
        assert_ne!(pending & sigmask(SIGHUP), 0);
        // A read without carrier on an open line: 0 bytes (EOF).
        assert_eq!(read(&tp, 4), (Ok(()), Vec::new()));
        // A write: EIO.
        assert_eq!(write(&tp, b"x", 0).0, Err(Errno::EIO));
        assert_eq!(ttymodem(&tp, 1), 1);
        assert!(tp.t_state_isset(TS_CARR_ON));
    }

    #[test]
    fn kqueue_filters_follow_input_output_and_carrier() {
        let (p, _d) = world(9021);
        let (_kfd, kq) = new_kqueue(p);
        let tp = test_tty();

        let h = test_knote(&kq, 1);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_READ);
        let count = PoolArc::count(&tp);
        assert_eq!(ttkqfilter(&tp, kn), Ok(()));
        assert_eq!(PoolArc::count(&tp), count + 1, "the knote holds the tty");
        assert_eq!(kn.fop().f_flags, FILTEROP_ISFD);

        // Canonical input: nothing to read until the line ends; then ttwakeup's selwakeup
        // queues the knote with the line's length.
        type_in(&tp, b"ab");
        assert_eq!(kq.count(), 0);
        type_in(&tp, b"\n");
        assert_eq!(kq.count(), 1);
        assert_eq!(kn.kn_data().get(), 3);

        // The carrier drops on a line that watches it: EOF for the reader.
        tp.set_t_cflag(tp.t_cflag() & !CLOCAL);
        tp.t_state_clr(TS_CARR_ON);
        assert!((kn.fop().f_event.unwrap())(kn, 0));
        assert!(kn.has_flags(EV_EOF) && !kn.has_flags(__EV_HUP));
        tp.t_state_set(TS_CARR_ON);
        assert!(
            (kn.fop().f_event.unwrap())(kn, 0),
            "the line is still readable"
        );
        assert!(!kn.has_flags(EV_EOF));
        (kn.fop().f_detach.unwrap())(kn);
        assert!(tp.t_rsel.si_note.is_empty());
        test_knote_drop(&kq, h);

        // EVFILT_WRITE: the room of the output queue, writable below the low water mark.
        let h = test_knote(&kq, 2);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_WRITE);
        assert_eq!(ttkqfilter(&tp, kn), Ok(()));
        let _ = drain(&tp.t_outq);
        assert!((kn.fop().f_event.unwrap())(kn, 0));
        assert_eq!(kn.kn_data().get(), i64::from(tp.t_outq.c_cn.get()));
        (kn.fop().f_detach.unwrap())(kn);
        test_knote_drop(&kq, h);

        // EVFILT_EXCEPT is poll's only.
        let h = test_knote(&kq, 3);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_EXCEPT);
        assert_eq!(ttkqfilter(&tp, kn), Err(Errno::EINVAL));
        test_knote_drop(&kq, h);
    }

    #[test]
    fn ttyfree_retires_the_tty() {
        // (A knote of the tests' kqueue stays KN_PROCESSING for its life, which
        // klist_invalidate waits out: the invalidation itself is kernel::event's, tested
        // there; here the klists are empty.)
        let _w = world(9022);
        let before = tty_count::<Kg>();
        let tp = test_tty();
        assert_eq!(tty_count::<Kg>(), before + 1);
        let count = PoolArc::count(&tp);
        ttyfree(&tp);
        assert_eq!(tty_count::<Kg>(), before);
        assert!(!tp.tty_link.is_linked());
        assert_eq!(
            PoolArc::count(&tp),
            count - 1,
            "the restart timeout's handle went"
        );
        assert_eq!(tp.t_rawq.getc(), -1, "the rings are gone");
        // A second free finds it off the list.
        ttyfree(&tp);
        assert_eq!(tty_count::<Kg>(), before);
    }

    #[test]
    fn kern_tty_answers_as_sysctl_tty() {
        let _w = world(9023);
        let tp = test_tty();
        tp.t_line.set(0);
        type_in(&tp, b"st");
        let ubuf = testutil::USER_BASE + 0xa000;

        let mut len = 8;
        let mut req = SysctlReq {
            oldp: ubuf,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        let node = TtyNode::<Kg>::NODE;
        assert_eq!(node.sysctl(&[KERN_TTY_TKNIN], &mut req), Ok(()));
        let nin = i64::from_ne_bytes(testutil::user_bytes(ubuf, 8).unwrap().try_into().unwrap());
        assert!(nin >= 2);

        // A size query of the ttys' array: a multiple of struct itty.
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[KERN_TTY_INFO], &mut req), Ok(()));
        assert_eq!(len, size_of::<Itty>() * tty_count::<Kg>() as usize);
        assert!(len >= 40 && len % 40 == 0);
        // The array: this tty's row, last on the list.
        let mut len = 4096;
        let mut req = SysctlReq {
            oldp: ubuf,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[KERN_TTY_INFO], &mut req), Ok(()));
        let bytes = testutil::user_bytes(ubuf, len).unwrap();
        let row: Itty = pod_from_bytes(&bytes[len - 40..]).unwrap();
        assert_eq!(row.t_dev, tp.t_dev.get());
        assert_eq!(row.t_rawq_c_cc, 2);
        assert_eq!(row.t_state, TS_ISOPEN | TS_CARR_ON);
        assert_eq!(row.t_column, 2, "OPOST: the echo's column");
        assert_eq!((row._pad0, row._pad1), (0, [0; 3]));
        // A count ahead of the list (a tty freed between the snapshot and the list's lock):
        // only the rows visited, no zeroed row past them (tty.c's `*ttycp = ntty`).
        let listed = tty_count::<Kg>() as usize;
        tables::<Kg>().tty_count.fetch_add(1, Ordering::Relaxed);
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[KERN_TTY_INFO], &mut req), Ok(()));
        tables::<Kg>().tty_count.fetch_sub(1, Ordering::Relaxed);
        assert_eq!(len, size_of::<Itty>() * listed);

        let mut len = 8;
        let mut req = SysctlReq {
            oldp: ubuf,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[1, 2], &mut req), Err(Errno::ENOTDIR));
        assert_eq!(node.sysctl(&[6], &mut req), Err(Errno::EOPNOTSUPP));
        // A new value of a read-only leaf.
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: ubuf,
            newlen: 8,
        };
        assert_eq!(node.sysctl(&[KERN_TTY_TKNOUT], &mut req), Err(Errno::EPERM));
    }

    #[test]
    fn ttyinfo_prints_the_c_s_lines() {
        let (p, _d) = world(9024);
        let tp = test_tty();
        // No session.
        ttyinfo(&tp);
        // (The load average is the scheduler's, which other tests move.)
        let line = drain(&tp.t_outq);
        assert!(line.starts_with(b"load: "));
        // tputchar's CR before a newline, then ONLCR's (the C's).
        assert!(
            line.ends_with(b" not a controlling terminal\r\r\n"),
            "{line:?}"
        );
        setsid(p);
        assert_eq!(ioctl_int(&tp, TIOCSCTTY, 0, p).0, Ok(true));
        p.process().set_comm(b"sh");
        p.set_stat(SONPROC);
        ttyinfo(&tp);
        let line = drain(&tp.t_outq);
        let text = core::str::from_utf8(&line).unwrap();
        assert!(text.starts_with("load: "), "{text}");
        assert!(
            text.contains("  cmd: sh 9024 [running] 0.00u 0.00s 0% "),
            "{text}"
        );
        assert!(text.ends_with("k\r\r\n"), "{text}");
        // ^T in canonical mode (stty status ^T: the default is disabled): the group gets
        // SIGINFO and the same report.
        let mut cc = tp.t_cc_all();
        cc[VSTATUS as usize] = ctrl(b't');
        tp.set_t_cc_all(cc);
        type_in(&tp, &[ctrl(b't')]);
        let again = drain(&tp.t_outq);
        assert!(
            std::string::String::from_utf8_lossy(&again).contains(" cmd: sh 9024 "),
            "{again:?}"
        );
    }

    #[test]
    fn the_tty_types_are_shared_and_the_pool_item_size_is_pinned() {
        const fn shared<T: Send + Sync>() {}
        shared::<Tty<Kg>>();
        shared::<TtyTables<Kg>>();
        let _w = world(9025);
        let _tp = test_tty();
        // vmstat -m's new ttypl row (decision 2 of N4.7), the user's rule: sizes pinned.
        let size = Kg::tables().tty.ttypl.item_size().unwrap();
        assert_eq!(size, 912);
    }

    #[test]
    fn char_type_classes_and_speed_tables() {
        assert_eq!(cclass(i32::from(b'\n')), NEWLINE);
        assert_eq!(cclass(i32::from(b'\t')), TAB);
        assert_eq!(cclass(i32::from(b'\r')), RETURN);
        assert_eq!(cclass(0x08), BACKSPACE);
        assert_eq!(cclass(i32::from(b'a')), ORDINARY);
        assert_eq!(cclass(0x7f), CONTROL);
        assert_ne!(isalpha(i32::from(b'_')), 0);
        assert_eq!(isalpha(i32::from(b'-')), 0);
        assert_eq!(CHAR_TYPE[0x80], ORDINARY | ALPHA);

        let table = [
            Speedtab {
                sp_speed: 9600,
                sp_code: 12,
            },
            Speedtab {
                sp_speed: 115200,
                sp_code: 1,
            },
            Speedtab {
                sp_speed: -1,
                sp_code: -1,
            },
        ];
        assert_eq!(ttspeedtab(115200, &table), 1);
        assert_eq!(ttspeedtab(300, &table), -1);

        let _w = world(9026);
        let tp = test_tty();
        // 9600 bps: 960 cps; low water 480 clamped to 256, high 256 + 960 clamped to
        // 1024 - 200.
        assert_eq!(tp.t_lowat.get(), 256);
        assert_eq!(tp.t_hiwat.get(), 824);
    }

    /// Whether `constty` is `tp`.
    fn constty_is(tp: &Tty<Kg>) -> bool {
        constty::<Kg>().is_some_and(|c| ptr::eq::<Tty<Kg>>(&*c, tp))
    }

    #[test]
    fn tioccons_is_refused_while_another_live_tty_has_it_and_checks_the_console() {
        let (p, d) = world(4710);
        let a = test_tty();
        let b = test_tty();
        assert_eq!(ioctl_int(&a, TIOCCONS, 1, p).0, Ok(true));
        assert!(constty_is(&a));
        // another tty, open with carrier: EBUSY; its "TIOCCONS 0" leaves a's
        assert_eq!(ioctl_int(&b, TIOCCONS, 1, p).0, Err(Errno::EBUSY));
        assert_eq!(ioctl_int(&b, TIOCCONS, 0, p).0, Ok(true));
        assert!(constty_is(&a));
        // a failed access check of /dev/console changes nothing, even for the holder
        d.deny.store(true, Ordering::Relaxed);
        assert_eq!(ioctl_int(&a, TIOCCONS, 1, p).0, Err(Errno::EACCES));
        assert!(constty_is(&a));
        // without carrier a's hold no longer blocks b, but the check still does
        a.t_state_clr(TS_CARR_ON);
        assert_eq!(ioctl_int(&b, TIOCCONS, 1, p).0, Err(Errno::EACCES));
        assert!(constty_is(&a));
        d.deny.store(false, Ordering::Relaxed);
        assert_eq!(ioctl_int(&b, TIOCCONS, 1, p).0, Ok(true));
        assert!(constty_is(&b));
        assert_eq!(ioctl_int(&b, TIOCCONS, 0, p).0, Ok(true));
        assert!(constty::<Kg>().is_none());
    }

    #[test]
    fn the_constty_leg_never_waits_for_a_tty_held_here() {
        let (_p, _d) = world(4711);
        let tp = test_tty();
        tp.set_t_oflag(0);
        // free: the character is queued. The host's threads share one CPU word, so a
        // scheduler's lock another test holds reads as this CPU's: retry until it is free.
        let mut r = None;
        for _ in 0..100_000 {
            r = tputchar_try(i32::from(b'a'), &tp);
            if r.is_some() {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(r, Some(0));
        // the state held on this CPU (an interrupt inside a section): nothing touched
        {
            let _g = tp.st.lock();
            assert_eq!(tputchar_try(i32::from(b'b'), &tp), None);
        }
        // the output queue's lock held on this CPU (an interrupt inside a clist op)
        tp.t_outq.peek_with(1, |_| {
            assert_eq!(tputchar_try(i32::from(b'c'), &tp), None);
        });
        assert_eq!(drain(&tp.t_outq), b"a");
        // a closed tty: -1, as tputchar
        tp.t_state_clr(TS_ISOPEN);
        let mut r = None;
        for _ in 0..100_000 {
            r = tputchar_try(i32::from(b'd'), &tp);
            if r.is_some() {
                break;
            }
            std::thread::yield_now();
        }
        assert_eq!(r, Some(-1));
    }

    #[test]
    fn the_constants_are_the_c_s() {
        assert_eq!(PTMGET, 0x4028_7401);
        assert_eq!(size_of::<Itty>(), 40);
        assert_eq!(size_of::<Ptmget>(), 40);
        assert_eq!(
            [
                TS_ASLEEP,
                TS_CARR_ON,
                TS_TTSTOP,
                TS_XCLUDE,
                TS_TYPEN,
                TS_TSTAMPCTSCLR
            ],
            [0x1, 0x8, 0x100, 0x400, 0x8000, 0x80000]
        );
        assert_eq!(
            [TTY_FE, TTY_PE, TTY_ERRORMASK],
            [0x0100_0000, 0x0200_0000, -0x0100_0000]
        );
        let _: fn(&Pgrp<Kg>) = pgrp_gone::<Kg>;
    }
}
/* </TESTS> */
