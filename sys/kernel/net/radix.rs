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
 * Copyright (c) 1988, 1989, 1993
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
 *	@(#)radix.h	8.2 (Berkeley) 10/31/94
 */
/*
 * Copyright (c) 1988, 1989, 1993
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
 *	@(#)radix.c	8.6 (Berkeley) 10/17/95
 */
/* </LICENSES> */

/* <CODE> */
//! Radix search trees for routing lookups: `<net/radix.h>` and `net/radix.c`, a tree that
//! owns its nodes, its keys and its values. The NFS server's export lists use it (one tree a
//! mount, `kernel::fs::export`); pf's tables and the IPsec policy database at N6.7b1.
//!
//! Upstream: sys/net/radix.h @ 3ce1f3f79392
//! Upstream: sys/net/radix.c @ 3ce1f3f79392
//! LZ: sys/net/radix.rs@3c62ede68802
//!
//! The keys are kept in a radix tree with one-way branching removed. An internal node `n`
//! tests one bit, its index `rn_b`; every descendant of `n` has keys that agree on the bits
//! before it, at least one of them has a one at bit `rn_b` and at least one a zero.
//!
//! A route is a key and a mask, the key being its own bitwise and with the mask. The index of
//! a mask is the number of its first zero bit (bit 0 the highest of the first byte). A mask
//! is normal when every bit past its index is zero. A route `(k, m)` under a node `n` with a
//! normal `m` of index `rn_b` (or less: the bits of `k` before `rn_b` are then zero, as they
//! are for every descendant of `n`) applies to every descendant of `n`; a non-normal mask of
//! index at most `rn_b` may apply to many of them. So each route with a mask leaves an
//! annotation ([`RadixMask`]) on the highest internal node it can apply under, and a lookup
//! that misses at the leaf climbs back through those annotations. A normal route's
//! annotation names its leaf, which short-cuts the mask-and-compare of the others; host
//! routes (no mask) sort first among the routes of one key.
//!
//! Keys and masks are byte strings whose first byte is their length (a `struct sockaddr`'s
//! `sa_len`, `SALEN`); every tree compares them from its offset (`rn_inithead`'s `off`, the
//! address inside the socket address). A tree normalises each mask once into its own mask
//! tree (the C's `mask_rnhead`), and routes name that copy, so masks compare by identity.
//!
//! Callers serialise every access to a tree (the export list under the kernel lock, pf's
//! tables under pf's lock, the policy database under the net lock): changes take `&mut`,
//! lookups and walks `&`.
//!
//! ## Deviations
//! - A key or mask shorter than the tree's `K` bytes is read as if zero bytes followed it,
//!   and a length byte above `K` reads as `K` (D-7). radix.c compares as far as the length
//!   bytes say, past the object when the object is shorter (the client's address in its
//!   mbuf, the bytes behind an export's address); every caller in the tree passes `K`-byte
//!   objects of its own length or less (`sockaddr_in`s of 16 for the exports, pf's 28-byte
//!   `pfsockaddr_union`, the policy database's 48-byte `sockaddr_encap`). Masks are clamped
//!   to `K` where the C clamps them to `max_keylen`, the largest of every `rn_init` call
//!   (48 once PF_KEY has started): only a mount(2) export mask could be longer than its tree
//!   (`ex_masklen` up to `MLEN`), and its bytes past a `sockaddr_in` have no address to mask.
//! - Each tree has its own mask tree and its own annotation records, both freed with the
//!   tree (D-4): no `rn_init`, `rn_zeros`, `rn_ones`, `max_keylen`, `mask_rnhead` or
//!   `rtmask_pool`, and no "rtmask" line in `vmstat -m` (D-12). Masks compare by identity
//!   inside one tree, which is all radix.c compares; the C's one mask tree is changed by
//!   mount(2) under the kernel lock and by PF_KEY under the net lock alone.
//! - A tree whose last route is deleted gives back its node chunks, its mask tree and its
//!   annotations, and the next add starts it again (the C keeps the head until its owner
//!   frees it, and never frees a mask). Nothing observes the difference: an empty tree
//!   answers no lookup either way. An add refused on an empty tree keeps the head it made,
//!   as the C's `rn_inithead` head stays, so the export list's next add answers what the
//!   C's does.
//! - An add takes the slot for its node pair from the tree (`M_NOWAIT`), where the C's
//!   caller embeds the pair in its own entry before the call: a slot the tree cannot have is
//!   the C's `NULL` (the value comes back in the `Err`).
//! - `rn_delete` of a root leaf (a key whose bytes from the offset are all zeros, or all ones,
//!   asked for without a mask) answers `None` in every build. The C's `KASSERT` stops a
//!   `DIAGNOSTIC` kernel there; without it the C unlinks the root.
//! - `rn_delete`'s compare of a key shorter than the offset compares nothing (the C passes
//!   `memcmp` a negative length), as `rn_insert`'s and `rn_match`'s do.
//! - `rn_delete`'s "Orphaned Mask" line names the annotation's and the leaf's indices; the C
//!   prints their addresses.
//! - `rn_addmask`'s `addmask_key` starts zeroed: the C's trailing-zero trim may read its
//!   uninitialised length byte, with the same result either way.
//! - `rn_walktree`'s callback and `rnh_rtableid` (never written in the C, so a walker's id is
//!   always 0) are a cursor, [`RadixTree::first`] and [`RadixTree::next`]: the successor is
//!   taken before the step's body runs, as the C computes it before calling `f`, so the body
//!   may remove the leaf it was given. Leaves duplicated onto the right root leaf (a key of all
//!   ones) are not visited, as the C's walk ends at that root.
//! - A [`Leaf`] carries the generation its slot had when its route was added; after 2^32
//!   adds to one tree the counter wraps and a handle that old may name a newer route.
//! - D-20 (the user, 2026-11-23: "repair on delete"). radix.c can leave a normal route's
//!   annotation naming a route it has freed: a route of a non-normal mask linked just before
//!   a normal route of the same key and index takes that route's annotation
//!   (`rn_add_radix_mask`'s duplicated-key branch, "new route is better", radix.c:584-586),
//!   and on its delete `rn_del_radix_mask` takes the mask branch by the route's own flag,
//!   reads the union (radix.h:82-89) as `rm_mask`, logs "inconsistent annotation" and
//!   returns (radix.c:856-858); the route is freed (`vfs_subr.c:1558-1559`'s `free` of the
//!   entry, pf's and the SPD's pools) and `rn_match` then answers it (radix.c:320). Here the
//!   branch follows the annotation's flag, which the kernel can read (its union words are
//!   tagged): a normal annotation held by the route being deleted passes to another route
//!   that holds it (the next of the chain, else the first before it, else the first under
//!   the node it hangs on: a route of another key shares it when its mask is the
//!   annotation's leaf's), or leaves the tree when none does, so no route is left naming a
//!   freed annotation; one shared by the route loses that reference, where radix.c leaves
//!   the count one high, so a delete radix.c would later refuse succeeds (its normal
//!   branch's "inconsistent dupedkey list", radix.c:844-849). No delete the C makes is
//!   refused. The tree answers as the one built without the route, with one difference:
//!   when only routes of non-normal masks still hold the annotation, the one that takes it
//!   is short-cut as normal by `rn_match`, as radix.c short-cuts a non-normal route that
//!   took it (radix.c:584-586, 316-321), so an address that route's mask refuses is still
//!   answered by it, where the tree built without would give that route an annotation of
//!   its mask. The C's state is not reachable from the tree's users at the pin: pf adds
//!   prefix masks only (pf_table.c:1271, 1280), the SPD's masks are non-normal at index 16
//!   (ip_spd.c:107, pfkeyv2_convert.c:473, 500), and the exports free their entries all at
//!   once (vfs_subr.c:1565-1575). Each annotation also records its leaf's generation, a
//!   check `rn_match` makes on a normal annotation it returns (after `rn_b`'s test): an
//!   annotation still listed whose leaf has gone (radix.c:866-868's "Wasn't lifted at all"
//!   return and the "Orphaned Mask" demote can leave one) is passed over; the property tests
//!   check that the deletes leave none. The differential test against `bsd`'s tree compares
//!   the two up to the first repair.
//!
//! Kept as radix.c answers (the differential test and the tests name them): a route linked
//! at the head of its key's chain takes the old head's flags (`rn_link_dupedkey`), so a
//! non-normal mask there may be short-cut as normal; `rn_add_dupedkey`'s duplicate test stops
//! where the chain's order places the new mask, so mixed masks can put one key and mask in a
//! chain twice (a delete then takes the first); `rn_del_radix_mask` refuses a delete (`-1`)
//! on annotations it finds inconsistent, and the route stays.
//!
//! ## Redesign
//! - LZ's intrusive tree (`struct radix_node treenodes[2]` embedded first in the caller's
//!   entry, `Cell<*const RadixNode>` links, keys as `*const u8` into the caller's memory, the
//!   leaf cast back to its entry, 60 `unsafe`) -> [`RadixTree<T, K>`], which owns its nodes,
//!   its keys and its values: a slot holds the C's pair (`nodes[0]` the leaf, `nodes[1]` the
//!   internal node `rn_newpair` makes beside it), the route's `K` key bytes, its value and a
//!   generation; every link is a `u32` index (node `2 * slot` the leaf, `2 * slot + 1` the
//!   internal node, so the C's `tt[1]` is `tt | 1`). `rn_swap_nodes` moves an internal node
//!   between slots as the C moves it between pairs, so a leaf, and its [`Leaf`] handle, stay
//!   put. The value is the C's owner: no leaf-to-entry cast (`rn2ke`, `ipsec_policy_of`, the
//!   export's `(struct netcred *)rn`).
//! - The algorithm is radix.c's, function by function under the C's names: the same branches
//!   in the same order, `rn_b` an `i16`, the bit arithmetic on bytes. The C's unions are kept
//!   as two shared words a node (`rn_l`/`rn_mask`, `rn_r`/`rn_dupedkey`) and one a record
//!   (`rm_mask`/`rm_leaf`); a mask reference carries a tag bit (`MASK`), so a mask never
//!   compares equal to a node where the C compares a mask pointer with a node pointer
//!   (`rn_del_radix_mask` on a route sharing a normal route's annotation).
//! - Storage (D-3; the user's sizing of 2026-11-23, "start small, grow"): the slots, the mask
//!   tree's slots and the annotation records live in three chunked arrays (`Slab`: chunks of
//!   8, 8, 16 and 32 elements, then 64, `malloc(9)` of the tree's type, `M_NOWAIT`, behind a
//!   directory that doubles by moving chunk handles); a node is the directory's entry, then
//!   the slot: two loads where the C has one. An export list of a few hosts takes 1,760 bytes
//!   asked of `malloc` (the node and mask trees' first chunks and their directories; 11,840
//!   with 64-element chunks). Lookups and walks never allocate; an add takes at most one
//!   slot, one mask slot and the annotations radix.c makes.
//! - `rn_addmask`'s search mode is `rn_findmask`, so a lookup takes the tree shared;
//!   `rn_delete`'s unlinking half is `rn_delete_leaf`, which [`RadixTree::remove`] shares.
//!   `rn_inithead` and `rn_initmask` are [`RadixTree::inithead`], which the first add calls.
//! - Costs against the C: `rn_match` is the C's steps with two loads a node (each leaf's slot
//!   read once for its node, key and mask), the key copied once into a `K`-byte array (the
//!   slices' bounds checked once), and, on a normal annotation it returns, one slot load for
//!   the leaf's generation; an add copies the key into its slot; the walk's successor is the
//!   C's climb, plus a climb of the duplicated key chain to its head once a chain (O(1)
//!   amortised a leaf, docs/COLLECTIONS.md).

use hal::malloc::{M_NOWAIT, MArray};
use hal::prf::log;

use crate::assert::{kassert, kpanic};
use crate::log::LOG_ERR;

/// `RNF_NORMAL`: the leaf holds a normal route.
pub const RNF_NORMAL: u8 = 1;
/// `RNF_ROOT`: the leaf is a root leaf of its tree (or the top node).
pub const RNF_ROOT: u8 = 2;
/// `RNF_ACTIVE`: the node is in a tree.
pub const RNF_ACTIVE: u8 = 4;

/// `KEYLEN_LIMIT`: the longest key a tree may be made for (`rn_init`'s `KASSERT`).
const KEYLEN_LIMIT: usize = 64;

/// A link's `NULL`: no node, no annotation, no mask.
const NONE: u32 = u32::MAX;
/// The tag of a mask reference (a leaf of the tree's mask tree): masks are objects of their
/// own in the C, never equal to a node.
const MASK: u32 = 1 << 31;

/// The left root leaf (`rnh_nodes[0]`, the key of zeros), the top node (`rnh_nodes[1]`,
/// `rnh_treetop`) and the right root leaf (`rnh_nodes[2]`, the key of ones): slots 0 and 1 of
/// every tree, the mask tree's included.
const ROOT_LEFT: u32 = 0;
const TOP: u32 = 1;
const ROOT_RIGHT: u32 = 2;

/// A full chunk of a `Slab`: `1 << CHUNK_SHIFT` elements. The first chunks are smaller (8, 8,
/// 16 and 32 elements: the capacity doubles up to a full chunk), so a small tree (an export
/// list of a few hosts) takes a few hundred bytes.
const CHUNK_SHIFT: u32 = 6;
const CHUNK: usize = 1 << CHUNK_SHIFT;
/// The first chunk's elements.
const FIRST: usize = 8;
/// The most elements a `Slab` holds: node indices stay below the `MASK` tag.
const SLAB_LIMIT: usize = 1 << 29;

/// `struct radix_node`: a leaf or an internal node. The C's union is two shared words: `lm`
/// is an internal node's `rn_l` and a leaf's `rn_mask` (a mask reference), `rd` an internal
/// node's `rn_r` and a leaf's `rn_dupedkey`. A leaf's `rn_key` is its slot's key.
#[derive(Clone, Copy)]
struct RadixNode {
    /// `rn_mklist`: the annotations of the routes in the subtree (a [`RadixMask`] index).
    mklist: u32,
    /// `rn_p`: the parent (the previous leaf, in a duplicated key chain).
    p: u32,
    /// `rn_b`: the bit tested; a leaf's is `-1 - index(netmask)`.
    b: i16,
    /// `rn_bmask`: the tested bit within its byte.
    bmask: u8,
    /// `rn_flags`.
    flags: u8,
    /// `rn_off`: the byte the tested bit is in.
    off: u32,
    /// `rn_l` (internal node) or `rn_mask` (leaf).
    lm: u32,
    /// `rn_r` (internal node) or `rn_dupedkey` (leaf).
    rd: u32,
}

impl RadixNode {
    /// A zeroed node, as the C's `M_ZERO` and `PR_ZERO` pairs are.
    const fn new() -> Self {
        Self {
            mklist: NONE,
            p: NONE,
            b: 0,
            bmask: 0,
            flags: 0,
            off: 0,
            lm: NONE,
            rd: NONE,
        }
    }

    /// `rn_l`.
    #[inline]
    fn l(&self) -> u32 {
        self.lm
    }

    /// `rn_r`.
    #[inline]
    fn r(&self) -> u32 {
        self.rd
    }

    /// `rn_mask`: a mask reference, or [`NONE`] for a host route.
    #[inline]
    fn mask(&self) -> u32 {
        self.lm
    }

    /// `rn_dupedkey`: the next route of the same key.
    #[inline]
    fn dupedkey(&self) -> u32 {
        self.rd
    }
}

/// `struct radix_mask`: the annotation of a route on an internal node above it. `rmu` is the
/// C's union: `rm_mask` (a mask reference) or, for a normal route, `rm_leaf` (its leaf, read
/// through [`RadixTree::rm_leaf`], which checks `generation`).
#[derive(Clone, Copy)]
struct RadixMask {
    /// `rm_b`: the route's `rn_b`.
    b: i16,
    /// `rm_flags`: the route's flags.
    flags: u8,
    /// `rm_mklist`: the next annotation (the free list's link while vacant).
    mklist: u32,
    /// `rm_mask` or `rm_leaf`.
    rmu: u32,
    /// The generation of `rm_leaf`'s slot when the annotation took it.
    generation: u32,
    /// `rm_refs`: the further routes sharing it.
    refs: i32,
}

/// A route's slot: the C's `treenodes[2]`, the key the leaf holds (`rn_key`, `K` bytes), the
/// value the C's entry would carry around the pair, and the generation [`Leaf`] checks.
struct Slot<V, const K: usize> {
    /// `[0]` the leaf, `[1]` the internal node made with it.
    nodes: [RadixNode; 2],
    /// The leaf's key (the mask tree's: the normalised mask).
    key: [u8; K],
    /// The route's value (`None` for the roots, the mask tree and a vacant slot).
    value: Option<V>,
    /// The tree's generation when the route was added (0: none).
    generation: u32,
    /// The free list's link while vacant.
    link: u32,
}

/// The storage of a tree's slots, its mask tree's or its annotations (D-3): chunks of 8, 8,
/// 16 and 32 elements, then of `CHUNK` (64), each one `malloc(9)` block of the tree's type
/// taken `M_NOWAIT`, behind a directory of chunk handles that doubles when full (the handles
/// move, the elements do not); element `i` is `dir[c][o]` for [`locate`]'s `(c, o)` (`i >> 6`
/// plus 3 and `i & 63` from 64 on). Vacant elements form a free list. The one place that
/// knows the layout, so a flat array is a change of this type alone (D-13).
struct Slab<E> {
    /// The chunk handles (`None` until the first element).
    dir: Option<MArray<Option<MArray<E>>>>,
    /// The chunks in use, from the directory's first entry.
    chunks: usize,
    /// The first vacant element.
    free: u32,
    /// The `malloc(9)` type of the directory and the chunks.
    mtype: i32,
}

impl<E> Slab<E> {
    /// No element, nothing allocated.
    const fn new(mtype: i32) -> Self {
        Self {
            dir: None,
            chunks: 0,
            free: NONE,
            mtype,
        }
    }

    /// No chunk.
    fn is_empty(&self) -> bool {
        self.chunks == 0
    }

    /// Element `i`, `None` past the chunks.
    #[inline]
    fn try_get(&self, i: u32) -> Option<&E> {
        let (c, o) = locate(i as usize);
        self.dir.as_ref()?.get(c)?.as_ref()?.get(o)
    }

    /// Element `i`, which a link of the tree names: `panic(9)` past the chunks (a broken
    /// tree).
    #[inline]
    fn get(&self, i: u32) -> &E {
        match self.try_get(i) {
            Some(e) => e,
            None => kpanic(format_args!("radix: no element {i}")),
        }
    }

    /// Element `i`, to change.
    #[inline]
    fn get_mut(&mut self, i: u32) -> &mut E {
        let (c, o) = locate(i as usize);
        let e = self
            .dir
            .as_mut()
            .and_then(|d| d.get_mut(c))
            .and_then(Option::as_mut)
            .and_then(|k| k.get_mut(o));
        match e {
            Some(e) => e,
            None => kpanic(format_args!("radix: no element {i}")),
        }
    }

    /// Every chunk and the directory freed (the elements dropped).
    fn clear(&mut self) {
        self.dir = None;
        self.chunks = 0;
        self.free = NONE;
    }
}

impl<E: SlabItem> Slab<E> {
    /// A vacant element, taken off the free list (a chunk more when it is empty): `None`
    /// when `malloc(M_NOWAIT)` has no memory.
    fn alloc(&mut self) -> Option<u32> {
        if self.free == NONE {
            self.grow()?;
        }
        let i = self.free;
        let link = self.get_mut(i).link();
        let next = *link;
        *link = NONE;
        self.free = next;
        Some(i)
    }

    /// Element `i` made vacant again (its old value dropped) and put on the free list.
    fn release(&mut self, i: u32) {
        let next = self.free;
        let e = self.get_mut(i);
        *e = E::vacant();
        *e.link() = next;
        self.free = i;
    }

    /// One more chunk, its elements on the free list; the directory doubled first when full.
    fn grow(&mut self) -> Option<()> {
        let n = self.chunks;
        let (base, len) = (chunk_base(n), chunk_len(n));
        if base + len > SLAB_LIMIT {
            return None;
        }
        let cap = self.dir.as_ref().map_or(0, |d| d.len());
        if n == cap {
            let mut old = self.dir.take();
            // `MArray::new` asks `malloc` before it runs the closure, so a failure leaves the
            // old directory whole.
            let grown = MArray::new(self.mtype, M_NOWAIT, (cap * 2).max(4), |i| {
                old.as_mut()
                    .and_then(|o| o.get_mut(i))
                    .and_then(Option::take)
            });
            match grown {
                Some(d) => self.dir = Some(d),
                None => {
                    self.dir = old;
                    return None;
                }
            }
        }
        let next = self.free;
        let chunk = MArray::new(self.mtype, M_NOWAIT, len, |j| {
            let mut e = E::vacant();
            *e.link() = if j + 1 < len {
                (base + j + 1) as u32
            } else {
                next
            };
            e
        })?;
        *self.dir.as_mut().and_then(|d| d.get_mut(n))? = Some(chunk);
        self.chunks = n + 1;
        self.free = base as u32;
        Some(())
    }
}

impl<V, const K: usize> Slab<Slot<V, K>> {
    /// Node `x` (`2 * slot` the leaf, `2 * slot + 1` the internal node).
    #[inline]
    fn n(&self, x: u32) -> &RadixNode {
        &self.get(x >> 1).nodes[(x & 1) as usize]
    }

    /// Node `x`, to change.
    #[inline]
    fn nm(&mut self, x: u32) -> &mut RadixNode {
        &mut self.get_mut(x >> 1).nodes[(x & 1) as usize]
    }

    /// The key of leaf `x` (`rn_key`).
    #[inline]
    fn key(&self, x: u32) -> &[u8; K] {
        &self.get(x >> 1).key
    }
}

/// A route of a [`RadixTree`] (the C's leaf, `struct radix_node *`): its slot and the
/// generation the slot had when the route was added, checked at every use, so a handle of a
/// removed route names nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Leaf {
    slot: u32,
    generation: u32,
}

