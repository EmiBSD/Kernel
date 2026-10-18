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
 * Copyright (c) 2011 Ariane van der Steldt <ariane@openbsd.org>
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
 *
 *
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
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
 *	@(#)vm_map.c    8.3 (Berkeley) 1/12/94
 * from: Id: uvm_map.c,v 1.1.2.27 1998/02/07 01:16:54 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*-
 * Copyright (c) 1991, 1992, 1993
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
 *	@(#)vm_extern.h	8.5 (Berkeley) 5/3/95
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm.h,v 1.1.2.14 1998/02/02 20:07:19 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_fault.h,v 1.1.2.2 1997/12/08 16:07:12 chuck Exp
 */
/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
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
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_fault.c,v 1.1.2.23 1998/02/06 05:29:05 chs Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_map.c`: uvm map operations, and `<uvm/uvm_map.h>`: `struct vm_map` and `struct
//! vm_map_entry`; `struct vmspace` of `<uvm/uvm_extern.h>`, the entry types of `<uvm/uvm.h>`,
//! the map half of `amap_copy` (`uvm_amap.c`), `struct vm_aref` (`<uvm/uvm_anon.h>`) and the
//! map the fault handler resolves addresses in (`uvm_fault.c`'s `struct vm_map` side).
//!
//! Upstream: sys/uvm/uvm_map.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_map.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_extern.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_amap.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_amap.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_anon.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_anon.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_fault.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_fault.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_map.rs@3c62ede68802
//! LZ: sys/uvm/uvm_extern.rs@3c62ede68802
//! LZ: sys/uvm/uvm.rs@3c62ede68802
//! LZ: sys/uvm/uvm_amap.rs@3c62ede68802
//! LZ: sys/uvm/uvm_anon.rs@3c62ede68802
//! LZ: sys/uvm/uvm_fault.rs@3c62ede68802
//!
//! A map ([`VmMap`]) is a range of virtual addresses, the pmap that backs them, and the
//! entries that describe every address of the range ([`VmMapEntry`]): a mapping
//! `[start, end)`, a guard, and the free space `fspace` after it, so the entries tile the map
//! without gaps. The entries sit in an address tree augmented with the largest free space
//! below each node (`fspace_augment`), which the address selectors (`super::addr`) search;
//! the free space of an entry belongs to at most one selector, which may index it in a free
//! tree of its own. A vmspace ([`Vmspace`]) is a process's map with its statistics.
//!
//! **Locks.** A map is locked by its reader-writer lock (`vmmaplk`/`kmmaplk`) or, for an
//! `INTRSAFE` map, by its mutex (`IPL_VM`); [`VmMap::lock`] and [`VmMap::lock_read`] give the
//! guard ([`MapWriteGuard`], [`MapGuard`]) through which the map's state and its entries are
//! reached; dropping it unlocks. A writer may mark the map busy ([`MapWriteGuard::into_busy`],
//! `vm_map_busy`) and unlock it: other writers then wait (`"vmmapbsy"`), readers do not, and
//! the busy thread locks it again without waiting ([`MapBusy`]). Every write lock, try-lock
//! and upgrade bumps `timestamp`, the version `uvmfault_relock` compares. The order is MAPS =>
//! AMAP => UOBJ => ANON => PQ.
//!
//! ## Deviations
//! - `vm_map_lock` and friends are the `*_ln` functions without the `VMMAP_DEBUG` file/line
//!   arguments and `LPRINTF`; the tree checks (`uvm_tree_sanity`, `uvm_tree_size_chk`,
//!   `vmspace_validate`) run in host tests, as `option VMMAP_DEBUG`, which is not configured.
//! - [`uvm_map_inentry`] prints with `printf` (no controlling terminal: `uprintf`); its
//!   format becomes two words (`what`, `reason`).
//! - `uvm_map_splitentry` takes the pager reference under `KERNEL_LOCK`, as the C (M11a).
//! - `uvm_map_fill_vmmap` (`kinfo_vmentry`, sysctl) and the `ddb` printers
//!   (`uvm_map_printit`, `uvm_object_printit`, `uvm_page_printit`) wait for their
//!   subsystems; `PMAP_CHECK_COPYIN`, `SYSVSHM` and `DEADBEEF0` are not configured;
//!   `TRACEPOINT` (dt) is not configured.
//! - `uvm_map_fix_space` reuses a free-only entry that already starts at the address where
//!   the C would allocate a new one (a kernel map whose first entry is free space at
//!   `uvm_maxkaddr`: amd64 reserves nothing at `uvm_km_init`, its image lives outside the
//!   window); the C would collide in the address tree.
//! - `uvm_maxkaddr` starts at the compile-time `VM_MIN_KERNEL_ADDRESS` and `uvm_init` raises
//!   it to the runtime kernel-map base before `uvm_km_init` (amd64 shifts the base above its
//!   direct map).
//! - Out parameters become return values (`Result`, tuples, `Option`): `uvm_map_findspace`
//!   yields the first and last entry and the address, `uvm_map_lookup_entry` the entry only
//!   when it contains the address, `uvm_map_extract` the destination address.
//! - [`uvm_map_deallocate`]'s last reference empties the map but cannot give its block back
//!   nor drop its pmap (`pmap_destroy`, uvm_map.c:3996: the pmap is the map's `[I]` field,
//!   which lives with the block): a map made by [`uvm_map_create`] is a leaked `M_VMMAP`
//!   block (`&'static`), and the only maps that are ever submapped (`kmem_map`, `exec_map`,
//!   `phys_map`, all over the kernel pmap) live for ever, as in the C, where their reference
//!   never reaches 0.
//! - The static entries' refill page is `km_alloc(PAGE_SIZE, &kv_page, &kp_zero,
//!   &kd_nowait)`, the direct map's single page as the C's, zeroed where the C takes
//!   `kp_dirty` (the page provider's contract hands out zeroed memory; the slots are written
//!   before use either way).
//! - `FaultMap::note_maxrss` keeps the C's `KASSERT` that the faulting map is a vmspace's;
//!   whether it is `curproc`'s own is `bsd`'s to know (the `VmProc` bridge answers for the
//!   current thread).
//!
//! ## Redesign
//! - **Ownership.** LZ's entries are pool items handed out as `&'static VmMapEntry` (the
//!   static array, never-freed pages carved into entries, and two pools), linked in a legacy
//!   `RbtHead` address tree, a `TailqHead` dead queue and an `SlistHead` free list through
//!   raw links, and freed with `pool_put` while the code still holds the reference: 22
//!   `unsafe`. Here every entry is a [`PoolBox`] whichever its source: the 1,024 static
//!   entries and the carved pages are a `hal::pool::StaticArena` behind their own `BoxPool`
//!   (`KENTRY_POOL`, the C's `UVM_MAP_STATIC`), and `"vmmpekpl"` and `"vmmpepl"` stay the
//!   C's pools. A box is owned by the map's address tree ([`ByAddr`], a
//!   `hal::owned::OwnedTree`, augmented with `fspace_augment`) or by a dead queue
//!   ([`DeadQueue`], a `hal::owned::OwnedList` over the same link: the C's `dfree` union),
//!   never both; the selectors' free trees and the pivots only count their links in the
//!   entry, so the tree refuses to give an entry back while one still names it. An entry's
//!   pool goes back with its box (`uvm_mapent_free`'s dispatch on `UVM_MAP_STATIC`/`KMEM`);
//!   `uvmexp.kmapent` is counted at the static allocation and at the static entry's drop.
//! - **Entries by key.** A reference to an entry borrows the map's state, so it cannot live
//!   across a change of the tree: functions name entries by their `start`, the address
//!   tree's unique key (`uvm_mapent_addr_insert` panics on a collision, as the C), and find
//!   them again after each insert or removal, and only then (decision 43): between two
//!   changes a walk holds the entry it found (`uvm_map_fix_space`, `uvm_map_mkentry`,
//!   `uvm_mapent_mkfree`, `uvm_unmap_remove`, the free-list update), the free-list functions
//!   take the entry with the state split around it ([`MapState`]'s `free_parts`: the tree
//!   shared, the selectors mutable), `uvm_map_mkentry` and `uvm_mapent_clone` give back the
//!   entry they made, and `uvmspace_fork` and `uvm_map_teardown` follow the links of a tree
//!   no step changes (`RBT_FOREACH`), as the C's pointers do. A clip takes the original out of the tree,
//!   edits the box and puts the parts back, which is what `uvm_map_clip_start` does in the C
//!   (unlink, copy, insert the copy, split); the C's rule that a clipper keeps the pointer it
//!   is given is the key it returns.
//! - **Fields.** Every scalar of an entry is a relaxed atomic (the tree adapters, the busy
//!   walkers and the trap's `sserial` read them without the lock, as the C's plain loads),
//!   written by setters that take the write lock's [`EntryWrite`]; an entry's two handles
//!   (`object`: [`EntryObject`]; the `aref`'s amap and page offset) are one [`Handles`] in a
//!   `hal::rwlock::Guarded` keyed to the map's lock (part 1 of its witness): readers clone
//!   them through any witness of the lock, the writer changes them through its
//!   [`EntryWrite`]. LZ's per-cell busy flags (N3.4e) are gone with the `Cell`s. The map's
//!   own scalars (`flags`, `size`, `timestamp`, `sserial`, `ref_count`, the bounds) are
//!   atomics; `flags`' read-modify-writes are under `flags_lock` as the C's.
//! - **The lock as values.** `vm_map_lock`/`vm_map_lock_read`/`vm_map_lock_try` give a guard;
//!   `vm_map_unlock*` is its drop; `vm_map_upgrade`/`vm_map_downgrade` consume it and give
//!   the other; `vm_map_busy` + `vm_map_unlock` is [`MapWriteGuard::into_busy`], whose
//!   [`MapBusy`] drops as `vm_map_unbusy`. The busy owner is `hal::sleep::curproc_word()` (0
//!   is never an owner) and `nbusy` its waiters, both under `flags_lock`, on whose address the
//!   waiters sleep (`"vmmapbsy"`, `PVM`). The state (`addr`, the selectors, the brk and stack
//!   areas) is a [`MapState`] in a `Guarded` keyed to the lock at `uvm_map_setup`, which
//!   runs where the map stays (a static, a leaked `M_VMMAP` block, or the vmspace's pool
//!   item), so no cell is ever keyed to an address the lock left. An `INTRSAFE` map's witness
//!   is its mutex's guard. The assertions `vm_map_assert_anylock`/`wrlock` and
//!   `UVM_MAP_REQ_WRITE` are the types of the functions' parameters.
//! - **Busy windows.** `uvm_map_pageable_wire`'s second pass and `uvm_map_clean` walk the
//!   map unlocked while it is busy: each step takes the read lock briefly (readers pass a busy
//!   map, the C's walkers read with no lock at all), copies the entry's scalars and clones
//!   its handles, and works with nothing held, as the C does between its steps. Writers stay
//!   out for the whole walk, as in the C.
//! - **The vmspace.** `(struct vmspace *)map` becomes the map's own [`VmspaceVars`] (the
//!   statistics and bounds of `struct vmspace`, atomics), which [`Vmspace`] derefs to; a
//!   [`Vmspace`] is a `PoolArc` item of `"vmsppl"` ([`VmspaceRef`]) with the C's explicit
//!   `vm_refcnt`; its pmap is the map's [`PmapRef`], dropped with it (`pmap_destroy`).
//!   `uvmspace_exec` of a shared vmspace returns the new one for the caller to install
//!   (`pmap_deactivate`, the two slots, `pmap_activate`, `uvmspace_free` of the old one are
//!   `struct proc`'s, `bsd`'s until N4).
//! - **`uvm_map_pageable`'s lock flags.** `UVM_LK_ENTER` is [`uvm_map_pageable_locked`],
//!   which consumes the write guard (the C's straight return on every path: LZ's relock
//!   after a failed wire is gone); `UVM_LK_ENTER | UVM_LK_EXIT` (uvm_map_protect's
//!   `VM_MAP_WIREFUTURE` path) is [`uvm_map_pageable_keep`], which gives the guard back.
//! - **Selectors.** The `uaddr_*` slots are `Option<Selector>` values of the state, created
//!   and destroyed with it ([`super::addr`]); which selector owns an address's free space
//!   (`uvm_map_uaddr`) is computed over a snapshot of their ranges ([`Owners`]) and named by
//!   its [`Slot`], not by a pointer.
//! - `curproc`'s facts (`p_ru`, the inentry report, `lim_cur`) come through the `VmProc`
//!   bridge (`super::proc`); the kernel maps are `super::km`'s slots; randomness is
//!   `hal::random`, the teardown's yield `hal::sleep::sched_pause`.
//! - [`uvm_map_protect`]'s `RLIMIT_DATA` check (LZ's `unported!`) is the C's again, over
//!   `VmProc::lim_cur`.
//! - `amap_copy(map, entry, ..)` (the map half; the kernel's `amap_copy_from` copies the
//!   anons) clips and installs the new reference under the write lock's [`EntryWrite`]; the
//!   fault reaches it through `FaultMapGuard::copy_amap`.
//! - `struct vm_aref` is the `amap`/`pageoff` pair of [`Handles`]; LZ's `EntryRefs` (the
//!   handles cloned for `uvm_map_lock_entry`) are a clone of the entry's [`Handles`].

use core::cmp::Ordering;
use core::ops::Deref;
use core::ptr;
use core::sync::atomic::{
    AtomicI32, AtomicI64, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering as Mem,
};

use core::ptr::NonNull;
use hal::arch::current::param::{PAGE_MASK, PAGE_SHIFT, PAGE_SIZE};
use hal::arch::current::vmparam::{
    BRKSIZ, MAXDSIZ, VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS, VM_MIN_KERNEL_ADDRESS,
};
use hal::intr::{IPL_NONE, IPL_VM, splassert};

use hal::kmbuf::{KmBuf, PageKind, PageMemory};
use hal::malloc::{M_NOWAIT, M_WAITOK, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::owned::{OwnedEntry, OwnedList, OwnedTree, SecondaryEntry};
use hal::pmap::PmapRef;
use hal::pool::{BoxPool, PR_NOWAIT, PR_WAITOK, PoolArc, PoolBox, StaticArena, TypedPool};
use hal::random::{arc4random, arc4random_uniform};
use hal::rwlock::{
    Divisible, Exclusive, Guarded, LockId, RawRwlock, RwMode, RwWitness, Witness, WitnessPart,
};
use hal::sleep::{INFSLP, PVM, cold, curproc_word, sched_pause, wakeup};
use hal::sys::errno::Errno;
use hal::sys::mman::{
    MADV_NORMAL, MADV_RANDOM, MADV_SEQUENTIAL, MAP_INHERIT_COPY, MAP_INHERIT_NONE,
    MAP_INHERIT_SHARE, MAP_INHERIT_ZERO, MCL_CURRENT, MCL_FUTURE, PROT_EXEC, PROT_NONE, PROT_READ,
    PROT_WRITE,
};
use hal::sys::types::Vaddr;
use hal::vm::param::{atop, ptoa, round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;

use super::addr::{
    Selector, uaddr_kbootstrap, uaddr_rnd_create, uaddr_stack_brk_create, uvm_addr_destroy,
    uvm_addr_init, uvm_addr_invoke,
};
use super::amap::{
    AMAP_REFALL, AMAP_SHARED, AmapRef, CopyOutcome, UVM_AMAP_CHUNK, UVM_AMAP_LARGE, amap_alloc,
    amap_copy_from, amap_cow_now, amap_lookup, amap_ref, amap_splitref, amap_unadd, amap_unref,
};
use super::anon::uvm_anfree;
use super::fault::{
    EntryLocks, FaultEntry, FaultMap, FaultMapGuard, uvm_fault_unwire_locked, uvm_fault_wire,
};
use super::km::{
    KD_NOWAIT, KP_ZERO, KV_PAGE, KvMap, is_kernel_map, kernel_map, km_alloc, km_free, map_of,
    mapper, uvm_km_pgremove, uvm_km_pgremove_intrsafe,
};
use super::object::ObjRef;
use super::page::{Owner, PQ_ANON, uvm_pagedeactivate};
use super::pager::{PGO_CLEANIT, PGO_DEACTIVATE, PGO_FREE};
use super::proc::{RLIMIT_DATA, vmproc};
use super::types::{
    PROT_MASK, UVM_FLAG_CONCEAL, UVM_FLAG_COPYONW, UVM_FLAG_FIXED, UVM_FLAG_HOLE, UVM_FLAG_NOFAULT,
    UVM_FLAG_NOMERGE, UVM_FLAG_OVERLAY, UVM_FLAG_QUERY, UVM_FLAG_SIGALTSTACK, UVM_FLAG_STACK,
    UVM_FLAG_TRYLOCK, UVM_FLAG_UNMAP, UVM_FLAG_WC, UVM_UNKNOWN_OFFSET, VmInherit, VmProt, Voff,
    uvm_advice, uvm_inherit, uvm_mapflag, uvm_maxprotection, uvm_protection,
};
use super::vnode::ratecheck;
use crate::assert::{kassert, kdassert, kpanic};

/// `VM_MAP_PAGEABLE`: ro: entries are pageable.
pub const VM_MAP_PAGEABLE: i32 = 0x01;
/// `VM_MAP_INTRSAFE`: ro: interrupt safe map.
pub const VM_MAP_INTRSAFE: i32 = 0x02;
/// `VM_MAP_WIREFUTURE`: rw: wire future mappings.
pub const VM_MAP_WIREFUTURE: i32 = 0x04;
/// `VM_MAP_GUARDPAGES`: rw: add guard pgs to map.
pub const VM_MAP_GUARDPAGES: i32 = 0x20;
/// `VM_MAP_ISVMSPACE`: ro: map is a vmspace.
pub const VM_MAP_ISVMSPACE: i32 = 0x40;
/// `VM_MAP_PINSYSCALL_ONCE`: rw: pinsyscall done.
pub const VM_MAP_PINSYSCALL_ONCE: i32 = 0x100;

/// `MAX_KMAPENT`: number of kernel maps and entries to statically allocate (sufficient to
/// make it to the scheduler).
pub const MAX_KMAPENT: usize = 1024;

/// `UVM_MAP_STATIC`: static map entry.
pub const UVM_MAP_STATIC: u8 = 0x01;
/// `UVM_MAP_KMEM`: from kmem entry pool.
pub const UVM_MAP_KMEM: u8 = 0x02;

/// `UVM_EXTRACT_FIXPROT`: set prot to maxprot as we go.
pub const UVM_EXTRACT_FIXPROT: i32 = 0x8;

/// `VM_MAP_KSIZE_INIT`: the kernel map will initially be this many bytes.
pub const VM_MAP_KSIZE_INIT: usize = 512 * PAGE_SIZE;
/// `VM_MAP_KSIZE_DELTA`: every time that gets cramped, we grow by at least this many bytes.
pub const VM_MAP_KSIZE_DELTA: usize = 256 * PAGE_SIZE;
/// `VM_MAP_KSIZE_ALLOCMUL`: we attempt to grow by this many times the allocation size each
/// time.
pub const VM_MAP_KSIZE_ALLOCMUL: usize = 4;

/// `VMMAP_MIN_ADDR`: auto-allocate address lower bound.
pub const VMMAP_MIN_ADDR: usize = PAGE_SIZE;

/// `M_VMMAP`: the malloc type of the maps `uvm_map_create` makes (`vmstat -m`'s "VM map").
pub const M_VMMAP: i32 = 30;

// vm_map_entry etype bits (`<uvm/uvm.h>`):

/// `UVM_ET_OBJ`: it is a uvm_object.
pub const UVM_ET_OBJ: i32 = 0x0001;
/// `UVM_ET_SUBMAP`: it is a vm_map submap.
pub const UVM_ET_SUBMAP: i32 = 0x0002;
/// `UVM_ET_COPYONWRITE`: copy_on_write.
pub const UVM_ET_COPYONWRITE: i32 = 0x0004;
/// `UVM_ET_NEEDSCOPY`: needs_copy.
pub const UVM_ET_NEEDSCOPY: i32 = 0x0008;
/// `UVM_ET_HOLE`: no backend.
pub const UVM_ET_HOLE: i32 = 0x0010;
/// `UVM_ET_NOFAULT`: don't fault.
pub const UVM_ET_NOFAULT: i32 = 0x0020;
/// `UVM_ET_STACK`: this is a stack.
pub const UVM_ET_STACK: i32 = 0x0040;
/// `UVM_ET_WC`: write combining.
pub const UVM_ET_WC: i32 = 0x0080;
/// `UVM_ET_CONCEAL`: omit from dumps.
pub const UVM_ET_CONCEAL: i32 = 0x0100;
/// `UVM_ET_IMMUTABLE`: entry may not be changed.
pub const UVM_ET_IMMUTABLE: i32 = 0x0400;
/// `UVM_ET_FREEMAPPED`: map entry is on free list (DEBUG).
pub const UVM_ET_FREEMAPPED: i32 = 0x8000;

/// `vmmapbsy`: the wait message of a thread waiting for a busy map.
const VMMAPBSY: &str = "vmmapbsy";

/// `uvm_kmapent_warn_rate`: `(tv_sec, tv_usec)`.
const UVM_KMAPENT_WARN_RATE: (i64, i64) = (10, 0);

/// `VM_PIE_MAX_ADDR` (the default).
pub const VM_PIE_MAX_ADDR: usize = VM_MAXUSER_ADDRESS / 4;
/// `VM_PIE_MIN_ADDR` (the default).
pub const VM_PIE_MIN_ADDR: usize = VM_MIN_ADDRESS;
/// `VM_PIE_MIN_ALIGN` (the default).
pub const VM_PIE_MIN_ALIGN: usize = PAGE_SIZE;

/// `object`: what a map entry points to (the C's `object` union). An object is the entry's
/// counted handle (`uo_refs`, the C's count of references, stays with
/// `pgo_reference`/`pgo_detach`).
#[derive(Clone, Default)]
pub enum EntryObject {
    /// Neither: anonymous memory or a hole.
    #[default]
    None,
    /// `uvm_obj`: uvm object.
    Obj(ObjRef),
    /// `sub_map`: belongs to another map (submaps live for ever).
    SubMap(&'static VmMap),
}

/// An entry's two handles, kept under the map's lock (`object` and `aref`): what read-lock
/// holders read and the write-lock holder changes.
#[derive(Clone, Default)]
pub struct Handles {
    /// `object`: object I point to.
    pub object: EntryObject,
    /// `aref.ar_amap`: the anonymous overlay (the entry's counted handle; `am_ref`, the C's
    /// count of map entries, stays with `amap_ref`/`amap_unref`/`amap_splitref`).
    pub amap: Option<AmapRef>,
    /// `aref.ar_pageoff`: page offset into amap we start.
    pub pageoff: i32,
}

impl Handles {
    /// `object.uvm_obj`, when the entry maps an object.
    pub fn uvm_obj(&self) -> Option<&ObjRef> {
        match &self.object {
            EntryObject::Obj(o) => Some(o),
            _ => None,
        }
    }

    /// `object.sub_map`, when the entry is a submap.
    pub fn sub_map(&self) -> Option<&'static VmMap> {
        match self.object {
            EntryObject::SubMap(m) => Some(m),
            _ => None,
        }
    }

    /// `aref.ar_pageoff = 0; aref.ar_amap = NULL`.
    pub fn clear_aref(&mut self) {
        self.pageoff = 0;
        self.amap = None;
    }
}

/// `struct vm_map_entry`: address map entries consist of start and end addresses, a VM
/// object (or sharing map) and offset into that object, and user-exported inheritance and
/// protection information. Also included is control information for virtual copy
/// operations.
///
/// Every scalar is guarded by the map's lock (`vm_map_lock`), written only by its write
/// lock's holder ([`EntryWrite`]), read by any holder and by the tree adapters; the two
/// handles are [`Handles`] in a cell keyed to the map's lock.
pub struct VmMapEntry {
    /// `daddrs.addr_entry` / `dfree.deadq`: the address tree's link, or a dead queue's.
    link: OwnedEntry<VmMapEntry>,
    /// `dfree.rbtree`: the link of a selector's free-space tree.
    pub(super) free: SecondaryEntry<VmMapEntry>,
    /// `start`: start address (`uvm_map_entry_start_copy`).
    start: AtomicUsize,
    /// `end`: end address.
    end: AtomicUsize,
    /// `guard`: bytes in guard.
    guard: AtomicUsize,
    /// `fspace`: free space.
    fspace: AtomicUsize,
    /// `offset`: offset into object.
    offset: AtomicI64,
    /// `etype`: entry type.
    etype: AtomicI32,
    /// `protection`: protection code.
    protection: AtomicI32,
    /// `max_protection`: maximum protection.
    max_protection: AtomicI32,
    /// `inheritance`: inheritance.
    inheritance: AtomicI32,
    /// `wired_count`: can be paged if == 0.
    wired_count: AtomicI32,
    /// `advice`: madvise advice.
    advice: AtomicI32,
    /// `flags`: `UVM_MAP_STATIC`, `UVM_MAP_KMEM` (`uvm_map_entry_stop_copy`).
    flags: AtomicU8,
    /// `fspace_augment`: max(fspace) in subtree.
    fspace_augment: AtomicUsize,
    /// `object`, `aref`: under the map's lock (part 1 of its witness).
    handles: Guarded<Handles>,
}

/// A scalar setter of [`VmMapEntry`]: the field, written with the write lock's proof.
macro_rules! entry_setter {
    ($(#[$m:meta])* $set:ident, $field:ident: $ty:ty) => {
        $(#[$m])*
        pub fn $set(&self, _w: &EntryWrite<'_>, v: $ty) {
            self.$field.store(v, Mem::Relaxed);
        }
    };
}

impl VmMapEntry {
    /// A zero entry, on no tree, its handles under the lock `key` (what the entry pools hand
    /// out with `PR_ZERO`).
    fn new(key: LockId, flags: u8) -> Self {
        Self {
            link: OwnedEntry::new(),
            free: SecondaryEntry::new(),
            start: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
            guard: AtomicUsize::new(0),
            fspace: AtomicUsize::new(0),
            offset: AtomicI64::new(0),
            etype: AtomicI32::new(0),
            protection: AtomicI32::new(0),
            max_protection: AtomicI32::new(0),
            inheritance: AtomicI32::new(0),
            wired_count: AtomicI32::new(0),
            advice: AtomicI32::new(0),
            flags: AtomicU8::new(flags),
            fspace_augment: AtomicUsize::new(0),
            handles: Guarded::keyed(key, ENTRY_PART, Handles::default()),
        }
    }

    /// `start`.
    pub fn start(&self) -> usize {
        self.start.load(Mem::Relaxed)
    }
    /// `end`.
    pub fn end(&self) -> usize {
        self.end.load(Mem::Relaxed)
    }
    /// `guard`.
    pub fn guard(&self) -> usize {
        self.guard.load(Mem::Relaxed)
    }
    /// `fspace`.
    pub fn fspace(&self) -> usize {
        self.fspace.load(Mem::Relaxed)
    }
    /// `fspace_augment`.
    pub fn fspace_augment(&self) -> usize {
        self.fspace_augment.load(Mem::Relaxed)
    }
    /// `offset`.
    pub fn offset(&self) -> Voff {
        self.offset.load(Mem::Relaxed)
    }
    /// `etype`.
    pub fn etype(&self) -> i32 {
        self.etype.load(Mem::Relaxed)
    }
    /// `protection`.
    pub fn protection(&self) -> VmProt {
        self.protection.load(Mem::Relaxed)
    }
    /// `max_protection`.
    pub fn max_protection(&self) -> VmProt {
        self.max_protection.load(Mem::Relaxed)
    }
    /// `inheritance`.
    pub fn inheritance(&self) -> VmInherit {
        self.inheritance.load(Mem::Relaxed)
    }
    /// `wired_count`.
    pub fn wired_count(&self) -> i32 {
        self.wired_count.load(Mem::Relaxed)
    }
    /// `advice`.
    pub fn advice(&self) -> i32 {
        self.advice.load(Mem::Relaxed)
    }
    /// `flags` (`UVM_MAP_STATIC`, `UVM_MAP_KMEM`).
    pub fn flags(&self) -> u8 {
        self.flags.load(Mem::Relaxed)
    }

    entry_setter!(
        /// `start = v` (an entry out of the address tree: its key).
        set_start, start: usize
    );
    entry_setter!(
        /// `end = v`.
        set_end, end: usize
    );
    entry_setter!(
        /// `guard = v`.
        set_guard, guard: usize
    );
    entry_setter!(
        /// `fspace = v` (the tree's augment is updated by `uvm_mapent_free_insert`).
        set_fspace, fspace: usize
    );
    entry_setter!(
        /// `offset = v`.
        set_offset, offset: Voff
    );
    entry_setter!(
        /// `etype = v`.
        set_etype, etype: i32
    );
    entry_setter!(
        /// `protection = v`.
        set_protection, protection: VmProt
    );
    entry_setter!(
        /// `max_protection = v`.
        set_max_protection, max_protection: VmProt
    );
    entry_setter!(
        /// `inheritance = v`.
        set_inheritance, inheritance: VmInherit
    );
    entry_setter!(
        /// `wired_count = v`.
        set_wired_count, wired_count: i32
    );
    entry_setter!(
        /// `advice = v`.
        set_advice, advice: i32
    );

    /// `etype |= bits`.
    pub fn etype_set(&self, w: &EntryWrite<'_>, bits: i32) {
        self.set_etype(w, self.etype() | bits);
    }

    /// `etype &= ~bits`.
    pub fn etype_clear(&self, w: &EntryWrite<'_>, bits: i32) {
        self.set_etype(w, self.etype() & !bits);
    }

    /// The entry's handles, for the holder of a witness of its map's lock (either mode, the
    /// whole witness or its part 1): clone what must outlive the witness.
    pub fn handles<'a, W: Witness + ?Sized>(&'a self, w: &'a W) -> &'a Handles {
        self.handles.get(w)
    }

    /// The entry's handles, mutably, for the write lock's holder ([`EntryWrite`]): one
    /// entry's at a time.
    pub fn handles_mut<'a>(&'a self, w: &'a mut EntryWrite<'_>) -> &'a mut Handles {
        self.handles.get_mut(&mut w.part)
    }

    /// The handles of an entry no container holds (its box is the caller's).
    fn handles_owned(&mut self) -> &mut Handles {
        self.handles.get_mut_owned()
    }
}

impl Drop for VmMapEntry {
    /// `uvm_mapent_free`'s static case: `atomic_dec_int(&uvmexp.kmapent)` as the static entry
    /// goes back to `uvm.kentry_free` (its box puts it back into the arena).
    fn drop(&mut self) {
        if *self.flags.get_mut() & UVM_MAP_STATIC != 0 {
            UVMEXP.kmapent.fetch_sub(1, Mem::Relaxed);
        }
    }
}

/// The part of the map lock's witness the entries' handles are keyed to (the map's state is
/// part 0).
const ENTRY_PART: u8 = 1;
/// The part of the map lock's witness the map's state is keyed to.
const STATE_PART: u8 = 0;

/// `UVM_ET_ISOBJ(E)`.
pub fn uvm_et_isobj(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_OBJ != 0
}
/// `UVM_ET_ISSUBMAP(E)`.
pub fn uvm_et_issubmap(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_SUBMAP != 0
}
/// `UVM_ET_ISCOPYONWRITE(E)`.
pub fn uvm_et_iscopyonwrite(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_COPYONWRITE != 0
}
/// `UVM_ET_ISNEEDSCOPY(E)`.
pub fn uvm_et_isneedscopy(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_NEEDSCOPY != 0
}
/// `UVM_ET_ISHOLE(E)`.
pub fn uvm_et_ishole(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_HOLE != 0
}
/// `UVM_ET_ISNOFAULT(E)`.
pub fn uvm_et_isnofault(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_NOFAULT != 0
}
/// `UVM_ET_ISSTACK(E)`.
pub fn uvm_et_isstack(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_STACK != 0
}
/// `UVM_ET_ISWC(E)`.
pub fn uvm_et_iswc(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_WC != 0
}
/// `UVM_ET_ISCONCEAL(E)`.
pub fn uvm_et_isconceal(e: &VmMapEntry) -> bool {
    e.etype() & UVM_ET_CONCEAL != 0
}

/// `VM_MAPENT_ISWIRED(entry)`.
pub fn vm_mapent_iswired(entry: &VmMapEntry) -> bool {
    entry.wired_count() != 0
}

/// `VMMAP_FREE_START(entry)`: where the free space after the entry starts.
pub fn vmmap_free_start(entry: &VmMapEntry) -> usize {
    entry.end() + entry.guard()
}

/// `VMMAP_FREE_END(entry)`: where the free space after the entry ends.
pub fn vmmap_free_end(entry: &VmMapEntry) -> usize {
    entry.end() + entry.guard() + entry.fspace()
}

/// `uvm_mapentry_addrcmp`: tree describing entries by address.
///
/// Addresses are unique. Entries with `start == end` may only exist if they are the first
/// entry (sorted by address) within a free-memory tree.
pub fn uvm_mapentry_addrcmp(e1: &VmMapEntry, e2: &VmMapEntry) -> Ordering {
    e1.start().cmp(&e2.start())
}

/// `uvm_map_addr_augment`'s step for one node: `fspace_augment` = the largest of the entry's
/// free space and its children's augments (`uvm_map_addr_augment_get`), stored; whether it
/// changed (the tree walks up while it does).
fn uvm_map_addr_augment(
    e: &VmMapEntry,
    left: Option<&VmMapEntry>,
    right: Option<&VmMapEntry>,
) -> bool {
    let mut augment = e.fspace();
    if let Some(l) = left {
        augment = augment.max(l.fspace_augment());
    }
    if let Some(r) = right {
        augment = augment.max(r.fspace_augment());
    }
    if e.fspace_augment() == augment {
        return false;
    }
    e.fspace_augment.store(augment, Mem::Relaxed);
    true
}

hal::owned_adapter!(
    /// `uvm_map_addr`: the entry tree of a map, by address (`daddrs.addr_entry`), augmented
    /// with `fspace_augment` (`RBT_GENERATE_AUGMENT`); also the dead queue's link.
    pub ByAddr: VmMapEntry, link, uvm_mapentry_addrcmp, augment = uvm_map_addr_augment
);

/// `struct uvm_map_deadq`: dead entry queue, carried out from under the map's lock and
/// emptied by [`uvm_unmap_detach`].
pub type DeadQueue = OwnedList<ByAddr>;

/// The statistics and boundaries of `struct vmspace` (`vm_startcopy` to its end), kept in
/// the vmspace's map (the C's `(struct vmspace *)map`; [`Vmspace`] derefs to them).
///
/// Locks: `vm_rssize`, `vm_swrss`, `vm_tsize`, `vm_dsize` and `vm_dused` have none in the C
/// (`vm_dused` is changed under the map's write lock and read by the `NOLOCK` `sys_mmap`
/// without it: the C's race, kept); `vm_ssize` \[v\]; the addresses \[I\] (exec sets them).
pub struct VmspaceVars {
    /// `vm_rssize`: current resident set size in pages.
    pub vm_rssize: AtomicI32,
    /// `vm_swrss`: resident set size before last swap.
    pub vm_swrss: AtomicI32,
    /// `vm_tsize`: text size (pages) XXX.
    pub vm_tsize: AtomicI32,
    /// `vm_dsize`: data size (pages) XXX.
    pub vm_dsize: AtomicI32,
    /// `vm_dused`: data segment length (pages) XXX.
    pub vm_dused: AtomicI32,
    /// \[v\] `vm_ssize`: stack size (pages).
    pub vm_ssize: AtomicI32,
    /// \[I\] `vm_taddr`: user virtual address of text.
    pub vm_taddr: AtomicUsize,
    /// \[I\] `vm_daddr`: user virtual address of data.
    pub vm_daddr: AtomicUsize,
    /// \[I\] `vm_maxsaddr`: user VA at max stack growth.
    pub vm_maxsaddr: AtomicUsize,
    /// \[I\] `vm_minsaddr`: user VA at top of stack.
    pub vm_minsaddr: AtomicUsize,
}

impl VmspaceVars {
    /// All zero (a fresh vmspace).
    pub const fn new() -> Self {
        Self {
            vm_rssize: AtomicI32::new(0),
            vm_swrss: AtomicI32::new(0),
            vm_tsize: AtomicI32::new(0),
            vm_dsize: AtomicI32::new(0),
            vm_dused: AtomicI32::new(0),
            vm_ssize: AtomicI32::new(0),
            vm_taddr: AtomicUsize::new(0),
            vm_daddr: AtomicUsize::new(0),
            vm_maxsaddr: AtomicUsize::new(0),
            vm_minsaddr: AtomicUsize::new(0),
        }
    }

    /// `memcpy(&vm2->vm_startcopy, &vm1->vm_startcopy, ...)`: the statistics and boundaries
    /// of `other` (fork).
    pub fn copy_startcopy_from(&self, other: &VmspaceVars) {
        let i = |a: &AtomicI32, b: &AtomicI32| a.store(b.load(Mem::Relaxed), Mem::Relaxed);
        let u = |a: &AtomicUsize, b: &AtomicUsize| a.store(b.load(Mem::Relaxed), Mem::Relaxed);
        i(&self.vm_rssize, &other.vm_rssize);
        i(&self.vm_swrss, &other.vm_swrss);
        i(&self.vm_tsize, &other.vm_tsize);
        i(&self.vm_dsize, &other.vm_dsize);
        i(&self.vm_dused, &other.vm_dused);
        i(&self.vm_ssize, &other.vm_ssize);
        u(&self.vm_taddr, &other.vm_taddr);
        u(&self.vm_daddr, &other.vm_daddr);
        u(&self.vm_maxsaddr, &other.vm_maxsaddr);
        u(&self.vm_minsaddr, &other.vm_minsaddr);
    }

    /// `memset(&vm->vm_startcopy, 0, ...)`: nukes the statistics and boundaries (exec).
    pub fn clear_startcopy(&self) {
        self.copy_startcopy_from(&VmspaceVars::new());
    }

    /// `vm_dused += d` (`d` pages, may be negative).
    fn dused_add(&self, d: isize) {
        self.vm_dused.fetch_add(d as i32, Mem::Relaxed);
    }
}

impl Default for VmspaceVars {
    fn default() -> Self {
        Self::new()
    }
}

/// `busy` and `nbusy` of `struct vm_map`, under `flags_lock`.
struct Busy {
    /// `busy`: the thread holding the map busy (`curproc_word()`), 0 for none.
    owner: usize,
    /// `nbusy`: waiters for busy.
    nbusy: u32,
}

/// `uvm_map_set_uaddr`'s `which`, by the name the callers use.
pub type UaddrSlot = Slot;

/// One of the selector slots of a map (the C's `struct uvm_addr_state **which`, and the
/// identity `uvm_map_uaddr` answers with).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// `map->uaddr_exe`.
    Exe,
    /// `map->uaddr_any[i]`.
    Any(usize),
    /// `map->uaddr_brk_stack`.
    BrkStack,
}

/// The state of a map its lock guards (\[v\]): the entries, the address selectors and the
/// brk/stack areas.
pub struct MapState {
    /// \[v\] `addr`: entry tree, by addr.
    pub addr: OwnedTree<ByAddr>,
    /// \[v\] `b_start`: start for brk() alloc.
    pub b_start: usize,
    /// \[v\] `b_end`: end for brk() alloc.
    pub b_end: usize,
    /// \[v\] `s_start`: start for stack alloc.
    pub s_start: usize,
    /// \[v\] `s_end`: end for stack alloc.
    pub s_end: usize,
    /// `uaddr_exe`: executable selector.
    ///
    /// Special address selectors. The uaddr_exe mapping is used if: it is set and the
    /// protection of the mapping contains `PROT_EXEC`. If uaddr_exe is not used, the other
    /// mappings are checked in order of appearance. If uaddr_any is NULL, the kernel will
    /// panic.
    pub uaddr_exe: Option<Selector>,
    /// `uaddr_any`: more selectors.
    pub uaddr_any: [Option<Selector>; 4],
    /// `uaddr_brk_stack`: brk/stack selector. The uaddr_brk_stack selector will select
    /// addresses that are in the brk/stack area of the map.
    pub uaddr_brk_stack: Option<Selector>,
}

impl MapState {
    /// An empty map's state (`uvm_map_setup` before its selectors).
    fn new() -> Self {
        Self {
            addr: OwnedTree::new(),
            b_start: 0,
            b_end: 0,
            s_start: 0,
            s_end: 0,
            uaddr_exe: None,
            uaddr_any: [None, None, None, None],
            uaddr_brk_stack: None,
        }
    }

    /// The selector in `slot`.
    pub fn selector(&self, slot: Slot) -> Option<&Selector> {
        match slot {
            Slot::Exe => self.uaddr_exe.as_ref(),
            Slot::Any(i) => self.uaddr_any[i].as_ref(),
            Slot::BrkStack => self.uaddr_brk_stack.as_ref(),
        }
    }

    /// The slot itself (`uvm_map_set_uaddr`'s `which`).
    fn slot_mut(&mut self, slot: Slot) -> &mut Option<Selector> {
        match slot {
            Slot::Exe => &mut self.uaddr_exe,
            Slot::Any(i) => &mut self.uaddr_any[i],
            Slot::BrkStack => &mut self.uaddr_brk_stack,
        }
    }

    /// The address tree, shared, and the selector in `slot`, mutably (the two fields apart).
    pub(super) fn tree_and_selector(
        &mut self,
        slot: Slot,
    ) -> (&OwnedTree<ByAddr>, Option<&mut Selector>) {
        let sel = match slot {
            Slot::Exe => self.uaddr_exe.as_mut(),
            Slot::Any(i) => self.uaddr_any[i].as_mut(),
            Slot::BrkStack => self.uaddr_brk_stack.as_mut(),
        };
        (&self.addr, sel)
    }

    /// The address tree, shared, with every selector slot, mutably (the fields apart), and
    /// `owners` (this state's [`MapState::owners`]): what the free-list functions work on
    /// while the caller holds entries of the tree by reference.
    fn free_parts(&mut self, owners: Owners) -> FreeParts<'_> {
        FreeParts {
            tree: &self.addr,
            owners,
            exe: &mut self.uaddr_exe,
            any: &mut self.uaddr_any,
            brk_stack: &mut self.uaddr_brk_stack,
        }
    }

    /// Which selector owns which free space now ([`Owners`]).
    pub fn owners(&self, map: &VmMap) -> Owners {
        let range = |s: &Option<Selector>| s.as_ref().map(Selector::range);
        let mut any = [None; 4];
        for (i, s) in self.uaddr_any.iter().enumerate() {
            any[i] = s.as_ref().map(|s| {
                let (lo, hi) = s.range();
                (lo, hi, s.has_free_ops())
            });
        }
        Owners {
            vmspace: map.is_vmspace(),
            exe: range(&self.uaddr_exe),
            brk_stack: range(&self.uaddr_brk_stack),
            any,
            b: (self.b_start, self.b_end),
            s: (self.s_start, self.s_end),
        }
    }
}

/// The selectors' ranges and the brk/stack areas of a map, copied out of its state: what
/// decides which selector owns the free space at an address (`uvm_map_uaddr`) and where free
/// space must be cut (`uvm_map_boundary`), while the selectors themselves are borrowed.
#[derive(Clone, Copy)]
pub struct Owners {
    /// `map->flags & VM_MAP_ISVMSPACE`.
    vmspace: bool,
    /// `uaddr_exe`'s range.
    exe: Option<(usize, usize)>,
    /// `uaddr_brk_stack`'s range.
    brk_stack: Option<(usize, usize)>,
    /// `uaddr_any[i]`'s range, and whether it has free operations.
    any: [Option<(usize, usize, bool)>; 4],
    /// `b_start`, `b_end`.
    b: (usize, usize),
    /// `s_start`, `s_end`.
    s: (usize, usize),
}

impl Owners {
    /// `uvm_map_uaddr`: choose free list based on address at start of free space.
    ///
    /// The selector returned contains addr and is the first of: uaddr_exe, uaddr_brk_stack,
    /// uaddr_any.
    pub fn uaddr(&self, addr: usize) -> Option<Slot> {
        // Special case the first page, to prevent mmap from returning 0.
        if addr < VMMAP_MIN_ADDR {
            return None;
        }

        // Upper bound for kernel maps at uvm_maxkaddr.
        if !self.vmspace && addr >= uvm_maxkaddr() {
            return None;
        }

        // Is the address inside the exe-only map?
        if let Some((lo, hi)) = self.exe
            && addr >= lo
            && addr < hi
        {
            return Some(Slot::Exe);
        }

        // Check if the space falls inside brk/stack area.
        if (addr >= self.b.0 && addr < self.b.1) || (addr >= self.s.0 && addr < self.s.1) {
            return match self.brk_stack {
                Some((lo, hi)) if addr >= lo && addr < hi => Some(Slot::BrkStack),
                _ => None,
            };
        }

        // Check the other selectors.
        //
        // These selectors are only marked as the owner, if they have insert functions.
        for (i, any) in self.any.iter().enumerate() {
            if let Some((lo, hi, true)) = *any
                && addr >= lo
                && addr < hi
            {
                return Some(Slot::Any(i));
            }
        }

        None
    }

    /// `uvm_map_uaddr_e`: the owner of the free space after `entry`.
    pub fn uaddr_e(&self, entry: &VmMapEntry) -> Option<Slot> {
        self.uaddr(vmmap_free_start(entry))
    }

    /// `uvm_map_boundary`: returns the first free-memory boundary that is crossed by
    /// `[min-max]`.
    pub fn boundary(&self, min: usize, max: usize) -> usize {
        // Never return first page.
        let mut max = uvm_map_boundfix(min, max, VMMAP_MIN_ADDR);

        // Treat the maxkaddr special, if the map is a kernel_map.
        if !self.vmspace {
            max = uvm_map_boundfix(min, max, uvm_maxkaddr());
        }

        // Check for exe-only boundaries.
        if let Some((lo, hi)) = self.exe {
            max = uvm_map_boundfix(min, max, lo);
            max = uvm_map_boundfix(min, max, hi);
        }

        // Check for exe-only boundaries.
        if let Some((lo, hi)) = self.brk_stack {
            max = uvm_map_boundfix(min, max, lo);
            max = uvm_map_boundfix(min, max, hi);
        }

        // Check other boundaries.
        for (lo, hi, _) in self.any.iter().flatten() {
            max = uvm_map_boundfix(min, max, *lo);
            max = uvm_map_boundfix(min, max, *hi);
        }

        // Boundaries at stack and brk() area.
        max = uvm_map_boundfix(min, max, self.s.0);
        max = uvm_map_boundfix(min, max, self.s.1);
        max = uvm_map_boundfix(min, max, self.b.0);
        max = uvm_map_boundfix(min, max, self.b.1);

        max
    }
}

/// A map's state split for the free-list functions ([`MapState::free_parts`]): the address
/// tree shared (`tree` is a plain shared reference, so an entry found through it stays
/// borrowed while a selector is changed), the selector slots mutable, and the owners.
struct FreeParts<'s> {
    /// `map->addr`.
    tree: &'s OwnedTree<ByAddr>,
    /// Which selector owns which free space ([`MapState::owners`]).
    owners: Owners,
    /// `map->uaddr_exe`.
    exe: &'s mut Option<Selector>,
    /// `map->uaddr_any`.
    any: &'s mut [Option<Selector>; 4],
    /// `map->uaddr_brk_stack`.
    brk_stack: &'s mut Option<Selector>,
}

impl FreeParts<'_> {
    /// The selector in `slot`, mutably.
    fn selector_mut(&mut self, slot: Slot) -> Option<&mut Selector> {
        match slot {
            Slot::Exe => self.exe.as_mut(),
            Slot::Any(i) => self.any[i].as_mut(),
            Slot::BrkStack => self.brk_stack.as_mut(),
        }
    }
}

