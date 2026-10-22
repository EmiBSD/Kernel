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
 * Copyright (c) 1982, 1986, 1993
 *	The Regents of the University of California.  All rights reserved.
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
 *	@(#)resource.h	8.2 (Berkeley) 1/4/94
 */
/* </LICENSES> */

/* <CODE> */
//! `<sys/resource.h>`: priorities, resource usage and limits.
//!
//! Upstream: sys/sys/resource.h @ 3ce1f3f79392
//!
//! Status: an adapter (N4.2). `struct rusage`, `struct rlimit`, the `RLIMIT_*`/`RLIM_*`
//! constants and `RUSAGE_SIZE` are the kernel crate's (`kernel::resource`); this header keeps
//! the `PRIO_*` and `RUSAGE_*` constants and `struct loadavg`. `dosetrlimit`, `donice` and
//! `dogetrusage` are `kern_resource.c`'s.
//!
//! ## Deviations
//! - `struct rusage`'s counters are relaxed atomic words (`kernel::resource::Word`), its
//!   times `kernel::resource::Timeval`s: the owning thread bumps them (`ru_nvcsw++`) and a
//!   pooled thread must be `Send` and `Sync`.

use crate::sys::types::Fixpt;

pub use kernel::resource::{
    RLIM_INFINITY, RLIM_NLIMITS, RLIM_SAVED_CUR, RLIM_SAVED_MAX, RLIMIT_CORE, RLIMIT_CPU,
    RLIMIT_DATA, RLIMIT_FSIZE, RLIMIT_MEMLOCK, RLIMIT_NOFILE, RLIMIT_NPROC, RLIMIT_RSS,
    RLIMIT_STACK, RUSAGE_SIZE, Rlimit, Rusage,
};

/// `PRIO_MIN`.
pub const PRIO_MIN: i32 = -20;
/// `PRIO_MAX`.
pub const PRIO_MAX: i32 = 20;

/// `PRIO_PROCESS`.
pub const PRIO_PROCESS: i32 = 0;
/// `PRIO_PGRP`.
pub const PRIO_PGRP: i32 = 1;
/// `PRIO_USER`.
pub const PRIO_USER: i32 = 2;

/// `RUSAGE_SELF`.
pub const RUSAGE_SELF: i32 = 0;
/// `RUSAGE_CHILDREN`.
pub const RUSAGE_CHILDREN: i32 = -1;
/// `RUSAGE_THREAD`.
pub const RUSAGE_THREAD: i32 = 1;

/// `struct loadavg`: load average structure.
#[derive(Clone, Copy, Debug, Default)]
pub struct Loadavg {
    /// `ldavg`.
    pub ldavg: [Fixpt; 3],
    /// `fscale`.
    pub fscale: i64,
}
/* </CODE> */