/// `struct radix_node_head` and the nodes it reaches: a radix tree of routes keyed by `K`-byte
/// keys (the `keylen` its C caller gives `rn_init`: 16 for the exports, 28 for pf, 48 for the
/// policy database), compared from byte `off` (`rn_inithead`'s), each route carrying a `T`.
pub struct RadixTree<T, const K: usize> {
    /// `rn_inithead`'s `off`: the first byte compared.
    off: usize,
    /// The route slots (`rnh_nodes` in slots 0 and 1).
    nodes: Slab<Slot<T, K>>,
    /// The tree's mask tree (`mask_rnhead`): the normalised masks, its own roots in 0 and 1.
    masks: Slab<Slot<(), K>>,
    /// The annotation records (`rtmask_pool`).
    marks: Slab<RadixMask>,
    /// The routes in the tree (0: nothing allocated).
    routes: usize,
    /// The last generation a route took.
    generation: u32,
    /// D-20's repairs made so far (a test's count).
    #[cfg(any(test, feature = "doubles"))]
    repairs: u32,
}

impl<T, const K: usize> RadixTree<T, K> {
    /// `rn_init(K)` and `rn_inithead(&head, off)`: an empty tree comparing keys from byte
    /// `off`, allocating `malloc(9)` blocks of type `mtype` once it holds a route. Allocates
    /// nothing.
    pub const fn new(off: usize, mtype: i32) -> Self {
        assert!(
            K >= 2 && K <= KEYLEN_LIMIT && off < K,
            "rn_init: key length"
        );
        Self {
            off,
            nodes: Slab::new(mtype),
            masks: Slab::new(mtype),
            marks: Slab::new(mtype),
            routes: 0,
            generation: 0,
            #[cfg(any(test, feature = "doubles"))]
            repairs: 0,
        }
    }

    /// No route.
    pub fn is_empty(&self) -> bool {
        self.routes == 0
    }

    /// `rn_match(v, head)`: the route `v` falls under, the most specific one first.
    pub fn matches(&self, v: &[u8]) -> Option<&T> {
        if self.routes == 0 {
            return None;
        }
        let v = key_of::<K>(v);
        self.value_at(rn_match(self, &v))
    }

    /// `rn_lookup(v, m, head)`: with a mask, the route of exactly that key and mask (`v`'s
    /// match, then its duplicated key chain); without, `rn_match`.
    pub fn lookup(&self, v: &[u8], m: Option<&[u8]>) -> Option<&T> {
        if self.routes == 0 {
            return None;
        }
        let v = key_of::<K>(v);
        let m = m.map(key_of::<K>);
        self.value_at(rn_lookup(self, &v, m.as_ref()))
    }

    /// `rn_addroute(v, m, head, treenodes, prio)`: the route of key `v` and mask `m` (none: a
    /// host route) added with `value`. `Err` gives the value back where the C returns `NULL`:
    /// the key and mask are there already, or no memory (`M_NOWAIT`) for the mask or the slot.
    pub fn add(&mut self, v: &[u8], m: Option<&[u8]>, value: T) -> Result<Leaf, T> {
        if !self.inithead() {
            return Err(value);
        }
        // A refused add keeps the head (the C's stays with its owner).
        let Some(slot) = self.nodes.alloc() else {
            return Err(value);
        };
        let v = key_of::<K>(v);
        let m = m.map(key_of::<K>);
        // The generation first: an annotation may name the leaf during the add.
        self.generation = match self.generation.wrapping_add(1) {
            0 => 1,
            g => g,
        };
        let generation = self.generation;
        let s = self.nodes.get_mut(slot);
        s.key = v;
        s.generation = generation;
        let tt = rn_addroute(self, &v, m.as_ref(), slot);
        if tt == NONE {
            self.nodes.release(slot);
            return Err(value);
        }
        self.nodes.get_mut(slot).value = Some(value);
        self.routes += 1;
        Ok(Leaf { slot, generation })
    }

    /// `rn_delete(v, m, head, rn)`: the route of key `v` and mask `m` (none: the head of the
    /// key's chain) taken out; its value.
    pub fn delete(&mut self, v: &[u8], m: Option<&[u8]>) -> Option<T> {
        if self.routes == 0 {
            return None;
        }
        let v = key_of::<K>(v);
        let m = m.map(key_of::<K>);
        let tt = rn_delete(self, &v, m.as_ref());
        self.take_route(tt)
    }

    /// The first route in `rn_walktree`'s order (key order, a key's routes in their chain's
    /// order).
    pub fn first(&self) -> Option<Leaf> {
        if self.routes == 0 {
            return None;
        }
        // First time through node, go left.
        let mut rn = TOP;
        while self.n(rn).b >= 0 {
            rn = self.n(rn).l();
        }
        // The left root leaf heads the first chain; its routes are the duplicated keys.
        match self.n(rn).dupedkey() {
            NONE => self.chain_head(rn_walktree(self, rn)),
            d => Some(self.leaf(d)),
        }
    }

    /// The route after `at` in `rn_walktree`'s order; `None` after the last, or for a handle
    /// of a removed route. Take it before removing `at`, as the C computes the successor
    /// before calling `f`.
    pub fn next(&self, at: Leaf) -> Option<Leaf> {
        let tt = self.check(at)?;
        let d = self.n(tt).dupedkey();
        if d != NONE {
            return Some(self.leaf(d));
        }
        // The chain's head: a leaf whose parent is an internal node.
        let mut base = tt;
        while self.n(self.n(base).p).b < 0 {
            base = self.n(base).p;
        }
        self.chain_head(rn_walktree(self, base))
    }

    /// The value of the route `at`.
    pub fn get(&self, at: Leaf) -> Option<&T> {
        let tt = self.check(at)?;
        self.value_at(tt)
    }

    /// The route `at` taken out of the tree (`rn_delete` of its key and mask); its value.
    pub fn remove(&mut self, at: Leaf) -> Option<T> {
        let tt = self.check(at)?;
        let mut saved_tt = tt;
        while self.n(self.n(saved_tt).p).b < 0 {
            saved_tt = self.n(saved_tt).p;
        }
        let tt = rn_delete_leaf(self, saved_tt, tt);
        self.take_route(tt)
    }

    /// `rn_inithead` and `rn_initmask`: the roots of the tree and of its mask tree, made now
    /// when the tree has none (a new tree has none, and a tree gives them back when its last
    /// route is deleted; a refused add keeps them); `false` when `malloc(M_NOWAIT)` has no
    /// memory for them, nothing kept. [`add`] calls it; a caller that answers the C's
    /// `rn_inithead` failure apart (`ENOBUFS`, the export list's) calls it first.
    ///
    /// [`add`]: RadixTree::add
    pub fn inithead(&mut self) -> bool {
        if !self.nodes.is_empty() {
            return true;
        }
        if rn_inithead0(&mut self.nodes, self.off).is_some()
            && rn_inithead0(&mut self.masks, 0).is_some()
        {
            return true;
        }
        self.drop_if_empty();
        false
    }

    /// With no route left, every chunk given back (the next add starts the tree again).
    fn drop_if_empty(&mut self) {
        if self.routes == 0 {
            self.nodes.clear();
            self.masks.clear();
            self.marks.clear();
        }
    }

    /// The value of the route leaf `tt` that `rn_delete` unlinked, its slot freed.
    fn take_route(&mut self, tt: u32) -> Option<T> {
        if tt == NONE {
            return None;
        }
        let slot = tt >> 1;
        let value = self.nodes.get_mut(slot).value.take();
        self.nodes.release(slot);
        self.routes -= 1;
        self.drop_if_empty();
        value
    }

    /// The leaf node of the live route `at`.
    fn check(&self, at: Leaf) -> Option<u32> {
        let s = self.nodes.try_get(at.slot)?;
        (s.generation == at.generation && s.value.is_some()).then_some(at.slot << 1)
    }

    /// The handle of route leaf `x`.
    fn leaf(&self, x: u32) -> Leaf {
        Leaf {
            slot: x >> 1,
            generation: self.nodes.get(x >> 1).generation,
        }
    }

    /// The first route of the chain headed by leaf `head` (`None` at the right root, where
    /// the C's walk ends; a root leaf's routes are its duplicated keys).
    fn chain_head(&self, head: u32) -> Option<Leaf> {
        (self.n(head).flags & RNF_ROOT == 0).then(|| self.leaf(head))
    }

    /// The value of leaf `x` (none for `NONE`).
    fn value_at(&self, x: u32) -> Option<&T> {
        if x == NONE {
            return None;
        }
        self.nodes.get(x >> 1).value.as_ref()
    }

