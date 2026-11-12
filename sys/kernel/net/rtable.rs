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
 * Copyright (c) 2014-2016 Martin Pieuchot
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
//! The routing tables: `<net/rtable.h>` and `net/rtable.c`, the table backend over the ART
//! ([`crate::net::art`]).
//!
//! Upstream: sys/net/rtable.h @ 3ce1f3f79392
//! Upstream: sys/net/rtable.c @ 3ce1f3f79392
//! LZ: sys/net/rtable.rs@3c62ede68802
//!
//! [`rtable_get`] finds the table of a pair of address family and routing table id through
//! `afmap`: `afmap[af2idx[af]]` is an array of tables indexed by the id, and `afmap[0]` maps
//! every id to its routing domain and loopback interface (8 unused bits, 16 bits of loopback
//! index, 8 bits of rdomain). Table heads are never freed, so they are not reference counted.
//! Each ART node ([`RtNode`]) holds a chain of routes (`rt_next`) for the same destination and
//! prefix length, ordered by priority: the multipath routes.
//!
//! Locks, as in the C: \[I\] immutable after creation, \[N\] net lock; each table's `r_lock`
//! serialises the changes of its tree and chains; lookups run inside SMR read sections;
//! `afmap`'s maps are replaced under the kernel lock.
//!
//! ## Deviations
//! - `afmap` is a static array of [`SmrPtr`]s (the C's `mallocarray(af2idx_max + 1)` of
//!   `struct srp`), sized for the routing domains this kernel can have; a map is an item of
//!   the pool `"rtmappl"` (D-N6.3b-12) whose array is the C's `mallocarray(M_RTABLE)`, where
//!   the C `malloc`s the `struct rtmap` too. `srp_update_locked`'s garbage collector is the
//!   deferred release of the replaced map (`SmrRetired::defer`).
//! - A table embeds its ART (`art_alloc`'s `malloc` folded into the table's): `M_RTABLE`
//!   counts one request fewer per table and family.
//! - `rtable_satoplen` answers `Err(EINVAL)` for the C's -1 (a family without a routing table
//!   or a mask that is not contiguous); `rtable_walk`'s and `rtable_read`'s callbacks are
//!   closures whose `Err` stops the walk, the C's non-zero `int`.
//! - `r_source` is a copy of the source address (D-N6.3b-13), where the C keeps a pointer
//!   into the interface address's storage; `rtable_clearsource` clears it on the same test.
//! - A key shorter than the table's address (a socket address whose `sa_len` stops before
//!   its address's end) is read with zeros past its end, where the C reads the bytes that
//!   follow it in memory.
//! - `malloc(M_WAITOK)` and `pool_get(PR_WAITOK)` that fail are `panic(9)`.
//!
//! ## Redesign
//! - The ART's node is [`RtNode`], an item of the pool `"art_node"` implementing
//!   [`ArtItem`]: its `an_value`, the head of the route chain, is a counted SMR word
//!   ([`SmrArcWord`]) and so is every route's `rt_next`: the word holds the table's reference
//!   of the route (the C's `rtref` in `rtable_insert`), so a reader's route is sound by SMR
//!   alone and the C's XXX in `rtable_delete` ("an and the rt ref could still be alive on
//!   other cpus") is closed by the type, not by the net lock. The lookups answer a counted
//!   handle taken inside the read section ([`SmrRef::arc`], the C's `rtref`).
//! - Chain edits move references rather than retire and take them, so `rmx_refcnt` (the
//!   count less the table's reference) stays the C's: an entry linked takes its
//!   predecessor's word and gives the old value to its own `rt_next`
//!   ([`SmrArcWord::restore`]); an entry unlinked gives its `rt_next` to its predecessor's
//!   word, and its own reference, from that word, retires. The C's order keeps a reader who
//!   is inside the chain from seeing it cut; here a reader inside the entry being linked or
//!   unlinked at that moment may end its walk there. The chains' writers hold the net lock
//!   exclusively or with the kernel lock (route.c's callers), so no reader of the net lock
//!   is inside then (N6.3b1's R2).
//! - A deleted route's table reference is released by the SMR thread after a grace period
//!   (the retired word's, or the deleted node's when its last reference goes), where the C's
//!   `rtable_delete` drops it at once (D-N6.3b-23): the last `rtfree` of a deleted route
//!   runs later.
//! - The routing domains' parameters (`dom_family`, `dom_rtoffset`, `dom_maxplen`,
//!   `dom_sasize`) are read from `domains[]` once by [`rtable_init`] into this module's own
//!   table, so the routing code is not generic over the process glue.
//! - Lock and spl transitions against the C: `rtable_match` and `rtable_lookup` take two read
//!   sections (the map's, the tree's: the C's `srp_enter` and `smr_read_enter`) and no lock;
//!   `rtable_insert`, `rtable_delete` and `rtable_mpath_reprio` take `r_lock` once and a read
//!   section inside it (one per-CPU count pair, no spl, where the C's writers read without
//!   one); `rtable_walk` takes `r_lock` once per node and a read section per step, as the C's.
//!   `rtable_read` holds `r_lock` throughout, as the C's, and a read section per step, never
//!   across its callback (which may sleep: `sysctl_dumpentry`'s copyout): each route is lent
//!   through a counted handle, one atomic pair per route the C does not pay (a dump, a
//!   control path).
//! - Costs against the C: the lookups are the C's loads; a chain edit pays one atomic per
//!   word it publishes (the reference it holds); `rtable_lookup` with a gateway compares it
//!   through the route's read-mostly cell (one compare-and-swap per entry looked at, the
//!   `rt_gateway` row of N12).

use core::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use hal::intr::{IPL_NONE, IPL_SOFTNET};
use hal::klock;
use hal::link::Link;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolArc, TypedPool};
use hal::rwlock::{Exclusive, RawRwlock};
use hal::slot::ReadMostly;
use hal::smr::{
    self, SmrArcLink, SmrArcWord, SmrCounted, SmrLink, SmrLinked, SmrPtr, SmrRead, SmrRef,
};
use hal::sys::errno::Errno;
use hal::sys::net_route::{RTF_MPATH, RTP_ANY, RTP_MASK};
use hal::sys::socket::{AF_MAX, RT_TABLEID_BITS, RT_TABLEID_MASK, RT_TABLEID_MAX};
use hal::sys::types::SaFamily;

use crate::assert::{kassert, kpanic};
use crate::net::art::{Art, ArtInsert, ArtItem, ArtIter, ArtPools};
use crate::net::domain::Domain;
use crate::net::proto::SockaddrView;
use crate::net::route::{Rtentry, rt_hash};
use crate::proc::ProcGlue;

/// `M_RTABLE`: the `malloc(9)` type of the routing tables (`<sys/malloc.h>`, `bsd`'s; a
/// copy).
pub const M_RTABLE: i32 = 5;

/// The routing domains a kernel can have (the domains with a `dom_rtoffset`: inet, inet6,
/// and mpls when configured).
const RT_MAXDOMS: usize = 4;

/// `afmap`'s length: index 0 for the routing domain map, one per routing domain.
const AFMAP_LEN: usize = RT_MAXDOMS + 1;

/// A copy of a socket address up to [`SA_COPY_MAX`] bytes (a route's key, a mask, a table's
/// source address: `struct sockaddr_in6`'s 28 bytes are the largest, a link-level address's
/// `struct sockaddr_dl` 32), held by value where the C keeps a `struct sockaddr_in6` buffer
/// or a pointer.
#[derive(Clone, Copy)]
pub struct SaCopy {
    /// The bytes; `buf[0]` is `sa_len`.
    buf: [u8; SA_COPY_MAX],
}

/// The bytes an [`SaCopy`] holds at most.
pub const SA_COPY_MAX: usize = 32;

impl SaCopy {
    /// An all-zero address (`sa_len` 0).
    pub const fn zeroed() -> Self {
        Self {
            buf: [0; SA_COPY_MAX],
        }
    }

    /// A copy of `sa`, `None` when its `sa_len` is above [`SA_COPY_MAX`].
    pub fn of(sa: SockaddrView<'_>) -> Option<Self> {
        let b = sa.bytes();
        if usize::from(sa.len()) > SA_COPY_MAX {
            return None;
        }
        let mut c = Self::zeroed();
        c.buf[..b.len()].copy_from_slice(b);
        Some(c)
    }

