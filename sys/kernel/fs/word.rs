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
//! The words of a file system's in-core structures: the members the C changes through a
//! shared pointer under the kernel lock (an inode's `i_flag`, a mount's `pm_fmod`, a
//! denode's `de_FileSize`), each a relaxed atomic read and written whole by `get()`/`set()`
//! as LZ's `Cell`s were. Every writer holds the kernel lock, as every OpenBSD file system
//! path runs `KERNEL_LOCK`ed; that lock orders the writes, so a change is a load and a store
//! and [`set`](U32Word::set) checks the lock under `diagnostic`. Readers may not hold it: a
//! relaxed load returns a whole word, one a writer stored (docs/IDIOMS.md, the counters' row).
//!
//! N5.7c1 lifted the macro and its five types out of `fs::ufs::ufs::inode` (which re-exports
//! them, so ufs's and ext2fs's names are unchanged) for their third user, msdosfs, which adds
//! [`U8Word`] and [`U16Word`] (the denode's `u_char` and `u_short` members).

use core::sync::atomic::{AtomicI8, AtomicI32, AtomicI64, AtomicU8, AtomicU16, AtomicU32};
use core::sync::atomic::{AtomicU64, Ordering};

/// Generates a word: a relaxed atomic of `$t` read and written whole by `get()`/`set()`.
macro_rules! word {
    ($(#[$doc:meta])* $name:ident, $t:ty, $atomic:ty) => {
        $(#[$doc])*
        #[derive(Debug, Default)]
        pub struct $name($atomic);

        impl $name {
            /// `v`.
            pub const fn new(v: $t) -> Self {
                Self(<$atomic>::new(v))
            }

            /// The value.
            #[inline]
            pub fn get(&self) -> $t {
                self.0.load(Ordering::Relaxed)
            }

            /// Stores `v`: by a holder of the kernel lock (checked under `diagnostic`).
            #[inline]
            pub fn set(&self, v: $t) {
                #[cfg(feature = "diagnostic")]
                hal::klock::kernel_assert_locked();
                self.0.store(v, Ordering::Relaxed);
            }
        }
    };
}

word!(
    /// A `u_int32_t` word (`i_flag`, `i_ino`, `i_reclen`, `um_maxsymlinklen`, the denode's
    /// cluster numbers and size).
    U32Word, u32, AtomicU32
);
word!(
    /// An `int32_t` word (`i_effnlink`, the directory offsets, `um_dev`, `de_fndcnt`).
    I32Word, i32, AtomicI32
);
word!(
    /// A `u_int64_t` word (`i_modrev`, the `um_*` words of the mount, a denode's fat cache
    /// slot).
    U64Word, u64, AtomicU64
);
word!(
    /// An `int8_t` word (EXT2FS's in-core super-block flags `e2fs_ronly`, `e2fs_fmod`).
    I8Word, i8, AtomicI8
);
word!(
    /// An `int64_t` word (EXT2FS's `e2fs_qbmask`, `e2fs_maxfilesize`, the extent cache's
    /// `ec_start`, the denode's `de_refcnt`).
    I64Word, i64, AtomicI64
);
word!(
    /// A `u_char` word (the denode's `de_Attributes`, `de_CTimeHundredth`).
    U8Word, u8, AtomicU8
);
word!(
    /// A `u_short` word (the denode's DOS times and dates).
    U16Word, u16, AtomicU16
);
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_keep_their_whole_value() {
        let w = U16Word::new(0xfffe);
        assert_eq!(w.get(), 0xfffe);
        w.set(0x1234);
        assert_eq!(w.get(), 0x1234);
        let b = U8Word::default();
        assert_eq!(b.get(), 0);
        b.set(0xff);
        assert_eq!(b.get(), 0xff);
        let s = I64Word::new(-1);
        assert_eq!(s.get(), -1);
        let u = U64Word::new(u64::MAX);
        u.set(u.get().wrapping_add(1));
        assert_eq!(u.get(), 0);
    }
}
/* </TESTS> */
