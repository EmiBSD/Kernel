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
/* </LICENSES> */

/* <CODE> */
//! The nfs client's host test fixture (N6.7a1): a scripted NFS server on 127.0.0.1.
//!
//! The test glue's network tables hold the UNIX domain alone (the process-wide tables of
//! `TestGlue`, set once by the UNIX socket tests), so the client's sockets are made over a
//! glue of their own, [`NfsGlue`]: the kernel's internet domain with the kernel's UDP and TCP
//! entries (`socreate` finds them by `pffindproto` as on a machine), process 0 of the thread
//! (root, `sobind`'s privilege), the requester a thread of a uid the test picks, a mount and a
//! vnode of a test file system that only holds the client's structure. `lo0` carries
//! 127.0.0.1/8. The peer ([`Peer`]) is a kernel socket bound to 127.0.0.1 port 2049 in a
//! thread of its own: it takes the packets `lo0` queued off its input queue and gives each to
//! its transport (`udp_input`, `tcp_input`: the softnet's work done by hand, `if_input_local`'s
//! checksum flags of a looped packet set, IP's input skipped since the registered
//! `ipv4_input` is one glue's for the whole process), reads the
//! calls that reach its socket and answers each as the test's script says (data, a call of
//! `nfs_timer` while the requester sleeps, a bare record mark). The requester blocks in
//! `soreceive` as on a machine and is woken by the reply's append.

use core::cell::Cell;
use core::pin::Pin;
use core::sync::atomic::Ordering::Relaxed;
use std::boxed::Box;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::vec::Vec;

use hal::malloc::{M_WAITOK, MArc, MBox};
use hal::netlock::{NETLOCK, NetHeld};
use hal::pod;
use hal::pool::PoolArc;
use hal::sys::errno::Errno;
use hal::sys::mount::{Fid, Statfs};
use hal::sys::netinet_in::{IPPROTO_TCP, IPPROTO_UDP, InAddr, SockaddrIn};
use hal::sys::netinet_in_var::InAliasreq;
use hal::sys::socket::{AF_INET, MSG_DONTWAIT, SO_LINGER, SOCK_DGRAM, SOCK_STREAM};
use hal::sys::sockio::SIOCAIFADDR;
use hal::sys::types::{Ino, Uid};

use super::nfs_socket::{nfs_connect, nfs_disconnect, nfs_timer};
use super::nfs_subs::nfs_init;
use super::nfsm_subs::test_bytes;
use super::nfsmount::{M_NFSMNT, NfsMount, NfsMountRef, NfsNam};
use crate::cred::{CredRef, Ucred};
use crate::fs::mount::{Mount, Vfsconf, vfs_mount_alloc, vfs_unbusy};
use crate::fs::subr::{getnewvnode, vfs_test_reset};
use crate::fs::vnode::{VT_NFS, Vnode};
use crate::fs::vops::{Cred, Nd, Vfs, Vnops};
use crate::net::domain::{Domain, domains_set, soinit};
use crate::net::if_var::Ifnet;
use crate::net::ifq::ifiq_test_take;
use crate::net::mbuf::{
    M_TCP_CSUM_IN_OK, M_TCP_CSUM_OUT, M_UDP_CSUM_IN_OK, M_UDP_CSUM_OUT, M_WAIT, MHLEN, MT_DATA,
    MbufBox, m_copydata, m_gethdr, mclget, ml_dequeue,
};
use crate::net::proto::Protocol;
use crate::net::socket::{SocketRef, sobind, soclose, socreate, solock, soreceive, sosend};
use crate::netinet::in_proto::{InetDomain, RipProtocol, TcpProtocol, UdpProtocol, inetsw_set};
use crate::proc::{ProcGlue, Thread, process_alloc, thread_alloc};
use crate::timeout::Timeout;

crate::test_file_glue!(NfsFiles, NfsGlue);
crate::test_proc_glue!(NfsGlue, NfsFiles);

/// The fixture's glue.
pub(crate) type G = NfsGlue;

/// `inetsw[]` of the glue: the kernel's UDP and TCP, and (N6.7b2) the `IPPROTO_RAW` row
/// (the nfs server's refusal of a raw socket).
static SW: [&dyn Protocol<G>; 3] = [
    &UdpProtocol::<G>::NEW,
    &TcpProtocol::<G>::NEW,
    &RipProtocol::<G>::RAW,
];
/// `domains[]` of the glue: the kernel's internet domain over [`SW`].
static DOMS: [&dyn Domain<G>; 1] = [&InetDomain::<G>::NEW];

