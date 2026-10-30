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
 * Copyright (c) 1989, 1993
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
 *	@(#)vfs_cache.c	8.3 (Berkeley) 8/22/94
 */

/*
 * Copyright (c) 1985, 1989, 1991, 1993
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
 *	@(#)namei.h	8.4 (Berkeley) 8/20/94
 */
/* </LICENSES> */

/* <CODE> */
//! The name cache: recent `namei` lookups, kept per directory in a red-black tree of the
//! names it contains (`v_nc_tree`), with positive entries on one LRU list and negative ones
//! ("this name does not exist") on another, and a reverse map from a directory vnode to the
//! entries naming it (`v_cache_dst`) for `getcwd`.
//!
//! For simplicity (and economy of storage), names longer than a maximum length of
//! `NAMECACHE_MAXLEN` are not cached; they occur infrequently in any case, and are almost
//! never of interest.
//!
//! Upon reaching the last segment of a path, if the reference is for `DELETE`, or `NOCACHE`
//! is set (rewrite), and the name is located in the cache, it will be dropped.
//!
//! Upstream: sys/kern/vfs_cache.c @ 3ce1f3f79392
//! Upstream: sys/sys/namei.h @ 3ce1f3f79392
//! LZ: sys/kern/vfs_cache.rs@3c62ede68802
//! LZ: sys/sys/namei.rs@3c62ede68802
//!
//! ## Deviations
//! - `cache_lookup` returns `Ok(Some(vp))` for a hit, `Err(ENOENT)` for a negative hit,
//!   `Ok(None)` for the C's `-1` (a miss: ask the file system) and `Err(e)` for a failure to
//!   relock; `cache_revlookup` returns `Ok(Some(dvp))`, `Ok(None)` for `-1` and `Err(ERANGE)`
//!   (LZ's).
//! - `cache_revlookup` copies the name into `buf` just before the index `*bpp`, which it moves
//!   backwards, where the C moves a `char *` inside the buffer (LZ's).
//!
//! ## Redesign
//! - The cache is locked, which the C's "TODO: namecache access should really be locked"
//!   asks for and the kernel lock gave it (D-N5.3-2, the user's decision: "own mutex now, N12
//!   removes it"): one mutex, [`NCLOCK`] (`IPL_NONE`, a leaf: a vnode's lock may be held, the
//!   free lists' lock is taken inside it by `vhold`/`vdrop`), whose witness splits in two
//!   parts: part 0 opens the glue's [`NameCache`] (LZ's seven statics, `NcList`'s `unsafe impl
//!   Sync` and the `RbtHead`/`TailqHead` raw links: the LRU lists, the counts, `nextvnodeid`,
//!   `initialvnodes`), part 1 every vnode's [`NcHeads`] (`v_nc_tree`, `v_cache_dst`, the
//!   vnode's `nc`). That is **one spl pair per `cache_lookup`, `cache_enter`,
//!   `cache_revlookup`, `cache_purge` and `cache_purgevfs` that the C does not have** (it runs
//!   them under the kernel lock); the switch's commit measures it. `nchstats` stays racy, as
//!   the C's (`AtomicU64`s outside the lock, in the glue's tables).
//! - An entry is a pool item owned by the LRU list it is on (`OwnedList`: `nc_lru` and
//!   `nc_neg` are one entry, as an entry is on exactly one of the two lists), indexed by its
//!   directory's tree and, for a directory other than `.` and `..`, by its vnode's reverse
//!   list (`SecondaryTree`, `SecondaryList`: counted links, so a `cache_zap` that forgot one
//!   panics instead of freeing an indexed entry). Its words are written before it is linked
//!   and only read after, so they are plain fields. `cache_zap` takes the entry through a
//!   counted link (the list's element cannot be lent across the list's change), which costs
//!   one count up and down per zap, O(1) as the C's pointer.
//! - `cache_zap` removes an entry from its vnode's reverse list when the entry is on it (its
//!   link says so), where the C tests that the capability still matches and that the entry is
//!   one `cache_enter` put there (a directory other than `.`/`..`): the same entries, since a
//!   vnode's capability changes only in `cache_purge`, which empties its reverse list first.
//! - `cache_enter` takes the name (`cnp->cn_nameptr`, `cn_namelen`), not the componentname
//!   (D-N5.3-9: nfs's `readdirplus` names an entry of its reply). It gets the item inside the
//!   section after recycling the LRU's head (the C's order) when the pool has one free
//!   (`BoxPool::get_idle`), and only otherwise leaves the section to sleep in
//!   `pool_get(PR_WAITOK)` and takes it again; the entry is linked on its LRU list first and
//!   then into its directory's tree (a secondary indexes only an owned element), inside the
//!   one section, `numcache`/`numneg` counted on success as the C does.
//! - `cache_purgevfs`'s `TAILQ_FOREACH_SAFE` holds the successor through a counted link
//!   across the zap (docs/IDIOMS.md, "a walk that changes an owned tree under it"); O(1) a
//!   step, as the C.

use core::cmp::Ordering;
use core::marker::PhantomData;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering as Mem};

use hal::intr::IPL_NONE;
use hal::mutex::Mutexed;
use hal::owned::{
    Member, OwnedAdapter, OwnedEntry, OwnedList, SecondaryAdapter, SecondaryEntry, SecondaryList,
    SecondaryRef, SecondaryTree,
};
use hal::pool::PR_WAITOK;
use hal::rwlock::{Divisible, Guarded, WitnessPart};
use hal::sys::errno::Errno;

use super::mount::Mount;
use super::namei::{
    CREATE, Componentname, ISDOTDOT, ISLASTCN, LOCKPARENT, MAKEENTRY, NAMECACHE_MAXLEN, PDIRUNLOCK,
};
use super::subr::{vdrop, vfs, vget, vhold, vput, vref};
use super::vnode::{LK_EXCLUSIVE, VDIR, Vnode, vn_lock};
use super::vops::vop_unlock;
use crate::assert::kpanic;
use crate::proc::ProcGlue;

/// The name cache's lock (`IPL_NONE`, a leaf the C does not have: D-N5.3-2). Part 0 of its
/// witness opens a glue's [`NameCache`], part 1 every vnode's [`NcHeads`].
pub static NCLOCK: Mutexed<()> = Mutexed::new(IPL_NONE, ());

/// The part of [`NCLOCK`]'s witness that opens the vnodes' heads.
pub const NC_HEADS_PART: u8 = 1;