    /// The address's bytes: `sa_len` of them (at least the two of the header).
    pub fn bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.buf[0]).clamp(2, SA_COPY_MAX)]
    }

    /// The whole buffer, for a writer that builds an address in place.
    pub fn buf_mut(&mut self) -> &mut [u8; SA_COPY_MAX] {
        &mut self.buf
    }

    /// The address as a view.
    pub fn view(&self) -> SockaddrView<'_> {
        match SockaddrView::new(self.bytes()) {
            Some(v) => v,
            None => kpanic(format_args!("SaCopy: a view of its own bytes")),
        }
    }
}

impl Default for SaCopy {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// `struct rtable`: a routing table of one family and id, never freed.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct rtable's r_* names so the C can be grepped"
)]
pub struct Rtable {
    /// `r_lock`: serialises the changes of the tree and its chains.
    r_lock: RawRwlock,
    /// \[I\] `r_art`: the tree, embedded (the C's `art_alloc`).
    r_art: Art<RtNode>,
    /// \[I\] `r_off`: offset of the key in the socket address, in bytes.
    r_off: u32,
    /// \[N\] `r_source`: the source address the table's users prefer, a copy.
    r_source: ReadMostly<Option<SaCopy>>,
}

/// The slots of a map: a family's tables, or (index 0) the routing domains of the ids.
enum RtmapSlots {
    /// `struct rtmap`'s `tbl`: the table of each id (none for an id without one).
    Tables(MArray<Link<Rtable>>),
    /// `struct dommp`'s `value`: each id's loopback index and routing domain.
    Domains(MArray<AtomicU64>),
}

/// `struct rtmap` and `struct dommp` (the same layout in the C, read one as the other):
/// one published array of one of `afmap`'s slots, an item of `"rtmappl"`.
pub struct Rtmap {
    /// The SMR link the replaced map's release is queued by.
    smr: SmrLink,
    /// The slots, `limit` of them (`mallocarray(M_RTABLE)` of a pointer each, the C's).
    slots: RtmapSlots,
}

impl Rtmap {
    /// `limit`.
    fn limit(&self) -> u32 {
        match &self.slots {
            RtmapSlots::Tables(t) => t.len() as u32,
            RtmapSlots::Domains(d) => d.len() as u32,
        }
    }

    /// The table of `id`.
    fn table(&self, id: u32) -> Option<&'static Rtable> {
        match &self.slots {
            RtmapSlots::Tables(t) => t.get(id as usize).and_then(Link::get),
            RtmapSlots::Domains(_) => None,
        }
    }

    /// The routing domain word of `id`.
    fn value(&self, id: u32) -> Option<u32> {
        match &self.slots {
            RtmapSlots::Domains(d) => d.get(id as usize).map(|v| v.load(Ordering::Relaxed) as u32),
            RtmapSlots::Tables(_) => None,
        }
    }
}

impl SmrLinked for Rtmap {
    const POOL: &'static BoxPool<Rtmap> = &RTMAP_POOL;
    fn smr_link(&self) -> &SmrLink {
        &self.smr
    }
}

/// `struct art_node` of the routing tables: a prefix and the chain of its routes, an item
/// of the pool `"art_node"` counted by the ART's heap words that name it.
pub struct RtNode {
    /// The retirement link of the heap words' references.
    link: SmrArcLink,
    /// `an_value`: the first route of the chain, the table's reference of it.
    an_value: SmrArcWord<Rtentry>,
    /// `an_addr`: the prefix, its first `an_plen` bits significant.
    an_addr: [u8; 16],
    /// `an_plen`: the prefix length.
    an_plen: u32,
}

impl RtNode {
    /// The first route of the node's chain, for the section `g`.
    pub fn first<'g>(&'g self, g: &'g SmrRead) -> Option<SmrRef<'g, Rtentry>> {
        self.an_value.load(g)
    }
}

impl SmrCounted for RtNode {
    fn smr_link(&self) -> &SmrArcLink {
        &self.link
    }
}

impl ArtItem for RtNode {
    fn addr(&self) -> &[u8; 16] {
        &self.an_addr
    }
    fn plen(&self) -> u32 {
        self.an_plen
    }
    fn pools() -> &'static ArtPools<Self> {
        &RT_ART_POOLS
    }
}

/// One routing domain's parameters, read from `domains[]` by [`rtable_init`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RtDom {
    /// `dom_family`.
    pub family: SaFamily,
    /// `dom_rtoffset`: offset of the key, in bytes.
    pub rtoffset: u32,
    /// `dom_maxplen`: maximum prefix length, in bits.
    pub maxplen: u32,
    /// `dom_sasize`: size of the domain's socket address.
    pub sasize: u32,
}

/// One slot of the routing domains' table (written once at boot, read without a lock).
struct RtDomSlot {
    family: AtomicU8,
    rtoffset: AtomicU32,
    maxplen: AtomicU32,
    sasize: AtomicU32,
}

impl RtDomSlot {
    const fn new() -> Self {
        Self {
            family: AtomicU8::new(0),
            rtoffset: AtomicU32::new(0),
            maxplen: AtomicU32::new(0),
            sasize: AtomicU32::new(0),
        }
    }

    fn get(&self) -> RtDom {
        RtDom {
            family: self.family.load(Ordering::Relaxed),
            rtoffset: self.rtoffset.load(Ordering::Relaxed),
            maxplen: self.maxplen.load(Ordering::Relaxed),
            sasize: self.sasize.load(Ordering::Relaxed),
        }
    }
}

/// The routing domains, in `domains[]`'s order.
static RT_DOMS: [RtDomSlot; RT_MAXDOMS] = [const { RtDomSlot::new() }; RT_MAXDOMS];

/// How many of [`RT_DOMS`] are set.
static RT_NDOMS: AtomicUsize = AtomicUsize::new(0);

/// `af2idx`: each family's slot of `afmap` (0: no routing table).
static AF2IDX: [AtomicU8; AF_MAX as usize + 1] = [const { AtomicU8::new(0) }; AF_MAX as usize + 1];

/// `af2idx_max`: one past the last slot given.
static AF2IDX_MAX: AtomicU8 = AtomicU8::new(0);

/// `afmap`: the published map of each slot, replaced under the kernel lock.
static AFMAP: [SmrPtr<Rtmap>; AFMAP_LEN] = [const { SmrPtr::new_kernel() }; AFMAP_LEN];

/// `rtmap_limit`: the ids every map has room for.
pub static RTMAP_LIMIT: AtomicU32 = AtomicU32::new(0);

/// The pool of the maps (`"rtmappl"`, D-N6.3b-12).
static RTMAP_POOL: BoxPool<Rtmap> = BoxPool::new();

/// `an_pool` (`"art_node"`): the routing tables' nodes.
static AN_POOL: TypedPool<RtNode> = TypedPool::new();

/// The tables' and heaps' pools of the routing tables' ART (`art_table`, `art_heap4`,
/// `art_heap8`).
static RT_ART_POOLS: ArtPools<RtNode> = ArtPools::new();

/// The routing domains (a snapshot of the table `rtable_init` filled).
fn rt_doms() -> impl Iterator<Item = RtDom> {
    RT_DOMS[..RT_NDOMS.load(Ordering::Acquire).min(RT_MAXDOMS)]
        .iter()
        .map(RtDomSlot::get)
}

/// The routing domain of family `af`.
pub fn rt_dom(af: SaFamily) -> Option<RtDom> {
    rt_doms().find(|d| d.family == af)
}

/// The published map of slot `idx`, for the section `g`.
fn afmap_load(idx: usize, g: &SmrRead) -> Option<&Rtmap> {
    AFMAP.get(idx)?.load(g)
}

/// `rtmap_init()`: a single table for every domain that requires it, and the id to
/// routing domain map; `rtmap_limit` is 1. Under the kernel lock.
fn rtmap_init() {
    // Start with a single table for every domain that requires it.
    for d in rt_doms() {
        rtmap_grow(1, d.family);
    }

    // Initialize the rtableid->rdomain mapping table.
    rtmap_grow(1, 0);

    RTMAP_LIMIT.store(1, Ordering::Release);
}

