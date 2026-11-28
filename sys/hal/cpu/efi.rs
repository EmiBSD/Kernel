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
 * Copyright (c) 2017 Mark Kettenis <kettenis@openbsd.org>
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
 * Copyright (c) 2022 Mark Kettenis <kettenis@openbsd.org>
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
//! The UEFI runtime services as one object with typed methods: `efi_map_runtime` and what
//! efi(4) calls between `efi_enter` and `efi_leave` (`arch/arm64/dev/efi_machdep.c`,
//! `machine/efivar.h`).
//!
//! Upstream: sys/arch/arm64/dev/efi_machdep.c @ 3ce1f3f79392
//! Upstream: sys/arch/arm64/include/efivar.h @ 3ce1f3f79392
//! LZ: sys/arch/arm64/dev/efi_machdep.rs@e8249449d905
//! LZ: sys/arch/arm64/include/efivar.rs@e8249449d905
//!
//! [`EfiRuntime::install`] runs once per machine: it makes the runtime services' pmap (the EFI
//! pmap of `hal::vm::pmap::create_efi`), fills it from the firmware's memory map with the C's
//! policy (`efi_map_runtime`: only the `EFI_MEMORY_RUNTIME` descriptors, identity-mapped when
//! `VirtualStart` is 0, device memory unless write-back, executable only
//! `EfiRuntimeServicesCode`, then `EFI_MEMORY_RP`, `XP` and `RO` taking their right away), and
//! keeps the address of `EFI_RUNTIME_SERVICES`. Nothing maps a page into it after that: no
//! method adds a mapping. Each later method is one transaction: this object's lock, then the
//! machine's `efi_enter` (interrupts masked, the EFI pmap in the lower half, the FPU usable),
//! the work, and `efi_leave` (the FPU trapped again, the CPU's own pmap back, the interrupt
//! state restored) on every path:
//! - [`EfiRuntime::reads`]: bytes of the firmware's tables (the vendor string, the
//!   configuration table) through `kcopy`, each read's fault being its own `EFAULT`, all in
//!   one transaction as the C's one `efi_enter`;
//! - [`EfiRuntime::get_time`], [`EfiRuntime::set_time`]: `GetTime(&time, NULL)` and
//!   `SetTime(&time)`, the service's address read from its slot and called in the same
//!   transaction, a fault in either being `EFAULT` (`efi_enter_check`'s `setjmp`).
//!
//! What stays trusted, as the C trusts it: the firmware's memory map (that its runtime
//! descriptors name the firmware's own memory and code) and the runtime services table it
//! names. Two bounds the C does not write hold the rest: the map may not name a frame the
//! VM manages or one of the kernel image (`install` refuses it with `EINVAL` before it maps
//! anything, as `km::map_device` refuses such frames), and what is read, and the service
//! called, lie below the EFI pmap's limit (`1 << EFI_SPACE_BITS`), so in the lower half, which
//! only the EFI pmap translates during a transaction: an address of the kernel's half is
//! `EFAULT`, never a read of kernel memory or a call into kernel code.
//!
//! The machine's part (`hal::arch::arm64::efi`: the trampoline, the switch) is reached through
//! [`EfiLocked`], which only this object's lock makes; the host double answers from a fake
//! firmware per test thread.
//!
//! ## Deviations
//! - A memory map whose runtime descriptors name managed memory or the kernel image, or reach
//!   at or past the EFI pmap's limit, maps nothing and installs no runtime (`EINVAL`); the C
//!   would map them.
//! - A memory map without a descriptor size maps nothing (the C would divide by zero), as LZ.
//!
//! ## Redesign
//! - LZ's `efi_map_runtime(sc)` (`bsd`'s, the descriptor loop over `pmap_enter` on the EFI
//!   pmap the softc kept in `sc_pm`) -> [`EfiRuntime::install`], which applies the policy itself
//!   ([`runtime_mapping`], [`descriptors`]) so that no caller can enter a page of its choosing,
//!   executable or not, into a pmap the trampoline runs with interrupts masked.
//! - `struct efi_softc`'s `sc_pm`, `sc_rs` and `sc_psw` (LZ: `Cell`s of raw pointers) ->
//!   [`EfiRuntime`]'s pmap handle and table address, and the transaction's interrupt guard.
//! - `efi_enter_check(sc)`, the call and `efi_leave(sc)` (LZ's `efi_call(sc, slot, a0, a1)`:
//!   any slot, any two integers) -> the typed methods above, each one transaction.

