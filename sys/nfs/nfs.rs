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
 * Copyright (c) 1989, 1993, 1995
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * Rick Macklem at The University of Guelph.
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
 *	@(#)nfs.h	8.4 (Berkeley) 5/1/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<nfs/nfs.h>`, the server's half (D-5; the client's, the request `struct nfsreq`, the RTO
//! timers, `nfs_niothreads` and the macros of the timeouts, is the kernel's,
//! `kernel::fs::nfs::nfs`, since N6.7a5): the `nfssvc(2)` arguments, the server socket
//! (`struct nfssvc_sock`), the server thread (`struct nfsd`) and the server's request
//! descriptor (`struct nfsrv_descript`).
//!
//! Upstream: sys/nfs/nfs.h @ 3ce1f3f79392
//!
//! Server sockets and server threads are pool or `malloc(9)` items that live until the C
//! frees them; they are handed around as `&'static T` with `Cell` members, the
//! `docs/C_TO_RUST.md` idiom for objects the C mutates through shared pointers, and they link
//! into their lists through `queue.rs` entries ([`NsChain`], [`NfsdChain`]). The server's
//! request descriptor is owned by the `nfsd` that dequeued it and is passed as
//! `&mut NfsrvDescript`, because its dissection cursor (`nd_md`, `nd_dpos`) is what
//! `nfsd_dissect` reads through (`nfsm_subs.rs`).
//!
//! ## Deviations
//! - `NFS_SRVMAXDATA` is a function with a lowercase name.
//! - `struct nfsstats` and the constants userland reads (the tunables, the `nfssvc(2)` flags,
//!   the `vfs.nfs` identifiers) are `hal::sys::nfs_nfs`'s image, re-exported here; the
//!   counters are the kernel's (`kernel::fs::nfs::nfs::NfsCounters`, the instance
//!   `nfs_subs.rs`'s `NFSSTATS`, N6.7a0).
//! - `struct nfsd_args` and `struct nfsd_srvargs` (the `nfssvc(2)` ABI) name the holes the
//!   C compiler leaves on LP64 (`_pad*`); user addresses (`name`, `nsd_authstr`,
//!   `nsd_verfstr`) and the kernel's `nsd_nfsd` cookie are `usize`.
//! - `union nethostaddr` is [`Nethostaddr`] with both members side by side; the flag that
//!   says which one is valid stays the C's (`RC_INETADDR`/`RC_NAM`).
//! - Procedure numbers (`nd_procnum`) are `usize` (see `nfsproto.rs`).
//! - The `struct proc *` of a server thread (`nfsd_procp`) is a `Cell<*const Proc>`, the
//!   long-lived-pointer idiom; `nfsd_nd` is a `NonNull` to the descriptor the thread owns.
//! - The globals the header declares for other files are declared here, each with the file
//!   that defines it in C: `nfsd_head` and `nfsd_head_flag` (`nfs_syscalls.c`).

use core::cell::Cell;
use core::ptr::{self, NonNull};
use core::sync::atomic::AtomicI32;

use crate::nfs::nfsproto::{NFS_MAXDATA, NFS_MAXDGRAMDATA, NFS_V2MAXDATA};
use crate::queue_adapter;
use crate::sys::file::FileSlot;
use crate::sys::mbuf::Mbuf;
use crate::sys::proc::Proc;
use crate::sys::queue::{TailqEntry, TailqHead};
use crate::sys::socketvar::Socket;
use crate::sys::time::Timeval;
use crate::sys::types::Uid;
use crate::sys::ucred::{UcredCell, Xucred};

pub use hal::sys::nfs_nfs::{
    FS_NFS_NAMES, NFS_DEFRAHEAD, NFS_DIRBLKSIZ, NFS_MAXASYNCDAEMON, NFS_MAXATTRTIMO, NFS_MAXGRPS,
    NFS_MAXID, NFS_MAXRAHEAD, NFS_MAXREXMIT, NFS_MINATTRTIMO, NFS_NFSSTATS, NFS_NIOTHREADS,
    NFS_READDIRBLKSIZ, NFS_READDIRSIZE, NFS_RETRANS, NFS_RSIZE, NFS_TICKINTVL, NFS_TIMEOUTMUL,
    NFS_WSIZE, NFSSVC_ADDSOCK, NFSSVC_NFSD, Nfsstats,
};

/// `SLP_VALID`: connection is usable; bits for `ns_flag`.
pub const SLP_VALID: i32 = 0x01;
/// `SLP_DOREC`: receive operation required.
pub const SLP_DOREC: i32 = 0x02;
/// `SLP_NEEDQ`: connection has data to queue from socket.
pub const SLP_NEEDQ: i32 = 0x04;
/// `SLP_DISCONN`: connection is closed.
pub const SLP_DISCONN: i32 = 0x08;
/// `SLP_GETSTREAM`: extracting RPC from TCP connection.
pub const SLP_GETSTREAM: i32 = 0x10;
/// `SLP_LASTFRAG`: last fragment received on TCP connection.
pub const SLP_LASTFRAG: i32 = 0x20;
/// `SLP_ALLFLAGS`: convenience.
pub const SLP_ALLFLAGS: i32 = 0xff;

/// `NFSD_WAITING`: bits for `nfsd_flag`.
pub const NFSD_WAITING: i32 = 0x01;
/// `NFSD_REQINPROG`.
pub const NFSD_REQINPROG: i32 = 0x02;

/// `ND_NFSV3`: bits for `nd_flag`.
pub const ND_NFSV3: i32 = 0x08;

/// `NFSD_CHECKSLP`: a bit of `nfsd_head_flag`.
pub const NFSD_CHECKSLP: i32 = 0x01;

/// `struct nfsd_args`: the `NFSSVC_ADDSOCK` argument of `nfssvc(2)`. Not that anyone besides
/// nfsd(8) should ever use it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NfsdArgs {
    /// `sock`: socket to serve.
    pub sock: i32,
    /// The hole before `name`.
    pub _pad0: i32,
    /// `name`: client addr for connection based sockets (a user address).
    pub name: usize,
    /// `namelen`: length of name.
    pub namelen: i32,
    /// The hole at the end.
    pub _pad1: i32,
}

