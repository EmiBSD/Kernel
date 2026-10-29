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
 * from: Utah $Hdr: vm_unix.c 1.1 89/11/07$
 *      @(#)vm_unix.c   8.1 (Berkeley) 6/11/93
 * from: Id: uvm_unix.c,v 1.1.2.2 1997/08/25 18:52:30 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
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
 *	@(#)vm_glue.c	8.6 (Berkeley) 1/5/94
 * from: Id: uvm_glue.c,v 1.1.2.8 1998/02/07 01:16:54 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1982, 1986, 1989, 1993
 *      The Regents of the University of California.
 *
 * All rights reserved.
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
 *      @(#)vm_meter.c  8.4 (Berkeley) 1/4/94
 * from: Id: uvm_meter.c,v 1.1.2.1 1997/08/14 19:10:35 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_init.c,v 1.1.2.3 1998/02/06 05:15:27 chs Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
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
 *	@(#)vm_page.h   7.3 (Berkeley) 4/21/91
 *	@(#)vm_page.c   8.3 (Berkeley) 3/21/94
 * from: Id: uvm_page.h,v 1.1.2.6 1998/02/04 02:31:42 chuck Exp
 * from: Id: uvm_page.c,v 1.1.2.18 1998/02/06 05:24:42 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */

/*
 *
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed by Charles D. Cranor and
 *      Washington University.
 * 4. The name of the author may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * from: Id: uvm_vnode.h,v 1.1.2.4 1997/10/03 21:18:24 chuck Exp
 */
/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993
 *      The Regents of the University of California.
 * Copyright (c) 1990 University of Utah.
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
 *      @(#)vnode_pager.c       8.8 (Berkeley) 2/13/94
 * from: Id: uvm_vnode.c,v 1.1.2.26 1998/02/02 20:38:07 chuck Exp
 */

/*
 * Copyright (c) 1997 Charles D. Cranor and Washington University.
 * Copyright (c) 1991, 1993, The Regents of the University of California.
 *
 * All rights reserved.
 *
 * This code is derived from software contributed to Berkeley by
 * The Mach Operating System project at Carnegie-Mellon University.
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
 *	@(#)vm_kern.c   8.3 (Berkeley) 1/12/94
 * from: Id: uvm_km.c,v 1.1.2.14 1998/02/06 05:19:27 chs Exp
 *
 *
 * Copyright (c) 1987, 1990 Carnegie-Mellon University.
 * All rights reserved.
 *
 * Permission to use, copy, modify and distribute this software and
 * its documentation is hereby granted, provided that both the copyright
 * notice and this permission notice appear in all copies of the
 * software, derivative works or modified versions, and any portions
 * thereof, and that both notices appear in supporting documentation.
 *
 * CARNEGIE MELLON ALLOWS FREE USE OF THIS SOFTWARE IN ITS "AS IS"
 * CONDITION.  CARNEGIE MELLON DISCLAIMS ANY LIABILITY OF ANY KIND
 * FOR ANY DAMAGES WHATSOEVER RESULTING FROM THE USE OF THIS SOFTWARE.
 *
 * Carnegie Mellon requests users of this software to return to
 *
 *  Software Distribution Coordinator  or  Software.Distribution@CS.CMU.EDU
 *  School of Computer Science
 *  Carnegie Mellon University
 *  Pittsburgh PA 15213-3890
 *
 * any improvements or extensions that they make and grant Carnegie the
 * rights to redistribute these changes.
 */
/* </LICENSES> */

/* <CODE> */
//! `bsd`'s side of the kernel's virtual memory system (`kernel::vm`, N3.5): the system call
//! table's adapters (which pass the thread and its address space), `struct process`'s
//! address space hand-overs and the boot order, each a thin call into the kernel or one call
//! of `bsd`'s (since N5.0 nothing of `struct proc` is answered through a bridge here).
//!
//! Upstream: sys/uvm/uvm_mmap.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_unix.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_glue.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_meter.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_init.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_page.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_page.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_vnode.h @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_vnode.c @ 3ce1f3f79392
//! Upstream: sys/uvm/uvm_km.c @ 3ce1f3f79392
//! LZ: sys/uvm/uvm_mmap.rs@3c62ede68802
//! LZ: sys/uvm/uvm_unix.rs@3c62ede68802
//! LZ: sys/uvm/uvm_glue.rs@3c62ede68802
//! LZ: sys/uvm/uvm_meter.rs@3c62ede68802
//! LZ: sys/uvm/uvm_init.rs@3c62ede68802
//! LZ: sys/uvm/uvm_page.rs@3c62ede68802
//! LZ: sys/uvm/uvm_vnode.rs@3c62ede68802
//! LZ: sys/uvm/uvm_km.rs@3c62ede68802
//!
//! - N5.0: no bridge left here. `BsdVmProc` (`kernel::vm::proc::VmProc`, curproc's facts, one
//!   call of `bsd`'s each, N3.5 S1) is gone: the vm system calls and the fault take the thread
//!   the adapters below pass, and the three calls that are still `bsd`'s are the glues'
//!   (`ProcGlue::pledge_protexec`, `ProcGlue::textvp_wxallowed`, `FileGlue::vnode_ops`).
//!   N4.6b: `BsdVmSysctl`
//!   (`kernel::vm::meter::VmSysctl`) is gone: the sysctl helpers, `averunnable` and `allproc`
//!   are the kernel's, `nkmempages` the `Malloc` bridge's, and `CTL_VM` is dispatched by
//!   `kernel::sysctl`.
//! - The fifteen `sysent[]` entries of the vm system calls (`sys_mmap`, ..., `sys_obreak`):
//!   the `sysargs` unpacked and handed to `kernel::vm::mmap`/`unix` with the process's
//!   address space; and their `Native` entries for the ABI module (`native_mmap`, ...,
//!   [`VM_ENTRIES`], `KernelApi::vm`), which run `mi_syscall` on the same row (pipe's
//!   precedent, `pipe_glue.rs`), so the prologue (`refreshcreds`, the pinned syscalls,
//!   pledge, the kernel lock rule) is the `Legacy` path's.
//! - `uvm_init` in the C's order around the kernel's two steps, `uvm_purge`, `uvm_exit`,
//!   `uvm_init_limits`, `uvmspace_exec`'s install of a new vmspace and `fork1`'s
//!   share-or-copy (operations on `struct proc`, `struct process` and `struct plimit`), the
//!   u-area with the legacy pointer.
//! - What `sys/uvm/`'s adapters kept for `bsd` (N3.3 to N3.4e), now here: the page system's
//!   machine inputs (`uvm_page.c`: the segment policy and `uvm_page_physload`'s lines,
//!   `uvm_page_init` over the machine's constraints with `BsdBufCacheStats`, `uvm_pagealloc`
//!   with the `(obj, anon)` pair, `uvm_pagealloc_multi`, `uvm_shutdown`,
//!   `paddr_is_dma_reachable`, `uvm_page_physdump`, `uvm_pmr_print`); the vnode as the
//!   kernel's pager reaches it (`uvm_vnode.c`: `impl VnodeOps for Vnode`; N5.0: `BsdSysInfo` is
//!   gone, the pager reads `kernel::tc` and `kernel::reboot` itself) and the
//!   file systems' `uvn_attach`/`uvm_vnp_*` with `bsd`'s signatures; and `uvm_km.c`'s `bsd`
//!   shapes (`kp_dma*` over the machine's `dma_constraint`, `km_alloc`/`km_free` with the
//!   legacy pointer, `vm_min_kernel_address`).
//!
//! ## Deviations
//! - `uvm_page_physload` after `uvm_init` needs `km_alloc` for the segment's pages: the
//!   segment is ignored, with the C's lines, as the C does when that allocation fails (LZ's
//!   deviation, kept).
//! - On a machine without an MMU (`PMAP_NOMMU`, the host double) `uvm_pagealloc_multi` asks
//!   for one physical segment: the buffer cache reaches a buffer's pages through the direct
//!   map there (`vfs_biomem.rs`).
//! - `uvn_attach` of a block device needs the device switch (`bdevsw[].d_type`, `DIOCGPART`
//!   through `d_ioctl`, `conf.c`): reported by the kernel's pager, and the attach fails (NULL)
//!   as for a device that is not a disk (LZ's). `uvm_vnp_sync(NULL)` is `uvm_vnp_sync(None)`;
//!   `uvm_vnp_uncache` returns `bool` (LZ's). `UVM_PAGE_OWN` is not configured; the `DEBUG`
//!   printfs are compiled out as in C (LZ's).
//! - `sysctl_proc_args` (`kern_sysctl.rs`) answers `EINVAL` when the victim's vmspace is gone
//!   between its `PS_EXITING` check and the reference (`Process::vmspace_share` is `None`),
//!   where the C would read a freed pointer.
//!
//! ## Redesign
//! - N3.5 S1: written dead (nothing registered the bridges or named these entries until the
//!   flip). N3.5 S2, the flip: `sys/uvm/` is gone; this module is `bsd`'s only file of the
//!   subsystem, `forbid` and with no `unsafe`, every call a safe function of `bsd`, `hal` or
//!   `kernel`.
//! - The process's address space is the kernel's counted handle in the C pointer's slot
//!   (`Process::ps_vmspace`, a `Cell<Option<VmspaceRef>>` of a pointer's size: the
//!   `processpl` item does not grow); `fork1` sets it ([`uvmspace_fork_or_share`]), `exec`
//!   replaces it ([`uvmspace_exec`]), the reaper takes it ([`uvm_exit`]); the threads' copy
//!   `p_vmspace` stays the raw pointer until N4.2. A reader that keeps the vmspace while it
//!   reads it (`fill_kproc`, `kern.proc_args`) holds a handle of its own
//!   (`Process::vmspace_share`) and a reference (the C's `uvmspace_addref`), given back by
//!   `uvmspace_free`.
//! - `pinsyscalls(2)`'s table is the kernel's `MArray` handed to the process (its
//!   `ps_libcpin`, a `hal::slot::ReadMostly<kernel::proc::PinTable>` since N4.6a):
//!   `pin_check` borrows it, `exit` and `exec` replace it and it goes with its owner (the C's
//!   `free(.., M_PINSYSCALL, ..)`).
//! - `uvmexp` goes out to `vm.uvmexp` as the bytes `UvmexpCopy::to_ne_bytes` lays out (hal),
//!   so `bsd`'s `unsafe impl SysctlPlain for UvmexpCopy` is gone.

use core::ffi::c_void;
use core::fmt;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize, Ordering};

use hal::syscall::SysResult;
use hal::user::UserPtr;
use hal::vm::param::{atop, ptoa, round_page, trunc_page};
use hal::vm::uvmexp::UVMEXP;
use kernel::api::VmEntries;
use kernel::pipe::UioMove;
use kernel::vm::anon::Anon;
use kernel::vm::init::VmMachine;
use kernel::vm::km::{KmemDynMode, KmemPaMode, KmemVaMode, NO_CONSTRAINT, pa_mode};
use kernel::vm::map::{Vmspace as KVmspace, VmspaceRef, uvmspace_free, uvmspace_purge};
use kernel::vm::mmap::MmapArgs;
use kernel::vm::object::{ObjRef, Object};
use kernel::vm::page::{
    MAX_SEGS, Owner, PG_CLEAN, PG_DEV, PG_FAKE, Pglist, PhysloadError, SegPolicy, SegStrat, VmPage,
    VmPhysseg, boot_segments, page_init_done, uvm_pagealloc_pg, uvm_pglistalloc, vm_physmem,
};
use kernel::vm::pmemrange::BufCacheStats;
use kernel::vm::types::{VmProt, Voff};
use kernel::vm::vnode::{Uvn, VType, VnodeOps};

use crate::kern::init_sysent::SYSENT;
use crate::kern::kern_subr::uiomove;
use crate::kern::subr_prf::panic;
use crate::kern::vfs_subr::{vget, vref, vrele};
use crate::kern::vfs_vnops::vn_lock;
use crate::kern::vfs_vops::{VOP_GETATTR, VOP_READ, VOP_UNLOCK, VOP_WRITE};
use crate::machine::conf::iszerodev;
use crate::machine::cpu::curproc;
use crate::machine::exec::MachineExec;
use crate::machine::param::MachineParam;
use crate::machine::{Machine, Pmap, VmParam};
use crate::sys::errno::Errno;
use crate::sys::mount::Mount;
use crate::sys::param::{USPACE, USPACE_ALIGN};
use crate::sys::proc::ProcExt;
use crate::sys::proc::{Proc, Process};
use crate::sys::resource::{RLIMIT_DATA, RLIMIT_RSS, RLIMIT_STACK};
use crate::sys::resourcevar::Plimit;
use crate::sys::smr::smr_flush;
use crate::sys::syscall::{
    SYS_break, SYS_kbind, SYS_madvise, SYS_mimmutable, SYS_minherit, SYS_mlock, SYS_mlockall,
    SYS_mmap, SYS_mprotect, SYS_mquery, SYS_msync, SYS_munlock, SYS_munlockall, SYS_munmap,
    SYS_pinsyscalls,
};
use crate::sys::syscall_mi::mi_syscall;
use crate::sys::syscallargs::{
    SysKbindArgs, SysMadviseArgs, SysMimmutableArgs, SysMinheritArgs, SysMlockArgs,
    SysMlockallArgs, SysMmapArgs, SysMprotectArgs, SysMqueryArgs, SysMsyncArgs, SysMunlockArgs,
    SysMunmapArgs, SysObreakArgs, SysPinsyscallsArgs,
};
use crate::sys::systm::{SysArgs, kernel_assert_locked, kernel_assert_unlocked, sysargs};
use crate::sys::types::{Paddr, Vaddr};
use crate::sys::types::{Register, Rlim};
use crate::sys::uio::{Iovec, Uio, UioRw, UioSeg};
use crate::sys::vnode::{VBLK, VTEXT, Vattr, Vnode, Vtype};
use crate::{kassert, kprintf};

/// The machine's constants `kernel::vm` reads (`kernel::vm::init::VmMachine`), handed down
/// by [`uvm_init`].
pub static VM_MACHINE: VmMachine = VmMachine {
    ldpgsz: <Machine as MachineExec>::LDPGSZ,
    uspace: USPACE,
    uspace_align: USPACE_ALIGN,
    have_uspace_guard: <Machine as MachineParam>::HAVE_USPACE_GUARD,
    dma_constraint: <Machine as Pmap>::DMA_CONSTRAINT,
};

/// `KernelApi::vm`: the `Native` entries of the vm system calls.
pub const VM_ENTRIES: VmEntries = VmEntries {
    obreak: native_obreak,
    mmap: native_mmap,
    munmap: native_munmap,
    mprotect: native_mprotect,
    madvise: native_madvise,
    mquery: native_mquery,
    kbind: native_kbind,
    pinsyscalls: native_pinsyscalls,
    mimmutable: native_mimmutable,
    mlock: native_mlock,
    munlock: native_munlock,
    minherit: native_minherit,
    msync: native_msync,
    mlockall: native_mlockall,
    munlockall: native_munlockall,
};

/// The thread on this CPU; a system call or a fault always has one.
fn curp(who: &str) -> &'static Proc {
    match curproc() {
        Some(p) => p,
        None => panic(format_args!("{who}: no curproc")),
    }
}

/// `p->p_vmspace`, the kernel's address space.
fn vmspace_of(p: &Proc) -> &'static KVmspace {
    p.vmspace()
}

/// `vm = pr->ps_vmspace; pr->ps_vmspace = NULL`: the process's handle.
fn vmspace_take(pr: &Process) -> Option<VmspaceRef> {
    pr.set_vmspace(None)
}

/// A `struct uio` as `uvm_io` moves bytes through it (`kernel::pipe::UioMove`): `uiomove`
/// in the uio's own direction; no file, so never non-blocking.
pub struct VmUio<'a, 'b>(pub &'a mut Uio<'b>);

impl UioMove for VmUio<'_, '_> {
    fn resid(&self) -> usize {
        self.0.uio_resid
    }

    fn uiomove(&mut self, buf: &mut [u8]) -> Result<(), Errno> {
        uiomove(buf, self.0)
    }

    fn nonblock(&self) -> bool {
        false
    }

    fn is_write(&self) -> bool {
        self.0.uio_rw == UioRw::UIO_WRITE
    }

    fn set_resid(&mut self, resid: usize) {
        self.0.uio_resid = resid;
    }
}

/// `sysent[]`'s `mquery(2)`.
pub fn sys_mquery(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMqueryArgs = sysargs(v);
    let a = MmapArgs {
        addr: uap.addr.get() as usize,
        len: uap.len.get(),
        prot: uap.prot.get(),
        flags: uap.flags.get(),
        fd: uap.fd.get(),
        pos: uap.pos.get(),
    };
    retval[0] = kernel::vm::mmap::sys_mquery(p, vmspace_of(p), a)? as Register;
    Ok(())
}

/// `sysent[]`'s `mmap(2)`.
pub fn sys_mmap(p: &Proc, v: &SysArgs, retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMmapArgs = sysargs(v);
    let a = MmapArgs {
        addr: uap.addr.get() as usize,
        len: uap.len.get(),
        prot: uap.prot.get(),
        flags: uap.flags.get(),
        fd: uap.fd.get(),
        pos: uap.pos.get(),
    };
    retval[0] = kernel::vm::mmap::sys_mmap(p, vmspace_of(p), a)? as Register;
    Ok(())
}

/// `sysent[]`'s `msync(2)`.
pub fn sys_msync(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMsyncArgs = sysargs(v);
    kernel::vm::mmap::sys_msync(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.flags.get(),
    )
}

/// `sysent[]`'s `munmap(2)`.
pub fn sys_munmap(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMunmapArgs = sysargs(v);
    kernel::vm::mmap::sys_munmap(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `mprotect(2)`.
pub fn sys_mprotect(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMprotectArgs = sysargs(v);
    kernel::vm::mmap::sys_mprotect(
        p,
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.prot.get(),
    )
}

/// `sysent[]`'s `pinsyscalls(2)`.
pub fn sys_pinsyscalls(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysPinsyscallsArgs = sysargs(v);
    kernel::vm::mmap::sys_pinsyscalls(
        p,
        vmspace_of(p),
        uap.base.get() as usize,
        uap.len.get(),
        UserPtr::new(uap.pins.get() as usize),
        uap.npins.get(),
    )
}

/// `sysent[]`'s `mimmutable(2)`.
pub fn sys_mimmutable(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMimmutableArgs = sysargs(v);
    kernel::vm::mmap::sys_mimmutable(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `minherit(2)`.
pub fn sys_minherit(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMinheritArgs = sysargs(v);
    kernel::vm::mmap::sys_minherit(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.inherit.get(),
    )
}

/// `sysent[]`'s `madvise(2)`.
pub fn sys_madvise(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMadviseArgs = sysargs(v);
    kernel::vm::mmap::sys_madvise(
        vmspace_of(p),
        uap.addr.get() as usize,
        uap.len.get(),
        uap.behav.get(),
    )
}

/// `sysent[]`'s `mlock(2)`.
pub fn sys_mlock(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMlockArgs = sysargs(v);
    kernel::vm::mmap::sys_mlock(p, vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `munlock(2)`.
pub fn sys_munlock(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMunlockArgs = sysargs(v);
    kernel::vm::mmap::sys_munlock(vmspace_of(p), uap.addr.get() as usize, uap.len.get())
}

/// `sysent[]`'s `mlockall(2)`.
pub fn sys_mlockall(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysMlockallArgs = sysargs(v);
    kernel::vm::mmap::sys_mlockall(p, vmspace_of(p), uap.flags.get())
}

/// `sysent[]`'s `munlockall(2)`.
pub fn sys_munlockall(p: &Proc, _v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    kernel::vm::mmap::sys_munlockall(vmspace_of(p))
}

/// `sysent[]`'s `kbind(2)`.
pub fn sys_kbind(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysKbindArgs = sysargs(v);
    kernel::vm::mmap::sys_kbind(
        p,
        vmspace_of(p),
        UserPtr::new(uap.param.get() as usize),
        uap.psize.get(),
        uap.proc_cookie.get() as u64,
    )
}

/// `sysent[]`'s `break` (`sys_obreak`).
pub fn sys_obreak(p: &Proc, v: &SysArgs, _retval: &mut [Register; 2]) -> Result<(), Errno> {
    let uap: &SysObreakArgs = sysargs(v);
    kernel::vm::unix::sys_obreak(p, vmspace_of(p), uap.nsize.get() as usize)
}

/// A `Native` vm entry: the thread on this CPU runs `mi_syscall` on row `nr` of `sysent[]`
/// (which must be `want`'s, else `ENOSYS`) with `args`: the prologue and the row's adapter
/// above, as on the `Legacy` path. `rval[0]` back in the `Ok`.
fn native(nr: Register, want: i32, args: SysArgs) -> Result<Register, i32> {
    let p = curp("native vm syscall");
    if nr != want as Register {
        return Err(Errno::ENOSYS.as_i32());
    }
    let mut rval: [Register; 2] = [0, 0];
    mi_syscall(p, nr, &SYSENT[nr as usize], &args, &mut rval).map_err(Errno::as_i32)?;
    Ok(rval[0])
}

/// `KernelApi::vm.obreak`.
pub fn native_obreak(nr: Register, nsize: usize) -> SysResult {
    native(nr, SYS_break, [nsize as Register, 0, 0, 0, 0, 0]).map(|_| ())
}

/// `KernelApi::vm.mmap`.
pub fn native_mmap(
    nr: Register,
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    pos: i64,
) -> Result<usize, i32> {
    let args = [
        addr as Register,
        len as Register,
        prot as Register,
        flags as Register,
        fd as Register,
        pos as Register,
    ];
    native(nr, SYS_mmap, args).map(|a| a as usize)
}

/// `KernelApi::vm.mquery`.
pub fn native_mquery(
    nr: Register,
    addr: usize,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    pos: i64,
) -> Result<usize, i32> {
    let args = [
        addr as Register,
        len as Register,
        prot as Register,
        flags as Register,
        fd as Register,
        pos as Register,
    ];
    native(nr, SYS_mquery, args).map(|a| a as usize)
}

/// A range system call's `Native` entry: `(addr, len)` on row `want`.
fn native_range(nr: Register, want: i32, addr: usize, len: usize) -> SysResult {
    native(nr, want, [addr as Register, len as Register, 0, 0, 0, 0]).map(|_| ())
}

/// A range system call's `Native` entry with an `int`: `(addr, len, v)` on row `want`.
fn native_range_int(nr: Register, want: i32, addr: usize, len: usize, v: i32) -> SysResult {
    let args = [addr as Register, len as Register, v as Register, 0, 0, 0];
    native(nr, want, args).map(|_| ())
}

/// `KernelApi::vm.munmap`.
pub fn native_munmap(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_munmap, addr, len)
}

/// `KernelApi::vm.mprotect`.
pub fn native_mprotect(nr: Register, addr: usize, len: usize, prot: i32) -> SysResult {
    native_range_int(nr, SYS_mprotect, addr, len, prot)
}

/// `KernelApi::vm.madvise`.
pub fn native_madvise(nr: Register, addr: usize, len: usize, behav: i32) -> SysResult {
    native_range_int(nr, SYS_madvise, addr, len, behav)
}

/// `KernelApi::vm.kbind`.
pub fn native_kbind(nr: Register, param: usize, psize: usize, cookie: u64) -> SysResult {
    let args = [
        param as Register,
        psize as Register,
        cookie as Register,
        0,
        0,
        0,
    ];
    native(nr, SYS_kbind, args).map(|_| ())
}

/// `KernelApi::vm.pinsyscalls`.
pub fn native_pinsyscalls(
    nr: Register,
    base: usize,
    len: usize,
    pins: usize,
    npins: i32,
) -> SysResult {
    let args = [
        base as Register,
        len as Register,
        pins as Register,
        npins as Register,
        0,
        0,
    ];
    native(nr, SYS_pinsyscalls, args).map(|_| ())
}

/// `KernelApi::vm.mimmutable`.
pub fn native_mimmutable(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_mimmutable, addr, len)
}

/// `KernelApi::vm.mlock`.
pub fn native_mlock(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_mlock, addr, len)
}

/// `KernelApi::vm.munlock`.
pub fn native_munlock(nr: Register, addr: usize, len: usize) -> SysResult {
    native_range(nr, SYS_munlock, addr, len)
}

/// `KernelApi::vm.minherit`.
pub fn native_minherit(nr: Register, addr: usize, len: usize, inherit: i32) -> SysResult {
    native_range_int(nr, SYS_minherit, addr, len, inherit)
}

/// `KernelApi::vm.msync`.
pub fn native_msync(nr: Register, addr: usize, len: usize, flags: i32) -> SysResult {
    native_range_int(nr, SYS_msync, addr, len, flags)
}

/// `KernelApi::vm.mlockall`.
pub fn native_mlockall(nr: Register, flags: i32) -> SysResult {
    native(nr, SYS_mlockall, [flags as Register, 0, 0, 0, 0, 0]).map(|_| ())
}

/// `KernelApi::vm.munlockall`.
pub fn native_munlockall(nr: Register) -> SysResult {
    native(nr, SYS_munlockall, [0; 6]).map(|_| ())
}

/// `uvm_init`: init the VM system. Called from `kern/init_main.c`: the bridges and the
/// machine's constants first, the page system over the machine's constraints, the kernel's
/// first step, `pmap_init`, the kernel memory allocators, the kernel's last step (the C's
/// order).
pub fn uvm_init() {
    // Ensure that the hardware set the page size.
    if UVMEXP.pagesize.load(Ordering::Relaxed) == 0 {
        panic(format_args!("uvm_init: page size not set"));
    }

    // averunnable.fscale = FSCALE: M5.

    // The kernel's objects and vnode pager reach the kernel lock, the net lock, the clock
    // and the shutdown state through bridges (N3.4e); the map, the system calls and the
    // sysctls reach curproc and the sysctl helpers (N3.5): registered before the first use.
    crate::kern::hal_bridges::hal_bridges_register_vm();
    // The u-areas `fork1` gives its threads (hal::thread's E4 provider).
    let _ = hal::thread::register_uarea_memory(&kernel::vm::glue::UAREAS);
    kernel::vm::init::uvm_machine_init(&VM_MACHINE);

    // Init the page sub-system. This includes allocating the vm_page structures, and setting
    // up all the page queues (and locks). Available memory will be put in the "free" queue,
    // kvm_start and kvm_end will be set to the area of kernel virtual memory which is
    // available for general use.
    let mut kvm_start = Vaddr::new(0);
    let mut kvm_end = Vaddr::new(0);
    uvm_page_init(&mut kvm_start, &mut kvm_end);

    // The map sub-system, kernel_map and the kernel object, the fault's tuning.
    let base = VM_MIN_KERNEL_ADDRESS.load(Ordering::Relaxed);
    kernel::vm::init::uvm_init_early(Vaddr::new(base), (kvm_start, kvm_end));

    // Init the pmap module. The pmap module is free to allocate memory for its private use
    // (e.g. pvlists).
    crate::machine::pmap::pmap_init();

    // step 6: init uvm_km_page allocator memory.
    let _ = crate::unported!("uvm_km_page_init");

    // Make kernel memory allocators ready for use. After this call the malloc memory allocator
    // can be used.
    crate::kern::kern_malloc::kmeminit();
    crate::kern::hal_bridges::hal_bridges_heap_ready();

    // step 7.5: init the dma allocator, which is backed by pools.
    crate::kern::dma_alloc::dma_alloc_init();

    // The pagers, the amaps, the kernel's swap object, the anons, the best-fit switches.
    kernel::vm::init::uvm_init_late(crate::machine::intr::IPL_MPFLOOR);
}

/// `uvm_purge`: teardown a virtual address space. If multi-threaded, must be called by the
/// last thread of a process.
pub fn uvm_purge() {
    let p = curp("uvm_purge");
    let vm = vmspace_of(p);

    kernel_assert_unlocked();

    // __HAVE_PMAP_PURGE: arm64's; nothing on a machine without it.
    crate::machine::pmap::pmap_purge(p);
    uvmspace_purge(vm);
}

/// `uvm_exit`: exit a virtual address space.
pub fn uvm_exit(pr: &Process) {
    if let Some(vm) = vmspace_take(pr) {
        uvmspace_free(vm);
    }
}

/// `uvmspace_exec(p, start, end)`: the process wants to exec a new program. The kernel's
/// `uvmspace_exec` recycles a vmspace only this process uses; a shared one comes back new,
/// and is installed here as the C does (`pmap_deactivate`, `p->p_vmspace = pr->ps_vmspace =
/// nvm`, `pmap_activate`) before the old one's reference is dropped (`uvmspace_free`).
pub fn uvmspace_exec(p: &Proc, start: usize, end: usize) {
    let pr = p.process();
    // The kernel's tells a vmspace only this process holds by its vm_refcnt; it reads the
    // vmspace, not the process's handle, and the slot is changed only below, after it.
    if let Some(nvm) = kernel::vm::map::uvmspace_exec(p.vmspace(), start, end) {
        // install new vmspace and drop our ref to the old one.
        crate::machine::pmap::pmap_deactivate(p);
        // p->p_vmspace: the thread's handle and word; the old handle goes with the process's.
        let oldp = p.set_vmspace(Some(nvm.clone()));
        let ovm = pr.set_vmspace(Some(nvm));
        drop(oldp);
        crate::machine::pmap::pmap_activate(p);
        if let Some(ovm) = ovm {
            uvmspace_free(ovm);
        }
    }
}

/// `pr->ps_vmspace = uvmspace_share(parent)` or `uvmspace_fork(parent)` (`process_new`'s
/// `FORK_SHAREVM` choice): the child's handle, set before the child is reachable.
pub fn uvmspace_fork_or_share(parent: &Process, pr: &Process, share: bool) {
    let vm = if share {
        match parent.vmspace_share() {
            Some(vm) => vm,
            None => panic(format_args!("process_new: the parent has no vmspace")),
        }
    } else {
        match parent.vmspace_share() {
            Some(pvm) => kernel::vm::map::uvmspace_fork(&pvm),
            None => panic(format_args!("process_new: the parent has no vmspace")),
        }
    };
    let prev = pr.set_vmspace(Some(vm));
    // process_new's child had no address space before this.
    kassert!(prev.is_none());
}

/// `uvm_uarea_alloc`: a thread's u-area (`kernel::vm::glue::uvm_uarea_alloc`), as the
/// legacy pointer `thread_new` stores in `p_addr`.
pub fn uvm_uarea_alloc() -> Option<hal::thread::UArea> {
    hal::thread::UArea::alloc()
}

/// `uvm_uarea_free(p)`: frees `p`'s u-area (`kernel::vm::glue::uvm_uarea_free`) and clears
/// `p_addr`.
pub fn uvm_uarea_free(p: &Proc) {
    drop(p.take_uarea());
}

/// `uvm_init_limits`: init per-process VM limits, the kernel's numbers
/// (`kernel::vm::glue::default_limits`) on `limit0`.
pub fn uvm_init_limits(limit0: &mut Plimit) {
    let l = kernel::vm::glue::default_limits();
    let mut set = |which: usize, cur: Rlim, max: Option<Rlim>| {
        let r = &mut limit0.pl_rlimit[which];
        r.rlim_cur = cur;
        if let Some(m) = max {
            r.rlim_max = m;
        }
    };
    set(RLIMIT_STACK, l.stack.0 as Rlim, Some(l.stack.1 as Rlim));
    set(RLIMIT_DATA, l.data.0 as Rlim, Some(l.data.1 as Rlim));
    set(RLIMIT_RSS, l.rss_cur as Rlim, None);
}

/*
 * The page system's machine inputs (uvm_page.c, uvm_page.h): the kernel's page system
 * (kernel::vm::page) over the machine's segment policy and DMA constraints, and bsd's buffer
 * cache statistics.
 */

// Physical memory segment strategies: how vm_physmem[] is kept.

/// Random: put it at the end (if this is "1" then we revert to a "contig" case).
pub const VM_PSTRAT_RANDOM: i32 = 1;
/// Binary search (sorted by address).
pub const VM_PSTRAT_BSEARCH: i32 = 2;
/// Linear search (sorted by largest segment first).
pub const VM_PSTRAT_BIGFIRST: i32 = 3;

/// `VM_PHYSSEG_MAX` of the selected machine.
pub const VM_PHYSSEG_MAX: usize = <Machine as VmParam>::VM_PHYSSEG_MAX;
/// `VM_PHYSSEG_STRAT` of the selected machine.
pub const VM_PHYSSEG_STRAT: i32 = <Machine as VmParam>::VM_PHYSSEG_STRAT;

/// The machine's segment policy, as the kernel's page system takes it (an unknown strategy,
/// the C's "unknown physseg strategy selected!" panic, fails the build: see the end).
const SEG_POLICY: SegPolicy = SegPolicy {
    max: VM_PHYSSEG_MAX,
    strat: match VM_PHYSSEG_STRAT {
        VM_PSTRAT_BSEARCH => SegStrat::Bsearch,
        VM_PSTRAT_BIGFIRST => SegStrat::Bigfirst,
        _ => SegStrat::Random,
    },
};

/// The buffer cache's statistics of `<sys/mount.h>`, for the kernel's allocator (the
/// `BufCacheStats` bridge): one call each into the legacy function of the same name.
struct BsdBufCacheStats;

impl BufCacheStats for BsdBufCacheStats {
    fn bufpages_deficit(&self) -> i64 {
        crate::sys::mount::bufpages_deficit()
    }

    fn bufpages_inact(&self) -> i64 {
        crate::sys::mount::bufpages_inact()
    }
}

static BSD_BUFCACHE_STATS: BsdBufCacheStats = BsdBufCacheStats;

/// The C's `(obj, anon)` pair (at most one of them) as the kernel's [`Owner`].
pub fn owner<'a>(obj: Option<&'a Object>, anon: Option<&'a Anon>) -> Owner<'a> {
    kassert!(obj.is_none() || anon.is_none());
    match (obj, anon) {
        (Some(o), _) => Owner::Object(o),
        (None, Some(a)) => Owner::Anon(a),
        (None, None) => Owner::None,
    }
}

/// `PADDR_IS_DMA_REACHABLE(paddr)`.
pub fn paddr_is_dma_reachable(paddr: Paddr) -> bool {
    let c = <Machine as Pmap>::DMA_CONSTRAINT;
    c.ucr_low <= paddr && c.ucr_high > paddr
}

/// `uvm_page_init`: init the page system (the kernel's, with the machine's DMA constraints
/// and the buffer cache's statistics). Called from `uvm_init()`. Returns the range of kernel
/// virtual memory left after the boot allocations in `kvm_startp`/`kvm_endp`.
pub fn uvm_page_init(kvm_startp: &mut Vaddr, kvm_endp: &mut Vaddr) {
    // A second registration (a host test's next set-up) keeps the first: the same statics.
    let _ = kernel::vm::pmemrange::uvm_pmr_install_stats(&BSD_BUFCACHE_STATS);
    // The page system steals its boot memory through the pmap (the host double bootstraps it
    // here, as no boot path does).
    <Machine as Pmap>::pmap_bootstrap_lazy();
    kernel::vm::page::uvm_page_init(<Machine as Pmap>::UVM_MD_CONSTRAINTS);

    // pass up the values of virtual_space_start and virtual_space_end (what the last
    // pmap_steal_memory left) to the upper layers of the VM.
    let (start, end) = <Machine as Pmap>::pmap_steal_bounds();
    *kvm_startp = Vaddr::new(round_page(start.as_usize()));
    *kvm_endp = Vaddr::new(trunc_page(end.as_usize()));
}

/// `uvm_page_physload`: load physical memory into VM system. All args are PFs; all pages in
/// start/end get vm_page structures; areas marked by avail_start/avail_end get added to the
/// free page pool; we are limited to `VM_PHYSSEG_MAX` physical memory segments.
pub fn uvm_page_physload(
    start: usize,
    end: usize,
    avail_start: usize,
    avail_end: usize,
    flags: i32,
) {
    match kernel::vm::page::uvm_page_physload(start, end, avail_start, avail_end, flags, SEG_POLICY)
    {
        Ok(()) => {}
        Err(PhysloadError::Full { max }) => {
            kprintf!("uvm_page_physload: unable to load physical memory segment\n");
            kprintf!(
                "\t{} segments allocated, ignoring {:#x} -> {:#x}\n",
                max,
                start,
                end
            );
            kprintf!("\tincrease VM_PHYSSEG_MAX\n");
        }
        Err(PhysloadError::AfterInit) => {
            kprintf!("uvm_page_physload: can not malloc vm_page structs for segment\n");
            kprintf!("\tignoring {:#x} -> {:#x}\n", start, end);
        }
    }
}

/// `uvm_page_physdump`: prints the physical memory config (XXXCDC: TMP TMP TMP DEBUG DEBUG
/// DEBUG). Call from DDB.
pub fn uvm_page_physdump() {
    let mut boot = [VmPhysseg::EMPTY; MAX_SEGS];
    let segs: &[VmPhysseg] = if page_init_done() {
        vm_physmem()
    } else {
        let n = boot_segments(&mut boot);
        &boot[..n]
    };
    kprintf!(
        "uvm_page_physdump: physical memory config [segs={} of {}]:\n",
        segs.len(),
        VM_PHYSSEG_MAX
    );
    for seg in segs {
        kprintf!(
            "{:#x}->{:#x} [{:#x}->{:#x}]\n",
            seg.start,
            seg.end,
            seg.avail_start,
            seg.avail_end
        );
    }
    kprintf!("STRATEGY = ");
    match VM_PHYSSEG_STRAT {
        VM_PSTRAT_RANDOM => kprintf!("RANDOM\n"),
        VM_PSTRAT_BSEARCH => kprintf!("BSEARCH\n"),
        VM_PSTRAT_BIGFIRST => kprintf!("BIGFIRST\n"),
        _ => kprintf!("<<UNKNOWN>>!!!!\n"),
    };
}

/// `uvm_pmr_print` for ddb: the kernel allocator's ranges through `printf`.
pub fn uvm_pmr_print() {
    struct Printf;
    impl fmt::Write for Printf {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            kprintf!("{}", s);
            Ok(())
        }
    }
    kernel::vm::pmemrange::uvm_pmr_print(&mut Printf);
}

/// `uvm_shutdown`: what the VM does on the way down.
pub fn uvm_shutdown() {
    // UVM_SWAP_ENCRYPT: uvm_swap_finicrypt_all(): not configured.
    smr_flush();
}

/// `uvm_pagealloc`: allocate vm_page from a particular free list. Returns `None` if no pages
/// free; only one of obj or anon can be non-null; caller must activate/deactivate page if it
/// is not wired.
pub fn uvm_pagealloc(
    obj: Option<&Object>,
    off: Voff,
    anon: Option<&Anon>,
    flags: i32,
) -> Option<&'static VmPage> {
    kernel::vm::page::uvm_pagealloc(owner(obj, anon), off, flags)
}

/// `uvm_pagealloc_multi`: interface used by the buffer cache to allocate a buffer at a time.
pub fn uvm_pagealloc_multi(obj: &Object, off: Voff, size: usize, flags: i32) -> Result<(), Errno> {
    kassert!(obj.is_bufcache());
    kernel_assert_locked();
    let plist = Pglist::new();
    let r = uvm_pglistalloc(
        size,
        NO_CONSTRAINT.ucr_low,
        NO_CONSTRAINT.ucr_high,
        Paddr::new(0),
        Paddr::new(0),
        &plist,
        // No MMU (the host double): the buffer cache reaches the pages through the direct
        // map, so they must be one segment (see the module's deviations).
        if <Machine as Pmap>::PMAP_NOMMU {
            1
        } else {
            atop(round_page(size)) as i32
        },
        flags,
    );
    if r.is_ok() {
        let mut i = 0;
        while let Some(pg) = plist.pop_head() {
            pg.set_wire_count(1);
            pg.set_bits(PG_CLEAN | PG_FAKE);
            kassert!(pg.flags() & PG_DEV == 0);
            uvm_pagealloc_pg(pg, Owner::Object(obj), off + ptoa(i) as Voff);
            i += 1;
        }
    }
    r
}

const _: () = {
    // Process::ps_vmspace holds the counted handle where the C has the pointer, in the same
    // word: the processpl item (vmstat -m) does not grow.
    assert!(
        core::mem::size_of::<core::cell::Cell<Option<VmspaceRef>>>()
            == core::mem::size_of::<*const KVmspace>()
    );
    // The kernel's segment table holds the machine's.
    assert!(VM_PHYSSEG_MAX <= MAX_SEGS);
    // uvm_page_physload: unknown physseg strategy selected!
    assert!(matches!(
        VM_PHYSSEG_STRAT,
        VM_PSTRAT_RANDOM | VM_PSTRAT_BSEARCH | VM_PSTRAT_BIGFIRST
    ));
};

/*
 * The vnode as the kernel's vnode pager reaches it (uvm_vnode.c, bridges.toml `VnodeOps`),
 * and the file systems' calls into the pager with bsd's signatures.
 */

/// `VOP_READ`/`VOP_WRITE` of `buf` at `off` (`UIO_SYSSPACE`, `curproc`'s credentials): the
/// bytes moved (`len - uio_resid`).
fn vop_rw(
    vp: &'static Vnode,
    rw: UioRw,
    off: Voff,
    base: *mut c_void,
    len: usize,
    ioflag: i32,
) -> Result<usize, Errno> {
    let mut iov = [Iovec {
        iov_base: base,
        iov_len: len,
    }];
    let p = curproc();
    let mut uio = Uio {
        uio_iov: &mut iov,
        uio_offset: off,
        uio_segflg: UioSeg::UIO_SYSSPACE,
        uio_rw: rw,
        uio_resid: len,
        uio_procp: p,
    };
    let cred = p.map_or(core::ptr::null(), |p| p.ucred_ptr());
    if rw == UioRw::UIO_READ {
        VOP_READ(vp, &mut uio, ioflag, cred)?;
    } else {
        VOP_WRITE(vp, &mut uio, ioflag, cred)?;
    }
    Ok(len - uio.uio_resid)
}

// The vnode as the vnode pager reaches it (bridges.toml `VnodeOps`): one call each into the
// vnode layer. Vnodes are never freed, so the pager keeps them as `&'static`.
impl VnodeOps for Vnode {
    fn uvn(&self) -> Option<&'static Uvn> {
        self.v_uvm.get()
    }

    fn set_uvn(&self, uvn: &'static Uvn) {
        self.v_uvm.set(Some(uvn));
    }

    fn attr_size(&'static self) -> Result<u64, Errno> {
        let Some(p) = curproc() else {
            panic(format_args!("uvn_attach: no curproc"));
        };
        let mut vattr = Vattr::new();
        VOP_GETATTR(self, &mut vattr, p.ucred_ptr(), p)?;
        Ok(vattr.va_size)
    }

    fn is_blk(&self) -> bool {
        self.v_type.get() == VBLK
    }

    fn vtext(&self) -> bool {
        self.v_flag.get() & VTEXT != 0
    }

    fn clear_vtext(&self) {
        self.v_flag.set(self.v_flag.get() & !VTEXT);
    }

    fn mount_id(&self) -> usize {
        mount_id(self.v_mount.get())
    }

    fn vref(&'static self) {
        vref(self);
    }

    fn vrele(&'static self) {
        let _ = vrele(self);
    }

    fn vget_nowait(&'static self) -> Result<(), Errno> {
        vget(self, crate::sys::lock::LK_NOWAIT)
    }

    fn lock(&'static self, flags: i32) -> Result<(), Errno> {
        vn_lock(self, flags)
    }

    fn unlock(&'static self) {
        let _ = VOP_UNLOCK(self);
    }

    fn read(&'static self, off: Voff, buf: &mut [u8], ioflag: i32) -> Result<usize, Errno> {
        vop_rw(
            self,
            UioRw::UIO_READ,
            off,
            buf.as_mut_ptr().cast(),
            buf.len(),
            ioflag,
        )
    }

    fn write(&'static self, off: Voff, buf: &[u8], ioflag: i32) -> Result<usize, Errno> {
        // UIO_WRITE only reads through the iovec (uiomove copies out of it).
        vop_rw(
            self,
            UioRw::UIO_WRITE,
            off,
            buf.as_ptr().cast_mut().cast(),
            buf.len(),
            ioflag,
        )
    }

    fn vtype(&self) -> VType {
        match self.v_type.get() {
            Vtype::VNON => VType::VNON,
            Vtype::VREG => VType::VREG,
            Vtype::VDIR => VType::VDIR,
            Vtype::VBLK => VType::VBLK,
            Vtype::VCHR => VType::VCHR,
            Vtype::VLNK => VType::VLNK,
            Vtype::VSOCK => VType::VSOCK,
            Vtype::VFIFO => VType::VFIFO,
            Vtype::VBAD => VType::VBAD,
        }
    }

    fn is_zero_device(&self) -> bool {
        iszerodev(self.v_rdev())
    }

    fn attr_flags(&'static self) -> Result<u64, Errno> {
        let Some(p) = curproc() else {
            panic(format_args!("mmap: no curproc"));
        };
        let mut vattr = Vattr::new();
        VOP_GETATTR(self, &mut vattr, p.ucred_ptr(), p)?;
        Ok(vattr.va_flags)
    }
}

/// A mount's identity for the pager (its address as a number, 0 for none).
fn mount_id(mp: Option<&'static Mount>) -> usize {
    mp.map_or(0, |m| core::ptr::from_ref(m).addr())
}

/// uvn_attach
///
/// attach a vnode structure to a VM object. if the vnode is already attached, then just bump
/// the reference count by one and return the VM object. if not already attached, attach and
/// return the new VM obj. the "accessprot" tells the max access the attaching thread wants to
/// our pages.
///
/// => in fact, nothing should be locked so that we can sleep here.
pub fn uvn_attach(vp: &'static Vnode, accessprot: VmProt) -> Option<ObjRef> {
    kernel::vm::vnode::uvn_attach(vp, accessprot)
}

/// uvm_vnp_terminate: external hook to clear out a vnode's VM
///
/// called in two cases: \[1\] when a persisting vnode vm object (i.e. one with a zero
/// reference count) needs to be freed so that a vnode can be reused, \[2\] at "umount -f" and
/// during a "reboot/halt" operation.
///
/// => the caller must XLOCK and VOP_LOCK the vnode before calling us
pub fn uvm_vnp_terminate(vp: &'static Vnode) {
    kernel::vm::vnode::uvm_vnp_terminate(vp);
}

/// uvm_vnp_uncache: disable "persisting" in a vnode... when last reference is gone we will
/// kill the object (flushing dirty pages back to the vnode if needed).
///
/// => returns TRUE if there was no uvm_object attached or if there was one and we killed it
///    [i.e. if there is no active uvn]
/// => called with the vnode VOP_LOCK'd [we will unlock it for I/O, if needed]
pub fn uvm_vnp_uncache(vp: &'static Vnode) -> bool {
    kernel::vm::vnode::uvm_vnp_uncache(vp)
}

/// uvm_vnp_setsize: grow or shrink a vnode uvn
///
/// grow   => just update size value
/// shrink => toss un-needed pages
///
/// => we assume that the caller has a reference of some sort to the vnode in question so that
///    it will not be yanked out from under us.
pub fn uvm_vnp_setsize(vp: &'static Vnode, newsize: Voff) {
    kernel::vm::vnode::uvm_vnp_setsize(vp, newsize);
}

/// uvm_vnp_sync: flush all dirty VM pages back to their backing vnodes.
///
/// => called from sys_sync with no VM structures locked
/// => only one process can do a sync at a time (the sync queue's lock).
pub fn uvm_vnp_sync(mp: Option<&'static Mount>) {
    kernel::vm::vnode::uvm_vnp_sync(mp.map(|m| mount_id(Some(m))));
}

/*
 * Kernel memory with bsd's shapes (uvm_km.c): the DMA modes over the machine's constraint,
 * km_alloc/km_free with the legacy pointer, and the kernel's first virtual address.
 */

/// `vm_min_kernel_address`: base of kernel virtual memory (a runtime value: amd64 shifts the
/// kernel map above its direct map, `init_x86_64`), read by [`uvm_init`].
pub static VM_MIN_KERNEL_ADDRESS: AtomicUsize =
    AtomicUsize::new(<Machine as VmParam>::VM_MIN_KERNEL_ADDRESS);

/// `kp_dma`: pages every DMA-capable device can reach.
pub static KP_DMA: KmemPaMode = pa_mode(<Machine as Pmap>::DMA_CONSTRAINT);

/// `kp_dma_contig`: one physically contiguous DMA-reachable segment.
pub static KP_DMA_CONTIG: KmemPaMode = KmemPaMode {
    kp_maxseg: 1,
    ..pa_mode(<Machine as Pmap>::DMA_CONSTRAINT)
};

/// `kp_dma_zero`: zeroed DMA-reachable pages.
pub static KP_DMA_ZERO: KmemPaMode = KmemPaMode {
    kp_zero: true,
    ..pa_mode(<Machine as Pmap>::DMA_CONSTRAINT)
};

/// `km_alloc`: allocate `sz` bytes of kernel memory with the given virtual, physical and
/// dynamic modes (`kernel::vm::km::km_alloc`, its address as the legacy pointer).
pub fn km_alloc(
    sz: usize,
    kv: &KmemVaMode,
    kp: &KmemPaMode,
    kd: &KmemDynMode,
) -> Option<NonNull<u8>> {
    let va = kernel::vm::km::km_alloc(sz, kv, kp, kd)?;
    NonNull::new(core::ptr::with_exposed_provenance_mut::<u8>(va.as_usize()))
}

/// `km_free`: returns what `km_alloc` gave, with the same modes (`kernel::vm::km::km_free`).
pub fn km_free(v: NonNull<u8>, sz: usize, kv: &KmemVaMode, kp: &KmemPaMode) {
    kernel::vm::km::km_free(Vaddr::new(v.as_ptr().addr()), sz, kv, kp);
}

/// The host tests' stand-in for the boot glue's `pmap_bootstrap`, which they never run:
/// bootstraps the host pmap, so the kernel map's token is adopted before any path of the
/// kernel's allocator that maps can run (a machine's boot glue does it before `uvm_init`).
#[cfg(test)]
pub(crate) fn uvm_km_test_init() {
    <Machine as Pmap>::pmap_bootstrap_lazy();
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Over the host's memory (`setup_real_memory`, which runs uvm_km_test_init).

    use hal::malloc::MArray;

    use super::*;
    use crate::kern::subr_pool::tests::setup_real_memory;
    use crate::sys::malloc::{M_PINSYSCALL, M_WAITOK};
    use crate::sys::param::PAGE_SIZE;
    use crate::sys::syscall::{SYS_MAXSYSCALL, SYS_getpid, SYS_getppid};
    use crate::sys::syscall_mi::pin_check;
    use kernel::vm::km::{KD_NOWAIT, KP_ZERO, KV_ANY, uvm_km_pgremove_intrsafe};

    fn free_pages() -> i32 {
        UVMEXP.free.load(Ordering::Relaxed)
    }

    #[test]
    fn km_free_puts_the_pages_back_on_the_free_list() {
        let _g = setup_real_memory();
        let before = free_pages();
        let p = km_alloc(2 * PAGE_SIZE, &KV_ANY, &KP_ZERO, &KD_NOWAIT).expect("two pages");
        assert_eq!(free_pages(), before - 2);
        km_free(p, 2 * PAGE_SIZE, &KV_ANY, &KP_ZERO);
        // uvm_pglistfree's way: counted free at once. On the host (no `multiprocessor`) the
        // per-CPU cache is the free list itself, so this cannot tell the two paths apart; the
        // kernel crate's km tests do (`UVMEXP.free` against the cache's count).
        assert_eq!(free_pages(), before);
    }

    #[test]
    fn pgremove_intrsafe_runs_after_the_test_set_up() {
        let _g = setup_real_memory();
        // The kernel's token is reachable once uvm_km_test_init ran: an empty range is a
        // kremove of nothing, not a panic.
        let va = Vaddr::new(0xffff_9800_0000_0000);
        uvm_km_pgremove_intrsafe(va, va);
    }

    #[test]
    fn a_pinsyscalls_table_is_honoured_at_syscall_entry() {
        // sys_pinsyscalls builds the table in an MArray and hands it to the process (its
        // ReadMostly ps_libcpin, replaced as below): pin_check, at every system call, reads it.
        let _g = setup_real_memory();
        let pr = crate::kern::proc_glue::test_process(1);
        let p = crate::kern::proc_glue::test_thread(pr);
        let mut pins =
            MArray::new(M_PINSYSCALL, M_WAITOK, SYS_MAXSYSCALL, |_| 0u32).expect("the table");
        // The host has no user mode: the system call instruction is at 0 (PROC_PC is 0, no
        // signal trampoline), inside [0, PAGE_SIZE).
        pins[SYS_getpid as usize] = u32::MAX; // "multiple locations"
        pins[SYS_getppid as usize] = 0; // not pinned
        drop(pr.ps_libcpin.replace(kernel::proc::PinTable {
            start: 0,
            end: PAGE_SIZE,
            pins: Some(pins),
        }));
        {
            let t = pr.ps_libcpin.read();
            assert_eq!((t.start, t.end), (0, PAGE_SIZE));
            assert_eq!(t.npins(), SYS_MAXSYSCALL);
        }
        assert_eq!(pin_check(&p, SYS_getpid as Register), Ok(()));
        // The table goes with its owner (the kernel test double's malloc).
        drop(pr.ps_libcpin.replace(kernel::proc::PinTable::NONE));
    }
}
/* </TESTS> */