use core::mem::{offset_of, size_of};

use sysdefs::base::errno::Errno;
use sysdefs::base::types::{Paddr, Vaddr};
use sysdefs::dev::efi::{
    EFI_MEMORY_RO, EFI_MEMORY_RP, EFI_MEMORY_RUNTIME, EFI_MEMORY_WB, EFI_MEMORY_XP,
    EfiMemoryDescriptor, EfiRuntimeServices, EfiRuntimeServicesCode, EfiStatus, EfiTime,
};
use sysdefs::vm::mman::{PROT_EXEC, PROT_READ, PROT_WRITE};

use crate::arch::current::efi as machine;
use crate::cpu::intr::IPL_HIGH;
use crate::sync::mutex::{Mutexed, MutexedGuard};
use crate::sync::once::OnceStatic;
use crate::user::pod_from_bytes;
use crate::vm::pmap::{PMAP_WIRED, PmapRef};

#[cfg(not(target_os = "none"))]
pub use crate::arch::current::efi::fake;

/// The UEFI's page size (`NumberOfPages` counts 4 KiB pages).
const EFI_PAGE_SIZE: u64 = 4096;

/// The runtime services of this machine: their pmap, the table's address and the lock every
/// transaction holds.
pub struct EfiRuntime {
    /// `sc_pm`: the EFI pmap, filled once by `install`, kept for the kernel's lifetime (the C
    /// never destroys it).
    pm: PmapRef,
    /// `sc_rs`: the address of `EFI_RUNTIME_SERVICES`, mapped by `pm`.
    rs: u64,
    /// One transaction at a time: the checkpoint the machine's trampoline writes is one per
    /// machine, as the C's `efi_jmpbuf` (and `sc_psw`), and a second CPU's call would overwrite
    /// it. Held at `IPL_HIGH`: the trap of a fault inside a call unmasks the CPU's interrupts
    /// (`do_el1h_sync`), and this level keeps every device interrupt out meanwhile.
    lock: Mutexed<()>,
}

/// The proof that the caller holds the runtime's lock: made only by [`EfiRuntime`]'s methods,
/// handed to the machine's transaction.
pub(crate) struct EfiLocked<'a> {
    _held: MutexedGuard<'a, ()>,
}

/// What a read inside [`EfiRuntime::reads`]'s transaction may do.
pub struct EfiReads<'a> {
    tx: &'a machine::Inside,
}

/// What a runtime descriptor maps ([`runtime_mapping`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeMapping {
    /// The first virtual address (the physical one when `VirtualStart` is 0).
    pub va: u64,
    /// The first physical address.
    pub pa: u64,
    /// Not write-back cacheable: mapped as device memory (`PMAP_DEVICE`).
    pub device: bool,
    /// `PROT_*`.
    pub prot: i32,
    /// `NumberOfPages`.
    pub npages: u64,
}

/// The installed runtime services. Installed once per process: on the host its pmap belongs
/// to the first test's memory, which is leaked and never freed or grown from again (nothing
/// enters a page after `install`), so no `*_test_reset` is needed; every test reads the
/// thread's own fake firmware through it.
static RUNTIME: OnceStatic<EfiRuntime> = OnceStatic::new();

