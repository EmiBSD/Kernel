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
 *	@(#)sys_socket.c	8.1 (Berkeley) 6/10/93
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
 *	@(#)uipc_socket.c	8.3 (Berkeley) 4/15/94
 */
/* </LICENSES> */

/* <CODE> */
//! The socket file: `socketops` (`kern/sys_socket.c`) and the socket filters of `kqueue(2)`
//! (`kern/uipc_socket.c`'s `soo_kqfilter` and `filt_so*`).
//!
//! Upstream: sys/kern/sys_socket.c @ 3ce1f3f79392
//! Upstream: sys/kern/uipc_socket.c @ 3ce1f3f79392
//! LZ: sys/kern/sys_socket.rs@3c62ede68802
//! LZ: sys/kern/uipc_socket.rs@3c62ede68802
//!
//! A `DTYPE_SOCKET` file holds its socket (the glue's `f_data`, [`FileGlue::socket_of`]);
//! [`SocketFileOps`] turns `read(2)`, `write(2)`, `ioctl(2)`, `fstat(2)` and the last
//! `close(2)` into `soreceive`, `sosend`, the socket ioctls (or the interface's and the
//! protocol's), a socket `stat` and `soclose`, and `kevent(2)`'s `EVFILT_READ`,
//! `EVFILT_WRITE` and `EVFILT_EXCEPT` into a knote on the receive or send buffer's list.
//!
//! ## Deviations
//! - `fo_ioctl`'s `data` is the kernel copy of the argument (`sys_ioctl`); the `int`
//!   arguments are read and written in native byte order.
//! - `MUTEX_ASSERT_LOCKED(&sb->sb_mtx)` and `soassertlocked_readonly(so)` in the filters
//!   are `DIAGNOSTIC` checks of a holding the filters are not given a witness of: the
//!   kernel's `kdassert` of the mutex's owner, the socket lock's not checked.
//!
//! ## Redesign
//! - LZ's `socketops` was a table of `fn`s over `fp_socket(fp)`, an `unsafe` deref of the
//!   file's `f_data` word, and the filters three `Filterops` statics reaching the socket the
//!   same way. Here [`SocketFileOps`] implements `FileOps` over the glue's counted socket of
//!   the file ([`FileGlue::socket_of`]; the file is made by [`FileGlue::socket_file`]), and
//!   the filters are the [`Filter`] impls [`SoRead`], [`SoWrite`] and [`SoExcept`]: the
//!   knote holds a counted handle of its socket in its hook (`Knote::set_hook_shared`, the
//!   tty's shape), so a filter reaches the socket with no lock and no reference taken, as
//!   the C's `kn->kn_fp->f_data`; the buffers' klists are their mutex's (`klist_insert` of
//!   a mutex klist: the insert and the removal under `sb_mtx`).
//! - `soo_ioctl`'s interface group (`'i'`) goes to the interface layer still in `bsd`
//!   through the glue's `IfLegacy` ([`crate::net::domain::IfLegacy`], the `IfLegacy` row of
//!   `bridges.toml`); a protocol's `pru_control` is called without the socket's lock, as the
//!   C's.

use core::marker::PhantomData;

use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::fcntl::FNONBLOCK;
use hal::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use hal::sys::ioccom::iocgroup;
use hal::sys::socket::{MSG_DONTWAIT, SO_ACCEPTCONN};
use hal::sys::sockio::{SIOCATMARK, SIOCGPGRP, SIOCSPGRP};
use hal::sys::stat::{S_IFSOCK, S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR, Stat};

use crate::assert::kpanic;
use crate::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_EOF, EVFILT_EXCEPT, EVFILT_READ, EVFILT_WRITE,
    FILTEROP_ISFD, FILTEROP_MPSAFE, Filter, Filterops, Kevent, Knote, NOTE_LOWAT, NOTE_OOB,
    knote_modify, knote_process,
};
use crate::file::{File, FileGlue, FileOps};
use crate::net::domain::net;
use crate::net::proto::PR_WANTRCVD;
use crate::net::sockbuf::{
    SB_ASYNC, SS_CANTRCVMORE, SS_CANTSENDMORE, SS_ISCONNECTED, SS_ISDISCONNECTED, SS_RCVATMARK,
    Sockbuf,
};
use crate::net::socket::{
    Socket, SocketRef, isspliced, soclose, solock_shared, soreadable, soreceive, soref, sosend,
};
use crate::proc::{ProcGlue, Thread};
use crate::sigio::{TIOCGPGRP, TIOCSPGRP, sigio_getown, sigio_setown};
use crate::uio::Uio;

/// `socketops`: the operations of a socket file (one value per glue).
pub struct SocketFileOps<G>(PhantomData<fn() -> G>);

/// `filt_soread` and its companions (`soread_filtops`): `EVFILT_READ` on a socket.
pub struct SoRead;
/// `filt_sowrite` and its companions (`sowrite_filtops`): `EVFILT_WRITE` on a socket.
pub struct SoWrite;
/// `filt_soexcept` and its companions (`soexcept_filtops`): `EVFILT_EXCEPT` on a socket.
pub struct SoExcept;

impl<G: ProcGlue> SocketFileOps<G> {
    /// `socketops`.
    pub const NEW: Self = Self(PhantomData);
    /// `soread_filtops`.
    pub const SOREAD_FILTOPS: Filterops<G> = Filterops::of::<SoRead>();
    /// `sowrite_filtops`.
    pub const SOWRITE_FILTOPS: Filterops<G> = Filterops::of::<SoWrite>();
    /// `soexcept_filtops`.
    pub const SOEXCEPT_FILTOPS: Filterops<G> = Filterops::of::<SoExcept>();
}

/// The socket of a socket file (`fp->f_data`): `panic(9)` for another file, which a
/// socket's operations are never given.
fn socket_of<G: ProcGlue>(fp: &File<G::FileGlue>) -> &SocketRef<G> {
    match fp.data().and_then(<G::FileGlue as FileGlue>::socket_of) {
        Some(so) => so,
        None => kpanic(format_args!("file {:#x}: not a socket", fp.addr())),
    }
}

/// The `int` an `ioctl` argument holds.
fn int_arg(data: &[u8]) -> i32 {
    data.first_chunk::<4>()
        .map_or(0, |b| i32::from_ne_bytes(*b))
}

/// Stores an `int` into an `ioctl` argument.
fn set_int_arg(data: &mut [u8], v: i32) {
    if let Some(b) = data.first_chunk_mut::<4>() {
        *b = v.to_ne_bytes();
    }
}

/// The `MSG_DONTWAIT` of a non-blocking file.
fn dontwait<G: FileGlue>(fp: &File<G>) -> i32 {
    if fp.flag() & FNONBLOCK != 0 {
        MSG_DONTWAIT
    } else {
        0
    }
}

impl<G: ProcGlue> FileOps<G::FileGlue> for SocketFileOps<G> {
    /// `soo_read(fp, uio, fflags)`.
    fn read(
        &self,
        fp: &File<G::FileGlue>,
        uio: &mut Uio<'_, G>,
        _fflags: i32,
    ) -> Result<(), Errno> {
        let so = socket_of::<G>(fp);
        let mut flags = dontwait(fp);
        soreceive(so, None, uio, None, None, Some(&mut flags), 0)
    }

    /// `soo_write(fp, uio, fflags)`.
    fn write(
        &self,
        fp: &File<G::FileGlue>,
        uio: &mut Uio<'_, G>,
        _fflags: i32,
    ) -> Result<(), Errno> {
        let so = socket_of::<G>(fp);
        sosend(so, None, Some(uio), None, None, dontwait(fp))
    }

    /// `soo_ioctl(fp, cmd, data, p)`.
    fn ioctl(
        &self,
        fp: &File<G::FileGlue>,
        cmd: u64,
        data: &mut [u8],
        p: &Thread<G>,
    ) -> Result<(), Errno> {
        let so = socket_of::<G>(fp);

        match cmd {
            FIOASYNC => {
                let mut rg = so.so_rcv.lock();
                let mut sg = so.so_snd.lock();
                if int_arg(data) != 0 {
                    rg.set_flags(SB_ASYNC);
                    sg.set_flags(SB_ASYNC);
                } else {
                    rg.clear_flags(SB_ASYNC);
                    sg.clear_flags(SB_ASYNC);
                }
                drop(sg);
                drop(rg);
            }
            FIONREAD => set_int_arg(data, so.so_rcv.datacc() as i32),
            FIOSETOWN | SIOCSPGRP | TIOCSPGRP => {
                return sigio_setown::<G>(so.sigio(), cmd, int_arg(data));
            }
            FIOGETOWN | SIOCGPGRP | TIOCGPGRP => {
                set_int_arg(data, sigio_getown(so.sigio(), cmd));
            }
            SIOCATMARK => set_int_arg(data, i32::from(so.so_rcv.has_state(SS_RCVATMARK))),
            _ => {
                // Interface/routing/protocol specific ioctls: interface and routing ioctls
                // should have a different entry since a socket's unnecessary
                if iocgroup(cmd) == u64::from(b'i') {
                    return match net::<G>().iflegacy.get() {
                        Some(i) => i.ifioctl(so, cmd, data, p),
                        None => kpanic(format_args!("soo_ioctl: no interface layer")),
                    };
                }
                if iocgroup(cmd) == u64::from(b'r') {
                    return Err(Errno::EOPNOTSUPP);
                }
                return so.usrreqs().control(so, cmd, data, None);
            }
        }
        Ok(())
    }

    /// `soo_kqfilter(fp, kn)`: the knote on the receive buffer's list (`EVFILT_READ`,
    /// `EVFILT_EXCEPT`) or the send buffer's (`EVFILT_WRITE`); `EINVAL` for another filter.
    fn kqfilter(&self, fp: &File<G::FileGlue>, kn: &Knote<G>) -> Result<(), Errno> {
        let so = socket_of::<G>(fp);
        let (fop, sb): (&'static Filterops<G>, &Sockbuf<G>) = match kn.kn_filter().get() {
            EVFILT_READ => (&Self::SOREAD_FILTOPS, &so.so_rcv),
            EVFILT_WRITE => (&Self::SOWRITE_FILTOPS, &so.so_snd),
            EVFILT_EXCEPT => (&Self::SOEXCEPT_FILTOPS, &so.so_rcv),
            _ => return Err(Errno::EINVAL),
        };
        kn.kn_fop.set(Some(fop));
        // The knote's own handle of its socket, which its filters read (`kn->kn_fp->f_data`).
        drop(kn.set_hook_shared(PoolArc::erase(soref(so))));

        let mut g = sb.lock();
        sb.klist.insert_locked(g.witness(), kn);
        drop(g);

        Ok(())
    }

    /// `soo_stat(fp, ub, p)`.
    fn stat(&self, fp: &File<G::FileGlue>, ub: &mut Stat, _p: &Thread<G>) -> Result<(), Errno> {
        let so = socket_of::<G>(fp);

        *ub = Stat::default();
        ub.st_mode = S_IFSOCK;
        let mut w = solock_shared(so);
        let rg = so.so_rcv.lock();
        if !so.so_rcv.has_state(SS_CANTRCVMORE) || so.so_rcv.cc() != 0 {
            ub.st_mode |= S_IRUSR | S_IRGRP | S_IROTH;
        }
        drop(rg);
        let sg = so.so_snd.lock();
        if !so.so_snd.has_state(SS_CANTSENDMORE) {
            ub.st_mode |= S_IWUSR | S_IWGRP | S_IWOTH;
        }
        drop(sg);
        let (euid, _, egid, _, _) = so.creds();
        ub.st_uid = euid;
        ub.st_gid = egid;
        let _ = so.usrreqs().sense(so, &mut w, ub);
        drop(w);
        Ok(())
    }

    /// `soo_close(fp, p)`: the file's last reference is gone: `soclose`; the socket goes
    /// with the file's object.
    fn close(&self, fp: &File<G::FileGlue>, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        match fp.data().and_then(<G::FileGlue as FileGlue>::socket_of) {
            Some(so) => soclose(so, dontwait(fp)),
            None => Ok(()),
        }
    }
}

/// The socket of a knote of [`SocketFileOps::kqfilter`] (its hook): `panic(9)` for another
/// knote, which the socket filters are never given.
fn so_of<G: ProcGlue>(kn: &Knote<G>) -> &Socket<G> {
    match kn.hook_shared::<Socket<G>>() {
        Some(so) => so,
        None => kpanic(format_args!("knote {kn:p}: no socket")),
    }
}

/// `filt_sordetach(kn)`, `filt_sowdetach(kn)`: the knote off the buffer's list.
fn filt_sodetach<G: ProcGlue>(kn: &Knote<G>, sb: &Sockbuf<G>) {
    let mut g = sb.lock();
    sb.klist.remove_locked(g.witness(), kn);
}

/// `filt_soread(kn, hint)`, `so_rcv`'s mutex held.
pub fn filt_soread<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    let so = so_of(kn);
    let state = so.state();
    let error = so.error_raw();
    crate::assert::kdassert!(so.so_rcv.mutex().owned());

    if so.has_options(SO_ACCEPTCONN) {
        let qlen = so.qlen();

        kn.kn_data().set(i64::from(qlen));
        let mut rv = qlen != 0;

        if kn.has_flags(__EV_POLL | __EV_SELECT) {
            if state & SS_ISDISCONNECTED != 0 {
                kn.set_flags(__EV_HUP);
                rv = true;
            } else {
                rv = qlen != 0 || soreadable(so);
            }
        }

        return rv;
    }

    kn.kn_data().set(so.so_rcv.cc() as i64);
    if isspliced(so) {
        false
    } else if so.so_rcv.has_state(SS_CANTRCVMORE) {
        kn.set_flags(EV_EOF);
        if kn.has_flags(__EV_POLL) && state & SS_ISDISCONNECTED != 0 {
            kn.set_flags(__EV_HUP);
        }
        kn.kn_fflags().set(error);
        true
    } else if error != 0 {
        true
    } else if kn.kn_sfflags.get() & NOTE_LOWAT != 0 {
        kn.kn_data().get() >= kn.kn_sdata.get()
    } else {
        kn.kn_data().get() >= so.so_rcv.lowat()
    }
}

