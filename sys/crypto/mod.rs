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
//! `sys/crypto`: the kernel's cryptographic primitives and the crypto framework (`crypto(9)`).

#[forbid(unsafe_code)]
pub mod aes;
#[forbid(unsafe_code)]
pub mod blake2s;
pub use kernel::crypto::blf;
pub use kernel::crypto::cast;
pub use kernel::crypto::chacha_private;
pub use kernel::crypto::chachapoly;
pub mod criov;
#[allow(clippy::module_inception)] // crypto.c, the file, in the crypto directory
pub mod crypto;
#[forbid(unsafe_code)]
pub mod cryptodev;
pub mod cryptosoft;
#[forbid(unsafe_code)]
pub mod curve25519;
pub use kernel::crypto::des_locl;
pub use kernel::crypto::ecb_enc;
pub use kernel::crypto::ecb3_enc;
#[forbid(unsafe_code)]
pub mod gmac;
pub use kernel::crypto::hmac;
#[forbid(unsafe_code)]
pub mod idgen;
pub use kernel::crypto::md5;
pub use kernel::crypto::podd;
pub use kernel::crypto::poly1305;
#[forbid(unsafe_code)]
pub mod rijndael;
pub use kernel::crypto::rmd160;
pub use kernel::crypto::set_key;
pub use kernel::crypto::sha1;
pub use kernel::crypto::sha2;
pub use kernel::crypto::sk;
pub use kernel::crypto::spr;
#[cfg(test)]
#[forbid(unsafe_code)]
pub(crate) mod testutil;
#[forbid(unsafe_code)]
pub mod xform;
pub use kernel::crypto::xform_ipcomp;

pub use kernel::crypto::wipe;
/* </CODE> */