/// Whether `[va, va + len)` lies below `limit`: the bound every read and call is held to.
pub(crate) fn below(va: u64, len: usize, limit: u64) -> bool {
    u64::try_from(len)
        .ok()
        .and_then(|l| va.checked_add(l))
        .is_some_and(|end| end <= limit)
}

/// The descriptors of the memory map `map`, `desc_size` bytes apart (`NextMemoryDescriptor`);
/// none when `desc_size` is 0, and a short last entry is not read.
pub fn descriptors(map: &[u8], desc_size: u32) -> impl Iterator<Item = EfiMemoryDescriptor> + '_ {
    let step = desc_size as usize;
    let n = map.len().checked_div(step).unwrap_or(0);
    (0..n).map_while(move |i| {
        map.get(i * step..i * step + size_of::<EfiMemoryDescriptor>())
            .and_then(pod_from_bytes::<EfiMemoryDescriptor>)
    })
}

/// `efi_map_runtime`'s decision for one descriptor: `None` unless it is runtime memory
/// (`EFI_MEMORY_RUNTIME`).
pub fn runtime_mapping(desc: &EfiMemoryDescriptor) -> Option<RuntimeMapping> {
    if desc.Attribute & EFI_MEMORY_RUNTIME == 0 {
        return None;
    }
    let mut prot = PROT_READ | PROT_WRITE;

    // If the virtual address is still zero, use an identity mapping.
    let va = if desc.VirtualStart == 0 {
        desc.PhysicalStart
    } else {
        desc.VirtualStart
    };

    // Normal memory is expected to be "write back" cacheable. Everything else is mapped as
    // device memory.
    let device = desc.Attribute & EFI_MEMORY_WB == 0;

    // Only make pages marked as runtime service code executable. This violates the standard
    // but it seems we can get away with it.
    if desc.Type == EfiRuntimeServicesCode {
        prot |= PROT_EXEC;
    }

    if desc.Attribute & EFI_MEMORY_RP != 0 {
        prot &= !PROT_READ;
    }
    if desc.Attribute & EFI_MEMORY_XP != 0 {
        prot &= !PROT_EXEC;
    }
    if desc.Attribute & EFI_MEMORY_RO != 0 {
        prot &= !PROT_WRITE;
    }

    Some(RuntimeMapping {
        va,
        pa: desc.PhysicalStart,
        device,
        prot,
        npages: desc.NumberOfPages,
    })
}

/// The pages of one mapping, as `(va, pa)` pairs; `None` when one of them overflows.
fn pages(m: &RuntimeMapping) -> impl Iterator<Item = Option<(u64, u64)>> + '_ {
    (0..m.npages).map(move |i| {
        let off = i.checked_mul(EFI_PAGE_SIZE)?;
        Some((m.va.checked_add(off)?, m.pa.checked_add(off)?))
    })
}

/// Every page the runtime descriptors of `map` would enter is below `limit` and none of its
/// frames is kernel memory (`kernel_memory`: a page the VM manages or one of the kernel
/// image): `EINVAL` otherwise.
fn check_map(
    map: &[u8],
    desc_size: u32,
    limit: u64,
    kernel_memory: impl Fn(u64) -> bool,
) -> Result<(), Errno> {
    for m in descriptors(map, desc_size).filter_map(|d| runtime_mapping(&d)) {
        for p in pages(&m) {
            let (va, pa) = p.ok_or(Errno::EINVAL)?;
            if !below(va, EFI_PAGE_SIZE as usize, limit) || kernel_memory(pa) {
                return Err(Errno::EINVAL);
            }
        }
    }
    Ok(())
}

