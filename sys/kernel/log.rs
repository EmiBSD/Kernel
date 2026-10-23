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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)subr_log.c	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1981, 1984, 1993
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
 *	@(#)msgbuf.h	8.1 (Berkeley) 6/2/93
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)syslog.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! The kernel's message buffer and its log device: `kern/subr_log.c` (the error log buffer for
//! kernel printf's, `/dev/klog`, the log stash and `sendsyslog(2)`), `<sys/msgbuf.h>` (the
//! buffer's layout, which `kern.msgbuf` hands `dmesg(8)`) and `<sys/syslog.h>` (the
//! priorities and facilities, as the kernel sees them).
//!
//! Upstream: sys/kern/subr_log.c @ 3ce1f3f79392
//! Upstream: sys/sys/msgbuf.h @ 3ce1f3f79392
//! Upstream: sys/sys/syslog.h @ 3ce1f3f79392
//! LZ: sys/kern/subr_log.rs@3c62ede68802
//! LZ: sys/sys/msgbuf.rs@3c62ede68802
//! LZ: sys/sys/syslog.rs@3c62ede68802
//!
//! Everything `printf(9)` prints is appended to the message buffer ([`msgbuf_putchar`]); a
//! full ring keeps the most recent bytes and counts the dropped ones in `msg_bufd`. Output to
//! `/dev/console` is kept apart in the console buffer (`kern.consbuf`). `syslogd(8)` reads the
//! message buffer through `/dev/klog` ([`logread`], woken every `LOG_TICK` by [`logtick`] once
//! [`logwakeup`] asked for it) and hands the device the socket `sendsyslog(2)` writes to
//! ([`logioctl`]'s `LIOCSFD`); while there is none, the messages wait in the log stash, at most
//! `LOGSTASH_SIZE` of them, the rest counted.
//!
//! Locking, as in the C: `log_mtx` (here [`LOG_MTX`], `IPL_HIGH`, a leaf: `printf(9)` runs
//! everywhere) owns both buffers and guards `sc_state`; `log_kq_mtx` is the klist's lock and
//! `filt_logmodify`/`filt_logprocess`'s; `syslogf_rwlock` owns `syslogf`; `logstash_rwlock`
//! owns the stash.
//!
//! ## Deviations
//! - The message buffer is a static of the machine's `MSGBUFSIZE` (hal's `<machine/param.h>`)
//!   in the kernel's image, not memory the machine reserves to survive a warm reboot (LZ's,
//!   kept): [`initmsgbuf`] finds it zeroed and initialises it, so `dmesg(8)` shows nothing of
//!   an earlier boot. The header checks and the fresh-line rule stay for the day it does.
//! - After a panic or inside ddb (`panicstr || db_active`) the C's `mtx_enter(&log_mtx)`
//!   returns without the mutex and writes anyway. Here a writer or reader that finds
//!   `log_mtx` held then gives up (the byte is dropped, the reader sees no buffer): the
//!   mutex owns the buffers, so no byte is written or read without it.
//! - The C copies the ring out (`logread`'s `uiomove`, `kern.msgbuf`'s `copyout`) with
//!   `log_mtx` released, reading the live buffer under a writer ("ignore that concurrent
//!   readers may consume the same data"). Here each piece is copied under the mutex into a
//!   chunk on the stack, then moved out with the mutex released: the same bytes, never a byte
//!   a writer is changing.
//! - The stash's cursors are indices into its ring, not pointers (LZ's).
//! - `KTRACE` is not configured (LZ's): `sendsyslog(2)` records no `ktrgenio`.
//! - `struct syslog_data`, the `SYSLOG_NAMES` tables and the userland prototypes of
//!   `<sys/syslog.h>` are not kernel material (LZ's).
//!
//! ## Redesign
//! - LZ lays a `struct msgbuf` over a static byte area with `Msgbuf::from_raw` (an unsized
//!   `[Cell<u8>]` tail, `unsafe` to make, `unsafe impl Sync` to share) and keeps
//!   `msgbufp`/`consbufp` in `StaticCell`s and `log_mtx` beside them. Here `log_mtx` owns the
//!   buffers: [`LOG_MTX`] is a `hal::mutex::Mutexed<LogBufs>` whose data are the two
//!   [`Msgbuf`]s, `#[repr(C)]` with the C's header ([`MsgbufHeader`], `abi_pod!`, the 40
//!   bytes `kern.msgbuf` copies out) and their rings as arrays, so no byte of either is
//!   reached without the mutex. `msgbufp == NULL` and `consbufp == NULL` are a buffer whose
//!   magic is not `MSG_MAGIC` yet (every reader of the C tests both together).
//! - `kern.consbuf` is live (decision 7 of N4.7): the console buffer is a 16 KiB static,
//!   initialised by [`initconsbuf`] as the C's `malloc(CONSBUFSIZE, M_TTYS, M_ZERO)`; LZ
//!   reported `initconsbuf` unported and answered `ENXIO`.
//! - `/dev/klog` is live (decision 6 of N4.7; LZ's `cdevsw` row 7 was `cnotdef`): the C's
//!   `logsoftc` (LZ kept only `sc_need_wakeup`) is [`LogDev`], with `log_open`, `syslogf` and
//!   the stash, in the process glue's tables (`ProcTables::log`): its klist and `syslogf`'s
//!   file are generic over the glue, which only those tables may hold.
//! - `logread_filtops` is [`LogRead`], an `impl kernel::event::Filter` (decision 46).
//! - `dosendsyslog`'s message is a [`SyslogBuf`]: a user address or a stashed message's bytes
//!   (LZ: a raw `*const u8`, `unsafe` to read). The socket leg writes through the file's
//!   `fo_write` (`soo_write` passes `MSG_DONTWAIT` on `FNONBLOCK`, which the C sets by hand
//!   for its direct `sosend`); a stashed message goes as a `UIO_SYSSPACE` uio borrowed from
//!   the stash ([`Uio::kernel_write`]).
//! - The console leg is the C's again: `cnwrite` when `constty` or `cn_devvp` is set, through
//!   the `kernel::tty::TtyDevsw` bridge (bridges.toml, paid at N7 with the console; declared
//!   here at N4.7a, moved to the terminal at N4.7b), else `cnputc` (LZ always took the
//!   second, "`constty` and `cn_devvp` are NULL without the tty layer").
//! - The stash's messages are `MArray`s of `M_LOG` (LZ: raw `malloc` blocks in
//!   `Cell<Option<NonNull<u8>>>`s under an `unsafe impl Sync`), owned by the stash inside
//!   `logstash_rwlock`; freeing one is dropping it.

use core::ffi::c_void;
use core::fmt::{self, Write as _};
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering, fence};

use hal::klock;
use hal::malloc::{M_WAITOK, MArray};
use hal::mutex::{Mutexed, MutexedGuard, SharedMutex};
use hal::pin::pin_static;
use hal::rwlock::Rwlocked;
use hal::sleep::{INFSLP, PCATCH};
use hal::sys::errno::Errno;
use hal::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use hal::sys::ioccom::_iow;
use hal::sys::signal::SIGIO;
use hal::sys::types::Pid;
use hal::user::{UserPtr, pod_from_bytes, pod_to_bytes};

use crate::event::{
    EVFILT_READ, FILTEROP_ISFD, FILTEROP_MPSAFE, Filter, Filterops, Kevent, Klist, Knote,
    knote_modify, knote_process,
};
use crate::file::{DTYPE_SOCKET, FileOps, FileRef, FileSlot};
use crate::proc::{ProcGlue, Thread, suser};
use crate::sched::PZERO;
use crate::sigio::{SigioRef, TIOCGPGRP, TIOCSPGRP, pgsigio, sigio_free, sigio_getown, sigio_init};
use crate::timeout::{KCLOCK_NONE, Timeout};
use crate::uio::{Iovec, Uio, UioMove, UioRw, UioSeg};

// <sys/syslog.h>

/// Set sendsyslog() fd (an `ioctl(2)` on `/dev/klog`).
pub const LIOCSFD: u64 = _iow::<i32>(b'l', 127);

/// Max line length.
pub const LOG_MAXLINE: usize = 8192;
/// Path of the log socket.
pub const _PATH_LOG: &str = "/dev/log";

// priorities (these are ordered)

/// System is unusable.
pub const LOG_EMERG: i32 = 0;
/// Action must be taken immediately.
pub const LOG_ALERT: i32 = 1;
/// Critical conditions.
pub const LOG_CRIT: i32 = 2;
/// Error conditions.
pub const LOG_ERR: i32 = 3;
/// Warning conditions.
pub const LOG_WARNING: i32 = 4;
/// Normal but significant condition.
pub const LOG_NOTICE: i32 = 5;
/// Informational.
pub const LOG_INFO: i32 = 6;
/// Debug-level messages.
pub const LOG_DEBUG: i32 = 7;

/// Mask to extract the priority part (internal).
pub const LOG_PRIMASK: i32 = 0x07;

// facility codes

/// Kernel messages.
pub const LOG_KERN: i32 = 0 << 3;
/// Random user-level messages.
pub const LOG_USER: i32 = 1 << 3;
/// Mail system.
pub const LOG_MAIL: i32 = 2 << 3;
/// System daemons.
pub const LOG_DAEMON: i32 = 3 << 3;
/// Security/authorization messages.
pub const LOG_AUTH: i32 = 4 << 3;
/// Messages generated internally by syslogd.
pub const LOG_SYSLOG: i32 = 5 << 3;
/// Line printer subsystem.
pub const LOG_LPR: i32 = 6 << 3;
/// Network news subsystem.
pub const LOG_NEWS: i32 = 7 << 3;
/// UUCP subsystem.
pub const LOG_UUCP: i32 = 8 << 3;
/// Clock daemon.
pub const LOG_CRON: i32 = 9 << 3;
/// Security/authorization messages (private).
pub const LOG_AUTHPRIV: i32 = 10 << 3;
/// Ftp daemon.
pub const LOG_FTP: i32 = 11 << 3;
/// Reserved for local use.
pub const LOG_LOCAL0: i32 = 16 << 3;
/// Reserved for local use.
pub const LOG_LOCAL1: i32 = 17 << 3;
/// Reserved for local use.
pub const LOG_LOCAL2: i32 = 18 << 3;
/// Reserved for local use.
pub const LOG_LOCAL3: i32 = 19 << 3;
/// Reserved for local use.
pub const LOG_LOCAL4: i32 = 20 << 3;
/// Reserved for local use.
pub const LOG_LOCAL5: i32 = 21 << 3;
/// Reserved for local use.
pub const LOG_LOCAL6: i32 = 22 << 3;
/// Reserved for local use.
pub const LOG_LOCAL7: i32 = 23 << 3;

