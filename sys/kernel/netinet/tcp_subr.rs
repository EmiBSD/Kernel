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
 * Copyright (c) 1982, 1986, 1988, 1990, 1993
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
 *	@(#)COPYRIGHT	1.1 (NRL) 17 January 1995
 *
 * NRL grants permission for redistribution and use in source and binary
 * forms, with or without modification, of the software and documentation
 * created at NRL provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgements:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 *	This product includes software developed at the Information
 *	Technology Division, US Naval Research Laboratory.
 * 4. Neither the name of the NRL nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THE SOFTWARE PROVIDED BY NRL IS PROVIDED BY NRL AND CONTRIBUTORS ``AS
 * IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
 * TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
 * PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL NRL OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * The views and conclusions contained in the software and documentation
 * are those of the authors and should not be interpreted as representing
 * official policies, either expressed or implied, of the US Naval
 * Research Laboratory (NRL).
 */
/* </LICENSES> */

/* <CODE> */
//! TCP support routines: `netinet/tcp_subr.c`.
//!
//! Upstream: sys/netinet/tcp_subr.c @ 3ce1f3f79392
//! LZ: sys/netinet/tcp_subr.rs@3c62ede68802
//!
//! The file's globals: the tunables `sysctl(2)` sets, the reassembly and SACK hole pools
//! with their hard limits, the ISN secret and counter, the random offset of `tcp_now`.
//!
//! Locks used to protect struct members in this file: \[I\] immutable after creation,
//! \[T\] `tcp_timer_mtx` (global tcp timer data structures), \[a\] atomic.
//!
//! ## Deviations
//! - The sysctl-controlled `int`s are `AtomicI32`s (the C reads them with
//!   `atomic_load_int`).
//! - `tcp_reass_limit` starts at `NMBCLUSTERS / 8` in the C's static initializer; here
//!   `tcp_init` sets it from `nmbclust` (`kern.maxclusters`), whose boot value is the
//!   machine's `NMBCLUSTERS` (`mbinit` runs before `domaininit`), since the kernel crate
//!   names no machine constant: the same number.
//!
//! ## Redesign
//! - N6.6a2: the kernel's TCP is built beside `bsd`'s, unregistered until N6.6a5 (D-2):
//!   these globals are the kernel's own, `bsd`'s `sys/netinet/tcp_subr.rs` keeps its until
//!   the switch. `tcpqe_pool` and `sackhl_pool` are `BoxPool`s whose items the queues own
//!   (`hal::pool::Pooled`), `tcpcb_pool` is the glue's (`tcp_var::TcpTables`); `tcp_iss` is
//!   the data of `tcp_timer_mtx` (a `Mutexed`, where LZ had an atomic beside a bare mutex);
//!   `tcp_secret` and `tcp_secret_ctx` are `OnceStatic`s set by `tcp_init` (LZ:
//!   `StaticCell`s written under `unsafe`).

use core::sync::atomic::{AtomicI32, AtomicU64};

use hal::intr::IPL_SOFTNET;
use hal::mutex::Mutexed;
use hal::once::OnceStatic;
use hal::pool::BoxPool;
use hal::sys::netinet_tcp::{TCP_MSS, TcpSeq};

use crate::crypto::sha2::Sha512Ctx;
use crate::netinet::tcp_timer::TCPTV_SRTTDFLT;
use crate::netinet::tcp_var::{Sackhole, Tcpqent};

// patchable/settable parameters for tcp

/// \[a\] `tcp_mssdflt`: default maximum segment size.
pub static TCP_MSSDFLT: AtomicI32 = AtomicI32::new(TCP_MSS);
/// `tcp_rttdflt`.
pub static TCP_RTTDFLT: AtomicI32 = AtomicI32::new(TCPTV_SRTTDFLT);

// values controllable via sysctl

/// \[a\] `tcp_do_rfc1323`.
pub static TCP_DO_RFC1323: AtomicI32 = AtomicI32::new(1);
/// \[a\] `tcp_do_sack`: RFC 2018 selective ACKs.
pub static TCP_DO_SACK: AtomicI32 = AtomicI32::new(1);
/// \[a\] `tcp_ack_on_push`: set to enable immediate ACK-on-PUSH.
pub static TCP_ACK_ON_PUSH: AtomicI32 = AtomicI32::new(0);
/// \[a\] `tcp_do_ecn`: RFC3168 ECN enabled/disabled? (`TCP_ECN`).
pub static TCP_DO_ECN: AtomicI32 = AtomicI32::new(0);
/// \[a\] `tcp_do_rfc3390`: increase TCP's Initial Window to 10*mss.
pub static TCP_DO_RFC3390: AtomicI32 = AtomicI32::new(2);
/// \[a\] `tcp_do_tso`: TCP segmentation offload for output.
pub static TCP_DO_TSO: AtomicI32 = AtomicI32::new(1);

/// `tcp_reass_limit`: hardlimit for `tcpqe_pool`; `NMBCLUSTERS / 8`, set by `tcp_init` (see
/// the deviations).
pub static TCP_REASS_LIMIT: AtomicI32 = AtomicI32::new(0);
/// `tcp_sackhole_limit`: hardlimit for `sackhl_pool`.
#[allow(non_upper_case_globals)] // TCP_SACKHOLE_LIMIT is a constant of tcp.h
pub static tcp_sackhole_limit: AtomicI32 = AtomicI32::new(32 * 1024);

/// `tcpqe_pool`: the reassembly queue entries.
pub static TCPQE_POOL: BoxPool<Tcpqent> = BoxPool::new();
/// `sackhl_pool`: the SACK holes.
pub static SACKHL_POOL: BoxPool<Sackhole> = BoxPool::new();

/// \[I\] `tcp_secret`.
pub(crate) static TCP_SECRET: OnceStatic<[u8; 16]> = OnceStatic::new();
/// \[I\] `tcp_secret_ctx`: the SHA-512 context over the secret.
pub(crate) static TCP_SECRET_CTX: OnceStatic<Sha512Ctx> = OnceStatic::new();
/// `tcp_timer_mtx` and what it keeps: \[T\] `tcp_iss`, updated by timer and connection.
pub static TCP_ISS: Mutexed<TcpSeq> = Mutexed::new(IPL_SOFTNET, 0);
/// \[I\] `tcp_starttime`: random offset for `tcp_now()`.
pub static TCP_STARTTIME: AtomicU64 = AtomicU64::new(0);
/* </CODE> */
