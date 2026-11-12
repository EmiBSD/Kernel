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
 * Copyright (c) 1990, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from the Stanford/CMU enet packet filter,
 * (net/enet.c) distributed as part of 4.3BSD, and code contributed
 * to Berkeley by Steven McCanne and Van Jacobson both of Lawrence
 * Berkeley Laboratory.
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
 *	@(#)bpf.h	8.1 (Berkeley) 6/10/93
 */
/*
 * Copyright (c) 1990, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 * Copyright (c) 2010, 2014 Henning Brauer <henning@openbsd.org>
 *
 * This code is derived from the Stanford/CMU enet packet filter,
 * (net/enet.c) distributed as part of 4.3BSD, and code contributed
 * to Berkeley by Steven McCanne and Van Jacobson both of Lawrence
 * Berkeley Laboratory.
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
 *	@(#)bpf.c	8.2 (Berkeley) 3/28/94
 */

/*
 * Copyright (c) 1990, 1991, 1993
 *	The Regents of the University of California.  All rights reserved.
 *
 * This code is derived from the Stanford/CMU enet packet filter,
 * (net/enet.c) distributed as part of 4.3BSD, and code contributed
 * to Berkeley by Steven McCanne and Van Jacobson both of Lawrence
 * Berkeley Laboratory.
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
 *	@(#)bpfdesc.h	8.1 (Berkeley) 6/10/93
 */
/* </LICENSES> */

/* <CODE> */
//! The Berkeley packet filter, bpf(4): `net/bpf.c`'s device (`/dev/bpf`, a cloning character
//! device, major 23 on both machines) and the taps the drivers call, with `<net/bpfdesc.h>`'s
//! descriptor (`struct bpf_d`, [`BpfD`]) and tap (`struct bpf_if`, [`BpfIf`]). `<net/bpf.h>`'s
//! user interface is `hal::sys::net_bpf`'s, the filter machine `kernel::net::bpf_filter`'s.
//!
//! Upstream: sys/net/bpf.h @ 3ce1f3f79392
//! Upstream: sys/net/bpf.c @ 3ce1f3f79392
//! Upstream: sys/net/bpfdesc.h @ 3ce1f3f79392
//! LZ: sys/net/bpf.rs@3c62ede68802
//! LZ: sys/net/bpfdesc.rs@3c62ede68802
//!
//! A driver attaches a tap per interface and link type (`bpfattach`, `bpfxattach`,
//! `bpfsattach`) on a word of its own ([`BpfTap`]: `if_bpf`, a ring's word), which names the
//! tap while a descriptor listens; it hands its packets to [`bpf_mtap`] and its siblings
//! when the word is set. Each open of `/dev/bpf` makes a descriptor that `BIOCSETIF` attaches
//! to a tap; its read filter picks the packets and how much of each to keep, and `read(2)`
//! returns them behind `struct bpf_hdr`s, rotating the descriptor's store and hold buffers.
//!
//! **Locks** (the C's: `[m]` the descriptor's `bd_mtx`, `IPL_NET`; the rest the kernel lock).
//! The cdev entry points run under the kernel lock (the row has no `D_MPSAFE`): the
//! descriptor list ([`BPF_D_LIST`]) and the tap list ([`BPF_IFLIST`]) are walked under its
//! reader and changed under its witness, as are an interface's chain of listeners (`bif_dlist`
//! and each `bd_next`), the driver's word and the filter words, which the taps read inside an
//! SMR read section with no lock. The `[m]` members are [`BpfSlots`], inside `bd_mtx`, but for
//! the lengths, the state and the drop count, atomics written only under the mutex, which the
//! read filter (given no witness) and `BIOCGSTATS` read; the members `bpfioctl` writes
//! without the mutex in the C (`bd_promisc`, `bd_dirfilt`, `bd_fildrop`, `bd_hdrcmplt`,
//! `bd_async`, `bd_sig`, `bd_locked`) are relaxed atomics, read by the taps unlocked as in the
//! C. `bpf_detachd` lets `bd_mtx` go around `ifpromisc`, under the net lock taken detached
//! (`hal::netlock::lock`, the C's `NET_LOCK()`), as `bpfwrite` holds it around the output.
//!
//! **Lifetimes.** A descriptor and a tap are counted `malloc(M_DEVBUF)` blocks (`MArc`), as
//! the C mallocs them, published to the taps through counted SMR words (`hal::smr`'s S4): a
//! word holds a reference, a reference that leaves a word is released after a grace period,
//! so a tap's walk is sound by SMR alone. The holders, against the C's `bpf_get`/`bpf_put`:
//!
//! | the C | here |
//! |---|---|
//! | `refcnt_init` at `bpfopen` (the list's), dropped by `bpfclose`'s `bpf_put` | `bd_self`, set at open, taken at close |
//! | `bpf_get`/`bpf_put` around `bpfread`, `bpfwrite`, `bpfioctl` | the lookup's handle for the call |
//! | `bpfkqfilter`'s `bpf_get`, `filt_bpfrdetach`'s `bpf_put` | the knote's hook (`Knote::set_hook_shared`), dropped with the knote |
//! | `bpf_wakeup`'s `bpf_get` for the wake task, `bpf_wakeup_cb`'s `bpf_put` | the task's handle (`Task::set_shared`), from open to close; a clone pins the task to queue it |
//! | `bpf_catchpacket`'s `bpf_get` for the wait timeout, `bpf_wait_cb`'s and `timeout_del`'s `bpf_put` | the timeout's handle (`Timeout::set_shared`), from open to close; a clone pins it to arm it |
//! | `bpf_detachd`'s `bpf_get` across `ifpromisc` | the caller's handle |
//! | `bpfsdetach`'s `bpf_get` across `vdevgone` | a handle taken from the chain inside a read section |
//! | the interface's `SMR_SLIST` (no count: `bpf_d_smr` frees after a grace period) | the chain word's reference (`bif_dlist` or the predecessor's `bd_next`), retired at the unlink |
//! | a tap's `malloc` freed by `bpfsdetach` at once | the tap list's `bif_self`, the driver's word, a descriptor's `bif`: freed when the last goes, a grace period after the word's |
//! | a program's `malloc`s freed by `bpf_prog_smr` | the filter word's reference, retired at the next `BIOCSETF` or with the descriptor |
//!
//! **Lock and spl transitions** against the C (the kernel lock's witness raises to `IPL_VM`:
//! one spl pair the C's `KERNEL_ASSERT_LOCKED` lacks):
//!
//! | operation | the C | here |
//! |---|---|---|
//! | tap, one listener (`_bpf_mtap`) | SMR section, `bd_mtx` section | the same; the driver's word one Acquire load more (the C passes the tap) |
//! | `bpfread`, no sleep (bpf.c:444-579) | `bd_mtx` twice (around the `uiomove`) | the same; the lookup's reader and a read of `bd_self` (a compare-and-swap pair) |
//! | `bpfwrite` (bpf.c:629-685) | the write filter's SMR section, the net lock | the same, and one `bd_mtx` section for the tap (D-N6.3c-23) |
//! | `BIOCSETF` (bpf.c:1108-1166) | the filter store, `bd_mtx` for `bpf_resetd` | the same, and the witness for the store (+1 spl pair) |
//! | `BIOCSETIF` (bpf.c:1169-1211, :285-357) | `bd_mtx`, the net lock for a promiscuous detach | the same, and the witness for each chain edit (+1 spl pair each) |
//! | `bpfopen` (bpf.c:371-408) | one `malloc`, the list insert | two `malloc`s (the klist's box), the task's and the timeout's set (one section each), the witness for the insert |
//! | `bpfclose` (bpf.c:410-425) | `bd_mtx` | the same, the witness for the list and the chain, the timeout's and the task's delete and clear (four sections the C lacks: the cycle broken, below) |
//!
//! ## Deviations
//! - The chains of `struct m_hdr` that `bpf_tap_hdr`, `bpf_mtap_hdr` and `bpf_mtap_af` build
//!   on the stack (a header from a linear buffer, then the mbuf chain or a second buffer) are
//!   [`BpfPkt`]s; `bpf_mcopy`, `bpf_mbuf_copy` and the loads walk its segments. The `mp` of
//!   `_bpf_mtap` is an `Option`: `None` for `bpf_tap_hdr`, whose stack header has none.
//! - The taps return `bool` (the C's `int` "drop it").
//! - `bpf_allocbufs` and `bpf_setf`'s copy of the program are zeroed (`MArray::new`), so no
//!   byte the kernel never wrote (the padding between a `bpf_hdr` and its packet) reaches
//!   user space; two passes over the buffers the C's `malloc` does not make (docs/ROADMAP.md
//!   N12).
//! - A descriptor lookup that the C assumes succeeds (`bpfclose`, `bpfread`, `bpfwrite`,
//!   `bpfioctl` on an open device) answers `ENXIO` when it does not.
//! - The ioctls read and write their argument through `hal::pod`; a request shorter than its
//!   structure is `EINVAL`. `BIOCGETIF`/`BIOCSETIF` use only the `struct ifreq`'s name, and
//!   `bpf_setif`'s name comparison stops at `IFNAMSIZ` where the C's `strcmp` reads on.
//! - `bpf_movein`: `MGETHDR`, `MCLGETL` and `m_tag_get` with `M_WAIT` sleep rather than fail
//!   in the C; a failure here answers `ENOBUFS`. `bpfwrite`'s destination carries `sa_len`
//!   16 (`sizeof(struct sockaddr)`, the C leaves 0), so the view the output takes covers the
//!   `sa_data` an Ethernet output reads; nothing reads `sa_len` on this path.
//! - A wait timeout or a wake task still pending when the descriptor closes runs its work in
//!   the close (the C's runs after it, holding a `bpf_get`): the handles they hold are given
//!   back there, the cycle through the descriptor broken (the counts' table above).
//! - `bpfsdetach` refuses (`panic(9)`) a tap with listeners before `bpfilterattach` named the
//!   device to revoke them through; the C's scan would revoke on the major it finds.
//! - `bpfdetach` looks for the interface's next tap from the list's head after each
//!   `bpfsdetach`, which sleeps (`vdevgone`): the C's `TAILQ_FOREACH_SAFE` keeps a successor
//!   another detach may free meanwhile. O(n) a tap, an interface has one or a few.
//! - `bpf_setif`'s `bpf_allocbufs` while `sbuf` is held by a reader, or a second reader's
//!   rotation while the first copies out (the C's `KASSERT(bd_in_uiomove == 0)`), drop the
//!   packet rather than write through a missing buffer: the C's store slot would be NULL.
//! - `NVLAN` is not configured: `bpf_mtap_ether` passes the packet as it is. `SMALL_KERNEL` is
//!   not defined: `bpf_sysctl` is in.
//!
//! ## Redesign
//! - LZ's `bpf_d` and `bpf_if` written into raw `malloc` blocks through `unsafe`, shared under
//!   `unsafe impl Sync`, freed by `smr_call` over a `void *` -> counted `MArc`s of `M_DEVBUF`
//!   ([`BpfDRef`], [`BpfIfRef`]): the C's `bd_refcnt` is the block's count, the deferred free
//!   the SMR words' retirement (the table above; docs/IDIOMS.md, the counted SMR word over a
//!   malloc'd item).
//! - The interface's `SMR_SLIST` of listeners (bsd's `SmrSlistHead`, ten `unsafe` there) ->
//!   a chain of counted words, `bif_dlist` and each `bd_next` (D-N6.3c-4): an insert stores
//!   the new element's next word before publishing the element, a removal stores a clone of
//!   the successor into the predecessor's word and leaves the element's own next word as it
//!   is, so a tap standing on the element goes on down the chain, as the C's
//!   `SMR_SLIST_REMOVE` lets it. The writer walks the chain to the predecessor, O(n), as the C.
//! - `bd_rfilter`/`bd_wfilter` (LZ's `BpfFilterPtr` over `&'static` programs) -> counted words
//!   of [`BpfProg`], a program's instructions an `MArray` of `M_DEVBUF` (the C's
//!   `mallocarray`) in a counted block (the C's `malloc` of `bpf_program_smr`). LZ's refusal of
//!   a `bf_len` of 0 before allocating is gone: the C's path (an empty `mallocarray`, then
//!   `bpf_validate`'s refusal) gives the same `EINVAL`.
//! - The driver's `caddr_t` cookie (`PtrWord<u8>`, `if_bpf`, `bif_driverp`) -> the typed word
//!   [`BpfTap`], a counted SMR word of the tap: a tap reads the tap it names inside its read
//!   section, so a tap the interface detached while a driver still taps is sound (the C frees
//!   it trusting the driver to have stopped). `ifiq_bpfp` is a `Link` to a ring's word.
//! - The three buffers (`Cell<*mut u8>`) -> `Option<MArray<u8>>` slots in [`BpfSlots`]:
//!   `bpfread` takes the hold buffer out for its copy and puts it back as the free one, so a
//!   rotation meanwhile finds no free buffer and drops, what the C's `bd_in_uiomove` asserts.
//! - The klist: `Klist<G>` is generic over the glue and a descriptor is not (the taps reach it
//!   from `kernel::net::if_`, which has none): it is a counted box beside the descriptor,
//!   erased (`AnyArc`), with the two functions `bpfopen::<G>` sets (`knote_locked` and
//!   `klist_invalidate` for that glue), docs/SCAFFOLDING.md (N9). `bpfilterattach::<G>` names
//!   the major and `vdevgone::<G>` for `bpfsdetach` likewise; `bsd`'s wrapper finds the major
//!   by the C's scan of `cdevsw`, once.
//! - `bpfread` sleeps on the descriptor's address in its mutex (`MutexedGuard::sleep_nsec`,
//!   the C's `msleep_nsec`, wmesg `"bpf"`); `bd_nreaders` counts the sleepers as the C's.
//! - `bpf_validate`'s bound, `net.bpf.maxbufsize`, is read once by `bpf_setf` and passed.

use core::cmp::{max, min};
use core::mem::size_of;
use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, Ordering};

use hal::intr::IPL_NET;
use hal::klock;
use hal::malloc::{M_CANFAIL, M_NOWAIT, M_WAITOK, MArc, MArray};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::pinlist::{PinEntry, PinList};
use hal::pool::AnyArc;
use hal::slot::ReadMostly;
use hal::smr::{self, SmrArcLink, SmrArcWord, SmrCounted, SmrRef};
use hal::sys::errno::Errno;
use hal::sys::filio::{FIOASYNC, FIOGETOWN, FIONREAD, FIOSETOWN};
use hal::sys::mbuf::MT_DATA;
use hal::sys::net_bpf::{
    BIOCDWTIMEOUT, BIOCFLUSH, BIOCGBLEN, BIOCGDIRFILT, BIOCGDLT, BIOCGDLTLIST, BIOCGETIF,
    BIOCGFILDROP, BIOCGHDRCMPLT, BIOCGRSIG, BIOCGRTIMEOUT, BIOCGSTATS, BIOCGWTIMEOUT,
    BIOCIMMEDIATE, BIOCLOCK, BIOCPROMISC, BIOCSBLEN, BIOCSDIRFILT, BIOCSDLT, BIOCSETF, BIOCSETFNR,
    BIOCSETIF, BIOCSETWF, BIOCSFILDROP, BIOCSHDRCMPLT, BIOCSRSIG, BIOCSRTIMEOUT, BIOCSWTIMEOUT,
    BIOCVERSION, BPF_DIRECTION_IN, BPF_DIRECTION_OUT, BPF_F_DIR_SHIFT, BPF_F_FLOWID,
    BPF_FILDROP_CAPTURE, BPF_FILDROP_DROP, BPF_FILDROP_PASS, BPF_MAJOR_VERSION, BPF_MAXBUFSIZE,
    BPF_MAXINSNS, BPF_MINBUFSIZE, BPF_MINOR_VERSION, BpfDltlist, BpfHdr, BpfInsn, BpfProgram,
    BpfStat, BpfVersion, DLT_EN10MB, DLT_IEEE802_11, DLT_IEEE802_11_RADIO, DLT_LOOP, DLT_NULL,
    DLT_PPP, DLT_RAW, DLT_SLIP, SIZEOF_BPF_HDR, bpf_wordalign,
};
use hal::sys::net_if::{IFF_UP, IFNAMSIZ};
use hal::sys::netinet_if_ether::ETHER_HDR_LEN;
use hal::sys::param::PAGE_SIZE;
use hal::sys::signal::{NSIG, SIGIO};
use hal::sys::socket::{
    AF_INET, AF_UNSPEC, NET_BPF_BUFSIZE, NET_BPF_MAXBUFSIZE, SockaddrStorage, pseudo_AF_HDRCMPLT,
};
use hal::sys::types::{Dev, minor};
use hal::user::{AbiPod, UserPtr};

