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
//! The trust boundary of the memory bridges: the checks `hal` makes on every block the
//! kernel's allocators hand it, before it trusts the block.
//!
//! The memory bridges (`PoolBackend`/`RawPool`, `Malloc`) and the kernel map's page provider
//! (`PageMemory`) are safe traits (decision 30): their implementations (the bridges registered
//! once by the kernel's boot path with the kernel's own `pool(9)` and `malloc(9)`; the
//! provider the kernel's own `km` is and passes down as a value, N3.2) build nothing unsafe;
//! they return a pointer and the size they granted. That the block is the kernel's memory,
//! exclusively the caller's until it is freed through the same implementation, is the
//! **registration contract** (`sys/hal/bridges.rs`; for the page provider, the contract of
//! `crate::kmbuf::PageMemory`), on which the `// SAFETY:` of every site that makes a
//! reference or a slice of such a block rests. It is a documented trust boundary, not a
//! compile-time proof, and it goes with the bridges (N9) and, for the pages, when the kernel
//! map's record is typed (N3.5). Since the page provider is a value and not a slot, the
//! boundary is chosen at each call: every caller of `KmBuf::alloc_pageable`/`alloc_wired` and
//! of `kernel::pipe::Pipes::new` names its provider, and in a kernel image the one production
//! choice is `sys/kern/pipe_glue.rs`'s `PIPES`, which passes the kernel's
//! `kernel::vm::km::page_memory()` (the test doubles are `cfg(test)` code).
//!
//! What can be checked is checked here, on every answer: the pointer is not null (it is a
//! `NonNull`), it is aligned as the request needs, and the size granted covers the request.
//! A failed check is `panic(9)` naming the bridge: a kernel whose allocator lies about its
//! memory cannot continue. Under `diagnostic` a fresh block is also probed: its first and
//! last bytes are written and read back (then restored), so a block that is not memory, or
//! not writable, fails at its first use instead of corrupting something later.
//!
//! The machine's `kcopy` (`kcopy_in`/`kcopy_out` of `arch/<machine>/copy.rs`, `hal`'s own since
//! N4.8b; the `UserCopy` bridge's before) writes and reads kernel memory named by an address.
//! `hal` calls it for two holders only. One is the `hal::user::KcopyToken`, taken once by
//! `bsd`'s boot (`hal_bridges_register`'s first step, before the boot glue calls
//! `abi::abi_init`; an earlier take panics the boot, so no `forbid` crate can hold it) for
//! `uiomove`'s `UIO_SYSSPACE` leg and
//! `/dev/mem`: its holder vouches that every address it
//! passes is a kernel buffer of the length copied that its builder owns for the copy
//! (`struct uio`'s contract), nothing here checks it, and it goes with `UioMove` (N7). The
//! other is a `hal::user::ForeignWindow`, which the kernel map's token alone mints
//! (`KernelMapper::foreign_window`): nothing checks the range, so a window is a write to any
//! kernel address its maker names, the strongest primitive the token gives. The token's
//! contract (as for `kenter`'s addresses, which no check here could establish either): its
//! holder mints a window only over a range its own `uvm_map_extract` returned into the
//! kernel map, onto foreign pages no Rust object occupies, or (N5.9t1) over a range its own
//! `uvm_map(kernel_map, .., uobj, ..)` of a memory object returned (tmpfs's aobj: an aobj
//! not marked as viewed by `hal`, `kernel::vm::aobj::UAO_FLAG_HALVIEW`, whose pages hold no
//! Rust object and fault in on first touch), and unmaps that range only after the
//! window is gone (N3.5 S1 makes this a type: windows minted only from the extract's or the
//! map's result, `kernel::vm::km`'s `KernelWindow` and `ObjectWindow`). The window bounds
//! every copy (a range past it is `panic(9)`); the fault on an unmapped page is the machine's
//! `pcb_onfault`, an `EFAULT`. A window's raw legs (`copyout_at`, `copyin_at`, `kcopy_to`,
//! `kcopy_from`, over the machine's `copy_raw`) copy between its pages and a user address the
//! routine checks, or a kernel buffer named by the one holder of `hal::user::WindowKcopyToken`
//! (taken once at boot by `kernel::uio`, which the two kernel legs require): only a
//! `UIO_SYSSPACE` iovec's, which its builder vouches for (`struct uio`'s contract, the
//! `KcopyToken`'s trust), so holding a window gives no copy onto a kernel address.
//!
//! The pmap's page provider (`hal::pmap::PhysPages`, N3.1, passed to `pmap_bootstrap` as a
//! value by the same boot path) is under the same contract for the frames it hands out, and a
//! machine's pmap views a frame through the direct map only after [`check_frame`]: the frame
//! page-aligned, its whole page inside the direct map the boot glue described.
//!
//! The "past caring" flags (`hal::cpu::set_panicking`, `set_db_active`, N4.5a) are a third
//! boundary: set, they turn off the kernel lock's witness check on release
//! (`hal::klock::release_all`, `bsd`'s `kern_lock` checks alike) and make every CPU own every
//! mutex for the assertions, as the C gives up its checks after a panic or in ddb. Their only
//! callers are `bsd`'s `panic` and ddb's entry and exit; a soundness argument that reads them
//! rests on that until `panic(9)` and ddb move into `hal` (N9).
//!
//! The u-area provider (`hal::thread::UAreaMemory`, N4.5a; the kernel's
//! `kernel::vm::glue::UAREAS`, registered once by the boot path) is under the same contract
//! for the u-areas it hands out: `USPACE` bytes of zeroed, wired kernel memory, mapped and
//! writable, nothing else's until `free` gets the address back. `hal` writes a thread's frames
//! and reads its pcb there (`hal::thread`'s E4); it checks the alignment of every answer, and
//! no other address reaches a slot's `p_addr`.
//!
//! A pool's page source (`hal::pool::PageSource`, N6.0) is a value the kernel passes when it
//! makes the pool (`BoxPool::init_with_source`, `TypedPool::init_with_source`): its pages
//! are kernel memory the pool may write its items into, nothing else's until `free` takes them
//! back. `bsd`'s pool backend asks for them through `hal::pool::source_page`, which runs
//! every page through [`check_block`] (the length asked for, aligned to it when the pool
//! wants it aligned) before `pool(9)` writes a header into it. The one production source is
//! the kernel's (N6.1's mbuf pages over `km_alloc`).
//!
//! The SMR CPU walk (`hal::smr::CpuWalk`, N6.0) is a value the SMR thread passes to
//! `hal::smr::Batch::run`: the kernel's scheduler (`kernel::smr::SchedCpus`), the one
//! production walk. Its contract: `running(n)` answers `true` for every CPU that may run a
//! thread, since a CPU it skips is not waited for and a read section open there could still
//! see an object the batch then frees. `hal` checks what it can see: a skipped CPU found inside
//! a read section, and a `peg(n)` after which CPU `n` has not passed a quiescent state, are
//! `panic(9)`; a skipped CPU that enters a section after the check is what the contract
//! covers. The `// SAFETY:` of `hal::smr::SmrPtr::load` rests on it.
//!
//! A socket address lent by an interface address (`hal::link::SaLink`, N6.3a) is a fifth
//! boundary: `struct ifaddr`'s `ifa_addr`, `ifa_dstaddr`, `ifa_netmask` and `struct ifmaddr`'s
//! `ifma_addr` name storage `bsd`'s internet protocols own (`struct in_ifaddr`'s `ia_addr`,
//! `ia_dstaddr`, `ia_sockmask`, `struct in6_ifaddr`'s, `in_multi`'s `inm_sin`, `in6_multi`'s),
//! and the kernel's interface layer copies their bytes (`ifa_ifwithaddr`,
//! `ifaof_ifpforaddr`, `ifconf`) through `SaLink::read`. What the types give: a link names a
//! storage only through `SaLink::set`, which takes a `&'static Cell<T>` of an `AbiPod` image
//! (so the storage is `size_of::<T>()` initialised bytes that live for ever, and the copy is
//! cut at that size whatever `sa_len` says) and the `hal::link::SaToken`, taken once by
//! `bsd`'s boot (`hal_bridges_register`'s first steps, before any code of the kernel or ABI
//! crates runs; a later take is `None`, and `bsd`'s own take panics the boot if someone took
//! it first, so no `forbid` crate can publish a storage); the storage's address and size
//! change and are read as one pair (a sequence); and the copy takes a proof that the net lock
//! is held (`hal::netlock::NetHeld`, review 2's S-a): kernel code makes it from a witness of
//! the lock (`NetHeld::of`, exact) or borrows it for a closure from its own detached write
//! holding (`hal::netlock::lend_detached`, exact, the holding kept until the closure returns:
//! N6.3b2), and only the token holder can make it from a check of `rw_status`
//! (`SaToken::net_held`), for its own holdings, which no witness exists of. The
//! token holder's contract, the C's own invariant: the bytes of a storage it published are
//! written only while it holds the net lock exclusively (`in_ifinit`, `in6_update_ifa`;
//! `bsd`'s `in_ifaddr`, `in_multi` and `in6_ifaddr` are `Sync` by `bsd`'s own `unsafe impl`,
//! so this, not the cells' type, keeps writers out); the storage behind its `&'static` lives
//! while the interface address naming it lives (it is freed only after `ifa_del` has unlinked
//! the address and its last reference is gone, `ifafree`: `bsd`'s `'static`, a lie of the
//! family its `&'static Ifnet` belongs to); and it asks for a proof by check only while it
//! holds the lock itself, and drops the proof before it lets that holding go (the `NetHeld`
//! of a `&'static SaToken` is `NetHeld<'static>`, so no borrow ends it for the holder): for a
//! shared holding `rw_status` cannot say whose it is (the C's `NET_ASSERT_LOCKED()` cannot
//! either), so a read holding it finds is its own, as every C reader of `ifa_addr` holds one.
//! The `// SAFETY:` of `SaLink::read` rests on these; the boundary goes when
//! `struct in_ifaddr` and `struct in6_ifaddr` are the kernel's and the interface address
//! embeds its storage (N6.4, N6.5).
//!
//! ## Redesign
//! - Not a port: the C trusts `km_alloc`, `pool_get` and `malloc` silently. Here the trust is
//!   one function every memory piece calls, so it is audited in one place.

