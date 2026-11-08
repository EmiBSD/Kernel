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
 * Copyright (c) 1982, 1986, 1988, 1991, 1993
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
 *	@(#)uipc_mbuf.c	8.2 (Berkeley) 1/4/94
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

/*
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)mbuf.h	8.5 (Berkeley) 2/19/95
 */
/* </LICENSES> */

/* <CODE> */
//! The mbuf: the network stack's packets, their pools, chains and lists (`kern/uipc_mbuf.c`
//! and the kernel half of `<sys/mbuf.h>`).
//!
//! Upstream: sys/kern/uipc_mbuf.c @ 3ce1f3f79392
//! Upstream: sys/sys/mbuf.h @ 3ce1f3f79392
//! LZ: sys/kern/uipc_mbuf.rs@3c62ede68802
//! LZ: sys/sys/mbuf.rs@3c62ede68802
//!
//! An mbuf is `MSIZE` (256) bytes from `mbufpl`: the links of its chain (`m_next`) and of its
//! packet list (`m_nextpkt`), where its data starts and how long it is, and a data area. A
//! plain mbuf keeps up to [`MLEN`] bytes of data there; the first mbuf of a packet (`M_PKTHDR`)
//! keeps a packet header ([`Pkthdr`]) at the start of the area and [`MHLEN`] bytes after it;
//! an mbuf with a cluster (`M_EXT`, one of the eight `mcl*` pools) keeps the cluster's
//! description ([`Ext`]) after the header's place and its data in the cluster. A cluster
//! shared by several mbufs (`m_copym`, `m_split`) is counted ([`ExtRefs`], from `mextrefs`).
//! The chain operations (copy, adjust, pull up, split, ...), the packet header helpers, the
//! ddb printers and the `mbuf_list`/`mbuf_queue` functions are here; `m_pulldown` and the
//! packet tags are [`super::mbuf2`]'s.
//!
//! ## Deviations
//! - A `PR_WAITOK` allocation that finds no memory fails instead of sleeping (`pool_get`'s
//!   deviation in `bsd`'s `kern/subr_pool.rs`, inherited).
//! - `m_clget` panics on a request larger than the largest cluster with or without
//!   `DIAGNOSTIC`; without it the C dereferences the NULL pool (inherited).
//! - `IFQ_DEFPRIO` (`<net/if.h>`) and `CACHELINESIZE` (`<sys/percpu.h>`) are private
//!   constants here until those headers move (N6.3a; inherited).
//! - `struct mbuf_queue`, the `mq_*` functions and `sysctl_mq` are not here yet: their only
//!   callers are `bsd`'s, which keeps LZ's shape for them (`sys/sys/mbuf.rs`'s `MbufQueue`,
//!   a softc's zero-allocated queue whose level `mq_init` sets at run time); N6.3a writes
//!   the kernel's queue with the interface queues, its first user.
//! - Where the C overruns an mbuf, the kernel stops or splits instead: an offset or a length
//!   that leaves the data storage is `panic(9)` ([`Mbuf::data`]'s bounds); `m_copym` of a
//!   packet whose first mbuf is empty into a header mbuf (the C writes up to `MLEN` bytes into
//!   `MHLEN`) copies what fits and continues in the next mbuf of the copy.
//! - `m_copyback` into a shared cluster (`M_READONLY`), which the C writes for every mbuf
//!   sharing it, gives the mbuf a cluster of its own first (the kernel cannot lend a shared
//!   cluster's bytes mutably); `ENOBUFS` when none can be had, as for its other allocations.
//!   No caller of the tree writes a shared cluster this way: the C's writers check
//!   `M_READONLY` first (`m_pullup`, `m_cat`, `m_makespace`, `m_pulldown`, ipsec's output).
//! - External storage of a driver's own (`MEXTADD` with a buffer and free function of its
//!   own, `mextfree_register`): not implemented, no user in the tree (only clusters are
//!   external storage); N7 adds it with its first driver (D-N6.1-2).
//! - `m_freem` of a chain and `m_purge` free mbuf by mbuf as the C does; `m_free` on a chain's
//!   head frees what its `m_nextpkt` link owns as well (the C forgets it): every packet has
//!   one owner here, and the owner of a list unlinks a packet before freeing it.
//!
//! ## Redesign
//! - LZ's `&'static Mbuf` from `pool_get` to `pool_put` (every field a `Cell`, the data area
//!   an `UnsafeCell` read through raw pointers, 35 `unsafe` here and 8 in the header) becomes
//!   an owned item: [`MbufBox`], a `PoolBox<Mbuf>` of `mbufpl`, whose drop is `m_free`'s body.
//!   The chain is the item's own: `m_next` and `m_nextpkt` are `hal::owned::SlistLink`s (8
//!   bytes, the C's pointer), each owning the next mbuf, walked by `get`/`get_mut` (O(1) a
//!   step) and changed by `set`/`take`; a dropped chain is freed one mbuf at a time. Every
//!   function that may replace a chain's head takes and returns the box (`m_pullup`,
//!   `m_prepend`, `m_free`, `m_freem`); the in-place ones take `&mut Mbuf` (`m_cat`,
//!   `m_copyback`, `m_defrag`, `m_split`, `m_copym`, which turns the source's clusters shared)
//!   or `&Mbuf` where they change only lengths and offsets (`m_adj`, `m_align`,
//!   `m_calchdrlen`); the readers `&Mbuf`. Out parameters are return values (LZ's).
//! - `struct mbuf`'s two untagged unions are `hal::overlay::Body`: the packet header and the
//!   cluster description are typed values lent while their bits (`M_PKTHDR`, `M_EXT`) are
//!   set, which only the typed transitions write ([`m_inithdr`], [`m_removehdr`],
//!   [`m_move_hdr`], [`m_dup_pkthdr`], [`m_clget`], the drop); `MLEN` stays 224 and `MHLEN`
//!   144. `m_flags` is read and set through [`MFlags`], which keeps the two tag bits.
//! - `m_data` is an offset (`m_off`) into the data storage ([`Mbuf::storage`]: the cluster,
//!   or the area after the header, or the whole area), so the data is a bounds-checked slice
//!   ([`Mbuf::data`], [`Mbuf::data_mut`]; `None` is the C's `M_READONLY`); the address
//!   `bsd`'s `mtod` needs is [`Mbuf::data_raw`], never dereferenced here. Cost against
//!   `mtod`: the two tag tests and one bounds check per access (the `mbuf.mtod` bench row;
//!   N12's).
//! - Cost against LZ (decision 43: `sys/kernel/benches/net.rs`'s mbuf rows against LZ's
//!   functions in a scratch test, ns an op, the median of rounds 2-4, on AC). Five runs, each
//!   number from one of them: (a) all rows after review 1's fixes, 2026-11-08 00:14; (b)
//!   `copym.share`, `copym.share.deep` and the controls `get.free`, `gethdr.cluster.free`
//!   beside the builder's fix, 07:14; (c) `mtod`, `header.view`, `header.copy`, `tag.find` and
//!   the control `get.free` beside review C1's atomic flags word, 07:42; (d) `mtod`,
//!   `header.view`, `header.copy`, `slist.push_tail`, `slist.concat` and the control `get.free`
//!   beside the list's tail filled through a shared borrow by a compare-exchange, 08:03; (e)
//!   `slist.push_tail`, `slist.concat` and the control `get.free` with that fill an atomic load
//!   and store, 08:10. Release. Faster: `get.free` 29.8 -> 23.1 (a; 29.8 -> 23.1 in b, 30.1 ->
//!   23.4 in c, 30.2 -> 23.7 in d) and `gethdr.cluster.free` 53.3 -> 39.0 (a; 53.9 -> 39.0 in
//!   b): one typed item, the header built in place. Within 10 %: `copydata.1500` 17.0 -> 18.4
//!   (a), `pullup.40` 8.5 -> 9.3 (a), `copym.share` 34.8 -> 35.7 and `copym.share.deep` 32.4 ->
//!   32.5 (b, with `MChainBuilder::into_head` leaving the builder's `Drop`, an unfinished
//!   chain's, unrun on a finished copy; 40.4 and 36.5 without it, same run). Slower, each a
//!   fixed cost per call, never a walk the C does not make: `mtod` 0.3 -> 0.5 (c, d;
//!   `data_raw`: the overlay's `M_EXT` tag test, then the cached cluster address or the
//!   storage's start, plus the offset, about ten instructions where LZ loads one field);
//!   `header.view` 0.5 -> 1.1 (c; 1.0 in d; the kernel row views 20 bytes through `data()`'s
//!   tag test and bounds check and `hal::pod::view`'s length and alignment checks; LZ's reads
//!   one byte through a raw pointer, unchecked); `header.copy` 4.4 -> 6.6 (c; 6.2 before C1 in
//!   the same run, 6.3 in d: `m_copydata` of 20 bytes at offset 14 on both sides, the kernel's
//!   a call that builds the `data()` slice, its tag tests and bounds check, and copies with a
//!   checked `copy_from_slice`, where LZ's inlines into a `memcpy`; since C1 the flags word is
//!   an atomic, whose `Relaxed` loads the compiler does not merge, so `Mbuf::data` reads it
//!   three times where it read it once, 44 -> 52 instructions in the release bench,
//!   `storage_len` twice where once); `adj` 1.8 -> 3.9 (a; LZ's row is bimodal, 2.9 in every
//!   round of the first A/B and 1.7-1.8 in (a), the kernel's 3.8-3.9 at every commit of the
//!   review; the row includes the bench's restore, `adj.restore` 0.3 -> 0.5 (a):
//!   `set_data_off`'s bounds check, the overlay's tag tests for the storage size and the
//!   header, through cells; the rest, the C's loop over the same cells, is not attributed to
//!   one instruction; the user accepted this partial reason on 2026-11-08, and the rest is an
//!   N12 row); `tag.find` 0.8 -> 1.0 (c; 0.9 before C1 in the same run, with `m_tag_find`'s
//!   code unchanged: 0.1-0.2 ns, the measure's resolution). Kernel only: `park.unpark` 2.9 (a;
//!   `bsd`'s `unpark` and `park` of one mbuf: a compare-and-swap of the park mark each way,
//!   paid at every crossing of the `bsd` boundary), `data` 0.9 (a; the bounds-checked slice),
//!   and the packet list's `slist.push_tail` 1.6 -> 1.6 and `slist.concat` 1.6 -> 1.6 (e, the
//!   kernel at 0ba8cb25 and with the tail's link filled through a shared borrow, an atomic
//!   load, a null test and an atomic store, `SlistLink::fill`, so that no `&mut` names a tail
//!   `bsd` may hold by address on another CPU, wg's serial queue; no LZ row: `ml_enqueue`'s and
//!   `ml_enlist`'s tail insert; the compare-exchange form of (d) cost 1.9 and 2.4).
//!   Dev (`opt-level = 1`, no LTO): `mtod` 0.4 -> 1.4, `header.view` 0.5 -> 2.4 (1.6 before C1:
//!   the dev row inlined `Mbuf::data` before C1 and calls it since, an out-of-line body of 183
//!   instructions with nine loads of the flags word, whose atomic loads `opt-level = 1` does
//!   not merge and which no longer fits its inlining budget), `header.copy` 5.2 -> 7.5,
//!   `tag.find` 0.8 -> 2.2, `get.free` 39.1 -> 32.0 (c; d: `mtod` 1.3, `header.view` 2.4,
//!   `header.copy` 7.4, `get.free` 31.7, and in e `slist.push_tail` 2.0 -> 1.9, `slist.concat`
//!   1.9 -> 2.3, `get.free` 40.0 -> 32.1); `adj` 2.9 -> 6.1 (`adj.restore` 0.3 -> 3.7),
//!   `pullup.40` 12.2 -> 16.1, `copydata.1500` 18.1 -> 18.6, `park.unpark` 6.3 (a);
//!   `copym.share` 45.7 -> 53.3, `copym.share.deep` 44.1 -> 45.1, `gethdr.cluster.free` 66.9 ->
//!   55.9 (b): at `opt-level = 1` `hal`'s `PoolBox::deref` (no `#[inline]`) and `Ext::bytes`
//!   (`ClusterBox`'s eight-way match) stay calls inside the rows' loops (the dev bench's
//!   disassembly: `bl PoolBox<Mbuf>::deref` and `bl Ext::bytes` per iteration; the release loop
//!   has none), where LZ's `Cell` getters on a `&'static` inline. N12 takes these: `#[inline]`
//!   on `PoolBox`'s and `PoolArc`'s `Deref`/`DerefMut` and on the cluster byte accessors (or
//!   `Ext` caching the cluster's length beside its address), the flags word read once per
//!   access (one load answering which values are live, which also keeps `Mbuf::data` inlined in
//!   dev), one typed view per operation instead of a tag test per access, the bounds checks
//!   hoisted out of the walks.
//! - A cluster is a `PoolBox<Bytes<N>>` of its pool ([`ClusterBox`], one variant per size),
//!   handed out unwritten as the C's (`BoxPool::get_bytes`); a shared cluster is a
//!   `PoolArc<ExtRefs>` of `mextrefs`, made on the first share as the C makes its
//!   `struct m_ext_refs` (64 bytes, the C's item), whose count is the C's `refcnt` and whose
//!   last drop zeroes the bytes when `m_zero` asked (`zero`) and gives the cluster back. One
//!   atomic per share and per free of a shared cluster, none for an unshared one, as the C.
//!   `mextfree_fns[]`, `m_extfree_pool`, `m_extfree_refs` and `m_extref`'s index juggling are
//!   those types' drops.
//! - `mbstat` is a `hal::percpu::Counters` from boot (the C's boot counters and
//!   `counters_alloc_ncpus` in one); `kern.mbstat` reads [`mbstat_read`]. The pools' pages
//!   are [`MbufPages`], the `PageSource` of `m_pool_allocator` (the memory limit, the DMA
//!   constraint `m_pool_noconstraints` lifts for every pool at once, as the C does pool by
//!   pool).
//! - The pf hooks of `m_free`, `m_clearhdr` and `m_dup_pkthdr` are the `Pf` bridge
//!   ([`super::pf_hooks`], bridges.toml), no-ops before `bsd`'s pf registers.
//! - `struct mbuf_list` is `OwnedSlist<MPkt>` ([`MbufList`]: head, tail, length);
//!   `ml_dechain` hands the list itself over.

use core::cell::Cell;
use core::cmp::min;
use core::fmt;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU64, Ordering};

use hal::intr::{IPL_NET, IPL_VM, splraise};
use hal::malloc::{M_NOWAIT, M_WAITOK};
use hal::once::OnceStatic;
use hal::overlay::{Body, Bytes};
use hal::owned::{OwnedSlist, SlistLink};
use hal::percpu::Counters;
use hal::pool::{
    BoxPool, PR_NOWAIT, PR_WAITOK, PageSource, ParkWord, Parkable, PoolArc, PoolBox, Pooled,
    TypedPool,
};
use hal::sys::errno::Errno;
pub use hal::sys::mbuf::{
    MT_CONTROL, MT_DATA, MT_FREE, MT_FTABLE, MT_HEADER, MT_NTYPES, MT_OOBDATA, MT_SONAME,
    MT_SOOPTS, Mbstat,
};
use hal::sys::param::PAGE_SIZE;
use hal::sys::types::Vaddr;
use libkern::explicit_bzero;

use crate::assert::{kassert, kdassert, kpanic};
use crate::clock::Timeval;
use crate::net::mbuf2::m_tag_copy_chain;
use crate::net::pf_hooks;
use crate::vm::km::{
    KMEM_DYN_INITIALIZER, KP_MBUF_CONTIG, KV_INTRSAFE, KmemDynMode, KmemPaMode, KmemVaMode,
    km_alloc, km_free, pa_mode,
};

// Constants related to network buffer management. MCLBYTES must be no larger than PAGE_SIZE
// (the software page size) and, on machines that exchange pages of input or output buffers
// with mbuf clusters (MAPPED_MBUFS), MCLBYTES must also be an integral multiple of the hardware
// page size.

/// `MSIZE`: size of an mbuf.
pub const MSIZE: usize = 256;
/// The mbuf's header before its data area (`struct m_hdr`): the two links, the offset and the
/// length, the flags, the type and the parked mark.
const MHDR_SIZE: usize = 32;
/// `MLEN`: normal data len.
pub const MLEN: usize = MSIZE - MHDR_SIZE;
/// `MHLEN`: data len w/pkthdr.
pub const MHLEN: usize = MLEN - size_of::<Pkthdr>();
/// `MAXMCLBYTES`: largest cluster from the stack.
pub const MAXMCLBYTES: usize = 64 * 1024;
/// `MINCLSIZE`: smallest amount to put in cluster.
pub const MINCLSIZE: usize = MHLEN + MLEN + 1;
/// `MCLSHIFT`: convert bytes to m_buf clusters; a 2K cluster can hold an Ethernet frame.
pub const MCLSHIFT: usize = 11;
/// `MCLBYTES`: size of a m_buf cluster.
pub const MCLBYTES: usize = 1 << MCLSHIFT;

// pkthdr_pf.flags

/// `PF_TAG_GENERATED`.
pub const PF_TAG_GENERATED: u8 = 0x01;
/// `PF_TAG_SYNCOOKIE_RECREATED`.
pub const PF_TAG_SYNCOOKIE_RECREATED: u8 = 0x02;
/// `PF_TAG_TRANSLATE_LOCALHOST`.
pub const PF_TAG_TRANSLATE_LOCALHOST: u8 = 0x04;
/// `PF_TAG_DIVERTED`.
pub const PF_TAG_DIVERTED: u8 = 0x08;
/// `PF_TAG_DIVERTED_PACKET`.
pub const PF_TAG_DIVERTED_PACKET: u8 = 0x10;
/// `PF_TAG_REROUTE`.
pub const PF_TAG_REROUTE: u8 = 0x20;
/// `PF_TAG_REFRAGMENTED`: refragmented ipv6 packet.
pub const PF_TAG_REFRAGMENTED: u8 = 0x40;
/// `PF_TAG_PROCESSED`: packet was checked by pf.
pub const PF_TAG_PROCESSED: u8 = 0x80;

/// `MPF_BITS`: the `%b` description of `pkthdr_pf.flags`.
pub const MPF_BITS: &[u8] = b"\x10\x01GENERATED\x02SYNCOOKIE_RECREATED\x03TRANSLATE_LOCALHOST\
\x04DIVERTED\x05DIVERTED_PACKET\x06REROUTE\x07REFRAGMENTED\x08PROCESSED";

// mbuf flags

/// `M_EXT`: has associated external storage.
pub const M_EXT: u16 = 0x0001;
/// `M_PKTHDR`: start of record.
pub const M_PKTHDR: u16 = 0x0002;
/// `M_EOR`: end of record.
pub const M_EOR: u16 = 0x0004;
/// `M_EXTWR`: external storage is writable.
pub const M_EXTWR: u16 = 0x0008;
/// `M_PROTO1`: protocol-specific.
pub const M_PROTO1: u16 = 0x0010;

// mbuf pkthdr flags, also in m_flags

/// `M_VLANTAG`: `ether_vtag` is valid.
pub const M_VLANTAG: u16 = 0x0020;
/// `M_LOOP`: packet has been sent from local machine.
pub const M_LOOP: u16 = 0x0040;
/// `M_BCAST`: sent/received as link-level broadcast.
pub const M_BCAST: u16 = 0x0100;
/// `M_MCAST`: sent/received as link-level multicast.
pub const M_MCAST: u16 = 0x0200;
/// `M_CONF`: payload was encrypted (ESP-transport).
pub const M_CONF: u16 = 0x0400;
/// `M_AUTH`: payload was authenticated (AH or ESP auth).
pub const M_AUTH: u16 = 0x0800;
/// `M_TUNNEL`: IP-in-IP added by tunnel mode IPsec.
pub const M_TUNNEL: u16 = 0x1000;
/// `M_ZEROIZE`: zeroize data part on free.
pub const M_ZEROIZE: u16 = 0x2000;
/// `M_COMP`: header was decompressed.
pub const M_COMP: u16 = 0x4000;
/// `M_LINK0`: link layer specific flag.
pub const M_LINK0: u16 = 0x8000;

/// `M_BITS`: the `%b` description of `m_flags`.
pub const M_BITS: &[u8] = b"\x10\x01M_EXT\x02M_PKTHDR\x03M_EOR\x04M_EXTWR\x05M_PROTO1\
\x06M_VLANTAG\x07M_LOOP\x09M_BCAST\x0aM_MCAST\x0bM_CONF\x0cM_AUTH\x0dM_TUNNEL\x0eM_ZEROIZE\
\x0fM_COMP\x10M_LINK0";

/// `M_COPYFLAGS`: flags copied when copying `m_pkthdr`.
pub const M_COPYFLAGS: u16 = M_PKTHDR
    | M_EOR
    | M_PROTO1
    | M_BCAST
    | M_MCAST
    | M_CONF
    | M_COMP
    | M_AUTH
    | M_LOOP
    | M_TUNNEL
    | M_LINK0
    | M_VLANTAG
    | M_ZEROIZE;

// Checksumming flags

