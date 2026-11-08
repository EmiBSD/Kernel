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
 * Copyright (c) 1982, 1986, 1989, 1990, 1993
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
 *	@(#)uipc_syscalls.c	8.4 (Berkeley) 2/21/94
 */
/* </LICENSES> */

/* <CODE> */
//! The socket system calls: `kern/uipc_syscalls.c`.
//!
//! Upstream: sys/kern/uipc_syscalls.c @ 3ce1f3f79392
//! LZ: sys/kern/uipc_syscalls.rs@3c62ede68802
//!
//! `socket(2)`, `socketpair(2)`, `bind(2)`, `listen(2)`, `accept(2)` and `accept4(2)`
//! (`doaccept`), `connect(2)`, the send calls (`sendto(2)`, `sendmsg(2)`, `sendmmsg(2)`, all
//! through [`sendit`]), the receive calls (`recvfrom(2)`, `recvmsg(2)`, `recvmmsg(2)`,
//! through [`recvit`]), `shutdown(2)`, `setsockopt(2)` and `getsockopt(2)`,
//! `getsockname(2)` and `getpeername(2)`, and `ypconnect(2)`, over the socket layer
//! (`kernel::net::socket`) and the descriptor tables (`kernel::descrip`). [`sockargs`] copies
//! a socket address or control data in from user space into an mbuf, [`getsock`] finds the
//! socket file of a descriptor, [`copyaddrout`] copies an address back out. `setrtable(2)`
//! and `getrtable(2)`, the C file's other two, are `kernel::prot`'s (N4.6a).
//!
//! The ABI reaches these through `KernelApi::socket` (`hal::api::SocketEntries`): `bsd`'s
//! `native_*` wrappers run the system call prologue still in `bsd` (pledge, the pinned
//! system calls; every row is `NOLOCK`) on their `sysent[]` row, whose function is a one-line
//! adapter to the function here (`sys/kern/socket_glue.rs`, until N9).
//!
//! ## Deviations
//! - `KTRACE` is not configured (`ktrsockaddr`, `ktrmsghdr`, `ktrmmsghdr`, `ktriovec`,
//!   `ktrgenio`, `ktrfds`, `ktrcmsghdr`, `ktrreltimespec`), as in LZ.
//! - `m_get(M_WAIT)` and `mallocarray(M_WAITOK)` answer `None` when the pool or the malloc
//!   type cannot give one (the C's sleep for it): `ENOBUFS` for an mbuf, `ENOMEM` for an
//!   iovec array (LZ's).
//! - `dns_portcheck`'s `AF_INET6` arm is not under `INET6`: only a socket of the INET6 domain
//!   reaches it, and the domain exists only with the option (`bsd`'s `inet6` feature), so the
//!   arm answers as the C's whether the option is on or not.
//! - The few `<netinet/in.h>` and `<netinet6/in6.h>` items these calls read (the size of
//!   `struct sockaddr_in` and `struct sockaddr_in6`, where their port is, `IPPORT_RESERVED`,
//!   `IPPROTO_IP`, `IP_PORTRANGE`, `IP_PORTRANGE_LOW`) are constants here until N6.4 moves the
//!   headers into `hal::sys` (`bsd`'s glue tests check them against `bsd`'s copies).
//!
//! ## Redesign
//! - LZ's calls were `bsd`'s `sysent[]` rows over `&'static` sockets and mbufs (`fp_socket`,
//!   an `unsafe` view of `f_data`; `mtod` slices built in `unsafe` blocks; the iovec arrays
//!   `mallocarray`ed and written through raw pointers, `iovs_free` on every path; eleven
//!   `unsafe`). Here every call takes typed arguments (`UserPtr<T>` for a user structure, the
//!   registers' integers) and returns its value (`Result<i32>` for a descriptor, `Result<usize>`
//!   for a byte count, `Result<u32>` for a message count): the adapter puts it in `retval[0]`.
//!   A file's socket is borrowed from the file (`FileGlue::socket_of`) for as long as the
//!   call's reference of the file lives; the file is let go once, after an inner function that
//!   returns early with `?` (the C's `goto bad`/`out` labels: [`sendit`], [`recvit`], the
//!   option and name calls).
//! - `struct msghdr` and `struct mmsghdr` are copied as `hal::sys::socket`'s images
//!   (`UserPtr<Msghdr>::copyin`, `copyout`), their holes with them: `recvmsg(2)` and the
//!   `mmsg` calls copy the structure back out whole, the user's padding bytes kept, as the
//!   C's `copyout(&msg)` of the structure it copied in (LZ copied field by field and zeroed
//!   them). The iovecs go to a `kernel::uio::IovBuf` (`IovBuf::copyin`: the stack part or one
//!   `M_IOV` array, freed with the buffer; `sendmmsg(2)` and `recvmmsg(2)` keep one buffer
//!   across their messages and grow its array as the C's), and the kernel copy of the iovecs
//!   is a slice passed beside the message (the C swaps `msg_iov` back and forth around the
//!   call). The lengths are summed by `kernel::uio::iovec_resid` where the C sums them, after
//!   the descriptor and the pledge check.
//! - An address or control mbuf is a `MbufBox` (dropped where the C `m_freem`s it), lent as
//!   `&Mbuf` to the protocol (`sobind`, `soconnect`, `sosend`'s `addr`); a received address
//!   and control chain are read through `Mbuf::data`. `sockargs` and `ypsockargs` share one
//!   body ([`mbuf_args`]), the bytes from user space or from the kernel.
//! - `ypconnect(2)`'s `sotoinpcb(so)->inp_flags |= INP_LOWPORT`: the kernel holds no
//!   reference to `bsd`'s internet control block (the socket's `legacy_pcb` word), so it asks
//!   the protocol for the same flag through the protocol's own option, `IP_PORTRANGE` set to
//!   `IP_PORTRANGE_LOW` (`ip_ctloutput`: `inp_flags &= ~INP_HIGHPORT; inp_flags |=
//!   INP_LOWPORT`), under the same `solock(so)`; a fresh socket has no `INP_HIGHPORT`, so the
//!   flags end as the C's. The binding file's name is a `namei_pool` buffer
//!   (`fs::namei::pathbuf_get`), as the C's (LZ built it on its stack).
//! - The pledge checks are the glue's (`ProcGlue::pledge_socket`, `pledge_sendit`,
//!   `pledge_sockopt`, `bsd`'s `kern_pledge` until N8), called at the C's points with the
//!   C's arguments (`pledge_sockopt`'s protocol is the socket's `Protocol`, whose domain's
//!   family and number `kern_pledge` reads); none is skipped or moved.
//! - `getsock` is this module's for every caller (`kernel::log`'s `LIOCSFD` had a copy);
//!   `copyaddrout` no longer takes the thread (it was `KTRACE`'s).
//! - Locks against the C, per call: one `fd_getfile` (the table's), the file's `f_mtx` once
//!   for the transfer counters, `solock_shared` around `sobind`, `solisten`, `soconnect` and
//!   its wait, `pru_sockaddr` and `pru_peeraddr` as the C; the send and receive paths take
//!   only the socket layer's own (`kernel::net::socket`'s table). Nothing is looked up twice:
//!   the socket is the file's, borrowed.

use core::ptr;
use core::sync::atomic::Ordering;

use hal::sleep::{INFSLP, PCATCH};
use hal::sys::errno::Errno;
use hal::sys::fcntl::{F_GETLK, F_POSIX, F_UNLCK, F_WRLCK, FNONBLOCK, FREAD, FWRITE, Flock};
use hal::sys::limits::UCHAR_MAX;
use hal::sys::pledge::{PLEDGE_DNS, PLEDGE_RPATH};
use hal::sys::socket::{
    AF_INET, AF_INET6, Cmsghdr, MSG_CTRUNC, MSG_DONTWAIT, MSG_NOSIGNAL, MSG_OOB, MSG_WAITFORONE,
    Mmsghdr, Msghdr, SO_ACCEPTCONN, SOCK_CLOEXEC, SOCK_CLOFORK, SOCK_DGRAM, SOCK_DNS,
    SOCK_NONBLOCK, SOCK_NONBLOCK_INHERIT, SOCK_STREAM, SOCK_TYPE_MASK, cmsg_align,
};
use hal::sys::syslimits::IOV_MAX;
use hal::sys::unistd::SEEK_SET;
use hal::user::UserPtr;

use crate::assert::kpanic;
use crate::clock::Timespec;
use crate::descrip::{
    UF_EXCLOSE, UF_FORKCLOSE, closef, falloc, fd_getfile, fdinsert, fdplock, fdpunlock, fdremove,
};
use crate::file::{DTYPE_SOCKET, File, FileGlue, FileRef, LockOwner};
use crate::fs::lookup::{namei, ndinit};
use crate::fs::namei::{KERNELPATH, LOCKLEAF, LOOKUP, MAXPATHLEN, NOFOLLOW, NiDirp, pathbuf_get};
use crate::fs::subr::vput;
use crate::fs::vnode::{VREG, Vattr, Vnode};
use crate::fs::vops::{Cred, vop_advlock, vop_getattr, vop_read};
use crate::net::mbuf::{
    M_EXT, M_WAIT, MCLBYTES, MLEN, MT_CONTROL, MT_SONAME, MT_SOOPTS, Mbuf, MbufBox, m_get,
    m_getclr, mclget,
};
use crate::net::proto::PRCO_SETOPT;
use crate::net::sockbuf::{PSOCK, SS_CANTRCVMORE, SS_DNS, SS_ISCONNECTED, SS_ISCONNECTING, SS_YP};
use crate::net::socket::{
    SoLocked, Socket, SocketRef, soaccept, soaccept_take, sobind, soclose, soconnect, soconnect2,
    socreate, sogetopt, solisten, solock, solock_shared, soreceive, sosend, sosetopt, soshutdown,
    sosleep_nsec,
};
use crate::proc::{PS_CHROOT, PS_PLEDGE, ProcGlue, Thread, suser};
use crate::sysctl::{MAXHOSTNAMELEN, with_domainname};
use crate::uio::{IovBuf, Iovec, Uio, UioRw, UioSeg, iovec_resid};

/// `sizeof(struct sockaddr_in)` (`<netinet/in.h>`).
pub const SOCKADDR_IN_SIZE: usize = 16;
/// `sizeof(struct sockaddr_in6)` (`<netinet6/in6.h>`).
pub const SOCKADDR_IN6_SIZE: usize = 28;
/// Where `sin_port` and `sin6_port` sit in their address (after the length and the family).
pub const SIN_PORT_OFFSET: usize = 2;
/// Where `sin_addr` sits in a `struct sockaddr_in`.
pub const SIN_ADDR_OFFSET: usize = 4;
/// `IPPORT_RESERVED`: ports below it are privileged (`<netinet/in.h>`).
pub const IPPORT_RESERVED: u16 = 1024;
/// `IPPROTO_IP`: the IP level of an option (`<netinet/in.h>`).
pub const IPPROTO_IP: i32 = 0;
/// `IP_PORTRANGE`: the range a socket's unspecified port is chosen in (`<netinet/in.h>`).
pub const IP_PORTRANGE: i32 = 19;
/// `IP_PORTRANGE_LOW`: the low, privileged range (`<netinet/in.h>`).
pub const IP_PORTRANGE_LOW: i32 = 2;
/// `offsetof(struct sockaddr, sa_data)`: the length and the family, which a name must hold.
const SA_DATA_OFFSET: usize = 2;
/// `sizeof(struct ypbinding)` (`__packed`): what `ypbind(8)` writes to the binding file.
const YPBINDING_SIZE: usize = 16;
/// `ypbinding.in`: the server's address (network order).
const YPB_IN: usize = 6;
/// `ypbinding.ypserv_udp_port` (network order).
const YPB_UDP_PORT: usize = 10;
/// `ypbinding.ypserv_tcp_port` (network order).
const YPB_TCP_PORT: usize = 14;
/// `sendmmsg(2)` and `recvmmsg(2)`: arbitrarily capped at 1024 datagrams.
const MMSG_MAX: u32 = 1024;

/// The descriptor flags a socket call's `SOCK_CLOEXEC` and `SOCK_CLOFORK` ask for.
fn fdflags_of(flags: i32) -> u8 {
    (if flags & SOCK_CLOEXEC != 0 {
        UF_EXCLOSE
    } else {
        0
    }) | (if flags & SOCK_CLOFORK != 0 {
        UF_FORKCLOSE
    } else {
        0
    })
}

/// `fp->f_type = DTYPE_SOCKET; fp->f_ops = &socketops; fp->f_data = so`: `so`'s reference
/// becomes the file's (its last close is the socket's `soclose`).
fn fp_install<G: ProcGlue>(fp: &File<G::FileGlue>, so: SocketRef<G>) {
    let (ops, data) = <G::FileGlue as FileGlue>::socket_file(so);
    fp.install(DTYPE_SOCKET, ops, data);
}

/// `(struct socket *)fp->f_data` of a file [`getsock`] found: `panic(9)` for another, which
/// `getsock` never returns.
fn so_of<G: ProcGlue>(fp: &File<G::FileGlue>) -> &SocketRef<G> {
    match fp.data().and_then(<G::FileGlue as FileGlue>::socket_of) {
        Some(so) => so,
        None => kpanic(format_args!("file {:#x}: not a socket", fp.addr())),
    }
}

/// `socket(2)`: a socket of `domain`, `type_` (with `SOCK_CLOEXEC`, `SOCK_CLOFORK`,
/// `SOCK_NONBLOCK` and `SOCK_DNS` or'ed in) and `protocol` at a new descriptor, returned.
pub fn sys_socket<G: ProcGlue>(
    p: &Thread<G>,
    domain: i32,
    type_: i32,
    protocol: i32,
) -> Result<i32, Errno> {
    let mut ss = 0;

    if type_ & SOCK_DNS != 0 && !(domain == i32::from(AF_INET) || domain == i32::from(AF_INET6)) {
        return Err(Errno::EINVAL);
    }

    if type_ & SOCK_DNS != 0 {
        ss |= SS_DNS;
    }
    G::pledge_socket(p, domain, ss)?;

    let ty = type_ & !(SOCK_CLOEXEC | SOCK_CLOFORK | SOCK_NONBLOCK | SOCK_DNS);
    let fdflags = fdflags_of(type_);
    let nonblock = type_ & SOCK_NONBLOCK != 0;
    let fflag = FREAD | FWRITE | if nonblock { FNONBLOCK } else { 0 };

    let so = socreate(p, domain, ty, protocol)?;

    let mut g = fdplock(p.fd());
    match falloc(p, &mut g) {
        Err(error) => {
            fdpunlock(g);
            let _ = soclose(&so, MSG_DONTWAIT);
            Err(error)
        }
        Ok((fp, fd)) => {
            fp.f_flag.store(fflag as u32, Ordering::SeqCst);
            so.st_set(ss);
            fp_install(&fp, so);
            fdinsert(&mut g, fd, fdflags, &fp);
            fdpunlock(g);
            let _ = fp.frele(Some(p));
            Ok(fd)
        }
    }
}

/// `isdnssocket(so)`: made with `SOCK_DNS`.
#[inline]
fn isdnssocket<G: ProcGlue>(so: &Socket<G>) -> bool {
    so.has_state(SS_DNS)
}

/// `dns_portcheck(p, so, nam, namelen)`: an `SS_DNS` socket may only name port 53 (DNS):
/// `EINVAL` for another address, `EPERM` through `pledge_fail` for a pledged process.
fn dns_portcheck<G: ProcGlue>(
    p: &Thread<G>,
    so: &Socket<G>,
    nam: &[u8],
    namelen: usize,
) -> Result<(), Errno> {
    if dns_port53(so.dom_family(), nam, namelen) {
        return Ok(());
    }
    if p.process().ps_flags.load(Ordering::Relaxed) & PS_PLEDGE != 0 {
        return Err(G::pledge_fail(p, Errno::EPERM, PLEDGE_DNS));
    }
    Err(Errno::EINVAL)
}

/// `dns_portcheck`'s test: whether the address `nam` (`namelen` bytes given) of the
/// `family` is long enough for its family's `struct sockaddr_in` or `sockaddr_in6` and names
/// port 53; never for another family.
fn dns_port53(family: i32, nam: &[u8], namelen: usize) -> bool {
    let port53 = |size: usize| {
        namelen >= size
            && nam.get(SIN_PORT_OFFSET..SIN_PORT_OFFSET + 2) == Some(&53u16.to_be_bytes()[..])
    };
    match family {
        f if f == i32::from(AF_INET) => port53(SOCKADDR_IN_SIZE),
        f if f == i32::from(AF_INET6) => port53(SOCKADDR_IN6_SIZE),
        _ => false,
    }
}

/// `bind(2)`: binds the socket `s` to the address of `namelen` bytes at `name`.
pub fn sys_bind<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    name: UserPtr<u8>,
    namelen: u32,
) -> Result<(), Errno> {
    let fp = getsock(p, s)?;
    let error = bind_file(p, so_of::<G>(&fp), name, namelen);
    let _ = fp.frele(Some(p));
    error
}

