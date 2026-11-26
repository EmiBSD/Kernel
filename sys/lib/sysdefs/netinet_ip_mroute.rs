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
//! The socket options of `<netinet/ip_mroute.h>`: the multicast routing requests (`MRT_*`)
//! a raw IP socket's `setsockopt(2)` and `getsockopt(2)` name at level `IPPROTO_IP`.
//!
//! Upstream: sys/netinet/ip_mroute.h @ 3ce1f3f79392
//! Upstream: sys/netinet/raw_ip.c @ 3ce1f3f79392
//! LZ: sys/netinet/raw_ip.rs@3c62ede68802
//!
//! ## Deviations
//! - Only the option names: `MROUTING` (`ip_mroute.c`) is not configured, so `rip_ctloutput`
//!   answers each `EOPNOTSUPP`, the C's `#else`; the header's structures (`struct vifctl`,
//!   `struct mfcctl2`, ...) and statistics have no user here.
//!
//! ## Redesign
//! - N6.6b1: the ten names LZ's `sys/netinet/raw_ip.rs` defined (`<netinet/ip_mroute.h>`
//!   was not ported) move into `hal::sys` under their header's name, where the kernel's raw
//!   IP (`kernel::netinet::raw_ip`) names them.

/// `MRT_INIT`: initialize forwarder.
pub const MRT_INIT: i32 = 100;
/// `MRT_DONE`: shut down forwarder.
pub const MRT_DONE: i32 = 101;
/// `MRT_ADD_VIF`: create virtual interface.
pub const MRT_ADD_VIF: i32 = 102;
/// `MRT_DEL_VIF`: delete virtual interface.
pub const MRT_DEL_VIF: i32 = 103;
/// `MRT_ADD_MFC`: insert forwarding cache entry.
pub const MRT_ADD_MFC: i32 = 104;
/// `MRT_DEL_MFC`: delete forwarding cache entry.
pub const MRT_DEL_MFC: i32 = 105;
/// `MRT_VERSION`: get kernel version number.
pub const MRT_VERSION: i32 = 106;
/// `MRT_ASSERT`: enable assert processing.
pub const MRT_ASSERT: i32 = 107;
/// `MRT_API_SUPPORT`: supported MRT API.
pub const MRT_API_SUPPORT: i32 = 109;
/// `MRT_API_CONFIG`: config MRT API.
pub const MRT_API_CONFIG: i32 = 110;
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/netinet/ip_mroute.h");
        let names = crate::reftest::assert_defines!(defs;
            MRT_INIT, MRT_DONE, MRT_ADD_VIF, MRT_DEL_VIF, MRT_ADD_MFC, MRT_DEL_MFC,
            MRT_VERSION, MRT_ASSERT, MRT_API_SUPPORT, MRT_API_CONFIG);
        assert_eq!(names.len(), 10);
    }
}
/* </TESTS> */