use crate::assert::{kassert, kpanic};
use crate::clock::{Timeval, sec_to_nsec};
use crate::event::{
    EVFILT_READ, FILTEROP_ISFD, FILTEROP_MPSAFE, Filter, Filterops, Kevent, Klist, Knote,
    knote_modify, knote_process,
};
use crate::file::FileGlue;
use crate::fs::spec::CLONE_SHIFT;
use crate::fs::vnode::{IO_NDELAY, VCHR};
use crate::net::bpf_filter::{_bpf_lfilter, BpfOps, bpf_validate};
use crate::net::if_::ifpromisc;
use crate::net::if_var::Ifnet;
use crate::net::mbuf::{
    M_EXT, M_FLOWID, M_WAIT, MAX_LINKHDR, MAXMCLBYTES, MHLEN, Mbuf, MbufBox, PACKET_TAG_DLT,
    m_align, m_gethdr, m_microtime, mclgetl,
};
use crate::net::mbuf2::{m_tag_get, m_tag_prepend};
use crate::net::proto::SockaddrView;
use crate::proc::{ProcGlue, Thread};
use crate::sigio::{
    SigioRef, TIOCGPGRP, TIOCSPGRP, pgsigio, sigio_free, sigio_getown, sigio_setown,
};
use crate::sysctl::{SysctlReq, int_bounded};
use crate::task::{SYSTQ, Task, task_add, task_del};
use crate::tc::{microtime, nsecuptime};
use crate::timeout::{KCLOCK_NONE, Timeout};
use crate::uio::Uio;

/// `M_DEVBUF` (`<sys/malloc.h>`, `bsd`'s: docs/SCAFFOLDING.md "copies"): the descriptors',
/// taps', programs' and buffers' malloc type.
const M_DEVBUF: i32 = 2;

/// `MALLOC_MAX` (`<sys/malloc.h>`): the largest `malloc(9)` request, `net.bpf.maxbufsize`'s
/// upper bound.
const MALLOC_MAX: i32 = (65535 * PAGE_SIZE) as i32;

/// `MAXTSLP` (`<sys/systm.h>`): the longest sleep.
const MAXTSLP: u64 = u64::MAX - 1;

/// `PRINET` (`<sys/param.h>`): the priority of a sleep for the network.
const PRINET: i32 = 26;

/// `INFSLP`: no timeout.
const INFSLP: u64 = hal::sleep::INFSLP;

/// `PCATCH`.
const PCATCH: i32 = hal::sleep::PCATCH;

/// `NBPFILTER`: the count `config(8)` writes into `bpfilter.h` for `pseudo-device bpfilter`
/// (GENERIC).
pub const NBPFILTER: i32 = 1;

/// `BPF_BUFSIZE`: the default read buffer size (`net.bpf.bufsize`'s).
const BPF_BUFSIZE: i32 = 32768;

/// `BPF_S_IDLE`: `bd_state` when no packet waits for a reader.
const BPF_S_IDLE: u8 = 0;
/// `BPF_S_WAIT`: a packet arrived; the wait timeout (`bd_wtout`) runs.
const BPF_S_WAIT: u8 = 1;
/// `BPF_S_DONE`: a read may return what is buffered.
const BPF_S_DONE: u8 = 2;

/// A packet as a tap hands it to the filter and to the capture: the C's chain of stack
/// `struct m_hdr`s (a header from a linear buffer) in front of an mbuf chain or of a second
/// linear buffer (`bpf_tap_hdr`, `bpf_mtap_hdr`, `bpf_mtap_af`), or an mbuf chain as it is.
#[derive(Clone, Copy)]
pub struct BpfPkt<'a> {
    /// The header the tap prepends (empty when there is none).
    hdr: &'a [u8],
    /// What follows it.
    body: BpfBody<'a>,
}

impl<'a> BpfPkt<'a> {
    /// An mbuf chain as it is (`bpf_mtap`).
    pub fn mbuf(m: &'a Mbuf) -> Self {
        Self {
            hdr: &[],
            body: BpfBody::Mbuf(m),
        }
    }

    /// The header `hdr` in front of the mbuf chain `m` (`bpf_mtap_hdr`).
    pub fn hdr_mbuf(hdr: &'a [u8], m: &'a Mbuf) -> Self {
        Self {
            hdr,
            body: BpfBody::Mbuf(m),
        }
    }

    /// The header `hdr` in front of the linear buffer `buf` (`bpf_tap_hdr`).
    pub fn hdr_buf(hdr: &'a [u8], buf: &'a [u8]) -> Self {
        Self {
            hdr,
            body: BpfBody::Buf(buf),
        }
    }

    /// The packet's bytes, segment by segment (the C's walk along `m_next`).
    fn segments(&self) -> BpfSegs<'a> {
        let (m, buf) = match self.body {
            BpfBody::Mbuf(m) => (Some(m), None),
            BpfBody::Buf(b) => (None, Some(b)),
        };
        BpfSegs {
            hdr: Some(self.hdr),
            m,
            buf,
        }
    }

    /// The packet's length, every segment's.
    fn len(&self) -> usize {
        self.segments().map(<[u8]>::len).sum()
    }
}

/// The part of a [`BpfPkt`] after its header.
#[derive(Clone, Copy)]
enum BpfBody<'a> {
    /// An mbuf chain.
    Mbuf(&'a Mbuf),
    /// A linear buffer.
    Buf(&'a [u8]),
}

/// The iterator of [`BpfPkt::segments`].
struct BpfSegs<'a> {
    hdr: Option<&'a [u8]>,
    m: Option<&'a Mbuf>,
    buf: Option<&'a [u8]>,
}

impl<'a> Iterator for BpfSegs<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if let Some(h) = self.hdr.take() {
            return Some(h);
        }
        if let Some(m) = self.m {
            self.m = m.m_next.get();
            return Some(m.data());
        }
        self.buf.take()
    }
}

/// `struct bpf_program_smr`: a filter program the kernel validated and keeps: its
/// instructions (`bps_bf`, the C's `mallocarray(M_DEVBUF)`) in a counted block (the C's
/// `malloc(M_DEVBUF)` of the structure), published by a descriptor's filter word.
pub struct BpfProg {
    /// `bps_bf.bf_insns`, `bps_bf.bf_len`.
    insns: MArray<BpfInsn>,
    /// `bps_smr`: where the filter word's retired reference waits for its grace period.
    smr: SmrArcLink,
}

impl SmrCounted for BpfProg {
    fn smr_link(&self) -> &SmrArcLink {
        &self.smr
    }
}

/// `struct bpf_program_smr *` held: a counted block of `M_DEVBUF`.
pub type BpfProgRef = MArc<BpfProg>;

/// `bd_rfilter`, `bd_wfilter`: a descriptor's program word (`SMR_PTR` of a
/// `struct bpf_program_smr *`).
type ProgWord = SmrArcWord<BpfProg, BpfProgRef>;

/// `bd_next`, `bif_dlist`: a word of an interface's chain of listeners (`SMR_SLIST`).
type DescWord = SmrArcWord<BpfD, BpfDRef>;

/// The `[m]` members of a descriptor, inside its `bd_mtx` (`IPL_NET`). Three buffer slots
/// hold `bd_bufsize` bytes each: the store slot is always full of a buffer once one is
/// attached, packets go into it; when it is full it moves into the hold slot for `read(2)`,
/// and the free buffer (the one a read gave back) becomes the store; with no free buffer a
/// packet that does not fit is dropped.
pub struct BpfSlots {
    /// `bd_sbuf`: the store slot.
    sbuf: Option<MArray<u8>>,
    /// `bd_hbuf`: the hold slot.
    hbuf: Option<MArray<u8>>,
    /// `bd_fbuf`: the free slot.
    fbuf: Option<MArray<u8>>,
    /// `bd_bufsize`: the buffers' length.
    bufsize: i32,
    /// `bd_in_uiomove`: a reader holds the hold buffer for its copy (the C's assertion's
    /// flag).
    in_uiomove: bool,
    /// `bd_bif`: the tap the descriptor listens on.
    bif: Option<BpfIfRef>,
    /// `bd_rtout`: the read timeout, in nanoseconds.
    rtout: u64,
    /// `bd_wtout`: the wait after a first packet, in nanoseconds (`INFSLP`: until the buffer
    /// fills; 0: immediate).
    wtout: u64,
    /// `bd_nreaders`: the threads asleep in `bpfread`.
    nreaders: u64,
}

/// `struct bpf_d`: the descriptor of an open `/dev/bpf` unit, a counted block of `M_DEVBUF`.
pub struct BpfD {
    /// `bd_next`: the next listener on the interface's chain.
    bd_next: DescWord,
    /// `bd_mtx`: the `[m]` members.
    bd_mtx: Mutexed<BpfSlots>,
    /// \[m\] `bd_slen`: the store buffer's length (written under `bd_mtx`).
    bd_slen: AtomicI32,
    /// \[m\] `bd_hlen`: the hold buffer's length.
    bd_hlen: AtomicI32,
    /// \[m\] `bd_state`: `BPF_S_IDLE`, `BPF_S_WAIT` or `BPF_S_DONE`.
    bd_state: AtomicU8,
    /// \[m\] `bd_dcount`: the packets dropped.
    bd_dcount: AtomicU64,
    /// `bd_rcount`: the packets received (`atomic_inc_long`).
    bd_rcount: AtomicU64,
    /// `bd_rfilter`: the read filter.
    bd_rfilter: ProgWord,
    /// `bd_wfilter`: the write filter.
    bd_wfilter: ProgWord,
    /// `bd_promisc`: listening promiscuously.
    bd_promisc: AtomicBool,
    /// `bd_locked`: `BIOCLOCK` was given.
    bd_locked: AtomicBool,
    /// `bd_fildrop`: what a filtered packet becomes (`BPF_FILDROP_*`).
    bd_fildrop: AtomicU8,
    /// `bd_dirfilt`: the directions not captured.
    bd_dirfilt: AtomicU8,
    /// `bd_hdrcmplt`: the link header a writer gives is complete (no source filled in).
    bd_hdrcmplt: AtomicBool,
    /// `bd_async`: a packet's reception signals (`FIOASYNC`).
    bd_async: AtomicI32,
    /// `bd_sig`: the signal it sends.
    bd_sig: AtomicI32,
    /// `bd_sigio`: the async I/O registration.
    bd_sigio: SigioRef,
    /// `bd_klist`: the knotes, in a box of the glue's.
    bd_klist: BpfKlist,
    /// `bd_unit`: the minor the descriptor was opened as.
    bd_unit: i32,
    /// `bd_list`: the link on [`BPF_D_LIST`].
    bd_link: PinEntry<BpfD>,
    /// The list's reference (the C's `refcnt_init`), set at open, taken at close.
    bd_self: ReadMostly<Option<BpfDRef>>,
    /// `bd_wake_task`: the deferred `pgsigio`.
    bd_wake_task: Task,
    /// `bd_wait_tmo`: the wakeup a while after the first packet.
    bd_wait_tmo: Timeout,
    /// `bd_smr`: where the chain words' retired references wait.
    smr: SmrArcLink,
}

hal::pin_adapter!(
    /// `bd_list`: a descriptor's link on [`BPF_D_LIST`].
    pub BpfDLink: BpfD, bd_link
);

/// `struct bpf_d *` held: a counted handle to a descriptor's block.
pub type BpfDRef = MArc<BpfD>;

impl BpfD {
    /// The descriptor of `unit`, as `bpfopen` initialises the C's `malloc(M_ZERO)` block.
    fn new(unit: i32, bufsize: i32, klist: BpfKlist) -> Self {
        Self {
            bd_next: DescWord::new(),
            bd_mtx: Mutexed::new(
                IPL_NET,
                BpfSlots {
                    sbuf: None,
                    hbuf: None,
                    fbuf: None,
                    bufsize,
                    in_uiomove: false,
                    bif: None,
                    rtout: 0,      // no timeout by default
                    wtout: INFSLP, // wait for the buffer to fill by default
                    nreaders: 0,
                },
            ),
            bd_slen: AtomicI32::new(0),
            bd_hlen: AtomicI32::new(0),
            bd_state: AtomicU8::new(BPF_S_IDLE),
            bd_dcount: AtomicU64::new(0),
            bd_rcount: AtomicU64::new(0),
            bd_rfilter: ProgWord::new(),
            bd_wfilter: ProgWord::new(),
            bd_promisc: AtomicBool::new(false),
            bd_locked: AtomicBool::new(false),
            bd_fildrop: AtomicU8::new(BPF_FILDROP_PASS),
            bd_dirfilt: AtomicU8::new(0),
            bd_hdrcmplt: AtomicBool::new(false),
            bd_async: AtomicI32::new(0),
            bd_sig: AtomicI32::new(SIGIO),
            bd_sigio: SigioRef::new(),
            bd_klist: klist,
            bd_unit: unit,
            bd_link: PinEntry::new(),
            bd_self: ReadMostly::new(None),
            bd_wake_task: Task::unset(),
            bd_wait_tmo: Timeout::unset(),
            smr: SmrArcLink::new(),
        }
    }

    /// The tap the descriptor listens on (`d->bd_bif`), read under `bd_mtx` where the C reads
    /// it under the kernel lock alone (D-N6.3c-23).
    fn bif(&self) -> Option<BpfIfRef> {
        self.bd_mtx.lock().bif.clone()
    }
}

impl SmrCounted for BpfD {
    fn smr_link(&self) -> &SmrArcLink {
        &self.smr
    }
}

impl Drop for BpfD {
    /// `bpf_d_smr`'s frees: the registration; the buffers, the programs and the klist go with
    /// their members.
    fn drop(&mut self) {
        sigio_free(&self.bd_sigio);
    }
}

/// `bd_klist`: the descriptor's klist, a `Klist<G>` locked by `bd_mtx` (the C's
/// `klist_init_mutex(&bd_klist, &bd_mtx)`), in a counted box of the glue `bpfopen::<G>`
/// named, with that glue's two functions over it (docs/SCAFFOLDING.md: the glue's erasure).
struct BpfKlist {
    /// The `MArc<KlistBox<G>>`, erased.
    arc: AnyArc,
    /// `knote_locked(&d->bd_klist, hint)`, with `bd_mtx` held.
    knote: fn(&BpfD, &MutexedGuard<'_, BpfSlots>, i64),
    /// `klist_invalidate(&d->bd_klist)`.
    invalidate: fn(&BpfD),
}

/// The klist of a descriptor of the glue `G`; its drop is `klist_free`'s check.
struct KlistBox<G: ProcGlue> {
    klist: Klist<G>,
}

impl<G: ProcGlue> Drop for KlistBox<G> {
    fn drop(&mut self) {
        self.klist.free();
    }
}

/// `struct bpf_if`: the tap of an attached interface (or of a driver's own word), a counted
/// block of `M_DEVBUF`.
pub struct BpfIf {
    /// `bif_next`: the link on [`BPF_IFLIST`].
    bif_link: PinEntry<BpfIf>,
    /// The list's reference, taken by `bpfsdetach`.
    bif_self: ReadMostly<Option<BpfIfRef>>,
    /// `bif_dlist`: the chain of listeners.
    bif_dlist: DescWord,
    /// `bif_driverp`: the driver's word.
    bif_driverp: &'static BpfTap,
    /// `bif_dlt`: the link layer type.
    pub bif_dlt: u32,
    /// `bif_hdrlen`: the `bpf_hdr` length with the padding that aligns the network header.
    pub bif_hdrlen: u32,
    /// `bif_name`: the name of the "subsystem", NUL-padded (a copy: the C points at
    /// `if_xname`, which never changes).
    pub bif_name: [u8; IFNAMSIZ],
    /// `bif_ifp`: the interface (`None` for a driver's own tap, `bpfsattach`'s).
    pub bif_ifp: Option<&'static Ifnet>,
    /// Where the words' retired references wait.
    smr: SmrArcLink,
}

hal::pin_adapter!(
    /// `bif_next`: a tap's link on [`BPF_IFLIST`].
    pub BpfIfLink: BpfIf, bif_link
);

/// `struct bpf_if *` held: a counted handle to a tap's block.
pub type BpfIfRef = MArc<BpfIf>;

impl SmrCounted for BpfIf {
    fn smr_link(&self) -> &SmrArcLink {
        &self.smr
    }
}

/// A driver's bpf word (`caddr_t if_bpf`, a ring's tap word): it names its tap while a
/// descriptor listens on it (`bpf_attachd`, `bpf_detachd`), read by the taps inside their
/// read section. Valid all zero (none), so a zeroed softc holds one.
pub type BpfTap = SmrArcWord<BpfIf, BpfIfRef>;

/// \[a\] `bpf_maxbufsize`: the largest buffer `BIOCSBLEN` grants (`net.bpf.maxbufsize`), and
/// `bpf_validate`'s bound on packet offsets.
#[allow(
    non_upper_case_globals,
    reason = "the C's name; BPF_MAXBUFSIZE is its default"
)]
pub static bpf_maxbufsize: AtomicI32 = AtomicI32::new(BPF_MAXBUFSIZE);

