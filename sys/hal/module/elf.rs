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
//! ELF64 as the module loader reads it: the file header, the program headers, the dynamic
//! section and its RELA relocations, written from the System V gABI and the AMD64 and
//! AArch64 processor supplements (no crate: docs/ARCHITECTURE.md, "Dependencies").
//!
//! The structures are declared `#[repr(C)]` as the specification lays them out (a
//! compile-time check pins their sizes) and are read from the file's bytes field by field
//! with checked offsets: a short or lying file is an [`ElfError`], never an out-of-bounds
//! read, and nothing is cast. Only what an `abi_openbsd.ko` may contain is accepted: a
//! little-endian `ET_DYN` (static PIE) of the running machine, page-aligned `PT_LOAD`s none
//! of which is both writable and executable, no interpreter, no TLS, no needed library,
//! and relocations that are all `R_*_RELATIVE` (the module imports nothing).

use core::fmt;
use core::mem::size_of;

use super::PAGE_SIZE;
// `<sys/exec_elf.h>`'s own values, where its type is the one read here.
pub use crate::sys::exec_elf::{
    ELFCLASS64, ELFDATA2LSB, ELFMAG, EM_AARCH64, ET_DYN, PF_R, PF_W, PF_X, PT_DYNAMIC, PT_INTERP,
    PT_LOAD, PT_TLS,
};

/// `EV_CURRENT` as the byte `e_ident[EI_VERSION]` holds (`<sys/exec_elf.h>`'s is a `u32`).
pub const EV_CURRENT: u8 = crate::sys::exec_elf::EV_CURRENT as u8;
/// `EM_X86_64` (`<sys/exec_elf.h>`'s `EM_AMD64`).
pub const EM_X86_64: u16 = crate::sys::exec_elf::EM_AMD64;

/// `DT_NULL`.
pub const DT_NULL: i64 = 0;
/// `DT_NEEDED`.
pub const DT_NEEDED: i64 = 1;
/// `DT_PLTRELSZ`.
pub const DT_PLTRELSZ: i64 = 2;
/// `DT_RELA`.
pub const DT_RELA: i64 = 7;
/// `DT_RELASZ`.
pub const DT_RELASZ: i64 = 8;
/// `DT_RELAENT`.
pub const DT_RELAENT: i64 = 9;
/// `DT_REL`.
pub const DT_REL: i64 = 17;
/// `DT_RELR`.
pub const DT_RELR: i64 = 36;

/// `R_X86_64_NONE`, `R_AARCH64_NONE`.
pub const R_NONE: u32 = 0;
/// `R_X86_64_RELATIVE`.
pub const R_X86_64_RELATIVE: u32 = 8;
/// `R_AARCH64_RELATIVE`.
pub const R_AARCH64_RELATIVE: u32 = 1027;

/// The most `PT_LOAD`s a module may have (it has three).
pub const MAX_LOADS: usize = 8;
/// The largest image a module may span (16 MiB): a bound on what a corrupt header can ask.
pub const MAX_SPAN: u64 = 16 << 20;

/// `Elf64_Ehdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elf64Ehdr {
    /// `e_ident`.
    pub e_ident: [u8; 16],
    /// `e_type`.
    pub e_type: u16,
    /// `e_machine`.
    pub e_machine: u16,
    /// `e_version`.
    pub e_version: u32,
    /// `e_entry`.
    pub e_entry: u64,
    /// `e_phoff`.
    pub e_phoff: u64,
    /// `e_shoff`.
    pub e_shoff: u64,
    /// `e_flags`.
    pub e_flags: u32,
    /// `e_ehsize`.
    pub e_ehsize: u16,
    /// `e_phentsize`.
    pub e_phentsize: u16,
    /// `e_phnum`.
    pub e_phnum: u16,
    /// `e_shentsize`.
    pub e_shentsize: u16,
    /// `e_shnum`.
    pub e_shnum: u16,
    /// `e_shstrndx`.
    pub e_shstrndx: u16,
}

