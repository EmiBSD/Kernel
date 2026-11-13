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
 * Copyright (C) 1995, 1996, 1997, and 1998 WIDE Project.
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
 * Copyright (c) 1982, 1986, 1993
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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! The Internet protocol switch as the input path reaches it: `inetsw[]` (the table of the
//! internet domain's protocols), `ip_protox[]` (an IP protocol number to its entry, every
//! number without one going to the raw entry) and the raw entry itself.
//!
//! Upstream: sys/netinet/in_proto.c @ 3ce1f3f79392
//! LZ: sys/netinet/in_proto.rs@3c62ede68802
//!
//! ## Deviations
//! - The table is `bsd`'s `inetsw[]` (`sys/netinet/in_proto.rs`), its entries the kernel's
//!   protocols through the `Protocol` bridge impl, until the table itself moves (N6.4c2's
//!   next step): `ip_init` records it here ([`inetsw_init`]).
//! - `INET6` is configured and IPv6 is `bsd`'s until N6.5: `ip_deliver`'s IPv6 steps reach
//!   `inet6sw[ip6_protox[nxt]]` through the `Inet` bridge's `ip6_protox` and the IPv6
//!   domain's table ([`inet6_protocol`]).
//!
//! ## Redesign
//! - LZ's `ip_protox[]` (a static array of atomics in `bsd`'s `in_proto.rs`) and the table's
//!   address are [`InetTables`], one per glue in the network's tables (`NetTables::inet`,
//!   D-9: the protocols are generic over the glue, `inetsw[]` cannot be a `static` of the
//!   kernel): the table, the index of its raw entry and `ip_protox[]`, filled once by
//!   [`inetsw_init`] as the C's `ip_init` fills `ip_protox[]`. A lookup is two loads and an
//!   index ([`ip_protocol`], `#[inline]`: per packet), the C's `inetsw[ip_protox[nxt]]`.

use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use hal::once::OnceStatic;
use hal::sys::netinet_in::{IPPROTO_MAX, IPPROTO_RAW};
use hal::sys::socket::{AF_INET6, PF_INET, SOCK_RAW};

use crate::assert::kpanic;
use crate::net::domain::{net, pffinddomain};
use crate::net::if_::inet;
use crate::net::proto::Protocol;
use crate::proc::ProcGlue;

/// The internet domain's switch of one glue: `inetsw[]`, its raw entry and `ip_protox[]`.
pub struct InetTables<G: ProcGlue> {
    /// `inetsw[]`, set once by [`inetsw_init`].
    sw: OnceStatic<&'static [&'static dyn Protocol<G>]>,
    /// The raw entry's index (`pffindproto(PF_INET, IPPROTO_RAW, SOCK_RAW) - inetsw`).
    raw: AtomicUsize,
    /// `ip_protox[]`: the entry of each IP protocol number.
    protox: [AtomicU8; IPPROTO_MAX as usize],
    /// The IPv6 domain's protocols (`inet6sw[]`, `bsd`'s until N6.5), found at their first
    /// use by `ip_deliver`.
    inet6: OnceStatic<&'static [&'static dyn Protocol<G>]>,
}

impl<G: ProcGlue> InetTables<G> {
    /// The tables before `ip_init`.
    pub const fn new() -> Self {
        Self {
            sw: OnceStatic::new(),
            raw: AtomicUsize::new(0),
            protox: [const { AtomicU8::new(0) }; IPPROTO_MAX as usize],
            inet6: OnceStatic::new(),
        }
    }
}

impl<G: ProcGlue> Default for InetTables<G> {
    fn default() -> Self {
        Self::new()
    }
}

/// The glue's internet switch.
#[inline]
fn tables<G: ProcGlue>() -> &'static InetTables<G> {
    &net::<G>().inet
}

/// `inetsw[]`: the internet protocols (none before `ip_init`).
pub fn inetsw<G: ProcGlue>() -> &'static [&'static dyn Protocol<G>] {
    tables::<G>().sw.get().copied().unwrap_or(&[])
}

/// `ip_init`'s switch: `sw` recorded (a second call, a test's, keeps the first table) and
/// `ip_protox[]` filled from it: every number to the raw entry, then each protocol of the
/// table (but the raw one) to its own. `panic(9)` without a raw entry (the C's
/// `panic("ip_init")`).
pub fn inetsw_init<G: ProcGlue>(sw: &'static [&'static dyn Protocol<G>]) {
    let t = tables::<G>();
    let _ = t.sw.set(sw);
    let sw = inetsw::<G>();

    // pffindproto(PF_INET, IPPROTO_RAW, SOCK_RAW): the raw protocol, or the raw wildcard.
    let exact = sw
        .iter()
        .position(|pr| i32::from(pr.protocol()) == IPPROTO_RAW && pr.ty() == SOCK_RAW as i16);
    let wildcard = sw
        .iter()
        .position(|pr| pr.ty() == SOCK_RAW as i16 && pr.protocol() == 0);
    let Some(raw) = exact.or(wildcard) else {
        kpanic(format_args!("ip_init"));
    };
    t.raw.store(raw, Ordering::Relaxed);
    for p in &t.protox {
        p.store(raw as u8, Ordering::Relaxed);
    }
    for (i, pr) in sw.iter().enumerate() {
        let proto = i32::from(pr.protocol());
        if pr.domain().family() == i32::from(PF_INET)
            && proto != 0
            && proto != IPPROTO_RAW
            && proto < IPPROTO_MAX
        {
            t.protox[proto as usize].store(i as u8, Ordering::Relaxed);
        }
    }
}

/// `inetsw[ip_protox[nxt]]`: the protocol of IP protocol number `nxt` (`None` before
/// `ip_init`, or for a number past `IPPROTO_MAX`, which no header holds).
#[inline]
pub fn ip_protocol<G: ProcGlue>(nxt: i32) -> Option<&'static dyn Protocol<G>> {
    let t = tables::<G>();
    let i = t
        .protox
        .get(usize::try_from(nxt).ok()?)?
        .load(Ordering::Relaxed);
    t.sw.get()?.get(usize::from(i)).copied()
}

/// The raw entry (`rip_input`'s, which ICMP and IGMP hand their messages to).
#[inline]
pub fn raw_protocol<G: ProcGlue>() -> Option<&'static dyn Protocol<G>> {
    let t = tables::<G>();
    t.sw.get()?.get(t.raw.load(Ordering::Relaxed)).copied()
}

/// `inet6sw[ip6_protox[nxt]]`: the IPv6 protocol of number `nxt`, through the `Inet` bridge
/// (IPv6 is `bsd`'s until N6.5); `None` without IPv6.
pub fn inet6_protocol<G: ProcGlue>(nxt: i32) -> Option<&'static dyn Protocol<G>> {
    let i = inet()?.ip6_protox(nxt)?;
    let t = tables::<G>();
    let sw = match t.inet6.get() {
        Some(sw) => *sw,
        None => {
            let sw = pffinddomain::<G>(i32::from(AF_INET6))?.protocols();
            let _ = t.inet6.set(sw);
            sw
        }
    };
    sw.get(i).copied()
}
/* </CODE> */
