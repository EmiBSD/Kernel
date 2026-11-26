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
//! LZ: sys/dev/ofw/fdt.rs@e8249449d905
//!
//! A parser over the bootloader's blob, which `hal::mem::fdtblob` hands over as a
//! `&'static [u8]` (the machine's boot gives it to [`fdt_init`]); the installed tree is only
//! read. The one write is the boot's, before the install: [`fdt_take_property`] wipes
//! boot(8)'s softraid secrets through `hal`'s one mutable view of the blob. Every word, name
//! and value is read with its length test: a malformed tree reads as the function's own
//! nothing, never past the blob. On a well-formed tree every result is the C's.
//!
//! ## Deviations
//! - The read side only, as LZ: the write side (`fdt_finalize`, `fdt_add_str`,
//!   `fdt_node_set_property`, `fdt_node_add_property`, `OF_setprop`),
//!   `fdt_next_property`/`OF_nextprop`, `OF_getpropstr`/`OF_freepropstr`, `OF_translate` and the
//!   `DEBUG` printers are not ported (LZ's gap: they come with the drivers that need them). A
//!   write side on the installed tree would need another `hal` piece: once `fdt_init` has
//!   adopted the blob it is only read (the boot's wipe runs before, over a borrowed blob).
//! - The blob is read in place for the kernel's run (LZ's standing deviation): the C copies it
//!   into stolen memory mapped read-only (`arm64/machdep.c`); arm64's `initarm` refuses a blob
//!   in usable memory instead.
//! - Names are byte strings without a NUL; the blob's strings are NUL-terminated. A property's
//!   value and a node's name come back as slices of the blob instead of a length and an out
//!   pointer; `None` is the C's `-1` (or `NULL`).
//!
//! ## Redesign
//! - LZ's `Fdt` (five raw pointers into the blob, `StaticCell`, `TREE_INITED` beside it) is a
//!   `Copy` value over the blob's slice and its offsets in one `ReadMostly<Option<Fdt>>`
//!   ([`TREE`]: `None` is the C's `tree_inited == 0`); a public call copies it out once and
//!   passes it down, so a walk costs one read of the cell, not one per step. `fdt_init` replaces
//!   it (the boot CPU, twice at boot; a test's install).
//! - A node is an [`FdtNode`], the byte offset of its `FDT_NODE_BEGIN` from the header (never
//!   0: the header is there), which is the number an `OF_*` handle already was; `None` is the
//!   C's `NULL`. A handle that is not positive is no node; one that is not at a
//!   `FDT_NODE_BEGIN` is no node in every function, as the C's token checks make it.
//! - Every raw read (LZ's `be32`, `cstr`, the header's fields, a property's value) is a bounded
//!   read of the slice: [`word`], [`cstr`]. The walks are the C's, step by step, over offsets;
//!   `skip_node` walks a subtree with a depth count instead of recursing (the same tokens in
//!   the same order, the C's one skipped word per node end included). The parent and phandle
//!   searches recurse as deep as the tree, as the C.
//! - `fdt_get_reg` returns the `FdtReg` (`Result<FdtReg, Errno>`) instead of filling an out
//!   parameter; `fdt_translate_reg`'s sums wrap as the C's unsigned arithmetic.
//! - `Fdt<'a>`: the reader also parses a borrowed blob without installing it (`parse`, the same
//!   `check_head` as `fdt_init`). [`fdt_take_property`] uses it for boot(8)'s
//!   `openbsd,sr-bootuuid` and `openbsd,sr-bootkey` (the C's `initarm` copies them out and
//!   `explicit_bzero`s them in the tree; LZ's `bootarg_bzero` wrote through a shared slice of
//!   the installed blob): the value is copied out when its length matches and zeroed whatever
//!   it is, over the boot's mutable view, before `fdt_init` installs the blob.

#![allow(
    non_snake_case,
    reason = "the OF_* functions keep OpenFirmware's names"
)]

use core::mem::offset_of;
use core::num::NonZeroU32;

use hal::mem::explicit_bzero;
use hal::sync::slot::ReadMostly;
use sysdefs::base::errno::Errno;

/// `FDT_MAGIC`.
pub const FDT_MAGIC: u32 = 0xd00d_feed;
/// `FDT_NODE_BEGIN`.
pub const FDT_NODE_BEGIN: u32 = 0x01;
/// `FDT_NODE_END`.
pub const FDT_NODE_END: u32 = 0x02;
/// `FDT_PROPERTY`.
pub const FDT_PROPERTY: u32 = 0x03;
/// `FDT_NOP`.
pub const FDT_NOP: u32 = 0x04;
/// `FDT_END`.
pub const FDT_END: u32 = 0x09;
/// `FDT_CODE_VERSION`: the newest version this parser knows.
pub const FDT_CODE_VERSION: u32 = 0x11;

/// `OFMAXPARAM` (`<dev/ofw/openfirm.h>`): the longest property name `OF_nextprop` returns.
pub const OFMAXPARAM: usize = 64;

/// `struct fdt_head`: the blob's header, big-endian; its layout gives the offsets the header's
/// words are read at.
#[repr(C)]
pub struct FdtHead {
    /// `fh_magic`.
    pub fh_magic: u32,
    /// `fh_size`.
    pub fh_size: u32,
    /// `fh_struct_off`.
    pub fh_struct_off: u32,
    /// `fh_strings_off`.
    pub fh_strings_off: u32,
    /// `fh_reserve_off`.
    pub fh_reserve_off: u32,
    /// `fh_version`.
    pub fh_version: u32,
    /// `fh_comp_ver`: last compatible version.
    pub fh_comp_ver: u32,
    /// `fh_boot_cpu_id`: `fh_version >= 2`.
    pub fh_boot_cpu_id: u32,
    /// `fh_strings_size`: `fh_version >= 3`.
    pub fh_strings_size: u32,
    /// `fh_struct_size`: `fh_version >= 17`.
    pub fh_struct_size: u32,
}

/// `struct fdt`: the parsed blob, its blocks as offsets from the header. The installed tree
/// is an `Fdt<'static>`; the boot's wipe parses a borrowed blob ([`fdt_take_property`]).
#[derive(Clone, Copy, Debug)]
pub struct Fdt<'a> {
    /// `header`: the whole blob.
    pub blob: &'a [u8],
    /// `tree`: the structure block's offset.
    pub struct_off: u32,
    /// `strings`: the strings block's offset.
    pub strings_off: u32,
    /// `memory`: the memory reservation block's offset.
    pub reserve_off: u32,
    /// `version`.
    pub version: u32,
    /// `strings_size`.
    pub strings_size: u32,
    /// `struct_size` (0 before version 17).
    pub struct_size: u32,
}

/// `struct fdt_reg`: an address and size from a `reg` property.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FdtReg {
    /// `addr`.
    pub addr: u64,
    /// `size`.
    pub size: u64,
}

/// A node: the byte offset of its `FDT_NODE_BEGIN` token from the header, the number its
/// `OF_*` handle is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FdtNode(NonZeroU32);

impl FdtNode {
    /// The node's offset from the header (its `OF_*` handle).
    pub fn offset(self) -> u32 {
        self.0.get()
    }

    /// The offset as an index into the blob.
    fn at(self) -> usize {
        self.0.get() as usize
    }
}

/// `tree` and `tree_inited`: the parsed blob, `None` before [`fdt_init`] accepted one.
static TREE: ReadMostly<Option<Fdt<'static>>> = ReadMostly::new(None);

/// The parsed tree, copied out of [`TREE`].
fn tree() -> Option<Fdt<'static>> {
    *TREE.read()
}

/// The node at byte offset `off`, if it can be one (not the header, inside 32 bits).
fn node_at(off: usize) -> Option<FdtNode> {
    u32::try_from(off)
        .ok()
        .and_then(NonZeroU32::new)
        .map(FdtNode)
}

/// `betoh32(*ptr)`: the big-endian word at byte offset `off`, `None` past the end.
fn word(b: &[u8], off: usize) -> Option<u32> {
    let w = b.get(off..off.checked_add(4)?)?;
    <[u8; 4]>::try_from(w).ok().map(u32::from_be_bytes)
}

/// The NUL-terminated string at byte offset `off`, without the NUL; `None` when no NUL ends it
/// inside the blob.
fn cstr(b: &[u8], off: usize) -> Option<&[u8]> {
    let rest = b.get(off..)?;
    rest.get(..rest.iter().position(|&c| c == 0)?)
}