/// The server's port.
pub(crate) const PEER_PORT: u16 = 2049;

/// The test file system the client's mount is of: it holds the client's structure and does
/// nothing else.
struct TestVfs;

impl Vfs<G> for TestVfs {
    fn mount(
        &self,
        _mp: &'static Mount<G>,
        _path: &[u8],
        _data: &mut [u8],
        _ndp: &mut Nd<'_, G>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn start(&self, _mp: &'static Mount<G>, _flags: i32, _p: &Thread<G>) -> Result<(), Errno> {
        Ok(())
    }
    fn unmount(&self, _mp: &'static Mount<G>, _f: i32, _p: &Thread<G>) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn root(&self, _mp: &'static Mount<G>) -> Result<&'static Vnode<G>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn quotactl(
        &self,
        _mp: &'static Mount<G>,
        _cmds: i32,
        _uid: Uid,
        _arg: usize,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn statfs(
        &self,
        _mp: &'static Mount<G>,
        _sbp: &mut Statfs,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn sync(
        &self,
        _mp: &'static Mount<G>,
        _waitfor: i32,
        _stall: i32,
        _cred: Cred<'_>,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Ok(())
    }
    fn vget(&self, _mp: &'static Mount<G>, _ino: Ino) -> Result<&'static Vnode<G>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn fhtovp(&self, _mp: &'static Mount<G>, _fid: &Fid) -> Result<&'static Vnode<G>, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    fn vptofh(&self, _vp: &'static Vnode<G>, _fid: &mut Fid) -> Result<(), Errno> {
        Err(Errno::EOPNOTSUPP)
    }
    /// No exports.
    fn checkexp(
        &self,
        _mp: &'static Mount<G>,
        _nam: &crate::net::mbuf::Mbuf,
    ) -> Result<crate::fs::export::ExportGrant, Errno> {
        Err(Errno::EOPNOTSUPP)
    }
}

/// The vnode operations of the fixture's vnode (none used).
struct TestVops;

impl Vnops<G> for TestVops {}

/// The device switch of the glue (`vfs_getnewfsid` reads `nblkdev`): no device answers.
struct NfsDevsw;

impl crate::fs::spec::Devsw<G> for NfsDevsw {
    fn nchrdev(&self) -> u32 {
        8
    }
    fn nblkdev(&self) -> u32 {
        8
    }
    fn cdev_type(&self, _maj: u32) -> u32 {
        0
    }
    fn cdev_flags(&self, _maj: u32) -> u32 {
        0
    }
    fn bdev_type(&self, _maj: u32) -> u32 {
        0
    }
    fn cdev_open(&self, _dev: i32, _o: i32, _t: i32, _p: &Thread<G>) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn cdev_close(&self, _dev: i32, _f: i32, _t: i32, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn cdev_read(
        &self,
        _dev: i32,
        _uio: &mut crate::uio::Uio<'_, G>,
        _i: i32,
    ) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn cdev_write(
        &self,
        _dev: i32,
        _uio: &mut crate::uio::Uio<'_, G>,
        _i: i32,
    ) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn cdev_ioctl(
        &self,
        _dev: i32,
        _cmd: u64,
        _data: &mut [u8],
        _fflag: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn cdev_kqfilter(&self, _dev: i32, _kn: &crate::event::Knote<G>) -> Option<Result<(), Errno>> {
        None
    }
    fn bdev_open(&self, _dev: i32, _o: i32, _t: i32, _p: &Thread<G>) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn bdev_close(&self, _dev: i32, _f: i32, _t: i32, _p: Option<&Thread<G>>) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn bdev_ioctl(
        &self,
        _dev: i32,
        _cmd: u64,
        _data: &mut [u8],
        _fflag: i32,
        _p: &Thread<G>,
    ) -> Result<(), Errno> {
        Err(Errno::ENXIO)
    }
    fn bdev_strategy(&self, _bp: crate::fs::vops::IoBuf) {}
    fn chrtoblk(&self, _dev: i32) -> i32 {
        crate::fs::bio::NODEV
    }
    fn iskmemdev(&self, _dev: i32) -> bool {
        false
    }
    fn iszerodev(&self, _dev: i32) -> bool {
        false
    }
}

static NFS_DEVSW: NfsDevsw = NfsDevsw;
static TEST_VFS: TestVfs = TestVfs;
static TEST_VOPS: TestVops = TestVops;
static CONF: [Vfsconf<G>; 1] = [Vfsconf::new(&TEST_VFS, b"nfs", 2, 0, 0)];

/// A thread of a process of its own, the credential's user `uid`.
fn new_thread(pid: i32, uid: u32) -> (&'static crate::proc::Process<G>, &'static Thread<G>) {
    let pr = process_alloc::<G>(pid, ());
    let mut c = Ucred::new();
    c.cr_uid.set(uid);
    c.cr_ruid.set(uid);
    drop(pr.set_ucred(CredRef::new(c)));
    crate::sig::test_sigacts(&pr);
    let t = thread_alloc::<G>(&pr, ());
    drop(t.set_cred(pr.ucred()));
    let p: &'static Thread<G> = hal::pool::PoolBox::leak(t);
    let pr: &'static crate::proc::Process<G> = match pr {
        crate::proc::StaticOrPooled::Static(r) => r,
        crate::proc::StaticOrPooled::Pooled(a) => PoolArc::leak(a),
    };
    (pr, p)
}

/// A test's fixture: the requester (this thread's `curproc`), the mount and the vnode of the
/// RPCs, `lo0`, and the guard of the tests that hold kernel memory.
pub(crate) struct Fixture {
    /// The requester.
    pub(crate) user: &'static Thread<G>,
    /// The client's mount.
    pub(crate) mp: &'static Mount<G>,
    /// A vnode of it (`nfs_request`'s).
    pub(crate) vp: &'static Vnode<G>,
    /// `lo0`.
    pub(crate) lo: &'static Ifnet,
    /// The memory's guard.
    _g: MutexGuard<'static, ()>,
}

/// The fixture over fresh kernel memory: the network of `testinet::setup` (interfaces and
/// routes forgotten), the glue's process 0 (root) once per thread, a requester of `uid`, the
/// glue's internet domain made again (`in_init`, `udp_init`, `tcp_init`), `lo0` with
/// 127.0.0.1/8, the glue's VFS tables with a mount and a vnode, "nfsreqpl".
pub(crate) fn setup(uid: u32) -> Fixture {
    let (g, _tp) = crate::netinet::in_pcb::testinet::setup();
    std::thread_local! {
        static MADE: Cell<bool> = const { Cell::new(false) };
    }
    if !MADE.with(|m| m.replace(true)) {
        crate::proc::procinit::<G>(64, 64);
        let _ = crate::descrip::filedesc_init::<NfsFiles>();
        let (pr0, p0) = new_thread(0, 0);
        G::tables().set_process0(pr0, Pin::static_ref(p0));
    }
    let (_, user) = new_thread(10, uid);
    NfsGlue::set_curproc(Some(user));
    // A stream's record mark is read into kernel bytes (`Uio::kernel_read`): the glue's
    // uiomove copies them.
    crate::testutil::set_copy_kernel(true);

    soinit::<G>();
    inetsw_set::<G>(&SW);
    domains_set::<G>(&DOMS);
    crate::netinet::in_pcb::in_init::<G>();
    crate::netinet::udp_usrreq::udp_init::<G>();
    crate::netinet::raw_ip::rip_init::<G>();
    crate::netinet::tcp_subr::tcp_test_reset::<G>();
    // The softnet's TCP input (`tcp_input_mlist_softnet`) is registered once per process,
    // by the first glue's `tcp_init`, and the kernel's TCP tests count on theirs
    // (`testtcp::setup`'s, testinet's glue): it is registered first here, whatever order the
    // tests run in (N6.7a1: an nfs test run first made it this glue's, and
    // `the_softnet_reaches_tcp_input_mlist` found no socket). `pump` calls `tcp_input`
    // itself, as it skips IP's input for the same reason.
    crate::netinet::tcp_input::tcp_input_mlist_register::<crate::netinet::in_pcb::testinet::G>();
    crate::netinet::tcp_subr::tcp_init::<G>();

    hal::klock::kernel_lock();
    let made = crate::net::if_loop::loop_clone_create(&crate::net::if_loop::LOOP_CLONER, 0);
    assert_eq!(made, Ok(()), "lo0");
    let lo = crate::net::if_::IFNETLIST
        .iter()
        .find(|i| i.if_xname.get()[..4] == *b"lo0\0")
        .expect("lo0 listed");
    let sin = |a: [u8; 4]| SockaddrIn {
        sin_len: 16,
        sin_family: AF_INET,
        sin_addr: InAddr {
            s_addr: u32::from_ne_bytes(a),
        },
        ..SockaddrIn::default()
    };
    let mut r = [0u8; core::mem::size_of::<InAliasreq>()];
    let _ = pod::put(
        &mut r,
        core::mem::offset_of!(InAliasreq, ifra_addr),
        &sin([127, 0, 0, 1]),
    );
    let _ = pod::put(
        &mut r,
        core::mem::offset_of!(InAliasreq, ifra_mask),
        &sin([255, 0, 0, 0]),
    );
    crate::netinet::in_::in_ioctl(SIOCAIFADDR, &mut r, Some(lo), true).expect("127.0.0.1");

    vfs_test_reset::<G>();
    crate::fs::init::vfsinit::<G>(&CONF, 64, || {});
    let _ = crate::fs::spec::register_devsw::<G>(&NFS_DEVSW);
    let mp = vfs_mount_alloc(None, &CONF[0]);
    vfs_unbusy(mp);
    let vp = getnewvnode(VT_NFS, Some(mp), &TEST_VOPS).expect("a vnode");
    nfs_init::<G>();
    Fixture {
        user,
        mp,
        vp,
        lo,
        _g: g,
    }
}

/// The `sockaddr_in` of 127.0.0.1:`port`.
pub(crate) fn lo_sin(port: u16) -> SockaddrIn {
    SockaddrIn {
        sin_len: 16,
        sin_family: AF_INET,
        sin_port: port.to_be(),
        sin_addr: InAddr {
            s_addr: u32::from_ne_bytes([127, 0, 0, 1]),
        },
        ..SockaddrIn::default()
    }
}

/// The client's structure for `fx`'s mount, of socket type `sotype`, mount flags `flag`,
/// the server 127.0.0.1:2049 (`mountnfs`'s fill, as `nfs_decode_args` leaves a default
/// mount): placed in the mount's slot, not connected yet. Its timeout calls nothing.
pub(crate) fn nfsmount(fx: &Fixture, sotype: i32, flag: i32) -> NfsMountRef<G> {
    fn nop(_: *mut core::ffi::c_void) {}
    let sa = lo_sin(PEER_PORT);
    let nam = NfsNam::new(super::xdr_subs::xdr_bytes(&sa)).expect("an address");
    let to =
        MBox::new(M_NFSMNT, M_WAITOK, Timeout::new(nop, core::ptr::null_mut())).expect("a timeout");
    let nmp = MArc::new(M_NFSMNT, M_WAITOK, NfsMount::new(fx.mp, nam, to)).expect("a mount");
    nmp.nm_sotype.store(sotype, Relaxed);
    nmp.nm_flag.store(flag, Relaxed);
    nmp.nm_retry.store(10, Relaxed);
    nmp.nm_timeo.store(10, Relaxed);
    nmp.nm_numgrps.store(16, Relaxed);
    for s in [&nmp.nm_rsize, &nmp.nm_wsize, &nmp.nm_readdirsize] {
        s.store(8192, Relaxed);
    }
    fx.mp.set_node(MArc::erase(nmp.clone()));
    nmp
}

/// Disconnects the client and empties the mount's slot (`nfs_unmount`'s tail), the timeout
/// deleted.
pub(crate) fn unmount(fx: &Fixture, nmp: NfsMountRef<G>) {
    nfs_disconnect(&nmp);
    nmp.nm_rtimeout.del();
    drop(fx.mp.take_node());
    drop(nmp);
}

/// What the peer does for a call.
pub(crate) enum Out {
    /// Sends these bytes back (a datagram, or a record with its mark).
    Data(Vec<u8>),
    /// Calls `nfs_timer` this many times (the softclock's ticks while the requester sleeps).
    Tick(usize),
    /// Sends a bare record mark of this length word (a stream only).
    Mark(u32),
}

/// The peer's script: the call's number (from 1) and its bytes (a stream's without the mark)
/// to what it does.
pub(crate) type Script = Box<dyn FnMut(usize, &[u8]) -> Vec<Out> + Send>;

/// The scripted peer: its socket(s), the calls it saw, its thread.
pub(crate) struct Peer {
    stop: Arc<AtomicBool>,
    calls: Arc<Mutex<Vec<(u16, Vec<u8>)>>>,
    thread: Option<JoinHandle<()>>,
    sock: SocketRef<G>,
}

impl Peer {
    /// A peer on 127.0.0.1:2049 of socket type `sotype`, answering by `script`; `nmp` is the
    /// mount whose timer `Out::Tick` runs.
    pub(crate) fn start(
        fx: &Fixture,
        sotype: i32,
        nmp: &NfsMountRef<G>,
        mut script: Script,
    ) -> Self {
        let so = socreate(fx.user, i32::from(AF_INET), sotype, 0).expect("the peer's socket");
        {
            let nam = name(&lo_sin(PEER_PORT));
            let mut w = solock(&so);
            sobind(&so, &mut w, &nam, fx.user).expect("2049");
            if sotype == SOCK_STREAM {
                crate::net::socket::solisten(&so, &mut w, 5).expect("listen");
            }
        }
        let stop = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (st, cl, lo, sock, me) = (stop.clone(), calls.clone(), fx.lo, so.clone(), fx.user);
        let timer = MArc::erase(nmp.clone());
        let thread = std::thread::spawn(move || {
            hal::klock::kernel_lock();
            // The softnet's thread has a curproc (`sonewconn`'s attach reads it).
            NfsGlue::set_curproc(Some(me));
            let mut n = 0;
            let mut conn: Option<SocketRef<G>> = None;
            let mut pending: Vec<u8> = Vec::new();
            while !st.load(Relaxed) {
                pump(lo);
                if sotype == SOCK_STREAM
                    && let Some(c) = accept(&sock)
                {
                    // A new connection (the client's reconnect) replaces the old one.
                    if let Some(old) = conn.replace(c) {
                        abort(&old);
                    }
                    pending.clear();
                }
                let mut got = Vec::new();
                if sotype == SOCK_DGRAM {
                    while let Some((from, data)) = recv_dgram(&sock) {
                        got.push((Some(from), data));
                    }
                } else if let Some(c) = &conn {
                    while let Some(data) = recv_stream(c) {
                        pending.extend_from_slice(&data);
                    }
                    while pending.len() >= 4 {
                        let len = (u32::from_be_bytes(pending[..4].try_into().unwrap())
                            & 0x7fff_ffff) as usize;
                        if pending.len() < 4 + len {
                            break;
                        }
                        let rec: Vec<u8> = pending.drain(..4 + len).skip(4).collect();
                        got.push((None, rec));
                    }
                }
                for (from, data) in got {
                    n += 1;
                    let port = from
                        .as_deref()
                        .map_or(0, |f| u16::from_be_bytes([f.data()[2], f.data()[3]]));
                    cl.lock().unwrap().push((port, data.clone()));
                    for out in script(n, &data) {
                        match out {
                            Out::Data(b) => match (&from, &conn) {
                                (Some(f), _) => send(&sock, Some(f), &b),
                                (None, Some(c)) => {
                                    let mut r =
                                        (0x8000_0000u32 | b.len() as u32).to_be_bytes().to_vec();
                                    r.extend_from_slice(&b);
                                    send(c, None, &r);
                                }
                                _ => {}
                            },
                            Out::Mark(len) => {
                                if let Some(c) = &conn {
                                    send(c, None, &(0x8000_0000u32 | len).to_be_bytes());
                                }
                            }
                            Out::Tick(k) => {
                                for _ in 0..k {
                                    nfs_timer::<G>(&timer);
                                    pump(lo);
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            if let Some(c) = conn {
                abort(&c);
            }
        });
        Self {
            stop,
            calls,
            thread: Some(thread),
            sock: so,
        }
    }

    /// The calls the peer saw, in order, each with its sender's port (0 on a stream).
    pub(crate) fn calls(&self) -> Vec<(u16, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
    }

    /// Stops the peer's thread and closes its socket.
    pub(crate) fn stop(mut self) {
        self.stop.store(true, Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().expect("the peer");
        }
        let _ = soclose(&self.sock, 0);
    }
}

/// `addr` in a name mbuf.
pub(crate) fn name(addr: &SockaddrIn) -> MbufBox {
    let b = super::xdr_subs::xdr_bytes(addr);
    let mut m = crate::net::mbuf::m_get(M_WAIT, crate::net::mbuf::MT_SONAME).expect("an mbuf");
    m.m_len().set(b.len() as u32);
    m.data_mut().expect("room").copy_from_slice(b);
    m
}

/// The softnet's work by hand: every packet `lo0` queued for input given to its transport,
/// under the net lock's shared holding (`ip_deliver`'s).
pub(crate) fn pump(lo: &'static Ifnet) {
    let mut ml = ifiq_test_take(lo.if_rcv());
    while let Some(m) = ml_dequeue(&mut ml) {
        let mut h = [0u8; 20];
        m_copydata(&m, 0, &mut h);
        let mut off = i32::from(h[0] & 0x0f) * 4;
        // `if_input_local`'s step for a looped packet: a checksum left to the interface is
        // good on input.
        let ph = m.m_pkthdr();
        let out = ph.csum_flags.get();
        if out & M_UDP_CSUM_OUT != 0 {
            ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_IN_OK);
        }
        if out & M_TCP_CSUM_OUT != 0 {
            ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_IN_OK);
        }
        let w = NETLOCK.read();
        let net = NetHeld::of(&w).expect("the net lock");
        let mut mp = Some(m);
        match i32::from(h[9]) {
            IPPROTO_UDP => {
                let _ = crate::netinet::udp_usrreq::udp_input::<G>(
                    &mut mp,
                    &mut off,
                    IPPROTO_UDP,
                    i32::from(AF_INET),
                    None,
                    &net,
                );
            }
            IPPROTO_TCP => {
                let _ = crate::netinet::tcp_input::tcp_input::<G>(
                    &mut mp,
                    &mut off,
                    IPPROTO_TCP,
                    i32::from(AF_INET),
                    None,
                    &net,
                );
            }
            _ => {}
        }
        drop(mp);
        drop(w);
    }
}

/// A datagram waiting on `so`, without waiting: its sender and its bytes.
fn recv_dgram(so: &SocketRef<G>) -> Option<(MbufBox, Vec<u8>)> {
    let mut uio = super::nfs_socket::count_uio::<G>(1 << 20, None);
    let (mut from, mut m, mut flags) = (None, None, MSG_DONTWAIT);
    soreceive(
        so,
        Some(&mut from),
        &mut uio,
        Some(&mut m),
        None,
        Some(&mut flags),
        0,
    )
    .ok()?;
    Some((from?, m.as_deref().map(test_bytes).unwrap_or_default()))
}

/// The stream bytes waiting on `so`, without waiting.
fn recv_stream(so: &SocketRef<G>) -> Option<Vec<u8>> {
    let mut uio = super::nfs_socket::count_uio::<G>(1 << 20, None);
    let (mut m, mut flags) = (None, MSG_DONTWAIT);
    soreceive(so, None, &mut uio, Some(&mut m), None, Some(&mut flags), 0).ok()?;
    m.as_deref().map(test_bytes)
}

/// A connection the listener `so` completed, taken off its queue and accepted (`sys_accept`'s
/// `soqremque` and `soaccept`).
fn accept(so: &SocketRef<G>) -> Option<SocketRef<G>> {
    let c = so.queued_first()?;
    drop(crate::net::socket::soqremque(
        &c,
        crate::net::socket::Onq::Q,
    ));
    let mut nam = crate::net::mbuf::m_get(M_WAIT, crate::net::mbuf::MT_SONAME)?;
    let mut w = solock(&c);
    crate::net::socket::soaccept(&c, &mut w, &mut nam).ok()?;
    drop(w);
    Some(c)
}

/// Closes the peer's connection `c` with a zero linger (`SO_LINGER`): `tcp_disconnect` drops
/// it with a reset, so neither side is left in a closing state or `TIME_WAIT` (control
/// blocks, their template mbufs and their timers outliving the test's memory: the mbuf
/// tests count every mbuf of the process, N6.7a1).
fn abort(c: &SocketRef<G>) {
    {
        let w = solock(c);
        c.set_options(&w, SO_LINGER, true);
    }
    let _ = soclose(c, 0);
}

/// Sends `b` on `so` (to `to` when given), without waiting.
fn send(so: &SocketRef<G>, to: Option<&MbufBox>, b: &[u8]) {
    let mut m = m_gethdr(M_WAIT, MT_DATA).expect("an mbuf");
    if b.len() > MHLEN {
        mclget(&mut m, M_WAIT);
    }
    m.m_len().set(b.len() as u32);
    m.m_pkthdr().len.set(b.len() as i32);
    m.data_mut().expect("room").copy_from_slice(b);
    let _ = sosend(so, to.map(|t| &**t), None, Some(m), None, MSG_DONTWAIT);
}

/// The raw words of a call: its xid (as it lies) and its procedure number.
pub(crate) fn call_xid(call: &[u8]) -> [u8; 4] {
    call[..4].try_into().unwrap()
}

/// An accepted reply to the call `call`, after its verifier (of `verf` bytes) the
/// `accept_stat` `astat` and, for a success, the NFS status `status` and `body`.
pub(crate) fn reply(call: &[u8], verf: u32, astat: u32, status: u32, body: &[u8]) -> Vec<u8> {
    let mut r = call_xid(call).to_vec();
    for w in [1u32, 0, 0, verf] {
        r.extend_from_slice(&w.to_be_bytes());
    }
    r.extend(core::iter::repeat_n(0u8, ((verf + 3) & !3) as usize));
    r.extend_from_slice(&astat.to_be_bytes());
    if astat == 0 {
        r.extend_from_slice(&status.to_be_bytes());
        r.extend_from_slice(body);
    }
    r
}

/// A denied reply: `RPC_MISMATCH` (with its two version words) or `AUTH_ERROR`.
pub(crate) fn denied(call: &[u8], mismatch: bool) -> Vec<u8> {
    let mut r = call_xid(call).to_vec();
    let words: &[u32] = if mismatch {
        &[1, 1, 0, 2, 2]
    } else {
        &[1, 1, 1, 1]
    };
    for w in words {
        r.extend_from_slice(&w.to_be_bytes());
    }
    r
}

/// Connects the client (`nfs_connect` as the requester).
pub(crate) fn connect(nmp: &NfsMount<G>) -> Result<(), Errno> {
    nfs_connect(nmp, None)
}

/// The test's end, while its memory is still the current one (the guard is the fixture's
/// last field), so that nothing of the test outlives it in a process-wide structure
/// (N6.7a1):
/// - the packets `lo0` still holds (the closing segments sent after the peer's last pump:
///   the client's FIN at the unmount, the resets of the peer's [`abort`]) are given to their
///   transports until none is left, as the softnet would: they were left on `lo0`'s input
///   queue and its softnet task, which a later test's softnet worker ran, freeing them into
///   its own memory, and the mbuf tests (which count every mbuf of the process) found them
///   and the connections' control blocks out;
/// - `lo0`'s addresses and their routes go, as `if_detach`'s `in_ifdetach` and
///   `in6_ifdetach` take them (`lo0` itself cannot be destroyed: `loop_clone_destroy`'s
///   `EPERM`): routing table 0 outlives the test, and a later test's interface of the same
///   index found them there (the cloner test's detach deleted `lo0`'s IPv6 routes, which
///   stayed in `rttrash`);
/// - the releases this test deferred to SMR run.
///
/// Skipped when the test is unwinding.
impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        let rcv = self.lo.if_rcv();
        for _ in 0..16 {
            if crate::net::ifq::ifiq_len(rcv) == 0 {
                break;
            }
            pump(self.lo);
        }
        assert_eq!(
            crate::net::ifq::ifiq_len(rcv),
            0,
            "lo0 quiet at the test's end"
        );
        {
            let w = NETLOCK.write();
            let net = NetHeld::of(&w).expect("the net lock");
            crate::netinet::in_::in_ifdetach(self.lo, &net);
            crate::netinet6::in6_ifattach::in6_ifdetach(self.lo, &net);
        }
        crate::net::route::tests::grace();
    }
}
/* </CODE> */
