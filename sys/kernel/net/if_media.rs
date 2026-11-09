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
 * Copyright (c) 1998, 2000 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe of the Numerical Aerospace Simulation Facility,
 * NASA Ames Research Center.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 1997
 *	Jonathan Stone and Jason R. Thorpe.  All rights reserved.
 *
 * This software is derived from information provided by Matt Thomas.
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
 *	This product includes software developed by Jonathan Stone
 *	and Jason R. Thorpe for the NetBSD Project.
 * 4. The names of the authors may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHORS ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/*-
 * Copyright (c) 1998 The NetBSD Foundation, Inc.
 * All rights reserved.
 *
 * This code is derived from software contributed to The NetBSD Foundation
 * by Jason R. Thorpe of the Numerical Aerospace Simulation Facility,
 * NASA Ames Research Center.
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
 * THIS SOFTWARE IS PROVIDED BY THE NETBSD FOUNDATION, INC. AND CONTRIBUTORS
 * ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
 * PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE FOUNDATION OR CONTRIBUTORS
 * BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

/*
 * Copyright (c) 1997
 *	Jonathan Stone and Jason R. Thorpe.  All rights reserved.
 *
 * This software is derived from information provided by Matt Thomas.
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
 *      This product includes software developed by Jonathan Stone
 *	and Jason R. Thorpe for the NetBSD Project.
 * 4. The names of the authors may not be used to endorse or promote products
 *    derived from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHORS ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/* </LICENSES> */

/* <CODE> */
//! `net/if_media.c`: network interface media selection, BSD/OS compatible. `struct ifmedia`
//! (the list of media a driver supports and the one selected), the functions drivers call to
//! fill it (`ifmedia_init`, `ifmedia_add`, `ifmedia_set`, `ifmedia_delete_instance`) and to
//! answer `SIOCGIFMEDIA`/`SIOCSIFMEDIA` (`ifmedia_ioctl`), `ifmedia_match` and
//! `ifmedia_baudrate`, the interface speed of a media word. `<net/if_media.h>`'s words are
//! `hal::sys::net_if_media`'s.
//!
//! Upstream: sys/net/if_media.h @ 3ce1f3f79392, sys/net/if_media.c @ 3ce1f3f79392
//! LZ: sys/net/if_media.rs@3c62ede68802
//!
//! Locks used to protect members: \[M\] `ifmedia_mtx` (here each `struct ifmedia`'s own
//! mutex at `IPL_NET`, as `if_hooks_mtx` is each interface's: the C's one global mutex keeps
//! per-media lists, at the same level, taken by no other path).
//!
//! An [`Ifmedia`] lives in a driver's softc, which `bsd` zero-fills (`malloc(M_ZERO)`,
//! `config_attach`'s): every member is valid all zero (the embed idiom, docs/IDIOMS.md):
//!
//! | member | type | all zero |
//! |---|---|---|
//! | `ifm_mask`, `ifm_media`, the selected entry's words | `crate::net::if_var::AtomicCell` | 0, no entry selected |
//! | the list, its count, the callbacks | `hal::once::OnceStatic` (what `ifmedia_init` builds) | empty (`an_all_zero_cell_is_an_empty_one`) |
//!
//! ## Deviations
//! - The description tables of `<net/if_media.h>` are left out (`hal::sys::net_if_media`'s
//!   deviation), and with them `ifmedia_debug` and `ifmedia_printword` (`IFMEDIA_DEBUG` is
//!   not defined); `ifmedia_get`'s "multiple match" message is printed under feature
//!   `diagnostic` (`DIAGNOSTIC`).
//! - A `struct ifmedia` is initialised once: a second `ifmedia_init` of the same one is
//!   `panic(9)` (the C initialises it again and leaks its entries). Every driver initialises
//!   its media once per attach, on a zeroed softc.
//! - Before `ifmedia_init`, `ifmedia_add`, `ifmedia_set` and `ifmedia_ioctl` are `panic(9)`
//!   (the C follows the zeroed list's NULL tail or a NULL callback); `ifmedia_match` and
//!   `ifmedia_delete_instance` find an empty list, as the C's walk of a zeroed `TAILQ` does.
//! - The C's `NULL` checks of `ifmedia_ioctl`'s pointers are the references' types; a request
//!   shorter than its structure (`struct ifreq` for `SIOCSIFMEDIA`, `struct ifmediareq` for
//!   `SIOCGIFMEDIA`) or misaligned for it is `EINVAL`, which no caller reaches.
//! - `SIOCSIFMEDIA`'s rollback after the driver refused puts the old entry back only when it
//!   is still on the list (the C puts back a pointer `ifmedia_delete_instance` may have
//!   freed in between); else no entry is selected.
//!
//! ## Redesign
//! - LZ's `struct ifmedia` of `Cell`s (a `TailqHead` of `malloc`ed entries, a raw `ifm_cur`,
//!   the two callbacks as `Cell<Option<fn>>`, `unsafe impl Sync`, the global `ifmedia_mtx`)
//!   -> [`Ifmedia`]: the mask and the user's media word atomics (`AtomicCell`, LZ's
//!   `get()`/`set()` API), and a set-once half [`ifmedia_init`] builds: the entries, their
//!   count and the selected one's identity under the media's own `IPL_NET` mutex, with the
//!   callbacks a [`MediaCallbacks`] object (D-N6.3a-11: `ifmedia_init` runs before
//!   `if_attach`, so the callbacks cannot be the `NetDriver`'s; `bsd`'s drivers keep their
//!   two functions, which its adapter wraps per `ifmedia_init`).
//! - The entries are a chain of `MBox`es (each the C's `malloc(M_IFADDR)` of one entry),
//!   newest first: `ifmedia_add` links at the head, O(1) as the C's `TAILQ_INSERT_TAIL`; the
//!   walks that need the C's order (`SIOCGIFMEDIA`'s words, `ifmedia_get`'s first match) read
//!   the chain whole, O(n) as the C's (the words filled from the end, the oldest match
//!   kept). An entry is named by a number given at `ifmedia_add`, where the C keeps a
//!   pointer (`ifm_cur`, the rollback's `oldentry`).
//! - `ifm->ifm_cur->ifm_media`, which drivers and mii(4) read with no lock: the selected
//!   entry's words are mirrored in atomics, stored under the mutex by every change of the
//!   selection; [`Ifmedia::ifm_cur`] reads them without it (one load per word, as the C's
//!   pointer and member; a reader racing a change may see one word of each entry, as two of
//!   the C's reads may).
//! - `ifmedia_ioctl(ifp, ifr, ifm, cmd)` takes the request's bytes (`&mut [u8]`, the image of
//!   the command's structure), as the driver's `if_ioctl` gets them; the media words reach
//!   the user through `hal::user::UserPtr`.
//! - Functions returning 0 or an errno return `Result<(), Errno>`; `ifmedia_match` returns
//!   `bool` (LZ's).