/// `rtmap_grow(nlimit, af)`: the array of family `af` (0: the routing domain map) grown to
/// `nlimit`: the new map made first (its allocations may sleep, as the C's `M_WAITOK`), then,
/// under the kernel lock's witness, the old entries copied into it and the new map
/// published, the old one released after a grace period. The caller holds the kernel lock
/// (the C's `KERNEL_ASSERT_LOCKED`).
fn rtmap_grow(nlimit: u32, af: SaFamily) {
    kassert!(klock::kernel_lock_held());
    kassert!(nlimit > RTMAP_LIMIT.load(Ordering::Relaxed));

    let idx = usize::from(AF2IDX[usize::from(af)].load(Ordering::Relaxed));
    let Some(slot) = AFMAP.get(idx) else {
        kpanic(format_args!("rtmap_grow: no slot for af {af}"));
    };
    let n = nlimit as usize;
    let made = if idx == 0 {
        MArray::new(M_RTABLE, M_WAITOK, n, |_| AtomicU64::new(0)).map(RtmapSlots::Domains)
    } else {
        MArray::new(M_RTABLE, M_WAITOK, n, |_| Link::new()).map(RtmapSlots::Tables)
    };
    let Some(slots) = made else {
        kpanic(format_args!("rtmap_grow: out of memory"));
    };
    let Some(nmap) = RTMAP_POOL.get(PR_WAITOK, || Rtmap {
        smr: SmrLink::new(),
        slots,
    }) else {
        kpanic(format_args!("rtmap_grow: out of memory"));
    };

    let mut w = klock::witness();
    if let Some(map) = slot.load_locked(&w) {
        kassert!(map.limit() == RTMAP_LIMIT.load(Ordering::Relaxed));
        for i in 0..map.limit() {
            match &nmap.slots {
                RtmapSlots::Tables(t) => t[i as usize].set(map.table(i)),
                RtmapSlots::Domains(d) => {
                    d[i as usize].store(u64::from(map.value(i).unwrap_or(0)), Ordering::Relaxed);
                }
            }
        }
    }
    if let Some(old) = slot.store(&mut w, Some(nmap)) {
        old.defer();
    }
}

/// `rtable_init()`: the routing domains read from `domains[]` (those with a
/// `dom_rtoffset`), then [`rtable_init_doms`].
pub fn rtable_init<G: ProcGlue>() {
    let mut doms = [RtDom {
        family: 0,
        rtoffset: 0,
        maxplen: 0,
        sasize: 0,
    }; RT_MAXDOMS];
    let mut n = 0;
    for dp in crate::net::domain::net::<G>().domains() {
        let dp: &dyn Domain<G> = *dp;
        if dp.rtoffset() == 0 {
            continue;
        }
        if n == RT_MAXDOMS {
            kpanic(format_args!(
                "rtable_init: more routing domains than {RT_MAXDOMS}"
            ));
        }
        doms[n] = RtDom {
            family: dp.family() as SaFamily,
            rtoffset: dp.rtoffset(),
            maxplen: dp.maxplen(),
            sasize: dp.sasize(),
        };
        n += 1;
    }
    rtable_init_doms(&doms[..n]);
}

/// `rtable_init()` over the routing domains `doms`: each family's slot of `afmap`
/// (`af2idx`), the ART's pools, a map per family and the routing domain map, then the
/// default table (`panic(9)` when it cannot be made) and the route timers.
pub fn rtable_init_doms(doms: &[RtDom]) {
    // We use index 0 for the rtable/rdomain map.
    let mut max = 1u8;
    for a in &AF2IDX {
        a.store(0, Ordering::Relaxed);
    }
    for (slot, d) in RT_DOMS.iter().zip(doms) {
        slot.family.store(d.family, Ordering::Relaxed);
        slot.rtoffset.store(d.rtoffset, Ordering::Relaxed);
        slot.maxplen.store(d.maxplen, Ordering::Relaxed);
        slot.sasize.store(d.sasize, Ordering::Relaxed);
        AF2IDX[usize::from(d.family)].store(max, Ordering::Relaxed);
        max += 1;
    }
    RT_NDOMS.store(doms.len().min(RT_MAXDOMS), Ordering::Release);
    AF2IDX_MAX.store(max, Ordering::Relaxed);
    rtable_init_backend();

    klock::kernel_lock();
    rtmap_init();
    klock::kernel_unlock();

    if rtable_add(0).is_err() {
        kpanic(format_args!("unable to create default routing table"));
    }

    crate::net::route::rt_timer_init();
}

/// `rtable_add(id)`: a table of every routing domain for `id` (`EINVAL` above
/// `RT_TABLEID_MAX`, `ENOMEM` without memory), in routing domain 0 with its loopback
/// unset until `rtable_l2set`; nothing for an id that exists. Takes the kernel lock.
pub fn rtable_add(id: u32) -> Result<(), Errno> {
    if id > RT_TABLEID_MAX {
        return Err(Errno::EINVAL);
    }

    klock::kernel_lock();
    let r = rtable_add_locked(id);
    klock::kernel_unlock();
    r
}

/// `rtable_add`'s body, under the kernel lock.
fn rtable_add_locked(id: u32) -> Result<(), Errno> {
    if rtable_exists(id) {
        return Ok(());
    }

    for d in rt_doms() {
        if id >= RTMAP_LIMIT.load(Ordering::Relaxed) {
            rtmap_grow(id + 1, d.family);
        }

        let tbl = rtable_alloc(id, d.maxplen, d.rtoffset).ok_or(Errno::ENOMEM)?;

        let idx = usize::from(AF2IDX[usize::from(d.family)].load(Ordering::Relaxed));
        let g = smr::read();
        match afmap_load(idx, &g) {
            Some(Rtmap {
                slots: RtmapSlots::Tables(t),
                ..
            }) if (id as usize) < t.len() => t[id as usize].set(Some(tbl)),
            _ => kpanic(format_args!("rtable_add: no map for af {}", d.family)),
        }
    }

    // Reflect possible growth.
    if id >= RTMAP_LIMIT.load(Ordering::Relaxed) {
        rtmap_grow(id + 1, 0);
        RTMAP_LIMIT.store(id + 1, Ordering::Release);
    }

    // Use main rtable/rdomain by default.
    let g = smr::read();
    match afmap_load(0, &g) {
        Some(Rtmap {
            slots: RtmapSlots::Domains(d),
            ..
        }) if (id as usize) < d.len() => d[id as usize].store(0, Ordering::Relaxed),
        _ => kpanic(format_args!("rtable_add: no routing domain map")),
    }
    Ok(())
}

/// `rtable_get(rtableid, af)`: the table of family `af` and id `rtableid`.
#[inline]
pub fn rtable_get(rtableid: u32, af: SaFamily) -> Option<&'static Rtable> {
    let idx = usize::from(AF2IDX.get(usize::from(af))?.load(Ordering::Relaxed));
    if idx == 0 {
        return None;
    }
    let g = smr::read();
    afmap_load(idx, &g)?.table(rtableid)
}

/// `rtable_exists(rtableid)`: whether a table of some routing domain has the id.
pub fn rtable_exists(rtableid: u32) -> bool {
    rt_doms().any(|d| rtable_get(rtableid, d.family).is_some())
}

/// `rtable_empty(rtableid)`: whether every table of the id holds no route.
pub fn rtable_empty(rtableid: u32) -> bool {
    rt_doms().all(|d| rtable_get(rtableid, d.family).is_none_or(|t| t.r_art.art_is_empty()))
}

/// `rtable_l2(rtableid)`: the id's routing domain (0 for an unknown id).
pub fn rtable_l2(rtableid: u32) -> u32 {
    let g = smr::read();
    afmap_load(0, &g)
        .and_then(|m| m.value(rtableid))
        .map_or(0, |v| v & RT_TABLEID_MASK)
}

/// `rtable_loindex(rtableid)`: the index of the id's loopback interface (0 for an unknown
/// id).
pub fn rtable_loindex(rtableid: u32) -> u32 {
    let g = smr::read();
    afmap_load(0, &g)
        .and_then(|m| m.value(rtableid))
        .map_or(0, |v| v >> RT_TABLEID_BITS)
}

/// `rtable_l2set(rtableid, rdomain, loifidx)`: puts the id in routing domain `rdomain`
/// with loopback interface `loifidx`; nothing unless both ids exist. Under the kernel lock.
pub fn rtable_l2set(rtableid: u32, rdomain: u32, loifidx: u32) {
    kassert!(klock::kernel_lock_held());

    if !rtable_exists(rtableid) || !rtable_exists(rdomain) {
        return;
    }

    let value = (rdomain & RT_TABLEID_MASK) | (loifidx << RT_TABLEID_BITS);

    let g = smr::read();
    if let Some(Rtmap {
        slots: RtmapSlots::Domains(d),
        ..
    }) = afmap_load(0, &g)
        && let Some(v) = d.get(rtableid as usize)
    {
        v.store(u64::from(value), Ordering::Relaxed);
    }
}

