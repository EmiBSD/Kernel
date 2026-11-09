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
 * Copyright (c) 2012-2013 Henning Brauer <henning@openbsd.org>
 * Copyright (c) 1982, 1986, 1989, 1993
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
 *	@(#)if.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! `<net/if_var.h>` for `bsd`'s network code: the interface, its addresses, its groups and its
//! softnet stack are the kernel's (`kernel::net::if_var`, N6.3a), re-exported here under their
//! LZ paths; what stays `bsd`'s is what names `bsd`'s types: the drivers' procedure handles
//! (the `If*Fn` types, kept per interface in a [`LegacyNetDriver`], the kernel's driver of a
//! `bsd` interface), the cloners' [`IfClone`] and the protocols' input queues ([`Niqueue`]).
//!
//! Upstream: sys/net/if_var.h @ 3ce1f3f79392
//!
//! ## Deviations
//! - The function pointers are `Option<fn>` with Rust signatures: errors are
//!   `Result<(), Errno>`, `if_ioctl`'s `caddr_t data` is a raw pointer to the request (the
//!   handlers are `unsafe fn`s: the buffer's type is the command's), `if_output` takes the
//!   destination as `*const Sockaddr` (a sockaddr may be longer than `struct sockaddr`) and is
//!   `unsafe` for the same reason, and `if_bpf_mtap` answers `bool` (the C's "drop it")
//!   (LZ's).
//! - A driver's handles are set through [`IfnetLegacy`] (`ifp.if_ioctl().set(..)` for the
//!   C's `ifp->if_ioctl = ..`): they live in the interface's [`LegacyNetDriver`], made at the
//!   first set and registered as the interface's driver (`Ifnet::if_driver`), which the
//!   kernel's `if_attach` takes; the box is never freed (one per attach, `M_DEVBUF`,
//!   D-N6.3a-8; docs/SCAFFOLDING.md). So `vmstat -m`'s `devbuf` grows by one table per
//!   interface ever made, a cloned one included (`ifconfig lo3 create`/`destroy` in a loop
//!   leaks one each, the C frees nothing of the kind), while `counters` shows less (the
//!   counters are inline in the interface): N7's `NetDevice` owns the table and frees it at
//!   detach (review nit, kept).
//! - `struct netstack`'s `ns_route` is not kept here: the softnet's route cache needs a
//!   `bsd` `struct route` the kernel's netstack cannot hold, and a static of one would need a
//!   new `unsafe impl Sync`; the protocols look a received packet's route up as the C does
//!   without a netstack (`ip_input`'s local `iproute`) until `struct route` is the kernel's
//!   (N6.3b).

use hal::link::{Link, SaToken};
use hal::malloc::{M_WAITOK, MBox};
use hal::once::OnceStatic;

use crate::kern::subr_prf::{Str, panic};
use crate::kern::uipc_mbuf::{mq_delist, mq_dequeue, sysctl_mq};
use crate::machine::intr::IPL_NET;
use crate::net::route::Rtentry;
use crate::sys::errno::Errno;
use crate::sys::malloc::M_DEVBUF;
use crate::sys::mbuf::{Mbuf, MbufList, MbufQueue, mq_drops, mq_len};
use crate::sys::socket::Sockaddr;
use hal::sys::net_if::SockaddrDl;

pub use crate::net::if_::{IF_TMPLIST_LOCK, IFNETLIST};
pub use kernel::net::if_var::{
    AtomicCell, IF_CARP_DEFAULT_PRIORITY, IF_TXMIT_DEFAULT, IF_TXMIT_MIN,
    IF_WIRED_DEFAULT_PRIORITY, IF_WIRELESS_DEFAULT_PRIORITY, IF_WWAN_DEFAULT_PRIORITY, IFA_ROUTE,
    IFC_NCOUNTERS, IFNET_SLOWTIMO, IfAddrlist, IfCounterArray, IfCounters, IfMaddrlist, IfTmplist,
    IfaTmplist, Ifaddr, IfgGroup, IfgMembers, IfgRef, IfgSet, Ifmaddr, Ifnet, IfnetList, NetDriver,
    Netstack, counters_inc, counters_pkt, if_rxr_cwm, if_rxr_inuse, if_rxr_needrefill, if_rxr_put,
};
pub use kernel::net::ifq::{Ifiqueue, Ifqueue};

/// `int (*ifc_create)(struct if_clone *, int)`: creates unit `unit` of a cloner.
pub type IfcCreateFn = fn(&'static IfClone, i32) -> Result<(), Errno>;
/// `int (*ifc_destroy)(struct ifnet *)`: destroys a cloned interface.
pub type IfcDestroyFn = fn(&'static Ifnet) -> Result<(), Errno>;

/// `struct if_clone`: a "cloning" interface, the kernel's `Cloner` (registered by
/// `if_clone_attach`, which keeps the list; `ifc_list` is the kernel's table).
pub struct IfClone {
    /// `ifc_name`: name of device, e.g. `gif` (no NUL).
    pub ifc_name: &'static [u8],
    /// `ifc_namelen`: length of name.
    pub ifc_namelen: usize,
    /// `ifc_create`.
    pub ifc_create: IfcCreateFn,
    /// `ifc_destroy`: NULL when the cloner's interfaces cannot be destroyed.
    pub ifc_destroy: Option<IfcDestroyFn>,
}

impl IfClone {
    /// `IF_CLONE_INITIALIZER(name, create, destroy)`.
    pub const fn new(
        name: &'static [u8],
        create: IfcCreateFn,
        destroy: Option<IfcDestroyFn>,
    ) -> Self {
        Self {
            ifc_name: name,
            ifc_namelen: name.len(),
            ifc_create: create,
            ifc_destroy: destroy,
        }
    }
}

impl kernel::net::if_var::Cloner for IfClone {
    fn name(&self) -> &'static [u8] {
        self.ifc_name
    }

    fn create(&'static self, unit: i32) -> Result<(), Errno> {
        (self.ifc_create)(self, unit)
    }

    fn has_destroy(&self) -> bool {
        self.ifc_destroy.is_some()
    }

    fn destroy(&self, ifp: &'static Ifnet) -> Result<(), Errno> {
        match self.ifc_destroy {
            Some(destroy) => destroy(ifp),
            None => Err(Errno::EOPNOTSUPP),
        }
    }
}

/// `void (*if_rtrequest)(struct ifnet *, int, struct rtentry *)`.
pub type IfRtrequestFn = fn(&'static Ifnet, i32, Option<&'static Rtentry>);
/// `void (*if_input)(struct ifnet *, struct mbuf *, struct netstack *)`: the netstack `None`
/// when the caller holds the net lock and wants the packet handled at once.
pub type IfInputFn = fn(&'static Ifnet, &'static Mbuf, Option<&mut Netstack>);
/// `int (*if_bpf_mtap)(caddr_t, const struct mbuf *, u_int)`: `true` drops the packet.
pub type IfBpfMtapFn = fn(*mut u8, &Mbuf, u32) -> bool;
/// `int (*if_output)(struct ifnet *, struct mbuf *, struct sockaddr *, struct rtentry *)`.
///
/// # Safety
///
/// `dst` points at a readable socket address of its `sa_len` bytes.
pub type IfOutputFn = unsafe fn(
    &'static Ifnet,
    &'static Mbuf,
    *const Sockaddr,
    Option<&'static Rtentry>,
) -> Result<(), Errno>;
/// `int (*if_enqueue)(struct ifnet *, struct mbuf *)`.
pub type IfEnqueueFn = fn(&'static Ifnet, &'static Mbuf) -> Result<(), Errno>;
/// `void (*if_start)(struct ifnet *)`.
pub type IfStartFn = fn(&'static Ifnet);
/// `int (*if_ioctl)(struct ifnet *, u_long, caddr_t)`.
///
/// # Safety
///
/// `data` points at the request's structure for the command (`IOCPARM_LEN(cmd)` bytes,
/// readable and writable).
pub type IfIoctlFn = unsafe fn(&'static Ifnet, u64, *mut u8) -> Result<(), Errno>;
/// `void (*if_watchdog)(struct ifnet *)`.
pub type IfWatchdogFn = fn(&'static Ifnet);
/// `int (*if_wol)(struct ifnet *, int)`.
pub type IfWolFn = fn(&'static Ifnet, bool) -> Result<(), Errno>;
/// `void (*if_qstart)(struct ifqueue *)`.
pub type IfQstartFn = fn(&'static Ifqueue);

/// The distinct functions one handle type may name over all the interfaces (the drivers').
const FN_SLOTS: usize = 64;

/// Defines a handle's slot type: an atomic link to an interned copy of the function, in a
/// table of its type's (a function pointer is not made back from a word without `unsafe`;
/// docs/IDIOMS.md). Two loads a read.
macro_rules! fn_slot {
    ($(#[$m:meta])* $slot:ident, $f:ty, $table:ident) => {
        /// The interned functions of the slot type of the same name.
        static $table: [OnceStatic<$f>; FN_SLOTS] = [const { OnceStatic::new() }; FN_SLOTS];

        $(#[$m])*
        pub struct $slot(Link<$f>);

        impl $slot {
            /// An empty handle (NULL).
            const fn new() -> Self {
                Self(Link::new())
            }

            /// The handle, `None` for NULL.
            #[inline]
            pub fn get(&self) -> Option<$f> {
                self.0.get().copied()
            }

            /// `ifp->if_x = f` (`None` for NULL).
            pub fn set(&self, f: Option<$f>) {
                self.0.set(f.map(|f| {
                    for slot in &$table {
                        if let Some(g) = slot.get() {
                            if core::ptr::fn_addr_eq(*g, f) {
                                return g;
                            }
                            continue;
                        }
                        let _ = slot.set(f);
                        if let Some(g) = slot.get()
                            && core::ptr::fn_addr_eq(*g, f)
                        {
                            return g;
                        }
                    }
                    panic(format_args!("{}: too many functions", stringify!($slot)))
                }));
            }
        }
    };
}

fn_slot!(
    /// `if_input`.
    InputSlot, IfInputFn, INPUT_FNS
);
fn_slot!(
    /// `if_bpf_mtap`.
    BpfMtapSlot, IfBpfMtapFn, BPF_MTAP_FNS
);
fn_slot!(
    /// `if_output` and `if_ll_output`.
    OutputSlot, IfOutputFn, OUTPUT_FNS
);
fn_slot!(
    /// `if_enqueue`.
    EnqueueSlot, IfEnqueueFn, ENQUEUE_FNS
);
fn_slot!(
    /// `if_start`.
    StartSlot, IfStartFn, START_FNS
);
fn_slot!(
    /// `if_ioctl`.
    IoctlSlot, IfIoctlFn, IOCTL_FNS
);
fn_slot!(
    /// `if_watchdog`.
    WatchdogSlot, IfWatchdogFn, WATCHDOG_FNS
);
fn_slot!(
    /// `if_wol`.
    WolSlot, IfWolFn, WOL_FNS
);
fn_slot!(
    /// `if_qstart`.
    QstartSlot, IfQstartFn, QSTART_FNS
);
fn_slot!(
    /// `if_rtrequest`.
    RtrequestSlot, IfRtrequestFn, RTREQUEST_FNS
);

/// A `bsd` interface's procedure handles (`struct ifnet`'s `if_input` .. `if_qstart` and
/// `if_rtrequest`), which its driver sets: the kernel's `NetDriver` of the interface
/// (`sys/net/if_.rs` implements it). One per interface, never freed (the module's
/// deviations).
pub struct LegacyNetDriver {
    /// `if_input`.
    pub if_input: InputSlot,
    /// `if_bpf_mtap`.
    pub if_bpf_mtap: BpfMtapSlot,
    /// `if_output`.
    pub if_output: OutputSlot,
    /// `if_ll_output`.
    pub if_ll_output: OutputSlot,
    /// `if_enqueue`.
    pub if_enqueue: EnqueueSlot,
    /// `if_start`.
    pub if_start: StartSlot,
    /// `if_ioctl`.
    pub if_ioctl: IoctlSlot,
    /// `if_watchdog`.
    pub if_watchdog: WatchdogSlot,
    /// `if_wol`.
    pub if_wol: WolSlot,
    /// `if_qstart`.
    pub if_qstart: QstartSlot,
    /// `if_rtrequest` (`route.c`'s, never the kernel's).
    pub if_rtrequest: RtrequestSlot,
}

impl LegacyNetDriver {
    /// Every handle NULL.
    const fn new() -> Self {
        Self {
            if_input: InputSlot::new(),
            if_bpf_mtap: BpfMtapSlot::new(),
            if_output: OutputSlot::new(),
            if_ll_output: OutputSlot::new(),
            if_enqueue: EnqueueSlot::new(),
            if_start: StartSlot::new(),
            if_ioctl: IoctlSlot::new(),
            if_watchdog: WatchdogSlot::new(),
            if_wol: WolSlot::new(),
            if_qstart: QstartSlot::new(),
            if_rtrequest: RtrequestSlot::new(),
        }
    }
}

/// The C's `ifp->if_x` handles of a `bsd` interface, through its [`LegacyNetDriver`] (made at
/// the first use, before `if_attach`, as the drivers set them).
pub trait IfnetLegacy {
    /// The interface's handle table, made and registered as its driver if it has none;
    /// `panic(9)` for an interface whose driver is not `bsd`'s.
    fn legacy(&self) -> &'static LegacyNetDriver;

    /// `ifp->if_input`.
    fn if_input(&self) -> &'static InputSlot {
        &self.legacy().if_input
    }
    /// `ifp->if_bpf_mtap`.
    fn if_bpf_mtap(&self) -> &'static BpfMtapSlot {
        &self.legacy().if_bpf_mtap
    }
    /// `ifp->if_output`.
    fn if_output(&self) -> &'static OutputSlot {
        &self.legacy().if_output
    }
    /// `ifp->if_ll_output`.
    fn if_ll_output(&self) -> &'static OutputSlot {
        &self.legacy().if_ll_output
    }
    /// `ifp->if_enqueue`.
    fn if_enqueue(&self) -> &'static EnqueueSlot {
        &self.legacy().if_enqueue
    }
    /// `ifp->if_start`.
    fn if_start(&self) -> &'static StartSlot {
        &self.legacy().if_start
    }
    /// `ifp->if_ioctl`.
    fn if_ioctl(&self) -> &'static IoctlSlot {
        &self.legacy().if_ioctl
    }
    /// `ifp->if_watchdog`.
    fn if_watchdog(&self) -> &'static WatchdogSlot {
        &self.legacy().if_watchdog
    }
    /// `ifp->if_wol`.
    fn if_wol(&self) -> &'static WolSlot {
        &self.legacy().if_wol
    }
    /// `ifp->if_qstart`.
    fn if_qstart(&self) -> &'static QstartSlot {
        &self.legacy().if_qstart
    }
    /// `ifp->if_rtrequest`.
    fn if_rtrequest(&self) -> &'static RtrequestSlot {
        &self.legacy().if_rtrequest
    }
}

