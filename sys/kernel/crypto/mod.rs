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
/* </LICENSES> */

/* <CODE> */
//! The crypto primitives (ciphers, hashes, MACs and their constant tables), moved from
//! `sys/crypto` (docs/ZERO_UNSAFE.md 0.8, step 5). `bsd` re-exports every module from here.

pub mod blf;
pub mod cast;
pub mod chacha_private;
pub mod chachapoly;
pub mod des_locl;
pub mod ecb3_enc;
pub mod ecb_enc;
pub mod hmac;
pub mod md5;
pub mod podd;
pub mod poly1305;
pub mod rmd160;
pub mod set_key;
pub mod sha1;
pub mod sha2;
pub mod sk;
pub mod spr;
#[cfg(test)]
#[allow(dead_code, unused_macros, unused_imports)]
pub(crate) mod testutil;
pub mod xform_ipcomp;

/// `explicit_bzero(&x, sizeof(x))` for a context that is a plain value: overwrites it with its
/// default (all zero) and keeps the compiler from proving the store dead, so the secret it held
/// does not outlive the call. The byte buffers use `libkern::explicit_bzero`.
pub fn wipe<T: Default>(x: &mut T) {
    *x = T::default();
    core::hint::black_box(&*x);
}
/* </CODE> */
