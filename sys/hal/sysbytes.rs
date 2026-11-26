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
//! The byte views of `sysdefs`' data shapes: what the C does with a cast (`(u_int16_t *)lp`,
//! `*(struct nfs_args *)data`, `&ifr->ifr_addr`), kept here because a reinterpretation of
//! bytes is `hal`'s (decision 54: `sysdefs` holds the types, constants and `#[repr(C)]`
//! structures and no `unsafe`, no logic).
//!
//! ## Redesign
//! - N7.sysdefs: the inherent methods of `hal::sys`'s types whose body was a byte
//!   reinterpretation leave the types, with the tests that used them (below, one module per
//!   header). Each is an extension trait, so a call site only adds `use hal::sysbytes::...`:
//!   [`DisklabelBytes`] (`Disklabel::from_bytes` and `cksum_words`), [`IcmpDun`] (the
//!   `icmp_dun` words of `struct icmp`), [`IfreqAddr`] and [`IfaliasreqAddr`] (the
//!   address members viewed in place) and [`NfsfhGeneric`] (the `fh_generic` member). The
//!   rest of what the types did is the generic [`AbiBytes`](crate::user::AbiBytes)
//!   (`as_bytes`, `as_bytes_mut`) and `user::{zeroed, pod_from_bytes, pod_to_bytes}`
//!   (`pod_from_bytes::<UfsArgs>(d)` is `pod_from_bytes::<UfsArgs>(d)`, `hal::user::zeroed::<KinfoFile>()` is
//!   `zeroed::<KinfoFile>()`).

use crate::sys::disklabel::{
    DISKLABEL_PARTITIONS_OFFSET, DISKLABEL_SIZE, Disklabel, MAXPARTITIONSUNIT, Partition,
};
use crate::sys::mount::Fhandle;
use crate::sys::net_if::{Ifaliasreq, Ifreq};
use crate::sys::netinet_ip_icmp::Icmp;
use crate::sys::nfs_nfsproto::Nfsfh;
use crate::sys::socket::Sockaddr;
use crate::user::{AbiBytes, bytes_of, bytes_of_mut, pod_from_bytes, pod_to_bytes, zeroed};

/// `Disklabel` as the C casts it from a sector buffer, and its checksum words.
pub trait DisklabelBytes: Sized {
    /// The label at the start of `bytes`, as the C casts a sector buffer to
    /// `struct disklabel *`; bytes past the end of `bytes` read as zero.
    fn from_bytes(bytes: &[u8]) -> Self;

    /// The `u_int16_t` words from the start of the label to `&d_partitions[npartitions]`,
    /// the range `dkcksum` XORs (`npartitions` is clamped to the table).
    fn cksum_words(&self, npartitions: usize) -> impl Iterator<Item = u16> + '_;
}

impl DisklabelBytes for Disklabel {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut lp = Self::zeroed();
        let n = bytes.len().min(DISKLABEL_SIZE);
        lp.as_bytes_mut()[..n].copy_from_slice(&bytes[..n]);
        lp
    }

    fn cksum_words(&self, npartitions: usize) -> impl Iterator<Item = u16> + '_ {
        let end = DISKLABEL_PARTITIONS_OFFSET
            + npartitions.min(MAXPARTITIONSUNIT as usize) * core::mem::size_of::<Partition>();
        self.as_bytes()[..end]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&w| u16::from_ne_bytes(w))
    }
}

