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
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993 The Regents of the University of California.
 * Copyright (c) 1988 University of Utah.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * the Systems Programming Group of the University of Utah Computer
 * Science Department.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * from: Utah $Hdr: vm_mmap.c 1.6 91/10/21$
 *      @(#)vm_mmap.c   8.5 (Berkeley) 5/19/94
 * from: Id: uvm_mmap.c,v 1.1.2.14 1998/01/05 21:04:26 chuck Exp
 */
/* </LICENSES> */

/* <CODE> */
//! `uvm_mmap.c`: system call interface into VM system, plus kernel vm_mmap function.
//!
//! Upstream: sys/uvm/uvm_mmap.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_mmap.rs@3c62ede68802
//!
//! Every function of the file: `sys_mquery`, `uvm_wxcheck`, `sys_mmap`, `sys_msync`,
//! `sys_munmap`, `sys_mprotect`, `sys_pinsyscalls`, `sys_mimmutable`, `sys_minherit`,
//! `sys_madvise`, `sys_mlock`, `sys_munlock`, `sys_mlockall`, `sys_munlockall`,
//! `uvm_mmaplock`, `uvm_mmapanon`, `uvm_mmapfile` and `sys_kbind`.
//!
//! ## Deviations
//! - Device mappings (`VCHR` other than `/dev/zero`): `udv_attach` (`uvm_device.c`) is not
//!   ported, so `uvm_mmapfile` reports it and fails with `ENOSYS`; `/dev/zero` becomes an
//!   anonymous mapping as in the C, and block devices go through `uvn_attach` as in the C
//!   (LZ's).
//! - `pmap_wired_count` exists on both machines, so the `suser` branches of `mlock(2)` and
//!   friends are not compiled, as in the C.
//!
//! ## Redesign
//! - The system calls are functions of the address space (`&Vmspace`) and of their typed
//!   arguments, returning `Result` (the value of `*retval` in the `Ok` of `mmap` and
//!   `mquery`); `bsd`'s `sysent[]` adapters (`sys/kern/vm_bsd.rs`) unpack the `sysargs` and
//!   pass the thread and `p->p_vmspace`. N5.0: the thread is the C's `struct proc *p`
//!   (`&Thread<G>`), whose fields the calls read themselves (the limits, `ps_wxcounter`, the
//!   kbind words under `ps_mtx`, `ps_libcpin`, `P_HASSIBLING`); `sigexit` is
//!   `kernel::sig`'s and `PROC_PC` `hal::trap::proc_pc`. What is not the kernel's yet is the
//!   glue's: `pledge_protexec` (`ProcGlue`, `kern_pledge` until N8), the text vnode's
//!   `MNT_WXALLOWED` (`ProcGlue::textvp_wxallowed`, N5) and a vnode file's vnode
//!   (`FileGlue::vnode_ops`). N3.5 to N4 asked all of it of the `VmProc` bridge about
//!   `curproc`; every check stayed here, in the C's order.
//! - `uvm_wxcheck`'s notice is `log(LOG_NOTICE, ...)`, `hal::prf::log`.
//! - The file half of `sys_mmap` runs between `fd_getfile` and `FRELE` on the thread's
//!   table (`kernel::descrip`), under the kernel lock taken here, as the C's; the vnode is
//!   the pager's `VnodeOps` (`v_type`, `iszerodev(v_rdev)` and `VOP_GETATTR`'s `va_flags`
//!   are its methods).
//! - The faults a wiring takes (`mlock(2)`, `mlockall(2)`, `uvm_mmaplock`'s
//!   `VM_MAP_WIREFUTURE`) count on the caller's `p_ru`, passed down as `Some(&p.p_ru)`
//!   (the C's `curproc->p_ru`).
//! - `sys_munmap` locks the map with its write guard; `uvm_unmap_remove` fills a
//!   `DeadQueue` value detached after the unlock, as the C's stack `TAILQ`.
//! - `uvm_mmaplock` hands its write guard to `uvm_map_pageable_locked` (the C's
//!   `UVM_LK_ENTER`), which consumes it: no flag says who unlocks.
//! - `sys_pinsyscalls` builds the table in an `MArray<u32>` (`mallocarray(M_PINSYSCALL)`,
//!   freed by its drop on every error path) filled by one `copyin` of the whole array, and
//!   hands it to the process (`ps_libcpin`'s replace); LZ's `unsafe` slice over the raw
//!   block is gone. Its "only once" flag is set under the map's flags lock and the old
//!   value tested after (`VmMap::modflags` returns it), as the C's.
//! - `sys_kbind`'s check-and-set of `ps_kbind_addr`/`ps_kbind_cookie` is one critical
//!   section of `ps_mtx` ([`kbind_check`] on a [`KbindSlot`] copy of the two words, written
//!   back before the unlock). The parameter block is parsed by
//!   [`kbind_blocks`] (the sizes table-tested), and each binding is written through the
//!   extracted page's [`KernelWindow`](super::km::KernelWindow) (`kcopy` with its fault
//!   recovery, never a reference to the other mapping); the window's drop is the C's
//!   `uvm_unmap_remove` of the page from `kernel_map`. LZ's `unsafe` `kcopy` is gone.

use core::sync::atomic::{AtomicI32, Ordering};

use hal::arch::current::param::{PAGE_MASK, PAGE_SIZE};
use hal::arch::current::vmparam::{VM_MAXUSER_ADDRESS, VM_MIN_ADDRESS};
use hal::klock::{kernel_lock, kernel_unlock};
use hal::malloc::{M_WAITOK, MArray};
use hal::sys::errno::Errno;
use hal::sys::mman::{
    __MAP_NOFAULT, __MAP_NOREPLACE, MADV_DONTNEED, MADV_FREE, MADV_NORMAL, MADV_RANDOM,
    MADV_SEQUENTIAL, MADV_SPACEAVAIL, MADV_WILLNEED, MAP_ANON, MAP_CONCEAL, MAP_FIXED,
    MAP_FLAGMASK, MAP_INHERIT_COPY, MAP_INHERIT_SHARE, MAP_PRIVATE, MAP_SHARED, MAP_STACK,
    MCL_CURRENT, MCL_FUTURE, MS_ASYNC, MS_INVALIDATE, MS_SYNC, PROT_EXEC, PROT_NONE, PROT_READ,
    PROT_WRITE,
};
use hal::sys::syscall::SYS_MAXSYSCALL;
use hal::sys::types::Off;
use hal::user::UserPtr;
use hal::vm::param::{atop, ptoa, round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;

use super::init::machine;
use super::km::uvm_map_extract_window;
use super::map::{
    DeadQueue, UVM_EXTRACT_FIXPROT, VM_MAP_PINSYSCALL_ONCE, VM_MAP_WIREFUTURE, VmMap, Vmspace,
    uvm_map, uvm_map_advice, uvm_map_checkprot, uvm_map_clean, uvm_map_hint, uvm_map_immutable,
    uvm_map_inherit, uvm_map_mquery, uvm_map_pageable, uvm_map_pageable_all,
    uvm_map_pageable_locked, uvm_map_protect, uvm_mapanon, uvm_unmap, uvm_unmap_detach,
    uvm_unmap_remove,
};
use super::pager::{PGO_CLEANIT, PGO_DEACTIVATE, PGO_FREE, PGO_SYNCIO};
use super::proc::{BOGO_PC, KbindSlot, LOG_NOTICE, RLIMIT_DATA, RLIMIT_MEMLOCK};
use super::types::{
    PROT_MASK, UVM_FLAG_CONCEAL, UVM_FLAG_COPYONW, UVM_FLAG_FIXED, UVM_FLAG_NOFAULT,
    UVM_FLAG_OVERLAY, UVM_FLAG_STACK, UVM_FLAG_UNMAP, UVM_UNKNOWN_OFFSET, VmProt, Voff,
    uvm_mapflag,
};
use super::vnode::{VType, VnodeOps, uvm_vnp_uncache, uvn_attach};
use crate::descrip::fd_getfile;
use crate::file::{DTYPE_VNODE, FileGlue, FileRef};
use crate::prf::unported;
use crate::proc::{PSI_WXNEEDED, ProcGlue, Thread};
use crate::resource::Rusage;

/// `M_PINSYSCALL`: pinsyscall (`<sys/malloc.h>`).
pub const M_PINSYSCALL: i32 = 100;

/// `FREAD` (`<sys/fcntl.h>`): the file was opened for reading.
const FREAD: u32 = 0x0001;
/// `FWRITE`: the file was opened for writing.
const FWRITE: u32 = 0x0002;

/// `IMMUTABLE` (`<sys/stat.h>`): file may not be changed.
const IMMUTABLE: u64 = 0x0000_0002;
/// `APPEND`: writes to file may only append.
const APPEND: u64 = 0x0000_0004;

/// `SIGILL` (`<sys/signal.h>`).
const SIGILL: i32 = 4;
/// `SIGABRT`.
const SIGABRT: i32 = 6;

/// `KBIND_BLOCK_MAX` (`<sys/unistd.h>`): the most bindings one `kbind(2)` makes.
pub const KBIND_BLOCK_MAX: usize = 2;
/// `KBIND_DATA_MAX`: the most bytes of one binding.
pub const KBIND_DATA_MAX: usize = 24;
/// `sizeof(struct __kbind)`: `void *kb_addr` and `size_t kb_size`.
const KBSZ: usize = 2 * size_of::<usize>();
/// `sizeof(param)`: the parameter block `kbind(2)` copies in.
const KBIND_PARAM_SIZE: usize = KBIND_BLOCK_MAX * KBSZ + KBIND_DATA_MAX;

/// The arguments of `mmap(2)` and `mquery(2)` (`struct sys_mmap_args`).
#[derive(Clone, Copy, Debug)]
pub struct MmapArgs {
    /// `addr`.
    pub addr: usize,
    /// `len`.
    pub len: usize,
    /// `prot`.
    pub prot: VmProt,
    /// `flags`.
    pub flags: i32,
    /// `fd`.
    pub fd: i32,
    /// `pos`.
    pub pos: Off,
}

/// What the file half of `sys_mmap` did with a descriptor.
enum VnodeMapping {
    /// `uvm_mmapfile` mapped the vnode.
    Mapped,
    /// The vnode is `/dev/zero`: `MAP_ANON` was set, and the mapping is anonymous (the C's
    /// `goto is_anon`).
    ZeroDev,
}

/// One binding of `kbind(2)`: where it goes, and where its bytes are in the parameter block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KbindBlock {
    /// `kb_addr`.
    pub addr: usize,
    /// `kb_size`.
    pub size: usize,
    /// The offset of its bytes in the parameter block.
    pub data: usize,
}

/// `uvm_wxabort`: kill a process that attempts a W^X violation (`kern.wxabort`). \[a\]
pub static UVM_WXABORT: AtomicI32 = AtomicI32::new(0);

/// `ALIGN_ADDR(addr, size, pageoff)`: page align `addr` and `size`, returning `EINVAL` on
/// wraparound. Yields the aligned address and size and the page offset taken off.
fn align_addr(addr: usize, size: usize) -> Result<(usize, usize, usize), Errno> {
    let (mut addr, mut size) = (addr, size);
    let pageoff = addr & PAGE_MASK;
    if pageoff != 0 {
        if size > usize::MAX - pageoff {
            return Err(Errno::EINVAL); // wraparound
        }
        addr -= pageoff;
        size += pageoff;
    }
    if size != 0 {
        // (vsize_t)round_page(size), which wraps to 0 near the top of the address space.
        size = size.wrapping_add(PAGE_MASK) & !PAGE_MASK;
        if size == 0 {
            return Err(Errno::EINVAL); // wraparound
        }
    }
    Ok((addr, size, pageoff))
}

/// [`align_addr`] and the C's "disallow wrap-around" of `addr + size` that every range
/// system call follows it with: the page-aligned `[addr, addr + size)`.
fn align_range(addr: usize, size: usize) -> Result<(usize, usize), Errno> {
    let (addr, size, _) = align_addr(addr, size)?;
    if addr > usize::MAX - size {
        return Err(Errno::EINVAL); // disallow wrap-around.
    }
    Ok((addr, addr + size))
}

/// The data limit's check of a mapping of `size` bytes: `ENOMEM` when `p`'s `RLIMIT_DATA`
/// cannot hold it beside what the process uses already (`vm_dused`, read without the map
/// lock, as the C's).
fn data_limit_check<G: ProcGlue>(p: &Thread<G>, vm: &Vmspace, size: usize) -> Result<(), Errno> {
    let limit = p.lim_cur(RLIMIT_DATA as usize) as usize;
    let dused = vm.vm_dused.load(Ordering::Relaxed);
    if limit < size || limit - size < ptoa(dused as usize) {
        return Err(Errno::ENOMEM);
    }
    Ok(())
}

/// `sys_mquery`: provide mapping hints to applications that do fixed mappings.
///
/// - flags: 0 or `MAP_FIXED` (`MAP_FIXED` means that we insist on this addr and don't care
///   about `PMAP_PREFER` or such)
/// - addr: hint where we'd like to place the mapping.
/// - size: size of the mapping
/// - fd: fd of the file we want to map
/// - off: offset within the file
pub fn sys_mquery<G: ProcGlue>(p: &Thread<G>, vm: &Vmspace, a: MmapArgs) -> Result<usize, Errno> {
    let prot = a.prot;
    let mut flags = 0;

    if prot & PROT_MASK != prot {
        return Err(Errno::EINVAL);
    }

    if a.flags & MAP_FIXED != 0 {
        flags |= UVM_FLAG_FIXED;
    }

    let query = |uoff: Voff| -> Result<usize, Errno> {
        let mut vaddr = a.addr;
        if vaddr == 0 {
            vaddr = uvm_map_hint(vm, prot, VM_MIN_ADDRESS, VM_MAXUSER_ADDRESS);
        }
        uvm_map_mquery(&vm.vm_map, &mut vaddr, a.len, uoff, flags)?;
        Ok(vaddr)
    };

    if a.fd < 0 {
        return query(UVM_UNKNOWN_OFFSET);
    }

    // getvnode(p, fd, &fp), then FRELE(fp, p) after the query.
    let fp = getvnode(p, a.fd)?;
    let result = query(a.pos);
    let _ = fp.frele(Some(p));
    result
}

/// `getvnode(p, fd, &fp)` (`vfs_syscalls.c`): the file at descriptor `fd` of `p`, which must
/// be a vnode's: `EBADF` without one, `EINVAL` when it is not a vnode (`DTYPE_VNODE`),
/// `EBADF` when the vnode was cleaned out (`VBAD`); the reference given back (`FRELE`) on
/// each error.
fn getvnode<G: ProcGlue>(p: &Thread<G>, fd: i32) -> Result<FileRef<G::FileGlue>, Errno> {
    let Some(fp) = fd_getfile(p.fd(), fd) else {
        return Err(Errno::EBADF);
    };
    let error = match file_vnode::<G>(&fp) {
        None => Errno::EINVAL,
        Some(vp) if vp.vtype() == VType::VBAD => Errno::EBADF,
        Some(_) => return Ok(fp),
    };
    let _ = fp.frele(Some(p));
    Err(error)
}

/// `(struct vnode *)fp->f_data` of a `DTYPE_VNODE` file, `None` for any other file.
fn file_vnode<G: ProcGlue>(fp: &FileRef<G::FileGlue>) -> Option<&'static dyn VnodeOps> {
    if fp.f_type() != DTYPE_VNODE {
        return None;
    }
    fp.data().and_then(<G::FileGlue as FileGlue>::vnode_ops)
}