/// `bind(2)` on the socket of the file the call holds.
fn bind_file<G: ProcGlue>(
    p: &Thread<G>,
    so: &Socket<G>,
    name: UserPtr<u8>,
    namelen: u32,
) -> Result<(), Errno> {
    if so.has_state(SS_YP) {
        return Err(Errno::ENOTSOCK);
    }
    G::pledge_socket(p, so.dom_family(), so.state())?;
    let nam = sockargs(name, namelen as usize, MT_SONAME)?;
    // KTRACE: not configured.
    let mut w = solock_shared(so);
    let error = sobind(so, &mut w, &nam, p);
    drop(w);
    error
}

/// `listen(2)`: makes the socket `s` accept connections, at most `backlog` of them queued.
pub fn sys_listen<G: ProcGlue>(p: &Thread<G>, s: i32, backlog: i32) -> Result<(), Errno> {
    let fp = getsock(p, s)?;
    let so = so_of::<G>(&fp);
    let error = if so.has_state(SS_YP) {
        Err(Errno::ENOTSOCK)
    } else {
        let mut w = solock_shared(so);
        solisten(so, &mut w, backlog)
    };
    let _ = fp.frele(Some(p));
    error
}

/// `accept(2)`: the first complete connection of the listening socket `s`, at a new
/// descriptor; its peer's address to `name`, the address's length to `anamelen` (which held
/// the room at `name`). The new socket is non-blocking as the listener's file is.
pub fn sys_accept<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    name: UserPtr<u8>,
    anamelen: UserPtr<u32>,
) -> Result<i32, Errno> {
    doaccept(p, s, name, anamelen, SOCK_NONBLOCK_INHERIT)
}

/// `accept4(2)`: `accept(2)` with `SOCK_CLOEXEC`, `SOCK_CLOFORK` and `SOCK_NONBLOCK` for the
/// new descriptor; `EINVAL` for another flag.
pub fn sys_accept4<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    name: UserPtr<u8>,
    anamelen: UserPtr<u32>,
    flags: i32,
) -> Result<i32, Errno> {
    if flags & !(SOCK_CLOEXEC | SOCK_CLOFORK | SOCK_NONBLOCK) != 0 {
        return Err(Errno::EINVAL);
    }
    doaccept(p, s, name, anamelen, flags)
}

/// `doaccept(p, sock, name, anamelen, flags, retval)`: takes the first complete connection
/// off the listening socket `sock` (sleeping for one unless its file is non-blocking), gives
/// it a descriptor, returned, and copies its peer's address out to `name`.
pub fn doaccept<G: ProcGlue>(
    p: &Thread<G>,
    sock: i32,
    name: UserPtr<u8>,
    anamelen: UserPtr<u32>,
    flags: i32,
) -> Result<i32, Errno> {
    let fdflags = fdflags_of(flags);

    let namelen = if name.is_null() {
        0
    } else {
        anamelen.copyin()?
    };
    let headfp = getsock(p, sock)?;

    let mut g = fdplock(p.fd());
    let alloc = falloc(p, &mut g);
    fdpunlock(g);
    let (fp, tmpfd) = match alloc {
        Ok(r) => r,
        Err(error) => {
            let _ = headfp.frele(Some(p));
            return Err(error);
        }
    };

    let error = accept_into::<G>(&headfp, &fp, name, namelen, anamelen, flags);
    match error {
        Ok(()) => {
            let mut g = fdplock(p.fd());
            fdinsert(&mut g, tmpfd, fdflags, &fp);
            fdpunlock(g);
            let _ = fp.frele(Some(p));
            let _ = headfp.frele(Some(p));
            Ok(tmpfd)
        }
        Err(error) => {
            // The descriptor (no slot holds the file yet) and the file, with the
            // connection it was given, if any.
            let mut g = fdplock(p.fd());
            let _ = fdremove(&mut g, tmpfd);
            fdpunlock(g);
            let _ = closef(fp, Some(p));
            let _ = headfp.frele(Some(p));
            Err(error)
        }
    }
}

/// `doaccept`'s work between the new file and its descriptor: the listener's checks, the
/// wait, the connection taken off the queue and given to the file `fp` (`soaccept`), and
/// the peer's address copied out.
fn accept_into<G: ProcGlue>(
    headfp: &File<G::FileGlue>,
    fp: &File<G::FileGlue>,
    name: UserPtr<u8>,
    namelen: u32,
    anamelen: UserPtr<u32>,
    flags: i32,
) -> Result<(), Errno> {
    let Some(mut nam) = m_get(M_WAIT, MT_SONAME) else {
        return Err(Errno::ENOBUFS);
    };

    let head = so_of::<G>(headfp);
    let wh = solock_shared(head);

    if isdnssocket(head) || !head.has_options(SO_ACCEPTCONN) {
        return Err(Errno::EINVAL);
    }
    let nonblock = headfp.flag() & FNONBLOCK != 0;
    if nonblock && head.qlen() == 0 {
        return Err(if head.so_rcv.has_state(SS_CANTRCVMORE) {
            Errno::ECONNABORTED
        } else {
            Errno::EWOULDBLOCK
        });
    }
    // The wait, the connection locked and taken off the queue, the knote, the listener
    // unlocked, then soaccept(so, nam) under the connection's lock: the C's sequence.
    let (so, error) = soaccept_take(head, wh, nonblock, |so, ws| {
        // Figure out whether the new socket should be non-blocking: the listener's flag as
        // it is now, after the wait (the C reads it after soqremque).
        let nflag = if flags & SOCK_NONBLOCK_INHERIT != 0 {
            headfp.flag() & FNONBLOCK
        } else if flags & SOCK_NONBLOCK != 0 {
            FNONBLOCK
        } else {
            0
        };
        fp.f_flag
            .store((FREAD | FWRITE | nflag) as u32, Ordering::SeqCst);
        soaccept(so, ws, &mut nam)
    })?;
    // `fp->f_data = so`: the queue's reference becomes the file's; the file is nobody
    // else's until `fdinsert`, so it takes the socket once `so` is unlocked, and a failed
    // accept closes it with the file.
    fp_install(fp, so);
    error?;

    if !name.is_null() {
        copyaddrout(&nam, name, namelen, anamelen)?;
    }
    Ok(())
}

/// `connect(2)`: connects the socket `s` to the address of `namelen` bytes at `name`,
/// waiting for the connection unless the socket's file is non-blocking (`EINPROGRESS`).
pub fn sys_connect<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    name: UserPtr<u8>,
    namelen: u32,
) -> Result<(), Errno> {
    let fp = getsock(p, s)?;
    let error = connect_file(p, &fp, name, namelen);
    let _ = fp.frele(Some(p));
    match error {
        Err(Errno::ERESTART) => Err(Errno::EINTR),
        error => error,
    }
}

/// `connect(2)` on the socket file the call holds.
fn connect_file<G: ProcGlue>(
    p: &Thread<G>,
    fp: &File<G::FileGlue>,
    name: UserPtr<u8>,
    namelen: u32,
) -> Result<(), Errno> {
    let so = so_of::<G>(fp);
    if so.has_state(SS_YP) {
        return Err(Errno::ENOTSOCK);
    }
    G::pledge_socket(p, so.dom_family(), so.state())?;
    let nam = sockargs(name, namelen as usize, MT_SONAME)?;
    // KTRACE: not configured.
    let mut w = solock_shared(so);
    if isdnssocket(so) {
        dns_portcheck(p, so, nam.data(), nam.m_len().get() as usize)?;
    }
    if so.has_state(SS_ISCONNECTING) {
        return Err(Errno::EALREADY);
    }
    let mut interrupted = false;
    let mut error = soconnect(so, &mut w, &nam);
    if error.is_ok() {
        if fp.flag() & FNONBLOCK != 0 && so.has_state(SS_ISCONNECTING) {
            return Err(Errno::EINPROGRESS);
        }
        while so.has_state(SS_ISCONNECTING) && so.error().is_none() {
            if let Err(e) = sosleep_nsec(&mut w, so.timeo_chan(), PSOCK | PCATCH, "netcon", INFSLP)
            {
                if e == Errno::EINTR || e == Errno::ERESTART {
                    interrupted = true;
                }
                error = Err(e);
                break;
            }
        }
        if error.is_ok() {
            if let Some(e) = so.error() {
                error = Err(e);
            }
            so.set_error(None);
        }
    }
    // bad:
    if !interrupted {
        so.st_clear(SS_ISCONNECTING);
    }
    drop(w);
    error
}

