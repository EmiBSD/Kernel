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
//! `m_pulldown` and the packet tags (`kern/uipc_mbuf2.c`).
//!
//! Upstream: sys/kern/uipc_mbuf2.c @ 3ce1f3f79392
//! LZ: sys/kern/uipc_mbuf2.rs@3c62ede68802
//!
//! `m_pulldown` makes a range of a packet contiguous without touching what lies before it (the
//! KAME alternative to `m_pullup`). Packet tags are small records from `mtagpl`
//! ([`super::mbuf::MTag`]) hung off the packet header (`ph_tags`), with `ph_tagsset` as the
//! summary of their types.
//!
//! ## Redesign
//! - LZ's `&'static MTag` from `pool_get` (13 `unsafe`: the item cast, the list's links, the
//!   data's raw copies) becomes an owned [`MTagBox`]: `m_tag_get` returns it,
//!   `m_tag_prepend` consumes it into the header's chain (`ph_tags`, a bare
//!   `hal::owned::SlistLink` the header owns, one word as the C's `SLIST_HEAD`), whose drop
//!   frees the tags one at a time (`m_tag_delete_chain`). `m_tag_delete` names the tag by its
//!   address (it is the chain's while the caller holds the mbuf) and walks to its predecessor
//!   as `SLIST_REMOVE` does, O(n); `m_tag_copy_chain` appends through a cursor on the last
//!   link, O(1) a tag (the C's `tprev`).
//! - `m_pulldown` takes the chain's head box and returns it with the position of the mbuf
//!   holding the range, as its index in the chain (`m_getptr`'s shape), and the offset there
//!   (the C's return value and `*offp`); `None` frees the chain as the C does. The data moves
//!   between an mbuf and its successor with the successor taken off its link while both are
//!   changed (O(1)): one mbuf cannot be lent mutably while its link lends the next.

use core::ptr;

use hal::malloc::M_WAITOK;
use hal::owned::SlistLink;
use hal::pool::{PR_NOWAIT, PR_WAITOK, PoolBox};
use hal::sys::errno::Errno;

use crate::assert::{kassert, kpanic};
use crate::net::mbuf::{
    M_DONTWAIT, MAXMCLBYTES, MHLEN, MLEN, MTAGPL, MTag, MbstatCounters, Mbuf, MbufBox,
    PACKET_TAG_MAXSIZE, m_adj, m_copydata, m_dup_pkthdr, m_get, m_gethdr, m_leadingspace,
    m_trailingspace, mbstat_inc, mclgetl,
};

/// An owned packet tag (`struct m_tag *` from `m_tag_get`, before `m_tag_prepend`).
pub type MTagBox = PoolBox<MTag>;

/// The mbuf at `loc` bytes into the chain, lent mutably, its offset there and its index in
/// the chain (`m_getptr`'s walk, the index counted as it goes).
fn getptr_at(m: &mut Mbuf, loc: i32) -> Option<(&mut Mbuf, i32, usize)> {
    let mut m = m;
    let mut loc = loc;
    let mut i = 0usize;
    while loc >= 0 {
        let mlen = m.m_len().get() as i32;
        // Normal end of search
        if mlen > loc {
            return Some((m, loc, i));
        }
        loc -= mlen;

        if !m.m_next.is_some() {
            // Point at the end of valid data
            return (loc == 0).then_some((m, mlen, i));
        }
        m = m.m_next.get_mut()?;
        i += 1;
    }

    None
}

/// The `i`th mbuf of the chain (0: `m`), lent mutably.
fn nth_mut(m: &mut Mbuf, i: usize) -> Option<&mut Mbuf> {
    let mut m = m;
    for _ in 0..i {
        m = m.m_next.get_mut()?;
    }
    Some(m)
}

