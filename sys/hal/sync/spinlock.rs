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
 * Copyright (c) 2014 Patrick Wildt <patrick@blueri.se>
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
//! `<machine/spinlock.h>`: the word userland's spinlocks are made of (`_atomic_lock_t`),
//! which `__thrsleep(2)` frees for its caller by copying [`_ATOMIC_LOCK_UNLOCKED`] out over
//! it (`kernel::sched::thrsleep`).
//!
//! Upstream: sys/arch/amd64/include/spinlock.h @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/spinlock.h @ 3ce1f3f79392
//! LZ: sys/arch/amd64/include/spinlock.rs@9fd12af2c575
//! LZ: sys/arch/arm64/include/spinlock.rs@9fd12af2c575
//!
//! The two machines' headers say the same three things: an `int`, 0 when free, 1 when held
//! (userland's `_atomic_lock` swaps 1 in and tests the old value). So the values are one
//! definition for both, as `hal::cpu::intr`'s `IPL_CLOCK` is, and nothing here depends on the
//! machine.
//!
//! ## Deviations
//! - The typedef `_atomic_lock_t` is the type alias [`AtomicLock`] (LZ's).
//!
//! ## Redesign
//! - LZ ported each machine's header as a file of its arch (`sys/arch/<arch>/include/
//!   spinlock.rs`) and named it for generic code through a new trait of its machine contract,
//!   `MachineSpinlock` (`type AtomicLock`, `ATOMIC_LOCK_UNLOCKED`, `ATOMIC_LOCK_LOCKED`,
//!   implemented on `Machine` by both files and by the host double). The kernel crate cannot
//!   name `bsd`'s `Machine`, and the two headers are identical, so the three items are one
//!   `hal` module with no `cfg`; the trait, its `sys/machine` hunks and the host impl are
//!   not taken. The `#[ignore]` test reads both C headers, so amd64's half is checked
//!   without building amd64 (decision 44).

/// `_atomic_lock_t`: an `int` on both machines.
pub type AtomicLock = i32;

/// `_ATOMIC_LOCK_UNLOCKED`: the value of a free lock.
pub const _ATOMIC_LOCK_UNLOCKED: AtomicLock = 0;

/// `_ATOMIC_LOCK_LOCKED`: the value of a held lock.
pub const _ATOMIC_LOCK_LOCKED: AtomicLock = 1;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    /// The word is an `int`, free at 0 and held at 1 (`_atomic_lock`'s convention).
    #[test]
    fn the_lock_word_is_an_int_free_at_zero() {
        assert_eq!(core::mem::size_of::<AtomicLock>(), 4);
        assert_eq!(_ATOMIC_LOCK_UNLOCKED, 0);
        assert_eq!(_ATOMIC_LOCK_LOCKED, 1);
    }

    /// Both machines' headers define the values these are, and the type as an `int`.
    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_both_c_headers() {
        for header in [
            "sys/arch/amd64/include/spinlock.h",
            "sys/arch/arm64/include/spinlock.h",
        ] {
            let defs = crate::reftest::defines(header);
            assert_eq!(
                crate::reftest::int(&defs, "_ATOMIC_LOCK_UNLOCKED"),
                Some(i64::from(_ATOMIC_LOCK_UNLOCKED)),
                "{header}"
            );
            assert_eq!(
                crate::reftest::int(&defs, "_ATOMIC_LOCK_LOCKED"),
                Some(i64::from(_ATOMIC_LOCK_LOCKED)),
                "{header}"
            );
            let text = std::fs::read_to_string(crate::reftest::openbsd_src().join(header))
                .unwrap_or_else(|e| panic!("{header}: {e}"));
            assert!(text.contains("typedef int _atomic_lock_t;"), "{header}");
        }
    }
}
/* </TESTS> */
