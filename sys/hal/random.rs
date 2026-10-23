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
 * Copyright (c) 2011,2020 Theo de Raadt.
 * Copyright (c) 2008 Damien Miller.
 * Copyright (c) 1996, 1997, 2000-2002 Michael Shalayeff.
 * Copyright (c) 2013 Markus Friedl.
 * Copyright Theodore Ts'o, 1994, 1995, 1996, 1997, 1998, 1999.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, and the entire permission notice in its entirety,
 *    including the disclaimer of warranties.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. The name of the author may not be used to endorse or promote
 *    products derived from this software without specific prior
 *    written permission.
 *
 * ALTERNATIVELY, this product may be distributed under the terms of
 * the GNU Public License, in which case the provisions of the GPL are
 * required INSTEAD OF the above restrictions.  (This clause is
 * necessary due to a potential bad interaction between the GPL and
 * the restrictions contained in a BSD-style copyright.)
 *
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND ANY EXPRESS OR IMPLIED
 * WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 * (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT,
 * STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED
 * OF THE POSSIBILITY OF SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `arc4random(9)` and `arc4random_uniform(9)` for the crates above `bsd`: the kernel's map
//! draws its address randomisation from them (`uvm_map_hint`, `uvm_map_pie`, the address
//! selectors of `uvm_addr.c`, N3.5).
//!
//! Upstream: sys/dev/rnd.c @ 3ce1f3f79392
//! LZ: sys/dev/rnd.rs@3c62ede68802
//!
//! The generator (`rnd.c`'s ChaCha20 state, its entropy pool and its stirring) stays in `bsd`
//! (`sys/dev/rnd.rs`, N7), so a word arrives through the [`Random`] bridge that `bsd`
//! registers before `main()` on every machine (`hal_bridges_register`; bridges.toml, paid by
//! N7). [`arc4random_uniform`] is `rnd.c`'s own function, written here over that word.
//!
//! **Before registration** there is no generator to draw from: [`arc4random`] is `panic(9)`
//! (`"arc4random: before rnd is registered"`), never a fixed value that would turn the
//! randomisation off unseen. Nothing above `bsd` draws before `main()`.
//!
//! ## Redesign
//! - `rnd.c`'s `arc4random` is reached through the existing `Random` bridge (arm64's ASID
//!   allocator's): its reader was `hal`'s alone; this module is its public face.
//! - `arc4random_uniform` is re-expressed over [`arc4random`] with the C's rejection of the
//!   low `2**32 % upper_bound` values, so the result is unbiased; `bsd`'s copy in
//!   `sys/dev/rnd.rs` stays for its legacy callers.
//! - N4.7b: [`enqueue_randomness`], the bridge's second member, for the terminal's
//!   per-character call (`ttyinput`).

pub use crate::pmap::Random;

use crate::assert::hal_panic;
use crate::bridges;

/// `arc4random(9)`: one word from `bsd`'s ChaCha20 generator; `panic(9)` before the bridge
/// is registered.
pub fn arc4random() -> u32 {
    match bridges::random() {
        Some(r) => r.random_u32(),
        None => hal_panic(format_args!("arc4random: before rnd is registered")),
    }
}

/// `arc4random_uniform(9)`: a uniformly distributed random number less than `upper_bound`
/// (0 when `upper_bound < 2`), without "modulo bias": words below `2**32 % upper_bound` are
/// drawn again, so the ones kept map evenly onto `[0, upper_bound)` (each draw is kept with
/// probability above one half).
pub fn arc4random_uniform(upper_bound: u32) -> u32 {
    uniform_with(upper_bound, arc4random)
}

/// `enqueue_randomness(val)`: `val` mixed into `bsd`'s entropy pool (the terminal's input
/// characters, `tty.c`'s `ttyinput`); nothing before the bridge is registered (no generator
/// to feed yet).
pub fn enqueue_randomness(val: u32) {
    if let Some(r) = bridges::random() {
        r.enqueue_randomness(val);
    }
}

/// [`arc4random_uniform`] over the word source `draw`.
fn uniform_with(upper_bound: u32, mut draw: impl FnMut() -> u32) -> u32 {
    if upper_bound < 2 {
        return 0;
    }
    // 2**32 % x == (2**32 - x) % x
    let min = upper_bound.wrapping_neg() % upper_bound;
    loop {
        let r = draw();
        if r >= min {
            return r % upper_bound;
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn the_threshold_is_two_to_the_32_mod_the_bound() {
        // (bound, 2**32 % bound): the first word kept.
        for (bound, min) in [
            (2u32, 0u32),
            (3, 1),
            (7, 4),
            (1000, 296),
            (0x8000_0001, 0x7fff_ffff),
            (u32::MAX, 1),
        ] {
            assert_eq!(bound.wrapping_neg() % bound, min);
            let mut words = [min.wrapping_sub(1), min].into_iter();
            if min == 0 {
                // Nothing is rejected: the first word is kept.
                assert_eq!(uniform_with(bound, || 5), 5 % bound);
                continue;
            }
            assert_eq!(
                uniform_with(bound, || words.next().unwrap_or(0)),
                min % bound
            );
        }
    }

    #[test]
    fn a_bound_below_two_draws_nothing() {
        assert_eq!(uniform_with(0, || panic!("drew")), 0);
        assert_eq!(uniform_with(1, || panic!("drew")), 0);
    }

    #[test]
    fn the_words_come_from_the_bridge_and_stay_below_the_bound() {
        testutil::install();
        testutil::set_random_seed(0x1234_5678);
        let a = arc4random();
        testutil::set_random_seed(0x1234_5678);
        assert_eq!(arc4random(), a);
        for bound in [2u32, 3, 7, 1000, 0x8000_0001, u32::MAX] {
            for _ in 0..200 {
                assert!(arc4random_uniform(bound) < bound);
            }
        }
    }
}
/* </TESTS> */