    /// Node `x`.
    #[inline]
    fn n(&self, x: u32) -> &RadixNode {
        self.nodes.n(x)
    }

    /// Node `x`, to change.
    #[inline]
    fn nm(&mut self, x: u32) -> &mut RadixNode {
        self.nodes.nm(x)
    }

    /// The bytes of the mask reference `r` (its mask tree leaf's key).
    #[inline]
    fn mk(&self, r: u32) -> &[u8; K] {
        self.masks.key(r & !MASK)
    }

    /// Annotation `m`.
    #[inline]
    fn rm(&self, m: u32) -> &RadixMask {
        self.marks.get(m)
    }

    /// Annotation `m`, to change.
    #[inline]
    fn rmm(&mut self, m: u32) -> &mut RadixMask {
        self.marks.get_mut(m)
    }

    /// `m->rm_leaf` of a normal route's annotation `m`: its leaf, or [`NONE`] when that route
    /// has left the tree since (radix.c's dangling `rm_leaf`, the module's deviations).
    #[inline]
    fn rm_leaf(&self, m: u32) -> u32 {
        let rm = self.rm(m);
        match self.nodes.try_get(rm.rmu >> 1) {
            Some(s) if rm.rmu & (MASK | 1) == 0 && s.generation == rm.generation => rm.rmu,
            _ => NONE,
        }
    }

    /// `m->rm_leaf = leaf`, with the leaf's generation.
    fn set_rm_leaf(&mut self, m: u32, leaf: u32) {
        let generation = self.nodes.get(leaf >> 1).generation;
        let rm = self.rmm(m);
        rm.rmu = leaf;
        rm.generation = generation;
    }

    /// D-20's repair made once more (`rn_del_radix_mask`): counted for the tests.
    #[inline]
    fn note_repair(&mut self) {
        #[cfg(any(test, feature = "doubles"))]
        {
            self.repairs += 1;
        }
    }

    /// The deletes D-20 repaired so far: where radix.c would have left a normal annotation
    /// naming a freed route. The differential test against `bsd`'s tree compares the two
    /// only until the first, since the C then reads freed memory.
    #[cfg(any(test, feature = "doubles"))]
    pub fn repairs(&self) -> u32 {
        self.repairs
    }

    /// The annotations of normal routes whose leaf has left the tree (radix.c's dangling
    /// `rm_leaf`): none, D-20's repair keeps them from arising; the property tests check it.
    #[cfg(any(test, feature = "doubles"))]
    pub fn dangling_annotations(&self) -> usize {
        let mut n = 0;
        let mut i = 0;
        while let Some(rm) = self.marks.try_get(i) {
            if rm.flags & RNF_NORMAL != 0 && rm.rmu != NONE && self.rm_leaf(i) == NONE {
                n += 1;
            }
            i += 1;
        }
        n
    }

    /// What the `struct radix_mask **mp` of the C's list walks points at.
    fn link(&self, l: MkLink) -> u32 {
        match l {
            MkLink::Node(x) => self.n(x).mklist,
            MkLink::Mark(m) => self.rm(m).mklist,
        }
    }

    /// `*mp = v`.
    fn set_link(&mut self, l: MkLink, v: u32) {
        match l {
            MkLink::Node(x) => self.nm(x).mklist = v,
            MkLink::Mark(m) => self.rmm(m).mklist = v,
        }
    }
}

/// The C's `struct radix_mask **mp`: a node's `rn_mklist` or an annotation's `rm_mklist`.
#[derive(Clone, Copy)]
enum MkLink {
    Node(u32),
    Mark(u32),
}

/// What a `Slab` holds: how a vacant element looks and where its free-list link is.
trait SlabItem {
    /// A vacant element (the C's zeroed memory).
    fn vacant() -> Self;
    /// The free list's link of a vacant element.
    fn link(&mut self) -> &mut u32;
}

impl SlabItem for RadixMask {
    fn vacant() -> Self {
        Self {
            b: 0,
            flags: 0,
            mklist: NONE,
            rmu: NONE,
            generation: 0,
            refs: 0,
        }
    }

    fn link(&mut self) -> &mut u32 {
        &mut self.mklist
    }
}

impl<V, const K: usize> SlabItem for Slot<V, K> {
    fn vacant() -> Self {
        Self {
            nodes: [RadixNode::new(), RadixNode::new()],
            key: [0; K],
            value: None,
            generation: 0,
            link: NONE,
        }
    }

    fn link(&mut self) -> &mut u32 {
        &mut self.link
    }
}

/// A key or a mask as the tree holds it: the slice's first `K` bytes, zeros past a shorter
/// slice (D-7).
fn key_of<const K: usize>(v: &[u8]) -> [u8; K] {
    let mut k = [0; K];
    let n = v.len().min(K);
    k[..n].copy_from_slice(&v[..n]);
    k
}

/// The chunk and the offset in it of a `Slab`'s element `i`: chunk 0 holds 0..8, chunk 1
/// 8..16, chunk 2 16..32, chunk 3 32..64, then 64 elements a chunk.
#[inline]
fn locate(i: usize) -> (usize, usize) {
    if i >= CHUNK {
        ((i >> CHUNK_SHIFT) + 3, i & (CHUNK - 1))
    } else if i < FIRST {
        (0, i)
    } else {
        let top = (usize::BITS - 1 - i.leading_zeros()) as usize;
        (top - 2, i - (1 << top))
    }
}

/// The first element of a `Slab`'s chunk `c`.
fn chunk_base(c: usize) -> usize {
    match c {
        0 => 0,
        1..=3 => (FIRST / 2) << c,
        _ => (c - 3) << CHUNK_SHIFT,
    }
}

/// The elements of a `Slab`'s chunk `c`.
fn chunk_len(c: usize) -> usize {
    match c {
        0 => FIRST,
        1..=3 => (FIRST / 2) << c,
        _ => CHUNK,
    }
}

/// `SALEN(sa)`: the length byte, at most `K` (D-7).
#[inline]
fn salen<const K: usize>(k: &[u8; K]) -> usize {
    usize::from(k[0]).min(K)
}

/// `rn_search(v, head)`: the leaf the bits of `v` lead to from `head`.
fn rn_search<V, const K: usize>(t: &Slab<Slot<V, K>>, v: &[u8; K], head: u32) -> u32 {
    let mut x = head;
    loop {
        let n = t.n(x);
        if n.b < 0 {
            return x;
        }
        x = if n.bmask & v[n.off as usize] != 0 {
            n.r()
        } else {
            n.l()
        };
    }
}

/// `rn_search_m(v, head, m)`: as [`rn_search`], a bit taken only where the mask `m` has it.
fn rn_search_m<V, const K: usize>(
    t: &Slab<Slot<V, K>>,
    v: &[u8; K],
    head: u32,
    m: &[u8; K],
) -> u32 {
    let mut x = head;
    loop {
        let n = t.n(x);
        if n.b < 0 {
            return x;
        }
        let at = n.off as usize;
        x = if n.bmask & m[at] != 0 && n.bmask & v[at] != 0 {
            n.r()
        } else {
            n.l()
        };
    }
}

/// `rn_refines(m, n)`: mask `m` is strictly more specific than mask `n` (every bit of `n` is
/// in `m`, and `m` has one more).
fn rn_refines<const K: usize>(m: &[u8; K], n: &[u8; K]) -> bool {
    let (nlen, mlen) = (salen(n), salen(m));
    let longer = nlen as isize - mlen as isize;
    // The bytes both have, from 1 (the length byte is not compared).
    let lim = if longer > 0 { mlen } else { nlen };
    let mut equal = true;
    let mut i = 1;
    while i < lim {
        if n[i] & !m[i] != 0 {
            return false;
        }
        if n[i] != m[i] {
            equal = false;
        }
        i += 1;
    }
    // `n`'s bytes past `m` must be zero.
    let mut j = i;
    while j < nlen {
        if n[j] != 0 {
            return false;
        }
        j += 1;
    }
    // Equal so far: `m` refines `n` when its own longer tail has a bit.
    if equal && longer < 0 {
        let end = i + longer.unsigned_abs();
        if (i..end).any(|k| m.get(k).is_some_and(|&b| b != 0)) {
            return true;
        }
    }
    !equal
}

/// `rn_lookup(v_arg, m_arg, head)`: a perfect match when a mask is given, else `rn_match`.
fn rn_lookup<T, const K: usize>(h: &RadixTree<T, K>, v: &[u8; K], m: Option<&[u8; K]>) -> u32 {
    let mut netmask = NONE;
    if let Some(m) = m {
        match rn_findmask(&h.masks, m, h.n(TOP).off as usize) {
            Ok(tm) => netmask = MASK | tm,
            Err(_) => return NONE,
        }
    }
    let mut x = rn_match(h, v);
    if x != NONE && netmask != NONE {
        while x != NONE && h.n(x).mask() != netmask {
            x = h.n(x).dupedkey();
        }
    }
    // Never return internal nodes to the upper layer.
    if x != NONE && h.n(x).flags & RNF_ROOT != 0 {
        return NONE;
    }
    x
}

/// `rn_satisfies_leaf(trial, leaf, skip)`: `trial` equals the leaf's key under its mask
/// (`mask`, all ones for a host route's [`NONE`]), from byte `skip`, for the shorter of their
/// lengths. The caller hands the leaf's slot's key and mask, which it has read already.
fn rn_satisfies_leaf<T, const K: usize>(
    h: &RadixTree<T, K>,
    trial: &[u8; K],
    key: &[u8; K],
    mask: u32,
    skip: usize,
) -> bool {
    let mut length = salen(trial).min(salen(key));
    let mask = match mask {
        NONE => None,
        r => {
            let m = h.mk(r);
            length = length.min(salen(m));
            Some(m)
        }
    };
    (skip..length).all(|i| (trial[i] ^ key[i]) & mask.map_or(0xff, |m| m[i]) == 0)
}

/// `rn_match(v_arg, head)`: the route `v` falls under, or [`NONE`]. Each leaf's slot is read
/// once (its node, key and mask together), each annotation's record once.
fn rn_match<T, const K: usize>(h: &RadixTree<T, K>, v: &[u8; K]) -> u32 {
    let top = TOP;
    let off = h.n(top).off as usize;
    let mut t = rn_search(&h.nodes, v, top);
    let s = h.nodes.get(t >> 1);
    let tn = s.nodes[0];
    // An exact match as a host, or how many bits match, for the normal masks' short cut. The
    // mask's length is enough: a match under it is a genuine match of the most specific
    // leaf, and a mismatch within it would be one at any length.
    let vlen = match tn.mask() {
        NONE => salen(v),
        r => salen(h.mk(r)),
    };
    let tkey = &s.key;
    let Some(matched_off) = (off..vlen).find(|&i| v[i] != tkey[i]) else {
        // Explicitly asked for the default: the root leaf's first route.
        if tn.flags & RNF_ROOT != 0 {
            t = tn.dupedkey();
        }
        kassert!(t == NONE || h.n(t).flags & RNF_ROOT == 0);
        return t;
    };
    // The first bit that differs.
    let b = (matched_off << 3) as i32 + (v[matched_off] ^ tkey[matched_off]).leading_zeros() as i32;
    let rn_b = -1 - b;
    // A host route of the key comes first in its chain.
    let saved_t = t;
    if tn.mask() == NONE {
        t = tn.dupedkey();
    }
    while t != NONE {
        // Not a host, but the leaf we wound up at may be a route to a net.
        let s = h.nodes.get(t >> 1);
        let n = &s.nodes[0];
        if n.flags & RNF_NORMAL != 0 {
            if rn_b <= i32::from(n.b) {
                kassert!(n.flags & RNF_ROOT == 0);
                return t;
            }
        } else if rn_satisfies_leaf(h, v, &s.key, n.mask(), matched_off) {
            kassert!(n.flags & RNF_ROOT == 0);
            return t;
        }
        t = n.dupedkey();
    }
    t = saved_t;
    // Start searching up the tree.
    loop {
        t = h.n(t).p;
        let tn = *h.n(t);
        let mut m = tn.mklist;
        while m != NONE {
            let rm = *h.rm(m);
            if rm.flags & RNF_NORMAL != 0 {
                if rn_b <= i32::from(rm.b) {
                    // The annotation's route (its generation checked: never a gone one, the
                    // deviations' D-20).
                    let leaf = h.rm_leaf(m);
                    if leaf != NONE {
                        kassert!(h.n(leaf).flags & RNF_ROOT == 0);
                        return leaf;
                    }
                }
            } else {
                let off = (tn.off as usize).min(matched_off);
                let mut x = rn_search_m(&h.nodes, v, t, h.mk(rm.rmu));
                while x != NONE {
                    let s = h.nodes.get(x >> 1);
                    let n = &s.nodes[0];
                    if n.mask() == rm.rmu {
                        if rn_satisfies_leaf(h, v, &s.key, n.mask(), off) {
                            kassert!(n.flags & RNF_ROOT == 0);
                            return x;
                        }
                        break;
                    }
                    x = n.dupedkey();
                }
            }
            m = rm.mklist;
        }
        if t == top {
            return NONE;
        }
    }
}

/// `rn_newpair(v, b, nodes)`: slot `slot`'s leaf (its key the slot's) under its internal
/// node, which tests bit `b`; the internal node.
fn rn_newpair<V, const K: usize>(t: &mut Slab<Slot<V, K>>, slot: u32, b: i32) -> u32 {
    let (tt, x) = (slot << 1, (slot << 1) | 1);
    let [leaf, node] = &mut t.get_mut(slot).nodes;
    node.b = b as i16;
    node.bmask = 0x80 >> (b & 7);
    node.lm = tt;
    node.off = (b >> 3) as u32;
    leaf.b = -1;
    leaf.p = x;
    leaf.flags = RNF_ACTIVE;
    node.flags = RNF_ACTIVE;
    x
}

/// `rn_insert(v_arg, head, &dupentry, nodes)`: slot `slot`'s leaf linked where `v` (the
/// slot's key) belongs; the leaf and `false`, or the leaf already holding the key and
/// `true` (nothing changed then).
fn rn_insert<V, const K: usize>(t: &mut Slab<Slot<V, K>>, v: &[u8; K], slot: u32) -> (u32, bool) {
    let top = TOP;
    let off = t.n(top).off as usize;
    let found = rn_search(t, v, top);
    // The first bit at which `v` and the key found differ.
    let vlen = salen(v);
    let tkey = t.key(found);
    let Some(i) = (off..vlen).find(|&i| v[i] != tkey[i]) else {
        return (found, true);
    };
    let b = (i << 3) as i32 + (v[i] ^ tkey[i]).leading_zeros() as i32;
    // Down to the first node testing a later bit (or a leaf).
    let mut x = top;
    let mut p;
    loop {
        p = x;
        let n = t.n(x);
        x = if v[n.off as usize] & n.bmask != 0 {
            n.r()
        } else {
            n.l()
        };
        let xb = t.n(x).b;
        if xb < 0 || b <= i32::from(xb) {
            break;
        }
    }
    let new = rn_newpair(t, slot, b);
    let tt = t.n(new).l();
    let pn = *t.n(p);
    if v[pn.off as usize] & pn.bmask == 0 {
        t.nm(p).lm = new;
    } else {
        t.nm(p).rd = new;
    }
    t.nm(x).p = new;
    t.nm(new).p = p;
    let nn = *t.n(new);
    if v[nn.off as usize] & nn.bmask == 0 {
        t.nm(new).rd = x;
    } else {
        t.nm(new).rd = tt;
        t.nm(new).lm = x;
    }
    (tt, false)
}

