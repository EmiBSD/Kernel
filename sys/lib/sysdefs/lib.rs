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
/* </LICENSES> */

/* <CODE> */
//! `sysdefs`: OpenBSD's basic types, constants and `#[repr(C)]` structures, the ABI edge's
//! data shapes (`errno.h`, `types.h`, `syscall.h`, the network headers, ...), in one leaf
//! crate under every layer (decision 54, the user, 2026-11-26): `hal`, `kernel`, `abi` and
//! `bsd` depend on it and it depends on nothing. `#![forbid(unsafe_code)]`, no state and no
//! logic; a byte reinterpretation of one of its types is `hal`'s (`hal::user::AbiBytes`,
//! `hal::sysbytes`), and `AbiPod`/`abi_pod!` (the marker those trust) are [`pod`].
//!
//! Moved from `hal::sys` (N2.2 to N7.1d); the module paths are the same with `hal::sys`
//! replaced by `sysdefs` (`sysdefs::errno`, `sysdefs::types`, ...).
#![no_std]
#![forbid(unsafe_code)]
// `clippy::pedantic` is on in this crate (the user's decision of 2026-10-29, measured first:
// docs/JOURNAL.md). What it asks and this crate declines, each with its reason; the three cast
// lints are allowed until N11 types their sites one by one (docs/ROADMAP.md, N11).
#![warn(clippy::pedantic)]
#![allow(
    clippy::unreadable_literal,
    reason = "the hex tables of the ciphers and the constants of the C headers read as the C writes them"
)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    reason = "the C's names in prose and the `Result`'s type say it; backticks and an `# Errors` section per function are noise"
)]
#![allow(
    clippy::must_use_candidate,
    reason = "a `#[must_use]` on every pure function is noise; the ones that matter carry it"
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::many_single_char_names,
    reason = "OpenBSD's names are kept so the C can be grepped (`uvm::uvm_fault::uvm_fault`, `p`, `pr`, `ps`)"
)]
#![allow(
    clippy::too_many_lines,
    clippy::inline_always,
    reason = "a function the C keeps whole stays whole; `#[inline(always)]` sits on the hot paths decision 43 measures"
)]
#![allow(
    clippy::used_underscore_items,
    clippy::pub_underscore_fields,
    reason = "the C's `_IOR`/`_IOW` names and the `_pad` fields of `#[repr(C)]` layouts, kept as the C spells them"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "typed one by one, 1,348 sites: abi's 235 (sysent.rs, the syscall arguments) in a slice of their own at the N5 close or the N6 start, the user's decision of 2026-10-29; kernel's 1,061 and hal's 52 at N11"
)]