/// `doingcache`: 1 => enable the cache.
pub static DOINGCACHE: AtomicBool = AtomicBool::new(true);

/// `struct namecache`: an element in the cache of recent names looked up by namei.
pub struct Namecache<G: ProcGlue> {
    /// `nc_lru` or `nc_neg`: the LRU chain the entry is on (one: positive or negative).
    lru: OwnedEntry<Self>,
    /// `n_rbcache`: Namecache rb tree from vnode.
    rb: SecondaryEntry<Self>,
    /// `nc_me`: ncp's referring to me.
    me: SecondaryEntry<Self>,
    /// `nc_dvp`: vnode of parent of name.
    pub nc_dvp: Option<&'static Vnode<G>>,
    /// `nc_dvpid`: capability number of `nc_dvp`.
    pub nc_dvpid: u32,
    /// `nc_vp`: vnode the name refers to (`None` for a negative entry).
    pub nc_vp: Option<&'static Vnode<G>>,
    /// `nc_vpid`: capability number of `nc_vp`.
    pub nc_vpid: u32,
    /// `nc_nlen`: length of name.
    pub nc_nlen: u8,
    /// `nc_name`: segment name.
    pub nc_name: [u8; NAMECACHE_MAXLEN],
}

impl<G: ProcGlue> Namecache<G> {
    /// The entry `cache_enter` fills (`pool_get(PR_ZERO)` and its stores): `name` at most
    /// `NAMECACHE_MAXLEN` bytes (the caller's check).
    fn new(dvp: &'static Vnode<G>, vp: Option<&'static Vnode<G>>, name: &[u8]) -> Self {
        let mut nc_name = [0; NAMECACHE_MAXLEN];
        nc_name[..name.len()].copy_from_slice(name);
        Self {
            lru: OwnedEntry::new(),
            rb: SecondaryEntry::new(),
            me: SecondaryEntry::new(),
            nc_dvp: Some(dvp),
            nc_dvpid: dvp.id(),
            nc_vp: vp,
            nc_vpid: vp.map_or(0, Vnode::id),
            nc_nlen: name.len() as u8,
            nc_name,
        }
    }

    /// The `nc_nlen` bytes of `nc_name`.
    #[inline]
    pub fn name(&self) -> &[u8] {
        &self.nc_name[..usize::from(self.nc_nlen)]
    }
}

/// `nc_lru`/`nc_neg`: the LRU list an entry is on, which owns it.
pub struct ByLru<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> OwnedAdapter for ByLru<G> {
    type Elem = Namecache<G>;
    fn entry(e: &Namecache<G>) -> &OwnedEntry<Namecache<G>> {
        &e.lru
    }
    /// A list has no order of its own.
    fn cmp(_a: &Namecache<G>, _b: &Namecache<G>) -> Ordering {
        Ordering::Equal
    }
}

/// `RBT_HEAD(namecache_rb_cache, namecache)`: a directory's cached names, through
/// `n_rbcache`, ordered by `namecache_compare`.
pub struct ByDir<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> SecondaryAdapter for ByDir<G> {
    type Owner = ByLru<G>;
    fn entry(e: &Namecache<G>) -> &SecondaryEntry<Namecache<G>> {
        &e.rb
    }
    fn cmp(a: &Namecache<G>, b: &Namecache<G>) -> Ordering {
        namecache_compare(a.name(), b.name())
    }
}

/// `TAILQ_HEAD(, namecache) v_cache_dst`: the entries naming a vnode, through `nc_me`.
pub struct ByDst<G: ProcGlue>(PhantomData<fn() -> G>);

impl<G: ProcGlue> SecondaryAdapter for ByDst<G> {
    type Owner = ByLru<G>;
    fn entry(e: &Namecache<G>) -> &SecondaryEntry<Namecache<G>> {
        &e.me
    }
    /// A list has no order of its own.
    fn cmp(_a: &Namecache<G>, _b: &Namecache<G>) -> Ordering {
        Ordering::Equal
    }
}

/// A vnode's heads of the cache (`v_nc_tree`, `v_cache_dst`), in its `nc` cell (part
/// [`NC_HEADS_PART`] of [`NCLOCK`]).
pub struct NcHeads<G: ProcGlue> {
    /// `v_nc_tree`: VFS namecache, the entries naming children of this directory.
    pub tree: SecondaryTree<ByDir<G>>,
    /// `v_cache_dst`: cache entries to us.
    pub dst: SecondaryList<ByDst<G>>,
}

impl<G: ProcGlue> NcHeads<G> {
    /// Empty heads (`cache_tree_init` and `TAILQ_INIT(&vp->v_cache_dst)`).
    pub const fn new() -> Self {
        Self {
            tree: SecondaryTree::new(),
            dst: SecondaryList::new(),
        }
    }

    /// A vnode's `nc` cell: empty heads under [`NCLOCK`]'s part [`NC_HEADS_PART`].
    pub fn cell() -> Guarded<Self> {
        Guarded::keyed(NCLOCK.id(), NC_HEADS_PART, Self::new())
    }
}

