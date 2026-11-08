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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)uipc_socket2.c	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! Primitive routines for operating on sockets and socket buffers: `kern/uipc_socket2.c`, the
//! kernel's (`kernel::net::socket`, `kernel::net::sockbuf`) under LZ's names for `bsd`'s
//! protocols.
//!
//! Upstream: sys/kern/uipc_socket2.c @ 3ce1f3f79392
//!
//! The socket locks are `bsd`'s C-style holdings: the net lock (`net_lock`) for the internet
//! domains, a detached holding of `so_lock` for the others; the kernel takes `so_lock` of a
//! socket of `bsd`'s domains the same way, so a protocol releasing and taking it again
//! inside a request (`route_output`) meets the kernel's own holding. The buffer routines take
//! the guard of the buffer's mutex (`sb_mtx`), which the caller holds where the C's caller
//! holds the mutex; the mbufs `bsd` keeps by address are unparked into the kernel's buffer
//! and parked back when it refuses them.
//!
//! ## Deviations
//! - The buffer routines take `SbGuard` (the mutex held) where the C takes the
//!   `struct sockbuf *` with its mutex held; `sbmtxassertlocked` is that guard.

use core::ffi::c_void;
use core::sync::atomic::Ordering;

use kernel::net::mbuf::MbufBox;
use kernel::net::socket::Onq;

pub use kernel::net::sockbuf::{SB_MAX_VAR, sbchecklowmem, sbcheckreserve};

use crate::kassert;
use crate::kern::subr_prf::{SPLASSERT_CTL, splassert_fail};
use crate::sys::errno::Errno;
use crate::sys::mbuf::{Mbuf, park, unpark};
use crate::sys::rwlock::{RW_READ, RW_WRITE};
use crate::sys::socket::{PF_INET, PF_INET6};
use crate::sys::socketvar::{Sockbuf, Socket, as_static};
use crate::sys::systm::{
    net_assert_locked, net_assert_locked_exclusive, net_lock, net_lock_shared, net_unlock,
    net_unlock_shared,
};

/// The guard of a socket buffer's mutex (`mtx_enter(&sb->sb_mtx)` held).
pub type SbGuard<'a> = kernel::net::sockbuf::SbGuard<'a, crate::kern::proc_glue::BsdProcGlue>;

/// Whether `so` belongs to an internet domain, which the net lock serialises.
fn is_inet(so: &Socket) -> bool {
    let family = so.dom_family();
    family == i32::from(PF_INET) || family == i32::from(PF_INET6)
}

/// `soisconnecting(so)`.
pub fn soisconnecting(so: &Socket) {
    kernel::net::socket::soisconnecting(so);
}

/// `soisconnected(so)`.
pub fn soisconnected(so: &Socket) {
    kernel::net::socket::soisconnected(so);
}

/// `soisdisconnecting(so)`.
pub fn soisdisconnecting(so: &Socket) {
    kernel::net::socket::soisdisconnecting(so);
}

/// `soisdisconnected(so)`.
pub fn soisdisconnected(so: &Socket) {
    kernel::net::socket::soisdisconnected(so);
}

/// `sonewconn(head, connstatus, wait)`: a new socket on `head`'s queue, returned locked
/// (`sounlock_nonet` ends its holding); its references are the queue's and its existence's.
pub fn sonewconn(head: &Socket, connstatus: u32, wait: i32) -> Option<&'static Socket> {
    let so = kernel::net::socket::sonewconn(head, connstatus, wait)?;
    Some(as_static(&so))
}

/// `soqremque(so, q)`: `so` off its listener's partial (0) or complete (1) queue; whether it
/// was on it. The queue's reference goes (its existence reference keeps it).
pub fn soqremque(so: &Socket, q: i32) -> bool {
    let onq = if q == 0 { Onq::Q0 } else { Onq::Q };
    kernel::net::socket::soqremque(so, onq).is_some()
}

/// `socantsendmore(so)`.
pub fn socantsendmore(so: &Socket) {
    kernel::net::socket::socantsendmore(so);
}

/// `socantrcvmore(so)`.
pub fn socantrcvmore(so: &Socket) {
    kernel::net::socket::socantrcvmore(so);
}

