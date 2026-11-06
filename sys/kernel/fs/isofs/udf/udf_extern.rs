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
 * Written by Pedro Martelletto <pedro@ambientworks.net> in February 2005.
 * Public domain.
 */
/* </LICENSES> */

/* <CODE> */
//! `<isofs/udf/udf_extern.h>`: the prototypes of the UDF functions, its memory pools and its
//! vnode operations table, for the other UDF files.
//!
//! Upstream: sys/isofs/udf/udf_extern.h @ 3ce1f3f79392
//! LZ: sys/isofs/udf/udf_extern.rs@3c62ede68802
//!
//! ## Deviations
//! - The prototypes are the functions of `udf_subr.rs`, `udf_vfsops.rs` and `udf_vnops.rs`,
//!   reached by their paths; nothing is re-exported here.
//!
//! ## Redesign
//! - LZ's `pub use` of the three `Pool` statics of `udf_vfsops.rs` (the C's `extern struct
//!   pool udf_trans_pool, unode_pool, udf_ds_pool`) -> [`UdfTables`], the three pools of a
//!   glue typed by what they hold (`hal::pool`'s `TypedPool` of the recorded unodes,
//!   `BoxPool`s of the directory streams and of the name buffers), the member `udf` of the
//!   glue's VFS tables (a static generic over the glue cannot exist), made by `udf_init` with
//!   the C's names and flags (D-N5.8-7), and the C's `extern struct vops udf_vops`
//!   (`UdfVnops`, `udf_vnops.rs`: one address per glue). The name buffer is [`TransName`],
//!   aligned as `pool_init`'s default (`ALIGN(1)`, 8, which the C asks for with its 0): the
//!   pool's item is then the C's, 512 bytes; at the array's own alignment (2) the pool's
//!   items were 510 bytes apart and the free list's item header `pool_p_alloc` writes in each
//!   was misaligned (the dev kernel's alignment check panicked at the first item, N5.8u2's
//!   first `smoke-fs`). 0 `unsafe` (LZ: 0).

use hal::pool::{BoxPool, TypedPool};
use hal::sys::dirent::MAXNAMLEN;

use crate::fs::isofs::udf::udf::{UdfDirstream, Unicode, Unode};
use crate::fs::isofs::udf::udf_vnops::UdfVnops;
use crate::proc::ProcGlue;

/// An item of `udf_trans_pool`: `MAXNAMLEN` `unicode_t`s, the C's `MAXNAMLEN *
/// sizeof(unicode_t)` bytes at `pool_init`'s default alignment (`ALIGN(1)`: the C's `0`),
/// which rounds the item to 512 bytes as the C's pool does.
#[repr(C, align(8))]
pub struct TransName(pub [Unicode; MAXNAMLEN]);

impl TransName {
    /// A buffer of zeros.
    pub const fn new() -> Self {
        Self([0; MAXNAMLEN])
    }
}

impl Default for TransName {
    fn default() -> Self {
        Self::new()
    }
}

/// udf's globals of a glue: the C's `extern` pools and vnode operations.
pub struct UdfTables<G: ProcGlue> {
    /// `udf_vops`: the operations of a UDF vnode.
    pub vops: UdfVnops<G>,
    /// `unode_pool` (`"udfndpl"`): the unodes (recorded: `VTOU` counts a handle back from
    /// a unode the vnode lends).
    pub unode_pool: TypedPool<Unode<G>>,
    /// `udf_ds_pool` (`"udfdspl"`): the directory streams.
    pub ds_pool: BoxPool<UdfDirstream>,
    /// `udf_trans_pool` (`"udftrpl"`): the names' 8- to 16-bit expansion buffers,
    /// `MAXNAMLEN * sizeof(unicode_t)` bytes each ([`TransName`]).
    pub trans_pool: BoxPool<TransName>,
}

impl<G: ProcGlue> UdfTables<G> {
    /// No pool made yet.
    pub const fn new() -> Self {
        Self {
            vops: UdfVnops::new(),
            unode_pool: TypedPool::new(),
            ds_pool: BoxPool::new(),
            trans_pool: BoxPool::new(),
        }
    }
}

impl<G: ProcGlue> Default for UdfTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

// The C's item: `roundup(MAXNAMLEN * sizeof(unicode_t), ALIGN(1))` bytes, 8-aligned.
const _: () = {
    assert!(size_of::<TransName>() == 512);
    assert!(align_of::<TransName>() == 8);
};
/* </CODE> */