impl<G: ProcGlue> Default for NcHeads<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The name cache of a glue's vnodes, under [`NCLOCK`]'s part 0 (the C's globals).
pub struct NameCache<G: ProcGlue> {
    /// `nclruhead`: Regular Entry LRU chain.
    lru: OwnedList<ByLru<G>>,
    /// `nclruneghead`: Negative Entry LRU chain.
    neg: OwnedList<ByLru<G>>,
    /// `numcache`: total number of cache entries allocated.
    pub numcache: i64,
    /// `numneg`: number of negative cache entries.
    pub numneg: i64,
    /// `nextvnodeid`: the capability number `cache_purge` hands out last.
    nextvnodeid: u64,
    /// `initialvnodes` (`conf/param.c`), the cache's size, set by `nchinit`.
    initialvnodes: i64,
}

impl<G: ProcGlue> NameCache<G> {
    /// An empty cache (`TAILQ_INIT` of both chains).
    pub const fn new() -> Self {
        Self {
            lru: OwnedList::new(),
            neg: OwnedList::new(),
            numcache: 0,
            numneg: 0,
            nextvnodeid: 0,
            initialvnodes: 0,
        }
    }
}

impl<G: ProcGlue> Default for NameCache<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// `struct nchstats`: stats on usefulness of namei caches, `AtomicU64`s bumped without a
/// lock, as the C's.
pub struct Nchstats {
    /// `ncs_goodhits`: hits that we can really use.
    pub ncs_goodhits: AtomicU64,
    /// `ncs_neghits`: negative hits that we can use.
    pub ncs_neghits: AtomicU64,
    /// `ncs_badhits`: hits we must drop.
    pub ncs_badhits: AtomicU64,
    /// `ncs_falsehits`: hits with id mismatch.
    pub ncs_falsehits: AtomicU64,
    /// `ncs_miss`: misses.
    pub ncs_miss: AtomicU64,
    /// `ncs_long`: long names that ignore cache.
    pub ncs_long: AtomicU64,
    /// `ncs_pass2`: names found with passes == 2.
    pub ncs_pass2: AtomicU64,
    /// `ncs_2passes`: number of times we attempt it.
    pub ncs_2passes: AtomicU64,
    /// `ncs_revhits`: reverse-cache hits.
    pub ncs_revhits: AtomicU64,
    /// `ncs_revmiss`: reverse-cache misses.
    pub ncs_revmiss: AtomicU64,
    /// `ncs_dothits`: hits on '.' lookups.
    pub ncs_dothits: AtomicU64,
    /// `ncs_dotdothits`: hits on '..' lookups.
    pub ncs_dotdothits: AtomicU64,
}

impl Nchstats {
    /// All counters zero.
    pub const fn new() -> Self {
        Self {
            ncs_goodhits: AtomicU64::new(0),
            ncs_neghits: AtomicU64::new(0),
            ncs_badhits: AtomicU64::new(0),
            ncs_falsehits: AtomicU64::new(0),
            ncs_miss: AtomicU64::new(0),
            ncs_long: AtomicU64::new(0),
            ncs_pass2: AtomicU64::new(0),
            ncs_2passes: AtomicU64::new(0),
            ncs_revhits: AtomicU64::new(0),
            ncs_revmiss: AtomicU64::new(0),
            ncs_dothits: AtomicU64::new(0),
            ncs_dotdothits: AtomicU64::new(0),
        }
    }

    /// The counters in the C's member order, as the `u_int64_t`s user space reads
    /// (`kern.nchstats`).
    pub fn snapshot(&self) -> [u64; 12] {
        [
            &self.ncs_goodhits,
            &self.ncs_neghits,
            &self.ncs_badhits,
            &self.ncs_falsehits,
            &self.ncs_miss,
            &self.ncs_long,
            &self.ncs_pass2,
            &self.ncs_2passes,
            &self.ncs_revhits,
            &self.ncs_revmiss,
            &self.ncs_dothits,
            &self.ncs_dotdothits,
        ]
        .map(|c| c.load(Mem::Relaxed))
    }
}

/// `kern.nchstats`: the glue `G`'s name cache statistics, read-only (kern_sysctl.c's
/// `sysctl_rdstruct` of `struct nchstats`: twelve `u_int64_t` in the C's member order). The
/// glue registers [`NchstatsNode::NODE`] in its `kern` table, as `kern.tty`'s node.
pub struct NchstatsNode<G>(PhantomData<fn() -> G>);

impl<G: ProcGlue> NchstatsNode<G> {
    /// The node.
    pub const NODE: NchstatsNode<G> = NchstatsNode(PhantomData);
}

impl Default for Nchstats {
    fn default() -> Self {
        Self::new()
    }
}

/// `x++` of a statistics counter (racy as the C's).
#[inline]
fn bump(c: &AtomicU64) {
    c.store(c.load(Mem::Relaxed).wrapping_add(1), Mem::Relaxed);
}

/// `namecache_compare(n1, n2)`: by length, then by the bytes of the name (the C's
/// `memcmp` of equal lengths, else the length difference).
pub fn namecache_compare(n1: &[u8], n2: &[u8]) -> Ordering {
    n1.len().cmp(&n2.len()).then_with(|| n1.cmp(n2))
}

/// The glue's cache, opened by part 0 of [`NCLOCK`]'s witness.
fn cache_of<'w, G: ProcGlue>(p0: &'w mut WitnessPart<'_>) -> &'w mut NameCache<G> {
    vfs::<G>().namecache.get_mut(p0)
}

/// A counted link to `e`, so the entry can be named while its list changes.
fn hold<G: ProcGlue>(e: &Namecache<G>) -> SecondaryRef<ByLru<G>> {
    let mut h = SecondaryRef::new();
    h.set(Some(e));
    h
}

/// Blow away a namecache entry (`cache_zap`), held by `h`: out of its directory's tree (the
/// directory's hold dropped with the tree's last entry), its vnode's reverse list and its
/// LRU list, and back to its pool.
fn cache_zap<G: ProcGlue>(
    nc: &mut NameCache<G>,
    p1: &mut WitnessPart<'_>,
    h: SecondaryRef<ByLru<G>>,
) {
    let Some(e) = h.get() else {
        kpanic(format_args!("cache_zap: no entry"));
    };
    let (dvp, vp) = (e.nc_dvp, e.nc_vp);
    let mut drop_dvp = None;
    if let Some(d) = dvp {
        let heads = d.nc.get_mut(p1);
        heads.tree.remove(e);
        if heads.tree.is_empty() {
            drop_dvp = Some(d);
        }
    }
    if e.me.is_linked()
        && let Some(v) = vp
    {
        drop(v.nc.get_mut(p1).dst.remove(e));
    }
    let b = if vp.is_some() {
        nc.numcache -= 1;
        nc.lru.take(h)
    } else {
        nc.numneg -= 1;
        nc.neg.take(h)
    };
    drop(b);
    if let Some(d) = drop_dvp {
        vdrop(d);
    }
}

/// What `cache_lookup`'s section found.
enum Found<G: ProcGlue> {
    /// The C's `-1`.
    Miss,
    /// A positive entry: its vnode and the capability it had.
    Hit(&'static Vnode<G>, u32),
}

/// Look for a name in the cache. `dvp` points to the directory to search; `cnp` holds the
/// information on the entry being sought, such as its length and its name. A hit is the
/// vnode, referenced and locked (`vget(LK_EXCLUSIVE)`, the parent unlocked unless
/// `LOCKPARENT` and `ISLASTCN`, as `PDIRUNLOCK` says); a negative hit is `ENOENT`; `Ok(None)`
/// is the C's `-1` (ask the file system), as is a hit whose vnode changed while it waited
/// for its lock.
pub fn cache_lookup<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    cnp: &mut Componentname<'_, G>,
) -> Result<Option<&'static Vnode<G>>, Errno> {
    let stats = &vfs::<G>().nchstats;
    if !DOINGCACHE.load(Mem::Relaxed) {
        cnp.cn_flags &= !MAKEENTRY;
        return Ok(None);
    }
    if cnp.namelen() > NAMECACHE_MAXLEN as i64 {
        bump(&stats.ncs_long);
        cnp.cn_flags &= !MAKEENTRY;
        return Ok(None);
    }

    let flags = cnp.cn_flags;
    let found = {
        let mut g = NCLOCK.lock();
        let [mut p0, mut p1] = g.split::<2>();
        let nc = cache_of::<G>(&mut p0);
        // lookup in directory vnode's redblack tree
        let name = cnp.name();
        match dvp
            .nc
            .get(&p1)
            .tree
            .find_by(|e| namecache_compare(name, e.name()))
        {
            None => {
                bump(&stats.ncs_miss);
                Found::Miss
            }
            Some(ncp) if flags & MAKEENTRY == 0 => {
                bump(&stats.ncs_badhits);
                // remove: the component is to be dropped from the cache.
                let h = hold(ncp);
                cache_zap(nc, &mut p1, h);
                Found::Miss
            }
            Some(ncp) => match ncp.nc_vp {
                None => {
                    if cnp.cn_nameiop != CREATE || flags & ISLASTCN == 0 {
                        bump(&stats.ncs_neghits);
                        // Move this slot to end of the negative LRU chain.
                        if nc.neg.next(ncp).is_some() {
                            nc.neg.move_to_tail(ncp);
                        }
                        return Err(Errno::ENOENT);
                    }
                    bump(&stats.ncs_badhits);
                    let h = hold(ncp);
                    cache_zap(nc, &mut p1, h);
                    Found::Miss
                }
                Some(vp) if ncp.nc_vpid != vp.id() => {
                    bump(&stats.ncs_falsehits);
                    let h = hold(ncp);
                    cache_zap(nc, &mut p1, h);
                    Found::Miss
                }
                Some(vp) => {
                    // Move this slot to end of the regular LRU chain.
                    if nc.lru.next(ncp).is_some() {
                        nc.lru.move_to_tail(ncp);
                    }
                    Found::Hit(vp, vp.id())
                }
            },
        }
    };
    let Found::Hit(vp, vpid) = found else {
        return Ok(None);
    };

    // `vget` and `vn_lock` sleep: outside the cache's section.
    let both = |f: u64| f & (LOCKPARENT | ISLASTCN) == LOCKPARENT | ISLASTCN;
    let error = if ptr::eq(vp, dvp) {
        // lookup on "."
        vref(dvp);
        Ok(())
    } else if flags & ISDOTDOT != 0 {
        let _ = vop_unlock(dvp);
        cnp.cn_flags |= PDIRUNLOCK;
        let error = vget(vp, LK_EXCLUSIVE);
        // If the above vget() succeeded and both LOCKPARENT and ISLASTCN is set, lock the
        // directory vnode as well.
        if error.is_ok() && both(cnp.cn_flags) {
            if let Err(e) = vn_lock(dvp, LK_EXCLUSIVE) {
                vput(vp);
                return Err(e);
            }
            cnp.cn_flags &= !PDIRUNLOCK;
        }
        error
    } else {
        let error = vget(vp, LK_EXCLUSIVE);
        // If the above vget() failed or either of LOCKPARENT or ISLASTCN is set, unlock the
        // directory vnode.
        if error.is_err() || !both(cnp.cn_flags) {
            let _ = vop_unlock(dvp);
            cnp.cn_flags |= PDIRUNLOCK;
        }
        error
    };

    // Check that the lock succeeded, and that the capability number did not change while we
    // were waiting for the lock.
    if error.is_err() || vpid != vp.id() {
        if error.is_ok() {
            vput(vp);
            bump(&stats.ncs_falsehits);
        } else {
            bump(&stats.ncs_badhits);
        }
        // The parent needs to be locked when we return to VOP_LOOKUP(). The `.' case here
        // should be extremely rare (if it can happen at all), so we don't bother optimizing
        // out the unlock/relock.
        if ptr::eq(vp, dvp) || error.is_err() || !both(cnp.cn_flags) {
            vn_lock(dvp, LK_EXCLUSIVE)?;
            cnp.cn_flags &= !PDIRUNLOCK;
        }
        return Ok(None);
    }

    bump(&stats.ncs_goodhits);
    Ok(Some(vp))
}