impl Elf64Ehdr {
    /// The header at the start of `file`.
    pub fn read(file: &[u8]) -> Result<Self, ElfError> {
        let mut e_ident = [0u8; 16];
        e_ident.copy_from_slice(bytes(file, 0, 16)?);
        Ok(Self {
            e_ident,
            e_type: u16_at(file, 16)?,
            e_machine: u16_at(file, 18)?,
            e_version: u32_at(file, 20)?,
            e_entry: u64_at(file, 24)?,
            e_phoff: u64_at(file, 32)?,
            e_shoff: u64_at(file, 40)?,
            e_flags: u32_at(file, 48)?,
            e_ehsize: u16_at(file, 52)?,
            e_phentsize: u16_at(file, 54)?,
            e_phnum: u16_at(file, 56)?,
            e_shentsize: u16_at(file, 58)?,
            e_shnum: u16_at(file, 60)?,
            e_shstrndx: u16_at(file, 62)?,
        })
    }
}

/// `Elf64_Phdr`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elf64Phdr {
    /// `p_type`.
    pub p_type: u32,
    /// `p_flags`.
    pub p_flags: u32,
    /// `p_offset`.
    pub p_offset: u64,
    /// `p_vaddr`.
    pub p_vaddr: u64,
    /// `p_paddr`.
    pub p_paddr: u64,
    /// `p_filesz`.
    pub p_filesz: u64,
    /// `p_memsz`.
    pub p_memsz: u64,
    /// `p_align`.
    pub p_align: u64,
}

impl Elf64Phdr {
    /// The program header at `off` in `file`.
    pub fn read(file: &[u8], off: usize) -> Result<Self, ElfError> {
        bytes(file, off, size_of::<Self>())?;
        Ok(Self {
            p_type: u32_at(file, off)?,
            p_flags: u32_at(file, off + 4)?,
            p_offset: u64_at(file, off + 8)?,
            p_vaddr: u64_at(file, off + 16)?,
            p_paddr: u64_at(file, off + 24)?,
            p_filesz: u64_at(file, off + 32)?,
            p_memsz: u64_at(file, off + 40)?,
            p_align: u64_at(file, off + 48)?,
        })
    }
}

/// `Elf64_Dyn`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elf64Dyn {
    /// `d_tag`.
    pub d_tag: i64,
    /// `d_un`.
    pub d_val: u64,
}

impl Elf64Dyn {
    /// The dynamic entry at `off` in `file`.
    pub fn read(file: &[u8], off: usize) -> Result<Self, ElfError> {
        bytes(file, off, size_of::<Self>())?;
        Ok(Self {
            d_tag: u64_at(file, off)? as i64,
            d_val: u64_at(file, off + 8)?,
        })
    }
}

/// `Elf64_Rela`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Elf64Rela {
    /// `r_offset`: where, as a virtual address of the image.
    pub r_offset: u64,
    /// `r_info`: the symbol index (high 32 bits) and the type (low 32 bits).
    pub r_info: u64,
    /// `r_addend`.
    pub r_addend: i64,
}

impl Elf64Rela {
    /// The relocation at `off` in `file`.
    pub fn read(file: &[u8], off: usize) -> Result<Self, ElfError> {
        bytes(file, off, size_of::<Self>())?;
        Ok(Self {
            r_offset: u64_at(file, off)?,
            r_info: u64_at(file, off + 8)?,
            r_addend: u64_at(file, off + 16)? as i64,
        })
    }

    /// `ELF64_R_TYPE`.
    pub const fn r_type(&self) -> u32 {
        self.r_info as u32
    }

    /// `ELF64_R_SYM`.
    pub const fn r_sym(&self) -> u32 {
        (self.r_info >> 32) as u32
    }
}

