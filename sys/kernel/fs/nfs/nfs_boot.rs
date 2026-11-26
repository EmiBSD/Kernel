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
 * Copyright (c) 1995 Adam Glass, Gordon Ross
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. The name of the authors may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHORS ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `nfs/nfs_boot.c` in the kernel: support for NFS diskless booting, specifically getting
//! information about where to boot from, what pathnames, etc. This implementation uses
//! RARP and the bootparam RPC. We are forced to implement RPC anyway (to get file handles)
//! so we might as well take advantage of it for bootparam, too.
//!
//! The diskless boot process goes like this:
//! 1. Use RARP to get our interface address;
//! 2. Use RPC/bootparam/whoami to get our hostname, our IP domain name, and a gateway
//!    address;
//! 3. Use RPC/bootparam/getfile to get the server:pathname for our root;
//! 4. Use RPC/mountd to get the root file handle;
//! 5. Use RPC/bootparam/getfile to get the swap path;
//! 6. Use RPC/mountd to get the swap file handle.
//!
//! (This happens to be the way Sun does it too.)
//!
//! Upstream: sys/nfs/nfs_boot.c @ 3ce1f3f79392
//! LZ: sys/nfs/nfs_boot.rs@e8249449d905
//!
//! The file is compiled only with `option NFSCLIENT` (`sys/conf/files`): the module is behind
//! the `nfsclient` feature.
//!
//! ## Deviations
//! - The C's `#if !defined(NFSCLIENT) || (NETHER == 0)` stubs (`nfs_boot_init` panics,
//!   `nfs_boot_getfh` is `EOPNOTSUPP`) are not written: the file is compiled only with
//!   `NFSCLIENT` (`sys/conf/files`), and ethernet is always built here, so they are the C's
//!   code for a kernel that never compiles them (LZ kept them behind `not(nfsclient)`).
//! - `nfsbootdevname` (a `char *` set by `setroot()` in `subr_disk.c`) is [`NFSBOOTDEVNAME`],
//!   a copy of the name, with a setter, [`set_nfsbootdevname`].
//! - The C's `hostname`/`hostnamelen`/`domainname`/`domainnamelen` are `kernel::base::sysctl`'s, the
//!   names read and written under their locks.
//! - `bp_whoami` returns the gateway address instead of storing it through `gw_ip`;
//!   `bp_getfile`'s `serv_name` and `pathname` are slices (`MNAMELEN` and `MAXPATHLEN`
//!   bytes); `md_mount` also takes the buffer the file handle goes to (the C reaches it
//!   through `argp->fh`, which `nfs_boot_getfh` sets to the same buffer); `nfs_boot_getfh`
//!   takes the calling thread, for `krpc_call`'s socket.
//! - `md_mount`'s `error` from the mount daemon (an errno number on the wire) that names no
//!   `Errno` is `EIO`. A version 3 handle's length is compared unsigned with `NFSX_V3FHMAX`
//!   (the C's `int` compare lets a length of 2^31 or more through to a negative `bcopy`).
//! - The C's contiguity tests (`m->m_len < n`, `m_pullup`) are tests of the chain's length:
//!   the cursor copies across mbufs (a mount reply whose first mbuf is empty after the RPC
//!   header's strip is read, not refused). The sender's address of the WHOAMI reply is read
//!   only when the name holds it (the C reads it unchecked).
//! - The remote path (`host:path`) is never written past `ndm_host`: with a server name of
//!   `MNAMELEN - 1` bytes the C stores the `:` in the last byte and the NUL one byte past the
//!   array (the handle's first byte); here the `:` is stored and the NUL is not, so the name
//!   reads as the C's does when the handle's first byte was 0 (readers copy at most
//!   `MNAMELEN` bytes), and the handle is kept.
//! - `NFS_BOOT_OPTIONS` and `NFS_BOOT_RWSIZE` (kernel config options, not defined in
//!   `GENERIC`) are not configured.
//! - The interface's first Internet address is read under the shared net lock (the C walks
//!   `if_addrlist` with no lock, at boot).
//!
//! ## Redesign
//! - LZ handed `&'static Mbuf` chains to `krpc_subr`'s decoders, each `m_pullup`ing and
//!   `m_adj`ing the head, wrote its requests through raw mbuf views and its interface requests
//!   through the raw `ifioctl` (`*mut c_void` data, 3 `unsafe`), and kept `nfsbootdevname` in
//!   a `StaticCell`. A request is built with an [`XdrWriter`] and a reply read with one
//!   [`XdrReader`]; the interface requests go through the kernel's `ifioctl` over the
//!   structures' bytes (`hal::mem::pod`); the device name is a `Mutexed` copy; LZ's `Ipv4` printer
//!   is `if_ether`'s `InAddrFmt`. The reply parsers are their own functions
//!   (`bp_whoami_reply`, `bp_getfile_reply`, `md_mount_reply`) so that the host tests drive
//!   them with a chain.
//! - `nfs_boot_getfh` leaves the mount arguments' `addr`, `fh` and `hostname` `NULL`: the C
//!   points them at the members of the same `nfs_dlmount` for `mountnfs`, which reads them in
//!   the kernel and stores `NULL` in `mount_info` (C:nfs/nfs_vfsops.c:655-658); the kernel's
//!   `nfs_mount_diskless` (N6.7a5) passes the members themselves (LZ's form), and a user
//!   address of the kernel's own structure is never made.

