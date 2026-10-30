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
//! The file systems' core, moving out of `bsd` piece by piece (N5, docs/ROADMAP.md): the
//! buffer header and its owner's token, `<sys/buf.h>`, the buffer cache and its memory, and
//! the disk queues and physio (N5.1); the vnode, the mount, their operations and the VFS core
//! (`vfs_subr.c`, `vfs_sync.c`, `vfs_init.c`, `vfs_default.c`, the special files and deadfs,
//! N5.2); the names, the name cache, `getcwd`, the vnode file operations and the record
//! locks (`vfs_lookup.c`, `vfs_cache.c`, `vfs_getcwd.c`, `vfs_vnops.c`, `vfs_lockf.c`,
//! N5.3).

pub mod bio;
pub mod biomem;
pub mod buf;
pub mod bufq;
pub mod cache;
pub mod default;
pub mod getcwd;
pub mod init;
pub mod lockf;
pub mod lookup;
pub mod miscfs;
pub mod mount;
pub mod namei;
pub mod physio;
pub mod spec;
pub mod subr;
pub mod sync;
pub mod syscalls;
#[cfg(any(test, feature = "doubles"))]
pub mod testfs;
pub mod ufs;
pub mod vnode;
pub mod vnops;
pub mod vops;
/* </CODE> */