/// \[a\] `bpf_bufsize`: a new descriptor's buffer size (`net.bpf.bufsize`).
#[allow(
    non_upper_case_globals,
    reason = "the C's name; BPF_BUFSIZE is its default"
)]
pub static bpf_bufsize: AtomicI32 = AtomicI32::new(BPF_BUFSIZE);

/// `bpf_iflist`: the taps, under the kernel lock.
pub static BPF_IFLIST: PinList<BpfIfLink> = PinList::new();

/// `bpf_d_list`: the open descriptors, under the kernel lock.
pub static BPF_D_LIST: PinList<BpfDLink> = PinList::new();

/// The major of `/dev/bpf` (`bpfsdetach`'s scan, made once by `bpfilterattach`).
static BPF_MAJOR: OnceStatic<u32> = OnceStatic::new();

/// `vdevgone(maj, unit, unit, VCHR)` for the glue `bpfilterattach::<G>` named.
static BPF_REVOKE: OnceStatic<fn(u32, u32)> = OnceStatic::new();

/// `bpfread_filtops`: the read filter.
pub struct BpfReadFilter;

/// The erased table of the read filter (`kn_fop`'s target).
struct BpfFiltops<G>(core::marker::PhantomData<fn() -> G>);

impl<G: ProcGlue> BpfFiltops<G> {
    /// `&bpfread_filtops`.
    const READ: Filterops<G> = Filterops::of::<BpfReadFilter>();
}

/// `bpf_movein`: the packet a writer hands `bpfwrite`, built in an mbuf (its link header
/// moved into `sockp` for the output, as the descriptor's link type has it), checked by the
/// write filter and tagged with the link type.
fn bpf_movein<G: ProcGlue>(
    uio: &mut Uio<'_, G>,
    d: &BpfD,
    bif: &BpfIf,
    sockp: &mut SockaddrStorage,
) -> Result<MbufBox, Errno> {
    // The link header is copied into the address's data (Ethernet's), or names the address
    // family (the loopback's), or is not there (SLIP, PPP, raw, 802.11): room for it is left in
    // front of the aligned payload.
    let linktype = bif.bif_dlt;
    let hlen: u32 = match linktype {
        DLT_SLIP => {
            sockp.ss_family = AF_INET;
            0
        }
        DLT_PPP | DLT_IEEE802_11 | DLT_IEEE802_11_RADIO | DLT_RAW | DLT_NULL => {
            sockp.ss_family = AF_UNSPEC;
            0
        }
        DLT_EN10MB => {
            sockp.ss_family = AF_UNSPEC;
            ETHER_HDR_LEN as u32
        }
        DLT_LOOP => {
            sockp.ss_family = AF_UNSPEC;
            size_of::<u32>() as u32
        }
        _ => return Err(Errno::EIO),
    };

    if uio.uio_resid > MAXMCLBYTES {
        return Err(Errno::EMSGSIZE);
    }
    let len = uio.uio_resid as u32;
    if len < hlen {
        return Err(Errno::EINVAL);
    }

    // The payload's length, for its alignment.
    let alen = len - hlen;

    // Room for the headers and the aligned payload.
    let max_linkhdr = u32::try_from(MAX_LINKHDR.load(Ordering::Relaxed)).unwrap_or(0);
    let mlen = max(max_linkhdr, hlen) + alen.next_multiple_of(size_of::<usize>() as u32);
    if mlen as usize > MAXMCLBYTES {
        return Err(Errno::EMSGSIZE);
    }

    // The C's MGETHDR(M_WAIT) and MCLGETL(M_WAIT) sleep rather than fail.
    let Some(mut m) = m_gethdr(M_WAIT, MT_DATA) else {
        return Err(Errno::ENOBUFS);
    };
    if mlen as usize > MHLEN && (!mclgetl(&mut m, M_WAIT, mlen) || m.flags() & M_EXT == 0) {
        return Err(Errno::ENOBUFS);
    }

    m_align(&m, alen as i32); // the payload aligned
    m.data_add(-(hlen as isize));

    m.m_pkthdr().ph_ifidx.set(0);
    m.m_pkthdr().len.set(len as i32);
    m.m_len().set(len);

    let Some(data) = m.data_mut() else {
        return Err(Errno::ENOBUFS);
    };
    <G::FileGlue as FileGlue>::uiomove(data, uio)?;

    let slen = {
        let g = smr::read();
        bpf_mfilter(d.bd_wfilter.load(&g), &BpfPkt::mbuf(&m), len)
    };
    if slen < len {
        return Err(Errno::EPERM);
    }

    // The link header out of the packet, into the address.
    if hlen != 0 {
        let data = m.data();
        if linktype == DLT_LOOP {
            // The link header names the address family, in network order.
            kassert!(hlen as usize == size_of::<u32>());
            let af = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            sockp.ss_family = af as u8;
        } else {
            let sa = hal::user::bytes_of_mut(core::slice::from_mut(sockp));
            sa[2..2 + hlen as usize].copy_from_slice(&data[..hlen as usize]);
        }

        m.m_pkthdr().len.set(m.m_pkthdr().len.get() - hlen as i32);
        m.m_len().set(m.m_len().get() - hlen);
        m.data_add(hlen as isize);
    }

    // The data link type, as a tag in front.
    let Some(mut mtag) = m_tag_get(PACKET_TAG_DLT, size_of::<u32>() as i32, M_WAIT) else {
        return Err(Errno::ENOBUFS);
    };
    mtag.data_mut()[..size_of::<u32>()].copy_from_slice(&linktype.to_ne_bytes());
    m_tag_prepend(&mut m, mtag);

    Ok(m)
}

/// `bpf_attachd`: `d` listens on `bp`: it names the tap, joins the head of the interface's
/// chain (its next word set before it is published, as the C's `SMR_SLIST_INSERT_HEAD`), and
/// the driver's word names the tap. With `bd_mtx` held (`s`), under the kernel lock.
fn bpf_attachd(d: &BpfDRef, bp: &BpfIfRef, s: &mut MutexedGuard<'_, BpfSlots>) {
    s.bif = Some(bp.clone());

    let w = klock::witness();
    let r = smr::read();
    drop(d.bd_next.store(&w, bp.bif_dlist.load(&r).map(SmrRef::arc)));
    drop(r);
    drop(bp.bif_dlist.store(&w, Some(d.clone())));

    if !bp.bif_driverp.is_set() {
        drop(bp.bif_driverp.store(&w, Some(bp.clone())));
    }
}

/// `bpf_detachd`: `d` stops listening: off the interface's chain (the predecessor's word
/// given a reference of the successor; `d`'s own next word left, so a tap standing on `d`
/// goes on), the driver's word emptied with the last listener, and a promiscuous listener's
/// `ifpromisc` undone with `bd_mtx` let go (the guard comes back, taken again). Under the
/// kernel lock.
fn bpf_detachd<'a>(d: &'a BpfD, mut s: MutexedGuard<'a, BpfSlots>) -> MutexedGuard<'a, BpfSlots> {
    // Not attached.
    let Some(bp) = s.bif.take() else {
        return s;
    };

    {
        let w = klock::witness();
        let r = smr::read();
        let mut word: &DescWord = &bp.bif_dlist;
        loop {
            match word.load(&r) {
                Some(e) if ptr::eq(e.get(), d) => break,
                Some(e) => word = &e.get().bd_next,
                None => kpanic(format_args!("bpf_detachd: {d:p} not on its tap's list")),
            }
        }
        drop(word.store(&w, d.bd_next.load(&r).map(SmrRef::arc)));
        drop(r);

        if !bp.bif_dlist.is_set() {
            // The driver taps no more.
            drop(bp.bif_driverp.take(&w));
        }
    }

    // A promiscuous listener's mode undone.
    if d.bd_promisc.load(Ordering::Relaxed) {
        let ifp = bp.bif_ifp;
        kassert!(ifp.is_some());

        d.bd_promisc.store(false, Ordering::Relaxed);

        drop(s);
        hal::netlock::lock();
        let error = ifp.map_or(Ok(()), |ifp| ifpromisc(ifp, false));
        hal::netlock::unlock();
        s = d.bd_mtx.lock();

        if let Err(e) = error
            && !matches!(e, Errno::EINVAL | Errno::ENODEV | Errno::ENXIO)
        {
            // The mode went on and will not come off again: a driver broken past repair.
            kpanic(format_args!("bpf: ifpromisc failed"));
        }
    }
    s
}

/// `bpfilterattach`: the pseudo-device's attach (descriptors are made on open): the major of
/// `/dev/bpf` (`bsd`'s scan of `cdevsw`) and the glue whose `vdevgone` revokes a detached
/// tap's listeners.
pub fn bpfilterattach<G: ProcGlue>(_n: i32, maj: u32) {
    let _ = BPF_MAJOR.set(maj);
    let _ = BPF_REVOKE.set(bpf_revoke::<G>);
}

/// `vdevgone(maj, unit, unit, VCHR)`.
fn bpf_revoke<G: ProcGlue>(maj: u32, unit: u32) {
    crate::fs::subr::vdevgone::<G>(maj, unit, unit, VCHR);
}

/// `knote_locked(&d->bd_klist, hint)` for the glue `G`.
fn bpf_knote<G: ProcGlue>(d: &BpfD, s: &MutexedGuard<'_, BpfSlots>, hint: i64) {
    klist_of::<G>(d).knote_locked(s, hint);
}

/// `klist_invalidate(&d->bd_klist)` for the glue `G`, the list's lock `bd_mtx`.
fn bpf_klist_invalidate<G: ProcGlue>(d: &BpfD) {
    klist_of::<G>(d).invalidate_with(|| d.bd_mtx.lock());
}

/// The descriptor's klist, of the glue `G` its box was made for.
fn klist_of<G: ProcGlue>(d: &BpfD) -> &Klist<G> {
    match d.bd_klist.arc.downcast_ref::<KlistBox<G>>() {
        Some(b) => &b.klist,
        None => kpanic(format_args!("bpf: descriptor {d:p} of another glue")),
    }
}

/// `bpfopen`: makes the descriptor of a clone's minor: `ENXIO` for a minor that is not one,
/// `EBUSY` when it cannot be allocated.
pub fn bpfopen<G: ProcGlue>(dev: Dev, _flag: i32, _mode: i32, _p: &Thread<G>) -> Result<(), Errno> {
    let unit = minor(dev);

    if unit & ((1 << CLONE_SHIFT) - 1) != 0 {
        return Err(Errno::ENXIO);
    }
    let unit = unit as i32;

    kassert!(bpfilter_lookup(unit).is_none());

    // created on demand
    let Some(kl) = MArc::new(
        M_DEVBUF,
        M_NOWAIT,
        KlistBox::<G> {
            klist: Klist::owner(),
        },
    ) else {
        return Err(Errno::EBUSY);
    };
    let klist = BpfKlist {
        arc: MArc::erase(kl),
        knote: bpf_knote::<G>,
        invalidate: bpf_klist_invalidate::<G>,
    };
    let bufsize = bpf_bufsize.load(Ordering::Relaxed);
    let Some(d) = MArc::new(M_DEVBUF, M_NOWAIT, BpfD::new(unit, bufsize, klist)) else {
        return Err(Errno::EBUSY);
    };

    d.bd_wake_task
        .set_shared(bpf_wakeup_cb::<G>, MArc::erase(d.clone()));
    d.bd_wait_tmo
        .set_shared(bpf_wait_cb, MArc::erase(d.clone()), KCLOCK_NONE, 0);
    drop(d.bd_self.replace(Some(d.clone())));

    let mut w = klock::witness();
    BPF_D_LIST.insert_head(&mut w, d.as_pin());
    Ok(())
}

/// The descriptor a cdev entry point runs on: `ENXIO` when its unit is not open.
fn bpf_d_of(dev: Dev) -> Result<BpfDRef, Errno> {
    bpfilter_lookup(minor(dev) as i32).ok_or(Errno::ENXIO)
}

/// `bpfclose`: the descriptor detached from its tap, woken, off the list; its handles given
/// back (the list's, the wake task's and the wait timeout's, after their pending work).
pub fn bpfclose<G: ProcGlue>(
    dev: Dev,
    _flag: i32,
    _mode: i32,
    _p: Option<&Thread<G>>,
) -> Result<(), Errno> {
    let d = bpf_d_of(dev)?;
    {
        let s = d.bd_mtx.lock();
        let s = bpf_detachd(&d, s);
        bpf_wakeup(&d, &s, || d.clone());
        let mut w = klock::witness();
        BPF_D_LIST.remove(&mut w, &d);
    }
    drop(d.bd_self.replace(None));

    // The wait timeout and the wake task hold the descriptor (the C's `bpf_get` while they
    // are pending): a pending one does its work now, then both let it go.
    if d.bd_wait_tmo.del_barrier() {
        bpf_wait(&d);
    }
    d.bd_wait_tmo.clear();
    if task_del(&SYSTQ, &d.bd_wake_task) {
        bpf_wakeup_sig::<G>(&d);
    }
    d.bd_wake_task.clear();

    Ok(())
}

/// `ROTATE_BUFFERS(d)`: the store buffer into the hold slot, the free one into the store, the
/// new store empty. With `bd_mtx` held.
fn rotate_buffers(d: &BpfD, s: &mut BpfSlots) {
    kassert!(!s.in_uiomove);
    s.hbuf = s.sbuf.take();
    d.bd_hlen
        .store(d.bd_slen.load(Ordering::Relaxed), Ordering::Relaxed);
    s.sbuf = s.fbuf.take();
    d.bd_state.store(BPF_S_IDLE, Ordering::Relaxed);
    d.bd_slen.store(0, Ordering::Relaxed);
}

