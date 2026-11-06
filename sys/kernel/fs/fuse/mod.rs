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
//! FUSE, the userland file system interface: OpenBSD `sys/miscfs/fuse/` and `<sys/fusebuf.h>`,
//! one module per file of LZ's, feature `fuse` (`option FUSE` and `pseudo-device fuse`):
//! the protocol and the request buffer (`fusebuf_h`, `<sys/fusebuf.h>`), the requests' life
//! (`fusebuf`), the `/dev/fuse0` character device the daemon reads requests from and writes
//! replies to (`fuse_device`), the node and its hash (`fusefs_node`, `fuse_ihash`), the
//! mount structure and the globals (`fusefs`), the file handles (`fuse_file`), the lookup
//! (`fuse_lookup`), the vnode operations (`fuse_vnops`) and the file-system-type operations
//! (`fuse_vfsops`); N5.9f1 brought the first seven, N5.9f2 the last three, unwired until its
//! switch (`bsd`'s `sys/miscfs/fuse/` serves every mount until then). `FUSE_DEBUG` is off, as
//! in GENERIC:
//! the `DPRINTF`s are not compiled. `testfuse` is the tests' fixture (host tests and benches
//! only).

pub mod fuse_device;
pub mod fuse_file;
pub mod fuse_ihash;
pub mod fuse_lookup;
pub mod fuse_vfsops;
pub mod fuse_vnops;
pub mod fusebuf;
pub mod fusebuf_h;
pub mod fusefs;
pub mod fusefs_node;
#[cfg(any(test, feature = "doubles"))]
pub mod testfuse;
/* </CODE> */
