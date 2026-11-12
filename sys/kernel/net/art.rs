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
 * Copyright (c) 2015 Martin Pieuchot
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
 * Copyright (c) 2015 Martin Pieuchot
 * Copyright (c) 2001 Yoichi Hariguchi
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
//! The Allotment Routing Table (ART): `<net/art.h>` and `net/art.c`, generic over its node
//! (the C's embeddable `struct art_node` with its `void *an_value`): the routing tables'
//! (`kernel::net::rtable`'s `RtNode`) and, at N8, wg(4)'s allowed IPs.
//!
//! Upstream: sys/net/art.h @ 3ce1f3f79392
//! Upstream: sys/net/art.c @ 3ce1f3f79392
//! LZ: sys/net/art.rs@3c62ede68802
//!
//! Yoichi Hariguchi's paper can be found at <http://www.hariguchi.org/art/art.pdf>. This
//! implementation is tweaked to minimise pointer traversal during lookups by only accessing
//! the heaps: it never follows a pointer to or through an [`ArtTable`]. A heap is an array of
//! words whose meaning depends on where they are: most name a node (a leaf entry) or the next
//! heap to traverse, the two kinds told apart by the word's low bit (`hal::smr`'s
//! [`SmrTaggedWord`]). The first two entries are not used by the search: `heap[0]` names the
//! heap's [`ArtTable`], and `heap[1]` holds the node that would be in the parent heap if this
//! table were not using its slot, the default entry (route) of the table. Nodes hold the
//! exact prefix they represent (address and prefix length) and the data associated with it:
//!
//! ```text
//!   Art --art_root--> heap --heap[0]--> ArtTable --at_parent--> None
//!                      |   <--at_heap--
//!                      | heap entry
//!                      v
//!                     heap --heap[0]--> ArtTable --at_parent--> (above)
//!                      | node entry
//!                      v
//!                     node (an ArtItem) --> "user" data
//! ```
//!
//! Locking: modification ([`Art::art_insert`], [`Art::art_delete`]) and iteration
//! ([`Art::art_iter_next`], ...) are serialised by the caller (its lock's witness,
//! [`Exclusive`]). Lookups ([`Art::art_match`], [`Art::art_lookup`]) run within an SMR read
//! section. Iteration manipulates the reference counts of the tables it traverses (their
//! topology) and keeps them until it runs out of entries, so a caller may release its lock
//! between [`Art::art_iter_open`] and [`Art::art_iter_next`]; [`Art::art_iter_close`] drops
//! them early. As the C's, the iterator holds no reference of the nodes it answers: each is
//! lent for the step's read section (the caller takes its own reference of what it keeps).
//!
//! ## Deviations
//! - `art_table_get` takes the heap from its pool before the table, which holds it for its
//!   whole life; the C takes the table first. The two items are taken or neither is, so only
//!   a pool's failure count under memory pressure tells the order.
//! - The first insert into an empty table builds its root heap's value before writing it into
//!   its pool item (`TypedPool` has no zeroed form): an IPv4 table's 8-bit root heap is 4 KiB
//!   of the caller's stack for that call, once per table and family (`art_table_get`'s frame
//!   is 4.2 KiB in the dev profile, measured on the host build, one copy of the value).
//! - A tree dropped with nodes in it keeps its tables and heaps (the table and heap handles
//!   form cycles that only `art_table_put` cuts): the C never frees a routing table, and
//!   wg(4) deletes its entries before it frees its trees.
//! - `art_alloc` is [`Art::new`]: the tree is embedded in its owner (the routing table's
//!   `M_RTABLE` block), so `malloc(M_RTABLE)`'s bytes are the C's minus `struct art`'s 32.
//! - Addresses are byte slices holding at least the address length's bytes; the C takes
//!   `const void *` into a socket address.
//! - `art_table_get`'s `j == -1` for the root is `u32::MAX` (the C's `unsigned int` -1).
//!
//! ## Redesign
//! - The heap words (`art_heap_entry`, a bare `uintptr_t` the C changes with
//!   `SMR_PTR_SET_LOCKED`) and `art_root` are `hal::smr`'s counted words: every word that
//!   names a node or a heap holds a reference of it (a node's home index and each fringe copy
//!   [`art_allot`] makes, `heap[0]` its table, a parent's slot its child's heap), so a reader's
//!   reference is sound by SMR alone. `art_allot` clones the new node's handle into each word
//!   it changes, and the old node's references retire there (one link per item counts them:
//!   an allot over a stride's fringe retires up to 256 references of one node, one release).
//!   A table owns its heap ([`ArtTable`]'s `at_heap`, immutable); `heap[0]` counts the table
//!   and `at_parent` the parent table; the cycles they make are cut where the C unlinks
//!   ([`Art::art_table_put`] retires the heap's `heap[0]` and the parent's slot).
//! - `art_table_gc`, `art_gc`, their lists, mutexes and `systqmp` tasks, and `an_gc` (the
//!   union arm of `an_addr`) are gone: a retired reference is released by the SMR thread after
//!   a grace period, which is what their `smr_barrier()` then `pool_put` did; the item goes
//!   back to its pool when its last reference does.
//! - The heaps are typed by their stride: [`Heap4`] (32 words, pool `"art_heap4"`) and
//!   [`Heap8`] (512, `"art_heap8"`), so a heap is a slice whose bound is its own; the C reads
//!   a heap's size from its table, which the lookups never touch. Only a root heap may be
//!   8-bit (the C's three level tables: IPv4's 8 then 4s, IPv6's and MPLS's 4s; [`Art::new`]
//!   checks it), so `art_root` tags its two kinds ([`Heap8`] untagged, [`Heap4`] tagged) and a
//!   heap's entries name only [`Heap4`]s.
//! - The tree is generic over its node ([`ArtItem`]: its address, prefix length and the
//!   tables' and heaps' pools of its kind, [`ArtPools`]); the C's pools are shared by every
//!   user, so a second kind of node (wg(4)'s, N8) gets pools of its own under the same names.
//!   The node pool (`"art_node"`), `art_get`, `art_node_init` and `art_put` are the node
//!   owner's (`kernel::net::rtable`).
//! - `art_allot`, which the C writes as Knuth's `goto` expansion of a double recursion, is the
//!   recursion: depth is at most a table's stride (8). It descends into a non-fringe slot only
//!   when the slot holds the replaced entry, as the expansion does, and stores a slot after its
//!   children, as the expansion's `moveup` does.
//! - `at_refcnt` is an `AtomicU32` changed only under the caller's serialisation (relaxed).
//! - `struct art_iter`'s `ai_table` pointer is a counted handle of the table the walk is in
//!   (its parents are held by its `at_parent`), since the caller lets its lock and its read
//!   section go between two steps: one atomic per table entered and left, beside the C's
//!   `at_refcnt`. The node a step answers is lent for that step's section, tied to the
//!   iterator's borrow, so it cannot be kept across the next step (the C's own rule: "the
//!   iterator does not hold a reference to the art_node").
//! - Lookups and changes read the heap words through `SmrTaggedWord::load`, inside a read
//!   section the caller opens (the C's writers read with `SMR_PTR_GET_LOCKED`, without one):
//!   the counted words lend references only for a section. A writer's section is one
//!   per-CPU count pair around its call, no lock or spl transition.
//! - Costs against the C. Lookups (`art_match`, `art_lookup`): the C's loads, one per level
//!   and its default, each an `Acquire` load and a tag test, no atomic write. `art_insert`,
//!   `art_delete`: one atomic per heap word written (the reference it gains) and one per word
//!   overwritten (the retirement's compare-and-swap of the item's state), up to a stride's
//!   fringe (256 on the IPv4 root) in an `art_allot`, where the C stores bare words: a
//!   control path (route changes), named for N12. The iteration: one atomic pair per table
//!   entered (its handle), none per node.

use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

use hal::intr::IPL_SOFTNET;
use hal::pool::{PR_NOWAIT, PoolArc, TypedPool};
use hal::rwlock::Exclusive;
use hal::smr::{SmrArcLink, SmrArcWord, SmrCounted, SmrRead, SmrRef, SmrTaggedWord, Tag};
use hal::sys::errno::Errno;

use crate::assert::{kassert, kpanic};

/// `ART_HEAP_IDX_TABLE`: the slot of a heap that names its table (`ArtHeap`'s own word).
pub const ART_HEAP_IDX_TABLE: usize = 0;
/// `ART_HEAP_IDX_DEFAULT`: the slot of a heap that holds its table's default entry.
pub const ART_HEAP_IDX_DEFAULT: usize = 1;

