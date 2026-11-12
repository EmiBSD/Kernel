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
 * Copyright (c) 1980, 1986, 1993
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
 *	@(#)route.h	8.3 (Berkeley) 4/19/94
 */

/*
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
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
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1980, 1986, 1991, 1993
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
 *	@(#)route.c	8.2 (Berkeley) 11/15/93
 */

/*
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! Routing tables and the routing socket's messages: `<net/route.h>` and `net/route.c`, for
//! `bsd`'s callers. The routes, their caches, timers and labels are the kernel's
//! (`kernel::net::route`, N6.3b) and route.h's ABI is `hal::sys::net_route`'s; this module is
//! their adapter, of no logic: the kernel's items under LZ's names, and the entry points in
//! the shapes `bsd`'s protocols call them with (a socket address as its bytes, a route as the
//! `&'static Rtentry` LZ's code keeps, the net lock held C-style).
//!
//! Upstream: sys/net/route.h @ 3ce1f3f79392
//! Upstream: sys/net/route.c @ 3ce1f3f79392
//!
//! A route the kernel answers (`rtalloc`, `rtrequest`'s `*ret_nrt`, `rtredirect`'s `*rtp`)
//! comes out as a `&'static Rtentry` whose counted reference is parked on the route
//! (`PoolArc::forget`), the C's `rtref`; `rtfree` adopts it back and drops it, so the count is
//! the C's (`rmx_refcnt`, D-N6.3b-19). A route the kernel lends for a call (a driver's
//! `if_rtrequest` and `if_output`, a timer queue's function, `rt_getll`'s next hop) is made
//! `&'static` by [`as_static`] without a count: the holder's reference keeps it for the call,
//! as the C's bare pointer.
//!
//! The kernel's functions take a proof that the net lock is held (`NetHeld`); `bsd`'s callers
//! hold it C-style, so each entry here makes one through the `SaToken`
//! (`crate::net::if_::net_held`), `panic(9)` without the lock where the C's
//! `NET_ASSERT_LOCKED()` would only print (N6.3a's adapters, the same).

use core::fmt;

use hal::pool::PoolArc;
use kernel::net::proto::SockaddrView;

use crate::kern::subr_prf::{db_printf, panic};
use crate::net::if_::net_held;
use crate::net::if_var::{Ifaddr, Ifnet};
use crate::netinet::in_::InAddr;
use crate::netinet6::in6::In6Addr;
use crate::sys::errno::Errno;
use crate::sys::types::SaFamily;

pub use hal::sys::net_route::*;
pub use kernel::net::route::{
    LABELID_MAX, RT_RESOLVE, RTCOUNTERS, RTGENERATION, RTS_BADREDIRECT, RTS_DYNAMIC, RTS_NCOUNTERS,
    RTS_NEWGATEWAY, RTS_UNREACH, RTS_WILDCARD, RTTRASH, Route, RtAddrinfo, RtKmetrics, Rtentry,
    RtqFunc, RttimerQueue, ifafree, ifaref, route_init, rt_hash, rt_if_linkstate_change,
    rt_maskedcopy, rt_plen2mask, rt_plentosa, rt_timer_get_expire, rt_timer_init,
    rt_timer_queue_change, rt_timer_queue_count, rt_timer_queue_init, rt_timer_remove_all,
    rtisvalid, rtlabel_id2name, rtlabel_id2sa, rtlabel_name2id, rtlabel_unref, rtstat_inc,
};
pub use kernel::net::rtable::SaCopy;

/// `rt` as the `&'static Rtentry` LZ's code keeps, for a route the kernel lends for a call.
pub fn as_static(rt: &Rtentry) -> &'static Rtentry {
    // SAFETY: `rt` is a route of the pool `"rtentry"` (a `PoolArc` item: the item stays
    // allocated, its value in place, until its last handle's drop, and is released only
    // after a grace period when the table's handle is the last, D-N6.3b-23). Every
    // `&Rtentry` `bsd` holds is lent by a holder of one of its counted references for as
    // long as `bsd` uses it, under the C's own conventions, which every `bsd` caller follows:
    // the kernel's calls of `NetDriver::rtrequest`/`output` and of a timer queue's function
    // (the caller's handle), `rtable_walk`'s callback (the walk's handle), `rt_getll`'s next
    // hop (the route's `rt_gwroute` handle, which changes only under the exclusive net lock
    // while `bsd`'s caller holds the net lock), a `Route` cache's route (the cache's handle,
    // kept while the caller uses it), and the references this module parks ([`rtalloc`],
    // [`rtref`]) until [`rtfree`]. No `bsd` path keeps one past its holder's release but
    // ARP's and ND's `la_rt`/`ln_rt`, which `arp_rtrequest`/`nd6_rtrequest` clear at
    // `RTM_DELETE`, before the route's last reference can go (the table's, released after
    // the delete). So the route outlives every use of this reference; `Rtentry` is `Sync`
    // (its members are atomics, read-mostly cells, links and a mutex).
    unsafe { &*core::ptr::from_ref(rt) }
}

/// The route `rt` given up for the `&'static` LZ's code keeps, its reference parked on it
/// until [`rtfree`] (the C's `rtref`'d pointer).
pub fn park(rt: PoolArc<Rtentry>) -> &'static Rtentry {
    let r = as_static(&rt);
    PoolArc::forget(rt);
    r
}

/// The counted reference [`park`] left on `rt`, taken back as a handle (a route `bsd` got
/// parked and now stores in a [`Route`]).
pub fn unpark(rt: &Rtentry) -> PoolArc<Rtentry> {
    match PoolArc::adopt(rt) {
        Some(r) => r,
        None => panic(format_args!("rtfree: route {rt:p} not referenced")),
    }
}

/// A counted handle to `rt`, a route the kernel lent.
pub(crate) fn handle(rt: &Rtentry) -> PoolArc<Rtentry> {
    match PoolArc::from_ref(rt) {
        Some(r) => r,
        None => panic(format_args!("route {rt:p} not in the rtentry pool")),
    }
}

/// The socket address in `b` (its `sa_len` bytes); `panic(9)` when `b` is shorter than its
/// header or its `sa_len`, the C's read past the address.
pub fn sa_view(b: &[u8]) -> SockaddrView<'_> {
    match SockaddrView::new(b) {
        Some(v) => v,
        None => panic(format_args!("a socket address shorter than its sa_len")),
    }
}

/// The bytes of a socket address structure (`sockaddr_in`, `sockaddr_in6`, ...), the C's
/// `sintosa(&sin)`.
pub fn sa_of<T: hal::user::AbiPod>(sa: &T) -> &[u8] {
    hal::user::bytes_of(core::slice::from_ref(sa))
}

/// `rt_key(rt)`: the destination's bytes.
pub fn rt_key(rt: &Rtentry) -> &[u8] {
    rt.dest()
}

/// `rtalloc(dst, flags, rtableid)`: the route to `dst`, its reference parked for [`rtfree`].
pub fn rtalloc(dst: &[u8], flags: i32, rtableid: u32) -> Option<&'static Rtentry> {
    kernel::net::route::rtalloc(sa_view(dst), flags, rtableid, &net_held("rtalloc")).map(park)
}

/// `rtalloc_mpath(dst, src, rtableid)`: the route to `dst` (a multipath one chosen by `src`),
/// its reference parked for [`rtfree`].
pub fn rtalloc_mpath(dst: &[u8], src: Option<&[u32]>, rtableid: u32) -> Option<&'static Rtentry> {
    kernel::net::route::rtalloc_mpath(sa_view(dst), src, rtableid, &net_held("rtalloc_mpath"))
        .map(park)
}

/// `rtref(rt)`: one more reference to `rt`, parked for [`rtfree`].
pub fn rtref(rt: &Rtentry) {
    PoolArc::forget(handle(rt));
}

/// `rtfree(rt)`: a reference [`rtalloc`] or [`rtref`] parked given back; the last one frees
/// the route.
pub fn rtfree(rt: Option<&Rtentry>) {
    if let Some(rt) = rt {
        drop(unpark(rt));
    }
}

/// `rt_getll(rt)`: the link-layer route of `rt` (its next hop's for a gateway route).
pub fn rt_getll(rt: &Rtentry) -> Option<&'static Rtentry> {
    kernel::net::route::rt_getll(rt, |r| r.map(as_static))
}

/// A `ret_nrt` slot of `bsd`'s (a parked reference or none) as the kernel's.
fn slot_in(s: &Option<&'static Rtentry>) -> Option<PoolArc<Rtentry>> {
    s.map(unpark)
}

/// `rtrequest(req, info, prio, ret_nrt, tableid)`, the answer's reference parked.
pub fn rtrequest(
    req: u8,
    info: &mut RtAddrinfo<'_>,
    prio: u8,
    ret_nrt: Option<&mut Option<&'static Rtentry>>,
    tableid: u32,
) -> Result<(), Errno> {
    let net = net_held("rtrequest");
    match ret_nrt {
        Some(s) => {
            let mut r = slot_in(s);
            let e = kernel::net::route::rtrequest(req, info, prio, Some(&mut r), tableid, &net);
            *s = r.map(park);
            e
        }
        None => kernel::net::route::rtrequest(req, info, prio, None, tableid, &net),
    }
}

/// `rtrequest_delete(info, prio, ifp, ret_nrt, tableid)`, the answer's reference parked.
pub fn rtrequest_delete(
    info: &mut RtAddrinfo<'_>,
    prio: u8,
    ifp: &'static Ifnet,
    ret_nrt: Option<&mut Option<&'static Rtentry>>,
    tableid: u32,
) -> Result<(), Errno> {
    let net = net_held("rtrequest_delete");
    match ret_nrt {
        Some(s) => {
            let mut r = slot_in(s);
            let e =
                kernel::net::route::rtrequest_delete(info, prio, ifp, Some(&mut r), tableid, &net);
            *s = r.map(park);
            e
        }
        None => kernel::net::route::rtrequest_delete(info, prio, ifp, None, tableid, &net),
    }
}

/// `rtdeletemsg(rt, ifp, tableid)`.
pub fn rtdeletemsg(rt: &Rtentry, ifp: &'static Ifnet, tableid: u32) -> Result<(), Errno> {
    kernel::net::route::rtdeletemsg(&handle(rt), ifp, tableid, &net_held("rtdeletemsg"))
}

/// `rtredirect(dst, gateway, src, rtp, rdomain)`: the redirect's route given to `rtp`, its
/// reference parked.
pub fn rtredirect(
    dst: &[u8],
    gateway: &[u8],
    src: &[u8],
    rtp: Option<&mut Option<&'static Rtentry>>,
    rdomain: u32,
) {
    let net = net_held("rtredirect");
    let (dst, gateway, src) = (sa_view(dst), sa_view(gateway), sa_view(src));
    match rtp {
        Some(s) => {
            let mut r = slot_in(s);
            kernel::net::route::rtredirect(dst, gateway, src, Some(&mut r), rdomain, &net);
            *s = r.map(park);
        }
        None => kernel::net::route::rtredirect(dst, gateway, src, None, rdomain, &net),
    }
}

/// `rt_ifa_add(ifa, flags, dst, rdomain)`.
pub fn rt_ifa_add(ifa: &'static Ifaddr, flags: u32, dst: &[u8], rdomain: u32) -> Result<(), Errno> {
    kernel::net::route::rt_ifa_add(ifa, flags, sa_view(dst), rdomain, &net_held("rt_ifa_add"))
}

/// `rt_ifa_del(ifa, flags, dst, rdomain)`.
pub fn rt_ifa_del(ifa: &'static Ifaddr, flags: u32, dst: &[u8], rdomain: u32) -> Result<(), Errno> {
    kernel::net::route::rt_ifa_del(ifa, flags, sa_view(dst), rdomain, &net_held("rt_ifa_del"))
}

/// `rt_ifa_addlocal(ifa)`.
pub fn rt_ifa_addlocal(ifa: &'static Ifaddr) -> Result<(), Errno> {
    kernel::net::route::rt_ifa_addlocal(ifa, &net_held("rt_ifa_addlocal"))
}

/// `rt_ifa_dellocal(ifa)`.
pub fn rt_ifa_dellocal(ifa: &'static Ifaddr) -> Result<(), Errno> {
    kernel::net::route::rt_ifa_dellocal(ifa, &net_held("rt_ifa_dellocal"))
}

/// `rt_ifa_purge(ifa)`.
pub fn rt_ifa_purge(ifa: &'static Ifaddr) {
    kernel::net::route::rt_ifa_purge(ifa, &net_held("rt_ifa_purge"));
}

/// `rt_timer_add(rt, queue, rtableid)`.
pub fn rt_timer_add(
    rt: &Rtentry,
    queue: &'static RttimerQueue,
    rtableid: u32,
) -> Result<(), Errno> {
    kernel::net::route::rt_timer_add(&handle(rt), queue, rtableid)
}

/// `rt_timer_queue_flush(rtq)`.
pub fn rt_timer_queue_flush(rtq: &'static RttimerQueue) {
    kernel::net::route::rt_timer_queue_flush(rtq, &net_held("rt_timer_queue_flush"));
}

/// `rt_if_track(ifp)`.
pub fn rt_if_track(ifp: &'static Ifnet) -> Result<(), Errno> {
    kernel::net::route::rt_if_track(ifp, &net_held("rt_if_track"))
}

/// `route_cache(ro, dst, src, rtableid)` (`Route::cache4`).
pub fn route_cache(
    ro: &mut Route,
    dst: &InAddr,
    src: Option<&InAddr>,
    rtableid: u32,
) -> Result<(), Errno> {
    ro.cache4(
        dst.s_addr.to_ne_bytes(),
        src.map(|s| s.s_addr.to_ne_bytes()),
        rtableid,
    )
}

/// `route_mpath(ro, dst, src, rtableid)` (`Route::mpath4`): the cached route, lent.
pub fn route_mpath(
    ro: &mut Route,
    dst: &InAddr,
    src: Option<&InAddr>,
    rtableid: u32,
) -> Option<&'static Rtentry> {
    ro.mpath4(
        dst.s_addr.to_ne_bytes(),
        src.map(|s| s.s_addr.to_ne_bytes()),
        rtableid,
        &net_held("route_mpath"),
    )
    .map(as_static)
}

/// `route6_cache(ro, dst, src, rtableid)` (`Route::cache6`).
pub fn route6_cache(
    ro: &mut Route,
    dst: &In6Addr,
    src: Option<&In6Addr>,
    rtableid: u32,
) -> Result<(), Errno> {
    ro.cache6(&dst.s6_addr, src.map(|s| &s.s6_addr), rtableid)
}

/// `route6_mpath(ro, dst, src, rtableid)` (`Route::mpath6`): the cached route, lent.
pub fn route6_mpath(
    ro: &mut Route,
    dst: &In6Addr,
    src: Option<&In6Addr>,
    rtableid: u32,
) -> Option<&'static Rtentry> {
    ro.mpath6(
        &dst.s6_addr,
        src.map(|s| &s.s6_addr),
        rtableid,
        &net_held("route6_mpath"),
    )
    .map(as_static)
}

/// A protocol control block's route cache (`Inpcb::inp_route`, `SynCache::sc_route`): the
/// kernel's owned [`Route`] in a cell of the pcb, which `bsd` reaches through `&`.
/// [`RouteCell::loan`] takes the route out for a call and puts it back when the loan ends
/// (docs/SCAFFOLDING.md, D-N6.3b-7).
///
/// Every method moves the route (a loan out and back, `take`, `set`): each is a write, so
/// the caller holds the lock of the cache's owner, the `[s]` of the C (the socket lock of
/// the pcb; for a syn cache entry the listening socket's, or the last reference), and never
/// reads another owner's cache under a lock of its own: two threads' moves of one cell race.
/// `Inpcb::route` and `Inpcb::set_route` check the pcb's socket lock before they call these.
pub struct RouteCell(core::cell::Cell<Option<Route>>);

impl RouteCell {
    /// An empty cache.
    pub const fn new() -> Self {
        Self(core::cell::Cell::new(Some(Route::new())))
    }

    /// The route taken out, an empty cache left (the C's struct assignment out of a cache
    /// whose `ro_rt` is then cleared).
    pub fn take(&self) -> Route {
        self.0.take().unwrap_or_default()
    }

    /// The route put in, the old one dropped (its handle given back).
    pub fn set(&self, ro: Route) {
        drop(self.0.replace(Some(ro)));
    }

    /// The route, lent until the loan's drop.
    pub fn loan(&self) -> RouteLoan<'_> {
        RouteLoan {
            cell: self,
            ro: Some(self.0.take().unwrap_or_default()),
        }
    }
}

impl Default for RouteCell {
    fn default() -> Self {
        Self::new()
    }
}

/// A [`RouteCell`]'s route, out of its cell until this drops.
pub struct RouteLoan<'a> {
    /// Where the route goes back.
    cell: &'a RouteCell,
    /// The route.
    ro: Option<Route>,
}

impl core::ops::Deref for RouteLoan<'_> {
    type Target = Route;
    fn deref(&self) -> &Route {
        match &self.ro {
            Some(r) => r,
            None => panic(format_args!("RouteLoan: no route")),
        }
    }
}

impl core::ops::DerefMut for RouteLoan<'_> {
    fn deref_mut(&mut self) -> &mut Route {
        match &mut self.ro {
            Some(r) => r,
            None => panic(format_args!("RouteLoan: no route")),
        }
    }
}

impl Drop for RouteLoan<'_> {
    fn drop(&mut self) {
        self.cell.0.set(self.ro.take());
    }
}

/// ddb's console as a `fmt::Write`.
struct DbOut;

impl fmt::Write for DbOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        db_printf(format_args!("{s}"));
        Ok(())
    }
}

/// `db_show_rtentry(rt, id)` on ddb's console; the interface addresses need the net lock's
/// proof, so without the lock only that is said.
pub fn db_show_rtentry(rt: &Rtentry, _id: u32) -> Result<(), Errno> {
    match crate::net::if_var::sa_token().net_held() {
        Some(net) => {
            let _ = kernel::net::route::db_show_rtentry(&mut DbOut, rt, &net);
        }
        None => {
            db_printf(format_args!("rtentry={rt:p}: the net lock is not held\n"));
        }
    }
    Ok(())
}

/// `db_show_rtable(af, rtableid)` on ddb's console (as [`db_show_rtentry`] without the net
/// lock).
pub fn db_show_rtable(af: SaFamily, rtableid: u32) -> Result<(), Errno> {
    match crate::net::if_var::sa_token().net_held() {
        Some(net) => {
            let _ = kernel::net::route::db_show_rtable(&mut DbOut, af, rtableid, &net);
        }
        None => {
            db_printf(format_args!(
                "rtable {rtableid}: the net lock is not held\n"
            ));
        }
    }
    Ok(())
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
        let defs = crate::reftest::defines("sys/net/route.h");
        let rtf = assert_defines!(defs;
        RTF_UP, RTF_GATEWAY, RTF_HOST, RTF_REJECT, RTF_DYNAMIC, RTF_MODIFIED, RTF_DONE,
        RTF_CLONING, RTF_MULTICAST, RTF_LLINFO, RTF_STATIC, RTF_BLACKHOLE, RTF_PROTO3,
        RTF_PROTO2, RTF_ANNOUNCE, RTF_PROTO1, RTF_CLONED, RTF_CACHED, RTF_MPATH, RTF_MPLS,
        RTF_LOCAL, RTF_BROADCAST, RTF_CONNECTED, RTF_BFD);
        // RTF_FMASK spans lines in the C.
        assert_eq!(RTF_FMASK, 0x0110_fc08);
        assert_complete(&defs, "RTF_", &[&rtf[..], &["RTF_FMASK"]].concat());
        let rtp = assert_defines!(defs;
        RTP_NONE, RTP_LOCAL, RTP_CONNECTED, RTP_STATIC, RTP_EIGRP, RTP_OSPF, RTP_ISIS,
        RTP_RIP, RTP_BGP, RTP_DEFAULT, RTP_PROPOSAL_STATIC, RTP_PROPOSAL_DHCLIENT,
        RTP_PROPOSAL_SLAAC, RTP_PROPOSAL_UMB, RTP_PROPOSAL_PPP, RTP_PROPOSAL_SOLICIT,
        RTP_MAX, RTP_ANY, RTP_MASK, RTP_DOWN);
        assert_complete(&defs, "RTP_", &rtp);
        let rtm = assert_defines!(defs;
        RTM_RTTUNIT, RTM_VERSION, RTM_MAXSIZE, RTM_ADD, RTM_DELETE, RTM_CHANGE, RTM_GET,
        RTM_LOSING, RTM_REDIRECT, RTM_MISS, RTM_RESOLVE, RTM_NEWADDR, RTM_DELADDR,
        RTM_IFINFO, RTM_IFANNOUNCE, RTM_DESYNC, RTM_INVALIDATE, RTM_BFD, RTM_PROPOSAL,
        RTM_CHGADDRATTR, RTM_80211INFO, RTM_SOURCE);
        assert_complete(&defs, "RTM_", &rtm);
        let rtv = assert_defines!(defs;
        RTV_MTU, RTV_HOPCOUNT, RTV_EXPIRE, RTV_RPIPE, RTV_SPIPE, RTV_SSTHRESH, RTV_RTT,
        RTV_RTTVAR);
        assert_complete(&defs, "RTV_", &rtv);
        let rta = assert_defines!(defs;
        RTA_DST, RTA_GATEWAY, RTA_NETMASK, RTA_GENMASK, RTA_IFP, RTA_IFA, RTA_AUTHOR,
        RTA_BRD, RTA_SRC, RTA_SRCMASK, RTA_LABEL, RTA_BFD, RTA_DNS, RTA_STATIC, RTA_SEARCH);
        assert_complete(&defs, "RTA_", &rta);
        let rtax = assert_defines!(defs;
        RTAX_DST, RTAX_GATEWAY, RTAX_NETMASK, RTAX_GENMASK, RTAX_IFP, RTAX_IFA, RTAX_AUTHOR,
        RTAX_BRD, RTAX_SRC, RTAX_SRCMASK, RTAX_LABEL, RTAX_BFD, RTAX_DNS, RTAX_STATIC,
        RTAX_SEARCH, RTAX_MAX);
        assert_complete(&defs, "RTAX_", &rtax);
        let route = assert_defines!(defs;
        ROUTE_MSGFILTER, ROUTE_TABLEFILTER, ROUTE_PRIOFILTER, ROUTE_FLAGFILTER);
        assert_complete(&defs, "ROUTE_", &route);
        let rest = assert_defines!(defs;
        RTABLE_ANY, RTLABEL_LEN, RTDNS_LEN, RTSTATIC_LEN, RTSEARCH_LEN, RT_RESOLVE);
        assert_complete(&defs, "RT_", &rest);
        assert_eq!(RTS_NCOUNTERS, 5);
    }
}
/* </TESTS> */