/// Why a file is not a module this loader takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElfError {
    /// A read past the end of the file, or an offset that overflows.
    Truncated,
    /// Not ELF64, little-endian, version 1, `ET_DYN`.
    NotModule,
    /// An ELF file for another machine (the `e_machine` found).
    Machine(u16),
    /// A program header this loader does not take (its type): an interpreter or TLS.
    Segment(u32),
    /// More `PT_LOAD`s than [`MAX_LOADS`], or none.
    LoadCount,
    /// A `PT_LOAD` that does not start on a page, whose file part passes its memory part
    /// or the file, or that overlaps the one before it (its virtual address).
    Layout(u64),
    /// A `PT_LOAD` both writable and executable (its virtual address).
    WriteExec(u64),
    /// The image spans more than [`MAX_SPAN`].
    TooLarge,
    /// A dynamic entry this loader does not take (its tag): a needed library, PLT or `REL`
    /// or `RELR` relocations, a `RELA` entry size other than 24.
    Dynamic(i64),
    /// A relocation table outside the file's loaded parts.
    RelocTable,
    /// A relocation of a type this loader does not apply, or against a symbol (the type and
    /// the offset).
    Reloc(u32, u64),
    /// A relocation whose target is outside the image (its offset).
    RelocTarget(u64),
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Truncated => write!(f, "truncated ELF file"),
            Self::NotModule => write!(f, "not a little-endian ELF64 ET_DYN file"),
            Self::Machine(m) => write!(f, "ELF file for machine {m}"),
            Self::Segment(t) => write!(f, "program header of type {t:#x} not supported"),
            Self::LoadCount => write!(f, "no PT_LOAD, or more than {MAX_LOADS}"),
            Self::Layout(va) => write!(f, "PT_LOAD at {va:#x} misplaced"),
            Self::WriteExec(va) => write!(f, "PT_LOAD at {va:#x} is writable and executable"),
            Self::TooLarge => write!(f, "image larger than {} MiB", MAX_SPAN >> 20),
            Self::Dynamic(t) => write!(f, "dynamic entry {t} not supported"),
            Self::RelocTable => write!(f, "relocation table outside the image"),
            Self::Reloc(t, o) => write!(f, "relocation type {t} at {o:#x} not supported"),
            Self::RelocTarget(o) => write!(f, "relocation at {o:#x} outside the image"),
        }
    }
}

/// A `PT_LOAD`, checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Load {
    /// `p_vaddr`, page-aligned.
    pub vaddr: u64,
    /// `p_memsz`.
    pub memsz: u64,
    /// `p_offset`.
    pub offset: u64,
    /// `p_filesz`, at most `memsz`.
    pub filesz: u64,
    /// `p_flags`.
    pub flags: u32,
}

/// A parsed module file: its loads, its entry address and its relocations, all checked
/// against the file.
#[derive(Clone, Copy, Debug)]
pub struct Module<'a> {
    file: &'a [u8],
    machine: u16,
    /// `e_entry`: where the module header is.
    pub entry: u64,
    loads: [Option<Load>; MAX_LOADS],
    /// The file offset and count of the RELA relocations.
    rela: (usize, usize),
    span: u64,
}

