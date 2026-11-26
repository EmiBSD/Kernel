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
 * Copyright (c) 2009 Dariusz Swiderski <sfires@sfires.net>
 * Copyright (c) 2009 Mark Kettenis <kettenis@openbsd.org>
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
//! The flattened device tree: `dev/ofw/fdt.c`, with the types of `<dev/ofw/fdt.h>` and the
//! `OF_*` accessors `<dev/ofw/openfirm.h>` declares (`fdt.c` implements them over the blob).
//!
//! Upstream: sys/dev/ofw/fdt.c @ 3ce1f3f79392
//! Upstream: sys/dev/ofw/fdt.h @ 3ce1f3f79392
//!
//! N7.2b0: an adapter with LZ's names and signatures over the kernel's reader
//! (`kernel::dev::ofw::fdt`) and `hal`'s view of the blob (`hal::mem::fdtblob`), so its callers
//! in `bsd` do not change. The constants, `FdtReg` and the `OF_*` functions are the kernel's,
//! re-exported (`dev/ofw/openfirm.rs` re-exports them from here, as before). A node stays a
//! pointer into the blob, [`FdtNode`]: the installed blob's address plus the kernel's node
//! offset, made and taken apart by address arithmetic and never dereferenced; null is no
//! node. The boot's three calls name the blob to `hal` by its address: [`fdt_get_size`] is
//! `fdtblob::probe` (the header's magic and size, the blob not yet mapped), [`fdt_init`]
//! `probe` then `adopt` then the kernel's `fdt_init`, [`fdt_check_head`] the kernel's check of
//! the adopted bytes.
//!
//! ## Deviations
//! - The read side only (LZ's gap, the kernel module's deviations): the write side,
//!   `fdt_next_property`/`OF_nextprop`, `OF_getpropstr`/`OF_freepropstr`, `OF_translate` and
//!   the `DEBUG` printers come with the drivers that need them.
//! - `fdt_get_size` checks the header's magic only, the C's look before the blob is mapped
//!   (`arm64/machdep.c`), where LZ ran the whole `fdt_check_head` on a blob not mapped yet; a
//!   blob that passes the magic and fails the check fails `fdt_init` instead.
//! - `of_fdt_node` of a handle that is not positive is null (LZ: the header's address plus the
//!   handle, a pointer no function reads a node at).

pub use kernel::dev::ofw::fdt::{
    FDT_CODE_VERSION, FDT_END, FDT_MAGIC, FDT_NODE_BEGIN, FDT_NODE_END, FDT_NOP, FDT_PROPERTY, Fdt,
    FdtHead, FdtReg, OF_child, OF_finddevice, OF_getindex, OF_getnodebyname, OF_getnodebyphandle,
    OF_getprop, OF_getpropbool, OF_getpropint, OF_getpropint64, OF_getpropint64array,
    OF_getpropintarray, OF_getproplen, OF_is_compatible, OF_is_enabled, OF_parent, OF_peer,
    OFMAXPARAM,
};

use core::ptr;

use hal::mem::fdtblob;
use kernel::dev::ofw::fdt as kfdt;

use crate::sys::errno::Errno;
use crate::sys::types::Vaddr;

/// A node: a pointer to its `FDT_NODE_BEGIN` token in the blob; null is no node.
pub type FdtNode = *const u32;

/// The kernel's node behind a pointer of the installed blob, `None` for null or an address
/// outside it.
fn node(p: FdtNode) -> Option<kfdt::FdtNode> {
    if p.is_null() {
        return None;
    }
    let off = p.addr().wrapping_sub(kfdt::fdt_blob()?.as_ptr().addr());
    kfdt::of_fdt_node(i32::try_from(off).ok()?)
}

/// The pointer of a kernel node in the installed blob, null for none.
fn to_ptr(node: Option<kfdt::FdtNode>) -> FdtNode {
    match (node, kfdt::fdt_blob()) {
        (Some(n), Some(blob)) => blob.as_ptr().wrapping_add(n.offset() as usize).cast(),
        _ => ptr::null(),
    }
}