#[forbid(unsafe_code)]
pub mod dirent;
#[forbid(unsafe_code)]
pub mod disk;
#[forbid(unsafe_code)]
pub mod disklabel;
#[forbid(unsafe_code)]
pub mod dkio;
#[forbid(unsafe_code)]
pub mod endian;
#[forbid(unsafe_code)]
pub mod errno;
#[forbid(unsafe_code)]
pub mod exec_elf;
#[forbid(unsafe_code)]
pub mod fcntl;
#[forbid(unsafe_code)]
pub mod filio;
#[forbid(unsafe_code)]
pub mod futex;
#[forbid(unsafe_code)]
pub mod intrmap;
#[forbid(unsafe_code)]
pub mod ioccom;
#[forbid(unsafe_code)]
pub mod kernel;
#[forbid(unsafe_code)]
pub mod limits;
#[forbid(unsafe_code)]
pub mod mbuf;
#[forbid(unsafe_code)]
pub mod mman;
#[forbid(unsafe_code)]
pub mod mount;
#[forbid(unsafe_code)]
pub mod net_bpf;
#[forbid(unsafe_code)]
pub mod net_ethertypes;
#[forbid(unsafe_code)]
pub mod net_if;
#[forbid(unsafe_code)]
pub mod net_if_arp;
#[forbid(unsafe_code)]
pub mod net_if_media;
#[forbid(unsafe_code)]
pub mod net_route;
#[forbid(unsafe_code)]
pub mod netinet6_in6;
#[forbid(unsafe_code)]
pub mod netinet6_in6_var;
#[forbid(unsafe_code)]
pub mod netinet6_ip6_var;
#[forbid(unsafe_code)]
pub mod netinet6_mld6;
#[forbid(unsafe_code)]
pub mod netinet6_nd6;
#[forbid(unsafe_code)]
pub mod netinet6_raw_ip6;
#[forbid(unsafe_code)]
pub mod netinet_icmp6;
#[forbid(unsafe_code)]
pub mod netinet_icmp_var;
#[forbid(unsafe_code)]
pub mod netinet_if_ether;
#[forbid(unsafe_code)]
pub mod netinet_igmp;
#[forbid(unsafe_code)]
pub mod netinet_igmp_var;
#[forbid(unsafe_code)]
pub mod netinet_in;
#[forbid(unsafe_code)]
pub mod netinet_in_var;
#[forbid(unsafe_code)]
pub mod netinet_ip;
#[forbid(unsafe_code)]
pub mod netinet_ip6;
#[forbid(unsafe_code)]
pub mod netinet_ip_divert;
#[forbid(unsafe_code)]
pub mod netinet_ip_icmp;
#[forbid(unsafe_code)]
pub mod netinet_ip_mroute;
#[forbid(unsafe_code)]
pub mod netinet_ip_var;
#[forbid(unsafe_code)]
pub mod netinet_tcp;
#[forbid(unsafe_code)]
pub mod netinet_tcp_fsm;
#[forbid(unsafe_code)]
pub mod netinet_tcp_var;
#[forbid(unsafe_code)]
pub mod netinet_udp;
#[forbid(unsafe_code)]
pub mod netinet_udp_var;
#[forbid(unsafe_code)]
pub mod nfs_nfs;
#[forbid(unsafe_code)]
pub mod nfs_nfsproto;
#[forbid(unsafe_code)]
pub mod nfs_rpcv2;
/// `<sys/param.h>`'s page size for the headers moved here (`ioccom.h`'s `IOCPARM_MAX`): the
/// machine's (`<machine/param.h>`), so `crate::param::PAGE_SIZE` names the same constant in
/// both crates.
#[forbid(unsafe_code)]
pub mod param {
    /// `PAGE_SIZE`: 4096 on arm64, amd64 and the host (`hal::vm::param` asserts the machine's equal).
    pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
    /// `PAGE_SHIFT`: log2 of `PAGE_SIZE`.
    pub const PAGE_SHIFT: usize = 12;
    /// `PAGE_MASK`: the offset bits of an address in a page.
    pub const PAGE_MASK: usize = PAGE_SIZE - 1;
}
#[forbid(unsafe_code)]
pub mod pclock;
#[forbid(unsafe_code)]
pub mod pledge;
#[forbid(unsafe_code)]
pub mod pod;
#[forbid(unsafe_code)]
pub mod reboot;
#[forbid(unsafe_code)]
pub mod select;
#[forbid(unsafe_code)]
pub mod sensors;
#[forbid(unsafe_code)]
pub mod siginfo;
#[forbid(unsafe_code)]
pub mod signal;
#[forbid(unsafe_code)]
pub mod socket;
#[forbid(unsafe_code)]
pub mod sockio;
#[forbid(unsafe_code)]
pub mod softintr;
#[forbid(unsafe_code)]
pub mod stat;
#[forbid(unsafe_code)]
pub mod syscall;
#[forbid(unsafe_code)]
pub mod sysctl;
#[forbid(unsafe_code)]
pub mod syslimits;
#[forbid(unsafe_code)]
pub mod timetc;
#[forbid(unsafe_code)]
pub mod types;
#[forbid(unsafe_code)]
pub mod un;
#[forbid(unsafe_code)]
pub mod unistd;
#[forbid(unsafe_code)]
pub mod uuid;
#[forbid(unsafe_code)]
pub mod vmmeter;
#[forbid(unsafe_code)]
pub mod wait;

// The headers still `inherited` from LZ (`dirent`, `fcntl`, `filio`, `ioccom`, `syslimits`) name
// their siblings as `crate::sys::x`, as LZ's tree does; this keeps them byte-identical.
pub(crate) use crate as sys;

#[cfg(test)]
extern crate std;
#[cfg(test)]
#[allow(dead_code, unused_macros, unused_imports)]
pub(crate) mod reftest;
/* </CODE> */
