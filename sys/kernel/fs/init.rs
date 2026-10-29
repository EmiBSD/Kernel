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
 * This code is derived from software contributed
 * to Berkeley by John Heidemann of the UCLA Ficus project.
 *
 * Source: * @(#)i405_init.c 2.10 92/04/27 UCLA Ficus project
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
 *	@(#)vfs_init.c	8.3 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! File system initialisation: `vfsinit` (the vnode table, each type's `vfs_init`), the
//! root vnode and the lookups by type name and number over the configured types.
//!
//! Upstream: sys/kern/vfs_init.c @ 3ce1f3f79392
//! LZ: sys/kern/vfs_init.rs@3c62ede68802
//!
//! ## Redesign
//! - `vfsconflist[]` (its `cfg`s, order and type numbers) stays `bsd`'s, which knows the file
//!   systems; it hands the table to [`vfsinit`], and the lookups read it from the subsystem's
//!   tables. `namei_pool` and `nchinit` are `bsd`'s (the name cache and `namei`, N5.3), made
//!   by `bsd`'s `vfsinit` before it calls this one.
//! - `rootvnode` is a link of the tables (LZ's `AtomicPtr` and its `unsafe` deref,
//!   vfs_init.rs:197, go).

use core::sync::atomic::Ordering;

use super::mount::Vfsconf;
use super::subr::{vfs, vntblinit};
use super::vnode::Vnode;
use crate::proc::ProcGlue;

/// `rootvnode`: the root (i.e. "/") vnode, `None` until a root file system is mounted.
pub fn rootvnode<G: ProcGlue>() -> Option<&'static Vnode<G>> {
    vfs::<G>().rootvnode.get()
}

/// `rootvnode = vp`.
pub fn set_rootvnode<G: ProcGlue>(vp: Option<&'static Vnode<G>>) {
    vfs::<G>().rootvnode.set(vp);
}

/// Initialize the vnode structures and initialize each file system type of `table`
/// (`vfsconflist[]`); `initialvnodes` is `conf/param.c`'s. `between` runs after the vnode
/// table and before the types, where the C has `vntblinit`'s `rn_init` (`NFSSERVER`) and
/// `nchinit`, the name cache's (`bsd`'s until N5.3 and N6).
pub fn vfsinit<G: ProcGlue>(
    table: &'static [Vfsconf<G>],
    initialvnodes: i32,
    between: impl FnOnce(),
) {
    let t = vfs::<G>();
    // A second call (a test's) keeps the first table.
    let _ = t.vfsconf.set(table);

    // Initialize the vnode table.
    vntblinit::<G>(initialvnodes);

    // Initialize the vnode name cache (and the export lists' radix keys).
    between();

    let mut maxvfsconf = 0;
    for vfsp in vfsconf::<G>() {
        if vfsp.vfc_typenum > maxvfsconf {
            maxvfsconf = vfsp.vfc_typenum;
        }
        let _ = vfsp.vfc_vfsops.init(vfsp);
    }
    t.maxvfsconf.store(maxvfsconf, Ordering::Relaxed);
}

/// The configured types (empty before `vfsinit`).
pub fn vfsconf<G: ProcGlue>() -> &'static [Vfsconf<G>] {
    vfs::<G>().vfsconf.get().copied().unwrap_or(&[])
}

/// `vfs_byname(name)`: the configured type called `name` (bytes, NUL or slice end
/// terminated).
pub fn vfs_byname<G: ProcGlue>(name: &[u8]) -> Option<&'static Vfsconf<G>> {
    let name = name.split(|&c| c == 0).next().unwrap_or(&[]);
    vfsconf::<G>().iter().find(|vfsp| vfsp.name() == name)
}

/// `vfs_bytypenum(typenum)`: the configured type numbered `typenum`.
pub fn vfs_bytypenum<G: ProcGlue>(typenum: i32) -> Option<&'static Vfsconf<G>> {
    vfsconf::<G>()
        .iter()
        .find(|vfsp| vfsp.vfc_typenum == typenum)
}
/* </CODE> */