/// `struct vm_map`: a virtual address space, the entries that describe it and the pmap that
/// backs it.
///
/// Locks used to protect struct members: `a` atomic operations, `I` immutable after creation
/// or exec(2), `v` `vm_map_lock` (this map `lock` or `mtx`), `f` flags_lock.
pub struct VmMap {
    /// \[I\] `pmap`: physical map (`uvm_map_setup` sets it; its drop is `pmap_destroy`).
    pmap: OnceStatic<PmapRef>,
    /// \[v\] `sserial`: # stack changes (read without the lock by the trap's inentry check).
    sserial: AtomicU64,
    /// `size`: virtual size.
    size: AtomicUsize,
    /// \[a\] `ref_count`: reference count.
    ref_count: AtomicI32,
    /// \[f\] `flags` (the static ones are read without the lock).
    flags: AtomicI32,
    /// `timestamp`: version number.
    timestamp: AtomicU32,
    /// \[I\] `min_offset`: first address in map.
    min_offset: AtomicUsize,
    /// \[I\] `max_offset`: last address in map.
    max_offset: AtomicUsize,
    /// The vmspace's statistics and bounds, for a vmspace's map (`VM_MAP_ISVMSPACE`).
    vs: VmspaceVars,
    /// `lock`: non-intrsafe lock (`"vmmaplk"` for a vmspace, `"kmmaplk"` otherwise).
    lock: RawRwlock,
    /// `mtx`: intrsafe lock.
    mtx: Mutexed<()>,
    /// `flags_lock`: flags lock (and `busy`, `nbusy`); its address is the busy waiters'
    /// channel.
    flags_lock: Mutexed<Busy>,
    /// \[v\] The entries, the selectors and the brk/stack areas, keyed to `lock` (or `mtx`)
    /// by `uvm_map_setup`, where the map stays.
    state: OnceStatic<Guarded<MapState>>,
}

impl VmMap {
    /// A map of `[min, max)` before `uvm_map_setup` (a kernel map's store: `"kmmaplk"`).
    pub const fn new(min: usize, max: usize) -> Self {
        Self::with_name(min, max, "kmmaplk")
    }

    /// A map whose lock sleeps say `name` (`uvm_map_setup`'s `rw_init`: `"vmmaplk"` for a
    /// vmspace, `"kmmaplk"` otherwise).
    const fn with_name(min: usize, max: usize, name: &'static str) -> Self {
        Self {
            pmap: OnceStatic::new(),
            sserial: AtomicU64::new(0),
            size: AtomicUsize::new(0),
            ref_count: AtomicI32::new(0),
            flags: AtomicI32::new(0),
            timestamp: AtomicU32::new(0),
            min_offset: AtomicUsize::new(min),
            max_offset: AtomicUsize::new(max),
            vs: VmspaceVars::new(),
            lock: RawRwlock::new(name),
            mtx: Mutexed::new(IPL_VM, ()),
            flags_lock: Mutexed::new(IPL_VM, Busy { owner: 0, nbusy: 0 }),
            state: OnceStatic::new(),
        }
    }

    /// `vm_map_min(map)`.
    pub fn min_offset(&self) -> usize {
        self.min_offset.load(Mem::Relaxed)
    }

    /// `vm_map_max(map)`.
    pub fn max_offset(&self) -> usize {
        self.max_offset.load(Mem::Relaxed)
    }

    /// `map->flags`.
    pub fn flags(&self) -> i32 {
        self.flags.load(Mem::Relaxed)
    }

    /// `map->flags & VM_MAP_INTRSAFE`.
    pub fn is_intrsafe(&self) -> bool {
        self.flags() & VM_MAP_INTRSAFE != 0
    }

    /// `map->flags & VM_MAP_ISVMSPACE`.
    pub fn is_vmspace(&self) -> bool {
        self.flags() & VM_MAP_ISVMSPACE != 0
    }

    /// `map->flags & VM_MAP_PAGEABLE`.
    pub fn is_pageable(&self) -> bool {
        self.flags() & VM_MAP_PAGEABLE != 0
    }

    /// `map->size`: the mapped bytes.
    pub fn size(&self) -> usize {
        self.size.load(Mem::Relaxed)
    }

    /// `map->sserial`: the stack changes (read with no lock: the inentry cache).
    pub fn sserial(&self) -> u64 {
        self.sserial.load(Mem::Relaxed)
    }

    /// `map->timestamp`.
    pub fn timestamp(&self) -> u32 {
        self.timestamp.load(Mem::Relaxed)
    }

    /// `map->ref_count`.
    pub fn ref_count(&self) -> i32 {
        self.ref_count.load(Mem::Relaxed)
    }

    /// `vm_map_pmap(map)`: the physical map (`uvm_map_setup` set it).
    pub fn pmap(&self) -> &PmapRef {
        match self.pmap.get() {
            Some(pm) => pm,
            None => kpanic(format_args!("vm_map_pmap: a map without a pmap")),
        }
    }

    /// `(struct vmspace *)map`'s statistics and bounds (meaningful for a vmspace's map,
    /// `VM_MAP_ISVMSPACE`; zero in any other).
    pub fn vmspace_vars(&self) -> &VmspaceVars {
        &self.vs
    }

    /// The vmspace's statistics of a map that must be one.
    fn vm(&self) -> &VmspaceVars {
        kassert!(self.is_vmspace());
        &self.vs
    }

    /// `vm_map_modflags(map, set, clear)`: changes the map's flags under `flags_lock`; the
    /// flags before (a test-and-set under the same lock: `sys_pinsyscalls`'
    /// `VM_MAP_PINSYSCALL_ONCE`).
    pub fn modflags(&self, set: i32, clear: i32) -> i32 {
        let _f = self.flags_lock.lock();
        let flags = self.flags.load(Mem::Relaxed);
        self.flags.store((flags | set) & !clear, Mem::Relaxed);
        flags
    }

    /// The identity of the lock that guards the state: the mutex for an `INTRSAFE` map, the
    /// rwlock otherwise.
    fn lock_id(&self) -> LockId {
        if self.is_intrsafe() {
            self.mtx.id()
        } else {
            self.lock.id()
        }
    }

    /// The state's cell; a map not yet set up is `panic(9)`.
    fn state_cell(&self) -> &Guarded<MapState> {
        match self.state.get() {
            Some(s) => s,
            None => kpanic(format_args!("vm_map: map {:#x} not set up", self.addr())),
        }
    }

    /// The map's address, as the C's `%p` prints it.
    fn addr(&self) -> usize {
        ptr::from_ref(self).addr()
    }

    /// `map->busy != NULL && map->busy != curproc`, under `flags_lock`.
    fn busy_other(f: &Busy, me: usize) -> bool {
        f.owner != 0 && f.owner != me
    }

    /// `timestamp++`.
    fn bump(&self) {
        self.timestamp.fetch_add(1, Mem::Relaxed);
    }

    /// `vm_map_lock(map)` (`vm_map_lock_ln`): takes the write lock, waiting out a thread that
    /// holds the map busy (a busy holder's own lock does not wait, nor bump the version).
    pub fn lock(&self) -> MapWriteGuard<'_> {
        let (w, mine) = if self.is_intrsafe() {
            (AnyWitness::Intr(self.mtx.lock()), false)
        } else {
            let me = curproc_word();
            let mut f = self.flags_lock.lock();
            let w = loop {
                while Self::busy_other(&f, me) {
                    f.nbusy += 1;
                    let _ = f.sleep_nsec(ptr::from_ref(&self.flags_lock), PVM, VMMAPBSY, INFSLP);
                    f.nbusy -= 1;
                }
                drop(f);

                let w = self.lock.write();

                // to be sure, to be sure
                f = self.flags_lock.lock();
                if !Self::busy_other(&f, me) {
                    // we won
                    break w;
                }
                // go around again
                drop(w);
            };
            let mine = f.owner != 0 && f.owner == me;
            drop(f);
            (AnyWitness::Rw(RwWitness::Write(w)), mine)
        };

        if !mine {
            self.bump();
        }
        MapWriteGuard(MapGuard {
            map: self,
            w,
            mode: RwMode::Write,
        })
    }

    /// `vm_map_lock_try(map)` (`vm_map_lock_try_ln`): takes the write lock without sleeping;
    /// `None` when the map is busy for another thread or already locked.
    pub fn lock_try(&self) -> Option<MapWriteGuard<'_>> {
        let w = if self.is_intrsafe() {
            AnyWitness::Intr(self.mtx.try_lock()?)
        } else {
            let me = curproc_word();
            if Self::busy_other(&self.flags_lock.lock(), me) {
                return None;
            }

            let w = self.lock.try_write()?;

            // to be sure, to be sure
            if Self::busy_other(&self.flags_lock.lock(), me) {
                drop(w);
                return None;
            }
            AnyWitness::Rw(RwWitness::Write(w))
        };

        self.bump();
        Some(MapWriteGuard(MapGuard {
            map: self,
            w,
            mode: RwMode::Write,
        }))
    }

    /// `vm_map_lock_read(map)` (`vm_map_lock_read_ln`): readers pass a busy map.
    pub fn lock_read(&self) -> MapGuard<'_> {
        let w = if self.is_intrsafe() {
            AnyWitness::Intr(self.mtx.lock())
        } else {
            AnyWitness::Rw(RwWitness::Read(self.lock.read()))
        };
        MapGuard {
            map: self,
            w,
            mode: RwMode::Read,
        }
    }

    /// `uvmfault_relock`'s map half: `vm_map_lock_read(map)`, kept when the map's version is
    /// still `version`; otherwise unlocked and `None`.
    pub fn relock(&self, version: u32) -> Option<MapGuard<'_>> {
        let g = self.lock_read();
        (g.version() == version).then_some(g)
    }
}

/// The witness a map guard holds: its rwlock's, in either mode, or an `INTRSAFE` map's mutex
/// guard.
enum AnyWitness<'a> {
    /// `rw_enter(&map->lock, ..)`.
    Rw(RwWitness<'a>),
    /// `mtx_enter(&map->mtx)`.
    Intr(MutexedGuard<'a, ()>),
}

/// A locked map (`vm_map_lock_read`, or `vm_map_lock` when [`MapGuard::mode`] is
/// `RwMode::Write`): the map's state and entries are read through it; dropping it is
/// `vm_map_unlock_read`/`vm_map_unlock`.
#[must_use = "dropping the guard at once unlocks the map"]
pub struct MapGuard<'a> {
    map: &'a VmMap,
    w: AnyWitness<'a>,
    mode: RwMode,
}

impl<'a> MapGuard<'a> {
    /// The map.
    pub fn map(&self) -> &'a VmMap {
        self.map
    }

    /// How the map is locked.
    pub fn mode(&self) -> RwMode {
        self.mode
    }

    /// `map->timestamp`.
    pub fn version(&self) -> u32 {
        self.map.timestamp()
    }

    /// The witness of the map's lock this guard holds.
    pub fn witness(&self) -> &dyn Witness {
        match &self.w {
            AnyWitness::Rw(w) => w,
            AnyWitness::Intr(g) => g,
        }
    }

    /// The map's state.
    pub fn state(&self) -> &MapState {
        self.map.state_cell().get(self.witness())
    }

    /// `uvm_map_lookup_entry(map, va, &entry)`: the entry holding `va` (not a hole).
    pub fn lookup(&self, va: usize) -> Option<&VmMapEntry> {
        uvm_map_lookup_entry(&self.state().addr, va)
    }

    /// `vm_map_upgrade(map)` (`vm_map_upgrade_ln`) of a read-locked map: the write lock
    /// without sleeping (the version bumped), or this guard back, still read-locked, when
    /// another thread is in the way (other readers, or the map busy for another).
    pub fn upgrade(self) -> Result<MapWriteGuard<'a>, MapGuard<'a>> {
        let map = self.map;
        let w = match self.w {
            // MUTEX_ASSERT_LOCKED(&map->mtx): the guard is the proof.
            AnyWitness::Intr(g) => AnyWitness::Intr(g),
            AnyWitness::Rw(rw) => {
                if VmMap::busy_other(&map.flags_lock.lock(), curproc_word()) {
                    return Err(MapGuard {
                        map,
                        w: AnyWitness::Rw(rw),
                        mode: self.mode,
                    });
                }
                match rw.try_upgrade() {
                    Ok(w) => AnyWitness::Rw(RwWitness::Write(w)),
                    Err(rw) => {
                        return Err(MapGuard {
                            map,
                            w: AnyWitness::Rw(rw),
                            mode: self.mode,
                        });
                    }
                }
            }
        };
        map.bump();
        Ok(MapWriteGuard(MapGuard {
            map,
            w,
            mode: RwMode::Write,
        }))
    }

    /// The write guard this is, when the map is write-locked.
    pub fn into_write(self) -> Result<MapWriteGuard<'a>, MapGuard<'a>> {
        if self.mode == RwMode::Write {
            Ok(MapWriteGuard(self))
        } else {
            Err(self)
        }
    }

    /// The witness divided: part 0 opens the state, part 1 the entries' handles; a guard not
    /// write-locked is `panic(9)`.
    fn split_write(&mut self) -> MapSplit<'_> {
        if self.mode != RwMode::Write {
            kpanic(format_args!("vm_map: a read lock changed the map"));
        }
        let map = self.map;
        let [state, entries] = match &mut self.w {
            AnyWitness::Rw(rw) => match rw.as_write() {
                Some(w) => w.split::<2>(),
                None => kpanic(format_args!("vm_map: a read lock changed the map")),
            },
            AnyWitness::Intr(g) => g.split::<2>(),
        };
        MapSplit {
            map,
            state,
            entries: EntryWrite { part: entries },
        }
    }
}

/// A write-locked map (`vm_map_lock`, `vm_map_lock_try`, a successful `vm_map_upgrade`):
/// what changes the map's state and its entries; dropping it is `vm_map_unlock`.
#[must_use = "dropping the guard at once unlocks the map"]
pub struct MapWriteGuard<'a>(MapGuard<'a>);

impl<'a> MapWriteGuard<'a> {
    /// The map's state, mutably (for what changes no entry's handles; see
    /// [`MapWriteGuard::split`]).
    pub fn state_mut(&mut self) -> &mut MapState {
        let map = self.0.map;
        let w: &mut dyn Exclusive = match &mut self.0.w {
            AnyWitness::Rw(rw) => match rw.as_write() {
                Some(w) => w,
                None => kpanic(format_args!("vm_map: a read lock changed the map")),
            },
            AnyWitness::Intr(g) => g,
        };
        map.state_cell().get_mut(w)
    }

    /// The state and the entries' write proof at once ([`MapSplit::parts`]): an entry
    /// borrowed from the state has its scalars and handles changed through the
    /// [`EntryWrite`].
    pub fn split(&mut self) -> MapSplit<'_> {
        self.0.split_write()
    }

    /// `vm_map_downgrade(map)` (`vm_map_downgrade_ln`): the write lock becomes a read lock,
    /// without a window (an `INTRSAFE` map keeps its mutex).
    pub fn downgrade(self) -> MapGuard<'a> {
        let MapGuard { map, w, .. } = self.0;
        let w = match w {
            AnyWitness::Rw(RwWitness::Write(w)) => AnyWitness::Rw(RwWitness::Read(w.downgrade())),
            other => other,
        };
        MapGuard {
            map,
            w,
            mode: RwMode::Read,
        }
    }

    /// `vm_map_busy(map)` then `vm_map_unlock(map)`: the map stays busy for this thread
    /// (other writers wait, readers pass) while unlocked; the [`MapBusy`]'s drop is
    /// `vm_map_unbusy`.
    pub fn into_busy(self) -> MapBusy<'a> {
        let map = self.0.map;
        kassert!(!map.is_intrsafe());
        {
            let mut f = map.flags_lock.lock();
            kassert!(f.owner == 0);
            f.owner = curproc_word();
        }
        drop(self);
        MapBusy { map }
    }

    /// The guard as the fault sees it (a write-locked [`MapGuard`]).
    pub fn into_guard(self) -> MapGuard<'a> {
        self.0
    }
}

impl<'a> Deref for MapWriteGuard<'a> {
    type Target = MapGuard<'a>;

    fn deref(&self) -> &MapGuard<'a> {
        &self.0
    }
}

/// A write guard's witness divided in two: the map's state (part 0) and the entries'
/// handles and scalars ([`EntryWrite`], part 1), reachable at once.
pub struct MapSplit<'g> {
    map: &'g VmMap,
    state: WitnessPart<'g>,
    entries: EntryWrite<'g>,
}

impl<'g> MapSplit<'g> {
    /// The state, mutably, and the entries' write proof.
    pub fn parts(&mut self) -> (&mut MapState, &mut EntryWrite<'g>) {
        (
            self.map.state_cell().get_mut(&mut self.state),
            &mut self.entries,
        )
    }
}

/// The write lock's proof for an entry's setters and handles (part 1 of the witness): one
/// exists per split write guard, so the scalars change and one entry's handles open mutably
/// only under the map's write lock.
pub struct EntryWrite<'w> {
    part: WitnessPart<'w>,
}

impl EntryWrite<'_> {
    /// The witness that reads the entries' handles.
    pub fn witness(&self) -> &WitnessPart<'_> {
        &self.part
    }
}

/// A busy map (`vm_map_busy`), unlocked: other writers wait for it, readers pass, and the
/// busy thread locks it without waiting (and without bumping the version). Dropped is
/// `vm_map_unbusy`.
#[must_use = "dropping the busy mark at once unbusies the map"]
pub struct MapBusy<'a> {
    map: &'a VmMap,
}

impl<'a> MapBusy<'a> {
    /// The map.
    pub fn map(&self) -> &'a VmMap {
        self.map
    }

    /// `vm_map_lock(map)` by the busy thread: no wait on itself, no version bump.
    pub fn lock(&self) -> MapWriteGuard<'a> {
        self.map.lock()
    }

    /// `vm_map_lock_read(map)`.
    pub fn lock_read(&self) -> MapGuard<'a> {
        self.map.lock_read()
    }

    /// `vm_map_lock(map); vm_map_unbusy(map);`.
    pub fn relock_unbusy(self) -> MapWriteGuard<'a> {
        let g = self.map.lock();
        drop(self);
        g
    }
}

impl Drop for MapBusy<'_> {
    /// `vm_map_unbusy(map)` (`vm_map_unbusy_ln`).
    fn drop(&mut self) {
        let map = self.map;
        kassert!(!map.is_intrsafe());
        let nbusy = {
            let mut f = map.flags_lock.lock();
            kassert!(f.owner == curproc_word());
            f.owner = 0;
            f.nbusy
        };
        if nbusy > 0 {
            wakeup(ptr::from_ref(&map.flags_lock));
        }
    }
}

/// `struct vmspace`: shareable process virtual address space. May eventually be merged with
/// `vm_map`. Several fields are temporary (text, data stuff): they are the map's
/// [`VmspaceVars`], which a vmspace derefs to (`vm.vm_dused`).
///
/// Locks used to protect struct members: `a` atomic operations, `v` `vm_map`'s lock.
pub struct Vmspace {
    /// `vm_map`: VM address map.
    pub vm_map: VmMap,
    /// \[a\] `vm_refcnt`: number of references (processes, kthreads, `kern.proc_args`).
    pub vm_refcnt: AtomicI32,
    // vm_shm: SYSVSHM is not configured.
}

impl Vmspace {
    /// A vmspace before `uvmspace_init`.
    const fn new() -> Self {
        Self {
            vm_map: VmMap::with_name(0, 0, "vmmaplk"),
            vm_refcnt: AtomicI32::new(0),
        }
    }
}

impl Deref for Vmspace {
    type Target = VmspaceVars;

    fn deref(&self) -> &VmspaceVars {
        &self.vm_map.vs
    }
}

/// A counted handle to a vmspace (`struct vmspace *`); the C's `vm_refcnt` is the vmspace's
/// own count, changed by `uvmspace_addref`/`uvmspace_free`.
pub type VmspaceRef = PoolArc<Vmspace>;

/// `struct p_inentry`: the range a thread's stack or pc was last found in, and the map's
/// `sserial` then (`uvm_map_inentry`'s cache).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InentryCache {
    /// `ie_start`.
    pub start: usize,
    /// `ie_end`.
    pub end: usize,
    /// `ie_serial`.
    pub serial: u64,
}

/// A write-locked map's state and the entries' proof, as the fault's guard
/// (`uvm_fault_unwire_locked` from inside a map operation, the C's call with the map
/// locked): reads only.
struct HeldView<'s, 'w> {
    map: &'s VmMap,
    st: &'s MapState,
    part: &'s WitnessPart<'w>,
}

/// `kernel_map_entry[MAX_KMAPENT]` and the pages `uvm_mapent_alloc` carves when they run out
/// (`uvm.kentry_free`): the static entries of `INTRSAFE` maps and of `cold`.
static KENTRY_ARENA: StaticArena<VmMapEntry, MAX_KMAPENT> = StaticArena::new();
/// The static entries' pool (`UVM_MAP_STATIC`), over [`KENTRY_ARENA`]; not a `pool(9)`, as
/// in the C (`vmstat -m` shows none).
static KENTRY_POOL: BoxPool<VmMapEntry> = BoxPool::new();
/// `uvm_map_entry_pool`: pool for dynamically-allocated map entries (`"vmmpepl"`).
static ENTRY_POOL: BoxPool<VmMapEntry> = BoxPool::new();
/// `uvm_map_entry_kmem_pool`: the kernel map's entries (`"vmmpekpl"`, `UVM_MAP_KMEM`).
static KMEM_ENTRY_POOL: BoxPool<VmMapEntry> = BoxPool::new();
/// `uvm_vmspace_pool`: pool for vmspace structures (`"vmsppl"`).
static VMSPACE_POOL: TypedPool<Vmspace> = TypedPool::new();

/// The static entries' refill pages: `km_alloc(PAGE_SIZE, &kv_page, &kp_zero,
/// &kd_nowait)`, the direct map's single page (uvm_map.c:1497's `kv_page`): no map entry, no
/// map lock, so a refill under a held map lock (`cold`, or an `INTRSAFE` map's mutex at
/// `IPL_VM`) never waits for a map.
struct KentryPages;

// The provider's contract (`hal::kmbuf::PageMemory`): a block `km_alloc` returns is fresh
// kernel memory of `len` bytes (one page here), zeroed (`kp_zero`), page-aligned, which nothing
// else references until `free` hands it to `km_free` with the same modes (never: the arena
// keeps its pages).
impl PageMemory for KentryPages {
    fn alloc(&self, len: usize, _kind: PageKind, _wait: bool) -> Option<(NonNull<u8>, usize)> {
        kassert!(len == PAGE_SIZE);
        #[cfg(test)]
        KENTRY_PAGE_ALLOCS.fetch_add(1, Mem::Relaxed);
        let va = km_alloc(len, &KV_PAGE, &KP_ZERO, &KD_NOWAIT)?;
        let p = NonNull::new(ptr::with_exposed_provenance_mut::<u8>(va.as_usize()))?;
        Some((p, len))
    }

    fn free(&self, p: NonNull<u8>, len: usize, _kind: PageKind) {
        km_free(Vaddr::new(p.as_ptr().addr()), len, &KV_PAGE, &KP_ZERO);
    }

    fn page_size(&self) -> usize {
        PAGE_SIZE
    }
}

/// [`KentryPages`]' provider.
static KENTRY_PAGES: KentryPages = KentryPages;

/// The pages [`KentryPages`] was asked for (the host tests: on the host both `kv_any` and
/// `kv_page` are served by the direct map, so the provider is what tells them apart).
#[cfg(test)]
static KENTRY_PAGE_ALLOCS: AtomicU32 = AtomicU32::new(0);