/// `filt_sowrite(kn, hint)`, `so_snd`'s mutex held.
pub fn filt_sowrite<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    let so = so_of(kn);
    let state = so.state();
    let error = so.error_raw();
    crate::assert::kdassert!(so.so_snd.mutex().owned());

    kn.kn_data().set(so.so_snd.space_unlocked());
    if so.so_snd.has_state(SS_CANTSENDMORE) {
        kn.set_flags(EV_EOF);
        if kn.has_flags(__EV_POLL) && state & SS_ISDISCONNECTED != 0 {
            kn.set_flags(__EV_HUP);
        }
        kn.kn_fflags().set(error);
        true
    } else if error != 0 {
        true
    } else if state & SS_ISCONNECTED == 0 && so.pr_flags(crate::net::proto::PR_CONNREQUIRED) {
        false
    } else if kn.kn_sfflags.get() & NOTE_LOWAT != 0 {
        kn.kn_data().get() >= kn.kn_sdata.get()
    } else {
        kn.kn_data().get() >= so.so_snd.lowat()
    }
}

/// `filt_soexcept(kn, hint)`, `so_rcv`'s mutex held.
pub fn filt_soexcept<G: ProcGlue>(kn: &Knote<G>, _hint: i64) -> bool {
    let so = so_of(kn);
    let mut rv = false;
    crate::assert::kdassert!(so.so_rcv.mutex().owned());

    if isspliced(so) {
        rv = false;
    } else if kn.kn_sfflags.get() & NOTE_OOB != 0
        && (so.oobmark() != 0 || so.so_rcv.has_state(SS_RCVATMARK))
    {
        kn.kn_fflags().set(kn.kn_fflags().get() | NOTE_OOB);
        kn.kn_data()
            .set(kn.kn_data().get().wrapping_sub(so.oobmark() as i64));
        rv = true;
    }

    if kn.has_flags(__EV_POLL) && so.state() & SS_ISDISCONNECTED != 0 {
        kn.set_flags(__EV_HUP);
        rv = true;
    }

    rv
}