/// `rn_addmask(n_arg, 1, skip)`, the search half of [`rn_addmask`]: `Ok` the mask tree leaf of
/// `n` normalised (the zero mask is the left root, as the C's `mask_rnhead->rnh_nodes`), or
/// `Err` the normalised key and its length when the tree does not hold it.
fn rn_findmask<const K: usize>(
    masks: &Slab<Slot<(), K>>,
    n: &[u8; K],
    skip: usize,
) -> Result<u32, ([u8; K], usize)> {
    let skip = skip.max(1);
    let mlen = salen(n);
    if mlen <= skip {
        return Ok(ROOT_LEFT);
    }
    // The bytes before `skip` are all ones; the mask's own from there.
    let mut key = [0u8; K];
    key[1..skip].fill(0xff);
    key[skip..mlen].copy_from_slice(&n[skip..mlen]);
    // Trim trailing zeroes.
    let mut end = mlen;
    while end > 0 && key[end - 1] == 0 {
        end -= 1;
    }
    let mlen = end;
    if mlen <= skip {
        return Ok(ROOT_LEFT);
    }
    key[0] = mlen as u8;
    let tm = rn_search(masks, &key, TOP);
    if masks.key(tm)[..mlen] == key[..mlen] {
        Ok(tm)
    } else {
        Err((key, mlen))
    }
}

/// `rn_addmask(n_arg, search, skip)`: the mask tree leaf of `n` normalised (its key from byte
/// `skip` on, ones before, trailing zeros trimmed), added when it is not there and `search`
/// is not set, with its index and its `RNF_NORMAL`; [`NONE`] when absent (`search`) or when
/// no memory is left.
fn rn_addmask<const K: usize>(
    masks: &mut Slab<Slot<(), K>>,
    n: &[u8; K],
    search: bool,
    skip: usize,
) -> u32 {
    let skip = skip.max(1);
    let (key, mlen) = match rn_findmask(masks, n, skip) {
        Ok(tm) => return tm,
        Err(_) if search => return NONE,
        Err(k) => k,
    };
    let Some(slot) = masks.alloc() else {
        return NONE;
    };
    masks.get_mut(slot).key = key;
    let (tm, duplicated) = rn_insert(masks, &key, slot);
    if duplicated {
        log(
            LOG_ERR,
            format_args!("rn_addmask: mask impossibly already in tree\n"),
        );
        masks.release(slot);
        return tm;
    }
    // The mask's index, and whether it is normal.
    let mut c = skip;
    while c < mlen && key[c] == 0xff {
        c += 1;
    }
    let mut b = 0;
    let mut isnormal = true;
    if c != mlen {
        b = (!key[c]).leading_zeros() as usize;
        if key[c] != !(0xffu8 >> b) || c != mlen - 1 {
            isnormal = false;
        }
    }
    b += c << 3;
    let node = masks.nm(tm);
    node.b = (-1 - b as i32) as i16;
    if isnormal {
        node.flags |= RNF_NORMAL;
    }
    tm
}

/// `rn_lexobetter(m_arg, n_arg)`: an arbitrary but fixed order of normalised masks, the
/// longer first, then by their first differing byte.
fn rn_lexobetter<const K: usize>(mp: &[u8; K], np: &[u8; K]) -> bool {
    let (ml, nl) = (salen(mp), salen(np));
    if ml != nl {
        return ml > nl;
    }
    mp[..ml] > np[..ml]
}

/// `rn_new_radix_mask(tt, next)`: an annotation of route `tt` ahead of `next`; [`NONE`] when
/// no memory is left ("Mask for route not entered").
fn rn_new_radix_mask<T, const K: usize>(h: &mut RadixTree<T, K>, tt: u32, next: u32) -> u32 {
    let Some(m) = h.marks.alloc() else {
        log(LOG_ERR, format_args!("Mask for route not entered\n"));
        return NONE;
    };
    let n = *h.n(tt);
    *h.rmm(m) = RadixMask {
        b: n.b,
        flags: n.flags,
        mklist: next,
        rmu: n.mask(),
        generation: 0,
        refs: 0,
    };
    if n.flags & RNF_NORMAL != 0 {
        h.set_rm_leaf(m, tt);
    }
    h.nm(tt).mklist = m;
    m
}

/// `rn_lift_node(t)`: the highest internal node route `t`'s annotation may hang on, or
/// [`NONE`] when it cannot rise above its chain's head.
fn rn_lift_node<V, const K: usize>(n: &Slab<Slot<V, K>>, t: u32) -> u32 {
    let b = -1 - i32::from(n.n(t).b);
    // Rewind a duplicated key chain to its head.
    let mut t = t;
    while n.n(t).b < 0 {
        t = n.n(t).p;
    }
    // Can't lift node above head of dupedkey list, give up.
    if b > i32::from(n.n(t).b) {
        return NONE;
    }
    let mut x;
    loop {
        x = t;
        t = n.n(t).p;
        if !(b <= i32::from(n.n(t).b) && x != t) {
            return x;
        }
    }
}

/// `rn_add_radix_mask(tt, keyduplicated)`: route `tt`'s annotation on the highest node it can
/// rise to, sorted by index, then from the most specific mask (the duplicated keys' order).
fn rn_add_radix_mask<T, const K: usize>(h: &mut RadixTree<T, K>, tt: u32, keyduplicated: bool) {
    let b_leaf = h.n(tt).b;
    if h.n(tt).mask() == NONE {
        return; // can't lift at all
    }
    let x = rn_lift_node(&h.nodes, tt);
    if x == NONE {
        return; // didn't lift either
    }
    let netmask = h.n(tt).mask();
    let mut mp = MkLink::Node(x);
    loop {
        let m = h.link(mp);
        if m == NONE {
            break;
        }
        let rm = *h.rm(m);
        if rm.b < b_leaf {
            mp = MkLink::Mark(m);
            continue;
        }
        if rm.b > b_leaf {
            break;
        }
        let mmask = if rm.flags & RNF_NORMAL != 0 {
            let leaf = h.rm_leaf(m);
            if keyduplicated {
                if leaf != NONE && h.n(leaf).p == tt {
                    // the new route is better
                    h.set_rm_leaf(m, tt);
                } else if cfg!(feature = "diagnostic") {
                    let mut t = leaf;
                    while t != NONE && h.n(t).mklist == m {
                        if t == tt {
                            break;
                        }
                        t = h.n(t).dupedkey();
                    }
                    if t == NONE {
                        log(
                            LOG_ERR,
                            format_args!("Non-unique normal route on dupedkey, mask not entered\n"),
                        );
                        return;
                    }
                }
                h.rmm(m).refs += 1;
                h.nm(tt).mklist = m;
                return;
            } else if h.n(tt).flags & RNF_NORMAL != 0 {
                log(
                    LOG_ERR,
                    format_args!("Non-unique normal route, mask not entered\n"),
                );
                return;
            }
            match leaf {
                NONE => NONE,
                l => h.n(l).mask(),
            }
        } else {
            rm.rmu
        };
        if mmask == netmask {
            h.rmm(m).refs += 1;
            h.nm(tt).mklist = m;
            return;
        }
        // A gone route's annotation has no mask to order by: passed over.
        if mmask != NONE
            && (rn_refines(h.mk(netmask), h.mk(mmask)) || rn_lexobetter(h.mk(netmask), h.mk(mmask)))
        {
            break;
        }
        mp = MkLink::Mark(m);
    }
    let next = h.link(mp);
    let new = rn_new_radix_mask(h, tt, next);
    h.set_link(mp, new);
}

/// `rn_add_dupedkey(saved_tt, head, tt, prio)`: route `tt` linked into the chain of the key's
/// leaf `saved_tt`, the host route first, then from the most specific mask; -1 when a route
/// of the same mask is there.
fn rn_add_dupedkey<T, const K: usize>(h: &mut RadixTree<T, K>, saved_tt: u32, tt: u32) -> i32 {
    let netmask = h.n(tt).mask();
    let b_leaf = if netmask != NONE { h.n(tt).b } else { 0 };
    let mut x = saved_tt;
    let mut xp = x;
    while x != NONE {
        let xn = *h.n(x);
        if xn.mask() == netmask {
            return -1;
        }
        if netmask == NONE
            || (xn.mask() != NONE
                && (b_leaf < xn.b // index(netmask) > node
                    || rn_refines(h.mk(netmask), h.mk(xn.mask()))
                    || rn_lexobetter(h.mk(netmask), h.mk(xn.mask()))))
        {
            break;
        }
        xp = x;
        x = xn.dupedkey();
    }
    // A mask not in the chain is placed among the key's routes as in a mask list, which may
    // put it at the head, in the tree in place of the old head.
    let before = x == saved_tt;
    rn_link_dupedkey(&mut h.nodes, tt, xp, before);
    0
}

/// `rn_link_dupedkey(tt, x, before)`: `tt` after `x`, or in place of `x` (the chain's head)
/// when `before`; the chain is doubly linked through `rn_p`.
fn rn_link_dupedkey<V, const K: usize>(t: &mut Slab<Slot<V, K>>, tt: u32, x: u32, before: bool) {
    if before {
        let xp = t.n(x).p;
        if t.n(xp).b > 0 {
            // link in at head of list
            let xn = *t.n(x);
            let node = t.nm(tt);
            node.rd = x;
            node.flags = xn.flags;
            node.p = xp;
            t.nm(x).p = tt;
            if t.n(xp).l() == x {
                t.nm(xp).lm = tt;
            } else {
                t.nm(xp).rd = tt;
            }
        } else {
            t.nm(tt).rd = x;
            t.nm(xp).rd = tt;
            t.nm(tt).p = xp;
            t.nm(x).p = tt;
        }
    } else {
        let d = t.n(x).dupedkey();
        t.nm(tt).rd = d;
        t.nm(x).rd = tt;
        t.nm(tt).p = x;
        if d != NONE {
            t.nm(d).p = tt;
        }
    }
}

/// `rn_fixup_nodes(tt)`: the annotations of the new internal node above the new leaf `tt`,
/// from its sibling (a chain of leaves, or an internal node whose list it takes the head of).
fn rn_fixup_nodes<T, const K: usize>(h: &mut RadixTree<T, K>, tt: u32) {
    let tp = h.n(tt).p;
    let mut x = if h.n(tp).r() == tt {
        h.n(tp).l()
    } else {
        h.n(tp).r()
    };
    let b_leaf = -1 - i32::from(h.n(tp).b);
    if h.n(x).b < 0 {
        // x is a leaf node
        let mut xx = NONE;
        let mut mp = MkLink::Node(tp);
        while x != NONE {
            let xn = *h.n(x);
            if xx != NONE
                && h.n(xx).mklist != NONE
                && h.n(xx).mask() == xn.mask()
                && xn.mklist == NONE
            {
                // multipath route
                let shared = h.n(xx).mklist;
                h.nm(x).mklist = shared;
                h.rmm(shared).refs += 1;
            }
            let xn = *h.n(x);
            if xn.mask() != NONE && i32::from(xn.b) >= b_leaf && xn.mklist == NONE {
                let m = rn_new_radix_mask(h, x, NONE);
                h.set_link(mp, m);
                if m != NONE {
                    mp = MkLink::Mark(m);
                }
            }
            xx = x;
            x = h.n(x).dupedkey();
        }
    } else if h.n(x).mklist != NONE {
        // x is an internal node: skip over masks whose index is > that of new node
        let mut mp = MkLink::Node(x);
        let mut m;
        loop {
            m = h.link(mp);
            if m == NONE || i32::from(h.rm(m).b) >= b_leaf {
                break;
            }
            mp = MkLink::Mark(m);
        }
        h.nm(tp).mklist = m;
        h.set_link(mp, NONE);
    }
}

/// `rn_addroute(v_arg, n_arg, head, treenodes, prio)`: slot `slot`'s route (key `v`, mask
/// `m`) in the tree; its leaf, or [`NONE`] (nothing linked then).
fn rn_addroute<T, const K: usize>(
    h: &mut RadixTree<T, K>,
    v: &[u8; K],
    m: Option<&[u8; K]>,
    slot: u32,
) -> u32 {
    // Non-contiguous masks may share a mask among many routes: one unique copy each, so
    // duplicates compare by identity.
    let mut tm = NONE;
    if let Some(m) = m {
        let off = h.n(TOP).off as usize;
        tm = rn_addmask(&mut h.masks, m, false, off);
        if tm == NONE {
            return NONE;
        }
    }
    let (mut tt, keyduplicated) = rn_insert(&mut h.nodes, v, slot);
    let mut saved_tt = NONE;
    if keyduplicated {
        saved_tt = tt;
        tt = slot << 1;
        let n = h.nm(tt);
        n.b = -1;
        n.flags = RNF_ACTIVE;
    }
    // Put mask into the node.
    if tm != NONE {
        let mn = *h.masks.n(tm);
        let n = h.nm(tt);
        n.lm = MASK | tm;
        n.b = mn.b;
        n.flags |= mn.flags & RNF_NORMAL;
    }
    // Either insert into dupedkey list or as a leaf node.
    if keyduplicated {
        if rn_add_dupedkey(h, saved_tt, tt) != 0 {
            return NONE;
        }
    } else {
        rn_fixup_nodes(h, tt);
    }
    // finally insert a radix_mask element if needed
    rn_add_radix_mask(h, tt, keyduplicated);
    tt
}

/// D-20: a route other than `tt` whose `rn_mklist` is `m`, the normal annotation `tt` holds:
/// the next one of `tt`'s chain (where radix.c's normal branch hands it on), else the first
/// before it (a route that shares it from earlier in the chain), else the first under the
/// node it hangs on (a route of another key, which shares it when its mask is the
/// annotation's leaf's, `rn_add_radix_mask`'s `mmask`); [`NONE`]. O(c) the chain, and the
/// node's subtree only when the chain holds none.
fn rn_holder<T, const K: usize>(h: &RadixTree<T, K>, tt: u32, m: u32) -> u32 {
    let mut d = h.n(tt).dupedkey();
    while d != NONE {
        if h.n(d).mklist == m {
            return d;
        }
        d = h.n(d).dupedkey();
    }
    let mut d = tt;
    while h.n(h.n(d).p).b < 0 {
        d = h.n(d).p;
    }
    while d != tt {
        if h.n(d).mklist == m {
            return d;
        }
        d = h.n(d).dupedkey();
    }
    let x = rn_lift_node(&h.nodes, tt);
    if x == NONE {
        return NONE;
    }
    // The chains under x, in rn_walktree's order: down the left, each chain, then up past
    // the right children and over to the next right subtree.
    let mut t = x;
    loop {
        while h.n(t).b >= 0 {
            t = h.n(t).l();
        }
        let mut d = t;
        while d != NONE {
            if d != tt && h.n(d).mklist == m {
                return d;
            }
            d = h.n(d).dupedkey();
        }
        loop {
            let p = h.n(t).p;
            if h.n(p).l() == t {
                t = h.n(p).r();
                break;
            }
            t = p;
            if t == x {
                return NONE;
            }
        }
    }
}

