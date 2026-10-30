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
//! The numbers the virtual memory system names when it asks about the thread it was given
//! (`RLIMIT_*`, `LOG_NOTICE`, `BOGO_PC`) and `sys_kbind`'s view of the process's kbind words.
//! N3.5 S1 made this the `VmProc` bridge (bridges.toml), `curproc` as `uvm` asked about it
//! while `struct proc` was `bsd`'s; since N5.0 the system calls, the fault and the map take
//! the thread (`&Thread<G>`, the C's `struct proc *p`) and the counters' record
//! (`Option<&Rusage>`, `curproc->p_ru`) and read its fields themselves, and the three calls
//! that are not the kernel's yet are `ProcGlue::pledge_protexec` and
//! `FileGlue::vnode_ops` (the text vnode is the process's `textvp` since N5.3). Nothing is
//! registered here.

/// `RLIMIT_DATA`: data size (`<sys/resource.h>`).
pub const RLIMIT_DATA: i32 = 2;
/// `RLIMIT_STACK`: stack size.
pub const RLIMIT_STACK: i32 = 3;
/// `RLIMIT_RSS`: resident set size.
pub const RLIMIT_RSS: i32 = 5;
/// `RLIMIT_MEMLOCK`: locked-in-memory address space.
pub const RLIMIT_MEMLOCK: i32 = 6;

/// `LOG_NOTICE`: normal but significant condition (`<sys/syslog.h>`).
pub const LOG_NOTICE: i32 = 5;

/// `BOGO_PC`: a program counter no thread can have (`<sys/proc.h>`, `(u_long)-1`).
pub const BOGO_PC: usize = usize::MAX;

/// `ps_kbind_addr` and `ps_kbind_cookie` of a process, as `sys_kbind` reads and sets them in
/// one section of `ps_mtx`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KbindSlot {
    /// `ps_kbind_addr`: the call site of the first `kbind(2)`, 0 before it, [`BOGO_PC`] when
    /// disabled.
    pub addr: usize,
    /// `ps_kbind_cookie`.
    pub cookie: u64,
}
/* </CODE> */
