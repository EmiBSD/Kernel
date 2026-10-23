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
//! The loader of `abi_openbsd.ko` (decision 22): takes the file the boot loader handed
//! over, places it in memory it owns, relocates it for the address it will run at, maps
//! each page with its segment's protection, checks the module's header and calls its entry
//! point once, before `init(8)` starts; the module registers its system call handler from
//! there (through `KernelApi`, into [`crate::syscall`]).
//!
//! Everything but the memory itself is safe code over slices: the parse, the placement and
//! the relocations are `super::elf`'s, bounds-checked. Three `unsafe` remain, each where a
//! fact comes from outside the type system: the file's bytes and the writable pages (what
//! the platform says it handed over) and the entry point (an address the module's header
//! names, checked to lie in its executable pages).
//!
//! Where the module runs: the machine's module window (`crate::pmap::module_window`), on
//! amd64 and arm64 alike (arm64's since N3.1 commit 7, decision 37's symmetry), each page
//! entered there by `crate::pmap::module_enter` with the physical page behind the writable
//! copy (`pmap_extract` of the kernel pmap), and nowhere else: without a window, or with one
//! the bootstrap could not link or too small, the module is not loaded (`NoMemory`), never
//! placed in `kernel_map` instead.
//!
//! # The platform premise
//!
//! The boot loader's hand-over is still `bsd`'s (the boot entry moves at N4.1), so the loader
//! reaches it through [`ModulePlatform`], which `bsd`'s boot glue implements
//! (`sys/kern/abi_module.rs`) and registers once, before `main`, in the bridge registry
//! (`crate::bridges::register_module_platform`). The kernel memory of the writable copy is
//! the kernel's own allocator since N3.2 (`kernel::vm::km::module_memory()`), a
//! [`ModuleMemory`] passed to [`load`] as a value. The soundness of the three `unsafe` rests
//! on those two doing what each method's documentation says: the kernel's own boot hand-over
//! and `km_alloc`, the same facts the rest of the kernel already trusts; the mapping is
//! `hal`'s own (`crate::pmap::module_enter`). When `hal` or `kernel` holds the boot entry
//! (N4.1) and the kernel map's record is typed (N3.5), the premise becomes a proof.

use core::fmt;

use super::elf::{self, ElfError, Module, PF_W, PF_X};
use super::{
    HEADER_API_SIZE, HEADER_API_VERSION, HEADER_INIT, HEADER_MAGIC, HEADER_SIZE, MODULE_MAGIC,
    MODULE_NAME, PAGE_SIZE,
};
use crate::api::{AbiInit, ApiPrefix, KernelApi};
use crate::once::OnceStatic;
use crate::sys::types::Vaddr;

use crate::syscall::handler_info;

/// The protection a page of the module is mapped with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prot {
    /// Read only (the read-only data, the header, the relocated pointers).
    Read,
    /// Read and write (the data, the bss).
    ReadWrite,
    /// Read and execute (the text).
    ReadExec,
}

/// What the loader needs from the boot hand-over (the platform premise, above). Implemented
/// by `bsd`'s boot glue until the boot entry moves (N4.1).
pub trait ModulePlatform: Sync {
    /// The module file as the boot loader handed it over: the kernel virtual address of its
    /// first byte and its length, mapped readable and never reused for the kernel's
    /// lifetime; `None` when the boot loader handed over none.
    fn image(&self) -> Option<(usize, usize)>;
}

/// The kernel memory the loader's writable copy takes (`km_alloc`/`km_free`), as the kernel
/// provides it (`kernel::vm::km::module_memory()`) and passes it to [`load`] as a value: a
/// safe trait whose contract the loader's `unsafe` rests on (the platform premise, above).
pub trait ModuleMemory: Sync {
    /// `km_alloc(round_page(len), &kv_any, &kp_zero, &kd_waitok)`: `len` bytes, rounded up to
    /// whole pages, of zeroed, wired kernel memory, mapped writable, page-aligned, owned by the
    /// caller until [`Self::release`]: its kernel virtual address; `None` when there is none.
    fn alloc(&self, len: usize) -> Option<Vaddr>;

