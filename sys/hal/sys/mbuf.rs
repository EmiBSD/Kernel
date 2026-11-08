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
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)mbuf.h	8.5 (Berkeley) 2/19/95
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/mbuf.h>`'s part userland reads: the mbuf types (`MT_*`) and `struct mbstat`, which
//! `kern.mbstat` copies out (`netstat -m`).
//!
//! Upstream: sys/sys/mbuf.h @ 3ce1f3f79392
//! LZ: sys/sys/mbuf.rs@3c62ede68802
//!
//! ## Redesign
//! - N6.1: moved from `bsd`'s `sys/sys/mbuf.rs` with the mbuf's redesign
//!   (`kernel::net::mbuf`), so `kernel::sysctl` and `bsd` name one copy (the N4.6b
//!   `sysctl.h` precedent): `Mbstat` is `AbiPod` through [`crate::abi_pod!`] (its bytes are
//!   `kern.mbstat`'s), no `unsafe`.

// mbuf types

/// `MT_FREE`: should be on free list.
pub const MT_FREE: i32 = 0;
/// `MT_DATA`: dynamic (data) allocation.
pub const MT_DATA: i32 = 1;
/// `MT_HEADER`: packet header.
pub const MT_HEADER: i32 = 2;
/// `MT_SONAME`: socket name.
pub const MT_SONAME: i32 = 3;
/// `MT_SOOPTS`: socket options.
pub const MT_SOOPTS: i32 = 4;
/// `MT_FTABLE`: fragment reassembly header.
pub const MT_FTABLE: i32 = 5;
/// `MT_CONTROL`: extra-data protocol message.
pub const MT_CONTROL: i32 = 6;
/// `MT_OOBDATA`: expedited data.
pub const MT_OOBDATA: i32 = 7;
/// `MT_NTYPES`.
pub const MT_NTYPES: usize = 8;

/// `struct mbstat`: mbuf statistics. For statistics related to mbuf and cluster allocations,
/// see also the pool headers (`mbufpl` and the `mcl*` pools). `#[repr(C)]`: `kern.mbstat`
/// copies it out.
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Mbstat {
    /// `m_drops`: times failed to find space.
    pub m_drops: u64,
    /// `m_wait`: times waited for space.
    pub m_wait: u64,
    /// `m_drain`: times drained protocols for space.
    pub m_drain: u64,
    /// `m_mtypes`: type specific mbuf allocations.
    pub m_mtypes: [u64; MT_NTYPES],
    /// `m_defrag_alloc`.
    pub m_defrag_alloc: u64,
    /// `m_prepend_alloc`.
    pub m_prepend_alloc: u64,
    /// `m_pullup_alloc`.
    pub m_pullup_alloc: u64,
    /// `m_pullup_copy`.
    pub m_pullup_copy: u64,
    /// `m_pulldown_alloc`.
    pub m_pulldown_alloc: u64,
    /// `m_pulldown_copy`.
    pub m_pulldown_copy: u64,
}

// The members tile the structure: its bytes are `kern.mbstat`'s.
crate::abi_pod!(Mbstat {
    m_drops: u64,
    m_wait: u64,
    m_drain: u64,
    m_mtypes: [u64; MT_NTYPES],
    m_defrag_alloc: u64,
    m_prepend_alloc: u64,
    m_pullup_alloc: u64,
    m_pullup_copy: u64,
    m_pulldown_alloc: u64,
    m_pulldown_copy: u64,
});

const _: () = {
    // struct mbstat on LP64: 17 u_int64_t.
    assert!(size_of::<Mbstat>() == 17 * 8);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mbstat_is_the_c_s_seventeen_words() {
        let mut s = Mbstat::default();
        s.m_mtypes[MT_DATA as usize] = 5;
        s.m_pulldown_copy = 9;
        let mut bytes = [0u8; size_of::<Mbstat>()];
        assert!(crate::user::pod_to_bytes(&s, &mut bytes));
        assert_eq!(bytes[(3 + 1) * 8], 5);
        assert_eq!(bytes[16 * 8], 9);
    }
}
/* </TESTS> */