/// `bpfread`: the next hold buffer of packets, whole: the read must ask for the buffer's
/// size (`EINVAL`); it waits for the store buffer to fill, for the wait timeout or for the
/// read timeout (`EWOULDBLOCK` then returns what there is, which may be nothing), unless
/// `IO_NDELAY`.
pub fn bpfread<G: ProcGlue>(dev: Dev, uio: &mut Uio<'_, G>, ioflag: i32) -> Result<(), Errno> {
    klock::kernel_assert_locked();

    let d = bpf_d_of(dev)?;
    let mut s = d.bd_mtx.lock();
    if s.bif.is_none() {
        return Err(Errno::ENXIO);
    }

    // The buffer the application reads into is the kernel's size.
    if uio.uio_resid != s.bufsize as usize {
        return Err(Errno::EINVAL);
    }

    // With a read timeout, when the read ends.
    let end = if s.rtout != 0 {
        nsecuptime().saturating_add(s.rtout)
    } else {
        0
    };

    // An empty hold slot: a sleep that ends with the timeout or when enough packets have
    // come to fill the store buffer.
    while s.hbuf.is_none() {
        if s.bif.is_none() {
            // Detached from its tap meanwhile.
            if d.bd_slen.load(Ordering::Relaxed) == 0 {
                return Err(Errno::EIO);
            }
            rotate_buffers(&d, &mut s);
            break;
        }
        if d.bd_state.load(Ordering::Relaxed) == BPF_S_DONE {
            // Packets came since the last read, or while we slept: what is here goes.
            rotate_buffers(&d, &mut s);
            break;
        }
        let error = if ioflag & IO_NDELAY != 0 {
            // A non-blocking read.
            Err(Errno::EWOULDBLOCK)
        } else if s.rtout == 0 {
            // No read timeout.
            s.nreaders += 1;
            let e = s.sleep_nsec(ptr::from_ref::<BpfD>(&d), PRINET | PCATCH, "bpf", INFSLP);
            s.nreaders -= 1;
            e
        } else {
            let now = nsecuptime();
            if now < end {
                // The read timeout has not passed yet.
                s.nreaders += 1;
                let e = s.sleep_nsec(ptr::from_ref::<BpfD>(&d), PRINET | PCATCH, "bpf", end - now);
                s.nreaders -= 1;
                e
            } else {
                // It has.
                Err(Errno::EWOULDBLOCK)
            }
        };
        match error {
            Err(Errno::EINTR | Errno::ERESTART) => return error,
            Err(Errno::EWOULDBLOCK) => {
                // On a timeout what is buffered goes, which may be nothing.
                if s.hbuf.is_some() {
                    // The store buffer filled between the timeout and here: no rotation.
                    break;
                }
                if d.bd_slen.load(Ordering::Relaxed) == 0 {
                    return Ok(());
                }
                rotate_buffers(&d, &mut s);
                break;
            }
            _ => {}
        }
    }

    // The hold slot is full: its buffer leaves the slots for the copy (a rotation of a
    // missing store buffer leaves it empty: nothing to read, as the C's copy of 0 bytes).
    let Some(hbuf) = s.hbuf.take() else {
        return Ok(());
    };
    let hlen = d.bd_hlen.swap(0, Ordering::Relaxed) as usize;
    drop(s.fbuf.take());
    s.in_uiomove = true;

    // The whole buffer goes to user space: the read asked for the kernel's size.
    drop(s);
    let error = <G::FileGlue as FileGlue>::uiomove_out(&hbuf[..hlen.min(hbuf.len())], uio);
    let mut s = d.bd_mtx.lock();

    // No bpf_resetd or rotation took a slot meanwhile.
    kassert!(s.fbuf.is_none());
    kassert!(s.hbuf.is_none());
    s.fbuf = Some(hbuf);
    s.in_uiomove = false;
    error
}

/// `bpf_wakeup`: the sleeping readers woken, the knotes told, and, when the descriptor
/// signals, its wake task queued (`pgsigio` needs the kernel lock, which the taps do not
/// hold); `pin` lends a handle to queue the task with. With `bd_mtx` held.
fn bpf_wakeup(d: &BpfD, s: &MutexedGuard<'_, BpfSlots>, pin: impl FnOnce() -> BpfDRef) {
    if s.nreaders != 0 {
        hal::sleep::wakeup(ptr::from_ref::<BpfD>(d));
    }

    (d.bd_klist.knote)(d, s, 0);

    if d.bd_async.load(Ordering::Relaxed) != 0 && d.bd_sig.load(Ordering::Relaxed) != 0 {
        let h = pin();
        let _ = task_add(&SYSTQ, hal::pin::project(h.as_pin(), |d| &d.bd_wake_task));
    }
}

/// `bpf_wakeup_cb`: the wake task, the deferred `pgsigio`.
fn bpf_wakeup_cb<G: ProcGlue>(a: &AnyArc) {
    if let Some(d) = a.downcast_ref::<BpfD>() {
        bpf_wakeup_sig::<G>(d);
    }
}

/// `bpf_wakeup_cb`'s body: the signal, when the descriptor still asks for it.
fn bpf_wakeup_sig<G: ProcGlue>(d: &BpfD) {
    let sig = d.bd_sig.load(Ordering::Relaxed);
    if d.bd_async.load(Ordering::Relaxed) != 0 && sig != 0 {
        pgsigio::<G>(&d.bd_sigio, sig, false);
    }
}

/// `bpf_wait_cb`: the wait timeout, the end of the wait after the first packet.
fn bpf_wait_cb(a: &AnyArc) {
    if let Some(d) = a.clone().downcast_arc::<BpfD>() {
        bpf_wait(&d);
    }
}

/// `bpf_wait_cb`'s body.
fn bpf_wait(d: &BpfDRef) {
    let s = d.bd_mtx.lock();
    if d.bd_state.load(Ordering::Relaxed) == BPF_S_WAIT {
        d.bd_state.store(BPF_S_DONE, Ordering::Relaxed);
        bpf_wakeup(d, &s, || d.clone());
    }
}

/// `bpfwrite`: the packet a writer hands, sent on the descriptor's interface.
pub fn bpfwrite<G: ProcGlue>(dev: Dev, uio: &mut Uio<'_, G>, _ioflag: i32) -> Result<(), Errno> {
    klock::kernel_assert_locked();

    let d = bpf_d_of(dev)?;
    let Some(bif) = d.bif() else {
        return Err(Errno::ENXIO);
    };

    let Some(ifp) = bif.bif_ifp.filter(|ifp| ifp.if_flags.get() & IFF_UP != 0) else {
        return Err(Errno::ENETDOWN);
    };

    if uio.uio_resid == 0 {
        return Ok(());
    }

    let mut dst = SockaddrStorage::zeroed();
    let m = bpf_movein(uio, &d, &bif, &mut dst)?;

    if m.m_pkthdr().len.get() as u32 > ifp.if_mtu.get() {
        return Err(Errno::EMSGSIZE);
    }

    m.m_pkthdr().ph_rtableid.set(ifp.if_rdomain.get());
    m.m_pkthdr().pf.prio.set(ifp.if_llprio.get());

    if d.bd_hdrcmplt.load(Ordering::Relaxed) && dst.ss_family == AF_UNSPEC {
        dst.ss_family = pseudo_AF_HDRCMPLT;
    }
    // The view the output reads covers the address's data (the module's deviations).
    dst.ss_len = 16;
    let bytes = hal::user::bytes_of(core::slice::from_ref(&dst));
    let Some(view) = SockaddrView::new(bytes) else {
        kpanic(format_args!("bpfwrite: a short destination"));
    };

    hal::netlock::lock();
    let error = ifp.driver().output(ifp, m, view, None);
    hal::netlock::unlock();
    error
}

/// `bpf_resetd`: the descriptor's buffers flushed and its counts cleared. With `bd_mtx` held.
fn bpf_resetd(d: &BpfD, s: &mut BpfSlots) {
    kassert!(!s.in_uiomove);

    // The timeout's own handle stays (the module's counts): a pending one is just taken off.
    let _ = d.bd_wait_tmo.del();

    if let Some(h) = s.hbuf.take() {
        // The hold buffer freed.
        s.fbuf = Some(h);
    }
    d.bd_state.store(BPF_S_IDLE, Ordering::Relaxed);
    d.bd_slen.store(0, Ordering::Relaxed);
    d.bd_hlen.store(0, Ordering::Relaxed);
    d.bd_rcount.store(0, Ordering::Relaxed);
    d.bd_dcount.store(0, Ordering::Relaxed);
}

/// `bpf_set_wtout`: the wait timeout.
fn bpf_set_wtout(d: &BpfD, wtout: u64) {
    d.bd_mtx.lock().wtout = wtout;
}

/// `bpf_set_wtimeout`: `BIOCSWTIMEOUT`, at most 300 seconds.
fn bpf_set_wtimeout(d: &BpfD, tv: &Timeval) -> Result<(), Errno> {
    if tv.tv_sec < 0 || !tv.is_valid() {
        return Err(Errno::EINVAL);
    }

    let nsec = tv.to_nsec();
    if nsec > sec_to_nsec(300) {
        return Err(Errno::EINVAL);
    }
    if nsec > MAXTSLP {
        return Err(Errno::EOVERFLOW);
    }

    bpf_set_wtout(d, nsec);
    Ok(())
}

/// `bpf_get_wtimeout`: `BIOCGWTIMEOUT`; `ENXIO` when no wait timeout is set.
fn bpf_get_wtimeout(d: &BpfD) -> Result<Timeval, Errno> {
    let nsec = d.bd_mtx.lock().wtout;

    if nsec == INFSLP {
        return Err(Errno::ENXIO);
    }

    Ok(Timeval::from_nsec(nsec))
}

/// The ioctl's argument, of type `T` (`*(T *)addr`): `EINVAL` for a request too short.
fn arg<T: AbiPod>(addr: &[u8]) -> Result<T, Errno> {
    hal::pod::at(addr, 0).ok_or(Errno::EINVAL)
}

/// The ioctl's answer (`*(T *)addr = v`): `EINVAL` for a request too short.
fn ret<T: AbiPod>(addr: &mut [u8], v: &T) -> Result<(), Errno> {
    if hal::pod::put(addr, 0, v) {
        Ok(())
    } else {
        Err(Errno::EINVAL)
    }
}

/// The ioctls a locked descriptor (`BIOCLOCK`) still answers; `EPERM` for the others.
const fn allowed_when_locked(cmd: u64) -> bool {
    matches!(
        cmd,
        BIOCGBLEN
            | BIOCFLUSH
            | BIOCGDLT
            | BIOCGDLTLIST
            | BIOCGETIF
            | BIOCGRTIMEOUT
            | BIOCGWTIMEOUT
            | BIOCGSTATS
            | BIOCVERSION
            | BIOCGRSIG
            | BIOCGHDRCMPLT
            | FIONREAD
            | BIOCLOCK
            | BIOCSRTIMEOUT
            | BIOCSWTIMEOUT
            | BIOCDWTIMEOUT
            | BIOCIMMEDIATE
            | TIOCGPGRP
            | BIOCGDIRFILT
    )
}

/// `bpfioctl`: the descriptor's ioctls (`FIONREAD`, the `BIOC*` of `<net/bpf.h>`,
/// `FIOASYNC`, the signal owner's); a locked descriptor answers only the read-only ones and
/// the timeouts (`EPERM`), an unknown request is `EINVAL`.
#[allow(
    clippy::too_many_lines,
    reason = "the C's one switch over the requests, kept as one match"
)]
pub fn bpfioctl<G: ProcGlue>(
    dev: Dev,
    cmd: u64,
    addr: &mut [u8],
    _flag: i32,
    _p: &Thread<G>,
) -> Result<(), Errno> {
    let d = bpf_d_of(dev)?;
    if d.bd_locked.load(Ordering::Relaxed) && !allowed_when_locked(cmd) {
        return Err(Errno::EPERM);
    }

    match cmd {
        // The bytes a read would return now.
        FIONREAD => {
            let n = {
                let s = d.bd_mtx.lock();
                let mut n = d.bd_slen.load(Ordering::Relaxed);
                if s.hbuf.is_some() {
                    n += d.bd_hlen.load(Ordering::Relaxed);
                }
                n
            };
            ret(addr, &n)
        }

        // The buffer length a read must ask for.
        BIOCGBLEN => {
            let size = d.bd_mtx.lock().bufsize as u32;
            ret(addr, &size)
        }

        // The buffer length, before the descriptor is attached: clamped to the bounds, the
        // clamped value given back.
        BIOCSBLEN => {
            let mut s = d.bd_mtx.lock();
            if s.bif.is_some() {
                return Err(Errno::EINVAL);
            }
            let mut size: u32 = arg(addr)?;
            let maxbufsize = bpf_maxbufsize.load(Ordering::Relaxed) as u32;

            if size > maxbufsize {
                size = maxbufsize;
                ret(addr, &size)?;
            } else if size < BPF_MINBUFSIZE as u32 {
                size = BPF_MINBUFSIZE as u32;
                ret(addr, &size)?;
            }
            s.bufsize = size as i32;
            Ok(())
        }

        // The read or write filter.
        BIOCSETF | BIOCSETFNR | BIOCSETWF => bpf_setf(&d, &arg::<BpfProgram>(addr)?, cmd),

        // The buffers flushed.
        BIOCFLUSH => {
            let mut s = d.bd_mtx.lock();
            bpf_resetd(&d, &mut s);
            Ok(())
        }

        // The interface promiscuous.
        BIOCPROMISC => match d.bif() {
            // No interface attached yet.
            None => Err(Errno::EINVAL),
            Some(bif) => match bif.bif_ifp {
                Some(ifp) if !d.bd_promisc.load(Ordering::Relaxed) => {
                    hal::netlock::lock();
                    let error = ifpromisc(ifp, true);
                    hal::netlock::unlock();
                    if error.is_ok() {
                        d.bd_promisc.store(true, Ordering::Relaxed);
                    }
                    error
                }
                _ => Ok(()),
            },
        },

        // The link types of the interface.
        BIOCGDLTLIST => {
            let Some(bif) = d.bif() else {
                return Err(Errno::EINVAL);
            };
            let mut bfl: BpfDltlist = arg(addr)?;
            let error = bpf_getdltlist(&bif, &mut bfl);
            ret(addr, &bfl)?;
            error
        }

        // The link type.
        BIOCGDLT => match d.bif() {
            None => Err(Errno::EINVAL),
            Some(bif) => ret(addr, &bif.bif_dlt),
        },

        // Another link type of the interface (`EINVAL` unattached: `bpf_setdlt`'s).
        BIOCSDLT => bpf_setdlt(&d, arg(addr)?),

        // The interface's name.
        BIOCGETIF => match d.bif() {
            None => Err(Errno::EINVAL),
            Some(bif) => {
                bpf_ifname(&bif, addr);
                Ok(())
            }
        },

        // The interface.
        BIOCSETIF => bpf_setif(&d, addr),

        // The read timeout.
        BIOCSRTIMEOUT => {
            let tv: Timeval = arg(addr)?;
            if tv.tv_sec < 0 || !tv.is_valid() {
                return Err(Errno::EINVAL);
            }
            let rtout = tv.to_nsec();
            if rtout > MAXTSLP {
                return Err(Errno::EOVERFLOW);
            }
            d.bd_mtx.lock().rtout = rtout;
            Ok(())
        }
        BIOCGRTIMEOUT => {
            let tv = Timeval::from_nsec(d.bd_mtx.lock().rtout);
            ret(addr, &tv)
        }

        // The packet counts.
        BIOCGSTATS => {
            let bs = BpfStat {
                bs_recv: d.bd_rcount.load(Ordering::Relaxed) as u32,
                bs_drop: d.bd_dcount.load(Ordering::Relaxed) as u32,
            };
            ret(addr, &bs)
        }

        // Immediate mode: a read returns as soon as a packet comes.
        BIOCIMMEDIATE => {
            let on: i32 = arg(addr)?;
            bpf_set_wtout(&d, if on != 0 { 0 } else { INFSLP });
            Ok(())
        }

        // The wait timeout.
        BIOCSWTIMEOUT => bpf_set_wtimeout(&d, &arg(addr)?),
        BIOCGWTIMEOUT => ret(addr, &bpf_get_wtimeout(&d)?),
        BIOCDWTIMEOUT => {
            bpf_set_wtout(&d, INFSLP);
            Ok(())
        }

        BIOCVERSION => {
            let bv = BpfVersion {
                bv_major: BPF_MAJOR_VERSION,
                bv_minor: BPF_MINOR_VERSION,
            };
            ret(addr, &bv)
        }

        // The "header already complete" flag.
        BIOCGHDRCMPLT => ret(addr, &u32::from(d.bd_hdrcmplt.load(Ordering::Relaxed))),
        BIOCSHDRCMPLT => {
            let on: u32 = arg(addr)?;
            d.bd_hdrcmplt.store(on != 0, Ordering::Relaxed);
            Ok(())
        }

        // Locked (no reset).
        BIOCLOCK => {
            d.bd_locked.store(true, Ordering::Relaxed);
            Ok(())
        }

        // The filter-drop mode.
        BIOCGFILDROP => ret(addr, &u32::from(d.bd_fildrop.load(Ordering::Relaxed))),
        BIOCSFILDROP => {
            let fildrop: u32 = arg(addr)?;
            match u8::try_from(fildrop) {
                Ok(f @ (BPF_FILDROP_PASS | BPF_FILDROP_CAPTURE | BPF_FILDROP_DROP)) => {
                    d.bd_fildrop.store(f, Ordering::Relaxed);
                    Ok(())
                }
                _ => Err(Errno::EINVAL),
            }
        }

        // The direction filter.
        BIOCGDIRFILT => ret(addr, &u32::from(d.bd_dirfilt.load(Ordering::Relaxed))),
        BIOCSDIRFILT => {
            let dirfilt: u32 = arg::<u32>(addr)? & (BPF_DIRECTION_IN | BPF_DIRECTION_OUT);
            d.bd_dirfilt.store(dirfilt as u8, Ordering::Relaxed);
            Ok(())
        }

        // A signal on each packet received.
        FIOASYNC => {
            let on: i32 = arg(addr)?;
            d.bd_async.store(on, Ordering::Relaxed);
            Ok(())
        }

        // The process or group the signal goes to.
        FIOSETOWN | TIOCSPGRP => sigio_setown::<G>(&d.bd_sigio, cmd, arg(addr)?),
        FIOGETOWN | TIOCGPGRP => ret(addr, &sigio_getown(&d.bd_sigio, cmd)),

        // The signal sent on each packet.
        BIOCSRSIG => {
            let sig: u32 = arg(addr)?;
            if sig >= NSIG as u32 {
                return Err(Errno::EINVAL);
            }
            d.bd_sig.store(sig as i32, Ordering::Relaxed);
            Ok(())
        }
        BIOCGRSIG => ret(addr, &(d.bd_sig.load(Ordering::Relaxed) as u32)),

        _ => Err(Errno::EINVAL),
    }
}