/// `M_IPV4_CSUM_OUT`: IPv4 checksum needed.
pub const M_IPV4_CSUM_OUT: u16 = 0x0001;
/// `M_TCP_CSUM_OUT`: TCP checksum needed.
pub const M_TCP_CSUM_OUT: u16 = 0x0002;
/// `M_UDP_CSUM_OUT`: UDP checksum needed.
pub const M_UDP_CSUM_OUT: u16 = 0x0004;
/// `M_IPV4_CSUM_IN_OK`: IPv4 checksum verified.
pub const M_IPV4_CSUM_IN_OK: u16 = 0x0008;
/// `M_IPV4_CSUM_IN_BAD`: IPv4 checksum bad.
pub const M_IPV4_CSUM_IN_BAD: u16 = 0x0010;
/// `M_TCP_CSUM_IN_OK`: TCP checksum verified.
pub const M_TCP_CSUM_IN_OK: u16 = 0x0020;
/// `M_TCP_CSUM_IN_BAD`: TCP checksum bad.
pub const M_TCP_CSUM_IN_BAD: u16 = 0x0040;
/// `M_UDP_CSUM_IN_OK`: UDP checksum verified.
pub const M_UDP_CSUM_IN_OK: u16 = 0x0080;
/// `M_UDP_CSUM_IN_BAD`: UDP checksum bad.
pub const M_UDP_CSUM_IN_BAD: u16 = 0x0100;
/// `M_ICMP_CSUM_OUT`: ICMP/ICMPv6 checksum needed.
pub const M_ICMP_CSUM_OUT: u16 = 0x0200;
/// `M_ICMP_CSUM_IN_OK`: ICMP/ICMPv6 checksum verified.
pub const M_ICMP_CSUM_IN_OK: u16 = 0x0400;
/// `M_ICMP_CSUM_IN_BAD`: ICMP/ICMPv6 checksum bad.
pub const M_ICMP_CSUM_IN_BAD: u16 = 0x0800;
/// `M_IPV6_DF_OUT`: don't fragment outgoing IPv6.
pub const M_IPV6_DF_OUT: u16 = 0x1000;
/// `M_TIMESTAMP`: `ph_timestamp` is set.
pub const M_TIMESTAMP: u16 = 0x2000;
/// `M_FLOWID`: `ph_flowid` is set.
pub const M_FLOWID: u16 = 0x4000;
/// `M_TCP_TSO`: TCP Segmentation Offload needed.
pub const M_TCP_TSO: u16 = 0x8000;

/// `MCS_BITS`: the `%b` description of `csum_flags`.
pub const MCS_BITS: &[u8] = b"\x10\x01IPV4_CSUM_OUT\x02TCP_CSUM_OUT\x03UDP_CSUM_OUT\
\x04IPV4_CSUM_IN_OK\x05IPV4_CSUM_IN_BAD\x06TCP_CSUM_IN_OK\x07TCP_CSUM_IN_BAD\x08UDP_CSUM_IN_OK\
\x09UDP_CSUM_IN_BAD\x0aICMP_CSUM_OUT\x0bICMP_CSUM_IN_OK\x0cICMP_CSUM_IN_BAD\x0dIPV6_NODF_OUT\
\x0eTIMESTAMP\x0fFLOWID\x10TCP_TSO";

// flags to m_get/MGET

/// `M_DONTWAIT`: `M_NOWAIT`.
pub const M_DONTWAIT: i32 = M_NOWAIT;
/// `M_WAIT`: `M_WAITOK`.
pub const M_WAIT: i32 = M_WAITOK;

/// `M_COPYALL`: length to `m_copym` to copy all.
pub const M_COPYALL: i32 = 1_000_000_000;

/// `M_MAXLOOP`: detect mbufs looping in the kernel when spliced too often.
pub const M_MAXLOOP: u8 = 128;

// Packet tag types

/// `PACKET_TAG_IPSEC_IN_DONE`: IPsec applied, in.
pub const PACKET_TAG_IPSEC_IN_DONE: u16 = 0x0001;
/// `PACKET_TAG_IPSEC_OUT_DONE`: IPsec applied, out.
pub const PACKET_TAG_IPSEC_OUT_DONE: u16 = 0x0002;
/// `PACKET_TAG_IPSEC_FLOWINFO`: IPsec flowinfo.
pub const PACKET_TAG_IPSEC_FLOWINFO: u16 = 0x0004;
/// `PACKET_TAG_IP_OFFNXT`: IPv4 offset and next proto.
pub const PACKET_TAG_IP_OFFNXT: u16 = 0x0010;
/// `PACKET_TAG_IP6_OFFNXT`: IPv6 offset and next proto.
pub const PACKET_TAG_IP6_OFFNXT: u16 = 0x0020;
/// `PACKET_TAG_WIREGUARD`: WireGuard data.
pub const PACKET_TAG_WIREGUARD: u16 = 0x0040;
/// `PACKET_TAG_GRE`: GRE processing done.
pub const PACKET_TAG_GRE: u16 = 0x0080;
/// `PACKET_TAG_DLT`: data link layer type.
pub const PACKET_TAG_DLT: u16 = 0x0100;
/// `PACKET_TAG_PF_DIVERT`: pf(4) diverted packet.
pub const PACKET_TAG_PF_DIVERT: u16 = 0x0200;
/// `PACKET_TAG_PF_REASSEMBLED`: pf reassembled ipv6 packet.
pub const PACKET_TAG_PF_REASSEMBLED: u16 = 0x0800;
/// `PACKET_TAG_SRCROUTE`: IPv4 source routing options.
pub const PACKET_TAG_SRCROUTE: u16 = 0x1000;
/// `PACKET_TAG_TUNNEL`: tunnel endpoint address.
pub const PACKET_TAG_TUNNEL: u16 = 0x2000;
/// `PACKET_TAG_CARP_BAL_IP`: carp(4) ip balanced marker.
pub const PACKET_TAG_CARP_BAL_IP: u16 = 0x4000;

/// `MTAG_BITS`: the `%b` description of `ph_tagsset`.
pub const MTAG_BITS: &[u8] = b"\x10\x01IPSEC_IN_DONE\x02IPSEC_OUT_DONE\x03IPSEC_FLOWINFO\
\x04IPSEC_OUT_CRYPTO_NEEDED\x05IPSEC_PENDING_TDB\x06BRIDGE\x07WG\x08GRE\x09DLT\x0aPF_DIVERT\
\x0cPF_REASSEMBLED\x0dSRCROUTE\x0eTUNNEL\x0fCARP_BAL_IP";

/// `PACKET_TAG_MAXSIZE`: maximum tag payload length (that is excluding the m_tag structure).
/// Please make sure to update this value when increasing the payload length for an existing
/// packet tag type or when adding a new one that has payload larger than the value below.
pub const PACKET_TAG_MAXSIZE: usize = 80;

/// `IFQ_DEFPRIO` (`<net/if.h>`, not moved yet): the default packet priority.
const IFQ_DEFPRIO: u8 = 3;

/// `CACHELINESIZE` (`<sys/percpu.h>`, not moved yet): the `mextrefs` items' alignment.
const CACHELINESIZE: usize = 64;

/// `mclsizes[MCLPOOLS]`: the mbuf cluster pools' sizes.
pub const MCLSIZES: [u32; 8] = [
    MCLBYTES as u32,     // must be at slot 0
    MCLBYTES as u32 + 2, // ETHER_ALIGNED 2k mbufs
    4 * 1024,
    8 * 1024,
    (9 * 1024) + 128, // use more of the pool page for ETHER_ALIGNED etc
    12 * 1024,
    16 * 1024,
    64 * 1024,
];

/// `mclnames`: the cluster pools' names, `mbinit`'s two formats (`"mcl%dk"`, and
/// `"mcl%dk%u"` with the low ten bits of a size that has them).
pub const MCLNAMES: [&str; 8] = [
    "mcl2k", "mcl2k2", "mcl4k", "mcl8k", "mcl9k128", "mcl12k", "mcl16k", "mcl64k",
];

/// The item size of each cluster pool (`pr_size`, `roundup(mclsizes[i], 64)`): what
/// `m_clpool` compares a length with, and every cluster's `ext_size`. Each pool's item is a
/// `Bytes` of exactly this length, so a cluster lends all of it (`mcl2k2`'s 2050 bytes are a
/// 2112-byte item, as the C's `ext_size` is its `pr_size`; checked at compile time below).
const MCL_ITEM: [u32; 8] = {
    let mut item = [0u32; 8];
    let mut i = 0;
    while i < item.len() {
        item[i] = MCLSIZES[i].next_multiple_of(64);
        i += 1;
    }
    item
};

/// `m_types`: the ddb names of the `MT_*` types.
pub static M_TYPES: [&str; MT_NTYPES] = ["fre", "dat", "hdr", "nam", "opt", "ftb", "ctl", "oob"];

/// `enum mbstat_counters`: the counters after the per-type ones in `mbstat`.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MbstatCounters {
    /// `mbs_drops`.
    MbsDrops = MT_NTYPES,
    /// `mbs_wait`.
    MbsWait,
    /// `mbs_drain`.
    MbsDrain,
    /// `mbs_defrag_alloc`.
    MbsDefragAlloc,
    /// `mbs_prepend_alloc`.
    MbsPrependAlloc,
    /// `mbs_pullup_alloc`.
    MbsPullupAlloc,
    /// `mbs_pullup_copy`.
    MbsPullupCopy,
    /// `mbs_pulldown_alloc`.
    MbsPulldownAlloc,
    /// `mbs_pulldown_copy`.
    MbsPulldownCopy,
    /// `mbs_ncounters`.
    MbsNcounters,
}

/// `mbs_ncounters`: the mbuf statistics counters, the `MT_*` types first, then
/// [`MbstatCounters`].
pub const MBS_NCOUNTERS: usize = MbstatCounters::MbsNcounters as usize;

/// What an mbuf keeps beside its flags (`Body`'s six bytes): its type and its parked mark.
#[repr(C)]
pub struct MbufMeta {
    /// `m_type`: type of data in this mbuf.
    m_type: Cell<i16>,
    /// The parked mark (`hal::pool::Parkable`): 1 while `bsd` holds the mbuf by address.
    park: ParkWord,
    _r: [u8; 3],
}

/// `struct m_tag`: a packet tag and its data (`PACKET_TAG_MAXSIZE` bytes of `mtagpl`'s item
/// after the C's structure: 96 bytes, the C's item), on its packet's chain (`ph_tags`).
#[repr(C)]
pub struct MTag {
    /// `m_tag_link`: list of packet tags, the next tag owned.
    pub(crate) link: SlistLink<MTag>,
    /// `m_tag_id`: tag ID.
    pub m_tag_id: Cell<u16>,
    /// `m_tag_len`: length of data.
    pub m_tag_len: Cell<u16>,
    /// The parked mark (`hal::pool::Parkable`): 1 while `bsd` holds the tag by address.
    park: ParkWord,
    _r: [u8; 3],
    /// `(t + 1)`: the tag's data, written in place by `bsd` through [`MTag::data`].
    data: Cell<[u8; PACKET_TAG_MAXSIZE]>,
}

impl MTag {
    /// A tag of `id` with `len` bytes of data, on no list.
    pub const fn new(id: u16, len: u16) -> Self {
        Self {
            link: SlistLink::new(),
            m_tag_id: Cell::new(id),
            m_tag_len: Cell::new(len),
            park: ParkWord::new(),
            _r: [0; 3],
            data: Cell::new([0; PACKET_TAG_MAXSIZE]),
        }
    }

    /// `(t + 1)`'s address: the tag's data, which `bsd` reads and writes as its own structure
    /// (`struct tdb_ident`, `struct pf_fragment_tag`, ...); never dereferenced here.
    pub fn data(&self) -> *mut u8 {
        self.data.as_ptr().cast::<u8>()
    }

    /// The tag's data, a copy (`m_tag_copy`'s source).
    pub fn bytes(&self) -> [u8; PACKET_TAG_MAXSIZE] {
        self.data.get()
    }

    /// The tag's data, to write it.
    pub fn data_mut(&mut self) -> &mut [u8; PACKET_TAG_MAXSIZE] {
        self.data.get_mut()
    }
}

/// `struct pkthdr_pf`: pf stuff. The two words are pf's (`struct pf_state_key *`, `struct
/// inpcb *`), opaque here: written only by `m_move_hdr`'s clear and the `Pf` bridge's legs.
#[repr(C)]
pub struct PkthdrPf {
    /// `statekey`: pf stackside statekey (pf's, through the `Pf` bridge).
    pub statekey: Cell<usize>,
    /// `inp`: connected pcb for outgoing packet (pf's, through the `Pf` bridge).
    pub inp: Cell<usize>,
    /// `qid`: queue id.
    pub qid: Cell<u32>,
    /// `tag`: tag id.
    pub tag: Cell<u16>,
    /// `delay`: delay packet by X ms.
    pub delay: Cell<u16>,
    /// `flags`: `PF_TAG_*`.
    pub flags: Cell<u8>,
    /// `routed`.
    pub routed: Cell<u8>,
    /// `prio`.
    pub prio: Cell<u8>,
    /// `pad`.
    pub pad: Cell<[u8; 1]>,
}

impl PkthdrPf {
    /// Every member 0 but `prio` (`memset(&m->m_pkthdr, 0, ..)`'s part, then `m_inithdr`'s
    /// priority).
    const fn new(prio: u8) -> Self {
        Self {
            statekey: Cell::new(0),
            inp: Cell::new(0),
            qid: Cell::new(0),
            tag: Cell::new(0),
            delay: Cell::new(0),
            flags: Cell::new(0),
            routed: Cell::new(0),
            prio: Cell::new(prio),
            pad: Cell::new([0]),
        }
    }
}

/// `struct pkthdr`: record/packet header in first mbuf of chain; live while `M_PKTHDR` is set.
/// Its drop is `m_clearhdr`'s: the tags freed, pf's references dropped.
#[repr(C)]
pub struct Pkthdr {
    /// `ph_cookie`: additional data (`if_input_proto`'s function, TCP's offset in a batch).
    pub ph_cookie: Cell<usize>,
    /// `ph_tags`: list of packet tags (`SLIST_HEAD`, one word: the first tag, owned).
    pub ph_tags: SlistLink<MTag>,
    /// `ph_timestamp`: packet timestamp.
    pub ph_timestamp: Cell<i64>,
    /// `len`: total packet length.
    pub len: Cell<i32>,
    /// `ph_rtableid`: routing table id.
    pub ph_rtableid: Cell<u32>,
    /// `ph_ifidx`: rcv interface index.
    pub ph_ifidx: Cell<u32>,
    /// `ph_tagsset`: mtags attached.
    pub ph_tagsset: Cell<u16>,
    /// `ph_flowid`: pseudo unique flow id.
    pub ph_flowid: Cell<u16>,
    /// `csum_flags`: checksum flags.
    pub csum_flags: Cell<u16>,
    /// `ether_vtag`: Ethernet 802.1p+Q vlan tag.
    pub ether_vtag: Cell<u16>,
    /// `ph_mss`: TCP max segment size.
    pub ph_mss: Cell<u16>,
    /// `ph_loopcnt`: mbuf is looping in kernel.
    pub ph_loopcnt: Cell<u8>,
    /// `ph_family`: af, used when queueing.
    pub ph_family: Cell<u8>,
    /// `pf`.
    pub pf: PkthdrPf,
}

impl Pkthdr {
    /// `m_inithdr`'s header: every member 0 but `pf.prio`, `IFQ_DEFPRIO`.
    pub const fn new() -> Self {
        Self {
            ph_cookie: Cell::new(0),
            ph_tags: SlistLink::new(),
            ph_timestamp: Cell::new(0),
            len: Cell::new(0),
            ph_rtableid: Cell::new(0),
            ph_ifidx: Cell::new(0),
            ph_tagsset: Cell::new(0),
            ph_flowid: Cell::new(0),
            csum_flags: Cell::new(0),
            ether_vtag: Cell::new(0),
            ph_mss: Cell::new(0),
            ph_loopcnt: Cell::new(0),
            ph_family: Cell::new(0),
            pf: PkthdrPf::new(IFQ_DEFPRIO),
        }
    }

    /// `to->m_pkthdr = from->m_pkthdr` but for what the copy must not share: no tags, pf's
    /// two words 0 (`m_dup_pkthdr` links its own references after).
    fn copy_of(from: &Pkthdr) -> Self {
        let h = Self::new();
        h.ph_cookie.set(from.ph_cookie.get());
        h.ph_timestamp.set(from.ph_timestamp.get());
        h.len.set(from.len.get());
        h.ph_rtableid.set(from.ph_rtableid.get());
        h.ph_ifidx.set(from.ph_ifidx.get());
        h.ph_tagsset.set(from.ph_tagsset.get());
        h.ph_flowid.set(from.ph_flowid.get());
        h.csum_flags.set(from.csum_flags.get());
        h.ether_vtag.set(from.ether_vtag.get());
        h.ph_mss.set(from.ph_mss.get());
        h.ph_loopcnt.set(from.ph_loopcnt.get());
        h.ph_family.set(from.ph_family.get());
        h.pf.qid.set(from.pf.qid.get());
        h.pf.tag.set(from.pf.tag.get());
        h.pf.delay.set(from.pf.delay.get());
        h.pf.flags.set(from.pf.flags.get());
        h.pf.routed.set(from.pf.routed.get());
        h.pf.prio.set(from.pf.prio.get());
        h.pf.pad.set(from.pf.pad.get());
        h
    }

    /// `m_tag_delete_chain`: every tag freed, one at a time (the C's `SLIST_REMOVE_HEAD`
    /// loop), `ph_tagsset` cleared.
    pub(crate) fn delete_tags(&mut self) {
        while let Some(mut t) = self.ph_tags.take() {
            if let Some(next) = t.link.take() {
                self.ph_tags.set(next);
            }
        }
        self.ph_tagsset.set(0);
    }
}

impl Default for Pkthdr {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Pkthdr {
    /// `m_clearhdr` (`m_free`'s, `m_removehdr`'s, `m_resethdr`'s): the tags deleted, then pf's
    /// state key and pcb references dropped (`NPF > 0`).
    fn drop(&mut self) {
        self.delete_tags();
        if let Some(pf) = pf_hooks::pf() {
            pf.mbuf_unlink_state_key(&self.pf);
            pf.mbuf_unlink_inpcb(&self.pf);
        }
    }
}

/// One cluster of each pool's size (`mclpools[i]`'s item), owned.
pub enum ClusterBox {
    /// `mcl2k`.
    K2(PoolBox<Bytes<2048>>),
    /// `mcl2k2` (2050 bytes asked, the 2112 of its item lent).
    K2a(PoolBox<Bytes<2112>>),
    /// `mcl4k`.
    K4(PoolBox<Bytes<4096>>),
    /// `mcl8k`.
    K8(PoolBox<Bytes<8192>>),
    /// `mcl9k128`.
    K9(PoolBox<Bytes<9344>>),
    /// `mcl12k`.
    K12(PoolBox<Bytes<12288>>),
    /// `mcl16k`.
    K16(PoolBox<Bytes<16384>>),
    /// `mcl64k`.
    K64(PoolBox<Bytes<65536>>),
}

impl ClusterBox {
    /// A cluster of `mclpools[i]` (`pool_get`), `None` when the pool has none.
    fn get(i: usize, flags: i32) -> Option<Self> {
        Some(match i {
            0 => Self::K2(MCL2K.get_bytes(flags)?),
            1 => Self::K2a(MCL2K2.get_bytes(flags)?),
            2 => Self::K4(MCL4K.get_bytes(flags)?),
            3 => Self::K8(MCL8K.get_bytes(flags)?),
            4 => Self::K9(MCL9K128.get_bytes(flags)?),
            5 => Self::K12(MCL12K.get_bytes(flags)?),
            6 => Self::K16(MCL16K.get_bytes(flags)?),
            _ => Self::K64(MCL64K.get_bytes(flags)?),
        })
    }

    /// The cluster's pool, `mclpools`' index.
    fn index(&self) -> usize {
        match self {
            Self::K2(_) => 0,
            Self::K2a(_) => 1,
            Self::K4(_) => 2,
            Self::K8(_) => 3,
            Self::K9(_) => 4,
            Self::K12(_) => 5,
            Self::K16(_) => 6,
            Self::K64(_) => 7,
        }
    }

    /// The cluster's bytes (`ext_buf`, `ext_size` of them).
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::K2(b) => b.bytes(),
            Self::K2a(b) => b.bytes(),
            Self::K4(b) => b.bytes(),
            Self::K8(b) => b.bytes(),
            Self::K9(b) => b.bytes(),
            Self::K12(b) => b.bytes(),
            Self::K16(b) => b.bytes(),
            Self::K64(b) => b.bytes(),
        }
    }

    /// The cluster's bytes, to write them.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        match self {
            Self::K2(b) => b.bytes_mut(),
            Self::K2a(b) => b.bytes_mut(),
            Self::K4(b) => b.bytes_mut(),
            Self::K8(b) => b.bytes_mut(),
            Self::K9(b) => b.bytes_mut(),
            Self::K12(b) => b.bytes_mut(),
            Self::K16(b) => b.bytes_mut(),
            Self::K64(b) => b.bytes_mut(),
        }
    }

    /// `ext_buf`, the first byte's address (`bsd`'s `mtod`, a driver's DMA load); never
    /// dereferenced here.
    pub fn as_raw(&self) -> *mut u8 {
        match self {
            Self::K2(b) => b.as_raw(),
            Self::K2a(b) => b.as_raw(),
            Self::K4(b) => b.as_raw(),
            Self::K8(b) => b.as_raw(),
            Self::K9(b) => b.as_raw(),
            Self::K12(b) => b.as_raw(),
            Self::K16(b) => b.as_raw(),
            Self::K64(b) => b.as_raw(),
        }
    }
}