/// `uvm_kmapent_mtx`: serialises the static entries' refill (the arena keeps its own list).
static UVM_KMAPENT_MTX: Mutexed<()> = Mutexed::new(IPL_VM, ());
/// `uvm_kmapent_last_warn_time`: `(tv_sec, tv_usec)`.
static UVM_KMAPENT_LAST_WARN_TIME: Mutexed<(i64, i64)> = Mutexed::new(IPL_VM, (0, 0));

/// `uvm_maxkaddr`: this global represents the end of the kernel virtual address space. If
/// we want to exceed this, we must grow the kernel virtual address space dynamically.
///
/// Note, this variable is locked by kernel_map's lock (an atomic: the selectors read it).
pub static UVM_MAXKADDR: AtomicUsize = AtomicUsize::new(VM_MIN_KERNEL_ADDRESS);

/// `uvm_maxkaddr`'s value.
pub fn uvm_maxkaddr() -> usize {
    UVM_MAXKADDR.load(Mem::Relaxed)
}

impl FaultMap for VmMap {
    type Object = ObjRef;
    type Entry = VmMapEntry;
    type Guard<'a> = MapGuard<'a>;

    fn min_offset(&self) -> usize {
        VmMap::min_offset(self)
    }

    fn max_offset(&self) -> usize {
        VmMap::max_offset(self)
    }

    fn lock(&self, mode: RwMode) -> MapGuard<'_> {
        match mode {
            RwMode::Write => VmMap::lock(self).into_guard(),
            RwMode::Read => self.lock_read(),
        }
    }

    fn relock(&self, version: u32) -> Option<MapGuard<'_>> {
        VmMap::relock(self, version)
    }

    fn pmap(&self) -> &PmapRef {
        VmMap::pmap(self)
    }

    fn is_vmspace(&self) -> bool {
        VmMap::is_vmspace(self)
    }

    fn is_intrsafe(&self) -> bool {
        VmMap::is_intrsafe(self)
    }

    fn is_pageable(&self) -> bool {
        VmMap::is_pageable(self)
    }

    fn note_minflt(&self) {
        vmproc().ru_minflt_inc();
    }

    fn note_majflt(&self) {
        vmproc().ru_majflt_inc();
    }

    fn note_maxrss(&self, kb: usize) {
        // KASSERT(p != NULL && &p->p_vmspace->vm_map == map): the map is a vmspace's (the
        // module's deviations).
        kassert!(VmMap::is_vmspace(self));
        vmproc().ru_maxrss(kb);
    }
}

impl<'a> FaultMapGuard for MapGuard<'a> {
    type Entry = VmMapEntry;
    type Map = VmMap;
    type Object = ObjRef;

    fn map(&self) -> &VmMap {
        self.map
    }

    fn mode(&self) -> RwMode {
        self.mode
    }

    fn version(&self) -> u32 {
        MapGuard::version(self)
    }

    fn lookup(&self, va: usize) -> Option<&VmMapEntry> {
        MapGuard::lookup(self, va)
    }

    fn next<'e>(&'e self, entry: &'e VmMapEntry) -> Option<&'e VmMapEntry> {
        self.state().addr.next(entry)
    }

    fn upgrade(self) -> Result<Self, Self> {
        MapGuard::upgrade(self).map(MapWriteGuard::into_guard)
    }

    fn downgrade(self) -> Self {
        match self.into_write() {
            Ok(w) => w.downgrade(),
            Err(r) => r,
        }
    }

    fn copy_amap(&mut self, va: usize, waitf: i32, canchunk: bool, start: usize, end: usize) {
        kassert!(self.mode == RwMode::Write);
        let map = self.map;
        let mut sp = self.split_write();
        let (st, ew) = sp.parts();
        let Some(key) = uvm_map_lookup_entry(&st.addr, va).map(VmMapEntry::start) else {
            kpanic(format_args!("amap_copy: no entry holds {va:#x}"));
        };
        amap_copy(map, st, ew, key, waitf, canchunk, start, end);
    }

    fn submap(&self, entry: &VmMapEntry) -> Option<&'static VmMap> {
        entry.handles(self.witness()).sub_map()
    }

    fn aref(&self, entry: &VmMapEntry) -> Option<(AmapRef, i32)> {
        let h = entry.handles(self.witness());
        h.amap.clone().map(|a| (a, h.pageoff))
    }

    fn object(&self, entry: &VmMapEntry) -> Option<ObjRef> {
        entry.handles(self.witness()).uvm_obj().cloned()
    }
}

impl FaultMapGuard for HeldView<'_, '_> {
    type Entry = VmMapEntry;
    type Map = VmMap;
    type Object = ObjRef;

    fn map(&self) -> &VmMap {
        self.map
    }

    fn mode(&self) -> RwMode {
        RwMode::Write
    }

    fn version(&self) -> u32 {
        self.map.timestamp()
    }

    fn lookup(&self, va: usize) -> Option<&VmMapEntry> {
        uvm_map_lookup_entry(&self.st.addr, va)
    }

    fn next<'e>(&'e self, entry: &'e VmMapEntry) -> Option<&'e VmMapEntry> {
        self.st.addr.next(entry)
    }

    fn upgrade(self) -> Result<Self, Self> {
        // Held for writing already.
        Ok(self)
    }

    fn downgrade(self) -> Self {
        kpanic(format_args!(
            "uvm_map: the view of a held write lock is not downgraded"
        ));
    }

    fn copy_amap(&mut self, _va: usize, _waitf: i32, _canchunk: bool, _s: usize, _e: usize) {
        kpanic(format_args!(
            "uvm_map: amap_copy through the view of a held write lock"
        ));
    }

    fn submap(&self, entry: &VmMapEntry) -> Option<&'static VmMap> {
        entry.handles(self.part).sub_map()
    }

    fn aref(&self, entry: &VmMapEntry) -> Option<(AmapRef, i32)> {
        let h = entry.handles(self.part);
        h.amap.clone().map(|a| (a, h.pageoff))
    }

    fn object(&self, entry: &VmMapEntry) -> Option<ObjRef> {
        entry.handles(self.part).uvm_obj().cloned()
    }
}

impl FaultEntry for VmMapEntry {
    type Map = VmMap;
    type Object = ObjRef;

    fn start(&self) -> usize {
        VmMapEntry::start(self)
    }

    fn end(&self) -> usize {
        VmMapEntry::end(self)
    }

    fn offset(&self) -> Voff {
        VmMapEntry::offset(self)
    }

    fn protection(&self) -> VmProt {
        VmMapEntry::protection(self)
    }

    fn advice(&self) -> i32 {
        VmMapEntry::advice(self)
    }

    fn is_wired(&self) -> bool {
        vm_mapent_iswired(self)
    }

    fn is_submap(&self) -> bool {
        uvm_et_issubmap(self)
    }

    fn is_copyonwrite(&self) -> bool {
        uvm_et_iscopyonwrite(self)
    }

    fn is_needscopy(&self) -> bool {
        uvm_et_isneedscopy(self)
    }

    fn is_nofault(&self) -> bool {
        uvm_et_isnofault(self)
    }

    fn is_stack(&self) -> bool {
        uvm_et_isstack(self)
    }

    fn is_wc(&self) -> bool {
        uvm_et_iswc(self)
    }
}

/// `uvm_fault_unwire_locked(map, start, end)` from inside a map operation, which holds the
/// write lock (as its `st` and `ew`).
fn unwire_held(map: &VmMap, st: &MapState, ew: &EntryWrite<'_>, start: usize, end: usize) {
    let view = HeldView {
        map,
        st,
        part: &ew.part,
    };
    uvm_fault_unwire_locked(&view, start, end);
}

/// The entry at `key` (its start) in `tree`; a missing one is a broken invariant of the
/// caller's (`who`).
fn ent<'t>(tree: &'t OwnedTree<ByAddr>, key: usize, who: &str) -> &'t VmMapEntry {
    match tree.find_by(|n| key.cmp(&n.start())) {
        Some(e) => e,
        None => kpanic(format_args!("{who}: no entry at {key:#x}")),
    }
}

/// The entry at `key`, if any.
fn find(tree: &OwnedTree<ByAddr>, key: usize) -> Option<&VmMapEntry> {
    tree.find_by(|n| key.cmp(&n.start()))
}

/// The key of the entry after the one at `key`.
fn next_key(tree: &OwnedTree<ByAddr>, key: usize) -> Option<usize> {
    find(tree, key)
        .and_then(|e| tree.next(e))
        .map(VmMapEntry::start)
}

/// The entry that describes `addr`, which every address inside the map has (the tree has
/// no gaps); a missing one is a corrupted tree.
fn entry_at<'t>(
    map: &VmMap,
    tree: &'t OwnedTree<ByAddr>,
    addr: usize,
    who: &str,
) -> &'t VmMapEntry {
    match uvm_map_entrybyaddr(tree, addr) {
        Some(e) => e,
        None => kpanic(format_args!(
            "{who}: map {:#x} has no entry for address {addr:#x}",
            map.addr()
        )),
    }
}

/// `uvm_mapent_copy`: copy mapentry (the fields from `uvm_map_entry_start_copy` to
/// `uvm_map_entry_stop_copy`: not the links, `flags` or `fspace_augment`) from `src`, whose
/// handles `w` reads, into `dst`, an entry no container holds (the handles cloned: the C's
/// pointers are copied without a reference, which the callers take where the C does).
fn uvm_mapent_copy(src: &VmMapEntry, w: &EntryWrite<'_>, dst: &mut VmMapEntry) {
    copy_scalars(src, w, dst);
    *dst.handles_owned() = src.handles(w.witness()).clone();
}

/// The scalars of [`uvm_mapent_copy`].
fn copy_scalars(src: &VmMapEntry, w: &EntryWrite<'_>, dst: &VmMapEntry) {
    dst.set_start(w, src.start());
    dst.set_end(w, src.end());
    dst.set_guard(w, src.guard());
    dst.set_fspace(w, src.fspace());
    dst.set_offset(w, src.offset());
    dst.set_etype(w, src.etype());
    dst.set_protection(w, src.protection());
    dst.set_max_protection(w, src.max_protection());
    dst.set_inheritance(w, src.inheritance());
    dst.set_wired_count(w, src.wired_count());
    dst.set_advice(w, src.advice());
}

/// `uvm_mapent_free_insert`: handle free-list insertion of `e`, an entry of `fp`'s tree,
/// whose free space `uaddr` owns.
fn uvm_mapent_free_insert(
    fp: &mut FreeParts<'_>,
    ew: &EntryWrite<'_>,
    uaddr: Option<Slot>,
    e: &VmMapEntry,
) {
    if cfg!(test) {
        // Boundary check (VMMAP_DEBUG). Boundaries are folded if they go on the same free
        // list.
        let mut min = vmmap_free_start(e);
        let max = vmmap_free_end(e);
        while min < max {
            let bound = fp.owners.boundary(min, max);
            kassert!(fp.owners.uaddr(min) == uaddr);
            min = bound;
        }
    }

    kdassert!(e.fspace() & PAGE_MASK == 0);
    kassert!(e.etype() & UVM_ET_FREEMAPPED == 0);

    // Actual insert: forward to uaddr pointer.
    if let Some(sel) = uaddr.and_then(|slot| fp.selector_mut(slot)) {
        if sel.has_free_ops() {
            sel.free_insert(e);
        }
        e.etype_set(ew, UVM_ET_FREEMAPPED);
    }

    // Update fspace augmentation.
    fp.tree.augment(e);
}

/// `uvm_mapent_free_remove`: handle free-list removal of `e`, an entry of `fp`'s tree.
fn uvm_mapent_free_remove(
    fp: &mut FreeParts<'_>,
    ew: &EntryWrite<'_>,
    uaddr: Option<Slot>,
    e: &VmMapEntry,
) {
    kassert!(fp.tree.contains(e));
    kassert!(e.etype() & UVM_ET_FREEMAPPED != 0 || uaddr.is_none());
    kassert!(fp.owners.uaddr_e(e) == uaddr);

    if let Some(sel) = uaddr.and_then(|slot| fp.selector_mut(slot)) {
        if sel.has_free_ops() {
            sel.free_remove(e);
        }
        e.etype_clear(ew, UVM_ET_FREEMAPPED);
    }
}

/// `uvm_mapent_addr_insert`: handle address tree insertion; the entry's key.
fn uvm_mapent_addr_insert(map: &VmMap, st: &mut MapState, entry: PoolBox<VmMapEntry>) -> usize {
    kdassert!(entry.start() <= entry.end());
    kdassert!(entry.start() & PAGE_MASK == 0 && entry.end() & PAGE_MASK == 0);

    // TRACEPOINT(uvm, map_insert, ...): dt is not configured.

    let key = entry.start();
    if let Some(back) = st.addr.insert(entry) {
        let res = ent(&st.addr, key, "uvm_mapent_addr_insert");
        kpanic(format_args!(
            "uvm_mapent_addr_insert: map {:#x} entry {:#x} ({:#x}-{:#x} G={:#x} F={:#x}) insert collision with entry {:#x} ({:#x}-{:#x} G={:#x} F={:#x})",
            map.addr(),
            PoolBox::as_ptr(&back).addr(),
            back.start(),
            back.end(),
            back.guard(),
            back.fspace(),
            ptr::from_ref(res).addr(),
            res.start(),
            res.end(),
            res.guard(),
            res.fspace()
        ));
    }
    key
}

/// `uvm_mapent_addr_remove`: handle address tree removal; the entry's box.
fn uvm_mapent_addr_remove(st: &mut MapState, key: usize) -> PoolBox<VmMapEntry> {
    // TRACEPOINT(uvm, map_remove, ...): dt is not configured.
    match st.addr.take_by(|n| key.cmp(&n.start())) {
        Some(b) => b,
        None => kpanic(format_args!("uvm_mapent_addr_remove")),
    }
}

/// `uvm_map_reference`: add reference to a map.
///
/// - map need not be locked
pub fn uvm_map_reference(map: &VmMap) {
    map.ref_count.fetch_add(1, Mem::Relaxed);
}

/// `uvm_map_lock_entry`: locks the entry's amap and object (for writing, the amap's first:
/// MAPS => AMAP => UOBJ), whose handles `h` holds; the locks are released by the drop
/// (`uvm_map_unlock_entry`: the object's lock, then the amap's).
pub fn uvm_map_lock_entry(h: &Handles) -> EntryLocks<'_> {
    EntryLocks::new(
        h.amap.as_ref().map(|a| a.lock()),
        h.uvm_obj().map(|o| o.lock()),
    )
}

/// `uvmspace_dused`: calculate the dused delta.
pub fn uvmspace_dused(map: &VmMap, min: usize, max: usize) -> usize {
    let vm = map.vm();
    let stack_begin = vm
        .vm_maxsaddr
        .load(Mem::Relaxed)
        .min(vm.vm_minsaddr.load(Mem::Relaxed));
    let stack_end = vm
        .vm_maxsaddr
        .load(Mem::Relaxed)
        .max(vm.vm_minsaddr.load(Mem::Relaxed));

    let mut sz = 0;
    let mut min = min;
    while min != max {
        let mut lmax = max;
        if min < stack_begin && lmax > stack_begin {
            lmax = stack_begin;
        } else if min < stack_end && lmax > stack_end {
            lmax = stack_end;
        }

        if !(min >= stack_begin && min < stack_end) {
            sz += lmax - min;
        }
        min = lmax;
    }

    sz >> PAGE_SHIFT
}

/// `uvm_map_entrybyaddr`: find the entry describing the given address.
pub fn uvm_map_entrybyaddr(atree: &OwnedTree<ByAddr>, addr: usize) -> Option<&VmMapEntry> {
    let mut iter = atree.root();
    while let Some(e) = iter {
        if e.start() > addr {
            iter = atree.left(e);
        } else if vmmap_free_end(e) <= addr {
            iter = atree.right(e);
        } else {
            return Some(e);
        }
    }
    None
}

/// `uvm_map_isavail`: test if memory starting at addr with sz bytes is free.
///
/// Fills in `start_ptr` and `end_ptr` to be the first and last entry describing the space.
/// If called with prefilled `start_ptr` and `end_ptr`, they are to be correct.
#[allow(clippy::too_many_arguments)] // the C's signature, the map's parts apart
pub fn uvm_map_isavail<'t>(
    map: &VmMap,
    atree: &'t OwnedTree<ByAddr>,
    owners: &Owners,
    uaddr: Option<Slot>,
    start_ptr: &mut Option<&'t VmMapEntry>,
    end_ptr: &mut Option<&'t VmMapEntry>,
    addr: usize,
    sz: usize,
) -> bool {
    if addr.wrapping_add(sz) < addr {
        return false;
    }

    // Kernel memory above uvm_maxkaddr is considered unavailable.
    if !map.is_vmspace() && addr + sz > uvm_maxkaddr() {
        return false;
    }

    // Fill in first, last, so they point at the entries containing the first and last
    // address of the range. Note that if they are not NULL, we don't perform the lookup.
    let first = match *start_ptr {
        None => match uvm_map_entrybyaddr(atree, addr) {
            Some(e) => {
                *start_ptr = Some(e);
                e
            }
            None => return false,
        },
        Some(e) => {
            kassert!(uvm_map_entrybyaddr(atree, addr).is_some_and(|f| ptr::eq(f, e)));
            e
        }
    };
    let last = match *end_ptr {
        None => {
            if vmmap_free_end(first) >= addr + sz {
                *end_ptr = Some(first);
                first
            } else {
                match uvm_map_entrybyaddr(atree, addr + sz - 1) {
                    Some(e) => {
                        *end_ptr = Some(e);
                        e
                    }
                    None => return false,
                }
            }
        }
        Some(e) => {
            kassert!(uvm_map_entrybyaddr(atree, addr + sz - 1).is_some_and(|l| ptr::eq(l, e)));
            e
        }
    };

    // Validation.
    kdassert!(
        first.start() <= addr
            && vmmap_free_end(first) > addr
            && last.start() < addr + sz
            && vmmap_free_end(last) >= addr + sz
    );

    // Check the none of the entries intersects with <addr, addr+sz>. Also, if the entry
    // belong to uaddr_exe or uaddr_brk_stack, it is considered unavailable unless called by
    // those allocators.
    let mut iter = Some(first);
    while let Some(i) = iter {
        if i.start() != i.end() && i.end() > addr {
            return false;
        }

        // uaddr_exe and uaddr_brk_stack may only be used by these allocators and the NULL
        // uaddr (i.e. no uaddr). Reject if this requirement is not met.
        if let Some(uaddr) = uaddr
            && let Some(free) = owners.uaddr_e(i)
            && uaddr != free
            && (free == Slot::Exe || free == Slot::BrkStack)
        {
            return false;
        }
        if ptr::eq(i, last) {
            break;
        }
        iter = atree.next(i);
    }

    true
}

/// `uvm_map_findspace`: invoke each address selector until an address is found. Will not
/// invoke uaddr_exe. Yields the first and last entry (their keys) and the address.
pub fn uvm_map_findspace(
    map: &VmMap,
    st: &mut MapState,
    sz: usize,
    pmap_align: usize,
    pmap_offset: usize,
    prot: VmProt,
    hint: usize,
) -> Result<(usize, usize, usize), Errno> {
    let mut hint = hint;
    loop {
        // Allocation for sz bytes at any address, using the addr selectors in order.
        for i in 0..st.uaddr_any.len() {
            if let Ok(found) = uvm_addr_invoke(
                map,
                st,
                Slot::Any(i),
                sz,
                pmap_align,
                pmap_offset,
                prot,
                hint,
            ) {
                return Ok(found);
            }
        }

        // Fall back to brk() and stack() address selectors.
        if let Ok(found) = uvm_addr_invoke(
            map,
            st,
            Slot::BrkStack,
            sz,
            pmap_align,
            pmap_offset,
            prot,
            hint,
        ) {
            return Ok(found);
        }

        if hint != 0 {
            hint = 0;
            continue;
        }

        return Err(Errno::ENOMEM);
    }
}

/// `pmap_remove(map->pmap, sva, eva)`: the kernel pmap through the kernel map's token, a user
/// pmap through its handle.
fn map_pmap_remove(map: &VmMap, sva: usize, eva: usize) {
    let pm = map.pmap();
    if pm.is_kernel() {
        mapper().remove(Vaddr::new(sva), Vaddr::new(eva));
    } else {
        pm.remove(Vaddr::new(sva), Vaddr::new(eva));
    }
}

/// `pmap_protect(map->pmap, sva, eva, prot)`, as [`map_pmap_remove`].
fn map_pmap_protect(map: &VmMap, sva: usize, eva: usize, prot: VmProt) {
    let pm = map.pmap();
    if pm.is_kernel() {
        mapper().protect(Vaddr::new(sva), Vaddr::new(eva), prot);
    } else {
        pm.protect(Vaddr::new(sva), Vaddr::new(eva), prot);
    }
}

/// `uvm_mapanon`: establish a valid mapping in map for an anon.
///
/// - `*addr` and sz must be a multiple of `PAGE_SIZE`.
/// - `*addr` is ignored, except if flags contains `UVM_FLAG_FIXED`.
/// - map must be unlocked.
/// - align: align vaddr, must be a power-of-2. Align is only a hint and will be ignored if
///   the alignment fails.
pub fn uvm_mapanon(
    map: &VmMap,
    addr: &mut usize,
    sz: usize,
    align: usize,
    flags: u32,
) -> Result<(), Errno> {
    kassert!(map.is_vmspace());
    kassert!(!is_kernel_map(map));
    kassert!(map.flags() & UVM_FLAG_HOLE as i32 == 0);
    kassert!(!map.is_intrsafe());
    splassert(IPL_NONE, "uvm_mapanon");
    kassert!(flags & UVM_FLAG_TRYLOCK == 0);

    // Decode parameters.
    let prot = uvm_protection(flags);
    let maxprot = uvm_maxprotection(flags);
    kassert!(sz & PAGE_MASK == 0);
    kassert!(align & align.wrapping_sub(1) == 0);

    // Check protection.
    if prot & maxprot != prot {
        return Err(Errno::EACCES);
    }

    // Before grabbing the lock, allocate a map entry for later use to ensure we don't wait
    // for memory while holding the vm_map_lock.
    let mut new = Some(uvm_mapent_alloc(map, flags).ok_or(Errno::ENOMEM)?);

    let mut dead = DeadQueue::new();
    let mut g = map.lock();
    let error = {
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        uvm_mapanon_locked(map, st, ew, addr, sz, align, flags, &mut dead, &mut new)
    };
    drop(g);

    // Remove dead entries.
    //
    // Dead entries may be the result of merging. uvm_map_mkentry may also create dead
    // entries, when it attempts to destroy free-space entries.
    uvm_unmap_detach(&mut dead, 0);

    // uvm_mapent_free(new), when unused.
    drop(new);
    error
}

/// [`uvm_mapanon`] between its lock and unlock (the C's body up to the `unlock` label).
#[allow(clippy::too_many_arguments)] // the C's locals
fn uvm_mapanon_locked(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    addr: &mut usize,
    sz: usize,
    align: usize,
    flags: u32,
    dead: &mut DeadQueue,
    new: &mut Option<PoolBox<VmMapEntry>>,
) -> Result<(), Errno> {
    // We use pmap_align and pmap_offset as alignment and offset variables.
    //
    // Because the align parameter takes precedence over pmap prefer, the pmap_align will
    // need to be set to align, with pmap_offset = 0, if pmap_prefer will not align.
    let pmap_align = align.max(PAGE_SIZE);
    let pmap_offset = 0;

    let prot = uvm_protection(flags);
    let maxprot = uvm_maxprotection(flags);
    let advice = uvm_advice(flags);
    let inherit = uvm_inherit(flags);
    let hint = trunc_page(*addr);

    let (first, last);
    if flags & UVM_FLAG_FIXED != 0 {
        // Fixed location.
        //
        // Note: we ignore align, pmap_prefer. Fill in first, last and *addr.
        kassert!(*addr & PAGE_MASK == 0);

        // Check that the space is available.
        if flags & UVM_FLAG_UNMAP != 0 {
            if flags & UVM_FLAG_STACK != 0
                && !uvm_map_is_stack_remappable(
                    &st.addr,
                    *addr,
                    sz,
                    flags & UVM_FLAG_SIGALTSTACK != 0,
                )
            {
                return Err(Errno::EINVAL);
            }
            if unmap_remove(
                map,
                st,
                ew,
                *addr,
                *addr + sz,
                dead,
                false,
                true,
                flags & UVM_FLAG_SIGALTSTACK == 0,
            )
            .is_err()
            {
                return Err(Errno::EPERM); // immutable entries found
            }
        }
        (first, last) = isavail_keys(map, st, None, *addr, sz).ok_or(Errno::ENOMEM)?;
    } else if let Some(found) =
        (*addr != 0 && *addr & PAGE_MASK == 0 && (align == 0 || *addr & (align - 1) == 0))
            .then(|| isavail_keys(map, st, None, *addr, sz))
            .flatten()
    {
        // Address used as hint.
        //
        // Note: we enforce the alignment restriction, but ignore pmap_prefer.
        (first, last) = found;
    } else if prot & PROT_EXEC != 0 && st.uaddr_exe.is_some() {
        // Run selection algorithm for executables.
        let (f, l, a) =
            uvm_addr_invoke(map, st, Slot::Exe, sz, pmap_align, pmap_offset, prot, hint)?;
        (first, last) = (f, l);
        *addr = a;
    } else {
        // Update freelists from vmspace.
        uvm_map_vmspace_update(map, st, ew, dead, flags);

        let (f, l, a) = uvm_map_findspace(map, st, sz, pmap_align, pmap_offset, prot, hint)?;
        (first, last) = (f, l);
        *addr = a;
    }

    // Double-check if selected address doesn't cause overflow.
    if addr.wrapping_add(sz) < *addr {
        return Err(Errno::ENOMEM);
    }

    // If we only want a query, return now.
    if flags & UVM_FLAG_QUERY != 0 {
        return Ok(());
    }

    // Create new entry. first and last may be invalidated after this call.
    let entry = uvm_map_mkentry(map, st, ew, first, last, *addr, sz, flags, dead, new.take())
        .ok_or(Errno::ENOMEM)?;
    kdassert!(entry.start() == *addr && entry.end() == *addr + sz);
    entry.set_offset(ew, 0);
    entry.set_protection(ew, prot);
    entry.set_max_protection(ew, maxprot);
    entry.set_inheritance(ew, inherit);
    entry.set_wired_count(ew, 0);
    entry.set_advice(ew, advice);
    if flags & UVM_FLAG_STACK != 0 {
        entry.etype_set(ew, UVM_ET_STACK);
        if flags & (UVM_FLAG_FIXED | UVM_FLAG_UNMAP) != 0 {
            map.sserial.fetch_add(1, Mem::Relaxed);
        }
    }
    if flags & UVM_FLAG_COPYONW != 0 {
        entry.etype_set(ew, UVM_ET_COPYONWRITE);
        if flags & UVM_FLAG_OVERLAY == 0 {
            entry.etype_set(ew, UVM_ET_NEEDSCOPY);
        }
    }
    if flags & UVM_FLAG_CONCEAL != 0 {
        entry.etype_set(ew, UVM_ET_CONCEAL);
    }
    {
        let h = entry.handles_mut(ew);
        h.object = EntryObject::None;
        if flags & UVM_FLAG_OVERLAY != 0 {
            h.pageoff = 0;
            h.amap = amap_alloc(sz, M_WAITOK, false);
        }
    }

    // Update map and process statistics.
    map.size.fetch_add(sz, Mem::Relaxed);
    if prot != PROT_NONE {
        map.vm()
            .dused_add(uvmspace_dused(map, *addr, *addr + sz) as isize);
    }

    Ok(())
}

/// [`uvm_map_isavail`] with nothing prefilled, the result as the entries' keys.
fn isavail_keys(
    map: &VmMap,
    st: &MapState,
    uaddr: Option<Slot>,
    addr: usize,
    sz: usize,
) -> Option<(usize, usize)> {
    let owners = st.owners(map);
    let mut first = None;
    let mut last = None;
    if !uvm_map_isavail(
        map, &st.addr, &owners, uaddr, &mut first, &mut last, addr, sz,
    ) {
        return None;
    }
    Some((first?.start(), last?.start()))
}

/// `uvm_map`: establish a valid mapping in map.
///
/// - `*addr` and sz must be a multiple of `PAGE_SIZE`.
/// - map must be unlocked.
/// - `<uobj,uoffset>` value meanings (4 cases):
///   1. `<NULL,uoffset>`: uoffset is a hint for `PMAP_PREFER`;
///   2. `<NULL,UVM_UNKNOWN_OFFSET>`: don't `PMAP_PREFER`;
///   3. `<uobj,uoffset>`: normal mapping;
///   4. `<uobj,UVM_UNKNOWN_OFFSET>`: uvm_map finds offset based on VA.
///
///   Case 4 is for kernel mappings where we don't know the offset until we've found a
///   virtual address. Note that kernel object offsets are always relative to
///   `vm_map_min(kernel_map)`.
/// - align: align vaddr, must be a power-of-2. Align is only a hint and will be ignored if
///   the alignment fails.
pub fn uvm_map(
    map: &VmMap,
    addr: &mut usize,
    sz: usize,
    uobj: Option<ObjRef>,
    uoffset: Voff,
    align: usize,
    flags: u32,
) -> Result<(), Errno> {
    if !map.is_intrsafe() {
        splassert(IPL_NONE, "uvm_map");
    } else {
        splassert(IPL_VM, "uvm_map");
    }

    // Decode parameters.
    let prot = uvm_protection(flags);
    let maxprot = uvm_maxprotection(flags);
    kassert!(sz & PAGE_MASK == 0);
    kassert!(align & align.wrapping_sub(1) == 0);

    // Holes are incompatible with other types of mappings.
    if flags & UVM_FLAG_HOLE != 0 {
        kassert!(
            uobj.is_none()
                && flags & UVM_FLAG_FIXED != 0
                && flags & (UVM_FLAG_OVERLAY | UVM_FLAG_COPYONW) == 0
        );
    }

    // Check protection.
    if prot & maxprot != prot {
        return Err(Errno::EACCES);
    }

    if is_kernel_map(map) && prot & (PROT_WRITE | PROT_EXEC) == (PROT_WRITE | PROT_EXEC) {
        kpanic(format_args!("uvm_map: kernel map W^X violation requested"));
    }

    // Before grabbing the lock, allocate a map entry for later use to ensure we don't wait
    // for memory while holding the vm_map_lock.
    let mut new = Some(uvm_mapent_alloc(map, flags).ok_or(Errno::ENOMEM)?);

    let mut g = if flags & UVM_FLAG_TRYLOCK != 0 {
        match map.lock_try() {
            Some(g) => g,
            None => return Err(Errno::EFAULT),
        }
    } else {
        map.lock()
    };

    let mut dead = DeadQueue::new();
    let error = {
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        uvm_map_locked(
            map, st, ew, addr, sz, uobj, uoffset, align, flags, &mut dead, &mut new,
        )
    };
    drop(g);

    // Remove dead entries.
    //
    // Dead entries may be the result of merging. uvm_map_mkentry may also create dead
    // entries, when it attempts to destroy free-space entries.
    if map.is_intrsafe() {
        uvm_unmap_detach_intrsafe(&mut dead);
    } else {
        uvm_unmap_detach(&mut dead, 0);
    }
    drop(new);
    error
}

/// [`uvm_map`] between its lock and unlock (the C's body up to the `unlock` label).
#[allow(clippy::too_many_arguments)] // the C's locals
fn uvm_map_locked(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    addr: &mut usize,
    sz: usize,
    uobj: Option<ObjRef>,
    uoffset: Voff,
    align: usize,
    flags: u32,
    dead: &mut DeadQueue,
    new: &mut Option<PoolBox<VmMapEntry>>,
) -> Result<(), Errno> {
    // We use pmap_align and pmap_offset as alignment and offset variables.
    //
    // Because the align parameter takes precedence over pmap prefer, the pmap_align will
    // need to be set to align, with pmap_offset = 0, if pmap_prefer will not align.
    let (pmap_align, pmap_offset) = if uoffset == UVM_UNKNOWN_OFFSET {
        (align.max(PAGE_SIZE), 0)
    } else {
        // PMAP_PREFER is not defined (amd64, arm64): PMAP_PREFER_ALIGN() and
        // PMAP_PREFER_OFFSET() are 0.
        let pmap_align = PAGE_SIZE;
        let pmap_offset = 0;

        if align == 0 || (align <= pmap_align && pmap_offset & (align - 1) == 0) {
            // pmap_offset satisfies align, no change.
            (pmap_align, pmap_offset)
        } else {
            // Align takes precedence over pmap prefer.
            (align, 0)
        }
    };

    let prot = uvm_protection(flags);
    let maxprot = uvm_maxprotection(flags);
    let advice = uvm_advice(flags);
    let inherit = uvm_inherit(flags);
    let mut hint = trunc_page(*addr);
    let mut uoffset = uoffset;

    // Unset hint for kernel_map non-fixed allocations.
    if !map.is_vmspace() && flags & UVM_FLAG_FIXED == 0 {
        hint = 0;
    }

    let (first, last);
    if flags & UVM_FLAG_FIXED != 0 {
        // Fixed location.
        //
        // Note: we ignore align, pmap_prefer. Fill in first, last and *addr.
        kassert!(*addr & PAGE_MASK == 0);

        // Grow pmap to include allocated address. If the growth fails, the allocation will
        // fail too.
        if !map.is_vmspace() && uvm_maxkaddr() < *addr + sz {
            uvm_map_kmem_grow(map, st, ew, dead, *addr + sz - uvm_maxkaddr(), flags);
        }

        // Check that the space is available.
        if flags & UVM_FLAG_UNMAP != 0
            && unmap_remove(map, st, ew, *addr, *addr + sz, dead, false, true, true).is_err()
        {
            return Err(Errno::EPERM); // immutable entries found
        }
        (first, last) = isavail_keys(map, st, None, *addr, sz).ok_or(Errno::ENOMEM)?;
    } else if let Some(found) = (*addr != 0
        && *addr & PAGE_MASK == 0
        && map.is_vmspace()
        && (align == 0 || *addr & (align - 1) == 0))
        .then(|| isavail_keys(map, st, None, *addr, sz))
        .flatten()
    {
        // Address used as hint.
        //
        // Note: we enforce the alignment restriction, but ignore pmap_prefer.
        (first, last) = found;
    } else if prot & PROT_EXEC != 0 && st.uaddr_exe.is_some() {
        // Run selection algorithm for executables.
        let mut found =
            uvm_addr_invoke(map, st, Slot::Exe, sz, pmap_align, pmap_offset, prot, hint);

        // Grow kernel memory and try again.
        if found.is_err() && !map.is_vmspace() {
            uvm_map_kmem_grow(map, st, ew, dead, sz, flags);

            found = uvm_addr_invoke(map, st, Slot::Exe, sz, pmap_align, pmap_offset, prot, hint);
        }

        let (f, l, a) = found?;
        (first, last) = (f, l);
        *addr = a;
    } else {
        // Update freelists from vmspace.
        if map.is_vmspace() {
            uvm_map_vmspace_update(map, st, ew, dead, flags);
        }

        let mut found = uvm_map_findspace(map, st, sz, pmap_align, pmap_offset, prot, hint);

        // Grow kernel memory and try again.
        if found.is_err() && !map.is_vmspace() {
            uvm_map_kmem_grow(map, st, ew, dead, sz, flags);

            found = uvm_map_findspace(map, st, sz, pmap_align, pmap_offset, prot, hint);
        }

        let (f, l, a) = found?;
        (first, last) = (f, l);
        *addr = a;
    }

    // Double-check if selected address doesn't cause overflow.
    if addr.wrapping_add(sz) < *addr {
        return Err(Errno::ENOMEM);
    }

    kassert!(map.is_vmspace() || uvm_maxkaddr() >= *addr + sz);

    // If we only want a query, return now.
    if flags & UVM_FLAG_QUERY != 0 {
        return Ok(());
    }

    if uobj.is_none() {
        uoffset = 0;
    } else if uoffset == UVM_UNKNOWN_OFFSET {
        kassert!(uobj.as_ref().is_some_and(|o| o.is_kern_object()));
        uoffset = (*addr - kernel_map().min_offset()) as Voff;
    }

    // Create new entry. first and last may be invalidated after this call.
    let entry = uvm_map_mkentry(map, st, ew, first, last, *addr, sz, flags, dead, new.take())
        .ok_or(Errno::ENOMEM)?;
    kdassert!(entry.start() == *addr && entry.end() == *addr + sz);
    let key = entry.start();
    let has_uobj = uobj.is_some();
    entry.handles_mut(ew).object = uobj.map_or(EntryObject::None, EntryObject::Obj);
    entry.set_offset(ew, uoffset);
    entry.set_protection(ew, prot);
    entry.set_max_protection(ew, maxprot);
    entry.set_inheritance(ew, inherit);
    entry.set_wired_count(ew, 0);
    entry.set_advice(ew, advice);
    let mut etype = entry.etype();
    if flags & UVM_FLAG_STACK != 0 {
        etype |= UVM_ET_STACK;
        if flags & UVM_FLAG_UNMAP != 0 {
            map.sserial.fetch_add(1, Mem::Relaxed);
        }
    }
    if has_uobj {
        etype |= UVM_ET_OBJ;
    } else if flags & UVM_FLAG_HOLE != 0 {
        etype |= UVM_ET_HOLE;
    }
    if flags & UVM_FLAG_NOFAULT != 0 {
        etype |= UVM_ET_NOFAULT;
    }
    if flags & UVM_FLAG_WC != 0 {
        etype |= UVM_ET_WC;
    }
    if flags & UVM_FLAG_COPYONW != 0 {
        etype |= UVM_ET_COPYONWRITE;
        if flags & UVM_FLAG_OVERLAY == 0 {
            etype |= UVM_ET_NEEDSCOPY;
        }
    }
    if flags & UVM_FLAG_CONCEAL != 0 {
        etype |= UVM_ET_CONCEAL;
    }
    entry.set_etype(ew, etype);
    if flags & UVM_FLAG_OVERLAY != 0 {
        let h = entry.handles_mut(ew);
        h.pageoff = 0;
        h.amap = amap_alloc(sz, M_WAITOK, false);
    }

    // Update map and process statistics.
    if flags & UVM_FLAG_HOLE == 0 {
        map.size.fetch_add(sz, Mem::Relaxed);
        if map.is_vmspace() && !has_uobj && prot != PROT_NONE {
            map.vm()
                .dused_add(uvmspace_dused(map, *addr, *addr + sz) as isize);
        }
    }

    // Try to merge entry.
    //
    // Userland allocations are kept separated most of the time. Forego the effort of
    // merging what most of the time can't be merged and only try the merge if it concerns
    // a kernel entry.
    if flags & UVM_FLAG_NOMERGE == 0 && !map.is_vmspace() {
        uvm_mapent_tryjoin(map, st, ew, key, dead);
    }

    Ok(())
}

