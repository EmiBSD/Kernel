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
//! The Berkeley packet filter's ABI, `<net/bpf.h>`: the filter language (the instruction
//! classes, modes, operations and `struct bpf_insn`), the device's user interface (the
//! `BIOC*` ioctls and the structures they pass: `struct bpf_program`, `bpf_stat`,
//! `bpf_version`, `bpf_dltlist`), the record `read(2)` returns (`struct bpf_hdr`) and the
//! data-link types (`DLT_*`). libpcap, tcpdump(8) and the kernel's `kernel::net::bpf` and
//! `bpf_filter` share them; `bsd`'s drivers name the `DLT_*` and `BPF_DIRECTION_*` through
//! `bsd`'s `net::bpf`.
//!
//! Upstream: sys/net/bpf.h @ 3ce1f3f79392
//! Upstream: sys/net/bpf.c @ 3ce1f3f79392
//! LZ: sys/net/bpf.rs@3c62ede68802
//!
//! A constant takes the type of the member that stores it: `DLT_*` are `u32` (`bif_dlt`,
//! `BIOCGDLT`'s `u_int`), the instruction codes `u16` (`code`), `BPF_FILDROP_*` and the
//! `BPF_F_*` header flags `u8` (`bd_fildrop`, `bh_flags`). The structures are the C's images
//! on LP64 (amd64 and arm64 alike), `AbiPod` by [`crate::abi_pod!`], their sizes asserted at
//! compile time.
//!
//! ## Deviations
//! - The LP64 holes of `struct bpf_program` and `struct bpf_dltlist` (after the `u_int`,
//!   before the pointer) and the two tail bytes of `struct bpf_hdr` are members (`_pad0`), so
//!   every byte of an image is initialised.
//! - The user pointers `bf_insns` and `bfl_list` are `u64` user addresses.
//! - `BPF_CLASS(code)`, `BPF_SIZE`, `BPF_MODE`, `BPF_OP`, `BPF_SRC`, `BPF_RVAL`, `BPF_MISCOP`,
//!   `BPF_WORDALIGN`, `BPF_STMT` and `BPF_JUMP` are `const fn`s with the lowercase names.
//! - `struct bpf_ops` and the userland prototypes (`bpf_filter`, `_bpf_filter`) are not here:
//!   the kernel's loads are `kernel::net::bpf_filter`'s, the userland functions libpcap's.
//! - The `#ifdef _KERNEL` constants `BPF_MAXINSNS` and `SIZEOF_BPF_HDR` are here with the
//!   rest, as LZ had them.
//!
//! ## Redesign
//! - LZ's `sys/net/bpf.rs` held bpf.h's ABI beside bpf.c's device and taps; the ABI is here,
//!   in `hal` (N6.3c; the `net_if` and `net_route` precedents of N6.3a and N6.3b), so the
//!   kernel's bpf and `bpf_filter` and `bsd`'s drivers name one set of constants and images.
//! - LZ's `#[repr(C)]` structures with an `unsafe impl AbiPod` each and a runtime size check
//!   -> `abi_pod!` images (no `unsafe`) with `const` size asserts.
//! - The `BIOCSRTIMEOUT`/`BIOCGRTIMEOUT`/`BIOCSWTIMEOUT`/`BIOCGWTIMEOUT` numbers are sized by
//!   `struct timeval`'s image in `hal`, [`IfTimeval`] (16 bytes on LP64, as `kernel::clock`'s
//!   `Timeval`), and `BIOCGETIF`/`BIOCSETIF` by [`Ifreq`].

use core::mem::size_of;

use crate::sys::ioccom::{_io, _ior, _iow, _iowr};
use crate::sys::net_if::{IfTimeval, Ifreq};

/// `BPF_RELEASE`: BSD style release date.
pub const BPF_RELEASE: i32 = 199606;

/// `BPF_ALIGNMENT`: what `BPF_WORDALIGN` rounds to (at least what a timeval needs), `sizeof
/// (u_int32_t)`.
pub const BPF_ALIGNMENT: usize = size_of::<u32>();