/// `struct m_ext_refs`: a cluster shared by several mbufs, made on the first share
/// (`m_extref`); the count is its `PoolArc`'s (the C's `refs`), and the last reference's drop
/// zeroes the bytes when `m_zero` asked (`zero`) and gives the cluster back to its pool
/// (`m_extfree_refs`). Padded so that the `mextrefs` item (the count, the pool and this) is
/// 64 bytes, the C's `sizeof(struct m_ext_refs)` at `CACHELINESIZE`.
#[repr(C)]
pub struct ExtRefs {
    cluster: ClusterBox,
    /// `zero`: zero the buffer when the last reference goes. Only ever set, by `m_zero`.
    zero: AtomicBool,
    _pad: [u64; 2],
}

impl ExtRefs {
    /// `m_extfree_refs`'s `if (mrefs->zero) explicit_bzero(...)`: the cluster's bytes zeroed
    /// when `m_zero` asked, before the cluster goes back to its pool.
    fn zero_if_asked(&mut self) {
        if self.zero.load(Ordering::Relaxed) {
            explicit_bzero(self.cluster.bytes_mut());
        }
    }
}

impl Drop for ExtRefs {
    fn drop(&mut self) {
        self.zero_if_asked();
    }
}

/// How an mbuf holds its cluster: alone, or shared (the C's `ext_free_fn` index
/// `MEXTFREE_POOL` and `m_extfree_refs_fn`).
pub enum ExtKind {
    /// A cluster of this mbuf's alone (`m_extfree_pool`).
    Pool(ClusterBox),
    /// A cluster shared since a `m_copym` or `m_split` (`m_extfree_refs`).
    Shared(PoolArc<ExtRefs>),
}

/// `struct mbuf_ext`: the external storage of an mbuf, live while `M_EXT` is set; its drop
/// is `m_extfree`'s.
pub struct Ext {
    kind: ExtKind,
    /// `ext_size`: the cluster's size, its pool's item size.
    size: u32,
    /// `ext_buf`: the cluster's address, its provenance exposed when the description was made
    /// (`bsd`'s `mtod` reads it on every packet; the cluster's own needs the size's match).
    buf: usize,
}

impl Ext {
    /// The description of `kind`'s cluster, `size` bytes (`MEXTADD`'s `ext_buf`/`ext_size`).
    fn new(kind: ExtKind, size: u32) -> Self {
        let buf = match &kind {
            ExtKind::Pool(c) => c.as_raw(),
            ExtKind::Shared(r) => r.cluster.as_raw(),
        }
        .expose_provenance();
        Self { kind, size, buf }
    }

    /// The cluster.
    fn cluster(&self) -> &ClusterBox {
        match &self.kind {
            ExtKind::Pool(c) => c,
            ExtKind::Shared(r) => &r.cluster,
        }
    }

    /// `ext_size`.
    pub fn ext_size(&self) -> u32 {
        self.size
    }

    /// `ext_buf`'s address (`bsd`'s `mtod`, a driver's DMA load); never dereferenced here.
    pub fn ext_buf(&self) -> *mut u8 {
        core::ptr::with_exposed_provenance_mut::<u8>(self.buf)
    }

    /// The cluster's bytes.
    pub fn bytes(&self) -> &[u8] {
        self.cluster().bytes()
    }

    /// The cluster's bytes, to write them: `None` while another mbuf shares the cluster.
    fn bytes_mut(&mut self) -> Option<&mut [u8]> {
        match &mut self.kind {
            ExtKind::Pool(c) => Some(c.bytes_mut()),
            ExtKind::Shared(r) => PoolArc::get_mut(r).map(|r| r.cluster.bytes_mut()),
        }
    }

    /// `m_ext_refs_shared`: whether another mbuf shares the cluster now.
    pub fn is_shared(&self) -> bool {
        match &self.kind {
            ExtKind::Pool(_) => false,
            ExtKind::Shared(r) => PoolArc::count(r) > 1,
        }
    }
}

/// `struct mbuf`: `MSIZE` bytes of `mbufpl`, aligned to 64 as `m_pool_init(&mbpool, MSIZE,
/// 64, ..)` asks.
///
/// The links own the next mbuf of the chain (`m_next`) and the next packet (`m_nextpkt`);
/// the data is `m_len` bytes at `m_off` in the data storage ([`Mbuf::storage`]); the packet
/// header and the cluster description are typed values of the overlay, live while
/// `M_PKTHDR` and `M_EXT` say so.
#[repr(C, align(64))]
pub struct Mbuf {
    /// `m_next`: next buffer in chain.
    pub m_next: SlistLink<Mbuf>,
    /// `m_nextpkt`: next chain in queue/record.
    pub m_nextpkt: SlistLink<Mbuf>,
    /// `m_data`, as an offset into the data storage.
    m_off: Cell<u32>,
    /// `m_len`: amount of data in this mbuf.
    m_len: Cell<u32>,
    /// `m_flags`, `m_type`, the parked mark and `M_dat`.
    body: Body<Pkthdr, Ext, MbufMeta, M_PKTHDR, M_EXT, MLEN>,
}

/// An owned mbuf (`struct mbuf *` that this code owns): its drop is `m_free`'s, for the whole
/// chain and packet list it owns.
pub type MbufBox = PoolBox<Mbuf>;

/// `m_flags` (`m->m_flags`), read and set: [`MFlags::set`] keeps `M_PKTHDR` and `M_EXT`, which
/// only the typed transitions change.
pub struct MFlags<'a>(&'a Mbuf);

impl MFlags<'_> {
    /// `m->m_flags`.
    #[inline]
    pub fn get(&self) -> u16 {
        self.0.body.flags()
    }

    /// `m->m_flags = v`. `panic(9)` when `v` would change `M_PKTHDR` or `M_EXT`: a packet
    /// header is removed by `m_removehdr` (`M_MOVE_HDR`), added by `m_inithdr` or
    /// `m_dup_pkthdr`, a cluster by `m_clget`; a flag word cannot make one appear or vanish.
    #[inline]
    pub fn set(&self, v: u16) {
        if (v ^ self.get()) & (M_PKTHDR | M_EXT) != 0 {
            kpanic(format_args!(
                "m_flags: {:#x} -> {v:#x} changes M_PKTHDR or M_EXT",
                self.get()
            ));
        }
        self.0.body.set_flags(v);
    }
}

impl Mbuf {
    /// A fresh mbuf of `type_`, no data, no header (`m_get`'s initialisation).
    fn fresh(type_: i16) -> Self {
        Self {
            m_next: SlistLink::new(),
            m_nextpkt: SlistLink::new(),
            m_off: Cell::new(0),
            m_len: Cell::new(0),
            body: Body::new(
                MbufMeta {
                    m_type: Cell::new(type_),
                    park: ParkWord::new(),
                    _r: [0; 3],
                },
                0,
            ),
        }
    }

    /// `m_next`: next buffer in chain.
    #[inline]
    pub fn m_next(&self) -> &SlistLink<Mbuf> {
        &self.m_next
    }

    /// `m_nextpkt`: next chain in queue/record.
    #[inline]
    pub fn m_nextpkt(&self) -> &SlistLink<Mbuf> {
        &self.m_nextpkt
    }

    /// `m_len`: amount of data in this mbuf.
    #[inline]
    pub fn m_len(&self) -> &Cell<u32> {
        &self.m_len
    }

    /// `m_type`: type of data in this mbuf.
    #[inline]
    pub fn m_type(&self) -> &Cell<i16> {
        &self.body.x().m_type
    }

    /// `m_flags`.
    #[inline]
    pub fn m_flags(&self) -> MFlags<'_> {
        MFlags(self)
    }

    /// `m->m_flags`, read.
    #[inline]
    pub fn flags(&self) -> u16 {
        self.body.flags()
    }

    /// The flags `bits` set (the tag bits kept).
    #[inline]
    fn set_bits(&self, bits: u16) {
        self.body.set_flags(self.body.flags() | bits);
    }

    /// The flags `bits` cleared (the tag bits kept).
    #[inline]
    fn clear_bits(&self, bits: u16) {
        self.body.set_flags(self.body.flags() & !bits);
    }

    /// The packet header, while `M_PKTHDR` is set.
    #[inline]
    pub fn pkthdr(&self) -> Option<&Pkthdr> {
        self.body.p()
    }

    /// The packet header, to change its tags.
    #[inline]
    pub fn pkthdr_mut(&mut self) -> Option<&mut Pkthdr> {
        self.body.p_mut()
    }

    /// `m_pkthdr`: the packet header. `panic(9)` for an mbuf without one (where the C reads
    /// the data area as a header).
    #[inline]
    pub fn m_pkthdr(&self) -> &Pkthdr {
        match self.body.p() {
            Some(h) => h,
            None => kpanic(format_args!("m_pkthdr: an mbuf without M_PKTHDR")),
        }
    }

    /// `m_ext`: the external storage, while `M_EXT` is set.
    #[inline]
    pub fn ext(&self) -> Option<&Ext> {
        self.body.e()
    }

    /// `M_DATABUF(m)`..`M_SIZE(m)`: the data storage, the cluster's bytes with `M_EXT`, else
    /// the data area after the header with `M_PKTHDR`, else the whole data area.
    #[inline]
    pub fn storage(&self) -> &[u8] {
        match self.body.e() {
            Some(e) => e.bytes(),
            None => self.body.inline().unwrap_or(&[]),
        }
    }

    /// The data storage, to write it: `None` when it is read-only (`M_READONLY`: a cluster
    /// without `M_EXTWR`, or shared).
    #[inline]
    pub fn storage_mut(&mut self) -> Option<&mut [u8]> {
        if self.body.e().is_none() {
            return self.body.inline_mut();
        }
        if self.body.flags() & M_EXTWR == 0 {
            return None;
        }
        self.body.e_mut().and_then(Ext::bytes_mut)
    }

    /// `M_SIZE(m)`: the size of the data storage.
    #[inline]
    pub fn storage_len(&self) -> usize {
        match self.body.e() {
            Some(e) => e.size as usize,
            None if self.body.p().is_some() => MHLEN,
            None => MLEN,
        }
    }

    /// `M_DATABUF(m)`'s address; never dereferenced here.
    #[inline]
    pub fn storage_raw(&self) -> *mut u8 {
        match self.body.e() {
            Some(e) => e.ext_buf(),
            None => self.body.as_raw().wrapping_add(self.body.inline_start()),
        }
    }

    /// Where the data starts in the storage (`m->m_data - M_DATABUF(m)`).
    #[inline]
    pub fn data_off(&self) -> usize {
        self.m_off.get() as usize
    }

    /// `m->m_data = M_DATABUF(m) + off`. `panic(9)` past the storage's end.
    #[inline]
    pub fn set_data_off(&self, off: usize) {
        if off > self.storage_len() {
            kpanic(format_args!(
                "m_data: offset {off} past the {} bytes of storage",
                self.storage_len()
            ));
        }
        self.m_off.set(off as u32);
    }

    /// `m->m_data += n` (`-=` for a negative `n`), checked as [`Mbuf::set_data_off`].
    #[inline]
    pub fn data_add(&self, n: isize) {
        match self.data_off().checked_add_signed(n) {
            Some(off) => self.set_data_off(off),
            None => kpanic(format_args!("m_data: {n} before the storage")),
        }
    }

    /// `mtod(m, caddr_t)`, `m_len` bytes: the data. `panic(9)` when the offset and the length
    /// leave the storage (where the C reads past the mbuf).
    #[inline]
    pub fn data(&self) -> &[u8] {
        let (off, len) = (self.data_off(), self.m_len.get() as usize);
        match self.storage().get(off..off + len) {
            Some(d) => d,
            None => kpanic(format_args!(
                "mtod: {len} bytes at {off} past the storage of {}",
                self.storage_len()
            )),
        }
    }

    /// The data, to write it: `None` when the storage is read-only (`M_READONLY`).
    #[inline]
    pub fn data_mut(&mut self) -> Option<&mut [u8]> {
        let (off, len) = (self.data_off(), self.m_len.get() as usize);
        let s = self.storage_mut()?;
        match s.get_mut(off..off + len) {
            Some(d) => Some(d),
            None => kpanic(format_args!("mtod: {len} bytes at {off} past the storage")),
        }
    }

    /// `mtod(m, caddr_t)`'s address, for `bsd` and the drivers' DMA loads; never dereferenced
    /// here.
    #[inline]
    pub fn data_raw(&self) -> *mut u8 {
        self.storage_raw().wrapping_add(self.data_off())
    }

    /// `M_READONLY(m)`: the data storage cannot be written: a cluster without `M_EXTWR`, or
    /// one that another mbuf shares.
    #[inline]
    pub fn is_readonly(&self) -> bool {
        match self.body.e() {
            Some(e) => self.body.flags() & M_EXTWR == 0 || e.is_shared(),
            None => false,
        }
    }

    /// The mbuf after this one in its chain, to change it (`m->m_next`).
    #[inline]
    pub fn next_mut(&mut self) -> Option<&mut Mbuf> {
        self.m_next.get_mut()
    }
}

impl Drop for Mbuf {
    /// `m_free`'s body for this mbuf (the `mbstat` count, `M_ZEROIZE`), then the chain it
    /// owns freed one mbuf at a time, `M_ZEROIZE` carried along as the C's `m_freem` loop
    /// carries it, and the packets its `m_nextpkt` owns, one at a time too. The header (its
    /// tags, pf's references) and the cluster go with the overlay's drop.
    fn drop(&mut self) {
        {
            let _s = splraise(IPL_NET);
            if let Ok(t) = usize::try_from(self.m_type().get())
                && t < MT_NTYPES
            {
                MBSTAT.dec(t);
            }
        }
        let mut zeroize = self.flags() & M_ZEROIZE != 0;
        if zeroize {
            m_zero(self);
        }
        while let Some(mut n) = self.m_next.take() {
            if let Some(after) = n.m_next.take() {
                self.m_next.set(after);
            }
            if zeroize {
                n.set_bits(M_ZEROIZE);
            }
            zeroize = n.flags() & M_ZEROIZE != 0;
            drop(n);
        }
        while let Some(mut p) = self.m_nextpkt.take() {
            if let Some(after) = p.m_nextpkt.take() {
                self.m_nextpkt.set(after);
            }
            drop(p);
        }
    }
}

hal::slist_adapter!(
    /// `m_nextpkt`'s list: a packet queue (`struct mbuf_list`).
    pub MPkt: Mbuf, m_nextpkt
);

/// `struct mbuf_list`: packets linked through `m_nextpkt`, with the C's head, tail and
/// length (24 bytes).
pub type MbufList = OwnedSlist<MPkt>;

/// The source of the mbuf and cluster pools' pages: `m_pool_allocator`, `pool_allocator_multi`
/// with the memory limit in front and the pools' DMA constraint.
pub struct MbufPages;

/// `mbufpl`: the mbuf pool.
pub static MBUFPL: BoxPool<Mbuf> = BoxPool::new();
/// `mtagpool`, `mtagpl`: the packet tag pool.
pub static MTAGPL: BoxPool<MTag> = BoxPool::new();
/// `mclpools[0]`, `mcl2k`.
static MCL2K: BoxPool<Bytes<2048>> = BoxPool::new();
/// `mclpools[1]`, `mcl2k2`: `MCLBYTES + 2` asked, its 64-aligned item lent whole.
static MCL2K2: BoxPool<Bytes<2112>> = BoxPool::new();
/// `mclpools[2]`, `mcl4k`.
static MCL4K: BoxPool<Bytes<4096>> = BoxPool::new();
/// `mclpools[3]`, `mcl8k`.
static MCL8K: BoxPool<Bytes<8192>> = BoxPool::new();
/// `mclpools[4]`, `mcl9k128`.
static MCL9K128: BoxPool<Bytes<9344>> = BoxPool::new();
/// `mclpools[5]`, `mcl12k`.
static MCL12K: BoxPool<Bytes<12288>> = BoxPool::new();
/// `mclpools[6]`, `mcl16k`.
static MCL16K: BoxPool<Bytes<16384>> = BoxPool::new();
/// `mclpools[7]`, `mcl64k`.
static MCL64K: BoxPool<Bytes<65536>> = BoxPool::new();
/// `m_ext_refs_pool`, `mextrefs`.
static MEXTREFS: TypedPool<ExtRefs> = TypedPool::new();

/// `mbstat`: the mbuf statistics counters, per CPU, from boot.
static MBSTAT: Counters<MBS_NCOUNTERS> = Counters::new();

/// `max_linkhdr`: largest link-level header.
pub static MAX_LINKHDR: AtomicI32 = AtomicI32::new(0);
/// `max_protohdr`: largest protocol header.
pub static MAX_PROTOHDR: AtomicI32 = AtomicI32::new(0);
/// `max_hdr`: largest link+protocol header.
pub static MAX_HDR: AtomicI32 = AtomicI32::new(0);

/// \[a\] `mbuf_mem_limit`: how much memory can be allocated.
pub static MBUF_MEM_LIMIT: AtomicU64 = AtomicU64::new(0);
/// \[a\] `mbuf_mem_alloc`: how much memory has been allocated.
pub static MBUF_MEM_ALLOC: AtomicU64 = AtomicU64::new(0);

/// `m_pool_allocator`'s page source.
static MBUF_PAGES: MbufPages = MbufPages;

/// Whether `m_pool_noconstraints` lifted the pools' DMA constraint (`kp_mbuf_contig` instead
/// of `kp_dma_contig`).
static NOCONSTRAINTS: AtomicBool = AtomicBool::new(false);

/// \[a\] `nmbclust`: the limit on the number of mbuf clusters, `bsd`'s variable (`conf/param.c`'s,
/// read by `kern.maxclusters` and pf), given by `mbinit`.
static NMBCLUST: OnceStatic<&'static AtomicI64> = OnceStatic::new();

impl Pooled for Mbuf {
    const POOL: &'static BoxPool<Self> = &MBUFPL;
}

impl Parkable for Mbuf {
    fn park_word(&self) -> &ParkWord {
        &self.body.x().park
    }
}

impl Pooled for MTag {
    const POOL: &'static BoxPool<Self> = &MTAGPL;
}

impl Parkable for MTag {
    fn park_word(&self) -> &ParkWord {
        &self.park
    }
}

/// The pool flags of an mbuf allocation's `how` (`M_WAIT` waits).
#[inline]
fn pr_flags(how: i32) -> i32 {
    if how == M_WAIT { PR_WAITOK } else { PR_NOWAIT }
}

