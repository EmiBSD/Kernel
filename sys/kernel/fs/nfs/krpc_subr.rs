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
//!   on success. A reply freed on an error path after it arrived is not leaked as in the C.
//!   `retries` of -1 loops "forever" as in C.
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

use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;
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
            let a = sa.sin_addr.s_addr;
            let b = a.to_ne_bytes();
            printf(format_args!(
                "RPC timeout for server {}.{}.{}.{} (0x{:x}) prog {}\n",
                b[0],
                b[1],
                b[2],
                b[3],
                u32::from_be(a),
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
    if slen < 0 || slen > i32::MAX - 3 - 4 {
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
/* </CODE> */