/// `BPF_MAXINSNS`: the longest program the kernel accepts.
pub const BPF_MAXINSNS: u32 = 512;

/// `BPF_MAXBUFSIZE`: the default of `net.bpf.maxbufsize`.
pub const BPF_MAXBUFSIZE: i32 = 2 * 1024 * 1024;
/// `BPF_MINBUFSIZE`: the smallest buffer `BIOCSBLEN` grants.
pub const BPF_MINBUFSIZE: i32 = 32;

/// `BPF_MAJOR_VERSION`: the filter architecture's version number.
pub const BPF_MAJOR_VERSION: u16 = 1;
/// `BPF_MINOR_VERSION`.
pub const BPF_MINOR_VERSION: u16 = 1;

/// `BIOCGBLEN`.
pub const BIOCGBLEN: u64 = _ior::<u32>(b'B', 102);
/// `BIOCSBLEN`.
pub const BIOCSBLEN: u64 = _iowr::<u32>(b'B', 102);
/// `BIOCSETF`.
pub const BIOCSETF: u64 = _iow::<BpfProgram>(b'B', 103);
/// `BIOCFLUSH`.
pub const BIOCFLUSH: u64 = _io(b'B', 104);
/// `BIOCPROMISC`.
pub const BIOCPROMISC: u64 = _io(b'B', 105);
/// `BIOCGDLT`.
pub const BIOCGDLT: u64 = _ior::<u32>(b'B', 106);
/// `BIOCGETIF`.
pub const BIOCGETIF: u64 = _ior::<Ifreq>(b'B', 107);
/// `BIOCSETIF`.
pub const BIOCSETIF: u64 = _iow::<Ifreq>(b'B', 108);
/// `BIOCSRTIMEOUT`.
pub const BIOCSRTIMEOUT: u64 = _iow::<IfTimeval>(b'B', 109);
/// `BIOCGRTIMEOUT`.
pub const BIOCGRTIMEOUT: u64 = _ior::<IfTimeval>(b'B', 110);
/// `BIOCGSTATS`.
pub const BIOCGSTATS: u64 = _ior::<BpfStat>(b'B', 111);
/// `BIOCIMMEDIATE`.
pub const BIOCIMMEDIATE: u64 = _iow::<u32>(b'B', 112);
/// `BIOCVERSION`.
pub const BIOCVERSION: u64 = _ior::<BpfVersion>(b'B', 113);
/// `BIOCSRSIG`.
pub const BIOCSRSIG: u64 = _iow::<u32>(b'B', 114);
/// `BIOCGRSIG`.
pub const BIOCGRSIG: u64 = _ior::<u32>(b'B', 115);
/// `BIOCGHDRCMPLT`.
pub const BIOCGHDRCMPLT: u64 = _ior::<u32>(b'B', 116);
/// `BIOCSHDRCMPLT`.
pub const BIOCSHDRCMPLT: u64 = _iow::<u32>(b'B', 117);
/// `BIOCLOCK`.
pub const BIOCLOCK: u64 = _io(b'B', 118);
/// `BIOCSETWF`.
pub const BIOCSETWF: u64 = _iow::<BpfProgram>(b'B', 119);
/// `BIOCGFILDROP`.
pub const BIOCGFILDROP: u64 = _ior::<u32>(b'B', 120);
/// `BIOCSFILDROP`.
pub const BIOCSFILDROP: u64 = _iow::<u32>(b'B', 121);
/// `BIOCSDLT`.
pub const BIOCSDLT: u64 = _iow::<u32>(b'B', 122);
/// `BIOCGDLTLIST`.
pub const BIOCGDLTLIST: u64 = _iowr::<BpfDltlist>(b'B', 123);
/// `BIOCGDIRFILT`.
pub const BIOCGDIRFILT: u64 = _ior::<u32>(b'B', 124);
/// `BIOCSDIRFILT`.
pub const BIOCSDIRFILT: u64 = _iow::<u32>(b'B', 125);
/// `BIOCSWTIMEOUT`.
pub const BIOCSWTIMEOUT: u64 = _iow::<IfTimeval>(b'B', 126);
/// `BIOCGWTIMEOUT`.
pub const BIOCGWTIMEOUT: u64 = _ior::<IfTimeval>(b'B', 126);
/// `BIOCDWTIMEOUT`.
pub const BIOCDWTIMEOUT: u64 = _io(b'B', 126);
/// `BIOCSETFNR`.
pub const BIOCSETFNR: u64 = _iow::<BpfProgram>(b'B', 127);