/// `knote_modify`/`knote_process` under `sb`'s mutex, and under the shared socket lock
/// first for a receive buffer of a protocol that wants `PRU_RCVD` (`filt_sormodify`,
/// `filt_sorprocess`).
fn under_sb<G: ProcGlue, R>(
    so: &Socket<G>,
    sb: &Sockbuf<G>,
    rcv: bool,
    f: impl FnOnce() -> R,
) -> R {
    let w = (rcv && so.pr_flags(PR_WANTRCVD)).then(|| solock_shared(so));
    let g = sb.lock();
    let rv = f();
    drop(g);
    drop(w);
    rv
}

impl<G: ProcGlue> Filter<G> for SoRead {
    const FLAGS: i32 = FILTEROP_ISFD | FILTEROP_MPSAFE;

    fn detach(kn: &Knote<G>) {
        filt_sodetach(kn, &so_of(kn).so_rcv);
    }

    fn event(kn: &Knote<G>, hint: i64) -> bool {
        filt_soread(kn, hint)
    }

    /// `filt_sormodify(kev, kn)`.
    fn modify(kev: &mut Kevent, kn: &Knote<G>) -> bool {
        let so = so_of(kn);
        under_sb(so, &so.so_rcv, true, || knote_modify(kev, kn))
    }