use hal::intr::IPL_NET;
use hal::malloc::{M_NOWAIT, M_WAITOK, MArray, MBox};
use hal::mutex::{Mutexed, MutexedGuard};
use hal::once::OnceStatic;
use hal::sys::errno::Errno;
use hal::sys::net_if::{Ifmediareq, Ifreq};
use hal::sys::net_if_media::{
    IFM_AUTO, IFM_INST_ANY, IFM_NMASK, IFM_NONE, IFM_TMASK, IFMEDIA_BAUDRATE_DESCRIPTIONS,
    ifm_inst, ifm_subtype,
};
use hal::sys::sockio::{SIOCGIFMEDIA, SIOCSIFMEDIA};
use hal::user::UserPtr;

use crate::assert::kpanic;
use crate::net::if_::M_IFADDR;
use crate::net::if_var::{AtomicCell, Ifnet};
use crate::prf::printf;

/// `M_TEMP` (`<sys/malloc.h>`, `bsd`'s): `SIOCGIFMEDIA`'s copy of the media words.
const M_TEMP: i32 = 127;

/// `struct ifmedia_entry`: in-kernel representation of a single supported media type, as
/// the media's readers get it (a copy; the list keeps the entry).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IfmediaEntry {
    /// `ifm_media`: description of this media attachment.
    pub ifm_media: u64,
    /// `ifm_data`: for driver-specific use.
    pub ifm_data: u32,
    /// `ifm_aux`: for driver-specific use (the driver's word, never followed here).
    pub ifm_aux: usize,
}

/// One entry of the list: the C's `malloc`ed `struct ifmedia_entry`, linked to the entry
/// added before it.
struct IfmNode {
    /// The entry added before this one (`TAILQ_PREV`): the chain runs newest first.
    prev: Option<MBox<IfmNode>>,
    /// The entry's number, given by `ifmedia_add` (never 0).
    id: u32,
    /// The entry.
    entry: IfmediaEntry,
}

/// \[M\] `ifm_list`, `ifm_nwords` and `ifm_cur` (as its entry's number).
struct IfmList {
    /// The newest entry, the others behind it.
    last: Option<MBox<IfmNode>>,
    /// `ifm_nwords`: number of `ifm_list` entries.
    nwords: usize,
    /// `ifm_cur`: the selected entry's number, 0 for none.
    cur: u32,
    /// The number the next entry gets.
    next_id: u32,
}

impl IfmList {
    /// The entries, newest first.
    fn iter(&self) -> impl Iterator<Item = &IfmNode> {
        core::iter::successors(self.last.as_deref(), |n| n.prev.as_deref())
    }
}

impl Drop for IfmList {
    fn drop(&mut self) {
        ifm_free(self.last.take());
    }
}

/// What `ifmedia_init` builds.
struct IfmRt {
    /// `ifmedia_mtx` and what it guards.
    list: Mutexed<IfmList>,
    /// `ifm_change_cb` and `ifm_status_cb`.
    callbacks: &'static dyn MediaCallbacks,
}