/// `n` as the C's `int` length, saturated (a length or an offset of the stack fits).
#[inline]
fn as_int(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// A length or an offset the C keeps in an `int`, as an index (negative ones are 0).
#[inline]
fn as_idx(n: i32) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// `mbstat_inc`: bumps counter `c` (an `MT_*` type or an [`MbstatCounters`] value) at
/// `splnet`.
#[inline]
pub fn mbstat_inc(c: usize) {
    let _s = splraise(IPL_NET);
    MBSTAT.inc(c);
}

/// `kern.mbstat`'s structure: the counters summed over the CPUs (`counters_read`).
pub fn mbstat_read() -> Mbstat {
    let counters = MBSTAT.read();
    let at = |c: MbstatCounters| counters[c as usize];
    let mut mbs = Mbstat {
        m_drops: at(MbstatCounters::MbsDrops),
        m_wait: at(MbstatCounters::MbsWait),
        m_drain: at(MbstatCounters::MbsDrain),
        m_mtypes: [0; MT_NTYPES],
        m_defrag_alloc: at(MbstatCounters::MbsDefragAlloc),
        m_prepend_alloc: at(MbstatCounters::MbsPrependAlloc),
        m_pullup_alloc: at(MbstatCounters::MbsPullupAlloc),
        m_pullup_copy: at(MbstatCounters::MbsPullupCopy),
        m_pulldown_alloc: at(MbstatCounters::MbsPulldownAlloc),
        m_pulldown_copy: at(MbstatCounters::MbsPulldownCopy),
    };
    mbs.m_mtypes.copy_from_slice(&counters[..MT_NTYPES]);
    mbs
}

/// `mbinit`: initialize the mbuf allocator. `nmbclust` is the cluster limit's variable
/// (`conf/param.c`'s, `bsd`'s until `kern.maxclusters` moves), whose value sets the memory
/// limit.
pub fn mbinit(nmbclust: &'static AtomicI64) {
    MBUF_MEM_ALLOC.store(0, Ordering::Relaxed);

    // CTASSERT(MSIZE == sizeof(struct mbuf)) and the DIAGNOSTIC checks of mclsizes: the
    // compile-time checks at the end of this file.
    let made = m_pool_init(&MBUFPL, "mbufpl")
        && MTAGPL.init(IPL_NET, 0, "mtagpl").is_ok()
        && MEXTREFS
            .init_aligned(IPL_NET, 0, "mextrefs", CACHELINESIZE)
            .is_ok()
        && m_pool_init(&MCL2K, MCLNAMES[0])
        && m_pool_init(&MCL2K2, MCLNAMES[1])
        && m_pool_init(&MCL4K, MCLNAMES[2])
        && m_pool_init(&MCL8K, MCLNAMES[3])
        && m_pool_init(&MCL9K128, MCLNAMES[4])
        && m_pool_init(&MCL12K, MCLNAMES[5])
        && m_pool_init(&MCL16K, MCLNAMES[6])
        && m_pool_init(&MCL64K, MCLNAMES[7]);
    // pool_init(9) cannot fail in the C; here a pool made already (a host test's second
    // mbinit) is kept.
    let _ = made;

    let _ = NMBCLUST.set(nmbclust);
    let error = nmbclust_update(nmbclust.load(Ordering::Relaxed));
    kassert!(error.is_ok());
    let _ = error;
}

/// `mbcpuinit`: the pool caches per CPU (the per-CPU `mbstat` exists from boot).
pub fn mbcpuinit() {
    MBUFPL.cache_init();
    MTAGPL.cache_init();
    MEXTREFS.cache_init();
    MCL2K.cache_init();
    MCL2K2.cache_init();
    MCL4K.cache_init();
    MCL8K.cache_init();
    MCL9K128.cache_init();
    MCL12K.cache_init();
    MCL16K.cache_init();
    MCL64K.cache_init();
}

/// `nmbclust_update`: sets the cluster limit and the memory limit derived from it.
pub fn nmbclust_update(newval: i64) -> Result<(), Errno> {
    if newval <= 0 || newval > i64::MAX / MCLBYTES as i64 {
        return Err(Errno::ERANGE);
    }
    // update the global mbuf memory limit
    if let Some(v) = NMBCLUST.get() {
        v.store(newval, Ordering::Relaxed);
    }
    MBUF_MEM_LIMIT.store(newval as u64 * MCLBYTES as u64, Ordering::Relaxed);

    MBUFPL.wakeup();
    MCL2K.wakeup();
    MCL2K2.wakeup();
    MCL4K.wakeup();
    MCL8K.wakeup();
    MCL9K128.wakeup();
    MCL12K.wakeup();
    MCL16K.wakeup();
    MCL64K.wakeup();

    Ok(())
}

/// `m_get`: space allocation routines.
pub fn m_get(nowait: i32, type_: i32) -> Option<MbufBox> {
    kassert!(type_ >= 0 && (type_ as usize) < MT_NTYPES);

    let m = MBUFPL.get(pr_flags(nowait), || Mbuf::fresh(type_ as i16))?;

    mbstat_inc(type_ as usize);

    Some(m)
}

/// `m_gethdr`. ATTN: When changing anything here check `m_inithdr()` and `m_defrag()` those
/// may need to change as well.
pub fn m_gethdr(nowait: i32, type_: i32) -> Option<MbufBox> {
    let mut m = m_get(nowait, type_)?;
    m_inithdr(&mut m);
    Some(m)
}

/// `m_inithdr`: makes `m` the first mbuf of an empty packet: no data, a fresh header,
/// `M_PKTHDR` the only flag (a cluster it had is given back).
pub fn m_inithdr(m: &mut Mbuf) {
    // keep in sync with m_gethdr
    drop(m.body.take_e());
    m.body.set_flags(0);
    drop(m.body.set_p(Pkthdr::new()));
    m.m_off.set(0);
}

/// `m_removehdr`: makes the first mbuf of a packet a plain one: the header's tags and pf's
/// references dropped (`m_clearhdr`), the data where it was.
pub fn m_removehdr(m: &mut Mbuf) {
    kassert!(m.pkthdr().is_some());
    let at = m.data_off();
    let hdr = m.body.take_p();
    if m.body.e().is_none() {
        // The storage now starts at the area's first byte, the header's place.
        m.m_off
            .set((at + Body::<Pkthdr, Ext, MbufMeta, M_PKTHDR, M_EXT, MLEN>::INLINE_P) as u32);
    }
    drop(hdr);
}

/// `m_resethdr`: like `m_inithdr()`, but keep any associated data and mbufs.
pub fn m_resethdr(m: &mut Mbuf) {
    let Some(h) = m.body.p() else {
        kpanic(format_args!("m_resethdr: an mbuf without M_PKTHDR"));
    };
    let len = h.len.get();
    let loopcnt = h.ph_loopcnt.get();

    m.body
        .set_flags(m.body.flags() & (M_EXT | M_PKTHDR | M_EOR | M_EXTWR | M_ZEROIZE));
    // m_clearhdr: the old header's drop; like m_inithdr(), but keep any associated data and
    // mbufs.
    if let Some(h) = m.body.p_mut() {
        *h = Pkthdr::new();
        h.len.set(len);
        h.ph_loopcnt.set(loopcnt);
    }
}

/// `m_calchdrlen`: sets the packet length from the chain's.
pub fn m_calchdrlen(m: &Mbuf) {
    let mut plen = 0i32;
    let mut n = Some(m);
    while let Some(nn) = n {
        plen += nn.m_len.get() as i32;
        n = nn.m_next.get();
    }
    m.m_pkthdr().len.set(plen);
}

/// `m_getclr`: an mbuf with its data area zeroed.
pub fn m_getclr(nowait: i32, type_: i32) -> Option<MbufBox> {
    let mut m = m_get(nowait, type_)?;
    if let Some(d) = m.storage_mut() {
        d.fill(0);
    }
    Some(m)
}

/// `m_clpool`: the smallest cluster pool for a packet of `pktlen` bytes, as `mclpools`' index.
pub fn m_clpool(pktlen: u32) -> Option<usize> {
    MCL_ITEM.iter().position(|&size| pktlen <= size)
}

/// `m_clget(m, how, pktlen)` with `m`: adds a cluster of at least `pktlen` bytes to `m`
/// (`MEXTADD` with `M_EXTWR`, the data at its start); `false` when none can be had, `m`
/// unchanged.
pub fn m_clget(m: &mut Mbuf, how: i32, pktlen: u32) -> bool {
    let Some(i) = m_clpool(pktlen) else {
        // DIAGNOSTIC's message; without it the C dereferences the NULL pool.
        kpanic(format_args!("m_clget: request for {pktlen} byte cluster"));
    };
    let Some(cl) = ClusterBox::get(i, pr_flags(how)) else {
        return false;
    };
    mextadd(m, Ext::new(ExtKind::Pool(cl), MCL_ITEM[i]), M_EXTWR);
    true
}

/// `m_clget(NULL, how, pktlen)`: a new packet header mbuf with a cluster of at least `pktlen`
/// bytes.
pub fn m_clget_hdr(how: i32, pktlen: u32) -> Option<MbufBox> {
    if m_clpool(pktlen).is_none() {
        kpanic(format_args!("m_clget: request for {pktlen} byte cluster"));
    }
    let mut m = m_gethdr(how, MT_DATA)?;
    // The header mbuf goes back with the failure.
    m_clget(&mut m, how, pktlen).then_some(m)
}

/// `MCLGET(m, how)`: allocates and adds an mbuf cluster to a normal mbuf; the flag `M_EXT` is
/// set upon success.
pub fn mclget(m: &mut Mbuf, how: i32) {
    let _ = m_clget(m, how, MCLBYTES as u32);
}

/// `MCLGETL(m, how, l)`: as `MCLGET`, for a cluster of at least `l` bytes; whether it was
/// added.
pub fn mclgetl(m: &mut Mbuf, how: i32, l: u32) -> bool {
    m_clget(m, how, l)
}

/// `MEXTADD(m, buf, size, mflags, ..)`: `ext` made the mbuf's external storage, `M_EXT` set
/// with `mflags`' `M_EXTWR`, the data at its start. A cluster the mbuf had goes back.
fn mextadd(m: &mut Mbuf, ext: Ext, mflags: u16) {
    drop(m.body.set_e(ext));
    m.set_bits(mflags & M_EXTWR);
    m.m_off.set(0);
}

/// `m_free`: frees one mbuf and returns the next in its chain (`M_ZEROIZE` carried to it).
/// What the mbuf's `m_nextpkt` owns goes with it; a list unlinks its packets before.
pub fn m_free(mut m: MbufBox) -> Option<MbufBox> {
    let n = m.m_next.take();
    if let Some(n) = &n
        && m.flags() & M_ZEROIZE != 0
    {
        // propagate M_ZEROIZE to the next mbuf in the chain
        n.set_bits(M_ZEROIZE);
    }
    drop(m);
    n
}

/// `m_extref`: makes `n` share `m`'s cluster, turning the cluster into a counted one on the
/// first share (`mextrefs`, which may fail: `ENOMEM`, `m` unchanged).
fn m_extref(m: &mut Mbuf, n: &mut Mbuf, how: i32) -> Result<(), Errno> {
    let extwr = m.flags() & M_EXTWR;
    let shared = match m.body.e() {
        Some(Ext {
            kind: ExtKind::Shared(r),
            ..
        }) => r.clone(),
        Some(Ext {
            kind: ExtKind::Pool(_),
            size,
            ..
        }) => {
            let size = *size;
            let Some(Ext {
                kind: ExtKind::Pool(cl),
                ..
            }) = m.body.take_e()
            else {
                kpanic(format_args!("m_extref: the cluster moved"));
            };
            let mut slot = Some(cl);
            // `how` is the pool's flags, as the C passes it.
            let made = MEXTREFS.get_shared(how, || {
                let Some(cluster) = slot.take() else {
                    kpanic(format_args!("m_extref: the cluster moved"));
                };
                ExtRefs {
                    cluster,
                    zero: AtomicBool::new(false),
                    _pad: [0; 2],
                }
            });
            let Some(r) = made else {
                if let Some(cl) = slot.take() {
                    drop(m.body.set_e(Ext::new(ExtKind::Pool(cl), size)));
                }
                return Err(Errno::ENOMEM);
            };
            drop(m.body.set_e(Ext::new(ExtKind::Shared(r.clone()), size)));
            r
        }
        None => kpanic(format_args!("m_extref: an mbuf without M_EXT")),
    };
    let size = m.body.e().map_or(0, |e| e.size);

    mextadd(n, Ext::new(ExtKind::Shared(shared), size), extwr);

    Ok(())
}

/// `m_freem`: frees a chain and returns the next packet.
pub fn m_freem(mut m: MbufBox) -> Option<MbufBox> {
    let n = m.m_nextpkt.take();
    drop(m);
    n
}

/// `m_purge`: frees a list of packets.
pub fn m_purge(m: Option<MbufBox>) {
    let mut m = m;
    while let Some(mm) = m {
        m = m_freem(mm);
    }
}

/// `m_defrag`: mbuf chain defragmenter. This function uses some evil tricks to defragment an
/// mbuf chain into a single buffer without changing the mbuf pointer. This needs to know a lot
/// of the mbuf internals to make this work. The resulting mbuf is not aligned to IP header to
/// assist DMA transfers.
pub fn m_defrag(m: &mut Mbuf, how: i32) -> Result<(), Errno> {
    if !m.m_next.is_some() {
        return Ok(());
    }

    kassert!(m.pkthdr().is_some());

    mbstat_inc(MbstatCounters::MbsDefragAlloc as usize);
    let mut m0 = m_gethdr(how, i32::from(m.m_type().get())).ok_or(Errno::ENOBUFS)?;
    let pktlen = m.m_pkthdr().len.get();
    if pktlen > MHLEN as i32 && !mclgetl(&mut m0, how, pktlen as u32) {
        return Err(Errno::ENOBUFS);
    }
    m0.m_len.set(pktlen as u32);
    m0.m_pkthdr().len.set(pktlen);
    match m0.data_mut() {
        Some(dst) => m_copydata(m, 0, dst),
        None => kpanic(format_args!("m_defrag: a fresh mbuf read-only")),
    }

    // free chain behind and possible ext buf on the first mbuf
    drop(m.m_next.take());
    drop(m.body.take_e());
    m.clear_bits(M_EXTWR);

    // Bounce copy mbuf over to the original mbuf and set everything up. This needs to reset
    // or clear all pointers that may go into the original mbuf chain.
    if let Some(e) = m0.body.take_e() {
        m.set_bits(m0.flags() & M_EXTWR);
        drop(m.body.set_e(e));
        m.m_off.set(0);
    } else {
        m.m_off.set(0);
        let n = m0.m_len.get() as usize;
        match m.storage_mut().and_then(|s| s.get_mut(..n)) {
            Some(dst) => dst.copy_from_slice(m0.data()),
            None => kpanic(format_args!("m_defrag: no room for {n} bytes")),
        }
    }
    m.m_len.set(m0.m_len.get());
    m.m_pkthdr().len.set(m0.m_len.get() as i32);

    // the cluster is gone from m0 (taken above)
    drop(m0);

    Ok(())
}

// Mbuffer utility routines.

/// `m_prepend`: ensure `len` bytes of contiguous space at the beginning of the mbuf chain.
pub fn m_prepend(mut m: MbufBox, len: i32, how: i32) -> Option<MbufBox> {
    if len > MHLEN as i32 {
        kpanic(format_args!("mbuf prepend length too big"));
    }

    if m_leadingspace(&m) >= len {
        m.data_add(-(len as isize));
        m.m_len.set(m.m_len.get() + len as u32);
    } else {
        mbstat_inc(MbstatCounters::MbsPrependAlloc as usize);
        let mut mn = m_get(how, i32::from(m.m_type().get()))?;
        if m.pkthdr().is_some() {
            m_move_pkthdr(&mut mn, &mut m);
        }
        drop(mn.m_next.set(m));
        m = mn;
        m_align(&m, len);
        m.m_len.set(len as u32);
    }
    if let Some(h) = m.pkthdr() {
        h.len.set(h.len.get() + len);
    }
    Some(m)
}

/// `m_copym`: make a copy of an mbuf chain starting `off` bytes from the beginning, continuing
/// for `len` bytes. If `len` is `M_COPYALL`, copy to end of mbuf. The wait parameter is a
/// choice of `M_WAIT`/`M_DONTWAIT` from caller. The source's clusters are shared with the
/// copy, so the source is changed (`&mut`).
pub fn m_copym(m0: &mut Mbuf, off: i32, len: i32, wait: i32) -> Option<MbufBox> {
    if off < 0 || len < 0 {
        kpanic(format_args!("m_copym0: off {off}, len {len}"));
    }
    let copyhdr = off == 0 && m0.pkthdr().is_some();

    // The header copy is made first, before the walk lends the source's mbufs mutably. Only
    // at `off` 0, where the mbuf the walk finds is the first with data (`m0` itself, unless
    // it is empty), whose type the copy's first mbuf takes as the C's `MGET(n, wait,
    // m->m_type)`; every other copy walks the chain once, as the C.
    let mut hdr = None;
    if copyhdr {
        let Some((first, _)) = m_getptr(m0, 0) else {
            kpanic(format_args!("m_copym0: short mbuf chain"));
        };
        let mut n = m_get(wait, i32::from(first.m_type().get()))?;
        m_dup_pkthdr(&mut n, m0, wait).ok()?;
        if len != M_COPYALL {
            n.m_pkthdr().len.set(len);
        }
        hdr = Some(n);
    }

    let Some((src, off)) = m_getptr_mut(m0, off) else {
        kpanic(format_args!("m_copym0: short mbuf chain"));
    };
    let mut m: Option<&mut Mbuf> = Some(src);
    let mut off = off;
    let mut len = len;
    let mut top = MChainBuilder::new();
    while len > 0 {
        let Some(mm) = m.take() else {
            if len != M_COPYALL {
                kpanic(format_args!("m_copym0: m == NULL and not COPYALL"));
            }
            break;
        };
        // nospace: the copy made so far goes with `top`.
        let mut n = match hdr.take() {
            Some(n) => n,
            None => m_get(wait, i32::from(mm.m_type().get()))?,
        };
        let mut nlen = min(len as u32, mm.m_len.get() - off as u32) as usize;
        if mm.body.e().is_some() {
            m_extref(mm, &mut n, wait).ok()?;
            n.m_off.set(mm.m_off.get() + off as u32);
        } else {
            // The data at the source's offset in the copy's storage; where the copy's storage
            // is shorter (a header mbuf copying a plain one), what fits.
            let room = n.storage_len();
            let at = min(mm.data_off() + as_idx(off), room.saturating_sub(nlen));
            nlen = min(nlen, room - at);
            n.m_off.set(at as u32);
            let src = &mm.data()[as_idx(off)..as_idx(off) + nlen];
            match n.storage_mut().and_then(|s| s.get_mut(at..at + nlen)) {
                Some(dst) => dst.copy_from_slice(src),
                None => kpanic(format_args!("m_copym0: no room in the copy")),
            }
        }
        n.m_len.set(nlen as u32);
        if len != M_COPYALL {
            len -= nlen as i32;
        }
        off += nlen as i32;
        #[cfg(feature = "diagnostic")]
        if off > mm.m_len.get() as i32 {
            kpanic(format_args!("m_copym0 overrun"));
        }
        if off == mm.m_len.get() as i32 {
            m = mm.m_next.get_mut();
            off = 0;
        } else {
            m = Some(mm);
        }
        top.push(n);
    }
    top.into_head()
}

/// A chain built mbuf by mbuf at its tail (the C's `np = &n->m_next`), O(1) a step. A chain
/// left unfinished (`m_copym`'s `nospace`, `m_devget`'s failure) is freed as the C's
/// `m_freem(top)` frees it: from its head, so the head's `M_ZEROIZE` reaches every mbuf
/// (pfkey's secrets), never mbuf by mbuf as the list's own drop would.
struct MChainBuilder {
    chain: OwnedSlist<MChain>,
}

impl Drop for MChainBuilder {
    fn drop(&mut self) {
        drop(core::mem::take(&mut self.chain).into_head());
    }
}

hal::slist_adapter!(
    /// `m_next`'s chain, for a chain built at its tail ([`MChainBuilder`]).
    MChain: Mbuf, m_next
);

impl MChainBuilder {
    fn new() -> Self {
        Self {
            chain: OwnedSlist::new(),
        }
    }

    /// `*np = n; np = &n->m_next`: `n` (alone: its `m_next` empty) after the chain's last.
    fn push(&mut self, n: MbufBox) {
        self.chain.push_tail(n);
    }

    /// The chain's first mbuf, owning the rest: the finished chain moves out and the
    /// builder's `Drop` (an unfinished chain's) does not run on it (`ManuallyDrop`: the list
    /// left behind is empty, so nothing is forgotten).
    fn into_head(self) -> Option<MbufBox> {
        let mut this = core::mem::ManuallyDrop::new(self);
        core::mem::take(&mut this.chain).into_head()
    }
}

/// `m_copydata`: copy data from an mbuf chain starting `off` bytes from the beginning,
/// continuing for `p.len()` bytes, into the indicated buffer.
pub fn m_copydata(m: &Mbuf, off: i32, p: &mut [u8]) {
    if off < 0 {
        kpanic(format_args!("m_copydata: off {off} < 0"));
    }
    let Some((m, off)) = m_getptr(m, off) else {
        kpanic(format_args!("m_copydata: short mbuf chain"));
    };
    let mut m = Some(m);
    let mut off = as_idx(off);
    let mut cp = 0usize;
    while cp < p.len() {
        let Some(mm) = m else {
            kpanic(format_args!("m_copydata: null mbuf"));
        };
        let d = mm.data();
        let count = min(d.len() - off, p.len() - cp);
        p[cp..cp + count].copy_from_slice(&d[off..off + count]);
        cp += count;
        off = 0;
        m = mm.m_next.get();
    }
}

/// The storage of `m`, writable: a shared cluster replaced by a copy of its own first (see
/// the module's deviations); `None` when no cluster can be had for the copy.
fn storage_writable(m: &mut Mbuf, wait: i32) -> Option<&mut [u8]> {
    if m.is_readonly() {
        let i = m.body.e().map(|e| e.cluster().index())?;
        let mut cl = ClusterBox::get(i, pr_flags(wait))?;
        if let Some(e) = m.body.e() {
            cl.bytes_mut().copy_from_slice(e.bytes());
        }
        drop(m.body.set_e(Ext::new(ExtKind::Pool(cl), MCL_ITEM[i])));
        m.set_bits(M_EXTWR);
    }
    m.storage_mut()
}

/// `m_copyback`: copy data from a buffer back into the indicated mbuf chain, starting `off`
/// bytes from the beginning, extending the mbuf chain if necessary. The mbuf needs to be
/// properly initialized including the setting of `m_len`.
pub fn m_copyback(m0: &mut Mbuf, off: i32, data: &[u8], wait: i32) -> Result<(), Errno> {
    let mut off = off;
    let mut len = as_int(data.len());
    let mut cp = 0usize;
    let mut totlen = 0i32;
    let hdr = m0.pkthdr().is_some();
    let mut error = Ok(());

    {
        let mut m: &mut Mbuf = &mut *m0;
        'out: {
            loop {
                let mlen = m.m_len.get() as i32;
                if off <= mlen {
                    break;
                }
                off -= mlen;
                totlen += mlen;
                if !m.m_next.is_some() {
                    let Some(mut n) = m_get(wait, i32::from(m.m_type().get())) else {
                        error = Err(Errno::ENOBUFS);
                        break 'out;
                    };

                    if off + len > MLEN as i32 && !mclgetl(&mut n, wait, (off + len) as u32) {
                        error = Err(Errno::ENOBUFS);
                        break 'out;
                    }
                    if let Some(s) = n.storage_mut() {
                        s[..as_idx(off)].fill(0);
                    }
                    n.m_len.set((len + off) as u32);
                    drop(m.m_next.set(n));
                }
                let Some(next) = m.m_next.get_mut() else {
                    break 'out;
                };
                m = next;
            }
            while len > 0 {
                // extend last packet to be filled fully
                if !m.m_next.is_some() && len > m.m_len.get() as i32 - off {
                    let grow = min(
                        (len - (m.m_len.get() as i32 - off)) as u32,
                        m_trailingspace(m) as u32,
                    );
                    m.m_len.set(m.m_len.get() + grow);
                }
                let mlen = min(m.m_len.get() as i32 - off, len);
                let at = m.data_off() + as_idx(off);
                let Some(s) = storage_writable(m, wait) else {
                    error = Err(Errno::ENOBUFS);
                    break 'out;
                };
                s[at..at + as_idx(mlen)].copy_from_slice(&data[cp..cp + as_idx(mlen)]);
                cp += as_idx(mlen);
                len -= mlen;
                totlen += mlen + off;
                if len == 0 {
                    break;
                }
                off = 0;

                if !m.m_next.is_some() {
                    let Some(mut n) = m_get(wait, i32::from(m.m_type().get())) else {
                        error = Err(Errno::ENOBUFS);
                        break 'out;
                    };

                    if len > MLEN as i32 && !mclgetl(&mut n, wait, len as u32) {
                        error = Err(Errno::ENOBUFS);
                        break 'out;
                    }
                    n.m_len.set(len as u32);
                    drop(m.m_next.set(n));
                }
                let Some(next) = m.m_next.get_mut() else {
                    break 'out;
                };
                m = next;
            }
        }
    }
    // out:
    if hdr && m0.m_pkthdr().len.get() < totlen {
        m0.m_pkthdr().len.set(totlen);
    }

    error
}