/// `BPF_DIRECTION_IN`: the inbound direction (`BIOCSDIRFILT`/`BIOCGDIRFILT`, the taps).
pub const BPF_DIRECTION_IN: u32 = 1 << 0;
/// `BPF_DIRECTION_OUT`.
pub const BPF_DIRECTION_OUT: u32 = 1 << 1;

/// `BPF_FILDROP_PASS`: capture, pass (`BIOCGFILDROP`/`BIOCSFILDROP`).
pub const BPF_FILDROP_PASS: u8 = 0;
/// `BPF_FILDROP_CAPTURE`: capture, drop.
pub const BPF_FILDROP_CAPTURE: u8 = 1;
/// `BPF_FILDROP_DROP`: no capture, drop.
pub const BPF_FILDROP_DROP: u8 = 2;

/// `BPF_F_PRI_MASK`: `bh_flags`' priority bits.
pub const BPF_F_PRI_MASK: u8 = 0x07;
/// `BPF_F_FLOWID`: `bh_flowid` is set.
pub const BPF_F_FLOWID: u8 = 0x08;
/// `BPF_F_DIR_SHIFT`: where the direction bits start.
pub const BPF_F_DIR_SHIFT: u32 = 4;
/// `BPF_F_DIR_MASK`.
pub const BPF_F_DIR_MASK: u8 = 0x3 << BPF_F_DIR_SHIFT;
/// `BPF_F_DIR_IN`.
pub const BPF_F_DIR_IN: u8 = (BPF_DIRECTION_IN << BPF_F_DIR_SHIFT) as u8;
/// `BPF_F_DIR_OUT`.
pub const BPF_F_DIR_OUT: u8 = (BPF_DIRECTION_OUT << BPF_F_DIR_SHIFT) as u8;

/// `SIZEOF_BPF_HDR`: `sizeof(struct bpf_hdr)`.
pub const SIZEOF_BPF_HDR: usize = size_of::<BpfHdr>();

