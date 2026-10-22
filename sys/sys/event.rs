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

/*-
 * Copyright (c) 1999,2000,2001 Jonathan Lemon <jlemon@FreeBSD.org>
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
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 *	$FreeBSD: src/sys/sys/event.h,v 1.11 2001/02/24 01:41:31 jlemon Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/event.h>`: `kqueue(2)`'s `struct kevent`, filters, flags and notes, and the kernel
//! half (`struct klist`, `struct knote`, `struct filterops`, `struct kqueue_scan_state`, the
//! inline helpers), as `bsd` names them: the kernel crate's (`kernel::event`), instantiated
//! with `bsd`'s process glue.
//!
//! Upstream: sys/sys/event.h @ 3ce1f3f79392
//! LZ: sys/sys/event.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.4: the types and the helpers are `kernel::event`'s (`sys/kernel/event.rs`, which says
//!   what changed shape: the knote owned by its kqueue's chain, the klist's three kinds, the
//!   counted kqueue); this header keeps their names for the filters still in `bsd` (the 44
//!   `struct filterops` of 18 files, their `kn_hook` derefs among them), with 0 `unsafe`:
//!   `Kevent` is `hal::abi_pod!`'s (LZ's `unsafe impl AbiPod`), `Klist` needs no `unsafe impl
//!   Sync`, `struct klistops` is gone with the kinds, and `kn_kq`/`kqs_kq` are counted
//!   handles.
//! - `knote_modify_fn`/`knote_process_fn` keep LZ's form (an event function as an `fn`
//!   pointer) over the kernel's `knote_assign`/`knote_submit`.

use crate::kern::proc_glue::BsdProcGlue;
use crate::sys::errno::Errno;

pub use kernel::event::{
    __EV_HUP, __EV_POLL, __EV_SELECT, EV_ADD, EV_CLEAR, EV_DELETE, EV_DISABLE, EV_DISPATCH,
    EV_ENABLE, EV_EOF, EV_ERROR, EV_FLAG1, EV_ONESHOT, EV_RECEIPT, EV_SYSFLAGS, EVFILT_AIO,
    EVFILT_DEVICE, EVFILT_EXCEPT, EVFILT_MARKER, EVFILT_PROC, EVFILT_READ, EVFILT_SIGNAL,
    EVFILT_SYSCOUNT, EVFILT_TIMER, EVFILT_USER, EVFILT_VNODE, EVFILT_WRITE, FILTEROP_ISFD,
    FILTEROP_MPSAFE, HookWord, KN_ACTIVE, KN_DETACHED, KN_DISABLED, KN_HASHSIZE, KN_PROCESSING,
    KN_QUEUED, KN_WAITING, Kevent, KlistKind, KnKevent, NOTE_ABSTIME, NOTE_ATTRIB, NOTE_CHANGE,
    NOTE_CHILD, NOTE_DELETE, NOTE_EOF, NOTE_EXEC, NOTE_EXIT, NOTE_EXTEND, NOTE_FFAND, NOTE_FFCOPY,
    NOTE_FFCTRLMASK, NOTE_FFLAGSMASK, NOTE_FFNOP, NOTE_FFOR, NOTE_FORK, NOTE_LINK, NOTE_LOWAT,
    NOTE_MSECONDS, NOTE_NSECONDS, NOTE_OOB, NOTE_PCTRLMASK, NOTE_PDATAMASK, NOTE_RENAME,
    NOTE_REVOKE, NOTE_SECONDS, NOTE_SIGNAL, NOTE_SUBMIT, NOTE_TRACK, NOTE_TRACKERR, NOTE_TRIGGER,
    NOTE_TRUNCATE, NOTE_USECONDS, NOTE_WRITE, ev_set, knote_assign, knote_submit,
};

/// `struct knote` of `bsd`'s processes.
pub type Knote = kernel::event::Knote<BsdProcGlue>;
/// `struct klist` of `bsd`'s processes.
pub type Klist = kernel::event::Klist<BsdProcGlue>;
/// `struct filterops` of `bsd`'s processes.
pub type Filterops = kernel::event::Filterops<BsdProcGlue>;
/// `struct kqueue_scan_state`.
pub type KqueueScanState<'m> = kernel::event::KqueueScanState<'m, BsdProcGlue>;

/// The type of `f_attach`.
pub type FAttach = fn(&Knote) -> Result<(), Errno>;
/// The type of `f_detach`.
pub type FDetach = fn(&Knote);
/// The type of `f_event`: `true` when the event is active.
pub type FEvent = fn(&Knote, i64) -> bool;
/// The type of `f_modify`: `true` when the knote has become active.
pub type FModify = fn(&mut Kevent, &Knote) -> bool;
/// The type of `f_process`: `true` when the event should be returned to the user.
pub type FProcess = fn(&Knote, Option<&mut Kevent>) -> bool;

/// `knote_modify_fn(kev, kn, f_event)`.
pub fn knote_modify_fn(kev: &Kevent, kn: &Knote, f_event: FEvent) -> bool {
    knote_assign(kev, kn);
    f_event(kn, 0)
}

/// `knote_modify(kev, kn)`: assigns the user's parameters and re-runs the event function.
pub fn knote_modify(kev: &Kevent, kn: &Knote) -> bool {
    kernel::event::knote_modify(kev, kn)
}

/// `knote_process_fn(kn, kev, f_event)`.
pub fn knote_process_fn(kn: &Knote, kev: Option<&mut Kevent>, f_event: FEvent) -> bool {
    // If called from kqueue_scan(), skip f_event when EV_ONESHOT is set, to preserve old
    // behaviour.
    let active = if kev.is_some() && kn.has_flags(EV_ONESHOT) {
        true
    } else {
        f_event(kn, 0)
    };
    if active {
        knote_submit(kn, kev);
    }
    active
}

/// `knote_process(kn, kev)`: whether the event is active, its state stored in `kev` if so.
pub fn knote_process(kn: &Knote, kev: Option<&mut Kevent>) -> bool {
    kernel::event::knote_process(kn, kev)
}

/// `klist_empty(klist)`.
pub fn klist_empty(klist: &Klist) -> bool {
    klist.is_empty()
}
/* </CODE> */