/// `m_cat`: concatenate mbuf chain `n` to `m`. `n` might be copied into `m` (when `n->m_len`
/// is small), therefore data portion of `n` could be copied into an mbuf of different mbuf
/// type. Therefore both chains should be of the same type (e.g. `MT_DATA`). Any `m_pkthdr`
/// is not updated.
pub fn m_cat(m: &mut Mbuf, n: Option<MbufBox>) {
    let mut m = m;
    while m.m_next.is_some() {
        m = match m.m_next.get_mut() {
            Some(next) => next,
            None => kpanic(format_args!("m_cat: a link emptied under the walk")),
        };
    }
    let mut n = n;
    while let Some(nn) = n {
        let nlen = nn.m_len.get();
        if m.is_readonly() || nlen as i32 > m_trailingspace(m) {
            // just join the two chains
            drop(m.m_next.set(nn));
            return;
        }
        // splat the data from one into the other
        let at = m.data_off() + m.m_len.get() as usize;
        match m
            .storage_mut()
            .and_then(|s| s.get_mut(at..at + nlen as usize))
        {
            Some(dst) => dst.copy_from_slice(nn.data()),
            None => kpanic(format_args!("m_cat: no trailing space")),
        }
        m.m_len.set(m.m_len.get() + nlen);
        n = m_free(nn);
    }
}

/// `m_adj`: trims `req_len` bytes from the head of the chain, or `-req_len` bytes from the
/// tail when negative. Only lengths and offsets change.
pub fn m_adj(mp: &Mbuf, req_len: i32) {
    let mut len = req_len;
    if len >= 0 {
        // Trim from head.
        // The offset moves within the data the length covers: no bound to check.
        let mut m = Some(mp);
        while let Some(mm) = m
            && len > 0
        {
            let mlen = mm.m_len.get();
            if mlen as i32 <= len {
                len -= mlen as i32;
                mm.m_off.set(mm.m_off.get() + mlen);
                mm.m_len.set(0);
                m = mm.m_next.get();
            } else {
                mm.m_off.set(mm.m_off.get() + len as u32);
                mm.m_len.set(mlen - len as u32);
                len = 0;
            }
        }
        if let Some(h) = mp.pkthdr() {
            h.len.set(h.len.get() - (req_len - len));
        }
    } else {
        // Trim from tail. Scan the mbuf chain, calculating its length and finding the last
        // mbuf. If the adjustment only affects this mbuf, then just adjust and return.
        // Otherwise, rescan and truncate after the remaining size.
        len = -len;
        let mut count = 0i32;
        let mut m: &Mbuf = mp;
        loop {
            count += m.m_len.get() as i32;
            match m.m_next.get() {
                None => break,
                Some(next) => m = next,
            }
        }
        if m.m_len.get() as i32 >= len {
            m.m_len.set(m.m_len.get() - len as u32);
            if let Some(h) = mp.pkthdr() {
                h.len.set(h.len.get() - len);
            }
            return;
        }
        count -= len;
        if count < 0 {
            count = 0;
        }
        // Correct length for chain is "count". Find the mbuf with last data, adjust its
        // length, and toss data from remaining mbufs on chain.
        if let Some(h) = mp.pkthdr() {
            h.len.set(count);
        }
        let mut m: &Mbuf = mp;
        loop {
            if m.m_len.get() as i32 >= count {
                m.m_len.set(count as u32);
                break;
            }
            count -= m.m_len.get() as i32;
            // the chain holds at least `count` bytes, so a next mbuf exists
            match m.m_next.get() {
                None => break,
                Some(next) => m = next,
            }
        }
        let mut next = m.m_next.get();
        while let Some(nn) = next {
            nn.m_len.set(0);
            next = nn.m_next.get();
        }
    }
}

/// `m_pullup`: rearrange an mbuf chain so that `len` bytes are contiguous and in the data area
/// of an mbuf (so that `mtod` will work for a structure of size `len`). Returns the resulting
/// mbuf chain on success, frees it and returns `None` on failure.
pub fn m_pullup(m0: MbufBox, len: i32) -> Option<MbufBox> {
    let mut m0 = m0;
    let mut len = len;

    // if len is already contig in m0, then don't do any work
    if len <= m0.m_len.get() as i32 {
        return Some(m0);
    }

    // look for some data; freem0 when there is none
    let mut m = m0.m_next.take()?;

    // The data's alignment within a long, kept where it moves (M_DATABUF is aligned to 8).
    let adj = if m0.m_len.get() == 0 {
        while m.m_len.get() == 0 {
            // freem0 when the chain holds no data
            m = m_free(m)?;
        }
        m.data_raw().addr() & (size_of::<usize>() - 1)
    } else {
        m0.data_raw().addr() & (size_of::<usize>() - 1)
    };

    // head = M_DATABUF(m0) + adj, tail = M_DATABUF(m0) + M_SIZE(m0), as offsets
    let tail = m0.storage_len();
    let head = adj;

    if !m0.is_readonly() && len as usize <= tail - head {
        // we can copy everything into the first mbuf
        if m0.m_len.get() == 0 {
            m0.m_off.set(head as u32);
        } else if len as usize > tail - m0.data_off() {
            // need to memmove to make space at the end
            let (from, n) = (m0.data_off(), m0.m_len.get() as usize);
            if let Some(s) = m0.storage_mut() {
                s.copy_within(from..from + n, head);
            }
            m0.m_off.set(head as u32);
        }
        len -= m0.m_len.get() as i32;
        mbstat_inc(MbstatCounters::MbsPullupCopy as usize);
    } else {
        // the first mbuf is too small or read-only, make a new one
        let space = adj + len as usize;

        if space > MAXMCLBYTES {
            // bad: the chain and m0 go
            drop(m);
            return None;
        }

        drop(m0.m_next.set(m));
        let mut old = m0;

        mbstat_inc(MbstatCounters::MbsPullupAlloc as usize);
        // bad: the chain goes with `old`
        let mut n0 = m_get(M_DONTWAIT, i32::from(old.m_type().get()))?;

        if space > MHLEN && !mclgetl(&mut n0, M_DONTWAIT, space as u32) {
            return None;
        }

        if old.pkthdr().is_some() {
            m_move_pkthdr(&mut n0, &mut old);
        }

        n0.m_len.set(0);
        n0.data_add(adj as isize);
        m = old;
        m0 = n0;
    }

    kdassert!(m_trailingspace(&m0) >= len);

    loop {
        let space = min(len as u32, m.m_len.get());
        let at = m0.data_off() + m0.m_len.get() as usize;
        match m0
            .storage_mut()
            .and_then(|s| s.get_mut(at..at + space as usize))
        {
            Some(dst) => dst.copy_from_slice(&m.data()[..space as usize]),
            None => kpanic(format_args!("m_pullup: no trailing space")),
        }
        len -= space as i32;
        m0.m_len.set(m0.m_len.get() + space);
        m.m_len.set(m.m_len.get() - space);

        let next = if m.m_len.get() > 0 {
            // within the data the length covered
            m.m_off.set(m.m_off.get() + space);
            Some(m)
        } else {
            m_free(m)
        };

        if len == 0 {
            // link the chain back up
            if let Some(next) = next {
                drop(m0.m_next.set(next));
            }
            return Some(m0);
        }

        // bad: m0 goes, the chain went
        m = next?;
    }
}

/// `m_getptr`: the mbuf and offset of location `loc` in the chain; the end of the valid data
/// when `loc` is the chain's length.
pub fn m_getptr(m: &Mbuf, loc: i32) -> Option<(&Mbuf, i32)> {
    let mut m = m;
    let mut loc = loc;
    while loc >= 0 {
        // Normal end of search
        if m.m_len.get() as i32 > loc {
            return Some((m, loc));
        }
        loc -= m.m_len.get() as i32;

        match m.m_next.get() {
            None => {
                if loc == 0 {
                    // Point at the end of valid data
                    return Some((m, m.m_len.get() as i32));
                }
                return None;
            }
            Some(next) => m = next,
        }
    }

    None
}

/// [`m_getptr`], the mbuf lent mutably.
pub fn m_getptr_mut(m: &mut Mbuf, loc: i32) -> Option<(&mut Mbuf, i32)> {
    let mut m = m;
    let mut loc = loc;
    while loc >= 0 {
        let mlen = m.m_len.get() as i32;
        // Normal end of search
        if mlen > loc {
            return Some((m, loc));
        }
        loc -= mlen;

        if !m.m_next.is_some() {
            // Point at the end of valid data
            return (loc == 0).then_some((m, mlen));
        }
        m = m.m_next.get_mut()?;
    }

    None
}

/// `m_split`: partition an mbuf chain in two pieces, returning the tail -- all but the first
/// `len0` bytes. In case of failure, it returns `None` and attempts to restore the chain to
/// its original state.
pub fn m_split(m0: &mut Mbuf, len0: i32, wait: i32) -> Option<MbufBox> {
    // The header copy is made before the walk lends the chain mutably (the C makes it once
    // the walk found the mbuf: a chain too short gives it back here).
    let mut hdr = None;
    let mut olen = None;
    if let Some(h) = m0.pkthdr() {
        let plen = h.len.get();
        let mut n = m_gethdr(wait, i32::from(m0.m_type().get()))?;
        m_dup_pkthdr(&mut n, m0, wait).ok()?;
        n.m_pkthdr().len.set(n.m_pkthdr().len.get() - len0);
        olen = Some(plen);
        m0.m_pkthdr().len.set(len0);
        hdr = Some(n);
    }
    let n = split_at(m0, len0 as u32, hdr, wait);
    if n.is_none()
        && let Some(olen) = olen
    {
        m0.m_pkthdr().len.set(olen);
    }
    n
}

/// `m_split` after the header copy (`hdr`, for a packet): the walk to the mbuf where the split
/// falls and the cut.
fn split_at(m0: &mut Mbuf, len: u32, hdr: Option<MbufBox>, wait: i32) -> Option<MbufBox> {
    let mut len = len;
    let mut m: &mut Mbuf = m0;
    while len > m.m_len.get() {
        len -= m.m_len.get();
        m = m.m_next.get_mut()?;
    }
    let remain = m.m_len.get() - len;
    let mut n = if let Some(mut n) = hdr {
        if remain == 0 {
            if let Some(rest) = m.m_next.take() {
                drop(n.m_next.set(rest));
            }
            n.m_len.set(0);
            return Some(n);
        }
        if m.ext().is_none() && remain as usize > MHLEN {
            // m can't be the lead packet
            m_align(&n, 0);
            // the failure frees n
            let rest = m_split(m, len as i32, wait)?;
            drop(n.m_next.set(rest));
            n.m_len.set(0);
            return Some(n);
        }
        n
    } else if remain == 0 {
        return m.m_next.take();
    } else {
        m_get(wait, i32::from(m.m_type().get()))?
    };
    if m.ext().is_some() {
        // the failure frees n
        m_extref(m, &mut n, wait).ok()?;
        n.m_off.set(m.m_off.get() + len);
    } else {
        m_align(&n, remain as i32);
        let at = n.data_off();
        let src = &m.data()[len as usize..(len + remain) as usize];
        match n
            .storage_mut()
            .and_then(|s| s.get_mut(at..at + remain as usize))
        {
            Some(dst) => dst.copy_from_slice(src),
            None => kpanic(format_args!("m_split: no room for {remain} bytes")),
        }
    }
    n.m_len.set(remain);
    m.m_len.set(len);
    if let Some(rest) = m.m_next.take() {
        drop(n.m_next.set(rest));
    }
    Some(n)
}

/// `m_makespace`: make space for a new header of length `hlen` at `skip` bytes into the
/// packet. When doing this we allocate new mbufs only when absolutely necessary. The mbuf
/// where the new header is to go is returned together with an offset into the mbuf. If `None`
/// is returned then the mbuf chain may have been modified; the caller is assumed to always
/// free the chain. The packet length is adjusted first (the C adjusts it last, on success):
/// the walk then lends the chain mutably to the end, and a chain `None` leaves is freed.
pub fn m_makespace(m0: &mut Mbuf, skip: i32, hlen: i32) -> Option<(&mut Mbuf, i32)> {
    kassert!(m0.pkthdr().is_some());
    // Limit the size of the new header to MHLEN. In case skip = 0 and the first buffer is not
    // a cluster this is the maximum space available in that mbuf. In other words this code
    // never prepends a mbuf.
    kassert!(hlen < MHLEN as i32);

    // adjust packet length
    let plen = m0.m_pkthdr().len.get();
    m0.m_pkthdr().len.set(plen + hlen);

    let mut skip = skip;
    let mut m: &mut Mbuf = m0;
    while skip > m.m_len.get() as i32 {
        skip -= m.m_len.get() as i32;
        m = m.m_next.get_mut()?;
    }
    // At this point skip is the offset into the mbuf m where the new header should be placed.
    // Figure out if there's space to insert the new header. If so, and copying the remainder
    // makes sense then do so. Otherwise insert a new mbuf in the chain, splitting the contents
    // of m as needed.
    let remain = m.m_len.get() - skip as u32; // data to move
    if (skip as u32) < remain && hlen <= m_leadingspace(m) {
        let at = m.data_off();
        if skip != 0
            && let Some(s) = m.storage_mut()
        {
            s.copy_within(at..at + skip as usize, at - hlen as usize);
        }
        m.data_add(-(hlen as isize));
        m.m_len.set(m.m_len.get() + hlen as u32);
        Some((m, skip))
    } else if hlen > m_trailingspace(m) {
        if remain > 0 {
            let mut n = m_get(M_DONTWAIT, i32::from(m.m_type().get()))?;
            if remain as usize > MLEN && !mclgetl(&mut n, M_DONTWAIT, remain) {
                return None;
            }

            let from = skip as usize;
            n.m_len.set(remain);
            match n.data_mut() {
                Some(dst) => dst.copy_from_slice(&m.data()[from..from + remain as usize]),
                None => kpanic(format_args!("m_makespace: a fresh mbuf read-only")),
            }
            m.m_len.set(m.m_len.get() - remain);

            if let Some(rest) = m.m_next.take() {
                drop(n.m_next.set(rest));
            }
            drop(m.m_next.set(n));
        }

        if hlen <= m_trailingspace(m) {
            m.m_len.set(m.m_len.get() + hlen as u32);
            Some((m, skip))
        } else {
            let mut n = m_get(M_DONTWAIT, i32::from(m.m_type().get()))?;

            n.m_len.set(hlen as u32);

            if let Some(rest) = m.m_next.take() {
                drop(n.m_next.set(rest));
            }
            drop(m.m_next.set(n));

            // header is at front ... of new mbuf
            let n = m.m_next.get_mut()?;
            Some((n, 0))
        }
    } else {
        // Copy the remainder to the back of the mbuf so there's space to write the new header.
        let at = m.data_off() + skip as usize;
        if remain > 0
            && let Some(s) = m.storage_mut()
        {
            s.copy_within(at..at + remain as usize, at + hlen as usize);
        }
        m.m_len.set(m.m_len.get() + hlen as u32);
        Some((m, skip))
    }
}

/// `m_devget`: routine to copy from device local memory into mbufs; the packet is `buf`.
pub fn m_devget(buf: &[u8], off: i32) -> Option<MbufBox> {
    let mut totlen = as_int(buf.len());
    let mut off = off;
    let mut top = MChainBuilder::new();
    let mut cp = 0usize;

    if off < 0 || off > MHLEN as i32 {
        return None;
    }

    let hdr = m_gethdr(M_DONTWAIT, MT_DATA)?;

    hdr.m_pkthdr().len.set(totlen);

    let mut len = MHLEN as i32;

    // A zero `totlen` returns NULL (the C keeps the header mbuf; it goes back here).
    let mut next = Some(hdr);
    while totlen > 0 {
        let first = top.chain.is_empty();
        let mut m = if let Some(m) = next.take() {
            m
        } else {
            let Some(n) = m_get(M_DONTWAIT, MT_DATA) else {
                // As we might get called by pfkey, make sure we do not leak sensitive data.
                if let Some(t) = top.chain.first() {
                    t.set_bits(M_ZEROIZE);
                }
                return None;
            };
            len = MLEN as i32;
            n
        };

        if totlen + off >= MINCLSIZE as i32 {
            mclget(&mut m, M_DONTWAIT);
            if m.ext().is_some() {
                len = MCLBYTES as i32;
            }
        } else {
            // Place initial small packet/header at end of mbuf.
            let max_linkhdr = MAX_LINKHDR.load(Ordering::Relaxed);
            if first && totlen + off + max_linkhdr <= len {
                m.data_add(max_linkhdr as isize);
                len -= max_linkhdr;
            }
        }

        if off != 0 {
            m.data_add(off as isize);
            len -= off;
            off = 0;
        }

        len = min(totlen, len);
        m.m_len.set(len as u32);
        match m.data_mut() {
            Some(dst) => dst.copy_from_slice(&buf[cp..cp + as_idx(len)]),
            None => kpanic(format_args!("m_devget: a fresh mbuf read-only")),
        }

        cp += as_idx(len);
        top.push(m);
        totlen -= len;
    }
    top.into_head()
}

/// `m_zero`: zeroes the mbuf's storage (`M_DATABUF`, `M_SIZE` bytes: a cluster of its own
/// whether `M_EXTWR` is set or not, as the C), or marks a shared cluster to be zeroed when
/// its last reference goes.
pub fn m_zero(m: &mut Mbuf) {
    if let Some(Ext {
        kind: ExtKind::Shared(r),
        ..
    }) = m.body.e()
    {
        // this variable only transitions in one direction, so if there is a race it will be
        // toward the same result and therefore there is no loss.
        r.zero.store(true, Ordering::Relaxed);
        return;
    }

    if m.body.e().is_none() {
        if let Some(s) = m.body.inline_mut() {
            explicit_bzero(s);
        }
        return;
    }
    if let Some(b) = m.body.e_mut().and_then(Ext::bytes_mut) {
        explicit_bzero(b);
    }
}

/// `m_apply`: apply function `f` to the data in an mbuf chain starting `off` bytes from the
/// beginning, continuing for `len` bytes; the first error stops the walk.
pub fn m_apply(
    m: &Mbuf,
    off: i32,
    len: i32,
    mut f: impl FnMut(&[u8]) -> Result<(), Errno>,
) -> Result<(), Errno> {
    if len < 0 {
        kpanic(format_args!("m_apply: len {len} < 0"));
    }
    if off < 0 {
        kpanic(format_args!("m_apply: off {off} < 0"));
    }
    let mut m = Some(m);
    let mut off = off;
    let mut len = len;
    while off > 0 {
        let Some(mm) = m else {
            kpanic(format_args!("m_apply: null mbuf in skip"));
        };
        if off < mm.m_len.get() as i32 {
            break;
        }
        off -= mm.m_len.get() as i32;
        m = mm.m_next.get();
    }
    while len > 0 {
        let Some(mm) = m else {
            kpanic(format_args!("m_apply: null mbuf"));
        };
        let count = min(mm.m_len.get() - off as u32, len as u32) as usize;

        f(&mm.data()[as_idx(off)..as_idx(off) + count])?;

        len -= count as i32;
        off = 0;
        m = mm.m_next.get();
    }

    Ok(())
}