use hal::cpu::intr::IPL_NONE;
use hal::sync::mutex::Mutexed;
use hal::sync::netlock::{NETLOCK, NetHeld};
use hal::user::{bytes_of_mut, zeroed};
use sysdefs::base::pod::AbiPod;
use sysdefs::fs::mount::{MNAMELEN, NFSMNT_NFSV3, NfsArgs};
use sysdefs::net::if_::{IFF_UP, IFNAMSIZ, Ifreq};
use sysdefs::net::socket::{AF_INET, SOCK_DGRAM};
use sysdefs::net::sockio::{SIOCAIFADDR, SIOCGIFFLAGS, SIOCSIFFLAGS};
use sysdefs::netinet::in_::{InAddr, SockaddrIn};
use sysdefs::netinet::in_var::InAliasreq;

use super::krpc_subr::{
    BOOTPARAM_GETFILE, BOOTPARAM_PROG, BOOTPARAM_VERS, BOOTPARAM_WHOAMI, PMAPPORT, PMAPPROC_CALLIT,
    PMAPPROG, PMAPVERS, XDR_INADDR_SIZE, krpc_call, krpc_portmap, xdr_inaddr_decode,
    xdr_inaddr_encode, xdr_string_decode, xdr_string_encode,
};
use super::nfsdiskless::{NfsDiskless, NfsDlmount};
use super::nfsm_subs::{XdrReader, XdrWriter};
use super::nfsproto::{NFS_PROG, NFS_VER2, NFS_VER3, NFSX_V2FH, NFSX_V3FHMAX};
use super::rpcv2::{RPCMNT_MOUNT, RPCPROG_MNT};
use crate::assert::kpanic;
use crate::base::prf::{Str, printf};
use crate::base::sysctl::{
    DOMAINNAMELEN, HOSTNAMELEN, MAXHOSTNAMELEN, with_domainname_mut, with_hostname,
    with_hostname_mut,
};
use crate::fs::namei::MAXPATHLEN;
use crate::net::if_::{if_put, if_unit, ifioctl};
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{Mbuf, MbufBox};
use crate::net::socket::{soclose, socreate};
use crate::netinet::if_ether::{InAddrFmt, revarpwhoami};
use crate::netinet::in_var::InIfaddr;
use crate::proc::{ProcGlue, Thread};
use sysdefs::base::errno::Errno;

/// `nfsbootdevname`: the name of the root device when booting from the network (a NUL-padded
/// copy of `rootdv->dv_xname`), set by `setroot()` through [`set_nfsbootdevname`].
static NFSBOOTDEVNAME: Mutexed<Option<[u8; IFNAMSIZ]>> = Mutexed::new(IPL_NONE, None);

/// `nfsbootdevname = rootdv->dv_xname` (`setroot()` in `subr_disk.c`): remembers the
/// network device the root file system is on (its name up to a NUL, cut to `IFNAMSIZ - 1`).
pub fn set_nfsbootdevname(name: &[u8]) {
    let mut n = [0u8; IFNAMSIZ];
    let len = cstr(name).len().min(IFNAMSIZ - 1);
    n[..len].copy_from_slice(&name[..len]);
    *NFSBOOTDEVNAME.lock() = Some(n);
}

/// The bytes of a NUL-terminated buffer before its NUL.
fn cstr(b: &[u8]) -> &[u8] {
    let len = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    &b[..len]
}

/// The kernel's `ifioctl(so, cmd, (caddr_t)req, procp)` over the request structure's bytes.
fn ifioctl_on<G: ProcGlue, T: AbiPod>(
    so: &crate::net::socket::Socket<G>,
    cmd: u64,
    req: &mut T,
    procp: &Thread<G>,
) -> Result<(), Errno> {
    ifioctl(
        Some(so),
        cmd,
        bytes_of_mut(core::slice::from_mut(req)),
        procp,
    )
}

/// `nfs_boot_init(nd, procp)`: called with an empty `nfs_diskless` to be filled in. Finds an
/// interface, RARPs for its IP address, stuffs it, the implied broadcast address and the
/// netmask into the interface, asks the bootparam server who we are, and records the boot
/// server's address in `nd`. Panics when there is no interface or nobody answers: there is
/// nothing to boot from then. Called with the kernel lock (`if_unit`'s).
///
/// This was moved here from `nfs_vfsops.c` because this procedure would be quite different if
/// someone decides to write (i.e.) a BOOTP version of this file (might not use RARP, etc.).
pub fn nfs_boot_init<G: ProcGlue>(nd: &mut NfsDiskless<G>, procp: &Thread<G>) -> Result<(), Errno> {
    // Find a network interface.
    let devname = *NFSBOOTDEVNAME.lock();
    let Some(ifp) = devname.and_then(|n| if_unit(cstr(&n))) else {
        kpanic(format_args!("nfs_boot: no suitable interface"));
    };

    let xname = ifp.if_xname.get();
    let mut ireq = Ifreq::zeroed();
    ireq.ifr_name = xname;
    printf(format_args!(
        "nfs_boot: using interface {}, with revarp & bootparams\n",
        Str(&xname)
    ));

    // Bring up the interface.
    //
    // Get the old interface flags and or IFF_UP into them; if IFF_UP set blindly, interface
    // selection can be clobbered.
    let so = match socreate(procp, i32::from(AF_INET), SOCK_DGRAM, 0) {
        Ok(so) => so,
        Err(e) => kpanic(format_args!("nfs_boot: socreate, error={}", e.as_i32())),
    };
    if let Err(e) = ifioctl_on(&so, SIOCGIFFLAGS, &mut ireq, procp) {
        kpanic(format_args!("nfs_boot: GIFFLAGS, error={}", e.as_i32()));
    }
    ireq.set_ifr_flags(ireq.ifr_flags() | IFF_UP as i16);
    if let Err(e) = ifioctl_on(&so, SIOCSIFFLAGS, &mut ireq, procp) {
        kpanic(format_args!("nfs_boot: SIFFLAGS, error={}", e.as_i32()));
    }

    // Do RARP for the interface address.
    let Ok(my_ip) = revarpwhoami(ifp) else {
        kpanic(format_args!(
            "reverse arp not answered by rarpd(8) or dhcpd(8)"
        ));
    };
    printf(format_args!("nfs_boot: client_addr={}\n", InAddrFmt(my_ip)));

    // Do enough of ifconfig(8) so that the chosen interface can talk to the servers. (just
    // set the address)
    let mut ifra = InAliasreq {
        ifra_name: xname,
        ifra_addr: SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: AF_INET,
            sin_addr: my_ip,
            ..SockaddrIn::default()
        },
        ..InAliasreq::default()
    };
    if let Err(e) = ifioctl_on(&so, SIOCAIFADDR, &mut ifra, procp) {
        kpanic(format_args!("nfs_boot: set if addr, error={}", e.as_i32()));
    }

    let _ = soclose(&so, 0);
    drop(so);

    let Some(broad) = nfs_boot_broadaddr(ifp) else {
        kpanic(format_args!(
            "nfs_boot: address not configured on {}",
            Str(&xname)
        ));
    };
    if_put(Some(ifp));

    // Get client name and gateway address. RPC: bootparam/whoami. The server address
    // returned by the WHOAMI call is used for all subsequent bootparam RPCs.
    let mut bp_sin = SockaddrIn {
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_family: AF_INET,
        sin_addr: InAddr { s_addr: broad },
        ..SockaddrIn::default()
    };
    HOSTNAMELEN.store(MAXHOSTNAMELEN as i32, core::sync::atomic::Ordering::Relaxed);

    // this returns gateway IP address
    let _gw_ip = match bp_whoami(procp, &mut bp_sin, my_ip) {
        Ok(gw) => gw,
        Err(e) => kpanic(format_args!(
            "nfs_boot: bootparam whoami, error={}",
            e.as_i32()
        )),
    };
    with_hostname(|hostname| {
        printf(format_args!(
            "nfs_boot: server_addr={} hostname={}\n",
            InAddrFmt(bp_sin.sin_addr),
            Str(hostname)
        ));
    });

    nd.nd_boot = bp_sin;

    Ok(())
}