/// `DLT_NULL`: no link-layer encapsulation.
pub const DLT_NULL: u32 = 0;
/// `DLT_EN10MB`: Ethernet (10Mb).
pub const DLT_EN10MB: u32 = 1;
/// `DLT_EN3MB`: Experimental Ethernet (3Mb).
pub const DLT_EN3MB: u32 = 2;
/// `DLT_AX25`: Amateur Radio AX.25.
pub const DLT_AX25: u32 = 3;
/// `DLT_PRONET`: Proteon ProNET Token Ring.
pub const DLT_PRONET: u32 = 4;
/// `DLT_CHAOS`: Chaos.
pub const DLT_CHAOS: u32 = 5;
/// `DLT_IEEE802`: IEEE 802 Networks.
pub const DLT_IEEE802: u32 = 6;
/// `DLT_ARCNET`: ARCNET.
pub const DLT_ARCNET: u32 = 7;
/// `DLT_SLIP`: Serial Line IP.
pub const DLT_SLIP: u32 = 8;
/// `DLT_PPP`: Point-to-point Protocol.
pub const DLT_PPP: u32 = 9;
/// `DLT_FDDI`: FDDI.
pub const DLT_FDDI: u32 = 10;
/// `DLT_ATM_RFC1483`: LLC/SNAP encapsulated atm.
pub const DLT_ATM_RFC1483: u32 = 11;
/// `DLT_LOOP`: loopback type (af header).
pub const DLT_LOOP: u32 = 12;
/// `DLT_ENC`: IPSEC enc type (af header, spi, flags).
pub const DLT_ENC: u32 = 13;
/// `DLT_RAW`: raw IP.
pub const DLT_RAW: u32 = 14;
/// `DLT_SLIP_BSDOS`: BSD/OS Serial Line IP.
pub const DLT_SLIP_BSDOS: u32 = 15;
/// `DLT_PPP_BSDOS`: BSD/OS Point-to-point Protocol.
pub const DLT_PPP_BSDOS: u32 = 16;
/// `DLT_PFSYNC`: Packet filter state syncing.
pub const DLT_PFSYNC: u32 = 18;
/// `DLT_PPP_SERIAL`: PPP over Serial with HDLC.
pub const DLT_PPP_SERIAL: u32 = 50;
/// `DLT_PPP_ETHER`: PPP over Ethernet; session only w/o ether header.
pub const DLT_PPP_ETHER: u32 = 51;
/// `DLT_C_HDLC`: Cisco HDLC.
pub const DLT_C_HDLC: u32 = 104;
/// `DLT_IEEE802_11`: IEEE 802.11 wireless.
pub const DLT_IEEE802_11: u32 = 105;
/// `DLT_PFLOG`: Packet filter logging, by pcap people.
pub const DLT_PFLOG: u32 = 117;
/// `DLT_IEEE802_11_RADIO`: IEEE 802.11 plus WLAN header.
pub const DLT_IEEE802_11_RADIO: u32 = 127;
/// `DLT_USER0`: reserved for private use.
pub const DLT_USER0: u32 = 147;
/// `DLT_USER1`: reserved for private use.
pub const DLT_USER1: u32 = 148;
/// `DLT_USER2`: reserved for private use.
pub const DLT_USER2: u32 = 149;
/// `DLT_USER3`: reserved for private use.
pub const DLT_USER3: u32 = 150;
/// `DLT_USER4`: reserved for private use.
pub const DLT_USER4: u32 = 151;
/// `DLT_USER5`: reserved for private use.
pub const DLT_USER5: u32 = 152;
/// `DLT_USER6`: reserved for private use.
pub const DLT_USER6: u32 = 153;
/// `DLT_USER7`: reserved for private use.
pub const DLT_USER7: u32 = 154;
/// `DLT_USER8`: reserved for private use.
pub const DLT_USER8: u32 = 155;
/// `DLT_USER9`: reserved for private use.
pub const DLT_USER9: u32 = 156;
/// `DLT_USER10`: reserved for private use.
pub const DLT_USER10: u32 = 157;
/// `DLT_USER11`: reserved for private use.
pub const DLT_USER11: u32 = 158;
/// `DLT_USER12`: reserved for private use.
pub const DLT_USER12: u32 = 159;
/// `DLT_USER13`: reserved for private use.
pub const DLT_USER13: u32 = 160;
/// `DLT_USER14`: reserved for private use.
pub const DLT_USER14: u32 = 161;
/// `DLT_USER15`: reserved for private use.
pub const DLT_USER15: u32 = 162;
/// `DLT_USBPCAP`: USBPcap.
pub const DLT_USBPCAP: u32 = 249;
/// `DLT_MPLS`: MPLS Provider Edge header.
pub const DLT_MPLS: u32 = 219;
/// `DLT_OPENFLOW`: in-kernel OpenFlow, by pcap.
pub const DLT_OPENFLOW: u32 = 267;