impl<'a> Module<'a> {
    /// Parses `file` as a module for `machine` (`EM_X86_64` or `EM_AARCH64`).
    pub fn parse(file: &'a [u8], machine: u16) -> Result<Self, ElfError> {
        let eh = Elf64Ehdr::read(file)?;
        if eh.e_ident[..4] != ELFMAG
            || eh.e_ident[4] != ELFCLASS64
            || eh.e_ident[5] != ELFDATA2LSB
            || eh.e_ident[6] != EV_CURRENT
            || eh.e_type != ET_DYN
            || usize::from(eh.e_phentsize) != size_of::<Elf64Phdr>()
        {
            return Err(ElfError::NotModule);
        }
        if eh.e_machine != machine {
            return Err(ElfError::Machine(eh.e_machine));
        }
        let phoff = usize::try_from(eh.e_phoff).map_err(|_| ElfError::Truncated)?;
        let mut loads = [None; MAX_LOADS];
        let mut nloads = 0;
        let mut dynamic = None;
        let mut end = 0u64;
        for i in 0..usize::from(eh.e_phnum) {
            let off = i
                .checked_mul(size_of::<Elf64Phdr>())
                .and_then(|o| o.checked_add(phoff))
                .ok_or(ElfError::Truncated)?;
            let ph = Elf64Phdr::read(file, off)?;
            match ph.p_type {
                PT_LOAD => {
                    let load = check_load(file, &ph, end)?;
                    end = load.vaddr + load.memsz;
                    *loads.get_mut(nloads).ok_or(ElfError::LoadCount)? = Some(load);
                    nloads += 1;
                }
                PT_DYNAMIC => dynamic = Some(ph),
                PT_INTERP | PT_TLS => return Err(ElfError::Segment(ph.p_type)),
                _ => {}
            }
        }
        if nloads == 0 {
            return Err(ElfError::LoadCount);
        }
        let span = round_page(end);
        if span > MAX_SPAN {
            return Err(ElfError::TooLarge);
        }
        let mut m = Self {
            file,
            machine,
            entry: eh.e_entry,
            loads,
            rela: (0, 0),
            span,
        };
        if let Some(d) = dynamic {
            m.rela = m.dynamic(&d)?;
        }
        Ok(m)
    }

    /// The bytes the image spans in memory, a whole number of pages.
    pub fn span(&self) -> usize {
        self.span as usize
    }

    /// The checked `PT_LOAD`s, in address order.
    pub fn loads(&self) -> impl Iterator<Item = &Load> + '_ {
        self.loads.iter().flatten()
    }

    /// The protection of the page at image offset `off`: that of the load covering it, or
    /// `None` for a page no load covers.
    pub fn flags_at(&self, off: u64) -> Option<u32> {
        self.loads()
            .find(|l| off >= l.vaddr && off < round_page(l.vaddr + l.memsz))
            .map(|l| l.flags)
    }

    /// Copies every load's file bytes to its place in `image` (`span()` bytes) and zeroes
    /// the rest.
    pub fn place(&self, image: &mut [u8]) -> Result<(), ElfError> {
        if image.len() != self.span() {
            return Err(ElfError::TooLarge);
        }
        image.fill(0);
        for l in self.loads() {
            let src = bytes(self.file, l.offset as usize, l.filesz as usize)?;
            let at = l.vaddr as usize;
            image
                .get_mut(at..at + src.len())
                .ok_or(ElfError::Layout(l.vaddr))?
                .copy_from_slice(src);
        }
        Ok(())
    }

    /// Applies every relocation to the placed `image`, as if it ran at `base`; the number
    /// applied.
    pub fn relocate(&self, image: &mut [u8], base: u64) -> Result<usize, ElfError> {
        let relative = match self.machine {
            EM_X86_64 => R_X86_64_RELATIVE,
            _ => R_AARCH64_RELATIVE,
        };
        let (table, count) = self.rela;
        let mut applied = 0;
        for i in 0..count {
            let rel = Elf64Rela::read(self.file, table + i * size_of::<Elf64Rela>())?;
            if rel.r_type() == R_NONE {
                continue;
            }
            if rel.r_type() != relative || rel.r_sym() != 0 {
                return Err(ElfError::Reloc(rel.r_type(), rel.r_offset));
            }
            let at =
                usize::try_from(rel.r_offset).map_err(|_| ElfError::RelocTarget(rel.r_offset))?;
            let value = base.wrapping_add(rel.r_addend as u64);
            image
                .get_mut(
                    at..at
                        .checked_add(8)
                        .ok_or(ElfError::RelocTarget(rel.r_offset))?,
                )
                .ok_or(ElfError::RelocTarget(rel.r_offset))?
                .copy_from_slice(&value.to_le_bytes());
            applied += 1;
        }
        Ok(applied)
    }

    /// Reads the dynamic section: the RELA table's file offset and count.
    fn dynamic(&self, d: &Elf64Phdr) -> Result<(usize, usize), ElfError> {
        let (mut rela, mut relasz, mut relaent) = (None, 0u64, size_of::<Elf64Rela>() as u64);
        let base = usize::try_from(d.p_offset).map_err(|_| ElfError::Truncated)?;
        let n =
            usize::try_from(d.p_filesz).map_err(|_| ElfError::Truncated)? / size_of::<Elf64Dyn>();
        bytes(self.file, base, n * size_of::<Elf64Dyn>())?;
        for i in 0..n {
            let e = Elf64Dyn::read(self.file, base + i * size_of::<Elf64Dyn>())?;
            match e.d_tag {
                DT_NULL => break,
                DT_RELA => rela = Some(e.d_val),
                DT_RELASZ => relasz = e.d_val,
                DT_RELAENT => relaent = e.d_val,
                DT_NEEDED | DT_REL | DT_RELR => return Err(ElfError::Dynamic(e.d_tag)),
                DT_PLTRELSZ if e.d_val != 0 => return Err(ElfError::Dynamic(e.d_tag)),
                _ => {}
            }
        }
        if relaent != size_of::<Elf64Rela>() as u64 {
            return Err(ElfError::Dynamic(DT_RELAENT));
        }
        let Some(va) = rela else {
            return Ok((0, 0));
        };
        let off = self.file_offset(va, relasz).ok_or(ElfError::RelocTable)?;
        Ok((off, (relasz / relaent) as usize))
    }

    /// The file offset of the `len` bytes at virtual address `va`, if one load's file part
    /// holds them all.
    fn file_offset(&self, va: u64, len: u64) -> Option<usize> {
        let end = va.checked_add(len)?;
        let l = self
            .loads()
            .find(|l| va >= l.vaddr && end <= l.vaddr + l.filesz)?;
        usize::try_from(va - l.vaddr + l.offset).ok()
    }
}