    /// `filt_sorprocess(kn, kev)`.
    fn process(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
        let so = so_of(kn);
        under_sb(so, &so.so_rcv, true, || knote_process(kn, kev))
    }
}

impl<G: ProcGlue> Filter<G> for SoWrite {
    const FLAGS: i32 = FILTEROP_ISFD | FILTEROP_MPSAFE;

    fn detach(kn: &Knote<G>) {
        filt_sodetach(kn, &so_of(kn).so_snd);
    }

    fn event(kn: &Knote<G>, hint: i64) -> bool {
        filt_sowrite(kn, hint)
    }

    /// `filt_sowmodify(kev, kn)`.
    fn modify(kev: &mut Kevent, kn: &Knote<G>) -> bool {
        let so = so_of(kn);
        under_sb(so, &so.so_snd, false, || knote_modify(kev, kn))
    }

    /// `filt_sowprocess(kn, kev)`.
    fn process(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
        let so = so_of(kn);
        under_sb(so, &so.so_snd, false, || knote_process(kn, kev))
    }
}

impl<G: ProcGlue> Filter<G> for SoExcept {
    const FLAGS: i32 = FILTEROP_ISFD | FILTEROP_MPSAFE;

    fn detach(kn: &Knote<G>) {
        filt_sodetach(kn, &so_of(kn).so_rcv);
    }

