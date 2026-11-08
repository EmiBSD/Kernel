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
 * Copyright (c) 1982, 1986, 1990, 1993
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
 *	@(#)socketvar.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! Kernel structure per socket and per socket buffer: `<sys/socketvar.h>`, the kernel's
//! (`kernel::net::socket`, `kernel::net::sockbuf`) as `bsd` names it.
//!
//! Upstream: sys/sys/socketvar.h @ 3ce1f3f79392
//!
//! `bsd`'s protocols (inet's, route's, pfkey's, nfs's sockets) reach the kernel's socket
//! through this adapter: the types and constants re-exported, the header's inline functions
//! over the kernel's, and the `&'static Socket` LZ's code keeps ([`as_static`]).
//!
//! ## Deviations
//! - A [`Socket`] `bsd` holds by `&'static` is counted by its holder, as the C's convention
//!   says (a file, a control block, a queue, a parked reference of [`soref`]): see
//!   [`as_static`].

use hal::pool::PoolArc;

pub use kernel::net::sockbuf::{
    SB_ASYNC, SB_MAX, SB_MAX_VAR, SB_NOINTR, SB_SPLICE, SB_WAIT, SBL_NOINTR, SBL_WAIT,
    SS_CANTRCVMORE, SS_CANTSENDMORE, SS_CONNECTOUT, SS_DNS, SS_ISCONNECTED, SS_ISCONNECTING,
    SS_ISDISCONNECTED, SS_ISDISCONNECTING, SS_ISSENDING, SS_NOFDREF, SS_PRIV, SS_RCVATMARK, SS_YP,
};
pub use kernel::net::socket::{
    isspliced, issplicedback, soissending, soreadable, sosendallatonce, sowriteable,
};

/// `struct socket` of `bsd`'s processes.
pub type Socket = kernel::net::socket::Socket<crate::kern::proc_glue::BsdProcGlue>;
/// `struct sockbuf` of `bsd`'s processes.
pub type Sockbuf = kernel::net::sockbuf::Sockbuf<crate::kern::proc_glue::BsdProcGlue>;
/// A counted socket of `bsd`'s processes.
pub type SocketRef = kernel::net::socket::SocketRef<crate::kern::proc_glue::BsdProcGlue>;
/// `struct sosplice` of `bsd`'s processes.
pub type Sosplice = kernel::net::socket::Sosplice<crate::kern::proc_glue::BsdProcGlue>;

/// `so` as the `&'static Socket` LZ's code keeps (`Inpcb::inp_socket`, the protocols'
/// arguments): the reference of the holder that lent `so` keeps the socket for as long as
/// `bsd` uses it.
pub fn as_static(so: &Socket) -> &'static Socket {
    // SAFETY: `so` is a socket of `sockpl` (a `PoolArc` item, which stays allocated, its
    // value in place, until its last handle's drop). Every `&Socket` `bsd` holds was lent by
    // a holder of one of its counted references, under the C's convention every `bsd`
    // caller already follows: a socket reached through a file (the file's `SocketRef`), a
    // protocol control block (`inp_socket`, set at attach and cleared with `so_pcb` before
    // `sofree`), a listener's queue (the queue's parked reference), a kernel request's own
    // argument (the caller's reference), or a reference `soref` parked (taken back by
    // `sorele`); no `bsd` path frees a socket it did not count, and every `bsd` use of the
    // `&'static` ends before its holder lets the reference go (the detach that clears
    // `inp_socket` runs before the socket's last reference can go, `sofree` adopting the
    // existence reference only once the pcb is gone). The kernel never lends a
    // `&'static Socket` and never takes one. So the socket outlives every use of this
    // reference; `Socket` is `Sync` (its members are atomics, mutexes and locked cells).
    unsafe { &*core::ptr::from_ref(so) }
}

/// `soref(so)`: one more reference to `so`, parked on the socket for [`sorele`] (the
/// C's `refcnt_take(&so->so_refcnt)` of a `bsd` holder that keeps `so` by address).
pub fn soref(so: Option<&'static Socket>) -> Option<&'static Socket> {
    let so = so?;
    PoolArc::forget(kernel::net::socket::soref(so));
    Some(so)
}

/// `sorele(so)`: the reference [`soref`] parked given back; the last one frees the socket.
pub fn sorele(so: &Socket) {
    match PoolArc::adopt(so) {
        Some(r) => drop(r),
        None => crate::kern::subr_prf::panic(format_args!("sorele: socket {so:p} not soref'd")),
    }
}

/// `so->so_pcb` of a socket of one of `bsd`'s domains, the C's pointer (the word
/// [`set_so_pcb`] stored; NULL for none).
pub fn so_pcb(so: &Socket) -> *mut core::ffi::c_void {
    core::ptr::with_exposed_provenance_mut(so.legacy_pcb())
}

/// `so->so_pcb = pcb` of a socket of one of `bsd`'s domains (its address exposed, so
/// [`so_pcb`] gives the pointer back).
pub fn set_so_pcb(so: &Socket, pcb: *mut core::ffi::c_void) {
    so.set_legacy_pcb(pcb.expose_provenance());
}

/// `sb_notify(sb)`: do we need to notify the other side when I/O is possible?
pub fn sb_notify(sb: &Sockbuf) -> bool {
    sb.sb_notify()
}

/// `sbspace_locked(sb)`: how much space is there in a socket buffer (its mutex held, or
/// the counts' snapshot)?
pub fn sbspace_locked(sb: &Sockbuf) -> i64 {
    sb.space_unlocked()
}

/// `sbspace(sb)`: [`sbspace_locked`] under the buffer's mutex.
pub fn sbspace(sb: &Sockbuf) -> i64 {
    sb.sbspace()
}
/* </CODE> */