    /// Unmaps the writable range `[va, va + len)` (from [`Self::alloc`], the same `len`) and
    /// gives back its virtual space, keeping its physical pages, which the module window's
    /// mappings (`crate::pmap::module_enter`) still hold: after it, no writable alias of the
    /// module's text and read-only data is left but the direct map, which aliases every page
    /// of memory as OpenBSD's does. Nothing may touch `va` after.
    fn release(&self, va: Vaddr, len: usize);
}

/// Why the module was not loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The boot loader handed over no module.
    NoImage,
    /// The file is not a module this loader takes.
    Elf(ElfError),
    /// The platform had no memory for it, or the machine no module window that holds it.
    NoMemory,
    /// The header at the ELF entry address is missing or malformed.
    Header,
    /// The module was built against another `KernelApi` (its version and size).
    Api(u32, u32),
    /// Its entry point is not in its executable pages.
    Entry,
    /// A page could not be mapped.
    Map,
    /// The module returned from its entry point without registering its handler.
    NoHandler,
    /// The table handed to `load` is not this tree's `KernelApi` (its prefix).
    Table(u32, u32),
    /// A module was loaded already, or a load is under way.
    Again,
}

impl From<ElfError> for LoadError {
    fn from(e: ElfError) -> Self {
        Self::Elf(e)
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{MODULE_NAME}: not loaded, ")?;
        match *self {
            Self::NoImage => write!(f, "the boot loader handed over none")?,
            Self::Elf(e) => write!(f, "{e}")?,
            Self::NoMemory => write!(f, "no memory for it")?,
            Self::Header => write!(f, "no module header at its entry address")?,
            Self::Api(v, s) => write!(f, "built against KernelApi version {v}, size {s}")?,
            Self::Entry => write!(f, "its entry point is not in its text")?,
            Self::Map => write!(f, "a page could not be mapped")?,
            Self::NoHandler => write!(f, "it registered no system call handler")?,
            Self::Table(v, s) => write!(f, "the kernel's table is version {v}, size {s}")?,
            Self::Again => return write!(f, "a module was loaded already"),
        }
        write!(f, "; every system call takes the legacy path")
    }
}

/// What a loaded module said about itself: the boot line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loaded {
    /// Entries in its system call table.
    pub syscalls: u32,
    /// How many of them are `Native`.
    pub native: u32,
}

impl fmt::Display for Loaded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{MODULE_NAME}: {} syscalls, {} native",
            self.syscalls, self.native
        )
    }
}

/// Set by the first [`load`]: the module is loaded at most once.
static LOADING: OnceStatic<()> = OnceStatic::new();

/// Loads the module the registered platform's boot hand-over holds
/// (`crate::bridges::register_module_platform`) into memory from `mem` (the kernel's
/// `km_alloc`) and calls its entry point with `api`, the table of this tree's version (its
/// prefix is checked, and the module's header must name the same). Once: a second call is
/// refused before it touches anything.
pub fn load(api: &'static KernelApi, mem: &dyn ModuleMemory) -> Result<Loaded, LoadError> {
    let platform = crate::bridges::module_platform().ok_or(LoadError::NoImage)?;
    load_with(platform, mem, api)
}