    fn event(kn: &Knote<G>, hint: i64) -> bool {
        filt_soexcept(kn, hint)
    }

    /// `filt_soemodify(kev, kn)`.
    fn modify(kev: &mut Kevent, kn: &Knote<G>) -> bool {
        let so = so_of(kn);
        under_sb(so, &so.so_rcv, false, || knote_modify(kev, kn))
    }

    /// `filt_soeprocess(kn, kev)`.
    fn process(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
        let so = so_of(kn);
        under_sb(so, &so.so_rcv, false, || knote_process(kn, kev))
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    //! The socket file over the UNIX domain's test setup (`kernel::net::unix`'s): `read(2)`,
    //! `write(2)` and their non-blocking form, the socket ioctls, `fstat(2)`, the last
    //! `close(2)`; `kevent(2)` on a pair (the knotes on the buffers' lists, activated by
    //! `sowakeup`, `EV_EOF` at the peer's close, `EINVAL` for a filter sockets do not have)
    //! and the filters called directly (`NOTE_LOWAT`, poll's `__EV_HUP`), as LZ's tests.

    use core::ffi::c_void;
    use core::sync::atomic::Ordering;
    use std::{assert, assert_eq};

    use hal::sys::socket::SHUT_WR;
    use hal::user::UserPtr;

    use super::*;
    use crate::clock::Timespec;
    use crate::descrip::{fd_getfile, sys_close};
    use crate::event::{
        EV_ADD, EV_ONESHOT, EVFILT_VNODE, KqRef, ev_set, kqueue_init, kqueue_register, kqueue_scan,
        kqueue_scan_setup, sys_kqueue,
    };
    use crate::fs::testfs::TestFiles;
    use crate::net::socket::soshutdown;
    use crate::net::unix::tests::{G, So, pair, recv, send, setup, teardown};
    use crate::testutil::{USER_BASE, user_bytes};
    use crate::uio::{Iovec, UioRw, UioSeg};

    /// The file of descriptor `fd` of `p`.
    fn fp_of(p: &Thread<G>, fd: i32) -> crate::file::FileRef<TestFiles> {
        fd_getfile(p.fd(), fd).expect("an open descriptor")
    }

    /// `read(2)` or `write(2)` of `len` bytes at `at` through the file's operations.
    fn rw(p: &Thread<G>, fd: i32, at: usize, len: usize, rw: UioRw) -> Result<usize, Errno> {
        let fp = fp_of(p, fd);
        let mut iov = [Iovec {
            iov_base: at as *mut c_void,
            iov_len: len,
        }];
        let mut uio = Uio {
            uio_resid: len,
            uio_iov: &mut iov,
            uio_offset: 0,
            uio_segflg: UioSeg::UIO_USERSPACE,
            uio_rw: rw,
            uio_procp: Some(p),
        };
        let ops = fp.ops();
        let r = match rw {
            UioRw::UIO_READ => ops.read(&fp, &mut uio, 0),
            UioRw::UIO_WRITE => ops.write(&fp, &mut uio, 0),
        };
        r.map(|()| len - uio.uio_resid)
    }

    /// An `ioctl(2)` with an `int` argument: the argument after the call.
    fn ioctl(p: &Thread<G>, fd: i32, cmd: u64, arg: i32) -> Result<i32, Errno> {
        let fp = fp_of(p, fd);
        let mut data = arg.to_ne_bytes();
        fp.ops().ioctl(&fp, cmd, &mut data, p)?;
        Ok(i32::from_ne_bytes(data))
    }

    /// One non-blocking scan of `kq`: the events and the scan's error.
    fn scan(p: &Thread<G>, kq: &KqRef<G>, out: &mut [Kevent]) -> (usize, Result<(), Errno>) {
        let start = core::pin::pin!(Knote::marker(kq));
        let end = core::pin::pin!(Knote::marker(kq));
        let s = kqueue_scan_setup(kq, start.as_ref(), end.as_ref());
        let mut ts = Timespec::ZERO;
        let n = out.len();
        kqueue_scan(&s, n, out, Some(&mut ts), p)
    }

    /// A knote for calling the filters directly: `filter`, `flags` and `sfflags` set, its
    /// hook the socket `so`.
    fn knote_on(so: &So, filter: i16, flags: u16, sfflags: u32) -> Knote<G> {
        let kn = Knote::detached();
        kn.kn_filter().set(filter);
        kn.kn_flags().set(flags);
        kn.kn_sfflags.set(sfflags);
        assert!(kn.set_hook_shared(PoolArc::erase(so.clone())).is_ok());
        kn
    }

    #[test]
    fn read_write_ioctl_and_stat_go_through_the_file() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, hal::sys::socket::SOCK_STREAM);
        let at = USER_BASE + 0x2_0000;