/// `BPF_LD`: an instruction class.
pub const BPF_LD: u16 = 0x00;
/// `BPF_LDX`.
pub const BPF_LDX: u16 = 0x01;
/// `BPF_ST`.
pub const BPF_ST: u16 = 0x02;
/// `BPF_STX`.
pub const BPF_STX: u16 = 0x03;
/// `BPF_ALU`.
pub const BPF_ALU: u16 = 0x04;
/// `BPF_JMP`.
pub const BPF_JMP: u16 = 0x05;
/// `BPF_RET`.
pub const BPF_RET: u16 = 0x06;
/// `BPF_MISC`.
pub const BPF_MISC: u16 = 0x07;

/// `BPF_W`: an ld/ldx size.
pub const BPF_W: u16 = 0x00;
/// `BPF_H`.
pub const BPF_H: u16 = 0x08;
/// `BPF_B`.
pub const BPF_B: u16 = 0x10;
/// `BPF_IMM`: an ld/ldx mode.
pub const BPF_IMM: u16 = 0x00;
/// `BPF_ABS`.
pub const BPF_ABS: u16 = 0x20;
/// `BPF_IND`.
pub const BPF_IND: u16 = 0x40;
/// `BPF_MEM`.
pub const BPF_MEM: u16 = 0x60;
/// `BPF_LEN`.
pub const BPF_LEN: u16 = 0x80;
/// `BPF_MSH`.
pub const BPF_MSH: u16 = 0xa0;
/// `BPF_RND`.
pub const BPF_RND: u16 = 0xc0;

/// `BPF_ADD`: an alu/jmp operation.
pub const BPF_ADD: u16 = 0x00;
/// `BPF_SUB`.
pub const BPF_SUB: u16 = 0x10;
/// `BPF_MUL`.
pub const BPF_MUL: u16 = 0x20;
/// `BPF_DIV`.
pub const BPF_DIV: u16 = 0x30;
/// `BPF_OR`.
pub const BPF_OR: u16 = 0x40;
/// `BPF_AND`.
pub const BPF_AND: u16 = 0x50;
/// `BPF_LSH`.
pub const BPF_LSH: u16 = 0x60;
/// `BPF_RSH`.
pub const BPF_RSH: u16 = 0x70;
/// `BPF_NEG`.
pub const BPF_NEG: u16 = 0x80;
/// `BPF_MOD`.
pub const BPF_MOD: u16 = 0x90;
/// `BPF_XOR`.
pub const BPF_XOR: u16 = 0xa0;
/// `BPF_JA`.
pub const BPF_JA: u16 = 0x00;
/// `BPF_JEQ`.
pub const BPF_JEQ: u16 = 0x10;
/// `BPF_JGT`.
pub const BPF_JGT: u16 = 0x20;
/// `BPF_JGE`.
pub const BPF_JGE: u16 = 0x30;
/// `BPF_JSET`.
pub const BPF_JSET: u16 = 0x40;
/// `BPF_K`: an alu/jmp source.
pub const BPF_K: u16 = 0x00;
/// `BPF_X`.
pub const BPF_X: u16 = 0x08;

/// `BPF_A`: a ret source (`BPF_K` and `BPF_X` also apply).
pub const BPF_A: u16 = 0x10;

/// `BPF_TAX`: a misc operation.
pub const BPF_TAX: u16 = 0x00;
/// `BPF_TXA`.
pub const BPF_TXA: u16 = 0x80;

/// `BPF_MEMWORDS`: the scratch memory words (for `BPF_LD|BPF_MEM` and `BPF_ST`).
pub const BPF_MEMWORDS: usize = 16;

/// `struct bpf_program`: `BIOCSETF`'s argument, as user space passes it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfProgram {
    /// `bf_len`: the number of instructions.
    pub bf_len: u32,
    /// The hole before `bf_insns`.
    pub _pad0: u32,
    /// `bf_insns`: the user address of the instructions (`struct bpf_insn *`).
    pub bf_insns: u64,
}

crate::abi_pod!(BpfProgram {
    bf_len: u32,
    _pad0: u32,
    bf_insns: u64,
});

/// `struct bpf_stat`: `BIOCGSTATS`' answer.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfStat {
    /// `bs_recv`: the packets received.
    pub bs_recv: u32,
    /// `bs_drop`: the packets dropped.
    pub bs_drop: u32,
}