/// `rn_del_radix_mask(tt)`: route `tt`'s reference to its annotation dropped, the
/// annotation unlinked and freed with its last; -1 when the tree's annotations are
/// inconsistent and the route must stay.
fn rn_del_radix_mask<T, const K: usize>(h: &mut RadixTree<T, K>, tt: u32) -> i32 {
    let ttn = *h.n(tt);
    let m = ttn.mklist;
    let saved_m = m;
    if ttn.mask() == NONE || m == NONE {
        return 0;
    }
    if ttn.flags & RNF_NORMAL != 0 {
        let leaf = h.rm_leaf(m);
        if leaf != tt && h.rm(m).refs == 0 {
            log(
                LOG_ERR,
                format_args!("rn_delete: inconsistent normal annotation\n"),
            );
            return -1;
        }
        if leaf != tt {
            h.rmm(m).refs -= 1;
            if h.rm(m).refs >= 0 {
                return 0;
            }
            log(
                LOG_ERR,
                format_args!("rn_delete: inconsistent mklist refcount\n"),
            );
        }
        // tt is the annotation's leaf, so the head of a multipath chain.
        if h.rm(m).refs > 0 {
            let d = ttn.dupedkey();
            if d == NONE || h.n(d).mklist != m {
                log(
                    LOG_ERR,
                    format_args!("rn_delete: inconsistent dupedkey list\n"),
                );
                return -1;
            }
            h.set_rm_leaf(m, d);
            h.rmm(m).refs -= 1;
            return 0;
        }
        // else tt is last and only route
    } else if h.rm(m).flags & RNF_NORMAL != 0 {
        // D-20 (C'): a normal route's annotation held by a route without RNF_NORMAL (it took
        // the annotation in rn_add_radix_mask's duplicated-key branch, or shares it). radix.c
        // reads the annotation as a mask's here, logs "inconsistent annotation" and leaves it
        // naming the route it frees; the normal branch's bookkeeping runs instead, so the
        // tree is the one built without this route.
        h.note_repair();
        if h.rm_leaf(m) != tt {
            // One sharer less (radix.c leaves the count as it was).
            let rm = h.rmm(m);
            if rm.refs > 0 {
                rm.refs -= 1;
            }
            return 0;
        }
        // Another route that holds the annotation takes it; none (or no other by the
        // count): it leaves the tree below.
        let d = if h.rm(m).refs > 0 {
            rn_holder(h, tt, m)
        } else {
            NONE
        };
        if d != NONE {
            h.set_rm_leaf(m, d);
            let rm = h.rmm(m);
            if rm.refs > 0 {
                rm.refs -= 1;
            }
            return 0;
        }
    } else {
        if h.rm(m).rmu != ttn.mask() {
            log(
                LOG_ERR,
                format_args!("rn_delete: inconsistent annotation\n"),
            );
            return 0;
        }
        h.rmm(m).refs -= 1;
        if h.rm(m).refs >= 0 {
            return 0;
        }
    }
    // No other reference holds the annotation: out of the tree.
    let x = rn_lift_node(&h.nodes, tt);
    if x == NONE {
        return 0; // Wasn't lifted at all
    }
    let mut mp = MkLink::Node(x);
    let mut found = false;
    loop {
        let m = h.link(mp);
        if m == NONE {
            break;
        }
        if m == saved_m {
            let next = h.rm(m).mklist;
            h.set_link(mp, next);
            h.marks.release(m);
            found = true;
            break;
        }
        mp = MkLink::Mark(m);
    }
    if !found {
        log(
            LOG_ERR,
            format_args!("rn_delete: couldn't find our annotation\n"),
        );
        if ttn.flags & RNF_NORMAL != 0 {
            return -1; // Dangling ref to us
        }
    }
    0
}

/// `rn_swap_nodes(from, to)`: internal node `from` moved into `to`'s place (another slot's
/// internal node), its parent's and children's links moved with it.
fn rn_swap_nodes<V, const K: usize>(t: &mut Slab<Slot<V, K>>, from: u32, to: u32) {
    let f = *t.n(from);
    *t.nm(to) = f;
    if t.n(f.p).l() == from {
        t.nm(f.p).lm = to;
    } else {
        t.nm(f.p).rd = to;
    }
    t.nm(f.l()).p = to;
    t.nm(f.r()).p = to;
}

/// `rn_delete(v_arg, n_arg, head, rn)`: the route of key `v` and mask `n` (none: the head of
/// the key's chain) found and unlinked; its leaf, or [`NONE`].
fn rn_delete<T, const K: usize>(h: &mut RadixTree<T, K>, v: &[u8; K], n: Option<&[u8; K]>) -> u32 {
    let top = TOP;
    let off = h.n(top).off as usize;
    let vlen = salen(v);
    // As `rn_lookup`, but the chain's head is kept.
    let mut tt = rn_search(&h.nodes, v, top);
    // make sure the key is a perfect match
    let tkey = h.nodes.key(tt);
    if (off..vlen).any(|i| v[i] != tkey[i]) {
        return NONE;
    }
    // tt is the deletion target, saved_tt the head of the duplicated key chain.
    let saved_tt = tt;
    if let Some(n) = n {
        let Ok(tm) = rn_findmask(&h.masks, n, off) else {
            return NONE;
        };
        let netmask = MASK | tm;
        while h.n(tt).mask() != netmask {
            tt = h.n(tt).dupedkey();
            if tt == NONE {
                return NONE;
            }
        }
    }
    rn_delete_leaf(h, saved_tt, tt)
}

/// The unlinking half of `rn_delete`: route leaf `tt` of the chain headed by `saved_tt` out
/// of the tree, with its annotation and, when it was the chain's only route, its internal
/// node; `tt`, or [`NONE`] when its annotation's state forbids it or `tt` is a root leaf.
fn rn_delete_leaf<T, const K: usize>(h: &mut RadixTree<T, K>, saved_tt: u32, tt: u32) -> u32 {
    kassert!(h.n(tt).flags & RNF_ROOT == 0);
    if h.n(tt).flags & RNF_ROOT != 0 {
        return NONE;
    }
    // remove possible radix_mask
    if rn_del_radix_mask(h, tt) != 0 {
        return NONE;
    }
    let tt1 = tt | 1;
    let tp = h.n(tt).p;
    if h.n(saved_tt).dupedkey() != NONE {
        let x = if tt == saved_tt {
            let x = h.n(saved_tt).dupedkey();
            h.nm(x).p = tp;
            if h.n(tp).l() == tt {
                h.nm(tp).lm = x;
            } else {
                h.nm(tp).rd = x;
            }
            x
        } else {
            let d = h.n(tt).dupedkey();
            h.nm(tp).rd = d;
            if d != NONE {
                h.nm(d).p = tp;
            }
            saved_tt
        };
        // We may be holding an active internal node in the tree.
        if h.n(tt1).flags & RNF_ACTIVE != 0 {
            rn_swap_nodes(&mut h.nodes, tt1, x | 1);
        }
    } else {
        // Not duplicated: tt and its parent leave the tree.
        let x = if h.n(tp).l() == tt {
            h.n(tp).r()
        } else {
            h.n(tp).l()
        };
        let pp = h.n(tp).p;
        if h.n(pp).r() == tp {
            h.nm(pp).rd = x;
        } else {
            h.nm(pp).lm = x;
        }
        h.nm(x).p = pp;
        // Demote routes attached to us (actually on the internal parent node).
        let tpm = h.n(tp).mklist;
        if tpm != NONE {
            if h.n(x).b >= 0 {
                let mut mp = MkLink::Node(x);
                while h.link(mp) != NONE {
                    mp = MkLink::Mark(h.link(mp));
                }
                h.set_link(mp, tpm);
            } else {
                // The routes of a sibling duplicated key chain hold, in their order, some of
                // our list's annotations.
                let mut m = tpm;
                let mut x = x;
                while m != NONE && x != NONE {
                    if m == h.n(x).mklist {
                        let mut mm = h.rm(m).mklist;
                        h.nm(x).mklist = NONE;
                        h.rmm(m).refs -= 1;
                        if h.rm(m).refs < 0 {
                            h.marks.release(m);
                        } else if h.rm(m).flags & RNF_NORMAL != 0 {
                            // A multipath route: the next route uses the same m.
                            mm = m;
                        }
                        m = mm;
                    }
                    x = h.n(x).dupedkey();
                }
                if m != NONE {
                    log(
                        LOG_ERR,
                        format_args!("rn_delete: Orphaned Mask {m} at {x}\n"),
                    );
                }
            }
        }
        // Our internal node, if it is not the parent just removed, takes the parent's place.
        if tp != tt1 {
            rn_swap_nodes(&mut h.nodes, tt1, tp);
        }
    }
    h.nm(tt).flags &= !RNF_ACTIVE;
    h.nm(tt1).flags &= !RNF_ACTIVE;
    tt
}

/// `rn_walktree`'s step: the head of the chain after the one headed by leaf `base` (the right
/// root leaf after the last; a leaf may vanish, so it is the next leaf, not a node).
fn rn_walktree<T, const K: usize>(h: &RadixTree<T, K>, base: u32) -> u32 {
    let mut rn = base;
    // If at right child go back up, otherwise, go right.
    while h.n(h.n(rn).p).r() == rn && h.n(rn).flags & RNF_ROOT == 0 {
        rn = h.n(rn).p;
    }
    // Find the next leaf.
    rn = h.n(h.n(rn).p).r();
    while h.n(rn).b >= 0 {
        rn = h.n(rn).l();
    }
    rn
}

/// `rn_inithead0(rnh, offset)`: the roots of an empty tree in its first two slots: the top
/// node testing bit `offset * 8` between the left root leaf (the key of zeros) and the right
/// one (the key of ones). `None` when no memory is left.
fn rn_inithead0<V, const K: usize>(t: &mut Slab<Slot<V, K>>, offset: usize) -> Option<()> {
    let (left, right) = (t.alloc()?, t.alloc()?);
    kassert!(left == ROOT_LEFT && right << 1 == ROOT_RIGHT);
    let off = (offset * 8) as i32;
    let top = rn_newpair(t, left, off);
    let tt = t.n(top).l();
    let node = t.nm(top);
    node.rd = ROOT_RIGHT;
    node.p = top;
    node.flags = RNF_ROOT | RNF_ACTIVE;
    let leaf = t.nm(tt);
    leaf.flags = RNF_ROOT | RNF_ACTIVE;
    leaf.b = (-1 - off) as i16;
    let copy = *t.n(tt);
    *t.nm(ROOT_RIGHT) = copy;
    t.get_mut(right).key = [0xff; K];
    Some(())
}