impl IfnetLegacy for Ifnet {
    fn legacy(&self) -> &'static LegacyNetDriver {
        if let Some(&d) = self.if_driver.get() {
            return match d.as_any().and_then(|a| a.downcast_ref::<LegacyNetDriver>()) {
                Some(l) => l,
                None => panic(format_args!(
                    "{}: not a bsd driver",
                    Str(&self.if_xname.get())
                )),
            };
        }
        let Some(b) = MBox::new(M_DEVBUF, M_WAITOK, LegacyNetDriver::new()) else {
            panic(format_args!("if_legacy: out of memory"));
        };
        let l: &'static LegacyNetDriver = MBox::leak(b);
        match self.if_driver.set(l) {
            Ok(()) => l,
            // Another CPU set the table first (a driver attaches one interface on one CPU, so
            // never): use it.
            Err(_) => self.legacy(),
        }
    }
}

/// `ifp->if_sadl` as the C's pointer (NULL before `if_alloc_sadl`), for the routing socket's
/// messages and the protocols that read the link name under the net lock, which its writers
/// (`if_alloc_sadl`, `if_setlladdr`, `if_sadl_set_lladdr`, `if_free_sadl`) hold too: the C's
/// protocol for the raw pointer, which this function does not dereference. Read-only: the
/// writes go through the kernel's `ReadMostly` cell.
pub fn if_sadl_ptr(ifp: &Ifnet) -> *const SockaddrDl {
    ifp.sadl()
        .as_deref()
        .map_or(core::ptr::null(), core::ptr::from_ref)
}