crate::abi_pod!(BpfStat {
    bs_recv: u32,
    bs_drop: u32,
});

/// `struct bpf_version`: `BIOCVERSION`'s answer, the version of the filter language (a
/// program is understood when its major equals the kernel's and its minor is at most the
/// kernel's); it has nothing to do with the source's version.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfVersion {
    /// `bv_major`.
    pub bv_major: u16,
    /// `bv_minor`.
    pub bv_minor: u16,
}

crate::abi_pod!(BpfVersion {
    bv_major: u16,
    bv_minor: u16,
});

/// `struct bpf_timeval`: a record's time stamp.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfTimeval {
    /// `tv_sec`.
    pub tv_sec: u32,
    /// `tv_usec`.
    pub tv_usec: u32,
}

crate::abi_pod!(BpfTimeval {
    tv_sec: u32,
    tv_usec: u32,
});

/// `struct bpf_hdr`: the header in front of each packet a read returns.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfHdr {
    /// `bh_tstamp`: the time stamp.
    pub bh_tstamp: BpfTimeval,
    /// `bh_caplen`: the length of the captured part.
    pub bh_caplen: u32,
    /// `bh_datalen`: the packet's length.
    pub bh_datalen: u32,
    /// `bh_hdrlen`: this header's length with the alignment padding after it.
    pub bh_hdrlen: u16,
    /// `bh_ifidx`: the receiving interface's index.
    pub bh_ifidx: u16,
    /// `bh_flowid`.
    pub bh_flowid: u16,
    /// `bh_flags`: `BPF_F_*`.
    pub bh_flags: u8,
    /// `bh_drops`.
    pub bh_drops: u8,
    /// `bh_csumflags`: the checksum flags.
    pub bh_csumflags: u16,
    /// The C structure's tail padding.
    pub _pad0: u16,
}

crate::abi_pod!(BpfHdr {
    bh_tstamp: BpfTimeval,
    bh_caplen: u32,
    bh_datalen: u32,
    bh_hdrlen: u16,
    bh_ifidx: u16,
    bh_flowid: u16,
    bh_flags: u8,
    bh_drops: u8,
    bh_csumflags: u16,
    _pad0: u16,
});

/// `struct bpf_insn`: one instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfInsn {
    /// `code`.
    pub code: u16,
    /// `jt`: the offset taken when the test is true.
    pub jt: u8,
    /// `jf`: when it is false.
    pub jf: u8,
    /// `k`: the operand.
    pub k: u32,
}

crate::abi_pod!(BpfInsn {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
});

/// `struct bpf_dltlist`: `BIOCGDLTLIST`'s argument, the interface's data-link types.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BpfDltlist {
    /// `bfl_len`: the entries of `bfl_list`.
    pub bfl_len: u32,
    /// The hole before `bfl_list`.
    pub _pad0: u32,
    /// `bfl_list`: the user address of the array of DLTs (`u_int *`).
    pub bfl_list: u64,
}

crate::abi_pod!(BpfDltlist {
    bfl_len: u32,
    _pad0: u32,
    bfl_list: u64,
});

/// `BPF_WORDALIGN(x)`: `x` rounded up to a multiple of [`BPF_ALIGNMENT`].
pub const fn bpf_wordalign(x: usize) -> usize {
    (x + (BPF_ALIGNMENT - 1)) & !(BPF_ALIGNMENT - 1)
}

/// `BPF_CLASS(code)`: the instruction class.
pub const fn bpf_class(code: u16) -> u16 {
    code & 0x07
}

/// `BPF_SIZE(code)`: the ld/ldx size field.
pub const fn bpf_size(code: u16) -> u16 {
    code & 0x18
}

/// `BPF_MODE(code)`: the ld/ldx mode field.
pub const fn bpf_mode(code: u16) -> u16 {
    code & 0xe0
}