/// `rtable_init_backend()`: `art_boot()`, the ART's pools of the routing tables (a second
/// call, a test's, leaves them as they are).
fn rtable_init_backend() {
    let _ = RT_ART_POOLS.art_boot();
    let _ = AN_POOL.init(IPL_SOFTNET, 0, "art_node");
    let _ = RTMAP_POOL.init(IPL_NONE, 0, "rtmappl");
}

/// `rtable_alloc(rtableid, alen, off)`: a new table for addresses of `alen` bits at offset
/// `off` of their socket address, never freed; `None` without memory.
fn rtable_alloc(_rtableid: u32, alen: u32, off: u32) -> Option<&'static Rtable> {
    let tbl = MBox::new(
        M_RTABLE,
        M_NOWAIT,
        Rtable {
            r_lock: RawRwlock::new("rtable"),
            r_art: Art::new(alen),
            r_off: off,
            r_source: ReadMostly::new(None),
        },
    )?;
    Some(MBox::leak(tbl))
}

/// `rtable_setsource(rtableid, af, src)`: the source address table `rtableid` of family
/// `af` prefers (`None` clears it); `EAFNOSUPPORT` without such a table. Under the net lock
/// held exclusively (the C's assertion, the caller's).
pub fn rtable_setsource(
    rtableid: u32,
    af: SaFamily,
    src: Option<SockaddrView<'_>>,
) -> Result<(), Errno> {
    let tbl = rtable_get(rtableid, af).ok_or(Errno::EAFNOSUPPORT)?;
    let _ = tbl.r_source.replace(src.and_then(SaCopy::of));
    Ok(())
}

/// `rtable_getsource(rtableid, af)`: a copy of the source address table `rtableid` of
/// family `af` prefers.
pub fn rtable_getsource(rtableid: u32, af: SaFamily) -> Option<SaCopy> {
    *rtable_get(rtableid, af)?.r_source.read()
}

/// `rtable_clearsource(rtableid, src)`: clears the preferred source of `src`'s family when
/// it is `src`.
pub fn rtable_clearsource(rtableid: u32, src: SockaddrView<'_>) {
    if let Some(addr) = rtable_getsource(rtableid, src.family())
        && addr.bytes().len() == src.bytes().len()
        && addr.bytes() == src.bytes()
    {
        let _ = rtable_setsource(rtableid, src.family(), None);
    }
}

/// The key of `sa` for `tbl` (`satoaddr`): the bytes from the table's offset, or, for an
/// address shorter than the table's address length, a copy padded with zeros, given to `f`.
#[inline]
fn with_key<R>(tbl: &Rtable, sa: SockaddrView<'_>, f: impl FnOnce(&[u8]) -> R) -> R {
    let off = tbl.r_off as usize;
    let need = (tbl.r_art.art_alen() as usize).div_ceil(8);
    let b = sa.bytes();
    if b.len() >= off + need {
        f(&b[off..])
    } else {
        let mut key = [0u8; 16];
        if let Some(have) = b.get(off..) {
            let n = have.len().min(key.len());
            key[..n].copy_from_slice(&have[..n]);
        }
        f(&key[..need.min(16)])
    }
}

/// Whether route `rt`'s priority is `prio` (any for `RTP_ANY`).
#[inline]
fn prio_is(rt: &Rtentry, prio: u8) -> bool {
    prio == RTP_ANY || (rt.rt_priority.get() & RTP_MASK) == (prio & RTP_MASK)
}

/// `rtable_lookup(rtableid, dst, mask, gateway, prio)`: the route of exactly `dst`/`mask`
/// (the best match of `dst` without a mask) of priority `prio` and gateway `gateway` (any
/// when `None`), a counted handle.
pub fn rtable_lookup(
    rtableid: u32,
    dst: SockaddrView<'_>,
    mask: Option<SockaddrView<'_>>,
    gateway: Option<SockaddrView<'_>>,
    prio: u8,
) -> Option<PoolArc<Rtentry>> {
    let tbl = rtable_get(rtableid, dst.family())?;
    let plen = match mask {
        Some(m) => Some(rtable_satoplen(dst.family(), Some(m)).ok()?),
        None => None,
    };

    with_key(tbl, dst, |addr| {
        let g = smr::read();
        let an = match plen {
            // No need for a perfect match.
            None => tbl.r_art.art_match(addr, &g),
            Some(p) => tbl.r_art.art_lookup(addr, p, &g),
        }?;

        let mut rt = an.get().an_value.load(&g);
        while let Some(r) = rt {
            if prio_is(&r, prio) {
                match gateway {
                    None => break,
                    Some(gw) if r.gateway_is(gw.bytes()) => break,
                    Some(_) => {}
                }
            }
            rt = r.get().rt_next.load(&g);
        }
        rt.map(SmrRef::arc)
    })
}

/// `rtable_match(rtableid, dst, src)`: the best match of `dst`, among a multipath chain of
/// its priority chosen by hash-threshold (RFC 2992) over `src` when given, a counted handle.
pub fn rtable_match(
    rtableid: u32,
    dst: SockaddrView<'_>,
    src: Option<&[u32]>,
) -> Option<PoolArc<Rtentry>> {
    let tbl = rtable_get(rtableid, dst.family())?;

    with_key(tbl, dst, |addr| {
        let g = smr::read();
        let an = tbl.r_art.art_match(addr, &g)?;

        let Some(mut rt) = an.get().an_value.load(&g) else {
            kpanic(format_args!("rtable_match: a node without a route"));
        };
        let prio = rt.rt_priority.get();

        // Gateway selection by Hash-Threshold (RFC 2992).
        if let Some(hash) = rt_hash(&rt, dst, src) {
            kassert!(hash <= 0xffff);

            // Only count nexthops with the same priority.
            let mut npaths = 1u32;
            let mut mrt = rt.get().rt_next.load(&g);
            while let Some(m) = mrt {
                if m.rt_priority.get() == prio {
                    npaths += 1;
                }
                mrt = m.get().rt_next.load(&g);
            }

            let threshold = (0xffff / npaths) + 1;

            // We have no protection against concurrent modification of the route list
            // attached to the node, so we won't necessarily have the same number of routes.
            // For most modifications, we'll pick a route that we wouldn't have if we only saw
            // the list before or after the change.
            let mut hash = hash;
            let mut mrt = rt;
            while hash > threshold {
                if mrt.rt_priority.get() == prio {
                    rt = mrt;
                    hash -= threshold;
                }
                match mrt.get().rt_next.load(&g) {
                    Some(m) => mrt = m,
                    None => break,
                }
            }
        }
        Some(rt.arc())
    })
}

/// `art_get(addr, plen)` with the C's `an->an_value = rt`: a new node for the prefix
/// `addr`/`plen` (`art_node_init`'s copy of its `roundup(plen, 8) / 8` bytes) whose chain is
/// the table's reference `rt`; `None` without memory.
fn art_get(addr: &[u8], plen: u32, rt: PoolArc<Rtentry>) -> Option<PoolArc<RtNode>> {
    let len = plen.div_ceil(8) as usize;
    kassert!(len <= 16);
    let mut a = [0u8; 16];
    let n = len.min(addr.len());
    a[..n].copy_from_slice(&addr[..n]);
    AN_POOL.get_shared(PR_NOWAIT, || RtNode {
        link: SmrArcLink::new(),
        an_value: SmrArcWord::with(rt),
        an_addr: a,
        an_plen: plen,
    })
}