/// `uvm_mapent_isjoinable`: true iff e1 and e2 can be joined together (`w` reads their
/// handles).
fn uvm_mapent_isjoinable(e1: &VmMapEntry, e2: &VmMapEntry, w: &WitnessPart<'_>) -> bool {
    // Must be the same entry type and not have free memory between.
    if e1.etype() != e2.etype() || e1.end() != e2.start() {
        return false;
    }

    // Submaps are never joined.
    if uvm_et_issubmap(e1) {
        return false;
    }

    // Never merge wired memory.
    if vm_mapent_iswired(e1) || vm_mapent_iswired(e2) {
        return false;
    }

    // Protection, inheritance and advice must be equal.
    if e1.protection() != e2.protection()
        || e1.max_protection() != e2.max_protection()
        || e1.inheritance() != e2.inheritance()
        || e1.advice() != e2.advice()
    {
        return false;
    }

    let (h1, h2) = (e1.handles(w), e2.handles(w));

    // If uvm_object: object itself and offsets within object must match.
    if uvm_et_isobj(e1) {
        match (h1.uvm_obj(), h2.uvm_obj()) {
            (Some(o1), Some(o2)) if ObjRef::ptr_eq(o1, o2) => {}
            _ => return false,
        }
        if e1.offset() + (e1.end() - e1.start()) as Voff != e2.offset() {
            return false;
        }
    }

    // Cannot join shared amaps. Note: no need to lock amap to look at refs, since we
    // don't care about its exact value. If it is 1 (i.e. we have the only reference) it
    // will stay there.
    if h1.amap.as_ref().is_some_and(|a| a.refs() != 1) {
        return false;
    }
    if h2.amap.as_ref().is_some_and(|a| a.refs() != 1) {
        return false;
    }

    // Apparently, e1 and e2 match.
    true
}

/// `uvm_mapent_merge`: join support function: the entry at `k2` into the one at `k1`.
///
/// Returns the merged entry (its key, `k1`).
fn uvm_mapent_merge(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    k1: usize,
    k2: usize,
    dead: &mut DeadQueue,
) -> usize {
    let owners = st.owners(map);
    let free = {
        let mut fp = st.free_parts(owners);
        let e1 = ent(fp.tree, k1, "uvm_mapent_merge");
        // Merging is not supported for map entries that contain an amap in e1. This should
        // never happen anyway, because only kernel entries are merged. These do not contain
        // amaps. e2 contains no real information in its amap, so it can be erased
        // immediately.
        kassert!(e1.handles(ew.witness()).amap.is_none());

        // Don't drop obj reference: uvm_unmap_detach will do this for us.
        uvm_mapent_free_remove(&mut fp, ew, owners.uaddr_e(e1), e1);

        let e2 = ent(fp.tree, k2, "uvm_mapent_merge");
        let free = owners.uaddr_e(e2);
        uvm_mapent_free_remove(&mut fp, ew, free, e2);
        free
    };
    let e2 = uvm_mapent_addr_remove(st, k2);
    let mut fp = st.free_parts(owners);
    let e1 = ent(fp.tree, k1, "uvm_mapent_merge");
    e1.set_end(ew, e2.end());
    e1.set_guard(ew, e2.guard());
    e1.set_fspace(ew, e2.fspace());
    uvm_mapent_free_insert(&mut fp, ew, free, e1);

    dead.push(e2);
    k1
}

/// `uvm_mapent_tryjoin`: attempt forward and backward joining of the entry at `key`.
///
/// Returns the entry after joins (its key). We are guaranteed that the amap of entry is
/// either non-existent or has never been used.
fn uvm_mapent_tryjoin(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    dead: &mut DeadQueue,
) -> usize {
    let mut key = key;

    // Merge with previous entry.
    let other = {
        let e = ent(&st.addr, key, "uvm_mapent_tryjoin");
        st.addr
            .prev(e)
            .filter(|o| uvm_mapent_isjoinable(o, e, ew.witness()))
            .map(VmMapEntry::start)
    };
    if let Some(other) = other {
        key = uvm_mapent_merge(map, st, ew, other, key, dead);
    }

    // Merge with next entry.
    //
    // Because amap can only extend forward and the next entry probably contains sensible
    // info, only perform forward merging in the absence of an amap.
    let other = {
        let e = ent(&st.addr, key, "uvm_mapent_tryjoin");
        let w = ew.witness();
        st.addr
            .next(e)
            .filter(|o| {
                e.handles(w).amap.is_none()
                    && o.handles(w).amap.is_none()
                    && uvm_mapent_isjoinable(e, o, w)
            })
            .map(VmMapEntry::start)
    };
    if let Some(other) = other {
        key = uvm_mapent_merge(map, st, ew, key, other, dead);
    }

    key
}

/// `uvm_unmap_detach`: kill entries that are no longer in a map: drop their references to
/// their amap and their object (or submap), then free them.
pub fn uvm_unmap_detach(deadq: &mut DeadQueue, flags: i32) {
    while let Some(mut entry) = deadq.pop() {
        let slots = (entry.end() - entry.start()) >> PAGE_SHIFT;
        let issubmap = uvm_et_issubmap(&entry);
        let isobj = uvm_et_isobj(&entry);
        let h = entry.handles_owned();

        // Drop reference to amap, if we've got one.
        if let Some(amap) = &h.amap {
            amap_unref(amap, h.pageoff as usize, slots, flags & AMAP_REFALL != 0);
        }

        // Drop reference to our backing object, if we've got one.
        if issubmap {
            // ... unlikely to happen, but play it safe
            if let Some(submap) = h.sub_map() {
                uvm_map_deallocate(submap);
            }
        } else if isobj && let Some(uobj) = h.uvm_obj() {
            uobj.detach();
        }

        // uvm_mapent_free(entry): the box goes back to its pool.
        drop(entry);
    }
}

/// `uvm_unmap_detach_intrsafe`: as [`uvm_unmap_detach`] for an intrsafe map, whose entries
/// hold no amap or object.
pub fn uvm_unmap_detach_intrsafe(deadq: &mut DeadQueue) {
    while let Some(mut entry) = deadq.pop() {
        kassert!(!uvm_et_issubmap(&entry));
        kassert!(!uvm_et_isobj(&entry));
        kassert!(entry.handles_owned().amap.is_none());
        drop(entry);
    }
}

/// `uvm_map_mkentry`: create and insert new entry (`new` when the caller allocated it).
///
/// Returned entry contains new addresses and is inserted properly in the tree. first and
/// last (keys) are (probably) no longer valid.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uvm_map_mkentry<'s>(
    map: &VmMap,
    st: &'s mut MapState,
    ew: &mut EntryWrite<'_>,
    first: usize,
    last: usize,
    addr: usize,
    sz: usize,
    flags: u32,
    dead: &mut DeadQueue,
    new: Option<PoolBox<VmMapEntry>>,
) -> Option<&'s VmMapEntry> {
    kdassert!(sz > 0);
    kdassert!(addr + sz > addr);
    if cfg!(test) {
        let f = ent(&st.addr, first, "uvm_map_mkentry");
        let l = ent(&st.addr, last, "uvm_map_mkentry");
        kassert!(f.end() <= addr && vmmap_free_end(f) > addr);
        kassert!(l.start() < addr + sz && vmmap_free_end(l) >= addr + sz);
        kassert!(isavail_keys(map, st, None, addr, sz) == Some((first, last)));
        uvm_tree_sanity(map, st, "uvm_map_mkentry");
    }

    let min = addr + sz;
    let max = vmmap_free_end(ent(&st.addr, last, "uvm_map_mkentry"));

    // Initialize new entry.
    let mut entry = match new {
        None => uvm_mapent_alloc(map, flags)?,
        Some(new) => new,
    };
    entry.set_offset(ew, 0);
    entry.set_etype(ew, 0);
    entry.set_wired_count(ew, 0);
    entry.handles_owned().clear_aref();

    entry.set_start(ew, addr);
    entry.set_end(ew, min);
    entry.set_guard(ew, 0);
    entry.set_fspace(ew, 0);

    // Reset free space in first. Its start and the start of its free space stay as they
    // are below (only entries after it leave the tree).
    let owners = st.owners(map);
    let (first_start, first_free) = {
        let mut fp = st.free_parts(owners);
        let f = ent(fp.tree, first, "uvm_map_mkentry");
        uvm_mapent_free_remove(&mut fp, ew, owners.uaddr_e(f), f);
        f.set_guard(ew, 0);
        f.set_fspace(ew, 0);
        (f.start(), vmmap_free_start(f))
    };

    // Remove all entries that are fully replaced. We are iterating using last in reverse
    // order.
    let mut last = last;
    while first != last {
        let prev = {
            let mut fp = st.free_parts(owners);
            let l = ent(fp.tree, last, "uvm_map_mkentry");
            let prev = fp.tree.prev(l).map(VmMapEntry::start);
            kdassert!(l.start() == l.end());
            uvm_mapent_free_remove(&mut fp, ew, owners.uaddr_e(l), l);
            prev
        };
        dead.push(uvm_mapent_addr_remove(st, last));

        let Some(prev) = prev else {
            kpanic(format_args!("uvm_map_mkentry: ran past first"));
        };
        last = prev;
    }
    // Remove first if it is entirely inside <addr, addr+sz>.
    if first_start == addr {
        dead.push(uvm_mapent_addr_remove(st, first));
    } else {
        uvm_map_fix_space(map, st, ew, Some(first), first_free, addr, flags);
    }

    // Finally, link in entry.
    let key = uvm_mapent_addr_insert(map, st, entry);
    uvm_map_fix_space(map, st, ew, Some(key), min, max, flags);

    if cfg!(test) {
        uvm_tree_sanity(map, st, "uvm_map_mkentry");
    }
    Some(ent(&st.addr, key, "uvm_map_mkentry"))
}

/// `uvm_mapent_alloc`: allocate a map entry, its handles keyed to `map`'s lock: the static
/// entries for an `INTRSAFE` map or while `cold` (refilled from a fresh page when they run
/// out), `"vmmpekpl"` for the kernel map, `"vmmpepl"` otherwise (`PR_NOWAIT` with
/// `UVM_FLAG_TRYLOCK`).
pub fn uvm_mapent_alloc(map: &VmMap, flags: u32) -> Option<PoolBox<VmMapEntry>> {
    let pool_flags = if flags & UVM_FLAG_TRYLOCK != 0 {
        PR_NOWAIT
    } else {
        PR_WAITOK
    };
    let key = map.lock_id();

    if map.is_intrsafe() || cold() {
        let _m = UVM_KMAPENT_MTX.lock();
        let static_entry = || VmMapEntry::new(key, UVM_MAP_STATIC);
        let me = match KENTRY_POOL.get(PR_NOWAIT, static_entry) {
            Some(me) => me,
            None => {
                let Some(page) = KmBuf::alloc_wired(&KENTRY_PAGES, PAGE_SIZE, false) else {
                    kpanic(format_args!("uvm_mapent_alloc: cannot allocate map entry"));
                };
                KENTRY_ARENA.extend(page);
                if ratecheck(&UVM_KMAPENT_LAST_WARN_TIME, UVM_KMAPENT_WARN_RATE) {
                    hal::prf::printf(format_args!(
                        "uvm_mapent_alloc: out of static map entries\n"
                    ));
                }
                match KENTRY_POOL.get(PR_NOWAIT, static_entry) {
                    Some(me) => me,
                    None => kpanic(format_args!("uvm_mapent_alloc: kentry_free is empty")),
                }
            }
        };
        UVMEXP.kmapent.fetch_add(1, Mem::Relaxed);
        Some(me)
    } else if is_kernel_map(map) {
        splassert(IPL_NONE, "uvm_mapent_alloc");
        KMEM_ENTRY_POOL.get(pool_flags, || VmMapEntry::new(key, UVM_MAP_KMEM))
    } else {
        splassert(IPL_NONE, "uvm_mapent_alloc");
        ENTRY_POOL.get(pool_flags, || VmMapEntry::new(key, 0))
    }
}

/// `uvm_map_lookup_entry`: find map entry at or before an address.
///
/// - map must at least be read-locked by caller (its tree is given)
/// - the entry is returned when the address is in it; `ET_HOLE` entries are considered to
///   not contain a mapping, ergo `None` is returned for those mappings.
pub fn uvm_map_lookup_entry(atree: &OwnedTree<ByAddr>, address: usize) -> Option<&VmMapEntry> {
    uvm_map_entrybyaddr(atree, address)
        .filter(|entry| !uvm_et_ishole(entry) && entry.start() <= address && entry.end() > address)
}

/// `uvm_map_inentry_sp`: stack must be in a `MAP_STACK` entry. `PROT_NONE` indicates stack
/// not yet grown -- then `uvm_map_check_region_range()` should not cache the entry because
/// growth won't be seen.
pub fn uvm_map_inentry_sp(entry: &VmMapEntry) -> i32 {
    if entry.etype() & UVM_ET_STACK == 0 {
        if entry.protection() == PROT_NONE {
            return -1; // don't update range
        }
        return 0;
    }
    1
}

/// `uvm_map_inentry_recheck`.
pub fn uvm_map_inentry_recheck(serial: u64, addr: usize, ie: &InentryCache) -> bool {
    serial != ie.serial || ie.start == 0 || addr < ie.start || addr >= ie.end
}

/// `uvm_map_inentry_fix`: inside a vm_map find the reg address and verify it via function.
/// Remember low and high addresses of region if valid and return true, else return false.
pub fn uvm_map_inentry_fix(
    map: &VmMap,
    ie: &mut InentryCache,
    addr: usize,
    f: fn(&VmMapEntry) -> i32,
    serial: u64,
) -> bool {
    if addr < map.min_offset() || addr >= map.max_offset() {
        return false;
    }

    // lock map
    let g = map.lock_read();

    // lookup
    let Some(entry) = g.lookup(trunc_page(addr)) else {
        return false;
    };

    match f(entry) {
        0 => return false,
        1 => {
            ie.start = entry.start();
            ie.end = entry.end();
            ie.serial = serial;
        }
        _ => {
            // do not update, re-check later
        }
    }
    true
}

/// `uvm_map_inentry`: checks that `addr` (the `what` register: "sp", "pc") lies in an entry
/// of `map`, `curproc`'s, that `f` accepts, caching the result in `ie`; on failure prints the
/// C's message with `reason` and signals the thread (see the module's deviations).
pub fn uvm_map_inentry(
    map: &VmMap,
    ie: &mut InentryCache,
    addr: usize,
    what: &str,
    reason: &str,
    f: fn(&VmMapEntry) -> i32,
    serial: u64,
) -> bool {
    let mut ok = true;

    if uvm_map_inentry_recheck(serial, addr, ie) {
        ok = uvm_map_inentry_fix(map, ie, addr, f, serial);
        if !ok {
            hal::klock::kernel_lock();
            let p = vmproc();
            let mut comm = [0u8; 24];
            let n = p.comm(&mut comm).min(comm.len());
            hal::prf::printf(format_args!(
                "[{}]{}/{} {}={:#x} inside {:#x}-{:#x}: {}\n",
                core::str::from_utf8(&comm[..n]).unwrap_or("?"),
                p.pid(),
                p.tid(),
                what,
                addr,
                ie.start,
                ie.end.wrapping_sub(1),
                reason
            ));
            p.set_acflag_amap();
            p.trapsignal_segv(p.pc());
            hal::klock::kernel_unlock();
        }
    }
    ok
}

/// `uvm_map_is_stack_remappable`: check whether the given address range can be converted
/// to a `MAP_STACK` mapping.
///
/// Must be called with map locked (its tree is given).
pub fn uvm_map_is_stack_remappable(
    atree: &OwnedTree<ByAddr>,
    addr: usize,
    sz: usize,
    sigaltstack_check: bool,
) -> bool {
    let end = addr + sz;

    let Some(first) = uvm_map_lookup_entry(atree, addr) else {
        return false;
    };

    // Check that the address range exists and is contiguous.
    let mut prev: Option<&VmMapEntry> = None;
    let mut iter = Some(first);
    while let Some(entry) = iter.filter(|e| e.start() < end) {
        // Make sure that we do not have holes in the range.
        if prev.is_some_and(|p| p.end() != entry.start()) {
            return false;
        }
        if entry.start() == entry.end() || uvm_et_ishole(entry) {
            return false;
        }
        if sigaltstack_check && entry.protection() != (PROT_READ | PROT_WRITE) {
            return false;
        }
        prev = Some(entry);
        iter = atree.next(entry);
    }

    true
}

/// `uvm_map_remap_as_stack`: remap the middle-pages of an existing mapping as a stack
/// range. If there exists a previous contiguous mapping with the given range
/// `[addr, addr + sz)`, with protection `PROT_READ|PROT_WRITE`, then the mapping is
/// dropped, and a new anon mapping is created and marked as a stack.
///
/// Must be called with map (`curproc`'s vmspace's) unlocked.
pub fn uvm_map_remap_as_stack(map: &VmMap, addr: usize, sz: usize) -> Result<(), Errno> {
    let flags = uvm_mapflag(
        PROT_READ | PROT_WRITE,
        PROT_READ | PROT_WRITE | PROT_EXEC,
        MAP_INHERIT_COPY,
        MADV_NORMAL,
        UVM_FLAG_STACK | UVM_FLAG_FIXED | UVM_FLAG_UNMAP | UVM_FLAG_COPYONW | UVM_FLAG_SIGALTSTACK,
    );

    let mut start = round_page(addr);
    let end = trunc_page(addr + sz);
    // MACHINE_STACK_GROWS_UP is not defined.
    if start == addr {
        start += PAGE_SIZE;
    }

    if start < map.min_offset() || end >= map.max_offset() || end < start {
        return Err(Errno::EINVAL);
    }

    // UVM_FLAG_SIGALTSTACK indicates that immutable may be bypassed, but the range is
    // checked that it is contiguous, is not a syscall mapping, and protection RW. Then, a
    // new mapping (all zero) is placed upon the region, which prevents an attacker from
    // pivoting into pre-placed MAP_STACK space.
    let len = end - start;
    uvm_mapanon(map, &mut start, len, 0, flags)
}

/// `uvm_map_pie`: return a random load address for a PIE executable properly aligned.
pub fn uvm_map_pie(align: usize) -> usize {
    let align = align.max(VM_PIE_MIN_ALIGN);

    // round up to next alignment
    let min = (VM_PIE_MIN_ADDR + align - 1) & !(align - 1);

    if align >= VM_PIE_MAX_ADDR || min >= VM_PIE_MAX_ADDR {
        return align;
    }

    let space = ((VM_PIE_MAX_ADDR - min) / align).min(u32::MAX as usize);

    let addr = arc4random_uniform(space as u32) as usize * align;
    addr + min
}

/// `uvm_unmap`: removes the mappings in `[start, end)` from the map.
pub fn uvm_unmap(map: &VmMap, start: usize, end: usize) {
    kassert!(start & PAGE_MASK == 0 && end & PAGE_MASK == 0);
    let mut dead = DeadQueue::new();
    let mut g = map.lock();
    {
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        let _ = unmap_remove(map, st, ew, start, end, &mut dead, false, true, false);
    }
    drop(g);

    if map.is_intrsafe() {
        uvm_unmap_detach_intrsafe(&mut dead);
    } else {
        uvm_unmap_detach(&mut dead, 0);
    }
}

/// `uvm_mapent_mkfree`: mark the entry at `key` as free.
///
/// entry will be put on the dead list. The free space will be merged into the previous or
/// a new entry, unless markfree is false.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uvm_mapent_mkfree(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    prev_ptr: &mut Option<usize>,
    dead: &mut DeadQueue,
    markfree: bool,
) {
    let mut prev = *prev_ptr;
    if prev == Some(key) {
        *prev_ptr = None;
        prev = None;
    }

    let owners = st.owners(map);
    let (addr, end) = {
        let mut fp = st.free_parts(owners);
        let tree = fp.tree;
        let entry = ent(tree, key, "uvm_mapent_mkfree");
        if prev.is_none_or(|p| vmmap_free_end(ent(tree, p, "uvm_mapent_mkfree")) != entry.start()) {
            prev = tree.prev(entry).map(VmMapEntry::start);
        }

        // Entry is describing only free memory and has nothing to drain into.
        if prev.is_none() && entry.start() == entry.end() && markfree {
            *prev_ptr = Some(key);
            return;
        }

        let range = (entry.start(), vmmap_free_end(entry));
        uvm_mapent_free_remove(&mut fp, ew, owners.uaddr_e(entry), entry);
        range
    };
    dead.push(uvm_mapent_addr_remove(st, key));

    if markfree {
        if let Some(p) = prev {
            let mut fp = st.free_parts(owners);
            let pe = ent(fp.tree, p, "uvm_mapent_mkfree");
            uvm_mapent_free_remove(&mut fp, ew, owners.uaddr_e(pe), pe);
        }
        *prev_ptr = uvm_map_fix_space(map, st, ew, prev, addr, end, 0);
    }
}

/// `uvm_unmap_kill_entry_withlock`: unwire and release referenced amap and object from the
/// map entry `entry` of `st`'s tree (the entry's amap and object locked when `needlock`).
fn uvm_unmap_kill_entry_withlock(
    map: &VmMap,
    st: &MapState,
    ew: &mut EntryWrite<'_>,
    entry: &VmMapEntry,
    needlock: bool,
) {
    let (start, end) = (entry.start(), entry.end());

    // Unwire removed map entry.
    if vm_mapent_iswired(entry) {
        entry.set_wired_count(ew, 0);
        unwire_held(map, st, ew, start, end);
    }

    let h = entry.handles(ew.witness()).clone();
    let mut locks = needlock.then(|| uvm_map_lock_entry(&h));

    // Entry-type specific code.
    if uvm_et_ishole(entry) {
        // Nothing to be done for holes.
    } else if map.is_intrsafe() {
        kassert!(map.pmap().is_kernel());

        uvm_km_pgremove_intrsafe(Vaddr::new(start), Vaddr::new(end));
    } else if let Some(uobj) = h.uvm_obj().filter(|uobj| uobj.is_kern_object()) {
        kassert!(map.pmap().is_kernel());
        // Note: kernel object mappings are currently used in two ways:
        //  [1] "normal" mappings of pages in the kernel object
        //  [2] uvm_km_valloc'd allocations in which we pmap_enter in some
        //      non-kernel-object page (e.g. vmapbuf).
        //
        // for case [1], we need to remove the mapping from the pmap and then remove the
        // page from the kernel object (because, once pages in a kernel object are unmapped
        // they are no longer needed, unlike, say, a vnode where you might want the data to
        // persist until flushed out of a queue).
        //
        // for case [2], we need to remove the mapping from the pmap. there shouldn't be
        // any pages at the specified offset in the kernel object [but it doesn't hurt to
        // call uvm_km_pgremove just to be safe?]
        //
        // uvm_km_pgremove currently does the following: for pages in the kernel object
        // range: drops the swap slot, uvm_pagefree the page.
        //
        // note there is version of uvm_km_pgremove() that is used for "intrsafe" objects.
        //
        // remove mappings from pmap and drop the pages from the object. offsets are always
        // relative to vm_map_min(kernel_map). The object's lock is the entry's (needlock); a
        // caller that did not ask for it holds none (uvm_km_pgremove's KASSERT), so it is
        // taken here (only the kernel map has kernel object entries, and it is killed with
        // needlock).
        let (s, e) = (Vaddr::new(start), Vaddr::new(end));
        match locks.as_mut().and_then(EntryLocks::lower_mut) {
            Some(w) => uvm_km_pgremove(uobj, w, s, e),
            None => uvm_km_pgremove(uobj, &mut uobj.lock().write(), s, e),
        }
    } else {
        // remove mappings the standard way.
        map_pmap_remove(map, start, end);
    }

    // uvm_map_unlock_entry(entry)
    drop(locks);
}

/// `uvm_unmap_remove`: remove all entries from start to end of the map `g` write-locks
/// (the dead entries onto `dead`, for [`uvm_unmap_detach`] once unlocked).
///
/// If remove_holes, then remove `ET_HOLE` entries as well. If markfree, entry will be
/// properly marked free, otherwise, no replacement entry will be put in the tree
/// (corrupting the tree). `EPERM` when checkimmutable finds an immutable entry.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_unmap_remove(
    g: &mut MapWriteGuard<'_>,
    start: usize,
    end: usize,
    dead: &mut DeadQueue,
    remove_holes: bool,
    markfree: bool,
    checkimmutable: bool,
) -> Result<(), Errno> {
    let map = g.map();
    let mut sp = g.split();
    let (st, ew) = sp.parts();
    unmap_remove(
        map,
        st,
        ew,
        start,
        end,
        dead,
        remove_holes,
        markfree,
        checkimmutable,
    )
}

/// [`uvm_unmap_remove`] over the map's split write guard.
#[allow(clippy::too_many_arguments)] // the C's signature
fn unmap_remove(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    start: usize,
    end: usize,
    dead: &mut DeadQueue,
    remove_holes: bool,
    markfree: bool,
    checkimmutable: bool,
) -> Result<(), Errno> {
    let start = start.max(map.min_offset());
    let end = end.min(map.max_offset());
    if start >= end {
        return Ok(());
    }

    // Find first affected entry.
    let entry = entry_at(map, &st.addr, start, "uvm_unmap_remove");
    kdassert!(entry.start() <= start);
    let ekey = entry.start();

    if checkimmutable {
        // Refuse to unmap if any entries are immutable
        let mut entry1 = if entry.end() <= start {
            st.addr.next(entry)
        } else {
            Some(entry)
        };
        while let Some(e1) = entry1.filter(|e| e.start() < end) {
            kdassert!(e1.start() >= start);
            // Treat memory holes as free space.
            if !(e1.start() == e1.end() || uvm_et_ishole(e1)) && e1.etype() & UVM_ET_IMMUTABLE != 0
            {
                return Err(Errno::EPERM);
            }
            entry1 = st.addr.next(e1);
        }
    }

    let mut iter = if entry.end() <= start && markfree {
        st.addr.next(entry).map(VmMapEntry::start)
    } else {
        Some(uvm_map_clip_start_at(map, st, ew, ekey, start))
    };

    // Iterate entries until we reach end address. prev_hint hints where the freed space
    // can be appended to. Each step finds its entry once and holds it until the entry
    // leaves the tree (a clip changes the tree: the entry is found again after it).
    let mut prev_hint: Option<usize> = None;
    while let Some(key) = iter {
        let e = ent(&st.addr, key, "uvm_unmap_remove");
        if e.start() >= end {
            break;
        }
        kdassert!(e.start() >= start);
        // `if (entry->end > end || !markfree) UVM_MAP_CLIP_END(map, entry, end)`: the clip
        // splits only an entry that reaches past end (its start is below end, as the loop
        // checked), so without markfree an entry inside the range stays as it is.
        let e = if e.end() > end {
            uvm_map_clip_end(map, st, ew, key, end);
            ent(&st.addr, key, "uvm_unmap_remove")
        } else {
            e
        };
        kdassert!(e.start() >= start && e.end() <= end);
        let next = st.addr.next(e).map(VmMapEntry::start);

        // Don't remove holes unless asked to do so.
        if uvm_et_ishole(e) && !remove_holes {
            prev_hint = Some(key);
            iter = next;
            continue;
        }

        // A stack has been removed..
        if uvm_et_isstack(e) && map.is_vmspace() {
            map.sserial.fetch_add(1, Mem::Relaxed);
        }

        // Kill entry (its place in the tree stays).
        uvm_unmap_kill_entry_withlock(map, st, ew, e, true);

        // Update space usage.
        let (es, ee, hole) = (e.start(), e.end(), uvm_et_ishole(e));
        if map.is_vmspace()
            && e.handles(ew.witness()).uvm_obj().is_none()
            && e.protection() != PROT_NONE
            && !hole
        {
            map.vm().dused_add(-(uvmspace_dused(map, es, ee) as isize));
        }
        if !hole {
            map.size.fetch_sub(ee - es, Mem::Relaxed);
        }

        // Actual removal of entry.
        uvm_mapent_mkfree(map, st, ew, key, &mut prev_hint, dead, markfree);
        iter = next;
    }

    // pmap_update(map->pmap): nothing to do (hal's pmap does its TLB work in each call).

    if cfg!(test) {
        if markfree {
            let mut iter = uvm_map_entrybyaddr(&st.addr, start);
            while let Some(entry) = iter.filter(|e| e.start() < end) {
                kassert!(
                    entry.end() <= start || entry.start() == entry.end() || uvm_et_ishole(entry)
                );
                iter = st.addr.next(entry);
            }
        } else {
            let mut a = start;
            while a < end {
                kassert!(uvm_map_entrybyaddr(&st.addr, a).is_none());
                a += PAGE_SIZE;
            }
        }
    }
    Ok(())
}

/// `uvm_map_pageable_pgon`: mark all entries from `first` until `end` (exclusive, `None`:
/// to the last) as pageable.
///
/// Lock must be exclusive on entry and will not be touched.
fn uvm_map_pageable_pgon(
    map: &VmMap,
    st: &MapState,
    ew: &mut EntryWrite<'_>,
    first: Option<usize>,
    end: Option<usize>,
    start_addr: usize,
    end_addr: usize,
) {
    let mut iter = first;
    while let Some(key) = iter.filter(|k| Some(*k) != end) {
        let e = ent(&st.addr, key, "uvm_map_pageable_pgon");
        kdassert!(e.start() >= start_addr && e.end() <= end_addr);
        if vm_mapent_iswired(e) && !uvm_et_ishole(e) {
            e.set_wired_count(ew, 0);
            unwire_held(map, st, ew, e.start(), e.end());
        }
        iter = next_key(&st.addr, key);
    }
}

/// The entries `uvm_map_pageable_wire` leaves alone: holes, free-only entries and
/// `PROT_NONE` ones.
fn wire_skips(e: &VmMapEntry) -> bool {
    uvm_et_ishole(e) || e.start() == e.end() || e.protection() == PROT_NONE
}

/// `uvm_map_pageable_wire`: mark all entries from `first` until `end` (exclusive, `None`:
/// to the last) as wired.
///
/// The write lock `g` is the caller's; it comes back when `keep` (`UVM_LK_EXIT`), else it is
/// released on every path.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uvm_map_pageable_wire<'a>(
    map: &'a VmMap,
    mut g: MapWriteGuard<'a>,
    first: usize,
    end: Option<usize>,
    start_addr: usize,
    end_addr: usize,
    keep: bool,
) -> (Option<MapWriteGuard<'a>>, Result<(), Errno>) {
    // Wire pages in two passes:
    //
    // 1: holding the write lock, we create any anonymous maps that need to be created.
    //    then we clip each map entry to the region to be wired and increment its wiring
    //    count.
    //
    // 2: we mark the map busy, unlock it and call uvm_fault_wire to fault in the pages for
    //    any newly wired area (wired_count == 1).
    let mut first = first;
    {
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        let mut iter = Some(first);
        while let Some(key) = iter.filter(|k| Some(*k) != end) {
            let e = ent(&st.addr, key, "uvm_map_pageable_wire");
            kdassert!(e.start() >= start_addr && e.end() <= end_addr);
            let mut key = key;
            if !wire_skips(e) {
                // Perform actions of vm_map_lookup that need the write lock: create an
                // anonymous map for copy-on-write, anonymous map for zero-fill. Skip
                // submaps.
                if !vm_mapent_iswired(e)
                    && !uvm_et_issubmap(e)
                    && uvm_et_isneedscopy(e)
                    && (e.protection() & PROT_WRITE != 0
                        || e.handles(ew.witness()).uvm_obj().is_none())
                {
                    let (s, x, stack) = (e.start(), e.end(), uvm_et_isstack(e));
                    let nk = amap_copy(map, st, ew, key, M_WAITOK, !stack, s, x);
                    if key == first {
                        first = nk;
                    }
                    key = nk;
                }
                let e = ent(&st.addr, key, "uvm_map_pageable_wire");
                e.set_wired_count(ew, e.wired_count() + 1);
            }
            iter = next_key(&st.addr, key);
        }
    }

    // Pass 2.
    let timestamp_save = map.timestamp();
    let busy = g.into_busy();

    let mut error = Ok(());
    let mut failed_at = None;
    let mut iter = Some(first);
    loop {
        // The entry's fields under a short read lock (the map is busy: writers wait).
        let step = {
            let r = busy.lock_read();
            let st = r.state();
            iter.filter(|k| Some(*k) != end).map(|key| {
                let e = ent(&st.addr, key, "uvm_map_pageable_wire");
                let wire = !wire_skips(e) && e.wired_count() == 1;
                (
                    key,
                    wire,
                    e.start(),
                    e.end(),
                    e.protection(),
                    st.addr.next(e).map(VmMapEntry::start),
                )
            })
        };
        let Some((key, wire, s, x, prot, next)) = step else {
            break;
        };
        if wire {
            error = uvm_fault_wire(map, s, x, prot);
            if error.is_err() {
                failed_at = Some(key);
                break;
            }
        }
        iter = next;
    }

    let mut g = busy.relock_unbusy();

    if error.is_err() {
        if cfg!(feature = "diagnostic") && timestamp_save != map.timestamp() {
            kpanic(format_args!("uvm_map_pageable_wire: stale map"));
        }

        {
            let mut sp = g.split();
            let (st, ew) = sp.parts();
            // first is no longer needed to restart loops. Use it as iterator to unwire
            // entries that were successfully wired above.
            let mut iter = Some(first);
            while let Some(key) = iter.filter(|k| Some(*k) != failed_at) {
                let e = ent(&st.addr, key, "uvm_map_pageable_wire");
                if !wire_skips(e) {
                    e.set_wired_count(ew, e.wired_count() - 1);
                    if !vm_mapent_iswired(e) {
                        unwire_held(map, st, ew, e.start(), e.end());
                    }
                }
                iter = next_key(&st.addr, key);
            }

            // decrease counter in the rest of the entries
            let mut iter = failed_at;
            while let Some(key) = iter.filter(|k| Some(*k) != end) {
                let e = ent(&st.addr, key, "uvm_map_pageable_wire");
                if !wire_skips(e) {
                    e.set_wired_count(ew, e.wired_count() - 1);
                }
                iter = next_key(&st.addr, key);
            }
        }

        return (keep.then_some(g), error);
    }

    if keep {
        if cfg!(feature = "diagnostic") && timestamp_save != map.timestamp() {
            kpanic(format_args!("uvm_map_pageable_wire: stale map"));
        }
        return (Some(g), Ok(()));
    }
    (None, Ok(()))
}