// SAFETY: `#[repr(C)]` integers with the holes named, no implicit padding.
unsafe impl crate::machine::copy::AbiPod for NfsdArgs {}

/// `struct nfsd_srvargs`: the `NFSSVC_NFSD` argument of `nfssvc(2)`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NfsdSrvargs {
    /// `nsd_nfsd`: pointer to in kernel nfsd struct (opaque to userland).
    pub nsd_nfsd: usize,
    /// `nsd_uid`: effective uid mapped to cred.
    pub nsd_uid: Uid,
    /// `nsd_haddr`: IP address of client.
    pub nsd_haddr: u32,
    /// `nsd_cr`: cred. uid maps to.
    pub nsd_cr: Xucred,
    /// `nsd_authlen`: length of auth string (ret).
    pub nsd_authlen: i32,
    /// `nsd_authstr`: auth string (ret), a user address.
    pub nsd_authstr: usize,
    /// `nsd_verflen`: and the verifier.
    pub nsd_verflen: i32,
    /// The hole before `nsd_verfstr`.
    pub _pad0: i32,
    /// `nsd_verfstr`, a user address.
    pub nsd_verfstr: usize,
    /// `nsd_timestamp`: timestamp from verifier.
    pub nsd_timestamp: Timeval,
    /// `nsd_ttl`: credential ttl (sec).
    pub nsd_ttl: u32,
    /// The hole at the end.
    pub _pad1: u32,
}

impl NfsdSrvargs {
    /// `sizeof(struct nfsd_srvargs)` on LP64.
    pub const SIZE: usize = size_of::<NfsdSrvargs>();
}