/// `AT_HEAPSIZE(bits)` in words (the C's is in bytes): the heap of a table of stride `bits`.
pub const fn at_heapsize(bits: u32) -> usize {
    1 << (bits + 1)
}

/// The words of a 4-bit heap.
const HEAP4_WORDS: usize = at_heapsize(4);
/// The words of an 8-bit heap.
const HEAP8_WORDS: usize = at_heapsize(8);

/// Stride lengths for IPv4 addresses.
const ART_PLEN32_LEVELS: [u32; 7] = [8, 4, 4, 4, 4, 4, 4];
/// Stride lengths for IPv6 addresses.
const ART_PLEN128_LEVELS: [u32; 32] = [4; 32];
/// Stride lengths for MPLS labels.
const ART_PLEN20_LEVELS: [u32; 5] = [4; 5];

/// A heap entry (`art_heap_entry`): a node, or (tagged) a child's 4-bit heap.
pub type HeapWord<N> = SmrTaggedWord<N, Heap4<N>>;
/// A 4-bit table's heap (pool `"art_heap4"`).
pub type Heap4<N> = ArtHeap<N, HEAP4_WORDS>;
/// An 8-bit table's heap (pool `"art_heap8"`): only a root.
pub type Heap8<N> = ArtHeap<N, HEAP8_WORDS>;

/// A heap: `heap[0]` names the table (`table`), `heap[1]` is the default entry and the rest
/// are the table's slots (`words[0]` unused, so an index is the C's).
pub struct ArtHeap<N: ArtItem, const W: usize> {
    /// The retirement link of the heap's references.
    link: SmrArcLink,
    /// `heap[ART_HEAP_IDX_TABLE]`: the table, cleared when the table is put.
    table: SmrArcWord<ArtTable<N>>,
    /// `heap[1..W]`.
    words: [HeapWord<N>; W],
}

impl<N: ArtItem, const W: usize> ArtHeap<N, W> {
    /// An empty heap (the C's `PR_ZERO` item).
    const fn new() -> Self {
        Self {
            link: SmrArcLink::new(),
            table: SmrArcWord::new(),
            words: [const { SmrTaggedWord::new() }; W],
        }
    }

    /// The slots, `words()[1]` the default entry.
    #[inline]
    pub fn words(&self) -> &[HeapWord<N>] {
        &self.words
    }
}

impl<N: ArtItem, const W: usize> SmrCounted for ArtHeap<N, W> {
    fn smr_link(&self) -> &SmrArcLink {
        &self.link
    }
}

/// The heap a table owns, of its stride.
enum TableHeap<N: ArtItem> {
    /// A 4-bit table's.
    H4(PoolArc<Heap4<N>>),
    /// An 8-bit (root) table's.
    H8(PoolArc<Heap8<N>>),
}

impl<N: ArtItem> TableHeap<N> {
    /// The heap's slots.
    #[inline]
    fn words(&self) -> &[HeapWord<N>] {
        match self {
            Self::H4(h) => h.words(),
            Self::H8(h) => h.words(),
        }
    }

    /// The heap's `heap[0]`.
    fn table(&self) -> &SmrArcWord<ArtTable<N>> {
        match self {
            Self::H4(h) => &h.table,
            Self::H8(h) => &h.table,
        }
    }
}

/// `struct art_table`: an allotment table, an item of its kind's pool (`"art_table"`).
pub struct ArtTable<N: ArtItem> {
    /// The retirement link of the table's references.
    link: SmrArcLink,
    /// `at_heap`: the table's heap, owned.
    at_heap: TableHeap<N>,
    /// `at_parent`: the parent table (`None` for the root).
    at_parent: Option<PoolArc<ArtTable<N>>>,
    /// `at_index`: index in the parent table (`u32::MAX` for the root).
    at_index: u32,
    /// `at_minfringe`: index that fringe begins.
    at_minfringe: u32,
    /// `at_level`: level of the table.
    at_level: u32,
    /// `at_bits`: stride length of the table.
    at_bits: u32,
    /// `at_offset`: sum of parents' stride len.
    at_offset: u32,
    /// `at_refcnt`: the entries of the table, its child tables and the iterators in it.
    at_refcnt: AtomicU32,
}

impl<N: ArtItem> ArtTable<N> {
    /// The heap's slots.
    #[inline]
    fn words(&self) -> &[HeapWord<N>] {
        self.at_heap.words()
    }

    /// `at_refcnt`, for the tests.
    #[cfg(test)]
    fn refcnt(&self) -> u32 {
        self.at_refcnt.load(Ordering::Relaxed)
    }
}

impl<N: ArtItem> SmrCounted for ArtTable<N> {
    fn smr_link(&self) -> &SmrArcLink {
        &self.link
    }
}

/// The pools of the tables and heaps of one kind of node: `art_boot`'s `at_pool`,
/// `at_heap_4_pool` and `at_heap_8_pool` (the node's own pool, `an_pool`, is its owner's).
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep art.c's pool names so the C can be grepped"
)]
pub struct ArtPools<N: ArtItem> {
    /// `at_pool`, `"art_table"`.
    at_pool: TypedPool<ArtTable<N>>,
    /// `at_heap_4_pool`, `"art_heap4"`.
    at_heap_4_pool: TypedPool<Heap4<N>>,
    /// `at_heap_8_pool`, `"art_heap8"`.
    at_heap_8_pool: TypedPool<Heap8<N>>,
}

impl<N: ArtItem> ArtPools<N> {
    /// The pools, not made yet.
    pub const fn new() -> Self {
        Self {
            at_pool: TypedPool::new(),
            at_heap_4_pool: TypedPool::new(),
            at_heap_8_pool: TypedPool::new(),
        }
    }

    /// `art_boot()`'s tables and heaps: the three pools made, at `IPL_SOFTNET`.
    pub fn art_boot(&'static self) -> Result<(), Errno> {
        self.at_pool.init(IPL_SOFTNET, 0, "art_table")?;
        self.at_heap_4_pool.init(IPL_SOFTNET, 0, "art_heap4")?;
        self.at_heap_8_pool.init(IPL_SOFTNET, 0, "art_heap8")
    }
}

