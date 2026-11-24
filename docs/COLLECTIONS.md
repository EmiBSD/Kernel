# Collections: the cost of every method

The reference for N12 (performance) and for the planner, the redesigner and the reviewer
(the rule of 2026-10-29, `4b57e42b`: every walk at the C's cost, step by step). One row per
method of every collection the `kernel` crate uses, with the OpenBSD macro it stands for and
its order. Taken from the implementations in `sys/hal/` on 2026-10-29 (list.rs, pinlist.rs,
tree.rs, owned.rs, pool.rs, malloc.rs, kmbuf.rs, mapped.rs, slot.rs, percpu.rs, refcnt.rs),
not from their doc comments. `n` is the number of elements in the container; `k` the number
of elements a walk visits before it stops.

Four families of intrusive containers exist, and the difference is who owns the element and
which lock guards the links (`docs/IDIOMS.md`: the rows on the `'static` TAILQ/RBT, the owned map entries, the embedded `struct timeout`, the two-phase run-queue move and the buffer cache queues):

| Family | Element | Lock | Since |
|---|---|---|---|
| `hal::list::List`, `hal::tree::Tree` | `&'static`, never freed (the page array, the pmemranges) | the caller's, unchecked | 2026-10-17 (N3) |
| `hal::owned::OwnedTree`, `OwnedList` (+ `SecondaryTree`, `SecondaryList`, `Member`, `SecondaryRef`) | a `PoolBox` the container owns and gives back | `&mut self` | 2026-10-18 (N3) |
| `hal::pinlist::PinList`, `PinRing` | pinned anywhere, a stack frame included | a witness of one lock, checked at every call | 2026-10-21 (N4) |

Every method of these containers does, besides what the table says, a constant number of
checks: `List`/`Tree` an atomic claim of the entry on insert; `PinList`/`PinRing` one atomic
compare of the lock's identity (`bind`) and one of the element's (`linked_entry`); `Owned*`
one poison load and one identity compare. None of them walks to check.

## `hal::list::List` (TAILQ, `&'static` elements)

| Method | C | Order |
|---|---|---|
| `new` | `TAILQ_HEAD_INITIALIZER` | O(1) |
| `is_empty` | `TAILQ_EMPTY` | O(1) |
| `first`, `last` | `TAILQ_FIRST`, `TAILQ_LAST` | O(1) |
| `next(e)`, `prev(e)` | `TAILQ_NEXT`, `TAILQ_PREV` | O(1) |
| `is_linked(e)` | | O(1) |
| `insert_head`, `insert_tail` | `TAILQ_INSERT_HEAD`, `TAILQ_INSERT_TAIL` | O(1) |
| `insert_before(at, e)`, `insert_after(at, e)` | `TAILQ_INSERT_BEFORE`, `TAILQ_INSERT_AFTER` | O(1) |
| `remove(e)` | `TAILQ_REMOVE` | O(1) |
| `pop_head` | `TAILQ_FIRST` + `TAILQ_REMOVE` | O(1) |
| `concat(other)` | `TAILQ_CONCAT` | O(1) |
| `iter` | `TAILQ_FOREACH_SAFE` | O(1) to make, O(1) a step, O(n) the walk |

## `hal::pinlist::PinList` (TAILQ, pinned elements, one lock)

| Method | C | Order |
|---|---|---|
| `new` | `TAILQ_HEAD_INITIALIZER` | O(1) |
| `is_empty(w)` | `TAILQ_EMPTY` | O(1) |
| `first(w)`, `last(w)`, `first_pinned(w)` | `TAILQ_FIRST`, `TAILQ_LAST` | O(1) |
| `next(w, e)` | `TAILQ_NEXT` | O(1) |
| `iter(w)`, `iter_pinned(w)` | `TAILQ_FOREACH` | O(1) a step, O(n) the walk |
| `insert_head`, `insert_tail` | `TAILQ_INSERT_HEAD`, `TAILQ_INSERT_TAIL` | O(1) |
| `insert_after(w, after, e)` | `TAILQ_INSERT_AFTER` | O(1) |
| `try_insert_tail` | `TAILQ_INSERT_TAIL` if not linked (task_add) | O(1) |
| `step_over(w, e)` | `TAILQ_REMOVE` + `TAILQ_INSERT_AFTER(next)` | O(1) |
| `remove(w, e)` | `TAILQ_REMOVE` | O(1) |
| `remove_first(w)` | `TAILQ_FIRST` + `TAILQ_REMOVE` | O(1) |
| `move_last_to_head(w)` | `TAILQ_LAST` + `TAILQ_REMOVE` + `TAILQ_INSERT_HEAD` (`nd6_rtrequest`'s forced GC) | O(1) |
| `hal::link::TaggedLink::move_b_to_head(w, list)` | `TAILQ_REMOVE` + `TAILQ_INSERT_HEAD` of the record a route's word owns (`nd6_resolve`) | O(1): one load of the word, the two link changes |
| `remove_where(w, pred)` | `TAILQ_FOREACH` until `pred`, `TAILQ_REMOVE` | O(k), O(n) worst: a walk from the head |
| `move_first_to(w, dst)` | `TAILQ_REMOVE` + `TAILQ_INSERT_TAIL(dst)` | O(1) |
| `move_where(w, max, pred, dst)` | `wakeup_n`'s walk | O(k): walks until `max` moved or the end, O(n) worst |
| `move_where_across(w, max, pred, dst, wd)` | `FUTEX_REQUEUE`'s walk | O(k), O(n) worst |
| `concat(w, other)` | `TAILQ_CONCAT` | O(1) |

## `hal::pinlist::PinRing` and `BoundRing` (`struct circq`, timeouts)

| Method | C | Order |
|---|---|---|
| `new` | | O(1) |
| `is_empty(w)`, `first(w)` | `CIRCQ_EMPTY`, `CIRCQ_FIRST` | O(1) |
| `iter(w)` | `CIRCQ_FOREACH` | O(1) a step, O(n) the walk |
| `insert_head`, `insert_tail` | `CIRCQ_INSERT_HEAD`, `CIRCQ_INSERT_TAIL` | O(1) |
| `remove(w, e)` | `CIRCQ_REMOVE` (no list named) | O(1) |
| `remove_first(w)` | `CIRCQ_FIRST` + `CIRCQ_REMOVE` | O(1) |
| `move_first(w, dst)` | `CIRCQ_REMOVE` + `CIRCQ_INSERT_TAIL(dst)` | O(1) |
| `concat(w, other)` | `CIRCQ_CONCAT` | O(1): four pointer writes |
| `bound(w)` | | O(1): the binding checked once for the view |
| `BoundRing::{witness, data_mut, first, remove_first, move_first, concat}` | as above | O(1) each |

## `hal::tree::Tree` (RBT, `&'static` elements)

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `root` | `RBT_INITIALIZER`, `RBT_EMPTY`, `RBT_ROOT` | O(1) |
| `left(e)`, `right(e)`, `parent(e)`, `is_linked(e)` | `RBT_LEFT`, `RBT_RIGHT`, `RBT_PARENT` | O(1) |
| `min`, `max` | `RBT_MIN`, `RBT_MAX` | O(log n) |
| `next(e)`, `prev(e)` | `RBT_NEXT`, `RBT_PREV` | O(log n) worst a step; O(1) amortized over a full in-order walk |
| `find`, `nfind`, `find_by`, `nfind_by` | `RBT_FIND`, `RBT_NFIND` | O(log n) |
| `iter` | `RBT_FOREACH_SAFE` | O(log n) to make (`min`), O(1) amortized a step, O(n) the walk |
| `insert(e)` | `RBT_INSERT` | O(log n): the search, then recolouring up with at most two rotations |
| `remove(e)` | `RBT_REMOVE` | O(log n): the successor when two children, then the fix-up |

### Note: `next` and `prev` are O(1) only inside a walk

The successor of one element (the C's `_rb_next`, `subr_tree.c`: down to the minimum of the
right subtree, or up to the first ancestor reached from a left child) costs up to the
tree's height, O(log n), when taken alone. Over a full in-order walk (`min`, then `next`
until `None`, which is what `iter` does) every edge of the tree is crossed at most twice,
once down and once up, so n steps cost at most 2(n - 1) moves: O(n) the walk, O(1)
amortized a step. The same holds for `prev` from `max`, and for every RBT of this file
(`Tree`, `OwnedTree`, `SecondaryTree`).

The amortized bound needs the walk to go on from the element it holds. It is lost when the
element is found again at each step: `find(key)` is O(log n) from the root every time, so
`find` + `next` per step is O(n log n) a walk. That was N3's `next_key` in
`sys/kernel/vm/map.rs` (a walk that kept only the entry's key, since a reference could not
outlive the `&mut`), in 13 places, removed by N12.1 (`a3743945`): the walks now hold their
entry through a counted link and call `next(e)`. Restarting a walk from `find` after each
change of the tree, as the C itself does in a few places, has the same cost and is taken
only where the C takes it, as a named decision in the brief. The same applies to a `PinList`
or `OwnedList` walk: `next(e)` is O(1), `remove_where`/`move_where` from the head at each
step is not.

## `hal::owned::OwnedTree` (RBT that owns its `PoolBox` elements; the map's address tree)

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `root` | `RBT_INITIALIZER`, `RBT_EMPTY`, `RBT_ROOT` | O(1) |
| `contains(e)` | | O(1): an identity compare, no search |
| `left`, `right`, `parent` | `RBT_LEFT`, `RBT_RIGHT`, `RBT_PARENT` | O(1) |
| `min`, `max` | `RBT_MIN`, `RBT_MAX` | O(log n) |
| `next(e)`, `prev(e)` | `RBT_NEXT`, `RBT_PREV` | O(log n) worst a step; O(1) amortized over a walk (the note under `Tree`) |
| `find`, `nfind`, `find_by`, `nfind_by` | `RBT_FIND`, `RBT_NFIND` | O(log n) |
| `iter` | `RBT_FOREACH` | O(log n) to make, O(1) amortized a step, O(n) the walk |
| `insert(box)`, `insert_get(box)` | `RBT_INSERT` | O(log n); an augmented tree adds the augment walk up the ancestors, O(log n) worst, which stops at the first unchanged summary |
| `take_by(key_vs)` | `RBT_FIND` + `RBT_REMOVE` | O(log n), plus the augment walk when augmented |
| `augment(e)` | `RBT_AUGMENT` | O(log n) worst; O(1) when `e`'s summary does not change; nothing on a tree that is not augmented |
| `drain_into(dead)` | `uvm_map_teardown`'s walk + `RBT_INIT` | O(n), no rebalancing |

## `hal::owned::OwnedList` (TAILQ that owns its `PoolBox` elements; the dead queue, knotes)

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `first`, `last` | `TAILQ_*` | O(1) |
| `next(e)`, `prev(e)`, `contains(e)` | `TAILQ_NEXT`, `TAILQ_PREV` | O(1) |
| `iter` | `TAILQ_FOREACH` | O(1) a step, O(n) the walk |
| `push(box)` | `TAILQ_INSERT_TAIL` | O(1) |
| `insert_before(at, box)`, `insert_after(at, box)` | `TAILQ_INSERT_BEFORE`, `TAILQ_INSERT_AFTER` | O(1) |
| `move_to_tail(e)` | `TAILQ_REMOVE` + `TAILQ_INSERT_TAIL` of an element the list keeps (the name cache's LRU) | O(1) |
| `move_before(at, e)`, `move_after(at, e)` | `TAILQ_REMOVE` + `TAILQ_INSERT_BEFORE`, `_AFTER` of an element the list keeps | O(1) |
| `pop()` | `TAILQ_FIRST` + `TAILQ_REMOVE` | O(1) |
| `take(r)` | `TAILQ_REMOVE(e)` by a counted reference, anywhere on the list | O(1), no walk (the stored pointer read from the predecessor's link) |

## `hal::owned::OwnedSlist` (singly-linked, the links own the next element; mbuf chains, packet lists, socket buffers)

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `len`, `first` | `MBUF_LIST_INITIALIZER`, `ml_empty`, `ml_len`, `ml_first` | O(1) |
| `last` | `ml_tail`, `sb_mbtail` | O(1) (the tail word) |
| `next(e)` | `m->m_next`, `m->m_nextpkt` | O(1) |
| `iter` | `for (m = first; m != NULL; m = m->m_next)` | O(1) a step, O(n) the walk |
| `push_head(box)` | insert first | O(1) |
| `push_tail(box)` | `ml_enqueue`, `sbappend`'s tail | O(1) |
| `pop_head()` | `ml_dequeue` | O(1) |
| `concat(&mut other)` | `ml_enlist`, `m_cat`'s chain | O(1) |
| `into_head()` | a chain built at its tail handed over whole (`m_copym`'s `np = &n->m_next`), the first element owning the rest | O(1) |
| `cursor_mut()`, `move_next`, `current` | a walk that changes the list where it stands (elements lent shared only) | O(1) a step |
| `split_after()` (cursor) | `m_split`, `sbdrop`'s record cut | O(1) |
| `remove_next()` (cursor) | `m->m_next = n->m_next` | O(1) |
| drop | `m_freem`, `ml_purge` | O(n), one element at a time (no recursion down the links) |

A bare `hal::owned::SlistLink` outside any list (an mbuf's `m_next`, a packet header's
`ph_tags`) is a chain its owner walks and changes itself:

| Method | C | Order |
|---|---|---|
| `get`, `get_mut`, `is_some` | `m->m_next`, `m->m_next != NULL` | O(1) |
| `set(box)` | `m->m_next = n` (the box linked before given back) | O(1) |
| `take()` | `n = m->m_next; m->m_next = NULL` | O(1) |
| drop | the element linked freed with the link (its owner empties a long chain first, one element at a time) | O(1) |

## `hal::owned::SecondaryTree` (a second RBT over owned elements; the free-space trees)

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `root`, `contains`, `left`, `right`, `parent` | `RBT_*` | O(1) |
| `min`, `max`, `find_by`, `nfind_by` | `RBT_MIN`, `RBT_MAX`, `RBT_FIND`, `RBT_NFIND` | O(log n) |
| `next(e)`, `prev(e)` | `RBT_NEXT`, `RBT_PREV` | O(log n) worst a step; O(1) amortized over a walk (the note under `Tree`) |
| `iter` | `RBT_FOREACH` | O(log n) to make, O(1) amortized a step, O(n) the walk |
| `insert(e)` | `RBT_INSERT` | O(log n) |
| `remove(e)` | `RBT_REMOVE` | O(log n) |
| `clear()` | `RBT_INIT` of a tree being destroyed | O(n), no rebalancing |

## `hal::owned::SecondaryList` (a second TAILQ over owned or `'static` elements; the buffer cache queues, `bufq`)

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `first`, `last`, `next`, `prev`, `contains` | `TAILQ_*` | O(1) |
| `iter` | `TAILQ_FOREACH` | O(1) a step, O(n) the walk |
| `push_head(m)`, `push_tail(m)` | `TAILQ_INSERT_HEAD`, `TAILQ_INSERT_TAIL` | O(1) |
| `insert_before_first(m, before)` | `bufq_simple_nscan`'s walk + `SIMPLEQ_INSERT_AFTER` | O(k), O(n) worst: the C's own walk |
| `remove(e)` | `TAILQ_REMOVE` | O(1) |
| `pop_head()` | `TAILQ_FIRST` + `TAILQ_REMOVE` | O(1) |
| `clear()` | `TAILQ_INIT` of a list being torn down | O(n) |

`Member` (`counted`, `of_static`, `from_ref`, `into_ref`, `get`, `is_counted`, drop) and
`SecondaryRef` (`new`, `get`, `is_some`, `points_to`, `set`, drop): O(1) each, one atomic
count up or down.

## `kernel::net::radix::RadixTree` (`net/radix.c`; the export lists, at N6.7b1 pf's tables and the SPD)

Not a `hal` collection: a kernel module that owns its nodes in index slabs (no `unsafe`), listed
here because its walk is a collection's. `b` is the key's bits from the tree's offset, `c` the
length of a duplicated key chain, `a` the annotations met on the way up. Every node access is
two loads (the slab's directory entry, then the slot) where the C has one.

| Method | C | Order |
|---|---|---|
| `new`, `is_empty`, `inithead` | `rn_inithead` (`new` allocates nothing; `inithead`, which the first add calls, the roots' first chunks) | O(1) |
| `matches(v)` | `rn_match` | O(b) down, O(c) the chain, O(b + a) back up with `rn_search_m` per non-normal annotation: the C's |
| `lookup(v, m)` | `rn_lookup` | `matches` + the mask's normalisation O(K) + O(c) |
| `add(v, m, value)` | `rn_addroute` | O(b) search and insert, O(K) the mask, O(c) the chain, O(a) the annotation list; a chunk when the slots run out (8, 8, 16, 32, then 64 slots: the capacity doubles up to 64 a chunk), the directory moved (O(n/64) handles) when it doubles |
| `delete(v, m)` | `rn_delete` | O(b) + O(K) + O(c) + O(a); D-20's repair walks the chain once (O(c): after the route, then before it), and the subtree of the annotation's node only when no route of the chain holds the annotation (a route of another key shares it) |
| `remove(leaf)` | `rn_delete` of the leaf's key and mask | O(c) to the chain's head + O(a); no search |
| `first()`, `next(leaf)` | `rn_walktree` | O(1) amortised a route: the C's successor climb, once a chain, plus the climb from a chain's last route to its head (O(c), once a chain); never a search by key |
| `get(leaf)` | the leaf-to-owner cast | O(1): the generation compared |

## Owned memory: pools, `malloc(9)`, `km_alloc`

Orders of the C's algorithms the `hal` types wrap (`subr_pool.c`, `kern_malloc.c`,
`uvm_km.c`), not re-measured here.

| Type / method | C | Order |
|---|---|---|
| `TypedPool::get_shared`, `get_homed`, `BoxPool::get`, `get_idle`, `get_linked` | `pool_get` | O(1) amortized: the per-CPU cache or the page's free list; a page grow when empty (`pool_allocator`, O(items per page) to carve it) |
| drop of a `PoolBox`, the last `PoolArc` | `pool_put` | O(1) amortized; a page release when it empties |
| `TypedPool::set_hardlimit`, `BoxPool::set_hardlimit` (N6.6a0) | `pool_sethardlimit` | O(1): the pool's lock pair and one compare with `pr_nout` |
| `PoolArc::from_ref`, `try_from_ref`, `clone`, `count`, `ptr_eq`, `try_last`, `get_mut`, `erase`, `leak`, `as_pin`, `as_ptr` | the refcount | O(1), one atomic |
| `PoolBox::leak`, `as_pin`, `as_ptr`, `ptr_eq` | | O(1) |
| `StaticArena::extend`, `init_arena` | | O(N) once, at init |
| `MBox::new`, `MArray::new` | `malloc(9)` | O(1) amortized: the bucket's free list; `km_alloc` when the bucket is empty (below) |
| drop of an `MBox`/`MArray` | `free(9)` | O(1) |
| `MArc::new`; drop of the last `MArc` | `malloc(9)`, `free(9)` | as `MBox::new` and its drop, plus one atomic |
| `MArc::clone`, drop of a handle | the refcount | O(1), one atomic |
| `MArc::ptr_eq`, `erase`, `as_pin` | | O(1), no atomic |
| `MArc::count` | `ifa_refcnt` read (ddb, the tests) | O(1), one relaxed load |
| `KmBuf::alloc_wired`, `alloc_pageable` | `km_alloc` | O(pages) plus the kernel map's entry allocation, O(log m) in the map's entries |
| drop of a `KmBuf` | `km_free` | O(pages) + O(log m) |
| `MappedBytes::map_frames`, `kenter_frames`, `map_direct` | `pmap_kenter_pa` per frame | O(frames) |
| `MappedBytes::unmap`, drop | `pmap_kremove` | O(frames) |

## Cells and counters

| Type / method | Order |
|---|---|
| `ReadMostly::read` | O(1): one compare-and-swap; spins while a writer holds it |
| `ReadMostly::update`, `replace` | O(1) plus a spin until the readers leave (it is for tables read often and written rarely: the pin tables, `hw.*` strings) |
| `PerCpu::get`, `cpu` | O(1) |
| `PerCpu::iter`, `Counters::read`, `zero` | O(ncpu) |
| `Refcnt::take`, `rele`, `rele_wake`, `read`, `shared` | O(1); `finalize` sleeps until the count is 0 |
| `OnceStatic::get`, `set`; `Mutexed::lock`; `Rwlocked::read`, `write` | O(1) apart from the wait for the lock |
| `Link::addr` | O(1): one relaxed load (the address printed, nothing followed) |
| `SyncLink::get`, `set`, `replace`, `addr`, `is_none`, `points_to` | O(1): one atomic load, store or swap (`fd_cdir`, `fd_rdir`, `s_ttyvp`) |
| `SaWords::read` (a record's own socket address copied, N6.5e) | O(1): at most four relaxed loads (`SA_READ_MAX` bytes) |
| `Guarded::new_net` (a net cell, N6.5e), its `get`, `get_mut` | O(1): `const` to make; an access is one key compare, as every `Guarded`'s |
| `LoanSlot::update`, `replace`, `take` (a value lent out and put back, never shared: `inp_options`, N6.6a0) | O(1): one compare-and-swap in and one store out (the `ReadMostly` writer's claim), spinning while another user holds it; no readers |
| `ExclGuarded::get`, `get_mut` (a `Guarded` of a `Send`-only value, opened by exclusive witnesses only, N6.6a0) | O(1): the key compare of every `Guarded` |
| `IfaList::first`, `IfaList::next`, `IfaList::iter` a step (an interface's addresses, N6.5e) | O(1): the cell's key compare, the handle's arm, one load |
| `IfaList::insert_tail`, `IfaList::remove` (`ifa_add`, `ifa_del`) | O(n): a walk to the last address or to the predecessor, one handle clone and drop for that one's cell (the C's `TAILQ` is O(1); control paths only, D-4) |
| `IfaList::for_each_safe` (`TAILQ_FOREACH_SAFE`) | O(1) a step plus a reader made and two handle clones (the current and the next) per step, so that the body may take the writer |
| `IfmaList::first`, `IfmaList::next`, `IfmaList::iter` a step (an interface's multicast records, N6.5e's 5e2) | O(1): the cell's key compare, the handle's arm, one load; reaching the list (`Ifnet::if_maddrlist`) reads two set-once cells' states, `Ifnet::rt`'s and the list's: two acquire loads where the C reads its head inside the interface (`in_lookupmulti`); `in_hasmulti` shares the first with its lock's, one more than before 5e2 |
| `IfmaList::insert_head`, `IfmaList::pop_head` (`in_addmulti`'s `TAILQ_INSERT_HEAD`, `if_detach`) | O(1), as the C's |
| `IfmaList::remove` (`in_delmulti`'s `TAILQ_REMOVE`, a group's last leave) | O(n): a walk to the predecessor, one handle clone and drop for its cell (the C's `TAILQ` is O(1); control path only, D-4) |
| `NetHeld::reader` and its drop (`NET_ASSERT_LOCKED()`) | O(1): one load of the lock word (`rw_status`, the C's own under `DIAGNOSTIC`); on the write holding one uncontended relaxed compare-and-swap, and one relaxed decrement at the drop; nothing on a read holding |
| `NetHeld::writer` and its drop (`NET_ASSERT_LOCKED_EXCLUSIVE()`) | O(1): one load of the lock word, one uncontended relaxed compare-and-swap; one relaxed store at the drop |
| `SaToken::lend_net` and the drop of its `NetLend` (`bsd`'s `NET_ASSERT_LOCKED()` before a call into the kernel, N6.5e) | O(1): one load of the lock word; on the write holding one uncontended read-modify-write of the lend count, and one at the drop; on a read holding a compare-and-swap of the lock word (the joined share, which every reader writes: contended), and one at the drop (`rw_exit_read`). A pair the C lacks, per `bsd` call; per packet on `bsd`'s TCP/UDP/raw output and divert (until N6.6), pf's route-to and `rtalloc`, IP-in-IP and wg(4) (until N8): the list in `sys/hal/netlock.rs`'s notes |
| A write witness's release (`WriteWitness` drop, sleeps, `unlocked`, `downgrade`) | the lock's own, plus one address compare (the net lock's lend and borrow checks, two relaxed loads, for `NETLOCK` alone) |
| `DetachableRwlock::detach`, `attach` (a witness given up for a detached holding and taken back, N6.6a0), `enter_write_detached`, `exit_write_detached` | O(1): one store (`detach`) or one compare-and-swap (`attach`, the exits) of the detached word, on the lock's line, beside the lock's own enter or exit; `attach` never waits |

## `alloc` in the `kernel` crate (six sites, test code apart)

| Site | Type | Methods | Order |
|---|---|---|---|
| `thread.rs` `STARTS` | `Vec<Option<Start>>` | `push`, `get`, `get_mut`, `len` | `push` O(1) amortized (a reallocation copies O(n)), the rest O(1) |
| `thread.rs` `DEFERRED` | `VecDeque<Deferred>` | `push_back`, `pop_front` | O(1) amortized / O(1) |
| `tc.rs` | `vec![0u8; n*32]` | `extend_from_slice`, `resize` | O(k) / O(n) |
| `crypto/xform_ipcomp.rs` | `Vec<u8>` | `try_reserve`, `iter`, `iter_mut` | O(n) when it grows |
| `acct.rs` | `Box<dyn AcctSink>` | `acct_open` (an `AcctVnode`, N5.3), `close` | O(1) |
| `sched.rs` | `Box::leak` of a `SchedCpu` | | O(1), once per CPU at boot |

## `bsd` (legacy, `sys/sys/queue.rs` and `sys/sys/tree.rs`)

The C macros' orders, kept 1:1 by LZ: `TAILQ_*`, `LIST_*`, `SIMPLEQ_*`, `STAILQ_*`,
`XSIMPLEQ_*` all O(1) except the walks (`*_FOREACH`, O(n)) and the singly linked removals
that must find the predecessor (`SLIST_REMOVE`, `SIMPLEQ_REMOVE`, `STAILQ_REMOVE`: O(n));
`RB_*`/`RBT_*` O(log n) for find, insert, remove, min, max, O(1) amortized a step for
`NEXT`/`PREV` over a walk; `SPLAY_*` O(log n) amortized. Deleted with the crate at N9.

## How to use this table

- The planner writes each walk's cost per step against the C's in the brief; a search per
  step is a named decision, never the default (`.claude/agents/planner.md`).
- The redesigner's step 6 (`.claude/agents/redesigner.md`): a walk the C makes by pointer
  walks with `iter`, `next(e)` or a cursor, O(1) a step; the C's `RBT_FIND` + `RBT_NEXT`
  is the only place a search per step is right.
- A `hal` collection method whose order changes, or a new method, updates its row in the
  same commit.