/// `fdt_check_head`: the blob's version, 0 if it is not one this parser accepts.
pub fn fdt_check_head(fdt: *const u8) -> u32 {
    if fdtblob::probe(Vaddr::new(fdt.addr())) == 0 {
        return 0;
    }
    fdtblob::adopt().map_or(0, kfdt::fdt_check_head)
}

/// `fdt_init`: the tree from the blob at `fdt` (null: no tree); its version, 0 when `fdt` is
/// null or not a blob.
pub fn fdt_init(fdt: *const u8) -> i32 {
    if fdt.is_null() || fdtblob::probe(Vaddr::new(fdt.addr())) == 0 {
        return kfdt::fdt_init(None);
    }
    kfdt::fdt_init(fdtblob::adopt())
}

/// `fdt_get_size`: return the size of the FDT, 0 if `fdt` is null or has no FDT magic.
pub fn fdt_get_size(fdt: *const u8) -> usize {
    fdtblob::probe(Vaddr::new(fdt.addr()))
}

/// `fdt_node_property`: the node's property `name`, a slice of the blob.
pub fn fdt_node_property(node_: FdtNode, name: &[u8]) -> Option<&'static [u8]> {
    kfdt::fdt_node_property(node(node_), name)
}

/// `fdt_next_node`: the next sibling of `node_`; the root for null.
pub fn fdt_next_node(node_: FdtNode) -> FdtNode {
    to_ptr(kfdt::fdt_next_node(node(node_)))
}

/// `fdt_node_property_ints`: the property as integers into `out`; how many, or `None`.
pub fn fdt_node_property_ints(node_: FdtNode, name: &[u8], out: &mut [i32]) -> Option<usize> {
    kfdt::fdt_node_property_ints(node(node_), name, out)
}

/// `fdt_node_property_int`: the property as an integer.
pub fn fdt_node_property_int(node_: FdtNode, name: &[u8]) -> Option<i32> {
    kfdt::fdt_node_property_int(node(node_), name)
}

/// `fdt_child_node`: the first child of `node_`, if any.
pub fn fdt_child_node(node_: FdtNode) -> FdtNode {
    to_ptr(kfdt::fdt_child_node(node(node_)))
}

/// `fdt_node_name`: the node's name.
pub fn fdt_node_name(node_: FdtNode) -> Option<&'static [u8]> {
    kfdt::fdt_node_name(node(node_))
}

/// `fdt_find_node`: the node at the absolute path `name`.
pub fn fdt_find_node(name: &[u8]) -> FdtNode {
    to_ptr(kfdt::fdt_find_node(name))
}

/// `fdt_parent_node`: the parent of `node_`, null for the root.
pub fn fdt_parent_node(node_: FdtNode) -> FdtNode {
    to_ptr(kfdt::fdt_parent_node(node(node_)))
}

/// `fdt_find_phandle`: the node with `phandle`.
pub fn fdt_find_phandle(phandle: u32) -> FdtNode {
    to_ptr(kfdt::fdt_find_phandle(phandle))
}

/// `fdt_get_cells`: the `#address-cells` and `#size-cells` in force at `node_`.
pub fn fdt_get_cells(node_: FdtNode) -> (i32, i32) {
    kfdt::fdt_get_cells(node(node_))
}

/// `fdt_get_reg`: entry `idx` of the node's `reg` into `reg`, translated; `reg` is written
/// only on success.
pub fn fdt_get_reg(node_: FdtNode, idx: usize, reg: &mut FdtReg) -> Result<(), Errno> {
    *reg = kfdt::fdt_get_reg(node(node_), idx)?;
    Ok(())
}

/// `fdt_is_compatible`: whether `name` is one of the node's `compatible` strings.
pub fn fdt_is_compatible(node_: FdtNode, name: &[u8]) -> bool {
    kfdt::fdt_is_compatible(node(node_), name)
}