/// The broadcast address (network order) of the first Internet address on `ifp`, the C's
/// `TAILQ_FOREACH` for an `AF_INET` address and its `ia_broadaddr`; `None` when it has none.
/// Read under the shared net lock (the interface's address list is a net cell).
fn nfs_boot_broadaddr(ifp: &Ifnet) -> Option<u32> {
    let w = NETLOCK.read();
    let net = NetHeld::of(&w)?;
    let r = net.reader();
    let broad = ifp
        .if_addrlist()
        .iter(&r)
        .find_map(|ifa| InIfaddr::of(ifa).map(|ia| ia.ia_broadaddr().get().sin_addr.s_addr));
    drop(r);
    broad
}

/// `nfs_boot_getfh(bpsin, key, ndmntp, retries)`: gets the file handle of the file system
/// `key` (`root` or `swap`) from the bootparam server `bpsin` (with `retries` tries: -1 is
/// forever): `ndmntp` is the output, the mount arguments set up to describe the file system
/// (`NFSMNT_NFSV3` cleared when the mount daemon speaks only an older version), the server's
/// address, its `host:path` name and the handle.
pub fn nfs_boot_getfh<G: ProcGlue>(
    p: &Thread<G>,
    bpsin: &SockaddrIn,
    key: &[u8],
    ndmntp: &mut NfsDlmount,
    retries: i32,
) -> Result<(), Errno> {
    let mut pathname = [0u8; MAXPATHLEN];

    // Initialize mount args (`addr`, `fh` and `hostname` stay NULL: the module's Redesign).
    let args = &mut ndmntp.ndm_args;
    *args = zeroed::<NfsArgs>();
    args.addrlen = i32::from(ndmntp.ndm_saddr.sin_len);
    args.sotype = SOCK_DGRAM;
    args.flags = NFSMNT_NFSV3;

    let sin = &mut ndmntp.ndm_saddr;

    // Get server:pathname for "key" (root or swap) using RPC to bootparam/getfile
    if let Err(e) = bp_getfile(
        p,
        bpsin,
        key,
        sin,
        &mut ndmntp.ndm_host,
        &mut pathname,
        retries,
    ) {
        printf(format_args!(
            "nfs_boot: bootparam get {}: {}\n",
            Str(key),
            e.as_i32()
        ));
        return Err(e);
    }

    // Get file handle for "key" (root or swap) using RPC to mountd/mount
    if let Err(e) = md_mount(p, sin, cstr(&pathname), args, &mut ndmntp.ndm_fh) {
        printf(format_args!(
            "nfs_boot: mountd {}, error={}\n",
            Str(key),
            e.as_i32()
        ));
        return Err(e);
    }

    // Set port number for NFS use.
    // XXX: NFS port is always 2049, right?
    let ver = if args.flags & NFSMNT_NFSV3 != 0 {
        NFS_VER3
    } else {
        NFS_VER2
    };
    match krpc_portmap(p, sin, NFS_PROG, ver) {
        Ok(port) => sin.sin_port = port,
        Err(e) => {
            printf(format_args!(
                "nfs_boot: portmap NFS, error={}\n",
                e.as_i32()
            ));
            return Err(e);
        }
    }

    // Construct remote path (for getmntinfo(3))
    remote_path(&mut ndmntp.ndm_host, cstr(&pathname));

    Ok(())
}

