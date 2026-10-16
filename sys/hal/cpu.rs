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
//! The running CPU, as the runtime needs it: its index (for [`crate::percpu::PerCpu`]), a
//! word that names it (the owner of a [`crate::mutex::Mutexed`]), its scheduler's spin
//! counter, and whether the kernel is past caring about locks.
//!
//! The machine answers through the [`CpuId`] bridge, registered by `bsd` at boot over
//! `curcpu()`, `cpu_number()`, `ci_schedstate.spc_spinning`, `panicstr` and `db_active`
//! (bridges.toml; paid by N4, when the hardware is `hal`'s own).
//!
//! **Before registration** (the boot CPU before `main()` registers the bridges, the host tests
//! that register none): one CPU runs, index 0, named by a fixed word; nobody counts its spins;
//! the kernel is not past caring. No lock's mutual exclusion rests on these answers: the
//! locks compare-and-swap a word that is never 0 whatever the bridge says (`mutex.rs`,
//! `rwlock.rs`), so a wrong answer costs a diagnostic, never an aliased `&mut`.
//!
//! ## Redesign
//! - Not a port: the C reads `curcpu()` and its fields directly; the runtime reaches them
//!   through one trait object until the hardware moves in (docs/ZERO_UNSAFE.md section 0.1).

use crate::bridges;

/// `MAXCPUS`: the most CPUs a kernel supports, at least the machine's (255 on amd64, 256 on
/// arm64, 1 without `MULTIPROCESSOR`).
#[cfg(feature = "multiprocessor")]
pub const MAXCPUS: usize = 256;
/// `MAXCPUS`: one CPU without `MULTIPROCESSOR`.
#[cfg(not(feature = "multiprocessor"))]
pub const MAXCPUS: usize = 1;

/// The word that names the boot CPU before the bridge is registered.
const BOOT_CPU_WORD: usize = 0x10;

/// The running CPU, as `bsd` registers it.
pub trait CpuId: Sync {
    /// `cpu_number()`: the running CPU's index, below [`MAXCPUS`].
    fn cpu_number(&self) -> usize;
    /// `curcpu()` as a word: distinct for every CPU (the C's `struct cpu_info *`).
    fn curcpu_word(&self) -> usize;
    /// `curcpu()->ci_schedstate.spc_spinning += delta`: the CPU is spinning for a lock.
    fn spinning(&self, delta: isize);
    /// `panicstr || db_active`: after a panic or inside ddb the locks stop excluding, so a
    /// CPU that stopped holding one cannot deadlock the debugger or the dump.
    fn past_caring(&self) -> bool;
    /// `curcpu()->ci_mutex_level += delta` (`DIAGNOSTIC`).
    fn mutex_level(&self, delta: i32);
    /// `ncpusfound`: how many CPUs the machine has (the per-CPU readers walk that many).
    fn ncpus(&self) -> usize;
}

/// `cpu_number()` (0 before the bridge is registered). An index at or past [`MAXCPUS`] is a
/// broken machine answer: `panic(9)` naming it, never a silent fold onto another CPU's slot
/// (which would make two CPUs write one per-CPU copy).
pub fn cpu_number() -> usize {
    let Some(b) = bridges::cpu_id() else {
        return 0;
    };
    let n = b.cpu_number();
    if n >= MAXCPUS {
        crate::assert::hal_panic(format_args!(
            "CpuId: cpu_number {n} is not below MAXCPUS {MAXCPUS}"
        ));
    }
    n
}

/// `curcpu()` as a word (a fixed word before the bridge is registered).
pub(crate) fn curcpu_word() -> usize {
    bridges::cpu_id().map_or(BOOT_CPU_WORD, |b| b.curcpu_word())
}

/// `spc_spinning += delta` (nothing before the bridge is registered).
pub(crate) fn spinning(delta: isize) {
    if let Some(b) = bridges::cpu_id() {
        b.spinning(delta);
    }
}

/// `panicstr || db_active` (`false` before the bridge is registered).
pub(crate) fn past_caring() -> bool {
    bridges::cpu_id().is_some_and(|b| b.past_caring())
}

/// `ci_mutex_level += delta` under `DIAGNOSTIC` (nothing before the bridge is registered).
#[cfg(feature = "diagnostic")]
pub(crate) fn mutex_level(delta: i32) {
    if let Some(b) = bridges::cpu_id() {
        b.mutex_level(delta);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn the_index_is_below_maxcpus_and_the_word_is_never_zero() {
        testutil::install();
        assert!(cpu_number() < MAXCPUS);
        assert_ne!(curcpu_word(), 0);
        assert!(!past_caring());
    }
}
/* </TESTS> */
