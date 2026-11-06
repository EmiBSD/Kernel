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
/* </LICENSES> */

/* <CODE> */
//! The memory file system: OpenBSD `sys/tmpfs/`, one module per file of LZ's, feature
//! `tmpfs` (`option TMPFS`): the node, the directory entry, the mount and the file handle
//! (`tmpfs`, `tmpfs.h`), the accounting, the pools' items and the names (`tmpfs_mem`), the
//! nodes' and entries' life, the directories, the attributes and the data path through the
//! object window (`tmpfs_subr`), and the mount and the file system's table
//! (`tmpfs_vfsops`), the vnode operations of files, device nodes and fifos (`tmpfs_vnops`,
//! `tmpfs_specops`, `tmpfs_fifoops`); N5.9t1 brought the first four, N5.9t2 the vnode
//! operations. `testtmpfs` is the tests' fixture (host tests and benches only).

#[cfg(any(test, feature = "doubles"))]
pub mod testtmpfs;
#[allow(
    clippy::module_inception,
    reason = "OpenBSD's layout: sys/tmpfs/tmpfs.h"
)]
pub mod tmpfs;
pub mod tmpfs_fifoops;
pub mod tmpfs_mem;
pub mod tmpfs_specops;
pub mod tmpfs_subr;
pub mod tmpfs_vfsops;
pub mod tmpfs_vnops;
/* </CODE> */