/// `socketpair(2)`: two connected sockets of `domain`, `type_` and `protocol` at two new
/// descriptors, written to the two `int`s at `rsv`.
pub fn sys_socketpair<G: ProcGlue>(
    p: &Thread<G>,
    domain: i32,
    type_: i32,
    protocol: i32,
    rsv: UserPtr<[i32; 2]>,
) -> Result<(), Errno> {
    let ty = type_ & !(SOCK_CLOEXEC | SOCK_CLOFORK | SOCK_NONBLOCK);
    let fdflags = fdflags_of(type_);
    let nonblock = type_ & SOCK_NONBLOCK != 0;
    let fflag = (FREAD | FWRITE | if nonblock { FNONBLOCK } else { 0 }) as u32;

    let so1 = socreate(p, domain, ty, protocol)?;
    let so2 = match socreate(p, domain, ty, protocol) {
        Ok(so2) => so2,
        Err(error) => {
            // free1:
            let _ = soclose(&so1, 0);
            return Err(error);
        }
    };

    if let Err(error) = soconnect2(&so1, &so2).and_then(|()| {
        // Datagram socket connection is asymmetric.
        if type_ & SOCK_TYPE_MASK == SOCK_DGRAM {
            soconnect2(&so2, &so1)
        } else {
            Ok(())
        }
    }) {
        // free2, free1:
        let _ = soclose(&so2, 0);
        let _ = soclose(&so1, 0);
        return Err(error);
    }

    let mut g = fdplock(p.fd());
    let (fp1, sv0) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(error) => {
            // free3, free2, free1:
            fdpunlock(g);
            let _ = soclose(&so2, 0);
            let _ = soclose(&so1, 0);
            return Err(error);
        }
    };
    fp1.f_flag.store(fflag, Ordering::SeqCst);
    fp_install(&fp1, so1);
    let (fp2, sv1) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(error) => {
            // free4, free3: the first file closes its socket, the second socket is closed.
            let _ = fdremove(&mut g, sv0);
            fdpunlock(g);
            let _ = closef(fp1, Some(p));
            let _ = soclose(&so2, 0);
            return Err(error);
        }
    };
    fp2.f_flag.store(fflag, Ordering::SeqCst);
    fp_install(&fp2, so2);
    match rsv.copyout(&[sv0, sv1]) {
        Ok(()) => {
            fdinsert(&mut g, sv0, fdflags, &fp1);
            fdinsert(&mut g, sv1, fdflags, &fp2);
            fdpunlock(g);
            // KTRACE: not configured.
            let _ = fp1.frele(Some(p));
            let _ = fp2.frele(Some(p));
            Ok(())
        }
        Err(error) => {
            let _ = fdremove(&mut g, sv1);
            // free4:
            let _ = fdremove(&mut g, sv0);
            // free3: the files close their sockets.
            fdpunlock(g);
            let _ = closef(fp2, Some(p));
            let _ = closef(fp1, Some(p));
            Err(error)
        }
    }
}

/// `sendto(2)`: sends `len` bytes at `buf` on the socket `s`, to the address of `tolen`
/// bytes at `to` (none for `NULL`); the bytes sent.
pub fn sys_sendto<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    buf: usize,
    len: usize,
    flags: i32,
    to: UserPtr<u8>,
    tolen: u32,
) -> Result<usize, Errno> {
    let msg = Msghdr {
        msg_name: to.addr(),
        msg_namelen: tolen,
        msg_iovlen: 1,
        ..Msghdr::default()
    };
    let mut aiov = [Iovec {
        iov_base: ptr::without_provenance_mut(buf),
        iov_len: len,
    }];
    sendit(p, s, &msg, &mut aiov, flags)
}

/// `sendmsg(2)`: sends the message whose `struct msghdr` is at `msg` on the socket `s`; the
/// bytes sent. `EMSGSIZE` for more than `IOV_MAX` iovecs.
pub fn sys_sendmsg<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    msg: UserPtr<Msghdr>,
    flags: i32,
) -> Result<usize, Errno> {
    let mut m = msg.copyin()?;
    // KTRACE: not configured.

    if m.msg_iovlen as usize > IOV_MAX {
        return Err(Errno::EMSGSIZE);
    }
    let mut buf = IovBuf::new();
    let iov = buf.copyin(UserPtr::new(m.msg_iov), m.msg_iovlen as usize)?;
    // KTRACE: not configured.
    m.msg_flags = 0;
    sendit(p, s, &m, iov, flags)
}

/// `sendmmsg(2)`: sends the `vlen` messages (at most 1024) whose `struct mmsghdr`s are at
/// `mmsg` on the socket `s`, each one's byte count to its `msg_len`; the count sent. An
/// error is returned only when no message was sent.
pub fn sys_sendmmsg<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    mmsg: UserPtr<Mmsghdr>,
    vlen: u32,
    flags: i32,
) -> Result<u32, Errno> {
    let mut buf = IovBuf::new();
    let mut error = Ok(());

    // Arbitrarily capped at 1024 datagrams.
    let vlen = vlen.min(MMSG_MAX);

    let mut dgrams: u32 = 0;
    while dgrams < vlen {
        if let Err(e) = sendmmsg_one(p, s, mmsg, dgrams, &mut buf, flags) {
            error = Err(e);
            break;
        }
        dgrams += 1;
    }

    if error.is_err() && dgrams > 0 {
        error = Ok(());
    }
    error.map(|()| dgrams)
}

/// `sendmmsg(2)`'s message `i`: copied in, its iovecs into `buf`, sent, and copied back out
/// with its byte count.
fn sendmmsg_one<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    mmsg: UserPtr<Mmsghdr>,
    i: u32,
    buf: &mut IovBuf,
    flags: i32,
) -> Result<(), Errno> {
    let at = mmsg.checked_add(i as usize).ok_or(Errno::EFAULT)?;
    let mut m = at.copyin()?;
    // KTRACE: not configured.

    if m.msg_hdr.msg_iovlen as usize > IOV_MAX {
        return Err(Errno::EMSGSIZE);
    }
    let iov = buf.copyin(
        UserPtr::new(m.msg_hdr.msg_iov),
        m.msg_hdr.msg_iovlen as usize,
    )?;
    // KTRACE: not configured.

    m.msg_hdr.msg_flags = 0;
    let retsnd = sendit(p, s, &m.msg_hdr, iov, flags)?;
    m.msg_len = retsnd as u32;
    at.copyout(&m)
}

/// `sendit(p, s, mp, flags, retsize)`: sends the message `mp`, whose iovecs' kernel copy is
/// `iov`, on the socket `s`; the bytes sent.
pub fn sendit<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    mp: &Msghdr,
    iov: &mut [Iovec],
    flags: i32,
) -> Result<usize, Errno> {
    let fp = getsock(p, s)?;
    let error = sendit_file(p, &fp, mp, iov, flags);
    let _ = fp.frele(Some(p));
    error
}

/// `sendit` on the socket file the call holds.
fn sendit_file<G: ProcGlue>(
    p: &Thread<G>,
    fp: &File<G::FileGlue>,
    mp: &Msghdr,
    iov: &mut [Iovec],
    flags: i32,
) -> Result<usize, Errno> {
    let so = so_of::<G>(fp);
    let mut flags = flags;
    if fp.flag() & FNONBLOCK != 0 {
        flags |= MSG_DONTWAIT;
    }

    G::pledge_sendit(p, mp.msg_name)?;

    // Don't allow sum > SSIZE_MAX
    let resid = iovec_resid(iov)?;
    let mut auio = Uio {
        uio_iov: iov,
        uio_offset: 0, // XXX
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_USERSPACE,
        uio_rw: UioRw::UIO_WRITE,
        uio_procp: Some(p),
    };
    let to = if mp.msg_name != 0 {
        let to = sockargs(
            UserPtr::new(mp.msg_name),
            mp.msg_namelen as usize,
            MT_SONAME,
        )?;
        if isdnssocket(so) {
            dns_portcheck(p, so, to.data(), mp.msg_namelen as usize)?;
        }
        // KTRACE: not configured.
        Some(to)
    } else {
        None
    };
    let control = if mp.msg_control != 0 {
        if (mp.msg_controllen as usize) < cmsg_align(size_of::<Cmsghdr>()) {
            return Err(Errno::EINVAL);
        }
        // KTRACE: not configured.
        Some(sockargs(
            UserPtr::new(mp.msg_control),
            mp.msg_controllen as usize,
            MT_CONTROL,
        )?)
    } else {
        None
    };
    // KTRACE: not configured.
    let len = auio.uio_resid;
    let mut error = sosend(so, to.as_deref(), Some(&mut auio), None, control, flags);
    if let Err(e) = error {
        if auio.uio_resid != len
            && (e == Errno::ERESTART || e == Errno::EINTR || e == Errno::EWOULDBLOCK)
        {
            error = Ok(());
        }
        if error == Err(Errno::EPIPE) && flags & MSG_NOSIGNAL == 0 {
            <G::FileGlue as FileGlue>::sigpipe(p);
        }
    }
    error?;
    let retsize = len - auio.uio_resid;
    {
        let mut st = fp.stats();
        st.f_wxfer += 1;
        st.f_wbytes += retsize as u64;
    }
    // KTRACE: not configured.
    Ok(retsize)
}

/// `recvfrom(2)`: receives at most `len` bytes into `buf` from the socket `s`, the sender's
/// address to `from` (when not `NULL`), its length to `fromlenaddr` (which held the room at
/// `from`); the bytes received.
pub fn sys_recvfrom<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    buf: usize,
    len: usize,
    flags: i32,
    from: UserPtr<u8>,
    fromlenaddr: UserPtr<u32>,
) -> Result<usize, Errno> {
    let namelen = if fromlenaddr.is_null() {
        0
    } else {
        fromlenaddr.copyin()?
    };
    let mut msg = Msghdr {
        msg_name: from.addr(),
        msg_namelen: namelen,
        msg_iovlen: 1,
        msg_flags: flags,
        ..Msghdr::default()
    };
    let mut aiov = [Iovec {
        iov_base: ptr::without_provenance_mut(buf),
        iov_len: len,
    }];
    recvit(p, s, &mut msg, &mut aiov, fromlenaddr)
}

/// `recvmsg(2)`: receives a message on the socket `s` as the `struct msghdr` at `msg` asks,
/// and copies the structure back out with the address's and the control data's lengths and
/// the flags; the bytes received. `EMSGSIZE` for more than `IOV_MAX` iovecs.
pub fn sys_recvmsg<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    msg: UserPtr<Msghdr>,
    flags: i32,
) -> Result<usize, Errno> {
    let mut m = msg.copyin()?;

    if m.msg_iovlen as usize > IOV_MAX {
        return Err(Errno::EMSGSIZE);
    }
    let mut buf = IovBuf::new();
    m.msg_flags = flags;
    let iov = buf.copyin(UserPtr::new(m.msg_iov), m.msg_iovlen as usize)?;
    let n = recvit(p, s, &mut m, iov, UserPtr::new(0))?;
    // KTRACE: not configured.
    msg.copyout(&m)?;
    Ok(n)
}

/// `recvmmsg(2)`: receives at most `vlen` messages (1024) on the socket `s` as the `struct
/// mmsghdr`s at `mmsg` ask, until `timeout` (when not `NULL`) has passed, a message brings
/// out-of-band data, or (`MSG_WAITFORONE`) none is left once one came; the count received.
/// An error is returned only when no message was received; after one, the socket keeps it
/// for the next call (`so_error`).
pub fn sys_recvmmsg<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    mmsg: UserPtr<Mmsghdr>,
    vlen: u32,
    flags: i32,
    timeout: UserPtr<Timespec>,
) -> Result<u32, Errno> {
    let deadline = if timeout.is_null() {
        None
    } else {
        let ts = timeout.copyin()?;
        // KTRACE: not configured.
        if !ts.is_valid() {
            return Err(Errno::EINVAL);
        }
        Some(crate::tc::getnanotime().add(ts))
    };

    let mut flags = flags;
    let mut buf = IovBuf::new();
    let mut error = Ok(());

    // Arbitrarily capped at 1024 datagrams.
    let vlen = vlen.min(MMSG_MAX);

    let mut dgrams: u32 = 0;
    while dgrams < vlen {
        let oob = match recvmmsg_one(p, s, mmsg, dgrams, &mut buf, flags) {
            Ok(oob) => oob,
            Err(e) => {
                error = if e == Errno::EAGAIN && dgrams > 0 {
                    Ok(())
                } else {
                    Err(e)
                };
                break;
            }
        };
        if flags & MSG_WAITFORONE != 0 {
            flags |= MSG_DONTWAIT;
        }

        dgrams += 1;
        if oob {
            break;
        }

        if let Some(ts) = deadline
            && crate::tc::getnanotime().sub(ts).tv_sec > 0
        {
            break;
        }
    }

    // If we succeeded at least once, return 0, hopefully so->so_error will catch it next
    // time.
    if let Err(e) = error
        && dgrams > 0
    {
        if let Ok(fp) = getsock(p, s) {
            so_of::<G>(&fp).set_error(Some(e));
            let _ = fp.frele(Some(p));
        }
        error = Ok(());
    }

    error.map(|()| dgrams)
}