/// `bpf_setf`: `d`'s read or write filter replaced by the program `fp` names (none for a
/// NULL `bf_insns` of length 0): copied in, validated, published; the old one released after
/// a grace period; `BIOCSETF` also flushes the buffers. `EINVAL` for a bad request or
/// program, `ENOMEM` when the copy cannot be allocated.
pub fn bpf_setf(d: &BpfD, fp: &BpfProgram, cmd: u64) -> Result<(), Errno> {
    klock::kernel_assert_locked();

    let bps = if fp.bf_insns == 0 {
        if fp.bf_len != 0 {
            return Err(Errno::EINVAL);
        }
        None
    } else {
        let flen = fp.bf_len;
        if flen > BPF_MAXINSNS {
            return Err(Errno::EINVAL);
        }
        let Some(mut fcode) = MArray::new(M_DEVBUF, M_WAITOK | M_CANFAIL, flen as usize, |_| {
            BpfInsn::default()
        }) else {
            return Err(Errno::ENOMEM);
        };
        let maxbufsize = bpf_maxbufsize.load(Ordering::Relaxed) as u32;
        if UserPtr::<BpfInsn>::new(fp.bf_insns as usize)
            .copyin_slice(&mut fcode)
            .is_err()
            || !bpf_validate(&fcode, maxbufsize)
        {
            return Err(Errno::EINVAL);
        }
        let Some(bps) = MArc::new(
            M_DEVBUF,
            M_WAITOK,
            BpfProg {
                insns: fcode,
                smr: SmrArcLink::new(),
            },
        ) else {
            kpanic(format_args!("bpf_setf: malloc(M_WAITOK) failed"));
        };
        Some(bps)
    };

    {
        let w = klock::witness();
        let word = if cmd == BIOCSETWF {
            &d.bd_wfilter
        } else {
            &d.bd_rfilter
        };
        // The old program retired: released after a grace period (`bpf_prog_smr`).
        drop(word.store(&w, bps));
    }

    if cmd == BIOCSETF {
        let mut s = d.bd_mtx.lock();
        bpf_resetd(d, &mut s);
    }

    Ok(())
}

/// Whether two `bif_name`-style names are equal (`strcmp(a, b) == 0` on the NUL-terminated
/// strings, each at most `IFNAMSIZ` bytes).
fn bpf_name_eq(a: &[u8], b: &[u8]) -> bool {
    let cstr = |s: &[u8]| -> usize {
        let s = &s[..min(s.len(), IFNAMSIZ)];
        s.iter().position(|&c| c == 0).unwrap_or(s.len())
    };
    a[..cstr(a)] == b[..cstr(b)]
}

/// `bpf_setif`: `d` detached from its tap (if any) and attached to the one named by the
/// `struct ifreq` in `ifr` (its first `IFNAMSIZ` bytes), its buffers allocated on the first
/// attach, flushed every time: `ENXIO` when no tap has the name, `ENOMEM` without memory for
/// the buffers.
fn bpf_setif(d: &BpfDRef, ifr: &[u8]) -> Result<(), Errno> {
    // The tap of that name.
    let bp = {
        let r = klock::reader();
        BPF_IFLIST
            .iter(&r)
            .find(|bp| bpf_name_eq(&bp.bif_name, ifr))
            .and_then(|bp| bp.bif_self.read().clone())
    };
    let Some(bp) = bp else {
        return Err(Errno::ENXIO);
    };

    // The buffers on the first attach; a new tap only when it is another.
    let mut s = d.bd_mtx.lock();
    if s.sbuf.is_none() {
        bpf_allocbufs(d, &mut s)?;
    }
    if !s.bif.as_ref().is_some_and(|b| MArc::ptr_eq(b, &bp)) {
        s = bpf_detachd(d, s);
        bpf_attachd(d, &bp, &mut s);
    }
    bpf_resetd(d, &mut s);
    Ok(())
}

/// `bpf_ifname`: the tap's name into the `struct ifreq` in `ifr` (its `ifr_name`).
fn bpf_ifname(bif: &BpfIf, ifr: &mut [u8]) {
    let n = min(ifr.len(), IFNAMSIZ);
    ifr[..n].copy_from_slice(&bif.bif_name[..n]);
}

/// `bpfkqfilter`: a knote on the descriptor, `EVFILT_READ` only (`EINVAL` for another
/// filter).
pub fn bpfkqfilter<G: ProcGlue>(dev: Dev, kn: &Knote<G>) -> Result<(), Errno> {
    klock::kernel_assert_locked();

    let Some(d) = bpfilter_lookup(minor(dev) as i32) else {
        return Err(Errno::ENXIO);
    };

    match kn.kn_filter().get() {
        EVFILT_READ => kn.kn_fop.set(Some(&BpfFiltops::<G>::READ)),
        _ => return Err(Errno::EINVAL),
    }

    // kn->kn_hook = d, with the C's bpf_get.
    if kn.set_hook_shared(MArc::erase(d.clone())).is_err() {
        kpanic(format_args!("bpfkqfilter: knote {kn:p} has a hook"));
    }
    let mut s = d.bd_mtx.lock();
    klist_of::<G>(&d).insert_locked(&mut s, kn);
    Ok(())
}

/// `kn->kn_hook` of a bpf knote: its descriptor.
fn kn_bpf<G: ProcGlue>(kn: &Knote<G>) -> &BpfD {
    match kn.hook_shared::<BpfD>() {
        Some(d) => d,
        None => kpanic(format_args!("knote {kn:p}: no bpf descriptor")),
    }
}

impl<G: ProcGlue> Filter<G> for BpfReadFilter {
    const FLAGS: i32 = FILTEROP_ISFD | FILTEROP_MPSAFE;

    /// `filt_bpfrdetach`: off the klist; the hook's handle goes with the knote (`bpf_put`).
    fn detach(kn: &Knote<G>) {
        let d = kn_bpf(kn);
        let mut s = d.bd_mtx.lock();
        klist_of::<G>(d).remove_locked(&mut s, kn);
    }

    /// `filt_bpfread`: readable when the hold buffer has packets, or the store buffer when a
    /// read may return it; `kn_data` the bytes. Called with `bd_mtx` held.
    fn event(kn: &Knote<G>, _hint: i64) -> bool {
        let d = kn_bpf(kn);

        let mut data = i64::from(d.bd_hlen.load(Ordering::Relaxed));
        if d.bd_state.load(Ordering::Relaxed) == BPF_S_DONE {
            data += i64::from(d.bd_slen.load(Ordering::Relaxed));
        }
        kn.kn_data().set(data);

        data > 0
    }

    /// `filt_bpfreadmodify`.
    fn modify(kev: &mut Kevent, kn: &Knote<G>) -> bool {
        let d = kn_bpf(kn);
        let _s = d.bd_mtx.lock();
        knote_modify(kev, kn)
    }

    /// `filt_bpfreadprocess`.
    fn process(kn: &Knote<G>, kev: Option<&mut Kevent>) -> bool {
        let d = kn_bpf(kn);
        let _s = d.bd_mtx.lock();
        knote_process(kn, kev)
    }
}

/// `bpf_mcopy`: the packet's first `dst.len()` bytes into `dst` (`m_copydata`'s walk).
fn bpf_mcopy(src: &BpfPkt<'_>, dst: &mut [u8]) {
    let mut segs = src.segments();
    let mut done = 0;
    while done < dst.len() {
        let Some(seg) = segs.next() else {
            kpanic(format_args!("bpf_mcopy"));
        };
        let count = min(seg.len(), dst.len() - done);
        dst[done..done + count].copy_from_slice(&seg[..count]);
        done += count;
    }
}

/// `bpf_mtap`: an mbuf chain to the listeners of the tap the word `w` names; `true` when a
/// listener asks for the packet to be dropped (`BPF_FILDROP_*`).
#[inline]
pub fn bpf_mtap(w: &BpfTap, m: &Mbuf, direction: u32) -> bool {
    _bpf_mtap(w, Some(m), BpfPkt::mbuf(m), direction)
}

/// `_bpf_mtap`: `m` to the listeners of the tap `w` names, inside one read section: each
/// listener counts it, skips it by direction, runs its read filter and captures what the
/// filter keeps; `mp` is the mbuf with the packet header (`None` for `bpf_tap_hdr`'s
/// buffers). `true` when a listener asks for the packet to be dropped.
pub fn _bpf_mtap(w: &BpfTap, mp: Option<&Mbuf>, m: BpfPkt<'_>, direction: u32) -> bool {
    let g = smr::read();
    let Some(bp) = w.load(&g) else {
        return false;
    };

    let pktlen = m.len();
    let mut tbh: Option<BpfHdr> = None;
    let mut drop = false;

    let mut next = bp.bif_dlist.load(&g);
    while let Some(dref) = next {
        let d = dref.get();
        next = d.bd_next.load(&g);

        d.bd_rcount.fetch_add(1, Ordering::Relaxed);

        if u32::from(d.bd_dirfilt.load(Ordering::Relaxed)) & direction != 0 {
            continue;
        }

        let slen = bpf_mfilter(d.bd_rfilter.load(&g), &m, pktlen as u32);
        if slen == 0 {
            continue;
        }
        let fildrop = d.bd_fildrop.load(Ordering::Relaxed);
        if fildrop != BPF_FILDROP_PASS {
            drop = true;
        }
        if fildrop != BPF_FILDROP_DROP {
            let hdr = *tbh.get_or_insert_with(|| bpf_tap_bh(mp, direction));

            let mut s = d.bd_mtx.lock();
            bpf_catchpacket(dref, &mut s, &m, pktlen, slen as usize, &hdr);
        }
    }

    drop
}

/// The `struct bpf_hdr` template `_bpf_mtap` fills once per packet (`gothdr`).
fn bpf_tap_bh(mp: Option<&Mbuf>, direction: u32) -> BpfHdr {
    let mut tbh = BpfHdr::default();

    let tv = if let Some(m) = mp
        && let Some(ph) = m.pkthdr()
    {
        tbh.bh_ifidx = ph.ph_ifidx.get() as u16;
        tbh.bh_flowid = ph.ph_flowid.get();
        tbh.bh_flags = ph.pf.prio.get();
        if ph.csum_flags.get() & M_FLOWID != 0 {
            tbh.bh_flags |= BPF_F_FLOWID;
        }
        tbh.bh_csumflags = ph.csum_flags.get();

        m_microtime(m)
    } else {
        microtime()
    };

    tbh.bh_tstamp.tv_sec = tv.tv_sec as u32;
    tbh.bh_tstamp.tv_usec = tv.tv_usec as u32;
    tbh.bh_flags |= (direction << BPF_F_DIR_SHIFT) as u8;

    tbh
}

/// `bpf_tap_hdr`: a driver's packet as a header and a buffer, either missing (both: nothing
/// tapped, the C's NULL chain).
pub fn bpf_tap_hdr(w: &BpfTap, hdr: Option<&[u8]>, buf: Option<&[u8]>, direction: u32) -> bool {
    if hdr.is_none() && buf.is_none() {
        return false;
    }
    let pkt = BpfPkt::hdr_buf(hdr.unwrap_or(&[]), buf.unwrap_or(&[]));
    _bpf_mtap(w, None, pkt, direction)
}

/// `bpf_mtap_hdr`: an mbuf chain behind a header from a linear buffer.
pub fn bpf_mtap_hdr(w: &BpfTap, data: &[u8], m: &Mbuf, direction: u32) -> bool {
    _bpf_mtap(w, Some(m), BpfPkt::hdr_mbuf(data, m), direction)
}

/// `bpf_mtap_af`: an mbuf chain behind its address family (four bytes, network order).
pub fn bpf_mtap_af(w: &BpfTap, af: u32, m: &Mbuf, direction: u32) -> bool {
    let afh = af.to_be_bytes();

    bpf_mtap_hdr(w, &afh, m, direction)
}

/// `bpf_mtap_ether`: an Ethernet frame, whose 802.1Q header the C puts back in front of a
/// packet with `M_VLANTAG`; vlan(4) is not configured (`NVLAN` 0), so the packet goes as it
/// is.
#[inline]
pub fn bpf_mtap_ether(w: &BpfTap, m: &Mbuf, direction: u32) -> bool {
    _bpf_mtap(w, Some(m), BpfPkt::mbuf(m), direction)
}