use core::ptr::NonNull;

use crate::assert::hal_panic;

/// Checks a frame `pa` (from `PhysPages`, or a caller's `Paddr`) before a pmap views it through
/// the direct map `[base, end)` that the boot glue described: `pa` page-aligned (`page` a
/// power of two), the map set (`base` not 0), and the whole page inside it. Its direct-map
/// address; panics naming the frame otherwise.
#[cfg_attr(
    not(all(target_os = "none", target_arch = "x86_64")),
    allow(dead_code, reason = "amd64's pmap views frames through its direct map")
)]
pub(crate) fn check_frame(pa: usize, page: usize, base: usize, end: usize) -> usize {
    let va = base.checked_add(pa).filter(|va| {
        base != 0 && pa & (page - 1) == 0 && va.checked_add(page).is_some_and(|e| e <= end)
    });
    match va {
        Some(va) => va,
        None => hal_panic(format_args!(
            "PhysPages: frame {pa:#x} outside the direct map [{base:#x}, {end:#x})"
        )),
    }
}

/// Checks a block `bridge` returned for a request of `requested` bytes aligned to `align`:
/// aligned, and `granted` covering the request; probes it under `diagnostic`. Panics naming
/// the bridge when a check fails.
pub(crate) fn check_block(
    bridge: &'static str,
    ptr: NonNull<u8>,
    granted: usize,
    requested: usize,
    align: usize,
) {
    if !ptr.as_ptr().addr().is_multiple_of(align.max(1)) {
        hal_panic(format_args!(
            "{bridge}: block {ptr:p} not aligned to {align} bytes"
        ));
    }
    if granted < requested {
        hal_panic(format_args!(
            "{bridge}: block of {granted} bytes for a request of {requested}"
        ));
    }
    if ptr.as_ptr().addr().checked_add(granted).is_none() {
        hal_panic(format_args!(
            "{bridge}: block {ptr:p} of {granted} bytes ends past the address space"
        ));
    }
    #[cfg(feature = "diagnostic")]
    probe(bridge, ptr, requested);
}