/// `rtable_insert(rtableid, dst, mask, gateway, prio, rt)`: links route `rt` (its key and
/// prefix length set by its maker) into the table under `dst`/`mask`, at its place in a
/// multipath chain of the same prefix, the table taking a reference (`rtref`). `EAFNOSUPPORT`
/// without a table, `EINVAL` for a bad mask, `ENOMEM` without memory, `EEXIST` when the
/// prefix holds a route of the same priority that is not multipath-capable or has the same
/// gateway.
pub fn rtable_insert(
    rtableid: u32,
    dst: SockaddrView<'_>,
    mask: Option<SockaddrView<'_>>,
    gateway: Option<SockaddrView<'_>>,
    prio: u8,
    rt: &PoolArc<Rtentry>,
) -> Result<(), Errno> {
    let tbl = rtable_get(rtableid, dst.family()).ok_or(Errno::EAFNOSUPPORT)?;
    let plen = rtable_satoplen(dst.family(), mask)?;

    with_key(tbl, dst, |addr| {
        // Prepare for immediate operation if insert succeeds.
        let rt_flags = rt.rt_flags.get();
        rt.rt_flags.clear(RTF_MPATH);
        let an = art_get(addr, plen, PoolArc::clone(rt)).ok_or(Errno::ENOMEM)?;

        let w = tbl.r_lock.write();
        let g = smr::read();
        let (prev, an) = match tbl.r_art.art_insert(&w, &g, an) {
            Err(_) => return Err(Errno::ENOMEM),
            Ok(ArtInsert::Inserted) => return Ok(()),
            Ok(ArtInsert::Exists(prev, an)) => (prev, an),
        };

        // An ART node with the same destination/netmask already exists.
        let mpathok = rt_flags & RTF_MPATH != 0;
        let mut mpath = false;

        // Do not permit exactly the same dst/mask/gw pair.
        let mut mrt = prev.get().an_value.load(&g);
        while let Some(m) = mrt {
            if prio_is(&m, prio) {
                let same_gw = gateway.is_some_and(|gw| m.gateway_is(gw.bytes()));
                if !mpathok || same_gw {
                    // The node `an` is dropped with its chain: the table's reference of `rt`
                    // goes with it (the C's `rtfree` at `leave`).
                    return Err(Errno::EEXIST);
                }
                mpath = true;
            }
            mrt = m.get().rt_next.load(&g);
        }

        // The new route can be added to the list.
        if mpath {
            rt.rt_flags.set_bits(RTF_MPATH);

            let mut mrt = prev.get().an_value.load(&g);
            while let Some(m) = mrt {
                if (m.rt_priority.get() & RTP_MASK) == (prio & RTP_MASK) {
                    m.rt_flags.set_bits(RTF_MPATH);
                }
                mrt = m.get().rt_next.load(&g);
            }
        }

        // Put the newly inserted entry at the right place: the table's reference moves from
        // the unpublished node `an` (the C's `art_put(an)`) into the chain.
        let moved = an.an_value.take(&w);
        rtable_mpath_insert(&w, &g, prev.get(), rt, moved);
        Ok(())
    })
}

/// `rtable_delete(rtableid, dst, mask, rt)`: unlinks route `rt` from the table: the node
/// goes with its last route, or `rt` leaves the multipath chain (`RTF_MPATH` cleared on a
/// last remaining path of its priority). `EAFNOSUPPORT` without a table, `EINVAL` for a bad
/// mask, `ESRCH` for no such prefix; `panic(9)` for a route not on the prefix's chain.
pub fn rtable_delete(
    rtableid: u32,
    dst: SockaddrView<'_>,
    mask: Option<SockaddrView<'_>>,
    rt: &Rtentry,
) -> Result<(), Errno> {
    let tbl = rtable_get(rtableid, dst.family()).ok_or(Errno::EAFNOSUPPORT)?;
    let plen = rtable_satoplen(dst.family(), mask)?;

    with_key(tbl, dst, |addr| {
        let w = tbl.r_lock.write();
        let g = smr::read();
        let Some(an) = tbl.r_art.art_lookup(addr, plen, &g) else {
            return Err(Errno::ESRCH);
        };

        // If this is the only route in the list then we can delete the node.
        let only = an
            .get()
            .an_value
            .load(&g)
            .is_some_and(|f| core::ptr::eq(f.get(), rt))
            && !rt.rt_next.is_set();
        if only {
            match tbl.r_art.art_delete(&w, &g, addr, plen) {
                Some(oan) if SmrRef::ptr_eq(oan, an) => {}
                _ => kpanic(format_args!("art changed shape during delete")),
            }
            // The node's references were retired; the route's (the node's `an_value`) goes
            // with the node's last one, after the grace period.
        } else {
            // If other multipath route entries are still attached to this ART node we only
            // have to unlink it.
            let mut found = false;
            let mut npaths = 0u32;
            let mut nrt = None;
            let mut prt: &SmrArcWord<Rtentry> = &an.get().an_value;
            while let Some(m) = prt.load(&g) {
                if core::ptr::eq(m.get(), rt) {
                    found = true;
                    // The predecessor's word takes `rt`'s successor; `rt`'s own reference,
                    // the table's, retires from it. The walk goes on from the same word.
                    let next = rt.rt_next.take(&w);
                    drop(prt.restore(&w, next));
                    continue;
                }
                if (m.rt_priority.get() & RTP_MASK) == (rt.rt_priority.get() & RTP_MASK) {
                    npaths += 1;
                    nrt = Some(m);
                }
                prt = &m.get().rt_next;
            }
            if !found {
                kpanic(format_args!("removing non-existent route"));
            }
            if npaths == 1
                && let Some(n) = nrt
            {
                n.rt_flags.clear(RTF_MPATH);
            }
        }
        Ok(())
    })
}

/// `rtable_walk(rtableid, af, prt, func)`: `func(rt, rtableid)` on every route of the table,
/// in the tree's order, with the table's lock let go around each call (the iterator keeps
/// the topology, a counted handle each route). The first `Err` stops the walk and is
/// answered, the route it stopped at given to `prt` when asked for. `EAFNOSUPPORT` without
/// a table.
pub fn rtable_walk(
    rtableid: u32,
    af: SaFamily,
    mut prt: Option<&mut Option<PoolArc<Rtentry>>>,
    mut func: impl FnMut(&Rtentry, u32) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let tbl = rtable_get(rtableid, af).ok_or(Errno::EAFNOSUPPORT)?;
    let mut ai = ArtIter::new();
    let mut opened = false;

    let mut w = tbl.r_lock.write();
    loop {
        // ART nodes have a list of rtentries. The iterator holds references to the topology
        // so it won't change, but not the nodes or their routes.
        let first = {
            let g = smr::read();
            let an = if opened {
                tbl.r_art.art_iter_next(&mut ai, &w, &g)
            } else {
                opened = true;
                tbl.r_art.art_iter_open(&mut ai, &w, &g)
            };
            match an {
                None => break,
                Some(an) => an.get().an_value.load(&g).map(SmrRef::arc),
            }
        };
        let Some(first) = first else { continue };

        let stopped = w.unlocked(|| {
            let mut rt = first;
            loop {
                // Get ready for the next entry.
                let nrt = {
                    let g = smr::read();
                    rt.rt_next.load(&g).map(SmrRef::arc)
                };
                if let Err(e) = func(&rt, rtableid) {
                    if let Some(p) = prt.as_deref_mut() {
                        *p = Some(rt);
                    }
                    return Some(e);
                }
                match nrt {
                    Some(n) => rt = n,
                    None => return None,
                }
            }
        });
        if let Some(e) = stopped {
            let g = smr::read();
            tbl.r_art.art_iter_close(&mut ai, &w, &g);
            return Err(e);
        }
    }
    Ok(())
}

/// `rtable_read(rtableid, af, func)`: `func(rt, rtableid)` on every route of the table, in
/// the tree's order, the table's lock held throughout (the callback may sleep, as
/// `sysctl_dumpentry`'s copyout does: each route is lent through a counted handle taken in a
/// read section of its own step, none held across the call); the first `Err` stops the walk
/// and is answered. `EAFNOSUPPORT` without a table.
pub fn rtable_read(
    rtableid: u32,
    af: SaFamily,
    mut func: impl FnMut(&Rtentry, u32) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let tbl = rtable_get(rtableid, af).ok_or(Errno::EAFNOSUPPORT)?;
    let mut ai = ArtIter::new();
    let mut opened = false;
    let w = tbl.r_lock.write();
    loop {
        let mut rt = {
            let g = smr::read();
            let an = if opened {
                tbl.r_art.art_iter_next(&mut ai, &w, &g)
            } else {
                opened = true;
                tbl.r_art.art_iter_open(&mut ai, &w, &g)
            };
            match an {
                None => break,
                Some(an) => an.get().an_value.load(&g).map(SmrRef::arc),
            }
        };
        while let Some(r) = rt {
            if let Err(e) = func(&r, rtableid) {
                let g = smr::read();
                tbl.r_art.art_iter_close(&mut ai, &w, &g);
                return Err(e);
            }
            rt = {
                let g = smr::read();
                r.rt_next.load(&g).map(SmrRef::arc)
            };
        }
    }
    Ok(())
}

