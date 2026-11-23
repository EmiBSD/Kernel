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
 *	@(#)in_pcb.h	8.1 (Berkeley) 6/10/93
 */

/*
 * Copyright (c) 1982, 1986, 1991, 1993
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
//! Internet protocol control blocks: `<netinet/in_pcb.h>` and `netinet/in_pcb.c`.
//!
//! Upstream: sys/netinet/in_pcb.h @ 3ce1f3f79392
//! Upstream: sys/netinet/in_pcb.c @ 3ce1f3f79392
//! LZ: sys/netinet/in_pcb.rs@3c62ede68802
//!
//! Every UDP, TCP and raw IP socket has an [`Inpcb`]: the local and foreign addresses and
//! ports, the cached route, the IP header prototype and the socket options of the IP layer.
//! The control blocks of one protocol live in an [`Inpcbtable`]: a queue of all of them and
//! two SipHash tables, one on the complete address quadruple ([`in_pcblookup`], connected
//! sockets) and one on the local port ([`in_pcblookup_local`], bind(2)'s checks and the
//! wildcard matches). [`in_pcblookup_listen`] finds a socket bound to the local address or the
//! wildcard address. Raw IP and UDP input walk the queue with an iterator that marks its place
//! ([`in_pcb_iterator`]), so the table mutex can be dropped while a packet is appended to a
//! socket.
//!
//! The pcb table mutex guarantees that all inpcb are consistent and that bind(2) and
//! connect(2) create unique combinations of laddr/faddr/lport/fport/rtableid. It protects both
//! address consistency and inpcb lookup during protocol input. All writes to
//! `inp_[lf]addr` take the table mutex. A per socket lock is needed so that the socket layer
//! input has a consistent view of these values. In `soconnect()` and `sosend()` a per pcb
//! mutex cannot be used: they eventually call IP output, which takes sleeping locks, and
//! connect(2) does a route lookup for source selection. Protocol input should not switch
//! process per packet, so it spins on the mutex. So there are three locks: the table mutex
//! for writing `inp_[lf]addr/port` and lookup, the socket rw-lock to separate sockets in
//! system calls, and the socket buffer mutex for the receive buffer. Changing
//! `inp_[lf]addr/port` takes both the per socket rw-lock and the global table mutex;
//! protocol input only reads them during lookup.
//!
//! Locks used to protect struct members: \[I\] immutable after creation, \[N\] net lock,
//! \[t\] `inpt_mtx` (pcb table mutex), \[s\] `so_lock` (socket rwlock), \[a\] atomic,
//! \[L\] `pf_inp_mtx` (pf's).
//!
//! ## Deviations
//! - The C's unions (`inp_faddru`, `inp_laddru`, the header prototype `inp_hu`, the options
//!   and the multicast options) are their two members side by side, as LZ had them; a pcb
//!   uses the members of its family (`INP_IPV6`).
//! - The IPv6 halves of the functions the C writes for both families are
//!   `crate::netinet6::in6_pcb`'s since N6.5c (D-17's second half): `in_pcbbind_locked`,
//!   `in_pcbconnect` and `in_pcbrtentry` choose the family themselves, as the C's `#ifdef
//!   INET6` branches; `in_setsockaddr`, `in_setpeeraddr` and `in_pcbset_addr` keep the IPv4
//!   socket address of their signature, and `bsd`'s adapters call `in6_setsockaddr`,
//!   `in6_setpeeraddr` and `in6_pcbset_addr` for an IPv6 pcb. The family-blind table code (the
//!   hashes, the port lookups, the rehash and the resize) is here for both, the IPv6 hash in
//!   `crate::netinet6::in6_pcb`.
//! - `hashinit(M_WAITOK)` cannot fail in C; here its failure in [`in_pcbinit`] is `panic(9)`.
//! - `in_pcbaddrisavail_lock` clears the port and `sin_zero` of a copy of the address (the C
//!   clears them in the caller's mbuf and puts the port back).
//! - `NSTOEPLITZ`: the flow id of a connected or accepted socket is set at the end of
//!   [`in_pcbconnect`] and [`in_pcbset_addr`], after the table mutex as in the C
//!   (`crate::net::toeplitz`, N6.6a1; `bsd`'s adapters set it before).
//! - `NPF` (pf(4)) is reached through the `Pf` bridge (`crate::net::pf_hooks`): the divert
//!   keys of [`in_pcblookup_listen`] (`find_divert`), `pf_remove_divert_state` and
//!   `pf_inp_unlink` (`inp_remove_divert_state`, `inp_unlink`); before pf registers they do
//!   what the C compiled with `NPF == 0` does.
//! - The `DIAGNOSTIC` `in_pcbnotifymiss` printfs are not made (the C prints them only when a
//!   debugger sets the variable).
//!
//! ## Redesign
//! - LZ's `Inpcb` was an `inpcb_pool` item of `Cell`s and `queue.h` entries handed around as
//!   `&'static Inpcb`, freed by `pool_put` at the last `in_pcbunref`. Here it is a counted item
//!   of the glue's "inpcb" pool ([`InpcbRef`], a `hal::pool::PoolArc`; `pool_home!` so a list
//!   element or a borrowed pcb gives a counted handle, the C's `in_pcbref`), on three pinned
//!   lists (`hal::pinlist`) keyed to the table's mutex: the queue and the two hash chains.
//!   Every member is an atomic or a cell that can be shared between CPUs: the \[t\] addresses,
//!   ports and routing table are relaxed atomics written only through setters that take the
//!   table's guard (D-6: the C reads them unlocked under the socket lock on the transports'
//!   paths; a 16-byte IPv6 address read without either lock can tear as the C's can), the
//!   flags and the small scalars [`AtomicCell`]s, the header prototypes atomic words.
//! - The C's `inp_refcnt` is two counts until the transports move (N6.6, R2): the handles
//!   (`PoolArc`'s count: the iterator's, the lookups' answers, the anchor) and `inp_refcnt`
//!   itself, the count of `bsd`'s holders of a `&'static Inpcb` (the tcpcb, the timers, the
//!   syn cache, pf's state key and packets, the lookups `bsd`'s adapters answer). While that
//!   count is above zero the pcb keeps a handle to itself, its anchor; the count starts at one
//!   (the socket's, the C's `refcnt_init`), which [`in_pcbdetach`] drops; `bsd` takes more with
//!   [`Inpcb::legacy_ref`] and drops them with [`Inpcb::legacy_rele`], which takes the
//!   [`LegacyKey`] only `bsd`'s boot holds, so no safe kernel code can drop a count `bsd`'s
//!   `&'static` rests on. docs/SCAFFOLDING.md row; docs/IDIOMS.md row.
//! - The \[s\] route cache is a `hal::slot::ReadMostly` (the kernel's pcb must be `Sync`; LZ's
//!   was a `Cell`) lent out and back as LZ's `RouteCell` was ([`Inpcb::route`]), checked
//!   against the socket lock at run time as LZ checked it (D-5's fallback, R1: the brief's
//!   cell opened by either of two locks cannot be opened by `bsd`'s detached holdings, and a
//!   `SoLocked` of `solock_nonet` holds `so_lock` without a proof of the net lock).
//! - N6.4c1, the IP socket options: the \[N\] multicast options (LZ's `inp_moptions`, a raw
//!   `malloc(M_IPMOPTS)` pointer read through an `unsafe` deref) are an
//!   `Option<MBox<IpMoptions>>` in a `ReadMostly` cell, lent under the same check as the
//!   route ([`Inpcb::moptions`], an [`InpLoan`]: the route's loan made generic over the
//!   member) to `ip_setmoptions` and freed by [`in_pcbdetach`] (`ip_freemoptions`, which
//!   leaves the groups: the detach takes the net lock's proof for it); `ip_output`'s callers
//!   copy the three scalars it reads out of the cell ([`Inpcb::moptions_out`], a read, since
//!   `ip_output` sleeps). `in_pcbconnect` and `in_pcbselsrc` read the socket's interface for
//!   multicasts themselves: their `mcast_ifidx` parameter (`bsd`'s read of its word) is gone.
//!   The cell is 40 bytes where `bsd`'s word was 8 (the box's malloc type and allocator).
//! - The "inpcb" item (D-3, the user's class: `vmstat -m` shows it under the C's pool name):
//!   the C's `struct inpcb` is 328 bytes on LP64 (measured: a `repr(C)` replica of in_pcb.h's
//!   members in the size test; the three queue entries 48, the two address unions 32,
//!   `struct route` 72, `struct refcnt` 8, the header union 40); LZ's `Inpcb` was 352 (measured: a replica of its members over
//!   this tree's types, LZ's `Route` of 72 and `Refcnt` of 8; the tree's at 2baa2aff, with
//!   `RouteCell`, the same); here 544, 560 with the counted item's header (the pool and the
//!   count): the three pinned entries (96: a lock identity each beside the two links), the
//!   route cache's cell (72), the IPv4 multicast options' and (N6.5c, +64) the IPv6 packet
//!   and multicast options' (40 each), the IPv4 options' slot (24, N6.6a0: +16 over `bsd`'s
//!   word), the anchor (16), the IPv4 and IPv6 members side by
//!   side (the C's unions), the hash slots (8). docs/SCAFFOLDING.md's pool-size row.
//! - N6.6a2's diet (the user's request of 2026-11-16, to keep the item within 584 B with the
//!   kernel's tcpcb slot, +32): `inp_mou` is one cell, the C's union of `inp_moptions` and
//!   `inp_moptions6` tagged by family ([`Mou`], lent by [`Inpcb::moptions`] and
//!   [`Inpcb::moptions6`] as a [`MouLoan`] of the family's member; a loan of the other
//!   family's is `panic(9)`, where the C would read the union as the wrong member): 48 B
//!   where the two cells were 80. The item was 568 B (552 and the header); 560 since N6.6a5,
//!   `bsd`'s `inp_ppcb` word gone with `bsd`'s TCP.
//! - The IPv6 packet and multicast options are the kernel's typed cells since N6.5c (D-10),
//!   lent by [`Inpcb::outputopts6`] and [`Inpcb::moptions6`] under the socket lock (the C's
//!   union of `inp_options` and `inp_outputopts6` is two members here, as in LZ).
//! - N6.6a0 (D-15): the IPv4 options are the kernel's cell, a `hal::slot::LoanSlot` of the
//!   chain (an mbuf chain is not `Sync`, so not a `ReadMostly`), lent by [`Inpcb::options`]
//!   under the socket lock's check and freed by [`in_pcbdetach`]; `bsd`'s transports reach
//!   it through their adapter's loans (`InpcbLegacy`). The re-keying of the pcb's `[s]`
//!   members to `so_lock`'s witness is N6.6b's.
//! - `bsd`'s own member, the ICMPv6 filter, is a word `bsd` keeps (`hal::link::PtrWord`),
//!   which the kernel never reads (N6.6b's; the per-protocol pcb's word went with `bsd`'s
//!   TCP at N6.6a5, the kernel's slot being the tcpcb's); `inp_upcall` is set once
//!   (`hal::once::OnceStatic`), as wg(4) sets
//!   it on a new socket; `inp_pf_sk` is pf's word ([`PfSlot`]).
//! - The table: [`Inpcbtable`] holds the queue (a pinned list keyed to its mutex) and the
//!   SipHash keys; its mutex holds the hash arrays (`MArray`s of pinned lists, `hashinit`'s),
//!   the masks and the counts ([`InpTable`]). The lookups and the hash changes take the
//!   arrays out of the guard for the call and put them back (two moves of a word, no lock or
//!   atomic: the lists need the guard as their witness while the arrays are borrowed).
//!   [`in_pcbresize`] (D-8) makes two new arrays, moves every pcb from the old chains to the
//!   new ones under the one guard (remove + insert at the head, O(1) each: the queue walk is
//!   the C's `TAILQ_FOREACH`, O(n)) and drops the old, empty arrays.
//! - The iterator (D-7) is a whole [`Inpcb`] with no table and no socket ([`Inpcb::marker`]),
//!   pinned on the walker's stack and moved past the next pcb with `PinList::step_over`
//!   (O(1) a step, docs/COLLECTIONS.md); [`in_pcb_iterator`] takes the previous answer by
//!   value and gives the next one counted, so the C's `in_pcbunref` of the previous pcb is
//!   the drop of its handle under the table mutex, as the C's `pool_put` under it.
//! - N6.6a0 (D-1): [`in_pcbsolock`] answers the socket's lock as a [`SoLocked`] under the
//!   caller's proof of the net lock: a witness of `so_lock` for a kernel protocol's socket
//!   (its `[s]` cells open through [`SoLocked::inet`]'s parts), a detached holding for a
//!   `bsd` protocol's; its drop is `in_pcbsounlock`. LZ's pair, taken in one function and
//!   released in another, stays for `bsd`'s adapter as [`in_pcbsolock_detached`] and
//!   [`in_pcbsounlock_detached`] until the other transports move (N6.6b; TCP, the kernel's
//!   since N6.6a5, uses neither).
//!   [`in_pcbnotifyall`] and `in6_pcbnotify` lock through the new form and lend `notify` the
//!   witness. A kernel TCP request never calls `in_pcbsolock` on the socket it holds (the
//!   C's `tcp_timer_rexmt` lets its socket go before `in_pcbnotifyall`).
//!
//! Lock and spl transitions per operation, against the C (`in_pcb.c`):
//!
//! | operation | the C | the kernel |
//! |---|---|---|
//! | `in_pcblookup` (`IN_PCBLOCK_GRAB`) | `inpt_mtx` once, `refcnt_take` | the same mutex once, the handle's count (one atomic) |
//! | `in_pcbbind`, `in_pcbconnect` | the socket lock (caller), `inpt_mtx` once | the same |
//! | `in_pcballoc` | `inpt_mtx` once | the same; the anchor's cell once (no lock, no spl) |
//! | `in_pcbdetach` | the socket lock (caller), `inpt_mtx` once, pf's mutex in pf | the same |
//! | `in_pcb_iterator` step | `inpt_mtx` held by the caller | the same |
//! | `in_pcbnotifyall` per pcb | `inpt_mtx` dropped and taken, the socket lock pair | the same (D-16: named for N12) |
//! | route cache loan | none (the socket lock held) | two claims of the cell (atomics, no spl) |
//! | multicast options loan (`ip_setmoptions`, `in_pcbdetach`) | none (the socket lock held) | two claims of the cell (atomics, no spl) |
//! | multicast options read (`ip_output`'s callers, `in_pcbselsrc`) | one load | a reader's count (two atomics, no spl) |
//!
//! Holders of a reference (the C's `in_pcbref`/`in_pcbunref`) and their shape here:
//!
//! | the C | here |
//! |---|---|
//! | `in_pcballoc`'s `refcnt_init` (the socket's) | `inp_refcnt` 1 and the anchor, dropped by [`in_pcbdetach`] |
//! | `in_pcb_iterator`'s `in_pcbref`/`in_pcbunref` | the handle it answers, dropped by the next call |
//! | `in_pcblookup*`'s answers | an [`InpcbRef`]; `bsd`'s adapters take the guard, call the `_held` lookups and turn the answer into a legacy count with [`Inpcb::legacy_ref_locked`] before they let the guard go (the C's `in_pcbref` before `mtx_leave`; review 1's B1: after it, a concurrent close and last `in_pcbunref` could free the pcb first) |
//! | divert, raw and udp deliveries, tcp timers and syn cache, pf (`sk_inp`, `pf.inp`) | legacy counts, `bsd`'s, until N6.6 and N8 |

use core::ffi::c_void;
use core::pin::Pin;
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::link::PtrWord;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::netlock::NetHeld;
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::{PR_NOWAIT, PR_WAITOK, PoolArc, PoolBox, PoolHome, TypedPool};
use hal::rwlock::{Guarded, RW_WRITE, Witness, WitnessPart};
use hal::slot::{LoanSlot, ReadMostly, SlotLoan};
use hal::sys::errno::Errno;
use hal::sys::net_if::IFF_UP;
use hal::sys::net_route::{RTF_DYNAMIC, RTF_GATEWAY};
use hal::sys::netinet_in::{
    INADDR_ANY, INADDR_BROADCAST, IPPORT_HIFIRSTAUTO, IPPORT_HILASTAUTO, IPPORT_RESERVED,
    IPPORT_USERRESERVED, IPPROTO_TCP, IPPROTO_UDP, IPSEC_AUTH_LEVEL_DEFAULT,
    IPSEC_ESP_NETWORK_LEVEL_DEFAULT, IPSEC_ESP_TRANS_LEVEL_DEFAULT, IPSEC_IPCOMP_LEVEL_DEFAULT,
    InAddr, SockaddrIn, in_multicast,
};
use hal::sys::netinet_ip::Ip;
use hal::sys::netinet6_in6::{In6Addr, in6_is_addr_unspecified};
use hal::sys::socket::{
    AF_INET, SO_ACCEPTCONN, SO_BINDANY, SO_REUSEADDR, SO_REUSEPORT, SOCK_DGRAM,
};
use libkern::siphash::AtomicSiphashKey;

use crate::assert::kassert;
use crate::assert::kpanic;
use crate::fs::subr::{hashinit, siphash_key};
use crate::net::if_::{if_get, ifa_ifwithaddr};
use crate::net::if_var::AtomicCell;
use crate::net::ipsec_hooks::{IpsecLevel, UDPENCAP_PORT};
use crate::net::mbuf::{M_WAIT, Mbuf, MbufBox, PF_TAG_DIVERTED, PF_TAG_TRANSLATE_LOCALHOST};
use crate::net::pf_hooks::{PF_DIVERT_REPLY, PF_DIVERT_TO, PfSlot, pf};
use crate::net::proto::PR_CONNREQUIRED;
use crate::net::route::{Route, Rtentry, rtdeletemsg};
use crate::net::rtable::{rtable_exists, rtable_getsource, rtable_l2};
use crate::net::sockbuf::SS_NOFDREF;
use crate::net::socket::{SoLocked, Socket, SocketRef, so_part, sofree_legacy, solock_net, soref};
use crate::net::toeplitz::stoeplitz_ip4port;
use crate::netinet::in_::{in_broadcast, in_ifp2ia, in_nam2sin};
use crate::netinet::in_var::ifatoia;
use crate::netinet::ip_output::ip_freemoptions;
use crate::netinet::ip_var::IpMoptions;
use crate::netinet::tcp_var::Tcpcb;
use crate::netinet6::in6::in6_nam2sin6;
use crate::netinet6::in6_pcb::{
    in6_pcbaddrisavail_held, in6_pcbconnect, in6_pcbhash, in6_pcbrtentry,
};
use crate::netinet6::ip6_output::{ip6_freemoptions, ip6_freepcbopts};
use crate::netinet6::ip6_var::{Ip6Moptions, Ip6Pktopts};
use crate::proc::{ProcGlue, Thread, suser};

// flags in inp_flags:

/// `INP_RECVOPTS`: receive incoming IP options.
pub const INP_RECVOPTS: i32 = 0x001;
/// `INP_RECVRETOPTS`: receive IP options for reply.
pub const INP_RECVRETOPTS: i32 = 0x002;
/// `INP_RECVDSTADDR`: receive IP dst address.
pub const INP_RECVDSTADDR: i32 = 0x004;

/// `INP_RXDSTOPTS`.
pub const INP_RXDSTOPTS: i32 = INP_RECVOPTS;
/// `INP_RXHOPOPTS`.
pub const INP_RXHOPOPTS: i32 = INP_RECVRETOPTS;
/// `INP_RXINFO`.
pub const INP_RXINFO: i32 = INP_RECVDSTADDR;
/// `INP_RXSRCRT`.
pub const INP_RXSRCRT: i32 = 0x010;
/// `INP_HOPLIMIT`.
pub const INP_HOPLIMIT: i32 = 0x020;

/// `INP_HDRINCL`: user supplies entire IP header.
pub const INP_HDRINCL: i32 = 0x008;
/// `INP_HIGHPORT`: user wants "high" port binding.
pub const INP_HIGHPORT: i32 = 0x010;
/// `INP_LOWPORT`: user wants "low" port binding.
pub const INP_LOWPORT: i32 = 0x020;
/// `INP_RECVIF`: receive incoming interface.
pub const INP_RECVIF: i32 = 0x080;
/// `INP_RECVTTL`: receive incoming IP TTL.
pub const INP_RECVTTL: i32 = 0x040;
/// `INP_RECVDSTPORT`: receive IP dst addr before rdr.
pub const INP_RECVDSTPORT: i32 = 0x200;
/// `INP_RECVRTABLE`: receive routing table.
pub const INP_RECVRTABLE: i32 = 0x400;
/// `INP_IPSECFLOWINFO`: receive IPsec flow info.
pub const INP_IPSECFLOWINFO: i32 = 0x800;

/// `INP_CONTROLOPTS`.
pub const INP_CONTROLOPTS: i32 = INP_RECVOPTS
    | INP_RECVRETOPTS
    | INP_RECVDSTADDR
    | INP_RXSRCRT
    | INP_HOPLIMIT
    | INP_RECVIF
    | INP_RECVTTL
    | INP_RECVDSTPORT
    | INP_RECVRTABLE;

// These flags' values should be determined by either the transport protocol at PRU_BIND,
// PRU_LISTEN, PRU_CONNECT, etc, or by in_pcb*().

/// `INP_IPV6`: socket, proto, domain, family is `PF_INET6`.
pub const INP_IPV6: i32 = 0x100;

// Flags in inp_flags for IPV6

/// `IN6P_HIGHPORT`: user wants "high" port.
pub const IN6P_HIGHPORT: i32 = INP_HIGHPORT;
/// `IN6P_LOWPORT`: user wants "low" port.
pub const IN6P_LOWPORT: i32 = INP_LOWPORT;
/// `IN6P_RECVDSTPORT`: receive IP dst addr before rdr.
pub const IN6P_RECVDSTPORT: i32 = INP_RECVDSTPORT;
/// `IN6P_PKTINFO`: receive IP6 dst and I/F.
pub const IN6P_PKTINFO: i32 = 0x010000;
/// `IN6P_HOPLIMIT`: receive hoplimit.
pub const IN6P_HOPLIMIT: i32 = 0x020000;
/// `IN6P_HOPOPTS`: receive hop-by-hop options.
pub const IN6P_HOPOPTS: i32 = 0x040000;
/// `IN6P_DSTOPTS`: receive dst options after rthdr.
pub const IN6P_DSTOPTS: i32 = 0x080000;
/// `IN6P_RTHDR`: receive routing header.
pub const IN6P_RTHDR: i32 = 0x100000;
/// `IN6P_TCLASS`: receive traffic class value.
pub const IN6P_TCLASS: i32 = 0x400000;
/// `IN6P_AUTOFLOWLABEL`: attach flowlabel automatically.
pub const IN6P_AUTOFLOWLABEL: i32 = 0x800000;

/// `IN6P_ANONPORT`: port chosen for user.
pub const IN6P_ANONPORT: i32 = 0x4000000;
/// `IN6P_RFC2292`: used RFC2292 API on the socket.
pub const IN6P_RFC2292: i32 = 0x40000000;
/// `IN6P_MTU`: receive path MTU (bit 31 of the C's `int`).
pub const IN6P_MTU: i32 = 0x80000000_u32 as i32;

/// `IN6P_MINMTU`: use minimum MTU.
pub const IN6P_MINMTU: i32 = 0x20000000;

/// `IN6P_CONTROLOPTS`.
pub const IN6P_CONTROLOPTS: i32 = IN6P_PKTINFO
    | IN6P_HOPLIMIT
    | IN6P_HOPOPTS
    | IN6P_DSTOPTS
    | IN6P_RTHDR
    | IN6P_TCLASS
    | IN6P_AUTOFLOWLABEL
    | IN6P_RFC2292
    | IN6P_MTU
    | IN6P_RECVDSTPORT;

/// `INPLOOKUP_WILDCARD`.
pub const INPLOOKUP_WILDCARD: i32 = 1;
/// `INPLOOKUP_SETLOCAL`.
pub const INPLOOKUP_SETLOCAL: i32 = 2;
/// `INPLOOKUP_IPV6`.
pub const INPLOOKUP_IPV6: i32 = 4;

// macros for handling bitmap of ports not to allocate dynamically

/// `DP_MAPBITS`: bits per map word.
pub const DP_MAPBITS: usize = u32::BITS as usize;
/// `DP_MAPSIZE`: words per map.
pub const DP_MAPSIZE: usize = 65536_usize.div_ceil(DP_MAPBITS);

/// `DEFBADDYNAMICPORTS_TCP`: default values for `baddynamicports` (see `ip_init()`).
pub const DEFBADDYNAMICPORTS_TCP: &[u16] = &[
    587, 749, 750, 751, 853, 871, 2049, 6000, 6001, 6002, 6003, 6004, 6005, 6006, 6007, 6008, 6009,
    6010,
];
/// `DEFBADDYNAMICPORTS_UDP` (3784, 3785, 7784: BFD/S-BFD ports).
pub const DEFBADDYNAMICPORTS_UDP: &[u16] = &[623, 664, 749, 750, 751, 2049, 3784, 3785, 7784];

/// `DEFROOTONLYPORTS_TCP`.
pub const DEFROOTONLYPORTS_TCP: &[u16] = &[2049];
/// `DEFROOTONLYPORTS_UDP`.
pub const DEFROOTONLYPORTS_UDP: &[u16] = &[2049];

/// `IN_PCBLOCK_HOLD`: the caller holds the table mutex.
pub const IN_PCBLOCK_HOLD: i32 = 1;
/// `IN_PCBLOCK_GRAB`: the lookup takes the table mutex and a reference.
pub const IN_PCBLOCK_GRAB: i32 = 2;

/// `INPCBHASH_LOADFACTOR(x)`.
const fn inpcbhash_loadfactor(x: i32) -> i32 {
    (x * 3) / 4
}

/// `M_PCB`: protocol control blocks (`malloc.h`; `hashinit`'s type here).
const M_PCB: i32 = 4;

/// `inp_upcall` and `inp_upcall_arg`, set once ([`Inpcb::set_upcall`]): a hook that sees UDP
/// datagrams before the socket buffer (wg(4)'s `wg_input`, set by `wg_bind`). The function is
/// `bsd`'s, erased (`bsd`'s UDP input names its type to call it); the argument a word.
#[derive(Clone, Copy)]
pub struct InpUpcall {
    /// `inp_upcall`.
    pub f: &'static (dyn core::any::Any + Send + Sync),
    /// `inp_upcall_arg`: the argument's address.
    pub arg: usize,
}

/// A counted control block: `in_pcbref` is a clone, `in_pcbunref` a drop.
pub type InpcbRef<G> = PoolArc<Inpcb<G>>;

/// The table mutex held (`mtx_enter(&table->inpt_mtx)`): the witness of the queue and the
/// hash chains, and the \[t\] members' setters' proof.
pub type InpGuard<'a, G> = MutexedGuard<'a, InpTable<G>>;

/// `union inpaddru`: an IPv4 or IPv6 address, the `const void *laddr` the port lookups take
/// and read as the address of the family their `INP_IPV6`/`INPLOOKUP_IPV6` says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Inpaddru {
    /// The bytes of the union: `iau_addr6`, whose first four bytes are `iau_addr`.
    bytes: [u8; 16],
}

impl Inpaddru {
    /// A union holding the IPv4 address `a` (`iau_addr`).
    pub const fn from_addr(a: InAddr) -> Self {
        let w = a.s_addr.to_ne_bytes();
        let mut bytes = [0; 16];
        bytes[0] = w[0];
        bytes[1] = w[1];
        bytes[2] = w[2];
        bytes[3] = w[3];
        Self { bytes }
    }

    /// A union holding the IPv6 address of bytes `b` (`iau_addr6`).
    pub const fn from_bytes6(b: [u8; 16]) -> Self {
        Self { bytes: b }
    }

    /// A union holding the IPv6 address `a6` (`iau_addr6`).
    pub fn from_addr6<A: Addr6>(a6: A) -> Self {
        Self::from_bytes6(a6.to_bytes())
    }

    /// `iau_addr`: the union read as an IPv4 address.
    pub const fn iau_addr(&self) -> InAddr {
        InAddr {
            s_addr: u32::from_ne_bytes([
                self.bytes[0],
                self.bytes[1],
                self.bytes[2],
                self.bytes[3],
            ]),
        }
    }

    /// `iau_addr6`: the union read as an IPv6 address.
    pub fn iau_addr6<A: Addr6>(&self) -> A {
        A::from_bytes(self.bytes)
    }

    /// The union's sixteen bytes.
    pub const fn bytes6(&self) -> [u8; 16] {
        self.bytes
    }
}

/// An IPv6 address as the pcb stores it: its 16 bytes. `hal`'s image and `bsd`'s
/// (`netinet6/in6.rs`, until N6.5) both are one, so the IPv6 members' readers name the type
/// they want.
pub trait Addr6: Copy {
    /// The address of bytes `b`.
    fn from_bytes(b: [u8; 16]) -> Self;
    /// The address's bytes.
    fn to_bytes(&self) -> [u8; 16];
}

impl Addr6 for In6Addr {
    #[inline]
    fn from_bytes(b: [u8; 16]) -> Self {
        In6Addr { s6_addr: b }
    }
    #[inline]
    fn to_bytes(&self) -> [u8; 16] {
        self.s6_addr
    }
}

impl Addr6 for [u8; 16] {
    #[inline]
    fn from_bytes(b: [u8; 16]) -> Self {
        b
    }
    #[inline]
    fn to_bytes(&self) -> [u8; 16] {
        *self
    }
}

/// An IPv6 header as the pcb's prototype stores it: its 40 bytes (`hal`'s image and `bsd`'s
/// `netinet/ip6.rs` copy until N6.5).
pub trait Hdr6: Copy {
    /// The header of bytes `b`.
    fn from_bytes(b: [u8; 40]) -> Self;
    /// The header's bytes.
    fn to_bytes(&self) -> [u8; 40];
}

impl Hdr6 for hal::sys::netinet_ip6::Ip6Hdr {
    fn from_bytes(b: [u8; 40]) -> Self {
        let mut src = [0; 16];
        let mut dst = [0; 16];
        src.copy_from_slice(&b[8..24]);
        dst.copy_from_slice(&b[24..40]);
        Self {
            ip6_flow: u32::from_ne_bytes([b[0], b[1], b[2], b[3]]),
            ip6_plen: u16::from_ne_bytes([b[4], b[5]]),
            ip6_nxt: b[6],
            ip6_hlim: b[7],
            ip6_src: In6Addr { s6_addr: src },
            ip6_dst: In6Addr { s6_addr: dst },
        }
    }
    fn to_bytes(&self) -> [u8; 40] {
        let mut b = [0; 40];
        b[0..4].copy_from_slice(&self.ip6_flow.to_ne_bytes());
        b[4..6].copy_from_slice(&self.ip6_plen.to_ne_bytes());
        b[6] = self.ip6_nxt;
        b[7] = self.ip6_hlim;
        b[8..24].copy_from_slice(&self.ip6_src.s6_addr);
        b[24..40].copy_from_slice(&self.ip6_dst.s6_addr);
        b
    }
}

/// A \[t\] scalar (`inp_[lf]port`, `inp_rtableid`): read anywhere by a relaxed load (the C's
/// unlocked reads under the socket lock), written only by this module's setters, which take
/// the table's guard.
pub struct TWord<T: crate::net::if_var::AtomicWord>(AtomicCell<T>);

impl<T: crate::net::if_var::AtomicWord> TWord<T> {
    /// 0.
    const fn new() -> Self {
        Self(AtomicCell::new())
    }

    /// The value.
    #[inline]
    pub fn get(&self) -> T {
        self.0.get()
    }

    /// Stores `v` (the caller holds the table's guard).
    #[inline]
    fn set(&self, v: T) {
        self.0.set(v);
    }
}

/// A \[t\] IPv4 address (`inp_faddr`, `inp_laddr`), as [`TWord`].
pub struct InAddrCell(AtomicU32);

impl InAddrCell {
    /// `INADDR_ANY`.
    const fn new() -> Self {
        Self(AtomicU32::new(0))
    }

    /// The address.
    #[inline]
    pub fn get(&self) -> InAddr {
        InAddr {
            s_addr: self.0.load(Ordering::Relaxed),
        }
    }

    /// Stores `a` (the caller holds the table's guard).
    #[inline]
    fn set(&self, a: InAddr) {
        self.0.store(a.s_addr, Ordering::Relaxed);
    }
}

/// A \[t\] IPv6 address (`inp_faddr6`, `inp_laddr6`): four relaxed words, read whole by a
/// holder of either lock (a reader with neither can see a torn address, as the C's can). Also
/// ND's `ln_saddr6`, under `nd6_mtx` (`kernel::netinet6::nd6`).
pub struct In6AddrCell([AtomicU32; 4]);

impl In6AddrCell {
    /// The unspecified address.
    pub(crate) const fn new() -> Self {
        Self([const { AtomicU32::new(0) }; 4])
    }

    /// The address, as the type the reader names.
    #[inline]
    pub fn get<A: Addr6>(&self) -> A {
        A::from_bytes(self.bytes())
    }

    /// The address's bytes.
    #[inline]
    pub fn bytes(&self) -> [u8; 16] {
        let mut b = [0; 16];
        for (i, w) in self.0.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&w.load(Ordering::Relaxed).to_ne_bytes());
        }
        b
    }

    /// Stores `a` (the caller holds the lock the address is under).
    pub(crate) fn set_bytes(&self, a: [u8; 16]) {
        for (i, w) in self.0.iter().enumerate() {
            w.store(
                u32::from_ne_bytes([a[i * 4], a[i * 4 + 1], a[i * 4 + 2], a[i * 4 + 3]]),
                Ordering::Relaxed,
            );
        }
    }
}

/// `inp_ip`: the IPv4 header prototype, five relaxed words with LZ's `Cell<Ip>` interface
/// (written by `in_pcballoc`'s callers and `ip_ctloutput` under the socket lock, read by the
/// transports under it).
pub struct IpProto([AtomicU32; 5]);

impl IpProto {
    /// All zero.
    const fn new() -> Self {
        Self([const { AtomicU32::new(0) }; 5])
    }

    /// The header.
    pub fn get(&self) -> Ip {
        let w = |i: usize| self.0[i].load(Ordering::Relaxed).to_ne_bytes();
        let (a, b, c) = (w(0), w(1), w(2));
        Ip {
            ip_vhl: a[0],
            ip_tos: a[1],
            ip_len: u16::from_ne_bytes([a[2], a[3]]),
            ip_id: u16::from_ne_bytes([b[0], b[1]]),
            ip_off: u16::from_ne_bytes([b[2], b[3]]),
            ip_ttl: c[0],
            ip_p: c[1],
            ip_sum: u16::from_ne_bytes([c[2], c[3]]),
            ip_src: InAddr {
                s_addr: self.0[3].load(Ordering::Relaxed),
            },
            ip_dst: InAddr {
                s_addr: self.0[4].load(Ordering::Relaxed),
            },
        }
    }

    /// Stores `ip`.
    pub fn set(&self, ip: Ip) {
        let len = ip.ip_len.to_ne_bytes();
        let id = ip.ip_id.to_ne_bytes();
        let off = ip.ip_off.to_ne_bytes();
        let sum = ip.ip_sum.to_ne_bytes();
        let words = [
            u32::from_ne_bytes([ip.ip_vhl, ip.ip_tos, len[0], len[1]]),
            u32::from_ne_bytes([id[0], id[1], off[0], off[1]]),
            u32::from_ne_bytes([ip.ip_ttl, ip.ip_p, sum[0], sum[1]]),
            ip.ip_src.s_addr,
            ip.ip_dst.s_addr,
        ];
        for (c, w) in self.0.iter().zip(words) {
            c.store(w, Ordering::Relaxed);
        }
    }
}

/// `inp_ipv6`: the IPv6 header prototype, ten relaxed words read and written whole as the
/// type the caller names ([`Hdr6`]).
pub struct Ip6Proto([AtomicU32; 10]);

impl Ip6Proto {
    /// All zero.
    const fn new() -> Self {
        Self([const { AtomicU32::new(0) }; 10])
    }

    /// The header.
    pub fn get<H: Hdr6>(&self) -> H {
        let mut b = [0; 40];
        for (i, w) in self.0.iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&w.load(Ordering::Relaxed).to_ne_bytes());
        }
        H::from_bytes(b)
    }

    /// Stores `h`.
    pub fn set<H: Hdr6>(&self, h: H) {
        let b = h.to_bytes();
        for (i, w) in self.0.iter().enumerate() {
            w.store(
                u32::from_ne_bytes([b[i * 4], b[i * 4 + 1], b[i * 4 + 2], b[i * 4 + 3]]),
                Ordering::Relaxed,
            );
        }
    }
}

/// `inp_seclevel`: the socket's IPsec levels, one relaxed word (\[N\]).
pub struct SeclevelCell(AtomicU32);

impl SeclevelCell {
    /// The C's zeroed levels.
    const fn new() -> Self {
        Self(AtomicU32::new(0))
    }

    /// The levels.
    pub fn get(&self) -> IpsecLevel {
        let b = self.0.load(Ordering::Relaxed).to_ne_bytes();
        IpsecLevel {
            sl_auth: b[0],
            sl_esp_trans: b[1],
            sl_esp_network: b[2],
            sl_ipcomp: b[3],
        }
    }

    /// Stores `sl`.
    pub fn set(&self, sl: IpsecLevel) {
        self.0.store(
            u32::from_ne_bytes([sl.sl_auth, sl.sl_esp_trans, sl.sl_esp_network, sl.sl_ipcomp]),
            Ordering::Relaxed,
        );
    }
}

/// The capability to drop a count of `bsd`'s ([`Inpcb::legacy_rele`]): one per kernel, taken
/// by `bsd`'s `in_init` at boot, so no code of the kernel crate can drop a count a `bsd`
/// `&'static Inpcb` rests on (the `hal::link::SaToken` and `TdbKey` precedents).
pub struct LegacyKey(());

/// The one [`LegacyKey`].
static LEGACY_KEY: LegacyKey = LegacyKey(());
/// Whether [`LegacyKey::take`] gave it.
static LEGACY_KEY_TAKEN: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

impl LegacyKey {
    /// The key, to its first taker only; `None` afterwards.
    pub fn take() -> Option<&'static LegacyKey> {
        if LEGACY_KEY_TAKEN.swap(true, Ordering::AcqRel) {
            None
        } else {
            Some(&LEGACY_KEY)
        }
    }
}

/// `struct inpcb`: common structure pcb for internet protocol implementation. Here are
/// stored pointers to local and foreign host table entries, local and foreign socket numbers,
/// and pointers up (to a socket structure) and down (to a protocol-specific) control block.
pub struct Inpcb<G: ProcGlue> {
    /// The item's place in the "inpcb" pool (`in_pcbref` of a borrowed pcb).
    inp_home: PoolHome<Self>,
    /// \[I\] `inp_table`: inet queue/hash table (`None` for an iterator).
    pub inp_table: Option<&'static Inpcbtable<G>>,
    /// \[t\] `inp_queue`: inet PCB queue.
    inp_queue: PinEntry<Self>,
    /// \[t\] `inp_hash`: local and foreign hash.
    inp_hash: PinEntry<Self>,
    /// \[t\] `inp_lhash`: local port hash.
    inp_lhash: PinEntry<Self>,
    /// \[t\] The bucket of `inp_hash` the pcb is on (a pinned list's removal names its list;
    /// the C's `LIST_REMOVE` needs no head).
    inp_hslot: AtomicU32,
    /// \[t\] The bucket of `inp_lhash` the pcb is on.
    inp_lslot: AtomicU32,
    /// \[t\] `inp_faddr`: foreign address (`inp_faddru.iau_addr`).
    pub inp_faddr: InAddrCell,
    /// \[t\] `inp_faddr6`: foreign IPv6 address (`inp_faddru.iau_addr6`).
    pub inp_faddr6: In6AddrCell,
    /// \[t\] `inp_laddr`: local address (`inp_laddru.iau_addr`).
    pub inp_laddr: InAddrCell,
    /// \[t\] `inp_laddr6`: local IPv6 address (`inp_laddru.iau_addr6`).
    pub inp_laddr6: In6AddrCell,
    /// \[t\] `inp_fport`: foreign port, network order.
    pub inp_fport: TWord<u16>,
    /// \[t\] `inp_lport`: local port, network order.
    pub inp_lport: TWord<u16>,
    /// \[I\] `inp_socket`: back pointer to socket, counted (`None` for an iterator).
    inp_socket: Option<SocketRef<G>>,
    /// \[s\] `inp_ppcb` of TCP (N6.6a2, D-12): the control block this pcb owns, keyed to the
    /// socket's `so_lock` in part `PCB` ([`Inpcb::tcpcb`]); `bsd`'s word went at N6.6a5.
    inp_tcpcb: Guarded<Option<PoolBox<Tcpcb<G>>>>,
    /// \[s\] `inp_route`: cached route, reached only through [`Inpcb::route`] and
    /// [`Inpcb::set_route`], which check the socket lock.
    inp_route: ReadMostly<Route>,
    /// `inp_refcnt`: the references of `bsd`'s holders (the module's Redesign).
    inp_refcnt: AtomicU32,
    /// The handle that keeps the item while `inp_refcnt` is above zero.
    inp_anchor: ReadMostly<Option<InpcbRef<G>>>,
    /// `inp_flags`: generic IP/datagram flags.
    pub inp_flags: AtomicCell<i32>,
    /// `inp_ip`: header prototype (`inp_hu.hu_ip`).
    pub inp_ip: IpProto,
    /// `inp_ipv6`: IPv6 header prototype (`inp_hu.hu_ipv6`).
    pub inp_ipv6: Ip6Proto,
    /// \[s\] `inp_options`: IPv4 options (`ip_pcbopts`' mbuf chain), reached only through
    /// [`Inpcb::options`], which checks the socket lock: a `LoanSlot`, since an mbuf chain is
    /// not `Sync` (N6.6a0, D-15).
    inp_options: LoanSlot<Option<MbufBox>>,
    /// \[s\] `inp_outputopts6`: IPv6 options (`ip6_pcbopt`'s `M_IP6OPT` allocation), reached
    /// only through [`Inpcb::outputopts6`], which checks the socket lock.
    inp_outputopts6: ReadMostly<Option<MBox<Ip6Pktopts>>>,
    /// `inp_hops`.
    pub inp_hops: AtomicCell<i32>,
    /// \[N\] `inp_mou`: the multicast options, IPv4's (`inp_moptions`, `ip_setmoptions`'s
    /// `M_IPMOPTS` allocation) or IPv6's (`inp_moptions6`, `ip6_setmoptions`'s), the C's union
    /// as one cell tagged by family, reached only through [`Inpcb::moptions`] and
    /// [`Inpcb::moptions6`], which check the socket lock, and copied by
    /// [`Inpcb::moptions_out`] and [`Inpcb::moptions6_out`] (N6.6a2's diet: one cell where
    /// N6.4c1 and N6.5c kept two, -32 B).
    inp_mou: ReadMostly<Mou>,
    /// \[N\] `inp_seclevel`: IPsec level of socket.
    pub inp_seclevel: SeclevelCell,
    /// `inp_ip_minttl`: minimum TTL or drop (`inp_ip6_minhlim` for IPv6).
    pub inp_ip_minttl: AtomicCell<u8>,
    /// `inp_cksum6`.
    pub inp_cksum6: AtomicCell<i32>,
    /// `inp_icmp6filt`: the `ICMP6_FILTER` of a raw ICMPv6 socket (`bsd`'s, until N6.5).
    pub inp_icmp6filt: PtrWord<c_void>,
    /// `inp_upcall` and `inp_upcall_arg`.
    inp_upcall: OnceStatic<InpUpcall>,
    /// \[t\] `inp_rtableid`.
    pub inp_rtableid: TWord<u32>,
    /// `inp_pipex`: pipex indication.
    pub inp_pipex: AtomicCell<i32>,
    /// \[s\] `inp_flowid`.
    pub inp_flowid: AtomicCell<u16>,
    /// \[L\] `inp_pf_sk`: the pf(4) state key the socket's packets match (pf's word).
    pub inp_pf_sk: PfSlot,
}

hal::pool_home!(impl[G: ProcGlue] Inpcb<G>, inp_home);

hal::pin_adapter!(
    /// `TAILQ_HEAD(inpthead, inpcb)`: a table's queue, through `inp_queue`.
    impl[G: ProcGlue] pub InpQueue<G>: Inpcb<G>, inp_queue
);

hal::pin_adapter!(
    /// `LIST_HEAD(inpcbhead, inpcb)` of the local and foreign hash, through `inp_hash`.
    impl[G: ProcGlue] pub InpHash<G>: Inpcb<G>, inp_hash
);

hal::pin_adapter!(
    /// `LIST_HEAD(inpcbhead, inpcb)` of the local port hash, through `inp_lhash`.
    impl[G: ProcGlue] pub InpLhash<G>: Inpcb<G>, inp_lhash
);

impl<G: ProcGlue> Inpcb<G> {
    /// A zeroed control block of `table` and `so`, as `pool_get(PR_ZERO)` and the two
    /// assignments of `in_pcballoc` leave it; both `None` make an iterator. A value made here
    /// outside the pool (a marker, a test's block) is on no list and gives no handle
    /// (`in_pcbref` of it is `panic(9)`).
    pub fn new(table: Option<&'static Inpcbtable<G>>, so: Option<SocketRef<G>>) -> Self {
        // The slot of the kernel's tcpcb is the socket lock's; an iterator's is never opened.
        let inp_tcpcb = match &so {
            Some(s) => Guarded::keyed(s.so_lock.lock().id(), so_part::PCB, None),
            None => Guarded::new_net(None),
        };
        Self {
            inp_home: PoolHome::new(),
            inp_table: table,
            inp_queue: PinEntry::new(),
            inp_hash: PinEntry::new(),
            inp_lhash: PinEntry::new(),
            inp_hslot: AtomicU32::new(0),
            inp_lslot: AtomicU32::new(0),
            inp_faddr: InAddrCell::new(),
            inp_faddr6: In6AddrCell::new(),
            inp_laddr: InAddrCell::new(),
            inp_laddr6: In6AddrCell::new(),
            inp_fport: TWord::new(),
            inp_lport: TWord::new(),
            inp_socket: so,
            inp_tcpcb,
            inp_route: ReadMostly::new(Route::new()),
            inp_refcnt: AtomicU32::new(1),
            inp_anchor: ReadMostly::new(None),
            inp_flags: AtomicCell::new(),
            inp_ip: IpProto::new(),
            inp_ipv6: Ip6Proto::new(),
            inp_options: LoanSlot::new(None),
            inp_outputopts6: ReadMostly::new(None),
            inp_hops: AtomicCell::new(),
            inp_mou: ReadMostly::new(Mou::None),
            inp_seclevel: SeclevelCell::new(),
            inp_ip_minttl: AtomicCell::new(),
            inp_cksum6: AtomicCell::new(),
            inp_icmp6filt: PtrWord::new(),
            inp_upcall: OnceStatic::new(),
            inp_rtableid: TWord::new(),
            inp_pipex: AtomicCell::new(),
            inp_flowid: AtomicCell::new(),
            inp_pf_sk: PfSlot::new(),
        }
    }

    /// `struct inpcb_iterator`'s `{ .inp_table = NULL }`: a place holder for a walk of a
    /// table's queue, which the walker pins on its stack (`core::pin::pin!`). Its cells are
    /// never opened.
    pub fn marker() -> Self {
        Self::new(None, None)
    }

    /// `inp->inp_socket` of a control block (not an iterator).
    pub fn socket(&self) -> &Socket<G> {
        match &self.inp_socket {
            Some(so) => so,
            None => kpanic(format_args!("inpcb {self:p}: no socket")),
        }
    }

    /// `inp->inp_socket`: `None` for an iterator.
    pub fn socket_opt(&self) -> Option<&Socket<G>> {
        self.inp_socket.as_deref()
    }

    /// `inp->inp_table` of a control block (not an iterator).
    pub fn table(&self) -> &'static Inpcbtable<G> {
        match self.inp_table {
            Some(t) => t,
            None => kpanic(format_args!("inpcb {self:p}: no table")),
        }
    }

    /// `&inp->inp_route`: the route cache, lent until the loan drops. The cache is `[s]`:
    /// the caller holds this control block's socket lock as `soassertlocked` defines it (the
    /// net lock exclusively, or this thread's write hold of `so_lock`), checked here, since a
    /// loan is a move of the cached route out of its cell and back, and a second thread's loan
    /// of the same cell would race it (`in6_pcbnotify`'s walk reads other sockets' caches only
    /// once it holds their locks). `panic(9)` without the lock.
    pub fn route(&self) -> InpLoan<'_, Route> {
        self.assert_route_locked("inp_route");
        InpLoan::of(&self.inp_route)
    }

    /// `inp->inp_route = *ro` (the struct assignment of `syn_cache_get`), the old route's
    /// handle given back; checked as [`Inpcb::route`].
    pub fn set_route(&self, ro: Route) {
        self.assert_route_locked("inp_route =");
        drop(self.inp_route.replace(ro));
    }

    /// `&inp->inp_moptions`: the IPv4 multicast options, lent until the loan drops (the C's
    /// `struct ip_moptions **` of `ip_setmoptions`, which may make, change or free them). The
    /// options are `[N]`: written under the socket lock, as `soassertlocked` defines it for an
    /// internet socket; checked here as [`Inpcb::route`] checks the route cache, a loan being
    /// a move out of the cell and back. `panic(9)` without the lock.
    pub fn moptions(&self) -> MouLoan<'_, IpMoptions> {
        self.assert_route_locked("inp_moptions");
        MouLoan::of(&self.inp_mou)
    }

    /// The multicast options `ip_output` reads (the interface, the TTL, the loopback switch),
    /// copied out of the cell: `None` when the socket has none (`inp->inp_moptions == NULL`).
    /// A read: the cell's readers count, never a move (the transports' `ip_output` calls
    /// sleep, which no reader of the cell may do while it holds it).
    #[inline]
    pub fn moptions_out(&self) -> Option<IpMoptions> {
        match &*self.inp_mou.read() {
            Mou::V4(imo) => Some(IpMoptions::output(imo.imo_ifidx, imo.imo_ttl, imo.imo_loop)),
            Mou::None | Mou::V6(_) => None,
        }
    }

    /// `&inp->inp_options`: the IPv4 options, lent until the loan drops (the C's `struct mbuf
    /// **` of `ip_pcbopts`, the transports' `ip_output` argument, `tcp_output`'s TSO test,
    /// `syn_cache_get`'s source route, `in_pcbdetach`'s free). The options are `[s]`: checked
    /// as [`Inpcb::route`] checks the route cache, a loan being a move out of the cell and
    /// back (two atomics). `panic(9)` without the lock.
    pub fn options(&self) -> SlotLoan<'_, Option<MbufBox>> {
        self.assert_route_locked("inp_options");
        self.inp_options.lend()
    }

    /// `intotcpcb(inp)`'s slot: the kernel's TCP control block of this pcb, pinned in its
    /// item, for the holder of the socket lock's part `PCB` (`pcb`, checked: another lock's
    /// or part's is `panic(9)`); `None` before `tcp_newtcpcb` and after `tcp_free`.
    #[inline]
    pub fn tcpcb<'a>(&'a self, pcb: &'a WitnessPart<'_>) -> Option<Pin<&'a Tcpcb<G>>> {
        self.inp_tcpcb.get(pcb).as_ref().map(PoolBox::as_pin)
    }

    /// `inp->inp_ppcb = tp` (`tcp_newtcpcb`): the slot owns `tp` from now on; the control
    /// block it held before, if any, comes back (none: `tcp_newtcpcb` runs once per pcb).
    pub fn set_tcpcb(
        &self,
        pcb: &mut WitnessPart<'_>,
        tp: PoolBox<Tcpcb<G>>,
    ) -> Option<PoolBox<Tcpcb<G>>> {
        self.inp_tcpcb.get_mut(pcb).replace(tp)
    }

    /// `inp->inp_ppcb = NULL` with the control block given back (`tcp_free`'s `pool_put`).
    pub fn take_tcpcb(&self, pcb: &mut WitnessPart<'_>) -> Option<PoolBox<Tcpcb<G>>> {
        self.inp_tcpcb.get_mut(pcb).take()
    }

    /// `&inp->inp_outputopts6`: the IPv6 packet options, lent until the loan drops (the C's
    /// `struct ip6_pktopts **` of `ip6_pcbopt`, and the transports' read of them for their
    /// `ip6_output` and source selection calls, which may sleep). The options are `[s]`:
    /// checked as [`Inpcb::route`] checks the route cache. `panic(9)` without the lock.
    pub fn outputopts6(&self) -> InpLoan<'_, Option<MBox<Ip6Pktopts>>> {
        self.assert_route_locked("inp_outputopts6");
        InpLoan::of(&self.inp_outputopts6)
    }

    /// `inp->inp_outputopts6 != NULL`, read without a loan (tcp_output's TSO test of the
    /// C's union slot, which `inp_options` shares there): a reader's count (two atomics).
    pub fn has_outputopts6(&self) -> bool {
        self.inp_outputopts6.read().is_some()
    }

    /// `&inp->inp_moptions6`: the IPv6 multicast options, lent until the loan drops (the C's
    /// `struct ip6_moptions **` of `ip6_setmoptions`), checked as [`Inpcb::moptions`].
    /// `panic(9)` without the socket lock.
    pub fn moptions6(&self) -> MouLoan<'_, Ip6Moptions> {
        self.assert_route_locked("inp_moptions6");
        MouLoan::of(&self.inp_mou)
    }

    /// The multicast options `ip6_output` and the source selection read (the interface, the
    /// hop limit, the loopback switch), copied out of the cell with no membership: `None`
    /// when the socket has none. A read, as [`Inpcb::moptions_out`].
    #[inline]
    pub fn moptions6_out(&self) -> Option<Ip6Moptions> {
        match &*self.inp_mou.read() {
            Mou::V6(im6o) => Some(Ip6Moptions::output(
                im6o.im6o_ifidx,
                im6o.im6o_hlim,
                im6o.im6o_loop,
            )),
            Mou::None | Mou::V4(_) => None,
        }
    }

    /// Whether this thread holds the socket lock that guards `inp_route`: this pcb's
    /// `so_lock` for writing, or the net lock exclusively (which excludes every other socket
    /// lock holder of an Internet socket, `soassertlocked`'s two cases).
    pub fn route_locked(&self) -> bool {
        self.socket().so_lock_legacy().lock().status() == RW_WRITE
            || hal::netlock::status() == RW_WRITE
    }

    /// `panic(9)` unless [`Inpcb::route_locked`].
    fn assert_route_locked(&self, what: &str) {
        if !self.route_locked() {
            kpanic(format_args!(
                "{what}: the socket lock of inpcb {self:p} is not held"
            ));
        }
    }

    /// `inp->inp_flags & bits`.
    #[inline]
    pub fn has_flags(&self, bits: i32) -> bool {
        self.inp_flags.get() & bits != 0
    }

    /// `inp->inp_flags |= bits`.
    #[inline]
    pub fn set_flags(&self, bits: i32) {
        self.inp_flags.set(self.inp_flags.get() | bits);
    }

    /// `inp->inp_flags &= ~bits`.
    #[inline]
    pub fn clear_flags(&self, bits: i32) {
        self.inp_flags.set(self.inp_flags.get() & !bits);
    }

    /// `inp_ip6_minhlim`: minimum Hop Limit or drop (the `inp_ip_minttl` member).
    #[inline]
    pub fn inp_ip6_minhlim(&self) -> &AtomicCell<u8> {
        &self.inp_ip_minttl
    }

    /// `inp_flowinfo` (`inp_ipv6.ip6_flow`): the flow information of the IPv6 prototype.
    pub fn inp_flowinfo(&self) -> u32 {
        self.inp_ipv6.0[0].load(Ordering::Relaxed)
    }

    /// `inp_flowinfo = flow`.
    pub fn set_inp_flowinfo(&self, flow: u32) {
        self.inp_ipv6.0[0].store(flow, Ordering::Relaxed);
    }

    /// `inp_upcall` and its argument, once set.
    #[inline]
    pub fn upcall(&self) -> Option<InpUpcall> {
        self.inp_upcall.get().copied()
    }

    /// `inp->inp_upcall = f; inp->inp_upcall_arg = arg` (wg(4)'s `wg_bind` on its new
    /// socket); `Err` when an upcall is set already.
    pub fn set_upcall(
        &self,
        f: &'static (dyn core::any::Any + Send + Sync),
        arg: usize,
    ) -> Result<(), Errno> {
        self.inp_upcall
            .set(InpUpcall { f, arg })
            .map_err(|_| Errno::EBUSY)
    }

    /// Whether `g` is the guard of this pcb's table: `panic(9)` otherwise (a setter's proof).
    fn check_guard(&self, g: &InpGuard<'_, G>) {
        if g.lock_id() != self.table().inpt_mtx.id() {
            kpanic(format_args!("inpcb {self:p}: another table's mutex"));
        }
    }

    /// `inp->inp_faddr = a`, the table mutex held.
    pub fn set_faddr(&self, g: &InpGuard<'_, G>, a: InAddr) {
        self.check_guard(g);
        self.inp_faddr.set(a);
    }

    /// `inp->inp_laddr = a`, the table mutex held.
    pub fn set_laddr(&self, g: &InpGuard<'_, G>, a: InAddr) {
        self.check_guard(g);
        self.inp_laddr.set(a);
    }

    /// `inp->inp_faddr6 = a`, the table mutex held.
    pub fn set_faddr6<A: Addr6>(&self, g: &InpGuard<'_, G>, a: A) {
        self.check_guard(g);
        self.inp_faddr6.set_bytes(a.to_bytes());
    }

    /// `inp->inp_laddr6 = a`, the table mutex held.
    pub fn set_laddr6<A: Addr6>(&self, g: &InpGuard<'_, G>, a: A) {
        self.check_guard(g);
        self.inp_laddr6.set_bytes(a.to_bytes());
    }

    /// `inp->inp_fport = p`, the table mutex held.
    pub fn set_fport(&self, g: &InpGuard<'_, G>, p: u16) {
        self.check_guard(g);
        self.inp_fport.set(p);
    }

    /// `inp->inp_lport = p`, the table mutex held.
    pub fn set_lport(&self, g: &InpGuard<'_, G>, p: u16) {
        self.check_guard(g);
        self.inp_lport.set(p);
    }

    /// `inp->inp_rtableid = r`, the table mutex held.
    pub fn set_rtableid(&self, g: &InpGuard<'_, G>, r: u32) {
        self.check_guard(g);
        self.inp_rtableid.set(r);
    }

    /// `in_pcbref(inp)` of a lookup's or a walk's answer, before `mtx_leave(&table->inpt_mtx)`
    /// (in_pcb.c:1220-1221, :1304-1305): the count `bsd` holds the answer by, taken with the
    /// table's guard `g` held. A pcb on the table's lists still holds its socket's count
    /// (`in_pcbdetach` takes it off them, under this mutex, before it drops that count), so
    /// the count is never taken from zero here; taken after the guard is let go, another
    /// CPU's detach and last `in_pcbunref` could run first. `panic(9)` for another table's
    /// guard.
    pub fn legacy_ref_locked(&self, g: &InpGuard<'_, G>) {
        self.check_guard(g);
        self.legacy_ref();
    }

    /// `refcnt_take(&inp->inp_refcnt)` of `bsd`'s holders: one more count of the C's, for a
    /// holder that has one already. The count is never taken from zero (`refcnt_take` of a
    /// freed pcb): `panic(9)`.
    pub fn legacy_ref(&self) {
        let old = self.inp_refcnt.fetch_add(1, Ordering::Relaxed);
        if old == 0 || old == u32::MAX {
            kpanic(format_args!("in_pcbref: inpcb {self:p} count {old}"));
        }
    }

    /// `refcnt_rele(&inp->inp_refcnt)` of `bsd`'s holders, with the key only `bsd` holds:
    /// one count fewer; the last gives back the anchor, the handle that kept the item, which
    /// the caller drops once nothing borrowed from the pcb is in use (its drop may free the
    /// pcb: `sorele` and `pool_put` of the C's last `in_pcbunref`).
    pub fn legacy_rele(&self, _key: &LegacyKey) -> Option<InpcbRef<G>> {
        self.rele()
    }

    /// One count of `inp_refcnt` fewer; the anchor back at zero.
    fn rele(&self) -> Option<InpcbRef<G>> {
        // Release: what this holder wrote happens before the last one's drop (its Acquire).
        let old = self.inp_refcnt.fetch_sub(1, Ordering::Release);
        match old {
            0 => kpanic(format_args!("in_pcbunref: inpcb {self:p} count 0")),
            1 => {
                core::sync::atomic::fence(Ordering::Acquire);
                self.inp_anchor.replace(None)
            }
            _ => None,
        }
    }

    /// `inp->inp_refcnt`, the count of `bsd`'s holders (a snapshot).
    pub fn legacy_count(&self) -> u32 {
        self.inp_refcnt.load(Ordering::Relaxed)
    }
}

/// A member of a pcb under its socket lock (the route cache, the multicast options), out of
/// its cell until this drops ([`Inpcb::route`], [`Inpcb::moptions`]): the cell holds the
/// member's default meanwhile, and the value goes back on drop.
pub struct InpLoan<'a, T: Default> {
    /// Where the value goes back.
    cell: &'a ReadMostly<T>,
    /// The value.
    val: T,
}

impl<'a, T: Default> InpLoan<'a, T> {
    /// The value of `cell`, taken out (the caller checked the socket lock).
    fn of(cell: &'a ReadMostly<T>) -> Self {
        Self {
            cell,
            val: cell.update(core::mem::take),
        }
    }
}

impl<T: Default> core::ops::Deref for InpLoan<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.val
    }
}

impl<T: Default> core::ops::DerefMut for InpLoan<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.val
    }
}

impl<T: Default> Drop for InpLoan<'_, T> {
    fn drop(&mut self) {
        drop(self.cell.replace(core::mem::take(&mut self.val)));
    }
}

/// `inp_mou`: the C's union of `inp_moptions` and `inp_moptions6`, tagged by family (a pcb is
/// one family for its life: `INP_IPV6` is set by `in_pcballoc`).
#[derive(Default)]
pub enum Mou {
    /// No multicast options (the C's NULL).
    #[default]
    None,
    /// An IPv4 pcb's `inp_moptions`.
    V4(MBox<IpMoptions>),
    /// An IPv6 pcb's `inp_moptions6`.
    V6(MBox<Ip6Moptions>),
}

/// The two members of [`Mou`]: what a [`MouLoan`] lends of the cell.
pub trait MouMember: Sized + 'static {
    /// The member's name, for the refusal of the other family's options.
    const NAME: &'static str;
    /// The member, out of the union: `Err` gives back the other family's options.
    fn of(m: Mou) -> Result<Option<MBox<Self>>, Mou>;
    /// The union holding the member.
    fn into_mou(v: Option<MBox<Self>>) -> Mou;
}

impl MouMember for IpMoptions {
    const NAME: &'static str = "inp_moptions";
    fn of(m: Mou) -> Result<Option<MBox<Self>>, Mou> {
        match m {
            Mou::None => Ok(None),
            Mou::V4(b) => Ok(Some(b)),
            other @ Mou::V6(_) => Err(other),
        }
    }
    fn into_mou(v: Option<MBox<Self>>) -> Mou {
        v.map_or(Mou::None, Mou::V4)
    }
}

impl MouMember for Ip6Moptions {
    const NAME: &'static str = "inp_moptions6";
    fn of(m: Mou) -> Result<Option<MBox<Self>>, Mou> {
        match m {
            Mou::None => Ok(None),
            Mou::V6(b) => Ok(Some(b)),
            other @ Mou::V4(_) => Err(other),
        }
    }
    fn into_mou(v: Option<MBox<Self>>) -> Mou {
        v.map_or(Mou::None, Mou::V6)
    }
}

/// One family's multicast options, out of the pcb's `inp_mou` until this drops
/// ([`Inpcb::moptions`], [`Inpcb::moptions6`]), as an [`InpLoan`]: the cell holds `None`
/// meanwhile, and the member goes back on drop. A loan of the other family's member is
/// `panic(9)` (the C's union read as the wrong member).
pub struct MouLoan<'a, T: MouMember> {
    /// Where the member goes back.
    cell: &'a ReadMostly<Mou>,
    /// The member.
    val: Option<MBox<T>>,
}

impl<'a, T: MouMember> MouLoan<'a, T> {
    /// The member of `cell`, taken out (the caller checked the socket lock).
    fn of(cell: &'a ReadMostly<Mou>) -> Self {
        match T::of(cell.update(core::mem::take)) {
            Ok(val) => Self { cell, val },
            Err(other) => {
                drop(cell.replace(other));
                kpanic(format_args!(
                    "{}: the multicast options of a pcb of the other family",
                    T::NAME
                ))
            }
        }
    }
}

impl<T: MouMember> core::ops::Deref for MouLoan<'_, T> {
    type Target = Option<MBox<T>>;
    fn deref(&self) -> &Option<MBox<T>> {
        &self.val
    }
}

impl<T: MouMember> core::ops::DerefMut for MouLoan<'_, T> {
    fn deref_mut(&mut self) -> &mut Option<MBox<T>> {
        &mut self.val
    }
}

impl<T: MouMember> Drop for MouLoan<'_, T> {
    fn drop(&mut self) {
        drop(self.cell.replace(T::into_mou(self.val.take())));
    }
}

/// The hash arrays of a table (`hashinit`'s two tables), out of the guard while a list
/// operation needs the guard as its witness.
pub struct Chains<G: ProcGlue> {
    /// `inpt_hashtbl`: local and foreign hash.
    hash: MArray<PinList<InpHash<G>>>,
    /// `inpt_lhashtbl`: local port hash.
    lhash: MArray<PinList<InpLhash<G>>>,
}

/// What `inpt_mtx` keeps of a table (\[t\]): the hash arrays, their masks and the counts.
pub struct InpTable<G: ProcGlue> {
    /// The hash arrays (`None` before [`in_pcbinit`], and while a call has them out).
    chains: Option<Chains<G>>,
    /// `inpt_mask`: hash mask.
    pub inpt_mask: u64,
    /// `inpt_lmask`: local port hash mask.
    pub inpt_lmask: u64,
    /// `inpt_count`: queue count.
    pub inpt_count: i32,
    /// `inpt_size`: hash size.
    pub inpt_size: i32,
}

/// `struct inpcbtable`.
pub struct Inpcbtable<G: ProcGlue> {
    /// `inpt_mtx`: protect queue and hash.
    inpt_mtx: Mutexed<InpTable<G>>,
    /// \[t\] `inpt_queue`: inet PCB queue.
    inpt_queue: PinList<InpQueue<G>>,
    /// \[I\] `inpt_key`: secret for the hash.
    pub inpt_key: AtomicSiphashKey,
    /// \[I\] `inpt_lkey`: secret for the local port hash.
    pub inpt_lkey: AtomicSiphashKey,
}

impl<G: ProcGlue> Inpcbtable<G> {
    /// An empty table; [`in_pcbinit`] sets it up.
    pub const fn new() -> Self {
        Self {
            inpt_mtx: Mutexed::new(
                IPL_SOFTNET,
                InpTable {
                    chains: None,
                    inpt_mask: 0,
                    inpt_lmask: 0,
                    inpt_count: 0,
                    inpt_size: 0,
                },
            ),
            inpt_queue: PinList::new(),
            inpt_key: AtomicSiphashKey::zeroed(),
            inpt_lkey: AtomicSiphashKey::zeroed(),
        }
    }

    /// `mtx_enter(&table->inpt_mtx)`.
    #[inline]
    pub fn lock(&self) -> InpGuard<'_, G> {
        self.inpt_mtx.lock()
    }

    /// `inpt_count`, read under the table mutex.
    pub fn count(&self) -> i32 {
        self.lock().inpt_count
    }

    /// `inpt_size`, read under the table mutex.
    pub fn size(&self) -> i32 {
        self.lock().inpt_size
    }

    /// `inpt_queue`, read under `g`: the control blocks and the iterators' markers.
    pub fn queue(&self) -> &PinList<InpQueue<G>> {
        &self.inpt_queue
    }
}

impl<G: ProcGlue> Default for Inpcbtable<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The "inpcb" pool of a glue (`in_pcb.c`'s `inpcb_pool`), in its `NetTables`.
pub struct InpcbTables<G: ProcGlue> {
    /// `inpcb_pool`.
    pool: TypedPool<Inpcb<G>>,
}

impl<G: ProcGlue> InpcbTables<G> {
    /// The pool, before [`in_init`].
    pub const fn new() -> Self {
        Self {
            pool: TypedPool::new(),
        }
    }
}

impl<G: ProcGlue> Default for InpcbTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct baddynamicports`: bitmaps of ports, one bit per port, for TCP and UDP.
pub struct Baddynamicports {
    /// `tcp`.
    pub tcp: [AtomicU32; DP_MAPSIZE],
    /// `udp`.
    pub udp: [AtomicU32; DP_MAPSIZE],
}

impl Baddynamicports {
    /// Empty maps (the C's zeroed globals).
    pub const fn new() -> Self {
        Self {
            tcp: [const { AtomicU32::new(0) }; DP_MAPSIZE],
            udp: [const { AtomicU32::new(0) }; DP_MAPSIZE],
        }
    }
}

impl Default for Baddynamicports {
    fn default() -> Self {
        Self::new()
    }
}

/// `zeroin_addr`.
pub static ZEROIN_ADDR: InAddr = InAddr { s_addr: 0 };
/// `zeroin46_addr`: the zero address of either family.
pub static ZEROIN46_ADDR: Inpaddru = Inpaddru::from_bytes6([0; 16]);

// These configure the range of local port addresses assigned to "unspecified" outgoing
// connections/packets/whatever.

/// \[a\] `ipport_firstauto`.
pub static IPPORT_FIRSTAUTO: AtomicI32 = AtomicI32::new(IPPORT_RESERVED);
/// \[a\] `ipport_lastauto`.
pub static IPPORT_LASTAUTO: AtomicI32 = AtomicI32::new(IPPORT_USERRESERVED);
/// \[a\] `ipport_hifirstauto`.
#[allow(non_upper_case_globals)] // the C name; `IPPORT_HIFIRSTAUTO` is `in.h`'s default
pub static ipport_hifirstauto: AtomicI32 = AtomicI32::new(IPPORT_HIFIRSTAUTO);
/// \[a\] `ipport_hilastauto`.
#[allow(non_upper_case_globals)] // the C name; `IPPORT_HILASTAUTO` is `in.h`'s default
pub static ipport_hilastauto: AtomicI32 = AtomicI32::new(IPPORT_HILASTAUTO);

/// `baddynamicports`: ports not to allocate dynamically.
pub static BADDYNAMICPORTS: Baddynamicports = Baddynamicports::new();
/// `rootonlyports`: ports only root can bind to.
pub static ROOTONLYPORTS: Baddynamicports = Baddynamicports::new();

/// `in_pcb_is_iterator(inp)`.
#[inline]
pub fn in_pcb_is_iterator<G: ProcGlue>(inp: &Inpcb<G>) -> bool {
    inp.inp_table.is_none()
}

/// `DP_SET(m, p)`.
pub fn dp_set(m: &[AtomicU32; DP_MAPSIZE], p: u16) {
    let p = usize::from(p);
    m[p / DP_MAPBITS].fetch_or(1 << (p % DP_MAPBITS), Ordering::Relaxed);
}

/// `DP_CLR(m, p)`.
pub fn dp_clr(m: &[AtomicU32; DP_MAPSIZE], p: u16) {
    let p = usize::from(p);
    m[p / DP_MAPBITS].fetch_and(!(1 << (p % DP_MAPBITS)), Ordering::Relaxed);
}

/// `DP_ISSET(m, p)`.
pub fn dp_isset(m: &[AtomicU32; DP_MAPSIZE], p: u16) -> bool {
    let p = usize::from(p);
    m[p / DP_MAPBITS].load(Ordering::Relaxed) & (1 << (p % DP_MAPBITS)) != 0
}

/// A counted handle to `inp` (the C's `in_pcbref` of a pcb a list or a caller lends):
/// `panic(9)` for an iterator's marker, which no pool placed.
fn href<G: ProcGlue>(inp: &Inpcb<G>) -> InpcbRef<G> {
    match PoolArc::from_ref(inp) {
        Some(h) => h,
        None => kpanic(format_args!("inpcb {inp:p}: not a pool item")),
    }
}

/// Runs `f` with the table's hash arrays out of the guard, so the lists can take the guard
/// as their witness while the arrays are borrowed; puts them back.
fn with_chains<'a, G: ProcGlue, R>(
    g: &mut InpGuard<'a, G>,
    f: impl FnOnce(&Chains<G>, &mut InpGuard<'a, G>) -> R,
) -> R {
    let Some(c) = g.chains.take() else {
        kpanic(format_args!("inpcb table: no hash tables"));
    };
    let r = f(&c, g);
    g.chains = Some(c);
    r
}

/// `in_init`: the `inpcb` pool. `in_pcb` is used for inet and inet6, `in6_pcb` only contains
/// special IPv6 cases, so the internet initializer is used for both domains.
pub fn in_init<G: ProcGlue>() {
    let _ = crate::net::domain::net::<G>()
        .inpcb
        .pool
        .init_recorded(IPL_SOFTNET, 0, "inpcb");
}

/// `in_pcbhash`: the hash of the complete address quadruple in routing domain `rdomain`.
#[inline]
pub fn in_pcbhash<G: ProcGlue>(
    table: &Inpcbtable<G>,
    rdomain: u32,
    faddr: &InAddr,
    fport: u16,
    laddr: &InAddr,
    lport: u16,
) -> u64 {
    let mut ctx = table.inpt_key.siphash24();
    ctx.update(&rdomain.to_be_bytes());
    ctx.update(&faddr.s_addr.to_ne_bytes());
    ctx.update(&fport.to_ne_bytes());
    ctx.update(&laddr.s_addr.to_ne_bytes());
    ctx.update(&lport.to_ne_bytes());
    ctx.end()
}

/// `in_pcblhash`: the hash of local port `lport` in routing domain `rdomain`.
#[inline]
pub fn in_pcblhash<G: ProcGlue>(table: &Inpcbtable<G>, rdomain: u32, lport: u16) -> u64 {
    let mut ctx = table.inpt_lkey.siphash24();
    ctx.update(&rdomain.to_be_bytes());
    ctx.update(&lport.to_ne_bytes());
    ctx.end()
}

/// Stores a fresh random hash key in `key` (`arc4random_buf(&key, sizeof(key))`).
fn random_key(key: &AtomicSiphashKey) {
    key.store(siphash_key());
}

/// `in_pcbinit`: sets up `table` with hash tables of `hashsize` buckets and fresh keys.
pub fn in_pcbinit<G: ProcGlue>(table: &Inpcbtable<G>, hashsize: i32) {
    let (Some((hash, mask)), Some((lhash, lmask))) = (
        hashinit::<InpHash<G>>(hashsize, M_PCB, M_WAITOK),
        hashinit::<InpLhash<G>>(hashsize, M_PCB, M_WAITOK),
    ) else {
        kpanic(format_args!("in_pcbinit: hashinit"));
    };
    let mut g = table.lock();
    // A table set up again (the host tests' fresh setup over a table of an earlier test):
    // the old chains and queue forgotten, never freed (their pcbs are in that test's memory).
    while table.inpt_queue.remove_first(&mut g) {}
    core::mem::forget(g.chains.replace(Chains { hash, lhash }));
    g.inpt_mask = mask;
    g.inpt_lmask = lmask;
    g.inpt_count = 0;
    g.inpt_size = hashsize;
    drop(g);
    random_key(&table.inpt_key);
    random_key(&table.inpt_lkey);
}

/// `in_baddynamic`: whether `port` is invalid for dynamic allocation.
pub fn in_baddynamic(port: u16, proto: u16) -> bool {
    match i32::from(proto) {
        IPPROTO_TCP => dp_isset(&BADDYNAMICPORTS.tcp, port),
        IPPROTO_UDP => {
            // Cannot preset this as it is a sysctl
            i32::from(port) == UDPENCAP_PORT.load(Ordering::Relaxed)
                || dp_isset(&BADDYNAMICPORTS.udp, port)
        }
        _ => false,
    }
}

/// `in_rootonly`: whether only root may bind `port`.
pub fn in_rootonly(port: u16, proto: u16) -> bool {
    match i32::from(proto) {
        IPPROTO_TCP => i32::from(port) < IPPORT_RESERVED || dp_isset(&ROOTONLYPORTS.tcp, port),
        IPPROTO_UDP => i32::from(port) < IPPORT_RESERVED || dp_isset(&ROOTONLYPORTS.udp, port),
        _ => false,
    }
}

/// `in_pcballoc`: a new control block for `so` in `table`, its address in the socket's
/// `so_pcb` and its first reference the socket's (`inp_refcnt` 1, the anchor); the handle
/// returned is the caller's.
pub fn in_pcballoc<G: ProcGlue>(
    so: &Socket<G>,
    table: &'static Inpcbtable<G>,
    wait: i32,
) -> Result<InpcbRef<G>, Errno> {
    let pool = &crate::net::domain::net::<G>().inpcb.pool;
    let flags = if wait == M_WAIT { PR_WAITOK } else { PR_NOWAIT };
    let Some(inp) = pool.get_homed(flags, || Inpcb::new(Some(table), Some(soref(so)))) else {
        return Err(Errno::ENOBUFS);
    };
    // refcnt_init: the socket's reference, kept by the anchor until `in_pcbdetach`.
    inp.inp_refcnt.store(1, Ordering::Relaxed);
    drop(inp.inp_anchor.replace(Some(inp.clone())));
    inp.inp_seclevel.set(IpsecLevel {
        sl_auth: IPSEC_AUTH_LEVEL_DEFAULT as u8,
        sl_esp_trans: IPSEC_ESP_TRANS_LEVEL_DEFAULT as u8,
        sl_esp_network: IPSEC_ESP_NETWORK_LEVEL_DEFAULT as u8,
        sl_ipcomp: IPSEC_IPCOMP_LEVEL_DEFAULT as u8,
    });
    let rtableid = G::with_curproc(|p| p.map(|p| p.process().ps_rtableid.load(Ordering::Relaxed)));
    let Some(rtableid) = rtableid else {
        kpanic(format_args!("in_pcballoc: no curproc"));
    };
    inp.inp_rtableid.set(rtableid);
    inp.inp_hops.set(-1);
    match so.dom_family() {
        f if f == i32::from(hal::sys::socket::PF_INET6) => inp.inp_flags.set(INP_IPV6),
        f if f == i32::from(hal::sys::socket::PF_INET) => {} // inp_flags is initialized to 0
        f => kpanic(format_args!("in_pcballoc: unhandled af {f}")),
    }
    inp.inp_cksum6.set(-1);

    let mut g = table.lock();
    let count = g.inpt_count;
    g.inpt_count = count + 1;
    if count > inpcbhash_loadfactor(g.inpt_size) {
        let size = g.inpt_size * 2;
        let _ = in_pcbresize(&mut g, size);
    }
    table.inpt_queue.insert_head(&mut g, inp.as_pin());
    in_pcbhash_insert(&mut g, &inp);
    drop(g);

    so.set_legacy_pcb(core::ptr::from_ref::<Inpcb<G>>(&inp).expose_provenance());
    // so->so_pcb of a kernel protocol's socket, counted, for sotoinpcb (N6.6a4).
    drop(so.set_inet_pcb(Some(PoolArc::erase(inp.clone()))));

    Ok(inp)
}

/// `in_pcbbind_locked`: binds `inp` to `nam` (or to `laddr` and a port picked here when `nam`
/// is `None`), the table mutex held (`g`); an IPv6 pcb's address checked by
/// `in6_pcbaddrisavail` (the C's `#ifdef INET6` branch).
pub fn in_pcbbind_locked<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    inp: &Inpcb<G>,
    nam: Option<&Mbuf>,
    laddr: &Inpaddru,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let so = inp.socket();
    let mut lport: u16 = 0;
    let mut wild = 0;
    let mut laddr = *laddr;

    if inp.inp_lport.get() != 0 {
        return Err(Errno::EINVAL);
    }

    if !so.has_options(SO_REUSEADDR | SO_REUSEPORT)
        && (!so.pr_flags(PR_CONNREQUIRED) || !so.has_options(SO_ACCEPTCONN))
    {
        wild = INPLOOKUP_WILDCARD;
    }

    let inet6 = inp.has_flags(INP_IPV6);
    if inet6 {
        if !in6_is_addr_unspecified(&inp.inp_laddr6.get::<In6Addr>()) {
            return Err(Errno::EINVAL);
        }
        wild |= INPLOOKUP_IPV6;

        if let Some(nam) = nam {
            let mut sin6 = in6_nam2sin6(nam)?;
            in6_pcbaddrisavail_held(g, inp, &mut sin6, wild, net)?;
            laddr = Inpaddru::from_addr6(sin6.sin6_addr);
            lport = sin6.sin6_port;
        }
    } else {
        if inp.inp_laddr.get().s_addr != INADDR_ANY {
            return Err(Errno::EINVAL);
        }

        if let Some(nam) = nam {
            let mut sin = in_nam2sin(nam)?;
            in_pcbaddrisavail_held(g, inp, &mut sin, wild, net)?;
            laddr = Inpaddru::from_addr(sin.sin_addr);
            lport = sin.sin_port;
        }
    }

    if lport == 0 {
        in_pcbpickport(g, &mut lport, &laddr, wild, inp, p)?;
    } else if in_rootonly(u16::from_be(lport), so.proto().protocol() as u16) && suser(p).is_err() {
        return Err(Errno::EACCES);
    }
    if nam.is_some() {
        if inet6 {
            inp.set_laddr6(g, laddr.bytes6());
        } else {
            inp.set_laddr(g, laddr.iau_addr());
        }
    }
    inp.set_lport(g, lport);
    in_pcbrehash(g, inp);

    Ok(())
}

/// `in_pcbbind`: binds `inp` to `nam`, or to a port picked here.
pub fn in_pcbbind<G: ProcGlue>(
    inp: &Inpcb<G>,
    nam: Option<&Mbuf>,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    // keep lookup, modification, and rehash in sync
    let mut g = inp.table().lock();
    in_pcbbind_locked(&mut g, inp, nam, &ZEROIN46_ADDR, p, net)
}

/// `in_pcbaddrisavail_lock(.., IN_PCBLOCK_HOLD)`: whether `inp` may bind to `sin` (an
/// address of ours and a port nobody else has, as `wild` and the reuse options allow), the
/// table mutex held.
pub fn in_pcbaddrisavail_held<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    inp: &Inpcb<G>,
    sin: &mut SockaddrIn,
    wild: i32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let so = inp.socket();
    let lport = sin.sin_port;
    let mut reuseport = so.options() & SO_REUSEPORT;

    if in_multicast(sin.sin_addr.s_addr) {
        // Treat SO_REUSEADDR as SO_REUSEPORT for multicast; allow complete duplication of
        // binding if SO_REUSEPORT is set, or if SO_REUSEADDR is set and a multicast address is
        // bound on both new and duplicated sockets.
        if so.has_options(SO_REUSEADDR | SO_REUSEPORT) {
            reuseport = SO_REUSEADDR | SO_REUSEPORT;
        }
    } else if sin.sin_addr.s_addr != INADDR_ANY {
        // we must check that we are binding to an address we own except when:
        // - SO_BINDANY is set or
        // - we are binding a UDP socket to 255.255.255.255 or
        // - we are binding a UDP socket to one of our broadcast addresses
        if !(so.has_options(SO_BINDANY)
            || (so.so_type() == SOCK_DGRAM && sin.sin_addr.s_addr == INADDR_BROADCAST)
            || (so.so_type() == SOCK_DGRAM
                && in_broadcast(sin.sin_addr, inp.inp_rtableid.get(), net)))
        {
            sin.sin_port = 0;
            sin.sin_zero = [0; 8];
            let found = ifa_ifwithaddr(
                net,
                &net.reader(),
                sin_bytes(sin).as_slice(),
                inp.inp_rtableid.get(),
            )
            .is_some();
            sin.sin_port = lport;

            if !found {
                return Err(Errno::EADDRNOTAVAIL);
            }
        }
    }
    if lport != 0 {
        let laddr = Inpaddru::from_addr(sin.sin_addr);
        if so.creds().0 != 0 && !in_multicast(sin.sin_addr.s_addr) {
            let t = in_pcblookup_local(
                g,
                inp.table(),
                &laddr,
                lport,
                INPLOOKUP_WILDCARD,
                inp.inp_rtableid.get(),
            );
            if t.is_some_and(|t| so.creds().0 != t.socket().creds().0) {
                return Err(Errno::EADDRINUSE);
            }
        }
        let t = in_pcblookup_local(g, inp.table(), &laddr, lport, wild, inp.inp_rtableid.get());
        if t.is_some_and(|t| reuseport & t.socket().options() == 0) {
            return Err(Errno::EADDRINUSE);
        }
    }

    Ok(())
}

/// `in_pcbaddrisavail`: [`in_pcbaddrisavail_held`] taking the table mutex
/// (`IN_PCBLOCK_GRAB`).
pub fn in_pcbaddrisavail<G: ProcGlue>(
    inp: &Inpcb<G>,
    sin: &mut SockaddrIn,
    wild: i32,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let mut g = inp.table().lock();
    in_pcbaddrisavail_held(&mut g, inp, sin, wild, net)
}

/// The bytes of a `struct sockaddr_in` (`sintosa`), as the address lookups take them.
fn sin_bytes(sin: &SockaddrIn) -> [u8; 16] {
    let mut b = [0; 16];
    b[0] = sin.sin_len;
    b[1] = sin.sin_family;
    b[2..4].copy_from_slice(&sin.sin_port.to_ne_bytes());
    b[4..8].copy_from_slice(&sin.sin_addr.s_addr.to_ne_bytes());
    for (d, z) in b[8..16].iter_mut().zip(sin.sin_zero) {
        *d = z as u8;
    }
    b
}

/// `in_pcbpickport`: a free local port for `inp` from the range its flags and the sysctls
/// give, into `lport` (network order), the table mutex held.
pub fn in_pcbpickport<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    lport: &mut u16,
    laddr: &Inpaddru,
    wild: i32,
    inp: &Inpcb<G>,
    p: &Thread<G>,
) -> Result<(), Errno> {
    let so = inp.socket();

    let (first, last): (u16, u16) = if inp.has_flags(INP_HIGHPORT) {
        (
            ipport_hifirstauto.load(Ordering::Relaxed) as u16, // sysctl
            ipport_hilastauto.load(Ordering::Relaxed) as u16,
        )
    } else if inp.has_flags(INP_LOWPORT) {
        if suser(p).is_err() {
            return Err(Errno::EACCES);
        }
        (IPPORT_RESERVED as u16 - 1, 600) // 1023; not IPPORT_RESERVED/2
    } else {
        (
            IPPORT_FIRSTAUTO.load(Ordering::Relaxed) as u16, // sysctl
            IPPORT_LASTAUTO.load(Ordering::Relaxed) as u16,
        )
    };
    let (lower, higher) = if first < last {
        (first, last)
    } else {
        (last, first)
    };

    // Simple check to ensure all ports are not used up causing a deadlock here.
    let mut count = i32::from(higher - lower);
    let mut candidate = lower.wrapping_add(hal::random::arc4random_uniform(count as u32) as u16);

    let localport = loop {
        let localport = loop {
            if count < 0 {
                // completely used?
                return Err(Errno::EADDRNOTAVAIL);
            }
            count -= 1;
            candidate = candidate.wrapping_add(1);
            if candidate < lower || candidate > higher {
                candidate = lower;
            }
            if !in_baddynamic(candidate, so.proto().protocol() as u16) {
                break candidate.to_be();
            }
        };
        if in_pcblookup_local(
            g,
            inp.table(),
            laddr,
            localport,
            wild,
            inp.inp_rtableid.get(),
        )
        .is_none()
        {
            break localport;
        }
    };
    *lport = localport;

    Ok(())
}

/// `in_pcbconnect`: connects `inp` to the address in `nam`; both address and port must be
/// specified. If the socket has no local address yet, one is picked. An IPv6 pcb is
/// `in6_pcbconnect`'s (the C's `#ifdef INET6` dispatch).
pub fn in_pcbconnect<G: ProcGlue>(
    inp: &Inpcb<G>,
    nam: &Mbuf,
    p: &Thread<G>,
    net: &NetHeld<'_>,
) -> Result<(), Errno> {
    let table = inp.table();

    if inp.has_flags(INP_IPV6) {
        return in6_pcbconnect(inp, nam, p, net);
    }
    let sin = in_nam2sin(nam)?;
    if sin.sin_port == 0 {
        return Err(Errno::EADDRNOTAVAIL);
    }
    let ina = in_pcbselsrc(&sin, inp, net)?;

    // keep lookup, modification, and rehash in sync
    let mut g = table.lock();

    let rdomain = rtable_l2(inp.inp_rtableid.get());
    let found = |g: &mut InpGuard<'_, G>, lport: u16| {
        let hash = in_pcbhash(table, rdomain, &sin.sin_addr, sin.sin_port, &ina, lport);
        in_pcbhash_lookup(g, hash, rdomain, &sin.sin_addr, sin.sin_port, &ina, lport).is_some()
    };
    if found(&mut g, inp.inp_lport.get()) {
        return Err(Errno::EADDRINUSE);
    }

    kassert!(inp.inp_laddr.get().s_addr == INADDR_ANY || inp.inp_lport.get() != 0);

    if inp.inp_laddr.get().s_addr == INADDR_ANY {
        if inp.inp_lport.get() == 0 {
            in_pcbbind_locked(&mut g, inp, None, &Inpaddru::from_addr(ina), p, net)?;
            if found(&mut g, inp.inp_lport.get()) {
                inp.set_lport(&g, 0);
                return Err(Errno::EADDRINUSE);
            }
        }
        inp.set_laddr(&g, ina);
    }
    inp.set_faddr(&g, sin.sin_addr);
    inp.set_fport(&g, sin.sin_port);
    in_pcbrehash(&mut g, inp);
    drop(g);

    in_pcb_newflow(inp);
    Ok(())
}

/// `NSTOEPLITZ`: the flow id of a connected or accepted IPv4 pcb from its addresses and ports
/// (the tail `in_pcbconnect` and `in_pcbset_addr` share), after the table mutex as the C.
fn in_pcb_newflow<G: ProcGlue>(inp: &Inpcb<G>) {
    inp.inp_flowid.set(stoeplitz_ip4port(
        inp.inp_faddr.get().s_addr,
        inp.inp_laddr.get().s_addr,
        inp.inp_fport.get(),
        inp.inp_lport.get(),
    ));
}

/// `in_pcbdisconnect`: forgets the flow; `true` when the socket has no file reference any
/// more, so the caller detaches the control block (`in_pcbdetach`), as the C does next.
pub fn in_pcbdisconnect<G: ProcGlue>(inp: &Inpcb<G>) -> bool {
    if let Some(pf) = pf() {
        pf.inp_remove_divert_state(&inp.inp_pf_sk);
        pf.inp_unlink(&inp.inp_pf_sk);
    }
    inp.inp_flowid.set(0);
    inp.socket().has_state(SS_NOFDREF)
}

/// `in_pcbdetach`: unlinks `inp` from its socket and its table and drops the socket's
/// reference; `free_opts` frees what `bsd` keeps of an IPv4 pcb (its IPv4 options), at the
/// C's point, before the multicast options; an IPv6 pcb's packet and multicast options are
/// freed here (`ip6_freepcbopts`, `ip6_freemoptions`).
pub fn in_pcbdetach<G: ProcGlue>(inp: InpcbRef<G>, net: &NetHeld<'_>) {
    let so = inp.socket();
    let table = inp.table();

    // soassertlocked(so) (in_pcb.c:589), before anything changes.
    inp.assert_route_locked("in_pcbdetach");
    so.set_legacy_pcb(0);
    // so->so_pcb = NULL of a kernel protocol's socket: its count of the pcb given back (the
    // caller's handle keeps the pcb), before sofree looks at it.
    drop(so.set_inet_pcb(None));
    sofree_legacy(so);
    // rtfree(inp->inp_route.ro_rt): the cache's handle dropped.
    drop(inp.route().take_rt());
    if inp.has_flags(INP_IPV6) {
        ip6_freepcbopts(core::mem::take(&mut *inp.outputopts6()));
        ip6_freemoptions(core::mem::take(&mut *inp.moptions6()), net);
    } else {
        // m_freem(inp->inp_options): the chain dropped with its box.
        drop(core::mem::take(&mut *inp.options()));
        ip_freemoptions(core::mem::take(&mut *inp.moptions()), net);
    }
    if let Some(pf) = pf() {
        pf.inp_remove_divert_state(&inp.inp_pf_sk);
        pf.inp_unlink(&inp.inp_pf_sk);
    }
    let mut g = table.lock();
    with_chains(&mut g, |c, g| {
        c.lhash[slot_of(&inp.inp_lslot)].remove(g, &inp);
        c.hash[slot_of(&inp.inp_hslot)].remove(g, &inp);
    });
    table.inpt_queue.remove(&mut g, &inp);
    g.inpt_count -= 1;
    drop(g);

    let anchor = inp.rele();
    drop(anchor);
    drop(inp);
}

/// `in_pcbsolock(inp)`: the socket of `inp` locked under the caller's net lock (`net`, either
/// mode); `None` when it is detached (a close ran first: its socket has no pcb any more). The
/// lock is `so_lock`, taken as the socket's protocol takes it (`solock_net`): a witness for
/// the kernel's protocols, whose `[s]` cells [`SoLocked::inet`] opens, a detached holding for
/// `bsd`'s. Its drop is `in_pcbsounlock`. Lock order: the net lock, then `so_lock`; the
/// caller holds no other socket's lock and not this one's (a second holding would wait for
/// itself, N6.6a0's S2).
pub fn in_pcbsolock<'a, G: ProcGlue>(
    inp: &'a Inpcb<G>,
    net: &'a NetHeld<'a>,
) -> Option<SoLocked<'a>> {
    let so = inp.socket_opt()?;
    let w = solock_net(so, net);
    if so.legacy_pcb() == 0 {
        drop(w);
        return None;
    }
    kassert!(so.legacy_pcb() == core::ptr::from_ref(inp).addr());
    Some(w)
}

/// `in_pcbref(sotoinpcb(so))` of a kernel protocol's socket: its pcb (`so_pcb`, the slot
/// [`in_pcballoc`] fills and `in_pcbdetach` empties), counted for the caller; `None` once
/// detached, and for a `bsd` protocol's socket (whose `so_pcb` is an address only). It takes
/// no witness and counts the handle (brief 3.2 had a witness and no count): the C's
/// `soassertlocked` is the caller's, who holds the socket's lock so the slot does not change
/// meanwhile, and the count is one atomic increment per call: once per connection in
/// `syn_cache_get`, never per segment, whose handle is the lookup's. TCP's user requests
/// borrow the pcb instead ([`sotoinpcb_locked`], N6.6a5).
pub fn sotoinpcb<G: ProcGlue>(so: &Socket<G>) -> Option<InpcbRef<G>> {
    let pcb = so.pcb_read();
    pcb.as_ref()?
        .downcast_ref::<Inpcb<G>>()
        .and_then(PoolArc::from_ref)
}

/// `sotoinpcb(so)` of a holder of the socket's lock (TCP's user requests, N6.6a5): the pcb
/// borrowed from the socket's locked slot ([`Socket::pcb_locked`]) for as long as `pcb`, the
/// lock's part `PCB`, is borrowed: the C's load of `so_pcb`, no count taken (the slot keeps
/// the pcb, and its writer needs the part mutably). `None` before the protocol's attach and
/// after its close (`tcp_free`), and for a socket whose slot another type fills.
#[inline]
pub fn sotoinpcb_locked<'a, G: ProcGlue>(
    so: &'a Socket<G>,
    pcb: &'a WitnessPart<'_>,
) -> Option<&'a Inpcb<G>> {
    so.pcb_locked(pcb)?.downcast_ref::<Inpcb<G>>()
}

/// `in_pcbsolock` held C-style, for `bsd`'s adapter (its protocols release the lock in
/// another function, `in_pcbsounlock`): `so_lock` by a detached holding, the caller holding
/// the net lock (checked: `panic(9)` without it); `None` when the socket is detached. Until
/// the other transports move (N6.6b; TCP's are the kernel's since N6.6a5).
pub fn in_pcbsolock_detached<G: ProcGlue>(inp: &Inpcb<G>) -> Option<&Socket<G>> {
    // NET_ASSERT_LOCKED(): this process's write holding or a read holding.
    if !matches!(hal::netlock::status(), RW_WRITE | hal::rwlock::RW_READ) {
        kpanic(format_args!(
            "in_pcbsolock: inpcb {inp:p}: the net lock is not held"
        ));
    }
    let so = inp.socket_opt()?;
    so.so_lock_legacy().enter_write_detached();
    if so.legacy_pcb() == 0 {
        so.so_lock_legacy().exit_write_detached();
        return None;
    }
    kassert!(so.legacy_pcb() == core::ptr::from_ref(inp).addr());
    Some(so)
}

/// `in_pcbsounlock` of what [`in_pcbsolock_detached`] locked.
pub fn in_pcbsounlock_detached<G: ProcGlue>(inp: Option<&Inpcb<G>>, so: Option<&Socket<G>>) {
    let Some(so) = so else {
        return;
    };
    if let Some(inp) = inp
        && so.legacy_pcb() != 0
    {
        kassert!(so.legacy_pcb() == core::ptr::from_ref(inp).addr());
    }
    so.so_lock_legacy().exit_write_detached();
}

/// `in_pcb_iterator`: the control block after `iter`'s place (the first one when the walk
/// starts, `iter` on no list) in `table`'s queue, counted, with `iter` moved after it; the
/// previous answer `prev` is dropped here, under the table mutex (`g`), as the C's
/// `in_pcbunref`. `None` ends the walk, `iter` out of the queue. `iter` is
/// [`Inpcb::marker`]'s, pinned by the walker for this walk.
pub fn in_pcb_iterator<G: ProcGlue>(
    table: &Inpcbtable<G>,
    g: &mut InpGuard<'_, G>,
    prev: Option<InpcbRef<G>>,
    iter: Pin<&Inpcb<G>>,
) -> Option<InpcbRef<G>> {
    kassert!(in_pcb_is_iterator(iter.get_ref()));
    let q = &table.inpt_queue;
    // The marker is on the queue once the walk has answered (the C's `inp != NULL`).
    let linked = iter.inp_queue.is_linked();
    let next = if linked {
        // The marker steps over the other walks' markers after it, then past the next pcb.
        loop {
            match q.next(&*g, iter.get_ref()) {
                None => break None,
                Some(t) if in_pcb_is_iterator(t) => {
                    q.step_over(g, iter.get_ref());
                }
                Some(t) => {
                    let t = href(t);
                    q.step_over(g, iter.get_ref());
                    break Some(t);
                }
            }
        }
    } else {
        let t = q.iter(&*g).find(|t| !in_pcb_is_iterator(t)).map(href);
        if let Some(t) = &t {
            q.insert_after(g, t, iter);
        }
        t
    };
    if next.is_none() && linked {
        q.remove(g, iter.get_ref());
    }
    drop(prev);
    next
}

/// `in_pcb_iterator_abort`: ends a walk before [`in_pcb_iterator`] returned `None`: the
/// marker out of the queue (if it is on it), the last answer dropped under the mutex.
pub fn in_pcb_iterator_abort<G: ProcGlue>(
    table: &Inpcbtable<G>,
    g: &mut InpGuard<'_, G>,
    prev: Option<InpcbRef<G>>,
    iter: Pin<&Inpcb<G>>,
) {
    if iter.inp_queue.is_linked() {
        table.inpt_queue.remove(g, iter.get_ref());
    }
    drop(prev);
}

/// `in_setsockaddr` of an IPv4 control block: its local address, as the `struct
/// sockaddr_in` the caller writes into the name mbuf.
pub fn in_setsockaddr<G: ProcGlue>(inp: &Inpcb<G>) -> SockaddrIn {
    SockaddrIn {
        sin_family: AF_INET,
        sin_len: core::mem::size_of::<SockaddrIn>() as u8,
        sin_port: inp.inp_lport.get(),
        sin_addr: inp.inp_laddr.get(),
        ..SockaddrIn::default()
    }
}

/// `in_setpeeraddr` of an IPv4 control block: its foreign address, as [`in_setsockaddr`].
pub fn in_setpeeraddr<G: ProcGlue>(inp: &Inpcb<G>) -> SockaddrIn {
    SockaddrIn {
        sin_family: AF_INET,
        sin_len: core::mem::size_of::<SockaddrIn>() as u8,
        sin_port: inp.inp_fport.get(),
        sin_addr: inp.inp_faddr.get(),
        ..SockaddrIn::default()
    }
}

/// `in_pcbnotifyall`: passes some notification to all connections of a protocol associated
/// with address `dst`. The "usual action" will be taken, depending on the ctlinput cmd. The
/// caller must filter any cmds that are uninteresting (e.g., no error in the map). Calls the
/// protocol specific routine `notify` to report any errors for each matching socket, with
/// the socket locked (`in_pcbsolock` under the caller's net lock, `net`; its witness lent to
/// `notify`).
pub fn in_pcbnotifyall<G: ProcGlue>(
    table: &Inpcbtable<G>,
    dst: &SockaddrIn,
    rtable: u32,
    errno: Option<Errno>,
    net: &NetHeld<'_>,
    mut notify: impl FnMut(&Inpcb<G>, &mut SoLocked<'_>, Option<Errno>),
) {
    if dst.sin_addr.s_addr == INADDR_ANY {
        return;
    }

    let iter = core::pin::pin!(Inpcb::<G>::marker());
    let mut inp: Option<InpcbRef<G>> = None;
    let rdomain = rtable_l2(rtable);
    let mut g = table.lock();
    while let Some(i) = in_pcb_iterator(table, &mut g, inp.take(), iter.as_ref()) {
        kassert!(!i.has_flags(INP_IPV6));

        if i.inp_faddr.get().s_addr != dst.sin_addr.s_addr
            || rtable_l2(i.inp_rtableid.get()) != rdomain
        {
            inp = Some(i);
            continue;
        }
        drop(g);
        if let Some(mut w) = in_pcbsolock(&i, net) {
            notify(&i, &mut w, errno);
        }
        g = table.lock();
        inp = Some(i);
    }
    drop(g);
}

/// `in_losing`: checks for alternatives when higher level complains about service problems.
/// For now, invalidate cached routing information. If the route was created dynamically (by
/// a redirect), time to try a default gateway again.
pub fn in_losing<G: ProcGlue>(inp: &Inpcb<G>, net: &NetHeld<'_>) {
    let Some(rt) = inp.route().take_rt() else {
        return;
    };

    if rt.rt_flags.get() & RTF_DYNAMIC != 0 {
        // If the interface is gone, all its attached route entries have been removed from
        // the table, so we're dealing with a stale cache and have nothing to do.
        if let Some(ifp) = if_get(rt.rt_ifidx.get()) {
            let _ = rtdeletemsg(&rt, ifp, inp.inp_rtableid.get(), net);
        }
    }
    // A new route can be allocated the next time output is attempted. rtfree() needs to be
    // called in anycase because the inp is still holding a reference to rt: the handle's
    // drop.
    drop(rt);
}

/// `in_pcbrtchange`: after a routing change, flushes the old route; a (hopefully) better one
/// is allocated the next time output is attempted.
pub fn in_pcbrtchange<G: ProcGlue>(inp: &Inpcb<G>, _errno: Option<Errno>) {
    drop(inp.route().take_rt());
}

/// `in_pcblookup_local_lock(.., IN_PCBLOCK_HOLD)`: the control block of the table bound to
/// local port `lport` and address `laddrp` with the fewest wildcards (`INPLOOKUP_WILDCARD`
/// allows some), counted; the table mutex held (`g`). `INPLOOKUP_IPV6` compares the IPv6
/// addresses.
pub fn in_pcblookup_local<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    laddrp: &Inpaddru,
    lport: u16,
    flags: i32,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let rdomain = rtable_l2(rtable);
    let laddr = laddrp.iau_addr();
    let laddr6 = laddrp.bytes6();
    let inet6 = flags & INPLOOKUP_IPV6 != 0;
    let lhash = in_pcblhash(table, rdomain, lport);
    let i = (lhash & g.inpt_lmask) as usize;
    with_chains(g, |c, g| {
        let mut matched: Option<&Inpcb<G>> = None;
        let mut matchwild = 3;
        for inp in c.lhash[i].iter(&*g) {
            if rtable_l2(inp.inp_rtableid.get()) != rdomain {
                continue;
            }
            if inp.inp_lport.get() != lport {
                continue;
            }
            let mut wildcard = 0;
            if inet6 {
                kassert!(inp.has_flags(INP_IPV6));

                if inp.inp_faddr6.bytes() != [0; 16] {
                    wildcard += 1;
                }

                let l6 = inp.inp_laddr6.bytes();
                if l6 != laddr6 {
                    if l6 == [0; 16] || laddr6 == [0; 16] {
                        wildcard += 1;
                    } else {
                        continue;
                    }
                }
            } else {
                kassert!(!inp.has_flags(INP_IPV6));

                if inp.inp_faddr.get().s_addr != INADDR_ANY {
                    wildcard += 1;
                }

                if inp.inp_laddr.get().s_addr != laddr.s_addr {
                    if inp.inp_laddr.get().s_addr == INADDR_ANY || laddr.s_addr == INADDR_ANY {
                        wildcard += 1;
                    } else {
                        continue;
                    }
                }
            }
            if (wildcard == 0 || flags & INPLOOKUP_WILDCARD != 0) && wildcard < matchwild {
                matched = Some(inp);
                matchwild = wildcard;
                if matchwild == 0 {
                    break;
                }
            }
        }
        matched.map(href)
    })
}

/// `in_pcblookup_local_lock(.., IN_PCBLOCK_GRAB)`: [`in_pcblookup_local`] taking `table`'s
/// mutex.
pub fn in_pcblookup_local_lock<G: ProcGlue>(
    table: &Inpcbtable<G>,
    laddrp: &Inpaddru,
    lport: u16,
    flags: i32,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let mut g = table.lock();
    in_pcblookup_local(&mut g, table, laddrp, lport, flags, rtable)
}

/// `in_pcbrtentry`: the route to the foreign address of `inp`, from its cache (counted: the
/// cache keeps its own); an IPv6 pcb's is `in6_pcbrtentry`'s.
pub fn in_pcbrtentry<G: ProcGlue>(inp: &Inpcb<G>, net: &NetHeld<'_>) -> Option<PoolArc<Rtentry>> {
    if inp.has_flags(INP_IPV6) {
        return in6_pcbrtentry(inp, net);
    }
    if inp.inp_faddr.get().s_addr == INADDR_ANY {
        return None;
    }
    let faddr = inp.inp_faddr.get();
    let laddr = inp.inp_laddr.get();
    let mut ro = inp.route();
    ro.mpath4(
        faddr.s_addr.to_ne_bytes(),
        Some(laddr.s_addr.to_ne_bytes()),
        inp.inp_rtableid.get(),
        net,
    )?;
    ro.ro_rt.clone()
}

/// `in_pcbselsrc`: the IPv4 address most appropriate as the source for destination
/// `dstsock`. If necessary, the routing table is looked up and the entry cached in `inp`.
pub fn in_pcbselsrc<G: ProcGlue>(
    dstsock: &SockaddrIn,
    inp: &Inpcb<G>,
    net: &NetHeld<'_>,
) -> Result<InAddr, Errno> {
    let dst = &dstsock.sin_addr;
    let laddr = inp.inp_laddr.get();
    let rtableid = inp.inp_rtableid.get();

    // If the socket(if any) is already bound, use that bound address unless it is
    // INADDR_ANY or INADDR_BROADCAST.
    if laddr.s_addr != INADDR_ANY && laddr.s_addr != INADDR_BROADCAST {
        return Ok(laddr);
    }
    let mopts = inp.moptions_out();

    // If the destination address is multicast or limited broadcast (255.255.255.255) and an
    // outgoing interface has been set as a multicast option, use the address of that
    // interface as our source address.
    if (in_multicast(dst.s_addr) || dst.s_addr == INADDR_BROADCAST)
        && let Some(mopts) = &mopts
        && let Some(ifp) = if_get(u32::from(mopts.imo_ifidx))
    {
        let r = net.reader();
        let ia = if ifp.if_rdomain.get() == rtable_l2(rtableid) {
            in_ifp2ia(&r, ifp)
        } else {
            None
        };
        return match ia {
            Some(ia) => Ok(ifatoia(ia).ia_addr.addr()),
            None => Err(Errno::EADDRNOTAVAIL),
        };
    }

    // If route is known or can be allocated now, our src addr is taken from the i/f, else
    // punt.
    let mut ro = inp.route();
    let rt = ro.mpath4(dst.s_addr.to_ne_bytes(), None, rtableid, net);

    // If we found a route, use the address corresponding to the outgoing interface (one
    // reader for the source selection: the route's address and the preferred one).
    let r = net.reader();
    let ia = rt.and_then(|rt| rt.ifa(&r)).map(ifatoia);

    // Use preferred source address if :
    // - destination is not onlink
    // - preferred source address is set
    // - output interface is UP
    if let Some(rt) = rt
        && rt.rt_flags.get() & RTF_GATEWAY != 0
        && let Some(ip4_source) = rtable_getsource(rtableid, AF_INET, net)
    {
        let ifa = ifa_ifwithaddr(net, &r, ip4_source.bytes(), rtableid);
        if let Some(ifa) = ifa
            && ifa
                .ifa_ifp
                .get()
                .is_some_and(|ifp| ifp.if_flags.get() & IFF_UP != 0)
            && let Some(sin) = hal::pod::at::<SockaddrIn>(ip4_source.bytes(), 0)
        {
            return Ok(sin.sin_addr);
        }
    }

    match ia {
        Some(ia) => Ok(ia.ia_addr.addr()),
        None => Err(Errno::EADDRNOTAVAIL),
    }
}

/// The bucket a pcb's hash slot word names.
#[inline]
fn slot_of(w: &AtomicU32) -> usize {
    w.load(Ordering::Relaxed) as usize
}

/// `in_pcbrehash`: moves `inp` to the hash buckets of its current addresses, the table mutex
/// held.
pub fn in_pcbrehash<G: ProcGlue>(g: &mut InpGuard<'_, G>, inp: &Inpcb<G>) {
    inp.check_guard(g);
    with_chains(g, |c, g| {
        c.lhash[slot_of(&inp.inp_lslot)].remove(g, inp);
        c.hash[slot_of(&inp.inp_hslot)].remove(g, inp);
    });
    in_pcbhash_insert(g, inp);
}

/// `in_pcbhash_insert`: links `inp` into the hash buckets of its addresses, the table mutex
/// held; the buckets are recorded in the pcb (the C's `LIST_REMOVE` needs no head, a pinned
/// list's removal names its list).
pub fn in_pcbhash_insert<G: ProcGlue>(g: &mut InpGuard<'_, G>, inp: &Inpcb<G>) {
    let table = inp.table();
    inp.check_guard(g);

    let lhash = in_pcblhash(table, inp.inp_rtableid.get(), inp.inp_lport.get());
    let rdomain = rtable_l2(inp.inp_rtableid.get());
    let hash = if inp.has_flags(INP_IPV6) {
        in6_pcbhash(
            table,
            rdomain,
            &inp.inp_faddr6.bytes(),
            inp.inp_fport.get(),
            &inp.inp_laddr6.bytes(),
            inp.inp_lport.get(),
        )
    } else {
        in_pcbhash(
            table,
            rdomain,
            &inp.inp_faddr.get(),
            inp.inp_fport.get(),
            &inp.inp_laddr.get(),
            inp.inp_lport.get(),
        )
    };
    let (mask, lmask) = (g.inpt_mask, g.inpt_lmask);
    let h = href(inp);
    let (hi, li) = ((hash & mask) as u32, (lhash & lmask) as u32);
    with_chains(g, |c, g| {
        c.lhash[li as usize].insert_head(g, h.as_pin());
        c.hash[hi as usize].insert_head(g, h.as_pin());
    });
    inp.inp_lslot.store(li, Ordering::Relaxed);
    inp.inp_hslot.store(hi, Ordering::Relaxed);
}

/// The control block of `hash`'s chain that `matches`, moved to the head of its chain so
/// that repeated accesses are quicker (analogous to the historic single-entry PCB cache),
/// counted; the table mutex held. The walk of `in_pcbhash_lookup` and `in6_pcbhash_lookup`,
/// `LIST_FOREACH` over the chain, O(k).
pub(crate) fn hash_chain_find<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    hash: u64,
    matches: impl Fn(&Inpcb<G>) -> bool,
) -> Option<InpcbRef<G>> {
    let i = (hash & g.inpt_mask) as usize;
    with_chains(g, |c, g| {
        let head = &c.hash[i];
        let inp = head.iter(&*g).find(|inp| matches(inp)).map(href)?;
        if !head
            .first(&*g)
            .is_some_and(|f| core::ptr::eq(f, &raw const *inp))
        {
            head.remove(g, &inp);
            head.insert_head(g, inp.as_pin());
        }
        Some(inp)
    })
}

/// `in_pcbhash_lookup` of IPv4: the control block with exactly these addresses, moved to the
/// head of its chain, counted; the table mutex held.
pub fn in_pcbhash_lookup<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    hash: u64,
    rdomain: u32,
    faddr: &InAddr,
    fport: u16,
    laddr: &InAddr,
    lport: u16,
) -> Option<InpcbRef<G>> {
    hash_chain_find(g, hash, |inp| {
        kassert!(!inp.has_flags(INP_IPV6));

        inp.inp_fport.get() == fport
            && inp.inp_lport.get() == lport
            && inp.inp_faddr.get().s_addr == faddr.s_addr
            && inp.inp_laddr.get().s_addr == laddr.s_addr
            && rtable_l2(inp.inp_rtableid.get()) == rdomain
    })
}

/// `in_pcbresize`: rehashes the table into tables of `hashsize` buckets, the table mutex
/// held (`g`). `ENOBUFS` when either table cannot be had without sleeping.
pub fn in_pcbresize<G: ProcGlue>(g: &mut InpGuard<'_, G>, hashsize: i32) -> Result<(), Errno> {
    let Some((nhash, mask)) = hashinit::<InpHash<G>>(hashsize, M_PCB, M_NOWAIT) else {
        return Err(Errno::ENOBUFS);
    };
    let Some((nlhash, lmask)) = hashinit::<InpLhash<G>>(hashsize, M_PCB, M_NOWAIT) else {
        return Err(Errno::ENOBUFS);
    };
    let Some(old) = g.chains.replace(Chains {
        hash: nhash,
        lhash: nlhash,
    }) else {
        kpanic(format_args!("in_pcbresize: no hash tables"));
    };
    g.inpt_mask = mask;
    g.inpt_lmask = lmask;
    g.inpt_size = hashsize;

    // Every pcb leaves the old chains for the new: the old buckets emptied one by one (the
    // C walks the queue; a marker is on no chain), O(n) plus the old size, as the C's walk
    // and `hashfree`.
    for (i, chain) in old.hash.iter().enumerate() {
        while let Some(inp) = chain.first(&*g).map(href) {
            kassert!(slot_of(&inp.inp_hslot) == i);
            chain.remove(g, &inp);
            let lslot = slot_of(&inp.inp_lslot);
            old.lhash[lslot].remove(g, &inp);
            in_pcbhash_insert(g, &inp);
        }
    }
    drop(old);

    Ok(())
}

/// `in_pcblookup_lock`: the connected IPv4 control block for `faddr.fport <-> laddr.lport`,
/// counted; no wildcard matching is done, so listening sockets are not found
/// ([`in_pcblookup_listen`] finds those). The table mutex held (`IN_PCBLOCK_HOLD`).
pub fn in_pcblookup_held<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    faddr: InAddr,
    fport: u16,
    laddr: InAddr,
    lport: u16,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let rdomain = rtable_l2(rtable);
    let hash = in_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    in_pcbhash_lookup(g, hash, rdomain, &faddr, fport, &laddr, lport)
}

/// `in_pcblookup`: [`in_pcblookup_held`] taking the table mutex (`IN_PCBLOCK_GRAB`).
pub fn in_pcblookup<G: ProcGlue>(
    table: &Inpcbtable<G>,
    faddr: InAddr,
    fport: u16,
    laddr: InAddr,
    lport: u16,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let rdomain = rtable_l2(rtable);
    let hash = in_pcbhash(table, rdomain, &faddr, fport, &laddr, lport);
    let mut g = table.lock();
    in_pcbhash_lookup(&mut g, hash, rdomain, &faddr, fport, &laddr, lport)
}

/// `in_pcblookup_listen`: the listening IPv4 control block for `laddr.lport`: unspecified
/// foreign address and port, bound to `laddr` or to the wildcard address, counted.
pub fn in_pcblookup_listen<G: ProcGlue>(
    table: &Inpcbtable<G>,
    laddr: InAddr,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<InpcbRef<G>> {
    let l = in_pcblookup_listen_keys(table, laddr, lport_arg, m, rtable)?;
    let mut g = table.lock();
    in_pcblookup_listen_held(&mut g, table, &l)
}

/// What `in_pcblookup_listen` computes before it takes the table mutex (in_pcb.c:1292-1294):
/// the first local address (`laddr`, or pf's divert-to address), the second (the wildcard,
/// or `laddr` for a connection redirected to localhost), the local port, the routing domain
/// and the first address's hash.
pub struct ListenLookup {
    key1: InAddr,
    key2: InAddr,
    lport: u16,
    rdomain: u32,
    hash1: u64,
}

/// [`ListenLookup`] of `laddr.lport_arg` in `table` (`m`'s pf tags choose the keys); `None`
/// for a packet pf diverts as a reply (no listener answers it).
pub fn in_pcblookup_listen_keys<G: ProcGlue>(
    table: &Inpcbtable<G>,
    laddr: InAddr,
    lport_arg: u16,
    m: Option<&Mbuf>,
    rtable: u32,
) -> Option<ListenLookup> {
    let mut key1 = laddr;
    let mut key2 = ZEROIN_ADDR;
    let mut lport = lport_arg;

    if let Some(m) = m
        && m.m_pkthdr().pf.flags.get() & PF_TAG_DIVERTED != 0
    {
        let divert = pf().and_then(|pf| pf.find_divert(m));
        kassert!(divert.is_some());
        let divert = divert?;
        match divert.type_ {
            PF_DIVERT_TO => {
                let a = InAddr {
                    s_addr: u32::from_ne_bytes([
                        divert.addr[0],
                        divert.addr[1],
                        divert.addr[2],
                        divert.addr[3],
                    ]),
                };
                key1 = a;
                key2 = a;
                lport = divert.port;
            }
            PF_DIVERT_REPLY => return None,
            t => kpanic(format_args!(
                "in_pcblookup_listen: unknown divert type {t}, mbuf {m:p}"
            )),
        }
    } else if let Some(m) = m
        && m.m_pkthdr().pf.flags.get() & PF_TAG_TRANSLATE_LOCALHOST != 0
    {
        // Redirected connections should not be treated the same as connections directed to
        // 127.0.0.0/8 since localhost can only be accessed from the host itself. For example
        // portmap(8) grants more permissions for connections to the socket bound to
        // 127.0.0.1 than to the * socket.
        key1 = ZEROIN_ADDR;
        key2 = laddr;
    }
    let rdomain = rtable_l2(rtable);
    let hash1 = in_pcbhash(table, rdomain, &ZEROIN_ADDR, 0, &key1, lport);
    Some(ListenLookup {
        key1,
        key2,
        lport,
        rdomain,
        hash1,
    })
}

/// `in_pcblookup_listen`'s lookups of `l` ([`in_pcblookup_listen_keys`]), the table mutex
/// held: the pcb bound to the first key (its hash computed before the mutex), else to the
/// second (hashed here, as the C), counted.
pub fn in_pcblookup_listen_held<G: ProcGlue>(
    g: &mut InpGuard<'_, G>,
    table: &Inpcbtable<G>,
    l: &ListenLookup,
) -> Option<InpcbRef<G>> {
    let &ListenLookup {
        key1,
        key2,
        lport,
        rdomain,
        hash1,
    } = l;
    let mut inp = in_pcbhash_lookup(g, hash1, rdomain, &ZEROIN_ADDR, 0, &key1, lport);
    if inp.is_none() && key1.s_addr != key2.s_addr {
        let hash = in_pcbhash(table, rdomain, &ZEROIN_ADDR, 0, &key2, lport);
        inp = in_pcbhash_lookup(g, hash, rdomain, &ZEROIN_ADDR, 0, &key2, lport);
    }
    inp
}

/// `in_pcbset_rtableid`: moves an unbound `inp` to routing table `rtableid`.
pub fn in_pcbset_rtableid<G: ProcGlue>(inp: &Inpcb<G>, rtableid: u32) -> Result<(), Errno> {
    // table must exist
    if !rtable_exists(rtableid) {
        return Err(Errno::EINVAL);
    }

    let mut g = inp.table().lock();
    if inp.inp_lport.get() != 0 {
        return Err(Errno::EBUSY);
    }
    inp.set_rtableid(&g, rtableid);
    in_pcbrehash(&mut g, inp);

    Ok(())
}

/// `in_pcbset_addr` of an IPv4 control block: gives `inp` both addresses at once (a
/// connection accepted from a listener), unless another control block has them.
pub fn in_pcbset_addr<G: ProcGlue>(
    inp: &Inpcb<G>,
    fsin: &SockaddrIn,
    lsin: &SockaddrIn,
    rtableid: u32,
) -> Result<(), Errno> {
    let table = inp.table();
    kassert!(!inp.has_flags(INP_IPV6));

    let mut g = table.lock();
    if in_pcblookup_held(
        &mut g,
        table,
        fsin.sin_addr,
        fsin.sin_port,
        lsin.sin_addr,
        lsin.sin_port,
        rtableid,
    )
    .is_some()
    {
        return Err(Errno::EADDRINUSE);
    }

    inp.set_rtableid(&g, rtableid);
    inp.set_laddr(&g, lsin.sin_addr);
    inp.set_lport(&g, lsin.sin_port);
    inp.set_faddr(&g, fsin.sin_addr);
    inp.set_fport(&g, fsin.sin_port);
    in_pcbrehash(&mut g, inp);
    drop(g);

    in_pcb_newflow(inp);
    Ok(())
}

/// `in_pcbunset_faddr`: forgets the foreign address.
pub fn in_pcbunset_faddr<G: ProcGlue>(inp: &Inpcb<G>) {
    let mut g = inp.table().lock();
    if inp.has_flags(INP_IPV6) {
        inp.set_faddr6(&g, [0u8; 16]);
    } else {
        inp.set_faddr(&g, InAddr { s_addr: INADDR_ANY });
    }
    inp.set_fport(&g, 0);
    in_pcbrehash(&mut g, inp);
}

/// `in_pcbunset_laddr`: forgets both addresses (the local port stays).
pub fn in_pcbunset_laddr<G: ProcGlue>(inp: &Inpcb<G>) {
    let mut g = inp.table().lock();
    if inp.has_flags(INP_IPV6) {
        inp.set_faddr6(&g, [0u8; 16]);
        inp.set_laddr6(&g, [0u8; 16]);
    } else {
        inp.set_faddr(&g, InAddr { s_addr: INADDR_ANY });
        inp.set_laddr(&g, InAddr { s_addr: INADDR_ANY });
    }
    inp.set_fport(&g, 0);
    in_pcbrehash(&mut g, inp);
}

/// The host tests' Internet sockets (`ip_output`'s socket options, this module's pcb tests):
/// a datagram protocol of an `AF_INET` domain over the UNIX domain tests' glue and root
/// thread, its sockets made by `soalloc` alone (the domain is not configured), the interface
/// layer and the routing tables of table 0 fresh, the "inpcb" pool made.
#[cfg(test)]
pub(crate) mod testinet {
    use std::boxed::Box;
    use std::sync::{MutexGuard, Once};

    use hal::netlock::NETLOCK;
    use hal::pod;
    use hal::sys::net_if::{IFF_BROADCAST, IFF_MULTICAST};
    use hal::sys::netinet_in_var::InAliasreq;
    use hal::sys::socket::{PF_INET, SOCK_DGRAM};
    use hal::sys::sockio::{SIOCADDMULTI, SIOCAIFADDR, SIOCDELMULTI, SIOCSIFADDR};

    use super::*;
    use crate::fs::testfs::TestGlue;
    use crate::net::domain::Domain;
    use crate::net::if_::IFNETLIST;
    use crate::net::if_var::{Ifnet, NetDriver};
    use crate::net::mbuf::MbufBox;
    use crate::net::proto::SockaddrView;
    use crate::net::proto::{PR_ADDR, PR_ATOMIC, Protocol, Usrreqs};
    use crate::net::socket::{SoInit, soalloc};

    /// The tests' glue: the UNIX domain tests' (a root thread is `curproc`).
    pub(crate) type G = TestGlue;

    /// The test domain: `AF_INET`, whose sockets the net lock serialises.
    pub(crate) struct InetDom;
    /// Its datagram protocol (`IPPROTO_UDP`), without user requests: the tests call the pcb
    /// functions themselves.
    struct UdpProto;

    static PROTOS: [&dyn Protocol<G>; 1] = [&UdpProto];

    impl Domain<G> for InetDom {
        fn family(&self) -> i32 {
            i32::from(PF_INET)
        }
        fn name(&self) -> &'static str {
            "inet"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
            &PROTOS
        }
    }

    /// The IPv6 test domain (`AF_INET6`), for the sockets of the IPv6 tests' protocols
    /// (the TCP tests' listeners): its family is what `dom_family` reads.
    pub(crate) struct Inet6Dom;

    impl Domain<G> for Inet6Dom {
        fn family(&self) -> i32 {
            i32::from(hal::sys::socket::PF_INET6)
        }
        fn name(&self) -> &'static str {
            "inet6"
        }
        fn protocols(&self) -> &'static [&'static dyn Protocol<G>] {
            &[]
        }
    }

    impl Protocol<G> for UdpProto {
        fn domain(&self) -> &'static dyn Domain<G> {
            &InetDom
        }
        fn ty(&self) -> i16 {
            SOCK_DGRAM as i16
        }
        fn protocol(&self) -> i16 {
            IPPROTO_UDP as i16
        }
        fn flags(&self) -> i16 {
            PR_ATOMIC | PR_ADDR
        }
        fn usrreqs(&self) -> Option<&'static dyn Usrreqs<G>> {
            None
        }
    }

    /// The fixture; the guard serialises the tests that hold kernel memory.
    pub(crate) fn setup() -> (MutexGuard<'static, ()>, &'static Thread<G>) {
        static IDS: Once = Once::new();
        let (g, p) = crate::net::unix::tests::setup();
        crate::net::if_::softnet_test_install();
        crate::net::if_::if_idxmap_test_reset();
        crate::net::if_::ifnet_test_reset();
        crate::netinet::igmp::igmp_test_reset();
        crate::netinet::if_ether::arp_test_reset();
        crate::netinet6::nd6::nd6_test_reset();
        crate::netinet6::nd6_nbr::dad_test_reset();
        crate::netinet6::frag6::frag6_test_reset();
        crate::netinet6::icmp6::icmp6_test_reset();
        crate::netinet6::ip6_input::ip6_test_reset();
        crate::net::rtable::rtable_test_reset();
        crate::net::route::rtlabel_test_reset();
        crate::net::route::route_init();
        crate::net::rtable::rtable_init_doms(&crate::net::route::tests::ROUTE_DOMS);
        in_init::<G>();
        IDS.call_once(crate::netinet::ip_id::ip_randomid_init);
        (g, p)
    }

    /// A table of `hashsize` buckets (`in_pcbinit`), for the run.
    pub(crate) fn table(hashsize: i32) -> &'static Inpcbtable<G> {
        let t: &'static Inpcbtable<G> = Box::leak(Box::new(Inpcbtable::new()));
        in_pcbinit(t, hashsize);
        t
    }

    /// A socket of the test protocol, with a control block in `table`: the socket's handle
    /// and the control block's (the caller's, beside the socket's `inp_refcnt`). Opened by
    /// root through a set-user-id program (euid 0, ruid [`TEST_RUID`]), so a reader of the
    /// two ids that swaps them (`tcp_ident`) is caught.
    pub(crate) fn pcb(table: &'static Inpcbtable<G>) -> (SocketRef<G>, InpcbRef<G>) {
        let so = soalloc(
            &UdpProto,
            M_WAIT,
            SoInit {
                so_type: SOCK_DGRAM,
                state: 0,
                options: 0,
                linger: 0,
                timeo: 0,
                euid: 0,
                ruid: TEST_RUID,
                egid: 0,
                rgid: 0,
                cpid: 1,
            },
        )
        .expect("a socket");
        let inp = in_pcballoc(&so, table, M_WAIT).expect("in_pcballoc");
        (so, inp)
    }

    /// The real user id of [`pcb`]'s sockets.
    pub(crate) const TEST_RUID: u32 = 1000;

    /// `udp_detach` and `soclose`: the control block detached under the net lock, the socket
    /// let go (no file holds it).
    pub(crate) fn release(so: SocketRef<G>, inp: InpcbRef<G>) {
        so.st_set(SS_NOFDREF);
        let w = NETLOCK.write();
        let net = NetHeld::of(&w).expect("the net lock");
        in_pcbdetach(inp, &net);
        drop(w);
        drop(so);
    }

    /// A driver that counts the packets it is given (and drops them) and the multicast
    /// filter changes.
    #[derive(Default)]
    pub(crate) struct InetDriver {
        /// Packets output.
        pub(crate) sent: AtomicU32,
        /// `SIOCADDMULTI` requests.
        pub(crate) addmulti: AtomicU32,
        /// `SIOCDELMULTI` requests.
        pub(crate) delmulti: AtomicU32,
        /// The bytes of the last packet output (TCP's reply tests).
        pub(crate) last: std::sync::Mutex<std::vec::Vec<u8>>,
    }

    impl NetDriver for InetDriver {
        fn ioctl(&self, ifp: &'static Ifnet, cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            match cmd {
                SIOCSIFADDR => ifp.if_flags.set(ifp.if_flags.get() | IFF_UP),
                SIOCADDMULTI => {
                    self.addmulti.fetch_add(1, Ordering::Relaxed);
                }
                SIOCDELMULTI => {
                    self.delmulti.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            Ok(())
        }
        fn output(
            &self,
            _ifp: &'static Ifnet,
            m: MbufBox,
            _dst: SockaddrView<'_>,
            _rt: Option<&Rtentry>,
        ) -> Result<(), Errno> {
            let mut bytes = std::vec![0u8; m.m_pkthdr().len.get() as usize];
            crate::net::mbuf::m_copydata(&m, 0, &mut bytes);
            drop(m);
            if let Ok(mut l) = self.last.lock() {
                *l = bytes;
            }
            self.sent.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    /// An attached, listed, running broadcast and multicast interface `name` (an
    /// [`InetDriver`]) with the IPv4 address `addr`/24, its local and connected routes made.
    pub(crate) fn iface(name: &[u8], addr: [u8; 4]) -> (&'static Ifnet, &'static InetDriver) {
        let d: &'static InetDriver = Box::leak(Box::default());
        let ifp = crate::net::if_::test_ifnet_with(name, d);
        ifp.if_flags.set(IFF_BROADCAST | IFF_MULTICAST);
        ifp.if_mtu.set(1500);
        crate::net::if_::if_alloc_sadl(ifp);
        {
            let _w = NETLOCK.write();
            IFNETLIST.insert_tail(ifp);
        }
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
            &sin(addr),
        );
        let _ = pod::put(
            &mut r,
            core::mem::offset_of!(InAliasreq, ifra_mask),
            &sin([255, 255, 255, 0]),
        );
        crate::netinet::in_::in_ioctl(SIOCAIFADDR, &mut r, Some(ifp), true).expect("an address");
        (ifp, d)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// The "inpcb" item's size, field by field (D-3: `vmstat -m` shows it under the C's pool
    /// name; the hand-off and the commit record the three columns).
    #[test]
    fn the_inpcb_item_size_field_by_field() {
        use crate::fs::testfs::TestGlue as G;
        use core::mem::size_of;
        let rows = [
            ("inp_home", size_of::<PoolHome<Inpcb<G>>>()),
            ("inp_table", size_of::<Option<&'static Inpcbtable<G>>>()),
            ("3 x PinEntry", 3 * size_of::<PinEntry<Inpcb<G>>>()),
            ("2 x slot", 8),
            ("2 x InAddrCell", 8),
            ("2 x In6AddrCell", 32),
            ("2 x port", 4),
            ("inp_socket", size_of::<Option<SocketRef<G>>>()),
            ("inp_icmp6filt", size_of::<PtrWord<c_void>>()),
            (
                "inp_tcpcb",
                size_of::<Guarded<Option<PoolBox<crate::netinet::tcp_var::Tcpcb<G>>>>>(),
            ),
            ("inp_options", size_of::<LoanSlot<Option<MbufBox>>>()),
            ("inp_route", size_of::<ReadMostly<Route>>()),
            ("inp_mou", size_of::<ReadMostly<Mou>>()),
            (
                "inp_outputopts6",
                size_of::<ReadMostly<Option<MBox<Ip6Pktopts>>>>(),
            ),
            ("inp_refcnt", 4),
            ("inp_anchor", size_of::<ReadMostly<Option<InpcbRef<G>>>>()),
            ("inp_ip", size_of::<IpProto>()),
            ("inp_ipv6", size_of::<Ip6Proto>()),
            ("inp_upcall", size_of::<OnceStatic<InpUpcall>>()),
            ("pf_sk", size_of::<PfSlot>()),
            ("scalars", 4 * 6 + 1 + 2),
        ];
        let mut sum = 0;
        for (n, z) in rows {
            std::println!("{n}: {z}");
            sum += z;
        }
        std::println!(
            "fields {sum}, Inpcb<G> {}, Route {}",
            size_of::<Inpcb<G>>(),
            size_of::<Route>()
        );
        // 544 bytes, 560 with the counted item's header (the count and the pool), against the
        // C's struct inpcb of 328 (LP64) and LZ's 352 (the module's Redesign): the three
        // pinned entries (96: a lock identity each besides the two links), the route cache's
        // cell (72), the multicast options' one cell (48: the C's union `inp_mou`, the box
        // carrying its malloc type and allocator and the family's tag; N6.6a2's diet, two cells
        // of 40 before), the IPv6 packet options' cell (40), the IPv4 options' slot (24: the
        // claim word and the chain's box, N6.6a0's D-15, a word of bsd's before), the anchor
        // (16) and the kernel's tcpcb slot (32: the cell's 16-byte key and the 16-byte box,
        // N6.6a2's D-12; bsd's 8-byte word beside it went at N6.6a5: 552 and 568 before) are
        // the growth; the item stays within the brief's bound of 584 (S12).
        assert_eq!(sum, 543);
        assert_eq!(size_of::<Inpcb<G>>(), 544);
        assert!(size_of::<Inpcb<G>>() + 16 <= 584, "S12: the \"inpcb\" item");

        // The C's struct inpcb on LP64, as repr(C) lays out in_pcb.h's members in order (the
        // pointers as words, the unions as their largest member's words).
        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CRoute {
            ro_rt: usize,
            ro_generation: u64,
            ro_tableid: u64,
            ro_dst: [u32; 7],
            ro_src: [u32; 4],
        }
        #[allow(dead_code, reason = "a layout replica: only its size is read")]
        #[repr(C)]
        struct CInpcb {
            inp_table: usize,
            inp_queue: [usize; 2],
            inp_hash: [usize; 2],
            inp_lhash: [usize; 2],
            inp_faddru: [u32; 4],
            inp_laddru: [u32; 4],
            inp_fport: u16,
            inp_lport: u16,
            inp_socket: usize,
            inp_ppcb: usize,
            inp_route: CRoute,
            inp_refcnt: [u32; 2],
            inp_flags: i32,
            inp_hu: [u32; 10],
            inp_options: usize,
            inp_hops: i32,
            inp_mou: usize,
            inp_seclevel: [u8; 4],
            inp_ip_minttl: u8,
            inp_cksum6: i32,
            inp_icmp6filt: usize,
            inp_pf_sk: usize,
            inp_upcall: usize,
            inp_upcall_arg: usize,
            inp_rtableid: u32,
            inp_pipex: i32,
            inp_flowid: u16,
        }
        assert_eq!(size_of::<CRoute>(), 72);
        assert_eq!(size_of::<CInpcb>(), 328);
    }

    #[test]
    fn the_port_bitmaps_and_their_defaults() {
        let m = Baddynamicports::new();
        dp_set(&m.tcp, 6000);
        assert!(dp_isset(&m.tcp, 6000));
        assert!(!dp_isset(&m.tcp, 6001));
        dp_clr(&m.tcp, 6000);
        assert!(!dp_isset(&m.tcp, 6000));
    }

    // The control blocks over the kernel's Internet socket fixture ([`testinet`]): the
    // iterator across a detach between two steps, the hash tables grown from 16 to 1,024
    // buckets, pf's state key word, the socket lock the route cache and the multicast options
    // need, and the counts a pcb shows.

    use core::pin::pin;
    use std::sync::OnceLock;
    use std::vec::Vec;

    use hal::netlock::NETLOCK;
    use hal::sys::endian::htons;

    use super::testinet::{self, G as Tg};
    use crate::net::pf_hooks::testpf;
    use crate::net::socket::solock_shared;

    /// The items the glue's "inpcb" pool lends now: `vmstat -m`'s in use.
    fn nout() -> u64 {
        crate::net::domain::net::<Tg>().inpcb.pool.nout()
    }

    fn a(b: [u8; 4]) -> InAddr {
        InAddr {
            s_addr: u32::from_ne_bytes(b),
        }
    }

    /// `inp` bound and connected as `in_pcbconnect` leaves it (the four members set and the
    /// pcb rehashed under the table mutex).
    fn place(inp: &Inpcb<Tg>, laddr: InAddr, lport: u16, faddr: InAddr, fport: u16) {
        let mut g = inp.table().lock();
        inp.set_laddr(&g, laddr);
        inp.set_lport(&g, lport);
        inp.set_faddr(&g, faddr);
        inp.set_fport(&g, fport);
        in_pcbrehash(&mut g, inp);
    }

    /// The pcb's address, to compare answers.
    fn at(inp: &Inpcb<Tg>) -> *const Inpcb<Tg> {
        core::ptr::from_ref(inp)
    }

    #[test]
    fn the_iterator_survives_a_detach_between_two_steps() {
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let before = nout();
        let mut pcbs: Vec<Option<(SocketRef<Tg>, InpcbRef<Tg>)>> =
            (0..5).map(|_| Some(testinet::pcb(t))).collect();
        // The queue's order: the last allocated first (`TAILQ_INSERT_HEAD`).
        pcbs.reverse();
        let addr = |p: &Option<(SocketRef<Tg>, InpcbRef<Tg>)>| {
            p.as_ref().map(|(_, i)| at(i)).expect("a pcb")
        };
        let order: Vec<*const Inpcb<Tg>> = pcbs.iter().map(addr).collect();
        assert_eq!(nout(), before + 5);

        let m = pin!(Inpcb::<Tg>::marker());
        let other = pin!(Inpcb::<Tg>::marker());

        // The first step.
        let mut g = t.lock();
        let cur = in_pcb_iterator(t, &mut g, None, m.as_ref()).expect("the first pcb");
        assert_eq!(at(&cur), order[0]);
        // A second walk starts and overtakes the first: each steps over the other's marker.
        let o1 = in_pcb_iterator(t, &mut g, None, other.as_ref()).expect("its first pcb");
        let o2 = in_pcb_iterator(t, &mut g, Some(o1), other.as_ref()).expect("its second");
        assert_eq!(at(&o2), order[1]);
        drop(g);

        // The next pcb is detached while the walker holds no lock (the C's callers drop the
        // table mutex for the socket lock): the walk goes on past it, and it is freed at
        // once (nothing holds it).
        let (so1, inp1) = pcbs[1].take().expect("pcb 1");
        drop(o2);
        testinet::release(so1, inp1);
        assert_eq!(nout(), before + 4);
        let mut g = t.lock();
        let cur = in_pcb_iterator(t, &mut g, Some(cur), m.as_ref()).expect("pcb 2");
        assert_eq!(
            at(&cur),
            order[2],
            "pcb 1 is gone, the marker stood after pcb 0"
        );
        drop(g);

        // The pcb the walker holds is detached: it leaves the queue but stays an item until
        // the walk's next step drops it, under the table mutex (the C's `in_pcbunref` in
        // `in_pcb_iterator`).
        let (so2, inp2) = pcbs[2].take().expect("pcb 2");
        testinet::release(so2, inp2);
        assert_eq!(nout(), before + 4, "the walk's handle keeps the item");
        let mut g = t.lock();
        let cur = in_pcb_iterator(t, &mut g, Some(cur), m.as_ref()).expect("pcb 3");
        assert_eq!(at(&cur), order[3]);
        assert_eq!(nout(), before + 3, "pcb 2 freed by the step");
        let cur = in_pcb_iterator(t, &mut g, Some(cur), m.as_ref()).expect("pcb 4");
        assert_eq!(at(&cur), order[4]);
        assert!(in_pcb_iterator(t, &mut g, Some(cur), m.as_ref()).is_none());
        assert!(
            !m.inp_queue.is_linked(),
            "the walk's end takes its marker out"
        );
        in_pcb_iterator_abort(t, &mut g, None, other.as_ref());
        assert!(!other.inp_queue.is_linked());
        // Only pcbs on the queue now.
        let left: Vec<*const Inpcb<Tg>> = t.queue().iter(&g).map(at).collect();
        assert_eq!(left, [order[0], order[3], order[4]]);
        drop(g);

        for (so, inp) in pcbs.into_iter().flatten() {
            testinet::release(so, inp);
        }
        assert_eq!(nout(), before);
        assert_eq!(t.count(), 0);
    }

    #[test]
    fn the_table_grows_from_16_to_1024_buckets_and_finds_every_pcb() {
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let n = 700u16;
        let key = |i: u16| {
            (
                a([10, 0, (i >> 8) as u8, i as u8]),
                htons(1024 + i),
                a([10, 1, 0, 1]),
                htons(53),
            )
        };
        let mut pcbs = Vec::new();
        for i in 0..n {
            let (so, inp) = testinet::pcb(t);
            // Hashed now, so every later resize has to move it.
            let (l, lp, f, fp) = key(i);
            place(&inp, l, lp, f, fp);
            pcbs.push((so, inp));
        }
        // in_pcballoc resized at the load factor: 16, 32, ..., 1024 (past 768 it would be 2048).
        assert_eq!(t.size(), 1024);
        assert_eq!(t.count(), i32::from(n));

        let found = |pcbs: &Vec<(SocketRef<Tg>, InpcbRef<Tg>)>| {
            for (i, (_, inp)) in (0..n).zip(pcbs) {
                let (l, lp, f, fp) = key(i);
                let hit = in_pcblookup(t, f, fp, l, lp, 0).expect("in_pcblookup");
                assert_eq!(at(&hit), at(inp), "pcb {i}");
                // A connected pcb is a wildcard match of its local port (its foreign address
                // is set): bind(2)'s lookup with INPLOOKUP_WILDCARD.
                let local =
                    in_pcblookup_local_lock(t, &Inpaddru::from_addr(l), lp, INPLOOKUP_WILDCARD, 0)
                        .expect("in_pcblookup_local");
                assert_eq!(at(&local), at(inp), "pcb {i}, local port");
            }
        };
        found(&pcbs);

        // A resize by hand: every pcb on the new chains, none left on the old.
        {
            let mut g = t.lock();
            in_pcbresize(&mut g, 2048).expect("in_pcbresize");
        }
        assert_eq!(t.size(), 2048);
        found(&pcbs);

        for (so, inp) in pcbs {
            testinet::release(so, inp);
        }
        assert_eq!(t.count(), 0);
    }

    #[test]
    fn pf_unlinks_its_state_key_at_disconnect_and_detach() {
        let (_g, _p) = testinet::setup();
        let pf = testpf::register();
        let t = testinet::table(16);
        let (so, inp) = testinet::pcb(t);
        let unlinks = || pf.inp_unlinks.load(Ordering::Relaxed);
        let removals = || pf.divert_removals.load(Ordering::Relaxed);
        let (u0, r0) = (unlinks(), removals());

        // pf_inp_link (pf's, under pf_inp_mtx): the state key's word in the pcb.
        inp.inp_pf_sk.set(testpf::key(), 0x2000);
        assert!(!in_pcbdisconnect(&inp), "a file still holds the socket");
        assert_eq!(inp.inp_pf_sk.get(), 0, "pf_inp_unlink emptied it");
        assert_eq!(pf.last_unlinked.load(Ordering::Relaxed), 0x2000);
        assert_eq!((unlinks(), removals()), (u0 + 1, r0 + 1));

        // in_pcbdetach: the divert state removed, the key unlinked, before the pcb goes.
        inp.inp_pf_sk.set(testpf::key(), 0x3000);
        testinet::release(so, inp);
        assert_eq!(pf.last_unlinked.load(Ordering::Relaxed), 0x3000);
        assert_eq!((unlinks(), removals()), (u0 + 2, r0 + 2));
    }

    #[test]
    fn in_pcbset_addr_sets_the_flow_id() {
        // NSTOEPLITZ: the flow id of the accepted connection, the system hash of its four
        // members (the test starts from another value, so a missing store cannot pass).
        let (_g, _p) = testinet::setup();
        crate::net::toeplitz::stoeplitz_init();
        let t = testinet::table(16);
        let (so, inp) = testinet::pcb(t);
        let (l, f) = (a([10, 0, 0, 1]), a([10, 0, 0, 2]));
        let (lport, fport) = (htons(1000), htons(2000));
        let want = crate::net::toeplitz::stoeplitz_ip4port(f.s_addr, l.s_addr, fport, lport);
        inp.inp_flowid.set(!want);
        let sin = |addr: InAddr, port: u16| SockaddrIn {
            sin_len: size_of::<SockaddrIn>() as u8,
            sin_family: hal::sys::socket::AF_INET,
            sin_port: port,
            sin_addr: addr,
            sin_zero: [0; 8],
        };
        in_pcbset_addr(&inp, &sin(f, fport), &sin(l, lport), 0).expect("a free quadruple");
        assert_eq!(inp.inp_flowid.get(), want);
        // The same flow seen from the peer's side hashes the same.
        assert_eq!(
            crate::net::toeplitz::stoeplitz_ip4port(l.s_addr, f.s_addr, lport, fport),
            want
        );
        testinet::release(so, inp);
    }

    #[test]
    fn the_route_cache_and_the_multicast_options_need_the_socket_lock() {
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let (so, inp) = testinet::pcb(t);
        let (so2, inp2) = testinet::pcb(t);

        assert!(!inp.route_locked(), "no lock at all");
        {
            let _r = NETLOCK.read();
            assert!(
                !inp.route_locked(),
                "the net lock shared alone: another socket's holder may be in"
            );
        }
        {
            let _w = NETLOCK.write();
            assert!(inp.route_locked(), "the net lock exclusively");
            drop(inp.route());
            drop(inp.moptions());
        }
        {
            // solock_shared: the net lock shared, then this socket's lock written.
            let w = solock_shared(&so);
            assert!(inp.route_locked());
            assert!(!inp2.route_locked(), "another socket's lock does not do");
            drop(inp.route());
            drop(w);
        }
        {
            let w = solock_shared(&so2);
            assert!(inp2.route_locked() && !inp.route_locked());
            drop(w);
        }
        testinet::release(so, inp);
        testinet::release(so2, inp2);
    }

    /// The pcb key `bsd`'s boot takes (no boot here: the first test that asks).
    fn key() -> &'static LegacyKey {
        static KEY: OnceLock<&'static LegacyKey> = OnceLock::new();
        KEY.get_or_init(|| LegacyKey::take().expect("the key, untaken in this binary"))
    }

    #[test]
    fn the_counts_a_pcb_shows_are_the_cs() {
        // The one count userland sees is `vmstat -m`'s in use of "inpcb": one item per pcb,
        // whatever handles exist (the anchor that keeps it while `inp_refcnt` is above zero,
        // a lookup's answer, a walk's). `inp_refcnt` (the C's refcount, which no userland
        // interface reports) starts at the C's 1, the socket's, and counts `bsd`'s holders
        // only: neither the anchor nor the kernel's handles are in it. kern.file reports the
        // pcb's address (`so_pcb`), not a count.
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let before = nout();
        let (so, inp) = testinet::pcb(t);
        assert_eq!(nout(), before + 1);
        assert_eq!(
            crate::net::domain::net::<Tg>().inpcb.pool.item_size(),
            Some(560),
            "vmstat -m's size of an \"inpcb\" item: the value and the counted item's header"
        );
        assert_eq!(inp.legacy_count(), 1, "refcnt_init: the socket's");
        assert_eq!(so.legacy_pcb(), at(&inp).addr(), "so_pcb names the pcb");

        let (l, lp, f, fp) = (a([10, 0, 0, 1]), htons(7), a([10, 0, 0, 2]), htons(9));
        place(&inp, l, lp, f, fp);
        let hit = in_pcblookup(t, f, fp, l, lp, 0).expect("found");
        assert_eq!(
            nout(),
            before + 1,
            "a lookup's answer is a handle, not an item"
        );
        assert_eq!(inp.legacy_count(), 1, "nor a count of inp_refcnt");

        // bsd's in_pcbref and in_pcbunref.
        inp.legacy_ref();
        assert_eq!(inp.legacy_count(), 2);
        assert!(inp.legacy_rele(key()).is_none(), "not the last");
        assert_eq!(inp.legacy_count(), 1);
        assert_eq!(nout(), before + 1);

        // The detach drops the socket's count; the lookup's handle keeps the item until it
        // goes (the C's in_pcbunref after in_pcbdetach, its pool_put at the last).
        testinet::release(so, inp);
        assert_eq!(hit.legacy_count(), 0);
        assert_eq!(nout(), before + 1);
        drop(hit);
        assert_eq!(nout(), before);
    }

    /// The quadruple of the race tests' pcb.
    fn race_quad() -> (InAddr, u16, InAddr, u16) {
        (a([10, 0, 0, 1]), htons(80), a([10, 0, 0, 2]), htons(4242))
    }

    #[test]
    fn a_lookup_counted_under_the_mutex_survives_a_detach_and_the_last_other_unref() {
        // tcp_input's lookup of a TIME_WAIT pcb against tcp_timer_2msl's close of it on
        // another CPU (review 1's B1): the answer's count is taken before the table mutex is
        // let go (bsd's adapters, in_pcb.c:1220-1221), so the detach and the timer's
        // in_pcbunref leave it the last count, and in_pcbsolock finds the socket gone (the C
        // drops the segment).
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let before = nout();
        let (so, inp) = testinet::pcb(t);
        let (l, lp, f, fp) = race_quad();
        place(&inp, l, lp, f, fp);
        let keep = inp.clone();
        // The timer's count (tcp_timer's in_pcbref).
        inp.legacy_ref();

        // The lookup, its count under the guard.
        {
            let mut g = t.lock();
            let h = in_pcblookup_held(&mut g, t, f, fp, l, lp, 0).expect("found");
            h.legacy_ref_locked(&g);
            drop(h);
            drop(g);
        }
        assert_eq!(keep.legacy_count(), 3);

        // The other CPU: the close's detach, then the timer's in_pcbunref.
        testinet::release(so, inp);
        assert!(
            keep.legacy_rele(key()).is_none(),
            "the lookup's count is left"
        );
        assert_eq!(keep.legacy_count(), 1);
        {
            let _r = NETLOCK.read();
            assert!(
                in_pcbsolock_detached(&keep).is_none(),
                "the socket is gone: drop the segment"
            );
        }
        // The lookup's caller lets go: the last count, the item freed with the last handle.
        drop(keep.legacy_rele(key()).expect("the anchor"));
        assert_eq!(nout(), before + 1, "the test's own handle");
        drop(keep);
        assert_eq!(nout(), before);
    }

    #[test]
    #[should_panic(expected = "in_pcbref: inpcb")]
    fn a_count_taken_after_the_mutex_can_find_the_pcb_released() {
        // The order bsd's adapters had before review 1's B1: the lookup lets the mutex go, the
        // close and the timer's unref run, then the count is taken from zero: panic(9).
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let (so, inp) = testinet::pcb(t);
        let (l, lp, f, fp) = race_quad();
        place(&inp, l, lp, f, fp);
        inp.legacy_ref();
        let h = in_pcblookup(t, f, fp, l, lp, 0).expect("found");
        testinet::release(so, inp);
        let _ = h.legacy_rele(key());
        h.legacy_ref();
    }

    #[test]
    #[should_panic(expected = "another table's mutex")]
    fn a_lookup_count_needs_its_own_table_s_guard() {
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let other = testinet::table(16);
        let (_so, inp) = testinet::pcb(t);
        let g = other.lock();
        inp.legacy_ref_locked(&g);
    }

    #[test]
    #[should_panic(expected = "inp_route: the socket lock of inpcb")]
    fn the_route_cache_is_refused_without_the_socket_lock() {
        let (_g, _p) = testinet::setup();
        let (_so, inp) = testinet::pcb(testinet::table(16));
        drop(inp.route());
    }

    #[test]
    #[should_panic(expected = "inp_options: the socket lock of inpcb")]
    fn the_ip_options_are_refused_without_the_socket_lock() {
        let (_g, _p) = testinet::setup();
        let (_so, inp) = testinet::pcb(testinet::table(16));
        let _r = NETLOCK.read();
        drop(inp.options());
    }

    #[test]
    fn the_ip_options_are_lent_under_the_socket_lock_and_freed_by_the_detach() {
        // N6.6a0 (D-15): inp_options is the kernel's cell, a LoanSlot of the chain: lent
        // under the socket lock (here the exclusive net lock, soassertlocked's first case),
        // put back by the loan's drop, and freed by in_pcbdetach (the C's m_freem there).
        use crate::net::mbuf::{M_WAIT, MT_SOOPTS, m_get, mbstat_read};

        let (_g, _p) = testinet::setup();
        let (so, inp) = testinet::pcb(testinet::table(16));
        let before = mbstat_read().m_mtypes[MT_SOOPTS as usize];
        let (lent, kept) = {
            let _w = NETLOCK.write();
            *inp.options() = m_get(M_WAIT, MT_SOOPTS);
            let lent = {
                let o = inp.options();
                let during = inp.inp_options.update(|v| v.is_none());
                o.is_some() && during
            };
            (lent, inp.options().is_some())
        };
        let held = mbstat_read().m_mtypes[MT_SOOPTS as usize];
        // A handle kept past the detach (a lookup's, a walk's): the options go at the
        // detach, not with the item.
        let keep = inp.clone();
        testinet::release(so, inp);
        let after = mbstat_read().m_mtypes[MT_SOOPTS as usize];
        drop(keep);
        assert!(lent, "lent out: the slot empty meanwhile");
        assert!(kept, "the loan put the chain back");
        assert_eq!(held, before + 1);
        assert_eq!(after, before, "in_pcbdetach freed the options");
    }

    #[test]
    fn the_multicast_options_share_one_cell_by_family() {
        // N6.6a2's diet: inp_moptions and inp_moptions6 are the C's union, one cell tagged by
        // family. An IPv4 pcb's options are its moptions, invisible to moptions6_out; the
        // loan gives the cell back with the member it holds.
        let (_g, _p) = testinet::setup();
        let (so, inp) = testinet::pcb(testinet::table(16));
        let w = NETLOCK.write();
        let imo =
            MBox::new(M_IPMOPTS_TEST, M_WAITOK, IpMoptions::output(3, 7, 1)).expect("the options");
        let before = inp.moptions().is_none();
        *inp.moptions() = Some(imo);
        let out = inp.moptions_out();
        let out6 = inp.moptions6_out();
        let kept = inp.moptions().as_deref().map(|m| m.imo_ttl);
        let freed = inp.moptions().take().is_some();
        let after = inp.moptions_out().is_none();
        drop(w);
        assert!(before);
        assert_eq!(
            out.map(|o| (o.imo_ifidx, o.imo_ttl, o.imo_loop)),
            Some((3, 7, 1))
        );
        assert!(out6.is_none(), "the other family's member");
        assert_eq!(kept, Some(7), "the loan put the member back");
        assert!(freed && after);
        testinet::release(so, inp);
    }

    /// `M_IPMOPTS` (`malloc.h`), the multicast options' type.
    const M_IPMOPTS_TEST: i32 = 53;

    #[test]
    #[should_panic(expected = "inp_moptions6: the multicast options of a pcb of the other family")]
    fn an_ipv4_pcbs_options_are_not_lent_as_ipv6_ones() {
        let (_g, _p) = testinet::setup();
        let (_so, inp) = testinet::pcb(testinet::table(16));
        let w = NETLOCK.write();
        *inp.moptions() = MBox::new(M_IPMOPTS_TEST, M_WAITOK, IpMoptions::output(1, 1, 0));
        // The C's union read as the other member: refused (the net lock is a RAII witness,
        // let go by the unwind).
        let _ = inp.moptions6();
        drop(w);
    }

    #[test]
    #[should_panic(expected = "inp_moptions: the socket lock of inpcb")]
    fn the_multicast_options_are_refused_without_the_socket_lock() {
        let (_g, _p) = testinet::setup();
        let (_so, inp) = testinet::pcb(testinet::table(16));
        let _r = NETLOCK.read();
        drop(inp.moptions());
    }

    #[test]
    fn in_pcbsolock_holds_a_kernel_socket_by_a_witness_until_its_pcb_goes() {
        // The test's protocol is a kernel one: its socket is locked by a witness of so_lock
        // under the caller's net lock, the parts open; once detached, the socket is not
        // locked (the C's so_pcb == NULL), and nothing is left held.
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let (so, inp) = testinet::pcb(t);
        let keep = inp.clone();
        let sock = keep.socket_opt().map(|s| core::ptr::from_ref(s).addr());
        let (held, parts) = {
            let r = NETLOCK.read();
            let net = NetHeld::of(&r).expect("the net lock's witness");
            let mut w = in_pcbsolock(&keep, &net).expect("attached");
            let held = keep.socket().so_lock_legacy().lock().status() == hal::rwlock::RW_WRITE;
            let parts = w.inet().is_some();
            drop(w);
            (held, parts)
        };
        assert!(held && parts, "a witness of so_lock, opened in parts");
        assert_eq!(keep.socket().so_lock_legacy().lock().status(), 0);
        testinet::release(so, inp);
        let gone = {
            let r = NETLOCK.read();
            let net = NetHeld::of(&r).expect("the net lock's witness");
            in_pcbsolock(&keep, &net).is_none()
        };
        assert!(gone, "the socket is detached: nothing to lock");
        assert_eq!(keep.socket().so_lock_legacy().lock().status(), 0);
        assert!(sock.is_some());
    }

    #[test]
    #[should_panic(expected = "in_pcbsolock: inpcb")]
    fn in_pcbsolock_detached_needs_the_net_lock() {
        let (_g, _p) = testinet::setup();
        let (_so, inp) = testinet::pcb(testinet::table(16));
        let _ = in_pcbsolock_detached(&inp);
    }

    #[test]
    fn the_hash_computed_before_the_mutex_finds_the_same_pcb() {
        // bsd's adapters hash before they take the table mutex (in_pcb.c:1208-1212,
        // :1292-1294), then walk the bucket under the guard: the same answer as the lookups
        // that hash under it.
        let (_g, _p) = testinet::setup();
        let t = testinet::table(16);
        let (so, inp) = testinet::pcb(t);
        let (l, lp, f, fp) = race_quad();
        place(&inp, l, lp, f, fp);
        let (so2, lis) = testinet::pcb(t);
        place(&lis, l, htons(81), ZEROIN_ADDR, 0);
        let (so3, any) = testinet::pcb(t);
        place(&any, ZEROIN_ADDR, htons(82), ZEROIN_ADDR, 0);

        // Connected.
        let rdomain = rtable_l2(0);
        let hash = in_pcbhash(t, rdomain, &f, fp, &l, lp);
        let found = {
            let mut g = t.lock();
            in_pcbhash_lookup(&mut g, hash, rdomain, &f, fp, &l, lp)
        };
        assert_eq!(found.as_deref().map(at), Some(at(&inp)));
        let held = {
            let mut g = t.lock();
            in_pcblookup_held(&mut g, t, f, fp, l, lp, 0)
        };
        assert_eq!(held.as_deref().map(at), Some(at(&inp)));

        // Listening on the address: the first key's hash, computed before the mutex.
        let keys = in_pcblookup_listen_keys(t, l, htons(81), None, 0).expect("keys");
        let found = {
            let mut g = t.lock();
            in_pcblookup_listen_held(&mut g, t, &keys)
        };
        assert_eq!(found.as_deref().map(at), Some(at(&lis)));
        // Listening on the wildcard: the second key, hashed under the mutex as the C.
        let keys = in_pcblookup_listen_keys(t, l, htons(82), None, 0).expect("keys");
        let found2 = {
            let mut g = t.lock();
            in_pcblookup_listen_held(&mut g, t, &keys)
        };
        assert_eq!(found2.as_deref().map(at), Some(at(&any)));
        assert_eq!(
            in_pcblookup_listen(t, l, htons(82), None, 0)
                .as_deref()
                .map(at),
            Some(at(&any))
        );

        drop((found, found2, held));
        testinet::release(so, inp);
        testinet::release(so2, lis);
        testinet::release(so3, any);
    }
}
/* </TESTS> */