/// `hal`'s one `SaToken` (`hal::link::SaLink::set`'s), for the internet protocols that publish
/// their interface addresses' storage (`ifa->ifa_addr = sintosa(&ia->ia_addr)`, N6.3a).
static SA_TOKEN: OnceStatic<&'static SaToken> = OnceStatic::new();

/// How long [`sa_token_take`] waits for a concurrent first taker's store before it calls the
/// token stolen.
const SA_TOKEN_TAKE_SPINS: u32 = 1 << 20;

/// Takes `hal`'s one `SaToken` for `bsd`'s internet protocols. `hal_bridges_register` calls it
/// in the boot, before any code of the kernel or ABI crates runs, so none of them can make an
/// interface address name a storage (the token's contract, `sys/hal/trust.rs`); another call
/// keeps what is held. `panic(9)` when the token is gone and was not stored here: someone
/// else took it.
pub fn sa_token_take() {
    if SA_TOKEN.get().is_some() {
        return;
    }
    if let Some(t) = SaToken::take() {
        let _ = SA_TOKEN.set(t);
        return;
    }
    // Taken already: by a first caller about to store it (two host test threads at once), or
    // by someone else.
    for _ in 0..SA_TOKEN_TAKE_SPINS {
        if SA_TOKEN.get().is_some() {
            return;
        }
        core::hint::spin_loop();
    }
    panic(format_args!(
        "SaLink: hal's token was taken outside bsd's boot"
    ));
}