/// The node of an `OF_*` handle (its byte offset from the header), for callers that mix the
/// two APIs as `fdt_find_cons` does.
pub fn of_fdt_node(handle: i32) -> FdtNode {
    to_ptr(kfdt::of_fdt_node(handle))
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // The adapter over the kernel's reader: the pointers it makes are the installed blob's
    // address plus the kernel's offsets. Trees come from the kernel's fixture; the boot's
    // pointer path through `hal::mem::fdtblob` (a process-wide record) is `hal`'s tests'.

    use std::{assert, assert_eq};

    use kernel::dev::ofw::fdt::testfdt::{empty, install, virt_like};

    use super::*;

    #[test]
    fn a_node_is_the_blob_plus_its_handle() {
        let g = install(virt_like());
        let soc = fdt_find_node(b"/soc");
        assert!(!soc.is_null());
        let handle = OF_finddevice(b"/soc");
        assert_eq!(
            soc.cast::<u8>(),
            g.blob.as_ptr().wrapping_add(handle as usize)
        );
        assert_eq!(of_fdt_node(handle), soc);
        assert_eq!(fdt_next_node(ptr::null()), fdt_find_node(b"/"));
        assert_eq!(fdt_parent_node(fdt_find_node(b"/soc/pl011")), soc);
        assert_eq!(fdt_node_name(soc), Some(&b"soc"[..]));
        assert_eq!(fdt_find_phandle(1), fdt_find_node(b"/intc"));
        assert_eq!(fdt_get_cells(fdt_find_node(b"/soc/pl011")), (1, 1));
        assert!(fdt_is_compatible(
            fdt_find_node(b"/intc"),
            b"arm,cortex-a15-gic"
        ));
        assert!(fdt_child_node(fdt_find_node(b"/chosen")).is_null());
    }

    #[test]
    fn null_in_and_out() {
        let _g = install(virt_like());
        assert!(fdt_node_property(ptr::null(), b"compatible").is_none());
        assert!(fdt_node_name(ptr::null()).is_none());
        assert!(fdt_child_node(ptr::null()).is_null());
        assert!(fdt_parent_node(ptr::null()).is_null());
        assert!(fdt_find_node(b"/nowhere").is_null());
        assert!(fdt_find_phandle(99).is_null());
        assert!(of_fdt_node(0).is_null());
        let outside = ptr::without_provenance::<u32>(8);
        assert!(
            fdt_node_name(outside).is_none(),
            "an address outside the blob"
        );
        // The boot's calls with no blob name nothing to hal.
        assert_eq!(fdt_get_size(ptr::null()), 0);
        assert_eq!(fdt_check_head(ptr::null()), 0);
        assert_eq!(fdt_init(ptr::null()), 0);
        assert!(
            fdt_find_node(b"/").is_null(),
            "fdt_init(NULL) emptied the tree"
        );
    }

    #[test]
    fn get_reg_writes_only_on_success() {
        let _g = install(virt_like());
        let uart = fdt_find_node(b"/soc/pl011");
        let mut reg = FdtReg { addr: 7, size: 7 };
        assert_eq!(fdt_get_reg(uart, 1, &mut reg), Err(Errno::EINVAL));
        assert_eq!(reg, FdtReg { addr: 7, size: 7 });
        assert_eq!(fdt_get_reg(uart, 0, &mut reg), Ok(()));
        assert_eq!(
            reg,
            FdtReg {
                addr: 0x0900_0000,
                size: 0x1000
            }
        );
        assert_eq!(fdt_get_reg(ptr::null(), 0, &mut reg), Err(Errno::EINVAL));
    }

    #[test]
    fn no_tree_is_null_everywhere() {
        let _g = empty();
        assert!(fdt_next_node(ptr::null()).is_null());
        assert!(fdt_find_node(b"/").is_null());
        assert_eq!(OF_peer(0), 0);
        let mut ints = [0i32; 2];
        assert_eq!(fdt_node_property_ints(ptr::null(), b"reg", &mut ints), None);
        assert_eq!(fdt_node_property_int(ptr::null(), b"reg"), None);
    }
}
/* </TESTS> */