        UserPtr::<u8>::new(at)
            .copyout_slice(b"hello")
            .expect("user memory");
        assert_eq!(rw(p, fa, at, 5, UioRw::UIO_WRITE), Ok(5));
        // FIONREAD: the data waiting; SIOCATMARK: no out-of-band mark.
        assert_eq!(ioctl(p, fb, FIONREAD, 0), Ok(5));
        assert_eq!(ioctl(p, fb, SIOCATMARK, 7), Ok(0));
        assert_eq!(rw(p, fb, at + 0x100, 16, UioRw::UIO_READ), Ok(5));
        assert_eq!(user_bytes(at + 0x100, 5), Some(b"hello".to_vec()));

        // A non-blocking file: an empty buffer is EWOULDBLOCK.
        fp_of(p, fb)
            .f_flag
            .fetch_or(FNONBLOCK as u32, Ordering::Relaxed);
        assert_eq!(rw(p, fb, at, 16, UioRw::UIO_READ), Err(Errno::EWOULDBLOCK));

        // FIOASYNC on both buffers; the owner set and read back.
        assert_eq!(ioctl(p, fb, FIOASYNC, 1), Ok(1));
        assert!(b.so_rcv.has_flags(SB_ASYNC) && b.so_snd.has_flags(SB_ASYNC));
        assert_eq!(ioctl(p, fb, FIOASYNC, 0), Ok(0));
        assert!(!b.so_rcv.has_flags(SB_ASYNC) && !b.so_snd.has_flags(SB_ASYNC));
        // The owner: a process that is not there is sigio_setown's ESRCH; none is 0.
        assert_eq!(ioctl(p, fb, FIOSETOWN, 99_999), Err(Errno::ESRCH));
        assert_eq!(ioctl(p, fb, FIOGETOWN, 7), Ok(0));
        assert_eq!(ioctl(p, fb, SIOCGPGRP, 7), Ok(0));
        assert_eq!(ioctl(p, fb, FIOSETOWN, 0), Ok(0));
        // The routing group, and a protocol request the UNIX domain does not have.
        assert_eq!(
            ioctl(p, fb, hal::sys::ioccom::_iow::<i32>(b'r', 1), 0),
            Err(Errno::EOPNOTSUPP)
        );
        assert_eq!(
            ioctl(p, fb, hal::sys::ioccom::_iow::<i32>(b'z', 1), 0),
            Err(Errno::EOPNOTSUPP)
        );