/// `m_leadingspace`: compute the amount of space available before the current start of data
/// in an mbuf. Read-only clusters never have space available.
pub fn m_leadingspace(m: &Mbuf) -> i32 {
    if m.is_readonly() {
        return 0;
    }
    as_int(m.data_off())
}

/// `m_trailingspace`: compute the amount of space available after the end of data in an mbuf.
/// Read-only clusters never have space available.
pub fn m_trailingspace(m: &Mbuf) -> i32 {
    if m.is_readonly() {
        return 0;
    }
    let end = m.data_off() + m.m_len.get() as usize;
    kassert!(m.storage_len() >= end);
    as_int(m.storage_len().saturating_sub(end))
}

/// `m_align`: set the `m_data` pointer of a newly-allocated mbuf to place an object of the
/// specified size at the end of the mbuf, longword aligned.
pub fn m_align(m: &Mbuf, len: i32) {
    const LONG: usize = size_of::<usize>();
    kassert!(len >= 0 && !m.is_readonly());
    kassert!(m.data_off() == 0); // newly-allocated check
    kassert!((as_idx(len) + LONG - 1) & !(LONG - 1) <= m.storage_len());

    m.m_off
        .set(((m.storage_len() - as_idx(len)) & !(LONG - 1)) as u32);
}

/// `m_dup_pkthdr`: duplicate mbuf pkthdr from `from` to `to`. `from` must have `M_PKTHDR`
/// set, and `to` must be empty.
pub fn m_dup_pkthdr(to: &mut Mbuf, from: &Mbuf, wait: i32) -> Result<(), Errno> {
    let Some(fh) = from.pkthdr() else {
        kpanic(format_args!("m_dup_pkthdr: from without M_PKTHDR"));
    };

    to.body
        .set_flags((to.flags() & (M_EXT | M_EXTWR)) | (from.flags() & M_COPYFLAGS));
    // to->m_pkthdr = from->m_pkthdr, pf's two words 0 and the tags none (SLIST_INIT), a header
    // `to` had dropped.
    drop(to.body.set_p(Pkthdr::copy_of(fh)));

    // NPF > 0: pf's references, counted again for the copy.
    if let (Some(pf), Some(th)) = (pf_hooks::pf(), to.body.p()) {
        pf.mbuf_link_state_key(&th.pf, &fh.pf);
        pf.mbuf_link_inpcb(&th.pf, &fh.pf);
    }

    if to.body.e().is_none() {
        to.m_off.set(0);
    }

    m_tag_copy_chain(to, from, wait)
}

/// `M_MOVE_HDR(to, from)`: move just `m_pkthdr` from `from` to `to`, remove `M_PKTHDR` and
/// clean flags/tags for `from`: the header moves whole (its tags and pf's state key with it,
/// nothing dropped), `from`'s data stays where it is.
pub fn m_move_hdr(to: &mut Mbuf, from: &mut Mbuf) {
    let at = from.data_off();
    if let Some(h) = from.body.take_p() {
        if from.body.e().is_none() {
            from.m_off
                .set((at + Body::<Pkthdr, Ext, MbufMeta, M_PKTHDR, M_EXT, MLEN>::INLINE_P) as u32);
        }
        drop(to.body.set_p(h));
    }
}

/// `M_MOVE_PKTHDR(to, from)`: move mbuf pkthdr from `from` to `to`. `from` must have
/// `M_PKTHDR` set, and `to` must be empty.
pub fn m_move_pkthdr(to: &mut Mbuf, from: &mut Mbuf) {
    to.body
        .set_flags((to.flags() & (M_EXT | M_EXTWR)) | (from.flags() & M_COPYFLAGS));
    m_move_hdr(to, from);
    if to.body.e().is_none() {
        to.m_off.set(0);
    }
}

/// `m_dup_pkt`: a copy of packet `m0` in one mbuf (or cluster), its data starting `adj` bytes
/// in.
pub fn m_dup_pkt(m0: &Mbuf, adj: u32, wait: i32) -> Option<MbufBox> {
    let Some(h0) = m0.pkthdr() else {
        kpanic(format_args!("m_dup_pkt: an mbuf without M_PKTHDR"));
    };

    let len = h0.len.get() + adj as i32;
    if len > MAXMCLBYTES as i32 {
        // XXX
        return None;
    }

    let mut m = m_get(wait, i32::from(m0.m_type().get()))?;

    // fail: m_freem(m), the copy's drop
    m_dup_pkthdr(&mut m, m0, wait).ok()?;

    if len > MHLEN as i32 && !mclgetl(&mut m, wait, len as u32) {
        return None;
    }

    m.m_len.set(len as u32);
    m.m_pkthdr().len.set(len);
    m_adj(&m, adj as i32);
    match m.data_mut() {
        Some(dst) => m_copydata(m0, 0, dst),
        None => kpanic(format_args!("m_dup_pkt: a fresh mbuf read-only")),
    }

    Some(m)
}

/// `m_microtime`: the packet's timestamp, or the current time when it has none.
pub fn m_microtime(m: &Mbuf) -> Timeval {
    let h = m.m_pkthdr();
    if h.csum_flags.get() & M_TIMESTAMP != 0 {
        // NSEC_TO_TIMEVAL(m->m_pkthdr.ph_timestamp, &utv)
        let ns = h.ph_timestamp.get();
        let utv = Timeval::new(ns / 1_000_000_000, (ns % 1_000_000_000) / 1000);
        crate::tc::microboottime().add(utv)
    } else {
        crate::tc::microtime()
    }
}

impl MbufPages {
    /// The pools' physical constraint: one contiguous segment, DMA-reachable until
    /// `m_pool_noconstraints` (`kp_dma_contig`, then `kp_mbuf_contig`).
    fn constraint() -> KmemPaMode {
        if NOCONSTRAINTS.load(Ordering::Relaxed) {
            KP_MBUF_CONTIG
        } else {
            KmemPaMode {
                kp_maxseg: 1,
                ..pa_mode(crate::vm::init::machine().dma_constraint)
            }
        }
    }

    /// `pool_multi_alloc`'s virtual mode: `kmem_map`, aligned to the page when the pool keeps
    /// its page header inside.
    fn va_mode(len: usize, aligned: bool) -> KmemVaMode {
        KmemVaMode {
            kv_align: if aligned { len } else { 0 },
            ..KV_INTRSAFE
        }
    }
}

impl PageSource for MbufPages {
    /// `m_pool_alloc`: a page from `pool_allocator_multi`, charged against the mbuf memory
    /// limit; `mbs_drops` counts a refusal.
    fn alloc(&self, len: usize, aligned: bool, wait: bool) -> Option<NonNull<u8>> {
        let pgsize = len as u64;

        if MBUF_MEM_ALLOC.fetch_add(pgsize, Ordering::Relaxed) + pgsize
            <= MBUF_MEM_LIMIT.load(Ordering::Relaxed)
        {
            let kd = KmemDynMode {
                kd_waitok: wait,
                ..KMEM_DYN_INITIALIZER
            };
            let v = {
                let _s = splraise(IPL_VM);
                km_alloc(len, &Self::va_mode(len, aligned), &Self::constraint(), &kd)
            };
            if let Some(v) = v.and_then(|v| {
                NonNull::new(core::ptr::with_exposed_provenance_mut::<u8>(v.as_usize()))
            }) {
                return Some(v);
            }
        }

        // fail:
        mbstat_inc(MbstatCounters::MbsDrops as usize);
        MBUF_MEM_ALLOC.fetch_sub(pgsize, Ordering::Relaxed);
        None
    }

    /// `m_pool_free`: a page back to `pool_allocator_multi`, uncharged.
    fn free(&self, page: NonNull<u8>, len: usize, aligned: bool) {
        {
            let _s = splraise(IPL_VM);
            km_free(
                Vaddr::new(page.as_ptr().expose_provenance()),
                len,
                &Self::va_mode(len, aligned),
                &Self::constraint(),
            );
        }

        MBUF_MEM_ALLOC.fetch_sub(len as u64, Ordering::Relaxed);
    }

    /// `pool_allocator_multi.pa_pagesz`: every power of two from a page to 2 GiB, alignable
    /// (`POOL_ALLOC_SIZES(PAGE_SIZE, 1UL << 31, POOL_ALLOC_ALIGNED)`).
    fn sizes(&self) -> usize {
        const MAX: usize = 1 << 31;
        ((MAX | (MAX - 1)) & !(PAGE_SIZE - 1)) | 1
    }
}

/// `m_pool_init`: an mbuf or cluster pool at `IPL_NET`, its pages from [`MbufPages`] (the
/// memory limit, DMA-reachable until `m_pool_noconstraints`). `false` when the pool cannot be
/// made (or was already).
fn m_pool_init<T: 'static>(pp: &'static BoxPool<T>, wmesg: &'static str) -> bool {
    pp.init_with_source(IPL_NET, 0, wmesg, &MBUF_PAGES).is_ok()
}

/// `m_pool_noconstraints`: lets the mbuf and cluster pools use all of memory (their pages'
/// constraint, [`MbufPages`]'s, for every pool at once).
pub fn m_pool_noconstraints() {
    NOCONSTRAINTS.store(true, Ordering::Relaxed);
}

/// The clusters out of the eight cluster pools (`mclpools[i].pr_nout` summed: the boot
/// self-test's count).
pub fn mcl_nout() -> u64 {
    MCL2K.nout()
        + MCL2K2.nout()
        + MCL4K.nout()
        + MCL8K.nout()
        + MCL9K128.nout()
        + MCL12K.nout()
        + MCL16K.nout()
        + MCL64K.nout()
}

/// `m_pool_used`: the mbuf memory in use, in percent of the limit.
pub fn m_pool_used() -> u32 {
    let limit = MBUF_MEM_LIMIT.load(Ordering::Relaxed).max(1);
    ((MBUF_MEM_ALLOC.load(Ordering::Relaxed) * 100) / limit) as u32
}

/// `%b`: a value followed by the set bits' names, from a descriptor string of the form
/// `"\x10\x01FLAG1\x02FLAG2..."` (subr_prf.c's `kprintf`; `bsd`'s `Bitmask` until the printf
/// family moves).
struct Bits<'a>(u64, &'a [u8]);

impl fmt::Display for Bits<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((&base, mut b)) = self.1.split_first() else {
            return Ok(());
        };
        let v = self.0;
        match base {
            8 => write!(f, "{v:o}")?,
            10 => write!(f, "{v}")?,
            16 => write!(f, "{v:x}")?,
            _ => return Ok(()),
        }
        if v == 0 {
            return Ok(());
        }
        let mut any = false;
        while let Some((&bit, rest)) = b.split_first() {
            b = rest;
            if bit == 0 {
                break;
            }
            let n = if bit & 0x80 != 0 {
                bit & 0x7f
            } else if bit <= b' ' {
                bit - 1
            } else {
                bit
            };
            let name_len = b.iter().take_while(|&&c| c > b' ' && c & 0x80 == 0).count();
            let (name, rest) = b.split_at(name_len);
            b = rest;
            if n < 64 && v & (1u64 << n) != 0 {
                f.write_str(if any { "," } else { "<" })?;
                for &c in name {
                    write!(f, "{}", char::from(c))?;
                }
                any = true;
            }
        }
        if any {
            f.write_str(">")?;
        }
        Ok(())
    }
}

/// The address of an optional mbuf, for `%p`.
fn mptr(m: Option<&Mbuf>) -> *const Mbuf {
    m.map_or(core::ptr::null(), core::ptr::from_ref)
}

/// `m_print` (`DDB`): prints one mbuf.
pub fn m_print(m: &Mbuf, pr: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(pr, "mbuf {m:p}")?;
    writeln!(
        pr,
        "m_type: {}\tm_flags: {}",
        m.m_type().get(),
        Bits(u64::from(m.flags()), M_BITS)
    )?;
    writeln!(
        pr,
        "m_next: {:p}\tm_nextpkt: {:p}",
        mptr(m.m_next.get()),
        mptr(m.m_nextpkt.get())
    )?;
    writeln!(pr, "m_data: {:p}\tm_len: {}", m.data_raw(), m.m_len.get())?;
    writeln!(
        pr,
        "m_dat: {:p}\tm_pktdat: {:p}",
        m.body.as_raw(),
        m.body.as_raw().wrapping_add(size_of::<Pkthdr>())
    )?;
    if let Some(ph) = m.pkthdr() {
        writeln!(
            pr,
            "m_ptkhdr.ph_ifidx: {}\tm_pkthdr.len: {}",
            ph.ph_ifidx.get(),
            ph.len.get()
        )?;
        writeln!(
            pr,
            "m_ptkhdr.ph_tags: {:p}\tm_pkthdr.ph_tagsset: {}",
            ph.ph_tags
                .get()
                .map_or(core::ptr::null(), core::ptr::from_ref),
            Bits(u64::from(ph.ph_tagsset.get()), MTAG_BITS)
        )?;
        writeln!(
            pr,
            "m_pkthdr.ph_flowid: {}\tm_pkthdr.ph_loopcnt: {}",
            ph.ph_flowid.get(),
            ph.ph_loopcnt.get()
        )?;
        writeln!(
            pr,
            "m_pkthdr.csum_flags: {}",
            Bits(u64::from(ph.csum_flags.get()), MCS_BITS)
        )?;
        writeln!(
            pr,
            "m_pkthdr.ether_vtag: {}\tm_ptkhdr.ph_rtableid: {}",
            ph.ether_vtag.get(),
            ph.ph_rtableid.get()
        )?;
        writeln!(
            pr,
            "m_pkthdr.pf.statekey: {:p}\tm_pkthdr.pf.inp {:p}",
            core::ptr::without_provenance::<u8>(ph.pf.statekey.get()),
            core::ptr::without_provenance::<u8>(ph.pf.inp.get())
        )?;
        writeln!(
            pr,
            "m_pkthdr.pf.qid: {}\tm_pkthdr.pf.tag: {}",
            ph.pf.qid.get(),
            ph.pf.tag.get()
        )?;
        writeln!(
            pr,
            "m_pkthdr.pf.flags: {}",
            Bits(u64::from(ph.pf.flags.get()), MPF_BITS)
        )?;
        writeln!(
            pr,
            "m_pkthdr.pf.routed: {}\tm_pkthdr.pf.prio: {}",
            ph.pf.routed.get(),
            ph.pf.prio.get()
        )?;
    }
    if let Some(e) = m.ext() {
        writeln!(
            pr,
            "m_ext.ext_buf: {:p}\tm_ext.ext_size: {}",
            e.ext_buf(),
            e.size
        )?;
        // ext_free_fn: the C's index (MEXTFREE_POOL 0 or m_extfree_refs_fn 1); ext_arg the
        // pool or the shared count's address.
        let (fnidx, arg) = match &e.kind {
            ExtKind::Pool(_) => (0, core::ptr::null::<u8>()),
            ExtKind::Shared(r) => (1, PoolArc::as_ptr(r).cast::<u8>()),
        };
        writeln!(pr, "m_ext.ext_free_fn: {fnidx}\tm_ext.ext_arg: {arg:p}")?;
        // if m_ext.ext_free_fn == m_extfree_refs_fn ?
    }
    Ok(())
}

/// `m_print_chain` (`DDB`): one line per mbuf of a chain and a total.
pub fn m_print_chain(v: Option<&Mbuf>, deep: bool, pr: &mut dyn fmt::Write) -> fmt::Result {
    let mut indent = if deep { "++-" } else { "-+-" };
    let (mut chain, mut len, mut size) = (0usize, 0usize, 0usize);

    let mut m = v;
    while let Some(mm) = m {
        chain += 1;
        len += mm.m_len.get() as usize;
        size += mm.storage_len();
        let t = mm.m_type().get();
        let type_ = usize::try_from(t)
            .ok()
            .and_then(|t| M_TYPES.get(t))
            .copied()
            .unwrap_or("???");
        write!(
            pr,
            "{} mbuf {:p}, {}, off {}, len {}",
            indent,
            mm,
            type_,
            mm.data_off(),
            mm.m_len.get()
        )?;
        if let Some(h) = mm.pkthdr() {
            write!(pr, ", pktlen {}", h.len.get())?;
        }
        match mm.ext() {
            Some(e) => write!(pr, ", clsize {}", e.size)?,
            None => write!(
                pr,
                ", size {}",
                if mm.pkthdr().is_some() { MHLEN } else { MLEN }
            )?,
        }
        writeln!(pr)?;
        indent = if deep { "|+-" } else { " +-" };
        m = mm.m_next.get();
    }
    indent = if deep { "|\\-" } else { " \\-" };
    if v.is_some() {
        writeln!(pr, "{indent} total chain {chain}, len {len}, size {size}")?;
    }
    Ok(())
}

/// `m_print_packet` (`DDB`): one line per packet of a list (or every chain, when `deep`).
pub fn m_print_packet(v: Option<&Mbuf>, deep: bool, pr: &mut dyn fmt::Write) -> fmt::Result {
    let mut indent = "+--";
    let mut pkts = 0usize;

    let mut m = v;
    while let Some(mm) = m {
        let (mut chain, mut len, mut size) = (0usize, 0usize, 0usize);

        pkts += 1;
        if deep {
            m_print_chain(Some(mm), deep, pr)?;
            m = mm.m_nextpkt.get();
            continue;
        }
        let mut n = Some(mm);
        while let Some(nn) = n {
            chain += 1;
            len += nn.m_len.get() as usize;
            size += nn.storage_len();
            n = nn.m_next.get();
        }
        write!(pr, "{indent} mbuf {mm:p}, chain {chain}")?;
        if let Some(h) = mm.pkthdr() {
            write!(pr, ", pktlen {}", h.len.get())?;
        }
        writeln!(pr, ", len {len}, size {size}")?;
        m = mm.m_nextpkt.get();
    }
    indent = "\\--";
    if v.is_some() {
        writeln!(pr, "{indent} total packets {pkts}")?;
    }
    Ok(())
}

// mbuf lists

/// `ml_init`: an empty list (what it held goes).
pub fn ml_init(ml: &mut MbufList) {
    *ml = MbufList::new();
}

/// `ml_enqueue`: appends the packet `m` (its `m_nextpkt` must link nothing).
pub fn ml_enqueue(ml: &mut MbufList, m: MbufBox) {
    ml.push_tail(m);
}

/// `ml_enlist`: appends all of `mlb` to `mla` and empties `mlb`.
pub fn ml_enlist(mla: &mut MbufList, mlb: &mut MbufList) {
    mla.concat(mlb);
}

/// `ml_dequeue`: takes the first packet.
pub fn ml_dequeue(ml: &mut MbufList) -> Option<MbufBox> {
    ml.pop_head()
}

/// `ml_dechain`: takes all packets: the list itself, `ml` left empty (the C returns the first
/// packet, the rest linked through `m_nextpkt`).
pub fn ml_dechain(ml: &mut MbufList) -> MbufList {
    core::mem::take(ml)
}

/// `ml_purge`: frees every packet; returns how many there were.
pub fn ml_purge(ml: &mut MbufList) -> u32 {
    let len = ml_len(ml);
    // one packet at a time, as the C's loop
    *ml = MbufList::new();
    len
}

/// `ml_hdatalen`: the length of the first packet.
pub fn ml_hdatalen(ml: &MbufList) -> u32 {
    let Some(m) = ml.first() else {
        return 0;
    };

    kassert!(m.pkthdr().is_some());
    m.m_pkthdr().len.get() as u32
}

/// `ml_len(ml)`.
#[inline]
pub fn ml_len(ml: &MbufList) -> u32 {
    ml.len() as u32
}

/// `ml_empty(ml)`.
#[inline]
pub fn ml_empty(ml: &MbufList) -> bool {
    ml.is_empty()
}