/// Scan cache looking for name of directory entry pointing at `vp`: its parent. If `buf` is
/// given, also place the name in it just before the index `*bpp`, which moves backwards to
/// the start of it. `Ok(None)` is the C's `-1` (a miss), `ERANGE` a name that does not fit.
pub fn cache_revlookup<G: ProcGlue>(
    vp: &'static Vnode<G>,
    bpp: &mut usize,
    buf: Option<&mut [u8]>,
) -> Result<Option<&'static Vnode<G>>, Errno> {
    let stats = &vfs::<G>().nchstats;
    if !DOINGCACHE.load(Mem::Relaxed) {
        return Ok(None);
    }
    let g = NCLOCK.lock();
    let heads = vp.nc.get(&g);
    let found = heads.dst.iter().find_map(|ncp| {
        let dvp = ncp.nc_dvp?;
        (!ptr::eq(dvp, vp) && ncp.nc_dvpid == dvp.id()).then_some((ncp, dvp))
    });
    let Some((ncp, dvp)) = found else {
        bump(&stats.ncs_revmiss);
        return Ok(None);
    };
    #[cfg(feature = "diagnostic")]
    {
        if ncp.name() == b"." {
            kpanic(format_args!("cache_revlookup: found entry for ."));
        }
        if ncp.name() == b".." {
            kpanic(format_args!("cache_revlookup: found entry for .."));
        }
    }
    bump(&stats.ncs_revhits);

    if let Some(buf) = buf {
        let nlen = usize::from(ncp.nc_nlen);
        if *bpp <= nlen {
            return Err(Errno::ERANGE);
        }
        let bp = *bpp - nlen;
        buf[bp..bp + nlen].copy_from_slice(ncp.name());
        *bpp = bp;
    }

    // XXX: Should we vget() here to have more consistent semantics with cache_lookup()?
    Ok(Some(dvp))
}

/// Whether an entry for `vp` named `name` goes on `vp`'s reverse list: a directory other
/// than `dvp` itself, `.` and `..` (the C's length-and-dot test).
fn reverse_mapped<G: ProcGlue>(dvp: &Vnode<G>, vp: &Vnode<G>, name: &[u8]) -> bool {
    !ptr::eq(vp, dvp)
        && vp.vtype() == VDIR
        && (name.len() > 2
            || (name.len() > 1 && name[1] != b'.')
            || (!name.is_empty() && name[0] != b'.'))
}