/// `struct ifmedia`: one of these goes into a network interface's softc structure. It is used
/// to keep general media state. Valid all zero (the module's table).
pub struct Ifmedia {
    /// `ifm_mask`: mask of changes we don't care about.
    pub ifm_mask: AtomicCell<u64>,
    /// `ifm_media`: current user-set media word.
    pub ifm_media: AtomicCell<u64>,
    /// \[M\] whether an entry is selected (`ifm_cur != NULL`), for the readers without the
    /// mutex.
    cur_set: AtomicCell<u8>,
    /// \[M\] `ifm_cur->ifm_media`, mirrored.
    cur_media: AtomicCell<u64>,
    /// \[M\] `ifm_cur->ifm_data`, mirrored.
    cur_data: AtomicCell<u32>,
    /// \[M\] `ifm_cur->ifm_aux`, mirrored.
    cur_aux: AtomicCell<u64>,
    /// The list and the callbacks, from `ifmedia_init` on.
    rt: OnceStatic<IfmRt>,
}

impl Ifmedia {
    /// An empty `struct ifmedia`, what a zeroed softc holds before `ifmedia_init`.
    pub const fn new() -> Self {
        Self {
            ifm_mask: AtomicCell::new(),
            ifm_media: AtomicCell::new(),
            cur_set: AtomicCell::new(),
            cur_media: AtomicCell::new(),
            cur_data: AtomicCell::new(),
            cur_aux: AtomicCell::new(),
            rt: OnceStatic::new(),
        }
    }

    /// `ifm->ifm_cur`: the currently selected media, if any, read without `ifmedia_mtx` as
    /// the C's drivers and mii(4) read it (each word one load).
    pub fn ifm_cur(&self) -> Option<IfmediaEntry> {
        (self.cur_set.get() != 0).then(|| IfmediaEntry {
            ifm_media: self.cur_media.get(),
            ifm_data: self.cur_data.get(),
            ifm_aux: self.cur_aux.get() as usize,
        })
    }

    /// `ifm->ifm_nwords`.
    pub fn ifm_nwords(&self) -> usize {
        self.rt.get().map_or(0, |rt| rt.list.lock().nwords)
    }

    /// The driver's callbacks (`ifm_change_cb`, `ifm_status_cb`), `None` before
    /// `ifmedia_init`.
    pub fn callbacks(&self) -> Option<&'static dyn MediaCallbacks> {
        self.rt.get().map(|rt| rt.callbacks)
    }

    /// The list and the callbacks; `panic(9)` before `ifmedia_init` (the C walks an
    /// uninitialised list).
    fn rt(&self, what: &str) -> &IfmRt {
        match self.rt.get() {
            Some(rt) => rt,
            None => kpanic(format_args!("{what}: no ifmedia_init")),
        }
    }

    /// `ifm->ifm_cur = entry` under the mutex `g` holds: the selection and its mirrored words.
    fn set_cur(&self, g: &mut MutexedGuard<'_, IfmList>, entry: Option<(u32, IfmediaEntry)>) {
        if let Some((id, e)) = entry {
            g.cur = id;
            self.cur_media.set(e.ifm_media);
            self.cur_data.set(e.ifm_data);
            self.cur_aux.set(e.ifm_aux as u64);
            self.cur_set.set(1);
        } else {
            g.cur = 0;
            self.cur_set.set(0);
        }
    }
}

impl Default for Ifmedia {
    fn default() -> Self {
        Self::new()
    }
}