/// The token [`sa_token_take`] stored at boot (the host tests, which have no boot, take it
/// here).
pub fn sa_token() -> &'static SaToken {
    if let Some(t) = SA_TOKEN.get() {
        return t;
    }
    sa_token_take();
    match SA_TOKEN.get() {
        Some(t) => t,
        None => panic(format_args!("SaLink: no token")),
    }
}

/// A test's link name for `ifp` (what `if_alloc_sadl` would make), in the test's memory.
#[cfg(test)]
pub(crate) fn if_sadl_test_set(ifp: &Ifnet, sdl: SockaddrDl) {
    let Some(b) = MBox::new(kernel::net::if_::M_IFADDR, M_WAITOK, sdl) else {
        panic(format_args!("if_sadl_test_set: out of memory"));
    };
    drop(ifp.if_sadl.replace(Some(b)));
}

/// `rw_assert_wrlock(&ifp->if_maddrlock)` (`DIAGNOSTIC`).
pub fn if_maddrlock_assert_wrlock(ifp: &Ifnet) {
    crate::kassert!(ifp.rt().if_maddrlock.status() == hal::rwlock::RW_WRITE);
}

/// `rw_assert_anylock(&ifp->if_maddrlock)` (`DIAGNOSTIC`).
pub fn if_maddrlock_assert_anylock(ifp: &Ifnet) {
    crate::kassert!(matches!(
        ifp.rt().if_maddrlock.status(),
        hal::rwlock::RW_WRITE | hal::rwlock::RW_READ
    ));
}