const _: () = {
    // CTASSERT(MSIZE == sizeof(struct mbuf)), and the C's layout of struct m_hdr (32 bytes),
    // struct pkthdr (80) and MLEN/MHLEN (224/144) on LP64.
    assert!(size_of::<Mbuf>() == MSIZE);
    assert!(align_of::<Mbuf>() == 64);
    assert!(size_of::<Pkthdr>() == 80);
    assert!(size_of::<MbufMeta>() == 6);
    assert!(MLEN == 224 && MHLEN == 144);
    assert!(core::mem::offset_of!(Mbuf, body) == MHDR_SIZE - 8);
    // mclsizes[0] must be MCLBYTES and the last MAXMCLBYTES (mbinit's DIAGNOSTIC checks).
    assert!(MCLSIZES[0] == MCLBYTES as u32);
    assert!(MCLSIZES[MCLSIZES.len() - 1] == MAXMCLBYTES as u32);
    // Every cluster pool's item is its `Bytes` whole: the storage a cluster lends is the
    // `ext_size` `m_trailingspace` and `M_SIZE` count (`MCL_ITEM`).
    assert!(size_of::<Bytes<2048>>() == MCL_ITEM[0] as usize);
    assert!(size_of::<Bytes<2112>>() == MCL_ITEM[1] as usize);
    assert!(size_of::<Bytes<4096>>() == MCL_ITEM[2] as usize);
    assert!(size_of::<Bytes<8192>>() == MCL_ITEM[3] as usize);
    assert!(size_of::<Bytes<9344>>() == MCL_ITEM[4] as usize);
    assert!(size_of::<Bytes<12288>>() == MCL_ITEM[5] as usize);
    assert!(size_of::<Bytes<16384>>() == MCL_ITEM[6] as usize);
    assert!(size_of::<Bytes<65536>>() == MCL_ITEM[7] as usize);
    // The mextrefs item is the C's 64 bytes (the count and the pool beside these 48).
    assert!(size_of::<ExtRefs>() == 48);
    // The mtagpl item: PACKET_TAG_MAXSIZE + sizeof(struct m_tag).
    assert!(size_of::<MTag>() == PACKET_TAG_MAXSIZE + 16);
    // M_COPYALL is an int in C.
    assert!(M_COPYALL > 0);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
pub(crate) mod tests {
    // Host tests of the mbuf allocator, the chain operations, `mbuf_list`/`mbuf_queue` and the
    // packet tags (`mbuf2`), over the pool double and the page system's real memory (the
    // pools' pages are `MbufPages`'s, `km_alloc`'s). Every test holds the page system's lock
    // (`testutil::MEMORY`), so the pools' counts are each test's own.

    use std::string::String;
    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicU32;
    use std::vec::Vec;
    use std::{assert, assert_eq, vec};

    use super::*;
    use crate::net::mbuf2::{
        m_pulldown, m_tag_delete, m_tag_find, m_tag_first, m_tag_get, m_tag_next, m_tag_prepend,
    };
    use crate::testutil;

    /// The page system's memory for the tests: clusters of 64 KiB, one a page of the double.
    const BYTES: usize = 64 << 20;

    /// The tests' `nmbclust`.
    static TEST_NMBCLUST: AtomicI64 = AtomicI64::new(NMBCLUSTERS_TEST);

    /// The host's limit, amd64's `NMBCLUSTERS`.
    const NMBCLUSTERS_TEST: i64 = 256 * 1024;

    /// The doubles, fresh memory for the pools' pages, `mbinit` once and the limit reset.
    pub(crate) fn setup() -> MutexGuard<'static, ()> {
        testutil::install();
        let g = testutil::pages_install(BYTES);
        testutil::kernel_maps_install();
        mbinit(&TEST_NMBCLUST);
        nmbclust_update(NMBCLUSTERS_TEST).expect("the limit");
        g
    }

    /// The bytes the tests write: position `i` holds a value that differs from its neighbours.
    fn pattern(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + 3) as u8).collect()
    }

    /// The chain's data length.
    fn chain_len(m: &Mbuf) -> usize {
        let mut len = 0;
        let mut n = Some(m);
        while let Some(nn) = n {
            len += nn.m_len().get() as usize;
            n = nn.m_next.get();
        }
        len
    }

    /// The number of mbufs in the chain.
    fn chain_count(m: &Mbuf) -> usize {
        let mut count = 0;
        let mut n = Some(m);
        while let Some(nn) = n {
            count += 1;
            n = nn.m_next.get();
        }
        count
    }

    /// The chain's data.
    fn contents(m: &Mbuf) -> Vec<u8> {
        let mut v = vec![0u8; chain_len(m)];
        m_copydata(m, 0, &mut v);
        v
    }

    /// A packet of mbufs holding `lens` bytes each, filled with `pattern`.
    fn packet(lens: &[usize]) -> MbufBox {
        let data = pattern(lens.iter().sum());
        let mut off = 0;
        let mut chain = MChainBuilder::new();
        for &len in lens {
            let mut m = if chain.chain.is_empty() {
                m_gethdr(M_DONTWAIT, MT_DATA).expect("a header mbuf")
            } else {
                m_get(M_DONTWAIT, MT_DATA).expect("an mbuf")
            };
            if len > m_trailingspace(&m) as usize {
                assert!(mclgetl(&mut m, M_DONTWAIT, len as u32), "a cluster");
            }
            m.m_len().set(len as u32);
            m.data_mut()
                .expect("writable")
                .copy_from_slice(&data[off..off + len]);
            off += len;
            chain.push(m);
        }
        let top = chain.into_head().expect("at least one mbuf");
        top.m_pkthdr().len.set(off as i32);
        top
    }

    /// Every pool back to no item out.
    fn all_back() {
        assert_eq!(MBUFPL.nout(), 0, "mbufs out");
        assert_eq!(MTAGPL.nout(), 0, "tags out");
        assert_eq!(MEXTREFS.nout(), 0, "shared clusters out");
        for (i, n) in cluster_nout().iter().enumerate() {
            assert_eq!(*n, 0, "clusters of {} out", MCLNAMES[i]);
        }
    }

    /// The items out of each cluster pool.
    fn cluster_nout() -> [u64; 8] {
        [
            MCL2K.nout(),
            MCL2K2.nout(),
            MCL4K.nout(),
            MCL8K.nout(),
            MCL9K128.nout(),
            MCL12K.nout(),
            MCL16K.nout(),
            MCL64K.nout(),
        ]
    }

    #[test]
    fn the_layout_is_the_c_s() {
        assert_eq!((MSIZE, MLEN, MHLEN), (256, 224, 144));
        assert_eq!(size_of::<Mbuf>(), 256);
        assert_eq!(size_of::<Pkthdr>(), 80);
        assert_eq!(MINCLSIZE, 369);
        assert_eq!(size_of::<MTag>(), 96);
        assert_eq!(size_of::<Ext>(), 40);
        assert_eq!(size_of::<ExtRefs>(), 48);
        assert_eq!(core::mem::offset_of!(Mbuf, m_nextpkt), 8);
        assert_eq!(core::mem::offset_of!(Mbuf, m_len), 20);
    }

    #[test]
    fn get_gethdr_and_free() {
        let _g = setup();

        let m = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        assert_eq!(m.data_off(), 0);
        assert_eq!(m.m_flags().get(), 0);
        assert_eq!(m.m_len().get(), 0);
        assert_eq!(i32::from(m.m_type().get()), MT_DATA);
        assert_eq!(m_leadingspace(&m), 0);
        assert_eq!(m_trailingspace(&m), MLEN as i32);

        let mut h = m_gethdr(M_WAIT, MT_HEADER).expect("a header mbuf");
        assert_eq!(h.m_flags().get(), M_PKTHDR);
        assert_eq!(h.storage_len(), MHLEN);
        assert_eq!(h.m_pkthdr().pf.prio.get(), IFQ_DEFPRIO);
        assert_eq!(h.m_pkthdr().len.get(), 0);
        assert!(h.m_pkthdr().ph_tags.get().is_none());
        assert_eq!(m_trailingspace(&h), MHLEN as i32);
        assert_eq!(MBUFPL.nout(), 2);

        let at = PoolBox::as_ptr(&m);
        drop(h.m_next.set(m));
        let next = m_free(h).expect("the next mbuf");
        assert!(core::ptr::eq(PoolBox::as_ptr(&next), at));
        assert!(m_free(next).is_none());
        assert_eq!(MBUFPL.nout(), 0);

        let mut c = m_getclr(M_DONTWAIT, MT_DATA).expect("a zeroed mbuf");
        c.m_len().set(MLEN as u32);
        assert_eq!(contents(&c), vec![0u8; MLEN]);
        assert!(c.data_mut().is_some());
        drop(c);
        all_back();
    }

    #[test]
    fn every_cluster_lends_its_whole_item() {
        let _g = setup();

        for (i, &size) in MCLSIZES.iter().enumerate() {
            let mut m = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
            assert!(
                m_clget(&mut m, M_DONTWAIT, size),
                "a cluster of {}",
                MCLNAMES[i]
            );
            assert_eq!(m.ext().map(|e| e.cluster().index()), Some(i));
            assert_eq!(m.storage_len(), MCL_ITEM[i] as usize, "{}", MCLNAMES[i]);
            assert_eq!(m.storage().len(), m.storage_len(), "{}", MCLNAMES[i]);
            assert_eq!(m.storage_mut().map(|s| s.len()), Some(m.storage_len()));
            drop(m);
        }
        all_back();
    }

    #[test]
    fn cat_fills_a_mcl2k2_cluster_to_its_last_byte() {
        let _g = setup();

        // em(4)'s EM_MCLBYTES receive cluster, then a fragment of 10 bytes joined to it:
        // m_cat copies into the cluster's trailing space, the item's last 10 bytes.
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        assert!(m_clget(&mut m, M_DONTWAIT, MCLSIZES[1]));
        assert_eq!(m.ext().map(|e| e.cluster().index()), Some(1));
        let room = m.storage_len();
        assert_eq!(room, 2112);
        m.m_len.set((room - 10) as u32);
        let mut n = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        n.m_len.set(10);
        n.data_mut().expect("writable").fill(0xa5);
        assert_eq!(m_trailingspace(&m), 10);
        m_cat(&mut m, Some(n));
        assert_eq!(m.m_len.get() as usize, room);
        assert!(m.m_next.get().is_none(), "copied, not linked");
        assert_eq!(&m.data()[room - 10..], &[0xa5; 10]);
        assert_eq!(m_trailingspace(&m), 0);
        drop(m);
        all_back();
    }

    #[test]
    fn every_pool_has_the_c_name_and_item_size() {
        let _g = setup();

        assert_eq!(MBUFPL.item_size(), Some(256));
        assert_eq!(MTAGPL.item_size(), Some(96));
        assert_eq!(MEXTREFS.item_size(), Some(64));
        assert_eq!(MCL2K.item_size(), Some(2048));
        assert_eq!(MCL2K2.item_size(), Some(2112));
        assert_eq!(MCL4K.item_size(), Some(4096));
        assert_eq!(MCL8K.item_size(), Some(8192));
        assert_eq!(MCL9K128.item_size(), Some(9344));
        assert_eq!(MCL12K.item_size(), Some(12288));
        assert_eq!(MCL16K.item_size(), Some(16384));
        assert_eq!(MCL64K.item_size(), Some(65536));
        assert_eq!(
            MCLNAMES,
            [
                "mcl2k", "mcl2k2", "mcl4k", "mcl8k", "mcl9k128", "mcl12k", "mcl16k", "mcl64k"
            ]
        );
        for wchan in ["mbufpl", "mtagpl", "mextrefs", "mcl2k2", "mcl64k"] {
            assert!(testutil::pool_knobs(wchan).is_some(), "{wchan} made");
        }
        assert_eq!(m_clpool(1500), Some(0));
        assert_eq!(m_clpool(2049), Some(1));
        assert_eq!(m_clpool(2112), Some(1));
        assert_eq!(m_clpool(9000), Some(4));
        assert!(m_clpool(64 * 1024 + 1).is_none());
        assert_eq!(nmbclust_update(0), Err(Errno::ERANGE));
        assert_eq!(nmbclust_update(i64::MAX), Err(Errno::ERANGE));
        let raised = 2 * NMBCLUSTERS_TEST;
        assert_eq!(nmbclust_update(raised), Ok(()));
        assert_eq!(TEST_NMBCLUST.load(Ordering::Relaxed), raised);
        assert_eq!(
            MBUF_MEM_LIMIT.load(Ordering::Relaxed),
            raised as u64 * MCLBYTES as u64
        );
    }

    #[test]
    fn clusters_come_and_go() {
        let _g = setup();

        let m = m_clget_hdr(M_DONTWAIT, 9000).expect("a cluster packet");
        assert_eq!(
            m.m_flags().get() & (M_EXT | M_EXTWR | M_PKTHDR),
            M_EXT | M_EXTWR | M_PKTHDR
        );
        assert_eq!(m.ext().map(Ext::ext_size), Some(9344));
        assert_eq!(
            m.data_raw(),
            m.ext().map_or(core::ptr::null_mut(), Ext::ext_buf)
        );
        assert!(!m.is_readonly());
        assert_eq!(cluster_nout()[4], 1);
        assert!(MBUF_MEM_ALLOC.load(Ordering::Relaxed) > 0);
        assert!(m_pool_used() < 100);
        drop(m);
        all_back();
        assert_eq!(MBUF_MEM_ALLOC.load(Ordering::Relaxed), 0, "every page back");
    }

    #[test]
    fn the_memory_limit_refuses_a_page_and_counts_a_drop() {
        let _g = setup();

        let drops = mbstat_read().m_drops;
        // One cluster's worth of memory: an mbuf's page fits, a 4 KiB cluster's does not.
        nmbclust_update(1).expect("the limit");
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf within the limit");
        assert!(!mclgetl(&mut m, M_DONTWAIT, 4096), "no page past the limit");
        assert_eq!(mbstat_read().m_drops, drops + 1);
        drop(m);
        nmbclust_update(NMBCLUSTERS_TEST).expect("the limit");
        all_back();
    }

    #[test]
    fn copydata_and_copyback_round_trip() {
        let _g = setup();

        let mut m = packet(&[100, 50, 200]);
        let data = pattern(350);
        assert_eq!(contents(&m), data);

        let mut part = vec![0u8; 100];
        m_copydata(&m, 120, &mut part);
        assert_eq!(part, &data[120..220]);

        // across the first two mbufs
        let patch: Vec<u8> = (0..40).map(|i| 200 + i as u8).collect();
        m_copyback(&mut m, 90, &patch, M_DONTWAIT).expect("copyback");
        let mut want = data.clone();
        want[90..130].copy_from_slice(&patch);
        assert_eq!(contents(&m), want);
        assert_eq!(m.m_pkthdr().len.get(), 350);

        // past the end: the gap is zeroed and the packet grows
        let tail = pattern(300);
        m_copyback(&mut m, 360, &tail, M_DONTWAIT).expect("copyback past the end");
        want.extend_from_slice(&[0u8; 10]);
        want.extend_from_slice(&tail);
        assert_eq!(contents(&m), want);
        assert_eq!(m.m_pkthdr().len.get(), 660);
        m_calchdrlen(&m);
        assert_eq!(m.m_pkthdr().len.get(), 660);

        drop(m);
        all_back();
    }

    #[test]
    fn copyback_into_a_shared_cluster_takes_a_copy_of_its_own() {
        let _g = setup();

        let mut m = packet(&[1000]);
        let copy = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT).expect("a copy");
        assert!(m.is_readonly() && copy.is_readonly());
        m_copyback(&mut m, 10, &[0xee; 4], M_DONTWAIT).expect("copyback");
        assert!(!m.is_readonly(), "a cluster of its own");
        assert_eq!(&contents(&m)[10..14], &[0xee; 4]);
        assert_eq!(contents(&copy), pattern(1000), "the copy keeps its bytes");
        assert!(!copy.is_readonly(), "the copy is the cluster's last holder");
        drop(copy);
        drop(m);
        all_back();
    }

    #[test]
    fn adj_trims_the_head_and_the_tail() {
        let _g = setup();

        let m = packet(&[100, 50, 200]);
        let data = pattern(350);

        m_adj(&m, 120);
        assert_eq!(m.m_pkthdr().len.get(), 230);
        assert_eq!(m.m_len().get(), 0);
        assert_eq!(m.m_next.get().expect("second").m_len().get(), 30);
        assert_eq!(contents(&m), &data[120..]);

        // within the last mbuf only
        m_adj(&m, -10);
        assert_eq!(m.m_pkthdr().len.get(), 220);
        assert_eq!(contents(&m), &data[120..340]);

        // across mbufs
        m_adj(&m, -200);
        assert_eq!(m.m_pkthdr().len.get(), 20);
        assert_eq!(contents(&m), &data[120..140]);

        drop(m);
        all_back();
    }

    #[test]
    fn pullup_makes_the_head_contiguous() {
        let _g = setup();

        let data = pattern(260);
        let m = packet(&[10, 50, 200]);
        let m = m_pullup(m, 40).expect("pulled up");
        assert!(m.m_len().get() >= 40);
        assert_eq!(m.m_flags().get() & M_PKTHDR, M_PKTHDR);
        assert_eq!(m.m_pkthdr().len.get(), 260);
        assert_eq!(contents(&m), data);

        // more than a header mbuf holds: a new mbuf with a cluster leads the chain
        let m = m_pullup(m, 200).expect("pulled up into a cluster");
        assert!(m.m_len().get() >= 200);
        assert_eq!(m.m_flags().get() & M_EXT, M_EXT);
        assert_eq!(m.m_pkthdr().len.get(), 260);
        assert_eq!(contents(&m), data);

        // longer than the chain: the chain is freed
        assert!(m_pullup(m, 1000).is_none());
        all_back();
    }

    #[test]
    fn pullup_across_a_cluster_boundary() {
        let _g = setup();

        // A header mbuf of 14 bytes and a cluster of 1486: 26 bytes copied in.
        let data = pattern(1500);
        let m = packet(&[14, 1486]);
        assert!(m.m_next.get().is_some_and(|n| n.ext().is_some()));
        let m = m_pullup(m, 40).expect("pulled up");
        assert_eq!(m.m_len().get(), 40);
        assert_eq!(m.m_next.get().map(|n| n.m_len().get()), Some(1460));
        assert_eq!(contents(&m), data);
        drop(m);
        all_back();
    }

    #[test]
    fn split_plain_and_shared_clusters() {
        let _g = setup();

        let data = pattern(350);
        let mut m = packet(&[100, 50, 200]);
        let tail = m_split(&mut m, 120, M_DONTWAIT).expect("a tail");
        assert_eq!(m.m_pkthdr().len.get(), 120);
        assert_eq!(contents(&m), &data[..120]);
        assert_eq!(tail.m_flags().get() & M_PKTHDR, M_PKTHDR);
        assert_eq!(tail.m_pkthdr().len.get(), 230);
        assert_eq!(contents(&tail), &data[120..]);
        drop(m);
        drop(tail);

        // a cluster is shared, not copied
        let data = pattern(1000);
        let mut m = packet(&[1000]);
        assert_eq!(m.m_flags().get() & M_EXT, M_EXT);
        let tail = m_split(&mut m, 300, M_DONTWAIT).expect("a tail");
        assert_eq!(tail.m_flags().get() & M_PKTHDR, M_PKTHDR);
        assert_eq!(chain_count(&tail), 1);
        assert_eq!(tail.ext().map(Ext::ext_buf), m.ext().map(Ext::ext_buf));
        assert_eq!(tail.data_raw().addr() - m.data_raw().addr(), 300);
        assert!(m.is_readonly() && tail.is_readonly());
        assert_eq!(MEXTREFS.nout(), 1);
        assert_eq!(m_trailingspace(&m), 0);
        assert_eq!(contents(&m), &data[..300]);
        assert_eq!(contents(&tail), &data[300..]);
        assert_eq!(cluster_nout()[0], 1);
        drop(m);
        assert_eq!(cluster_nout()[0], 1, "the tail still holds the cluster");
        assert!(!tail.is_readonly(), "the last reference is writable again");
        drop(tail);
        all_back();
    }

    #[test]
    fn prepend_uses_leading_space_then_a_new_mbuf() {
        let _g = setup();

        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("a header mbuf");
        m_align(&m, 20);
        m.m_len().set(20);
        m_copyback(&mut m, 0, &pattern(20), M_DONTWAIT).expect("copyback");
        m.m_pkthdr().len.set(20);
        let room = m_leadingspace(&m);
        assert_eq!(room, (MHLEN - 24) as i32);

        let m = m_prepend(m, 8, M_DONTWAIT).expect("prepended in place");
        assert_eq!(chain_count(&m), 1);
        assert_eq!(m.m_len().get(), 28);
        assert_eq!(m.m_pkthdr().len.get(), 28);

        let big = m_leadingspace(&m) + 1;
        let first = PoolBox::as_ptr(&m);
        let m = m_prepend(m, big, M_DONTWAIT).expect("prepended in a new mbuf");
        assert!(!core::ptr::eq(PoolBox::as_ptr(&m), first));
        let old = m.m_next.get().expect("the old head");
        assert!(core::ptr::eq(old, first));
        assert_eq!(m.m_flags().get() & M_PKTHDR, M_PKTHDR);
        assert_eq!(old.m_flags().get() & M_PKTHDR, 0);
        assert_eq!(m.m_len().get(), big as u32);
        assert_eq!(m.m_pkthdr().len.get(), 28 + big);
        let mut tail = vec![0u8; 20];
        m_copydata(&m, 8 + big, &mut tail);
        assert_eq!(tail, pattern(20));
        drop(m);
        all_back();
    }

    #[test]
    fn cat_copies_small_data_and_links_large() {
        let _g = setup();

        let mut a = packet(&[30]);
        let mut b = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        b.m_len().set(10);
        b.data_mut().expect("writable").fill(9);
        m_cat(&mut a, Some(b));
        assert_eq!(chain_count(&a), 1, "the data was copied, b freed");
        let mut want = pattern(30);
        want.extend_from_slice(&[9u8; 10]);
        assert_eq!(contents(&a), want);

        let mut big = packet(&[200]);
        m_removehdr(&mut big);
        m_cat(&mut a, Some(big));
        assert_eq!(chain_count(&a), 2, "too big to copy: linked");
        drop(a);
        all_back();
    }

    #[test]
    fn copym_copies_and_shares() {
        let _g = setup();

        let data = pattern(1250);
        let mut m = packet(&[100, 150, 1000]);
        let copy = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT).expect("a copy");
        assert_eq!(copy.m_flags().get() & M_PKTHDR, M_PKTHDR);
        assert_eq!(copy.m_pkthdr().len.get(), 1250);
        assert_eq!(contents(&copy), data);
        assert_eq!(cluster_nout()[0], 1, "the cluster is shared");

        let part = m_copym(&mut m, 50, 200, M_DONTWAIT).expect("a partial copy");
        assert_eq!(part.m_flags().get() & M_PKTHDR, 0);
        assert_eq!(contents(&part), &data[50..250]);

        drop(m);
        drop(part);
        assert_eq!(cluster_nout()[0], 1);
        drop(copy);
        all_back();
    }

    #[test]
    fn copym_from_deep_in_the_chain() {
        let _g = setup();

        // One walk to offset 1100, in the third mbuf; no header copy away from offset 0.
        let mut m = packet(&[500, 500, 500]);
        let c = m_copym(&mut m, 1100, 100, M_DONTWAIT).expect("a copy");
        assert!(c.pkthdr().is_none());
        assert!(
            c.m_next.get().is_none(),
            "one mbuf: the copy fits the third's cluster"
        );
        let mut got = vec![0u8; 100];
        m_copydata(&c, 0, &mut got);
        assert_eq!(got, pattern(1500)[1100..1200].to_vec());
        // Across the second and the third.
        let c2 = m_copym(&mut m, 900, 200, M_DONTWAIT).expect("a copy");
        let mut got = vec![0u8; 200];
        m_copydata(&c2, 0, &mut got);
        assert_eq!(got, pattern(1500)[900..1100].to_vec());
        drop(c);
        drop(c2);
        drop(m);
        all_back();
    }

    #[test]
    fn defrag_and_dup_pkt() {
        let _g = setup();

        let data = pattern(600);
        let mut m = packet(&[100, 300, 200]);
        m_defrag(&mut m, M_DONTWAIT).expect("defragmented");
        assert_eq!(chain_count(&m), 1);
        assert_eq!(m.m_flags().get() & M_EXT, M_EXT);
        assert_eq!(m.m_pkthdr().len.get(), 600);
        assert_eq!(contents(&m), data);

        let d = m_dup_pkt(&m, 2, M_DONTWAIT).expect("a duplicate");
        assert_eq!(contents(&d), data);
        assert_eq!(d.m_pkthdr().len.get(), 600);
        assert_eq!(d.data_off(), 2);

        drop(m);
        drop(d);
        all_back();
    }

    #[test]
    fn getptr_apply_and_devget() {
        let _g = setup();

        let data = pattern(350);
        let m = packet(&[100, 50, 200]);
        let (n, off) = m_getptr(&m, 120).expect("inside");
        assert_eq!(n.m_len().get(), 50);
        assert_eq!(off, 20);
        let (n, off) = m_getptr(&m, 350).expect("the end");
        assert_eq!((n.m_len().get(), off), (200, 200));
        assert!(m_getptr(&m, 351).is_none());

        let mut sum = 0u64;
        m_apply(&m, 90, 100, |b| {
            sum += b.iter().map(|&x| u64::from(x)).sum::<u64>();
            Ok(())
        })
        .expect("applied");
        assert_eq!(
            sum,
            data[90..190].iter().map(|&x| u64::from(x)).sum::<u64>()
        );
        assert_eq!(
            m_apply(&m, 0, 10, |_| Err(Errno::EINVAL)),
            Err(Errno::EINVAL)
        );
        drop(m);

        MAX_LINKHDR.store(16, Ordering::Relaxed);
        let small = m_devget(&data[..60], 0).expect("a small packet");
        assert_eq!(small.m_pkthdr().len.get(), 60);
        assert_eq!(m_leadingspace(&small), 16);
        assert_eq!(contents(&small), &data[..60]);
        drop(small);

        let frame: Vec<u8> = pattern(1514);
        let big = m_devget(&frame, 2).expect("a cluster packet");
        assert_eq!(big.m_flags().get() & M_EXT, M_EXT);
        assert_eq!(contents(&big), frame);
        drop(big);
        MAX_LINKHDR.store(0, Ordering::Relaxed);
        assert!(m_devget(&[], 0).is_none());
        all_back();
    }

    #[test]
    fn pulldown_and_makespace() {
        let _g = setup();

        let data = pattern(260);
        let m = packet(&[10, 50, 200]);
        let (m, at, off) = m_pulldown(m, 5, 30, false).expect("pulled down");
        assert_eq!(off, 0);
        let mut n: &Mbuf = &m;
        for _ in 0..at {
            n = n.m_next.get().expect("the mbuf at the index");
        }
        assert!(n.m_len().get() >= 30);
        let mut got = vec![0u8; 30];
        m_copydata(n, 0, &mut got);
        assert_eq!(got, &data[5..35]);
        assert_eq!(contents(&m), data);

        let (m, at, off) = m_pulldown(m, 100, 20, true).expect("already contiguous");
        let mut n: &Mbuf = &m;
        for _ in 0..at {
            n = n.m_next.get().expect("the mbuf at the index");
        }
        assert!(n.m_len().get() as i32 - off >= 20);
        m_copydata(n, off, &mut got[..20]);
        assert_eq!(&got[..20], &data[100..120]);
        drop(m);

        let mut m = packet(&[40]);
        let head = PoolBox::as_ptr(&m);
        let (n, off) = m_makespace(&mut m, 20, 8).expect("space");
        assert!(core::ptr::eq(n, head));
        assert_eq!(off, 20);
        assert_eq!(m.m_pkthdr().len.get(), 48);
        let all = contents(&m);
        assert_eq!(&all[..20], &pattern(40)[..20]);
        assert_eq!(&all[28..], &pattern(40)[20..]);
        drop(m);
        all_back();
    }

    #[test]
    fn mbuf_lists() {
        let _g = setup();

        let mut ml = MbufList::new();
        assert!(ml_empty(&ml));
        let pkts: Vec<MbufBox> = (0..3).map(|_| packet(&[10])).collect();
        let addrs: Vec<*const Mbuf> = pkts.iter().map(PoolBox::as_ptr).collect();
        for p in pkts {
            ml_enqueue(&mut ml, p);
        }
        assert_eq!(ml_len(&ml), 3);
        assert_eq!(ml_hdatalen(&ml), 10);
        assert_eq!(ml.iter().count(), 3);
        let first = ml_dequeue(&mut ml).expect("first");
        assert!(core::ptr::eq(PoolBox::as_ptr(&first), addrs[0]));
        assert_eq!(ml_len(&ml), 2);

        let mut other = MbufList::new();
        ml_enqueue(&mut other, first);
        ml_enlist(&mut ml, &mut other);
        assert!(ml_empty(&other));
        let order: Vec<*const Mbuf> = ml.iter().map(core::ptr::from_ref).collect();
        assert_eq!(order, [addrs[1], addrs[2], addrs[0]].to_vec());
        let whole = ml_dechain(&mut ml);
        assert!(ml_empty(&ml) && ml_len(&whole) == 3);
        ml = whole;
        assert_eq!(ml_purge(&mut ml), 3);
        assert!(ml_empty(&ml));
        all_back();
    }

    /// `struct ip` (netinet/ip.h): 20 bytes.
    #[derive(Clone, Copy)]
    #[repr(C)]
    struct Ip {
        vhl: u8,
        tos: u8,
        len: u16,
        id: u16,
        off: u16,
        ttl: u8,
        p: u8,
        sum: u16,
        src: u32,
        dst: u32,
    }

    hal::abi_pod!(Ip {
        vhl: u8,
        tos: u8,
        len: u16,
        id: u16,
        off: u16,
        ttl: u8,
        p: u8,
        sum: u16,
        src: u32,
        dst: u32,
    });

    #[test]
    fn a_header_is_viewed_in_place_when_aligned() {
        let _g = setup();

        // ETHER_ALIGN: the frame 2 bytes into the cluster, so the IP header after the 14-byte
        // Ethernet header is aligned to 4.
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        assert!(mclgetl(&mut m, M_DONTWAIT, 1500));
        m.set_data_off(2);
        m.m_len().set(60);
        m.data_mut().expect("writable")[14 + 8] = 64;
        let ip = hal::pod::view::<Ip>(m.data(), 14).expect("aligned");
        assert_eq!(ip.ttl, 64);
        m.set_data_off(3);
        assert!(hal::pod::view::<Ip>(m.data(), 14).is_none(), "misaligned");
        drop(m);
        all_back();
    }

    /// A pf that counts its legs.
    struct CountingPf {
        links: AtomicU32,
        unlinks: AtomicU32,
    }

    impl pf_hooks::Pf for CountingPf {
        fn mbuf_link_state_key(&self, to: &PkthdrPf, from: &PkthdrPf) {
            if from.statekey.get() != 0 {
                to.statekey.set(from.statekey.get());
                self.links.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn mbuf_link_inpcb(&self, to: &PkthdrPf, from: &PkthdrPf) {
            if from.inp.get() != 0 {
                to.inp.set(from.inp.get());
                self.links.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn mbuf_unlink_state_key(&self, pf: &PkthdrPf) {
            if pf.statekey.get() != 0 {
                pf.statekey.set(0);
                self.unlinks.fetch_add(1, Ordering::Relaxed);
            }
        }
        fn mbuf_unlink_inpcb(&self, pf: &PkthdrPf) {
            if pf.inp.get() != 0 {
                pf.inp.set(0);
                self.unlinks.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    static COUNTING_PF: CountingPf = CountingPf {
        links: AtomicU32::new(0),
        unlinks: AtomicU32::new(0),
    };

    #[test]
    fn the_pf_legs_run_at_the_c_s_points() {
        let _g = setup();
        let _ = pf_hooks::register_pf(&COUNTING_PF);

        let (l0, u0) = (
            COUNTING_PF.links.load(Ordering::Relaxed),
            COUNTING_PF.unlinks.load(Ordering::Relaxed),
        );
        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_pkthdr().pf.statekey.set(0x1000);
        m.m_pkthdr().pf.inp.set(0x2000);
        // m_dup_pkthdr links both references for the copy.
        let mut d = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m_dup_pkthdr(&mut d, &m, M_DONTWAIT).expect("duplicated");
        assert_eq!(COUNTING_PF.links.load(Ordering::Relaxed), l0 + 2);
        assert_eq!(d.m_pkthdr().pf.statekey.get(), 0x1000);
        // M_MOVE_HDR moves them: nothing unlinked.
        let mut n = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m_move_pkthdr(&mut n, &mut m);
        assert_eq!(COUNTING_PF.unlinks.load(Ordering::Relaxed), u0);
        assert!(m.pkthdr().is_none());
        // m_free and m_removehdr unlink them.
        drop(d);
        assert_eq!(COUNTING_PF.unlinks.load(Ordering::Relaxed), u0 + 2);
        m_removehdr(&mut n);
        assert_eq!(COUNTING_PF.unlinks.load(Ordering::Relaxed), u0 + 4);
        drop(n);
        drop(m);
        all_back();
    }

    #[test]
    fn removehdr_drops_the_tags_and_keeps_the_data() {
        let _g = setup();

        let mut m = packet(&[40]);
        let t = m_tag_get(PACKET_TAG_GRE, 4, M_DONTWAIT).expect("a tag");
        m_tag_prepend(&mut m, t);
        assert_eq!(MTAGPL.nout(), 1);
        let at = m.data_raw();
        m_removehdr(&mut m);
        assert_eq!(MTAGPL.nout(), 0);
        assert_eq!(m.m_flags().get() & M_PKTHDR, 0);
        assert_eq!(m.data_raw(), at, "the data where it was");
        assert_eq!(m.data(), &pattern(40)[..]);
        assert_eq!(m.storage_len(), MLEN);
        drop(m);
        all_back();
    }

    #[test]
    fn the_flags_cannot_make_a_header_or_a_cluster() {
        let _g = setup();

        let m = m_gethdr(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m.m_flags().set(m.m_flags().get() | M_BCAST);
        assert_eq!(m.m_flags().get(), M_PKTHDR | M_BCAST);
        let r = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            m.m_flags().set(m.m_flags().get() & !M_PKTHDR);
        }));
        assert!(r.is_err(), "a header removed by a flag write");
        assert!(m.pkthdr().is_some());
        drop(m);
        all_back();
    }

    #[test]
    fn mbstat_counts_the_types() {
        let _g = setup();

        let before = mbstat_read();
        let m = m_get(M_DONTWAIT, MT_SONAME).expect("an mbuf");
        let during = mbstat_read();
        assert_eq!(
            during.m_mtypes[MT_SONAME as usize],
            before.m_mtypes[MT_SONAME as usize] + 1
        );
        drop(m);
        assert_eq!(mbstat_read(), before);
    }

    #[test]
    fn tags_are_added_found_copied_and_deleted() {
        let _g = setup();

        let mut m = m_gethdr(M_DONTWAIT, MT_DATA).expect("a header mbuf");
        assert!(m_tag_find(&m, PACKET_TAG_GRE, None).is_none());
        assert!(m_tag_get(PACKET_TAG_GRE, -1, M_DONTWAIT).is_none());

        let mut gre = m_tag_get(PACKET_TAG_GRE, 4, M_DONTWAIT).expect("a tag");
        gre.data_mut()[..4].copy_from_slice(&[1, 2, 3, 4]);
        let gre_at = PoolBox::as_ptr(&gre);
        m_tag_prepend(&mut m, gre);
        let dlt = m_tag_get(PACKET_TAG_DLT, 0, M_DONTWAIT).expect("a tag");
        let dlt_at = PoolBox::as_ptr(&dlt);
        m_tag_prepend(&mut m, dlt);
        let gre2 = m_tag_get(PACKET_TAG_GRE, 0, M_DONTWAIT).expect("a tag");
        let gre2_at = PoolBox::as_ptr(&gre2);
        m_tag_prepend(&mut m, gre2);
        assert_eq!(
            m.m_pkthdr().ph_tagsset.get(),
            PACKET_TAG_GRE | PACKET_TAG_DLT
        );
        assert_eq!(MTAGPL.nout(), 3);

        // newest first
        let first = m_tag_find(&m, PACKET_TAG_GRE, None).expect("found");
        assert!(core::ptr::eq(first, gre2_at));
        let second = m_tag_find(&m, PACKET_TAG_GRE, Some(first)).expect("found again");
        assert!(core::ptr::eq(second, gre_at));
        assert!(m_tag_find(&m, PACKET_TAG_TUNNEL, None).is_none());
        let f = m_tag_first(&m).expect("first");
        assert!(core::ptr::eq(f, gre2_at));
        assert!(core::ptr::eq(m_tag_next(&m, f).expect("next"), dlt_at));

        // a duplicate header copies the chain in order
        let mut d = m_get(M_DONTWAIT, MT_DATA).expect("an mbuf");
        m_dup_pkthdr(&mut d, &m, M_DONTWAIT).expect("duplicated");
        assert_eq!(MTAGPL.nout(), 6);
        let mut ids = Vec::new();
        let mut t = m_tag_first(&d);
        while let Some(tt) = t {
            ids.push(tt.m_tag_id.get());
            t = m_tag_next(&d, tt);
        }
        assert_eq!(ids, [PACKET_TAG_GRE, PACKET_TAG_DLT, PACKET_TAG_GRE]);
        let copied = m_tag_find(&d, PACKET_TAG_GRE, m_tag_find(&d, PACKET_TAG_GRE, None))
            .expect("the copied data tag");
        assert_eq!(&copied.bytes()[..4], &[1, 2, 3, 4]);
        drop(d);
        assert_eq!(MTAGPL.nout(), 3);

        m_tag_delete(&mut m, dlt_at);
        assert_eq!(m.m_pkthdr().ph_tagsset.get(), PACKET_TAG_GRE);
        m_tag_delete(&mut m, gre2_at);
        m_tag_delete(&mut m, gre_at);
        assert_eq!(m.m_pkthdr().ph_tagsset.get(), 0);
        assert!(m_tag_find(&m, PACKET_TAG_GRE, None).is_none());

        let t = m_tag_get(PACKET_TAG_TUNNEL, 8, M_DONTWAIT).expect("a tag");
        m_tag_prepend(&mut m, t);
        m_resethdr(&mut m);
        assert!(m.m_pkthdr().ph_tags.get().is_none());
        assert_eq!(m.m_pkthdr().pf.prio.get(), IFQ_DEFPRIO);
        drop(m);
        all_back();
    }

    #[test]
    fn ddb_printers() {
        let _g = setup();

        let m = packet(&[100, 2000]);
        let mut out = String::new();
        m_print(&m, &mut out).expect("printed");
        m_print_chain(Some(&m), false, &mut out).expect("printed");
        m_print_packet(Some(&m), false, &mut out).expect("printed");
        assert!(out.contains("m_type: 1\tm_flags: 2<M_PKTHDR>\n"), "{out}");
        assert!(out.contains("m_pkthdr.len: 2100\n"), "{out}");
        assert!(
            out.contains(", dat, off 0, len 100, pktlen 2100, size 144\n"),
            "{out}"
        );
        assert!(out.contains(", clsize 2048\n"), "{out}");
        assert!(
            out.contains(" \\- total chain 2, len 2100, size 2192\n"),
            "{out}"
        );
        assert!(
            out.contains(", chain 2, pktlen 2100, len 2100, size 2192\n"),
            "{out}"
        );
        assert!(out.contains("\\-- total packets 1\n"), "{out}");
        drop(m);
        all_back();
    }

    #[test]
    fn zero_clears_the_data_area() {
        let _g = setup();

        let mut m = packet(&[64]);
        m_zero(&mut m);
        assert_eq!(contents(&m), vec![0u8; 64]);
        m.m_flags().set(m.m_flags().get() | M_ZEROIZE);
        drop(m);

        // a shared cluster is zeroed when its last reference goes
        let mut m = packet(&[1000]);
        let mut c = m_copym(&mut m, 0, M_COPYALL, M_DONTWAIT).expect("a copy");
        m_zero(&mut m);
        assert!(shared_zero(&m), "the zeroing asked of the shared cluster");
        assert_eq!(contents(&c), pattern(1000), "not yet: the copy holds it");
        drop(m);
        // `c` holds the last reference: its release (the `ExtRefs` drop's) zeroes the bytes.
        match c.body.e_mut() {
            Some(Ext {
                kind: ExtKind::Shared(r),
                ..
            }) => PoolArc::get_mut(r)
                .expect("the last reference")
                .zero_if_asked(),
            _ => panic!("a shared cluster"),
        }
        assert_eq!(
            contents(&c),
            vec![0u8; 1000],
            "zeroed at the last reference"
        );
        drop(c);

        // a cluster of its own without M_EXTWR is zeroed all the same (the C's M_DATABUF)
        let mut m = packet(&[1000]);
        m.clear_bits(M_EXTWR);
        assert!(m.data_mut().is_none(), "read-only");
        m_zero(&mut m);
        assert_eq!(contents(&m), vec![0u8; 1000]);
        drop(m);
        all_back();
    }

    /// Whether `m`'s shared cluster was asked to be zeroed (`mrefs->zero`).
    fn shared_zero(m: &Mbuf) -> bool {
        match m.ext() {
            Some(Ext {
                kind: ExtKind::Shared(r),
                ..
            }) => r.zero.load(Ordering::Relaxed),
            _ => panic!("a shared cluster"),
        }
    }

    #[test]
    fn a_failed_copy_zeroes_every_mbuf_it_made() {
        let _g = setup();

        // Three clusters, shared once (the copy below shares them without a mextrefs
        // allocation), then the packet marked M_ZEROIZE (pfkey's), which its copies inherit
        // (M_COPYFLAGS).
        let mut src = packet(&[1000, 1000, 1000]);
        let first = m_copym(&mut src, 0, M_COPYALL, M_DONTWAIT).expect("a first copy");
        src.set_bits(M_ZEROIZE);
        // The copy's header and its second mbuf are made, the third refused: m_copym's
        // nospace frees the two from the head (the C's m_freem(top)).
        testutil::fail_pool_after("mbufpl", 2, 1);
        assert!(m_copym(&mut src, 0, M_COPYALL, M_DONTWAIT).is_none());
        testutil::fail_pool("mbufpl", 0);
        let m2 = src.m_next.get().expect("a second mbuf");
        let m3 = m2.m_next.get().expect("a third mbuf");
        assert!(shared_zero(&src), "the copy's header zeroed its cluster");
        assert!(
            shared_zero(m2),
            "M_ZEROIZE carried to the copy's second mbuf"
        );
        assert!(!shared_zero(m3), "the third cluster was never in the copy");
        drop(first);
        drop(src);
        all_back();
    }
}
/* </TESTS> */