/// `uvm_map_pageable`: set pageability of a range in a map (the map unlocked: it is locked
/// and released here).
///
/// The full range must be in use (entries may not have fspace != 0). `UVM_ET_HOLE` counts
/// as unmapped.
pub fn uvm_map_pageable(
    map: &VmMap,
    start: usize,
    end: usize,
    new_pageable: bool,
) -> Result<(), Errno> {
    let (start, end) = (trunc_page(start), round_page(end));
    pageable_checks(map, start, end)?;
    if start == end {
        return Ok(()); // nothing to do
    }
    uvm_map_pageable_core(map.lock(), start, end, new_pageable, false).1
}

/// `uvm_map_pageable(map, start, end, new_pageable, UVM_LK_ENTER)`: the caller's write lock
/// `g`, released on every path (`uvm_mmaplock`).
pub fn uvm_map_pageable_locked(
    g: MapWriteGuard<'_>,
    start: usize,
    end: usize,
    new_pageable: bool,
) -> Result<(), Errno> {
    let (start, end) = (trunc_page(start), round_page(end));
    pageable_checks(g.map(), start, end)?;
    if start == end {
        return Ok(()); // nothing to do
    }
    uvm_map_pageable_core(g, start, end, new_pageable, false).1
}

/// `uvm_map_pageable(map, start, end, new_pageable, UVM_LK_ENTER | UVM_LK_EXIT)`: the
/// caller's write lock `g`, given back (`uvm_map_protect`'s `VM_MAP_WIREFUTURE` path).
pub fn uvm_map_pageable_keep(
    g: MapWriteGuard<'_>,
    start: usize,
    end: usize,
    new_pageable: bool,
) -> (MapWriteGuard<'_>, Result<(), Errno>) {
    let (start, end) = (trunc_page(start), round_page(end));
    if let Err(e) = pageable_checks(g.map(), start, end) {
        return (g, Err(e));
    }
    if start == end {
        return (g, Ok(())); // nothing to do
    }
    let map = g.map();
    match uvm_map_pageable_core(g, start, end, new_pageable, true) {
        (Some(g), r) => (g, r),
        (None, _) => kpanic(format_args!(
            "uvm_map_pageable: map {:#x}: UVM_LK_EXIT lost the lock",
            map.addr()
        )),
    }
}

/// `uvm_map_pageable`'s checks before it locks.
fn pageable_checks(map: &VmMap, start: usize, end: usize) -> Result<(), Errno> {
    if start > end {
        return Err(Errno::EINVAL);
    }
    if start == end {
        return Ok(());
    }
    if start < map.min_offset() {
        return Err(Errno::EFAULT); // why? see first XXX below
    }
    if end > map.max_offset() {
        return Err(Errno::EINVAL); // why? see second XXX below
    }
    kassert!(map.is_pageable());
    Ok(())
}

/// [`uvm_map_pageable`] with the map write-locked by `g`, given back when `keep`.
fn uvm_map_pageable_core<'a>(
    mut g: MapWriteGuard<'a>,
    start: usize,
    end: usize,
    new_pageable: bool,
    keep: bool,
) -> (Option<MapWriteGuard<'a>>, Result<(), Errno>) {
    let map = g.map();

    // Find first entry.
    //
    // Initial test on start is different, because of the different error returned. Rest
    // is tested further down.
    let found = {
        let st = g.state();
        let first = entry_at(map, &st.addr, start, "uvm_map_pageable");
        if first.end() <= start || uvm_et_ishole(first) {
            // XXX if the first address is not mapped, it is EFAULT?
            Err(Errno::EFAULT)
        } else {
            // Check that the range has no holes.
            let mut last = Some(first);
            let mut bad = false;
            while let Some(l) = last.filter(|l| l.start() < end) {
                if uvm_et_ishole(l) || (l.end() < end && vmmap_free_end(l) != l.end()) {
                    // XXX unmapped memory in range, why is it EINVAL instead of EFAULT?
                    bad = true;
                    break;
                }
                last = st.addr.next(l);
            }

            // Last ended at the first entry after the range. Move back one step.
            //
            // Note that last may be NULL.
            if bad {
                Err(Errno::EINVAL)
            } else {
                match last {
                    None => match st.addr.max() {
                        Some(max) if max.end() >= end => Ok((first.start(), max.start())),
                        _ => Err(Errno::EINVAL),
                    },
                    Some(l) => {
                        kassert!(!ptr::eq(l, first));
                        match st.addr.prev(l) {
                            Some(prev) => Ok((first.start(), prev.start())),
                            None => Err(Errno::EINVAL),
                        }
                    }
                }
            }
        }
    };
    let (first, last) = match found {
        Ok(f) => f,
        Err(e) => return (keep.then_some(g), Err(e)),
    };

    // Wire/unwire pages here.
    let (first, tmp) = {
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        let wired = |k: usize| vm_mapent_iswired(ent(&st.addr, k, "uvm_map_pageable"));
        // Mark pageable: entries that are not wired are untouched. Mark wired: entries are
        // always touched (because recovery needs this).
        let mut last = last;
        let mut first = first;
        if wired(first) == new_pageable {
            let nf = uvm_map_clip_start_at(map, st, ew, first, start);
            if last == first {
                last = nf;
            }
            first = nf;
        }
        // Split last at end. Make tmp be the first entry after what is to be touched. If
        // last is not wired (pageable: is wired), don't touch it.
        let tmp = if vm_mapent_iswired(ent(&st.addr, last, "uvm_map_pageable")) == new_pageable {
            uvm_map_clip_end_at(map, st, ew, last, end);
            next_key(&st.addr, last)
        } else {
            Some(last)
        };
        if new_pageable {
            uvm_map_pageable_pgon(map, st, ew, Some(first), tmp, start, end);
        }
        (first, tmp)
    };

    if new_pageable {
        return (keep.then_some(g), Ok(()));
    }
    uvm_map_pageable_wire(map, g, first, tmp, start, end, keep)
}

/// `uvm_map_pageable_all`: special case of `uvm_map_pageable` - affects all mapped
/// regions.
///
/// Map must not be locked. If no flags are specified, all regions are unwired.
pub fn uvm_map_pageable_all(map: &VmMap, flags: i32, limit: usize) -> Result<(), Errno> {
    kassert!(map.is_pageable());
    let mut g = map.lock();

    if flags == 0 {
        {
            let mut sp = g.split();
            let (st, ew) = sp.parts();
            let first = st.addr.min().map(VmMapEntry::start);
            uvm_map_pageable_pgon(map, st, ew, first, None, map.min_offset(), map.max_offset());
        }
        map.modflags(0, VM_MAP_WIREFUTURE);
        return Ok(());
    }

    if flags & MCL_FUTURE != 0 {
        map.modflags(VM_MAP_WIREFUTURE, 0);
    }
    if flags & MCL_CURRENT == 0 {
        return Ok(());
    }

    // Count number of pages in all non-wired entries. If the number exceeds the limit,
    // abort.
    let mut size = 0;
    for iter in g.state().addr.iter() {
        if vm_mapent_iswired(iter) || uvm_et_ishole(iter) {
            continue;
        }

        size += iter.end() - iter.start();
    }

    if atop(size) as i64 + i64::from(UVMEXP.wired.load(Mem::Relaxed))
        > i64::from(UVMEXP.wiredmax.load(Mem::Relaxed))
    {
        return Err(Errno::ENOMEM);
    }

    // XXX non-pmap_wired_count case must be handled by caller (pmap_wired_count is defined
    // on amd64 and arm64).
    let wired = usize::try_from(map.pmap().wired_count()).unwrap_or(0);
    if limit != 0 && size + ptoa(wired) > limit {
        return Err(Errno::ENOMEM);
    }

    // uvm_map_pageable_wire will release lock
    let Some(first) = g.state().addr.min().map(VmMapEntry::start) else {
        return Ok(());
    };
    uvm_map_pageable_wire(
        map,
        g,
        first,
        None,
        map.min_offset(),
        map.max_offset(),
        false,
    )
    .1
}

/// `uvm_map_setup`: initialize map, where it stays (a static, a leaked block, a vmspace's
/// pool item): its lock keys the state here.
///
/// Allocates sufficient entries to describe the free memory in the map.
pub fn uvm_map_setup(map: &VmMap, pmap: PmapRef, min: usize, max: usize, flags: i32) {
    let mut max = max;

    kassert!(min & PAGE_MASK == 0);
    kassert!(max & PAGE_MASK == 0 || max & PAGE_MASK == PAGE_MASK);

    // Update parameters.
    //
    // This code handles (vaddr_t)-1 and other page mask ending addresses properly. We lose
    // the top page if the full virtual address space is used.
    if max & PAGE_MASK != 0 {
        max = max.wrapping_add(1);
        if max == 0 {
            // overflow
            max -= PAGE_SIZE;
        }
    }

    // The lock (rw_init "vmmaplk" with RWL_DUPOK, or "kmmaplk"; the INTRSAFE map's mutex)
    // was named when the map was made; the flags choose which one guards it.
    map.flags.store(flags, Mem::Relaxed);
    if map.pmap.set(pmap).is_err() {
        kpanic(format_args!(
            "uvm_map_setup: map {:#x} set up twice",
            map.addr()
        ));
    }
    map.size.store(0, Mem::Relaxed);
    map.ref_count.store(0, Mem::Relaxed);
    map.min_offset.store(min, Mem::Relaxed);
    map.max_offset.store(max, Mem::Relaxed);

    // Configure the allocators. Empty brk() and stack areas by default.
    let mut state = MapState::new();
    if flags & VM_MAP_ISVMSPACE != 0 {
        uvm_map_setup_md(&mut state, min, max);
    } else {
        state.uaddr_any[3] = Some(uaddr_kbootstrap());
    }
    if map
        .state
        .set(Guarded::keyed(map.lock_id(), STATE_PART, state))
        .is_err()
    {
        kpanic(format_args!(
            "uvm_map_setup: map {:#x} set up twice",
            map.addr()
        ));
    }

    // Fill map entries. Only the current thread sees the map now; the write lock is the
    // entries' proof.
    {
        let mut g = map.lock();
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        uvm_map_setup_entries(map, st, ew);
        if cfg!(test) {
            uvm_tree_sanity(map, st, "uvm_map_setup");
        }
    }
    map.timestamp.store(0, Mem::Relaxed);
    map.ref_count.store(1, Mem::Relaxed);
}

/// `uvm_map_teardown`: destroy the map.
///
/// This is the inverse operation to `uvm_map_setup`.
pub fn uvm_map_teardown(map: &VmMap) {
    kassert!(!map.is_intrsafe());

    let mut dead = DeadQueue::new();
    let mut g = map.lock();
    let selectors = {
        let mut sp = g.split();
        let (st, ew) = sp.parts();

        // Remove entries: every entry is killed, then the whole tree moves onto the dead
        // queue in linear time (the C's breadth-first walk; the order is not observable).
        // Killing an entry leaves the tree as it is: the walk follows the links.
        let mut iter = st.addr.min();
        while let Some(e) = iter {
            sched_pause();
            uvm_unmap_kill_entry_withlock(map, st, ew, e, false);
            iter = st.addr.next(e);
        }
        st.addr.drain_into(&mut dead);

        // Remove address selectors (after the unlock, as the C).
        let mut any = [None, None, None, None];
        for (slot, sel) in any.iter_mut().zip(st.uaddr_any.iter_mut()) {
            *slot = sel.take();
        }
        (st.uaddr_exe.take(), any, st.uaddr_brk_stack.take())
    };
    drop(g);

    let (exe, any, brk_stack) = selectors;
    uvm_addr_destroy(exe);
    for sel in any {
        uvm_addr_destroy(sel);
    }
    uvm_addr_destroy(brk_stack);

    uvm_unmap_detach(&mut dead, 0);
}

/// `uvm_map_setup_entries`: populate map with free-memory entries.
///
/// Map must be initialized and empty.
pub fn uvm_map_setup_entries(map: &VmMap, st: &mut MapState, ew: &mut EntryWrite<'_>) {
    kdassert!(st.addr.is_empty());

    uvm_map_fix_space(map, st, ew, None, map.min_offset(), map.max_offset(), 0);
}

/// `uvm_map_splitentry`: split entry at given address.
///
/// orig (its key): entry that is to be split. next: a newly allocated map entry that is not
/// linked. split: address at which the split is done.
fn uvm_map_splitentry(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    orig: usize,
    mut next: PoolBox<VmMapEntry>,
    split: usize,
) {
    if split & PAGE_MASK != 0 {
        kpanic(format_args!(
            "uvm_map_splitentry: split address {split:#x} not on page boundary!"
        ));
    }
    if cfg!(test) {
        uvm_tree_sanity(map, st, "uvm_map_splitentry");
    }

    // orig stays where it is in the tree until next goes in: found once.
    let owners = st.owners(map);
    let free = {
        let mut fp = st.free_parts(owners);
        let o = ent(fp.tree, orig, "uvm_map_splitentry");
        kassert!(o.start() < split && vmmap_free_end(o) > split);

        // Free space will change, unlink from free space tree.
        let free = owners.uaddr_e(o);
        uvm_mapent_free_remove(&mut fp, ew, free, o);

        let adj = split - o.start();

        uvm_mapent_copy(o, ew, &mut next);
        if split >= o.end() {
            next.set_etype(ew, 0);
            next.set_offset(ew, 0);
            next.set_wired_count(ew, 0);
            next.set_start(ew, split);
            next.set_end(ew, split);
            next.set_guard(ew, 0);
            next.set_fspace(ew, vmmap_free_end(o) - split);
            // The free-space half holds nothing (the C leaves the stale object union).
            *next.handles_owned() = Handles::default();
            o.set_guard(ew, o.guard().min(split - o.end()));
            o.set_fspace(ew, split - vmmap_free_start(o));
        } else {
            o.set_fspace(ew, 0);
            o.set_guard(ew, 0);
            o.set_end(ew, split);
            next.set_start(ew, split);

            let orig_pageoff = o.handles(ew.witness()).pageoff;
            let issubmap = uvm_et_issubmap(o);
            let isobj = uvm_et_isobj(o);
            let nh = next.handles_owned();
            if let Some(amap) = nh.amap.clone() {
                nh.pageoff = amap_splitref(&amap, orig_pageoff, adj);
            }
            let mut adjust_offset = false;
            if issubmap {
                if let Some(submap) = nh.sub_map() {
                    uvm_map_reference(submap);
                }
                adjust_offset = true;
            } else if isobj {
                if let Some(uobj) = nh.uvm_obj() {
                    hal::klock::kernel_lock(); // KERNEL_LOCK()
                    uobj.reference();
                    hal::klock::kernel_unlock(); // KERNEL_UNLOCK()
                }
                adjust_offset = true;
            }
            if adjust_offset {
                next.set_offset(ew, next.offset() + adj as Voff);
            }
        }

        // Link next into address tree. Link orig and next into free-space tree.
        //
        // Don't insert 'next' into the addr tree until orig has been linked, in case the
        // free-list looks at adjacent entries in the addr tree for its decisions.
        let free_before = if o.fspace() > 0 {
            free
        } else {
            owners.uaddr_e(o)
        };
        uvm_mapent_free_insert(&mut fp, ew, free_before, o);
        free
    };
    let nkey = uvm_mapent_addr_insert(map, st, next);
    {
        let mut fp = st.free_parts(owners);
        let n = ent(fp.tree, nkey, "uvm_map_splitentry");
        uvm_mapent_free_insert(&mut fp, ew, free, n);
    }

    if cfg!(test) {
        uvm_tree_sanity(map, st, "uvm_map_splitentry");
    }
}

/// `uvm_tree_assert`: panics when `test` is false (`UVM_ASSERT`), naming the map.
fn uvm_tree_assert(map: &VmMap, test: bool, test_str: &str, who: &str) {
    if test {
        return;
    }

    let map_special = if is_kernel_map(map) {
        " (kernel_map)"
    } else if map_of(KvMap::Kmem).is_some_and(|k| ptr::eq(k, map)) {
        " (kmem_map)"
    } else {
        ""
    };
    kpanic(format_args!(
        "uvm_tree_sanity {:#x}{} ({}): {}",
        map.addr(),
        map_special,
        who,
        test_str
    ));
}

/// `uvm_tree_sanity`: check that map is sane (`VMMAP_DEBUG`; host tests).
pub fn uvm_tree_sanity(map: &VmMap, st: &MapState, who: &str) {
    let owners = st.owners(map);
    let mut addr = map.min_offset();
    for iter in st.addr.iter() {
        // Valid start, end. Catch overflow for end+fspace.
        uvm_tree_assert(map, iter.end() >= iter.start(), "end >= start", who);
        uvm_tree_assert(
            map,
            vmmap_free_end(iter) >= iter.end(),
            "free_end >= end",
            who,
        );

        // May not be empty.
        uvm_tree_assert(
            map,
            iter.start() < vmmap_free_end(iter),
            "start < free_end",
            who,
        );

        // Addresses for entry must lie within map boundaries.
        uvm_tree_assert(
            map,
            iter.start() >= map.min_offset() && vmmap_free_end(iter) <= map.max_offset(),
            "entry inside the map",
            who,
        );

        // Tree may not have gaps.
        uvm_tree_assert(map, iter.start() == addr, "no gaps", who);
        addr = vmmap_free_end(iter);

        // Free space may not cross boundaries, unless the same free list is used on both
        // sides of the border.
        let mut min = vmmap_free_start(iter);
        let max = vmmap_free_end(iter);

        while min < max {
            let bound = owners.boundary(min, max);
            if bound == max {
                break;
            }
            uvm_tree_assert(
                map,
                owners.uaddr(bound - 1) == owners.uaddr(bound),
                "free space on one free list",
                who,
            );
            min = bound;
        }

        if owners.uaddr_e(iter).is_some() {
            uvm_tree_assert(
                map,
                iter.etype() & UVM_ET_FREEMAPPED != 0,
                "free-mapped entry on a free list",
                who,
            );
        } else {
            uvm_tree_assert(
                map,
                iter.etype() & UVM_ET_FREEMAPPED == 0,
                "entry off the free lists",
                who,
            );
        }
    }
    uvm_tree_assert(map, addr == map.max_offset(), "tree ends at max", who);
}

/// `uvm_tree_size_chk`: checks the map's size against its entries (`VMMAP_DEBUG`; host
/// tests); `w` reads the entries' handles for `vmspace_validate`.
pub fn uvm_tree_size_chk<W: Witness + ?Sized>(map: &VmMap, st: &MapState, w: &W, who: &str) {
    let mut size = 0;
    for iter in st.addr.iter() {
        if !uvm_et_ishole(iter) {
            size += iter.end() - iter.start();
        }
    }

    if map.size() != size {
        hal::prf::printf(format_args!(
            "map size = {:#x}, should be {:#x}\n",
            map.size(),
            size
        ));
    }
    uvm_tree_assert(map, map.size() == size, "map size", who);

    vmspace_validate(map, st, w);
}

/// `vmspace_validate`: this function validates the statistics on vmspace (`VMMAP_DEBUG`;
/// host tests).
pub fn vmspace_validate<W: Witness + ?Sized>(map: &VmMap, st: &MapState, w: &W) {
    if !map.is_vmspace() {
        return;
    }
    let vm = map.vmspace_vars();

    let (maxs, mins) = (
        vm.vm_maxsaddr.load(Mem::Relaxed),
        vm.vm_minsaddr.load(Mem::Relaxed),
    );
    let stack_begin = maxs.min(mins);
    let stack_end = maxs.max(mins);

    let mut heap = 0;
    for iter in st.addr.iter() {
        let mut imin = iter.start();

        if uvm_et_ishole(iter)
            || iter.handles(w).uvm_obj().is_some()
            || iter.protection() == PROT_NONE
        {
            continue;
        }

        // Update stack, heap. Keep in mind that (theoretically) the entries of userspace
        // and stack may be joined.
        while imin != iter.end() {
            // Set imax to the first boundary crossed between imin and stack addresses.
            let mut imax = iter.end();
            if imin < stack_begin && imax > stack_begin {
                imax = stack_begin;
            } else if imin < stack_end && imax > stack_end {
                imax = stack_end;
            }

            if !(imin >= stack_begin && imin < stack_end) {
                heap += imax - imin;
            }
            imin = imax;
        }
    }

    heap >>= PAGE_SHIFT;
    let dused = vm.vm_dused.load(Mem::Relaxed);
    if heap != dused as usize {
        hal::prf::printf(format_args!(
            "vmspace stack range: {stack_begin:#x}-{stack_end:#x}\n"
        ));
        kpanic(format_args!(
            "vmspace_validate: vmspace.vm_dused invalid, expected {} pgs, got {} pgs in map {:#x}",
            heap,
            dused,
            map.addr()
        ));
    }
}

/// `uvm_map_init`: init mapping system at boot time: the static entries' pool over their
/// arena, the vmspace and entry pools, and the address selectors.
pub fn uvm_map_init() {
    // now set up static pool of kernel map entries ...
    let _ = KENTRY_POOL.init_arena(&KENTRY_ARENA);

    // initialize the map-related pools.
    let _ = VMSPACE_POOL.init(IPL_NONE, PR_WAITOK, "vmsppl");
    let _ = ENTRY_POOL.init(IPL_VM, PR_WAITOK, "vmmpepl");
    let _ = KMEM_ENTRY_POOL.init(IPL_VM, 0, "vmmpekpl");
    ENTRY_POOL.set_hiwat(8192);

    uvm_addr_init();
}

// DDB hooks (uvm_map_printit, uvm_object_printit, uvm_page_printit): with ddb.

/// `uvm_map_protect`: change map protection.
///
/// - set_max means set max_protection.
/// - map must be unlocked.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_map_protect(
    map: &VmMap,
    start: usize,
    end: usize,
    new_prot: VmProt,
    etype: i32,
    set_max: bool,
    checkimmutable: bool,
) -> Result<(), Errno> {
    kassert!(etype & !UVM_ET_STACK == 0); // only UVM_ET_STACK allowed

    if start > end {
        return Err(Errno::EINVAL);
    }
    let start = start.max(map.min_offset());
    let end = end.min(map.max_offset());
    if start >= end {
        return Ok(());
    }

    let g = map.lock();
    // The etype as the body left it at its `out` label (narrowed to 0 unless the mapping
    // becomes RW), which is the one the C's `map->sserial++` tests.
    let (g, error, etype) =
        uvm_map_protect_locked(map, g, start, end, new_prot, etype, set_max, checkimmutable);
    if etype & UVM_ET_STACK != 0 {
        map.sserial.fetch_add(1, Mem::Relaxed);
    }
    drop(g);
    error
}

/// [`uvm_map_protect`] with the map locked (the C's body up to the `out` label); the guard
/// comes back, with the etype as it stood at the `out` label.
#[allow(clippy::too_many_arguments)] // the C's locals
fn uvm_map_protect_locked<'a>(
    map: &'a VmMap,
    mut g: MapWriteGuard<'a>,
    start: usize,
    end: usize,
    new_prot: VmProt,
    etype: i32,
    set_max: bool,
    checkimmutable: bool,
) -> (MapWriteGuard<'a>, Result<(), Errno>, i32) {
    let mut dused = 0;
    let mut error = Ok(());

    // Set up first and last.
    // - first will contain first entry at or after start.
    let first = {
        let st = g.state();
        let first = entry_at(map, &st.addr, start, "uvm_map_protect");
        let first = if first.end() <= start {
            st.addr.next(first)
        } else {
            Some(first)
        };

        // First, check for protection violations.
        let mut iter = first;
        while let Some(e) = iter.filter(|e| e.start() < end) {
            iter = st.addr.next(e);
            // Treat memory holes as free space.
            if e.start() == e.end() || uvm_et_ishole(e) {
                continue;
            }

            if checkimmutable && e.etype() & UVM_ET_IMMUTABLE != 0 {
                error = Err(Errno::EPERM);
                break;
            }
            let old_prot = e.protection();
            if old_prot == PROT_NONE && new_prot != old_prot {
                dused += uvmspace_dused(map, start.max(e.start()), end.min(e.end()));
            }

            if uvm_et_issubmap(e) {
                error = Err(Errno::EINVAL);
                break;
            }
            if new_prot & e.max_protection() != new_prot {
                error = Err(Errno::EACCES);
                break;
            }
            if is_kernel_map(map) && new_prot & (PROT_WRITE | PROT_EXEC) == (PROT_WRITE | PROT_EXEC)
            {
                kpanic(format_args!(
                    "uvm_map_protect: kernel map W^X violation requested"
                ));
            }
        }
        first.map(VmMapEntry::start)
    };
    if error.is_err() {
        return (g, error, etype);
    }

    // Check limits.
    if dused > 0 && map.is_vmspace() {
        let limit = usize::try_from(vmproc().lim_cur(RLIMIT_DATA)).unwrap_or(usize::MAX);
        let dused = ptoa(dused);
        let vm_dused = usize::try_from(map.vm().vm_dused.load(Mem::Relaxed)).unwrap_or(0);
        if limit < dused || limit - dused < ptoa(vm_dused) {
            return (g, Err(Errno::ENOMEM), etype);
        }
    }

    // only apply UVM_ET_STACK on a mapping changing to RW
    let etype = if etype != 0 && new_prot != (PROT_READ | PROT_WRITE) {
        0
    } else {
        etype
    };

    // Fix protections.
    let mut iter = first;
    while let Some(key) = iter {
        // The entry's changes under the write lock; whether to wire it (VM_MAP_WIREFUTURE)
        // after.
        let step = {
            let mut sp = g.split();
            let (st, ew) = sp.parts();
            let e = ent(&st.addr, key, "uvm_map_protect");
            if e.start() >= end {
                break;
            }
            // Treat memory holes as free space.
            if e.start() == e.end() || uvm_et_ishole(e) {
                (key, None)
            } else {
                let old_prot = e.protection();

                // Skip adapting protection iff old and new protection are equal.
                let skip = if set_max {
                    old_prot == new_prot & old_prot && e.max_protection() == new_prot
                } else {
                    old_prot == new_prot
                };
                if skip {
                    (key, None)
                } else {
                    let key = uvm_map_clip_start_at(map, st, ew, key, start);
                    uvm_map_clip_end_at(map, st, ew, key, end);
                    let e = ent(&st.addr, key, "uvm_map_protect");

                    if set_max {
                        e.set_max_protection(ew, new_prot);
                        e.set_protection(ew, e.protection() & new_prot);
                    } else {
                        e.set_protection(ew, new_prot);
                    }
                    e.etype_set(ew, etype); // potentially add UVM_ET_STACK

                    // update physical map if necessary. worry about copy-on-write here --
                    // CHECK THIS XXX
                    if e.protection() != old_prot {
                        let mask = if uvm_et_iscopyonwrite(e) {
                            !PROT_WRITE
                        } else {
                            PROT_MASK
                        };

                        if map.is_vmspace() {
                            if old_prot == PROT_NONE {
                                map.vm()
                                    .dused_add(uvmspace_dused(map, e.start(), e.end()) as isize);
                            }
                            if e.protection() == PROT_NONE {
                                map.vm()
                                    .dused_add(-(uvmspace_dused(map, e.start(), e.end()) as isize));
                            }
                        }

                        // update pmap
                        if e.protection() & mask == PROT_NONE && vm_mapent_iswired(e) {
                            // TODO(ariane) this is stupid. wired_count is 0 if not wired,
                            // otherwise anything larger than 0 (incremented once each time
                            // wire is called). Mostly to be able to undo the damage on
                            // failure. Not the actually be a wired refcounter...
                            // Originally: iter->wired_count--;
                            // (don't we have to unwire this in the pmap as well?)
                            e.set_wired_count(ew, 0);
                        }
                        let h = e.handles(ew.witness()).clone();
                        let locks = uvm_map_lock_entry(&h);
                        map_pmap_protect(map, e.start(), e.end(), e.protection() & mask);
                        drop(locks);
                    }

                    // If the map is configured to lock any future mappings, wire this entry
                    // now if the old protection was PROT_NONE and the new protection is
                    // not PROT_NONE.
                    let wire = map.flags() & VM_MAP_WIREFUTURE != 0
                        && !vm_mapent_iswired(e)
                        && old_prot == PROT_NONE
                        && new_prot != PROT_NONE;
                    (key, wire.then(|| (e.start(), e.end())))
                }
            }
        };

        let (key, wire) = step;
        if let Some((ws, we)) = wire {
            let (back, r) = uvm_map_pageable_keep(g, ws, we, false);
            g = back;
            if r.is_err() {
                // If locking the entry fails, remember the error if it's the first one.
                // Note we still continue setting the protection in the map, but it will
                // return the resource storage condition regardless.
                //
                // XXX Ignore what the actual error is, XXX just call it a resource shortage
                // XXX so that it doesn't get confused XXX what uvm_map_protect() itself
                // would XXX normally return.
                error = Err(Errno::ENOMEM);
            }
        }
        iter = next_key(&g.state().addr, key);
    }
    // pmap_update(map->pmap): nothing to do.

    (g, error, etype)
}

/// `uvmspace_alloc`: allocate a vmspace structure.
///
/// - structure includes vm_map and pmap
/// - XXX: no locking on this structure
/// - refcnt set to 1, rest must be init'd by caller
pub fn uvmspace_alloc(min: usize, max: usize, pageable: bool, remove_holes: bool) -> VmspaceRef {
    let vm = new_vmspace();
    uvmspace_init(&vm, None, min, max, pageable, remove_holes);
    vm
}

/// `vmspace0`'s set-up (`uvmspace_init(&vmspace0, pmap_kernel(), min, max, TRUE, TRUE)` in
/// `main`): a vmspace over the kernel pmap, an item of `"vmsppl"` like every other (the C's
/// is a static: `vmstat -m` shows one item more).
pub fn uvmspace_alloc_kernel(min: usize, max: usize) -> VmspaceRef {
    let vm = new_vmspace();
    uvmspace_init(&vm, Some(PmapRef::Kernel), min, max, true, true);
    vm
}

/// A zeroed vmspace of `"vmsppl"` (`pool_get(PR_WAITOK | PR_ZERO)`).
fn new_vmspace() -> VmspaceRef {
    match VMSPACE_POOL.get_shared(PR_WAITOK, Vmspace::new) {
        Some(vm) => vm,
        None => kpanic(format_args!("uvmspace_alloc: uvm_vmspace_pool is empty")),
    }
}

/// `uvmspace_init`: initialize a vmspace structure (in its pool item: the map's lock keys
/// its state there).
///
/// - XXX: no locking on this structure
/// - refcnt set to 1, rest must be init'd by caller
pub fn uvmspace_init(
    vm: &Vmspace,
    pmap: Option<PmapRef>,
    min: usize,
    max: usize,
    pageable: bool,
    remove_holes: bool,
) {
    kassert!(pmap.as_ref().is_none_or(|pm| pm.is_kernel()));

    let pmap = match pmap {
        Some(pm) => pm,
        None => match hal::pmap::create() {
            Some(pm) => pm,
            None => kpanic(format_args!("uvmspace_init: no pmap")),
        },
    };

    uvm_map_setup(
        &vm.vm_map,
        pmap,
        min,
        max,
        (if pageable { VM_MAP_PAGEABLE } else { 0 }) | VM_MAP_ISVMSPACE,
    );

    vm.vm_refcnt.store(1, Mem::Relaxed);

    if remove_holes {
        // pmap_remove_holes(vm): nothing on amd64 and arm64 (no hole in their user
        // address spaces needs an entry).
    }
}

/// `uvmspace_share`: share a vmspace between two processes.
///
/// - used for vfork
pub fn uvmspace_share(vm: &VmspaceRef) -> VmspaceRef {
    uvmspace_addref(vm);
    vm.clone()
}

/// `uvmspace_exec`: the process wants to exec a new program.
///
/// A vmspace only this process uses is recycled for `[start, end)` and `None` comes back;
/// a shared one gets a new vmspace, returned for the caller to install (`pmap_deactivate`,
/// `p->p_vmspace = pr->ps_vmspace = nvm`, `pmap_activate`) before it frees the old one
/// (`uvmspace_free`).
///
/// - XXX: no locking on vmspace
pub fn uvmspace_exec(vm: &Vmspace, start: usize, end: usize) -> Option<VmspaceRef> {
    let map = &vm.vm_map;
    let mut end = end;

    kassert!(start & PAGE_MASK == 0);
    kassert!(end & PAGE_MASK == 0 || end & PAGE_MASK == PAGE_MASK);

    // pmap_unuse_final(p) before stack addresses go away: nothing on amd64 and arm64.
    let mut dead = DeadQueue::new();

    // see if more than one process is using this vmspace...
    let nvm = if vm.vm_refcnt.load(Mem::Relaxed) == 1 {
        // If pr is the only process using its vmspace then we can safely recycle that
        // vmspace for the program that is being exec'd.

        // SYSVSHM is not configured: no segments to kill.

        // POSIX 1003.1b -- "lock future mappings" is revoked when a process execs another
        // program image.
        let mut g = map.lock();
        map.modflags(0, VM_MAP_WIREFUTURE | VM_MAP_PINSYSCALL_ONCE);
        {
            let mut sp = g.split();
            let (st, ew) = sp.parts();

            // now unmap the old program
            //
            // Instead of attempting to keep the map valid, we simply nuke all entries and
            // ask uvm_map_setup to reinitialize the map to the new boundaries.
            //
            // uvm_unmap_remove will actually nuke all entries for us (as in, not replace
            // them with free-memory entries).
            let (min, max) = (map.min_offset(), map.max_offset());
            let _ = unmap_remove(map, st, ew, min, max, &mut dead, true, false, false);

            kdassert!(st.addr.is_empty());

            // Nuke statistics and boundaries.
            vm.clear_startcopy();

            if end & PAGE_MASK != 0 {
                end = end.wrapping_add(1);
                if end == 0 {
                    // overflow
                    end -= PAGE_SIZE;
                }
            }

            // Setup new boundaries and populate map with entries.
            map.min_offset.store(start, Mem::Relaxed);
            map.max_offset.store(end, Mem::Relaxed);
            uvm_map_setup_entries(map, st, ew);
        }
        drop(g);

        // but keep MMU holes unavailable: pmap_remove_holes(ovm), nothing on amd64 and
        // arm64.
        None
    } else {
        // pr's vmspace is being shared, so we can't reuse it for pr since it is still being
        // used for others. allocate a new vmspace for pr
        Some(uvmspace_alloc(start, end, map.is_pageable(), true))
    };
    // PMAP_CHECK_COPYIN is not configured.

    // Release dead entries
    uvm_unmap_detach(&mut dead, 0);
    nvm
}