/// `strcmp(a, b) == 0` for the NUL-terminated `b` at byte offset `off` and a NUL-free `a`,
/// byte by byte as `strcmp` walks: the first difference (or the blob's end) ends it.
fn name_eq(b: &[u8], a: &[u8], off: usize) -> bool {
    let at = |i: usize| off.checked_add(i).and_then(|j| b.get(j)).copied();
    a.iter()
        .enumerate()
        .all(|(i, &c)| c != 0 && at(i) == Some(c))
        && at(a.len()) == Some(0)
}

/// `fdt_check_head`: the blob's version, 0 if it is not one this parser accepts.
pub fn fdt_check_head(blob: &[u8]) -> u32 {
    check_head(blob).unwrap_or(0)
}

/// [`fdt_check_head`], `None` for 0. The structure block is found as the C finds it, its
/// offset counted in words.
fn check_head(b: &[u8]) -> Option<u32> {
    if word(b, offset_of!(FdtHead, fh_magic))? != FDT_MAGIC {
        return None;
    }
    let version = word(b, offset_of!(FdtHead, fh_version))?;
    if version > FDT_CODE_VERSION {
        return None;
    }
    let struct_words = word(b, offset_of!(FdtHead, fh_struct_off))? as usize / 4;
    let tok = skip_nops(b, struct_words * 4)?;
    if word(b, tok)? != FDT_NODE_BEGIN {
        return None;
    }
    // check for end signature on version 17 blob
    if version >= 17 {
        let size_words = word(b, offset_of!(FdtHead, fh_struct_size))? as usize / 4;
        let last = (struct_words + size_words).checked_sub(1)?;
        if word(b, last * 4)? != FDT_END {
            return None;
        }
    }
    Some(version)
}

/// `fdt_init`: initializes internal structures of module. Has to be called once, preferably
/// in `machdep.c`. Replaces the tree with `blob` (`None` is the C's `NULL`: no tree); the
/// version, 0 when there is no tree or `blob` is not one.
pub fn fdt_init(blob: Option<&'static [u8]>) -> i32 {
    let _ = TREE.replace(None);
    let Some(fdt) = blob.and_then(parse) else {
        return 0;
    };
    let _ = TREE.replace(Some(fdt));
    fdt.version as i32
}

/// `fdt_init`'s parse of `b`, without installing it: `None` when it is not a blob this parser
/// accepts.
fn parse(b: &[u8]) -> Option<Fdt<'_>> {
    let version = check_head(b)?;
    if version == 0 {
        return None;
    }
    // fdt_check_head read the words up to the version's, so the header has them.
    let head = |off| word(b, off).unwrap_or(0);
    Some(Fdt {
        blob: b,
        struct_off: head(offset_of!(FdtHead, fh_struct_off)),
        strings_off: head(offset_of!(FdtHead, fh_strings_off)),
        reserve_off: head(offset_of!(FdtHead, fh_reserve_off)),
        version,
        strings_size: head(offset_of!(FdtHead, fh_strings_size)),
        struct_size: if version >= 17 {
            head(offset_of!(FdtHead, fh_struct_size))
        } else {
            0
        },
    })
}

/// The blob [`fdt_init`] installed, `None` without a tree.
pub fn fdt_blob() -> Option<&'static [u8]> {
    tree().map(|t| t.blob)
}

/// `fdt_get_size`: return the size of the FDT, 0 if `blob` is not one.
pub fn fdt_get_size(blob: &[u8]) -> usize {
    if fdt_check_head(blob) == 0 {
        return 0;
    }
    word(blob, offset_of!(FdtHead, fh_size)).map_or(0, |s| s as usize)
}

/// `fdt_get_str`: retrieve the offset of string `num` from the strings table.
fn fdt_get_str(t: &Fdt<'_>, num: u32) -> Option<usize> {
    if num > t.strings_size {
        return None;
    }
    Some(t.strings_off as usize + num as usize)
}

/// `skip_nops`.
fn skip_nops(b: &[u8], mut p: usize) -> Option<usize> {
    while word(b, p)? == FDT_NOP {
        p += 4;
    }
    Some(p)
}

/// `skip_property`: move forward by magic + size + nameid + rounded up property size.
fn skip_property(b: &[u8], p: usize) -> Option<usize> {
    let size = word(b, p + 4)? as usize;
    skip_nops(b, p + 12 + size.div_ceil(4) * 4)
}

/// `skip_props`.
fn skip_props(b: &[u8], mut p: usize) -> Option<usize> {
    while word(b, p)? == FDT_PROPERTY {
        p = skip_property(b, p)?;
    }
    Some(p)
}

/// `skip_node_name`: skip name, aligned to 4 bytes, this is NULL term., so must add 1.
fn skip_node_name(b: &[u8], p: usize) -> Option<usize> {
    let len = cstr(b, p)?.len() + 1;
    skip_nops(b, p + len.div_ceil(4) * 4)
}

/// `fdt_node_property`: retrieves node property, the returned slice is inside the fdt tree.
/// `None` when the node has no such property (or no tree is installed).
pub fn fdt_node_property(node: Option<FdtNode>, name: &[u8]) -> Option<&'static [u8]> {
    node_property(&tree()?, node?, name)
}

/// [`fdt_node_property`] in the tree `t`.
fn node_property<'a>(t: &Fdt<'a>, node: FdtNode, name: &[u8]) -> Option<&'a [u8]> {
    let (off, len) = property_at(t, node, name)?;
    t.blob.get(off..off + len)
}

/// The offset and length of the value of the node's property `name`, inside the blob.
fn property_at(t: &Fdt<'_>, node: FdtNode, name: &[u8]) -> Option<(usize, usize)> {
    let b = t.blob;
    let mut p = node.at();
    if word(b, p)? != FDT_NODE_BEGIN {
        return None;
    }
    p = skip_node_name(b, p + 4)?;
    while word(b, p)? == FDT_PROPERTY {
        let nameid = word(b, p + 8)?; // id of name in strings table
        if fdt_get_str(t, nameid).is_some_and(|s| name_eq(b, name, s)) {
            let size = word(b, p + 4)? as usize; // size of value
            b.get(p + 12..p + 12 + size)?;
            return Some((p + 12, size));
        }
        p = skip_property(b, p)?;
    }
    None
}

/// `skip_node`: the offset past the node at `node` and its subtree, no matter whether a node
/// follows. A depth count stands for the C's recursion: at each node's end the C skips one
/// word, whatever it reads, then the NOPs after it.
fn skip_node(b: &[u8], node: usize) -> Option<usize> {
    let mut p = node;
    let mut depth = 0usize;
    loop {
        // The node at `p`: its token, its name and its properties.
        p = skip_props(b, skip_node_name(b, p + 4)?)?;
        depth += 1;
        // Its end, and its parents' ends, until a child (or a sibling of an ancestor) starts.
        while word(b, p)? != FDT_NODE_BEGIN {
            p = skip_nops(b, p + 4)?;
            depth -= 1;
            if depth == 0 {
                return Some(p);
            }
        }
    }
}

/// `fdt_next_node`: retrieves next node, skipping all the children nodes of the pointed
/// node, returns next node if exists, otherwise `None`. If passed `None` will return first
/// node of the tree (root).
pub fn fdt_next_node(node: Option<FdtNode>) -> Option<FdtNode> {
    let t = tree()?;
    match node {
        None => root(&t),
        Some(n) => next_node(&t, n),
    }
}

/// `fdt_next_node(NULL)`: the root.
fn root(t: &Fdt<'_>) -> Option<FdtNode> {
    let p = skip_nops(t.blob, t.struct_off as usize)?;
    if word(t.blob, p)? == FDT_NODE_BEGIN {
        node_at(p)
    } else {
        None
    }
}

/// `fdt_next_node` of a node in the tree `t`.
fn next_node(t: &Fdt<'_>, node: FdtNode) -> Option<FdtNode> {
    let b = t.blob;
    let mut p = node.at();
    if word(b, p)? != FDT_NODE_BEGIN {
        return None;
    }
    p = skip_props(b, skip_node_name(b, p + 4)?)?;
    // skip children
    while word(b, p)? == FDT_NODE_BEGIN {
        p = skip_node(b, p)?;
    }
    if word(b, p)? != FDT_NODE_END {
        return None;
    }
    p = skip_nops(b, p + 4)?;
    if word(b, p)? != FDT_NODE_BEGIN {
        return None;
    }
    node_at(p)
}

/// `fdt_node_property_ints`: retrieves node property as integers and puts them in the given
/// integer array; how many it put, or `None` when there is no such property or it holds no
/// whole integer.
pub fn fdt_node_property_ints(
    node: Option<FdtNode>,
    name: &[u8],
    out: &mut [i32],
) -> Option<usize> {
    property_ints(fdt_node_property(node, name)?, out)
}

