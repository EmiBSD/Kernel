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
 * Copyright (c) 2004 Artur Grabowski <art@openbsd.org>
 * Copyright (c) 2004 Aaron Campbell <aaron@openbsd.org>
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES,
 * INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY
 * AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
 * THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL  DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS;
 * OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
 * OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF
 * ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/evcount.h>`: event counters, the interrupt counts `vmstat -i` shows.
//!
//! Upstream: sys/sys/evcount.h @ 3ce1f3f79392
//!
//! Status: `ported`. `struct evcount` and its functions are `kernel::dev::evcount`'s since N7.1b,
//! re-exported here for the drivers still in `bsd` (`docs/SCAFFOLDING.md`).

pub use kernel::dev::evcount::Evcount;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use kernel::dev::evcount::{evcount_attach, evcount_detach, evcount_inc};

    use super::*;
    use crate::net::if_::tests::zeroed_static;

    #[test]
    fn a_zeroed_counter_attaches_counts_and_detaches() {
        kernel::testutil::install();
        // SAFETY: `Evcount` is valid all zero (its members are atomics, the per-CPU slot
        // number 0 for none among them, hal's `StrSlot` and `PinEntry`, each 0 when empty), as
        // the zeroed interrupt records that embed one rely on.
        let ec: &'static Evcount = unsafe { zeroed_static() };
        assert_eq!(ec.count(), 0);
        assert!(!ec.is_attached());
        evcount_attach(ec, "zeroed0", Some(7));
        evcount_inc(ec);
        ec.ec_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(ec.count(), 2);
        assert_eq!(ec.vector(), Some(7));
        assert_eq!(ec.name(), "zeroed0");
        evcount_detach(ec);
        assert!(!ec.is_attached());
    }
}
/* </TESTS> */