impl<N: ArtItem> Default for ArtPools<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct art`: the root of the ART, equivalent to the radix head; embedded in its owner.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct art's art_* names so the C can be grepped"
)]
pub struct Art<N: ArtItem> {
    /// `art_root`: the root heap, 8-bit (untagged) or 4-bit (tagged); none while empty.
    art_root: SmrTaggedWord<Heap8<N>, Heap4<N>>,
    /// `art_levels`: the stride of each level (`art_nlevels` is its length).
    art_levels: &'static [u32],
    /// `art_alen`: the address length in bits.
    art_alen: u32,
}

/// What [`Art::art_insert`] did with a node.
pub enum ArtInsert<'g, N: ArtItem> {
    /// The node is in the tree: its handle's reference is the words'.
    Inserted,
    /// A node of the same prefix is there already; the new one comes back, never published.
    Exists(SmrRef<'g, N>, PoolArc<N>),
}

/// `struct art_iter`: an iteration over the nodes of a tree, holding the references of the
/// tables on its path (`at_refcnt`: their topology) and a handle of the one it is in (its
/// memory, while the caller's lock and read section are let go between two steps). As the
/// C's, it holds no reference of the nodes it answers: each is lent for the step's section.
/// One given up without [`Art::art_iter_close`] keeps its tables in the tree for good.
#[allow(
    clippy::struct_field_names,
    reason = "the fields keep the C struct art_iter's ai_* names so the C can be grepped"
)]
pub struct ArtIter<N: ArtItem> {
    /// `ai_table`: the table being walked, `None` when done.
    ai_table: Option<PoolArc<ArtTable<N>>>,
    /// `ai_j`.
    ai_j: u32,
    /// `ai_i`.
    ai_i: u32,
}

impl<N: ArtItem> ArtIter<N> {
    /// An iterator before [`Art::art_iter_open`].
    pub const fn new() -> Self {
        Self {
            ai_table: None,
            ai_j: 0,
            ai_i: 0,
        }
    }

    /// Whether the iteration holds no table (done or closed).
    pub fn is_done(&self) -> bool {
        self.ai_table.is_none()
    }

    /// The node in slot `i` of the table the walk is in, for the section `g`.
    fn node<'a>(&'a self, i: usize, g: &'a SmrRead) -> Option<SmrRef<'a, N>> {
        entry_node(self.ai_table.as_deref()?.words()[i].load(g))
    }
}

impl<N: ArtItem> Default for ArtIter<N> {
    fn default() -> Self {
        Self::new()
    }
}

/// A root heap, of its stride.
enum RootRef<'g, N: ArtItem> {
    /// An 8-bit root.
    H8(SmrRef<'g, Heap8<N>>),
    /// A 4-bit root.
    H4(SmrRef<'g, Heap4<N>>),
}

impl<N: ArtItem> Clone for RootRef<'_, N> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: ArtItem> Copy for RootRef<'_, N> {}

impl<'g, N: ArtItem> RootRef<'g, N> {
    /// The root heap's slots.
    #[inline]
    fn words(self) -> &'g [HeapWord<N>] {
        match self {
            Self::H8(h) => h.get().words(),
            Self::H4(h) => h.get().words(),
        }
    }

    /// `art_heap_to_table(heap)`.
    fn table(self, g: &'g SmrRead) -> Option<SmrRef<'g, ArtTable<N>>> {
        match self {
            Self::H8(h) => h.get().table.load(g),
            Self::H4(h) => h.get().table.load(g),
        }
    }
}

/// A node an ART holds: the C's embedded `struct art_node` (its address and prefix length),
/// an item of a pool of its own, counted by the heap words that name it.
pub trait ArtItem: SmrCounted {
    /// `an_addr`: the prefix, its first `plen` bits significant.
    fn addr(&self) -> &[u8; 16];
    /// `an_plen`: the prefix length in bits.
    fn plen(&self) -> u32;
    /// The pools of this kind's tables and heaps.
    fn pools() -> &'static ArtPools<Self>;
}

/// The node an entry names, if it names one (a heap entry or none answers `None`).
#[inline]
fn entry_node<'g, N: ArtItem>(
    e: Option<Tag<SmrRef<'g, N>, SmrRef<'g, Heap4<N>>>>,
) -> Option<SmrRef<'g, N>> {
    match e {
        Some(Tag::A(n)) => Some(n),
        _ => None,
    }
}

/// Whether entry `e` is the node `old` (none for none; a heap entry is never a node).
#[inline]
fn entry_is<N: ArtItem>(
    e: Option<Tag<SmrRef<'_, N>, SmrRef<'_, Heap4<N>>>>,
    old: Option<*const N>,
) -> bool {
    match (e, old) {
        (None, None) => true,
        (Some(Tag::A(n)), Some(o)) => ptr::eq(n.get(), o),
        _ => false,
    }
}

impl<N: ArtItem> Art<N> {
    /// `art_alloc(alen)`, `art_init(art, alen)`: an empty tree for addresses of `alen` bits
    /// (32, 128 or 20); any other length is `panic(9)`, as is a level table that sums to
    /// another length or puts an 8-bit stride below the root.
    pub fn new(alen: u32) -> Self {
        let levels: &'static [u32] = match alen {
            32 => &ART_PLEN32_LEVELS,
            128 => &ART_PLEN128_LEVELS,
            20 => &ART_PLEN20_LEVELS,
            _ => kpanic(format_args!("no configuration for alen {alen}")),
        };
        let bits: u32 = levels.iter().sum();
        if alen != bits {
            kpanic(format_args!("sum of levels {bits} != address len {alen}"));
        }
        if levels.iter().skip(1).any(|&b| b != 4) || !matches!(levels[0], 4 | 8) {
            kpanic(format_args!("art: an unsupported stride below the root"));
        }
        Self {
            art_root: SmrTaggedWord::new(),
            art_levels: levels,
            art_alen: alen,
        }
    }

    /// `art_alen`: the address length in bits.
    pub fn art_alen(&self) -> u32 {
        self.art_alen
    }

    /// `art_nlevels`.
    fn art_nlevels(&self) -> u32 {
        self.art_levels.len() as u32
    }

    /// The root heap, of its stride, for the section `g`.
    #[inline]
    fn root<'g>(&'g self, g: &'g SmrRead) -> Option<RootRef<'g, N>> {
        Some(match self.art_root.load(g)? {
            Tag::A(h) => RootRef::H8(h),
            Tag::B(h) => RootRef::H4(h),
        })
    }

    /// `art_match(art, addr)`: the best existing match for a destination (the longest prefix
    /// that covers it), for the read section `g`.
    pub fn art_match<'g>(&'g self, addr: &[u8], g: &'g SmrRead) -> Option<SmrRef<'g, N>> {
        let mut words = self.root(g)?.words();
        let mut offset = 0;
        let mut level = 0;
        let mut dflt = None;

        // Iterate until we find a leaf.
        loop {
            let bits = self.art_levels[level];
            let p = offset + bits;

            // Remember the default route of each table we visit in case we do not find a
            // better matching route.
            if let Some(n) = entry_node(words[ART_HEAP_IDX_DEFAULT].load(g)) {
                dflt = Some(n);
            }

            // Do a single level route lookup.
            let j = art_bindex(offset, bits, addr, p) as usize;
            match words[j].load(g) {
                // A leaf (none is a leaf): we're done.
                Some(Tag::A(n)) => return Some(n),
                None => return dflt,
                Some(Tag::B(h)) => {
                    words = h.get().words();
                    offset = p;
                    level += 1;
                    kassert!(level < self.art_levels.len());
                }
            }
        }
    }

    /// `art_lookup(art, addr, plen)`: the node of exactly the prefix `addr`/`plen`, for the
    /// read section `g`.
    pub fn art_lookup<'g>(
        &'g self,
        addr: &[u8],
        plen: u32,
        g: &'g SmrRead,
    ) -> Option<SmrRef<'g, N>> {
        kassert!(plen <= self.art_alen);
        let mut words = self.root(g)?.words();

        // Default route.
        if plen == 0 {
            return entry_node(words[ART_HEAP_IDX_DEFAULT].load(g));
        }

        // If the prefix length is smaller than the sum of the stride length at this level
        // the entry must be in the current table.
        let mut offset = 0;
        let mut level = 0;
        let bits = loop {
            let bits = self.art_levels[level];
            let p = offset + bits;
            if plen <= p {
                break bits;
            }

            // Do a single level route lookup; a leaf is a match, but not a perfect one.
            let j = art_bindex(offset, bits, addr, p) as usize;
            let Some(Tag::B(h)) = words[j].load(g) else {
                return None;
            };
            words = h.get().words();
            offset = p;
            level += 1;
            kassert!(level < self.art_levels.len());
        };

        let i = art_bindex(offset, bits, addr, plen) as usize;
        let ahe = match words[i].load(g) {
            Some(Tag::B(h)) => h.get().words()[ART_HEAP_IDX_DEFAULT].load(g),
            e => e,
        };

        // Make sure we've got a perfect match.
        entry_node(ahe).filter(|an| art_node_check(an.get(), addr, plen))
    }

    /// `art_is_empty(art)`: no node (no root heap).
    pub fn art_is_empty(&self) -> bool {
        !self.art_root.is_set()
    }

    /// `art_insert(art, an)`: inserts `an`, or answers the node already there for its prefix
    /// (and `an` back, unpublished); `Err(an)` when a table could not be allocated. The
    /// caller holds the tree's lock (`w`) and the read section `g` for the call.
    pub fn art_insert<'g, W: Exclusive + ?Sized>(
        &'g self,
        w: &W,
        g: &'g SmrRead,
        an: PoolArc<N>,
    ) -> Result<ArtInsert<'g, N>, PoolArc<N>> {
        let plen = an.plen();
        let addr = *an.addr();
        kassert!(plen <= self.art_alen);

        if !self.art_root.is_set() {
            let Some(at) = self.art_table_get(w, g, None, u32::MAX) else {
                return Err(an);
            };
            drop(self.art_root.store(
                w,
                Some(match &at.at_heap {
                    TableHeap::H8(h) => Tag::A(h.clone()),
                    TableHeap::H4(h) => Tag::B(h.clone()),
                }),
            ));
        }
        let Some(root) = self.root(g) else {
            kpanic(format_args!("art: a root heap just published is gone"));
        };
        let mut words = root.words();
        let Some(mut at) = root.table(g) else {
            kpanic(format_args!("art: a heap without its table"));
        };

        // Default route.
        if plen == 0 {
            let ahep = &words[ART_HEAP_IDX_DEFAULT];
            if let Some(oan) = entry_node(ahep.load(g)) {
                return Ok(ArtInsert::Exists(oan, an));
            }
            art_table_ref(&at);
            drop(ahep.store(w, Some(Tag::A(an))));
            return Ok(ArtInsert::Inserted);
        }

        // If the prefix length is smaller than the sum of the stride length at this level the
        // entry must be in the current table.
        loop {
            let p = at.at_offset + at.at_bits;
            if p >= plen {
                break;
            }
            // Do a single level route lookup.
            let j = art_bindex(at.at_offset, at.at_bits, &addr, p) as usize;
            let ahep = &words[j];
            if !matches!(ahep.load(g), Some(Tag::B(_))) {
                // The node of this fringe index is a leaf: allocate a subtable, whose default
                // route the node becomes (`art_table_get` copies it there).
                let Some(child) = self.art_table_get(w, g, Some(at), j as u32) else {
                    return Err(an);
                };
                art_table_ref(&at);
                let TableHeap::H4(heap) = &child.at_heap else {
                    kpanic(format_args!("art: an 8-bit subtable"));
                };
                drop(ahep.store(w, Some(Tag::B(heap.clone()))));
                drop(child);
            }
            let Some(Tag::B(h)) = ahep.load(g) else {
                kpanic(format_args!("art: a subtable just published is gone"));
            };
            words = h.get().words();
            at = match h.get().table.load(g) {
                Some(t) => t,
                None => kpanic(format_args!("art: a heap without its table")),
            };
        }

        let i = art_bindex(at.at_offset, at.at_bits, &addr, plen) as usize;
        let mut ahep = &words[i];
        let mut oahe = ahep.load(g);
        if let Some(Tag::B(h)) = oahe {
            ahep = &h.get().words()[ART_HEAP_IDX_DEFAULT];
            oahe = ahep.load(g);
        }

        // Check if there's an existing node.
        let oan = entry_node(oahe);
        if let Some(o) = oan
            && art_node_check(o.get(), &addr, plen)
        {
            return Ok(ArtInsert::Exists(o, an));
        }

        // If the index `i` of the route that we are inserting is not a fringe index, we need
        // to allot this new route pointer to all the corresponding fringe indices.
        art_table_ref(&at);
        if (i as u32) < at.at_minfringe {
            let old = oan.map(|o| ptr::from_ref(o.get()));
            art_allot(
                w,
                g,
                at.words(),
                at.at_minfringe as usize,
                i,
                old,
                Some(&an),
            );
        } else {
            drop(ahep.store(w, Some(Tag::A(an))));
        }
        Ok(ArtInsert::Inserted)
    }

    /// `art_delete(art, addr, plen)`: removes the node of exactly the prefix `addr`/`plen`,
    /// the next most specific route taking its place, and answers it (its references retired,
    /// so it stays readable for the section `g`). The caller holds the tree's lock (`w`) and,
    /// as the C's, has found the node with [`Art::art_lookup`] under it: the slot of the
    /// prefix's index is not checked against the prefix (a less specific route's copy there
    /// would be removed from that range instead).
    pub fn art_delete<'g, W: Exclusive + ?Sized>(
        &'g self,
        w: &W,
        g: &'g SmrRead,
        addr: &[u8],
        plen: u32,
    ) -> Option<SmrRef<'g, N>> {
        kassert!(plen <= self.art_alen);
        let root = self.root(g)?;
        let mut words = root.words();
        let Some(mut at) = root.table(g) else {
            kpanic(format_args!("art: a heap without its table"));
        };

        // Default route.
        if plen == 0 {
            let ahep = &words[ART_HEAP_IDX_DEFAULT];
            let an = entry_node(ahep.load(g))?;
            drop(ahep.take(w));
            self.art_table_free(w, g, at.get());
            return Some(an);
        }

        // If the prefix length is smaller than the sum of the stride length at this level the
        // entry must be in the current table.
        loop {
            let p = at.at_offset + at.at_bits;
            if p >= plen {
                break;
            }
            // Do a single level route lookup; a leaf has no route to delete.
            let j = art_bindex(at.at_offset, at.at_bits, addr, p) as usize;
            let Some(Tag::B(h)) = words[j].load(g) else {
                return None;
            };
            words = h.get().words();
            at = match h.get().table.load(g) {
                Some(t) => t,
                None => kpanic(format_args!("art: a heap without its table")),
            };
        }

        let i = art_bindex(at.at_offset, at.at_bits, addr, plen) as usize;
        let mut ahep = &words[i];
        let mut ahe = ahep.load(g);
        if let Some(Tag::B(h)) = ahe {
            ahep = &h.get().words()[ART_HEAP_IDX_DEFAULT];
            ahe = ahep.load(g);
        }

        // No route to delete.
        let an = entry_node(ahe)?;

        // Get the next most specific route for the index `i`.
        let j = i >> 1;
        let pahe = if j > 1 {
            entry_node(words[j].load(g)).map(SmrRef::arc)
        } else {
            None
        };

        // If the index `i` of the route that we are removing is not a fringe index, we need
        // to allot the next most specific route pointer to all the corresponding fringe
        // indices.
        if (i as u32) < at.at_minfringe {
            let old = Some(ptr::from_ref(an.get()));
            art_allot(
                w,
                g,
                at.words(),
                at.at_minfringe as usize,
                i,
                old,
                pahe.as_ref(),
            );
        } else {
            drop(ahep.store(w, pahe.map(Tag::A)));
        }

        // We have removed an entry from this table.
        self.art_table_free(w, g, at.get());

        Some(an)
    }

    /// `art_table_free(art, at)`: one reference of `at` dropped; with the last, the table and
    /// every parent left without one are put. Whether `at` was put.
    fn art_table_free<W: Exclusive + ?Sized>(&self, w: &W, g: &SmrRead, at: &ArtTable<N>) -> bool {
        if !art_table_rele(Some(at)) {
            return false;
        }
        // Garbage collect this table and all its parents that are empty.
        let mut cur = Some(at);
        loop {
            cur = match cur {
                Some(t) => self.art_table_put(w, g, t),
                None => None,
            };
            if !art_table_rele(cur) {
                break;
            }
        }
        true
    }

    /// `art_iter_descend(ai, heap, pahe)`: the iteration enters `table` (the table of the
    /// heap it reached), a reference of it taken (`at_refcnt`, and a handle the iterator keeps
    /// while the caller's lock and section may be let go); whether its default route is to be
    /// answered now, which it is unless it is `pahe` (the parent's entry, which the parent's
    /// walk visits).
    fn art_iter_descend(
        g: &SmrRead,
        table: SmrRef<'_, ArtTable<N>>,
        pahe: Option<*const N>,
    ) -> (PoolArc<ArtTable<N>>, bool) {
        art_table_ref(&table);
        // Process the default route now.
        let dflt = table.words()[ART_HEAP_IDX_DEFAULT].load(g);
        let visit = entry_node(dflt).is_some() && !entry_is(dflt, pahe);
        (table.arc(), visit)
    }

    /// `art_iter_open(art, ai)`: the first node of a walk in "natural" order (the default
    /// route first), for the section `g` and until `ai` is moved on; `None` for an empty
    /// tree. The caller holds the tree's lock (`w`).
    pub fn art_iter_open<'a, W: Exclusive + ?Sized>(
        &self,
        ai: &'a mut ArtIter<N>,
        w: &W,
        g: &'a SmrRead,
    ) -> Option<SmrRef<'a, N>> {
        ai.ai_table = None;
        // Empty: we're already done.
        let root = self.root(g)?;
        let Some(table) = root.table(g) else {
            kpanic(format_args!("art: a heap without its table"));
        };
        let (at, visit) = Self::art_iter_descend(g, table, None);
        // Start looking at non-fringe nodes; the default route (index 1) is processed by the
        // parent table (where it belongs), otherwise it could be processed more than once.
        ai.ai_j = 1;
        ai.ai_i = 2;
        ai.ai_table = Some(at);
        if visit {
            return ai.node(ART_HEAP_IDX_DEFAULT, g); // The default route.
        }
        self.art_iter_next(ai, w, g)
    }

    /// `art_iter_next(ai)`: the next node of the walk, for the section `g` and until `ai` is
    /// moved on; `None` when the walk is over (its tables' references given back). The caller
    /// holds the tree's lock (`w`), which it may have let go and taken again since the last
    /// call, with another section.
    pub fn art_iter_next<'a, W: Exclusive + ?Sized>(
        &self,
        ai: &'a mut ArtIter<N>,
        w: &W,
        g: &'a SmrRead,
    ) -> Option<SmrRef<'a, N>> {
        /// What a fringe slot asks of the walk.
        enum Step<T> {
            /// Nothing to answer: the next slot.
            Skip,
            /// Answer the node of this slot.
            Visit(usize),
            /// Enter the child table (and answer its default route or not).
            Descend(T, bool),
        }

        // The table being walked, held by this frame until the walk answers or ascends past
        // the root, then given back to `ai`.
        let mut at = ai.ai_table.take()?;
        let found = 'descend: loop {
            let minfringe = at.at_minfringe;

            // Iterate non-fringe nodes in "natural" order.
            if ai.ai_j < minfringe {
                loop {
                    while ai.ai_i < minfringe {
                        let i = ai.ai_i as usize;
                        ai.ai_i <<= 1;
                        let words = at.words();
                        let pahe =
                            entry_node(words[i >> 1].load(g)).map(|n| ptr::from_ref(n.get()));
                        let ahe = words[i].load(g);
                        if entry_node(ahe).is_some() && !entry_is(ahe, pahe) {
                            break 'descend i;
                        }
                    }
                    ai.ai_j += 2;
                    if ai.ai_j < minfringe {
                        ai.ai_i = ai.ai_j;
                    } else {
                        // Set up the fringe loop.
                        ai.ai_i = minfringe;
                        break;
                    }
                }
            }

            // Descendent tables are only found on fringe nodes, and they record where they
            // were placed in their parent. This allows the iterator to know where to resume
            // traversal when it ascends back to the parent table. By keeping the table refs
            // when going down the tree, the topology is preserved even if all the nodes are
            // removed.
            loop {
                let maxfringe = at.at_minfringe << 1;

                // Iterate fringe nodes.
                while ai.ai_i < maxfringe {
                    let i = ai.ai_i as usize;
                    ai.ai_i += 1;
                    let step = {
                        let words = at.words();
                        let pahe =
                            entry_node(words[i >> 1].load(g)).map(|n| ptr::from_ref(n.get()));
                        let ahe = words[i].load(g);
                        match ahe {
                            Some(Tag::B(h)) => match h.get().table.load(g) {
                                Some(child) => {
                                    let (child, visit) = Self::art_iter_descend(g, child, pahe);
                                    Step::Descend(child, visit)
                                }
                                None => kpanic(format_args!("art: a heap without its table")),
                            },
                            Some(Tag::A(_)) if !entry_is(ahe, pahe) => Step::Visit(i),
                            _ => Step::Skip,
                        }
                    };
                    match step {
                        Step::Skip => {}
                        Step::Visit(i) => break 'descend i,
                        Step::Descend(child, visit) => {
                            // The child's own entries from its index 2 on; the parent's handle
                            // goes (the child's `at_parent` and the parent's count keep it).
                            at = child;
                            ai.ai_j = 1;
                            ai.ai_i = 2;
                            if visit {
                                // Its default route.
                                break 'descend ART_HEAP_IDX_DEFAULT;
                            }
                            // Start looping over the child table.
                            continue 'descend;
                        }
                    }
                }

                // Ascend back up to the parent.
                let parent = at.at_parent.clone();
                ai.ai_i = at.at_index.wrapping_add(1);
                self.art_table_free(w, g, &at);

                // The root table has no parent.
                at = parent?;
                ai.ai_j = at.at_minfringe;
            }
        };
        ai.ai_table = Some(at);
        ai.node(found, g)
    }

    /// `art_iter_close(ai)`: the walk given up early, its tables' references dropped.
    pub fn art_iter_close<W: Exclusive + ?Sized>(&self, ai: &mut ArtIter<N>, w: &W, g: &SmrRead) {
        let mut at = ai.ai_table.take();
        while let Some(t) = at.take() {
            at.clone_from(&t.at_parent);
            self.art_table_free(w, g, &t);
        }
    }

    /// `art_table_get(art, parent, j)`: a new table and its heap, below `parent` at its index
    /// `j` (`u32::MAX` and no parent for the root), whose default route is the parent's entry
    /// at `j`; neither the root nor the parent is changed. `None` without memory.
    fn art_table_get<W: Exclusive + ?Sized>(
        &self,
        w: &W,
        g: &SmrRead,
        parent: Option<SmrRef<'_, ArtTable<N>>>,
        j: u32,
    ) -> Option<PoolArc<ArtTable<N>>> {
        kassert!(j as usize != ART_HEAP_IDX_TABLE && j as usize != ART_HEAP_IDX_DEFAULT);
        kassert!(parent.is_some() || j == u32::MAX);

        let level = parent.map_or(0, |p| p.at_level + 1);
        kassert!(level < self.art_nlevels());

        let pools = N::pools();
        let bits = self.art_levels[level as usize];
        let heap = match bits {
            4 => TableHeap::H4(pools.at_heap_4_pool.get_shared(PR_NOWAIT, ArtHeap::new)?),
            8 => TableHeap::H8(pools.at_heap_8_pool.get_shared(PR_NOWAIT, ArtHeap::new)?),
            _ => kpanic(format_args!("incorrect stride length {bits}")),
        };
        let at = pools.at_pool.get_shared(PR_NOWAIT, || ArtTable {
            link: SmrArcLink::new(),
            at_heap: heap,
            at_parent: parent.map(SmrRef::arc),
            at_index: j,
            at_minfringe: 1 << bits,
            at_level: level,
            at_bits: bits,
            at_offset: parent.map_or(0, |p| p.at_offset + p.at_bits),
            at_refcnt: AtomicU32::new(0),
        })?;

        drop(at.at_heap.table().store(w, Some(at.clone())));
        if let Some(p) = parent {
            let ahe = entry_node(p.words()[j as usize].load(g)).map(SmrRef::arc);
            drop(at.words()[ART_HEAP_IDX_DEFAULT].store(w, ahe.map(Tag::A)));
        }
        Some(at)
    }

    /// `art_table_put(art, at)`: deletes a table with no reference left, its default route
    /// given back to its parent's slot (or the root emptied); answers the parent. The heap's
    /// `heap[0]` is retired, cutting the table's cycle with its heap: both go back to their
    /// pools once their last references (the retired ones among them) are released.
    fn art_table_put<'a, W: Exclusive + ?Sized>(
        &self,
        w: &W,
        g: &SmrRead,
        at: &'a ArtTable<N>,
    ) -> Option<&'a ArtTable<N>> {
        let parent = at.at_parent.as_deref();
        let j = at.at_index;

        kassert!(at.at_refcnt.load(Ordering::Relaxed) == 0);
        kassert!(j != 0 && j != 1);

        if let Some(p) = parent {
            kassert!(j != u32::MAX);
            kassert!(at.at_level == p.at_level + 1);
            kassert!(p.at_refcnt.load(Ordering::Relaxed) >= 1);

            // Give the route back to its parent.
            let ahe = entry_node(at.words()[ART_HEAP_IDX_DEFAULT].load(g)).map(SmrRef::arc);
            drop(p.words()[j as usize].store(w, ahe.map(Tag::A)));
        } else {
            kassert!(j == u32::MAX);
            kassert!(at.at_level == 0);
            drop(self.art_root.take(w));
        }

        drop(at.at_heap.table().take(w));
        parent
    }
}

/// `art_bindex(offset, bits, addr, plen)`: the base index of the part of `addr`/`plen` that
/// the table of stride `bits` at `offset` covers: the multi-level (complete) address and
/// prefix length made the single-level index of that table. With an address of 32 bits in
/// four 8-bit tables, a prefix longer than 24 has four base indexes.
fn art_bindex(offset: u32, bits: u32, addr: &[u8], plen: u32) -> u32 {
    kassert!(plen >= offset);
    kassert!(plen <= offset + bits);

    // Only the part of the prefix length in this table's range counts.
    let plen = plen - offset;

    // The first byte of the address with bits in this table.
    let a = &addr[(offset / 8) as usize..];

    // The table covers the bits between `boff` and `bend`.
    let boff = offset % 8;
    let bend = bits + boff;

    kassert!(bend <= 32);

    let first = u32::from(a[0]) & ((1 << (8 - boff)) - 1);
    let k = if bend > 24 {
        (first << (bend - 8))
            | (u32::from(a[1]) << (bend - 16))
            | (u32::from(a[2]) << (bend - 24))
            | (u32::from(a[3]) >> (32 - bend))
    } else if bend > 16 {
        (first << (bend - 8)) | (u32::from(a[1]) << (bend - 16)) | (u32::from(a[2]) >> (24 - bend))
    } else if bend > 8 {
        (first << (bend - 8)) | (u32::from(a[1]) >> (16 - bend))
    } else {
        (u32::from(a[0]) >> (8 - bend)) & ((1 << bits) - 1)
    };

    // The single level base index formula.
    (k >> (bits - plen)) + (1 << plen)
}

/// `art_node_check(an, addr, plen)`: whether `an` is the prefix `addr`/`plen`.
fn art_node_check<N: ArtItem>(an: &N, addr: &[u8], plen: u32) -> bool {
    if an.plen() != plen {
        return false;
    }
    let a = an.addr();
    let full = (plen / 8) as usize;
    if a[..full] != addr[..full] {
        return false;
    }
    let rem = plen % 8;
    if rem == 0 {
        return true;
    }
    let shift = 8 - rem;
    (a[full] >> shift) == (addr[full] >> shift)
}

/// `art_table_ref(art, at)`: one more reference of `at` (an entry, a child, an iterator).
fn art_table_ref<N: ArtItem>(at: &ArtTable<N>) {
    at.at_refcnt.fetch_add(1, Ordering::Relaxed);
}

/// `art_table_rele(at)`: one reference of `at` dropped; whether it was the last (`false`
/// for no table).
fn art_table_rele<N: ArtItem>(at: Option<&ArtTable<N>>) -> bool {
    let Some(at) = at else {
        return false;
    };
    let old = at.at_refcnt.fetch_sub(1, Ordering::Relaxed);
    kassert!(old > 0);
    old == 1
}

/// `art_allot(at, i, old, new)`: substitutes `new` for `old` in the subtree of the slots
/// `words` whose root index is `i` (not a fringe index): the C's double recursion, which its
/// `goto`s expand (Don Knuth's CWEB). A fringe slot that holds a child's heap has its
/// default route changed instead; a non-fringe slot that holds another entry keeps its
/// subtree (a more specific route owns it). Each slot is stored after its children.
fn art_allot<N: ArtItem, W: Exclusive + ?Sized>(
    w: &W,
    g: &SmrRead,
    words: &[HeapWord<N>],
    minfringe: usize,
    i: usize,
    old: Option<*const N>,
    new: Option<&PoolArc<N>>,
) {
    kassert!(i < minfringe);
    let put = |word: &HeapWord<N>| drop(word.store(w, new.map(|h| Tag::A(h.clone()))));
    for k in [i << 1, (i << 1) + 1] {
        if k >= minfringe {
            // Change fringe nodes.
            let mut target = &words[k];
            let mut e = target.load(g);
            if let Some(Tag::B(h)) = e {
                target = &h.get().words()[ART_HEAP_IDX_DEFAULT];
                e = target.load(g);
            }
            if entry_is(e, old) {
                put(target);
            }
        } else if entry_is(words[k].load(g), old) {
            art_allot(w, g, words, minfringe, k, old, new);
        }
    }
    // Change non-fringe node.
    put(&words[i]);
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the ART over a test node: insertion, longest-prefix matching, perfect
    // lookups, deletion with the next most specific route taking over, the allotment's counted
    // words, iteration (its order and its references across a released lock), a reference
    // model of the longest-prefix match over random prefixes, and the release of every table,
    // heap and node after the grace periods.

    use std::sync::MutexGuard;
    use std::vec;
    use std::vec::Vec;

    use hal::rwlock::RawRwlock;
    use hal::smr;

    use super::*;
    use crate::testutil;

    /// The test's node: a prefix.
    struct TestNode {
        link: SmrArcLink,
        addr: [u8; 16],
        plen: u32,
    }

    impl SmrCounted for TestNode {
        fn smr_link(&self) -> &SmrArcLink {
            &self.link
        }
    }

    static TEST_POOLS: ArtPools<TestNode> = ArtPools::new();
    static NODES: TypedPool<TestNode> = TypedPool::new();

    impl ArtItem for TestNode {
        fn addr(&self) -> &[u8; 16] {
            &self.addr
        }
        fn plen(&self) -> u32 {
            self.plen
        }
        fn pools() -> &'static ArtPools<Self> {
            &TEST_POOLS
        }
    }

    /// The tree's lock (the routing table's `r_lock` in the kernel).
    static LOCK: RawRwlock = RawRwlock::new("arttest");

    /// No CPU but the caller's.
    struct Alone;

    impl smr::CpuWalk for Alone {
        fn running(&self, _n: usize) -> bool {
            false
        }
        fn peg(&self, n: usize) {
            panic!("peg({n}) with no other CPU");
        }
        fn unpeg(&self) {}
    }

    /// Every release queued so far run, grace period after grace period.
    fn grace() {
        loop {
            smr::idle();
            if smr::collect().run(&Alone) == 0 {
                break;
            }
        }
    }

    fn setup() -> MutexGuard<'static, ()> {
        testutil::install();
        let g = testutil::pages_reset();
        let _ = TEST_POOLS.art_boot();
        let _ = NODES.init(0, 0, "arttestnode");
        g
    }

    fn node(addr: &[u8], plen: u32) -> PoolArc<TestNode> {
        let mut a = [0; 16];
        a[..addr.len()].copy_from_slice(addr);
        NODES
            .get_shared(PR_NOWAIT, || TestNode {
                link: SmrArcLink::new(),
                addr: a,
                plen,
            })
            .expect("a node")
    }

    /// The node for `addr`/`plen`, inserted (new).
    fn insert(art: &Art<TestNode>, addr: &[u8], plen: u32) -> PoolArc<TestNode> {
        let an = node(addr, plen);
        let w = LOCK.write();
        let g = smr::read();
        match art.art_insert(&w, &g, an.clone()) {
            Ok(ArtInsert::Inserted) => {}
            _ => panic!("{addr:?}/{plen} was new"),
        }
        an
    }

    fn delete(art: &Art<TestNode>, addr: &[u8], plen: u32) -> bool {
        let w = LOCK.write();
        let g = smr::read();
        art.art_delete(&w, &g, addr, plen)
            .is_some_and(|an| an.plen == plen)
    }

    fn matched(art: &Art<TestNode>, addr: [u8; 4]) -> Option<(Vec<u8>, u32)> {
        let g = smr::read();
        art.art_match(&addr, &g)
            .map(|an| (an.addr[..4].to_vec(), an.plen))
    }

    fn walk(art: &Art<TestNode>) -> Vec<(Vec<u8>, u32)> {
        let mut seen = Vec::new();
        let mut ai = ArtIter::new();
        let w = LOCK.write();
        let g = smr::read();
        let mut an = art.art_iter_open(&mut ai, &w, &g);
        while let Some(n) = an {
            seen.push((n.addr[..4].to_vec(), n.plen));
            an = art.art_iter_next(&mut ai, &w, &g);
        }
        assert!(ai.is_done(), "a finished walk holds no table");
        seen
    }

    #[test]
    fn bindex_matches_the_papers_formula() {
        // First level, stride 8: a /8 of 10 is index 10 + 256; a /4 of 10 (0000 1010) is 0 + 16.
        assert_eq!(art_bindex(0, 8, &[10, 0, 2, 15], 8), 10 + 256);
        assert_eq!(art_bindex(0, 8, &[10, 0, 2, 15], 4), 16);
        assert_eq!(art_bindex(0, 8, &[0xff, 0, 0, 0], 1), 1 + 2);
        // Second level (offset 8, stride 4): the high nibble of the second byte.
        assert_eq!(art_bindex(8, 4, &[10, 0xa5, 0, 0], 12), 0xa + 16);
        // Last level (offset 28): the low nibble of the fourth byte.
        assert_eq!(art_bindex(28, 4, &[10, 0, 2, 0x0f], 32), 0xf + 16);
        // Wider strides than the configured tables use (the formula's other branches).
        assert_eq!(
            art_bindex(4, 16, &[0x0a, 0xbc, 0xde, 0], 20),
            0xabcd + (1 << 16)
        );
        assert_eq!(
            art_bindex(0, 24, &[1, 2, 3, 4], 24),
            0x0001_0203 + (1 << 24)
        );
        assert_eq!(
            art_bindex(4, 28, &[0xf1, 2, 3, 4], 32),
            0x0102_0304 + (1 << 28)
        );
    }

    #[test]
    fn a_node_check_compares_the_prefix_bits_only() {
        let _g = setup();
        let an = node(&[10, 0, 2, 0xff], 28);
        assert!(art_node_check(&*an, &[10, 0, 2, 0xf0], 28));
        assert!(!art_node_check(&*an, &[10, 0, 2, 0xe0], 28));
        assert!(
            !art_node_check(&*an, &[10, 0, 2, 0xf0], 24),
            "another length"
        );
        let an = node(&[10, 0, 2, 0], 24);
        assert!(art_node_check(&*an, &[10, 0, 2, 99], 24));
    }

    #[test]
    fn longest_prefix_wins() {
        let _g = setup();
        let art: Art<TestNode> = Art::new(32);
        assert!(art.art_is_empty());
        assert!(matched(&art, [10, 0, 2, 2]).is_none());

        let nodes = [
            insert(&art, &[0, 0, 0, 0], 0),
            insert(&art, &[10, 0, 0, 0], 8),
            insert(&art, &[10, 0, 2, 0], 24),
            insert(&art, &[10, 0, 2, 2], 32),
            insert(&art, &[192, 168, 0, 0], 16),
        ];
        assert!(!art.art_is_empty());

        assert_eq!(matched(&art, [10, 0, 2, 2]), Some((vec![10, 0, 2, 2], 32)));
        assert_eq!(matched(&art, [10, 0, 2, 3]), Some((vec![10, 0, 2, 0], 24)));
        assert_eq!(matched(&art, [10, 9, 9, 9]), Some((vec![10, 0, 0, 0], 8)));
        assert_eq!(
            matched(&art, [192, 168, 7, 1]),
            Some((vec![192, 168, 0, 0], 16))
        );
        assert_eq!(matched(&art, [8, 8, 8, 8]), Some((vec![0, 0, 0, 0], 0)));

        // Perfect lookups only find the exact prefix.
        {
            let g = smr::read();
            assert!(art.art_lookup(&[10, 0, 2, 0], 24, &g).is_some());
            assert!(art.art_lookup(&[10, 0, 2, 0], 23, &g).is_none());
            assert!(art.art_lookup(&[10, 0, 0, 0], 8, &g).is_some());
            assert!(art.art_lookup(&[0, 0, 0, 0], 0, &g).is_some());
            assert!(art.art_lookup(&[10, 0, 2, 2], 32, &g).is_some());
            assert!(art.art_lookup(&[10, 0, 2, 3], 32, &g).is_none());
        }

        // A second node for an existing prefix gets the existing one, and itself, back.
        let dup = node(&[10, 0, 2, 0], 24);
        {
            let w = LOCK.write();
            let g = smr::read();
            match art.art_insert(&w, &g, dup.clone()) {
                Ok(ArtInsert::Exists(old, back)) => {
                    assert!(ptr::eq(old.get(), &*nodes[2]));
                    assert!(PoolArc::ptr_eq(&back, &dup));
                }
                _ => panic!("the existing node"),
            }
        }
        assert_eq!(PoolArc::count(&dup), 1, "never published");

        for (addr, plen) in [
            ([0, 0, 0, 0], 0),
            ([10, 0, 0, 0], 8),
            ([10, 0, 2, 0], 24),
            ([10, 0, 2, 2], 32),
            ([192, 168, 0, 0], 16),
        ] {
            assert!(delete(&art, &addr, plen));
        }
        assert!(art.art_is_empty());
        grace();
        for n in &nodes {
            assert_eq!(PoolArc::count(n), 1, "every word's reference released");
        }
    }

    #[test]
    fn deletion_hands_the_range_to_the_next_most_specific_prefix() {
        let _g = setup();
        let art: Art<TestNode> = Art::new(32);
        let _n8 = insert(&art, &[10, 0, 0, 0], 8);
        let _n24 = insert(&art, &[10, 0, 2, 0], 24);
        let _n32 = insert(&art, &[10, 0, 2, 2], 32);

        assert!(delete(&art, &[10, 0, 2, 0], 24));
        assert!(!delete(&art, &[10, 0, 2, 0], 24), "already gone");
        assert_eq!(matched(&art, [10, 0, 2, 7]), Some((vec![10, 0, 0, 0], 8)));
        assert_eq!(matched(&art, [10, 0, 2, 2]), Some((vec![10, 0, 2, 2], 32)));

        assert!(delete(&art, &[10, 0, 2, 2], 32));
        assert_eq!(matched(&art, [10, 0, 2, 2]), Some((vec![10, 0, 0, 0], 8)));

        assert!(delete(&art, &[10, 0, 0, 0], 8));
        assert!(matched(&art, [10, 0, 2, 2]).is_none());
        assert!(art.art_is_empty(), "the last reference put every table");
        grace();
    }

    #[test]
    fn an_allotment_counts_every_word_that_names_the_node() {
        let _g = setup();
        let tables = TEST_POOLS.at_pool.nout();
        let art: Art<TestNode> = Art::new(32);
        let n8 = insert(&art, &[10, 0, 0, 0], 8);
        assert_eq!(PoolArc::count(&n8), 2, "a fringe slot of the root");
        // 0/4 is index 16 of the 8-bit root: its 15 non-fringe slots (16, 32-33, 64-67,
        // 128-135) and the 16 fringe ones 256-271 but 266, where 10/8 is.
        let n4 = insert(&art, &[0, 0, 0, 0], 4);
        assert_eq!(PoolArc::count(&n4), 1 + 30);
        assert_eq!(
            PoolArc::count(&n8),
            2,
            "a more specific route keeps its slot"
        );
        assert_eq!(matched(&art, [10, 1, 1, 1]), Some((vec![10, 0, 0, 0], 8)));
        assert_eq!(matched(&art, [11, 1, 1, 1]), Some((vec![0, 0, 0, 0], 4)));

        // Deleting it retires its 30 references; a reader that loaded it keeps a good
        // reference for its section, and the references are released only after the grace
        // periods that follow.
        {
            let r = smr::read();
            let seen = art.art_match(&[11, 1, 1, 1], &r).expect("0/4");
            {
                let w = LOCK.write();
                let g = smr::read();
                let old = art.art_delete(&w, &g, &[0, 0, 0, 0], 4).expect("deleted");
                assert!(SmrRef::ptr_eq(old, seen));
            }
            assert_eq!(seen.plen, 4, "still readable in the reader's section");
            assert!(
                art.art_match(&[11, 1, 1, 1], &r).is_none(),
                "gone for new loads"
            );
        }
        assert_eq!(PoolArc::count(&n4), 31, "retired, not released yet");
        assert_eq!(matched(&art, [11, 1, 1, 1]), None);
        grace();
        assert_eq!(PoolArc::count(&n4), 1);

        assert!(delete(&art, &[10, 0, 0, 0], 8));
        grace();
        assert_eq!(PoolArc::count(&n8), 1);
        assert_eq!(
            TEST_POOLS.at_pool.nout(),
            tables,
            "every table back in its pool"
        );
    }

    #[test]
    fn subtables_take_the_parents_route_as_their_default() {
        let _g = setup();
        let tables = TEST_POOLS.at_pool.nout();
        let heaps = TEST_POOLS.at_heap_4_pool.nout();
        let art: Art<TestNode> = Art::new(32);
        let _n8 = insert(&art, &[10, 0, 0, 0], 8);
        // A /32 under it needs a table per level below the root: 10.0/12, 10.0.0/16, ...
        let _n32 = insert(&art, &[10, 0, 2, 2], 32);
        assert_eq!(
            TEST_POOLS.at_pool.nout(),
            tables + 7,
            "the root and six subtables"
        );
        assert_eq!(TEST_POOLS.at_heap_4_pool.nout(), heaps + 6);
        assert_eq!(matched(&art, [10, 0, 2, 3]), Some((vec![10, 0, 0, 0], 8)));
        {
            let g = smr::read();
            let root = art.root(&g).expect("a root");
            let at = root.table(&g).expect("its table");
            assert_eq!(at.refcnt(), 2, "10/8's entry and one child table");
        }
        assert!(delete(&art, &[10, 0, 2, 2], 32));
        assert_eq!(matched(&art, [10, 0, 2, 2]), Some((vec![10, 0, 0, 0], 8)));
        assert!(delete(&art, &[10, 0, 0, 0], 8));
        grace();
        assert_eq!(TEST_POOLS.at_pool.nout(), tables);
        assert_eq!(TEST_POOLS.at_heap_4_pool.nout(), heaps);
    }

    const PREFIXES: [([u8; 4], u32); 7] = [
        ([0, 0, 0, 0], 0),
        ([10, 0, 0, 0], 8),
        ([10, 0, 0, 0], 12),
        ([10, 0, 2, 0], 24),
        ([10, 0, 2, 2], 32),
        ([10, 0, 2, 15], 32),
        ([172, 16, 0, 0], 12),
    ];

    /// Whether `a`/`alen` covers `b`/`blen`.
    fn covers(a: &[u8], alen: u32, b: &[u8], blen: u32) -> bool {
        alen <= blen && (0..alen).all(|bit| bitat(a, bit) == bitat(b, bit))
    }

    fn bitat(a: &[u8], bit: u32) -> bool {
        a[(bit / 8) as usize] & (0x80 >> (bit % 8)) != 0
    }

    #[test]
    fn iteration_visits_every_prefix_once_the_covering_ones_first() {
        let _g = setup();
        let art: Art<TestNode> = Art::new(32);
        let _nodes: Vec<_> = PREFIXES.iter().map(|(a, p)| insert(&art, a, *p)).collect();

        let seen = walk(&art);
        assert_eq!(seen.len(), PREFIXES.len());
        for (addr, plen) in PREFIXES {
            assert_eq!(
                seen.iter().filter(|s| **s == (addr.to_vec(), plen)).count(),
                1,
                "{addr:?}/{plen} seen once"
            );
        }
        assert_eq!(seen[0], (vec![0, 0, 0, 0], 0), "the default route first");
        for (i, (a, al)) in seen.iter().enumerate() {
            for (b, bl) in &seen[..i] {
                assert!(
                    !(covers(a, *al, b, *bl) && al != bl),
                    "{a:?}/{al} after {b:?}/{bl}"
                );
            }
        }
        // Two disjoint /8s and a /12: the fringe's order.
        assert_eq!(
            seen.iter().map(|s| s.1).collect::<Vec<_>>(),
            vec![0, 8, 12, 24, 32, 32, 12]
        );

        // Stopping early closes the iterator and gives the tables' references back.
        {
            let mut ai = ArtIter::new();
            let w = LOCK.write();
            let g = smr::read();
            assert!(art.art_iter_open(&mut ai, &w, &g).is_some());
            assert!(art.art_iter_next(&mut ai, &w, &g).is_some());
            assert!(!ai.is_done());
            art.art_iter_close(&mut ai, &w, &g);
            assert!(ai.is_done());
        }

        // Every reference the walks took is given back: deleting everything empties the tree.
        for (addr, plen) in PREFIXES {
            assert!(delete(&art, &addr, plen));
        }
        assert!(art.art_is_empty());
        grace();
    }

    #[test]
    fn a_walk_goes_on_after_deletions_made_with_the_lock_released() {
        let _g = setup();
        let tables = TEST_POOLS.at_pool.nout();
        let art: Art<TestNode> = Art::new(32);
        let _nodes: Vec<_> = PREFIXES.iter().map(|(a, p)| insert(&art, a, *p)).collect();
        let order = walk(&art);

        let mut ai = ArtIter::new();
        let mut seen = Vec::new();
        {
            let w = LOCK.write();
            let g = smr::read();
            if let Some(an) = art.art_iter_open(&mut ai, &w, &g) {
                seen.push((an.addr[..4].to_vec(), an.plen));
            }
            if let Some(an) = art.art_iter_next(&mut ai, &w, &g) {
                seen.push((an.addr[..4].to_vec(), an.plen));
            }
        }
        assert_eq!(seen, order[..2]);
        // The lock let go: the walk's next table loses all its routes, and a route ahead of
        // the walk goes too. The walk's references keep the tables it is in.
        for (addr, plen) in [
            ([10, 0, 0, 0], 12),
            ([10, 0, 2, 0], 24),
            ([10, 0, 2, 2], 32),
        ] {
            assert!(delete(&art, &addr, plen));
        }
        {
            let w = LOCK.write();
            let g = smr::read();
            while let Some(an) = art.art_iter_next(&mut ai, &w, &g) {
                seen.push((an.addr[..4].to_vec(), an.plen));
            }
        }
        assert!(ai.is_done());
        assert_eq!(
            seen,
            vec![
                (vec![0, 0, 0, 0], 0),
                (vec![10, 0, 0, 0], 8),
                (vec![10, 0, 2, 15], 32),
                (vec![172, 16, 0, 0], 12),
            ]
        );
        for (addr, plen) in [
            ([0, 0, 0, 0], 0),
            ([10, 0, 0, 0], 8),
            ([10, 0, 2, 15], 32),
            ([172, 16, 0, 0], 12),
        ] {
            assert!(delete(&art, &addr, plen));
        }
        assert!(art.art_is_empty());
        grace();
        assert_eq!(
            TEST_POOLS.at_pool.nout(),
            tables,
            "the walk's tables put after it"
        );
    }

    #[test]
    fn ipv6_prefixes_of_every_length() {
        let _g = setup();
        let art: Art<TestNode> = Art::new(128);
        let db8 = [0x20, 0x01, 0x0d, 0xb8];
        let mut a64 = [0u8; 16];
        a64[..4].copy_from_slice(&db8);
        let mut a128 = a64;
        a128[15] = 1;
        let _n = [
            insert(&art, &[0; 16], 0),
            insert(&art, &db8, 32),
            insert(&art, &a64, 64),
            insert(&art, &a128, 128),
        ];
        let g = smr::read();
        let plen_of = |a: &[u8; 16]| art.art_match(a, &g).map(|n| n.plen);
        assert_eq!(plen_of(&a128), Some(128));
        let mut other = a128;
        other[15] = 2;
        assert_eq!(plen_of(&other), Some(64));
        // Byte 7 is the /64's last; byte 8 would still fall under it.
        other[7] = 1;
        assert_eq!(plen_of(&other), Some(32));
        other[3] = 0xb9;
        assert_eq!(plen_of(&other), Some(0));
        assert!(art.art_lookup(&a64, 64, &g).is_some());
        assert!(art.art_lookup(&a64, 63, &g).is_none());
        drop(g);
        for (a, p) in [([0u8; 16], 0), (a64, 32), (a64, 64), (a128, 128)] {
            assert!(delete(&art, &a, p));
        }
        assert!(art.art_is_empty());
        grace();
    }

    /// A small linear congruential generator: the same prefixes on every run.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) as u32
        }
    }

    #[test]
    fn matches_agree_with_a_flat_longest_prefix_search() {
        let _g = setup();
        let tables = TEST_POOLS.at_pool.nout();
        let art: Art<TestNode> = Art::new(32);
        let mut rng = Lcg(0x5eed);
        let mut set: Vec<([u8; 4], u32, PoolArc<TestNode>)> = Vec::new();
        let masked = |a: u32, plen: u32| {
            let m = if plen == 0 {
                0
            } else {
                u32::MAX << (32 - plen)
            };
            (a & m).to_be_bytes()
        };
        for step in 0..300 {
            if set.len() > 4 && rng.next() % 3 == 0 {
                let i = rng.next() as usize % set.len();
                let (a, p, _) = set.swap_remove(i);
                assert!(delete(&art, &a, p), "step {step}: {a:?}/{p} deleted");
            } else {
                // Prefixes around a few networks, so they nest.
                let base = [0x0a00_0000u32, 0xac10_0000, 0xc0a8_0000][rng.next() as usize % 3];
                let plen = rng.next() % 33;
                let a = masked(base | (rng.next() & 0x00ff_ffff), plen);
                if set.iter().any(|(b, q, _)| *b == a && *q == plen) {
                    continue;
                }
                let n = insert(&art, &a, plen);
                set.push((a, plen, n));
            }
            for _ in 0..16 {
                let probe = [0x0a00_0000u32, 0xac10_0000, 0xc0a8_0000][rng.next() as usize % 3]
                    | (rng.next() & 0x00ff_ffff);
                let pb = probe.to_be_bytes();
                let want = set
                    .iter()
                    .filter(|(a, p, _)| covers(a, *p, &pb, 32))
                    .max_by_key(|(_, p, _)| *p)
                    .map(|(a, p, _)| (a.to_vec(), *p));
                assert_eq!(matched(&art, pb), want, "step {step}: {pb:?}");
            }
        }
        // The walk sees the set, once each.
        let seen = walk(&art);
        assert_eq!(seen.len(), set.len());
        for (a, p, _) in &set {
            let g = smr::read();
            assert!(art.art_lookup(a, *p, &g).is_some(), "{a:?}/{p}");
        }
        for (a, p, _) in core::mem::take(&mut set) {
            assert!(delete(&art, &a, p));
        }
        assert!(art.art_is_empty());
        grace();
        assert_eq!(TEST_POOLS.at_pool.nout(), tables);
    }
}
/* </TESTS> */