impl EfiRuntime {
    /// `efi_map_runtime` and the table's address, once per machine: the EFI pmap
    /// (`pmap_create` with `pm_privileged` and `have_4_level_pt`), every page of the runtime
    /// descriptors of the firmware's memory map `map` (`desc_size` bytes apart) entered in it
    /// with the C's policy ([`runtime_mapping`]), wired, and `rs`, the address of
    /// `EFI_RUNTIME_SERVICES` under it. `EBUSY` when they are installed already, `ENOMEM` when
    /// no pmap could be had, `EINVAL` (nothing mapped) when a page would be kernel memory or
    /// lie at or above the pmap's limit.
    pub fn install(rs: u64, map: &[u8], desc_size: u32) -> Result<&'static EfiRuntime, Errno> {
        if RUNTIME.get().is_some() {
            return Err(Errno::EBUSY);
        }
        let pm = crate::vm::pmap::create_efi().ok_or(Errno::ENOMEM)?;
        let limit = pm.va_limit().as_usize() as u64;
        check_map(map, desc_size, limit, |pa| {
            crate::vm::pmap::frame_is_kernel_memory(Paddr::new(pa as usize))
        })?;
        for m in descriptors(map, desc_size).filter_map(|d| runtime_mapping(&d)) {
            let flag = if m.device { machine::PMAP_DEVICE } else { 0 };
            for (va, pa) in pages(&m).flatten() {
                // A wired mapping without PMAP_CANFAIL: the pmap panics rather than fail.
                let _ = pm.enter(
                    Vaddr::new(va as usize),
                    Paddr::new(pa as usize | flag),
                    m.prot,
                    m.prot | PMAP_WIRED,
                );
            }
        }
        let rt = EfiRuntime {
            pm,
            rs,
            lock: Mutexed::new(IPL_HIGH, ()),
        };
        RUNTIME.set(rt).map_err(|_| Errno::EBUSY)?;
        RUNTIME.get().ok_or(Errno::EBUSY)
    }

    /// The installed runtime services, if any.
    pub fn get() -> Option<&'static EfiRuntime> {
        RUNTIME.get()
    }

    /// The page mapped at `va` in the runtime's pmap: a test's view of what `install` entered.
    #[cfg(test)]
    pub(crate) fn extract(&self, va: u64) -> Option<u64> {
        self.pm
            .extract(Vaddr::new(va as usize))
            .map(|pa| pa.as_usize() as u64)
    }

    /// This runtime's lock, as the proof the machine's transaction takes.
    fn locked(&self) -> EfiLocked<'_> {
        EfiLocked {
            _held: self.lock.lock(),
        }
    }

    /// `f` with reads of the firmware's tables, in one transaction (the C's `efi_enter` around
    /// the vendor string and the configuration table): each [`EfiReads::read`] is `EFAULT`
    /// alone when its bytes are not all mapped or not all below the limit.
    pub fn reads<R>(&self, f: impl FnOnce(&EfiReads<'_>) -> R) -> R {
        let held = self.locked();
        machine::transaction(&held, &self.pm, |tx| f(&EfiReads { tx }))
    }

    /// The service of the slot at byte `slot` of `EFI_RUNTIME_SERVICES` called with `time`, in
    /// one transaction: the slot read, then the call; `EFAULT` when either faults or lies at
    /// or above the limit.
    fn time_service(&self, slot: usize, time: &mut EfiTime) -> Result<EfiStatus, Errno> {
        let va = self.rs.checked_add(slot as u64).ok_or(Errno::EFAULT)?;
        let held = self.locked();
        machine::transaction(&held, &self.pm, |tx| {
            let mut w = [0u8; 8];
            tx.read(va, &mut w)?;
            tx.time_call(u64::from_ne_bytes(w), time)
        })
    }

    /// `GetTime(&time, NULL)`: the status and the time the firmware wrote; `EFAULT` on a
    /// fault (`efi_enter_check`).
    pub fn get_time(&self) -> Result<(EfiStatus, EfiTime), Errno> {
        let mut time = EfiTime::default();
        let status = self.time_service(offset_of!(EfiRuntimeServices, GetTime), &mut time)?;
        Ok((status, time))
    }

    /// `SetTime(&time)`: the status; `EFAULT` on a fault.
    pub fn set_time(&self, time: &EfiTime) -> Result<EfiStatus, Errno> {
        let mut time = *time;
        self.time_service(offset_of!(EfiRuntimeServices, SetTime), &mut time)
    }
}

