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
 * Copyright (c) 1995 Gordon Ross, Adam Glass
 * Copyright (c) 1992 Regents of the University of California.
 * All rights reserved.
 *
 * This software was developed by the Computer Systems Engineering group
 * at Lawrence Berkeley Laboratory under DARPA contract BG 91-66 and
 * contributed to Berkeley.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Lawrence Berkeley Laboratory and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
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
 * partially based on:
 *      libnetboot/rpc.c
 *               @(#) Header: rpc.c,v 1.12 93/09/28 08:31:56 leres Exp  (LBL)
 */
/* </LICENSES> */

/* <CODE> */
//! `nfs/krpc_subr.c` and `nfs/krpc.h` in the kernel: Sun RPC for the diskless boot, a one-shot
//! UDP client (`krpc_call`, with the portmapper lookup `krpc_portmap` on top of it) and the
//! XDR codecs of the boot protocols (strings and Internet addresses) on the client's cursors.
//! `krpc.h` is the prototypes and the portmapper's and bootparamd's program numbers, here.
//!
//! Upstream: sys/nfs/krpc_subr.c @ 3ce1f3f79392
//! Upstream: sys/nfs/krpc.h @ 3ce1f3f79392 (the constants and the prototypes)
//! LZ: sys/nfs/krpc_subr.rs@e8249449d905
//!
//! The file is compiled only with `option NFSCLIENT` (`sys/conf/files`): the module is behind
//! the `nfsclient` feature.
//!
//! ## Deviations
//! - `krpc_portmap` returns the port (network order, as the C stores it through `portp`);
//!   `krpc_call` takes the request body by value (it is always freed: the C leaves it to the
//!   caller on the failures before the header is made and leaves `*data` dangling on the
//!   later ones) and returns the reply body; the broadcast's sender address goes to `from_p`
//!   on success. A failed `socreate` returns at once: the C goes to `out:` and `soclose`s the
//!   uninitialised `so`. A reply freed on an error path after it arrived is not leaked as in
//!   the C. `retries` of -1 loops "forever" as in C.
//! - `krpc_call` and `krpc_portmap` take the calling thread: the kernel's `socreate` and
//!   `sosetopt` name it where the C's read `curproc`. The bind is still `proc0`'s.
//! - The server address is copied into the name mbuf for at most `sizeof(struct
//!   sockaddr_in)` bytes (the C copies `sin_len` bytes unchecked; every caller sets 16).
//! - The C's contiguity tests (`m->m_len < n`, `m_pullup`) are tests of the chain's length:
//!   the cursor copies across mbufs. A reply whose first mbuf holds fewer than
//!   `MIN_REPLY_HDR` bytes is no longer ignored when the chain holds them. The reply's status
//!   word is read only when the chain holds it (the C reads past a 16-byte reply's end; such a
//!   reply is refused by the strip after). A reply chain without a packet header (the C's
//!   `KASSERT`) is `EBADRPC`.
//! - `xdr_inaddr_decode` returns the address (the C stores it through `ia`).
//! - `xdr_string_encode` is `ENOMEM` (the callers' answer to the C's `NULL`) for a string whose
//!   XDR form exceeds `MCLBYTES`, and zeroes the padding (the C leaves the mbuf's old bytes).
//!   `xdr_string_decode` copies at most `min(*len_p, str.len() - 1)` bytes and NUL-terminates,
//!   so it cannot write past `str`.
//! - `krpc_get_xid`'s `static struct idgen32_ctx` and `called` are one `Mutexed` (the C runs it
//!   at boot with no lock).
//! - `krpc_call`'s `static u_int32_t xid` is a local (the C only ever assigns it right before
//!   use).
//!
//! ## Redesign
//! - LZ's codecs were `&'static Mbuf` chains handed from function to function, each decoder
//!   `m_pullup`ing and `m_adj`ing the head, the encoders an mbuf each written through raw
//!   views (6 `unsafe`), the socket names and options mbufs written through `ptr::write`. A
//!   decoder now reads from the reply's [`XdrReader`] (copies, bounded by the chain) and an
//!   encoder appends to an [`XdrWriter`]; a request is one chain, a reply one owned
//!   `MbufBox`. Every length from the wire (a string's, the verifier's) is compared as the
//!   C's `int` before it becomes a size, and an XDR string is checked whole against the chain
//!   before a byte of it is copied (the C's order: the destination is untouched on a short
//!   reply).
//! - The socket is the kernel's (`kernel::net::socket`): `socreate`, `sosetopt`, `sobind`
//!   under `solock_shared` with `proc0`'s privilege (the reserved port, `in_pcbbind`'s
//!   `INP_LOWPORT` test), `sosend` of an `m_copym` of the request, `soreceive` of the whole
//!   datagram, `soclose` on every path. No net-lock lend.
//! - The transaction id generator is the kernel's `idgen32` (`kernel::crypto::idgen`) in a
//!   `Mutexed<Option<..>>` made on first use (LZ: a `StaticCell` under a private mutex and an
//!   `AtomicBool`).

use hal::cpu::intr::IPL_NONE;
use hal::sync::mutex::Mutexed;
use hal::sys::mbuf::{MT_DATA, MT_SONAME, MT_SOOPTS};
use hal::sys::netinet_in::{
    INADDR_ANY, IP_PORTRANGE, IP_PORTRANGE_DEFAULT, IP_PORTRANGE_LOW, IPPROTO_IP, IPPROTO_UDP,
    InAddr, SockaddrIn,
};
use hal::sys::socket::{AF_INET, SO_BROADCAST, SO_RCVTIMEO, SOCK_DGRAM, SOL_SOCKET};
use hal::user::{AbiPod, bytes_of};

use super::nfsm_subs::{XdrReader, XdrWriter, nfsm_rndup};
use super::rpcv2::{RPC_REPLY, RPCAUTH_MAXSIZ, RPCAUTH_UNIX};
use super::xdr_subs::txdr_unsigned;
use crate::clock::Timeval;
use crate::crypto::idgen::Idgen32Ctx;
use crate::net::mbuf::{
    M_COPYALL, M_WAIT, MCLBYTES, Mbuf, MbufBox, m_adj, m_calchdrlen, m_cat, m_copym, m_get,
    m_gethdr,
};
use crate::net::socket::{
    Socket, sobind, soclose, socreate, solock_shared, soreceive, sosend, sosetopt,
};
use crate::netinet::if_ether::InAddrFmt;
use crate::prf::printf;
use crate::proc::{ProcGlue, Thread};
use crate::sys::errno::Errno;
use crate::uio::{Uio, UioRw, UioSeg};

/// `PMAPPORT`: RPC definitions for the portmapper.
pub const PMAPPORT: u16 = 111;
/// `PMAPPROG`.
pub const PMAPPROG: u32 = 100000;
/// `PMAPVERS`.
pub const PMAPVERS: u32 = 2;
/// `PMAPPROC_NULL`.
pub const PMAPPROC_NULL: u32 = 0;
/// `PMAPPROC_SET`.
pub const PMAPPROC_SET: u32 = 1;
/// `PMAPPROC_UNSET`.
pub const PMAPPROC_UNSET: u32 = 2;
/// `PMAPPROC_GETPORT`.
pub const PMAPPROC_GETPORT: u32 = 3;
/// `PMAPPROC_DUMP`.
pub const PMAPPROC_DUMP: u32 = 4;
/// `PMAPPROC_CALLIT`.
pub const PMAPPROC_CALLIT: u32 = 5;

/// `BOOTPARAM_PROG`: RPC definitions for bootparamd.
pub const BOOTPARAM_PROG: u32 = 100026;
/// `BOOTPARAM_VERS`.
pub const BOOTPARAM_VERS: u32 = 1;
/// `BOOTPARAM_WHOAMI`.
pub const BOOTPARAM_WHOAMI: u32 = 1;
/// `BOOTPARAM_GETFILE`.
pub const BOOTPARAM_GETFILE: u32 = 2;

/// `MIN_REPLY_HDR`: xid, dir, astat, errno.
const MIN_REPLY_HDR: usize = 16;

/// `MAX_RESEND_DELAY`: what is the longest we will wait before re-sending a request? Note
/// this is also the frequency of "RPC timeout" messages. The re-send loop count sup linearly
/// to this maximum, so the first complaint will happen after (1+2+3+4+5)=15 seconds.
const MAX_RESEND_DELAY: i32 = 5; // seconds

/// `struct rpc_call`: the generic RPC call header, with AUTH_UNIX credentials (`struct
/// auth_unix`: time, a null hostname, uid, gid and a null gid list, all zero: root) and an
/// AUTH_NULL verifier, every member a raw XDR word.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RpcCall {
    /// `rp_xid`: request transaction id.
    rp_xid: u32,
    /// `rp_direction`: call direction (0).
    rp_direction: u32,
    /// `rp_rpcvers`: rpc version (2).
    rp_rpcvers: u32,
    /// `rp_prog`: program.
    rp_prog: u32,
    /// `rp_vers`: version.
    rp_vers: u32,
    /// `rp_proc`: procedure.
    rp_proc: u32,
    /// `rpc_auth.authtype`: auth type.
    auth_authtype: u32,
    /// `rpc_auth.authlen`: auth length.
    auth_authlen: u32,
    /// `rpc_unix`: `ua_time`, `ua_hostname`, `ua_uid`, `ua_gid`, `ua_gidlist`, all zero.
    rpc_unix: [u32; 5],
    /// `rpc_verf.authtype`.
    verf_authtype: u32,
    /// `rpc_verf.authlen`.
    verf_authlen: u32,
}