/// The machine this kernel runs on, as `e_machine` names it.
pub const fn host_machine() -> u16 {
    if cfg!(target_arch = "aarch64") {
        EM_AARCH64
    } else {
        EM_X86_64
    }
}

/// `x` rounded up to a page.
pub const fn round_page(x: u64) -> u64 {
    (x + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1)
}

/// Checks one `PT_LOAD` against the file and the load before it (which ended at `prev`).
fn check_load(file: &[u8], ph: &Elf64Phdr, prev: u64) -> Result<Load, ElfError> {
    let bad = ElfError::Layout(ph.p_vaddr);
    let file_end = ph.p_offset.checked_add(ph.p_filesz).ok_or(bad)?;
    let mem_end = ph.p_vaddr.checked_add(ph.p_memsz).ok_or(bad)?;
    if !ph.p_vaddr.is_multiple_of(PAGE_SIZE as u64)
        || ph.p_filesz > ph.p_memsz
        || file_end > file.len() as u64
        || ph.p_vaddr < round_page(prev)
        || mem_end > MAX_SPAN
    {
        return Err(bad);
    }
    if ph.p_flags & PF_W != 0 && ph.p_flags & PF_X != 0 {
        return Err(ElfError::WriteExec(ph.p_vaddr));
    }
    Ok(Load {
        vaddr: ph.p_vaddr,
        memsz: ph.p_memsz,
        offset: ph.p_offset,
        filesz: ph.p_filesz,
        flags: ph.p_flags,
    })
}

/// `len` bytes of `file` at `off`.
fn bytes(file: &[u8], off: usize, len: usize) -> Result<&[u8], ElfError> {
    file.get(off..off.checked_add(len).ok_or(ElfError::Truncated)?)
        .ok_or(ElfError::Truncated)
}

/// A little-endian `u16` of `file` at `off`.
fn u16_at(file: &[u8], off: usize) -> Result<u16, ElfError> {
    let mut b = [0u8; 2];
    b.copy_from_slice(bytes(file, off, 2)?);
    Ok(u16::from_le_bytes(b))
}

