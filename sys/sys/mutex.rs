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
//! `<sys/mutex.h>`: the mutex. A mutex is owned by a cpu, non-recursive, spinning, and not
//! providing mutual exclusion between processes, only cpus.
//!
//! Upstream: sys/sys/mutex.h @ 3ce1f3f79392
//! LZ: sys/sys/mutex.rs@3c62ede68802
//!
//! `bsd`'s names over `hal::mutex` (N4.5c): `struct mutex` is `hal::mutex::RawMutex`,
//! `struct db_mutex` `hal::mutex::DbMutex`; the functions (`mtx_enter`, ...) stay in
//! `kern/kern_lock.rs`. `WITNESS` (`lock_object`, `MTX_LO_INITIALIZER`) is not configured.
//!
//! ## Deviations
//! - The fields are atomics inside `hal`'s types, so a `static` mutex is entered through `&`.
//! - `db_mutex`'s owner is the owning CPU's word (`mtx_curcpu()`), like `struct mutex`'s,
//!   instead of a `struct cpu_info *volatile`.
//!
//! ## Redesign
//! - N4.5c: `struct mutex` and `struct db_mutex` are `hal`'s (`RawMutex`, `DbMutex`; LZ's
//!   two `unsafe impl Sync` are gone with their `Cell`s): this module keeps the C's names
//!   (`Mutex`, `MUTEX_INITIALIZER` as `Mutex::new`, `mtx_owner`, `mtx_owned`,
//!   `MUTEX_ASSERT_LOCKED`/`UNLOCKED`, `__MUTEX_IPL`, `mtx_curcpu`) for the 128 files that
//!   use them. The uniprocessor `__MUTEX_IPL` is gone (decision 40).

pub use hal::mutex::{DbMutex, RawMutex as Mutex, mtx_curcpu, mutex_ipl};

/// `MTX_NOWITNESS`.
pub const MTX_NOWITNESS: i32 = 0x01;
/// `MTX_DUPOK`.
pub const MTX_DUPOK: i32 = 0x02;

/// `mtx_owner(mtx)`: the owning CPU, without the waiter bit.
pub fn mtx_owner(mtx: &Mutex) -> usize {
    mtx.owner()
}

/// `MUTEX_ASSERT_LOCKED(mtx)` (`DIAGNOSTIC`).
pub fn mutex_assert_locked(mtx: &Mutex, func: &str) {
    #[cfg(feature = "diagnostic")]
    if !mtx.owned() {
        crate::kern::subr_prf::panic(format_args!("mutex {:p} not held in {func}", mtx));
    }
    #[cfg(not(feature = "diagnostic"))]
    let _ = (mtx, func);
}

/// `MUTEX_ASSERT_UNLOCKED(mtx)` (`DIAGNOSTIC`).
pub fn mutex_assert_unlocked(mtx: &Mutex, func: &str) {
    #[cfg(feature = "diagnostic")]
    if mtx.owner() == mtx_curcpu() && !hal::cpu::past_caring() {
        crate::kern::subr_prf::panic(format_args!("mutex {:p} held in {func}", mtx));
    }
    #[cfg(not(feature = "diagnostic"))]
    let _ = (mtx, func);
}

/// `mtx_owned(mtx)`: whether this CPU holds `mtx` (or the kernel is past caring).
pub fn mtx_owned(mtx: &Mutex) -> bool {
    mtx.owned()
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::intr::IPL_NONE;

    #[test]
    fn initializer_is_free() {
        let m = Mutex::new(7);
        assert_eq!(mtx_owner(&m), 0);
        // `__MUTEX_IPL`: a level below `IPL_MPFLOOR` is raised to it.
        assert!(7 < crate::machine::intr::IPL_MPFLOOR);
        assert_eq!(m.wantipl(), crate::machine::intr::IPL_MPFLOOR);
        assert_eq!(m.oldipl(), IPL_NONE);
        assert!(!mtx_owned(&m));
        assert_eq!(mutex_ipl(7), crate::machine::intr::IPL_MPFLOOR);
    }
}
/* </TESTS> */