/// [`fdt_node_property_ints`] of a value already found.
fn property_ints(data: &[u8], out: &mut [i32]) -> Option<usize> {
    if data.len() / 4 == 0 {
        return None;
    }
    let mut n = 0;
    for (slot, w) in out.iter_mut().zip(data.as_chunks::<4>().0) {
        *slot = u32::from_be_bytes(*w) as i32;
        n += 1;
    }
    Some(n)
}

/// `fdt_node_property_int`: retrieves node property as an integer.
pub fn fdt_node_property_int(node: Option<FdtNode>, name: &[u8]) -> Option<i32> {
    let mut v = [0i32; 1];
    fdt_node_property_ints(node, name, &mut v).map(|_| v[0])
}

/// `fdt_child_node`: the first child of `node`, if any.
pub fn fdt_child_node(node: Option<FdtNode>) -> Option<FdtNode> {
    child_node(&tree()?, node?)
}

/// [`fdt_child_node`] in the tree `t`.
fn child_node(t: &Fdt<'_>, node: FdtNode) -> Option<FdtNode> {
    let b = t.blob;
    let p = node.at();
    if word(b, p)? != FDT_NODE_BEGIN {
        return None;
    }
    let p = skip_props(b, skip_node_name(b, p + 4)?)?;
    // check if there is a child node
    if word(b, p)? == FDT_NODE_BEGIN {
        node_at(p)
    } else {
        None
    }
}

/// The children of `node` in order: the C's `for (child = fdt_child_node(node); child;
/// child = fdt_next_node(child))`, one step from the last.
fn children<'t>(t: &'t Fdt<'_>, node: FdtNode) -> impl Iterator<Item = FdtNode> + 't {
    core::iter::successors(child_node(t, node), move |&c| next_node(t, c))
}

/// `fdt_node_name`: retrieves node name.
pub fn fdt_node_name(node: Option<FdtNode>) -> Option<&'static [u8]> {
    node_name(&tree()?, node?)
}

/// [`fdt_node_name`] in the tree `t`.
fn node_name<'a>(t: &Fdt<'a>, node: FdtNode) -> Option<&'a [u8]> {
    let b = t.blob;
    if word(b, node.at())? != FDT_NODE_BEGIN {
        return None;
    }
    cstr(b, node.at() + 4)
}

/// Whether the node name `s` is `name` with a unit address (`name@...`).
fn unit_match(s: &[u8], name: &[u8]) -> bool {
    s.strip_prefix(name)
        .is_some_and(|rest| rest.first() == Some(&b'@'))
}

/// The child of `node` called `name`: an exact match first, then one without its unit
/// address; the C's two walks.
fn child_by_name(t: &Fdt<'_>, node: FdtNode, name: &[u8]) -> Option<FdtNode> {
    children(t, node)
        .find(|&c| node_name(t, c) == Some(name))
        .or_else(|| {
            children(t, node).find(|&c| node_name(t, c).is_some_and(|s| unit_match(s, name)))
        })
}

/// `fdt_find_node`: the node at the absolute path `name`, matching each component with or
/// without its unit address.
pub fn fdt_find_node(name: &[u8]) -> Option<FdtNode> {
    find_node(&tree()?, name)
}

/// [`fdt_find_node`] in the tree `t`: the components between runs of `/`, from the root.
fn find_node(t: &Fdt<'_>, name: &[u8]) -> Option<FdtNode> {
    if name.first() != Some(&b'/') {
        return None;
    }
    let mut node = root(t);
    for component in name.split(|&c| c == b'/').filter(|c| !c.is_empty()) {
        // No match found: none.
        node = Some(child_by_name(t, node?, component)?);
    }
    node
}

/// `fdt_parent_node_recurse`.
fn fdt_parent_node_recurse(t: &Fdt<'_>, pnode: FdtNode, child: FdtNode) -> Option<FdtNode> {
    for node in children(t, pnode) {
        if node == child {
            return Some(pnode);
        }
        if let Some(found) = fdt_parent_node_recurse(t, node, child) {
            return Some(found);
        }
    }
    None
}

/// `fdt_parent_node`: the parent of `node`, `None` for the root.
pub fn fdt_parent_node(node: Option<FdtNode>) -> Option<FdtNode> {
    parent_node(&tree()?, node?)
}

/// [`fdt_parent_node`] in the tree `t`.
fn parent_node(t: &Fdt<'_>, node: FdtNode) -> Option<FdtNode> {
    let pnode = root(t)?;
    if node == pnode {
        return None;
    }
    fdt_parent_node_recurse(t, pnode, node)
}

/// `fdt_find_phandle_recurse`.
fn fdt_find_phandle_recurse(t: &Fdt<'_>, node: FdtNode, phandle: u32) -> Option<FdtNode> {
    let data =
        node_property(t, node, b"phandle").or_else(|| node_property(t, node, b"linux,phandle"));
    if data.is_some_and(|d| d.len() == 4 && word(d, 0) == Some(phandle)) {
        return Some(node);
    }
    children(t, node).find_map(|child| fdt_find_phandle_recurse(t, child, phandle))
}

/// `fdt_find_phandle`: the node with `phandle`.
pub fn fdt_find_phandle(phandle: u32) -> Option<FdtNode> {
    find_phandle(&tree()?, phandle)
}

/// [`fdt_find_phandle`] in the tree `t`.
fn find_phandle(t: &Fdt<'_>, phandle: u32) -> Option<FdtNode> {
    fdt_find_phandle_recurse(t, root(t)?, phandle)
}

/// `fdt_get_cells`: the `#address-cells` and `#size-cells` in force at `node` (inherited
/// from the parents, 1 and 1 at the top).
pub fn fdt_get_cells(node: Option<FdtNode>) -> (i32, i32) {
    match (tree(), node) {
        (Some(t), Some(n)) => get_cells(&t, n),
        _ => (1, 1),
    }
}

/// [`fdt_get_cells`] in the tree `t`.
fn get_cells(t: &Fdt<'_>, node: FdtNode) -> (i32, i32) {
    let (mut ac, mut sc) = parent_node(t, node).map_or((1, 1), |p| get_cells(t, p));
    let int = |name: &[u8]| {
        let mut v = [0i32; 1];
        node_property(t, node, name)
            .and_then(|d| property_ints(d, &mut v))
            .map(|_| v[0])
    };
    if let Some(v) = int(b"#address-cells") {
        ac = v;
    }
    if let Some(v) = int(b"#size-cells") {
        sc = v;
    }
    (ac, sc)
}

/// One big-endian cell of a property value (0 past its end, which the callers' length tests
/// rule out).
fn cell(data: &[u8], i: usize) -> u64 {
    i.checked_mul(4)
        .and_then(|off| word(data, off))
        .map_or(0, u64::from)
}

/// Whether `#address-cells`/`#size-cells` are widths this code reads: 32-bit (1) or 64-bit
/// (2) addresses and sizes.
fn supported_cells(ac: i32, sc: i32) -> Option<(usize, usize)> {
    ((1..=2).contains(&ac) && (1..=2).contains(&sc)).then_some((ac as usize, sc as usize))
}

