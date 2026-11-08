/*	$KAME: uipc_mbuf2.c,v 1.29 2001/02/14 13:42:10 itojun Exp $	*/
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
 * Copyright (C) 1999 WIDE Project.
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
 * 3. Neither the name of the project nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE PROJECT AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE PROJECT OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/*
 * Copyright (c) 1982, 1986, 1988, 1991, 1993
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
 *	@(#)uipc_mbuf.c	8.4 (Berkeley) 2/14/95
 */
/* </LICENSES> */

/* <CODE> */
//! `m_pulldown` and the packet tags (`kern/uipc_mbuf2.c`), for `bsd`'s legacy code over the
//! kernel's (`kernel::net::mbuf2`, N6.1).
//!
//! Upstream: sys/kern/uipc_mbuf2.c @ 3ce1f3f79392
//!
//! LZ's names and signatures over `&'static Mbuf` and `&'static MTag` (`sys/sys/mbuf.rs`'s
//! boundary): a tag `m_tag_get` gives `bsd` is parked until `m_tag_prepend` links it into
//! its packet's chain, which owns it from then on.
//!
//! ## Deviations
//! - `m_pulldown`'s `offp` is `Option<&mut i32>`: `None` is the C's NULL, which also changes
//!   where the data may start (LZ's).
//! - `m_tag_delete` is safe: the kernel finds the tag by its address on the packet's chain
//!   (`panic(9)` when it is not there, where the C's `SLIST_REMOVE` corrupts the list).

use kernel::net::mbuf2 as k;

use crate::sys::errno::Errno;
use crate::sys::mbuf::{MTag, Mbuf, mut_of, park, unpark};

pub use kernel::net::mbuf2::{m_tag_find, m_tag_first, m_tag_next};

/// `m_pulldown`: ensure that `[off, off + len]` is contiguous on the mbuf chain `m`. Packet
/// chain before `off` is kept untouched. If `offp` is `None`, the target will start at
/// `<retval, 0>` on resulting chain. If `offp` is `Some`, the target will start at
/// `<retval, *offp>` on resulting chain.
///
/// On error return (`None` return value), original `m` will be freed.
pub fn m_pulldown(
    m: &'static Mbuf,
    off: i32,
    len: i32,
    offp: Option<&mut i32>,
) -> Option<&'static Mbuf> {
    let (head, at, o) = k::m_pulldown(unpark(m), off, len, offp.is_some())?;
    let mut n: &'static Mbuf = park(head);
    for _ in 0..at {
        n = n.m_next().get()?;
    }
    if let Some(offp) = offp {
        *offp = o;
    }
    Some(n)
}

/// `m_tag_get`: get a packet tag structure along with specified data following.
pub fn m_tag_get(type_: u16, len: i32, wait: i32) -> Option<&'static MTag> {
    k::m_tag_get(type_, len, wait).map(park)
}

/// `m_tag_prepend`: prepend a packet tag (`t`, from `m_tag_get`).
pub fn m_tag_prepend(m: &Mbuf, t: &'static MTag) {
    k::m_tag_prepend(mut_of(m), unpark(t));
}

/// `m_tag_delete`: unlink and free a packet tag on `m`'s list.
pub fn m_tag_delete(m: &Mbuf, t: &MTag) {
    k::m_tag_delete(mut_of(m), core::ptr::from_ref(t));
}

/// `m_tag_delete_chain`: unlink and free a packet tag chain.
pub fn m_tag_delete_chain(m: &Mbuf) {
    k::m_tag_delete_chain(mut_of(m));
}

/// `m_tag_copy`: copy a single tag.
pub fn m_tag_copy(t: &MTag, wait: i32) -> Option<&'static MTag> {
    k::m_tag_copy(t, wait).map(park)
}

/// `m_tag_copy_chain`: copy two tag chains.
pub fn m_tag_copy_chain(to: &Mbuf, from: &Mbuf, wait: i32) -> Result<(), Errno> {
    k::m_tag_copy_chain(mut_of(to), from, wait)
}
/* </CODE> */
