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
/* </LICENSES> */

/* <CODE> */
//! The Berkeley packet filter, bpf(4), for `bsd`'s callers: `<net/bpf.h>`'s ABI is
//! `hal::sys::net_bpf`'s and the device and the taps are the kernel's (`kernel::net::bpf`,
//! N6.3c1); this module is their adapter, of no logic: the taps and the attach functions
//! under LZ's names for the drivers, and the device's entry points as plain functions of
//! `bsd`'s glue for the character device switch (major 23) and `pledge(2)`'s check, which
//! compare `d_open` with `bpfopen` by address (a generic instantiation has no one address
//! across codegen units).
//!
//! Upstream: sys/net/bpf.h @ 3ce1f3f79392
//! Upstream: sys/net/bpf.c @ 3ce1f3f79392

use core::ptr;

use kernel::sysctl::SysctlReq;

use crate::kern::proc_glue::BsdProcGlue;
use crate::machine::conf::{cdevsw, nchrdev};
use crate::sys::conf::DevTypeOpen;
use crate::sys::errno::Errno;
use crate::sys::event::Knote;
use crate::sys::proc::Proc;
use crate::sys::types::Dev;
use crate::sys::uio::Uio;

pub use hal::sys::net_bpf::*;
pub use kernel::net::bpf::{
    BpfIf, BpfIfRef, BpfTap, NBPFILTER, bpf_mtap, bpf_mtap_af, bpf_mtap_ether, bpf_mtap_hdr,
    bpf_tap_hdr, bpfattach, bpfdetach, bpfsattach, bpfsdetach, bpfxattach,
};

/// `bpfopen` (the device switch's and `pledge(2)`'s one address): the kernel's for `bsd`'s
/// glue.
pub fn bpfopen(dev: Dev, flag: i32, mode: i32, p: &Proc) -> Result<(), Errno> {
    kernel::net::bpf::bpfopen::<BsdProcGlue>(dev, flag, mode, p)
}

/// `bpfclose`.
pub fn bpfclose(dev: Dev, flag: i32, mode: i32, p: Option<&Proc>) -> Result<(), Errno> {
    kernel::net::bpf::bpfclose::<BsdProcGlue>(dev, flag, mode, p)
}

/// `bpfread`.
pub fn bpfread(dev: Dev, uio: &mut Uio<'_>, ioflag: i32) -> Result<(), Errno> {
    kernel::net::bpf::bpfread::<BsdProcGlue>(dev, uio, ioflag)
}

/// `bpfwrite`.
pub fn bpfwrite(dev: Dev, uio: &mut Uio<'_>, ioflag: i32) -> Result<(), Errno> {
    kernel::net::bpf::bpfwrite::<BsdProcGlue>(dev, uio, ioflag)
}

/// `bpfioctl`.
pub fn bpfioctl(dev: Dev, cmd: u64, addr: &mut [u8], flag: i32, p: &Proc) -> Result<(), Errno> {
    kernel::net::bpf::bpfioctl::<BsdProcGlue>(dev, cmd, addr, flag, p)
}

/// `bpfkqfilter`.
pub fn bpfkqfilter(dev: Dev, kn: &Knote) -> Result<(), Errno> {
    kernel::net::bpf::bpfkqfilter::<BsdProcGlue>(dev, kn)
}

/// `bpfilterattach`: the major of `/dev/bpf` found by `bpfsdetach`'s scan of the device
/// switch (the C makes it at each detach; the table is constant), handed to the kernel's
/// attach with `bsd`'s glue.
pub fn bpfilterattach(n: i32) {
    let maj = (0..nchrdev())
        .find(|&maj| ptr::fn_addr_eq(cdevsw(maj).d_open, bpfopen as DevTypeOpen))
        .unwrap_or(nchrdev());
    kernel::net::bpf::bpfilterattach::<BsdProcGlue>(n, maj);
}

/// `bpf_sysctl`: `net.bpf.*`, from `net_sysctl`'s `PF_BPF` arm.
pub fn bpf_sysctl(
    name: &[i32],
    oldp: usize,
    oldlenp: &mut usize,
    newp: usize,
    newlen: usize,
) -> Result<(), Errno> {
    let mut req = SysctlReq {
        oldp,
        oldlenp,
        newp,
        newlen,
    };
    kernel::net::bpf::bpf_sysctl(name, &mut req)
}

/// Host tests: the kernel's bpf state of a fresh test (`kernel::net::bpf::bpf_test_reset`).
#[cfg(test)]
pub(crate) fn bpf_test_reset() {
    kernel::net::bpf::bpf_test_reset();
}
/* </CODE> */