/// `solock(so)`: the net lock for the internet domains, the socket's lock for the others.
pub fn solock(so: &Socket) {
    if is_inet(so) {
        net_lock();
    } else {
        so.so_lock_legacy().enter_write_detached();
    }
}

/// `solock_shared(so)`: the shared net lock (internet domains) and the socket's lock.
pub fn solock_shared(so: &Socket) {
    if is_inet(so) {
        net_lock_shared();
    }
    so.so_lock_legacy().enter_write_detached();
}

/// `solock_nonet(so)`: the socket's lock; an internet socket's caller holds the net lock.
pub fn solock_nonet(so: &Socket) {
    if is_inet(so) {
        net_assert_locked("solock_nonet");
    }
    so.so_lock_legacy().enter_write_detached();
}

/// `solock_persocket(so)`: whether `so` has a lock of its own (it is not an internet one).
pub fn solock_persocket(so: &Socket) -> bool {
    !is_inet(so)
}

/// `solock_pair(so1, so2)`: locks two sockets of one type, in address order.
pub fn solock_pair(so1: &Socket, so2: &Socket) {
    kassert!(so1.so_type() == so2.so_type());

    if is_inet(so1) {
        net_lock_shared();
    }
    if core::ptr::eq(so1, so2) {
        so1.so_lock_legacy().enter_write_detached();
    } else if core::ptr::from_ref(so1) < core::ptr::from_ref(so2) {
        so1.so_lock_legacy().enter_write_detached();
        so2.so_lock_legacy().enter_write_detached();
    } else {
        so2.so_lock_legacy().enter_write_detached();
        so1.so_lock_legacy().enter_write_detached();
    }
}

/// `sounlock(so)`: undoes `solock`.
pub fn sounlock(so: &Socket) {
    if is_inet(so) {
        net_unlock();
    } else {
        so.so_lock_legacy().exit_write_detached();
    }
}

/// `sounlock_shared(so)`: undoes `solock_shared`.
pub fn sounlock_shared(so: &Socket) {
    if is_inet(so) {
        net_unlock_shared();
    }
    so.so_lock_legacy().exit_write_detached();
}

/// `sounlock_nonet(so)`: undoes `solock_nonet` (and ends the holding of a socket
/// `sonewconn` returned locked).
pub fn sounlock_nonet(so: &Socket) {
    so.so_lock_legacy().exit_write_detached();
}

/// `sounlock_pair(so1, so2)`: undoes `solock_pair`.
pub fn sounlock_pair(so1: &Socket, so2: &Socket) {
    if is_inet(so1) {
        net_unlock_shared();
    }
    if core::ptr::eq(so1, so2) {
        so1.so_lock_legacy().exit_write_detached();
    } else if core::ptr::from_ref(so1) < core::ptr::from_ref(so2) {
        so2.so_lock_legacy().exit_write_detached();
        so1.so_lock_legacy().exit_write_detached();
    } else {
        so1.so_lock_legacy().exit_write_detached();
        so2.so_lock_legacy().exit_write_detached();
    }
}

/// `soassertlocked_readonly(so)`: enough of the socket's locks are held to read its state.
pub fn soassertlocked_readonly(so: &Socket) {
    if is_inet(so) {
        net_assert_locked("soassertlocked_readonly");
    } else {
        kassert!(so.so_lock_legacy().lock().status() == RW_WRITE);
    }
}

/// `soassertlocked(so)`: the socket's locks are held to change its state.
pub fn soassertlocked(so: &Socket) {
    if is_inet(so) {
        if hal::netlock::status() == RW_READ {
            net_assert_locked("soassertlocked");

            if SPLASSERT_CTL.load(Ordering::Relaxed) > 0
                && so.so_lock_legacy().lock().status() != RW_WRITE
            {
                splassert_fail(0, RW_WRITE, "soassertlocked");
            }
        } else {
            net_assert_locked_exclusive("soassertlocked");
        }
    } else {
        kassert!(so.so_lock_legacy().lock().status() == RW_WRITE);
    }
}