/// A little-endian `u32` of `file` at `off`.
fn u32_at(file: &[u8], off: usize) -> Result<u32, ElfError> {
    let mut b = [0u8; 4];
    b.copy_from_slice(bytes(file, off, 4)?);
    Ok(u32::from_le_bytes(b))
}

/// A little-endian `u64` of `file` at `off`.
pub fn u64_at(file: &[u8], off: usize) -> Result<u64, ElfError> {
    let mut b = [0u8; 8];
    b.copy_from_slice(bytes(file, off, 8)?);
    Ok(u64::from_le_bytes(b))
}

// The layouts of the specification.
const _: () = {
    assert!(size_of::<Elf64Ehdr>() == 64);
    assert!(size_of::<Elf64Phdr>() == 56);
    assert!(size_of::<Elf64Dyn>() == 16);
    assert!(size_of::<Elf64Rela>() == 24);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    const BASE: u64 = 0xffff_8000_1234_0000;

    fn put16(f: &mut [u8], at: usize, v: u16) {
        f[at..at + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn put32(f: &mut [u8], at: usize, v: u32) {
        f[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn put64(f: &mut [u8], at: usize, v: u64) {
        f[at..at + 8].copy_from_slice(&v.to_le_bytes());
    }

    /// One program header: type, flags, offset, vaddr, filesz, memsz.
    fn phdr(f: &mut [u8], i: usize, p: (u32, u32, u64, u64, u64, u64)) {
        let at = 64 + i * 56;
        put32(f, at, p.0);
        put32(f, at + 4, p.1);
        put64(f, at + 8, p.2);
        put64(f, at + 16, p.3);
        put64(f, at + 24, p.3);
        put64(f, at + 32, p.4);
        put64(f, at + 40, p.5);
        put64(f, at + 48, 0x1000);
    }

    /// A module as abi.ld lays one out: text (R X) at 0, rodata (R) at 0x1000 with the
    /// header, the dynamic section and two RELATIVE relocations, data (R W) at 0x2000 with a
    /// bss tail.
    fn fixture(machine: u16) -> Vec<u8> {
        let relative = if machine == EM_X86_64 {
            R_X86_64_RELATIVE
        } else {
            R_AARCH64_RELATIVE
        };
        let mut f = vec![0u8; 0x2108];
        f[..4].copy_from_slice(&ELFMAG);
        f[4] = ELFCLASS64;
        f[5] = ELFDATA2LSB;
        f[6] = EV_CURRENT;
        put16(&mut f, 16, ET_DYN);
        put16(&mut f, 18, machine);
        put32(&mut f, 20, 1);
        put64(&mut f, 24, 0x1000); // e_entry: the header
        put64(&mut f, 32, 64); // e_phoff
        put16(&mut f, 52, 64);
        put16(&mut f, 54, 56);
        put16(&mut f, 56, 4);
        phdr(&mut f, 0, (PT_LOAD, PF_R | PF_X, 0x1000, 0, 16, 16));
        phdr(&mut f, 1, (PT_LOAD, PF_R, 0x2000, 0x1000, 0x100, 0x100));
        phdr(&mut f, 2, (PT_LOAD, PF_R | PF_W, 0x2100, 0x2000, 8, 0x40));
        phdr(&mut f, 3, (PT_DYNAMIC, PF_R, 0x2080, 0x1080, 64, 64));
        // text: two recognisable words.
        put64(&mut f, 0x1000, 0x1111_1111_1111_1111);
        put64(&mut f, 0x1008, 0x2222_2222_2222_2222);
        // rodata: the header (magic, version, size, init) ...
        f[0x2000..0x2008].copy_from_slice(b"EMIABIKO");
        put32(&mut f, 0x2008, 1);
        put32(&mut f, 0x200c, 48);
        // ... the dynamic section ...
        let dynamic = [
            (DT_RELA, 0x10c0),
            (DT_RELASZ, 48),
            (DT_RELAENT, 24),
            (DT_NULL, 0),
        ];
        for (i, (tag, val)) in dynamic.iter().enumerate() {
            put64(&mut f, 0x2080 + i * 16, *tag as u64);
            put64(&mut f, 0x2088 + i * 16, *val);
        }
        // ... and the relocations: the header's init, a data pointer to the rodata.
        let relocs = [(0x1010u64, 8u64), (0x2000, 0x1000)];
        for (i, (off, addend)) in relocs.iter().enumerate() {
            put64(&mut f, 0x20c0 + i * 24, *off);
            put64(&mut f, 0x20c8 + i * 24, u64::from(relative));
            put64(&mut f, 0x20d0 + i * 24, *addend);
        }
        // data
        put64(&mut f, 0x2100, 0x3333_3333_3333_3333);
        f
    }

    #[test]
    fn both_machines_parse() {
        for machine in [EM_X86_64, EM_AARCH64] {
            let f = fixture(machine);
            let m = Module::parse(&f, machine).unwrap();
            assert_eq!(m.span(), 0x3000);
            assert_eq!(m.entry, 0x1000);
            assert_eq!(m.loads().count(), 3);
            assert_eq!(m.rela, (0x20c0, 2));
        }
    }

    #[test]
    fn another_machine_is_refused() {
        let f = fixture(EM_AARCH64);
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Machine(EM_AARCH64)
        );
    }

    #[test]
    fn placed_and_relocated() {
        for machine in [EM_X86_64, EM_AARCH64] {
            let f = fixture(machine);
            let m = Module::parse(&f, machine).unwrap();
            let mut image = vec![0xaau8; m.span()];
            m.place(&mut image).unwrap();
            assert_eq!(u64_at(&image, 0).unwrap(), 0x1111_1111_1111_1111);
            assert_eq!(&image[0x1000..0x1008], b"EMIABIKO");
            assert_eq!(u64_at(&image, 0x2000).unwrap(), 0x3333_3333_3333_3333);
            assert!(
                image[0x2008..0x3000].iter().all(|&b| b == 0),
                "bss and padding zeroed"
            );
            assert!(image[0x10..0x1000].iter().all(|&b| b == 0));
            assert_eq!(m.relocate(&mut image, BASE).unwrap(), 2);
            assert_eq!(u64_at(&image, 0x1010).unwrap(), BASE + 8);
            assert_eq!(u64_at(&image, 0x2000).unwrap(), BASE + 0x1000);
        }
    }

    #[test]
    fn the_protection_of_each_page() {
        let f = fixture(EM_X86_64);
        let m = Module::parse(&f, EM_X86_64).unwrap();
        assert_eq!(m.flags_at(0), Some(PF_R | PF_X));
        assert_eq!(m.flags_at(0xfff), Some(PF_R | PF_X));
        assert_eq!(m.flags_at(0x1000), Some(PF_R));
        assert_eq!(m.flags_at(0x2fff), Some(PF_R | PF_W));
        assert_eq!(m.flags_at(0x3000), None);
    }

    #[test]
    fn a_writable_executable_segment_is_refused() {
        let mut f = fixture(EM_X86_64);
        phdr(&mut f, 0, (PT_LOAD, PF_R | PF_W | PF_X, 0x1000, 0, 16, 16));
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::WriteExec(0)
        );
    }

    #[test]
    fn misplaced_segments_are_refused() {
        // Not on a page.
        let mut f = fixture(EM_X86_64);
        phdr(&mut f, 1, (PT_LOAD, PF_R, 0x2000, 0x1010, 0x100, 0x100));
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Layout(0x1010)
        );
        // Overlapping the one before.
        let mut f = fixture(EM_X86_64);
        phdr(&mut f, 1, (PT_LOAD, PF_R, 0x2000, 0, 0x100, 0x100));
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Layout(0)
        );
        // File part past the file.
        let mut f = fixture(EM_X86_64);
        phdr(
            &mut f,
            2,
            (PT_LOAD, PF_R | PF_W, 0x2100, 0x2000, 0x100, 0x100),
        );
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Layout(0x2000)
        );
        // File part past the memory part.
        let mut f = fixture(EM_X86_64);
        phdr(&mut f, 2, (PT_LOAD, PF_R | PF_W, 0x2100, 0x2000, 8, 4));
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Layout(0x2000)
        );
        // Too large.
        let mut f = fixture(EM_X86_64);
        phdr(
            &mut f,
            2,
            (PT_LOAD, PF_R | PF_W, 0x2100, 0x2000, 8, MAX_SPAN),
        );
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Layout(0x2000)
        );
    }

    #[test]
    fn an_interpreter_or_tls_is_refused() {
        for t in [PT_INTERP, PT_TLS] {
            let mut f = fixture(EM_X86_64);
            phdr(&mut f, 3, (t, PF_R, 0x2080, 0x1080, 64, 64));
            assert_eq!(
                Module::parse(&f, EM_X86_64).unwrap_err(),
                ElfError::Segment(t)
            );
        }
    }

    #[test]
    fn imports_are_refused() {
        // DT_NEEDED in place of DT_RELAENT.
        let mut f = fixture(EM_X86_64);
        put64(&mut f, 0x20a0, DT_NEEDED as u64);
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::Dynamic(DT_NEEDED)
        );
        // R_X86_64_64 (1), a symbol's address.
        let mut f = fixture(EM_X86_64);
        put64(&mut f, 0x20c8, 1);
        let m = Module::parse(&f, EM_X86_64).unwrap();
        let mut image = vec![0u8; m.span()];
        m.place(&mut image).unwrap();
        assert_eq!(
            m.relocate(&mut image, BASE),
            Err(ElfError::Reloc(1, 0x1010))
        );
        // A RELATIVE relocation that names a symbol.
        let mut f = fixture(EM_AARCH64);
        put64(&mut f, 0x20c8, (5 << 32) | u64::from(R_AARCH64_RELATIVE));
        let m = Module::parse(&f, EM_AARCH64).unwrap();
        let mut image = vec![0u8; m.span()];
        assert_eq!(
            m.relocate(&mut image, BASE),
            Err(ElfError::Reloc(R_AARCH64_RELATIVE, 0x1010))
        );
    }

    #[test]
    fn a_relocation_outside_the_image_is_refused() {
        let mut f = fixture(EM_X86_64);
        put64(&mut f, 0x20c0, 0x2ffc); // the last 4 bytes of the image
        let m = Module::parse(&f, EM_X86_64).unwrap();
        let mut image = vec![0u8; m.span()];
        assert_eq!(
            m.relocate(&mut image, BASE),
            Err(ElfError::RelocTarget(0x2ffc))
        );
    }

    #[test]
    fn every_truncation_fails_cleanly() {
        let f = fixture(EM_AARCH64);
        for n in 0..f.len() {
            if let Ok(m) = Module::parse(&f[..n], EM_AARCH64) {
                let mut image = vec![0u8; m.span()];
                let _ = m.place(&mut image);
                let _ = m.relocate(&mut image, BASE);
            }
        }
        assert_eq!(
            Module::parse(&f[..63], EM_AARCH64).unwrap_err(),
            ElfError::Truncated
        );
    }

    #[test]
    fn not_a_module() {
        let mut f = fixture(EM_X86_64);
        put16(&mut f, 16, 2); // ET_EXEC
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::NotModule
        );
        let mut f = fixture(EM_X86_64);
        f[4] = 1; // ELFCLASS32
        assert_eq!(
            Module::parse(&f, EM_X86_64).unwrap_err(),
            ElfError::NotModule
        );
    }

    #[test]
    fn page_rounding() {
        assert_eq!(round_page(0), 0);
        assert_eq!(round_page(1), 0x1000);
        assert_eq!(round_page(0x1000), 0x1000);
        assert_eq!(round_page(0x1001), 0x2000);
    }
}
/* </TESTS> */