/// The tail of `nfs_boot_getfh`: appends `:` and `pathname` to the server name in `host`,
/// cut to fit `MNAMELEN` (the remote path `getmntinfo(3)` shows). A name that fills the
/// array but its last byte leaves no room for the NUL: the `:` takes that byte and the NUL
/// is not written (the C writes it one byte past the array; readers copy at most
/// `MNAMELEN` bytes).
fn remote_path(host: &mut [u8; MNAMELEN], pathname: &[u8]) {
    let endp = MNAMELEN - 1;
    let mut dp = cstr(host).len();
    if dp >= MNAMELEN {
        return;
    }
    host[dp] = b':';
    dp += 1;
    for &c in pathname {
        if dp >= endp {
            break;
        }
        host[dp] = c;
        dp += 1;
    }
    if dp < MNAMELEN {
        host[dp] = 0;
    }
}

/// `bp_whoami(bpsin, my_ip, gw_ip)`: RPC bootparam/whoami: given the client IP address, gets
/// the client name (`hostname`), the domain name (`domainname`) and the gateway address (the
/// return value). The hostname and domainname are set here for convenience.
///
/// Note: `bpsin` is initialized to the broadcast address, and will be replaced with the
/// bootparam server address after this call is complete. Have to use `PMAP_PROC_CALL` to make
/// sure we get responses only from a servers that know about us (don't want to broadcast a
/// getport call).
fn bp_whoami<G: ProcGlue>(
    p: &Thread<G>,
    bpsin: &mut SockaddrIn,
    my_ip: InAddr,
) -> Result<InAddr, Errno> {
    // Build request message for PMAPPROC_CALLIT: struct whoami_call (the program, version and
    // procedure to call and the length of the argument, all words) and the encapsulated data
    // (client IP address).
    let mut w = XdrWriter::reqhead(0);
    w.u32(BOOTPARAM_PROG);
    w.u32(BOOTPARAM_VERS);
    w.u32(BOOTPARAM_WHOAMI);
    w.u32(XDR_INADDR_SIZE as u32);
    xdr_inaddr_encode(&mut w, &my_ip);

    // RPC: portmap/callit
    bpsin.sin_port = PMAPPORT.to_be();
    let mut from = None;
    let m = krpc_call(
        p,
        bpsin,
        PMAPPROG,
        PMAPVERS,
        PMAPPROC_CALLIT,
        w.into_chain(),
        Some(&mut from),
        -1,
    )?;

    // Parse result message.
    let r = bp_whoami_reply(&m, from.as_deref(), bpsin);
    if r.is_err() {
        printf(format_args!("nfs_boot: bootparam_whoami: bad reply\n"));
    }
    r
}

/// The reply half of `bp_whoami`: the callit reply in `m` (the port and the length of the
/// encapsulated reply, then the whoami reply: hostname, domainname, gateway), and the
/// address it came from; `EBADRPC` for a bad reply. The names are stored as they are read,
/// as the C's.
fn bp_whoami_reply(m: &Mbuf, from: Option<&Mbuf>, bpsin: &mut SockaddrIn) -> Result<InAddr, Errno> {
    let mut r = XdrReader::new(m);
    // struct callit_reply: the port and the length of the encapsulated data (words).
    let port = r.u32()? as i16;
    let _msg_len = r.u32()?; // not used by the C either

    // Save bootparam server address: sin_addr, at offset 4 of the sender's sockaddr_in.
    let from = from.ok_or(Errno::EBADRPC)?;
    let addr = XdrReader::at(from, 4)?.word()?;
    bpsin.sin_port = (port as u16).to_be();
    bpsin.sin_addr.s_addr = addr;

    // client name
    let mut len = MAXHOSTNAMELEN - 1;
    HOSTNAMELEN.store(len as i32, core::sync::atomic::Ordering::Relaxed);
    with_hostname_mut(|hostname| xdr_string_decode(&mut r, hostname, &mut len))?;
    HOSTNAMELEN.store(len as i32, core::sync::atomic::Ordering::Relaxed);

    // domain name
    let mut len = MAXHOSTNAMELEN - 1;
    DOMAINNAMELEN.store(len as i32, core::sync::atomic::Ordering::Relaxed);
    with_domainname_mut(|domainname| xdr_string_decode(&mut r, domainname, &mut len))?;
    DOMAINNAMELEN.store(len as i32, core::sync::atomic::Ordering::Relaxed);

    // gateway address
    xdr_inaddr_decode(&mut r)
}

/// `bp_getfile(bpsin, key, md_sin, serv_name, pathname, retries)`: RPC bootparam/getfile:
/// given the client name and the file `key`, gets the server name (`serv_name`), the server IP
/// address (`md_sin`, a new `sockaddr_in` of it, for mountd and NFS) and the server pathname
/// (`pathname`).
fn bp_getfile<G: ProcGlue>(
    p: &Thread<G>,
    bpsin: &SockaddrIn,
    key: &[u8],
    md_sin: &mut SockaddrIn,
    serv_name: &mut [u8],
    pathname: &mut [u8],
    retries: i32,
) -> Result<(), Errno> {
    // Build request message.

    // client name (hostname), the bytes `hostnamelen` says, read under its lock
    let mut w = XdrWriter::reqhead(0);
    let hostnamelen = HOSTNAMELEN.load(core::sync::atomic::Ordering::Relaxed);
    with_hostname(|h| {
        let n = usize::try_from(hostnamelen).unwrap_or(0).min(h.len());
        xdr_string_encode(&mut w, &h[..n])
    })?;

    // key name (root or swap)
    xdr_string_encode(&mut w, key)?;

    // RPC: bootparam/getfile
    let m = krpc_call(
        p,
        bpsin,
        BOOTPARAM_PROG,
        BOOTPARAM_VERS,
        BOOTPARAM_GETFILE,
        w.into_chain(),
        None,
        retries,
    )?;

    // Parse result message.
    if bp_getfile_reply(&m, md_sin, serv_name, pathname).is_err() {
        printf(format_args!("nfs_boot: bootparam_getfile: bad reply\n"));
        return Err(Errno::EBADRPC);
    }
    Ok(())
}