/// Writes the complement of the first and the last byte of a fresh block, reads it back and
/// restores it: a block that is not writable memory fails here, at its first use.
#[cfg(feature = "diagnostic")]
fn probe(bridge: &'static str, ptr: NonNull<u8>, len: usize) {
    if len == 0 {
        return;
    }
    let first = ptr.as_ptr();
    let last = first.wrapping_add(len - 1);
    for p in [first, last] {
        // SAFETY: the registration contract (module docs): `ptr` is a fresh block of at least
        // `len` bytes of kernel memory that nothing else references yet, so its first and last
        // bytes are writable and owned by the caller; the byte is restored before the block is
        // used, so its contents (zeroed or not) are as the allocator left them.
        let ok = unsafe {
            let old = p.read_volatile();
            p.write_volatile(!old);
            let back = p.read_volatile();
            p.write_volatile(old);
            back == !old
        };
        if !ok {
            hal_panic(format_args!(
                "{bridge}: block {ptr:p} is not writable memory"
            ));
        }
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_block_passes() {
        let mut b = [0u64; 4];
        let p = NonNull::from(&mut b).cast::<u8>();
        check_block("test", p, 32, 32, 8);
        assert_eq!(b, [0; 4]);
    }

    #[test]
    #[should_panic(expected = "test: block of 16 bytes for a request of 32")]
    fn a_short_block_panics() {
        let mut b = [0u64; 4];
        check_block("test", NonNull::from(&mut b).cast::<u8>(), 16, 32, 8);
    }

    #[test]
    #[should_panic(expected = "ends past the address space")]
    fn a_block_past_the_end_of_memory_panics() {
        let mut b = [0u64; 4];
        let p = NonNull::from(&mut b).cast::<u8>();
        check_block("test", p, usize::MAX, 32, 8);
    }

    #[test]
    #[should_panic(expected = "not aligned to 8 bytes")]
    fn a_misaligned_block_panics() {
        let mut b = [0u64; 4];
        let p = NonNull::from(&mut b).cast::<u8>();
        let odd = NonNull::new(p.as_ptr().wrapping_add(1)).unwrap();
        check_block("test", odd, 31, 8, 8);
    }
}
/* </TESTS> */