/// The word of `icmp_dun` at byte `off` (a member of `id_ts`, `id_mask`).
#[inline]
fn dun32(icp: &Icmp, off: usize) -> u32 {
    let b = bytes_of(core::slice::from_ref(&icp.icmp_dun));
    u32::from_ne_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Writes the word of `icmp_dun` at byte `off`.
#[inline]
fn set_dun32(icp: &mut Icmp, off: usize, v: u32) {
    let b = bytes_of_mut(core::slice::from_mut(&mut icp.icmp_dun));
    b[off..off + 4].copy_from_slice(&v.to_ne_bytes());
}

/// The members of `struct icmp`'s `icmp_dun` union that overlay the returned IP header's
/// first words.
pub trait IcmpDun {
    /// `icmp_otime` (`icmp_dun.id_ts.its_otime`): the originate timestamp.
    fn icmp_otime(&self) -> u32;
    /// Sets `icmp_otime`.
    fn set_icmp_otime(&mut self, v: u32);
    /// `icmp_rtime` (`icmp_dun.id_ts.its_rtime`): the receive timestamp.
    fn icmp_rtime(&self) -> u32;
    /// Sets `icmp_rtime`.
    fn set_icmp_rtime(&mut self, v: u32);
    /// `icmp_ttime` (`icmp_dun.id_ts.its_ttime`): the transmit timestamp.
    fn icmp_ttime(&self) -> u32;
    /// Sets `icmp_ttime`.
    fn set_icmp_ttime(&mut self, v: u32);
    /// `icmp_mask` (`icmp_dun.id_mask`): the address mask.
    fn icmp_mask(&self) -> u32;
    /// Sets `icmp_mask`.
    fn set_icmp_mask(&mut self, v: u32);
}

impl IcmpDun for Icmp {
    fn icmp_otime(&self) -> u32 {
        dun32(self, 0)
    }

    fn set_icmp_otime(&mut self, v: u32) {
        set_dun32(self, 0, v);
    }

    fn icmp_rtime(&self) -> u32 {
        dun32(self, 4)
    }

    fn set_icmp_rtime(&mut self, v: u32) {
        set_dun32(self, 4, v);
    }

    fn icmp_ttime(&self) -> u32 {
        dun32(self, 8)
    }

    fn set_icmp_ttime(&mut self, v: u32) {
        set_dun32(self, 8, v);
    }

    fn icmp_mask(&self) -> u32 {
        dun32(self, 0)
    }

    fn set_icmp_mask(&mut self, v: u32) {
        set_dun32(self, 0, v);
    }
}

/// A union's first 16 bytes as the `sockaddr` its address members are (a `sockaddr` is 16
/// bytes of alignment 1, so the view always fits).
fn sockaddr_mut(u: &mut [u8]) -> &mut Sockaddr {
    match crate::mem::pod::view_mut::<Sockaddr>(u, 0) {
        Some(sa) => sa,
        None => crate::assert::hal_panic(format_args!("a union shorter than a sockaddr")),
    }
}

/// `struct ifreq`'s address members, to write in place.
pub trait IfreqAddr {
    /// `&ifr->ifr_addr`, to write in place (the union's 16 bytes viewed as a `sockaddr`).
    fn ifr_addr_mut(&mut self) -> &mut Sockaddr;
    /// `&ifr->ifr_dstaddr`, to write in place.
    fn ifr_dstaddr_mut(&mut self) -> &mut Sockaddr;
    /// `&ifr->ifr_broadaddr`, to write in place.
    fn ifr_broadaddr_mut(&mut self) -> &mut Sockaddr;
}

impl IfreqAddr for Ifreq {
    fn ifr_addr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifr_ifru)
    }

    fn ifr_dstaddr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifr_ifru)
    }

    fn ifr_broadaddr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifr_ifru)
    }
}

/// `struct ifaliasreq`'s address member, to write in place.
pub trait IfaliasreqAddr {
    /// `&ifra->ifra_addr`, to write in place.
    fn ifra_addr_mut(&mut self) -> &mut Sockaddr;
}

impl IfaliasreqAddr for Ifaliasreq {
    fn ifra_addr_mut(&mut self) -> &mut Sockaddr {
        sockaddr_mut(&mut self.ifra_ifrau)
    }
}

/// `struct nfsfh`'s `fh_generic` member, the `fhandle_t` at the start of the bytes.
pub trait NfsfhGeneric {
    /// `fh_generic`: the `fhandle_t` member (a copy).
    fn fh_generic(&self) -> Fhandle;
    /// Stores `fh` as the `fh_generic` member.
    fn set_fh_generic(&mut self, fh: &Fhandle);
}

impl NfsfhGeneric for Nfsfh {
    fn fh_generic(&self) -> Fhandle {
        // `fhandle_t` fits in the 64 bytes (the size check in `nfs_nfsproto`), so this never
        // falls back.
        pod_from_bytes(&self.fh_bytes).unwrap_or_else(zeroed)
    }

    fn set_fh_generic(&mut self, fh: &Fhandle) {
        // Always room: `NFSX_V3FH <= NFS_MAXFHSIZE` (the size check in `nfs_nfsproto`).
        let _ = pod_to_bytes(fh, &mut self.fh_bytes);
    }
}
/* </CODE> */

/* <TESTS> */
#[cfg(test)]
mod tests {
    mod disk {
        use super::super::*;
        use crate::sys::disk::*;