/// Add an entry to the cache: `name` in `dvp` names `vp`, or nothing (`None`: a negative
/// entry).
pub fn cache_enter<G: ProcGlue>(
    dvp: &'static Vnode<G>,
    vp: Option<&'static Vnode<G>>,
    name: &[u8],
) {
    if !DOINGCACHE.load(Mem::Relaxed) || name.len() > NAMECACHE_MAXLEN {
        return;
    }
    let pool = &vfs::<G>().nch_pool;
    let mut item = None;
    let mut yields = None;
    loop {
        let mut g = NCLOCK.lock();
        let [mut p0, mut p1] = g.split::<2>();
        let nc = cache_of::<G>(&mut p0);
        // allocate, or recycle (free and allocate) an ncp.
        let b = if let Some(b) = item.take() {
            b
        } else {
            if nc.numcache >= nc.initialvnodes {
                let head = nc.lru.first().or_else(|| nc.neg.first()).map(hold);
                let Some(h) = head else {
                    kpanic(format_args!("wtf? leak?"));
                };
                cache_zap(nc, &mut p1, h);
            }
            if let Some((b, y)) = pool.get_idle(PR_WAITOK, || Namecache::new(dvp, vp, name)) {
                yields = Some(y);
                b
            } else {
                // The pool must grow, which may sleep: not in the section.
                drop(p1);
                drop(p0);
                drop(g);
                let Some(b) = pool.get(PR_WAITOK, || Namecache::new(dvp, vp, name)) else {
                    return;
                };
                item = Some(b);
                continue;
            }
        };
        if dvp.nc.get(&p1).tree.is_empty() {
            vhold(dvp);
        }
        let neg = vp.is_none();
        if neg {
            nc.neg.push(b);
        } else {
            nc.lru.push(b);
        }
        let Some(e) = (if neg { nc.neg.last() } else { nc.lru.last() }) else {
            kpanic(format_args!("cache_enter: the entry is not on its list"));
        };
        if dvp.nc.get_mut(&mut p1).tree.insert(e).is_some() {
            // someone has raced us and added a different entry for the same vnode
            // (different ncp) - we don't need this entry, so free it and we are done.
            // We know now dvp->v_nc_tree is not empty, no need to vdrop here.
            let h = hold(e);
            let list = if neg { &mut nc.neg } else { &mut nc.lru };
            drop(list.take(h));
            break;
        }
        match vp {
            Some(vp) => {
                nc.numcache += 1;
                // don't put . or .. in the reverse map
                if reverse_mapped(dvp, vp, name) {
                    vp.nc.get_mut(&mut p1).dst.push_tail(Member::counted(e));
                }
            }
            None => nc.numneg += 1,
        }
        if nc.numneg > nc.initialvnodes
            && let Some(first) = nc.neg.first()
        {
            let h = hold(first);
            cache_zap(nc, &mut p1, h);
        }
        break;
    }
    if let Some(y) = yields {
        y.now();
    }
}

/// Name cache initialization, from `vfsinit` when we are booting: the pool and the cache's
/// size (`initialvnodes`).
pub fn nchinit<G: ProcGlue>(initialvnodes: i32) {
    let t = vfs::<G>();
    // A second call (a test's) keeps the first pool.
    let _ = t.nch_pool.init(IPL_NONE, PR_WAITOK, "nchpl");
    let mut g = NCLOCK.lock();
    let [mut p0] = g.split::<1>();
    cache_of::<G>(&mut p0).initialvnodes = i64::from(initialvnodes);
}

/// Cache flush, a particular vnode; called when a vnode is renamed to hide entries that
/// would now be invalid: every entry naming it or naming a child of it goes, and it gets a
/// new capability (`v_id`).
pub fn cache_purge<G: ProcGlue>(vp: &'static Vnode<G>) {
    let mut g = NCLOCK.lock();
    let [mut p0, mut p1] = g.split::<2>();
    let nc = cache_of::<G>(&mut p0);

    // We should never have destinations cached for a non-VDIR vnode.
    crate::assert::kassert!(vp.vtype() == VDIR || vp.nc.get(&p1).dst.is_empty());

    loop {
        let Some(h) = vp.nc.get(&p1).dst.first().map(hold) else {
            break;
        };
        cache_zap(nc, &mut p1, h);
    }
    loop {
        let Some(h) = vp.nc.get(&p1).tree.root().map(hold) else {
            break;
        };
        cache_zap(nc, &mut p1, h);
    }

    // XXX this blows goats
    nc.nextvnodeid = nc.nextvnodeid.wrapping_add(1);
    let mut id = nc.nextvnodeid as u32;
    if id == 0 {
        nc.nextvnodeid = nc.nextvnodeid.wrapping_add(1);
        id = nc.nextvnodeid as u32;
    }
    vp.set_id(id);
}

/// Cache flush, a whole filesystem; called when filesys is umounted to remove entries that
/// would now be invalid.
pub fn cache_purgevfs<G: ProcGlue>(mp: &'static Mount<G>) {
    let mut g = NCLOCK.lock();
    let [mut p0, mut p1] = g.split::<2>();
    let nc = cache_of::<G>(&mut p0);
    let of_mp = |e: &Namecache<G>| {
        e.nc_dvp
            .and_then(Vnode::mount)
            .is_some_and(|m| ptr::eq(m, mp))
    };

    // whack the regular entries
    let mut cur = nc.lru.first().map(hold);
    while let Some(h) = cur {
        let Some(e) = h.get() else { break };
        cur = nc.lru.next(e).map(hold);
        if of_mp(e) {
            // free the resources we had
            cache_zap(nc, &mut p1, h);
        }
    }
    // whack the negative entries
    let mut cur = nc.neg.first().map(hold);
    while let Some(h) = cur {
        let Some(e) = h.get() else { break };
        cur = nc.neg.next(e).map(hold);
        if of_mp(e) {
            cache_zap(nc, &mut p1, h);
        }
    }
}

/// The counts of the glue's cache (`numcache`, `numneg`), read under its lock.
pub fn cache_counts<G: ProcGlue>() -> (i64, i64) {
    let mut g = NCLOCK.lock();
    let [mut p0] = g.split::<1>();
    let nc = cache_of::<G>(&mut p0);
    (nc.numcache, nc.numneg)
}