/// `sosleep_nsec(so, ident, prio, wmesg, nsecs)`: sleeps on `ident`, releasing the lock
/// that serialises `so` (held C-style) while asleep.
pub fn sosleep_nsec(
    so: &Socket,
    ident: *const c_void,
    prio: i32,
    wmesg: &'static str,
    nsecs: u64,
) -> Result<(), Errno> {
    if is_inet(so) {
        let shared = hal::netlock::status() == RW_READ;
        if shared {
            so.so_lock_legacy().exit_write_detached();
        }
        let ret = hal::netlock::sleep_nsec(ident, prio, wmesg, nsecs);
        if shared {
            so.so_lock_legacy().enter_write_detached();
        }
        ret
    } else {
        so.so_lock_legacy()
            .sleep_detached(ident, prio, wmesg, nsecs)
    }
}

/// `sbwait(sb)`: wait for data to arrive at/drain from a socket buffer (its mutex held).
pub fn sbwait(g: &mut SbGuard<'_>) -> Result<(), Errno> {
    g.sbwait()
}

/// `sowakeup(so, sb)`.
pub fn sowakeup(so: &Socket, sb: &Sockbuf) {
    kernel::net::socket::sowakeup(so, sb);
}

/// `soreserve(so, sndcc, rcvcc)`.
pub fn soreserve(so: &Socket, sndcc: u64, rcvcc: u64) -> Result<(), Errno> {
    kernel::net::socket::soreserve(so, sndcc, rcvcc)
}

/// `sbreserve(sb, cc)` (its mutex held).
pub fn sbreserve(g: &mut SbGuard<'_>, cc: u64) -> Result<(), Errno> {
    g.sbreserve(cc)
}

/// `sbrelease(sb)` (its mutex held).
pub fn sbrelease(g: &mut SbGuard<'_>) {
    g.sbrelease();
}

/// `sbappend(sb, m)` (its mutex held).
pub fn sbappend(g: &mut SbGuard<'_>, m: Option<&'static Mbuf>) {
    g.sbappend(m.map(unpark));
}

/// `sbappendstream(sb, m)` (its mutex held).
pub fn sbappendstream(g: &mut SbGuard<'_>, m: &'static Mbuf) {
    g.sbappendstream(unpark(m));
}

/// `sbappendrecord(sb, m0)` (its mutex held).
pub fn sbappendrecord(g: &mut SbGuard<'_>, m0: Option<&'static Mbuf>) {
    g.sbappendrecord(m0.map(unpark));
}

/// `sbappendaddr(sb, asa, m0, control)` (its mutex held): 1, or 0 with the data and the
/// control left to the caller (parked again).
pub fn sbappendaddr(
    g: &mut SbGuard<'_>,
    asa: &[u8],
    m0: Option<&'static Mbuf>,
    control: Option<&'static Mbuf>,
) -> i32 {
    match g.sbappendaddr(asa, m0.map(unpark), control.map(unpark)) {
        Ok(()) => 1,
        Err((m0, control)) => {
            repark(m0);
            repark(control);
            0
        }
    }
}

/// `sbappendcontrol(sb, m0, control)` (its mutex held): whether it was appended; the data
/// and the control left to the caller (parked again) when not.
pub fn sbappendcontrol(
    g: &mut SbGuard<'_>,
    m0: Option<&'static Mbuf>,
    control: &'static Mbuf,
) -> bool {
    match g.sbappendcontrol(m0.map(unpark), unpark(control)) {
        Ok(()) => true,
        Err((m0, control)) => {
            repark(m0);
            repark(Some(control));
            false
        }
    }
}

/// An mbuf the kernel gave back, parked again for the `bsd` caller that still names it.
fn repark(m: Option<MbufBox>) {
    if let Some(m) = m {
        let _ = park(m);
    }
}

/// `sbflush(sb)` (its mutex held).
pub fn sbflush(g: &mut SbGuard<'_>) {
    g.sbflush();
}

/// `sbdrop(sb, len)` (its mutex held).
pub fn sbdrop(g: &mut SbGuard<'_>, len: i32) {
    g.sbdrop(len);
}

/// `sbdroprecord(sb)` (its mutex held).
pub fn sbdroprecord(g: &mut SbGuard<'_>) {
    g.sbdroprecord();
}

/// `sbcreatecontrol(p, size, type, level)`: a control message of `p`'s bytes, parked for
/// the caller.
pub fn sbcreatecontrol(p: &[u8], type_: i32, level: i32) -> Option<&'static Mbuf> {
    kernel::net::sockbuf::sbcreatecontrol(p, type_, level).map(park)
}
/* </CODE> */
