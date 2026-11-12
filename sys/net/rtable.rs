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
 * Copyright (c) 2014-2016 Martin Pieuchot
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
//! The routing tables, `net/rtable.h` and `net/rtable.c`, for `bsd`'s callers: the tables are
//! the kernel's (`kernel::net::rtable`, N6.3b); this module is their adapter, of no logic (the
//! kernel's items under LZ's names; a route answered as the `&'static Rtentry` LZ's code
//! keeps, its reference parked as `sys/net/route.rs` describes; the net lock's proof made
//! through the `SaToken` where the kernel asks for one).
//!
//! Upstream: sys/net/rtable.h @ 3ce1f3f79392
//! Upstream: sys/net/rtable.c @ 3ce1f3f79392

pub use kernel::net::rtable::{
    SaCopy, rtable_add, rtable_empty, rtable_exists, rtable_l2, rtable_l2set, rtable_loindex,
};

use crate::kern::proc_glue::BsdProcGlue;
use crate::net::if_::net_held;
use crate::net::route::{Rtentry, as_static, park};
use crate::sys::errno::Errno;
use crate::sys::types::SaFamily;

/// `rt_key(rt)`: the destination's bytes.
pub fn rt_key(rt: &Rtentry) -> &[u8] {
    rt.dest()
}

/// `rt_plen(rt)`.
pub fn rt_plen(rt: &Rtentry) -> u32 {
    rt.rt_plen()
}

/// `rtable_init()`: the routing tables of the domains `domaininit` will record, before lo0
/// attaches (`init_main`); first `bsd`'s ART pools, which wg(4)'s allowed IPs use until N8
/// (D-N6.3b-25: the C's `rtable_init` calls `art_boot`, the kernel's calls its own).
pub fn rtable_init() {
    crate::net::art::art_boot();
    kernel::net::rtable::rtable_init::<BsdProcGlue>(crate::kern::uipc_domain::NET_DOMAINS);
}

/// `rtable_getsource(rtableid, af)`: a copy of the table's preferred source address of `af`.
pub fn rtable_getsource(rtableid: u32, af: SaFamily) -> Option<SaCopy> {
    kernel::net::rtable::rtable_getsource(rtableid, af, &net_held("rtable_getsource"))
}

/// `rtable_iterate(rt0)`: the route after `rt0` on its multipath chain, `rt0`'s parked
/// reference given back and the answer's parked.
pub fn rtable_iterate(rt0: &'static Rtentry) -> Option<&'static Rtentry> {
    let Some(rt0) = hal::pool::PoolArc::adopt(rt0) else {
        crate::kern::subr_prf::panic(format_args!("rtable_iterate: route not referenced"));
    };
    kernel::net::rtable::rtable_iterate(rt0).map(park)
}

/// `rtable_walk(rtableid, af, prt, func, arg)`: `func` of every route of table `rtableid` of
/// `af`, each lent for its call; the route a callback answered `EEXIST` for given to `prt`
/// with its reference parked.
pub fn rtable_walk(
    rtableid: u32,
    af: SaFamily,
    prt: Option<&mut Option<&'static Rtentry>>,
    mut func: impl FnMut(&'static Rtentry, u32) -> Result<(), Errno>,
) -> Result<(), Errno> {
    let mut f = |rt: &Rtentry, id: u32| func(as_static(rt), id);
    match prt {
        Some(p) => {
            let mut r = None;
            let e = kernel::net::rtable::rtable_walk(rtableid, af, Some(&mut r), &mut f);
            *p = r.map(park);
            e
        }
        None => kernel::net::rtable::rtable_walk(rtableid, af, None, &mut f),
    }
}
/* </CODE> */