impl<G: ProcGlue> crate::sysctl::SysctlNode for NchstatsNode<G> {
    /// `case KERN_NCHSTATS: return sysctl_rdstruct(oldp, oldlenp, newp, &nchstats, ...)`.
    fn sysctl(&self, _name: &[i32], req: &mut crate::sysctl::SysctlReq<'_>) -> Result<(), Errno> {
        let mut bytes = [0u8; 12 * size_of::<u64>()];
        let (words, _) = bytes.as_chunks_mut::<{ size_of::<u64>() }>();
        for (w, v) in words.iter_mut().zip(vfs::<G>().nchstats.snapshot()) {
            *w = v.to_ne_bytes();
        }
        crate::sysctl::rdstruct(req, &bytes)
    }
}

/// `namecache_test_reset`: the glue's cache forgotten (its entries live in the memory of an
/// earlier test, testing.md: never freed), its size kept.
#[cfg(any(test, feature = "doubles"))]
pub fn namecache_test_reset<G: ProcGlue>() {
    let mut g = NCLOCK.lock();
    let [mut p0] = g.split::<1>();
    let nc = cache_of::<G>(&mut p0);
    let initialvnodes = nc.initialvnodes;
    core::mem::forget(core::mem::replace(nc, NameCache::new()));
    nc.initialvnodes = initialvnodes;
}
/* </CODE> */
/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the name cache over the kernel's test file system: `cache_enter` and
    // `cache_lookup` hits (the vnode comes back referenced and locked, the parent as the flags
    // say), negative entries, the names it ignores (too long, `MAKEENTRY` clear), the
    // capability check after `cache_purge`, the reverse map of `cache_revlookup`,
    // `cache_purgevfs` (with entries of another mount between), the LRU moves and recycling,
    // and that `namei` fills and uses it (LZ's six tests, and the owned shape's own).

    use std::vec::Vec;

    use super::*;
    use crate::fs::lookup::{namei, ndinit};
    use crate::fs::namei::{FOLLOW, LOOKUP, NiDirp};
    use crate::fs::subr::vrele;
    use crate::fs::testfs::{
        self, A, B, C, LONG, NAMES, TestGlue, any_locked, node_of, test_thread, vget_node,
    };
    use crate::fs::vops::{Cred, vop_islocked};
    use crate::proc::Thread;

    type G = TestGlue;

    /// `kern.nchstats`: the twelve counters in the C's member order, read-only (`EPERM` for a
    /// new value), `ENOMEM` for a short room, the length always given back.
    #[test]
    fn kern_nchstats_reads_the_counters_in_the_c_s_order() {
        use crate::sysctl::{SysctlNode, SysctlReq};
        let (_g, _p) = test_thread();
        let st = &vfs::<G>().nchstats;
        st.ncs_goodhits.store(7, Mem::Relaxed);
        st.ncs_revmiss.store(9, Mem::Relaxed);
        st.ncs_dotdothits.store(11, Mem::Relaxed);
        let ubuf = crate::testutil::USER_BASE + 0xB000;
        let node = NchstatsNode::<G>::NODE;
        let mut len = 96;
        let mut req = SysctlReq {
            oldp: ubuf,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[], &mut req), Ok(()));
        assert_eq!(len, 96);
        let b = crate::testutil::user_bytes(ubuf, 96).unwrap();
        let word = |i: usize| u64::from_ne_bytes(b[i * 8..i * 8 + 8].try_into().unwrap());
        assert_eq!((word(0), word(9), word(11)), (7, 9, 11));
        assert_eq!(word(0), st.snapshot()[0]);
        let mut len = 95;
        let mut req = SysctlReq {
            oldp: ubuf,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[], &mut req), Err(Errno::ENOMEM));
        let mut len = 96;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: ubuf,
            newlen: 96,
        };
        assert_eq!(node.sysctl(&[], &mut req), Err(Errno::EPERM));
        let mut len = 0;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut len,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(node.sysctl(&[], &mut req), Ok(()));
        assert_eq!(len, 96, "a size query");
    }

    /// A componentname of `p` for `name` in a lookup with `flags`.
    fn cn<'p>(p: &'p Thread<G>, name: &'static [u8], op: u64, flags: u64) -> Componentname<'p, G> {
        let mut c = Componentname::new(op, flags, p, Cred::real(p.ucred()));
        c.set_name_lit(name);
        c
    }

    /// `(numcache, numneg)`.
    fn counts() -> (i64, i64) {
        cache_counts::<G>()
    }

    /// The test glue's statistics.
    fn stats() -> &'static Nchstats {
        &vfs::<G>().nchstats
    }

    /// The names on the regular LRU list, oldest first.
    fn lru_names() -> Vec<Vec<u8>> {
        let mut g = NCLOCK.lock();
        let [mut p0] = g.split::<1>();
        let nc = cache_of::<G>(&mut p0);
        nc.lru.iter().map(|e| e.name().to_vec()).collect()
    }

    /// The names in `dvp`'s tree, in its order, and how many entries name `dvp`.
    fn heads_of(vp: &Vnode<G>) -> (Vec<Vec<u8>>, usize) {
        let g = NCLOCK.lock();
        let h = vp.nc.get(&g);
        (
            h.tree.iter().map(|e| e.name().to_vec()).collect(),
            h.dst.iter().count(),
        )
    }

    /// The root's mount.
    fn root_mount() -> &'static crate::fs::mount::Mount<G> {
        let Some(mp) = crate::fs::init::rootvnode::<G>().and_then(Vnode::mount) else {
            panic!("no root mount");
        };
        mp
    }

    #[test]
    fn compare_orders_by_length_then_bytes() {
        assert_eq!(namecache_compare(b"b", b"aa"), Ordering::Less);
        assert_eq!(namecache_compare(b"ab", b"aa"), Ordering::Greater);
        assert_eq!(namecache_compare(b"ab", b"ab"), Ordering::Equal);
    }

    #[test]
    fn enter_then_hit() {
        let (_g, p) = test_thread();
        let mp = root_mount();
        let dvp = vget_node(mp, A).unwrap();
        let vp = vget_node(mp, C).unwrap();
        let _ = vop_unlock(vp);
        let uses = vp.usecount();

        cache_enter(dvp, Some(vp), b"c");
        assert_eq!(counts(), (1, 0));
        // A directory entry other than . and .. goes into the reverse map, and the parent is
        // held while it has entries.
        assert_eq!(heads_of(vp).1, 1);
        assert_eq!(dvp.holdcnt(), 1);

        // A hit on the last component without LOCKPARENT unlocks the parent.
        let mut c = cn(p, b"c", LOOKUP, MAKEENTRY | ISLASTCN);
        let hits = stats().ncs_goodhits.load(Mem::SeqCst);
        let found = cache_lookup(dvp, &mut c).unwrap().unwrap();
        assert!(ptr::eq(found, vp));
        assert_eq!(vp.usecount(), uses + 1);
        assert_ne!(vop_islocked(vp), 0);
        assert_eq!(vop_islocked(dvp), 0);
        assert!(c.cn_flags & PDIRUNLOCK != 0);
        assert_eq!(stats().ncs_goodhits.load(Mem::SeqCst), hits + 1);
        vput(vp);

        // With LOCKPARENT on the last component the parent stays locked.
        vn_lock(dvp, LK_EXCLUSIVE).unwrap();
        let mut c = cn(p, b"c", LOOKUP, MAKEENTRY | ISLASTCN | LOCKPARENT);
        let found = cache_lookup(dvp, &mut c).unwrap().unwrap();
        assert_ne!(vop_islocked(dvp), 0);
        assert!(c.cn_flags & PDIRUNLOCK == 0);
        vput(found);

        // ".." of c names a: the parent is unlocked first, then relocked for LOCKPARENT.
        cache_enter(vp, Some(dvp), b"..");
        assert_eq!(heads_of(dvp).1, 0, ".. is not in the reverse map");
        vn_lock(vp, LK_EXCLUSIVE).unwrap();
        let _ = vop_unlock(dvp);
        let mut c = cn(
            p,
            b"..",
            LOOKUP,
            MAKEENTRY | ISLASTCN | ISDOTDOT | LOCKPARENT,
        );
        let found = cache_lookup(vp, &mut c).unwrap().unwrap();
        assert!(ptr::eq(found, dvp));
        assert_ne!(vop_islocked(vp), 0);
        assert!(c.cn_flags & PDIRUNLOCK == 0);
        vput(found);
        vput(vp);
        vrele(dvp);
        assert!(!any_locked());
    }

    #[test]
    fn negative_entries_and_misses() {
        let (_g, p) = test_thread();
        let dvp = vget_node(root_mount(), A).unwrap();

        let mut c = cn(p, b"nope", LOOKUP, MAKEENTRY);
        let miss = stats().ncs_miss.load(Mem::SeqCst);
        assert!(matches!(cache_lookup(dvp, &mut c), Ok(None))); // a miss
        assert_eq!(stats().ncs_miss.load(Mem::SeqCst), miss + 1);
        cache_enter(dvp, None, c.name());
        assert_eq!(counts(), (0, 1));
        let neg = stats().ncs_neghits.load(Mem::SeqCst);
        assert!(matches!(cache_lookup(dvp, &mut c), Err(Errno::ENOENT)));
        assert_eq!(stats().ncs_neghits.load(Mem::SeqCst), neg + 1);

        // CREATE of the last component drops the negative entry instead.
        let mut c = cn(p, b"nope", CREATE, MAKEENTRY | ISLASTCN);
        assert!(matches!(cache_lookup(dvp, &mut c), Ok(None)));
        assert_eq!(counts(), (0, 0));
        assert_eq!(dvp.holdcnt(), 0, "the last entry gone, the hold is dropped");

        // Names longer than NAMECACHE_MAXLEN are never cached.
        cache_enter(dvp, None, NAMES[LONG].0);
        assert_eq!(counts(), (0, 0));
        let mut long = cn(p, NAMES[LONG].0, LOOKUP, MAKEENTRY);
        let n = stats().ncs_long.load(Mem::SeqCst);
        assert!(matches!(cache_lookup(dvp, &mut long), Ok(None)));
        assert_eq!(stats().ncs_long.load(Mem::SeqCst), n + 1);
        assert!(long.cn_flags & MAKEENTRY == 0);

        // A hit without MAKEENTRY drops the entry (a "bad hit").
        cache_enter(dvp, None, b"gone");
        let mut c = cn(p, b"gone", LOOKUP, 0);
        let bad = stats().ncs_badhits.load(Mem::SeqCst);
        assert!(matches!(cache_lookup(dvp, &mut c), Ok(None)));
        assert_eq!(stats().ncs_badhits.load(Mem::SeqCst), bad + 1);
        assert_eq!(counts(), (0, 0));
        vput(dvp);
    }

    #[test]
    fn purge_invalidates_entries_naming_the_vnode() {
        let (_g, p) = test_thread();
        let mp = root_mount();
        let dvp = vget_node(mp, A).unwrap();
        let vp = vget_node(mp, B).unwrap();
        let _ = vop_unlock(vp);
        cache_enter(dvp, Some(vp), b"b");
        assert_eq!(heads_of(dvp).0.len(), 1);
        assert_eq!(dvp.holdcnt(), 1, "a directory with entries is held");

        let id = vp.id();
        forbid_sleep(|| cache_purge(vp));
        assert_ne!(vp.id(), id);
        assert_ne!(vp.id(), 0);
        // The entry is still in the parent's tree but its capability is stale: a false hit.
        let mut c = cn(p, b"b", LOOKUP, MAKEENTRY | ISLASTCN);
        let falsehits = stats().ncs_falsehits.load(Mem::SeqCst);
        assert!(matches!(cache_lookup(dvp, &mut c), Ok(None)));
        assert_eq!(stats().ncs_falsehits.load(Mem::SeqCst), falsehits + 1);
        assert!(heads_of(dvp).0.is_empty());
        assert_eq!(dvp.holdcnt(), 0, "the last entry gone, the hold is dropped");

        // Purging the directory empties its own tree, and its reverse list's entries.
        cache_enter(dvp, Some(vp), b"b");
        let cvp = vget_node(mp, C).unwrap();
        let _ = vop_unlock(cvp);
        cache_enter(cvp, Some(dvp), b"up"); // an entry naming dvp, from another directory
        assert_eq!(heads_of(dvp).1, 1);
        cache_purge(dvp);
        assert_eq!(heads_of(dvp), (Vec::new(), 0));
        assert_eq!(counts(), (0, 0));
        vrele(cvp);
        vrele(vp);
        vput(dvp);
    }

    #[test]
    fn revlookup_finds_the_parent_and_the_name() {
        let (_g, _p) = test_thread();
        let mp = root_mount();
        let dvp = vget_node(mp, A).unwrap();
        let vp = vget_node(mp, C).unwrap();
        let mut buf = [0u8; 8];
        let mut bp = buf.len();
        let miss = stats().ncs_revmiss.load(Mem::SeqCst);
        assert!(matches!(
            cache_revlookup(vp, &mut bp, Some(&mut buf)),
            Ok(None)
        ));
        assert_eq!(stats().ncs_revmiss.load(Mem::SeqCst), miss + 1);
        cache_enter(dvp, Some(vp), b"c");
        let found = forbid_sleep(|| cache_revlookup(vp, &mut bp, Some(&mut buf))).unwrap();
        assert!(found.is_some_and(|d| ptr::eq(d, dvp)));
        assert_eq!((bp, &buf[bp..]), (7, &b"c"[..]));
        // No room for the name: ERANGE.
        let mut small = [0u8; 1];
        let mut bp = 1;
        assert!(matches!(
            cache_revlookup(vp, &mut bp, Some(&mut small)),
            Err(Errno::ERANGE)
        ));
        // The parent purged: its entries are gone, the reverse list with them.
        cache_purge(dvp);
        let mut bp = buf.len();
        assert!(matches!(cache_revlookup(vp, &mut bp, None), Ok(None)));
        vput(vp);
        vput(dvp);
    }

    #[test]
    fn namei_fills_the_cache_and_hits_it() {
        let (_g, p) = test_thread();
        let lookup = || {
            let mut nd = ndinit(LOOKUP, FOLLOW, NiDirp::Sys(b"/a/c"), p);
            namei(&mut nd).unwrap();
            let vp = nd.ni_vp.unwrap();
            assert!(node_of(vp).is_some_and(|n| n.idx == C));
            vrele(vp);
        };
        lookup();
        let hits = stats().ncs_goodhits.load(Mem::SeqCst);
        lookup();
        // "a" and "c" both come from the cache the second time.
        assert_eq!(stats().ncs_goodhits.load(Mem::SeqCst), hits + 2);
        assert!(!any_locked());

        // Unmounting purges the file system's entries.
        assert!(counts().0 >= 2);
        forbid_sleep(|| cache_purgevfs(root_mount()));
        assert_eq!(counts(), (0, 0));
    }

    #[test]
    fn a_hit_moves_its_entry_to_the_lru_tail_keeping_its_links() {
        let (_g, p) = test_thread();
        let mp = root_mount();
        let dvp = vget_node(mp, A).unwrap();
        let (bvp, cvp) = (vget_node(mp, B).unwrap(), vget_node(mp, C).unwrap());
        let _ = vop_unlock(bvp);
        let _ = vop_unlock(cvp);
        cache_enter(dvp, Some(cvp), b"c");
        cache_enter(dvp, Some(bvp), b"b");
        assert_eq!(lru_names(), [b"c".to_vec(), b"b".to_vec()]);
        vn_lock(dvp, LK_EXCLUSIVE).unwrap();
        let mut c = cn(p, b"c", LOOKUP, MAKEENTRY | ISLASTCN | LOCKPARENT);
        let found = cache_lookup(dvp, &mut c).unwrap().unwrap();
        vput(found);
        assert_eq!(lru_names(), [b"b".to_vec(), b"c".to_vec()]);
        // The tree (ordered by length, then bytes) and c's reverse list are as they were.
        assert_eq!(heads_of(dvp).0, [b"b".to_vec(), b"c".to_vec()]);
        assert_eq!(heads_of(cvp).1, 1);
        cache_purge(dvp);
        vrele(bvp);
        vrele(cvp);
        vput(dvp);
    }

    #[test]
    fn a_full_cache_recycles_its_oldest_entries() {
        let (_g, _p) = test_thread();
        let mp = root_mount();
        let dvp = vget_node(mp, A).unwrap();
        let bvp = vget_node(mp, B).unwrap();
        let _ = vop_unlock(bvp);
        nchinit::<G>(2);
        cache_enter(dvp, Some(bvp), b"one");
        cache_enter(dvp, Some(bvp), b"two");
        assert_eq!(counts(), (2, 0));
        // At initialvnodes entries the LRU's head goes for each new one.
        forbid_sleep(|| cache_enter(dvp, Some(bvp), b"three"));
        assert_eq!(counts(), (2, 0));
        assert_eq!(lru_names(), [b"two".to_vec(), b"three".to_vec()]);
        assert_eq!(heads_of(dvp).0, [b"two".to_vec(), b"three".to_vec()]);
        // Negative entries past initialvnodes: the negative LRU's head goes.
        cache_enter(dvp, None, b"n1");
        cache_enter(dvp, None, b"n2");
        cache_enter(dvp, None, b"n3");
        let (_, numneg) = counts();
        assert!(numneg <= 2, "numneg {numneg}");
        cache_purge(dvp);
        assert_eq!(counts(), (0, 0));
        nchinit::<G>(64);
        vrele(bvp);
        vput(dvp);
    }

    #[test]
    fn purgevfs_takes_one_mounts_entries_from_among_anothers() {
        let (_g, _p) = test_thread();
        let mp = root_mount();
        let avp = vget_node(mp, A).unwrap();
        let cvp = vget_node(mp, C).unwrap();
        let _ = vop_unlock(cvp);
        let mp2 = testfs::mount_on(cvp);
        let root2 = vget_node(mp2, testfs::ROOT).unwrap();
        let a2 = vget_node(mp2, A).unwrap();
        let _ = vop_unlock(a2);
        // Entries of the two mounts interleaved on both LRU lists.
        cache_enter(avp, Some(cvp), b"c");
        cache_enter(root2, Some(a2), b"a");
        cache_enter(avp, None, b"x");
        cache_enter(root2, None, b"y");
        cache_enter(avp, None, b"z");
        assert_eq!(counts(), (2, 3));
        cache_purgevfs(mp);
        assert_eq!(counts(), (1, 1));
        assert_eq!(lru_names(), [b"a".to_vec()]);
        assert!(heads_of(avp).0.is_empty());
        cache_purgevfs(mp2);
        assert_eq!(counts(), (0, 0));
        vrele(a2);
        vput(root2);
        vrele(cvp);
        vput(avp);
    }

    /// `f` with `testutil::forbid_sleep` on: the cache's sections never sleep.
    fn forbid_sleep<R>(f: impl FnOnce() -> R) -> R {
        crate::testutil::forbid_sleep(true);
        let r = f();
        crate::testutil::forbid_sleep(false);
        r
    }
}
/* </TESTS> */