/// Current number of facilities.
pub const LOG_NFACILITIES: i32 = 24;
/// Mask to extract the facility part.
pub const LOG_FACMASK: i32 = 0x03f8;

/// Pseudo-priority to indicate use of printf (kernel only).
pub const LOG_PRINTF: i32 = -1;

// Option flags for openlog. LOG_ODELAY no longer does anything; LOG_NDELAY is the inverse of
// what it used to be.

/// Log the pid with each message.
pub const LOG_PID: i32 = 0x01;
/// Log on the console if errors in sending.
pub const LOG_CONS: i32 = 0x02;
/// Delay open until first syslog() (default).
pub const LOG_ODELAY: i32 = 0x04;
/// Don't delay open.
pub const LOG_NDELAY: i32 = 0x08;
/// Don't wait for console forks: DEPRECATED.
pub const LOG_NOWAIT: i32 = 0x10;
/// Log to stderr as well.
pub const LOG_PERROR: i32 = 0x20;

/// `LOG_PRI(p)`: the priority part of `p`.
pub const fn log_pri(p: i32) -> i32 {
    p & LOG_PRIMASK
}

/// `LOG_FAC(p)`: the facility of `p`, as a facility number (not shifted).
pub const fn log_fac(p: i32) -> i32 {
    (p & LOG_FACMASK) >> 3
}

/// `LOG_MASK(pri)`: mask for one priority, for `setlogmask`.
pub const fn log_mask(pri: i32) -> i32 {
    1 << pri
}

/// `LOG_UPTO(pri)`: mask for all priorities through `pri`.
pub const fn log_upto(pri: i32) -> i32 {
    (1 << (pri + 1)) - 1
}

// <sys/msgbuf.h>

/// `MSG_MAGIC`: buffer magic value.
pub const MSG_MAGIC: i64 = 0x063061;
/// `CONSBUFSIZE`: console message buffer size.
pub const CONSBUFSIZE: usize = 16 * 1024;
/// `MSGBUFSIZE` of the machine (`<machine/param.h>`): the message buffer's size.
pub const MSGBUFSIZE: usize = hal::arch::current::param::MSGBUFSIZE;
/// `offsetof(struct msgbuf, msg_bufc)`: the header before the ring.
pub const MSG_HDRSIZE: usize = size_of::<MsgbufHeader>();

// subr_log.c

/// `LOG_RDPRI`: the priority `logread` sleeps at.
const LOG_RDPRI: i32 = PZERO + 1;
/// `LOG_TICK`: log tick interval in msec.
const LOG_TICK: u64 = 50;

/// `LOG_ASYNC`: `FIOASYNC` is on (`sc_state`).
const LOG_ASYNC: i32 = 0x04;
/// `LOG_RDWAIT`: a reader sleeps (`sc_state`).
const LOG_RDWAIT: i32 = 0x08;

/// `IO_NDELAY` (`<sys/vnode.h>`): a non-blocking read.
const IO_NDELAY: i32 = 0x10;
/// `M_LOG` (`<sys/malloc.h>`): the log stash's messages.
const M_LOG: i32 = 24;
/// `M_TEMP` (`<sys/malloc.h>`): `dosendsyslog`'s console copy.
const M_TEMP: i32 = 127;

/// `LOGSTASH_SIZE`: how many messages the log stash keeps while `syslogd(8)` is away.
pub const LOGSTASH_SIZE: usize = 100;

/// The bytes a reader copies out per hold of `log_mtx` (a chunk on its stack).
const COPY_CHUNK: usize = 256;

/// The ring of the message buffer.
const MSG_RING: usize = MSGBUFSIZE - MSG_HDRSIZE;
/// The ring of the console buffer.
const CONS_RING: usize = CONSBUFSIZE - MSG_HDRSIZE;

/// The header of `struct msgbuf`, as `kern.msgbuf` copies it out ahead of the ring.
///
/// Locking: `I` immutable after creation, `L` `log_mtx`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MsgbufHeader {
    /// \[I\] buffer magic value.
    pub msg_magic: i64,
    /// \[L\] write pointer.
    pub msg_bufx: i64,
    /// \[L\] read pointer.
    pub msg_bufr: i64,
    /// \[I\] real `msg_bufc` size (bytes).
    pub msg_bufs: i64,
    /// \[L\] number of dropped bytes.
    pub msg_bufd: i64,
}

hal::abi_pod!(MsgbufHeader {
    msg_magic: i64,
    msg_bufx: i64,
    msg_bufr: i64,
    msg_bufs: i64,
    msg_bufd: i64,
});

/// `struct msgbuf` with an `N`-byte ring (`msg_bufc`), owned by `log_mtx` ([`LOG_MTX`]).
#[repr(C)]
pub struct Msgbuf<const N: usize> {
    /// The header.
    hdr: MsgbufHeader,
    /// \[L\] `msg_bufc`: the ring.
    msg_bufc: [u8; N],
}

impl<const N: usize> Msgbuf<N> {
    /// All zero, as the C's `M_ZERO` allocation or a fresh `.bss` area.
    pub const fn new() -> Self {
        Self {
            hdr: MsgbufHeader {
                msg_magic: 0,
                msg_bufx: 0,
                msg_bufr: 0,
                msg_bufs: 0,
                msg_bufd: 0,
            },
            msg_bufc: [0; N],
        }
    }

    /// The header.
    pub fn header(&self) -> MsgbufHeader {
        self.hdr
    }

    /// The ring's bytes in use: the first `msg_bufs` (at most the array).
    fn ring_len(&self) -> usize {
        usize::try_from(self.hdr.msg_bufs).map_or(0, |s| s.min(N))
    }

    /// The `initmsgbuf` check: keeps a header left by a previous boot (right magic, the
    /// ring `new_bufs` long, the pointers inside it), clears and initialises any other, then
    /// starts new output on a fresh line.
    fn init(&mut self, new_bufs: i64) {
        let h = self.hdr;
        if h.msg_magic != MSG_MAGIC
            || h.msg_bufs != new_bufs
            || h.msg_bufr < 0
            || h.msg_bufr >= h.msg_bufs
            || h.msg_bufx < 0
            || h.msg_bufx >= h.msg_bufs
        {
            // If the buffer magic number is wrong, has changed size (which shouldn't happen
            // often), or is internally inconsistent, initialize it.
            *self = Self::new();
            self.hdr.msg_magic = MSG_MAGIC;
            self.hdr.msg_bufs = new_bufs;
        }

        // Always start new buffer data on a new line.
        let x = self.hdr.msg_bufx;
        if x > 0 && self.msg_bufc.get((x - 1) as usize) != Some(&b'\n') {
            self.putchar_locked(b'\n');
        }
    }

    /// `msgbuf_putchar_locked`: appends `c` to the ring; when it is full the oldest byte is
    /// dropped and counted in `msg_bufd`.
    fn putchar_locked(&mut self, c: u8) {
        let bufs = self.hdr.msg_bufs;
        let mut x = self.hdr.msg_bufx;
        // A write pointer outside the ring (a header `init` never accepted) starts over at
        // the ring's beginning: the C would write past the buffer.
        if x < 0 || x as usize >= self.ring_len() {
            x = 0;
        }
        if let Some(b) = self.msg_bufc.get_mut(x as usize) {
            *b = c;
        }
        x += 1;
        if x < 0 || x >= bufs {
            x = 0;
        }
        self.hdr.msg_bufx = x;
        // If the buffer is full, keep the most recent data.
        if self.hdr.msg_bufr == x {
            let mut r = self.hdr.msg_bufr + 1;
            if r >= bufs {
                r = 0;
            }
            self.hdr.msg_bufr = r;
            self.hdr.msg_bufd += 1;
        }
    }

    /// `msgbuf_getlen`: the bytes a reader has not taken yet.
    fn getlen(&self) -> usize {
        let mut len = self.hdr.msg_bufx - self.hdr.msg_bufr;
        if len < 0 {
            len += self.hdr.msg_bufs;
        }
        usize::try_from(len).unwrap_or(0)
    }

    /// Copies ring bytes from `off` on into `out`, as many as the ring holds past `off`
    /// (no wrap); how many.
    fn read_at(&self, off: usize, out: &mut [u8]) -> usize {
        let ring = &self.msg_bufc[..self.ring_len()];
        let Some(src) = ring.get(off..) else {
            return 0;
        };
        let n = src.len().min(out.len());
        out[..n].copy_from_slice(&src[..n]);
        n
    }
}