/// `ifm_change_cb_t` and `ifm_stat_cb_t`: the driver's media callbacks (D-N6.3a-11; at N7 the
/// driver's `NetDevice` answers both).
pub trait MediaCallbacks: Sync + 'static {
    /// `ifm_change_cb`: the driver switches the interface to the media just selected.
    fn change(&self, ifp: &'static Ifnet) -> Result<(), Errno>;

    /// `ifm_status_cb`: the driver reports the media's status (`ifm_status`, `ifm_active`).
    fn status(&self, ifp: &'static Ifnet, ifmr: &mut Ifmediareq);
}

/// Frees a chain of entries one at a time (no recursion down the chain).
fn ifm_free(mut chain: Option<MBox<IfmNode>>) {
    while let Some(mut n) = chain {
        chain = n.prev.take();
        drop(n);
    }
}

/// `ifmedia_init`: initialize if_media struct for a specific interface instance: no entry,
/// none selected, `dontcare_mask` the bits of a word a match ignores, `callbacks` the
/// driver's.
pub fn ifmedia_init(ifm: &Ifmedia, dontcare_mask: u64, callbacks: &'static dyn MediaCallbacks) {
    let rt = IfmRt {
        list: Mutexed::new(
            IPL_NET,
            IfmList {
                last: None,
                nwords: 0,
                cur: 0,
                next_id: 1,
            },
        ),
        callbacks,
    };
    if ifm.rt.set(rt).is_err() {
        kpanic(format_args!("ifmedia_init: initialised twice"));
    }
    ifm.cur_set.set(0);
    ifm.ifm_media.set(0);
    ifm.ifm_mask.set(dontcare_mask); // IF don't-care bits
}

/// `ifmedia_add`: add a media configuration to the list of supported media for a specific
/// interface instance. `panic(9)` when the entry cannot be allocated (`M_NOWAIT`, the C's).
pub fn ifmedia_add(ifm: &Ifmedia, mword: u64, data: i32, aux: usize) {
    let rt = ifm.rt("ifmedia_add");
    let node = IfmNode {
        prev: None,
        id: 0,
        entry: IfmediaEntry {
            ifm_media: mword,
            ifm_data: data as u32,
            ifm_aux: aux,
        },
    };
    let Some(mut node) = MBox::new(M_IFADDR, M_NOWAIT, node) else {
        kpanic(format_args!("ifmedia_add: can't malloc entry"));
    };

    let mut g = rt.list.lock();
    node.id = g.next_id;
    g.next_id = g.next_id.wrapping_add(1).max(1);
    node.prev = g.last.take();
    g.last = Some(node);
    g.nwords += 1;
}

/// `ifmedia_set`: set the default active media.
///
/// Called by device-specific code which is assumed to have already selected the default media
/// in hardware. We do _not_ call the media-change callback.
pub fn ifmedia_set(ifm: &Ifmedia, mut target: u64) {
    let rt = ifm.rt("ifmedia_set");
    let mut g = rt.list.lock();
    let mut found = ifmedia_get(&g, target, ifm.ifm_mask.get());

    // If we didn't find the requested media, then we try to fall back to target-type
    // (IFM_ETHER, e.g.) | IFM_NONE. If that's not on the list, then we add it and set the
    // media to it.
    //
    // Since ifmedia_set is almost always called with IFM_AUTO or with a known-good media,
    // this really should only occur if we:
    //
    // a) didn't find any PHYs, or
    // b) didn't find an autoselect option on the PHY when the parent ethernet driver
    //    expected to.
    //
    // In either case, it makes sense to select no media.
    if found.is_none() {
        printf(format_args!(
            "ifmedia_set: no match for 0x{:x}/0x{:x}\n",
            target,
            !ifm.ifm_mask.get()
        ));
        target = (target & IFM_NMASK) | IFM_NONE;
        found = ifmedia_get(&g, target, ifm.ifm_mask.get());
        if found.is_none() {
            drop(g);
            ifmedia_add(ifm, target, 0, 0);
            g = rt.list.lock();
            found = ifmedia_get(&g, target, ifm.ifm_mask.get());
            if found.is_none() {
                drop(g);
                kpanic(format_args!("ifmedia_set failed"));
            }
        }
    }
    ifm.set_cur(&mut g, found);
}

/// `ifmedia_ioctl`: device-independent media ioctl support function: `SIOCSIFMEDIA` (the
/// request a `struct ifreq`) selects the media the request's word matches (`EINVAL` for
/// none) and has the driver switch to it, the old selection back when it refuses (but for
/// `ENETRESET`); `SIOCGIFMEDIA` (a `struct ifmediareq`) reports the selection and the
/// driver's status and copies the supported words out (`ifm_count` 0: their number; too few:
/// `E2BIG`; negative: `EINVAL`). `ENOTTY` for another command.
pub fn ifmedia_ioctl(
    ifp: &'static Ifnet,
    data: &mut [u8],
    ifm: &Ifmedia,
    cmd: u64,
) -> Result<(), Errno> {
    match cmd {
        // Set the current media.
        SIOCSIFMEDIA => {
            let newmedia = hal::pod::view_mut::<Ifreq>(data, 0)
                .ok_or(Errno::EINVAL)?
                .ifr_media();
            let rt = ifm.rt("ifmedia_ioctl");

            let mut g = rt.list.lock();
            let Some(found) = ifmedia_get(&g, newmedia, ifm.ifm_mask.get()) else {
                return Err(Errno::EINVAL);
            };

            // If no change, we're done.
            // XXX Automedia may involve software intervention.
            //     Keep going in case the connected media changed.
            //     Similarly, if best match changed (kernel debugger?).
            if ifm_subtype(newmedia) != IFM_AUTO
                && newmedia == ifm.ifm_media.get()
                && found.0 == g.cur
            {
                return Ok(());
            }

            // We found a match, now make the driver switch to it. Make sure to preserve our
            // old media type in case the driver can't switch.
            let oldentry = g.cur;
            let oldmedia = ifm.ifm_media.get();
            ifm.set_cur(&mut g, Some(found));
            ifm.ifm_media.set(newmedia);
            drop(g);

            let error = rt.callbacks.change(ifp);
            if let Err(e) = error
                && e != Errno::ENETRESET
            {
                let mut g = rt.list.lock();
                if g.cur == found.0 {
                    let old = ifmedia_find(&g, oldentry);
                    ifm.set_cur(&mut g, old);
                    ifm.ifm_media.set(oldmedia);
                }
            }
            error
        }

        // Get list of available media and current media on interface.
        SIOCGIFMEDIA => {
            let ifmr = hal::pod::view_mut::<Ifmediareq>(data, 0).ok_or(Errno::EINVAL)?;
            let rt = ifm.rt("ifmedia_ioctl");

            if ifmr.ifm_count < 0 {
                return Err(Errno::EINVAL);
            }

            {
                let g = rt.list.lock();
                let cur = if g.cur != 0 {
                    ifm.cur_media.get()
                } else {
                    IFM_NONE
                };
                ifmr.ifm_active = cur;
                ifmr.ifm_current = cur;
                ifmr.ifm_mask = ifm.ifm_mask.get();
                ifmr.ifm_status = 0;
            }

            rt.callbacks.status(ifp, ifmr);

            let nwords = rt.list.lock().nwords;

            if ifmr.ifm_count == 0 {
                ifmr.ifm_count = nwords as i32;
                return Ok(());
            }

            ifmedia_copywords(&rt.list, ifmr, nwords)
        }

        _ => Err(Errno::ENOTTY),
    }
}

/// `SIOCGIFMEDIA`'s copy of the media words to the user's `ifm_ulist` (`ifm_count` of them
/// at most), `nwords` the count read before, without the mutex: a buffer of that many words,
/// made again while the list grew meanwhile; `E2BIG` when the user's array is shorter than
/// the list; the words copied are the list's under the mutex, never the buffer's length (a
/// list that shrank meanwhile copies fewer: N6.3a2 review's B1, the C's `nwords * sizeof`).
/// `ifm_count` ends as the list's length.
fn ifmedia_copywords(
    list: &Mutexed<IfmList>,
    ifmr: &mut Ifmediareq,
    mut nwords: usize,
) -> Result<(), Errno> {
    let error = loop {
        let Some(mut kptr) = MArray::new(M_TEMP, M_WAITOK, nwords, |_| 0u64) else {
            kpanic(format_args!("ifmedia_ioctl: no memory"));
        };

        let g = list.lock();
        // Media list might grow during malloc().
        if nwords < g.nwords {
            nwords = g.nwords;
            drop(g);
            drop(kptr);
            continue;
        }
        // Request memory too small, set error and ifm_count.
        if (ifmr.ifm_count as usize) < g.nwords {
            nwords = g.nwords;
            drop(g);
            drop(kptr);
            break Err(Errno::E2BIG);
        }
        // Get the media words from the interface's list (the chain runs newest first:
        // filled from the end of the list's own count).
        nwords = g.nwords;
        for (i, n) in g.iter().enumerate() {
            kptr[nwords - 1 - i] = n.entry.ifm_media;
        }
        drop(g);

        let error = UserPtr::<u64>::new(ifmr.ifm_ulist as usize).copyout_slice(&kptr[..nwords]);
        drop(kptr);
        break error;
    };
    ifmr.ifm_count = nwords as i32;
    error
}

/// `ifmedia_match`: find media entry matching a given ifm word; true if found (never before
/// `ifmedia_init`: an empty list, as the C's zeroed one).
pub fn ifmedia_match(ifm: &Ifmedia, target: u64, mask: u64) -> bool {
    let Some(rt) = ifm.rt.get() else {
        return false;
    };
    let g = rt.list.lock();
    ifmedia_get(&g, target, mask).is_some()
}

/// `ifmedia_get`: the entry whose word equals `target` outside the don't-care `mask`, the
/// first added of those (the C's first in list order); none when there is none. Called with
/// the media's mutex held (the guard's list).
fn ifmedia_get(list: &IfmList, target: u64, mask: u64) -> Option<(u32, IfmediaEntry)> {
    let mask = !mask;
    let mut found = None;
    let mut matches = 0u32;

    // Newest first: the last match seen is the first added.
    for n in list.iter() {
        if (n.entry.ifm_media & mask) == (target & mask) {
            found = Some((n.id, n.entry));
            matches += 1;
        }
    }

    #[cfg(feature = "diagnostic")]
    if matches > 1
        && let Some((_, m)) = found
    {
        printf(format_args!(
            "ifmedia_get: multiple match for 0x{:x}/0x{:x}, selected instance {}\n",
            target,
            mask,
            ifm_inst(m.ifm_media) as i64
        ));
    }
    let _ = matches;

    found
}

/// The entry numbered `id` (0: none), still on the list.
fn ifmedia_find(list: &IfmList, id: u32) -> Option<(u32, IfmediaEntry)> {
    if id == 0 {
        return None;
    }
    list.iter().find(|n| n.id == id).map(|n| (n.id, n.entry))
}

/// `ifmedia_delete_instance`: delete all media for a given instance (`IFM_INST_ANY` for
/// all); none is selected after. The entries are freed after the mutex is let go. Before
/// `ifmedia_init` there is nothing to delete and nothing selected (the C's zeroed list).
pub fn ifmedia_delete_instance(ifm: &Ifmedia, inst: u64) {
    let Some(rt) = ifm.rt.get() else {
        return;
    };
    let mut freed = None;

    {
        let mut g = rt.list.lock();
        // Newest first off the list; the kept ones come back in their order.
        let mut rest = g.last.take();
        let mut kept = None;
        while let Some(mut n) = rest {
            rest = n.prev.take();
            if inst == IFM_INST_ANY || inst == ifm_inst(n.entry.ifm_media) {
                n.prev = freed.take();
                freed = Some(n);
                g.nwords -= 1;
            } else {
                n.prev = kept.take();
                kept = Some(n);
            }
        }
        while let Some(mut n) = kept {
            kept = n.prev.take();
            n.prev = g.last.take();
            g.last = Some(n);
        }
        ifm.set_cur(&mut g, None);
    }

    // Do not hold mutex longer than necessary, call free() without.
    ifm_free(freed);
}

/// `ifmedia_baudrate`: compute the interface `baudrate` from the media, for the interface
/// metrics (used by routing daemons); 0 when not known.
pub fn ifmedia_baudrate(mword: u64) -> u64 {
    for d in &IFMEDIA_BAUDRATE_DESCRIPTIONS {
        if d.ifmb_word == 0 {
            break;
        }
        if (mword & (IFM_NMASK | IFM_TMASK)) == d.ifmb_word {
            return d.ifmb_baudrate;
        }
    }

    // Not known.
    0
}

/// The words of `ifm`'s list in the C's order, for the tests.
#[cfg(test)]
fn ifmedia_words(ifm: &Ifmedia) -> std::vec::Vec<u64> {
    let mut w: std::vec::Vec<u64> = ifm
        .rt("ifmedia_words")
        .list
        .lock()
        .iter()
        .map(|n| n.entry.ifm_media)
        .collect();
    w.reverse();
    w
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;
    use std::boxed::Box;
    use std::sync::atomic::AtomicU32;
    use std::{assert, assert_eq};

    use hal::sys::net_if::{if_kbps, if_mbps};
    use hal::sys::net_if_media::{
        IFM_10_T, IFM_100_TX, IFM_100G_SR4, IFM_1000_T, IFM_ACTIVE, IFM_AVALID, IFM_ETHER, IFM_FDX,
        IFM_IEEE80211, IFM_IEEE80211_HT_MCS0, ifm_makeword,
    };
    use hal::sys::sockio::SIOCSIFADDR;

    use super::*;
    use crate::net::if_::test_ifnet;
    use crate::net::if_::tests::setup;

    /// The user's memory for the media words: a slot of the arguments' range of the host's
    /// user memory double that no other kernel test uses (sysctl's runs from 0xc800, tc's,
    /// clockintr's and time's after it, without the memory lock; ffs's and mfs's buffer over
    /// 0xb400..0xd000 holds it, so it takes turns with these tests).
    const UWORDS: usize = crate::testutil::USER_BASE + 0xc400;

    /// Callbacks that count the changes (refusing them with `EIO` when `fail`) and report a
    /// full-duplex link.
    struct Cb {
        changes: AtomicU32,
        fail: bool,
    }

    impl MediaCallbacks for Cb {
        fn change(&self, _ifp: &'static Ifnet) -> Result<(), Errno> {
            self.changes.fetch_add(1, Ordering::Relaxed);
            if self.fail { Err(Errno::EIO) } else { Ok(()) }
        }

        fn status(&self, _ifp: &'static Ifnet, ifmr: &mut Ifmediareq) {
            ifmr.ifm_status = IFM_AVALID | IFM_ACTIVE;
            ifmr.ifm_active |= IFM_FDX;
        }
    }

    /// A media with the 10/100 copper words and autoselect, set to autoselect: what a
    /// driver's attach makes in its zeroed softc.
    fn copper(fail: bool) -> (&'static Ifmedia, &'static Cb) {
        let ifm: &'static Ifmedia = Box::leak(Box::new(Ifmedia::new()));
        let cb: &'static Cb = Box::leak(Box::new(Cb {
            changes: AtomicU32::new(0),
            fail,
        }));
        ifmedia_init(ifm, 0, cb);
        ifmedia_add(ifm, IFM_ETHER | IFM_10_T, 1, 0);
        ifmedia_add(ifm, IFM_ETHER | IFM_100_TX | IFM_FDX, 2, 0);
        ifmedia_add(ifm, IFM_ETHER | IFM_AUTO, 3, 0);
        ifmedia_set(ifm, IFM_ETHER | IFM_AUTO);
        (ifm, cb)
    }

    /// `ifmedia_ioctl` on the image of the request `req`.
    fn ioctl<T: hal::user::AbiPod>(
        ifp: &'static Ifnet,
        ifm: &Ifmedia,
        cmd: u64,
        req: &mut T,
    ) -> Result<(), Errno> {
        ifmedia_ioctl(
            ifp,
            hal::user::bytes_of_mut(core::slice::from_mut(req)),
            ifm,
            cmd,
        )
    }

    #[test]
    fn a_zeroed_media_is_empty_until_it_is_initialised() {
        let ifm = Ifmedia::new();
        assert!(ifm.ifm_cur().is_none() && ifm.callbacks().is_none());
        assert_eq!(
            (ifm.ifm_nwords(), ifm.ifm_media.get(), ifm.ifm_mask.get()),
            (0, 0, 0)
        );
        // An empty list for the walks, as the C's zeroed TAILQ.
        assert!(!ifmedia_match(&ifm, IFM_ETHER | IFM_AUTO, 0));
        ifmedia_delete_instance(&ifm, IFM_INST_ANY);
        assert!(ifm.ifm_cur().is_none());
    }

    #[test]
    fn add_set_match_and_delete() {
        let _g = setup();
        let (ifm, _) = copper(false);
        assert_eq!(ifm.ifm_nwords(), 3);
        assert_eq!(ifm.ifm_cur().map(|e| e.ifm_data), Some(3));
        assert!(ifmedia_match(ifm, IFM_ETHER | IFM_100_TX | IFM_FDX, 0));
        assert!(!ifmedia_match(ifm, IFM_ETHER | IFM_100_TX, 0));
        assert!(ifmedia_match(ifm, IFM_ETHER | IFM_100_TX, IFM_FDX));
        assert_eq!(
            ifmedia_words(ifm),
            [
                IFM_ETHER | IFM_10_T,
                IFM_ETHER | IFM_100_TX | IFM_FDX,
                IFM_ETHER | IFM_AUTO
            ]
        );

        // The first added of several matches is selected.
        ifmedia_add(ifm, IFM_ETHER | IFM_10_T, 4, 0);
        ifmedia_set(ifm, IFM_ETHER | IFM_10_T);
        assert_eq!(ifm.ifm_cur().map(|e| e.ifm_data), Some(1));

        // A word not on the list falls back to IFM_NONE, which is added.
        ifmedia_set(ifm, IFM_ETHER | IFM_1000_T);
        assert_eq!(ifm.ifm_nwords(), 5);
        assert_eq!(
            ifm.ifm_cur().map(|e| e.ifm_media),
            Some(IFM_ETHER | IFM_NONE)
        );

        ifmedia_add(ifm, ifm_makeword(IFM_ETHER, IFM_10_T, 0, 1), 0, 0);
        ifmedia_delete_instance(ifm, 1);
        assert_eq!(ifm.ifm_nwords(), 5);
        assert!(ifm.ifm_cur().is_none());
        assert_eq!(
            ifmedia_words(ifm),
            [
                IFM_ETHER | IFM_10_T,
                IFM_ETHER | IFM_100_TX | IFM_FDX,
                IFM_ETHER | IFM_AUTO,
                IFM_ETHER | IFM_10_T,
                IFM_ETHER | IFM_NONE
            ]
        );
        ifmedia_delete_instance(ifm, IFM_INST_ANY);
        assert_eq!(ifm.ifm_nwords(), 0);
        assert!(!ifmedia_match(ifm, IFM_ETHER | IFM_AUTO, 0));
    }

    #[test]
    fn ioctl_sets_media_and_restores_on_failure() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tmedia0");
        let (ifm, cb) = copper(false);
        let mut ifr = Ifreq::zeroed();

        ifr.set_ifr_media(IFM_ETHER | IFM_100_TX | IFM_FDX);
        assert_eq!(ioctl(ifp, ifm, SIOCSIFMEDIA, &mut ifr), Ok(()));
        assert_eq!(ifm.ifm_media.get(), IFM_ETHER | IFM_100_TX | IFM_FDX);
        assert_eq!(ifm.ifm_cur().map(|e| e.ifm_data), Some(2));
        assert_eq!(cb.changes.load(Ordering::Relaxed), 1);
        // The same word again: nothing to do.
        assert_eq!(ioctl(ifp, ifm, SIOCSIFMEDIA, &mut ifr), Ok(()));
        assert_eq!(cb.changes.load(Ordering::Relaxed), 1);

        ifr.set_ifr_media(IFM_ETHER | IFM_1000_T);
        assert_eq!(ioctl(ifp, ifm, SIOCSIFMEDIA, &mut ifr), Err(Errno::EINVAL));

        let (ifm2, _) = copper(true);
        ifr.set_ifr_media(IFM_ETHER | IFM_10_T);
        assert_eq!(ioctl(ifp, ifm2, SIOCSIFMEDIA, &mut ifr), Err(Errno::EIO));
        assert_eq!(ifm2.ifm_cur().map(|e| e.ifm_data), Some(3));
        assert_eq!(ifm2.ifm_media.get(), 0);

        // A request shorter than an ifreq is refused, as a NULL one is in C.
        assert_eq!(
            ifmedia_ioctl(ifp, &mut [0u8; 8], ifm2, SIOCSIFMEDIA),
            Err(Errno::EINVAL)
        );
    }

    #[test]
    fn ioctl_reports_the_status_and_copies_the_words_out() {
        let _g = setup();
        let (ifp, _) = test_ifnet(b"tmedia1");
        let (ifm, _) = copper(false);
        let mut ifmr: Ifmediareq = hal::user::zeroed();

        assert_eq!(ioctl(ifp, ifm, SIOCGIFMEDIA, &mut ifmr), Ok(()));
        assert_eq!(ifmr.ifm_count, 3);
        assert_eq!(ifmr.ifm_current, IFM_ETHER | IFM_AUTO);
        assert_eq!(ifmr.ifm_active, IFM_ETHER | IFM_AUTO | IFM_FDX);
        assert_eq!(ifmr.ifm_status, IFM_AVALID | IFM_ACTIVE);

        ifmr.ifm_count = 2;
        assert_eq!(ioctl(ifp, ifm, SIOCGIFMEDIA, &mut ifmr), Err(Errno::E2BIG));
        assert_eq!(ifmr.ifm_count, 3);

        ifmr.ifm_count = 4;
        ifmr.ifm_ulist = UWORDS as u64;
        assert_eq!(ioctl(ifp, ifm, SIOCGIFMEDIA, &mut ifmr), Ok(()));
        assert_eq!(ifmr.ifm_count, 3);
        let mut words = [0u64; 3];
        UserPtr::<u64>::new(UWORDS)
            .copyin_slice(&mut words)
            .expect("user memory");
        assert_eq!(
            words,
            [
                IFM_ETHER | IFM_10_T,
                IFM_ETHER | IFM_100_TX | IFM_FDX,
                IFM_ETHER | IFM_AUTO
            ]
        );

        ifmr.ifm_count = -1;
        assert_eq!(ioctl(ifp, ifm, SIOCGIFMEDIA, &mut ifmr), Err(Errno::EINVAL));

        assert_eq!(ioctl(ifp, ifm, SIOCSIFADDR, &mut ifmr), Err(Errno::ENOTTY));
    }

    #[test]
    fn a_list_that_shrank_since_it_was_counted_copies_only_its_words() {
        let _g = setup();
        let (ifm, _) = copper(false);
        let rt = ifm.rt("test");
        let mut ifmr: Ifmediareq = hal::user::zeroed();
        ifmr.ifm_count = 3;
        ifmr.ifm_ulist = UWORDS as u64;
        UserPtr::<u64>::new(UWORDS)
            .copyout_slice(&[0xaaaa; 4])
            .expect("user memory");

        // Counted at 4 without the mutex (an entry deleted since): a 4-word buffer, the
        // list's 3 words copied, the user's fourth word untouched.
        assert_eq!(ifmedia_copywords(&rt.list, &mut ifmr, 4), Ok(()));
        assert_eq!(ifmr.ifm_count, 3);
        let mut words = [0u64; 4];
        UserPtr::<u64>::new(UWORDS)
            .copyin_slice(&mut words)
            .expect("user memory");
        assert_eq!(
            words,
            [
                IFM_ETHER | IFM_10_T,
                IFM_ETHER | IFM_100_TX | IFM_FDX,
                IFM_ETHER | IFM_AUTO,
                0xaaaa
            ]
        );

        // Counted at 2 (an entry added since): the buffer made again at the list's 3.
        ifmr.ifm_count = 3;
        assert_eq!(ifmedia_copywords(&rt.list, &mut ifmr, 2), Ok(()));
        assert_eq!(ifmr.ifm_count, 3);
        // A user's array shorter than the list: E2BIG and the list's length.
        ifmr.ifm_count = 2;
        assert_eq!(ifmedia_copywords(&rt.list, &mut ifmr, 3), Err(Errno::E2BIG));
        assert_eq!(ifmr.ifm_count, 3);
    }

    #[test]
    fn baudrate_of_known_and_unknown_words() {
        assert_eq!(
            ifmedia_baudrate(IFM_ETHER | IFM_100_TX | IFM_FDX),
            if_mbps(100)
        );
        assert_eq!(ifmedia_baudrate(IFM_ETHER | IFM_1000_T), if_mbps(1000));
        assert_eq!(
            ifmedia_baudrate(IFM_IEEE80211 | IFM_IEEE80211_HT_MCS0),
            if_kbps(6500)
        );
        assert_eq!(ifmedia_baudrate(IFM_ETHER | IFM_AUTO), 0);
        assert_eq!(ifmedia_baudrate(IFM_ETHER | IFM_100G_SR4), 0);
    }
}
/* </TESTS> */