/// `uvmspace_addref`: add a reference to a vmspace.
pub fn uvmspace_addref(vm: &Vmspace) {
    kassert!(vm.vm_refcnt.load(Mem::Relaxed) > 0);
    vm.vm_refcnt.fetch_add(1, Mem::Relaxed);
}

/// `uvmspace_purge`: tears the address space down: locks the map, to wait out all other
/// references to it, and deletes all of the mappings and pages they hold.
pub fn uvmspace_purge(vm: &Vmspace) {
    // SYSVSHM is not configured: no shared memory segments to get rid of.
    uvm_map_teardown(&vm.vm_map);
}

/// `uvmspace_free`: free a vmspace data structure: the last reference purges it, and the
/// item goes back to `"vmsppl"` with the last handle (its pmap with it: `pmap_destroy`).
pub fn uvmspace_free(vm: VmspaceRef) {
    if vm.vm_refcnt.fetch_sub(1, Mem::AcqRel) == 1 {
        // Sanity check. Kernel threads never end up here and userland ones already tear
        // down there VM space in exit1().
        uvmspace_purge(&vm);
    }
    drop(vm);
}

/// What `uvm_mapent_clone` copies of the entry it clones: the scalars and the handles, read
/// under the source map's lock.
struct OldEntry {
    start: usize,
    end: usize,
    offset: Voff,
    etype: i32,
    protection: VmProt,
    max_protection: VmProt,
    inheritance: VmInherit,
    advice: i32,
    wired: bool,
    handles: Handles,
}

impl OldEntry {
    /// `e`, an entry of the source map, its handles read with `w`.
    fn of(e: &VmMapEntry, w: &EntryWrite<'_>) -> Self {
        Self {
            start: e.start(),
            end: e.end(),
            offset: e.offset(),
            etype: e.etype(),
            protection: e.protection(),
            max_protection: e.max_protection(),
            inheritance: e.inheritance(),
            advice: e.advice(),
            wired: vm_mapent_iswired(e),
            handles: e.handles(w.witness()).clone(),
        }
    }
}

/// `uvm_mapent_clone`: clone map entry into other map.
///
/// Mapping will be placed at dstaddr, for the same length. Space must be available.
/// Reference counters are incremented.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uvm_mapent_clone<'s>(
    dstmap: &VmMap,
    st: &'s mut MapState,
    ew: &mut EntryWrite<'_>,
    dstaddr: usize,
    dstlen: usize,
    off: usize,
    prot: VmProt,
    maxprot: VmProt,
    old: &OldEntry,
    dead: &mut DeadQueue,
    mapent_flags: u32,
    amap_share_flags: i32,
) -> Option<&'s VmMapEntry> {
    kdassert!(old.etype & UVM_ET_SUBMAP == 0);

    // Create new entry (linked in on creation). Fill in first, last.
    let Some((first, last)) = isavail_keys(dstmap, st, None, dstaddr, dstlen) else {
        kpanic(format_args!(
            "uvm_mapent_clone: no space in map for entry in empty map"
        ));
    };
    let ne = uvm_map_mkentry(
        dstmap,
        st,
        ew,
        first,
        last,
        dstaddr,
        dstlen,
        mapent_flags,
        dead,
        None,
    )?;
    // old_entry -> new_entry
    *ne.handles_mut(ew) = old.handles.clone();
    ne.set_offset(ew, old.offset);
    ne.etype_set(ew, old.etype & !UVM_ET_FREEMAPPED);
    ne.set_protection(ew, prot);
    ne.set_max_protection(ew, maxprot);
    ne.set_inheritance(ew, old.inheritance);
    ne.set_advice(ew, old.advice);

    // gain reference to object backing the map (can't be a submap).
    let len = (ne.end() - ne.start()) >> PAGE_SHIFT;
    let isobj = uvm_et_isobj(ne);
    let h = ne.handles_mut(ew);
    if let Some(amap) = h.amap.clone() {
        h.pageoff += (off >> PAGE_SHIFT) as i32;
        amap_ref(&amap, h.pageoff as usize, len, amap_share_flags);
    }
    let uobj = if isobj { h.uvm_obj().cloned() } else { None };
    if let Some(uobj) = uobj {
        ne.set_offset(ew, ne.offset() + off as Voff);
        uobj.reference();
    }

    Some(ne)
}

/// `uvm_mapent_share`: clones the entry at `old_key` of `old_map` into `dstmap` sharing its
/// amap.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uvm_mapent_share<'s>(
    dstmap: &VmMap,
    st: &'s mut MapState,
    ew: &mut EntryWrite<'_>,
    dstaddr: usize,
    dstlen: usize,
    off: usize,
    prot: VmProt,
    maxprot: VmProt,
    old_map: &VmMap,
    ost: &mut MapState,
    oew: &mut EntryWrite<'_>,
    old_key: usize,
    dead: &mut DeadQueue,
) -> Option<&'s VmMapEntry> {
    // If old_entry refers to a copy-on-write region that has not yet been written to
    // (needs_copy flag is set), then we need to allocate a new amap for old_entry.
    //
    // If we do not do this, and the process owning old_entry does a copy-on write later,
    // old_entry and new_entry will refer to different memory regions, and the memory
    // between the processes is no longer shared.
    //
    // [in other words, we need to clear needs_copy]
    if uvm_et_isneedscopy(ent(&ost.addr, old_key, "uvm_mapent_share")) {
        // get our own amap, clears needs_copy
        amap_copy(old_map, ost, oew, old_key, M_WAITOK, false, 0, 0);
        // XXXCDC: WAITOK???
    }

    let old = OldEntry::of(ent(&ost.addr, old_key, "uvm_mapent_share"), oew);
    uvm_mapent_clone(
        dstmap,
        st,
        ew,
        dstaddr,
        dstlen,
        off,
        prot,
        maxprot,
        &old,
        dead,
        0,
        AMAP_SHARED,
    )
}

/// The child's map, write-locked, for the fork functions.
struct ForkDst<'s, 'w> {
    map: &'s VmMap,
    st: &'s mut MapState,
    ew: &'s mut EntryWrite<'w>,
}

/// The parent's map, write-locked, for the fork functions.
struct ForkSrc<'s, 'w> {
    map: &'s VmMap,
    st: &'s mut MapState,
    ew: &'s mut EntryWrite<'w>,
}

/// What `uvmspace_fork` reads of the new entry a fork function made, once that function is
/// done with it (the process statistics).
#[derive(Clone, Copy)]
struct Forked {
    start: usize,
    end: usize,
    etype: i32,
    protection: VmProt,
}

impl Forked {
    /// `e`'s fields as they are now.
    fn of(e: &VmMapEntry) -> Self {
        Self {
            start: e.start(),
            end: e.end(),
            etype: e.etype(),
            protection: e.protection(),
        }
    }
}

/// `uvm_mapent_forkshared`: share the mapping: this means we want the old and new entries
/// to share amaps and backing objects.
fn uvm_mapent_forkshared(
    new: &mut ForkDst<'_, '_>,
    old: &mut ForkSrc<'_, '_>,
    old_key: usize,
    dead: &mut DeadQueue,
) -> Option<Forked> {
    let oe = ent(&old.st.addr, old_key, "uvm_mapent_forkshared");
    let (start, end) = (oe.start(), oe.end());
    let (prot, maxprot) = (oe.protection(), oe.max_protection());
    uvm_mapent_share(
        new.map,
        new.st,
        new.ew,
        start,
        end - start,
        0,
        prot,
        maxprot,
        old.map,
        old.st,
        old.ew,
        old_key,
        dead,
    )
    .map(Forked::of)
}

/// `uvm_mapent_forkcopy`: copy-on-write the mapping (using mmap's `MAP_PRIVATE`
/// semantics): `oe` is the entry of `old_map`, whose write lock `oew` is.
///
/// allocate new_entry, adjust reference counts. (note that new references are read-only).
fn uvm_mapent_forkcopy(
    new: &mut ForkDst<'_, '_>,
    old_map: &VmMap,
    oew: &EntryWrite<'_>,
    oe: &VmMapEntry,
    dead: &mut DeadQueue,
) -> Option<Forked> {
    let o = OldEntry::of(oe, oew);
    let ne = uvm_mapent_clone(
        new.map,
        new.st,
        new.ew,
        o.start,
        o.end - o.start,
        0,
        o.protection,
        o.max_protection,
        &o,
        dead,
        0,
        0,
    )?;

    ne.etype_set(new.ew, UVM_ET_COPYONWRITE | UVM_ET_NEEDSCOPY);

    // the new entry will need an amap. it will either need to be copied from the old entry
    // or created from scratch (if the old entry does not have an amap). can we defer this
    // process until later (by setting "needs_copy") or do we need to copy the amap now?
    //
    // we must copy the amap now if any of the following conditions hold:
    // 1. the old entry has an amap and that amap is being shared. this means that the old
    //    (parent) process is sharing the amap with another process. if we do not clear
    //    needs_copy here we will end up in a situation where both the parent and child
    //    process are referring to the same amap with "needs_copy" set. if the parent
    //    write-faults, the fault routine will clear "needs_copy" in the parent by allocating
    //    a new amap. this is wrong because the parent is supposed to be sharing the old amap
    //    and the new amap will break that.
    //
    // 2. if the old entry has an amap and a non-zero wire count then we are going to have
    //    to call amap_cow_now to avoid page faults in the parent process. since
    //    amap_cow_now requires "needs_copy" to be clear we might as well clear it here as
    //    well.
    let ne = if let Some(old_amap) = &o.handles.amap
        && (old_amap.flags() & AMAP_SHARED != 0 || o.wired)
    {
        // XXXCDC: M_WAITOK ... ok? (the entry is found again after the copy, which may
        // change the child's map through its write lock)
        let key = ne.start();
        let key = amap_copy(new.map, new.st, new.ew, key, M_WAITOK, false, 0, 0);
        ent(&new.st.addr, key, "uvm_mapent_forkcopy")
    } else {
        ne
    };

    // if the parent's entry is wired down, then the parent process does not want page
    // faults on access to that memory. this means that we cannot do copy-on-write because
    // we can't write protect the old entry. in this case we resolve all copy-on-write
    // faults now, using amap_cow_now. note that we have already allocated any needed amap
    // (above).
    if o.wired {
        // resolve all copy-on-write faults now (note that there is nothing to do if the
        // old mapping does not have an amap).
        if o.handles.amap.is_some()
            && let Some(amap) = &ne.handles(new.ew.witness()).amap
        {
            // the child's map is write-locked (amap_cow_now(new_map, new_entry)).
            amap_cow_now(amap);
        }
    } else {
        let protect_child = if o.handles.amap.is_some() {
            // setup mappings to trigger copy-on-write faults we must write-protect the
            // parent if it has an amap and it is not already "needs_copy"... if it is
            // already "needs_copy" then the parent has already been write-protected by a
            // previous fork operation.
            //
            // if we do not write-protect the parent, then we must be sure to write-protect
            // the child.
            if o.etype & UVM_ET_NEEDSCOPY == 0 {
                if o.max_protection & PROT_WRITE != 0 {
                    let locks = uvm_map_lock_entry(&o.handles);
                    map_pmap_protect(old_map, o.start, o.end, o.protection & !PROT_WRITE);
                    drop(locks);
                    // pmap_update(old_map->pmap): nothing to do.
                }
                oe.etype_set(oew, UVM_ET_NEEDSCOPY);
            }

            // parent must now be write-protected
            false
        } else {
            // we only need to protect the child if the parent has write access.
            o.max_protection & PROT_WRITE != 0
        };

        // protect the child's mappings if necessary
        if protect_child {
            map_pmap_protect(new.map, ne.start(), ne.end(), ne.protection() & !PROT_WRITE);
        }
    }

    Some(Forked::of(ne))
}

/// `uvm_mapent_forkzero`: zero the mapping: the new entry will be zero initialized (`oe`
/// is the entry of the parent's map, whose write lock `oew` is).
fn uvm_mapent_forkzero(
    new: &mut ForkDst<'_, '_>,
    oew: &EntryWrite<'_>,
    oe: &VmMapEntry,
    dead: &mut DeadQueue,
) -> Option<Forked> {
    let o = OldEntry::of(oe, oew);
    let ne = uvm_mapent_clone(
        new.map,
        new.st,
        new.ew,
        o.start,
        o.end - o.start,
        0,
        o.protection,
        o.max_protection,
        &o,
        dead,
        0,
        0,
    )?;

    ne.etype_set(new.ew, UVM_ET_COPYONWRITE | UVM_ET_NEEDSCOPY);

    let slots = atop(ne.end() - ne.start());
    let isobj = uvm_et_isobj(ne);
    let h = ne.handles_mut(new.ew);
    if let Some(amap) = h.amap.take() {
        amap_unref(&amap, h.pageoff as usize, slots, false);
        h.pageoff = 0;
    }

    if isobj {
        if let Some(uobj) = h.uvm_obj() {
            uobj.detach();
        }
        h.object = EntryObject::None;
        ne.etype_clear(new.ew, UVM_ET_OBJ);
    }

    Some(Forked::of(ne))
}

/// `uvmspace_fork`: fork a process' main map.
///
/// - create a new vmspace for child process from parent.
/// - parent's map must not be locked.
pub fn uvmspace_fork(vm1: &Vmspace) -> VmspaceRef {
    let old_map = &vm1.vm_map;

    let mut g1 = old_map.lock();

    let vm2 = uvmspace_alloc(
        old_map.min_offset(),
        old_map.max_offset(),
        old_map.is_pageable(),
        false,
    );
    vm2.copy_startcopy_from(vm1);
    vm2.vm_dused.store(0, Mem::Relaxed); // Statistic managed by us.
    let new_map = &vm2.vm_map;
    let mut g2 = new_map.lock();

    // go entry-by-entry
    let mut dead = DeadQueue::new();
    {
        let mut sp1 = g1.split();
        let (ost, oew) = sp1.parts();
        let mut sp2 = g2.split();
        let (nst, new_ew) = sp2.parts();
        let mut old = ForkSrc {
            map: old_map,
            st: ost,
            ew: oew,
        };
        let mut new = ForkDst {
            map: new_map,
            st: nst,
            ew: new_ew,
        };

        // RBT_FOREACH over the parent's entries: no fork function moves, adds or removes an
        // entry of the parent's map (the shared case's amap_copy does not chunk), so the walk
        // follows the links from the entry in hand; the shared case, which changes the
        // parent's map through its write lock, finds its entry again by its start.
        let mut iter = old.st.addr.min();
        while let Some(oe) = iter {
            if oe.start() == oe.end() {
                iter = old.st.addr.next(oe);
                continue;
            }

            // first, some sanity checks on the old entry
            if uvm_et_issubmap(oe) {
                kpanic(format_args!(
                    "fork: encountered a submap during fork (illegal)"
                ));
            }

            if !uvm_et_iscopyonwrite(oe) && uvm_et_isneedscopy(oe) {
                kpanic(format_args!(
                    "fork: non-copy_on_write map entry marked needs_copy (illegal)"
                ));
            }

            // Apply inheritance.
            let forked = match oe.inheritance() {
                MAP_INHERIT_SHARE => {
                    let key = oe.start();
                    let forked = uvm_mapent_forkshared(&mut new, &mut old, key, &mut dead);
                    iter = find(&old.st.addr, key).and_then(|e| old.st.addr.next(e));
                    forked
                }
                MAP_INHERIT_COPY => {
                    let forked = uvm_mapent_forkcopy(&mut new, old.map, old.ew, oe, &mut dead);
                    iter = old.st.addr.next(oe);
                    forked
                }
                MAP_INHERIT_ZERO => {
                    let forked = uvm_mapent_forkzero(&mut new, old.ew, oe, &mut dead);
                    iter = old.st.addr.next(oe);
                    forked
                }
                _ => {
                    iter = old.st.addr.next(oe);
                    continue;
                }
            };
            let Some(ne) = forked else {
                continue;
            };

            // Update process statistics.
            let hole = ne.etype & UVM_ET_HOLE != 0;
            if !hole {
                new_map.size.fetch_add(ne.end - ne.start, Mem::Relaxed);
            }
            if ne.etype & UVM_ET_OBJ == 0 && !hole && ne.protection != PROT_NONE {
                vm2.dused_add(uvmspace_dused(new_map, ne.start, ne.end) as isize);
            }
        }
    }
    new_map.modflags(old_map.flags() & VM_MAP_PINSYSCALL_ONCE, 0);
    // PMAP_CHECK_COPYIN is not configured.

    drop(g1);
    drop(g2);

    // This can actually happen, if multiple entries described a space in which an entry
    // was inherited.
    uvm_unmap_detach(&mut dead, 0);

    // SYSVSHM is not configured: no shmfork.

    vm2
}

/// `uvm_map_hint`: return the beginning of the best area suitable for creating a new
/// mapping with "prot" protection.
pub fn uvm_map_hint(vm: &VmspaceVars, _prot: VmProt, minaddr: usize, maxaddr: usize) -> usize {
    // __i386__: no.

    // __LP64__.
    let mut spacing = (4usize * 1024 * 1024 * 1024).min(MAXDSIZ) - 1;

    // Start malloc/mmap after the brk.
    let mut addr = vm.vm_daddr.load(Mem::Relaxed) + BRKSIZ;
    addr = addr.max(minaddr);

    if addr < maxaddr {
        while spacing > maxaddr - addr {
            spacing >>= 1;
        }
    }
    addr += arc4random() as usize & spacing;
    round_page(addr)
}

/// `uvm_map_submap`: punch down part of a map into a submap.
///
/// - only the kernel_map is allowed to be submapped
/// - the purpose of submapping is to break up the locking granularity of a larger map
/// - the range specified must have been mapped previously with a `uvm_map()` call [with
///   uobj==NULL] to create a blank map entry in the main map. [And it had better still be
///   blank!]
/// - maps which contain submaps should never be copied or forked.
/// - to remove a submap, use `uvm_unmap()` on the main map and then
///   `uvm_map_deallocate()` the submap.
/// - main map must be unlocked.
/// - submap must have been init'd and have a zero reference count. [need not be locked as
///   we don't actually reference it]
pub fn uvm_map_submap(
    map: &VmMap,
    start: usize,
    end: usize,
    submap: &'static VmMap,
) -> Result<(), Errno> {
    if start > map.max_offset()
        || end > map.max_offset()
        || start < map.min_offset()
        || end < map.min_offset()
    {
        return Err(Errno::EINVAL);
    }

    let mut g = map.lock();
    let mut sp = g.split();
    let (st, ew) = sp.parts();

    let key = uvm_map_lookup_entry(&st.addr, start).map(VmMapEntry::start);
    let key = key.map(|k| {
        let k = uvm_map_clip_start_at(map, st, ew, k, start);
        uvm_map_clip_end_at(map, st, ew, k, end);
        k
    });

    let ok = key.is_some_and(|k| {
        let e = ent(&st.addr, k, "uvm_map_submap");
        let h = e.handles(ew.witness());
        e.start() == start
            && e.end() == end
            && h.uvm_obj().is_none()
            && h.amap.is_none()
            && !uvm_et_iscopyonwrite(e)
            && !uvm_et_isneedscopy(e)
    });
    match key {
        Some(k) if ok => {
            let e = ent(&st.addr, k, "uvm_map_submap");
            e.etype_set(ew, UVM_ET_SUBMAP);
            e.handles_mut(ew).object = EntryObject::SubMap(submap);
            e.set_offset(ew, 0);
            uvm_map_reference(submap);
            Ok(())
        }
        _ => Err(Errno::EINVAL),
    }
}

/// `uvm_map_checkprot`: check protection in map.
///
/// - must allow specific protection in a fully allocated region.
/// - map must be read or write locked by caller (`g`).
pub fn uvm_map_checkprot(g: &MapGuard<'_>, start: usize, end: usize, protection: VmProt) -> bool {
    let map = g.map();
    if start < map.min_offset() || end > map.max_offset() || start > end {
        return false;
    }
    if start == end {
        return true;
    }

    // Iterate entries.
    let atree = &g.state().addr;
    let mut iter = uvm_map_entrybyaddr(atree, start);
    while let Some(entry) = iter.filter(|e| e.start() < end) {
        // Fail if a hole is found.
        if uvm_et_ishole(entry) || (entry.end() < end && entry.end() != vmmap_free_end(entry)) {
            return false;
        }

        // Check protection.
        if entry.protection() & protection != protection {
            return false;
        }
        iter = atree.next(entry);
    }
    true
}

/// `uvm_map_create`: create map (an `M_VMMAP` block that lives for ever: the boot's
/// submaps).
pub fn uvm_map_create(pmap: PmapRef, min: usize, max: usize, flags: i32) -> &'static VmMap {
    let Some(b) = MBox::new(M_VMMAP, M_WAITOK, VmMap::new(min, max)) else {
        kpanic(format_args!("uvm_map_create: no memory for the map"));
    };
    let map = MBox::leak(b);
    uvm_map_setup(map, pmap, min, max, flags);
    map
}

/// `uvm_map_deallocate`: drop reference to a map.
///
/// - caller must not lock map
/// - we will zap map if ref count goes to zero (its block stays: see the module's
///   deviations)
pub fn uvm_map_deallocate(map: &'static VmMap) {
    let c = map.ref_count.fetch_sub(1, Mem::AcqRel) - 1;
    if c > 0 {
        return;
    }

    // all references gone. unmap and free.
    //
    // No lock required: we are only one to access this map (the write lock is the
    // entries' proof).
    let mut dead = DeadQueue::new();
    {
        let mut g = map.lock();
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        let (min, max) = (map.min_offset(), map.max_offset());
        let _ = unmap_remove(map, st, ew, min, max, &mut dead, true, false, false);
        kassert!(st.addr.is_empty());
    }

    uvm_unmap_detach(&mut dead, 0);
}

/// The key of the entry an operation on `[start, end)` starts at: the one holding `start`,
/// clipped there, or the next one when `start` is in its free space
/// (`uvm_map_inherit`'s, `uvm_map_immutable`'s and `uvm_map_advice`'s first step).
fn clip_first(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    start: usize,
    who: &str,
) -> Option<usize> {
    let e = entry_at(map, &st.addr, start, who);
    let key = e.start();
    if e.end() > start {
        Some(uvm_map_clip_start_at(map, st, ew, key, start))
    } else {
        next_key(&st.addr, key)
    }
}

/// `uvm_map_inherit`: set inheritance code for range of addrs in map.
///
/// - map must be unlocked
/// - note that the inherit code is used during a "fork". see fork code for details.
pub fn uvm_map_inherit(
    map: &VmMap,
    start: usize,
    end: usize,
    new_inheritance: VmInherit,
) -> Result<(), Errno> {
    match new_inheritance {
        MAP_INHERIT_NONE | MAP_INHERIT_COPY | MAP_INHERIT_SHARE | MAP_INHERIT_ZERO => {}
        _ => return Err(Errno::EINVAL),
    }

    if start > end {
        return Err(Errno::EINVAL);
    }
    let start = start.max(map.min_offset());
    let end = end.min(map.max_offset());
    if start >= end {
        return Ok(());
    }

    let mut g = map.lock();
    let mut sp = g.split();
    let (st, ew) = sp.parts();

    let first = clip_first(map, st, ew, start, "uvm_map_inherit");

    // First check for illegal operations
    let mut entry1 = first.and_then(|k| find(&st.addr, k));
    while let Some(e1) = entry1.filter(|e| e.start() < end) {
        if e1.etype() & UVM_ET_IMMUTABLE != 0 {
            return Err(Errno::EPERM);
        }
        if new_inheritance == MAP_INHERIT_ZERO && e1.protection() & PROT_WRITE == 0 {
            return Err(Errno::EPERM);
        }
        entry1 = st.addr.next(e1);
    }

    let mut iter = first;
    while let Some(key) = iter.filter(|k| *k < end) {
        uvm_map_clip_end_at(map, st, ew, key, end);
        ent(&st.addr, key, "uvm_map_inherit").set_inheritance(ew, new_inheritance);
        iter = next_key(&st.addr, key);
    }

    Ok(())
}

// uvm_map_check_copyin_add: PMAP_CHECK_COPYIN is not configured.

/// `uvm_map_immutable`: block mapping/mprotect for range of addrs in map.
///
/// - map must be unlocked
pub fn uvm_map_immutable(map: &VmMap, start: usize, end: usize, imut: bool) -> Result<(), Errno> {
    if start > end {
        return Err(Errno::EINVAL);
    }
    let start = start.max(map.min_offset());
    let end = end.min(map.max_offset());
    if start >= end {
        return Ok(());
    }

    let mut g = map.lock();
    let mut sp = g.split();
    let (st, ew) = sp.parts();

    let first = clip_first(map, st, ew, start, "uvm_map_immutable");

    // First check for illegal operations
    let mut entry1 = first.and_then(|k| find(&st.addr, k));
    while let Some(e1) = entry1.filter(|e| e.start() < end) {
        if e1.inheritance() == MAP_INHERIT_ZERO {
            return Err(Errno::EPERM);
        }
        entry1 = st.addr.next(e1);
    }

    let mut iter = first;
    while let Some(key) = iter.filter(|k| *k < end) {
        uvm_map_clip_end_at(map, st, ew, key, end);
        let e = ent(&st.addr, key, "uvm_map_immutable");
        if imut {
            e.etype_set(ew, UVM_ET_IMMUTABLE);
        } else {
            e.etype_clear(ew, UVM_ET_IMMUTABLE);
        }
        iter = next_key(&st.addr, key);
    }

    Ok(())
}

/// `uvm_map_advice`: set advice code for range of addrs in map.
///
/// - map must be unlocked
pub fn uvm_map_advice(map: &VmMap, start: usize, end: usize, new_advice: i32) -> Result<(), Errno> {
    match new_advice {
        MADV_NORMAL | MADV_RANDOM | MADV_SEQUENTIAL => {}
        _ => return Err(Errno::EINVAL),
    }

    if start > end {
        return Err(Errno::EINVAL);
    }
    let start = start.max(map.min_offset());
    let end = end.min(map.max_offset());
    if start >= end {
        return Ok(());
    }

    let mut g = map.lock();
    let mut sp = g.split();
    let (st, ew) = sp.parts();

    let mut iter = match uvm_map_entrybyaddr(&st.addr, start) {
        Some(entry) if entry.end() > start => {
            let key = entry.start();
            Some(uvm_map_clip_start_at(map, st, ew, key, start))
        }
        Some(entry) => st.addr.next(entry).map(VmMapEntry::start),
        None => None,
    };

    // XXXJRT: disallow holes?
    while let Some(key) = iter.filter(|k| *k < end) {
        uvm_map_clip_end_at(map, st, ew, key, end);
        ent(&st.addr, key, "uvm_map_advice").set_advice(ew, new_advice);
        iter = next_key(&st.addr, key);
    }

    Ok(())
}

/// `uvm_map_extract`: extract a mapping from a map and put it somewhere in the kernel_map,
/// setting protection to max_prot. Yields the destination address.
///
/// - map should be unlocked (we will write lock it and kernel_map)
/// - start must be page aligned
/// - len must be page sized
/// - flags: `UVM_EXTRACT_FIXPROT`: set prot to maxprot as we go
///
/// Mappings are QREF's.
pub fn uvm_map_extract(
    srcmap: &VmMap,
    start: usize,
    len: usize,
    flags: i32,
) -> Result<usize, Errno> {
    let mut dead = DeadQueue::new();
    let end = start.wrapping_add(len);

    // Sanity check on the parameters. Also, since the mapping may not contain gaps, error
    // out if the mapped area is not in source map.
    if start & PAGE_MASK != 0 || end & PAGE_MASK != 0 || end < start {
        return Err(Errno::EINVAL);
    }
    if start < srcmap.min_offset() || end > srcmap.max_offset() {
        return Err(Errno::EINVAL);
    }

    // Initialize dead entries. Handle len == 0 case.
    if len == 0 {
        return Ok(0);
    }

    // Acquire lock on srcmap.
    let mut g = srcmap.lock();
    let result = {
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        uvm_map_extract_locked(srcmap, st, ew, start, len, flags, &mut dead)
    };
    drop(g);

    uvm_unmap_detach(&mut dead, 0);
    result
}

/// [`uvm_map_extract`] with the source map locked (the C's body up to the `fail` label).
fn uvm_map_extract_locked(
    srcmap: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    start: usize,
    len: usize,
    flags: i32,
    dead: &mut DeadQueue,
) -> Result<usize, Errno> {
    let end = start + len;

    // Lock srcmap, lookup first and last entry in <start,len>.
    let Some(mut first) = uvm_map_entrybyaddr(&st.addr, start).map(VmMapEntry::start) else {
        return Err(Errno::EINVAL);
    };

    // Check that the range is contiguous.
    let mut entry = find(&st.addr, first);
    while let Some(e) = entry.filter(|e| e.end() < end) {
        if vmmap_free_end(e) != e.end() || uvm_et_ishole(e) {
            return Err(Errno::EINVAL);
        }
        entry = st.addr.next(e);
    }
    match entry {
        None => return Err(Errno::EINVAL),
        Some(e) if uvm_et_ishole(e) => return Err(Errno::EINVAL),
        _ => {}
    }

    // Handle need-copy flag.
    let mut iter = Some(first);
    while let Some(key) = iter.filter(|k| ent(&st.addr, *k, "uvm_map_extract").start() < end) {
        let e = ent(&st.addr, key, "uvm_map_extract");
        let mut key = key;
        if uvm_et_isneedscopy(e) {
            let stack = uvm_et_isstack(e);
            let nk = amap_copy(srcmap, st, ew, key, M_NOWAIT, !stack, start, end);
            if key == first {
                first = nk;
            }
            key = nk;
        }
        if uvm_et_isneedscopy(ent(&st.addr, key, "uvm_map_extract")) {
            // amap_copy failure
            return Err(Errno::ENOMEM);
        }
        iter = next_key(&st.addr, key);
    }

    // Lock destination map (kernel_map).
    let kmap = kernel_map();
    let mut kg = kmap.lock();
    let mut ksp = kg.split();
    let (kst, kew) = ksp.parts();

    let Ok((_, _, dstaddr)) = uvm_map_findspace(kmap, kst, len, PAGE_SIZE, 0, PROT_NONE, 0) else {
        return Err(Errno::ENOMEM);
    };

    // We now have srcmap and kernel_map locked. dstaddr contains the destination offset in
    // dstmap.
    // step 1: start looping through map entries, performing extraction.
    let mut error = Ok(dstaddr);
    let mut iter = Some(first);
    while let Some(key) = iter.filter(|k| ent(&st.addr, *k, "uvm_map_extract").start() < end) {
        iter = next_key(&st.addr, key);
        let e = ent(&st.addr, key, "uvm_map_extract");
        kdassert!(!uvm_et_isneedscopy(e));
        if uvm_et_ishole(e) {
            continue;
        }

        // Calculate uvm_mapent_clone parameters.
        let mut cp_start = e.start();
        let cp_off;
        if cp_start < start {
            cp_off = start - cp_start;
            cp_start = start;
        } else {
            cp_off = 0;
        }
        let cp_len = e.end().min(end) - cp_start;

        let old = OldEntry::of(e, ew);
        let Some(ne) = uvm_mapent_clone(
            kmap,
            kst,
            kew,
            cp_start - start + dstaddr,
            cp_len,
            cp_off,
            old.protection,
            old.max_protection,
            &old,
            dead,
            // The C passes its `int` flags on as uvm_mapent_alloc's (no TRYLOCK among them).
            flags as u32,
            AMAP_SHARED | AMAP_REFALL,
        ) else {
            error = Err(Errno::ENOMEM);
            break;
        };
        kmap.size.fetch_add(cp_len, Mem::Relaxed);

        // Figure out the best protection
        if flags & UVM_EXTRACT_FIXPROT != 0 && ne.protection() != PROT_NONE {
            ne.set_protection(kew, ne.max_protection());
        }
        ne.set_protection(kew, ne.protection() & !PROT_EXEC);
    }
    // pmap_update(kernel_map->pmap): nothing to do.

    // Unmap copied entries on failure.
    if error.is_err() {
        let _ = unmap_remove(
            kmap,
            kst,
            kew,
            dstaddr,
            dstaddr + len,
            dead,
            false,
            true,
            false,
        );
    }

    error
}

/// `uvm_map_clean`: clean out a map range.
///
/// - valid flags: if `PGO_CLEANIT`: dirty pages are cleaned first; if `PGO_SYNCIO`: dirty
///   pages are written synchronously; if `PGO_DEACTIVATE`: any cached pages are deactivated
///   after clean; if `PGO_FREE`: any cached pages are freed after clean.
/// - returns an error if any part of the specified range isn't mapped
/// - never a need to flush amap layer since the anonymous memory has no permanent home,
///   but may deactivate pages there
/// - called from `sys_msync()` and `sys_madvise()`
/// - caller must not have map locked
pub fn uvm_map_clean(map: &VmMap, start: usize, end: usize, flags: i32) -> Result<(), Errno> {
    kassert!(flags & (PGO_FREE | PGO_DEACTIVATE) != (PGO_FREE | PGO_DEACTIVATE));

    if start > end || start < map.min_offset() || end > map.max_offset() {
        return Err(Errno::EINVAL);
    }

    let g = map.lock();
    let (first, imut) = {
        let st = g.state();
        let first = uvm_map_entrybyaddr(&st.addr, start);

        // Make a first pass to check for various conditions.
        let mut imut = false;
        let mut entry = first;
        while let Some(e) = entry.filter(|e| e.start() < end) {
            if e.etype() & UVM_ET_IMMUTABLE != 0 {
                imut = true;
            }
            if uvm_et_issubmap(e) {
                return Err(Errno::EINVAL);
            }
            if uvm_et_issubmap(e)
                || uvm_et_ishole(e)
                || (e.end() < end && vmmap_free_end(e) != e.end())
            {
                return Err(Errno::EFAULT);
            }
            entry = st.addr.next(e);
        }
        (first.map(VmMapEntry::start), imut)
    };

    let busy = g.into_busy();
    let mut error = Ok(());
    let mut iter = first;
    loop {
        // The entry's fields and handles under a short read lock (the map is busy:
        // writers wait).
        let step = {
            let r = busy.lock_read();
            let st = r.state();
            iter.and_then(|k| find(&st.addr, k))
                .filter(|e| e.start() < end)
                .map(|e| {
                    let h = e.handles(r.witness());
                    let uobj = h.uvm_obj().cloned().filter(|_| uvm_et_isobj(e));
                    (
                        e.start(),
                        e.end(),
                        e.etype(),
                        e.max_protection(),
                        e.offset(),
                        h.amap.clone(),
                        h.pageoff,
                        uobj,
                        st.addr.next(e).map(VmMapEntry::start),
                    )
                })
        };
        let Some((es, ee, etype, maxprot, offset, amap, pageoff, uobj, next)) = step else {
            break;
        };
        iter = next;

        // No amap cleaning necessary if: there's no amap; we're not deactivating or freeing
        // pages.
        if let Some(amap) = amap.filter(|_| flags & (PGO_DEACTIVATE | PGO_FREE) != 0) {
            if imut {
                return Err(Errno::EPERM);
            }

            let mut cp_start = es.max(start);
            let cp_end = ee.min(end);

            let mut w = amap.lock().write();
            while cp_start != cp_end {
                let off = cp_start - es;
                cp_start += PAGE_SIZE;
                let Some(anon) = amap_lookup(&amap, &w, pageoff, off).cloned() else {
                    continue;
                };

                kassert!(anon.shares_lock(amap.lock()));
                let Some(pg) = anon.page() else {
                    continue;
                };
                kassert!(pg.flags() & PQ_ANON != 0);

                match flags & (PGO_CLEANIT | PGO_FREE | PGO_DEACTIVATE) {
                    // In these first 3 cases, we just deactivate the page.
                    f if f == PGO_CLEANIT | PGO_FREE
                        || f == PGO_CLEANIT | PGO_DEACTIVATE
                        || f == PGO_DEACTIVATE =>
                    {
                        kassert!(pg.owned_by_anon(&*anon));
                        uvm_pagedeactivate(pg, Owner::Anon(&*anon));
                    }
                    f if f == PGO_FREE => {
                        // If there are multiple references to the amap, just deactivate
                        // the page.
                        if amap.refs() > 1 {
                            kassert!(pg.owned_by_anon(&*anon));
                            uvm_pagedeactivate(pg, Owner::Anon(&*anon));
                            continue;
                        }

                        // skip the page if it's wired
                        if pg.wire_count() != 0 {
                            continue;
                        }
                        let anon = amap_unadd(&amap, &mut w, pageoff, off);
                        let refs = anon.refs() - 1;
                        anon.set_refs(refs);
                        if refs == 0 {
                            uvm_anfree(&anon);
                        }
                    }
                    _ => kpanic(format_args!("uvm_map_clean: weird flags")),
                }
            }
            drop(w);
        }

        // flush_object:
        let cp_start = es.max(start);
        let cp_end = ee.min(end);

        // flush pages if we've got a valid backing object.
        //
        // Don't PGO_FREE if we don't have write permission and don't flush if this is a
        // copy-on-write object since we can't know our permissions on it.
        if let Some(uobj) = uobj
            && (flags & PGO_FREE == 0
                || (maxprot & PROT_WRITE != 0 && etype & UVM_ET_COPYONWRITE == 0))
        {
            let mut w = uobj.lock().write();
            let rv = uobj.flush(
                &mut w,
                (cp_start - es) as Voff + offset,
                (cp_end - es) as Voff + offset,
                flags,
            );
            drop(w);

            if !rv {
                error = Err(Errno::EFAULT);
            }
        }
    }

    // vm_map_unbusy(map): the busy mark's drop.
    drop(busy);
    error
}