/// `union nethostaddr`: network address hash list element. Which member is valid is the
/// holder's flag (`RC_INETADDR` or `RC_NAM` in `nfsrvcache.rs`).
#[derive(Clone, Copy, Default)]
pub struct Nethostaddr {
    /// `had_inetaddr`.
    pub had_inetaddr: u32,
    /// `had_nam`.
    pub had_nam: Option<&'static Mbuf>,
}

/// `struct nfssvc_sock`: a socket the server receives requests on.
pub struct NfssvcSock {
    /// `ns_chain`: list of all `nfssvc_sock`s.
    pub ns_chain: TailqEntry<NfssvcSock>,
    /// `ns_fp`: fp from the...
    pub ns_fp: Cell<FileSlot>,
    /// `ns_so`: ...socket this struct wraps.
    pub ns_so: Cell<Option<&'static Socket>>,
    /// `ns_nam`: `MT_SONAME` of client.
    pub ns_nam: Cell<Option<&'static Mbuf>>,
    /// `ns_raw`: head of unpeeked mbufs.
    pub ns_raw: Cell<Option<&'static Mbuf>>,
    /// `ns_rawend`: tail of unpeeked mbufs.
    pub ns_rawend: Cell<Option<&'static Mbuf>>,
    /// `ns_rec`: queued RPC records.
    pub ns_rec: Cell<Option<&'static Mbuf>>,
    /// `ns_recend`: last queued RPC record.
    pub ns_recend: Cell<Option<&'static Mbuf>>,
    /// `ns_frag`: end of record fragment.
    pub ns_frag: Cell<Option<&'static Mbuf>>,
    /// `ns_flag`: socket status flags (`SLP_*`).
    pub ns_flag: Cell<i32>,
    /// `ns_solock`: lock for connected socket.
    pub ns_solock: Cell<i32>,
    /// `ns_cc`: actual chars queued.
    pub ns_cc: Cell<i32>,
    /// `ns_reclen`: length of first queued record.
    pub ns_reclen: Cell<i32>,
    /// `ns_sref`: # of refs to this struct.
    pub ns_sref: Cell<u32>,
}

// SAFETY: server sockets are changed under the kernel lock, as in C.
unsafe impl Sync for NfssvcSock {}

impl NfssvcSock {
    /// A socket with every member cleared (`malloc(M_ZERO)`).
    pub const fn new() -> Self {
        Self {
            ns_chain: TailqEntry::new(),
            ns_fp: Cell::new(FileSlot::new()),
            ns_so: Cell::new(None),
            ns_nam: Cell::new(None),
            ns_raw: Cell::new(None),
            ns_rawend: Cell::new(None),
            ns_rec: Cell::new(None),
            ns_recend: Cell::new(None),
            ns_frag: Cell::new(None),
            ns_flag: Cell::new(0),
            ns_solock: Cell::new(0),
            ns_cc: Cell::new(0),
            ns_reclen: Cell::new(0),
            ns_sref: Cell::new(0),
        }
    }
}

impl Default for NfssvcSock {
    fn default() -> Self {
        Self::new()
    }
}

queue_adapter!(
    /// `TAILQ_HEAD(, nfssvc_sock)`: the server's sockets, through `ns_chain`.
    pub NsChain: NfssvcSock, ns_chain => TailqEntry<NfssvcSock>
);

/// `struct nfsd`: one of these structures is allocated for each nfsd.
pub struct Nfsd {
    /// `nfsd_chain`: list of all nfsd's.
    pub nfsd_chain: TailqEntry<Nfsd>,
    /// `nfsd_flag`: `NFSD_` flags.
    pub nfsd_flag: Cell<i32>,
    /// `nfsd_slp`: current socket.
    pub nfsd_slp: Cell<Option<&'static NfssvcSock>>,
    /// `nfsd_procp`: proc ptr.
    pub nfsd_procp: Cell<*const Proc>,
    /// `nfsd_nd`: associated `nfsrv_descript`, owned by this nfsd while it serves it.
    pub nfsd_nd: Cell<Option<NonNull<NfsrvDescript>>>,
}