impl<const N: usize> Default for Msgbuf<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Which buffer: `msgbufp` or `consbufp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buf {
    /// `msgbufp`: the message buffer, everything printed.
    Msgbuf,
    /// `consbufp`: what was written to `/dev/console`.
    Consbuf,
}

/// The two buffers `log_mtx` owns.
pub struct LogBufs {
    /// `*msgbufp`.
    msgbuf: Msgbuf<MSG_RING>,
    /// `*consbufp`.
    consbuf: Msgbuf<CONS_RING>,
}

impl LogBufs {
    /// Both zeroed: neither mapped yet.
    const fn new() -> Self {
        Self {
            msgbuf: Msgbuf::new(),
            consbuf: Msgbuf::new(),
        }
    }

    /// The header of `which`.
    fn header(&self, which: Buf) -> MsgbufHeader {
        match which {
            Buf::Msgbuf => self.msgbuf.hdr,
            Buf::Consbuf => self.consbuf.hdr,
        }
    }

    /// `msgbuf_putchar_locked(which, c)`.
    fn putchar_locked(&mut self, which: Buf, c: u8) {
        match which {
            Buf::Msgbuf => self.msgbuf.putchar_locked(c),
            Buf::Consbuf => self.consbuf.putchar_locked(c),
        }
    }

    /// [`Msgbuf::read_at`] of `which`.
    fn read_at(&self, which: Buf, off: usize, out: &mut [u8]) -> usize {
        match which {
            Buf::Msgbuf => self.msgbuf.read_at(off, out),
            Buf::Consbuf => self.consbuf.read_at(off, out),
        }
    }
}

/// `struct logstash_message`s and their cursors: the messages `sendsyslog(2)` could not
/// deliver, waiting for `syslogd(8)`. Owned by `logstash_rwlock`.
pub struct Logstash {
    /// `logstash_messages`: a message's bytes (`lgs_buffer`, `lgs_size` its length), or none.
    messages: [Option<MArray<u8>>; LOGSTASH_SIZE],
    /// `logstash_in`.
    r#in: usize,
    /// `logstash_out`.
    out: usize,
    /// `logstash_dropped`.
    dropped: i32,
    /// `logstash_error`.
    error: i32,
    /// `logstash_pid`.
    pid: Pid,
}

impl Logstash {
    /// Empty.
    const fn new() -> Self {
        Self {
            messages: [const { None }; LOGSTASH_SIZE],
            r#in: 0,
            out: 0,
            dropped: 0,
            error: 0,
            pid: 0,
        }
    }
}

/// `logsoftc` and the rest of `/dev/klog`'s state, in the glue's tables
/// (`ProcTables::log`).
///
/// Locking: `L` `log_mtx` ([`LOG_MTX`]), `Q` `log_kq_mtx` (`kq_mtx`).
pub struct LogDev<G: ProcGlue> {
    /// `log_open`: is `/dev/klog` open? (also used in `log()`).
    open: AtomicBool,
    /// \[L\] `sc_state`: `LOG_ASYNC`, `LOG_RDWAIT`; written under `log_mtx`, read bare by
    /// `sleep_finish`'s test, as the C.
    state: AtomicI32,
    /// `sc_need_wakeup`: if set, wake up waiters.
    need_wakeup: AtomicBool,
    /// `log_kq_mtx`: the klist's lock, and `filt_logmodify`/`filt_logprocess`'s.
    kq_mtx: SharedMutex,
    /// \[Q\] `sc_klist`: process waiting on kevent call.
    klist: Klist<G>,
    /// `sc_sigio`: async I/O registration.
    sigio: SigioRef,
    /// `sc_tick`: wakeup poll timeout.
    tick: Timeout,
    /// `syslogf` under `syslogf_rwlock`: the socket `syslogd(8)` handed `LIOCSFD`.
    syslogf: Rwlocked<FileSlot<G::FileGlue>>,
    /// The log stash under `logstash_rwlock`.
    stash: Rwlocked<Logstash>,
}

impl<G: ProcGlue> LogDev<G> {
    /// `logsoftc`, closed, with an empty stash.
    pub const fn new() -> Self {
        Self {
            open: AtomicBool::new(false),
            state: AtomicI32::new(0),
            need_wakeup: AtomicBool::new(false),
            kq_mtx: Mutexed::new(hal::intr::IPL_HIGH, ()),
            klist: Klist::owner(),
            sigio: SigioRef::new(),
            tick: Timeout::new(logtick::<G>, ptr::null_mut()),
            syslogf: Rwlocked::new("syslogf", FileSlot::new()),
            stash: Rwlocked::new("logstash", Logstash::new()),
        }
    }

    /// The `LogDev` of the glue `G`.
    fn get() -> &'static Self {
        &G::tables().log
    }

    /// `&logread_filtops`.
    const READ_FILTOPS: Filterops<G> = Filterops::of::<LogRead>();
}

impl<G: ProcGlue> Default for LogDev<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `dosendsyslog`'s message: a user address (`UIO_USERSPACE`) or a stashed message's bytes
/// (`UIO_SYSSPACE`).
#[derive(Clone, Copy)]
pub enum SyslogBuf<'a> {
    /// A user address.
    User(usize),
    /// A stashed message.
    Sys(&'a [u8]),
}

impl SyslogBuf<'_> {
    /// Copies `out.len()` bytes from `off` on into `out` (`copyin` or `memcpy`).
    fn read(self, off: usize, out: &mut [u8]) -> Result<(), Errno> {
        match self {
            SyslogBuf::User(addr) => match addr.checked_add(off) {
                Some(a) => UserPtr::<u8>::new(a).copyin_slice(out),
                None => Err(Errno::EFAULT),
            },
            SyslogBuf::Sys(b) => match b.get(off..off.saturating_add(out.len())) {
                Some(src) => {
                    out.copy_from_slice(src);
                    Ok(())
                }
                None => Err(Errno::EFAULT),
            },
        }
    }
}

/// `snprintf(buf, sizeof(buf), ...)`: writes what fits (the last byte kept for the NUL) and
/// counts every byte the format makes, which is `snprintf`'s return value.
struct Snprintf<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl fmt::Write for Snprintf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() {
            if self.len + 1 < self.buf.len() {
                self.buf[self.len] = b;
            }
            self.len += 1;
        }
        Ok(())
    }
}

/// `snprintf(buf, sizeof(buf), fmt, ...)`.
fn snprintf(buf: &mut [u8], args: fmt::Arguments<'_>) -> usize {
    let mut w = Snprintf { buf, len: 0 };
    let _ = w.write_fmt(args);
    w.len
}

/// `log_mtx` and the buffers it owns (`msgbufp`, `consbufp`): a leaf at `IPL_HIGH`, so
/// `printf(9)` can run anywhere.
pub static LOG_MTX: Mutexed<LogBufs> = Mutexed::new(hal::intr::IPL_HIGH, LogBufs::new());

/// `msgbufmapped`: is the message buffer mapped?
static MSGBUFMAPPED: AtomicBool = AtomicBool::new(false);

/// What `logread` sleeps on and `logtick` wakes (the C's `msgbufp`).
static LOG_RDCHAN: u8 = 0;

/// `logread_filtops`: `EVFILT_READ` on `/dev/klog`.
pub struct LogRead;

/// `log_mtx`, or `None` after a panic or inside ddb when another CPU (or this one) holds it
/// (see the module's deviations).
fn log_mtx() -> Option<MutexedGuard<'static, LogBufs>> {
    if hal::cpu::past_caring() && LOG_MTX.is_locked() {
        return None;
    }
    Some(LOG_MTX.lock())
}

/// `initmsgbuf(buf, bufsize)` over the machine's static buffer (LZ's `init_static_msgbuf`):
/// keeps a sane header, initialises any other, starts new output on a fresh line, and marks
/// the buffer mapped.
pub fn initmsgbuf() {
    if let Some(mut g) = log_mtx() {
        g.msgbuf.init(MSG_RING as i64);
    }
    // mark it as ready for use.
    MSGBUFMAPPED.store(true, Ordering::Release);
}

/// `initconsbuf`: sets up the buffer that collects `/dev/console` output.
pub fn initconsbuf() {
    if let Some(mut g) = log_mtx() {
        g.consbuf = Msgbuf::new();
        g.consbuf.hdr.msg_magic = MSG_MAGIC;
        g.consbuf.hdr.msg_bufs = CONS_RING as i64;
    }
}

/// `msgbufmapped`: whether [`initmsgbuf`] has run.
pub fn msgbufmapped() -> bool {
    MSGBUFMAPPED.load(Ordering::Acquire)
}

/// `msgbuf_putchar(which, c)`: appends `c` to the buffer under `log_mtx`; nothing happens
/// if the buffer was never initialised.
pub fn msgbuf_putchar(which: Buf, c: u8) {
    let Some(mut g) = log_mtx() else {
        return;
    };
    if g.header(which).msg_magic != MSG_MAGIC {
        // Nothing we can do
        return;
    }
    g.putchar_locked(which, c);
}

/// The header of `which` (`msgbufp`, `consbufp`) when it holds a buffer (magic
/// `MSG_MAGIC`): `None` where the C finds `NULL` or a corrupted buffer.
pub fn msgbuf_header(which: Buf) -> Option<MsgbufHeader> {
    let h = log_mtx()?.header(which);
    (h.msg_magic == MSG_MAGIC).then_some(h)
}

/// Copies `which`'s ring bytes from `off` on into `out`, up to the ring's end (no wrap);
/// how many (0 past the end, or without the mutex after a panic).
pub fn msgbuf_read(which: Buf, off: usize, out: &mut [u8]) -> usize {
    log_mtx().map_or(0, |g| g.read_at(which, off, out))
}

/// `kern.msgbuf`, `kern.consbuf` (`kern_sysctl`'s `KERN_MSGBUF` case: the caller checked
/// `suser` for the console's): the header then the ring, copied out to `oldp`. `ENXIO` for a
/// buffer not set up or corrupted, `EPERM` for a new value, `ENOMEM` when `*oldlenp` is
/// shorter; without `oldp` nothing (and `*oldlenp` untouched, as the C).
pub fn sysctl_msgbuf(
    which: Buf,
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
) -> Result<(), Errno> {
    // deal with cases where the message buffer has become corrupted.
    let Some(h) = msgbuf_header(which) else {
        return Err(Errno::ENXIO);
    };
    if newp != 0 {
        return Err(Errno::EPERM);
    }
    let bufs = usize::try_from(h.msg_bufs).map_err(|_| Errno::ENXIO)?;
    if oldp == 0 {
        return Ok(());
    }
    if MSG_HDRSIZE + bufs > *oldlenp {
        return Err(Errno::ENOMEM);
    }

    // copy header...
    UserPtr::<MsgbufHeader>::new(oldp).copyout(&h)?;
    // ...and the data.
    let mut chunk = [0u8; COPY_CHUNK];
    let mut off = 0;
    while off < bufs {
        let want = (bufs - off).min(COPY_CHUNK);
        let n = msgbuf_read(which, off, &mut chunk[..want]);
        if n == 0 {
            break;
        }
        UserPtr::<u8>::new(oldp + MSG_HDRSIZE + off).copyout_slice(&chunk[..n])?;
        off += n;
    }
    Ok(())
}

/// `kern.msgbufsize`, `kern.consbufsize`: the ring's size; `ENXIO` for a buffer not set up
/// or corrupted.
pub fn msgbuf_size(which: Buf) -> Result<i64, Errno> {
    msgbuf_header(which).map(|h| h.msg_bufs).ok_or(Errno::ENXIO)
}

/// `log_open`: whether `/dev/klog` is open (`log()` writes to the console when it is not).
pub fn log_open<G: ProcGlue>() -> bool {
    LogDev::<G>::get().open.load(Ordering::Relaxed)
}

/// `logopen`: `EBUSY` while open; arms the tick.
pub fn logopen<G: ProcGlue>() -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    if log
        .open
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err(Errno::EBUSY);
    }
    // klist_init_mutex(&sc_klist, &log_kq_mtx): the klist is the owner kind, keyed by
    // log_kq_mtx's guard at every use.
    sigio_init(&log.sigio);
    log.tick
        .set(Some(logtick::<G>), ptr::null_mut(), KCLOCK_NONE, 0);
    pin_static(&log.tick).add_msec(LOG_TICK);
    Ok(())
}

/// `logclose`: drops `syslogf`, stops the tick, detaches the knotes, forgets the async
/// registration.
pub fn logclose<G: ProcGlue>(p: Option<&Thread<G>>) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    let fp = log.syslogf.write().take();
    if let Some(fp) = fp {
        let _ = fp.frele(p);
    }
    log.open.store(false, Ordering::Relaxed);
    log.tick.del();

    log.klist.invalidate_with(|| log.kq_mtx.lock());
    log.klist.free();

    log.state.store(0, Ordering::Relaxed);
    sigio_free(&log.sigio);
    Ok(())
}

/// `logread`: blocks (unless `IO_NDELAY`) until the message buffer has unread bytes, gives
/// the "klog: dropped N bytes" line first when the ring overran, then as many bytes as the
/// reader asked for, advancing the read pointer.
pub fn logread<G: ProcGlue>(uio: &mut dyn UioMove, flag: i32) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    let mut g = LOG_MTX.lock();
    while g.msgbuf.hdr.msg_bufr == g.msgbuf.hdr.msg_bufx {
        if flag & IO_NDELAY != 0 {
            return Err(Errno::EWOULDBLOCK);
        }
        log.state.fetch_or(LOG_RDWAIT, Ordering::Relaxed);
        drop(g);
        // Set up and enter sleep manually instead of using msleep() to keep log_mtx as a
        // leaf lock.
        hal::sleep::sleep_setup(&LOG_RDCHAN, LOG_RDPRI | PCATCH, "klog");
        let error =
            hal::sleep::sleep_finish(INFSLP, log.state.load(Ordering::Relaxed) & LOG_RDWAIT != 0);
        g = LOG_MTX.lock();
        error?;
    }

    if g.msgbuf.hdr.msg_bufd > 0 {
        let mut buf = [0u8; 64];
        let ndropped = g.msgbuf.hdr.msg_bufd;
        drop(g);
        let l = snprintf(
            &mut buf,
            format_args!(
                "<{}>klog: dropped {} byte{}, message buffer full\n",
                LOG_KERN | LOG_WARNING,
                ndropped,
                if ndropped == 1 { "" } else { "s" }
            ),
        );
        let n = l.min(buf.len() - 1);
        uio.uiomove(&mut buf[..n])?;
        g = LOG_MTX.lock();
        g.msgbuf.hdr.msg_bufd -= ndropped;
    }

    let mut chunk = [0u8; COPY_CHUNK];
    while uio.resid() > 0 {
        let h = g.msgbuf.hdr;
        let l = if h.msg_bufx >= h.msg_bufr {
            h.msg_bufx - h.msg_bufr
        } else {
            h.msg_bufs - h.msg_bufr
        };
        let l = usize::try_from(l)
            .unwrap_or(0)
            .min(uio.resid())
            .min(COPY_CHUNK);
        if l == 0 {
            break;
        }
        let rpos = usize::try_from(h.msg_bufr).unwrap_or(0);
        let l = g.msgbuf.read_at(rpos, &mut chunk[..l]);
        if l == 0 {
            break;
        }
        drop(g);
        // Ignore that concurrent readers may consume the same data.
        let error = uio.uiomove(&mut chunk[..l]);
        g = LOG_MTX.lock();
        error?;
        let mut r = g.msgbuf.hdr.msg_bufr + l as i64;
        if r < 0 || r >= g.msgbuf.hdr.msg_bufs {
            r = 0;
        }
        g.msgbuf.hdr.msg_bufr = r;
    }
    Ok(())
}

/// `logkqfilter`: `EVFILT_READ` only (`EINVAL` otherwise), hooked on `sc_klist`.
pub fn logkqfilter<G: ProcGlue>(kn: &Knote<G>) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    if kn.kn_filter().get() != EVFILT_READ {
        return Err(Errno::EINVAL);
    }
    kn.kn_fop.set(Some(&LogDev::<G>::READ_FILTOPS));
    // kn->kn_hook = msgbufp: the filter reads the one message buffer itself.
    let mut g = log.kq_mtx.lock();
    log.klist.insert_locked(&mut g, kn);
    Ok(())
}

impl<G: ProcGlue> Filter<G> for LogRead {
    const FLAGS: i32 = FILTEROP_ISFD | FILTEROP_MPSAFE;

    /// `filt_logrdetach`.
    fn detach(kn: &Knote<G>) {
        let log = LogDev::<G>::get();
        let mut g = log.kq_mtx.lock();
        log.klist.remove_locked(&mut g, kn);
    }

    /// `filt_logread`: the bytes `logread` would return.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let n = log_mtx().map_or(0, |g| g.msgbuf.getlen());
        kn.kn_data().set(n as i64);
        n != 0
    }

    /// `filt_logmodify`.
    fn modify(kev: &mut Kevent, kn: &Knote<G>) -> bool {
        let _g = LogDev::<G>::get().kq_mtx.lock();
        knote_modify(kev, kn)
    }

    /// `filt_logprocess`.
    fn process(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
        let _g = LogDev::<G>::get().kq_mtx.lock();
        knote_process(kn, kev)
    }
}

/// `logwakeup`: asks for the `/dev/klog` readers to be woken. The actual wakeup has to be
/// deferred because `logwakeup()` can be called in very varied contexts; keeping the print
/// routines usable in as many situations as possible means no locking here.
pub fn logwakeup<G: ProcGlue>() {
    // Ensure that preceding stores become visible to other CPUs before the flag
    // (membar_producer).
    fence(Ordering::Release);
    LogDev::<G>::get()
        .need_wakeup
        .store(true, Ordering::Relaxed);
}

/// `logtick`: every `LOG_TICK` while the device is open, the deferred wakeup of
/// [`logwakeup`]: the knotes, the `SIGIO` owner, the sleeping reader.
pub fn logtick<G: ProcGlue>(_arg: *mut c_void) {
    let log = LogDev::<G>::get();
    if !log.open.load(Ordering::Relaxed) {
        return;
    }

    if log.need_wakeup.swap(false, Ordering::Relaxed) {
        // sc_need_wakeup has to be cleared before handling the wakeup. Visiting log_mtx
        // ensures the proper order.
        let state = {
            let _g = LOG_MTX.lock();
            log.state.fetch_and(!LOG_RDWAIT, Ordering::Relaxed)
        };

        {
            let g = log.kq_mtx.lock();
            log.klist.knote_locked(&g, 0);
        }

        if state & LOG_ASYNC != 0 {
            pgsigio::<G>(&log.sigio, SIGIO, false);
        }
        if state & LOG_RDWAIT != 0 {
            hal::sleep::wakeup(&LOG_RDCHAN);
        }
    }
    pin_static(&log.tick).add_msec(LOG_TICK);
}

/// The `int` an ioctl's argument holds.
fn int_arg(data: &[u8]) -> Result<i32, Errno> {
    pod_from_bytes::<i32>(data).ok_or(Errno::EINVAL)
}

/// Stores `v` as the ioctl's `int` result.
fn int_ret(data: &mut [u8], v: i32) -> Result<(), Errno> {
    if pod_to_bytes(&v, data) {
        Ok(())
    } else {
        Err(Errno::EINVAL)
    }
}

/// `logioctl`: `FIONREAD` (the bytes immediately available), `FIOASYNC`, the `SIGIO`
/// owner (`FIOSETOWN`/`TIOCSPGRP`, `FIOGETOWN`/`TIOCGPGRP`), and `LIOCSFD` (root only: the
/// socket `sendsyslog(2)` writes to); `ENOTTY` otherwise.
pub fn logioctl<G: ProcGlue>(com: u64, data: &mut [u8], p: &Thread<G>) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    match com {
        // return number of characters immediately available
        FIONREAD => {
            let n = LOG_MTX.lock().msgbuf.getlen();
            int_ret(data, n as i32)
        }
        FIOASYNC => {
            let on = int_arg(data)? != 0;
            let _g = LOG_MTX.lock();
            if on {
                log.state.fetch_or(LOG_ASYNC, Ordering::Relaxed);
            } else {
                log.state.fetch_and(!LOG_ASYNC, Ordering::Relaxed);
            }
            Ok(())
        }
        FIOSETOWN | TIOCSPGRP => crate::sigio::sigio_setown::<G>(&log.sigio, com, int_arg(data)?),
        FIOGETOWN | TIOCGPGRP => int_ret(data, sigio_getown(&log.sigio, com)),
        LIOCSFD => {
            suser(p)?;
            let newfp = getsock(p, int_arg(data)?)?;
            let fp = log.syslogf.write().set(Some(newfp));
            if let Some(fp) = fp {
                let _ = fp.frele(Some(p));
            }
            Ok(())
        }
        _ => Err(Errno::ENOTTY),
    }
}

/// `getsock(p, fdes, &fp)` (`uipc_syscalls.c`): the socket open at `fdes`, a reference of
/// the caller's; `EBADF` for no file, `ENOTSOCK` for a file of another type.
fn getsock<G: ProcGlue>(p: &Thread<G>, fdes: i32) -> Result<FileRef<G::FileGlue>, Errno> {
    let Some(fp) = crate::descrip::fd_getfile(p.fd(), fdes) else {
        return Err(Errno::EBADF);
    };
    if fp.f_type() != DTYPE_SOCKET {
        let _ = fp.frele(Some(p));
        return Err(Errno::ENOTSOCK);
    }
    Ok(fp)
}

/// `logstash_full`.
fn logstash_full(st: &Logstash) -> bool {
    st.messages[st.out].is_some() && st.r#in == st.out
}

/// `logstash_increment`: advances a cursor around the ring.
fn logstash_increment(cursor: &mut usize) {
    crate::assert::kassert!(*cursor < LOGSTASH_SIZE);
    if *cursor == LOGSTASH_SIZE - 1 {
        *cursor = 0;
    } else {
        *cursor += 1;
    }
}

/// `logstash_insert`: keeps a copy of the user message at `buf` for later, or counts it as
/// dropped (with the first error and pid) when the stash is full.
pub fn logstash_insert<G: ProcGlue>(
    buf: usize,
    nbyte: usize,
    logerror: Errno,
    pid: Pid,
) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    let mut st = log.stash.write();

    if logstash_full(&st) {
        if st.dropped == 0 {
            st.error = logerror.as_i32();
            st.pid = pid;
        }
        st.dropped += 1;
        return Ok(());
    }

    let Some(mut msg) = MArray::new(M_LOG, M_WAITOK, nbyte, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };
    // A copyin that fails drops (frees) the message and leaves the slot empty.
    UserPtr::<u8>::new(buf).copyin_slice(&mut msg)?;
    let i = st.r#in;
    st.messages[i] = Some(msg);
    logstash_increment(&mut st.r#in);
    Ok(())
}

/// `logstash_remove`: frees the oldest stashed message and, if messages were dropped,
/// stashes a note saying how many in their place in the sequence.
fn logstash_remove(st: &mut Logstash) {
    let out = st.out;
    crate::assert::kassert!(st.messages[out].is_some());
    st.messages[out] = None;
    logstash_increment(&mut st.out);

    // Insert dropped message in sequence where messages were dropped.
    if st.dropped != 0 {
        let mut buf = [0u8; 80];
        let l = snprintf(
            &mut buf,
            format_args!(
                "<{}>sendsyslog: dropped {} message{}, error {}, pid {}",
                LOG_KERN | LOG_WARNING,
                st.dropped,
                if st.dropped == 1 { "" } else { "s" },
                st.error,
                st.pid
            ),
        );
        st.dropped = 0;
        st.error = 0;
        st.pid = 0;

        // Cannot fail, we have just freed a slot.
        crate::assert::kassert!(!logstash_full(st));
        let nbyte = l.min(buf.len() - 1);
        if let Some(msg) = MArray::new(M_LOG, M_WAITOK, nbyte, |i| buf[i]) {
            let i = st.r#in;
            st.messages[i] = Some(msg);
            logstash_increment(&mut st.r#in);
        }
    }
}

/// `logstash_sendsyslog`: sends the stashed messages, oldest first, until one fails.
pub fn logstash_sendsyslog<G: ProcGlue>(p: &Thread<G>) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    let mut st = log.stash.write();

    while let Some(msg) = &st.messages[st.out] {
        dosendsyslog(p, SyslogBuf::Sys(msg), msg.len(), 0)?;
        logstash_remove(&mut st);
    }
    Ok(())
}

/// `sendsyslog(2)`: sends a `syslog(3)` message from userland to the `socketpair(2)` created
/// by `syslogd(8)`. Stores the message in the kernel log stash for later if `syslogd(8)` is
/// not available or sending fails. Sends it to the console if `LOG_CONS` is set and the
/// `syslogd(8)` socket does not exist.
pub fn sys_sendsyslog<G: ProcGlue>(
    p: &Thread<G>,
    buf: usize,
    nbyte: usize,
    flags: i32,
) -> Result<(), Errno> {
    let nbyte = nbyte.min(LOG_MAXLINE);

    let _ = logstash_sendsyslog(p);
    let error = dosendsyslog(p, SyslogBuf::User(buf), nbyte, flags);
    if let Err(e) = error
        && e != Errno::EFAULT
    {
        let _ = logstash_insert::<G>(buf, nbyte, e, p.process().ps_pid);
    }
    error
}

/// Holds the kernel lock for its life (`KERNEL_LOCK()` ... `KERNEL_UNLOCK()`).
struct KernelHold;

impl KernelHold {
    fn take() -> Self {
        klock::kernel_lock();
        Self
    }
}

impl Drop for KernelHold {
    fn drop(&mut self) {
        klock::kernel_unlock();
    }
}

/// `dosendsyslog`: sends `nbyte` bytes of `buf` to `syslogd(8)`'s socket, or, when there is
/// none and `LOG_CONS` is set, to the console without the `<pri>` prefix. `ENOTCONN` when
/// nothing took the message (an `EFAULT` stays one).
pub fn dosendsyslog<G: ProcGlue>(
    p: &Thread<G>,
    buf: SyslogBuf<'_>,
    nbyte: usize,
    flags: i32,
) -> Result<(), Errno> {
    let log = LogDev::<G>::get();
    let mut nbyte = nbyte;
    let mut start = 0usize;

    // Global variable syslogf may change during sleep, use local copy.
    let fp: Option<FileRef<G::FileGlue>> = log.syslogf.read().get().cloned();

    if fp.is_none() {
        if flags & LOG_CONS == 0 {
            return Err(Errno::ENOTCONN);
        }
        // Strip off syslog priority when logging to console. LOG_PRIMASK | LOG_FACMASK is
        // 0x03ff, so at most 4 decimal digits may appear in priority as <1023>.
        let mut pri = [0u8; 6];
        let len = nbyte.min(pri.len());
        buf.read(0, &mut pri[..len])?;
        if 0 < len && pri[0] == b'<' {
            let mut i = 1;
            while i < len && pri[i].is_ascii_digit() {
                i += 1;
            }
            if i < len && pri[i] == b'>' {
                i += 1;
                // There must be at least one digit <0>.
                if i >= 3 {
                    start = i;
                    nbyte -= i;
                }
            }
        }
    }

    // aiov = (buf + start, nbyte), UIO_WRITE, the bytes' own segment.
    let mut iov = [Iovec::new()];
    let mut auio = match buf {
        SyslogBuf::User(addr) => {
            iov[0] = Iovec {
                iov_base: ptr::without_provenance_mut(addr.wrapping_add(start)),
                iov_len: nbyte,
            };
            Uio {
                uio_iov: &mut iov,
                uio_offset: 0,
                uio_resid: nbyte,
                uio_segflg: UioSeg::UIO_USERSPACE,
                uio_rw: UioRw::UIO_WRITE,
                uio_procp: Some(p),
            }
        }
        SyslogBuf::Sys(b) => {
            let end = start.saturating_add(nbyte).min(b.len());
            Uio::kernel_write(&mut iov, &b[start.min(end)..end], Some(p))
        }
    };

    let error = match &fp {
        Some(fp) => fp.ops().write(fp, &mut auio, 0),
        None => {
            let _k = KernelHold::take();
            match crate::tty::console_bridge::<G>() {
                Some(d) if d.console_tty() => {
                    let error = d.console_write(&mut auio);
                    let mut crlf = [Iovec::new()];
                    let mut auio = Uio::kernel_write(&mut crlf, b"\r\n", Some(p));
                    let _ = d.console_write(&mut auio);
                    error
                }
                // XXX console redirection breaks down...
                _ => console_putc(buf, start, nbyte),
            }
        }
    };

    // KTRACE: not configured.
    match fp {
        Some(fp) => {
            let _ = fp.frele(Some(p));
            error
        }
        None => match error {
            Err(Errno::EFAULT) => Err(Errno::EFAULT),
            _ => Err(Errno::ENOTCONN),
        },
    }
}

/// `dosendsyslog`'s last resort: the message's `len` bytes from `start` on, up to a NUL,
/// through `cnputc`, then a newline; a user message is copied in first (`M_TEMP`), and a
/// fault prints nothing but the newline.
fn console_putc(buf: SyslogBuf<'_>, start: usize, len: usize) -> Result<(), Errno> {
    let error = match buf {
        SyslogBuf::User(addr) => match MArray::new(M_TEMP, M_WAITOK, len, |_| 0u8) {
            Some(mut kbuf) => {
                let r = SyslogBuf::User(addr).read(start, &mut kbuf);
                if r.is_ok() {
                    putc_until_nul(&kbuf);
                }
                r
            }
            None => Err(Errno::ENOMEM),
        },
        SyslogBuf::Sys(b) => {
            let end = start.saturating_add(len).min(b.len());
            putc_until_nul(&b[start.min(end)..end]);
            Ok(())
        }
    };
    hal::cons::cnputc(b'\n');
    error
}

/// `cnputc` of each byte of `kbuf` up to a NUL.
fn putc_until_nul(kbuf: &[u8]) {
    for &c in kbuf {
        if c == 0 {
            break;
        }
        hal::cons::cnputc(c);
    }
}

/// The log device's state and the stash forgotten (not freed: an earlier test's memory),
/// for `bsd`'s host tests, whose glue's tables live for the run (`testing.md`). The buffers
/// hold no kernel memory.
#[cfg(any(test, feature = "doubles"))]
pub fn log_test_reset<G: ProcGlue>() {
    let log = LogDev::<G>::get();
    let fp = log.syslogf.write().take();
    if let Some(fp) = fp {
        fp.forget();
    }
    let mut st = log.stash.write();
    for m in &mut st.messages {
        if let Some(m) = m.take() {
            core::mem::forget(m);
        }
    }
    st.r#in = 0;
    st.out = 0;
    st.dropped = 0;
    st.error = 0;
    st.pid = 0;
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use std::format;
    use std::string::String;
    use std::sync::{Mutex, MutexGuard};
    use std::vec::Vec;

    use core::sync::atomic::Ordering;

    use super::*;
    use crate::cred::{CredRef, Ucred};
    use crate::descrip::{falloc, fdinsert, fdplock};
    use crate::event::tests::{Kf, Kg, T, new_kqueue, setup};
    use crate::event::{EVFILT_WRITE, test_knote, test_knote_drop};
    use crate::file::{File, FileOps};
    use crate::testutil::{self, TestFileData, TestStat, USER_BASE};
    use crate::tty::{TtyDevsw, register_ttydevsw};

    /// The tests that use the global buffers (`LOG_MTX`) run one at a time.
    static BUFS: Mutex<()> = Mutex::new(());

    fn bufs() -> MutexGuard<'static, ()> {
        BUFS.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Where the tests put their user messages (past every other test's range: `kern.msgbuf`
    /// copies up to the end of the double's user memory).
    const UMSG: usize = USER_BASE + 0xe000;

    /// `Errno::ENOTCONN`'s value on OpenBSD (`<sys/errno.h>`).
    const ENOTCONN_VALUE: i32 = 57;

    #[test]
    fn the_header_is_the_40_bytes_kern_msgbuf_copies_out() {
        assert_eq!(MSG_HDRSIZE, 40);
        let h = MsgbufHeader {
            msg_magic: MSG_MAGIC,
            msg_bufx: 1,
            msg_bufr: 2,
            msg_bufs: 3,
            msg_bufd: 4,
        };
        let mut b = [0u8; 40];
        assert!(pod_to_bytes(&h, &mut b));
        for (i, v) in [MSG_MAGIC, 1, 2, 3, 4].into_iter().enumerate() {
            assert_eq!(&b[i * 8..i * 8 + 8], &v.to_ne_bytes(), "field {i}");
        }
        // The ring follows the header, as in `struct msgbuf`.
        assert_eq!(core::mem::offset_of!(Msgbuf<24>, msg_bufc), 40);
        assert_eq!(MSG_RING + MSG_HDRSIZE, MSGBUFSIZE);
        assert_eq!(CONS_RING + MSG_HDRSIZE, 16 * 1024);
        assert_eq!(LIOCSFD, 0x8004_6c7f);
    }

    #[test]
    fn the_ring_wraps_and_drops_the_oldest() {
        let mut mb = Msgbuf::<24>::new();
        mb.init(24);
        for b in b"0123456789abcdefghijklm" {
            mb.putchar_locked(*b);
        }
        assert_eq!(
            (mb.hdr.msg_bufx, mb.hdr.msg_bufr, mb.hdr.msg_bufd),
            (23, 0, 0)
        );
        assert_eq!(mb.getlen(), 23);
        mb.putchar_locked(b'n'); // fills the ring: bufx wraps to 0 == bufr
        assert_eq!(
            (mb.hdr.msg_bufx, mb.hdr.msg_bufr, mb.hdr.msg_bufd),
            (0, 1, 1)
        );
        assert_eq!(mb.getlen(), 23);
        mb.putchar_locked(b'o'); // still full: the read pointer keeps running ahead
        assert_eq!(mb.msg_bufc[0], b'o');
        assert_eq!((mb.hdr.msg_bufr, mb.hdr.msg_bufd), (2, 2));
        // read_at stops at the ring's end, never wraps.
        let mut out = [0u8; 8];
        assert_eq!(mb.read_at(20, &mut out), 4);
        assert_eq!(&out[..4], b"klmn");
        assert_eq!(mb.read_at(24, &mut out), 0);
    }

    #[test]
    fn init_keeps_a_sane_header_and_resets_a_bad_one() {
        let mut mb = Msgbuf::<24>::new();
        mb.msg_bufc = [0xff; 24];
        mb.init(24);
        assert_eq!(mb.hdr.msg_magic, MSG_MAGIC);
        assert_eq!(mb.hdr.msg_bufs, 24);
        assert_eq!(mb.hdr.msg_bufx, 0);
        assert_eq!(mb.msg_bufc, [0; 24], "a bad header clears the ring too");
        mb.putchar_locked(b'a');
        // A second init over the same bytes sees a valid header and starts a fresh line.
        mb.init(24);
        assert_eq!(
            mb.hdr.msg_bufx, 2,
            "a newline was appended after the kept 'a'"
        );
        assert_eq!(mb.msg_bufc[1], b'\n');
        // A header that disagrees with the size is reset.
        mb.hdr.msg_bufs = 23;
        mb.init(24);
        assert_eq!((mb.hdr.msg_bufs, mb.hdr.msg_bufx), (24, 0));
        // A write pointer outside the ring (left by a bad header) starts over at 0.
        mb.hdr.msg_bufx = 99;
        mb.putchar_locked(b'z');
        assert_eq!((mb.msg_bufc[0], mb.hdr.msg_bufx), (b'z', 1));
    }

    #[test]
    fn snprintf_truncates_and_counts_as_the_c() {
        let mut b = [0u8; 8];
        assert_eq!(snprintf(&mut b, format_args!("{}", "abc")), 3);
        assert_eq!(&b[..3], b"abc");
        assert_eq!(snprintf(&mut b, format_args!("{}", "0123456789")), 10);
        assert_eq!(&b[..7], b"0123456", "the last byte is the NUL's");
        assert_eq!(b[7], 0);
    }

    /// The message buffer emptied and set up afresh (the tests share it).
    fn fresh_msgbuf() {
        initmsgbuf();
        let mut g = LOG_MTX.lock();
        g.msgbuf = Msgbuf::new();
        g.msgbuf.init(MSG_RING as i64);
    }

    #[test]
    fn the_static_buffers_set_up_as_the_c_s() {
        let _b = bufs();
        fresh_msgbuf();
        assert!(msgbufmapped());
        assert_eq!(msgbuf_size(Buf::Msgbuf), Ok(MSG_RING as i64));
        initconsbuf();
        assert_eq!(msgbuf_size(Buf::Consbuf), Ok(CONS_RING as i64));
        msgbuf_putchar(Buf::Consbuf, b'x');
        msgbuf_putchar(Buf::Msgbuf, b'y');
        let h = msgbuf_header(Buf::Consbuf).unwrap();
        assert_eq!((h.msg_bufx, h.msg_bufr), (1, 0));
        let mut one = [0u8; 1];
        assert_eq!(msgbuf_read(Buf::Consbuf, 0, &mut one), 1);
        assert_eq!(one, *b"x");
        assert_eq!(msgbuf_read(Buf::Msgbuf, 0, &mut one), 1);
        assert_eq!(one, *b"y");
        // A buffer whose magic is gone is the C's NULL: ENXIO, and printing does nothing.
        LOG_MTX.lock().consbuf.hdr.msg_magic = 0;
        assert_eq!(msgbuf_size(Buf::Consbuf), Err(Errno::ENXIO));
        msgbuf_putchar(Buf::Consbuf, b'z');
        assert_eq!(LOG_MTX.lock().consbuf.hdr.msg_bufx, 1);
        initconsbuf();
        assert_eq!(LOG_MTX.lock().consbuf.hdr.msg_bufx, 0);
    }

    #[test]
    fn kern_msgbuf_copies_the_header_then_the_ring() {
        let _b = bufs();
        testutil::install();
        fresh_msgbuf();
        for c in b"dmesg line\n" {
            msgbuf_putchar(Buf::Msgbuf, *c);
        }
        let mut len = MSG_HDRSIZE + MSG_RING;
        // Errors first: a new value, a short buffer, a size query.
        assert_eq!(
            sysctl_msgbuf(Buf::Msgbuf, UMSG, &mut len, 1),
            Err(Errno::EPERM)
        );
        let mut short = MSG_HDRSIZE + MSG_RING - 1;
        assert_eq!(
            sysctl_msgbuf(Buf::Msgbuf, UMSG, &mut short, 0),
            Err(Errno::ENOMEM)
        );
        assert_eq!(sysctl_msgbuf(Buf::Msgbuf, 0, &mut len, 0), Ok(()));
        assert_eq!(len, MSG_HDRSIZE + MSG_RING, "the C leaves *oldlenp alone");
        // The whole buffer does not fit the double's user memory: the copy faults past it,
        // after the header and the first chunks.
        let r = sysctl_msgbuf(Buf::Msgbuf, UMSG, &mut len, 0);
        if MSG_HDRSIZE + MSG_RING > testutil::USER_LEN - 0xe000 {
            assert_eq!(r, Err(Errno::EFAULT));
        } else {
            assert_eq!(r, Ok(()));
        }
        let got = testutil::user_bytes(UMSG, MSG_HDRSIZE + 11).unwrap();
        assert_eq!(&got[..8], &MSG_MAGIC.to_ne_bytes());
        assert_eq!(&got[8..16], &11i64.to_ne_bytes(), "msg_bufx");
        assert_eq!(&got[24..32], &(MSG_RING as i64).to_ne_bytes(), "msg_bufs");
        assert_eq!(&got[40..], b"dmesg line\n");
    }

    /// A reader's uio: the bytes it took, at most `room`.
    struct Reader {
        room: usize,
        got: Vec<u8>,
    }

    impl UioMove for Reader {
        fn resid(&self) -> usize {
            self.room - self.got.len()
        }
        fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
            let n = buf.len().min(self.resid());
            self.got.extend_from_slice(&buf[..n]);
            Ok(())
        }
        fn nonblock(&self) -> bool {
            false
        }
        fn is_write(&self) -> bool {
            false
        }
        fn set_resid(&mut self, resid: usize) {
            self.room = self.got.len() + resid;
        }
    }

    fn read(room: usize) -> (Result<(), Errno>, Vec<u8>) {
        let mut r = Reader {
            room,
            got: Vec::new(),
        };
        let e = logread::<Kg>(&mut r, IO_NDELAY);
        (e, r.got)
    }

    #[test]
    fn logread_gives_the_drop_line_then_the_bytes_and_never_blocks_with_ndelay() {
        let _b = bufs();
        let _p = setup(7101);
        fresh_msgbuf();
        assert_eq!(read(64), (Err(Errno::EWOULDBLOCK), Vec::new()));
        for c in b"abc\n" {
            msgbuf_putchar(Buf::Msgbuf, *c);
        }
        LOG_MTX.lock().msgbuf.hdr.msg_bufd = 3;
        let (e, got) = read(1000);
        assert_eq!(e, Ok(()));
        let line = b"<4>klog: dropped 3 bytes, message buffer full\nabc\n";
        assert_eq!(got, line);
        assert_eq!(LOG_MTX.lock().msgbuf.hdr.msg_bufd, 0);
        // All read: the next one would block.
        assert_eq!(read(64).0, Err(Errno::EWOULDBLOCK));
        // One dropped byte has no "s"; a short reader takes part of the ring and leaves the
        // rest.
        LOG_MTX.lock().msgbuf.hdr.msg_bufd = 1;
        for c in b"0123456789" {
            msgbuf_putchar(Buf::Msgbuf, *c);
        }
        let (_, got) = read(48);
        assert_eq!(
            String::from_utf8(got).unwrap(),
            "<4>klog: dropped 1 byte, message buffer full\n012"
        );
        assert_eq!(read(100).1, b"3456789");
        // Across the ring's wrap.
        {
            let mut g = LOG_MTX.lock();
            let s = g.msgbuf.hdr.msg_bufs;
            g.msgbuf.hdr.msg_bufx = s - 2;
            g.msgbuf.hdr.msg_bufr = s - 2;
        }
        for c in b"wxyz" {
            msgbuf_putchar(Buf::Msgbuf, *c);
        }
        assert_eq!(read(100).1, b"wxyz");
    }

    #[test]
    fn klog_opens_once_and_closes() {
        let p = setup(7102);
        assert!(!log_open::<Kg>());
        assert_eq!(logopen::<Kg>(), Ok(()));
        assert!(log_open::<Kg>());
        assert!(LogDev::<Kg>::get().tick.pending(), "logopen armed the tick");
        assert_eq!(logopen::<Kg>(), Err(Errno::EBUSY));
        assert_eq!(logclose::<Kg>(Some(p)), Ok(()));
        assert!(!log_open::<Kg>());
        assert!(!LogDev::<Kg>::get().tick.pending());
        assert_eq!(logopen::<Kg>(), Ok(()));
        assert_eq!(logclose::<Kg>(Some(p)), Ok(()));
    }

    #[test]
    fn logtick_wakes_only_after_logwakeup_and_rearms_while_open() {
        let p = setup(7103);
        let log = LogDev::<Kg>::get();
        // Closed: nothing, not even the re-arm.
        logwakeup::<Kg>();
        logtick::<Kg>(ptr::null_mut());
        assert!(!log.tick.pending());
        assert!(log.need_wakeup.load(Ordering::Relaxed));
        assert_eq!(logopen::<Kg>(), Ok(()));
        log.state.store(LOG_RDWAIT | LOG_ASYNC, Ordering::Relaxed);
        logtick::<Kg>(ptr::null_mut());
        assert!(!log.need_wakeup.load(Ordering::Relaxed));
        assert_eq!(
            log.state.load(Ordering::Relaxed),
            LOG_ASYNC,
            "the reader's flag cleared, as its wakeup is sent"
        );
        assert!(log.tick.pending());
        // No logwakeup since: the state stays.
        log.state.fetch_or(LOG_RDWAIT, Ordering::Relaxed);
        logtick::<Kg>(ptr::null_mut());
        assert_ne!(log.state.load(Ordering::Relaxed) & LOG_RDWAIT, 0);
        assert_eq!(logclose::<Kg>(Some(p)), Ok(()));
        assert_eq!(log.state.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn the_read_filter_hooks_on_the_klist_and_counts_the_unread_bytes() {
        let _b = bufs();
        let p = setup(7104);
        let (_kfd, kq) = new_kqueue(p);
        let h = test_knote(&kq, 1);
        let kn = h.kn();
        kn.kn_kevent.filter.set(EVFILT_WRITE);
        assert_eq!(logkqfilter::<Kg>(kn), Err(Errno::EINVAL));
        kn.kn_kevent.filter.set(EVFILT_READ);
        assert_eq!(logkqfilter::<Kg>(kn), Ok(()));
        let log = LogDev::<Kg>::get();
        assert!(!log.klist.is_empty());
        // logread_filtops: ISFD and MPSAFE, every entry LogRead's.
        assert_eq!(kn.fop().f_flags, FILTEROP_ISFD | FILTEROP_MPSAFE);
        assert!(kn.fop().f_modify.is_some() && kn.fop().f_process.is_some());

        fresh_msgbuf();
        let event = kn.fop().f_event.unwrap();
        assert!(!event(kn, 0));
        assert_eq!(kn.kn_data().get(), 0);
        for c in b"12345" {
            msgbuf_putchar(Buf::Msgbuf, *c);
        }
        assert!(event(kn, 0));
        assert_eq!(kn.kn_data().get(), 5);

        (kn.fop().f_detach.unwrap())(kn);
        assert!(log.klist.is_empty());
        test_knote_drop(&kq, h);
    }

    /// The files the tests hand `LIOCSFD`: a "socket" whose writes are recorded as (segment,
    /// length, flags), the bytes of a user write included; object 9 fails with `EPIPE`.
    struct SockOps;

    std::thread_local! {
        static WRITES: core::cell::RefCell<Vec<(UioSeg, usize, Vec<u8>)>> =
            const { core::cell::RefCell::new(Vec::new()) };
    }

    fn writes() -> Vec<(UioSeg, usize, Vec<u8>)> {
        WRITES.with(|w| core::mem::take(&mut *w.borrow_mut()))
    }

    impl FileOps<Kf> for SockOps {
        fn read(&self, _: &File<Kf>, _: &mut Uio<'_, Kg>, _: i32) -> Result<(), Errno> {
            Ok(())
        }
        fn write(&self, fp: &File<Kf>, uio: &mut Uio<'_, Kg>, _: i32) -> Result<(), Errno> {
            if fp.data().map_or(0, |d| d.id) == 9 {
                return Err(Errno::EPIPE);
            }
            let iov = uio.uio_iov[0];
            let bytes = match uio.uio_segflg {
                UioSeg::UIO_USERSPACE => {
                    testutil::user_bytes(iov.iov_base.addr(), iov.iov_len).unwrap_or_default()
                }
                UioSeg::UIO_SYSSPACE => Vec::new(),
            };
            WRITES.with(|w| w.borrow_mut().push((uio.uio_segflg, iov.iov_len, bytes)));
            uio.uio_resid = 0;
            Ok(())
        }
        fn ioctl(&self, _: &File<Kf>, _: u64, _: &mut [u8], _: &Thread<Kg>) -> Result<(), Errno> {
            Err(Errno::ENOTTY)
        }
        fn kqfilter(&self, _: &File<Kf>, _: &Knote<Kg>) -> Result<(), Errno> {
            Err(Errno::EINVAL)
        }
        fn stat(&self, _: &File<Kf>, _: &mut TestStat, _: &Thread<Kg>) -> Result<(), Errno> {
            Ok(())
        }
        fn close(&self, _: &File<Kf>, _: Option<&Thread<Kg>>) -> Result<(), Errno> {
            Ok(())
        }
    }

    static SOCKOPS: SockOps = SockOps;

    /// A file of type `ty` on object `oid` at the lowest free descriptor.
    fn open_file(p: T, ty: i32, oid: usize) -> i32 {
        let mut g = fdplock(p.fd());
        let (fp, fd) = falloc(p, &mut g).unwrap();
        fp.install(
            ty,
            &SOCKOPS,
            TestFileData {
                id: oid,
                ..TestFileData::default()
            },
        );
        fdinsert(&mut g, fd, 0, &fp);
        drop(g);
        fp.frele(Some(p)).unwrap();
        fd
    }

    fn ioctl_int(com: u64, v: i32, p: T) -> (Result<(), Errno>, i32) {
        let mut data = v.to_ne_bytes();
        let e = logioctl::<Kg>(com, &mut data, p);
        (e, i32::from_ne_bytes(data))
    }

    #[test]
    fn logioctl_answers_each_command_as_the_c() {
        let _b = bufs();
        let p = setup(7105);
        let log = LogDev::<Kg>::get();
        fresh_msgbuf();
        for c in b"xyz" {
            msgbuf_putchar(Buf::Msgbuf, *c);
        }
        assert_eq!(ioctl_int(FIONREAD, 0, p), (Ok(()), 3));
        assert_eq!(ioctl_int(FIOASYNC, 1, p).0, Ok(()));
        assert_ne!(log.state.load(Ordering::Relaxed) & LOG_ASYNC, 0);
        assert_eq!(ioctl_int(FIOASYNC, 0, p).0, Ok(()));
        assert_eq!(log.state.load(Ordering::Relaxed) & LOG_ASYNC, 0);
        // The SIGIO owner: none, then this process, read back (negated by TIOCGPGRP).
        assert_eq!(ioctl_int(FIOGETOWN, -1, p), (Ok(()), 0));
        Kg::set_curproc(Some(p));
        assert_eq!(ioctl_int(FIOSETOWN, 7105, p).0, Ok(()));
        Kg::set_curproc(None);
        assert_eq!(ioctl_int(FIOGETOWN, 0, p), (Ok(()), 7105));
        assert_eq!(ioctl_int(TIOCGPGRP, 0, p), (Ok(()), -7105));
        assert_eq!(ioctl_int(FIOSETOWN, 0, p).0, Ok(()));
        // LIOCSFD: a bad descriptor, a file that is no socket, a socket.
        assert_eq!(ioctl_int(LIOCSFD, 99, p).0, Err(Errno::EBADF));
        let pipe = open_file(p, crate::file::DTYPE_PIPE, 1);
        assert_eq!(ioctl_int(LIOCSFD, pipe, p).0, Err(Errno::ENOTSOCK));
        assert!(log.syslogf.read().is_none());
        let sock = open_file(p, DTYPE_SOCKET, 2);
        assert_eq!(ioctl_int(LIOCSFD, sock, p).0, Ok(()));
        assert!(log.syslogf.read().is_some());
        // Another command is ENOTTY.
        assert_eq!(ioctl_int(TIOCSPGRP + 1, 0, p).0, Err(Errno::ENOTTY));
        // Not root: EPERM before the descriptor is looked at.
        let mut ids = Ucred::new();
        ids.cr_uid.set(1000);
        drop(p.set_cred(CredRef::new(ids)));
        assert_eq!(ioctl_int(LIOCSFD, sock, p).0, Err(Errno::EPERM));
        // logclose drops syslogf.
        assert_eq!(logclose::<Kg>(Some(p)), Ok(()));
        assert!(log.syslogf.read().is_none());
    }

    /// The console double: what `cnwrite` was given, as (segment, length, user bytes).
    struct Console {
        tty: bool,
        seen: Mutex<Vec<(UioSeg, usize, Vec<u8>)>>,
    }

    impl TtyDevsw<Kg> for Console {
        fn console_tty(&self) -> bool {
            self.tty
        }
        fn stop(&self, _tp: &crate::tty::Tty<Kg>, _rw: i32) {}
        fn console_access(&self, _p: &Thread<Kg>) -> Result<(), Errno> {
            Ok(())
        }
        fn session_has_ttyvp(&self, _sess: &crate::proc::Session<Kg>) -> bool {
            false
        }
        fn console_write(&self, uio: &mut Uio<'_, Kg>) -> Result<(), Errno> {
            let iov = uio.uio_iov[0];
            let bytes = match uio.uio_segflg {
                UioSeg::UIO_USERSPACE => {
                    testutil::user_bytes(iov.iov_base.addr(), iov.iov_len).unwrap_or_default()
                }
                UioSeg::UIO_SYSSPACE => Vec::new(),
            };
            self.seen
                .lock()
                .unwrap()
                .push((uio.uio_segflg, iov.iov_len, bytes));
            uio.uio_resid = 0;
            Ok(())
        }
    }

    fn put_user(addr: usize, s: &[u8]) {
        UserPtr::<u8>::new(addr).copyout_slice(s).unwrap();
    }

    fn stash_len() -> usize {
        let st = LogDev::<Kg>::get().stash.read();
        st.messages.iter().filter(|m| m.is_some()).count()
    }

    #[test]
    fn sendsyslog_without_syslogd_strips_the_priority_for_the_console_and_stashes() {
        let p = setup(7106);
        let console: &'static Console = std::boxed::Box::leak(std::boxed::Box::new(Console {
            tty: true,
            seen: Mutex::new(Vec::new()),
        }));
        register_ttydevsw::<Kg>(console);
        put_user(UMSG, b"<13>hello");
        // Without LOG_CONS: ENOTCONN, and the message waits in the stash.
        assert_eq!(sys_sendsyslog(p, UMSG, 9, 0), Err(Errno::ENOTCONN));
        assert_eq!(stash_len(), 1);
        assert!(console.seen.lock().unwrap().is_empty());
        // With LOG_CONS: the console gets "hello" (the "<13>" stripped) then "\r\n", and the
        // caller still hears ENOTCONN, the message stashed.
        assert_eq!(sys_sendsyslog(p, UMSG, 9, LOG_CONS), Err(Errno::ENOTCONN));
        let seen = core::mem::take(&mut *console.seen.lock().unwrap());
        assert_eq!(
            seen,
            [
                (UioSeg::UIO_USERSPACE, 5, b"hello".to_vec()),
                (UioSeg::UIO_SYSSPACE, 2, Vec::new()),
            ]
        );
        assert_eq!(stash_len(), 2);
        // The prefix must be <digits> within the first six bytes: "<>x", "<1x" and
        // "<12345>a" keep every byte; "<0>" and "<1234>" go.
        for (msg, kept) in [
            (&b"<>x"[..], 3),
            (b"<1x", 3),
            (b"<0>", 0),
            (b"<1234>ab", 2),
            (b"<12345>a", 8),
        ] {
            put_user(UMSG, msg);
            let _ = sys_sendsyslog(p, UMSG, msg.len(), LOG_CONS);
            let seen = core::mem::take(&mut *console.seen.lock().unwrap());
            assert_eq!(seen[0].1, kept, "{}", String::from_utf8_lossy(msg));
        }
        // A bad address: EFAULT, not stashed.
        let before = stash_len();
        assert_eq!(
            sys_sendsyslog(p, USER_BASE - 0x1000, 5, LOG_CONS),
            Err(Errno::EFAULT)
        );
        assert_eq!(stash_len(), before);
    }

    #[test]
    fn sendsyslog_flushes_the_stash_to_syslogd_first_and_counts_what_it_dropped() {
        let p = setup(7107);
        put_user(UMSG + 0x100, b"<14>m");
        // Fill the stash, then two more: dropped, counted with the first error and the pid.
        for _ in 0..LOGSTASH_SIZE + 2 {
            assert_eq!(sys_sendsyslog(p, UMSG + 0x100, 5, 0), Err(Errno::ENOTCONN));
        }
        assert_eq!(stash_len(), LOGSTASH_SIZE);
        {
            let st = LogDev::<Kg>::get().stash.read();
            assert_eq!((st.dropped, st.error, st.pid), (2, ENOTCONN_VALUE, 7107));
        }
        // syslogd comes: the next message flushes the stash, oldest first, the drop note in
        // the place of the dropped messages, then the new message.
        let sock = open_file(p, DTYPE_SOCKET, 3);
        let (e, _) = ioctl_int(LIOCSFD, sock, p);
        assert_eq!(e, Ok(()));
        put_user(UMSG + 0x200, b"<14>new");
        assert_eq!(sys_sendsyslog(p, UMSG + 0x200, 7, 0), Ok(()));
        let w = writes();
        assert_eq!(w.len(), LOGSTASH_SIZE + 2);
        assert!(
            w[..LOGSTASH_SIZE]
                .iter()
                .all(|(seg, n, _)| *seg == UioSeg::UIO_SYSSPACE && *n == 5)
        );
        let note = format!("<4>sendsyslog: dropped 2 messages, error {ENOTCONN_VALUE}, pid 7107");
        assert_eq!(w[LOGSTASH_SIZE].0, UioSeg::UIO_SYSSPACE);
        assert_eq!(w[LOGSTASH_SIZE].1, note.len());
        assert_eq!(
            w[LOGSTASH_SIZE + 1],
            (UioSeg::UIO_USERSPACE, 7, b"<14>new".to_vec())
        );
        assert_eq!(stash_len(), 0);
        {
            let st = LogDev::<Kg>::get().stash.read();
            assert_eq!((st.dropped, st.error, st.pid), (0, 0, 0));
        }
        // A socket that fails: the message is stashed with the error, and the next call tries
        // the stash first and stops at its first failure.
        assert_eq!(logclose::<Kg>(Some(p)), Ok(()));
        let bad = open_file(p, DTYPE_SOCKET, 9);
        assert_eq!(ioctl_int(LIOCSFD, bad, p).0, Ok(()));
        assert_eq!(sys_sendsyslog(p, UMSG + 0x200, 7, 0), Err(Errno::EPIPE));
        assert_eq!(stash_len(), 1);
        assert_eq!(logstash_sendsyslog(p), Err(Errno::EPIPE));
        assert_eq!(stash_len(), 1);
        assert_eq!(logclose::<Kg>(Some(p)), Ok(()));
        log_test_reset::<Kg>();
        assert_eq!(stash_len(), 0);
    }

    #[test]
    fn priorities_and_facilities_encode_as_syslog_h() {
        assert_eq!(log_pri(LOG_DAEMON | LOG_WARNING), LOG_WARNING);
        assert_eq!(log_fac(LOG_DAEMON | LOG_WARNING), 3);
        assert_eq!(log_fac(LOG_LOCAL7), 23);
        assert_eq!(log_mask(LOG_ERR), 0b1000);
        assert_eq!(log_upto(LOG_ERR), 0b1111);
        assert_eq!(LOG_KERN | LOG_WARNING, 4, "the drop lines' <4>");
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_headers() {
        let defs = crate::reftest::defines("sys/sys/syslog.h");
        let ours: &[(&str, i64)] = &[
            ("LOG_MAXLINE", LOG_MAXLINE as i64),
            ("LOG_EMERG", LOG_EMERG as i64),
            ("LOG_ALERT", LOG_ALERT as i64),
            ("LOG_CRIT", LOG_CRIT as i64),
            ("LOG_ERR", LOG_ERR as i64),
            ("LOG_WARNING", LOG_WARNING as i64),
            ("LOG_NOTICE", LOG_NOTICE as i64),
            ("LOG_INFO", LOG_INFO as i64),
            ("LOG_DEBUG", LOG_DEBUG as i64),
            ("LOG_PRIMASK", LOG_PRIMASK as i64),
            ("LOG_KERN", LOG_KERN as i64),
            ("LOG_USER", LOG_USER as i64),
            ("LOG_MAIL", LOG_MAIL as i64),
            ("LOG_DAEMON", LOG_DAEMON as i64),
            ("LOG_AUTH", LOG_AUTH as i64),
            ("LOG_SYSLOG", LOG_SYSLOG as i64),
            ("LOG_LPR", LOG_LPR as i64),
            ("LOG_NEWS", LOG_NEWS as i64),
            ("LOG_UUCP", LOG_UUCP as i64),
            ("LOG_CRON", LOG_CRON as i64),
            ("LOG_AUTHPRIV", LOG_AUTHPRIV as i64),
            ("LOG_FTP", LOG_FTP as i64),
            ("LOG_LOCAL0", LOG_LOCAL0 as i64),
            ("LOG_LOCAL7", LOG_LOCAL7 as i64),
            ("LOG_NFACILITIES", LOG_NFACILITIES as i64),
            ("LOG_FACMASK", LOG_FACMASK as i64),
            ("LOG_PRINTF", LOG_PRINTF as i64),
            ("LOG_PID", LOG_PID as i64),
            ("LOG_CONS", LOG_CONS as i64),
            ("LOG_ODELAY", LOG_ODELAY as i64),
            ("LOG_NDELAY", LOG_NDELAY as i64),
            ("LOG_NOWAIT", LOG_NOWAIT as i64),
            ("LOG_PERROR", LOG_PERROR as i64),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
        let defs = crate::reftest::defines("sys/sys/msgbuf.h");
        assert_eq!(crate::reftest::int(&defs, "MSG_MAGIC"), Some(MSG_MAGIC));
        assert_eq!(
            crate::reftest::int(&defs, "CONSBUFSIZE"),
            Some(CONSBUFSIZE as i64)
        );
        let defs = crate::reftest::defines("sys/kern/subr_log.c");
        assert_eq!(
            crate::reftest::int(&defs, "LOG_TICK"),
            Some(LOG_TICK as i64)
        );
        assert_eq!(
            crate::reftest::int(&defs, "LOG_ASYNC"),
            Some(LOG_ASYNC.into())
        );
        assert_eq!(
            crate::reftest::int(&defs, "LOG_RDWAIT"),
            Some(LOG_RDWAIT.into())
        );
        assert_eq!(
            crate::reftest::int(&defs, "LOGSTASH_SIZE"),
            Some(LOGSTASH_SIZE as i64)
        );
    }

    #[test]
    fn the_log_device_is_send_and_sync() {
        const fn send_sync<X: Send + Sync>() {}
        send_sync::<LogDev<Kg>>();
        send_sync::<LogBufs>();
        send_sync::<Logstash>();
    }
}
/* </TESTS> */