/// `fdt_translate_reg`: translate memory address depending on parent's range.
///
/// Ranges are a way of mapping one address to another. This ranges attribute is set on a
/// node's parent. This means if a node does not have a parent, there's nothing to translate.
/// If it does have a parent and the parent does not have a ranges attribute, there's nothing
/// to translate either.
///
/// If the parent has a ranges attribute and the attribute is not empty, the node's memory
/// address has to be in one of the given ranges. This range is then used to translate the
/// memory address.
///
/// If the parent has a ranges attribute, but the attribute is empty, there's nothing to
/// translate. But it's not a translation barrier. It can be treated as a simple 1:1 mapping.
///
/// Translation does not end here. We need to check if the parent's parent also has a ranges
/// attribute and ask the same questions again.
fn fdt_translate_reg(t: &Fdt<'_>, node: FdtNode, mut reg: FdtReg) -> Result<FdtReg, Errno> {
    // No parent, no translation.
    let Some(parent) = parent_node(t, node) else {
        return Ok(reg);
    };

    // Extract ranges property from node. No ranges means translation barrier. Translation
    // stops here.
    let Some(range) = node_property(t, node, b"ranges") else {
        return Ok(reg);
    };
    let mut rlen = range.len() / 4;

    // Empty ranges means 1:1 mapping. Continue translation on parent.
    if rlen == 0 {
        return fdt_translate_reg(t, parent, reg);
    }

    // Get parent address/size width (32 or 64 bits).
    let (pac, psc) = get_cells(t, parent);
    if supported_cells(pac, psc).is_none() {
        return Err(Errno::EINVAL);
    }
    let pac = pac as usize;

    // Get our own address/size width, the same two widths only.
    let (ac, sc) = get_cells(t, node);
    let Some((ac, sc)) = supported_cells(ac, sc) else {
        return Err(Errno::EINVAL);
    };

    // Must have at least one range.
    let rone = pac + ac + sc;
    if rlen < rone {
        return Err(Errno::ESRCH);
    }

    // For each range.
    let mut base = 0;
    while rlen >= rone {
        let r = |i: usize| cell(range, base + i);
        // Extract from and size, so we can see if we fit.
        let mut from = r(0);
        if ac == 2 {
            from = (from << 32) + r(1);
        }
        let mut size = r(ac + pac);
        if sc == 2 {
            size = (size << 32) + r(ac + pac + 1);
        }

        // Try next, if we're not in the range.
        if reg.addr < from || reg.addr.wrapping_add(reg.size) > from.wrapping_add(size) {
            rlen -= rone;
            base += rone;
            continue;
        }

        // All good, extract to address and translate.
        let mut to = r(ac);
        if pac == 2 {
            to = (to << 32) + r(ac + 1);
        }
        reg.addr = reg.addr.wrapping_sub(from).wrapping_add(to);
        return fdt_translate_reg(t, parent, reg);
    }

    // To be successful, we must have returned in the for-loop.
    Err(Errno::ESRCH)
}

/// `fdt_get_reg`: parse the memory address and size of a node (entry `idx` of `reg`),
/// translated through the parents' `ranges`.
pub fn fdt_get_reg(node: Option<FdtNode>, idx: usize) -> Result<FdtReg, Errno> {
    let (Some(t), Some(node)) = (tree(), node) else {
        return Err(Errno::EINVAL);
    };
    let parent = parent_node(&t, node).ok_or(Errno::EINVAL)?;

    // Get parent address/size width (32 or 64 bits).
    let (ac, sc) = get_cells(&t, parent);
    let (ac, sc) = supported_cells(ac, sc).ok_or(Errno::EINVAL)?;

    let data = node_property(&t, node, b"reg").ok_or(Errno::EINVAL)?;
    let inlen = data.len() / 4;
    let need = idx.checked_add(1).and_then(|n| n.checked_mul(ac + sc));
    if need.is_none_or(|need| inlen < need) {
        return Err(Errno::EINVAL);
    }

    let off = idx * (ac + sc);
    let mut reg = FdtReg {
        addr: cell(data, off),
        size: cell(data, off + ac),
    };
    if ac == 2 {
        reg.addr = (reg.addr << 32) + cell(data, off + 1);
    }
    if sc == 2 {
        reg.size = (reg.size << 32) + cell(data, off + ac + 1);
    }

    fdt_translate_reg(&t, parent, reg)
}

/// `fdt_is_compatible`: whether `name` is one of the node's `compatible` strings.
pub fn fdt_is_compatible(node: Option<FdtNode>, name: &[u8]) -> bool {
    match (tree(), node) {
        (Some(t), Some(n)) => is_compatible(&t, n, name),
        _ => false,
    }
}

/// [`fdt_is_compatible`] in the tree `t`.
fn is_compatible(t: &Fdt<'_>, node: FdtNode, name: &[u8]) -> bool {
    node_property(t, node, b"compatible")
        .is_some_and(|data| data.split(|&c| c == 0).any(|s| s == name))
}

/// The node of an `OF_*` handle, for callers that mix the two APIs as `fdt_find_cons` does:
/// the node at that offset, `None` for a handle that is not positive.
pub fn of_fdt_node(handle: i32) -> Option<FdtNode> {
    of_node(handle)
}

/// The node of an `OF_*` handle (its byte offset from the header).
fn of_node(handle: i32) -> Option<FdtNode> {
    usize::try_from(handle).ok().and_then(node_at)
}

/// The `OF_*` handle of a node, 0 for none.
fn of_handle(node: Option<FdtNode>) -> i32 {
    node.map_or(0, |n| n.offset() as i32)
}

/// `OF_peer`: the next sibling; `OF_peer(0)` is the root.
pub fn OF_peer(handle: i32) -> i32 {
    let Some(t) = tree() else {
        return 0;
    };
    of_handle(if handle == 0 {
        find_node(&t, b"/")
    } else {
        of_node(handle).and_then(|n| next_node(&t, n))
    })
}

/// `OF_child`: the first child.
pub fn OF_child(handle: i32) -> i32 {
    of_handle(fdt_child_node(of_node(handle)))
}

/// `OF_parent`: the parent.
pub fn OF_parent(handle: i32) -> i32 {
    of_handle(fdt_parent_node(of_node(handle)))
}

/// `OF_finddevice`: the node at the path `name`, or `-1`.
pub fn OF_finddevice(name: &[u8]) -> i32 {
    fdt_find_node(name).map_or(-1, |n| of_handle(Some(n)))
}

/// `OF_getnodebyname`: the child of `handle` (the root for 0) called `name`, with or without
/// its unit address.
pub fn OF_getnodebyname(handle: i32, name: &[u8]) -> i32 {
    let Some(t) = tree() else {
        return 0;
    };
    let node = if handle == 0 {
        find_node(&t, b"/")
    } else {
        of_node(handle)
    };
    of_handle(node.and_then(|n| child_by_name(&t, n, name)))
}

/// `OF_getnodebyphandle`.
pub fn OF_getnodebyphandle(phandle: u32) -> i32 {
    of_handle(fdt_find_phandle(phandle))
}

/// The "name" property is optional since version 16 of the flattened device tree
/// specification, so we synthesize one from the unit name of the node if it is missing.
fn synthesized_name<'a>(t: &Fdt<'a>, node: FdtNode) -> Option<&'a [u8]> {
    let name = node_name(t, node)?;
    name.split(|&c| c == b'@').next()
}