/// [`load`] over a given platform (the registered one, or a test's).
fn load_with(
    platform: &dyn ModulePlatform,
    mem: &dyn ModuleMemory,
    api: &'static KernelApi,
) -> Result<Loaded, LoadError> {
    if api.prefix != ApiPrefix::CURRENT {
        return Err(LoadError::Table(api.prefix.version, api.prefix.size));
    }
    LOADING.set(()).map_err(|()| LoadError::Again)?;
    let (va, len) = platform.image().ok_or(LoadError::NoImage)?;
    let first = core::ptr::with_exposed_provenance::<u8>(va);
    // SAFETY: the platform premise: `image` names `len` readable bytes the boot loader
    // handed over, never written or reused for the kernel's lifetime, so a shared `'static`
    // view of them aliases no write. The boot path establishes it before registering the
    // image: Limine's module is Limine's own response; boot(8)'s record is accepted only
    // when `len` is not 0 nor above the kernel's 64 MiB room and `[pa, pa + len)` lies
    // inside the kernel's image (from its physical start, within the room, covered by one
    // region the memory map calls the kernel's image: below `esym`), checked with `checked_add` by
    // `sys/stand/mod.rs`'s `abi_module_handover` (otherwise the record is dropped with one
    // line), and that range is in the direct map on both machines.
    let file: &'static [u8] = unsafe { core::slice::from_raw_parts(first, len) };
    let module = Module::parse(file, elf::host_machine())?;
    let span = module.span();

    let rw = mem.alloc(span).ok_or(LoadError::NoMemory)?.as_usize();
    let mapped = place_and_map(&module, rw, span);
    // W^X: whatever happened, the writable copy goes; the module's pages keep only the
    // protections entered.
    mem.release(Vaddr::new(rw), span);
    let init = mapped?;

    let entry = core::ptr::with_exposed_provenance::<()>(init);
    // SAFETY: `init` is the relocated `init` field of the module's header, which matched
    // the magic and names this tree's `KernelApi` (version and size, `entry_point`); the
    // module was built against `hal::api::AbiInit`, so it is the address of its
    // `fn(&'static KernelApi)` entry, and `api` is that type (its prefix checked above).
    // `entry_point` checked the address lies in a page of an executable segment, and the
    // loop above mapped every such page executable at `exec` with the relocated bytes (the
    // platform premise for the bytes, `crate::pmap::module_enter` for the mapping, which
    // makes the instruction caches coherent with them). The module stays mapped for the
    // kernel's lifetime.
    let init: AbiInit = unsafe { core::mem::transmute::<*const (), AbiInit>(entry) };
    init(api);

    let info = handler_info().ok_or(LoadError::NoHandler)?;
    Ok(Loaded {
        syscalls: info.syscalls,
        native: info.native,
    })
}

/// Places and relocates the module in the writable copy at `rw` (`span` bytes from the
/// [`ModuleMemory`]'s `alloc`), checks its header, and maps every page of it in the machine's module
/// window with its segment's protection; the relocated entry point. The caller releases `rw`
/// after it, whether it succeeded or not.
fn place_and_map(module: &Module<'_>, rw: usize, span: usize) -> Result<usize, LoadError> {
    // The machine's window, the module's alone (it is loaded once): refused when missing,
    // not linked or too small, never placed elsewhere.
    let exec = match crate::pmap::module_window() {
        Some((base, size)) if span <= size => base.as_usize(),
        _ => return Err(LoadError::NoMemory),
    };
    let init = {
        let first = core::ptr::with_exposed_provenance_mut::<u8>(rw);
        // SAFETY: the platform premise: the kernel's `ModuleMemory::alloc` handed over `span`
        // bytes of writable, wired memory owned by this caller alone, mapped until the
        // caller's `release`; this is
        // the only reference to it, and it is not used after this block, which ends before
        // this function returns and so before that `release`.
        let image: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(first, span) };
        module.place(image)?;
        module.relocate(image, exec as u64)?;
        entry_point(
            module,
            image,
            exec,
            ApiPrefix::CURRENT.size as usize,
            ApiPrefix::CURRENT.version,
        )?
    };
    for page in (0..span).step_by(PAGE_SIZE) {
        let prot = match module.flags_at(page as u64) {
            Some(f) if f & PF_X != 0 => Prot::ReadExec,
            Some(f) if f & PF_W != 0 => Prot::ReadWrite,
            Some(_) => Prot::Read,
            None => continue,
        };
        if !window_enter(exec + page, rw + page, prot) {
            return Err(LoadError::Map);
        }
    }
    Ok(init)
}

/// Maps the physical page behind the writable page at `rw` at `va`, a page of the machine's
/// module window, with `prot`; `false` if `rw` is not mapped or the window refuses it.
fn window_enter(va: usize, rw: usize, prot: Prot) -> bool {
    use crate::sys::mman::{PROT_EXEC, PROT_READ, PROT_WRITE};
    let prot = match prot {
        Prot::Read => PROT_READ,
        Prot::ReadWrite => PROT_READ | PROT_WRITE,
        Prot::ReadExec => PROT_READ | PROT_EXEC,
    };
    crate::pmap::kernel()
        .extract(Vaddr::new(rw))
        .is_some_and(|pa| crate::pmap::module_enter(Vaddr::new(va), pa, prot))
}