// SAFETY: server threads are changed under the kernel lock, as in C.
unsafe impl Sync for Nfsd {}

impl Nfsd {
    /// An nfsd with every member cleared (`malloc(M_ZERO)`).
    pub const fn new() -> Self {
        Self {
            nfsd_chain: TailqEntry::new(),
            nfsd_flag: Cell::new(0),
            nfsd_slp: Cell::new(None),
            nfsd_procp: Cell::new(ptr::null()),
            nfsd_nd: Cell::new(None),
        }
    }
}

impl Default for Nfsd {
    fn default() -> Self {
        Self::new()
    }
}

queue_adapter!(
    /// `TAILQ_HEAD(nfsdhead, nfsd)`: every nfsd, through `nfsd_chain`.
    pub NfsdChain: Nfsd, nfsd_chain => TailqEntry<Nfsd>
);

/// `struct nfsdhead`: the list of nfsds.
pub struct Nfsdhead(pub TailqHead<NfsdChain>);

// SAFETY: the list is changed under the kernel lock, as in C.
unsafe impl Sync for Nfsdhead {}

/// `struct nfsrv_descript`: used by the server for describing each request. The nfsd that
/// dequeued it owns it (`&mut`); `nd_md`/`nd_dpos` are the dissection cursor of
/// `nfsd_dissect` and friends (`nfsm_subs.rs`).
pub struct NfsrvDescript {
    /// `nd_mrep`: request mbuf list.
    pub nd_mrep: Option<&'static Mbuf>,
    /// `nd_md`: current dissect mbuf.
    pub nd_md: Option<&'static Mbuf>,
    /// `nd_nam`: and socket addr.
    pub nd_nam: Option<&'static Mbuf>,
    /// `nd_nam2`: return socket addr.
    pub nd_nam2: Option<&'static Mbuf>,
    /// `nd_dpos`: current dissect pos, inside `nd_md`'s data.
    pub nd_dpos: *mut u8,
    /// `nd_procnum`: RPC #.
    pub nd_procnum: usize,
    /// `nd_flag`: `ND_*`.
    pub nd_flag: i32,
    /// `nd_repstat`: reply status (an NFS status number).
    pub nd_repstat: i32,
    /// `nd_retxid`: reply xid, in host order (`nfs_getreq` converts it, `nfs_rephead` converts back).
    pub nd_retxid: u32,
    /// `nd_cr`: credentials, the request's own, updated in place (`nfsrv_fhtovp`'s
    /// anonymous mapping). `nfs_getreq` fills it; `nfssvc_nfsd` takes it out for the server
    /// procedure's call, which receives it as an argument of its own (`NfsrvProc`), and puts it
    /// back after: `None` during the call, so a read of the member there finds no credential.
    /// [`NfsrvDescript::new`] stores a placeholder with root's ids, which no one reads: a
    /// request without an AUTH_UNIX credential reaches only `nfsrv_null` and `nfsrv_noop`,
    /// which read none, as in the C (nfs_socket.c:1472-1496, :1551 run before :1520). Never
    /// make it `None` there: every NULL ping would then panic at the take.
    pub nd_cr: Option<UcredCell>,
}

impl NfsrvDescript {
    /// A descriptor with every member cleared.
    pub const fn new() -> Self {
        Self {
            nd_mrep: None,
            nd_md: None,
            nd_nam: None,
            nd_nam2: None,
            nd_dpos: ptr::null_mut(),
            nd_procnum: 0,
            nd_flag: 0,
            nd_repstat: 0,
            nd_retxid: 0,
            nd_cr: Some(UcredCell::new()),
        }
    }
}

impl Default for NfsrvDescript {
    fn default() -> Self {
        Self::new()
    }
}

/// `nfsd_head`: every nfsd (defined in `nfs_syscalls.c`).
pub static NFSD_HEAD: Nfsdhead = Nfsdhead(TailqHead::new());

