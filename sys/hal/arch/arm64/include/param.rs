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

/*-
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * William Jolitz.
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
 */
/* </LICENSES> */

/* <CODE> */
//! arm64 `<machine/param.h>`: the page geometry and the kernel's base, the part `hal` needs.
//!
//! Upstream: sys/arch/arm64/include/param.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/include/param.rs@3c62ede68802
//!
//! ## Redesign
//! - LZ's `include/param.rs` holds every constant of the header and the `MachineParam`
//!   implementation for `bsd`'s `Machine`. The page geometry and `KERNBASE` moved here at N3.1
//!   (decision 27's pattern), since `hal`'s page arithmetic (`sys/hal/vm/param.rs`), its pmap
//!   and `pte.rs` (`PTE_RPGN` reads `PAGE_MASK`) need them and `hal` names nothing of `bsd`;
//!   `bsd`'s file keeps the rest and the trait implementation (the orphan rule pins it to
//!   `bsd`) and re-exports these. `sys/machine/mod.rs` asserts at compile time that the two
//!   agree.
//! - N4.7a: `MSGBUFSIZE`, the size of `kernel::log`'s message buffer (a static there).

/// log2 of the page size.
pub const PAGE_SHIFT: usize = 12;
/// Bytes per page.
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
/// Byte offset mask within a page.
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

/// Start of kernel virtual space.
pub const KERNBASE: usize = 0xffff_ff80_0000_0000;

/// Default message buffer size.
pub const MSGBUFSIZE: usize = 16 * PAGE_SIZE;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/arch/arm64/include/param.h");
        let ours: &[(&str, i64)] = &[
            ("PAGE_SHIFT", PAGE_SHIFT as i64),
            ("PAGE_SIZE", PAGE_SIZE as i64),
            ("KERNBASE", KERNBASE as i64),
        ];
        for (name, value) in ours {
            assert_eq!(crate::reftest::int(&defs, name), Some(*value), "{name}");
        }
    }
}
/* </TESTS> */