/// `recvmmsg(2)`'s message `i`: copied in, its iovecs into `buf`, received with `flags`
/// (`MSG_WAITFORONE` taken off), and copied back out with its byte count; whether it brought
/// out-of-band data (`MSG_OOB` in its flags). The copy-out error is the C's after the
/// receive: the message is consumed, the call stops.
fn recvmmsg_one<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    mmsg: UserPtr<Mmsghdr>,
    i: u32,
    buf: &mut IovBuf,
    flags: i32,
) -> Result<bool, Errno> {
    let at = mmsg.checked_add(i as usize).ok_or(Errno::EFAULT)?;
    let mut m = at.copyin()?;

    if m.msg_hdr.msg_iovlen as usize > IOV_MAX {
        return Err(Errno::EMSGSIZE);
    }
    let iov = buf.copyin(
        UserPtr::new(m.msg_hdr.msg_iov),
        m.msg_hdr.msg_iovlen as usize,
    )?;

    m.msg_hdr.msg_flags = flags & !MSG_WAITFORONE;
    let retrec = recvit(p, s, &mut m.msg_hdr, iov, UserPtr::new(0))?;
    m.msg_len = retrec as u32;
    // KTRACE: not configured.
    at.copyout(&m)?;
    Ok(m.msg_hdr.msg_flags & MSG_OOB != 0)
}

/// `recvit(p, s, mp, namelenp, retsize)`: receives a message on the socket `s` into `mp`,
/// whose iovecs' kernel copy is `iov`: the sender's address to `mp.msg_name` (its length to
/// `mp.msg_namelen`, and to `namelenp` when not `NULL`), the control data to
/// `mp.msg_control` (its length to `mp.msg_controllen`, `MSG_CTRUNC` in `mp.msg_flags` when
/// cut); the bytes received.
pub fn recvit<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    mp: &mut Msghdr,
    iov: &mut [Iovec],
    namelenp: UserPtr<u32>,
) -> Result<usize, Errno> {
    let fp = getsock(p, s)?;
    let error = recvit_file(p, &fp, mp, iov, namelenp);
    let _ = fp.frele(Some(p));
    error
}

/// `recvit` on the socket file the call holds.
fn recvit_file<G: ProcGlue>(
    p: &Thread<G>,
    fp: &File<G::FileGlue>,
    mp: &mut Msghdr,
    iov: &mut [Iovec],
    namelenp: UserPtr<u32>,
) -> Result<usize, Errno> {
    // Don't allow sum > SSIZE_MAX
    let resid = iovec_resid(iov)?;
    let mut auio = Uio {
        uio_iov: iov,
        uio_offset: 0, // XXX
        uio_resid: resid,
        uio_segflg: UioSeg::UIO_USERSPACE,
        uio_rw: UioRw::UIO_READ,
        uio_procp: Some(p),
    };
    // KTRACE: not configured.
    let len = auio.uio_resid;
    if fp.flag() & FNONBLOCK != 0 {
        mp.msg_flags |= MSG_DONTWAIT;
    }
    let want_control = mp.msg_control != 0;
    let mut from: Option<MbufBox> = None;
    let mut control: Option<MbufBox> = None;
    let mut error = soreceive(
        so_of::<G>(fp),
        Some(&mut from),
        &mut auio,
        None,
        want_control.then_some(&mut control),
        Some(&mut mp.msg_flags),
        if want_control { mp.msg_controllen } else { 0 },
    );
    if let Err(e) = error
        && auio.uio_resid != len
        && (e == Errno::ERESTART || e == Errno::EINTR || e == Errno::EWOULDBLOCK)
    {
        error = Ok(());
    }
    // KTRACE: not configured.
    error?;
    let retsize = len - auio.uio_resid;
    if mp.msg_name != 0 {
        let alen = match &from {
            None => 0,
            Some(f) => {
                let alen = f.m_len().get();
                let n = alen.min(mp.msg_namelen) as usize;
                UserPtr::<u8>::new(mp.msg_name).copyout_slice(&f.data()[..n])?;
                // KTRACE: not configured.
                alen
            }
        };
        mp.msg_namelen = alen;
        if !namelenp.is_null() {
            namelenp.copyout(&alen)?;
        }
    }
    let mut error = Ok(());
    if want_control {
        let mut clen = mp.msg_controllen as usize;
        match control.as_deref() {
            Some(first) if clen > 0 => {
                error = copyout_control(first, mp, &mut clen);
            }
            _ => clen = 0,
        }
        mp.msg_controllen = clen as u32;
    }
    if error.is_ok() {
        let mut st = fp.stats();
        st.f_rxfer += 1;
        st.f_rbytes += retsize as u64;
    }
    error.map(|()| retsize)
}

/// `recvit`'s copy of the control chain `first` out to `mp.msg_control`, at most `*clen`
/// bytes (the room), each mbuf's data after the last one's `ALIGN`ed end; `MSG_CTRUNC` in
/// `mp.msg_flags` when cut. `*clen` becomes the bytes the chain took (the room it went past
/// included), the copy's error returned after it.
fn copyout_control(first: &Mbuf, mp: &mut Msghdr, clen: &mut usize) -> Result<(), Errno> {
    let base = mp.msg_control;
    let mut cp = base;
    let mut m = first;
    let error = loop {
        let mut i = m.m_len().get() as usize;
        if *clen < i {
            mp.msg_flags |= MSG_CTRUNC;
            i = *clen;
        }
        let error = UserPtr::<u8>::new(cp).copyout_slice(&m.data()[..i]);
        // KTRACE: not configured.
        let next = m.m_next().get();
        if next.is_some() {
            i = cmsg_align(i);
            if *clen < i {
                mp.msg_flags |= MSG_CTRUNC;
                i = *clen;
            }
        }
        cp = cp.wrapping_add(i);
        *clen -= i;
        if error.is_err() || *clen == 0 {
            break error;
        }
        match next {
            Some(n) => m = n,
            None => break error,
        }
    };
    *clen = cp.wrapping_sub(base);
    error
}

/// `shutdown(2)`: shuts the socket `s` down for receiving (`SHUT_RD`), sending (`SHUT_WR`) or
/// both (`SHUT_RDWR`).
pub fn sys_shutdown<G: ProcGlue>(p: &Thread<G>, s: i32, how: i32) -> Result<(), Errno> {
    let fp = getsock(p, s)?;
    let error = soshutdown(so_of::<G>(&fp), how);
    let _ = fp.frele(Some(p));
    error
}

/// `setsockopt(2)`: sets the option `name` of `level` on the socket `s` to the `valsize`
/// bytes at `val` (none for `NULL`); `EINVAL` past `MCLBYTES`.
pub fn sys_setsockopt<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    level: i32,
    name: i32,
    val: UserPtr<u8>,
    valsize: u32,
) -> Result<(), Errno> {
    let fp = getsock(p, s)?;
    let error = setsockopt_file(p, so_of::<G>(&fp), level, name, val, valsize);
    let _ = fp.frele(Some(p));
    error
}

/// `setsockopt(2)` on the socket of the file the call holds.
fn setsockopt_file<G: ProcGlue>(
    p: &Thread<G>,
    so: &Socket<G>,
    level: i32,
    name: i32,
    val: UserPtr<u8>,
    valsize: u32,
) -> Result<(), Errno> {
    G::pledge_sockopt(p, true, so.proto(), level, name)?;
    let valsize = valsize as usize;
    if valsize > MCLBYTES {
        return Err(Errno::EINVAL);
    }
    let mut m = if val.is_null() {
        None
    } else {
        Some(mbuf_args(valsize, MT_SOOPTS, |d| val.copyin_slice(d))?)
    };
    sosetopt(so, p, level, name, m.as_deref_mut())
}

/// `getsockopt(2)`: reads the option `name` of `level` of the socket `s` into `val`, at most
/// the `socklen_t` at `avalsize` bytes of it, and writes the length back there (nothing for a
/// `NULL` `val` or a zero length).
pub fn sys_getsockopt<G: ProcGlue>(
    p: &Thread<G>,
    s: i32,
    level: i32,
    name: i32,
    val: UserPtr<u8>,
    avalsize: UserPtr<u32>,
) -> Result<(), Errno> {
    let fp = getsock(p, s)?;
    let error = getsockopt_file(p, so_of::<G>(&fp), level, name, val, avalsize);
    let _ = fp.frele(Some(p));
    error
}

/// `getsockopt(2)` on the socket of the file the call holds.
fn getsockopt_file<G: ProcGlue>(
    p: &Thread<G>,
    so: &Socket<G>,
    level: i32,
    name: i32,
    val: UserPtr<u8>,
    avalsize: UserPtr<u32>,
) -> Result<(), Errno> {
    G::pledge_sockopt(p, false, so.proto(), level, name)?;
    let mut valsize = if val.is_null() { 0 } else { avalsize.copyin()? };
    let Some(mut m) = m_get(M_WAIT, MT_SOOPTS) else {
        return Err(Errno::ENOBUFS);
    };
    sogetopt(so, level, name, &mut m)?;
    if !val.is_null() && valsize != 0 {
        valsize = valsize.min(m.m_len().get());
        val.copyout_slice(&m.data()[..valsize as usize])?;
        avalsize.copyout(&valsize)?;
    }
    Ok(())
}

/// `getsockname(2)`: the socket `fdes`'s own address to `asa`, at most the `socklen_t` at
/// `alen` bytes of it, its whole length back there.
pub fn sys_getsockname<G: ProcGlue>(
    p: &Thread<G>,
    fdes: i32,
    asa: UserPtr<u8>,
    alen: UserPtr<u32>,
) -> Result<(), Errno> {
    let fp = getsock(p, fdes)?;
    let error = sockname(so_of::<G>(&fp), false, asa, alen);
    let _ = fp.frele(Some(p));
    error
}

/// `getpeername(2)`: the connected socket `fdes`'s peer's address to `asa`, as
/// `getsockname(2)`; `ENOTCONN` for a socket not connected.
pub fn sys_getpeername<G: ProcGlue>(
    p: &Thread<G>,
    fdes: i32,
    asa: UserPtr<u8>,
    alen: UserPtr<u32>,
) -> Result<(), Errno> {
    let fp = getsock(p, fdes)?;
    let error = sockname(so_of::<G>(&fp), true, asa, alen);
    let _ = fp.frele(Some(p));
    error
}

/// `getsockname(2)` (`pru_sockaddr`) or, `peer`, `getpeername(2)` (`pru_peeraddr`) on the
/// socket of the file the call holds.
fn sockname<G: ProcGlue>(
    so: &Socket<G>,
    peer: bool,
    asa: UserPtr<u8>,
    alen: UserPtr<u32>,
) -> Result<(), Errno> {
    if so.has_state(SS_YP) {
        return Err(Errno::ENOTSOCK);
    }
    if peer && !so.has_state(SS_ISCONNECTED) {
        return Err(Errno::ENOTCONN);
    }
    let len = alen.copyin()?;
    let Some(mut m) = m_getclr(M_WAIT, MT_SONAME) else {
        return Err(Errno::ENOBUFS);
    };
    let mut w = solock_shared(so);
    let error = if peer {
        so.usrreqs().peeraddr(so, &mut w, &mut m)
    } else {
        so.usrreqs().sockaddr(so, &mut w, &mut m)
    };
    drop(w);
    error?;
    copyaddrout(&m, asa, len, alen)
}

/// `sockargs(mp, buf, buflen, type)`: the `buflen` bytes at `buf` in an mbuf of `type_`: a
/// socket address (`MT_SONAME`, at most `UCHAR_MAX` bytes and at least its length and family,
/// `sa_len` set to `buflen`) or control data (at most `MCLBYTES`); `EINVAL` past those.
pub fn sockargs(buf: UserPtr<u8>, buflen: usize, type_: i32) -> Result<MbufBox, Errno> {
    mbuf_args(buflen, type_, |d| buf.copyin_slice(d))
}

