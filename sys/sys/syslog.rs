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
 * Copyright (c) 1982, 1986, 1988, 1993
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
 *	@(#)syslog.h	8.1 (Berkeley) 6/2/93
 */
/* </LICENSES> */

/* <CODE> */
//! Priorities and facilities of the system log: `<sys/syslog.h>`, as the kernel sees it, for
//! `bsd`: the kernel crate's (`kernel::log`), re-exported at the path its callers use.
//!
//! Upstream: sys/sys/syslog.h @ 3ce1f3f79392
//! LZ: sys/sys/syslog.rs@3c62ede68802
//!
//! ## Redesign
//! - N4.7a: the header moved into `kernel::log` with the message buffer and
//!   `sendsyslog(2)`, which read it; its 37 users in `bsd` keep this path. LZ's deviation
//!   (`struct syslog_data`, `SYSLOG_NAMES` and the userland prototypes are not kernel
//!   material) went with it.

pub use kernel::log::{
    _PATH_LOG, LIOCSFD, LOG_ALERT, LOG_AUTH, LOG_AUTHPRIV, LOG_CONS, LOG_CRIT, LOG_CRON,
    LOG_DAEMON, LOG_DEBUG, LOG_EMERG, LOG_ERR, LOG_FACMASK, LOG_FTP, LOG_INFO, LOG_KERN,
    LOG_LOCAL0, LOG_LOCAL1, LOG_LOCAL2, LOG_LOCAL3, LOG_LOCAL4, LOG_LOCAL5, LOG_LOCAL6, LOG_LOCAL7,
    LOG_LPR, LOG_MAIL, LOG_MAXLINE, LOG_NDELAY, LOG_NEWS, LOG_NFACILITIES, LOG_NOTICE, LOG_NOWAIT,
    LOG_ODELAY, LOG_PERROR, LOG_PID, LOG_PRIMASK, LOG_PRINTF, LOG_SYSLOG, LOG_USER, LOG_UUCP,
    LOG_WARNING, log_fac, log_mask, log_pri, log_upto,
};
/* </CODE> */