/// Checks the module header at the entry address of the placed, relocated `image` (which
/// will run at `exec`) against the table the kernel hands out; the relocated entry point.
fn entry_point(
    module: &Module<'_>,
    image: &[u8],
    exec: usize,
    api_size: usize,
    api_version: u32,
) -> Result<usize, LoadError> {
    let at = usize::try_from(module.entry).map_err(|_| LoadError::Header)?;
    let header = image
        .get(at..at.checked_add(HEADER_SIZE).ok_or(LoadError::Header)?)
        .ok_or(LoadError::Header)?;
    if header.get(HEADER_MAGIC..HEADER_MAGIC + 8) != Some(&MODULE_MAGIC[..]) {
        return Err(LoadError::Header);
    }
    let word = |off: usize| {
        let mut b = [0u8; 4];
        b.copy_from_slice(&header[off..off + 4]);
        u32::from_le_bytes(b)
    };
    let (version, size) = (word(HEADER_API_VERSION), word(HEADER_API_SIZE));
    if version != api_version || size as usize != api_size {
        return Err(LoadError::Api(version, size));
    }
    let init = elf::u64_at(header, HEADER_INIT).map_err(|_| LoadError::Header)?;
    let off = init.wrapping_sub(exec as u64);
    match module.flags_at(off) {
        Some(f) if f & PF_X != 0 && off < image.len() as u64 => {
            usize::try_from(init).map_err(|_| LoadError::Entry)
        }
        _ => Err(LoadError::Entry),
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use std::format;
    use std::vec;

    /// A placed image with a header at 0x1000 whose init points `init_off` into a module
    /// whose text is the first page.
    fn image_with_header(init: u64, version: u32, size: u32) -> std::vec::Vec<u8> {
        let mut image = vec![0u8; 0x3000];
        image[0x1000..0x1008].copy_from_slice(&MODULE_MAGIC);
        image[0x1008..0x100c].copy_from_slice(&version.to_le_bytes());
        image[0x100c..0x1010].copy_from_slice(&size.to_le_bytes());
        image[0x1010..0x1018].copy_from_slice(&init.to_le_bytes());
        image
    }

    /// A module file: text page 0 (R X), rodata page 1 (R), data page 2 (R W), entry
    /// 0x1000.
    fn module_file() -> std::vec::Vec<u8> {
        let mut f = vec![0u8; 0x1000];
        f[..4].copy_from_slice(&elf::ELFMAG);
        f[4] = elf::ELFCLASS64;
        f[5] = elf::ELFDATA2LSB;
        f[6] = elf::EV_CURRENT;
        f[16..18].copy_from_slice(&elf::ET_DYN.to_le_bytes());
        f[18..20].copy_from_slice(&elf::EM_X86_64.to_le_bytes());
        f[24..32].copy_from_slice(&0x1000u64.to_le_bytes());
        f[32..40].copy_from_slice(&64u64.to_le_bytes());
        f[54..56].copy_from_slice(&56u16.to_le_bytes());
        f[56..58].copy_from_slice(&3u16.to_le_bytes());
        let loads = [
            (elf::PF_R | PF_X, 0u64),
            (elf::PF_R, 0x1000),
            (elf::PF_R | PF_W, 0x2000),
        ];
        for (i, (flags, va)) in loads.iter().enumerate() {
            let at = 64 + i * 56;
            f[at..at + 4].copy_from_slice(&elf::PT_LOAD.to_le_bytes());
            f[at + 4..at + 8].copy_from_slice(&flags.to_le_bytes());
            f[at + 16..at + 24].copy_from_slice(&va.to_le_bytes());
            f[at + 40..at + 48].copy_from_slice(&0x1000u64.to_le_bytes());
        }
        f
    }

    const EXEC: usize = 0x7f00_0000;

    #[test]
    fn a_good_header_gives_the_relocated_entry() {
        let f = module_file();
        let m = Module::parse(&f, elf::EM_X86_64).unwrap();
        let image = image_with_header(EXEC as u64 + 0x40, 1, 48);
        assert_eq!(entry_point(&m, &image, EXEC, 48, 1), Ok(EXEC + 0x40));
    }

    #[test]
    fn a_bad_header_is_refused() {
        let f = module_file();
        let m = Module::parse(&f, elf::EM_X86_64).unwrap();
        let mut image = image_with_header(EXEC as u64, 1, 48);
        image[0x1000] = b'X';
        assert_eq!(entry_point(&m, &image, EXEC, 48, 1), Err(LoadError::Header));
    }

    #[test]
    fn another_kernel_api_is_refused() {
        let f = module_file();
        let m = Module::parse(&f, elf::EM_X86_64).unwrap();
        let image = image_with_header(EXEC as u64, 2, 48);
        assert_eq!(
            entry_point(&m, &image, EXEC, 48, 1),
            Err(LoadError::Api(2, 48))
        );
        let image = image_with_header(EXEC as u64, 1, 40);
        assert_eq!(
            entry_point(&m, &image, EXEC, 48, 1),
            Err(LoadError::Api(1, 40))
        );
    }

    #[test]
    fn an_entry_outside_the_text_is_refused() {
        let f = module_file();
        let m = Module::parse(&f, elf::EM_X86_64).unwrap();
        for init in [EXEC + 0x1000, EXEC + 0x2008, EXEC + 0x3000, EXEC - 8, 0] {
            let image = image_with_header(init as u64, 1, 48);
            assert_eq!(
                entry_point(&m, &image, EXEC, 48, 1),
                Err(LoadError::Entry),
                "init at {init:#x}"
            );
        }
    }

    #[test]
    fn the_boot_lines() {
        let l = Loaded {
            syscalls: 331,
            native: 0,
        };
        assert_eq!(format!("{l}"), "abi_openbsd.ko: 331 syscalls, 0 native");
        assert_eq!(
            format!("{}", LoadError::NoImage),
            "abi_openbsd.ko: not loaded, the boot loader handed over none; every system call takes the legacy path"
        );
    }

    /// A platform and a memory with nothing to give: `load` stops before any `unsafe`.
    struct Empty;

    impl ModulePlatform for Empty {
        fn image(&self) -> Option<(usize, usize)> {
            None
        }
    }

    impl ModuleMemory for Empty {
        fn alloc(&self, _: usize) -> Option<Vaddr> {
            None
        }
        fn release(&self, _: Vaddr, _: usize) {}
    }

    fn register(_: &'static KernelApi, _: crate::api::SyscallHandler, _: HandlerInfo) {}
    fn legacy(_: isize, _: &crate::syscall::SysArgs, _: &mut crate::syscall::Rval) -> SysResult {
        Ok(())
    }
    fn copyin(_: usize, _: &mut [u8]) -> Result<(), i32> {
        Ok(())
    }
    fn copyout(_: &[u8], _: usize) -> Result<(), i32> {
        Ok(())
    }

    use crate::syscall::{HandlerInfo, Rval, SysResult};

    fn pipe2(_: isize, _: usize, _: isize) -> SysResult {
        Ok(())
    }

    fn nr(_: isize) -> SysResult {
        Ok(())
    }
    fn nr_int(_: isize, _: i32) -> SysResult {
        Ok(())
    }
    fn obreak(_: isize, _: usize) -> SysResult {
        Ok(())
    }
    fn range(_: isize, _: usize, _: usize) -> SysResult {
        Ok(())
    }
    fn range_int(_: isize, _: usize, _: usize, _: i32) -> SysResult {
        Ok(())
    }
    fn mmap(_: isize, a: usize, _: usize, _: i32, _: i32, _: i32, _: i64) -> Result<usize, i32> {
        Ok(a)
    }
    fn kbind(_: isize, _: usize, _: usize, _: u64) -> SysResult {
        Ok(())
    }
    fn pins(_: isize, _: usize, _: usize, _: usize, _: i32) -> SysResult {
        Ok(())
    }

    /// Version 3's entries, every one answering `Ok`.
    const VM: crate::api::VmEntries = crate::api::VmEntries {
        obreak,
        mmap,
        munmap: range,
        mprotect: range_int,
        madvise: range_int,
        mquery: mmap,
        kbind,
        pinsyscalls: pins,
        mimmutable: range,
        mlock: range,
        munlock: range,
        minherit: range_int,
        msync: range_int,
        mlockall: nr_int,
        munlockall: nr,
    };

    fn fd1(_: isize, _: i32, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn fd2(_: isize, _: i32, _: i32, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn io(_: isize, _: i32, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn iov(_: isize, _: i32, _: usize, _: i32, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn pio(_: isize, _: i32, _: usize, _: usize, _: i64, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn piov(_: isize, _: i32, _: usize, _: i32, _: i64, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn count(_: isize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn fstat(_: isize, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn ioctl(_: isize, _: i32, _: u64, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn fcntl(_: isize, _: i32, _: i32, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn dup3(_: isize, _: i32, _: i32, _: i32, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn utrace(_: isize, _: usize, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }

    /// Version 4's entries, every one answering `Ok`.
    const FD: crate::api::FdEntries = crate::api::FdEntries {
        read: io,
        write: io,
        close: fd1,
        getdtablecount: count,
        dup: fd1,
        fstat,
        ioctl,
        dup2: fd2,
        fcntl,
        dup3,
        readv: iov,
        writev: iov,
        flock: fd2,
        pread: pio,
        pwrite: pio,
        preadv: piov,
        pwritev: piov,
        fpathconf: fd2,
        utrace,
        closefrom: fd1,
    };

    fn select(_: isize, _: i32, _: [usize; 3], _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn kevent(
        _: isize,
        _: i32,
        _: (usize, i32),
        _: (usize, i32),
        _: usize,
        _: &mut Rval,
    ) -> SysResult {
        Ok(())
    }
    fn ppoll(_: isize, _: usize, _: u32, _: usize, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn pselect(_: isize, _: i32, _: [usize; 3], _: usize, _: usize, _: &mut Rval) -> SysResult {
        Ok(())
    }
    fn poll(_: isize, _: usize, _: u32, _: i32, _: &mut Rval) -> SysResult {
        Ok(())
    }

    /// Version 5's entries, every one answering `Ok`.
    const EVENT: crate::api::EventEntries = crate::api::EventEntries {
        select,
        kevent,
        ppoll,
        pselect,
        poll,
        kqueue: count,
        kqueue1: fd1,
    };

    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_fork(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_vfork(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_tfork(nr: isize, param: usize, psize: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((param as i64) * 10 + (psize as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_exit(nr: isize, status: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((status as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_threxit(nr: isize, notdead: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((notdead as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_wait4(
        nr: isize,
        pid: i32,
        status: usize,
        options: i32,
        rusage: usize,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1000
            + (status as i64) * 100
            + (options as i64) * 10
            + (rusage as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_waitid(
        nr: isize,
        idtype: i32,
        id: u32,
        info: usize,
        options: i32,
        rval: &mut Rval,
    ) -> SysResult {
        rval[0] = nr;
        Err(
            ((idtype as i64) * 1000 + (id as i64) * 100 + (info as i64) * 10 + (options as i64) * 1)
                as i32,
        )
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrusage(nr: isize, who: i32, rusage: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((who as i64) * 10 + (rusage as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrlimit(nr: isize, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (rlp as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setrlimit(nr: isize, which: i32, rlp: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (rlp as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpriority(nr: isize, which: i32, who: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 10 + (who as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setpriority(nr: isize, which: i32, who: u32, prio: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((which as i64) * 100 + (who as i64) * 10 + (prio as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_acct(nr: isize, path: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((path as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getppid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getuid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_geteuid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getgid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getegid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getgroups(nr: isize, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gidsetsize as i64) * 10 + (gidset as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setgroups(nr: isize, gidsetsize: i32, gidset: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gidsetsize as i64) * 10 + (gidset as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpgrp(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getpgid(nr: isize, pid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getsid(nr: isize, pid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setpgid(nr: isize, pid: i32, pgid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((pid as i64) * 10 + (pgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setsid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setuid(nr: isize, uid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((uid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_seteuid(nr: isize, euid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((euid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setgid(nr: isize, gid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((gid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setegid(nr: isize, egid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((egid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setreuid(nr: isize, ruid: u32, euid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 10 + (euid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setregid(nr: isize, rgid: u32, egid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 10 + (egid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setresuid(nr: isize, ruid: u32, euid: u32, suid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 100 + (euid as i64) * 10 + (suid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setresgid(nr: isize, rgid: u32, egid: u32, sgid: u32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 100 + (egid as i64) * 10 + (sgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getresuid(nr: isize, ruid: usize, euid: usize, suid: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((ruid as i64) * 100 + (euid as i64) * 10 + (suid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getresgid(nr: isize, rgid: usize, egid: usize, sgid: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rgid as i64) * 100 + (egid as i64) * 10 + (sgid as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_issetugid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getlogin_r(nr: isize, namebuf: usize, namelen: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((namebuf as i64) * 10 + (namelen as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setlogin(nr: isize, namebuf: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((namebuf as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_set_tcb(nr: isize, tcb: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tcb as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_get_tcb(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getthrid(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getthrname(nr: isize, tid: i32, name: usize, len: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tid as i64) * 100 + (name as i64) * 10 + (len as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setthrname(nr: isize, tid: i32, name: usize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((tid as i64) * 10 + (name as i64) * 1) as i32)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_getrtable(nr: isize, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(0)
    }
    #[allow(clippy::identity_op, clippy::erasing_op)]
    fn p_setrtable(nr: isize, rtableid: i32, rval: &mut Rval) -> SysResult {
        rval[0] = nr;
        Err(((rtableid as i64) * 1) as i32)
    }

    const PROC: crate::api::ProcEntries = crate::api::ProcEntries {
        fork: p_fork,
        vfork: p_vfork,
        tfork: p_tfork,
        exit: p_exit,
        threxit: p_threxit,
        wait4: p_wait4,
        waitid: p_waitid,
        getrusage: p_getrusage,
        getrlimit: p_getrlimit,
        setrlimit: p_setrlimit,
        getpriority: p_getpriority,
        setpriority: p_setpriority,
        acct: p_acct,
        getpid: p_getpid,
        getppid: p_getppid,
        getuid: p_getuid,
        geteuid: p_geteuid,
        getgid: p_getgid,
        getegid: p_getegid,
        getgroups: p_getgroups,
        setgroups: p_setgroups,
        getpgrp: p_getpgrp,
        getpgid: p_getpgid,
        getsid: p_getsid,
        setpgid: p_setpgid,
        setsid: p_setsid,
        setuid: p_setuid,
        seteuid: p_seteuid,
        setgid: p_setgid,
        setegid: p_setegid,
        setreuid: p_setreuid,
        setregid: p_setregid,
        setresuid: p_setresuid,
        setresgid: p_setresgid,
        getresuid: p_getresuid,
        getresgid: p_getresgid,
        issetugid: p_issetugid,
        getlogin_r: p_getlogin_r,
        setlogin: p_setlogin,
        set_tcb: p_set_tcb,
        get_tcb: p_get_tcb,
        getthrid: p_getthrid,
        getthrname: p_getthrname,
        setthrname: p_setthrname,
        getrtable: p_getrtable,
        setrtable: p_setrtable,
    };

    static API: KernelApi = KernelApi::new(
        register, legacy, copyin, copyout, pipe2, VM, FD, EVENT, PROC,
    );

    /// The one test that calls `load` (its once-guard is process-wide): no image is
    /// reported, and the second call is refused.
    #[test]
    fn no_image_then_no_second_load() {
        assert_eq!(load_with(&Empty, &Empty, &API), Err(LoadError::NoImage));
        assert_eq!(load_with(&Empty, &Empty, &API), Err(LoadError::Again));
        assert_eq!(
            format!("{}", LoadError::Again),
            "abi_openbsd.ko: not loaded, a module was loaded already"
        );
    }

    #[test]
    fn another_table_is_refused() {
        // Version 1's table (N2.4), before pipe2: 40 bytes.
        static OTHER: KernelApi = KernelApi {
            prefix: ApiPrefix {
                version: 1,
                size: 40,
            },
            ..KernelApi::new(
                register, legacy, copyin, copyout, pipe2, VM, FD, EVENT, PROC,
            )
        };
        assert_eq!(
            load_with(&Empty, &Empty, &OTHER),
            Err(LoadError::Table(1, 40))
        );
    }

    /// Without a module window (the host double's, as on a machine whose bootstrap could not
    /// link one) the module is refused as `NoMemory` before anything is placed or mapped:
    /// the writable copy stays as `alloc` gave it. `place_and_map` rather than `load_with`,
    /// whose once-guard the test above takes for the whole process.
    #[test]
    fn no_window_is_no_memory_and_maps_nothing() {
        assert_eq!(crate::pmap::module_window(), None);
        let f = module_file();
        let m = Module::parse(&f, elf::EM_X86_64).unwrap();
        let span = m.span();
        let mut copy = vec![0u8; span];
        let rw = copy.as_mut_ptr().expose_provenance();
        assert_eq!(place_and_map(&m, rw, span), Err(LoadError::NoMemory));
        assert!(copy.iter().all(|&b| b == 0));
    }
}
/* </TESTS> */