/// `sockargs`'s and `ypsockargs`'s body: the bounds, the mbuf (a cluster past `MLEN`), `fill`
/// for its `buflen` bytes, `sa_len` of a name.
fn mbuf_args(
    buflen: usize,
    type_: i32,
    fill: impl FnOnce(&mut [u8]) -> Result<(), Errno>,
) -> Result<MbufBox, Errno> {
    // We can't allow socket names > UCHAR_MAX in length, since that will overflow sa_len.
    // Also, control data more than MCLBYTES in length is just too much. Memory for sa_len and
    // sa_family must exist.
    let max = if type_ == MT_SONAME {
        usize::from(UCHAR_MAX)
    } else {
        MCLBYTES
    };
    if buflen > max || (type_ == MT_SONAME && buflen < SA_DATA_OFFSET) {
        return Err(Errno::EINVAL);
    }

    // Allocate an mbuf to hold the arguments.
    let Some(mut m) = m_get(M_WAIT, type_) else {
        return Err(Errno::ENOBUFS);
    };
    if buflen > MLEN {
        mclget(&mut m, M_WAIT);
        if m.m_flags().get() & M_EXT == 0 {
            return Err(Errno::ENOBUFS);
        }
    }
    m.m_len().set(buflen as u32);
    let Some(data) = m.data_mut() else {
        kpanic(format_args!("sockargs: a new mbuf is read-only"));
    };
    fill(data)?;
    if type_ == MT_SONAME {
        data[0] = buflen as u8;
    }
    Ok(m)
}

/// `getsock(p, fdes, fpp)`: the socket file open at `fdes`, with a reference of the
/// caller's; `EBADF` for no file, `ENOTSOCK` for a file of another type.
pub fn getsock<G: ProcGlue>(p: &Thread<G>, fdes: i32) -> Result<FileRef<G::FileGlue>, Errno> {
    let Some(fp) = fd_getfile(p.fd(), fdes) else {
        return Err(Errno::EBADF);
    };
    if fp.f_type() != DTYPE_SOCKET {
        let _ = fp.frele(Some(p));
        return Err(Errno::ENOTSOCK);
    }
    Ok(fp)
}

/// `copyaddrout(p, name, sa, buflen, outlen)`: the address in `name` out to `sa`, at most
/// `buflen` bytes of it, and its whole length to `outlen`.
pub fn copyaddrout(
    name: &Mbuf,
    sa: UserPtr<u8>,
    buflen: u32,
    outlen: UserPtr<u32>,
) -> Result<(), Errno> {
    let namelen = name.m_len().get();

    // SHOULD COPY OUT A CHAIN HERE
    sa.copyout_slice(&name.data()[..buflen.min(namelen) as usize])?;
    // KTRACE: not configured.
    outlen.copyout(&namelen)
}

/// `ypsockargs(mp, buf, buflen, type)`: `sockargs` of the kernel bytes `buf`.
fn ypsockargs(buf: &[u8], type_: i32) -> Result<MbufBox, Errno> {
    mbuf_args(buf.len(), type_, |d| {
        d.copy_from_slice(buf);
        Ok(())
    })
}

/// `ypconnect(2)`: a socket of `type_` (`SOCK_STREAM` or `SOCK_DGRAM`) connected to the YP
/// server `ypbind(8)` found for this domain, at a new close-on-exec, non-blocking
/// descriptor, returned. `EAFNOSUPPORT` without a domain name or for another type, `EACCES`
/// in a chroot, `EFTYPE` for a binding file that is not one, `EOWNERDEAD` for one no process
/// holds a lock on (left by a `ypbind` gone), `EPERM` for a server port that is not
/// privileged (or is 20).
pub fn sys_ypconnect<G: ProcGlue>(p: &Thread<G>, type_: i32) -> Result<i32, Errno> {
    // `domainname`, the string up to its NUL, copied under its lock (the C reads it without
    // one).
    let mut domain = [0u8; MAXHOSTNAMELEN];
    let dlen = with_domainname(|d| {
        let n = d
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(d.len())
            .min(MAXHOSTNAMELEN);
        domain[..n].copy_from_slice(&d[..n]);
        n
    });
    let domain = &domain[..dlen];
    if domain.is_empty() || domain.contains(&b'/') {
        return Err(Errno::EAFNOSUPPORT);
    }

    match type_ {
        SOCK_STREAM | SOCK_DGRAM => {}
        _ => return Err(Errno::EAFNOSUPPORT),
    }

    if p.process().ps_flags.load(Ordering::Relaxed) & PS_CHROOT != 0 {
        return Err(Errno::EACCES);
    }
    hal::klock::kernel_lock();
    let data = ypbinding_read(p, domain);
    hal::klock::kernel_unlock();
    let data = data?;

    let port_at = if type_ == SOCK_STREAM {
        YPB_TCP_PORT
    } else {
        YPB_UDP_PORT
    };
    let port = [data[port_at], data[port_at + 1]];
    let hport = u16::from_be_bytes(port);
    if hport >= IPPORT_RESERVED || hport == 20 {
        return Err(Errno::EPERM);
    }
    // struct sockaddr_in: its length, AF_INET, the port and the address as the binding
    // holds them (network order), zero.
    let mut ypsin = [0u8; SOCKADDR_IN_SIZE];
    ypsin[0] = SOCKADDR_IN_SIZE as u8;
    ypsin[1] = AF_INET;
    ypsin[SIN_PORT_OFFSET..SIN_PORT_OFFSET + 2].copy_from_slice(&port);
    ypsin[SIN_ADDR_OFFSET..SIN_ADDR_OFFSET + 4].copy_from_slice(&data[YPB_IN..YPB_IN + 4]);

    let so = socreate(p, i32::from(AF_INET), type_, 0)?;

    let nam = match ypsockargs(&ypsin, MT_SONAME) {
        Ok(nam) => nam,
        Err(error) => {
            let _ = soclose(&so, MSG_DONTWAIT);
            return Err(error);
        }
    };

    // KTRACE: not configured.
    let mut w = solock(&so);

    // Secure YP maps require reserved ports
    let mut error = if suser(p).is_ok() {
        ypconnect_lowport(&so, &mut w)
    } else {
        Ok(())
    };

    if error.is_ok() {
        error = soconnect(&so, &mut w, &nam);
        while so.has_state(SS_ISCONNECTING) && so.error().is_none() {
            error = sosleep_nsec(&mut w, so.timeo_chan(), PSOCK | PCATCH, "ypcon", INFSLP);
            if error.is_err() {
                break;
            }
        }
    }
    drop(nam);
    so.st_set(SS_YP); // impose some restrictions
    drop(w);
    if let Err(error) = error {
        let _ = soclose(&so, MSG_DONTWAIT);
        return Err(error);
    }

    let mut g = fdplock(p.fd());
    let (fp, fd) = match falloc(p, &mut g) {
        Ok(r) => r,
        Err(error) => {
            fdpunlock(g);
            let _ = soclose(&so, MSG_DONTWAIT);
            return Err(error);
        }
    };

    fp.f_flag
        .store((FREAD | FWRITE | FNONBLOCK) as u32, Ordering::SeqCst);
    fp_install(&fp, so);
    fdinsert(&mut g, fd, UF_EXCLOSE, &fp);
    fdpunlock(g);
    let _ = fp.frele(Some(p));
    Ok(fd)
}

/// `ypconnect(2)`'s binding file, `/var/yp/binding/<domain>.2`, read whole, the kernel lock
/// held: it must be a regular file of `sizeof(struct ypbinding)` bytes that a process holds
/// a write lock on (`ypbind(8)`).
fn ypbinding_read<G: ProcGlue>(
    p: &Thread<G>,
    domain: &[u8],
) -> Result<[u8; YPBINDING_SIZE], Errno> {
    let Some(mut name) = pathbuf_get::<G>() else {
        return Err(Errno::ENOMEM);
    };
    // snprintf(name, MAXPATHLEN, "/var/yp/binding/%s.2", domainname)
    let mut n = 0;
    for &c in b"/var/yp/binding/".iter().chain(domain).chain(b".2") {
        if n + 1 >= MAXPATHLEN {
            break;
        }
        name[n] = c;
        n += 1;
    }
    let mut nid = ndinit(
        LOOKUP,
        NOFOLLOW | LOCKLEAF | KERNELPATH,
        NiDirp::Sys(&name[..n]),
        p,
    );
    nid.ni_pledge = PLEDGE_RPATH;

    namei(&mut nid)?;
    let Some(vp) = nid.ni_vp else {
        kpanic(format_args!("sys_ypconnect: no vnode from namei"));
    };
    let error = ypbinding_check(p, vp);
    vput(vp);
    error
}

/// The checks and the read of the binding file `vp` (locked).
fn ypbinding_check<G: ProcGlue>(
    p: &Thread<G>,
    vp: &'static Vnode<G>,
) -> Result<[u8; YPBINDING_SIZE], Errno> {
    let mut va = Vattr::new();
    vop_getattr(vp, &mut va, Cred::real(p.ucred()), p)?;
    if vp.vtype() != VREG || va.va_size != YPBINDING_SIZE as u64 {
        return Err(Errno::EFTYPE);
    }

    // Check that a lock is held on the file (hopefully by ypbind), otherwise the file might
    // be old
    let mut fl = Flock {
        l_start: 0,
        l_len: 0,
        l_pid: 0,
        l_type: F_WRLCK,
        l_whence: SEEK_SET as i16,
    };
    vop_advlock(
        vp,
        LockOwner::Table(p.fd().addr()),
        F_GETLK,
        &mut fl,
        F_POSIX,
    )?;
    if fl.l_type == F_UNLCK {
        return Err(Errno::EOWNERDEAD);
    }

    let mut data = [0u8; YPBINDING_SIZE];
    let mut iov = [Iovec::new()];
    let mut uio = Uio::kernel_read(&mut iov, &mut data, Some(p));
    vop_read(vp, &mut uio, 0, Cred::real(p.ucred()))?;
    Ok(data)
}