/// `m_pulldown`: ensure that `[off, off + len]` is contiguous on the mbuf chain `m`. Packet
/// chain before `off` is kept untouched. If `has_offp` is false, the target will start at
/// `<retval, 0>` on resulting chain; if true, at `<retval, *offp>`. The result is the chain's
/// head, the index of `retval` in it and the offset there.
///
/// On error return (`None`), original `m` will be freed.
///
/// XXX `m_trailingspace`/`m_leadingspace` on shared cluster (sharedcluster)
pub fn m_pulldown(
    mut m: MbufBox,
    off: i32,
    len: i32,
    has_offp: bool,
) -> Option<(MbufBox, usize, i32)> {
    let head_type = i32::from(m.m_type().get());

    // mbuf chain too short: m freed
    let (n, off, i) = getptr_at(&mut m, off)?;

    let sharedcluster = n.is_readonly();
    let nlen = n.m_len().get() as i32;

    // the target data is on <n, off>. if we got enough data on the mbuf "n", we're done.
    let (at, off) = 'ok: {
        if (off == 0 || has_offp) && len <= nlen - off && !sharedcluster {
            break 'ok (i, off);
        }

        // when len <= n->m_len - off and off != 0, it is a special case. len bytes from
        // <n, off> sits in single mbuf, but the caller does not like the starting position
        // (off). chop the current mbuf into two pieces, set off to 0.
        if len <= nlen - off {
            mbstat_inc(MbstatCounters::MbsPulldownAlloc as usize);
            // ENOBUFS: m freed
            let mut o = m_dup1(n, off, nlen - off, M_DONTWAIT)?;
            n.m_len().set(off as u32);
            if let Some(rest) = n.m_next.take() {
                drop(o.m_next.set(rest));
            }
            drop(n.m_next.set(o));
            break 'ok (i + 1, 0);
        }

        // we need to take hlen from <n, off> and tlen from <n->m_next, 0>, and construct
        // contiguous mbuf with m_len == len. note that hlen + tlen == len, and tlen > 0.
        let hlen = nlen - off;
        let tlen = len - hlen;

        // ensure that we have enough trailing data on mbuf chain. if not, we can do nothing
        // about the chain.
        let mut olen = 0i32;
        let mut o = n.m_next.get();
        while let Some(oo) = o {
            olen += oo.m_len().get() as i32;
            o = oo.m_next.get();
        }
        if hlen + olen < len {
            return None; // mbuf chain too short
        }
        // tlen > 0 bytes follow n, so it has a successor: taken off its link while both
        // change.
        let mut next = n.m_next.take()?;

        // easy cases first. we need to use m_copydata() to get data from <n->m_next, 0>.
        if (off == 0 || has_offp) && m_trailingspace(n) >= tlen && !sharedcluster {
            mbstat_inc(MbstatCounters::MbsPulldownCopy as usize);
            let at = n.data_off() + n.m_len().get() as usize;
            match n
                .storage_mut()
                .and_then(|s| s.get_mut(at..at + tlen as usize))
            {
                Some(dst) => m_copydata(&next, 0, dst),
                None => kpanic(format_args!("m_pulldown: no trailing space")),
            }
            n.m_len().set(n.m_len().get() + tlen as u32);
            m_adj(&next, tlen);
            drop(n.m_next.set(next));
            break 'ok (i, off);
        }
        if (off == 0 || has_offp)
            && m_leadingspace(&next) >= hlen
            && !sharedcluster
            && next.m_len().get() as i32 >= tlen
        {
            next.data_add(-(hlen as isize));
            next.m_len().set(next.m_len().get() + hlen as u32);
            mbstat_inc(MbstatCounters::MbsPulldownCopy as usize);
            let from = n.data_off() + off as usize;
            match next.data_mut() {
                Some(dst) => {
                    dst[..hlen as usize].copy_from_slice(&n.storage()[from..from + hlen as usize]);
                }
                None => kpanic(format_args!("m_pulldown: the next mbuf read-only")),
            }
            n.m_len().set(n.m_len().get() - hlen as u32);
            drop(n.m_next.set(next));
            break 'ok (i + 1, 0);
        }

        // now, we need to do the hard way. don't m_copym as there's no room on both ends.
        if len > MAXMCLBYTES as i32 {
            return None;
        }
        mbstat_inc(MbstatCounters::MbsPulldownAlloc as usize);
        // ENOBUFS: m (and next) freed
        let mut o = m_get(M_DONTWAIT, head_type)?;
        if len > MLEN as i32 && !mclgetl(&mut o, M_DONTWAIT, len as u32) {
            return None;
        }
        // get hlen from <n, off> into <o, 0>
        o.m_len().set(len as u32);
        let from = n.data_off() + off as usize;
        match o.data_mut() {
            Some(dst) => {
                dst[..hlen as usize].copy_from_slice(&n.storage()[from..from + hlen as usize]);
                // get tlen from <n->m_next, 0> into <o, hlen>
                m_copydata(&next, 0, &mut dst[hlen as usize..]);
            }
            None => kpanic(format_args!("m_pulldown: a fresh mbuf read-only")),
        }
        n.m_len().set(n.m_len().get() - hlen as u32);
        m_adj(&next, tlen);
        drop(o.m_next.set(next));
        drop(n.m_next.set(o));
        (i + 1, 0)
    };

    // ok:
    kassert!(nth_mut(&mut m, at).is_some_and(|t| t.m_len().get() as i32 >= off + len));
    Some((m, at, off))
}