/// `BPF_OP(code)`: the alu/jmp operation field.
pub const fn bpf_op(code: u16) -> u16 {
    code & 0xf0
}

/// `BPF_SRC(code)`: the alu/jmp source field.
pub const fn bpf_src(code: u16) -> u16 {
    code & 0x08
}

/// `BPF_RVAL(code)`: the ret source field.
pub const fn bpf_rval(code: u16) -> u16 {
    code & 0x18
}

/// `BPF_MISCOP(code)`: the misc operation field.
pub const fn bpf_miscop(code: u16) -> u16 {
    code & 0xf8
}

/// `BPF_STMT(code, k)`: an instruction initialiser.
pub const fn bpf_stmt(code: u16, k: u32) -> BpfInsn {
    BpfInsn {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

/// `BPF_JUMP(code, k, jt, jf)`: a jump instruction initialiser.
pub const fn bpf_jump(code: u16, k: u32, jt: u8, jf: u8) -> BpfInsn {
    BpfInsn { code, jt, jf, k }
}

const _: () = {
    assert!(size_of::<BpfProgram>() == 16);
    assert!(size_of::<BpfStat>() == 8);
    assert!(size_of::<BpfVersion>() == 4);
    assert!(size_of::<BpfTimeval>() == 8);
    assert!(size_of::<BpfHdr>() == 28);
    assert!(size_of::<BpfInsn>() == 8);
    assert!(size_of::<BpfDltlist>() == 16);
    assert!(size_of::<IfTimeval>() == 16);
    assert!(SIZEOF_BPF_HDR == 28);
};
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ioctl_numbers_are_bpf_hs() {
        // _IOR('B', 102, u_int) and friends, as an LP64 OpenBSD compiles them.
        let table: [(u64, u64); 29] = [
            (BIOCGBLEN, 0x4004_4266),
            (BIOCSBLEN, 0xc004_4266),
            (BIOCSETF, 0x8010_4267),
            (BIOCFLUSH, 0x2000_4268),
            (BIOCPROMISC, 0x2000_4269),
            (BIOCGDLT, 0x4004_426a),
            (BIOCGETIF, 0x4020_426b),
            (BIOCSETIF, 0x8020_426c),
            (BIOCSRTIMEOUT, 0x8010_426d),
            (BIOCGRTIMEOUT, 0x4010_426e),
            (BIOCGSTATS, 0x4008_426f),
            (BIOCIMMEDIATE, 0x8004_4270),
            (BIOCVERSION, 0x4004_4271),
            (BIOCSRSIG, 0x8004_4272),
            (BIOCGRSIG, 0x4004_4273),
            (BIOCGHDRCMPLT, 0x4004_4274),
            (BIOCSHDRCMPLT, 0x8004_4275),
            (BIOCLOCK, 0x2000_4276),
            (BIOCSETWF, 0x8010_4277),
            (BIOCGFILDROP, 0x4004_4278),
            (BIOCSFILDROP, 0x8004_4279),
            (BIOCSDLT, 0x8004_427a),
            (BIOCGDLTLIST, 0xc010_427b),
            (BIOCGDIRFILT, 0x4004_427c),
            (BIOCSDIRFILT, 0x8004_427d),
            (BIOCSWTIMEOUT, 0x8010_427e),
            (BIOCGWTIMEOUT, 0x4010_427e),
            (BIOCDWTIMEOUT, 0x2000_427e),
            (BIOCSETFNR, 0x8010_427f),
        ];
        for (ours, c) in table {
            assert_eq!(ours, c, "{ours:#x}");
        }
    }

    #[test]
    fn the_word_alignment_and_the_field_macros() {
        assert_eq!(bpf_wordalign(0), 0);
        assert_eq!(bpf_wordalign(1), 4);
        assert_eq!(bpf_wordalign(28), 28);
        assert_eq!(bpf_wordalign(14 + 28), 44);
        let code = BPF_LD | BPF_H | BPF_ABS;
        assert_eq!(bpf_class(code), BPF_LD);
        assert_eq!(bpf_size(code), BPF_H);
        assert_eq!(bpf_mode(code), BPF_ABS);
        let jmp = BPF_JMP | BPF_JGE | BPF_X;
        assert_eq!((bpf_op(jmp), bpf_src(jmp)), (BPF_JGE, BPF_X));
        assert_eq!(bpf_rval(BPF_RET | BPF_A), BPF_A);
        assert_eq!(bpf_miscop(BPF_MISC | BPF_TXA), BPF_TXA);
        assert_eq!(
            bpf_jump(BPF_JMP | BPF_JEQ | BPF_K, 0x800, 0, 3),
            BpfInsn {
                code: 0x15,
                jt: 0,
                jf: 3,
                k: 0x800
            }
        );
        assert_eq!(bpf_stmt(BPF_RET | BPF_K, 96).code, 0x06);
        assert_eq!(BPF_F_DIR_IN, 0x10);
        assert_eq!(BPF_F_DIR_OUT, 0x20);
        assert_eq!(BPF_F_DIR_MASK, 0x30);
    }

    #[test]
    #[ignore = "needs OPENBSD_SRC (just test-ref)"]
    fn values_match_the_c_header() {
        let defs = crate::reftest::defines("sys/net/bpf.h");
        let ours = crate::reftest::assert_defines!(defs;
            DLT_NULL, DLT_EN10MB, DLT_EN3MB, DLT_AX25, DLT_PRONET, DLT_CHAOS, DLT_IEEE802,
            DLT_ARCNET, DLT_SLIP, DLT_PPP, DLT_FDDI, DLT_ATM_RFC1483, DLT_LOOP, DLT_ENC, DLT_RAW,
            DLT_SLIP_BSDOS, DLT_PPP_BSDOS, DLT_PFSYNC, DLT_PPP_SERIAL, DLT_PPP_ETHER, DLT_C_HDLC,
            DLT_IEEE802_11, DLT_PFLOG, DLT_IEEE802_11_RADIO, DLT_USER0, DLT_USER1, DLT_USER2,
            DLT_USER3, DLT_USER4, DLT_USER5, DLT_USER6, DLT_USER7, DLT_USER8, DLT_USER9,
            DLT_USER10, DLT_USER11, DLT_USER12, DLT_USER13, DLT_USER14, DLT_USER15, DLT_USBPCAP,
            DLT_MPLS, DLT_OPENFLOW,
        );
        crate::reftest::assert_complete(&defs, "DLT_", &ours);
        let _ = crate::reftest::assert_defines!(defs;
            BPF_RELEASE, BPF_MAXINSNS, BPF_MAXBUFSIZE, BPF_MINBUFSIZE, BPF_MAJOR_VERSION,
            BPF_MINOR_VERSION, BPF_DIRECTION_IN, BPF_DIRECTION_OUT, BPF_FILDROP_PASS,
            BPF_FILDROP_CAPTURE, BPF_FILDROP_DROP, BPF_F_PRI_MASK, BPF_F_FLOWID, BPF_F_DIR_SHIFT,
            BPF_LD, BPF_LDX, BPF_ST, BPF_STX, BPF_ALU, BPF_JMP, BPF_RET, BPF_MISC, BPF_W, BPF_H,
            BPF_B, BPF_IMM, BPF_ABS, BPF_IND, BPF_MEM, BPF_LEN, BPF_MSH, BPF_RND, BPF_ADD,
            BPF_SUB, BPF_MUL, BPF_DIV, BPF_OR, BPF_AND, BPF_LSH, BPF_RSH, BPF_NEG, BPF_MOD,
            BPF_XOR, BPF_JA, BPF_JEQ, BPF_JGT, BPF_JGE, BPF_JSET, BPF_K, BPF_X, BPF_A, BPF_TAX,
            BPF_TXA, BPF_MEMWORDS,
        );
    }
}
/* </TESTS> */