/// `uvm_wxcheck`: W^X violations are only allowed on permitted filesystems.
fn uvm_wxcheck<G: ProcGlue>(p: &Thread<G>, call: &str) -> Result<(), Errno> {
    let pr = p.process();
    let wxallowed = G::textvp_wxallowed(pr);

    if wxallowed && pr.ps_iflags.get() & PSI_WXNEEDED != 0 {
        return Ok(());
    }

    if UVM_WXABORT.load(Ordering::Relaxed) != 0 {
        kernel_lock(); // KERNEL_LOCK()
        // Report W^X failures
        let n = pr.ps_wxcounter.get();
        pr.ps_wxcounter.set(n.wrapping_add(1));
        if n == 0 {
            let comm = pr.comm();
            hal::prf::log(
                LOG_NOTICE,
                format_args!(
                    "{}({}): {} W^X violation\n",
                    core::str::from_utf8(&comm).unwrap_or("?"),
                    pr.ps_pid,
                    call
                ),
            );
        }
        // Send uncatchable SIGABRT for coredump
        crate::sig::sigexit(p, SIGABRT);
    }

    Err(Errno::ENOTSUP)
}

/// `sys_mmap`: mmap system call: the address of the mapping.
///
/// File offset and address may not be page aligned:
/// - if `MAP_FIXED`, offset and address must have remainder mod `PAGE_SIZE`
/// - if address isn't page aligned the mapping starts at `trunc_page(addr)` and the return
///   value is adjusted up by the page offset.
pub fn sys_mmap<G: ProcGlue>(p: &Thread<G>, vm: &Vmspace, a: MmapArgs) -> Result<usize, Errno> {
    let mut addr = a.addr;
    let prot = a.prot;
    let mut flags = a.flags;

    // Validate the flags.
    mmap_check(a)?;
    if prot & (PROT_WRITE | PROT_EXEC) == (PROT_WRITE | PROT_EXEC) {
        uvm_wxcheck(p, "mmap")?;
    }
    mmap_check_flags(a)?;

    G::pledge_protexec(p, prot)?;

    // align file position and save offset. adjust size.
    let (pos, size, pageoff) = align_addr(a.pos as usize, a.len)?;
    let pos = pos as Off;

    // now check (MAP_FIXED) or get (!MAP_FIXED) the "addr"
    if flags & MAP_FIXED != 0 {
        // adjust address by the same amount as we did the offset
        addr = addr.wrapping_sub(pageoff);
        mmap_check_fixed(addr, size)?;
    }

    // check for file mappings (i.e. not anonymous) and verify file.
    if flags & MAP_ANON == 0 {
        kernel_lock(); // KERNEL_LOCK()
        let Some(fp) = fd_getfile(p.fd(), a.fd) else {
            kernel_unlock(); // out: KERNEL_UNLOCK()
            return Err(Errno::EBADF);
        };
        let mapping = match file_vnode::<G>(&fp) {
            None => Err(Errno::ENODEV), // only mmap vnodes!
            Some(vp) => {
                let fflag = fp.flag() as u32;
                mmap_vnode(p, vm, vp, fflag, &mut addr, size, prot, &mut flags, pos)
            }
        };
        // FRELE(fp, p) (the /dev/zero case drops it before is_anon, the others at out:),
        // then KERNEL_UNLOCK().
        let _ = fp.frele(Some(p));
        kernel_unlock(); // KERNEL_UNLOCK()
        match mapping? {
            // remember to add offset
            VnodeMapping::Mapped => return Ok(addr + pageoff),
            // special case: SunOS style /dev/zero, mapped as anonymous memory below.
            VnodeMapping::ZeroDev => {}
        }
    } else if a.fd != -1 {
        // MAP_ANON case
        return Err(Errno::EINVAL);
    }

    // is_anon: label for SunOS style /dev/zero

    // __MAP_NOFAULT only makes sense with a backing object
    if flags & __MAP_NOFAULT != 0 {
        return Err(Errno::EINVAL);
    }

    if prot != PROT_NONE || flags & MAP_SHARED != 0 {
        data_limit_check(p, vm, size)?;
    }

    // We've been treating (MAP_SHARED|MAP_PRIVATE) == 0 as MAP_PRIVATE, so make that clear.
    if flags & MAP_SHARED == 0 {
        flags |= MAP_PRIVATE;
    }

    let maxprot = PROT_MASK;
    let locklimit = p.lim_cur(RLIMIT_MEMLOCK as usize) as usize;
    uvm_mmapanon(
        &vm.vm_map,
        &mut addr,
        size,
        prot,
        maxprot,
        flags,
        locklimit,
        Some(&p.p_ru),
    )?;

    // remember to add offset
    Ok(addr + pageoff)
}