/// `m_dup1`: a copy of `len` bytes of `m` from `off` in one mbuf (or cluster); can't call it
/// `m_dup()`, as freebsd\[34\] uses `m_dup()` with different arg.
fn m_dup1(m: &Mbuf, off: i32, len: i32, wait: i32) -> Option<MbufBox> {
    if len > MAXMCLBYTES as i32 {
        return None;
    }
    let (mut n, l) = if off == 0 && m.pkthdr().is_some() {
        let mut n = m_gethdr(wait, i32::from(m.m_type().get()))?;
        m_dup_pkthdr(&mut n, m, wait).ok()?;
        (n, MHLEN as i32)
    } else {
        (m_get(wait, i32::from(m.m_type().get()))?, MLEN as i32)
    };
    if len > l && !mclgetl(&mut n, wait, len as u32) {
        return None;
    }

    n.m_len().set(len as u32);
    match n.data_mut() {
        Some(dst) => m_copydata(m, off, dst),
        None => kpanic(format_args!("m_dup1: a fresh mbuf read-only")),
    }

    Some(n)
}

/// `m_tag_get`: get a packet tag structure along with specified data following.
pub fn m_tag_get(type_: u16, len: i32, wait: i32) -> Option<MTagBox> {
    if len < 0 {
        return None;
    }
    if len > PACKET_TAG_MAXSIZE as i32 {
        kpanic(format_args!(
            "requested tag size for pool {type_:#x} is too big"
        ));
    }
    MTAGPL.get(
        if wait == M_WAITOK {
            PR_WAITOK
        } else {
            PR_NOWAIT
        },
        || MTag::new(type_, len as u16),
    )
}

/// `m_tag_prepend`: prepend a packet tag.
pub fn m_tag_prepend(m: &mut Mbuf, t: MTagBox) {
    let Some(h) = m.pkthdr_mut() else {
        kpanic(format_args!("m_tag_prepend: an mbuf without M_PKTHDR"));
    };
    let id = t.m_tag_id.get();
    let mut t = t;
    if let Some(first) = h.ph_tags.take() {
        drop(t.link.set(first));
    }
    drop(h.ph_tags.set(t));
    h.ph_tagsset.set(h.ph_tagsset.get() | id);
}