/// `bpf_catchpacket`: the packet into the store buffer behind its `bpf_hdr`, rotating when
/// it does not fit (dropped without a free buffer), and the readers woken as the wait mode
/// says. With `bd_mtx` held; `dref` lends a handle to arm the timeout and queue the task.
fn bpf_catchpacket(
    dref: SmrRef<'_, BpfD, BpfDRef>,
    s: &mut MutexedGuard<'_, BpfSlots>,
    pkt: &BpfPkt<'_>,
    pktlen: usize,
    snaplen: usize,
    tbh: &BpfHdr,
) {
    let d = dref.get();
    let mut do_wakeup = false;

    let Some(hdrlen) = s.bif.as_ref().map(|b| b.bif_hdrlen as usize) else {
        return;
    };
    let bufsize = usize::try_from(s.bufsize).unwrap_or(0);

    // What to move: the snapshot length's worth, or the whole packet, within the buffer.
    let totlen = min(hdrlen + min(snaplen, pktlen), bufsize);

    // The end of the previous packet rounded up to a longword.
    let mut curlen = bpf_wordalign(usize::try_from(d.bd_slen.load(Ordering::Relaxed)).unwrap_or(0));
    if curlen + totlen > bufsize {
        // The packet overflows the store buffer: the buffers rotated if they can be, the
        // readers woken.
        if s.fbuf.is_none() {
            // The previous read has not finished: the packet dropped.
            d.bd_dcount.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // The pending wait cancelled.
        let _ = d.bd_wait_tmo.del();

        rotate_buffers(d, s);
        do_wakeup = true;
        curlen = 0;
    }

    let Some(rec) = s
        .sbuf
        .as_mut()
        .and_then(|b| b.get_mut(curlen..curlen + totlen))
        .filter(|rec| rec.len() >= hdrlen)
    else {
        // No store buffer (the module's deviations): the packet dropped.
        d.bd_dcount.fetch_add(1, Ordering::Relaxed);
        return;
    };

    // The bpf header, then the packet's bytes.
    let mut bh = *tbh;
    bh.bh_datalen = pktlen as u32;
    bh.bh_hdrlen = hdrlen as u16;
    bh.bh_caplen = (totlen - hdrlen) as u32;
    let _ = hal::pod::put(rec, 0, &bh);
    bpf_mcopy(pkt, &mut rec[hdrlen..]);
    d.bd_slen.store((curlen + totlen) as i32, Ordering::Relaxed);

    match s.wtout {
        0 => {
            // Immediate mode: a packet came, the readers go.
            if d.bd_state.load(Ordering::Relaxed) == BPF_S_IDLE {
                d.bd_state.store(BPF_S_DONE, Ordering::Relaxed);
            }
            do_wakeup = true;
        }
        INFSLP => {}
        wtout => {
            if d.bd_state.load(Ordering::Relaxed) == BPF_S_IDLE {
                d.bd_state.store(BPF_S_WAIT, Ordering::Relaxed);

                let h = dref.arc();
                let _ = hal::pin::project(h.as_pin(), |d| &d.bd_wait_tmo).add_nsec(wtout);
            }
        }
    }

    if do_wakeup {
        bpf_wakeup(d, s, || dref.arc());
    }
}

/// `bpf_allocbufs`: the store and free buffers. With `bd_mtx` held.
fn bpf_allocbufs(d: &BpfD, s: &mut BpfSlots) -> Result<(), Errno> {
    let size = usize::try_from(s.bufsize).unwrap_or(0);
    let Some(fbuf) = MArray::new(M_DEVBUF, M_NOWAIT, size, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };
    let Some(sbuf) = MArray::new(M_DEVBUF, M_NOWAIT, size, |_| 0u8) else {
        return Err(Errno::ENOMEM);
    };
    s.fbuf = Some(fbuf);
    s.sbuf = Some(sbuf);

    d.bd_slen.store(0, Ordering::Relaxed);
    d.bd_hlen.store(0, Ordering::Relaxed);

    Ok(())
}

/// The tap of `name` (at most `IFNAMSIZ` bytes), of the link type `dlt` with `hdrlen` bytes of
/// link header, on the driver's word `w`, for the interface `ifp` or none.
fn bpf_ifattach(
    w: &'static BpfTap,
    name: &[u8],
    ifp: Option<&'static Ifnet>,
    dlt: u32,
    hdrlen: u32,
) -> BpfIfRef {
    let mut bif_name = [0u8; IFNAMSIZ];
    let n = name
        .iter()
        .take(IFNAMSIZ)
        .position(|&c| c == 0)
        .unwrap_or(min(name.len(), IFNAMSIZ));
    bif_name[..n].copy_from_slice(&name[..n]);

    let Some(bp) = MArc::new(
        M_DEVBUF,
        M_NOWAIT,
        BpfIf {
            bif_link: PinEntry::new(),
            bif_self: ReadMostly::new(None),
            bif_dlist: DescWord::new(),
            bif_driverp: w,
            bif_dlt: dlt,
            // The bpf header with the padding that puts the network header after the link
            // header on a longword (for speed, and for the machines that need it).
            bif_hdrlen: (bpf_wordalign(hdrlen as usize + SIZEOF_BPF_HDR) - hdrlen as usize) as u32,
            bif_name,
            bif_ifp: ifp,
            smr: SmrArcLink::new(),
        },
    ) else {
        kpanic(format_args!("bpfattach"));
    };
    drop(bp.bif_self.replace(Some(bp.clone())));

    // The list is the kernel lock's: the C's attaches hold it (autoconf, a cloner's create),
    // which it takes again here (`bsd`'s host tests attach without it).
    klock::kernel_lock();
    {
        let mut kw = klock::witness();
        BPF_IFLIST.insert_tail(&mut kw, bp.as_pin());

        drop(w.take(&kw));
    }
    klock::kernel_unlock();

    bp
}

/// `bpfsattach`: a tap named `name` (at most `IFNAMSIZ` bytes) on the driver's word `w`, for
/// the link type `dlt` with `hdrlen` bytes of link header, of no interface.
pub fn bpfsattach(w: &'static BpfTap, name: &[u8], dlt: u32, hdrlen: u32) -> BpfIfRef {
    bpf_ifattach(w, name, None, dlt, hdrlen)
}

/// `bpfxattach`: `bpfsattach` for the interface `ifp`.
pub fn bpfxattach(
    w: &'static BpfTap,
    name: &[u8],
    ifp: &'static Ifnet,
    dlt: u32,
    hdrlen: u32,
) -> BpfIfRef {
    bpf_ifattach(w, name, Some(ifp), dlt, hdrlen)
}

/// `bpfattach`: the interface `ifp` attached under its own name; `w` is its `if_bpf` (or
/// another word of the driver's).
pub fn bpfattach(w: &'static BpfTap, ifp: &'static Ifnet, dlt: u32, hdrlen: u32) {
    drop(bpfxattach(w, &ifp.if_xname.get(), ifp, dlt, hdrlen));
}

/// `bpfdetach`: the interface's taps detached, its `if_bpf` emptied. Under the kernel lock.
pub fn bpfdetach(ifp: &Ifnet) {
    klock::kernel_assert_locked();

    // The next tap of the interface, from the head each time: bpfsdetach sleeps.
    loop {
        let bp = {
            let r = klock::reader();
            BPF_IFLIST
                .iter(&r)
                .find(|bp| bp.bif_ifp.is_some_and(|i| ptr::eq(i, ifp)))
                .and_then(|bp| bp.bif_self.read().clone())
        };
        let Some(bp) = bp else {
            break;
        };
        bpfsdetach(&bp);
    }

    let w = klock::witness();
    drop(ifp.if_bpf.take(&w));
}

/// `bpfsdetach`: a tap detached: every descriptor listening on it revoked (its device's vnode
/// gone, which closes it) and its knotes made EOF, then off the list. The block goes once
/// the driver's word and every tap that saw it let it go. Under the kernel lock.
pub fn bpfsdetach(bp: &BpfIf) {
    klock::kernel_assert_locked();

    loop {
        let first = {
            let g = smr::read();
            bp.bif_dlist.load(&g).map(SmrRef::arc)
        };
        let Some(bd) = first else {
            break;
        };
        let (Some(&maj), Some(&revoke)) = (BPF_MAJOR.get(), BPF_REVOKE.get()) else {
            kpanic(format_args!(
                "bpfsdetach: no device to revoke listeners through"
            ));
        };
        let unit = bd.bd_unit as u32;
        revoke(maj, unit);
        (bd.bd_klist.invalidate)(&bd);
        drop(bd);
    }

    {
        let mut w = klock::witness();
        BPF_IFLIST.remove(&mut w, bp);
    }
    drop(bp.bif_self.replace(None));
}

/// `bpf_sysctl`: `net.bpf.bufsize` (`[BPF_MINBUFSIZE, net.bpf.maxbufsize]`) and
/// `net.bpf.maxbufsize` (`[BPF_MINBUFSIZE, MALLOC_MAX]`); `ENOTDIR` below a leaf,
/// `EOPNOTSUPP` for another name.
pub fn bpf_sysctl(name: &[i32], req: &mut SysctlReq<'_>) -> Result<(), Errno> {
    let [leaf] = name else {
        return Err(Errno::ENOTDIR);
    };

    match *leaf {
        NET_BPF_BUFSIZE => int_bounded(
            req,
            &bpf_bufsize,
            BPF_MINBUFSIZE,
            bpf_maxbufsize.load(Ordering::Relaxed),
        ),
        NET_BPF_MAXBUFSIZE => int_bounded(req, &bpf_maxbufsize, BPF_MINBUFSIZE, MALLOC_MAX),
        _ => Err(Errno::EOPNOTSUPP),
    }
}

/// `bpfilter_lookup`: the open descriptor of `unit`, with a reference (the C's `bpf_get` its
/// callers make next). Under the kernel lock.
pub fn bpfilter_lookup(unit: i32) -> Option<BpfDRef> {
    let r = klock::reader();
    BPF_D_LIST
        .iter(&r)
        .find(|bd| bd.bd_unit == unit)
        .and_then(|bd| bd.bd_self.read().clone())
}

/// The link types `bpf_getdltlist` copies out per hold of the kernel lock's reader (the copy
/// may fault and sleep, which the reader must not span).
const DLT_BATCH: usize = 16;

/// `bpf_getdltlist`: the link types of the taps named as `bif` (its interface's), copied to
/// `bfl_list` one by one when it is not NULL (`ENOMEM` past `bfl_len` of them), their count
/// in `bfl_len`; a copy's fault stops it there.
fn bpf_getdltlist(bif: &BpfIf, bfl: &mut BpfDltlist) -> Result<(), Errno> {
    let name = &bif.bif_name;
    let mut n: u32 = 0;
    loop {
        // The next batch of matches, past the `n` seen.
        let mut batch = [0u32; DLT_BATCH];
        let mut got = 0;
        let mut more = false;
        {
            let r = klock::reader();
            let mut skip = n as usize;
            for bp in BPF_IFLIST.iter(&r) {
                if !bpf_name_eq(name, &bp.bif_name) {
                    continue;
                }
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                if got == DLT_BATCH {
                    more = true;
                    break;
                }
                batch[got] = bp.bif_dlt;
                got += 1;
            }
        }
        for dlt in &batch[..got] {
            if bfl.bfl_list != 0 {
                if n >= bfl.bfl_len {
                    return Err(Errno::ENOMEM);
                }
                let at = bfl.bfl_list as usize + n as usize * size_of::<u32>();
                if let Err(e) = UserPtr::<u32>::new(at).copyout(dlt) {
                    bfl.bfl_len = n;
                    return Err(e);
                }
            }
            n += 1;
        }
        if !more {
            break;
        }
    }

    bfl.bfl_len = n;
    Ok(())
}

/// `bpf_setdlt`: the descriptor moved to the tap of its interface's name with the link type
/// `dlt` (`EINVAL` when there is none); flushed.
fn bpf_setdlt(d: &BpfDRef, dlt: u32) -> Result<(), Errno> {
    let mut s = d.bd_mtx.lock();
    let Some(bif) = s.bif.clone() else {
        return Err(Errno::EINVAL);
    };
    if bif.bif_dlt == dlt {
        return Ok(());
    }
    let bp = {
        let r = klock::reader();
        BPF_IFLIST
            .iter(&r)
            .find(|bp| bpf_name_eq(&bif.bif_name, &bp.bif_name) && bp.bif_dlt == dlt)
            .and_then(|bp| bp.bif_self.read().clone())
    };
    let Some(bp) = bp else {
        return Err(Errno::EINVAL);
    };
    s = bpf_detachd(d, s);
    bpf_attachd(d, &bp, &mut s);
    bpf_resetd(d, &mut s);
    Ok(())
}

/// `bpf_mbuf_copy`: `buf.len()` bytes at offset `off` of the packet into `buf`; `false` (the
/// C's -1) when the packet is too short.
fn bpf_mbuf_copy(m: &BpfPkt<'_>, off: u32, buf: &mut [u8]) -> bool {
    let mut off = off as usize;
    let mut done = 0;
    for seg in m.segments() {
        if off >= seg.len() {
            off -= seg.len();
            continue;
        }
        let count = min(seg.len() - off, buf.len() - done);
        buf[done..done + count].copy_from_slice(&seg[off..off + count]);
        done += count;
        if done == buf.len() {
            return true;
        }
        off = 0;
    }
    false
}

/// `bpf_mbuf_ops`: the loads `bpf_mfilter` runs a program with.
impl BpfOps for BpfPkt<'_> {
    /// `bpf_mbuf_ldw`: the 32-bit word at `k`, in host order.
    fn ldw(&self, k: u32) -> Option<u32> {
        let mut v = [0u8; 4];
        bpf_mbuf_copy(self, k, &mut v).then(|| u32::from_be_bytes(v))
    }

    /// `bpf_mbuf_ldh`: the 16-bit half-word at `k`, in host order.
    fn ldh(&self, k: u32) -> Option<u32> {
        let mut v = [0u8; 2];
        bpf_mbuf_copy(self, k, &mut v).then(|| u32::from(u16::from_be_bytes(v)))
    }

    /// `bpf_mbuf_ldb`: the byte at `k`.
    fn ldb(&self, k: u32) -> Option<u32> {
        let mut v = [0u8; 1];
        bpf_mbuf_copy(self, k, &mut v).then(|| u32::from(v[0]))
    }
}

/// `bpf_mfilter`: the program `bf` run over the packet `m` of `wirelen` bytes: how many bytes
/// to capture (`u32::MAX` for all, no program), 0 to reject.
pub fn bpf_mfilter(
    bf: Option<SmrRef<'_, BpfProg, BpfProgRef>>,
    m: &BpfPkt<'_>,
    wirelen: u32,
) -> u32 {
    match bf {
        // No filter accepts all.
        None => u32::MAX,
        Some(p) => _bpf_lfilter(Some(&p.get().insns), m, wirelen),
    }
}

/// Host tests: the sysctl values back to their defaults (the lists are left: their elements
/// live in memory no test frees, and every test detaches and closes what it made).
#[cfg(any(test, feature = "doubles"))]
pub fn bpf_test_reset() {
    bpf_bufsize.store(BPF_BUFSIZE, Ordering::Relaxed);
    bpf_maxbufsize.store(BPF_MAXBUFSIZE, Ordering::Relaxed);
}