        #[test]
        fn diskstats_is_its_112_bytes() {
            let mut ds = Diskstats::default();
            assert_eq!(ds.as_bytes().len(), 112);
            ds.as_bytes_mut()[24..32].copy_from_slice(&7u64.to_ne_bytes());
            assert_eq!(ds.ds_rxfer, 7);
        }
    }

    mod disklabel {
        use super::super::*;
        use crate::sys::disklabel::*;

        #[test]
        fn label_bytes_follow_the_c_layout() {
            let mut d = Disklabel::zeroed();
            d.d_magic = DISKMAGIC;
            d.d_secsize = 512;
            d.d_npartitions = 3;
            d.d_partitions[0].p_fstype = FS_BSDFFS;
            let b = d.as_bytes();
            assert_eq!(&b[0..4], &DISKMAGIC.to_ne_bytes());
            assert_eq!(&b[40..44], &512u32.to_ne_bytes());
            assert_eq!(&b[138..140], &3u16.to_ne_bytes());
            assert_eq!(b[148 + 12], FS_BSDFFS);
            assert_eq!(
                Disklabel::from_bytes(&b[..512]).d_partitions[0].p_fstype,
                FS_BSDFFS
            );
            // The checksum range ends after the third partition.
            assert_eq!(d.cksum_words(3).count(), (148 + 3 * 16) / 2);
        }
    }

    mod mbuf {
        use crate::sys::mbuf::*;

        #[test]
        fn mbstat_is_the_c_s_seventeen_words() {
            let mut s = Mbstat::default();
            s.m_mtypes[MT_DATA as usize] = 5;
            s.m_pulldown_copy = 9;
            let mut bytes = [0u8; size_of::<Mbstat>()];
            assert!(crate::user::pod_to_bytes(&s, &mut bytes));
            assert_eq!(bytes[(3 + 1) * 8], 5);
            assert_eq!(bytes[16 * 8], 9);
        }
    }

    mod mount {
        use super::super::*;
        use crate::sys::mount::*;
        use crate::user::bytes_of;

        /// The bytes of a `T` whose every byte is its own offset (mod 256), then the `T` read
        /// out of them.
        fn offsets<T: crate::user::AbiPod>(len: usize) -> Option<T> {
            let b: [u8; 256] = core::array::from_fn(|i| i as u8);
            let v: [u8; 512] = core::array::from_fn(|i| b[i % 256]);
            crate::user::pod_from_bytes(&v[..len])
        }

        #[test]
        fn short_arguments_are_none_like_the_cs_null_data() {
            assert!(pod_from_bytes::<UfsArgs>(&[0; UfsArgs::SIZE - 1]).is_none());
            assert!(pod_from_bytes::<MfsArgs>(&[]).is_none());
            assert!(pod_from_bytes::<NfsArgs>(&[0; 111]).is_none());
            assert!(pod_from_bytes::<TmpfsArgs>(&[0; 39]).is_none());
            assert!(pod_from_bytes::<FusefsArgs>(&[0; 23]).is_none());
            assert!(pod_from_bytes::<UdfArgs>(&[0; 16]).is_some());
        }

        #[test]
        fn the_fields_are_read_at_the_cs_offsets() {
            let m: MfsArgs = offsets(MfsArgs::SIZE).unwrap();
            assert_eq!(
                m.base.to_ne_bytes(),
                core::array::from_fn(|i| (128 + i) as u8)
            );
            assert_eq!(
                m.size.to_ne_bytes(),
                core::array::from_fn(|i| (136 + i) as u8)
            );
            assert_eq!(
                m.export_info.ex_addr.to_ne_bytes(),
                core::array::from_fn(|i| (8 + 88 + i) as u8)
            );
            let n: NfsArgs = offsets(NfsArgs::SIZE).unwrap();
            assert_eq!(
                n.hostname.to_ne_bytes(),
                core::array::from_fn(|i| (88 + i) as u8)
            );
            assert_eq!(
                n.acdirmax.to_ne_bytes(),
                core::array::from_fn(|i| (108 + i) as u8)
            );
            let t: TmpfsArgs = offsets(TmpfsArgs::SIZE).unwrap();
            assert_eq!(
                t.ta_root_mode.to_ne_bytes(),
                core::array::from_fn(|i| (32 + i) as u8)
            );
            let x: NtfsArgs = offsets(NtfsArgs::SIZE).unwrap();
            assert_eq!(
                x.flag.to_ne_bytes(),
                core::array::from_fn(|i| (144 + i) as u8)
            );
            assert_eq!(
                x.export_info.ex_anon.cr_groups[0].to_ne_bytes(),
                core::array::from_fn(|i| (8 + 8 + 12 + i) as u8)
            );
        }