/// `rtable_iterate(rt0)`: the next route of `rt0`'s multipath chain, a counted handle;
/// `rt0`'s handle given back.
pub fn rtable_iterate(rt0: PoolArc<Rtentry>) -> Option<PoolArc<Rtentry>> {
    let rt = {
        let g = smr::read();
        rt0.rt_next.load(&g).map(SmrRef::arc)
    };
    // rtfree(rt0).
    drop(rt0);
    rt
}

/// `rtable_mpath_capable(rtableid, af)`: the ART tables support multipath, always.
pub fn rtable_mpath_capable(_rtableid: u32, _af: SaFamily) -> bool {
    true
}

/// `rtable_mpath_reprio(rtableid, dst, plen, prio, rt)`: gives route `rt` of the prefix
/// `dst`/`plen` priority `prio`, moving it to its new place in its chain (`Err(EAGAIN)`
/// then, the walkers' signal that the chain changed); `ESRCH` for no such prefix,
/// `EAFNOSUPPORT` without a table.
pub fn rtable_mpath_reprio(
    rtableid: u32,
    dst: SockaddrView<'_>,
    plen: u32,
    prio: u8,
    rt: &Rtentry,
) -> Result<(), Errno> {
    let tbl = rtable_get(rtableid, dst.family()).ok_or(Errno::EAFNOSUPPORT)?;

    with_key(tbl, dst, |addr| {
        let w = tbl.r_lock.write();
        let g = smr::read();
        let Some(an) = tbl.r_art.art_lookup(addr, plen, &g) else {
            return Err(Errno::ESRCH);
        };
        let only = an
            .get()
            .an_value
            .load(&g)
            .is_some_and(|f| core::ptr::eq(f.get(), rt))
            && !rt.rt_next.is_set();
        if only {
            // If there's only one entry on the list do not go through an insert/remove
            // cycle. This is done to guarantee that the node's list is never empty when a
            // node is in the tree.
            rt.rt_priority.set(prio);
            return Ok(());
        }

        let mut prt: &SmrArcWord<Rtentry> = &an.get().an_value;
        loop {
            match prt.load(&g) {
                Some(m) if core::ptr::eq(m.get(), rt) => break,
                Some(m) => prt = &m.get().rt_next,
                None => kpanic(format_args!(
                    "rtable_mpath_reprio: the route is not on its chain"
                )),
            }
        }

        // `rt` leaves its place: its successor's reference moves into its predecessor's
        // word, and its own reference, out of that word, moves to its new place.
        let next = rt.rt_next.take(&w);
        let own = prt.restore(&w, next);
        rt.rt_priority.set(prio);
        rtable_mpath_insert(&w, &g, an.get(), rt, own);
        Err(Errno::EAGAIN)
    })
}

/// `rtable_mpath_insert(an, rt)`: links route `rt` into node `an`'s chain before the first
/// route of a higher priority value (at the end otherwise), its reference `own` (one just
/// unpublished from another word, moved without a retirement) published there; the word it
/// takes the place of gives its value to `rt`'s `rt_next`.
fn rtable_mpath_insert<W: Exclusive + ?Sized>(
    w: &W,
    g: &SmrRead,
    an: &RtNode,
    rt: &Rtentry,
    own: Option<hal::smr::SmrRetiredArc<Rtentry>>,
) {
    let prio = rt.rt_priority.get();

    // Iterate until we find the route to be placed after `rt`.
    let mut prt: &SmrArcWord<Rtentry> = &an.an_value;
    while let Some(m) = prt.load(g) {
        if m.rt_priority.get() > prio {
            break;
        }
        prt = &m.get().rt_next;
    }

    let mrt = prt.restore(w, own);
    drop(rt.rt_next.restore(w, mrt));
}

/// `rtable_satoplen(af, mask)`: the prefix length of `mask` for family `af`: the family's
/// maximum for a host route (no mask), 0 for a default route (`sa_len` 0, or nothing past
/// the trailing zeros); `EINVAL` for a family without a routing table, a mask shorter than
/// the key's offset, longer than the maximum or not contiguous.
pub fn rtable_satoplen(af: SaFamily, mask: Option<SockaddrView<'_>>) -> Result<u32, Errno> {
    let dp = rt_dom(af).ok_or(Errno::EINVAL)?;

    // Host route.
    let Some(mask) = mask else {
        return Ok(dp.maxplen);
    };

    let mlen = usize::from(mask.len());

    // Default route.
    if mlen == 0 {
        return Ok(0);
    }

    let ap = dp.rtoffset as usize;
    if ap > mlen {
        return Err(Errno::EINVAL);
    }
    let bytes = mask.bytes();
    let mut m = &bytes[ap.min(bytes.len())..mlen.min(bytes.len())];

    // Trim trailing zeroes.
    while let [rest @ .., 0] = m {
        m = rest;
    }

    if m.is_empty() {
        return Ok(0);
    }

    // "Beauty" adapted from sbin/route/show.c ...
    let mut plen = 0u32;
    let mut used = 0;
    for &b in m {
        used += 1;
        match b {
            0xff => {
                plen += 8;
                continue;
            }
            0xfe => plen += 7,
            0xfc => plen += 6,
            0xf8 => plen += 5,
            0xf0 => plen += 4,
            0xe0 => plen += 3,
            0xc0 => plen += 2,
            0x80 => plen += 1,
            // Non contiguous mask.
            _ => return Err(Errno::EINVAL),
        }
        break;
    }

    if plen > dp.maxplen || used != m.len() {
        return Err(Errno::EINVAL);
    }

    Ok(plen)
}