/// The reply half of `bp_getfile`: server name, server address, server pathname; `EBADRPC`
/// for a bad reply.
fn bp_getfile_reply(
    m: &Mbuf,
    md_sin: &mut SockaddrIn,
    serv_name: &mut [u8],
    pathname: &mut [u8],
) -> Result<(), Errno> {
    let mut r = XdrReader::new(m);

    // server name
    let mut sn_len = MNAMELEN - 1;
    xdr_string_decode(&mut r, serv_name, &mut sn_len)?;

    // server IP address (mountd/NFS)
    let inaddr = xdr_inaddr_decode(&mut r)?;

    // server pathname
    let mut path_len = MAXPATHLEN - 1;
    xdr_string_decode(&mut r, pathname, &mut path_len)?;

    // setup server socket address
    *md_sin = SockaddrIn {
        sin_len: size_of::<SockaddrIn>() as u8,
        sin_family: AF_INET,
        sin_addr: inaddr,
        ..SockaddrIn::default()
    };
    Ok(())
}

/// `md_mount(mdsin, path, argp)`: RPC mountd/mount: given a server pathname, gets an NFS file
/// handle (into `fh`, its size in `argp.fhsize`). Also sets `mdsin.sin_port` to the mount
/// daemon's port. `mdsin` is the mountd server address. Tries mount protocol version 3 (for
/// NFSv3) and falls back to lower versions, clearing `NFSMNT_NFSV3` when it ends up with
/// less than 3.
fn md_mount<G: ProcGlue>(
    p: &Thread<G>,
    mdsin: &mut SockaddrIn,
    path: &[u8],
    argp: &mut NfsArgs,
    fh: &mut [u8; NFSX_V3FHMAX],
) -> Result<(), Errno> {
    let mut mntver: u32 = if argp.flags & NFSMNT_NFSV3 != 0 { 3 } else { 2 };
    let reply: MbufBox = loop {
        // A failed portmap (`continue` in the C's do-while) or a mount daemon that does not
        // speak this version (`EPROGMISMATCH`) tries the next lower version; any other
        // result of the call ends the loop.
        let error = match krpc_portmap(p, mdsin, RPCPROG_MNT, mntver) {
            Err(e) => e,
            Ok(port) => {
                mdsin.sin_port = port;

                let mut w = XdrWriter::reqhead(0);
                xdr_string_encode(&mut w, path)?;

                // Do RPC to mountd.
                match krpc_call(
                    p,
                    mdsin,
                    RPCPROG_MNT,
                    mntver,
                    RPCMNT_MOUNT,
                    w.into_chain(),
                    None,
                    -1,
                ) {
                    Ok(m) => break m,
                    Err(Errno::EPROGMISMATCH) => Errno::EPROGMISMATCH,
                    Err(e) => return Err(e), // message already freed
                }
            }
        };
        // Try lower version of mountd.
        mntver -= 1;
        if mntver < 1 {
            return Err(error);
        }
    };

    if mntver != 3 {
        argp.flags &= !NFSMNT_NFSV3;
    }

    md_mount_reply(&reply, mntver, argp, fh)
}

