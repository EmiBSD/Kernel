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
 *	@(#)if.c	8.3 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! Network interfaces: `net/if.c`, the interface index map, the softnet task queues and
//! their barriers, the protocol software interrupt (`if_netisr`), the softnet input path
//! (`if_input_process`), the dispatch to an interface's driver, and the interface requests
//! (`ifioctl`: the `'i'` group of `ioctl(2)` on a socket, `ifconf`, the groups' and the
//! cloners' lists).
//!
//! Upstream: sys/net/if.h @ 3ce1f3f79392
//! Upstream: sys/net/if.c @ 3ce1f3f79392
//! LZ: sys/net/if_.rs@3c62ede68802
//!
//! `if.c` keeps the interfaces on `ifnetlist` and in an index map (`if_get(index)` takes a
//! reference, `if_put` drops it); attaches them (`if_attach`: an index, a send queue `if_snd`
//! and an input queue `if_rcv` bound to a softnet task queue); passes received packets from
//! the input queues to the driver's `if_input` and the protocols in the softnet threads
//! (`if_input_process`, `if_input_proto`); and owns the `netisr` word (`if_netisr`).
//! `<net/if.h>`'s constants and structures are `hal::sys::net_if`'s.
//!
//! Locks and their order (the C's): the index map's `idxmaplk` (an rwlock, taken alone or
//! inside the net lock), the net lock (`hal::netlock::NETLOCK`: `if_input_process` shared,
//! `if_netisr` exclusive). The index map is read under SMR (`hal::smr`): `if_get` is a read
//! section, two loads and a reference count, as the C's. Transitions against the C: `if_get`
//! and `if_put`, one SMR section and two atomics each way (the C's); `if_input_process`, one
//! shared net lock pair around its batch (the C's).
//!
//! What `if.c` asks of the subsystems that have not moved goes through two bridges
//! (bridges.toml), registered by `bsd` at boot, each answering as the C compiled without the
//! subsystem before registration: [`Inet`] (netinet, netinet6: the protocols' software
//! interrupts and `tcp_input_mlist`) and [`Link`] (bpf(4) and `if_ethersubr.c`); pf's legs
//! are the `Pf` bridge's (`kernel::net::pf_hooks`). The routing code is the kernel's
//! (`kernel::net::{route, rtable, rtsock}`, N6.3b).
//!
//! Lock and spl transitions of the operations against the C (if.c's line numbers at the
//! pin), each the C's count:
//!
//! | operation | the C | here |
//! |---|---|---|
//! | `if_attach` (:588-610) | `NET_LOCK` around the list insert and `if_attachsetup`; `idxmaplk` in `if_idxmap_alloc` and `_insert` | the same (`NETLOCK.write()`, the map's lock twice) |
//! | `if_remove` (:1213-1228) | `NET_LOCK` around the list removal, `idxmaplk`, `smr_barrier`, the softnet barriers, `refcnt_finalize` | the same, but for the softnet barriers (`net_tq_barriers`, if.c:3823-3835: stack tasks and a stack `struct refcnt`, no lock): the `netbarlk` rwlock around the call, which serialises its callers over the softnets' own barrier tasks, and the barrier count's `IPL_NET` mutex once per softnet and once per wait (detach and the protocols' teardown only, never per packet) |
//! | `if_detach` (:1265-1375) | `NET_LOCK` + `splnet` around the protocols' detach, the groups, the link name and the left addresses | the same, `if_deactivate`'s `NET_LOCK` before it; `splnet` ends before the groups (review 1's S2: removing a member allocates the group's smaller array `M_WAITOK`, which needs `IPL_NONE`; no interrupt handler reads the groups, the link name or the address list), so one `splnet` pair as the C's |
//! | `if_get`/`if_put` (:1984-2024) | one SMR read section, two loads, a count each way | the same |
//! | `if_input_process` (:1031-1093) | `NET_LOCK_SHARED` around the batch | the same |
//! | `if_addgroup`/`if_delgroup` (:3068-3167) | none (the caller's `NET_LOCK`) | none: each changed list is one `ReadMostly` claim (a compare-and-swap) |
//! | `if_clone_create`/`_destroy` (:1408-1475) | `if_cloners_lock`, then `NET_LOCK` inside | the same |
//! | `ifpromisc`, `ifsetlro`, `ifnewlladdr` (:3478-3650) | the caller's locks, `splnet` in `ifsetlro` | the same |
//! | `ifioctl(SIOCSIFFLAGS)` (:2193-2208, :2318-2340) | `KERNEL_LOCK`, then `NET_LOCK` around the flags, the driver's `if_ioctl` and `if_up`/`if_down` (`splnet` there) | the same: the kernel lock, then the net lock as a detached holding (`hal::netlock::lock`), one `splnet` pair when `IFF_UP` changes |
//! | `ifioctl`'s other requests (:2136-2597) | `KERNEL_LOCK` around the per-interface requests, `NET_LOCK` inside where each takes it; none for `SIOCGIFSFFPAGE`, `SIOCSIFMEDIA`, `SIOCGIFMEDIA`; `SIOCIFCREATE`/`DESTROY` the kernel lock alone, `SIOCSIFGATTR` both | the same, request by request |
//! | `ifioctl_get` (:2599-2712) | `KERNEL_LOCK` around `if_unit`; `NET_LOCK_SHARED` then `KERNEL_LOCK` for `if_getdata`; `NET_LOCK_SHARED` for `SIOCGIFGATTR` | the same (`NETLOCK.read()`) |
//! | `ifconf` (:2799-2930) | `if_tmplist_lock`, one `NET_LOCK_SHARED` for the interfaces, one per interface for its addresses | the same for an interface of up to 8 addresses; one more shared hold per further 8 (the redesign's `ifconf` paragraph) |
//! | the group requests (:3169-3380) | `if_tmplist_lock` and `NET_LOCK_SHARED` around the temporary lists | `NET_LOCK_SHARED` around the snapshot; no `if_tmplist_lock` (a snapshot is no shared list) |
//!
//! ## Deviations
//! - Without `MULTIPROCESSOR` `NET_TASKQ` is 1; the kernel is always `MULTIPROCESSOR`
//!   (decision 40): `softnet_init` makes eight softnet queues and `softnet_percpu` destroys
//!   those past `softnet_count()`, one per CPU, whose memory stays (`kernel::task`'s
//!   `taskq_destroy_leaked`, LZ's: six `taskqpl` items on two CPUs, four on four).
//! - `malloc(M_WAITOK)` that fails panics where the C would sleep (the index map's arrays;
//!   `malloc(9)` does not sleep yet; LZ's).
//! - The index map's object is an item of the pool "ifmappl" (`hal::smr::SmrPtr` publishes
//!   pool items only, D-N6.3a-5): `vmstat -m` shows one pool row the C has not; the map's
//!   slot array and the used-index bitmap keep the C's `malloc(M_IFADDR)`s and sizes.
//! - `if_alloc_sadl` `panic(9)`s for a link name longer than a `sockaddr_dl` (an interface
//!   name and a link address of more than 24 bytes together): the box is one `sockaddr_dl`,
//!   which every configured link type fits (`malloc(M_IFADDR)` of 32 bytes, the C's size).
//! - `if_detach` leaves the arrays of the extra queues of `if_attach_queues` and
//!   `if_attach_iqueues` allocated (`M_DEVBUF`, two arrays and their queues for a
//!   multi-queue driver): they live in the interface's set-once `IfRt`, which cannot be
//!   emptied; the queues themselves are torn down (`ifq_destroy`, `ifiq_destroy`). No
//!   multi-queue driver here detaches (vio, vmx); N7's `NetDevice` owns them.
//! - The groups' lists are arrays replaced whole (the module's redesign): `vmstat -m`'s
//!   "ifgroup" row counts one array per change where the C counts one `ifg_list` and one
//!   `ifg_member` per membership; the removal allocates its smaller array `M_WAITOK` (so
//!   `if_detach` ends its `splnet` section before the groups, review 1's S2).
//! - `if_attach_queues`/`if_attach_iqueues` publish the extra queues (the set-once array in
//!   `IfRt`) and then set their tasks (`ifq_init_tasks`, `ifiq_init_tasks`: a task's object
//!   must be `'static`, which a queue is once in the array), where the C's `ifq_init` sets
//!   them before the array is stored. The drivers call them at attach, before the interface
//!   is up, so no packet reaches an extra queue meanwhile; a task reached without its
//!   function would be `panic(9)`, never a stale call (review nit, kept).
//! - The cloners are at most 16 (a table filled at boot; `if_clone_attach` `panic(9)`s past
//!   it); the C's list has no bound.
//! - `if_slowtimo`'s first run follows `if_idxmap_insert` in `if_attachsetup` (the C runs it
//!   just before): the timer finds the interface by its index (D-N6.3a-7). Its first run
//!   arms it one second later in both orders.
//! - `INET6` is the `Inet` bridge's: configured once it is registered (GENERIC's option,
//!   `bsd`'s default feature). Before that `SIOCIFAFATTACH`/`DETACH` of `AF_INET6` answer
//!   `EAFNOSUPPORT` and `SIOCSIFXFLAGS` skips the IPv6 flags, as the C without `INET6`.
//! - `ifioctl`'s request of the kernel itself comes with no socket and goes to `in_ioctl` as a
//!   privileged one (the boot self-test configures an interface that way; LZ's): the C's
//!   `pru_control` follows the socket.
//! - `SIOCSIFXFLAGS`'s wake on LAN with a driver that has no `if_wol` answers `ENOTSUP` (the
//!   C calls the handle unchecked once `IFCAP_WOL` is set; LZ's).
//! - `ifconf` copies an address's `sa_len` bytes cut at its storage (`hal::link::SaLink`'s
//!   copy), which every interface address fills; the row's length is its `sa_len`, as the C's.
//!
//! ## Redesign
//! - LZ's `struct if_idxmap` of `Cell`s and raw arrays (slot 0 holding the length, the old
//!   bitmap reused as the `smr_call` destructor of the old map) -> [`IfMap`], an
//!   `SmrPtr`-published pool item holding its length and its slots (`MArray<Link<Ifnet>>`,
//!   the C's `malloc` and size), beside the used-index bitmap and the serial under
//!   `idxmaplk` (a `Guarded` keyed to that lock). Growth copies every slot into a map twice
//!   as long (a reference taken per slot, the C's `if_ref`), publishes it and retires the
//!   old one: the SMR thread drops it after the grace period, and its `Drop` puts each slot's
//!   reference (`if_idxmap_free`). Cost as the C's: `insert`/`remove` O(1), `alloc` O(limit)
//!   at worst (the free-index scan).
//! - The C's `if_get` returns a counted pointer the caller must `if_put`: [`if_get`] returns
//!   the `&'static Ifnet` with the count taken (the C's two-owner protocol, kept for `bsd`'s
//!   callers), and [`IfRef`] is the guard kernel code uses, which puts it when dropped.
//! - `struct netstack` is a value on `if_input_process`'s stack, lent `&mut` to each input
//!   function: the C reinitialises the softnet's own at each call, so its lists carry nothing
//!   between calls (D-N6.3a-10). The C's `ns_route` (`struct route`, `route.c`'s) has no
//!   home yet: `bsd`'s `ip_input`/`ip6_input` use a route of the call, one route lookup per
//!   received packet more than the C, until N6.3b (`bsd`'s `Route` of `Cell`s cannot be a
//!   shared static without a new `unsafe impl Sync`; docs/SCAFFOLDING.md).
//! - `if_input_proto` keeps the protocol's input function in the packet's `ph_cookie`, as the
//!   C, as an index into a table of the functions seen ([`proto_fn_id`]): a function pointer
//!   is not made back from a word without `unsafe`.
//! - `TAILQ_HEAD(, ifg_list) if_groups`, `ifg_head` and each group's `ifg_members` (lists of
//!   `malloc`ed links the C walks under the net lock, `bsd`'s pf walking them too) -> arrays of
//!   counted references (`IfgSet`, `IfgMembers` in `ReadMostly` cells) replaced whole by
//!   `if_addgroup` and `if_delgroup`: a reader borrows the array it finds (pf's walks hold the
//!   net lock *detached*, which no `Guarded` keyed to the net lock can serve, D-N6.3a-2), a
//!   writer installs a new one once no reader is inside, and a group is freed with its last
//!   reference (`MArc`). D-N6.3a-6's owned chains under a net-lock `Guarded` assumed `bsd`
//!   never walks the lists; `bsd`'s pf_if.rs does (six walks). Cost against the C: one
//!   compare-and-swap per walk (pf's `pfi_kif_match` per rule with an interface group, the
//!   ioctls), an O(n) copy per membership change where the C links in O(1); its removal by
//!   name O(n) as the C's search (N12's row).
//! - `struct if_clone`'s list -> a table of `&'static dyn Cloner` filled at boot (the C's
//!   list is immutable after `main()`), walked from the last registered as the C's
//!   `LIST_INSERT_HEAD` order; `if_cloners_lock` kept for creation and destruction.
//! - `if_output_ml`, `if_output_tso` and `if_output_mq` are not here: they call the driver's
//!   `if_output` with `bsd`'s route and socket address, and stay `bsd`'s with routing until
//!   N6.3b (D-N6.3a-9); `if_output_local` is.
//! - `ifa_ifwithaddr`, `ifa_ifwithdstaddr`, `ifaof_ifpforaddr` compare the addresses' bytes
//!   where the C compares through pointers: each interface address is a copy
//!   (`hal::link::SaLink::read`, under the net lock it checks: `sa_len` bytes cut at the
//!   storage, and the family whatever `sa_len` says); an address shorter than the other's
//!   length never matches (the C reads past it). The copy and the lock check are one per
//!   address walked, beyond the C's loads (review 1's B2; the decision-43 row
//!   `ifa_ifwithaddr.8x2`).
//! - `if_detach`'s swap of `if_qstart` and `if_ioctl` for the detached stubs is two bits of
//!   the interface; `if_watchdog = NULL` is not needed: the watchdog's timeout and task find
//!   the interface by its index, which `if_remove` freed before it.
//! - `ifioctl(so, cmd, data, p)` (`caddr_t data`, cast to each request's structure) works on
//!   `&mut [u8]`, the request viewed as its `abi_pod!` image (`hal::pod::view_mut`): in place
//!   when it is aligned and at least an `ifreq` long (`sys_ioctl`'s buffers are), else on an
//!   aligned zeroed copy (64 bytes on the stack, an `M_IOCTLOPS` block past that) copied back
//!   after, so no request is read past its end or misaligned (N6.2a review's B1, `bsd`'s
//!   `ifioctl_buffer`, which goes). A request is reread after each driver call, as the C reads
//!   `ifr` after it (`SIOCDIFGROUP`'s group, `SIOCSIFLLADDR`'s address). The user's
//!   memory is reached through `hal::user::UserPtr` only (`SIOCGIFDATA`, `SIOCSIFDESCR`, the
//!   rows of `ifconf`, of the groups and of the cloners, `if_rxr_info_ioctl`).
//! - `ifioctl` and `if_downall` take the net lock as a detached holding
//!   (`hal::netlock::lock`/`unlock`, the C's `NET_LOCK()`), not a witness: the drivers they
//!   call through `if_ioctl` let it go and take it back (wg(4)'s `wg_up`/`wg_down`,
//!   pflow(4)'s `pflowioctl`, pfsync(4)'s `pfsync_up`/`pfsync_down`), which only a detached
//!   holding allows. `ifioctl`'s read-only paths take witnesses (`NETLOCK.read()`), which
//!   they hold across no driver call.
//! - `SIOCSIFXFLAGS`'s `goto forceup` into `SIOCSIFFLAGS` is a call of the one `forceup`
//!   function from both, inside the same net lock section.
//! - `ifconf` keeps the C's temporary lists (`if_tmplist`, `ifa_tmplist`, references taken)
//!   under `if_tmplist_lock`; an address's bytes are a copy that needs the net lock
//!   (`SaLink::read`), so each interface's shared hold that lists its addresses also copies
//!   the first eight of them, and an interface with more takes the lock again per further
//!   eight before it copies them out with the lock let go: the C's count of lock sections up
//!   to eight addresses, one more per eight after (N6.4/N6.5, when the address embeds its
//!   storage, reads it under the reference alone). As the C, `ifc_len` is an `int` compared
//!   with a `size_t` for the `sockaddr`-sized rows (a negative length bounds none of them)
//!   and with an `int` for the longer ones.
//! - The group requests copy out of a snapshot (`IfgSet::snapshot`, the members' with a
//!   reference to each interface) taken under the shared net lock: no `if_tmplist_lock`, no
//!   temporary references to the groups.
//! - `net_tq_barriers`' stack tasks with a `struct refcnt` on the stack -> one barrier task per
//!   softnet in the softnet, its count the barrier's own; the callers are serialised by the
//!   barrier's lock (the C's `net_tq_barriers` callers hold the net lock or nothing, and wait
//!   for every softnet in turn either way).

use core::any::Any;
use core::mem::size_of;
use core::pin::Pin;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::intr::IPL_NET;
use hal::link::Link as HalLink;
use hal::link::SaBytes;
use hal::list::List;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::mutex::Mutexed;
use hal::netlock::{NETLOCK, NetHeld};
use hal::once::OnceStatic;
use hal::pool::BoxPool;
use hal::rwlock::{Guarded, RW_READ, RW_WRITE, RawRwlock};
use hal::slot::ReadMostly;
use hal::smr::{self, SmrLink, SmrLinked, SmrPtr, SmrRead};
use hal::sys::errno::Errno;
use hal::sys::ioccom::{_ioc, IOC_IN, iocparm_len};
use hal::sys::kernel::HZ;
use hal::sys::net_bpf::{BPF_DIRECTION_IN, BPF_DIRECTION_OUT};
use hal::sys::net_if::{
    IF_HDRPRIO_MAX, IF_HDRPRIO_MIN, IF_HDRPRIO_OUTER, IF_HDRPRIO_PACKET, IF_HDRPRIO_PAYLOAD,
    IFAN_ARRIVAL, IFAN_DEPARTURE, IFCAP_LRO, IFCAP_TSOv4, IFCAP_TSOv6, IFCAP_WOL, IFDESCRSIZE,
    IFF_CANTCHANGE, IFF_LOOPBACK, IFF_OACTIVE, IFF_POINTOPOINT, IFF_PROMISC, IFF_UP, IFG_ALL,
    IFG_EGRESS, IFNAMSIZ, IFQ_DEFPRIO, IFQ_MAXPRIO, IFQ_MINPRIO, IFSFF_ADDR_DDM, IFSFF_ADDR_EEPROM,
    IFT_CARP, IFT_ETHER, IFT_ISO88025, IFT_XETHER, IFXF_AUTOCONF4, IFXF_AUTOCONF6,
    IFXF_AUTOCONF6TEMP, IFXF_CANTCHANGE, IFXF_CLONED, IFXF_INET6_NOSOII, IFXF_LRO, IFXF_MONITOR,
    IFXF_MPSAFE, IFXF_WOL, IfAfreq, IfClonereq, IfData, IfRxrinfo, IfRxring, IfRxringInfo,
    IfSffpage, IfTimeval, Ifconf, IfgAttrib, IfgReq, Ifgroupreq, Ifreq, SDL_DATA_OFFSET,
    SockaddrDl,
};
use hal::sys::net_route::{RTF_BLACKHOLE, RTF_REJECT, RTP_ANY};
use hal::sys::socket::{AF_INET, AF_INET6, AF_LINK, AF_MAX, RT_TABLEID_MAX, Sockaddr};
use hal::sys::sockio::{
    SIOCADDMULTI, SIOCAIFADDR, SIOCAIFGROUP, SIOCDELLABEL, SIOCDELMULTI, SIOCDIFADDR, SIOCDIFGROUP,
    SIOCDIFPARENT, SIOCDIFPHYADDR, SIOCDPWE3NEIGHBOR, SIOCDVNETID, SIOCGIFCONF, SIOCGIFDATA,
    SIOCGIFDESCR, SIOCGIFFLAGS, SIOCGIFGATTR, SIOCGIFGLIST, SIOCGIFGMEMB, SIOCGIFGROUP,
    SIOCGIFHARDMTU, SIOCGIFLLPRIO, SIOCGIFMEDIA, SIOCGIFMETRIC, SIOCGIFMTU, SIOCGIFPRIORITY,
    SIOCGIFRDOMAIN, SIOCGIFRTLABEL, SIOCGIFSFFPAGE, SIOCGIFXFLAGS, SIOCIFAFATTACH, SIOCIFAFDETACH,
    SIOCIFCREATE, SIOCIFDESTROY, SIOCIFGCLONERS, SIOCSETKALIVE, SIOCSETLABEL, SIOCSETMPWCFG,
    SIOCSIFADDR, SIOCSIFBRDADDR, SIOCSIFDESCR, SIOCSIFDSTADDR, SIOCSIFFLAGS, SIOCSIFGATTR,
    SIOCSIFLLADDR, SIOCSIFLLPRIO, SIOCSIFMEDIA, SIOCSIFMETRIC, SIOCSIFMTU, SIOCSIFNETMASK,
    SIOCSIFPAIR, SIOCSIFPARENT, SIOCSIFPRIORITY, SIOCSIFRDOMAIN, SIOCSIFRTLABEL, SIOCSIFXFLAGS,
    SIOCSLIFPHYADDR, SIOCSLIFPHYDF, SIOCSLIFPHYECN, SIOCSLIFPHYRTABLE, SIOCSLIFPHYTTL,
    SIOCSPWE3CTRLWORD, SIOCSPWE3FAT, SIOCSPWE3NEIGHBOR, SIOCSRXHPRIO, SIOCSTXHPRIO,
    SIOCSVNETFLOWID, SIOCSVNETID,
};
use hal::sys::types::SaFamily;
use hal::user::UserPtr;

use crate::assert::{kassert, kpanic};
use crate::net::if_var::{
    AtomicCell, Cloner, DETACHED_IOCTL, DETACHED_QSTART, IF_TXMIT_DEFAULT, IFC_NCOUNTERS,
    IFNET_SLOWTIMO, IfCounterArray, IfCounters, IfRt, IfTmplist, IfaTmplist, Ifaddr, IfgGroup,
    IfgMembers, IfgRef, IfgSet, Ifnet, IfnetList, NetDriver, Netstack, counters_pkt,
};
use crate::net::ifq::{
    Ifiqueue, Ifqueue, QueueDiscipline, ifiq_add_data, ifiq_destroy, ifiq_enqueue,
    ifiq_enqueue_qlim, ifiq_init_tasks, ifq_add_data, ifq_attach, ifq_barrier, ifq_clr_oactive,
    ifq_destroy, ifq_enqueue, ifq_idx, ifq_init_tasks, ifq_purge, ifq_start,
};
use crate::net::mbuf::{
    M_BCAST, M_FLOWID, M_ICMP_CSUM_IN_OK, M_ICMP_CSUM_OUT, M_IPV4_CSUM_IN_OK, M_IPV4_CSUM_OUT,
    M_LOOP, M_MCAST, M_TCP_CSUM_IN_OK, M_TCP_CSUM_OUT, M_TCP_TSO, M_TIMESTAMP, M_UDP_CSUM_IN_OK,
    M_UDP_CSUM_OUT, Mbuf, MbufBox, MbufList, m_resethdr, ml_dequeue, ml_empty, ml_enqueue, ml_len,
    ml_purge,
};
use crate::net::pf_hooks::pf;
use crate::net::proto::SockaddrView;
use crate::net::rtable::{self, rtable_l2, rtable_loindex};
use crate::net::socket::Socket;
use crate::net::{route, rtsock};
use crate::prf::{Str, printf};
use crate::proc::{ProcGlue, Thread, suser};
use crate::task::TaskChain;
use crate::task::{Task, Taskq, TaskqRef, task_add, task_del, taskq_create, taskq_destroy_leaked};
use hal::pinlist::PinList;

/// `NET_TASKQ`: the softnet task queues (`MULTIPROCESSOR`'s, the only kernel built).
pub const NET_TASKQ: usize = 8;

/// `M_IFADDR`: the `malloc(9)` type of the index map's arrays (`<sys/malloc.h>`, `bsd`'s; a
/// copy).
pub const M_IFADDR: i32 = 9;

/// `M_DEVBUF`: the `malloc(9)` type of `if_attach_queues`' queues and of the loopback's
/// interface (a copy).
pub(crate) const M_DEVBUF: i32 = 2;

/// `M_IFGROUP`: the `malloc(9)` type of the interface groups (a copy).
const M_IFGROUP: i32 = 10;

/// `ETHER_ADDR_LEN` (`<netinet/if_ether.h>`, `bsd`'s; a copy).
pub const ETHER_ADDR_LEN: usize = 6;

/// `RTLABEL_LEN` (`<net/route.h>`, `bsd`'s): a route label's bytes, its NUL included.
const RTLABEL_LEN: usize = 32;

/// `SIOCDIFADDR_IN6` (`<netinet6/in6_var.h>`, `bsd`'s): `_IOW('i', 25, struct in6_ifreq)`,
/// a 288-byte request (`ifioctl`'s superuser check).
pub const SIOCDIFADDR_IN6: u64 = _ioc(IOC_IN, b'i', 25, 288);

/// `SIOCAIFADDR_IN6` (`<netinet6/in6_var.h>`, `bsd`'s): `_IOW('i', 26, struct
/// in6_aliasreq)`, a 128-byte request.
pub const SIOCAIFADDR_IN6: u64 = _ioc(IOC_IN, b'i', 26, 128);

/// The offset of `sa_data` in a `struct sockaddr` (after `sa_len` and `sa_family`).
const SA_DATA_OFFSET: usize = 2;

/// `USHRT_MAX`: the index map's bound on interfaces and on the serial.
const USHRT_MAX: u32 = 0xffff;

/// The bits of a byte (`NBBY`).
const NBBY: usize = 8;

/// The size of the C's `struct if_idxmap_dtor` on LP64 (a `struct smr_entry` of three
/// pointers and the map's pointer), which the used-index bitmap is at least as long as (the
/// C reuses the old bitmap as the destructor): kept so the bitmap's `malloc(M_IFADDR)` is the
/// C's size.
const IF_IDXMAP_DTOR_SIZE: usize = 32;

/// What the kernel's interface layer asks of the internet protocols (netinet and netinet6,
/// `bsd`'s until N6.4-N6.6): a bridge (bridges.toml's `Inet` row, paid by N6). Before
/// registration every leg does nothing: the C without `INET`/`INET6`.
pub trait Inet: Sync + 'static {
    /// `arpintr()`: the ARP queue (`NETISR_ARP`).
    fn arpintr(&self);
    /// `ipintr()`: the IPv4 queue (`NETISR_IP`).
    fn ipintr(&self);
    /// `ip6intr()`: the IPv6 queue (`NETISR_IPV6`).
    fn ip6intr(&self);
    /// `tcp_input_mlist(ml, af)`: the TCP segments a softnet gathered.
    fn tcp_input_mlist(&self, ml: &mut MbufList, af: SaFamily);
    /// `ipv4_input`, as `if_input_local` hands a packet to it (`if_input_proto`).
    fn ipv4_input(&self) -> ProtoFn;
    /// `ipv6_input`, the same for IPv6.
    fn ipv6_input(&self) -> ProtoFn;
    /// `tcpstat_inc(tcps_inhwlro)` (`hw`) or `tcpstat_inc(tcps_inbadlro)`.
    fn tcpstat_lro(&self, hw: bool);
    /// `nd6_ifattach(ifp)`.
    fn nd6_ifattach(&self, ifp: &'static Ifnet);
    /// `nd6_ifdetach(ifp)`.
    fn nd6_ifdetach(&self, ifp: &'static Ifnet);
    /// `rti_delete(ifp)`: igmp's router info.
    fn rti_delete(&self, ifp: &'static Ifnet);
    /// `if_detach`'s `NETHER > 0 && NFSCLIENT` block: the reverse ARP interface forgotten
    /// (`revarp_ifidx = 0`) when it is `ifp`.
    fn revarp_ifdetach(&self, ifp: &'static Ifnet);
    /// `in_ifdetach(ifp)`: the interface's IPv4 addresses and memberships gone.
    fn in_ifdetach(&self, ifp: &'static Ifnet);
    /// `in6_ifdetach(ifp)`.
    fn in6_ifdetach(&self, ifp: &'static Ifnet);
    /// `in6_ifattach(ifp)`: IPv6 on the interface; `ifioctl`'s `SIOCIFAFATTACH` and
    /// `SIOCSIFXFLAGS` answer its error, `if_up` and `ifnewlladdr` ignore it (the C's).
    fn in6_ifattach(&self, ifp: &'static Ifnet) -> Result<(), Errno>;
    /// `in_ioctl(cmd, data, ifp, privileged)`: an IPv4 address request of the kernel itself,
    /// which has no socket for `pru_control` (`ifioctl` with no socket: the boot self-test's
    /// `SIOCAIFADDR`); `data` the request's image, `IOCPARM_LEN(cmd)` bytes.
    fn in_ioctl(
        &self,
        cmd: u64,
        data: &mut [u8],
        ifp: &'static Ifnet,
        privileged: bool,
    ) -> Result<(), Errno>;
    /// `ip6_forwarding != 0`.
    fn ip6_forwarding(&self) -> bool;
    /// `ifnewlladdr`'s first IPv6 step: the link-local address purged
    /// (`in6ifa_ifpforlinklocal`, `in6_purgeaddr`); whether there was one.
    fn in6_purge_linklocal(&self, ifp: &'static Ifnet) -> bool;
    /// `free(ifa, M_IFADDR, 0)` of an interface address whose last reference `ifafree` gave
    /// back (route.c's; the protocols own the storage until N6.4 and N6.5). Default: nothing
    /// (N6.3b: `bsd`'s implementation comes with the routing switch, N6.3b2).
    fn ifa_free(&self, _ifa: &'static Ifaddr) {}
    /// `atomic_load_int(&ip6_multipath)` (`net.inet6.ip6.multipath`). Default: off.
    fn ip6_multipath(&self) -> bool {
        false
    }
    /// `ip6stat_inc(ip6s_rtcachehit)` (`hit`) or `ip6stat_inc(ip6s_rtcachemiss)`. Default:
    /// nothing.
    fn ip6stat_rtcache(&self, _hit: bool) {}
    /// `arpinput`, as `ether_input` hands an ARP frame to it (`if_input_proto`; N6.3c2).
    /// Default: a function that frees the frame (no `INET`).
    fn arpinput(&self) -> ProtoFn {
        proto_drop
    }
    /// `revarpinput`, the same for a reverse ARP frame. Default: as `arpinput`'s.
    fn revarpinput(&self) -> ProtoFn {
        proto_drop
    }
    /// `arpresolve(ifp, rt, m, dst, desten)`: `desten` filled with the link address of the
    /// IPv4 destination `dst` (a `sockaddr_in`) and the packet back to be sent;
    /// `Err(EAGAIN)` when ARP kept the packet for a reply, another error once it is freed
    /// (`ether_resolve`'s call). Default: the packet freed, `EAFNOSUPPORT` (no `INET`).
    fn arpresolve(
        &self,
        _ifp: &'static Ifnet,
        _rt: Option<&crate::net::route::Rtentry>,
        m: MbufBox,
        _dst: SockaddrView<'_>,
        _desten: &mut [u8; ETHER_ADDR_LEN],
    ) -> Result<MbufBox, Errno> {
        drop(m);
        Err(Errno::EAFNOSUPPORT)
    }
    /// `nd6_resolve(ifp, rt, m, dst, desten)`: the same for an IPv6 destination (a
    /// `sockaddr_in6`). Default: as `arpresolve`'s.
    fn nd6_resolve(
        &self,
        _ifp: &'static Ifnet,
        _rt: Option<&crate::net::route::Rtentry>,
        m: MbufBox,
        _dst: SockaddrView<'_>,
        _desten: &mut [u8; ETHER_ADDR_LEN],
    ) -> Result<MbufBox, Errno> {
        drop(m);
        Err(Errno::EAFNOSUPPORT)
    }
    /// `arp_rtrequest(ifp, req, rt)`: `ether_rtrequest`'s IPv4 route change. Default: nothing.
    fn arp_rtrequest(&self, _ifp: &'static Ifnet, _req: u8, _rt: &crate::net::route::Rtentry) {}
    /// `nd6_rtrequest(ifp, req, rt)`: the IPv6 one. Default: nothing.
    fn nd6_rtrequest(&self, _ifp: &'static Ifnet, _req: u8, _rt: &crate::net::route::Rtentry) {}
    /// `in6_proto_cksum_out(m, ifp)`: the IPv6 transport checksum in software when `ifp`
    /// cannot (`ether_offload_ifcap`, `if_output_tso`; `m`'s data starts at the IPv6
    /// header). Default: nothing.
    fn in6_proto_cksum_out(&self, _m: &Mbuf, _ifp: &'static Ifnet) {}
    /// `tcp_if_output_tso(ifp, mp, dst, rt, ifcap, mtu)`: TCP's segmentation offload of
    /// `*mp` (by the hardware when `ifp` has `ifcap`, else chopped in software and sent),
    /// the packet consumed then; left in `*mp` when it did not ask for TSO, or with its TSO
    /// flag cleared when its segments exceed `mtu` (`if_output_tso`'s first step; N6.4a, until
    /// TCP moves at N6.6a). Default: the packet left, the C without TCP's offload.
    fn tcp_if_output_tso(
        &self,
        _ifp: &'static Ifnet,
        _mp: &mut Option<MbufBox>,
        _dst: SockaddrView<'_>,
        _rt: Option<&crate::net::route::Rtentry>,
        _ifcap: u32,
        _mtu: u32,
    ) -> Result<(), Errno> {
        Ok(())
    }
    /// `tcpstat_inc(tcps_outswcsum)` (`tcp`) or `udpstat_inc(udps_outswcsum)`: a transport
    /// checksum computed in software on output (`in_proto_cksum_out`; N6.4a, until TCP and
    /// UDP move at N6.6). Default: nothing.
    fn proto_outswcsum(&self, _tcp: bool) {}
    /// `icmp_mtudisc_clone(dst, rtableid, ipsec)`: a host route to `dst` in `rtableid` that
    /// can carry a path MTU, cloned when there is none (`ip_output` after pf moved the packet
    /// to another table; N6.4a, until ICMP moves at N6.4c2). Default: none.
    fn icmp_mtudisc_clone(
        &self,
        _dst: [u8; 4],
        _rtableid: u32,
        _ipsec: bool,
    ) -> Option<hal::pool::PoolArc<crate::net::route::Rtentry>> {
        None
    }
}

/// [`Inet`]'s default protocol input: the frame freed, as a kernel without the protocol
/// drops it.
fn proto_drop(_ifp: &'static Ifnet, m: MbufBox, _ns: Option<&mut Netstack>) {
    drop(m);
}

/// The internet protocols' legs, once registered.
static INET: OnceStatic<&'static dyn Inet> = OnceStatic::new();

/// Registers the internet protocols' legs, once (`bsd`'s boot, before the first interface).
pub fn register_inet(imp: &'static dyn Inet) -> Result<(), hal::bridges::AlreadyRegistered> {
    INET.set(imp)
        .map_err(|_| hal::bridges::AlreadyRegistered("Inet"))
}

/// The internet protocols' legs, `None` before registration.
#[inline]
pub(crate) fn inet() -> Option<&'static dyn Inet> {
    INET.get().copied()
}

/// `struct softnet`: a softnet task queue and its barrier.
#[repr(align(64))]
pub struct Softnet {
    /// `sn_name`.
    name: &'static [u8],
    /// `sn_taskq`: set by `softnet_init` (its handle given up for a `&'static`, the module's
    /// deviations).
    taskq: HalLink<Taskq>,
    /// `net_tq_barriers`' task on this softnet.
    bartask: Task,
}

impl Softnet {
    /// Softnet `name`, its queue not made yet.
    const fn new(name: &'static [u8]) -> Self {
        Self {
            name,
            taskq: HalLink::new(),
            bartask: Task::unset(),
        }
    }
}

/// The softnets.
static SOFTNETS: [Softnet; NET_TASKQ] = [
    Softnet::new(b"softnet0"),
    Softnet::new(b"softnet1"),
    Softnet::new(b"softnet2"),
    Softnet::new(b"softnet3"),
    Softnet::new(b"softnet4"),
    Softnet::new(b"softnet5"),
    Softnet::new(b"softnet6"),
    Softnet::new(b"softnet7"),
];

/// `softnet_count()`'s answer, cached at its first call (`nsoftnets`).
static NSOFTNETS: AtomicU32 = AtomicU32::new(0);

/// `net_tq_barriers`' state: the barriers still to run of the current call (the C's stack
/// `struct refcnt`), under its own mutex, the callers serialised by it.
static NET_TQ_BARRIER: Mutexed<u32> = Mutexed::new(IPL_NET, 0);

/// The lock `net_tq_barriers`' callers take in turn (each waits for its own barriers).
static NET_TQ_BARRIER_LOCK: RawRwlock = RawRwlock::new("netbarlk");

/// `ifnetlist`: all attached interfaces, changed with the net lock (and the kernel lock in
/// `ifioctl`'s paths) held, read with either.
pub static IFNETLIST: List<IfnetList> = List::new();

/// `if_tmplist_lock`: serialises the users of the interfaces' and the addresses' temporary
/// lists (`if_tmplist`, `ifa_tmplist`: `ifconf`, and `bsd`'s `sysctl_ifnames`).
pub static IF_TMPLIST_LOCK: RawRwlock = RawRwlock::new("iftmplk");

/// `ifinit`: the index map (8 is a nice power of 2 for malloc) and its pool.
pub fn ifinit() {
    let _ = IF_MAP_POOL.init(IPL_NET, 0, "ifmappl");
    if_idxmap_init(8);
}

/// `softnet_init`: the `NET_TASKQ` softnet task queues (`softnet<i>`, one thread each,
/// `IPL_NET`, MP-safe).
pub fn softnet_init() {
    for (i, sn) in SOFTNETS.iter().enumerate() {
        let Some(tq) = taskq_create(sn.name, 1, IPL_NET, crate::task::TASKQ_MPSAFE) else {
            kpanic(format_args!("unable to create network taskq {i}"));
        };
        sn.taskq.set(Some(TaskqRef::leak(tq)));
        sn.bartask.set_ref(net_tq_barrier_task, sn);
    }
}

/// `softnet_percpu`: one softnet per CPU (`softnet_count()`); the others destroyed once.
pub fn softnet_percpu() {
    let n = softnet_count() as usize;
    for sn in &SOFTNETS[n..] {
        if let Some(tq) = sn.taskq.get() {
            // Emptied first: nothing reaches it any more (`net_sn` stops at the count).
            sn.taskq.set(None);
            taskq_destroy_leaked(tq);
        }
    }
}

/// `softnet_count()`: the softnets in use, one per CPU up to `NET_TASKQ`.
pub fn softnet_count() -> u32 {
    let n = NSOFTNETS.load(Ordering::Relaxed);
    if n != 0 {
        return n;
    }
    let n = (hal::cpu::ncpus() as u32).clamp(1, NET_TASKQ as u32);
    NSOFTNETS.store(n, Ordering::Relaxed);
    n
}

/// `net_sn(ifindex)`: the softnet of an interface (or a flow) index.
pub fn net_sn(ifindex: u32) -> &'static Softnet {
    &SOFTNETS[(ifindex % softnet_count()) as usize]
}

/// `net_tq(ifindex)`: the softnet task queue of an interface (or flow) index; `None` before
/// `softnet_init`.
pub fn net_tq(ifindex: u32) -> Option<&'static Taskq> {
    net_sn(ifindex).taskq.get()
}

/// `net_tq_barrier_rele`: one softnet's barrier ran.
fn net_tq_barrier_task(_sn: &'static (dyn Any + Send + Sync)) {
    let mut g = NET_TQ_BARRIER.lock();
    *g -= 1;
    if *g == 0 {
        drop(g);
        hal::sleep::wakeup(&raw const NET_TQ_BARRIER);
    }
}

/// `net_tq_barriers(wmesg)`: waits until every softnet in use has run what was queued on it
/// before the call (a barrier task on each, the C's `refcnt_finalize`).
pub fn net_tq_barriers(wmesg: &'static str) {
    let _serial = NET_TQ_BARRIER_LOCK.write();
    let n = softnet_count() as usize;
    *NET_TQ_BARRIER.lock() = n as u32;
    for sn in &SOFTNETS[..n] {
        match sn.taskq.get() {
            Some(tq) => {
                let _ = task_add(tq, Pin::static_ref(&sn.bartask));
            }
            None => net_tq_barrier_task(sn),
        }
    }
    let mut g = NET_TQ_BARRIER.lock();
    while *g != 0 {
        let _ = g.sleep_nsec(&raw const NET_TQ_BARRIER, 0, wmesg, hal::sleep::INFSLP);
    }
}

/// `if_map`'s published object: the map's length and its slots, one counted reference per
/// interface in a slot (slot 0 never used, the C's "skip ifidx 0").
pub struct IfMap {
    /// The SMR link its retirement is queued by.
    smr: SmrLink,
    /// The slots: `limit` of them (`malloc(M_IFADDR)`, the C's array).
    slots: MArray<HalLink<Ifnet>>,
}

/// The pool of the index map's objects ("ifmappl", D-N6.3a-5).
static IF_MAP_POOL: BoxPool<IfMap> = BoxPool::new();

impl SmrLinked for IfMap {
    const POOL: &'static BoxPool<IfMap> = &IF_MAP_POOL;
    fn smr_link(&self) -> &SmrLink {
        &self.smr
    }
}

impl IfMap {
    /// A map of `limit` empty slots.
    fn new(limit: u32) -> Self {
        let Some(slots) = MArray::new(M_IFADDR, M_WAITOK, limit as usize, |_| HalLink::new())
        else {
            kpanic(format_args!("if_idxmap: out of memory"));
        };
        Self {
            smr: SmrLink::new(),
            slots,
        }
    }

    /// The map's length (`if_idxmap_limit`).
    fn limit(&self) -> u32 {
        self.slots.len() as u32
    }

    /// Slot `index`'s interface.
    fn slot(&self, index: u32) -> Option<&'static Ifnet> {
        self.slots.get(index as usize).and_then(HalLink::get)
    }
}

impl Drop for IfMap {
    /// `if_idxmap_free`: the old map's references put, after the grace period (the SMR
    /// thread, or the last holder).
    fn drop(&mut self) {
        for s in self.slots.iter() {
            if_put(s.get());
        }
    }
}

/// What `idxmaplk` keeps beside the published map.
struct IdxmapState {
    /// `serial`: the next index to try.
    serial: u32,
    /// `count`: the interfaces with an index.
    count: u32,
    /// `usedidx`: the used-index bitmap (`malloc(M_IFADDR)`, at least the C's
    /// destructor's size).
    usedidx: Option<MArray<u8>>,
}

/// `if_idxmap.lock` ("idxmaplk").
static IF_IDXMAP_LOCK: RawRwlock = RawRwlock::new("idxmaplk");

/// The index map's locked state.
static IF_IDXMAP: Guarded<IdxmapState> = Guarded::new_static(
    &IF_IDXMAP_LOCK,
    IdxmapState {
        serial: 0,
        count: 0,
        usedidx: None,
    },
);

/// `if_idxmap.map`: the published map.
static IF_MAP: SmrPtr<IfMap> = SmrPtr::new_static(&IF_IDXMAP_LOCK);

/// The bitmap's size for `limit` indices (`max(howmany(limit, NBBY), sizeof(dtor))`).
fn if_idxmap_usedidx_size(limit: u32) -> usize {
    (limit as usize).div_ceil(NBBY).max(IF_IDXMAP_DTOR_SIZE)
}

/// A zeroed bitmap for `limit` indices.
fn if_idxmap_newusedidx(limit: u32) -> MArray<u8> {
    match MArray::new(M_IFADDR, M_WAITOK, if_idxmap_usedidx_size(limit), |_| 0) {
        Some(b) => b,
        None => kpanic(format_args!("if_idxmap: out of memory")),
    }
}

/// `isset(a, i)`.
fn isset(a: &[u8], i: usize) -> bool {
    a[i / NBBY] & (1 << (i % NBBY)) != 0
}

/// `setbit(a, i)`.
fn setbit(a: &mut [u8], i: usize) {
    a[i / NBBY] |= 1 << (i % NBBY);
}

/// `clrbit(a, i)`.
fn clrbit(a: &mut [u8], i: usize) {
    a[i / NBBY] &= !(1 << (i % NBBY));
}

/// The bitmap, `panic(9)` before `if_idxmap_init`.
fn usedidx(st: &mut IdxmapState) -> &mut [u8] {
    match st.usedidx.as_deref_mut() {
        Some(b) => b,
        None => kpanic(format_args!("if_idxmap used before if_idxmap_init")),
    }
}

/// `if_idxmap_init(limit)`: an empty map of `limit` slots, index 0 blacklisted, the serial
/// at 1. Called again (a test's fresh memory), it starts over: the old map is retired.
pub fn if_idxmap_init(limit: u32) {
    let mut w = IF_IDXMAP_LOCK.write();
    let st = IF_IDXMAP.get_mut(&mut w);
    st.serial = 1; // skip ifidx 0
    st.count = 0;
    let mut b = if_idxmap_newusedidx(limit);
    setbit(&mut b, 0); // blacklist ifidx 0
    drop(st.usedidx.replace(b));

    let Some(map) = IF_MAP_POOL.get(hal::pool::PR_WAITOK, || IfMap::new(limit)) else {
        kpanic(format_args!("if_idxmap: out of memory"));
    };
    if let Some(old) = IF_MAP.store(&mut w, Some(map)) {
        old.defer();
    }
}

/// The next serial, wrapped to 16 bits.
fn next_serial(st: &mut IdxmapState) -> u32 {
    let serial = st.serial;
    st.serial = serial.wrapping_add(1);
    serial & USHRT_MAX
}

/// `if_idxmap_alloc(ifp)`: picks `ifp`'s index (the next free serial), growing the map
/// (twice as long, the old one retired) when the serial passes its length.
pub fn if_idxmap_alloc(ifp: &Ifnet) {
    let mut w = IF_IDXMAP_LOCK.write();

    let mut index = {
        let st = IF_IDXMAP.get_mut(&mut w);
        st.count += 1;
        if st.count >= USHRT_MAX {
            kpanic(format_args!("too many interfaces"));
        }
        next_serial(st)
    };
    let mut limit = match IF_MAP.load_locked(&w) {
        Some(m) => m.limit(),
        None => kpanic(format_args!("if_idxmap used before if_idxmap_init")),
    };

    if index >= limit {
        let olimit = limit;
        limit = olimit * 2;
        let nmap = IfMap::new(limit);
        if let Some(omap) = IF_MAP.load_locked(&w) {
            for i in 1..olimit {
                if let Some(oifp) = omap.slot(i) {
                    // the new map isn't visible yet: no SMR_PTR_SET_LOCKED's membar needed.
                    nmap.slots[i as usize].set(Some(if_ref(oifp)));
                }
            }
        }

        let mut nusedidx = if_idxmap_newusedidx(limit);
        {
            let st = IF_IDXMAP.get_mut(&mut w);
            let o = usedidx(st);
            let n = (olimit as usize).div_ceil(NBBY);
            nusedidx[..n].copy_from_slice(&o[..n]);
            st.usedidx = Some(nusedidx);
        }

        let Some(nmap) = IF_MAP_POOL.get(hal::pool::PR_WAITOK, || nmap) else {
            kpanic(format_args!("if_idxmap: out of memory"));
        };
        if let Some(old) = IF_MAP.store(&mut w, Some(nmap)) {
            // smr_call(if_idxmap_free): the old map's references go after the grace period.
            old.defer();
        }
    }

    // pick the next free index
    let st = IF_IDXMAP.get_mut(&mut w);
    for _ in 0..USHRT_MAX {
        if index != 0 && !isset(usedidx(st), index as usize) {
            break;
        }
        index = next_serial(st);
    }
    kassert!(index != 0 && index < limit);
    kassert!(!isset(usedidx(st), index as usize));

    setbit(usedidx(st), index as usize);
    ifp.if_index.set(index);
}

/// `if_idxmap_insert(ifp)`: publishes `ifp` in its slot, a reference taken for the map.
pub fn if_idxmap_insert(ifp: &'static Ifnet) {
    let index = ifp.if_index.get();

    let w = IF_IDXMAP_LOCK.write();
    let Some(map) = IF_MAP.load_locked(&w) else {
        kpanic(format_args!("if_idxmap used before if_idxmap_init"));
    };
    let limit = map.limit();
    if index == 0 || index >= limit {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!(
                "{}({:p}) index {index} vs limit {limit}",
                Str(n),
                ifp
            ))
        });
    }
    let slot = &map.slots[index as usize];
    kassert!(slot.is_none());

    // commit
    slot.set(Some(if_ref(ifp)));
}

/// `if_idxmap_remove(ifp)`: `ifp`'s slot emptied and its index freed; once no reader can
/// still see it there (`smr_barrier`), the map's reference put.
pub fn if_idxmap_remove(ifp: &'static Ifnet) {
    if_idxmap_unlink(ifp);

    smr::barrier(false);
    if_put(Some(ifp));
}

/// `if_idxmap_remove`'s locked part.
fn if_idxmap_unlink(ifp: &'static Ifnet) {
    let index = ifp.if_index.get();

    let mut w = IF_IDXMAP_LOCK.write();
    {
        let Some(map) = IF_MAP.load_locked(&w) else {
            kpanic(format_args!("if_idxmap used before if_idxmap_init"));
        };
        kassert!(index != 0 && index < map.limit());
        let slot = &map.slots[index as usize];
        kassert!(slot.points_to(ifp));
        slot.set(None);
    }

    let st = IF_IDXMAP.get_mut(&mut w);
    kassert!(isset(usedidx(st), index as usize));
    st.count -= 1;
    clrbit(usedidx(st), index as usize);
    // end of if_idxmap modifications
}

/// The interface at `index` for the read section `g` (`if_idxmap`'s `SMR_PTR_GET`).
fn if_idxmap_get(index: u32, g: &SmrRead) -> Option<&'static Ifnet> {
    if index == 0 {
        return None;
    }
    IF_MAP.load(g)?.slot(index)
}

/// `if_get(index)`: the interface at `index`, a reference taken (the caller `if_put`s it,
/// the C's protocol; kernel code holds an [`IfRef`] instead).
pub fn if_get(index: u32) -> Option<&'static Ifnet> {
    let g = smr::read();
    let ifp = if_idxmap_get(index, &g)?;
    Some(if_ref(ifp))
}

/// `if_get_smr(index)`: the interface at `index` for a reader inside the SMR read section
/// `g`, no reference taken.
pub fn if_get_smr(index: u32, g: &SmrRead) -> Option<&'static Ifnet> {
    if_idxmap_get(index, g)
}

/// `if_ref(ifp)`: one more reference.
pub fn if_ref(ifp: &'static Ifnet) -> &'static Ifnet {
    ifp.rt().if_refcnt.take();
    ifp
}

/// `if_put(ifp)`: one reference less (`NULL` is nothing); the last one wakes `if_remove`'s
/// `refcnt_finalize`.
pub fn if_put(ifp: Option<&Ifnet>) {
    if let Some(ifp) = ifp {
        ifp.rt().if_refcnt.rele_wake();
    }
}

/// An interface with a reference held, put when dropped (`if_get` ... `if_put`).
pub struct IfRef(&'static Ifnet);

impl IfRef {
    /// `if_get(index)` as a guard.
    pub fn get(index: u32) -> Option<IfRef> {
        if_get(index).map(IfRef)
    }

    /// The interface, for as long as the reference is held or longer (`'static`).
    pub fn ifp(&self) -> &'static Ifnet {
        self.0
    }
}

impl core::ops::Deref for IfRef {
    type Target = Ifnet;
    fn deref(&self) -> &Ifnet {
        self.0
    }
}

impl Drop for IfRef {
    fn drop(&mut self) {
        if_put(Some(self.0));
    }
}

/// `ifp->if_qstart(ifq)`: the driver's start routine for the ring, or the detached stub
/// after `if_detach`'s swap (`if_detached_qstart`: the queue purged).
pub fn if_qstart(ifp: &'static Ifnet, ifq: &'static Ifqueue) {
    if ifp.is_detached(DETACHED_QSTART) {
        if_detached_qstart(ifq);
        return;
    }
    ifp.driver().qstart(ifq);
}

/// `if_qstart_compat`: a driver without `IFXF_MPSAFE`'s `if_start`, under the kernel lock at
/// `splnet`.
pub fn if_qstart_compat<D: NetDriver + ?Sized>(driver: &D, ifq: &'static Ifqueue) {
    let ifp = ifq.ifq_if();

    hal::klock::kernel_lock();
    let s = hal::intr::splraise(IPL_NET);
    driver.start(ifp);
    drop(s);
    hal::klock::kernel_unlock();
}

/// `if_start(ifp)`: the old start path of a driver without `IFXF_MPSAFE`.
pub fn if_start(ifp: &'static Ifnet) {
    kassert!(ifp.if_xflags.get() & IFXF_MPSAFE == 0);
    if_qstart(ifp, ifp.ifq(0));
}

/// `if_detached_qstart`: the detached interface's start, which drops what is queued.
pub fn if_detached_qstart(ifq: &Ifqueue) {
    let _ = crate::net::ifq::ifq_purge(ifq);
}

/// `if_enqueue_ifq`: the default `if_enqueue`, onto the send queue the conditioner picks (of
/// `if_nifqs`), then the ring started.
pub fn if_enqueue_ifq(ifp: &'static Ifnet, m: MbufBox) -> Result<(), Errno> {
    // The queues looked up once (review 1's S3: the C reads `if_snd`, `if_nifqs` and
    // `if_ifqs` from the interface it holds).
    let rt = ifp.rt();
    let mut ifq: &'static Ifqueue = &rt.if_snd;

    if let Some(more) = rt.ifqs.get() {
        // use the operations on the first ifq to pick which of the array gets this mbuf.
        let idx = ifq_idx(ifq, more.len() as u32 + 1, &m);
        if idx != 0 {
            ifq = match more.get(idx as usize - 1) {
                Some(q) => q,
                None => kpanic(format_args!("interface queue {idx} past the array")),
            };
        }
    }

    ifq_enqueue(ifq, m)?;

    ifq_start(ifq);

    Ok(())
}

/// The protocol input functions `if_input_proto` was given (the module's redesign): a
/// function's index, kept in the packet's `ph_cookie`, names it again in `if_input_process`.
static PROTO_FNS: [OnceStatic<ProtoFn>; 16] = [const { OnceStatic::new() }; 16];

/// A protocol's input function (`ipv4_input`, `ipv6_input`, ...), as `if_input_proto` calls
/// it: the interface, the packet, the softnet's stack (`None`: at once, the net lock held).
pub type ProtoFn = fn(&'static Ifnet, MbufBox, Option<&mut Netstack>);

/// The index of `f` in the table of protocol input functions, entered on first sight (1-based:
/// 0 is the C's empty cookie). `panic(9)` past the table's 16 functions.
pub fn proto_fn_id(f: ProtoFn) -> usize {
    for (i, slot) in PROTO_FNS.iter().enumerate() {
        match slot.get() {
            Some(g) if core::ptr::fn_addr_eq(*g, f) => return i + 1,
            Some(_) => {}
            None => match slot.set(f) {
                Ok(()) => return i + 1,
                Err(_) => {
                    if slot.get().is_some_and(|g| core::ptr::fn_addr_eq(*g, f)) {
                        return i + 1;
                    }
                }
            },
        }
    }
    kpanic(format_args!(
        "if_input_proto: too many protocol input functions"
    ))
}

/// The protocol input function of a cookie [`proto_fn_id`] made.
fn proto_fn(id: usize) -> Option<ProtoFn> {
    id.checked_sub(1)
        .and_then(|i| PROTO_FNS.get(i))
        .and_then(|s| s.get().copied())
}

/// `if_input_proto(ifp, m, input, ns)`: `m` for the protocol `input`, at once when there is
/// no softnet stack (the caller holds the net lock), otherwise queued on the stack's
/// `ns_proto` with the function in its cookie.
pub fn if_input_proto(ifp: &'static Ifnet, m: MbufBox, input: ProtoFn, ns: Option<&mut Netstack>) {
    let Some(ns) = ns else {
        input(ifp, m, None);
        return;
    };

    m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
    m.m_pkthdr().ph_cookie.set(proto_fn_id(input));
    ml_enqueue(&mut ns.ns_proto, m);
}

/// `if_input_process_proto`: the protocol input function in the packet's cookie, called.
fn if_input_process_proto(ifp: &'static Ifnet, m: MbufBox, ns: &mut Netstack) {
    match proto_fn(m.m_pkthdr().ph_cookie.get()) {
        Some(input) => input(ifp, m, Some(ns)),
        None => kpanic(format_args!("if_input_process_proto: no input function")),
    }
}

/// `if_input_process(ifp, ml, idx)`: the softnet `idx`'s work on a batch an input queue
/// passed (`ifiq_process`), under the shared net lock: each packet to the driver's
/// `if_input`, then what those queued on the stack (`if_vinput`'s packets again through
/// their interface's `if_input`, `if_input_proto`'s through their protocol, TCP's segments
/// through `tcp_input_mlist`) until none is left.
pub fn if_input_process(ifp: &'static Ifnet, ml: &mut MbufList, idx: u32) {
    if ml_empty(ml) {
        return;
    }

    if ifp.if_xflags.get() & IFXF_CLONED == 0 {
        let first = ml.first().map_or(0, |m| core::ptr::from_ref(m).addr());
        hal::random::enqueue_randomness(ml_len(ml) ^ first as u32);
    }

    // We grab the shared netlock for packet processing in the softnet threads. Packets can
    // regrab the exclusive lock via queues. ioctl, sysctl, and socket syscall may use shared
    // lock if access is read only or MP safe. Usually they hold the exclusive net lock.

    let mut ns = Netstack::new(idx);

    let _shared = NETLOCK.read();
    let driver = ifp.driver();
    while let Some(m) = ml_dequeue(ml) {
        driver.input(ifp, m, Some(&mut ns));
    }

    loop {
        while let Some(m) = ml_dequeue(&mut ns.ns_input) {
            let index = m.m_pkthdr().ph_ifidx.get();
            // The interface is borrowed past the read section, as the C does: `if_remove`
            // waits for the softnets (`net_tq_barriers`) before the interface goes.
            let ifp = if_get_smr(index, &smr::read());
            match ifp {
                Some(ifp) => ifp.driver().input(ifp, m, Some(&mut ns)),
                None => drop(m),
            }
        }

        while let Some(m) = ml_dequeue(&mut ns.ns_proto) {
            let index = m.m_pkthdr().ph_ifidx.get();
            let ifp = if_get_smr(index, &smr::read());
            match ifp {
                Some(ifp) => if_input_process_proto(ifp, m, &mut ns),
                None => drop(m),
            }
        }

        if let Some(inet) = inet() {
            inet.tcp_input_mlist(&mut ns.ns_tcp_ml, AF_INET);
            inet.tcp_input_mlist(&mut ns.ns_tcp6_ml, AF_INET6);
        }

        if ml_empty(&ns.ns_input) {
            break;
        }
    }
}

/// `if_attach_common(ifp)`: what an interface needs before it is listed: its driver (the one
/// registered in [`Ifnet::if_driver`]; none is `panic(9)`, the C's `KASSERT(if_ioctl !=
/// NULL)`), an index, the first send and input queues on their softnets, the reference count,
/// the hook lists, `if_txmit` and `if_llprio`'s defaults, all built into [`Ifnet::rt`] once.
/// The C's defaults for a NULL `if_enqueue`, `if_bpf_mtap` and `if_qstart` are the driver's
/// default methods. `panic(9)` for an interface attached twice (LZ's: the C would corrupt
/// `ifnetlist`).
pub fn if_attach_common(ifp: &'static Ifnet) {
    let Some(&driver) = ifp.if_driver.get() else {
        ifp.if_xname
            .with_str(|n| kpanic(format_args!("{}: no driver (if_ioctl)", Str(n))))
    };
    if ifp.if_index.get() != 0 || ifp.rt.is_set() {
        ifp.if_xname.with_str(|n| {
            kpanic(format_args!(
                "{}: attached twice (index {})",
                Str(n),
                ifp.if_index.get()
            ))
        });
    }

    if_idxmap_alloc(ifp);

    let rt = IfRt {
        if_refcnt: hal::refcnt::Refcnt::new(),
        if_snd: Ifqueue::new(ifp, 0, net_tq(0), ifp.if_snd_maxlen.get()),
        if_rcv: Ifiqueue::new(ifp, 0, net_tq(0)),
        ifqs: OnceStatic::new(),
        iqs: OnceStatic::new(),
        if_maddrlock: hal::rwlock::SplitRwlock::new("maddr"),
        if_slowtimo: crate::timeout::Timeout::unset(),
        if_watchdogtask: Task::unset(),
        if_linkstatetask: Task::unset(),
        hooks_mtx: Mutexed::new(hal::intr::IPL_NONE, ()),
        if_addrhooks: hal::pinlist::PinList::new(),
        if_linkstatehooks: hal::pinlist::PinList::new(),
        if_detachhooks: hal::pinlist::PinList::new(),
        driver,
    };
    if ifp.rt.set(rt).is_err() {
        kpanic(format_args!("if_attach_common: attached twice"));
    }
    let rt = ifp.rt();
    rt.if_refcnt.init();
    ifq_init_tasks(&rt.if_snd);
    ifiq_init_tasks(&rt.if_rcv);

    if ifp.if_txmit.get() == 0 {
        ifp.if_txmit.set(IF_TXMIT_DEFAULT);
    }
    ifp.if_llprio.set(IFQ_DEFPRIO as u8);
}

/// `if_attach_queues(ifp, nqs)`: `nqs` send queues, the first the interface's own, the others
/// made here with its length limit (after `if_attach`, once).
pub fn if_attach_queues(ifp: &'static Ifnet, nqs: u32) {
    kassert!(nqs != 0);
    let rt = ifp.rt();
    let maxlen = rt.if_snd.ifq_maxlen.get();

    rt.if_snd.ifq_softc.set(core::ptr::null());
    let Some(map) = MArray::new(M_DEVBUF, M_WAITOK, (nqs - 1) as usize, |i| {
        let i = i as u32 + 1;
        match MBox::new(M_DEVBUF, M_WAITOK, Ifqueue::new(ifp, i, net_tq(i), maxlen)) {
            Some(q) => q,
            None => kpanic(format_args!("if_attach_queues: out of memory")),
        }
    }) else {
        kpanic(format_args!("if_attach_queues: out of memory"));
    };
    if rt.ifqs.set(map).is_err() {
        kpanic(format_args!("if_attach_queues: called twice"));
    }
    if let Some(map) = rt.ifqs.get() {
        for q in map.iter() {
            ifq_init_tasks(q);
        }
    }
}

/// `if_attach_iqueues(ifp, niqs)`: `niqs` input queues, as [`if_attach_queues`].
pub fn if_attach_iqueues(ifp: &'static Ifnet, niqs: u32) {
    kassert!(niqs != 0);
    let rt = ifp.rt();

    rt.if_rcv.ifiq_softc.set(core::ptr::null());
    let Some(map) = MArray::new(M_DEVBUF, M_WAITOK, (niqs - 1) as usize, |i| {
        let i = i as u32 + 1;
        match MBox::new(M_DEVBUF, M_WAITOK, Ifiqueue::new(ifp, i, net_tq(i))) {
            Some(q) => q,
            None => kpanic(format_args!("if_attach_iqueues: out of memory")),
        }
    }) else {
        kpanic(format_args!("if_attach_iqueues: out of memory"));
    };
    if rt.iqs.set(map).is_err() {
        kpanic(format_args!("if_attach_iqueues: called twice"));
    }
    if let Some(map) = rt.iqs.get() {
        for q in map.iter() {
            ifiq_init_tasks(q);
        }
    }
}

/// `if_counters_alloc(ifp)`: the interface's stack counters turned on (at 0).
pub fn if_counters_alloc(ifp: &Ifnet) {
    kassert!(ifp.if_counters.get().is_none());
    ifp.if_counters.turn(true);
}

/// `if_counters_free(ifp)`: the counters turned off.
pub fn if_counters_free(ifp: &Ifnet) {
    kassert!(ifp.if_counters.get().is_some());
    ifp.if_counters.turn(false);
}

/// Forgets the index map of an earlier test (its arrays and pool item live in that test's
/// memory, so nothing may be freed into it) and starts a fresh one of 8 slots, the pool made
/// if it is not: `kernel::testutil`'s rule for a global holding kernel memory.
#[cfg(any(test, feature = "doubles"))]
pub fn if_idxmap_test_reset() {
    let _ = IF_MAP_POOL.init(IPL_NET, 0, "ifmappl");
    {
        let mut w = IF_IDXMAP_LOCK.write();
        core::mem::forget(IF_IDXMAP.get_mut(&mut w).usedidx.take());
        core::mem::forget(IF_MAP.take(&mut w));
    }
    if_idxmap_init(8);
}

/// The host tests' softnets: task queues no thread serves (`kernel::task`'s
/// `taskq_unthreaded`), made once and run by hand, in place of `softnet_init`'s.
#[cfg(any(test, feature = "doubles"))]
pub fn softnet_test_install() {
    for sn in &SOFTNETS {
        if sn.taskq.is_none() {
            let tq: &'static Taskq =
                std::boxed::Box::leak(std::boxed::Box::new(crate::task::taskq_unthreaded(sn.name)));
            crate::task::run_by_hand(tq);
            sn.taskq.set(Some(tq));
            sn.bartask.set_ref(net_tq_barrier_task, sn);
        }
    }
}

/// Forgets the interfaces and the groups of an earlier test (`ifnetlist` emptied, the groups'
/// array forgotten, not freed: it lives in that test's memory).
#[cfg(any(test, feature = "doubles"))]
pub fn ifnet_test_reset() {
    while IFNETLIST.pop_head().is_some() {}
    core::mem::forget(IFG_HEAD.replace(IfgSet::new()));
}

/// The interface layer of a fresh test's memory (the three resets above), for `bsd`'s tests.
#[cfg(any(test, feature = "doubles"))]
pub fn if_test_reset() {
    softnet_test_install();
    if_idxmap_test_reset();
    ifnet_test_reset();
}

/// A driver that counts what it is asked, for the host tests and the benches (`bsd`'s
/// zeroed `test_ifnet` with its `test_ioctl`, in the kernel's shape); its input sends a packet
/// round once through `ns_input` and then to a protocol that counts and frees it.
#[cfg(any(test, feature = "doubles"))]
pub struct TestDriver {
    /// `if_input` calls.
    pub inputs: AtomicU32,
    /// `if_qstart` calls.
    pub qstarts: AtomicU32,
    /// `if_ioctl` calls (`SIOCSIFFLAGS` and the others).
    pub ioctls: AtomicU32,
    /// The last request's `ifr_flags` (an `Ifreq`'s first 16 bits after its name).
    pub last_flags: AtomicU32,
    /// `if_ioctl` answers `EIO` while set.
    pub fail: core::sync::atomic::AtomicBool,
}

#[cfg(any(test, feature = "doubles"))]
impl TestDriver {
    /// A driver that has seen nothing.
    pub const fn new() -> Self {
        Self {
            inputs: AtomicU32::new(0),
            qstarts: AtomicU32::new(0),
            ioctls: AtomicU32::new(0),
            last_flags: AtomicU32::new(0),
            fail: core::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[cfg(any(test, feature = "doubles"))]
impl Default for TestDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "doubles"))]
impl NetDriver for TestDriver {
    fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
        self.ioctls.fetch_add(1, Ordering::Relaxed);
        if let Some(ifr) = hal::pod::at::<Ifreq>(data, 0) {
            self.last_flags
                .store(u32::from(ifr.ifr_flags() as u16), Ordering::Relaxed);
        }
        if self.fail.load(Ordering::Relaxed) {
            return Err(Errno::EIO);
        }
        Ok(())
    }

    fn qstart(&self, _ifq: &'static Ifqueue) {
        self.qstarts.fetch_add(1, Ordering::Relaxed);
    }

    fn input(&self, ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
        self.inputs.fetch_add(1, Ordering::Relaxed);
        let Some(ns) = ns else {
            drop(m);
            return;
        };
        if m.m_pkthdr().ph_loopcnt.get() == 0 {
            // if_vinput: once more through the interface's input.
            m.m_pkthdr().ph_loopcnt.set(1);
            m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
            ml_enqueue(&mut ns.ns_input, m);
        } else {
            if_input_proto(ifp, m, test_proto_count, Some(ns));
        }
    }
}

/// The packets [`TestDriver`]'s protocol has seen ([`test_proto_count`]).
#[cfg(any(test, feature = "doubles"))]
pub static TEST_PROTO_INPUTS: AtomicU32 = AtomicU32::new(0);

/// [`TestDriver`]'s protocol: counts the packet in [`TEST_PROTO_INPUTS`] and frees it; a
/// softnet's packet always comes with its netstack.
#[cfg(any(test, feature = "doubles"))]
pub fn test_proto_count(_ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
    let Some(_ns) = ns else {
        kpanic(format_args!("test_proto_count: no netstack"));
    };
    TEST_PROTO_INPUTS.fetch_add(1, Ordering::Relaxed);
    drop(m);
}

/// A fresh interface named `name` driven by `driver` (`IFXF_MPSAFE`), not attached, its
/// softc leaked as a driver's lives for ever: what [`if_attach`] takes.
#[cfg(any(test, feature = "doubles"))]
pub fn test_ifnet_fresh(name: &[u8], driver: &'static dyn NetDriver) -> &'static Ifnet {
    let ifp: &'static Ifnet = std::boxed::Box::leak(std::boxed::Box::new(Ifnet::new()));
    let mut n = [0u8; IFNAMSIZ];
    n[..name.len()].copy_from_slice(name);
    ifp.if_xname.set(n);
    if ifp.if_driver.set(driver).is_err() {
        kpanic(format_args!("test_ifnet_fresh: a driver already"));
    }
    ifp.if_xflags.set(IFXF_MPSAFE);
    ifp
}

/// A [`test_ifnet_fresh`] attached (`if_attach_common` and the index map's slot; not
/// listed).
#[cfg(any(test, feature = "doubles"))]
pub fn test_ifnet_with(name: &[u8], driver: &'static dyn NetDriver) -> &'static Ifnet {
    let ifp = test_ifnet_fresh(name, driver);
    if_attach_common(ifp);
    if_idxmap_insert(ifp);
    ifp
}

/// An interface named `name` with a [`TestDriver`], attached as [`test_ifnet_with`] does.
#[cfg(any(test, feature = "doubles"))]
pub fn test_ifnet(name: &[u8]) -> (&'static Ifnet, &'static TestDriver) {
    let d: &'static TestDriver = std::boxed::Box::leak(std::boxed::Box::new(TestDriver::new()));
    (test_ifnet_with(name, d), d)
}

/// A packet header mbuf of priority `prio` and length `len` (no data).
#[cfg(any(test, feature = "doubles"))]
pub fn test_packet(prio: u8, len: i32) -> MbufBox {
    let Some(m) = crate::net::mbuf::m_gethdr(crate::net::mbuf::M_DONTWAIT, hal::sys::mbuf::MT_DATA)
    else {
        kpanic(format_args!("test_packet: no mbuf"));
    };
    m.m_pkthdr().pf.prio.set(prio);
    m.m_pkthdr().len.set(len);
    m
}

/// `if_ioctl(ifp, cmd, data)`: the driver's `if_ioctl`, or the detached stub after
/// `if_detach`'s swap (`if_detached_ioctl`: `ENODEV`).
pub fn if_ioctl(ifp: &'static Ifnet, cmd: u64, data: &mut [u8]) -> Result<(), Errno> {
    if ifp.is_detached(DETACHED_IOCTL) {
        return if_detached_ioctl(ifp, cmd, data);
    }
    ifp.driver().ioctl(ifp, cmd, data)
}

/// `if_detached_ioctl`: a detached interface answers no request.
pub fn if_detached_ioctl(_ifp: &Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
    Err(Errno::ENODEV)
}

/// `if_attachsetup`'s timers: the watchdog timeout and the two tasks, whose argument is the
/// interface's index (the C's `(void *)ifidx`; `if_slowtimo`'s too here, D-N6.3a-7), then a
/// first `if_slowtimo`.
pub fn if_attach_timers(ifp: &'static Ifnet) {
    let rt = ifp.rt();
    let ifidx = core::ptr::without_provenance_mut(ifp.if_index.get() as usize);
    rt.if_slowtimo
        .set(Some(if_slowtimo), ifidx, crate::timeout::KCLOCK_NONE, 0);
    rt.if_watchdogtask.set(if_watchdog_task, ifidx);
    rt.if_linkstatetask.set(if_linkstate_task, ifidx);
    if_slowtimo(ifidx);
}

/// The interface index a timer or task of [`if_attach_timers`] was set with.
fn timer_ifidx(arg: *mut core::ffi::c_void) -> u32 {
    arg.addr() as u32
}

/// `if_slowtimo`: once a second, the driver's watchdog timer counted down (its task queued
/// when it reaches zero) and the timeout armed again, for a driver that has a watchdog. The
/// interface is found by its index (D-N6.3a-7: one SMR read and count a second where the C
/// follows its pointer).
fn if_slowtimo(arg: *mut core::ffi::c_void) {
    let Some(ifp) = IfRef::get(timer_ifidx(arg)) else {
        return;
    };
    let rt = ifp.rt();
    let s = hal::intr::splraise(IPL_NET);

    if rt.driver.has_watchdog() {
        let timer = ifp.if_timer.get();
        if timer > 0 {
            ifp.if_timer.set(timer - 1);
            if timer - 1 == 0
                && let Some(tq) = net_tq(ifp.if_index.get())
            {
                let _ = task_add(tq, Pin::static_ref(&ifp.ifp().rt().if_watchdogtask));
            }
        }
        let _ = Pin::static_ref(&ifp.ifp().rt().if_slowtimo).add_sec(IFNET_SLOWTIMO);
    }
    drop(s);
}

/// `if_watchdog_task`: the driver's watchdog, under the kernel lock at `splnet`.
fn if_watchdog_task(arg: *mut core::ffi::c_void) {
    let Some(ifp) = IfRef::get(timer_ifidx(arg)) else {
        return;
    };

    hal::klock::kernel_lock();
    let s = hal::intr::splraise(IPL_NET);
    let driver = ifp.rt().driver;
    if driver.has_watchdog() {
        driver.watchdog(ifp.ifp());
    }
    drop(s);
    hal::klock::kernel_unlock();
}

/// `if_linkstate_task`: `if_linkstate` under the net and kernel locks.
fn if_linkstate_task(arg: *mut core::ffi::c_void) {
    let net = NETLOCK.write();
    hal::klock::kernel_lock();

    if let Some(ifp) = IfRef::get(timer_ifidx(arg)) {
        if_linkstate(ifp.ifp(), &net_held(&net));
    }

    hal::klock::kernel_unlock();
}

/// `if_linkstate(ifp)`: the routing socket and the routes told of the link state (not after a
/// panic), then the link state hooks run, under the net lock `net`.
pub fn if_linkstate(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    if !hal::cpu::past_caring() {
        rtsock::rtm_ifchg(ifp);
        // The C ignores rt_if_track's result here.
        let _ = route::rt_if_track(ifp, net);
    }

    if_hooks_run(ifp, &ifp.rt().if_linkstatehooks);
}

/// `if_link_state_change(ifp)`: `if_linkstate` queued on the interface's softnet.
pub fn if_link_state_change(ifp: &'static Ifnet) {
    if let Some(tq) = net_tq(ifp.if_index.get()) {
        let _ = task_add(tq, Pin::static_ref(&ifp.rt().if_linkstatetask));
    }
}

/// `if_hooks_add(head, tail, t)`: `t` linked on one of the interface's hook lists, at its tail
/// or its head, under the hooks' mutex. `t` stays pinned until it is removed
/// ([`if_hooks_del`]).
fn if_hooks_add(ifp: &Ifnet, list: &PinList<TaskChain>, tail: bool, t: Pin<&Task>) {
    let mut g = ifp.rt().hooks_mtx.lock();
    if tail {
        list.insert_tail(&mut g, t);
    } else {
        list.insert_head(&mut g, t);
    }
}

/// `if_hooks_del(head, t)`: `t` taken off the list.
fn if_hooks_del(ifp: &Ifnet, list: &PinList<TaskChain>, t: &Task) {
    let mut g = ifp.rt().hooks_mtx.lock();
    list.remove(&mut g, t);
}

/// `if_hooks_run(head)`: every hook of the list run in order, the hooks' mutex released
/// around each, a cursor (a task without a function, which the walk skips) pinned on this
/// stack after the hook running so that a hook may add or remove hooks, itself included.
fn if_hooks_run(ifp: &Ifnet, list: &PinList<TaskChain>) {
    let rt = ifp.rt();
    let cursor = core::pin::pin!(Task::unset());
    let cursor = cursor.into_ref();

    let mut g = rt.hooks_mtx.lock();
    if list.is_empty(&g) {
        return;
    }
    list.insert_head(&mut g, cursor);
    // The hook after the cursor: `None` at the end; `Some(None)` for another walk's cursor.
    while let Some(work) = list.next(&g, &cursor).map(Task::work) {
        let _ = list.step_over(&mut g, &cursor);
        if let Some(w) = work {
            drop(g);
            w.run();
            g = rt.hooks_mtx.lock();
        }
    }
    list.remove(&mut g, &cursor);
}

/// `if_addrhook_add(ifp, t)`: an address change hook, at the tail.
pub fn if_addrhook_add(ifp: &Ifnet, t: Pin<&Task>) {
    if_hooks_add(ifp, &ifp.rt().if_addrhooks, true, t);
}

/// `if_addrhook_del(ifp, t)`.
pub fn if_addrhook_del(ifp: &Ifnet, t: &Task) {
    if_hooks_del(ifp, &ifp.rt().if_addrhooks, t);
}

/// `if_addrhooks_run(ifp)`.
pub fn if_addrhooks_run(ifp: &Ifnet) {
    if_hooks_run(ifp, &ifp.rt().if_addrhooks);
}

/// `if_linkstatehook_add(ifp, t)`: a link state hook, at the head.
pub fn if_linkstatehook_add(ifp: &Ifnet, t: Pin<&Task>) {
    if_hooks_add(ifp, &ifp.rt().if_linkstatehooks, false, t);
}

/// `if_linkstatehook_del(ifp, t)`.
pub fn if_linkstatehook_del(ifp: &Ifnet, t: &Task) {
    if_hooks_del(ifp, &ifp.rt().if_linkstatehooks, t);
}

/// `if_detachhook_add(ifp, t)`: a detach hook, at the head.
pub fn if_detachhook_add(ifp: &Ifnet, t: Pin<&Task>) {
    if_hooks_add(ifp, &ifp.rt().if_detachhooks, false, t);
}

/// `if_detachhook_del(ifp, t)`.
pub fn if_detachhook_del(ifp: &Ifnet, t: &Task) {
    if_hooks_del(ifp, &ifp.rt().if_detachhooks, t);
}

/// `if_deactivate(ifp)`: the detach hooks run under the net lock (pseudo-drivers undo their
/// changes).
pub fn if_deactivate(ifp: &Ifnet) {
    let _net = NETLOCK.write();
    if_hooks_run(ifp, &ifp.rt().if_detachhooks);
}

/// `if_input(ifp, ml)`: a driver's received packets, onto its first input queue.
pub fn if_input(ifp: &'static Ifnet, ml: &mut MbufList) {
    let _ = crate::net::ifq::ifiq_input(ifp.if_rcv(), ml);
}

/// `if_isconnected`'s and others' guard: an interface is up.
pub fn if_is_up(ifp: &Ifnet) -> bool {
    ifp.if_flags.get() & IFF_UP != 0
}

/// The bytes of a NUL-padded name (`if_xname`) up to the NUL.
pub fn ifname(b: &[u8; IFNAMSIZ]) -> &[u8] {
    let n = b.iter().position(|&c| c == 0).unwrap_or(IFNAMSIZ);
    &b[..n]
}

/// `NET_ASSERT_LOCKED()` (`DIAGNOSTIC`): the net lock held, shared or exclusive, by a witness
/// or a detached holding.
fn net_assert_locked() {
    kassert!(matches!(hal::netlock::status(), RW_WRITE | RW_READ));
}

/// `NET_ASSERT_LOCKED_EXCLUSIVE()` (`DIAGNOSTIC`): the running process writes the net lock.
fn net_assert_locked_exclusive() {
    kassert!(hal::netlock::write_held());
}

/// The bytes of a NUL-terminated name up to its NUL (the C's string), at most all of them.
fn cstr(s: &[u8]) -> &[u8] {
    let n = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    &s[..n]
}

/// `strcmp(a, b) == 0` of two NUL-terminated names.
fn name_eq(a: &[u8], b: &[u8]) -> bool {
    cstr(a) == cstr(b)
}

/// `strlcpy` of a name into an `IFNAMSIZ` array: at most `IFNAMSIZ - 1` bytes and a NUL.
fn name_array(name: &[u8]) -> [u8; IFNAMSIZ] {
    name_array_of::<IFNAMSIZ>(name)
}

/// `strlcpy` of a NUL-terminated string into a zeroed `N`-byte array: at most `N - 1` bytes,
/// then NULs.
fn name_array_of<const N: usize>(name: &[u8]) -> [u8; N] {
    let name = cstr(name);
    let mut n = [0u8; N];
    let len = name.len().min(N.saturating_sub(1));
    n[..len].copy_from_slice(&name[..len]);
    n
}

/// `ifp->if_counters` for a path the C knows has them (`if_counters_alloc` was called: the
/// loopback, wg, the pseudo-devices); `panic(9)` otherwise (the C follows a NULL pointer).
fn ifp_counters(ifp: &Ifnet) -> &IfCounterArray {
    match ifp.if_counters.get() {
        Some(c) => c,
        None => ifp
            .if_xname
            .with_str(|n| kpanic(format_args!("{}: no if_counters", Str(n)))),
    }
}

/// `if_attachsetup(ifp)`: what follows the list insert in `if_attach`, under the net lock:
/// the interface in group "all", nd6's and pf's state, its index published in the map, its
/// timers and tasks set (the first `if_slowtimo` after the map insert here, since the timer
/// finds the interface by index, D-N6.3a-7), and the routing socket told.
fn if_attachsetup(ifp: &'static Ifnet) {
    net_assert_locked();

    let _ = if_addgroup(ifp, IFG_ALL);

    if let Some(i) = inet() {
        i.nd6_ifattach(ifp);
    }

    if let Some(pf) = pf() {
        pf.attach_ifnet(ifp);
    }

    if_idxmap_insert(ifp);
    kassert!(if_get(0).is_none());

    if_attach_timers(ifp);

    // Announce the interface.
    rtsock::rtm_ifannounce(ifp, IFAN_ARRIVAL);
}

/// `if_alloc_sadl(ifp)`: the interface's link-level name, made after `if_addrlen` is set
/// (which `if_attach` may precede); an older one is freed first (an interface whose link type
/// changes). `panic(9)` when the name and the address do not fit a `sockaddr_dl`'s 24 bytes of
/// data (the C's longer allocation; no configured link type needs one: the module's
/// deviations).
pub fn if_alloc_sadl(ifp: &Ifnet) {
    // If the interface already has a link name, release it now. This is useful for
    // interfaces that can change link types, and thus switch link names often.
    if_free_sadl(ifp);

    let xname = ifp.if_xname.get();
    let name = cstr(&xname);
    let namelen = name.len();
    let masklen = SDL_DATA_OFFSET + namelen;
    let socksize = (masklen + usize::from(ifp.if_addrlen.get()))
        .max(size_of::<SockaddrDl>())
        .next_multiple_of(size_of::<u64>()); // ROUNDUP to sizeof(long)
    if socksize > size_of::<SockaddrDl>() {
        kpanic(format_args!(
            "if_alloc_sadl: {}'s link name of {socksize} bytes",
            Str(name)
        ));
    }

    let mut sdl: SockaddrDl = hal::user::zeroed();
    sdl.sdl_len = socksize as u8;
    sdl.sdl_family = AF_LINK;
    sdl.sdl_data[..namelen].copy_from_slice(name);
    sdl.sdl_nlen = namelen as u8;
    sdl.sdl_alen = ifp.if_addrlen.get();
    sdl.sdl_index = ifp.if_index.get() as u16;
    sdl.sdl_type = ifp.if_type.get();
    let Some(sdl) = MBox::new(M_IFADDR, M_WAITOK, sdl) else {
        kpanic(format_args!("if_alloc_sadl: out of memory"));
    };
    drop(ifp.if_sadl.replace(Some(sdl)));
}

/// `if_free_sadl(ifp)`: the link-level name freed (`if_detach`, or a link layer's detach).
pub fn if_free_sadl(ifp: &Ifnet) {
    drop(ifp.if_sadl.replace(None));
}

/// `if_attachhead(ifp)`: as [`if_attach`], at the head of `ifnetlist` (`lo0`).
pub fn if_attachhead(ifp: &'static Ifnet) {
    if_attach_common(ifp);
    let _net = NETLOCK.write();
    IFNETLIST.insert_head(ifp);
    if_attachsetup(ifp);
}

/// `if_attach(ifp)`: a network interface (a driver's `struct ifnet` with its name, its
/// `if_softc`, flags and driver) attached to the system.
pub fn if_attach(ifp: &'static Ifnet) {
    if_attach_common(ifp);
    let _net = NETLOCK.write();
    IFNETLIST.insert_tail(ifp);
    if_attachsetup(ifp);
}

/// `if_attach_ifq(ifp, newops, args)`: only the first send queue's conditioner is switched:
/// the code uses the first queue's (`if_snd`) to pick a queue for a packet, so switching it
/// changes the algorithm and routes every new packet to it (hfsc works on a single queue).
pub fn if_attach_ifq(ifp: &Ifnet, newops: &'static dyn QueueDiscipline, args: usize) {
    ifq_attach(ifp.if_snd(), newops, args);
}

/// `if_enqueue(ifp, m)`: a finished packet handed to the interface for transmission (pf's
/// delay and address change first, `NPF > 0`; `NBRIDGE` is not configured).
pub fn if_enqueue(ifp: &'static Ifnet, m: MbufBox) -> Result<(), Errno> {
    let ph = m.m_pkthdr();
    ph.csum_flags.set(ph.csum_flags.get() & !M_TIMESTAMP);

    if let Some(pf) = pf() {
        if m.m_pkthdr().pf.delay.get() > 0 {
            return pf.delay_pkt(m, ifp.if_index.get());
        }

        // NBRIDGE > 0: a bridge port (if_bridgeidx) hands the packet to bridge_enqueue
        // unless it has M_PROTO1; bridge(4) is not configured.

        pf.pkt_addr_changed(&m);
    }

    ifp.driver().enqueue(ifp, m)
}

/// `if_input_local(ifp, m, af, ns)`: a packet sent to a local address (the loopback's or a
/// `IFF_SIMPLEX` interface's copy) received again on `ifp`: tapped (the loopback's only),
/// its header reset but for the checksum flags it keeps, and handed to its protocol.
pub fn if_input_local(
    ifp: &'static Ifnet,
    mut m: MbufBox,
    af: SaFamily,
    ns: Option<&mut Netstack>,
) -> Result<(), Errno> {
    // Only send packets to bpf if they are destined to local addresses.
    //
    // if_input_local() is also called for SIMPLEX interfaces to duplicate packets for local
    // use. But don't dup them to bpf.
    if ifp.if_flags.get() & IFF_LOOPBACK != 0 && ifp.if_bpf.is_set() {
        let _ = crate::net::bpf::bpf_mtap_af(&ifp.if_bpf, u32::from(af), &m, BPF_DIRECTION_OUT);
    }

    let keepflags = m.flags() & (M_BCAST | M_MCAST);
    // Preserve outgoing checksum flags, in case the packet is forwarded to another interface.
    // Then the checksum, which is now incorrect, will be calculated before sending.
    let (keepcksum, keepmss, keepflowid) = {
        let ph = m.m_pkthdr();
        (
            ph.csum_flags.get()
                & (M_IPV4_CSUM_OUT
                    | M_TCP_CSUM_OUT
                    | M_UDP_CSUM_OUT
                    | M_ICMP_CSUM_OUT
                    | M_TCP_TSO
                    | M_FLOWID),
            ph.ph_mss.get(),
            ph.ph_flowid.get(),
        )
    };
    m_resethdr(&mut m);
    m.m_flags().set(m.flags() | M_LOOP | keepflags);
    let ph = m.m_pkthdr();
    ph.csum_flags.set(keepcksum);
    ph.ph_mss.set(keepmss);
    ph.ph_flowid.set(keepflowid);
    ph.ph_ifidx.set(ifp.if_index.get());
    ph.ph_rtableid.set(ifp.if_rdomain.get());

    if keepcksum & M_TCP_TSO != 0 && ph.len.get() as u32 > ifp.if_mtu.get() {
        if ifp.if_mtu.get() > 0
            && ((af == AF_INET && ifp.if_capabilities.get() & IFCAP_TSOv4 != 0)
                || (af == AF_INET6 && ifp.if_capabilities.get() & IFCAP_TSOv6 != 0))
        {
            if let Some(i) = inet() {
                i.tcpstat_lro(true);
            }
        } else {
            if let Some(i) = inet() {
                i.tcpstat_lro(false);
            }
            drop(m);
            return Err(Errno::EPROTONOSUPPORT);
        }
    }

    if keepcksum & M_TCP_CSUM_OUT != 0 {
        ph.csum_flags.set(ph.csum_flags.get() | M_TCP_CSUM_IN_OK);
    }
    if keepcksum & M_UDP_CSUM_OUT != 0 {
        ph.csum_flags.set(ph.csum_flags.get() | M_UDP_CSUM_IN_OK);
    }
    if keepcksum & M_ICMP_CSUM_OUT != 0 {
        ph.csum_flags.set(ph.csum_flags.get() | M_ICMP_CSUM_IN_OK);
    }

    // do not count multicast loopback and simplex interfaces
    if ifp.if_flags.get() & IFF_LOOPBACK != 0 {
        counters_pkt(
            ifp_counters(ifp),
            IfCounters::IfcOpackets,
            IfCounters::IfcObytes,
            ph.len.get() as u64,
        );
    }

    match (af, inet()) {
        (AF_INET, Some(i)) => {
            if keepcksum & M_IPV4_CSUM_OUT != 0 {
                ph.csum_flags.set(ph.csum_flags.get() | M_IPV4_CSUM_IN_OK);
            }
            if_input_proto(ifp, m, i.ipv4_input(), ns);
            Ok(())
        }
        (AF_INET6, Some(i)) => {
            if_input_proto(ifp, m, i.ipv6_input(), ns);
            Ok(())
        }
        // MPLS: AF_MPLS goes to mpls_input; MPLS is not configured.
        _ => {
            ifp.if_xname.with_str(|n| {
                printf(format_args!("{}: can't handle af{}\n", Str(n), af));
            });
            drop(m);
            Err(Errno::EAFNOSUPPORT)
        }
    }
}

/// `if_output_ml(ifp, ml, dst, rt)`: every packet of `ml` given to the interface's output
/// (`if_output`) toward `dst` through route `rt`; the first error stops it, the rest of the
/// list freed (N6.3b1: the kernel's routes; `if_output_tso` and `if_output_mq` stay `bsd`'s
/// with their netinet and ARP callers, docs/SCAFFOLDING.md).
pub fn if_output_ml(
    ifp: &'static Ifnet,
    ml: &mut MbufList,
    dst: crate::net::proto::SockaddrView<'_>,
    rt: Option<&crate::net::route::Rtentry>,
) -> Result<(), Errno> {
    let mut error = Ok(());

    while let Some(m) = ml_dequeue(ml) {
        // An interface with no driver has no `if_output`: the C calls through NULL, as
        // `NetDriver::output`'s default does.
        let Some(d) = ifp.if_driver.get() else {
            drop(m);
            kpanic(format_args!("if_output_ml: an interface without if_output"));
        };
        error = d.output(ifp, m, dst, rt);
        if error.is_err() {
            break;
        }
    }
    if error.is_err() {
        let _ = ml_purge(ml);
    }

    error
}

/// `if_output_tso(ifp, mp, dst, rt, mtu)`: the packet `*mp` sent toward `dst` by TCP's
/// segmentation offload when it asks for it (`Inet::tcp_if_output_tso`), else whole when it
/// fits `mtu` (its checksums finished for the interface first), the packet consumed then;
/// `*mp` is left set when the packet still has to be fragmented or dropped (N6.4a: the
/// kernel's, its TCP half `bsd`'s until N6.6a).
pub fn if_output_tso(
    ifp: &'static Ifnet,
    mp: &mut Option<MbufBox>,
    dst: SockaddrView<'_>,
    rt: Option<&crate::net::route::Rtentry>,
    mtu: u32,
) -> Result<(), Errno> {
    use hal::sys::net_if::{IFCAP_TSOv4, IFCAP_TSOv6};

    let af = dst.family();
    let ifcap = match af {
        AF_INET => IFCAP_TSOv4,
        AF_INET6 => IFCAP_TSOv6,
        _ => unhandled_af(i32::from(af)),
    };

    // Try to send with TSO first. When forwarding LRO may set maximum segment size in mbuf
    // header. Chop TCP segment even if it would fit interface MTU to preserve maximum path
    // MTU.
    if let Some(i) = inet() {
        i.tcp_if_output_tso(ifp, mp, dst, rt, ifcap, mtu)?;
    }
    let Some(mut m) = mp.take() else {
        return Ok(());
    };

    // The C compares the `int` length with the `u_int` MTU, as unsigned.
    if m.m_pkthdr().len.get() as u32 <= mtu {
        if af == AF_INET {
            crate::netinet::ip_output::in_hdr_cksum_out(&mut m, Some(ifp));
            crate::netinet::ip_output::in_proto_cksum_out(&mut m, Some(ifp));
        } else if let Some(i) = inet() {
            i.in6_proto_cksum_out(&m, ifp);
        }
        // An interface with no driver has no `if_output`: the C calls through NULL.
        let Some(d) = ifp.if_driver.get() else {
            drop(m);
            kpanic(format_args!(
                "if_output_tso: an interface without if_output"
            ));
        };
        return d.output(ifp, m, dst, rt);
    }

    // mp still contains mbuf that has to be fragmented or dropped.
    *mp = Some(m);
    Ok(())
}

/// `p2p_rtrequest(ifp, req, rt)`: the default action when a local route is installed on a
/// point-to-point interface: the route loses `RTF_LLINFO` when the address is the
/// interface's and the routing domain's loopback has an address of its family. Under the
/// net lock (`net`). The link layer of the point-to-point drivers (`bsd`'s gif, gre, tun, ...:
/// their `NetDriver::rtrequest`, N6.3b2).
pub fn p2p_rtrequest(ifp: &Ifnet, req: u8, rt: &crate::net::route::Rtentry, net: &NetHeld<'_>) {
    use hal::sys::net_route::{RTF_LLINFO, RTF_LOCAL, RTM_ADD};

    if req != RTM_ADD || !rt.rt_flags.isset(RTF_LOCAL) {
        return;
    }

    let key = rt.dest();
    let n = usize::from(key.first().copied().unwrap_or(0));
    let mut found = None;
    for ifa in &ifp.if_addrlist {
        let Some(a) = ifa.ifa_addr.read(net) else {
            continue;
        };
        if a.get(..n) == key.get(..n) {
            found = Some((ifa, a.family()));
            break;
        }
    }
    let Some((ifa, af)) = found else {
        return;
    };

    kassert!(rt.rt_ifa.points_to(ifa));

    let lo0ifp = if_get(crate::net::rtable::rtable_loindex(ifp.if_rdomain.get()));
    kassert!(lo0ifp.is_some());
    let Some(lo0ifp) = lo0ifp else {
        return;
    };
    let lo0ifa = (&lo0ifp.if_addrlist)
        .into_iter()
        .any(|l| l.ifa_addr.read(net).is_some_and(|a| a.family() == af));
    if_put(Some(lo0ifp));

    if !lo0ifa {
        return;
    }

    rt.rt_flags.clear(RTF_LLINFO);
}

/// `if_output_local(ifp, m, af)`: a packet for local delivery queued on one of `ifp`'s input
/// queues (by its flow), for the softnet task to give to `if_input_local`.
pub fn if_output_local(ifp: &'static Ifnet, m: MbufBox, af: SaFamily) -> Result<(), Errno> {
    let mut flow = 0;

    m.m_pkthdr().ph_family.set(af);

    if m.m_pkthdr().csum_flags.get() & M_FLOWID != 0 {
        flow = u32::from(m.m_pkthdr().ph_flowid.get());
    }

    let ifiq = ifp.ifiq(flow % ifp.if_niqs());

    ifiq_enqueue_qlim(ifiq, m, 8192).map_err(|_| Errno::ENOBUFS)
}

/// `if_vinput(ifp, m, ns)`: a packet a virtual interface received from its parent: queued on
/// `if_rcv` when there is no softnet stack, otherwise stamped, counted, tapped and given to
/// the interface's input again through `ns_input` (dropped by a monitor interface).
pub fn if_vinput(ifp: &'static Ifnet, m: MbufBox, ns: Option<&mut Netstack>) {
    if let Some(pf) = pf() {
        pf.pkt_addr_changed(&m);
    }

    let Some(ns) = ns else {
        let _ = ifiq_enqueue(ifp.if_rcv(), m);
        return;
    };

    m.m_pkthdr().ph_ifidx.set(ifp.if_index.get());
    m.m_pkthdr().ph_rtableid.set(ifp.if_rdomain.get());

    counters_pkt(
        ifp_counters(ifp),
        IfCounters::IfcIpackets,
        IfCounters::IfcIbytes,
        m.m_pkthdr().len.get() as u64,
    );

    if ifp.if_bpf.is_set() && ifp.driver().bpf_mtap(&ifp.if_bpf, &m, BPF_DIRECTION_IN) {
        drop(m);
        return;
    }

    if ifp.if_xflags.get() & IFXF_MONITOR == 0 {
        ml_enqueue(&mut ns.ns_input, m);
    } else {
        drop(m);
    }
}

/// `if_remove(ifp)`: the interface off `ifnetlist` (under the net lock) and off the index map,
/// the softnets past their work on it, then the wait for its last reference.
fn if_remove(ifp: &'static Ifnet) {
    // Remove the interface from the list of all interfaces.
    {
        let _net = NETLOCK.write();
        IFNETLIST.remove(ifp);
    }

    // Remove the interface from the interface index map.
    if_idxmap_remove(ifp);

    // Make sure softnet threads have finished with it
    net_tq_barriers("ifrmnet");

    // Sleep until the last reference is released.
    ifp.rt().if_refcnt.finalize("ifrm");
}

/// `if_detach(ifp)`: an interface detached from everything in the kernel and its resources
/// freed: hooks run, unlisted and unreferenced, its queues stopped and emptied, its
/// addresses, groups, link name, timers and protocol state gone, the routing socket told.
/// The softc holding `ifp` may be freed after the call (the module's redesign); the arrays
/// of the queues `if_attach_queues` added stay (the module's deviations).
pub fn if_detach(ifp: &'static Ifnet) {
    // Undo pseudo-driver changes.
    if_deactivate(ifp);

    // Other CPUs must not have a reference before we start destroying.
    if_remove(ifp);

    // ifp->if_qstart = if_detached_qstart;
    ifp.detached.fetch_or(DETACHED_QSTART, Ordering::AcqRel);

    // Wait until the start routines finished.
    for i in 0..ifp.if_nifqs() {
        let ifq = ifp.ifq(i);
        ifq_barrier(ifq);
        ifq_clr_oactive(ifq);
    }

    crate::net::bpf::bpfdetach(ifp);

    {
        let _net = NETLOCK.write();
        let s = hal::intr::splraise(IPL_NET);
        // ifp->if_ioctl = if_detached_ioctl; ifp->if_watchdog = NULL: the timeout and the
        // watchdog task find the interface by its index, which if_remove freed, so neither
        // reaches the driver any more.
        ifp.detached.fetch_or(DETACHED_IOCTL, Ordering::AcqRel);

        // Remove the watchdog timeout & task
        let rt = ifp.rt();
        let _ = rt.if_slowtimo.del();
        if let Some(tq) = net_tq(ifp.if_index.get()) {
            let _ = task_del(tq, &rt.if_watchdogtask);

            // Remove the link state task
            let _ = task_del(tq, &rt.if_linkstatetask);
        }

        if let Some(i) = inet() {
            i.rti_delete(ifp);
            // NETHER > 0 && NFSCLIENT
            i.revarp_ifdetach(ifp);
            // MROUTING: vif_delete(ifp); not configured.
            i.in_ifdetach(ifp);
            i.in6_ifdetach(ifp);
        }
        if let Some(pf) = pf() {
            pf.detach_ifnet(ifp);
        }
        // splnet ends here (the C's ends with the block): removing a member allocates the
        // group's smaller array `M_WAITOK`, which needs IPL_NONE, and no interrupt handler
        // reads the groups, the link name or the address list (review 1's S2).
        drop(s);

        loop {
            // The name copied out of the snapshot first: if_delgroup replaces it.
            let first = ifp.groups().iter().next().map(|g| g.ifg_group);
            let Some(name) = first else {
                break;
            };
            if if_delgroup(ifp, &name).is_err() {
                break;
            }
        }

        if_free_sadl(ifp);

        // We should not have any address left at this point.
        if !ifp.if_addrlist.is_empty() {
            #[cfg(feature = "diagnostic")]
            ifp.if_xname.with_str(|n| {
                printf(format_args!("{}: address list non empty\n", Str(n)));
            });
            while let Some(ifa) = ifp.if_addrlist.first() {
                ifa_del(ifp, ifa);
                ifa.ifa_ifp.set(None);
                route::ifafree(ifa);
            }
        }
    }

    {
        let rt = ifp.rt();
        let g = rt.hooks_mtx.lock();
        kassert!(rt.if_addrhooks.is_empty(&g));
        kassert!(rt.if_linkstatehooks.is_empty(&g));
        kassert!(rt.if_detachhooks.is_empty(&g));
    }

    if let Some(i) = inet() {
        i.nd6_ifdetach(ifp);
    }

    // Announce that the interface is gone.
    rtsock::rtm_ifannounce(ifp, IFAN_DEPARTURE);

    if ifp.if_counters.get().is_some() {
        if_counters_free(ifp);
    }

    for i in 0..ifp.if_nifqs() {
        ifq_destroy(ifp.ifq(i));
    }

    for i in 0..ifp.if_niqs() {
        ifiq_destroy(ifp.ifiq(i));
    }
}

/// `if_isconnected(ifp0, ifidx)`: `ifp0` is the interface `ifidx` (`NBRIDGE`'s ports of one
/// bridge and `NCARP`'s carpdev are not configured).
pub fn if_isconnected(ifp0: &Ifnet, ifidx: u32) -> bool {
    let Some(ifp) = IfRef::get(ifidx) else {
        return false;
    };

    // NBRIDGE > 0: two ports of one bridge (if_bridgeidx) are connected; not configured.
    // NCARP > 0: a carp(4) interface and its carpdev are connected; not configured.

    ifp0.if_index.get() == ifp.if_index.get()
}

/// The cloners `if_clone_attach` registered (`if_cloners`), in registration order; the C
/// inserts each at the list's head, so its walks run from the last one registered.
static IF_CLONERS: [OnceStatic<&'static dyn Cloner>; 16] = [const { OnceStatic::new() }; 16];

/// `if_cloners_count`.
static IF_CLONERS_COUNT: AtomicI32 = AtomicI32::new(0);

/// `if_cloners_lock`: serialises clone creation and destruction.
static IF_CLONERS_LOCK: RawRwlock = RawRwlock::new("clonelk");

/// `LIST_FOREACH(ifc, &if_cloners, ifc_list)`: the cloners, the last registered first.
pub fn if_cloners() -> impl Iterator<Item = &'static dyn Cloner> {
    let n = IF_CLONERS_COUNT.load(Ordering::Acquire) as usize;
    IF_CLONERS[..n]
        .iter()
        .rev()
        .filter_map(|c| c.get().copied())
}

/// `if_cloners_count`: the cloners registered.
pub fn if_clone_count() -> i32 {
    IF_CLONERS_COUNT.load(Ordering::Acquire)
}

/// `if_clone_create(name, rdomain)`: a cloned interface (`ifconfig lo1 create`): its cloner's
/// `create`, then the interface put in its cloner's group and, for a domain other than 0,
/// moved there. `EEXIST` when it exists, `EINVAL` for a name no cloner makes.
pub fn if_clone_create(name: &[u8], rdomain: i32) -> Result<(), Errno> {
    let Some((ifc, unit)) = if_clone_lookup(name) else {
        return Err(Errno::EINVAL);
    };

    let _clone = IF_CLONERS_LOCK.write();

    if let Some(ifp) = if_unit(name) {
        if_put(Some(ifp));
        return Err(Errno::EEXIST);
    }

    ifc.create(unit)?;

    let Some(ifp) = if_unit(name) else {
        return Ok(());
    };

    // Detached: if_setrdomain calls the driver (SIOCSIFRDOMAIN), which may let the lock go.
    net_locked(|| {
        let _ = if_addgroup(ifp, ifc.name());
        if rdomain != 0 {
            let _ = if_setrdomain(ifp, rdomain);
        }
    });
    if_put(Some(ifp));

    Ok(())
}

/// `if_clone_destroy(name)`: a cloned interface destroyed (brought down first) by its
/// cloner; `EOPNOTSUPP` for a cloner that cannot, `ENXIO` for no such interface. Called with
/// the kernel lock.
pub fn if_clone_destroy(name: &[u8]) -> Result<(), Errno> {
    let Some((ifc, _)) = if_clone_lookup(name) else {
        return Err(Errno::EINVAL);
    };

    if !ifc.has_destroy() {
        return Err(Errno::EOPNOTSUPP);
    }

    hal::klock::kernel_assert_locked();
    let _clone = IF_CLONERS_LOCK.write();

    let Some(ifp) = IFNETLIST
        .iter()
        .find(|ifp| name_eq(&ifp.if_xname.get(), name))
    else {
        return Err(Errno::ENXIO);
    };

    {
        let w = NETLOCK.write();
        if ifp.if_flags.get() & IFF_UP != 0 {
            let s = hal::intr::splraise(IPL_NET);
            if_down(ifp, &net_held(&w));
            drop(s);
        }
    }

    ifc.destroy(ifp)
}

/// `if_clone_lookup(name)`: the cloner of a name and the unit it asks for (`lo12` -> lo's
/// cloner and 12); `None` without a unit, with a zero-padded one, or one past `INT_MAX`.
pub fn if_clone_lookup(name: &[u8]) -> Option<(&'static dyn Cloner, i32)> {
    let name = cstr(name);

    // separate interface name from unit
    let cp = name
        .iter()
        .take(IFNAMSIZ)
        .position(u8::is_ascii_digit)
        .unwrap_or(name.len().min(IFNAMSIZ));

    if cp == 0 || cp == IFNAMSIZ || cp == name.len() {
        return None; // No name or unit number
    }

    if cp < IFNAMSIZ - 1 && name[cp] == b'0' && cp + 1 < name.len() {
        return None; // unit number 0 padded
    }

    let ifc = if_cloners().find(|ifc| ifc.name() == &name[..cp])?;

    let mut unit: i32 = 0;
    for &c in name[cp..].iter().take(IFNAMSIZ - cp) {
        let d = i32::from(c) - i32::from(b'0');
        if !c.is_ascii_digit() || unit > (i32::MAX - d) / 10 {
            // Bogus unit number.
            return None;
        }
        unit = unit * 10 + d;
    }

    Some((ifc, unit))
}

/// `if_clone_attach(ifc)`: a cloner registered, at boot by `main()` while the pseudo-devices
/// attach (the list is immutable afterwards: `bsd`'s caller asserts it). `panic(9)` past the
/// table's 16 cloners.
pub fn if_clone_attach(ifc: &'static dyn Cloner) {
    let n = IF_CLONERS_COUNT.load(Ordering::Acquire) as usize;
    match IF_CLONERS.get(n).map(|slot| slot.set(ifc)) {
        Some(Ok(())) => {
            IF_CLONERS_COUNT.store(n as i32 + 1, Ordering::Release);
        }
        _ => kpanic(format_args!("if_clone_attach: too many cloners")),
    }
}

/// `if_clone_list(ifcr)`: provide list of interface cloners to userspace (`SIOCIFGCLONERS`):
/// their count in `ifcr_total` and, with a buffer, as many names as `ifcr_count` asks for,
/// `IFNAMSIZ` bytes each, NUL-padded (`EINVAL` for a negative count).
fn if_clone_list(ifcr: &mut IfClonereq) -> Result<(), Errno> {
    let total = if_clone_count();
    if ifcr.ifcr_buffer == 0 {
        // Just asking how many there are.
        ifcr.ifcr_total = total;
        return Ok(());
    }

    if ifcr.ifcr_count < 0 {
        return Err(Errno::EINVAL);
    }

    ifcr.ifcr_total = total;
    let mut count = total.min(ifcr.ifcr_count);
    let mut dst = ifcr.ifcr_buffer as usize;

    for ifc in if_cloners() {
        if count == 0 {
            break;
        }
        UserPtr::<u8>::new(dst).copyout_slice(&name_array(ifc.name()))?;
        count -= 1;
        dst += IFNAMSIZ;
    }

    Ok(())
}

/// `ifq_congestion`: the tick a queue was last congested.
pub static IFQ_CONGESTION: AtomicI32 = AtomicI32::new(0);

/// `if_congestion()`: set queue congestion marker.
pub fn if_congestion() {
    IFQ_CONGESTION.store(crate::clock::ticks(), Ordering::Relaxed);
}

/// `if_congested()`: a queue was congested in the last hundredth of a second.
pub fn if_congested() -> bool {
    let ticks = crate::clock::ticks();
    let diff = ticks.wrapping_sub(IFQ_CONGESTION.load(Ordering::Relaxed));
    if diff < 0 {
        IFQ_CONGESTION.store(ticks.wrapping_sub(HZ), Ordering::Relaxed);
        return false;
    }

    diff <= HZ / 100
}

/// `equal(a1, a2)`: the two socket addresses are the same `a1->sa_len` bytes (`false` when
/// either is shorter than that: the C would read past it).
fn sa_equal(a1: &[u8], a2: &[u8]) -> bool {
    let len = usize::from(a1.first().copied().unwrap_or(0));
    match (a1.get(..len), a2.get(..len)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// A socket address's family (`sa_family`, its second byte).
fn sa_family(sa: &[u8]) -> Option<SaFamily> {
    sa.get(1).copied()
}

/// `ifa_ifwithaddr(addr, rtableid)`: the address `addr` (a socket address's bytes) of an
/// interface of `rtableid`'s domain. Under the net lock, which `held` proves (the C's
/// `NET_ASSERT_LOCKED()`, once per walk): the walk over `ifnetlist` and the address lists,
/// the C's.
pub fn ifa_ifwithaddr(held: &NetHeld<'_>, addr: &[u8], rtableid: u32) -> Option<&'static Ifaddr> {
    let family = sa_family(addr)?;
    let rdomain = rtable_l2(rtableid);
    for ifp in &IFNETLIST {
        if ifp.if_rdomain.get() != rdomain {
            continue;
        }

        for ifa in &ifp.if_addrlist {
            let Some(ifa_addr) = ifa.ifa_addr.read(held) else {
                continue;
            };
            if ifa_addr.family() != family {
                continue;
            }

            if sa_equal(addr, &ifa_addr) {
                return Some(ifa);
            }
        }
    }
    None
}

/// `ifa_ifwithdstaddr(addr, rdomain)`: the point-to-point interface address whose
/// destination is `addr`, under the net lock `held` proves.
pub fn ifa_ifwithdstaddr(held: &NetHeld<'_>, addr: &[u8], rdomain: u32) -> Option<&'static Ifaddr> {
    let family = sa_family(addr)?;
    let rdomain = rtable_l2(rdomain);
    for ifp in &IFNETLIST {
        if ifp.if_rdomain.get() != rdomain {
            continue;
        }
        if ifp.if_flags.get() & IFF_POINTOPOINT != 0 {
            for ifa in &ifp.if_addrlist {
                // An address without a destination is skipped before any copy (review 2's
                // S-b: the C tests the pointer, not the bytes).
                if ifa.ifa_dstaddr.is_none() {
                    continue;
                }
                if ifa.ifa_addr.read(held).map(|a| a.family()) != Some(family) {
                    continue;
                }
                let Some(dstaddr) = ifa.ifa_dstaddr.read(held) else {
                    continue;
                };
                if sa_equal(addr, &dstaddr) {
                    return Some(ifa);
                }
            }
        }
    }
    None
}

/// `ifaof_ifpforaddr(addr, ifp)`: the address of `ifp` that best matches `addr`: equal (or
/// its destination equal, point-to-point), or in its subnet; else the first of the family.
/// Under the net lock `held` proves (the C's callers hold it).
pub fn ifaof_ifpforaddr(held: &NetHeld<'_>, addr: &[u8], ifp: &Ifnet) -> Option<&'static Ifaddr> {
    let mut ifa_maybe = None;
    let af = sa_family(addr)?;

    if af >= AF_MAX {
        return None;
    }
    for ifa in &ifp.if_addrlist {
        let Some(ifa_addr) = ifa.ifa_addr.read(held) else {
            continue;
        };
        if ifa_addr.family() != af {
            continue;
        }
        if ifa_maybe.is_none() {
            ifa_maybe = Some(ifa);
        }
        let netmask = match ifa.ifa_netmask.read(held) {
            Some(nm) if ifp.if_flags.get() & IFF_POINTOPOINT == 0 => nm,
            _ => {
                if sa_equal(addr, &ifa_addr)
                    || ifa
                        .ifa_dstaddr
                        .read(held)
                        .is_some_and(|d| sa_equal(addr, &d))
                {
                    return Some(ifa);
                }
                continue;
            }
        };
        // The bytes after sa_len and sa_family (sa_data) compared under the netmask, up to the
        // netmask's length.
        let byte = |sa: &[u8], i: usize| sa.get(i).copied().unwrap_or(0);
        let masklen = usize::from(byte(&netmask, 0));
        let matches = (SA_DATA_OFFSET..masklen)
            .all(|i| (byte(addr, i) ^ byte(&ifa_addr, i)) & byte(&netmask, i) == 0);
        if matches {
            return Some(ifa);
        }
    }
    ifa_maybe
}

/// `if_downall()`: every interface that is up marked down and told so (`SIOCSIFFLAGS`), at
/// shutdown. The net lock is the detached holding, as `ifioctl`'s: wg(4)'s `wg_down`,
/// pflow(4)'s `pflowioctl` and pfsync(4)'s `pfsync_down` let it go and take it back
/// (N6.3a2 review's S1). The next interface is read after the body, as the C's
/// `TAILQ_FOREACH` reads `tqe_next`: one detached while a handler let the lock go is not
/// visited (an iterator would have read it before).
pub fn if_downall() {
    net_locked(|| {
        let mut next = IFNETLIST.first();
        while let Some(ifp) = next {
            if ifp.if_flags.get() & IFF_UP != 0 {
                hal::netlock::lend_detached(|net| if_down(ifp, net));
                let _ = if_ioctl_flags(ifp, SIOCSIFFLAGS, ifp.if_flags.get());
            }
            next = List::<IfnetList>::next(ifp);
        }
    });
}

/// The driver's `if_ioctl(ifp, cmd, &ifr)` of a `struct ifreq` whose only member set is
/// `ifr_flags` (`if_downall`, `ifpromisc`, `ifsetlro`, `ifnewlladdr`).
fn if_ioctl_flags(ifp: &'static Ifnet, cmd: u64, flags: i32) -> Result<(), Errno> {
    let mut ifr = Ifreq::zeroed();
    ifr.set_ifr_flags(flags as i16);
    if_ioctl(
        ifp,
        cmd,
        hal::user::bytes_of_mut(core::slice::from_mut(&mut ifr)),
    )
}

/// `if_down(ifp)`: an interface marked down, its send queue emptied, the protocols told,
/// under the net lock `net`.
pub fn if_down(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    net_assert_locked();

    ifp.if_flags.set(ifp.if_flags.get() & !IFF_UP);
    let now = crate::tc::getmicrotime();
    ifp.if_lastchange.set(now.tv_sec, now.tv_usec);
    let _ = ifq_purge(ifp.if_snd());

    if_linkstate(ifp, net);
}

/// `if_up(ifp)`: an interface marked up and the protocols told, under the net lock `net`; the
/// domain's loopback gets its `::1` (userland expects the kernel to set it on the default
/// lo(4)).
pub fn if_up(ifp: &'static Ifnet, net: &NetHeld<'_>) {
    net_assert_locked();

    ifp.if_flags.set(ifp.if_flags.get() | IFF_UP);
    let now = crate::tc::getmicrotime();
    ifp.if_lastchange.set(now.tv_sec, now.tv_usec);

    // Userland expects the kernel to set ::1 on default lo(4).
    if ifp.if_index.get() == rtable_loindex(ifp.if_rdomain.get())
        && let Some(i) = inet()
    {
        let _ = i.in6_ifattach(ifp);
    }

    if_linkstate(ifp, net);
}

/// `if_unit(name)`: the interface named `name`, a reference taken (the caller `if_put`s it).
/// Called with the kernel lock.
pub fn if_unit(name: &[u8]) -> Option<&'static Ifnet> {
    hal::klock::kernel_assert_locked();

    let ifp = IFNETLIST
        .iter()
        .find(|ifp| name_eq(&ifp.if_xname.get(), name))?;
    Some(if_ref(ifp))
}

/// `if_setlladdr(ifp, lladdr)`: an Ethernet interface's address changed (its `arpcom` and
/// its link-level name); `EINVAL` before `if_alloc_sadl`.
pub fn if_setlladdr(ifp: &'static Ifnet, lladdr: &[u8; ETHER_ADDR_LEN]) -> Result<(), Errno> {
    if ifp.sadl().is_none() {
        return Err(Errno::EINVAL);
    }

    // memcpy(((struct arpcom *)ifp)->ac_enaddr, lladdr, ETHER_ADDR_LEN): an Ethernet
    // interface's (the C's callers are; another has no arpcom to write).
    if let Some(ac) = ifp.if_arpcom.get() {
        ac.ac_enaddr.set(*lladdr);
    }
    ifp.if_sadl.update(|sdl| {
        if let Some(sdl) = sdl {
            // memcpy(LLADDR(sdl), lladdr, ETHER_ADDR_LEN)
            let start = SDL_DATA_OFFSET + usize::from(sdl.sdl_nlen);
            let bytes = hal::user::bytes_of_mut(core::slice::from_mut(&mut **sdl));
            if let Some(a) = bytes.get_mut(start..start + ETHER_ADDR_LEN) {
                a.copy_from_slice(lladdr);
            }
        }
    });

    Ok(())
}

/// `memcpy(LLADDR(ifp->if_sadl), lladdr, ifp->if_addrlen)`: the link address written into the
/// link-level name `if_alloc_sadl` made (`ether_ifattach`), its `sdl_alen` bytes at most;
/// nothing before `if_alloc_sadl`.
pub fn if_sadl_set_lladdr(ifp: &Ifnet, lladdr: &[u8]) {
    ifp.if_sadl.update(|sdl| {
        if let Some(sdl) = sdl {
            let bytes = hal::user::bytes_of_mut(core::slice::from_mut(&mut **sdl));
            if let Some(a) = SockaddrDl::lladdr_mut(bytes) {
                let n = a.len().min(lladdr.len());
                a[..n].copy_from_slice(&lladdr[..n]);
            }
        }
    });
}

/// `if_createrdomain(rdomain, ifp)`: routing domain `rdomain` with its loopback
/// `lo<rdomain>` (`ifp` itself when that is the interface being moved); `EEXIST` for a
/// table that has routes.
pub fn if_createrdomain(rdomain: i32, ifp: &Ifnet) -> Result<(), Errno> {
    rtable::rtable_add(rdomain as u32)?;
    if !rtable::rtable_empty(rdomain as u32) {
        return Err(Errno::EEXIST);
    }

    // Create rdomain including its loopback if with unit == rdomain
    let loifname = lo_name(rdomain as u32);
    let error = if_clone_create(&loifname, 0);
    let Some(loifp) = if_unit(&loifname) else {
        return Err(Errno::ENXIO);
    };
    if let Err(e) = error
        && (!core::ptr::eq(ifp, loifp) || e != Errno::EEXIST)
    {
        if_put(Some(loifp));
        return Err(e);
    }

    rtable::rtable_l2set(rdomain as u32, rdomain as u32, loifp.if_index.get());
    loifp.if_rdomain.set(rdomain as u32);
    if_put(Some(loifp));

    Ok(())
}

/// `snprintf(loifname, sizeof(loifname), "lo%u", rdomain)` (`loop_clone_create`'s
/// `"lo%d"` of its unit, which is never negative).
pub(crate) fn lo_name(rdomain: u32) -> [u8; IFNAMSIZ] {
    let mut n = [0u8; IFNAMSIZ];
    n[..2].copy_from_slice(b"lo");
    let mut digits = [0u8; 10];
    let mut len = 0;
    let mut v = rdomain;
    loop {
        digits[len] = b'0' + (v % 10) as u8;
        len += 1;
        v /= 10;
        if v == 0 {
            break;
        }
    }
    for (i, d) in digits[..len].iter().rev().enumerate() {
        n[2 + i] = *d;
    }
    n
}

/// `if_setrdomain(ifp, rdomain)`: the interface moved to routing domain `rdomain` (taken
/// down and its addresses dropped first, brought up again after): `EPERM` for a domain's
/// loopback, `ESRCH` for no such table, `EINVAL` for one that is no domain.
pub fn if_setrdomain(ifp: &'static Ifnet, rdomain: i32) -> Result<(), Errno> {
    let mut up = false;

    if rdomain < 0 || rdomain as u32 > RT_TABLEID_MAX {
        return Err(Errno::EINVAL);
    }
    let rdomain = rdomain as u32;

    if rdomain != ifp.if_rdomain.get()
        && ifp.if_flags.get() & IFF_LOOPBACK != 0
        && ifp.if_index.get() == rtable_loindex(ifp.if_rdomain.get())
    {
        return Err(Errno::EPERM);
    }

    if !rtable::rtable_exists(rdomain) {
        return Err(Errno::ESRCH);
    }

    // make sure that the routing table is a real rdomain
    if rdomain != rtable_l2(rdomain) {
        return Err(Errno::EINVAL);
    }

    if rdomain != ifp.if_rdomain.get() {
        let s = hal::intr::splraise(IPL_NET);
        // We are tearing down the world. Take down the IF so:
        // 1. everything that cares gets a message
        // 2. the automagic IPv6 bits are recreated
        if ifp.if_flags.get() & IFF_UP != 0 {
            up = true;
            hal::netlock::lend_detached(|net| if_down(ifp, net));
        }
        if let Some(i) = inet() {
            i.rti_delete(ifp);
            // MROUTING: vif_delete(ifp); not configured.
            i.in_ifdetach(ifp);
            i.in6_ifdetach(ifp);
        }
        drop(s);
    }

    // Let devices like enc(4) or mpe(4) know about the change
    let mut ifr = Ifreq::zeroed();
    ifr.set_ifr_rdomainid(rdomain as i32);
    match if_ioctl(
        ifp,
        SIOCSIFRDOMAIN,
        hal::user::bytes_of_mut(core::slice::from_mut(&mut ifr)),
    ) {
        Err(Errno::ENOTTY) => {}
        error => return error,
    }

    // Add interface to the specified rdomain
    ifp.if_rdomain.set(rdomain);

    // If we took down the IF, bring it back
    if up {
        let s = hal::intr::splraise(IPL_NET);
        hal::netlock::lend_detached(|net| if_up(ifp, net));
        drop(s);
    }

    Ok(())
}

/// The bytes `ifioctl` works on for `cmd`: the structure the command names
/// (`IOCPARM_LEN(cmd)`, which `sys_ioctl` copied whole) and at least the `struct ifreq` every
/// interface request is first read as (its name, `if_unit`'s key).
fn ifioctl_need(cmd: u64) -> usize {
    (iocparm_len(cmd) as usize).max(size_of::<Ifreq>())
}

/// The `u64` words of a request copy `ifioctl` keeps on its stack: every interface request
/// but the long ones (`if_laddrreq`'s 536 bytes, `in6_ifreq`'s 288, `if_sffpage`'s 274).
const IFIOCTL_STACK_WORDS: usize = 8;

/// `f` on the request `data` in place when it is `need` bytes or more and aligned for any
/// request structure (`sys_ioctl`'s stack buffer and its `M_IOCTLOPS` copy of a long request
/// are), else on a zeroed copy aligned for them and at least `need` bytes long (on the stack
/// up to 64 bytes, else an `M_IOCTLOPS` block), whose first `data.len()` bytes go back to
/// `data` after the call: never on a shorter buffer (N6.2a review's B1; the C reads an
/// `ifreq` out of `sys_ioctl`'s 128-byte stack buffer whatever the command's length).
fn with_request<R>(
    data: &mut [u8],
    need: usize,
    f: impl FnOnce(&mut [u8]) -> Result<R, Errno>,
) -> Result<R, Errno> {
    if data.len() >= need && data.as_ptr().addr().is_multiple_of(align_of::<u64>()) {
        return f(data);
    }
    let len = need.max(data.len());
    let words = len.div_ceil(size_of::<u64>());
    let mut stack = [0u64; IFIOCTL_STACK_WORDS];
    let mut heap: MArray<u64>;
    let buf: &mut [u64] = if words <= IFIOCTL_STACK_WORDS {
        &mut stack[..words]
    } else {
        heap =
            MArray::new(crate::descrip::M_IOCTLOPS, M_WAITOK, words, |_| 0).ok_or(Errno::ENOMEM)?;
        &mut heap[..]
    };
    let bytes = &mut hal::user::bytes_of_mut(buf)[..len];
    let n = data.len();
    bytes[..n].copy_from_slice(data);
    let r = f(bytes);
    data.copy_from_slice(&bytes[..n]);
    r
}

/// The request's structure `T` at its start ([`with_request`] made it aligned and at least
/// [`ifioctl_need`] long, so this fails only for a command whose length is shorter than its
/// structure, which no `SIOC*` is: `EINVAL` then, never a short view).
fn req_view<T: hal::user::AbiPod>(req: &mut [u8]) -> Result<&mut T, Errno> {
    hal::pod::view_mut::<T>(req, 0).ok_or(Errno::EINVAL)
}

/// `ifr->ifr_name`: the interface name every per-interface request starts with.
fn req_name(req: &[u8]) -> [u8; IFNAMSIZ] {
    let mut name = [0u8; IFNAMSIZ];
    let n = req.len().min(IFNAMSIZ);
    name[..n].copy_from_slice(&req[..n]);
    name
}

/// The request as the driver's `if_ioctl` and the protocol's `pru_control` get it: the
/// command's `IOCPARM_LEN(cmd)` bytes.
fn req_arg(req: &mut [u8], cmd: u64) -> &mut [u8] {
    let n = (iocparm_len(cmd) as usize).min(req.len());
    &mut req[..n]
}

/// `NET_LOCK()` around `f`, as the detached holding `hal::netlock::lock` takes: the drivers
/// `f` reaches through `if_ioctl` may let it go and take it back (wg(4)'s `wg_up` and
/// `wg_down`, pflow(4)'s `pflowioctl`), which a witness would refuse (the module's redesign).
fn net_locked<R>(f: impl FnOnce() -> R) -> R {
    hal::netlock::lock();
    let r = f();
    hal::netlock::unlock();
    r
}

/// `ETHER_IS_MULTICAST(addr)` (`<netinet/if_ether.h>`, `bsd`'s): the group bit of the first
/// byte.
fn ether_is_multicast(addr: [u8; ETHER_ADDR_LEN]) -> bool {
    addr[0] & 0x01 != 0
}

/// `ifp->if_wol(ifp, on)`: wake on LAN through the driver; `ENOTSUP` for a driver without
/// one (the C calls it unchecked once `IFCAP_WOL` is set; LZ's answer).
fn if_wol(ifp: &'static Ifnet, on: bool) -> Result<(), Errno> {
    ifp.driver().wol(ifp, on).unwrap_or(Err(Errno::ENOTSUP))
}

/// The requests that need the superuser before they go to the protocol and the driver
/// (`NBRIDGE > 0` adds the `SIOCBRDG*` ones: bridge(4) is not configured).
fn ifioctl_privileged(cmd: u64) -> bool {
    matches!(
        cmd,
        SIOCSETKALIVE
            | SIOCDIFPHYADDR
            | SIOCSLIFPHYADDR
            | SIOCSLIFPHYRTABLE
            | SIOCSLIFPHYTTL
            | SIOCSLIFPHYDF
            | SIOCSLIFPHYECN
            | SIOCADDMULTI
            | SIOCDELMULTI
            | SIOCSVNETID
            | SIOCDVNETID
            | SIOCSVNETFLOWID
            | SIOCSTXHPRIO
            | SIOCSRXHPRIO
            | SIOCSIFPAIR
            | SIOCSIFPARENT
            | SIOCDIFPARENT
            | SIOCSETMPWCFG
            | SIOCSETLABEL
            | SIOCDELLABEL
            | SIOCSPWE3CTRLWORD
            | SIOCSPWE3FAT
            | SIOCSPWE3NEIGHBOR
            | SIOCDPWE3NEIGHBOR
    )
}

/// The address requests no protocol answered, which need the superuser before they go to
/// the driver (`INET6`'s two among them).
fn ifioctl_address(cmd: u64) -> bool {
    matches!(
        cmd,
        SIOCAIFADDR
            | SIOCDIFADDR
            | SIOCSIFADDR
            | SIOCSIFNETMASK
            | SIOCSIFDSTADDR
            | SIOCSIFBRDADDR
            | SIOCAIFADDR_IN6
            | SIOCDIFADDR_IN6
    )
}

/// `pru_control(so, cmd, data, ifp)` (`<sys/protosw.h>`): the socket's protocol's request on
/// the interface (`EOPNOTSUPP` for a protocol without one); a request of the kernel itself
/// (no socket) goes to `in_ioctl` as a privileged one (the module's deviations).
fn pru_control<G: ProcGlue>(
    so: Option<&Socket<G>>,
    cmd: u64,
    data: &mut [u8],
    ifp: &'static Ifnet,
) -> Result<(), Errno> {
    match so {
        Some(so) => so.usrreqs().control(so, cmd, data, Some(ifp)),
        None => match inet() {
            Some(i) => i.in_ioctl(cmd, data, ifp, true),
            None => Err(Errno::EOPNOTSUPP),
        },
    }
}

/// `ifioctl(so, cmd, data, p)`: an interface request (the `'i'` group) on socket `so`
/// (`None` for a request of the kernel itself, the boot self-test's), `data` the kernel's
/// copy of its argument (`sys_ioctl`'s), `p` the caller. Every check the C makes is made at
/// its point: `suser` before each request that changes something, the interface found by
/// name (`ENXIO`), and each request's own (`EINVAL`, `ENODEV`, `EAFNOSUPPORT`, ...). The
/// kernel lock around the requests on one interface, the net lock inside it where the C
/// takes it (if.c:2136-2597).
pub fn ifioctl<G: ProcGlue>(
    so: Option<&Socket<G>>,
    cmd: u64,
    data: &mut [u8],
    p: &Thread<G>,
) -> Result<(), Errno> {
    with_request(data, ifioctl_need(cmd), |req| ifioctl_req(so, cmd, req, p))
}

/// [`ifioctl`] on `req`, the request aligned for its structure and at least an `ifreq` long.
fn ifioctl_req<G: ProcGlue>(
    so: Option<&Socket<G>>,
    cmd: u64,
    req: &mut [u8],
    p: &Thread<G>,
) -> Result<(), Errno> {
    match cmd {
        SIOCIFCREATE | SIOCIFDESTROY => {
            suser(p)?;
            let name = req_name(req);
            hal::klock::kernel_lock();
            let error = if cmd == SIOCIFCREATE {
                if_clone_create(&name, 0)
            } else {
                if_clone_destroy(&name)
            };
            hal::klock::kernel_unlock();
            return error;
        }
        SIOCSIFGATTR => {
            suser(p)?;
            hal::klock::kernel_lock();
            let error = net_locked(|| if_setgroupattribs(req));
            hal::klock::kernel_unlock();
            return error;
        }
        SIOCGIFCONF | SIOCIFGCLONERS | SIOCGIFGMEMB | SIOCGIFGATTR | SIOCGIFGLIST
        | SIOCGIFFLAGS | SIOCGIFXFLAGS | SIOCGIFMETRIC | SIOCGIFMTU | SIOCGIFHARDMTU
        | SIOCGIFDATA | SIOCGIFDESCR | SIOCGIFRTLABEL | SIOCGIFPRIORITY | SIOCGIFRDOMAIN
        | SIOCGIFGROUP | SIOCGIFLLPRIO => return ifioctl_get(cmd, req),
        _ => {}
    }

    hal::klock::kernel_lock();

    let Some(ifp) = if_unit(&req_name(req)) else {
        hal::klock::kernel_unlock();
        return Err(Errno::ENXIO);
    };
    let oif_flags = ifp.if_flags.get();
    let oif_xflags = ifp.if_xflags.get();

    let error = ifioctl_ifp(so, cmd, req, p, ifp, (oif_flags, oif_xflags));

    let flags = ifp.if_flags.get();
    // if_up() and if_down() already sent an update, skip here
    if (oif_flags != flags || oif_xflags != ifp.if_xflags.get())
        && (oif_flags ^ flags) & IFF_UP == 0
    {
        rtsock::rtm_ifchg(ifp);
    }

    if (oif_flags ^ flags) & IFF_UP != 0 {
        let now = crate::tc::getmicrotime();
        ifp.if_lastchange.set(now.tv_sec, now.tv_usec);
    }

    hal::klock::kernel_unlock();

    if_put(Some(ifp));

    error
}

/// [`ifioctl`]'s requests on the interface `ifp` (`if_unit`'s, under the kernel lock), whose
/// flags and extra flags were `old` before.
fn ifioctl_ifp<G: ProcGlue>(
    so: Option<&Socket<G>>,
    cmd: u64,
    req: &mut [u8],
    p: &Thread<G>,
    ifp: &'static Ifnet,
    old: (i32, i32),
) -> Result<(), Errno> {
    match cmd {
        SIOCIFAFATTACH | SIOCIFAFDETACH => {
            suser(p)?;
            let af = req_view::<IfAfreq>(req)?.ifar_af;
            net_locked(|| match (af, inet()) {
                (AF_INET, i) => {
                    // attach is a noop for AF_INET
                    if cmd == SIOCIFAFDETACH
                        && let Some(i) = i
                    {
                        i.in_ifdetach(ifp);
                    }
                    Ok(())
                }
                (AF_INET6, Some(i)) => {
                    if cmd == SIOCIFAFATTACH {
                        i.in6_ifattach(ifp)
                    } else {
                        i.in6_ifdetach(ifp);
                        Ok(())
                    }
                }
                _ => Err(Errno::EAFNOSUPPORT),
            })
        }

        SIOCSIFXFLAGS => {
            suser(p)?;
            net_locked(|| if_setxflags(req, ifp, old))
        }

        SIOCSIFFLAGS => {
            suser(p)?;
            net_locked(|| if_setflags(SIOCSIFFLAGS, req, ifp, old))
        }

        SIOCSIFMETRIC => {
            suser(p)?;
            let metric = req_view::<Ifreq>(req)?.ifr_metric();
            net_locked(|| ifp.if_metric.set(metric as u32));
            Ok(())
        }

        SIOCSIFMTU => {
            suser(p)?;
            let error = net_locked(|| if_ioctl(ifp, cmd, req_arg(req, cmd)));
            if error.is_ok() {
                rtsock::rtm_ifchg(ifp);
            }
            error
        }

        SIOCSIFDESCR => {
            suser(p)?;
            let mut ifdescrbuf = [0u8; IFDESCRSIZE];
            let uaddr = req_view::<Ifreq>(req)?.ifr_data() as usize;
            UserPtr::<u8>::new(uaddr).copyinstr(&mut ifdescrbuf)?;
            // memset(if_description, 0) and strlcpy: the string and zeros after it.
            ifp.if_description
                .set(name_array_of::<IFDESCRSIZE>(&ifdescrbuf));
            Ok(())
        }

        SIOCSIFRTLABEL => {
            suser(p)?;
            let mut ifrtlabelbuf = [0u8; RTLABEL_LEN];
            let uaddr = req_view::<Ifreq>(req)?.ifr_data() as usize;
            UserPtr::<u8>::new(uaddr).copyinstr(&mut ifrtlabelbuf)?;
            route::rtlabel_unref(ifp.if_rtlabelid.get());
            ifp.if_rtlabelid.set(route::rtlabel_name2id(&ifrtlabelbuf));
            Ok(())
        }

        SIOCSIFPRIORITY => {
            suser(p)?;
            let metric = req_view::<Ifreq>(req)?.ifr_metric();
            if !(0..=15).contains(&metric) {
                return Err(Errno::EINVAL);
            }
            ifp.if_priority.set(metric as u8);
            Ok(())
        }

        SIOCSIFRDOMAIN => {
            suser(p)?;
            let rdomain = req_view::<Ifreq>(req)?.ifr_rdomainid();
            match if_createrdomain(rdomain, ifp) {
                Ok(()) | Err(Errno::EEXIST) => net_locked(|| if_setrdomain(ifp, rdomain)),
                error => error,
            }
        }

        SIOCAIFGROUP => {
            suser(p)?;
            net_locked(|| {
                let group = *req_view::<Ifgroupreq>(req)?.ifgr_group();
                if_addgroup(ifp, &group)?;
                match if_ioctl(ifp, cmd, req_arg(req, cmd)) {
                    Err(Errno::ENOTTY) => Ok(()),
                    error => error,
                }
            })
        }

        SIOCDIFGROUP => {
            suser(p)?;
            net_locked(|| {
                match if_ioctl(ifp, cmd, req_arg(req, cmd)) {
                    Ok(()) | Err(Errno::ENOTTY) => {}
                    error => return error,
                }
                // The group's name read after the driver's call, as the C reads it.
                let group = *req_view::<Ifgroupreq>(req)?.ifgr_group();
                if_delgroup(ifp, &group)
            })
        }

        SIOCSIFLLADDR => {
            suser(p)?;
            let sa = req_view::<Ifreq>(req)?.ifr_addr();
            let first = sa_lladdr(&sa);
            if ifp.sadl().is_none()
                || usize::from(sa.sa_len) != ETHER_ADDR_LEN
                || ether_is_multicast(first)
            {
                return Err(Errno::EINVAL);
            }
            let error = net_locked(|| {
                let error = match ifp.if_type.get() {
                    IFT_ETHER | IFT_CARP | IFT_XETHER | IFT_ISO88025 => {
                        match if_ioctl(ifp, cmd, req_arg(req, cmd)) {
                            Ok(()) | Err(Errno::ENOTTY) => {
                                // The address read after the driver's call, as the C does.
                                let sa = req_view::<Ifreq>(req)?.ifr_addr();
                                if_setlladdr(ifp, &sa_lladdr(&sa))
                            }
                            error => error,
                        }
                    }
                    _ => Err(Errno::ENODEV),
                };

                if error.is_ok() {
                    ifnewlladdr(ifp);
                }
                error
            });
            if error.is_ok() {
                rtsock::rtm_ifchg(ifp);
            }
            error
        }

        SIOCSIFLLPRIO => {
            suser(p)?;
            let llprio = req_view::<Ifreq>(req)?.ifr_llprio();
            if llprio < IFQ_MINPRIO as i32 || llprio > IFQ_MAXPRIO as i32 {
                return Err(Errno::EINVAL);
            }
            net_locked(|| ifp.if_llprio.set(llprio as u8));
            Ok(())
        }

        SIOCGIFSFFPAGE => {
            suser(p)?;
            if_sffpage_check(req)?;
            // don't take NET_LOCK because i2c reads take a long time
            if_ioctl(ifp, cmd, req_arg(req, cmd))
        }

        SIOCSIFMEDIA | SIOCGIFMEDIA => {
            if cmd == SIOCSIFMEDIA {
                suser(p)?;
            }
            // net lock is not needed
            if_ioctl(ifp, cmd, req_arg(req, cmd))
        }

        _ => {
            if ifioctl_privileged(cmd) {
                suser(p)?;
            }
            match pru_control(so, cmd, req_arg(req, cmd), ifp) {
                Err(Errno::EOPNOTSUPP) => {}
                error => return error,
            }
            if ifioctl_address(cmd) {
                suser(p)?;
            }
            net_locked(|| if_ioctl(ifp, cmd, req_arg(req, cmd)))
        }
    }
}

/// The first `ETHER_ADDR_LEN` bytes of a socket address's `sa_data` (`SIOCSIFLLADDR`'s
/// link-level address).
fn sa_lladdr(sa: &Sockaddr) -> [u8; ETHER_ADDR_LEN] {
    let mut a = [0u8; ETHER_ADDR_LEN];
    a.copy_from_slice(&sa.sa_data[..ETHER_ADDR_LEN]);
    a
}

/// `SIOCSIFXFLAGS` (under the net lock): IPv6's automatic configuration and its stable
/// addresses (`INET6`), wake on LAN, LRO, then the flags a user may change set; an interface
/// that turned automatic configuration on while down is brought up (`forceup`).
fn if_setxflags(req: &mut [u8], ifp: &'static Ifnet, old: (i32, i32)) -> Result<(), Errno> {
    let mut error = Ok(());
    let rflags = i32::from(req_view::<Ifreq>(req)?.ifr_flags());

    // INET6: the protocol's legs answer when it is configured (registered).
    if let Some(i) = inet() {
        let autoconf6 = IFXF_AUTOCONF6 | IFXF_AUTOCONF6TEMP;
        if rflags & autoconf6 != 0 && ifp.if_xflags.get() & autoconf6 == 0 {
            i.in6_ifattach(ifp)?;
        }

        if rflags & IFXF_INET6_NOSOII != 0 && ifp.if_xflags.get() & IFXF_INET6_NOSOII == 0 {
            ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_INET6_NOSOII);
        }

        if rflags & IFXF_INET6_NOSOII == 0 && ifp.if_xflags.get() & IFXF_INET6_NOSOII != 0 {
            ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_INET6_NOSOII);
        }
    }

    // MPLS: IFXF_MPLS swaps if_output with mpls_output; MPLS is not configured.

    // !SMALL_KERNEL
    if ifp.if_capabilities.get() & IFCAP_WOL != 0 {
        if rflags & IFXF_WOL != 0 && ifp.if_xflags.get() & IFXF_WOL == 0 {
            let s = hal::intr::splraise(IPL_NET);
            ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_WOL);
            error = if_wol(ifp, true);
            drop(s);
        }
        if ifp.if_xflags.get() & IFXF_WOL != 0 && rflags & IFXF_WOL == 0 {
            let s = hal::intr::splraise(IPL_NET);
            ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_WOL);
            error = if_wol(ifp, false);
            drop(s);
        }
    } else if rflags & IFXF_WOL != 0 {
        req_view::<Ifreq>(req)?.set_ifr_flags((rflags & !IFXF_WOL) as i16);
        error = Err(Errno::ENOTSUP);
    }

    // The request's flags again: the line above may have cleared IFXF_WOL in them.
    let rflags = i32::from(req_view::<Ifreq>(req)?.ifr_flags());
    if (rflags & IFXF_LRO != 0) != (ifp.if_xflags.get() & IFXF_LRO != 0) {
        error = ifsetlro(ifp, rflags & IFXF_LRO != 0);
    }

    if error.is_ok() {
        ifp.if_xflags
            .set((ifp.if_xflags.get() & IFXF_CANTCHANGE) | (rflags & !IFXF_CANTCHANGE));
    }

    let (_, oif_xflags) = old;
    let newly = |bit: i32| oif_xflags & bit == 0 && ifp.if_xflags.get() & bit != 0;
    if ifp.if_flags.get() & IFF_UP == 0
        && (newly(IFXF_AUTOCONF4) || newly(IFXF_AUTOCONF6) || newly(IFXF_AUTOCONF6TEMP))
    {
        req_view::<Ifreq>(req)?.set_ifr_flags((ifp.if_flags.get() | IFF_UP) as i16);
        return if_setflags(SIOCSIFXFLAGS, req, ifp, old);
    }

    error
}

/// `forceup` (under the net lock): `SIOCSIFFLAGS`, and `SIOCSIFXFLAGS` bringing an interface
/// with automatic configuration up. The flags a user may change are set from the request
/// and the driver told; when it refuses, the old flags (and, for `SIOCSIFXFLAGS`, the old
/// extra flags) come back, else `IFF_UP` changing brings the interface up or down.
fn if_setflags(
    cmd: u64,
    req: &mut [u8],
    ifp: &'static Ifnet,
    old: (i32, i32),
) -> Result<(), Errno> {
    let (oif_flags, oif_xflags) = old;
    // `if_flags` is an `unsigned short` in C, `ifr_flags` a `short`: the assignment keeps the
    // low sixteen bits of the sign-extended request.
    let rflags = i32::from(req_view::<Ifreq>(req)?.ifr_flags());
    ifp.if_flags.set(
        ((ifp.if_flags.get() & IFF_CANTCHANGE) | (rflags & !IFF_CANTCHANGE)) & USHRT_MAX as i32,
    );
    let error = if_ioctl(ifp, SIOCSIFFLAGS, req_arg(req, SIOCSIFFLAGS));
    if error.is_err() {
        ifp.if_flags.set(oif_flags);
        if cmd == SIOCSIFXFLAGS {
            ifp.if_xflags.set(oif_xflags);
        }
    } else if (oif_flags ^ ifp.if_flags.get()) & IFF_UP != 0 {
        let s = hal::intr::splraise(IPL_NET);
        hal::netlock::lend_detached(|net| {
            if ifp.if_flags.get() & IFF_UP != 0 {
                if_up(ifp, net);
            } else {
                if_down(ifp, net);
            }
        });
        drop(s);
    }
    error
}

/// `ifioctl_get(cmd, data)`: the requests that only read (no privilege); an interface's
/// under a reference, its `if_data` under the shared net lock and the kernel lock.
fn ifioctl_get(cmd: u64, req: &mut [u8]) -> Result<(), Errno> {
    match cmd {
        SIOCGIFCONF => return ifconf(req),
        SIOCIFGCLONERS => return if_clone_list(req_view::<IfClonereq>(req)?),
        SIOCGIFGMEMB => return if_getgroupmembers(req),
        SIOCGIFGATTR => {
            let _net = NETLOCK.read();
            return if_getgroupattribs(req);
        }
        SIOCGIFGLIST => return if_getgrouplist(req),
        _ => {}
    }

    hal::klock::kernel_lock();
    let ifp = if_unit(&req_name(req));
    hal::klock::kernel_unlock();

    let Some(ifp) = ifp.map(IfRef) else {
        return Err(Errno::ENXIO);
    };

    if cmd == SIOCGIFGROUP {
        return if_getgroup(req, &ifp);
    }

    let ifr = req_view::<Ifreq>(req)?;
    match cmd {
        SIOCGIFFLAGS => {
            let mut flags = ifp.if_flags.get();
            if crate::net::ifq::ifq_is_oactive(ifp.if_snd()) {
                flags |= IFF_OACTIVE;
            }
            ifr.set_ifr_flags(flags as i16);
        }

        SIOCGIFXFLAGS => {
            ifr.set_ifr_flags((ifp.if_xflags.get() & !(IFXF_MPSAFE | IFXF_CLONED)) as i16);
        }

        SIOCGIFMETRIC => ifr.set_ifr_metric(ifp.if_metric.get() as i32),

        SIOCGIFMTU => ifr.set_ifr_mtu(ifp.if_mtu.get() as i32),

        SIOCGIFHARDMTU => ifr.set_ifr_hardmtu(ifp.if_hardmtu.get() as i32),

        SIOCGIFDATA => {
            let mut ifdata: IfData = hal::user::zeroed();
            {
                let _net = NETLOCK.read();
                hal::klock::kernel_lock();
                if_getdata(&ifp, &mut ifdata);
                hal::klock::kernel_unlock();
            }
            return UserPtr::<IfData>::new(ifr.ifr_data() as usize).copyout(&ifdata);
        }

        SIOCGIFDESCR => {
            hal::klock::kernel_lock();
            let ifdescrbuf = name_array_of::<IFDESCRSIZE>(&ifp.if_description.get());
            hal::klock::kernel_unlock();

            return UserPtr::<u8>::new(ifr.ifr_data() as usize)
                .copyoutstr(&ifdescrbuf)
                .map(|_| ());
        }

        SIOCGIFRTLABEL => {
            let rtlabelid = ifp.if_rtlabelid.get();
            let mut ifrtlabelbuf = [0u8; RTLABEL_LEN];
            if rtlabelid != 0 && route::rtlabel_id2name(rtlabelid, &mut ifrtlabelbuf).is_some() {
                return UserPtr::<u8>::new(ifr.ifr_data() as usize)
                    .copyoutstr(&ifrtlabelbuf)
                    .map(|_| ());
            }
            return Err(Errno::ENOENT);
        }

        SIOCGIFPRIORITY => ifr.set_ifr_metric(i32::from(ifp.if_priority.get())),

        SIOCGIFRDOMAIN => ifr.set_ifr_rdomainid(ifp.if_rdomain.get() as i32),

        SIOCGIFLLPRIO => ifr.set_ifr_llprio(i32::from(ifp.if_llprio.get())),

        _ => kpanic(format_args!("invalid ioctl {cmd}")),
    }

    Ok(())
}

/// `if_sffpage_check(data)`: a transceiver page request names the EEPROM or the diagnostics
/// (`EINVAL` otherwise).
fn if_sffpage_check(req: &mut [u8]) -> Result<(), Errno> {
    match req_view::<IfSffpage>(req)?.sff_addr {
        IFSFF_ADDR_EEPROM | IFSFF_ADDR_DDM => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_txhprio_l2_check(hdrprio)`: a valid layer 2 transmit header priority setting.
pub fn if_txhprio_l2_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_txhprio_l3_check(hdrprio)`: a valid layer 3 transmit header priority setting.
pub fn if_txhprio_l3_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET | IF_HDRPRIO_PAYLOAD => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_rxhprio_l2_check(hdrprio)`: a valid layer 2 receive header priority setting.
pub fn if_rxhprio_l2_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET | IF_HDRPRIO_OUTER => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// `if_rxhprio_l3_check(hdrprio)`: a valid layer 3 receive header priority setting.
pub fn if_rxhprio_l3_check(hdrprio: i32) -> Result<(), Errno> {
    match hdrprio {
        IF_HDRPRIO_PACKET | IF_HDRPRIO_PAYLOAD | IF_HDRPRIO_OUTER => Ok(()),
        _ if (IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX).contains(&hdrprio) => Ok(()),
        _ => Err(Errno::EINVAL),
    }
}

/// The addresses of one interface `ifconf` copies under one shared hold of the net lock
/// before it lets the lock go to copy them out: an interface with more takes the lock again
/// for each further batch (the module's redesign).
const IFCONF_BATCH: usize = 8;

/// `ifconf(data)`: return interface configuration of system. List may be used in later
/// ioctl's (above) to get other information. One `struct ifreq` per interface address (an
/// address longer than a `sockaddr` written whole after the name, its row that much longer;
/// an interface without one gets a row of an empty address) into the user's `ifc_req` array
/// of `ifc_len` bytes; `ifc_len` ends as the bytes written. `ifc_len` 0 asks for the length
/// needed. As the C, which compares the `int` `ifc_len` with a `size_t`, a negative `ifc_len`
/// bounds no `sockaddr`-sized row, and bounds the longer rows (an `int` comparison).
fn ifconf(req: &mut [u8]) -> Result<(), Errno> {
    let ifc = req_view::<Ifconf>(req)?;
    let mut space = ifc.ifc_len;

    // If ifc->ifc_len is 0, fill it in with the needed size and return.
    if space == 0 {
        let net = NETLOCK.read();
        let held = net_held(&net);
        for ifp in &IFNETLIST {
            if ifp.if_addrlist.is_empty() {
                space = space.wrapping_add(IFREQ_SIZE as i32);
            } else {
                for ifa in &ifp.if_addrlist {
                    let sa_len = ifa.ifa_addr.read(&held).map_or(0, |sa| usize::from(sa[0]));
                    if sa_len > SOCKADDR_SIZE {
                        space = space.wrapping_add((sa_len - SOCKADDR_SIZE) as i32);
                    }
                    space = space.wrapping_add(IFREQ_SIZE as i32);
                }
            }
        }
        drop(net);
        ifc.ifc_len = space;
        return Ok(());
    }

    let mut ifrp = ifc.ifc_req() as usize;

    let tmp = IF_TMPLIST_LOCK.write();
    let if_tmplist: List<IfTmplist> = List::new();
    {
        let _net = NETLOCK.read();
        for ifp in &IFNETLIST {
            if_tmplist.insert_tail(if_ref(ifp));
        }
    }

    let error = ifconf_rows(&if_tmplist, &mut ifrp, &mut space);

    // free:
    while let Some(ifp) = if_tmplist.pop_head() {
        if_put(Some(ifp));
    }
    drop(tmp);

    if error.is_ok() {
        ifc.ifc_len = ifc.ifc_len.wrapping_sub(space);
    }
    error
}

/// The proof the running code holds the net lock, from its witness `w`.
fn net_held<W: hal::rwlock::Witness>(w: &W) -> NetHeld<'_> {
    match NetHeld::of(w) {
        Some(h) => h,
        None => kpanic(format_args!("not a witness of the net lock")),
    }
}

/// The `sizeof(struct ifreq)` rows `ifconf` writes.
const IFREQ_SIZE: usize = size_of::<Ifreq>();

/// `sizeof(struct sockaddr)`.
const SOCKADDR_SIZE: usize = size_of::<Sockaddr>();

/// `space < sizeof(ifr)`, the C's comparison of an `int` with a `size_t` (the `int`
/// converted: a negative `space` is never short).
fn ifconf_short(space: i32) -> bool {
    (space as isize as usize) < IFREQ_SIZE
}

/// `ifconf`'s rows of the interfaces of `if_tmplist` (each referenced, the temporary list
/// `IF_TMPLIST_LOCK` keeps), from the user address `ifrp` on, while `space` lasts: the
/// interface's addresses referenced on a temporary list under the shared net lock (`ifaref`,
/// as the C), the first [`IFCONF_BATCH`] of them copied in that same hold and the others a
/// batch per hold after it, then copied out with the lock let go, then their references
/// dropped (`ifafree`).
fn ifconf_rows(
    if_tmplist: &List<IfTmplist>,
    ifrp: &mut usize,
    space: &mut i32,
) -> Result<(), Errno> {
    let addr_tmplist: List<IfaTmplist> = List::new();
    let mut ifr = Ifreq::zeroed();
    let mut error = Ok(());

    'free: for ifp in if_tmplist {
        if ifconf_short(*space) {
            break 'free;
        }
        ifr.ifr_name = ifp.if_xname.get();

        let mut batch: [Option<SaBytes>; IFCONF_BATCH] = [const { None }; IFCONF_BATCH];
        {
            let net = NETLOCK.read();
            let held = net_held(&net);
            for (i, ifa) in ifp.if_addrlist.iter().enumerate() {
                ifa.ifa_refcnt.take(); // ifaref
                addr_tmplist.insert_tail(ifa);
                if let Some(slot) = batch.get_mut(i) {
                    *slot = ifa.ifa_addr.read(&held);
                }
            }
        }

        if addr_tmplist.is_empty() {
            ifr.ifr_ifru = [0; SOCKADDR_SIZE];
            error = UserPtr::<Ifreq>::new(*ifrp).copyout(&ifr);
            if error.is_err() {
                break 'free;
            }

            *space = space.wrapping_sub(IFREQ_SIZE as i32);
            *ifrp = ifrp.wrapping_add(IFREQ_SIZE);
        } else {
            let mut next = addr_tmplist.first();
            let mut i = 0;
            while let Some(ifa) = next {
                if i == IFCONF_BATCH {
                    // The next batch, from this address on, in another hold of the lock.
                    let net = NETLOCK.read();
                    let held = net_held(&net);
                    let mut at = Some(ifa);
                    for slot in &mut batch {
                        *slot = at.and_then(|a| a.ifa_addr.read(&held));
                        at = at.and_then(List::<IfaTmplist>::next);
                    }
                    i = 0;
                }
                let sa = batch[i].take();
                i += 1;
                next = List::<IfaTmplist>::next(ifa);

                if ifconf_short(*space) {
                    break 'free;
                }
                // The address's bytes (`sa_len` of them, cut at its storage by the copy) and
                // its `sa_len`, the row's length.
                let bytes: &[u8] = sa.as_deref().unwrap_or(&[]);
                let sa_len = bytes.first().map_or(0, |&l| usize::from(l));
                if sa_len <= SOCKADDR_SIZE {
                    // memset(&ifr.ifr_addr, 0) and the address's bytes over it
                    ifr.ifr_ifru = [0; SOCKADDR_SIZE];
                    let n = bytes.len().min(SOCKADDR_SIZE);
                    ifr.ifr_ifru[..n].copy_from_slice(&bytes[..n]);
                    error = UserPtr::<Ifreq>::new(*ifrp).copyout(&ifr);
                    if error.is_err() {
                        break 'free;
                    }

                    *space = space.wrapping_sub(IFREQ_SIZE as i32);
                    *ifrp = ifrp.wrapping_add(IFREQ_SIZE);
                } else {
                    let total = (IFNAMSIZ + sa_len) as i32;

                    if *space < total {
                        break 'free;
                    }
                    error = UserPtr::<u8>::new(*ifrp).copyout_slice(&ifr.ifr_name);
                    if error.is_err() {
                        break 'free;
                    }
                    error = UserPtr::<u8>::new(ifrp.wrapping_add(IFNAMSIZ)).copyout_slice(bytes);
                    if error.is_err() {
                        break 'free;
                    }

                    *space = space.wrapping_sub(total);
                    *ifrp = ifrp.wrapping_add(total as usize);
                }
            }
        }

        while let Some(ifa) = addr_tmplist.pop_head() {
            route::ifafree(ifa);
        }
    }

    // free:
    while let Some(ifa) = addr_tmplist.pop_head() {
        route::ifafree(ifa);
    }
    error
}

/// `if_getdata(ifp, data)`: the interface's `struct if_data` (`SIOCGIFDATA`, the routing
/// socket's messages): its members, its counters and its queues' counters added.
pub fn if_getdata(ifp: &Ifnet, data: &mut IfData) {
    data.ifi_type = ifp.if_type.get();
    data.ifi_addrlen = ifp.if_addrlen.get();
    data.ifi_hdrlen = ifp.if_hdrlen.get();
    data.ifi_link_state = ifp.if_link_state.get();
    data.ifi_mtu = ifp.if_mtu.get();
    data.ifi_metric = ifp.if_metric.get();
    data.ifi_baudrate = ifp.if_baudrate.get();
    data.ifi_capabilities = ifp.if_capabilities.get();
    data.ifi_rdomain = ifp.if_rdomain.get();
    let (sec, usec) = ifp.if_lastchange.get();
    data.ifi_lastchange = IfTimeval {
        tv_sec: sec,
        tv_usec: usec,
    };

    let counters: [u64; IFC_NCOUNTERS] = core::array::from_fn(|i| {
        let mut v = ifp.if_data_counters[i].get();
        if let Some(c) = ifp.if_counters.get() {
            v += c[i].load(Ordering::Relaxed);
        }
        v
    });
    data.ifi_ipackets = counters[IfCounters::IfcIpackets as usize];
    data.ifi_ierrors = counters[IfCounters::IfcIerrors as usize];
    data.ifi_opackets = counters[IfCounters::IfcOpackets as usize];
    data.ifi_oerrors = counters[IfCounters::IfcOerrors as usize];
    data.ifi_collisions = counters[IfCounters::IfcCollisions as usize];
    data.ifi_ibytes = counters[IfCounters::IfcIbytes as usize];
    data.ifi_obytes = counters[IfCounters::IfcObytes as usize];
    data.ifi_imcasts = counters[IfCounters::IfcImcasts as usize];
    data.ifi_omcasts = counters[IfCounters::IfcOmcasts as usize];
    data.ifi_iqdrops = counters[IfCounters::IfcIqdrops as usize];
    data.ifi_oqdrops = counters[IfCounters::IfcOqdrops as usize];
    data.ifi_noproto = counters[IfCounters::IfcNoproto as usize];

    for i in 0..ifp.if_nifqs() {
        ifq_add_data(ifp.ifq(i), data);
    }

    for i in 0..ifp.if_niqs() {
        ifiq_add_data(ifp.ifiq(i), data);
    }
}

/// `ifg_head`: every interface group (the module's redesign: one array replaced whole).
static IFG_HEAD: ReadMostly<IfgSet> = ReadMostly::new(IfgSet::new());

/// `ifg_head`, borrowed for a reader (the rule of `IfgGroup::members`).
pub fn ifg_head() -> hal::slot::ReadMostlyRef<'static, IfgSet> {
    IFG_HEAD.read()
}

/// `set` with `g` appended (a new array, `M_IFGROUP`); `None` when it cannot be had now.
fn ifg_set_with(set: &IfgSet, g: &IfgRef, flags: i32) -> Option<IfgSet> {
    let old = set.refs();
    let a = MArray::new(M_IFGROUP, flags, old.len() + 1, |i| {
        old.get(i).map_or_else(|| g.clone(), Clone::clone)
    })?;
    Some(IfgSet(Some(hal::malloc::MArc::new(M_IFGROUP, flags, a)?)))
}

/// `set` without `g` (a new array, the order kept; empty as `None`).
fn ifg_set_without(set: &IfgSet, g: &IfgGroup) -> IfgSet {
    let keep = |r: &&IfgRef| !core::ptr::eq(core::ptr::from_ref::<IfgGroup>(r), g);
    let n = set.refs().iter().filter(keep).count();
    if n == 0 {
        return IfgSet::new();
    }
    let mut it = set.refs().iter().filter(keep);
    let a = MArray::new(M_IFGROUP, M_WAITOK, n, |_| {
        it.next()
            .cloned()
            .unwrap_or_else(|| kpanic(format_args!("ifg_set_without: count")))
    });
    match a.and_then(|a| hal::malloc::MArc::new(M_IFGROUP, M_WAITOK, a)) {
        Some(a) => IfgSet(Some(a)),
        None => kpanic(format_args!("if_delgroup: out of memory")),
    }
}

/// `members` with `ifp` appended; `None` when it cannot be had now.
fn ifg_members_with(m: &IfgMembers, ifp: &'static Ifnet) -> Option<IfgMembers> {
    let old = m.as_slice();
    let a = MArray::new(M_IFGROUP, M_NOWAIT, old.len() + 1, |i| {
        old.get(i).copied().unwrap_or(ifp)
    })?;
    Some(IfgMembers(Some(hal::malloc::MArc::new(
        M_IFGROUP, M_NOWAIT, a,
    )?)))
}

/// `members` without `ifp` (the order kept; empty as `None`).
fn ifg_members_without(m: &IfgMembers, ifp: &Ifnet) -> IfgMembers {
    let keep = |x: &&&'static Ifnet| !core::ptr::eq(**x, ifp);
    let n = m.as_slice().iter().filter(keep).count();
    if n == 0 {
        return IfgMembers::new();
    }
    let mut it = m.as_slice().iter().filter(keep);
    let a = MArray::new(M_IFGROUP, M_WAITOK, n, |_| {
        it.next()
            .copied()
            .unwrap_or_else(|| kpanic(format_args!("ifg_members_without: count")))
    });
    match a.and_then(|a| hal::malloc::MArc::new(M_IFGROUP, M_WAITOK, a)) {
        Some(a) => IfgMembers(Some(a)),
        None => kpanic(format_args!("if_delgroup: out of memory")),
    }
}

/// A new group named `groupname` with no member (its count 1), unpublished.
fn ifg_new(groupname: &[u8]) -> Option<IfgRef> {
    hal::malloc::MArc::new(
        M_IFGROUP,
        M_NOWAIT,
        IfgGroup {
            ifg_group: name_array(groupname),
            ifg_refcnt: AtomicU32::new(1),
            ifg_pf_kif: hal::link::PtrWord::new(),
            ifg_carp_demoted: AtomicCell::new(),
            ifg_members: ReadMostly::new(IfgMembers::new()),
        },
    )
}

/// `if_creategroup(groupname)`: an interface group without members, made known to pf and
/// listed; `None` when memory is short (`M_NOWAIT`). Under the net lock.
pub fn if_creategroup(groupname: &[u8]) -> Option<IfgRef> {
    let ifg = ifg_new(groupname)?;
    let head = ifg_set_with(&IFG_HEAD.read(), &ifg, M_NOWAIT)?;

    if let Some(pf) = pf() {
        pf.attach_ifgroup(&ifg);
    }
    drop(IFG_HEAD.replace(head));

    Some(ifg)
}

/// The group named `groupname` on `ifg_head`, a reference taken.
fn ifg_find(groupname: &[u8]) -> Option<IfgRef> {
    IFG_HEAD
        .read()
        .refs()
        .iter()
        .find(|g| name_eq(&g.ifg_group, groupname))
        .cloned()
}

/// `if_addgroup(ifp, groupname)`: `ifp` joins the group (made if it is new): `EINVAL` for an
/// empty or too long name or one ending in a digit, `EEXIST` for a group it is in, `ENOMEM`.
/// Under the net lock.
pub fn if_addgroup(ifp: &'static Ifnet, groupname: &[u8]) -> Result<(), Errno> {
    let groupname = cstr(groupname);
    let namelen = groupname.len();
    if namelen == 0 || namelen >= IFNAMSIZ || groupname[namelen - 1].is_ascii_digit() {
        return Err(Errno::EINVAL);
    }

    if ifp
        .groups()
        .iter()
        .any(|g| name_eq(&g.ifg_group, groupname))
    {
        return Err(Errno::EEXIST);
    }

    // What the C allocates first (its `ifg_list` and `ifg_member`, M_NOWAIT) is the
    // interface's and the group's new arrays here, made before anything is published.
    let (ifg, created) = match ifg_find(groupname) {
        Some(g) => (g, false),
        None => (ifg_new(groupname).ok_or(Errno::ENOMEM)?, true),
    };
    let groups = ifg_set_with(&ifp.groups(), &ifg, M_NOWAIT).ok_or(Errno::ENOMEM)?;
    let members = ifg_members_with(&ifg.members(), ifp).ok_or(Errno::ENOMEM)?;

    if created {
        // if_creategroup
        let head = ifg_set_with(&IFG_HEAD.read(), &ifg, M_NOWAIT).ok_or(Errno::ENOMEM)?;
        if let Some(pf) = pf() {
            pf.attach_ifgroup(&ifg);
        }
        drop(IFG_HEAD.replace(head));
    } else {
        ifg.ifg_refcnt.fetch_add(1, Ordering::Relaxed);
    }
    kassert!(ifg.refcnt() != 0);

    drop(ifg.ifg_members.replace(members));
    drop(ifp.if_groups.replace(groups));

    if let Some(pf) = pf() {
        pf.group_addmember(groupname);
    }

    Ok(())
}

/// `if_delgroup(ifp, groupname)`: `ifp` leaves the group (freed with its last member);
/// `ENOENT` for a group it is not in. Under the net lock.
pub fn if_delgroup(ifp: &Ifnet, groupname: &[u8]) -> Result<(), Errno> {
    let Some(ifg) = ifp
        .groups()
        .refs()
        .iter()
        .find(|g| name_eq(&g.ifg_group, groupname))
        .cloned()
    else {
        return Err(Errno::ENOENT);
    };

    let groups = ifg_set_without(&ifp.groups(), &ifg);
    drop(ifp.if_groups.replace(groups));

    let members = ifg_members_without(&ifg.members(), ifp);
    drop(ifg.ifg_members.replace(members));

    if let Some(pf) = pf() {
        pf.group_delmember(cstr(groupname));
    }

    kassert!(ifg.refcnt() != 0);
    if ifg.ifg_refcnt.fetch_sub(1, Ordering::Relaxed) == 1 {
        let head = ifg_set_without(&IFG_HEAD.read(), &ifg);
        drop(IFG_HEAD.replace(head));
        if let Some(pf) = pf() {
            pf.detach_ifgroup(&ifg);
        }
    }

    // The last reference (the group's own, when it left ifg_head) goes here.
    drop(ifg);
    Ok(())
}

/// `sizeof(struct ifg_req)`: one row of a group request's user array.
const IFG_REQ_SIZE: usize = size_of::<IfgReq>();

/// One `ifg_req` naming `name` (a group's or a member's: one union, zeroed, the name
/// `strlcpy`d) copied out to the user's array at `ifgp`.
fn ifgrq_copyout(name: &[u8], ifgp: usize) -> Result<(), Errno> {
    let ifgrq = IfgReq {
        ifgrq_ifgrqu: name_array(name),
    };
    UserPtr::<IfgReq>::new(ifgp).copyout(&ifgrq)
}

/// The rows of a group request (`SIOCGIFGROUP`, `SIOCGIFGMEMB`, `SIOCGIFGLIST`): each of
/// `names` copied out to the user's array `ifgr_groups` while `ifgr_len` bytes last; `EINVAL`
/// at the first that does not fit (the rows written before it stay, as the C's).
fn ifgrq_rows(ifgr: &Ifgroupreq, names: impl Iterator<Item = [u8; IFNAMSIZ]>) -> Result<(), Errno> {
    let mut len = ifgr.ifgr_len as usize;
    let mut ifgp = ifgr.ifgr_groups() as usize;

    for name in names {
        if len < IFG_REQ_SIZE {
            return Err(Errno::EINVAL);
        }
        ifgrq_copyout(&name, ifgp)?;
        len -= IFG_REQ_SIZE;
        ifgp += IFG_REQ_SIZE;
    }

    Ok(())
}

/// `if_getgroup(data, ifp)`: stores all groups from an interface in memory pointed to by
/// `data` (`ifgr_len` 0: the length they need). The groups are a snapshot taken under the
/// shared net lock and copied out without it, where the C keeps a temporary list of counted
/// groups under `if_tmplist_lock` (the snapshot holds them).
fn if_getgroup(req: &mut [u8], ifp: &Ifnet) -> Result<(), Errno> {
    let ifgr = req_view::<Ifgroupreq>(req)?;

    if ifgr.ifgr_len == 0 {
        let _net = NETLOCK.read();
        let n = ifp.groups().len();
        ifgr.ifgr_len = ifgr.ifgr_len.wrapping_add((n * IFG_REQ_SIZE) as u32);
        return Ok(());
    }

    let groups = {
        let _net = NETLOCK.read();
        ifp.groups().snapshot()
    };

    ifgrq_rows(ifgr, groups.iter().map(|g| g.ifg_group))
}

/// `if_getgroupmembers(data)`: stores all members of a group in memory pointed to by `data`
/// (`ENOENT` for no such group; `ifgr_len` 0: the length they need), from a snapshot of the
/// members taken with a reference to each under the shared net lock, as the C's `if_tmplist`.
fn if_getgroupmembers(req: &mut [u8]) -> Result<(), Errno> {
    let ifgr = req_view::<Ifgroupreq>(req)?;

    let members = {
        let _net = NETLOCK.read();

        let Some(ifg) = ifg_find(&ifgr.ifgr_name) else {
            return Err(Errno::ENOENT);
        };

        if ifgr.ifgr_len == 0 {
            let n = ifg.members().as_slice().len();
            ifgr.ifgr_len = ifgr.ifgr_len.wrapping_add((n * IFG_REQ_SIZE) as u32);
            return Ok(());
        }

        let members = ifg.members().snapshot();
        for ifp in members.iter() {
            let _ = if_ref(ifp);
        }
        members
    };

    let error = ifgrq_rows(ifgr, members.iter().map(|ifp| ifp.if_xname.get()));

    for ifp in members.iter() {
        if_put(Some(ifp));
    }

    error
}

/// `if_getgroupattribs(data)`: the group's attributes (`carp(4)`'s demotion counter);
/// `ENOENT` for no such group. Under the shared net lock.
fn if_getgroupattribs(req: &mut [u8]) -> Result<(), Errno> {
    let ifgr = req_view::<Ifgroupreq>(req)?;

    let Some(ifg) = ifg_find(&ifgr.ifgr_name) else {
        return Err(Errno::ENOENT);
    };

    ifgr.set_ifgr_attrib(IfgAttrib {
        ifg_carp_demoted: ifg.ifg_carp_demoted.get(),
    });

    Ok(())
}

/// `if_setgroupattribs(data)`: the group's demotion counter moved by the request's
/// (`EINVAL` outside 0..255, `ENOENT` for no such group), each member's driver told
/// (`SIOCSIFGATTR`). Under the net lock.
fn if_setgroupattribs(req: &mut [u8]) -> Result<(), Errno> {
    let (name, demote) = {
        let ifgr = req_view::<Ifgroupreq>(req)?;
        (ifgr.ifgr_name, ifgr.ifgr_attrib().ifg_carp_demoted)
    };

    let Some(ifg) = ifg_find(&name) else {
        return Err(Errno::ENOENT);
    };

    let sum = demote.wrapping_add(ifg.ifg_carp_demoted.get());
    if !(0..=0xff).contains(&sum) {
        return Err(Errno::EINVAL);
    }

    ifg.ifg_carp_demoted.set(sum);

    let members = ifg.members().snapshot();
    for ifp in members.iter() {
        let _ = if_ioctl(ifp, SIOCSIFGATTR, req_arg(req, SIOCSIFGATTR));
    }

    Ok(())
}

/// `if_getgrouplist(data)`: stores all groups in memory pointed to by `data` (`ifgr_len` 0:
/// the length they need), from a snapshot of `ifg_head` (the C's `ifg_tmplist`).
fn if_getgrouplist(req: &mut [u8]) -> Result<(), Errno> {
    let ifgr = req_view::<Ifgroupreq>(req)?;

    if ifgr.ifgr_len == 0 {
        let _net = NETLOCK.read();
        let n = ifg_head().len();
        ifgr.ifgr_len = ifgr.ifgr_len.wrapping_add((n * IFG_REQ_SIZE) as u32);
        return Ok(());
    }

    let groups = {
        let _net = NETLOCK.read();
        ifg_head().snapshot()
    };

    ifgrq_rows(ifgr, groups.iter().map(|g| g.ifg_group))
}

/// `if_group_routechange(dst, mask)`: a route changed: the egress group is built again when
/// it is a default route (`0.0.0.0/0` or `::/0`; `mask` `None` or of length 0 counts).
pub fn if_group_routechange(dst: &[u8], mask: Option<&[u8]>) {
    let zero = |sa: &[u8], r: core::ops::Range<usize>| {
        sa.get(r).is_some_and(|b| b.iter().all(|&c| c == 0))
    };
    match sa_family(dst) {
        // sin_addr at 4..8 of a sockaddr_in
        Some(AF_INET)
            if zero(dst, 4..8) && mask.is_some_and(|m| m.first() == Some(&0) || zero(m, 4..8)) =>
        {
            let _ = if_group_egress_build();
        }
        // sin6_addr at 8..24 of a sockaddr_in6
        Some(AF_INET6)
            if zero(dst, 8..24)
                && mask.is_some_and(|m| m.first() == Some(&0) || zero(m, 8..24)) =>
        {
            let _ = if_group_egress_build();
        }
        _ => {}
    }
}

/// `if_group_egress_build()`: the egress group made again of the interfaces of the default
/// routes of table 0 that are not rejects or black holes, IPv4's then IPv6's.
pub fn if_group_egress_build() -> Result<(), Errno> {
    if let Some(ifg) = ifg_find(IFG_EGRESS) {
        // TAILQ_FOREACH_SAFE over the members, each leaving: the first one until none is left.
        loop {
            // The member read out of the snapshot first: if_delgroup replaces it.
            let first = ifg.members().iter().next();
            let Some(ifp) = first else {
                break;
            };
            if if_delgroup(ifp, IFG_EGRESS).is_err() {
                break;
            }
        }
    }

    // The default routes of table 0, IPv4's then IPv6's (a zero address and mask of each).
    for af in [AF_INET, AF_INET6] {
        let mut any = [0u8; 28];
        any[0] = if af == AF_INET { 16 } else { 28 };
        any[1] = af;
        let Some(sa) = SockaddrView::new(&any) else {
            continue;
        };
        let mut rt = rtable::rtable_lookup(0, sa, Some(sa), None, RTP_ANY);
        while let Some(r) = rt {
            if !r.rt_flags.isset(RTF_REJECT | RTF_BLACKHOLE)
                && let Some(ifp) = IfRef::get(r.rt_ifidx.get())
            {
                let _ = if_addgroup(ifp.ifp(), IFG_EGRESS);
            }
            rt = rtable::rtable_iterate(r);
        }
    }

    Ok(())
}

/// `ifpromisc(ifp, pswitch)`: promiscuous mode on or off, counted so that only the first
/// "on" and the last "off" reach the driver (`SIOCSIFFLAGS`, its flags and count put back
/// when it fails). Under the net lock.
pub fn ifpromisc(ifp: &'static Ifnet, pswitch: bool) -> Result<(), Errno> {
    net_assert_locked(); // modifying if_flags and if_pcount

    let oif_flags = ifp.if_flags.get();
    let oif_pcount = ifp.if_pcount.get();
    if pswitch {
        ifp.if_pcount.set(oif_pcount + 1);
        if oif_pcount != 0 {
            return Ok(());
        }
        ifp.if_flags.set(ifp.if_flags.get() | IFF_PROMISC);
    } else {
        ifp.if_pcount.set(oif_pcount - 1);
        if oif_pcount - 1 > 0 {
            return Ok(());
        }
        ifp.if_flags.set(ifp.if_flags.get() & !IFF_PROMISC);
    }

    if ifp.if_flags.get() & IFF_UP == 0 {
        return Ok(());
    }

    let error = if_ioctl_flags(ifp, SIOCSIFFLAGS, ifp.if_flags.get());
    if error.is_err() {
        ifp.if_flags.set(oif_flags);
        ifp.if_pcount.set(oif_pcount);
    }

    error
}

/// `ifsetlro(ifp, on)`: LRO on or off: the driver's `SIOCSIFXFLAGS`, else the flag set by the
/// stack for an interface that can (`ENOTSUP` otherwise; `EBUSY` for a bridge port). Under
/// the net and kernel locks.
pub fn ifsetlro(ifp: &'static Ifnet, on: bool) -> Result<(), Errno> {
    let s = hal::intr::splraise(IPL_NET);

    net_assert_locked(); // for ioctl
    hal::klock::kernel_assert_locked();

    let error = 'out: {
        if if_ioctl_flags(ifp, SIOCSIFXFLAGS, if on { IFXF_LRO } else { 0 }).is_ok() {
            break 'out Ok(());
        }

        if ifp.if_capabilities.get() & IFCAP_LRO == 0 {
            break 'out Err(Errno::ENOTSUP);
        }

        if on && ifp.if_xflags.get() & IFXF_LRO == 0 {
            // NETHER > 0
            if ifp.if_type.get() == IFT_ETHER
                && ifp.if_arpcom.get().is_some()
                && crate::net::if_ethersubr::ether_brport_isset(ifp).is_err()
            {
                break 'out Err(Errno::EBUSY);
            }
            ifp.if_xflags.set(ifp.if_xflags.get() | IFXF_LRO);
        } else if !on && ifp.if_xflags.get() & IFXF_LRO != 0 {
            ifp.if_xflags.set(ifp.if_xflags.get() & !IFXF_LRO);
        }
        Ok(())
    };

    drop(s);

    error
}

/// `ifa_add(ifp, ifa)`: a protocol's address linked at the tail of the interface's list
/// (under the exclusive net lock). The address lives while it is linked (`bsd`'s `in_ifaddr`,
/// the module's redesign).
pub fn ifa_add(ifp: &Ifnet, ifa: &'static Ifaddr) {
    net_assert_locked_exclusive();
    ifp.if_addrlist.insert_tail(ifa);
}

/// `ifa_del(ifp, ifa)`: the address taken off the interface's list.
pub fn ifa_del(ifp: &Ifnet, ifa: &Ifaddr) {
    net_assert_locked_exclusive();
    kassert!(ifp.if_addrlist.iter().any(|a| core::ptr::eq(a, ifa)));
    ifp.if_addrlist.remove(ifa);
}

/// `ifnewlladdr(ifp)`: the interface bounced after its link address changed (down for a
/// moment if it was up, then up), its IPv6 link-local address made again unless it routes,
/// and left down if it was. Under the net and kernel locks.
pub fn ifnewlladdr(ifp: &'static Ifnet) {
    let i_am_router = inet().is_some_and(Inet::ip6_forwarding);

    net_assert_locked(); // for ioctl and in6
    hal::klock::kernel_assert_locked();

    let up = ifp.if_flags.get() & IFF_UP != 0;
    let setflags = |flags: i32| {
        ifp.if_flags.set(flags);
        let _ = if_ioctl_flags(ifp, SIOCSIFFLAGS, flags);
    };

    if up {
        // go down for a moment...
        setflags(ifp.if_flags.get() & !IFF_UP);
    }

    setflags(ifp.if_flags.get() | IFF_UP);

    // Update the link-local address. Don't do it if we're a router to avoid confusing hosts
    // on the network.
    if !i_am_router
        && let Some(i) = inet()
        && i.in6_purge_linklocal(ifp)
    {
        if_hooks_run(ifp, &ifp.rt().if_addrhooks);
        let _ = i.in6_ifattach(ifp);
    }

    if !up {
        // go back down
        setflags(ifp.if_flags.get() & !IFF_UP);
    }
}

/// `if_rxr_init(rxr, lwm, hwm)`: a receive ring's accounting, its current watermark at the
/// low one.
pub fn if_rxr_init(rxr: &mut IfRxring, lwm: u32, hwm: u32) {
    *rxr = hal::user::zeroed();

    rxr.rxr_adjusted = crate::clock::ticks();
    rxr.rxr_lwm = lwm;
    rxr.rxr_cwm = lwm;
    rxr.rxr_hwm = hwm;
}

/// `if_rxr_adjust_cwm(rxr)`: the current watermark one higher when the ring ran below its
/// low one.
fn if_rxr_adjust_cwm(rxr: &mut IfRxring) {
    if rxr.rxr_alive >= rxr.rxr_lwm {
        return;
    } else if rxr.rxr_cwm < rxr.rxr_hwm {
        rxr.rxr_cwm += 1;
    }

    rxr.rxr_adjusted = crate::clock::ticks();
}

/// `if_rxr_livelocked(rxr)`: the system is too busy: the current watermark one lower, at
/// most once a tick.
pub fn if_rxr_livelocked(rxr: &mut IfRxring) {
    let ticks = crate::clock::ticks();
    if ticks.wrapping_sub(rxr.rxr_adjusted) >= 1 {
        if rxr.rxr_cwm > rxr.rxr_lwm {
            rxr.rxr_cwm -= 1;
        }

        rxr.rxr_adjusted = ticks;
    }
}

/// `if_rxr_get(rxr, max)`: how many of `max` slots the driver may fill now.
pub fn if_rxr_get(rxr: &mut IfRxring, max: u32) -> u32 {
    if crate::clock::ticks().wrapping_sub(rxr.rxr_adjusted) >= 1 {
        // we're free to try for an adjustment
        if_rxr_adjust_cwm(rxr);
    }

    if rxr.rxr_alive >= rxr.rxr_cwm {
        return 0;
    }

    let diff = (rxr.rxr_cwm - rxr.rxr_alive).min(max);
    rxr.rxr_alive += diff;

    diff
}

/// `if_rxr_info_ioctl(uifri, t, e)`: `SIOCGIFRXR`'s answer: the user's `struct if_rxrinfo`
/// at `uifri` (`ifr_data`) told the `t` rings, and as many of `e` as it has room for (its
/// `ifri_total`) copied to its `ifri_entries`. `e` holds the `t` rings (a shorter slice
/// bounds the copy where the C trusts its caller).
pub fn if_rxr_info_ioctl(uifri: usize, t: u32, e: &[IfRxringInfo]) -> Result<(), Errno> {
    let uifri = UserPtr::<IfRxrinfo>::new(uifri);
    let mut kifri = uifri.copyin()?;

    let n = t.min(kifri.ifri_total) as usize;
    kifri.ifri_total = t;

    if n > 0 {
        let rings = &e[..n.min(e.len())];
        UserPtr::<IfRxringInfo>::new(kifri.ifri_entries as usize).copyout_slice(rings)?;
    }

    uifri.copyout(&kifri)
}

/// `if_rxr_ioctl(ifri, name, size, rxr)`: `SIOCGIFRXR` for a driver with one ring, named
/// `name` (none: an empty name), of packets of `size` bytes.
pub fn if_rxr_ioctl(
    ifri: usize,
    name: Option<&[u8]>,
    size: u32,
    rxr: &IfRxring,
) -> Result<(), Errno> {
    let ifr = IfRxringInfo {
        ifr_name: name.map_or([0; IFNAMSIZ], name_array),
        ifr_size: size,
        ifr_info: *rxr,
    };

    if_rxr_info_ioctl(ifri, 1, core::slice::from_ref(&ifr))
}

/// `unhandled_af(af)`: `panic(9)` on an address family the caller cannot handle.
pub fn unhandled_af(af: i32) -> ! {
    kpanic(format_args!("unhandled af {af}"))
}

/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests of the interface core over the mbuf tests' real memory: every test holds
    // the page system's lock (`mbuf::tests::setup`), so the index map, `ifnetlist` and the
    // counts are each test's own; interfaces are leaked, as a softc's lives for ever.

    use std::boxed::Box;
    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicU32;
    use std::vec::Vec;

    use super::*;

    fn proto_other(_ifp: &'static Ifnet, m: MbufBox, _ns: Option<&mut Netstack>) {
        drop(m);
    }

    /// Real memory, a fresh index map, an empty `ifnetlist` and no group.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        let g = crate::net::mbuf::tests::setup();
        softnet_test_install();
        if_idxmap_test_reset();
        ifnet_test_reset();
        g
    }

    fn map_limit() -> u32 {
        IF_MAP.load(&smr::read()).map_or(0, IfMap::limit)
    }

    #[test]
    fn attached_interfaces_get_unique_indexes_and_the_map_grows() {
        let _g = setup();
        assert_eq!(map_limit(), 8);
        let ifps: Vec<&'static Ifnet> = (0..20).map(|_| test_ifnet(b"tst0").0).collect();
        let idx: Vec<u32> = ifps.iter().map(|i| i.if_index.get()).collect();
        assert_eq!(idx, (1..=20).collect::<Vec<u32>>());
        // 8 -> 16 -> 32, the old maps retired.
        assert_eq!(map_limit(), 32);
        for &ifp in &ifps {
            let before = ifp.rt().if_refcnt.read();
            let got = if_get(ifp.if_index.get()).expect("attached");
            assert!(core::ptr::eq(got, ifp));
            assert_eq!(ifp.rt().if_refcnt.read(), before + 1);
            if_put(Some(got));
            assert_eq!(ifp.rt().if_refcnt.read(), before);
        }
        assert!(if_get(0).is_none() && if_get(21).is_none() && if_get(99).is_none());
        {
            let g = smr::read();
            assert!(if_get_smr(5, &g).is_some_and(|i| core::ptr::eq(i, ifps[4])));
        }
        // The index map's own reference, then the interface's: `if_idxmap_remove`'s locked
        // part empties the slot and frees the index.
        if_idxmap_unlink(ifps[2]);
        assert!(if_get(3).is_none());
        {
            let r = IfRef::get(4).expect("attached");
            assert_eq!(r.if_index.get(), 4);
            assert!(core::ptr::eq(r.ifp(), ifps[3]));
        }
    }

    #[test]
    fn the_softnet_input_path_reaches_the_driver_and_the_protocol() {
        let _g = setup();
        let (ifp, d) = test_ifnet(b"tst0");
        TEST_PROTO_INPUTS.store(0, Ordering::Relaxed);
        let mut ml = MbufList::new();
        ml_enqueue(&mut ml, test_packet(0, 60));
        ml_enqueue(&mut ml, test_packet(0, 60));
        if_input_process(ifp, &mut ml, 0);
        // Each packet: the driver, the driver again (ns_input), then the protocol.
        assert_eq!(d.inputs.load(Ordering::Relaxed), 4);
        assert_eq!(TEST_PROTO_INPUTS.load(Ordering::Relaxed), 2);
        assert!(ml_empty(&ml));
    }

    static HOOK_RUNS: AtomicU32 = AtomicU32::new(0);
    static HOOK_A: OnceStatic<&'static Task> = OnceStatic::new();

    /// A hook that takes itself off its list as it runs.
    fn hook_a(t: &'static (dyn Any + Send + Sync)) {
        let ifp = t.downcast_ref::<Ifnet>().expect("the interface");
        HOOK_RUNS.fetch_add(1, Ordering::Relaxed);
        if let Some(&me) = HOOK_A.get() {
            if_addrhook_del(ifp, me);
        }
    }

    /// A hook that stays.
    fn hook_b(_t: &'static (dyn Any + Send + Sync)) {
        HOOK_RUNS.fetch_add(10, Ordering::Relaxed);
    }

    #[test]
    fn the_hooks_run_in_order_and_may_remove_themselves() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        HOOK_RUNS.store(0, Ordering::Relaxed);
        let a: &'static Task = HOOK_A.get().copied().unwrap_or_else(|| {
            let t: &'static Task = Box::leak(Box::new(Task::unset()));
            let _ = HOOK_A.set(t);
            t
        });
        let b: &'static Task = Box::leak(Box::new(Task::unset()));
        a.set_ref(hook_a, ifp);
        b.set_ref(hook_b, ifp);
        if_addrhook_add(ifp, Pin::static_ref(a));
        if_addrhook_add(ifp, Pin::static_ref(b));
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 11);
        assert!(!a.pending() && b.pending());
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 21);
        if_addrhook_del(ifp, b);
        assert!(!b.pending());
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 21);
        // A detached interface answers no request.
        assert_eq!(if_ioctl(ifp, 0, &mut []), Ok(()));
        ifp.detached
            .fetch_or(crate::net::if_var::DETACHED_IOCTL, Ordering::Release);
        assert_eq!(if_ioctl(ifp, 0, &mut []), Err(Errno::ENODEV));
    }

    static HOOK_C: OnceStatic<&'static Task> = OnceStatic::new();
    static HOOK_D: OnceStatic<&'static Task> = OnceStatic::new();

    /// A hook that adds another hook (at the tail) as it runs.
    fn hook_c(t: &'static (dyn Any + Send + Sync)) {
        let ifp = t.downcast_ref::<Ifnet>().expect("the interface");
        HOOK_RUNS.fetch_add(1, Ordering::Relaxed);
        if let Some(&d) = HOOK_D.get()
            && !d.pending()
        {
            if_addrhook_add(ifp, Pin::static_ref(d));
        }
    }

    /// The hook `hook_c` adds.
    fn hook_d(_t: &'static (dyn Any + Send + Sync)) {
        HOOK_RUNS.fetch_add(100, Ordering::Relaxed);
    }

    /// Review 1's S6: a hook that adds a hook. The one added at the tail runs in the same walk
    /// (the cursor follows the running hook, the C's `if_hooks_run`), once.
    #[test]
    fn a_hook_added_by_a_hook_runs_in_the_same_walk() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        HOOK_RUNS.store(0, Ordering::Relaxed);
        let leak = || -> &'static Task { Box::leak(Box::new(Task::unset())) };
        let c = leak();
        let d = leak();
        assert!(HOOK_C.set(c).is_ok() && HOOK_D.set(d).is_ok());
        c.set_ref(hook_c, ifp);
        d.set_ref(hook_d, ifp);
        if_addrhook_add(ifp, Pin::static_ref(c));
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 101);
        assert!(c.pending() && d.pending());
        // Both stay: the next walk runs each once (d already linked, not added again).
        if_addrhooks_run(ifp);
        assert_eq!(HOOK_RUNS.load(Ordering::Relaxed), 202);
        if_addrhook_del(ifp, c);
        if_addrhook_del(ifp, d);
    }

    /// Review 1's S6: `if_get` across a detach. A counted reference taken before the detach
    /// holds `if_remove`'s `refcnt_finalize("ifrm")` off until it is put; the index is gone
    /// from the map meanwhile.
    #[test]
    fn a_counted_reference_holds_the_detach_until_it_is_put() {
        static DETACHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        let _g = setup();
        let (ifp, _) = fresh_ifnet(b"tget0");
        if_attach(ifp);
        let idx = ifp.if_index.get();
        let held = if_get(idx).expect("attached");
        DETACHED.store(false, Ordering::SeqCst);
        with_smr_thread(|| {
            let d = std::thread::spawn(move || {
                detach_locked(ifp);
                DETACHED.store(true, Ordering::SeqCst);
            });
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while if_get(idx).inspect(|i| if_put(Some(i))).is_some() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "never removed from the map"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            assert!(
                !DETACHED.load(Ordering::SeqCst),
                "the detach waits for the reference"
            );
            if_put(Some(held));
            d.join().expect("the detach");
            assert!(DETACHED.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn a_protocol_function_keeps_its_cookie() {
        let a = proto_fn_id(test_proto_count);
        let b = proto_fn_id(proto_other);
        assert_ne!(a, 0);
        assert_ne!(a, b);
        assert_eq!(proto_fn_id(test_proto_count), a);
        assert!(proto_fn(a).is_some_and(|f| core::ptr::fn_addr_eq(f, test_proto_count as ProtoFn)));
        assert!(proto_fn(0).is_none());
    }

    #[test]
    fn more_queues_after_attach_are_reached_by_index() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        assert_eq!((ifp.if_nifqs(), ifp.if_niqs()), (1, 1));
        if_attach_queues(ifp, 4);
        if_attach_iqueues(ifp, 2);
        assert_eq!((ifp.if_nifqs(), ifp.if_niqs()), (4, 2));
        for i in 0..4 {
            assert_eq!(ifp.ifq(i).idx(), i);
            assert!(core::ptr::eq(ifp.ifq(i).ifq_if(), ifp));
        }
        assert_eq!(ifp.ifiq(1).idx(), 1);
        assert!(core::ptr::eq(ifp.ifq(0), ifp.if_snd()));
        if_counters_alloc(ifp);
        assert!(ifp.if_counters.get().is_some());
        if_counters_free(ifp);
        assert!(ifp.if_counters.get().is_none());
    }

    /// No CPU but the caller's (the SMR thread's walk over the CPUs, on the host).
    struct Alone;

    impl hal::smr::CpuWalk for Alone {
        fn running(&self, _n: usize) -> bool {
            false
        }
        fn peg(&self, n: usize) {
            panic!("peg({n}) with no other CPU");
        }
        fn unpeg(&self) {}
    }

    fn nothing(_arg: *mut core::ffi::c_void) {}

    /// `f` with an SMR thread and the softnets' workers running beside it (`if_remove`'s
    /// `smr_barrier` and `net_tq_barriers` wait for them), stopped after: a last empty SMR
    /// call wakes the first.
    pub(crate) fn with_smr_thread<R>(f: impl FnOnce() -> R) -> R {
        static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        STOP.store(false, Ordering::SeqCst);
        let th = std::thread::spawn(|| {
            loop {
                hal::smr::wait();
                let _ = hal::smr::collect().run(&Alone);
                if STOP.load(Ordering::SeqCst) {
                    break;
                }
            }
        });
        let workers = std::thread::spawn(|| {
            while !STOP.load(Ordering::SeqCst) {
                for sn in &SOFTNETS {
                    if let Some(tq) = sn.taskq.get() {
                        while !crate::task::worklist_is_empty(tq) {
                            crate::task::run_next_by_hand(tq);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });
        let r = f();
        STOP.store(true, Ordering::SeqCst);
        workers.join().expect("the softnets' workers");
        hal::smr::defer_link(
            Box::leak(Box::new(hal::smr::SmrLink::new())),
            nothing,
            core::ptr::null_mut(),
        );
        hal::smr::idle();
        th.join().expect("the SMR thread");
        r
    }

    /// A fresh interface named `name` with a [`TestDriver`], not attached, its softc leaked.
    fn fresh_ifnet(name: &[u8]) -> (&'static Ifnet, &'static TestDriver) {
        let d: &'static TestDriver = Box::leak(Box::new(TestDriver::new()));
        (test_ifnet_fresh(name, d), d)
    }

    /// The groups of `ifp`, by name.
    fn groups_of(ifp: &Ifnet) -> Vec<Vec<u8>> {
        ifp.groups().iter().map(|g| g.name().to_vec()).collect()
    }

    /// The group `name` of `ifg_head`, its count and its members' indexes.
    fn group(name: &[u8]) -> Option<(u32, Vec<u32>)> {
        ifg_find(name).map(|g| {
            let members = g.members().iter().map(|i| i.if_index.get()).collect();
            (g.refcnt(), members)
        })
    }

    #[test]
    fn an_interface_attaches_with_its_early_settings_and_detaches_whole() {
        let _g = setup();
        let (ifp, _) = fresh_ifnet(b"tst0");
        // The driver's writes before if_attach: the send queue's length, the counters.
        ifp.if_snd_maxlen.set(10);
        if_counters_alloc(ifp);
        if_attach(ifp);
        let (other, _) = fresh_ifnet(b"tst1");
        if_attachhead(other);

        let listed: Vec<u32> = IFNETLIST.iter().map(|i| i.if_index.get()).collect();
        assert_eq!(listed, [other.if_index.get(), ifp.if_index.get()]);
        assert_eq!(ifp.if_snd().ifq_maxlen.get(), 10);
        assert_eq!(other.if_snd().ifq_maxlen.get(), crate::net::ifq::IFQ_MAXLEN);
        assert!(ifp.if_counters.get().is_some());
        assert_eq!(groups_of(ifp), [b"all".to_vec()]);
        assert_eq!(
            group(IFG_ALL),
            Some((2, std::vec![ifp.if_index.get(), other.if_index.get()]))
        );
        let ifidx = ifp.if_index.get();
        if_put(Some(if_get(ifidx).expect("published")));

        {
            let _net = NETLOCK.write();
            assert_eq!(if_addgroup(ifp, b"grp"), Ok(()));
            assert_eq!(if_addgroup(ifp, b"grp"), Err(Errno::EEXIST));
            assert_eq!(if_addgroup(ifp, b"grp1"), Err(Errno::EINVAL));
            assert_eq!(if_addgroup(ifp, b""), Err(Errno::EINVAL));
            assert_eq!(if_delgroup(ifp, b"none"), Err(Errno::ENOENT));
        }
        assert_eq!(groups_of(ifp), [b"all".to_vec(), b"grp".to_vec()]);
        assert_eq!(group(b"grp"), Some((1, std::vec![ifidx])));

        with_smr_thread(|| detach_locked(ifp));
        assert!(if_get(ifidx).is_none());
        let listed: Vec<u32> = IFNETLIST.iter().map(|i| i.if_index.get()).collect();
        assert_eq!(listed, [other.if_index.get()]);
        assert!(ifp.groups().is_empty());
        assert_eq!(group(b"grp"), None);
        assert_eq!(group(IFG_ALL), Some((1, std::vec![other.if_index.get()])));
        assert!(ifp.if_counters.get().is_none());
        // Detached: the driver is no longer asked.
        let mut ifr = Ifreq::zeroed();
        let data = hal::user::bytes_of_mut(core::slice::from_mut(&mut ifr));
        assert_eq!(if_ioctl(ifp, SIOCSIFFLAGS, data), Err(Errno::ENODEV));
        assert!(ifp.if_ioctl_detached());
        with_smr_thread(|| detach_locked(other));
        assert!(IFNETLIST.is_empty());
        assert_eq!(group(IFG_ALL), None);
    }

    /// `if_detach(ifp)` under the kernel lock, as its callers hold it (a driver's detach,
    /// `SIOCIFDESTROY`): bpf(4)'s `bpfdetach` asserts it.
    fn detach_locked(ifp: &'static Ifnet) {
        hal::klock::kernel_lock();
        if_detach(ifp);
        hal::klock::kernel_unlock();
    }

    /// A cloner of `tcl<unit>` interfaces over [`fresh_ifnet`].
    struct TestCloner;

    impl Cloner for TestCloner {
        fn name(&self) -> &'static [u8] {
            b"tcl"
        }

        fn create(&'static self, unit: i32) -> Result<(), Errno> {
            if unit == 99 {
                return Err(Errno::ENOMEM);
            }
            let mut name = std::vec::Vec::from(&b"tcl"[..]);
            name.extend_from_slice(std::format!("{unit}").as_bytes());
            let (ifp, _) = fresh_ifnet(&name);
            if_attach(ifp);
            Ok(())
        }

        fn has_destroy(&self) -> bool {
            true
        }

        fn destroy(&self, ifp: &'static Ifnet) -> Result<(), Errno> {
            if_detach(ifp);
            Ok(())
        }
    }

    static TEST_CLONER: TestCloner = TestCloner;

    fn test_cloner() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| if_clone_attach(&TEST_CLONER));
    }

    #[test]
    fn a_cloner_parses_units_and_creates_and_destroys_its_interfaces() {
        let _g = setup();
        test_cloner();
        let unit = |n: &[u8]| if_clone_lookup(n).map(|(c, u)| (c.name(), u));
        assert_eq!(unit(b"tcl3"), Some((&b"tcl"[..], 3)));
        assert_eq!(unit(b"tcl0"), Some((&b"tcl"[..], 0)));
        assert_eq!(unit(b"tcl12\0junk"), Some((&b"tcl"[..], 12)));
        for bad in [
            &b"tcl"[..],
            b"tcl03",
            b"3",
            b"xyz3",
            b"tcl3a",
            b"tcl99999999999",
        ] {
            assert_eq!(unit(bad), None, "{:?}", core::str::from_utf8(bad));
        }
        assert!(if_cloners().any(|c| c.name() == b"tcl"));

        hal::klock::kernel_lock();
        assert_eq!(if_clone_create(b"tcl3", 0), Ok(()));
        assert_eq!(if_clone_create(b"tcl3", 0), Err(Errno::EEXIST));
        assert_eq!(if_clone_create(b"tcl99", 0), Err(Errno::ENOMEM));
        assert_eq!(if_clone_create(b"nope3", 0), Err(Errno::EINVAL));
        let ifp = if_unit(b"tcl3").expect("created");
        if_put(Some(ifp));
        assert_eq!(groups_of(ifp), [b"all".to_vec(), b"tcl".to_vec()]);
        assert_eq!(if_clone_destroy(b"tcl4"), Err(Errno::ENXIO));
        assert_eq!(with_smr_thread(|| if_clone_destroy(b"tcl3")), Ok(()));
        assert!(if_unit(b"tcl3").is_none());
        assert_eq!(group(b"tcl"), None);
        hal::klock::kernel_unlock();
    }

    #[test]
    fn the_link_name_is_the_interface_s_and_takes_a_new_address() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"vio0");
        ifp.if_type.set(IFT_ETHER);
        ifp.if_addrlen.set(ETHER_ADDR_LEN as u8);
        assert_eq!(if_setlladdr(ifp, &[1; 6]), Err(Errno::EINVAL));
        if_alloc_sadl(ifp);
        {
            let sdl = ifp.sadl();
            let sdl = sdl.as_ref().expect("a link name");
            assert_eq!(
                (sdl.sdl_len, sdl.sdl_family, sdl.sdl_nlen, sdl.sdl_alen),
                (32, AF_LINK, 4, 6)
            );
            assert_eq!(u32::from(sdl.sdl_index), ifp.if_index.get());
            assert_eq!(&sdl.sdl_data[..4], b"vio0");
        }
        assert_eq!(if_setlladdr(ifp, &[2, 4, 6, 8, 10, 12]), Ok(()));
        {
            let sdl = ifp.sadl();
            let sdl = sdl.as_ref().expect("a link name");
            let bytes = hal::user::bytes_of(core::slice::from_ref(&**sdl));
            assert_eq!(SockaddrDl::lladdr(bytes), Some(&[2u8, 4, 6, 8, 10, 12][..]));
        }
        if_free_sadl(ifp);
        assert!(ifp.sadl().is_none());
    }

    /// A `sockaddr_in` of `a.b.c.d`, leaked (an address's storage lives while it is linked):
    /// an `in_ifaddr`'s `ia_addr`, as `bsd` publishes it.
    fn sin(a: [u8; 4]) -> &'static core::cell::Cell<hal::sys::socket::Sockaddr> {
        let mut sa: hal::sys::socket::Sockaddr = hal::user::zeroed();
        sa.sa_len = 16;
        sa.sa_family = AF_INET;
        sa.sa_data[2..6].copy_from_slice(&a);
        Box::leak(Box::new(core::cell::Cell::new(sa)))
    }

    /// The bytes of a socket address.
    fn bytes(sa: &core::cell::Cell<hal::sys::socket::Sockaddr>) -> std::vec::Vec<u8> {
        hal::user::bytes_of(core::slice::from_ref(&sa.get())).to_vec()
    }

    /// An address `addr` with netmask `mask` (and destination `dst`) added to `ifp`.
    fn address(
        ifp: &'static Ifnet,
        addr: [u8; 4],
        mask: Option<[u8; 4]>,
        dst: Option<[u8; 4]>,
    ) -> &'static Ifaddr {
        let ifa: &'static Ifaddr = Box::leak(Box::new(Ifaddr::new()));
        let token = crate::net::if_var::sa_test_token();
        ifa.ifa_addr.set(token, Some(sin(addr)));
        if let Some(m) = mask {
            ifa.ifa_netmask.set(token, Some(sin(m)));
        }
        if let Some(d) = dst {
            ifa.ifa_dstaddr.set(token, Some(sin(d)));
        }
        ifa.ifa_ifp.set(Some(ifp));
        ifa_add(ifp, ifa);
        ifa
    }

    #[test]
    fn addresses_are_found_whole_by_destination_and_by_subnet() {
        let _g = setup();
        let mut ifps = Vec::new();
        let mut addrs = Vec::new();
        for _ in 0..8 {
            let (ifp, _) = fresh_ifnet(b"tst0");
            if_attach(ifp);
            ifps.push(ifp);
        }
        let net = NETLOCK.write();
        let held = &NetHeld::of(&net).expect("the net lock's witness");
        for (i, &ifp) in ifps.iter().enumerate() {
            let i = i as u8;
            addrs.push(address(ifp, [10, i, 0, 1], Some([255, 255, 255, 0]), None));
            addrs.push(address(ifp, [172, 16, i, 1], Some([255, 255, 0, 0]), None));
        }
        ifps[7].if_flags.set(IFF_POINTOPOINT);
        let p2p = address(ifps[7], [192, 168, 0, 1], None, Some([192, 168, 0, 2]));

        let found = |a: [u8; 4]| ifa_ifwithaddr(held, &bytes(sin(a)), 0);
        assert!(found([10, 5, 0, 1]).is_some_and(|f| core::ptr::eq(f, addrs[10])));
        assert!(found([172, 16, 3, 1]).is_some_and(|f| core::ptr::eq(f, addrs[7])));
        assert!(found([10, 5, 0, 2]).is_none());
        let dst = ifa_ifwithdstaddr(held, &bytes(sin([192, 168, 0, 2])), 0);
        assert!(dst.is_some_and(|f| core::ptr::eq(f, p2p)));
        assert!(ifa_ifwithdstaddr(held, &bytes(sin([10, 5, 0, 1])), 0).is_none());

        // ifp 2: 10.2.0.0/24 and 172.16.0.0/16; the first of the family otherwise.
        let best = |a: [u8; 4]| ifaof_ifpforaddr(held, &bytes(sin(a)), ifps[2]);
        assert!(best([10, 2, 0, 77]).is_some_and(|f| core::ptr::eq(f, addrs[4])));
        assert!(best([172, 16, 200, 9]).is_some_and(|f| core::ptr::eq(f, addrs[5])));
        assert!(best([8, 8, 8, 8]).is_some_and(|f| core::ptr::eq(f, addrs[4])));
        // A different domain sees none of them.
        ifps[5].if_rdomain.set(1);
        assert!(found([10, 5, 0, 1]).is_none());

        for (i, a) in addrs.iter().enumerate() {
            ifa_del(ifps[i / 2], a);
        }
        ifa_del(ifps[7], p2p);
        assert!(ifps.iter().all(|i| i.if_addrlist.is_empty()));
    }

    #[test]
    fn promiscuous_mode_reaches_the_driver_once_and_rolls_back_a_failure() {
        let _g = setup();
        let (ifp, d) = test_ifnet(b"tst0");
        let _net = NETLOCK.write();
        // Down: counted, the driver not asked.
        assert_eq!(ifpromisc(ifp, true), Ok(()));
        assert_eq!(
            (ifp.if_pcount.get(), d.ioctls.load(Ordering::Relaxed)),
            (1, 0)
        );
        assert_eq!(ifpromisc(ifp, false), Ok(()));
        if_up(ifp, &net_held(&_net));
        assert_eq!(ifpromisc(ifp, true), Ok(()));
        assert_eq!(ifpromisc(ifp, true), Ok(()));
        assert_eq!(d.ioctls.load(Ordering::Relaxed), 1);
        assert_ne!(ifp.if_flags.get() & IFF_PROMISC, 0);
        assert_eq!(
            d.last_flags.load(Ordering::Relaxed),
            u32::from((IFF_UP | IFF_PROMISC) as u16)
        );
        assert_eq!(ifpromisc(ifp, false), Ok(()));
        d.fail.store(true, Ordering::Relaxed);
        assert_eq!(ifpromisc(ifp, false), Err(Errno::EIO));
        assert_eq!(ifp.if_pcount.get(), 1);
        assert_ne!(ifp.if_flags.get() & IFF_PROMISC, 0);
        d.fail.store(false, Ordering::Relaxed);
        if_down(ifp, &net_held(&_net));
        assert_eq!(ifp.if_flags.get() & IFF_UP, 0);
    }

    #[test]
    fn the_data_counts_the_interface_and_its_queues() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tst0");
        if_counters_alloc(ifp);
        ifp.if_type.set(IFT_ETHER);
        ifp.if_mtu.set(1500);
        ifp.if_ipackets().set(3);
        counters_pkt(
            ifp_counters(ifp),
            IfCounters::IfcIpackets,
            IfCounters::IfcIbytes,
            100,
        );
        let mut data: IfData = hal::user::zeroed();
        if_getdata(ifp, &mut data);
        assert_eq!((data.ifi_type, data.ifi_mtu), (IFT_ETHER, 1500));
        assert_eq!((data.ifi_ipackets, data.ifi_ibytes), (4, 100));
        if_counters_free(ifp);
    }

    #[test]
    fn a_local_packet_without_its_protocol_is_refused_and_freed() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"lo0");
        if_counters_alloc(ifp);
        ifp.if_flags.set(IFF_LOOPBACK);
        assert_eq!(
            if_input_local(ifp, test_packet(0, 64), AF_INET, None),
            Err(Errno::EAFNOSUPPORT)
        );
        assert_eq!(
            ifp_counters(ifp)[IfCounters::IfcOpackets as usize].load(Ordering::Relaxed),
            1
        );
        assert_eq!(if_output_local(ifp, test_packet(0, 64), AF_INET), Ok(()));
        assert_eq!(crate::net::ifq::ifiq_len(ifp.if_rcv()), 1);
        with_smr_thread(|| crate::net::ifq::ifiq_destroy(ifp.if_rcv()));
        if_counters_free(ifp);
    }

    #[test]
    fn the_egress_group_follows_the_default_routes() {
        let _g = setup();
        let (ifp, _) = fresh_ifnet(b"tst0");
        if_attach(ifp);
        let _net = NETLOCK.write();
        assert_eq!(if_addgroup(ifp, IFG_EGRESS), Ok(()));
        // Not a default route: nothing changes.
        if_group_routechange(
            &bytes(sin([10, 0, 0, 0])),
            Some(&bytes(sin([255, 0, 0, 0]))),
        );
        assert!(group(IFG_EGRESS).is_some());
        // 0.0.0.0/0: built again, from no route (no routing registered).
        if_group_routechange(&bytes(sin([0; 4])), Some(&[0u8][..]));
        assert_eq!(group(IFG_EGRESS), None);
        assert_eq!(groups_of(ifp), [b"all".to_vec()]);
    }

    #[test]
    fn a_receive_ring_hands_out_slots_up_to_its_watermark() {
        let mut rxr: IfRxring = hal::user::zeroed();
        if_rxr_init(&mut rxr, 2, 8);
        assert_eq!((rxr.rxr_lwm, rxr.rxr_cwm, rxr.rxr_hwm), (2, 2, 8));
        assert_eq!(if_rxr_get(&mut rxr, 1), 1);
        assert_eq!(if_rxr_get(&mut rxr, 10), 1);
        assert_eq!(if_rxr_get(&mut rxr, 10), 0);
        crate::net::if_var::if_rxr_put(&mut rxr, 2);
        assert!(crate::net::if_var::if_rxr_needrefill(&rxr));
        if_congestion();
        assert!(if_congested());
        assert_eq!(&lo_name(12)[..5], b"lo12\0");
        assert_eq!(&lo_name(0)[..3], b"lo0");
    }

    #[test]
    fn ether_ifattach_writes_its_address_into_the_link_name() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"vio1");
        ifp.if_addrlen.set(ETHER_ADDR_LEN as u8);
        if_sadl_set_lladdr(ifp, &[1; 6]); // nothing before if_alloc_sadl
        if_alloc_sadl(ifp);
        if_sadl_set_lladdr(ifp, &[7, 6, 5, 4, 3, 2]);
        let sdl = ifp.sadl();
        let sdl = sdl.as_ref().expect("a link name");
        let bytes = hal::user::bytes_of(core::slice::from_ref(&**sdl));
        assert_eq!(SockaddrDl::lladdr(bytes), Some(&[7u8, 6, 5, 4, 3, 2][..]));
    }

    // The interface requests (`ifioctl`), over the UNIX domain's test glue: a root thread
    // (`unix::tests::setup`), the interface layer reset as `setup` does, interfaces with a
    // `TestDriver` attached by name, the user's memory the host double's.

    use hal::sys::net_if::{IFF_RUNNING, IfLaddrreq, IfParent, Ifaliasreq, Ifmediareq};
    use hal::sys::socket::{AF_UNIX, SOCK_DGRAM};
    use hal::sys::sockio::{SIOCGIFPARENT, SIOCGLIFPHYADDR};

    use crate::fs::testfs::TestGlue;

    /// The process glue of the request tests.
    type Tg = TestGlue;

    /// Where the request tests' user buffers are, in the host's user memory double: the
    /// arguments' range, which the tests holding the memory lock share in turn (the end
    /// range is `kern.msgbuf`'s copy's, whose tests run beside these:
    /// `hal::arch::host::copy`'s docs).
    const UBUF: usize = crate::testutil::USER_BASE + 0xc000;

    /// The request tests' fixture: a root thread, the mbufs and the socket pools (the UNIX
    /// domain's), and an empty interface layer.
    fn ioctl_setup() -> (MutexGuard<'static, ()>, &'static Thread<Tg>) {
        let (g, p) = crate::net::unix::tests::setup();
        softnet_test_install();
        if_idxmap_test_reset();
        ifnet_test_reset();
        // The routing tables of table 0 (the requests that make a routing domain).
        crate::net::rtable::rtable_test_reset();
        crate::net::route::rtlabel_test_reset();
        crate::net::route::route_init();
        crate::net::rtable::rtable_init_doms(&crate::net::route::tests::ROUTE_DOMS);
        (g, p)
    }

    /// `f` with `p` as a user that is not the superuser, then `p` root again.
    fn as_user<R>(p: &Thread<Tg>, f: impl FnOnce() -> R) -> R {
        let mut ids = crate::cred::Ucred::new();
        ids.cr_uid.set(1000);
        ids.cr_ruid.set(1000);
        let root = p.set_cred(crate::cred::CredRef::new(ids));
        let r = f();
        if let Some(root) = root {
            drop(p.set_cred(root));
        }
        r
    }

    /// An attached interface `name` with a counting driver, on `ifnetlist`.
    fn listed_ifnet(name: &[u8]) -> (&'static Ifnet, &'static TestDriver) {
        let (ifp, d) = fresh_ifnet(name);
        if_attach(ifp);
        (ifp, d)
    }

    /// A request structure `T` whose first bytes are the interface name `name`.
    fn named<T: hal::user::AbiPod>(name: &[u8]) -> T {
        let mut r: T = hal::user::zeroed();
        hal::user::bytes_of_mut(core::slice::from_mut(&mut r))[..name.len()].copy_from_slice(name);
        r
    }

    /// `ifioctl(NULL, cmd, req, p)` on the request structure `req` (the kernel's own request,
    /// no socket).
    fn ioctl<T: hal::user::AbiPod>(p: &Thread<Tg>, cmd: u64, req: &mut T) -> Result<(), Errno> {
        assert_eq!(iocparm_len(cmd) as usize, size_of::<T>(), "{cmd:#x}");
        ifioctl::<Tg>(
            None,
            cmd,
            hal::user::bytes_of_mut(core::slice::from_mut(req)),
            p,
        )
    }

    /// `b` copied to the user's memory at `at`.
    fn put_user(at: usize, b: &[u8]) {
        UserPtr::<u8>::new(at)
            .copyout_slice(b)
            .expect("user memory");
    }

    /// `n` bytes of the user's memory at `at`.
    fn get_user(at: usize, n: usize) -> Vec<u8> {
        let mut b = std::vec![0u8; n];
        UserPtr::<u8>::new(at)
            .copyin_slice(&mut b)
            .expect("user memory");
        b
    }

    #[test]
    fn every_request_that_changes_an_interface_asks_for_the_superuser() {
        let (_g, p) = ioctl_setup();
        let (ifp, d) = listed_ifnet(b"tsu0");
        test_cloner();
        let flags = ifp.if_flags.get();
        let set_requests = [
            SIOCSIFXFLAGS,
            SIOCSIFFLAGS,
            SIOCSIFMETRIC,
            SIOCSIFMTU,
            SIOCSIFDESCR,
            SIOCSIFRTLABEL,
            SIOCSIFPRIORITY,
            SIOCSIFRDOMAIN,
            SIOCAIFGROUP,
            SIOCDIFGROUP,
            SIOCSIFLLADDR,
            SIOCSIFLLPRIO,
            SIOCSIFMEDIA,
            // Before the protocol: the privileged list.
            SIOCADDMULTI,
            SIOCDELMULTI,
            SIOCSVNETID,
            SIOCSTXHPRIO,
            // After the protocol answered EOPNOTSUPP: the address requests.
            SIOCSIFADDR,
            SIOCSIFNETMASK,
            SIOCSIFDSTADDR,
            SIOCSIFBRDADDR,
        ];
        as_user(p, || {
            for cmd in set_requests {
                // The command's own length (an `ifreq`, an `ifgroupreq`): the name, then the
                // flags of an `ifreq`'s union.
                let mut words = [0u64; 8];
                let req = hal::user::bytes_of_mut(&mut words[..]);
                req[..4].copy_from_slice(b"tsu0");
                req[16..18].copy_from_slice(&(IFF_UP as i16).to_ne_bytes());
                let req = &mut req[..iocparm_len(cmd) as usize];
                assert_eq!(
                    ifioctl::<Tg>(None, cmd, req, p),
                    Err(Errno::EPERM),
                    "{cmd:#x}"
                );
            }
            let mut ifar: IfAfreq = named(b"tsu0");
            ifar.ifar_af = AF_INET;
            assert_eq!(ioctl(p, SIOCIFAFDETACH, &mut ifar), Err(Errno::EPERM));
            let mut ifra: Ifaliasreq = named(b"tsu0");
            assert_eq!(ioctl(p, SIOCAIFADDR, &mut ifra), Err(Errno::EPERM));
            let mut sff: IfSffpage = named(b"tsu0");
            sff.sff_addr = IFSFF_ADDR_EEPROM;
            assert_eq!(ioctl(p, SIOCGIFSFFPAGE, &mut sff), Err(Errno::EPERM));
            let mut ifgr: Ifgroupreq = named(b"all");
            assert_eq!(ioctl(p, SIOCSIFGATTR, &mut ifgr), Err(Errno::EPERM));
            // The cloner's requests check before they look for anything.
            let mut ifr: Ifreq = named(b"tcl5");
            assert_eq!(ioctl(p, SIOCIFCREATE, &mut ifr), Err(Errno::EPERM));
            let mut ifr: Ifreq = named(b"nosuch5");
            assert_eq!(ioctl(p, SIOCIFDESTROY, &mut ifr), Err(Errno::EPERM));
            // A per-interface request finds the interface first: ENXIO before EPERM.
            let mut ifr: Ifreq = named(b"nosuch0");
            assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Err(Errno::ENXIO));
            // The reads and the unprivileged requests need nothing.
            let mut ifr: Ifreq = named(b"tsu0");
            assert_eq!(ioctl(p, SIOCGIFFLAGS, &mut ifr), Ok(()));
            let mut ifmr: Ifmediareq = named(b"tsu0");
            assert_eq!(ioctl(p, SIOCGIFMEDIA, &mut ifmr), Ok(()));
            let mut parent: IfParent = named(b"tsu0");
            assert_eq!(ioctl(p, SIOCGIFPARENT, &mut parent), Ok(()));
        });
        // Nothing changed, and the driver saw only the two unprivileged requests.
        assert_eq!(ifp.if_flags.get(), flags);
        assert_eq!(d.ioctls.load(Ordering::Relaxed), 2);
        assert!(if_unit_test(b"tcl5").is_none());
    }

    /// `if_unit(name)` under the kernel lock, its reference dropped.
    fn if_unit_test(name: &[u8]) -> Option<&'static Ifnet> {
        hal::klock::kernel_lock();
        let ifp = if_unit(name);
        hal::klock::kernel_unlock();
        if_put(ifp);
        ifp
    }

    /// A driver whose requests let the net lock go and take it back, as wg(4)'s `wg_down`,
    /// pflow(4)'s `pflowioctl` and pfsync(4)'s `pfsync_down` do; while it has let it go, it
    /// takes the interface after it off `ifnetlist` when asked (another thread's detach).
    struct DropsNetLock {
        detach_next: core::sync::atomic::AtomicBool,
    }

    impl NetDriver for DropsNetLock {
        fn ioctl(&self, ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            hal::netlock::unlock();
            if self.detach_next.swap(false, Ordering::Relaxed)
                && let Some(next) = List::<IfnetList>::next(ifp)
            {
                let _net = NETLOCK.write();
                IFNETLIST.remove(next);
            }
            hal::netlock::lock();
            Ok(())
        }
    }

    #[test]
    fn a_driver_that_lets_the_net_lock_go_runs_under_ifioctl_and_if_downall() {
        static D: DropsNetLock = DropsNetLock {
            detach_next: core::sync::atomic::AtomicBool::new(false),
        };
        let (_g, p) = ioctl_setup();
        let ifp = test_ifnet_fresh(b"tdl0", &D);
        if_attach(ifp);
        let (gone, _) = listed_ifnet(b"tdl1");
        let (last, _) = listed_ifnet(b"tdl2");
        for i in [ifp, gone, last] {
            i.if_flags.set(IFF_UP);
        }
        let mut ifr: Ifreq = named(b"tdl0");
        ifr.set_ifr_flags(IFF_UP as i16);
        assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Ok(()));
        assert_ne!(ifp.if_flags.get() & IFF_UP, 0);
        // Shutdown: each interface marked down and told, the lock its handler let go free;
        // tdl1, taken off the list while tdl0's handler had let the lock go, is not visited
        // and the walk goes on to tdl2 (the C reads the next one after the body).
        D.detach_next.store(true, Ordering::Relaxed);
        if_downall();
        assert_eq!(ifp.if_flags.get() & IFF_UP, 0);
        assert_ne!(gone.if_flags.get() & IFF_UP, 0);
        assert_eq!(last.if_flags.get() & IFF_UP, 0);
        assert_eq!(hal::netlock::status(), 0);
    }

    #[test]
    fn the_reads_answer_the_interface_s_values() {
        let (_g, p) = ioctl_setup();
        let (ifp, _) = listed_ifnet(b"trd0");
        ifp.if_flags.set(IFF_UP | IFF_LOOPBACK);
        ifp.if_xflags
            .set(IFXF_MPSAFE | IFXF_CLONED | IFXF_AUTOCONF4);
        ifp.if_metric.set(7);
        ifp.if_mtu.set(1500);
        ifp.if_hardmtu.set(9000);
        ifp.if_priority.set(3);
        ifp.if_llprio.set(5);
        let get = |cmd| {
            let mut ifr: Ifreq = named(b"trd0");
            assert_eq!(ioctl(p, cmd, &mut ifr), Ok(()), "{cmd:#x}");
            ifr
        };
        assert_eq!(
            i32::from(get(SIOCGIFFLAGS).ifr_flags()),
            IFF_UP | IFF_LOOPBACK
        );
        // IFXF_MPSAFE and IFXF_CLONED are the kernel's.
        assert_eq!(i32::from(get(SIOCGIFXFLAGS).ifr_flags()), IFXF_AUTOCONF4);
        assert_eq!(get(SIOCGIFMETRIC).ifr_metric(), 7);
        assert_eq!(get(SIOCGIFMTU).ifr_mtu(), 1500);
        assert_eq!(get(SIOCGIFHARDMTU).ifr_hardmtu(), 9000);
        assert_eq!(get(SIOCGIFPRIORITY).ifr_metric(), 3);
        assert_eq!(get(SIOCGIFRDOMAIN).ifr_rdomainid(), 0);
        assert_eq!(get(SIOCGIFLLPRIO).ifr_llprio(), 5);
        // The send queue's oactive shows in the flags.
        crate::net::ifq::ifq_set_oactive(ifp.if_snd());
        assert_eq!(
            i32::from(get(SIOCGIFFLAGS).ifr_flags()),
            IFF_UP | IFF_LOOPBACK | IFF_OACTIVE
        );
        crate::net::ifq::ifq_clr_oactive(ifp.if_snd());

        // if_data, copied out whole.
        let mut ifr: Ifreq = named(b"trd0");
        ifr.set_ifr_data(UBUF as u64);
        assert_eq!(ioctl(p, SIOCGIFDATA, &mut ifr), Ok(()));
        let data: IfData = UserPtr::<IfData>::new(UBUF).copyin().expect("if_data");
        assert_eq!((data.ifi_mtu, data.ifi_metric), (1500, 7));

        // A description round trip; a label without routing is none.
        put_user(UBUF, b"uplink\0");
        let mut ifr: Ifreq = named(b"trd0");
        ifr.set_ifr_data(UBUF as u64);
        assert_eq!(ioctl(p, SIOCSIFDESCR, &mut ifr), Ok(()));
        ifr.set_ifr_data((UBUF + 0x100) as u64);
        assert_eq!(ioctl(p, SIOCGIFDESCR, &mut ifr), Ok(()));
        assert_eq!(get_user(UBUF + 0x100, 7), b"uplink\0");
        assert_eq!(ioctl(p, SIOCGIFRTLABEL, &mut ifr), Err(Errno::ENOENT));
        // A description of 64 bytes or more without its NUL: ENAMETOOLONG, the old one kept.
        put_user(UBUF, &[b'x'; 70]);
        let mut ifr: Ifreq = named(b"trd0");
        ifr.set_ifr_data(UBUF as u64);
        assert_eq!(ioctl(p, SIOCSIFDESCR, &mut ifr), Err(Errno::ENAMETOOLONG));
        assert_eq!(&ifp.if_description.get()[..7], b"uplink\0");

        let mut ifr: Ifreq = named(b"nosuch0");
        assert_eq!(ioctl(p, SIOCGIFFLAGS, &mut ifr), Err(Errno::ENXIO));
    }

    #[test]
    fn flags_are_set_through_the_driver_and_put_back_when_it_refuses() {
        let (_g, p) = ioctl_setup();
        let (ifp, d) = listed_ifnet(b"tfl0");
        ifp.if_flags.set(IFF_RUNNING);

        // IFF_RUNNING is the driver's; the request's IFF_UP is set and brings it up.
        let mut ifr: Ifreq = named(b"tfl0");
        ifr.set_ifr_flags((IFF_UP | IFF_PROMISC) as i16);
        assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Ok(()));
        assert_eq!(ifp.if_flags.get(), IFF_UP | IFF_PROMISC | IFF_RUNNING);
        assert_eq!(
            d.last_flags.load(Ordering::Relaxed) as i32,
            IFF_UP | IFF_PROMISC
        );

        // A short with its sign bit set: the flags keep their low sixteen bits.
        let mut ifr: Ifreq = named(b"tfl0");
        ifr.set_ifr_flags(-1);
        assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Ok(()));
        assert_eq!(ifp.if_flags.get() & !0xffff, 0);

        // The driver refuses: the old flags come back.
        let before = ifp.if_flags.get();
        d.fail.store(true, Ordering::Relaxed);
        let mut ifr: Ifreq = named(b"tfl0");
        assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Err(Errno::EIO));
        assert_eq!(ifp.if_flags.get(), before);
        d.fail.store(false, Ordering::Relaxed);

        // Down; then the extra flags: IFXF_MPSAFE is the kernel's, wake on LAN without the
        // capability is refused and taken out of the request.
        let mut ifr: Ifreq = named(b"tfl0");
        assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Ok(()));
        assert_eq!(ifp.if_flags.get() & IFF_UP, 0);
        let mut ifr: Ifreq = named(b"tfl0");
        ifr.set_ifr_flags(IFXF_WOL as i16);
        assert_eq!(ioctl(p, SIOCSIFXFLAGS, &mut ifr), Err(Errno::ENOTSUP));
        assert_eq!(i32::from(ifr.ifr_flags()), 0);
        assert_eq!(ifp.if_xflags.get(), IFXF_MPSAFE);

        // Automatic configuration turned on while down: forced up through SIOCSIFFLAGS.
        let mut ifr: Ifreq = named(b"tfl0");
        ifr.set_ifr_flags(IFXF_AUTOCONF4 as i16);
        assert_eq!(ioctl(p, SIOCSIFXFLAGS, &mut ifr), Ok(()));
        assert_eq!(ifp.if_xflags.get(), IFXF_MPSAFE | IFXF_AUTOCONF4);
        assert_ne!(ifp.if_flags.get() & IFF_UP, 0);
        assert_ne!(d.last_flags.load(Ordering::Relaxed) as i32 & IFF_UP, 0);

        // Down again; a refused forceup puts both the flags and the extra flags back.
        let mut ifr: Ifreq = named(b"tfl0");
        assert_eq!(ioctl(p, SIOCSIFFLAGS, &mut ifr), Ok(()));
        let mut ifr: Ifreq = named(b"tfl0");
        assert_eq!(ioctl(p, SIOCSIFXFLAGS, &mut ifr), Ok(()));
        let old = (ifp.if_flags.get(), ifp.if_xflags.get());
        d.fail.store(true, Ordering::Relaxed);
        let mut ifr: Ifreq = named(b"tfl0");
        ifr.set_ifr_flags(IFXF_AUTOCONF4 as i16);
        assert_eq!(ioctl(p, SIOCSIFXFLAGS, &mut ifr), Err(Errno::EIO));
        assert_eq!((ifp.if_flags.get(), ifp.if_xflags.get()), old);
        d.fail.store(false, Ordering::Relaxed);
    }

    #[test]
    fn each_set_request_checks_its_value_as_the_c_does() {
        let (_g, p) = ioctl_setup();
        let (ifp, d) = listed_ifnet(b"tva0");

        let mut ifr: Ifreq = named(b"tva0");
        ifr.set_ifr_metric(16);
        assert_eq!(ioctl(p, SIOCSIFPRIORITY, &mut ifr), Err(Errno::EINVAL));
        ifr.set_ifr_metric(15);
        assert_eq!(ioctl(p, SIOCSIFPRIORITY, &mut ifr), Ok(()));
        assert_eq!(ifp.if_priority.get(), 15);
        ifr.set_ifr_metric(-1);
        assert_eq!(ioctl(p, SIOCSIFPRIORITY, &mut ifr), Err(Errno::EINVAL));

        let mut ifr: Ifreq = named(b"tva0");
        ifr.set_ifr_llprio(IFQ_MAXPRIO as i32 + 1);
        assert_eq!(ioctl(p, SIOCSIFLLPRIO, &mut ifr), Err(Errno::EINVAL));
        ifr.set_ifr_llprio(IFQ_MAXPRIO as i32);
        assert_eq!(ioctl(p, SIOCSIFLLPRIO, &mut ifr), Ok(()));
        assert_eq!(u32::from(ifp.if_llprio.get()), IFQ_MAXPRIO);

        let mut ifr: Ifreq = named(b"tva0");
        ifr.set_ifr_metric(9);
        assert_eq!(ioctl(p, SIOCSIFMETRIC, &mut ifr), Ok(()));
        assert_eq!(ifp.if_metric.get(), 9);

        // A link-level address: none without a link name, a multicast one or one of another
        // length refused, an interface that is no Ethernet ENODEV.
        let lladdr = |a: [u8; 6], len: u8| {
            let mut ifr: Ifreq = named(b"tva0");
            let mut sa: Sockaddr = hal::user::zeroed();
            sa.sa_len = len;
            sa.sa_family = AF_LINK;
            sa.sa_data[..6].copy_from_slice(&a);
            ifr.set_ifr_addr(&sa);
            ioctl(p, SIOCSIFLLADDR, &mut ifr)
        };
        assert_eq!(lladdr([2, 0, 0, 0, 0, 1], 6), Err(Errno::EINVAL));
        ifp.if_addrlen.set(ETHER_ADDR_LEN as u8);
        if_alloc_sadl(ifp);
        assert_eq!(lladdr([3, 0, 0, 0, 0, 1], 6), Err(Errno::EINVAL));
        assert_eq!(lladdr([2, 0, 0, 0, 0, 1], 5), Err(Errno::EINVAL));
        assert_eq!(lladdr([2, 0, 0, 0, 0, 1], 6), Err(Errno::ENODEV));
        ifp.if_type.set(IFT_ETHER);
        assert_eq!(lladdr([2, 0, 0, 0, 0, 1], 6), Ok(()));
        {
            let sdl = ifp.sadl();
            let sdl = sdl.as_ref().expect("a link name");
            let bytes = hal::user::bytes_of(core::slice::from_ref(&**sdl));
            assert_eq!(SockaddrDl::lladdr(bytes), Some(&[2u8, 0, 0, 0, 0, 1][..]));
        }

        // A transceiver page names the EEPROM or the diagnostics.
        let mut sff: IfSffpage = named(b"tva0");
        sff.sff_addr = 0x50;
        assert_eq!(ioctl(p, SIOCGIFSFFPAGE, &mut sff), Err(Errno::EINVAL));
        sff.sff_addr = IFSFF_ADDR_DDM;
        let asked = d.ioctls.load(Ordering::Relaxed);
        assert_eq!(ioctl(p, SIOCGIFSFFPAGE, &mut sff), Ok(()));
        assert_eq!(d.ioctls.load(Ordering::Relaxed), asked + 1);

        // A family: AF_INET's attach is nothing; no IPv6 without the protocol; others none.
        let af = |cmd, af| {
            let mut ifar: IfAfreq = named(b"tva0");
            ifar.ifar_af = af;
            ioctl(p, cmd, &mut ifar)
        };
        assert_eq!(af(SIOCIFAFATTACH, AF_INET), Ok(()));
        assert_eq!(af(SIOCIFAFDETACH, AF_INET), Ok(()));
        assert_eq!(af(SIOCIFAFATTACH, AF_INET6), Err(Errno::EAFNOSUPPORT));
        assert_eq!(af(SIOCIFAFATTACH, AF_LINK), Err(Errno::EAFNOSUPPORT));

        // Routing domain 1: its table is made, but not its loopback lo1 (no lo(4) cloner
        // here), so if_createrdomain answers ENXIO, the C's.
        let mut ifr: Ifreq = named(b"tva0");
        ifr.set_ifr_rdomainid(1);
        assert_eq!(ioctl(p, SIOCSIFRDOMAIN, &mut ifr), Err(Errno::ENXIO));
        assert!(crate::net::rtable::rtable_exists(1));

        // A request no protocol answers goes to the driver (the UNIX domain's socket has no
        // `pru_control`; the kernel's own request has no socket).
        let so =
            crate::net::socket::socreate(p, i32::from(AF_UNIX), SOCK_DGRAM, 0).expect("socreate");
        let asked = d.ioctls.load(Ordering::Relaxed);
        let mut ifr: Ifreq = named(b"tva0");
        let data = hal::user::bytes_of_mut(core::slice::from_mut(&mut ifr));
        assert_eq!(ifioctl::<Tg>(Some(&so), SIOCSIFADDR, data, p), Ok(()));
        assert_eq!(d.ioctls.load(Ordering::Relaxed), asked + 1);
        assert_eq!(crate::net::socket::soclose(&so, 0), Ok(()));
    }

    #[test]
    fn the_group_requests_add_list_count_and_remove() {
        let (_g, p) = ioctl_setup();
        let (a, da) = listed_ifnet(b"tgr0");
        let (b, _) = listed_ifnet(b"tgr1");

        let group = |cmd, name: &[u8], g: &[u8]| {
            let mut ifgr: Ifgroupreq = named(name);
            ifgr.ifgr_group_mut()[..g.len()].copy_from_slice(g);
            ioctl(p, cmd, &mut ifgr)
        };
        assert_eq!(group(SIOCAIFGROUP, b"tgr0", b"lan"), Ok(()));
        assert_eq!(group(SIOCAIFGROUP, b"tgr1", b"lan"), Ok(()));
        assert_eq!(group(SIOCAIFGROUP, b"tgr0", b"lan"), Err(Errno::EEXIST));
        assert_eq!(groups_of(a), [b"all".to_vec(), b"lan".to_vec()]);

        // The rows: `ifgr_len` 0 asks for the length, a short buffer is EINVAL after the
        // rows that fit.
        let rows = |cmd, name: &[u8], len: u32| {
            let mut ifgr: Ifgroupreq = named(name);
            ifgr.ifgr_len = len;
            ifgr.ifgr_ifgru[..8].copy_from_slice(&(UBUF as u64).to_ne_bytes());
            put_user(UBUF, &[0xaa; 64]);
            let r = ioctl(p, cmd, &mut ifgr);
            (r, ifgr.ifgr_len, get_user(UBUF, 64))
        };
        let (r, len, _) = rows(SIOCGIFGROUP, b"tgr0", 0);
        assert_eq!((r, len), (Ok(()), 2 * IFG_REQ_SIZE as u32));
        let (r, _, user) = rows(SIOCGIFGROUP, b"tgr0", 32);
        assert_eq!(r, Ok(()));
        assert_eq!(&user[..16], b"all\0\0\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(&user[16..20], b"lan\0");
        let (r, _, user) = rows(SIOCGIFGROUP, b"tgr0", 20);
        assert_eq!(r, Err(Errno::EINVAL));
        assert_eq!((&user[..3], user[16]), (&b"all"[..], 0xaa));
        let (r, len, _) = rows(SIOCGIFGMEMB, b"lan", 0);
        assert_eq!((r, len), (Ok(()), 2 * IFG_REQ_SIZE as u32));
        let (r, _, user) = rows(SIOCGIFGMEMB, b"lan", 32);
        assert_eq!(r, Ok(()));
        assert_eq!(
            (&user[..5], &user[16..21]),
            (&b"tgr0\0"[..], &b"tgr1\0"[..])
        );
        assert_eq!(rows(SIOCGIFGMEMB, b"none", 32).0, Err(Errno::ENOENT));
        let (r, len, _) = rows(SIOCGIFGLIST, b"", 0);
        assert_eq!((r, len), (Ok(()), 2 * IFG_REQ_SIZE as u32));
        let (r, _, user) = rows(SIOCGIFGLIST, b"", 32);
        assert_eq!(
            (r, &user[..3], &user[16..19]),
            (Ok(()), &b"all"[..], &b"lan"[..])
        );

        // The demotion counter: read, moved, bounded, each member's driver told.
        let attrib = |cmd, demote: i32| {
            let mut ifgr: Ifgroupreq = named(b"lan");
            ifgr.set_ifgr_attrib(IfgAttrib {
                ifg_carp_demoted: demote,
            });
            let r = ioctl(p, cmd, &mut ifgr);
            (r, ifgr.ifgr_attrib().ifg_carp_demoted)
        };
        let told = da.ioctls.load(Ordering::Relaxed);
        assert_eq!(attrib(SIOCSIFGATTR, 5).0, Ok(()));
        assert_eq!(da.ioctls.load(Ordering::Relaxed), told + 1);
        assert_eq!(attrib(SIOCGIFGATTR, 0), (Ok(()), 5));
        assert_eq!(attrib(SIOCSIFGATTR, 251).0, Err(Errno::EINVAL));
        assert_eq!(attrib(SIOCSIFGATTR, -6).0, Err(Errno::EINVAL));
        assert_eq!(attrib(SIOCSIFGATTR, -5).0, Ok(()));
        let mut ifgr: Ifgroupreq = named(b"none");
        assert_eq!(ioctl(p, SIOCGIFGATTR, &mut ifgr), Err(Errno::ENOENT));

        assert_eq!(group(SIOCDIFGROUP, b"tgr0", b"lan"), Ok(()));
        assert_eq!(group(SIOCDIFGROUP, b"tgr0", b"lan"), Err(Errno::ENOENT));
        assert_eq!(groups_of(b), [b"all".to_vec(), b"lan".to_vec()]);
        assert_eq!(group(SIOCDIFGROUP, b"tgr1", b"lan"), Ok(()));
        assert!(ifg_find(b"lan").is_none());
    }

    /// An address of `ifp` whose storage is the `N` bytes `sa` (an `in_ifaddr`'s or an
    /// `in6_ifaddr`'s, as `bsd` publishes it).
    fn address_of<const N: usize>(ifp: &'static Ifnet, sa: [u8; N]) -> &'static Ifaddr {
        let ifa: &'static Ifaddr = Box::leak(Box::new(Ifaddr::new()));
        let storage: &'static core::cell::Cell<[u8; N]> =
            Box::leak(Box::new(core::cell::Cell::new(sa)));
        ifa.ifa_addr
            .set(crate::net::if_var::sa_test_token(), Some(storage));
        ifa.ifa_ifp.set(Some(ifp));
        let _net = NETLOCK.write();
        ifa_add(ifp, ifa);
        ifa
    }

    /// `ifconf` of `len` bytes into the user's buffer: its result, the `ifc_len` it leaves and
    /// the buffer's first 200 bytes.
    fn ifconf_of(p: &Thread<Tg>, len: i32) -> (Result<(), Errno>, i32, Vec<u8>) {
        put_user(UBUF, &[0xaa; 200]);
        let mut ifc = Ifconf {
            ifc_len: len,
            _pad0: 0,
            ifc_ifcu: UBUF as u64,
        };
        let r = ioctl(p, SIOCGIFCONF, &mut ifc);
        (r, ifc.ifc_len, get_user(UBUF, 200))
    }

    #[test]
    fn ifconf_answers_rows_of_every_address_whatever_the_buffer() {
        let (_g, p) = ioctl_setup();
        let (a, _) = listed_ifnet(b"tcf0");
        let (b, _) = listed_ifnet(b"tcf1");
        let mut sin = [0u8; 16];
        sin[..2].copy_from_slice(&[16, AF_INET]);
        sin[4..8].copy_from_slice(&[10, 0, 0, 1]);
        address_of(b, sin);
        let mut sin6 = [0u8; 28];
        sin6[..2].copy_from_slice(&[28, AF_INET6]);
        sin6[8] = 0xfe;
        address_of(b, sin6);

        // The rows: tcf0 without an address (32), tcf1's two (32, and 16 + 28).
        let need = 32 + 32 + 16 + 28;
        assert_eq!(ifconf_of(p, 0).1, need);
        let (r, len, user) = ifconf_of(p, 200);
        assert_eq!((r, len), (Ok(()), need));
        assert_eq!(&user[..5], b"tcf0\0");
        assert_eq!(&user[16..32], &[0; 16]);
        assert_eq!(&user[32..37], b"tcf1\0");
        assert_eq!(&user[48..56], &sin[..8]);
        assert_eq!(&user[64..69], b"tcf1\0");
        assert_eq!(&user[80..108], &sin6[..]);
        assert_eq!(user[108], 0xaa);

        // A short buffer: whole rows only, `ifc_len` the bytes written.
        let (r, len, user) = ifconf_of(p, 40);
        assert_eq!((r, len), (Ok(()), 32));
        assert_eq!(user[32], 0xaa);
        // The long row needs its whole length (an `int` comparison).
        let (r, len, user) = ifconf_of(p, 64 + 43);
        assert_eq!((r, len), (Ok(()), 64));
        assert_eq!(user[64], 0xaa);
        // A negative length bounds no `sockaddr`-sized row (the C's `size_t` comparison) but
        // bounds the long ones (an `int`).
        let (r, len, user) = ifconf_of(p, -1);
        assert_eq!((r, len), (Ok(()), 64));
        assert_eq!(&user[32..37], b"tcf1\0");
        assert_eq!(user[64], 0xaa);
        let _ = a;
    }

    #[test]
    fn ifconf_stops_at_a_fault_and_drops_every_reference() {
        let (_g, p) = ioctl_setup();
        let (ifp, _) = listed_ifnet(b"tcx0");
        let addrs: Vec<&'static Ifaddr> = (0..3u8)
            .map(|i| {
                let mut sin = [0u8; 16];
                sin[..2].copy_from_slice(&[16, AF_INET]);
                sin[4..8].copy_from_slice(&[10, 0, 0, i]);
                address_of(ifp, sin)
            })
            .collect();
        let if_refs = ifp.rt().if_refcnt.read();
        // The interface's first row fits just before the end of the user's memory and its
        // second crosses it: that copy is refused whole (EFAULT), partway through the
        // interface's addresses. The first row lands in the end slots, whose tests read back
        // nothing that depends on them (`hal::arch::host::copy`'s docs: uio's array faults
        // whatever it holds).
        let at = crate::testutil::USER_BASE + crate::testutil::USER_LEN - 48;
        let mut ifc = Ifconf {
            ifc_len: 0x100,
            _pad0: 0,
            ifc_ifcu: at as u64,
        };
        assert_eq!(ioctl(p, SIOCGIFCONF, &mut ifc), Err(Errno::EFAULT));
        // `ifc_len` stays the caller's on an error, as the C's.
        assert_eq!(ifc.ifc_len, 0x100);
        // Every reference `ifconf` took is dropped on its way out: the addresses' and the
        // interface's.
        assert!(addrs.iter().all(|a| a.ifa_refcnt.read() == 1));
        assert_eq!(ifp.rt().if_refcnt.read(), if_refs);
    }

    #[test]
    fn ifconf_copies_an_interface_s_addresses_past_one_batch() {
        let (_g, p) = ioctl_setup();
        let (ifp, _) = listed_ifnet(b"tcb0");
        let n = IFCONF_BATCH + 3;
        for i in 0..n {
            let mut sin = [0u8; 16];
            sin[..2].copy_from_slice(&[16, AF_INET]);
            sin[4..8].copy_from_slice(&[10, 0, 0, i as u8]);
            address_of(ifp, sin);
        }
        put_user(UBUF, &[0xaa; 0x200]);
        let mut ifc = Ifconf {
            ifc_len: 0x200,
            _pad0: 0,
            ifc_ifcu: UBUF as u64,
        };
        assert_eq!(ioctl(p, SIOCGIFCONF, &mut ifc), Ok(()));
        assert_eq!(ifc.ifc_len as usize, n * IFREQ_SIZE);
        let user = get_user(UBUF, n * IFREQ_SIZE);
        for i in 0..n {
            let row = &user[i * IFREQ_SIZE..][..IFREQ_SIZE];
            assert_eq!(&row[..5], b"tcb0\0", "row {i}");
            assert_eq!(&row[20..24], &[10, 0, 0, i as u8], "row {i}");
        }
        // Every reference ifconf took is dropped again.
        assert!(ifp.if_addrlist.iter().all(|ifa| ifa.ifa_refcnt.read() == 1));
    }

    #[test]
    fn the_cloners_are_listed_by_name() {
        let (_g, p) = ioctl_setup();
        test_cloner();
        let total = if_clone_count();
        let mut ifcr = IfClonereq::default();
        assert_eq!(ioctl(p, SIOCIFGCLONERS, &mut ifcr), Ok(()));
        assert_eq!(ifcr.ifcr_total, total);
        ifcr.ifcr_buffer = UBUF as u64;
        ifcr.ifcr_count = -1;
        assert_eq!(ioctl(p, SIOCIFGCLONERS, &mut ifcr), Err(Errno::EINVAL));
        put_user(UBUF, &[0xaa; 64]);
        ifcr.ifcr_count = 1;
        assert_eq!(ioctl(p, SIOCIFGCLONERS, &mut ifcr), Ok(()));
        let user = get_user(UBUF, 32);
        let first = if_cloners().next().expect("a cloner").name();
        assert_eq!(&user[..first.len()], first);
        assert!(user[first.len()..16].iter().all(|&c| c == 0));
        assert_eq!(user[16], 0xaa);
    }

    #[test]
    fn a_request_is_worked_on_whole_however_the_caller_lays_it_out() {
        let (_g, p) = ioctl_setup();
        // A request longer than STK_PARAMS (536 bytes, sys_ioctl's heap buffer), aligned and
        // not, and a short one: each found the interface missing by its whole name.
        let need = ifioctl_need(SIOCGLIFPHYADDR);
        assert_eq!(need, size_of::<IfLaddrreq>());
        let mut words = std::vec![0u64; need.div_ceil(8) + 1];
        let bytes = hal::user::bytes_of_mut(&mut words[..]);
        for at in [0usize, 1] {
            let data = &mut bytes[at..at + need];
            data[..8].copy_from_slice(b"nosuch0\0");
            data[need - 1] = 0x5a;
            assert_eq!(
                ifioctl::<Tg>(None, SIOCGLIFPHYADDR, data, p),
                Err(Errno::ENXIO),
                "at {at}"
            );
            assert_eq!((&data[..8], data[need - 1]), (&b"nosuch0\0"[..], 0x5a));
        }
        // A void command's 8 bytes: read as an ifreq of a zeroed copy (no name: none found).
        let mut word = [0u8; 8];
        let void = hal::sys::ioccom::_io(b'i', 200);
        assert_eq!(ifioctl::<Tg>(None, void, &mut word, p), Err(Errno::ENXIO));
        // A short, misaligned request in place of an if_afreq still reaches its interface.
        let (_ifp, _) = listed_ifnet(b"twh0");
        let mut b = [0u8; 18];
        b[1..5].copy_from_slice(b"twh0");
        b[1 + 16] = AF_INET;
        assert_eq!(ifioctl::<Tg>(None, SIOCIFAFATTACH, &mut b[1..], p), Ok(()));
        assert!(
            with_request(&mut b[1..], 64, |r| {
                assert!(r.len() >= 64 && r.as_ptr().addr().is_multiple_of(8));
                Ok(())
            })
            .is_ok()
        );
    }

    #[test]
    fn header_priority_checks() {
        assert_eq!(if_txhprio_l2_check(IF_HDRPRIO_PACKET), Ok(()));
        assert_eq!(if_txhprio_l2_check(IF_HDRPRIO_PAYLOAD), Err(Errno::EINVAL));
        assert_eq!(if_txhprio_l3_check(IF_HDRPRIO_PAYLOAD), Ok(()));
        assert_eq!(if_rxhprio_l2_check(IF_HDRPRIO_OUTER), Ok(()));
        assert_eq!(if_rxhprio_l3_check(IF_HDRPRIO_OUTER), Ok(()));
        assert_eq!(if_txhprio_l3_check(IF_HDRPRIO_OUTER), Err(Errno::EINVAL));
        for prio in IF_HDRPRIO_MIN..=IF_HDRPRIO_MAX {
            assert_eq!(if_rxhprio_l3_check(prio), Ok(()));
        }
        assert_eq!(if_txhprio_l2_check(IF_HDRPRIO_MAX + 1), Err(Errno::EINVAL));
    }

    #[test]
    fn a_receive_ring_s_info_is_copied_out_up_to_the_user_s_room() {
        let (_g, _p) = ioctl_setup();
        let mut rxr: IfRxring = hal::user::zeroed();
        if_rxr_init(&mut rxr, 2, 8);
        let info = UBUF + 0x100;
        let ifri = |total: u32| IfRxrinfo {
            ifri_total: total,
            _pad0: 0,
            ifri_entries: info as u64,
        };
        put_user(info, &[0xaa; 2 * size_of::<IfRxringInfo>()]);
        UserPtr::<IfRxrinfo>::new(UBUF)
            .copyout(&ifri(4))
            .expect("user");
        assert_eq!(if_rxr_ioctl(UBUF, Some(b"rx"), 2048, &rxr), Ok(()));
        let k = UserPtr::<IfRxrinfo>::new(UBUF).copyin().expect("user");
        assert_eq!(k.ifri_total, 1);
        let e = UserPtr::<IfRxringInfo>::new(info).copyin().expect("user");
        assert_eq!(
            (&e.ifr_name[..3], e.ifr_size, e.ifr_info),
            (&b"rx\0"[..], 2048, rxr)
        );
        // No room: the total only.
        put_user(info, &[0xaa; size_of::<IfRxringInfo>()]);
        UserPtr::<IfRxrinfo>::new(UBUF)
            .copyout(&ifri(0))
            .expect("user");
        assert_eq!(if_rxr_ioctl(UBUF, None, 2048, &rxr), Ok(()));
        assert_eq!(get_user(info, 1), [0xaa]);
        assert_eq!(
            UserPtr::<IfRxrinfo>::new(UBUF)
                .copyin()
                .expect("user")
                .ifri_total,
            1
        );
    }

    #[test]
    fn the_inet6_request_constants_are_the_header_s() {
        assert_eq!(SIOCAIFADDR_IN6, 0x8080_691a);
        assert_eq!(SIOCDIFADDR_IN6, 0x8120_6919);
    }
}
/* </TESTS> */