        // fstat(2): a socket, readable and writable, owned by its creator, the fake inode.
        let fp = fp_of(p, fb);
        let mut st = Stat::default();
        assert_eq!(fp.ops().stat(&fp, &mut st, p), Ok(()));
        assert_eq!(st.st_mode, S_IFSOCK | 0o666);
        assert_eq!(st.st_uid, p.ucred().cr_uid.get());
        assert_ne!(st.st_ino, 0);
        // Shut down for writing: not writable any more; the peer's side still readable.
        assert_eq!(soshutdown(&b, SHUT_WR), Ok(()));
        assert_eq!(fp.ops().stat(&fp, &mut st, p), Ok(()));
        assert_eq!(st.st_mode, S_IFSOCK | 0o444);
        drop(fp);

        drop((a, b));
        assert_eq!(sys_close(p, fa), Ok(()));
        assert_eq!(sys_close(p, fb), Ok(()));
        teardown();
    }

    #[test]
    fn kevent_on_a_socket_pair() {
        let (_g, p) = setup();
        let _ = kqueue_init::<G>();
        let ((a, fa), (b, fb)) = pair(p, hal::sys::socket::SOCK_STREAM);
        let kfd = sys_kqueue(p).expect("kqueue");
        let kq = {
            let fp = fp_of(p, kfd);
            fp.data()
                .and_then(<TestFiles as FileGlue>::kqueue_of)
                .expect("a kqueue")
                .clone()
        };
        let mut out = [Kevent::default(); 4];

        // EVFILT_READ on a: hooked on a's receive buffer, nothing pending.
        let mut kev = ev_set(fa as usize, EVFILT_READ, EV_ADD, 0, 0, 7);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert!(!a.so_rcv.klist.is_empty());
        assert!(a.so_snd.klist.is_empty());
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));

        // b sends: sowakeup's knote_locked activates the knote, with the byte count.
        assert_eq!(send(p, &b, b"hello", 0), Ok(5));
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].filter), (fa as usize, EVFILT_READ));
        assert_eq!(
            (out[0].data, out[0].udata, out[0].flags & EV_EOF),
            (5, 7, 0)
        );

        // Drained: the next scan finds the knote inactive.
        assert_eq!(recv(p, &a, 8, 0).map(|(d, _)| d.len()), Ok(5));
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));

        // A one-shot EVFILT_WRITE on b: room to send, then gone from b's send buffer.
        let mut kev = ev_set(fb as usize, EVFILT_WRITE, EV_ADD | EV_ONESHOT, 0, 0, 8);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert!(!b.so_snd.klist.is_empty());
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].filter), (fb as usize, EVFILT_WRITE));
        assert!(out[0].data > 0);
        assert!(b.so_snd.klist.is_empty());

        // EVFILT_EXCEPT for out-of-band data: hooked on a's receive buffer, quiet; no mark.
        let mut kev = ev_set(fa as usize, EVFILT_EXCEPT, EV_ADD, NOTE_OOB, 0, 0);
        assert_eq!(kqueue_register(&kq, &mut kev, 0, Some(p)), Ok(()));
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));
        crate::net::socket::sohasoutofband(&a);
        assert_eq!(scan(p, &kq, &mut out), (0, Ok(())));

        // Sockets have no vnode filter.
        let mut kev = ev_set(fa as usize, EVFILT_VNODE, EV_ADD, 0, 0, 0);
        assert_eq!(
            kqueue_register(&kq, &mut kev, 0, Some(p)),
            Err(Errno::EINVAL)
        );

        // b goes away: a's read knote reports EOF; the except knote stays quiet.
        drop(b);
        assert_eq!(sys_close(p, fb), Ok(()));
        assert_eq!(scan(p, &kq, &mut out), (1, Ok(())));
        assert_eq!((out[0].ident, out[0].filter), (fa as usize, EVFILT_READ));
        assert!(out[0].flags & EV_EOF != 0);

        // Closing the kqueue detaches its knotes from a's lists.
        drop(kq);
        assert_eq!(sys_close(p, kfd), Ok(()));
        assert!(a.so_rcv.klist.is_empty());
        drop(a);
        assert_eq!(sys_close(p, fa), Ok(()));
        teardown();
    }

    #[test]
    fn the_read_write_and_except_filters() {
        let (_g, p) = setup();
        let ((a, fa), (b, fb)) = pair(p, hal::sys::socket::SOCK_STREAM);

        // NOTE_LOWAT: readable from the user's mark on, not the buffer's.
        let rd = knote_on(&a, EVFILT_READ, 0, NOTE_LOWAT);
        rd.kn_sdata.set(3);
        assert_eq!(send(p, &b, b"ab", 0), Ok(2));
        let g = a.so_rcv.lock();
        assert!(!filt_soread(&rd, 0));
        assert_eq!(rd.kn_data().get(), 2);
        drop(g);
        assert_eq!(send(p, &b, b"c", 0), Ok(1));
        let g = a.so_rcv.lock();
        assert!(filt_soread(&rd, 0));
        assert_eq!(rd.kn_data().get(), 3);
        drop(g);

        // The writer has room on a connected stream.
        let wr = knote_on(&b, EVFILT_WRITE, 0, 0);
        let g = b.so_snd.lock();
        assert!(filt_sowrite(&wr, 0));
        assert!(wr.kn_data().get() > 0 && !wr.has_flags(EV_EOF));
        drop(g);
        drop((rd, wr));

        // b goes away: for poll, a reads EOF and a hang-up, and so does its except filter.
        drop(b);
        assert_eq!(sys_close(p, fb), Ok(()));
        let rd = knote_on(&a, EVFILT_READ, __EV_POLL, 0);
        let ex = knote_on(&a, EVFILT_EXCEPT, __EV_POLL, 0);
        let g = a.so_rcv.lock();
        assert!(filt_soread(&rd, 0));
        assert!(rd.has_flags(EV_EOF) && rd.has_flags(__EV_HUP));
        assert!(filt_soexcept(&ex, 0) && ex.has_flags(__EV_HUP));
        drop(g);
        drop((rd, ex));
        // Through kevent(2) (no __EV_POLL): EOF without the hang-up; no except event.
        let rd = knote_on(&a, EVFILT_READ, 0, 0);
        let ex = knote_on(&a, EVFILT_EXCEPT, 0, 0);
        let g = a.so_rcv.lock();
        assert!(filt_soread(&rd, 0));
        assert!(rd.has_flags(EV_EOF) && !rd.has_flags(__EV_HUP));
        assert!(!filt_soexcept(&ex, 0));
        drop(g);
        drop((rd, ex));

        drop(a);
        assert_eq!(sys_close(p, fa), Ok(()));
        teardown();
    }
}
/* </TESTS> */
