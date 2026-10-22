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
 * Copyright (c) 1999,2000 Jonathan Lemon <jlemon@FreeBSD.org>
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
 *	$FreeBSD: src/sys/sys/eventvar.h,v 1.3 2000/05/26 02:06:54 jake Exp $
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/eventvar.h>`: `struct kqueue`, the kernel side of a `kqueue(2)` descriptor (and of a
//! thread's `select(2)`/`poll(2)` queue, `p_kq`), as `bsd` names it: the kernel crate's
//! (`kernel::event`), instantiated with `bsd`'s process glue.
//!
//! Upstream: sys/sys/eventvar.h @ 3ce1f3f79392
//! LZ: sys/sys/eventvar.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.4: `struct kqueue` is `kernel::event::Kqueue`, a counted `kqueuepl` item held by
//!   `KqRef`s (LZ: a `&'static` pool item whose `kq_fdp`, `kq_knlist` and `kq_knhash` were
//!   raw pointers read in `unsafe` accessors, `Sync` by an `unsafe impl`); the table's
//!   `fd_kqlist` adapter (`KqList`) and `Proc::kq` went with it (`kernel::event::thread_kq`).

use crate::kern::proc_glue::BsdProcGlue;

pub use kernel::event::{KQ_DYING, KQ_NEVENTS, KQ_SLEEP, KQ_TASK, KQEXTENT};

/// `struct kqueue` of `bsd`'s processes.
pub type Kqueue = kernel::event::Kqueue<BsdProcGlue>;
/// A counted reference to a kqueue of `bsd`'s processes.
pub type KqRef = kernel::event::KqRef<BsdProcGlue>;
/* </CODE> */