/// `UVM_MAP_CLIP_END(map, entry, addr)`: ensure that the entry at `key` ends at or before
/// the ending address, if it doesn't we split the entry (`key` stays the lower part's).
///
/// - map must be locked by caller
fn uvm_map_clip_end_at(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    addr: usize,
) {
    let e = ent(&st.addr, key, "uvm_map_clip_end");
    kassert!(e.start() < addr);
    if e.end() > addr {
        uvm_map_clip_end(map, st, ew, key, addr);
    }
}

/// `uvm_map_clip_end`: `UVM_MAP_CLIP_END` implementation.
fn uvm_map_clip_end(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    addr: usize,
) {
    let e = ent(&st.addr, key, "uvm_map_clip_end");
    kassert!(e.start() < addr && vmmap_free_end(e) > addr);
    let Some(tmp) = uvm_mapent_alloc(map, 0) else {
        kpanic(format_args!("uvm_map_clip_end: no map entry"));
    };

    // Invoke splitentry.
    uvm_map_splitentry(map, st, ew, key, tmp, addr);
}

/// `UVM_MAP_CLIP_START(map, entry, addr)`: ensure that the entry at `key` begins at or after
/// the starting address, if it doesn't we split the entry; the key of the entry the C's
/// pointer names afterwards (the part from `addr`).
///
/// - map must be locked by caller
fn uvm_map_clip_start_at(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    addr: usize,
) -> usize {
    let e = ent(&st.addr, key, "uvm_map_clip_start");
    kassert!(e.end() + e.fspace() > addr);
    if e.start() < addr {
        uvm_map_clip_start(map, st, ew, key, addr)
    } else {
        key
    }
}

/// `uvm_map_clip_start`: `UVM_MAP_CLIP_START` implementation.
///
/// Clippers are required to not change the pointers to the entry they are clipping on.
/// Since `uvm_map_splitentry` turns the original entry into the lowest entry (address
/// wise) we do a swap between the new entry and the original entry, prior to calling
/// `uvm_map_splitentry`: the original's box becomes the upper part, whose key is returned.
fn uvm_map_clip_start(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    addr: usize,
) -> usize {
    // Unlink original.
    let owners = st.owners(map);
    let free = {
        let mut fp = st.free_parts(owners);
        let e = ent(fp.tree, key, "uvm_map_clip_start");
        let free = owners.uaddr_e(e);
        uvm_mapent_free_remove(&mut fp, ew, free, e);
        free
    };
    let entry = uvm_mapent_addr_remove(st, key);

    // Copy entry.
    kassert!(entry.start() < addr && vmmap_free_end(&entry) > addr);
    let Some(mut tmp) = uvm_mapent_alloc(map, 0) else {
        kpanic(format_args!("uvm_map_clip_start: no map entry"));
    };
    uvm_mapent_copy(&entry, ew, &mut tmp);

    // Put new entry in place of original entry.
    let tkey = uvm_mapent_addr_insert(map, st, tmp);
    {
        let mut fp = st.free_parts(owners);
        let t = ent(fp.tree, tkey, "uvm_map_clip_start");
        uvm_mapent_free_insert(&mut fp, ew, free, t);
    }

    // Invoke splitentry.
    uvm_map_splitentry(map, st, ew, tkey, entry, addr);
    addr
}

/// `uvm_map_boundfix`: boundary fixer.
#[inline]
fn uvm_map_boundfix(min: usize, max: usize, bound: usize) -> usize {
    if min < bound && max > bound {
        bound
    } else {
        max
    }
}

/// `uvm_map_vmspace_update`: update map allocation start and end addresses from proc
/// vmspace.
fn uvm_map_vmspace_update(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    dead: &mut DeadQueue,
    flags: u32,
) {
    // Derive actual allocation boundaries from vmspace.
    let vm = map.vm();
    let b_start = vm.vm_daddr.load(Mem::Relaxed);
    let b_end = b_start + BRKSIZ;
    let (maxs, mins) = (
        vm.vm_maxsaddr.load(Mem::Relaxed),
        vm.vm_minsaddr.load(Mem::Relaxed),
    );
    let s_start = maxs.min(mins);
    let s_end = maxs.max(mins);
    if cfg!(feature = "diagnostic")
        && (b_start & PAGE_MASK != 0
            || b_end & PAGE_MASK != 0
            || s_start & PAGE_MASK != 0
            || s_end & PAGE_MASK != 0)
    {
        kpanic(format_args!(
            "uvm_map_vmspace_update: vmspace {:#x} invalid bounds: b={b_start:#x}-{b_end:#x} s={s_start:#x}-{s_end:#x}",
            map.addr()
        ));
    }

    if st.b_start == b_start && st.b_end == b_end && st.s_start == s_start && st.s_end == s_end {
        return;
    }

    uvm_map_freelist_update(map, st, ew, dead, b_start, b_end, s_start, s_end, flags);
}

/// `uvm_map_kmem_grow`: grow kernel memory.
///
/// This function is only called for kernel maps when an allocation fails.
///
/// If the map has a gap that is large enough to accommodate alloc_sz, this function will
/// make sure `map->free` will include it.
fn uvm_map_kmem_grow(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    dead: &mut DeadQueue,
    alloc_sz: usize,
    flags: u32,
) {
    // Kernel memory only.
    kassert!(!map.is_vmspace());
    // Destroy free list.
    uvm_map_freelist_update_clear(map, st, ew, dead);

    // Include the guard page in the hard minimum requirement of alloc_sz.
    let alloc_sz = if map.flags() & VM_MAP_GUARDPAGES != 0 {
        alloc_sz + PAGE_SIZE
    } else {
        alloc_sz
    };

    // Grow by ALLOCMUL * alloc_sz, but at least VM_MAP_KSIZE_DELTA.
    //
    // Don't handle the case where the multiplication overflows: if that happens, the
    // allocation is probably too big anyway.
    let sz = VM_MAP_KSIZE_ALLOCMUL
        .wrapping_mul(alloc_sz)
        .max(VM_MAP_KSIZE_DELTA);

    // Walk forward until a gap large enough for alloc_sz shows up.
    //
    // We assume the kernel map has no boundaries.
    let mut end = uvm_maxkaddr().max(map.min_offset());
    let mut entry = uvm_map_entrybyaddr(&st.addr, end);
    while let Some(e) = entry.filter(|e| e.fspace() < alloc_sz) {
        entry = st.addr.next(e);
    }
    if let Some(e) = entry {
        end = vmmap_free_start(e).max(end);
        end += sz.min(map.max_offset() - end);
    } else {
        end = map.max_offset();
    }

    // Reserve pmap entries (PMAP_GROWKERNEL is defined on amd64 and arm64).
    UVM_MAXKADDR.store(
        hal::pmap::growkernel(Vaddr::new(end)).as_usize(),
        Mem::Relaxed,
    );

    // Rebuild free list.
    uvm_map_freelist_update_refill(map, st, ew, flags);
}

/// `uvm_map_freelist_update_clear`: freelist update subfunction: unlink all entries from
/// freelists.
fn uvm_map_freelist_update_clear(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    dead: &mut DeadQueue,
) {
    let owners = st.owners(map);
    let mut prev: Option<usize> = None;
    let mut iter = st.addr.min().map(VmMapEntry::start);
    while let Some(key) = iter {
        // One lookup per entry: the tree changes only when a free-only entry leaves it.
        let remove = {
            let mut fp = st.free_parts(owners);
            let tree = fp.tree;
            let e = ent(tree, key, "uvm_map_freelist_update_clear");
            iter = tree.next(e).map(VmMapEntry::start);

            uvm_mapent_free_remove(&mut fp, ew, owners.uaddr_e(e), e);

            match prev {
                Some(p) if e.start() == e.end() => {
                    let add = vmmap_free_end(e) - e.end();
                    let pe = ent(tree, p, "uvm_map_freelist_update_clear");
                    pe.set_fspace(ew, pe.fspace() + add);
                    true
                }
                _ => {
                    prev = Some(key);
                    false
                }
            }
        };
        if remove {
            dead.push(uvm_mapent_addr_remove(st, key));
        }
    }
}

/// `uvm_map_freelist_update_refill`: freelist update subfunction: refill the freelists
/// with entries.
fn uvm_map_freelist_update_refill(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    flags: u32,
) {
    // RBT_FOREACH, where the body replaces `entry` by what uvm_map_fix_space returns and
    // the next step continues from there.
    let mut iter = st.addr.min().map(VmMapEntry::start);
    while let Some(key) = iter {
        let e = ent(&st.addr, key, "uvm_map_freelist_update_refill");
        let min = vmmap_free_start(e);
        let max = vmmap_free_end(e);
        e.set_fspace(ew, 0);

        let last = uvm_map_fix_space(map, st, ew, Some(key), min, max, flags);
        iter = last.and_then(|l| next_key(&st.addr, l));
    }

    if cfg!(test) {
        uvm_tree_sanity(map, st, "uvm_map_freelist_update_refill");
    }
}

/// `uvm_map_freelist_update`: change `{a,b}_{start,end}` allocation ranges and associated
/// free lists.
#[allow(clippy::too_many_arguments)] // the C's signature
fn uvm_map_freelist_update(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    dead: &mut DeadQueue,
    b_start: usize,
    b_end: usize,
    s_start: usize,
    s_end: usize,
    flags: u32,
) {
    kdassert!(b_end >= b_start && s_end >= s_start);

    // Clear all free lists.
    uvm_map_freelist_update_clear(map, st, ew, dead);

    // Apply new bounds.
    st.b_start = b_start;
    st.b_end = b_end;
    st.s_start = s_start;
    st.s_end = s_end;

    // Refill free lists.
    uvm_map_freelist_update_refill(map, st, ew, flags);
}

/// `uvm_map_set_uaddr`: assign a selector to the slot `which` of the map (the old one
/// destroyed).
///
/// May sleep.
pub fn uvm_map_set_uaddr(map: &VmMap, which: Slot, newval: Option<Selector>) {
    let mut dead = DeadQueue::new();
    {
        let mut g = map.lock();
        let mut sp = g.split();
        let (st, ew) = sp.parts();
        uvm_map_freelist_update_clear(map, st, ew, &mut dead);

        uvm_addr_destroy(core::mem::replace(st.slot_mut(which), newval));

        uvm_map_freelist_update_refill(map, st, ew, 0);
    }
    uvm_unmap_detach(&mut dead, 0);
}

/// `uvm_map_fix_space`: correct space insert.
///
/// Entry (its key) must not be on any freelist. Returns the entry that describes the end of
/// the range (the one that took the last of the free space).
///
/// The entry being filled is held by reference from one step to the next and found again
/// (by its key) only after a new entry went into the tree.
fn uvm_map_fix_space(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    entry: Option<usize>,
    min: usize,
    max: usize,
    flags: u32,
) -> Option<usize> {
    let owners = st.owners(map);
    let mut fp = st.free_parts(owners);
    let mut cur = entry.map(|k| ent(fp.tree, k, "uvm_map_fix_space"));

    kassert!(cur.is_none_or(|e| e.etype() & UVM_ET_FREEMAPPED == 0));
    kdassert!(min <= max);
    kdassert!(cur.is_some_and(|e| vmmap_free_end(e) == min) || min == map.min_offset());

    // During the function, entfree will always point at the uaddr state for entry.
    let mut entfree = cur.and_then(|e| owners.uaddr_e(e));

    let mut min = min;
    while min != max {
        if let Some(e) = cur {
            // Claim guard page for entry.
            if map.flags() & VM_MAP_GUARDPAGES != 0
                && vmmap_free_end(e) == e.end()
                && e.start() != e.end()
            {
                if max - min == 2 * PAGE_SIZE {
                    // If the free-space gap is exactly 2 pages, we make the guard 2 pages
                    // instead of 1. Because in a guarded map, an area needs at least 2
                    // pages to allocate from: one page for the allocation and one for the
                    // guard.
                    e.set_guard(ew, 2 * PAGE_SIZE);
                    min = max;
                } else {
                    e.set_guard(ew, PAGE_SIZE);
                    min += PAGE_SIZE;
                }
                continue;
            }

            // Handle the case where entry has a 2-page guard, but the space after entry is
            // freed.
            if e.fspace() == 0 && e.guard() > PAGE_SIZE {
                e.set_guard(ew, PAGE_SIZE);
                min = vmmap_free_start(e);
            }
        }

        let lmax = owners.boundary(min, max);
        let free = owners.uaddr(min);

        // Entries are merged if they point at the same uvm_free(). Exception to that rule:
        // if min == uvm_maxkaddr, a new entry is started regardless (otherwise the
        // allocators will get confused).
        let merge =
            cur.filter(|_| free == entfree && !(!map.is_vmspace() && min == uvm_maxkaddr()));
        let reuse = cur.filter(|e| e.start() == min && e.end() == min && e.fspace() == 0);
        if let Some(e) = merge {
            kdassert!(vmmap_free_end(e) == min);
            e.set_fspace(ew, e.fspace() + (lmax - min));
        } else if let Some(e) = reuse {
            // A free-only entry that already sits at min (a kernel map's first entry when
            // nothing is mapped below uvm_maxkaddr, see the module's deviations): it is the
            // new entry the C would allocate, which would collide with it.
            e.set_fspace(ew, lmax - min);
            entfree = free;
        } else {
            // Commit entry to free list: it'll not be added to anymore. We'll start a new
            // entry and add to that entry instead.
            if let Some(e) = cur {
                uvm_mapent_free_insert(&mut fp, ew, entfree, e);
            }

            // New entry for new uaddr.
            let Some(mut ne) = uvm_mapent_alloc(map, flags) else {
                kpanic(format_args!("uvm_map_fix_space: no map entry"));
            };
            ne.set_end(ew, min);
            ne.set_start(ew, min);
            ne.set_guard(ew, 0);
            ne.set_fspace(ew, lmax - min);
            ne.set_offset(ew, 0);
            ne.set_etype(ew, 0);
            ne.set_protection(ew, 0);
            ne.set_max_protection(ew, 0);
            ne.set_inheritance(ew, 0);
            ne.set_wired_count(ew, 0);
            ne.set_advice(ew, 0);
            *ne.handles_owned() = Handles::default();
            let k = uvm_mapent_addr_insert(map, st, ne);

            // The tree changed: the new entry is found again by its key.
            fp = st.free_parts(owners);
            cur = Some(ent(fp.tree, k, "uvm_map_fix_space"));
            entfree = free;
        }

        min = lmax;
    }
    // Finally put entry on the uaddr state.
    if let Some(e) = cur {
        uvm_mapent_free_insert(&mut fp, ew, entfree, e);
    }

    cur.map(VmMapEntry::start)
}

/// `uvm_map_mquery`: MQuery style of allocation.
///
/// This allocator searches forward until sufficient space is found to map the given size.
///
/// XXX: factor in offset (via pmap_prefer) and protection?
pub fn uvm_map_mquery(
    map: &VmMap,
    addr_p: &mut usize,
    sz: usize,
    offset: Voff,
    flags: u32,
) -> Result<(), Errno> {
    let mut addr = *addr_p;
    let g = map.lock_read();

    let error = uvm_map_mquery_locked(map, g.state(), &mut addr, sz, offset, flags);

    drop(g);
    if error.is_ok() {
        *addr_p = addr;
    }
    error
}

/// [`uvm_map_mquery`] with the map read-locked (the C's body up to the `out` label).
fn uvm_map_mquery_locked(
    map: &VmMap,
    st: &MapState,
    addr: &mut usize,
    sz: usize,
    offset: Voff,
    flags: u32,
) -> Result<(), Errno> {
    // Configure pmap prefer (PMAP_PREFER is not defined: PAGE_SIZE and 0).
    let (pmap_align, pmap_offset) = (PAGE_SIZE, 0);

    // Align address to pmap_prefer unless FLAG_FIXED is set.
    if flags & UVM_FLAG_FIXED == 0 && offset != UVM_UNKNOWN_OFFSET {
        let mut tmp = (*addr & !(pmap_align - 1)) | pmap_offset;
        if tmp < *addr {
            tmp += pmap_align;
        }
        *addr = tmp;
    }

    let atree = &st.addr;
    let owners = st.owners(map);

    // First, check if the requested range is fully available.
    let mut entry = uvm_map_entrybyaddr(atree, *addr);
    let mut last = None;
    if uvm_map_isavail(map, atree, &owners, None, &mut entry, &mut last, *addr, sz) {
        return Ok(());
    }
    if flags & UVM_FLAG_FIXED != 0 {
        return Err(Errno::EINVAL);
    }

    // ENOMEM is the default error from here.

    // At this point, the memory at <addr, sz> is not available. The reasons are:
    // [1] it's outside the map,
    // [2] it starts in used memory (and therefore needs to move toward the first free page
    //     in entry),
    // [3] it starts in free memory but bumps into used memory.
    //
    // Note that for case [2], the forward moving is handled by the for loop below.
    match entry {
        None => {
            // [1] Outside the map.
            if *addr >= map.max_offset() {
                return Err(Errno::ENOMEM);
            }
            entry = atree.min();
        }
        Some(e) if vmmap_free_start(e) <= *addr => {
            // [3] Bumped into used memory.
            entry = atree.next(e);
        }
        _ => {}
    }

    // Test if the next entry is sufficient for the allocation.
    'entries: while let Some(e) = entry {
        entry = atree.next(e);
        if e.fspace() == 0 {
            continue;
        }
        *addr = vmmap_free_start(e);

        // restart: Restart address checks on address change.
        loop {
            let mut tmp = (*addr & !(pmap_align - 1)) | pmap_offset;
            if tmp < *addr {
                tmp += pmap_align;
            }
            *addr = tmp;
            if *addr >= vmmap_free_end(e) {
                continue 'entries;
            }

            // Skip brk() allocation addresses.
            if *addr + sz > st.b_start && *addr < st.b_end {
                if vmmap_free_end(e) > st.b_end {
                    *addr = st.b_end;
                    continue;
                } else {
                    continue 'entries;
                }
            }
            // Skip stack allocation addresses.
            if *addr + sz > st.s_start && *addr < st.s_end {
                if vmmap_free_end(e) > st.s_end {
                    *addr = st.s_end;
                    continue;
                } else {
                    continue 'entries;
                }
            }
            break;
        }

        let mut first = Some(e);
        let mut last = None;
        if uvm_map_isavail(map, atree, &owners, None, &mut first, &mut last, *addr, sz) {
            return Ok(());
        }
    }

    Err(Errno::ENOMEM)
}

// uvm_map_fill_vmmap: kinfo_vmentry (sysctl), see the module's deviations.

/// `uvm_map_setup_md`: MD code: vmspace allocator setup (the `__LP64__` version), into the
/// state of a map of `[min, max)`.
fn uvm_map_setup_md(st: &mut MapState, min: usize, max: usize) {
    // Ensure the selectors will not try to manage page 0; it's too special.
    let min = min.max(VMMAP_MIN_ADDR);

    // "Cool stuff, not yet": the pivot selector is #if 0; "Crappy stuff, for now":
    st.uaddr_any[0] = Some(uaddr_rnd_create(min, max));

    // SMALL_KERNEL is not configured.
    st.uaddr_brk_stack = Some(uaddr_stack_brk_create(min, max));
}

/// `amap_copy`: ensure that the entry at `key`'s "needs_copy" flag is false by copying the
/// amap if necessary; the key of the entry afterwards (a chunking clip may move it).
///
/// An entry with a null amap pointer will get a new (blank) one. The map that the map entry
/// belongs to must be locked by caller (its write lock: `st`, `ew`). The amap currently
/// attached to "entry" (if any) must be unlocked. If canchunk is true, then we may clip the
/// entry into a chunk. "startva" and "endva" are used only if canchunk is true. They are
/// used to limit chunking (e.g. if you have a large space that you know you are going to
/// need to allocate amaps for, there is no point in allowing that to be chunked).
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn amap_copy(
    map: &VmMap,
    st: &mut MapState,
    ew: &mut EntryWrite<'_>,
    key: usize,
    waitf: i32,
    canchunk: bool,
    startva: usize,
    endva: usize,
) -> usize {
    let mut key = key;
    let mut lazyalloc = false;

    // KASSERT(map != kernel_map): we use sleeping locks (the kernel map, M7a part 2).

    // Is there an amap to copy? If not, create one.
    let e = ent(&st.addr, key, "amap_copy");
    let src = e.handles(ew.witness()).amap.clone();
    let Some(srcamap) = src else {
        // Check to see if we have a large amap that we can chunk. We align startva/endva
        // to chunk-sized boundaries and then clip to them.
        //
        // If we cannot chunk the amap, allocate it in a way that makes it grow or shrink
        // dynamically with the number of slots.
        if atop(e.end() - e.start()) >= UVM_AMAP_LARGE {
            if canchunk {
                // convert slots to bytes
                let chunksize = UVM_AMAP_CHUNK << PAGE_SHIFT;
                let startva = (startva / chunksize) * chunksize;
                // roundup(endva, chunksize), wrapping as the C's unsigned arithmetic does.
                let endva = (endva.wrapping_add(chunksize - 1) / chunksize) * chunksize;
                key = uvm_map_clip_start_at(map, st, ew, key, startva);
                // watch out for endva wrap-around!
                if endva >= startva {
                    uvm_map_clip_end_at(map, st, ew, key, endva);
                }
            } else {
                lazyalloc = true;
            }
        }

        let e = ent(&st.addr, key, "amap_copy");
        let amap = amap_alloc(e.end() - e.start(), waitf, lazyalloc);
        let made = amap.is_some();
        {
            let h = e.handles_mut(ew);
            h.pageoff = 0;
            h.amap = amap;
        }
        if made {
            e.etype_clear(ew, UVM_ET_NEEDSCOPY);
        }
        return key;
    };

    // The amap's half: take the source over when the entry holds its only reference, else
    // copy its slots (sharing the anons and the lock) and drop the entry's reference to it.
    let slots = (e.end() - e.start()) >> PAGE_SHIFT;
    let pageoff = e.handles(ew.witness()).pageoff;
    match amap_copy_from(&srcamap, pageoff, slots, waitf) {
        // Just take over the existing amap.
        CopyOutcome::TakeOver => e.etype_clear(ew, UVM_ET_NEEDSCOPY),
        // Install new amap (the entry's handle to the source goes with the old value).
        CopyOutcome::New(amap) => {
            let old = {
                let h = e.handles_mut(ew);
                h.pageoff = 0;
                h.amap.replace(amap)
            };
            drop(old);
            e.etype_clear(ew, UVM_ET_NEEDSCOPY);
        }
        // No memory: the entry still needs a copy, as the C leaves it.
        CopyOutcome::NoMemory => {}
    }
    key
}

// The map's state and every entry's handles are cells of different parts of the lock.
const _: () = assert!(STATE_PART != ENTRY_PART);