impl EfiReads<'_> {
    /// `buf.len()` bytes at the firmware's address `va`, under the EFI pmap: `EFAULT` when
    /// they are not all mapped there or not all below its limit.
    pub fn read(&self, va: u64, buf: &mut [u8]) -> Result<(), Errno> {
        self.tx.read(va, buf)
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;
    use sysdefs::dev::efi::{EfiConventionalMemory, EfiRuntimeServicesData};

    const RS: u64 = 0x4000_0000;
    const LIMIT: u64 = 1 << 48;
    /// The runtime region the process's runtime is installed with: one page of code.
    const CODE_PA: u64 = 0x7f00_0000;

    fn desc(ty: u32, pa: u64, va: u64, pages: u64, attr: u64) -> EfiMemoryDescriptor {
        EfiMemoryDescriptor {
            Type: ty,
            Pad: 0,
            PhysicalStart: pa,
            VirtualStart: va,
            NumberOfPages: pages,
            Attribute: attr,
        }
    }

    /// A memory map of `descs`, 48 bytes apart (a descriptor size larger than the structure).
    fn map(descs: &[EfiMemoryDescriptor]) -> std::vec::Vec<u8> {
        let mut m = std::vec![0u8; descs.len() * 48];
        for (i, d) in descs.iter().enumerate() {
            let b = &mut m[i * 48..];
            b[0..4].copy_from_slice(&d.Type.to_ne_bytes());
            b[8..16].copy_from_slice(&d.PhysicalStart.to_ne_bytes());
            b[16..24].copy_from_slice(&d.VirtualStart.to_ne_bytes());
            b[24..32].copy_from_slice(&d.NumberOfPages.to_ne_bytes());
            b[32..40].copy_from_slice(&d.Attribute.to_ne_bytes());
        }
        m
    }

    #[test]
    fn the_bound_holds_every_byte_below_the_limit() {
        assert!(below(LIMIT - 8, 8, LIMIT));
        assert!(!below(LIMIT - 7, 8, LIMIT));
        assert!(below(LIMIT, 0, LIMIT));
        assert!(!below(u64::MAX - 3, 8, LIMIT), "an overflow is out");
        assert!(below(0, 0, LIMIT));
    }

    #[test]
    fn walk_descriptors() {
        // LZ's walk test: two descriptors 48 bytes apart, a size of 0 walking nothing, a
        // short last entry not read.
        let m = map(&[
            desc(
                EfiRuntimeServicesCode,
                0x4000_0000,
                0,
                2,
                EFI_MEMORY_RUNTIME,
            ),
            desc(11, 0, 0, 0, 0),
        ]);
        let d: std::vec::Vec<_> = descriptors(&m, 48).collect();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].Type, EfiRuntimeServicesCode);
        assert_eq!(d[0].PhysicalStart, 0x4000_0000);
        assert_eq!(d[0].NumberOfPages, 2);
        assert_eq!(d[0].Attribute, EFI_MEMORY_RUNTIME);
        assert_eq!(d[1].Type, 11);
        assert_eq!(descriptors(&m, 0).count(), 0);
        assert_eq!(descriptors(&m[..80], 40).count(), 2);
        assert_eq!(descriptors(&m[..79], 40).count(), 1);
    }

    #[test]
    fn runtime_descriptors_map_as_the_c_decides() {
        let rt = EFI_MEMORY_RUNTIME;
        // Not runtime memory: nothing.
        assert_eq!(
            runtime_mapping(&desc(
                EfiConventionalMemory,
                0x4000_0000,
                0,
                1,
                EFI_MEMORY_WB
            )),
            None
        );
        // Runtime code, write-back: identity (VirtualStart 0), executable.
        assert_eq!(
            runtime_mapping(&desc(
                EfiRuntimeServicesCode,
                0x5000_0000,
                0,
                3,
                rt | EFI_MEMORY_WB
            )),
            Some(RuntimeMapping {
                va: 0x5000_0000,
                pa: 0x5000_0000,
                device: false,
                prot: PROT_READ | PROT_WRITE | PROT_EXEC,
                npages: 3,
            })
        );
        // Runtime data without WB: device, not executable, at its virtual address.
        assert_eq!(
            runtime_mapping(&desc(EfiRuntimeServicesData, 0x900_0000, 0x1000, 1, rt)),
            Some(RuntimeMapping {
                va: 0x1000,
                pa: 0x900_0000,
                device: true,
                prot: PROT_READ | PROT_WRITE,
                npages: 1,
            })
        );
        // RP, XP and RO take their right away.
        let all = rt | EFI_MEMORY_WB | EFI_MEMORY_RP | EFI_MEMORY_XP | EFI_MEMORY_RO;
        let m = runtime_mapping(&desc(EfiRuntimeServicesCode, 0x6000_0000, 0, 1, all));
        assert_eq!(m.map(|m| m.prot), Some(0));
        let m = runtime_mapping(&desc(
            EfiRuntimeServicesCode,
            0x6000_0000,
            0,
            1,
            rt | EFI_MEMORY_WB | EFI_MEMORY_RO,
        ));
        assert_eq!(m.map(|m| m.prot), Some(PROT_READ | PROT_EXEC));
    }

    #[test]
    fn a_map_naming_kernel_memory_or_past_the_limit_is_refused() {
        let rt = EFI_MEMORY_RUNTIME | EFI_MEMORY_WB;
        let managed = 0x4100_0000u64;
        let kernel_memory = |pa: u64| (managed..managed + 0x1_0000).contains(&pa);
        // A runtime region over a managed frame: refused, whatever comes first.
        let m = map(&[
            desc(EfiRuntimeServicesCode, CODE_PA, 0, 1, rt),
            desc(EfiRuntimeServicesData, managed + 0x2000, 0, 1, rt),
        ]);
        assert_eq!(check_map(&m, 48, LIMIT, kernel_memory), Err(Errno::EINVAL));
        // The same frame named by a descriptor that is not runtime memory is never mapped,
        // so it is not refused.
        let m = map(&[
            desc(EfiRuntimeServicesCode, CODE_PA, 0, 1, rt),
            desc(EfiConventionalMemory, managed, 0, 4, EFI_MEMORY_WB),
        ]);
        assert_eq!(check_map(&m, 48, LIMIT, kernel_memory), Ok(()));
        // A region identity-mapped at or past the limit is refused.
        let m = map(&[desc(EfiRuntimeServicesData, LIMIT - 0x1000, 0, 2, rt)]);
        assert_eq!(check_map(&m, 48, LIMIT, kernel_memory), Err(Errno::EINVAL));
        // A page count that overflows the address space is refused.
        let m = map(&[desc(EfiRuntimeServicesData, 0x1000, 0, u64::MAX, rt)]);
        assert_eq!(check_map(&m, 48, LIMIT, kernel_memory), Err(Errno::EINVAL));
    }

    /// The process's runtime (installed once), over the host's EFI pmap: one runtime code page
    /// at `CODE_PA`, a conventional page beside it.
    fn runtime() -> &'static EfiRuntime {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let _ = testutil::pmap_boot();
            let m = map(&[
                desc(
                    EfiRuntimeServicesCode,
                    CODE_PA,
                    0,
                    1,
                    EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
                ),
                desc(EfiConventionalMemory, CODE_PA + 0x1000, 0, 1, EFI_MEMORY_WB),
            ]);
            assert!(EfiRuntime::install(RS, &m, 48).is_ok());
        });
        EfiRuntime::get().expect("installed")
    }

    fn time(year: u16, month: u8, day: u8) -> EfiTime {
        EfiTime {
            Year: year,
            Month: month,
            Day: day,
            Hour: 3,
            Minute: 4,
            Second: 5,
            ..EfiTime::default()
        }
    }

    #[test]
    fn the_runtime_maps_its_regions_once_and_nothing_after() {
        let rt = runtime();
        assert_eq!(rt.extract(CODE_PA), Some(CODE_PA));
        assert_eq!(rt.extract(CODE_PA + 0x1000), None, "not runtime memory");
        // A second install maps nothing: the pmap stays as the first left it.
        let more = map(&[desc(
            EfiRuntimeServicesCode,
            0x7e00_0000,
            0,
            1,
            EFI_MEMORY_RUNTIME | EFI_MEMORY_WB,
        )]);
        assert_eq!(EfiRuntime::install(RS, &more, 48).err(), Some(Errno::EBUSY));
        assert_eq!(rt.extract(0x7e00_0000), None);
        assert!(core::ptr::eq(rt, runtime()));
    }

    #[test]
    fn a_read_is_refused_across_the_limit_and_where_nothing_is_mapped() {
        let rt = runtime();
        fake::reset();
        fake::map(0x5000, &[1, 2, 3, 4]);
        let mut b = [0u8; 4];
        rt.reads(|r| {
            assert_eq!(r.read(0x5000, &mut b), Ok(()));
            assert_eq!(b, [1, 2, 3, 4]);
            assert_eq!(
                r.read(0x5002, &mut b),
                Err(Errno::EFAULT),
                "past the mapping"
            );
            fake::map(LIMIT - 2, &[9, 9]);
            assert_eq!(
                r.read(LIMIT - 2, &mut b),
                Err(Errno::EFAULT),
                "across the limit"
            );
            assert_eq!(r.read(0xffff_0000_0000_0000, &mut b), Err(Errno::EFAULT));
            // A failed read leaves the transaction usable.
            assert_eq!(r.read(0x5000, &mut b[..2]), Ok(()));
        });
    }

    #[test]
    fn get_time_reads_the_slot_and_calls_the_service() {
        let rt = runtime();
        fake::reset();
        fake::map(RS + 24, &0x6000u64.to_ne_bytes());
        let t = time(2026, 11, 28);
        fake::service(0x6000, 0, Some(t));
        assert_eq!(rt.get_time(), Ok((0, t)));
        // A status other than success is the firmware's, handed back.
        fake::service(0x6000, 7, Some(t));
        assert_eq!(rt.get_time().map(|(s, _)| s), Ok(7));
        assert_eq!(fake::calls(), 2);
    }

    #[test]
    fn a_fault_or_a_service_of_the_kernel_half_is_efault() {
        let rt = runtime();
        fake::reset();
        // No slot mapped: the read of the slot faults.
        assert_eq!(rt.get_time(), Err(Errno::EFAULT));
        // A slot naming the kernel's half is refused before any call.
        fake::map(RS + 24, &0xffff_8000_0000_1000u64.to_ne_bytes());
        assert_eq!(rt.get_time(), Err(Errno::EFAULT));
        assert_eq!(fake::calls(), 0);
        // The service itself faults.
        fake::map(RS + 24, &0x6000u64.to_ne_bytes());
        fake::fault(0x6000);
        assert_eq!(rt.get_time(), Err(Errno::EFAULT));
        assert_eq!(fake::calls(), 1);
    }

    #[test]
    fn set_time_hands_the_firmware_the_time() {
        let rt = runtime();
        fake::reset();
        fake::map(RS + 32, &0x7000u64.to_ne_bytes());
        fake::service(0x7000, 0, None);
        let t = time(2027, 1, 2);
        assert_eq!(rt.set_time(&t), Ok(0));
        assert_eq!(fake::last_time(), Some(t));
    }
}
/* </TESTS> */