/// `nfsd_head_flag`: `NFSD_CHECKSLP` (defined in `nfs_syscalls.c`).
pub static NFSD_HEAD_FLAG: AtomicI32 = AtomicI32::new(0);

/// `NFS_SRVMAXDATA(n)`: the largest data a server reply may carry.
pub fn nfs_srvmaxdata(n: &NfsrvDescript) -> usize {
    if n.nd_flag & ND_NFSV3 != 0 {
        if n.nd_nam2.is_some() {
            NFS_MAXDGRAMDATA
        } else {
            NFS_MAXDATA
        }
    } else {
        NFS_V2MAXDATA
    }
}

const _: () = {
    assert!(size_of::<NfsdArgs>() == 24);
    assert!(NfsdSrvargs::SIZE == 144);
    assert!(core::mem::offset_of!(NfsdSrvargs, nsd_authstr) == 96);
    assert!(core::mem::offset_of!(NfsdSrvargs, nsd_timestamp) == 120);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for `<nfs/nfs.h>`: the statistics snapshot, the helpers and, against the C
    // header, the constants.

    use super::*;

    #[test]
    fn server_reply_limits() {
        let mut nd = NfsrvDescript::new();
        assert_eq!(nfs_srvmaxdata(&nd), NFS_V2MAXDATA);
        nd.nd_flag = ND_NFSV3;
        assert_eq!(nfs_srvmaxdata(&nd), NFS_MAXDATA);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn constants_match_the_c_header() {
        let defs = crate::reftest::defines("sys/nfs/nfs.h");
        let ours: &[(&str, i64)] = &[
            ("NFS_TICKINTVL", NFS_TICKINTVL.into()),
            ("NFS_TIMEOUTMUL", NFS_TIMEOUTMUL.into()),
            ("NFS_MAXREXMIT", NFS_MAXREXMIT.into()),
            ("NFS_RETRANS", NFS_RETRANS.into()),
            ("NFS_MAXGRPS", NFS_MAXGRPS.into()),
            ("NFS_MINATTRTIMO", NFS_MINATTRTIMO.into()),
            ("NFS_MAXATTRTIMO", NFS_MAXATTRTIMO.into()),
            ("NFS_WSIZE", NFS_WSIZE.into()),
            ("NFS_RSIZE", NFS_RSIZE.into()),
            ("NFS_READDIRSIZE", NFS_READDIRSIZE.into()),
            ("NFS_DEFRAHEAD", NFS_DEFRAHEAD.into()),
            ("NFS_MAXRAHEAD", NFS_MAXRAHEAD.into()),
            ("NFS_MAXASYNCDAEMON", NFS_MAXASYNCDAEMON.into()),
            ("NFS_DIRBLKSIZ", NFS_DIRBLKSIZ.into()),
            ("NFS_READDIRBLKSIZ", NFS_READDIRBLKSIZ.into()),
            ("NFSSVC_NFSD", NFSSVC_NFSD.into()),
            ("NFSSVC_ADDSOCK", NFSSVC_ADDSOCK.into()),
            ("NFS_NFSSTATS", NFS_NFSSTATS.into()),
            ("NFS_NIOTHREADS", NFS_NIOTHREADS.into()),
            ("NFS_MAXID", NFS_MAXID.into()),
            ("SLP_VALID", SLP_VALID.into()),
            ("SLP_DOREC", SLP_DOREC.into()),
            ("SLP_NEEDQ", SLP_NEEDQ.into()),
            ("SLP_DISCONN", SLP_DISCONN.into()),
            ("SLP_GETSTREAM", SLP_GETSTREAM.into()),
            ("SLP_LASTFRAG", SLP_LASTFRAG.into()),
            ("SLP_ALLFLAGS", SLP_ALLFLAGS.into()),
            ("NFSD_WAITING", NFSD_WAITING.into()),
            ("NFSD_REQINPROG", NFSD_REQINPROG.into()),
            ("ND_NFSV3", ND_NFSV3.into()),
            ("NFSD_CHECKSLP", NFSD_CHECKSLP.into()),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