/// The reply half of `md_mount`: the mount status and the file handle, its size in
/// `argp.fhsize`.
fn md_mount_reply(
    m: &Mbuf,
    mntver: u32,
    argp: &mut NfsArgs,
    fh: &mut [u8; NFSX_V3FHMAX],
) -> Result<(), Errno> {
    let mut r = XdrReader::new(m);
    // The reply might have only the errno; have at least errno, so check that.
    let error = r.u32()?;
    if error != 0 {
        return Err(i32::try_from(error)
            .ok()
            .and_then(Errno::from_raw)
            .unwrap_or(Errno::EIO));
    }

    // Have errno==0, so the fh must be there.
    let fhsize = if mntver == 3 {
        let fhsize = r.u32()?;
        if fhsize as usize > NFSX_V3FHMAX {
            return Err(Errno::EBADRPC);
        }
        fhsize as usize
    } else {
        NFSX_V2FH
    };
    argp.fhsize = fhsize as i32;

    r.bytes_into(&mut fh[..fhsize])
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the diskless boot's parsers (WHOAMI, GETFILE, MNT) over chains, the
    // remote path, the device name and the broadcast address, and the RPCs end to end
    // against the scripted peer of `krpc_subr`: WHOAMI by broadcast, the root's handle (mount
    // version 3), the fall back to version 2.

    use std::vec;
    use std::vec::Vec;

    use super::super::krpc_subr::testkrpc::{Peer, Seen, accepted, setup, sin, words, xaddr, xstr};
    use super::*;
    use crate::fs::nfs::nfsm_subs::{nfsm_rndup, test_chain};
    use crate::netinet::udp_usrreq::testudp;

    /// The names `f` sees, `kernel::base::sysctl`'s, put back after (the WHOAMI tests store theirs),
    /// under the lock of the tests that write them (`kern.hostname`'s round trip).
    fn with_names_saved(f: impl FnOnce()) {
        use core::sync::atomic::Ordering::Relaxed;
        let _names = crate::base::sysctl::names_test_lock();
        let h = with_hostname(|h| h.to_vec());
        let hl = HOSTNAMELEN.load(Relaxed);
        let d = with_domainname_mut(|d| d.to_vec());
        let dl = DOMAINNAMELEN.load(Relaxed);
        f();
        with_hostname_mut(|x| x.copy_from_slice(&h));
        HOSTNAMELEN.store(hl, Relaxed);
        with_domainname_mut(|x| x.copy_from_slice(&d));
        DOMAINNAMELEN.store(dl, Relaxed);
    }

    #[test]
    fn the_boot_device_name_is_kept_up_to_its_nul() {
        set_nfsbootdevname(b"vio0\0junk");
        assert_eq!(cstr(&(*NFSBOOTDEVNAME.lock()).unwrap_or_default()), b"vio0");
        set_nfsbootdevname(&[b'x'; 40]);
        let n = (*NFSBOOTDEVNAME.lock()).unwrap_or_default();
        assert_eq!(cstr(&n).len(), IFNAMSIZ - 1);
    }

    #[test]
    fn the_remote_path_is_host_colon_path_cut_to_mnamelen() {
        let mut h = [0u8; MNAMELEN];
        h[..6].copy_from_slice(b"server");
        remote_path(&mut h, b"/export/root");
        assert_eq!(cstr(&h), b"server:/export/root");
        // A long path stops one byte short of the end, for the NUL.
        let mut h = [0u8; MNAMELEN];
        h[..2].copy_from_slice(b"sv");
        remote_path(&mut h, &[b'p'; 200]);
        assert_eq!(cstr(&h).len(), MNAMELEN - 1);
        assert_eq!(h[MNAMELEN - 1], 0);
        // A name that fills the array (MNAMELEN - 1 bytes, the decode's limit): the `:` takes
        // its last byte and no NUL is written (the C writes it one byte past the array).
        let mut h = [b'n'; MNAMELEN];
        h[MNAMELEN - 1] = 0;
        remote_path(&mut h, b"/x");
        assert_eq!(h[MNAMELEN - 2..], [b'n', b':']);
        assert_eq!(cstr(&h).len(), MNAMELEN);
    }

    #[test]
    fn a_getfile_reply_gives_the_name_the_address_and_the_path() {
        let _g = crate::net::mbuf::tests::setup();
        let mut b = xstr(b"server");
        b.extend(xaddr([10, 77, 0, 2]));
        b.extend(xstr(b"/export/root"));
        let m = test_chain(&[&b[..9], &b[9..30], &b[30..]]);
        let mut md = SockaddrIn::default();
        let (mut name, mut path) = ([0u8; MNAMELEN], [0u8; MAXPATHLEN]);
        assert_eq!(bp_getfile_reply(&m, &mut md, &mut name, &mut path), Ok(()));
        assert_eq!(
            (cstr(&name), cstr(&path)),
            (&b"server"[..], &b"/export/root"[..])
        );
        assert_eq!(md.sin_addr.s_addr.to_ne_bytes(), [10, 77, 0, 2]);
        assert_eq!((md.sin_len, md.sin_family, md.sin_port), (16, AF_INET, 0));
        // Cut anywhere: EBADRPC.
        for cut in [0, 3, 10, 29, b.len() - 1] {
            let m = test_chain(&[&b[..cut]]);
            assert_eq!(
                bp_getfile_reply(&m, &mut md, &mut name, &mut path),
                Err(Errno::EBADRPC),
                "cut at {cut}"
            );
        }
        // A server name longer than MNAMELEN - 1 is cut to it.
        let mut b = xstr(&[b's'; 120]);
        b.extend(xaddr([1, 2, 3, 4]));
        b.extend(xstr(b"/"));
        let m = test_chain(&[&b]);
        assert_eq!(bp_getfile_reply(&m, &mut md, &mut name, &mut path), Ok(()));
        assert_eq!(cstr(&name).len(), MNAMELEN - 1);
    }

    /// A mount reply: the status, then the handle as version `v` sends it.
    fn mnt_reply(status: u32, v: u32, fh: &[u8]) -> Vec<u8> {
        let mut b = words(&[status]);
        if v == 3 {
            b.extend(words(&[fh.len() as u32]));
        }
        b.extend_from_slice(fh);
        b
    }

    #[test]
    fn a_mount_reply_gives_the_handle_by_its_version() {
        let _g = crate::net::mbuf::tests::setup();
        let mut args = zeroed::<NfsArgs>();
        let mut fh = [0u8; NFSX_V3FHMAX];
        let h28: Vec<u8> = (1..=28).collect();
        let m = test_chain(&[&mnt_reply(0, 3, &h28)[..6], &mnt_reply(0, 3, &h28)[6..]]);
        assert_eq!(md_mount_reply(&m, 3, &mut args, &mut fh), Ok(()));
        assert_eq!((args.fhsize, &fh[..28]), (28, &h28[..]));
        // Version 2: NFSX_V2FH bytes, no length word.
        let h32: Vec<u8> = (100..132).collect();
        let m = test_chain(&[&mnt_reply(0, 2, &h32)]);
        assert_eq!(md_mount_reply(&m, 2, &mut args, &mut fh), Ok(()));
        assert_eq!((args.fhsize, &fh[..32]), (32, &h32[..]));
        // The largest version 3 handle; one more, or a length with the top bit, EBADRPC.
        let h64 = [7u8; 64];
        let m = test_chain(&[&mnt_reply(0, 3, &h64)]);
        assert_eq!(md_mount_reply(&m, 3, &mut args, &mut fh), Ok(()));
        assert_eq!(args.fhsize, 64);
        for bad in [65u32, 0x8000_0000, 0xffff_ffff] {
            let mut b = words(&[0, bad]);
            b.extend_from_slice(&[0; 64]);
            let m = test_chain(&[&b]);
            assert_eq!(
                md_mount_reply(&m, 3, &mut args, &mut fh),
                Err(Errno::EBADRPC),
                "{bad:#x}"
            );
        }
        // The daemon's errno, EIO for a number that is none; a reply short of its handle or
        // of its status word.
        let m = test_chain(&[&words(&[13])]);
        assert_eq!(
            md_mount_reply(&m, 3, &mut args, &mut fh),
            Err(Errno::EACCES)
        );
        let m = test_chain(&[&words(&[70_000])]);
        assert_eq!(md_mount_reply(&m, 3, &mut args, &mut fh), Err(Errno::EIO));
        let m = test_chain(&[&mnt_reply(0, 2, &h32)[..20]]);
        assert_eq!(
            md_mount_reply(&m, 2, &mut args, &mut fh),
            Err(Errno::EBADRPC)
        );
        let m = test_chain(&[&[0, 0]]);
        assert_eq!(
            md_mount_reply(&m, 3, &mut args, &mut fh),
            Err(Errno::EBADRPC)
        );
    }

    /// The WHOAMI answer inside a CALLIT reply: bootparamd's `port`, then the names and the
    /// gateway.
    fn whoami_reply(port: u32, host: &[u8], domain: &[u8], gw: [u8; 4]) -> Vec<u8> {
        let mut inner = xstr(host);
        inner.extend(xstr(domain));
        inner.extend(xaddr(gw));
        let mut b = words(&[port, inner.len() as u32]);
        b.extend(inner);
        b
    }

    #[test]
    fn a_whoami_reply_names_the_host_and_the_server() {
        let (_g, _p) = testudp::setup();
        with_names_saved(|| {
            let b = whoami_reply(700, b"emibsd", b"example.org", [10, 77, 0, 254]);
            let m = test_chain(&[&b[..5], &b[5..]]);
            let from = testudp::nam(&sin([10, 77, 0, 2], 111));
            let mut bp = sin([10, 77, 0, 255], 111);
            assert_eq!(
                bp_whoami_reply(&m, Some(&from), &mut bp).map(|a| a.s_addr.to_ne_bytes()),
                Ok([10, 77, 0, 254])
            );
            assert_eq!(bp.sin_addr.s_addr.to_ne_bytes(), [10, 77, 0, 2]);
            assert_eq!(bp.sin_port, 700u16.to_be());
            use core::sync::atomic::Ordering::Relaxed;
            assert_eq!(HOSTNAMELEN.load(Relaxed), 6);
            assert_eq!(with_hostname(|h| h[..7].to_vec()), b"emibsd\0");
            assert_eq!(DOMAINNAMELEN.load(Relaxed), 11);
            assert_eq!(
                crate::base::sysctl::with_domainname(|d| d[..12].to_vec()),
                b"example.org\0"
            );
            // Short anywhere, or no sender: EBADRPC.
            for cut in [0, 7, 12, 20, b.len() - 1] {
                let m = test_chain(&[&b[..cut]]);
                assert_eq!(
                    bp_whoami_reply(&m, Some(&from), &mut bp),
                    Err(Errno::EBADRPC),
                    "cut at {cut}"
                );
            }
            assert_eq!(bp_whoami_reply(&m, None, &mut bp), Err(Errno::EBADRPC));
        });
    }

    #[test]
    fn the_broadcast_address_is_the_first_internet_address() {
        let (_g, _p) = testudp::setup();
        let (ifp, _d) = crate::netinet::in_pcb::testinet::iface(b"nfsb0", [10, 77, 30, 1]);
        assert_eq!(
            nfs_boot_broadaddr(ifp).map(u32::to_ne_bytes),
            Some([10, 77, 30, 255])
        );
        let bare = crate::net::if_::test_ifnet_with(b"nfsb1", &BARE);
        assert_eq!(nfs_boot_broadaddr(bare), None);
    }

    /// A driver for an interface with no address.
    static BARE: Bare = Bare;
    struct Bare;
    impl crate::net::if_var::NetDriver for Bare {
        fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            Ok(())
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            _m: MbufBox,
            _dst: crate::net::proto::SockaddrView<'_>,
            _rt: Option<&crate::net::route::Rtentry>,
        ) -> Result<(), Errno> {
            Ok(())
        }
    }

    /// A server for the boot's RPCs at 10.77.`n`.2: the portmapper (mountd at 635, nfsd at
    /// 2049; mountd version 3 answered with no port when `no_v3`), bootparamd's CALLIT and
    /// GETFILE, and mountd's MNT of either version.
    fn boot_server(n: u8, no_v3: bool) -> impl FnMut(&Seen) -> Vec<Vec<u8>> + Send + 'static {
        const NFS_PROG_T: u32 = 100003;
        const MNT_PROG_T: u32 = 100005;
        move |s| {
            let x = s.xid();
            let body = s.body();
            let w =
                |i: usize| u32::from_be_bytes(body[4 * i..4 * i + 4].try_into().unwrap_or([0; 4]));
            let reply = match s.call() {
                (PMAPPROG, PMAPVERS, 3) => match (w(0), w(1)) {
                    (MNT_PROG_T, 3) if no_v3 => vec![],
                    (MNT_PROG_T, _) => words(&[635]),
                    (NFS_PROG_T, _) => words(&[2049]),
                    _ => words(&[0]),
                },
                (PMAPPROG, PMAPVERS, PMAPPROC_CALLIT) => {
                    whoami_reply(700, b"emibsd", b"example.org", [10, 77, n, 254])
                }
                (BOOTPARAM_PROG, BOOTPARAM_VERS, BOOTPARAM_GETFILE) => {
                    // The client's name, then the key.
                    let hl = w(0) as usize;
                    let key_at = 4 + nfsm_rndup(hl);
                    let kl =
                        u32::from_be_bytes(body[key_at..key_at + 4].try_into().unwrap_or([0; 4]));
                    let key = &body[key_at + 4..key_at + 4 + kl as usize];
                    let mut b = xstr(b"server");
                    b.extend(xaddr([10, 77, n, 2]));
                    b.extend(xstr(&[b"/export/", key].concat()));
                    b
                }
                (MNT_PROG_T, 3, 1) => {
                    let mut b = words(&[0, 28]);
                    b.extend((1..=28).collect::<Vec<u8>>());
                    b
                }
                (MNT_PROG_T, 2, 1) => {
                    let mut b = words(&[0]);
                    b.extend((100..132).collect::<Vec<u8>>());
                    b
                }
                _ => return vec![],
            };
            vec![accepted(x, &reply)]
        }
    }

    #[test]
    fn whoami_asks_by_broadcast_through_the_portmapper() {
        let (_g, p, wire) = setup(b"nfsb2", 32);
        with_names_saved(|| {
            let peer = Peer::start(wire, [10, 77, 32, 2], boot_server(32, false));
            let mut bp = sin([10, 77, 32, 255], 0);
            let me = InAddr {
                s_addr: u32::from_ne_bytes([10, 77, 32, 1]),
            };
            assert_eq!(
                bp_whoami(p, &mut bp, me).map(|a| a.s_addr.to_ne_bytes()),
                Ok([10, 77, 32, 254])
            );
            assert_eq!(bp.sin_addr.s_addr.to_ne_bytes(), [10, 77, 32, 2]);
            assert_eq!(bp.sin_port, 700u16.to_be());
            let seen = peer.stop();
            assert_eq!(seen.len(), 1);
            assert_eq!((seen[0].dst, seen[0].dport), ([10, 77, 32, 255], PMAPPORT));
            assert_eq!(seen[0].call(), (PMAPPROG, PMAPVERS, PMAPPROC_CALLIT));
            // CALLIT of bootparamd's WHOAMI, its argument the client's address.
            let mut want = words(&[BOOTPARAM_PROG, BOOTPARAM_VERS, BOOTPARAM_WHOAMI, 20]);
            want.extend(xaddr([10, 77, 32, 1]));
            assert_eq!(seen[0].body(), &want[..]);
        });
    }

    #[test]
    fn getfh_finds_the_roots_server_path_and_version_3_handle() {
        let (_g, p, wire) = setup(b"nfsb3", 33);
        with_names_saved(|| {
            with_hostname_mut(|h| h[..7].copy_from_slice(b"emibsd\0"));
            HOSTNAMELEN.store(6, core::sync::atomic::Ordering::Relaxed);
            let peer = Peer::start(wire, [10, 77, 33, 2], boot_server(33, false));
            let mut nd = NfsDlmount::new();
            let bp = sin([10, 77, 33, 2], 700);
            assert_eq!(nfs_boot_getfh(p, &bp, b"root", &mut nd, 2), Ok(()));
            assert_eq!(cstr(&nd.ndm_host), b"server:/export/root");
            assert_eq!(nd.ndm_saddr.sin_addr.s_addr.to_ne_bytes(), [10, 77, 33, 2]);
            assert_eq!(nd.ndm_saddr.sin_port, 2049u16.to_be());
            assert_eq!(
                (nd.ndm_saddr.sin_len, nd.ndm_saddr.sin_family),
                (16, AF_INET)
            );
            let a = &nd.ndm_args;
            assert_eq!(
                (a.flags, a.fhsize, a.sotype),
                (NFSMNT_NFSV3, 28, SOCK_DGRAM)
            );
            // addrlen is read before the address is filled (0 in a fresh structure), and
            // the pointers stay NULL.
            assert_eq!((a.addrlen, a.addr, a.fh, a.hostname), (0, 0, 0, 0));
            assert_eq!(nd.ndm_fh[..28], (1..=28).collect::<Vec<u8>>()[..]);
            let seen = peer.stop();
            let calls: Vec<_> = seen.iter().map(Seen::call).collect();
            assert_eq!(
                calls,
                [
                    (BOOTPARAM_PROG, BOOTPARAM_VERS, BOOTPARAM_GETFILE),
                    (PMAPPROG, PMAPVERS, 3),
                    (100005, 3, 1),
                    (PMAPPROG, PMAPVERS, 3),
                ]
            );
            // GETFILE: the client's name (hostnamelen bytes), the key; MNT: the path.
            assert_eq!(seen[0].body(), [xstr(b"emibsd"), xstr(b"root")].concat());
            assert_eq!(seen[0].dport, 700);
            assert_eq!(seen[2].body(), xstr(b"/export/root"));
            assert_eq!(seen[2].dport, 635);
            assert_eq!(&seen[3].body()[..8], words(&[100003, 3]));
        });
    }

    #[test]
    fn getfh_falls_back_to_mount_version_2() {
        let (_g, p, wire) = setup(b"nfsb4", 34);
        with_names_saved(|| {
            let peer = Peer::start(wire, [10, 77, 34, 2], boot_server(34, true));
            let mut nd = NfsDlmount::new();
            let bp = sin([10, 77, 34, 2], 700);
            assert_eq!(nfs_boot_getfh(p, &bp, b"swap", &mut nd, 2), Ok(()));
            let a = &nd.ndm_args;
            assert_eq!((a.flags & NFSMNT_NFSV3, a.fhsize), (0, 32));
            assert_eq!(nd.ndm_fh[..32], (100..132).collect::<Vec<u8>>()[..]);
            assert_eq!(cstr(&nd.ndm_host), b"server:/export/swap");
            let seen = peer.stop();
            let calls: Vec<_> = seen.iter().map(Seen::call).collect();
            assert_eq!(
                calls,
                [
                    (BOOTPARAM_PROG, BOOTPARAM_VERS, BOOTPARAM_GETFILE),
                    (PMAPPROG, PMAPVERS, 3),
                    (PMAPPROG, PMAPVERS, 3),
                    (100005, 2, 1),
                    (PMAPPROG, PMAPVERS, 3),
                ]
            );
            // The portmapper asked for mountd 3, then 2, then nfsd 2.
            assert_eq!(&seen[1].body()[..8], words(&[100005, 3]));
            assert_eq!(&seen[2].body()[..8], words(&[100005, 2]));
            assert_eq!(&seen[4].body()[..8], words(&[100003, 2]));
        });
    }
}
/* </TESTS> */