hal::abi_pod!(RpcCall {
    rp_xid: u32,
    rp_direction: u32,
    rp_rpcvers: u32,
    rp_prog: u32,
    rp_vers: u32,
    rp_proc: u32,
    auth_authtype: u32,
    auth_authlen: u32,
    rpc_unix: [u32; 5],
    verf_authtype: u32,
    verf_authlen: u32,
});

/// The words of `struct rpc_reply`: `rp_xid`, `rp_direction`, `rp_astatus`, then the union:
/// `rp_errno`, or `rp_auth.authtype`, `rp_auth.authlen` and `rp_status`.
const REPLY_XID: usize = 0;
const REPLY_DIRECTION: usize = 1;
const REPLY_ASTATUS: usize = 2;
const REPLY_ERRNO: usize = 3;
const REPLY_AUTHTYPE: usize = 3;
const REPLY_AUTHLEN: usize = 4;
const REPLY_STATUS: usize = 5;
/// `sizeof(struct rpc_reply)`.
const RPC_REPLY_SIZE: usize = 6 * 4;

/// `sizeof(struct xdr_inaddr)`: an address type word and four words, one per byte.
pub const XDR_INADDR_SIZE: usize = 5 * 4;

/// `krpc_xid_ctx` and `called`: the transaction id generator, made on first use.
static KRPC_XID_CTX: Mutexed<Option<Idgen32Ctx>> = Mutexed::new(IPL_NONE, None);

/// The raw XDR word `i` of the reply `m` (`struct rpc_reply`'s members by index), `None`
/// when the chain is shorter.
fn reply_word(m: &Mbuf, i: usize) -> Option<u32> {
    XdrReader::at(m, 4 * i).and_then(|mut r| r.word()).ok()
}

/// `krpc_get_xid()`: returns an unpredictable XID.
pub fn krpc_get_xid() -> u32 {
    let mut g = KRPC_XID_CTX.lock();
    let ctx = g.get_or_insert_with(|| {
        let mut c = Idgen32Ctx::zeroed();
        c.init();
        c
    });
    ctx.generate()
}

/// `krpc_portmap(sin, prog, vers, portp)`: calls the portmapper at `sin` to look up the port
/// number (network order) of a particular RPC program, over UDP. The portmapper's own port is
/// fixed and needs no call. `sin_port` is set to the portmapper's port.
pub fn krpc_portmap<G: ProcGlue>(
    p: &Thread<G>,
    sin: &mut SockaddrIn,
    prog: u32,
    vers: u32,
) -> Result<u16, Errno> {
    // The portmapper port is fixed.
    if prog == PMAPPROG {
        return Ok(PMAPPORT.to_be());
    }

    // struct sdata: the program, the version, the protocol and a port (unused), all words.
    let mut w = XdrWriter::reqhead(0);
    w.u32(prog);
    w.u32(vers);
    w.u32(IPPROTO_UDP as u32);
    w.u32(0);

    // Do the RPC to get it.
    sin.sin_port = PMAPPORT.to_be();
    let m = krpc_call(
        p,
        sin,
        PMAPPROG,
        PMAPVERS,
        PMAPPROC_GETPORT,
        w.into_chain(),
        None,
        -1,
    )?;

    // struct rdata: a 16-bit pad and the 16-bit port (the XDR word of the port, as it
    // lies). A reply too short for it is the C's failed `m_pullup`: ENOBUFS.
    let rdata = XdrReader::new(&m).get::<4>().map_err(|_| Errno::ENOBUFS)?;
    Ok(u16::from_ne_bytes([rdata[2], rdata[3]]))
}

/// `krpc_call(sa, prog, vers, func, data, from_p, retries)`: does a remote procedure call
/// (RPC) over UDP and waits for its reply. `data` is the request body; the answer is the
/// reply body, after the RPC header. If `from_p` is given, then we are doing broadcast, and
/// the address from whence the response came is saved there.
///
/// The request is sent from a reserved port (some NFS servers refuse requests from
/// non-privileged ports), and sent again after an increasing delay, up to `retries` times
/// (`-1`: until it is answered); after `MAX_RESEND_DELAY` seconds of delay it complains
/// about each retry. `ETIMEDOUT` when the retries run out.
#[allow(
    clippy::too_many_arguments,
    reason = "the C's argument list and the thread"
)]
pub fn krpc_call<G: ProcGlue>(
    p: &Thread<G>,
    sa: &SockaddrIn,
    prog: u32,
    vers: u32,
    func: u32,
    data: MbufBox,
    from_p: Option<&mut Option<MbufBox>>,
    retries: i32,
) -> Result<MbufBox, Errno> {
    // Validate address family. Sorry, this is INET specific...
    if sa.sin_family != AF_INET {
        return Err(Errno::EAFNOSUPPORT);
    }

    // Create socket and set its receive timeout.
    let so = socreate(p, i32::from(AF_INET), SOCK_DGRAM, 0)?;

    let r = krpc_exchange(
        p,
        &so,
        sa,
        [prog, vers, func],
        data,
        from_p.is_some(),
        retries,
    );

    // out:
    let _ = soclose(&so, 0);
    let (reply, from) = r?;
    // result
    if let Some(from_p) = from_p {
        *from_p = from;
    }
    Ok(reply)
}

/// An mbuf of type `type_` holding the bytes of `v` (a socket option's value, `MT_SOOPTS`,
/// or an address, `MT_SONAME`).
fn sockopt_mbuf<T: AbiPod>(v: &T, type_: i32) -> Result<MbufBox, Errno> {
    let b = bytes_of(core::slice::from_ref(v));
    let mut m = m_get(M_WAIT, type_).ok_or(Errno::ENOBUFS)?;
    m.m_len().set(b.len() as u32);
    m.data_mut().ok_or(Errno::ENOBUFS)?.copy_from_slice(b);
    Ok(m)
}