/// `ypconnect(2)`'s `sotoinpcb(so)->inp_flags |= INP_LOWPORT`: the internet protocol's own
/// `IP_PORTRANGE_LOW` (`ip_ctloutput`), the socket locked.
fn ypconnect_lowport<G: ProcGlue>(so: &Socket<G>, w: &mut SoLocked<'_>) -> Result<(), Errno> {
    let mut m = mbuf_args(size_of::<i32>(), MT_SOOPTS, |d| {
        d.copy_from_slice(&IP_PORTRANGE_LOW.to_ne_bytes());
        Ok(())
    })?;
    so.proto()
        .ctloutput(PRCO_SETOPT, so, w, IPPROTO_IP, IP_PORTRANGE, Some(&mut m))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    //! The socket system calls over the UNIX domain of the test glue (`net::unix::tests`'
    //! setup: a test thread with its descriptor table and root, the UNIX domain alone), with
    //! the user's structures in the user-copy double: the descriptors and their flags, the
    //! C's order of checks and errors, `struct msghdr`'s holes kept, descriptor passing, the
    //! `mmsg` counts and the error kept for the next call, the names, the options, the
    //! bounds of `sockargs`, `dns_portcheck`'s port test and the control chain's copy.

    use std::vec::Vec;
    use std::{assert, assert_eq, format, vec};

    use hal::sys::socket::{
        AF_UNIX, MSG_PEEK, SCM_RIGHTS, SHUT_RDWR, SHUT_WR, SO_SNDBUF, SO_TYPE, SOCK_SEQPACKET,
        SOL_SOCKET, cmsg_len, cmsg_space,
    };
    use hal::user::{AbiPod, bytes_of, pod_from_bytes};

    use super::*;
    use crate::descrip::{fd_getfile_flags, sys_close};
    use crate::file::DTYPE_VNODE;
    use crate::net::domain::net;
    use crate::net::unix::tests::{G, file, setup, teardown};
    use crate::testutil::{USER_BASE, USER_LEN, file_log, set_pledge_net, user_bytes};

    /// The tests' user memory: 64 KiB of the double's range of the memory file system's
    /// image, which the tests holding `MEMORY` (`setup`'s guard, mfs's and the UNIX domain's
    /// tests too) take in turn; the faults only read past the end of the memory, or are refused
    /// whole before a byte is written.
    const U: usize = USER_BASE + 0x1_0000;
    /// Where a `struct msghdr` (or an array of `struct mmsghdr`) goes.
    const UMSG: usize = U;
    /// Where the iovecs go.
    const UIOV: usize = U + 0x400;
    /// A name's room and its length.
    const UNAME: usize = U + 0x800;
    /// A `socklen_t`.
    const ULEN: usize = U + 0x900;
    /// The control data.
    const UCTL: usize = U + 0x1000;
    /// The data sent.
    const USEND: usize = U + 0x2000;
    /// The data received.
    const URECV: usize = U + 0x4000;
    /// `socketpair(2)`'s two descriptors.
    const URSV: usize = U + 0x7000;

    /// `v`'s bytes at the user address `at`.
    fn put<T: AbiPod>(at: usize, v: &T) {
        UserPtr::<T>::new(at).copyout(v).expect("user memory");
    }

    /// The `T` at the user address `at`.
    fn get<T: AbiPod>(at: usize) -> T {
        UserPtr::<T>::new(at).copyin().expect("user memory")
    }

    /// `b` at the user address `at`.
    fn put_bytes(at: usize, b: &[u8]) {
        UserPtr::<u8>::new(at)
            .copyout_slice(b)
            .expect("user memory");
    }

    /// An iovec of `len` bytes at `base`, as its user bytes.
    fn iovec(base: usize, len: usize) -> [u8; Iovec::SIZE] {
        let mut b = [0u8; Iovec::SIZE];
        b[..8].copy_from_slice(&base.to_ne_bytes());
        b[8..].copy_from_slice(&len.to_ne_bytes());
        b
    }

    /// The iovecs `v` (`(base, len)`) at `UIOV`.
    fn put_iovecs(v: &[(usize, usize)]) {
        for (i, &(base, len)) in v.iter().enumerate() {
            put_bytes(UIOV + i * Iovec::SIZE, &iovec(base, len));
        }
    }

    /// A `struct sockaddr_un` of `path`, as the user gives it: its bytes and its length.
    fn sun(path: &[u8]) -> Vec<u8> {
        let len = 2 + path.len() + 1;
        let mut b = vec![0u8; len];
        b[0] = len as u8;
        b[1] = AF_UNIX;
        b[2..2 + path.len()].copy_from_slice(path);
        b
    }

    /// `socketpair(2)` of the UNIX domain: the two descriptors.
    fn pair(p: &Thread<G>, ty: i32) -> (i32, i32) {
        assert_eq!(
            sys_socketpair(p, i32::from(AF_UNIX), ty, 0, UserPtr::new(URSV)),
            Ok(())
        );
        let sv: [i32; 2] = get(URSV);
        (sv[0], sv[1])
    }

    /// The socket open at `fd`, counted, and the file's flags.
    fn sock_at(p: &Thread<G>, fd: i32) -> (SocketRef<G>, i32) {
        let fp = getsock(p, fd).expect("a socket");
        let so = so_of::<G>(&fp).clone();
        let flag = fp.flag();
        let _ = fp.frele(Some(p));
        (so, flag)
    }

    /// The sockets `sockpl` lends now.
    fn sockets() -> u64 {
        net::<G>().sockpl.nout()
    }

    /// Closes the descriptors `fds`.
    fn close(p: &Thread<G>, fds: &[i32]) {
        for &fd in fds {
            assert_eq!(sys_close(p, fd), Ok(()), "close({fd})");
        }
    }

    #[test]
    fn socket_gives_a_descriptor_with_its_flags_after_the_dns_and_pledge_checks() {
        let (_g, p) = setup();
        let before = sockets();
        let _ = file_log();

        let fd = sys_socket(
            p,
            i32::from(AF_UNIX),
            SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK,
            0,
        )
        .expect("a socket");
        assert_eq!(file_log(), vec![format!("pledge_socket {} 0x0", AF_UNIX)]);
        let (fp, flags) = fd_getfile_flags(p.fd(), fd).expect("the file");
        assert_eq!(fp.f_type(), DTYPE_SOCKET);
        assert_eq!(fp.flag(), FREAD | FWRITE | FNONBLOCK);
        assert_eq!(flags, UF_EXCLOSE);
        assert_eq!(so_of::<G>(&fp).so_type(), SOCK_STREAM);
        let _ = fp.frele(Some(p));
        assert_eq!(sockets(), before + 1);

        let fd2 = sys_socket(p, i32::from(AF_UNIX), SOCK_DGRAM | SOCK_CLOFORK, 0).expect("one");
        let (fp, flags) = fd_getfile_flags(p.fd(), fd2).expect("the file");
        assert_eq!((fp.flag(), flags), (FREAD | FWRITE, UF_FORKCLOSE));
        let _ = fp.frele(Some(p));
        let _ = file_log();

        // SOCK_DNS is for the internet domains, refused before pledge is asked.
        assert_eq!(
            sys_socket(p, i32::from(AF_UNIX), SOCK_STREAM | SOCK_DNS, 0),
            Err(Errno::EINVAL)
        );
        assert_eq!(file_log(), Vec::<std::string::String>::new());
        // A pledge refusal comes before the socket is made.
        set_pledge_net(Some(Errno::EPERM));
        assert_eq!(
            sys_socket(p, i32::from(AF_UNIX), SOCK_STREAM, 0),
            Err(Errno::EPERM)
        );
        set_pledge_net(None);
        assert_eq!(sockets(), before + 2);
        // A type the domain has not.
        assert_eq!(
            sys_socket(p, i32::from(AF_UNIX), 99, 0),
            Err(Errno::EPROTONOSUPPORT)
        );
        assert_eq!(sockets(), before + 2);

        close(p, &[fd, fd2]);
        assert_eq!(sockets(), before);
        teardown();
    }

    #[test]
    fn socketpair_sendto_and_recvfrom_move_bytes_and_count_them() {
        let (_g, p) = setup();
        let before = sockets();
        let (a, b) = pair(p, SOCK_STREAM);
        assert!(a >= 0 && b >= 0 && a != b);

        put_bytes(USEND, b"hello");
        assert_eq!(sys_sendto(p, a, USEND, 5, 0, UserPtr::new(0), 0), Ok(5));
        // A stream peer has no address: 0 to fromlenaddr, nothing to from.
        put(ULEN, &64u32);
        put_bytes(UNAME, &[0xee; 4]);
        assert_eq!(
            sys_recvfrom(p, b, URECV, 16, 0, UserPtr::new(UNAME), UserPtr::new(ULEN)),
            Ok(5)
        );
        assert_eq!(user_bytes(URECV, 5).unwrap(), b"hello");
        assert_eq!(get::<u32>(ULEN), 0);
        assert_eq!(user_bytes(UNAME, 4).unwrap(), vec![0xee; 4]);
        // The files counted the transfers.
        for (fd, rx, tx) in [(a, 0, 1), (b, 1, 0)] {
            let fp = getsock(p, fd).expect("a socket");
            let st = fp.stats();
            assert_eq!(
                (st.f_rxfer, st.f_rbytes, st.f_wxfer, st.f_wbytes),
                (rx, rx * 5, tx, tx * 5)
            );
            drop(st);
            let _ = fp.frele(Some(p));
        }

        // A datagram pair: connected both ways, the sender's name comes with each datagram.
        let (c, d) = pair(p, SOCK_DGRAM);
        put_bytes(USEND, b"dg");
        assert_eq!(sys_sendto(p, d, USEND, 2, 0, UserPtr::new(0), 0), Ok(2));
        assert_eq!(
            sys_recvfrom(p, c, URECV, 1, 0, UserPtr::new(0), UserPtr::new(0)),
            Ok(1)
        );
        // A connected UNIX socket refuses an address.
        let name = sun(b"/nowhere");
        put_bytes(UNAME, &name);
        assert_eq!(
            sys_sendto(p, c, USEND, 2, 0, UserPtr::new(UNAME), name.len() as u32),
            Err(Errno::EISCONN)
        );

        // rsv not user memory: EFAULT, and every descriptor and socket of the call gone.
        let at = UserPtr::new(USER_BASE + USER_LEN - 4);
        assert_eq!(
            sys_socketpair(p, i32::from(AF_UNIX), SOCK_STREAM, 0, at),
            Err(Errno::EFAULT)
        );
        assert_eq!(sockets(), before + 4);
        let (e, f) = pair(p, SOCK_SEQPACKET);
        assert_eq!(
            (e, f),
            (d + 1, d + 2),
            "the failed call's descriptors were freed"
        );

        close(p, &[a, b, c, d, e, f]);
        assert_eq!(sockets(), before);
        teardown();
    }

    #[test]
    fn sendto_on_a_shut_socket_signals_sigpipe_unless_nosignal() {
        let (_g, p) = setup();
        let (a, b) = pair(p, SOCK_STREAM);
        assert_eq!(sys_shutdown(p, a, SHUT_WR), Ok(()));
        // The peer reads the end of the stream.
        assert_eq!(
            sys_recvfrom(p, b, URECV, 4, 0, UserPtr::new(0), UserPtr::new(0)),
            Ok(0)
        );
        let _ = file_log();
        put_bytes(USEND, b"x");
        assert_eq!(
            sys_sendto(p, a, USEND, 1, 0, UserPtr::new(0), 0),
            Err(Errno::EPIPE)
        );
        assert_eq!(file_log(), vec!["pledge_sendit 0x0", "SIGPIPE"]);
        assert_eq!(
            sys_sendto(p, a, USEND, 1, MSG_NOSIGNAL, UserPtr::new(0), 0),
            Err(Errno::EPIPE)
        );
        assert_eq!(file_log(), vec!["pledge_sendit 0x0"]);
        assert_eq!(sys_shutdown(p, a, 5), Err(Errno::EINVAL));
        assert_eq!(sys_shutdown(p, b, SHUT_RDWR), Ok(()));
        close(p, &[a, b]);
        teardown();
    }

    #[test]
    fn sendmsg_and_recvmsg_pass_a_descriptor_and_keep_the_holes() {
        let (_g, p) = setup();
        let (a, b) = pair(p, SOCK_STREAM);
        let passed = sys_socket(p, i32::from(AF_UNIX), SOCK_DGRAM, 0).expect("a socket");
        let (passed_so, _) = sock_at(p, passed);

        // Two iovecs, a rights message of one descriptor; the holes hold the user's bytes.
        put_bytes(USEND, b"abcdefgh");
        put_iovecs(&[(USEND, 3), (USEND + 3, 5)]);
        let mut cm = vec![0u8; cmsg_len(4)];
        cm[..4].copy_from_slice(&(cmsg_len(4) as u32).to_ne_bytes());
        cm[4..8].copy_from_slice(&SOL_SOCKET.to_ne_bytes());
        cm[8..12].copy_from_slice(&SCM_RIGHTS.to_ne_bytes());
        cm[16..20].copy_from_slice(&passed.to_ne_bytes());
        put_bytes(UCTL, &cm);
        let msg = Msghdr {
            msg_iov: UIOV,
            msg_iovlen: 2,
            msg_control: UCTL,
            msg_controllen: cm.len() as u32,
            msg_flags: 0x55,
            _pad0: 0xdead_beef,
            _pad1: 0xcafe_babe,
            ..Msghdr::default()
        };
        put(UMSG, &msg);
        assert_eq!(sys_sendmsg(p, a, UserPtr::new(UMSG), 0), Ok(8));
        // sendmsg(2) copies nothing back.
        assert_eq!(get::<Msghdr>(UMSG), msg);
        close(p, &[passed]);

        // recvmsg(2): the bytes, the descriptor, the lengths and the flags written back, the
        // holes as they were.
        put_iovecs(&[(URECV, 8)]);
        put_bytes(UCTL, &[0; 64]);
        let msg = Msghdr {
            msg_name: UNAME,
            msg_namelen: 32,
            msg_iov: UIOV,
            msg_iovlen: 1,
            msg_control: UCTL,
            msg_controllen: cmsg_space(4) as u32,
            msg_flags: 0x55,
            _pad0: 0x1234_5678,
            _pad1: 0x9abc_def0,
        };
        put(UMSG, &msg);
        assert_eq!(sys_recvmsg(p, b, UserPtr::new(UMSG), 0), Ok(8));
        assert_eq!(user_bytes(URECV, 8).unwrap(), b"abcdefgh");
        let back: Msghdr = get(UMSG);
        assert_eq!(
            back,
            Msghdr {
                msg_namelen: 0,
                msg_controllen: cmsg_len(4) as u32,
                msg_flags: 0,
                ..msg
            }
        );
        let got = user_bytes(UCTL, cmsg_len(4)).unwrap();
        assert_eq!(&got[..12], &cm[..12]);
        let newfd = i32::from_ne_bytes(*got[16..20].first_chunk().unwrap());
        let (so, _) = sock_at(p, newfd);
        assert!(
            core::ptr::eq(&*so, &*passed_so),
            "the same socket came back"
        );
        drop((so, passed_so));

        close(p, &[a, b, newfd]);
        teardown();
    }

    #[test]
    fn sendmsg_and_recvmsg_check_in_the_cs_order() {
        let (_g, p) = setup();
        let (a, b) = pair(p, SOCK_DGRAM);
        let base = Msghdr {
            msg_iov: UIOV,
            msg_iovlen: 1,
            ..Msghdr::default()
        };

        // More than IOV_MAX iovecs: EMSGSIZE before the descriptor is looked at.
        put(
            UMSG,
            &Msghdr {
                msg_iovlen: IOV_MAX as u32 + 1,
                ..base
            },
        );
        assert_eq!(
            sys_sendmsg(p, -1, UserPtr::new(UMSG), 0),
            Err(Errno::EMSGSIZE)
        );
        assert_eq!(
            sys_recvmsg(p, -1, UserPtr::new(UMSG), 0),
            Err(Errno::EMSGSIZE)
        );
        // A structure that is not user memory.
        let far = UserPtr::new(USER_BASE + USER_LEN - 8);
        assert_eq!(sys_sendmsg(p, a, far, 0), Err(Errno::EFAULT));
        assert_eq!(sys_recvmsg(p, a, far, 0), Err(Errno::EFAULT));

        // A length past SSIZE_MAX: the descriptor's EBADF first, then pledge, then EINVAL.
        put_iovecs(&[(USEND, usize::MAX / 2 + 1)]);
        put(UMSG, &base);
        assert_eq!(sys_sendmsg(p, 99, UserPtr::new(UMSG), 0), Err(Errno::EBADF));
        assert_eq!(sys_recvmsg(p, 99, UserPtr::new(UMSG), 0), Err(Errno::EBADF));
        let _ = file_log();
        set_pledge_net(Some(Errno::EPERM));
        put(
            UMSG,
            &Msghdr {
                msg_name: UNAME,
                msg_namelen: 3,
                ..base
            },
        );
        assert_eq!(sys_sendmsg(p, a, UserPtr::new(UMSG), 0), Err(Errno::EPERM));
        assert_eq!(file_log(), vec![format!("pledge_sendit {UNAME:#x}")]);
        set_pledge_net(None);
        assert_eq!(sys_sendmsg(p, a, UserPtr::new(UMSG), 0), Err(Errno::EINVAL));
        put(UMSG, &base);
        assert_eq!(sys_recvmsg(p, a, UserPtr::new(UMSG), 0), Err(Errno::EINVAL));

        // A name too short for its family, control data shorter than its header.
        put_iovecs(&[(USEND, 1)]);
        put(
            UMSG,
            &Msghdr {
                msg_name: UNAME,
                msg_namelen: 1,
                ..base
            },
        );
        assert_eq!(sys_sendmsg(p, a, UserPtr::new(UMSG), 0), Err(Errno::EINVAL));
        put(
            UMSG,
            &Msghdr {
                msg_control: UCTL,
                msg_controllen: 8,
                ..base
            },
        );
        assert_eq!(sys_sendmsg(p, a, UserPtr::new(UMSG), 0), Err(Errno::EINVAL));
        // No iovec at all is a message of no bytes.
        put(
            UMSG,
            &Msghdr {
                msg_iovlen: 0,
                ..base
            },
        );
        assert_eq!(sys_sendmsg(p, a, UserPtr::new(UMSG), 0), Ok(0));
        assert_eq!(sys_recvmsg(p, b, UserPtr::new(UMSG), MSG_DONTWAIT), Ok(0));
        let _ = file_log();

        close(p, &[a, b]);
        teardown();
    }

    #[test]
    fn mmsg_calls_count_messages_and_keep_a_late_error_for_the_next_call() {
        let (_g, p) = setup();
        let (a, b) = pair(p, SOCK_DGRAM);
        const MM: usize = size_of::<Mmsghdr>();

        // Three datagrams of three bytes, one iovec each; nine iovecs on the first, past the
        // stack part, so the array grows and is kept for the others.
        put_bytes(USEND, b"aaabbbccc");
        let mut iov = vec![(USEND, 3)];
        iov.extend((0..8).map(|_| (USEND, 0)));
        put_iovecs(&iov);
        for i in 0..3 {
            put_bytes(UIOV + (16 + i) * Iovec::SIZE, &iovec(USEND + 3 * i, 3));
        }
        let m0 = Mmsghdr {
            msg_hdr: Msghdr {
                msg_iov: UIOV,
                msg_iovlen: 9,
                ..Msghdr::default()
            },
            msg_len: 77,
            _pad: 0xabcd,
        };
        put(UMSG, &m0);
        for i in 1..3 {
            let m = Mmsghdr {
                msg_hdr: Msghdr {
                    msg_iov: UIOV + (16 + i) * Iovec::SIZE,
                    msg_iovlen: 1,
                    ..Msghdr::default()
                },
                ..m0
            };
            put(UMSG + i * MM, &m);
        }
        assert_eq!(sys_sendmmsg(p, a, UserPtr::new(UMSG), 3, 0), Ok(3));
        for i in 0..3 {
            let m: Mmsghdr = get(UMSG + i * MM);
            assert_eq!((m.msg_len, m._pad), (3, 0xabcd), "message {i}");
        }
        assert_eq!(sys_sendmmsg(p, a, UserPtr::new(UMSG), 0, 0), Ok(0));

        // recvmmsg(2) of up to five: three, then EAGAIN, which a count hides.
        for i in 0..5 {
            put_bytes(UIOV + (32 + i) * Iovec::SIZE, &iovec(URECV + 4 * i, 4));
            let m = Mmsghdr {
                msg_hdr: Msghdr {
                    msg_iov: UIOV + (32 + i) * Iovec::SIZE,
                    msg_iovlen: 1,
                    ..Msghdr::default()
                },
                ..Mmsghdr::default()
            };
            put(UMSG + 0x200 + i * MM, &m);
        }
        let rmsg = UserPtr::new(UMSG + 0x200);
        put_bytes(URECV, &[0; 16]);
        assert_eq!(
            sys_recvmmsg(p, b, rmsg, 5, MSG_DONTWAIT, UserPtr::new(0)),
            Ok(3)
        );
        assert_eq!(user_bytes(URECV, 12).unwrap(), b"aaa\0bbb\0ccc\0");
        let lens: Vec<u32> = (0..3)
            .map(|i| get::<Mmsghdr>(UMSG + 0x200 + i * MM).msg_len)
            .collect();
        assert_eq!(lens, vec![3, 3, 3]);
        // Nothing left: the first message's EAGAIN is the call's.
        assert_eq!(
            sys_recvmmsg(p, b, rmsg, 5, MSG_DONTWAIT, UserPtr::new(0)),
            Err(Errno::EAGAIN)
        );
        // MSG_WAITFORONE: the first waits (one is there), the others do not.
        put_bytes(USEND, b"xy");
        assert_eq!(sys_sendto(p, a, USEND, 1, 0, UserPtr::new(0), 0), Ok(1));
        assert_eq!(sys_sendto(p, a, USEND + 1, 1, 0, UserPtr::new(0), 0), Ok(1));
        assert_eq!(
            sys_recvmmsg(p, b, rmsg, 5, MSG_WAITFORONE, UserPtr::new(0)),
            Ok(2)
        );
        // A timeout that is not a time.
        put(URSV, &Timespec::new(0, 1_000_000_000));
        assert_eq!(
            sys_recvmmsg(p, b, rmsg, 5, 0, UserPtr::new(URSV)),
            Err(Errno::EINVAL)
        );

        // The second message refused (more than IOV_MAX iovecs): one message received, the
        // refusal left in so_error for the next call, the second datagram still there.
        assert_eq!(sys_sendto(p, a, USEND, 2, 0, UserPtr::new(0), 0), Ok(2));
        assert_eq!(sys_sendto(p, a, USEND, 1, 0, UserPtr::new(0), 0), Ok(1));
        let mut big: Mmsghdr = get(UMSG + 0x200 + MM);
        big.msg_hdr.msg_iovlen = IOV_MAX as u32 + 1;
        put(UMSG + 0x200 + MM, &big);
        assert_eq!(sys_recvmmsg(p, b, rmsg, 2, 0, UserPtr::new(0)), Ok(1));
        let (so, _) = sock_at(p, b);
        assert_eq!(so.error(), Some(Errno::EMSGSIZE));
        drop(so);
        // The datagram first (an atomic protocol's receive with data does not look at
        // so_error), then the error, once.
        assert_eq!(
            sys_recvfrom(p, b, URECV, 4, 0, UserPtr::new(0), UserPtr::new(0)),
            Ok(1)
        );
        assert_eq!(
            sys_recvfrom(p, b, URECV, 4, 0, UserPtr::new(0), UserPtr::new(0)),
            Err(Errno::EMSGSIZE)
        );
        assert_eq!(
            sys_recvfrom(
                p,
                b,
                URECV,
                4,
                MSG_DONTWAIT,
                UserPtr::new(0),
                UserPtr::new(0)
            ),
            Err(Errno::EAGAIN)
        );

        close(p, &[a, b]);
        teardown();
    }

    #[test]
    fn bind_listen_connect_accept_and_the_names() {
        let (_g, p) = setup();
        let before = sockets();
        let l = sys_socket(p, i32::from(AF_UNIX), SOCK_STREAM | SOCK_NONBLOCK, 0).expect("l");
        let name = sun(b"/n62b.sock");
        put_bytes(UNAME, &name);
        let nlen = name.len() as u32;
        let (lso, _) = sock_at(p, l);
        let state = lso.state();
        drop(lso);
        let _ = file_log();
        assert_eq!(sys_bind(p, l, UserPtr::new(UNAME), nlen), Ok(()));
        // pledge_socket with the socket's family and state (SS_PRIV: root's), before namei.
        let log = file_log();
        assert_eq!(log[0], format!("pledge_socket {} {state:#x}", AF_UNIX));
        assert!(log[1].starts_with("pledge_namei"), "{log:?}");
        // accept(2) on a socket that does not listen, then on an empty queue of a
        // non-blocking listener.
        put(ULEN, &64u32);
        assert_eq!(
            sys_accept(p, l, UserPtr::new(UNAME + 0x40), UserPtr::new(ULEN)),
            Err(Errno::EINVAL)
        );
        assert_eq!(sys_listen(p, l, 5), Ok(()));
        assert_eq!(
            sys_accept(p, l, UserPtr::new(UNAME + 0x40), UserPtr::new(ULEN)),
            Err(Errno::EWOULDBLOCK)
        );
        assert_eq!(
            sys_accept4(p, l, UserPtr::new(0), UserPtr::new(0), 0x4000_0000),
            Err(Errno::EINVAL)
        );
        // The failed calls left no descriptor behind.
        let c = sys_socket(p, i32::from(AF_UNIX), SOCK_STREAM, 0).expect("c");
        assert_eq!(c, l + 1);

        // connect(2), then accept4(2): a close-on-exec descriptor, blocking (not the
        // listener's flag), the unbound client's name (sun_noname).
        assert_eq!(sys_connect(p, c, UserPtr::new(UNAME), nlen), Ok(()));
        assert_eq!(
            sys_connect(p, c, UserPtr::new(UNAME), nlen),
            Err(Errno::EISCONN)
        );
        put(ULEN, &64u32);
        let s = sys_accept4(
            p,
            l,
            UserPtr::new(UNAME + 0x40),
            UserPtr::new(ULEN),
            SOCK_CLOEXEC,
        )
        .expect("a connection");
        let (fp, flags) = fd_getfile_flags(p.fd(), s).expect("the file");
        assert_eq!((fp.flag(), flags), (FREAD | FWRITE, UF_EXCLOSE));
        let _ = fp.frele(Some(p));
        assert_eq!(get::<u32>(ULEN), 16);
        assert_eq!(user_bytes(UNAME + 0x40, 2).unwrap(), vec![16, AF_UNIX]);

        // The names: a room shorter than the name gets its start, the length is whole.
        put(ULEN, &5u32);
        put_bytes(UNAME + 0x80, &[0; 32]);
        assert_eq!(
            sys_getsockname(p, l, UserPtr::new(UNAME + 0x80), UserPtr::new(ULEN)),
            Ok(())
        );
        assert_eq!(user_bytes(UNAME + 0x80, 6).unwrap(), b"\x6a\x01/n6\0");
        assert_eq!(get::<u32>(ULEN), 106);
        put(ULEN, &64u32);
        assert_eq!(
            sys_getpeername(p, c, UserPtr::new(UNAME + 0x80), UserPtr::new(ULEN)),
            Ok(())
        );
        assert_eq!(user_bytes(UNAME + 0x82, 10).unwrap(), b"/n62b.sock");
        assert_eq!(
            sys_getpeername(p, l, UserPtr::new(UNAME + 0x80), UserPtr::new(ULEN)),
            Err(Errno::ENOTCONN)
        );
        assert_eq!(
            sys_getsockname(p, l, UserPtr::new(UNAME + 0x80), UserPtr::new(0)),
            Err(Errno::EFAULT)
        );

        // The connection carries bytes.
        put_bytes(USEND, b"ping");
        assert_eq!(sys_sendto(p, c, USEND, 4, 0, UserPtr::new(0), 0), Ok(4));
        assert_eq!(
            sys_recvfrom(p, s, URECV, 8, MSG_PEEK, UserPtr::new(0), UserPtr::new(0)),
            Ok(4)
        );
        assert_eq!(
            sys_recvfrom(p, s, URECV, 8, 0, UserPtr::new(0), UserPtr::new(0)),
            Ok(4)
        );
        assert_eq!(user_bytes(URECV, 4).unwrap(), b"ping");

        close(p, &[c, s, l]);
        assert_eq!(sockets(), before);
        teardown();
    }

    #[test]
    fn options_and_the_descriptor_checks() {
        let (_g, p) = setup();
        let s = sys_socket(p, i32::from(AF_UNIX), SOCK_STREAM, 0).expect("s");
        let (so, _) = sock_at(p, s);
        let proto = (so.proto().domain().family(), so.proto().protocol());

        let _ = file_log();
        put(USEND, &8192i32);
        assert_eq!(
            sys_setsockopt(p, s, SOL_SOCKET, SO_SNDBUF, UserPtr::new(USEND), 4),
            Ok(())
        );
        assert_eq!(
            file_log(),
            vec![format!(
                "pledge_sockopt true {} {} {SOL_SOCKET} {SO_SNDBUF}",
                proto.0, proto.1
            )]
        );
        assert_eq!(so.so_snd.hiwat(), 8192);
        // Past MCLBYTES: EINVAL, after pledge.
        assert_eq!(
            sys_setsockopt(
                p,
                s,
                SOL_SOCKET,
                SO_SNDBUF,
                UserPtr::new(USEND),
                MCLBYTES as u32 + 1
            ),
            Err(Errno::EINVAL)
        );
        set_pledge_net(Some(Errno::EPERM));
        assert_eq!(
            sys_setsockopt(
                p,
                s,
                SOL_SOCKET,
                SO_SNDBUF,
                UserPtr::new(USEND),
                MCLBYTES as u32 + 1
            ),
            Err(Errno::EPERM)
        );
        assert_eq!(
            sys_getsockopt(p, s, SOL_SOCKET, SO_TYPE, UserPtr::new(0), UserPtr::new(0)),
            Err(Errno::EPERM)
        );
        set_pledge_net(None);

        // getsockopt(2): at most the room, the length written back; nothing without val.
        put(ULEN, &8u32);
        put_bytes(URECV, &[0xee; 8]);
        assert_eq!(
            sys_getsockopt(
                p,
                s,
                SOL_SOCKET,
                SO_TYPE,
                UserPtr::new(URECV),
                UserPtr::new(ULEN)
            ),
            Ok(())
        );
        assert_eq!(get::<i32>(URECV), SOCK_STREAM);
        assert_eq!(user_bytes(URECV + 4, 4).unwrap(), vec![0xee; 4]);
        assert_eq!(get::<u32>(ULEN), 4);
        put(ULEN, &2u32);
        assert_eq!(
            sys_getsockopt(
                p,
                s,
                SOL_SOCKET,
                SO_TYPE,
                UserPtr::new(URECV + 8),
                UserPtr::new(ULEN)
            ),
            Ok(())
        );
        assert_eq!(get::<u32>(ULEN), 2);
        put(ULEN, &99u32);
        assert_eq!(
            sys_getsockopt(
                p,
                s,
                SOL_SOCKET,
                SO_TYPE,
                UserPtr::new(0),
                UserPtr::new(ULEN)
            ),
            Ok(())
        );
        assert_eq!(get::<u32>(ULEN), 99);
        let _ = file_log();

        // getsock: EBADF for no file, ENOTSOCK for another kind of file.
        let other = file(p, DTYPE_VNODE, None);
        for fd in [other, 77] {
            let want = if fd == other {
                Errno::ENOTSOCK
            } else {
                Errno::EBADF
            };
            assert_eq!(sys_listen(p, fd, 1), Err(want));
            assert_eq!(sys_shutdown(p, fd, SHUT_WR), Err(want));
            assert_eq!(
                sys_setsockopt(p, fd, SOL_SOCKET, SO_SNDBUF, UserPtr::new(0), 0),
                Err(want)
            );
        }
        assert!(file_log().is_empty(), "no pledge check before getsock");

        // A ypconnect(2) socket refuses the calls that would name it otherwise.
        so.st_set(SS_YP);
        put_bytes(UNAME, &sun(b"/n62b.yp"));
        assert_eq!(
            sys_bind(p, s, UserPtr::new(UNAME), 12),
            Err(Errno::ENOTSOCK)
        );
        assert_eq!(
            sys_connect(p, s, UserPtr::new(UNAME), 12),
            Err(Errno::ENOTSOCK)
        );
        assert_eq!(sys_listen(p, s, 1), Err(Errno::ENOTSOCK));
        put(ULEN, &16u32);
        assert_eq!(
            sys_getsockname(p, s, UserPtr::new(URECV), UserPtr::new(ULEN)),
            Err(Errno::ENOTSOCK)
        );
        assert_eq!(
            sys_getpeername(p, s, UserPtr::new(URECV), UserPtr::new(ULEN)),
            Err(Errno::ENOTSOCK)
        );
        so.st_clear(SS_YP);
        drop(so);

        close(p, &[s, other]);
        teardown();
    }

    #[test]
    fn sockargs_bounds_are_the_cs() {
        let (_g, p) = setup();
        let _ = p;
        put_bytes(USEND, &vec![0x77; MCLBYTES + 1]);
        // (type, length): Ok with the mbuf's length, or the error.
        let table: [(i32, usize, Result<usize, Errno>); 9] = [
            (MT_SONAME, 0, Err(Errno::EINVAL)),
            (MT_SONAME, 1, Err(Errno::EINVAL)),
            (MT_SONAME, 2, Ok(2)),
            (MT_SONAME, 255, Ok(255)),
            (MT_SONAME, 256, Err(Errno::EINVAL)),
            (MT_CONTROL, 0, Ok(0)),
            (MT_CONTROL, MLEN + 1, Ok(MLEN + 1)),
            (MT_CONTROL, MCLBYTES, Ok(MCLBYTES)),
            (MT_CONTROL, MCLBYTES + 1, Err(Errno::EINVAL)),
        ];
        for (ty, len, want) in table {
            let got = sockargs(UserPtr::new(USEND), len, ty);
            match (&got, want) {
                (Ok(m), Ok(n)) => {
                    assert_eq!(m.m_len().get() as usize, n, "{ty} {len}");
                    assert_eq!(m.m_flags().get() & M_EXT != 0, n > MLEN, "{ty} {len}");
                    let d = m.data();
                    if ty == MT_SONAME {
                        // sa_len is the length given.
                        assert_eq!(d[0] as usize, len);
                        assert!(d[1..].iter().all(|&b| b == 0x77), "{len}: {d:x?}");
                    } else {
                        assert!(d.iter().all(|&b| b == 0x77));
                    }
                }
                (Err(e), Err(w)) => assert_eq!(*e, w, "{ty} {len}"),
                _ => std::panic!("{ty} {len}: {:?}", got.as_ref().map(|m| m.m_len().get())),
            }
        }
        // Not user memory.
        assert_eq!(
            sockargs(UserPtr::new(USER_BASE + USER_LEN - 4), 16, MT_SONAME).err(),
            Some(Errno::EFAULT)
        );
        teardown();
    }

    #[test]
    fn dns_port53_is_the_cs() {
        let sin = |port: u16, len: usize| {
            let mut b = vec![0u8; len];
            b[2..4].copy_from_slice(&port.to_be_bytes());
            b
        };
        let (inet, inet6) = (i32::from(AF_INET), i32::from(AF_INET6));
        // (family, the address's length, port, the name length given): port 53 or not.
        let table = [
            (inet, 16, 53, 16, true),
            (inet, 16, 54, 16, false),
            (inet, 16, 53, 15, false),
            (inet6, 28, 53, 28, true),
            (inet6, 28, 53, 16, false),
            (inet6, 28, 5353, 28, false),
            (i32::from(AF_UNIX), 16, 53, 16, false),
        ];
        for (family, len, port, namelen, want) in table {
            assert_eq!(
                dns_port53(family, &sin(port, len), namelen),
                want,
                "{family} {len} {port} {namelen}"
            );
        }
    }

    #[test]
    fn copyout_control_aligns_and_cuts_as_the_c() {
        let (_g, p) = setup();
        let _ = p;
        // A chain of 12 and 20 bytes.
        let mut m = m_get(M_WAIT, MT_CONTROL).expect("an mbuf");
        m.m_len().set(12);
        m.data_mut().expect("writable").fill(1);
        let mut n = m_get(M_WAIT, MT_CONTROL).expect("an mbuf");
        n.m_len().set(20);
        n.data_mut().expect("writable").fill(2);
        let _ = m.m_next.set(n);

        // (room, the length written back, cut, the bytes the user sees).
        let table: [(usize, usize, bool, Vec<u8>); 4] = [
            (
                64,
                36,
                false,
                [vec![1; 12], vec![0; 4], vec![2; 20]].concat(),
            ),
            (20, 20, true, [vec![1; 12], vec![0; 4], vec![2; 4]].concat()),
            (14, 14, true, [vec![1; 12], vec![0; 2]].concat()),
            (8, 8, true, vec![1; 8]),
        ];
        for (room, len, cut, bytes) in table {
            put_bytes(UCTL, &[0; 64]);
            let mut mp = Msghdr {
                msg_control: UCTL,
                msg_controllen: room as u32,
                ..Msghdr::default()
            };
            let mut clen = room;
            assert_eq!(copyout_control(&m, &mut mp, &mut clen), Ok(()));
            assert_eq!(clen, len, "room {room}");
            assert_eq!(mp.msg_flags & MSG_CTRUNC != 0, cut, "room {room}");
            assert_eq!(user_bytes(UCTL, bytes.len()).unwrap(), bytes, "room {room}");
        }
        // A buffer that is not user memory: the error, after the length it went past.
        let mut mp = Msghdr {
            msg_control: USER_BASE + USER_LEN - 4,
            msg_controllen: 64,
            ..Msghdr::default()
        };
        let mut clen = 64;
        assert_eq!(copyout_control(&m, &mut mp, &mut clen), Err(Errno::EFAULT));
        assert_eq!(clen, 16);
        drop(m);
        teardown();
    }

    #[test]
    fn ypconnect_refuses_in_the_cs_order() {
        let (_g, p) = setup();
        let saved = with_domainname(|d| {
            let mut b = [0u8; MAXHOSTNAMELEN];
            b.copy_from_slice(d);
            b
        });
        let set = |name: &[u8]| {
            crate::sysctl::with_domainname_mut(|d| {
                d.fill(0);
                d[..name.len()].copy_from_slice(name);
            });
        };

        set(b"");
        assert_eq!(sys_ypconnect(p, SOCK_STREAM), Err(Errno::EAFNOSUPPORT));
        set(b"bad/name");
        assert_eq!(sys_ypconnect(p, SOCK_STREAM), Err(Errno::EAFNOSUPPORT));
        set(b"n62b");
        assert_eq!(sys_ypconnect(p, SOCK_SEQPACKET), Err(Errno::EAFNOSUPPORT));
        let pr = p.process();
        pr.ps_flags.fetch_or(PS_CHROOT, Ordering::Relaxed);
        assert_eq!(sys_ypconnect(p, SOCK_DGRAM), Err(Errno::EACCES));
        pr.ps_flags.fetch_and(!PS_CHROOT, Ordering::Relaxed);
        // No binding file: namei's ENOENT, the kernel lock let go.
        let depth = crate::testutil::kernel_lock_depth();
        assert_eq!(sys_ypconnect(p, SOCK_DGRAM), Err(Errno::ENOENT));
        assert_eq!(crate::testutil::kernel_lock_depth(), depth);

        crate::sysctl::with_domainname_mut(|d| d.copy_from_slice(&saved));
        teardown();
    }

    #[test]
    fn the_images_are_the_cs() {
        assert_eq!(size_of::<Msghdr>(), 48);
        assert_eq!(size_of::<Mmsghdr>(), 56);
        let m = Msghdr {
            msg_name: 1,
            msg_namelen: 2,
            msg_iov: 3,
            msg_iovlen: 4,
            msg_control: 5,
            msg_controllen: 6,
            msg_flags: 7,
            _pad0: 8,
            _pad1: 9,
        };
        assert_eq!(
            pod_from_bytes::<Msghdr>(bytes_of(core::slice::from_ref(&m))),
            Some(m)
        );
        assert_eq!(MMSG_MAX, 1024);
        assert_eq!(
            SA_DATA_OFFSET,
            core::mem::offset_of!(hal::sys::socket::Sockaddr, sa_data)
        );
    }
}
/* </TESTS> */