/// `sys_mmap`'s first check: the protection's bits.
fn mmap_check(a: MmapArgs) -> Result<(), Errno> {
    if a.prot & PROT_MASK != a.prot {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `sys_mmap`'s checks of the flags, after `uvm_wxcheck`: in the C's order.
fn mmap_check_flags(a: MmapArgs) -> Result<(), Errno> {
    let (flags, prot) = (a.flags, a.prot);
    if flags & MAP_FLAGMASK != flags {
        return Err(Errno::EINVAL);
    }
    if flags & (MAP_SHARED | MAP_PRIVATE) == (MAP_SHARED | MAP_PRIVATE) {
        return Err(Errno::EINVAL);
    }
    if flags & (MAP_FIXED | __MAP_NOREPLACE) == __MAP_NOREPLACE {
        return Err(Errno::EINVAL);
    }
    if flags & MAP_STACK != 0 {
        if flags & (MAP_ANON | MAP_PRIVATE) != (MAP_ANON | MAP_PRIVATE) {
            return Err(Errno::EINVAL);
        }
        if flags & !(MAP_STACK | MAP_FIXED | MAP_ANON | MAP_PRIVATE) != 0 {
            return Err(Errno::EINVAL);
        }
        if a.pos != 0 {
            return Err(Errno::EINVAL);
        }
        if prot & (PROT_READ | PROT_WRITE) != (PROT_READ | PROT_WRITE) {
            return Err(Errno::EINVAL);
        }
    }
    if a.len == 0 {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// `sys_mmap`'s checks of a `MAP_FIXED` address (adjusted by the offset's page offset) and
/// the aligned size.
fn mmap_check_fixed(addr: usize, size: usize) -> Result<(), Errno> {
    if addr & PAGE_MASK != 0 {
        return Err(Errno::EINVAL); // not page aligned
    }
    if addr > usize::MAX - size {
        return Err(Errno::EINVAL); // no wrapping!
    }
    if VM_MAXUSER_ADDRESS > 0 && addr + size > VM_MAXUSER_ADDRESS {
        return Err(Errno::EINVAL);
    }
    if VM_MIN_ADDRESS > 0 && addr < VM_MIN_ADDRESS {
        return Err(Errno::EINVAL);
    }
    Ok(())
}

/// The file half of `sys_mmap`, between `fd_getfile` and `FRELE` (run under the kernel
/// lock): checks the vnode, settles the sharing type and `maxprot`, and maps through
/// `uvm_mmapfile`. `pos` and `size` are page aligned; `fflag` is the file's `f_flag`.
#[allow(clippy::too_many_arguments)] // the C's locals the two halves share
fn mmap_vnode<G: ProcGlue>(
    p: &Thread<G>,
    vm: &Vmspace,
    vp: &'static dyn VnodeOps,
    fflag: u32,
    addr: &mut usize,
    size: usize,
    prot: VmProt,
    flags: &mut i32,
    pos: Off,
) -> Result<VnodeMapping, Errno> {
    let vtype = vp.vtype(); // convert to vnode
    if vtype != VType::VREG && vtype != VType::VCHR && vtype != VType::VBLK {
        return Err(Errno::ENODEV); // only REG/CHR/BLK support mmap
    }

    // (pos + size) < pos, in the C's unsigned arithmetic.
    if vtype == VType::VREG && (pos as u64).wrapping_add(size as u64) < pos as u64 {
        return Err(Errno::EINVAL); // no offset wrapping
    }

    // special case: catch SunOS style /dev/zero
    if vtype == VType::VCHR && vp.is_zero_device() {
        *flags |= MAP_ANON;
        return Ok(VnodeMapping::ZeroDev);
    }

    // Old programs may not select a specific sharing type, so default to an appropriate one.
    if *flags & (MAP_SHARED | MAP_PRIVATE) == 0 {
        // DEBUG's "WARNING: defaulted mmap() share type" printf: not configured.
        if vtype == VType::VCHR {
            *flags |= MAP_SHARED; // for a device
        } else {
            *flags |= MAP_PRIVATE; // for a file
        }
    }

    // MAP_PRIVATE device mappings don't make sense (and aren't supported anyway). However,
    // some programs rely on this, so just change it to MAP_SHARED.
    if vtype == VType::VCHR && *flags & MAP_PRIVATE != 0 {
        *flags = (*flags & !MAP_PRIVATE) | MAP_SHARED;
    }

    // now check protection
    let maxprot = mmap_maxprot(fflag, prot, *flags, || vp.attr_flags())?;
    if *flags & __MAP_NOFAULT != 0 || (*flags & MAP_PRIVATE != 0 && prot & PROT_WRITE != 0) {
        data_limit_check(p, vm, size)?;
    }
    let locklimit = p.lim_cur(RLIMIT_MEMLOCK as usize) as usize;
    uvm_mmapfile(
        &vm.vm_map,
        addr,
        size,
        prot,
        maxprot,
        *flags,
        vp,
        pos,
        locklimit,
        Some(&p.p_ru),
    )?;
    Ok(VnodeMapping::Mapped)
}

/// `sys_mmap`'s protection check of a file mapping: the `maxprot` the file allows, or why
/// `prot` asks more (`EACCES`: the file is not open for it; `EPERM`: a shared writable
/// mapping of an immutable or append-only file). `va_flags` is `VOP_GETATTR`'s, asked only
/// for a shared mapping of a file open for writing, as the C asks it.
fn mmap_maxprot(
    fflag: u32,
    prot: VmProt,
    flags: i32,
    va_flags: impl FnOnce() -> Result<u64, Errno>,
) -> Result<VmProt, Errno> {
    let mut maxprot = PROT_EXEC;

    // check read access
    if fflag & FREAD != 0 {
        maxprot |= PROT_READ;
    } else if prot & PROT_READ != 0 {
        return Err(Errno::EACCES);
    }

    // check write access, shared case first
    if flags & MAP_SHARED != 0 {
        // if the file is writable, only add PROT_WRITE to maxprot if the file is not
        // immutable, append-only. otherwise, if we have asked for PROT_WRITE, return EPERM.
        if fflag & FWRITE != 0 {
            if va_flags()? & (IMMUTABLE | APPEND) == 0 {
                maxprot |= PROT_WRITE;
            } else if prot & PROT_WRITE != 0 {
                return Err(Errno::EPERM);
            }
        } else if prot & PROT_WRITE != 0 {
            return Err(Errno::EACCES);
        }
    } else {
        // MAP_PRIVATE mappings can always write to
        maxprot |= PROT_WRITE;
    }
    Ok(maxprot)
}

/// `sys_msync`: the msync system call (a front-end for flush).
pub fn sys_msync(vm: &Vmspace, addr: usize, size: usize, flags: i32) -> Result<(), Errno> {
    // sanity check flags
    if flags & !(MS_ASYNC | MS_SYNC | MS_INVALIDATE) != 0
        || flags & (MS_ASYNC | MS_SYNC | MS_INVALIDATE) == 0
        || flags & (MS_ASYNC | MS_SYNC) == (MS_ASYNC | MS_SYNC)
    {
        return Err(Errno::EINVAL);
    }
    // MS_SYNC is the default when neither is set (the C's `flags |= MS_SYNC`); both force a
    // synchronous flush below.

    // align the address to a page boundary, and adjust the size accordingly
    let (start, end) = align_range(addr, size)?;

    // translate MS_ flags into PGO_ flags
    let mut uvmflags = PGO_CLEANIT;
    if flags & MS_INVALIDATE != 0 {
        uvmflags |= PGO_FREE;
    }
    // MS_SYNC, and MS_ASYNC too: XXXCDC: force sync for now!
    uvmflags |= PGO_SYNCIO;

    uvm_map_clean(&vm.vm_map, start, end, uvmflags)
}

/// `sys_munmap`: unmap a users memory.
pub fn sys_munmap(vm: &Vmspace, addr: usize, size: usize) -> Result<(), Errno> {
    // align address to a page boundary, and adjust size accordingly
    let (addr, size, _) = align_addr(addr, size)?;

    // Check for illegal addresses. Watch out for address wrap...
    if addr > usize::MAX - size {
        return Err(Errno::EINVAL);
    }
    if VM_MAXUSER_ADDRESS > 0 && addr + size > VM_MAXUSER_ADDRESS {
        return Err(Errno::EINVAL);
    }
    if VM_MIN_ADDRESS > 0 && addr < VM_MIN_ADDRESS {
        return Err(Errno::EINVAL);
    }
    let map = &vm.vm_map;

    let mut dead_entries = DeadQueue::new();
    {
        let mut guard = map.lock(); // lock map so we can checkprot

        // interesting system call semantic: make sure entire range is allocated before
        // allowing an unmap.
        if !uvm_map_checkprot(&guard, addr, addr + size, PROT_NONE) {
            return Err(Errno::EINVAL);
        }

        if uvm_unmap_remove(
            &mut guard,
            addr,
            addr + size,
            &mut dead_entries,
            false,
            true,
            true,
        )
        .is_err()
        {
            return Err(Errno::EPERM); // immutable entries found
        }
    } // and unlock

    uvm_unmap_detach(&mut dead_entries, 0);

    Ok(())
}

/// `sys_mprotect`: the mprotect system call.
pub fn sys_mprotect<G: ProcGlue>(
    p: &Thread<G>,
    vm: &Vmspace,
    addr: usize,
    size: usize,
    prot: VmProt,
) -> Result<(), Errno> {
    if prot & PROT_MASK != prot {
        return Err(Errno::EINVAL);
    }
    if prot & (PROT_WRITE | PROT_EXEC) == (PROT_WRITE | PROT_EXEC) {
        uvm_wxcheck(p, "mprotect")?;
    }

    G::pledge_protexec(p, prot)?;

    // align the address to a page boundary, and adjust the size accordingly
    let (start, end) = align_range(addr, size)?;

    uvm_map_protect(p, &vm.vm_map, start, end, prot, 0, false, true)
}

/// `sys_pinsyscalls`. The caller is required to normalize base,len to the minimum .text
/// region, and adjust pintable offsets relative to that base.
pub fn sys_pinsyscalls<G: ProcGlue>(
    p: &Thread<G>,
    vm: &Vmspace,
    base: usize,
    len: usize,
    pins: UserPtr<u32>,
    npins: i32,
) -> Result<(), Errno> {
    let map = &vm.vm_map;

    // Must be called before any threads are created
    if p.has_sibling() {
        return Err(Errno::EPERM);
    }

    // Only allow libc syscall pinning once per process
    let map_flags = map.modflags(VM_MAP_PINSYSCALL_ONCE, 0);
    if map_flags & VM_MAP_PINSYSCALL_ONCE != 0 {
        return Err(Errno::EPERM);
    }

    if base > usize::MAX - len {
        return Err(Errno::EINVAL); // disallow wrap-around.
    }
    if base < map.min_offset() || base + len > map.max_offset() {
        return Err(Errno::EINVAL);
    }

    let n = pins_count(npins)?;
    let Some(mut table) = MArray::new(M_PINSYSCALL, M_WAITOK, n, |_| 0u32) else {
        return Err(Errno::ENOMEM);
    };
    // err: the table's drop is the C's free(pins, M_PINSYSCALL, npins * sizeof(u_int)).
    pins.copyin_slice(&mut table)?;

    // Range-check pintable offsets
    pins_check(&table, len)?;

    // pr->ps_libcpin = { base, base + len, pins, npins }
    drop(p.process().ps_libcpin.replace(crate::proc::PinTable {
        start: base,
        end: base + len,
        pins: Some(table),
    }));

    // PMAP_CHECK_COPYIN: not configured.
    Ok(())
}

/// `sys_pinsyscalls`' count of pins: at least one, at most `SYS_MAXSYSCALL` (`E2BIG`).
fn pins_count(npins: i32) -> Result<usize, Errno> {
    if npins < 1 || npins as usize > SYS_MAXSYSCALL {
        return Err(Errno::E2BIG);
    }
    Ok(npins as usize)
}

/// `sys_pinsyscalls`' range check of the offsets: each is 0 (no system call), `(u_int)-1`
/// (several locations) or within `len` (`ERANGE` otherwise).
fn pins_check(pins: &[u32], len: usize) -> Result<(), Errno> {
    for &pin in pins {
        if pin == u32::MAX || pin == 0 {
            continue;
        }
        if pin as usize > len {
            return Err(Errno::ERANGE);
        }
    }
    Ok(())
}

/// `sys_mimmutable`: the mimmutable system call.
pub fn sys_mimmutable(vm: &Vmspace, addr: usize, size: usize) -> Result<(), Errno> {
    // align the address to a page boundary, and adjust the size accordingly
    let (start, end) = align_range(addr, size)?;
    uvm_map_immutable(&vm.vm_map, start, end, true)
}

/// `sys_minherit`: the minherit system call.
pub fn sys_minherit(vm: &Vmspace, addr: usize, size: usize, inherit: i32) -> Result<(), Errno> {
    // align the address to a page boundary, and adjust the size accordingly
    let (start, end) = align_range(addr, size)?;
    uvm_map_inherit(&vm.vm_map, start, end, inherit)
}

/// `sys_madvise`: give advice about memory usage.
pub fn sys_madvise(vm: &Vmspace, addr: usize, size: usize, advice: i32) -> Result<(), Errno> {
    // align the address to a page boundary, and adjust the size accordingly
    let (start, end) = align_range(addr, size)?;

    let map = &vm.vm_map;
    match advice {
        MADV_NORMAL | MADV_RANDOM | MADV_SEQUENTIAL => uvm_map_advice(map, start, end, advice),

        // Activate all these pages, pre-faulting them in if necessary.
        //
        // XXX IMPLEMENT ME. Should invent a "weak" mode for uvm_fault() which would only do
        // the PGO_LOCKED pgo_get().
        MADV_WILLNEED => Ok(()),

        // Deactivate all these pages. We don't need them any more. We don't, however, toss
        // the data in the pages.
        MADV_DONTNEED => uvm_map_clean(map, start, end, PGO_DEACTIVATE),

        // These pages contain no valid data, and may be garbage-collected. Toss all
        // resources, including any swap space in use.
        MADV_FREE => uvm_map_clean(map, start, end, PGO_FREE),

        // XXXMRG What is this? I think it's: Ensure that we have allocated backing-store for
        // these pages. This is going to require changes to the page daemon, as it will free
        // swap space allocated to pages in core. There's also what to do for
        // device/file/anonymous memory.
        MADV_SPACEAVAIL => Err(Errno::EINVAL),

        _ => Err(Errno::EINVAL),
    }
}

/// `mlock(2)`'s and `uvm_mmaplock`'s limits: `EAGAIN` from `mlock` (`uvm_mmaplock` turns it
/// into `ENOMEM`) when `size` more wired bytes would pass the system's `wiredmax` or the
/// process's `locklimit` (0: none) beside what `wired` (the map's pmap) holds already.
fn wire_limits_ok(size: usize, wired: i64, locklimit: u64) -> bool {
    let sys_wired = i64::from(UVMEXP.wired.load(Ordering::Relaxed));
    let wiredmax = i64::from(UVMEXP.wiredmax.load(Ordering::Relaxed));
    if atop(size) as i64 + sys_wired > wiredmax {
        return false;
    }
    !(locklimit != 0 && (size + ptoa(wired.max(0) as usize)) as u64 > locklimit)
}

/// `sys_mlock`: memory lock.
pub fn sys_mlock<G: ProcGlue>(
    p: &Thread<G>,
    vm: &Vmspace,
    addr: usize,
    size: usize,
) -> Result<(), Errno> {
    // align address to a page boundary and adjust size accordingly
    let (start, end) = align_range(addr, size)?;
    let size = end - start;

    let map = &vm.vm_map;
    // pmap_wired_count exists on both machines: the limit is lim_cur(RLIMIT_MEMLOCK) itself
    // (an infinite one is a limit no wiring passes).
    let limit = p.lim_cur(RLIMIT_MEMLOCK as usize);
    if !wire_limits_ok(size, 0, 0)
        || (size + ptoa(map.pmap().wired_count().max(0) as usize)) as u64 > limit
    {
        return Err(Errno::EAGAIN);
    }

    uvm_map_pageable(map, start, end, false, Some(&p.p_ru)).map_err(|_| Errno::ENOMEM)
}

/// `sys_munlock`: unlock wired pages.
pub fn sys_munlock(vm: &Vmspace, addr: usize, size: usize) -> Result<(), Errno> {
    // align address to a page boundary, and adjust size accordingly
    let (start, end) = align_range(addr, size)?;

    // pmap_wired_count exists: no suser check. An unwiring takes no fault: no counters.
    uvm_map_pageable(&vm.vm_map, start, end, true, None).map_err(|_| Errno::ENOMEM)
}

/// `sys_mlockall`: lock all pages mapped into an address space.
pub fn sys_mlockall<G: ProcGlue>(p: &Thread<G>, vm: &Vmspace, flags: i32) -> Result<(), Errno> {
    if flags == 0 || flags & !(MCL_CURRENT | MCL_FUTURE) != 0 {
        return Err(Errno::EINVAL);
    }

    // pmap_wired_count exists: no suser check.
    let limit = p.lim_cur(RLIMIT_MEMLOCK as usize) as usize;
    match uvm_map_pageable_all(&vm.vm_map, flags, limit, Some(&p.p_ru)) {
        Err(e) if e != Errno::ENOMEM => Err(Errno::EAGAIN),
        r => r,
    }
}

/// `sys_munlockall`: unlock all pages mapped into an address space.
pub fn sys_munlockall(vm: &Vmspace) -> Result<(), Errno> {
    // An unwiring takes no fault: no counters.
    let _ = uvm_map_pageable_all(&vm.vm_map, 0, 0, None);
    Ok(())
}

/// `uvm_mmaplock`: common code for mmapanon and mmapfile to lock a mmaping; the wiring's
/// faults count on `ru` (the caller's `p_ru`).
pub fn uvm_mmaplock(
    map: &VmMap,
    addr: &mut usize,
    size: usize,
    prot: VmProt,
    locklimit: usize,
    ru: Option<&Rusage>,
) -> Result<(), Errno> {
    // POSIX 1003.1b -- if our address space was configured to lock all future mappings,
    // wire the one we just made.
    if prot == PROT_NONE {
        // No more work to do in this case.
        return Ok(());
    }

    let guard = map.lock();
    if map.flags() & VM_MAP_WIREFUTURE == 0 {
        return Ok(()); // the guard's drop: vm_map_unlock
    }

    kernel_lock(); // KERNEL_LOCK()
    if !wire_limits_ok(size, map.pmap().wired_count(), locklimit as u64) {
        drop(guard);
        // unmap the region!
        uvm_unmap(map, *addr, *addr + size);
        kernel_unlock(); // KERNEL_UNLOCK()
        return Err(Errno::ENOMEM);
    }
    // uvm_map_pageable_locked consumes the guard: the map is unlocked on return.
    if let Err(e) = uvm_map_pageable_locked(guard, *addr, *addr + size, false, ru) {
        // unmap the region!
        uvm_unmap(map, *addr, *addr + size);
        kernel_unlock(); // KERNEL_UNLOCK()
        return Err(e);
    }
    kernel_unlock(); // KERNEL_UNLOCK()
    Ok(())
}

/// The flags `uvm_mmapanon` and `uvm_mmapfile` share: a non-fixed address is rounded up, a
/// fixed one must be page aligned (`EINVAL`) and replaces what is there unless
/// `__MAP_NOREPLACE`.
fn mmap_fixed_flags(addr: &mut usize, flags: i32) -> Result<u32, Errno> {
    if flags & MAP_FIXED == 0 {
        *addr = round_page(*addr); // round
        return Ok(0);
    }
    if *addr & PAGE_MASK != 0 {
        return Err(Errno::EINVAL);
    }
    let mut uvmflag = UVM_FLAG_FIXED;
    if flags & __MAP_NOREPLACE == 0 {
        uvmflag |= UVM_FLAG_UNMAP;
    }
    Ok(uvmflag)
}

/// The inheritance of a mapping: shared mappings are shared with a child, private ones
/// copied.
fn mmap_inherit(flags: i32) -> i32 {
    if flags & MAP_SHARED != 0 {
        MAP_INHERIT_SHARE
    } else {
        MAP_INHERIT_COPY
    }
}

/// `uvm_mmapanon`: internal version of mmap for anons, used by `sys_mmap`; a wiring's
/// faults count on `ru` (`p->p_ru`).
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_mmapanon(
    map: &VmMap,
    addr: &mut usize,
    size: usize,
    prot: VmProt,
    maxprot: VmProt,
    flags: i32,
    locklimit: usize,
    ru: Option<&Rusage>,
) -> Result<(), Errno> {
    let advice = MADV_NORMAL;
    let mut align = 0; // userland page size

    // for non-fixed mappings, round off the suggested address. for fixed mappings, check
    // alignment and zap old mappings.
    let mut uvmflag = mmap_fixed_flags(addr, flags)?;

    let ldpgsz = machine().ldpgsz;
    if flags & MAP_FIXED == 0 && size >= ldpgsz {
        align = ldpgsz;
    }
    if flags & MAP_SHARED == 0 {
        uvmflag |= UVM_FLAG_COPYONW;
    } else {
        uvmflag |= UVM_FLAG_OVERLAY;
    }
    if flags & MAP_STACK != 0 {
        uvmflag |= UVM_FLAG_STACK;
    }
    if flags & MAP_CONCEAL != 0 {
        uvmflag |= UVM_FLAG_CONCEAL;
    }

    // set up mapping flags
    let uvmflag = uvm_mapflag(prot, maxprot, mmap_inherit(flags), advice, uvmflag);

    uvm_mapanon(map, addr, size, align, uvmflag)?;
    uvm_mmaplock(map, addr, size, prot, locklimit, ru)
}

/// `uvm_mmapfile`: internal version of mmap for non-anons, used by `sys_mmap`. The caller
/// must page-align the file offset.
#[allow(clippy::too_many_arguments)] // the C's signature
pub fn uvm_mmapfile(
    map: &VmMap,
    addr: &mut usize,
    size: usize,
    prot: VmProt,
    maxprot: VmProt,
    flags: i32,
    vp: &'static dyn VnodeOps,
    foff: Voff,
    locklimit: usize,
    ru: Option<&Rusage>,
) -> Result<(), Errno> {
    let advice = MADV_NORMAL;
    let align = 0; // userland page size

    // for non-fixed mappings, round off the suggested address. for fixed mappings, check
    // alignment and zap old mappings.
    let mut uvmflag = mmap_fixed_flags(addr, flags)?;

    // the access a private mapping asks of the object: never write.
    let accessprot = if flags & MAP_SHARED != 0 {
        maxprot
    } else {
        maxprot & !PROT_WRITE
    };

    // attach to underlying vm object.
    if vp.vtype() == VType::VCHR {
        // udv_attach(vp->v_rdev, accessprot, foff, size), retried with PROT_EXEC dropped
        // from maxprot for devices that refuse it, then advice = MADV_RANDOM: see the
        // module's deviations.
        return Err(unported!("uvm_mmapfile: udv_attach (uvm_device.c)"));
    }
    let uobj = uvn_attach(vp, accessprot);

    // XXXCDC: hack from old code: don't allow vnodes which have been mapped shared-writeable
    // to persist [forces them to be flushed out when last reference goes]. It also avoids a
    // deadlock between the uncache and a write of the same area of the file, and protects
    // from the "persistbug" program; not a long term solution. The uncache needs no
    // VOP_LOCK: the reference uvn_attach took keeps the uvn alive.
    if flags & MAP_SHARED != 0 && (prot & PROT_WRITE != 0 || maxprot & PROT_WRITE != 0) {
        uvm_vnp_uncache(vp);
    }

    let Some(uobj) = uobj else {
        return Err(if vp.vtype() == VType::VREG {
            Errno::ENOMEM
        } else {
            Errno::EINVAL
        });
    };

    if flags & MAP_SHARED == 0 {
        uvmflag |= UVM_FLAG_COPYONW;
    }
    if flags & __MAP_NOFAULT != 0 {
        uvmflag |= UVM_FLAG_NOFAULT | UVM_FLAG_OVERLAY;
    }
    if flags & MAP_STACK != 0 {
        uvmflag |= UVM_FLAG_STACK;
    }
    if flags & MAP_CONCEAL != 0 {
        uvmflag |= UVM_FLAG_CONCEAL;
    }

    // set up mapping flags
    let uvmflag = uvm_mapflag(prot, maxprot, mmap_inherit(flags), advice, uvmflag);

    match uvm_map(map, addr, size, Some(uobj.clone()), foff, align, uvmflag) {
        Ok(()) => uvm_mmaplock(map, addr, size, prot, locklimit, ru),
        Err(error) => {
            // errors: first detach from the uobj, if any.
            uobj.detach();
            Err(error)
        }
    }
}

/// `sys_kbind`'s parameter block: the bindings of the first `psize` bytes of `param`, each
/// with its bytes' offset, and the bytes' total; `EINVAL` when the sizes do not add up to
/// `psize`, a binding is empty, too large, or reaches `VM_MAXUSER_ADDRESS`. The C's loop
/// over `paramp[count]`.
pub fn kbind_blocks(
    param: &[u8; KBIND_PARAM_SIZE],
    psize: usize,
) -> Result<([KbindBlock; KBIND_BLOCK_MAX], usize), Errno> {
    let word = |at: usize| -> usize {
        let mut b = [0u8; size_of::<usize>()];
        b.copy_from_slice(&param[at..at + size_of::<usize>()]);
        usize::from_ne_bytes(b)
    };

    // The param argument points to an array of __kbind structures followed by the
    // corresponding new data areas for them. Verify that the sizes in the __kbind structures
    // add up to the total size and find the start of the new area.
    let mut blocks = [KbindBlock {
        addr: 0,
        size: 0,
        data: 0,
    }; KBIND_BLOCK_MAX];
    let mut s = psize;
    let mut count = 0;
    while s > 0 && count < KBIND_BLOCK_MAX {
        if s < KBSZ {
            return Err(Errno::EINVAL);
        }
        s -= KBSZ;

        let kb_addr = word(count * KBSZ);
        let kb_size = word(count * KBSZ + size_of::<usize>());
        let baseva = kb_addr;
        let endva = baseva.wrapping_add(kb_size).wrapping_sub(1);
        if kb_addr == 0
            || kb_size == 0
            || kb_size > KBIND_DATA_MAX
            || baseva >= VM_MAXUSER_ADDRESS
            || endva >= VM_MAXUSER_ADDRESS
            || s < kb_size
        {
            return Err(Errno::EINVAL);
        }

        s -= kb_size;
        blocks[count] = KbindBlock {
            addr: kb_addr,
            size: kb_size,
            data: 0,
        };
        count += 1;
    }
    if s > 0 {
        return Err(Errno::EINVAL);
    }
    let mut data = count * KBSZ;
    for b in blocks.iter_mut().take(count) {
        b.data = data;
        data += b.size;
    }
    Ok((blocks, count))
}

/// `sys_kbind`'s security check under `ps_mtx`: `paramp` (0 for `NULL`) and the cookie
/// against the process's slot, at the caller's `pc`. Initialises or disables the slot as the
/// C does; `true` when `SIGILL` is due.
fn kbind_check(slot: &mut KbindSlot, paramp: usize, pc: usize, cookie: u64) -> bool {
    if paramp == 0 {
        // ld.so disables kbind() when lazy binding is disabled
        if slot.addr == 0 {
            slot.addr = BOGO_PC;
            return false;
        }
        // pre-7.3 static binaries disable kbind
        // XXX delete check in 2026
        return slot.addr != BOGO_PC;
    }
    if slot.addr == 0 {
        slot.addr = pc;
        slot.cookie = cookie;
        return false;
    }
    pc != slot.addr || pc == BOGO_PC || slot.cookie != cookie
}

/// `sys_kbind`: the lazy-binding update of `ld.so`: writes the new PLT/GOT data into the
/// caller's (possibly read-only) text through a temporary kernel mapping.
pub fn sys_kbind<G: ProcGlue>(
    p: &Thread<G>,
    vm: &Vmspace,
    paramp: UserPtr<u8>,
    psize: usize,
    cookie: u64,
) -> Result<(), Errno> {
    let pr = p.process();
    let param_addr = paramp.addr();

    // If paramp is NULL and we're uninitialized, disable the syscall for the process. Raise
    // SIGILL if paramp is NULL and we're already initialized.
    //
    // If paramp is non-NULL and we're uninitialized, do initialization. Otherwise, do
    // security checks and raise SIGILL on failure.
    let pc = hal::trap::proc_pc(); // PROC_PC(p): p is the running thread
    let sigill = {
        let g_pr = pr.ps_mtx.lock(); // mtx_enter(&pr->ps_mtx)
        let mut slot = KbindSlot {
            addr: pr.ps_kbind_addr.get(),
            cookie: pr.ps_kbind_cookie.get() as u64,
        };
        let sigill = kbind_check(&mut slot, param_addr, pc, cookie);
        pr.ps_kbind_addr.set(slot.addr);
        pr.ps_kbind_cookie.set(slot.cookie as i64);
        drop(g_pr); // mtx_leave(&pr->ps_mtx)
        sigill
    };

    // Raise SIGILL if something is off.
    if sigill {
        kernel_lock(); // KERNEL_LOCK()
        crate::sig::sigexit(p, SIGILL);
    }

    // We're done if we were disabling the syscall.
    if param_addr == 0 {
        return Ok(());
    }

    if !(KBSZ..=KBIND_PARAM_SIZE).contains(&psize) {
        return Err(Errno::EINVAL);
    }
    let mut param = [0u8; KBIND_PARAM_SIZE];
    paramp.copyin_slice(&mut param[..psize])?;
    let (blocks, count) = kbind_blocks(&param, psize)?;

    // all looks good, so do the bindings
    let mut last_baseva = VM_MAXUSER_ADDRESS;
    let mut window = None;
    for kb in &blocks[..count] {
        let mut data = kb.data;
        let mut baseva = kb.addr;
        let mut s = kb.size;
        let mut pageoffset = baseva & PAGE_MASK;
        baseva = trunc_page(baseva);

        // hppa at least runs PLT entries over page edge
        let mut extra = (pageoffset + s) & PAGE_MASK;
        if extra > pageoffset {
            extra = 0;
        } else {
            s -= extra;
        }
        loop {
            // redo: make sure the desired page is mapped into kernel_map (the old page's
            // window, dropped first, unmaps it)
            if baseva != last_baseva {
                drop(window.take());
                window = Some(uvm_map_extract_window(
                    &vm.vm_map,
                    baseva,
                    PAGE_SIZE,
                    UVM_EXTRACT_FIXPROT,
                )?);
                last_baseva = baseva;
            }

            // do the update
            if let Some(w) = &window {
                w.window().write_at(pageoffset, &param[data..data + s])?;
            }
            data += s;

            if extra > 0 {
                baseva += PAGE_SIZE;
                s = extra;
                pageoffset = 0;
                extra = 0;
                continue;
            }
            break;
        }
    }

    // the last page's window: uvm_unmap_remove from kernel_map, uvm_unmap_detach.
    drop(window);
    Ok(())
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_addr_rounds_and_reports_the_offset() {
        assert_eq!(align_addr(0x1000, 0x10), Ok((0x1000, 0x1000, 0)));
        assert_eq!(align_addr(0x1234, 0x10), Ok((0x1000, 0x1000, 0x234)));
        assert_eq!(align_addr(0x1ff0, 0x20), Ok((0x1000, 0x2000, 0xff0)));
        assert_eq!(align_addr(0x1000, 0), Ok((0x1000, 0, 0)));
        assert_eq!(align_addr(0x1001, usize::MAX), Err(Errno::EINVAL));
        assert_eq!(align_addr(0, usize::MAX - 10), Err(Errno::EINVAL));
    }

    #[test]
    fn a_range_may_not_wrap() {
        assert_eq!(align_range(0x1234, 0x10), Ok((0x1000, 0x2000)));
        assert_eq!(
            align_range(usize::MAX & !PAGE_MASK, PAGE_SIZE),
            Err(Errno::EINVAL)
        );
    }

    fn args(addr: usize, len: usize, prot: VmProt, flags: i32, fd: i32, pos: Off) -> MmapArgs {
        MmapArgs {
            addr,
            len,
            prot,
            flags,
            fd,
            pos,
        }
    }

    /// `sys_mmap`'s validation, row by row in the C's order.
    #[test]
    fn mmap_refuses_what_the_c_refuses() {
        let rw = PROT_READ | PROT_WRITE;
        let anon = MAP_ANON | MAP_PRIVATE;
        assert_eq!(
            mmap_check(args(0, 1, 0x80, anon, -1, 0)),
            Err(Errno::EINVAL)
        );
        for (a, want) in [
            (args(0, 1, rw, 0x4000_0000, -1, 0), Err(Errno::EINVAL)), // unknown flag
            (args(0, 1, rw, anon | MAP_SHARED, -1, 0), Err(Errno::EINVAL)),
            (
                args(0, 1, rw, anon | __MAP_NOREPLACE, -1, 0),
                Err(Errno::EINVAL),
            ),
            (
                args(0, 1, rw, MAP_STACK | MAP_ANON, -1, 0),
                Err(Errno::EINVAL),
            ),
            (
                args(0, 1, rw, MAP_STACK | anon | MAP_CONCEAL, -1, 0),
                Err(Errno::EINVAL),
            ),
            (
                args(0, 1, rw, MAP_STACK | anon, -1, 4096),
                Err(Errno::EINVAL),
            ),
            (
                args(0, 1, PROT_READ, MAP_STACK | anon, -1, 0),
                Err(Errno::EINVAL),
            ),
            (args(0, 0, rw, anon, -1, 0), Err(Errno::EINVAL)), // size 0
            (args(0, 1, rw, MAP_STACK | anon | MAP_FIXED, -1, 0), Ok(())),
            (
                args(0, 1, rw, anon | MAP_FIXED | __MAP_NOREPLACE, -1, 0),
                Ok(()),
            ),
        ] {
            assert_eq!(mmap_check_flags(a), want, "{a:?}");
        }
        assert_eq!(mmap_check_fixed(0x1001, PAGE_SIZE), Err(Errno::EINVAL));
        assert_eq!(
            mmap_check_fixed(0, PAGE_SIZE),
            Err(Errno::EINVAL),
            "below VM_MIN_ADDRESS"
        );
        assert_eq!(
            mmap_check_fixed(VM_MAXUSER_ADDRESS, PAGE_SIZE),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            mmap_check_fixed(usize::MAX & !PAGE_MASK, 2 * PAGE_SIZE),
            Err(Errno::EINVAL)
        );
        assert_eq!(mmap_check_fixed(0x10_0000, PAGE_SIZE), Ok(()));
    }

    /// mquery's `getvnode` on a thread's own descriptors: no file `EBADF`, not a vnode
    /// `EINVAL`, a vnode `vgone` cleaned out `EBADF`, a vnode the file; every refusal gives
    /// its reference back (`FRELE`), so each file is held by its slot alone afterwards.
    #[test]
    fn mquery_takes_a_vnode_as_getvnode_does() {
        use crate::event::tests::open_data;
        use crate::testutil::{self, TestFileData};
        let p = crate::vm::map::tests::thread(91);
        let good: &'static dyn VnodeOps = testutil::mem_vnode(std::vec![0; 16]);
        let bad = testutil::mem_vnode(std::vec![0; 16]);
        bad.set_vtype(VType::VBAD);
        let vnode = |vp: &'static dyn VnodeOps| TestFileData {
            vnode: Some(vp),
            ..TestFileData::default()
        };
        let plain = open_data(p, crate::file::DTYPE_PIPE, TestFileData::default());
        let gfd = open_data(p, DTYPE_VNODE, vnode(good));
        let bfd = open_data(p, DTYPE_VNODE, vnode(bad));
        let check = |fd| getvnode(p, fd).map(|fp| fp.frele(Some(p)).unwrap());
        assert_eq!(check(gfd + 10), Err(Errno::EBADF), "no such descriptor");
        assert_eq!(check(plain), Err(Errno::EINVAL), "not a vnode");
        assert_eq!(check(gfd), Ok(()));
        assert_eq!(check(bfd), Err(Errno::EBADF), "VBAD");
        for fd in [plain, gfd, bfd] {
            let fp = fd_getfile(p.fd(), fd).unwrap();
            assert_eq!(fp.f_count(), 2, "descriptor {fd}: the slot's and this one");
            fp.frele(Some(p)).unwrap();
        }
    }

    /// `sys_mmap` and `sys_mprotect` ask in the C's order: the protection's bits, then
    /// `uvm_wxcheck` for a writable and executable request (the text's mount `wxallowed`
    /// and `PSI_WXNEEDED`, both, or `ENOTSUP`), then the flags, then `pledge_protexec` for
    /// any `PROT_EXEC` (the glue's answer, `kern_pledge` in `bsd`); the limits are the
    /// thread's own.
    #[test]
    fn the_w_x_check_and_pledge_are_asked_in_the_cs_order() {
        use crate::testutil;
        let _g = crate::vm::map::tests::setup();
        let vm = crate::vm::map::tests::user_vmspace();
        let p = crate::vm::map::tests::thread(92);
        let pr = p.process();
        let rw = PROT_READ | PROT_WRITE;
        let rx = PROT_READ | PROT_EXEC;
        let rwx = rw | PROT_EXEC;
        let anon = MAP_ANON | MAP_PRIVATE;
        let mmap = |prot, flags| sys_mmap(p, &vm, args(0, PAGE_SIZE, prot, flags, -1, 0));

        testutil::set_pledge_protexec(true);
        testutil::set_textvp_wxallowed(false);
        assert_eq!(mmap(rwx, anon), Err(Errno::ENOTSUP), "W^X before pledge");
        assert_eq!(
            mmap(rwx, 0x4000_0000),
            Err(Errno::ENOTSUP),
            "W^X before the flags"
        );
        testutil::set_textvp_wxallowed(true);
        assert_eq!(
            mmap(rwx, anon),
            Err(Errno::ENOTSUP),
            "a wxallowed mount alone"
        );
        pr.ps_iflags.set(pr.ps_iflags.get() | PSI_WXNEEDED);
        assert_eq!(mmap(rwx, 0x4000_0000), Err(Errno::EINVAL), "then the flags");
        assert_eq!(mmap(rwx, anon), Err(Errno::EPERM), "then pledge");
        assert_eq!(mmap(rx, anon), Err(Errno::EPERM), "any PROT_EXEC");
        pr.ps_iflags.set(pr.ps_iflags.get() & !PSI_WXNEEDED);
        testutil::set_textvp_wxallowed(false);
        let a = mmap(rw, anon).expect("no PROT_EXEC: pledge allows it");

        assert_eq!(
            sys_mprotect(p, &vm, a, PAGE_SIZE, rwx),
            Err(Errno::ENOTSUP),
            "mprotect's W^X"
        );
        assert_eq!(
            sys_mprotect(p, &vm, a, PAGE_SIZE, rx),
            Err(Errno::EPERM),
            "mprotect's pledge"
        );
        testutil::set_pledge_protexec(false);
        assert_eq!(sys_mprotect(p, &vm, a, PAGE_SIZE, rx), Ok(()));
        assert_eq!(
            pr.ps_wxcounter.get(),
            0,
            "kern.wxabort is 0: nothing counted"
        );

        // RLIMIT_DATA is the thread's: one page fits, then none does.
        let mut l = crate::resource::Plimit::new();
        l.pl_rlimit[RLIMIT_DATA as usize].rlim_cur = PAGE_SIZE as u64;
        drop(pr.set_limit(crate::resource::PlimitRef::new(l)));
        assert_eq!(mmap(rw, anon), Err(Errno::ENOMEM), "past the data limit");
        let _ = super::super::map::uvmspace_free(vm);
    }

    /// `kbind(2)`'s check-and-set runs on the thread's process under `ps_mtx`: the first
    /// call records the call site and the cookie, a call with another cookie is `SIGILL`
    /// (not exercised: `sigexit` ends the thread), a `NULL` block on a fresh process
    /// disables it.
    #[test]
    fn kbind_records_its_caller_in_the_process() {
        let _g = crate::vm::map::tests::setup();
        let vm = crate::vm::map::tests::user_vmspace();
        let p = crate::vm::map::tests::thread(94);
        let pr = p.process();
        hal::trap::set_user_regs(0x4_1000, 0);
        // A NULL block on a fresh process disables kbind.
        assert_eq!(sys_kbind(p, &vm, UserPtr::new(0), 0, 0), Ok(()));
        assert_eq!(pr.ps_kbind_addr.get(), BOGO_PC);
        // A second process: its first non-NULL call records the pc and the cookie, and a
        // bad size is EINVAL after the check.
        let q = crate::event::tests::thread_of(95);
        let qr = q.process();
        assert_eq!(
            sys_kbind(q, &vm, UserPtr::new(testutil_user(0x40)), 1, 77),
            Err(Errno::EINVAL)
        );
        assert_eq!(
            (qr.ps_kbind_addr.get(), qr.ps_kbind_cookie.get()),
            (0x4_1000, 77)
        );
        hal::trap::set_user_regs(0, 0);
        let _ = super::super::map::uvmspace_free(vm);
    }

    /// An address in the user-copy double's memory.
    fn testutil_user(off: usize) -> usize {
        crate::testutil::USER_BASE + 0x4000 + off
    }

    #[test]
    fn a_file_allows_what_it_was_opened_for() {
        let none = || -> Result<u64, Errno> { Ok(0) };
        let immutable = || -> Result<u64, Errno> { Ok(IMMUTABLE) };
        let rx = PROT_READ | PROT_EXEC;
        // read-only file, private: write allowed (copy on write), read and exec.
        assert_eq!(
            mmap_maxprot(FREAD, PROT_READ, MAP_PRIVATE, none),
            Ok(rx | PROT_WRITE)
        );
        // not opened for reading.
        assert_eq!(
            mmap_maxprot(FWRITE, PROT_READ, MAP_PRIVATE, none),
            Err(Errno::EACCES)
        );
        // shared write of a read-only file.
        assert_eq!(
            mmap_maxprot(FREAD, PROT_WRITE, MAP_SHARED, none),
            Err(Errno::EACCES)
        );
        // shared write of an immutable file open for writing.
        assert_eq!(
            mmap_maxprot(FREAD | FWRITE, PROT_WRITE, MAP_SHARED, immutable),
            Err(Errno::EPERM)
        );
        // shared read of it: maxprot without write.
        assert_eq!(
            mmap_maxprot(FREAD | FWRITE, PROT_READ, MAP_SHARED, immutable),
            Ok(rx)
        );
        assert_eq!(
            mmap_maxprot(FREAD | FWRITE, PROT_READ, MAP_SHARED, none),
            Ok(rx | PROT_WRITE)
        );
        // VOP_GETATTR's error.
        assert_eq!(
            mmap_maxprot(FREAD | FWRITE, PROT_READ, MAP_SHARED, || Err(Errno::EIO)),
            Err(Errno::EIO)
        );
    }

    #[test]
    fn the_pin_table_is_bounded() {
        assert_eq!(pins_count(0), Err(Errno::E2BIG));
        assert_eq!(pins_count(-1), Err(Errno::E2BIG));
        assert_eq!(pins_count(SYS_MAXSYSCALL as i32 + 1), Err(Errno::E2BIG));
        assert_eq!(pins_count(SYS_MAXSYSCALL as i32), Ok(SYS_MAXSYSCALL));
        assert_eq!(pins_check(&[0, u32::MAX, 0x100], 0x100), Ok(()));
        assert_eq!(pins_check(&[0, 0x101], 0x100), Err(Errno::ERANGE));
    }

    fn param(blocks: &[(usize, usize)], data: usize) -> ([u8; KBIND_PARAM_SIZE], usize) {
        let mut p = [0u8; KBIND_PARAM_SIZE];
        for (i, &(a, s)) in blocks.iter().enumerate() {
            p[i * KBSZ..i * KBSZ + 8].copy_from_slice(&a.to_ne_bytes());
            p[i * KBSZ + 8..i * KBSZ + 16].copy_from_slice(&s.to_ne_bytes());
        }
        (p, blocks.len() * KBSZ + data)
    }

    /// kbind's size table: the parameter block's sizes must add up exactly.
    #[test]
    fn kbind_blocks_add_up() {
        let (p, n) = param(&[(0x1000, 8)], 8);
        let (b, count) = kbind_blocks(&p, n).unwrap();
        assert_eq!(count, 1);
        assert_eq!(
            b[0],
            KbindBlock {
                addr: 0x1000,
                size: 8,
                data: 16
            }
        );
        let (p, n) = param(&[(0x1000, 8), (0x2ffc, 8)], 16);
        let (b, count) = kbind_blocks(&p, n).unwrap();
        assert_eq!(count, 2);
        assert_eq!(b[1].data, 2 * KBSZ + 8);
        for (blocks, data, why) in [
            (&[(0x1000usize, 8usize)][..], 7, "short data"),
            (&[(0x1000, 8)][..], 9, "data left over"),
            (&[(0, 8)][..], 8, "NULL address"),
            (&[(0x1000, 0)][..], 0, "empty"),
            (&[(0x1000, KBIND_DATA_MAX + 1)][..], 0, "too large"),
            (&[(VM_MAXUSER_ADDRESS, 8)][..], 8, "at the top"),
            (&[(VM_MAXUSER_ADDRESS - 4, 8)][..], 8, "across the top"),
        ] {
            let (p, n) = param(blocks, data);
            if n <= KBIND_PARAM_SIZE {
                assert_eq!(kbind_blocks(&p, n), Err(Errno::EINVAL), "{why}");
            }
        }
        // a size that is not a whole block.
        let (p, _) = param(&[(0x1000, 8)], 8);
        assert_eq!(kbind_blocks(&p, KBSZ - 1), Err(Errno::EINVAL));
    }

    /// kbind's check-and-set of the slot, the C's four cases.
    #[test]
    fn kbind_checks_its_caller() {
        let mut s = KbindSlot::default();
        assert!(
            !kbind_check(&mut s, 0, 0x500, 0),
            "disable an uninitialised slot"
        );
        assert_eq!(s.addr, BOGO_PC);
        assert!(!kbind_check(&mut s, 0, 0x500, 0), "disable again: fine");
        assert!(
            kbind_check(&mut s, 0x9000, BOGO_PC, 0),
            "disabled, then used"
        );
        let mut s = KbindSlot::default();
        assert!(
            !kbind_check(&mut s, 0x9000, 0x500, 7),
            "first use initialises"
        );
        assert_eq!(
            s,
            KbindSlot {
                addr: 0x500,
                cookie: 7
            }
        );
        assert!(!kbind_check(&mut s, 0x9000, 0x500, 7));
        assert!(kbind_check(&mut s, 0x9000, 0x504, 7), "another call site");
        assert!(kbind_check(&mut s, 0x9000, 0x500, 8), "another cookie");
        assert!(
            kbind_check(&mut s, 0, 0x500, 7),
            "NULL once initialised (pre-7.3 static)"
        );
    }

    #[test]
    fn the_wire_limits_are_the_systems_and_the_processs() {
        let wiredmax = UVMEXP.wiredmax.load(Ordering::Relaxed);
        let wired = UVMEXP.wired.load(Ordering::Relaxed);
        if wiredmax > wired + 4 {
            assert!(wire_limits_ok(PAGE_SIZE, 0, 0));
            assert!(
                !wire_limits_ok(PAGE_SIZE, 1, PAGE_SIZE as u64),
                "the lock limit"
            );
            assert!(wire_limits_ok(PAGE_SIZE, 1, 2 * PAGE_SIZE as u64));
        }
        let over = ptoa((wiredmax - wired + 1).max(1) as usize);
        assert!(!wire_limits_ok(over, 0, 0), "wiredmax");
    }
}
/* </TESTS> */