        #[test]
        fn a_structure_reads_back_from_its_own_bytes() {
            let mut u = pod_from_bytes::<UfsArgs>(&[0; UfsArgs::SIZE]).unwrap();
            u.fspec = 0x1234;
            u.export_info.ex_flags = 7;
            u.export_info.ex_anon.cr_ngroups = 3;
            u.export_info.ex_masklen = 16;
            let back = pod_from_bytes::<UfsArgs>(bytes_of(core::slice::from_ref(&u))).unwrap();
            assert_eq!(back.fspec, 0x1234);
            assert_eq!(back.export_info.ex_flags, 7);
            assert_eq!(back.export_info.ex_anon.cr_ngroups, 3);
            assert_eq!(back.export_info.ex_masklen, 16);
            // A longer copy (the kernel's vfc_datasize bytes) reads its first SIZE.
            let mut long = [0u8; UfsArgs::SIZE + 8];
            long[..UfsArgs::SIZE].copy_from_slice(bytes_of(core::slice::from_ref(&u)));
            assert_eq!(pod_from_bytes::<UfsArgs>(&long).unwrap().fspec, 0x1234);
        }
    }

    mod net_if {
        use crate::sys::net_if::*;

        #[test]
        fn lladdr_follows_the_name() {
            let mut sdl = SockaddrDl {
                sdl_len: 32,
                sdl_family: 18,
                sdl_nlen: 4,
                sdl_alen: 6,
                ..SockaddrDl::default()
            };
            sdl.sdl_data[..10].copy_from_slice(b"vio0\x52\x54\x00\x12\x34\x56");
            let mut b = [0u8; 32];
            assert!(crate::mem::pod::put(&mut b, 0, &sdl));
            assert_eq!(
                SockaddrDl::lladdr(&b),
                Some(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56][..])
            );
            SockaddrDl::lladdr_mut(&mut b)
                .unwrap()
                .copy_from_slice(&[1, 2, 3, 4, 5, 6]);
            assert_eq!(
                &b[SDL_DATA_OFFSET + 4..SDL_DATA_OFFSET + 10],
                &[1, 2, 3, 4, 5, 6]
            );
            // A header that names more bytes than there are.
            b[6] = 200;
            assert_eq!(SockaddrDl::lladdr(&b), None);
            assert_eq!(SockaddrDl::lladdr(&b[..3]), None);
        }
    }

    mod netinet6_in6_var {
        use crate::sys::netinet6_in6::In6Addr;
        use crate::sys::netinet6_in6::SockaddrIn6;
        use crate::sys::netinet6_in6_var::*;
        use crate::user::bytes_of;
        use core::mem::offset_of;

        #[test]
        fn the_union_members_sit_at_its_start() {
            let mut r = In6Ifreq::zeroed();
            r.ifr_name[..4].copy_from_slice(b"vio1");
            let a = SockaddrIn6::with_addr(In6Addr::new([
                0xfd, 0, 0, 0x77, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]));
            r.set_ifr_addr(a);
            assert_eq!(r.ifr_addr(), a);
            assert_eq!(r.ifr_dstaddr(), a);
            r.set_ifr_flags6(0x12);
            assert_eq!(r.ifr_flags6(), 0x12);
            // ifru_flags6 overwrote the address's first four bytes (sin6_len, family, port).
            assert_eq!(r.ifr_addr().sin6_addr, a.sin6_addr);
            let lt = In6Addrlifetime {
                ia6t_expire: 1,
                ia6t_preferred: 2,
                ia6t_vltime: 3,
                ia6t_pltime: 4,
            };
            r.set_ifr_lifetime(lt);
            assert_eq!(r.ifr_lifetime(), lt);
            let rs = [r];
            let b = bytes_of(&rs);
            assert_eq!(&b[..4], b"vio1");
            assert_eq!(b[16..24], 1i64.to_ne_bytes());
            assert_eq!(b[36..40], 4u32.to_ne_bytes());
            assert_eq!(offset_of!(In6Aliasreq, ifra_addr), 16);
            assert_eq!(offset_of!(In6Aliasreq, ifra_prefixmask), 72);
            assert_eq!(offset_of!(In6Aliasreq, ifra_lifetime), 104);
        }
    }

    mod netinet6_raw_ip6 {
        use crate::sys::netinet6_raw_ip6::*;
        use crate::user::bytes_of;

        #[test]
        fn the_statistics_are_the_c_layout() {
            let s = Rip6stat {
                rip6s_ipackets: 1,
                rip6s_opackets: 0x0102_0304_0506_0708,
                ..Rip6stat::default()
            };
            let one = [s];
            let b = bytes_of(&one);
            assert_eq!(b.len(), 56);
            assert_eq!(b[..8], 1u64.to_ne_bytes());
            assert_eq!(b[48..56], 0x0102_0304_0506_0708u64.to_ne_bytes());
        }
    }

    mod netinet_in_var {
        use crate::sys::netinet_in::InAddr;
        use crate::sys::netinet_in_var::*;
        use crate::user::bytes_of;

        #[test]
        fn the_address_sits_where_the_union_does() {
            let mut r = InAliasreq::default();
            r.ifra_name[..4].copy_from_slice(b"vio1");
            r.ifra_addr.sin_len = 16;
            r.ifra_addr.sin_family = 2;
            r.ifra_addr.sin_addr = InAddr {
                s_addr: u32::from_ne_bytes([192, 168, 77, 2]),
            };
            r.ifra_mask.sin_addr = InAddr {
                s_addr: u32::from_ne_bytes([255, 255, 255, 0]),
            };
            let rs = [r];
            let b = bytes_of(&rs);
            assert_eq!(&b[..4], b"vio1");
            assert_eq!(b[16..18], [16, 2]);
            assert_eq!(b[20..24], [192, 168, 77, 2]);
            assert_eq!(b[52..56], [255, 255, 255, 0]);
            assert_eq!(core::mem::align_of::<InAliasreq>(), 4);
            assert!(core::ptr::eq(r.ifra_broadaddr(), &r.ifra_dstaddr));
        }
    }

    mod netinet_ip {
        use crate::sys::endian::{htonl, htons, ntohs};
        use crate::sys::netinet_ip::*;
        use crate::user::bytes_of;

        #[test]
        fn header_layout_on_the_wire() {
            let mut ip = Ip::default();
            ip.set_ip_v(IPVERSION);
            ip.set_ip_hl((size_of::<Ip>() >> 2) as u8);
            ip.ip_len = htons(84);
            ip.ip_off = htons(IP_DF);
            ip.ip_ttl = IPDEFTTL;
            ip.ip_p = 1;
            ip.ip_src.s_addr = htonl(0x0a00_020f);
            ip.ip_dst.s_addr = htonl(0x0a00_0202);
            assert_eq!((ip.ip_v(), ip.ip_hl()), (4, 5));
            assert_eq!(
                bytes_of(&[ip]),
                [
                    0x45, 0, 0, 84, 0, 0, 0x40, 0, 64, 1, 0, 0, 10, 0, 2, 15, 10, 0, 2, 2
                ]
            );
            assert_eq!(ntohs(ip.ip_off) & IP_DF, IP_DF);

            let mut ts = IpTimestamp {
                ipt_code: IPOPT_TS,
                ipt_len: 12,
                ipt_ptr: 5,
                ipt_oflwflg: 0,
                ipt_timestamp: [0x0100_000a, 7],
            };
            ts.set_ipt_flg(IPOPT_TS_PRESPEC);
            ts.set_ipt_oflw(2);
            assert_eq!(ts.ipt_oflwflg, 0x23);
            assert_eq!((ts.ipt_flg(), ts.ipt_oflw()), (3, 2));
            assert_eq!(ts.ipt_time(), 0x0100_000a);
            assert_eq!(ts.ipt_ta().ipt_addr.s_addr, 0x0100_000a);
            assert_eq!(ts.ipt_ta().ipt_time, 7);
            assert_eq!(bytes_of(&[ts])[..4], [IPOPT_TS, 12, 5, 0x23]);
            assert_eq!(ipopt_number(IPOPT_LSRR), 3);
            assert_eq!(ipopt_copied(IPOPT_LSRR), 0x80);
            assert_eq!(ipopt_class(IPOPT_TS), IPOPT_DEBMEAS);
        }
    }

    mod netinet_ip_divert {
        use crate::sys::netinet_ip_divert::*;
        use crate::user::bytes_of;

        #[test]
        fn the_statistics_are_the_c_layout() {
            let s = Divstat {
                divs_ipackets: 1,
                divs_errors: 0x0102_0304_0506_0708,
                ..Divstat::default()
            };
            let one = [s];
            let b = bytes_of(&one);
            assert_eq!(b.len(), 40);
            assert_eq!(b[..8], 1u64.to_ne_bytes());
            assert_eq!(b[32..40], 0x0102_0304_0506_0708u64.to_ne_bytes());
        }
    }

    mod netinet_ip_icmp {
        use super::super::*;
        use crate::sys::netinet_in::InAddr;
        use crate::sys::netinet_ip_icmp::*;

        #[test]
        fn the_unions_members_sit_at_their_offsets() {
            let mut icp = Icmp {
                icmp_type: ICMP_ECHO,
                ..Icmp::default()
            };
            icp.set_icmp_id(0x1234_u16.to_be());
            icp.set_icmp_seq(7_u16.to_be());
            assert_eq!(u16::from_be(icp.icmp_id()), 0x1234);
            assert_eq!(u16::from_be(icp.icmp_seq()), 7);
            let head = &bytes_of(core::slice::from_ref(&icp))[..8];
            assert_eq!(head, [8, 0, 0, 0, 0x12, 0x34, 0, 7]);
            assert!(icmp_infotype(ICMP_ECHO) && icmp_infotype(ICMP_ECHOREPLY));
            assert!(!icmp_infotype(ICMP_UNREACH));

            // id_ts and id_mask overlay the returned header's first words.
            icp.set_icmp_rtime(0x0102_0304);
            icp.set_icmp_mask(0xa0b0_c0d0);
            let b = bytes_of(core::slice::from_ref(&icp));
            assert_eq!(&b[12..16], &0x0102_0304_u32.to_ne_bytes());
            assert_eq!(&b[8..12], &0xa0b0_c0d0_u32.to_ne_bytes());
            assert_eq!(icp.icmp_otime(), 0xa0b0_c0d0);

            // the pmtu and the gateway overlay the second word.
            icp.set_icmp_nextmtu(1280_u16.to_be());
            assert_eq!(u16::from_be(icp.icmp_nextmtu()), 1280);
            icp.set_icmp_gwaddr(InAddr {
                s_addr: u32::from_ne_bytes([10, 0, 2, 2]),
            });
            assert_eq!(icp.icmp_hun, [10, 0, 2, 2]);
            icp.set_icmp_pptr(21);
            assert_eq!(icp.icmp_hun[0], 21);

            icp.icmp_ip_mut().set_ip_hl(5);
            assert_eq!(icp.icmp_ip().ip_hl(), 5);
            assert_eq!(icmp_advlen(&icp), ICMP_ADVLENMIN);
            assert_eq!(icmp_v6advlen(&icp), ICMP_V6ADVLENMIN);
        }
    }

    mod netinet_tcp {
        use crate::sys::endian::{htonl, htons, ntohl, ntohs};
        use crate::sys::netinet_tcp::*;
        use crate::user::bytes_of;

        #[test]
        fn header_layout_on_the_wire() {
            let mut th = Tcphdr {
                th_sport: htons(12345),
                th_dport: htons(80),
                th_seq: htonl(0x0102_0304),
                th_ack: htonl(0x0a0b_0c0d),
                th_flags: TH_SYN | TH_ACK,
                th_win: htons(TCP_MAXWIN as u16),
                ..Tcphdr::default()
            };
            th.set_th_off((size_of::<Tcphdr>() >> 2) as u8);
            assert_eq!((th.th_off(), th.th_x2()), (5, 0));
            assert_eq!(
                bytes_of(&[th]),
                [
                    0x30, 0x39, 0, 80, 1, 2, 3, 4, 0x0a, 0x0b, 0x0c, 0x0d, 0x50, 0x12, 0xff, 0xff,
                    0, 0, 0, 0
                ]
            );
            assert_eq!((ntohs(th.th_sport), ntohl(th.th_ack)), (12345, 0x0a0b_0c0d));
        }

        /// The offsets the C compiler gives `struct tcp_info` for `aarch64-unknown-openbsd`
        /// (clang's `_Static_assert`s over `reference/openbsd-src`, N6.6a0): the `TCP_INFO`
        /// option's bytes, and the cut where the unprivileged part ends (`tcpi_snd_wnd`).
        #[test]
        fn tcp_info_matches_the_c_layout() {
            assert_eq!(size_of::<TcpInfo>(), 212);
            assert_eq!(align_of::<TcpInfo>(), 4);
            assert_eq!(core::mem::offset_of!(TcpInfo, tcpi_rto), 8);
            assert_eq!(core::mem::offset_of!(TcpInfo, tcpi_snd_wnd), 100);
            assert_eq!(core::mem::offset_of!(TcpInfo, tcpi_so_snd_sb_wat), 208);
            let info = TcpInfo {
                tcpi_state: 4,
                tcpi_options: TCPI_OPT_SACK | TCPI_OPT_TIMESTAMPS,
                tcpi_rto: 0x0102_0304,
                ..TcpInfo::default()
            };
            let one = [info];
            let b = bytes_of(&one);
            assert_eq!(b.len(), 212);
            assert_eq!(b[..9], [4, 0, 0, 0, 0, 3, 0, 0, 4]);
        }
    }

    mod netinet_tcp_var {
        use crate::sys::netinet_tcp_var::*;
        use crate::sys::socket::AF_INET6;
        use crate::user::{bytes_of, pod_from_bytes};

        #[test]
        fn a_mapping_round_trips_through_its_bytes_whole() {
            let mut m = TcpIdentMapping::zeroed();
            m.faddr.ss_len = 28;
            m.faddr.ss_family = AF_INET6;
            m.faddr.__ss_pad3[239] = 0xa5;
            m.laddr.__ss_pad2 = 0x0102_0304_0506_0708;
            m.euid = -1;
            m.ruid = 1000;
            m.rdomain = 3;
            let b = bytes_of(&[m]).to_vec();
            assert_eq!(b.len(), 528);
            // The storage is read whole, never cut at `ss_len`.
            assert_eq!(b[255], 0xa5);
            assert_eq!(b[512..516], (-1i32).to_ne_bytes());
            assert_eq!(b[520..524], 3u32.to_ne_bytes());
            let back: TcpIdentMapping = pod_from_bytes(&b).unwrap_or_else(TcpIdentMapping::zeroed);
            assert_eq!(back.faddr, m.faddr);
            assert_eq!(back.laddr, m.laddr);
            assert_eq!((back.euid, back.ruid, back.rdomain), (-1, 1000, 3));
        }

        #[test]
        fn the_statistics_are_the_c_layout() {
            let s = Tcpstat {
                tcps_connattempt: 1,
                tcps_sndbyte: 0x0102_0304_0506_0708,
                tcps_inbadlro: 9,
                ..Tcpstat::default()
            };
            let one = [s];
            let b = bytes_of(&one);
            assert_eq!(b.len(), 592);
            assert_eq!(b[..4], 1u32.to_ne_bytes());
            assert_eq!(b[72..80], 0x0102_0304_0506_0708u64.to_ne_bytes());
            assert_eq!(b[588..592], 9u32.to_ne_bytes());
        }
    }

    mod netinet_udp_var {
        use crate::sys::netinet_ip_var::Ipovly;
        use crate::sys::netinet_udp::Udphdr;
        use crate::sys::netinet_udp_var::*;
        use crate::user::bytes_of;

        #[test]
        fn the_statistics_are_thirteen_words_in_the_c_order() {
            let s = Udpstat {
                udps_ipackets: 1,
                udps_nosec: 0x0102_0304_0506_0708,
                udps_outswcsum: 13,
                ..Udpstat::default()
            };
            let one = [s];
            let b = bytes_of(&one);
            assert_eq!(b.len(), 104);
            assert_eq!(b[..8], 1u64.to_ne_bytes());
            assert_eq!(b[56..64], 0x0102_0304_0506_0708u64.to_ne_bytes());
            assert_eq!(b[96..104], 13u64.to_ne_bytes());
        }

        #[test]
        fn the_header_is_the_overlay_then_the_udp_header() {
            let ui = Udpiphdr {
                ui_i: Ipovly {
                    ih_pr: 17,
                    ih_len: 0x1234u16.to_be(),
                    ..Ipovly::default()
                },
                ui_u: Udphdr {
                    uh_sport: 0x0035u16.to_be(),
                    uh_sum: 0xbeefu16.to_be(),
                    ..Udphdr::default()
                },
            };
            let one = [ui];
            let b = bytes_of(&one);
            assert_eq!(b.len(), 28);
            assert_eq!(b[9], 17);
            assert_eq!(b[10..12], [0x12, 0x34]);
            assert_eq!(b[20..22], [0x00, 0x35]);
            assert_eq!(b[26..28], [0xbe, 0xef]);
        }
    }

    mod nfs_nfsproto {
        use super::super::*;
        use crate::sys::nfs_nfsproto::*;

        #[test]
        fn file_handle_generic_member_is_the_first_bytes() {
            let mut fh = Nfsfh::new();
            let mut g = Fhandle::default();
            g.fh_fsid.val = [1, 2];
            g.fh_fid.fid_len = 12;
            g.fh_fid.fid_data[0] = 0xaa;
            fh.set_fh_generic(&g);
            assert_eq!(fh.fh_generic(), g);
            assert_eq!(fh.fh_bytes[0..4], 1i32.to_ne_bytes());
            assert!(fh.fh_bytes[NFSX_V3FH..].iter().all(|&b| b == 0));
        }
    }

    mod socket {
        use crate::sys::socket::*;
        use crate::sys::types::Socklen;
        use crate::user::{bytes_of, pod_from_bytes};

        #[test]
        fn the_images_round_trip_through_their_bytes() {
            let cm = Cmsghdr {
                cmsg_len: cmsg_len(8) as Socklen,
                cmsg_level: SOL_SOCKET,
                cmsg_type: SCM_RIGHTS,
            };
            let b = bytes_of(core::slice::from_ref(&cm));
            assert_eq!(b.len(), 12);
            assert_eq!(&b[..4], &24u32.to_ne_bytes());
            assert_eq!(pod_from_bytes::<Cmsghdr>(b), Some(cm));

            let sa = Sockaddr {
                sa_len: 16,
                sa_family: AF_INET,
                sa_data: [7; 14],
            };
            assert_eq!(bytes_of(core::slice::from_ref(&sa))[..2], [16, AF_INET]);
            let ss = SockaddrStorage::zeroed();
            assert!(bytes_of(core::slice::from_ref(&ss)).iter().all(|&b| b == 0));
            let cred = Sockpeercred {
                uid: 1,
                gid: 2,
                pid: 3,
            };
            assert_eq!(
                pod_from_bytes::<Sockpeercred>(bytes_of(core::slice::from_ref(&cred))),
                Some(cred)
            );
        }

        #[test]
        fn a_msghdr_keeps_its_holes_through_its_bytes() {
            // The C's LP64 layout: a pointer, a socklen_t and its hole, a pointer, an unsigned
            // int and its hole, a pointer, a socklen_t and an int.
            let mut b = [0u8; 56];
            for (i, x) in b.iter_mut().enumerate() {
                *x = i as u8;
            }
            let m = pod_from_bytes::<Mmsghdr>(&b).expect("56 bytes");
            assert_eq!(
                m.msg_hdr.msg_name,
                usize::from_ne_bytes(*b[..8].first_chunk().unwrap())
            );
            assert_eq!(m.msg_hdr.msg_namelen, u32::from_ne_bytes([8, 9, 10, 11]));
            assert_eq!(m.msg_hdr._pad0, u32::from_ne_bytes([12, 13, 14, 15]));
            assert_eq!(m.msg_hdr.msg_iovlen, u32::from_ne_bytes([24, 25, 26, 27]));
            assert_eq!(
                m.msg_hdr.msg_controllen,
                u32::from_ne_bytes([40, 41, 42, 43])
            );
            assert_eq!(m.msg_hdr.msg_flags, i32::from_ne_bytes([44, 45, 46, 47]));
            assert_eq!(m.msg_len, u32::from_ne_bytes([48, 49, 50, 51]));
            // Back to the same bytes, the holes included.
            assert_eq!(bytes_of(core::slice::from_ref(&m)), &b[..]);
            assert_eq!(pod_from_bytes::<Msghdr>(&b[..48]), Some(m.msg_hdr));
        }
    }

    mod un {
        use crate::sys::un::*;
        use crate::user::{bytes_of, pod_from_bytes};

        #[test]
        fn a_sockaddr_un_round_trips_through_its_bytes() {
            let mut su = SockaddrUn {
                sun_len: 9,
                sun_family: 1,
                sun_path: [0; SUN_PATH_LEN],
            };
            su.sun_path[..6].copy_from_slice(b"/tmp/s");
            let b = bytes_of(core::slice::from_ref(&su));
            assert_eq!(&b[..8], &[9, 1, b'/', b't', b'm', b'p', b'/', b's']);
            assert_eq!(pod_from_bytes::<SockaddrUn>(b), Some(su));
            assert_eq!(pod_from_bytes::<SockaddrUn>(&b[..105]), None);
        }
    }
}
/* </TESTS> */