/// `m_tag_delete`: unlink and free the packet tag at `t` (the C's `SLIST_REMOVE`, which
/// assumes it is on the list: `panic(9)` when it is not).
pub fn m_tag_delete(m: &mut Mbuf, t: *const MTag) {
    let Some(h) = m.pkthdr_mut() else {
        kpanic(format_args!("m_tag_delete: an mbuf without M_PKTHDR"));
    };
    let mut link: &mut SlistLink<MTag> = &mut h.ph_tags;
    loop {
        match link.get() {
            None => kpanic(format_args!("m_tag_delete: a tag not on the list")),
            Some(x) if ptr::eq(x, t) => break,
            Some(_) => {}
        }
        let Some(x) = link.get_mut() else {
            kpanic(format_args!("m_tag_delete: a tag not on the list"));
        };
        link = &mut x.link;
    }
    if let Some(mut removed) = link.take() {
        if let Some(after) = removed.link.take() {
            drop(link.set(after));
        }
        drop(removed);
    }

    let mut ph_tagsset = 0u16;
    let mut p = h.ph_tags.get();
    while let Some(pp) = p {
        ph_tagsset |= pp.m_tag_id.get();
        p = pp.link.get();
    }
    h.ph_tagsset.set(ph_tagsset);
}

/// `m_tag_delete_chain`: unlink and free a packet tag chain.
pub fn m_tag_delete_chain(m: &mut Mbuf) {
    if let Some(h) = m.pkthdr_mut() {
        h.delete_tags();
    }
}

/// `m_tag_find`: find a tag, starting from a given position.
pub fn m_tag_find<'a>(m: &'a Mbuf, type_: u16, t: Option<&'a MTag>) -> Option<&'a MTag> {
    let h = m.m_pkthdr();
    if h.ph_tagsset.get() & type_ == 0 {
        return None;
    }

    let mut p = match t {
        None => h.ph_tags.get(),
        Some(t) => t.link.get(),
    };
    while let Some(pp) = p {
        if pp.m_tag_id.get() == type_ {
            return Some(pp);
        }
        p = pp.link.get();
    }
    None
}

/// `m_tag_copy`: copy a single tag.
pub fn m_tag_copy(t: &MTag, wait: i32) -> Option<MTagBox> {
    let mut p = m_tag_get(t.m_tag_id.get(), i32::from(t.m_tag_len.get()), wait)?;
    // Copy the data.
    let n = usize::from(t.m_tag_len.get());
    p.data_mut()[..n].copy_from_slice(&t.bytes()[..n]);
    Some(p)
}

/// `m_tag_copy_chain`: copy two tag chains. The destination mbuf (`to`) loses any attached
/// tags even if the operation fails. This should not be a problem, as `m_tag_copy_chain()` is
/// typically called with a newly-allocated destination mbuf.
pub fn m_tag_copy_chain(to: &mut Mbuf, from: &Mbuf, wait: i32) -> Result<(), Errno> {
    m_tag_delete_chain(to);
    let Some(fh) = from.pkthdr() else {
        return Ok(());
    };
    let Some(th) = to.pkthdr_mut() else {
        kpanic(format_args!("m_tag_copy_chain: to without M_PKTHDR"));
    };
    let mut failed = false;
    {
        // tprev: the link the next copy goes into, the chain's last.
        let mut tail: &mut SlistLink<MTag> = &mut th.ph_tags;
        let mut p = fh.ph_tags.get();
        while let Some(pp) = p {
            let Some(t) = m_tag_copy(pp, wait) else {
                failed = true;
                break;
            };
            th.ph_tagsset.set(th.ph_tagsset.get() | t.m_tag_id.get());
            drop(tail.set(t));
            let Some(last) = tail.get_mut() else {
                break;
            };
            tail = &mut last.link;
            p = pp.link.get();
        }
    }
    if failed {
        th.delete_tags();
        return Err(Errno::ENOBUFS);
    }
    Ok(())
}

/// `m_tag_first`: get first tag in chain.
pub fn m_tag_first(m: &Mbuf) -> Option<&MTag> {
    m.m_pkthdr().ph_tags.get()
}

/// `m_tag_next`: get next tag in chain.
pub fn m_tag_next<'a>(_m: &'a Mbuf, t: &'a MTag) -> Option<&'a MTag> {
    t.link.get()
}
/* </CODE> */