const _: () = {
    assert!(SIZEOF_BPF_HDR == 28);
    assert!(MALLOC_MAX > BPF_MAXBUFSIZE);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    // Host tests of the device and the taps (LZ's re-expressed, over the kernel's interface
    // doubles and the user-copy double): the loads over a packet's segments, the names, a
    // capture through a descriptor (open, attach, the ioctls, the tap, the read, the filters,
    // the counters, the drop and direction modes, the lock, the close), the store and hold
    // buffers' rotation and drop, the locked descriptor's ioctls, `BIOCSETF`'s bounds and the
    // old program's retirement, the link types, a tap's detach revoking its listeners, the
    // write path, the sysctl bounds, and a tap racing a close.

    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::thread;
    use std::vec;
    use std::vec::Vec;

    use hal::smr::CpuWalk;
    use hal::sys::net_bpf::{BPF_F_DIR_IN, BPF_F_DIR_MASK, BPF_K, BPF_RET, bpf_stmt};
    use hal::sys::types::makedev;

    use super::*;
    use crate::net::if_::{if_test_reset, test_ifnet, test_ifnet_with};
    use crate::net::if_var::NetDriver;
    use crate::net::mbuf::{M_DONTWAIT, m_cat, m_get};
    use crate::net::unix::tests::G;
    use crate::testutil::{USER_BASE, USER_LEN, user_bytes};
    use crate::uio::{Iovec, UioRw, UioSeg};

    /// The tests' user memory (the user-copy double's map, `hal/arch/host/copy.rs`): a program
    /// at `UPROG`, a read or write buffer of up to 8 KiB at `UBUF`, words at `UWORD`.
    const UPROG: usize = USER_BASE + 0x2_8000;
    const UBUF: usize = USER_BASE + 0x2_9000;
    const UWORD: usize = USER_BASE + 0x2_b000;

    /// The units the tests' revoke closed (`bpfsdetach`'s `vdevgone`, whose `spec_close` is
    /// `bpfclose`).
    static REVOKED: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    fn revoked() -> MutexGuard<'static, Vec<u32>> {
        REVOKED.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn test_revoke(maj: u32, unit: u32) {
        revoked().push(unit);
        let _ = bpfclose::<G>(makedev(maj, unit), 0, 0, None);
    }

    /// A thread, the mbufs and sockets, the interfaces, the kernel lock (the cdev entry
    /// points' context), bpf's defaults, and the revoke above for the major 23.
    fn setup() -> (MutexGuard<'static, ()>, &'static Thread<G>) {
        let (g, p) = crate::net::unix::tests::setup();
        if_test_reset();
        hal::klock::kernel_lock();
        bpf_test_reset();
        let _ = BPF_MAJOR.set(23);
        let _ = BPF_REVOKE.set(test_revoke);
        (g, p)
    }

    /// The clone `n` of `/dev/bpf`.
    fn dev(n: u32) -> Dev {
        makedev(23, n << CLONE_SHIFT)
    }

    /// `ioctl(2)` on the descriptor with an argument of type `T`, given and read back.
    fn ioctl<T: AbiPod>(d: Dev, p: &Thread<G>, cmd: u64, arg: &mut T) -> Result<(), Errno> {
        let mut data = vec![0u8; size_of::<T>().max(32)];
        assert!(hal::pod::put(&mut data, 0, arg));
        let r = bpfioctl::<G>(d, cmd, &mut data, 0, p);
        *arg = hal::pod::at(&data, 0).expect("the answer");
        r
    }

    /// `BIOCSETIF` of the interface `name`.
    fn setif(d: Dev, p: &Thread<G>, name: &[u8]) -> Result<(), Errno> {
        let mut ifr = [0u8; 32];
        ifr[..name.len()].copy_from_slice(name);
        bpfioctl::<G>(d, BIOCSETIF, &mut ifr, 0, p)
    }

    /// A user uio of `len` bytes at `UBUF`.
    fn uio<'a>(iov: &'a mut [Iovec; 1], p: &'a Thread<G>, rw: UioRw, len: usize) -> Uio<'a, G> {
        iov[0] = Iovec {
            iov_base: ptr::without_provenance_mut(UBUF),
            iov_len: len,
        };
        Uio {
            uio_iov: iov,
            uio_offset: 0,
            uio_resid: len,
            uio_segflg: UioSeg::UIO_USERSPACE,
            uio_rw: rw,
            uio_procp: Some(p),
        }
    }

    /// `read(2)` of `len` bytes, non-blocking: the answer and the bytes read.
    fn read(d: Dev, p: &Thread<G>, len: usize) -> (Result<(), Errno>, Vec<u8>) {
        let mut iov = [Iovec::new()];
        let mut u = uio(&mut iov, p, UioRw::UIO_READ, len);
        let r = bpfread::<G>(d, &mut u, IO_NDELAY);
        let got = len - u.uio_resid;
        (r, user_bytes(UBUF, got).expect("the buffer"))
    }

    /// A packet holding `bytes`, in one mbuf.
    fn packet(bytes: &[u8]) -> MbufBox {
        let Some(mut m) = m_gethdr(M_DONTWAIT, MT_DATA) else {
            panic!("an mbuf");
        };
        m.m_len().set(bytes.len() as u32);
        m.m_pkthdr().len.set(bytes.len() as i32);
        m.data_mut().expect("writable").copy_from_slice(bytes);
        m
    }

    /// The program `prog` copied to `UPROG`, as `BIOCSETF` names it.
    fn program(prog: &[BpfInsn]) -> BpfProgram {
        UserPtr::<BpfInsn>::new(UPROG)
            .copyout_slice(prog)
            .expect("the program in user memory");
        BpfProgram {
            bf_len: prog.len() as u32,
            _pad0: 0,
            bf_insns: UPROG as u64,
        }
    }

    /// No CPU but the caller's: a grace period waits for no other.
    struct Alone;

    impl CpuWalk for Alone {
        fn running(&self, _n: usize) -> bool {
            false
        }
        fn peg(&self, n: usize) {
            panic!("peg({n}) with no other CPU");
        }
        fn unpeg(&self) {}
    }

    /// The releases queued on this CPU, run after a grace period.
    fn grace() {
        hal::smr::idle();
        let _ = hal::smr::collect().run(&Alone);
    }

    #[test]
    fn loads_across_segments() {
        let _g = setup();
        let bytes: Vec<u8> = (0u8..20).collect();
        let mut m = packet(&bytes[..6]);
        let mut n = m_get(M_DONTWAIT, MT_DATA).expect("mbuf");
        n.m_len().set(14);
        n.data_mut().expect("writable").copy_from_slice(&bytes[6..]);
        m_cat(&mut m, Some(n));
        let pkt = BpfPkt::mbuf(&m);
        assert_eq!(pkt.len(), 20);
        assert_eq!(pkt.ldw(4), Some(0x0405_0607));
        assert_eq!(pkt.ldh(5), Some(0x0506));
        assert_eq!(pkt.ldb(19), Some(19));
        assert_eq!(pkt.ldb(20), None);
        assert_eq!(pkt.ldw(17), None);

        // bpf_mtap_af's header in front of the chain.
        let afh = 2u32.to_be_bytes();
        let pkt = BpfPkt::hdr_mbuf(&afh, &m);
        assert_eq!(pkt.ldw(0), Some(2));
        assert_eq!(pkt.ldw(2), Some(0x0002_0001));
        let mut all = [0u8; 24];
        bpf_mcopy(&pkt, &mut all);
        assert_eq!(&all[4..], &bytes[..]);

        // bpf_tap_hdr's two linear buffers.
        let pkt = BpfPkt::hdr_buf(&[0xaa, 0xbb], &[1, 2, 3]);
        assert_eq!(pkt.ldw(1), Some(0xbb01_0203));
        assert_eq!(pkt.len(), 5);
    }

    #[test]
    fn names() {
        assert!(bpf_name_eq(
            b"vio0\0\0\0\0\0\0\0\0\0\0\0\0",
            b"vio0\0garbage"
        ));
        assert!(!bpf_name_eq(b"vio0", b"vio1"));
        assert!(!bpf_name_eq(b"vio", b"vio0"));
        assert!(bpf_name_eq(
            b"abcdefghijklmnop-past-ifnamsiz",
            b"abcdefghijklmnop"
        ));
    }

    #[test]
    fn capture_through_a_descriptor() {
        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpftest0");
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        assert!(!ifp.if_bpf.is_set(), "no listener yet");

        let d = dev(7);
        assert_eq!(bpfopen::<G>(makedev(23, 1), 0, 0, p), Err(Errno::ENXIO));
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");

        // The buffer size, before the attach: clamped and given back.
        let mut size = 10u32;
        ioctl(d, p, BIOCSBLEN, &mut size).expect("BIOCSBLEN");
        assert_eq!(size, BPF_MINBUFSIZE as u32);
        size = u32::MAX;
        ioctl(d, p, BIOCSBLEN, &mut size).expect("BIOCSBLEN");
        assert_eq!(size, BPF_MAXBUFSIZE as u32);
        size = 4096;
        ioctl(d, p, BIOCSBLEN, &mut size).expect("BIOCSBLEN");
        assert_eq!(size, 4096);

        assert_eq!(setif(d, p, b"nosuch0"), Err(Errno::ENXIO));
        assert_eq!(setif(d, p, b"bpftest0"), Ok(()));
        assert!(
            ifp.if_bpf.is_set(),
            "the listener points the driver at the tap"
        );
        assert_eq!(
            ioctl(d, p, BIOCSBLEN, &mut size),
            Err(Errno::EINVAL),
            "not once attached"
        );
        let mut name = [0u8; 32];
        assert_eq!(bpfioctl::<G>(d, BIOCGETIF, &mut name, 0, p), Ok(()));
        assert_eq!(&name[..9], b"bpftest0\0");
        let mut one = 1i32;
        ioctl(d, p, BIOCIMMEDIATE, &mut one).expect("BIOCIMMEDIATE");
        let mut blen = 0u32;
        ioctl(d, p, BIOCGBLEN, &mut blen).expect("BIOCGBLEN");
        assert_eq!(blen, 4096);
        let mut dlt = 0u32;
        ioctl(d, p, BIOCGDLT, &mut dlt).expect("BIOCGDLT");
        assert_eq!(dlt, DLT_EN10MB);

        // A frame comes in: captured behind a bpf_hdr whose length aligns the IP header.
        let frame: Vec<u8> = (0u8..60).collect();
        let m = packet(&frame);
        m.m_pkthdr().ph_ifidx.set(3);
        assert!(!bpf_mtap_ether(&ifp.if_bpf, &m, BPF_DIRECTION_IN));
        let mut n = 0i32;
        ioctl(d, p, FIONREAD, &mut n).expect("FIONREAD");
        assert_eq!(n, 30 + 60);
        let (r, buf) = read(d, p, blen as usize);
        assert_eq!(r, Ok(()));
        let bh: BpfHdr = hal::pod::at(&buf, 0).expect("a header");
        assert_eq!((bh.bh_caplen, bh.bh_datalen, bh.bh_hdrlen), (60, 60, 30));
        assert_eq!(bh.bh_ifidx, 3);
        assert_eq!(bh.bh_flags & BPF_F_DIR_MASK, BPF_F_DIR_IN);
        assert_eq!(&buf[30..90], &frame[..]);
        // Nothing more: a non-blocking read returns nothing, successfully.
        let (r, buf) = read(d, p, blen as usize);
        assert_eq!((r, buf.len()), (Ok(()), 0));
        assert_eq!(
            read(d, p, 100).0,
            Err(Errno::EINVAL),
            "not the buffer's size"
        );

        // A read filter that keeps 4 bytes, then one that rejects; the counters see both.
        let mut prog = program(&[bpf_stmt(BPF_RET | BPF_K, 4)]);
        ioctl(d, p, BIOCSETF, &mut prog).expect("BIOCSETF");
        assert!(!bpf_mtap_ether(&ifp.if_bpf, &m, BPF_DIRECTION_IN));
        let (_, buf) = read(d, p, blen as usize);
        let bh: BpfHdr = hal::pod::at(&buf, 0).expect("a header");
        assert_eq!((bh.bh_caplen, bh.bh_datalen), (4, 60));
        let mut prog = program(&[bpf_stmt(BPF_RET | BPF_K, 0)]);
        ioctl(d, p, BIOCSETFNR, &mut prog).expect("BIOCSETFNR");
        assert!(!bpf_mtap_ether(&ifp.if_bpf, &m, BPF_DIRECTION_IN));
        let mut st = BpfStat::default();
        ioctl(d, p, BIOCGSTATS, &mut st).expect("BIOCGSTATS");
        assert_eq!(
            st,
            BpfStat {
                bs_recv: 2,
                bs_drop: 0
            }
        );
        let mut prog = program(&[
            bpf_stmt(hal::sys::net_bpf::BPF_LD | hal::sys::net_bpf::BPF_MEM, 99),
            bpf_stmt(BPF_RET | BPF_K, 0),
        ]);
        assert_eq!(ioctl(d, p, BIOCSETF, &mut prog), Err(Errno::EINVAL));

        // Without a filter, a listener with BPF_FILDROP_DROP drops the packet uncaptured.
        let mut none = BpfProgram::default();
        ioctl(d, p, BIOCSETF, &mut none).expect("BIOCSETF NULL");
        let mut drop_mode = u32::from(BPF_FILDROP_DROP);
        ioctl(d, p, BIOCSFILDROP, &mut drop_mode).expect("BIOCSFILDROP");
        assert!(bpf_mtap_ether(&ifp.if_bpf, &m, BPF_DIRECTION_IN));
        let mut bad = 3u32;
        assert_eq!(ioctl(d, p, BIOCSFILDROP, &mut bad), Err(Errno::EINVAL));
        // The direction filter skips outgoing packets.
        let mut out = BPF_DIRECTION_OUT;
        ioctl(d, p, BIOCSDIRFILT, &mut out).expect("BIOCSDIRFILT");
        assert!(!bpf_mtap_ether(&ifp.if_bpf, &m, BPF_DIRECTION_OUT));
        drop(m);

        // Locked: only the read-only ioctls remain.
        ioctl(d, p, BIOCLOCK, &mut one).expect("BIOCLOCK");
        assert_eq!(ioctl(d, p, BIOCSFILDROP, &mut drop_mode), Err(Errno::EPERM));
        let mut v = BpfVersion::default();
        ioctl(d, p, BIOCVERSION, &mut v).expect("BIOCVERSION");
        assert_eq!((v.bv_major, v.bv_minor), (1, 1));

        bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
        assert!(!ifp.if_bpf.is_set(), "the last listener is gone");
        assert!(bpfilter_lookup(7 << CLONE_SHIFT).is_none());
        assert_eq!(bpfclose::<G>(d, 0, 0, None), Err(Errno::ENXIO));
        bpfdetach(ifp);
        let r = hal::klock::reader();
        assert!(
            !BPF_IFLIST
                .iter(&r)
                .any(|bp| bpf_name_eq(&bp.bif_name, b"bpftest0"))
        );
    }

    #[test]
    fn the_store_and_hold_buffers_rotate_and_drop_as_the_c_counts() {
        // bufsize 32 on an Ethernet tap (bif_hdrlen 30): a 10-byte packet takes the whole
        // buffer with 2 bytes captured.
        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpfrot0");
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        let d = dev(8);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        let mut size = 32u32;
        ioctl(d, p, BIOCSBLEN, &mut size).expect("BIOCSBLEN");
        setif(d, p, b"bpfrot0").expect("BIOCSETIF");

        let tap = |first: u8| {
            let m = packet(&[first; 10]);
            assert!(!bpf_mtap(&ifp.if_bpf, &m, BPF_DIRECTION_IN));
        };
        let caught = |buf: &[u8]| -> (u32, u32, u8) {
            let bh: BpfHdr = hal::pod::at(buf, 0).expect("a header");
            (bh.bh_caplen, bh.bh_datalen, buf[30])
        };
        tap(1); // stored
        tap(2); // rotates: 1 held, 2 stored
        tap(3); // no free buffer: dropped
        let mut st = BpfStat::default();
        ioctl(d, p, BIOCGSTATS, &mut st).expect("BIOCGSTATS");
        assert_eq!(
            st,
            BpfStat {
                bs_recv: 3,
                bs_drop: 1
            }
        );
        let (r, buf) = read(d, p, 32);
        assert_eq!((r, buf.len()), (Ok(()), 32));
        assert_eq!(caught(&buf), (2, 10, 1));
        tap(4); // rotates: 2 held, 4 stored
        let (_, buf) = read(d, p, 32);
        assert_eq!(caught(&buf), (2, 10, 2));
        // Nothing held: the non-blocking read rotates the store buffer out.
        let (_, buf) = read(d, p, 32);
        assert_eq!(caught(&buf), (2, 10, 4));
        let (r, buf) = read(d, p, 32);
        assert_eq!((r, buf.len()), (Ok(()), 0), "empty");

        bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
        bpfdetach(ifp);
    }

    #[test]
    fn listeners_leave_the_chain_from_its_head_middle_and_end() {
        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpfchain0");
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        let devs = [dev(16), dev(17), dev(18), dev(19)];
        for d in devs {
            bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
            setif(d, p, b"bpfchain0").expect("BIOCSETIF");
        }
        let m = packet(&[5; 20]);
        // The bytes each open descriptor holds after a tap.
        let pending = |d: Dev| {
            let mut n = 0i32;
            ioctl(d, p, FIONREAD, &mut n).expect("FIONREAD");
            n
        };
        // Inserted at the head: 19 18 17 16. Close the middle (17), the head (19), the end
        // (16): the others keep receiving.
        for (gone, left) in [(17, &[16, 18, 19][..]), (19, &[16, 18]), (16, &[18])] {
            bpfclose::<G>(dev(gone), 0, 0, None).expect("bpfclose");
            let before: Vec<i32> = left.iter().map(|&u| pending(dev(u))).collect();
            assert!(!bpf_mtap(&ifp.if_bpf, &m, BPF_DIRECTION_IN));
            for (&u, b) in left.iter().zip(before) {
                // The record after the previous one's end rounded to a longword.
                let want = bpf_wordalign(b as usize) + 30 + 20;
                assert_eq!(pending(dev(u)) as usize, want, "unit {u} after {gone} left");
            }
        }
        bpfclose::<G>(dev(18), 0, 0, None).expect("bpfclose");
        assert!(!ifp.if_bpf.is_set(), "no listener left");
        bpfdetach(ifp);
    }

    #[test]
    fn a_locked_descriptor_answers_the_c_s_nineteen_requests() {
        // (request, answered when locked), bpf.c:780-800.
        let table: [(u64, bool); 36] = [
            (BIOCGBLEN, true),
            (BIOCFLUSH, true),
            (BIOCGDLT, true),
            (BIOCGDLTLIST, true),
            (BIOCGETIF, true),
            (BIOCGRTIMEOUT, true),
            (BIOCGWTIMEOUT, true),
            (BIOCGSTATS, true),
            (BIOCVERSION, true),
            (BIOCGRSIG, true),
            (BIOCGHDRCMPLT, true),
            (FIONREAD, true),
            (BIOCLOCK, true),
            (BIOCSRTIMEOUT, true),
            (BIOCSWTIMEOUT, true),
            (BIOCDWTIMEOUT, true),
            (BIOCIMMEDIATE, true),
            (TIOCGPGRP, true),
            (BIOCGDIRFILT, true),
            (BIOCSBLEN, false),
            (BIOCSETF, false),
            (BIOCSETFNR, false),
            (BIOCSETWF, false),
            (BIOCPROMISC, false),
            (BIOCSDLT, false),
            (BIOCSETIF, false),
            (BIOCSHDRCMPLT, false),
            (BIOCGFILDROP, false),
            (BIOCSFILDROP, false),
            (BIOCSDIRFILT, false),
            (BIOCSRSIG, false),
            (FIOASYNC, false),
            (FIOSETOWN, false),
            (TIOCSPGRP, false),
            (FIOGETOWN, false),
            (0x2000_42ff, false),
        ];
        assert_eq!(table.iter().filter(|(_, ok)| *ok).count(), 19);
        for (cmd, ok) in table {
            assert_eq!(allowed_when_locked(cmd), ok, "{cmd:#x}");
        }

        let (_g, p) = setup();
        let d = dev(9);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        let mut one = 1i32;
        ioctl(d, p, BIOCLOCK, &mut one).expect("BIOCLOCK");
        for (cmd, ok) in table {
            let mut data = [0u8; 32];
            let r = bpfioctl::<G>(d, cmd, &mut data, 0, p);
            assert_eq!(r == Err(Errno::EPERM), !ok, "{cmd:#x}: {r:?}");
        }
        bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
    }

    #[test]
    fn biocsetf_keeps_the_c_s_bounds_and_retires_the_old_program() {
        let (_g, p) = setup();
        let d = dev(10);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        let ret = bpf_stmt(BPF_RET | BPF_K, 1);

        // A NULL program of a length, a program past BPF_MAXINSNS, an empty one, one whose
        // copy faults partway (its second instruction past the user memory).
        let mut p0 = BpfProgram {
            bf_len: 1,
            _pad0: 0,
            bf_insns: 0,
        };
        assert_eq!(ioctl(d, p, BIOCSETF, &mut p0), Err(Errno::EINVAL));
        let mut long = program(&vec![ret; BPF_MAXINSNS as usize + 1][..1]);
        long.bf_len = BPF_MAXINSNS + 1;
        assert_eq!(ioctl(d, p, BIOCSETF, &mut long), Err(Errno::EINVAL));
        let mut empty = program(&[ret]);
        empty.bf_len = 0;
        assert_eq!(ioctl(d, p, BIOCSETF, &mut empty), Err(Errno::EINVAL));
        let mut fault = BpfProgram {
            bf_len: 2,
            _pad0: 0,
            bf_insns: (USER_BASE + USER_LEN - size_of::<BpfInsn>()) as u64,
        };
        assert_eq!(ioctl(d, p, BIOCSETF, &mut fault), Err(Errno::EINVAL));
        let mut max = program(&vec![ret; BPF_MAXINSNS as usize]);
        assert_eq!(ioctl(d, p, BIOCSETF, &mut max), Ok(()), "512 instructions");

        // The program in place, then replaced: the old one waits for a grace period.
        let d0 = bpfilter_lookup(10 << CLONE_SHIFT).expect("open");
        let old = {
            let g = hal::smr::read();
            d0.bd_rfilter
                .load(&g)
                .map(SmrRef::arc)
                .expect("a read filter")
        };
        assert_eq!(old.insns.len(), BPF_MAXINSNS as usize);
        let mut one = program(&[ret]);
        ioctl(d, p, BIOCSETF, &mut one).expect("BIOCSETF");
        assert_eq!(hal::smr::retired_count(&*old), 1, "retired, not released");
        // The write filter is the other word.
        ioctl(d, p, BIOCSETWF, &mut one).expect("BIOCSETWF");
        {
            let g = hal::smr::read();
            assert!(d0.bd_wfilter.load(&g).is_some() && d0.bd_rfilter.load(&g).is_some());
        }
        while hal::smr::retired_count(&*old) != 0 {
            grace();
        }
        drop(old);
        drop(d0);
        bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
    }

    #[test]
    fn the_link_types_of_an_interface_are_listed_and_switched() {
        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpfdlt0");
        static SECOND: BpfTap = BpfTap::new();
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        drop(bpfxattach(&SECOND, b"bpfdlt0", ifp, DLT_LOOP, 4));
        let d = dev(11);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        let mut bfl = BpfDltlist::default();
        assert_eq!(
            ioctl(d, p, BIOCGDLTLIST, &mut bfl),
            Err(Errno::EINVAL),
            "unattached"
        );
        setif(d, p, b"bpfdlt0").expect("BIOCSETIF");

        // Counted only, with no list.
        ioctl(d, p, BIOCGDLTLIST, &mut bfl).expect("BIOCGDLTLIST");
        assert_eq!(bfl.bfl_len, 2);
        // Bounded by bfl_len.
        bfl = BpfDltlist {
            bfl_len: 1,
            _pad0: 0,
            bfl_list: UWORD as u64,
        };
        assert_eq!(ioctl(d, p, BIOCGDLTLIST, &mut bfl), Err(Errno::ENOMEM));
        bfl.bfl_len = 4;
        ioctl(d, p, BIOCGDLTLIST, &mut bfl).expect("BIOCGDLTLIST");
        assert_eq!(bfl.bfl_len, 2);
        let got = user_bytes(UWORD, 8).expect("the list");
        assert_eq!(&got[..4], &DLT_EN10MB.to_ne_bytes());
        assert_eq!(&got[4..], &DLT_LOOP.to_ne_bytes());

        // The other link type of the name, then one it has not.
        let mut dlt = DLT_LOOP;
        ioctl(d, p, BIOCSDLT, &mut dlt).expect("BIOCSDLT");
        ioctl(d, p, BIOCGDLT, &mut dlt).expect("BIOCGDLT");
        assert_eq!(dlt, DLT_LOOP);
        assert!(
            SECOND.is_set() && !ifp.if_bpf.is_set(),
            "moved to the other tap"
        );
        dlt = DLT_RAW;
        assert_eq!(ioctl(d, p, BIOCSDLT, &mut dlt), Err(Errno::EINVAL));

        bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
        bpfdetach(ifp);
        assert!(!SECOND.is_set());
    }

    #[test]
    fn a_detached_tap_revokes_its_listeners() {
        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpfgone0");
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        let d = dev(12);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        setif(d, p, b"bpfgone0").expect("BIOCSETIF");
        assert!(ifp.if_bpf.is_set());

        bpfdetach(ifp);
        assert!(
            revoked().contains(&(12 << CLONE_SHIFT)),
            "vdevgone of its unit"
        );
        assert!(
            bpfilter_lookup(12 << CLONE_SHIFT).is_none(),
            "closed by the revoke"
        );
        assert!(!ifp.if_bpf.is_set());
        let r = hal::klock::reader();
        assert!(
            !BPF_IFLIST
                .iter(&r)
                .any(|bp| bpf_name_eq(&bp.bif_name, b"bpfgone0"))
        );
    }

    /// A driver whose output keeps the destination's family and data and the packet's bytes.
    struct WriteDriver {
        last: Mutex<Option<(u8, Vec<u8>, Vec<u8>)>>,
    }

    impl NetDriver for WriteDriver {
        fn ioctl(&self, _ifp: &'static Ifnet, _cmd: u64, _data: &mut [u8]) -> Result<(), Errno> {
            Ok(())
        }

        fn output(
            &self,
            _ifp: &'static Ifnet,
            m: MbufBox,
            dst: SockaddrView<'_>,
            _rt: Option<&crate::net::route::Rtentry>,
        ) -> Result<(), Errno> {
            *self.last.lock().unwrap_or_else(PoisonError::into_inner) =
                Some((dst.family(), dst.bytes()[2..].to_vec(), m.data().to_vec()));
            Ok(())
        }
    }

    /// `write(2)` of `bytes` (at `UBUF`).
    fn write(d: Dev, p: &Thread<G>, bytes: &[u8]) -> Result<(), Errno> {
        UserPtr::<u8>::new(UBUF)
            .copyout_slice(bytes)
            .expect("the bytes in user memory");
        let mut iov = [Iovec::new()];
        let mut u = uio(&mut iov, p, UioRw::UIO_WRITE, bytes.len());
        bpfwrite::<G>(d, &mut u, 0)
    }

    #[test]
    fn a_write_goes_out_with_its_link_header_in_the_destination() {
        static DRIVER: WriteDriver = WriteDriver {
            last: Mutex::new(None),
        };
        let (_g, p) = setup();
        let ifp = test_ifnet_with(b"bpfwr0", &DRIVER);
        ifp.if_mtu.set(1500);
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        let d = dev(13);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        assert_eq!(write(d, p, &[0; 20]), Err(Errno::ENXIO), "unattached");
        setif(d, p, b"bpfwr0").expect("BIOCSETIF");
        assert_eq!(write(d, p, &[0; 20]), Err(Errno::ENETDOWN));
        ifp.if_flags.set(ifp.if_flags.get() | IFF_UP);
        assert_eq!(write(d, p, &[]), Ok(()), "an empty write sends nothing");
        assert_eq!(
            write(d, p, &[0; 10]),
            Err(Errno::EINVAL),
            "shorter than the header"
        );

        let frame: Vec<u8> = (0u8..34).collect();
        assert_eq!(write(d, p, &frame), Ok(()));
        let last = DRIVER
            .last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .expect("sent");
        assert_eq!(last.0, AF_UNSPEC);
        assert_eq!(&last.1[..14], &frame[..14], "the header in sa_data");
        assert_eq!(&last.2[..], &frame[14..], "the payload in the packet");

        // A complete header: pseudo_AF_HDRCMPLT.
        let mut on = 1u32;
        ioctl(d, p, BIOCSHDRCMPLT, &mut on).expect("BIOCSHDRCMPLT");
        assert_eq!(write(d, p, &frame), Ok(()));
        let last = DRIVER
            .last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
            .expect("sent");
        assert_eq!(last.0, pseudo_AF_HDRCMPLT);

        // Past the interface's MTU.
        ifp.if_mtu.set(10);
        assert_eq!(write(d, p, &frame), Err(Errno::EMSGSIZE));
        ifp.if_mtu.set(1500);

        // A write filter that keeps less than the packet refuses it.
        let mut prog = program(&[bpf_stmt(BPF_RET | BPF_K, 8)]);
        ioctl(d, p, BIOCSETWF, &mut prog).expect("BIOCSETWF");
        assert_eq!(write(d, p, &frame), Err(Errno::EPERM));

        bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
        bpfdetach(ifp);
    }

    #[test]
    fn the_sysctls_keep_their_bounds() {
        let _g = setup();
        let set = |name: i32, v: i32| -> Result<(), Errno> {
            UserPtr::<i32>::new(UWORD).copyout(&v).expect("the value");
            let mut oldlen = 0usize;
            let mut req = SysctlReq {
                oldp: 0,
                oldlenp: &mut oldlen,
                newp: UWORD,
                newlen: size_of::<i32>(),
            };
            bpf_sysctl(&[name], &mut req)
        };
        assert_eq!(set(NET_BPF_BUFSIZE, 31), Err(Errno::EINVAL));
        assert_eq!(set(NET_BPF_BUFSIZE, 4096), Ok(()));
        assert_eq!(bpf_bufsize.load(Ordering::Relaxed), 4096);
        assert_eq!(
            set(NET_BPF_BUFSIZE, BPF_MAXBUFSIZE + 1),
            Err(Errno::EINVAL),
            "above net.bpf.maxbufsize"
        );
        assert_eq!(set(NET_BPF_MAXBUFSIZE, 31), Err(Errno::EINVAL));
        assert_eq!(set(NET_BPF_MAXBUFSIZE, 1 << 20), Ok(()));
        assert_eq!(bpf_maxbufsize.load(Ordering::Relaxed), 1 << 20);
        let mut oldlen = 0usize;
        let mut req = SysctlReq {
            oldp: 0,
            oldlenp: &mut oldlen,
            newp: 0,
            newlen: 0,
        };
        assert_eq!(bpf_sysctl(&[1, 2], &mut req), Err(Errno::ENOTDIR));
        assert_eq!(bpf_sysctl(&[3], &mut req), Err(Errno::EOPNOTSUPP));
        bpf_test_reset();
    }

    #[test]
    fn a_tap_inside_its_section_keeps_the_descriptor_a_close_unlinks() {
        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpfrace0");
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        let d = dev(14);
        bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
        setif(d, p, b"bpfrace0").expect("BIOCSETIF");

        let g = hal::smr::read();
        let bp = ifp.if_bpf.load(&g).expect("tapped");
        let held = bp.bif_dlist.load(&g).expect("a listener");
        // The close, on another thread, while this reader stands on the descriptor.
        thread::scope(|s| {
            s.spawn(|| {
                hal::klock::kernel_lock();
                bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
                hal::klock::kernel_unlock();
            });
        });
        // The old descriptor, whole; the chain and the driver's word empty.
        assert_eq!(held.bd_unit, (14 << CLONE_SHIFT) as i32);
        assert!(held.bd_mtx.lock().bif.is_none(), "detached");
        assert!(bp.bif_dlist.load(&g).is_none() && !ifp.if_bpf.is_set());
        assert!(
            hal::smr::retired_count(held.get()) >= 1,
            "the chain's reference waits for the section"
        );
        drop(g);
        grace();
        bpfdetach(ifp);
    }

    #[test]
    fn taps_racing_opens_and_closes_see_live_descriptors() {
        // One tapping thread on its own CPU, a grace period that waits for it, and 50 opens
        // and closes on the tap it walks.
        struct Tapper {
            cpu: AtomicUsize,
            asked: AtomicBool,
        }
        impl CpuWalk for Tapper {
            fn running(&self, n: usize) -> bool {
                self.cpu.load(Ordering::SeqCst) == n
            }
            fn peg(&self, _n: usize) {
                self.asked.store(true, Ordering::SeqCst);
                while self.asked.load(Ordering::SeqCst) {
                    thread::yield_now();
                }
            }
            fn unpeg(&self) {}
        }

        let (_g, p) = setup();
        let (ifp, _) = test_ifnet(b"bpfrace1");
        bpfattach(&ifp.if_bpf, ifp, DLT_EN10MB, ETHER_HDR_LEN as u32);
        let walk = Tapper {
            cpu: AtomicUsize::new(usize::MAX),
            asked: AtomicBool::new(false),
        };
        let stop = AtomicBool::new(false);
        let taps = AtomicUsize::new(0);
        thread::scope(|s| {
            s.spawn(|| {
                walk.cpu.store(hal::cpu::cpu_number(), Ordering::SeqCst);
                let m = packet(&[7; 40]);
                while !stop.load(Ordering::SeqCst) {
                    let _ = bpf_mtap(&ifp.if_bpf, &m, BPF_DIRECTION_IN);
                    taps.fetch_add(1, Ordering::Relaxed);
                    if walk.asked.load(Ordering::SeqCst) {
                        hal::smr::idle();
                        walk.asked.store(false, Ordering::SeqCst);
                    }
                }
            });
            while walk.cpu.load(Ordering::SeqCst) == usize::MAX {
                thread::yield_now();
            }
            for _ in 0..50 {
                let d = dev(15);
                bpfopen::<G>(d, 0, 0, p).expect("bpfopen");
                let mut one = 1i32;
                ioctl(d, p, BIOCIMMEDIATE, &mut one).expect("BIOCIMMEDIATE");
                setif(d, p, b"bpfrace1").expect("BIOCSETIF");
                thread::yield_now();
                bpfclose::<G>(d, 0, 0, None).expect("bpfclose");
                hal::smr::idle();
                let _ = hal::smr::collect().run(&walk);
            }
            stop.store(true, Ordering::SeqCst);
        });
        assert!(taps.load(Ordering::Relaxed) > 0);
        assert!(!ifp.if_bpf.is_set());
        bpfdetach(ifp);
    }
}
/* </TESTS> */