const _: () = {
    assert!(TOP == ROOT_LEFT | 1);
    assert!(SLAB_LIMIT * 2 <= MASK as usize);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the radix tree as its users key it: IPv4 addresses laid out as
    // `sockaddr_in`s (length, family, port, address, zeros) in a tree at the address's
    // offset; LZ's seven tests re-expressed over the owned tree, then the generation of a
    // handle, the short and long keys (D-7), the mask normalisation, the tree emptied, and
    // property tests over random operations against a linear model, with the tree's links
    // checked after every change.

    use std::format;
    use std::vec::Vec;

    use hal::malloc::MBox;

    use super::*;
    use crate::crypto::testutil::XorShift;
    use crate::net::rtable::M_RTABLE;
    use crate::testutil;

    /// `offsetof(struct sockaddr_in, sin_addr)`.
    const OFF: usize = 4;
    /// pf's key length (`sizeof(union pfsockaddr_union)`), as LZ's tests' `rn_init(28)`.
    const KL: usize = 28;
    type Tree = RadixTree<u32, KL>;

    fn tree() -> Tree {
        testutil::install();
        RadixTree::new(OFF, M_RTABLE)
    }

    /// A `sockaddr_in` for `a`.
    fn sin(a: [u8; 4]) -> [u8; 16] {
        let mut k = [0u8; 16];
        k[0] = 16;
        k[1] = 2;
        k[4..8].copy_from_slice(&a);
        k
    }

    /// The mask of a `plen`-bit prefix, as `pfr_prepare_network` builds it.
    fn mask(plen: u32) -> [u8; 16] {
        let m = if plen == 0 {
            0
        } else {
            u32::MAX << (32 - plen)
        };
        sin(m.to_be_bytes())
    }

    fn add(h: &mut Tree, a: [u8; 4], plen: Option<u32>, v: u32) -> Result<Leaf, u32> {
        let m = plen.map(mask);
        h.add(&sin(a), m.as_ref().map(|m| &m[..]), v)
    }

    fn del(h: &mut Tree, a: [u8; 4], plen: Option<u32>) -> Option<u32> {
        let m = plen.map(mask);
        h.delete(&sin(a), m.as_ref().map(|m| &m[..]))
    }

    fn matched(h: &Tree, a: [u8; 4]) -> Option<u32> {
        h.matches(&sin(a)).copied()
    }

    fn lookup(h: &Tree, a: [u8; 4], plen: Option<u32>) -> Option<u32> {
        let m = plen.map(mask);
        h.lookup(&sin(a), m.as_ref().map(|m| &m[..])).copied()
    }

    /// The values in walk order.
    fn walk(h: &Tree) -> Vec<u32> {
        let mut seen = Vec::new();
        let mut c = h.first();
        while let Some(l) = c {
            c = h.next(l);
            seen.push(*h.get(l).expect("a live route"));
        }
        seen
    }

    /// What `visit` gathers: the routes' values in order, the chain heads, the annotations
    /// listed on internal nodes and the leaves' `rn_mklist`s.
    #[derive(Default)]
    struct Seen {
        values: Vec<u32>,
        heads: Vec<u32>,
        listed: Vec<u32>,
        held: Vec<(u32, u32)>,
    }

    /// In order from `x`: every internal node's children point back at it and test later
    /// bits, every chain is doubly linked with a root only at its head, every node reached is
    /// `RNF_ACTIVE`; what it sees into `s`.
    fn visit(h: &Tree, x: u32, s: &mut Seen) {
        let n = *h.n(x);
        assert!(n.flags & RNF_ACTIVE != 0, "node {x} not active");
        if n.b >= 0 {
            let mut m = n.mklist;
            while m != NONE {
                s.listed.push(m);
                m = h.rm(m).mklist;
            }
            for c in [n.l(), n.r()] {
                assert_eq!(h.n(c).p, x, "child {c} of {x}");
                if h.n(c).b >= 0 {
                    assert!(h.n(c).b > n.b, "bits grow down the tree");
                }
                visit(h, c, s);
            }
            return;
        }
        s.heads.push(x);
        if n.flags & RNF_ROOT == 0 {
            s.values.push(*h.value_at(x).expect("a route"));
        }
        let mut prev = NONE;
        let mut d = x;
        while d != NONE {
            if prev != NONE {
                assert_eq!(h.n(d).p, prev, "chain link");
                assert!(h.n(d).flags & RNF_ROOT == 0, "a root only heads its chain");
                assert!(h.n(d).flags & RNF_ACTIVE != 0);
                s.values.push(*h.value_at(d).expect("a route"));
            }
            if h.n(d).mklist != NONE {
                s.held.push((d, h.n(d).mklist));
            }
            prev = d;
            d = h.n(d).dupedkey();
        }
    }

    /// The tree's links checked from the top (`visit`), every route reachable once, the roots
    /// at both ends, every leaf's `rn_mklist` an annotation still listed on a node (D-20);
    /// nothing allocated without a head, no annotation on a head without routes (the head a
    /// refused add keeps). The routes' values in the tree's order.
    fn check_links(h: &Tree) -> Vec<u32> {
        if h.nodes.is_empty() {
            assert_eq!(h.routes, 0, "routes without a head");
            assert!(h.masks.is_empty() && h.marks.is_empty());
            return Vec::new();
        }
        let mut s = Seen::default();
        assert_eq!(h.n(TOP).p, TOP, "the top is its own parent");
        visit(h, TOP, &mut s);
        assert_eq!(s.values.len(), h.routes, "every route reachable once");
        assert_eq!(s.heads.first(), Some(&ROOT_LEFT));
        assert_eq!(s.heads.last(), Some(&ROOT_RIGHT));
        for (leaf, m) in &s.held {
            assert!(
                s.listed.contains(m),
                "leaf {leaf} names annotation {m}, listed on no node"
            );
        }
        if h.routes == 0 {
            assert!(
                s.listed.is_empty(),
                "a head without routes holds no annotation"
            );
        }
        s.values
    }

    #[test]
    fn masks_refine_and_order() {
        let k = |m: [u8; 16]| key_of::<KL>(&m);
        let (m8, m16, m24) = (k(mask(8)), k(mask(16)), k(mask(24)));
        assert!(rn_refines(&m16, &m8), "/16 is inside /8");
        assert!(!rn_refines(&m8, &m16));
        assert!(!rn_refines(&m16, &m16), "equal masks");
        assert!(rn_refines(&m24, &m16));
        let odd = k(sin([255, 0, 255, 0]));
        assert!(!rn_refines(&odd, &m16));
        assert!(rn_lexobetter(&m24, &m16));
        assert!(!rn_lexobetter(&m16, &m24));
        assert!(!rn_lexobetter(&m16, &m16));
        // A longer mask equal on the shorter one's bytes refines it when its tail has a bit.
        let mut short = [0u8; KL];
        short[0] = 6;
        short[4] = 0xff;
        short[5] = 0xff;
        let mut long = short;
        long[0] = 7;
        long[6] = 0x80;
        assert!(rn_refines(&long, &short));
        long[6] = 0;
        assert!(!rn_refines(&long, &short), "a zero tail adds nothing");
    }

    #[test]
    fn longest_prefix_wins() {
        let mut h = tree();
        assert_eq!(matched(&h, [10, 1, 2, 3]), None, "empty tree");
        assert!(add(&mut h, [10, 1, 0, 0], Some(16), 16).is_ok());
        assert!(add(&mut h, [10, 0, 0, 0], Some(8), 8).is_ok());
        assert!(add(&mut h, [10, 1, 2, 3], None, 32).is_ok());
        assert!(add(&mut h, [10, 1, 2, 0], Some(24), 24).is_ok());
        assert!(add(&mut h, [192, 168, 0, 0], Some(16), 192).is_ok());
        check_links(&h);

        assert_eq!(matched(&h, [10, 1, 2, 3]), Some(32));
        assert_eq!(matched(&h, [10, 1, 2, 4]), Some(24));
        assert_eq!(matched(&h, [10, 1, 2, 255]), Some(24));
        assert_eq!(matched(&h, [10, 1, 3, 1]), Some(16));
        assert_eq!(matched(&h, [10, 1, 255, 255]), Some(16));
        assert_eq!(matched(&h, [10, 2, 0, 0]), Some(8));
        assert_eq!(matched(&h, [10, 255, 255, 255]), Some(8));
        assert_eq!(matched(&h, [192, 168, 77, 1]), Some(192));
        assert_eq!(matched(&h, [192, 169, 0, 1]), None);
        assert_eq!(matched(&h, [11, 0, 0, 0]), None);
        assert_eq!(
            matched(&h, [0, 0, 0, 0]),
            None,
            "the root leaf is not a route"
        );

        // A default route catches the rest; it is a duplicated key of the left root leaf.
        assert!(add(&mut h, [0, 0, 0, 0], Some(0), 0).is_ok());
        assert_eq!(matched(&h, [11, 0, 0, 0]), Some(0));
        assert_eq!(
            matched(&h, [0, 0, 0, 0]),
            Some(0),
            "explicitly asked for the default"
        );
        assert_eq!(matched(&h, [10, 1, 2, 3]), Some(32));
        check_links(&h);
    }

    #[test]
    fn lookup_wants_the_exact_mask() {
        let mut h = tree();
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        add(&mut h, [10, 1, 0, 0], Some(16), 16).expect("added");
        add(&mut h, [10, 1, 2, 0], Some(24), 24).expect("added");

        assert_eq!(lookup(&h, [10, 1, 0, 0], Some(16)), Some(16));
        assert_eq!(lookup(&h, [10, 0, 0, 0], Some(8)), Some(8));
        assert_eq!(lookup(&h, [10, 1, 2, 0], Some(24)), Some(24));
        assert_eq!(
            lookup(&h, [10, 1, 0, 0], Some(24)),
            None,
            "a known mask, but not on that key"
        );
        assert_eq!(
            lookup(&h, [10, 1, 0, 0], Some(20)),
            None,
            "a mask the mask tree does not have"
        );
        assert_eq!(
            lookup(&h, [10, 1, 9, 9], None),
            Some(16),
            "no mask is rn_match"
        );
        assert_eq!(lookup(&h, [12, 0, 0, 0], None), None);
    }

    #[test]
    fn duplicated_keys_chain_most_specific_first() {
        let mut h = tree();
        assert!(add(&mut h, [10, 1, 0, 0], Some(16), 16).is_ok());
        assert_eq!(
            add(&mut h, [10, 1, 0, 0], Some(16), 99),
            Err(99),
            "same key and mask"
        );
        assert!(
            add(&mut h, [10, 1, 0, 0], Some(24), 24).is_ok(),
            "same key, longer mask"
        );
        assert!(
            add(&mut h, [10, 1, 0, 0], None, 32).is_ok(),
            "same key, no mask"
        );
        assert_eq!(
            add(&mut h, [10, 1, 0, 0], None, 98),
            Err(98),
            "a second host route"
        );
        assert!(add(&mut h, [10, 0, 0, 0], Some(8), 8).is_ok());
        assert_eq!(add(&mut h, [10, 0, 0, 0], Some(8), 97), Err(97));

        // The chain starts with the host route, then the masks from the most specific.
        let v = key_of::<KL>(&sin([10, 1, 0, 0]));
        let mut chain = Vec::new();
        let mut x = rn_match(&h, &v);
        while x != NONE {
            chain.push(*h.value_at(x).expect("a route"));
            x = h.n(x).dupedkey();
        }
        assert_eq!(chain, [32, 24, 16]);
        check_links(&h);

        assert_eq!(matched(&h, [10, 1, 0, 0]), Some(32));
        assert_eq!(matched(&h, [10, 1, 0, 7]), Some(24));
        assert_eq!(matched(&h, [10, 1, 7, 7]), Some(16));
        assert_eq!(matched(&h, [10, 7, 7, 7]), Some(8));
        assert_eq!(lookup(&h, [10, 1, 0, 0], Some(16)), Some(16));
        assert_eq!(lookup(&h, [10, 1, 0, 0], Some(24)), Some(24));

        // Deleting from the chain: the head (the leaf in the tree), then the rest.
        assert_eq!(del(&mut h, [10, 1, 0, 0], None), Some(32));
        assert_eq!(matched(&h, [10, 1, 0, 0]), Some(24));
        check_links(&h);
        assert_eq!(del(&mut h, [10, 1, 0, 0], Some(16)), Some(16));
        assert_eq!(matched(&h, [10, 1, 7, 7]), Some(8));
        assert_eq!(matched(&h, [10, 1, 0, 9]), Some(24));
        assert_eq!(del(&mut h, [10, 1, 0, 0], Some(24)), Some(24));
        assert_eq!(matched(&h, [10, 1, 0, 0]), Some(8));
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(8)), Some(8));
        assert_eq!(matched(&h, [10, 1, 0, 0]), None);
        assert!(walk(&h).is_empty());
        check_links(&h);
    }

    #[test]
    fn delete_unlinks_and_returns_the_value() {
        let mut h = tree();
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        add(&mut h, [10, 1, 0, 0], Some(16), 16).expect("added");
        let l24 = add(&mut h, [10, 1, 2, 0], Some(24), 24).expect("added");
        add(&mut h, [10, 1, 2, 3], None, 32).expect("added");
        add(&mut h, [172, 16, 0, 0], Some(12), 172).expect("added");

        assert_eq!(del(&mut h, [10, 9, 9, 9], None), None, "not in the tree");
        assert_eq!(
            del(&mut h, [10, 1, 0, 0], Some(24)),
            None,
            "the key with another mask"
        );
        assert_eq!(
            del(&mut h, [10, 1, 0, 0], Some(20)),
            None,
            "an unknown mask"
        );

        assert_eq!(del(&mut h, [10, 1, 2, 0], Some(24)), Some(24));
        assert_eq!(h.get(l24), None, "its handle names nothing");
        assert_eq!(h.nodes.get(l24.slot).generation, 0, "its slot is vacant");
        assert_eq!(matched(&h, [10, 1, 2, 4]), Some(16));
        assert_eq!(matched(&h, [10, 1, 2, 3]), Some(32));
        assert_eq!(del(&mut h, [10, 1, 2, 0], Some(24)), None, "already gone");
        check_links(&h);

        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(8)), Some(8));
        assert_eq!(matched(&h, [10, 2, 0, 0]), None);
        assert_eq!(matched(&h, [10, 1, 9, 9]), Some(16));
        assert_eq!(matched(&h, [172, 31, 1, 1]), Some(172));

        // The route can go back in.
        assert!(add(&mut h, [10, 1, 2, 0], Some(24), 24).is_ok());
        assert_eq!(matched(&h, [10, 1, 2, 4]), Some(24));
        check_links(&h);

        for (a, plen, v) in [
            ([10, 1, 2, 3], None, 32),
            ([10, 1, 0, 0], Some(16), 16),
            ([172, 16, 0, 0], Some(12), 172),
            ([10, 1, 2, 0], Some(24), 24),
        ] {
            assert_eq!(del(&mut h, a, plen), Some(v));
            check_links(&h);
        }
        assert!(walk(&h).is_empty());
        assert_eq!(matched(&h, [10, 1, 2, 3]), None);
    }

    #[test]
    fn non_contiguous_masks_match_through_the_annotations() {
        let mut h = tree();
        let m = sin([255, 0, 255, 0]);
        assert!(h.add(&sin([10, 0, 5, 0]), Some(&m), 1).is_ok());
        add(&mut h, [10, 0, 4, 0], Some(24), 2).expect("added");
        check_links(&h);
        assert_eq!(matched(&h, [10, 7, 5, 9]), Some(1));
        assert_eq!(matched(&h, [10, 0, 5, 1]), Some(1));
        assert_eq!(matched(&h, [10, 0, 4, 1]), Some(2));
        assert_eq!(matched(&h, [10, 7, 6, 9]), None);
        assert_eq!(matched(&h, [11, 0, 5, 0]), None);
        assert_eq!(h.lookup(&sin([10, 0, 5, 0]), Some(&m)).copied(), Some(1));
        assert_eq!(h.delete(&sin([10, 0, 5, 0]), Some(&m)), Some(1));
        assert_eq!(matched(&h, [10, 7, 5, 9]), None);
        check_links(&h);
    }

    #[test]
    fn walk_visits_every_route_in_key_order() {
        let mut h = tree();
        assert!(walk(&h).is_empty(), "the root leaves are not routes");
        let routes = [
            ([192, 168, 1, 0], Some(24)),
            ([10, 0, 0, 0], Some(8)),
            ([10, 1, 0, 0], Some(16)),
            ([10, 1, 0, 0], Some(24)),
            ([10, 1, 2, 3], None),
            ([0, 0, 0, 0], Some(0)),
            ([255, 255, 255, 255], None),
            ([172, 16, 0, 0], Some(12)),
        ];
        for (i, &(a, plen)) in routes.iter().enumerate() {
            assert!(add(&mut h, a, plen, i as u32).is_ok());
        }
        let seen = walk(&h);
        assert_eq!(seen.len(), routes.len(), "duplicated keys included");
        assert_eq!(seen, check_links(&h), "the walk is the tree's order");
        let addrs: Vec<[u8; 4]> = seen.iter().map(|&i| routes[i as usize].0).collect();
        let mut sorted = addrs.clone();
        sorted.sort_unstable();
        assert_eq!(addrs, sorted, "in key order");

        // The body may remove the route it is given: the successor is taken first.
        let mut c = h.first();
        let mut removed = 0;
        while let Some(l) = c {
            c = h.next(l);
            assert!(h.remove(l).is_some());
            removed += 1;
            check_links(&h);
        }
        assert_eq!(removed, routes.len());
        assert!(h.is_empty() && walk(&h).is_empty());
    }

    #[test]
    fn a_handle_of_a_removed_route_names_nothing() {
        let mut h = tree();
        let a = add(&mut h, [10, 0, 0, 1], None, 1).expect("added");
        let b = add(&mut h, [10, 0, 0, 2], None, 2).expect("added");
        assert_eq!(h.remove(a), Some(1));
        assert_eq!(h.remove(a), None);
        assert_eq!(h.get(a), None);
        assert_eq!(h.next(a), None);
        // The slot is used again, under a new generation.
        let c = add(&mut h, [10, 0, 0, 3], None, 3).expect("added");
        assert_eq!(c.slot, a.slot);
        assert_ne!(c, a);
        assert_eq!(h.get(a), None);
        assert_eq!(h.get(c), Some(&3));
        assert_eq!(h.get(b), Some(&2));
        // A handle past this tree's chunks.
        let far = Leaf {
            slot: 1 << 20,
            generation: 1,
        };
        assert_eq!(h.get(far), None);
    }

    #[test]
    fn a_small_tree_takes_small_chunks() {
        // The user's sizing (2026-11-23, "start small, grow"): an export list of one host takes
        // the node and mask trees' first chunks of 8 slots and their directories of 4 handles;
        // the chunks then grow 8, 16, 32, 64.
        testutil::install();
        let before = testutil::malloc_out(M_RTABLE);
        let mut h: RadixTree<MBox<u32>, 16> = RadixTree::new(OFF, M_RTABLE);
        let v = MBox::new(99, hal::malloc::M_WAITOK, 1u32).expect("a box");
        assert!(h.add(&sin([10, 0, 0, 7]), None, v).is_ok());
        let node = size_of::<Slot<MBox<u32>, 16>>();
        let mask = size_of::<Slot<(), 16>>();
        let dir = 4 * size_of::<Option<MArray<Slot<(), 16>>>>();
        let small = testutil::malloc_out(M_RTABLE) - before;
        std::eprintln!(
            "one-host tree: {small} bytes ({node} a slot, {mask} a mask slot, {dir} a directory)"
        );
        assert_eq!(small as usize, 8 * node + dir + 8 * mask + dir);
        assert_eq!(small, 1760, "the module's docs");
        drop(h);
        assert_eq!(testutil::malloc_out(M_RTABLE), before, "all given back");
        // The chunk layout: 0..8, 8..16, 16..32, 32..64, then 64 a chunk.
        for (i, want) in [
            (0, (0, 0)),
            (7, (0, 7)),
            (8, (1, 0)),
            (15, (1, 7)),
            (16, (2, 0)),
            (31, (2, 15)),
            (32, (3, 0)),
            (63, (3, 31)),
            (64, (4, 0)),
            (127, (4, 63)),
            (128, (5, 0)),
        ] {
            assert_eq!(locate(i), want, "element {i}");
            assert_eq!(chunk_base(want.0) + want.1, i);
            assert!(want.1 < chunk_len(want.0));
        }
    }

    #[test]
    fn an_emptied_tree_gives_its_memory_back() {
        let mut h = tree();
        assert!(h.nodes.is_empty() && h.masks.is_empty() && h.marks.is_empty());
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        add(&mut h, [10, 1, 0, 0], Some(16), 16).expect("added");
        assert!(!h.nodes.is_empty() && !h.masks.is_empty() && !h.marks.is_empty());
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(8)), Some(8));
        assert_eq!(del(&mut h, [10, 1, 0, 0], Some(16)), Some(16));
        assert!(h.is_empty());
        assert!(h.nodes.is_empty() && h.masks.is_empty() && h.marks.is_empty());
        // A refused add on an empty tree keeps the head it made (the C's head stays), and
        // no route; the tree's drop gives it back.
        assert_eq!(add(&mut h, [0, 0, 0, 0], None, 1), Err(1));
        assert!(h.is_empty() && !h.nodes.is_empty() && h.marks.is_empty());
        assert_eq!(matched(&h, [0, 0, 0, 0]), None);
        check_links(&h);
        // And it starts again, past one chunk.
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        assert_eq!(matched(&h, [10, 9, 9, 9]), Some(8));
        for i in 0..200u32 {
            let [_, _, a, b] = i.to_be_bytes();
            assert!(add(&mut h, [11, a, b, 1], None, 1000 + i).is_ok());
        }
        assert!(h.nodes.chunks > 1);
        assert_eq!(matched(&h, [11, 0, 150, 1]), Some(1150));
        check_links(&h);
    }

    #[test]
    fn a_root_leaf_is_never_deleted() {
        let mut h = tree();
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        // The key of zeros without a mask names the left root leaf: no route.
        assert_eq!(del(&mut h, [0, 0, 0, 0], None), None);
        assert_eq!(
            add(&mut h, [0, 0, 0, 0], None, 1),
            Err(1),
            "a host route on a root"
        );
        check_links(&h);
        // With the default route there, a masked delete finds it.
        add(&mut h, [0, 0, 0, 0], Some(0), 0).expect("added");
        assert_eq!(del(&mut h, [0, 0, 0, 0], None), None);
        assert_eq!(del(&mut h, [0, 0, 0, 0], Some(0)), Some(0));
        check_links(&h);
    }

    #[test]
    fn short_keys_read_as_zero_padded_and_long_lengths_as_k() {
        // D-7: a key or a mask shorter than K reads with zeros behind it; a length byte above
        // K reads as K.
        testutil::install();
        let mut h: RadixTree<u32, 16> = RadixTree::new(OFF, M_RTABLE);
        let host = sin([10, 0, 0, 1]);
        assert!(h.add(&host, None, 1).is_ok());
        // The same address in 8 bytes (no `sin_zero`): the zeros are read.
        assert_eq!(h.matches(&host[..8]).copied(), Some(1));
        // A length byte past K, bytes past K given: they are not compared.
        let mut long = [0u8; 40];
        long[..16].copy_from_slice(&host);
        long[0] = 200;
        long[30] = 0xaa;
        assert_eq!(h.matches(&long).copied(), Some(1));
        assert_eq!(salen(&key_of::<16>(&long)), 16);
        // A mask of 2 bytes (shorter than the offset) is the zero mask: the default route.
        let net = sin([0, 0, 0, 0]);
        assert!(h.add(&net, Some(&[2, 2]), 0).is_ok());
        assert_eq!(h.matches(&sin([192, 168, 0, 1])).copied(), Some(0));
        assert_eq!(
            h.lookup(&net, Some(&[16])).copied(),
            Some(0),
            "every zero mask is the one"
        );
    }

    #[test]
    fn masks_are_normalised_once() {
        let mut h = tree();
        let skip = OFF;
        let k = |m: [u8; 16]| key_of::<KL>(&m);
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        // /16: index 4 * 8 + 16, normal; trailing zeros trimmed to 6 bytes, ones before the
        // offset.
        let m16 = rn_addmask(&mut h.masks, &k(mask(16)), false, skip);
        assert_ne!(m16, NONE);
        assert_eq!(h.masks.key(m16)[0], 6);
        assert_eq!(&h.masks.key(m16)[1..6], &[0xff, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(i32::from(h.masks.n(m16).b), -1 - (4 * 8 + 16));
        assert!(h.masks.n(m16).flags & RNF_NORMAL != 0);
        // The same mask again is the same leaf, found or added; and so is a copy whose family
        // and port bytes differ (the bytes before the offset are ones).
        assert_eq!(rn_addmask(&mut h.masks, &k(mask(16)), false, skip), m16);
        let mut other = mask(16);
        other[1] = 0;
        other[2] = 7;
        assert_eq!(rn_findmask(&h.masks, &k(other), skip), Ok(m16));
        // A non-contiguous mask: index at its first zero bit, not normal.
        let odd = rn_addmask(&mut h.masks, &k(sin([255, 0, 255, 0])), false, skip);
        assert_eq!(i32::from(h.masks.n(odd).b), -1 - (4 * 8 + 8));
        assert!(h.masks.n(odd).flags & RNF_NORMAL == 0);
        // /20: a partial last byte, normal.
        let m20 = rn_addmask(&mut h.masks, &k(mask(20)), false, skip);
        assert_eq!(i32::from(h.masks.n(m20).b), -1 - (4 * 8 + 20));
        assert!(h.masks.n(m20).flags & RNF_NORMAL != 0);
        // 0xf0 then 0x0f: not normal.
        let ragged = rn_addmask(&mut h.masks, &k(sin([255, 0xf0, 0x0f, 0])), false, skip);
        assert!(h.masks.n(ragged).flags & RNF_NORMAL == 0);
        // The zero mask is the mask tree's left root; search mode does not add.
        assert_eq!(rn_addmask(&mut h.masks, &k(mask(0)), true, skip), ROOT_LEFT);
        assert_eq!(rn_addmask(&mut h.masks, &k(mask(13)), true, skip), NONE);
        assert!(rn_findmask(&h.masks, &k(mask(13)), skip).is_err());
    }

    /// One route of the model: its key, its prefix length (33: a host route) and its value.
    #[derive(Clone, Copy, Debug)]
    struct Route {
        a: [u8; 4],
        plen: u32,
        v: u32,
    }

    fn model_add(h: &mut Tree, r: Route) -> Result<Leaf, u32> {
        add(h, r.a, (r.plen <= 32).then_some(r.plen), r.v)
    }

    /// `a` under a `plen`-bit prefix (33: the address itself).
    fn net(a: [u8; 4], plen: u32) -> [u8; 4] {
        let m = if plen == 0 {
            0
        } else {
            u32::MAX << (32 - plen.min(32))
        };
        (u32::from_be_bytes(a) & m).to_be_bytes()
    }

    /// A random address from a small universe, so keys collide and share prefixes.
    fn addr(rng: &mut XorShift) -> [u8; 4] {
        let mut pick = || [0u8, 1, 10, 0x80, 0xc0, 0xfe, 0xff][rng.below(7)];
        [pick(), pick(), pick(), pick()]
    }

    /// The model's answer for `a`: the longest prefix holding it.
    fn best(model: &[Route], a: [u8; 4]) -> Option<u32> {
        model
            .iter()
            .filter(|m| net(a, m.plen) == m.a)
            .max_by_key(|m| m.plen)
            .map(|m| m.v)
    }

    #[test]
    fn contiguous_masks_answer_the_longest_prefix() {
        // Property: with contiguous masks, `matches` is the model's longest matching prefix
        // (a host route above a /32), `lookup` with a mask is exact presence, adds and deletes
        // agree with the model, the walk is the routes in key order and survives removing the
        // route it is on; the links are checked after every operation.
        for seed in 1..=24u64 {
            let mut rng = XorShift::new(seed.wrapping_mul(0x9e37_79b9));
            let mut h = tree();
            let mut model: Vec<Route> = Vec::new();
            let mut next = 0u32;
            for step in 0..400 {
                let ctx = format!("seed {seed} step {step}");
                match rng.below(10) {
                    0..=3 => {
                        let plens = [0, 1, 7, 8, 9, 15, 16, 17, 23, 24, 25, 31, 32, 33];
                        let plen = plens[rng.below(plens.len())];
                        let r = Route {
                            a: net(addr(&mut rng), plen),
                            plen,
                            v: next,
                        };
                        next += 1;
                        let there = model.iter().any(|m| m.a == r.a && m.plen == r.plen);
                        // A host route on the left root's key (0.0.0.0) is refused by radix.c.
                        let root = r.plen == 33 && r.a == [0; 4];
                        match model_add(&mut h, r) {
                            Ok(_) => {
                                assert!(!there && !root, "{ctx}: added twice");
                                model.push(r);
                            }
                            Err(v) => assert!(there || root, "{ctx}: {v} refused"),
                        }
                    }
                    4..=5 if !model.is_empty() => {
                        let i = rng.below(model.len());
                        let r = model[i];
                        let got = del(&mut h, r.a, (r.plen <= 32).then_some(r.plen));
                        // Without a mask, the chain's head goes: the host route, when there.
                        assert_eq!(got, Some(r.v), "{ctx}: delete {r:?}");
                        model.swap_remove(i);
                    }
                    6 => {
                        let plen = rng.below(34) as u32;
                        let a = net(addr(&mut rng), plen);
                        let want = if plen <= 32 {
                            model
                                .iter()
                                .find(|m| m.a == a && m.plen == plen)
                                .map(|m| m.v)
                        } else {
                            // Without a mask, lookup is rn_match.
                            best(&model, a)
                        };
                        assert_eq!(
                            lookup(&h, a, (plen <= 32).then_some(plen)),
                            want,
                            "{ctx}: lookup {a:?}/{plen}"
                        );
                    }
                    _ => {
                        let a = addr(&mut rng);
                        assert_eq!(matched(&h, a), best(&model, a), "{ctx}: match {a:?}");
                    }
                }
                assert_eq!(check_links(&h).len(), model.len(), "{ctx}");
            }
            // The walk: every route once, in key order; then removing every other route as it
            // goes.
            let order = walk(&h);
            assert_eq!(order.len(), model.len());
            let keys: Vec<[u8; 4]> = order
                .iter()
                .map(|v| model.iter().find(|m| m.v == *v).map_or([0; 4], |m| m.a))
                .collect();
            let mut sorted = keys.clone();
            sorted.sort_unstable();
            assert_eq!(keys, sorted, "seed {seed}: key order");
            let mut c = h.first();
            let mut odd = false;
            let mut kept = Vec::new();
            while let Some(l) = c {
                c = h.next(l);
                if odd {
                    let v = h.remove(l).expect("a live route");
                    model.retain(|m| m.v != v);
                } else {
                    kept.push(*h.get(l).expect("a live route"));
                }
                odd = !odd;
            }
            assert_eq!(
                walk(&h),
                kept,
                "seed {seed}: the walk survives its removals"
            );
            assert_eq!(kept.len(), model.len());
            check_links(&h);
        }
    }

    #[test]
    fn non_contiguous_masks_answer_only_routes_that_hold_the_key() {
        // Property: with any masks, a match is a route whose masked key is the address's, a
        // host route of the address is the answer, and adds, deletes and walks keep the
        // links; the differential test against bsd's tree (sys/net/radix.rs) checks the
        // exact answers.
        let (mut quirks, mut answers, mut covered) = (0, 0, 0);
        for seed in 1..=24u64 {
            let mut rng = XorShift::new(seed.wrapping_mul(0x2545_f491));
            let mut h = tree();
            let mut model: Vec<([u8; 4], Option<[u8; 4]>, u32)> = Vec::new();
            for step in 0..300u32 {
                let ctx = format!("seed {seed} step {step}");
                match rng.below(8) {
                    0..=3 => {
                        let m = match rng.below(4) {
                            0 => None,
                            1 => Some(net([0xff; 4], rng.below(33) as u32)),
                            _ => Some(addr(&mut rng)),
                        };
                        let a = addr(&mut rng);
                        let a = m.map_or(a, |m| {
                            (u32::from_be_bytes(a) & u32::from_be_bytes(m)).to_be_bytes()
                        });
                        let mk = m.map(sin);
                        if h.add(&sin(a), mk.as_ref().map(|m| &m[..]), step).is_ok() {
                            model.push((a, m, step));
                        }
                    }
                    4 if !model.is_empty() => {
                        let i = rng.below(model.len());
                        let (a, m, v) = model[i];
                        let mk = m.map(sin);
                        let got = h.delete(&sin(a), mk.as_ref().map(|m| &m[..]));
                        // Without a mask the chain's head goes, whatever its mask. With one,
                        // the first route of that key and mask goes (radix.c's duplicate test
                        // stops where the chain's order places the new mask, so mixed masks
                        // can put one key and mask in a chain twice), or radix.c refuses
                        // (`rn_del_radix_mask`'s -1 on annotations it finds inconsistent) and
                        // the route stays.
                        if m.is_some() {
                            match got {
                                Some(g) => {
                                    let j = model
                                        .iter()
                                        .position(|r| r.2 == g)
                                        .expect("a route of the model");
                                    assert_eq!((model[j].0, model[j].1), (a, m), "{ctx}");
                                    model.swap_remove(j);
                                }
                                None => assert_eq!(
                                    h.lookup(&sin(a), mk.as_ref().map(|m| &m[..])).copied(),
                                    Some(v),
                                    "{ctx}: a refused delete keeps the route"
                                ),
                            }
                        } else if let Some(g) = got {
                            model.retain(|r| r.2 != g);
                        }
                    }
                    _ => {
                        let a = addr(&mut rng);
                        if let Some(v) = matched(&h, a) {
                            answers += 1;
                            let (k, m, _) = model
                                .iter()
                                .find(|r| r.2 == v)
                                .copied()
                                .expect("a route of the model");
                            let m = m.map_or(u32::MAX, u32::from_be_bytes);
                            if u32::from_be_bytes(a) & m != u32::from_be_bytes(k) {
                                // radix.c's own answer, only for a route whose mask is not
                                // normal, reached through a normal route's flag or annotation
                                // it took over (`rn_link_dupedkey`, `rn_add_radix_mask`; the
                                // test above).
                                assert!(!has_normal_mask(&h, v), "{ctx}: {a:?} under {v}");
                                quirks += 1;
                            }
                        }
                        if let Some(host) = model.iter().find(|r| r.0 == a && r.1.is_none()) {
                            assert_eq!(matched(&h, a), Some(host.2), "{ctx}: host {a:?}");
                        }
                        // Misses: an address a host route or a route of a contiguous (normal)
                        // mask holds is always answered (D-20's dangling annotation answered
                        // nothing there).
                        let held = model.iter().any(|r| {
                            let m = r.1.map_or(u32::MAX, u32::from_be_bytes);
                            m.leading_ones() + m.trailing_zeros() == 32
                                && u32::from_be_bytes(a) & m == u32::from_be_bytes(r.0)
                        });
                        if held {
                            assert!(matched(&h, a).is_some(), "{ctx}: {a:?} missed");
                            covered += 1;
                        }
                    }
                }
                assert_eq!(check_links(&h).len(), model.len(), "{ctx}");
                assert_eq!(h.dangling_annotations(), 0, "{ctx}");
            }
        }
        assert!(covered > 100, "{covered} addresses checked for misses");
        // The quirk is rare: most answers are checked.
        assert!(
            quirks * 10 < answers,
            "{quirks} of {answers} answers through a taken-over normal flag"
        );
    }

    /// Whether the route of value `v` is a host route or has a normal mask.
    fn has_normal_mask(h: &Tree, v: u32) -> bool {
        let mut c = h.first();
        while let Some(l) = c {
            c = h.next(l);
            if h.get(l) == Some(&v) {
                let n = h.n(l.slot << 1);
                return n.mask() == NONE || h.masks.n(n.mask() & !MASK).flags & RNF_NORMAL != 0;
            }
        }
        panic!("no route {v}");
    }

    #[test]
    fn a_new_chain_head_takes_the_old_heads_flags() {
        // radix.c's `rn_link_dupedkey` gives a route linked at the head of its key's chain
        // the old head's flags (after `rn_addroute` set its own `RNF_NORMAL`): a route with a
        // non-contiguous mask placed before a normal one is short-cut as normal by `rn_match`.
        // Kept as the C answers (the differential test against bsd's tree agrees).
        let mut h = tree();
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("added");
        let odd = sin([255, 0, 255, 0]);
        let l = h.add(&sin([10, 0, 0, 0]), Some(&odd), 1).expect("added");
        let n = *h.n(l.slot << 1);
        assert!(n.flags & RNF_NORMAL != 0, "the old head's flag");
        assert!(h.masks.n(n.mask() & !MASK).flags & RNF_NORMAL == 0);
        assert_eq!(
            h.n(h.n(l.slot << 1).dupedkey()).flags & RNF_NORMAL,
            RNF_NORMAL
        );
        // 10.7.7.7 & 255.0.255.0 is 10.0.7.0, not the key: the C answers the route all the
        // same; 10.7.0.7 is the route's.
        assert_eq!(matched(&h, [10, 7, 7, 7]), Some(1));
        assert_eq!(matched(&h, [10, 7, 0, 7]), Some(1));
        check_links(&h);
    }

    /// The tree of the D-20 test: a host route heading the key's chain, a sibling that puts the
    /// chain's parent at bit 63, the /16 (normal, annotated above the chain) and, when `odd`,
    /// a non-normal mask of the same index linked just before the /16.
    fn d20(odd: bool) -> Tree {
        let mut h = tree();
        add(&mut h, [10, 0, 0, 0], None, 100).expect("the chain's head, a host route");
        add(&mut h, [10, 0, 0, 1], None, 101).expect("a sibling");
        add(&mut h, [10, 0, 0, 0], Some(16), 16).expect("a normal route, annotated");
        if odd {
            h.add(&sin([10, 0, 0, 0]), Some(&sin([255, 255, 0, 1])), 1)
                .expect("a non-normal mask of the same index, before the /16");
        }
        h
    }

    /// What the D-20 trees answer: the addresses the annotation serves and the chain's.
    fn d20_answers(h: &Tree) -> [Option<u32>; 4] {
        [
            matched(h, [10, 0, 0, 129]),
            matched(h, [10, 0, 255, 255]),
            matched(h, [10, 0, 255, 254]),
            matched(h, [10, 0, 0, 0]),
        ]
    }

    #[test]
    fn a_deleted_route_hands_its_normal_annotation_on() {
        // D-20 (the user: "repair on delete"). The non-normal route takes the /16's annotation
        // (`rn_add_radix_mask`: "new route is better"); radix.c's delete of it reads the
        // annotation as a mask's ("inconsistent annotation") and leaves it naming the freed
        // route, which `rn_match` then answers. Here the normal branch runs: the /16 takes its
        // annotation back, and the tree answers as the one built without the route.
        let mut h = d20(true);
        assert_eq!(
            matched(&h, [10, 0, 0, 129]),
            Some(1),
            "through the annotation"
        );
        let odd = sin([255, 255, 0, 1]);
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&odd)), Some(1));
        assert_eq!(h.repairs(), 1);
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        let without = d20(false);
        assert_eq!(d20_answers(&h), d20_answers(&without));
        assert_eq!(d20_answers(&h), [Some(16), Some(16), Some(16), Some(100)]);
        // The /16 deleted and added again: its annotation is its own, the answers the same.
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(16)), Some(16));
        assert_eq!(matched(&h, [10, 0, 0, 129]), None);
        assert!(add(&mut h, [10, 0, 0, 0], Some(16), 16).is_ok());
        assert_eq!(d20_answers(&h), d20_answers(&without));
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        // The non-normal route added and deleted again, then the /16 deleted first: each
        // delete leaves a tree that answers as built without the route.
        assert!(h.add(&sin([10, 0, 0, 0]), Some(&odd), 1).is_ok());
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(16)), Some(16));
        assert_eq!(matched(&h, [10, 0, 0, 129]), Some(1));
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&odd)), Some(1));
        assert_eq!(matched(&h, [10, 0, 0, 129]), None);
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        // And the tree empties cleanly.
        assert_eq!(del(&mut h, [10, 0, 0, 1], None), Some(101));
        assert_eq!(del(&mut h, [10, 0, 0, 0], None), Some(100));
        assert!(h.is_empty() && h.nodes.is_empty());
    }

    #[test]
    fn an_annotation_no_route_holds_leaves_the_tree() {
        // D-20: the non-normal route holds the /16's annotation alone once the /16 is gone,
        // and the next route of the chain, a /8 of the same key, holds its own: the delete
        // takes the annotation out of the tree instead of handing it to the /8, which would
        // leave it naming the /8 once that goes too.
        let mut h = d20(true);
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("a /8 of the chain's key, after the /16");
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(16)), Some(16));
        assert_eq!(
            matched(&h, [10, 0, 0, 129]),
            Some(1),
            "through the annotation"
        );
        let odd = sin([255, 255, 0, 1]);
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&odd)), Some(1));
        assert_eq!(h.repairs(), 1);
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        let mut without = tree();
        add(&mut without, [10, 0, 0, 0], None, 100).expect("the chain's head");
        add(&mut without, [10, 0, 0, 1], None, 101).expect("a sibling");
        add(&mut without, [10, 0, 0, 0], Some(8), 8).expect("the /8");
        assert_eq!(d20_answers(&h), d20_answers(&without));
        assert_eq!(d20_answers(&h), [Some(8), Some(8), Some(8), Some(100)]);
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(8)), Some(8));
        assert_eq!(h.dangling_annotations(), 0, "no annotation names the /8");
        assert_eq!(matched(&h, [10, 0, 0, 129]), None);
        check_links(&h);
        assert_eq!(del(&mut h, [10, 0, 0, 1], None), Some(101));
        assert_eq!(del(&mut h, [10, 0, 0, 0], None), Some(100));
        assert!(h.is_empty() && h.nodes.is_empty());
    }

    /// The addresses the D-20 trees are compared on.
    const D20_PROBES: [[u8; 4]; 8] = [
        [10, 0, 1, 129],
        [10, 0, 1, 128],
        [10, 0, 0, 129],
        [10, 0, 0, 0],
        [10, 0, 0, 1],
        [10, 0, 0, 2],
        [10, 0, 0, 6],
        [10, 1, 0, 0],
    ];

    #[test]
    fn an_earlier_route_that_shares_the_annotation_takes_it() {
        // D-20, the closing review's sequence: 255.255.0.2 sorts before the /24 (a longer
        // mask, rn_lexobetter) and shares the /16's annotation from before the odd route; the
        // /16 and then the odd route deleted, the annotation passes back to that earlier route
        // instead of leaving the tree while it still names it. A /8 after them holds its own
        // annotation, so the next route of the chain is not the holder.
        let mut h = d20(true);
        add(&mut h, [10, 0, 0, 0], Some(24), 24).expect("a /24, before the odd route");
        let two = sin([255, 255, 0, 2]);
        h.add(&sin([10, 0, 0, 0]), Some(&two), 2)
            .expect("255.255.0.2, before the /24");
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("a /8, the chain's last");
        assert_eq!(walk(&h), [100, 2, 24, 1, 16, 8, 101], "the chain's order");
        check_links(&h);
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(16)), Some(16));
        let odd = sin([255, 255, 0, 1]);
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&odd)), Some(1));
        assert_eq!(h.repairs(), 1);
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        let mut without = tree();
        add(&mut without, [10, 0, 0, 0], None, 100).expect("the chain's head");
        add(&mut without, [10, 0, 0, 1], None, 101).expect("a sibling");
        add(&mut without, [10, 0, 0, 0], Some(24), 24).expect("the /24");
        without
            .add(&sin([10, 0, 0, 0]), Some(&two), 2)
            .expect("255.255.0.2");
        add(&mut without, [10, 0, 0, 0], Some(8), 8).expect("the /8");
        for a in D20_PROBES {
            assert_eq!(matched(&h, a), matched(&without, a), "{a:?}");
        }
        assert_eq!(matched(&h, [10, 0, 1, 129]), Some(2));
        // The deviations' one difference: the route that took the annotation is short-cut
        // as normal, as radix.c short-cuts the odd route that took it, so an address its mask
        // refuses is answered by it; the tree built without gives it an annotation of its
        // mask, and the /8 answers.
        assert_eq!(matched(&h, [10, 0, 1, 131]), Some(2));
        assert_eq!(matched(&without, [10, 0, 1, 131]), Some(8));
        // Its own delete takes the annotation out; the tree empties cleanly.
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&two)), Some(2));
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        assert_eq!(matched(&h, [10, 0, 1, 131]), Some(8));
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(8)), Some(8));
        assert_eq!(h.dangling_annotations(), 0);
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(24)), Some(24));
        assert_eq!(del(&mut h, [10, 0, 0, 1], None), Some(101));
        assert_eq!(del(&mut h, [10, 0, 0, 0], None), Some(100));
        assert!(h.is_empty() && h.nodes.is_empty());
    }

    #[test]
    fn a_route_of_another_key_that_shares_the_annotation_takes_it() {
        // D-20: a route of another key whose mask is the odd route's shares the /16's
        // annotation once the odd route is its leaf (rn_add_radix_mask's `mmask`); with the
        // /16 and the odd route deleted, no route of the chain holds it, and the route under
        // the node it hangs on takes it.
        let mut h = d20(true);
        let odd = sin([255, 255, 0, 1]);
        h.add(&sin([10, 0, 0, 2]), Some(&odd), 3)
            .expect("another key, the odd route's mask");
        check_links(&h);
        assert_eq!(del(&mut h, [10, 0, 0, 0], Some(16)), Some(16));
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&odd)), Some(1));
        assert_eq!(h.repairs(), 1);
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        let mut without = tree();
        add(&mut without, [10, 0, 0, 0], None, 100).expect("the chain's head");
        add(&mut without, [10, 0, 0, 1], None, 101).expect("a sibling");
        without
            .add(&sin([10, 0, 0, 2]), Some(&odd), 3)
            .expect("the other key's route");
        for a in [
            [10, 0, 0, 0],
            [10, 0, 0, 1],
            [10, 0, 0, 2],
            [10, 0, 0, 6],
            [10, 1, 0, 0],
        ] {
            assert_eq!(matched(&h, a), matched(&without, a), "{a:?}");
        }
        assert_eq!(matched(&h, [10, 0, 0, 6]), Some(3));
        // The deviations' one difference, wider here: the route that took the annotation is
        // short-cut as normal under the node it hangs on, where the tree built without
        // searches under its mask and finds it only from its own key's side.
        assert_eq!(matched(&h, [10, 0, 0, 129]), Some(3));
        assert_eq!(matched(&without, [10, 0, 0, 129]), None);
        assert_eq!(h.delete(&sin([10, 0, 0, 2]), Some(&odd)), Some(3));
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        assert_eq!(del(&mut h, [10, 0, 0, 1], None), Some(101));
        assert_eq!(del(&mut h, [10, 0, 0, 0], None), Some(100));
        assert!(h.is_empty() && h.nodes.is_empty());
    }

    #[test]
    fn the_chain_takes_the_annotation_before_another_key() {
        // D-20: a route before the deleted one in its chain and a route of another key that
        // sorts first under the node both share the annotation; the chain's route takes it,
        // then the other key's when that one goes too.
        let mut h = tree();
        add(&mut h, [10, 0, 0, 4], None, 100).expect("the chain's head, a host route");
        add(&mut h, [10, 0, 0, 5], None, 101).expect("a sibling");
        add(&mut h, [10, 0, 0, 4], Some(16), 16).expect("a normal route, annotated");
        let odd = sin([255, 255, 0, 1]);
        let two = sin([255, 255, 0, 2]);
        h.add(&sin([10, 0, 0, 4]), Some(&odd), 1)
            .expect("the odd route, just before the /16: takes its annotation");
        add(&mut h, [10, 0, 0, 4], Some(24), 24).expect("a /24, before the odd route");
        h.add(&sin([10, 0, 0, 4]), Some(&two), 2)
            .expect("255.255.0.2, before the /24: shares the annotation");
        h.add(&sin([10, 0, 0, 0]), Some(&odd), 3)
            .expect("another key with the odd route's mask: shares it too");
        assert_eq!(walk(&h), [3, 100, 2, 24, 1, 16, 101], "the other key first");
        check_links(&h);
        assert_eq!(del(&mut h, [10, 0, 0, 4], Some(16)), Some(16));
        assert_eq!(h.delete(&sin([10, 0, 0, 4]), Some(&odd)), Some(1));
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        // An address the annotation's short-cut answers: the chain's route took it.
        assert_eq!(matched(&h, [10, 0, 1, 5]), Some(2));
        // That route deleted, the other key's takes it; then it leaves with that one.
        assert_eq!(h.delete(&sin([10, 0, 0, 4]), Some(&two)), Some(2));
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        assert_eq!(matched(&h, [10, 0, 1, 5]), Some(3));
        assert_eq!(h.delete(&sin([10, 0, 0, 0]), Some(&odd)), Some(3));
        assert_eq!(h.repairs(), 3);
        assert_eq!(h.dangling_annotations(), 0);
        check_links(&h);
        assert_eq!(matched(&h, [10, 0, 1, 5]), None);
    }

    #[test]
    fn a_listed_annotation_whose_leaf_is_gone_is_passed_over() {
        // What radix.c:866-868's "Wasn't lifted at all" return or the "Orphaned Mask" demote
        // can leave: a normal annotation still listed whose leaf has gone (its slot's
        // generation moved on). rn_match passes over it to the next annotation.
        let mut h = tree();
        add(&mut h, [10, 0, 0, 0], None, 100).expect("a host route");
        add(&mut h, [10, 0, 0, 1], None, 101).expect("a sibling");
        let l16 = add(&mut h, [10, 0, 0, 0], Some(16), 16).expect("a /16");
        add(&mut h, [10, 0, 0, 0], Some(8), 8).expect("a /8");
        assert_eq!(matched(&h, [10, 0, 0, 129]), Some(16));
        let m = h.n(l16.slot << 1).mklist;
        assert!(
            m != NONE && h.rm(m).flags & RNF_NORMAL != 0,
            "the /16's annotation"
        );
        let g = h.rm(m).generation;
        h.rmm(m).generation = g.wrapping_add(1);
        assert_eq!(h.dangling_annotations(), 1);
        assert_eq!(
            matched(&h, [10, 0, 0, 129]),
            Some(8),
            "passed over: the /8's annotation answers"
        );
    }
}
/* </TESTS> */