/// A lone entry of `[start, start)` with `fspace` bytes after it, on no map (the selectors'
/// tests of their orders).
#[cfg(test)]
pub(super) fn test_entry(start: usize, fspace: usize) -> VmMapEntry {
    static LOCK: RawRwlock = RawRwlock::new("testlk");
    let e = VmMapEntry::new(LOCK.id(), 0);
    e.start.store(start, Mem::Relaxed);
    e.end.store(start, Mem::Relaxed);
    e.fspace.store(fspace, Mem::Relaxed);
    e
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests for the map operations over the kernel's page system (`testutil`): the map,
    // the entry tree, the selectors, the lock and its busy protocol, the vmspace life cycle,
    // and the fault handler over the real map; the tree checks of `VMMAP_DEBUG` run in them.

    use std::sync::MutexGuard;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne};

    use hal::arch::current::vmparam::{MAXSSIZ, USRSTACK, VM_MAXUSER_ADDRESS};
    use hal::sys::types::Vsize;
    use hal::vm::uvmexp::{UvmExpCounters, counters_read};

    use super::*;
    use crate::testutil;
    use crate::vm::addr::{uaddr_bestfit_create, uaddr_pivot_create};
    use crate::vm::amap::amap_init;
    use crate::vm::anon::{AnonRef, uvm_anon_init};
    use crate::vm::aobj::{uao_create, uao_init};
    use crate::vm::fault::{
        VM_FAULT_INVALID, uvm_fault, uvm_fault_unwire, uvm_fault_wire, uvmfault_init,
    };
    use crate::vm::page::{PG_BUSY, PG_CLEAN, PG_FAKE, PHYS_TO_VM_PAGE};

    const RW: VmProt = PROT_READ | PROT_WRITE;
    const RWX: VmProt = PROT_READ | PROT_WRITE | PROT_EXEC;

    /// Real page memory, the lock objects, the anon, amap and aobj pools, the pmaps and the
    /// map pools.
    fn setup() -> MutexGuard<'static, ()> {
        let guard = testutil::pages_install(1024 * 4096);
        testutil::init_shared_once();
        let _ = uvm_anon_init(0);
        let _ = amap_init(0);
        let _ = uao_init();
        hal::pmap::init();
        uvm_map_init();
        uvmfault_init();
        guard
    }

    /// A vmspace laid out as `exec` leaves one: data at 256 MiB, the stack below `USRSTACK`.
    fn user_vmspace() -> VmspaceRef {
        let vm = uvmspace_alloc(VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS, true, false);
        vm.vm_daddr.store(256 << 20, Mem::Relaxed);
        vm.vm_minsaddr.store(USRSTACK, Mem::Relaxed);
        vm.vm_maxsaddr.store(USRSTACK - MAXSSIZ, Mem::Relaxed);
        vm
    }

    /// `uvm_mapanon` flags for a private read-write mapping, `extra` on top.
    fn anon_flags(extra: u32) -> u32 {
        uvm_mapflag(
            RW,
            RWX,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_COPYONW | extra,
        )
    }

    /// The entry that holds `addr`, under the read lock.
    fn entry_holding(map: &VmMap, addr: usize) -> Option<(usize, usize, VmProt, i32)> {
        let g = map.lock_read();
        g.lookup(addr)
            .map(|e| (e.start(), e.end(), e.protection(), e.etype()))
    }

    /// The map's checks of `VMMAP_DEBUG` and the augment of every node.
    fn sane(map: &VmMap) {
        let g = map.lock_read();
        let st = g.state();
        uvm_tree_sanity(map, st, "test");
        uvm_tree_size_chk(map, st, g.witness(), "test");
        for e in st.addr.iter() {
            let mut want = e.fspace();
            if let Some(l) = st.addr.left(e) {
                want = want.max(l.fspace_augment());
            }
            if let Some(r) = st.addr.right(e) {
                want = want.max(r.fspace_augment());
            }
            assert_eq!(e.fspace_augment(), want, "the augment at {:#x}", e.start());
        }
    }

    /// A kernel-style map of 1 GiB at the start of kernel space, `uvm_maxkaddr` there.
    fn kernel_style_map(flags: i32) -> &'static VmMap {
        let min = VM_MIN_KERNEL_ADDRESS;
        UVM_MAXKADDR.store(min, Mem::Relaxed);
        uvm_map_create(PmapRef::Kernel, min, min + (1 << 30), flags)
    }

    #[test]
    fn setup_entries_cover_the_map() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        sane(map);
        assert_eq!(map.size(), 0);
        let g = map.lock_read();
        assert!(g.state().addr.min().is_some());
        assert!(g.state().uaddr_any[0].is_some(), "the rnd selector");
        assert!(
            g.state().uaddr_brk_stack.is_some(),
            "the stack/brk selector"
        );
        drop(g);
        uvmspace_free(vm);
    }

    #[test]
    fn concurrent_readers_always_see_the_object() {
        // Faults on one entry read its object under the map's read lock at once: every
        // reader sees the handle (the handles are a cell of the lock, read shared).
        let _g = setup();
        let vm = user_vmspace();
        let uobj = uao_create(Vsize::new(PAGE_SIZE), 0).expect("an aobj");
        let mut va = 0x2000_0000;
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_SHARE, MADV_NORMAL, UVM_FLAG_FIXED);
        uvm_map(
            &vm.vm_map,
            &mut va,
            PAGE_SIZE,
            Some(uobj.clone()),
            0,
            0,
            flags,
        )
        .expect("map");
        let readers: Vec<_> = (0..4)
            .map(|_| {
                let vm = vm.clone();
                let want = uobj.clone();
                thread::spawn(move || {
                    (0..2_000).all(|_| {
                        let g = vm.vm_map.lock_read();
                        g.lookup(va)
                            .and_then(|e| g.object(e))
                            .is_some_and(|o| ObjRef::ptr_eq(&o, &want))
                    })
                })
            })
            .collect();
        for r in readers {
            assert!(r.join().expect("a reader"), "a reader saw no object");
        }
        uvm_unmap(&vm.vm_map, va, va + PAGE_SIZE);
        drop(uobj);
        uvmspace_free(vm);
    }

    #[test]
    fn mapanon_fixed_then_unmap() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut addr = 0x2000_0000;
        let len = 4 * PAGE_SIZE;

        uvm_mapanon(map, &mut addr, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");
        assert_eq!(addr, 0x2000_0000);
        assert_eq!(map.size(), len);
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 4);
        let (start, end, prot, etype) = entry_holding(map, addr + PAGE_SIZE).expect("mapped");
        assert_eq!((start, end, prot), (addr, addr + len, RW));
        assert_ne!(etype & UVM_ET_COPYONWRITE, 0);
        assert_ne!(etype & UVM_ET_NEEDSCOPY, 0);
        assert!(
            entry_holding(map, addr + len).is_none(),
            "nothing past the end"
        );
        sane(map);

        // The same range again is not available.
        let mut again = addr;
        assert_eq!(
            uvm_mapanon(map, &mut again, len, 0, anon_flags(UVM_FLAG_FIXED)),
            Err(Errno::ENOMEM)
        );

        uvm_unmap(map, addr, addr + len);
        assert_eq!(map.size(), 0);
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 0);
        assert!(entry_holding(map, addr).is_none());
        sane(map);
        uvmspace_free(vm);
    }

    #[test]
    fn mapanon_finds_space_and_honours_a_hint() {
        let _g = setup();
        testutil::set_random_seed(7);
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let len = 8 * PAGE_SIZE;

        let mut a = 0;
        uvm_mapanon(map, &mut a, len, 0, anon_flags(0)).expect("first");
        assert_eq!(a & PAGE_MASK, 0);
        assert!(a >= VM_MIN_ADDRESS);
        assert!(a + len <= VM_MAXUSER_ADDRESS);

        let mut b = 0;
        uvm_mapanon(map, &mut b, len, 0, anon_flags(0)).expect("second");
        assert!(b + len <= a || a + len <= b, "no overlap: {a:#x} {b:#x}");

        // A free, aligned address is taken as given.
        let mut c = 0x3000_0000;
        uvm_mapanon(map, &mut c, len, 0, anon_flags(0)).expect("hinted");
        assert_eq!(c, 0x3000_0000);

        // An alignment request is honoured.
        let mut d = 0;
        uvm_mapanon(map, &mut d, len, 1 << 21, anon_flags(0)).expect("aligned");
        assert_eq!(d & ((1 << 21) - 1), 0);

        assert_eq!(map.size(), 4 * len);
        let g = map.lock_read();
        assert!(uvm_map_checkprot(&g, c, c + len, PROT_READ));
        assert!(!uvm_map_checkprot(&g, c, c + len, PROT_EXEC));
        assert!(
            !uvm_map_checkprot(&g, c, c + 2 * len, PROT_READ),
            "a hole fails"
        );
        drop(g);
        sane(map);
        uvmspace_free(vm);
    }

    #[test]
    fn the_randomised_addresses_differ_with_the_draws() {
        // uvm_map_hint's spacing comes from arc4random: two seeds, two layouts (ASLR).
        let _g = setup();
        let len = 8 * PAGE_SIZE;
        let first_of = |seed| {
            testutil::set_random_seed(seed);
            let vm = user_vmspace();
            let mut a = 0;
            uvm_mapanon(&vm.vm_map, &mut a, len, 0, anon_flags(0)).expect("mapanon");
            uvmspace_free(vm);
            a
        };
        let (a, b) = (first_of(1), first_of(0x5eed_5eed));
        assert_eq!(first_of(1), a, "the same draws give the same address");
        assert_ne!(a, b);
    }

    #[test]
    fn protect_clips_and_checks_limits() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut addr = 0x4000_0000;
        let len = 4 * PAGE_SIZE;
        uvm_mapanon(map, &mut addr, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");

        // Beyond max_protection.
        assert_eq!(
            uvm_map_protect(map, addr, addr + len, RWX | 0x8, 0, false, true),
            Err(Errno::EACCES)
        );

        // The middle two pages become read-only: three entries.
        uvm_map_protect(
            map,
            addr + PAGE_SIZE,
            addr + 3 * PAGE_SIZE,
            PROT_READ,
            0,
            false,
            true,
        )
        .expect("protect");
        let (s0, e0, p0, _) = entry_holding(map, addr).expect("first");
        let (s1, e1, p1, _) = entry_holding(map, addr + PAGE_SIZE).expect("middle");
        let (s2, e2, p2, _) = entry_holding(map, addr + 3 * PAGE_SIZE).expect("last");
        assert_eq!((s0, e0, p0), (addr, addr + PAGE_SIZE, RW));
        assert_eq!(
            (s1, e1, p1),
            (addr + PAGE_SIZE, addr + 3 * PAGE_SIZE, PROT_READ)
        );
        assert_eq!((s2, e2, p2), (addr + 3 * PAGE_SIZE, addr + len, RW));
        assert_eq!(map.size(), len);
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 4);

        // PROT_NONE drops the pages from vm_dused; back to RW restores them.
        uvm_map_protect(map, addr, addr + len, PROT_NONE, 0, false, true).expect("none");
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 0);
        uvm_map_protect(map, addr, addr + len, RW, 0, false, true).expect("rw");
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 4);
        sane(map);

        uvmspace_free(vm);
    }

    #[test]
    fn protect_out_of_prot_none_keeps_to_the_data_limit() {
        // The C's RLIMIT_DATA check (LZ's unported!): PROT_NONE pages made accessible count
        // against lim_cur(RLIMIT_DATA) with what vm_dused already holds.
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut addr = 0x4400_0000;
        let len = 4 * PAGE_SIZE;
        uvm_mapanon(map, &mut addr, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");
        uvm_map_protect(map, addr, addr + len, PROT_NONE, 0, false, true).expect("none");

        testutil::vmproc_with(|p| {
            p.limits
                .insert(crate::vm::proc::RLIMIT_DATA, 3 * PAGE_SIZE as u64);
        });
        assert_eq!(
            uvm_map_protect(map, addr, addr + len, RW, 0, false, true),
            Err(Errno::ENOMEM)
        );
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 0, "nothing changed");
        testutil::vmproc_with(|p| {
            p.limits
                .insert(crate::vm::proc::RLIMIT_DATA, 4 * PAGE_SIZE as u64);
        });
        uvm_map_protect(map, addr, addr + len, RW, 0, false, true).expect("within the limit");
        assert_eq!(vm.vm_dused.load(Mem::Relaxed), 4);
        testutil::vmproc_with(|p| {
            p.limits.remove(&crate::vm::proc::RLIMIT_DATA);
        });
        uvmspace_free(vm);
    }

    #[test]
    fn immutable_entries_refuse_changes() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut addr = 0x5000_0000;
        let len = 2 * PAGE_SIZE;
        uvm_mapanon(map, &mut addr, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");
        uvm_map_immutable(map, addr, addr + len, true).expect("immutable");

        assert_eq!(
            uvm_map_protect(map, addr, addr + len, PROT_READ, 0, false, true),
            Err(Errno::EPERM)
        );
        assert_eq!(
            uvm_map_inherit(map, addr, addr + len, MAP_INHERIT_SHARE),
            Err(Errno::EPERM)
        );
        let mut over = addr;
        assert_eq!(
            uvm_mapanon(
                map,
                &mut over,
                len,
                0,
                anon_flags(UVM_FLAG_FIXED | UVM_FLAG_UNMAP)
            ),
            Err(Errno::EPERM)
        );

        // Without the check the entries go away as usual.
        uvm_unmap(map, addr, addr + len);
        assert_eq!(map.size(), 0);
        uvmspace_free(vm);
    }

    #[test]
    fn inherit_advice_and_fork() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let len = 2 * PAGE_SIZE;
        let (mut copy, mut share, mut none, mut overlay) =
            (0x6000_0000, 0x6100_0000, 0x6200_0000, 0x6300_0000);
        uvm_mapanon(map, &mut copy, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("copy");
        uvm_mapanon(map, &mut share, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("share");
        uvm_mapanon(map, &mut none, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("none");
        uvm_mapanon(
            map,
            &mut overlay,
            len,
            0,
            anon_flags(UVM_FLAG_FIXED | UVM_FLAG_OVERLAY),
        )
        .expect("overlay");
        uvm_map_inherit(map, share, share + len, MAP_INHERIT_SHARE).expect("inherit share");
        uvm_map_inherit(map, none, none + len, MAP_INHERIT_NONE).expect("inherit none");
        assert_eq!(
            uvm_map_inherit(map, none, none + len, 7),
            Err(Errno::EINVAL)
        );
        uvm_map_advice(map, copy, copy + PAGE_SIZE, MADV_RANDOM).expect("advice");
        assert_eq!(
            uvm_map_advice(map, copy, copy + len, 99),
            Err(Errno::EINVAL)
        );
        assert_eq!(map.size(), 4 * len);

        let child = uvmspace_fork(&vm);
        let cmap = &child.vm_map;

        // copy, share and overlay are inherited; none is not.
        assert_eq!(cmap.size(), 3 * len);
        assert_eq!(child.vm_dused.load(Mem::Relaxed), 6);
        let (_, _, _, etype) = entry_holding(cmap, copy).expect("copy inherited");
        assert_ne!(etype & UVM_ET_NEEDSCOPY, 0);
        assert!(entry_holding(cmap, share).is_some(), "share inherited");
        assert!(entry_holding(cmap, none).is_none(), "none not inherited");
        assert!(entry_holding(cmap, overlay).is_some(), "overlay inherited");
        let child_amap = {
            let g = cmap.lock_read();
            g.lookup(overlay)
                .and_then(|e| e.handles(g.witness()).amap.clone())
        }
        .expect("the overlay's amap is shared with the child");
        assert_eq!(child_amap.refs(), 2);
        sane(cmap);

        uvmspace_free(child);
        assert_eq!(child_amap.refs(), 1, "the child's reference is gone");
        uvmspace_free(vm);
    }

    #[test]
    fn kernel_style_map_reserves_grows_and_allocates() {
        let _g = setup();
        let map = kernel_style_map(VM_MAP_PAGEABLE);
        let min = map.min_offset();
        let max = map.max_offset();

        assert!(
            map.lock_read().state().uaddr_any[3].is_some(),
            "the bootstrap selector"
        );
        let kflags = uvm_mapflag(RW, RW, MAP_INHERIT_NONE, MADV_RANDOM, UVM_FLAG_FIXED);

        // The bootstrap reservation, as uvm_km_init makes it, grows uvm_maxkaddr.
        let mut base = min;
        let reserved = 16 * PAGE_SIZE;
        uvm_map(
            map,
            &mut base,
            reserved,
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            kflags,
        )
        .expect("reserve");
        assert_eq!(base, min);
        assert!(uvm_maxkaddr() >= min + reserved);

        // A non-fixed allocation lands above it, through uaddr_kbootstrap.
        let mut va = 0;
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_NONE, MADV_RANDOM, 0);
        uvm_map(
            map,
            &mut va,
            4 * PAGE_SIZE,
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            flags,
        )
        .expect("alloc");
        assert!(va >= min + reserved && va + 4 * PAGE_SIZE <= max, "{va:#x}");
        assert_eq!(map.size(), reserved + 4 * PAGE_SIZE);

        // W^X is refused on the kernel map only; this map is not kernel_map.
        assert!(!is_kernel_map(map));
        sane(map);

        uvm_unmap(map, va, va + 4 * PAGE_SIZE);
        assert_eq!(map.size(), reserved);
        uvm_map_deallocate(map);
    }

    #[test]
    fn bestfit_switch_on_an_empty_kernel_map() {
        let _g = setup();
        let map = kernel_style_map(VM_MAP_PAGEABLE);
        let (min, max) = (map.min_offset(), map.max_offset());

        // uvm_init's switch, with nothing reserved below uvm_maxkaddr (amd64's case).
        uvm_map_set_uaddr(map, Slot::Any(3), Some(uaddr_bestfit_create(min, max)));
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_NONE, MADV_RANDOM, 0);
        let mut va = 0;
        uvm_map(
            map,
            &mut va,
            2 * PAGE_SIZE,
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            flags,
        )
        .expect("alloc");
        assert!(va >= min && va + 2 * PAGE_SIZE <= max, "{va:#x}");
        let mut vb = 0;
        uvm_map(
            map,
            &mut vb,
            2 * PAGE_SIZE,
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            flags,
        )
        .expect("alloc 2");
        assert!(vb + 2 * PAGE_SIZE <= va || va + 2 * PAGE_SIZE <= vb);
        assert!(uvm_maxkaddr() >= va + 2 * PAGE_SIZE);
        sane(map);
        uvm_unmap(map, va, va + 2 * PAGE_SIZE);
        uvm_unmap(map, vb, vb + 2 * PAGE_SIZE);
        assert_eq!(map.size(), 0);
        uvm_map_deallocate(map);
    }

    #[test]
    fn a_vmspace_with_a_bestfit_selector_tears_down() {
        // The free tree's links are counted in the entries: the teardown moves the tree onto
        // the dead queue, destroys the selectors (their links go) and only then frees.
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        uvm_map_set_uaddr(
            map,
            Slot::Any(1),
            Some(uaddr_bestfit_create(0x1_0000_0000, 0x2_0000_0000)),
        );
        let mut a = 0x1_2000_0000;
        uvm_mapanon(map, &mut a, 4 * PAGE_SIZE, 0, anon_flags(UVM_FLAG_FIXED)).expect("in it");
        {
            let g = map.lock_read();
            let st = g.state();
            let freemapped = st
                .addr
                .iter()
                .filter(|e| st.owners(map).uaddr_e(e) == Some(Slot::Any(1)))
                .all(|e| e.etype() & UVM_ET_FREEMAPPED != 0);
            assert!(
                freemapped,
                "the bestfit selector owns its range's free space"
            );
        }
        sane(map);
        uvmspace_free(vm);
    }

    #[test]
    fn pivot_selector_allocations_do_not_overlap() {
        let _g = setup();
        testutil::set_random_seed(99);
        let vm = user_vmspace();
        let map = &vm.vm_map;
        // Above the brk area (vm_daddr + BRKSIZ), whose free space is the stack/brk
        // selector's.
        let (lo, hi) = (0x3_0000_0000, 0x3_4000_0000);
        uvm_map_set_uaddr(map, Slot::Any(0), Some(uaddr_pivot_create(lo, hi)));
        let mut got: Vec<(usize, usize)> = Vec::new();
        for _ in 0..24 {
            let mut a = 0;
            uvm_mapanon(map, &mut a, 3 * PAGE_SIZE, 0, anon_flags(0)).expect("a pivot address");
            assert!(
                a >= lo && a + 3 * PAGE_SIZE <= hi,
                "{a:#x} in the pivots' range"
            );
            for &(s, e) in &got {
                assert!(a + 3 * PAGE_SIZE <= s || e <= a, "{a:#x} overlaps {s:#x}");
            }
            got.push((a, a + 3 * PAGE_SIZE));
        }
        sane(map);
        for (s, e) in got {
            uvm_unmap(map, s, e);
        }
        sane(map);
        uvmspace_free(vm);
    }

    #[test]
    fn mquery_skips_used_space() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut addr = 0x7000_0000;
        let len = 4 * PAGE_SIZE;
        uvm_mapanon(map, &mut addr, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");

        let mut q = addr + PAGE_SIZE;
        uvm_map_mquery(map, &mut q, PAGE_SIZE, UVM_UNKNOWN_OFFSET, 0).expect("mquery");
        assert!(q >= addr + len, "{q:#x} is past the mapping");

        let mut fixed = addr + PAGE_SIZE;
        assert_eq!(
            uvm_map_mquery(
                map,
                &mut fixed,
                PAGE_SIZE,
                UVM_UNKNOWN_OFFSET,
                UVM_FLAG_FIXED
            ),
            Err(Errno::EINVAL)
        );
        uvmspace_free(vm);
    }

    #[test]
    fn pie_addresses_stay_in_range() {
        testutil::install();
        for _ in 0..64 {
            let addr = uvm_map_pie(1 << 21);
            assert_eq!(addr & ((1 << 21) - 1), 0);
            assert!((VM_PIE_MIN_ADDR..VM_PIE_MAX_ADDR + (1 << 21)).contains(&addr));
        }
    }

    #[test]
    fn exec_recycles_a_single_user_vmspace() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut addr = 0x2000_0000;
        uvm_mapanon(map, &mut addr, PAGE_SIZE, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");

        assert!(uvmspace_exec(&vm, PAGE_SIZE, 0x1000_0000).is_none());
        assert_eq!(map.min_offset(), PAGE_SIZE);
        assert_eq!(map.max_offset(), 0x1000_0000);
        assert_eq!(map.size(), 0);
        assert!(entry_holding(map, addr).is_none());
        let g = map.lock_read();
        uvm_tree_sanity(map, g.state(), "after exec");
        drop(g);
        uvmspace_free(vm);
    }

    #[test]
    fn exec_of_a_shared_vmspace_gives_a_new_one() {
        let _g = setup();
        let vm = user_vmspace();
        let mut addr = 0x2000_0000;
        uvm_mapanon(
            &vm.vm_map,
            &mut addr,
            PAGE_SIZE,
            0,
            anon_flags(UVM_FLAG_FIXED),
        )
        .expect("mapanon");
        let shared = uvmspace_share(&vm);
        assert_eq!(vm.vm_refcnt.load(Mem::Relaxed), 2);

        let nvm = uvmspace_exec(&vm, PAGE_SIZE, 0x1000_0000).expect("a new vmspace");
        assert!(!PoolArc::ptr_eq(&nvm, &vm));
        assert_eq!(nvm.vm_refcnt.load(Mem::Relaxed), 1);
        assert_eq!(nvm.vm_map.max_offset(), 0x1000_0000);
        assert!(
            entry_holding(&vm.vm_map, addr).is_some(),
            "the old one is the other user's, untouched"
        );
        uvmspace_free(nvm);
        uvmspace_free(shared);
        uvmspace_free(vm);
    }

    #[test]
    fn a_writer_waits_for_a_busy_map_and_a_reader_passes() {
        let _g = setup();
        let vm = user_vmspace();
        let busy = vm.vm_map.lock().into_busy();
        let v0 = vm.vm_map.timestamp();

        // Another thread's reader passes the busy map.
        let r = {
            let vm = vm.clone();
            thread::spawn(move || vm.vm_map.lock_read().version())
        };
        assert_eq!(r.join().expect("reader"), v0);

        // Another thread's writer waits until the map is unbusied.
        let locked = std::sync::Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let w = {
            let vm = vm.clone();
            let locked = locked.clone();
            thread::spawn(move || {
                tx.send(()).expect("started");
                let g = vm.vm_map.lock();
                locked.store(true, Mem::SeqCst);
                drop(g);
            })
        };
        rx.recv().expect("the writer started");
        thread::sleep(Duration::from_millis(50));
        assert!(
            !locked.load(Mem::SeqCst),
            "the writer waits on the busy map"
        );

        // The busy thread locks without waiting and without bumping the version.
        let g = busy.lock();
        assert_eq!(vm.vm_map.timestamp(), v0);
        drop(g);
        let g = busy.relock_unbusy();
        assert_eq!(
            vm.vm_map.timestamp(),
            v0,
            "the busy holder's lock is not a version"
        );
        drop(g);
        w.join().expect("writer");
        assert!(locked.load(Mem::SeqCst));
        assert_eq!(vm.vm_map.timestamp(), v0 + 1, "the other writer's lock is");
        uvmspace_free(vm);
    }

    #[test]
    fn relock_holds_only_while_the_version_is_the_same() {
        let _g = setup();
        let vm = user_vmspace();
        let v = vm.vm_map.lock_read().version();
        assert!(vm.vm_map.relock(v).is_some());
        drop(vm.vm_map.lock());
        assert!(
            vm.vm_map.relock(v).is_none(),
            "a write lock changed the version"
        );
        // An upgrade bumps it too; a failed try-lock does not.
        let v = vm.vm_map.timestamp();
        let r = vm.vm_map.lock_read();
        let other = vm.vm_map.lock_read();
        let r = match r.upgrade() {
            Ok(_) => panic!("upgraded beside another reader"),
            Err(r) => r,
        };
        assert_eq!(vm.vm_map.timestamp(), v);
        assert!(vm.vm_map.lock_try().is_none(), "readers hold it");
        drop(other);
        let w = r.upgrade().ok().expect("the only reader upgrades");
        assert_eq!(vm.vm_map.timestamp(), v + 1);
        drop(w);
        uvmspace_free(vm);
    }

    #[test]
    fn an_intrsafe_map_locks_its_mutex_and_counts_static_entries() {
        let _g = setup();
        let before = UVMEXP.kmapent.load(Mem::Relaxed);
        let map = kernel_style_map(VM_MAP_INTRSAFE);
        let after_setup = UVMEXP.kmapent.load(Mem::Relaxed);
        assert!(
            after_setup > before,
            "an INTRSAFE map's entries are the static ones"
        );
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_NONE, MADV_RANDOM, UVM_FLAG_FIXED);
        let mut va = map.min_offset();
        // An INTRSAFE map is used at IPL_VM (uvm_map's splassert).
        let spl = hal::intr::splraise(IPL_VM);
        uvm_map(
            map,
            &mut va,
            2 * PAGE_SIZE,
            None,
            UVM_UNKNOWN_OFFSET,
            0,
            flags,
        )
        .expect("map");
        {
            let g = map.lock_read();
            assert!(g.lookup(va).is_some());
            let e = g.lookup(va).map(|e| e.flags());
            assert_eq!(e, Some(UVM_MAP_STATIC));
        }
        uvm_unmap(map, va, va + 2 * PAGE_SIZE);
        drop(spl);
        uvm_map_deallocate(map);
        assert_eq!(
            UVMEXP.kmapent.load(Mem::Relaxed),
            before,
            "every static entry went back"
        );
    }

    #[test]
    fn mlock_all_respects_the_limits_and_wires_future_mappings() {
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut a = 0x2000_0000;
        uvm_mapanon(map, &mut a, 2 * PAGE_SIZE, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");
        let wiredmax = UVMEXP.wiredmax.load(Mem::Relaxed);
        UVMEXP.wiredmax.store(1 << 20, Mem::Relaxed);
        assert_eq!(
            uvm_map_pageable_all(map, MCL_CURRENT, PAGE_SIZE),
            Err(Errno::ENOMEM),
            "two pages over a one-page limit"
        );
        uvm_map_pageable_all(map, MCL_CURRENT | MCL_FUTURE, 0).expect("wire all");
        assert_ne!(map.flags() & VM_MAP_WIREFUTURE, 0);
        assert_eq!(entry_holding(map, a).map(|e| e.0), Some(a));
        {
            let g = map.lock_read();
            assert!(g.lookup(a).is_some_and(vm_mapent_iswired));
        }
        uvm_map_pageable_all(map, 0, 0).expect("unwire all");
        UVMEXP.wiredmax.store(wiredmax, Mem::Relaxed);
        assert_eq!(map.flags() & VM_MAP_WIREFUTURE, 0);
        {
            let g = map.lock_read();
            assert!(g.lookup(a).is_some_and(|e| !vm_mapent_iswired(e)));
        }
        uvmspace_free(vm);
    }

    /// A vmspace with `len` bytes of private anonymous read-write memory at `addr`.
    fn vmspace_with_anon(addr: usize, len: usize) -> VmspaceRef {
        let vm = user_vmspace();
        let mut a = addr;
        let flags = uvm_mapflag(
            RW,
            RW | PROT_EXEC,
            MAP_INHERIT_COPY,
            MADV_NORMAL,
            UVM_FLAG_FIXED | UVM_FLAG_COPYONW,
        );
        uvm_mapanon(&vm.vm_map, &mut a, len, 0, flags).expect("mapanon");
        vm
    }

    /// The anon the amap of `vm` holds at `va` (a handle), under the map's and the amap's
    /// locks.
    fn anon_of(vm: &Vmspace, va: usize) -> Option<AnonRef> {
        let guard = vm.vm_map.lock_read();
        let entry = guard.lookup(va)?;
        let (amap, pageoff) = FaultMapGuard::aref(&guard, entry)?;
        let w = amap.lock().read();
        amap_lookup(&amap, &w, pageoff, va - entry.start()).cloned()
    }

    fn same(a: &AnonRef, b: &AnonRef) -> bool {
        core::ptr::eq(&**a, &**b)
    }

    fn extract(map: &VmMap, va: usize) -> Option<hal::sys::types::Paddr> {
        map.pmap().extract(Vaddr::new(va))
    }

    #[test]
    fn zero_fill_fault_maps_a_fresh_anon_page() {
        let _g = setup();
        let va = 0x2000_0000;
        let vm = vmspace_with_anon(va, 4 * PAGE_SIZE);
        assert!(extract(&vm.vm_map, va).is_none());
        let faults = counters_read(UvmExpCounters::Faults);

        // A read fault: the lazy amap is made, an anon with a zeroed page is promoted and
        // mapped (case 2B), the neighbours are not (no anons yet).
        uvm_fault(&vm.vm_map, va + 8, VM_FAULT_INVALID, PROT_READ).expect("read fault");
        let pa = extract(&vm.vm_map, va).expect("mapped");
        let anon = anon_of(&vm, va).expect("an anon");
        let pg = anon.page().expect("its page");
        assert_eq!(pg.phys_addr(), pa.trunc_page());
        assert_ne!(pg.flags() & PQ_ANON, 0);
        assert_eq!(pg.flags() & (PG_BUSY | PG_FAKE), 0);
        assert_eq!(anon.refs(), 1);
        assert_eq!(counters_read(UvmExpCounters::Faults), faults + 1);

        // A write fault on the same page is case 1A: the same anon and page stay.
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("write fault");
        assert!(anon_of(&vm, va).is_some_and(|a| same(&a, &anon)));
        assert_eq!(extract(&vm.vm_map, va), Some(pa));

        // The next page faults in with its own anon; the first stays.
        uvm_fault(&vm.vm_map, va + PAGE_SIZE, VM_FAULT_INVALID, PROT_WRITE).expect("second page");
        let anon2 = anon_of(&vm, va + PAGE_SIZE).expect("second anon");
        assert!(!same(&anon, &anon2));
        drop((anon, anon2));
        uvmspace_free(vm);
    }

    #[test]
    fn faults_outside_or_beyond_protection_fail() {
        let _g = setup();
        let va = 0x3000_0000;
        let vm = vmspace_with_anon(va, PAGE_SIZE);

        // Free space: a case 0 fault.
        assert_eq!(
            uvm_fault(&vm.vm_map, va + 0x10_0000, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT)
        );
        // Outside the map.
        assert_eq!(
            uvm_fault(&vm.vm_map, 0, VM_FAULT_INVALID, PROT_READ),
            Err(Errno::EFAULT)
        );
        // More access than the entry allows.
        assert_eq!(
            uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_EXEC),
            Err(Errno::EACCES)
        );
        uvmspace_free(vm);
    }

    #[test]
    fn copy_on_write_after_fork_promotes_a_new_anon() {
        let _g = setup();
        let va = 0x4000_0000;
        let vm = vmspace_with_anon(va, PAGE_SIZE);
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("parent fault");
        let parent_anon = anon_of(&vm, va).expect("parent anon");
        let parent_pa = extract(&vm.vm_map, va).expect("parent pa");

        let child = uvmspace_fork(&vm);
        // The amap is shared and the anon has two references until the child writes.
        let parent_amap = {
            let guard = vm.vm_map.lock_read();
            guard
                .lookup(va)
                .and_then(|e| e.handles(guard.witness()).amap.clone())
        }
        .expect("parent amap");
        assert_eq!(parent_amap.refs(), 2);

        // The child's read fault sees the parent's page, read-only (case 1A, an_ref > 1).
        uvm_fault(&child.vm_map, va, VM_FAULT_INVALID, PROT_READ).expect("child read");
        assert_eq!(extract(&child.vm_map, va), Some(parent_pa));

        // The child's write fault copies the amap (needs_copy), then promotes a new anon with
        // a copy of the page (case 1B): the two maps now differ.
        uvm_fault(&child.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("child write");
        let child_anon = anon_of(&child, va).expect("child anon");
        assert!(!same(&child_anon, &parent_anon));
        assert_eq!(parent_anon.refs(), 1);
        assert_eq!(child_anon.refs(), 1);
        let child_pa = extract(&child.vm_map, va).expect("child pa");
        assert_ne!(child_pa, parent_pa);
        assert_eq!(extract(&vm.vm_map, va), Some(parent_pa));
        assert_eq!(parent_amap.refs(), 1);
        assert!(counters_read(UvmExpCounters::FltAcow) >= 1);

        drop((parent_anon, child_anon, parent_amap));
        uvmspace_free(child);
        uvmspace_free(vm);
    }

    #[test]
    fn wire_and_unwire_a_range() {
        let _g = setup();
        let va = 0x5000_0000;
        let len = 3 * PAGE_SIZE;
        let vm = vmspace_with_anon(va, len);

        uvm_fault_wire(&vm.vm_map, va, va + len, RW).expect("wire");
        for i in 0..3 {
            let pa = extract(&vm.vm_map, va + i * PAGE_SIZE).expect("mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
            assert_eq!(pg.flags() & PG_CLEAN, 0);
        }

        uvm_fault_unwire(&vm.vm_map, va, va + len);
        for i in 0..3 {
            let pa = extract(&vm.vm_map, va + i * PAGE_SIZE).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        uvmspace_free(vm);
    }

    #[test]
    fn pageable_wires_through_the_busy_map_and_unwires_under_the_lock() {
        // uvm_map_pageable's wire path: pass 1 under the write lock, pass 2 with the map busy
        // and unlocked (the fault takes the read lock), and the unwire under the write lock
        // (uvm_fault_unwire_locked from inside the map's own operation).
        let _g = setup();
        let va = 0x5800_0000;
        let len = 2 * PAGE_SIZE;
        let vm = vmspace_with_anon(va, len);
        let map = &vm.vm_map;
        let v = map.timestamp();

        uvm_map_pageable(map, va, va + len, false).expect("wire");
        assert_eq!(
            map.timestamp(),
            v + 1,
            "one write lock; the busy relock is none"
        );
        for i in 0..2 {
            let pa = extract(map, va + i * PAGE_SIZE).expect("mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
        }
        assert_eq!(entry_holding(map, va).map(|e| e.0), Some(va));

        uvm_map_pageable(map, va, va + len, true).expect("unwire");
        for i in 0..2 {
            let pa = extract(map, va + i * PAGE_SIZE).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        // A range with free space in it is refused.
        assert_eq!(
            uvm_map_pageable(map, va, va + len + PAGE_SIZE, false),
            Err(Errno::EINVAL)
        );
        sane(map);
        uvmspace_free(vm);
    }

    #[test]
    fn unwire_an_object_entry_without_an_amap() {
        // A shared mapping of an object (a MAP_SHARED file, here an aobj) has no amap: wiring
        // it faults the object's pages in, and unwiring must find their owner without looking
        // up an amap.
        let _g = setup();
        let vm = vmspace_with_anon(0x7000_0000, PAGE_SIZE);
        let map = &vm.vm_map;
        let len = 2 * PAGE_SIZE;
        let uobj = uao_create(Vsize::new(len), 0).expect("an aobj");
        let mut va = 0x7100_0000;
        let flags = uvm_mapflag(RW, RW, MAP_INHERIT_SHARE, MADV_NORMAL, UVM_FLAG_FIXED);
        uvm_map(map, &mut va, len, Some(uobj.clone()), 0, 0, flags).expect("map the aobj");
        {
            let guard = map.lock_read();
            let entry = guard.lookup(va).expect("the entry");
            assert!(
                entry.handles(guard.witness()).amap.is_none(),
                "a shared object entry has no amap"
            );
        }

        uvm_fault_wire(map, va, va + len, PROT_READ).expect("wire");
        for i in 0..2 {
            let pa = extract(map, va + i * PAGE_SIZE).expect("mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert!(pg.owned_by_object(&*uobj), "page {i} is the object's");
            assert_eq!(pg.wire_count(), 1, "page {i} wired");
        }

        uvm_fault_unwire(map, va, va + len);
        for i in 0..2 {
            let pa = extract(map, va + i * PAGE_SIZE).expect("still mapped");
            let pg = PHYS_TO_VM_PAGE(pa).expect("a managed page");
            assert_eq!(pg.wire_count(), 0, "page {i} unwired");
        }
        drop(uobj);
        uvmspace_free(vm);
    }

    #[test]
    fn neighbour_anons_are_mapped_ahead() {
        let _g = setup();
        let va = 0x6000_0000;
        let vm = vmspace_with_anon(va, 8 * PAGE_SIZE);
        let map = &vm.vm_map;

        // Fault the pages in one by one, then drop their mappings: the anons stay.
        for i in 0..8 {
            uvm_fault(map, va + i * PAGE_SIZE, VM_FAULT_INVALID, PROT_WRITE).expect("fault");
        }
        map_pmap_remove(map, va, va + 8 * PAGE_SIZE);
        assert!(extract(map, va + 4 * PAGE_SIZE).is_none());

        // One read fault in the middle maps the MADV_NORMAL window (3 back, 4 ahead) too.
        uvm_fault(map, va + 4 * PAGE_SIZE, VM_FAULT_INVALID, PROT_READ).expect("refault");
        for i in 1..8 {
            assert!(
                extract(map, va + i * PAGE_SIZE).is_some(),
                "page {i} mapped by the fault-ahead"
            );
        }
        assert!(extract(map, va).is_none(), "page 0 is behind the window");
        assert!(counters_read(UvmExpCounters::FltNamap) >= 6);
        uvmspace_free(vm);
    }

    #[test]
    fn the_fault_counts_its_process_faults() {
        let _g = setup();
        let va = 0x6400_0000;
        let vm = vmspace_with_anon(va, PAGE_SIZE);
        let before = testutil::vmproc_with(|p| p.minflt);
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("fault");
        assert_eq!(testutil::vmproc_with(|p| p.minflt), before + 1, "ru_minflt");
        uvmspace_free(vm);
    }

    #[test]
    fn extract_shares_the_source_pages_in_kernel_space() {
        // uvm_map_extract (kern.proc_args' uvm_io): the source's range cloned into the kernel
        // map, needs-copy resolved first, the protection without PROT_EXEC.
        let _g = setup();
        let kmap = the_kernel_map();
        // The test kernel map lies wholly below uvm_maxkaddr (testutil::KM_VA_BASE), as the
        // kernel map's space has once it ran (extract does not grow it, as the C).
        assert!(kmap.max_offset() <= uvm_maxkaddr());
        let va = 0x2000_0000;
        let vm = vmspace_with_anon(va, 2 * PAGE_SIZE);
        uvm_fault(&vm.vm_map, va, VM_FAULT_INVALID, PROT_WRITE).expect("fault");
        let src_pa = extract(&vm.vm_map, va).expect("source page");

        let kva =
            uvm_map_extract(&vm.vm_map, va, 2 * PAGE_SIZE, UVM_EXTRACT_FIXPROT).expect("extract");
        {
            let g = kmap.lock_read();
            let e = g.lookup(kva).expect("the clone");
            assert_eq!((e.start(), e.end()), (kva, kva + 2 * PAGE_SIZE));
            assert_eq!(e.protection() & PROT_EXEC, 0);
        }
        let (_, _, _, etype) = entry_holding(&vm.vm_map, va).expect("source");
        assert_eq!(etype & UVM_ET_NEEDSCOPY, 0, "the source's copy was made");

        // The kernel's fault on the window reaches the source's anon page.
        uvm_fault(kmap, kva, VM_FAULT_INVALID, PROT_READ).expect("kernel fault");
        assert_eq!(kmap.pmap().extract(Vaddr::new(kva)), Some(src_pa));

        uvm_unmap(kmap, kva, kva + 2 * PAGE_SIZE);
        uvmspace_free(vm);
    }

    #[test]
    fn the_pool_items_stay_near_the_cs() {
        // vmstat -m's item sizes: an entry about the C's (LZ measured 200 bytes; the owned
        // link and the Guarded header grow it a little, DECISIONS G4), a vmspace's map with
        // its statistics.
        let entry = core::mem::size_of::<VmMapEntry>();
        let vmspace = core::mem::size_of::<Vmspace>();
        std::println!("vmmpepl item {entry} bytes, vmsppl value {vmspace} bytes");
        assert!(entry <= 256, "{entry}");
        assert!(vmspace <= 512, "{vmspace}");
    }

    /// The kernel map of the tests: the first kernel-style map a test installed.
    fn the_kernel_map() -> &'static VmMap {
        testutil::kernel_maps_install();
        kernel_map()
    }

    /// Takes static entries of `map` until the arena has none free, then one more (the
    /// refill): whether that refill asked the kernel maps for space, and the entries.
    fn exhaust_and_refill(map: &VmMap) -> (bool, Vec<PoolBox<VmMapEntry>>) {
        let mut got = Vec::new();
        while KENTRY_ARENA.has_free() {
            got.push(uvm_mapent_alloc(map, 0).expect("a static entry"));
        }
        let (calls, nout) = (testutil::km_map_calls(), KENTRY_ARENA.nout());
        let pages = KENTRY_PAGE_ALLOCS.load(Mem::Relaxed);
        got.push(uvm_mapent_alloc(map, 0).expect("the refill's entry"));
        assert_eq!(KENTRY_ARENA.nout(), nout + 1);
        assert_eq!(
            KENTRY_PAGE_ALLOCS.load(Mem::Relaxed),
            pages + 1,
            "the refill page is kv_page's (km_alloc(PAGE_SIZE, &kv_page, ..))"
        );
        assert!(got.iter().all(|e| e.flags() == UVM_MAP_STATIC));
        (testutil::km_map_calls() != calls, got)
    }

    #[test]
    fn static_entries_refill_from_the_direct_map_under_a_held_kernel_map() {
        // cold: uvm_map on kernel_map holds its write lock when uvm_mapent_alloc runs out of
        // static entries; the refill page is kv_page's (the direct map), so it takes no
        // kernel map entry and no map lock (a kv_any page would wait for the lock held here).
        let _g = setup();
        let kmap = the_kernel_map();
        let before = UVMEXP.kmapent.load(Mem::Relaxed);
        let held = kmap.lock();
        testutil::set_cold(true);
        let (mapped, got) = exhaust_and_refill(kmap);
        testutil::set_cold(false);
        assert!(!mapped, "the refill page asked a kernel map for space");
        drop(got);
        drop(held);
        assert_eq!(UVMEXP.kmapent.load(Mem::Relaxed), before);
    }

    #[test]
    fn an_intrsafe_map_refills_its_static_entries_without_a_map() {
        // kmem_map at IPL_VM under its mutex (and uvm_kmapent_mtx): the refill touches no
        // kernel map, whose rwlock and IPL_NONE paths cannot be taken there.
        let _g = setup();
        let map = kernel_style_map(VM_MAP_INTRSAFE);
        let before = UVMEXP.kmapent.load(Mem::Relaxed);
        let spl = hal::intr::splraise(IPL_VM);
        let held = map.lock();
        let (mapped, got) = exhaust_and_refill(map);
        assert!(!mapped, "the refill page asked a kernel map for space");
        drop(got);
        drop(held);
        drop(spl);
        assert_eq!(UVMEXP.kmapent.load(Mem::Relaxed), before);
        uvm_map_deallocate(map);
    }

    #[test]
    fn protect_counts_a_stack_change_only_for_a_stack_made_rw() {
        // uvm_map_protect narrows UVM_ET_STACK to 0 unless the mapping becomes RW, and the C
        // bumps sserial from that narrowed etype (uvm_map.c's `out` label).
        let _g = setup();
        let vm = user_vmspace();
        let map = &vm.vm_map;
        let mut a = 0x4800_0000;
        let len = 2 * PAGE_SIZE;
        uvm_mapanon(map, &mut a, len, 0, anon_flags(UVM_FLAG_FIXED)).expect("mapanon");
        let s0 = map.sserial();

        uvm_map_protect(map, a, a + len, PROT_NONE, UVM_ET_STACK, false, true).expect("none");
        assert_eq!(map.sserial(), s0, "a PROT_NONE change is no stack change");
        let (_, _, _, etype) = entry_holding(map, a).expect("still mapped");
        assert_eq!(etype & UVM_ET_STACK, 0);

        uvm_map_protect(map, a, a + len, RW, UVM_ET_STACK, false, true).expect("rw");
        assert_eq!(map.sserial(), s0 + 1, "a stack made RW");
        let (_, _, _, etype) = entry_holding(map, a).expect("still mapped");
        assert_ne!(etype & UVM_ET_STACK, 0);
        uvmspace_free(vm);
    }
}
/* </TESTS> */
