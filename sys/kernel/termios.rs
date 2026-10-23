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
 * Copyright (c) 1988, 1989, 1993, 1994
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
 *	@(#)termios.h	8.3 (Berkeley) 3/28/94
 */

/*-
 * Copyright (c) 1982, 1986, 1990, 1993, 1994
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
 *	@(#)ttycom.h	8.1 (Berkeley) 3/28/94
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
 *	@(#)ttydefaults.h	8.4 (Berkeley) 1/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The terminal's ABI headers: `<sys/termios.h>` (the attributes, `struct termios`, the
//! speeds), `<sys/ttycom.h>` (the tty `ioctl(2)` commands, `struct winsize`, `struct
//! tstamps`, the modem bits and the line discipline numbers) and `<sys/ttydefaults.h>` (the
//! defaults a terminal gets on its first open), as the kernel sees them.
//!
//! Upstream: sys/sys/termios.h @ 3ce1f3f79392
//! Upstream: sys/sys/ttycom.h @ 3ce1f3f79392
//! Upstream: sys/sys/ttydefaults.h @ 3ce1f3f79392
//! LZ: sys/sys/termios.rs@3c62ede68802
//! LZ: sys/sys/ttycom.rs@3c62ede68802
//! LZ: sys/sys/ttydefaults.rs@3c62ede68802
//!
//! Every value here is what OpenBSD's userland is compiled with: the structures' layouts
//! (`struct termios` 44 bytes, `struct winsize` 8, `struct tstamps` 8) and the command
//! numbers (`commands_encode_as_on_openbsd` pins the ones userland's binaries carry).
//!
//! ## Deviations
//! - Every flag is visible (`__BSD_VISIBLE`); `CCEQ` is [`cceq`], `CTRL(x)` [`ctrl`],
//!   `UIOCCMD(n)` [`uioccmd`]; the commands are `u64` (`u_long` in C).
//! - The modem, packet and flag bits are `i32`: they travel in the `int` argument of
//!   `TIOCMSET`, `TIOCPKT` and `TIOCSFLAGS`.
//! - The `tcsetattr` family of prototypes is userland and not here; the `TCIFLUSH`..`TCION`
//!   constants of the same `#ifndef _KERNEL` block are, so the header is complete for the
//!   userland that is built against it.
//! - `ttydefchars[]` (behind `TTYDEFCHARS` in C) is always defined, as [`TTYDEFCHARS`].
//!
//! ## Redesign
//! - LZ's three modules (`sys/sys/termios.rs`, `ttycom.rs`, `ttydefaults.rs`) are one here:
//!   the kernel's terminal (`kernel::tty`, N4.7b) and its users name one path, and
//!   `ttydefaults.h` and `ttycom.h` exist only to complete `termios.h`'s types.
//! - `struct termios`, `struct winsize` and `struct tstamps` are `hal::abi_pod!` types (LZ:
//!   an `unsafe impl AbiPod` each, three `unsafe`, over the legacy trait of
//!   `sys/machine/copy.rs`): `TIOCGETA`/`TIOCSETA`, `TIOCGWINSZ` and `TIOCSTSTAMP` read and
//!   write them with `hal::user::pod_from_bytes`/`pod_to_bytes`, no `unsafe` anywhere.
//! - `TIOCGTSTAMP` is sized by the kernel's `struct timeval` (`kernel::resource::Timeval`,
//!   two `i64`s, as LZ's).
//! - `bsd`'s three headers stay as they are until the terminal moves (N4.7b), when they
//!   become aliases of these.

use hal::sys::ioccom::{_io, _ior, _iow};

use crate::resource::Timeval;

/// `tcflag_t`: a set of terminal flags.
pub type Tcflag = u32;
/// `cc_t`: a control character.
pub type Cc = u8;
/// `speed_t`: a line speed in bits per second.
pub type Speed = u32;

// Special Control Characters: index into c_cc[]; the comment says which flag enables each.

/// End of file (ICANON).
pub const VEOF: u32 = 0;
/// End of line (ICANON).
pub const VEOL: u32 = 1;
/// Second end of line (ICANON).
pub const VEOL2: u32 = 2;
/// Erase (ICANON).
pub const VERASE: u32 = 3;
/// Word erase (ICANON).
pub const VWERASE: u32 = 4;
/// Kill (ICANON).
pub const VKILL: u32 = 5;
/// Reprint (ICANON).
pub const VREPRINT: u32 = 6;
/// Interrupt (ISIG).
pub const VINTR: u32 = 8;
/// Quit (ISIG).
pub const VQUIT: u32 = 9;
/// Suspend (ISIG).
pub const VSUSP: u32 = 10;
/// Delayed suspend (ISIG).
pub const VDSUSP: u32 = 11;
/// Start (IXON, IXOFF).
pub const VSTART: u32 = 12;
/// Stop (IXON, IXOFF).
pub const VSTOP: u32 = 13;
/// Literal next (IEXTEN).
pub const VLNEXT: u32 = 14;
/// Discard (IEXTEN).
pub const VDISCARD: u32 = 15;
/// Minimum characters (!ICANON).
pub const VMIN: u32 = 16;
/// Timeout (!ICANON).
pub const VTIME: u32 = 17;
/// Status (ICANON).
pub const VSTATUS: u32 = 18;
/// Number of control characters.
pub const NCCS: usize = 20;

/// The control character value that disables its function.
pub const _POSIX_VDISABLE: Cc = 0o377;

// Input flags - software input processing

/// Ignore BREAK condition.
pub const IGNBRK: Tcflag = 0x0000_0001;
/// Map BREAK to SIGINT.
pub const BRKINT: Tcflag = 0x0000_0002;
/// Ignore (discard) parity errors.
pub const IGNPAR: Tcflag = 0x0000_0004;
/// Mark parity and framing errors.
pub const PARMRK: Tcflag = 0x0000_0008;
/// Enable checking of parity errors.
pub const INPCK: Tcflag = 0x0000_0010;
/// Strip 8th bit off chars.
pub const ISTRIP: Tcflag = 0x0000_0020;
/// Map NL into CR.
pub const INLCR: Tcflag = 0x0000_0040;
/// Ignore CR.
pub const IGNCR: Tcflag = 0x0000_0080;
/// Map CR to NL (ala CRMOD).
pub const ICRNL: Tcflag = 0x0000_0100;
/// Enable output flow control.
pub const IXON: Tcflag = 0x0000_0200;
/// Enable input flow control.
pub const IXOFF: Tcflag = 0x0000_0400;
/// Any char will restart after stop.
pub const IXANY: Tcflag = 0x0000_0800;
/// Translate upper to lower case.
pub const IUCLC: Tcflag = 0x0000_1000;
/// Ring bell on input queue full.
pub const IMAXBEL: Tcflag = 0x0000_2000;

// Output flags - software output processing

/// Enable following output processing.
pub const OPOST: Tcflag = 0x0000_0001;
/// Map NL to CR-NL (ala CRMOD).
pub const ONLCR: Tcflag = 0x0000_0002;
/// Horizontal tab delay mask.
pub const TABDLY: Tcflag = 0x0000_0004;
/// No tab delay or expansion.
pub const TAB0: Tcflag = 0x0000_0000;
/// Expand tabs to spaces.
pub const TAB3: Tcflag = 0x0000_0004;
/// BSD name for TAB3.
pub const OXTABS: Tcflag = TAB3;
/// Discard EOT's (^D) on output.
pub const ONOEOT: Tcflag = 0x0000_0008;
/// Map CR to NL.
pub const OCRNL: Tcflag = 0x0000_0010;
/// Translate lower case to upper case.
pub const OLCUC: Tcflag = 0x0000_0020;
/// No CR output at column 0.
pub const ONOCR: Tcflag = 0x0000_0040;
/// NL performs the CR function.
pub const ONLRET: Tcflag = 0x0000_0080;

// Control flags - hardware control of terminal

/// Ignore control flags.
pub const CIGNORE: Tcflag = 0x0000_0001;
/// Character size mask.
pub const CSIZE: Tcflag = 0x0000_0300;
/// 5 bits (pseudo).
pub const CS5: Tcflag = 0x0000_0000;
/// 6 bits.
pub const CS6: Tcflag = 0x0000_0100;
/// 7 bits.
pub const CS7: Tcflag = 0x0000_0200;
/// 8 bits.
pub const CS8: Tcflag = 0x0000_0300;
/// Send 2 stop bits.
pub const CSTOPB: Tcflag = 0x0000_0400;
/// Enable receiver.
pub const CREAD: Tcflag = 0x0000_0800;
/// Parity enable.
pub const PARENB: Tcflag = 0x0000_1000;
/// Odd parity, else even.
pub const PARODD: Tcflag = 0x0000_2000;
/// Hang up on last close.
pub const HUPCL: Tcflag = 0x0000_4000;
/// Ignore modem status lines.
pub const CLOCAL: Tcflag = 0x0000_8000;
/// RTS/CTS full-duplex flow control.
pub const CRTSCTS: Tcflag = 0x0001_0000;
/// Compat name for CRTSCTS.
pub const CRTS_IFLOW: Tcflag = CRTSCTS;
/// Compat name for CRTSCTS.
pub const CCTS_OFLOW: Tcflag = CRTSCTS;
/// DTR/DCD hardware flow control.
pub const MDMBUF: Tcflag = 0x0010_0000;
/// All types of hw flow control.
pub const CHWFLOW: Tcflag = MDMBUF | CRTSCTS;

// "Local" flags - dumping ground for other state. Warning: some flags in this structure begin
// with the letter "I" and look like they belong in the input flag.

/// Visual erase for line kill.
pub const ECHOKE: Tcflag = 0x0000_0001;
/// Visually erase chars.
pub const ECHOE: Tcflag = 0x0000_0002;
/// Echo NL after line kill.
pub const ECHOK: Tcflag = 0x0000_0004;
/// Enable echoing.
pub const ECHO: Tcflag = 0x0000_0008;
/// Echo NL even if ECHO is off.
pub const ECHONL: Tcflag = 0x0000_0010;
/// Visual erase mode for hardcopy.
pub const ECHOPRT: Tcflag = 0x0000_0020;
/// Echo control chars as ^(Char).
pub const ECHOCTL: Tcflag = 0x0000_0040;
/// Enable signals INTR, QUIT, [D]SUSP.
pub const ISIG: Tcflag = 0x0000_0080;
/// Canonicalize input lines.
pub const ICANON: Tcflag = 0x0000_0100;
/// Use alternate WERASE algorithm.
pub const ALTWERASE: Tcflag = 0x0000_0200;
/// Enable DISCARD and LNEXT.
pub const IEXTEN: Tcflag = 0x0000_0400;
/// External processing.
pub const EXTPROC: Tcflag = 0x0000_0800;
/// Stop background jobs from output.
pub const TOSTOP: Tcflag = 0x0040_0000;
/// Output being flushed (state).
pub const FLUSHO: Tcflag = 0x0080_0000;
/// Canonical upper/lower case.
pub const XCASE: Tcflag = 0x0100_0000;
/// No kernel output from VSTATUS.
pub const NOKERNINFO: Tcflag = 0x0200_0000;
/// XXX retype pending input (state).
pub const PENDIN: Tcflag = 0x2000_0000;
/// Don't flush after interrupt.
pub const NOFLSH: Tcflag = 0x8000_0000;

/// `struct termios`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Termios {
    /// Input flags.
    pub c_iflag: Tcflag,
    /// Output flags.
    pub c_oflag: Tcflag,
    /// Control flags.
    pub c_cflag: Tcflag,
    /// Local flags.
    pub c_lflag: Tcflag,
    /// Control chars.
    pub c_cc: [Cc; NCCS],
    /// Input speed.
    pub c_ispeed: i32,
    /// Output speed.
    pub c_ospeed: i32,
}

impl Termios {
    /// An all-zero termios, what a freshly allocated `struct tty` holds.
    pub const fn zeroed() -> Self {
        Self {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_cc: [0; NCCS],
            c_ispeed: 0,
            c_ospeed: 0,
        }
    }
}

hal::abi_pod!(Termios {
    c_iflag: Tcflag,
    c_oflag: Tcflag,
    c_cflag: Tcflag,
    c_lflag: Tcflag,
    c_cc: [Cc; NCCS],
    c_ispeed: i32,
    c_ospeed: i32,
});

// Commands passed to tcsetattr() for setting the termios structure.

/// Make change immediate.
pub const TCSANOW: i32 = 0;
/// Drain output, then change.
pub const TCSADRAIN: i32 = 1;
/// Drain output, flush input.
pub const TCSAFLUSH: i32 = 2;
/// Flag: don't alter h.w. state.
pub const TCSASOFT: i32 = 0x10;

// Standard speeds

/// 0 baud (hang up).
pub const B0: Speed = 0;
/// 50 baud.
pub const B50: Speed = 50;
/// 75 baud.
pub const B75: Speed = 75;
/// 110 baud.
pub const B110: Speed = 110;
/// 134 baud.
pub const B134: Speed = 134;
/// 150 baud.
pub const B150: Speed = 150;
/// 200 baud.
pub const B200: Speed = 200;
/// 300 baud.
pub const B300: Speed = 300;
/// 600 baud.
pub const B600: Speed = 600;
/// 1200 baud.
pub const B1200: Speed = 1200;
/// 1800 baud.
pub const B1800: Speed = 1800;
/// 2400 baud.
pub const B2400: Speed = 2400;
/// 4800 baud.
pub const B4800: Speed = 4800;
/// 9600 baud.
pub const B9600: Speed = 9600;
/// 19200 baud.
pub const B19200: Speed = 19200;
/// 38400 baud.
pub const B38400: Speed = 38400;
/// 7200 baud.
pub const B7200: Speed = 7200;
/// 14400 baud.
pub const B14400: Speed = 14400;
/// 28800 baud.
pub const B28800: Speed = 28800;
/// 57600 baud.
pub const B57600: Speed = 57600;
/// 76800 baud.
pub const B76800: Speed = 76800;
/// 115200 baud.
pub const B115200: Speed = 115200;
/// 230400 baud.
pub const B230400: Speed = 230400;
/// External A clock (19200).
pub const EXTA: Speed = 19200;
/// External B clock (38400).
pub const EXTB: Speed = 38400;

// tcflush(3) queue selectors and tcflow(3) actions (userland, `#ifndef _KERNEL`).

/// Flush the input queue.
pub const TCIFLUSH: i32 = 1;
/// Flush the output queue.
pub const TCOFLUSH: i32 = 2;
/// Flush both queues.
pub const TCIOFLUSH: i32 = 3;
/// Suspend output.
pub const TCOOFF: i32 = 1;
/// Restart output.
pub const TCOON: i32 = 2;
/// Send a STOP character.
pub const TCIOFF: i32 = 3;
/// Send a START character.
pub const TCION: i32 = 4;

/// `CCEQ(val, c)`: whether `c` equals the control character `val`, which must be enabled.
pub const fn cceq(val: Cc, c: Cc) -> bool {
    if c == val {
        val != _POSIX_VDISABLE
    } else {
        false
    }
}

/// `struct winsize`: window/terminal size. This information is stored by the kernel in
/// order to provide a consistent interface, but is not used by the kernel.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Winsize {
    /// Rows, in characters.
    pub ws_row: u16,
    /// Columns, in characters.
    pub ws_col: u16,
    /// Horizontal size, pixels.
    pub ws_xpixel: u16,
    /// Vertical size, pixels.
    pub ws_ypixel: u16,
}

hal::abi_pod!(Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
});

/// `struct tstamps`: which modem transitions `TIOCGTSTAMP` records.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tstamps {
    /// `TIOCM_CAR` and/or `TIOCM_CTS`.
    pub ts_set: i32,
    /// Likewise, for the transitions that clear them.
    pub ts_clr: i32,
}

hal::abi_pod!(Tstamps {
    ts_set: i32,
    ts_clr: i32,
});

/// Line enable.
pub const TIOCM_LE: i32 = 0o001;
/// Data terminal ready.
pub const TIOCM_DTR: i32 = 0o002;
/// Request to send.
pub const TIOCM_RTS: i32 = 0o004;
/// Secondary transmit.
pub const TIOCM_ST: i32 = 0o010;
/// Secondary receive.
pub const TIOCM_SR: i32 = 0o020;
/// Clear to send.
pub const TIOCM_CTS: i32 = 0o040;
/// Carrier detect.
pub const TIOCM_CAR: i32 = 0o100;
/// Carrier detect.
pub const TIOCM_CD: i32 = TIOCM_CAR;
/// Ring.
pub const TIOCM_RNG: i32 = 0o200;
/// Ring.
pub const TIOCM_RI: i32 = TIOCM_RNG;
/// Data set ready.
pub const TIOCM_DSR: i32 = 0o400;

// 8-10 compat

/// Set exclusive use of tty.
pub const TIOCEXCL: u64 = _io(b't', 13);
/// Reset exclusive use of tty.
pub const TIOCNXCL: u64 = _io(b't', 14);
// 15 unused
/// Flush buffers.
pub const TIOCFLUSH: u64 = _iow::<i32>(b't', 16);
// 17-18 compat
/// Get termios struct.
pub const TIOCGETA: u64 = _ior::<Termios>(b't', 19);
/// Set termios struct.
pub const TIOCSETA: u64 = _iow::<Termios>(b't', 20);
/// Drain output, set.
pub const TIOCSETAW: u64 = _iow::<Termios>(b't', 21);
/// Drain output, flush input, set.
pub const TIOCSETAF: u64 = _iow::<Termios>(b't', 22);
/// Get line discipline.
pub const TIOCGETD: u64 = _ior::<i32>(b't', 26);
/// Set line discipline.
pub const TIOCSETD: u64 = _iow::<i32>(b't', 27);
/// Set verified auth.
pub const TIOCSETVERAUTH: u64 = _iow::<i32>(b't', 28);
/// Clear verified auth.
pub const TIOCCLRVERAUTH: u64 = _io(b't', 29);
/// Check verified auth.
pub const TIOCCHKVERAUTH: u64 = _io(b't', 30);
// 127-124 compat
/// Set break bit.
pub const TIOCSBRK: u64 = _io(b't', 123);
/// Clear break bit.
pub const TIOCCBRK: u64 = _io(b't', 122);
/// Set data terminal ready.
pub const TIOCSDTR: u64 = _io(b't', 121);
/// Clear data terminal ready.
pub const TIOCCDTR: u64 = _io(b't', 120);
/// Get pgrp of tty.
pub const TIOCGPGRP: u64 = _ior::<i32>(b't', 119);
/// Set pgrp of tty.
pub const TIOCSPGRP: u64 = _iow::<i32>(b't', 118);
// 117-116 compat
/// Output queue size.
pub const TIOCOUTQ: u64 = _ior::<i32>(b't', 115);
/// Void tty association.
pub const TIOCNOTTY: u64 = _io(b't', 113);
/// pty: set/clear packet mode.
pub const TIOCPKT: u64 = _iow::<i32>(b't', 112);
/// Data packet.
pub const TIOCPKT_DATA: i32 = 0x00;
/// Flush packet.
pub const TIOCPKT_FLUSHREAD: i32 = 0x01;
/// Flush packet.
pub const TIOCPKT_FLUSHWRITE: i32 = 0x02;
/// Stop output.
pub const TIOCPKT_STOP: i32 = 0x04;
/// Start output.
pub const TIOCPKT_START: i32 = 0x08;
/// No more ^S, ^Q.
pub const TIOCPKT_NOSTOP: i32 = 0x10;
/// Now do ^S ^Q.
pub const TIOCPKT_DOSTOP: i32 = 0x20;
/// State change of pty driver.
pub const TIOCPKT_IOCTL: i32 = 0x40;
/// Stop output, like ^S.
pub const TIOCSTOP: u64 = _io(b't', 111);
/// Start output, like ^Q.
pub const TIOCSTART: u64 = _io(b't', 110);
/// Set all modem bits.
pub const TIOCMSET: u64 = _iow::<i32>(b't', 109);
/// Bis modem bits.
pub const TIOCMBIS: u64 = _iow::<i32>(b't', 108);
/// Bic modem bits.
pub const TIOCMBIC: u64 = _iow::<i32>(b't', 107);
/// Get all modem bits.
pub const TIOCMGET: u64 = _ior::<i32>(b't', 106);
/// Remote input editing.
pub const TIOCREMOTE: u64 = _iow::<i32>(b't', 105);
/// Get window size.
pub const TIOCGWINSZ: u64 = _ior::<Winsize>(b't', 104);
/// Set window size.
pub const TIOCSWINSZ: u64 = _iow::<Winsize>(b't', 103);
/// pty: set/clr usr cntl mode.
pub const TIOCUCNTL: u64 = _iow::<i32>(b't', 102);

/// `UIOCCMD(n)`: user control op "n".
pub const fn uioccmd(n: u8) -> u64 {
    _io(b'u', n)
}

/// Set break bit, user control.
pub const TIOCUCNTL_SBRK: u64 = TIOCSBRK & 0xff;
/// Clear break bit, user control.
pub const TIOCUCNTL_CBRK: u64 = TIOCCBRK & 0xff;
/// Generate status message.
pub const TIOCSTAT: u64 = _io(b't', 101);
/// Get sid of tty.
pub const TIOCGSID: u64 = _ior::<i32>(b't', 99);
/// Become virtual console.
pub const TIOCCONS: u64 = _iow::<i32>(b't', 98);
/// Become controlling tty.
pub const TIOCSCTTY: u64 = _io(b't', 97);
/// pty: external processing.
pub const TIOCEXT: u64 = _iow::<i32>(b't', 96);
/// pty: generate signal.
pub const TIOCSIG: u64 = _iow::<i32>(b't', 95);
/// Wait till output drained.
pub const TIOCDRAIN: u64 = _io(b't', 94);
/// Get device flags.
pub const TIOCGFLAGS: u64 = _ior::<i32>(b't', 93);
/// Set device flags.
pub const TIOCSFLAGS: u64 = _iow::<i32>(b't', 92);
/// Ignore hardware carrier.
pub const TIOCFLAG_SOFTCAR: i32 = 0x01;
/// Set clocal on open.
pub const TIOCFLAG_CLOCAL: i32 = 0x02;
/// Set crtscts on open.
pub const TIOCFLAG_CRTSCTS: i32 = 0x04;
/// Set mdmbuf on open.
pub const TIOCFLAG_MDMBUF: i32 = 0x08;
/// Call hardpps on carrier up.
pub const TIOCFLAG_PPS: i32 = 0x10;
/// Get timestamp.
pub const TIOCGTSTAMP: u64 = _ior::<Timeval>(b't', 91);
/// Timestamp reasons.
pub const TIOCSTSTAMP: u64 = _iow::<Tstamps>(b't', 90);

// Backwards compatibility

/// `TIOCMODG`: `TIOCMGET`.
pub const TIOCMODG: u64 = TIOCMGET;
/// `TIOCMODS`: `TIOCMSET`.
pub const TIOCMODS: u64 = TIOCMSET;

/// Termios tty line discipline.
pub const TTYDISC: i32 = 0;
/// Tablet discipline.
pub const TABLDISC: i32 = 3;
/// Serial IP discipline.
pub const SLIPDISC: i32 = 4;
/// ppp discipline.
pub const PPPDISC: i32 = 5;
/// Metricom wireless IP discipline.
pub const STRIPDISC: i32 = 6;
/// NMEA0183 discipline.
pub const NMEADISC: i32 = 7;
/// Meinberg time string discipline.
pub const MSTSDISC: i32 = 8;
/// EndRun time format discipline.
pub const ENDRUNDISC: i32 = 9;

// Defaults on "first" open.

/// Default input flags.
pub const TTYDEF_IFLAG: Tcflag = BRKINT | ICRNL | IMAXBEL | IXON | IXANY;
/// Default output flags.
pub const TTYDEF_OFLAG: Tcflag = OPOST | ONLCR;
/// Default local flags.
pub const TTYDEF_LFLAG: Tcflag = ECHO | ICANON | ISIG | IEXTEN | ECHOE | ECHOKE | ECHOCTL;
/// Default control flags.
pub const TTYDEF_CFLAG: Tcflag = CREAD | CS8 | HUPCL;
/// Default speed.
pub const TTYDEF_SPEED: Speed = B9600;

// Control Character Defaults

/// `CTRL(x)`: the control character of `x`.
pub const fn ctrl(x: u8) -> Cc {
    x & 0o37
}

/// End of file.
pub const CEOF: Cc = ctrl(b'd');
/// End of line (avoids `_POSIX_VDISABLE`).
pub const CEOL: Cc = 0o377;
/// Erase.
pub const CERASE: Cc = 0o177;
/// Interrupt.
pub const CINTR: Cc = ctrl(b'c');
/// Status (avoids `_POSIX_VDISABLE`).
pub const CSTATUS: Cc = 0o377;
/// Kill.
pub const CKILL: Cc = ctrl(b'u');
/// Minimum characters for a non-canonical read.
pub const CMIN: Cc = 1;
/// Quit (FS, `^\`).
pub const CQUIT: Cc = 0o34;
/// Suspend.
pub const CSUSP: Cc = ctrl(b'z');
/// Timeout for a non-canonical read.
pub const CTIME: Cc = 0;
/// Delayed suspend.
pub const CDSUSP: Cc = ctrl(b'y');
/// Start output.
pub const CSTART: Cc = ctrl(b'q');
/// Stop output.
pub const CSTOP: Cc = ctrl(b's');
/// Literal next.
pub const CLNEXT: Cc = ctrl(b'v');
/// Discard output.
pub const CDISCARD: Cc = ctrl(b'o');
/// Word erase.
pub const CWERASE: Cc = ctrl(b'w');
/// Reprint line.
pub const CREPRINT: Cc = ctrl(b'r');
/// End of transmission.
pub const CEOT: Cc = CEOF;
// compat
/// Break (compat).
pub const CBRK: Cc = CEOL;
/// Reprint (compat).
pub const CRPRNT: Cc = CREPRINT;
/// Flush (compat).
pub const CFLUSH: Cc = CDISCARD;

/// `ttydefchars[NCCS]`: the default control characters, indexed by `V*`.
pub const TTYDEFCHARS: [Cc; NCCS] = [
    CEOF,
    CEOL,
    CEOL,
    CERASE,
    CWERASE,
    CKILL,
    CREPRINT,
    _POSIX_VDISABLE,
    CINTR,
    CQUIT,
    CSUSP,
    CDSUSP,
    CSTART,
    CSTOP,
    CLNEXT,
    CDISCARD,
    CMIN,
    CTIME,
    CSTATUS,
    _POSIX_VDISABLE,
];
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_and_helpers() {
        assert_eq!(core::mem::size_of::<Termios>(), 4 * 4 + NCCS + 2 * 4);
        assert_eq!(core::mem::size_of::<Termios>(), 44);
        assert_eq!(core::mem::size_of::<Winsize>(), 8);
        assert_eq!(core::mem::size_of::<Tstamps>(), 8);
        assert_eq!(core::mem::size_of::<Timeval>(), 16);
        assert!(cceq(3, 3));
        assert!(!cceq(3, 4));
        assert!(!cceq(_POSIX_VDISABLE, _POSIX_VDISABLE));
        assert_eq!(CS8 & CSIZE, CS8);
        assert_eq!(Termios::zeroed(), Termios::default());
    }

    #[test]
    fn a_termios_is_the_bytes_userland_passes() {
        let mut t = Termios::zeroed();
        t.c_iflag = TTYDEF_IFLAG;
        t.c_oflag = TTYDEF_OFLAG;
        t.c_cflag = TTYDEF_CFLAG;
        t.c_lflag = TTYDEF_LFLAG;
        t.c_cc = TTYDEFCHARS;
        t.c_ispeed = TTYDEF_SPEED as i32;
        t.c_ospeed = TTYDEF_SPEED as i32;
        let mut b = [0u8; 44];
        assert!(hal::user::pod_to_bytes(&t, &mut b));
        assert_eq!(&b[0..4], &TTYDEF_IFLAG.to_ne_bytes());
        assert_eq!(&b[12..16], &TTYDEF_LFLAG.to_ne_bytes());
        assert_eq!(&b[16..36], &TTYDEFCHARS);
        assert_eq!(&b[40..44], &9600i32.to_ne_bytes());
        assert_eq!(hal::user::pod_from_bytes::<Termios>(&b), Some(t));
        let ws = Winsize {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 1,
            ws_ypixel: 2,
        };
        let mut wb = [0u8; 8];
        assert!(hal::user::pod_to_bytes(&ws, &mut wb));
        assert_eq!(&wb[0..2], &24u16.to_ne_bytes());
        assert_eq!(&wb[2..4], &80u16.to_ne_bytes());
        assert_eq!(hal::user::pod_from_bytes::<Winsize>(&wb), Some(ws));
    }

    #[test]
    fn commands_encode_as_on_openbsd() {
        // The values OpenBSD's userland compiles in (LP64): direction, size, group, number.
        assert_eq!(TIOCGETA, 0x402c_7413);
        assert_eq!(TIOCSETA, 0x802c_7414);
        assert_eq!(TIOCSETAW, 0x802c_7415);
        assert_eq!(TIOCSETAF, 0x802c_7416);
        assert_eq!(TIOCGWINSZ, 0x4008_7468);
        assert_eq!(TIOCSWINSZ, 0x8008_7467);
        assert_eq!(TIOCSCTTY, 0x2000_7461);
        assert_eq!(TIOCGPGRP, 0x4004_7477);
        assert_eq!(TIOCSPGRP, 0x8004_7476);
        assert_eq!(TIOCGTSTAMP, 0x4010_745b);
        assert_eq!(TIOCSTSTAMP, 0x8008_745a);
        assert_eq!(TIOCUCNTL_SBRK, 123);
        assert_eq!(uioccmd(5), 0x2000_7505);
        // The kernel's other copy of the two pgrp commands (kernel::sigio's) agrees.
        assert_eq!(TIOCGPGRP, crate::sigio::TIOCGPGRP);
        assert_eq!(TIOCSPGRP, crate::sigio::TIOCSPGRP);
    }

    #[test]
    fn control_characters() {
        assert_eq!(ctrl(b'd'), 4);
        assert_eq!(CEOF, 4);
        assert_eq!(CINTR, 3);
        assert_eq!(CQUIT, 0o34);
        assert_eq!(TTYDEFCHARS[VEOF as usize], CEOF);
        assert_eq!(TTYDEFCHARS[VEOL as usize], CEOL);
        assert_eq!(TTYDEFCHARS[VEOL2 as usize], CEOL);
        assert_eq!(TTYDEFCHARS[VERASE as usize], CERASE);
        assert_eq!(TTYDEFCHARS[7], 0o377);
        assert_eq!(TTYDEFCHARS[19], 0o377);
        assert_eq!(TTYDEF_CFLAG, 0x0800 | 0x0300 | 0x4000);
        assert_eq!(TTYDEF_SPEED, 9600);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_headers() {
        let defs = crate::reftest::defines("sys/sys/termios.h");
        let ours: &[(&str, i64)] = &[
            ("VEOF", VEOF as i64),
            ("VEOL", VEOL as i64),
            ("VEOL2", VEOL2 as i64),
            ("VERASE", VERASE as i64),
            ("VWERASE", VWERASE as i64),
            ("VKILL", VKILL as i64),
            ("VREPRINT", VREPRINT as i64),
            ("VINTR", VINTR as i64),
            ("VQUIT", VQUIT as i64),
            ("VSUSP", VSUSP as i64),
            ("VDSUSP", VDSUSP as i64),
            ("VSTART", VSTART as i64),
            ("VSTOP", VSTOP as i64),
            ("VLNEXT", VLNEXT as i64),
            ("VDISCARD", VDISCARD as i64),
            ("VMIN", VMIN as i64),
            ("VTIME", VTIME as i64),
            ("VSTATUS", VSTATUS as i64),
            ("NCCS", NCCS as i64),
            ("IGNBRK", IGNBRK as i64),
            ("BRKINT", BRKINT as i64),
            ("IGNPAR", IGNPAR as i64),
            ("PARMRK", PARMRK as i64),
            ("INPCK", INPCK as i64),
            ("ISTRIP", ISTRIP as i64),
            ("INLCR", INLCR as i64),
            ("IGNCR", IGNCR as i64),
            ("ICRNL", ICRNL as i64),
            ("IXON", IXON as i64),
            ("IXOFF", IXOFF as i64),
            ("IXANY", IXANY as i64),
            ("IUCLC", IUCLC as i64),
            ("IMAXBEL", IMAXBEL as i64),
            ("OPOST", OPOST as i64),
            ("ONLCR", ONLCR as i64),
            ("TABDLY", TABDLY as i64),
            ("TAB0", TAB0 as i64),
            ("TAB3", TAB3 as i64),
            ("ONOEOT", ONOEOT as i64),
            ("OCRNL", OCRNL as i64),
            ("OLCUC", OLCUC as i64),
            ("ONOCR", ONOCR as i64),
            ("ONLRET", ONLRET as i64),
            ("CIGNORE", CIGNORE as i64),
            ("CSIZE", CSIZE as i64),
            ("CS5", CS5 as i64),
            ("CS6", CS6 as i64),
            ("CS7", CS7 as i64),
            ("CS8", CS8 as i64),
            ("CSTOPB", CSTOPB as i64),
            ("CREAD", CREAD as i64),
            ("PARENB", PARENB as i64),
            ("PARODD", PARODD as i64),
            ("HUPCL", HUPCL as i64),
            ("CLOCAL", CLOCAL as i64),
            ("CRTSCTS", CRTSCTS as i64),
            ("MDMBUF", MDMBUF as i64),
            ("ECHOKE", ECHOKE as i64),
            ("ECHOE", ECHOE as i64),
            ("ECHOK", ECHOK as i64),
            ("ECHO", ECHO as i64),
            ("ECHONL", ECHONL as i64),
            ("ECHOPRT", ECHOPRT as i64),
            ("ECHOCTL", ECHOCTL as i64),
            ("ISIG", ISIG as i64),
            ("ICANON", ICANON as i64),
            ("ALTWERASE", ALTWERASE as i64),
            ("IEXTEN", IEXTEN as i64),
            ("EXTPROC", EXTPROC as i64),
            ("TOSTOP", TOSTOP as i64),
            ("FLUSHO", FLUSHO as i64),
            ("XCASE", XCASE as i64),
            ("NOKERNINFO", NOKERNINFO as i64),
            ("PENDIN", PENDIN as i64),
            ("NOFLSH", NOFLSH as i64),
            ("TCSANOW", TCSANOW as i64),
            ("TCSADRAIN", TCSADRAIN as i64),
            ("TCSAFLUSH", TCSAFLUSH as i64),
            ("TCSASOFT", TCSASOFT as i64),
            ("B0", B0 as i64),
            ("B300", B300 as i64),
            ("B9600", B9600 as i64),
            ("B38400", B38400 as i64),
            ("B115200", B115200 as i64),
            ("B230400", B230400 as i64),
            ("EXTA", EXTA as i64),
            ("EXTB", EXTB as i64),
            ("TCIFLUSH", TCIFLUSH as i64),
            ("TCOFLUSH", TCOFLUSH as i64),
            ("TCIOFLUSH", TCIOFLUSH as i64),
            ("TCOOFF", TCOOFF as i64),
            ("TCOON", TCOON as i64),
            ("TCIOFF", TCIOFF as i64),
            ("TCION", TCION as i64),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
        let defs = crate::reftest::defines("sys/sys/ttycom.h");
        let ours: &[(&str, i32)] = &[
            ("TIOCM_LE", TIOCM_LE),
            ("TIOCM_DTR", TIOCM_DTR),
            ("TIOCM_RTS", TIOCM_RTS),
            ("TIOCM_ST", TIOCM_ST),
            ("TIOCM_SR", TIOCM_SR),
            ("TIOCM_CTS", TIOCM_CTS),
            ("TIOCM_CAR", TIOCM_CAR),
            ("TIOCM_RNG", TIOCM_RNG),
            ("TIOCM_DSR", TIOCM_DSR),
            ("TIOCPKT_DATA", TIOCPKT_DATA),
            ("TIOCPKT_FLUSHREAD", TIOCPKT_FLUSHREAD),
            ("TIOCPKT_FLUSHWRITE", TIOCPKT_FLUSHWRITE),
            ("TIOCPKT_STOP", TIOCPKT_STOP),
            ("TIOCPKT_START", TIOCPKT_START),
            ("TIOCPKT_NOSTOP", TIOCPKT_NOSTOP),
            ("TIOCPKT_DOSTOP", TIOCPKT_DOSTOP),
            ("TIOCPKT_IOCTL", TIOCPKT_IOCTL),
            ("TIOCFLAG_SOFTCAR", TIOCFLAG_SOFTCAR),
            ("TIOCFLAG_CLOCAL", TIOCFLAG_CLOCAL),
            ("TIOCFLAG_CRTSCTS", TIOCFLAG_CRTSCTS),
            ("TIOCFLAG_MDMBUF", TIOCFLAG_MDMBUF),
            ("TIOCFLAG_PPS", TIOCFLAG_PPS),
            ("TTYDISC", TTYDISC),
            ("TABLDISC", TABLDISC),
            ("SLIPDISC", SLIPDISC),
            ("PPPDISC", PPPDISC),
            ("STRIPDISC", STRIPDISC),
            ("NMEADISC", NMEADISC),
            ("MSTSDISC", MSTSDISC),
            ("ENDRUNDISC", ENDRUNDISC),
        ];
        for (name, value) in ours {
            assert_eq!(
                crate::reftest::int(&defs, name),
                Some(i64::from(*value)),
                "{name}"
            );
        }
    }
}
/* </TESTS> */