/// A property of the node `handle` names: its value, or the synthesized "name".
enum Prop {
    /// The property's value.
    Value(&'static [u8]),
    /// No "name" property: the node's name without its unit address.
    Name(&'static [u8]),
}

/// The property `prop` of the node `handle`, `None` when there is none (and no name to
/// synthesize).
fn of_prop(t: &Fdt<'static>, handle: i32, prop: &[u8]) -> Option<Prop> {
    let node = of_node(handle)?;
    match node_property(t, node, prop) {
        Some(data) => Some(Prop::Value(data)),
        None if prop == b"name" => synthesized_name(t, node).map(Prop::Name),
        None => None,
    }
}

/// `OF_getproplen`: the length of a property, `-1` when there is none.
pub fn OF_getproplen(handle: i32, prop: &[u8]) -> i32 {
    match tree().and_then(|t| of_prop(&t, handle, prop)) {
        Some(Prop::Value(data)) => data.len() as i32,
        Some(Prop::Name(name)) => name.len() as i32 + 1,
        None => -1,
    }
}

/// `OF_getprop` in the tree `t`.
fn getprop(t: &Fdt<'static>, handle: i32, prop: &[u8], buf: &mut [u8]) -> i32 {
    match of_prop(t, handle, prop) {
        Some(Prop::Value(data)) => {
            let n = data.len().min(buf.len());
            buf[..n].copy_from_slice(&data[..n]);
            data.len() as i32
        }
        Some(Prop::Name(name)) => {
            // strlcpy, then the unit address is cut off
            let n = name.len().min(buf.len().saturating_sub(1));
            buf[..n].copy_from_slice(&name[..n]);
            if let Some(nul) = buf.get_mut(n) {
                *nul = 0;
            }
            name.len() as i32 + 1
        }
        None => -1,
    }
}

/// `OF_getprop`: copies a property into `buf` (at most `buf.len()` bytes); returns the
/// property's full length, or `-1` when there is none.
pub fn OF_getprop(handle: i32, prop: &[u8], buf: &mut [u8]) -> i32 {
    tree().map_or(-1, |t| getprop(&t, handle, prop, buf))
}

/// `OF_getpropbool`: whether the property exists.
pub fn OF_getpropbool(handle: i32, prop: &[u8]) -> bool {
    fdt_node_property(of_node(handle), prop).is_some()
}

/// `OF_getpropint`: a 32-bit property, or `defval`.
pub fn OF_getpropint(handle: i32, prop: &[u8], defval: u32) -> u32 {
    let mut val = [0u8; 4];
    if OF_getprop(handle, prop, &mut val) != 4 {
        return defval;
    }
    u32::from_be_bytes(val)
}

/// `OF_getpropintarray`: a property of 32-bit cells into `buf`, byte-swapped; returns the
/// property's length in bytes, or `-1` when it is missing or not a whole number of cells.
pub fn OF_getpropintarray(handle: i32, prop: &[u8], buf: &mut [u32]) -> i32 {
    let Some(data) = fdt_node_property(of_node(handle), prop) else {
        return -1;
    };
    if data.len() % 4 != 0 {
        return -1;
    }
    for (i, slot) in buf.iter_mut().take(data.len() / 4).enumerate() {
        *slot = cell(data, i) as u32;
    }
    data.len() as i32
}

/// `OF_getpropint64`: a 64-bit property, or `defval`.
pub fn OF_getpropint64(handle: i32, prop: &[u8], defval: u64) -> u64 {
    let mut val = [0u8; 8];
    if OF_getprop(handle, prop, &mut val) != 8 {
        return defval;
    }
    u64::from_be_bytes(val)
}

/// `OF_getpropint64array`: a property of 64-bit cells into `buf`, byte-swapped.
pub fn OF_getpropint64array(handle: i32, prop: &[u8], buf: &mut [u64]) -> i32 {
    let Some(data) = fdt_node_property(of_node(handle), prop) else {
        return -1;
    };
    if data.len() % 8 != 0 {
        return -1;
    }
    for (i, slot) in buf.iter_mut().take(data.len() / 8).enumerate() {
        *slot = (cell(data, 2 * i) << 32) | cell(data, 2 * i + 1);
    }
    data.len() as i32
}

/// `OF_is_compatible`.
pub fn OF_is_compatible(handle: i32, name: &[u8]) -> bool {
    fdt_is_compatible(of_node(handle), name)
}

/// `OF_is_enabled`: the node's `status` is not "disabled" or "reserved".
pub fn OF_is_enabled(handle: i32) -> bool {
    let mut status = [0u8; 32];
    if OF_getprop(handle, b"status", &mut status) > 0 {
        let s = status.split(|&c| c == 0).next().unwrap_or(&[]);
        if s == b"disabled" || s == b"reserved" {
            return false;
        }
    }
    true
}

/// `OF_getindex`: the index of `entry` in the string list property `prop`, `-1` when it is
/// not there; 0 when `entry` is `None`.
pub fn OF_getindex(handle: i32, entry: Option<&[u8]>, prop: &[u8]) -> i32 {
    let Some(entry) = entry else {
        return 0;
    };
    let Some(names) = fdt_node_property(of_node(handle), prop) else {
        return -1;
    };
    if names.is_empty() {
        return -1;
    }
    names
        .split(|&c| c == 0)
        .position(|name| name == entry)
        .map_or(-1, |idx| idx as i32)
}

/// boot(8)'s secrets in `/chosen` (`openbsd,sr-bootuuid`, `openbsd,sr-bootkey`: the C's
/// `initarm` copies them for softraid(4) and wipes them from the tree): the value of the
/// property `name` of the node at `path` in `blob`, copied into `out` when their lengths
/// match, then zeroed in `blob` (`explicit_bzero`) whatever its length; whether it was
/// copied. `blob` is not the installed tree but the boot's one mutable view of it
/// (`hal::mem::fdtblob::with_boot_mut`), before `fdt_init` adopts it. A blob that is not a
/// tree, or a missing node or property, copies and zeroes nothing.
pub fn fdt_take_property(blob: &mut [u8], path: &[u8], name: &[u8], out: &mut [u8]) -> bool {
    let Some((off, len)) = parse(blob).and_then(|t| property_at(&t, find_node(&t, path)?, name))
    else {
        return false;
    };
    let Some(value) = blob.get_mut(off..off + len) else {
        return false;
    };
    let copied = value.len() == out.len();
    if copied {
        out.copy_from_slice(value);
    }
    explicit_bzero(value);
    copied
}

/// A blob built by a test, the tree it installs, and the trees shaped like QEMU `virt`'s:
/// the fixture of the reader's tests and of the drivers' that read the tree (`bsd`'s adapter,
/// the interrupt core). The tree is process-wide, so an install holds a test lock until its
/// guard drops, which empties the tree again. The blobs are host memory, leaked, never kernel
/// memory: `setup_real_memory` has nothing to reset.
#[cfg(any(test, feature = "doubles"))]
pub mod testfdt {
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::vec::Vec;

    use super::{FDT_END, FDT_MAGIC, FDT_NODE_BEGIN, FDT_NODE_END, FDT_PROPERTY, fdt_init};

    /// Builds a blob: the structure block token by token, the strings block beside it.
    #[derive(Default)]
    pub struct FdtBuilder {
        structure: Vec<u8>,
        strings: Vec<u8>,
    }

    impl FdtBuilder {
        /// An empty structure block.
        pub fn new() -> Self {
            Self::default()
        }

        /// One big-endian word of the structure block.
        pub fn word(&mut self, w: u32) {
            self.structure.extend_from_slice(&w.to_be_bytes());
        }

        /// Pads the structure block to a word.
        fn pad(&mut self) {
            while !self.structure.len().is_multiple_of(4) {
                self.structure.push(0);
            }
        }

        /// `FDT_NODE_BEGIN` and the node's name.
        pub fn begin(&mut self, name: &[u8]) {
            self.word(FDT_NODE_BEGIN);
            self.structure.extend_from_slice(name);
            self.structure.push(0);
            self.pad();
        }

        /// `FDT_NODE_END`.
        pub fn end(&mut self) {
            self.word(FDT_NODE_END);
        }

        /// The offset of a new string in the strings block.
        fn string_off(&mut self, name: &[u8]) -> u32 {
            let off = self.strings.len() as u32;
            self.strings.extend_from_slice(name);
            self.strings.push(0);
            off
        }

        /// A property and its value.
        pub fn prop(&mut self, name: &[u8], value: &[u8]) {
            let off = self.string_off(name);
            self.word(FDT_PROPERTY);
            self.word(value.len() as u32);
            self.word(off);
            self.structure.extend_from_slice(value);
            self.pad();
        }

        /// A property of big-endian cells.
        pub fn prop_cells(&mut self, name: &[u8], cells: &[u32]) {
            let v: Vec<u8> = cells.iter().flat_map(|c| c.to_be_bytes()).collect();
            self.prop(name, &v);
        }

        /// The blob: a version 17 header, one empty memory reservation, the structure block
        /// ended by `FDT_END`, the strings.
        pub fn finish(mut self) -> Vec<u8> {
            self.word(FDT_END);
            let header_len = 40usize;
            let reserve_off = header_len;
            let reserve_len = 16; // one empty entry
            let struct_off = reserve_off + reserve_len;
            let strings_off = struct_off + self.structure.len();
            let size = strings_off + self.strings.len();
            let mut blob = Vec::with_capacity(size);
            for w in [
                FDT_MAGIC,
                size as u32,
                struct_off as u32,
                strings_off as u32,
                reserve_off as u32,
                17,
                16,
                0,
                self.strings.len() as u32,
                self.structure.len() as u32,
            ] {
                blob.extend_from_slice(&w.to_be_bytes());
            }
            blob.extend_from_slice(&[0u8; 16]);
            blob.extend_from_slice(&self.structure);
            blob.extend_from_slice(&self.strings);
            blob
        }
    }

    /// A tree with a root, an interrupt controller, a UART under a bus with ranges, and
    /// chosen.
    pub fn virt_like() -> Vec<u8> {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"#address-cells", &[2]);
        b.prop_cells(b"#size-cells", &[2]);
        b.prop(b"compatible", b"linux,dummy-virt\0");
        b.begin(b"intc@8000000");
        b.prop(b"compatible", b"arm,cortex-a15-gic\0");
        b.prop_cells(
            b"reg",
            &[0, 0x0800_0000, 0, 0x10000, 0, 0x0801_0000, 0, 0x10000],
        );
        b.prop_cells(b"#interrupt-cells", &[3]);
        b.prop(b"interrupt-controller", b"");
        b.prop_cells(b"phandle", &[1]);
        b.end();
        b.begin(b"soc");
        b.prop_cells(b"#address-cells", &[1]);
        b.prop_cells(b"#size-cells", &[1]);
        // 32-bit child addresses 0x1000_0000.. map to parent 0x9000_0000..
        b.prop_cells(b"ranges", &[0x1000_0000, 0, 0x0900_0000, 0x0100_0000]);
        b.begin(b"pl011@10000000");
        b.prop(b"compatible", b"arm,pl011\0arm,primecell\0");
        b.prop_cells(b"reg", &[0x1000_0000, 0x1000]);
        b.prop_cells(b"interrupts", &[0, 1, 4]);
        b.prop_cells(b"interrupt-parent", &[1]);
        b.prop(b"status", b"okay\0");
        b.end();
        b.end();
        b.begin(b"chosen");
        b.prop(b"stdout-path", b"/soc/pl011@10000000\0");
        b.end();
        b.end();
        b.finish()
    }

    /// The tree is process-wide: one test at a time holds it.
    static LOCK: Mutex<()> = Mutex::new(());

    /// A test's hold on the tree: the tree it installed (if any) until it drops, which
    /// empties it (`fdt_init(NULL)`).
    pub struct TreeGuard {
        /// What `fdt_init` answered.
        pub version: i32,
        /// The blob installed (leaked), empty when none was.
        pub blob: &'static [u8],
        _lock: MutexGuard<'static, ()>,
    }

    impl Drop for TreeGuard {
        fn drop(&mut self) {
            let _ = fdt_init(None);
        }
    }

    /// Installs `blob` as the tree (`fdt_init`), for the guard's life.
    pub fn install(blob: Vec<u8>) -> TreeGuard {
        let lock = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let blob: &'static [u8] = blob.leak();
        TreeGuard {
            version: fdt_init(Some(blob)),
            blob,
            _lock: lock,
        }
    }

    /// No tree, for the guard's life.
    pub fn empty() -> TreeGuard {
        let lock = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        TreeGuard {
            version: fdt_init(None),
            blob: &[],
            _lock: lock,
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests over blobs built here (`testfdt`), shaped like QEMU `virt`'s (a GIC and a
    // PL011), and over every damaged form of one.

    use std::vec::Vec;
    use std::{assert, assert_eq, assert_ne};

    use super::testfdt::{FdtBuilder, empty, install, virt_like};
    use super::*;

    #[test]
    fn parses_a_virt_like_tree() {
        let blob = virt_like();
        assert_eq!(fdt_check_head(&blob), 17);
        assert_eq!(fdt_get_size(&blob), blob.len());
        let g = install(blob);
        assert_eq!(g.version, 17);
        assert_eq!(fdt_blob().map(<[u8]>::as_ptr), Some(g.blob.as_ptr()));

        let root = fdt_next_node(None);
        assert!(root.is_some());
        assert_eq!(fdt_node_name(root), Some(&b""[..]));
        assert_eq!(fdt_find_node(b"/"), root);
        assert!(fdt_parent_node(root).is_none());

        let gic = fdt_find_node(b"/intc@8000000");
        assert!(gic.is_some());
        assert_eq!(
            fdt_find_node(b"/intc"),
            gic,
            "a match without the unit address"
        );
        assert!(fdt_is_compatible(gic, b"arm,cortex-a15-gic"));
        assert!(!fdt_is_compatible(gic, b"arm,gic-v3"));
        assert_eq!(fdt_node_property_int(gic, b"#interrupt-cells"), Some(3));
        assert!(fdt_node_property(gic, b"interrupt-controller").is_some());
        assert!(fdt_node_property(gic, b"nonsense").is_none());
        assert_eq!(fdt_find_phandle(1), gic);
        assert!(fdt_find_phandle(7).is_none());

        assert_eq!(
            fdt_get_reg(gic, 0),
            Ok(FdtReg {
                addr: 0x0800_0000,
                size: 0x10000
            })
        );
        assert_eq!(fdt_get_reg(gic, 1).map(|r| r.addr), Ok(0x0801_0000));
        assert_eq!(fdt_get_reg(gic, 2), Err(Errno::EINVAL));

        let uart = fdt_find_node(b"/soc/pl011@10000000");
        assert!(uart.is_some());
        assert_eq!(fdt_find_node(b"/soc/pl011"), uart);
        assert_eq!(
            fdt_find_node(b"//soc//pl011/"),
            uart,
            "runs of '/' as the C"
        );
        assert_eq!(fdt_parent_node(uart), fdt_find_node(b"/soc"));
        assert_eq!(fdt_get_cells(uart), (1, 1));
        assert_eq!(fdt_get_cells(gic), (2, 2));
        assert_eq!(
            fdt_get_reg(uart, 0),
            Ok(FdtReg {
                addr: 0x0900_0000,
                size: 0x1000
            }),
            "translated through the bus's ranges"
        );
        let mut ints = [0i32; 3];
        assert_eq!(
            fdt_node_property_ints(uart, b"interrupts", &mut ints),
            Some(3)
        );
        assert_eq!(ints, [0, 1, 4]);
        let mut two = [0i32; 2];
        assert_eq!(
            fdt_node_property_ints(uart, b"interrupts", &mut two),
            Some(2)
        );

        assert!(fdt_find_node(b"/nowhere").is_none());
        assert!(fdt_find_node(b"relative").is_none());
        assert!(fdt_child_node(fdt_find_node(b"/chosen")).is_none());
        assert_eq!(fdt_next_node(gic), fdt_find_node(b"/soc"));
        assert_eq!(
            fdt_next_node(fdt_find_node(b"/soc")),
            fdt_find_node(b"/chosen")
        );
        assert!(fdt_next_node(fdt_find_node(b"/chosen")).is_none());
        assert_eq!(of_fdt_node(OF_finddevice(b"/soc")), fdt_find_node(b"/soc"));
    }

    #[test]
    fn openfirmware_handles() {
        let g = install(virt_like());
        assert_eq!(g.version, 17);

        let root = OF_peer(0);
        assert_ne!(root, 0);
        let gic = OF_child(root);
        assert_eq!(gic, OF_finddevice(b"/intc@8000000"));
        assert_eq!(OF_parent(gic), root);
        assert_eq!(OF_peer(gic), OF_finddevice(b"/soc"));
        assert_eq!(OF_finddevice(b"/nope"), -1);
        assert_eq!(OF_getnodebyname(0, b"soc"), OF_finddevice(b"/soc"));
        assert_eq!(OF_getnodebyname(0, b"intc"), gic);
        assert_eq!(OF_getnodebyname(0, b"zzz"), 0);
        assert_eq!(OF_getnodebyphandle(1), gic);

        assert_eq!(OF_getpropint(gic, b"#interrupt-cells", 0), 3);
        assert_eq!(OF_getpropint(gic, b"missing", 42), 42);
        assert!(OF_getpropbool(gic, b"interrupt-controller"));
        assert!(!OF_getpropbool(gic, b"missing"));
        assert!(OF_is_compatible(gic, b"arm,cortex-a15-gic"));
        assert!(OF_is_enabled(gic));
        let mut regs = [0u64; 4];
        assert_eq!(OF_getpropint64array(gic, b"reg", &mut regs), 32);
        assert_eq!(regs, [0x0800_0000, 0x10000, 0x0801_0000, 0x10000]);
        assert_eq!(OF_getpropint64(gic, b"phandle", 9), 9, "not 8 bytes");

        let uart = OF_finddevice(b"/soc/pl011@10000000");
        assert_eq!(OF_getproplen(uart, b"interrupts"), 12);
        let mut cells = [0u32; 4];
        assert_eq!(OF_getpropintarray(uart, b"interrupts", &mut cells), 12);
        assert_eq!(&cells[..3], &[0, 1, 4]);
        assert_eq!(OF_getpropintarray(uart, b"missing", &mut cells), -1);
        assert_eq!(OF_getpropint(uart, b"interrupt-parent", 0), 1);
        assert!(OF_is_enabled(uart));
        assert_eq!(OF_getindex(uart, Some(b"arm,primecell"), b"compatible"), 1);
        assert_eq!(OF_getindex(uart, Some(b"nope"), b"compatible"), -1);
        assert_eq!(OF_getindex(uart, None, b"compatible"), 0);

        // the synthesized "name" of a node without one
        assert_eq!(OF_getproplen(uart, b"name"), 6);
        let mut name = [0u8; 16];
        assert_eq!(OF_getprop(uart, b"name", &mut name), 6);
        assert_eq!(&name[..6], b"pl011\0");
        let mut short = [0u8; 3];
        assert_eq!(OF_getprop(uart, b"name", &mut short), 6);
        assert_eq!(&short, b"pl\0");
        assert_eq!(OF_getprop(uart, b"name", &mut []), 6);

        let mut buf = [0u8; 64];
        let chosen = OF_finddevice(b"/chosen");
        let len = OF_getprop(chosen, b"stdout-path", &mut buf);
        assert_eq!(&buf[..len as usize], b"/soc/pl011@10000000\0");
    }

    #[test]
    fn a_disabled_node_and_nops_between_tokens() {
        let mut b = FdtBuilder::new();
        b.word(FDT_NOP);
        b.begin(b"");
        b.word(FDT_NOP);
        b.prop(b"model", b"x\0");
        b.word(FDT_NOP);
        b.begin(b"a@1");
        b.prop(b"status", b"disabled\0");
        b.begin(b"b");
        b.begin(b"c");
        b.end();
        b.word(FDT_NOP);
        b.end();
        b.end();
        b.word(FDT_NOP);
        b.begin(b"d");
        b.prop(b"status", b"reserved\0");
        b.end();
        b.end();
        let _g = install(b.finish());
        let a = OF_finddevice(b"/a");
        assert!(a > 0);
        assert!(!OF_is_enabled(a));
        assert!(!OF_is_enabled(OF_finddevice(b"/d")));
        assert!(OF_is_enabled(OF_peer(0)), "no status");
        assert_eq!(
            OF_peer(a),
            OF_finddevice(b"/d"),
            "the subtree skipped whole"
        );
        assert_eq!(OF_parent(OF_finddevice(b"/a/b/c")), OF_finddevice(b"/a/b"));
        assert_eq!(OF_getproplen(OF_peer(0), b"model"), 2);
    }

    #[test]
    fn no_tree_answers_nothing() {
        let g = empty();
        assert_eq!(g.version, 0);
        assert!(fdt_blob().is_none());
        assert!(fdt_next_node(None).is_none());
        assert!(fdt_find_node(b"/").is_none());
        assert!(fdt_find_phandle(1).is_none());
        assert_eq!(fdt_get_cells(None), (1, 1));
        assert_eq!(fdt_get_reg(None, 0), Err(Errno::EINVAL));
        assert_eq!(OF_peer(0), 0);
        assert_eq!(OF_finddevice(b"/"), -1);
        assert_eq!(OF_getproplen(4, b"name"), -1);
        assert_eq!(OF_getprop(4, b"name", &mut [0; 8]), -1);
        assert_eq!(OF_getpropint(4, b"x", 5), 5);
        assert!(OF_is_enabled(4));
        assert_eq!(OF_getindex(4, None, b"x"), 0);
        assert_eq!(OF_getindex(4, Some(b"y"), b"x"), -1);
        assert_eq!(fdt_init(None), 0);
    }

    #[test]
    fn handles_that_name_no_node() {
        let g = install(virt_like());
        let len = g.blob.len() as i32;
        for h in [0, -1, i32::MIN, 1, 3, 5, len, len + 4, i32::MAX] {
            assert_eq!(OF_child(h), 0, "handle {h}");
            assert_eq!(OF_parent(h), 0, "handle {h}");
            assert_eq!(OF_getproplen(h, b"compatible"), -1, "handle {h}");
            assert_eq!(OF_getproplen(h, b"name"), -1, "handle {h}");
            assert!(!OF_is_compatible(h, b"arm,pl011"), "handle {h}");
            if h != 0 {
                assert_eq!(OF_peer(h), 0, "handle {h}");
                assert_eq!(OF_getnodebyname(h, b"soc"), 0, "handle {h}");
            }
        }
        assert!(of_fdt_node(0).is_none());
        assert!(of_fdt_node(-4).is_none());
    }

    /// A version 17 blob's header words, replaced one at a time.
    fn with_header_word(i: usize, w: u32) -> Vec<u8> {
        let mut blob = virt_like();
        blob[4 * i..4 * i + 4].copy_from_slice(&w.to_be_bytes());
        blob
    }

    #[test]
    fn a_header_that_does_not_hold_is_no_tree() {
        assert_eq!(fdt_check_head(&[]), 0);
        assert_eq!(fdt_check_head(&virt_like()[..39]), 0, "cut in the header");
        assert_eq!(
            fdt_check_head(&with_header_word(0, 0xfeed_d00d)),
            0,
            "magic"
        );
        assert_eq!(
            fdt_check_head(&with_header_word(5, 18)),
            0,
            "a newer version"
        );
        assert_eq!(
            fdt_check_head(&with_header_word(2, 0x7fff_fff0)),
            0,
            "struct past the end"
        );
        assert_eq!(
            fdt_check_head(&with_header_word(2, 40)),
            0,
            "struct at the reservations"
        );
        assert_eq!(
            fdt_check_head(&with_header_word(9, 0)),
            0,
            "no FDT_END word"
        );
        assert_eq!(
            fdt_check_head(&with_header_word(9, 8)),
            0,
            "FDT_END missing"
        );
        assert_eq!(
            fdt_check_head(&with_header_word(9, u32::MAX)),
            0,
            "struct size past the end"
        );
        // Version 16: no end signature is looked for.
        assert_eq!(fdt_check_head(&with_header_word(5, 16)), 16);
        assert_eq!(fdt_get_size(&with_header_word(0, 0)), 0);
        let g = install(with_header_word(0, 0));
        assert_eq!(g.version, 0);
        assert!(fdt_blob().is_none());
    }

    #[test]
    fn a_value_a_name_or_a_string_past_the_end_is_none() {
        // A property whose size runs past the blob.
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"big", &[1]);
        b.end();
        let mut blob = b.finish();
        let size_at = 56 + 4 + 4 + 4; // reservations end, the root's token and name, FDT_PROPERTY
        blob[size_at..size_at + 4].copy_from_slice(&0x7fff_0000u32.to_be_bytes());
        let g = install(blob);
        let root = fdt_next_node(None);
        assert!(root.is_some());
        assert!(fdt_node_property(root, b"big").is_none());
        assert!(fdt_node_property(root, b"other").is_none());
        drop(g);

        // A string id past the strings block, and one inside its size but past the blob.
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"p", &[1]);
        b.end();
        let mut blob = b.finish();
        let id_at = 56 + 8 + 8;
        blob[id_at..id_at + 4].copy_from_slice(&1000u32.to_be_bytes());
        let g = install(blob.clone());
        assert!(fdt_node_property(fdt_next_node(None), b"p").is_none());
        drop(g);
        blob[32..36].copy_from_slice(&5000u32.to_be_bytes()); // fh_strings_size
        let _g = install(blob);
        assert!(fdt_node_property(fdt_next_node(None), b"p").is_none());
    }

    #[test]
    fn a_name_without_its_nul_is_none() {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.end();
        let mut blob = b.finish();
        // The structure block's root name runs to the blob's end with no NUL.
        let n = blob.len();
        for byte in &mut blob[60..n] {
            *byte = b'x';
        }
        assert_eq!(fdt_check_head(&blob), 0, "no FDT_END any more");
        let mut v16 = blob.clone();
        v16[20..24].copy_from_slice(&16u32.to_be_bytes());
        let g = install(v16);
        assert_eq!(g.version, 16);
        let root = fdt_next_node(None);
        assert!(root.is_some());
        assert!(fdt_node_name(root).is_none());
        assert!(fdt_child_node(root).is_none());
        assert!(fdt_next_node(root).is_none());
        assert!(fdt_find_node(b"/a").is_none());
        assert_eq!(OF_getproplen(OF_peer(0), b"name"), -1);
    }

    #[test]
    fn reg_errors_and_translations() {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop_cells(b"#address-cells", &[1]);
        b.prop_cells(b"#size-cells", &[1]);
        b.prop_cells(b"reg", &[1, 2]);
        b.begin(b"wide");
        b.prop_cells(b"#address-cells", &[3]);
        b.begin(b"x");
        b.prop_cells(b"reg", &[1, 2, 3, 4]);
        b.end();
        b.end();
        b.begin(b"flat");
        b.prop(b"ranges", b"");
        b.begin(b"y");
        b.prop_cells(b"reg", &[0x100, 0x10]);
        b.end();
        b.end();
        b.begin(b"short");
        b.prop_cells(b"ranges", &[0, 0]);
        b.begin(b"z");
        b.prop_cells(b"reg", &[0x100, 0x10]);
        b.end();
        b.end();
        b.begin(b"window");
        b.prop_cells(b"ranges", &[0x1000, 0x8000, 0x100]);
        b.begin(b"in");
        b.prop_cells(b"reg", &[0x1010, 0x10]);
        b.end();
        b.begin(b"out");
        b.prop_cells(b"reg", &[0x10f8, 0x10]);
        b.end();
        b.end();
        b.end();
        let _g = install(b.finish());
        let n = |p: &[u8]| fdt_find_node(p);
        assert_eq!(
            fdt_get_reg(n(b"/"), 0),
            Err(Errno::EINVAL),
            "the root has no parent"
        );
        assert_eq!(
            fdt_get_reg(n(b"/wide/x"), 0),
            Err(Errno::EINVAL),
            "3 address cells"
        );
        assert_eq!(
            fdt_get_reg(n(b"/flat/y"), 0),
            Ok(FdtReg {
                addr: 0x100,
                size: 0x10
            }),
            "empty ranges: 1:1"
        );
        assert_eq!(
            fdt_get_reg(n(b"/short/z"), 0),
            Err(Errno::ESRCH),
            "not one whole range"
        );
        assert_eq!(fdt_get_reg(n(b"/window/in"), 0).map(|r| r.addr), Ok(0x8010));
        assert_eq!(
            fdt_get_reg(n(b"/window/out"), 0),
            Err(Errno::ESRCH),
            "past the window"
        );
        assert_eq!(
            fdt_get_reg(n(b"/window/in"), usize::MAX),
            Err(Errno::EINVAL)
        );
        assert_eq!(fdt_get_reg(n(b"/window/in"), 1), Err(Errno::EINVAL));
    }

    /// Every node, name, property and handle of the installed tree read, and the walks run:
    /// what a damaged tree must survive.
    fn walk_everything(blob: &[u8]) {
        fn walk(node: Option<FdtNode>, depth: usize) {
            let mut child = fdt_child_node(node);
            while let Some(c) = child {
                let _ = fdt_node_name(Some(c));
                let _ = fdt_node_property(Some(c), b"compatible");
                let _ = fdt_node_property_int(Some(c), b"#address-cells");
                let _ = fdt_is_compatible(Some(c), b"arm,pl011");
                let _ = fdt_get_reg(Some(c), 0);
                let _ = fdt_get_cells(Some(c));
                let _ = fdt_parent_node(Some(c));
                if depth < 64 {
                    walk(Some(c), depth + 1);
                }
                let next = fdt_next_node(Some(c));
                assert!(
                    next.is_none_or(|n| n.offset() > c.offset()),
                    "a walk moves forward"
                );
                child = next;
            }
        }
        let root = fdt_next_node(None);
        let _ = fdt_node_name(root);
        walk(root, 0);
        let _ = fdt_find_node(b"/soc/pl011");
        let _ = fdt_find_phandle(1);
        let mut buf = [0u8; 16];
        for h in (-4..blob.len() as i32 + 8).step_by(4) {
            let _ = OF_getprop(h, b"name", &mut buf);
            let _ = OF_getprop(h, b"compatible", &mut buf);
            let _ = OF_peer(h);
            let _ = OF_child(h);
            let _ = OF_getindex(h, Some(b"arm,primecell"), b"compatible");
        }
    }

    #[test]
    fn every_prefix_of_a_tree_is_read_without_a_fault() {
        let blob = virt_like();
        for len in 0..=blob.len() {
            let g = install(blob[..len].to_vec());
            if g.version != 0 {
                walk_everything(g.blob);
            }
            assert_eq!(fdt_check_head(&blob[..len]) != 0, g.version != 0);
        }
        // A version 16 tree has no end signature: every prefix past the header installs.
        let mut v16 = blob.clone();
        v16[20..24].copy_from_slice(&16u32.to_be_bytes());
        for len in 0..=v16.len() {
            let g = install(v16[..len].to_vec());
            walk_everything(g.blob);
        }
    }

    #[test]
    fn every_damaged_word_is_read_without_a_fault() {
        let mut v16 = virt_like();
        v16[20..24].copy_from_slice(&16u32.to_be_bytes());
        for at in (40..v16.len() - 3).step_by(4) {
            for w in [
                0,
                FDT_NODE_BEGIN,
                FDT_NODE_END,
                FDT_PROPERTY,
                FDT_NOP,
                FDT_END,
                u32::MAX,
            ] {
                let mut blob = v16.clone();
                blob[at..at + 4].copy_from_slice(&w.to_be_bytes());
                let g = install(blob);
                walk_everything(g.blob);
            }
        }
    }

    /// A tree whose `/chosen` holds boot(8)'s two secrets between other properties.
    fn with_secrets() -> Vec<u8> {
        let mut b = FdtBuilder::new();
        b.begin(b"");
        b.prop(b"model", b"virt\0");
        b.begin(b"chosen");
        b.prop(b"bootargs", b"-s\0");
        b.prop(b"openbsd,sr-bootuuid", &[0x11; 16]);
        b.prop(b"openbsd,sr-bootkey", &[0x5a; 64]);
        b.prop(b"openbsd,bootduid", &[0x22; 8]);
        b.end();
        b.end();
        b.finish()
    }

    /// The bytes where `a` and `b` differ.
    fn differing(a: &[u8], b: &[u8]) -> Vec<usize> {
        (0..a.len()).filter(|&i| a[i] != b[i]).collect()
    }

    #[test]
    fn the_wipe_zeroes_both_secrets_and_nothing_else() {
        let before = with_secrets();
        let mut blob = before.clone();
        let mut uuid = [0u8; 16];
        let mut key = [0u8; 64];
        assert!(fdt_take_property(
            &mut blob,
            b"/chosen",
            b"openbsd,sr-bootuuid",
            &mut uuid
        ));
        assert!(fdt_take_property(
            &mut blob,
            b"/chosen",
            b"openbsd,sr-bootkey",
            &mut key
        ));
        assert_eq!(uuid, [0x11; 16]);
        assert_eq!(key, [0x5a; 64]);
        let changed = differing(&before, &blob);
        assert_eq!(changed.len(), 16 + 64, "only the two values changed");
        assert!(changed.iter().all(|&i| blob[i] == 0), "to zeroes");
        // The tree reads as before, the two values zeroed in place.
        let _g = install(blob);
        let chosen = fdt_find_node(b"/chosen");
        assert_eq!(
            fdt_node_property(chosen, b"openbsd,sr-bootuuid"),
            Some(&[0u8; 16][..])
        );
        assert_eq!(
            fdt_node_property(chosen, b"openbsd,sr-bootkey"),
            Some(&[0u8; 64][..])
        );
        assert_eq!(
            fdt_node_property(chosen, b"openbsd,bootduid"),
            Some(&[0x22u8; 8][..])
        );
        assert_eq!(fdt_node_property(chosen, b"bootargs"), Some(&b"-s\0"[..]));
    }

    #[test]
    fn a_secret_of_another_length_is_zeroed_not_copied() {
        let before = with_secrets();
        let mut blob = before.clone();
        let mut short = [7u8; 32];
        assert!(!fdt_take_property(
            &mut blob,
            b"/chosen",
            b"openbsd,sr-bootkey",
            &mut short
        ));
        assert_eq!(short, [7; 32], "not copied");
        let changed = differing(&before, &blob);
        assert_eq!(changed.len(), 64, "zeroed all the same, as the C");
        assert!(changed.iter().all(|&i| blob[i] == 0));
    }

    #[test]
    fn no_secret_and_no_tree_change_nothing() {
        let before = with_secrets();
        let mut blob = before.clone();
        let mut out = [0u8; 16];
        assert!(!fdt_take_property(
            &mut blob,
            b"/chosen",
            b"openbsd,missing",
            &mut out
        ));
        assert!(!fdt_take_property(
            &mut blob,
            b"/nowhere",
            b"openbsd,sr-bootuuid",
            &mut out
        ));
        assert_eq!(blob, before);
        let mut bad = with_header_word(0, 0);
        let bad_before = bad.clone();
        assert!(!fdt_take_property(
            &mut bad,
            b"/chosen",
            b"stdout-path",
            &mut [0u8; 20]
        ));
        assert_eq!(bad, bad_before);
    }

    #[test]
    fn names_compare_as_strcmp() {
        let b = b"abc\0abd\0ab";
        assert!(name_eq(b, b"abc", 0));
        assert!(!name_eq(b, b"ab", 0), "a prefix is no match");
        assert!(!name_eq(b, b"abcd", 0));
        assert!(name_eq(b, b"abd", 4));
        assert!(!name_eq(b, b"ab", 8), "no NUL before the end");
        assert!(!name_eq(b, b"a\0c", 0), "a NUL in the name");
        assert!(!name_eq(b, b"", usize::MAX));
        assert!(name_eq(b, b"", 3));
    }
}
/* </TESTS> */