/// Forgets the tables, maps and routing domains (never frees them), for the host tests
/// that load fresh kernel memory: an earlier test's maps and tables are dropped from
/// `afmap`, never read again.
#[cfg(any(test, feature = "doubles"))]
pub fn rtable_test_reset() {
    klock::kernel_lock();
    {
        let mut w = klock::witness();
        for slot in &AFMAP {
            if let Some(old) = slot.store(&mut w, None) {
                // The old map's memory is an earlier test's: forgotten, never released.
                core::mem::forget(old);
            }
        }
    }
    klock::kernel_unlock();
    RT_NDOMS.store(0, Ordering::Release);
    RTMAP_LIMIT.store(0, Ordering::Release);
    AF2IDX_MAX.store(0, Ordering::Relaxed);
    for a in &AF2IDX {
        a.store(0, Ordering::Relaxed);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the routing tables: the masks' prefix lengths, the tables of each id and
    // their growth, the routing domain map, insertion with the multipath rules, lookups and
    // matches (hash-threshold), deletion from a node and from a chain, the priority changes,
    // the walks and the source addresses.

    use std::vec;
    use std::vec::Vec;

    use hal::sys::net_route::{RTF_GATEWAY, RTF_HOST, RTF_UP, RTP_DOWN, RTP_STATIC};
    use hal::sys::socket::{AF_INET, AF_INET6, AF_UNIX};

    use super::*;
    use crate::net::route::tests::{ROUTE_DOMS, entry, grace, sin, sin6, testroutes_setup};

    fn v(b: &[u8]) -> SockaddrView<'_> {
        SockaddrView::new(b).expect("a view")
    }

    fn plen(mask: Option<[u8; 16]>) -> Result<u32, Errno> {
        match mask {
            Some(m) => rtable_satoplen(AF_INET, Some(SockaddrView::new(&m).expect("a view"))),
            None => rtable_satoplen(AF_INET, None),
        }
    }

    fn mask4(m: [u8; 4], len: u8) -> [u8; 16] {
        let mut s = sin(m);
        s[0] = len;
        s
    }

    #[test]
    fn masks_become_prefix_lengths() {
        let _g = testroutes_setup();
        assert_eq!(plen(None), Ok(32), "host route");
        assert_eq!(
            plen(Some(mask4([0; 4], 0))),
            Ok(0),
            "sa_len 0: default route"
        );
        assert_eq!(plen(Some(mask4([0; 4], 16))), Ok(0));
        assert_eq!(plen(Some(mask4([255, 255, 255, 0], 16))), Ok(24));
        assert_eq!(
            plen(Some(mask4([255, 255, 255, 0], 7))),
            Ok(24),
            "trimmed by in_socktrim"
        );
        assert_eq!(plen(Some(mask4([255, 255, 254, 0], 16))), Ok(23));
        assert_eq!(plen(Some(mask4([255, 128, 0, 0], 16))), Ok(9));
        assert_eq!(plen(Some(mask4([255, 255, 255, 255], 16))), Ok(32));
        assert_eq!(
            plen(Some(mask4([255, 0, 255, 0], 16))),
            Err(Errno::EINVAL),
            "non contiguous"
        );
        assert_eq!(plen(Some(mask4([255, 0x7f, 0, 0], 16))), Err(Errno::EINVAL));
        assert_eq!(
            plen(Some(mask4([255, 0, 0, 0], 3))),
            Err(Errno::EINVAL),
            "too short"
        );
        assert_eq!(rtable_satoplen(AF_UNIX, None), Err(Errno::EINVAL));
        // IPv6: the key at offset 8, 128 bits at most.
        let mut m = sin6([0xff; 16]);
        assert_eq!(
            rtable_satoplen(AF_INET6, Some(SockaddrView::new(&m).expect("a view"))),
            Ok(128)
        );
        m[16..24].fill(0);
        assert_eq!(
            rtable_satoplen(AF_INET6, Some(SockaddrView::new(&m).expect("a view"))),
            Ok(64)
        );
    }

    #[test]
    fn table_zero_exists_in_routing_domain_zero() {
        let _g = testroutes_setup();
        assert!(rtable_exists(0));
        assert!(!rtable_exists(1));
        assert!(rtable_get(0, AF_INET).is_some());
        assert!(rtable_get(0, AF_INET6).is_some());
        assert!(rtable_get(0, AF_UNIX).is_none());
        assert_eq!(rtable_l2(0), 0);
        assert!(rtable_empty(0));

        // A new table, in rdomain 0 with no loopback until rtable_l2set; the maps grow to
        // its id and every older id keeps its table.
        let t0 = rtable_get(0, AF_INET).expect("table 0");
        rtable_add(3).expect("rtable_add");
        assert_eq!(RTMAP_LIMIT.load(Ordering::Relaxed), 4);
        assert!(rtable_exists(3) && !rtable_exists(2));
        assert!(core::ptr::eq(rtable_get(0, AF_INET).expect("table 0"), t0));
        assert!(rtable_empty(3));
        assert_eq!(rtable_l2(3), 0);
        klock::kernel_lock();
        rtable_l2set(3, 3, 9);
        rtable_l2set(2, 0, 1); // id 2 has no table: nothing
        klock::kernel_unlock();
        assert_eq!(rtable_l2(3), 3);
        assert_eq!(rtable_loindex(3), 9);
        assert_eq!(rtable_l2(2), 0);
        for id in 4..8 {
            rtable_add(id).expect("rtable_add");
        }
        assert_eq!(RTMAP_LIMIT.load(Ordering::Relaxed), 8);
        assert_eq!(
            rtable_l2(3),
            3,
            "the domain map's values survive its growth"
        );
        assert!(rtable_add(3).is_ok(), "an existing id");
        assert_eq!(rtable_add(RT_TABLEID_MAX + 1), Err(Errno::EINVAL));
        grace();
    }

    #[test]
    fn insert_lookup_match_and_delete() {
        let _g = testroutes_setup();
        let gw = sin([10, 0, 2, 2]);
        let dflt = entry(&sin([0; 4]), 0, Some(&gw), RTF_GATEWAY, RTP_STATIC);
        let net = entry(&sin([10, 0, 2, 0]), 24, Some(&gw), 0, RTP_STATIC);
        let host = entry(&sin([10, 0, 2, 2]), 32, None, RTF_HOST, RTP_STATIC);
        let m0 = mask4([0; 4], 0);
        let m24 = mask4([255, 255, 255, 0], 16);
        for (rt, d, m) in [
            (&dflt, sin([0; 4]), Some(m0)),
            (&net, sin([10, 0, 2, 0]), Some(m24)),
            (&host, sin([10, 0, 2, 2]), None),
        ] {
            rtable_insert(
                0,
                v(&d),
                m.as_ref().map(|m| v(m)),
                Some(v(&gw)),
                RTP_STATIC,
                rt,
            )
            .expect("insert");
            assert_eq!(PoolArc::count(rt), 2, "the table's reference");
        }
        assert!(!rtable_empty(0));
        let found = |d: [u8; 4]| rtable_match(0, v(&sin(d)), None).map(|r| r.rt_plen());
        assert_eq!(found([10, 0, 2, 2]), Some(32));
        assert_eq!(found([10, 0, 2, 9]), Some(24));
        assert_eq!(found([8, 8, 8, 8]), Some(0));

        // A perfect lookup finds the /24, by gateway too; another gateway or a /25 nothing.
        let n24 = rtable_lookup(0, v(&sin([10, 0, 2, 0])), Some(v(&m24)), None, RTP_ANY)
            .expect("the /24");
        assert!(PoolArc::ptr_eq(&n24, &net));
        assert_eq!(PoolArc::count(&net), 3, "the table's and the lookup's");
        drop(n24);
        assert!(
            rtable_lookup(
                0,
                v(&sin([10, 0, 2, 0])),
                Some(v(&m24)),
                Some(v(&gw)),
                RTP_ANY
            )
            .is_some()
        );
        let other = sin([10, 0, 2, 3]);
        assert!(
            rtable_lookup(
                0,
                v(&sin([10, 0, 2, 0])),
                Some(v(&m24)),
                Some(v(&other)),
                RTP_ANY
            )
            .is_none()
        );
        let m25 = mask4([255, 255, 255, 128], 16);
        assert!(rtable_lookup(0, v(&sin([10, 0, 2, 0])), Some(v(&m25)), None, RTP_ANY).is_none());

        // The same prefix and priority again: EEXIST, the table takes no reference.
        let dup = entry(&sin([10, 0, 2, 0]), 24, Some(&other), 0, RTP_STATIC);
        assert_eq!(
            rtable_insert(
                0,
                v(&sin([10, 0, 2, 0])),
                Some(v(&m24)),
                Some(v(&other)),
                RTP_STATIC,
                &dup
            ),
            Err(Errno::EEXIST)
        );
        grace();
        assert_eq!(PoolArc::count(&dup), 1);

        // The walk sees the three, the default route first.
        let mut seen = Vec::new();
        rtable_walk(0, AF_INET, None, |rt, id| {
            assert_eq!(id, 0);
            seen.push(rt.rt_plen());
            Ok(())
        })
        .expect("walk");
        assert_eq!(seen, [0, 24, 32]);
        // rtable_read stops at the first error.
        let mut n = 0;
        assert_eq!(
            rtable_read(0, AF_INET, |_, _| {
                n += 1;
                Err(Errno::EEXIST)
            }),
            Err(Errno::EEXIST)
        );
        assert_eq!(n, 1);
        // Its callback runs outside every read section (it may sleep: `smr::idle` panics
        // inside one).
        let mut n = 0;
        rtable_read(0, AF_INET, |_, _| {
            smr::idle();
            n += 1;
            Ok(())
        })
        .expect("read");
        assert_eq!(n, 3);
        // rtable_walk gives the route it stopped at.
        let mut at = None;
        assert_eq!(
            rtable_walk(0, AF_INET, Some(&mut at), |rt, _| if rt.rt_plen() == 24 {
                Err(Errno::EEXIST)
            } else {
                Ok(())
            }),
            Err(Errno::EEXIST)
        );
        assert!(at.as_ref().is_some_and(|r| PoolArc::ptr_eq(r, &net)));
        drop(at);

        assert_eq!(
            rtable_delete(0, v(&sin([10, 0, 3, 0])), Some(v(&m24)), &net),
            Err(Errno::ESRCH)
        );
        for (rt, d, m) in [
            (&host, sin([10, 0, 2, 2]), None),
            (&net, sin([10, 0, 2, 0]), Some(m24)),
            (&dflt, sin([0; 4]), Some(m0)),
        ] {
            rtable_delete(0, v(&d), m.as_ref().map(|m| v(m)), rt).expect("delete");
        }
        assert!(rtable_empty(0));
        assert_eq!(found([10, 0, 2, 2]), None);
        grace();
        for rt in [&dflt, &net, &host] {
            assert_eq!(PoolArc::count(rt), 1, "the table's reference released");
        }
    }

    #[test]
    fn multipath_chains_keep_their_priority_order() {
        let _g = testroutes_setup();
        let d = sin([192, 168, 0, 0]);
        let m16 = mask4([255, 255, 0, 0], 16);
        let gws = [sin([10, 0, 0, 1]), sin([10, 0, 0, 2]), sin([10, 0, 0, 3])];
        let a = entry(&d, 16, Some(&gws[0]), RTF_GATEWAY | RTF_MPATH, 8);
        let b = entry(&d, 16, Some(&gws[1]), RTF_GATEWAY | RTF_MPATH, 8);
        let c = entry(&d, 16, Some(&gws[2]), RTF_GATEWAY | RTF_MPATH, 4);
        for (rt, gw) in [(&a, &gws[0]), (&b, &gws[1]), (&c, &gws[2])] {
            let prio = rt.rt_priority.get();
            rtable_insert(0, v(&d), Some(v(&m16)), Some(v(gw)), prio, rt).expect("insert");
        }
        let chain = || {
            let g = smr::read();
            let tbl = rtable_get(0, AF_INET).expect("table");
            let an = with_key(tbl, v(&d), |k| {
                tbl.r_art.art_lookup(k, 16, &g).map(|n| n.arc())
            })
            .expect("the node");
            let mut out = Vec::new();
            let mut r = an.an_value.load(&g);
            while let Some(x) = r {
                out.push(x.with_gateway(|g| g.expect("a gateway")[4..8].to_vec()));
                r = x.get().rt_next.load(&g);
            }
            out
        };
        // c (4) first, then a and b (8) in their order of insertion.
        assert_eq!(
            chain(),
            [vec![10, 0, 0, 3], vec![10, 0, 0, 1], vec![10, 0, 0, 2]]
        );
        assert!(a.rt_flags.get() & RTF_MPATH != 0 && b.rt_flags.get() & RTF_MPATH != 0);
        assert_eq!(c.rt_flags.get() & RTF_MPATH, 0, "alone at its priority");

        // The same gateway again is refused; a route without RTF_MPATH too.
        let a2 = entry(&d, 16, Some(&gws[0]), RTF_GATEWAY | RTF_MPATH, 8);
        assert_eq!(
            rtable_insert(0, v(&d), Some(v(&m16)), Some(v(&gws[0])), 8, &a2),
            Err(Errno::EEXIST)
        );
        let lone = entry(&d, 16, Some(&sin([10, 0, 0, 9])), RTF_GATEWAY, 8);
        assert_eq!(
            rtable_insert(
                0,
                v(&d),
                Some(v(&m16)),
                Some(v(&sin([10, 0, 0, 9]))),
                8,
                &lone
            ),
            Err(Errno::EEXIST)
        );

        // A priority change moves the route: EAGAIN, the chain reordered, no reference
        // taken or lost on the way.
        assert_eq!(rtable_mpath_reprio(0, v(&d), 16, 2, &b), Err(Errno::EAGAIN));
        assert_eq!(
            chain(),
            [vec![10, 0, 0, 2], vec![10, 0, 0, 3], vec![10, 0, 0, 1]]
        );
        assert_eq!(PoolArc::count(&b), 2);
        assert_eq!(
            rtable_mpath_reprio(0, v(&d), 16, 8 | RTP_DOWN, &b),
            Err(Errno::EAGAIN)
        );
        assert_eq!(
            chain(),
            [vec![10, 0, 0, 3], vec![10, 0, 0, 1], vec![10, 0, 0, 2]]
        );

        // rtable_iterate walks the chain.
        let first = rtable_match(0, v(&sin([192, 168, 1, 1])), None).expect("a match");
        assert!(PoolArc::ptr_eq(&first, &c));
        let second = rtable_iterate(first).expect("the next");
        assert!(PoolArc::ptr_eq(&second, &a));

        // Unlinking from the middle and the head; the last path of priority 8 loses
        // RTF_MPATH.
        rtable_delete(0, v(&d), Some(v(&m16)), &a).expect("delete a");
        assert_eq!(chain(), [vec![10, 0, 0, 3], vec![10, 0, 0, 2]]);
        assert_eq!(b.rt_flags.get() & RTF_MPATH, 0);
        rtable_delete(0, v(&d), Some(v(&m16)), &c).expect("delete c");
        assert_eq!(chain(), [vec![10, 0, 0, 2]]);
        rtable_delete(0, v(&d), Some(v(&m16)), &b).expect("delete b");
        assert!(rtable_empty(0));
        drop(second);
        grace();
        for rt in [&a, &b, &c, &a2, &lone] {
            assert_eq!(PoolArc::count(rt), 1);
        }
    }

    #[test]
    fn a_walk_whose_callback_deletes() {
        let _g = testroutes_setup();
        let m32 = None;
        let rts: Vec<_> = (1..=5u8)
            .map(|i| {
                let d = sin([10, 0, 0, i]);
                let rt = entry(&d, 32, None, RTF_HOST, RTP_STATIC);
                rtable_insert(0, v(&d), m32, None, RTP_STATIC, &rt).expect("insert");
                rt
            })
            .collect();
        // Every second route deleted from inside the walk, with the table's lock let go.
        let mut seen = 0;
        rtable_walk(0, AF_INET, None, |rt, id| {
            seen += 1;
            if rt.dest()[7] % 2 == 0 {
                let d = SaCopy::of(rt.dest_view()).expect("a key");
                rtable_delete(id, d.view(), None, rt)?;
            }
            Ok(())
        })
        .expect("walk");
        assert_eq!(seen, 5);
        let mut left = Vec::new();
        rtable_read(0, AF_INET, |rt, _| {
            left.push(rt.dest()[7]);
            Ok(())
        })
        .expect("read");
        assert_eq!(left, [1, 3, 5]);
        for rt in &rts {
            if rt.dest()[7] % 2 == 1 {
                rtable_delete(0, rt.dest_view(), None, rt).expect("delete");
            }
        }
        assert!(rtable_empty(0));
        grace();
        let _ = ROUTE_DOMS;
    }

    #[test]
    fn multipath_selection_by_hash_threshold_is_stable() {
        let _g = testroutes_setup();
        let d = sin([0; 4]);
        let m0 = mask4([0; 4], 0);
        let gws: Vec<_> = (1..=3u8).map(|i| sin([10, 0, 0, i])).collect();
        let rts: Vec<_> = gws
            .iter()
            .map(|gw| {
                let rt = entry(&d, 0, Some(gw), RTF_UP | RTF_MPATH, RTP_STATIC);
                rtable_insert(0, v(&d), Some(v(&m0)), Some(v(gw)), RTP_STATIC, &rt)
                    .expect("insert");
                rt
            })
            .collect();
        crate::net::route::tests::set_multipath(true);
        let pick = |dst: [u8; 4], src: u32| {
            let r = rtable_match(0, v(&sin(dst)), Some(&[src])).expect("a route");
            rts.iter()
                .position(|x| PoolArc::ptr_eq(x, &r))
                .expect("one of them")
        };
        // The same flow always takes the same path, and flows spread over the paths as the
        // C's loop spreads them: its first round picks the first path again, so a hash
        // above the threshold moves one path less than it is thresholds high and the last
        // of the three is never taken (rtable.c's own selection, kept).
        let mut used = [0; 3];
        for s in 0..64u32 {
            let p = pick([8, 8, 8, 8], s.wrapping_mul(0x9e37_79b9));
            assert_eq!(p, pick([8, 8, 8, 8], s.wrapping_mul(0x9e37_79b9)));
            used[p] += 1;
        }
        assert!(used[0] > 0 && used[1] > 0 && used[2] == 0, "{used:?}");
        // Without a source, or with multipath off, the first path.
        assert_eq!(
            rtable_match(0, v(&sin([8, 8, 8, 8])), None).map(|r| PoolArc::ptr_eq(&r, &rts[0])),
            Some(true)
        );
        crate::net::route::tests::set_multipath(false);
        assert_eq!(pick([8, 8, 8, 8], 12345), 0);
        for rt in &rts {
            rtable_delete(0, v(&d), Some(v(&m0)), rt).expect("delete");
            rt.rt_flags.clear(RTF_UP);
        }
        grace();
    }

    #[test]
    fn the_source_address_is_set_read_and_cleared() {
        let _g = testroutes_setup();
        assert!(rtable_getsource(0, AF_INET).is_none());
        let a = sin([10, 0, 2, 15]);
        rtable_setsource(0, AF_INET, Some(v(&a))).expect("set");
        assert_eq!(
            rtable_getsource(0, AF_INET).map(|s| s.bytes().to_vec()),
            Some(a.to_vec())
        );
        rtable_clearsource(0, v(&sin([10, 0, 2, 16])));
        assert!(
            rtable_getsource(0, AF_INET).is_some(),
            "another address leaves it"
        );
        rtable_clearsource(0, v(&a));
        assert!(rtable_getsource(0, AF_INET).is_none());
        assert_eq!(rtable_setsource(9, AF_INET, None), Err(Errno::EAFNOSUPPORT));
    }
}
/* </TESTS> */