/// `sosetopt(so, level, name, m)` with `v` in a fresh `MT_SOOPTS` mbuf, freed after (the C's
/// `m_freem(m)`).
fn setopt<G: ProcGlue, T: AbiPod>(
    so: &Socket<G>,
    p: &Thread<G>,
    level: i32,
    name: i32,
    v: &T,
) -> Result<(), Errno> {
    let mut m = sockopt_mbuf(v, MT_SOOPTS)?;
    sosetopt(so, p, level, name, Some(&mut *m))
}

/// The body of `krpc_call` between creating the socket and `out:`: sets the socket up, sends
/// the request `call` (program, version, procedure), repeatedly, and returns the reply with
/// its RPC header stripped, and its sender's address.
fn krpc_exchange<G: ProcGlue>(
    p: &Thread<G>,
    so: &Socket<G>,
    sa: &SockaddrIn,
    call: [u32; 3],
    data: MbufBox,
    broadcast: bool,
    retries: i32,
) -> Result<(MbufBox, Option<MbufBox>), Errno> {
    let [prog, vers, func] = call;
    setopt(so, p, SOL_SOCKET, SO_RCVTIMEO, &Timeval::new(1, 0))?;

    // Enable broadcast if necessary.
    if broadcast {
        setopt(so, p, SOL_SOCKET, SO_BROADCAST, &1i32)?;
    }

    // Bind the local endpoint to a reserved port, because some NFS servers refuse requests
    // from non-reserved (non-privileged) ports. The bind is process 0's, whose privilege
    // `in_pcbbind` tests for the low range, whoever calls.
    setopt(so, p, IPPROTO_IP, IP_PORTRANGE, &IP_PORTRANGE_LOW)?;

    let any = SockaddrIn {
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_family: AF_INET,
        sin_port: 0u16.to_be(),
        sin_addr: InAddr { s_addr: INADDR_ANY },
        ..SockaddrIn::default()
    };
    let m = sockopt_mbuf(&any, MT_SONAME)?;
    let error = {
        let mut w = solock_shared(so);
        sobind(so, &mut w, &m, G::tables().proc0().get_ref())
    };
    drop(m);
    if let Err(e) = error {
        printf(format_args!("bind failed\n"));
        return Err(e);
    }

    setopt(so, p, IPPROTO_IP, IP_PORTRANGE, &IP_PORTRANGE_DEFAULT)?;

    // Setup socket address for the server (at most a `sockaddr_in`, the module's
    // deviations).
    let nam = sockopt_mbuf(sa, MT_SONAME)?;
    let len = usize::from(sa.sin_len).min(size_of::<SockaddrIn>());
    nam.m_len().set(len as u32);

    // Prepend RPC message header.
    let xid = krpc_get_xid();
    let header = RpcCall {
        rp_xid: txdr_unsigned(xid),
        // rp_direction = 0
        rp_rpcvers: txdr_unsigned(2),
        rp_prog: txdr_unsigned(prog),
        rp_vers: txdr_unsigned(vers),
        rp_proc: txdr_unsigned(func),
        // rpc_auth part (auth_unix as root)
        auth_authtype: txdr_unsigned(RPCAUTH_UNIX),
        auth_authlen: txdr_unsigned(size_of::<[u32; 5]>() as u32),
        // rpc_verf part (auth_null) is zero.
        ..RpcCall::default()
    };
    let mut mhead = m_gethdr(M_WAIT, MT_DATA).ok_or(Errno::ENOBUFS)?;
    let hb = bytes_of(core::slice::from_ref(&header));
    mhead.m_len().set(hb.len() as u32);
    mhead.data_mut().ok_or(Errno::ENOBUFS)?.copy_from_slice(hb);
    // mhead->m_next = *data (m_cat links it, or copies a body that fits behind the header:
    // the same bytes go out).
    m_cat(&mut mhead, Some(data));

    // Setup packet header
    m_calchdrlen(&mhead);
    mhead.m_pkthdr().ph_ifidx.set(0);

    // Send it, repeatedly, until a reply is received, but delay each re-send by an
    // increasing amount. If the delay hits the maximum, start complaining.
    let mut retries = retries;
    let mut timo = 0;
    while retries != 0 {
        // Send RPC request (or re-send).
        let copy = m_copym(&mut mhead, 0, M_COPYALL, M_WAIT).ok_or(Errno::ENOBUFS)?;
        if let Err(e) = sosend(so, Some(&*nam), None, Some(copy), None, 0) {
            printf(format_args!("krpc_call: sosend: {}\n", e.as_i32()));
            return Err(e);
        }

        // Determine new timeout.
        if timo < MAX_RESEND_DELAY {
            timo += 1;
        } else {
            printf(format_args!(
                "RPC timeout for server {} (0x{:x}) prog {}\n",
                InAddrFmt(sa.sin_addr),
                u32::from_be(sa.sin_addr.s_addr),
                prog
            ));
        }

        // Wait for up to timo seconds for a reply. The socket receive timeout was set to 1
        // second.
        let mut secs = timo;
        while secs > 0 {
            let mut from: Option<MbufBox> = None;
            let mut m: Option<MbufBox> = None;
            let mut iov: [crate::uio::Iovec; 0] = [];
            let mut auio = Uio::<G> {
                uio_iov: &mut iov,
                uio_offset: 0,
                uio_resid: 1 << 16,
                uio_segflg: UioSeg::UIO_SYSSPACE,
                uio_rw: UioRw::UIO_READ,
                uio_procp: None,
            };
            let len = auio.uio_resid;
            let mut rcvflg = 0;
            match soreceive(
                so,
                Some(&mut from),
                &mut auio,
                Some(&mut m),
                None,
                Some(&mut rcvflg),
                0,
            ) {
                Err(Errno::EWOULDBLOCK) => {
                    secs -= 1;
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => {}
            }
            let len = len - auio.uio_resid;
            let Some(m) = m else {
                continue;
            };

            if krpc_reply_ok(&m, len, xid) {
                // gotreply:
                return strip_reply(m).map(|m| (m, from));
            }
        } // while secs
        retries = retries.wrapping_sub(1);
    } // forever send/receive

    Err(Errno::ETIMEDOUT)
}

/// The tests of `krpc_call`'s receive loop on a datagram `m` of `len` bytes: a header at
/// least, a reply, the request's `xid`, accepted, and a call that succeeded; a denied one is
/// reported as the C reports it. The status word is looked at only when the reply holds it
/// (`strip_reply` refuses a shorter one).
fn krpc_reply_ok(m: &Mbuf, len: usize, xid: u32) -> bool {
    // Does the reply contain at least a header?
    if len < MIN_REPLY_HDR {
        return false;
    }
    let word = |i| reply_word(m, i);

    // Is it the right reply?
    if word(REPLY_DIRECTION) != Some(txdr_unsigned(RPC_REPLY)) {
        return false;
    }
    if word(REPLY_XID) != Some(txdr_unsigned(xid)) {
        return false;
    }

    // Was RPC accepted? (authorization OK)
    match word(REPLY_ASTATUS) {
        Some(0) => {}
        Some(_) => {
            let e = u32::from_be(word(REPLY_ERRNO).unwrap_or(0));
            printf(format_args!("rpc denied, error={}\n", e as i32));
            return false;
        }
        None => return false,
    }

    // Did the call succeed?
    if let Some(s) = word(REPLY_STATUS).filter(|&s| s != 0) {
        let e = u32::from_be(s);
        printf(format_args!("rpc denied, status={}\n", e as i32));
        return false;
    }
    true
}

/// `gotreply:` of `krpc_call`: the RPC reply header (and the verifier its `authlen` says)
/// stripped off the reply `m`; `EBADRPC` for a reply too short for them or a verifier length
/// out of bounds.
fn strip_reply(m: MbufBox) -> Result<MbufBox, Errno> {
    let Some(ph) = m.pkthdr() else {
        return Err(Errno::EBADRPC);
    };
    let total = i64::from(ph.len.get());
    let mut len = RPC_REPLY_SIZE as i64;
    if total < len {
        return Err(Errno::EBADRPC);
    }
    if reply_word(&m, REPLY_AUTHTYPE) != Some(0) {
        // The C's `int authlen = fxdr_unsigned(u_int32_t, ...)`: compared as an `int`.
        let authlen = u32::from_be(reply_word(&m, REPLY_AUTHLEN).ok_or(Errno::EBADRPC)?) as i32;
        if authlen < 0 || authlen as usize > RPCAUTH_MAXSIZ {
            return Err(Errno::EBADRPC);
        }
        len += i64::from((authlen + 3) & !3); // XXX?
    }
    if len < 0 || total < len {
        return Err(Errno::EBADRPC);
    }
    m_adj(&m, len as i32);
    Ok(m)
}

/// `xdr_string_encode(str, len)`: the XDR representation of a string for an RPC appended to
/// `w`: its length (without NUL or padding) and its bytes padded to a word boundary.
/// `ENOMEM` (the callers' answer to the C's `NULL`) when it would not fit a cluster.
pub fn xdr_string_encode(w: &mut XdrWriter, str: &[u8]) -> Result<(), Errno> {
    let dlen = nfsm_rndup(str.len()); // padded string length
    let mlen = dlen + 4; // message length
    if mlen > MCLBYTES {
        // If too big, we just can't do it.
        return Err(Errno::ENOMEM);
    }
    w.strtombuf(str);
    Ok(())
}

/// `xdr_string_decode(m, str, len_p)`: the XDR string at `r` into `str` as a NUL-terminated
/// string, at most `*len_p` bytes of it (and at most what `str` holds but its NUL); `*len_p`
/// set to the length kept, `r` past the whole string and its padding. `EBADRPC` (the C's
/// `NULL`) when the length is absurd or the chain is too short for the whole string, checked
/// before a byte is copied.
pub fn xdr_string_decode(
    r: &mut XdrReader<'_>,
    str: &mut [u8],
    len_p: &mut usize,
) -> Result<(), Errno> {
    let mut probe = r.clone();
    let slen = probe.i32()?; // string length
    if !(0..=i32::MAX - 3 - 4).contains(&slen) {
        return Err(Errno::EBADRPC);
    }
    let padded = nfsm_rndup(slen as usize);
    // The whole string is in the chain (the C's `m_pkthdr.len < mlen`).
    probe.adv(padded)?;

    let Some(room) = str.len().checked_sub(1) else {
        crate::assert::kpanic(format_args!("xdr_string_decode: no room for a string"));
    };
    let keep = (slen as usize).min(*len_p).min(room);
    r.adv(4)?;
    r.bytes_into(&mut str[..keep])?;
    r.adv(padded - keep)?;

    str[keep] = 0;
    *len_p = keep;
    Ok(())
}

/// `xdr_inaddr_encode(ia)`: the XDR representation of an Internet address in an RPC message
/// (really four ints, NOT chars; blech): the type word 1 and one word per byte of the
/// address, appended to `w`.
pub fn xdr_inaddr_encode(w: &mut XdrWriter, ia: &InAddr) {
    w.u32(1);
    for b in ia.s_addr.to_ne_bytes() {
        w.u32(u32::from(b));
    }
}

/// `xdr_inaddr_decode(m, ia)`: the XDR Internet address at `r` (`INADDR_ANY` if the type word
/// is not 1), `r` past it; `EBADRPC` (the C's `NULL`) when the chain is too short.
pub fn xdr_inaddr_decode(r: &mut XdrReader<'_>) -> Result<InAddr, Errno> {
    let xi: [u8; XDR_INADDR_SIZE] = r.get()?;
    let word =
        |i: usize| u32::from_be_bytes([xi[4 * i], xi[4 * i + 1], xi[4 * i + 2], xi[4 * i + 3]]);
    if word(0) != 1 {
        return Ok(InAddr { s_addr: INADDR_ANY });
    }
    // fxdr_unsigned(u_int8_t, *ip): each word's low byte.
    let b = [word(1) as u8, word(2) as u8, word(3) as u8, word(4) as u8];
    Ok(InAddr {
        s_addr: u32::from_ne_bytes(b),
    })
}

const _: () = {
    assert!(size_of::<RpcCall>() == 15 * 4);
    assert!(size_of::<RpcCall>() <= crate::net::mbuf::MHLEN);
};
/// The diskless boot's test fixture (`cfg(test)`), shared by this module's tests and
/// `nfs_boot`'s: chains and XDR bytes, and a scripted UDP peer behind a test interface (the
/// UDP of `testudp` over the test glue).
#[cfg(test)]
pub(crate) mod testkrpc {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::vec;
    use std::vec::Vec;

    use hal::sys::netinet_in::{IPPROTO_UDP, InAddr, SockaddrIn};
    use hal::sys::netinet_ip::Ip;
    use hal::sys::netinet_udp::Udphdr;
    use hal::sys::socket::AF_INET;

    use super::RPC_REPLY;
    use crate::fs::nfs::nfsm_subs::nfsm_rndup;
    use crate::net::if_var::{Ifnet, NetDriver};
    use crate::net::mbuf::{
        M_UDP_CSUM_IN_OK, M_WAIT, MChainBuilder, MHLEN, MT_DATA, MbufBox, m_get, m_gethdr, mclget,
    };
    use crate::net::route::Rtentry;
    use crate::netinet::in_pcb::testinet;
    use crate::netinet::udp_usrreq::testudp::{self, G};
    use crate::proc::{ProcGlue, Thread};
    use crate::sys::errno::Errno;

    /// The segments `segs` as one chain, its first mbuf a packet header's (the total length
    /// in it), the others plain mbufs.
    pub(crate) fn pkt_chain(segs: &[&[u8]]) -> MbufBox {
        let mut b = MChainBuilder::new();
        let total: usize = segs.iter().map(|s| s.len()).sum();
        for (i, s) in segs.iter().enumerate() {
            let mut m = if i == 0 {
                m_gethdr(M_WAIT, MT_DATA).expect("an mbuf")
            } else {
                m_get(M_WAIT, MT_DATA).expect("an mbuf")
            };
            if s.len() > MHLEN {
                mclget(&mut m, M_WAIT);
            }
            m.m_len().set(s.len() as u32);
            m.data_mut().expect("writable").copy_from_slice(s);
            if i == 0 {
                m.m_pkthdr().len.set(total as i32);
            }
            b.push(m);
        }
        b.into_head().expect("a chain")
    }

    /// An XDR string: its length word, its bytes, its zero padding.
    pub(crate) fn xstr(s: &[u8]) -> Vec<u8> {
        let mut v = (s.len() as u32).to_be_bytes().to_vec();
        v.extend_from_slice(s);
        v.resize(4 + nfsm_rndup(s.len()), 0);
        v
    }

    /// An XDR `struct xdr_inaddr` of `a`.
    pub(crate) fn xaddr(a: [u8; 4]) -> Vec<u8> {
        let mut v = 1u32.to_be_bytes().to_vec();
        for b in a {
            v.extend_from_slice(&u32::from(b).to_be_bytes());
        }
        v
    }

    /// The words of `w`, big-endian.
    pub(crate) fn words(w: &[u32]) -> Vec<u8> {
        w.iter().flat_map(|x| x.to_be_bytes()).collect()
    }

    /// An accepted reply's header (AUTH_NULL verifier, success) before `body`.
    pub(crate) fn accepted(xid: u32, body: &[u8]) -> Vec<u8> {
        let mut v = vec![];
        v.extend_from_slice(&xid.to_ne_bytes());
        v.extend_from_slice(&words(&[RPC_REPLY, 0, 0, 0, 0]));
        v.extend_from_slice(body);
        v
    }

    /// The peer's wire: the packets the client's interface outputs, kept for the peer.
    #[derive(Default)]
    pub(crate) struct Wire {
        out: Mutex<VecDeque<Vec<u8>>>,
    }

    impl NetDriver for Wire {
        fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            if cmd == hal::sys::sockio::SIOCSIFADDR {
                ifp.if_flags
                    .set(ifp.if_flags.get() | hal::sys::net_if::IFF_UP);
            }
            Ok(())
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            m: MbufBox,
            _dst: crate::net::proto::SockaddrView<'_>,
            _rt: Option<&Rtentry>,
        ) -> Result<(), Errno> {
            let mut bytes = vec![0u8; m.m_pkthdr().len.get() as usize];
            crate::net::mbuf::m_copydata(&m, 0, &mut bytes);
            if let Ok(mut q) = self.out.lock() {
                q.push_back(bytes);
            }
            Ok(())
        }
    }

    /// A request the peer saw: the addresses, the ports and the UDP payload.
    #[derive(Clone, Debug)]
    pub(crate) struct Seen {
        pub(crate) src: [u8; 4],
        pub(crate) dst: [u8; 4],
        pub(crate) sport: u16,
        pub(crate) dport: u16,
        pub(crate) payload: Vec<u8>,
    }

    impl Seen {
        /// The call's word `i` (host order).
        pub(crate) fn word(&self, i: usize) -> u32 {
            u32::from_be_bytes(self.payload[4 * i..4 * i + 4].try_into().unwrap_or([0; 4]))
        }
        /// The call's raw xid.
        pub(crate) fn xid(&self) -> u32 {
            u32::from_ne_bytes(self.payload[..4].try_into().unwrap_or([0; 4]))
        }
        /// The program, version and procedure called.
        pub(crate) fn call(&self) -> (u32, u32, u32) {
            (self.word(3), self.word(4), self.word(5))
        }
        /// The call's body, after the 60-byte header.
        pub(crate) fn body(&self) -> &[u8] {
            &self.payload[60..]
        }
    }

    /// The scripted server at `server`: a thread that hands each request the client sends to
    /// `script` and sends the client every payload it answers, from `server` and the port the
    /// request went to, until stopped.
    pub(crate) struct Peer {
        stop: Arc<AtomicBool>,
        seen: Arc<Mutex<Vec<Seen>>>,
        th: Option<std::thread::JoinHandle<()>>,
    }

    impl Peer {
        pub(crate) fn start(
            wire: &'static Wire,
            server: [u8; 4],
            mut script: impl FnMut(&Seen) -> Vec<Vec<u8>> + Send + 'static,
        ) -> Self {
            let stop = Arc::new(AtomicBool::new(false));
            let seen = Arc::new(Mutex::new(Vec::new()));
            let (st, sn) = (stop.clone(), seen.clone());
            let th = std::thread::spawn(move || {
                while !st.load(Ordering::SeqCst) {
                    let next = wire.out.lock().ok().and_then(|mut q| q.pop_front());
                    let Some(p) = next else {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        continue;
                    };
                    let hl = usize::from(p[0] & 0x0f) * 4;
                    let ip4 = |o: usize| [p[o], p[o + 1], p[o + 2], p[o + 3]];
                    let s = Seen {
                        src: ip4(12),
                        dst: ip4(16),
                        sport: u16::from_be_bytes([p[hl], p[hl + 1]]),
                        dport: u16::from_be_bytes([p[hl + 2], p[hl + 3]]),
                        payload: p[hl + 8..].to_vec(),
                    };
                    if let Ok(mut v) = sn.lock() {
                        v.push(s.clone());
                    }
                    for reply in script(&s) {
                        let m = dgram(server, s.dport, s.src, s.sport, &reply);
                        let _ = testudp::input(m, 20, i32::from(AF_INET));
                    }
                }
            });
            Self {
                stop,
                seen,
                th: Some(th),
            }
        }

        /// Stops the peer: the requests it saw.
        pub(crate) fn stop(mut self) -> Vec<Seen> {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(th) = self.th.take() {
                th.join().expect("the peer");
            }
            self.seen.lock().map(|v| v.clone()).unwrap_or_default()
        }
    }

    impl Drop for Peer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            if let Some(th) = self.th.take() {
                let _ = th.join();
            }
        }
    }

    /// An IPv4 datagram from `src`:`sport` to `dst`:`dport` with `payload`, its checksum
    /// verified by the interface (`M_UDP_CSUM_IN_OK`).
    fn dgram(src: [u8; 4], sport: u16, dst: [u8; 4], dport: u16, payload: &[u8]) -> MbufBox {
        let len = 28 + payload.len();
        let mut ip = Ip {
            ip_len: (len as u16).to_be(),
            ip_ttl: 64,
            ip_p: IPPROTO_UDP as u8,
            ip_src: InAddr {
                s_addr: u32::from_ne_bytes(src),
            },
            ip_dst: InAddr {
                s_addr: u32::from_ne_bytes(dst),
            },
            ..Ip::default()
        };
        ip.set_ip_v(4);
        ip.set_ip_hl(5);
        let uh = Udphdr {
            uh_sport: sport.to_be(),
            uh_dport: dport.to_be(),
            uh_ulen: ((8 + payload.len()) as u16).to_be(),
            uh_sum: 0x1234u16.to_be(),
        };
        let mut p = vec![0u8; len];
        assert!(hal::mem::pod::put(&mut p, 0, &ip));
        assert!(hal::mem::pod::put(&mut p, 20, &uh));
        p[28..].copy_from_slice(payload);
        let m = pkt_chain(&[&p]);
        let ph = m.m_pkthdr();
        ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_IN_OK);
        m
    }

    /// The test domain and UDP (`testudp`), `proc0` recorded (this test thread's root thread,
    /// once per thread: a test glue's process tables are the thread's, `test_proc_glue!`),
    /// and an interface `krpc<n>` at 10.77.n.1/24 whose packets go to the returned wire.
    pub(crate) fn setup(
        name: &[u8],
        n: u8,
    ) -> (MutexGuard<'static, ()>, &'static Thread<G>, &'static Wire) {
        let (g, p) = testudp::setup();
        crate::testutil::install();
        std::thread_local! {
            static PROC0: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
        }
        if !PROC0.with(|m| m.replace(true)) {
            G::tables().set_process0(p.process(), hal::ds::pin::pin_static(p));
        }
        let wire: &'static Wire = std::boxed::Box::leak(std::boxed::Box::default());
        let _ifp = testinet::iface_with(name, [10, 77, n, 1], wire);
        (g, p, wire)
    }

    /// A thread of user `uid` (not the superuser), for the privilege tests.
    pub(crate) fn user_thread(uid: u32) -> &'static Thread<G> {
        let pr = crate::proc::process_alloc::<G>(4000 + uid as i32, ());
        let mut ids = crate::cred::Ucred::new();
        ids.cr_uid.set(uid);
        ids.cr_ruid.set(uid);
        drop(pr.set_ucred(crate::cred::CredRef::new(ids)));
        let t = crate::proc::thread_alloc::<G>(&pr, ());
        drop(t.set_cred(pr.ucred()));
        core::mem::forget(pr);
        hal::mem::pool::PoolBox::leak(t)
    }

    /// The address `a`, port 0, as a `sockaddr_in`.
    pub(crate) fn sin(a: [u8; 4], port: u16) -> SockaddrIn {
        testudp::sin(a, port)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the boot's RPC: the XDR codecs against the C's rules (strings with their
    // bounds and padding, addresses), the reply's strip, the receive loop's tests, property
    // tests of the decoders over chains split at random, and `krpc_call` end to end against a
    // scripted UDP peer: the header, the reserved port bound by `proc0`, a lost request sent
    // again, `ETIMEDOUT`, stray and denied replies skipped, the broadcast and its sender.

    use std::vec;
    use std::vec::Vec;

    use super::testkrpc::{
        Peer, Seen, accepted, pkt_chain, setup, sin, user_thread, words, xaddr, xstr,
    };
    use super::*;
    use crate::fs::nfs::nfsm_subs::{TestRng, test_bytes, test_chain};

    /// The bytes an [`XdrWriter`] built.
    fn written(f: impl FnOnce(&mut XdrWriter)) -> Vec<u8> {
        let mut w = XdrWriter::reqhead(0);
        f(&mut w);
        test_bytes(&w.into_chain())
    }

    #[test]
    fn strings_encode_with_their_padding_and_the_cs_limit() {
        let _g = crate::net::mbuf::tests::setup();
        for s in [&b""[..], b"a", b"root", b"swap!", b"/export/root"] {
            let mut r = Ok(());
            let b = written(|w| r = xdr_string_encode(w, s));
            assert_eq!(r, Ok(()));
            assert_eq!(b, xstr(s), "{s:?}");
        }
        // 4 + rndup(len) > MCLBYTES is the C's NULL, ENOMEM to its callers; one byte less
        // fits.
        let big = vec![b'x'; MCLBYTES - 4];
        let mut r = Err(Errno::EIO);
        let b = written(|w| r = xdr_string_encode(w, &big));
        assert_eq!((r, b.len()), (Ok(()), MCLBYTES));
        let big = vec![b'x'; MCLBYTES - 3];
        let mut w = XdrWriter::reqhead(0);
        assert_eq!(xdr_string_encode(&mut w, &big), Err(Errno::ENOMEM));
    }

    #[test]
    fn strings_decode_bounded_by_the_length_wanted_and_the_buffer() {
        let _g = crate::net::mbuf::tests::setup();
        // (wire string, len wanted, buffer size): the bytes kept and the length reported;
        // the cursor always ends past the whole string and its padding.
        let table: [(&[u8], usize, usize, &[u8]); 6] = [
            (b"client", 255, 256, b"client"),
            (b"", 255, 256, b""),
            (b"abcdefgh", 3, 256, b"abc"),
            (b"abcdefgh", 255, 5, b"abcd"),
            (b"abcde", 5, 6, b"abcde"),
            (b"abcde", 0, 6, b""),
        ];
        for (s, want, room, kept) in table {
            let mut bytes = xstr(s);
            bytes.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]);
            let m = test_chain(&[&bytes]);
            let mut r = XdrReader::new(&m);
            let mut buf = vec![0xeeu8; room];
            let mut len = want;
            assert_eq!(xdr_string_decode(&mut r, &mut buf, &mut len), Ok(()));
            assert_eq!(len, kept.len());
            assert_eq!(&buf[..len], kept);
            assert_eq!(buf[len], 0, "NUL-terminated");
            assert_eq!(r.word(), Ok(u32::from_ne_bytes([0xaa, 0xbb, 0xcc, 0xdd])));
        }
    }

    #[test]
    fn a_bad_string_leaves_the_buffer_untouched() {
        let _g = crate::net::mbuf::tests::setup();
        let cases: [Vec<u8>; 6] = [
            // Negative (-2^31, and -1, which as a size would overflow the rounding), and
            // above INT_MAX - 3 - 4.
            words(&[0x8000_0000]),
            words(&[0xffff_ffff]),
            words(&[0x7fff_fff9]),
            // Longer than the chain: the bytes, or only the padding, missing.
            words(&[8, 0x6162_6364]),
            {
                let mut v = words(&[5]);
                v.extend_from_slice(b"abcde");
                v
            },
            // No length word at all.
            vec![0, 0],
        ];
        for bytes in cases {
            let m = test_chain(&[&bytes]);
            let mut r = XdrReader::new(&m);
            let mut buf = [0xeeu8; 16];
            let mut len = 15;
            assert_eq!(
                xdr_string_decode(&mut r, &mut buf, &mut len),
                Err(Errno::EBADRPC),
                "{bytes:?}"
            );
            assert_eq!(buf, [0xee; 16]);
            assert_eq!(len, 15);
        }
        // The largest length the C accepts is still refused by the chain's end.
        let m = test_chain(&[&words(&[0x7fff_fff8])]);
        let mut buf = [0u8; 4];
        let mut len = 3;
        assert_eq!(
            xdr_string_decode(&mut XdrReader::new(&m), &mut buf, &mut len),
            Err(Errno::EBADRPC)
        );
    }

    #[test]
    fn addresses_round_trip_and_another_type_is_any() {
        let _g = crate::net::mbuf::tests::setup();
        let a = InAddr {
            s_addr: u32::from_ne_bytes([10, 77, 1, 200]),
        };
        let b = written(|w| xdr_inaddr_encode(w, &a));
        assert_eq!(b, xaddr([10, 77, 1, 200]));
        let m = test_chain(&[&b[..7], &b[7..]]);
        assert_eq!(xdr_inaddr_decode(&mut XdrReader::new(&m)), Ok(a));
        // Each word's low byte (fxdr_unsigned(u_int8_t, ..)).
        let m = test_chain(&[&words(&[1, 0x1_0a, 0xff_4d, 0x01, 0x0102])]);
        assert_eq!(
            xdr_inaddr_decode(&mut XdrReader::new(&m)).map(|a| a.s_addr.to_ne_bytes()),
            Ok([10, 77, 1, 2])
        );
        let m = test_chain(&[&words(&[2, 10, 77, 1, 2])]);
        assert_eq!(
            xdr_inaddr_decode(&mut XdrReader::new(&m)).map(|a| a.s_addr),
            Ok(INADDR_ANY)
        );
        let m = test_chain(&[&words(&[1, 10, 77, 1])]);
        assert_eq!(
            xdr_inaddr_decode(&mut XdrReader::new(&m)),
            Err(Errno::EBADRPC)
        );
    }

    #[test]
    fn the_strip_takes_the_header_and_the_verifier_by_its_bounded_length() {
        let _g = crate::net::mbuf::tests::setup();
        let xid = txdr_unsigned(7);
        // AUTH_NULL: 24 bytes go.
        let r = accepted(xid, b"body");
        let m = strip_reply(pkt_chain(&[&r[..10], &r[10..]])).expect("stripped");
        assert_eq!(test_bytes(&m), b"body");
        // A verifier of 5 bytes takes 8 more (rounded), as the C.
        let mut r = vec![];
        r.extend_from_slice(&words(&[7, RPC_REPLY, 0, 1, 5, 0]));
        r.extend_from_slice(b"VVVVVVVV");
        r.extend_from_slice(b"rest");
        let m = strip_reply(pkt_chain(&[&r])).expect("stripped");
        assert_eq!(test_bytes(&m), b"rest");
        // RPCAUTH_MAXSIZ is the bound; above it, or a negative int, EBADRPC, though the
        // chain holds the verifier it announces.
        for (authlen, ok) in [(400u32, true), (401, false), (0x8000_0000, false)] {
            let mut r = words(&[7, RPC_REPLY, 0, 1, authlen, 0]);
            r.resize(24 + 404 + 8, 0);
            assert_eq!(strip_reply(pkt_chain(&[&r])).is_ok(), ok, "{authlen}");
        }
        // Shorter than the header, or than the verifier it announces.
        let r = words(&[7, RPC_REPLY, 0, 0, 0]);
        assert_eq!(strip_reply(pkt_chain(&[&r])).err(), Some(Errno::EBADRPC));
        let r = words(&[7, RPC_REPLY, 0, 1, 8, 0, 1]);
        assert_eq!(strip_reply(pkt_chain(&[&r])).err(), Some(Errno::EBADRPC));
        // No packet header: EBADRPC, not a panic.
        let r = accepted(xid, b"x");
        assert_eq!(strip_reply(test_chain(&[&r])).err(), Some(Errno::EBADRPC));
    }

    #[test]
    fn the_receive_loop_takes_only_the_answer_to_its_call() {
        let _g = crate::net::mbuf::tests::setup();
        let xid = 0x1234_5678;
        let ok = accepted(txdr_unsigned(xid), b"");
        let m = pkt_chain(&[&ok[..3], &ok[3..]]);
        assert!(krpc_reply_ok(&m, ok.len(), xid));
        // Under MIN_REPLY_HDR by the received count.
        assert!(!krpc_reply_ok(&m, 15, xid));
        // Another xid, a call, a denial, a failed call.
        let other = accepted(txdr_unsigned(xid + 1), b"");
        assert!(!krpc_reply_ok(&pkt_chain(&[&other]), 24, xid));
        let mut call = ok.clone();
        call[4..8].copy_from_slice(&0u32.to_be_bytes());
        assert!(!krpc_reply_ok(&pkt_chain(&[&call]), 24, xid));
        let before = crate::testutil::printf_count("rpc denied, error=1\n");
        let denied = [
            &txdr_unsigned(xid).to_ne_bytes()[..],
            &words(&[RPC_REPLY, 1, 1]),
        ]
        .concat();
        assert!(!krpc_reply_ok(&pkt_chain(&[&denied]), 16, xid));
        assert_eq!(
            crate::testutil::printf_count("rpc denied, error=1\n"),
            before + 1
        );
        let before = crate::testutil::printf_count("rpc denied, status=1\n");
        let failed = [
            &txdr_unsigned(xid).to_ne_bytes()[..],
            &words(&[RPC_REPLY, 0, 0, 0, 1]),
        ]
        .concat();
        assert!(!krpc_reply_ok(&pkt_chain(&[&failed]), 24, xid));
        assert_eq!(
            crate::testutil::printf_count("rpc denied, status=1\n"),
            before + 1
        );
        // A 16-byte accepted reply passes the loop; the strip refuses it.
        let short = &ok[..16];
        let m = pkt_chain(&[short]);
        assert!(krpc_reply_ok(&m, 16, xid));
        assert_eq!(strip_reply(m).err(), Some(Errno::EBADRPC));
    }

    /// A flat decoder of the same wire: the strings and the address the C's rules keep, or
    /// `None` where it refuses.
    fn flat_decode(b: &[u8], want: usize) -> Option<(Vec<u8>, [u8; 4], usize)> {
        let len = i32::from_be_bytes(b.get(..4)?.try_into().ok()?);
        if len < 0 || len > i32::MAX - 7 {
            return None;
        }
        let padded = nfsm_rndup(len as usize);
        let s = b.get(4..4 + padded)?;
        let kept = s[..(len as usize).min(want)].to_vec();
        let at = 4 + padded;
        let a = b.get(at..at + 20)?;
        let w = |i: usize| u32::from_be_bytes(a[4 * i..4 * i + 4].try_into().unwrap_or([0; 4]));
        let addr = if w(0) == 1 {
            [w(1) as u8, w(2) as u8, w(3) as u8, w(4) as u8]
        } else {
            [0; 4]
        };
        Some((kept, addr, at + 20))
    }

    #[test]
    fn property_the_decoders_equal_a_flat_decoder_over_any_split() {
        let _g = crate::net::mbuf::tests::setup();
        let mut rng = TestRng(0x6b72_7063_5f73_7472);
        for case in 0..2000 {
            // A string of 0..40 bytes (its length word sometimes wrong), an address of
            // either type, sometimes cut short.
            let n = rng.below(41);
            let mut b = xstr(&rng.bytes(n));
            match rng.below(8) {
                0 => b[..4].copy_from_slice(&(rng.next_u64() as u32).to_be_bytes()),
                1 => b[..4].copy_from_slice(&((n + rng.below(9)) as u32).to_be_bytes()),
                _ => {}
            }
            let atype = if rng.below(4) == 0 { 2 } else { 1 };
            b.extend_from_slice(&words(&[
                atype,
                rng.next_u64() as u32,
                rng.next_u64() as u32,
                rng.next_u64() as u32,
                rng.next_u64() as u32,
            ]));
            if rng.below(4) == 0 {
                b.truncate(rng.below(b.len() + 1));
            }
            let want = rng.below(48);
            let expect = flat_decode(&b, want);

            let segs = rng.split(&b);
            let m = test_chain(&segs);
            let mut r = XdrReader::new(&m);
            let mut buf = [0u8; 48];
            let mut len = want;
            let got = xdr_string_decode(&mut r, &mut buf, &mut len)
                .and_then(|()| xdr_inaddr_decode(&mut r))
                .map(|a| (buf[..len].to_vec(), a.s_addr.to_ne_bytes(), r.offset()));
            assert_eq!(got.ok(), expect, "case {case}: {b:?} want {want}");
        }
    }

    #[test]
    fn property_the_strip_never_reads_past_the_chain() {
        let _g = crate::net::mbuf::tests::setup();
        let mut rng = TestRng(0x7374_7269_705f_7270);
        for case in 0..2000 {
            let authtype = if rng.below(3) == 0 { 0 } else { 1 };
            let authlen = match rng.below(4) {
                0 => rng.next_u64() as u32,
                _ => rng.below(420) as u32,
            };
            let mut b = words(&[1, RPC_REPLY, 0, authtype, authlen, 0]);
            let extra = rng.below(440);
            b.extend(rng.bytes(extra));
            if rng.below(2) == 0 {
                b.truncate(rng.below(b.len() + 1));
            }
            let total = b.len();
            let expect = if total < 24 {
                None
            } else if authtype == 0 {
                Some(24)
            } else {
                let a = authlen as i32;
                if a < 0 || a as usize > RPCAUTH_MAXSIZ {
                    None
                } else {
                    let l = 24 + ((a + 3) & !3) as usize;
                    (l <= total).then_some(l)
                }
            };
            let segs = rng.split(&b);
            let got = strip_reply(pkt_chain(&segs)).ok().map(|m| test_bytes(&m));
            assert_eq!(
                got,
                expect.map(|l| b[l..].to_vec()),
                "case {case}: authlen {authlen} total {total}"
            );
        }
    }

    /// The `getport` answers of a portmapper: `port` for every program.
    fn portmapper(port: u32) -> impl FnMut(&Seen) -> Vec<Vec<u8>> + Send + 'static {
        move |s| vec![accepted(s.xid(), &words(&[port]))]
    }

    #[test]
    fn a_portmap_call_carries_the_cs_header_from_a_reserved_port() {
        let (_g, p, wire) = setup(b"krpc1", 1);
        let peer = Peer::start(wire, [10, 77, 1, 2], portmapper(2049));
        let mut s = sin([10, 77, 1, 2], 0);
        // The portmapper's own port needs no call.
        assert_eq!(
            krpc_portmap(p, &mut s, PMAPPROG, PMAPVERS),
            Ok(PMAPPORT.to_be())
        );
        assert_eq!(
            krpc_portmap(p, &mut s, RPCPROG_MNT_TEST, 3),
            Ok(2049u16.to_be())
        );
        assert_eq!(s.sin_port, PMAPPORT.to_be());
        let seen = peer.stop();
        assert_eq!(seen.len(), 1);
        let c = &seen[0];
        assert_eq!((c.dst, c.dport), ([10, 77, 1, 2], PMAPPORT));
        // The reserved port: IP_PORTRANGE_LOW's range (600..1023), not the default's.
        assert!((600..1024).contains(&c.sport), "port {}", c.sport);
        // xid, CALL, RPC version 2, the program, version and procedure, AUTH_UNIX of 20
        // zero bytes (root), an AUTH_NULL verifier; then sdata.
        assert_eq!(
            c.payload[4..60],
            words(&[
                0,
                2,
                PMAPPROG,
                PMAPVERS,
                PMAPPROC_GETPORT,
                1,
                20,
                0,
                0,
                0,
                0,
                0,
                0,
                0
            ])[..]
        );
        assert_eq!(
            c.body(),
            words(&[RPCPROG_MNT_TEST, 3, IPPROTO_UDP as u32, 0])
        );
    }

    /// The mount daemon's program number (rpcv2.h's `RPCPROG_MNT`), as the tests write it.
    const RPCPROG_MNT_TEST: u32 = 100005;

    #[test]
    fn the_bind_is_proc0s_whoever_calls() {
        let (_g, _p, wire) = setup(b"krpc2", 2);
        let user = user_thread(1000);
        assert!(crate::proc::suser(user).is_err());
        let peer = Peer::start(wire, [10, 77, 2, 2], portmapper(635));
        let mut s = sin([10, 77, 2, 2], 0);
        assert_eq!(
            krpc_portmap(user, &mut s, RPCPROG_MNT_TEST, 1),
            Ok(635u16.to_be())
        );
        let seen = peer.stop();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].sport < 1024, "port {}", seen[0].sport);
    }

    #[test]
    fn a_lost_request_is_sent_again_with_its_xid() {
        let (_g, p, wire) = setup(b"krpc3", 3);
        let mut first = true;
        let peer = Peer::start(wire, [10, 77, 3, 2], move |s| {
            if core::mem::replace(&mut first, false) {
                return vec![];
            }
            vec![accepted(s.xid(), b"done")]
        });
        let mut w = XdrWriter::reqhead(0);
        w.u32(42);
        let r = krpc_call(
            p,
            &sin([10, 77, 3, 2], 700),
            300_001,
            1,
            9,
            w.into_chain(),
            None,
            5,
        );
        assert_eq!(r.map(|m| test_bytes(&m)), Ok(b"done".to_vec()));
        let seen = peer.stop();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].xid(), seen[1].xid());
        assert_eq!(seen[0].payload, seen[1].payload);
        assert_eq!(seen[0].call(), (300_001, 1, 9));
        assert_eq!(seen[0].body(), words(&[42]));
    }

    #[test]
    fn no_answer_is_etimedout_when_the_retries_run_out() {
        let (_g, p, wire) = setup(b"krpc4", 4);
        let peer = Peer::start(wire, [10, 77, 4, 2], |_| vec![]);
        let w = XdrWriter::reqhead(0);
        let r = krpc_call(
            p,
            &sin([10, 77, 4, 2], 700),
            300_002,
            1,
            1,
            w.into_chain(),
            None,
            1,
        );
        assert_eq!(r.err(), Some(Errno::ETIMEDOUT));
        assert_eq!(peer.stop().len(), 1);
        // Another family is refused before a socket is made.
        let mut s = sin([10, 77, 4, 2], 700);
        s.sin_family = 24;
        let w = XdrWriter::reqhead(0);
        let r = krpc_call(p, &s, 1, 1, 1, w.into_chain(), None, 1);
        assert_eq!(r.err(), Some(Errno::EAFNOSUPPORT));
    }

    #[test]
    fn stray_and_denied_replies_are_skipped_for_the_answer() {
        let (_g, p, wire) = setup(b"krpc5", 5);
        let peer = Peer::start(wire, [10, 77, 5, 2], |s| {
            let x = s.xid();
            let other = x ^ 0x0100_0000;
            let mut call = accepted(x, b"no");
            call[4..8].copy_from_slice(&0u32.to_be_bytes());
            vec![
                accepted(other, b"stray"),
                call,
                [&x.to_ne_bytes()[..], &words(&[RPC_REPLY, 1, 7])].concat(),
                [&x.to_ne_bytes()[..], &words(&[RPC_REPLY, 0, 0, 0, 3])].concat(),
                vec![1, 2, 3],
                accepted(x, b"yes!"),
            ]
        });
        let denied = crate::testutil::printf_count("rpc denied, error=7\n");
        let failed = crate::testutil::printf_count("rpc denied, status=3\n");
        let w = XdrWriter::reqhead(0);
        let r = krpc_call(
            p,
            &sin([10, 77, 5, 2], 700),
            300_003,
            1,
            1,
            w.into_chain(),
            None,
            3,
        );
        assert_eq!(r.map(|m| test_bytes(&m)), Ok(b"yes!".to_vec()));
        assert_eq!(peer.stop().len(), 1);
        assert_eq!(
            crate::testutil::printf_count("rpc denied, error=7\n"),
            denied + 1
        );
        assert_eq!(
            crate::testutil::printf_count("rpc denied, status=3\n"),
            failed + 1
        );
    }

    #[test]
    fn a_broadcast_call_reports_its_answers_sender() {
        let (_g, p, wire) = setup(b"krpc6", 6);
        let peer = Peer::start(wire, [10, 77, 6, 9], |s| vec![accepted(s.xid(), b"hi")]);
        let mut from = None;
        let w = XdrWriter::reqhead(0);
        let r = krpc_call(
            p,
            &sin([10, 77, 6, 255], PMAPPORT),
            PMAPPROG,
            PMAPVERS,
            PMAPPROC_CALLIT,
            w.into_chain(),
            Some(&mut from),
            2,
        );
        assert_eq!(r.map(|m| test_bytes(&m)), Ok(b"hi".to_vec()));
        let from = from.expect("the sender");
        let sa: SockaddrIn = hal::mem::pod::at(from.data(), 0).expect("a sockaddr_in");
        assert_eq!(sa.sin_addr.s_addr.to_ne_bytes(), [10, 77, 6, 9]);
        assert_eq!(sa.sin_port, PMAPPORT.to_be());
        let seen = peer.stop();
        assert_eq!(seen[0].dst, [10, 77, 6, 255]);
        // Without SO_BROADCAST (no `from_p`) the broadcast is refused by ip_output.
        let w = XdrWriter::reqhead(0);
        let r = krpc_call(
            p,
            &sin([10, 77, 6, 255], PMAPPORT),
            1,
            1,
            1,
            w.into_chain(),
            None,
            1,
        );
        assert_eq!(r.err(), Some(Errno::EACCES));
    }

    #[test]
    fn xids_are_drawn_from_the_generator() {
        crate::testutil::install();
        let a = krpc_get_xid();
        let b = krpc_get_xid();
        assert_ne!(a, b);
        assert!(KRPC_XID_CTX.lock().is_some());
    }
}
/* </TESTS> */