/// `rw_enter_write(&ifp->if_maddrlock)`.
pub fn if_maddrlock_enter_write(ifp: &Ifnet) {
    let _ = ifp.rt().if_maddrlock.enter(hal::rwlock::RW_WRITE);
}

/// `rw_exit_write(&ifp->if_maddrlock)`.
pub fn if_maddrlock_exit_write(ifp: &Ifnet) {
    ifp.rt().if_maddrlock.exit();
}

/// `rw_enter_read(&ifp->if_maddrlock)`.
pub fn if_maddrlock_enter_read(ifp: &Ifnet) {
    let _ = ifp.rt().if_maddrlock.enter(hal::rwlock::RW_READ);
}

/// `rw_exit_read(&ifp->if_maddrlock)`.
pub fn if_maddrlock_exit_read(ifp: &Ifnet) {
    ifp.rt().if_maddrlock.exit();
}

/// `ifnetlist` as LZ's callers name it (`IFNETLIST.0.iter()`): the kernel's list.
pub struct IfnetHead(pub &'static hal::list::List<IfnetList>);

/// `struct niqueue`: a network stack input queue.
pub struct Niqueue {
    /// `ni_q`.
    pub ni_q: MbufQueue,
    /// `ni_isr`: the `NETISR_*` bit to schedule.
    pub ni_isr: i32,
}

impl Niqueue {
    /// `NIQUEUE_INITIALIZER(len, isr)`.
    pub const fn new(len: u32, isr: i32) -> Self {
        Self {
            ni_q: MbufQueue::new(len, IPL_NET),
            ni_isr: isr,
        }
    }
}

/// `niq_dequeue(q)`.
pub fn niq_dequeue(q: &Niqueue) -> Option<&'static Mbuf> {
    mq_dequeue(&q.ni_q)
}

/// `niq_delist(q, ml)`.
pub fn niq_delist(q: &Niqueue, ml: &MbufList) {
    mq_delist(&q.ni_q, ml);
}

/// `niq_len(q)`.
pub fn niq_len(q: &Niqueue) -> u32 {
    mq_len(&q.ni_q)
}

/// `niq_drops(q)`.
pub fn niq_drops(q: &Niqueue) -> u32 {
    mq_drops(&q.ni_q)
}

/// `sysctl_niq(n, l, op, olp, np, nl, niq)`.
pub fn sysctl_niq(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
    niq: &Niqueue,
) -> Result<(), Errno> {
    sysctl_mq(name, oldp, oldlenp, newp, newlen, &niq.ni_q)
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::reftest::{assert_complete, assert_defines};

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/net/if_var.h");
        let ours = assert_defines!(defs;
            IF_TXMIT_MIN, IF_TXMIT_DEFAULT, IF_WIRED_DEFAULT_PRIORITY,
            IF_WIRELESS_DEFAULT_PRIORITY, IF_WWAN_DEFAULT_PRIORITY, IF_CARP_DEFAULT_PRIORITY);
        assert_complete(&defs, "IF_", &ours);
        assert_defines!(defs; IFNET_SLOWTIMO, IFA_ROUTE);
        assert_eq!(IFC_NCOUNTERS, 12);
    }
}
/* </TESTS> */
